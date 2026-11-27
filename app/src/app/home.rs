//! The home screen: a box to describe a design in, canvases to start from, and the recent files with a small
//! picture of each.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use egui::{Align, Align2, Color32, FontFamily, FontId, Layout, Rect, Response, RichText, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};

use super::App;
use crate::editor::Editor;
use crate::sidebar::{ChatAsk, text_button};
use crate::theme::{
    self, ACCENT, CHIP_HEIGHT, CHIP_RADIUS, HOME_CARD_GAP, HOME_CARD_LINES, HOME_CARD_PADDING, HOME_CARD_RADIUS, HOME_CARD_TEXT, HOME_EMPTY_HEIGHT,
    HOME_KEY_HEIGHT, HOME_MARGIN, HOME_PICTURE_INSET, HOME_PROMPT_TEXT, HOME_RECENT_SHAPE, HOME_RECENT_WIDTH, HOME_SECTION_GAP,
    HOME_SECTION_TITLE_GAP, HOME_SEND, HOME_SMALL_TEXT, HOME_START_PICTURE, HOME_START_WIDTH, HOME_THUMB_SCALE, HOME_THUMB_SIZE, HOME_TITLE_GAP,
    HOME_TITLE_SIZE, HOME_TOP, HOME_WIDTH, ICON_SIZE, ISLAND_PADDING, ISLAND_RADIUS, Palette, SEND_ARROW, SIDEBAR_ROW, SIDEBAR_TITLE_SIZE,
};

/// What the box at the top shows while it is empty.
const HINT: &str = "Describe a screen or a page, and the assistant draws it on a new canvas";
/// Designs to describe, offered under the box; a click writes one into it.
const IDEAS: [&str; 4] = [
    "A pricing page with three plans",
    "A sign-in screen for a banking app",
    "A dashboard for a bike rental fleet",
    "A landing page for an independent bookshop",
];
/// How often the recent files are looked at on disk while the home screen shows.
const LOOK_EVERY: Duration = Duration::from_secs(2);
/// The largest design file a small picture is made of; a larger one shows without.
const MOST_BYTES: u64 = 6_000_000;
/// How often a small picture is made again because a face of its text wasn't ready.
const MOST_TRIES: u8 = 3;
/// The most cards in a row.
const MOST_COLUMNS: u8 = 6;
/// The name the starter board's small picture is kept under; no file has it.
const SAMPLE: &str = "<starter board>";

/// A screen size to start a design at.
#[derive(Debug, PartialEq)]
struct Screen {
    name: &'static str,
    w: f32,
    h: f32,
}

/// The screens offered: a phone, a tablet and a desktop window, at the sizes designs are most often drawn at.
const SCREENS: [Screen; 3] =
    [Screen { name: "Phone", w: 390.0, h: 844.0 }, Screen { name: "Tablet", w: 820.0, h: 1180.0 }, Screen { name: "Desktop", w: 1440.0, h: 900.0 }];

