//! The settings window, after the user's settings design: a list of pages on the left, the chosen page on the
//! right. General holds the interface and canvas settings; Agents lists the assistant providers and lets the
//! user describe a provider of their own.
//!
//! No assistant is connected in this version. A custom provider's API key goes to the operating system's
//! keychain (`secrets`); the settings file only holds the key's name there.

use std::path::PathBuf;
use std::sync::mpsc;

use egui::{Align, Align2, Color32, FontId, Layout, Margin, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};

use crate::icons;
use craft_ai_core::SecretString;

use crate::prefs::{CustomProvider, Prefs, ProviderModel};
use crate::secrets::{self, Keychain, Vault};
use crate::sidebar::icon_button;
use crate::theme::{
    self, CHECKING, CHIP_RADIUS, CONNECTED, FIELD_HEIGHT, ICON_SIZE, PROVIDER_CARD, PROVIDER_RADIUS, PROVIDER_TILE, Palette, SETTINGS_CONTROL,
    SETTINGS_MARGIN, SETTINGS_MAX, SETTINGS_NAV_ROW, SETTINGS_NAV_WIDTH, SETTINGS_RADIUS, SETTINGS_ROW, SETTINGS_SECTION_SIZE, SETTINGS_TALL_ROW,
    SETTINGS_TITLE_SIZE, SIDEBAR_TEXT_SIZE, SWITCH_SIZE,
};

/// Said when a provider is chosen: nothing can be connected in this version.
const NOT_YET: &str = "Providers can't be connected yet.";
/// The kinds of API a custom provider can speak.
const API_TYPES: [&str; 3] = ["OpenAI Completions", "OpenAI Responses", "Anthropic Messages"];
/// Token counts a new model row starts with (the design's example values).
const DEFAULT_TOKENS: &str = "512000";

/// The known providers: name, their models as the design lists them, and whether that list is only a sample
/// of a larger catalogue (shown with a link sign).
const PROVIDERS: [(&str, &str, bool); 16] = [
    ("OpenAI ChatGPT", "GPT 6.1 Sol · GPT 6.1 Terra · GPT 6 Astra · GPT 6 Luna", false),
    ("Anthropic Claude Code", "Fable 5.2 · Opus 5.5 · Sonnet 5.5 · Haiku 5.5", false),
    ("Cursor", "Gemini 3.8 Flash · Grok 4.6 · Composer 2.5 and other models", false),
    ("xAI", "Grok 4.6 · Grok 4.5", false),
    ("Google Gemini", "Gemini 3.7 Flash · Gemini 3.6 Flash · Gemini 3.1 Pro", false),
    ("OpenCode Go", "Multiple models*, e.g. DeepSeek V4 Pro · Kimi K2.7 Code", true),
    ("Open Router", "Multiple models*, e.g. Claude Opus 5 · Claude Haiku 4.5 · GPT-5.5", true),
    ("GitHub Copilot", "Multiple models*, e.g. Claude Opus 5", true),
    ("Meta", "Muse Wonder · Muse Spark 1.3", false),
    ("Moonshot AI", "e.g. Kimi K2.5 · Kimi K2.6 · Kimi K2.7 Code", false),
    ("Kimi For Coding", "e.g. Kimi K3 · Kimi K2.7 Code", false),
    ("Deepseek", "e.g. DeepSeek V4 Flash · DeepSeek V4 Pro", false),
    ("Together AI", "e.g. MiniMax-M3 · Qwen3 235B · Qwen3.5 397B", true),
    ("Fireworks AI", "Multiple models, e.g. Opus 4.8 · GPT-5.5 · DeepSeek V4 Pro · GLM 5.2 · GLM 5.1", true),
    ("Z.AI", "e.g. GLM-5.3 Flash · GLM-5.3 · GLM-5.2", false),
    ("OpenCode Zen", "Multiple models*, e.g. Claude Opus 5.5 · Claude Sonnet 5.5 · Claude Haiku 5.5", true),
];

/// A page of the settings window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Page {
    /// Interface and canvas.
    #[default]
    General,
    /// Assistant providers.
    Agents,
    /// Names of assistants.
    AgentNames,
    /// The chat.
    Chat,
    /// Tool servers.
    Mcp,
}

/// The pages in the order of the list: page, icon, name.
const PAGES: [(Page, &str, &str); 5] = [
    (Page::General, "settings", "General"),
    (Page::Agents, "sparkle", "Agents"),
    (Page::AgentNames, "tag", "Agent names"),
    (Page::Chat, "message-square", "Chat"),
    (Page::Mcp, "terminal", "MCP"),
];

/// One model row of the custom provider form, as typed.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelRow {
    id: String,
    context_window: String,
    max_tokens: String,
}

impl ModelRow {
    fn empty() -> ModelRow {
        ModelRow { id: String::new(), context_window: DEFAULT_TOKENS.to_owned(), max_tokens: DEFAULT_TOKENS.to_owned() }
    }

    /// The model this row describes, if its name and both counts are usable.
    fn model(&self) -> Option<ProviderModel> {
        let count = |text: &str| text.trim().parse::<u32>().ok().filter(|n| *n > 0);
        let id = self.id.trim();
        (!id.is_empty()).then_some(())?;
        Some(ProviderModel { id: id.to_owned(), context_window: count(&self.context_window)?, max_tokens: count(&self.max_tokens)? })
    }
}

/// The custom provider form.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Form {
    /// The saved provider being changed; `None` for a new one.
    index: Option<usize>,
    name: String,
    api: String,
    base_url: String,
    /// A newly typed key; it goes to the keychain when the form is saved, never to the settings.
    key: String,
    /// The name the provider's saved key has in the keychain, if it has one.
    key_ref: Option<String>,
    /// The saved key is to be removed when the form is saved.
    forget_key: bool,
    models: Vec<ModelRow>,
    advanced: bool,
}

impl Form {
    fn new() -> Form {
        Form {
            index: None,
            name: String::new(),
            api: API_TYPES[0].to_owned(),
            base_url: String::new(),
            key: String::new(),
            key_ref: None,
            forget_key: false,
            models: vec![ModelRow::empty()],
            advanced: false,
        }
    }

    fn of(index: usize, provider: &CustomProvider) -> Form {
        let models = provider
            .models
            .iter()
            .map(|m| ModelRow { id: m.id.clone(), context_window: m.context_window.to_string(), max_tokens: m.max_tokens.to_string() })
            .collect();
        Form {
            index: Some(index),
            name: provider.name.clone(),
            api: provider.api.clone(),
            base_url: provider.base_url.clone(),
            key_ref: provider.key_ref.clone(),
            models,
            ..Form::new()
        }
    }

