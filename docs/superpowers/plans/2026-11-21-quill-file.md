# The `.quill` File Format: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Save writes one binary `.quill` file by default, holding the document and the files it names; `.pen` stays supported and old `.pencraft` bundles still open.

**Architecture:** Two new units. `pen-doc::binary` turns a document into a binary tree of values with each key written once, and back. `bundle::quill` is a container that knows nothing about documents: a 32-byte header, an index, then blocks, each with a CRC-32. `bundle.rs` keeps its working folder and swaps the zip writer for the new container; the zip reader stays for old files.

**Tech Stack:** Rust (edition 2024), serde_json, flate2 (deflate and CRC-32, already in use). No new dependency.

**Spec:** `docs/superpowers/specs/2026-11-21-quill-file-design.md`. This plan is step 1 of the four it lists. Chat in the file, Import Image copying, and crash recovery keeping pictures each get their own plan.

**House rules (read `app/AGENTS.md` first):**

- All commands run from `app/`.
- No `unwrap`/`expect`/`panic!` outside tests, no `#[allow]`, `///` on every public item, `// Human:` / `// Agent:` comments on non-trivial code.
- Clippy runs pedantic with warnings as errors. Do not use `as` to convert between number types: use `try_from`, `from` and `to_le_bytes` as the code below does.
- An item nothing uses yet fails clippy's dead-code check. Until Task 4 wires the container in, verify each task with its `cargo test` filter; run the full gate only in Task 9.
- **Do not commit.** The project rule is that nothing is committed without the user asking. There are no commit steps in this plan.

## File map

| File | Responsibility |
|---|---|
| `crates/pen-doc/src/binary.rs` (new) | A document, or any JSON value, as binary with a key table, and back |
| `crates/pen-doc/src/lib.rs` | Declares the module |
| `src/bundle/quill.rs` (new) | The container: header, index, blocks, CRC-32, deflate, one copy of equal blocks |
| `src/bundle/zip.rs` | The reader stays; the writer is kept for tests only |
| `src/bundle.rs` | Endings, which ending Save offers, `pack` / `open` / `peek` through the container |
| `src/editor.rs` | Opens by the file's first bytes; refuses to write `.pencraft` |
| `src/app.rs` | `.quill` by default; Save on a `.pencraft` writes a `.quill` beside it |
| `src/menus.rs`, `src/palette.rs` | The item's words |
| `src/app/tools.rs`, `src/agent.rs` | `save_document` |
| `src/app/pictures.rs` | Its bundle test reads the new format |
| `installer/pencraft.iss` | The `.quill` association |
| `MECHANICS.md`, `mechanics.json`, `AI.md`, `../README.md`, `../CHANGELOG.md` | The rules and words |

---

### Task 1: The document as binary (`pen-doc::binary`)

