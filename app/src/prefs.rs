//! What the app remembers between runs, stored by craft-settings in the Craft suite's settings folder.

use craft_settings::{DocumentMut, MigrateError, Settings};
use serde::{Deserialize, Serialize};

/// How many recent files the home screen lists.
pub const RECENT_LIMIT: usize = 12;

/// All settings, one `[section]` each in `config.toml`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// Look of the window.
    pub appearance: Appearance,
    /// Recent and open files.
    pub files: Files,
    /// How the canvas behaves.
    pub canvas: CanvasPrefs,
    /// Importing pages and generating drawings.
    pub generation: Generation,
    /// The image model services and who may use them.
    pub images: Images,
    /// The assistants and their providers.
    pub agents: Agents,
    /// Looking for newer versions of the app.
    pub updates: Updates,
}

/// Look of the window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    /// Dark instead of light.
    pub dark: bool,
    /// The interface language; "System" follows the computer.
    pub language: String,
    /// Animations and effects in the interface.
    pub animations: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance { dark: false, language: "System".to_owned(), animations: true }
    }
}

/// How the canvas behaves (the settings window's Canvas and nudge rows).
#[expect(clippy::struct_excessive_bools, reason = "each is one independent switch of the settings window")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CanvasPrefs {
    /// Show the pixel grid when zoomed far in.
    pub show_pixel_grid: bool,
    /// New and drawn things land on whole pixels (on by default).
    pub snap_to_pixels: bool,
    /// Moved things snap to the edges of others.
    pub snap_to_objects: bool,
    /// The wheel zooms without ⌘ or ctrl.
    pub wheel_zooms: bool,
    /// Zooming goes the other way.
    pub invert_zoom: bool,
    /// How far an arrow key moves the selection.
    pub small_nudge: f32,
    /// How far ⇧ + arrow moves it.
    pub big_nudge: f32,
}

impl Default for CanvasPrefs {
    fn default() -> Self {
        CanvasPrefs {
            show_pixel_grid: true,
            snap_to_pixels: true,
            snap_to_objects: true,
            wheel_zooms: false,
            invert_zoom: false,
            small_nudge: crate::ops::NUDGE,
            big_nudge: crate::ops::BIG_NUDGE,
        }
    }
}

/// Importing pages and generating drawings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Generation {
    /// Keep a page's HTML metadata when importing it.
    pub html_metadata: bool,
    /// Who makes generated SVG drawings.
    pub svg_provider: String,
}

impl Default for Generation {
    fn default() -> Self {
        Generation { html_metadata: false, svg_provider: "Pencraft".to_owned() }
    }
}

/// The image model services and who may use them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Images {
    /// The services the user added. Their API keys are in the keychain, not here.
    pub services: Vec<ImageService>,
    /// The name of the service used when none is asked for; empty for the first one that works.
    pub default: String,
    /// AI models may generate pictures (each one costs money on the user's key).
    pub assistant: bool,
}

impl Default for Images {
    fn default() -> Self {
        Images { services: Vec::new(), default: String::new(), assistant: true }
    }
}

/// One account at one image model provider.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageService {
    /// What the user calls it.
    pub name: String,
    /// The kind of API it speaks, as [`crate::imagery::Kind::name`] gives it.
    pub kind: String,
    /// Where it is reached; empty for the kind's own address.
    pub base_url: String,
    /// The model; empty for the kind's usual one.
    pub model: String,
    /// The name its API key is kept under in the keychain; `None` when it has no key. Never the key itself.
    pub key_ref: Option<String>,
}

/// The assistants and their providers.
#[expect(clippy::struct_excessive_bools, reason = "each is one independent switch of the settings window")]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Agents {
    /// AI models on this computer may read and change open documents through the app's tools.
    pub tools: bool,
    /// The assistant the chat talks to, as [`crate::chat::Choice::id`] names it; empty for the first one there is.
    pub chat_model: String,
    /// Send messages to a running assistant at once instead of queueing them.
    pub always_steer: bool,
    /// Size of the chat's text.
    pub chat_text_size: String,
    /// Size of an assistant's cursor on the canvas.
    pub cursor_size: String,
    /// What an assistant's cursor is labelled with.
    pub cursor_label: String,
    /// Show the assistant's message box on the home screen.
    pub input_on_dashboard: bool,
    /// Show the row of ideas under the message bar of an empty document while nothing is typed.
    pub ideas: bool,
    /// Have a reviewer look at each design the chat's assistant finishes, and send it back with what to change.
    pub review: bool,
    /// Providers the user added. Their API keys are in the keychain, not here.
    pub providers: Vec<CustomProvider>,
}

impl Default for Agents {
    fn default() -> Self {
        Agents {
            tools: true,
            chat_model: String::new(),
            always_steer: false,
            chat_text_size: "Small".to_owned(),
            cursor_size: "Small (Default)".to_owned(),
            cursor_label: "Agent (Default)".to_owned(),
            input_on_dashboard: true,
            ideas: true,
            review: true,
            providers: Vec::new(),
        }
    }
}

/// Looking for newer versions of the app.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Updates {
    /// Ask GitHub for a newer version when the app starts and once a day.
    pub check: bool,
    /// The version the user chose not to be offered again; empty for none.
    pub skipped: String,
}

impl Default for Updates {
    fn default() -> Self {
        Updates { check: true, skipped: String::new() }
    }
}

/// A provider the user added: any `OpenAI` or Anthropic compatible service.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomProvider {
    /// What the user calls it.
    pub name: String,
    /// The kind of API it speaks, as the settings window names it.
    pub api: String,
    /// Where it is reached.
    pub base_url: String,
    /// Its models.
    pub models: Vec<ProviderModel>,
    /// The name its API key is kept under in the keychain; `None` when it has no key. Never the key itself.
    pub key_ref: Option<String>,
}

/// One model of a custom provider.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderModel {
    /// The name the provider knows it by.
    pub id: String,
    /// How many tokens it reads at most.
    pub context_window: u32,
    /// How many tokens it writes at most.
    pub max_tokens: u32,
}

/// Recent and open files.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Files {
    /// Most recent first.
    pub recent: Vec<String>,
    /// Files open in tabs when the app last closed.
    pub tabs: Vec<String>,
}

impl Files {
    /// Puts `path` first in the recent list.
    pub fn touch(&mut self, path: &str) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_owned());
        self.recent.truncate(RECENT_LIMIT);
    }
}

impl Settings for Prefs {
    const APP_ID: &'static str = "pencraft";
    const SCHEMA_VERSION: u32 = 1;

    fn migrate(_from: u32, _doc: &mut DocumentMut) -> Result<(), MigrateError> {
        // Human: version 1 is the first; there is nothing older to convert.
        Ok(())
    }
}
