//! The settings window: a list of pages on the left, in three groups, and the chosen page on the right. Every
//! setting is on one page only, in a card with the settings it belongs to. General and Canvas are about the
//! app; Assistants, Chat and Images about the AI models; Permissions and MCP about what they may do and how
//! other apps reach them. A field over the list of pages finds a setting by its name.
//!
//! The chat talks to Claude Code when it is installed and to custom providers that speak the Messages API;
//! the other known providers can't be connected yet. A custom provider's API key goes to the operating system's
//! keychain (`secrets`); the settings file only holds the key's name there.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use egui::{Align, Align2, FontId, Layout, Rect, RichText, Sense, Stroke, Ui, UiBuilder, Vec2, pos2, vec2};

use crate::chat::catalog;
use crate::icons;
use craft_ai_core::SecretString;

use crate::chat::Choice;
use crate::chat::{KIND_CLAUDE, KIND_GROK, KIND_PLAIN, KINDS};
use crate::prefs::{AgentName, Agents, Appearance, CustomProvider, ImageService, Images, LocalAgent, Prefs, ProviderModel};
use crate::secrets::{self, Keychain, Vault};
use crate::sidebar::icon_button;
use crate::theme::{
    self, CHIP_RADIUS, DIALOG_BACKDROP, FIELD_HEIGHT, ICON_SIZE, Palette, SETTINGS_FOOTER, SETTINGS_MARGIN, SETTINGS_MAX, SETTINGS_NAME_FIELD,
    SETTINGS_NAV_ROW, SETTINGS_NAV_WIDTH, SETTINGS_PAGE_INSET, SETTINGS_PAGE_WIDTH, SETTINGS_PROVIDERS_SHOWN, SETTINGS_RADIUS, SETTINGS_TOOLS_SHOWN,
    SIDEBAR_TEXT_SIZE,
};

mod models;
mod parts;

use parts::{
    Kind, Lead, Segment, Tone, about, button, card, choice, field, heading, heading_with, hint, link_state, note, pill, pill_at, search_field,
    segmented, sized_field, strong, tile_button, title, warning, wide_choice,
};
pub(crate) use parts::{caption, choice_at, row, switch};

/// Said when a provider is chosen that has no API another app can use with a key.
const NO_KEY_API: &str = " has no API that other apps can use with a key, so it can't be connected.";
/// The kinds of API a custom provider can speak.
const API_TYPES: [&str; 4] = ["OpenAI Completions", "OpenAI Responses", "Anthropic Messages", "Ollama"];
/// What the Images page's default service choice shows when no service is named: the first that works.
const ANY_SERVICE: &str = "First that works";
/// Token counts a new model row starts with (the design's example values).
const DEFAULT_TOKENS: &str = "512000";

/// The known providers: name, their models as the design lists them, and whether that list is only a sample
/// of a larger catalogue (shown with a link sign).
const PROVIDERS: [(&str, &str, bool); 18] = [
    ("OpenAI ChatGPT", "GPT 6.1 Sol · GPT 6.1 Terra · GPT 6 Astra · GPT 6 Luna", false),
    ("Anthropic Claude Code", "Fable 5.2 · Opus 5.5 · Sonnet 5.5 · Haiku 5.5", false),
    ("Cursor", "Gemini 3.8 Flash · Grok 4.6 · Composer 2.5 and other models", false),
    ("xAI", "Grok 4.6 · Grok 4.5", false),
    ("Google Gemini", "Gemini 3.7 Flash · Gemini 3.6 Flash · Gemini 3.1 Pro", false),
    (catalog::OLLAMA, "Models that run on this computer, no key needed", true),
    (catalog::OLLAMA_CLOUD, "Multiple models*, e.g. GPT-OSS 120B · DeepSeek V4 Pro · Kimi K3 · GLM-5.3", true),
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

/// What the "Send while the assistant is working" row says under its name.
const STEER_HINT: &str =
    "A message reaches a running assistant at once instead of waiting in the queue. Only assistants connected by API key can be steered.";
/// What the "Message box on the home screen" row says under its name.
const DASHBOARD_HINT: &str = "What you send there starts a new design.";
/// What the "Ideas under the message bar" row says under its name.
const IDEAS_HINT: &str = "Shown in an empty document while the sidebar is folded and nothing is typed.";
/// What the "Let AI work on my documents" row says under its name.
const TOOLS_HINT: &str = "The chat and AI apps on this computer can read and change open documents through `pencraft mcp`, also in the background. Off: every tool call is refused.";
/// What the "Look up model prices" row says under its name.
const PRICES_HINT: &str = "To say what an answer cost, the public price list at models.dev is fetched once a day. Nothing about you or your chats is sent. Turned off, a price shows only where the provider bills each answer or you typed prices in for a model.";
/// What the "Have finished designs reviewed" row says under its name.
const REVIEW_HINT: &str = "A second model that didn't make the design judges a picture of it like a strict art director, and the assistant works in what it wants changed. Up to two rounds; a design takes longer and costs more.";
/// What the "Generate and edit pictures" row says under its name.
const PICTURES_HINT: &str = "With the generate_image and edit_image tools, which send your pictures to the image service. Each one costs money on your key. The Generate Image dialog works either way.";
/// The longest a tool's description is on the Permissions page, characters.
const MOST_TOOL_WORDS: usize = 96;
/// What the theme control calls following the computer's appearance.
const THEME_SYSTEM: &str = "System";
/// What the theme control calls the light appearance.
const THEME_LIGHT: &str = "Light";
/// What the theme control calls the dark appearance.
const THEME_DARK: &str = "Dark";

/// What the settings window shows of the running app.
pub struct Facts<'a> {
    /// The assistants the chat can talk to.
    pub assistants: &'a [Choice],
    /// The port the local server for AI clients listens on; `None` when it isn't running.
    pub port: Option<u16>,
    /// What is known about a newer version.
    pub update: crate::update::Seen,
}

/// A page of the settings window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Page {
    /// The window's look, and updates.
    #[default]
    General,
    /// How the canvas behaves.
    Canvas,
    /// The assistants: agent programs and providers.
    Agents,
    /// The chat.
    Chat,
    /// Image model services and stock photo sites.
    Images,
    /// What AI models may do.
    Permissions,
    /// The tool server other AI apps reach Quill through.
    Mcp,
}

/// One entry of the list of pages: the page, its icon, its name.
type Entry = (Page, &'static str, &'static str);

/// The pages in the order of the list, in their groups: the group's name, then its pages.
const PAGES: [(&str, &[Entry]); 3] = [
    ("Quill", &[(Page::General, "settings", "General"), (Page::Canvas, "frame", "Canvas")]),
    ("AI", &[(Page::Agents, "sparkles", "Assistants"), (Page::Chat, "message-square", "Chat"), (Page::Images, "image", "Images")]),
    ("Access", &[(Page::Permissions, "shield-check", "Permissions"), (Page::Mcp, "terminal", "MCP")]),
];

/// What the search field finds: the page, the card's heading, and the setting's name, as the page shows them.
// Agent: a row added to a page MUST be added here too, or the search doesn't find it. Tools and providers are
// added by `finds`, from their own lists.
const INDEX: &[(Page, &str, &str)] = &[
    (Page::General, "Appearance", "Theme"),
    (Page::General, "Appearance", "Animations and effects"),
    (Page::General, "Updates", "Check now"),
    (Page::General, "Updates", "Look for new versions automatically"),
    (Page::General, "Updates", "Check every"),
    (Page::Canvas, "Grid and snapping", "Show pixel grid"),
    (Page::Canvas, "Grid and snapping", "Snap to pixel grid"),
    (Page::Canvas, "Grid and snapping", "Snap to objects"),
    (Page::Canvas, "Zoom", "Zoom with the scroll wheel"),
    (Page::Canvas, "Zoom", "Invert zoom direction"),
    (Page::Canvas, "Nudge", "Arrow keys"),
    (Page::Canvas, "Nudge", "Shift + arrow keys"),
    (Page::Canvas, "AI cursors", "Cursor size"),
    (Page::Canvas, "AI cursors", "Cursor label"),
    (Page::Canvas, "AI cursors", "Label text"),
    (Page::Canvas, "Import", "Include HTML metadata when importing pages"),
    (Page::Agents, "Your assistants", "Assistant names"),
    (Page::Agents, "Add an assistant", "Command-line agent"),
    (Page::Agents, "Add an assistant", "Custom provider"),
    (Page::Agents, "Add an assistant", "API key"),
    (Page::Chat, "Default assistant", "Answers in the chat"),
    (Page::Chat, "Custom instructions", "Custom instructions"),
    (Page::Chat, "Conversation", "Text size"),
    (Page::Chat, "Conversation", "Send while the assistant is working"),
    (Page::Chat, "Conversation", "Have finished designs reviewed"),
    (Page::Chat, "Conversation", "Look up model prices"),
    (Page::Chat, "Starting a design", "Message box on the home screen"),
    (Page::Chat, "Starting a design", "Ideas under the message bar"),
    (Page::Images, "Image generation", "Add image service"),
    (Page::Images, "Image generation", "Use by default"),
    (Page::Images, "Photo search", "Openverse and Wikimedia Commons"),
    (Page::Images, "Photo search", "Pexels"),
    (Page::Images, "Photo search", "Pixabay"),
    (Page::Permissions, "Permissions", "Let AI work on my documents"),
    (Page::Permissions, "AI may also", "Use the web"),
    (Page::Permissions, "AI may also", "Generate and edit pictures"),
    (Page::Mcp, "Server", "Quill MCP server"),
    (Page::Mcp, "Server", "Address"),
    (Page::Mcp, "Connect an AI app", "Claude Code"),
    (Page::Mcp, "Connect an AI app", "Codex"),
    (Page::Mcp, "Connect an AI app", "Cursor, Claude Desktop and others"),
];

/// The list entry of `page`.
fn entry(page: Page) -> Entry {
    PAGES.iter().flat_map(|(_, pages)| pages.iter()).find(|(known, ..)| *known == page).copied().unwrap_or(PAGES[0].1[0])
}

/// What the search for `query` finds: the page, the card's heading and the setting's name, in the pages' order.
fn finds(query: &str) -> Vec<(Page, String, String)> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let hit = |text: &str| text.to_lowercase().contains(&query);
    let own = |page: Page, card: &str, name: &str| (page, card.to_owned(), name.to_owned());
    let mut found: Vec<(Page, String, String)> = INDEX
        .iter()
        .filter(|(page, card, name)| hit(name) || hit(card) || hit(entry(*page).2))
        .map(|(page, card, name)| own(*page, card, name))
        .collect();
    found.extend(PROVIDERS.iter().filter(|(name, ..)| hit(name)).map(|(name, ..)| own(Page::Agents, "Add an assistant", name)));
    let tools = crate::agent::tools();
    let names = tools.as_array().into_iter().flatten().filter_map(|tool| tool["name"].as_str());
    found.extend(names.filter(|name| hit(name) || hit("tools")).map(|name| own(Page::Permissions, "Tools", name)));
    found.sort_by_key(|(page, ..)| PAGES.iter().flat_map(|(_, pages)| pages.iter()).position(|(known, ..)| known == page));
    found
}

/// One model row of the custom provider form, as typed.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ModelRow {
    id: String,
    context_window: String,
    max_tokens: String,
    /// Dollars for a million tokens of input; empty to go by the price list.
    input_price: String,
    /// The same for output.
    output_price: String,
}

impl ModelRow {
    fn empty() -> ModelRow {
        ModelRow {
            id: String::new(),
            context_window: DEFAULT_TOKENS.to_owned(),
            max_tokens: DEFAULT_TOKENS.to_owned(),
            input_price: String::new(),
            output_price: String::new(),
        }
    }

    /// The model this row describes, if its name and both counts are usable, and its prices are both there
    /// as numbers or both left empty.
    // Human: one price alone, or a word ("lots", or "0,075" with a comma) is no price; the row isn't saved
    // rather than saved with a price that silently counts for nothing.
    fn model(&self) -> Option<ProviderModel> {
        let count = |text: &str| text.trim().parse::<u32>().ok().filter(|n| *n > 0);
        let id = self.id.trim();
        (!id.is_empty()).then_some(())?;
        let (input, output) = (self.input_price.trim(), self.output_price.trim());
        let priced = crate::chat::usage::Rates::typed(input, output).is_some();
        (priced || (input.is_empty() && output.is_empty())).then_some(())?;
        Some(ProviderModel {
            id: id.to_owned(),
            context_window: count(&self.context_window)?,
            max_tokens: count(&self.max_tokens)?,
            input_price: input.to_owned(),
            output_price: output.to_owned(),
        })
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
    /// The models the provider said it has, once asked.
    offered: Vec<String>,
    /// What the provider said each of them can take, as far as it said.
    limits: std::collections::BTreeMap<String, catalog::Limits>,
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
            offered: Vec::new(),
            limits: std::collections::BTreeMap::new(),
        }
    }

    /// The form for connecting the known provider called `name`: everything but the key filled in.
    fn preset(name: &str, preset: catalog::Preset) -> Form {
        let models: Vec<ModelRow> =
            catalog::starting_models(name).into_iter().map(|id| ModelRow { id: id.to_owned(), ..ModelRow::empty() }).collect();
        Form {
            name: name.to_owned(),
            api: preset.api.to_owned(),
            base_url: preset.base_url.to_owned(),
            models: if models.is_empty() { vec![ModelRow::empty()] } else { models },
            ..Form::new()
        }
    }

    /// Takes what was ticked in the window for choosing models: of the models the provider `offered`, the
    /// `chosen` ones that aren't in the form are added, and the ones in the form that are no longer chosen go.
    /// Models the provider didn't offer (typed by hand) stay, and so do the numbers of a model that stays.
    fn take_choice(&mut self, offered: &[String], chosen: &std::collections::BTreeSet<String>) {
        self.models.retain(|row| !offered.iter().any(|id| id == row.id.trim()) || chosen.contains(row.id.trim()));
        for id in offered.iter().filter(|id| chosen.contains(*id)) {
            if self.models.iter().all(|row| row.id.trim() != id) {
                self.add_model(id);
            }
        }
        if self.models.is_empty() {
            self.models.push(ModelRow::empty());
        }
        self.fill();
    }

    /// Takes in what the provider said its models can take, and writes it into their rows.
    fn learn(&mut self, told: impl IntoIterator<Item = (String, catalog::Limits)>) {
        for (id, limits) in told {
            let known = self.limits.entry(id).or_default();
            *known = catalog::Limits { context: limits.context.or(known.context), output: limits.output.or(known.output) };
        }
        self.fill();
    }

    /// Writes what the provider said a model can take into the model's row, where the row still has the
    /// numbers the form put there.
    // Human: numbers the user typed are the user's; only the form's own placeholder numbers give way. A
    // provider that says how much a model reads but not how much it writes: the model can't write more than
    // it reads, so the placeholder comes down to that.
    fn fill(&mut self) {
        let placeholder: u64 = DEFAULT_TOKENS.parse().unwrap_or(u64::MAX);
        for row in &mut self.models {
            let Some(limits) = self.limits.get(row.id.trim()) else { continue };
            if let Some(context) = limits.context.filter(|_| row.context_window == DEFAULT_TOKENS) {
                row.context_window = context.to_string();
            }
            let output = limits.output.or(limits.context.filter(|context| *context < placeholder));
            if let Some(output) = output.filter(|_| row.max_tokens == DEFAULT_TOKENS) {
                row.max_tokens = output.to_string();
            }
        }
    }

    /// The chosen models an Ollama is still to be asked about one by one: it is one, and it hasn't said yet
    /// how much they read.
    fn unasked(&self) -> Vec<String> {
        if !catalog::tells_per_model(&self.api, &self.base_url) {
            return Vec::new();
        }
        let ids = self.models.iter().map(|row| row.id.trim().to_owned()).filter(|id| !id.is_empty());
        ids.filter(|id| self.limits.get(id).is_none_or(|limits| limits.context.is_none())).collect()
    }

    /// Adds the model called `id`: into the first row without a name, or a new row.
    fn add_model(&mut self, id: &str) {
        match self.models.iter_mut().find(|row| row.id.trim().is_empty()) {
            Some(row) => id.clone_into(&mut row.id),
            None => self.models.push(ModelRow { id: id.to_owned(), ..ModelRow::empty() }),
        }
    }

    fn of(index: usize, provider: &CustomProvider) -> Form {
        let models = provider
            .models
            .iter()
            .map(|m| ModelRow {
                id: m.id.clone(),
                context_window: m.context_window.to_string(),
                max_tokens: m.max_tokens.to_string(),
                input_price: m.input_price.clone(),
                output_price: m.output_price.clone(),
            })
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

/// The image service form.
#[derive(Clone, PartialEq, Eq)]
struct ImageForm {
    /// The saved service being changed; `None` for a new one.
    index: Option<usize>,
    name: String,
    kind: String,
    base_url: String,
    model: String,
    /// A newly typed key; it goes to the keychain when the form is saved, never to the settings.
    key: String,
    /// The name the service's saved key has in the keychain, if it has one.
    key_ref: Option<String>,
    /// The saved key is to be removed when the form is saved.
    forget_key: bool,
}

// Human: written out so a typed key never reaches a log or a test's message: it shows as "…".
impl std::fmt::Debug for ImageForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageForm")
            .field("index", &self.index)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("key", &if self.key.is_empty() { "" } else { "…" })
            .field("key_ref", &self.key_ref)
            .field("forget_key", &self.forget_key)
            .finish()
    }
}