    /// The provider the form describes, if everything needed is filled in: a name, an address, and at least one
    /// model, with every model row usable and no model named twice.
    fn provider(&self) -> Option<CustomProvider> {
        let (name, base_url) = (self.name.trim(), self.base_url.trim());
        let models: Vec<ProviderModel> = self.models.iter().map(ModelRow::model).collect::<Option<_>>()?;
        let twice = models.iter().enumerate().any(|(i, m)| models[..i].iter().any(|other| other.id == m.id));
        (!name.is_empty() && !base_url.is_empty() && !models.is_empty() && !twice).then(|| CustomProvider {
            name: name.to_owned(),
            api: self.api.clone(),
            base_url: base_url.to_owned(),
            models,
            key_ref: self.key_ref.clone(),
        })
    }
}

/// What a provider's card says about its connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Link {
    /// Nothing is set up.
    NotConnected,
    /// Still looking for the provider's own program on this computer.
    Checking,
    /// The provider's own program is installed on this computer.
    Automatic,
}

impl Link {
    fn words(self) -> &'static str {
        match self {
            Link::NotConnected => "Not connected",
            Link::Checking => "Checking status...",
            Link::Automatic => "Connected automatically",
        }
    }
}

/// Which of the assistants' own command-line programs are installed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Installed {
    /// The `claude` program.
    claude: bool,
    /// The `codex` program.
    codex: bool,
}

/// The settings window's state.
pub struct Settings {
    /// Whether the window shows.
    pub open: bool,
    page: Page,
    /// The custom provider form, when it is open on the Agents page.
    form: Option<Form>,
    /// The search for installed assistant programs, while it runs.
    search: Option<mpsc::Receiver<Installed>>,
    /// What the search found; `None` until it has finished.
    installed: Option<Installed>,
    /// Where API keys are kept.
    vault: Box<dyn Vault>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { open: false, page: Page::default(), form: None, search: None, installed: None, vault: Box::new(Keychain) }
    }
}

impl Settings {
    /// Opens the window on the page called `name` on the command line (`--settings`): `general`, `agents`, or
    /// `provider` for the custom provider form.
    pub fn open_named(&mut self, name: &str) {
        self.open = true;
        self.page = if name == "general" { Page::General } else { Page::Agents };
        self.form = (name == "provider").then(Form::new);
        self.installed = None;
    }

    /// Looks for the assistants' own programs on another thread, and takes the answer when it is there.
    // Human: looking through the folders of PATH can take a moment on a slow disk; the cards say "Checking
    // status..." meanwhile.
    // Agent: SPAWNS one short-lived thread per opening of the window; it WAKES the UI when done. If the thread
    // can't start, nothing counts as installed.
    fn search(&mut self, ctx: &egui::Context) {
        if self.installed.is_some() {
            return;
        }
        if let Some(search) = &self.search {
            match search.try_recv() {
                Ok(found) => self.installed = Some(found),
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => self.installed = Some(Installed::default()),
            }
            self.search = None;
            return;
        }
        let (send, receive) = mpsc::channel();
        let wake = ctx.clone();
        let started = std::thread::Builder::new().name("find-assistants".to_owned()).spawn(move || {
            let folders = program_folders();
            // The window may have closed meanwhile; then nobody listens, which is fine.
            let _unheard = send.send(Installed { claude: installed_in("claude", &folders), codex: installed_in("codex", &folders) });
            wake.request_repaint();
        });
        match started {
            Ok(_) => self.search = Some(receive),
            Err(e) => {
                log::warn!("couldn't look for installed assistants: {e}");
                self.installed = Some(Installed::default());
            }
        }
    }

    /// The connection state of the known provider called `name`.
    fn link(&self, name: &str) -> Link {
        let found = |pick: fn(Installed) -> bool| match self.installed {
            None => Link::Checking,
            Some(installed) if pick(installed) => Link::Automatic,
            Some(_) => Link::NotConnected,
        };
        match name {
            "OpenAI ChatGPT" => found(|i| i.codex),
            "Anthropic Claude Code" => found(|i| i.claude),
            _ => Link::NotConnected,
        }
    }

    /// Opens the window on its first page.
    pub fn open_general(&mut self) {
        self.open = true;
        self.page = Page::General;
        self.form = None;
        self.installed = None;
    }

