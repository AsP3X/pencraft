//! Pictures for image fills: read from disk on threads of their own, and kept as textures while they are in
//! use and there is room for them.
//!
//! A fill's `url` is a file path, relative to the folder of the `.pen` file. Nothing is fetched from the network;
//! a web address, a missing file or an unreadable picture leaves the fill on its grey placeholder.
//! Pictures an image service made are files like any other by the time they get here ([`crate::imagery`]).
//!
// Human: a photo takes a tenth of a second to read, and a design can show dozens: read where they are asked
// for, they froze the window for seconds when a design was opened. The canvas now asks without waiting and
// draws a plain colour until a picture is there. An export, a picture for a model and the `--screenshot` run
// wait instead: what they make must show the pictures themselves.
// Human: a picture is a texture of up to 4096 pixels a side (64 MB) and a small copy for effects. They used
// to be kept until the app ended, every one any document had ever shown. Now they are kept up to
// [`BUDGET`]; past it, the ones used longest ago go, never one that is on the canvas now. A picture that
// went is read again when it is next asked for.
// Agent: one thread at a time (RefCell): the display list keeps the store behind its lock. Decoding SPAWNS
// up to [`WORKERS`] threads named `pencraft-picture`, which live as long as the store and READ the files;
// each tells the context to draw again when a picture is ready. Textures are made by whoever takes the
// pictures in ([`ImageStore::get`], [`ImageStore::take_in`]), through the context.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use egui::{ColorImage, TextureHandle, TextureId, TextureOptions};
use pen_doc::value::narrow;

/// Pictures wider or taller than this are scaled down before they become a texture, to bound memory.
const MAX_SIDE: u32 = 4096;
/// The longest side of the copy of a picture kept for effects (blurs and shadows are worked out on the CPU).
const EFFECT_SIDE: u32 = 1024;
/// The most bytes of pictures kept, textures and copies for effects together, before the ones used longest
/// ago are let go (mechanics.json: "picture memory budget"). Pictures on the canvas stay whatever they come to.
pub const BUDGET: usize = 512 * 1024 * 1024;
/// The most threads that read picture files at a time.
const WORKERS: usize = 4;

/// A loaded picture.
#[derive(Clone, Copy)]
pub struct Picture {
    /// The texture to draw with.
    pub texture: TextureId,
    /// Width of the file's picture in pixels (before any scaling down).
    pub width: f32,
    /// Height of the file's picture in pixels.
    pub height: f32,
}

/// What asking for a picture gave.
#[derive(Clone, Copy)]
pub enum Found {
    /// The picture.
    Ready(Picture),
    /// It is being read; the context is told to draw again when it is there.
    Coming,
    /// There is none: no path, a web address, a missing or unreadable file.
    Missing,
}

/// What the store knows of one file.
enum Entry {
    /// A thread is reading it.
    Coming,
    /// It could not be read; remembered, so it is tried (and reported) once.
    Failed,
    /// It is here.
    Ready(Kept),
}

/// A picture that is kept.
struct Kept {
    /// Its texture, which lives as long as this.
    handle: TextureHandle,
    picture: Picture,
    /// A copy of its pixels no larger than [`EFFECT_SIDE`] a side, for effects.
    small: Arc<ColorImage>,
    /// What it takes: the texture and that copy.
    bytes: usize,
    /// When it was last asked for or seen on the canvas ([`Shelf::clock`]).
    used: u64,
}

/// The pictures by file path, and what they come to.
#[derive(Default)]
struct Shelf {
    files: HashMap<PathBuf, Entry>,
    /// The file each texture was made from.
    textures: HashMap<TextureId, PathBuf>,
    /// Counts the times a picture was asked for.
    clock: u64,
    /// What the pictures kept take together.
    bytes: usize,
}

/// A file a thread has read, or why it couldn't.
type Read = (PathBuf, Result<Decoded, String>);

/// The threads that read files: where files to read are handed in, and where what was read arrives.
struct Readers {
    hand_in: mpsc::Sender<(PathBuf, u32)>,
    arrive: mpsc::Receiver<Read>,
}