**Files:**
- Create: `crates/pen-doc/src/binary.rs`
- Modify: `crates/pen-doc/src/lib.rs:12-22` (module list and the list in the file's opening comment)

- [ ] **Step 1: Declare the module**

In `crates/pen-doc/src/lib.rs`, add before `pub mod color;`:

```rust
pub mod binary;
```

and add this line to the list in the opening comment, after the `[`model`]` line:

```rust
//! - [`binary`]: the same tree as binary values, each key written once: what a `.quill` file holds.
```

- [ ] **Step 2: Write the file with its tests and no bodies yet**

Create `crates/pen-doc/src/binary.rs`:

```rust
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
/// How deep lists and maps may lie in each other, as deep as the JSON reader goes.
const DEEPEST: usize = 128;

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
    let _ = value;
    Err(bad("not written yet"))
}

/// Reads a JSON value from binary.
///
/// # Errors
/// When the bytes aren't this encoding, or are cut off or damaged.
pub fn read_value(bytes: &[u8]) -> Result<Value, BinaryError> {
    let _ = bytes;
    Err(bad("not written yet"))
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
}
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p pen-doc binary`
Expected: 4 tests, all FAIL (the first with `not written yet`).

- [ ] **Step 4: Write the writer**

Replace the body of `write_value` and add what it uses, below it:

```rust
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
    /// Takes in the keys of `value`, which lies `depth` lists and maps deep.
    fn gather(&mut self, value: &'a Value, depth: usize) -> Result<(), BinaryError> {
        if depth > DEEPEST {
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
```

- [ ] **Step 5: Write the reader**

Replace the body of `read_value` and add what it uses, below it:

```rust
pub fn read_value(bytes: &[u8]) -> Result<Value, BinaryError> {
    let mut reader = Reader { bytes, at: 0, keys: Vec::new() };
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
    fn count(&mut self, each: usize) -> Result<usize, BinaryError> {
        let count = usize::try_from(self.varint()?).ok();
        let left = self.bytes.len() - self.at;
        count.filter(|count| count.checked_mul(each).is_some_and(|least| least <= left)).ok_or_else(|| bad("it says it holds more than it does"))
    }

    fn text(&mut self) -> Result<String, BinaryError> {
        let length = self.count(1)?;
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| bad("a text in it isn't UTF-8"))
    }

    /// The value that starts here, `depth` lists and maps deep.
    fn value(&mut self, depth: usize) -> Result<Value, BinaryError> {
        if depth > DEEPEST {
            return Err(bad("it is nested too deeply"));
        }
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
                let count = self.count(1)?;
                let mut list = Vec::with_capacity(count);
                for _ in 0..count {
                    list.push(self.value(depth + 1)?);
                }
                Ok(Value::Array(list))
            }
            MAP => {
                let count = self.count(2)?;
                let mut map = Map::new();
                for _ in 0..count {
                    let number = usize::try_from(self.varint()?).ok();
                    let key = number.and_then(|number| self.keys.get(number)).ok_or_else(|| bad("a key isn't in its table"))?.clone();
                    let inner = self.value(depth + 1)?;
                    map.insert(key, inner);
                }
                Ok(Value::Object(map))
            }
            _ => Err(bad("it holds a kind of value this version doesn't know")),
        }
    }
}
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test -p pen-doc binary`
Expected: 4 passed.

If `plain_values_go_there_and_back` fails on `json!(-0.0)`, the two sides differ only in how they print; keep the first `assert_eq!` (values) and compare the float's bits instead of its text for that one value.

- [ ] **Step 7: Run the crate's own lints**

Run: `cargo clippy -p pen-doc --all-targets -- -D warnings`
Expected: no warnings. Fix what it names in `binary.rs` only.

---

### Task 2: The container (`bundle::quill`), writing and reading whole files

**Files:**
- Create: `src/bundle/quill.rs`
- Modify: `src/bundle.rs:19` (module list)

- [ ] **Step 1: Declare the module**

In `src/bundle.rs`, add before `pub mod zip;`:

```rust
pub mod quill;
```

- [ ] **Step 2: Write the file with its tests and a writer that fails**

Create `src/bundle/quill.rs`:

```rust
//! The container of a `.quill` file: a header, an index, then blocks.
//!
//! All numbers are little-endian.
//!
//! - Header, 32 bytes: the magic (8), the layout number (4), the number of index entries (4), the index's
//!   length (8), the CRC-32 of the index (4), zero (4).
//! - Index entry: kind (1), packing (1), the name's length (2), where the block starts from the start of the
//!   file (8), its length as stored (8), its length unpacked (8), the CRC-32 of the block as stored (4), the
//!   name (UTF-8).
//! - Blocks: each entry's bytes, stored or deflated. Entries with the same bytes share one block.
// Human: it knows nothing about designs: it takes and gives entries (a kind, a name, bytes). What the kinds mean
// and what happens when one is damaged is `bundle`'s to say.
// Agent: no I/O except [`document`], which reads from the file it is handed. Bytes from outside never panic:
// every place and length is checked against the file's length before anything is read or set aside.

use std::borrow::Cow;
use std::io::{Read, Seek, SeekFrom, Write};

use flate2::Compression;
use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;

/// What every `.quill` file starts with. The first byte is no letter and the last two are a line's end as
/// Windows writes it, so a file that went through something that treats it as text is noticed.
pub const MAGIC: [u8; 8] = [0x89, b'Q', b'U', b'I', b'L', b'L', 0x0D, 0x0A];
/// The number of the layout this version writes.
pub const LAYOUT: u32 = 1;
/// The kind of the entry that holds the document.
pub const DOCUMENT: u8 = 1;
/// The kind of an entry that holds a file the document names. (`2` is kept for the chat.)
pub const FILE: u8 = 3;

/// The length of the header.
const HEADER: usize = 32;
/// The length of an index entry before its name.
const SLOT: usize = 32;
const STORED: u8 = 0;
const DEFLATED: u8 = 1;
/// Endings of files that are compressed already; deflating them again costs time and gains nothing.
const PACKED: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

/// One part of a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// What it is: [`DOCUMENT`] or [`FILE`].
    pub kind: u8,
    /// A file's path as the document names it, parts joined by `/`; empty for the document.
    pub name: String,
    /// What it holds.
    pub bytes: Vec<u8>,
}

/// What was read from a file.
#[derive(Debug, PartialEq, Eq)]
pub struct Unpacked {
    /// The entries of the kinds this version knows, in the file's order.
    pub entries: Vec<Entry>,
    /// The kind and name of each entry whose block doesn't match its checksum or can't be unpacked.
    pub damaged: Vec<(u8, String)>,
    /// The file's layout number, when it is higher than this version writes or the file holds a kind of entry
    /// this version doesn't know.
    pub newer: Option<u32>,
}

/// Whether `bytes` start as a `.quill` file does.
#[must_use]
pub fn is_quill(bytes: &[u8]) -> bool {
    bytes.starts_with(&MAGIC)
}

fn checksum(bytes: &[u8]) -> u32 {
    let mut sum = flate2::Crc::new();
    sum.update(bytes);
    sum.sum()
}

fn wide(length: usize) -> u64 {
    u64::try_from(length).unwrap_or(u64::MAX)
}

fn damaged() -> String {
    "it is cut off or damaged".to_owned()
}

/// Writes `entries` as a `.quill` file.
///
/// # Errors
/// A sentence when there are more entries, or a longer name, than the format holds.
pub fn write(entries: &[Entry]) -> Result<Vec<u8>, String> {
    let _ = entries;
    Err("not written yet".to_owned())
}

/// Reads the `.quill` file `bytes`. Stops when its entries come to more than `most` bytes unpacked.
///
/// # Errors
/// A sentence when it isn't a `.quill` file, its header or index is damaged, a block lies outside the file,
/// or it holds more than `most` bytes.
pub fn read(bytes: &[u8], most: u64) -> Result<Unpacked, String> {
    let _ = (bytes, most);
    Err("not written yet".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: u8, name: &str, bytes: &[u8]) -> Entry {
        Entry { kind, name: name.to_owned(), bytes: bytes.to_vec() }
    }

    /// A document that deflates well, a picture, a shader file and an empty file.
    fn sample() -> Vec<Entry> {
        vec![
            entry(DOCUMENT, "", "a line that repeats. ".repeat(200).as_bytes()),
            entry(FILE, "fx/glow.glsl", b"void main(){}"),
            entry(FILE, "empty.txt", b""),
            entry(FILE, "images/ä b.png", &[7u8; 300]),
        ]
    }

    /// `file` with `change` made to its index and the index's checksum put right again.
    fn reindexed(mut file: Vec<u8>, change: impl FnOnce(&mut [u8])) -> Vec<u8> {
        let length = usize::try_from(u64::from_le_bytes(file[16..24].try_into().unwrap())).unwrap();
        change(&mut file[HEADER..HEADER + length]);
        let sum = checksum(&file[HEADER..HEADER + length]);
        file[24..28].copy_from_slice(&sum.to_le_bytes());
        file
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let entries = sample();
        let file = write(&entries).unwrap();
        assert!(is_quill(&file) && !is_quill(b"PK\x03\x04") && !is_quill(&MAGIC[..7]));
        assert_eq!(read(&file, 1 << 20).unwrap(), Unpacked { entries: entries.clone(), damaged: Vec::new(), newer: None });
        assert_eq!(write(&entries).unwrap(), file, "the same entries give the same bytes");
        // The text is deflated; the picture lies in the file as it is, at its end.
        assert!(file.len() < entries[0].bytes.len());
        assert!(file.ends_with(&[7u8; 300]));
        let none = write(&[]).unwrap();
        assert_eq!((none.len(), read(&none, 0).unwrap().entries.len()), (HEADER, 0));
    }

    #[test]
    fn files_with_the_same_bytes_take_one_block() {
        let mut entries = sample();
        let once = write(&entries).unwrap().len();
        entries.push(entry(FILE, "images/again.png", &[7u8; 300]));
        let file = write(&entries).unwrap();
        assert_eq!(file.len(), once + SLOT + "images/again.png".len());
        assert_eq!(read(&file, 1 << 20).unwrap().entries, entries);
        // The same bytes under a name that is deflated are a block of their own.
        entries.push(entry(FILE, "images/again.dat", &[7u8; 300]));
        assert!(write(&entries).unwrap().len() > file.len() + SLOT + "images/again.dat".len());
    }

    #[test]
    fn a_damaged_block_is_named_and_the_rest_reads() {
        let entries = sample();
        let mut file = write(&entries).unwrap();
        let last = file.len() - 1;
        file[last] ^= 0x55;
        let unpacked = read(&file, 1 << 20).unwrap();
        assert_eq!(unpacked.entries, entries[..3]);
        assert_eq!(unpacked.damaged, [(FILE, "images/ä b.png".to_owned())]);
        // A way of packing this version doesn't know is the same to it as damage.
        let odd = reindexed(write(&entries).unwrap(), |index| index[1] = 9);
        assert_eq!(read(&odd, 1 << 20).unwrap().damaged, [(DOCUMENT, String::new())]);
    }

    #[test]
    fn what_is_not_a_whole_quill_file_is_refused() {
        let file = write(&sample()).unwrap();
        assert_eq!(read(b"PK\x03\x04 and more bytes than a header has, to be sure of it", 100).unwrap_err(), "it is not a .quill file");
        assert_eq!(read(&file[..20], 100).unwrap_err(), "it is cut off or damaged");
        assert_eq!(read(&file[..40], 1 << 20).unwrap_err(), "it is cut off or damaged");
        assert_eq!(read(&file[..file.len() - 10], 1 << 20).unwrap_err(), "it is cut off or damaged");
        // A flipped byte in the index.
        let mut flipped = file.clone();
        flipped[HEADER + 5] ^= 0x55;
        assert_eq!(read(&flipped, 1 << 20).unwrap_err(), "its index is damaged");
        // A block that starts past the end, and one longer than the file, in an index whose checksum is right.
        let far = reindexed(file.clone(), |index| index[4..12].copy_from_slice(&u64::MAX.to_le_bytes()));
        assert_eq!(read(&far, 1 << 20).unwrap_err(), "it is cut off or damaged");
        let long = reindexed(file.clone(), |index| index[12..20].copy_from_slice(&1_000_000u64.to_le_bytes()));
        assert_eq!(read(&long, 1 << 20).unwrap_err(), "it is cut off or damaged");
        // More entries than the index has room for.
        let mut many = file.clone();
        many[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(read(&many, 1 << 20).unwrap_err(), "it is cut off or damaged");
        // More bytes unpacked than asked for.
        assert_eq!(read(&file, 4_512).unwrap_err(), "it unpacks to more than this version opens");
        assert!(read(&file, 4_513).is_ok());
    }

    #[test]
    fn a_newer_file_reads_with_what_this_version_knows() {
        let entries = sample();
        let mut later = write(&entries).unwrap();
        later[8..12].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(read(&later, 1 << 20).unwrap().newer, Some(2));
        // The second entry is of a kind this version has no name for.
        let odd = reindexed(write(&entries).unwrap(), |index| index[SLOT] = 9);
        let unpacked = read(&odd, 1 << 20).unwrap();
        assert_eq!((unpacked.newer, unpacked.entries.len(), unpacked.damaged.len()), (Some(LAYOUT), 3, 0));
    }
}
```

The numbers in the last lines of `what_is_not_a_whole_quill_file_is_refused` are the sample's unpacked lengths together: 4 200 + 13 + 0 + 300 = 4 513.

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test bundle::quill`
Expected: 5 tests, all FAIL with `not written yet`.

- [ ] **Step 4: Write the writer**

Replace the body of `write` and add what it uses, below it:

```rust
pub fn write(entries: &[Entry]) -> Result<Vec<u8>, String> {
    let count = u32::try_from(entries.len()).map_err(|_| "the design would hold more files than this format can".to_owned())?;
    let index_length: usize = entries.iter().map(|entry| SLOT + entry.name.len()).sum();
    // Human: each entry's bytes as they are stored. One whose stored bytes are a block that is there already
    // points at that block.
    let mut blocks: Vec<Block<'_>> = Vec::new();
    let mut placed = Vec::with_capacity(entries.len());
    let mut end = HEADER + index_length;
    for entry in entries {
        let (packing, data) = packed(entry);
        let sum = checksum(&data);
        let same = blocks.iter().position(|block| block.packing == packing && block.sum == sum && block.data == data);
        placed.push(same.unwrap_or(blocks.len()));
        if same.is_none() {
            let start = end;
            end += data.len();
            blocks.push(Block { packing, sum, start, unpacked: entry.bytes.len(), data });
        }
    }
    let mut index = Vec::with_capacity(index_length);
    for (entry, at) in entries.iter().zip(&placed) {
        let Some(block) = blocks.get(*at) else { continue };
        let name_length = u16::try_from(entry.name.len()).map_err(|_| format!("the file name {} is too long", entry.name))?;
        index.push(entry.kind);
        index.push(block.packing);
        index.extend(name_length.to_le_bytes());
        index.extend(wide(block.start).to_le_bytes());
        index.extend(wide(block.data.len()).to_le_bytes());
        index.extend(wide(block.unpacked).to_le_bytes());
        index.extend(block.sum.to_le_bytes());
        index.extend(entry.name.as_bytes());
    }
    let mut out = Vec::with_capacity(end);
    out.extend(MAGIC);
    out.extend(LAYOUT.to_le_bytes());
    out.extend(count.to_le_bytes());
    out.extend(wide(index.len()).to_le_bytes());
    out.extend(checksum(&index).to_le_bytes());
    out.extend([0u8; 4]);
    out.extend(index);
    for block in &blocks {
        out.extend(&*block.data);
    }
    Ok(out)
}

/// A block as it will lie in the file.
struct Block<'a> {
    packing: u8,
    /// The CRC-32 of `data`.
    sum: u32,
    /// Where it starts, from the start of the file.
    start: usize,
    /// How long the entry's bytes are unpacked.
    unpacked: usize,
    data: Cow<'a, [u8]>,
}