    /// Draws the window when it is open and applies changes to `prefs`. Returns a sentence to tell the user, if
    /// a control asks for one.
    // Agent: MAIN-THREAD; WRITES prefs only; the caller saves them and follows a changed appearance.
    pub fn show(&mut self, ctx: &egui::Context, prefs: &mut Prefs, dark: bool) -> Option<String> {
        if !self.open {
            return None;
        }
        self.search(ctx);
        let palette = theme::palette(dark);
        let mut notice = None;
        let screen = ctx.content_rect();
        let size =
            vec2((screen.width() - 2.0 * SETTINGS_MARGIN).min(SETTINGS_MAX[0]), (screen.height() - 2.0 * SETTINGS_MARGIN).min(SETTINGS_MAX[1]));
        let frame = egui::Frame::new().fill(palette.island).stroke(Stroke::new(1.0, palette.outline)).corner_radius(SETTINGS_RADIUS);
        // Human: the window appears at once; a fade would show the canvas through it for a moment.
        let id = egui::Id::unique("settings");
        let modal = egui::Modal::new(id)
            .area(egui::Modal::default_area(id).fade_in(false))
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(140))
            .show(ctx, |ui| {
                let (window, _) = ui.allocate_exact_size(size, Sense::hover());
                self.pages(ui, window, palette);
                let close = Rect::from_min_size(pos2(window.right() - 40.0, window.top() + 18.0), Vec2::splat(26.0));
                let page = Rect::from_min_max(
                    pos2(window.left() + SETTINGS_NAV_WIDTH + 29.0, window.top() + 1.0),
                    pos2(window.right() - 14.0, window.bottom() - 1.0),
                );
                ui.scope_builder(UiBuilder::new().max_rect(page), |ui| {
                    ui.set_clip_rect(page.intersect(ui.clip_rect()));
                    egui::ScrollArea::vertical().id_salt(("settings-page", self.page, self.form.is_some())).auto_shrink([false, false]).show(
                        ui,
                        |ui| {
                            // Human: room on the right for the close cross and the scroll bar.
                            ui.set_max_width(page.width() - 24.0);
                            match self.page {
                                Page::General => general(ui, prefs, palette),
                                Page::Agents => self.agents(ui, prefs, palette, &mut notice),
                                Page::AgentNames | Page::Chat | Page::Mcp => {
                                    let name = PAGES.iter().find(|(page, ..)| *page == self.page).map_or("", |(.., name)| name);
                                    title(ui, name);
                                    ui.add_space(14.0);
                                    ui.label(RichText::new("Nothing to set here yet.").color(palette.muted));
                                }
                            }
                            ui.add_space(28.0);
                        },
                    );
                });
                ui.scope_builder(UiBuilder::new().max_rect(close), |ui| {
                    if icon_button(ui, "x", close.size(), false, palette).on_hover_text("Close").clicked() {
                        self.open = false;
                    }
                });
            });
        if modal.should_close() {
            self.open = false;
        }
        notice
    }

    /// The list of pages on the left.
    fn pages(&mut self, ui: &mut Ui, window: Rect, palette: Palette) {
        let line = window.left() + SETTINGS_NAV_WIDTH;
        ui.painter().vline(line, window.y_range().shrink(1.0), Stroke::new(1.0, palette.outline.gamma_multiply(0.7)));
        ui.painter().text(window.min + vec2(29.0, 40.0), Align2::LEFT_CENTER, "Settings", FontId::proportional(15.0), palette.foreground);
        let mut top = window.top() + 64.0;
        for (page, icon, name) in PAGES {
            let row = Rect::from_min_size(pos2(window.left() + 18.0, top), vec2(SETTINGS_NAV_WIDTH - 36.0, SETTINGS_NAV_ROW));
            top = row.bottom() + 1.5;
            let response = ui.interact(row, ui.scope_id().with(("settings-page", name)), Sense::click());
            let active = self.page == page;
            if active || response.hovered() {
                ui.painter().rect_filled(
                    row,
                    CHIP_RADIUS,
                    if active { palette.hover.gamma_multiply(0.6) } else { palette.hover.gamma_multiply(0.3) },
                );
            }
            let color = if active || response.hovered() { palette.foreground } else { palette.muted };
            icons::paint(ui.painter(), Rect::from_center_size(pos2(row.left() + 17.0, row.center().y), Vec2::splat(ICON_SIZE - 2.0)), icon, color);
            ui.painter().text(pos2(row.left() + 32.0, row.center().y), Align2::LEFT_CENTER, name, FontId::proportional(SIDEBAR_TEXT_SIZE), color);
            if response.clicked() {
                self.page = page;
                self.form = None;
            }
        }
    }

    /// The Agents page: the providers, or the custom provider form.
    fn agents(&mut self, ui: &mut Ui, prefs: &mut Prefs, palette: Palette, notice: &mut Option<String>) {
        if let Some(form) = &mut self.form {
            if provider_form(ui, form, prefs, self.vault.as_ref(), palette, notice) {
                self.form = None;
            }
            return;
        }
        title(ui, "Connect your AI provider");
        ui.add_space(8.0);
        let about = "Use tokens from your existing subscriptions. Pencraft automatically connects to Claude Code and Codex on your system.";
        ui.scope(|ui| {
            ui.set_max_width(450.0);
            ui.add(egui::Label::new(RichText::new(about).size(SIDEBAR_TEXT_SIZE + 0.5).color(palette.muted)).wrap());
        });
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 11.0;
            ui.spacing_mut().button_padding = vec2(14.0, 9.0);
            let chatgpt = egui::Button::new(RichText::new("Sign in with ChatGPT").size(SIDEBAR_TEXT_SIZE).color(palette.primary_text))
                .fill(palette.primary.gamma_multiply(0.85))
                .corner_radius(CHIP_RADIUS);
            let copilot = egui::Button::new(RichText::new("Sign in with GitHub Copilot").size(SIDEBAR_TEXT_SIZE))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::new(1.0, palette.outline))
                .corner_radius(CHIP_RADIUS);
            if ui.add(chatgpt).clicked() || ui.add(copilot).clicked() {
                *notice = Some("Signing in isn't available yet.".to_owned());
            }
        });
        ui.add_space(26.0);
        ui.label(RichText::new("Setup providers").size(SETTINGS_SECTION_SIZE));
        ui.add_space(10.0);
        ui.spacing_mut().item_spacing.y = 7.5;
        for (name, models, sample) in PROVIDERS {
            if provider_card(ui, name, models, sample, Some(self.link(name)), palette).clicked() {
                *notice = Some(NOT_YET.to_owned());
            }
        }
        let saved: Vec<(String, String)> =
            prefs.agents.providers.iter().map(|p| (p.name.clone(), p.models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>().join(" · "))).collect();
        for (index, (name, models)) in saved.iter().enumerate() {
            if provider_card(ui, name, models, false, Some(Link::NotConnected), palette).clicked()
                && let Some(provider) = prefs.agents.providers.get(index)
            {
                self.form = Some(Form::of(index, provider));
            }
        }
        if provider_card(ui, "Add custom provider", "", false, None, palette).clicked() {
            self.form = Some(Form::new());
        }
        ui.add_space(6.0);
        let footnote =
            "*Not all models from each provider are supported by Pencraft, we are working on validating and adding more models as they fit.";
        ui.add(egui::Label::new(RichText::new(footnote).small().color(palette.muted.gamma_multiply(0.8))).wrap());
    }
}

