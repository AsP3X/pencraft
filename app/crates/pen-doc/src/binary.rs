//! The document as a tree of values in binary, each key written once.
//!
//! What a `.quill` file holds in the place of a `.pen` file's JSON text: the same values in the same order, so
//! a document goes there and back without losing a key, a kind or a digit.
//!
//! The bytes: the encoding's number, the key table (a count, then each key as a length and its UTF-8 bytes, in
//! the order the keys first appear), then one value. Counts, lengths and whole numbers are LEB128 varints. A
//! value is a tag byte and what follows it.
// Human: a design names the same few keys thousands of times (`type`, `x`, `fill`); written once and counted
// from then on, they cost a byte or two each. The document goes through `serde_json::Value`, so keys and kinds
// this version doesn't know travel as they are.
// Agent: no I/O. Bytes from outside never panic and never make it allocate more than they are long: every
// count and length is checked against what is left.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use serde_json::{Map, Number, Value};

use crate::Document;

/// The number of this encoding, the first byte.
const ENCODING: u8 = 1;
/// How many lists and maps may lie in each other: 127, as many as `serde_json` reads from text, so what opens
/// from a `.quill` file also opens once it is saved as `.pen`.
const DEEPEST: usize = 127;
/// The most the reader sets aside for a list before it has seen what is in it.
const RESERVE_AT_MOST: usize = 1024;
/// The most key text a file may have copied into its maps in all.
const MOST_KEY_BYTES: usize = 256 * 1024 * 1024;

const NULL: u8 = 0;
const FALSE: u8 = 1;
const TRUE: u8 = 2;
/// A whole number, zero or above: a varint.
const WHOLE: u8 = 3;
/// A whole number below zero: the varint of `-(n + 1)`.
const BELOW: u8 = 4;
/// A float: 8 bytes, IEEE 754, little-endian.
const FLOAT: u8 = 5;
/// A string: its length, then UTF-8 bytes.
const TEXT: u8 = 6;
/// A list: its count, then the values.
const LIST: u8 = 7;
/// A map: its count, then pairs of the key's number in the table and the value.
const MAP: u8 = 8;

/// Why bytes could not be read as a document, or a document not be written.
#[derive(Debug, PartialEq, Eq)]
pub struct BinaryError(String);

impl std::fmt::Display for BinaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not a readable document: {}", self.0)
    }
}

impl std::error::Error for BinaryError {}

fn bad(why: &str) -> BinaryError {
    BinaryError(why.to_owned())
}

/// Writes a document as binary.
///
/// # Errors
/// When its lists and maps lie deeper in each other than [`read`] goes.
pub fn write(doc: &Document) -> Result<Vec<u8>, BinaryError> {
    let value = serde_json::to_value(doc).map_err(|error| BinaryError(error.to_string()))?;
    write_value(&value)
}

/// Reads a document from binary.
///
/// # Errors
/// When the bytes aren't this encoding, are cut off or damaged, or don't hold a document.
pub fn read(bytes: &[u8]) -> Result<Document, BinaryError> {
    serde_json::from_value(read_value(bytes)?).map_err(|error| BinaryError(error.to_string()))
}

/// Writes any JSON value as binary.
///
/// # Errors
/// When its lists and maps lie deeper in each other than [`read_value`] goes.
pub fn write_value(value: &Value) -> Result<Vec<u8>, BinaryError> {
    let mut keys = Keys::default();
    keys.gather(value, 0)?;
    let mut out = vec![ENCODING];
    put_count(&mut out, keys.names.len());
    for name in &keys.names {
        put_text(&mut out, name);
    }
    put(&mut out, value, &keys);
    Ok(out)
}

/// The keys of a value, each once, in the order they first appear, and the number of each.
#[derive(Default)]
struct Keys<'a> {
    names: Vec<&'a str>,
    numbers: HashMap<&'a str, usize>,
}

impl<'a> Keys<'a> {
    /// Takes in the keys of `value`, which has `depth` lists and maps open around it.
    fn gather(&mut self, value: &'a Value, depth: usize) -> Result<(), BinaryError> {
        if matches!(value, Value::Array(_) | Value::Object(_)) && depth >= DEEPEST {
            return Err(bad("it is nested too deeply"));
        }
        match value {
            Value::Array(list) => list.iter().try_for_each(|inner| self.gather(inner, depth + 1)),
            Value::Object(map) => map.iter().try_for_each(|(key, inner)| {
                if let Entry::Vacant(free) = self.numbers.entry(key.as_str()) {
                    free.insert(self.names.len());
                    self.names.push(key.as_str());
                }
                self.gather(inner, depth + 1)
            }),
            _ => Ok(()),
        }
    }
}

