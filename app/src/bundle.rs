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

/// Where the design opened from `path` is saved, when that is not `path` itself: only a `.quill` file is saved
/// to. For a `.pen` document or an old bundle the same name ending `.quill`; for a file of any other name,
/// which opened as a plain document, that name with `.quill` after it (`design.json.quill`). `None` for a
/// `.quill` file, which is saved where it is, and for a path that names no file.
#[must_use]
pub fn successor(path: &Path) -> Option<PathBuf> {
    if ends(path, ENDING) {
        return None;
    }
    if is_old(path) || ends(path, PLAIN) {
        return Some(path.with_extension(ENDING));
    }
    let mut name = path.file_name()?.to_os_string();
    name.push(".");
    name.push(ENDING);
    Some(path.with_file_name(name))
}

/// `path` as a file of the kind `ending` is written under it (`.quill` for Save, `.pen` for an export): as it
/// is when it ends so, in capitals or not; else with `ending` put after its name.
///
/// # Errors
/// A sentence when `path` names no file (a drive, a folder's `..`), ends as a design file of another kind does
/// (that says how such a file is made), or got the ending added and a file of that name is there already.
// Human: the ending is added, never put in place of something: `home.v2` becomes `home.v2.quill`, not
// `home.quill`, which may be another design. A name that ends in a dot gets no second one. A name that asks
// for another kind of file than this command writes is not bent into shape: the user meant something else.
// The file dialog asks before a file is replaced, but about the name that was typed: a file under the name
// with the ending added was never asked about, and is not written over.
// Agent: READS whether the file with the added ending is there.
pub fn ended(path: PathBuf, ending: &str) -> Result<PathBuf, String> {
    let Some(name) = path.file_name() else { return Err(format!("{} names no file", path.display())) };
    if ends(&path, ending) {
        return Ok(path);
    }
    if ends(&path, PLAIN) {
        return Err("a .pen file is not saved but exported: use File \u{203a} Export .pen File\u{2026}".to_owned());
    }
    if ends(&path, OLD) {
        return Err("a .pencraft file is no longer written".to_owned());
    }
    if ends(&path, ENDING) {
        return Err("a .quill file is not exported but saved: use File \u{203a} Save As\u{2026}".to_owned());
    }
    let mut name = name.to_os_string();
    if !name.to_string_lossy().ends_with('.') {
        name.push(".");
    }
    name.push(ending);
    let ended = path.with_file_name(&name);
    if ended.exists() {
        return Err(format!("{} is there already: type the whole name to replace it", name.to_string_lossy()));
    }
    Ok(ended)
}

/// Whether `bytes` start as a bundle does, new or old.
#[must_use]
pub fn looks_packed(bytes: &[u8]) -> bool {
    quill::is_quill(bytes) || bytes.starts_with(b"PK")
}

/// The folder an open bundle lies unpacked in, held for as long as this lives.
// Human: the lock on the file inside says "in use" to every Pencraft on this computer, so the folders of
// documents that are open are never cleared away ([`sweep`]), and those a crash left behind are.
#[derive(Debug)]
pub struct Work {
    folder: PathBuf,
    held: File,
}

