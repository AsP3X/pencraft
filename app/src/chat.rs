//! The chat with an AI model inside the app: what was said, and the run that answers the user's last message.
//!
//! A message is answered by one of two engines, on a thread of its own: the Claude Code program installed on
//! this computer ([`code`]), or a provider's API called directly with the user's key ([`api`]). Either way the
//! model works on the document through the same tools as any outside AI client ([`crate::agent`]), run on the
//! main thread, so the user sees the same flashes and cursor on the canvas.

use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::app::lock;
use crate::link::Call;
use crate::prefs::Agents;

pub mod api;
pub mod code;
pub mod critic;
pub mod files;
pub mod steps;

use steps::{State, Step};

/// The models offered, by the name the API knows them by and the name shown; the first is the default.
// Human: the current Claude models; Opus is the default, as Anthropic recommends for general work.
pub const MODELS: [(&str, &str); 4] =
    [("claude-opus-5-5", "Opus 5.5"), ("claude-fable-5-2", "Fable 5.2"), ("claude-sonnet-5-5", "Sonnet 5.5"), ("claude-haiku-5-5", "Haiku 5.5")];
/// Where Anthropic's own API is.
const ANTHROPIC: &str = "https://api.anthropic.com";
/// The environment variable an Anthropic API key is read from when no provider with a key is set up.
const KEY_VARIABLE: &str = "ANTHROPIC_API_KEY";
/// The API type of a custom provider (as the settings window names it) that the chat can talk to.
const MESSAGES_API: &str = "Anthropic Messages";
/// How many earlier messages are retold to an engine that wasn't there for them.
const RETOLD: usize = 24;

/// What the model is told about its job, whichever engine runs it.
pub const BRIEF: &str = "You are the design assistant built into Pencraft, a canvas design editor. The user talks to you in the app's chat panel and watches the canvas while you work. \
Work on the user's open document through the Pencraft tools: call get_state first, read layers before you change them, make changes with edit_layers (one meaningful step per call, with a short description), and look at the result with screenshot before you say it is done. \
When edit_layers answers with advice, or a screenshot comes with a review, act on it. \
When you design something with a look of its own (a screen, a page, a poster, a card), work as \"Designing well\" below says: decide the direction for this subject, set colours, type and spacing as variables first, build with a clear hierarchy, then judge your screenshot as a strict art director would and improve the weakest parts before you answer. Never fall back on your defaults (Inter or Arial, a violet gradient, three equal cards, everything centred). \
When the user names a library, a product or a style to follow, or asks for standard or plain components, that reference is the brief instead: keep its face, palette and measures and add no direction of your own. \
Before the first layer, note for yourself what the user named, piece by piece; before you answer, check each piece is there and that you added nothing that was not asked for, and say so if you left something out or added something. \
Reuse the document's variables and components instead of making new ones for the same thing. Make what was asked for and nothing around it: a thing the user asks for (a dog, a logo, a button) gets no backdrop, board or coloured frame behind it unless the user asks for one or the thing itself is one, like a screen, a page or a card; keep its parts together in a group or a frame without a fill. \
A mock of a phone or tablet app is the exception the other way: its screen always stands inside the device it is for (the body, the screen's corners, the status bar, the Dynamic Island or camera, the home indicator), as \"A screen on a device\" below says, unless the user asks for the screen alone. \
A layer id followed by its name in quotes and brackets inside a message, like card2 (\"Card\"), is a layer the user pasted into the message to point at it: work on exactly those layers. \
Each message from the user starts with a note from the app in <pencraft_context> tags saying which document this chat works on (your tools work on it unless a call names another) and what is selected; \"this\" or \"the selection\" means those layers. \
Keep replies short and plain: a few sentences on what you made and what to look at, without lists of ids. The chat panel shows your words exactly as you write them, so use no Markdown of any kind: no asterisks for bold, no bullet or numbered lists, no headings, no backticks, no tables. \
If a request is unclear in a way that changes the design, ask one short question instead of guessing.";

/// The last thing an engine is told, after everything long: how its replies have to look.
// Human: said once at the start, the rule is forgotten by the time a long design is done; the end of the
// brief is what a model has freshest when it writes its answer.
const PLAIN: &str = "One more time, because it is easy to forget after a long piece of work: your reply to the user is a few plain sentences. No Markdown: no asterisks, no lists, no headings, no backticks.";

