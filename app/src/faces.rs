//! The font faces installed on this computer, by family name.
//!
//! A design's text names a font family and a weight. To draw it in that face the app has to know which font
//! file holds it, so the system's font folders are read once: each file's family name, weight, slant and (for
//! a variable font) its range of weights. Only the few tables that say so are read, never a whole file.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// How deep a font folder's subfolders are looked into.
const DEPTH: usize = 3;
/// The most faces one font collection file is read for.
const MOST_IN_FILE: u32 = 64;
/// The most bytes of one font table that are read.
const MOST_TABLE: u32 = 1 << 20;
/// Families that hold signs instead of letters; a design's text can't be set in them.
const NO_LETTERS: [&str; 9] = ["wingdings", "webdings", "symbol", "marlett", "assets", "icons", "emoji", "holo", "dingbats"];
/// Families made for other scripts or for the system's own use, by how their names start; they are installed on
/// every computer of their kind, and a design's Latin text is not what they are for.
const NOT_FOR_TEXT: [&str; 22] = [
    "microsoft ",
    "ms ",
    "simsun",
    "nsimsun",
    "mingliu",
    "pmingliu",
    "yu gothic",
    "malgun",
    "leelawadee",
    "nirmala",
    "ebrima",
    "gadugi",
    "javanese",
    "mongolian",
    "myanmar",
    "mv boli",
    "sylfaen",
    "mt extra",
    "cambria math",
    "segoe ui historic",
    "sans serif collection",
    "power ",
];
/// What stands in for a sans-serif family that isn't installed, first match wins.
// Human: the format's default family is "Inter", which most computers don't have; the system's own interface
// face is the nearest thing with a full set of weights.
const SANS: [&str; 7] = ["Inter", "Segoe UI", "Helvetica Neue", "Helvetica", "Arial", "Liberation Sans", "DejaVu Sans"];
/// What stands in for a serif family that isn't installed.
const SERIF: [&str; 5] = ["Georgia", "Times New Roman", "Times", "Liberation Serif", "DejaVu Serif"];
/// What stands in for a fixed-width family that isn't installed.
const MONO: [&str; 6] = ["Cascadia Code", "Consolas", "SF Mono", "Menlo", "Liberation Mono", "DejaVu Sans Mono"];

/// One face: a family in one weight, upright or italic.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    /// The font file.
    pub path: PathBuf,
    /// Which face of the file (a collection holds several).
    pub index: u32,
    /// The weight, 100 to 900; for a variable font the weight it has when none is asked for.
    pub weight: u16,
    /// Whether it is an italic.
    pub italic: bool,
    /// For a variable font: the lightest and the heaviest weight it can take.
    pub weights: Option<(f32, f32)>,
}

/// A family and its faces.
struct Family {
    /// The name as the font writes it.
    name: String,
    faces: Vec<Face>,
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes([*bytes.get(at)?, *bytes.get(at + 1)?, *bytes.get(at + 2)?, *bytes.get(at + 3)?]))
}