/// Every picture asked for and still kept, by file path. A file that could not be read is remembered too, so
/// it is tried (and reported) once.
// Agent: a picture changed on disk is NOT reloaded while it is kept.
#[derive(Default)]
pub struct ImageStore {
    pictures: RefCell<Shelf>,
    /// The reading threads, started when a picture is first asked for without waiting.
    readers: RefCell<Option<Readers>>,
    /// How many files the threads have read so far, for whoever draws to see that there are new ones.
    arrivals: Arc<AtomicU64>,
    /// Whether every picture is waited for, whoever asks ([`ImageStore::wait_always`]).
    patient: Cell<bool>,
}

impl ImageStore {
    /// The picture `url` names. `base` is the folder of the document. One that isn't here yet is read at
    /// once when `wait` is set (an export, a picture of the design); otherwise a thread reads it and this
    /// says [`Found::Coming`] until it is here.
    // Agent: BLOCKS on reading and decoding the file when `wait` is set and the picture isn't kept.
    pub fn get(&self, ctx: &egui::Context, base: Option<&Path>, url: &str, wait: bool) -> Found {
        let Some(path) = resolve(base, url) else { return Found::Missing };
        self.take_in(ctx);
        let mut pictures = self.pictures.borrow_mut();
        pictures.clock += 1;
        let now = pictures.clock;
        match pictures.files.get_mut(&path) {
            Some(Entry::Ready(kept)) => {
                kept.used = now;
                return Found::Ready(kept.picture);
            }
            Some(Entry::Failed) => return Found::Missing,
            Some(Entry::Coming) if !wait && !self.patient.get() => return Found::Coming,
            // Asked for by someone who waits while a thread is at it: read here; what the thread brings is
            // then left aside.
            Some(Entry::Coming) | None => {}
        }
        let max_side = max_side(ctx);
        if wait || self.patient.get() {
            let read = decode(&path, max_side).map_err(|error| error.to_string());
            return pictures.put(ctx, path, read);
        }
        if self.hand_in(ctx, path.clone(), max_side) {
            pictures.files.insert(path, Entry::Coming);
            return Found::Coming;
        }
        // No thread to read it: read here, as an export does.
        let read = decode(&path, max_side).map_err(|error| error.to_string());
        pictures.put(ctx, path, read)
    }

    /// Makes the pictures the threads have read since the last time ready to draw. Says whether any came.
    pub fn take_in(&self, ctx: &egui::Context) -> bool {
        let readers = self.readers.borrow();
        let Some(readers) = readers.as_ref() else { return false };
        let mut pictures = self.pictures.borrow_mut();
        let mut any = false;
        for (path, read) in readers.arrive.try_iter() {
            // Only what is still waited for: a picture read meanwhile by someone who waited is kept as it is.
            if matches!(pictures.files.get(&path), Some(Entry::Coming)) {
                pictures.put(ctx, path, read);
                any = true;
            }
        }
        any
    }

    /// The pixels of a loaded picture (at most [`EFFECT_SIDE`] a side), for effects that blur it.
    pub fn pixels(&self, texture: TextureId) -> Option<Arc<ColorImage>> {
        let pictures = self.pictures.borrow();
        match pictures.files.get(pictures.textures.get(&texture)?)? {
            Entry::Ready(kept) => Some(kept.small.clone()),
            Entry::Coming | Entry::Failed => None,
        }
    }

    /// From now on every picture is waited for, whoever asks: for a run that makes one picture of the
    /// window and ends (`--screenshot`), which must show the pictures themselves.
    pub fn wait_always(&self) {
        self.patient.set(true);
    }

    /// The count of files the threads have read so far; it goes up whenever a picture is ready to be taken in.
    pub fn arrivals(&self) -> Arc<AtomicU64> {
        self.arrivals.clone()
    }

    /// How many pictures threads are reading at the moment.
    #[cfg(test)]
    pub fn coming(&self) -> usize {
        self.pictures.borrow().files.values().filter(|entry| matches!(entry, Entry::Coming)).count()
    }

    /// How many pictures are kept, and the bytes they take: textures and copies for effects.
    pub fn held(&self) -> (usize, usize) {
        let pictures = self.pictures.borrow();
        (pictures.textures.len(), pictures.bytes)
    }