/// The brief with how to design and which fonts are installed: what an engine is told on top of what it is.
#[must_use]
pub fn brief(guide: bool) -> String {
    // Human: an engine that can't call get_guide by itself (the API) is given the document guide here.
    let guide = if guide { crate::agent::GUIDE } else { "" };
    // Human: which skills there are is said on both routes; a model that never calls get_guide would not
    // learn of them otherwise.
    format!("{BRIEF}\n\n{}\n{}\n{}\n{guide}\n{PLAIN}", crate::skills::index(), crate::craft::CRAFT, crate::craft::fonts())
}

/// Who a line of the conversation is from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The user.
    User,
    /// The model's words.
    Assistant,
    /// A note on what the model did (a tool it used), or that the run was stopped.
    Step,
    /// Something that went wrong.
    Problem,
}

/// One line of the conversation as the panel shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Who it is from.
    pub role: Role,
    /// What it says.
    pub text: String,
    /// For a note on a tool call: the call it is about.
    pub step: Option<Step>,
}

impl Entry {
    /// A line from `role` saying `text`.
    pub fn new(role: Role, text: impl Into<String>) -> Self {
        Entry { role, text: text.into(), step: None }
    }

    /// A note on a call of `tool` that is in `state`, saying the tool's usual words and `detail`.
    fn note(tool: &str, state: State, detail: &str) -> Self {
        let text = if state == State::Failed { steps::failed(tool) } else { steps::did(tool) };
        let step = Step { tool: tool.to_owned(), id: String::new(), detail: detail.to_owned(), state, times: 1, named: false };
        Entry { role: Role::Step, text, step: Some(step) }
    }
}

/// What a run tells the chat as it goes.
#[derive(Debug)]
pub enum Event {
    /// More of the model's words.
    Text(String),
    /// The model's next words start a new paragraph of the conversation.
    Break,
    /// The model starts a call of the tool `name`; `id` tells this call from the others.
    Tool {
        /// The call.
        id: String,
        /// The tool.
        name: String,
    },
    /// What the call `id` asks the tool for, once the model has written it out.
    ToolInput {
        /// The call.
        id: String,
        /// The tool's input.
        input: Value,
    },
    /// The call `id` is over.
    ToolEnd {
        /// The call.
        id: String,
        /// The tool answered with an error.
        failed: bool,
    },
    /// The Claude Code session to continue with the next message.
    Session(String),
    /// The API conversation so far, to continue with the next message.
    History(Vec<Value>),
    /// Something went wrong; the run goes on or ends as it says next.
    Problem(String),
    /// The run is over.
    Done,
}

/// Where an API key comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeySource {
    /// The `ANTHROPIC_API_KEY` environment variable.
    Environment,
    /// The keychain, under this name.
    Keychain(String),
}

/// What answers a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Engine {
    /// The Claude Code program at `program`, with the user's own sign-in.
    Code {
        /// Where the program is.
        program: PathBuf,
        /// The model it is asked to use.
        model: String,
    },
    /// A provider's Messages API, called directly.
    Api {
        /// The address messages are posted to.
        endpoint: String,
        /// Whether that is Anthropic's own API (which takes Anthropic's newer request fields).
        official: bool,
        /// The model.
        model: String,
        /// Where the key is.
        key: KeySource,
    },
}

/// One entry of the assistant menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The name the choice is remembered by in the settings.
    pub id: String,
    /// What the menu and the message box show.
    pub label: String,
    /// What answers when it is chosen.
    pub engine: Engine,
}

/// The address messages are posted to for a provider reached at `base`.
// Human: people write a provider's address with or without the "/v1" at its end; both lead to the same place.
fn endpoint(base: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    if base.ends_with("/v1") { format!("{base}/messages") } else { format!("{base}/v1/messages") }
}

/// Whether `base` is the address of Anthropic's own API.
fn is_anthropic(base: &str) -> bool {
    base.contains("://api.anthropic.com")
}

