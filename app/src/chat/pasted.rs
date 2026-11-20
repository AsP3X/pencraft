//! Pictures pasted into the message box: kept as files, and shown small where they stand in the text.
// Human: a screenshot or a copied picture pasted into the message becomes a chip in the sentence, with a tiny
// preview of it. The picture is kept as a PNG file under `~/.pencraft/chats/pictures`, so a message of an
// earlier day still shows it when its design is opened again.
// Agent: READS and WRITES files in the window's thread; a screenshot takes a few dozen milliseconds to pack.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use egui::{Color32, ColorImage, Rect, TextureHandle, TextureOptions, Ui, Vec2, pos2};

/// The longest side a pasted picture is kept with, in pixels: more than the models read.
const MOST_SIDE: u32 = 2048;
/// The longest side of the copy kept as a texture, for the chip and the preview under the pointer.
const PREVIEW_SIDE: u32 = 320;
/// The longest side of the preview under the pointer, in points.
const PREVIEW_SHOWN: f32 = 240.0;

/// Counts the pictures pasted in this run, so two pasted in the same instant get files of their own.
static PASTED: AtomicU32 = AtomicU32::new(0);

/// The folder pasted pictures are kept in.
// Human: the tests keep out of the home folder.
fn folder() -> PathBuf {
    if cfg!(test) {
        return std::env::temp_dir().join(format!("pencraft-pasted-test-{}", std::process::id()));
    }
    let chats =
        crate::imagery::store::home().map_or_else(|| std::env::temp_dir().join("pencraft-chats"), |home| home.join(".pencraft").join("chats"));
    chats.join("pictures")
}

/// `picture` as PNG bytes no larger than `most`: scaled down to [`MOST_SIDE`] first, then by half for as long
/// as the file is too large.
fn packed(picture: &ColorImage, most: u64) -> Result<Vec<u8>, String> {
    let (Ok(width), Ok(height)) = (u32::try_from(picture.size[0]), u32::try_from(picture.size[1])) else {
        return Err("it is too large".to_owned());
    };
    let pixels: Vec<u8> = picture.pixels.iter().flat_map(Color32::to_srgba_unmultiplied).collect();
    let mut straight = image::RgbaImage::from_raw(width, height, pixels).filter(|_| width > 0 && height > 0).ok_or("it is empty")?;
    let mut side = MOST_SIDE;
    loop {
        if straight.width().max(straight.height()) > side {
            straight = image::DynamicImage::ImageRgba8(straight).resize(side, side, image::imageops::FilterType::Triangle).into_rgba8();
        }
        let mut bytes = std::io::Cursor::new(Vec::new());
        straight.write_to(&mut bytes, image::ImageFormat::Png).map_err(|error| error.to_string())?;
        let bytes = bytes.into_inner();
        if u64::try_from(bytes.len()).is_ok_and(|size| size <= most) {
            return Ok(bytes);
        }
        side = straight.width().max(straight.height()) / 2;
        if side < 16 {
            return Err("it is too large".to_owned());
        }
    }
}

/// Keeps the pasted `picture` as a PNG file and gives the file.
///
/// # Errors
/// A few words on why the picture couldn't be kept.
pub fn save(picture: &ColorImage) -> Result<PathBuf, String> {
    save_in(&folder(), picture)
}

fn save_in(folder: &Path, picture: &ColorImage) -> Result<PathBuf, String> {
    let bytes = packed(picture, super::files::MOST_PICTURE)?;
    std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_millis());
    let file = folder.join(format!("pasted-{now}-{}-{}.png", std::process::id(), PASTED.fetch_add(1, Ordering::Relaxed)));
    std::fs::write(&file, bytes).map_err(|error| format!("{}: {error}", file.display()))?;
    Ok(file)
}

/// Forgets the file of a pasted picture that was taken out of the message again.
pub fn forget(file: &Path) {
    if file.parent() == Some(folder().as_path()) {
        drop(std::fs::remove_file(file));
    }
}

