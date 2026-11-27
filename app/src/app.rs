//! The window: tab strip, home screen, editor layout, file commands, keyboard shortcuts and quitting.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use craft_dialogs::{Answer, Dialogs, FileDialog};
use craft_settings::{Location, SaveStatus, SettingsStore};
use egui::{Color32, Key, Modifiers, RichText};
use pen_doc::Node;

use crate::canvas::{self, Canvas};
use crate::editor::{Editor, MAX_ZOOM, Tool};
use crate::ops::{AlignTo, Order};
use crate::palette::Command;
use crate::prefs::Prefs;
use crate::settings::Settings;
use crate::sidebar::Ask;
use crate::sidebar::{Sidebar, icon_button, text_button};
use crate::theme::{
    self, AGENT_CURSOR_LARGE, AGENT_CURSOR_MEDIUM, CAPTION_BUTTON, CAPTION_SQUARE, ICON_SIZE, ISLAND_GAP, ISLAND_RADIUS, MESSAGE_BAR_HEIGHT,
    PROPERTIES_WIDTH, RAIL_WIDTH, TAB_BAR_HEIGHT, TAB_HEIGHT, TAB_ICON, TAB_PADDING, TAB_WORK_DOT, TITLE_BUTTON, TITLE_DIVIDER, TITLE_GAP,
    TITLE_PADDING, TITLE_SIZE, TITLE_TEXT, TRAFFIC_LIGHT_LEFT, TRAFFIC_LIGHT_SPACE, ZOOM_ISLAND,
};
use crate::{bundle, fonts, samples, screenshot};

mod home;
mod pictures;
mod pour;
mod talk;
mod tools;

/// What to do with a file dialog's answer.
enum Then {
    /// Open the chosen files.
    Open,
    /// Save tab `tab` under the chosen name; close it afterwards when `close` (part of closing or quitting).
    SaveAs { tab: usize, close: bool },
    /// Write the document with this id ([`Editor::id`]) as a `.pen` file under the chosen name, if it is still
    /// open.
    ExportPen(u64),
    /// Add the chosen pictures and SVG files to the document.
    Import,
    /// Attach the chosen files to the chat message being written.
    Attach,
    /// Export the selection, as these options say, to the chosen file or into the chosen folder.
    Export(crate::export::Options),
    /// Add the chosen pictures as references to the row of the Generate Image dialog opened for the document
    /// with this id ([`Editor::id`]), if it is still open for it.
    Reference(u64),
}

/// An unsaved-changes question waiting for an answer.
#[derive(Clone, Copy)]
struct ClosePrompt {
    tab: usize,
    quitting: bool,
}

/// Clears away the folders that bundles open in an earlier run lay unpacked in, off this thread; those a running
/// Quill holds stay.
fn sweep_working_folders() {
    if let Err(error) = std::thread::Builder::new().name("working-folders".to_owned()).spawn(bundle::sweep) {
        log::warn!("working folders: {error}");
    }
}

/// Failures that last until something works again, each said once (see [`crate::toast::Lasting`]).
#[derive(Default)]
struct Troubles {
    /// Saving the settings.
    settings: crate::toast::Lasting,
    /// Keeping unsaved work for recovery.
    recovery: crate::toast::Lasting,
}

/// Crash recovery, as the app holds it.
#[derive(Default)]
struct Recovery {
    /// Keeps the documents with unsaved changes on disk in case this run doesn't end properly; `None` in a
    /// private run or a picture's, and when its folder couldn't be made.
    keeper: Option<crate::recover::Keeper>,
    /// The documents an earlier run that didn't end properly left behind, until the user says what to do with them.
    left: Vec<crate::recover::Left>,
    /// The user chose Discard and is being asked whether they mean it; nothing is cleared until they do.
    discarding: bool,
}

impl Recovery {
    /// Finds what earlier runs left behind and starts keeping this run's documents; nothing of either in a
    /// `private` run.
    // Human: what a crashed run left is looked for before this run makes a folder of its own.
    fn start(private: bool) -> Recovery {
        if private {
            return Recovery::default();
        }
        let left = crate::recover::left();
        let keeper = crate::recover::Keeper::start().map_err(|problem| log::warn!("{problem}")).ok();
        Recovery { keeper, left, discarding: false }
    }
}

/// The app's name in the menu bar ("Quit Quill").
const APP_NAME: &str = "Quill";

/// How the app was started.
#[expect(clippy::struct_excessive_bools, reason = "each is one independent switch of the command line")]
pub struct Launch {
    /// `.pen` files named on the command line.
    pub files: Vec<PathBuf>,
    /// `--screenshot <file.png>`: draw, save a picture of the window, quit.
    pub screenshot: Option<PathBuf>,
    /// `--dark`: the screenshot shows the dark appearance.
    pub dark: bool,
    /// `--sidebar <view>`: the sidebar view the screenshot shows, or `hidden` for the folded sidebar.
    pub sidebar: Option<String>,
    /// `--settings <page>`: the screenshot shows the settings window on `general`, `agents`, `provider`, `images` or
    /// `image-service`.
    pub settings: Option<String>,
    /// `--select <id>[,<id>…]`: the layers selected in the picture, to show their properties.
    pub select: Option<String>,
    /// `--actions`: the picture shows the action search open.
    pub actions: bool,
    /// `--generate-image`: the picture shows the "Generate Image…" dialog open for the selection.
    pub generate_image: bool,
    /// `--export <file>`: with `--screenshot`, also exports the selected layer (or every top-level layer) to
    /// this file, in the format its ending names.
    pub export: Option<PathBuf>,
    /// `--activity`: the picture shows an AI model at work on the selected layers.
    pub activity: bool,
    /// `--attach <file>` (repeatable): the picture shows these files attached to the chat message; with `--ask`
    /// they are sent with it.
    pub attach: Vec<PathBuf>,
    /// `--find <text>`: the picture shows find layers open with this text searched for.
    pub find: Option<String>,
    /// `--points`: the picture shows the selected path with its points being edited, all of them selected.
    pub points: bool,
    /// `--ask <message>`: sends this message to the chat's assistant once the app is up.
    pub ask: Option<String>,
    /// `--assistant <id>`: the assistant the chat talks to in this run (`code/claude-haiku-5-5`, `api/anthropic/claude-opus-5-5`, ...).
    pub assistant: Option<String>,
    /// `--private`: the settings are neither read nor saved, and no tabs are restored or remembered.
    pub private: bool,
    /// `--background`: the window opens behind what the user is doing and doesn't take the keyboard (how an AI
    /// client starts the app).
    pub background: bool,
    /// `--no-update-check`: this run doesn't ask GitHub for a newer version.
    pub no_update_check: bool,
    /// `--update-prompt`: the picture shows the card that offers a newer version.
    pub update_prompt: bool,
    /// `--update-required`: the picture shows the card of a made-up update that is required.
    pub update_required: bool,
    /// `--update-collapsed`: the picture shows the card folded away into its pill.
    pub update_collapsed: bool,
    /// `--update-download [PERCENT]`: the card shows a made-up download that has come this far. In a picture
    /// it stands still; in an ordinary run it goes on by itself, to watch the bar move. Nothing is fetched.
    pub update_download: Option<u8>,
    /// `--home`: the picture shows the home screen, with the files named on the command line as its recent files.
    pub home: bool,
    /// `--welcome [SECONDS]`: the welcome film plays although it has been seen; a picture shows it standing
    /// still at this moment.
    pub welcome: Option<f32>,
}

impl Launch {
    /// Whether this run may ask GitHub for a newer version and offer it.
    // Human: a picture, a private run and one an AI client started in the background stay offline and show
    // nothing nobody asked for. `pencraft mcp` never gets this far.
    #[must_use]
    pub fn looks_for_updates(&self) -> bool {
        // A made-up download keeps the card to itself: a real answer from GitHub would take its place.
        self.screenshot.is_none() && !self.private && !self.background && !self.no_update_check && self.update_download.is_none()
    }
}

/// The whole app.
pub struct App {
    prefs: SettingsStore<Prefs>,
    tabs: Vec<Editor>,
    /// What is kept in case this run doesn't end properly, and what an earlier run left behind.
    recovery: Recovery,
    /// The tab in front; `None` shows the home screen.
    active: Option<usize>,
    canvas: Canvas,
    /// The action search (⌘P).
    actions: crate::palette::Palette,
    find: crate::find::Find,
    /// An export waiting to be made: where to, and what. Made in the next frame, where drawing is possible.
    exporting: Option<(PathBuf, crate::export::Options)>,
    /// The menu bar at the top of the screen, where the system has one (macOS).
    native_menu: Option<craft_menu::NativeMenu>,
    sidebar: Sidebar,
    settings: Settings,
    dialogs: Dialogs<Then>,
    clipboard: Vec<Node>,
    hide_ui: bool,
    prompt: Option<ClosePrompt>,
    discard_confirmed: bool,
    toasts: crate::toast::Toasts,
    /// Lasting failures the user has been told of once.
    troubles: Troubles,
    screenshot: Option<screenshot::Job>,
    seed: u64,
    canvas_view: egui::Rect,
    /// The local server AI models reach the app through; `None` when it couldn't start, or in a screenshot run.
    link: Option<crate::link::Server>,
    /// Tool calls come in here, from the local server and from the chat's own assistant; `tool_send` is the
    /// end they are sent into.
    tool_calls: std::sync::mpsc::Receiver<crate::link::Call>,
    tool_send: std::sync::mpsc::Sender<crate::link::Call>,
    /// The chats with an AI model: one for each open document that has been talked in, and the home screen's.
    talks: Vec<talk::Talk>,
    /// How many chats this run has started; the next one's number follows from it.
    talks_made: u64,
    /// The number of the chat the sidebar shows, whose message is being written there; 0 before it has shown one.
    shown: u64,
    /// The assistants the chat can talk to, looked up when the app starts and when the assistant menu opens.
    assistants: Vec<crate::chat::Choice>,
    /// The assistant asked for on the command line, which goes before the one in the settings.
    assistant_asked: Option<String>,
    /// The chat the user asked to start over while it had something in it, until they confirm or cancel.
    new_chat_asked: Option<u64>,
    /// Nothing of this run is remembered (`--private`, or a picture).
    private: bool,
    /// Tool calls that wait for the user to finish a drag, each with when it came in.
    waiting: Vec<(crate::link::Call, Instant)>,
    /// `generate_image` calls whose picture is still being made.
    picturing: Vec<pictures::Waiting>,
    /// The "Generate Image…" dialog and the picture it is having made.
    generating: pictures::Generating,
    /// The top-level layers models have changed since a reviewer last looked, each with its document's id and
    /// how many layers were written in it.
    pub(crate) designed: Vec<(u64, String, usize)>,
    /// What is known about a newer version of the app, and the card that offers it.
    updates: crate::update::Updates,
    /// A downloaded update to install when the app quits.
    install: Option<crate::update::Prepared>,
    /// What the home screen keeps: the words being written there, and the recent files' small pictures.
    home: home::Home,
    /// The price list of the providers' models, which a fetch on another thread may replace.
    prices: Arc<std::sync::Mutex<crate::chat::prices::Catalogue>>,
    /// The welcome film, while it shows in the place of everything else.
    film: Option<crate::splash::Splash>,
    /// The question to the computer whether it asks for less motion, asked once at the start.
    motion: crate::motion::System,
    /// The app's icon as the title bar shows it before the name; `None` when it couldn't be read.
    mark: Option<egui::TextureHandle>,
    /// The tab that is pouring out of the "new file" button and the one draining away, while they do.
    pours: pour::Pours,
}

/// The app's icon, made small for the title bar.
// Human: the icon file is 256 points a side and the bar shows it at 20. It is made 80 a side here, sharp on a
// screen that draws up to four pixels a point, since shrinking it while it is drawn would leave it ragged.
fn title_mark(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let picture = image::load_from_memory(crate::ICON).ok()?.resize_exact(80, 80, image::imageops::FilterType::Lanczos3).to_rgba8();
    let pixels = egui::ColorImage::from_rgba_unmultiplied([80, 80], picture.as_raw());
    Some(ctx.load_texture("title mark", pixels, egui::TextureOptions::LINEAR))
}

/// The interface's bold face at `size`.
fn bold(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name(crate::render::BOLD_FAMILY.into()))
}

/// A button of the title bar that is only an icon, drawn in `tint`; it is filled while `active` or under the
/// pointer.
fn bar_button(ui: &mut egui::Ui, icon: &'static str, active: bool, tint: Color32, palette: theme::Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(TITLE_BUTTON), egui::Sense::click());
    if active || response.hovered() {
        ui.painter().rect_filled(
            rect,
            f32::from(theme::CONTROL_RADIUS),
            if active { palette.field } else { palette.field.gamma_multiply(theme::OUTLINE_SOFT) },
        );
    }
    crate::icons::paint(ui.painter(), egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(ICON_SIZE)), icon, tint);
    response
}

/// A dialog's main button, in the primary colour.
pub(crate) fn dialog_primary(text: &str, palette: theme::Palette) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).color(palette.primary_text))
        .fill(palette.primary)
        .corner_radius(theme::CHIP_RADIUS)
        .min_size(egui::vec2(theme::DIALOG_BUTTON, theme::FIELD_HEIGHT))
}

/// A dialog's lesser button, the same size as its main one.
pub(crate) fn dialog_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(text).corner_radius(theme::CHIP_RADIUS).min_size(egui::vec2(theme::DIALOG_BUTTON, theme::FIELD_HEIGHT))
}

