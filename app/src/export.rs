//! Exporting layers to files: PNG, JPEG, WebP, PDF and SVG.
//!
//! A layer is drawn into a picture by the same code that bakes effects (so an export looks like the canvas),
//! or written as SVG shapes, and saved where the user chose.

use std::io::Cursor;
use std::path::Path;

use pen_doc::vars::Vars;

use crate::editor::Editor;
use crate::render::DisplayList;

/// A file format layers can be exported to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// A picture with transparency.
    Png,
    /// A photo-style picture on white.
    Jpeg,
    /// A picture with transparency, smaller than PNG.
    WebP,
    /// One page per layer.
    Pdf,
    /// The shapes themselves.
    Svg,
}

impl Format {
    /// Every format, in the order the interface lists them.
    pub const ALL: [Format; 5] = [Format::Png, Format::Jpeg, Format::WebP, Format::Pdf, Format::Svg];

    /// Its name in the interface.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Format::Png => "PNG",
            Format::Jpeg => "JPEG",
            Format::WebP => "WEBP",
            Format::Pdf => "PDF",
            Format::Svg => "SVG",
        }
    }

    /// Its files' ending.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpeg => "jpg",
            Format::WebP => "webp",
            Format::Pdf => "pdf",
            Format::Svg => "svg",
        }
    }

    /// Whether it is a picture of pixels, which has a scale to choose.
    #[must_use]
    pub fn has_scale(self) -> bool {
        matches!(self, Format::Png | Format::Jpeg | Format::WebP)
    }
}

/// JPEG quality: High, Medium and Low (95, 75, 50).
pub const QUALITIES: [(&str, u8); 3] = [("High", 95), ("Medium", 75), ("Low", 50)];
/// The scales a picture can be exported at (1x, 2x, 3x).
pub const SCALES: [u8; 3] = [1, 2, 3];
/// Pixels per unit a PDF page's picture is drawn at.
const PDF_SCALE: f32 = 2.0;
/// JPEG quality of a PDF page's picture.
const PDF_QUALITY: u8 = 92;

/// What an export makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// The file format.
    pub format: Format,
    /// Pixels per unit, for the picture formats.
    pub scale: u8,
    /// JPEG quality, 1–100.
    pub quality: u8,
}

impl Default for Options {
    fn default() -> Options {
        Options { format: Format::Png, scale: 1, quality: QUALITIES[0].1 }
    }
}

/// A drawn layer: its size in pixels and its premultiplied RGBA bytes.
pub struct Picture {
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
    /// Four bytes a pixel, colour already multiplied by alpha.
    pub rgba: Vec<u8>,
}

impl Picture {
    /// The pixels with colour not multiplied by alpha, as PNG and WebP store them.
    fn straight(&self) -> Vec<u8> {
        let mut out = self.rgba.clone();
        for pixel in out.as_chunks_mut::<4>().0 {
            let a = u16::from(pixel[3]);
            if a > 0 && a < 255 {
                for channel in &mut pixel[..3] {
                    *channel = u8::try_from((u16::from(*channel) * 255 + a / 2) / a).unwrap_or(255);
                }
            }
        }
        out
    }

    /// The pixels on white without alpha, as JPEG stores them.
    fn on_white(&self) -> Vec<u8> {
        self.rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2]].map(|channel| channel.saturating_add(255 - p[3]))).collect()
    }

    fn size(&self) -> Result<(u32, u32), String> {
        Ok((u32::try_from(self.w).map_err(|e| e.to_string())?, u32::try_from(self.h).map_err(|e| e.to_string())?))
    }

    /// The picture as a JPEG file.
    fn jpeg(&self, quality: u8) -> Result<Vec<u8>, String> {
        let (w, h) = self.size()?;
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality.clamp(1, 100))
            .encode(&self.on_white(), w, h, image::ExtendedColorType::Rgb8)
            .map_err(|e| e.to_string())?;
        Ok(out)
    }

    /// The picture as a file of `format` (a picture format).
    pub fn encode(&self, format: Format, quality: u8) -> Result<Vec<u8>, String> {
        let (w, h) = self.size()?;
        let mut out = Vec::new();
        match format {
            Format::Jpeg => return self.jpeg(quality),
            Format::Png => {
                let image = image::RgbaImage::from_raw(w, h, self.straight()).ok_or("the picture has the wrong size")?;
                image.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png).map_err(|e| e.to_string())?;
            }
            // Human: the image library writes WebP without loss only (the "Lossless" quality).
            Format::WebP => image::codecs::webp::WebPEncoder::new_lossless(&mut out)
                .encode(&self.straight(), w, h, image::ExtendedColorType::Rgba8)
                .map_err(|e| e.to_string())?,
            Format::Pdf | Format::Svg => return Err("not a picture format".to_owned()),
        }
        Ok(out)
    }
}