/// `len` bytes of `file` from `at`.
fn read_at(file: &mut (impl Read + Seek), at: u32, len: u32) -> Option<Vec<u8>> {
    let mut bytes = vec![0; usize::try_from(len.min(MOST_TABLE)).ok()?];
    file.seek(SeekFrom::Start(u64::from(at))).ok()?;
    file.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

/// The table `tag` of the font whose table records are `records`, at most `most` bytes of it.
fn table(file: &mut (impl Read + Seek), records: &[u8], tag: [u8; 4], most: u32) -> Option<Vec<u8>> {
    let record = records.as_chunks::<16>().0.iter().find(|record| record.starts_with(&tag))?;
    read_at(file, u32_at(record, 8)?, u32_at(record, 12)?.min(most))
}

/// The family name a font's `name` table gives.
// Human: a font can name its family twice: the old way, where "Segoe UI Semibold" is a family of its own, and
// the typographic way (name 16), where it is the semibold of "Segoe UI". The typographic one is wanted, in
// English when the font has it.
fn family_name(names: &[u8]) -> Option<String> {
    let count = usize::from(u16_at(names, 2)?);
    let strings = usize::from(u16_at(names, 4)?);
    let mut best: Option<(u8, String)> = None;
    for record in names.get(6..)?.as_chunks::<12>().0.iter().take(count) {
        let (platform, language, name) = (u16_at(record, 0)?, u16_at(record, 4)?, u16_at(record, 6)?);
        if name != 16 && name != 1 {
            continue;
        }
        let start = strings + usize::from(u16_at(record, 10)?);
        let Some(bytes) = names.get(start..start + usize::from(u16_at(record, 8)?)) else { continue };
        let (rank, text) = match (platform, language) {
            (3, 0x409) | (0, _) => (3, utf16(bytes)),
            (1, 0) => (2, bytes.iter().map(|byte| char::from(*byte)).collect()),
            (3, _) => (1, utf16(bytes)),
            _ => continue,
        };
        let rank = rank + if name == 16 { 10 } else { 0 };
        let text = text.trim().to_owned();
        if !text.is_empty() && best.as_ref().is_none_or(|(had, _)| rank > *had) {
            best = Some((rank, text));
        }
    }
    best.map(|(_, name)| name)
}

fn utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|pair| u16::from_be_bytes(*pair)).collect();
    String::from_utf16_lossy(&units)
}

/// The lightest and heaviest weight of a variable font, from its `fvar` table.
fn weight_range(fvar: &[u8]) -> Option<(f32, f32)> {
    let (start, count, size) = (usize::from(u16_at(fvar, 4)?), usize::from(u16_at(fvar, 8)?), usize::from(u16_at(fvar, 10)?));
    #[expect(clippy::cast_precision_loss, reason = "a 16.16 fixed-point weight, far below where f32 loses whole numbers")]
    let fixed = |at: usize| u32_at(fvar, at).map(|bits| i32::from_be_bytes(bits.to_be_bytes()) as f32 / 65536.0);
    (0..count)
        .map(|axis| start + axis * size)
        .find(|at| fvar.get(*at..*at + 4) == Some(b"wght"))
        .and_then(|at| Some((fixed(at + 4)?, fixed(at + 12)?)))
}

/// The faces in the font file `file` (several for a collection), each with its family's name.
fn faces_in(file: &mut (impl Read + Seek), path: &Path) -> Vec<(String, Face)> {
    let mut found = Vec::new();
    let Some(head) = read_at(file, 0, 12) else { return found };
    let starts: Vec<u32> = if head.starts_with(b"ttcf") {
        let count = u32_at(&head, 8).unwrap_or(0).min(MOST_IN_FILE);
        let Some(list) = read_at(file, 12, count * 4) else { return found };
        list.as_chunks::<4>().0.iter().map(|start| u32::from_be_bytes(*start)).collect()
    } else {
        vec![0]
    };
    for (index, start) in (0u32..).zip(starts) {
        let face = (|| {
            let count = u16_at(&read_at(file, start, 12)?, 4)?;
            let records = read_at(file, start.checked_add(12)?, u32::from(count) * 16)?;
            let family = family_name(&table(file, &records, *b"name", MOST_TABLE)?)?;
            let os2 = table(file, &records, *b"OS/2", 64).unwrap_or_default();
            let weight = u16_at(&os2, 4).filter(|weight| (1..=1000).contains(weight)).unwrap_or(400);
            let italic = u16_at(&os2, 62).is_some_and(|selection| selection & 1 == 1);
            let weights = table(file, &records, *b"fvar", 4096).and_then(|fvar| weight_range(&fvar));
            Some((family, Face { path: path.to_owned(), index, weight, italic, weights }))
        })();
        found.extend(face);
    }
    found
}