/// The General page.
fn general(ui: &mut Ui, prefs: &mut Prefs, palette: Palette) {
    title(ui, "General");
    section(ui, "Interface");
    row(ui, "Appearance", None, palette, |ui| appearance_switch(ui, &mut prefs.appearance.dark, palette));
    divider(ui, palette);
    row(ui, "Language", None, palette, |ui| choice(ui, "language", &mut prefs.appearance.language, &["System"], palette));
    divider(ui, palette);
    row(ui, "Animations & effects", Some("Turn off to save GPU/CPU."), palette, |ui| switch(ui, &mut prefs.appearance.animations, palette));

    section(ui, "Updates");
    let updates = format!("When Pencraft starts and once a day, ask GitHub whether there is a newer version than {}.", crate::update::VERSION);
    row(ui, "Look for new versions", Some(&updates), palette, |ui| switch(ui, &mut prefs.updates.check, palette));

    section(ui, "Canvas");
    let canvas = &mut prefs.canvas;
    row(ui, "Show pixel grid", None, palette, |ui| switch(ui, &mut canvas.show_pixel_grid, palette));
    divider(ui, palette);
    row(ui, "Snap to pixel grid", None, palette, |ui| switch(ui, &mut canvas.snap_to_pixels, palette));
    divider(ui, palette);
    row(ui, "Snap to objects", None, palette, |ui| switch(ui, &mut canvas.snap_to_objects, palette));
    divider(ui, palette);
    row(ui, "Use scroll wheel to zoom", None, palette, |ui| switch(ui, &mut canvas.wheel_zooms, palette));
    divider(ui, palette);
    row(ui, "Invert zoom direction", None, palette, |ui| switch(ui, &mut canvas.invert_zoom, palette));

    section(ui, "Custom nudge amount");
    ui.label(RichText::new("How far selected objects move with the arrow keys").color(palette.muted));
    ui.add_space(4.0);
    row(ui, "Small nudge", None, palette, |ui| pixels(ui, &mut canvas.small_nudge));
    divider(ui, palette);
    row(ui, "Big nudge", None, palette, |ui| pixels(ui, &mut canvas.big_nudge));

    section(ui, "Import & generation");
    let generation = &mut prefs.generation;
    row(ui, "Include HTML metadata when importing pages", None, palette, |ui| switch(ui, &mut generation.html_metadata, palette));
    divider(ui, palette);
    row(ui, "Default image generation provider", None, palette, |ui| {
        choice(ui, "image-provider", &mut generation.image_provider, &["Nano Banana Lite"], palette);
    });
    divider(ui, palette);
    row(ui, "Default SVG generation provider", None, palette, |ui| choice(ui, "svg-provider", &mut generation.svg_provider, &["Pencraft"], palette));

    section(ui, "Agents");
    let agents = &mut prefs.agents;
    let tools = "AI models on this computer (Claude Code, Cursor, Codex and others) can read and change open documents through `pencraft mcp`, also while Pencraft is in the background.";
    row(ui, "Let AI models work on documents", Some(tools), palette, |ui| switch(ui, &mut agents.tools, palette));
    divider(ui, palette);
    let review = "When the assistant has finished a design, a second model that didn't make it judges a picture of it like a strict art director, and the assistant works in what it wants changed. Up to two rounds; a design takes longer and costs more.";
    row(ui, "Have designs reviewed", Some(review), palette, |ui| switch(ui, &mut agents.review, palette));
    divider(ui, palette);
    let steer = "Send messages immediately to running agents that support steering instead of queueing them.";
    row(ui, "Always steer messages", Some(steer), palette, |ui| switch(ui, &mut agents.always_steer, palette));
    divider(ui, palette);
    row(ui, "Chat text size", None, palette, |ui| choice(ui, "chat-text", &mut agents.chat_text_size, &["Small", "Medium", "Large"], palette));
    divider(ui, palette);
    row(ui, "Agent cursor size", None, palette, |ui| {
        choice(ui, "cursor-size", &mut agents.cursor_size, &["Small (Default)", "Medium", "Large"], palette);
    });
    divider(ui, palette);
    row(ui, "Agent cursor label", None, palette, |ui| {
        choice(ui, "cursor-label", &mut agents.cursor_label, &["Agent (Default)", "Model name", "None"], palette);
    });
    divider(ui, palette);
    row(ui, "Show agent input on dashboard", None, palette, |ui| switch(ui, &mut agents.input_on_dashboard, palette));
    divider(ui, palette);
    let ideas = "A row of ideas under the message bar of an empty document while the sidebar is folded and nothing is typed.";
    row(ui, "Show ideas under the message bar", Some(ideas), palette, |ui| switch(ui, &mut agents.ideas, palette));
}

/// The custom provider form. Returns `true` when it is finished: the provider was saved or removed.
fn provider_form(ui: &mut Ui, form: &mut Form, prefs: &mut Prefs, vault: &dyn Vault, palette: Palette, notice: &mut Option<String>) -> bool {
    let mut finished = false;
    ui.add_space(24.0);
    // The header: a picture, the name, what the form is for, the state.
    let (header, _) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::hover());
    let tile = Rect::from_min_size(header.min, Vec2::splat(50.0));
    ui.painter().rect_filled(tile, PROVIDER_RADIUS, palette.hover.gamma_multiply(0.6));
    icons::paint(ui.painter(), Rect::from_center_size(tile.center(), Vec2::splat(26.0)), "server", palette.foreground);
    let name = if form.index.is_some() && !form.name.trim().is_empty() { form.name.trim() } else { "Custom provider" };
    ui.painter().text(
        pos2(tile.right() + 13.0, header.top() + 14.0),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(SETTINGS_TITLE_SIZE),
        palette.foreground,
    );
    let purpose = "Connect any OpenAI or Anthropic compatible API";
    ui.painter().text(
        pos2(tile.right() + 13.0, header.top() + 40.0),
        Align2::LEFT_CENTER,
        purpose,
        FontId::proportional(SIDEBAR_TEXT_SIZE),
        palette.muted,
    );
    status(ui, pos2(header.right(), header.center().y), Link::NotConnected, palette);

    section(ui, "Provider");
    ui.columns(2, |columns| {
        caption(&mut columns[0], "Name");
        field(&mut columns[0], &mut form.name, "My provider", false, false, palette);
        caption(&mut columns[1], "API type");
        wide_choice(&mut columns[1], &mut form.api, &API_TYPES, palette);
    });
    ui.add_space(8.0);
    caption(ui, "Base URL");
    field(ui, &mut form.base_url, "https://api.example.com/v1", true, false, palette);
    if unprotected(&form.base_url) {
        ui.add_space(2.0);
        ui.label(RichText::new("This address isn't encrypted: the key and your messages could be read on the way.").color(CHECKING));
    }

    section(ui, "Authentication & models");
    key_field(ui, form, palette);
    ui.add_space(8.0);
    model_rows(ui, form, palette);
    ui.add_space(12.0);
    // Advanced: nothing to set yet, but the row opens as in the design.
    let (advanced, response) = ui.allocate_exact_size(vec2(ui.available_width(), 37.0), Sense::click());
    ui.painter().rect_stroke(advanced, CHIP_RADIUS, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    ui.painter().text(
        pos2(advanced.left() + 12.0, advanced.center().y),
        Align2::LEFT_CENTER,
        "Advanced",
        FontId::proportional(SIDEBAR_TEXT_SIZE),
        palette.foreground,
    );
    let arrow = if form.advanced { "chevron-down" } else { "chevron-right" };
    icons::paint(ui.painter(), Rect::from_center_size(pos2(advanced.right() - 18.0, advanced.center().y), Vec2::splat(12.0)), arrow, palette.muted);
    if response.clicked() {
        form.advanced = !form.advanced;
    }
    if form.advanced {
        ui.add_space(6.0);
        ui.label(RichText::new("No advanced options yet.").color(palette.muted));
    }
    ui.add_space(16.0);
    let provider = form.provider();
    ui.horizontal(|ui| {
        let label = if form.index.is_some() { "Save provider" } else { "Add provider" };
        let save = egui::Button::new(RichText::new(label).color(palette.primary_text))
            .fill(palette.primary)
            .corner_radius(CHIP_RADIUS)
            .min_size(vec2(92.0, FIELD_HEIGHT));
        if ui.add_enabled(provider.is_some(), save).clicked()
            && let Some(provider) = provider
        {
            match save_provider(form, provider, prefs, vault) {
                Ok(()) => finished = true,
                Err(why) => *notice = Some(why),
            }
        }
        if let Some(index) = form.index
            && ui.add(egui::Button::new("Remove").corner_radius(CHIP_RADIUS).min_size(vec2(72.0, FIELD_HEIGHT))).clicked()
        {
            match remove_provider(index, prefs, vault) {
                Ok(()) => finished = true,
                Err(why) => *notice = Some(why),
            }
        }
    });
    finished
}