/// What a design can be started from.
#[derive(Clone, Copy)]
enum Start {
    /// An empty document.
    Blank,
    /// A document with one frame the size of a screen.
    Screen(&'static Screen),
    /// The starter board.
    Sample,
}

impl Start {
    /// The card's two lines: its name, and what it holds.
    fn lines(self) -> (&'static str, String) {
        match self {
            Start::Blank => ("Blank canvas", "An empty document".to_owned()),
            Start::Screen(screen) => (screen.name, format!("{} × {}", screen.w, screen.h)),
            Start::Sample => ("Starter board", "Layouts and themes".to_owned()),
        }
    }
}

/// What the home screen was asked to do this frame; done once it is drawn.
enum Act {
    Start(Start),
    Open,
    /// Open the recent file at this path.
    Recent(String),
    /// Take this path out of the recent files.
    Forget(String),
    /// Start a design and ask the assistant for what these words describe.
    Describe(String),
    /// Show where assistants are set up.
    Assistants,
}

/// One recent file as the home screen shows it.
struct Recent {
    path: String,
    name: String,
    /// The folder it is in, with the user's own folder written `~`.
    folder: String,
    /// When it was last written; `None` when it isn't there any more.
    changed: Option<SystemTime>,
}

impl Recent {
    fn of(path: &str, own: Option<&Path>) -> Recent {
        let file = Path::new(path);
        Recent {
            path: path.to_owned(),
            name: file.file_stem().map_or_else(|| path.to_owned(), |name| name.to_string_lossy().into_owned()),
            folder: file.parent().map(|folder| shortened(folder, own)).unwrap_or_default(),
            changed: std::fs::metadata(file).and_then(|data| data.modified()).ok(),
        }
    }
}

/// A design's small picture.
struct Thumb {
    /// When the file it was made of was last written.
    stamp: Option<SystemTime>,
    /// `None` when the design is empty, too large or can't be read.
    texture: Option<egui::TextureHandle>,
    /// A face of its text wasn't ready when it was made.
    rough: bool,
    /// How often it was made again for that.
    tries: u8,
}

/// What the home screen keeps from frame to frame.
#[derive(Default)]
pub(super) struct Home {
    /// What is written in the box at the top.
    prompt: String,
    /// The recent files as last looked at on disk, and when that was.
    files: Vec<Recent>,
    looked: Option<Instant>,
    /// The small pictures, by the file's path.
    thumbs: HashMap<String, Thumb>,
    /// The one small picture that is being made on another thread, while one is.
    making: Option<Making>,
}

/// A small picture on its way: which file it is of, and where it arrives.
struct Making {
    path: String,
    /// When the file was last written, as it was when the making began.
    stamp: Option<SystemTime>,
    arrives: mpsc::Receiver<Made>,
}

/// A small picture as its thread made it.
struct Made {
    /// `None` when the design is empty, too large or can't be read.
    image: Option<egui::ColorImage>,
    /// A face of its text wasn't ready while it was made.
    rough: bool,
}

impl App {
    /// The home screen.
    // Human: one column in the middle of the window, everything in it starting at the same left edge: the box to
    // describe a design in, the canvases to start from, the recent files.
    // Agent: MAIN-THREAD; what a click asks for is done after the frame is drawn, when nothing of the screen's
    // own is borrowed any more.
    pub(super) fn home(&mut self, ui: &mut egui::Ui) {
        let palette = theme::palette(self.dark());
        let ctx = ui.ctx().clone();
        self.look_at_recent();
        self.home.tend_pictures(&ctx);
        let assistant = self.assistant().map(|choice| choice.label.clone());
        let ideas = self.prefs.get().agents.ideas;
        let describe = self.prefs.get().agents.input_on_dashboard;
        let now = SystemTime::now();
        let mut act = None;
        let home = &mut self.home;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let room = ui.available_width();
            let width = column(room);
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.add_space(((room - width) / 2.0).max(0.0).floor());
                ui.vertical(|ui| {
                    ui.set_width(width);
                    ui.add_space(HOME_TOP);
                    let bold = FontFamily::Name(crate::render::BOLD_FAMILY.into());
                    // Human: the settings can take the box away ("Show agent input on dashboard"); the home
                    // screen then starts at the designs to begin from.
                    if describe {
                        words(ui, "What are you designing?", FontId::new(HOME_TITLE_SIZE, bold), palette.foreground);
                        ui.add_space(HOME_TITLE_GAP);
                        home.describe(ui, assistant.as_deref(), ideas, palette, &mut act);
                        ui.add_space(HOME_SECTION_GAP);
                    } else {
                        words(ui, "Start a design", FontId::new(HOME_TITLE_SIZE, bold), palette.foreground);
                        ui.add_space(HOME_TITLE_GAP);
                    }
                    words(ui, "Start from", FontId::proportional(SIDEBAR_TITLE_SIZE), palette.foreground);
                    ui.add_space(HOME_SECTION_TITLE_GAP);
                    home.starts(ui, width, palette, &mut act);
                    ui.add_space(HOME_SECTION_GAP - (SIDEBAR_ROW - SIDEBAR_TITLE_SIZE) / 2.0);
                    ui.horizontal(|ui| {
                        words(ui, "Recent", FontId::proportional(SIDEBAR_TITLE_SIZE), palette.foreground);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let open = text_button(ui, Some("folder-open"), "Open…", vec2(88.0, SIDEBAR_ROW), false, true, palette);
                            if open.on_hover_text(crate::keys::text("Open a design (⌘O)")).clicked() {
                                act = Some(Act::Open);
                            }
                        });
                    });
                    ui.add_space(HOME_SECTION_TITLE_GAP - (SIDEBAR_ROW - SIDEBAR_TITLE_SIZE) / 2.0);
                    home.recent(ui, width, now, palette, &mut act);
                    ui.add_space(HOME_SECTION_GAP);
                    last_line(ui, palette);
                    ui.add_space(HOME_TOP);
                });
            });
        });
        if let Some(act) = act {
            self.home_do(act);
        }
    }

    /// Does what the home screen asked for.
    fn home_do(&mut self, act: Act) {
        match act {
            Act::Start(Start::Blank) => self.new_file(),
            Act::Start(Start::Sample) => self.open_sample(),
            Act::Start(Start::Screen(screen)) => self.new_screen(screen),
            Act::Open => self.ask_open(),
            Act::Recent(path) => {
                if Path::new(&path).exists() {
                    self.open_path(Path::new(&path));
                } else {
                    self.warn(format!("{path} isn't there any more. Right-click it to take it off this list."));
                }
            }
            Act::Forget(path) => {
                self.prefs.update(|prefs| prefs.files.recent.retain(|recent| *recent != path));
            }
            Act::Describe(text) => {
                // Human: the words go to the chat of a new, empty document, which comes to the front with its
                // chat showing, so the design can be watched as the assistant draws it.
                self.new_file();
                self.bring_chat();
                self.sidebar.pose("chat");
                self.sidebar.chat_asks.push(ChatAsk::said(text, Vec::new()));
            }
            Act::Assistants => self.settings.open_named("agents"),
        }
    }

    /// Starts a document with one frame the size of `screen`, selected.
    fn new_screen(&mut self, screen: &Screen) {
        let seed = self.next_seed();
        let mut editor = Editor::new(pen_doc::Document::default(), None, seed);
        let mut frame = pen_doc::Node::new(pen_doc::Kind::Frame, editor.fresh_id());
        frame.name = Some(screen.name.to_owned());
        frame.layout = Some("none".to_owned());
        frame.clip = Some(pen_doc::value::Flag::Value(true));
        frame.fill = Some(pen_doc::model::Paints::One(pen_doc::model::Paint::Solid(crate::canvas::FRAME_FILL.to_owned())));
        frame.width = Some(pen_doc::value::Num::new(screen.w));
        frame.height = Some(pen_doc::value::Num::new(screen.h));
        crate::ops::set_position(&mut frame, 0.0, 0.0);
        editor.selection = vec![frame.id.clone()];
        editor.doc.children.push(frame);
        self.add_tab(editor);
    }

    /// For a picture of the app (`--home`): the home screen, with `files` as its recent files.
    pub(super) fn pose_home(&mut self, files: &[PathBuf]) {
        for file in files.iter().rev() {
            let shown = file.display().to_string();
            self.prefs.update(|prefs| prefs.files.touch(&shown));
        }
    }

    /// Looks at the recent files on disk again when the list changed or a while has passed.
    // Agent: READS each recent file's entry on disk (not the file), at most every `LOOK_EVERY`.
    fn look_at_recent(&mut self) {
        let recent = &self.prefs.get().files.recent;
        let same = recent.len() == self.home.files.len() && recent.iter().zip(&self.home.files).all(|(path, file)| *path == file.path);
        if same && self.home.looked.is_some_and(|at| at.elapsed() < LOOK_EVERY) {
            return;
        }
        let own = std::env::home_dir();
        self.home.files = recent.iter().map(|path| Recent::of(path, own.as_deref())).collect();
        self.home.looked = Some(Instant::now());
    }
}