impl ImageForm {
    fn new() -> ImageForm {
        ImageForm {
            index: None,
            name: String::new(),
            kind: crate::imagery::Kind::ALL[0].name().to_owned(),
            base_url: String::new(),
            model: String::new(),
            key: String::new(),
            key_ref: None,
            forget_key: false,
        }
    }

    fn of(index: usize, service: &ImageService) -> ImageForm {
        ImageForm {
            index: Some(index),
            name: service.name.clone(),
            kind: service.kind.clone(),
            base_url: service.base_url.clone(),
            model: service.model.clone(),
            key_ref: service.key_ref.clone(),
            ..ImageForm::new()
        }
    }

    /// Whether the saved key would go to another service than it was saved for: the saved service's kind or
    /// address was changed and no new key was typed (nor the saved one forgotten).
    fn key_moves(&self, images: &Images) -> bool {
        let Some(saved) = self.index.and_then(|index| images.services.get(index)) else { return false };
        saved.key_ref.is_some()
            && !self.forget_key
            && self.key.trim().is_empty()
            && (saved.kind != self.kind || saved.base_url.trim() != self.base_url.trim())
    }

    /// The service the form describes, when it has a kind Quill knows and a name no other saved service has.
    fn service(&self, images: &Images) -> Option<ImageService> {
        let name = self.name.trim();
        // Human: names are compared without regard to case, as the generate_image tool picks a service by name.
        let taken = images.services.iter().enumerate().any(|(index, other)| Some(index) != self.index && other.name.eq_ignore_ascii_case(name));
        let known = crate::imagery::Kind::named(&self.kind).is_some();
        (!name.is_empty() && !taken && known).then(|| ImageService {
            name: name.to_owned(),
            kind: self.kind.clone(),
            base_url: self.base_url.trim().to_owned(),
            model: self.model.trim().to_owned(),
            key_ref: self.key_ref.clone(),
        })
    }
}

/// The form for a command-line agent.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CliForm {
    /// The saved agent being changed; `None` for a new one.
    index: Option<usize>,
    name: String,
    kind: String,
    program: String,
    /// The models, as typed: separated by commas.
    models: String,
    arguments: String,
    /// Offered in the chat.
    on: bool,
}

impl CliForm {
    fn new() -> CliForm {
        CliForm {
            index: None,
            name: String::new(),
            kind: KIND_PLAIN.to_owned(),
            program: String::new(),
            models: String::new(),
            arguments: "{prompt}".to_owned(),
            on: true,
        }
    }

    fn of(index: usize, agent: &LocalAgent) -> CliForm {
        CliForm {
            index: Some(index),
            name: agent.name.clone(),
            kind: agent.kind.clone(),
            program: agent.program.clone(),
            models: agent.models.join(", "),
            arguments: agent.arguments.clone(),
            on: !agent.off,
        }
    }

    /// The agent the form describes, if everything needed is filled in: a name no other agent in `others`
    /// has, and for a program without a usual command where it is.
    fn agent(&self, others: &[LocalAgent]) -> Option<LocalAgent> {
        let name = self.name.trim();
        let taken = others.iter().enumerate().any(|(index, other)| Some(index) != self.index && other.name == name);
        let agent = LocalAgent {
            name: name.to_owned(),
            kind: self.kind.clone(),
            program: self.program.trim().to_owned(),
            models: self.models.split(',').map(str::trim).filter(|model| !model.is_empty()).map(str::to_owned).collect(),
            arguments: self.arguments.trim().to_owned(),
            off: !self.on,
        };
        (!name.is_empty() && !name.contains('/') && !taken && crate::chat::command_of(&agent).is_some()).then_some(agent)
    }
}

/// What a card says about a provider, an agent program, a service or the tool server.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Link {
    /// Nothing is set up.
    NotConnected,
    /// Still looking for the provider's own program on this computer.
    Checking,
    /// A provider the user set up that the chat can talk to.
    Ready,
    /// The local server for AI clients is listening.
    Running,
    /// The local server for AI clients is not listening.
    Stopped,
    /// A command-line agent's program is on this computer.
    Installed,
    /// A command-line agent's program wasn't found.
    Missing,
    /// A command-line agent the user doesn't want offered in the chat.
    Off,
    /// An image service that has no key yet.
    NoKey,
}

impl Link {
    fn words(self) -> &'static str {
        match self {
            Link::NotConnected => "Not connected",
            Link::Checking => "Checking…",
            // Human: an installed agent program that is turned on is offered in the chat, like a provider
            // that is set up; both say the same.
            Link::Ready | Link::Installed => "Ready",
            Link::Running => "Running",
            Link::Stopped => "Not running",
            Link::Missing => "Not installed",
            Link::Off => "Turned off",
            Link::NoKey => "No key yet",
        }
    }

    fn tone(self) -> Tone {
        match self {
            Link::NotConnected | Link::Stopped | Link::Missing | Link::Off | Link::NoKey => Tone::Quiet,
            Link::Checking => Tone::Busy,
            Link::Ready | Link::Running | Link::Installed => Tone::Good,
        }
    }
}

/// A state pill with its right edge at `right`.
fn status(ui: &Ui, right: egui::Pos2, link: Link, palette: Palette) {
    pill_at(ui, right, link.words(), link.tone(), palette);
}

/// The connection state of the provider called `name`: ready once it is set up with what the chat needs to
/// reach it (for most, an API key).
// Human: a provider's card is about its API only. That Claude Code is installed shows on its own row under
// "Your assistants", not on Anthropic's, which would then claim a key that was never set.
fn provider_link(name: &str, prefs: &Prefs) -> Link {
    let ready = prefs.agents.providers.iter().any(|provider| provider.name == name && crate::chat::usable(provider));
    if ready { Link::Ready } else { Link::NotConnected }
}

/// Whether the assistant the app calls `plain` ("Claude Code · Opus 5.5") belongs to the agent program or
/// provider called `name`.
fn owns(name: &str, plain: &str) -> bool {
    plain.strip_prefix(name).is_some_and(|rest| rest.is_empty() || rest.starts_with(" · "))
}

/// "1 model", "3 models".
fn counted(count: usize, one: &str) -> String {
    if count == 1 { format!("1 {one}") } else { format!("{count} {one}s") }
}

/// How long ago something was, in words.
fn ago(time: Duration) -> String {
    match time.as_secs() {
        0..60 => "just now".to_owned(),
        seconds @ 60..3600 => format!("{} ago", counted(usize::try_from(seconds / 60).unwrap_or(usize::MAX), "minute")),
        seconds => format!("{} ago", counted(usize::try_from(seconds / 3600).unwrap_or(usize::MAX), "hour")),
    }
}

/// What the Updates card says about a newer version.
fn update_line(seen: &crate::update::Seen) -> String {
    match (seen.looking, &seen.offered, seen.ago) {
        (true, ..) => "Looking for a newer version…".to_owned(),
        (false, Some(version), _) => format!("Quill {version} is available."),
        (false, None, Some(time)) => format!("Last checked {}.", ago(time)),
        (false, None, None) => "Not checked yet since Quill started.".to_owned(),
    }
}

/// What a provider says about its models, once it has answered.
type Models = Result<Vec<catalog::Offered>, String>;
/// What an Ollama said its models can take, asked one by one.
type Told = Vec<(String, catalog::Limits)>;

/// The settings window's state.
#[expect(clippy::struct_excessive_bools, reason = "each says one independent thing about what the window shows")]
pub struct Settings {
    /// Whether the window shows.
    pub open: bool,
    /// The user asked to look for a newer version now; the app takes this away and looks.
    pub check_now: bool,
    page: Page,
    /// What is typed into the search field over the list of pages.
    query: String,
    /// What is typed into the search field over the providers.
    provider_query: String,
    /// Every provider shows, not only the first few.
    all_providers: bool,
    /// Every tool shows, not only the first few.
    all_tools: bool,
    /// The AI app the MCP page shows the setup for, as [`client_setups`] names it.
    client: String,
    /// The stock photo site (its place in [`crate::web::KEYED`]) whose key is being typed.
    picture_editing: Option<usize>,
    /// The custom provider form, when it is open on the Assistants page.
    form: Option<Form>,
    /// The image service form, when it is open on the Images page.
    image_form: Option<ImageForm>,
    /// The command-line agent form, when it is open on the Assistants page.
    cli: Option<CliForm>,
    /// The search for the command-line agents' programs, while it runs.
    search: Option<mpsc::Receiver<Vec<(String, bool)>>>,
    /// Which commands are installed; `None` until the search has finished.
    installed: Option<Vec<(String, bool)>>,
    /// Adding Quill to an agent program's MCP servers, while that runs; it answers with what to tell the user.
    hooking: Option<mpsc::Receiver<String>>,
    /// What is typed into the stock photo sites' key fields, in the order of [`crate::web::KEYED`].
    picture_drafts: [String; 2],
    /// The question to a provider which models it has, while it is out.
    loading: Option<mpsc::Receiver<Models>>,
    /// The questions to an Ollama what the chosen models can take, while they are out.
    telling: Option<mpsc::Receiver<Told>>,
    /// That question was asked to test the connection, so its answer is said.
    testing: bool,
    /// The window the form's provider's models are chosen in, while it is open over the form.
    picker: Option<models::Picker>,
    /// Where API keys are kept.
    vault: Box<dyn Vault>,
    /// What the user asked to remove, while they are asked whether they are sure.
    removing: Option<Removal>,
}

/// Something in the settings that takes a key out of the keychain when removed, so it is asked about first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Removal {
    /// The custom provider at this place in the settings, with its key.
    Provider(usize),
    /// The image service at this place in the settings, with its key.
    Service(usize),
    /// The key of the stock photo site at this place in [`crate::web::KEYED`].
    PictureKey(usize),
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            open: false,
            check_now: false,
            page: Page::default(),
            query: String::new(),
            provider_query: String::new(),
            all_providers: false,
            all_tools: false,
            client: client_setups("pencraft")[0].0.to_owned(),
            picture_editing: None,
            form: None,
            image_form: None,
            cli: None,
            search: None,
            installed: None,
            hooking: None,
            picture_drafts: [String::new(), String::new()],
            loading: None,
            telling: None,
            testing: false,
            picker: None,
            vault: Box::new(Keychain),
            removing: None,
        }
    }
}

/// What a click in the list of assistants opens.
enum Open {
    /// The command-line agent at this place.
    Local(usize),
    /// The saved provider at this place.
    Provider(usize),
}

impl Settings {
    /// Opens the window on the page called `name` on the command line (`--settings`): `general`, `canvas`,
    /// `agents`, `provider` for the custom provider form, `cli` for the command-line agent form, `chat`,
    /// `images`, `image-service` for the image service form, `permissions` or `mcp`; `models` for the window a
    /// provider's models are chosen in, over the provider form, and `models-search` for a search in it.
    pub fn open_named(&mut self, name: &str) {
        self.open = true;
        self.page = match name {
            "general" => Page::General,
            "canvas" => Page::Canvas,
            "images" | "image-service" => Page::Images,
            "chat" => Page::Chat,
            "permissions" => Page::Permissions,
            "mcp" => Page::Mcp,
            _ => Page::Agents,
        };
        self.query.clear();
        self.form = matches!(name, "provider" | "models" | "models-search").then(Form::new);
        self.picker = match name {
            "models" => Some(models::Picker::sample("")),
            "models-search" => Some(models::Picker::sample("kpro")),
            _ => None,
        };
        self.cli = (name == "cli").then(CliForm::new);
        self.image_form = (name == "image-service").then(ImageForm::new);
        self.installed = None;
    }

    /// Looks for the command-line agents' programs on another thread, and takes the answer when it is there.
    // Human: looking through the folders of PATH can take a moment on a slow disk; the cards say "Checking
    // status..." meanwhile.
    // Agent: SPAWNS one short-lived thread per opening of the window; it WAKES the UI when done. If the thread
    // can't start, nothing counts as installed.
    fn search(&mut self, ctx: &egui::Context, agents: &Agents) {
        if self.installed.is_some() {
            return;
        }
        if let Some(search) = &self.search {
            match search.try_recv() {
                Ok(found) => self.installed = Some(found),
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => self.installed = Some(Vec::new()),
            }
            self.search = None;
            return;
        }
        let commands: Vec<String> = agents.local.iter().filter_map(crate::chat::command_of).map(str::to_owned).collect();
        let (send, receive) = mpsc::channel();
        let wake = ctx.clone();
        let started = std::thread::Builder::new().name("find-assistants".to_owned()).spawn(move || {
            let found = commands.into_iter().map(|command| (locate(&command).is_some(), command)).map(|(found, command)| (command, found)).collect();
            // The window may have closed meanwhile; then nobody listens, which is fine.
            let _unheard = send.send(found);
            wake.request_repaint();
        });
        match started {
            Ok(_) => self.search = Some(receive),
            Err(e) => {
                log::warn!("couldn't look for installed assistants: {e}");
                self.installed = Some(Vec::new());
            }
        }
    }

    /// Whether the program called by `command` is installed; `None` while the search runs.
    fn found(&self, command: &str) -> Option<bool> {
        self.installed.as_ref().map(|all| all.iter().any(|(known, found)| known == command && *found))
    }

    /// What a command-line agent's card says.
    fn local_link(&self, agent: &LocalAgent) -> Link {
        match crate::chat::command_of(agent).map(|command| self.found(command)) {
            _ if agent.off => Link::Off,
            Some(None) => Link::Checking,
            Some(Some(true)) => Link::Installed,
            Some(Some(false)) | None => Link::Missing,
        }
    }

    /// Opens the form for the known provider called `name`: the one saved under that name, or a new one with
    /// what is known about it filled in. Returns what to tell the user when it can't be connected.
    fn open_known(&mut self, name: &str, prefs: &Prefs) -> Option<String> {
        self.loading = None;
        let saved = prefs.agents.providers.iter().enumerate().find(|(_, provider)| provider.name == name);
        match (saved, catalog::preset(name)) {
            (Some((index, provider)), _) => self.form = Some(Form::of(index, provider)),
            (None, Some(preset)) => self.form = Some(Form::preset(name, preset)),
            (None, None) => return Some(format!("{name}{NO_KEY_API}")),
        }
        None
    }

    /// Opens the window on its first page.
    pub fn open_general(&mut self) {
        self.open = true;
        self.page = Page::General;
        self.query.clear();
        self.leave_forms();
        self.installed = None;
    }

    /// Closes whatever form is open, without saving it.
    fn leave_forms(&mut self) {
        self.form = None;
        self.image_form = None;
        self.cli = None;
        self.loading = None;
        self.telling = None;
        self.picker = None;
        self.picture_editing = None;
    }

    /// Tidies up after the window, every frame it is closed.
    // Human: however the window was closed (its cross, Escape, a click beside it, or the app), an image
    // service form left open goes, and a key typed in it with it.
    pub fn closed(&mut self) {
        self.image_form = None;
    }