    /// Lets go of pictures while they take more than `budget` together: the ones used longest ago first,
    /// and never one of `shown`, the textures on the canvas now, which count as used this moment.
    // Agent: FREES textures; nothing drawn from now on may use a texture that is not in `shown`.
    pub fn trim(&self, shown: &HashSet<TextureId>, budget: usize) {
        let mut pictures = self.pictures.borrow_mut();
        pictures.clock += 1;
        let now = pictures.clock;
        let mut idle: Vec<(u64, PathBuf)> = Vec::new();
        for (path, entry) in &mut pictures.files {
            let Entry::Ready(kept) = entry else { continue };
            if shown.contains(&kept.handle.id()) {
                kept.used = now;
            } else {
                idle.push((kept.used, path.clone()));
            }
        }
        if pictures.bytes <= budget {
            return;
        }
        idle.sort();
        for (_, path) in idle {
            if pictures.bytes <= budget {
                break;
            }
            if let Some(Entry::Ready(kept)) = pictures.files.remove(&path) {
                pictures.textures.remove(&kept.handle.id());
                pictures.bytes = pictures.bytes.saturating_sub(kept.bytes);
            }
        }
    }

    /// Hands `path` to the reading threads, which are started for the first one. `false` when there are none.
    fn hand_in(&self, ctx: &egui::Context, path: PathBuf, max_side: u32) -> bool {
        let mut readers = self.readers.borrow_mut();
        if readers.is_none() {
            *readers = start_readers(ctx, &self.arrivals);
        }
        readers.as_ref().is_some_and(|readers| readers.hand_in.send((path, max_side)).is_ok())
    }
}

impl Shelf {
    /// Keeps what was read of `path`, as a texture, or remembers that it couldn't be read.
    fn put(&mut self, ctx: &egui::Context, path: PathBuf, read: Result<Decoded, String>) -> Found {
        // What was kept of the file before goes (nothing, but for a file read twice).
        if let Some(Entry::Ready(old)) = self.files.remove(&path) {
            self.textures.remove(&old.handle.id());
            self.bytes = self.bytes.saturating_sub(old.bytes);
        }
        match read {
            Ok(Decoded { pixels, small, width, height }) => {
                // Human: a picture small enough to be its own copy for effects is one set of pixels, shared
                // with the texture's while that is on its way to the graphics card.
                // What it takes: the texture on the graphics card, and the copy that stays here.
                let bytes = bytes_of(&pixels) + bytes_of(&small);
                let handle = ctx.load_texture(path.display().to_string(), egui::ImageData::Color(pixels), TextureOptions::LINEAR);
                let picture = Picture { texture: handle.id(), width, height };
                self.textures.insert(handle.id(), path.clone());
                self.bytes += bytes;
                self.files.insert(path, Entry::Ready(Kept { handle, picture, small, bytes, used: self.clock }));
                Found::Ready(picture)
            }
            Err(error) => {
                log::warn!("image fill: {}: {error}", path.display());
                self.files.insert(path, Entry::Failed);
                Found::Missing
            }
        }
    }
}

/// The bytes a picture's pixels take.
fn bytes_of(pixels: &ColorImage) -> usize {
    pixels.pixels.len() * std::mem::size_of::<egui::Color32>()
}

/// The longest side a texture may have here.
fn max_side(ctx: &egui::Context) -> u32 {
    u32::try_from(ctx.input(|i| i.max_texture_side)).unwrap_or(MAX_SIDE).min(MAX_SIDE)
}

/// Starts the threads that read picture files; `None` when not one could be started.
// Human: they take files from one line, so a design's photos are read several at a time, in the order they
// were asked for. They end when the store goes: nothing is handed in any more.
fn start_readers(ctx: &egui::Context, arrivals: &Arc<AtomicU64>) -> Option<Readers> {
    let (hand_in, line) = mpsc::channel::<(PathBuf, u32)>();
    let (done, arrive) = mpsc::channel::<Read>();
    let line = Arc::new(Mutex::new(line));
    let count = std::thread::available_parallelism().map_or(1, |cores| (cores.get() / 2).clamp(1, WORKERS));
    let mut started = 0;
    for _ in 0..count {
        let (line, done, wake, arrivals) = (line.clone(), done.clone(), ctx.clone(), arrivals.clone());
        let work = move || {
            loop {
                // The line is held only while a file is taken from it, not while it is read.
                let next = crate::app::lock(&line).recv();
                let Ok((path, max_side)) = next else { return };
                let read = decode(&path, max_side).map_err(|error| error.to_string());
                if done.send((path, read)).is_err() {
                    return;
                }
                arrivals.fetch_add(1, Ordering::Relaxed);
                wake.request_repaint();
            }
        };
        match std::thread::Builder::new().name("pencraft-picture".to_owned()).spawn(work) {
            Ok(_) => started += 1,
            Err(error) => log::warn!("couldn't start a thread to read pictures: {error}"),
        }
    }
    (started > 0).then_some(Readers { hand_in, arrive })
}

