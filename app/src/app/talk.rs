//! The chat panel's side of the app: finding the assistants there are, and sending the user's message with
//! what the model needs to know about the document.

use std::sync::Arc;

use std::path::PathBuf;

use craft_dialogs::FileDialog;

use super::{App, Then};
use crate::chat::{self, Ask, files};
use crate::sidebar::ChatAsk;

/// What the user is told when there is nobody to answer.
const NO_ASSISTANT: &str = "No assistant is set up. Install Claude Code and sign in (run `claude` in a terminal once), or add a provider with the \"Anthropic Messages\" API and a key in Settings → Agents, or start Pencraft with ANTHROPIC_API_KEY set.";
/// What the user is told when the tools are turned off.
const TOOLS_OFF: &str = "The assistant works on the document through Pencraft's AI tools, which are turned off. Turn on \"Let AI models work on documents\" in Settings → Agents.";

impl App {
    /// Looks up which assistants there are: Claude Code if it is installed, and the providers with a key.
    // Agent: READS the folders of PATH and the environment; fast enough for a click.
    pub(super) fn find_assistants(&mut self) {
        let claude = crate::settings::program("claude");
        self.assistants = chat::choices(&self.prefs.get().agents, claude.as_deref(), chat::key_in_environment());
    }

    /// The assistant the chat talks to.
    pub(super) fn assistant(&self) -> Option<&chat::Choice> {
        chat::chosen(&self.assistants, self.assistant_asked.as_deref().unwrap_or(&self.prefs.get().agents.chat_model))
    }

    /// Does what the chat panel asked for this frame: stop, start over, another assistant, send.
    pub(super) fn chat_asks(&mut self, ctx: &egui::Context) {
        for ask in std::mem::take(&mut self.sidebar.chat_asks) {
            match ask {
                ChatAsk::Stop => self.chat.stop(),
                ChatAsk::New => self.chat.clear(),
                ChatAsk::Looked => self.find_assistants(),
                ChatAsk::Pick(id) => {
                    self.assistant_asked = None;
                    self.prefs.update(|prefs| prefs.agents.chat_model = id);
                }
                ChatAsk::Send { shown, told, files } => self.send_chat(shown, told, &files, ctx),
                ChatAsk::Attach => self.ask_attach(),
            }
        }
    }

    /// Writes the chat's finished lines to the log (`RUST_LOG=info`), so a run can be followed from a terminal.
    pub(super) fn log_chat(&mut self) {
        // The last line is still being written while the assistant works.
        let done = if self.chat.running() { self.chat.entries.len().saturating_sub(1) } else { self.chat.entries.len() };
        for entry in self.chat.entries.iter().take(done).skip(self.chat_logged) {
            log::info!("chat: {:?}: {}", entry.role, entry.text);
        }
        self.chat_logged = done.max(self.chat_logged).min(self.chat.entries.len());
    }

    /// The app's note to the model on what the user is looking at.
    fn chat_context(&self) -> String {
        let Some(editor) = self.active.and_then(|tab| self.tabs.get(tab)) else {
            return "<pencraft_context>\nNo document is open; open_document starts one.\n</pencraft_context>".to_owned();
        };
        let file = editor.path.as_ref().map_or_else(|| "not saved to a file yet".to_owned(), |path| path.display().to_string());
        let selected: Vec<String> =
            editor.selection.iter().filter_map(|id| editor.scene_node(id).map(|scene| format!("\"{}\" ({id})", scene.node.display_name()))).collect();
        let selection = if selected.is_empty() { "Nothing is selected.".to_owned() } else { format!("Selected layers: {}.", selected.join(", ")) };
        format!("<pencraft_context>\nDocument in front: \"{}\" ({file}).\n{selection}\n</pencraft_context>", editor.title())
    }

    /// Asks which files to attach to the message being written.
    pub(super) fn ask_attach(&mut self) {
        self.dialogs.ask(FileDialog::open_many().title("Attach Files"), Then::Attach);
    }

    /// Attaches `paths` to the message being written; a file that can't be attached is said so and left out.
    // Agent: READS each file once here to check it; it is read again when the message is sent.
    pub(super) fn attach(&mut self, paths: Vec<PathBuf>) {
        let (good, bad): (Vec<_>, Vec<_>) = paths.into_iter().map(|path| files::read(&path).map(|_| path)).partition(Result::is_ok);
        let room = files::MOST_FILES.saturating_sub(self.sidebar.attached.len());
        if let Some(why) = bad.into_iter().find_map(Result::err) {
            self.say(why);
        } else if good.len() > room {
            self.say(format!("A message can carry {} files.", files::MOST_FILES));
        }
        self.sidebar.attach(good.into_iter().flatten());
    }

    /// Files dropped on the window: designs are opened, anything else is attached to the chat message.
    pub(super) fn dropped(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|file| file.path().to_path_buf()).collect());
        let (designs, others): (Vec<_>, Vec<_>) =
            dropped.into_iter().partition(|path| path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pen")));
        for path in designs {
            self.open_path(&path);
        }
        if !others.is_empty() {
            self.attach(others);
        }
    }

    /// Sends the user's message to the chosen assistant.
    fn send_chat(&mut self, shown: String, text: String, paths: &[PathBuf], ctx: &egui::Context) {
        // Human: the panel shows the attached files by name under the message; a message of files alone asks
        // the assistant to look at them.
        let names: Vec<String> = paths.iter().map(|path| files::name(path)).collect();
        let (shown, text) = match (names.is_empty(), text.is_empty()) {
            (true, _) => (shown, text),
            (false, true) => (format!("Attached: {}", names.join(", ")), "Look at the attached files.".to_owned()),
            (false, false) => (format!("{shown}\n\nAttached: {}", names.join(", ")), text),
        };
        let attached: Result<Vec<files::Attachment>, String> = paths.iter().map(|path| files::read(path)).collect();
        let attached = match attached {
            Ok(attached) => attached,
            Err(why) => {
                self.chat.refuse(shown, &why);
                return;
            }
        };
        self.find_assistants();
        let refusal = if !self.prefs.get().agents.tools {
            Some(TOOLS_OFF)
        } else if self.assistant().is_none() {
            Some(NO_ASSISTANT)
        } else {
            None
        };
        if let Some(why) = refusal {
            self.chat.refuse(shown, why);
            return;
        }
        let Some(engine) = self.assistant().map(|choice| choice.engine.clone()) else { return };
        let wake = ctx.clone();
        self.chat.send(Ask {
            text,
            shown,
            files: attached,
            context: self.chat_context(),
            engine,
            tools: self.tool_send.clone(),
            wake: Arc::new(move || wake.request_repaint()),
            server: self.link.as_ref().map(|server| (server.port, server.token.clone())),
            review: self.prefs.get().agents.review,
        });
    }
}