    /// Draws the window when it is open and applies changes to `prefs`. Returns a sentence to tell the user, if
    /// a control asks for one.
    // Agent: MAIN-THREAD; WRITES prefs only; the caller saves them and follows a changed appearance.
    pub fn show(&mut self, ctx: &egui::Context, prefs: &mut Prefs, facts: &Facts<'_>, dark: bool) -> Option<String> {
        if !self.open {
            self.closed();
            return None;
        }
        self.search(ctx, &prefs.agents);
        let palette = theme::palette(dark);
        let mut notice = None;
        let screen = ctx.content_rect();
        let size =
            vec2((screen.width() - 2.0 * SETTINGS_MARGIN[0]).min(SETTINGS_MAX[0]), (screen.height() - 2.0 * SETTINGS_MARGIN[1]).min(SETTINGS_MAX[1]));
        let frame = egui::Frame::new().fill(palette.island).stroke(Stroke::new(1.0, palette.outline)).corner_radius(SETTINGS_RADIUS);
        // Human: the window appears at once; a fade would show the canvas through it for a moment.
        let id = egui::Id::unique("settings");
        let modal = egui::Modal::new(id)
            .area(egui::Modal::default_area(id).fade_in(false))
            .frame(frame)
            .backdrop_color(egui::Color32::from_black_alpha(DIALOG_BACKDROP))
            .show(ctx, |ui| {
                let (window, _) = ui.allocate_exact_size(size, Sense::hover());
                self.pages(ui, window, facts, palette);
                let searching = !self.query.trim().is_empty();
                // Human: a form's buttons stay in a bar under the page, so they are there however far the
                // form is scrolled.
                let form = !searching && (self.form.is_some() || self.cli.is_some() || self.image_form.is_some());
                let bar = if form { SETTINGS_FOOTER } else { 0.0 };
                let area = Rect::from_min_max(pos2(window.left() + SETTINGS_NAV_WIDTH + 1.0, window.top() + 1.0), window.max - vec2(1.0, 1.0));
                let page = Rect::from_min_max(pos2(area.left() + SETTINGS_PAGE_INSET, area.top()), pos2(area.right() - 13.0, area.bottom() - bar));
                ui.scope_builder(UiBuilder::new().max_rect(page), |ui| {
                    ui.set_clip_rect(page.intersect(ui.clip_rect()));
                    egui::ScrollArea::vertical().id_salt(("settings-page", self.page, form, searching)).auto_shrink([false, false]).show(ui, |ui| {
                        // Human: room on the right for the close cross and the scroll bar.
                        ui.set_max_width((page.width() - 40.0).min(SETTINGS_PAGE_WIDTH));
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.add_space(30.0);
                        if searching {
                            self.results(ui, palette);
                        } else {
                            match self.page {
                                Page::General => general(ui, prefs, facts, palette, &mut self.check_now),
                                Page::Canvas => canvas_page(ui, prefs, palette),
                                Page::Agents => self.agents(ui, prefs, facts, palette, &mut notice),
                                Page::Chat => chat_page(ui, &mut prefs.agents, facts, palette),
                                Page::Images => self.images(ui, prefs, palette, &mut notice),
                                Page::Permissions => self.permissions(ui, prefs, palette),
                                Page::Mcp => {
                                    if mcp_page(ui, prefs, facts, &mut self.client, palette, &mut notice) {
                                        self.page = Page::Permissions;
                                    }
                                }
                            }
                        }
                        ui.add_space(36.0);
                    });
                });
                if form {
                    let bar = Rect::from_min_max(pos2(area.left(), area.bottom() - bar), area.max);
                    ui.painter().hline(bar.x_range(), bar.top(), Stroke::new(1.0, palette.outline));
                    let inner = Rect::from_min_max(pos2(bar.left() + SETTINGS_PAGE_INSET, bar.top()), pos2(bar.right() - 20.0, bar.bottom()));
                    let buttons = UiBuilder::new().max_rect(inner).layout(Layout::right_to_left(Align::Center));
                    ui.scope_builder(buttons, |ui| self.footer(ui, prefs, palette, &mut notice));
                }
                let close = Rect::from_min_size(pos2(window.right() - 44.0, window.top() + 16.0), Vec2::splat(28.0));
                ui.scope_builder(UiBuilder::new().max_rect(close), |ui| {
                    if icon_button(ui, "x", close.size(), false, "Close", palette).clicked() {
                        self.open = false;
                    }
                });
            });
        if modal.should_close() {
            self.open = false;
        }
        self.choose_models(ctx, palette);
        self.confirm_removal(ctx, prefs, palette, &mut notice);
        notice
    }