/// The API key field with what it says about a key that is already saved.
fn key_field(ui: &mut Ui, form: &mut Form, palette: Palette) {
    caption(ui, "API key");
    let saved = form.key_ref.is_some() && !form.forget_key;
    field(ui, &mut form.key, if saved { "Saved - type a new key to replace it" } else { "Optional" }, true, true, palette);
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        let help = "Leave blank if your endpoint doesn't require a key. The key is kept in your system's keychain.";
        ui.label(RichText::new(help).color(palette.muted));
        if saved && form.key.is_empty() && ui.small_button("Forget saved key").clicked() {
            form.forget_key = true;
        }
    });
}

/// Saves the form's `provider` in the settings and its key in the keychain.
///
/// A newly typed key replaces the saved one; "Forget saved key" removes it; otherwise the saved key stays.
///
/// # Errors
/// A sentence for the user when the keychain refused; the settings are then left as they were.
// Agent: WRITES the keychain first, the settings only after that worked, so the settings never name a key
// that isn't there.
fn save_provider(form: &mut Form, mut provider: CustomProvider, prefs: &mut Prefs, vault: &dyn Vault) -> Result<(), String> {
    let typed = form.key.trim();
    if !typed.is_empty() {
        let name = provider.key_ref.clone().unwrap_or_else(secrets::new_name);
        vault.put(&name, &SecretString::new(typed)).map_err(|e| format!("The key couldn't be saved in the keychain: {e}"))?;
        provider.key_ref = Some(name);
    } else if form.forget_key
        && let Some(name) = provider.key_ref.take()
    {
        vault.forget(&name).map_err(|e| format!("The key couldn't be removed from the keychain: {e}"))?;
    }
    form.key.clear();
    match form.index.and_then(|index| prefs.agents.providers.get_mut(index)) {
        Some(saved) => *saved = provider,
        None => prefs.agents.providers.push(provider),
    }
    Ok(())
}

/// Removes the saved provider at `index` and its key.
///
/// # Errors
/// A sentence for the user when the keychain refused; the provider then stays.
fn remove_provider(index: usize, prefs: &mut Prefs, vault: &dyn Vault) -> Result<(), String> {
    let Some(provider) = prefs.agents.providers.get(index) else { return Ok(()) };
    if let Some(name) = &provider.key_ref {
        vault.forget(name).map_err(|e| format!("The key couldn't be removed from the keychain: {e}"))?;
    }
    prefs.agents.providers.remove(index);
    Ok(())
}

/// Whether `address` would send the key without encryption: plain `http` to anything but this computer.
fn unprotected(address: &str) -> bool {
    let address = address.trim().to_ascii_lowercase();
    let Some(rest) = address.strip_prefix("http://") else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host =
        if host.starts_with('[') { host.split(']').next().unwrap_or("").trim_start_matches('[') } else { host.split(':').next().unwrap_or("") };
    !matches!(host, "localhost" | "127.0.0.1" | "::1")
}

/// The model rows of the custom provider form, and the button that adds one.
fn model_rows(ui: &mut Ui, form: &mut Form, palette: Palette) {
    // Human: three fields side by side and a bin; only the first row carries the fields' names.
    let gap = 13.0;
    let bin = 26.0;
    let fields = ui.available_width() - bin - 3.0 * gap;
    let widths = [fields * 0.5, fields * 0.25, fields * 0.25];
    let mut remove = None;
    let rows = form.models.len();
    for (index, model) in form.models.iter_mut().enumerate() {
        let names = if index == 0 { 27.0 } else { 0.0 };
        let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), names + FIELD_HEIGHT + 4.0), Sense::hover());
        let mut left = line.left();
        for (text, name, hint, width) in [
            (&mut model.id, "Model ID", "my-model-id", widths[0]),
            (&mut model.context_window, "Context window", DEFAULT_TOKENS, widths[1]),
            (&mut model.max_tokens, "Max tokens", DEFAULT_TOKENS, widths[2]),
        ] {
            let cell = Rect::from_min_size(pos2(left, line.top()), vec2(width, line.height()));
            left = cell.right() + gap;
            // Human: a child placed by hand, so the page's own flow continues under the whole row.
            let mut cell = ui.new_child(UiBuilder::new().max_rect(cell).id_salt((index, name)));
            if index == 0 {
                caption(&mut cell, name);
            }
            field(&mut cell, text, hint, true, false, palette);
        }
        let cell = Rect::from_min_size(pos2(left, line.bottom() - FIELD_HEIGHT), Vec2::splat(bin));
        let mut cell = ui.new_child(UiBuilder::new().max_rect(cell).id_salt((index, "bin")));
        if rows > 1 && icon_button(&mut cell, "trash-2", Vec2::splat(bin), false, palette).on_hover_text("Remove model").clicked() {
            remove = Some(index);
        }
        ui.add_space(6.0);
    }
    if let Some(index) = remove {
        form.models.remove(index);
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new("Each model appears under this provider in the model picker.").color(palette.muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let add = egui::Button::new("+  Add model")
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::new(1.0, palette.outline))
                .corner_radius(CHIP_RADIUS)
                .min_size(vec2(96.0, FIELD_HEIGHT - 2.0));
            if ui.add(add).clicked() {
                form.models.push(ModelRow::empty());
            }
        });
    });
}

