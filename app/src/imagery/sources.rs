//! Getting the pictures an edit sends ready: read, scaled down, and encoded as the service takes them.

use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageError, ImageFormat, ImageReader, RgbImage, RgbaImage, imageops::FilterType};

use super::service::Source;

/// A picture to get ready for sending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Take {
    /// Where its bytes come from.
    pub from: Origin,
    /// This is the picture to change.
    pub change: bool,
    /// What the credit calls it: the file's name, or [`DRAWN`].
    pub name: String,
    /// The layer whose image fill the file is, when it is one: what is wrong with the file is said of that layer.
    pub layer: Option<String>,
}

impl Take {
    /// The picture file at `path`, the image fill of `layer` when it is one; named after the file.
    #[must_use]
    pub fn file(path: PathBuf, change: bool, layer: Option<String>) -> Take {
        let name = path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned());
        Take { from: Origin::File(path), change, name, layer }
    }

    /// Layers drawn as they look: `width` × `height` pixels of premultiplied RGBA, as the renderer draws them.
    #[must_use]
    pub fn pixels(width: u32, height: u32, rgba: Vec<u8>, change: bool) -> Take {
        Take { from: Origin::Pixels { width, height, rgba }, change, name: DRAWN.to_owned(), layer: None }
    }
}

/// Where a picture to send comes from.
#[derive(Clone, PartialEq, Eq)]
pub enum Origin {
    /// A picture file, read on the worker thread.
    File(PathBuf),
    /// Layers drawn as they look, on the main thread before the job starts; encoded on the worker thread.
    Pixels {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
        /// Four bytes a pixel, colour already multiplied by alpha (as `export::Picture` holds them).
        rgba: Vec<u8>,
    },
}

// Human: a drawn picture is shown by its size, so what the user drew never ends up in a log.
impl std::fmt::Debug for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Origin::File(path) => f.debug_tuple("File").field(path).finish(),
            Origin::Pixels { width, height, rgba } => write!(f, "Pixels({width} × {height}, {} bytes)", rgba.len()),
        }
    }
}

/// What the credit calls layers that were drawn and sent.
pub const DRAWN: &str = "drawn";
/// The longest side of a picture that is sent, in pixels.
pub const MOST_SIDE: u32 = 2048;
/// The most reference pictures in one request.
pub const MOST_REFERENCES: usize = 4;
/// How good a JPEG that is sent is, out of 100.
const JPEG_QUALITY: u8 = 90;
/// One megabyte, as the sentences count them.
const MEGABYTE: u64 = 1024 * 1024;
/// The largest picture file that is read at all, in bytes.
const MOST_FILE: u64 = 100 * MEGABYTE;
/// The longest side of a picture file that is read, in pixels.
const MOST_READ_SIDE: u32 = 16384;
/// The most memory reading one picture file may take, in bytes.
const MOST_DECODED: u64 = 256 * MEGABYTE;
/// The most the pictures of one request may come to together, once ready, in bytes.
const MOST_SENT: u64 = 15 * MEGABYTE;
/// The share of a picture's pixels, in percent, that may be less than opaque before it is sent with its
/// transparency (as PNG) rather than flattened onto white (as JPEG).
const CLEAR_PERCENT: u64 = 1;

/// How much is read and sent: the shipped limits, or smaller ones in tests.
struct Caps {
    /// [`MOST_FILE`].
    file: u64,
    /// [`MOST_READ_SIDE`].
    side: u32,
    /// [`MOST_SENT`].
    sent: u64,
}

/// The limits Pencraft reads and sends pictures with.
const CAPS: Caps = Caps { file: MOST_FILE, side: MOST_READ_SIDE, sent: MOST_SENT };

/// `takes` in the order they are sent: the picture to change first, the others as given.
#[must_use]
pub fn in_order(takes: &[Take]) -> Vec<&Take> {
    let mut ordered: Vec<&Take> = takes.iter().collect();
    // A stable sort, so the references keep the order they were named in.
    ordered.sort_by_key(|take| !take.change);
    ordered
}

