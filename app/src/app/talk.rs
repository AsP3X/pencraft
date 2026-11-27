//! The chat panel's side of the app: the chats there are (one for each open document, and one for the home
//! screen), finding the assistants there are, and sending the user's message with what the model needs to know
//! about the document.

use std::sync::Arc;

use std::path::PathBuf;

use craft_dialogs::FileDialog;

use super::{App, Then};
use crate::chat::{self, Ask, Chat, Waiting, files};
use crate::sidebar::{ChatAsk, Look, Writing};

/// One conversation: the chat of one open document, or of the home screen.
// Human: every open document has a chat of its own. They run side by side: an assistant at work in one document
// keeps to that document while the user looks at another one and talks to its chat.
#[derive(Default)]
pub(super) struct Talk {
    /// Its number among the chats of this run, which its tool calls carry ([`crate::link::Call::chat`]).
    pub(super) id: u64,
    /// The document it belongs to ([`crate::editor::Editor::id`]); `None` for the home screen's.
    pub(super) doc: Option<u64>,
    /// The conversation and its run.
    pub(super) chat: Chat,
    /// How many of its lines have been written to the log.
    logged: usize,
    /// What was being written in it when another chat came to the front.
    kept: Writing,
    /// The design file its conversation was last kept for, how many lines it had then, and what it had taken.
    saved: Option<(PathBuf, usize, chat::usage::Spent)>,
}

/// The document a tool call works on when it names none.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Home {
    /// The one in front: the call comes from outside the app, or from the home screen's chat.
    Front,
    /// The document with this id: the one the call's chat belongs to.
    Doc(u64),
    /// None any more: the call's chat was closed with its document.
    Gone,
}

/// Where a call from `chat` works, given the chats there are.
pub(super) fn home(talks: &[Talk], chat: Option<u64>) -> Home {
    let Some(id) = chat else { return Home::Front };
    match talks.iter().find(|talk| talk.id == id) {
        Some(talk) => talk.doc.map_or(Home::Front, Home::Doc),
        None => Home::Gone,
    }
}

/// What the user is told when there is nobody to answer.
const NO_ASSISTANT: &str = "No assistant is set up. Install Claude Code or Grok and sign in (run `claude` or `grok` in a terminal once), or connect a provider with its API key in Settings → Agents, or start Quill with ANTHROPIC_API_KEY set.";
/// What the user is told when the tools are turned off.
const TOOLS_OFF: &str = "The assistant works on the document through Quill's AI tools, which are turned off. Turn on \"Let AI models work on documents\" in Settings → Agents.";

/// How the sidebar shows the chat `talk`: as the settings `prefs` want it, with the `assistants` to pick from
/// and the one `asked` for when the app was started.
pub(super) fn look<'a>(talk: &'a Talk, prefs: &crate::prefs::Prefs, assistants: &'a [chat::Choice], asked: Option<&str>) -> Look<'a> {
    let assistant = chat::chosen(assistants, asked.unwrap_or(&prefs.agents.chat_model)).map_or("No assistant", |choice| choice.label.as_str());
    Look {
        dark: prefs.appearance.dark,
        ideas: prefs.agents.ideas,
        chat_text: match prefs.agents.chat_text_size.as_str() {
            "Medium" => crate::theme::CHAT_TEXT_MEDIUM,
            "Large" => crate::theme::CHAT_TEXT_LARGE,
            _ => 1.0,
        },
        chat: &talk.chat,
        chat_id: talk.id,
        assistants,
        assistant,
        animate: prefs.appearance.animations,
    }
}

impl App {
    /// Looks up which assistants there are: the command-line agents that are installed, and the providers.
    // Agent: READS the folders of PATH and the environment; fast enough for a click.
    pub(super) fn find_assistants(&mut self) {
        let agents = &self.prefs.get().agents;
        let mut found = chat::choices(agents, &crate::settings::locate, chat::key_in_environment());
        chat::rename(&mut found, &agents.names);
        self.assistants = found;
    }

    /// The assistant the chat talks to.
    pub(super) fn assistant(&self) -> Option<&chat::Choice> {
        chat::chosen(&self.assistants, self.assistant_asked.as_deref().unwrap_or(&self.prefs.get().agents.chat_model))
    }

    /// The id of the document in front; `None` on the home screen.
    pub(super) fn front_doc(&self) -> Option<u64> {
        self.active.and_then(|tab| self.tabs.get(tab)).map(|editor| editor.id)
    }

