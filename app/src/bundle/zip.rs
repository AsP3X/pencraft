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

/// A file in an archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Its path in the archive, parts joined by `/`.
    pub name: String,
    /// What it holds.
    pub bytes: Vec<u8>,
}

const LOCAL: u32 = 0x0403_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const END: u32 = 0x0605_4b50;
/// The length of each record before its file name.
const LOCAL_LENGTH: usize = 30;
const CENTRAL_LENGTH: usize = 46;
const END_LENGTH: usize = 22;
/// Zip 2.0: deflate and folders.
#[cfg(test)]
const VERSION: u16 = 20;
/// Bit 11: names are UTF-8.
#[cfg(test)]
const UTF8: u16 = 0x0800;
const STORED: u16 = 0;
const DEFLATED: u16 = 8;
/// 1 January 1980, the earliest day the format can say: the same bytes for the same files, whenever written.
#[cfg(test)]
const DAY: u16 = 0x0021;
/// Endings of files that are compressed already; deflating them again costs time and gains nothing.
#[cfg(test)]
const PACKED: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

/// Writes `entries` as a zip archive.
///
/// # Errors
/// A sentence when there are more files or bytes than a plain zip holds.
#[cfg(test)]
pub fn write(entries: &[Entry]) -> Result<Vec<u8>, String> {
    let too_large = |_| "the bundle would be larger than 4 GB, which this format can't hold".to_owned();
    let count = u16::try_from(entries.len()).map_err(|_| "the bundle would hold more than 65 535 files".to_owned())?;
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for entry in entries {
        let name = entry.name.as_bytes();
        let name_length = u16::try_from(name.len()).map_err(|_| format!("the file name {} is too long", entry.name))?;
        let (method, data) = packed(entry);
        let offset = u32::try_from(out.len()).map_err(too_large)?;
        // Human: what the local record and the directory's record share: version, flags, method, time, day,
        // checksum, both sizes and the name's length.
        let mut shared = Vec::with_capacity(26);
        for field in [VERSION, UTF8, method, 0, DAY] {
            shared.extend(field.to_le_bytes());
        }
        shared.extend(checksum(&entry.bytes).to_le_bytes());
        shared.extend(u32::try_from(data.len()).map_err(too_large)?.to_le_bytes());
        shared.extend(u32::try_from(entry.bytes.len()).map_err(too_large)?.to_le_bytes());
        shared.extend(name_length.to_le_bytes());
        out.extend(LOCAL.to_le_bytes());
        out.extend(&shared);
        out.extend(0u16.to_le_bytes());
        out.extend(name);
        out.extend(&*data);
        directory.extend(CENTRAL.to_le_bytes());
        directory.extend(VERSION.to_le_bytes());
        directory.extend(&shared);
        // Extra length, comment length, disk, inner attributes (2 bytes each), outer attributes (4).
        directory.extend([0u8; 12]);
        directory.extend(offset.to_le_bytes());
        directory.extend(name);
    }
    let start = u32::try_from(out.len()).map_err(too_large)?;
    let length = u32::try_from(directory.len()).map_err(too_large)?;
    out.extend(directory);
    out.extend(END.to_le_bytes());
    out.extend([0u8; 4]);
    out.extend(count.to_le_bytes());
    out.extend(count.to_le_bytes());
    out.extend(length.to_le_bytes());
    out.extend(start.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    u32::try_from(out.len()).map_err(too_large)?;
    Ok(out)
}

/// The method an entry is kept with, and its bytes that way: deflated when that is smaller, else as it is.
#[cfg(test)]
fn packed(entry: &Entry) -> (u16, std::borrow::Cow<'_, [u8]>) {
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

fn checksum(bytes: &[u8]) -> u32 {
    let mut sum = flate2::Crc::new();
    sum.update(bytes);
    sum.sum()
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at.checked_add(2)?)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

fn usize_at(bytes: &[u8], at: usize) -> Option<usize> {
    usize::try_from(u32_at(bytes, at)?).ok()
}

/// Reads the files of the zip archive `bytes` whose names `wanted` says yes to; folders are left out. Stops
/// when they come to more than `most` bytes unpacked.
///
/// # Errors
/// A sentence when it isn't a zip archive, is damaged or encrypted, uses what this reader doesn't know, or
/// holds more than `most` bytes.
// Human: the sizes come from the archive's directory, which a file can lie in; no more than the size it says
// is ever unpacked for a file, and the checksum is compared.
pub fn read(bytes: &[u8], most: u64, wanted: &dyn Fn(&str) -> bool) -> Result<Vec<Entry>, String> {
    let damaged = || "it is not a zip archive, or it is damaged".to_owned();
    let last = bytes.len().checked_sub(END_LENGTH).ok_or_else(damaged)?;
    // Human: the closing record is the last thing in the file, before a comment of at most 65 535 bytes.
    let first = last.saturating_sub(usize::from(u16::MAX));
    let end = (first..=last).rev().find(|at| u32_at(bytes, *at) == Some(END)).ok_or_else(damaged)?;
    let count = u16_at(bytes, end + 10).ok_or_else(damaged)?;
    let start = u32_at(bytes, end + 16).ok_or_else(damaged)?;
    if count == u16::MAX || start == u32::MAX {
        return Err("it is larger than this version can read (zip64)".to_owned());
    }
    let mut at = usize::try_from(start).map_err(|_| damaged())?;
    let mut entries = Vec::new();
    let mut left = most;
    for _ in 0..count {
        if u32_at(bytes, at) != Some(CENTRAL) {
            return Err(damaged());
        }
        let field = |offset: usize| u16_at(bytes, at + offset).ok_or_else(damaged);
        let (flags, method) = (field(8)?, field(10)?);
        let (name_length, extra_length, comment_length) = (usize::from(field(28)?), usize::from(field(30)?), usize::from(field(32)?));
        let sum = u32_at(bytes, at + 16).ok_or_else(damaged)?;
        let (packed_size, size) = (usize_at(bytes, at + 20).ok_or_else(damaged)?, u32_at(bytes, at + 24).ok_or_else(damaged)?);
        let local = usize_at(bytes, at + 42).ok_or_else(damaged)?;
        let name = bytes.get(at + CENTRAL_LENGTH..at + CENTRAL_LENGTH + name_length).ok_or_else(damaged)?;
        let name = String::from_utf8(name.to_vec()).map_err(|_| "a file name in it isn't UTF-8".to_owned())?;
        at += CENTRAL_LENGTH + name_length + extra_length + comment_length;
        if name.ends_with('/') || !wanted(&name) {
            continue;
        }
        if flags & 1 != 0 {
            return Err("it is encrypted".to_owned());
        }
        if size == u32::MAX || packed_size == usize::try_from(u32::MAX).unwrap_or(usize::MAX) {
            return Err("it is larger than this version can read (zip64)".to_owned());
        }
        left = left.checked_sub(u64::from(size)).ok_or_else(|| "it unpacks to more than this version opens".to_owned())?;
        // Human: the local record repeats the name and may carry extra fields of its own length; the file's
        // bytes come after them.
        if u32_at(bytes, local) != Some(LOCAL) {
            return Err(damaged());
        }
        let skip = usize::from(u16_at(bytes, local + 26).ok_or_else(damaged)?) + usize::from(u16_at(bytes, local + 28).ok_or_else(damaged)?);
        let from = local.checked_add(LOCAL_LENGTH + skip).ok_or_else(damaged)?;
        let data = from.checked_add(packed_size).and_then(|to| bytes.get(from..to)).ok_or_else(damaged)?;
        let unpacked = match method {
            STORED => data.to_vec(),
            DEFLATED => {
                let mut unpacked = Vec::new();
                // One byte more than it says, so a file that is longer than it says is noticed.
                let read = DeflateDecoder::new(data).take(u64::from(size) + 1).read_to_end(&mut unpacked);
                read.map_err(|_| format!("the file {name} in it is damaged"))?;
                unpacked
            }
            _ => return Err(format!("the file {name} in it is compressed in a way this version doesn't know")),
        };
        if u32::try_from(unpacked.len()) != Ok(size) || checksum(&unpacked) != sum {
            return Err(format!("the file {name} in it is damaged"));
        }
        entries.push(Entry { name, bytes: unpacked });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(bytes: &[u8], most: u64) -> Result<Vec<Entry>, String> {
        super::read(bytes, most, &|_| true)
    }

    fn entry(name: &str, bytes: &[u8]) -> Entry {
        Entry { name: name.to_owned(), bytes: bytes.to_vec() }
    }

    #[test]
    fn what_is_written_reads_back_the_same() {
        let text = "a line that repeats. ".repeat(200);
        let entries = [entry("document.pen", text.as_bytes()), entry("images/ä b.png", &[0, 1, 2, 255]), entry("empty.txt", b"")];
        let archive = write(&entries).unwrap();
        assert!(archive.len() < text.len(), "the text is deflated");
        assert_eq!(read(&archive, 1 << 20).unwrap(), entries);
        assert_eq!(write(&entries).unwrap(), archive, "the same files give the same bytes");
        assert_eq!(write(&[]).unwrap().len(), END_LENGTH);
        assert_eq!(read(&write(&[]).unwrap(), 0).unwrap(), []);
        // Only the files asked for are unpacked, and only they count towards the most.
        assert_eq!(super::read(&archive, 4, &|name| name.starts_with("images/")).unwrap(), entries[1..2]);
    }

    #[test]
    fn an_archive_of_the_zip_program_reads() {
        // `printf 'hello hello hello hello\n' > a.txt; mkdir d; printf 'x' > d/b; zip -X t.zip a.txt d d/b`:
        // one deflated file, a folder and a stored file.
        let hex = "504b030414000000080037b6725d0088590b0b0000001800000005000000\
                   612e747874cb48cdc9c957c84027b900504b03040a000000000037b6725d\
                   00000000000000000000000002000000642f504b03040a000000000037b6\
                   725d8316dc8c010000000100000003000000642f6278504b01021e031400\
                   0000080037b6725d0088590b0b0000001800000005000000000000000100\
                   0000a48100000000612e747874504b01021e030a000000000037b6725d00\
                   0000000000000000000000020000000000000000001000ed412e00000064\
                   2f504b01021e030a000000000037b6725d8316dc8c010000000100000003\
                   0000000000000001000000a4814e000000642f62504b0506000000000300\
                   030094000000700000000000";
        let bytes: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
        assert_eq!(read(&bytes, 1000).unwrap(), [entry("a.txt", b"hello hello hello hello\n"), entry("d/b", b"x")]);
    }

    #[test]
    fn damage_and_too_much_are_refused() {
        let archive = write(&[entry("a.txt", &b"abcdefgh".repeat(100))]).unwrap();
        assert!(read(b"not a zip", 100).is_err() && read(&[], 100).is_err());
        assert!(read(&archive, 799).unwrap_err().contains("more than"));
        assert!(read(&archive, 800).is_ok());
        // A flipped byte in the data, a cut-off file, and a directory that points past the end.
        let mut flipped = archive.clone();
        flipped[LOCAL_LENGTH + 5 + 2] ^= 0x55;
        assert!(read(&flipped, 800).is_err());
        assert!(read(&archive[..archive.len() - 30], 800).is_err());
        let mut lost = archive.clone();
        let end = lost.len() - END_LENGTH;
        lost[end + 16..end + 20].copy_from_slice(&9_000u32.to_le_bytes());
        assert!(read(&lost, 800).is_err());
        // A file that says it is encrypted.
        let mut locked = archive;
        let directory = usize_at(&locked, end + 16).unwrap();
        locked[directory + 8] |= 1;
        assert_eq!(read(&locked, 800).unwrap_err(), "it is encrypted");
    }
}
