//! The chat with an AI model inside the app: what was said, and the run that answers the user's last message.
//!
//! A message is answered by one of these engines, on a thread of its own: an agent program installed on this
//! computer (Claude Code or Grok in [`code`], any other in [`plain`]), or a provider's API called directly with
//! the user's key ([`api`] for the Messages API, [`openai`] for Chat Completions). Either way the
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
use crate::prefs::{AgentName, Agents, CustomProvider, LocalAgent};

pub mod api;
pub mod asks;
pub mod catalog;
pub mod code;
pub mod critic;
pub mod files;
pub mod history;
pub mod openai;
pub mod plain;
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
/// The API types of a provider, as the settings window names them, that the chat can talk to.
pub const MESSAGES_API: &str = "Anthropic Messages";
/// See [`MESSAGES_API`].
pub const COMPLETIONS_API: &str = "OpenAI Completions";
/// The kinds of command-line agent, as the settings window names them: Anthropic's Claude Code, xAI's Grok,
/// and any other program that takes a prompt and prints its answer.
pub const KIND_CLAUDE: &str = "Claude Code";
/// See [`KIND_CLAUDE`].
pub const KIND_GROK: &str = "Grok";
/// See [`KIND_CLAUDE`].
pub const KIND_PLAIN: &str = "Other (prints its answer)";
/// Every kind, in the order the settings window offers them.
pub const KINDS: [&str; 3] = [KIND_CLAUDE, KIND_GROK, KIND_PLAIN];
/// How many earlier messages are retold to an engine that wasn't there for them.
const RETOLD: usize = 24;

/// What the model is told about its job, whichever engine runs it.
pub const BRIEF: &str = "You are the design assistant built into Pencraft, a canvas design editor. The user talks to you in the app's chat panel and watches the canvas while you work. \
Work on the user's open document through the Pencraft tools: call get_state first, read layers before you change them, make changes with edit_layers (one meaningful step per call, with a short description), and look at the result with screenshot before you say it is done. \
When edit_layers answers with advice, or a screenshot comes with a review, act on it. \
When you design something with a look of its own (a screen, a page, a poster, a card), work as \"Designing well\" below says: decide the direction for this subject, set colours, type and spacing as variables first, build with a clear hierarchy, then judge your screenshot as a strict art director would and improve the weakest parts before you answer. Never fall back on your defaults (Inter or Arial, a violet gradient, three equal cards, everything centred). \
A logo, a mark, a wordmark, a monogram, an emblem or an app icon is not a page and is made another way: read the logos skill with read_skill before the first layer and work as it says (one idea, three rough marks in black, one chosen and constructed exactly, proven small and in one ink), as \"A logo or a mark\" below says. \
When the user names a library, a product or a style to follow, or asks for standard or plain components, that reference is the brief instead: keep its face, palette and measures and add no direction of your own. \
Before the first layer, note for yourself what the user named, piece by piece; before you answer, check each piece is there and that you added nothing that was not asked for, and say so if you left something out or added something. \
Reuse the document's variables and components instead of making new ones for the same thing. Make what was asked for and nothing around it: a thing the user asks for (a dog, a logo, a button) gets no backdrop, board or coloured frame behind it unless the user asks for one or the thing itself is one, like a screen, a page or a card; keep its parts together in a group or a frame without a fill. \
A mock of a phone or tablet app is the exception the other way: its screen always stands inside the device it is for (the body, the screen's corners, the status bar, the Dynamic Island or camera, the home indicator), as \"A screen on a device\" below says, unless the user asks for the screen alone. \
A layer id followed by its name in quotes and brackets inside a message, like card2 (\"Card\"), is a layer the user pasted into the message to point at it: work on exactly those layers. \
For real facts or wording use search_web and read_web_page; for a real photo use search_images, then download_image, and put the url it returns into an image fill. \
Each message from the user starts with a note from the app in <pencraft_context> tags saying which document this chat works on (your tools work on it unless a call names another) and what is selected; \"this\" or \"the selection\" means those layers. \
Keep replies short and plain: a few sentences on what you made and what to look at, without lists of ids. The chat panel shows your words exactly as you write them, so use no Markdown of any kind: no asterisks for bold, no bullet or numbered lists, no headings, no backticks, no tables. \
If a request is unclear in a way that changes the design, ask instead of guessing: with the ask_user tool when you have it (one to three short questions, each with two to four answers to pick from), then end your turn, since the answer comes as the user's next message; without that tool, ask one short question in your reply. \
When you have finished something and there are clear next steps, you may offer two or three with ask_user's `suggestions`, each worded as the user would ask for it; offer none that only repeat what you did.";

/// The last thing an engine is told, after everything long: how its replies have to look.
// Human: said once at the start, the rule is forgotten by the time a long design is done; the end of the
// brief is what a model has freshest when it writes its answer.
const PLAIN: &str = "One more time, because it is easy to forget after a long piece of work: your reply to the user is a few plain sentences. No Markdown: no asterisks, no lists, no headings, no backticks.";

