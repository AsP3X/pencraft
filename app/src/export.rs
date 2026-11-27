//! Exporting layers to files: PNG, JPEG, WebP, PDF and SVG.
//!
//! A layer is drawn into a picture by the same code that bakes effects (so an export looks like the canvas),
//! or written as SVG shapes, and saved where the user chose.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use pen_doc::scene::SceneNode;
use pen_doc::vars::Vars;

use crate::editor::Editor;
use crate::render::DisplayList;
use crate::theme::{
    CHIP_RADIUS, DIALOG_BACKDROP, DIALOG_BUTTON, DIALOG_GAP, DIALOG_PADDING, DIALOG_TITLE_SIZE, DIALOG_WIDTH, FIELD_HEIGHT, Palette, SETTINGS_RADIUS,
};

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
pub(crate) const PDF_SCALE: f32 = 2.0;
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
    pub(crate) fn straight(&self) -> Vec<u8> {
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
/// `target`, here and now.
///
/// # Errors
/// A sentence when nothing can be exported, the picture would be too large, or the file can't be written.
// Human: for the run that makes one picture and ends (`--export`), which has nobody to show progress to.
// Agent: BLOCKS while drawing and writing; replaces a file that is there.
pub fn run(list: &DisplayList, editor: &Editor, ctx: &egui::Context, options: Options, target: &Path) -> Result<(), String> {
    Job::of(editor, options, target.to_path_buf(), true)?.make(list, ctx, &AtomicBool::new(false)).map(|_| ())
}

/// An export, taken from the document so that a thread of its own can make it: the layers as they are laid
/// out, the document's variables, and where the file goes.
// Human: several layers come out as one picture (or one PDF page, or one SVG): each where it is on the canvas,
// in a file as large as the box around them all, without what lies between them unselected.
pub struct Job {
    layers: Vec<SceneNode>,
    variables: Option<serde_json::Map<String, serde_json::Value>>,
    folder: Option<PathBuf>,
    options: Options,
    target: PathBuf,
    /// Whether a file that is there may be replaced (the user said so in the save dialog; a model never may).
    replace: bool,
}

/// How an export that didn't fail ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// The file is written.
    Written,
    /// It was called off; no file was written.
    Cancelled,
}

impl Job {
    /// The export of the selected layers of `editor` (or all of them, with nothing selected) to `target`.
    ///
    /// # Errors
    /// A sentence when there is nothing to export.
    // Agent: COPIES the layers to export; the document can change afterwards.
    pub fn of(editor: &Editor, options: Options, target: PathBuf, replace: bool) -> Result<Job, String> {
        let layers: Vec<SceneNode> = layers(editor).into_iter().cloned().collect();
        if layers.is_empty() {
            return Err("There is nothing to export.".to_owned());
        }
        Ok(Job { layers, variables: editor.doc.variables.clone(), folder: editor.folder().map(Path::to_path_buf), options, target, replace })
    }

    /// Draws the layers and writes the file, unless `stop` is set before the file is put in its place.
    ///
    /// # Errors
    /// A sentence when the picture would be too large or the file can't be written.
    // Human: an export that is called off leaves nothing behind. The file is written beside its place under
    // another name and given its own name only at the end; `stop` is looked at one last time just before
    // that, and an export called off by then is thrown away with what was written.
    // Agent: BLOCKS while drawing and writing (drawing text needs the egui context, a clone of which works
    // on any thread). WRITES `target` through `craft_atomic`, or nothing.
    pub fn make(&self, list: &DisplayList, ctx: &egui::Context, stop: &AtomicBool) -> Result<Ended, String> {
        let stopped = || stop.load(Ordering::Relaxed);
        let (layers, options): (Vec<&SceneNode>, Options) = (self.layers.iter().collect(), self.options);
        let (vars, folder) = (Vars::of(self.variables.as_ref()), self.folder.as_deref());
        let draw =
            |scale: f32| list.snapshot(&layers, scale, vars, ctx, folder).ok_or_else(|| "The layers are empty or too large to export.".to_owned());
        if stopped() {
            return Ok(Ended::Cancelled);
        }
        let bytes = match options.format {
            // Human: a PDF is vectors and real text wherever a PDF can say what the design shows; a design with a
            // material, a background blur or a shader in it is one picture on its page, as it always was.
            Format::Pdf => {
                if let Some(vectors) = list.pdf(&layers, vars, ctx, folder) {
                    vectors?
                } else {
                    let picture = draw(PDF_SCALE)?;
                    let size = (narrow_len(picture.w) / PDF_SCALE, narrow_len(picture.h) / PDF_SCALE);
                    pdf(&[Page { size, jpeg: picture.jpeg(PDF_QUALITY)?, pixels: (picture.w, picture.h) }])
                }
            }
            Format::Svg => list.svg(&layers, vars, ctx, folder).ok_or("The layers have no size to export.")?.into_bytes(),
            Format::Png | Format::Jpeg | Format::WebP => {
                let picture = draw(f32::from(options.scale.max(1)))?;
                if stopped() {
                    return Ok(Ended::Cancelled);
                }
                picture.encode(options.format, options.quality)?
            }
        };
        put(&self.target, &bytes, self.replace, stop)
    }