/// A page's title.
fn title(ui: &mut Ui, text: &str) {
    ui.add_space(34.0);
    ui.add(egui::Label::new(RichText::new(text).size(SETTINGS_TITLE_SIZE)).selectable(false));
}

/// A section's title.
fn section(ui: &mut Ui, text: &str) {
    ui.add_space(24.0);
    ui.add(egui::Label::new(RichText::new(text).size(SETTINGS_SECTION_SIZE)).selectable(false));
    ui.add_space(6.0);
}

/// The small name over a form field.
fn caption(ui: &mut Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).size(SIDEBAR_TEXT_SIZE)).selectable(false));
    ui.add_space(2.0);
}

/// The line between two rows of a section.
fn divider(ui: &mut Ui, palette: Palette) {
    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(line.x_range(), line.center().y, Stroke::new(1.0, palette.outline.gamma_multiply(0.7)));
}

/// A settings row: its name (and a second, quieter line) on the left, its control on the right.
fn row(ui: &mut Ui, name: &str, more: Option<&str>, palette: Palette, control: impl FnOnce(&mut Ui)) {
    ui.spacing_mut().item_spacing.y = 0.0;
    let height = if more.is_some() { SETTINGS_TALL_ROW } else { SETTINGS_ROW };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE);
    if let Some(more) = more {
        ui.painter().text(pos2(rect.left(), rect.center().y - 9.0), Align2::LEFT_CENTER, name, font, palette.foreground);
        ui.painter().text(pos2(rect.left(), rect.center().y + 10.0), Align2::LEFT_CENTER, more, FontId::proportional(12.0), palette.muted);
    } else {
        ui.painter().text(rect.left_center(), Align2::LEFT_CENTER, name, font, palette.foreground);
    }
    // Human: a child placed by hand, so every row is exactly as tall as the design says.
    control(&mut ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::right_to_left(Align::Center)).id_salt(name)));
}

/// An on/off switch.
fn switch(ui: &mut Ui, on: &mut bool, palette: Palette) {
    let (rect, response) = ui.allocate_exact_size(Vec2::from(SWITCH_SIZE), Sense::click());
    if response.clicked() {
        *on = !*on;
    }
    let radius = rect.height() / 2.0;
    let knob = radius - 2.0;
    if *on {
        ui.painter().rect_filled(rect, radius, palette.foreground);
        ui.painter().circle_filled(pos2(rect.right() - radius, rect.center().y), knob, palette.island);
    } else {
        ui.painter().rect(rect, radius, palette.input.gamma_multiply(0.5), Stroke::new(1.0, palette.outline), StrokeKind::Inside);
        ui.painter().circle_filled(pos2(rect.left() + radius, rect.center().y), knob, palette.muted.gamma_multiply(0.6));
    }
}

/// The Light / Dark switch.
fn appearance_switch(ui: &mut Ui, dark: &mut bool, palette: Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(122.0, SETTINGS_CONTROL), Sense::hover());
    ui.painter().rect_filled(rect, CHIP_RADIUS, palette.hover.gamma_multiply(0.5));
    let halves = [(false, "sun", "Light", rect.left()), (true, "moon", "Dark", rect.center().x)];
    for (value, icon, name, left) in halves {
        let half = Rect::from_min_size(pos2(left, rect.top()), vec2(rect.width() / 2.0, rect.height())).shrink(1.5);
        let response = ui.interact(half, ui.scope_id().with(("appearance", name)), Sense::click());
        let active = *dark == value;
        if active {
            ui.painter().rect_filled(half, CHIP_RADIUS - 1.5, palette.hover);
        }
        let color = if active || response.hovered() { palette.foreground } else { palette.muted };
        icons::paint(ui.painter(), Rect::from_center_size(pos2(half.left() + 15.0, half.center().y), Vec2::splat(11.0)), icon, color);
        ui.painter().text(pos2(half.left() + 25.0, half.center().y), Align2::LEFT_CENTER, name, FontId::proportional(12.0), color);
        if response.clicked() {
            *dark = value;
        }
    }
}

/// A small outlined button showing `value`; a click offers `options`.
fn choice(ui: &mut Ui, salt: &str, value: &mut String, options: &[&str], palette: Palette) {
    let font = FontId::proportional(12.0);
    let width = ui.painter().layout_no_wrap(value.clone(), font.clone(), palette.foreground).size().x;
    let (rect, _) = ui.allocate_exact_size(vec2(width + 36.0, SETTINGS_CONTROL), Sense::hover());
    choice_at(ui, rect, salt, value, options, font, palette);
}

/// A choice as wide and tall as a text field.
fn wide_choice(ui: &mut Ui, value: &mut String, options: &[&str], palette: Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), FIELD_HEIGHT), Sense::hover());
    choice_at(ui, rect, "wide", value, options, FontId::proportional(SIDEBAR_TEXT_SIZE), palette);
}

fn choice_at(ui: &mut Ui, rect: Rect, salt: &str, value: &mut String, options: &[&str], font: FontId, palette: Palette) {
    let response = ui.interact(rect, ui.scope_id().with(("choice", salt)), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, CHIP_RADIUS, palette.hover.gamma_multiply(0.4));
    }
    ui.painter().rect_stroke(rect, CHIP_RADIUS, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    ui.painter().text(pos2(rect.left() + 11.0, rect.center().y), Align2::LEFT_CENTER, value.as_str(), font, palette.foreground);
    icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(rect.right() - 13.0, rect.center().y), Vec2::splat(11.0)),
        "chevrons-up-down",
        palette.muted,
    );
    egui::Popup::menu(&response).show(|ui| {
        for option in options {
            if ui.selectable_label(value.as_str() == *option, *option).clicked() {
                (*option).clone_into(value);
            }
        }
    });
}

/// A number of pixels.
fn pixels(ui: &mut Ui, value: &mut f32) {
    ui.add(egui::DragValue::new(value).range(1.0..=1000.0).speed(0.2).max_decimals(0).suffix(" px"));
}