/// The brief with how to design and which fonts are installed: what an engine is told on top of what it is.
/// `instructions` is what the user wants kept to in every conversation (the settings' custom instructions).
#[must_use]
pub fn brief(guide: bool, instructions: &str) -> String {
    // Human: an engine that can't call get_guide by itself (the API) is given the document guide here.
    let guide = if guide { crate::agent::GUIDE } else { "" };
    let instructions = instructions.trim();
    let own =
        if instructions.is_empty() { String::new() } else { format!("The user's standing instructions for every conversation:\n{instructions}\n") };
    // Human: which skills there are is said on both routes; a model that never calls get_guide would not
    // learn of them otherwise.
    format!("{BRIEF}\n\n{}\n{}\n{}\n{guide}\n{own}{PLAIN}", crate::skills::index(), crate::craft::CRAFT, crate::craft::fonts())
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

/// A layer a message of the user's points at, as the panel shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pointed {
    /// Where its name stands in the message's text (in bytes): a layer pasted into the sentence. `None` for
    /// what was selected when the message was sent, which stands over the text.
    pub at: Option<std::ops::Range<usize>>,
    /// The icon of its kind, or a stack for several layers.
    pub icon: &'static str,
    /// Its name, or "3 layers".
    pub name: String,
}

/// One line of the conversation as the panel shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Who it is from.
    pub role: Role,
    /// What it says.
    pub text: String,
    /// For a message of the user's: the layers it points at, shown as chips.
    pub pointed: Vec<Pointed>,
    /// For a note on a tool call: the call it is about.
    pub step: Option<Step>,
    /// For the note of an `ask_user` call: what it asks the user, and how far the user has got with it.
    pub asked: Option<asks::Asked>,
}

impl Entry {
    /// A line from `role` saying `text`.
    pub fn new(role: Role, text: impl Into<String>) -> Self {
        Entry { role, text: text.into(), pointed: Vec::new(), step: None, asked: None }
    }

    /// A note on a call of `tool` that is in `state`, saying the tool's usual words and `detail`.
    fn note(tool: &str, state: State, detail: &str) -> Self {
        let text = if state == State::Failed { steps::failed(tool) } else { steps::did(tool) };
        let step = Step { tool: tool.to_owned(), id: String::new(), detail: detail.to_owned(), state, times: 1, named: false };
        Entry { role: Role::Step, text, pointed: Vec::new(), step: Some(step), asked: None }
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
    /// The model was given this message of the user's, written while it worked.
    Heard(Waiting),
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
    /// The provider takes requests without a key (a model server on this computer).
    None,
}

/// The shape of an API's requests and answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Wire {
    /// Anthropic's Messages API.
    #[default]
    Messages,
    /// The Chat Completions API `OpenAI` introduced.
    Completions,
}

impl Wire {
    /// The wire a provider with the API type `api` speaks, if the chat knows it.
    #[must_use]
    pub fn of(api: &str) -> Option<Wire> {
        match api {
            MESSAGES_API => Some(Wire::Messages),
            COMPLETIONS_API => Some(Wire::Completions),
            _ => None,
        }
    }
}

/// What answers a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Engine {
    /// Claude Code or Grok at `program`, with the user's own sign-in.
    Code {
        /// Which of the two it is.
        flavor: code::Flavor,
        /// Where the program is.
        program: PathBuf,
        /// The model it is asked to use; empty for the program's own default.
        model: String,
    },
    /// Any other command-line agent, which takes a prompt and prints its answer.
    Plain {
        /// Where the program is.
        program: PathBuf,
        /// Its arguments, with `{prompt}` and `{model}` to fill in.
        arguments: String,
        /// The model it is asked to use; empty for the program's own default.
        model: String,
    },
    /// A provider's API, called directly.
    Api {
        /// The shape of its requests.
        wire: Wire,
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
    /// What the menu and the message box show: the user's name for it, or `plain`.
    pub label: String,
    /// Who provides it and which model it is, as the app names it.
    pub plain: String,
    /// What answers when it is chosen.
    pub engine: Engine,
}

impl Choice {
    /// The model's name alone, without who provides it: what its cursor on the canvas can be labelled with.
    #[must_use]
    pub fn model_name(&self) -> &str {
        self.label.rsplit(" · ").next().unwrap_or(&self.label)
    }
}

/// Gives every assistant in `all` the name the user chose for it, if any.
pub fn rename(all: &mut [Choice], names: &[AgentName]) {
    for choice in all {
        let own = names.iter().find(|name| name.id == choice.id).map(|name| name.name.trim()).filter(|name| !name.is_empty());
        choice.label = own.unwrap_or(&choice.plain).to_owned();
    }
}

/// How the chat reaches `provider`: its wire and where its key is. `None` when it can't: an API the chat
/// doesn't speak, or the Messages API without a key.
// Human: a Chat Completions provider may have no key: model servers on one's own computer take none.
fn reach(provider: &CustomProvider) -> Option<(Wire, KeySource)> {
    let wire = Wire::of(&provider.api)?;
    match (&provider.key_ref, wire) {
        (Some(name), _) => Some((wire, KeySource::Keychain(name.clone()))),
        (None, Wire::Completions) => Some((wire, KeySource::None)),
        (None, Wire::Messages) => None,
    }
}

/// Whether the chat can talk to `provider`.
#[must_use]
pub fn usable(provider: &CustomProvider) -> bool {
    reach(provider).is_some() && !provider.models.is_empty()
}

/// A message of the user's that hasn't gone to an assistant yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Waiting {
    /// As the panel shows it.
    pub shown: String,
    /// As the assistant reads it.
    pub told: String,
    /// The files attached to it.
    pub files: Vec<PathBuf>,
    /// The layers it points at, as the panel shows them.
    pub pointed: Vec<Pointed>,
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

/// The command a program of `kind` is usually installed as; `None` for a kind that has no usual one.
#[must_use]
pub fn usual_command(kind: &str) -> Option<&'static str> {
    match kind {
        KIND_CLAUDE => Some("claude"),
        KIND_GROK => Some("grok"),
        _ => None,
    }
}

