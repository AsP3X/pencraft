//! Pictures for image fills: read from disk once, kept as textures for as long as the canvas lives.
//!
//! A fill's `url` is a file path, relative to the folder of the `.pen` file. Nothing is fetched from the network;
//! a web address, a missing file or an unreadable picture leaves the fill on its grey placeholder.
//! Pictures an image service made are files like any other by the time they get here ([`crate::imagery`]).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::{ColorImage, TextureHandle, TextureId, TextureOptions};
use pen_doc::value::narrow;

/// Pictures wider or taller than this are scaled down before they become a texture, to bound memory.
const MAX_SIDE: u32 = 4096;
/// The longest side of the copy of a picture kept for effects (blurs and shadows are worked out on the CPU).
const EFFECT_SIDE: u32 = 1024;

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

/// Every picture asked for so far, by file path. A file that could not be read is remembered too, so it is
/// tried (and reported) once.
// Agent: a picture changed on disk is NOT reloaded until the app restarts.
#[derive(Default)]
pub struct ImageStore {
    loaded: RefCell<HashMap<PathBuf, Option<(TextureHandle, Picture)>>>,
    /// A smaller copy of each picture's pixels, for effects.
    pixels: RefCell<HashMap<TextureId, Arc<ColorImage>>>,
}

impl ImageStore {
    /// The picture `url` names, loading it on first use. `base` is the folder of the document.
    pub fn get(&self, ctx: &egui::Context, base: Option<&Path>, url: &str) -> Option<Picture> {
        let path = resolve(base, url)?;
        let mut loaded = self.loaded.borrow_mut();
        if let Some(entry) = loaded.get(&path) {
            return entry.as_ref().map(|(_, picture)| *picture);
        }
        let max_side = u32::try_from(ctx.input(|i| i.max_texture_side)).unwrap_or(MAX_SIDE).min(MAX_SIDE);
        let entry = match decode(&path, max_side) {
            Ok(Decoded { pixels, small, width, height }) => {
                let handle = ctx.load_texture(path.display().to_string(), pixels, TextureOptions::LINEAR);
                self.pixels.borrow_mut().insert(handle.id(), Arc::new(small));
                let picture = Picture { texture: handle.id(), width, height };
                Some((handle, picture))
            }
            Err(error) => {
                log::warn!("image fill: {}: {error}", path.display());
                None
            }
        };
        let picture = entry.as_ref().map(|(_, picture)| *picture);
        loaded.insert(path, entry);
        picture
    }

    /// The pixels of a loaded picture (at most [`EFFECT_SIDE`] a side), for effects that blur it.
    pub fn pixels(&self, texture: TextureId) -> Option<Arc<ColorImage>> {
        self.pixels.borrow().get(&texture).cloned()
    }
}

/// A picture file read into egui's pixel format.
struct Decoded {
    /// The picture, for its texture.
    pixels: ColorImage,
    /// A copy no larger than [`EFFECT_SIDE`] a side.
    small: ColorImage,
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
    let path = Path::new(local(url));
    // Human: on Windows a path can start at the top of a drive without naming the drive (`/pictures/a.png`,
    // as a file made on a Mac has it): that is the drive the document is on. Elsewhere such a path is whole.
    if path.is_absolute() || (path.has_root() && base.is_none()) { Some(path.to_path_buf()) } else { base.map(|folder| folder.join(path)) }
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
    let pixels = color_image(&picture)?;
    let small = if picture.width() > EFFECT_SIDE || picture.height() > EFFECT_SIDE {
        color_image(&picture.thumbnail(EFFECT_SIDE, EFFECT_SIDE))?
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