/// The folders fonts are installed in, on any of the systems the app runs on.
fn folders() -> Vec<PathBuf> {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let mut all = Vec::new();
    all.extend(var("SystemRoot").or_else(|| var("windir")).map(|windows| windows.join("Fonts")));
    all.extend(var("LOCALAPPDATA").map(|local| local.join("Microsoft").join("Windows").join("Fonts")));
    all.extend(["/System/Library/Fonts", "/Library/Fonts", "/usr/share/fonts", "/usr/local/share/fonts"].map(PathBuf::from));
    if let Some(home) = var("HOME") {
        all.extend([home.join("Library/Fonts"), home.join(".fonts"), home.join(".local/share/fonts")]);
    }
    all
}

/// Reads every font file in `folder` and the folders in it into `families`.
fn gather(folder: &Path, depth: usize, families: &mut BTreeMap<String, Family>) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth > 0 {
                gather(&path, depth - 1, families);
            }
            continue;
        }
        let ending = path.extension().map(|ending| ending.to_string_lossy().to_ascii_lowercase());
        if !matches!(ending.as_deref(), Some("ttf" | "otf" | "ttc" | "otc")) {
            continue;
        }
        let Ok(mut file) = File::open(&path) else { continue };
        for (name, face) in faces_in(&mut file, &path) {
            // A family whose name starts with a dot is one the system keeps for itself.
            if name.starts_with('.') {
                continue;
            }
            let family = families.entry(name.to_lowercase()).or_insert_with(|| Family { name, faces: Vec::new() });
            if !family.faces.iter().any(|had| had.weight == face.weight && had.italic == face.italic && had.weights == face.weights) {
                family.faces.push(face);
            }
        }
    }
}

/// Every installed family, by its name in small letters. Read once, when first asked for.
fn families() -> &'static BTreeMap<String, Family> {
    static FAMILIES: OnceLock<BTreeMap<String, Family>> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        let mut families = BTreeMap::new();
        for folder in folders() {
            gather(&folder, DEPTH, &mut families);
        }
        families
    })
}

/// Starts reading the installed families on a thread of its own, so the first [`installed`] or [`pick`] finds
/// them ready (or waits only for what is left) instead of reading every font file on the window's thread.
// Human: the system's fonts are many files to open and parse; doing that while the window draws its first
// frames would hold it still.
// Agent: SPAWNS one short-lived thread that fills the `families` OnceLock; a caller on another thread blocks
// in `get_or_init` until it is done. Not starting the thread only means the first caller reads them itself.
pub fn warm() {
    if let Err(why) = std::thread::Builder::new().name("faces".to_owned()).spawn(|| {
        families();
    }) {
        log::warn!("couldn't read the installed fonts ahead: {why}");
    }
}

/// The face of `faces` nearest to `weight`, italic if `italic` and there is one.
fn nearest(faces: &[Face], weight: u16, italic: bool) -> Option<&Face> {
    let slanted = faces.iter().any(|face| face.italic == italic);
    let wanted = f32::from(weight);
    let off = |face: &Face| match face.weights {
        Some((lightest, heaviest)) => (wanted.clamp(lightest, heaviest) - wanted).abs(),
        None => (f32::from(face.weight) - wanted).abs(),
    };
    faces.iter().filter(|face| !slanted || face.italic == italic).min_by(|a, b| off(a).total_cmp(&off(b)))
}

/// Whether the family `name` is installed.
#[must_use]
pub fn installed(name: &str) -> bool {
    families().contains_key(&name.trim().to_lowercase())
}

/// The families that stand in for `name` when it isn't installed, by what kind of face its name says it is.
fn stand_ins(name: &str) -> &'static [&'static str] {
    let name = name.to_lowercase();
    if name.contains("mono") || name.contains("code") || name.contains("courier") {
        &MONO
    } else if (name.contains("serif") && !name.contains("sans"))
        || ["georgia", "times", "garamond", "playfair", "merriweather", "lora"].iter().any(|serif| name.contains(serif))
    {
        &SERIF
    } else {
        &SANS
    }
}

