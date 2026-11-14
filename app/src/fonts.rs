//! Fonts for the interface and the canvas.
//!
//! egui's bundled face is a light weight without bold or italic. Designs need both, so the system's Arial
//! family is used when it is installed (it is on every Mac and every Windows PC); otherwise the bundled face
//! stays and bold and italic text fall back to it. Nothing is downloaded or shipped.
//!
//! A design's text is drawn in the family and weight it names when that face is installed ([`crate::faces`]).
//! A face is read from its file the first time text asks for it, so the first frame may still show the
//! interface face; the next one shows the real one.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use egui::epaint::text::{Blob, VariationCoords};
use egui::{FontData, FontDefinitions, FontFamily};

use crate::app::lock;
use crate::faces::{self, Face};
use crate::render::{BOLD_FAMILY, ITALIC_FAMILY};

/// The faces designs are drawn in, as far as they have been handed to egui.
struct Bound {
    /// Everything egui has been given: the interface fonts and each face a design asked for.
    definitions: FontDefinitions,
    /// The fonts tried after a face, for the characters it lacks.
    fallback: Vec<String>,
    /// The faces egui draws with now, by the name they were given.
    active: HashSet<String>,
    /// The faces given to egui that it takes up with its next frame.
    sent: Vec<String>,
    /// The faces text asked for that have not been read yet.
    wanted: Vec<&'static Face>,
    /// The faces whose file couldn't be read; they are not asked for again.
    broken: HashSet<String>,
    /// The font files read so far; a collection's faces share one.
    files: HashMap<PathBuf, Blob>,
}

/// `None` until [`install`] ran (and in tests, where no text is drawn).
static BOUND: Mutex<Option<Bound>> = Mutex::new(None);

/// How text of one family, weight and slant is drawn.
pub struct Chosen {
    /// The egui family holding the face.
    pub family: FontFamily,
    /// For a variable font: the weight to set it to.
    pub coords: VariationCoords,
    /// Whether the letters are to be slanted, because the family has no italic of its own.
    pub slant: bool,
}

/// The name a face has in egui.
fn name_of(face: &Face) -> String {
    format!("face:{}:{}", face.path.display(), face.index)
}

/// The installed face for text of `family` at `weight`, once egui can draw with it. `None` when there is no
/// such face or it isn't ready yet: it is then asked for, and [`sync`] makes it ready.
// Agent: called while text is laid out, possibly inside egui's font lock: TOUCHES no egui context.
#[must_use]
pub fn choose(family: &str, weight: u16, italic: bool) -> Option<Chosen> {
    let mut bound = lock(&BOUND);
    let bound = bound.as_mut()?;
    let face = faces::pick(family, weight, italic)?;
    let name = name_of(face);
    if bound.active.contains(&name) {
        let coords = face.weights.map_or_else(VariationCoords::default, |(lightest, heaviest)| {
            VariationCoords::new([(b"wght", f32::from(weight).clamp(lightest, heaviest))])
        });
        return Some(Chosen { family: FontFamily::Name(name.into()), coords, slant: italic && !face.italic });
    }
    if !bound.broken.contains(&name) && !bound.sent.contains(&name) && !bound.wanted.iter().any(|had| std::ptr::eq(*had, face)) {
        bound.wanted.push(face);
    }
    None
}

/// Asks for the face of `family` at `weight` ahead of the text that will use it, so [`sync`] can make it ready
/// before that text is measured.
pub fn want(family: &str, weight: u16, italic: bool) {
    drop(choose(family, weight, italic));
}

/// Whether faces are waiting to be made ready, so another frame is needed.
#[must_use]
pub fn pending() -> bool {
    lock(&BOUND).as_ref().is_some_and(|bound| !bound.wanted.is_empty() || !bound.sent.is_empty())
}

