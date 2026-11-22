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
use std::time::{Duration, Instant};

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
pub mod ollama;
pub mod openai;
pub mod pasted;
pub mod plain;
pub mod prices;
pub mod steps;
pub mod tasks;
pub mod usage;

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
/// See [`MESSAGES_API`]: the API of an Ollama model server.
pub const OLLAMA_API: &str = "Ollama";
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
/// How long the model must have thought for the conversation to keep a note of it.
// Human: a thought of a second or two is no news; a note for each would bury the notes on what was done.
const THOUGHT_NOTED: Duration = Duration::from_secs(3);

/// What the model is told about its job, whichever engine runs it.
pub const BRIEF: &str = "You are the design assistant built into Quill, a canvas design editor. The user talks to you in the app's chat panel and watches the canvas while you work. \
Work on the user's open document through the Quill tools: call get_state first, read layers before you change them, make changes with edit_layers (one meaningful step per call, with a short description), and look at the result with screenshot before you say it is done. \
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
When you have finished something and there are clear next steps, you may offer two or three with ask_user's `suggestions`, each worded as the user would ask for it; offer none that only repeat what you did, and never one that means several of the others (\"Do both\", \"All of the above\"): the user can take all of them with one click. A suggestion is the user's to choose: never start on one yourself, and end your answer once you have offered them. \
A job of many steps (a whole screen or page, several screens, a redesign, a set of components, anything you would not finish in two or three changes) is worked through as a task list, and this is not optional: after get_state and before the first layer, call plan_tasks with the tasks in the order you will do them, each a few plain words the user can follow (\"Set up colours and type\", \"Build the navigation\", \"Check the result\"). Then work on the task at work only, and call finish_task with its number the moment it is done; the next task is at work then. Tasks can't be skipped or finished out of their turn, and you never work ahead of the task at work. When a task turns up more work (checking the result shows something to fix or change), add it with add_tasks before you finish that task: the new tasks come right after it. If the whole plan changes, call plan_tasks again with the tasks still to do. The user follows the list above the chat, so finish every task before you answer: never end your answer while a task is open, and if you do, you are sent back to the list. A small request (one layer, one change, a question) needs no list.";

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
    /// For a picture pasted into the sentence instead of layers: its file, shown small in the chip.
    pub picture: Option<PathBuf>,
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
    /// For a problem: what is known about the run it ended or troubled, each with its name ("Assistant").
    pub about: Vec<(&'static str, String)>,
    /// For the note of a task list that was worked through: its tasks, which the note opens to show.
    pub finished: Vec<String>,
    /// For the last line of a run: what the run took and cost.
    pub spent: Option<usage::Spent>,
}

impl Entry {
    /// A line from `role` saying `text`.
    pub fn new(role: Role, text: impl Into<String>) -> Self {
        Entry { role, text: text.into(), pointed: Vec::new(), step: None, asked: None, about: Vec::new(), finished: Vec::new(), spent: None }
    }

    /// A problem written out to be copied into a report: what went wrong, what is known about the run, and
    /// which Quill on which system it happened in.
    // Human: nothing of the conversation is in it, and no key: it can be pasted anywhere.
    #[must_use]
    pub fn report(&self) -> String {
        let app = format!("Quill {} on {} ({})", env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH);
        let facts = self.about.iter().map(|(name, fact)| (*name, fact.as_str())).chain([("App", app.as_str())]);
        std::iter::once(("Problem", self.text.as_str())).chain(facts).map(|(name, fact)| format!("{name}: {fact}")).collect::<Vec<_>>().join("\n")
    }

    /// A note on a call of `tool` that is in `state`, saying the tool's usual words and `detail`.
    fn note(tool: &str, state: State, detail: &str) -> Self {
        let text = if state == State::Failed { steps::failed(tool) } else { steps::did(tool) };
        let step = Step { tool: tool.to_owned(), id: String::new(), detail: detail.to_owned(), state, times: 1, named: false, why: String::new() };
        Entry { step: Some(step), ..Entry::new(Role::Step, text) }
    }
}

/// What a run tells the chat as it goes.
#[derive(Debug)]
pub enum Event {
    /// More of the model's words.
    Text(String),
    /// The model thinks before it writes or uses a tool: more of what it thinks, or nothing when its engine
    /// only says that it does.
    Thought(String),
    /// These words, the last ones told as the model's, were its thinking after all: they are no part of
    /// the answer.
    Unsaid(String),
    /// The model's next words start a new paragraph of the conversation.
    Break,
    /// The answer broke off before it was whole and the same request is sent once more: what the chat says
    /// about it. What the broken answer began (a tool call it was still writing) never happened.
    Again(String),
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
    /// What a request took (a program: what its whole run took).
    Used(usage::Report),
    /// What the request under way (a program: the run under way) has taken so far, before its report. Each
    /// one replaces the one before, and the report replaces them all.
    Meanwhile(usage::Used),
    /// Something went wrong; the run goes on or ends as it says next.
    Problem(String),
    /// The reviewer has asked for changes: the assistant goes back to work on the design.
    Revising,
    /// The run is over.
    Done,
}

/// What the model is doing while it neither writes nor has a tool at work, as far as the app can tell.
// Human: "Thinking…" for every wait says nothing. What the app does know is told: that nothing has come back
// yet, that the model thinks, or what it was just handed to go on with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pause {
    /// Nothing has come back yet: the message is on its way, or the model hasn't begun.
    Reaching,
    /// It thinks.
    Thinking,
    /// It was just handed the picture it took of the design.
    Looking,
    /// It was just handed what the reviewer wants changed.
    Reviewed,
    /// The tool it used last answered with an error.
    Retrying,
    /// It was just handed what a tool answered.
    Next,
    /// It is in the middle of its own words.
    Writing,
}

impl Pause {
    /// What the conversation `entries` waits on in a run: `reached` once the run has told anything, `thinking`
    /// while the engine says the model thinks.
    #[must_use]
    pub fn of(entries: &[Entry], reached: bool, thinking: bool) -> Pause {
        let last = entries.last();
        match last.and_then(|entry| entry.step.as_ref()) {
            _ if thinking => Pause::Thinking,
            Some(step) if step.state == State::Failed => Pause::Retrying,
            Some(step) if step.tool == "screenshot" => Pause::Looking,
            Some(step) if step.tool == critic::TOOL => Pause::Reviewed,
            Some(_) => Pause::Next,
            None if last.is_some_and(|entry| entry.role == Role::Assistant) => Pause::Writing,
            None if !reached => Pause::Reaching,
            None => Pause::Thinking,
        }
    }

    /// The wait in a few words, without the dots after them; `model` is the name of the model that answers.
    #[must_use]
    pub fn words(self, model: &str) -> String {
        match self {
            Pause::Reaching if model.is_empty() => "Waiting for the assistant".to_owned(),
            Pause::Reaching => format!("Waiting for {model}"),
            Pause::Thinking => "Thinking".to_owned(),
            Pause::Looking => "Looking at the picture".to_owned(),
            Pause::Reviewed => "Reading the review".to_owned(),
            Pause::Retrying => "Finding another way".to_owned(),
            Pause::Next => "Planning the next step".to_owned(),
            Pause::Writing => "Writing".to_owned(),
        }
    }
}

/// "8 s", "1 min 5 s": how long something took, in whole seconds.
#[must_use]
pub fn span(time: Duration) -> String {
    match time.as_secs() {
        seconds @ 0..60 => format!("{seconds} s"),
        seconds => format!("{} min {} s", seconds / 60, seconds % 60),
    }
}