/// The installed face to draw text of the family `name` at `weight` in: the family's own face nearest to the
/// weight, or that of a family standing in for it. `None` when nothing fitting is installed.
#[must_use]
pub fn pick(name: &str, weight: u16, italic: bool) -> Option<&'static Face> {
    let families = families();
    let own = families.get(&name.trim().to_lowercase());
    let family = own.or_else(|| stand_ins(name).iter().find_map(|stand_in| families.get(&stand_in.to_lowercase())))?;
    nearest(&family.faces, weight, italic)
}

/// How far the heaviest and the lightest weight a face is drawn in may be from the weight asked for before
/// it is worth saying: one step of the usual scale (400 for 500) is how most families are built.
const WEIGHT_OFF: f32 = 150.0;

/// What the installed family `name` lacks for text that asks for `weight` and, with `italic`, an italic:
/// the weight it is drawn in instead when the family has no face near the one asked for, and that its
/// upright is slanted when it has no italic. `None` when the family isn't installed (which is said
/// elsewhere) or has what was asked for.
#[must_use]
pub fn falls_short(name: &str, weight: u16, italic: bool) -> Option<String> {
    let family = families().get(&name.trim().to_lowercase())?;
    let face = nearest(&family.faces, weight, italic)?;
    let wanted = f32::from(weight);
    let drawn = face.weights.map_or(f32::from(face.weight), |(lightest, heaviest)| wanted.clamp(lightest, heaviest));
    let mut said = Vec::new();
    if (drawn - wanted).abs() >= WEIGHT_OFF {
        said.push(format!("has no{} weight {weight}; it is drawn in {drawn:.0}", if italic && face.italic { " italic of" } else { "" }));
    }
    if italic && !face.italic {
        said.push("has no italic; its upright is slanted instead".to_owned());
    }
    (!said.is_empty()).then(|| said.join(", and "))
}

/// The name of the family that is drawn when text asks for `name`.
#[must_use]
pub fn shown_as(name: &str) -> Option<&'static str> {
    let families = families();
    let own = families.get(&name.trim().to_lowercase());
    own.or_else(|| stand_ins(name).iter().find_map(|stand_in| families.get(&stand_in.to_lowercase()))).map(|family| family.name.as_str())
}