/// Hands the faces text asked for to egui. Returns whether faces became ready, in which case every document
/// has to be laid out again: its text was measured in another face.
// Agent: MAIN-THREAD; READS font files; call outside any egui lock.
pub fn sync(ctx: &egui::Context) -> bool {
    let mut guard = lock(&BOUND);
    let Some(bound) = guard.as_mut() else { return false };
    if !bound.wanted.is_empty() {
        // Human: hinting pulls letters onto whole pixels, which suits interface text at one size; a design's
        // text is seen at every zoom, where that makes the spacing between letters uneven. Its faces are
        // drawn as the font's outlines are.
        let plain = egui::epaint::text::FontTweak { hinting: Some(false), ..Default::default() };
        for face in std::mem::take(&mut bound.wanted) {
            let name = name_of(face);
            let bytes = match bound.files.get(&face.path) {
                Some(bytes) => Some(bytes.clone()),
                None => std::fs::read(&face.path).ok().map(|bytes| -> Blob { Arc::new(bytes) }),
            };
            let Some(bytes) = bytes else {
                log::warn!("couldn't read the font file {}", face.path.display());
                bound.broken.insert(name);
                continue;
            };
            bound.files.insert(face.path.clone(), bytes.clone());
            // Human: egui puts a font's whole line gap under its letters, so a face with a large one (Bahnschrift,
            // Calibri) sits high in its line and initials look off-centre in a circle. Half the gap belongs above
            // the letters, as browsers set it; this moves the letters only, not what is measured.
            let tweak = egui::epaint::text::FontTweak { y_offset_factor: half_line_gap((*bytes).as_ref(), face.index), ..plain.clone() };
            bound.definitions.font_data.insert(name.clone(), Arc::new(FontData::from_blob(bytes, face.index).tweak(tweak)));
            let mut chain = vec![name.clone()];
            chain.extend(bound.fallback.iter().cloned());
            bound.definitions.families.insert(FontFamily::Name(name.clone().into()), chain);
            bound.sent.push(name);
        }
        ctx.set_fonts(bound.definitions.clone());
    }
    if bound.sent.is_empty() {
        return false;
    }
    // Human: egui takes new fonts up at once when no text was laid out yet in this frame, else with the next.
    let sent = &bound.sent;
    if !ctx.fonts(|fonts| sent.iter().all(|name| fonts.definitions().font_data.contains_key(name))) {
        return false;
    }
    let ready = std::mem::take(&mut bound.sent);
    bound.active.extend(ready);
    true
}

/// A whole number of `N` bytes at `at` in a font file, most significant byte first.
fn bytes_at<const N: usize>(file: &[u8], at: usize) -> Option<[u8; N]> {
    file.get(at..at.checked_add(N)?)?.try_into().ok()
}

/// Half of the line gap of face `index` in a font file, as a fraction of its size; 0 when the file can't be
/// read that far. The gap is the one egui adds to a line: the typographic one when the font asks for its
/// typographic metrics to be used, else the one of its `hhea` table.
fn half_line_gap(file: &[u8], index: u32) -> f32 {
    let read = || -> Option<f32> {
        let short = |at: usize| bytes_at::<2>(file, at).map(u16::from_be_bytes);
        let signed = |at: usize| bytes_at::<2>(file, at).map(i16::from_be_bytes);
        let long = |at: usize| bytes_at::<4>(file, at).map(u32::from_be_bytes).and_then(|value| usize::try_from(value).ok());
        // A collection starts with a list of where each of its faces begins.
        let start = if file.get(..4)? == b"ttcf" { long(12 + 4 * usize::try_from(index).ok()?)? } else { 0 };
        let table = |tag: &[u8; 4]| -> Option<usize> {
            (0..usize::from(short(start + 4)?))
                .map(|at| start + 12 + 16 * at)
                .find(|entry| file.get(*entry..*entry + 4) == Some(tag))
                .and_then(|entry| long(entry + 8))
        };
        let per_em = f32::from(short(table(b"head")? + 18)?);
        let typographic = table(b"OS/2").filter(|os2| short(os2 + 62).is_some_and(|selection| selection & 0x80 != 0));
        let gap = match typographic {
            Some(os2) => signed(os2 + 72)?,
            None => signed(table(b"hhea")? + 8)?,
        };
        (per_em > 0.0).then(|| (f32::from(gap) / per_em / 2.0).clamp(0.0, 0.5))
    };
    read().unwrap_or(0.0)
}

/// Where to look for the regular, bold and italic faces, first match wins.
// Human: macOS keeps Arial in Supplemental; Windows has it in its fonts folder (a bare file name here is
// looked for there, see `read`); the Linux paths are the common Liberation Sans (metric-compatible with Arial)
// for the Linux port later.
const CANDIDATES: [[&str; 3]; 3] = [
    [
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/System/Library/Fonts/Supplemental/Arial Italic.ttf",
    ],
    ["arial.ttf", "arialbd.ttf", "ariali.ttf"],
    [
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Italic.ttf",
    ],
];

/// Symbol faces tried after the text face, for interface glyphs Arial lacks (⌂ ▭ ◯ ☾ ☰ ▾).
const SYMBOL_FALLBACKS: [&str; 3] = ["/System/Library/Fonts/Apple Symbols.ttf", "seguisym.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"];

/// The bytes of the font file `name`: a whole path, or a bare file name in Windows' fonts folder.
// Human: Windows says where it is installed in `SystemRoot` (usually `C:\Windows`); a Mac has no such variable,
// so the bare names are simply not found there.
fn read(name: &str) -> Option<Vec<u8>> {
    if name.contains('/') {
        return std::fs::read(name).ok();
    }
    let windows = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("windir"))?;
    std::fs::read(std::path::Path::new(&windows).join("Fonts").join(name)).ok()
}