    /// Starts making the export on a thread of its own, which tells `ctx` to draw again when it is done.
    ///
    /// # Errors
    /// A sentence when no thread could be started.
    // Human: drawing a large design and packing it into a file takes seconds, and the window stood still for
    // them. The thread draws with a list of its own, so the canvas is never kept waiting for it; the
    // pictures of the design are read again there.
    // Agent: SPAWNS one thread, `pencraft-export`, which ends with the export.
    pub fn start(self, ctx: &egui::Context) -> Result<Running, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let (send, arrives) = mpsc::channel();
        let (wake, called_off, target) = (ctx.clone(), stop.clone(), self.target.clone());
        let work = move || {
            let ended = self.make(&DisplayList::default(), &wake, &called_off);
            // Whoever started it may be gone (the app is closing); then nobody takes the answer.
            let _unheard = send.send(ended);
            wake.request_repaint();
        };
        match std::thread::Builder::new().name("pencraft-export".to_owned()).spawn(work) {
            Ok(_) => Ok(Running { target, stop, arrives }),
            Err(error) => Err(format!("no thread to make the export on: {error}")),
        }
    }
}

/// Writes `bytes` as the file `target`, unless `stop` is set by the time they are written: then nothing is
/// left, neither the file nor what was written beside it. A file that is there stays unless `replace`.
// Agent: WRITES through `craft_atomic`, which writes beside `target` and renames at the end; an error from
// the closure makes it remove what it wrote and rename nothing.
fn put(target: &Path, bytes: &[u8], replace: bool, stop: &AtomicBool) -> Result<Ended, String> {
    if !replace && target.exists() {
        return Err(format!("{} is there already.", target.display()));
    }
    let mut called_off = false;
    let written = craft_atomic::atomic_write_with(target, |file| {
        std::io::Write::write_all(file, bytes)?;
        if stop.load(Ordering::Relaxed) {
            called_off = true;
            return Err(std::io::Error::other("the export was called off"));
        }
        Ok(())
    });
    match written {
        _ if called_off => Ok(Ended::Cancelled),
        Ok(()) => Ok(Ended::Written),
        Err(error) => Err(format!("{}: {error}", target.display())),
    }
}

/// An export being made on a thread of its own.
pub struct Running {
    /// The file it writes.
    pub target: PathBuf,
    stop: Arc<AtomicBool>,
    arrives: mpsc::Receiver<Result<Ended, String>>,
}

impl Running {
    /// Calls the export off: it writes no file. One that has already put its file in place stays written,
    /// and says so.
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// How the export ended, once it has; `None` while it is being made.
    pub fn ended(&self) -> Option<Result<Ended, String>> {
        match self.arrives.try_recv() {
            Ok(ended) => Some(ended),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err("the export ended without an answer".to_owned())),
        }
    }
}

/// The window that shows while an export to the file `name` is made. Says whether Cancel was chosen (the
/// button, Esc, or a click beside the window).
// Human: in the manner of "Generate image": a small window over the app with what is going on, a turning
// wheel (how far the export is isn't known), and the one thing there is to do.
// Agent: MAIN-THREAD.
pub fn progress(ctx: &egui::Context, palette: Palette, name: &str) -> bool {
    let frame = egui::Frame::new()
        .fill(palette.island)
        .stroke(egui::Stroke::new(1.0, palette.outline))
        .corner_radius(SETTINGS_RADIUS)
        .inner_margin(egui::Margin::same(DIALOG_PADDING));
    let id = egui::Id::unique("exporting");
    let mut cancel = false;
    let modal = egui::Modal::new(id)
        .area(egui::Modal::default_area(id).fade_in(false))
        .frame(frame)
        .backdrop_color(egui::Color32::from_black_alpha(DIALOG_BACKDROP))
        .show(ctx, |ui| {
            ui.set_width(DIALOG_WIDTH[0]);
            ui.add(egui::Label::new(egui::RichText::new("Exporting").size(DIALOG_TITLE_SIZE).color(palette.foreground)).selectable(false));
            ui.add_space(DIALOG_GAP);
            ui.horizontal(|ui| {
                ui.spinner();
                ui.add(egui::Label::new(egui::RichText::new(name).color(palette.muted)).truncate().selectable(false));
            });
            ui.add_space(DIALOG_GAP);
            let button = egui::Button::new("Cancel").corner_radius(CHIP_RADIUS).min_size(egui::vec2(DIALOG_BUTTON, FIELD_HEIGHT));
            cancel = ui.add(button).clicked();
        });
    cancel || modal.should_close()
}