impl Work {
    /// A new, empty working folder.
    ///
    /// # Errors
    /// A sentence when the folder can't be made.
    // Agent: WRITES a folder under `~/.pencraft/open` (the system's temporary folder without a home folder).
    pub fn fresh() -> Result<Work, String> {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let folder = root().join(format!("{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        let made = || -> std::io::Result<File> {
            // Human: a folder of this name is one an earlier run with the same process number left behind.
            if folder.exists() {
                std::fs::remove_dir_all(&folder)?;
            }
            std::fs::create_dir_all(&folder)?;
            let held = File::create(folder.join(HELD))?;
            held.lock()?;
            Ok(held)
        };
        let held = made().map_err(|error| format!("the working folder {} couldn't be made: {error}", folder.display()))?;
        Ok(Work { folder, held })
    }

    /// The folder.
    #[must_use]
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// Where the document would lie in the folder: what its relative paths start beside.
    #[must_use]
    pub fn document(&self) -> PathBuf {
        self.folder.join(DOCUMENT)
    }

    /// Lets go of the folder and removes it.
    // Agent: WRITES (removes the folder); a folder that can't be removed is left for [`sweep`].
    pub fn discard(self) {
        let Work { folder, held } = self;
        drop(held);
        if let Err(error) = std::fs::remove_dir_all(&folder) {
            log::warn!("working folder {}: {error}", folder.display());
        }
    }
}

/// The folder working folders are made in.
// Human: the tests keep out of the home folder.
fn root() -> PathBuf {
    if cfg!(test) {
        return std::env::temp_dir().join(format!("pencraft-open-test-{}", std::process::id()));
    }
    crate::imagery::store::home().map_or_else(|| std::env::temp_dir().join("pencraft-open"), |home| home.join(".pencraft").join("open"))
}

/// Removes the working folders no running Pencraft holds.
// Human: a folder is in use while its lock file is locked. One made a moment ago may not be locked yet and is
// left alone; one without a lock file is rubbish, and goes after a day.
// Agent: READS and WRITES under the working folders' root; BLOCKS on removing files: call it off the main thread.
pub fn sweep() {
    let Ok(folders) = std::fs::read_dir(root()) else { return };
    for folder in folders.filter_map(Result::ok).map(|entry| entry.path()).filter(|path| path.is_dir()) {
        let Some(age) = std::fs::metadata(&folder).and_then(|meta| meta.modified()).ok().and_then(|at| at.elapsed().ok()) else { continue };
        let free = match File::open(folder.join(HELD)) {
            Ok(held) => age > SETTLED_AFTER && held.try_lock().is_ok(),
            Err(_) => age > UNHELD_FOR,
        };
        if free && let Err(error) = std::fs::remove_dir_all(&folder) {
            log::warn!("working folder {}: {error}", folder.display());
        }
    }
}

/// A bundle that was opened.
#[derive(Debug)]
pub struct Opened {
    /// Its document.
    pub doc: Document,
    /// The folder its files lie in.
    pub work: Work,
    /// The number of the bundle's layout, when it is higher than this version writes or the bundle holds a kind
    /// of part this version doesn't know.
    pub newer: Option<u64>,
    /// Whether its layout is one a later version wrote (of a `.quill` file or of an old bundle): saved here it
    /// is written in this version's layout, and parts this version doesn't know may be left out.
    pub later: bool,
    /// The parts of it of kinds this version doesn't know, to be packed with the document again ([`pack`]).
    pub kept: Vec<quill::Entry>,
    /// A sentence for each file in it that was damaged and left out.
    pub told: Vec<String>,
}

/// What a bundle holds, read into memory.
struct Parts {
    doc: Document,
    /// Its files by their path inside it.
    files: Vec<(String, Vec<u8>)>,
    newer: Option<u64>,
    later: bool,
    kept: Vec<quill::Entry>,
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
    let Parts { doc, files, newer, later, kept, told } = parts;
    let work = Work::fresh()?;
    let unpacked = files.iter().filter(|(name, _)| name != MANIFEST && name != HELD).try_for_each(|(name, bytes)| {
        let file = work.folder.join(name);
        file.parent().map_or(Ok(()), std::fs::create_dir_all)?;
        std::fs::write(file, bytes)
    });
    match unpacked {
        Ok(()) => Ok(Opened { doc, work, newer, later, kept, told }),
        Err(error) => {
            let why = format!("it couldn't be unpacked to {}: {error}", work.folder.display());
            work.discard();
            Err(why)
        }
    }
}

/// The parts of the `.quill` file `bytes`.
// Human: a damaged document refuses the file. A damaged picture is left out and said, and the rest opens: the
// design is worth more than the one picture. Parts of a kind this version doesn't know are kept in memory, not
// unpacked: their names are a later version's to read, and nothing here opens them.
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
    // Human: a part of a kind this version doesn't know has a name that is a later version's to read; it is not
    // shown as a file's.
    let told = unpacked.damaged.iter().map(|(kind, name)| {
        if *kind == quill::FILE {
            format!("The file {name} in it is damaged and was left out.")
        } else {
            "A part of it that this version doesn't know is damaged and was left out.".to_owned()
        }
    });
    let told = told.collect();
    let later = unpacked.newer.is_some_and(|layout| layout > quill::LAYOUT);
    Ok(Parts { doc: doc.ok_or("it holds no document")?, files, newer: unpacked.newer.map(u64::from), later, kept: unpacked.unknown, told })
}

