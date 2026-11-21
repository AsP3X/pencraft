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
    /// What it is: [`DOCUMENT`] or [`FILE`], or a number a later version gave a kind this one doesn't know.
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
    /// The kind and name of each entry whose block doesn't match its checksum or can't be unpacked, also one
    /// of a kind this version doesn't know.
    pub damaged: Vec<(u8, String)>,
    /// The entries of kinds this version doesn't know, in the file's order, to be written back as they are.
    /// One whose block is damaged is left out, and is among `damaged`.
    pub unknown: Vec<Entry>,
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
        let block = blocks.get(*at).ok_or_else(|| "the design could not be laid out in the file".to_owned())?;
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

/// Reads the `.quill` file `bytes`. Stops when its entries come to more than `most` bytes unpacked.
///
/// # Errors
/// A sentence when it isn't a `.quill` file, its header or index is damaged, a block lies outside the file,
/// or it holds more than `most` bytes.
pub fn read(bytes: &[u8], most: u64) -> Result<Unpacked, String> {
    let head = head(bytes)?;
    let index = usize::try_from(head.index).ok().and_then(|length| HEADER.checked_add(length)).and_then(|end| bytes.get(HEADER..end));
    let slots = slots(index.ok_or_else(damaged)?, &head, wide(bytes.len()))?;
    // Human: an entry of a kind this version doesn't know is read like any other and handed on beside the
    // rest, so whoever saves the file again can put it back: a later version's part isn't lost by an older one.
    let (mut entries, mut broken, mut unknown, mut later, mut left) = (Vec::new(), Vec::new(), Vec::new(), false, most);
    for slot in slots {
        let known = slot.kind == DOCUMENT || slot.kind == FILE;
        later |= !known;
        // Agent: the budget is charged for the larger of the two lengths, as both cost work (a checksum, a copy).
        left = left.checked_sub(slot.unpacked.max(slot.stored)).ok_or_else(|| "it unpacks to more than this version opens".to_owned())?;
        let data = usize::try_from(slot.start)
            .ok()
            .zip(usize::try_from(slot.stored).ok())
            .and_then(|(from, length)| bytes.get(from..from.checked_add(length)?));
        match data.and_then(|data| unpack(&slot, data)) {
            Some(bytes) if known => entries.push(Entry { kind: slot.kind, name: slot.name, bytes }),
            Some(bytes) => unknown.push(Entry { kind: slot.kind, name: slot.name, bytes }),
            None => broken.push((slot.kind, slot.name)),
        }
    }
    Ok(Unpacked { entries, damaged: broken, unknown, newer: (head.layout > LAYOUT || later).then_some(head.layout) })
}

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
    // Agent: reads up to a header's length, so a file shorter than that still gets the right sentence from `head`.
    let mut header = Vec::with_capacity(HEADER);
    file.by_ref().take(wide(HEADER)).read_to_end(&mut header).map_err(told)?;
    let head = head(&header)?;
    let too_large = || "its document is too large for this".to_owned();
    if head.index > length.saturating_sub(wide(HEADER)) {
        return Err(damaged());
    }
    let mut index = vec![0u8; usize::try_from(head.index).ok().filter(|_| head.index <= most).ok_or_else(too_large)?];
    file.read_exact(&mut index).map_err(told)?;
    let slot = slots(&index, &head, length)?.into_iter().find(|slot| slot.kind == DOCUMENT).ok_or("it holds no document")?;
    if slot.unpacked > most || slot.stored > most {
        return Err(too_large());
    }
    let mut data = vec![0u8; usize::try_from(slot.stored).map_err(|_| too_large())?];
    file.seek(SeekFrom::Start(slot.start)).map_err(told)?;
    file.read_exact(&mut data).map_err(told)?;
    unpack(&slot, &data).ok_or_else(|| "its document is damaged".to_owned())
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
    // Agent: what is shorter than the magic is refused as not ours unless it is the start of the magic.
    let shown = bytes.len().min(MAGIC.len());
    if bytes.get(..shown) != MAGIC.get(..shown) {
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
    let mut slots: Vec<Slot> = Vec::with_capacity(count);
    let mut at = 0usize;
    // Human: no block lies in the header or the index, and a file has one document at most.
    let first = wide(HEADER).saturating_add(wide(index.len()));
    for _ in 0..count {
        let fixed = at.checked_add(SLOT).and_then(|end| index.get(at..end)).ok_or_else(damaged)?;
        let name_length = usize::from(u16_at(fixed, 2)?);
        let (start, stored, unpacked, sum) = (u64_at(fixed, 4)?, u64_at(fixed, 12)?, u64_at(fixed, 20)?, u32_at(fixed, 28)?);
        let name = index.get(at + SLOT..at + SLOT + name_length).ok_or_else(damaged)?;
        let name = String::from_utf8(name.to_vec()).map_err(|_| "a file name in it isn't UTF-8".to_owned())?;
        if start < first || start.checked_add(stored).is_none_or(|end| end > length) {
            return Err(damaged());
        }
        let (kind, packing) = (fixed.first().copied().unwrap_or_default(), fixed.get(1).copied().unwrap_or_default());
        if kind == DOCUMENT && slots.iter().any(|slot| slot.kind == DOCUMENT) {
            return Err("its index is damaged".to_owned());
        }
        slots.push(Slot { kind, packing, name, start, stored, unpacked, sum });
        at += SLOT + name_length;
    }
    if at != index.len() {
        return Err("its index is damaged".to_owned());
    }
    Ok(slots)
}