/// The command `agent` is looked for as: what the user wrote, or its kind's usual command.
#[must_use]
pub fn command_of(agent: &LocalAgent) -> Option<&str> {
    Some(agent.program.trim()).filter(|program| !program.is_empty()).or_else(|| usual_command(&agent.kind))
}

/// The choices one command-line agent gives, with its program at `program`: one per model.
// Human: Claude Code keeps the ids it always had ("code/<model>"), so the assistant remembered in the
// settings is still found; every other program is known by its name there.
fn local_choices(agent: &LocalAgent, program: &Path) -> Vec<Choice> {
    let named: Vec<(String, String)> = agent.models.iter().map(|model| (model.clone(), model.clone())).collect();
    let models = match (named.is_empty(), agent.kind.as_str()) {
        (false, _) => named,
        (true, KIND_CLAUDE) => MODELS.iter().map(|(id, name)| ((*id).to_owned(), (*name).to_owned())).collect(),
        (true, _) => vec![(String::new(), String::new())],
    };
    models
        .into_iter()
        .map(|(model, shown)| {
            let plain = if shown.is_empty() { agent.name.clone() } else { format!("{} · {shown}", agent.name) };
            let id = match (agent.kind.as_str(), agent.name.as_str()) {
                (KIND_CLAUDE, KIND_CLAUDE) => format!("code/{model}"),
                _ => format!("cli/{}/{}", agent.name, if model.is_empty() { "default" } else { &model }),
            };
            let program = program.to_path_buf();
            let engine = match agent.kind.as_str() {
                KIND_CLAUDE => Engine::Code { flavor: code::Flavor::Claude, program, model },
                KIND_GROK => Engine::Code { flavor: code::Flavor::Grok, program, model },
                _ => Engine::Plain { program, arguments: agent.arguments.clone(), model },
            };
            Choice { id, label: plain.clone(), plain, engine }
        })
        .collect()
}