    /// The chat with the number `id`.
    pub(super) fn talk(&self, id: u64) -> Option<&Talk> {
        self.talks.iter().find(|talk| talk.id == id)
    }

    /// The chat with the id `id`, to change it.
    pub(super) fn talk_mut(&mut self, id: u64) -> Option<&mut Talk> {
        self.talks.iter_mut().find(|talk| talk.id == id)
    }

    /// The chat of the document in front (on the home screen, the home screen's), started now if it has none.
    pub(super) fn front_talk(&mut self) -> &mut Talk {
        let doc = self.front_doc();
        let at = self.talks.iter().position(|talk| talk.doc == doc).unwrap_or_else(|| {
            self.talks_made += 1;
            let mut talk = Talk { id: self.talks_made, doc, ..Talk::default() };
            // Human: a design's chat starts with what was said about it before.
            let file = self.active.and_then(|tab| self.tabs.get(tab)).and_then(|editor| editor.path.clone()).filter(|_| !self.private);
            if let Some(file) = file {
                (talk.chat.entries, talk.chat.total) = chat::history::read(&file);
                talk.logged = talk.chat.entries.len();
                talk.saved = Some((file, talk.chat.entries.len(), talk.chat.total.clone()));
            }
            self.talks.push(talk);
            self.talks.len() - 1
        });
        &mut self.talks[at]
    }

    /// The chat the sidebar shows; before it has shown one, the chat of the document in front.
    fn shown_talk(&mut self) -> &mut Talk {
        match self.talks.iter().position(|talk| talk.id == self.shown) {
            Some(at) => &mut self.talks[at],
            None => self.front_talk(),
        }
    }

    /// Makes the sidebar show the chat of the document in front: what was being written in the chat it showed
    /// is kept for that chat, and what was being written in this one comes back.
    pub(super) fn bring_chat(&mut self) {
        let front = self.front_talk().id;
        if front == self.shown {
            return;
        }
        // Human: before the sidebar has shown any chat, what it holds (files attached when the app started)
        // is meant for the first one.
        if self.shown != 0 {
            let mut writing = Writing::default();
            self.sidebar.trade_writing(&mut writing);
            // A chat that was closed with its document takes what was written in it along.
            if let Some(left) = self.talks.iter_mut().find(|talk| talk.id == self.shown) {
                left.kept = writing;
            }
            if let Some(talk) = self.talks.iter_mut().find(|talk| talk.id == front) {
                self.sidebar.trade_writing(&mut talk.kept);
            }
        }
        self.shown = front;
    }

    /// The document `doc` is closed: its chat ends with it.
    pub(super) fn close_talk(&mut self, doc: u64) {
        self.talks.retain_mut(|talk| {
            let closed = talk.doc == Some(doc);
            if closed {
                talk.chat.clear();
            }
            !closed
        });
        self.designed.retain(|(at, _, _)| *at != doc);
    }

    /// The chat `id` has opened the document `doc`: the chat goes with it, when that document has no
    /// conversation of its own.
    // Human: a chat started on the home screen that starts a design is from then on that design's chat, and so
    // is a chat that opens another file to work there. The document it came from starts a new chat.
    pub(super) fn follow(&mut self, id: u64, doc: u64) {
        if self.talk(id).is_none_or(|talk| talk.doc == Some(doc)) {
            return;
        }
        if let Some(at) = self.talks.iter().position(|talk| talk.doc == Some(doc)) {
            let there = &self.talks[at];
            if !there.chat.entries.is_empty() || there.chat.running() || !there.kept.is_empty() || there.id == self.shown {
                return;
            }
            self.talks.remove(at);
        }
        if let Some(talk) = self.talks.iter_mut().find(|talk| talk.id == id) {
            talk.doc = Some(doc);
        }
    }

    /// Whether the chat of the document at `tab` is answering a message.
    pub(super) fn chat_at_work(&self, tab: usize) -> bool {
        let doc = self.tabs.get(tab).map(|editor| editor.id);
        doc.is_some() && self.talks.iter().any(|talk| talk.doc == doc && talk.chat.running())
    }

    /// Takes in what the chats' runs have said since the last time. Returns whether anything changed.
    pub(super) fn poll_chats(&mut self) -> bool {
        let mut changed = false;
        for talk in &mut self.talks {
            changed |= talk.chat.poll();
        }
        changed
    }

