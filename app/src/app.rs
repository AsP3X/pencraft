//! The window: tab strip, home screen, editor layout, file commands, keyboard shortcuts and quitting.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

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
    self, ACCENT, AGENT_CURSOR_LARGE, AGENT_CURSOR_MEDIUM, ICON_SIZE, ISLAND_GAP, ISLAND_RADIUS, MESSAGE_BAR_HEIGHT, PROPERTIES_WIDTH, RAIL_WIDTH,
    SIDEBAR_TEXT_SIZE, TAB_BAR_HEIGHT, TAB_HEIGHT, TAB_WORK_DOT, TITLE_BUTTON, TITLE_ISLAND_HEIGHT, TITLE_ISLAND_TOP, TITLE_SIZE, TRAFFIC_LIGHT_LEFT,
    TRAFFIC_LIGHT_SPACE, ZOOM_ISLAND,
};
use crate::{fonts, samples, screenshot};

mod talk;
mod tools;

/// What to do with a file dialog's answer.
enum Then {
    /// Open the chosen files.
    Open,
    /// Save tab `tab` under the chosen name; close it afterwards when `close` (part of closing or quitting).
    SaveAs { tab: usize, close: bool },
    /// Add the chosen pictures and SVG files to the document.
    Import,
    /// Attach the chosen files to the chat message being written.
    Attach,
    /// Export the selection, as these options say, to the chosen file or into the chosen folder.
    Export(crate::export::Options),
}

/// An unsaved-changes question waiting for an answer.
#[derive(Clone, Copy)]
struct ClosePrompt {
    tab: usize,
    quitting: bool,
}

/// The app's name in the menu bar ("Quit Pencraft").
const APP_NAME: &str = "Pencraft";

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
    /// `--settings <page>`: the screenshot shows the settings window on `general`, `agents` or `provider`.
    pub settings: Option<String>,
    /// `--select <id>[,<id>…]`: the layers selected in the picture, to show their properties.
    pub select: Option<String>,
    /// `--actions`: the picture shows the action search open.
    pub actions: bool,
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
}

impl Launch {
    /// Whether this run may ask GitHub for a newer version and offer it.
    // Human: a picture, a private run and one an AI client started in the background stay offline and show
    // nothing nobody asked for. `pencraft mcp` never gets this far.
    #[must_use]
    pub fn looks_for_updates(&self) -> bool {
        self.screenshot.is_none() && !self.private && !self.background && !self.no_update_check
    }
}

/// The whole app.
pub struct App {
    prefs: SettingsStore<Prefs>,
    tabs: Vec<Editor>,
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
    message: Option<(String, Instant)>,
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
    /// Nothing of this run is remembered (`--private`, or a picture).
    private: bool,
    /// Tool calls that wait for the user to finish a drag, each with when it came in.
    waiting: Vec<(crate::link::Call, Instant)>,
    /// The top-level layers models have changed since a reviewer last looked, each with its document's id and
    /// how many layers were written in it.
    pub(crate) designed: Vec<(u64, String, usize)>,
    /// What is known about a newer version of the app, and the card that offers it.
    updates: crate::update::Updates,
}

/// How long a message stays at the bottom of the window.
const MESSAGE_TIME: Duration = Duration::from_secs(6);
/// The clipboard text format: a JSON object with this key holding the copied nodes.
const CLIPBOARD_KEY: &str = "pencraftNodes";