/// An outlined text field as wide as the space it is in.
fn field(ui: &mut Ui, text: &mut String, hint: &str, mono: bool, secret: bool, palette: Palette) -> Response {
    let font = if mono { egui::TextStyle::Monospace } else { egui::TextStyle::Body };
    let frame = egui::Frame::new().stroke(Stroke::new(1.0, palette.outline)).corner_radius(CHIP_RADIUS).inner_margin(Margin::symmetric(11, 7));
    let hint = RichText::new(hint).text_style(font.clone()).color(palette.muted.gamma_multiply(0.75));
    ui.add(egui::TextEdit::singleline(text).frame(frame).hint_text(hint).font(font).password(secret).desired_width(ui.available_width()))
}

/// A provider's card: a square with its initial, its name, its models, its state and an arrow.
fn provider_card(ui: &mut Ui, name: &str, models: &str, sample: bool, state: Option<Link>, palette: Palette) -> Response {
    let (card, response) = ui.allocate_exact_size(vec2(ui.available_width(), PROVIDER_CARD), Sense::click());
    let fill = if response.hovered() { palette.hover.gamma_multiply(0.25) } else { Color32::TRANSPARENT };
    ui.painter().rect(card, PROVIDER_RADIUS, fill, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    let tile = Rect::from_center_size(pos2(card.left() + 15.0 + PROVIDER_TILE / 2.0, card.center().y), Vec2::splat(PROVIDER_TILE));
    ui.painter().rect_filled(tile, CHIP_RADIUS, palette.hover.gamma_multiply(0.6));
    let text_left = tile.right() + 15.0;
    if state.is_some() {
        // Human: brand marks aren't bundled, so the square shows the provider's first letter.
        let initial: String = name.chars().take(1).collect();
        ui.painter().text(tile.center(), Align2::CENTER_CENTER, initial, FontId::proportional(15.0), palette.foreground);
    } else {
        icons::paint(ui.painter(), Rect::from_center_size(tile.center(), Vec2::splat(ICON_SIZE)), "plus", palette.foreground);
    }
    if models.is_empty() {
        ui.painter().text(pos2(text_left, card.center().y), Align2::LEFT_CENTER, name, FontId::proportional(13.5), palette.foreground);
    } else {
        ui.painter().text(pos2(text_left, card.top() + 23.0), Align2::LEFT_CENTER, name, FontId::proportional(13.5), palette.foreground);
        let mut left = text_left;
        if sample {
            icons::paint(ui.painter(), Rect::from_center_size(pos2(left + 6.0, card.top() + 44.0), Vec2::splat(11.0)), "link", palette.muted);
            left += 19.0;
        }
        ui.painter().text(pos2(left, card.top() + 43.0), Align2::LEFT_CENTER, models, FontId::proportional(12.0), palette.muted);
    }
    icons::paint(ui.painter(), Rect::from_center_size(pos2(card.right() - 21.0, card.center().y), Vec2::splat(12.0)), "chevron-right", palette.muted);
    if let Some(state) = state {
        status(ui, pos2(card.right() - 42.0, card.center().y), state, palette);
    }
    response
}

/// A state pill ("Not connected") with its right edge at `right`.
fn status(ui: &Ui, right: egui::Pos2, link: Link, palette: Palette) {
    let font = FontId::proportional(12.0);
    let (text, dot, edge, fill) = match link {
        Link::NotConnected => (palette.muted, palette.muted.gamma_multiply(0.5), palette.outline, Color32::TRANSPARENT),
        Link::Checking => (CHECKING, CHECKING, CHECKING.gamma_multiply(0.5), CHECKING.gamma_multiply(0.14)),
        Link::Automatic => (CONNECTED, CONNECTED, CONNECTED.gamma_multiply(0.5), CONNECTED.gamma_multiply(0.14)),
    };
    let width = ui.painter().layout_no_wrap(link.words().to_owned(), font.clone(), text).size().x;
    let pill = Rect::from_min_size(pos2(right.x - width - 32.0, right.y - 11.5), vec2(width + 32.0, 23.0));
    ui.painter().rect(pill, 11.5, fill, Stroke::new(1.0, edge), StrokeKind::Inside);
    ui.painter().circle_filled(pos2(pill.left() + 12.0, pill.center().y), 2.5, dot);
    ui.painter().text(pos2(pill.left() + 21.0, pill.center().y), Align2::LEFT_CENTER, link.words(), font, text);
}

/// The folders a command-line program may be installed in: those of `PATH`, and the usual ones a program
/// started from the Finder or the Start menu doesn't have in its `PATH`.
fn program_folders() -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = std::env::var_os("PATH").map(|path| std::env::split_paths(&path).collect()).unwrap_or_default();
    if !cfg!(windows) {
        folders.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(PathBuf::from));
    }
    // Human: the user's own folder is `HOME` on a Mac and on Linux, `USERPROFILE` on Windows.
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from) {
        folders.extend([".local/bin", ".claude/local", ".npm-global/bin", ".bun/bin"].map(|folder| home.join(folder)));
    }
    // Human: where npm puts the programs it installs on Windows.
    if let Some(roaming) = std::env::var_os("APPDATA").filter(|_| cfg!(windows)).map(PathBuf::from) {
        folders.push(roaming.join("npm"));
    }
    folders
}

/// The file names the program called `name` can have, the one to prefer first.
// Human: on Windows a program's file has an ending: `.exe` for a real program, `.cmd` for the starter npm
// writes. A file there with no ending at all is a script for another shell, which Windows can't start.
fn file_names(name: &str) -> Vec<String> {
    if cfg!(windows) { vec![format!("{name}.exe"), format!("{name}.cmd")] } else { vec![name.to_owned()] }
}

/// Where the program called `name` is in one of `folders`: the first folder that has it wins.
fn find_in(name: &str, folders: &[PathBuf]) -> Option<PathBuf> {
    let names = file_names(name);
    folders.iter().flat_map(|folder| names.iter().map(move |file| folder.join(file))).find(|path| path.is_file())
}

/// Where the command-line program called `name` is installed, if it is.
#[must_use]
pub fn program(name: &str) -> Option<PathBuf> {
    find_in(name, &program_folders())
}