/// The installed families a design's text can be set in, by name, with the weights each comes in
/// (`100-900` for a variable font).
#[must_use]
pub fn listed() -> Vec<String> {
    families()
        .values()
        .filter(|family| {
            let name = family.name.to_lowercase();
            // Human: families in other scripts name themselves in letters a model can't tell apart by look;
            // the list is for picking a Latin text face.
            name.is_ascii() && !NO_LETTERS.iter().any(|signs| name.contains(signs)) && !NOT_FOR_TEXT.iter().any(|start| name.starts_with(start))
        })
        .map(|family| {
            let mut weights: Vec<String> = Vec::new();
            let mut upright: Vec<&Face> = family.faces.iter().filter(|face| !face.italic).collect();
            if upright.is_empty() {
                upright = family.faces.iter().collect();
            }
            upright.sort_by_key(|face| face.weight);
            for face in upright {
                let text = match face.weights {
                    #[expect(clippy::cast_possible_truncation, reason = "a font weight, 1 to 1000")]
                    Some((lightest, heaviest)) => format!("{}-{}", lightest.round() as i32, heaviest.round() as i32),
                    None => face.weight.to_string(),
                };
                if !weights.contains(&text) {
                    weights.push(text);
                }
            }
            format!("{} ({})", family.name, weights.join(", "))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    /// A font file with a `name`, an `OS/2` and (when `range` is given) an `fvar` table.
    fn font(family: &str, typographic: Option<&str>, weight: u16, italic: bool, range: Option<(u16, u16)>) -> Vec<u8> {
        let mut strings = Vec::new();
        let mut records = Vec::new();
        for (id, text) in [(1u16, Some(family)), (16, typographic)] {
            let Some(text) = text else { continue };
            let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_be_bytes).collect();
            for part in [3u16, 1, 0x409, id, u16::try_from(bytes.len()).unwrap(), u16::try_from(strings.len()).unwrap()] {
                records.extend(part.to_be_bytes());
            }
            strings.extend(bytes);
        }
        let count = u16::try_from(records.len() / 12).unwrap();
        let mut name = Vec::new();
        for part in [0u16, count, 6 + count * 12] {
            name.extend(part.to_be_bytes());
        }
        name.extend(records);
        name.extend(strings);
        let mut os2 = vec![0u8; 64];
        os2[4..6].copy_from_slice(&weight.to_be_bytes());
        os2[62..64].copy_from_slice(&u16::from(italic).to_be_bytes());
        let mut tables: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"OS/2", os2), (b"name", name)];
        if let Some((lightest, heaviest)) = range {
            let mut fvar = Vec::new();
            for part in [1u16, 0, 16, 2, 1, 20] {
                fvar.extend(part.to_be_bytes());
            }
            fvar.extend([0; 4]);
            fvar.extend(b"wght");
            for value in [lightest, 400, heaviest] {
                fvar.extend((u32::from(value) << 16).to_be_bytes());
            }
            fvar.extend([0; 4]);
            tables.push((b"fvar", fvar));
        }
        let mut file = Vec::new();
        file.extend(0x0001_0000u32.to_be_bytes());
        file.extend(u16::try_from(tables.len()).unwrap().to_be_bytes());
        file.extend([0; 6]);
        let mut at = 12 + tables.len() * 16;
        for (tag, bytes) in &tables {
            file.extend(*tag);
            file.extend([0; 4]);
            file.extend(u32::try_from(at).unwrap().to_be_bytes());
            file.extend(u32::try_from(bytes.len()).unwrap().to_be_bytes());
            at += bytes.len();
        }
        for (_, bytes) in tables {
            file.extend(bytes);
        }
        file
    }

    fn read(bytes: Vec<u8>) -> Vec<(String, Face)> {
        faces_in(&mut Cursor::new(bytes), Path::new("font.ttf"))
    }

    #[test]
    fn a_font_file_says_its_family_weight_slant_and_range() {
        let plain = read(font("Segoe UI Semibold", Some("Segoe UI"), 600, false, None));
        assert_eq!(plain.len(), 1);
        assert_eq!((plain[0].0.as_str(), plain[0].1.weight, plain[0].1.italic, plain[0].1.weights), ("Segoe UI", 600, false, None));
        let variable = read(font("Bahnschrift", None, 400, true, Some((300, 700))));
        assert_eq!((variable[0].0.as_str(), variable[0].1.italic, variable[0].1.weights), ("Bahnschrift", true, Some((300.0, 700.0))));
        // A file that isn't a font, or is cut off, gives nothing and doesn't fail.
        assert_eq!(read(b"not a font at all".to_vec()), []);
        let mut cut = font("Cut", None, 400, false, None);
        cut.truncate(40);
        assert_eq!(read(cut), []);
    }

    #[test]
    fn the_nearest_weight_is_picked_and_an_italic_when_there_is_one() {
        let face = |weight, italic, weights| Face { path: PathBuf::new(), index: 0, weight, italic, weights };
        let faces = [face(300, false, None), face(400, false, None), face(700, false, None), face(400, true, None)];
        assert_eq!(nearest(&faces, 800, false).map(|face| face.weight), Some(700));
        assert_eq!(nearest(&faces, 320, false).map(|face| face.weight), Some(300));
        assert_eq!(nearest(&faces, 700, true).map(|face| (face.weight, face.italic)), Some((400, true)));
        // A variable font covers every weight in its range; without an italic the upright stands in.
        let variable = [face(400, false, Some((100.0, 900.0))), face(700, false, None)];
        assert_eq!(nearest(&variable, 500, true).map(|face| face.weights.is_some()), Some(true));
        assert_eq!(stand_ins("JetBrains Mono")[0], MONO[0]);
        assert_eq!(stand_ins("Playfair Display")[0], SERIF[0]);
        assert_eq!(stand_ins("Open Sans")[0], SANS[0]);
    }
}