impl App {
    /// Builds the app from the eframe creation context.
    pub fn new(cc: &eframe::CreationContext<'_>, launch: Launch) -> App {
        fonts::install(&cc.egui_ctx);
        // Human: shader effects run on the same GPU device the window is drawn with.
        if let Some(state) = &cc.wgpu_render_state {
            crate::gpu::install(state.device.clone(), state.queue.clone());
        }
        let location = if launch.screenshot.is_some() || launch.private { Location::Memory } else { Location::Auto };
        let (prefs, report) = SettingsStore::<Prefs>::open(location);
        if report.has_problems() {
            log::warn!("settings: some values were reset: {report:?}");
        }
        let ctx = cc.egui_ctx.clone();
        let dialogs = Dialogs::new(Arc::new(move || ctx.request_repaint()));
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(7, |d| u64::from(d.subsec_nanos()) ^ d.as_secs());
        let (tool_send, tool_calls) = std::sync::mpsc::channel();
        let mut app = App {
            prefs,
            tabs: Vec::new(),
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
            message: None,
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
            private: launch.private || launch.screenshot.is_some(),
            waiting: Vec::new(),
            designed: Vec::new(),
            updates: crate::update::Updates::new(launch.looks_for_updates(), launch.update_prompt),
        };
        app.find_assistants();
        if launch.screenshot.is_none() {
            app.start_link(&cc.egui_ctx);
            app.install_menus(&cc.egui_ctx);
        }
        if let Some(path) = launch.screenshot {
            // Human: the picture shows the files named on the command line, or without any the editor with the
            // starter board, one card selected.
            for file in &launch.files {
                app.open_path(file);
            }
            if app.tabs.is_empty() {
                app.open_sample();
                if let Some(editor) = app.tabs.last_mut() {
                    editor.selection = vec!["card2".to_owned()];
                }
            }
            // Human: a picture can ask for the dark look and a sidebar view; neither is saved (settings are in
            // memory for a screenshot run).
            if launch.dark {
                app.prefs.update(|p| p.appearance.dark = true);
            }
            if let Some(name) = launch.settings.as_deref() {
                app.settings.open_named(name);
            }
            if let (Some(ids), Some(editor)) = (launch.select, app.tabs.first_mut()) {
                editor.selection = ids.split(',').map(str::to_owned).collect();
                editor.pose_points(launch.points);
            }
            app.pose_extras(launch.actions, launch.find.as_deref(), launch.attach.clone());
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
        // Human: `--sidebar chat` opens the app, or takes its picture, with that view showing.
        if let Some(name) = launch.sidebar.as_deref() {
            app.pose_sidebar(name);
        }
        if let Some(text) = launch.ask {
            app.sidebar.chat_asks.push(crate::sidebar::ChatAsk::Send { shown: text.clone(), told: text, files: launch.attach });
        }
        cc.egui_ctx.set_visuals(theme::visuals(app.dark()));
        app
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
        if name == "conversation" {
            self.front_talk().chat.entries = crate::chat::sample();
        }
        self.sidebar.pose(if name == "conversation" { "chat" } else { name });
    }

    /// Starts the local server AI models reach the app through.
    // Human: a tool call wakes the app, also while its window is hidden or behind others.
    fn start_link(&mut self, ctx: &egui::Context) {
        let wake = ctx.clone();
        match crate::link::start(self.tool_send.clone(), move || wake.request_repaint()) {
            Ok(server) => {
                log::info!("AI tools: listening on 127.0.0.1:{}", server.port);
                self.link = Some(server);
            }
            Err(error) => log::warn!("AI tools: the local server didn't start: {error}"),
        }
    }

    fn dark(&self) -> bool {
        self.prefs.get().appearance.dark
    }

    fn next_seed(&mut self) -> u64 {
        self.seed = self.seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.seed
    }

    fn say(&mut self, text: impl Into<String>) {
        self.message = Some((text.into(), Instant::now()));
    }

    /// Does what the tool rail asked for; a sentence for the user when that is all there is to do.
    fn asked(&mut self, asked: Option<Ask>) -> Option<&'static str> {
        match asked? {
            Ask::Say(sentence) => return Some(sentence),
            Ask::Import => self.ask_import(),
            Ask::Actions => self.actions.toggle(),
            Ask::Find => self.open_find(),
        }
        None
    }