/// Whether the program called `program` is in one of `folders`.
fn installed_in(program: &str, folders: &[PathBuf]) -> bool {
    find_in(program, folders).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled() -> Form {
        let mut form = Form::new();
        form.name = " Mine ".to_owned();
        form.base_url = "https://api.example.com/v1".to_owned();
        form.key = "secret".to_owned();
        form.models[0].id = "model-a".to_owned();
        form
    }

    /// A vault in memory, which can be told to refuse.
    #[derive(Default)]
    struct Memory {
        kept: std::cell::RefCell<std::collections::HashMap<String, String>>,
        refuse: bool,
    }

    impl Vault for Memory {
        fn put(&self, name: &str, key: &SecretString) -> Result<(), String> {
            if self.refuse {
                return Err("locked".to_owned());
            }
            self.kept.borrow_mut().insert(name.to_owned(), key.expose().to_owned());
            Ok(())
        }

        fn get(&self, name: &str) -> Result<SecretString, String> {
            self.kept.borrow().get(name).map(|key| SecretString::new(key.as_str())).ok_or_else(|| "nothing kept".to_owned())
        }

        fn forget(&self, name: &str) -> Result<(), String> {
            if self.refuse {
                return Err("locked".to_owned());
            }
            self.kept.borrow_mut().remove(name);
            Ok(())
        }
    }

    /// Saves `form` as the user's click on the save button would.
    fn save(form: &mut Form, prefs: &mut Prefs, vault: &Memory) -> Result<(), String> {
        let provider = form.provider().unwrap();
        save_provider(form, provider, prefs, vault)
    }

    #[test]
    fn saving_puts_the_key_in_the_vault_and_only_its_name_in_the_settings() {
        let (mut prefs, vault, mut form) = (Prefs::default(), Memory::default(), filled());
        save(&mut form, &mut prefs, &vault).unwrap();
        let saved = prefs.agents.providers[0].clone();
        let name = saved.key_ref.clone().unwrap();
        assert_eq!(vault.kept.borrow().get(&name).map(String::as_str), Some("secret"));
        assert!(!serde_json::to_string(&prefs).unwrap().contains("secret"));
        assert_eq!(form.key, "");

        // Saving again without typing keeps the key; typing replaces it under the same name.
        save(&mut Form::of(0, &saved), &mut prefs, &vault).unwrap();
        assert_eq!(prefs.agents.providers[0].key_ref.as_deref(), Some(name.as_str()));
        let mut form = Form::of(0, &saved);
        form.key = "newer".to_owned();
        save(&mut form, &mut prefs, &vault).unwrap();
        assert_eq!(vault.kept.borrow().get(&name).map(String::as_str), Some("newer"));
        assert_eq!(prefs.agents.providers.len(), 1);

        // Forgetting removes the key and its name; removing the provider removes its key too.
        let mut form = Form::of(0, &saved);
        form.forget_key = true;
        save(&mut form, &mut prefs, &vault).unwrap();
        assert!(vault.kept.borrow().is_empty() && prefs.agents.providers[0].key_ref.is_none());
        let mut form = Form::of(0, &prefs.agents.providers[0]);
        form.key = "again".to_owned();
        save(&mut form, &mut prefs, &vault).unwrap();
        assert_eq!(vault.kept.borrow().len(), 1);
        remove_provider(0, &mut prefs, &vault).unwrap();
        assert!(vault.kept.borrow().is_empty() && prefs.agents.providers.is_empty());
    }

    #[test]
    fn a_refusing_vault_leaves_the_settings_as_they_were() {
        let (mut prefs, vault, mut form) = (Prefs::default(), Memory { refuse: true, ..Memory::default() }, filled());
        let why = save(&mut form, &mut prefs, &vault).unwrap_err();
        assert!(why.contains("keychain") && !why.contains("secret"));
        assert_eq!(prefs.agents.providers.len(), 0);
        assert_eq!(form.key, "secret");
    }

    #[test]
    fn only_plain_http_to_another_computer_is_unprotected() {
        assert!(unprotected("http://api.example.com/v1"));
        assert!(unprotected(" HTTP://192.168.1.5:8080"));
        assert!(!unprotected("https://api.example.com/v1"));
        assert!(!unprotected("http://localhost:11434/v1"));
        assert!(!unprotected("http://127.0.0.1/v1"));
        assert!(!unprotected("http://[::1]:8080/v1"));
        assert!(!unprotected(""));
    }

    #[test]
    fn a_filled_form_gives_a_provider_without_its_key() {
        let provider = filled().provider().unwrap();
        assert_eq!(provider.name, "Mine");
        assert_eq!(provider.models, vec![ProviderModel { id: "model-a".to_owned(), context_window: 512_000, max_tokens: 512_000 }]);
        assert!(!serde_json::to_string(&provider).unwrap().contains("secret"));
        assert_eq!(Form::of(0, &provider).provider(), Some(provider));
    }

    #[test]
    fn missing_or_wrong_entries_give_no_provider() {
        let change = |f: fn(&mut Form)| {
            let mut form = filled();
            f(&mut form);
            form.provider()
        };
        assert!(change(|f| f.name.clear()).is_none());
        assert!(change(|f| f.base_url = "  ".to_owned()).is_none());
        assert!(change(|f| f.models[0].id.clear()).is_none());
        assert!(change(|f| f.models[0].max_tokens = "0".to_owned()).is_none());
        assert!(change(|f| f.models[0].context_window = "many".to_owned()).is_none());
        assert!(change(|f| f.models.push(f.models[0].clone())).is_none());
        assert!(change(|f| f.models.clear()).is_none());
    }

    #[test]
    fn a_program_counts_as_installed_only_where_its_file_is() {
        let folder = std::env::temp_dir().join(format!("pencraft-installed-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(&file_names("claude")[0]), "").unwrap();
        let folders = [PathBuf::from("/nowhere/at/all"), folder.clone()];
        assert!(installed_in("claude", &folders));
        assert!(!installed_in("codex", &folders));
        assert!(!installed_in("claude", &folders[..1]));
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn only_the_two_local_assistants_follow_what_is_installed() {
        let mut settings = Settings::default();
        assert_eq!(settings.link("Anthropic Claude Code"), Link::Checking);
        assert_eq!(settings.link("Cursor"), Link::NotConnected);
        settings.installed = Some(Installed { claude: true, codex: false });
        assert_eq!(settings.link("Anthropic Claude Code"), Link::Automatic);
        assert_eq!(settings.link("OpenAI ChatGPT"), Link::NotConnected);
    }

    #[test]
    fn every_settings_icon_is_bundled() {
        for (_, icon, _) in PAGES {
            assert!(icons::outline("lucide", icon).is_some(), "{icon}");
        }
        for icon in ["server", "trash-2", "link", "chevrons-up-down", "chevron-right", "sun", "moon", "x"] {
            assert!(icons::outline("lucide", icon).is_some(), "{icon}");
        }
    }
}