/// Gets `takes` ready: reads files (or takes the pixels of layers drawn already), scales down to `MOST_SIDE`,
/// and encodes each as JPEG on white, or as PNG when its transparency matters. The picture to change comes
/// first.
///
/// # Errors
/// A sentence naming the picture that is missing, can't be read, isn't one or is too large; or saying the
/// pictures together are more than one request carries. Nothing of a file that isn't a picture is in it.
// Agent: READS the files named; BLOCKS on decoding, scaling and encoding: worker thread only.
pub fn prepare(takes: &[Take]) -> Result<Vec<Source>, String> {
    prepare_within(takes, &CAPS)
}

/// [`prepare`] within `caps`.
fn prepare_within(takes: &[Take], caps: &Caps) -> Result<Vec<Source>, String> {
    let ready = in_order(takes).into_iter().map(|take| one(take, caps)).collect::<Result<Vec<_>, String>>()?;
    let total: u64 = ready.iter().map(|source| source.bytes.len() as u64).sum();
    if total > caps.sent {
        return Err(format!(
            "The pictures come to {} MB, more than the {} MB one request can carry, so nothing was sent. Send fewer or smaller pictures.",
            megabytes(total),
            megabytes(caps.sent)
        ));
    }
    Ok(ready)
}

/// `bytes` in megabytes, to a tenth, rounded up: "15", "15.3".
fn megabytes(bytes: u64) -> String {
    let tenths = (bytes * 10).div_ceil(MEGABYTE);
    if tenths.is_multiple_of(10) { format!("{}", tenths / 10) } else { format!("{}.{}", tenths / 10, tenths % 10) }
}

/// What a sentence calls the picture `take`: its name, and the layer it is the fill of.
fn called(take: &Take) -> String {
    match &take.layer {
        Some(layer) => format!("\"{}\" (the picture of the layer \"{layer}\")", take.name),
        None => format!("\"{}\"", take.name),
    }
}

/// The sentence for a file that isn't a picture that can be sent.
fn not_a_picture(take: &Take) -> String {
    format!("{} isn't a picture that can be sent: it must be a PNG, JPEG, WebP or GIF file.", called(take))
}

/// One picture got ready.
fn one(take: &Take, caps: &Caps) -> Result<Source, String> {
    let picture = match &take.from {
        Origin::File(path) => read(path, take, caps)?,
        Origin::Pixels { width, height, rgba } => unmultiplied(*width, *height, rgba)
            .ok_or_else(|| format!("{} couldn't be made ready to send: its pixels don't fill its size.", called(take)))?,
    };
    let picture = scaled(picture, MOST_SIDE);
    let not_ready = |error: &dyn std::fmt::Display| format!("{} couldn't be made ready to send: {error}.", called(take));
    let mut out = std::io::Cursor::new(Vec::new());
    // Human: JPEG is far smaller for a photograph; PNG keeps transparency, which JPEG can't hold. Layers are
    // drawn on a clear background, so a few clear pixels at the edges don't make a picture go as PNG.
    let kind = if clear_matters(&picture) {
        DynamicImage::ImageRgba8(picture.to_rgba8()).write_to(&mut out, ImageFormat::Png).map_err(|error| not_ready(&error))?;
        "image/png"
    } else {
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
        DynamicImage::ImageRgb8(on_white(&picture)).write_with_encoder(encoder).map_err(|error| not_ready(&error))?;
        "image/jpeg"
    };
    Ok(Source { bytes: out.into_inner(), kind, change: take.change })
}

/// Why a picture file couldn't be opened ([`open`]).
#[derive(Debug)]
pub enum Unopened {
    /// There is nothing at the path.
    Missing,
    /// It isn't a PNG, JPEG, WebP or GIF file that can be decoded (or isn't a file at all).
    NotAPicture,
    /// It couldn't be read: the system's words.
    Unreadable(String),
    /// The file is larger than the size cap: its size in bytes.
    LargeFile(u64),
    /// Decoding it would go past the decoder's limits: its size in pixels, when its header says it.
    LargePicture(Option<(u32, u32)>),
}