    /// For a picture of the app: the action search open (`--actions`), find layers with a query (`--find`),
    /// files attached to the chat message (`--attach`).
    fn pose_extras(&mut self, actions: bool, find: Option<&str>, attach: Vec<PathBuf>) {
        self.attach(attach);
        if actions {
            self.actions.toggle();
        }
        if let Some(query) = find {
            self.open_find();
            self.find.pose(query);
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
            self.say(error);
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
        let doc = std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|text| pen_doc::read(&text).map_err(|e| e.to_string()))
            .map_err(|e| format!("Couldn't open {}: {e}", path.display()))?;
        let seed = self.next_seed();
        self.add_tab(Editor::new(doc, Some(path.to_path_buf()), seed));
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
            Err(e) => self.say(format!("The starter board didn't load: {e}")),
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
        self.dialogs.ask(FileDialog::open_many().title("Open").filter("Design files", &["pen"]), Then::Open);
    }

    fn save(&mut self, tab: usize, close: bool) {
        let Some(editor) = self.tabs.get_mut(tab) else { return };
        if editor.path.is_none() {
            let name = format!("{}.pen", editor.title());
            self.dialogs.ask(FileDialog::save().title("Save").filter("Design files", &["pen"]).file_name(name), Then::SaveAs { tab, close });
            return;
        }
        match editor.save() {
            Ok(()) => {
                if let Some(path) = editor.path.as_ref().map(|p| p.display().to_string()) {
                    self.prefs.update(|p| p.files.touch(&path));
                }
                if close {
                    self.close_tab_now(tab);
                }
            }
            Err(e) => self.say(format!("Couldn't save: {e}")),
        }
    }

    fn save_as(&mut self, tab: usize) {
        let Some(editor) = self.tabs.get(tab) else { return };
        let name = format!("{}.pen", editor.title());
        self.dialogs.ask(FileDialog::save().title("Save As").filter("Design files", &["pen"]).file_name(name), Then::SaveAs { tab, close: false });
    }

    /// Closes a tab, asking first when it has unsaved changes.
    fn close_tab(&mut self, tab: usize) {
        if self.tabs.get(tab).is_some_and(|t| t.dirty) {
            self.prompt = Some(ClosePrompt { tab, quitting: false });
        } else {
            self.close_tab_now(tab);
        }
    }