/// One page of a PDF: its size in points, and its picture as a JPEG with the picture's size in pixels.
struct Page {
    size: (f32, f32),
    jpeg: Vec<u8>,
    pixels: (usize, usize),
}

/// A PDF with one page per picture.
// Human: each page is the layer's size in points (one unit is one point) and holds the layer as a picture;
// the PDF is not vector. A PDF is a list of numbered objects followed by a table of where each one starts.
fn pdf(pages: &[Page]) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-1.4\n".to_vec();
    let mut starts = Vec::new();
    let mut object = |out: &mut Vec<u8>, body: &[u8]| {
        starts.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", starts.len()).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };
    object(&mut out, b"<< /Type /Catalog /Pages 2 0 R >>");
    let kids: Vec<String> = (0..pages.len()).map(|page| format!("{} 0 R", 3 + 3 * page)).collect();
    object(&mut out, format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), pages.len()).as_bytes());
    for (index, page) in pages.iter().enumerate() {
        let first = 3 + 3 * index;
        let (w, h) = page.size;
        object(
            &mut out,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w:.2} {h:.2}] /Resources << /XObject << /Im0 {} 0 R >> >> /Contents {} 0 R >>",
                first + 2,
                first + 1
            )
            .as_bytes(),
        );
        let drawing = format!("q {w:.2} 0 0 {h:.2} 0 0 cm /Im0 Do Q");
        object(&mut out, format!("<< /Length {} >>\nstream\n{drawing}\nendstream", drawing.len()).as_bytes());
        let mut picture = format!(
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
            page.pixels.0,
            page.pixels.1,
            page.jpeg.len()
        )
        .into_bytes();
        picture.extend_from_slice(&page.jpeg);
        picture.extend_from_slice(b"\nendstream");
        object(&mut out, &picture);
    }
    let table = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", starts.len() + 1).as_bytes());
    for start in &starts {
        out.extend_from_slice(format!("{start:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{table}\n%%EOF\n", starts.len() + 1).as_bytes());
    out
}

/// A layer's name as a file name: without the characters a file system refuses.
fn file_stem(name: &str) -> String {
    let clean: String = name.chars().map(|c| if matches!(c, '/' | '\\' | ':' | '\0') || c.is_control() { '-' } else { c }).collect();
    let clean = clean.trim().trim_matches('.').to_owned();
    if clean.is_empty() { "Layer".to_owned() } else { clean.chars().take(80).collect() }
}

/// The layers an export takes, back to front as the canvas paints them: the selected ones, or with nothing
/// selected every top-level layer of the document. A layer inside another one that is taken is part of that one.
// Human: several layers are exported together, as they lie on the canvas, in one file.
fn layers(editor: &Editor) -> Vec<&pen_doc::scene::SceneNode> {
    fn chosen<'a>(nodes: &'a [pen_doc::scene::SceneNode], selection: &[String], out: &mut Vec<&'a pen_doc::scene::SceneNode>) {
        for node in nodes {
            if selection.contains(&node.id) {
                out.push(node);
            } else {
                chosen(&node.children, selection, out);
            }
        }
    }
    if editor.selection.is_empty() {
        return editor.scene.iter().collect();
    }
    let mut out = Vec::new();
    chosen(&editor.scene, &editor.selection, &mut out);
    out
}

/// The file name the save dialog suggests for exporting the selection.
#[must_use]
pub fn suggested_name(editor: &Editor, options: Options) -> String {
    let name = match layers(editor).as_slice() {
        [scene] => scene.node.display_name(),
        _ => editor.title(),
    };
    format!("{}.{}", file_stem(&name), options.format.extension())
}