/// How full egui's picture of letters is.
#[expect(clippy::redundant_closure_for_method_calls, reason = "the method's path is not general over the font view's lifetime and doesn't compile")]
fn font_fill(ctx: &egui::Context) -> f32 {
    ctx.fonts(|fonts| fonts.font_atlas_fill_ratio())
}

/// The small picture of the design at `path` (or of the starter board), made here and now.
// Agent: runs on the `pencraft-thumb` thread: READS one design file and BLOCKS while it is laid out and drawn.
// It measures and lays out text through `ctx`, under egui's own locks, as the thread that builds the canvas's
// drawing does (`render::build_beside`).
fn made(ctx: &egui::Context, path: &str) -> Made {
    let faces = crate::fonts::changes();
    let before = font_fill(ctx);
    let image = if path == SAMPLE {
        pen_doc::read(&crate::samples::starter(&crate::faces::installed)).ok().and_then(|doc| picture(ctx, doc, None))
    } else {
        // Human: a bundle's pictures lie inside it and aren't unpacked for a small picture; its picture
        // layers show their placeholder, never a file that happens to lie beside the bundle.
        let beside = Some(Path::new(path)).filter(|path| !crate::bundle::is_bundle(path));
        design(Path::new(path)).and_then(|doc| picture(ctx, doc, beside))
    };
    // A face that is still waited for, or became ready while this was drawn: some of the text is in another.
    // Human: the window empties its picture of letters when that is nearly full. Should that happen while
    // this picture's text is drawn from it, letters may be missing; the picture is then made once more.
    let emptied = font_fill(ctx) < before;
    Made { image, rough: emptied || crate::fonts::pending() || crate::fonts::changes() != faces }
}

impl Home {
    /// Takes in the small picture that was being made, and has the next one made that is missing or out of date.
    // Human: reading a design, laying it out and drawing it takes a moment for a large one, and the home screen
    // stood still for it. Now a thread of its own does that, one picture at a time; the card shows its empty
    // ground until the picture is there, and the screen stays quick. A picture made while a face of its text
    // was still being read is made once more when the face is there.
    // Agent: MAIN-THREAD; the texture is handed to egui here. SPAWNS at most one thread at a time
    // (`pencraft-thumb`), which WAKES the UI when it is done; no file is read on this thread.
    fn tend_pictures(&mut self, ctx: &egui::Context) {
        let ready = !crate::fonts::pending();
        let Some((path, stamp)) =
            self.next_picture(ready, &mut |path, image| ctx.load_texture(format!("home {path}"), image, egui::TextureOptions::LINEAR))
        else {
            return;
        };
        let (send, arrives) = mpsc::channel();
        let (ctx, of) = (ctx.clone(), path.clone());
        let work = move || {
            // The home screen may be gone when the picture is done; then nobody takes it, which is fine.
            let _unheard = send.send(made(&ctx, &of));
            ctx.request_repaint();
        };
        match std::thread::Builder::new().name("pencraft-thumb".to_owned()).spawn(work) {
            Ok(_) => self.making = Some(Making { path, stamp, arrives }),
            Err(error) => {
                // Human: without a thread the card stays without a picture; it is not asked for again.
                log::warn!("no thread to make a design's small picture on: {error}");
                self.thumbs.insert(path, Thumb { stamp, texture: None, rough: false, tries: 0 });
            }
        }
    }