    fn close_tab_now(&mut self, tab: usize) {
        if tab >= self.tabs.len() {
            return;
        }
        let closed = self.tabs.remove(tab);
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
                let dark = !self.dark();
                self.prefs.update(|p| p.appearance.dark = dark);
                ctx.set_visuals(theme::visuals(dark));
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
        menu.set_gate(craft_menu::Gate { modal: self.prompt.is_some(), recording_shortcut: false, text_editing: ctx.egui_wants_keyboard_input() });
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
                    if let (Some(path), Some(editor)) = (paths.into_iter().next(), self.tabs.get_mut(tab)) {
                        let path = if path.extension().is_some() { path } else { path.with_extension("pen") };
                        editor.path = Some(path);
                        self.save(tab, close);
                        self.remember_tabs();
                    }
                }
                (Then::Export(options), Answer::Paths(paths)) => self.exporting = paths.into_iter().next().map(|path| (path, options)),
                (Then::Attach, Answer::Paths(paths)) => self.attach(paths),
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
                (_, Answer::Unavailable(why)) => self.say(format!("The file dialog couldn't open: {why}")),
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
        let clipboard_events: Vec<egui::Event> =
            ctx.input(|i| i.events.iter().filter(|e| matches!(e, egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_))).cloned().collect());
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
                    if nodes.is_empty() && !text.trim().is_empty() {
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
            (Key::K, Tool::Scale),
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

    /// The top bar: the title island with the app's name, home and the open files, and a small island with the
    /// appearance switch and the menu at the other end.
    fn tab_bar(&mut self, ui: &mut egui::Ui) {
        let palette = theme::palette(self.dark());
        let bar = ui.max_rect();
        // Human: the empty part of the bar moves the window, as the title bar it replaces did.
        let drag = ui.interact(bar, ui.scope_id().with("window-drag"), egui::Sense::click_and_drag());
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if drag.double_clicked() {
            let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
        }
        let row = egui::Rect::from_min_size(
            bar.min + egui::vec2(ISLAND_GAP, TITLE_ISLAND_TOP),
            egui::vec2(bar.width() - 2.0 * ISLAND_GAP, TITLE_ISLAND_HEIGHT),
        );
        let island = egui::Frame::new()
            .fill(palette.island)
            .stroke(egui::Stroke::new(1.0, palette.outline))
            .corner_radius(TITLE_ISLAND_HEIGHT / 2.0)
            .inner_margin(egui::Margin::symmetric(8, 0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::left_to_right(egui::Align::Center)), |ui| {
            island.show(ui, |ui| {
                ui.set_height(TITLE_ISLAND_HEIGHT - 2.0);
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space(if cfg!(target_os = "macos") { TRAFFIC_LIGHT_SPACE - 8.0 } else { 8.0 });
                ui.add(egui::Label::new(RichText::new("Pencraft").size(TITLE_SIZE).color(palette.foreground)).selectable(false));
                ui.add_space(10.0);
                let home = self.active.is_none();
                if icon_button(ui, "house", egui::Vec2::splat(TITLE_BUTTON), home, palette).on_hover_text(crate::keys::text("Home (⌘N)")).clicked()
                {
                    self.active = None;
                }
                let (line, _) = ui.allocate_exact_size(egui::vec2(9.0, 18.0), egui::Sense::hover());
                ui.painter().vline(line.center().x, line.y_range(), egui::Stroke::new(1.0, palette.outline));
                let mut close = None;
                for i in 0..self.tabs.len() {
                    // Human: a saved file's tab shows its whole name, ending included.
                    let (title, dirty) = (self.tabs[i].tab_name(), self.tabs[i].dirty);
                    let look = TabLook { active: self.active == Some(i), dirty, at_work: self.chat_at_work(i) };
                    let (response, closed) = tab_button(ui, &title, look, palette);
                    if closed || response.middle_clicked() {
                        close = Some(i);
                    } else if response.clicked() {
                        self.active = Some(i);
                        self.canvas.editing = None;
                    }
                }
                if icon_button(ui, "plus", egui::Vec2::splat(TITLE_BUTTON), false, palette)
                    .on_hover_text(crate::keys::text("New file (⇧⌘N)"))
                    .clicked()
                {
                    self.new_file();
                }
                if let Some(i) = close {
                    self.close_tab(i);
                }
            });
        });
        ui.scope_builder(egui::UiBuilder::new().max_rect(row).layout(egui::Layout::right_to_left(egui::Align::Center)), |ui| {
            island.show(ui, |ui| {
                ui.set_height(TITLE_ISLAND_HEIGHT - 2.0);
                ui.spacing_mut().item_spacing.x = 4.0;
                // Human: right to left, so the menu ends up first and the globe last.
                for (icon, hint, sentence) in
                    [("globe", "Publish", "Publishing isn't available yet."), ("play", "Present", "Presenting isn't available yet.")]
                {
                    if icon_button(ui, icon, egui::Vec2::splat(TITLE_BUTTON), false, palette).on_hover_text(hint).clicked() {
                        self.say(sentence);
                    }
                }
                if text_button(ui, Some("share"), "Share", egui::vec2(68.0, TITLE_BUTTON), false, false, palette).clicked() {
                    self.say("Sharing isn't available yet.");
                }
                let (line, _) = ui.allocate_exact_size(egui::vec2(9.0, 18.0), egui::Sense::hover());
                ui.painter().vline(line.center().x, line.y_range(), egui::Stroke::new(1.0, palette.outline));
                if icon_button(ui, "settings", egui::Vec2::splat(TITLE_BUTTON), false, palette)
                    .on_hover_text(crate::keys::text("Settings (⌘,)"))
                    .clicked()
                {
                    self.settings.open_general();
                }
                self.main_menu(ui);
            });
        });
    }

    /// The in-window menu with the File, Edit and View commands.
    fn main_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("☰", |ui| {
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

    fn home(&mut self, ui: &mut egui::Ui) {
        let palette = theme::palette(self.dark());
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(56.0);
                ui.label(RichText::new("Pencraft").size(30.0).strong());
                ui.label(RichText::new("Design on an infinite canvas. Files stay .pen documents on your disk.").color(palette.muted));
                ui.add_space(24.0);
                ui.horizontal(|ui| {
                    let width = 360.0;
                    ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
                    let primary = egui::Button::new(RichText::new("New design").color(palette.primary_text))
                        .fill(palette.primary)
                        .min_size(egui::vec2(170.0, 36.0));
                    if ui.add(primary).clicked() {
                        self.new_file();
                    }
                    if ui.add(egui::Button::new("Open…").min_size(egui::vec2(170.0, 36.0))).clicked() {
                        self.ask_open();
                    }
                });
                ui.add_space(32.0);
                ui.label(RichText::new("Start from").strong());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let width = 2.0 * 220.0 + 16.0;
                    ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
                    if starter_card(ui, "Blank canvas", "An empty document", palette, None).clicked() {
                        self.new_file();
                    }
                    ui.add_space(16.0);
                    if starter_card(ui, "Starter board", "Layouts, a component, themes", palette, Some(ACCENT)).clicked() {
                        self.open_sample();
                    }
                });
                ui.add_space(32.0);
                let recent = self.prefs.get().files.recent.clone();
                if !recent.is_empty() {
                    ui.label(RichText::new("Recent").strong());
                    ui.add_space(6.0);
                    let mut open = None;
                    for path in recent {
                        let name = Path::new(&path).file_stem().map_or_else(|| path.clone(), |s| s.to_string_lossy().into_owned());
                        let response = ui.add(egui::Button::new(RichText::new(&name)).frame(false)).on_hover_text(&path);
                        if response.clicked() {
                            open = Some(PathBuf::from(&path));
                        }
                    }
                    if let Some(path) = open {
                        self.open_path(&path);
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
            self.canvas_view = ui.max_rect();
            editor.covered = covered(hide, open, !editor.selection.is_empty());
            self.canvas.options = canvas_options(self.prefs.get());
            self.canvas.show(ui, editor, dark);
            if !hide {
                let rect = ui.max_rect();
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
                                notice = notice.or(crate::panels::properties(ui, editor, dark));
                            });
                        },
                    );
                }
                // Human: the zoom island at the canvas's bottom right corner, left of the properties island: smaller,
                // the zoom (a click sets 100 %), larger.
                let view = rect;
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
                        if icon_button(ui, "minus", button, false, palette).on_hover_text(crate::keys::text("Zoom out (⌘−)")).clicked() {
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
                        if icon_button(ui, "plus", button, false, palette).on_hover_text(crate::keys::text("Zoom in (⌘+)")).clicked() {
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
        notice = notice.or(self.asked(asked));
        if let Some(sentence) = notice {
            self.say(sentence);
        }
    }

    /// Shows the settings window and saves what it changed.
    fn settings_window(&mut self, ctx: &egui::Context) {
        let dark = self.dark();
        let mut prefs = self.prefs.get().clone();
        let notice = self.settings.show(ctx, &mut prefs, dark);
        if prefs.appearance.dark != dark {
            ctx.set_visuals(theme::visuals(prefs.appearance.dark));
        }
        self.prefs.update(|p| *p = prefs);
        if let Some(sentence) = notice {
            self.say(sentence);
        }
    }

    /// Takes in what a look for a newer version found, and shows the card that offers one.
    fn update_prompt(&mut self, ctx: &egui::Context) {
        let (wanted, skipped) = (self.prefs.get().updates.check, self.prefs.get().updates.skipped.clone());
        if let Some(sentence) = self.updates.tick(ctx, wanted, &skipped) {
            self.say(sentence);
        }
        match self.updates.show(ctx, theme::palette(self.dark())) {
            Some(crate::update::Answer::Get(address)) => {
                if let Err(problem) = crate::update::open_in_browser(&address) {
                    self.say(problem);
                }
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
        egui::Modal::new(egui::Id::unique("unsaved")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(RichText::new(format!("Save changes to “{title}”?")).strong());
            ui.label("Your changes will be lost if you don't save them.");
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    choice = Some(1);
                }
                if ui.button("Don't Save").clicked() {
                    choice = Some(2);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(0);
                }
            });
        });
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
            }
            None => {}
        }
    }

    /// After one unsaved question while quitting: ask about the next unsaved tab, or quit.
    fn continue_quit(&mut self, ctx: &egui::Context) {
        if let Some(tab) = self.tabs.iter().position(|t| t.dirty) {
            self.prompt = Some(ClosePrompt { tab, quitting: true });
        } else if !self.dialogs.is_open() && self.dialogs.queued() == 0 {
            self.discard_confirmed = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn message(&mut self, ctx: &egui::Context) {
        let Some((text, since)) = self.message.clone() else { return };
        if since.elapsed() > MESSAGE_TIME {
            self.message = None;
            return;
        }
        ctx.request_repaint_after(MESSAGE_TIME.saturating_sub(since.elapsed()));
        let palette = theme::palette(self.dark());
        egui::Area::new(egui::Id::unique("message")).anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -90.0)).order(egui::Order::Tooltip).show(
            ctx,
            |ui| {
                egui::Frame::new().fill(palette.primary).corner_radius(8.0).inner_margin(10.0).show(ui, |ui| {
                    ui.label(RichText::new(text).color(palette.primary_text));
                });
            },
        );
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
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Closing the window (or ⌘Q) asks about unsaved tabs first, one at a time.
        if ctx.input(|i| i.viewport().close_requested())
            && !self.discard_confirmed
            && self.screenshot.is_none()
            && let Some(tab) = self.tabs.iter().position(|t| t.dirty)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.active = Some(tab);
            self.prompt = Some(ClosePrompt { tab, quitting: true });
        }
        let tick = self.prefs.tick(Instant::now());
        if let SaveStatus::Failed(err) = &tick.status {
            log::warn!("settings not saved yet: {err}");
        }
        if let Some(deadline) = tick.next_deadline {
            ctx.request_repaint_after(deadline.saturating_duration_since(Instant::now()));
        }
        self.settle_fonts(ctx);
        if let Some(job) = &mut self.screenshot {
            job.frame(ctx);
        }
        self.serve_tools(ctx);
        if self.poll_chats() {
            ctx.request_repaint();
        }
        self.log_chats();
        // Last, so it sees the app's CancelClose.
        craft_quit::on_close_request(ctx);
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        // Human: the menu bar takes the shortcuts it shows; this hands them back to egui as the key presses
        // they were, so text fields and the app's own shortcuts see them.
        if let Some(menu) = &mut self.native_menu {
            menu.raw_input(raw_input);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_dialog_answers();
        self.finish_export(&ctx);
        self.native_menus(&ctx);
        self.shortcuts(&ctx);
        let palette = theme::palette(self.dark());
        egui::Panel::top("tabs")
            .exact_size(TAB_BAR_HEIGHT)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(if self.active.is_some() { palette.canvas } else { palette.background }))
            .show(ui, |ui| {
                self.tab_bar(ui);
            });
        // Human: the macOS window buttons sit inside the title island, centred on its height.
        frame.set_traffic_lights_position(&ctx, egui::Rangef::new(TITLE_ISLAND_TOP, TITLE_ISLAND_TOP + TITLE_ISLAND_HEIGHT), TRAFFIC_LIGHT_LEFT);
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
        self.settings_window(&ctx);
        self.update_prompt(&ctx);
        self.close_prompt(&ctx);
        self.message(&ctx);
        let title = self.active.and_then(|i| self.tabs.get(i)).map_or_else(|| "Pencraft".to_owned(), |t| format!("{} — Pencraft", t.title()));
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
        Command::Align(to) => editor.align(to),
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
        | Command::NewFile
        | Command::OpenFile
        | Command::CloseTab
        | Command::Import
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
/// design clear of them: the tool rail at the left, the message bar at the bottom while the sidebar is folded
/// (`open` under a half), and the properties island at the right while something is `selected`.
fn covered(hidden: bool, open: f32, selected: bool) -> [f32; 4] {
    if hidden {
        return [0.0; 4];
    }
    let bar = if open < 0.5 { ISLAND_GAP + MESSAGE_BAR_HEIGHT } else { 0.0 };
    let properties = if selected { ISLAND_GAP + PROPERTIES_WIDTH } else { 0.0 };
    [ISLAND_GAP + RAIL_WIDTH, 0.0, properties, bar]
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
}