/// Installs the fonts into `ctx`. Logs once and keeps egui's face when no system font is found.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut fallback: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    for (n, path) in SYMBOL_FALLBACKS.iter().enumerate() {
        if let Some(bytes) = read(path) {
            let name = format!("symbols-{n}");
            fonts.font_data.insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
            fallback.push(name);
        }
    }
    let found = CANDIDATES.iter().find_map(|[regular, bold, italic]| Some((read(regular)?, read(bold), read(italic))));
    let family = |name: &str, bytes: Option<Vec<u8>>, fonts: &mut FontDefinitions| {
        let mut list = Vec::new();
        if let Some(bytes) = bytes {
            fonts.font_data.insert(name.to_owned(), Arc::new(FontData::from_owned(bytes)));
            list.push(name.to_owned());
        }
        list.extend(fallback.iter().cloned());
        list
    };
    if let Some((regular, bold, italic)) = found {
        let proportional = family("system-regular", Some(regular), &mut fonts);
        let bold = family("system-bold", bold, &mut fonts);
        let italic = family("system-italic", italic, &mut fonts);
        fonts.families.insert(FontFamily::Proportional, proportional);
        fonts.families.insert(FontFamily::Name(BOLD_FAMILY.into()), bold);
        fonts.families.insert(FontFamily::Name(ITALIC_FAMILY.into()), italic);
    } else {
        log::warn!("no system sans-serif font found; using egui's bundled face (no bold or italic)");
        fonts.families.insert(FontFamily::Name(BOLD_FAMILY.into()), fallback.clone());
        fonts.families.insert(FontFamily::Name(ITALIC_FAMILY.into()), fallback);
    }
    let fallback = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    *lock(&BOUND) = Some(Bound {
        definitions: fonts.clone(),
        fallback,
        active: HashSet::new(),
        sent: Vec::new(),
        wanted: Vec::new(),
        broken: HashSet::new(),
        files: HashMap::new(),
    });
    ctx.set_fonts(fonts);
    // Human: the interface text is small (12 px); egui's defaults are larger.
    ctx.all_styles_mut(|style| {
        use egui::{FontId, TextStyle};
        style.text_styles.insert(TextStyle::Body, FontId::proportional(12.5));
        style.text_styles.insert(TextStyle::Button, FontId::proportional(12.5));
        style.text_styles.insert(TextStyle::Small, FontId::proportional(10.5));
        style.text_styles.insert(TextStyle::Heading, FontId::proportional(16.0));
        style.text_styles.insert(TextStyle::Monospace, FontId::monospace(12.0));
        style.spacing.item_spacing = egui::vec2(6.0, 5.0);
        style.spacing.button_padding = egui::vec2(6.0, 3.0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A font file with only the tables the line gap is read from: 1000 units per em, an `hhea` gap of 200
    /// and a typographic gap of 100, which counts when `typographic` is set.
    fn font(typographic: bool) -> Vec<u8> {
        let mut file = vec![0, 1, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0];
        let tables: [(&[u8; 4], Vec<u8>); 3] = [
            (b"head", {
                let mut head = vec![0; 54];
                head[18..20].copy_from_slice(&1000_u16.to_be_bytes());
                head
            }),
            (b"hhea", {
                let mut hhea = vec![0; 36];
                hhea[8..10].copy_from_slice(&200_i16.to_be_bytes());
                hhea
            }),
            (b"OS/2", {
                let mut os2 = vec![0; 96];
                os2[62..64].copy_from_slice(&(if typographic { 0x80_u16 } else { 0 }).to_be_bytes());
                os2[72..74].copy_from_slice(&100_i16.to_be_bytes());
                os2
            }),
        ];
        let mut at = 12 + 16 * tables.len();
        for (tag, table) in &tables {
            file.extend_from_slice(*tag);
            file.extend_from_slice(&[0; 4]);
            file.extend_from_slice(&u32::try_from(at).unwrap().to_be_bytes());
            file.extend_from_slice(&u32::try_from(table.len()).unwrap().to_be_bytes());
            at += table.len();
        }
        for (_, table) in tables {
            file.extend(table);
        }
        file
    }

    #[test]
    fn half_of_a_fonts_line_gap_goes_above_its_letters() {
        assert!((half_line_gap(&font(false), 0) - 0.1).abs() < 1e-6);
        assert!((half_line_gap(&font(true), 0) - 0.05).abs() < 1e-6);
        // A file that ends early, or is no font at all, moves nothing.
        assert!(half_line_gap(&font(false)[..40], 0).abs() < f32::EPSILON);
        assert!(half_line_gap(b"not a font", 0).abs() < f32::EPSILON);
    }
}