/// The window's own three buttons at the end of the title bar, where Quill draws it itself (Windows): close,
/// maximise or restore, minimise, and a line between them and what comes before.
// Human: laid out right to left, so close comes first and ends up nearest the window's corner. Close goes the
// same way as the system's own button did, so unsaved documents are asked about.
fn window_buttons(ui: &mut egui::Ui, palette: theme::Palette) {
    let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
    ui.spacing_mut().item_spacing.x = 0.0;
    let ctx = ui.ctx().clone();
    let button = |ui: &mut egui::Ui, icon: &'static str, side: f32, danger: bool, hint: &str| {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(CAPTION_BUTTON, TITLE_BUTTON), egui::Sense::click());
        let (fill, mark) = match (crate::sidebar::lit(&response), danger) {
            (true, true) => (theme::PROBLEM, theme::ON_PROBLEM),
            (true, false) => (palette.field, palette.strong),
            (false, _) => (Color32::TRANSPARENT, palette.strong),
        };
        let radius = f32::from(theme::CONTROL_RADIUS);
        ui.painter().rect_filled(rect, radius, fill);
        crate::sidebar::focus_ring(ui, &response, rect, radius, palette);
        crate::icons::paint(ui.painter(), egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(side)), icon, mark);
        crate::sidebar::name_button(&response, hint);
        response.on_hover_text(hint).clicked()
    };
    if button(ui, "x", ICON_SIZE, true, "Close") {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    let (icon, hint) = if maximized { ("copy", "Restore") } else { ("square", "Maximise") };
    if button(ui, icon, CAPTION_SQUARE, false, hint) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
    if button(ui, "minus", ICON_SIZE, false, "Minimise") {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
    ui.spacing_mut().item_spacing.x = TITLE_GAP;
    let (line, _) = ui.allocate_exact_size(egui::vec2(1.0, TITLE_DIVIDER), egui::Sense::hover());
    ui.painter().rect_filled(line, 0.0, palette.line);
}

/// Which way a press at `at` resizes a window that covers `window`: `None` away from its edges.
// Human: within `WINDOW_EDGE` of an edge the window is resized that way; within `WINDOW_CORNER` of a corner
// along that edge, both ways at once.
fn edge_at(window: egui::Rect, at: egui::Pos2) -> Option<egui::ResizeDirection> {
    use egui::ResizeDirection as To;
    if !window.contains(at) {
        return None;
    }
    let (left, right) = (at.x - window.left(), window.right() - at.x);
    let (top, bottom) = (at.y - window.top(), window.bottom() - at.y);
    let near = left.min(right).min(top).min(bottom) < theme::WINDOW_EDGE;
    let reach = |edge: f32| edge < theme::WINDOW_CORNER;
    match (near, reach(top), reach(bottom), reach(left), reach(right)) {
        (false, ..) => None,
        (_, true, _, true, _) => Some(To::NorthWest),
        (_, true, _, _, true) => Some(To::NorthEast),
        (_, _, true, true, _) => Some(To::SouthWest),
        (_, _, true, _, true) => Some(To::SouthEast),
        _ if top < theme::WINDOW_EDGE => Some(To::North),
        _ if bottom < theme::WINDOW_EDGE => Some(To::South),
        _ if left < theme::WINDOW_EDGE => Some(To::West),
        _ => Some(To::East),
    }
}

/// Lets the window's edges and corners resize it, where Quill draws the title bar itself (Windows) and the
/// system's frame, which did that, is gone. Not while the window is maximised or fills the screen.
// Agent: while the pointer is at an edge a small unseen area lies under it, above everything else, so the press
// that starts the resize goes to nothing beneath it.
fn resize_edges(ctx: &egui::Context) {
    use egui::{CursorIcon, ResizeDirection as To};
    let fixed = ctx.input(|i| i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false));
    let Some(at) = ctx.input(|i| i.pointer.latest_pos()).filter(|_| cfg!(windows) && !fixed) else { return };
    let Some(to) = edge_at(ctx.content_rect(), at) else { return };
    let catch = egui::Rect::from_center_size(at, egui::Vec2::splat(2.0 * theme::WINDOW_CORNER));
    let caught = egui::Area::new(egui::Id::unique("window-edge"))
        .order(egui::Order::Tooltip)
        .fixed_pos(catch.min)
        .show(ctx, |ui| ui.allocate_exact_size(catch.size(), egui::Sense::click_and_drag()).1)
        .inner;
    ctx.set_cursor_icon(match to {
        To::North | To::South => CursorIcon::ResizeVertical,
        To::East | To::West => CursorIcon::ResizeHorizontal,
        To::NorthWest | To::SouthEast => CursorIcon::ResizeNwSe,
        To::NorthEast | To::SouthWest => CursorIcon::ResizeNeSw,
    });
    if caught.is_pointer_button_down_on() && ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(to));
    }
}

/// Opens the settings: the user's, or for a picture and a private run ones that are kept in memory only.
/// Opens the settings; with them a sentence for the user when some couldn't be read or won't be saved.
fn open_settings(launch: &Launch) -> (SettingsStore<Prefs>, Option<String>) {
    let location = if launch.screenshot.is_some() || launch.private { Location::Memory } else { Location::Auto };
    let (prefs, report) = SettingsStore::<Prefs>::open(location);
    if !report.has_problems() {
        return (prefs, None);
    }
    log::warn!("settings: some values were reset: {report:?}");
    // Human: the details stay in the log; the user learns what it means for them.
    let sentence = if let Some(moved) = &report.broken_moved_to {
        format!("Your settings file was damaged, so Quill started with its defaults. The old file is kept as {}.", moved.display())
    } else if report.read_only.is_some() {
        "Your settings can't be saved this time; changes last until Quill closes.".to_owned()
    } else {
        "Some of your settings couldn't be read and were reset to their defaults.".to_owned()
    };
    (prefs, Some(sentence))
}

/// Makes the fonts ready, and the GPU for shader effects.
fn install_drawing(cc: &eframe::CreationContext<'_>) {
    fonts::install(&cc.egui_ctx);
    // Human: the starter document and the home page ask which fonts are installed; reading them starts now,
    // off the window's thread.
    crate::faces::warm();
    // Human: shader effects run on the same GPU device the window is drawn with.
    if let Some(state) = &cc.wgpu_render_state {
        crate::gpu::install(state.device.clone(), state.queue.clone());
    }
}

/// The welcome film this run starts with, if any; `seen` is whether the user has been shown it before.
// Human: someone who starts Quill for the first time gets it once. A run that was started to do something (a
// file to open, a message to send, a picture, an AI client's background run, a private run) goes straight to
// that. `--welcome` plays it whatever else is true, and in a picture it stands still at the moment asked for.
fn welcome_film(launch: &Launch, seen: bool) -> Option<crate::splash::Splash> {
    let busy = launch.private || launch.background || launch.ask.is_some() || !launch.files.is_empty();
    match (launch.welcome, launch.screenshot.is_some()) {
        (Some(at), true) => Some(crate::splash::Splash::new(Some(at))),
        (Some(_), false) => Some(crate::splash::Splash::new(None)),
        (None, false) if !seen && !busy => Some(crate::splash::Splash::new(None)),
        _ => None,
    }
}

/// Whether this run fetches the price list of the providers' models when the kept one is a day old.
// Human: the list is looked up for someone who pays a provider by the token: a provider with a key, or a
// key in the environment. A picture of the app fetches nothing, and nor does a private run: it reads a kept list
// but reaches out to nobody.
fn looks_up_prices(agents: &crate::prefs::Agents, launch: &Launch) -> bool {
    agents.prices
        && launch.screenshot.is_none()
        && !launch.private
        && (agents.providers.iter().any(|provider| provider.key_ref.is_some()) || crate::chat::key_in_environment())
}

/// The clipboard text format: a JSON object with this key holding the copied nodes.
const CLIPBOARD_KEY: &str = "pencraftNodes";