/// Writes `number` as a varint: seven bits a byte, the lowest first, the top bit set on all but the last.
fn put_varint(out: &mut Vec<u8>, mut number: u64) {
    loop {
        let low = number.to_le_bytes()[0] & 0x7f;
        number >>= 7;
        if number == 0 {
            out.push(low);
            return;
        }
        out.push(low | 0x80);
    }
}

fn put_count(out: &mut Vec<u8>, count: usize) {
    put_varint(out, u64::try_from(count).unwrap_or(u64::MAX));
}

fn put_text(out: &mut Vec<u8>, text: &str) {
    put_count(out, text.len());
    out.extend(text.as_bytes());
}

/// Writes `value`; every key in it is in `keys` ([`Keys::gather`] went through the same value).
fn put(out: &mut Vec<u8>, value: &Value, keys: &Keys<'_>) {
    match value {
        Value::Null => out.push(NULL),
        Value::Bool(false) => out.push(FALSE),
        Value::Bool(true) => out.push(TRUE),
        Value::Number(number) => put_number(out, number),
        Value::String(text) => {
            out.push(TEXT);
            put_text(out, text);
        }
        Value::Array(list) => {
            out.push(LIST);
            put_count(out, list.len());
            for inner in list {
                put(out, inner, keys);
            }
        }
        Value::Object(map) => {
            out.push(MAP);
            put_count(out, map.len());
            for (key, inner) in map {
                put_count(out, keys.numbers.get(key.as_str()).copied().unwrap_or_default());
                put(out, inner, keys);
            }
        }
    }
}

// Human: a number keeps its kind. `1` and `1.0` are different text in a `.pen` file, and a file that goes
// through a `.quill` must come out the same.
// Agent: `as_u64` first, then `as_i64`, then float: a `serde_json` number is exactly one of the three. Below zero
// is stored as `-(n + 1)` so `i64::MIN` fits; `unsigned_abs` cannot overflow.
fn put_number(out: &mut Vec<u8>, number: &Number) {
    if let Some(whole) = number.as_u64() {
        out.push(WHOLE);
        put_varint(out, whole);
    } else if let Some(below) = number.as_i64() {
        out.push(BELOW);
        put_varint(out, below.unsigned_abs().saturating_sub(1));
    } else {
        out.push(FLOAT);
        out.extend(number.as_f64().unwrap_or_default().to_le_bytes());
    }
}

/// Reads a JSON value from binary.
///
/// # Errors
/// When the bytes aren't this encoding, or are cut off or damaged.
pub fn read_value(bytes: &[u8]) -> Result<Value, BinaryError> {
    read_with_key_budget(bytes, MOST_KEY_BYTES)
}

/// [`read_value`] with the most key text a file may have copied into its maps given, so a test can reach it.
fn read_with_key_budget(bytes: &[u8], key_bytes: usize) -> Result<Value, BinaryError> {
    let mut reader = Reader { bytes, at: 0, keys: Vec::new(), key_bytes_left: key_bytes };
    if reader.byte()? != ENCODING {
        return Err(bad("it is written in a way this version doesn't know"));
    }
    for _ in 0..reader.count(1)? {
        let key = reader.text()?;
        reader.keys.push(key);
    }
    let value = reader.value(0)?;
    if reader.at != bytes.len() {
        return Err(bad("there is more after its end"));
    }
    Ok(value)
}

/// Bytes being read, how far, and the key table read from them.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    keys: Vec<String>,
    /// How much more key text may be copied into maps.
    key_bytes_left: usize,
}

impl<'a> Reader<'a> {
    fn byte(&mut self) -> Result<u8, BinaryError> {
        let byte = self.bytes.get(self.at).copied().ok_or_else(|| bad("it ends too early"))?;
        self.at += 1;
        Ok(byte)
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], BinaryError> {
        let to = self.at.checked_add(length).ok_or_else(|| bad("it ends too early"))?;
        let taken = self.bytes.get(self.at..to).ok_or_else(|| bad("it ends too early"))?;
        self.at = to;
        Ok(taken)
    }