    /// Asks whether to remove what [`Settings::removing`] names, saying what goes and that its key leaves the
    /// keychain; removes it on yes.
    // Human: a removed key can't be got back from Quill; the user would have to find it at the provider again.
    // Cancel is the main button, so Return and Escape both keep everything.
    // Agent: drawn after the settings window, so it lies over it and takes Escape first. WRITES the keychain,
    // then prefs, as the remove functions do.
    fn confirm_removal(&mut self, ctx: &egui::Context, prefs: &mut Prefs, palette: Palette, notice: &mut Option<String>) {
        let Some(removal) = self.removing.filter(|_| self.open) else {
            self.removing = None;
            return;
        };
        let (what, name) = match removal {
            Removal::Provider(index) => ("provider", prefs.agents.providers.get(index).map(|p| p.name.clone())),
            Removal::Service(index) => ("image service", prefs.images.services.get(index).map(|s| s.name.clone())),
            Removal::PictureKey(index) => ("key", crate::web::KEYED.get(index).map(|(site, ..)| format!("{site} key"))),
        };
        let Some(name) = name else {
            self.removing = None;
            return;
        };
        let mut choice = None;
        let modal = egui::Modal::new(egui::Id::unique("settings-remove")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(RichText::new(format!("Remove “{name}”?")).strong());
            ui.label(if what == "key" {
                "The key will be deleted from your system's keychain.".to_owned()
            } else {
                format!("The {what} and its key in your system's keychain will be deleted.")
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.add(crate::app::dialog_primary("Cancel", palette)).clicked() {
                    choice = Some(false);
                }
                if ui.add(crate::app::dialog_button("Remove")).clicked() {
                    choice = Some(true);
                }
            });
        });
        if choice.is_none() && (modal.should_close() || ctx.input(|i| i.key_pressed(egui::Key::Enter))) {
            choice = Some(false);
        }
        let Some(remove) = choice else { return };
        self.removing = None;
        if !remove {
            return;
        }
        let vault: &dyn Vault = self.vault.as_ref();
        let done = match removal {
            Removal::Provider(index) => remove_provider(index, prefs, vault),
            Removal::Service(index) => remove_image_service(index, prefs, vault),
            Removal::PictureKey(index) => {
                let Some((site, key_name, _)) = crate::web::KEYED.get(index) else { return };
                vault
                    .forget(key_name)
                    .map(|()| prefs.agents.picture_keys.retain(|known| known != site))
                    .map_err(|why| format!("The key couldn't be removed from the keychain: {why}"))
            }
        };
        match done {
            // Human: the form of what was removed has nothing left to show.
            Ok(()) if !matches!(removal, Removal::PictureKey(_)) => {
                self.form = None;
                self.image_form = None;
                self.loading = None;
                self.installed = None;
            }
            Ok(()) => {}
            Err(why) => *notice = Some(why),
        }
    }

    /// Draws the window the form's provider's models are chosen in, when it is open, and does what was decided
    /// in it.
    // Agent: drawn after the settings window, so it lies over it and takes Escape first.
    fn choose_models(&mut self, ctx: &egui::Context, palette: Palette) {
        // Human: what an Ollama says about the chosen models comes in a moment after they are chosen.
        if let (Some(form), Some(Ok(told))) = (&mut self.form, self.telling.as_ref().map(mpsc::Receiver::try_recv)) {
            form.learn(told);
            self.telling = None;
        }
        let (Some(picker), Some(form), true) = (&mut self.picker, &mut self.form, self.open) else {
            self.picker = None;
            return;
        };
        match picker.show(ctx, palette) {
            Some(models::Answer::Apply) => {
                form.take_choice(picker.offered(), picker.chosen());
                self.telling = ask_limits(form, self.vault.as_ref(), ctx);
                self.picker = None;
            }
            Some(models::Answer::Cancel) => self.picker = None,
            Some(models::Answer::Reload) => match ask_models(form, self.vault.as_ref(), ctx) {
                Ok(answer) => {
                    self.loading = Some(answer);
                    self.testing = false;
                    picker.loading();
                }
                Err(why) => picker.failed(why),
            },
            None => {}
        }
    }

    /// The left of the window: its name, the search field, the list of pages in their groups, and the version.
    fn pages(&mut self, ui: &mut Ui, window: Rect, facts: &Facts<'_>, palette: Palette) {
        let line = window.left() + SETTINGS_NAV_WIDTH;
        ui.painter().vline(line, window.y_range().shrink(1.0), Stroke::new(1.0, palette.outline));
        let inner = Rect::from_min_max(pos2(window.left() + 14.0, window.top() + 22.0), pos2(line - 14.0, window.bottom() - 18.0));
        ui.painter().text(pos2(inner.left() + 10.0, inner.top() + 14.0), Align2::LEFT_CENTER, "Settings", strong(15.0), palette.foreground);
        let search = Rect::from_min_size(pos2(inner.left(), inner.top() + 44.0), vec2(inner.width(), FIELD_HEIGHT));
        ui.scope_builder(UiBuilder::new().max_rect(search), |ui| {
            search_field(ui, &mut self.query, "Search settings", palette);
        });
        let searching = !self.query.trim().is_empty();
        let mut top = search.bottom() + 16.0;
        let mut go = None;
        for (group, pages) in PAGES {
            ui.painter().text(pos2(inner.left() + 10.0, top + 11.0), Align2::LEFT_CENTER, group, strong(11.0), palette.muted);
            top += 24.0;
            for &(page, icon, name) in pages {
                let row = Rect::from_min_size(pos2(inner.left(), top), vec2(inner.width(), SETTINGS_NAV_ROW));
                top = row.bottom() + 2.0;
                let response = ui.interact(row, ui.scope_id().with(("settings-page", name)), Sense::click());
                let active = self.page == page && !searching;
                if active || response.hovered() {
                    ui.painter().rect_filled(row, CHIP_RADIUS, if active { palette.hover } else { palette.hover.gamma_multiply(0.5) });
                }
                let color = if active || response.hovered() { palette.foreground } else { palette.muted };
                icons::paint(
                    ui.painter(),
                    Rect::from_center_size(pos2(row.left() + 17.0, row.center().y), Vec2::splat(ICON_SIZE - 1.0)),
                    icon,
                    color,
                );
                ui.painter().text(pos2(row.left() + 35.0, row.center().y), Align2::LEFT_CENTER, name, FontId::proportional(SIDEBAR_TEXT_SIZE), color);
                if response.clicked() {
                    go = Some(page);
                }
            }
            top += 14.0;
        }
        if let Some(page) = go {
            self.page = page;
            self.query.clear();
            self.leave_forms();
        }
        let foot = pos2(inner.left() + 10.0, inner.bottom() - 8.0);
        let version = format!("Quill {}", crate::update::VERSION);
        ui.painter().text(foot, Align2::LEFT_CENTER, version, FontId::proportional(11.5), palette.muted);
        if facts.update.offered.is_some() {
            let right = pos2(inner.right() - 10.0, foot.y);
            ui.painter().text(right, Align2::RIGHT_CENTER, "Update available", FontId::proportional(11.5), palette.foreground);
        }
    }

    /// What the search field found, in the page's place; a click goes to the setting's page.
    fn results(&mut self, ui: &mut Ui, palette: Palette) {
        let found = finds(&self.query);
        title(ui, "Search", palette);
        about(ui, &format!("{} for “{}”", counted(found.len(), "setting"), self.query.trim()), palette);
        ui.add_space(18.0);
        if found.is_empty() {
            ui.label(RichText::new("No setting has that in its name.").color(palette.muted));
            return;
        }
        let mut go = None;
        card(ui, palette, |rows| {
            for (page, group, name) in &found {
                let (_, icon, page_name) = entry(*page);
                if rows.link(Lead::Icon(icon), name, &format!("{page_name} › {group}"), None).clicked() {
                    go = Some(*page);
                }
            }
        });
        if let Some(page) = go {
            self.page = page;
            self.query.clear();
            self.leave_forms();
        }
    }

    /// The bar under a form: what is still missing on the left, the form's buttons on the right.
    // Agent: laid out from the right. WRITES prefs and the keychain on Save and Remove.
    fn footer(&mut self, ui: &mut Ui, prefs: &mut Prefs, palette: Palette, notice: &mut Option<String>) {
        ui.spacing_mut().item_spacing.x = 8.0;
        let Settings { form, cli, image_form, loading, testing, installed, vault, removing, .. } = self;
        let vault: &dyn Vault = &**vault;
        let mut finished = false;
        if let Some(open) = form {
            let provider = open.provider();
            let label = if open.index.is_some() { "Save provider" } else { "Add provider" };
            if button(ui, label, None, Kind::Primary, provider.is_some(), palette).clicked()
                && let Some(provider) = provider
            {
                match save_provider(open, provider, prefs, vault) {
                    Ok(()) => finished = true,
                    Err(why) => *notice = Some(why),
                }
            }
            let busy = loading.is_some();
            let test = if busy && *testing { "Testing…" } else { "Test connection" };
            if button(ui, test, None, Kind::Outline, !busy && !open.base_url.trim().is_empty(), palette).clicked() {
                match ask_models(open, vault, ui.ctx()) {
                    Ok(answer) => {
                        *loading = Some(answer);
                        *testing = true;
                    }
                    Err(why) => *notice = Some(why),
                }
            }
            if let Some(index) = open.index
                && button(ui, "Remove", None, Kind::Quiet, true, palette).clicked()
            {
                *removing = Some(Removal::Provider(index));
            }
            finished |= button(ui, "Cancel", None, Kind::Quiet, true, palette).clicked();
            missing(ui, open.missing(), palette);
        } else if let Some(open) = cli {
            let agent = open.agent(&prefs.agents.local);
            let label = if open.index.is_some() { "Save agent" } else { "Add agent" };
            if button(ui, label, None, Kind::Primary, agent.is_some(), palette).clicked()
                && let Some(agent) = agent
            {
                match open.index.and_then(|index| prefs.agents.local.get_mut(index)) {
                    Some(saved) => *saved = agent,
                    None => prefs.agents.local.push(agent),
                }
                finished = true;
            }
            if let Some(index) = open.index
                && button(ui, "Remove", None, Kind::Quiet, true, palette).clicked()
            {
                if index < prefs.agents.local.len() {
                    prefs.agents.local.remove(index);
                }
                finished = true;
            }
            finished |= button(ui, "Cancel", None, Kind::Quiet, true, palette).clicked();
            missing(ui, open.missing(&prefs.agents.local), palette);
        } else if let Some(open) = image_form {
            let service = open.service(&prefs.images);
            let label = if open.index.is_some() { "Save service" } else { "Add service" };
            if button(ui, label, None, Kind::Primary, service.is_some(), palette).clicked()
                && let Some(service) = service
            {
                match save_image_service(open, service, prefs, vault) {
                    Ok(()) => finished = true,
                    Err(why) => *notice = Some(why),
                }
            }
            if let Some(index) = open.index
                && button(ui, "Remove", None, Kind::Quiet, true, palette).clicked()
            {
                *removing = Some(Removal::Service(index));
            }
            finished |= button(ui, "Cancel", None, Kind::Quiet, true, palette).clicked();
            missing(ui, open.missing(&prefs.images), palette);
        }
        if finished {
            *form = None;
            *cli = None;
            *image_form = None;
            *loading = None;
            // What is installed is looked up again, for the rows.
            *installed = None;
        }
    }

    /// The Assistants page: the assistants that are set up and the ones to add, or the form for one.
    fn agents(&mut self, ui: &mut Ui, prefs: &mut Prefs, facts: &Facts<'_>, palette: Palette, notice: &mut Option<String>) {
        if let Some(form) = &mut self.form {
            let asking = Asking { loading: &mut self.loading, testing: &mut self.testing, picker: &mut self.picker };
            if provider_form(ui, form, prefs, facts, self.vault.as_ref(), asking, palette, notice) {
                self.form = None;
                self.loading = None;
            }
            return;
        }
        if let Some(form) = &mut self.cli {
            if cli_form(ui, form, &mut prefs.agents, facts, &mut self.hooking, palette, notice) {
                self.cli = None;
                // What is installed is looked up again, for the rows.
                self.installed = None;
            }
            return;
        }
        title(ui, "Assistants", palette);
        about(ui, "The AI models that design with you. Set up as many as you like and switch between them in the chat.", palette);
        heading(ui, "Your assistants", palette);
        // Human: the agent programs come first, each saying whether its program is installed; the providers
        // set up with a key follow. A click opens its settings.
        let default = crate::chat::chosen(facts.assistants, &prefs.agents.chat_model).map(|choice| choice.plain.clone());
        let tag = |name: &str| default.as_deref().filter(|plain| owns(name, plain)).map(|_| "Default");
        let local: Vec<(String, String, Link)> = prefs
            .agents
            .local
            .iter()
            .map(|agent| {
                let command = crate::chat::command_of(agent).unwrap_or("no program set");
                (agent.name.clone(), format!("On this computer · {command}"), self.local_link(agent))
            })
            .collect();
        let saved: Vec<(String, String, Link)> = prefs
            .agents
            .providers
            .iter()
            .map(|provider| {
                // Human: a model server on one's own computer has no key; its card doesn't claim one.
                let key = if provider.key_ref.is_some() { "API key" } else { "No key" };
                (provider.name.clone(), format!("{key} · {}", counted(provider.models.len(), "model")), provider_link(&provider.name, prefs))
            })
            .collect();
        let mut open = None;
        card(ui, palette, |rows| {
            for (index, (name, line, link)) in local.iter().enumerate() {
                let row = rows.link(Lead::Letter(name), name, line, tag(name));
                status(rows.ui, link_state(row.rect), *link, palette);
                if row.clicked() {
                    open = Some(Open::Local(index));
                }
            }
            for (index, (name, line, link)) in saved.iter().enumerate() {
                let row = rows.link(Lead::Letter(name), name, line, tag(name));
                status(rows.ui, link_state(row.rect), *link, palette);
                if row.clicked() {
                    open = Some(Open::Provider(index));
                }
            }
            if local.is_empty() && saved.is_empty() {
                rows.block(|ui| {
                    ui.label(RichText::new("No assistant is set up yet. Add one below.").color(palette.muted));
                });
            }
        });
        match open {
            Some(Open::Local(index)) => self.cli = prefs.agents.local.get(index).map(|agent| CliForm::of(index, agent)),
            Some(Open::Provider(index)) => {
                self.loading = None;
                self.form = prefs.agents.providers.get(index).map(|provider| Form::of(index, provider));
            }
            None => {}
        }
        self.offered_assistants(ui, prefs, palette, notice);
    }

    /// The Assistants page's second half: a field to search the known providers, the providers as tiles, and
    /// the two kinds the user describes themselves.
    fn offered_assistants(&mut self, ui: &mut Ui, prefs: &Prefs, palette: Palette, notice: &mut Option<String>) {
        heading(ui, "Add an assistant", palette);
        hint(
            ui,
            "Agent programs use their own sign-in. Ollama runs on this computer and needs no key. Every other provider connects with your API key, kept in the system keychain.",
            palette,
        );
        search_field(ui, &mut self.provider_query, &format!("Search {} providers", PROVIDERS.len()), palette);
        ui.add_space(8.0);
        let query = self.provider_query.trim().to_lowercase();
        // Human: a provider that is set up is in the list above, not offered again.
        let offered: Vec<(&str, &str)> = PROVIDERS
            .iter()
            .filter(|(name, ..)| prefs.agents.providers.iter().all(|provider| provider.name != *name))
            .filter(|(name, models, _)| query.is_empty() || name.to_lowercase().contains(&query) || models.to_lowercase().contains(&query))
            .map(|(name, models, _)| (*name, *models))
            .collect();
        let shown = if self.all_providers || !query.is_empty() { offered.len() } else { offered.len().min(SETTINGS_PROVIDERS_SHOWN) };
        ui.spacing_mut().item_spacing.x = 8.0;
        let mut picked = None;
        for pair in offered[..shown].chunks(2) {
            ui.columns(2, |columns| {
                for (column, (name, models)) in columns.iter_mut().zip(pair) {
                    if tile_button(column, Lead::Letter(name), name, models, true, palette).clicked() {
                        picked = Some(*name);
                    }
                }
            });
            ui.add_space(8.0);
        }
        ui.columns(2, |columns| {
            if tile_button(&mut columns[0], Lead::Icon("terminal"), "Command-line agent", "A program on this computer", false, palette).clicked() {
                self.cli = Some(CliForm::new());
            }
            if tile_button(&mut columns[1], Lead::Icon("server"), "Custom provider", "OpenAI or Anthropic compatible API", false, palette).clicked() {
                self.loading = None;
                self.form = Some(Form::new());
            }
        });
        if shown < offered.len() {
            ui.add_space(8.0);
            let more = format!("Show {}", counted(offered.len() - shown, "more provider"));
            if button(ui, &more, Some("chevron-down"), Kind::Quiet, true, palette).clicked() {
                self.all_providers = true;
            }
        }
        ui.add_space(10.0);
        let footnote = "*Not all models from each provider are supported by Quill, we are working on validating and adding more models as they fit.";
        ui.add(egui::Label::new(RichText::new(footnote).size(11.0).color(palette.muted)).wrap());
        if let Some(name) = picked {
            *notice = self.open_known(name, prefs);
        }
    }

    /// The Images page: the image model services and the stock photo sites, or the form for a service.
    fn images(&mut self, ui: &mut Ui, prefs: &mut Prefs, palette: Palette, notice: &mut Option<String>) {
        if let Some(form) = &mut self.image_form {
            if image_form(ui, form, prefs, palette) {
                self.image_form = None;
            }
            return;
        }
        title(ui, "Images", palette);
        heading(ui, "Image generation", palette);
        hint(
            ui,
            "Services that make pictures from a description. Add the ones you have an API key for; each picture is paid for on that key.",
            palette,
        );
        let ready = crate::imagery::ready(&prefs.images, &|name| std::env::var(name).ok());
        let saved: Vec<(String, String, Link)> = prefs
            .images
            .services
            .iter()
            .map(|service| {
                let usable = ready.iter().any(|ready| ready.name == service.name);
                (service.name.clone(), service_about(service), if usable { Link::Ready } else { Link::NoKey })
            })
            .collect();
        // Human: the choice offers the services that can be used now. An empty default means "the first service
        // that works"; a saved default that can't be used now shows as that too, but stays saved until the
        // person picks another, so a key that comes back brings it back.
        let usable: Vec<String> = ready.iter().map(|ready| ready.name.clone()).collect();
        let names: Vec<&str> = std::iter::once(ANY_SERVICE).chain(usable.iter().map(String::as_str)).collect();
        let shown = shown_default(&usable, &prefs.images.default);
        let mut chosen = shown.clone();
        let mut open = None;
        card(ui, palette, |rows| {
            for (index, (name, line, link)) in saved.iter().enumerate() {
                let row = rows.link(Lead::Letter(name), name, line, None);
                status(rows.ui, link_state(row.rect), *link, palette);
                if row.clicked() {
                    open = Some(Some(index));
                }
            }
            // Human: a service from the environment can't be opened: it is set where Quill was started, not here.
            for (name, line) in ready.iter().filter_map(environment_line) {
                rows.row(name, Some(&line), Lead::Letter(name), |ui| pill(ui, Link::Ready.words(), Tone::Good, palette));
            }
            if rows.link(Lead::Icon("plus"), "Add image service", "", None).clicked() {
                open = Some(None);
            }
            rows.row("Use by default", None, Lead::Nothing, |ui| choice(ui, "image-default", &mut chosen, &names, palette));
        });
        if chosen != shown {
            prefs.images.default = if chosen == ANY_SERVICE { String::new() } else { chosen };
        }
        match open {
            Some(Some(index)) => self.image_form = prefs.images.services.get(index).map(|service| ImageForm::of(index, service)),
            Some(None) => self.image_form = Some(ImageForm::new()),
            None => {}
        }
        self.picture_sources(ui, &mut prefs.agents, palette, notice);
    }

    /// Where assistants look for stock photos: the two sources that need no key, and a row for each stock photo
    /// site, which takes a key.
    // Agent: WRITES the keychain first and the settings only after that worked, so the settings never name a key
    // that isn't there.
    fn picture_sources(&mut self, ui: &mut Ui, agents: &mut Agents, palette: Palette, notice: &mut Option<String>) {
        heading(ui, "Photo search", palette);
        let why = "Where assistants look for stock photos. With a free key of your own, these sites are searched too, so there is more to choose from. Keys are kept in your system's keychain.";
        hint(ui, why, palette);
        let Settings { picture_drafts, picture_editing, vault, removing, .. } = self;
        card(ui, palette, |rows| {
            rows.row("Openverse and Wikimedia Commons", Some("Free to search, no key needed."), Lead::Icon("globe"), |ui| {
                pill(ui, "Always on", Tone::Good, palette);
            });
            for (index, ((name, key_name, where_from), draft)) in crate::web::KEYED.iter().zip(picture_drafts.iter_mut()).enumerate() {
                let saved = agents.picture_keys.iter().any(|known| known == name);
                let editing = *picture_editing == Some(index);
                let said = if saved { "Key saved in your keychain.".to_owned() } else { format!("Free key at {where_from}") };
                rows.row(name, Some(&said), Lead::Letter(name), |ui| {
                    if editing {
                        if button(ui, "Cancel", None, Kind::Quiet, true, palette).clicked() {
                            draft.clear();
                            *picture_editing = None;
                        }
                        if button(ui, "Save", None, Kind::Primary, !draft.trim().is_empty(), palette).clicked() {
                            match vault.put(key_name, &SecretString::new(draft.trim())) {
                                Ok(()) => {
                                    draft.clear();
                                    *picture_editing = None;
                                    if !saved {
                                        agents.picture_keys.push((*name).to_owned());
                                    }
                                }
                                Err(why) => *notice = Some(format!("The key couldn't be saved in the keychain: {why}")),
                            }
                        }
                        sized_field(ui, draft, "API key", 170.0, true, true, palette);
                    } else if saved {
                        if button(ui, "Forget", None, Kind::Quiet, true, palette).clicked() {
                            *removing = Some(Removal::PictureKey(index));
                        }
                        if button(ui, "Replace", None, Kind::Outline, true, palette).clicked() {
                            *picture_editing = Some(index);
                        }
                    } else if button(ui, "Add key", Some("key-round"), Kind::Outline, true, palette).clicked() {
                        *picture_editing = Some(index);
                    }
                });
            }
        });
    }

    /// The Permissions page: everything AI models may do, for the chat and for every connected app.
    fn permissions(&mut self, ui: &mut Ui, prefs: &mut Prefs, palette: Palette) {
        title(ui, "Permissions", palette);
        about(ui, "One place for everything AI is allowed to do. It holds for the chat and for every connected app.", palette);
        ui.add_space(18.0);
        card(ui, palette, |rows| {
            rows.row("Let AI work on my documents", Some(TOOLS_HINT), Lead::Strong("shield-check"), |ui| {
                switch(ui, &mut prefs.agents.tools, palette);
            });
        });
        heading(ui, "AI may also", palette);
        card(ui, palette, |rows| {
            let web = "Search the web, read pages and fetch pictures. Queries go to DuckDuckGo and your photo sources.";
            rows.row("Use the web", Some(web), Lead::Icon("globe"), |ui| switch(ui, &mut prefs.agents.web, palette));
            rows.row("Generate and edit pictures", Some(PICTURES_HINT), Lead::Icon("image"), |ui| switch(ui, &mut prefs.images.assistant, palette));
        });
        let tools = crate::agent::tools();
        let all: Vec<(&str, &str)> =
            tools.as_array().into_iter().flatten().filter_map(|tool| Some((tool["name"].as_str()?, tool["description"].as_str()?))).collect();
        let disabled = &mut prefs.agents.disabled_tools;
        let off = all.iter().filter(|(name, _)| disabled.iter().any(|known| known == name)).count();
        heading_with(ui, "Tools", palette, |ui| {
            if off > 0 && button(ui, "Turn all on", None, Kind::Quiet, true, palette).clicked() {
                disabled.clear();
            }
            ui.label(RichText::new(format!("{} of {} on", all.len() - off, all.len())).size(12.0).color(palette.muted));
        });
        hint(ui, "A tool that is off isn't offered to any assistant or app, and a call to it is refused.", palette);
        let shown = if self.all_tools { all.len() } else { all.len().min(SETTINGS_TOOLS_SHOWN) };
        card(ui, palette, |rows| {
            rows.mono = true;
            for (name, description) in &all[..shown] {
                let before = !disabled.iter().any(|known| known == name);
                let mut on = before;
                rows.row(name, Some(&tool_words(description)), Lead::Nothing, |ui| switch(ui, &mut on, palette));
                if on != before {
                    disabled.retain(|known| known != name);
                    if !on {
                        disabled.push((*name).to_owned());
                    }
                }
            }
            if shown < all.len() && rows.more(&format!("Show all {} tools", all.len())).clicked() {
                self.all_tools = true;
            }
        });
    }
}

/// What a form's bar says is still missing, left of its buttons.
fn missing(ui: &mut Ui, what: Option<&str>, palette: Palette) {
    if let Some(what) = what {
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.add(egui::Label::new(RichText::new(what).size(12.0).color(palette.muted)).truncate());
        });
    }
}

/// The General page: the window's look, and updates.
fn general(ui: &mut Ui, prefs: &mut Prefs, facts: &Facts<'_>, palette: Palette, check_now: &mut bool) {
    title(ui, "General", palette);
    heading(ui, "Appearance", palette);
    card(ui, palette, |rows| {
        rows.row("Theme", None, Lead::Nothing, |ui| theme_choice(ui, &mut prefs.appearance, palette));
        rows.row("Animations and effects", Some("Turn off to save GPU and CPU."), Lead::Nothing, |ui| {
            switch(ui, &mut prefs.appearance.animations, palette);
        });
    });
    heading(ui, "Updates", palette);
    card(ui, palette, |rows| {
        let icon = if facts.update.offered.is_some() { "circle-arrow-up" } else { "circle-check" };
        rows.row(&format!("Quill {}", crate::update::VERSION), Some(&update_line(&facts.update)), Lead::Icon(icon), |ui| {
            if button(ui, "Check now", None, Kind::Outline, !facts.update.looking, palette).clicked() {
                *check_now = true;
            }
        });
        let automatic = "Asks GitHub when Quill starts and while it stays open. A version you answered with Later is offered again at the next look.";
        rows.row("Look for new versions automatically", Some(automatic), Lead::Nothing, |ui| switch(ui, &mut prefs.updates.check, palette));
        let choices: Vec<&str> = crate::update::EVERY.iter().map(|(name, _)| *name).collect();
        // Human: a choice saved by an earlier version that is no longer offered counts as the default
        // (`update::every`); the control then shows and saves that.
        if !choices.contains(&prefs.updates.every.as_str()) {
            choices[0].clone_into(&mut prefs.updates.every);
        }
        rows.row("Check every", None, Lead::Nothing, |ui| choice(ui, "update-every", &mut prefs.updates.every, &choices, palette));
    });
}

/// The System / Light / Dark control.
fn theme_choice(ui: &mut Ui, appearance: &mut Appearance, palette: Palette) {
    let before = match (appearance.follow_system, appearance.dark) {
        (true, _) => THEME_SYSTEM,
        (false, true) => THEME_DARK,
        (false, false) => THEME_LIGHT,
    };
    let mut chosen = before.to_owned();
    let options = [
        Segment { value: THEME_SYSTEM, shown: THEME_SYSTEM, icon: Some("monitor") },
        Segment { value: THEME_LIGHT, shown: THEME_LIGHT, icon: Some("sun") },
        Segment { value: THEME_DARK, shown: THEME_DARK, icon: Some("moon") },
    ];
    segmented(ui, "theme", &mut chosen, &options, palette);
    if chosen != before {
        follow_theme(appearance, &chosen);
    }
}

/// Sets the appearance to what the theme control's choice `chosen` means.
// Human: with System the app sets `dark` to what the computer shows, frame by frame; here it stays as it is.
fn follow_theme(appearance: &mut Appearance, chosen: &str) {
    appearance.follow_system = chosen == THEME_SYSTEM;
    if !appearance.follow_system {
        appearance.dark = chosen == THEME_DARK;
    }
}