/// How an entry is kept, and its bytes that way: deflated when that is smaller, else as it is.
fn packed(entry: &Entry) -> (u8, Cow<'_, [u8]>) {
    let ending = entry.name.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    if PACKED.contains(&ending.as_str()) {
        return (STORED, entry.bytes.as_slice().into());
    }
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    match encoder.write_all(&entry.bytes).and_then(|()| encoder.finish()) {
        Ok(small) if small.len() < entry.bytes.len() => (DEFLATED, small.into()),
        _ => (STORED, entry.bytes.as_slice().into()),
    }
}
```

- [ ] **Step 5: Write the reader**

Replace the body of `read` and add what it uses, below it:

```rust
pub fn read(bytes: &[u8], most: u64) -> Result<Unpacked, String> {
    let head = head(bytes)?;
    let index = usize::try_from(head.index).ok().and_then(|length| HEADER.checked_add(length)).and_then(|end| bytes.get(HEADER..end));
    let slots = slots(index.ok_or_else(damaged)?, &head, wide(bytes.len()))?;
    let (mut entries, mut broken, mut unknown, mut left) = (Vec::new(), Vec::new(), false, most);
    for slot in slots {
        if slot.kind != DOCUMENT && slot.kind != FILE {
            unknown = true;
            continue;
        }
        left = left.checked_sub(slot.unpacked).ok_or_else(|| "it unpacks to more than this version opens".to_owned())?;
        let data = usize::try_from(slot.start).ok().zip(usize::try_from(slot.stored).ok()).and_then(|(from, length)| bytes.get(from..from.checked_add(length)?));
        match data.and_then(|data| unpack(&slot, data)) {
            Some(bytes) => entries.push(Entry { kind: slot.kind, name: slot.name, bytes }),
            None => broken.push((slot.kind, slot.name)),
        }
    }
    Ok(Unpacked { entries, damaged: broken, newer: (head.layout > LAYOUT || unknown).then_some(head.layout) })
}

/// What a file's header says.
struct Head {
    layout: u32,
    /// The number of index entries.
    count: u32,
    /// The index's length in bytes.
    index: u64,
    /// The CRC-32 of the index.
    sum: u32,
}

/// One index entry.
struct Slot {
    kind: u8,
    packing: u8,
    name: String,
    /// Where its block starts, from the start of the file.
    start: u64,
    /// How long its block is as stored, and unpacked.
    stored: u64,
    unpacked: u64,
    /// The CRC-32 of the block as stored.
    sum: u32,
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, String> {
    bytes.get(at..at + 2).and_then(|raw| raw.try_into().ok()).map(u16::from_le_bytes).ok_or_else(damaged)
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    bytes.get(at..at + 4).and_then(|raw| raw.try_into().ok()).map(u32::from_le_bytes).ok_or_else(damaged)
}

fn u64_at(bytes: &[u8], at: usize) -> Result<u64, String> {
    bytes.get(at..at + 8).and_then(|raw| raw.try_into().ok()).map(u64::from_le_bytes).ok_or_else(damaged)
}

/// The header at the start of `bytes`.
fn head(bytes: &[u8]) -> Result<Head, String> {
    if !is_quill(bytes) && bytes.len() >= MAGIC.len() {
        return Err("it is not a .quill file".to_owned());
    }
    let header = bytes.get(..HEADER).ok_or_else(damaged)?;
    Ok(Head { layout: u32_at(header, 8)?, count: u32_at(header, 12)?, index: u64_at(header, 16)?, sum: u32_at(header, 24)? })
}

/// The entries of the index `index`, in a file `length` bytes long.
// Human: the index is believed only once its checksum is right, and then still not about where blocks lie: a
// block outside the file refuses the file.
fn slots(index: &[u8], head: &Head, length: u64) -> Result<Vec<Slot>, String> {
    let count = usize::try_from(head.count).ok().filter(|count| count.checked_mul(SLOT).is_some_and(|least| least <= index.len()));
    let count = count.ok_or_else(damaged)?;
    if checksum(index) != head.sum {
        return Err("its index is damaged".to_owned());
    }
    let mut slots = Vec::with_capacity(count);
    let mut at = 0usize;
    for _ in 0..count {
        let fixed = at.checked_add(SLOT).and_then(|end| index.get(at..end)).ok_or_else(damaged)?;
        let name_length = usize::from(u16_at(fixed, 2)?);
        let (start, stored, unpacked, sum) = (u64_at(fixed, 4)?, u64_at(fixed, 12)?, u64_at(fixed, 20)?, u32_at(fixed, 28)?);
        let name = index.get(at + SLOT..at + SLOT + name_length).ok_or_else(damaged)?;
        let name = String::from_utf8(name.to_vec()).map_err(|_| "a file name in it isn't UTF-8".to_owned())?;
        if start.checked_add(stored).is_none_or(|end| end > length) {
            return Err(damaged());
        }
        let (kind, packing) = (fixed.first().copied().unwrap_or_default(), fixed.get(1).copied().unwrap_or_default());
        slots.push(Slot { kind, packing, name, start, stored, unpacked, sum });
        at += SLOT + name_length;
    }
    Ok(slots)
}

/// The bytes of the block `data` unpacked; `None` when it doesn't match its checksum, is packed in a way this
/// version doesn't know, or isn't as long unpacked as its entry says.
fn unpack(slot: &Slot, data: &[u8]) -> Option<Vec<u8>> {
    if checksum(data) != slot.sum {
        return None;
    }
    let unpacked = match slot.packing {
        STORED => data.to_vec(),
        DEFLATED => {
            let mut unpacked = Vec::new();
            // One byte more than it says, so a block that is longer than it says is noticed.
            DeflateDecoder::new(data).take(slot.unpacked.saturating_add(1)).read_to_end(&mut unpacked).ok()?;
            unpacked
        }
        _ => return None,
    };
    (wide(unpacked.len()) == slot.unpacked).then_some(unpacked)
}
```

Two notes on the tests this must satisfy. `read(&file[..20], …)` starts with the magic but is shorter than a header, so `head` gives "it is cut off or damaged". The flipped index byte is caught by the checksum, which is why `slots` checks the count against the index's length first (so a wild count never sets memory aside) and the checksum second.

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test bundle::quill`
Expected: 5 passed. `Seek` and `SeekFrom` are imported and not used yet; that warning goes in Task 3.

---

### Task 3: Reading only the document from a file

The home screen draws a small picture of every recent design. It must not read a design's pictures for that.

**Files:**
- Modify: `src/bundle/quill.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module of `src/bundle/quill.rs`:

```rust
    /// A reader that counts the bytes read through it.
    struct Counted<R> {
        inner: R,
        read: usize,
    }