/// The assistants the user can pick from: Claude Code when it is installed (`claude` is where), Anthropic's API
/// when a key is in the environment (`key_in_environment`), and every custom provider that speaks the Messages API.
#[must_use]
pub fn choices(agents: &Agents, claude: Option<&Path>, key_in_environment: bool) -> Vec<Choice> {
    // Human: with a key in the environment, `ANTHROPIC_BASE_URL` says where it belongs, as it does for
    // Anthropic's own tools; without it that is Anthropic's API.
    let base = std::env::var("ANTHROPIC_BASE_URL").ok().filter(|base| !base.trim().is_empty()).unwrap_or_else(|| ANTHROPIC.to_owned());
    let mut out = Vec::new();
    if let Some(program) = claude {
        out.extend(MODELS.map(|(model, name)| Choice {
            id: format!("code/{model}"),
            label: format!("Claude Code · {name}"),
            engine: Engine::Code { program: program.to_path_buf(), model: model.to_owned() },
        }));
    }
    if key_in_environment {
        out.extend(MODELS.map(|(model, name)| Choice {
            id: format!("api/anthropic/{model}"),
            label: format!("Anthropic API · {name}"),
            engine: Engine::Api { endpoint: endpoint(&base), official: is_anthropic(&base), model: model.to_owned(), key: KeySource::Environment },
        }));
    }
    for provider in agents.providers.iter().filter(|p| p.api == MESSAGES_API) {
        let Some(key) = provider.key_ref.clone() else { continue };
        let official = is_anthropic(&provider.base_url);
        out.extend(provider.models.iter().map(|model| Choice {
            id: format!("api/{}/{}", provider.name, model.id),
            label: format!("{} · {}", provider.name, MODELS.iter().find(|(id, _)| *id == model.id).map_or(model.id.as_str(), |(_, name)| name)),
            engine: Engine::Api { endpoint: endpoint(&provider.base_url), official, model: model.id.clone(), key: KeySource::Keychain(key.clone()) },
        }));
    }
    out
}

/// Whether an Anthropic API key is in the environment.
#[must_use]
pub fn key_in_environment() -> bool {
    std::env::var(KEY_VARIABLE).is_ok_and(|key| !key.trim().is_empty())
}

/// The choice remembered as `id`, or the first one.
#[must_use]
pub fn chosen<'a>(all: &'a [Choice], id: &str) -> Option<&'a Choice> {
    all.iter().find(|choice| choice.id == id).or_else(|| all.first())
}

/// A short exchange, for a picture of the chat panel (`--sidebar conversation`).
#[must_use]
pub fn sample() -> Vec<Entry> {
    let mut change = Entry::note("edit_layers", State::Done, "2 changes");
    "Lift the Interface card with a soft shadow".clone_into(&mut change.text);
    if let Some(step) = &mut change.step {
        step.named = true;
    }
    let mut read = Entry::note("read_layers", State::Done, "3 layers");
    if let Some(step) = &mut read.step {
        step.times = 2;
    }
    vec![
        Entry::new(Role::User, "Make the middle card stand out a little more."),
        Entry::new(Role::Assistant, "I'll look at the cards first."),
        Entry::note("get_state", State::Done, ""),
        read,
        change,
        Entry::note("screenshot", State::Done, "1 layer"),
        Entry::new(
            Role::Assistant,
            "Done. The Interface card now has a soft shadow and a slightly larger title, so it reads as the featured one. The other two cards are unchanged.",
        ),
        Entry::new(Role::User, "Export it as a PNG to my desktop."),
        Entry::note("export_layers", State::Failed, "card.png"),
        Entry::new(Role::Problem, "The provider couldn't be reached: connection refused"),
    ]
}

/// What a run needs besides the conversation.
pub struct Ask {
    /// What the user wrote, as the assistant reads it (a pasted layer as its id and name).
    pub text: String,
    /// What the user wrote, as the panel shows it (a pasted layer as its name).
    pub shown: String,
    /// The files attached to the message.
    pub files: Vec<files::Attachment>,
    /// The app's note on the chat's document and the selection.
    pub context: String,
    /// The chat's id in the app: its tool calls carry it, so they work on its own document.
    pub chat: u64,
    /// What answers.
    pub engine: Engine,
    /// Where tool calls go to be run on the main thread.
    pub tools: Sender<Call>,
    /// Gets the app's attention after an event or a tool call.
    pub wake: Arc<dyn Fn() + Send + Sync>,
    /// The app's local server (port and secret), which Claude Code reaches the tools through.
    pub server: Option<(u16, String)>,
    /// Whether a finished design is shown to a reviewer and sent back with what it wants changed.
    pub review: bool,
}