/// Opens the picture file at `path` safely: refused before reading when it is larger than [`MOST_FILE`] on
/// disk, refused when its first bytes don't say PNG, JPEG, WebP or GIF (whatever its name says), and decoded
/// within [`MOST_READ_SIDE`] pixels a side and [`MOST_DECODED`] bytes of memory.
///
/// # Errors
/// Why it couldn't be opened; nothing of what the file holds is in it.
// Human: the one way Pencraft opens a picture file it is given (to send it, or to show it small in the dialog),
// so the checks can't drift apart.
// Agent: READS `path`; BLOCKS on decoding: worker thread only.
pub fn open(path: &Path) -> Result<DynamicImage, Unopened> {
    open_within(path, &CAPS)
}

/// [`open`] within `caps`.
// Human: nothing is read whole before it is known to be a picture of a size worth reading: the size on disk
// first, then the first bytes, then the decoder within limits.
fn open_within(path: &Path, caps: &Caps) -> Result<DynamicImage, Unopened> {
    let size = match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() => meta.len(),
        Ok(_) => return Err(Unopened::NotAPicture),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(Unopened::Missing),
        Err(error) => return Err(Unopened::Unreadable(error.to_string())),
    };
    if size > caps.file {
        return Err(Unopened::LargeFile(size));
    }
    let mut reader = opened(path).map_err(|error| Unopened::Unreadable(error.to_string()))?;
    if !matches!(reader.format(), Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP | ImageFormat::Gif)) {
        return Err(Unopened::NotAPicture);
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(caps.side);
    limits.max_image_height = Some(caps.side);
    limits.max_alloc = Some(MOST_DECODED);
    reader.limits(limits);
    match reader.decode() {
        Ok(picture) => Ok(picture),
        Err(ImageError::Limits(_)) => Err(Unopened::LargePicture(opened(path).ok().and_then(|reader| reader.into_dimensions().ok()))),
        // Human: the decoder's own words may quote the file; they are left out.
        Err(_) => Err(Unopened::NotAPicture),
    }
}

/// Reads the picture file at `path` for `take` ([`open_within`]), or says why not, naming it.
// Agent: READS `path`; worker thread only.
fn read(path: &Path, take: &Take, caps: &Caps) -> Result<DynamicImage, String> {
    open_within(path, caps).map_err(|why| match why {
        Unopened::Missing => match &take.layer {
            Some(layer) => format!("The picture of the layer \"{layer}\" isn't there any more: {}.", path.display()),
            None => format!("There is no picture file at {}.", path.display()),
        },
        Unopened::NotAPicture => not_a_picture(take),
        Unopened::Unreadable(error) => format!("{} couldn't be read: {error}.", called(take)),
        Unopened::LargeFile(size) => {
            format!("{} is too large to read ({} MB); Pencraft sends picture files up to {} MB.", called(take), megabytes(size), megabytes(caps.file))
        }
        Unopened::LargePicture(pixels) => too_large(take, pixels, caps.side),
    })
}

/// A reader of the file at `path` whose format is what its first bytes say, never what its name says.
fn opened(path: &Path) -> std::io::Result<ImageReader<std::io::BufReader<std::fs::File>>> {
    let mut reader = ImageReader::open(path)?;
    reader.clear_format();
    reader.with_guessed_format()
}

/// The sentence for a picture too large to read: what its header says of its size, when it says it.
fn too_large(take: &Take, pixels: Option<(u32, u32)>, side: u32) -> String {
    match pixels {
        Some((width, height)) if width.max(height) <= side => {
            format!("{} is too large to read ({width} × {height} pixels): it needs more memory than Pencraft gives one picture.", called(take))
        }
        Some((width, height)) => {
            format!("{} is too large to read ({width} × {height} pixels); Pencraft sends pictures up to {side} pixels a side.", called(take))
        }
        None => format!("{} is too large to read; Pencraft sends pictures up to {side} pixels a side.", called(take)),
    }
}

/// Premultiplied pixels as a picture with straight alpha, as PNG stores it; `None` when they don't fill
/// `width` × `height`.
fn unmultiplied(width: u32, height: u32, rgba: &[u8]) -> Option<DynamicImage> {
    let mut straight = rgba.to_vec();
    for pixel in straight.as_chunks_mut::<4>().0 {
        let a = u16::from(pixel[3]);
        if a > 0 && a < 255 {
            for channel in &mut pixel[..3] {
                *channel = u8::try_from((u16::from(*channel) * 255 + a / 2) / a).unwrap_or(u8::MAX);
            }
        }
    }
    RgbaImage::from_raw(width, height, straight).map(DynamicImage::ImageRgba8)
}