/// The Canvas page: grid, snapping, zoom, nudging, the assistants' cursors, and importing.
fn canvas_page(ui: &mut Ui, prefs: &mut Prefs, palette: Palette) {
    title(ui, "Canvas", palette);
    let canvas = &mut prefs.canvas;
    heading(ui, "Grid and snapping", palette);
    card(ui, palette, |rows| {
        rows.row("Show pixel grid", None, Lead::Nothing, |ui| switch(ui, &mut canvas.show_pixel_grid, palette));
        rows.row("Snap to pixel grid", None, Lead::Nothing, |ui| switch(ui, &mut canvas.snap_to_pixels, palette));
        rows.row("Snap to objects", None, Lead::Nothing, |ui| switch(ui, &mut canvas.snap_to_objects, palette));
    });
    heading(ui, "Zoom", palette);
    card(ui, palette, |rows| {
        let wheel = crate::keys::text("Off: the wheel scrolls, and ⌘ + wheel zooms.");
        rows.row("Zoom with the scroll wheel", Some(&wheel), Lead::Nothing, |ui| switch(ui, &mut canvas.wheel_zooms, palette));
        rows.row("Invert zoom direction", None, Lead::Nothing, |ui| switch(ui, &mut canvas.invert_zoom, palette));
    });
    heading(ui, "Nudge", palette);
    hint(ui, "How far selected objects move.", palette);
    card(ui, palette, |rows| {
        rows.row("Arrow keys", None, Lead::Nothing, |ui| pixels(ui, &mut canvas.small_nudge));
        rows.row("Shift + arrow keys", None, Lead::Nothing, |ui| pixels(ui, &mut canvas.big_nudge));
    });
    let agents = &mut prefs.agents;
    heading(ui, "AI cursors", palette);
    card(ui, palette, |rows| {
        let sizes = [Segment { value: "Small (Default)", shown: "Small", icon: None }, Segment::plain("Medium"), Segment::plain("Large")];
        rows.row("Cursor size", None, Lead::Nothing, |ui| segmented(ui, "cursor-size", &mut agents.cursor_size, &sizes, palette));
        let labels = [Segment { value: "Agent (Default)", shown: "Agent", icon: None }, Segment::plain("Model name"), Segment::plain("None")];
        let says = "What an assistant's cursor says while it works on the canvas.";
        rows.row("Cursor label", Some(says), Lead::Nothing, |ui| segmented(ui, "cursor-label", &mut agents.cursor_label, &labels, palette));
        rows.row("Label text", Some("What a cursor says while its label is Agent."), Lead::Nothing, |ui| {
            sized_field(ui, &mut agents.agent_label, "Agent", 160.0, false, false, palette);
        });
    });
    heading(ui, "Import", palette);
    card(ui, palette, |rows| {
        rows.row("Include HTML metadata when importing pages", None, Lead::Nothing, |ui| switch(ui, &mut prefs.generation.html_metadata, palette));
    });
}

/// Sets the user's own name for the assistant `id`; an empty one takes it away.
fn set_name(names: &mut Vec<AgentName>, id: &str, name: String) {
    names.retain(|known| known.id != id);
    if !name.trim().is_empty() {
        names.push(AgentName { id: id.to_owned(), name });
    }
}

/// The names the user gives the assistants of the agent program or provider called `owner`, as a card under
/// its form. Nothing shows while it has no assistant in the chat.
// Human: names are kept when typed, not with the form's Save: they belong to the assistants, which are there
// whatever the form says.
fn names_card(ui: &mut Ui, owner: Option<&str>, agents: &mut Agents, facts: &Facts<'_>, palette: Palette) {
    let Some(owner) = owner else { return };
    let own: Vec<&Choice> = facts.assistants.iter().filter(|choice| owns(owner, &choice.plain)).collect();
    if own.is_empty() {
        return;
    }
    heading(ui, "Names in Quill", palette);
    let why = "Call the models what you like. A name shows in the chat's assistant menu, and beside the cursor on the canvas when the cursor label is the model's name.";
    hint(ui, why, palette);
    card(ui, palette, |rows| {
        for choice in own {
            let before = agents.names.iter().find(|name| name.id == choice.id).map(|name| name.name.clone()).unwrap_or_default();
            let mut name = before.clone();
            rows.row(&choice.plain, Some(&choice.id), Lead::Nothing, |ui| {
                sized_field(ui, &mut name, &choice.plain, SETTINGS_NAME_FIELD, false, false, palette);
            });
            if name != before {
                set_name(&mut agents.names, &choice.id, name);
            }
        }
    });
}

/// The Chat page: who answers, what every conversation is told, and how the conversation behaves.
fn chat_page(ui: &mut Ui, agents: &mut Agents, facts: &Facts<'_>, palette: Palette) {
    title(ui, "Chat", palette);
    heading(ui, "Default assistant", palette);
    card(ui, palette, |rows| {
        if facts.assistants.is_empty() {
            rows.row("Answers in the chat", Some("No assistant is set up yet. Add one on the Assistants page."), Lead::Nothing, |_| {});
            return;
        }
        let labels: Vec<&str> = facts.assistants.iter().map(|choice| choice.label.as_str()).collect();
        let before = crate::chat::chosen(facts.assistants, &agents.chat_model).map_or("No assistant", |choice| choice.label.as_str()).to_owned();
        let mut picked = before.clone();
        let who = "The menu beside the send button changes it too.";
        rows.row("Answers in the chat", Some(who), Lead::Nothing, |ui| choice(ui, "default-assistant", &mut picked, &labels, palette));
        if picked != before
            && let Some(choice) = facts.assistants.iter().find(|choice| choice.label == picked)
        {
            agents.chat_model.clone_from(&choice.id);
        }
    });

    heading(ui, "Custom instructions", palette);
    hint(ui, "What every conversation should keep to: your brand, your taste, how the assistant should answer.", palette);
    let frame = egui::Frame::new()
        .fill(palette.card)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(theme::SETTINGS_CARD_RADIUS)
        .inner_margin(egui::Margin::symmetric(12, 10));
    let example = RichText::new("For example: Use our brand colours #0f172a and #f97316. Answer in German.").color(palette.quiet);
    ui.add(egui::TextEdit::multiline(&mut agents.instructions).frame(frame).hint_text(example).desired_rows(5).desired_width(ui.available_width()));
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(RichText::new("Applies from the next message you send.").size(12.0).color(palette.muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(4.0);
            ui.label(RichText::new(counted(agents.instructions.chars().count(), "character")).size(12.0).color(palette.muted));
        });
    });

    heading(ui, "Conversation", palette);
    card(ui, palette, |rows| {
        let sizes = [Segment::plain("Small"), Segment::plain("Medium"), Segment::plain("Large")];
        rows.row("Text size", None, Lead::Nothing, |ui| segmented(ui, "chat-text", &mut agents.chat_text_size, &sizes, palette));
        rows.row("Send while the assistant is working", Some(STEER_HINT), Lead::Nothing, |ui| switch(ui, &mut agents.always_steer, palette));
        rows.row("Have finished designs reviewed", Some(REVIEW_HINT), Lead::Nothing, |ui| switch(ui, &mut agents.review, palette));
        rows.row("Look up model prices", Some(PRICES_HINT), Lead::Nothing, |ui| switch(ui, &mut agents.prices, palette));
    });
    heading(ui, "Starting a design", palette);
    card(ui, palette, |rows| {
        rows.row("Message box on the home screen", Some(DASHBOARD_HINT), Lead::Nothing, |ui| switch(ui, &mut agents.input_on_dashboard, palette));
        rows.row("Ideas under the message bar", Some(IDEAS_HINT), Lead::Nothing, |ui| switch(ui, &mut agents.ideas, palette));
    });
}

/// How an AI app is told to start Quill's tool server at `program`: the app, where the lines go, and what
/// to paste.
// Human: the same three ways AI.md describes, with this copy of the app's own path filled in.
fn client_setups(program: &str) -> [(&'static str, &'static str, String); 3] {
    let quoted = serde_json::to_string(program).unwrap_or_else(|_| format!("\"{program}\""));
    [
        ("Claude Code", "Run in a terminal", format!("claude mcp add pencraft -- {quoted} mcp")),
        ("Codex", "Add to ~/.codex/config.toml", format!("[mcp_servers.pencraft]\ncommand = {quoted}\nargs = [\"mcp\"]")),
        (
            "Cursor, Claude Desktop and others",
            "Add to the app's MCP settings file",
            format!("{{ \"mcpServers\": {{ \"pencraft\": {{ \"command\": {quoted}, \"args\": [\"mcp\"] }} }} }}"),
        ),
    ]
}

/// The start of a tool's description: its first sentence, cut short if it is long.
fn tool_words(description: &str) -> String {
    let first = description.split_once(". ").map_or(description, |(first, _)| first).trim_end_matches('.');
    if first.chars().count() > MOST_TOOL_WORDS { first.chars().take(MOST_TOOL_WORDS - 1).chain(['…']).collect() } else { first.to_owned() }
}

/// What the MCP page says connected apps may do, in a line.
fn allowed_line(prefs: &Prefs) -> String {
    let tools = crate::agent::tools();
    let all = tools.as_array().map_or(0, Vec::len);
    let on = all.saturating_sub(prefs.agents.disabled_tools.len());
    let word = |on: bool| if on { "on" } else { "off" };
    if prefs.agents.tools {
        format!("{on} of {all} tools on · web {} · pictures {}", word(prefs.agents.web), word(prefs.images.assistant))
    } else {
        "Nothing: AI may not work on your documents.".to_owned()
    }
}

/// The MCP page: the tool server other AI apps reach Quill through, and how to connect one. Returns whether
/// the row that leads to the Permissions page was clicked.
fn mcp_page(ui: &mut Ui, prefs: &Prefs, facts: &Facts<'_>, client: &mut String, palette: Palette, notice: &mut Option<String>) -> bool {
    title(ui, "MCP", palette);
    let what = "Quill is an MCP server: other AI apps on this computer can work on your open documents with the same tools the chat uses.";
    about(ui, what, palette);
    heading(ui, "Server", palette);
    let (link, said) = match facts.port {
        Some(_) if prefs.agents.tools => (Link::Running, "On this computer only."),
        Some(_) => (Link::Stopped, "Listening, but every tool call is refused while AI may not work on your documents."),
        None => (Link::Stopped, "It didn't start; restart Quill."),
    };
    card(ui, palette, |rows| {
        rows.row("Quill MCP server", Some(said), Lead::Icon("terminal"), |ui| pill(ui, link.words(), link.tone(), palette));
        let Some(port) = facts.port else { return };
        let mut copied = |rows: &mut parts::Rows<'_>, name: &str, more: Option<&str>, value: String| {
            rows.row(name, more, Lead::Nothing, |ui| {
                if icon_button(ui, "copy", Vec2::splat(26.0), false, "Copy", palette).clicked() {
                    ui.ctx().copy_text(value.clone());
                    *notice = Some("Copied.".to_owned());
                }
                ui.label(RichText::new(&value).monospace().size(12.0).color(palette.muted));
            });
        };
        copied(rows, "Address", None, format!("http://127.0.0.1:{port}/mcp"));
        copied(rows, "Secret", Some("An app that uses the address shows the secret from this file."), "~/.pencraft/agent.json".to_owned());
    });

    heading(ui, "Connect an AI app", palette);
    let program = std::env::current_exe().map_or_else(|_| "pencraft".to_owned(), |path| path.display().to_string());
    let setups = client_setups(&program);
    card(ui, palette, |rows| {
        rows.block(|ui| {
            let tabs: Vec<Segment<'_>> = setups.iter().map(|(app, ..)| Segment::plain(app)).collect();
            segmented(ui, "mcp-client", client, &tabs, palette);
            let (_, how, setup) = setups.iter().find(|(app, ..)| *app == client.as_str()).unwrap_or(&setups[0]);
            ui.add_space(12.0);
            egui::Frame::new().fill(palette.hover).corner_radius(10.0).inner_margin(egui::Margin::symmetric(12, 10)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(*how).size(12.0).color(palette.muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if button(ui, "Copy", Some("copy"), Kind::Outline, true, palette).clicked() {
                            ui.ctx().copy_text(setup.clone());
                            *notice = Some("Copied.".to_owned());
                        }
                    });
                });
                ui.add_space(6.0);
                ui.add(egui::Label::new(RichText::new(setup).monospace().size(12.0)).wrap());
            });
            ui.add_space(12.0);
            let starts = "The app starts `pencraft mcp` and talks to it. If Quill isn't running, that starts it in the background.";
            ui.add(egui::Label::new(RichText::new(starts).size(12.0).color(palette.muted)).wrap());
        });
    });
    ui.add_space(theme::SETTINGS_GROUP_GAP);
    card(ui, palette, |rows| rows.link(Lead::Icon("shield-check"), "What connected apps may do", &allowed_line(prefs), None).clicked())
}

/// A number of pixels.
fn pixels(ui: &mut Ui, value: &mut f32) {
    ui.add(egui::DragValue::new(value).range(1.0..=1000.0).speed(0.2).max_decimals(0).suffix(" px"));
}

/// The way back from a form to the page called `to`, at the form's top. Returns whether it was clicked.
// Human: a form takes the page's place, so it needs a way back that doesn't save.
fn back(ui: &mut Ui, to: &str, palette: Palette) -> bool {
    let clicked = button(ui, to, Some("chevron-left"), Kind::Quiet, true, palette).on_hover_text("Back without saving").clicked();
    ui.add_space(8.0);
    clicked
}

impl Form {
    /// What still keeps the form from being saved, as its bar says it; `None` when it can be.
    fn missing(&self) -> Option<&'static str> {
        match (self.name.trim().is_empty(), self.base_url.trim().is_empty()) {
            (true, true) => Some("Name and base URL are still missing."),
            (true, false) => Some("A name is still missing."),
            (false, true) => Some("The base URL is still missing."),
            (false, false) if self.provider().is_none() => Some("Every model needs an ID of its own and both token counts."),
            (false, false) => None,
        }
    }
}

impl CliForm {
    /// What still keeps the form from being saved, as its bar says it; `None` when it can be.
    fn missing(&self, others: &[LocalAgent]) -> Option<&'static str> {
        if self.agent(others).is_some() {
            None
        } else if self.name.trim().is_empty() {
            Some("A name is still missing.")
        } else if self.name.contains('/') {
            Some("The name can't have a / in it.")
        } else if crate::chat::command_of(&LocalAgent { kind: self.kind.clone(), program: self.program.trim().to_owned(), ..LocalAgent::default() })
            .is_none()
        {
            Some("The program is still missing.")
        } else {
            Some("Another agent has this name already.")
        }
    }
}

impl ImageForm {
    /// What still keeps the form from being saved, as its bar says it; `None` when it can be.
    fn missing(&self, images: &Images) -> Option<&'static str> {
        if self.service(images).is_some() {
            None
        } else if self.name.trim().is_empty() {
            Some("A name is still missing.")
        } else if crate::imagery::Kind::named(&self.kind).is_none() {
            Some("Pick a kind of service.")
        } else {
            Some("Another service has this name already.")
        }
    }
}