/// A run in progress.
struct Run {
    events: Receiver<Event>,
    stop: Arc<AtomicBool>,
    /// The Claude Code program while it runs, so stopping can end it.
    child: Arc<Mutex<Option<Child>>>,
}

/// The conversation and its run.
#[derive(Default)]
pub struct Chat {
    /// What the panel shows, oldest first.
    pub entries: Vec<Entry>,
    run: Option<Run>,
    /// The model's next words start a new entry.
    fresh: bool,
    /// The Claude Code session the conversation is in, if that engine has answered.
    session: Option<String>,
    /// The API conversation, if that engine has answered.
    history: Vec<Value>,
}

impl Chat {
    /// Whether a message is being answered.
    #[must_use]
    pub fn running(&self) -> bool {
        self.run.is_some()
    }

    /// Forgets the conversation and ends its run.
    pub fn clear(&mut self) {
        self.stop();
        *self = Chat::default();
    }

    /// Takes the user's message and answers it with why nothing can answer.
    pub fn refuse(&mut self, text: String, why: &str) {
        self.entries.push(Entry::new(Role::User, text));
        self.entries.push(Entry::new(Role::Problem, why));
    }

    /// Ends the run, if there is one.
    // Human: stopping is immediate for the user; the engine's thread notices on its own and winds up.
    pub fn stop(&mut self) {
        let Some(run) = self.run.take() else { return };
        run.stop.store(true, Ordering::Relaxed);
        if let Some(mut child) = lock(&run.child).take() {
            // A program that has already ended can't be ended again; either way it is gone.
            let _ = child.kill();
            let _ = child.wait();
        }
        self.settle();
        self.entries.push(Entry::new(Role::Step, "Stopped"));
    }