/// The parts of the old bundle `bytes`, a zip archive.
// Human: one with a higher layout number is "later": it is saved as a `.quill` file with the document and the
// files it names, so what else a later version put in it isn't promised to be kept.
fn from_zip(bytes: &[u8]) -> Result<Parts, String> {
    let files = inside(zip::read(bytes, MOST_BYTES, &|_| true)?)?;
    let text = files.iter().find(|(name, _)| name == DOCUMENT).map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned());
    let doc = pen_doc::read(&text.ok_or("it holds no document")?).map_err(|error| error.to_string())?;
    let manifest = files.iter().find(|(name, _)| name == MANIFEST).and_then(|(_, bytes)| serde_json::from_slice::<Value>(bytes).ok());
    let newer = manifest.as_ref().and_then(|manifest| manifest.get("version")).and_then(Value::as_u64).filter(|layout| *layout > OLD_LAYOUT);
    Ok(Parts { doc, files, newer, later: newer.is_some(), kept: Vec::new(), told: Vec::new() })
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

/// The files of a bundle by their path inside it, from the archive's entries.
///
/// # Errors
/// A sentence when a name leads outside the bundle.
// Human: a bundle someone unpacked and zipped again with the Finder has everything in one folder, and the
// Finder's own files beside it; both are seen through. A name is taken apart and put together again from its
// plain parts only, so no `..`, drive or root in it reaches the disk.
fn inside(entries: Vec<zip::Entry>) -> Result<Vec<(String, Vec<u8>)>, String> {
    let ours = |name: &str| !name.starts_with("__MACOSX/") && name.rsplit('/').next() != Some(".DS_Store");
    let entries: Vec<zip::Entry> = entries.into_iter().filter(|entry| ours(&entry.name)).collect();
    let wrapped = entries.iter().all(|entry| entry.name != DOCUMENT);
    let prefix = entries.iter().find_map(|entry| entry.name.strip_suffix(DOCUMENT).filter(|folder| wrapped && folder.matches('/').count() == 1));
    let prefix = prefix.unwrap_or_default().to_owned();
    let mut files = Vec::new();
    for entry in entries {
        let Some(name) = entry.name.strip_prefix(&prefix) else { continue };
        let plain = relative(name).ok_or_else(|| format!("it names a file outside itself ({})", entry.name))?;
        files.push((plain, entry.bytes));
    }
    Ok(files)
}