/// The custom provider form. Returns `true` when it was left by its way back; its buttons are in the bar
/// under the page ([`Settings::footer`]).
#[expect(clippy::too_many_arguments, reason = "the form, what it changes, and the question to the provider that is out")]
fn provider_form(
    ui: &mut Ui,
    form: &mut Form,
    prefs: &mut Prefs,
    facts: &Facts<'_>,
    vault: &dyn Vault,
    asking: Asking<'_>,
    palette: Palette,
    notice: &mut Option<String>,
) -> bool {
    let Asking { loading, testing, picker } = asking;
    take_models(form, loading, testing, picker, notice);
    let left = back(ui, "Assistants", palette);
    let saved = form.index.and_then(|index| prefs.agents.providers.get(index));
    let owner = saved.map(|provider| provider.name.clone());
    let link = if saved.is_some_and(crate::chat::usable) { Link::Ready } else { Link::NotConnected };
    title(ui, if saved.is_some() { "Provider" } else { "Add provider" }, palette);
    ui.add_space(18.0);
    let name = if form.name.trim().is_empty() { "Custom provider".to_owned() } else { form.name.trim().to_owned() };
    card(ui, palette, |rows| {
        rows.row(&name, Some("Any OpenAI or Anthropic compatible API"), Lead::Icon("server"), |ui| pill(ui, link.words(), link.tone(), palette));
    });

    heading(ui, "1 · Connection", palette);
    card(ui, palette, |rows| {
        rows.block(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            ui.columns(2, |columns| {
                caption(&mut columns[0], "Name", palette);
                field(&mut columns[0], &mut form.name, "My provider", false, false, palette);
                caption(&mut columns[1], "API type", palette);
                wide_choice(&mut columns[1], &mut form.api, &API_TYPES, palette);
            });
            // Human: asked the Chat Completions way, Ollama reads only the first few thousand tokens of a
            // conversation, less than the app's brief; the model then answers without knowing its job.
            if form.api == crate::chat::COMPLETIONS_API && catalog::is_local_ollama(&form.base_url) {
                warning(ui, "Choose the API type \"Ollama\" for this address: asked this way, Ollama cuts long conversations short.");
            }
            if crate::chat::Wire::of(&form.api).is_none() {
                warning(ui, "The chat can't talk to this API type yet; the provider is kept but not offered there.");
            }
            ui.add_space(14.0);
            caption(ui, "Base URL", palette);
            field(ui, &mut form.base_url, "https://api.example.com/v1", true, false, palette);
            if unprotected(&form.base_url) {
                warning(ui, "This address isn't encrypted: the key and your messages could be read on the way.");
            }
            ui.add_space(14.0);
            caption(ui, "API key", palette);
            let saved = form.key_ref.is_some() && !form.forget_key;
            field(ui, &mut form.key, if saved { "Saved - type a new key to replace it" } else { "Optional" }, true, true, palette);
            let help = "Leave blank if the endpoint needs no key. Kept in your system's keychain, never in the settings file.";
            key_help(ui, help, saved && form.key.is_empty(), &mut form.forget_key, palette);
            if !saved && form.key.trim().is_empty() && catalog::wants_key(&form.base_url) {
                warning(ui, "This provider answers only with an API key; without one the chat is refused.");
            }
        });
    });

    heading(ui, "2 · Models", palette);
    hint(
        ui,
        "Each model appears under this provider in the chat's model picker. Choose them from the provider's own list, or add one by its ID.",
        palette,
    );
    card(ui, palette, |rows| {
        rows.block(|ui| {
            model_rows(ui, form, palette);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                // Human: model names change often and differ by provider, so the form asks the provider instead
                // of guessing them, and shows its answer in a window of its own to search and tick in.
                if button(ui, "Choose models…", Some("list-checks"), Kind::Primary, !form.base_url.trim().is_empty(), palette).clicked() {
                    *picker = Some(open_picker(form, vault, loading, testing, ui.ctx()));
                }
                if button(ui, "Add by ID", Some("plus"), Kind::Outline, true, palette).clicked() {
                    form.models.push(ModelRow::empty());
                }
            });
        });
    });
    names_card(ui, owner.as_deref(), &mut prefs.agents, facts, palette);
    left
}

/// The image service form. Returns `true` when it was left by its way back; its buttons are in the bar under
/// the page ([`Settings::footer`]).
fn image_form(ui: &mut Ui, form: &mut ImageForm, prefs: &Prefs, palette: Palette) -> bool {
    let left = back(ui, "Images", palette);
    title(ui, if form.index.is_some() { "Image service" } else { "Add image service" }, palette);
    let kind = crate::imagery::Kind::named(&form.kind);
    heading(ui, "Service", palette);
    card(ui, palette, |rows| {
        rows.block(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            ui.columns(2, |columns| {
                caption(&mut columns[0], "Name", palette);
                field(&mut columns[0], &mut form.name, "My image service", false, false, palette);
                caption(&mut columns[1], "Kind", palette);
                let kinds: Vec<&str> = crate::imagery::Kind::ALL.iter().map(|kind| kind.name()).collect();
                wide_choice(&mut columns[1], &mut form.kind, &kinds, palette);
            });
            // Human: a kind from a newer Quill, or typed into the settings file; the service can't be saved until
            // one Quill knows is picked (`ImageForm::service`).
            if kind.is_none() {
                warning(ui, "Quill doesn't know this kind of service; pick one.");
            }
            ui.add_space(14.0);
            caption(ui, "Model", palette);
            // Human: the hints show the kind's usual model and address, which an empty field stands for.
            field(ui, &mut form.model, kind.map_or("", crate::imagery::Kind::model), true, false, palette);
            ui.add_space(14.0);
            caption(ui, "Address", palette);
            field(ui, &mut form.base_url, kind.map_or("", crate::imagery::Kind::base), true, false, palette);
            if unprotected(&form.base_url) {
                warning(ui, "This address isn't encrypted: the key and your descriptions could be read on the way.");
            }
            note(
                ui,
                "Leave model and address empty for the usual ones. \"OpenAI Images\" also fits other services that speak the same API.",
                palette,
            );
        });
    });
    heading(ui, "API key", palette);
    card(ui, palette, |rows| {
        rows.block(|ui| {
            let saved = form.key_ref.is_some() && !form.forget_key;
            field(ui, &mut form.key, if saved { "Saved - type a new key to replace it" } else { "Needed to make pictures" }, true, true, palette);
            key_help(ui, "The key is kept in your system's keychain.", saved && form.key.is_empty(), &mut form.forget_key, palette);
            if form.key_moves(&prefs.images) {
                warning(ui, "The saved key will be sent to the new address.");
            }
        });
    });
    left
}

/// The line that adds Quill's tool server to an agent program's MCP servers, for the user to read.
fn hook_line(command: &str, pencraft: &str) -> String {
    format!("{command} mcp add pencraft -- {} mcp", serde_json::to_string(pencraft).unwrap_or_else(|_| pencraft.to_owned()))
}

/// Adds Quill's tool server to the MCP servers of the agent program at `program` (`<program> mcp add`).
// Agent: SPAWNS one short-lived thread, which runs the program and WAKES the UI with what to tell the user;
// WRITES the program's own settings file, on the user's click only.
fn hook(program: PathBuf, ctx: &egui::Context) -> Option<mpsc::Receiver<String>> {
    let pencraft = std::env::current_exe().ok()?;
    let (send, receive) = mpsc::channel();
    let wake = ctx.clone();
    let started = std::thread::Builder::new().name("hook-agent".to_owned()).spawn(move || {
        let ran = std::process::Command::new(&program).args(["mcp", "add", "pencraft", "--"]).arg(&pencraft).arg("mcp").output();
        let said = match ran {
            Ok(output) if output.status.success() => "Quill's tools were added to the program's MCP servers.".to_owned(),
            Ok(output) => {
                format!("The program didn't take it: {}", String::from_utf8_lossy(&output.stderr).trim().chars().take(300).collect::<String>())
            }
            Err(error) => format!("The program couldn't be run: {error}"),
        };
        // The form may have closed meanwhile; then nobody listens, which is fine.
        let _unheard = send.send(said);
        wake.request_repaint();
    });
    started.ok().map(|_| receive)
}

/// What the command-line agent form says about how the program gets Quill's tools, and for Grok the button
/// that adds them.
fn cli_tools(
    ui: &mut Ui,
    form: &CliForm,
    command: Option<&str>,
    program: Option<&PathBuf>,
    hooking: &mut Option<mpsc::Receiver<String>>,
    palette: Palette,
    notice: &mut Option<String>,
) {
    let tools = match form.kind.as_str() {
        KIND_CLAUDE => "Claude Code is handed Quill's tools each time it answers. There is nothing to set up.",
        KIND_GROK => {
            "Grok works on your documents through its own MCP servers: Quill has to be added there once, under the name \"pencraft\". Only that server's tools are approved for the chat."
        }
        _ => {
            "The program works on your documents through its own MCP servers: add `pencraft mcp` there once (the MCP page has the lines to copy), and let it use that server's tools without asking."
        }
    };
    ui.add(egui::Label::new(RichText::new(tools).size(12.0).color(palette.muted)).wrap());
    if form.kind != KIND_GROK {
        return;
    }
    ui.add_space(10.0);
    let pencraft = std::env::current_exe().map_or_else(|_| "pencraft".to_owned(), |path| path.display().to_string());
    egui::Frame::new().fill(palette.hover).corner_radius(10.0).inner_margin(egui::Margin::symmetric(12, 10)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.add(egui::Label::new(RichText::new(hook_line(command.unwrap_or("grok"), &pencraft)).monospace().size(12.0)).wrap());
    });
    ui.add_space(10.0);
    let add = if hooking.is_some() { "Adding…" } else { "Add Quill to Grok" };
    if button(ui, add, None, Kind::Outline, hooking.is_none() && program.is_some(), palette).on_hover_text("Runs the line above").clicked()
        && let Some(program) = program
    {
        *hooking = hook(program.clone(), ui.ctx());
        if hooking.is_none() {
            *notice = Some("The program couldn't be run.".to_owned());
        }
    }
}

/// The command-line agent form. Returns `true` when it was left by its way back; its buttons are in the bar
/// under the page ([`Settings::footer`]).
// Agent: READS the file system once a frame to say whether the program is there (one look-up, form only).
fn cli_form(
    ui: &mut Ui,
    form: &mut CliForm,
    agents: &mut Agents,
    facts: &Facts<'_>,
    hooking: &mut Option<mpsc::Receiver<String>>,
    palette: Palette,
    notice: &mut Option<String>,
) -> bool {
    if let Some(said) = hooking.as_ref().and_then(|waiting| waiting.try_recv().ok()) {
        *notice = Some(said);
        *hooking = None;
    }
    let left = back(ui, "Assistants", palette);
    let draft = LocalAgent { kind: form.kind.clone(), program: form.program.trim().to_owned(), ..LocalAgent::default() };
    let command = crate::chat::command_of(&draft).map(str::to_owned);
    let program = command.as_deref().and_then(locate);
    let owner = form.index.and_then(|index| agents.local.get(index)).map(|agent| agent.name.clone());
    title(ui, if owner.is_some() { "Agent program" } else { "Add command-line agent" }, palette);
    ui.add_space(18.0);
    let link = match (form.on, program.is_some()) {
        (false, _) => Link::Off,
        (true, true) => Link::Installed,
        (true, false) => Link::Missing,
    };
    let name = if form.name.trim().is_empty() { "Command-line agent".to_owned() } else { form.name.trim().to_owned() };
    card(ui, palette, |rows| {
        let purpose = "An agent program on this computer, with its own sign-in";
        rows.row(&name, Some(purpose), Lead::Icon("terminal"), |ui| pill(ui, link.words(), link.tone(), palette));
    });

    heading(ui, "Program", palette);
    card(ui, palette, |rows| {
        rows.block(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            ui.columns(2, |columns| {
                caption(&mut columns[0], "Name", palette);
                field(&mut columns[0], &mut form.name, "My agent", false, false, palette);
                caption(&mut columns[1], "Kind", palette);
                wide_choice(&mut columns[1], &mut form.kind, &KINDS, palette);
            });
            ui.add_space(14.0);
            caption(ui, "Program", palette);
            let usual = crate::chat::usual_command(&form.kind);
            let example = usual.map_or_else(|| "/full/path/to/program, or its command".to_owned(), |usual| format!("{usual} (found automatically)"));
            field(ui, &mut form.program, &example, true, false, palette);
            match (&program, &command) {
                (Some(path), _) => note(ui, &format!("Found at {}", path.display()), palette),
                (None, Some(command)) => warning(ui, &format!("`{command}` wasn't found on this computer. Install it, or give its full path.")),
                (None, None) => note(ui, "Give the program's full path, or the command it is run with.", palette),
            }
            ui.add_space(14.0);
            caption(ui, "Models", palette);
            field(ui, &mut form.models, "Leave empty for the program's own default; several are separated by commas", true, false, palette);
            if form.kind == KIND_PLAIN {
                ui.add_space(14.0);
                caption(ui, "Arguments", palette);
                field(ui, &mut form.arguments, "exec {prompt}", true, false, palette);
                let how = "{prompt} is replaced by the message and {model} by the model. Without {prompt} the message goes to the program's input. The program's printed answer is shown in the chat.";
                note(ui, how, palette);
            }
        });
    });
    heading(ui, "Quill's tools", palette);
    card(ui, palette, |rows| rows.block(|ui| cli_tools(ui, form, command.as_deref(), program.as_ref(), hooking, palette, notice)));
    heading(ui, "Chat", palette);
    card(ui, palette, |rows| {
        let offer = "Off keeps the settings here but takes it out of the assistant menu.";
        rows.row("Offer in the chat", Some(offer), Lead::Nothing, |ui| switch(ui, &mut form.on, palette));
    });
    names_card(ui, owner.as_deref(), agents, facts, palette);
    left
}

/// What a form says under its key field, and its "Forget saved key" button when `forgettable` (a key is saved
/// and none is typed); the button sets `forget`.
fn key_help(ui: &mut Ui, help: &str, forgettable: bool, forget: &mut bool, palette: Palette) {
    note(ui, help, palette);
    if forgettable {
        ui.add_space(6.0);
        if button(ui, "Forget saved key", None, Kind::Outline, true, palette).clicked() {
            *forget = true;
        }
    }
}

/// The keychain name a form's key has once it is saved, `key_ref` being the one it had: a newly `typed` key
/// is put under it (or under a new name); `forget` removes the saved key; otherwise the saved key stays.
///
/// # Errors
/// A sentence for the user when the keychain refused.
fn kept_key(typed: &str, forget: bool, key_ref: Option<String>, vault: &dyn Vault) -> Result<Option<String>, String> {
    let typed = typed.trim();
    if !typed.is_empty() {
        let name = key_ref.unwrap_or_else(secrets::new_name);
        vault.put(&name, &SecretString::new(typed)).map_err(|e| format!("The key couldn't be saved in the keychain: {e}"))?;
        Ok(Some(name))
    } else if forget && let Some(name) = key_ref {
        vault.forget(&name).map_err(|e| format!("The key couldn't be removed from the keychain: {e}"))?;
        Ok(None)
    } else {
        Ok(key_ref)
    }
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
    provider.key_ref = kept_key(&form.key, form.forget_key, provider.key_ref.take(), vault)?;
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

/// Saves the form's `service` in the settings and its key in the keychain.
///
/// A newly typed key replaces the saved one; "Forget saved key" removes it; otherwise the saved key stays.
///
/// # Errors
/// A sentence for the user when the keychain refused; the settings are then left as they were.
// Agent: WRITES the keychain first, the settings only after that worked, so the settings never name a key
// that isn't there.
fn save_image_service(form: &mut ImageForm, mut service: ImageService, prefs: &mut Prefs, vault: &dyn Vault) -> Result<(), String> {
    service.key_ref = kept_key(&form.key, form.forget_key, service.key_ref.take(), vault)?;
    form.key.clear();
    match form.index.and_then(|index| prefs.images.services.get_mut(index)) {
        Some(saved) => {
            // Human: the default follows a service that was renamed.
            if prefs.images.default == saved.name {
                prefs.images.default.clone_from(&service.name);
            }
            *saved = service;
        }
        None => prefs.images.services.push(service),
    }
    Ok(())
}

/// Removes the saved image service at `index` and its key.
///
/// # Errors
/// A sentence for the user when the keychain refused; the service then stays.
fn remove_image_service(index: usize, prefs: &mut Prefs, vault: &dyn Vault) -> Result<(), String> {
    let Some(service) = prefs.images.services.get(index) else { return Ok(()) };
    if let Some(name) = &service.key_ref {
        vault.forget(name).map_err(|e| format!("The key couldn't be removed from the keychain: {e}"))?;
    }
    if prefs.images.default == service.name {
        prefs.images.default.clear();
    }
    prefs.images.services.remove(index);
    Ok(())
}

/// What a saved image service's card says under its name: its kind, its model, and when it has no key yet.
// Human: parts that are empty (the model of a kind Quill doesn't know) are left out with their dot.
fn service_about(service: &ImageService) -> String {
    let kind = crate::imagery::Kind::named(&service.kind);
    let model = if service.model.trim().is_empty() { kind.map_or("", crate::imagery::Kind::model) } else { service.model.trim() };
    let key = if service.key_ref.is_some() { "" } else { "no key yet" };
    [service.kind.trim(), model, key].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(" · ")
}

/// The row the Images page shows for a service whose key comes from the environment, if `ready` is one: its
/// name, and where its key is from.
fn environment_line(ready: &crate::imagery::Ready) -> Option<(&str, String)> {
    match ready.key {
        crate::imagery::KeyFrom::Environment(variable) => Some((ready.name.as_str(), format!("Key from the {variable} environment variable"))),
        crate::imagery::KeyFrom::Keychain(_) => None,
    }
}

/// What the default service choice shows: the saved default when it is among the `usable` services (by name,
/// whatever the case), else "First that works".
fn shown_default(usable: &[String], saved: &str) -> String {
    usable.iter().find(|name| !saved.trim().is_empty() && name.eq_ignore_ascii_case(saved.trim())).cloned().unwrap_or_else(|| ANY_SERVICE.to_owned())
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

/// The model rows of the custom provider form.
fn model_rows(ui: &mut Ui, form: &mut Form, palette: Palette) {
    // Human: five fields side by side and a bin; only the first row carries the fields' names. The prices are
    // dollars for a million tokens and may stay empty: the price list is asked then.
    let gap = 8.0;
    let bin = 30.0;
    let fields = ui.available_width() - bin - 5.0 * gap;
    let widths = [fields * 0.34, fields * 0.18, fields * 0.18, fields * 0.15, fields * 0.15];
    let mut remove = None;
    let rows = form.models.len();
    for (index, model) in form.models.iter_mut().enumerate() {
        let names = if index == 0 { 24.0 } else { 0.0 };
        let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), names + FIELD_HEIGHT + 4.0), Sense::hover());
        let mut left = line.left();
        for (text, name, example, width) in [
            (&mut model.id, "Model ID", "my-model-id", widths[0]),
            (&mut model.context_window, "Context window", DEFAULT_TOKENS, widths[1]),
            (&mut model.max_tokens, "Max tokens", DEFAULT_TOKENS, widths[2]),
            (&mut model.input_price, "Input $/M", "from list", widths[3]),
            (&mut model.output_price, "Output $/M", "from list", widths[4]),
        ] {
            let cell = Rect::from_min_size(pos2(left, line.top()), vec2(width, line.height()));
            left = cell.right() + gap;
            // Human: a child placed by hand, so the page's own flow continues under the whole row.
            let mut cell = ui.new_child(UiBuilder::new().max_rect(cell).id_salt((index, name)));
            cell.spacing_mut().item_spacing.y = 0.0;
            if index == 0 {
                caption(&mut cell, name, palette);
            }
            field(&mut cell, text, example, true, false, palette);
        }
        let cell = Rect::from_min_size(pos2(left, line.bottom() - FIELD_HEIGHT - 4.0), Vec2::splat(bin));
        let mut cell = ui.new_child(UiBuilder::new().max_rect(cell).id_salt((index, "bin")));
        if rows > 1 && icon_button(&mut cell, "trash-2", Vec2::splat(bin), false, "Remove model", palette).clicked() {
            remove = Some(index);
        }
        ui.add_space(4.0);
    }
    if let Some(index) = remove {
        form.models.remove(index);
    }
}