impl App {
    /// Builds the app from the eframe creation context.
    pub fn new(cc: &eframe::CreationContext<'_>, launch: Launch) -> App {
        install_drawing(cc);
        sweep_working_folders();
        let (prefs, settings_problem) = open_settings(&launch);
        let ctx = cc.egui_ctx.clone();
        let dialogs = Dialogs::new(Arc::new(move || ctx.request_repaint()));
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(7, |d| u64::from(d.subsec_nanos()) ^ d.as_secs());
        let (tool_send, tool_calls) = std::sync::mpsc::channel();
        let mut app = App {
            // Human: the price list goes by the settings, so it is started before the app takes them over.
            prices: crate::chat::prices::start(looks_up_prices(&prefs.get().agents, &launch)),
            film: welcome_film(&launch, prefs.get().welcome.seen),
            // A picture of the app shows what its settings say, whatever this computer asks for.
            motion: if launch.screenshot.is_some() { crate::motion::System::unasked() } else { crate::motion::System::ask(&cc.egui_ctx) },
            mark: title_mark(&cc.egui_ctx),
            pours: pour::Pours::default(),
            prefs,
            tabs: Vec::new(),
            recovery: Recovery::start(launch.private || launch.screenshot.is_some()),
            active: None,
            canvas: Canvas::default(),
            actions: crate::palette::Palette::default(),
            find: crate::find::Find::default(),
            native_menu: None,
            exporting: None,
            sidebar: Sidebar::default(),
            settings: Settings::default(),
            dialogs,
            clipboard: Vec::new(),
            hide_ui: false,
            prompt: None,
            discard_confirmed: false,
            toasts: crate::toast::Toasts::saying(settings_problem.as_deref()),
            troubles: Troubles::default(),
            screenshot: None,
            seed,
            canvas_view: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000.0, 700.0)),
            link: None,
            tool_calls,
            tool_send,
            talks: Vec::new(),
            talks_made: 0,
            shown: 0,
            assistants: Vec::new(),
            assistant_asked: launch.assistant.clone(),
            new_chat_asked: None,
            private: launch.private || launch.screenshot.is_some(),
            waiting: Vec::new(),
            picturing: Vec::new(),
            generating: pictures::Generating::default(),
            designed: Vec::new(),
            updates: crate::update::Updates::new(launch.looks_for_updates(), launch.update_prompt),
            install: None,
            home: home::Home::default(),
        };
        app.find_assistants();
        // Human: a made-up download stands still in a picture and goes on by itself in a window.
        app.updates.pose_as(
            launch.update_download.map(|percent| (percent, launch.screenshot.is_none())),
            launch.update_required,
            launch.update_collapsed,
        );
        if launch.screenshot.is_none() {
            app.start_link(&cc.egui_ctx);
            app.install_menus(&cc.egui_ctx);
        }
        if let Some(path) = launch.screenshot {
            app.pose_documents(launch.home, &launch.files);
            // Human: a picture can ask for the dark look and a sidebar view; neither is saved (settings are in
            // memory for a screenshot run).
            if launch.dark {
                app.prefs.update(|p| p.appearance.dark = true);
            }
            if let (Some(ids), Some(editor)) = (launch.select, app.tabs.first_mut()) {
                editor.selection = ids.split(',').map(str::to_owned).collect();
                editor.pose_points(launch.points);
            }
            app.pose_extras(launch.settings.as_deref(), launch.actions, launch.find.as_deref(), launch.attach.clone(), launch.generate_image);
            if let (true, Some(editor)) = (launch.activity, app.tabs.first_mut()) {
                editor.activity.posed = true;
            }
            if let (Some(target), Some(editor)) = (launch.export, app.tabs.first_mut()) {
                if editor.selection.is_empty() {
                    editor.selection = editor.doc.children.iter().map(|node| node.id.clone()).collect();
                }
                let ending = target.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
                let format =
                    crate::export::Format::ALL.into_iter().find(|f| ending.as_deref() == Some(f.extension())).unwrap_or(crate::export::Format::Png);
                app.exporting = Some((target, crate::export::Options { format, scale: 2, ..Default::default() }));
            }
            app.screenshot = Some(screenshot::Job::new(path));
        } else {
            let restore: Vec<PathBuf> = if launch.private { Vec::new() } else { app.prefs.get().files.tabs.iter().map(PathBuf::from).collect() };
            for path in restore.iter().chain(&launch.files) {
                app.open_path(path);
            }
        }
        // Human: what an earlier run that crashed left behind is looked for before this run starts keeping its own.
        // Human: `--sidebar chat` opens the app, or takes its picture, with that view showing.
        if let Some(name) = launch.sidebar.as_deref() {
            app.pose_sidebar(name);
        }
        if let Some(text) = launch.ask {
            app.sidebar.chat_asks.push(crate::sidebar::ChatAsk::said(text, launch.attach));
        }
        cc.egui_ctx.set_visuals(theme::visuals(app.dark()));
        // Human: the tabs a run starts with are simply there; only one made while the app is open pours in.
        app.pours = pour::Pours::default();
        app
    }

    /// For a picture of the app: what it shows of `files`, the files named on the command line.
    // Human: the picture shows those files in tabs, or without any the editor with the starter board, one card
    // selected; with `--home` it shows the home screen with those files as its recent ones.
    fn pose_documents(&mut self, home: bool, files: &[PathBuf]) {
        if home {
            self.pose_home(files);
            return;
        }
        for file in files {
            self.open_path(file);
        }
        if self.tabs.is_empty() {
            self.open_sample();
            if let Some(editor) = self.tabs.last_mut() {
                editor.selection = vec!["card2".to_owned()];
            }
        }
    }

    /// Puts the app's menus into the menu bar at the top of the screen, where the system has one.
    // Human: the real menu bar replaces the small app menu `main` put up; a screenshot run keeps that one, so
    // it never takes the keyboard from whatever the user is doing.
    #[cfg_attr(not(target_os = "macos"), expect(clippy::unused_self, clippy::needless_pass_by_ref_mut, reason = "only macOS has a menu bar to fill"))]
    fn install_menus(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "macos")]
        {
            let laid_out = craft_menu::mac_layout(&self.menu_bar(), &craft_menu::LayoutOptions::new(APP_NAME)).bar;
            self.native_menu = craft_menu::mac::install_with(ctx, &laid_out, craft_menu::mac::MacOptions::default().window_list(false));
            if self.native_menu.is_none() {
                log::warn!("menu bar: macOS refused the menus; the app keeps its own");
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = ctx;
    }

    /// Sets the sidebar up for a picture: a view's name, or `conversation` for the chat view with a short
    /// exchange in it.
    fn pose_sidebar(&mut self, name: &str) {
        // Human: `question` and `suggestions` show the dialog over the bar of the folded sidebar, `asked` the
        // same question in the conversation.
        let (entries, view) = match name {
            "conversation" => (Some(crate::chat::sample()), "chat"),
            "question" => (Some(crate::chat::sample_asking(false)), "hidden"),
            "suggestions" => (Some(crate::chat::sample_asking(true)), "hidden"),
            "offers" | "offered" => (Some(crate::chat::sample()), if name == "offers" { "chat" } else { "hidden" }),
            "asked" => (Some(crate::chat::sample_asking(false)), "chat"),
            "pinned" => (Some(crate::chat::sample_asking(false)), "layers"),
            _ => (None, name),
        };
        if let Some(entries) = entries {
            self.front_talk().chat.entries = entries;
        }
        // Human: `offers` shows the assistant's next steps beside the open chat, `offered` beside the folded bar.
        if matches!(name, "offers" | "offered") {
            let steps = ["Unify the matcha level across all screens of the ordering app", "Make the headline bolder", "Export the hero as a PNG"];
            self.front_talk().chat.offer(&steps.map(str::to_owned));
        }
        // Human: `thinking` shows the chat in the middle of a run, while the model thinks.
        let view = if name == "thinking" {
            self.front_talk().chat.pose_thinking();
            "chat"
        } else {
            view
        };
        // Human: `tasks` shows the task list of a long job with two tasks finished; `pooling` and `pouring` a new
        // list as it comes in, `stepping` the step from one task to the next, `adding` two tasks joining it,
        // `finished` the list filed into the conversation, `handing` and `landing` its way there.
        let done = match name {
            "pooling" | "pouring" => Some(0),
            "tasks" | "adding" => Some(2),
            "stepping" => Some(3),
            "finished" | "handing" | "landing" => Some(usize::MAX),
            _ => None,
        };
        if let Some(done) = done {
            let chat = &mut self.front_talk().chat;
            if done == usize::MAX {
                chat.entries = crate::chat::sample();
            } else {
                chat.pose_thinking();
            }
            chat.tasks = crate::chat::tasks::Tasks::sample(done);
            if name == "adding" {
                // The list is the sample's, so the two tasks are new to it and there is room for them.
                let _ = chat.tasks.add(&serde_json::json!({ "tasks": ["Fix the cut-off Save button", "Check the result again"] }));
            }
        }
        self.sidebar.pose(view);
    }

    /// Starts the local server AI models reach the app through.
    // Human: a tool call wakes the app, also while its window is hidden or behind others.
    fn start_link(&mut self, ctx: &egui::Context) {
        let wake = ctx.clone();
        match crate::link::start(self.tool_send.clone(), move || wake.request_repaint()) {
            Ok(server) => {
                log::info!("AI tools: listening on 127.0.0.1:{}", server.port);
                self.link = Some(server);
                self.tell_link();
            }
            Err(error) => {
                log::warn!("AI tools: the local server didn't start: {error}");
                self.warn(format!("AI apps can't reach Quill: its local tool server didn't start ({error})."));
            }
        }
    }

    /// Tells the local server which tools the settings turn off, so it leaves them out of its list.
    fn tell_link(&self) {
        if let Some(server) = &self.link {
            *lock(&server.off) = self.prefs.get().agents.tools_off();
        }
    }

    fn dark(&self) -> bool {
        self.prefs.get().appearance.dark
    }

    /// Changes between the light and the dark look.
    // Human: choosing a look by hand ends following the computer's.
    fn toggle_theme(&mut self, ctx: &egui::Context) {
        let dark = !self.dark();
        self.prefs.update(|p| {
            p.appearance.dark = dark;
            p.appearance.follow_system = false;
        });
        ctx.set_visuals(theme::visuals(dark));
    }

    /// While the settings say to follow the computer's appearance, takes the look from it.
    // Human: the computer can change its appearance at any time (at sunset, say), so this is asked every frame.
    fn follow_system_look(&mut self, ctx: &egui::Context) {
        if !self.prefs.get().appearance.follow_system {
            return;
        }
        let Some(look) = ctx.system_theme() else { return };
        let dark = look == egui::Theme::Dark;
        if dark != self.dark() {
            self.prefs.update(|p| p.appearance.dark = dark);
            ctx.set_visuals(theme::visuals(dark));
        }
    }

    fn next_seed(&mut self) -> u64 {
        self.seed = self.seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.seed
    }

    /// Shows `text` as a message: as a problem when its words say something went wrong ([`crate::toast::kind_of`]).
    // Human: many sentences come from places that give one for a success and one for a failure alike (an
    // export, a settings page); those are told apart by their words. Where a failure is known as one, it is
    // said with `App::warn`.
    fn say(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.toasts.say("", &text, crate::toast::kind_of(&text));
    }

    /// Shows `text` as a message that says what went wrong: it looks the part and stays longer.
    fn warn(&mut self, text: impl Into<String>) {
        self.toasts.say("", &text.into(), crate::toast::Kind::Problem);
    }

    /// [`App::say`] for what a model says: its name stands before the sentence.
    fn say_from(&mut self, who: &str, text: &str) {
        self.toasts.say(who, text, crate::toast::Kind::Note);
    }

    /// Does what the tool rail asked for.
    fn asked(&mut self, asked: Option<Ask>) {
        match asked {
            Some(Ask::Import) => self.ask_import(),
            Some(Ask::Actions) => self.actions.toggle(),
            Some(Ask::Find) => self.open_find(),
            None => {}
        }
    }

    /// For a picture of the app: the settings window on a page (`--settings`), the action search open
    /// (`--actions`), find layers with a query (`--find`), files attached to the chat message (`--attach`), the
    /// "Generate Image…" dialog for the selection (`--generate-image`).
    fn pose_extras(&mut self, settings: Option<&str>, actions: bool, find: Option<&str>, attach: Vec<PathBuf>, generate: bool) {
        if let Some(name) = settings {
            self.settings.open_named(name);
        }
        self.attach(attach);
        if actions {
            self.actions.toggle();
        }
        if let Some(query) = find {
            self.open_find();
            self.find.pose(query);
        }
        if generate {
            self.open_picture_dialog();
        }
    }

    /// Opens find layers for the document in front.
    fn open_find(&mut self) {
        if let Some(editor) = self.active.and_then(|tab| self.tabs.get_mut(tab)) {
            self.find.open(editor);
        }
    }

    /// Shows find layers beside the tool rail while it is open. Only the document in front carries its marks.
    fn find_layers(&mut self, ctx: &egui::Context) {
        if !self.find.is_open() {
            return;
        }
        let (view, dark) = (self.canvas_view, self.dark());
        for (index, editor) in self.tabs.iter_mut().enumerate() {
            if Some(index) != self.active {
                editor.spotlight = (Vec::new(), None);
            }
        }
        let at = view.min + egui::vec2(2.0 * ISLAND_GAP + RAIL_WIDTH, ISLAND_GAP);
        match self.active.and_then(|tab| self.tabs.get_mut(tab).map(|editor| (tab, editor))) {
            Some((tab, editor)) => self.find.show(ctx, at, view, editor, tab, dark),
            None => self.find.hide(),
        }
    }

    fn editor(&mut self) -> Option<&mut Editor> {
        self.active.and_then(|i| self.tabs.get_mut(i))
    }

    fn add_tab(&mut self, editor: Editor) {
        self.tabs.push(editor);
        self.active = Some(self.tabs.len() - 1);
        if self.prefs.get().appearance.animations {
            self.pours.made(self.tabs.len() - 1);
        }
        self.canvas.editing = None;
        self.remember_tabs();
    }

    /// Opens a file in a tab, or brings its tab to the front when it is already open.
    fn open_path(&mut self, path: &Path) {
        if let Some(i) = self.tabs.iter().position(|t| t.path.as_deref() == Some(path)) {
            self.active = Some(i);
            return;
        }
        if let Err(error) = self.try_open(path) {
            self.warn(error);
        }
    }

    /// Opens a file in a tab, or brings its tab to the front when it is already open.
    ///
    /// # Errors
    /// A sentence when the file can't be read or isn't a design file.
    fn try_open(&mut self, path: &Path) -> Result<(), String> {
        if let Some(i) = self.tabs.iter().position(|t| t.path.as_deref() == Some(path)) {
            self.active = Some(i);
            return Ok(());
        }
        let seed = self.next_seed();
        let (editor, newer) = Editor::open(path, seed).map_err(|e| format!("Couldn't open {}: {e}", path.display()))?;
        self.add_tab(editor);
        if let Some(newer) = newer {
            self.say(newer);
        }
        let shown = path.display().to_string();
        self.prefs.update(|p| p.files.touch(&shown));
        Ok(())
    }

    fn new_file(&mut self) {
        let seed = self.next_seed();
        self.add_tab(Editor::new(pen_doc::Document::default(), None, seed));
    }

    fn open_sample(&mut self) {
        match pen_doc::read(&samples::starter(&crate::faces::installed)) {
            Ok(doc) => {
                let seed = self.next_seed();
                self.add_tab(Editor::new(doc, None, seed));
            }
            Err(e) => self.warn(format!("The starter board didn't load: {e}")),
        }
    }

    fn remember_tabs(&mut self) {
        if self.private {
            return;
        }
        let tabs: Vec<String> = self.tabs.iter().filter_map(|t| t.path.as_ref().map(|p| p.display().to_string())).collect();
        self.prefs.update(|p| p.files.tabs = tabs);
    }

    fn ask_open(&mut self) {
        self.dialogs.ask(FileDialog::open_many().title("Open").filter("Design files", &[bundle::ENDING, bundle::PLAIN, bundle::OLD]), Then::Open);
    }

    fn save(&mut self, tab: usize, close: bool) {
        let was = self.tabs.get(tab).and_then(Editor::anchor);
        self.save_from(tab, close, was.as_deref(), None);
    }

    /// Saves the document in the tab `tab`, whose files lay beside `was` when it was last saved (its
    /// [`Editor::anchor`] then; `None` when never), asking for a file first when it has none; its pictures are
    /// brought to where it is saved (`pictures::settle`). `moved` is where the document was before it was given
    /// the name it has now for this save (Save As): it goes back there when the save fails.
    fn save_from(&mut self, tab: usize, close: bool, was: Option<&Path>, moved: Option<crate::editor::Place>) {
        let Some(editor) = self.tabs.get_mut(tab) else { return };
        if editor.path.is_none() {
            self.ask_save("Save", tab, close);
            return;
        }
        // Human: a `.pen` document, an old `.pencraft` bundle and a document opened from a file of any other
        // name are opened, not saved to: they are saved as a `.quill` file beside them, and stay as they are. A `.quill` file of that name that is
        // there already is somebody's: the user says where to.
        let (mut renamed, mut from) = (None, moved);
        if let Some(next) = editor.path.as_deref().and_then(bundle::successor) {
            let name = |path: &Path| path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
            if next.exists() {
                self.say(format!("{} is there already: choose where to save.", name(&next)));
                self.ask_save("Save As", tab, close);
                return;
            }
            let left = editor.path.as_deref().map(name).unwrap_or_default();
            from = Some(editor.place());
            if let Err(error) = editor.move_to(next.clone()) {
                self.warn(format!("Couldn't save: {error}"));
                return;
            }
            renamed = Some(format!("It is now {}; {left} is left as it is.", name(&next)));
        }
        let mut told = pictures::settle(editor, was, crate::imagery::store::home().as_deref());
        // Human: when the file can't be written under its new name, the document has the file it had again.
        let moved = from.is_some();
        match editor.save_or_return(from) {
            Ok(more) => {
                if let Some(path) = editor.path.as_ref().map(|p| p.display().to_string()) {
                    self.prefs.update(|p| p.files.touch(&path));
                }
                told.extend(renamed);
                told.extend(more);
                if !told.is_empty() {
                    self.say(format!("Saved. {}", told.join(" ")));
                }
                if moved {
                    self.remember_tabs();
                }
                if close {
                    self.close_tab_now(tab);
                }
            }
            Err(e) => self.warn(format!("Couldn't save: {e}")),
        }
    }

    fn save_as(&mut self, tab: usize) {
        self.ask_save("Save As", tab, false);
    }

    /// Asks where to save the document in the tab `tab`: as a `.quill` file, the one format a document is
    /// saved as.
    // Human: a `.quill` file is one file with the document's pictures and files in it. A `.pen` file, the
    // document alone, is made by File › Export .pen File… ([`App::ask_export_pen`]).
    fn ask_save(&mut self, title: &str, tab: usize, close: bool) {
        let Some(editor) = self.tabs.get(tab) else { return };
        let name = format!("{}.{}", editor.title(), bundle::ENDING);
        self.dialogs.ask(FileDialog::save().title(title).filter("Quill designs", &[bundle::ENDING]).file_name(name), Then::SaveAs { tab, close });
    }

    /// Asks where to write the document in front as a `.pen` file.
    fn ask_export_pen(&mut self) {
        let Some(editor) = self.active.and_then(|tab| self.tabs.get(tab)) else { return };
        let name = format!("{}.{}", editor.title(), bundle::PLAIN);
        self.dialogs
            .ask(FileDialog::save().title("Export .pen File").filter(".pen documents", &[bundle::PLAIN]).file_name(name), Then::ExportPen(editor.id));
    }

    /// Writes the document with the id `doc` as a `.pen` file at `path`, and says how it went.
    // Human: an export: the document keeps its file, its working folder and its unsaved changes, and the file
    // that is written is not opened or remembered.
    // Agent: READS and WRITES files (`pictures::export_pen`); BLOCKS on them.
    fn export_pen(&mut self, doc: u64, path: PathBuf) {
        let Some(editor) = self.tabs.iter().find(|tab| tab.id == doc) else { return };
        let made = bundle::ended(path, bundle::PLAIN)
            .and_then(|target| pictures::export_pen(editor, &target, crate::imagery::store::home().as_deref()).map(|told| (target, told)));
        // Human: the file's name alone, as Save says it: a whole path is cut off before the name in the message.
        let name = |target: &Path| target.file_name().map_or_else(|| target.display().to_string(), |name| name.to_string_lossy().into_owned());
        let said = match made {
            Ok((target, told)) if told.is_empty() => format!("Exported {}", name(&target)),
            Ok((target, told)) => format!("Exported {}. {}", name(&target), told.join(" ")),
            Err(error) => format!("Couldn't export: {error}"),
        };
        self.say(said);
    }

    /// Closes a tab, asking first when it has unsaved changes.
    fn close_tab(&mut self, tab: usize) {
        if self.tabs.get(tab).is_some_and(Editor::dirty) {
            self.prompt = Some(ClosePrompt { tab, quitting: false });
        } else {
            self.close_tab_now(tab);
        }
    }

    fn close_tab_now(&mut self, tab: usize) {
        if tab >= self.tabs.len() {
            return;
        }
        let mut closed = self.tabs.remove(tab);
        if self.prefs.get().appearance.animations {
            self.pours.closed(tab, closed.tab_name());
        }
        closed.close();
        if let Some(keeper) = &mut self.recovery.keeper {
            keeper.forget(closed.id);
        }
        self.close_talk(closed.id);
        self.active = match self.active {
            Some(a) if a == tab => (!self.tabs.is_empty()).then(|| tab.min(self.tabs.len() - 1)),
            Some(a) if a > tab => Some(a - 1),
            other => other,
        };
        self.canvas.editing = None;
        self.remember_tabs();
    }

    /// Asks for the file to export the selection to.
    fn ask_export(&mut self, options: crate::export::Options) {
        let Some(editor) = self.active.and_then(|tab| self.tabs.get(tab)) else { return };
        if editor.doc.children.is_empty() {
            self.say("There is nothing to export.");
            return;
        }
        let name = crate::export::suggested_name(editor, options);
        let dialog = FileDialog::save().title("Export").filter(options.format.label(), &[options.format.extension()]).file_name(name);
        self.dialogs.ask(dialog, Then::Export(options));
    }

    /// Makes the export the user chose a place for, and says how it went.
    fn finish_export(&mut self, ctx: &egui::Context) {
        let Some((target, options)) = self.exporting.take() else { return };
        let Some(editor) = self.active.and_then(|tab| self.tabs.get(tab)) else { return };
        // The document is laid out with the first frame; an export asked for before that waits for it.
        // Human: so does one asked for while a face of the document's text is still being made ready: until
        // then the text is measured in a stand-in, and what follows it in a row would sit in the wrong place.
        if (editor.scene.is_empty() && !editor.doc.children.is_empty()) || fonts::pending() || editor.stale() {
            self.exporting = Some((target, options));
            ctx.request_repaint();
            return;
        }
        let target = if target.extension().is_none() { target.with_extension(options.format.extension()) } else { target };
        let said = match crate::export::run(self.canvas.list(), editor, ctx, options, &target) {
            Ok(()) => format!("Exported {}", target.display()),
            Err(error) => format!("Couldn't export: {error}"),
        };
        self.say(said);
    }

    fn ask_import(&mut self) {
        let kinds = ["png", "jpg", "jpeg", "webp", "gif", "svg"];
        self.dialogs.ask(FileDialog::open_many().title("Import Image or SVG").filter("Pictures", &kinds), Then::Import);
    }

    /// Asks for picture files to send as references with the picture of the Generate Image dialog opened for the
    /// document `doc`.
    fn ask_reference(&mut self, doc: u64) {
        let kinds = ["png", "jpg", "jpeg", "webp", "gif"];
        self.dialogs.ask(FileDialog::open_many().title("Add Reference Pictures").filter("Pictures", &kinds), Then::Reference(doc));
    }

    /// Does what an action of the action search (or its shortcut) asks.
    // Human: one place for everything the app can be told to do, so the search and the keyboard can't drift
    // apart in what an action means.
    // Agent: MAIN-THREAD; commands on the document do nothing while no document is open.
    fn run(&mut self, command: Command, ctx: &egui::Context) {
        let view = self.canvas_view;
        match command {
            Command::Dashboard => self.active = None,
            Command::Save => {
                if let Some(tab) = self.active {
                    self.save(tab, false);
                }
            }
            Command::SaveAs => {
                if let Some(tab) = self.active {
                    self.save_as(tab);
                }
            }
            Command::ExportPen => self.ask_export_pen(),
            Command::NewFile => self.new_file(),
            Command::OpenFile => self.ask_open(),
            Command::CloseTab => {
                if let Some(tab) = self.active {
                    self.close_tab(tab);
                }
            }
            Command::Import => {
                if self.active.is_some() {
                    self.ask_import();
                }
            }
            // Human: it opens only for a document in front (`open_picture_dialog`).
            Command::GenerateImage => self.open_picture_dialog(),
            Command::Settings => self.settings.open_general(),
            Command::CheckForUpdates => {
                self.say("Looking for a new version…");
                self.updates.look_now(ctx);
            }
            Command::Export(format) => {
                let saved = self.active.and_then(|tab| self.tabs.get(tab)).map_or_else(crate::export::Options::default, |editor| editor.export.0);
                self.ask_export(crate::export::Options { format, ..saved });
            }
            Command::Actions => self.actions.toggle(),
            Command::FindLayers => self.open_find(),
            Command::OpenRecent(index) => {
                if let Some(path) = self.prefs.get().files.recent.get(index).cloned() {
                    self.open_path(Path::new(&path));
                }
            }
            Command::ShowTab(index) => {
                if index < self.tabs.len() {
                    self.active = Some(index);
                }
            }
            Command::PreviousTab => {
                self.active = self.active.and_then(|a| a.checked_sub(1)).or(if self.tabs.is_empty() { None } else { Some(self.tabs.len() - 1) });
            }
            Command::NextTab => {
                self.active = self.active.map_or(if self.tabs.is_empty() { None } else { Some(0) }, |a| Some((a + 1) % self.tabs.len().max(1)));
            }
            Command::MoveTab(left) => {
                // Human: the tab in front swaps places with its neighbour and stays in front.
                let Some(from) = self.active else { return };
                let to = if left { from.checked_sub(1) } else { Some(from + 1).filter(|to| *to < self.tabs.len()) };
                if let Some(to) = to {
                    self.tabs.swap(from, to);
                    self.active = Some(to);
                    self.remember_tabs();
                }
            }
            Command::ToggleTheme => {
                self.toggle_theme(ctx);
            }
            Command::FullScreen => {
                let full = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!full));
            }
            Command::ToggleInterface => self.hide_ui = !self.hide_ui,
            Command::Show(name) => self.sidebar.pose(name),
            Command::Copy | Command::Cut => {
                let Some(editor) = self.editor() else { return };
                let nodes = editor.copy_selection();
                if command == Command::Cut {
                    editor.delete_selection();
                }
                if !nodes.is_empty() {
                    if let Ok(text) = serde_json::to_string(&serde_json::json!({ CLIPBOARD_KEY: nodes })) {
                        ctx.copy_text(text);
                    }
                    self.clipboard = nodes;
                }
            }
            Command::Paste => {
                let nodes = self.clipboard.clone();
                if let Some(editor) = self.editor() {
                    let visible = editor.visible_area(view.width(), view.height());
                    editor.paste(&nodes, visible);
                }
            }
            _ => {
                if let Some(editor) = self.editor() {
                    run_on(editor, command, view);
                }
            }
        }
    }

    /// What the menu bar shows of the app right now.
    fn menu_bar(&self) -> craft_menu::MenuBar {
        let editor = self.active.and_then(|tab| self.tabs.get(tab));
        crate::menus::bar(&crate::menus::State {
            document: editor.is_some(),
            selection: editor.is_some_and(|e| !e.selection.is_empty()),
            recent: &self.prefs.get().files.recent,
            tabs: self.tabs.iter().enumerate().map(|(index, tab)| (tab.title(), self.active == Some(index))).collect(),
        })
    }

    /// Runs what was chosen in the menu bar since the last frame, and brings the menus up to date.
    // Human: while a dialog asks something the menus' own commands wait; while a text field has the keys, ⌘C
    // and the like go to the field. The shortcuts shown in the menus still arrive as key presses, so the
    // app's own shortcut handling runs them; only a click, or a shortcut while typing, comes through here.
    // Agent: MAIN-THREAD; lays the menus out every frame and lets craft-menu rebuild them only when they differ.
    fn native_menus(&mut self, ctx: &egui::Context) {
        let Some(mut menu) = self.native_menu.take() else { return };
        // Human: the Generate Image dialog counts as asking something: a menu click must not change the
        // document behind it either (see `shortcuts`).
        let modal = self.prompt.is_some() || self.generating.is_open();
        menu.set_gate(craft_menu::Gate { modal, recording_shortcut: false, text_editing: ctx.egui_wants_keyboard_input() });
        for chosen in menu.take_clicks(ctx) {
            if let Some(command) = crate::menus::command(&chosen) {
                self.run(command, ctx);
            }
        }
        for clash in menu.sync(craft_menu::mac_layout(&self.menu_bar(), &craft_menu::LayoutOptions::new(APP_NAME))) {
            log::info!("menu: {} is {} on macOS and {} in the app; {}", clash.shortcut, clash.system, clash.command, clash.resolution);
        }
        self.native_menu = Some(menu);
    }

    fn handle_dialog_answers(&mut self) {
        for (then, answer) in self.dialogs.take_answers() {
            match (then, answer) {
                (Then::Open, Answer::Paths(paths)) => {
                    for path in paths {
                        self.open_path(&path);
                    }
                }
                (Then::SaveAs { tab, close }, Answer::Paths(paths)) => {
                    let Some(path) = paths.into_iter().next() else { continue };
                    // Human: a name ending `.pen` or `.pencraft` is refused, with how such a file is made.
                    let path = match bundle::ended(path, bundle::ENDING) {
                        Ok(path) => path,
                        Err(error) => {
                            self.warn(format!("Couldn't save: {error}"));
                            continue;
                        }
                    };
                    if let Some(editor) = self.tabs.get_mut(tab) {
                        // Human: the tabs to open next time are noted when the save worked (`save_from`).
                        let before = editor.place();
                        match editor.move_to(path) {
                            Ok(was) => self.save_from(tab, close, was.as_deref(), Some(before)),
                            Err(error) => self.warn(format!("Couldn't save: {error}")),
                        }
                    }
                }
                (Then::ExportPen(doc), Answer::Paths(paths)) => {
                    if let Some(path) = paths.into_iter().next() {
                        self.export_pen(doc, path);
                    }
                }
                (Then::Export(options), Answer::Paths(paths)) => self.exporting = paths.into_iter().next().map(|path| (path, options)),
                (Then::Attach, Answer::Paths(paths)) => self.attach(paths),
                (Then::Reference(doc), Answer::Paths(paths)) => self.add_references(doc, paths),
                (Then::Import, Answer::Paths(paths)) => {
                    let view = self.canvas_view;
                    let mut failed = None;
                    if let Some(editor) = self.editor() {
                        let visible = editor.visible_area(view.width(), view.height());
                        for path in paths {
                            if let Err(error) = editor.import_file(&path, visible) {
                                failed = Some(format!("Couldn't import {}: {error}", path.display()));
                            }
                        }
                    }
                    if let Some(sentence) = failed {
                        self.say(sentence);
                    }
                }
                (_, Answer::Unavailable(why)) => self.warn(format!("The file dialog couldn't open: {why}")),
                (Then::SaveAs { close: true, .. }, Answer::Cancelled) => self.discard_confirmed = false,
                (_, Answer::Cancelled | Answer::Message(_)) => {}
            }
        }
    }

    /// Runs the keyboard shortcuts of the shortcut sheet and menus.
    #[expect(clippy::too_many_lines, reason = "one table of shortcuts")]
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let presses: Vec<(Key, Modifiers)> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key { key, physical_key, pressed: true, modifiers, .. } => Some((shortcut_key(*key, *physical_key), *modifiers)),
                    _ => None,
                })
                .collect()
        });
        // Human: words selected in the chat take ⌘C and ⌘X for themselves (egui copies them): the layers selected
        // on the canvas are neither copied over them nor cut away. Pasting still goes to the canvas.
        let words_selected = ctx.plugin_opt::<egui::text_selection::LabelSelectionState>().is_some_and(|state| state.lock().has_selection());
        let clipboard_events: Vec<egui::Event> = ctx.input(|i| {
            i.events
                .iter()
                .filter(|e| match e {
                    egui::Event::Copy | egui::Event::Cut => !words_selected,
                    egui::Event::Paste(_) | egui::Event::PasteImage(_) => true,
                    _ => false,
                })
                .cloned()
                .collect()
        });
        // Human: while the Generate Image dialog shows, no shortcut acts on the documents behind it: Backspace
        // would delete the layer the picture is meant for, ⌘W close its tab. Here, before any of them, because
        // this is the one place every app and editor shortcut goes through; the dialog's own keys (Escape, typing
        // in its field) are egui's and aren't touched. The Settings window keeps its own way.
        if self.generating.is_open() {
            return;
        }
        let typing = ctx.egui_wants_keyboard_input();
        let none = Modifiers::NONE;
        let cmd = Modifiers::COMMAND;
        let shift = Modifiers::SHIFT;
        let alt = Modifiers::ALT;
        let cmd_shift = cmd | shift;
        let cmd_alt = cmd | alt;
        let alt_shift = alt | shift;
        let hit = |key: Key, mods: Modifiers| {
            presses.iter().any(|(k, m)| *k == key && m.command == mods.command && m.shift == mods.shift && m.alt == mods.alt)
        };
        // App-wide, also while typing.
        if hit(Key::S, cmd)
            && let Some(tab) = self.active
        {
            self.save(tab, false);
        }
        if hit(Key::S, cmd_shift)
            && let Some(tab) = self.active
        {
            self.save_as(tab);
        }
        if hit(Key::O, cmd) {
            self.ask_open();
        }
        if hit(Key::N, cmd) {
            self.active = None;
        }
        if hit(Key::N, cmd_shift) {
            self.new_file();
        }
        if hit(Key::W, cmd)
            && let Some(tab) = self.active
        {
            self.close_tab(tab);
        }
        if hit(Key::P, cmd) {
            self.actions.toggle();
        }
        // Human: find layers, ⌘F or ⇧⌘F.
        if hit(Key::F, cmd) || hit(Key::F, cmd_shift) {
            self.open_find();
        }
        if hit(Key::Comma, cmd) {
            self.settings.open_general();
        }
        if hit(Key::Backslash, cmd) {
            self.hide_ui = !self.hide_ui;
        }
        if hit(Key::OpenBracket, cmd_shift) {
            self.run(Command::PreviousTab, ctx);
        }
        if hit(Key::CloseBracket, cmd_shift) {
            self.run(Command::NextTab, ctx);
        }
        // Human: the Window menu: ⌃⇧Page Up and Page Down move the tab; ⌃⌘F fills the screen.
        let held = |key: Key, command: bool| presses.iter().any(|(k, m)| *k == key && m.ctrl && m.shift != command && m.mac_cmd == command && !m.alt);
        if held(Key::PageUp, false) {
            self.run(Command::MoveTab(true), ctx);
        }
        if held(Key::PageDown, false) {
            self.run(Command::MoveTab(false), ctx);
        }
        // Human: off the Mac there is no ⌘ to hold with Ctrl; F11 fills the screen there, as in other Windows apps.
        if held(Key::F, true) || (!cfg!(target_os = "macos") && hit(Key::F11, none)) {
            self.run(Command::FullScreen, ctx);
        }
        for (n, key) in [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9].into_iter().enumerate() {
            if hit(key, cmd) && !self.tabs.is_empty() {
                self.active = Some(if n == 8 { self.tabs.len() - 1 } else { n.min(self.tabs.len() - 1) });
            }
        }
        if typing || self.canvas.busy() {
            return;
        }
        // Human: a picture on the clipboard (a screenshot, a picture copied in a browser) is pasted as a layer.
        for event in &clipboard_events {
            if let egui::Event::PasteImage(picture) = event {
                self.paste_picture(picture, ctx);
            }
        }
        let view = self.canvas_view;
        let clipboard = self.clipboard.clone();
        let (nudge, big_nudge) = (self.prefs.get().canvas.small_nudge, self.prefs.get().canvas.big_nudge);
        let Some(editor) = self.editor() else { return };
        let mut copied: Option<Vec<Node>> = None;
        for event in clipboard_events {
            match event {
                egui::Event::Copy => copied = Some(editor.copy_selection()),
                egui::Event::Cut => {
                    copied = Some(editor.copy_selection());
                    editor.delete_selection();
                }
                egui::Event::Paste(text) => {
                    let nodes = parse_clipboard(&text).unwrap_or_else(|| clipboard.clone());
                    let visible = editor.visible_area(view.width(), view.height());
                    if nodes.is_empty() && editor.paste_svg(&text, visible) {
                        // Drawn as the shapes it describes.
                    } else if nodes.is_empty() && !text.trim().is_empty() {
                        paste_text(editor, &text, visible);
                    } else {
                        editor.paste(&nodes, visible);
                    }
                }
                _ => {}
            }
        }
        if hit(Key::Z, cmd) {
            editor.undo();
        }
        if hit(Key::Z, cmd_shift) {
            editor.redo();
        }
        if hit(Key::D, cmd) {
            editor.duplicate();
        }
        if hit(Key::Delete, none) || hit(Key::Backspace, none) {
            editor.delete_selection();
        }
        if hit(Key::G, cmd) {
            editor.wrap_selection(&pen_doc::Kind::Group);
        }
        if hit(Key::L, cmd_shift) {
            editor.toggle_lock();
        }
        if hit(Key::G, cmd_shift) || hit(Key::Backspace, cmd) {
            editor.ungroup();
        }
        if hit(Key::G, cmd_alt) {
            editor.wrap_selection(&pen_doc::Kind::Frame);
        }
        if hit(Key::A, shift) {
            editor.add_flex_layout();
        }
        if hit(Key::A, alt_shift) {
            editor.remove_flex_layout();
        }
        if hit(Key::H, shift) {
            editor.flip(true);
        }
        if hit(Key::V, shift) {
            editor.flip(false);
        }
        for (key, to) in [
            (Key::W, AlignTo::Top),
            (Key::A, AlignTo::Left),
            (Key::S, AlignTo::Bottom),
            (Key::D, AlignTo::Right),
            (Key::H, AlignTo::Center),
            (Key::V, AlignTo::Middle),
        ] {
            if hit(key, alt) {
                editor.align(to);
            }
        }
        for (key, align) in [(Key::L, "left"), (Key::T, "center"), (Key::R, "right"), (Key::J, "justify")] {
            if hit(key, cmd_alt) {
                editor.text_align(align);
            }
        }
        if hit(Key::CloseBracket, cmd) {
            editor.reorder(Order::Forward);
        }
        if hit(Key::OpenBracket, cmd) {
            editor.reorder(Order::Backward);
        }
        if hit(Key::CloseBracket, none) {
            editor.reorder(Order::Front);
        }
        if hit(Key::OpenBracket, none) {
            editor.reorder(Order::Back);
        }
        if hit(Key::K, cmd_alt) {
            editor.create_component();
        }
        if hit(Key::X, cmd_alt) {
            editor.detach_instances();
        }
        if hit(Key::A, cmd) {
            editor.select_all();
        }
        // Human: in a path being edited Escape first lets the selected points go, then ends the editing.
        if hit(Key::Escape, none) && !editor.leave_path_edit() {
            if editor.tool == Tool::Move {
                editor.selection.clear();
            }
            editor.tool = Tool::Move;
        }
        if hit(Key::Enter, none) {
            editor.select_children();
        }
        if hit(Key::Enter, shift) {
            editor.select_parent();
        }
        for (key, dx, dy) in [(Key::ArrowLeft, -1.0, 0.0), (Key::ArrowRight, 1.0, 0.0), (Key::ArrowUp, 0.0, -1.0), (Key::ArrowDown, 0.0, 1.0)] {
            if hit(key, none) {
                editor.nudge(dx * nudge, dy * nudge);
            }
            if hit(key, shift) {
                editor.nudge(dx * big_nudge, dy * big_nudge);
            }
        }
        // View.
        if hit(Key::Num1, shift) {
            let bounds = editor.content_bounds();
            canvas::fit(editor, view, bounds, MAX_ZOOM);
        }
        if hit(Key::Num2, shift) {
            let bounds = editor.selection_bounds().or_else(|| editor.content_bounds());
            canvas::fit(editor, view, bounds, MAX_ZOOM);
        }
        if hit(Key::Num0, cmd) {
            canvas::zoom_at(editor, view, view.center(), 1.0);
        }
        if hit(Key::Equals, cmd) || hit(Key::Plus, cmd) || hit(Key::Equals, cmd_shift) {
            canvas::zoom_at(editor, view, view.center(), canvas::zoom_step(editor.camera.zoom, true));
        }
        if hit(Key::Minus, cmd) {
            canvas::zoom_at(editor, view, view.center(), canvas::zoom_step(editor.camera.zoom, false));
        }
        // Tools (shortcut sheet: V, H, F or A, R, O, T, N).
        for (key, tool) in [
            (Key::V, Tool::Move),
            (Key::H, Tool::Hand),
            (Key::F, Tool::Frame),
            (Key::A, Tool::Frame),
            (Key::R, Tool::Rectangle),
            (Key::O, Tool::Ellipse),
            (Key::T, Tool::Text),
            (Key::N, Tool::Note),
            (Key::P, Tool::Pen),
            (Key::C, Tool::Connector),
            (Key::K, Tool::Scale),
            (Key::I, Tool::Eyedropper),
        ] {
            if hit(key, none) {
                editor.tool = tool;
            }
        }
        if let Some(nodes) = copied.filter(|n| !n.is_empty()) {
            if let Ok(text) = serde_json::to_string(&serde_json::json!({ CLIPBOARD_KEY: nodes })) {
                ctx.copy_text(text);
            }
            self.clipboard = nodes;
        }
    }

    /// The top bar, which is the window's title bar: from the left the app's name, home, the open files and the
    /// button for a new one; from the right the menu and the settings, and on Windows the
    /// window's own three buttons after them (`design/pencraft.pen`, "Redesign: Title bar").
    fn tab_bar(&mut self, ui: &mut egui::Ui) {
        let palette = theme::palette(self.dark());
        let bar = ui.max_rect();
        // Human: the bar lies on the window's own colour, also over a file, where the canvas ends under it.
        ui.painter().rect_filled(bar, 0.0, palette.background);
        // Human: the empty part of the bar moves the window, as the title bar it replaces did.
        let drag = ui.interact(bar, ui.scope_id().with("window-drag"), egui::Sense::click_and_drag());
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        }
        let row = egui::Rect::from_min_max(bar.min + egui::vec2(TITLE_PADDING[0], 0.0), bar.max - egui::vec2(TITLE_PADDING[1], 0.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
            self.bar_files(ui, palette);
        });
        ui.scope_builder(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| {
            ui.spacing_mut().item_spacing.x = TITLE_GAP;
            if cfg!(windows) {
                window_buttons(ui, palette);
            }
            // Human: right to left, so the menu ends up first. Share, Present and Publish (the design's) stay out
            // of the bar until they do something.
            if bar_button(ui, "settings", false, palette.strong, palette).on_hover_text(crate::keys::text("Settings (⌘,)")).clicked() {
                self.settings.open_general();
            }
            self.main_menu(ui, palette);
        });
    }

    /// The title bar's left part: the app's icon and name, home, the open files and the button for a new one.
    // Human: a tab made while the app is open pours out of that button (`pour`): the liquid is drawn under the
    // tab and the button, in a place kept for it before either is drawn.
    fn bar_files(&mut self, ui: &mut egui::Ui, palette: theme::Palette) {
        ui.spacing_mut().item_spacing.x = TITLE_GAP;
        if cfg!(target_os = "macos") {
            ui.add_space(TRAFFIC_LIGHT_SPACE - TITLE_PADDING[0]);
        }
        if let Some(mark) = &self.mark {
            let (place, _) = ui.allocate_exact_size(egui::Vec2::splat(theme::TITLE_MARK), egui::Sense::hover());
            let whole = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter().image(mark.id(), place, whole, Color32::WHITE);
        }
        ui.add(egui::Label::new(RichText::new("Quill").font(bold(TITLE_SIZE)).color(palette.strong)).selectable(false));
        let home = self.active.is_none();
        if bar_button(ui, "house", home, palette.strong, palette).on_hover_text(crate::keys::text("Home (⌘N)")).clicked() {
            self.active = None;
        }
        self.pours.settle();
        let under = [ui.painter().add(egui::Shape::Noop), ui.painter().add(egui::Shape::Noop)];
        let (mut close, mut pouring, mut draining) = (None, None, None);
        for i in 0..=self.tabs.len() {
            // Human: a tab that was just closed is drawn once more where it was, draining away.
            if let Some((title, drained)) = self.pours.gone_at(i) {
                let look = TabLook { active: false, dirty: false, at_work: false, born: Some(drained) };
                draining = Some(tab_button(ui, &title, look, palette).0.rect);
            }
            if i == self.tabs.len() {
                break;
            }
            // Human: a saved file's tab shows its whole name, ending included.
            let (title, dirty) = (self.tabs[i].tab_name(), self.tabs[i].dirty());
            let born = self.pours.born_at(i);
            let look = TabLook { active: self.active == Some(i), dirty, at_work: self.chat_at_work(i), born };
            let (response, closed) = tab_button(ui, &title, look, palette);
            if born.is_some() {
                pouring = Some(response.rect);
            }
            if closed || response.middle_clicked() {
                close = Some(i);
            } else if response.clicked() {
                self.active = Some(i);
                self.canvas.editing = None;
            }
        }
        let plus = bar_button(ui, "plus", false, palette.soft, palette).on_hover_text(crate::keys::text("New file (⇧⌘N)"));
        self.pours.paint(ui, under, [pouring, draining], plus.rect, self.tabs.len(), palette);
        if plus.clicked() {
            self.new_file();
        }
        if let Some(i) = close {
            self.close_tab(i);
        }
    }

    /// The in-window menu with the File, Edit and View commands.
    fn main_menu(&mut self, ui: &mut egui::Ui, palette: theme::Palette) {
        let button = bar_button(ui, "menu", false, palette.strong, palette).on_hover_text("Menu");
        egui::Popup::menu(&button).show(|ui| {
            ui.menu_button("File", |ui| {
                if menu_item(ui, "New file          ⇧⌘N") {
                    self.new_file();
                }
                if menu_item(ui, "Open…              ⌘O") {
                    self.ask_open();
                }
                if menu_item(ui, "Home                ⌘N") {
                    self.active = None;
                }
                ui.separator();
                if let Some(tab) = self.active {
                    if menu_item(ui, "Save                 ⌘S") {
                        self.save(tab, false);
                    }
                    if menu_item(ui, "Save As…        ⇧⌘S") {
                        self.save_as(tab);
                    }
                    if menu_item(ui, "Export .pen File…") {
                        self.ask_export_pen();
                    }
                    if menu_item(ui, "Close tab         ⌘W") {
                        self.close_tab(tab);
                    }
                }
            });
            let has_editor = self.active.is_some();
            ui.add_enabled_ui(has_editor, |ui| {
                ui.menu_button("Edit", |ui| {
                    if let Some(editor) = self.editor() {
                        if menu_item(ui, "Undo        ⌘Z") {
                            editor.undo();
                        }
                        if menu_item(ui, "Redo      ⇧⌘Z") {
                            editor.redo();
                        }
                        ui.separator();
                        if menu_item(ui, "Duplicate   ⌘D") {
                            editor.duplicate();
                        }
                        if menu_item(ui, "Delete        ⌫") {
                            editor.delete_selection();
                        }
                        if menu_item(ui, "Select all  ⌘A") {
                            editor.select_all();
                        }
                        ui.separator();
                        if menu_item(ui, "Group       ⌘G") {
                            editor.wrap_selection(&pen_doc::Kind::Group);
                        }
                        if menu_item(ui, "Ungroup  ⇧⌘G") {
                            editor.ungroup();
                        }
                        if menu_item(ui, "Frame selection  ⌥⌘G") {
                            editor.wrap_selection(&pen_doc::Kind::Frame);
                        }
                        if menu_item(ui, "Add flex layout  ⇧A") {
                            editor.add_flex_layout();
                        }
                        if menu_item(ui, "Create component  ⌥⌘K") {
                            editor.create_component();
                        }
                    }
                });
            });
            ui.menu_button("View", |ui| {
                if menu_item(ui, "Toggle UI   ⌘\\") {
                    self.hide_ui = !self.hide_ui;
                }
                let view = self.canvas_view;
                if let Some(editor) = self.editor() {
                    if menu_item(ui, "Zoom to fit   ⇧1") {
                        let bounds = editor.content_bounds();
                        canvas::fit(editor, view, bounds, MAX_ZOOM);
                    }
                    if menu_item(ui, "Zoom to 100%  ⌘0") {
                        canvas::zoom_at(editor, view, view.center(), 1.0);
                    }
                }
            });
        });
    }

    fn editor_view(&mut self, ui: &mut egui::Ui) {
        let dark = self.dark();
        let palette = theme::palette(dark);
        let hide = self.hide_ui;
        let Some(index) = self.active else { return };
        let ctx = ui.ctx().clone();
        let measure = text_measure(&ctx);
        if let Some(editor) = self.tabs.get_mut(index) {
            editor.refresh(&measure);
        }
        let mut notice = None;
        let mut asked = None;
        let mut pictures = self.quick_look(index);
        // Human: with "Animations & effects" off the sidebar opens and folds at once.
        let open = self.sidebar.openness(&ctx, self.prefs.get().appearance.animations);
        if !hide {
            // Human: the sidebar's islands float over the window; this empty strip of canvas colour is the room
            // the canvas gives up for them, growing as they grow.
            let room = self.sidebar.room(open);
            if room > 0.0 {
                egui::Panel::left("sidebar")
                    .exact_size(room)
                    .resizable(false)
                    .show_separator_line(false)
                    .frame(egui::Frame::new().fill(palette.canvas))
                    .show(ui, |_| {});
            }
        }
        egui::CentralPanel::default().frame(egui::Frame::new()).show(ui, |ui| {
            let Some(editor) = self.tabs.get_mut(index) else { return };
            // Panel edits made this frame are laid out before the canvas draws.
            editor.refresh(&measure);
            // Human: the canvas fills the window, top bar included; the islands keep to the part under the bar.
            let view = ui.max_rect();
            self.canvas_view = view;
            editor.covered = covered(hide, open, !editor.selection.is_empty());
            (self.canvas.options, self.canvas.animations) = (canvas_options(self.prefs.get()), self.prefs.get().appearance.animations);
            self.canvas.show(ui, editor, dark);
            if !hide {
                let rect = egui::Rect::from_min_max(egui::pos2(view.left(), view.top() + TAB_BAR_HEIGHT), view.max);
                let rail = self.sidebar.rail(ui.ctx(), rect.min + egui::vec2(ISLAND_GAP, ISLAND_GAP), editor, dark);
                asked = rail;
                // Human: with the properties island showing, the message bar keeps to the left of it.
                let selected = !editor.selection.is_empty();
                let beside = if selected { PROPERTIES_WIDTH + ISLAND_GAP } else { 0.0 };
                let under_bar = egui::Rect::from_min_max(egui::pos2(ui.ctx().content_rect().left(), rect.top()), rect.max - egui::vec2(beside, 0.0));
                if let Some(talk) = self.talks.iter().find(|talk| talk.id == self.shown) {
                    let look = talk::look(talk, self.prefs.get(), &self.assistants, self.assistant_asked.as_deref());
                    notice = notice.or(self.sidebar.show(ui.ctx(), under_bar, open, editor, look));
                }
                // Human: the properties island floats over the canvas's right edge, only while something is
                // selected, as in the design; the canvas keeps its size, so nothing jumps when it appears.
                if selected {
                    let island = egui::Rect::from_min_max(
                        egui::pos2(rect.right() - ISLAND_GAP - PROPERTIES_WIDTH, rect.top() + ISLAND_GAP),
                        rect.max - egui::vec2(ISLAND_GAP, ISLAND_GAP),
                    );
                    egui::Area::new(egui::Id::unique("properties")).fade_in(false).fixed_pos(island.min).order(egui::Order::Foreground).show(
                        ui.ctx(),
                        |ui| {
                            let (place, _) = ui.allocate_exact_size(island.size(), egui::Sense::hover());
                            crate::sidebar::island(ui, place, ISLAND_RADIUS, palette);
                            ui.scope_builder(egui::UiBuilder::new().max_rect(place.shrink(1.0)), |ui| {
                                ui.set_clip_rect(place.shrink(1.0).intersect(ui.clip_rect()));
                                notice = notice.or(crate::panels::properties(ui, editor, dark, &mut pictures));
                            });
                        },
                    );
                }
                // Human: the zoom island at the canvas's bottom right corner, left of the properties island: smaller,
                // the zoom (a click sets 100 %), larger.
                let place = egui::Rect::from_min_size(
                    egui::pos2(rect.right() - ISLAND_GAP - beside - ZOOM_ISLAND[0], rect.bottom() - ISLAND_GAP - ZOOM_ISLAND[1]),
                    egui::Vec2::from(ZOOM_ISLAND),
                );
                egui::Area::new(egui::Id::unique("zoom")).fade_in(false).fixed_pos(place.min).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
                    let (island, _) = ui.allocate_exact_size(place.size(), egui::Sense::hover());
                    crate::sidebar::island(ui, island, island.height() / 2.0, palette);
                    let inside = egui::UiBuilder::new()
                        .max_rect(island.shrink2(egui::vec2(6.0, 0.0)))
                        .layout(egui::Layout::left_to_right(egui::Align::Center));
                    ui.scope_builder(inside, |ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        let button = egui::Vec2::splat(TITLE_BUTTON);
                        if icon_button(ui, "minus", button, false, &crate::keys::text("Zoom out (⌘−)"), palette).clicked() {
                            canvas::zoom_at(editor, view, view.center(), canvas::zoom_step(editor.camera.zoom, false));
                        }
                        let label = format!("{:.0}%", editor.camera.zoom * 100.0);
                        let middle = egui::vec2(ui.available_width() - TITLE_BUTTON, TITLE_BUTTON);
                        if text_button(ui, None, &label, middle, false, false, palette)
                            .on_hover_text(crate::keys::text("Zoom to 100% (⌘0)"))
                            .clicked()
                        {
                            canvas::zoom_at(editor, view, view.center(), 1.0);
                        }
                        if icon_button(ui, "plus", button, false, &crate::keys::text("Zoom in (⌘+)"), palette).clicked() {
                            canvas::zoom_at(editor, view, view.center(), canvas::zoom_step(editor.camera.zoom, true));
                        }
                    });
                });
            }
        });
        // The Export section's button: ask where to.
        let wanted = self.tabs.get_mut(index).and_then(|editor| std::mem::take(&mut editor.export.1).then_some(editor.export.0));
        if let Some(options) = wanted {
            self.ask_export(options);
        }
        self.quick_asked(pictures, &ctx);
        self.asked(asked);
        if let Some(sentence) = notice {
            self.say(sentence);
        }
    }

    /// Shows the settings window and saves what it changed.
    fn settings_window(&mut self, ctx: &egui::Context) {
        // Human: a closed window changes nothing, so the settings aren't copied and compared every frame for it.
        if !self.settings.open {
            self.settings.closed();
            return;
        }
        let dark = self.dark();
        let mut prefs = self.prefs.get().clone();
        let facts =
            crate::settings::Facts { assistants: &self.assistants, port: self.link.as_ref().map(|server| server.port), update: self.updates.seen() };
        let notice = self.settings.show(ctx, &mut prefs, &facts, dark);
        if std::mem::take(&mut self.settings.check_now) {
            self.updates.look_now(ctx);
        }
        if prefs.appearance.dark != dark {
            ctx.set_visuals(theme::visuals(prefs.appearance.dark));
        }
        // Human: a provider connected or an assistant renamed shows in the chat's menu at once.
        let assistants_changed = prefs.agents != self.prefs.get().agents;
        self.prefs.update(|p| *p = prefs);
        if assistants_changed {
            self.find_assistants();
            self.tell_link();
        }
        if let Some(sentence) = notice {
            self.say(sentence);
        }
    }

    /// Takes in what a look for a newer version found, and shows the card that offers one.
    fn update_prompt(&mut self, ctx: &egui::Context) {
        let (wanted, skipped) = (self.prefs.get().updates.check, self.prefs.get().updates.skipped.clone());
        let every = crate::update::every(&self.prefs.get().updates.every);
        if let Some(sentence) = self.updates.tick(ctx, wanted, every, &skipped) {
            self.say(sentence);
        }
        // Human: while the question about unsaved designs is open, the card waits: a required update covers the
        // window, and quitting from it asks that question first.
        if self.prompt.is_some() {
            return;
        }
        match self.updates.show(ctx, theme::palette(self.dark()), self.prefs.get().appearance.animations) {
            // Human: the usual way out, so every unsaved design is asked about first.
            Some(crate::update::Answer::Quit) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Some(crate::update::Answer::Get(address)) => {
                if let Err(problem) = crate::update::open_in_browser(&address) {
                    self.warn(problem);
                }
            }
            Some(crate::update::Answer::Install(prepared)) => {
                // Human: the usual way out, so every unsaved design is asked about first; the install starts
                // in `on_exit`, once the window is really closing.
                self.install = Some(prepared);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Some(crate::update::Answer::Skip(version)) => {
                self.prefs.update(|p| p.updates.skipped = version);
            }
            None => {}
        }
    }

    fn close_prompt(&mut self, ctx: &egui::Context) {
        let Some(prompt) = self.prompt else { return };
        let Some(title) = self.tabs.get(prompt.tab).map(Editor::title) else {
            self.prompt = None;
            return;
        };
        let mut choice = None;
        let palette = theme::palette(self.dark());
        let modal = egui::Modal::new(egui::Id::unique("unsaved")).show(ctx, |ui| {
            ui.set_width(theme::DIALOG_WIDTH[0]);
            ui.label(RichText::new(format!("Save changes to “{title}”?")).strong());
            ui.label("Your changes will be lost if you don't save them.");
            ui.add_space(theme::DIALOG_BUTTONS_GAP);
            ui.horizontal(|ui| {
                if ui.add(dialog_primary("Save", palette)).clicked() {
                    choice = Some(1);
                }
                if ui.add(dialog_button("Don't Save")).clicked() {
                    choice = Some(2);
                }
                if ui.add(dialog_button("Cancel")).clicked() {
                    choice = Some(0);
                }
            });
        });
        // Human: Return saves, the highlighted choice; Escape or a click beside the question cancels.
        if choice.is_none() {
            if modal.should_close() {
                choice = Some(0);
            } else if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                choice = Some(1);
            }
        }
        match choice {
            Some(1) => {
                self.prompt = None;
                self.save(prompt.tab, true);
                if prompt.quitting {
                    self.continue_quit(ctx);
                }
            }
            Some(2) => {
                self.prompt = None;
                self.close_tab_now(prompt.tab);
                if prompt.quitting {
                    self.continue_quit(ctx);
                }
            }
            Some(_) => {
                self.prompt = None;
                self.discard_confirmed = false;
                // Human: staying in the app also means not installing the update when it is quit later.
                self.install = None;
            }
            None => {}
        }
    }

    /// Pastes a picture from the clipboard into the document in front: it is written as a PNG file where the
    /// document's pictures are kept (`images/` beside it, or the home folder's until it has a file) and
    /// becomes a rectangle with that picture as its fill.
    // Agent: WRITES the file and its line in the credits in the window's thread; a screenshot takes a few
    // dozen milliseconds to pack.
    fn paste_picture(&mut self, picture: &egui::ColorImage, view_of: &egui::Context) {
        let view = self.canvas_view;
        let Some(editor) = self.editor() else { return };
        let saved = png_of(picture).and_then(|bytes| {
            let credit = serde_json::json!({ "source": "clipboard" });
            crate::imagery::store::save(editor.anchor().as_deref(), crate::imagery::store::home().as_deref(), &bytes, "Pasted picture", &credit)
        });
        match saved {
            Ok(saved) => {
                let visible = editor.visible_area(view.width(), view.height());
                editor.paste_picture(&saved.url, (saved.width, saved.height), visible);
                view_of.request_repaint();
            }
            Err(problem) => self.warn(format!("Couldn't paste the picture: {problem}")),
        }
    }

    /// Hands the documents with unsaved changes to the keeper, and asks for another look when some wait their turn.
    // Human: not in the middle of a drag, which changes the document with every frame; it is kept when the drag ends.
    fn keep_for_recovery(&mut self, ctx: &egui::Context) {
        let Some(keeper) = &mut self.recovery.keeper else { return };
        if self.canvas.dragging() {
            return;
        }
        let now = Instant::now();
        let mut waiting = false;
        for tab in &self.tabs {
            let seen = crate::recover::Seen { id: tab.id, dirty: tab.dirty(), scene: tab.generation, path: tab.path.as_deref(), title: &tab.title() };
            waiting |= keeper.look(&seen, &tab.doc, now);
        }
        if waiting {
            ctx.request_repaint_after(crate::recover::EVERY);
        }
        let trouble = keeper.trouble();
        match trouble {
            Some(problem) if self.troubles.recovery.fails() => self.warn(problem),
            Some(_) => {}
            None => self.troubles.recovery.works(),
        }
    }

    /// Asks what to do with the documents an earlier run left behind when it didn't end properly.
    fn recovery_prompt(&mut self, ctx: &egui::Context) {
        if self.recovery.left.is_empty() {
            return;
        }
        let palette = theme::palette(self.dark());
        let discarding = self.recovery.discarding;
        let mut choice = None;
        // Human: Return takes the highlighted choice: Restore, or Keep when asked whether to discard. Escape steps
        // back from that question, and on the list puts the question off: the copies stay on disk and are
        // offered again at the next start. A click beside it does nothing, so the list isn't lost by accident.
        egui::Modal::new(egui::Id::unique("recovery")).show(ctx, |ui| {
            ui.set_width(theme::DIALOG_WIDTH[1]);
            // Human: Discard deletes the kept copies for good, so it asks once more; Restore is the safe,
            // highlighted choice in both steps.
            // Agent: `choice` is Some(true) restore, Some(false) discard for good; Keep only clears `discarding`.
            if discarding {
                ui.label(RichText::new("Discard the recovered work?").strong());
                ui.label("This can't be undone.");
                ui.add_space(theme::DIALOG_BUTTONS_GAP);
                ui.horizontal(|ui| {
                    if ui.add(dialog_primary("Keep", palette)).clicked() {
                        self.recovery.discarding = false;
                    }
                    if ui.add(dialog_button("Discard")).clicked() {
                        choice = Some(false);
                    }
                });
                return;
            }
            ui.label(RichText::new("Quill didn't close properly last time").strong());
            ui.label(if self.recovery.left.len() == 1 {
                "This design had changes that weren't saved:"
            } else {
                "These designs had changes that weren't saved:"
            });
            ui.add_space(theme::DIALOG_LIST_GAP);
            for left in &self.recovery.left {
                let place = left.path.as_ref().map_or_else(|| "never saved".to_owned(), |path| path.display().to_string());
                ui.label(format!("{} ({place})", left.title));
            }
            ui.add_space(theme::DIALOG_BUTTONS_GAP);
            ui.horizontal(|ui| {
                if ui.add(dialog_primary("Restore", palette)).clicked() {
                    choice = Some(true);
                }
                if ui.add(dialog_button("Discard")).clicked() {
                    self.recovery.discarding = true;
                }
            });
        });
        // Agent: only when no button acted this frame: a button reached with Tab takes Return itself.
        let acted = choice.is_some() || self.recovery.discarding != discarding;
        let (enter, escape) = ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        if !acted && discarding && (enter || escape) {
            self.recovery.discarding = false;
        } else if !acted && enter {
            choice = Some(true);
        } else if !acted && escape {
            self.recovery.left.clear();
            return;
        }
        let Some(restore) = choice else { return };
        self.recovery.discarding = false;
        let left = std::mem::take(&mut self.recovery.left);
        if !restore {
            crate::recover::clear(&left);
            return;
        }
        // Human: one that can't be brought back stays on disk and is offered again at the next start.
        let (mut restored, mut failed) = (Vec::new(), Vec::new());
        for one in left {
            match self.restore_left(&one) {
                Ok(()) => restored.push(one),
                Err(problem) => failed.push(format!("{}: {problem}", one.title)),
            }
        }
        crate::recover::clear(&restored);
        if !failed.is_empty() {
            self.warn(format!("Couldn't restore {}", failed.join("; ")));
        }
    }

    /// Opens one document an earlier run left behind: its file, with the kept document in the place of the
    /// file's, or a new document when it has no file (any more). Its changes count as unsaved.
    fn restore_left(&mut self, left: &crate::recover::Left) -> Result<(), String> {
        let doc = left.document()?;
        if let Some(path) = left.path.as_deref().filter(|path| path.is_file()) {
            self.try_open(path)?;
            self.editor().ok_or("its tab didn't open")?.restore(doc);
        } else {
            let seed = self.next_seed();
            let mut editor = Editor::new(doc, None, seed);
            editor.mark_unsaved();
            self.add_tab(editor);
        }
        Ok(())
    }

    /// After one unsaved question while quitting: ask about the next unsaved tab, or quit.
    fn continue_quit(&mut self, ctx: &egui::Context) {
        if let Some(tab) = self.tabs.iter().position(Editor::dirty) {
            self.prompt = Some(ClosePrompt { tab, quitting: true });
        } else if !self.dialogs.is_open() && self.dialogs.queued() == 0 {
            self.discard_confirmed = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl App {
    /// Makes the faces ready that text asked for, and lays every document out again when some became ready:
    /// its text was measured in another face.
    // Agent: MAIN-THREAD; call before text is laid out in a frame, so egui takes the faces up at once.
    pub(crate) fn settle_fonts(&mut self, ctx: &egui::Context) {
        if fonts::sync(ctx) {
            for tab in &mut self.tabs {
                tab.relayout();
            }
            ctx.request_repaint();
        }
        // Human: egui takes new faces up when a frame begins, and a window that is hidden or behind others
        // gets no frame: only `logic` runs. Faces asked for then (a model opens a document or writes text
        // while the app is in the background) would never become ready, and that text would be measured and
        // drawn in the stand-in. One frame is asked for although nothing shows it.
        if fonts::pending() {
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
            ctx.request_repaint();
        }
    }
}

impl App {
    /// Shows the welcome film in the place of everything else. Returns whether it took this frame.
    /// Takes in the computer's answer whether it asks for less motion, the one time it arrives: "Animations
    /// and effects" follows it, unless the user has set that switch themselves.
    // Agent: WRITES prefs (saved like any other change) only when the switch has to change.
    fn follow_system_motion(&mut self) {
        let Some(reduced) = self.motion.answer() else { return };
        let appearance = &self.prefs.get().appearance;
        let now = crate::motion::Switch {
            on: appearance.animations,
            chosen: appearance.animations_chosen,
            off_for_system: appearance.animations_off_for_system,
        };
        if let Some(on) = crate::motion::follow(now, reduced) {
            self.prefs.update(|prefs| (prefs.appearance.animations, prefs.appearance.animations_off_for_system) = (on, !on));
        }
    }

    // Human: when the user goes on, the film is over for good: the settings remember that it was seen. "Set up
    // an assistant" goes on to the settings window where assistants are set up.
    fn show_film(&mut self, ui: &mut egui::Ui) -> bool {
        self.follow_system_motion();
        let Some(film) = &mut self.film else { return false };
        // Human: the film and its music don't start before it is known whether the computer asks for less
        // motion (a moment, at the very start): until then only the film's ground shows.
        if self.motion.pending() {
            egui::CentralPanel::default().frame(egui::Frame::new()).show(ui, |ui| crate::splash::Splash::hold(ui));
            ui.ctx().request_repaint();
            return true;
        }
        let animations = self.prefs.get().appearance.animations;
        let mut outcome = crate::splash::Outcome::Playing;
        egui::CentralPanel::default().frame(egui::Frame::new()).show(ui, |ui| outcome = film.show(ui, animations));
        if outcome == crate::splash::Outcome::Playing {
            return true;
        }
        self.film = None;
        self.prefs.update(|prefs| prefs.welcome.seen = true);
        if outcome == crate::splash::Outcome::Assistant {
            self.settings.open_named("agents");
        }
        ui.ctx().request_repaint();
        true
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Closing the window (or ⌘Q) asks about unsaved tabs first, one at a time.
        if ctx.input(|i| i.viewport().close_requested())
            && !self.discard_confirmed
            && self.screenshot.is_none()
            && let Some(tab) = self.tabs.iter().position(Editor::dirty)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.active = Some(tab);
            self.prompt = Some(ClosePrompt { tab, quitting: true });
        }
        let tick = self.prefs.tick(Instant::now());
        // Human: the store tries again by itself; the user hears of it once, and again only if it fails after
        // having worked.
        match &tick.status {
            SaveStatus::Failed(err) => {
                log::warn!("settings not saved yet: {err}");
                if self.troubles.settings.fails() {
                    self.warn(format!("Your settings couldn't be saved: {err}. Quill keeps trying."));
                }
            }
            SaveStatus::Saved => self.troubles.settings.works(),
            SaveStatus::Idle | SaveStatus::Pending => {}
        }
        if let Some(deadline) = tick.next_deadline {
            ctx.request_repaint_after(deadline.saturating_duration_since(Instant::now()));
        }
        self.settle_fonts(ctx);
        self.follow_system_look(ctx);
        if let Some(job) = &mut self.screenshot {
            job.frame(ctx);
        }
        self.serve_tools(ctx);
        if self.poll_chats() {
            ctx.request_repaint();
        }
        self.send_waiting(ctx);
        self.log_chats();
        self.keep_for_recovery(ctx);
        self.keep_chats();
        // Last, so it sees the app's CancelClose.
        craft_quit::on_close_request(ctx);
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        // Human: the menu bar takes the shortcuts it shows; this hands them back to egui as the key presses
        // they were, so text fields and the app's own shortcuts see them.
        if let Some(menu) = &mut self.native_menu {
            menu.raw_input(raw_input);
        }
        // Human: egui sizes its font atlas by the largest picture the graphics card takes; a smaller one is
        // enough, and is sent to the graphics card far quicker when it grows.
        // Agent: egui-winit sends the limit once, with the first input; it is cut down as it passes.
        if let Some(side) = &mut raw_input.max_texture_side {
            *side = (*side).min(crate::render::MAX_ATLAS_SIDE);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        resize_edges(&ctx);
        if self.show_film(ui) {
            return;
        }
        self.handle_dialog_answers();
        self.finish_export(&ctx);
        self.native_menus(&ctx);
        self.shortcuts(&ctx);
        let palette = theme::palette(self.dark());
        if self.active.is_some() {
            // Human: over a file the top bar has no background of its own: the two islands float over the canvas,
            // which runs up to the window's top edge behind them.
            let window = ctx.content_rect();
            egui::Area::new(egui::Id::unique("tabs")).fade_in(false).fixed_pos(window.min).order(egui::Order::Foreground).show(&ctx, |ui| {
                let (bar, _) = ui.allocate_exact_size(egui::vec2(window.width(), TAB_BAR_HEIGHT), egui::Sense::hover());
                ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| self.tab_bar(ui));
            });
        } else {
            egui::Panel::top("tabs").exact_size(TAB_BAR_HEIGHT).show_separator_line(false).frame(egui::Frame::new().fill(palette.background)).show(
                ui,
                |ui| {
                    self.tab_bar(ui);
                },
            );
        }
        // Human: the macOS window buttons sit inside the title island, centred on its height.
        frame.set_traffic_lights_position(&ctx, egui::Rangef::new(0.0, TAB_BAR_HEIGHT), TRAFFIC_LIGHT_LEFT);
        if self.active.is_some() {
            self.bring_chat();
            self.editor_view(ui);
        } else {
            egui::CentralPanel::default().frame(egui::Frame::new().fill(palette.background)).show(ui, |ui| self.home(ui));
        }
        self.find_layers(&ctx);
        if let Some(command) = self.actions.show(&ctx, self.dark()) {
            self.run(command, &ctx);
        }
        self.dropped(&ctx);
        self.chat_asks(&ctx);
        self.new_chat_prompt(&ctx);
        self.settings_window(&ctx);
        self.show_picture_dialog(&ctx);
        self.update_prompt(&ctx);
        self.close_prompt(&ctx);
        self.recovery_prompt(&ctx);
        self.toasts.show(&ctx, self.dark(), self.prefs.get().appearance.animations);
        let title = self.active.and_then(|i| self.tabs.get(i)).map_or_else(|| "Quill".to_owned(), |t| format!("{} — Quill", t.title()));
        let current = ctx.input(|i| i.viewport().title.clone());
        if current.as_deref() != Some(title.as_str()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }
        self.dialogs.show_pending(Some(&*frame));
        // Human: text that asked for a face not read yet was drawn in the interface face; one more frame
        // shows it in its own.
        if fonts::pending() {
            ctx.request_repaint();
        }
    }

    fn on_exit(&mut self) {
        if self.link.is_some() {
            crate::link::forget();
        }
        if let Err(e) = self.prefs.flush() {
            log::warn!("settings not saved: {e}");
        }
        // Human: a run that ends properly has nothing to recover: the user was asked about every unsaved document.
        if let Some(keeper) = self.recovery.keeper.take() {
            keeper.finish();
        }
        if let Some(update) = self.install.take()
            && let Err(problem) = update.start()
        {
            log::warn!("{problem}");
        }
    }
}

/// Does a command that only concerns the open document. `view` is the canvas's place on screen.
fn run_on(editor: &mut Editor, command: Command, view: egui::Rect) {
    match command {
        Command::Undo => editor.undo(),
        Command::Redo => editor.redo(),
        Command::Duplicate => editor.duplicate(),
        Command::Delete => editor.delete_selection(),
        Command::Group => editor.wrap_selection(&pen_doc::Kind::Group),
        Command::FrameSelection => editor.wrap_selection(&pen_doc::Kind::Frame),
        Command::Ungroup => editor.ungroup(),
        Command::ToggleLock => editor.toggle_lock(),
        Command::Align(to) => editor.align(to),
        Command::Distribute(across) => editor.distribute(across),
        Command::Flip(horizontal) => editor.flip(horizontal),
        Command::Rotate(degrees) => editor.rotate_selection(degrees),
        Command::Reorder(order) => editor.reorder(order),
        Command::AddLayout => editor.add_flex_layout(),
        Command::RemoveLayout => editor.remove_flex_layout(),
        Command::TextAlign(side) => editor.text_align(side),
        Command::CreateComponent => editor.create_component(),
        Command::DetachComponent => editor.detach_components(),
        Command::DetachInstance => editor.detach_instances(),
        Command::GoToComponent => editor.go_to_component(),
        Command::Deselect => editor.selection.clear(),
        Command::SelectAll => editor.select_all(),
        Command::SelectParent => editor.select_parent(),
        Command::EnterSelection => editor.select_children(),
        Command::ZoomIn => canvas::zoom_at(editor, view, view.center(), canvas::zoom_step(editor.camera.zoom, true)),
        Command::ZoomOut => canvas::zoom_at(editor, view, view.center(), canvas::zoom_step(editor.camera.zoom, false)),
        Command::ZoomTo(zoom) => canvas::zoom_at(editor, view, view.center(), zoom),
        Command::ZoomToFit => {
            let bounds = editor.content_bounds();
            canvas::fit(editor, view, bounds, MAX_ZOOM);
        }
        Command::ZoomToSelection => {
            let bounds = editor.selection_bounds().or_else(|| editor.content_bounds());
            canvas::fit(editor, view, bounds, MAX_ZOOM);
        }
        Command::Tool(tool) => editor.tool = tool,
        // These concern the app, not one document; `App::run` does them.
        Command::Dashboard
        | Command::Save
        | Command::SaveAs
        | Command::ExportPen
        | Command::NewFile
        | Command::OpenFile
        | Command::CloseTab
        | Command::Import
        | Command::GenerateImage
        | Command::Copy
        | Command::Cut
        | Command::Paste
        | Command::Show(_)
        | Command::Export(_)
        | Command::OpenRecent(_)
        | Command::ShowTab(_)
        | Command::PreviousTab
        | Command::NextTab
        | Command::MoveTab(_)
        | Command::ToggleTheme
        | Command::FullScreen
        | Command::Actions
        | Command::FindLayers
        | Command::Settings
        | Command::CheckForUpdates
        | Command::ToggleInterface => {}
    }
}

/// What the floating islands cover of the canvas's left, top, right and bottom edge, so zoom-to-fit keeps the
/// design clear of them: the tool rail at the left, the top bar's islands (they stay with the interface
/// `hidden`), the message bar at the bottom while the sidebar is folded (`open` under a half), and the properties
/// island at the right while something is `selected`.
fn covered(hidden: bool, open: f32, selected: bool) -> [f32; 4] {
    if hidden {
        return [0.0, TAB_BAR_HEIGHT, 0.0, 0.0];
    }
    let bar = if open < 0.5 { ISLAND_GAP + MESSAGE_BAR_HEIGHT } else { 0.0 };
    let properties = if selected { ISLAND_GAP + PROPERTIES_WIDTH } else { 0.0 };
    [ISLAND_GAP + RAIL_WIDTH, TAB_BAR_HEIGHT, properties, bar]
}

/// A tab in the title island: its button, and whether it was closed with the cross.
// Human: the folder in front of the name turns into a cross while the pointer is on the tab.
/// The key a shortcut is looked up by, for a press of `key` (what the keyboard layout calls it) at the place
/// `physical` (what a US keyboard has there).
// Human: a Mac goes by the place, since ⌥ changes what a key types there. Windows and Linux go by the letter
// the layout gives, so Ctrl+Z is the key marked Z on a German keyboard too; other keys go by their place, so
// ⇧1 is the 1 key whatever sign it types with Shift.
fn shortcut_key(key: Key, physical: Option<Key>) -> Key {
    let letter = key.name().len() == 1 && key.name().starts_with(|sign: char| sign.is_ascii_alphabetic());
    if letter && !cfg!(target_os = "macos") { key } else { physical.unwrap_or(key) }
}

/// One line of the in-window menu, written the Mac way (`Save     ⌘S`); whether it was clicked.
// Human: a Mac shows the line as written. Elsewhere the shortcut is spelled out (Ctrl+S) at the line's right edge.
fn menu_item(ui: &mut egui::Ui, line: &str) -> bool {
    if cfg!(target_os = "macos") {
        return ui.button(line).clicked();
    }
    let (name, keys) = crate::keys::menu_line(line);
    ui.add(egui::Button::new(name).shortcut_text(crate::keys::text(keys).into_owned())).clicked()
}

/// How a tab shows.
#[derive(Clone, Copy)]
struct TabLook {
    /// It is the one in front.
    active: bool,
    /// Its document has changes that are not saved.
    dirty: bool,
    /// Its chat's assistant is at work.
    at_work: bool,
    /// How it is drawn while it pours in or drains away; `None` for a tab that is simply there.
    born: Option<pour::Look>,
}

// Human: each document has its own chat, and one can work while another document is in front. A dot in the
// assistant's colour in place of the tab's icon says that this document's assistant is at work.
fn tab_button(ui: &mut egui::Ui, title: &str, look: TabLook, palette: theme::Palette) -> (egui::Response, bool) {
    let TabLook { active, dirty, at_work, born } = look;
    let text = if dirty { format!("{title} •") } else { title.to_owned() };
    let color = if active { palette.strong } else { palette.soft };
    let galley = ui.painter().layout_no_wrap(text, bold(TITLE_TEXT), color);
    let [end, gap] = TAB_PADDING;
    let full = end + TAB_ICON + gap + galley.size().x + end;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(born.map_or(full, |look| full * look.width), TAB_HEIGHT), egui::Sense::click());
    let hovered = response.hovered();
    // Human: while a tab pours in, the liquid is its body, and its icon and name come in inside what is there of it.
    let mut painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.set_opacity(born.map_or(1.0, |look| look.shown));
    if born.is_none() && (active || hovered) {
        ui.painter().rect_filled(
            rect,
            f32::from(theme::CONTROL_RADIUS),
            if active { palette.field } else { palette.field.gamma_multiply(theme::OUTLINE_SOFT) },
        );
    }
    let slide = born.map_or(0.0, |look| look.slide);
    let slot = egui::Rect::from_min_size(egui::pos2(rect.left() + end + slide, rect.center().y - TAB_ICON / 2.0), egui::Vec2::splat(TAB_ICON));
    if at_work && !hovered {
        painter.circle_filled(slot.center(), TAB_WORK_DOT / 2.0, theme::AGENT);
    } else {
        crate::icons::paint(&painter, slot, if hovered { "x" } else { "folder" }, color);
    }
    painter.galley(egui::pos2(slot.right() + gap, rect.center().y - galley.size().y / 2.0), galley, color);
    let on_cross = response.interact_pointer_pos().is_some_and(|p| slot.expand(5.0).contains(p));
    let closed = response.clicked() && on_cross;
    (response.on_hover_text(if at_work { format!("{title}: the assistant is working") } else { title.to_owned() }), closed)
}