/// Exports the selected layers of `editor` (or all of them, with nothing selected) together into the file
/// `target`.
///
/// # Errors
/// A sentence when nothing can be exported, the picture would be too large, or the file can't be written.
// Human: several layers come out as one picture (or one PDF page, or one SVG): each where it is on the canvas,
// in a file as large as the box around them all, without what lies between them unselected.
// Agent: MAIN-THREAD (drawing text needs the egui context); BLOCKS while drawing and writing.
pub fn run(list: &DisplayList, editor: &Editor, ctx: &egui::Context, options: Options, target: &Path) -> Result<(), String> {
    let layers = layers(editor);
    if layers.is_empty() {
        return Err("There is nothing to export.".to_owned());
    }
    let vars = Vars::new(&editor.doc);
    let folder = editor.path.as_deref().and_then(Path::parent);
    let draw = |scale: f32| list.snapshot(&layers, scale, vars, ctx, folder).ok_or_else(|| "The layers are empty or too large to export.".to_owned());
    let bytes = match options.format {
        Format::Pdf => {
            let picture = draw(PDF_SCALE)?;
            let size = (narrow_len(picture.w) / PDF_SCALE, narrow_len(picture.h) / PDF_SCALE);
            pdf(&[Page { size, jpeg: picture.jpeg(PDF_QUALITY)?, pixels: (picture.w, picture.h) }])
        }
        Format::Svg => list.svg(&layers, vars, ctx, folder).ok_or("The layers have no size to export.")?.into_bytes(),
        Format::Png | Format::Jpeg | Format::WebP => draw(f32::from(options.scale.max(1)))?.encode(options.format, options.quality)?,
    };
    craft_atomic::atomic_write(target, &bytes).map_err(|e| format!("{}: {e}", target.display()))
}

/// A count of pixels as a length.
#[expect(clippy::cast_precision_loss, reason = "pictures are far smaller than f32's exact range")]
fn narrow_len(pixels: usize) -> f32 {
    pixels as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture() -> Picture {
        // Two pixels: opaque red, and half-transparent green (premultiplied).
        Picture { w: 2, h: 1, rgba: vec![255, 0, 0, 255, 0, 128, 0, 128] }
    }

    #[test]
    fn pictures_encode_to_files_that_read_back() {
        let png = picture().encode(Format::Png, 95).unwrap();
        let back = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!((back.width(), back.height()), (2, 1));
        // Colour is stored unmultiplied: the half-transparent green is full green at half alpha.
        assert_eq!(back.as_raw(), &[255, 0, 0, 255, 0, 255, 0, 128]);
        let webp = picture().encode(Format::WebP, 95).unwrap();
        assert_eq!(image::load_from_memory(&webp).unwrap().to_rgba8().as_raw(), back.as_raw());
        // A JPEG has no transparency: the green lies on white.
        let jpeg = image::load_from_memory(&picture().encode(Format::Jpeg, 95).unwrap()).unwrap().to_rgb8();
        assert_eq!((jpeg.width(), jpeg.height()), (2, 1));
        assert!(picture().encode(Format::Svg, 95).is_err());
        assert_eq!(picture().on_white(), [255, 0, 0, 127, 255, 127]);
    }

    #[test]
    fn a_pdf_lists_its_pages_and_where_every_object_starts() {
        let jpeg = picture().jpeg(90).unwrap();
        let file = pdf(&[Page { size: (100.0, 50.0), jpeg: jpeg.clone(), pixels: (2, 1) }, Page { size: (20.0, 10.0), jpeg, pixels: (2, 1) }]);
        let text = String::from_utf8_lossy(&file);
        assert!(text.starts_with("%PDF-1.4") && text.trim_end().ends_with("%%EOF"));
        assert!(
            text.contains("/Kids [3 0 R 6 0 R] /Count 2") && text.contains("/MediaBox [0 0 100.00 50.00]") && text.contains("/Filter /DCTDecode")
        );
        // The table at the end points at each object's first byte.
        let table = text.rfind("startxref\n").map(|at| text[at + 10..].lines().next().unwrap().parse::<usize>().unwrap()).unwrap();
        assert!(file[table..].starts_with(b"xref\n0 9\n"));
        let entries: Vec<usize> = String::from_utf8_lossy(&file[table..]).lines().skip(3).take(8).map(|line| line[..10].parse().unwrap()).collect();
        for (number, start) in entries.iter().enumerate() {
            assert!(file[*start..].starts_with(format!("{} 0 obj", number + 1).as_bytes()), "object {}", number + 1);
        }
    }

    #[test]
    fn layer_names_become_safe_file_names() {
        assert_eq!(file_stem("Card / Title: big"), "Card - Title- big");
        assert_eq!(file_stem("  .."), "Layer");
        assert_eq!((Format::Jpeg.extension(), Format::WebP.label(), Format::Pdf.has_scale(), Format::Png.has_scale()), ("jpg", "WEBP", false, true));
    }
}