// Human: each document has its own chat, and one can work while another document is in front. A dot in the
// assistant's colour in place of the tab's icon says that this document's assistant is at work.
fn tab_button(ui: &mut egui::Ui, title: &str, look: TabLook, palette: theme::Palette) -> (egui::Response, bool) {
    let TabLook { active, dirty, at_work } = look;
    let text = if dirty { format!("{title} •") } else { title.to_owned() };
    let color = if active { palette.foreground } else { palette.muted };
    let galley = ui.painter().layout_no_wrap(text, egui::FontId::proportional(SIDEBAR_TEXT_SIZE), color);
    let icon = ICON_SIZE - 2.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(13.0 + icon + 6.0 + galley.size().x + 12.0, TAB_HEIGHT), egui::Sense::click());
    let hovered = response.hovered();
    if active || hovered {
        ui.painter().rect_filled(rect, TAB_HEIGHT / 2.0, if active { palette.hover } else { palette.hover.gamma_multiply(0.5) });
    }
    let slot = egui::Rect::from_min_size(egui::pos2(rect.left() + 13.0, rect.center().y - icon / 2.0), egui::Vec2::splat(icon));
    if at_work && !hovered {
        ui.painter().circle_filled(slot.center(), TAB_WORK_DOT / 2.0, theme::AGENT);
    } else {
        crate::icons::paint(ui.painter(), slot, if hovered { "x" } else { "folder" }, color);
    }
    ui.painter().galley(egui::pos2(slot.right() + 6.0, rect.center().y - galley.size().y / 2.0), galley, color);
    let on_cross = response.interact_pointer_pos().is_some_and(|p| slot.expand(5.0).contains(p));
    let closed = response.clicked() && on_cross;
    (response.on_hover_text(if at_work { format!("{title}: the assistant is working") } else { title.to_owned() }), closed)
}