    /// Takes in the picture that was being made when it has arrived (`load` hands it to egui), and says which
    /// one to make next: the first that is missing or out of date. `None` while one is being made, and when
    /// none is needed. `ready` is whether every face asked for is there.
    // Human: one at a time, and each file once: while a picture is on its way no other is started, so
    // drawing the home screen again, in another frame or after coming back to it, starts nothing twice. A
    // file written while its picture was being made gets one more, of what it holds now.
    fn next_picture(
        &mut self,
        ready: bool,
        load: &mut dyn FnMut(&str, egui::ColorImage) -> egui::TextureHandle,
    ) -> Option<(String, Option<SystemTime>)> {
        self.thumbs.retain(|path, _| path == SAMPLE || self.files.iter().any(|file| file.path == *path));
        if let Some(making) = &self.making {
            let made = match making.arrives.try_recv() {
                Ok(made) => made,
                Err(mpsc::TryRecvError::Empty) => return None,
                // Its thread ended without a picture: the card stays without one.
                Err(mpsc::TryRecvError::Disconnected) => Made { image: None, rough: false },
            };
            let Making { path, stamp, .. } = self.making.take()?;
            let tries = self.thumbs.get(&path).filter(|old| old.stamp == stamp).map_or(0, |old| old.tries + 1);
            let texture = made.image.map(|image| load(&path, image));
            self.thumbs.insert(path, Thumb { stamp, texture, rough: made.rough, tries });
        }
        let listed = self.files.iter().filter(|file| file.changed.is_some()).map(|file| (file.path.as_str(), file.changed));
        let (path, stamp) = std::iter::once((SAMPLE, None)).chain(listed).find(|(path, stamp)| stale(self.thumbs.get(*path), *stamp, ready))?;
        Some((path.to_owned(), stamp))
    }

    /// The box a design is described in, and a row of ideas under it.
    // Human: Return sends, as in the chat; ⇧Return starts a new line. The words start a new document and go to
    // its assistant.
    fn describe(&mut self, ui: &mut egui::Ui, assistant: Option<&str>, ideas: bool, palette: Palette, act: &mut Option<Act>) {
        let id = egui::Id::unique("home-describe");
        let edge = if ui.memory(|memory| memory.has_focus(id)) { palette.muted } else { palette.outline };
        let frame = egui::Frame::new().fill(palette.island).stroke(Stroke::new(1.0, edge)).corner_radius(ISLAND_RADIUS).inner_margin(ISLAND_PADDING);
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            let font = FontId::proportional(HOME_PROMPT_TEXT);
            let field = egui::TextEdit::multiline(&mut self.prompt)
                .id(id)
                .frame(egui::Frame::new())
                .background_color(Color32::TRANSPARENT)
                .hint_text(RichText::new(HINT).font(font.clone()).color(palette.quiet))
                .font(font)
                .return_key(egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::Enter))
                .desired_rows(2)
                .desired_width(f32::INFINITY);
            let edit = ui.add(field);
            let ready = !self.prompt.trim().is_empty();
            let mut send =
                ready && edit.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter) && !input.modifiers.shift && !input.modifiers.alt);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if let Some(name) = assistant {
                    let (icon, _) = ui.allocate_exact_size(Vec2::splat(ICON_SIZE - 2.0), Sense::hover());
                    crate::icons::paint(ui.painter(), icon, "sparkles", palette.muted);
                    words(ui, name, FontId::proportional(HOME_SMALL_TEXT), palette.muted);
                } else if text_button(ui, Some("settings"), "Set up an assistant", vec2(160.0, HOME_SEND), false, true, palette).clicked() {
                    *act = Some(Act::Assistants);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (rect, button) = ui.allocate_exact_size(Vec2::splat(HOME_SEND), Sense::click());
                    let (fill, mark) = if ready { (palette.primary, palette.primary_text) } else { (palette.input, palette.muted) };
                    ui.painter().circle_filled(rect.center(), HOME_SEND / 2.0, fill);
                    crate::icons::paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(SEND_ARROW + 2.0)), "arrow-up", mark);
                    send |= button.on_hover_text(crate::keys::text("Start the design (↵)")).clicked() && ready;
                });
            });
            if send {
                *act = Some(Act::Describe(std::mem::take(&mut self.prompt).trim().to_owned()));
            }
        });
        if !ideas {
            return;
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = theme::BAR_IDEA_GAP;
            ui.spacing_mut().button_padding = theme::CHIP_PADDING;
            // Human: an idea that doesn't fit whole is shortened with "…"; none is started in less room than a
            // few words need, so the row never wraps.
            for idea in IDEAS {
                let room = ui.available_width();
                if room < 150.0 {
                    break;
                }
                let chip = egui::Button::new(RichText::new(idea).color(palette.muted))
                    .truncate()
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::new(1.0, palette.outline))
                    .corner_radius(CHIP_RADIUS)
                    .min_size(vec2(0.0, CHIP_HEIGHT));
                let clicked = ui.scope(|ui| {
                    ui.set_max_width(room);
                    ui.add(chip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                });
                if clicked.inner {
                    idea.clone_into(&mut self.prompt);
                    ui.memory_mut(|memory| memory.request_focus(id));
                }
            }
        });
    }

    /// The cards a design is started from, as many in a row as fit.
    fn starts(&self, ui: &mut egui::Ui, width: f32, palette: Palette, act: &mut Option<Act>) {
        let starts = [Start::Blank, Start::Screen(&SCREENS[0]), Start::Screen(&SCREENS[1]), Start::Screen(&SCREENS[2]), Start::Sample];
        let count = columns(width, HOME_START_WIDTH, HOME_CARD_GAP);
        let size = vec2(card_width(width, count, HOME_CARD_GAP), HOME_CARD_PADDING + HOME_START_PICTURE + HOME_CARD_LINES);
        let sample = self.thumbs.get(SAMPLE).and_then(|thumb| thumb.texture.as_ref());
        for (row, cards) in starts.chunks(usize::from(count)).enumerate() {
            if row > 0 {
                ui.add_space(HOME_CARD_GAP);
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = HOME_CARD_GAP;
                for start in cards {
                    let (rect, response) = card(ui, size, palette);
                    let inside = picture_place(ui, rect, HOME_START_PICTURE, palette);
                    match start {
                        Start::Blank => {
                            crate::icons::paint(ui.painter(), Rect::from_center_size(inside.center(), Vec2::splat(18.0)), "plus", palette.muted);
                        }
                        Start::Screen(screen) => {
                            // Human: the frame the design starts with, in its own shape: tall for a phone, wide
                            // for a desktop window.
                            let sheet = fitted(inside, vec2(screen.w, screen.h), f32::INFINITY);
                            ui.painter().rect(sheet, 3.0, palette.island, Stroke::new(1.0, palette.muted), StrokeKind::Inside);
                        }
                        Start::Sample => {
                            if let Some(texture) = sample {
                                paint_picture(ui, inside, texture);
                            }
                        }
                    }
                    let (title, note) = start.lines();
                    card_lines(ui, rect, title, &note, palette.foreground, palette);
                    if response.clicked() {
                        *act = Some(Act::Start(*start));
                    }
                }
            });
        }
    }

    /// The recent files, each with its small picture, or a note while there are none.
    fn recent(&self, ui: &mut egui::Ui, width: f32, now: SystemTime, palette: Palette, act: &mut Option<Act>) {
        if self.files.is_empty() {
            let (rect, _) = ui.allocate_exact_size(vec2(width, HOME_EMPTY_HEIGHT), Sense::hover());
            ui.painter().rect_stroke(rect, HOME_CARD_RADIUS, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
            let middle = rect.center();
            crate::icons::paint(ui.painter(), Rect::from_center_size(middle - vec2(0.0, 26.0), Vec2::splat(18.0)), "folder-open", palette.muted);
            let font = FontId::proportional(HOME_CARD_TEXT);
            ui.painter().text(middle + vec2(0.0, 2.0), Align2::CENTER_CENTER, "Designs you open or save show up here", font, palette.foreground);
            let small = FontId::proportional(HOME_SMALL_TEXT);
            ui.painter().text(
                middle + vec2(0.0, 22.0),
                Align2::CENTER_CENTER,
                "Drop a .quill or .pen file on the window to open it",
                small,
                palette.muted,
            );
            return;
        }
        let count = columns(width, HOME_RECENT_WIDTH, HOME_CARD_GAP);
        let across = card_width(width, count, HOME_CARD_GAP);
        let tall = ((across - 2.0 * HOME_CARD_PADDING) * HOME_RECENT_SHAPE).round();
        for (row, files) in self.files.chunks(usize::from(count)).enumerate() {
            if row > 0 {
                ui.add_space(HOME_CARD_GAP);
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = HOME_CARD_GAP;
                for file in files {
                    let (rect, response) = card(ui, vec2(across, HOME_CARD_PADDING + tall + HOME_CARD_LINES), palette);
                    let inside = picture_place(ui, rect, tall, palette);
                    let texture = self.thumbs.get(&file.path).and_then(|thumb| thumb.texture.as_ref());
                    let mark = Rect::from_center_size(inside.center(), Vec2::splat(18.0));
                    let (note, ink) = if let Some(changed) = file.changed {
                        match texture {
                            Some(texture) => paint_picture(ui, inside, texture),
                            None => crate::icons::paint(ui.painter(), mark, "frame", palette.muted),
                        }
                        (format!("{} · {}", ago(now.duration_since(changed).unwrap_or_default()), file.folder), palette.foreground)
                    } else {
                        crate::icons::paint(ui.painter(), mark, "circle-question-mark", palette.muted);
                        (format!("Not found · {}", file.folder), palette.muted)
                    };
                    card_lines(ui, rect, &file.name, &note, ink, palette);
                    response.context_menu(|ui| {
                        if ui.button("Remove from Recent").clicked() {
                            *act = Some(Act::Forget(file.path.clone()));
                            ui.close();
                        }
                    });
                    if response.on_hover_text(&file.path).clicked() {
                        *act = Some(Act::Recent(file.path.clone()));
                    }
                }
            });
        }
    }
}