/// A picture file read into egui's pixel format.
struct Decoded {
    /// The picture, for its texture.
    pixels: Arc<ColorImage>,
    /// A copy no larger than [`EFFECT_SIDE`] a side; the picture itself when it is no larger.
    small: Arc<ColorImage>,
    /// Width of the file's picture in pixels.
    width: f32,
    /// Height of the file's picture in pixels.
    height: f32,
}

/// The file a fill's `url` names, or `None` for web addresses, embedded data and relative paths in a document
/// that has no file yet.
#[must_use]
pub fn resolve(base: Option<&Path>, url: &str) -> Option<PathBuf> {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    if url.is_empty() || ["http:", "https:", "data:", "blob:"].iter().any(|scheme| lower.starts_with(scheme)) {
        return None;
    }
    let path = local(url);
    if network_path(url) || network_path(path) {
        return None;
    }
    let path = Path::new(path);
    // Human: on Windows a path can start at the top of a drive without naming the drive (`/pictures/a.png`,
    // as a file made on a Mac has it): that is the drive the document is on. Elsewhere such a path is whole.
    if path.is_absolute() || (path.has_root() && base.is_none()) { Some(path.to_path_buf()) } else { base.map(|folder| folder.join(path)) }
}

/// Whether `text` names a file on another computer: a Windows network path (`\\server\share`,
/// `\\?\UNC\…`, `//server/…`) or a device path (`\\.\…`).
// Human: opening such a path makes Windows sign in to that computer with the user's credentials, so a path a
// document or a model names must not be one. A folder the user opened a document from may still be one.
// Agent: two separators at the start are refused on every system; the prefix kinds are only parsed on Windows.
#[must_use]
pub fn network_path(text: &str) -> bool {
    let text = text.trim();
    let separator = |byte: Option<&u8>| matches!(byte, Some(b'\\' | b'/'));
    if separator(text.as_bytes().first()) && separator(text.as_bytes().get(1)) {
        return true;
    }
    matches!(
        Path::new(text).components().next(),
        Some(std::path::Component::Prefix(prefix))
            if matches!(prefix.kind(), std::path::Prefix::UNC(..) | std::path::Prefix::VerbatimUNC(..) | std::path::Prefix::DeviceNS(..))
    )
}

/// The path in `url`, which is a path already or a `file://` address.
// Human: a `file://` address has a slash before a Windows drive (`file:///C:/pictures/a.png`) that isn't
// part of the path there.
fn local(url: &str) -> &str {
    let path = url.strip_prefix("file://").unwrap_or(url);
    let drive = |rest: &str| rest.as_bytes().first().is_some_and(u8::is_ascii_alphabetic) && rest.as_bytes().get(1) == Some(&b':');
    match path.strip_prefix('/') {
        Some(rest) if cfg!(windows) && path.len() < url.len() && drive(rest) => rest,
        _ => path,
    }
}

/// Reads a picture file into egui's pixel format, with the file's own pixel size.
fn decode(path: &Path, max_side: u32) -> Result<Decoded, Box<dyn std::error::Error>> {
    let picture = image::ImageReader::open(path)?.with_guessed_format()?.decode()?;
    let (width, height) = (picture.width(), picture.height());
    if width == 0 || height == 0 {
        return Err("the picture is empty".into());
    }
    let picture = if width > max_side || height > max_side { picture.thumbnail(max_side, max_side) } else { picture };
    let pixels = Arc::new(color_image(&picture)?);
    let small = if picture.width() > EFFECT_SIDE || picture.height() > EFFECT_SIDE {
        Arc::new(color_image(&picture.thumbnail(EFFECT_SIDE, EFFECT_SIDE))?)
    } else {
        pixels.clone()
    };
    Ok(Decoded { pixels, small, width: narrow(f64::from(width)), height: narrow(f64::from(height)) })
}