/// The bytes of the block `data` unpacked; `None` when it doesn't match its checksum, is packed in a way this
/// version doesn't know, or isn't as long unpacked as its entry says.
fn unpack(slot: &Slot, data: &[u8]) -> Option<Vec<u8>> {
    // Agent: a block that is stored as it is can only be as long as it says; refused before it is checksummed.
    if slot.packing == STORED && slot.stored != slot.unpacked {
        return None;
    }
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
        assert_eq!(read(&file, 1 << 20).unwrap(), Unpacked { entries: entries.clone(), damaged: Vec::new(), unknown: Vec::new(), newer: None });
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
        // It is handed on as it is, and written back it reads the same again.
        assert_eq!(unpacked.unknown, [entry(9, "fx/glow.glsl", b"void main(){}")]);
        let again: Vec<Entry> = unpacked.entries.iter().chain(&unpacked.unknown).cloned().collect();
        assert_eq!(read(&write(&again).unwrap(), 1 << 20).unwrap().unknown, unpacked.unknown);
        // It counts towards the most, and one that is damaged is left out and named.
        assert_eq!(read(&odd, 4_512).unwrap_err(), "it unpacks to more than this version opens");
        let spoiled = reindexed(odd, |index| index[SLOT + 1] = 9);
        let unpacked = read(&spoiled, 1 << 20).unwrap();
        assert_eq!((unpacked.newer, unpacked.unknown.len()), (Some(LAYOUT), 0));
        assert_eq!(unpacked.damaged, [(9, "fx/glow.glsl".to_owned())]);
    }

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

    /// The length of `file`'s index.
    fn index_length(file: &[u8]) -> usize {
        usize::try_from(u64::from_le_bytes(file[16..24].try_into().unwrap())).unwrap()
    }

    /// A little data that doesn't deflate.
    fn noise(length: usize) -> Vec<u8> {
        let mut seed = 12345u32;
        (0..length)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                seed.to_le_bytes()[2]
            })
            .collect()
    }

    #[test]
    fn work_is_charged_for_the_larger_length() {
        let file = write(&[entry(FILE, "a.png", &[1u8; 3000])]).unwrap();
        // It says it unpacks to nothing; its block is 3000 bytes.
        let lying = reindexed(file, |index| index[20..28].copy_from_slice(&0u64.to_le_bytes()));
        assert_eq!(read(&lying, 100).unwrap_err(), "it unpacks to more than this version opens");
        // Stored as it is, a block is as long unpacked as it is stored; else that part is damaged.
        assert_eq!(read(&lying, 1 << 20).unwrap().damaged, [(FILE, "a.png".to_owned())]);
    }

    #[test]
    fn a_stored_document_longer_than_it_says_is_refused_before_it_is_read() {
        let file = write(&[entry(DOCUMENT, "", &noise(3000))]).unwrap();
        let lying = reindexed(file.clone(), |index| index[20..28].copy_from_slice(&10u64.to_le_bytes()));
        assert_eq!(document(&mut std::io::Cursor::new(&lying), 100).unwrap_err(), "its document is too large for this");
        assert_eq!(document(&mut std::io::Cursor::new(&lying), 1 << 20).unwrap_err(), "its document is damaged");
        assert_eq!(document(&mut std::io::Cursor::new(&file), 1 << 20).unwrap().len(), 3000);
    }

    #[test]
    fn two_documents_are_a_damaged_index_to_both_readers() {
        let file = write(&[entry(DOCUMENT, "", b"one"), entry(DOCUMENT, "", b"two")]).unwrap();
        assert_eq!(read(&file, 100).unwrap_err(), "its index is damaged");
        assert_eq!(document(&mut std::io::Cursor::new(file), 100).unwrap_err(), "its index is damaged");
    }

    #[test]
    fn a_block_may_not_start_in_the_header_or_the_index() {
        let file = write(&sample()).unwrap();
        let inside = reindexed(file.clone(), |index| index[SLOT + 4..SLOT + 12].copy_from_slice(&0u64.to_le_bytes()));
        assert_eq!(read(&inside, 1 << 20).unwrap_err(), "it is cut off or damaged");
        let in_index = reindexed(file, |index| index[SLOT + 4..SLOT + 12].copy_from_slice(&40u64.to_le_bytes()));
        assert_eq!(document(&mut std::io::Cursor::new(in_index), 1 << 20).unwrap_err(), "it is cut off or damaged");
    }

    #[test]
    fn bytes_left_over_in_the_index_are_a_damaged_index() {
        let mut file = write(&sample()).unwrap();
        let length = index_length(&file);
        file.insert(HEADER + length, 0);
        // The blocks moved a byte on; the index says so.
        let mut at = HEADER;
        while at < HEADER + length {
            let start = u64::from_le_bytes(file[at + 4..at + 12].try_into().unwrap()) + 1;
            file[at + 4..at + 12].copy_from_slice(&start.to_le_bytes());
            at += SLOT + usize::from(u16::from_le_bytes([file[at + 2], file[at + 3]]));
        }
        file[16..24].copy_from_slice(&u64::try_from(length + 1).unwrap().to_le_bytes());
        let sum = checksum(&file[HEADER..=HEADER + length]);
        file[24..28].copy_from_slice(&sum.to_le_bytes());
        assert_eq!(read(&file, 1 << 20).unwrap_err(), "its index is damaged");
    }

    #[test]
    fn the_sentences_for_what_is_short_or_not_ours() {
        assert_eq!(read(b"{}", 100).unwrap_err(), "it is not a .quill file");
        assert_eq!(read(&MAGIC[..5], 100).unwrap_err(), "it is cut off or damaged");
        assert_eq!(read(b"", 100).unwrap_err(), "it is cut off or damaged");
        assert_eq!(document(&mut std::io::Cursor::new(b"{}".to_vec()), 100).unwrap_err(), "it is not a .quill file");
        // An index longer than the file is cut off, however much is asked for.
        let mut file = write(&sample()).unwrap();
        file[16..24].copy_from_slice(&1_000_000u64.to_le_bytes());
        assert_eq!(document(&mut std::io::Cursor::new(file), 100).unwrap_err(), "it is cut off or damaged");
    }

    #[test]
    fn a_block_that_inflates_past_its_length_is_damaged() {
        let file = write(&sample()).unwrap();
        let short = reindexed(file.clone(), |index| index[20..28].copy_from_slice(&10u64.to_le_bytes()));
        assert_eq!(read(&short, 1 << 20).unwrap().damaged, [(DOCUMENT, String::new())]);
        assert_eq!(document(&mut std::io::Cursor::new(short), 1 << 20).unwrap_err(), "its document is damaged");
    }

    #[test]
    fn a_flipped_byte_in_the_document_block_is_found_by_both_readers() {
        let entries = sample();
        let mut file = write(&entries).unwrap();
        let at = HEADER + index_length(&file);
        file[at] ^= 0x55;
        let unpacked = read(&file, 1 << 20).unwrap();
        assert_eq!(unpacked.damaged, [(DOCUMENT, String::new())]);
        assert_eq!(unpacked.entries, entries[1..]);
        assert_eq!(document(&mut std::io::Cursor::new(file), 1 << 20).unwrap_err(), "its document is damaged");
    }

    #[test]
    fn names_that_cannot_be_read_or_written() {
        let file = write(&sample()).unwrap();
        // The second entry's name starts after the first's entry and its own fixed part.
        let bad = reindexed(file.clone(), |index| index[2 * SLOT] = 0xFF);
        assert_eq!(read(&bad, 1 << 20).unwrap_err(), "a file name in it isn't UTF-8");
        let past = reindexed(file, |index| index[2..4].copy_from_slice(&60_000u16.to_le_bytes()));
        assert_eq!(read(&past, 1 << 20).unwrap_err(), "it is cut off or damaged");
        let long = "a".repeat(65_536);
        let error = write(&[entry(FILE, &long, b"")]).unwrap_err();
        assert!(error.starts_with("the file name ") && error.ends_with(" is too long"));
    }

    #[test]
    fn document_reads_a_cut_off_file_as_cut_off() {
        let file = write(&sample()).unwrap();
        for cut in [0, 20, 40] {
            let error = document(&mut std::io::Cursor::new(file[..cut].to_vec()), 1 << 20).unwrap_err();
            assert_eq!(error, "it is cut off or damaged", "cut at {cut}");
        }
        let short = file[..file.len() - 10].to_vec();
        assert!(document(&mut std::io::Cursor::new(short), 1 << 20).is_err());
    }
}