/// How wide the home screen's column is in a window with `room` across.
fn column(room: f32) -> f32 {
    (room - 2.0 * HOME_MARGIN).clamp(0.0, HOME_WIDTH)
}

/// How many cards at least `least` wide, `gap` apart, fit in a row `width` wide: one at least.
fn columns(width: f32, least: f32, gap: f32) -> u8 {
    let mut count = 1;
    while count < MOST_COLUMNS && f32::from(count + 1) * least + f32::from(count) * gap <= width {
        count += 1;
    }
    count
}

/// The width of each of `count` cards that share a row `width` wide, `gap` apart.
fn card_width(width: f32, count: u8, gap: f32) -> f32 {
    ((width - f32::from(count.saturating_sub(1)) * gap) / f32::from(count.max(1))).floor()
}

/// How long ago, in words, something `since` back happened.
fn ago(since: Duration) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    let many = |count: u64, unit: &str| if count == 1 { format!("1 {unit} ago") } else { format!("{count} {unit}s ago") };
    let seconds = since.as_secs();
    if seconds < MINUTE {
        "Just now".to_owned()
    } else if seconds < HOUR {
        format!("{} min ago", seconds / MINUTE)
    } else if seconds < DAY {
        many(seconds / HOUR, "hour")
    } else if seconds < 2 * DAY {
        "Yesterday".to_owned()
    } else if seconds < 14 * DAY {
        many(seconds / DAY, "day")
    } else if seconds < 60 * DAY {
        many(seconds / (7 * DAY), "week")
    } else if seconds < 730 * DAY {
        many(seconds / (30 * DAY), "month")
    } else {
        many(seconds / (365 * DAY), "year")
    }
}