fn starter_card(ui: &mut egui::Ui, title: &str, subtitle: &str, palette: theme::Palette, accent: Option<Color32>) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(220.0, 150.0), egui::Sense::click());
    let painter = ui.painter();
    let fill = if response.hovered() { palette.hover } else { palette.island };
    painter.rect(rect, 12.0, fill, egui::Stroke::new(1.0, palette.border), egui::StrokeKind::Inside);
    let preview = egui::Rect::from_min_size(rect.min + egui::vec2(14.0, 14.0), egui::vec2(rect.width() - 28.0, 76.0));
    painter.rect_filled(preview, 8.0, palette.canvas);
    if let Some(accent) = accent {
        let bar = egui::Rect::from_min_size(preview.min + egui::vec2(14.0, 14.0), egui::vec2(90.0, 10.0));
        painter.rect_filled(bar, 3.0, palette.foreground.gamma_multiply(0.6));
        painter.rect_filled(bar.translate(egui::vec2(0.0, 18.0)), 3.0, palette.muted.gamma_multiply(0.5));
        painter.rect_filled(egui::Rect::from_min_size(bar.min + egui::vec2(0.0, 36.0), egui::vec2(48.0, 14.0)), 4.0, accent);
    }
    painter.text(rect.min + egui::vec2(14.0, 102.0), egui::Align2::LEFT_TOP, title, egui::FontId::proportional(14.0), palette.foreground);
    painter.text(rect.min + egui::vec2(14.0, 122.0), egui::Align2::LEFT_TOP, subtitle, egui::FontId::proportional(12.0), palette.muted);
    response
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