/// `path` as a path inside a folder, its parts joined by `/`: `None` when it is empty, whole, or leaves the
/// folder.
#[must_use]
pub fn relative(path: &str) -> Option<String> {
    if path.contains(['\\', ':']) {
        return None;
    }
    let mut parts = Vec::new();
    for part in Path::new(path).components() {
        match part {
            Component::Normal(part) => parts.push(part.to_str()?),
            Component::CurDir => {}
            _ => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The files `doc` names by a path inside its folder, each once, in order: the `url` of every picture and
/// shader, the pictures a shader is given, and the documents it imports.
// Human: read from the document as JSON, so what an instance overrides and what lies in keys this version
// doesn't know is found too. Web addresses, whole paths and paths that leave the folder aren't among them.
#[must_use]
pub fn named(doc: &Document) -> Vec<String> {
    fn look(value: &Value, found: &mut BTreeSet<String>) {
        let mut add = |value: &Value| {
            let url = value.as_str().map(str::trim).filter(|url| crate::images::resolve(None, url).is_none());
            found.extend(url.filter(|url| crate::images::resolve(Some(Path::new("")), url).is_some()).and_then(relative));
        };
        match value {
            Value::Object(object) => {
                object.get("url").into_iter().for_each(&mut add);
                object.get("uniforms").and_then(Value::as_object).into_iter().flat_map(serde_json::Map::values).for_each(&mut add);
                object.values().for_each(|inner| look(inner, found));
            }
            Value::Array(list) => list.iter().for_each(|inner| look(inner, found)),
            _ => {}
        }
    }
    let mut found = BTreeSet::new();
    if let Ok(layers) = serde_json::to_value(&doc.children) {
        look(&layers, &mut found);
    }
    let imports = doc.imports.iter().flat_map(serde_json::Map::values).filter_map(Value::as_str);
    found.extend(imports.filter(|url| crate::images::resolve(None, url).is_none()).filter_map(relative));
    found.into_iter().collect()
}

/// What a bundle was packed from.
#[derive(Debug, PartialEq, Eq)]
pub struct Packed {
    /// The bundle's bytes.
    pub bytes: Vec<u8>,
    /// The files the document names that weren't in the folder, and so aren't in the bundle.
    pub missing: Vec<String>,
}

/// Packs `doc` and the files it names in `folder` into a bundle, and with them `kept`: the parts the bundle had
/// when it was opened that this version doesn't know ([`Opened::kept`]), as they are.
///
/// # Errors
/// A sentence when the document can't be written, a file can't be read, or the document, the kept parts and
/// the files come to more than a bundle may unpack to (2 GB): such a file would be written and never open again.
// Human: only what the document names goes in, so a picture that was tried and thrown away doesn't travel
// with it; the credits are cut down to the pictures that go in, for the same reason. What a later version put
// in the file goes back in after the document and before the files, so saving here doesn't lose it.
// Agent: READS the named files whole into memory; BLOCKS on them.
pub fn pack(doc: &Document, folder: &Path, kept: &[quill::Entry]) -> Result<Packed, String> {
    let entry = |name: &str, bytes: Vec<u8>| quill::Entry { kind: quill::FILE, name: name.to_owned(), bytes };
    let document = pen_doc::binary::write(doc).map_err(|error| error.to_string())?;
    let mut entries = vec![quill::Entry { kind: quill::DOCUMENT, name: String::new(), bytes: document }];
    entries.extend(kept.iter().filter(|entry| entry.kind != quill::DOCUMENT && entry.kind != quill::FILE).cloned());
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
        && entries.iter().all(|entry| entry.kind != quill::FILE || entry.name != credits)
    {
        entries.push(entry(&credits, credits_of));
    }
    fits(&entries, MOST_BYTES)?;
    Ok(Packed { bytes: quill::write(&entries)?, missing })
}

/// Whether `entries` are a bundle that opens again: together at most `most` bytes.
///
/// # Errors
/// A sentence saying how large a design and its files may be.
// Human: opening counts every entry with its full length, also one that shares its block with another
// (`quill::read`); what is counted here is never less than that, so what is saved opens.
fn fits(entries: &[quill::Entry], most: u64) -> Result<(), String> {
    const GB: u64 = 1024 * 1024 * 1024;
    let length = |entry: &quill::Entry| u64::try_from(entry.bytes.len()).unwrap_or(u64::MAX);
    let all = entries.iter().map(length).fold(0u64, u64::saturating_add);
    if all <= most {
        return Ok(());
    }
    let size = if most >= GB && most.is_multiple_of(GB) { format!("{} GB", most / GB) } else { format!("{most} bytes") };
    Err(format!("the design and its files are too large for one .quill file: together they may be {size} at most"))
}

/// The credits file at `file`, cut down to the pictures among `entries`; `None` without a file or a line left.
/// A credits file that can't be read as one goes in as it is.
fn credits_for(file: &Path, entries: &[quill::Entry]) -> Option<Vec<u8>> {
    let bytes = std::fs::read(file).ok()?;
    let Ok(Value::Object(mut credits)) = serde_json::from_slice::<Value>(&bytes) else { return Some(bytes) };
    let files = || entries.iter().filter(|entry| entry.kind == quill::FILE);
    credits.retain(|name, _| files().any(|entry| entry.name.strip_prefix(FOLDER).and_then(|rest| rest.strip_prefix('/')) == Some(name.as_str())));
    (!credits.is_empty()).then(|| serde_json::to_vec_pretty(&credits).ok()).flatten()
}

/// Copies the files `doc` names from the folder `from` to the same place in the folder `to`, where there is
/// none yet. A sentence for each that couldn't be copied.
// Human: for a document that moves into or out of a bundle: its shader files and imported documents come
// along as its pictures do (`pictures::settle`, which also brings their credits).
// Agent: READS under `from`, WRITES under `to`; BLOCKS on both.
pub fn carry(doc: &Document, from: &Path, to: &Path) -> Vec<String> {
    let mut told = Vec::new();
    for name in named(doc) {
        let (source, target) = (from.join(&name), to.join(&name));
        if !source.is_file() || target.exists() {
            continue;
        }
        let copied = target.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::copy(&source, &target));
        if let Err(error) = copied {
            told.push(format!("The file {} couldn't be brought to {}: {error}.", source.display(), to.display()));
        }
    }
    told
}

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
        let packed = pack(&doc, &from, &[]).unwrap();
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
        assert_eq!(pack(&opened.doc, &at, &opened.kept).unwrap(), packed);
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
        let whole = pack(&doc, &from, &[]).unwrap().bytes;
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
    fn what_a_later_version_put_in_a_quill_file_is_kept_when_it_is_packed_again() {
        let from = filled("later");
        let doc = pen_doc::read(DESIGN).unwrap();
        let entry = |kind: u8, name: &str, bytes: &[u8]| quill::Entry { kind, name: name.to_owned(), bytes: bytes.to_vec() };
        // Parts of kinds this version has no name for: one under a name that would lead outside were it a file,
        // one under the name of a picture.
        let chat = entry(2, "../chat", "what was said about the design. ".repeat(40).as_bytes());
        let odd = entry(9, "images/fox.png", b"not a picture");
        let mut parts = quill::read(&pack(&doc, &from, &[]).unwrap().bytes, MOST_BYTES).unwrap().entries;
        parts.insert(1, chat.clone());
        parts.push(odd.clone());
        let file = from.join("design.quill");
        std::fs::write(&file, quill::write(&parts).unwrap()).unwrap();
        let opened = open(&file).unwrap();
        assert_eq!((opened.newer, opened.later, opened.told.len()), (Some(1), false, 0));
        assert_eq!(opened.kept, [chat, odd]);
        let at = opened.work.folder().to_path_buf();
        assert_eq!(std::fs::read(at.join("images/fox.png")).unwrap(), FOX);
        // Packed again they are in the file byte for byte, after the document and before the files (the kind of
        // the second entry of the index is the chat's), and the files are the same as without them.
        let packed = pack(&opened.doc, &at, &opened.kept).unwrap().bytes;
        let again = quill::read(&packed, MOST_BYTES).unwrap();
        assert_eq!((again.unknown, packed[64]), (opened.kept.clone(), 2));
        assert_eq!(again.entries, quill::read(&pack(&doc, &from, &[]).unwrap().bytes, MOST_BYTES).unwrap().entries);
        opened.work.discard();
        // A file of a later layout says so: what is lost by saving it here can't be known.
        let mut later = quill::write(&parts).unwrap();
        later[8..12].copy_from_slice(&2u32.to_le_bytes());
        std::fs::write(&file, later).unwrap();
        let opened = open(&file).unwrap();
        assert_eq!((opened.newer, opened.later, opened.kept.len()), (Some(2), true, 2));
        opened.work.discard();
        // One of the parts is damaged: it is left out, and that is said without its name being taken for a file's.
        let whole = quill::write(&parts).unwrap();
        let said = b"what was said about the design. what was said";
        let mut spoiled = quill::write(&[parts[0].clone(), entry(2, "../chat.png", said), parts[2].clone()]).unwrap();
        let at = spoiled.windows(said.len()).position(|window| window == said).unwrap();
        spoiled[at] ^= 0x55;
        std::fs::write(&file, spoiled).unwrap();
        let opened = open(&file).unwrap();
        assert_eq!(opened.told, ["A part of it that this version doesn't know is damaged and was left out."]);
        assert_eq!((opened.newer, opened.kept.len()), (Some(1), 0));
        opened.work.discard();
        // What is packed is counted as it is counted when it is opened: every entry whole, also a shared block.
        let entries = quill::read(&whole, MOST_BYTES).unwrap();
        let all: Vec<quill::Entry> = entries.entries.into_iter().chain(entries.unknown).collect();
        let length: u64 = all.iter().map(|entry| u64::try_from(entry.bytes.len()).unwrap()).sum();
        assert_eq!(fits(&all, length), Ok(()));
        assert!(quill::read(&quill::write(&all).unwrap(), length).is_ok());
        assert_eq!(
            fits(&all, length - 1).unwrap_err(),
            format!("the design and its files are too large for one .quill file: together they may be {} bytes at most", length - 1)
        );
        assert!(quill::read(&quill::write(&all).unwrap(), length - 1).is_err());
        let twice = [entry(quill::FILE, "a.png", &[7; 600]), entry(quill::FILE, "b.png", &[7; 600])];
        assert!(fits(&twice, 1199).is_err() && fits(&twice, 1200).is_ok());
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
            assert_eq!((opened.newer, opened.later), (Some(7), true), "{name}");
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
    fn endings_say_what_a_file_is_and_where_it_is_saved() {
        let (quill, old, pen) = (Path::new("/a/site.QUILL"), Path::new("/a/site.pencraft"), Path::new("/a/site.pen"));
        assert!(is_bundle(quill) && is_bundle(old) && !is_bundle(pen) && !is_bundle(Path::new("/a/quill")));
        assert!(is_old(old) && !is_old(quill) && !is_old(pen));
        assert!(is_design(pen) && is_design(quill) && is_design(old) && !is_design(Path::new("a.png")));
        // A `.pen` document and an old bundle are saved as the `.quill` file of their name; a `.quill` file where it is.
        assert_eq!(successor(old), Some(PathBuf::from("/a/site.quill")));
        assert_eq!(successor(pen), Some(PathBuf::from("/a/site.quill")));
        assert_eq!(successor(Path::new("/a/site.PEN")), Some(PathBuf::from("/a/site.quill")));
        // A file of another name, which opened as a plain document, keeps its name before the new ending.
        assert_eq!(successor(Path::new("/a/design.json")), Some(PathBuf::from("/a/design.json.quill")));
        assert_eq!(successor(Path::new("/a/site.pen.bak")), Some(PathBuf::from("/a/site.pen.bak.quill")));
        assert_eq!(successor(Path::new("/a/site")), Some(PathBuf::from("/a/site.quill")));
        assert_eq!((successor(quill), successor(Path::new("/a/site.quill")), successor(Path::new("/"))), (None, None, None));
        // A name gets the ending of what is written after it, never in place of a part, and keeps it as typed.
        for (typed, saved, exported) in [
            ("/a/site", "/a/site.quill", "/a/site.pen"),
            ("/a/home.v2", "/a/home.v2.quill", "/a/home.v2.pen"),
            ("/a/site.", "/a/site.quill", "/a/site.pen"),
            ("/a.b/site", "/a.b/site.quill", "/a.b/site.pen"),
            ("/a/.hidden", "/a/.hidden.quill", "/a/.hidden.pen"),
        ] {
            assert_eq!(ended(PathBuf::from(typed), ENDING), Ok(PathBuf::from(saved)), "{typed}");
            assert_eq!(ended(PathBuf::from(typed), PLAIN), Ok(PathBuf::from(exported)), "{typed}");
        }
        assert_eq!(ended(PathBuf::from("/a/site.QUILL"), ENDING), Ok(PathBuf::from("/a/site.QUILL")));
        assert_eq!(ended(PathBuf::from("/a/site.PEN"), PLAIN), Ok(PathBuf::from("/a/site.PEN")));
        // A name that asks for another kind of design file is refused, with how that one is made.
        let said = |typed: &str, ending: &str| ended(PathBuf::from(typed), ending).unwrap_err();
        assert_eq!(said("/a/site.pen", ENDING), "a .pen file is not saved but exported: use File \u{203a} Export .pen File\u{2026}");
        assert_eq!(said("/a/site.Pen", ENDING), said("/a/site.pen", ENDING));
        assert_eq!(said("/a/site.pencraft", ENDING), "a .pencraft file is no longer written");
        assert_eq!(said("/a/site.pencraft", PLAIN), "a .pencraft file is no longer written");
        assert_eq!(said("/a/site.quill", PLAIN), "a .quill file is not exported but saved: use File \u{203a} Save As\u{2026}");
        // A name that got its ending added is not written over a file that is there: nobody was asked about it.
        let there = folder("ended");
        std::fs::write(there.join("home.v2.quill"), b"somebody's").unwrap();
        std::fs::write(there.join("copy.pen"), b"somebody's").unwrap();
        assert_eq!(ended(there.join("home.v2"), ENDING).unwrap_err(), "home.v2.quill is there already: type the whole name to replace it");
        assert_eq!(ended(there.join("copy"), PLAIN).unwrap_err(), "copy.pen is there already: type the whole name to replace it");
        assert_eq!(ended(there.join("home.v2.quill"), ENDING), Ok(there.join("home.v2.quill")));
        assert_eq!(ended(there.join("copy.pen"), PLAIN), Ok(there.join("copy.pen")));
        assert_eq!(ended(there.join("copy"), ENDING), Ok(there.join("copy.quill")));
        std::fs::remove_dir_all(&there).unwrap();
        // What names no file is refused, also when its last part looks like a design's name.
        for none in ["/", "/a/..", "/a/site.quill/..", ""] {
            assert!(ended(PathBuf::from(none), ENDING).is_err_and(|said| said.ends_with("names no file")), "{none}");
        }
        assert!(looks_packed(&quill::MAGIC) && looks_packed(b"PK\x03\x04") && !looks_packed(b"{\"version\""));
    }
}