/// `picture` with its longest side at most `most`: shrunk cheaply to about twice that first, then resized.
// Human: `thumbnail` averages whole pixels and needs little memory; the finer filter then works on a picture
// at most four times the size of the result.
fn scaled(picture: DynamicImage, most: u32) -> DynamicImage {
    let side = picture.width().max(picture.height());
    if side <= most {
        return picture;
    }
    let near = most.saturating_mul(2);
    let picture = if side > near { picture.thumbnail(near, near) } else { picture };
    picture.resize(most, most, FilterType::CatmullRom)
}

/// Whether so many of the picture's pixels are less than opaque that its transparency must be kept.
fn clear_matters(picture: &DynamicImage) -> bool {
    if !picture.color().has_alpha() {
        return false;
    }
    let rgba = picture.to_rgba8();
    let clear = rgba.pixels().filter(|pixel| pixel.0[3] < u8::MAX).count() as u64;
    clear * 100 > u64::from(rgba.width()) * u64::from(rgba.height()) * CLEAR_PERCENT
}

/// The picture laid on white, without alpha, as JPEG stores it.
fn on_white(picture: &DynamicImage) -> RgbImage {
    if !picture.color().has_alpha() {
        return picture.to_rgb8();
    }
    let rgba = picture.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0;
        let a = u16::from(a);
        let lay = |channel: u8| u8::try_from((u16::from(channel) * a + 255 * (255 - a) + 127) / 255).unwrap_or(u8::MAX);
        image::Rgb([lay(r), lay(g), lay(b)])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32, alpha: u8) -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(width, height, image::Rgba([200, 30, 30, alpha])).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        bytes.into_inner()
    }

    /// `width` × `height` premultiplied pixels of one colour at `alpha`.
    fn pixels(width: u32, height: u32, alpha: u8) -> Vec<u8> {
        let shade = |channel: u8| u8::try_from(u16::from(channel) * u16::from(alpha) / 255).unwrap();
        [shade(200), shade(30), shade(30), alpha].repeat(usize::try_from(width * height).unwrap())
    }

    /// A folder of its own for the test `name`, empty.
    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("pencraft-sources-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// `bytes` written as the file `name` in `folder`, to be sent.
    fn file(folder: &Path, name: &str, bytes: &[u8], change: bool) -> Take {
        std::fs::write(folder.join(name), bytes).unwrap();
        Take::file(folder.join(name), change, None)
    }

    #[test]
    fn a_large_picture_is_scaled_down_and_a_small_one_is_left_its_size() {
        let folder = folder("sizes");
        let ready =
            prepare(&[file(&folder, "large.png", &png(4096, 1024, 255), true), file(&folder, "small.png", &png(64, 32, 255), false)]).unwrap();
        let sizes: Vec<(u32, u32)> = ready.iter().map(|s| image::load_from_memory(&s.bytes).map(|p| (p.width(), p.height())).unwrap()).collect();
        assert_eq!(sizes, [(2048, 512), (64, 32)]);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_very_large_picture_is_shrunk_cheaply_first_and_keeps_its_shape() {
        let big = DynamicImage::ImageRgba8(RgbaImage::from_pixel(100, 20, image::Rgba([1, 2, 3, 255])));
        let size = |picture: DynamicImage| (picture.width(), picture.height());
        // Over twice the side: thumbnailed to 20 × 4 first, then resized.
        assert_eq!(size(scaled(big.clone(), 10)), (10, 2));
        assert_eq!(size(scaled(big.clone(), 40)), (40, 8));
        assert_eq!(size(scaled(big, 100)), (100, 20));
    }

    #[test]
    fn an_opaque_picture_goes_as_jpeg_and_one_with_transparency_as_png() {
        let folder = folder("kinds");
        let ready = prepare(&[file(&folder, "opaque.png", &png(8, 8, 255), false), file(&folder, "clear.png", &png(8, 8, 120), false)]).unwrap();
        assert_eq!(ready.iter().map(|s| s.kind).collect::<Vec<_>>(), ["image/jpeg", "image/png"]);
        assert_eq!(image::guess_format(&ready[0].bytes).unwrap(), image::ImageFormat::Jpeg);
        assert_eq!(image::guess_format(&ready[1].bytes).unwrap(), image::ImageFormat::Png);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_picture_goes_with_its_transparency_only_when_more_than_one_pixel_in_a_hundred_is_clear() {
        // Human: 100 × 100 pixels, the first `clear` of them fully transparent, the rest opaque.
        let drawn = |clear: usize| {
            let mut rgba = pixels(100, 100, 255);
            rgba[..clear * 4].fill(0);
            Take::pixels(100, 100, rgba, true)
        };
        let ready = prepare(&[drawn(100)]).unwrap();
        assert_eq!(ready[0].kind, "image/jpeg", "exactly one in a hundred: flattened");
        // Human: the clear pixels are laid on white.
        let flat = image::load_from_memory(&ready[0].bytes).unwrap().to_rgb8();
        assert!(flat.get_pixel(0, 0).0.iter().all(|&channel| channel > 240), "{:?}", flat.get_pixel(0, 0));
        assert!(flat.get_pixel(50, 50).0[0] > 180 && flat.get_pixel(50, 50).0[1] < 60, "{:?}", flat.get_pixel(50, 50));
        assert_eq!(prepare(&[drawn(101)]).unwrap()[0].kind, "image/png", "more than one in a hundred: kept");
    }

    #[test]
    fn the_picture_to_change_comes_first_whatever_the_order_given() {
        let folder = folder("order");
        let takes = [file(&folder, "first.png", &png(8, 8, 255), false), file(&folder, "second.png", &png(16, 16, 255), true)];
        let ready = prepare(&takes).unwrap();
        assert_eq!(ready.iter().map(|s| s.change).collect::<Vec<_>>(), [true, false]);
        assert_eq!(image::load_from_memory(&ready[0].bytes).unwrap().width(), 16);
        assert_eq!(in_order(&takes).iter().map(|take| take.name.as_str()).collect::<Vec<_>>(), ["second.png", "first.png"]);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_file_that_is_missing_or_not_a_picture_is_named() {
        let missing = std::env::temp_dir().join("pencraft-no-such-picture.png");
        let why = prepare(&[Take::file(missing.clone(), true, None)]).unwrap_err();
        assert_eq!(why, format!("There is no picture file at {}.", missing.display()));
        // Human: a layer's fill whose file is gone, or isn't a picture, is said of the layer.
        let why = prepare(&[Take::file(missing.clone(), true, Some("hero".to_owned()))]).unwrap_err();
        assert_eq!(why, format!("The picture of the layer \"hero\" isn't there any more: {}.", missing.display()));
        let folder = folder("not-a-picture");
        let why = prepare(&[file(&folder, "page.png", b"<html>secret", true)]).unwrap_err();
        assert!(why.contains("isn't a picture") && why.contains("page.png"), "{why}");
        // Human: nothing of what the file holds is told.
        assert!(!why.contains("html") && !why.contains("secret"), "{why}");
        let layered = Take { layer: Some("hero".to_owned()), ..file(&folder, "broken.png", b"\x89PNG\r\n\x1a\nbroken", true) };
        let why = prepare(&[layered]).unwrap_err();
        assert_eq!(
            why,
            "\"broken.png\" (the picture of the layer \"hero\") isn't a picture that can be sent: it must be a PNG, JPEG, WebP or GIF file."
        );
        // A picture in a format that isn't sent, whatever its name says.
        // Human: the start of a TIFF file, which the first bytes tell apart from the PNG its name promises.
        let why = prepare(&[file(&folder, "photo.png", b"II*\0\x08\0\0\0", true)]).unwrap_err();
        assert!(why.contains("isn't a picture that can be sent"), "{why}");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_picture_too_large_to_read_says_so_with_its_size() {
        let folder = folder("too-large");
        let small = Caps { file: MOST_FILE, side: 32, sent: MOST_SENT };
        let why = prepare_within(&[file(&folder, "a.png", &png(64, 16, 255), true)], &small).unwrap_err();
        assert_eq!(why, "\"a.png\" is too large to read (64 × 16 pixels); Pencraft sends pictures up to 32 pixels a side.");
        assert!(prepare_within(&[file(&folder, "b.png", &png(32, 16, 255), true)], &small).is_ok());
        let tiny = Caps { file: 10, ..small };
        let why = prepare_within(&[file(&folder, "c.png", &png(8, 8, 255), true)], &tiny).unwrap_err();
        assert!(why.starts_with("\"c.png\" is too large to read ("), "{why}");
        assert_eq!(MOST_READ_SIDE, 16384);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_picture_file_is_opened_within_the_same_limits_wherever_it_is_opened() {
        let folder = folder("open");
        std::fs::write(folder.join("a.png"), png(64, 16, 255)).unwrap();
        assert_eq!(open(&folder.join("a.png")).map(|picture| (picture.width(), picture.height())).ok(), Some((64, 16)));
        assert!(matches!(open(&folder.join("missing.png")), Err(Unopened::Missing)));
        assert!(matches!(open(&folder), Err(Unopened::NotAPicture)));
        std::fs::write(folder.join("photo.png"), b"II*\0\x08\0\0\0").unwrap();
        assert!(matches!(open(&folder.join("photo.png")), Err(Unopened::NotAPicture)));
        // Human: the size on disk is asked before anything is read; the decoder keeps to its side.
        let tiny = Caps { file: 10, side: MOST_READ_SIDE, sent: MOST_SENT };
        assert!(matches!(open_within(&folder.join("a.png"), &tiny), Err(Unopened::LargeFile(_))));
        let narrow = Caps { file: MOST_FILE, side: 32, sent: MOST_SENT };
        assert!(matches!(open_within(&folder.join("a.png"), &narrow), Err(Unopened::LargePicture(Some((64, 16))))));
        assert_eq!((MOST_FILE, MOST_DECODED), (100 * MEGABYTE, 256 * MEGABYTE));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn pictures_that_together_are_more_than_a_request_carries_are_not_sent() {
        let takes = [Take::pixels(8, 8, pixels(8, 8, 255), true), Take::pixels(8, 8, pixels(8, 8, 255), false)];
        let one = prepare(&takes[..1]).unwrap()[0].bytes.len() as u64;
        let budget = Caps { file: MOST_FILE, side: MOST_READ_SIDE, sent: one + 1 };
        let why = prepare_within(&takes, &budget).unwrap_err();
        assert!(why.starts_with("The pictures come to 0.1 MB, more than the 0.1 MB one request can carry, so nothing was sent."), "{why}");
        assert!(why.ends_with("Send fewer or smaller pictures."), "{why}");
        assert!(prepare_within(&takes[..1], &budget).is_ok());
        assert_eq!((megabytes(MOST_SENT), megabytes(MOST_SENT + MEGABYTE / 3)), ("15".to_owned(), "15.4".to_owned()));
    }

    #[test]
    fn layers_drawn_already_are_sent_from_their_pixels() {
        let drawn = Take::pixels(4096, 2048, pixels(4096, 2048, 255), true);
        let clear = Take::pixels(8, 8, pixels(8, 8, 0), false);
        let ready = prepare(&[clear, drawn]).unwrap();
        assert_eq!(ready.iter().map(|s| (s.kind, s.change)).collect::<Vec<_>>(), [("image/jpeg", true), ("image/png", false)]);
        assert_eq!(image::load_from_memory(&ready[0].bytes).map(|p| (p.width(), p.height())).unwrap(), (2048, 1024));
        // Half-transparent pixels are sent with their colour as it is, not darkened.
        let half = prepare(&[Take::pixels(1, 1, pixels(1, 1, 128), true)]).unwrap();
        let pixel = image::load_from_memory(&half[0].bytes).unwrap().to_rgba8().get_pixel(0, 0).0;
        assert!(pixel[0].abs_diff(200) <= 2 && pixel[3] == 128, "{pixel:?}");
        let why = prepare(&[Take::pixels(4, 4, vec![0; 7], true)]).unwrap_err();
        assert_eq!(why, "\"drawn\" couldn't be made ready to send: its pixels don't fill its size.");
        // Human: what is printed of a drawn picture is its size, never its pixels.
        assert_eq!(format!("{:?}", Take::pixels(1, 1, vec![1, 2, 3, 4], true).from), "Pixels(1 × 1, 4 bytes)");
    }
}