/// Asks the form's provider which models it has; the answer comes through what is returned.
// Agent: SPAWNS one short-lived thread per question; it WAKES the UI when the answer is there. READS the saved
// key from the keychain when no new one is typed.
fn ask_models(form: &Form, vault: &dyn Vault, ctx: &egui::Context) -> Result<mpsc::Receiver<Models>, String> {
    let key = key_of(form, vault)?;
    let (send, receive) = mpsc::channel();
    let (api, base, wake) = (form.api.clone(), form.base_url.clone(), ctx.clone());
    let started = std::thread::Builder::new().name("load-models".to_owned()).spawn(move || {
        // The form may have closed meanwhile; then nobody listens, which is fine.
        let _unheard = send.send(catalog::list_models(&api, &base, key.as_ref()));
        wake.request_repaint();
    });
    started.map(|_| receive).map_err(|e| format!("The provider couldn't be asked: {e}"))
}

/// The key the form's provider is asked with: the one just typed, or the saved one; none when it has none.
// Agent: READS the saved key from the keychain when no new one is typed.
fn key_of(form: &Form, vault: &dyn Vault) -> Result<Option<SecretString>, String> {
    let typed = form.key.trim();
    match (&form.key_ref, typed.is_empty(), form.forget_key) {
        (_, false, _) => Ok(Some(SecretString::new(typed))),
        (Some(name), true, false) => vault.get(name).map(Some).map_err(|e| format!("The saved key couldn't be read from the keychain: {e}")),
        _ => Ok(None),
    }
}

/// Asks the form's provider, when it is an Ollama, what each chosen model can take that it hasn't said yet;
/// the answers come through what is returned. `None` when there is nothing to ask or it can't be asked: the
/// rows then keep their numbers.
// Human: Ollama's list of models has names only; how much a model reads is said about one model at a time. So
// only the models that were chosen are asked about, not the whole list.
// Agent: SPAWNS one short-lived thread, which asks about the models one after the other and WAKES the UI.
fn ask_limits(form: &Form, vault: &dyn Vault, ctx: &egui::Context) -> Option<mpsc::Receiver<Told>> {
    let ids = form.unasked();
    if ids.is_empty() {
        return None;
    }
    let key = key_of(form, vault).ok()?;
    let (send, receive) = mpsc::channel();
    let (base, wake) = (form.base_url.clone(), ctx.clone());
    let started = std::thread::Builder::new().name("model-limits".to_owned()).spawn(move || {
        let told: Told = ids.into_iter().map(|id| (catalog::limits(&base, key.as_ref(), &id), id)).map(|(limits, id)| (id, limits)).collect();
        // The form may have closed meanwhile; then nobody listens, which is fine.
        let _unheard = send.send(told);
        wake.request_repaint();
    });
    started.ok().map(|_| receive)
}

/// Takes in the provider's answer to [`ask_models`], when it is there. Asked as a test of the connection
/// (`testing`), the answer is also said.
// Human: with the window for choosing models open and waiting, the answer goes there, a refusal too; a refusal
// is said as a notice only when nothing on the screen waits for it.
fn take_models(
    form: &mut Form,
    loading: &mut Option<mpsc::Receiver<Models>>,
    testing: &mut bool,
    picker: &mut Option<models::Picker>,
    notice: &mut Option<String>,
) {
    let Some(waiting) = loading else { return };
    let answer = match waiting.try_recv() {
        Ok(answer) => answer,
        Err(mpsc::TryRecvError::Empty) => return,
        Err(mpsc::TryRecvError::Disconnected) => Err("The provider didn't answer.".to_owned()),
    };
    match (answer, picker.as_mut().filter(|picker| picker.is_loading())) {
        (Ok(found), waiting) => {
            if *testing {
                *notice = Some(format!("Connected: the provider answered with {}.", counted(found.len(), "model")));
            }
            form.offered = found.iter().map(|model| model.id.clone()).collect();
            form.learn(found.into_iter().map(|model| (model.id, model.limits)));
            if let Some(picker) = waiting {
                picker.listed(form.offered.clone());
            }
        }
        (Err(why), Some(picker)) => picker.failed(why),
        (Err(why), None) => *notice = Some(why),
    }
    *loading = None;
    *testing = false;
}

/// The question to a provider which models it has, and the window its answer is shown in.
struct Asking<'a> {
    /// The question, while it is out.
    loading: &'a mut Option<mpsc::Receiver<Models>>,
    /// It was asked to test the connection.
    testing: &'a mut bool,
    /// The window the models are chosen in, when it is open.
    picker: &'a mut Option<models::Picker>,
}