    /// Keeps what was said in each design's chat for the next run, once an answer is done or the chat was
    /// started over. A design without a file has no place to keep it yet; it is kept once it is saved.
    // Agent: WRITES a small file per chat that changed, in the window's thread.
    pub(super) fn keep_chats(&mut self) {
        if self.private {
            return;
        }
        for talk in &mut self.talks {
            let file = talk.doc.and_then(|doc| self.tabs.iter().find(|tab| tab.id == doc)).and_then(|editor| editor.path.clone());
            let Some(file) = file.filter(|_| !talk.chat.running()) else { continue };
            // Human: the sums count too: a stopped answer can add tokens to the total without adding a line.
            let now = (file, talk.chat.entries.len(), talk.chat.total.clone());
            // A design never talked about is not written for.
            let nothing = talk.saved.is_none() && now.1 == 0;
            if nothing || talk.saved.as_ref() == Some(&now) {
                continue;
            }
            if let Err(problem) = chat::history::keep(&now.0, &talk.chat.entries, &talk.chat.total) {
                log::warn!("chat history: {problem}");
            }
            talk.saved = Some(now);
        }
    }

    /// Does what the chat panel asked for this frame: stop, start over, another assistant, send.
    pub(super) fn chat_asks(&mut self, ctx: &egui::Context) {
        for ask in std::mem::take(&mut self.sidebar.chat_asks) {
            match ask {
                ChatAsk::Stop => self.shown_talk().chat.stop(),
                ChatAsk::New => self.ask_new_chat(),
                ChatAsk::Looked => self.find_assistants(),
                ChatAsk::Assistants => self.settings.open_named("agents"),
                ChatAsk::Pick(id) => {
                    self.assistant_asked = None;
                    self.prefs.update(|prefs| prefs.agents.chat_model = id);
                }
                ChatAsk::Send { shown, pointed, told, files, pasted } => self.send_chat(shown, pointed, told, &files, pasted, ctx),
                ChatAsk::Attach => self.ask_attach(),
                ChatAsk::Answer(pick) => self.answer_chat(&pick, ctx),
                ChatAsk::Offer(act) => self.take_offer(act, ctx),
            }
        }
    }

    /// Starts the shown chat over, asking first when that would lose something.
    // Human: starting over replaces the kept conversation with an empty one, so it can't be got back; an empty
    // chat with nothing running has nothing to lose and starts over without a question.
    fn ask_new_chat(&mut self) {
        let talk = self.shown_talk();
        if talk.chat.entries.is_empty() && !talk.chat.running() {
            talk.chat.clear();
        } else {
            self.new_chat_asked = Some(talk.id);
        }
    }