/// A count of pixels as a length.
#[expect(clippy::cast_precision_loss, reason = "pictures are far smaller than f32's exact range")]
pub(crate) fn narrow_len(pixels: usize) -> f32 {
    pixels as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture() -> Picture {
        // Two pixels: opaque red, and half-transparent green (premultiplied).
        Picture { w: 2, h: 1, rgba: vec![255, 0, 0, 255, 0, 128, 0, 128] }
    }

    /// A folder of its own for a test's files.
    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("quill-export-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn files_in(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> =
            std::fs::read_dir(folder).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    fn editor() -> Editor {
        let json = r##"{"version":"2.22","children":[{"type":"rectangle","id":"box","x":0,"y":0,"width":20,"height":10,"fill":"#ff0000"}]}"##;
        let mut editor = Editor::new(pen_doc::read(json).unwrap(), None, 1);
        editor.refresh(&|_| (10.0, 10.0));
        editor
    }

    #[test]
    fn an_export_called_off_leaves_no_file_and_nothing_beside_it() {
        let folder = folder("called-off");
        let target = folder.join("out.png");
        // Called off by the time the bytes are written: nothing is put in place, and nothing is left beside.
        assert_eq!(put(&target, b"bytes", true, &AtomicBool::new(true)), Ok(Ended::Cancelled));
        assert_eq!(files_in(&folder), Vec::<String>::new());
        // A file that was there is as it was after an export over it was called off.
        std::fs::write(&target, b"mine").unwrap();
        assert_eq!(put(&target, b"bytes", true, &AtomicBool::new(true)), Ok(Ended::Cancelled));
        assert_eq!((std::fs::read(&target).unwrap(), files_in(&folder)), (b"mine".to_vec(), vec!["out.png".to_owned()]));
        // Not called off, it is replaced only when that was allowed.
        assert!(put(&target, b"bytes", false, &AtomicBool::new(false)).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"mine");
        assert_eq!(put(&target, b"bytes", true, &AtomicBool::new(false)), Ok(Ended::Written));
        assert_eq!((std::fs::read(&target).unwrap(), files_in(&folder)), (b"bytes".to_vec(), vec!["out.png".to_owned()]));
        // A whole export called off before it began draws nothing and writes nothing.
        let job = Job::of(&editor(), Options::default(), folder.join("never.png"), true).unwrap();
        assert_eq!(job.make(&DisplayList::default(), &egui::Context::default(), &AtomicBool::new(true)), Ok(Ended::Cancelled));
        assert_eq!(files_in(&folder), vec!["out.png".to_owned()]);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn an_export_is_made_on_a_thread_of_its_own_from_the_layers_as_they_were() {
        let folder = folder("thread");
        let (ctx, mut editor) = (egui::Context::default(), editor());
        let ended = |running: &Running| {
            for _ in 0..5000 {
                if let Some(ended) = running.ended() {
                    return ended;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            Err("the export never ended".to_owned())
        };
        let running = Job::of(&editor, Options { format: Format::Png, scale: 2, ..Default::default() }, folder.join("box.png"), true)
            .unwrap()
            .start(&ctx)
            .unwrap();
        // The document changes while the file is made: the file shows it as it was when the export began.
        editor.doc.children.clear();
        assert_eq!(ended(&running), Ok(Ended::Written));
        let back = image::open(&running.target).unwrap().to_rgba8();
        assert_eq!((back.width(), back.height(), back.get_pixel(20, 10).0), (40, 20, [255, 0, 0, 255]));
        // Called off at once, it writes nothing; nothing to export is said before a thread is started.
        let running = Job::of(&self::editor(), Options::default(), folder.join("off.png"), true).unwrap().start(&ctx).unwrap();
        running.cancel();
        let how = ended(&running);
        assert!(how == Ok(Ended::Cancelled) || how == Ok(Ended::Written), "{how:?}");
        assert_eq!(folder.join("off.png").exists(), how == Ok(Ended::Written));
        editor.touched();
        editor.refresh(&|_| (10.0, 10.0));
        assert!(Job::of(&editor, Options::default(), folder.join("empty.png"), true).is_err());
        std::fs::remove_dir_all(&folder).unwrap();
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