/// Opens the window the form's models are chosen in. The provider is asked for its models unless the form has
/// them from before, or a question is out already.
fn open_picker(
    form: &Form,
    vault: &dyn Vault,
    loading: &mut Option<mpsc::Receiver<Models>>,
    testing: &mut bool,
    ctx: &egui::Context,
) -> models::Picker {
    let named = form.models.iter().map(|row| row.id.trim().to_owned()).filter(|id| !id.is_empty());
    let mut picker = models::Picker::new(&form.name, &form.offered, named);
    if !form.offered.is_empty() {
        return picker;
    }
    if loading.is_some() {
        picker.loading();
        return picker;
    }
    match ask_models(form, vault, ctx) {
        Ok(answer) => {
            *loading = Some(answer);
            *testing = false;
            picker.loading();
        }
        Err(why) => picker.failed(why),
    }
    picker
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
        folders.extend([".local/bin", ".claude/local", ".grok/bin", ".npm-global/bin", ".bun/bin", ".cargo/bin"].map(|folder| home.join(folder)));
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

/// Where the program run by `command` is: `command` itself when it is a path to a file (a leading `~/` is the
/// user's home folder), or the installed program of that name.
#[must_use]
pub fn locate(command: &str) -> Option<PathBuf> {
    let command = command.trim();
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from);
    let path = match (command.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => home.join(rest),
        _ => PathBuf::from(command),
    };
    if path.components().count() > 1 { path.is_file().then_some(path) } else { program(command) }
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
    fn an_image_service_form_needs_a_name_no_other_service_has() {
        let mut prefs = Prefs::default();
        let mut form = ImageForm::new();
        assert_eq!(form.service(&prefs.images), None);
        form.name = " Studio ".to_owned();
        let service = form.service(&prefs.images).unwrap();
        assert_eq!(
            (service.name.as_str(), service.kind.as_str(), service.base_url.as_str(), service.model.as_str()),
            ("Studio", "Google Gemini", "", "")
        );
        prefs.images.services.push(service);
        assert_eq!(form.service(&prefs.images), None, "a second service of the same name");
        let again = ImageForm::of(0, &prefs.images.services[0]);
        assert!(again.service(&prefs.images).is_some(), "the service being changed may keep its name");
    }

    #[test]
    fn an_image_services_key_goes_to_the_vault_and_leaves_with_the_service() {
        let vault = Memory::default();
        let mut prefs = Prefs::default();
        let mut form = ImageForm::new();
        form.name = "Studio".to_owned();
        form.key = "secret".to_owned();
        let service = form.service(&prefs.images).unwrap();
        save_image_service(&mut form, service, &mut prefs, &vault).unwrap();
        let name = prefs.images.services[0].key_ref.clone().unwrap();
        assert_eq!(vault.kept.borrow().get(&name).map(String::as_str), Some("secret"));
        assert!(!serde_json::to_string(&prefs).unwrap().contains("secret"));
        assert_eq!(form.key, "");
        prefs.images.default = "Studio".to_owned();
        remove_image_service(0, &mut prefs, &vault).unwrap();
        assert!(prefs.images.services.is_empty() && prefs.images.default.is_empty());
        assert!(!vault.kept.borrow().contains_key(&name));
    }

    /// A saved image service called `name` of the kind `kind` whose key is kept as `key_ref`.
    fn service(name: &str, kind: &str, key_ref: Option<&str>) -> ImageService {
        ImageService {
            name: name.to_owned(),
            kind: kind.to_owned(),
            base_url: String::new(),
            model: String::new(),
            key_ref: key_ref.map(str::to_owned),
        }
    }

    #[test]
    fn an_image_services_saved_key_can_be_forgotten_and_the_service_stays() {
        let vault = Memory::default();
        let mut prefs = Prefs::default();
        let mut form = ImageForm::new();
        form.name = "Studio".to_owned();
        form.key = "secret".to_owned();
        let service = form.service(&prefs.images).unwrap();
        save_image_service(&mut form, service, &mut prefs, &vault).unwrap();
        let mut form = ImageForm::of(0, &prefs.images.services[0]);
        form.forget_key = true;
        let service = form.service(&prefs.images).unwrap();
        save_image_service(&mut form, service, &mut prefs, &vault).unwrap();
        assert!(vault.kept.borrow().is_empty());
        assert_eq!(prefs.images.services.len(), 1);
        assert_eq!((prefs.images.services[0].name.as_str(), prefs.images.services[0].key_ref.as_ref()), ("Studio", None));
    }

    #[test]
    fn a_saved_key_is_said_to_go_elsewhere_only_when_kind_or_address_changed_without_a_new_key() {
        let mut images = Images::default();
        images.services.push(service("Studio", "Google Gemini", Some("k")));
        let form = ImageForm::of(0, &images.services[0]);
        assert!(!form.key_moves(&images));
        let moved = ImageForm { base_url: "https://elsewhere.example/v1".to_owned(), ..form.clone() };
        assert!(moved.key_moves(&images));
        assert!(ImageForm { kind: "OpenAI Images".to_owned(), ..form.clone() }.key_moves(&images));
        assert!(!ImageForm { base_url: "  ".to_owned(), ..form.clone() }.key_moves(&images), "spaces are no address");
        assert!(!ImageForm { key: "new".to_owned(), ..moved.clone() }.key_moves(&images), "a new key goes there instead");
        assert!(!ImageForm { forget_key: true, ..moved.clone() }.key_moves(&images), "no key goes there");
        // A new service, or a saved one without a key, has no saved key to send.
        assert!(!ImageForm { index: None, ..moved.clone() }.key_moves(&images));
        images.services[0].key_ref = None;
        assert!(!moved.key_moves(&images));
    }

    #[test]
    fn a_service_of_a_kind_quill_doesnt_know_can_be_saved_once_a_known_kind_is_picked() {
        let mut images = Images::default();
        images.services.push(service("Odd", "Something else", Some("k")));
        let mut form = ImageForm::of(0, &images.services[0]);
        assert_eq!(form.service(&images), None);
        "OpenAI Images".clone_into(&mut form.kind);
        assert!(form.service(&images).is_some());
        // Its card has no dangling dot.
        assert_eq!(service_about(&images.services[0]), "Something else");
        assert_eq!(service_about(&service("Odd", "Something else", None)), "Something else · no key yet");
        assert_eq!(
            service_about(&service("Studio", "OpenAI Images", None)),
            format!("OpenAI Images · {} · no key yet", crate::imagery::openai::MODEL)
        );
    }

    #[test]
    fn the_default_choice_shows_a_saved_default_only_while_it_can_be_used() {
        let usable = ["Studio".to_owned(), "Gemini".to_owned()];
        assert_eq!(shown_default(&usable, "Gemini"), "Gemini");
        assert_eq!(shown_default(&usable, "studio"), "Studio");
        assert_eq!(shown_default(&usable, "Gone"), ANY_SERVICE);
        assert_eq!(shown_default(&usable, ""), ANY_SERVICE);
        assert_eq!(shown_default(&[], "Gemini"), ANY_SERVICE);
    }

    #[test]
    fn a_service_from_the_environment_gets_a_line_of_its_own() {
        let mut images = Images::default();
        images.services.push(service("Studio", "OpenAI Images", Some("k")));
        let ready = crate::imagery::ready(&images, &|name| (name == "GEMINI_API_KEY").then(|| "g".to_owned()));
        let lines: Vec<(&str, String)> = ready.iter().filter_map(environment_line).collect();
        assert_eq!(lines, [("Gemini", "Key from the GEMINI_API_KEY environment variable".to_owned())]);
    }

    #[test]
    fn a_typed_key_never_shows_in_the_forms_debug_text() {
        let mut form = ImageForm::new();
        assert!(format!("{form:?}").contains("key: \"\""));
        form.key = "sk-very-secret".to_owned();
        let shown = format!("{form:?}");
        assert!(!shown.contains("sk-very-secret") && shown.contains("key: \"…\""), "{shown}");
    }

    #[test]
    fn closing_the_window_drops_an_open_image_form_and_its_key() {
        let mut settings = Settings { vault: Box::new(Memory::default()), ..Settings::default() };
        settings.open_named("image-service");
        if let Some(form) = &mut settings.image_form {
            form.key = "secret".to_owned();
        }
        settings.open = false;
        let ctx = egui::Context::default();
        assert_eq!(
            settings.show(&ctx, &mut Prefs::default(), &Facts { assistants: &[], port: None, update: crate::update::Seen::default() }, false),
            None
        );
        assert!(settings.image_form.is_none());
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
        let model = ProviderModel { id: "model-a".to_owned(), context_window: 512_000, max_tokens: 512_000, ..ProviderModel::default() };
        assert_eq!(provider.models, vec![model]);
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
        // Prices are optional, but one alone, a word or a decimal comma is no price.
        assert!(change(|f| f.models[0].input_price = "3".to_owned()).is_none());
        assert!(change(|f| (f.models[0].input_price, f.models[0].output_price) = ("3".to_owned(), "lots".to_owned())).is_none());
        assert!(change(|f| (f.models[0].input_price, f.models[0].output_price) = ("0,075".to_owned(), "15".to_owned())).is_none());
        assert!(change(|f| (f.models[0].input_price, f.models[0].output_price) = ("3".to_owned(), "1,5".to_owned())).is_none());
        let priced = change(|f| (f.models[0].input_price, f.models[0].output_price) = (" 3 ".to_owned(), "15.5".to_owned())).unwrap();
        assert_eq!((priced.models[0].input_price.as_str(), priced.models[0].output_price.as_str()), ("3", "15.5"));
    }

    #[test]
    fn typed_prices_survive_opening_a_provider_and_saving_it_again() {
        let mut form = filled();
        (form.models[0].input_price, form.models[0].output_price) = ("3".to_owned(), "15.5".to_owned());
        let (mut prefs, vault) = (Prefs::default(), Memory::default());
        save(&mut form, &mut prefs, &vault).unwrap();
        let saved = prefs.agents.providers[0].clone();
        assert_eq!((saved.models[0].input_price.as_str(), saved.models[0].output_price.as_str()), ("3", "15.5"));
        save(&mut Form::of(0, &saved), &mut prefs, &vault).unwrap();
        assert_eq!(prefs.agents.providers[0].models, saved.models);
    }

    #[test]
    fn a_program_is_found_by_its_path_or_its_command() {
        let folder = std::env::temp_dir().join(format!("pencraft-installed-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let file = folder.join(&file_names("claude")[0]);
        std::fs::write(&file, "").unwrap();
        let folders = [PathBuf::from("/nowhere/at/all"), folder.clone()];
        assert_eq!(find_in("claude", &folders), Some(file.clone()));
        assert!(find_in("codex", &folders).is_none() && find_in("claude", &folders[..1]).is_none());
        // A path names the file itself; a bare command is looked for where programs are installed.
        assert_eq!(locate(&format!(" {} ", file.display())), Some(file));
        assert_eq!(locate(&folder.join("codex").display().to_string()), None);
        assert_eq!(locate("no-such-program-anywhere"), None);
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn a_command_line_agent_needs_a_name_of_its_own_and_a_program() {
        let agents = Agents::default();
        // The two known programs are there from the start, found by their usual commands.
        let known: Vec<(&str, Option<&str>)> = agents.local.iter().map(|a| (a.name.as_str(), crate::chat::command_of(a))).collect();
        assert_eq!(known, [("Claude Code", Some("claude")), ("Grok", Some("grok"))]);
        let mut form = CliForm::new();
        assert!(form.agent(&agents.local).is_none());
        form.name = " Codex ".to_owned();
        assert!(form.agent(&agents.local).is_none());
        form.program = "codex".to_owned();
        form.models = "gpt, , mini ".to_owned();
        form.arguments = " exec {prompt} ".to_owned();
        let agent = form.agent(&agents.local).unwrap();
        assert_eq!((agent.name.as_str(), agent.arguments.as_str(), agent.off), ("Codex", "exec {prompt}", false));
        assert_eq!(agent.models, ["gpt", "mini"]);
        assert_eq!(CliForm::of(2, &agent).agent(&agents.local), Some(agent));
        // A name another agent has is taken, unless it is the agent being changed; a known kind needs no program.
        form.name = "Grok".to_owned();
        assert!(form.agent(&agents.local).is_none());
        let grok = CliForm::of(1, &agents.local[1]);
        assert_eq!(grok.agent(&agents.local).as_ref(), Some(&agents.local[1]));
        assert_eq!(hook_line("grok", "/Apps/My Pencraft/pencraft"), r#"grok mcp add pencraft -- "/Apps/My Pencraft/pencraft" mcp"#);
        // The cards follow what the search found, and a turned-off agent says so.
        let mut settings = Settings::default();
        assert_eq!(settings.local_link(&agents.local[1]), Link::Checking);
        settings.installed = Some(vec![("claude".to_owned(), true), ("grok".to_owned(), false)]);
        assert_eq!((settings.local_link(&agents.local[0]), settings.local_link(&agents.local[1])), (Link::Installed, Link::Missing));
        assert_eq!(settings.local_link(&LocalAgent { off: true, ..agents.local[0].clone() }), Link::Off);
    }

    #[test]
    fn a_card_says_what_is_installed_or_set_up() {
        let (mut settings, mut prefs) = (Settings::default(), Prefs::default());
        // Claude Code being installed says nothing about Anthropic's API: without a key its card isn't connected.
        settings.installed = Some(vec![("claude".to_owned(), true)]);
        assert_eq!(provider_link("Anthropic Claude Code", &prefs), Link::NotConnected);
        assert_eq!(provider_link("Cursor", &prefs), Link::NotConnected);
        assert_eq!(provider_link("xAI", &prefs), Link::NotConnected);
        // A known provider's card opens its form with everything but the key and the models filled in.
        assert_eq!(settings.open_known("xAI", &prefs), None);
        let mut form = settings.form.take().unwrap();
        assert_eq!(
            (form.index, form.name.as_str(), form.api.as_str(), form.base_url.as_str()),
            (None, "xAI", "OpenAI Completions", "https://api.x.ai/v1")
        );
        assert!(form.provider().is_none());
        form.add_model("grok");
        form.add_model("grok-mini");
        assert_eq!(form.models.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(), ["grok", "grok-mini"]);
        form.key = "secret".to_owned();
        save(&mut form, &mut prefs, &Memory::default()).unwrap();
        assert_eq!(provider_link("xAI", &prefs), Link::Ready);
        // Saved, the same card opens what was saved.
        assert_eq!(settings.open_known("xAI", &prefs), None);
        assert_eq!(settings.form.take().map(|form| (form.index, form.models.len())), Some((Some(0), 2)));
        // Anthropic's comes with its models; a provider without a key API says so.
        settings.open_known("Anthropic Claude Code", &prefs);
        assert_eq!(settings.form.take().map(|form| (form.api, form.models.len())), Some(("Anthropic Messages".to_owned(), 4)));
        assert!(settings.open_known("Cursor", &prefs).is_some_and(|why| why.starts_with("Cursor has no API")) && settings.form.is_none());
        // Ollama's card opens on this computer's address and is ready without a key.
        assert_eq!(settings.open_known(catalog::OLLAMA, &prefs), None);
        let mut form = settings.form.take().unwrap();
        assert_eq!((form.api.as_str(), form.base_url.as_str()), ("Ollama", "http://localhost:11434"));
        assert!(!unprotected(&form.base_url));
        form.add_model("llama3.3");
        save(&mut form, &mut prefs, &Memory::default()).unwrap();
        assert_eq!(provider_link(catalog::OLLAMA, &prefs), Link::Ready);
        assert!(prefs.agents.providers.iter().any(|provider| provider.name == catalog::OLLAMA && provider.key_ref.is_none()));
        // Ollama's cloud opens on its own address, encrypted, and wants a key.
        assert_eq!(settings.open_known(catalog::OLLAMA_CLOUD, &prefs), None);
        let form = settings.form.take().unwrap();
        assert_eq!((form.api.as_str(), form.base_url.as_str()), ("OpenAI Completions", "https://ollama.com/v1"));
        assert!(!unprotected(&form.base_url) && catalog::wants_key(&form.base_url));
        // Every card either opens a form or says why not.
        let closed: Vec<&str> = PROVIDERS.iter().map(|(name, ..)| *name).filter(|name| catalog::preset(name).is_none()).collect();
        assert_eq!(closed, ["Cursor", "GitHub Copilot"]);
    }

    #[test]
    fn names_are_kept_per_assistant_and_setups_name_this_program() {
        let mut names = Vec::new();
        set_name(&mut names, "code/a", "Otto".to_owned());
        set_name(&mut names, "code/b", "Ada".to_owned());
        set_name(&mut names, "code/a", "Otto II".to_owned());
        assert_eq!(names.iter().map(|n| (n.id.as_str(), n.name.as_str())).collect::<Vec<_>>(), [("code/b", "Ada"), ("code/a", "Otto II")]);
        set_name(&mut names, "code/b", " ".to_owned());
        assert_eq!(names.len(), 1);
        // The settings file takes the names and the tools that are off, and gives them back.
        let mut prefs = Prefs::default();
        prefs.agents.names = names;
        prefs.agents.disabled_tools = vec!["export_layers".to_owned()];
        let written = serde_json::to_string(&prefs).unwrap();
        assert_eq!(serde_json::from_str::<Prefs>(&written).unwrap(), prefs);
        // A path with a space stays one word for the shell and one string in the files.
        let [(_, _, claude), (_, _, codex), (_, _, json)] = client_setups("/Apps/My Pencraft/pencraft");
        assert_eq!(claude, r#"claude mcp add pencraft -- "/Apps/My Pencraft/pencraft" mcp"#);
        assert!(codex.contains(r#"command = "/Apps/My Pencraft/pencraft""#));
        assert_eq!(serde_json::from_str::<serde_json::Value>(&json).unwrap()["mcpServers"]["pencraft"]["args"], serde_json::json!(["mcp"]));
        assert_eq!(tool_words("Read layers. Then more."), "Read layers");
        assert_eq!(tool_words(&"a".repeat(200)).chars().count(), MOST_TOOL_WORDS);
        // Every tool's line on the page is short enough for its row.
        let tools = crate::agent::tools();
        assert!(tools.as_array().unwrap().iter().all(|tool| !tool_words(tool["description"].as_str().unwrap()).is_empty()));
    }

    #[test]
    fn every_settings_icon_is_bundled() {
        for (_, icon, _) in PAGES.iter().flat_map(|(_, pages)| pages.iter()) {
            assert!(icons::outline("lucide", icon).is_some(), "{icon}");
        }
        let used = [
            "server",
            "trash-2",
            "chevrons-up-down",
            "chevron-right",
            "chevron-left",
            "chevron-down",
            "terminal",
            "sun",
            "moon",
            "monitor",
            "x",
            "search",
            "plus",
            "copy",
            "download",
            "globe",
            "image",
            "key-round",
            "shield-check",
            "circle-check",
            "circle-arrow-up",
        ];
        for icon in used {
            assert!(icons::outline("lucide", icon).is_some(), "{icon}");
        }
    }

    #[test]
    fn the_search_finds_settings_tools_and_providers_on_their_pages() {
        assert!(finds("  ").is_empty() && finds("no such setting").is_empty());
        let grid = finds("Pixel Grid");
        assert_eq!(
            grid.iter().map(|(page, _, name)| (*page, name.as_str())).collect::<Vec<_>>(),
            [(Page::Canvas, "Show pixel grid"), (Page::Canvas, "Snap to pixel grid")]
        );
        assert_eq!(finds("edit_layers"), [(Page::Permissions, "Tools".to_owned(), "edit_layers".to_owned())]);
        assert!(finds("deepseek").contains(&(Page::Agents, "Add an assistant".to_owned(), "Deepseek".to_owned())));
        // A page's name finds everything on it, in the order of the list of pages.
        let pages: Vec<Page> = finds("a").iter().map(|(page, ..)| *page).collect();
        assert!(pages.is_sorted_by_key(|page| PAGES.iter().flat_map(|(_, pages)| pages.iter()).position(|(known, ..)| known == page)));
        assert!(finds("mcp").iter().any(|(page, ..)| *page == Page::Mcp));
        // Every page of the index is in the list of pages.
        assert!(INDEX.iter().all(|(page, ..)| entry(*page).0 == *page));
    }

    #[test]
    fn a_models_row_takes_the_providers_numbers_unless_the_user_typed_their_own() {
        use catalog::Limits;
        let mut form = Form::new();
        form.models = vec![
            ModelRow { id: "big".to_owned(), ..ModelRow::empty() },
            ModelRow { id: "mine".to_owned(), context_window: "8000".to_owned(), max_tokens: "2000".to_owned(), ..ModelRow::empty() },
            ModelRow { id: "reads-only".to_owned(), ..ModelRow::empty() },
            ModelRow { id: "unknown".to_owned(), ..ModelRow::empty() },
        ];
        form.learn([
            ("big".to_owned(), Limits { context: Some(1_048_576), output: Some(65_536) }),
            ("mine".to_owned(), Limits { context: Some(200_000), output: Some(64_000) }),
            ("reads-only".to_owned(), Limits { context: Some(131_072), output: None }),
        ]);
        let rows: Vec<(&str, &str)> = form.models.iter().map(|row| (row.context_window.as_str(), row.max_tokens.as_str())).collect();
        // A model that can't write more than it reads has its placeholder brought down to what it reads.
        assert_eq!(rows, [("1048576", "65536"), ("8000", "2000"), ("131072", "131072"), (DEFAULT_TOKENS, DEFAULT_TOKENS)]);
        // A later answer that says less doesn't take away what is known, and a chosen model gets its numbers.
        form.learn([("big".to_owned(), Limits::default()), ("new".to_owned(), Limits { context: Some(32_768), output: Some(4_096) })]);
        let offered = ["big", "new"].map(str::to_owned);
        form.take_choice(&offered, &offered.iter().cloned().collect());
        let new = form.models.iter().find(|row| row.id == "new").map(|row| (row.context_window.as_str(), row.max_tokens.as_str()));
        assert_eq!((new, form.models[0].context_window.as_str()), (Some(("32768", "4096")), "1048576"));
        // Only an Ollama is asked about its models one by one, and only about those it hasn't told of.
        assert_eq!(form.unasked(), [] as [String; 0]);
        form.base_url = "https://ollama.com/v1".to_owned();
        assert_eq!(form.unasked(), ["unknown"]);
    }

    #[test]
    fn the_theme_follows_the_computer_or_stays_as_chosen() {
        let mut look = Appearance { dark: true, ..Appearance::default() };
        follow_theme(&mut look, THEME_SYSTEM);
        assert!(look.follow_system && look.dark);
        follow_theme(&mut look, THEME_LIGHT);
        assert!(!look.follow_system && !look.dark);
        follow_theme(&mut look, THEME_DARK);
        assert!(!look.follow_system && look.dark);
    }

    #[test]
    fn rows_say_who_owns_an_assistant_and_how_long_ago_a_look_was() {
        assert!(owns("Claude Code", "Claude Code · Opus 5.5") && owns("Grok", "Grok"));
        assert!(!owns("Grok", "Grok 2 · fast") && !owns("Claude", "Claude Code · Opus 5.5"));
        assert_eq!((counted(1, "model"), counted(3, "model")), ("1 model".to_owned(), "3 models".to_owned()));
        assert_eq!(ago(Duration::from_secs(20)), "just now");
        assert_eq!(ago(Duration::from_secs(90)), "1 minute ago");
        assert_eq!(ago(Duration::from_secs(3 * 3600 + 5)), "3 hours ago");
        let seen = |looking, offered: Option<&str>, ago| crate::update::Seen { looking, offered: offered.map(str::to_owned), ago };
        assert_eq!(update_line(&seen(true, Some("9.0.0"), None)), "Looking for a newer version…");
        assert_eq!(update_line(&seen(false, Some("9.0.0"), None)), "Quill 9.0.0 is available.");
        assert_eq!(update_line(&seen(false, None, Some(Duration::from_secs(600)))), "Last checked 10 minutes ago.");
        assert!(update_line(&seen(false, None, None)).starts_with("Not checked yet"));
    }

    #[test]
    fn a_forms_bar_says_what_is_missing() {
        let mut form = Form::new();
        assert_eq!(form.missing(), Some("Name and base URL are still missing."));
        form.name = "Mine".to_owned();
        assert_eq!(form.missing(), Some("The base URL is still missing."));
        form.base_url = "https://example.com/v1".to_owned();
        assert!(form.missing().is_some_and(|what| what.starts_with("Every model")));
        form.models[0].id = "m".to_owned();
        assert_eq!(form.missing(), None);
        let agents = Agents::default();
        let mut cli = CliForm::new();
        assert_eq!(cli.missing(&agents.local), Some("A name is still missing."));
        cli.name = "Grok".to_owned();
        cli.program = "grok".to_owned();
        assert_eq!(cli.missing(&agents.local), Some("Another agent has this name already."));
        cli.name = "Codex".to_owned();
        assert_eq!(cli.missing(&agents.local), None);
        let mut image = ImageForm::new();
        assert_eq!(image.missing(&Images::default()), Some("A name is still missing."));
        image.name = "Studio".to_owned();
        assert_eq!(image.missing(&Images::default()), None);
    }
}