/// `folder` as the home screen writes it: the user's own folder `own` becomes `~`.
fn shortened(folder: &Path, own: Option<&Path>) -> String {
    match own.and_then(|own| folder.strip_prefix(own).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~{}{}", std::path::MAIN_SEPARATOR, rest.display()),
        None => folder.display().to_string(),
    }
}

/// Whether a small picture has to be made: there is none, its file was written since (`stamp`), or it was made
/// before the faces of its text were `ready` and they are now.
fn stale(thumb: Option<&Thumb>, stamp: Option<SystemTime>, ready: bool) -> bool {
    thumb.is_none_or(|thumb| thumb.stamp != stamp || (thumb.rough && ready && thumb.tries < MOST_TRIES))
}

/// The design in the file at `path`; `None` when it can't be read or is too large for a small picture.
fn design(path: &Path) -> Option<pen_doc::Document> {
    if crate::bundle::is_bundle(path) {
        return crate::bundle::peek(path);
    }
    if std::fs::metadata(path).ok()?.len() > MOST_BYTES {
        return None;
    }
    pen_doc::read(&std::fs::read_to_string(path).ok()?).ok()
}

/// A small picture of everything in `doc`, which is saved at `path`; `None` when it is empty.
// Human: the whole canvas, as zoom-to-fit would show it, at the size the largest card needs on this screen.
// Agent: runs on the `pencraft-thumb` thread (see `made`); lays the document out and draws it, as an export does.
fn picture(ctx: &egui::Context, doc: pen_doc::Document, path: Option<&Path>) -> Option<egui::ColorImage> {
    let mut editor = Editor::new(doc, path.map(Path::to_path_buf), 0);
    editor.refresh(&super::text_measure(ctx));
    let bounds = editor.content_bounds().filter(|bounds| bounds.w > 0.0 && bounds.h > 0.0)?;
    let scale = (HOME_THUMB_SIZE[0] / bounds.w).min(HOME_THUMB_SIZE[1] / bounds.h).min(HOME_THUMB_SCALE) * ctx.pixels_per_point();
    let layers: Vec<&pen_doc::scene::SceneNode> = editor.scene.iter().collect();
    let drawn =
        crate::render::DisplayList::default().snapshot(&layers, scale, pen_doc::vars::Vars::new(&editor.doc), ctx, path.and_then(Path::parent))?;
    // A picture whose bytes don't match its size would not be shown as one.
    (drawn.rgba.len() == drawn.w * drawn.h * 4).then(|| egui::ColorImage::from_rgba_premultiplied([drawn.w, drawn.h], &drawn.rgba))
}

/// Text at the layout's next place, not selectable and never wrapped.
fn words(ui: &mut egui::Ui, text: &str, font: FontId, color: Color32) {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, color);
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, color);
}

/// A card's place and ground: brighter under the pointer, outlined in the accent while it has the keyboard.
fn card(ui: &mut egui::Ui, size: Vec2, palette: Palette) -> (Rect, Response) {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if response.hovered() { palette.hover.gamma_multiply(0.6) } else { palette.island };
    let edge = if response.has_focus() { ACCENT } else { palette.outline };
    ui.painter().rect(rect, HOME_CARD_RADIUS, fill, Stroke::new(1.0, edge), StrokeKind::Inside);
    (rect, response.on_hover_cursor(egui::CursorIcon::PointingHand))
}

/// Paints the ground of a card's picture, `tall` high, and returns the room for what is drawn on it.
fn picture_place(ui: &egui::Ui, card: Rect, tall: f32, palette: Palette) -> Rect {
    let place = Rect::from_min_size(card.min + Vec2::splat(HOME_CARD_PADDING), vec2(card.width() - 2.0 * HOME_CARD_PADDING, tall));
    ui.painter().rect_filled(place, HOME_CARD_RADIUS - HOME_CARD_PADDING / 2.0, palette.canvas);
    place.shrink(HOME_PICTURE_INSET)
}

/// The two lines at the foot of a card: its name in `ink`, and a quieter note. Each is cut short with "…".
fn card_lines(ui: &egui::Ui, card: Rect, title: &str, note: &str, ink: Color32, palette: Palette) {
    let left = card.left() + HOME_CARD_PADDING + 4.0;
    let width = card.right() - HOME_CARD_PADDING - 4.0 - left;
    let top = card.bottom() - HOME_CARD_LINES;
    for (text, size, color, down) in [(title, HOME_CARD_TEXT, ink, 10.0), (note, HOME_SMALL_TEXT, palette.muted, 29.0)] {
        let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), FontId::proportional(size), color);
        job.wrap = egui::text::TextWrapping::truncate_at_width(width);
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
        ui.painter().galley(pos2(left, top + down), galley, color);
    }
}

/// The largest box of the shape `size` that fits in the middle of `area`, no more than `most` times `size`.
fn fitted(area: Rect, size: Vec2, most: f32) -> Rect {
    if size.x <= 0.0 || size.y <= 0.0 {
        return area;
    }
    Rect::from_center_size(area.center(), size * (area.width() / size.x).min(area.height() / size.y).min(most))
}