/// The canvas's settings, from the user's.
fn canvas_options(prefs: &Prefs) -> canvas::Options {
    let agent_cursor = match prefs.agents.cursor_size.as_str() {
        "Medium" => AGENT_CURSOR_MEDIUM,
        "Large" => AGENT_CURSOR_LARGE,
        _ => 1.0,
    };
    canvas::Options {
        snap_to_pixels: prefs.canvas.snap_to_pixels,
        snap_to_objects: prefs.canvas.snap_to_objects,
        wheel_zooms: prefs.canvas.wheel_zooms,
        invert_zoom: prefs.canvas.invert_zoom,
        agent_cursor,
    }
}

/// Text measurement for the layout, with the same fonts the canvas draws with.
fn text_measure(ctx: &egui::Context) -> impl Fn(&pen_doc::layout::TextStyle) -> (f32, f32) + '_ {
    move |style| {
        if style.content.is_empty() {
            return (0.0, style.line_px.unwrap_or(style.size * 1.2));
        }
        let job = crate::render::text_job(style, Color32::WHITE, 1.0);
        let galley = ctx.fonts_mut(|f| f.layout_job(job));
        let size = galley.size();
        (size.x, size.y)
    }
}

/// Nodes from clipboard text this app wrote.
pub fn parse_clipboard(text: &str) -> Option<Vec<Node>> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    serde_json::from_value(value.get(CLIPBOARD_KEY)?.clone()).ok()
}