/// The small copies of the pasted pictures shown so far, by file. A file that can't be read is remembered
/// too, so it is tried once.
#[derive(Clone, Default)]
struct Previews(HashMap<PathBuf, Option<TextureHandle>>);

/// The small copy of the picture in `file` as a texture, read on first use.
fn preview(ctx: &egui::Context, file: &Path) -> Option<TextureHandle> {
    let id = egui::Id::unique("pasted-picture-previews");
    if let Some(known) = ctx.data(|data| data.get_temp::<Previews>(id)).and_then(|previews| previews.0.get(file).cloned()) {
        return known;
    }
    let texture = image::open(file).ok().map(|picture| {
        let small = picture.thumbnail(PREVIEW_SIDE, PREVIEW_SIDE).into_rgba8();
        let size = [usize::try_from(small.width()).unwrap_or(0), usize::try_from(small.height()).unwrap_or(0)];
        ctx.load_texture(file.display().to_string(), ColorImage::from_rgba_unmultiplied(size, &small), TextureOptions::LINEAR)
    });
    ctx.data_mut(|data| data.get_temp_mut_or_default::<Previews>(id).0.insert(file.to_path_buf(), texture.clone()));
    texture
}

/// Paints the tiny preview of the picture in `file` into the square `place`: the middle of the picture,
/// filling the square. Returns whether there was a picture to paint.
pub fn paint_thumb(ui: &Ui, painter: &egui::Painter, place: Rect, file: &Path) -> bool {
    let Some(texture) = preview(ui.ctx(), file) else { return false };
    let size = texture.size_vec2();
    // The part of the picture that is as square as the place.
    let part = if size.x > size.y { Vec2::new(size.y / size.x, 1.0) } else { Vec2::new(1.0, size.x / size.y.max(1.0)) };
    let uv = Rect::from_center_size(pos2(0.5, 0.5), part);
    painter.image(texture.id(), place, uv, Color32::WHITE);
    true
}

/// Shows the picture in `file` larger, for the hint under the pointer.
pub fn show_preview(ui: &mut Ui, file: &Path) {
    if let Some(texture) = preview(ui.ctx(), file) {
        let size = texture.size_vec2();
        let scale = (PREVIEW_SHOWN / size.x.max(size.y).max(1.0)).min(1.0);
        ui.image((texture.id(), size * scale));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pasted_picture_is_kept_as_a_png_small_enough_to_send() {
        let folder = folder().join("kept");
        let picture = ColorImage::filled([40, 20], Color32::from_rgb(200, 30, 30));
        let file = save_in(&folder, &picture).unwrap();
        let back = image::open(&file).unwrap();
        assert_eq!((back.width(), back.height(), file.extension().and_then(|end| end.to_str())), (40, 20, Some("png")));
        // Two pasted one after the other get files of their own.
        assert_ne!(save_in(&folder, &picture).unwrap(), file);
        // A picture wider than the models read is scaled down, keeping its shape.
        let wide = ColorImage::filled([4096, 1024], Color32::WHITE);
        let small = image::load_from_memory(&packed(&wide, super::super::files::MOST_PICTURE).unwrap()).unwrap();
        assert_eq!((small.width(), small.height()), (2048, 512));
        // One that is still too large as a file is halved until it fits; nothing at all is no picture.
        let noisy = ColorImage::from_rgba_unmultiplied(
            [64, 64],
            &(0..64 * 64 * 4).map(|n: u32| u8::try_from((n.wrapping_mul(2_654_435_761) >> 13) % 251).unwrap()).collect::<Vec<u8>>(),
        );
        let fitted = packed(&noisy, 6000).unwrap();
        assert!(fitted.len() <= 6000 && image::load_from_memory(&fitted).unwrap().width() < 64);
        assert!(packed(&ColorImage::filled([0, 0], Color32::WHITE), 6000).is_err());
        drop(std::fs::remove_dir_all(folder));
    }
}