    impl<R: Read> Read for Counted<R> {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let read = self.inner.read(buffer)?;
            self.read += read;
            Ok(read)
        }
    }

    impl<R: Seek> Seek for Counted<R> {
        fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(to)
        }
    }

    #[test]
    fn the_document_is_read_without_the_other_blocks() {
        let entries = sample();
        let file = write(&entries).unwrap();
        let mut counted = Counted { inner: std::io::Cursor::new(file.clone()), read: 0 };
        assert_eq!(document(&mut counted, 1 << 20).unwrap(), entries[0].bytes);
        assert!(counted.read < file.len() - 300, "{} of {} bytes were read", counted.read, file.len());
        // A damaged picture doesn't matter to it; a damaged document does.
        let mut spoiled = file.clone();
        let last = spoiled.len() - 1;
        spoiled[last] ^= 0x55;
        assert!(document(&mut std::io::Cursor::new(spoiled), 1 << 20).is_ok());
        let odd = reindexed(file.clone(), |index| index[1] = 9);
        assert_eq!(document(&mut std::io::Cursor::new(odd), 1 << 20).unwrap_err(), "its document is damaged");
        // A document larger than asked for, a file without one, and a file that is none.
        assert_eq!(document(&mut std::io::Cursor::new(file), 100).unwrap_err(), "its document is too large for this");
        let without = write(&entries[1..]).unwrap();
        assert_eq!(document(&mut std::io::Cursor::new(without), 1 << 20).unwrap_err(), "it holds no document");
        assert!(document(&mut std::io::Cursor::new(b"{}".to_vec()), 1 << 20).is_err());
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test bundle::quill`
Expected: does not compile: `cannot find function 'document'`.

- [ ] **Step 3: Write `document`**

Add after `read` in `src/bundle/quill.rs`:

```rust
/// The document in the `.quill` file `file`, unpacked, without reading any other block. Refuses an index or a
/// document longer than `most` bytes.
///
/// # Errors
/// A sentence when it isn't a `.quill` file, is cut off or damaged, holds no document, or the document is
/// larger than `most`.
// Agent: READS the header, the index and one block from `file`; BLOCKS on them.
pub fn document(file: &mut (impl Read + Seek), most: u64) -> Result<Vec<u8>, String> {
    let told = |error: std::io::Error| if error.kind() == std::io::ErrorKind::UnexpectedEof { damaged() } else { error.to_string() };
    let length = file.seek(SeekFrom::End(0)).map_err(told)?;
    file.seek(SeekFrom::Start(0)).map_err(told)?;
    let mut header = [0u8; HEADER];
    file.read_exact(&mut header).map_err(told)?;
    let head = head(&header)?;
    let too_large = || "its document is too large for this".to_owned();
    let mut index = vec![0u8; usize::try_from(head.index).ok().filter(|_| head.index <= most.min(length)).ok_or_else(too_large)?];
    file.read_exact(&mut index).map_err(told)?;
    let slot = slots(&index, &head, length)?.into_iter().find(|slot| slot.kind == DOCUMENT).ok_or("it holds no document")?;
    if slot.unpacked > most {
        return Err(too_large());
    }
    let mut data = vec![0u8; usize::try_from(slot.stored).map_err(|_| too_large())?];
    file.seek(SeekFrom::Start(slot.start)).map_err(told)?;
    file.read_exact(&mut data).map_err(told)?;
    unpack(&slot, &data).ok_or_else(|| "its document is damaged".to_owned())
}
```

`slots` has already checked that the block lies inside the file, so `slot.stored` is never more than the file's length.

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test bundle::quill`
Expected: 6 passed, and no warning about `Seek`.

---

### Task 4: `bundle.rs` writes and reads `.quill`; the zip writer goes

**Files:**
- Modify: `src/bundle.rs` (whole file except `Work`, `root`, `sweep`, `inside`, `relative`, `named`, `credits_for`, `carry`)
- Modify: `src/bundle/zip.rs:1-107`

- [ ] **Step 1: Keep the zip writer for tests only**

Old bundles are still read, and the tests need old bundles to read. In `src/bundle/zip.rs`:

Replace the opening comment and the imports (lines 1-10) with:

```rust
//! The zip container of an old `.pencraft` bundle: files by name, deflated or stored, read in memory.
//!
//! Only what a bundle needed: no zip64 (4 GB and 65 535 files at most), no encryption, no multi-part archives.
//! Archives other programs made read too, when they stay inside that (Finder, Explorer, `zip`).
// Human: bundles are written as `.quill` files now (`super::quill`). This reader stays so the bundles people
// already have still open; the writer is here for the tests, which need old bundles to read.

use std::io::Read;
#[cfg(test)]
use std::io::Write;

#[cfg(test)]
use flate2::Compression;
use flate2::read::DeflateDecoder;
#[cfg(test)]
use flate2::write::DeflateEncoder;
```

Put `#[cfg(test)]` on its own line directly above each of these items: `const VERSION`, `const UTF8`, `const DAY`, `const PACKED`, `pub fn write`, `fn packed`. Each keeps its `///` comment above the attribute. Nothing else in the file changes.

- [ ] **Step 2: Replace the top of `src/bundle.rs`**

Replace lines 1-52 (the opening comment through `is_design`) with:

```rust
//! A `.quill` file: one file that holds a document and the files it names.
//!
//! The document as binary (`pen_doc::binary`) and, by the paths the document names them by, its pictures,
//! shader files and imported documents, with the pictures' credits, in the container of [`quill`]. An open
//! file lies unpacked in a working folder; the document's relative paths start there, so everything that reads
//! a picture or a shader beside a `.pen` file reads a `.quill` file's the same way. Saving packs the document
//! and the files it names, and nothing else of the folder.
//!
//! "Bundle" in this module's names means such a file. The bundles of earlier versions (`.pencraft`, a zip
//! archive) still open and are no longer written.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use pen_doc::Document;
use serde_json::Value;

use crate::imagery::store::{CREDITS, FOLDER};

pub mod quill;
pub mod zip;

/// The ending of a bundle's file name.
pub const ENDING: &str = "quill";
/// The ending of the bundles earlier versions wrote.
pub const OLD: &str = "pencraft";
/// The ending of a plain document's file name.
pub const PLAIN: &str = "pen";
/// The document's name in a working folder, and inside an old bundle.
pub const DOCUMENT: &str = "document.pen";
/// The file in an old bundle that says what it is.
const MANIFEST: &str = "manifest.json";
/// The number of the layout the old bundles had.
const OLD_LAYOUT: u64 = 1;
/// The file in a working folder that the app using it keeps locked.
const HELD: &str = ".held";
/// The most a bundle may unpack to.
const MOST_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// The largest old bundle [`peek`] reads, and the largest document it takes out of any bundle.
const MOST_PEEKED: u64 = 64 * 1024 * 1024;
/// A working folder is left alone this long after it was made: it may not be locked yet.
const SETTLED_AFTER: std::time::Duration = std::time::Duration::from_mins(1);
/// A working folder without its lock file is left alone this long.
const UNHELD_FOR: std::time::Duration = std::time::Duration::from_hours(24);

fn ends(path: &Path, ending: &str) -> bool {
    path.extension().is_some_and(|has| has.eq_ignore_ascii_case(ending))
}

/// Whether `path` names a bundle, new or old, by its ending.
#[must_use]
pub fn is_bundle(path: &Path) -> bool {
    ends(path, ENDING) || ends(path, OLD)
}

/// Whether `path` names a bundle of the kind earlier versions wrote, which this one only reads.
#[must_use]
pub fn is_old(path: &Path) -> bool {
    ends(path, OLD)
}

/// Whether `path` names a file Pencraft opens as a design: a `.pen` document or a bundle.
#[must_use]
pub fn is_design(path: &Path) -> bool {
    is_bundle(path) || ends(path, PLAIN)
}

/// Where the old bundle at `path` is saved from now on: the same name ending `.quill`. `None` for any other file.
#[must_use]
pub fn successor(path: &Path) -> Option<PathBuf> {
    is_old(path).then(|| path.with_extension(ENDING))
}

/// `path` with the ending `.quill` when it has none.
#[must_use]
pub fn ended(path: PathBuf) -> PathBuf {
    if path.extension().is_some() { path } else { path.with_extension(ENDING) }
}

/// The ending Save As offers for a document saved at `path` (`None` for one without a file), and both endings
/// with that one first; `other` asks for the format the document doesn't have.
// Human: a new document is a `.quill` file unless the user says otherwise; a document keeps the format it has.
#[must_use]
pub fn offered(path: Option<&Path>, other: bool) -> [&'static str; 2] {
    if path.is_none_or(is_bundle) == other { [PLAIN, ENDING] } else { [ENDING, PLAIN] }
}

/// Whether `bytes` start as a bundle does, new or old.
#[must_use]
pub fn looks_packed(bytes: &[u8]) -> bool {
    quill::is_quill(bytes) || bytes.starts_with(b"PK")
}
```

- [ ] **Step 3: Replace `Opened`, `open` and `peek`**

Replace from `/// A bundle that was opened.` through the end of `peek` (old lines 136-190) with:

```rust
/// A bundle that was opened.
#[derive(Debug)]
pub struct Opened {
    /// Its document.
    pub doc: Document,
    /// The folder its files lie in.
    pub work: Work,
    /// The number of the bundle's layout, when it is higher than this version writes.
    pub newer: Option<u64>,
    /// A sentence for each file in it that was damaged and left out.
    pub told: Vec<String>,
}

/// What a bundle holds, read into memory.
struct Parts {
    doc: Document,
    /// Its files by their path inside it.
    files: Vec<(String, Vec<u8>)>,
    newer: Option<u64>,
    told: Vec<String>,
}

/// Opens the bundle at `path`: unpacks it into a new working folder and reads its document.
///
/// # Errors
/// A sentence when the file can't be read, isn't a bundle, its document is damaged, or it names a file outside
/// itself.
// Human: what the file is, is read from its first bytes: an old bundle that was given the new ending opens too.
// Agent: READS `path` whole into memory; WRITES the working folder. BLOCKS on both.
pub fn open(path: &Path) -> Result<Opened, String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let parts = if quill::is_quill(&bytes) {
        from_quill(&bytes)?
    } else if bytes.starts_with(b"PK") {
        from_zip(&bytes)?
    } else {
        return Err("it is not a design Pencraft packed".to_owned());
    };
    drop(bytes);
    let Parts { doc, files, newer, told } = parts;
    let work = Work::fresh()?;
    let unpacked = files.iter().filter(|(name, _)| name != MANIFEST && name != HELD).try_for_each(|(name, bytes)| {
        let file = work.folder.join(name);
        file.parent().map_or(Ok(()), std::fs::create_dir_all)?;
        std::fs::write(file, bytes)
    });
    match unpacked {
        Ok(()) => Ok(Opened { doc, work, newer, told }),
        Err(error) => {
            let why = format!("it couldn't be unpacked to {}: {error}", work.folder.display());
            work.discard();
            Err(why)
        }
    }
}

/// The parts of the `.quill` file `bytes`.
// Human: a damaged document refuses the file. A damaged picture is left out and said, and the rest opens: the
// design is worth more than the one picture.
fn from_quill(bytes: &[u8]) -> Result<Parts, String> {
    let unpacked = quill::read(bytes, MOST_BYTES)?;
    if unpacked.damaged.iter().any(|(kind, _)| *kind == quill::DOCUMENT) {
        return Err("its document is damaged".to_owned());
    }
    let (mut doc, mut files) = (None, Vec::new());
    for entry in unpacked.entries {
        if entry.kind == quill::DOCUMENT {
            doc = Some(pen_doc::binary::read(&entry.bytes).map_err(|error| error.to_string())?);
        } else {
            let plain = relative(&entry.name).ok_or_else(|| format!("it names a file outside itself ({})", entry.name))?;
            files.push((plain, entry.bytes));
        }
    }
    let told = unpacked.damaged.iter().map(|(_, name)| format!("The file {name} in it is damaged and was left out.")).collect();
    Ok(Parts { doc: doc.ok_or("it holds no document")?, files, newer: unpacked.newer.map(u64::from), told })
}

/// The parts of the old bundle `bytes`, a zip archive.
fn from_zip(bytes: &[u8]) -> Result<Parts, String> {
    let files = inside(zip::read(bytes, MOST_BYTES, &|_| true)?)?;
    let text = files.iter().find(|(name, _)| name == DOCUMENT).map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned());
    let doc = pen_doc::read(&text.ok_or("it holds no document")?).map_err(|error| error.to_string())?;
    let manifest = files.iter().find(|(name, _)| name == MANIFEST).and_then(|(_, bytes)| serde_json::from_slice::<Value>(bytes).ok());
    let newer = manifest.as_ref().and_then(|manifest| manifest.get("version")).and_then(Value::as_u64).filter(|layout| *layout > OLD_LAYOUT);
    Ok(Parts { doc, files, newer, told: Vec::new() })
}

/// The document in the bundle at `path`, without unpacking anything else; `None` when it can't be read or is
/// too large to look into for a small picture.
// Agent: READS the header, the index and the document of a `.quill` file; an old bundle whole into memory.
// BLOCKS on it.
#[must_use]
pub fn peek(path: &Path) -> Option<Document> {
    let mut file = File::open(path).ok()?;
    let mut first = [0u8; 8];
    if file.read_exact(&mut first).is_ok() && quill::is_quill(&first) {
        return pen_doc::binary::read(&quill::document(&mut file, MOST_PEEKED).ok()?).ok();
    }
    drop(file);
    if std::fs::metadata(path).ok()?.len() > MOST_PEEKED {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let entries = zip::read(&bytes, MOST_PEEKED, &|name| name.rsplit('/').next() == Some(DOCUMENT)).ok()?;
    let files = inside(entries).ok()?;
    let (_, text) = files.iter().find(|(name, _)| name == DOCUMENT)?;
    pen_doc::read(&String::from_utf8_lossy(text)).ok()
}
```

`inside` and `relative` below it stay as they are.

- [ ] **Step 4: Replace `pack` and adjust `credits_for`**

Replace the `pack` function (its comments stay, with "bundle" where they say it) with:

```rust
pub fn pack(doc: &Document, folder: &Path) -> Result<Packed, String> {
    let entry = |name: &str, bytes: Vec<u8>| quill::Entry { kind: quill::FILE, name: name.to_owned(), bytes };
    let document = pen_doc::binary::write(doc).map_err(|error| error.to_string())?;
    let mut entries = vec![quill::Entry { kind: quill::DOCUMENT, name: String::new(), bytes: document }];
    let mut missing = Vec::new();
    for name in named(doc).into_iter().filter(|name| name != MANIFEST && name != DOCUMENT) {
        let file = folder.join(&name);
        if file.is_file() {
            entries.push(entry(&name, std::fs::read(&file).map_err(|error| format!("{}: {error}", file.display()))?));
        } else {
            missing.push(name);
        }
    }
    let credits = format!("{FOLDER}/{CREDITS}");
    if let Some(credits_of) = credits_for(&folder.join(&credits), &entries)
        && entries.iter().all(|entry| entry.name != credits)
    {
        entries.push(entry(&credits, credits_of));
    }
    Ok(Packed { bytes: quill::write(&entries)?, missing })
}
```

In `credits_for`, change the parameter type from `&[zip::Entry]` to `&[quill::Entry]`; its body stays. In its last line, `serde_json::to_vec_pretty(&credits)` stays.

- [ ] **Step 5: Replace the tests of `src/bundle.rs`**

Replace the whole `#[cfg(test)] mod tests { … }` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("pencraft-bundle-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("images")).unwrap();
        folder
    }

    const DESIGN: &str = r#"{"version":"2.22","imports":{"kit":"./kit/base.pen","far":"/shared/far.pen"},"children":[
        {"type":"rectangle","id":"a","fill":{"type":"image","url":"./images/fox.png"},
         "effect":[{"type":"shader","url":"fx/glow.glsl","uniforms":{"grain":"images/grain.png","back":"pencil:background","amount":0.5}}]},
        {"type":"rectangle","id":"b","fill":[{"type":"image","url":"https://example.com/a.png"},{"type":"image","url":"../outside.png"},{"type":"image","url":"/whole/path.png"},{"type":"image","url":"images/fox.png"}]},
        {"type":"ref","id":"i","ref":"a","descendants":{"a":{"fill":{"type":"image","url":"images/gone.png"}}}},
        {"type":"text","id":"t","content":"images/unused.png"}]}"#;

    /// What the picture `images/fox.png` holds: words that are nowhere else in a packed file.
    const FOX: &[u8] = b"the picture of a fox";

    /// A folder with the files [`DESIGN`] names, one it doesn't, and credits for both pictures.
    fn filled(name: &str) -> PathBuf {
        let from = folder(name);
        for (name, bytes) in [
            ("images/fox.png", FOX),
            ("images/grain.png", b"grain"),
            ("images/unused.png", b"unused"),
            ("fx/glow.glsl", b"void main(){}"),
            ("kit/base.pen", b"{}"),
        ] {
            std::fs::create_dir_all(from.join(name).parent().unwrap()).unwrap();
            std::fs::write(from.join(name), bytes).unwrap();
        }
        std::fs::write(from.join("images").join(CREDITS), r#"{"fox.png":{"model":"m"},"unused.png":{"model":"n"}}"#).unwrap();
        from
    }

    #[test]
    fn the_files_a_document_names_inside_its_folder_are_found_once() {
        let doc = pen_doc::read(DESIGN).unwrap();
        assert_eq!(named(&doc), ["fx/glow.glsl", "images/fox.png", "images/gone.png", "images/grain.png", "kit/base.pen"]);
        assert_eq!(relative("./a/./b.png").as_deref(), Some("a/b.png"));
        assert_eq!([relative("../a"), relative("/a"), relative("a/../../b"), relative("C:/a"), relative(r"a\b"), relative(".")], [const { None }; 6]);
    }

    #[test]
    fn a_quill_file_holds_the_document_and_what_it_names_and_opens_again() {
        let from = filled("pack");
        let doc = pen_doc::read(DESIGN).unwrap();
        let packed = pack(&doc, &from).unwrap();
        assert_eq!(packed.missing, ["images/gone.png"]);
        let parts = quill::read(&packed.bytes, MOST_BYTES).unwrap();
        let names: Vec<(u8, &str)> = parts.entries.iter().map(|entry| (entry.kind, entry.name.as_str())).collect();
        assert_eq!(
            names,
            [
                (quill::DOCUMENT, ""),
                (quill::FILE, "fx/glow.glsl"),
                (quill::FILE, "images/fox.png"),
                (quill::FILE, "images/grain.png"),
                (quill::FILE, "kit/base.pen"),
                (quill::FILE, "images/credits.json")
            ]
        );

        let file = from.join("design.quill");
        std::fs::write(&file, &packed.bytes).unwrap();
        assert_eq!(peek(&file).as_ref(), Some(&doc));
        let opened = open(&file).unwrap();
        assert_eq!((opened.doc == doc, opened.newer, opened.told.len()), (true, None, 0));
        let at = opened.work.folder().to_path_buf();
        assert_eq!(std::fs::read(at.join("images/fox.png")).unwrap(), FOX);
        assert_eq!(std::fs::read(at.join("fx/glow.glsl")).unwrap(), b"void main(){}");
        assert!(!at.join("images/unused.png").exists() && !at.join(MANIFEST).exists());
        let credits: Value = serde_json::from_slice(&std::fs::read(at.join("images").join(CREDITS)).unwrap()).unwrap();
        assert_eq!(credits, json!({ "fox.png": { "model": "m" } }));
        // Packed again from where it was unpacked, it is the same file byte for byte.
        assert_eq!(pack(&opened.doc, &at).unwrap(), packed);
        // The folder is held while the file is open, and gone when it is let go.
        sweep();
        assert!(at.is_dir());
        opened.work.discard();
        assert!(!at.exists());
        std::fs::remove_dir_all(&from).unwrap();
    }

    #[test]
    fn a_damaged_picture_is_left_out_and_a_damaged_document_refuses_the_file() {
        let from = filled("damaged");
        let doc = pen_doc::read(DESIGN).unwrap();
        let whole = pack(&doc, &from).unwrap().bytes;
        let file = from.join("design.quill");
        // A picture is stored as it is, so its bytes are found in the file; one of them is flipped.
        let at = whole.windows(FOX.len()).position(|window| window == FOX).unwrap();
        let mut spoiled = whole.clone();
        spoiled[at] ^= 0x55;
        std::fs::write(&file, &spoiled).unwrap();
        let opened = open(&file).unwrap();
        assert_eq!(opened.told, ["The file images/fox.png in it is damaged and was left out."]);
        assert!(!opened.work.folder().join("images/fox.png").exists() && opened.work.folder().join("images/grain.png").is_file());
        assert_eq!(peek(&file).as_ref(), Some(&doc));
        opened.work.discard();
        // The document is the first block, right after the index.
        let index = usize::try_from(u64::from_le_bytes(whole[16..24].try_into().unwrap())).unwrap();
        let mut spoiled = whole.clone();
        spoiled[32 + index] ^= 0x55;
        std::fs::write(&file, &spoiled).unwrap();
        assert_eq!(open(&file).unwrap_err(), "its document is damaged");
        assert_eq!(peek(&file), None);
        // A file that leads outside itself is refused whole.
        let entry = |kind: u8, name: &str, bytes: Vec<u8>| quill::Entry { kind, name: name.to_owned(), bytes };
        let document = pen_doc::binary::write(&doc).unwrap();
        for bad in ["../escape.png", "/etc/escape", "images/../../escape", r"..\escape"] {
            let outside = [entry(quill::DOCUMENT, "", document.clone()), entry(quill::FILE, bad, b"x".to_vec())];
            std::fs::write(&file, quill::write(&outside).unwrap()).unwrap();
            assert!(open(&file).unwrap_err().contains("outside itself"), "{bad}");
        }
        std::fs::write(&file, quill::write(&[entry(quill::FILE, "readme.txt", b"x".to_vec())]).unwrap()).unwrap();
        assert_eq!(open(&file).unwrap_err(), "it holds no document");
        std::fs::write(&file, b"{}").unwrap();
        assert_eq!(open(&file).unwrap_err(), "it is not a design Pencraft packed");
        std::fs::remove_dir_all(&from).unwrap();
    }

    #[test]
    fn an_old_bundle_still_opens_under_either_ending() {
        let from = folder("old");
        let entry = |name: &str, bytes: &[u8]| zip::Entry { name: name.to_owned(), bytes: bytes.to_vec() };
        let design = br#"{"version":"2.22","children":[]}"#;
        // Zipped again inside one folder by the Finder, with a manifest of a later layout.
        let wrapped = [
            entry("design/manifest.json", br#"{"format":"pencraft-bundle","version":7}"#),
            entry("design/document.pen", design),
            entry("design/images/a.png", b"a"),
            entry("__MACOSX/design/._document.pen", b"finder"),
            entry("design/.DS_Store", b"finder"),
        ];
        for name in ["wrapped.pencraft", "renamed.quill"] {
            let file = from.join(name);
            std::fs::write(&file, zip::write(&wrapped).unwrap()).unwrap();
            assert!(peek(&file).is_some(), "{name}");
            let opened = open(&file).unwrap();
            assert_eq!(opened.newer, Some(7), "{name}");
            let at = opened.work.folder().to_path_buf();
            assert!(at.join("images/a.png").is_file() && !at.join("__MACOSX").exists() && !at.join(".DS_Store").exists());
            opened.work.discard();
        }
        let file = from.join("wrapped.pencraft");
        for bad in ["../escape.png", "/etc/escape", "images/../../escape", r"..\escape"] {
            std::fs::write(&file, zip::write(&[entry(DOCUMENT, design), entry(bad, b"x")]).unwrap()).unwrap();
            assert!(open(&file).unwrap_err().contains("outside itself"), "{bad}");
        }
        std::fs::write(&file, zip::write(&[entry("readme.txt", b"x")]).unwrap()).unwrap();
        assert_eq!(open(&file).unwrap_err(), "it holds no document");
        std::fs::remove_dir_all(&from).unwrap();
    }

    #[test]
    fn named_files_are_carried_to_another_folder_once() {
        let (from, to) = (folder("carry-from"), folder("carry-to"));
        std::fs::create_dir_all(from.join("fx")).unwrap();
        std::fs::write(from.join("fx/glow.glsl"), b"new").unwrap();
        std::fs::write(from.join("images/fox.png"), b"fox").unwrap();
        std::fs::write(to.join("images/fox.png"), b"already there").unwrap();
        let doc = pen_doc::read(DESIGN).unwrap();
        assert_eq!(carry(&doc, &from, &to), Vec::<String>::new());
        assert_eq!(std::fs::read(to.join("fx/glow.glsl")).unwrap(), b"new");
        assert_eq!(std::fs::read(to.join("images/fox.png")).unwrap(), b"already there");
        std::fs::remove_dir_all(&from).unwrap();
        std::fs::remove_dir_all(&to).unwrap();
    }

    #[test]
    fn endings_say_what_a_file_is_and_what_save_offers() {
        let (quill, old, pen) = (Path::new("/a/site.QUILL"), Path::new("/a/site.pencraft"), Path::new("/a/site.pen"));
        assert!(is_bundle(quill) && is_bundle(old) && !is_bundle(pen) && !is_bundle(Path::new("/a/quill")));
        assert!(is_old(old) && !is_old(quill) && !is_old(pen));
        assert!(is_design(pen) && is_design(quill) && is_design(old) && !is_design(Path::new("a.png")));
        assert_eq!(successor(old), Some(PathBuf::from("/a/site.quill")));
        assert_eq!((successor(quill), successor(pen)), (None, None));
        assert_eq!(ended(PathBuf::from("/a/site")), PathBuf::from("/a/site.quill"));
        assert_eq!(ended(PathBuf::from("/a/site.pen")), PathBuf::from("/a/site.pen"));
        // A new document is offered `.quill`; a document keeps the format it has; `other` turns it round.
        assert_eq!(offered(None, false), [ENDING, PLAIN]);
        assert_eq!(offered(Some(pen), false), [PLAIN, ENDING]);
        assert_eq!(offered(Some(quill), false), [ENDING, PLAIN]);
        assert_eq!(offered(Some(old), false), [ENDING, PLAIN]);
        assert_eq!(offered(None, true), [PLAIN, ENDING]);
        assert_eq!(offered(Some(pen), true), [ENDING, PLAIN]);
        assert_eq!(offered(Some(quill), true), [PLAIN, ENDING]);
        assert!(looks_packed(&quill::MAGIC) && looks_packed(b"PK\x03\x04") && !looks_packed(b"{\"version\""));
    }
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test bundle::`
Expected: every test of `bundle`, `bundle::quill` and `bundle::zip` passes. The rest of the app still builds: `Opened` kept its fields and gained one. The bundle test in `src/app/pictures.rs` fails until Task 5, because it still reads the file as a zip.

---

### Task 5: The editor opens by first bytes and stops writing `.pencraft`

**Files:**
- Modify: `src/editor.rs:215-233` (`Editor::open`), `src/editor.rs:600-624` (`Editor::save`), the comments at `:115-117`
- Modify: `src/app/pictures.rs:1578-1647` and `:1659` (tests)

- [ ] **Step 1: Replace `Editor::open`**

```rust
    /// An editor for the design file at `path`: a `.pen` document, or a bundle, which is unpacked. With it, a
    /// sentence for what the user should know about the file: a newer Pencraft wrote it and this one doesn't
    /// know all of it, or a file in it was damaged and left out.
    ///
    /// # Errors
    /// A sentence when the file can't be read or isn't a design file.
    // Agent: READS `path`; for a bundle WRITES its working folder. BLOCKS on both.
    pub fn open(path: &Path, seed: u64) -> Result<(Editor, Option<String>), String> {
        let (doc, work, layout, told) = if crate::bundle::is_bundle(path) {
            let opened = crate::bundle::open(path)?;
            (opened.doc, Some(opened.work), opened.newer, opened.told)
        } else {
            let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
            // Human: a packed design under a `.pen` name would be read as text and fail with words about JSON.
            if crate::bundle::looks_packed(&bytes) {
                return Err("it is a packed design: give its name the ending .quill".to_owned());
            }
            let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
            (pen_doc::read(&text).map_err(|e| e.to_string())?, None, None, Vec::new())
        };
        let said: Vec<String> = newer_than_this(&doc, layout).into_iter().chain(told).collect();
        let mut editor = Editor::new(doc, Some(path.to_path_buf()), seed);
        editor.work = work;
        Ok((editor, (!said.is_empty()).then(|| said.join(" "))))
    }
```

- [ ] **Step 2: Replace `Editor::save`**

```rust
    /// Writes the document to its file, crash-safely: a `.pen` file as JSON, a `.quill` file with the files the
    /// document names. Stamps it as Pencraft's first ([`pen_doc::stamp`]). Gives a sentence for what the user
    /// should know about the file that was written.
    ///
    /// # Errors
    /// When there is no path yet, the path is an old `.pencraft` bundle's, the document can't be written, or
    /// the file can't be.
    // Agent: WRITES the file; for a `.quill` file READS the files it names, whole. BLOCKS on both.
    pub fn save(&mut self) -> Result<Vec<String>, String> {
        let Some(path) = self.path.clone() else { return Err("the document has no file yet".to_owned()) };
        // Human: nothing writes the old format any more; whoever saves gives the file the new ending first.
        if crate::bundle::is_old(&path) {
            return Err("a .pencraft file is no longer written: save it as a .quill file".to_owned());
        }
        pen_doc::stamp::apply(&mut self.doc);
        let mut told = Vec::new();
        let bytes = match self.unpacked() {
            Some(work) => {
                let packed = crate::bundle::pack(&self.doc, work.folder())?;
                if !packed.missing.is_empty() {
                    told.push(format!("These files the document names weren't found and aren't in the .quill file: {}.", packed.missing.join(", ")));
                }
                packed.bytes
            }
            None => pen_doc::write(&self.doc).map_err(|e| e.to_string())?.into_bytes(),
        };
        craft_atomic::atomic_write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        self.dirty = false;
        Ok(told)
    }
```

- [ ] **Step 3: Update the bundle test in `src/app/pictures.rs`**

In `a_document_goes_into_a_bundle_and_out_again_with_its_files`, replace the lines from `let bundle = folder.join("site.pencraft");` through the `assert!(written.contains("\"pencraft\": {…` line with:

```rust
        let bundle = folder.join("site.quill");
        let was = editor.move_to(bundle.clone()).unwrap();
        assert_eq!(was, Some(folder.join("site.pen")));
        assert_eq!(settle(&mut editor, was.as_deref(), None), Vec::<String>::new());
        let told = editor.save().unwrap();
        assert!(told.len() == 1 && told[0].contains("images/gone.png"), "{told:?}");
        let files = crate::bundle::quill::read(&std::fs::read(&bundle).unwrap(), 1 << 20).unwrap().entries;
        let names: Vec<&str> = files.iter().map(|file| file.name.as_str()).collect();
        assert_eq!(names, ["", "fx/glow.glsl", "images/a.png", "images/far.png", "images/credits.json"]);
        let written = pen_doc::write(&pen_doc::binary::read(&files[0].bytes).unwrap()).unwrap();
        assert!(written.contains("\"./images/far.png\"") && !written.contains(&far), "{written}");
        assert!(written.contains("\"pencraft\": {\n    \"format\": 1,\n    \"features\": [\n      \"mask\"\n    ]"), "{written}");
```

In `a_file_a_newer_pencraft_wrote_opens_with_a_sentence`, change `missing.pencraft` to `missing.quill`.

- [ ] **Step 4: Add tests for what the editor refuses**

Add to the `tests` module of `src/app/pictures.rs`, after `a_file_a_newer_pencraft_wrote_opens_with_a_sentence`:

```rust
    #[test]
    fn a_pencraft_file_is_not_written_and_a_packed_file_named_pen_is_not_read_as_text() {
        let folder = empty("old-bundle");
        let mut editor = Editor::new(pen_doc::Document::default(), None, 7);
        editor.move_to(folder.join("old.pencraft")).unwrap();
        assert!(editor.save().unwrap_err().contains(".quill"));
        assert!(!folder.join("old.pencraft").exists());
        // Saved as a `.quill` file, then given the wrong ending by hand.
        editor.move_to(folder.join("new.quill")).unwrap();
        editor.save().unwrap();
        editor.close();
        std::fs::copy(folder.join("new.quill"), folder.join("wrong.pen")).unwrap();
        assert!(Editor::open(&folder.join("wrong.pen"), 7).err().is_some_and(|said| said.contains("ending .quill")));
        let (mut again, said) = Editor::open(&folder.join("new.quill"), 7).unwrap();
        assert_eq!((said, again.bundled()), (None, true));
        again.close();
        std::fs::remove_dir_all(&folder).unwrap();
    }
```

- [ ] **Step 5: Update the comments that name the old format**

- `src/editor.rs`, the `path` field: `/// Where it is saved; `None` until first saved. A `.pen` file or a bundle (a `.quill` file, [`crate::bundle`]).`
- `src/app/pictures.rs`, in the comment above `settle`: no change of meaning; leave "bundle".

- [ ] **Step 6: Run the tests**

Run: `cargo test pictures::tests::a_document_goes` then `cargo test pictures::tests::a_pencraft`
Expected: both pass. (If the build fails in `app.rs` or `tools.rs`, those are Tasks 6 and 8; the code there still compiles against this task's `Editor`, so a failure here is in this task's edits.)

---

### Task 6: `.quill` by default, and Save on a `.pencraft`

**Files:**
- Modify: `src/app.rs:624-626` (`ask_open`), `:633-662` (`save_from`), `:668-680` (`ask_save`), `:902-913` (the Save As answer)

- [ ] **Step 1: Offer all three endings on Open**

```rust
    fn ask_open(&mut self) {
        self.dialogs.ask(FileDialog::open_many().title("Open").filter("Design files", &[bundle::ENDING, bundle::PLAIN, bundle::OLD]), Then::Open);
    }
```

- [ ] **Step 2: Replace `save_from`**

```rust
    /// Saves the document in the tab `tab`, whose files lay beside `was` when it was last saved (its
    /// [`Editor::anchor`] then; `None` when never), asking for a file first when it has none; its pictures are
    /// brought to where it is saved (`pictures::settle`).
    fn save_from(&mut self, tab: usize, close: bool, was: Option<&Path>) {
        let Some(editor) = self.tabs.get_mut(tab) else { return };
        if editor.path.is_none() {
            self.ask_save("Save", tab, close, false);
            return;
        }
        // Human: an old `.pencraft` bundle is saved as a `.quill` file of the same name beside it, and stays as
        // it is. A `.quill` file of that name that is there already is somebody's: the user says where to.
        let mut renamed = None;
        if let Some(next) = editor.path.as_deref().and_then(bundle::successor) {
            if next.exists() {
                self.ask_save("Save As", tab, close, false);
                return;
            }
            if let Err(error) = editor.move_to(next.clone()) {
                self.say(format!("Couldn't save: {error}"));
                return;
            }
            let name = next.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
            renamed = Some(format!("It is now {name}; the .pencraft file is left as it is."));
        }
        let mut told = pictures::settle(editor, was, crate::imagery::store::home().as_deref());
        // Human: said once, when the document first goes to a `.pen` file of this name, not at every save.
        let plain = editor.path.as_deref().is_some_and(|path| !bundle::is_bundle(path));
        let only_here = editor.only_here().filter(|_| plain && was != editor.anchor().as_deref());
        match editor.save() {
            Ok(more) => {
                if let Some(path) = editor.path.as_ref().map(|p| p.display().to_string()) {
                    self.prefs.update(|p| p.files.touch(&path));
                }
                let moved = renamed.is_some();
                told.extend(renamed);
                told.extend(more);
                told.extend(only_here);
                if !told.is_empty() {
                    self.say(format!("Saved. {}", told.join(" ")));
                }
                if moved {
                    self.remember_tabs();
                }
                if close {
                    self.close_tab_now(tab);
                }
            }
            Err(e) => self.say(format!("Couldn't save: {e}")),
        }
    }
```

The working folder does not change when the name does, so `was` and `Editor::anchor` still agree and no picture is copied anywhere.

- [ ] **Step 3: Replace `ask_save`**

```rust
    /// Asks where to save the document in the tab `tab`. The name offered ends as the file it has does (`.quill`
    /// for one without a file), or the other way when `other` says so; either ending can be typed.
    // Human: the ending picks the format: `.quill` is one file with the document's pictures and files in it,
    // `.pen` the document alone.
    fn ask_save(&mut self, title: &str, tab: usize, close: bool, other: bool) {
        let Some(editor) = self.tabs.get(tab) else { return };
        let endings = bundle::offered(editor.path.as_deref(), other);
        let name = format!("{}.{}", editor.title(), endings[0]);
        self.dialogs.ask(FileDialog::save().title(title).filter("Design files", &endings).file_name(name), Then::SaveAs { tab, close });
    }
```

- [ ] **Step 4: Replace the Save As answer**

In `handle_dialog_answers`, replace the `(Then::SaveAs { tab, close }, Answer::Paths(paths)) => { … }` arm with:

```rust
                (Then::SaveAs { tab, close }, Answer::Paths(paths)) => {
                    let Some(path) = paths.into_iter().next().map(bundle::ended) else { continue };
                    if bundle::is_old(&path) {
                        self.say("A .pencraft file is no longer written. Give the name the ending .quill.".to_owned());
                        continue;
                    }
                    if let Some(editor) = self.tabs.get_mut(tab) {
                        match editor.move_to(path) {
                            Ok(was) => {
                                self.save_from(tab, close, was.as_deref());
                                self.remember_tabs();
                            }
                            Err(error) => self.say(format!("Couldn't save: {error}")),
                        }
                    }
                }
```

If `self.say` takes `impl Into<String>` or `&str` in this codebase, pass the literal without `.to_owned()`; look at its signature in `src/app.rs` and match the other calls.

- [ ] **Step 5: Build**

Run: `cargo build`
Expected: builds. The only errors left may be in `src/menus.rs`, `src/palette.rs` and `src/app/tools.rs` tests, which are the next tasks.

---

### Task 7: The menu and the command list say `.quill`

**Files:**
- Modify: `src/menus.rs:17`, `:155-156`, `:258`
- Modify: `src/palette.rs:20`, `:161`, `:422`

- [ ] **Step 1: Change the tests first**

- `src/menus.rs:258`: `"Save As Bundle…",` becomes `"Save As .quill File…",`
- `src/palette.rs:422`: `"Save as bundle or .pen file…"` becomes `"Save as .quill or .pen file…"`

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test menus::` then `cargo test palette::`
Expected: `the_menus_keep_their_items_and_order` and the palette's `matching("save")` test FAIL on the old words.

- [ ] **Step 3: Change the words**

`src/menus.rs:155-156`:

```rust
            // Human: the way into the other format: a `.quill` file holds the document's pictures and files with it.
            item("file.save_as_other", if state.bundle { "Save As .pen File…" } else { "Save As .quill File…" }, "", doc),
```

`src/menus.rs:17`: `/// Whether that document is saved as a `.quill` file (or an old bundle).`

`src/palette.rs:20`: `/// Save the document under a new name in the other format: a `.pen` file as a `.quill` file, and the other way.`

`src/palette.rs:161`:

```rust
    action(Command::SaveAsOther, "package", "Save as .quill or .pen file…", "File", &[], "quill bundle pencraft package pictures one file convert send"),
```

- [ ] **Step 4: Run them to see them pass**

Run: `cargo test menus::` then `cargo test palette::`
Expected: all pass.

---

### Task 8: `save_document`

**Files:**
- Modify: `src/app/tools.rs:393-420`
- Modify: `src/agent.rs:307-308`

- [ ] **Step 1: Replace the path handling in the tool**

In the `"save_document"` arm of `src/app/tools.rs`, replace from `let mut was = editor.anchor();` through the line `let mut notes = editor.save()…;` with:

```rust
                let mut was = editor.anchor();
                let mut renamed = false;
                if let Some(path) = path {
                    if crate::bundle::is_old(&path) {
                        return Err("A .pencraft file is no longer written: give `path` the ending .quill.".to_owned());
                    }
                    // Human: `.quill` saves the document with its pictures and files in one file, `.pen` the
                    // document alone; any other ending, or none, becomes `.quill`.
                    let path = if crate::bundle::is_design(&path) { path } else { path.with_extension(crate::bundle::ENDING) };
                    was = editor.move_to(path).map_err(|error| format!("Couldn't save: {error}."))?;
                } else if let Some(next) = editor.path.as_deref().and_then(crate::bundle::successor) {
                    // Human: as when the person saves an old bundle: it becomes a `.quill` file beside it.
                    if next.exists() {
                        return Err(format!("{} is there already: give `path` to say where this .pencraft design is saved as a .quill file.", next.display()));
                    }
                    was = editor.move_to(next).map_err(|error| format!("Couldn't save: {error}."))?;
                    renamed = true;
                }
                // Human: as when the person saves: the pictures come to where the document is saved.
                let stuck = super::pictures::settle(editor, was.as_deref(), crate::imagery::store::home().as_deref());
                let plain = editor.path.as_deref().is_some_and(|path| !crate::bundle::is_bundle(path));
                let only_here = editor.only_here().filter(|_| plain && was != editor.anchor());
                let mut notes = editor.save().map_err(|error| format!("Couldn't save: {error}. A document without a file needs `path`."))?;
                if renamed {
                    notes.push("The design was a .pencraft file. It is saved as a .quill file of the same name now; the .pencraft file is left as it is.".to_owned());
                }
```

The lines after it (`notes.extend(only_here);` onward) stay.

- [ ] **Step 2: Change the tool's description**

`src/agent.rs:307-308`:

```rust
            "description": "Write the document to its file. A document that has no file yet needs `path`. A path ending in .quill saves one file that holds the document with its pictures and shader files; a path ending in .pen saves the document alone.",
            "inputSchema": { "type": "object", "properties": { "document": document, "path": { "type": "string", "description": "Save under this full path (ending in .quill, or in .pen for the document alone) from now on." } } }
```

- [ ] **Step 3: Find tests that pin the old words or the old default**

Run: `cargo test tools::` and `cargo test agent::`
Expected: pass. If a test saves under a path with no ending or an unknown one and expects `.pen`, change what it expects to `.quill`: that is the new rule. If a test pins the old description text, change the text it expects to the new one.

---

### Task 9: The installer, the rules and the words

**Files:**
- Modify: `../installer/pencraft.iss:54`, `:66-67`
- Modify: `MECHANICS.md:19-22`, `:24`
- Modify: `mechanics.json:14-18`
- Modify: `AI.md:182`
- Modify: `../README.md:30-31`
- Modify: `../CHANGELOG.md` (under `## [Unreleased]`)

- [ ] **Step 1: The Windows association**

In `installer/pencraft.iss`, the task's words:

```
Name: "associate"; Description: "Open .quill, .pen and .pencraft files with {#AppName}"
```

and add this line after the `.pencraft\OpenWithProgids` line:

```
Root: HKA; Subkey: "Software\Classes\.quill\OpenWithProgids"; ValueType: string; ValueName: "Pencraft.Design"; ValueData: ""; Flags: uninsdeletevalue; Tasks: associate
```

- [ ] **Step 2: `MECHANICS.md`**

Replace rows 19 to 22 (Bundle, Opening a bundle, Saving into a bundle, Which format) with these six rows:

```markdown
| Design file (`.quill`) | one binary file holding a document and the files it names. Numbers little-endian. Header, 32 bytes: magic `89 51 55 49 4C 4C 0D 0A`, layout number `1` (4), number of index entries (4), index length (8), CRC-32 of the index (4), zero (4). Index entry: kind (1: `1` document, `2` kept for the chat, `3` named file), packing (1: `0` stored, `1` deflate), name length (2), block start from the file's start (8), block length stored (8) and unpacked (8), CRC-32 of the block as stored (4), name (UTF-8, the path the document names the file by; empty for the document). Then the blocks: the document, the named files in name order, `images/credits.json` cut down to the pictures that go in. Named files: the `url` of every picture and shader, the pictures a shader is given (`uniforms`), and `imports`, where they are paths inside the document's folder. Files ending `.png`, `.jpg`, `.jpeg`, `.webp`, `.gif` are stored; everything else is deflated, or stored when that isn't smaller. Entries with the same stored bytes share one block. Files in the folder the document doesn't name stay out; a named file that isn't there is left out and the save says so. The same document and files give the same bytes | Pencraft's own format |
| Document block | the document's JSON tree in binary: encoding number `1`, the key table (count, then each key as length and UTF-8 bytes, in the order keys first appear), one value. Counts, lengths and whole numbers are LEB128 varints. Value tags: `0` null, `1` false, `2` true, `3` whole number zero or above (varint), `4` whole number below zero (varint of `-(n + 1)`), `5` float (8 bytes IEEE 754), `6` string (length, bytes), `7` list (count, values), `8` map (count, pairs of key number and value). Key order, unknown keys and whether a number is whole or a float are kept, so `.pen` → `.quill` → `.pen` gives the same bytes. Nesting at most 128 deep | Pencraft's own format |
| Opening a `.quill` file | unpacked into a working folder of its own under `~/.pencraft/open` (2 GB unpacked at most), where the document's relative paths start. Every place and length is checked against the file's length first. A damaged index or document, a block outside the file, or a name that leads outside the working folder refuses the whole file. A named file whose block doesn't match its CRC-32 is left out and the rest opens, with a sentence naming it. A higher layout number, or an entry kind this version doesn't know (skipped), opens with the sentence about a newer Pencraft. The folder is removed when the tab closes; folders no running Pencraft holds (a lock on `.held` inside) are removed at the next start, a minute after they were made at the earliest. The home screen's small pictures read the header, the index and the document block only | Pencraft's own rule |
| Old bundles (`.pencraft`) | still open: a zip archive (deflate or stored, no zip64, no encryption) with `manifest.json` (`{format: "pencraft-bundle", version: 1}`), `document.pen` and the named files. One zipped again inside one folder, and the Finder's `__MACOSX` and `.DS_Store`, are read through. No longer written: Save writes the same name ending `.quill` beside it, the tab goes to that file, the `.pencraft` file stays as it is; when a file of that name is there already, Save As is asked. `save_document` refuses a path ending `.pencraft` | Pencraft's own rule |
| Saving into a `.quill` file | every picture file the document shows is copied into `images/` in the working folder and named from there, also one named by a full path or from outside the folder (`../`); shader files and imported documents named inside the folder come along. Going from a `.quill` file to a `.pen` file, the named files are copied beside it, never over a file there | Pencraft's own rule |
| Which format | the file name's ending picks what Save writes: `.quill` one file with the document's files in it, `.pen` the document alone. What a file ending `.quill` or `.pencraft` is, is read from its first bytes (the magic, or `PK` for an old bundle), so an old bundle renamed to `.quill` opens; a file ending `.pen` that starts with either is refused with a sentence to give it the ending `.quill`. Save As offers the ending the document has, `.quill` for a new one; a name typed without an ending gets `.quill`. File › Save As .quill File… / Save As .pen File… offers the other | Pencraft's own rule |
```

In row 24 ("A file opened while Pencraft runs"), change `ties `.pen` and `.pencraft` to Pencraft` to `ties `.quill`, `.pen` and `.pencraft` to Pencraft`.

In the title of row 5's section, `## File format (`.pen`)` becomes `## File formats (`.pen`, `.quill`)`.

- [ ] **Step 3: `mechanics.json`**

Replace the entry named `bundle: layout version in the manifest` with these three, keeping the file's one-space indent:

```json
  {
   "name": ".quill file: layout number",
   "group": "file",
   "value": 1
  },
  {
   "name": ".quill file: deepest nesting of values in the document block",
   "group": "file",
   "value": 128
  },
  {
   "name": "old bundle: layout version in the manifest",
   "group": "file",
   "value": 1
  },
```

- [ ] **Step 4: `AI.md`, `README.md`, `CHANGELOG.md`**

`AI.md:182`:

```markdown
| `save_document` | Saves the document, or saves it under a new path; a path ending in `.quill` saves one file with its pictures and shader files in it, a path ending in `.pen` the document alone |
```

`README.md:30-31`:

```markdown
- **One file per design.** Save writes a `.quill` file that holds the design
  with its pictures and shader files, to send or to keep. `.pen` files open
  and save as before.
```

`CHANGELOG.md`, directly under `## [Unreleased]`:

```markdown

### Added

- **A design is one file now: `.quill`.** Save writes a single `.quill`
  file that holds the design together with its pictures and shader files,
  so it opens the same on any computer. New designs are saved this way
  unless you choose `.pen` in File › Save As .pen File…. A picture used
  several times is stored once, and if one picture in a file is damaged the
  rest of the design still opens.

### Changed

- **`.pencraft` bundles become `.quill` files.** Your bundles still open.
  The first time you save one, Pencraft writes a `.quill` file of the same
  name beside it and continues there; the `.pencraft` file is left as it
  is. `.pen` files open and save as before.
```

- [ ] **Step 5: Look for anything else that names the old format**

Run (from the repo root, one level above `app/`):

```bash
grep -rn "Save As Bundle\|as bundle\|pencraft-bundle\|\.pencraft\b" --include=*.rs --include=*.md --include=*.iss --include=*.json --include=*.wxs --include=*.plist . | grep -v "~/.pencraft\|\.pencraft\")\|/\.pencraft/\|docs/superpowers\|CHANGELOG.md\|target/"
```

Expected: only lines that are about reading old bundles (`bundle.rs`, `zip.rs`, their tests, the `MECHANICS.md` row, the installer's `.pencraft` line, `app.rs` and `tools.rs` where they refuse or redirect one). Anything else that tells a person or an AI model to make a `.pencraft` file gets the new words.

---

### Task 10: The full gate, and a look at the real thing

- [ ] **Step 1: Run the gate**

Run on Windows: `powershell -File scripts/check.ps1` (elsewhere: `scripts/check.sh`)
Expected: `check: all passed`.

What is likely to need a touch: pedantic clippy lints in the two new files (it names the line and the fix), and `cargo fmt` (run `cargo fmt --all`, then the gate again). Do not silence a lint; change the code.

- [ ] **Step 2: Check documents of the real world**

Run: `cargo run -p pen-doc --example pen_check -- <every .pen file under ../design and any sample designs in the repo>`
Expected: as before this change (this proves nothing in the `.pen` path moved).

- [ ] **Step 3: Try it in the app**

Run the app (`cargo run`) and check each line by hand:

1. New File, add a rectangle, Save: the dialog offers `Untitled.quill`. Save it; the file starts with the bytes `89 51 55 49 4C 4C 0D 0A` (`Format-Hex -Count 8 <file>` in PowerShell).
2. Import a picture from another folder, Save, close the tab, move the `.quill` file to another folder, open it: the picture shows.
3. File › Save As .pen File…: a `.pen` file is written with `images/` beside it; Save again keeps writing `.pen`.
4. Open a `.pen` file, Save: it is still a `.pen` file.
5. Open an old `.pencraft` bundle (make one with the released version if none is at hand), change something, Save: a `.quill` file of the same name appears beside it, the tab shows it, the `.pencraft` file's date is unchanged, and the message says so.
6. The home screen shows a small picture of each `.quill` file.
7. Duplicate a picture layer several times, Save: the file is barely larger than with one.

Report what you saw for each line; a line you could not try is said, not skipped.