/// A picture from the clipboard as the bytes of a PNG file.
///
/// # Errors
/// A sentence when the picture has no size or can't be packed.
fn png_of(picture: &egui::ColorImage) -> Result<Vec<u8>, String> {
    let (Ok(width), Ok(height)) = (u32::try_from(picture.size[0]), u32::try_from(picture.size[1])) else {
        return Err("it is too large".to_owned());
    };
    let pixels: Vec<u8> = picture.pixels.iter().flat_map(egui::Color32::to_srgba_unmultiplied).collect();
    let straight = image::RgbaImage::from_raw(width, height, pixels).filter(|_| width > 0 && height > 0).ok_or("it is empty")?;
    let mut bytes = std::io::Cursor::new(Vec::new());
    straight.write_to(&mut bytes, image::ImageFormat::Png).map_err(|error| error.to_string())?;
    Ok(bytes.into_inner())
}

/// Pasting plain text makes a text layer with it.
// Human: plain text becomes a text layer at 0, 0 with font size 12, family
// "Inter" and fill #000000, which then goes through the same placement as any paste.
// Agent: the layer has no measured size yet, so the middle-of-view placement centres its top-left corner.
fn paste_text(editor: &mut Editor, text: &str, visible: pen_doc::geom::Rect) {
    let mut node = Node::new(pen_doc::Kind::Text, editor.fresh_id());
    node.content = Some(text.to_owned());
    node.fill = Some(pen_doc::model::Paints::One(pen_doc::model::Paint::Solid("#000000".to_owned())));
    node.font_size = Some(pen_doc::value::Num::new(12.0));
    node.font_family = Some("Inter".to_owned());
    crate::ops::set_position(&mut node, 0.0, 0.0);
    editor.paste(&[node], visible);
}