/// The second `seconds` after the start of 1970 as a date and a time of day, in UTC: "2026-11-20 13:32:10 UTC".
// Human: a report is read by someone in another place; one clock for all saves asking which.
#[must_use]
pub fn stamp(seconds: u64) -> String {
    // The civil date of a day number, by Howard Hinnant's method: years are counted from March, in eras of 400.
    let (days, of_day) = (seconds / 86_400 + 719_468, seconds % 86_400);
    let (era, day_of_era) = (days / 146_097, days % 146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 { month_from_march + 3 } else { month_from_march - 9 };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC", of_day / 3_600, of_day % 3_600 / 60, of_day % 60)
}

/// What answers through `engine`, for a report of a problem: the kind of API and the provider's host, or the
/// program. Never the key, and of an address only the host.
fn via(engine: &Engine) -> String {
    let file = |program: &Path| program.file_name().map_or_else(|| program.display().to_string(), |name| name.to_string_lossy().into_owned());
    match engine {
        Engine::Code { flavor, .. } => flavor.name().to_owned(),
        Engine::Plain { program, .. } => format!("the program {}", file(program)),
        Engine::Api { wire, endpoint, .. } => {
            let api = match wire {
                Wire::Messages => MESSAGES_API,
                Wire::Completions => COMPLETIONS_API,
                Wire::Ollama => OLLAMA_API,
            };
            let host = endpoint.split_once("://").map_or(endpoint.as_str(), |(_, rest)| rest).split(['/', '?', '#']).next().unwrap_or_default();
            format!("{api} at {host}")
        }
    }
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
    /// Ollama's own chat API.
    Ollama,
}

impl Wire {
    /// The wire a provider with the API type `api` speaks, if the chat knows it.
    #[must_use]
    pub fn of(api: &str) -> Option<Wire> {
        match api {
            MESSAGES_API => Some(Wire::Messages),
            COMPLETIONS_API => Some(Wire::Completions),
            OLLAMA_API => Some(Wire::Ollama),
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
        /// How many tokens the model reads at most, as the settings have it; 0 when that isn't known.
        window: u32,
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
// Human: a Chat Completions or Ollama provider may have no key: model servers on one's own computer take none.
fn reach(provider: &CustomProvider) -> Option<(Wire, KeySource)> {
    let wire = Wire::of(&provider.api)?;
    match (&provider.key_ref, wire) {
        (Some(name), _) => Some((wire, KeySource::Keychain(name.clone()))),
        (None, Wire::Completions | Wire::Ollama) => Some((wire, KeySource::None)),
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
    /// The pictures pasted into its text, in the order they stand in it: each one's name and its file.
    pub pasted: Vec<(String, PathBuf)>,
    /// The layers it points at, as the panel shows them.
    pub pointed: Vec<Pointed>,
}

impl Waiting {
    /// Reads what travels with the message: the pictures pasted into it, then the attached files.
    ///
    /// # Errors
    /// A sentence for the user on the first one that can't be read.
    pub fn read(&self) -> Result<Vec<files::Attachment>, String> {
        let pasted = self.pasted.iter().map(|(name, path)| files::read_pasted(name, path));
        pasted.chain(self.files.iter().map(|path| files::read(path))).collect()
    }
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

/// Where a key in the environment belongs: `ANTHROPIC_BASE_URL`, as for Anthropic's own tools, or Anthropic's API.
fn environment_base() -> String {
    std::env::var("ANTHROPIC_BASE_URL").ok().filter(|base| !base.trim().is_empty()).unwrap_or_else(|| ANTHROPIC.to_owned())
}

/// The assistants the user can pick from: every command-line agent of the settings whose program is installed
/// (`find` says where a command is), Anthropic's API when a key is in the environment (`key_in_environment`),
/// and every provider set up in the settings that speaks an API the chat knows.
#[must_use]
pub fn choices(agents: &Agents, find: &dyn Fn(&str) -> Option<PathBuf>, key_in_environment: bool) -> Vec<Choice> {
    let base = environment_base();
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
                window: 0,
                key: KeySource::Environment,
            },
        }));
    }
    for provider in &agents.providers {
        let Some((wire, key)) = reach(provider) else { continue };
        let (endpoint, official) = match wire {
            Wire::Messages => (endpoint(&provider.base_url), is_anthropic(&provider.base_url)),
            Wire::Completions => (openai::endpoint(&provider.base_url), false),
            Wire::Ollama => (ollama::endpoint(&provider.base_url), false),
        };
        out.extend(provider.models.iter().map(|model| {
            let plain = format!("{} · {}", provider.name, MODELS.iter().find(|(id, _)| *id == model.id).map_or(model.id.as_str(), |(_, name)| name));
            Choice {
                id: format!("api/{}/{}", provider.name, model.id),
                label: plain.clone(),
                plain,
                engine: Engine::Api {
                    wire,
                    endpoint: endpoint.clone(),
                    official,
                    model: model.id.clone(),
                    window: model.context_window,
                    key: key.clone(),
                },
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

/// How what `choice` uses is priced: the user's own prices for its model, the price list's for its provider,
/// and how the provider is paid.
// Human: a program (Claude Code, Grok) says itself what a run cost, and doesn't say whether the user pays by
// the token or by plan; its cost is shown as a worth. A model on this computer costs nothing.
#[must_use]
pub fn pricing(choice: &Choice, agents: &Agents, list: &prices::Catalogue) -> usage::Pricing {
    match &choice.engine {
        Engine::Code { model, .. } => usage::Pricing { model: model.clone(), terms: usage::Terms::Plan, ..usage::Pricing::default() },
        Engine::Plain { model, .. } => usage::Pricing { model: model.clone(), ..usage::Pricing::default() },
        Engine::Api { model, .. } => {
            let provider = agents.providers.iter().find(|provider| choice.id == format!("api/{}/{model}", provider.name));
            let base = provider.map_or_else(environment_base, |provider| provider.base_url.clone());
            let typed = provider.and_then(|provider| provider.models.iter().find(|known| known.id == *model));
            let local = catalog::is_local_ollama(&base);
            usage::Pricing {
                model: model.clone(),
                own: typed.and_then(|typed| usage::Rates::typed(&typed.input_price, &typed.output_price)),
                listed: if agents.prices && !local { list.models(&base) } else { std::collections::BTreeMap::new() },
                terms: if local { usage::Terms::Local } else { prices::terms(&base) },
            }
        }
    }
}

/// The reports of requests among what a run has told and nobody has taken in; everything else is dropped.
fn reports_left(events: &Receiver<Event>) -> Vec<usage::Report> {
    let mut reports = Vec::new();
    while let Ok(event) = events.try_recv() {
        if let Event::Used(report) = event {
            reports.push(report);
        }
    }
    reports
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
                Pointed { at: None, icon: "frame", name: "Cards".to_owned(), picture: None },
                Pointed { at: Some(5..14), icon: "frame", name: "Interface".to_owned(), picture: None },
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
        Entry {
            about: vec![
                ("Assistant", "Opus 5.5 through Anthropic Messages at api.anthropic.com".to_owned()),
                ("While", "Exporting layers (card.png)".to_owned()),
                ("After", "12 s and 1 tool call".to_owned()),
                ("When", stamp(1_795_181_530)),
            ],
            ..Entry::new(Role::Problem, "The provider answered with an error (500): Internal Server Error (request req_011CZ4kPq)")
        },
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
    // The picture shows a question in the middle of being answered: two answers ticked.
    if let Some(asked) = note.asked.as_mut().filter(|asked| !asked.suggests) {
        asked.toggle(0, "Warm and editorial");
        asked.toggle(0, "Bright and playful");
    }
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
    /// How what the run uses is priced.
    pub pricing: usage::Pricing,
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
            // Human: a program that only takes text is told where the pictures lie as files; one that can
            // open files looks at them there.
            let prompt = if said.pictures.is_empty() { files::flat(said.prompt) } else { files::with_paths(said.prompt, &ask.files) };
            plain::run(&plain::Job { program, arguments, model, instructions: &ask.instructions, prompt: &prompt }, flag, held, tell);
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
        Engine::Api { wire, endpoint, official, model, window, key } => {
            let run = match wire {
                Wire::Messages => api::run,
                Wire::Completions => openai::run,
                Wire::Ollama => ollama::run,
            };
            run(
                &api::Job {
                    endpoint,
                    official: *official,
                    model,
                    window: u64::from(*window),
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
    /// What it answers through ([`via`]).
    via: String,
    /// When the run began.
    started: Instant,
    /// Where a message for the run goes while it works; `None` for an engine that takes none (Claude Code
    /// reads its one message when it starts).
    steering: Option<Arc<api::Inbox>>,
    /// How what the run uses is priced.
    pricing: usage::Pricing,
    /// How many lines the conversation had when the run began: its own lines come after.
    from: usize,
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
    /// What the model has thought since it last wrote or used a tool, as far as its engine lets it be read.
    thought: String,
    /// Since when the model thinks.
    thinking: Option<Instant>,
    /// When the run last wrote, used a tool or was handed something: the wait the panel counts starts there.
    quiet: Option<Instant>,
    /// The run has told something of the model's: the model was reached.
    reached: bool,
    /// The reasons tools gave for calls that failed and whose notes are not finished yet: the tool, and why.
    refused: Vec<(String, String)>,
    /// The next steps the assistant has offered that the user has neither taken nor dropped, newest first.
    offered: Vec<String>,
    /// The run under way has offered next steps: until it is over it changes nothing more.
    holds: bool,
    /// How often in a row the assistant was sent back to its unfinished task list, and which list that was
    /// with how many tasks finished ([`tasks::Tasks::made`]); a task finished since starts the count anew.
    urged: (u32, u64, usize),
    /// The task list of a long job, which the assistant makes and works through ([`tasks`]).
    pub tasks: tasks::Tasks,
    /// What the run under way has taken so far, by the reports of its finished requests.
    spent: usage::Spent,
    /// What the request under way has taken so far, as its engine counts along; nothing between requests.
    live: usage::Used,
    /// What the whole conversation has taken.
    pub total: usage::Spent,
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

    /// What the run waits on while the model neither writes nor has a tool at work; `None` without a run.
    #[must_use]
    pub fn pause(&self) -> Option<Pause> {
        self.running().then(|| Pause::of(&self.entries, self.reached, self.thinking.is_some()))
    }

    /// How long that wait has lasted.
    #[must_use]
    pub fn paused(&self) -> Duration {
        self.quiet.map_or(Duration::ZERO, |since| since.elapsed())
    }

    /// What the model has thought in that wait, where its engine lets it be read; empty otherwise.
    #[must_use]
    pub fn thought(&self) -> &str {
        &self.thought
    }

    /// Puts the chat in the middle of a made-up run in which the model thinks, for a picture of the panel
    /// (`--sidebar thinking`).
    // Human: the run's thread tells nothing and only keeps the run open until it is stopped.
    // Agent: SPAWNS a thread that sleeps until the chat is stopped or cleared.
    pub fn pose_thinking(&mut self) {
        let (send, events) = channel::<Event>();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let held = std::thread::Builder::new().name("chat-pose".to_owned()).spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
            }
            drop(send);
        });
        if held.is_err() {
            return;
        }
        self.entries = vec![
            Entry::new(
                Role::User,
                "Create a landingpage for my minecraft community server. It must have a hightly pixelated look while staying modern and AI Slop free",
            ),
            Entry::note("get_state", State::Done, ""),
        ];
        "The document is empty, so the page starts from nothing. A dark ground with one loud green would read as a server at night.          The headline should sit on a hard pixel grid, set in a blocky face, with the join address right under it"
            .clone_into(&mut self.thought);
        let since = Instant::now().checked_sub(Duration::from_secs(14));
        (self.thinking, self.quiet, self.reached) = (since, since, true);
        self.run = Some(Run {
            events,
            stop,
            child: Arc::new(Mutex::new(None)),
            model: "Opus 5.5".to_owned(),
            via: KIND_CLAUDE.to_owned(),
            started: Instant::now(),
            steering: None,
            pricing: usage::Pricing::default(),
            from: self.entries.len(),
        });
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
        // What the app sends by itself to keep the assistant at its task list is not a message of the user's.
        let waiting = self.waiting.iter().filter(|waiting| waiting.shown != tasks::WENT_ON).map(|waiting| (waiting.shown.clone(), false));
        steered.into_iter().map(|text| (text, true)).chain(waiting).collect()
    }

    /// Takes back the message at `place` of those that haven't gone to an assistant yet ([`Chat::pending`]):
    /// the user doesn't want it sent after all.
    // Agent: LOCKS the run's inbox briefly. A message the assistant took a moment ago is no longer there; then
    // nothing happens.
    pub fn unqueue(&mut self, place: usize) {
        let inbox = self.run.as_ref().and_then(|run| run.steering.as_ref());
        let steered = inbox.map_or(0, |inbox| lock(inbox).len());
        if place < steered {
            if let Some(mut inbox) = inbox.map(|inbox| lock(inbox))
                && place < inbox.len()
            {
                inbox.remove(place);
            }
            return;
        }
        // What the app itself queued to keep the assistant at its task list is not among the messages shown.
        let own: Vec<usize> = (0..self.waiting.len()).filter(|at| self.waiting[*at].shown != tasks::WENT_ON).collect();
        if let Some(at) = own.get(place - steered) {
            self.waiting.remove(*at);
        }
    }

    /// Does what the user did in the question the assistant asks: ticks an answer, takes what they wrote, or
    /// sends what they chose. Sending the last answer gives the message to send to the assistant.
    pub fn choose(&mut self, pick: &crate::asking::Pick) -> Option<Waiting> {
        use crate::asking::Pick;
        // Human: taking a queued message back has nothing to do with a question; it is done whatever is asked.
        if let Pick::Unqueue(place) = pick {
            self.unqueue(*place);
            return None;
        }
        let asked = self.entries.iter_mut().rev().find_map(|entry| entry.asked.as_mut())?;
        let (shown, told) = match pick {
            Pick::Toggle { question, answer } => {
                asked.toggle(*question, answer);
                return None;
            }
            Pick::Write { question, text } => {
                asked.write(*question, text);
                return None;
            }
            Pick::Send { question } => asked.confirm(*question)?,
            Pick::Dismiss => {
                self.dismiss();
                return None;
            }
            // Taken care of above.
            Pick::Unqueue(_) => return None,
        };
        Some(Waiting { shown, told, files: Vec::new(), pasted: Vec::new(), pointed: Vec::new() })
    }

    /// The assistant has just offered next steps: for the rest of its answer it changes nothing more.
    // Human: what the assistant suggests is for the user to choose. Left alone, a model offers three steps and
    // then starts on the first one itself; from the offer on its tools that change the design are refused
    // until the answer is over, so a suggestion is only ever worked on because the user picked it.
    pub fn hold_for_choice(&mut self) {
        self.holds = self.running();
    }

    /// Whether the run under way has offered next steps and so may change nothing more.
    #[must_use]
    pub fn awaits_choice(&self) -> bool {
        self.holds && self.running()
    }

    /// The next steps the assistant has offered and the user hasn't taken or dropped yet, newest first.
    #[must_use]
    pub fn offers(&self) -> &[String] {
        &self.offered
    }

    /// Takes in next steps the assistant offers: they go before the ones already there, none twice, and the
    /// oldest give way when there are too many.
    pub fn offer(&mut self, steps: &[String]) {
        // Human: "Do both" is not a step; taking several at once is what the button beside the pills is for.
        for step in steps.iter().rev().filter(|step| !asks::only_points_at_others(step)) {
            if !self.offered.contains(step) {
                self.offered.insert(0, step.clone());
            }
        }
        self.offered.truncate(asks::MOST_OFFERED);
    }

    /// Takes the offered step at `place`: gives the message that asks for it.
    pub fn take_offer(&mut self, place: usize) -> Option<Waiting> {
        (place < self.offered.len()).then(|| {
            let step = self.offered.remove(place);
            Waiting { shown: step.clone(), told: step, files: Vec::new(), pasted: Vec::new(), pointed: Vec::new() }
        })
    }

    /// Takes every offered step at once: gives the one message that asks for all of them.
    pub fn take_offers(&mut self) -> Option<Waiting> {
        let steps = std::mem::take(&mut self.offered);
        (!steps.is_empty()).then(|| Waiting {
            shown: asks::all_shown(&steps),
            told: asks::all_told(&steps),
            files: Vec::new(),
            pasted: Vec::new(),
            pointed: Vec::new(),
        })
    }

    /// Drops the offered step at `place`: the user doesn't want it.
    pub fn drop_offer(&mut self, place: usize) {
        if place < self.offered.len() {
            self.offered.remove(place);
        }
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
        // Human: what the run had told of its requests and the chat hadn't taken in yet still counts; whatever
        // else it had told is left unsaid, as the run was stopped.
        for report in reports_left(&run.events) {
            self.count(&run.pricing, &report);
        }
        self.close(run.from);
        // Human: the task that was at work is marked as broken off until work on the list goes on.
        self.tasks.halt();
        self.thought.clear();
        (self.thinking, self.quiet) = (None, None);
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
        // Human: a list that is not worked through is said again with every message, so the assistant goes
        // on with the task at work instead of starting over.
        let list = match self.tasks.reminder() {
            reminder if reminder.is_empty() => String::new(),
            reminder => format!("<task_list>{reminder}</task_list>\n"),
        };
        files::written(&format!("{}\n{dealt}{device}{logo}{list}\n{}", self.retold(&ask.context, known), ask.text), &ask.files)
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
        self.tasks.resume();
        self.refused.clear();
        // Human: what the app itself sends to keep the assistant at its list is a note, not words of the user's.
        let role = if ask.shown == tasks::WENT_ON { Role::Step } else { Role::User };
        self.entries.push(Entry { pointed: ask.pointed.clone(), ..Entry::new(role, ask.shown.clone()) });
        let from = self.entries.len();
        self.fresh = true;
        self.thought.clear();
        (self.thinking, self.quiet, self.reached) = (None, Some(Instant::now()), false);
        let (send, events) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let child = Arc::new(Mutex::new(None));
        let kept = Mutex::new((self.session.clone(), self.history.clone()));
        let (flag, held) = (stop.clone(), child.clone());
        // Human: the ids of the run's `ask_user` calls, and whether one of them asked a question: the run then
        // waits for the user's answer, and nothing is reviewed or sent back to the assistant meanwhile.
        let asking: Mutex<(Vec<String>, bool)> = Mutex::new((Vec::new(), false));
        let (model, via) = (ask.model.clone(), via(&ask.engine));
        let pricing = ask.pricing.clone();
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
                let verdict = critic::judge(&ask.engine, &files::flat(&ask.text), picture, round == critic::ROUNDS, &held, &|report| {
                    tell(Event::Used(report));
                });
                if let Err(error) = &verdict {
                    log::warn!("the design couldn't be reviewed: {error}");
                }
                tell(Event::ToolEnd { id, failed: verdict.is_err() });
                let Ok(Some(changes)) = verdict else { break };
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                tell(Event::Break);
                tell(Event::Revising);
                answer(&critic::follow_up(&changes), &[]);
            }
            tell(Event::Done);
        });
        match started {
            Ok(_) => self.run = Some(Run { events, stop, child, model, via, started: Instant::now(), steering, pricing, from }),
            Err(error) => self.entries.push(Entry::new(Role::Problem, format!("The assistant couldn't be started: {error}"))),
        }
    }

    /// Does a call of one of the task list's tools ([`tasks`]), and gives what the model is told.
    ///
    /// # Errors
    /// What the list refuses, said so that the model can put it right.
    // Human: a finished list that still stands at the top gives way at once to a new one.
    pub fn task_call(&mut self, tool: &str, input: &Value) -> Result<String, String> {
        match tool {
            tasks::PLAN => {
                self.file_tasks();
                self.tasks.plan(input)
            }
            tasks::ADD => self.tasks.add(input),
            _ => self.tasks.finish(input),
        }
    }

    /// Files a task list that was worked through into the conversation, at its end: a note that opens to show
    /// the tasks. The top of the panel is free for the next list then.
    fn file_tasks(&mut self) {
        let finished = self.tasks.hand_over();
        if finished.is_empty() {
            return;
        }
        let count = finished.len();
        self.fresh = true;
        self.entries.push(Entry { finished, ..Entry::new(Role::Step, format!("Tasks completed: {count} of {count}")) });
    }

    /// Takes in what the run has said since the last time. Returns whether anything changed.
    pub fn poll(&mut self) -> bool {
        // Human: a list whose last task is finished stays at the top for a moment, then moves into the
        // conversation.
        let filed = self.tasks.due(Instant::now());
        if filed {
            self.file_tasks();
        }
        let Some(run) = &self.run else { return filed };
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
        let changed = filed || over || !events.is_empty();
        let now = Instant::now();
        for event in events {
            self.hear(event, now);
        }
        if over {
            let run = self.run.take();
            let from = run.as_ref().map_or(self.entries.len(), |run| run.from);
            // Human: a message handed to the run that it ended without taking goes first among the waiting ones.
            if let Some(inbox) = run.and_then(|run| run.steering) {
                let unheard: Vec<Waiting> = std::mem::take(&mut *lock(&inbox)).into_iter().map(|steer| steer.message).collect();
                self.waiting.splice(0..0, unheard);
            }
            self.thought_over(now);
            self.quiet = None;
            self.settle();
            self.close(from);
            self.holds = false;
            self.keep_at_the_list();
        }
        changed
    }

    /// The run is over. When it left its task list unfinished, the assistant is sent back to it.
    // Human: the list is the job the user was promised. An assistant that ends its answer in the middle of it
    // (a weaker model that loses the thread, or one that ran into the most requests one message may take) is
    // told where the list stands and to go on, as a waiting message of the app's own. A run that ended with a
    // problem, that asks the user something, or that the user stopped is left alone, and so is a list on
    // which nothing got finished [`tasks::URGED_MOST`] times in a row: its task is then marked as broken off.
    fn keep_at_the_list(&mut self) {
        let ended_badly = self.entries.last().is_some_and(|entry| entry.role == Role::Problem);
        if !self.tasks.open() || ended_badly || !self.waiting.is_empty() || asking(&self.entries).is_some() {
            return;
        }
        let stands = (self.tasks.made(), self.tasks.done());
        if (self.urged.1, self.urged.2) != stands {
            self.urged = (0, stands.0, stands.1);
        }
        if self.urged.0 >= tasks::URGED_MOST {
            self.tasks.halt();
            self.entries.push(Entry::new(Role::Step, tasks::GAVE_UP));
            return;
        }
        self.urged.0 += 1;
        self.waiting.push(Waiting {
            shown: tasks::WENT_ON.to_owned(),
            told: self.tasks.unfinished(),
            files: Vec::new(),
            pasted: Vec::new(),
            pointed: Vec::new(),
        });
    }

    /// What the run under way has taken so far, to show while it works; `None` when nothing is answered or
    /// nothing is counted yet.
    // Human: while a request is under way only its tokens are known. A price for the finished requests alone
    // would read as the price so far, so the line shows tokens until the request reports.
    #[must_use]
    pub fn spending(&self) -> Option<usage::Spent> {
        let mut so_far = self.spent.clone();
        if !self.live.is_empty() {
            so_far.used.add(self.live);
            (so_far.cost, so_far.partial) = (None, false);
        }
        (self.run.is_some() && !so_far.is_empty()).then_some(so_far)
    }

    /// What the counter by the message box shows: what the answer under way has taken so far, or what the last
    /// answer took. Also says whether the answer is still under way. `None` when nothing was counted.
    #[must_use]
    pub fn counter(&self) -> Option<(usage::Spent, bool)> {
        let last = || self.entries.iter().rev().find_map(|entry| entry.spent.clone());
        self.spending().map(|so_far| (so_far, true)).or_else(|| last().map(|spent| (spent, false)))
    }

    /// Adds what one request took, priced by `pricing`, to the run's sum.
    fn count(&mut self, pricing: &usage::Pricing, report: &usage::Report) {
        self.spent.add(&pricing.price(report));
    }

    /// The run whose lines begin at `from` is over: its sum goes on its last line and into the conversation's.
    // Human: the last line is the assistant's last words, or the problem the run ended with. A run that left
    // neither (it was stopped before it wrote) still counts in the conversation's sum.
    fn close(&mut self, from: usize) {
        // Human: a request that never reported (it broke off, or the run was stopped) still took what was
        // counted along; its price is not known, so the sum becomes the least the run cost.
        let live = std::mem::take(&mut self.live);
        if !live.is_empty() {
            self.spent.add(&usage::Spent { used: live, ..usage::Spent::default() });
        }
        let spent = std::mem::take(&mut self.spent);
        if spent.is_empty() {
            return;
        }
        self.total.add(&spent);
        let last = self.entries.iter().rposition(|entry| matches!(entry.role, Role::Assistant | Role::Problem)).filter(|at| *at >= from);
        if let Some(entry) = last.and_then(|at| self.entries.get_mut(at)) {
            entry.spent = Some(spent);
        }
    }

    /// Takes in one thing the run said, at `now`.
    fn hear(&mut self, event: Event, now: Instant) {
        // Human: what a problem interrupted is noted before the problem itself changes it.
        let about = if matches!(event, Event::Problem(_)) { self.about(now) } else { Vec::new() };
        // Human: the model thinks until it writes, uses a tool or is done; what its engine tells for its own
        // books (the session, the record of the conversation) says nothing about that.
        match &event {
            Event::Session(_) | Event::History(_) | Event::Used(_) | Event::Meanwhile(_) => {}
            Event::Thought(_) => self.reached = true,
            _ => {
                self.thought_over(now);
                (self.quiet, self.reached) = (Some(now), true);
            }
        }
        match event {
            Event::Text(more) => match self.entries.last_mut() {
                Some(last) if last.role == Role::Assistant && !self.fresh => last.text.push_str(&more),
                _ if more.trim().is_empty() => {}
                _ => {
                    self.entries.push(Entry::new(Role::Assistant, more.trim_start()));
                    self.fresh = false;
                }
            },
            Event::Thought(more) => {
                self.thinking.get_or_insert(now);
                self.thought.push_str(&more);
            }
            // Human: thinking that arrived as the answer's words (see `openai::Turn::say`) leaves the
            // conversation again; an entry that held nothing else goes with it.
            Event::Unsaid(said) => {
                if let Some(last) = self.entries.last_mut().filter(|last| last.role == Role::Assistant && !self.fresh) {
                    if let Some(kept) = last.text.strip_suffix(said.as_str()).or_else(|| last.text.strip_suffix(said.trim_start())).map(str::len) {
                        last.text.truncate(kept);
                    }
                    if last.text.trim().is_empty() {
                        self.entries.pop();
                        self.fresh = true;
                    }
                }
            }
            Event::Break => self.fresh = true,
            Event::Again(text) => {
                // Human: a call the broken answer was still writing was never made; its note would turn for ever.
                self.entries.retain(|entry| entry.step.as_ref().is_none_or(|step| step.state != State::Running));
                self.fresh = true;
                self.entries.push(Entry::new(Role::Step, text));
            }
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
                self.tasks.halt();
                self.fresh = true;
                self.entries.push(Entry { about, ..Entry::new(Role::Problem, text) });
            }
            Event::Meanwhile(used) => self.live = used,
            Event::Used(report) => {
                // Human: the report is the engine's last word on what it counted along.
                self.live = usage::Used::default();
                if let Some(pricing) = self.run.as_ref().map(|run| run.pricing.clone()) {
                    self.count(&pricing, &report);
                }
            }
            // Human: what the reviewer asks for is work on what was made, not a step the assistant suggested;
            // an answer that had offered next steps may change the design again for it.
            Event::Revising => self.holds = false,
            Event::Done => {}
        }
    }

    /// What is known at `now` about the run a problem comes from: who answers and through what, when, how far
    /// into the answer, and what the assistant was doing.
    fn about(&self, now: Instant) -> Vec<(&'static str, String)> {
        let Some(run) = &self.run else { return Vec::new() };
        let at_work = self.entries.iter().rev().filter_map(|entry| entry.step.as_ref()).find(|step| step.state == State::Running);
        let doing = match at_work {
            Some(step) if step.detail.is_empty() => steps::doing(&step.tool),
            Some(step) => format!("{} ({})", steps::doing(&step.tool), step.detail),
            None => Pause::of(&self.entries, self.reached, self.thinking.is_some()).words(&run.model),
        };
        let clock = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs());
        let steps = self.entries.iter().rev().take_while(|entry| entry.role != Role::User).filter(|entry| entry.step.is_some()).count();
        vec![
            ("Assistant", format!("{} through {}", run.model, run.via)),
            ("While", doing),
            ("After", format!("{} and {steps} tool {}", span(now.saturating_duration_since(run.started)), if steps == 1 { "call" } else { "calls" })),
            ("When", stamp(clock)),
        ]
    }

    /// The model has stopped thinking at `now`: a thought that took a while leaves a note saying how long.
    fn thought_over(&mut self, now: Instant) {
        self.thought.clear();
        let Some(since) = self.thinking.take() else { return };
        let took = now.saturating_duration_since(since);
        if took >= THOUGHT_NOTED {
            self.fresh = true;
            self.entries.push(Entry::new(Role::Step, format!("Thought for {}", span(took))));
        }
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
            let Ok(mut asked) = asks::parse(input) else { return };
            self.dismiss();
            // Human: next steps are not a question that waits in the conversation: they are collected beside
            // the message box, where the user picks them when they like, one by one or all at once.
            if asked.suggests {
                self.offer(asked.questions.first().map_or(&[][..], |steps| &steps.options));
                asked.closed = true;
            }
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

    /// A call of `tool` from this chat has failed, and the tool said `why`. The note of the call says it once
    /// the call is over.
    // Human: "Couldn't change the design" alone leaves the user guessing; the tool's own reason is what tells
    // them, and whoever they report it to, what went wrong.
    pub fn tool_failed(&mut self, tool: &str, why: &str) {
        self.refused.push((tool.to_owned(), why.to_owned()));
    }

    /// The call `id` is over, well or with an error.
    fn finish(&mut self, id: &str, failed: bool) {
        let Some(at) = self.note_of(id) else { return };
        let Some(entry) = self.entries.get_mut(at) else { return };
        let Some(step) = &mut entry.step else { return };
        step.state = if failed { State::Failed } else { State::Done };
        // The reason the tool gave for this call, when it gave one: calls of one tool end in the order they failed.
        let heard = self.refused.iter().position(|(tool, _)| *tool == step.tool);
        step.why = match heard {
            Some(place) if failed => self.refused.remove(place).1,
            _ => String::new(),
        };
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

    #[test]
    fn a_failed_call_keeps_the_reason_its_tool_gave() {
        let why = "Operation 0 failed: there is no layer with the id \"hero\". Nothing of this call was kept.";
        let mut chat = with_events(vec![
            Event::Tool { id: "t1".to_owned(), name: "mcp__pencraft__edit_layers".to_owned() },
            Event::ToolInput { id: "t1".to_owned(), input: serde_json::json!({ "description": "darken the hero", "operations": [{}] }) },
            Event::ToolEnd { id: "t1".to_owned(), failed: true },
            Event::Tool { id: "t2".to_owned(), name: "read_layers".to_owned() },
            Event::ToolInput { id: "t2".to_owned(), input: serde_json::json!({ "ids": ["a"] }) },
            Event::ToolEnd { id: "t2".to_owned(), failed: true },
            Event::Done,
        ]);
        // The tool says why before the engine says that the call is over.
        chat.tool_failed("edit_layers", why);
        assert!(chat.poll());
        let notes: Vec<(&str, &str, &str)> = chat
            .entries
            .iter()
            .filter_map(|entry| entry.step.as_ref().map(|step| (entry.text.as_str(), step.detail.as_str(), step.why.as_str())))
            .collect();
        // What was wanted stays beside the reason; a failure nothing was heard about has none.
        assert_eq!(notes, [("Couldn't change the design", "Darken the hero", why), ("Couldn't read the layers", "1 layer", "")]);
        assert_eq!(steps::brief(why), "there is no layer with the id \"hero\".");
        assert_eq!(
            steps::brief("Operation 3 failed: `update` needs `properties`. Nothing of this call was kept."),
            "Operation 3 failed: `update` needs `properties`."
        );
        assert_eq!(steps::brief("The document was closed."), "The document was closed.");
    }

    #[test]
    fn an_assistant_that_leaves_its_task_list_unfinished_is_sent_back_to_it() {
        let ended = |chat: &mut Chat| {
            let (send, receive) = channel();
            send.send(Event::Done).unwrap();
            chat.run.as_mut().unwrap().events = receive;
            assert!(chat.poll() && !chat.running());
        };
        let mut chat = with_events(vec![Event::Text("Halfway.".to_owned())]);
        chat.tasks = tasks::Tasks::sample(2);
        ended(&mut chat);
        // The app's own message waits to be sent, and the panel doesn't show it as one of the user's.
        assert_eq!(chat.pending(), []);
        let urged = chat.next_waiting().unwrap();
        assert_eq!(urged.shown, tasks::WENT_ON);
        assert!(urged.told.contains("task 3 of 6"), "{}", urged.told);
        // Sent back a second time without a task finished in between; the third time the list is left.
        for expected in [true, false] {
            chat.run = with_events(Vec::new()).run;
            ended(&mut chat);
            assert_eq!(chat.next_waiting().is_some(), expected);
        }
        assert!(chat.tasks.halted());
        assert_eq!(chat.entries.last().map(|entry| entry.text.as_str()), Some(tasks::GAVE_UP));
        // A task finished since starts the count anew.
        chat.tasks.resume();
        chat.tasks.finish(&serde_json::json!({})).unwrap();
        chat.run = with_events(Vec::new()).run;
        ended(&mut chat);
        assert!(chat.next_waiting().is_some());
        // A run that ended with a problem, and a list that is worked through, are left alone.
        chat.entries.push(Entry::new(Role::Problem, "The provider answered with an error."));
        chat.run = with_events(Vec::new()).run;
        ended(&mut chat);
        assert!(chat.next_waiting().is_none());
    }

    #[test]
    fn next_steps_are_collected_and_taken_one_by_one_or_all_at_once() {
        let offer = |id: &str, steps: Value| {
            vec![
                Event::Tool { id: id.to_owned(), name: asks::TOOL.to_owned() },
                Event::ToolInput { id: id.to_owned(), input: serde_json::json!({ "suggestions": steps }) },
                Event::ToolEnd { id: id.to_owned(), failed: false },
            ]
        };
        let mut events = offer("a1", serde_json::json!(["Add a dark version", "Export it"]));
        events.extend(offer("a2", serde_json::json!(["Make it bolder", "Export it"])));
        events.push(Event::Done);
        let mut chat = with_events(events);
        assert!(chat.poll());
        // An answer that has offered next steps changes nothing more; when it is over, the next one may.
        let mut offering = with_events(Vec::new());
        assert!(!offering.awaits_choice());
        offering.hold_for_choice();
        assert!(offering.awaits_choice());
        offering.run.as_mut().unwrap().events = {
            let (send, receive) = channel();
            send.send(Event::Done).unwrap();
            receive
        };
        assert!(offering.poll() && !offering.awaits_choice());
        // The reviewer's changes are work on what was made: for them the answer may change the design again.
        let mut reviewed = with_events(Vec::new());
        reviewed.hold_for_choice();
        reviewed.hear(Event::Revising, Instant::now());
        assert!(reviewed.running() && !reviewed.awaits_choice());
        // Outside an answer there is nothing to hold.
        offering.hold_for_choice();
        assert!(!offering.awaits_choice());
        // The newest come first and none stands twice; nothing waits in the conversation as a question.
        assert_eq!(chat.offers(), ["Make it bolder", "Add a dark version", "Export it"]);
        assert!(asking(&chat.entries).is_none());
        // One is taken as the user's own message, one is dropped.
        let taken = chat.take_offer(1).unwrap();
        assert_eq!((taken.shown.as_str(), taken.told.as_str()), ("Add a dark version", "Add a dark version"));
        chat.drop_offer(0);
        assert_eq!(chat.offers(), ["Export it"]);
        assert!(chat.take_offer(4).is_none());
        // All at once is one message that lists them.
        chat.offer(&["Add a footer".to_owned()]);
        let all = chat.take_offers().unwrap();
        assert_eq!(all.shown, "Add a footer\nExport it");
        assert_eq!(all.told, "Do these next steps, one after the other:\n- Add a footer\n- Export it");
        assert!(chat.offers().is_empty() && chat.take_offers().is_none());
        // "Do both" and the like only point at the other steps: they are not kept.
        chat.offer(&["Do both".to_owned(), "All of the above".to_owned(), "Do all three.".to_owned(), "Both".to_owned()]);
        assert_eq!(chat.offers().len(), 0);
        assert!(!asks::only_points_at_others("Do all the icons in one style") && !asks::only_points_at_others("Make both buttons round"));
        // More than are kept push the oldest out.
        let many: Vec<String> = (0..12).map(|n| format!("Step {n}")).collect();
        chat.offer(&many);
        assert_eq!((chat.offers().len(), chat.offers()[0].as_str()), (asks::MOST_OFFERED, "Step 0"));
    }

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
            via: "Anthropic Messages at api.anthropic.com".to_owned(),
            started: Instant::now(),
            steering: Some(Arc::new(api::Inbox::default())),
            pricing: usage::Pricing { model: "m".to_owned(), own: usage::Rates::typed("1", "2"), ..usage::Pricing::default() },
            from: 1,
        };
        Chat { entries: vec![Entry::new(Role::User, "Make a card")], run: Some(run), fresh: true, ..Chat::default() }
    }

    #[test]
    fn a_runs_requests_are_summed_onto_its_last_line() {
        use usage::{Basis, Pricing, Rates, Report, Used};
        let pricing = Pricing { model: "m".to_owned(), own: Rates::typed("1", "2"), ..Pricing::default() };
        let report = |input, output| Report { used: Used { input, output, ..Used::default() }, model: None, billed: None };
        let mut chat = Chat::default();
        chat.entries.push(Entry::new(Role::User, "Make a card"));
        let from = chat.entries.len();
        chat.entries.push(Entry::new(Role::Assistant, "On it."));
        chat.entries.push(Entry::new(Role::Step, "Changed 2 layers"));
        chat.entries.push(Entry::new(Role::Assistant, "Done."));
        chat.count(&pricing, &report(1000, 100));
        chat.count(&pricing, &report(2000, 50));
        chat.close(from);
        let spent = chat.entries[3].spent.clone().unwrap();
        assert_eq!((spent.used.input, spent.used.output, spent.cost, spent.basis), (3000, 150, Some(3000 * 1_000_000 + 150 * 2_000_000), Basis::Own));
        assert!(chat.entries[1].spent.is_none());
        assert_eq!(chat.total, spent);
        // The next run ends in a problem; its line is under the problem, and the total grows.
        chat.entries.push(Entry::new(Role::User, "Again"));
        let from = chat.entries.len();
        chat.entries.push(Entry::new(Role::Problem, "The provider didn't answer."));
        chat.count(&pricing, &report(500, 0));
        chat.close(from);
        assert_eq!(chat.entries[5].spent.as_ref().map(|spent| spent.used.input), Some(500));
        assert_eq!(chat.total.used.input, 3500);
        // A run that used nothing, and one that left no line of its own, mark nothing.
        chat.close(chat.entries.len());
        chat.count(&pricing, &report(7, 0));
        chat.close(chat.entries.len());
        assert_eq!((chat.total.used.input, chat.entries.iter().filter(|entry| entry.spent.is_some()).count()), (3507, 2));
        // A new conversation starts at nothing.
        chat.clear();
        assert!(chat.total.is_empty());
    }

    #[test]
    fn what_an_answer_takes_counts_up_while_the_assistant_works() {
        use usage::{Report, Used};
        let used = |input, output| Used { input, output, ..Used::default() };
        let (send, receive) = channel();
        let mut chat = with_events(Vec::new());
        if let Some(run) = &mut chat.run {
            run.events = receive;
        }
        // Nothing is counted yet, so nothing is shown.
        assert!(chat.spending().is_none());
        // The request under way says what it has taken so far, each time anew: tokens, and no price yet.
        send.send(Event::Meanwhile(used(1000, 0))).unwrap();
        send.send(Event::Meanwhile(used(1000, 40))).unwrap();
        chat.poll();
        let so_far = chat.spending().unwrap();
        assert_eq!((so_far.used, so_far.cost, so_far.line().as_str()), (used(1000, 40), None, "1k in · 40 out"));
        // Its report takes the place of what was counted along, and has a price.
        send.send(Event::Used(Report { used: used(1000, 100), model: None, billed: None })).unwrap();
        chat.poll();
        let so_far = chat.spending().unwrap();
        assert_eq!((so_far.used, so_far.cost), (used(1000, 100), Some(1000 * 1_000_000 + 100 * 2_000_000)));
        // The next request counts on top of the finished one, again without a price until it reports.
        send.send(Event::Text("On it.".to_owned())).unwrap();
        send.send(Event::Meanwhile(used(2000, 5))).unwrap();
        chat.poll();
        assert_eq!(chat.spending().map(|so_far| (so_far.used, so_far.cost)), Some((used(3000, 105), None)));
        // A run that ends before that request reports keeps what was counted, as the least it cost.
        chat.stop();
        let spent = chat.entries[1].spent.clone().unwrap();
        assert_eq!((spent.used, spent.cost, spent.partial), (used(3000, 105), Some(1000 * 1_000_000 + 100 * 2_000_000), true));
        assert!(chat.spending().is_none() && chat.total == spent);
    }

    #[test]
    fn what_a_run_tells_of_its_requests_reaches_its_last_line_also_when_it_is_stopped() {
        use usage::{Report, Used};
        let report = |input, output| Event::Used(Report { used: Used { input, output, ..Used::default() }, model: None, billed: None });
        // A run that ends by itself: both requests are on the line under its last words, priced as the run was.
        let mut chat = with_events(vec![
            Event::Text("On it.".to_owned()),
            report(1000, 100),
            Event::Break,
            Event::Text("Done.".to_owned()),
            report(2000, 50),
            Event::Done,
        ]);
        assert!(chat.poll() && !chat.running());
        let spent = chat.entries[2].spent.clone().unwrap();
        assert_eq!((spent.used.input, spent.used.output, spent.cost), (3000, 150, Some(3000 * 1_000_000 + 150 * 2_000_000)));
        assert!(chat.entries[1].spent.is_none() && chat.total == spent);
        // A report outside a run counts for nothing.
        chat.hear(report(9, 9), Instant::now());
        chat.close(0);
        assert_eq!(chat.total, spent);
        // A run stopped with a report still on its way: the report counts, the words that came with it don't.
        // Human: the test holds the run's sending end, so the run goes on until it is stopped.
        let (send, receive) = channel();
        let mut chat = with_events(Vec::new());
        if let Some(run) = &mut chat.run {
            run.events = receive;
        }
        send.send(Event::Text("On it.".to_owned())).unwrap();
        send.send(report(1000, 100)).unwrap();
        assert!(chat.poll() && chat.running());
        send.send(Event::Text(" More".to_owned())).unwrap();
        send.send(report(2000, 50)).unwrap();
        send.send(Event::Problem("Too late".to_owned())).unwrap();
        chat.stop();
        let said: Vec<(Role, &str)> = chat.entries.iter().map(|entry| (entry.role, entry.text.as_str())).collect();
        assert_eq!(said, vec![(Role::User, "Make a card"), (Role::Assistant, "On it."), (Role::Step, "Stopped")]);
        assert_eq!(chat.entries[1].spent.as_ref().map(|spent| (spent.used.input, spent.used.output)), Some((3000, 150)));
        assert_eq!((chat.total.used.input, chat.spent.is_empty()), (3000, true));
        // Stopped before it wrote a line: nothing is marked, and the conversation's sum still has it.
        let mut chat = with_events(vec![report(40, 2)]);
        chat.stop();
        assert_eq!((chat.total.used.input, chat.entries.iter().filter(|entry| entry.spent.is_some()).count()), (40, 0));
    }

    #[test]
    fn a_choice_knows_how_its_provider_is_priced() {
        use usage::Terms;
        let list = prices::Catalogue::parse(
            r#"{ "xai": { "models": { "grok-4.5": { "cost": { "input": 2, "output": 6 } } } },
                 "ollama-cloud": { "models": { "kimi-k3": { "cost": { "input": 1, "output": 4 } } } } }"#,
        )
        .unwrap();
        let model = |id: &str, input: &str, output: &str| ProviderModel {
            id: id.to_owned(),
            input_price: input.to_owned(),
            output_price: output.to_owned(),
            ..ProviderModel::default()
        };
        let provider = |name: &str, api: &str, base: &str, models: Vec<ProviderModel>| CustomProvider {
            name: name.to_owned(),
            api: api.to_owned(),
            base_url: base.to_owned(),
            models,
            key_ref: Some("k".to_owned()),
        };
        let agents = Agents {
            local: Vec::new(),
            providers: vec![
                provider("Grok", COMPLETIONS_API, "https://api.x.ai/v1", vec![model("grok-4.5", "", ""), model("grok-mini", "0.5", "1")]),
                provider("Cloud", COMPLETIONS_API, "https://ollama.com/v1", vec![model("kimi-k3", "", "")]),
                provider("Here", OLLAMA_API, "http://localhost:11434", vec![model("qwen3", "", "")]),
            ],
            ..Agents::default()
        };
        let all = choices(&agents, &|_| None, false);
        let of = |id: &str| pricing(chosen(&all, id).unwrap(), &agents, &list);
        let grok = of("api/Grok/grok-4.5");
        assert_eq!((grok.model.as_str(), grok.own.is_none(), grok.listed.len(), grok.terms), ("grok-4.5", true, 1, Terms::PerToken));
        assert!(of("api/Grok/grok-mini").own.is_some());
        assert_eq!(of("api/Cloud/kimi-k3").terms, Terms::Plan);
        assert_eq!((of("api/Here/qwen3").terms, of("api/Here/qwen3").listed.len()), (Terms::Local, 0));
        // With the look-up turned off the list is not used; the user's own prices still are.
        let off = Agents { prices: false, ..agents.clone() };
        let quiet = |id: &str| pricing(chosen(&all, id).unwrap(), &off, &list);
        assert!(quiet("api/Grok/grok-4.5").listed.is_empty() && quiet("api/Grok/grok-mini").own.is_some());
        // A program's cost is a worth, whatever it is; one that only prints its answer has nothing to price.
        let code = Choice {
            id: "code/m".to_owned(),
            label: String::new(),
            plain: String::new(),
            engine: Engine::Code { flavor: code::Flavor::Claude, program: PathBuf::from("claude"), model: "m".to_owned() },
        };
        assert_eq!((pricing(&code, &agents, &list).terms, pricing(&code, &agents, &list).listed.len()), (Terms::Plan, 0));
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
    fn words_that_were_thinking_leave_the_conversation_again() {
        let mut chat = with_events(Vec::new());
        let now = Instant::now();
        let texts =
            |chat: &Chat| chat.entries.iter().filter(|entry| entry.role == Role::Assistant).map(|entry| entry.text.clone()).collect::<Vec<_>>();
        let before = texts(&chat).len();
        // An answer that was thinking all along goes whole, and the real one takes its place.
        chat.hear(Event::Text("\nSo I call ".to_owned()), now);
        chat.hear(Event::Text("get_state.".to_owned()), now);
        chat.hear(Event::Unsaid("\nSo I call get_state.".to_owned()), now);
        assert_eq!(texts(&chat).len(), before);
        chat.hear(Event::Text("Done.".to_owned()), now);
        // Words that follow an earlier answer without a break lose only what is taken back.
        chat.hear(Event::Text(" Hm, more".to_owned()), now);
        chat.hear(Event::Unsaid(" Hm, more".to_owned()), now);
        assert_eq!(texts(&chat).last().map(String::as_str), Some("Done."));
        // What was never said takes nothing away.
        chat.hear(Event::Unsaid("something else".to_owned()), now);
        assert_eq!(texts(&chat).last().map(String::as_str), Some("Done."));
    }

    #[test]
    fn a_wait_says_what_the_model_is_doing_and_a_long_thought_leaves_a_note() {
        let mut chat = with_events(Vec::new());
        let start = Instant::now();
        let after = |seconds: u64| start + Duration::from_secs(seconds);
        // Nothing has come back: the wait is for the model, by its name.
        assert_eq!(chat.pause(), Some(Pause::Reaching));
        assert_eq!(Pause::Reaching.words("Opus 5.5"), "Waiting for Opus 5.5");
        assert_eq!(Pause::Reaching.words(""), "Waiting for the assistant");
        // It thinks, at first without saying what; what it thinks is kept only while it does.
        chat.hear(Event::Session("s1".to_owned()), after(0));
        assert_eq!(chat.pause(), Some(Pause::Reaching));
        chat.hear(Event::Thought(String::new()), after(1));
        chat.hear(Event::Thought("A dark page. ".to_owned()), after(2));
        chat.hear(Event::Thought("Blocky type.".to_owned()), after(3));
        assert_eq!((chat.pause(), chat.thought()), (Some(Pause::Thinking), "A dark page. Blocky type."));
        chat.hear(Event::Tool { id: "t1".to_owned(), name: "screenshot".to_owned() }, after(9));
        assert_eq!((chat.thought(), chat.entries[1].text.as_str()), ("", "Thought for 8 s"));
        assert!(chat.entries[1].step.is_none() && chat.entries[1].role == Role::Step);
        // What it was just handed says what it does next.
        chat.hear(Event::ToolEnd { id: "t1".to_owned(), failed: false }, after(10));
        assert_eq!(chat.pause(), Some(Pause::Looking));
        chat.hear(Event::Tool { id: "t2".to_owned(), name: "edit_layers".to_owned() }, after(11));
        chat.hear(Event::ToolEnd { id: "t2".to_owned(), failed: true }, after(11));
        assert_eq!(chat.pause(), Some(Pause::Retrying));
        chat.hear(Event::Tool { id: "t3".to_owned(), name: "read_layers".to_owned() }, after(12));
        chat.hear(Event::ToolEnd { id: "t3".to_owned(), failed: false }, after(12));
        assert_eq!(chat.pause(), Some(Pause::Next));
        // A short thought leaves no note, and its own words are not a wait to name.
        chat.hear(Event::Thought("Now the answer.".to_owned()), after(13));
        chat.hear(Event::Text("A page.".to_owned()), after(14));
        assert_eq!((chat.pause(), chat.entries.len()), (Some(Pause::Writing), 6));
        assert_eq!(Pause::of(&[Entry::note(critic::TOOL, State::Done, "")], true, false), Pause::Reviewed);
        assert_eq!(Pause::of(&[Entry::new(Role::User, "wider")], true, false), Pause::Thinking);
        assert_eq!((span(Duration::from_secs(59)), span(Duration::from_secs(65))), ("59 s".to_owned(), "1 min 5 s".to_owned()));
        chat.stop();
        assert_eq!(chat.pause(), None);
    }

    #[test]
    fn a_problem_says_what_it_interrupted_and_can_be_reported() {
        let mut chat = with_events(Vec::new());
        let start = chat.run.as_ref().map_or_else(Instant::now, |run| run.started);
        let after = |seconds: u64| start + Duration::from_secs(seconds);
        chat.hear(Event::Tool { id: "t1".to_owned(), name: "get_state".to_owned() }, after(1));
        chat.hear(Event::ToolEnd { id: "t1".to_owned(), failed: false }, after(1));
        chat.hear(Event::Tool { id: "t2".to_owned(), name: "search_web".to_owned() }, after(2));
        chat.hear(Event::ToolInput { id: "t2".to_owned(), input: serde_json::json!({ "query": "pixel fonts" }) }, after(2));
        chat.hear(Event::Problem("Internal Server Error".to_owned()), after(65));
        let problem = chat.entries.last().unwrap();
        let fact = |name: &str| problem.about.iter().find(|(known, _)| *known == name).map(|(_, fact)| fact.as_str());
        assert_eq!(fact("Assistant"), Some("Opus 5.5 through Anthropic Messages at api.anthropic.com"));
        assert_eq!(fact("While"), Some("Searching the web (“pixel fonts”)"));
        assert_eq!(fact("After"), Some("1 min 5 s and 2 tool calls"));
        assert!(fact("When").is_some_and(|when| when.ends_with(" UTC")));
        let report = problem.report();
        assert!(report.starts_with("Problem: Internal Server Error\nAssistant: Opus 5.5 through"), "{report}");
        assert!(report.lines().last().is_some_and(|app| app.starts_with(concat!("App: Quill ", env!("CARGO_PKG_VERSION"), " on "))));
        // A problem outside a run is reported too, with what there is.
        let mut idle = Chat::default();
        idle.refuse("Hello".to_owned(), "No assistant is set up.");
        assert!(idle.entries[1].about.is_empty() && idle.entries[1].report().starts_with("Problem: No assistant is set up.\nApp: "));
        // Dates, across a leap day and at the start of the count.
        assert_eq!((stamp(0), stamp(1_795_181_530)), ("1970-01-01 00:00:00 UTC".to_owned(), "2026-11-20 13:32:10 UTC".to_owned()));
        assert_eq!(stamp(1_709_251_199), "2024-02-29 23:59:59 UTC");
        // Of a provider only the kind of API and the host are told.
        let api = Engine::Api {
            wire: Wire::Completions,
            endpoint: "https://openrouter.ai/api/v1/chat/completions?x=1".to_owned(),
            official: false,
            model: "m".to_owned(),
            window: 0,
            key: KeySource::None,
        };
        assert_eq!(via(&api), "OpenAI Completions at openrouter.ai");
        assert_eq!(
            via(&Engine::Plain { program: PathBuf::from("/usr/local/bin/aider"), arguments: String::new(), model: String::new() }),
            "the program aider"
        );
    }

    #[test]
    fn an_answer_that_broke_off_is_asked_for_again_and_its_unmade_call_is_forgotten() {
        let mut chat = with_events(Vec::new());
        let now = Instant::now();
        chat.hear(Event::Text("I'll change the headline.".to_owned()), now);
        chat.hear(Event::Tool { id: "t1".to_owned(), name: "edit_layers".to_owned() }, now);
        chat.hear(Event::Again("The provider broke off its answer; asking again (1 of 2)".to_owned()), now);
        chat.hear(Event::Text("The headline is larger now.".to_owned()), now);
        let said: Vec<(Role, &str)> = chat.entries.iter().map(|entry| (entry.role, entry.text.as_str())).collect();
        assert_eq!(
            said,
            vec![
                (Role::User, "Make a card"),
                (Role::Assistant, "I'll change the headline."),
                (Role::Step, "The provider broke off its answer; asking again (1 of 2)"),
                (Role::Assistant, "The headline is larger now."),
            ]
        );
        // Only an answer that broke off is asked for again, twice in a row at most, and never once stopped.
        let stop = AtomicBool::new(true);
        let told = std::cell::RefCell::new(Vec::new());
        let tell = |event: Event| {
            told.borrow_mut().push(format!("{event:?}"));
            true
        };
        let mut tries = 0;
        assert!(!api::again(api::BROKE_OFF, &mut tries, &stop, &tell) && tries == 0);
        let stop = AtomicBool::new(false);
        assert!(!api::again(500, &mut tries, &stop, &tell) && !api::again(0, &mut tries, &stop, &tell) && tries == 0);
        tries = 2;
        assert!(!api::again(api::BROKE_OFF, &mut tries, &stop, &tell) && told.borrow().is_empty());
        assert_eq!(
            (api::broke(&serde_json::json!({ "error": { "message": "x" } })), api::broke(&serde_json::json!({ "choices": [] }))),
            (api::BROKE_OFF, 0)
        );
    }

    #[test]
    fn a_question_of_the_assistants_waits_for_the_users_answer() {
        use crate::asking::Pick;
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
        let tick = |question: usize, answer: &str| Pick::Toggle { question, answer: answer.to_owned() };
        let write = |question: usize, text: &str| Pick::Write { question, text: text.to_owned() };
        // Nothing chosen sends nothing; what is ticked and written is sent together.
        assert_eq!(chat.choose(&Pick::Send { question: 0 }), None);
        assert_eq!((chat.choose(&tick(0, "Dark")), chat.choose(&Pick::Send { question: 0 })), (None, None));
        assert_eq!(chat.choose(&write(1, "Phone")), None);
        let message = chat.choose(&Pick::Send { question: 1 }).unwrap();
        assert_eq!((message.shown.as_str(), message.told.as_str()), ("Dark\nPhone", "Mood?\nAnswer: Dark\nScreen?\nAnswer: Phone"));
        assert!(open(&chat).is_none() && chat.choose(&Pick::Send { question: 1 }).is_none());
        // A message of the user's own closes the question and takes along what was picked before it.
        let mut events = ask("a3", questions);
        events.push(Event::Done);
        let mut chat = with_events(events);
        chat.poll();
        chat.choose(&tick(0, "Warm"));
        chat.choose(&Pick::Send { question: 0 });
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
        let message =
            |text: &str| Waiting { shown: text.to_owned(), told: text.to_owned(), files: Vec::new(), pasted: Vec::new(), pointed: Vec::new() };
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
        let chip = Pointed { at: Some(0..5), icon: "frame", name: "wider".to_owned(), picture: None };
        send.send(Event::Heard(Waiting { pointed: vec![chip.clone()], ..message("wider") })).unwrap();
        send.send(Event::Done).unwrap();
        assert!(chat.poll() && !chat.running());
        assert_eq!(chat.entries.last().map(|e| (e.role, e.text.as_str(), e.pointed.as_slice())), Some((Role::User, "wider", &[chip][..])));
        assert_eq!((chat.next_waiting(), chat.next_waiting(), chat.next_waiting()), (Some(message("too late")), Some(message("then export")), None));
        // A message that waits can be taken back, by its place among those the panel shows; a place that
        // isn't there changes nothing.
        chat.waiting = vec![message("one"), message("two"), message("three")];
        chat.unqueue(1);
        chat.unqueue(7);
        assert_eq!(chat.pending(), [("one".to_owned(), false), ("three".to_owned(), false)]);
        assert!(chat.choose(&crate::asking::Pick::Unqueue(0)).is_none());
        assert_eq!(chat.pending(), [("three".to_owned(), false)]);
        chat.waiting.clear();
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
                window: 0,
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
                window: 0,
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