/// A decoded picture in egui's pixel format.
fn color_image(picture: &image::DynamicImage) -> Result<ColorImage, std::num::TryFromIntError> {
    let rgba = picture.to_rgba8();
    let size = [usize::try_from(rgba.width())?, usize::try_from(rgba.height())?];
    Ok(ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_resolve_against_the_document_folder_and_never_leave_the_disk() {
        let base = Path::new("/designs/site");
        assert_eq!(resolve(Some(base), "./hero.jpg"), Some(PathBuf::from("/designs/site/./hero.jpg")));
        assert_eq!(resolve(Some(base), "/pictures/a.png"), Some(PathBuf::from("/pictures/a.png")));
        assert_eq!(resolve(None, "file:///pictures/a.png"), Some(PathBuf::from("/pictures/a.png")));
        assert_eq!(resolve(None, "hero.jpg"), None);
        if cfg!(windows) {
            assert_eq!(resolve(None, "file:///C:/pictures/a.png"), Some(PathBuf::from("C:/pictures/a.png")));
            assert_eq!(resolve(Some(Path::new("D:/designs")), "/pictures/a.png"), Some(PathBuf::from("D:/pictures/a.png")));
            assert_eq!(resolve(Some(Path::new("D:/designs")), r"C:\pictures\a.png"), Some(PathBuf::from("C:/pictures/a.png")));
        }
        assert_eq!(resolve(Some(base), "https://example.com/a.png"), None);
        assert_eq!(resolve(Some(base), "data:image/png;base64,AAAA"), None);
        assert_eq!(resolve(Some(base), "  "), None);
    }

    #[test]
    fn network_paths_are_never_resolved() {
        let base = Path::new("/designs/site");
        for url in [r"\\server\share\a.png", "//server/share/a.png", r"\/server/a.png", "file:////server/share/a.png", r"  \\server\a.png"] {
            assert!(network_path(url) || network_path(local(url.trim())), "{url}");
            assert_eq!(resolve(Some(base), url), None, "{url}");
            assert_eq!(resolve(None, url), None, "{url}");
        }
        assert!(!network_path("/pictures/a.png"));
        assert!(!network_path("./a.png"));
        assert!(!network_path("a/b.png"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_network_and_device_prefixes_are_refused() {
        for path in [r"\\?\UNC\server\share\a.png", r"\\server\share\a.png", r"\\.\pipe\x", r"\\?\C:\a.png"] {
            assert!(network_path(path), "{path}");
        }
        assert!(!network_path(r"C:\pictures\a.png"));
        assert_eq!(resolve(Some(Path::new("D:/designs")), r"\\?\UNC\server\share\a.png"), None);
    }

    /// A folder of its own with `count` pictures of `side` pixels square, each one colour.
    fn pictures(name: &str, count: u8, side: u32) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("quill-images-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        for at in 0..count {
            image::RgbaImage::from_pixel(side, side, image::Rgba([at * 40, 10, 200, 255])).save(folder.join(format!("{at}.png"))).unwrap();
        }
        folder
    }

    /// Asks for `url` until it is no longer on its way; what it then is.
    fn arrived(store: &ImageStore, ctx: &egui::Context, folder: &Path, url: &str) -> Found {
        for _ in 0..2000 {
            match store.get(ctx, Some(folder), url, false) {
                Found::Coming => std::thread::sleep(std::time::Duration::from_millis(2)),
                found => return found,
            }
        }
        Found::Coming
    }

    #[test]
    fn the_canvas_is_told_a_picture_is_coming_and_an_export_waits_for_it() {
        let (ctx, folder, store) = (egui::Context::default(), pictures("coming", 3, 8), ImageStore::default());
        // Asked for without waiting: a thread reads it, and it is there a moment later.
        assert!(matches!(store.get(&ctx, Some(&folder), "0.png", false), Found::Coming));
        assert!(matches!(store.get(&ctx, Some(&folder), "0.png", false), Found::Coming | Found::Ready(_)));
        let Found::Ready(first) = arrived(&store, &ctx, &folder, "0.png") else { panic!("the picture never came") };
        assert_eq!((first.width, first.height), (8.0, 8.0));
        assert!(store.arrivals().load(Ordering::Relaxed) >= 1 && store.coming() == 0);
        assert_eq!(store.pixels(first.texture).unwrap().size, [8, 8]);
        // Asked for by someone who waits: there at once, also while a thread is still at it.
        assert!(matches!(store.get(&ctx, Some(&folder), "1.png", true), Found::Ready(_)));
        let on_its_way = store.get(&ctx, Some(&folder), "2.png", false);
        let Found::Ready(waited) = store.get(&ctx, Some(&folder), "2.png", true) else { panic!("not waited for") };
        assert!(matches!(on_its_way, Found::Coming | Found::Ready(_)));
        // What the thread brings afterwards changes nothing: the picture keeps its texture.
        std::thread::sleep(std::time::Duration::from_millis(50));
        store.take_in(&ctx);
        assert!(matches!(store.get(&ctx, Some(&folder), "2.png", false), Found::Ready(again) if again.texture == waited.texture));
        assert_eq!(store.held().0, 3);
        // A file that can't be read, and no file at all, are missing whoever asks; it is tried once.
        std::fs::write(folder.join("broken.png"), b"not a picture").unwrap();
        assert!(matches!(arrived(&store, &ctx, &folder, "broken.png"), Found::Missing));
        assert!(matches!(store.get(&ctx, Some(&folder), "broken.png", true), Found::Missing));
        assert!(matches!(store.get(&ctx, Some(&folder), "https://example.com/a.png", false), Found::Missing));
        // A store that always waits never says a picture is coming.
        let patient = ImageStore::default();
        patient.wait_always();
        assert!(matches!(patient.get(&ctx, Some(&folder), "0.png", false), Found::Ready(_)));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn pictures_past_the_budget_go_longest_unused_first_and_never_one_on_the_canvas() {
        let (ctx, folder, store) = (egui::Context::default(), pictures("budget", 4, 16), ImageStore::default());
        let texture = |at: u8| match store.get(&ctx, Some(&folder), &format!("{at}.png"), true) {
            Found::Ready(picture) => picture.texture,
            Found::Coming | Found::Missing => panic!("picture {at} is not there"),
        };
        let textures: Vec<TextureId> = (0..4).map(texture).collect();
        // A picture this small is its own copy for effects: its pixels are counted for the texture and for
        // that copy, 16 x 16 x 4 bytes each.
        let each = 2 * 16 * 16 * 4;
        assert_eq!(store.held(), (4, 4 * each));
        // Within the budget nothing goes.
        store.trim(&HashSet::new(), 4 * each);
        assert_eq!(store.held().0, 4);
        // Room for two: the first is on the canvas and stays; of the others the two asked for longest ago go.
        let _again = texture(1);
        store.trim(&HashSet::from([textures[0]]), 2 * each);
        assert_eq!(store.held(), (2, 2 * each));
        assert!(store.pixels(textures[0]).is_some() && store.pixels(textures[1]).is_some());
        assert!(store.pixels(textures[2]).is_none() && store.pixels(textures[3]).is_none());
        // Everything on the canvas stays, whatever the budget.
        store.trim(&HashSet::from([textures[0], textures[1]]), 0);
        assert_eq!(store.held().0, 2);
        // A picture that went is read again when it is asked for.
        assert_ne!(texture(3), textures[3]);
        assert_eq!(store.held().0, 3);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_small_picture_is_its_own_copy_for_effects_and_a_large_one_has_a_smaller_one() {
        let folder = pictures("copies", 1, 8);
        let Decoded { pixels, small, .. } = decode(&folder.join("0.png"), MAX_SIDE).unwrap();
        assert!(Arc::ptr_eq(&pixels, &small));
        image::RgbaImage::from_pixel(EFFECT_SIDE + 200, 40, image::Rgba([1, 2, 3, 255])).save(folder.join("wide.png")).unwrap();
        let Decoded { pixels, small, width, .. } = decode(&folder.join("wide.png"), MAX_SIDE).unwrap();
        assert!(!Arc::ptr_eq(&pixels, &small));
        assert_eq!((pixels.size[0], small.size[0], width), (1224, 1024, 1224.0));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_picture_file_decodes_with_its_size_and_a_broken_one_is_refused() {
        let folder = std::env::temp_dir().join(format!("pencraft-images-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let good = folder.join("two-by-one.png");
        image::RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 128]).unwrap().save(&good).unwrap();
        let Decoded { pixels, small, width, height } = decode(&good, MAX_SIDE).unwrap();
        assert_eq!((pixels.size, small.size, width, height), ([2, 1], [2, 1], 2.0, 1.0));
        let bad = folder.join("broken.png");
        std::fs::write(&bad, b"not a picture").unwrap();
        assert!(decode(&bad, MAX_SIDE).is_err());
        assert!(decode(&folder.join("missing.png"), MAX_SIDE).is_err());
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