/// Shared slot a screenshot callback fills from another thread.
pub type ShotSlot = Arc<Mutex<Option<Arc<egui::ColorImage>>>>;

/// Locks a mutex even if a panicking thread left it poisoned (the data is a plain picture).
pub fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_at_the_windows_edge_resizes_it_that_way_and_at_a_corner_both_ways() {
        use egui::ResizeDirection as To;
        let window = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let at = |x: f32, y: f32| edge_at(window, egui::pos2(x, y));
        assert_eq!(at(400.0, 300.0), None);
        assert_eq!(at(400.0, theme::WINDOW_EDGE + 1.0), None, "the tab strip under the top edge is not the edge");
        assert_eq!(at(400.0, 2.0), Some(To::North));
        assert_eq!(at(400.0, 598.0), Some(To::South));
        assert_eq!(at(2.0, 300.0), Some(To::West));
        assert_eq!(at(798.0, 300.0), Some(To::East));
        assert_eq!(at(2.0, 2.0), Some(To::NorthWest));
        assert_eq!(at(790.0, 2.0), Some(To::NorthEast), "along the top edge, near the corner");
        assert_eq!(at(2.0, 590.0), Some(To::SouthWest));
        assert_eq!(at(798.0, 598.0), Some(To::SouthEast));
        assert_eq!(at(900.0, 300.0), None, "outside the window");
    }
}