/// Paints a small picture whole in the middle of `area`, never larger than it was made.
fn paint_picture(ui: &egui::Ui, area: Rect, texture: &egui::TextureHandle) {
    let place = fitted(area, texture.size_vec2(), 1.0 / ui.ctx().pixels_per_point());
    ui.painter().image(texture.id(), place, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
}

/// The home screen's last line: the shortcuts worth knowing here, and the app's version at the other end.
fn last_line(ui: &mut egui::Ui, palette: Palette) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(18.0, 8.0);
        for (label, keys) in [("New design", "⇧⌘N"), ("Open", "⌘O"), ("Search actions", "⌘P"), ("Settings", "⌘,")] {
            let font = FontId::proportional(HOME_SMALL_TEXT);
            let label = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), palette.muted);
            let keys = ui.painter().layout_no_wrap(crate::keys::text(keys).into_owned(), font, palette.muted);
            let key = vec2(keys.size().x + 12.0, HOME_KEY_HEIGHT);
            let (rect, _) = ui.allocate_exact_size(vec2(label.size().x + 7.0 + key.x, HOME_KEY_HEIGHT), Sense::hover());
            ui.painter().galley(pos2(rect.left(), rect.center().y - label.size().y / 2.0), label, palette.muted);
            let cap = Rect::from_min_size(pos2(rect.right() - key.x, rect.top()), key);
            ui.painter().rect_stroke(cap, 5.0, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
            ui.painter().galley(cap.center() - keys.size() / 2.0, keys, palette.muted);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let version = concat!("Quill ", env!("CARGO_PKG_VERSION"), " · formerly Pencraft");
            words(ui, version, FontId::proportional(HOME_SMALL_TEXT), palette.quiet);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The icons the home screen draws.
    const ICONS: [&str; 7] = ["plus", "folder-open", "sparkles", "arrow-up", "circle-question-mark", "frame", "settings"];

    #[test]
    fn a_row_holds_as_many_cards_as_fit_and_one_at_least() {
        assert_eq!(columns(920.0, 150.0, 12.0), 5);
        assert_eq!(columns(920.0, 200.0, 12.0), 4);
        assert_eq!(columns(411.0, 200.0, 12.0), 1);
        assert_eq!(columns(412.0, 200.0, 12.0), 2);
        assert_eq!(columns(0.0, 200.0, 12.0), 1);
        assert_eq!(columns(9000.0, 150.0, 12.0), MOST_COLUMNS);
        // The cards of a row, with the gaps between them, never reach past it.
        for width in [300.0, 504.0, 920.0] {
            let count = columns(width, 200.0, 12.0);
            let card = card_width(width, count, 12.0);
            assert!(f32::from(count) * card + f32::from(count - 1) * 12.0 <= width);
        }
    }

    #[test]
    fn the_column_keeps_a_margin_and_stops_growing() {
        assert!((column(560.0) - (560.0 - 2.0 * HOME_MARGIN)).abs() < f32::EPSILON);
        assert!((column(3000.0) - HOME_WIDTH).abs() < f32::EPSILON);
        assert!(column(10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn how_long_ago_is_said_in_words() {
        let said = |seconds: u64| ago(Duration::from_secs(seconds));
        assert_eq!(said(5), "Just now");
        assert_eq!(said(60 * 12), "12 min ago");
        assert_eq!(said(3600), "1 hour ago");
        assert_eq!(said(3600 * 5), "5 hours ago");
        assert_eq!(said(86_400 + 60), "Yesterday");
        assert_eq!(said(86_400 * 3), "3 days ago");
        assert_eq!(said(86_400 * 21), "3 weeks ago");
        assert_eq!(said(86_400 * 90), "3 months ago");
        assert_eq!(said(86_400 * 800), "2 years ago");
    }

    #[test]
    fn the_users_own_folder_is_written_short() {
        let own = Path::new("own");
        assert_eq!(shortened(own, Some(own)), "~");
        assert_eq!(shortened(&own.join("designs"), Some(own)), format!("~{}designs", std::path::MAIN_SEPARATOR));
        assert_eq!(shortened(Path::new("elsewhere"), Some(own)), "elsewhere");
        assert_eq!(shortened(Path::new("elsewhere"), None), "elsewhere");
    }

    #[test]
    fn a_picture_is_made_when_it_is_missing_out_of_date_or_was_made_too_early() {
        let then = SystemTime::UNIX_EPOCH;
        let later = then + Duration::from_secs(1);
        let thumb = |rough: bool, tries: u8| Thumb { stamp: Some(then), texture: None, rough, tries };
        assert!(stale(None, Some(then), true));
        assert!(!stale(Some(&thumb(false, 0)), Some(then), true));
        assert!(stale(Some(&thumb(false, 0)), Some(later), true));
        // Made while a face was being read: once more when it is there, and not for ever.
        assert!(!stale(Some(&thumb(true, 0)), Some(then), false));
        assert!(stale(Some(&thumb(true, 0)), Some(then), true));
        assert!(!stale(Some(&thumb(true, MOST_TRIES)), Some(then), true));
    }

    fn recent(path: &str, changed: SystemTime) -> Recent {
        Recent { path: path.to_owned(), name: path.to_owned(), folder: String::new(), changed: Some(changed) }
    }

    #[test]
    fn one_picture_is_made_at_a_time_each_file_once_and_again_when_the_file_was_written() {
        let ctx = egui::Context::default();
        let loaded = std::cell::Cell::new(0);
        let mut load = |path: &str, image: egui::ColorImage| {
            loaded.set(loaded.get() + 1);
            ctx.load_texture(path, image, egui::TextureOptions::LINEAR)
        };
        let then = SystemTime::UNIX_EPOCH;
        let at = |seconds: u64| then + Duration::from_secs(seconds);
        let mut home = Home { files: vec![recent("a.quill", at(1)), recent("b.quill", at(1))], ..Home::default() };
        home.thumbs.insert(SAMPLE.to_owned(), Thumb { stamp: None, texture: None, rough: false, tries: 0 });
        // The first file without a picture is next; while it is being made, nothing else is started, however
        // often the home screen is drawn.
        assert_eq!(home.next_picture(true, &mut load), Some(("a.quill".to_owned(), Some(at(1)))));
        let (send, arrives) = mpsc::channel();
        home.making = Some(Making { path: "a.quill".to_owned(), stamp: Some(at(1)), arrives });
        for _ in 0..3 {
            assert_eq!(home.next_picture(true, &mut load), None);
        }
        // The file is written while its picture is on its way: the picture that arrives is of the old file.
        home.files[0].changed = Some(at(2));
        send.send(Made { image: Some(egui::ColorImage::filled([2, 2], Color32::WHITE)), rough: false }).unwrap();
        // It is taken in (handed to egui once, here), and the next one starts: the same file, as it is now.
        assert_eq!(home.next_picture(true, &mut load), Some(("a.quill".to_owned(), Some(at(2)))));
        assert!(home.making.is_none() && loaded.get() == 1);
        assert!(home.thumbs["a.quill"].stamp == Some(at(1)) && home.thumbs["a.quill"].texture.is_some());
        // A thread that ends without a picture leaves the card without one, and is not started again.
        let (send, arrives) = mpsc::channel::<Made>();
        home.making = Some(Making { path: "a.quill".to_owned(), stamp: Some(at(2)), arrives });
        drop(send);
        assert_eq!(home.next_picture(true, &mut load), Some(("b.quill".to_owned(), Some(at(1)))));
        assert!(home.thumbs["a.quill"].stamp == Some(at(2)) && home.thumbs["a.quill"].texture.is_none());
        // Everything has its picture: nothing more is made.
        home.thumbs.insert("b.quill".to_owned(), Thumb { stamp: Some(at(1)), texture: None, rough: false, tries: 0 });
        assert_eq!(home.next_picture(true, &mut load), None);
        assert_eq!(loaded.get(), 1);
        // A picture made too early is made once more, and one of a file that left the list is dropped.
        home.thumbs.get_mut("b.quill").unwrap().rough = true;
        assert_eq!(home.next_picture(true, &mut load), Some(("b.quill".to_owned(), Some(at(1)))));
        home.files.remove(0);
        home.next_picture(false, &mut load);
        assert!(!home.thumbs.contains_key("a.quill"));
    }

    /// How long the window stands still for a large design's small picture. Run by hand:
    /// `cargo test --release -- --ignored timing_35 --nocapture`.
    #[test]
    #[ignore = "timing: prints frame times"]
    fn timing_35_a_large_designs_small_picture() {
        let folder = std::env::temp_dir().join(format!("quill-thumb-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let file = folder.join("large.pen");
        std::fs::write(&file, crate::timing::large_design()).unwrap();
        let path = file.display().to_string();
        let ctx = egui::Context::default();
        let frame = |ctx: &egui::Context, draw: &mut dyn FnMut()| {
            let start = Instant::now();
            let mut drawn = ctx.run_ui(egui::RawInput::default(), |_| draw());
            drawn.textures_delta.clear();
            start.elapsed()
        };
        // As it was: read, laid out and drawn in the frame, on the window's thread.
        let mut here = None;
        let before = frame(&ctx, &mut || here = made(&ctx, &path).image);
        assert!(here.is_some());
        println!("35 before: the frame that makes the picture stands still for {:.1} ms", before.as_secs_f64() * 1000.0);
        // As it is: frames go on while a thread makes it.
        let mut home = Home { files: vec![recent(&path, SystemTime::UNIX_EPOCH)], ..Home::default() };
        home.thumbs.insert(SAMPLE.to_owned(), Thumb { stamp: None, texture: None, rough: false, tries: 0 });
        let (began, mut longest, mut frames) = (Instant::now(), Duration::ZERO, 0);
        while home.thumbs.get(&path).is_none_or(|thumb| thumb.texture.is_none()) {
            longest = longest.max(frame(&ctx, &mut || home.tend_pictures(&ctx)));
            frames += 1;
            std::thread::sleep(Duration::from_millis(2));
            assert!(began.elapsed() < Duration::from_secs(60), "the picture never arrived");
        }
        println!(
            "35 after: {frames} frames went on meanwhile, the longest {:.3} ms; the picture was there after {:.1} ms",
            longest.as_secs_f64() * 1000.0,
            began.elapsed().as_secs_f64() * 1000.0
        );
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn every_icon_of_the_home_screen_is_bundled() {
        for icon in ICONS {
            assert!(crate::icons::outline("lucide", icon).is_some(), "{icon}");
        }
    }

    #[test]
    fn a_screen_starts_with_its_name_and_size() {
        assert_eq!(Start::Screen(&SCREENS[0]).lines(), ("Phone", "390 × 844".to_owned()));
        assert_eq!(Start::Blank.lines().0, "Blank canvas");
    }
}