    // Human: a varint is seven bits a byte, so a number that fits 64 bits takes at most ten.
    // Agent: the tenth byte may only carry the one bit left of 64; anything else, or an eleventh byte, is refused.
    fn varint(&mut self) -> Result<u64, BinaryError> {
        let mut number = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = self.byte()?;
            let part = u64::from(byte & 0x7f);
            // The tenth byte holds the one bit that is left of 64.
            if shift == 63 && part > 1 {
                return Err(bad("a number in it is too large"));
            }
            number |= part << shift;
            if byte & 0x80 == 0 {
                return Ok(number);
            }
        }
        Err(bad("a number in it is too large"))
    }

    /// A count of things that take `each` bytes at least; one that says more than the bytes left hold is refused
    /// before anything is set aside for it.
    // Human: a few bytes can claim a billion items; the bytes that are left say how many can be there.
    // Agent: `at` never passes the length, so the subtraction cannot underflow.
    fn count(&mut self, each: usize) -> Result<usize, BinaryError> {
        let count = usize::try_from(self.varint()?).ok();
        let left = self.bytes.len() - self.at;
        count.filter(|count| count.checked_mul(each).is_some_and(|least| least <= left)).ok_or_else(|| bad("it says it holds more than it does"))
    }

    fn text(&mut self) -> Result<String, BinaryError> {
        let length = self.count(1)?;
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| bad("a text in it isn't UTF-8"))
    }

    // Human: a map names its keys by number, and each use copies the key's text. A file could name one huge key
    // in a quarter of a million maps, so what is copied in all is capped.
    // Agent: charge the key's length before copying it; the reader never holds more than the cap of key copies.
    fn key(&mut self) -> Result<String, BinaryError> {
        let number = usize::try_from(self.varint()?).ok();
        let key = number.and_then(|number| self.keys.get(number)).ok_or_else(|| bad("a key isn't in its table"))?;
        self.key_bytes_left = self.key_bytes_left.checked_sub(key.len()).ok_or_else(|| bad("it holds more than this version opens"))?;
        Ok(key.clone())
    }

    // Human: one value, and everything inside it. Lists and maps reserve nothing for what a file only claims
    // they hold, and a map may not name a key twice.
    // Agent: `depth` is the number of lists and maps open around this value; entering one more past `DEEPEST`
    // is refused, which is where `serde_json::from_str` stops too.
    fn value(&mut self, depth: usize) -> Result<Value, BinaryError> {
        match self.byte()? {
            NULL => Ok(Value::Null),
            FALSE => Ok(Value::Bool(false)),
            TRUE => Ok(Value::Bool(true)),
            WHOLE => Ok(Value::Number(self.varint()?.into())),
            BELOW => {
                let under = i64::try_from(self.varint()?).map_err(|_| bad("a number in it is too large"))?;
                Ok(Value::Number((-under - 1).into()))
            }
            FLOAT => {
                let raw: [u8; 8] = self.take(8)?.try_into().map_err(|_| bad("it ends too early"))?;
                Number::from_f64(f64::from_le_bytes(raw)).map(Value::Number).ok_or_else(|| bad("a number in it is not finite"))
            }
            TEXT => self.text().map(Value::String),
            LIST => {
                if depth >= DEEPEST {
                    return Err(bad("it is nested too deeply"));
                }
                let count = self.count(1)?;
                let mut list = Vec::with_capacity(count.min(RESERVE_AT_MOST));
                for _ in 0..count {
                    list.push(self.value(depth + 1)?);
                }
                Ok(Value::Array(list))
            }
            MAP => {
                if depth >= DEEPEST {
                    return Err(bad("it is nested too deeply"));
                }
                let count = self.count(2)?;
                let mut map = Map::new();
                for _ in 0..count {
                    let key = self.key()?;
                    if map.contains_key(&key) {
                        return Err(bad("a key is in a map twice"));
                    }
                    let inner = self.value(depth + 1)?;
                    map.insert(key, inner);
                }
                Ok(Value::Object(map))
            }
            _ => Err(bad("it holds a kind of value this version doesn't know")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const DESIGN: &str = r##"{"version":"2.22","pencraft":{"format":1,"features":["mask"]},"children":[
        {"type":"frame","id":"a","x":0,"y":-12,"width":267.8010819142764,"height":40,"clip":true,"children":[
            {"type":"path","id":"b","rotation":-45,"stroke":"#ff8302ff","geometry":"M7 0l0 14",
             "futureProperty":{"kept":true,"ratio":1.0,"whole":1,"list":[1,-2,3.5,"x",null,false,[]],
                               "big":18446744073709551615,"small":-9223372036854775808}}]},
        {"type":"hologram","id":"h","beam":3,"children":[]}],
        "themes":{"Mode":["Light","Dark"]},"plugins":[{"children":"not layers"}]}"##;

    #[test]
    fn a_document_goes_to_bytes_and_back_and_writes_the_same_pen_text() {
        let doc = crate::read(DESIGN).unwrap();
        let text = crate::write(&doc).unwrap();
        let bytes = write(&doc).unwrap();
        let back = read(&bytes).unwrap();
        assert_eq!(back, doc);
        assert_eq!(crate::write(&back).unwrap(), text);
        // Whole numbers and floats keep their kind, and what this version doesn't know is still there.
        assert!(text.contains("\"ratio\": 1.0") && text.contains("\"whole\": 1") && text.contains("\"hologram\""), "{text}");
        assert!(text.contains("18446744073709551615") && text.contains("-9223372036854775808"), "{text}");
        // The same document gives the same bytes.
        assert_eq!(write(&back).unwrap(), bytes);
    }

    #[test]
    fn each_key_is_written_once() {
        let doc = crate::read(DESIGN).unwrap();
        let bytes = write(&doc).unwrap();
        let times = |word: &[u8]| bytes.windows(word.len()).filter(|window| *window == word).count();
        // `children` is a key four times in the design.
        assert_eq!((times(b"children"), times(b"futureProperty")), (1, 1));
        assert!(bytes.len() < crate::write(&doc).unwrap().len(), "{} bytes", bytes.len());
    }

    #[test]
    fn plain_values_go_there_and_back() {
        for value in [
            json!(null),
            json!(true),
            json!(0),
            json!(127),
            json!(128),
            json!(u64::MAX),
            json!(-1),
            json!(i64::MIN),
            json!(0.5),
            json!(-0.0),
            json!(""),
            json!("ä b"),
            json!([]),
            json!({}),
            json!({ "b": 1, "a": { "b": [2, { "a": null }] } }),
        ] {
            let bytes = write_value(&value).unwrap();
            assert_eq!(read_value(&bytes).unwrap(), value, "{value}");
            assert_eq!(read_value(&bytes).unwrap().to_string(), value.to_string(), "{value}");
        }
        // 127 fits one byte of a varint, 128 takes two.
        assert_eq!(write_value(&json!(127)).unwrap(), [ENCODING, 0, WHOLE, 127]);
        assert_eq!(write_value(&json!(128)).unwrap(), [ENCODING, 0, WHOLE, 0x80, 1]);
        assert_eq!(write_value(&json!(-1)).unwrap(), [ENCODING, 0, BELOW, 0]);
        assert_eq!(write_value(&json!({ "a": true })).unwrap(), [ENCODING, 1, 1, b'a', MAP, 1, 0, TRUE]);
    }

    /// `depth` lists in each other around a null.
    fn nested(depth: usize) -> Vec<u8> {
        let mut bytes = vec![ENCODING, 0];
        for _ in 0..depth {
            bytes.extend([LIST, 1]);
        }
        bytes.push(NULL);
        bytes
    }

    #[test]
    fn what_cannot_be_read_is_refused_with_a_sentence() {
        let refused = |bytes: &[u8]| read_value(bytes).unwrap_err().0;
        assert_eq!(refused(&[]), "it ends too early");
        assert_eq!(refused(&[2, 0, NULL]), "it is written in a way this version doesn't know");
        assert_eq!(refused(&[ENCODING, 0, 9]), "it holds a kind of value this version doesn't know");
        assert_eq!(refused(&[ENCODING, 0, MAP, 1, 0, NULL]), "a key isn't in its table");
        assert_eq!(refused(&[ENCODING, 0, TEXT, 5, b'a']), "it says it holds more than it does");
        assert_eq!(refused(&[ENCODING, 0, LIST, 200, 1]), "it says it holds more than it does");
        assert_eq!(refused(&[ENCODING, 9, 1, b'a']), "it says it holds more than it does");
        assert_eq!(refused(&[ENCODING, 0, TEXT, 1, 0xff]), "a text in it isn't UTF-8");
        assert_eq!(refused(&[ENCODING, 0, FLOAT, 0, 0]), "it ends too early");
        let mut endless = vec![ENCODING, 0, FLOAT];
        endless.extend(f64::NAN.to_le_bytes());
        assert_eq!(refused(&endless), "a number in it is not finite");
        assert_eq!(refused(&[ENCODING, 0, NULL, NULL]), "there is more after its end");
        assert_eq!(refused(&[ENCODING, 0, WHOLE, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 2]), "a number in it is too large");
        assert_eq!(refused(&[ENCODING, 0, BELOW, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 1]), "a number in it is too large");
        // 128 deep reads; one deeper is refused, and is not written either.
        assert!(read_value(&nested(DEEPEST)).is_ok());
        assert_eq!(refused(&nested(DEEPEST + 1)), "it is nested too deeply");
        let deep = read_value(&nested(DEEPEST)).unwrap();
        assert!(write_value(&deep).is_ok());
        assert_eq!(write_value(&json!([deep])).unwrap_err().0, "it is nested too deeply");
        assert!(read(&write_value(&json!([1])).unwrap()).is_err(), "a list is no document");
    }

    #[test]
    fn the_key_table_is_in_the_order_the_keys_first_appear() {
        let bytes = write_value(&json!({ "b": 1, "a": { "b": 2 } })).unwrap();
        let expected = [ENCODING, 2, 1, b'b', 1, b'a', MAP, 2, 0, WHOLE, 1, 1, MAP, 1, 0, WHOLE, 2];
        assert_eq!(bytes, expected);
    }

    #[test]
    fn a_count_that_passes_the_bytes_left_and_broken_numbers_are_refused() {
        let refused = |bytes: &[u8]| read_value(bytes).unwrap_err().0;
        // One pair claimed with one byte left for it; and two pairs with room for one.
        assert_eq!(refused(&[ENCODING, 1, 1, b'a', MAP, 1, 0]), "it says it holds more than it does");
        assert_eq!(refused(&[ENCODING, 1, 1, b'a', MAP, 2, 0, NULL]), "it says it holds more than it does");
        // A varint cut off after a byte that says more follows.
        assert_eq!(refused(&[ENCODING, 0, WHOLE, 0x80]), "it ends too early");
        // Eleven bytes of a varint.
        let mut long = vec![ENCODING, 0, WHOLE];
        long.extend([0x80; 10]);
        long.push(0);
        assert_eq!(refused(&long), "a number in it is too large");
    }

    #[test]
    fn lists_claiming_a_million_items_reserve_nothing_for_them() {
        // Each list says 1,000,000 items with a few bytes behind it: refused for its count at the first list,
        // before anything is reserved. (The cap on the reservation itself can't be seen from outside.)
        let mut bytes = vec![ENCODING, 0];
        for _ in 0..100 {
            bytes.extend([LIST, 0xc0, 0x84, 0x3d]);
        }
        assert_eq!(read_value(&bytes).unwrap_err().0, "it says it holds more than it does");
    }

    #[test]
    fn a_key_copied_into_too_many_maps_is_refused() {
        // Table: one key of two bytes; a list of three maps naming it.
        let mut bytes = vec![ENCODING, 1, 2, b'a', b'b', LIST, 3];
        for _ in 0..3 {
            bytes.extend([MAP, 1, 0, NULL]);
        }
        assert!(read_with_key_budget(&bytes, 6).is_ok());
        let over = read_with_key_budget(&bytes, 5).unwrap_err().0;
        assert_eq!(over, "it holds more than this version opens");
    }

    #[test]
    fn a_key_twice_in_one_map_is_refused() {
        let bytes = [ENCODING, 1, 1, b'a', MAP, 2, 0, NULL, 0, NULL];
        assert_eq!(read_value(&bytes).unwrap_err().0, "a key is in a map twice");
    }

    /// `depth` lists or maps in each other, the innermost empty.
    fn lying(depth: usize, maps: bool) -> Value {
        let mut value = if maps { json!({}) } else { json!([]) };
        for _ in 1..depth {
            value = if maps { json!({ "k": value }) } else { json!([value]) };
        }
        value
    }

    #[test]
    fn what_binary_opens_the_json_reader_opens_too() {
        for maps in [false, true] {
            let deepest = lying(DEEPEST, maps);
            let bytes = write_value(&deepest).unwrap();
            assert_eq!(read_value(&bytes).unwrap(), deepest);
            assert!(serde_json::from_str::<Value>(&deepest.to_string()).is_ok(), "{DEEPEST} deep, maps: {maps}");

            let too_deep = lying(DEEPEST + 1, maps);
            assert_eq!(write_value(&too_deep).unwrap_err().0, "it is nested too deeply");
            assert!(serde_json::from_str::<Value>(&too_deep.to_string()).is_err(), "{} deep, maps: {maps}", DEEPEST + 1);
        }
        // Read side with an empty innermost list: 127 pass, 128 are refused.
        let mut bytes = vec![ENCODING, 0];
        bytes.extend([LIST, 1].repeat(DEEPEST - 1));
        bytes.extend([LIST, 0]);
        assert!(read_value(&bytes).is_ok());
        let mut bytes = vec![ENCODING, 0];
        bytes.extend([LIST, 1].repeat(DEEPEST));
        bytes.extend([LIST, 0]);
        assert_eq!(read_value(&bytes).unwrap_err().0, "it is nested too deeply");
    }
}