    /// What earlier turns an engine that wasn't there for them is told, with the app's note.
    fn retold(&self, context: &str, known: bool) -> String {
        let said: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| matches!(entry.role, Role::User | Role::Assistant))
            .map(|entry| format!("{}: {}", if entry.role == Role::User { "User" } else { "Assistant" }, entry.text))
            .collect();
        if known || said.is_empty() {
            return context.to_owned();
        }
        let from = said.len().saturating_sub(RETOLD);
        format!("{context}\n<earlier_conversation>\n{}\n</earlier_conversation>", said[from..].join("\n"))
    }

    /// Sends the user's message and starts the run that answers it.
    // Human: a conversation can move from one engine to the other; the engine that takes over is told what
    // was said so far, since each keeps its own record.
    // Agent: SPAWNS the run's thread; it ends by itself, or soon after `stop`.
    pub fn send(&mut self, ask: Ask) {
        if self.running() {
            return;
        }
        let known = match &ask.engine {
            Engine::Code { .. } => self.session.is_some(),
            Engine::Api { .. } => !self.history.is_empty(),
        };
        // Human: attached text files travel inside the message, pictures beside it.
        // Human: a conversation's first message also carries three starting points for a design's look.
        // Human: a request that names what to follow has its look; worlds to choose from would pull it away.
        let dealt = if self.entries.is_empty() && !crate::craft::names_a_reference(&ask.text) {
            let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs());
            format!("{}\n", crate::craft::directions(seed))
        } else {
            String::new()
        };
        // Human: any message that names a phone or a tablet, not only the first, is reminded that a screen for
        // it is shown in the device; left alone the model draws the screen bare.
        let device = if crate::craft::names_a_device(&ask.text) { format!("{}\n", crate::craft::DEVICE_NOTE) } else { String::new() };
        let prompt = files::written(&format!("{}\n{dealt}{device}\n{}", self.retold(&ask.context, known), ask.text), &ask.files);
        let pictures = files::pictures(&ask.files);
        self.entries.push(Entry::new(Role::User, ask.shown.clone()));
        self.fresh = true;
        let (send, events) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let child = Arc::new(Mutex::new(None));
        let kept = Mutex::new((self.session.clone(), self.history.clone()));
        let (flag, held) = (stop.clone(), child.clone());
        let started = std::thread::Builder::new().name("chat".to_owned()).spawn(move || {
            let tell = |event: Event| {
                // Human: the run remembers where the conversation is, so a review's follow-up continues it.
                match &event {
                    Event::Session(session) => lock(&kept).0 = Some(session.clone()),
                    Event::History(history) => lock(&kept).1.clone_from(history),
                    _ => {}
                }
                // The chat was cleared or stopped: nobody listens, and the run winds up.
                let heard = send.send(event).is_ok();
                (ask.wake)();
                heard
            };
            let answer = |prompt: &str, pictures: &[Value]| {
                let (session, history) = lock(&kept).clone();
                match &ask.engine {
                    Engine::Code { program, model } => {
                        code::run(
                            &code::Job { program, model, prompt, pictures, session: session.as_deref(), server: ask.server.as_ref(), chat: ask.chat },
                            &flag,
                            &held,
                            &tell,
                        );
                    }
                    Engine::Api { endpoint, official, model, key } => {
                        api::run(
                            &api::Job {
                                endpoint,
                                official: *official,
                                model,
                                key,
                                prompt,
                                pictures,
                                history,
                                tools: &ask.tools,
                                wake: &*ask.wake,
                                chat: ask.chat,
                            },
                            &flag,
                            &tell,
                        );
                    }
                }
            };
            answer(&prompt, &pictures);
            // Human: a model finds its own design good. So what it made is shown to a reviewer that didn't
            // make it, and what the reviewer wants changed goes back to the assistant, twice at most.
            // After the reviews of the design comes one proofreading of its execution, when the last review
            // still asked for changes.
            for round in 0..if ask.review { critic::ROUNDS + 1 } else { 0 } {
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                let Some(picture) = critic::picture(&ask.tools, &*ask.wake, ask.chat, round > 0) else { break };
                let id = format!("review-{round}");
                tell(Event::Break);
                tell(Event::Tool { id: id.clone(), name: critic::TOOL.to_owned() });
                let verdict = critic::judge(&ask.engine, &ask.text, picture, round == critic::ROUNDS, &held);
                if let Err(error) = &verdict {
                    log::warn!("the design couldn't be reviewed: {error}");
                }
                tell(Event::ToolEnd { id, failed: verdict.is_err() });
                let Ok(Some(changes)) = verdict else { break };
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                tell(Event::Break);
                answer(&critic::follow_up(&changes), &[]);
            }
            tell(Event::Done);
        });
        match started {
            Ok(_) => self.run = Some(Run { events, stop, child }),
            Err(error) => self.entries.push(Entry::new(Role::Problem, format!("The assistant couldn't be started: {error}"))),
        }
    }

    /// Takes in what the run has said since the last time. Returns whether anything changed.
    pub fn poll(&mut self) -> bool {
        let Some(run) = &self.run else { return false };
        let mut events = Vec::new();
        let mut over = false;
        loop {
            match run.events.try_recv() {
                Ok(Event::Done) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    over = true;
                    break;
                }
                Ok(event) => events.push(event),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
            }
        }
        let changed = over || !events.is_empty();
        for event in events {
            match event {
                Event::Text(more) => match self.entries.last_mut() {
                    Some(last) if last.role == Role::Assistant && !self.fresh => last.text.push_str(&more),
                    _ if more.trim().is_empty() => {}
                    _ => {
                        self.entries.push(Entry::new(Role::Assistant, more.trim_start()));
                        self.fresh = false;
                    }
                },
                Event::Break => self.fresh = true,
                Event::Tool { id, name } => {
                    self.fresh = true;
                    let mut note = Entry::note(steps::short(&name), State::Running, "");
                    if let Some(step) = &mut note.step {
                        step.id = id;
                    }
                    self.entries.push(note);
                }
                Event::ToolInput { id, input } => self.describe(&id, &input),
                Event::ToolEnd { id, failed } => self.finish(&id, failed),
                Event::Session(session) => self.session = Some(session),
                Event::History(history) => self.history = history,
                Event::Problem(text) => {
                    self.fresh = true;
                    self.entries.push(Entry::new(Role::Problem, text));
                }
                Event::Done => {}
            }
        }
        if over {
            self.run = None;
            self.settle();
        }
        changed
    }

    /// No call is at work any more: the run is over or was stopped.
    fn settle(&mut self) {
        for step in self.entries.iter_mut().filter_map(|entry| entry.step.as_mut()) {
            if step.state == State::Running {
                step.state = State::Done;
            }
        }
    }

    /// The note of the call `id`, if it has one.
    fn note_of(&mut self, id: &str) -> Option<usize> {
        self.entries.iter().rposition(|entry| entry.step.as_ref().is_some_and(|step| step.id == id))
    }

    /// Takes in what the call `id` asks its tool for: the note says more, or joins the note before it.
    // Human: a change of the design is told in the model's own few words. A call with nothing to add that
    // follows the same kind of call (reading the guide twice, say) is told by that one note, with a count.
    fn describe(&mut self, id: &str, input: &Value) {
        let Some(at) = self.note_of(id) else { return };
        let Some(tool) = self.entries[at].step.as_ref().map(|step| step.tool.clone()) else { return };
        let (name, detail) = (steps::name(&tool, input), steps::detail(&tool, input));
        let same = |before: &Entry| {
            before.step.as_ref().is_some_and(|step| step.tool == tool && !step.named && step.state != State::Failed && step.detail == detail)
        };
        if name.is_none() && at + 1 == self.entries.len() && at > 0 && same(&self.entries[at - 1]) {
            self.entries.pop();
            if let Some(step) = self.entries.last_mut().and_then(|entry| entry.step.as_mut()) {
                step.times += 1;
                step.state = State::Running;
                id.clone_into(&mut step.id);
            }
            return;
        }
        let entry = &mut self.entries[at];
        if let Some(step) = &mut entry.step {
            step.detail = detail;
            step.named = name.is_some();
        }
        if let Some(name) = name {
            entry.text = name;
        }
    }

    /// The call `id` is over, well or with an error.
    fn finish(&mut self, id: &str, failed: bool) {
        let Some(entry) = self.note_of(id).and_then(|at| self.entries.get_mut(at)) else { return };
        let Some(step) = &mut entry.step else { return };
        step.state = if failed { State::Failed } else { State::Done };
        if failed {
            // Human: what the model wanted to do stays readable beside the tool's words for a failure.
            if step.named {
                step.detail = std::mem::take(&mut entry.text);
                step.named = false;
            }
            entry.text = steps::failed(&step.tool);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::{CustomProvider, ProviderModel};

    fn with_events(events: Vec<Event>) -> Chat {
        let (send, receive) = channel();
        for event in events {
            send.send(event).unwrap();
        }
        let run = Run { events: receive, stop: Arc::new(AtomicBool::new(false)), child: Arc::new(Mutex::new(None)) };
        Chat { entries: vec![Entry::new(Role::User, "Make a card")], run: Some(run), fresh: true, ..Chat::default() }
    }

    #[test]
    fn a_run_becomes_lines_of_the_conversation() {
        let mut chat = with_events(vec![
            Event::Session("s1".to_owned()),
            Event::Text("I'll look ".to_owned()),
            Event::Text("first.".to_owned()),
            Event::Tool { id: "t1".to_owned(), name: "mcp__pencraft__get_state".to_owned() },
            Event::ToolInput { id: "t1".to_owned(), input: serde_json::json!({}) },
            Event::ToolEnd { id: "t1".to_owned(), failed: false },
            Event::Tool { id: "t2".to_owned(), name: "get_state".to_owned() },
            Event::ToolInput { id: "t2".to_owned(), input: serde_json::json!({}) },
            Event::ToolEnd { id: "t2".to_owned(), failed: false },
            Event::Tool { id: "t3".to_owned(), name: "read_layers".to_owned() },
            Event::ToolInput { id: "t3".to_owned(), input: serde_json::json!({ "ids": ["a", "b"] }) },
            Event::ToolEnd { id: "t3".to_owned(), failed: true },
            Event::Tool { id: "t4".to_owned(), name: "edit_layers".to_owned() },
            Event::ToolInput { id: "t4".to_owned(), input: serde_json::json!({ "description": "add a card", "operations": [{}, {}, {}] }) },
            Event::Break,
            Event::Text("\n\nDone: a card.".to_owned()),
            Event::Problem("The picture failed.".to_owned()),
            Event::Done,
        ]);
        assert!(chat.running() && chat.poll() && !chat.running() && !chat.poll());
        let lines: Vec<(Role, &str)> = chat.entries.iter().map(|e| (e.role, e.text.as_str())).collect();
        assert_eq!(
            lines,
            vec![
                (Role::User, "Make a card"),
                (Role::Assistant, "I'll look first."),
                (Role::Step, "Looked at the document"),
                (Role::Step, "Couldn't read the layers"),
                (Role::Step, "Add a card"),
                (Role::Assistant, "Done: a card."),
                (Role::Problem, "The picture failed."),
            ]
        );
        // The two looks are one note; the failed read says so; the change that never answered is settled.
        let notes: Vec<(u32, State, &str, bool)> =
            chat.entries.iter().filter_map(|e| e.step.as_ref()).map(|s| (s.times, s.state, s.detail.as_str(), s.named)).collect();
        assert_eq!(notes, vec![(2, State::Done, "", false), (1, State::Failed, "2 layers", false), (1, State::Done, "3 changes", true)]);
        assert_eq!(chat.session.as_deref(), Some("s1"));
        // An engine that knows the conversation gets only the app's note; a new one is told what was said.
        assert_eq!(chat.retold("note", true), "note");
        assert!(chat.retold("note", false).contains("User: Make a card\nAssistant: I'll look first.\nAssistant: Done: a card."));
        chat.clear();
        assert!(chat.entries.is_empty() && chat.session.is_none());
    }

    #[test]
    fn stopping_ends_the_run_and_says_so() {
        let mut chat = with_events(vec![Event::Text("Working".to_owned())]);
        chat.stop();
        assert!(!chat.running());
        assert_eq!(chat.entries.last().map(|e| (e.role, e.text.as_str())), Some((Role::Step, "Stopped")));
    }

    #[test]
    fn the_assistants_offered_follow_what_is_installed_and_set_up() {
        let mut agents = Agents::default();
        assert_eq!(choices(&agents, None, false), Vec::new());
        let model = |id: &str| ProviderModel { id: id.to_owned(), ..ProviderModel::default() };
        agents.providers = vec![
            CustomProvider {
                name: "Mine".to_owned(),
                api: MESSAGES_API.to_owned(),
                base_url: "https://llm.example.com/v1/".to_owned(),
                models: vec![model("claude-opus-5-5"), model("other")],
                key_ref: Some("k".to_owned()),
            },
            CustomProvider {
                name: "No key".to_owned(),
                api: MESSAGES_API.to_owned(),
                base_url: "https://x.example.com".to_owned(),
                models: vec![model("m")],
                key_ref: None,
            },
            CustomProvider {
                name: "Other API".to_owned(),
                api: "OpenAI Completions".to_owned(),
                base_url: "https://y.example.com".to_owned(),
                models: vec![model("m")],
                key_ref: Some("k2".to_owned()),
            },
        ];
        let all = choices(&agents, Some(Path::new("/bin/claude")), true);
        let labels: Vec<&str> = all.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels.len(), 10);
        assert_eq!(
            (labels[0], labels[4], labels[8], labels[9]),
            ("Claude Code · Opus 5.5", "Anthropic API · Opus 5.5", "Mine · Opus 5.5", "Mine · other")
        );
        assert_eq!(
            all[8].engine,
            Engine::Api {
                endpoint: "https://llm.example.com/v1/messages".to_owned(),
                official: false,
                model: "claude-opus-5-5".to_owned(),
                key: KeySource::Keychain("k".to_owned())
            }
        );
        assert_eq!(endpoint("https://api.anthropic.com"), "https://api.anthropic.com/v1/messages");
        // What was chosen is found again; anything else falls back to the first.
        assert_eq!(chosen(&all, "api/Mine/other").map(|c| c.label.as_str()), Some("Mine · other"));
        assert_eq!(chosen(&all, "gone").map(|c| c.id.as_str()), Some("code/claude-opus-5-5"));
    }
}