/// The assistants the user can pick from: every command-line agent of the settings whose program is installed
/// (`find` says where a command is), Anthropic's API when a key is in the environment (`key_in_environment`),
/// and every provider set up in the settings that speaks an API the chat knows.
#[must_use]
pub fn choices(agents: &Agents, find: &dyn Fn(&str) -> Option<PathBuf>, key_in_environment: bool) -> Vec<Choice> {
    // Human: with a key in the environment, `ANTHROPIC_BASE_URL` says where it belongs, as it does for
    // Anthropic's own tools; without it that is Anthropic's API.
    let base = std::env::var("ANTHROPIC_BASE_URL").ok().filter(|base| !base.trim().is_empty()).unwrap_or_else(|| ANTHROPIC.to_owned());
    let mut out = Vec::new();
    for agent in agents.local.iter().filter(|agent| !agent.off && !agent.name.trim().is_empty()) {
        if let Some(program) = command_of(agent).and_then(find) {
            out.extend(local_choices(agent, &program));
        }
    }
    if key_in_environment {
        out.extend(MODELS.map(|(model, name)| Choice {
            id: format!("api/anthropic/{model}"),
            label: format!("Anthropic API · {name}"),
            plain: format!("Anthropic API · {name}"),
            engine: Engine::Api {
                wire: Wire::Messages,
                endpoint: endpoint(&base),
                official: is_anthropic(&base),
                model: model.to_owned(),
                key: KeySource::Environment,
            },
        }));
    }
    for provider in &agents.providers {
        let Some((wire, key)) = reach(provider) else { continue };
        let (endpoint, official) = match wire {
            Wire::Messages => (endpoint(&provider.base_url), is_anthropic(&provider.base_url)),
            Wire::Completions => (openai::endpoint(&provider.base_url), false),
        };
        out.extend(provider.models.iter().map(|model| {
            let plain = format!("{} · {}", provider.name, MODELS.iter().find(|(id, _)| *id == model.id).map_or(model.id.as_str(), |(_, name)| name));
            Choice {
                id: format!("api/{}/{}", provider.name, model.id),
                label: plain.clone(),
                plain,
                engine: Engine::Api { wire, endpoint: endpoint.clone(), official, model: model.id.clone(), key: key.clone() },
            }
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
        // The user had the row of cards selected and pasted one card into the sentence.
        Entry {
            pointed: vec![
                Pointed { at: None, icon: "frame", name: "Cards".to_owned() },
                Pointed { at: Some(5..14), icon: "frame", name: "Interface".to_owned() },
            ],
            ..Entry::new(Role::User, "Make Interface stand out a little more.")
        },
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

/// A short exchange that ends with the assistant asking (`suggests`: offering next steps instead), for a
/// picture of the question in the chat panel and over the message bar (`--sidebar question`).
#[must_use]
pub fn sample_asking(suggests: bool) -> Vec<Entry> {
    let input = if suggests {
        serde_json::json!({ "suggestions": ["Add a dark version of the page", "Make the headline bolder", "Export the hero as a PNG"] })
    } else {
        serde_json::json!({ "questions": [
            { "question": "Which mood should the landing page have?", "options": ["Warm and editorial", "Dark and technical", "Bright and playful"] },
            { "question": "Is it for a phone or for a desktop browser?", "options": ["Phone", "Desktop"] },
        ] })
    };
    let mut note = Entry::note(asks::TOOL, State::Done, "");
    note.asked = asks::parse(&input).ok();
    let (user, said) = if suggests {
        ("Design a landing page for a small coffee roaster.", "Done. The page has a hero, three coffees and a footer.")
    } else {
        ("Design a landing page for a small coffee roaster.", "Two things decide how this looks.")
    };
    vec![Entry::new(Role::User, user), Entry::note("get_state", State::Done, ""), Entry::new(Role::Assistant, said), note]
}

/// The question or the suggestions among `entries` that wait for the user: the latest, unless it is closed.
#[must_use]
pub fn asking(entries: &[Entry]) -> Option<(usize, &asks::Asked)> {
    entries.iter().enumerate().rev().find_map(|(at, entry)| entry.asked.as_ref().map(|asked| (at, asked))).filter(|(_, asked)| !asked.closed)
}

/// What a run needs besides the conversation.
pub struct Ask {
    /// What the user wrote, as the assistant reads it (a pasted layer as its id and name).
    pub text: String,
    /// What the user wrote, as the panel shows it (a pasted layer as its name).
    pub shown: String,
    /// The layers the message points at, which the panel shows as chips.
    pub pointed: Vec<Pointed>,
    /// The files attached to the message.
    pub files: Vec<files::Attachment>,
    /// The app's note on the chat's document and the selection.
    pub context: String,
    /// The chat's id in the app: its tool calls carry it, so they work on its own document.
    pub chat: u64,
    /// What answers.
    pub engine: Engine,
    /// What the user wants kept to in every conversation (the settings' custom instructions).
    pub instructions: String,
    /// The tools the user has turned off, which an API model isn't offered (Claude Code asks the app's
    /// server, which leaves them out itself).
    pub off: Vec<String>,
    /// The answering model's name, for its cursor on the canvas.
    pub model: String,
    /// Where tool calls go to be run on the main thread.
    pub tools: Sender<Call>,
    /// Gets the app's attention after an event or a tool call.
    pub wake: Arc<dyn Fn() + Send + Sync>,
    /// The app's local server (port and secret), which Claude Code reaches the tools through.
    pub server: Option<(u16, String)>,
    /// Whether a finished design is shown to a reviewer and sent back with what it wants changed.
    pub review: bool,
}

/// One message for an engine, with where the conversation is.
struct Said<'a> {
    /// The message, with the app's note before it.
    prompt: &'a str,
    /// The pictures attached to it, as content blocks.
    pictures: &'a [Value],
    /// The session to continue, with a program that keeps one.
    session: Option<String>,
    /// The conversation so far, as an API takes it.
    history: Vec<Value>,
    /// Where messages the user sends meanwhile wait for an API run.
    inbox: &'a api::Inbox,
}

/// Has the engine of `ask` answer `said`. Returns when the engine is done.
// Agent: BLOCKS for the whole answer; runs on the chat's thread.
fn answer(ask: &Ask, said: Said<'_>, flag: &AtomicBool, held: &Mutex<Option<Child>>, tell: &dyn Fn(Event) -> bool) {
    match &ask.engine {
        Engine::Plain { program, arguments, model } => {
            if !said.pictures.is_empty() {
                tell(Event::Problem("This program is given the message without its pictures.".to_owned()));
            }
            plain::run(&plain::Job { program, arguments, model, instructions: &ask.instructions, prompt: said.prompt }, flag, held, tell);
        }
        Engine::Code { flavor, program, model } => {
            code::run(
                &code::Job {
                    flavor: *flavor,
                    program,
                    model,
                    instructions: &ask.instructions,
                    prompt: said.prompt,
                    pictures: said.pictures,
                    session: said.session.as_deref(),
                    server: ask.server.as_ref(),
                    chat: ask.chat,
                },
                flag,
                held,
                tell,
            );
        }
        Engine::Api { wire, endpoint, official, model, key } => {
            let run = match wire {
                Wire::Messages => api::run,
                Wire::Completions => openai::run,
            };
            run(
                &api::Job {
                    endpoint,
                    official: *official,
                    model,
                    key,
                    instructions: &ask.instructions,
                    off: &ask.off,
                    prompt: said.prompt,
                    pictures: said.pictures,
                    history: said.history,
                    tools: &ask.tools,
                    wake: &*ask.wake,
                    chat: ask.chat,
                    steering: said.inbox,
                },
                flag,
                tell,
            );
        }
    }
}

/// A run in progress.
struct Run {
    events: Receiver<Event>,
    stop: Arc<AtomicBool>,
    /// The Claude Code program while it runs, so stopping can end it.
    child: Arc<Mutex<Option<Child>>>,
    /// The answering model's name.
    model: String,
    /// Where a message for the run goes while it works; `None` for an engine that takes none (Claude Code
    /// reads its one message when it starts).
    steering: Option<Arc<api::Inbox>>,
}

/// The conversation and its run.
#[derive(Default)]
pub struct Chat {
    /// What the panel shows, oldest first.
    pub entries: Vec<Entry>,
    run: Option<Run>,
    /// The model's next words start a new entry.
    fresh: bool,
    /// The session the conversation is in with a program that keeps one (Claude Code, Grok), if it has answered.
    session: Option<String>,
    /// The program `session` belongs to.
    session_of: PathBuf,
    /// The API conversation, if that engine has answered.
    history: Vec<Value>,
    /// The API whose shape `history` has.
    wire: Wire,
    /// Messages written while the assistant worked, oldest first; each is sent when the run before it is over.
    pub waiting: Vec<Waiting>,
}

impl Chat {
    /// Whether a message is being answered.
    #[must_use]
    pub fn running(&self) -> bool {
        self.run.is_some()
    }

    /// The name of the model that is answering, while one is.
    #[must_use]
    pub fn answering(&self) -> Option<&str> {
        self.run.as_ref().map(|run| run.model.as_str())
    }

    /// Whether the assistant at work takes a further message while it works.
    #[must_use]
    pub fn steers(&self) -> bool {
        self.run.as_ref().is_some_and(|run| run.steering.is_some())
    }

    /// Hands the assistant at work a further message, which it reads at its next step.
    ///
    /// # Errors
    /// The message, when nothing is at work that takes one.
    // Human: the message shows greyed until the assistant has taken it; one the run ends without taking is
    // sent like any waiting message.
    pub fn steer(&mut self, message: Waiting, context: &str, files: &[files::Attachment]) -> Result<(), Waiting> {
        let Some(inbox) = self.run.as_ref().and_then(|run| run.steering.as_ref()) else { return Err(message) };
        let prompt = files::written(&format!("{context}\n\n{}", message.told), files);
        lock(inbox).push(api::Steer { message, prompt, pictures: files::pictures(files) });
        Ok(())
    }

    /// The messages that haven't gone to an assistant yet, as the panel shows them, each with whether the
    /// assistant at work takes it at its next step (otherwise it waits for the run to end).
    #[must_use]
    pub fn pending(&self) -> Vec<(String, bool)> {
        let steered =
            self.run.as_ref().and_then(|run| run.steering.as_ref()).map(|inbox| lock(inbox).iter().map(|s| s.message.shown.clone()).collect());
        let steered: Vec<String> = steered.unwrap_or_default();
        steered.into_iter().map(|text| (text, true)).chain(self.waiting.iter().map(|waiting| (waiting.shown.clone(), false))).collect()
    }

    /// Takes the user's `answer` to the question at `at` of what the assistant asks. With the last answer it
    /// gives the message to send to the assistant.
    pub fn answer(&mut self, at: usize, answer: &str) -> Option<Waiting> {
        let asked = self.entries.iter_mut().rev().find_map(|entry| entry.asked.as_mut())?;
        let (shown, told) = asked.answer(at, answer)?;
        Some(Waiting { shown, told, files: Vec::new(), pointed: Vec::new() })
    }

    /// Closes what the assistant asks: the user doesn't want to answer it.
    pub fn dismiss(&mut self) {
        for asked in self.entries.iter_mut().filter_map(|entry| entry.asked.as_mut()) {
            asked.closed = true;
        }
    }

    /// The user writes a message of their own instead of picking answers: what the assistant asks is closed.
    /// Returns the answers picked before that, as the assistant reads them, to go with the message.
    pub fn leave_question(&mut self) -> Option<String> {
        let asked = self.entries.iter_mut().rev().find_map(|entry| entry.asked.as_mut()).filter(|asked| !asked.closed)?;
        asked.closed = true;
        (!asked.suggests && !asked.answers.is_empty()).then(|| asked.told())
    }

    /// The waiting message whose turn it is, once nothing is being answered.
    pub fn next_waiting(&mut self) -> Option<Waiting> {
        (!self.running() && !self.waiting.is_empty()).then(|| self.waiting.remove(0))
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
    // Human: stopping is immediate for the user; the engine's thread notices on its own and winds up. Messages
    // that were waiting for the run are not sent after it: stopping would otherwise start the next answer.
    pub fn stop(&mut self) {
        let Some(run) = self.run.take() else { return };
        run.stop.store(true, Ordering::Relaxed);
        if let Some(mut child) = lock(&run.child).take() {
            // A program that has already ended can't be ended again; either way it is gone.
            let _ = child.kill();
            let _ = child.wait();
        }
        self.settle();
        let unsent = self.waiting.len() + run.steering.map_or(0, |inbox| lock(&inbox).len());
        self.waiting.clear();
        let text = match unsent {
            0 => "Stopped".to_owned(),
            1 => "Stopped; the waiting message wasn't sent".to_owned(),
            many => format!("Stopped; the {many} waiting messages weren't sent"),
        };
        self.entries.push(Entry::new(Role::Step, text));
    }

    /// What earlier turns an engine that wasn't there for them is told, with the app's note.
    fn retold(&self, context: &str, known: bool) -> String {
        let said: Vec<String> = self
            .entries
            .iter()
            .filter_map(|entry| match (entry.role, &entry.asked) {
                (Role::User, _) => Some(format!("User: {}", entry.text)),
                (Role::Assistant, _) => Some(format!("Assistant: {}", entry.text)),
                // Human: a question the assistant asked through its tool is part of what was said.
                (_, Some(asked)) if !asked.suggests => {
                    Some(format!("Assistant asked: {}", asked.questions.iter().map(|question| question.text.as_str()).collect::<Vec<_>>().join(" ")))
                }
                _ => None,
            })
            .collect();
        if known || said.is_empty() {
            return context.to_owned();
        }
        let from = said.len().saturating_sub(RETOLD);
        format!("{context}\n<earlier_conversation>\n{}\n</earlier_conversation>", said[from..].join("\n"))
    }

    /// Whether `engine` was there for the conversation so far. One that wasn't starts its own record of it.
    // Human: each API keeps the conversation in its own shape; moving to a provider with the other one starts
    // its record anew. Likewise a session belongs to the program that made it; another program starts its own.
    fn knows(&mut self, engine: &Engine) -> bool {
        if let Engine::Api { wire, .. } = engine
            && *wire != self.wire
        {
            self.wire = *wire;
            self.history.clear();
        }
        if let Engine::Code { program, .. } = engine
            && *program != self.session_of
        {
            self.session_of.clone_from(program);
            self.session = None;
        }
        match engine {
            Engine::Code { .. } => self.session.is_some(),
            Engine::Api { .. } => !self.history.is_empty(),
            Engine::Plain { .. } => false,
        }
    }

    /// The message of `ask` as its engine reads it: the app's note (with what was said so far, for an engine
    /// that wasn't there), starting points for a design's look, and the user's words with their attached text.
    fn prompt_for(&mut self, ask: &Ask) -> String {
        let known = self.knows(&ask.engine);
        // Human: attached text files travel inside the message, pictures beside it.
        // Human: a conversation's first message also carries three starting points for a design's look.
        // Human: a request that names what to follow has its look; worlds to choose from would pull it away.
        // Human: a logo has a method of its own; a ground and a display face dealt for a page would make a
        // poster of it.
        let logo = crate::craft::names_a_logo(&ask.text);
        let dealt = if self.entries.is_empty() && !logo && !crate::craft::names_a_reference(&ask.text) {
            let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs());
            format!("{}\n", crate::craft::directions(seed))
        } else {
            String::new()
        };
        // Human: any message that names a phone or a tablet, not only the first, is reminded that a screen for
        // it is shown in the device; left alone the model draws the screen bare.
        let device = if crate::craft::names_a_device(&ask.text) { format!("{}\n", crate::craft::DEVICE_NOTE) } else { String::new() };
        // Human: so is any message that asks for a logo: left alone the model draws the subject's most
        // obvious picture out of rectangles and hangs a ribbon under it.
        let logo = if logo { format!("{}\n", crate::craft::LOGO_NOTE) } else { String::new() };
        files::written(&format!("{}\n{dealt}{device}{logo}\n{}", self.retold(&ask.context, known), ask.text), &ask.files)
    }

    /// Sends the user's message and starts the run that answers it.
    // Human: a conversation can move from one engine to the other; the engine that takes over is told what
    // was said so far, since each keeps its own record.
    // Agent: SPAWNS the run's thread; it ends by itself, or soon after `stop`.
    pub fn send(&mut self, ask: Ask) {
        if self.running() {
            return;
        }
        let prompt = self.prompt_for(&ask);
        let pictures = files::pictures(&ask.files);
        // Human: a message of the user's is their answer; a question still open then is closed by it.
        self.dismiss();
        self.entries.push(Entry { pointed: ask.pointed.clone(), ..Entry::new(Role::User, ask.shown.clone()) });
        self.fresh = true;
        let (send, events) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let child = Arc::new(Mutex::new(None));
        let kept = Mutex::new((self.session.clone(), self.history.clone()));
        let (flag, held) = (stop.clone(), child.clone());
        // Human: the ids of the run's `ask_user` calls, and whether one of them asked a question: the run then
        // waits for the user's answer, and nothing is reviewed or sent back to the assistant meanwhile.
        let asking: Mutex<(Vec<String>, bool)> = Mutex::new((Vec::new(), false));
        let model = ask.model.clone();
        // Human: an API run asks the model again after every tool it used, and can hand it a new message of
        // the user's then; a program is given its one message when it starts.
        let steering = matches!(ask.engine, Engine::Api { .. }).then(|| Arc::new(api::Inbox::default()));
        let inbox = steering.clone().unwrap_or_default();
        let started = std::thread::Builder::new().name("chat".to_owned()).spawn(move || {
            let tell = |event: Event| {
                // Human: the run remembers where the conversation is, so a review's follow-up continues it.
                match &event {
                    Event::Session(session) => lock(&kept).0 = Some(session.clone()),
                    Event::History(history) => lock(&kept).1.clone_from(history),
                    Event::Tool { id, name } if steps::short(name) == asks::TOOL => lock(&asking).0.push(id.clone()),
                    Event::ToolInput { id, input } => {
                        let mut asking = lock(&asking);
                        asking.1 |= asking.0.contains(id) && asks::parse(input).is_ok_and(|asked| !asked.suggests);
                    }
                    _ => {}
                }
                // The chat was cleared or stopped: nobody listens, and the run winds up.
                let heard = send.send(event).is_ok();
                (ask.wake)();
                heard
            };
            let answer = |prompt: &str, pictures: &[Value]| {
                let (session, history) = lock(&kept).clone();
                answer(&ask, Said { prompt, pictures, session, history, inbox: &inbox }, &flag, &held, &tell);
            };
            answer(&prompt, &pictures);
            // Human: a model finds its own design good. So what it made is shown to a reviewer that didn't
            // make it, and what the reviewer wants changed goes back to the assistant, twice at most.
            // After the reviews of the design comes one proofreading of its execution, when the last review
            // still asked for changes.
            // An engine the reviewer can't speak through gets no review.
            for round in 0..if ask.review && critic::can_judge(&ask.engine) { critic::ROUNDS + 1 } else { 0 } {
                if flag.load(Ordering::Relaxed) || lock(&asking).1 {
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
            Ok(_) => self.run = Some(Run { events, stop, child, model, steering }),
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
                Event::Heard(message) => {
                    self.fresh = true;
                    self.entries.push(Entry { pointed: message.pointed, ..Entry::new(Role::User, message.shown) });
                }
                Event::Problem(text) => {
                    self.fresh = true;
                    self.entries.push(Entry::new(Role::Problem, text));
                }
                Event::Done => {}
            }
        }
        if over {
            // Human: a message handed to the run that it ended without taking goes first among the waiting ones.
            if let Some(inbox) = self.run.take().and_then(|run| run.steering) {
                let unheard: Vec<Waiting> = std::mem::take(&mut *lock(&inbox)).into_iter().map(|steer| steer.message).collect();
                self.waiting.splice(0..0, unheard);
            }
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
        // Human: the note of a question is the question itself, with its answers to pick from; an earlier
        // question that is still open gives way to it.
        if tool == asks::TOOL {
            let Ok(asked) = asks::parse(input) else { return };
            self.dismiss();
            let entry = &mut self.entries[at];
            if asked.suggests { "Suggested next steps" } else { "Asked a question" }.clone_into(&mut entry.text);
            if let Some(step) = &mut entry.step {
                step.named = true;
            }
            entry.asked = Some(asked);
            return;
        }
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
            // A question the app refused is not put to the user.
            entry.asked = None;
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
        let run = Run {
            events: receive,
            stop: Arc::new(AtomicBool::new(false)),
            child: Arc::new(Mutex::new(None)),
            model: "Opus 5.5".to_owned(),
            steering: Some(Arc::new(api::Inbox::default())),
        };
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
    fn a_question_of_the_assistants_waits_for_the_users_answer() {
        let open = |chat: &Chat| asking(&chat.entries).map(|(_, asked)| asked.clone());
        let ask = |id: &str, input: Value| {
            vec![
                Event::Tool { id: id.to_owned(), name: "mcp__pencraft__ask_user".to_owned() },
                Event::ToolInput { id: id.to_owned(), input },
                Event::ToolEnd { id: id.to_owned(), failed: false },
            ]
        };
        let questions = serde_json::json!({ "questions": [{ "question": "Mood?", "options": ["Warm", "Dark"] }, { "question": "Screen?", "options": ["Phone"] }] });
        let mut events = ask("a1", serde_json::json!({ "suggestions": ["Export it"] }));
        events.extend(ask("a2", questions.clone()));
        events.extend([Event::Text("Two things first.".to_owned()), Event::Done]);
        let mut chat = with_events(events);
        assert!(open(&chat).is_none() && chat.poll());
        // The later call takes the place of the earlier one; its note is the question.
        let notes: Vec<(&str, bool)> = chat.entries.iter().filter_map(|e| e.asked.as_ref().map(|asked| (e.text.as_str(), asked.closed))).collect();
        assert_eq!(notes, vec![("Suggested next steps", true), ("Asked a question", false)]);
        assert_eq!(open(&chat).as_ref().and_then(asks::Asked::current).map(|(at, question)| (at, question.text.as_str())), Some((0, "Mood?")));
        assert!(chat.retold("note", false).contains("Assistant asked: Mood? Screen?\nAssistant: Two things first."));
        // The answers are taken one by one; the last makes the message for the assistant.
        assert_eq!(chat.answer(0, "Dark"), None);
        let message = chat.answer(1, "Phone").unwrap();
        assert_eq!((message.shown.as_str(), message.told.as_str()), ("Dark\nPhone", "Mood?\nAnswer: Dark\nScreen?\nAnswer: Phone"));
        assert!(open(&chat).is_none() && chat.answer(1, "Phone").is_none());
        // A message of the user's own closes the question and takes along what was picked before it.
        let mut events = ask("a3", questions);
        events.push(Event::Done);
        let mut chat = with_events(events);
        chat.poll();
        chat.answer(0, "Warm");
        assert_eq!(chat.leave_question().as_deref(), Some("Mood?\nAnswer: Warm"));
        assert!(open(&chat).is_none() && chat.leave_question().is_none());
        // Closing it asks nothing more; a question the app refused was never put.
        let mut chat = with_events(vec![
            Event::Tool { id: "a4".to_owned(), name: "ask_user".to_owned() },
            Event::ToolInput { id: "a4".to_owned(), input: serde_json::json!({ "questions": ["Why?"] }) },
        ]);
        chat.poll();
        assert!(open(&chat).is_some());
        chat.finish("a4", true);
        assert!(chat.entries.iter().all(|entry| entry.asked.is_none()));
        let mut asking = Chat { entries: sample_asking(false), ..Chat::default() };
        assert!(open(&asking).is_some() && open(&Chat { entries: sample_asking(true), ..Chat::default() }).is_some_and(|asked| asked.suggests));
        asking.dismiss();
        assert!(open(&asking).is_none());
    }

    #[test]
    fn stopping_ends_the_run_and_says_so() {
        let mut chat = with_events(vec![Event::Text("Working".to_owned())]);
        chat.stop();
        assert!(!chat.running());
        assert_eq!(chat.entries.last().map(|e| (e.role, e.text.as_str())), Some((Role::Step, "Stopped")));
    }

    #[test]
    fn a_message_written_meanwhile_is_steered_or_waits_its_turn() {
        let message = |text: &str| Waiting { shown: text.to_owned(), told: text.to_owned(), files: Vec::new(), pointed: Vec::new() };
        // Nothing at work: there is nobody to hand it to.
        assert_eq!(Chat::default().steer(message("a"), "note", &[]), Err(message("a")));
        // Human: the test holds the run's sending end, so the run goes on until the test ends it.
        let (send, receive) = channel();
        let mut chat = with_events(Vec::new());
        if let Some(run) = &mut chat.run {
            run.events = receive;
        }
        send.send(Event::Text("Working".to_owned())).unwrap();
        assert!(chat.steers() && chat.answering() == Some("Opus 5.5"));
        chat.steer(message("wider"), "note", &[]).unwrap();
        chat.waiting.push(message("then export"));
        assert_eq!(chat.pending(), vec![("wider".to_owned(), true), ("then export".to_owned(), false)]);
        // While the run goes on nothing is sent after it.
        assert!(chat.poll() && chat.running() && chat.next_waiting().is_none());
        let inbox = chat.run.as_ref().and_then(|run| run.steering.clone()).unwrap();
        assert_eq!(lock(&inbox)[0].prompt, "note\n\nwider");
        // The run takes one message and says so; another, handed in too late, goes first among the waiting.
        lock(&inbox).clear();
        chat.steer(message("too late"), "note", &[]).unwrap();
        let chip = Pointed { at: Some(0..5), icon: "frame", name: "wider".to_owned() };
        send.send(Event::Heard(Waiting { pointed: vec![chip.clone()], ..message("wider") })).unwrap();
        send.send(Event::Done).unwrap();
        assert!(chat.poll() && !chat.running());
        assert_eq!(chat.entries.last().map(|e| (e.role, e.text.as_str(), e.pointed.as_slice())), Some((Role::User, "wider", &[chip][..])));
        assert_eq!((chat.next_waiting(), chat.next_waiting(), chat.next_waiting()), (Some(message("too late")), Some(message("then export")), None));
        // Stopping sends none of them.
        let mut chat = with_events(Vec::new());
        chat.steer(message("a"), "note", &[]).unwrap();
        chat.waiting.push(message("b"));
        chat.stop();
        assert!(chat.waiting.is_empty() && chat.pending().is_empty());
        assert_eq!(chat.entries.last().map(|e| e.text.as_str()), Some("Stopped; the 2 waiting messages weren't sent"));
    }

    #[test]
    fn command_line_agents_are_offered_when_their_program_is_installed() {
        // Command-line agents: Grok with its own default model, another program with the models named for it,
        // one that is turned off, and one whose program isn't installed.
        let mut agents = Agents::default();
        agents.local.push(LocalAgent {
            name: "Codex".to_owned(),
            kind: KIND_PLAIN.to_owned(),
            program: "/opt/codex".to_owned(),
            models: vec!["gpt".to_owned()],
            arguments: "exec {prompt}".to_owned(),
            off: false,
        });
        agents.local.push(LocalAgent { name: "Second Grok".to_owned(), off: true, ..LocalAgent::known(KIND_GROK) });
        agents.local.push(LocalAgent {
            name: "Gone".to_owned(),
            kind: KIND_PLAIN.to_owned(),
            program: "nowhere".to_owned(),
            ..LocalAgent::default()
        });
        let found = |command: &str| ["claude", "grok", "/opt/codex"].contains(&command).then(|| PathBuf::from(command));
        let all = choices(&agents, &found, false);
        let ids: Vec<&str> = all.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids[..4], ["code/claude-opus-5-5", "code/claude-fable-5-2", "code/claude-sonnet-5-5", "code/claude-haiku-5-5"]);
        assert_eq!(ids[4..], ["cli/Grok/default", "cli/Codex/gpt"]);
        assert_eq!((all[4].label.as_str(), all[5].label.as_str()), ("Grok", "Codex · gpt"));
        assert_eq!(all[4].engine, Engine::Code { flavor: code::Flavor::Grok, program: PathBuf::from("grok"), model: String::new() });
        assert_eq!(
            all[5].engine,
            Engine::Plain { program: PathBuf::from("/opt/codex"), arguments: "exec {prompt}".to_owned(), model: "gpt".to_owned() }
        );
        assert_eq!((command_of(&agents.local[0]), command_of(&agents.local[2])), (Some("claude"), Some("/opt/codex")));
    }

    #[test]
    fn the_assistants_offered_follow_what_is_installed_and_set_up() {
        let mut agents = Agents::default();
        agents.local.clear();
        assert_eq!(choices(&agents, &|_| None, true).len(), 4);
        let mut agents = Agents::default();
        assert_eq!(choices(&agents, &|_| None, false), Vec::new());
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
                api: "OpenAI Responses".to_owned(),
                base_url: "https://y.example.com".to_owned(),
                models: vec![model("m")],
                key_ref: Some("k2".to_owned()),
            },
            CustomProvider {
                name: "Local".to_owned(),
                api: COMPLETIONS_API.to_owned(),
                base_url: "http://localhost:11434/v1/".to_owned(),
                models: vec![model("llama")],
                key_ref: None,
            },
        ];
        assert_eq!(agents.providers.iter().map(usable).collect::<Vec<_>>(), [true, false, false, true]);
        let claude = |command: &str| (command == "claude").then(|| PathBuf::from("/bin/claude"));
        let all = choices(&agents, &claude, true);
        let labels: Vec<&str> = all.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels.len(), 11);
        assert_eq!(
            all[10].engine,
            Engine::Api {
                wire: Wire::Completions,
                endpoint: "http://localhost:11434/v1/chat/completions".to_owned(),
                official: false,
                model: "llama".to_owned(),
                key: KeySource::None
            }
        );
        assert_eq!(
            (labels[0], labels[4], labels[8], labels[9]),
            ("Claude Code · Opus 5.5", "Anthropic API · Opus 5.5", "Mine · Opus 5.5", "Mine · other")
        );
        assert_eq!(
            all[8].engine,
            Engine::Api {
                wire: Wire::Messages,
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
        assert_eq!((all[0].model_name(), all[9].model_name()), ("Opus 5.5", "other"));
        // The user's own name takes the place of the app's, in the menu and beside the cursor; none gives it back.
        let mut all = all;
        let own = |name: &str| vec![AgentName { id: "api/Mine/other".to_owned(), name: name.to_owned() }];
        rename(&mut all, &own(" Otto "));
        assert_eq!(
            (all[9].label.as_str(), all[9].model_name(), all[9].plain.as_str(), all[0].label.as_str()),
            ("Otto", "Otto", "Mine · other", "Claude Code · Opus 5.5")
        );
        rename(&mut all, &own("  "));
        assert_eq!(all[9].label, "Mine · other");
        // The user's standing instructions go into the brief, just before its last reminder; none adds nothing.
        assert!(!brief(false, "  ").contains("standing instructions"));
        let own = brief(false, " Use our brand colours. ");
        assert!(own.contains("every conversation:\nUse our brand colours.\n") && own.ends_with(PLAIN));
    }
}