    /// Asks whether to start a chat over that has a conversation or a run in it; clears it on yes.
    // Agent: the chat is found again by its number, so a tab closed while asking just ends the question.
    pub(super) fn new_chat_prompt(&mut self, ctx: &egui::Context) {
        let Some(id) = self.new_chat_asked else { return };
        let Some(talk) = self.talks.iter().find(|talk| talk.id == id) else {
            self.new_chat_asked = None;
            return;
        };
        let running = talk.chat.running();
        let palette = crate::theme::palette(self.dark());
        let mut choice = None;
        let modal = egui::Modal::new(egui::Id::unique("new-chat")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(egui::RichText::new("Start a new chat?").strong());
            ui.label(if running {
                "The assistant will stop, and this conversation will be gone for good."
            } else {
                "This conversation will be gone for good."
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.add(super::dialog_primary("Cancel", palette)).clicked() {
                    choice = Some(false);
                }
                if ui.add(super::dialog_button("New chat")).clicked() {
                    choice = Some(true);
                }
            });
        });
        // Human: as in the other dialogs, Return takes the highlighted choice (Cancel); Escape or a click beside
        // the question is Cancel too.
        // Agent: only when no button acted this frame: a button reached with Tab takes Return itself.
        let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        if choice.is_none() && (enter || modal.should_close()) {
            choice = Some(false);
        }
        let Some(clear) = choice else { return };
        self.new_chat_asked = None;
        if clear && let Some(talk) = self.talks.iter_mut().find(|talk| talk.id == id) {
            talk.chat.clear();
        }
    }

    /// Writes the chats' finished lines to the log (`RUST_LOG=info`), so a run can be followed from a terminal.
    pub(super) fn log_chats(&mut self) {
        for talk in &mut self.talks {
            let chat = &talk.chat;
            // The last line is still being written while the assistant works.
            let done = if chat.running() { chat.entries.len().saturating_sub(1) } else { chat.entries.len() };
            for entry in chat.entries.iter().take(done).skip(talk.logged) {
                log::info!("chat {}: {:?}: {}", talk.id, entry.role, entry.text);
            }
            talk.logged = done.max(talk.logged).min(chat.entries.len());
        }
    }

    /// The app's note to the model on the document `doc` its chat belongs to (`None`: the home screen's chat).
    fn chat_context(&self, doc: Option<u64>) -> String {
        let Some(editor) = doc.and_then(|doc| self.tabs.iter().find(|editor| editor.id == doc)) else {
            let open = if self.tabs.is_empty() {
                "No document is open; open_document starts one."
            } else {
                "This chat has no document of its own yet. get_state lists the open ones; open_document opens or starts one."
            };
            return format!("<pencraft_context>\n{open}\n</pencraft_context>");
        };
        let file = editor.path.as_ref().map_or_else(|| "not saved to a file yet".to_owned(), |path| path.display().to_string());
        let selected: Vec<String> =
            editor.selection.iter().filter_map(|id| editor.scene_node(id).map(|scene| format!("\"{}\" ({id})", scene.node.display_name()))).collect();
        let selection = if selected.is_empty() { "Nothing is selected.".to_owned() } else { format!("Selected layers: {}.", selected.join(", ")) };
        format!("<pencraft_context>\nThis chat's document: \"{}\" ({file}).\n{selection}\n</pencraft_context>", editor.title())
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
        let (designs, others): (Vec<_>, Vec<_>) = dropped.into_iter().partition(|path| crate::bundle::is_design(path));
        for path in designs {
            self.open_path(&path);
        }
        if !others.is_empty() {
            self.attach(others);
        }
    }

    /// Takes a message the user sent: it goes to the chosen assistant, or, while one is at work, to that one
    /// at once or to the end of the waiting messages.
    // Human: "Always steer messages" hands a message to a working assistant that can take one (an API
    // assistant, between its steps); without it, or with Claude Code, the message waits for the run to end.
    fn send_chat(
        &mut self,
        shown: String,
        mut pointed: Vec<chat::Pointed>,
        text: String,
        paths: &[PathBuf],
        pasted: Vec<(String, PathBuf)>,
        ctx: &egui::Context,
    ) {
        // Human: the panel shows the attached files by name under the message; a message of files alone asks
        // the assistant to look at them.
        let names: Vec<String> = paths.iter().map(|path| files::name(path)).collect();
        let (shown, told) = match (names.is_empty(), text.is_empty()) {
            (true, _) => (shown, text),
            (false, true) => {
                // No words of the user's are shown, so no pasted layer has a place; what was selected stays.
                pointed.retain(|layer| layer.at.is_none());
                (format!("Attached: {}", names.join(", ")), "Look at the attached files.".to_owned())
            }
            (false, false) => (format!("{shown}\n\nAttached: {}", names.join(", ")), text),
        };
        // Human: a message written while the assistant's question is open is the answer to it; the answers
        // picked before that go along.
        let told = match self.shown_talk().chat.leave_question() {
            Some(picked) => format!("{picked}\n\n{told}"),
            None => told,
        };
        self.hand_over(Waiting { shown, told, files: paths.to_vec(), pasted, pointed }, ctx);
    }

    /// Does what the user did in a question of the assistant's: an answer is ticked or written, or what is
    /// chosen is sent, and with the last question's the answers go to the assistant as the user's message; or
    /// the question is closed.
    fn answer_chat(&mut self, pick: &crate::conversation::Pick, ctx: &egui::Context) {
        if let Some(message) = self.shown_talk().chat.choose(pick) {
            self.hand_over(message, ctx);
        }
    }

    /// Does what the user did with the next steps the assistant offers: one is sent as their message, all are
    /// sent as one message, or one is dropped.
    fn take_offer(&mut self, act: crate::offers::Act, ctx: &egui::Context) {
        use crate::offers::Act;
        let chat = &mut self.shown_talk().chat;
        let message = match act {
            Act::Take(place) => chat.take_offer(place),
            Act::TakeAll => chat.take_offers(),
            Act::Drop(place) => {
                chat.drop_offer(place);
                None
            }
        };
        if let Some(message) = message {
            self.hand_over(message, ctx);
        }
    }

    /// Gives `message` to the chat the sidebar shows: its assistant answers it, or, while one is at work, takes
    /// it at its next step or after its run, as the settings say.
    fn hand_over(&mut self, message: Waiting, ctx: &egui::Context) {
        let steer = self.prefs.get().agents.always_steer;
        let talk = self.shown_talk();
        let id = talk.id;
        if !talk.chat.running() {
            self.start_chat(id, message, ctx);
        } else if steer && talk.chat.steers() {
            self.steer_chat(id, message);
        } else {
            talk.chat.waiting.push(message);
        }
    }

    /// Hands `message` to the assistant at work in the chat `id`; if it can't take it any more, the message
    /// waits its turn.
    fn steer_chat(&mut self, id: u64, message: Waiting) {
        match message.read() {
            Ok(attached) => {
                let context = self.chat_context(self.talk(id).and_then(|talk| talk.doc));
                if let Some(talk) = self.talks.iter_mut().find(|talk| talk.id == id)
                    && let Err(message) = talk.chat.steer(message, &context, &attached)
                {
                    talk.chat.waiting.push(message);
                }
            }
            Err(why) => self.say(why),
        }
    }

    /// Sends the waiting message whose turn it is in each chat, once its assistant is done with the one before.
    // Agent: MAIN-THREAD; runs from `logic`, so a queue is worked off also while the window is hidden.
    pub(super) fn send_waiting(&mut self, ctx: &egui::Context) {
        let next: Vec<(u64, Waiting)> = self.talks.iter_mut().filter_map(|talk| talk.chat.next_waiting().map(|message| (talk.id, message))).collect();
        for (id, message) in next {
            self.start_chat(id, message, ctx);
        }
    }

    /// Starts the run that answers `message` in the chat `id` with the chosen assistant.
    fn start_chat(&mut self, id: u64, message: Waiting, ctx: &egui::Context) {
        let attached = message.read();
        let Waiting { shown, told: text, pointed, .. } = message;
        self.find_assistants();
        let refusal = match &attached {
            Err(why) => Some(why.clone()),
            Ok(_) if !self.prefs.get().agents.tools => Some(TOOLS_OFF.to_owned()),
            Ok(_) if self.assistant().is_none() => Some(NO_ASSISTANT.to_owned()),
            Ok(_) => None,
        };
        let picked = self.assistant().map(|choice| {
            let pricing = chat::pricing(choice, &self.prefs.get().agents, &crate::app::lock(&self.prices));
            (choice.engine.clone(), choice.model_name().to_owned(), pricing)
        });
        let (Ok(attached), None, Some((engine, model, pricing))) = (attached, &refusal, picked) else {
            if let (Some(why), Some(talk)) = (refusal, self.talks.iter_mut().find(|talk| talk.id == id)) {
                talk.chat.refuse(shown, &why);
            }
            return;
        };
        let wake = ctx.clone();
        let doc = self.talk(id).and_then(|talk| talk.doc);
        let agents = &self.prefs.get().agents;
        let ask = Ask {
            text,
            shown,
            pointed,
            files: attached,
            context: self.chat_context(doc),
            chat: id,
            engine,
            instructions: agents.instructions.clone(),
            off: agents.tools_off(),
            model,
            tools: self.tool_send.clone(),
            wake: Arc::new(move || wake.request_repaint()),
            server: self.link.as_ref().map(|server| (server.port, server.token.clone())),
            review: agents.review,
            pricing,
        };
        if let Some(talk) = self.talks.iter_mut().find(|talk| talk.id == id) {
            talk.chat.send(ask);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_works_on_its_chats_document() {
        let talks = [Talk { id: 1, doc: None, ..Talk::default() }, Talk { id: 2, doc: Some(7), ..Talk::default() }];
        // A client outside the app, and the home screen's chat, work on the document in front.
        assert_eq!((home(&talks, None), home(&talks, Some(1))), (Home::Front, Home::Front));
        // A document's chat works on that document, wherever its tab is.
        assert_eq!(home(&talks, Some(2)), Home::Doc(7));
        // A chat that was closed with its document works on nothing else.
        assert_eq!(home(&talks, Some(3)), Home::Gone);
    }
}
