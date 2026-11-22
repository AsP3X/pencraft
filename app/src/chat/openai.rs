//! Answering by calling a provider's Chat Completions API: the one `OpenAI` introduced and most providers speak.
//!
//! The run is the same as with the Messages API ([`super::api`]): the model answers with words and tool calls,
//! the app runs the tools and sends their results back, until the model has nothing more to do. Only the shape
//! of the requests and of the streamed answers differs.

use std::io::{BufRead, BufReader};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};

use super::api::{self, Job, MOST_ROUNDS};
use super::usage::{self, Report};
use super::{Event, KeySource};
use crate::app::lock;

/// The most tool calls one answer may hold; a stream naming more is broken.
pub(super) const MOST_CALLS: usize = 64;
/// What stands in for a picture the provider's model can't take.
const NO_PICTURES: &str = "(A picture belongs here, but this model doesn't take pictures. Check your work with read_layers instead.)";

/// What a tool's picture is told with in the conversation.
const TOOL_PICTURE: &str = "The picture the tool made:";
/// How many of the model's latest answers are kept whole, with what their tools gave, when a conversation is
/// shortened.
const KEPT_ANSWERS: usize = 2;
/// A tool's result of up to this many bytes is kept whole however old it is.
const SHORT: usize = 600;
/// How many bytes of an older, longer result stay.
const HEAD: usize = 240;
/// What stands behind the start of a result that was shortened.
const DROPPED: &str = "…\n(The rest of this result was dropped to save room. Call the tool again if you need it.)";
/// What stands in for a tool's picture that was dropped.
const PICTURE_DROPPED: &str = "(The picture a tool made here was dropped to save room. Take a screenshot again if you need to see it.)";

/// The address chat requests are posted to for a provider reached at `base`.
// Human: the base address is the one the provider's own documentation gives for OpenAI-compatible clients; it
// already ends in the API's version where there is one.
#[must_use]
pub fn endpoint(base: &str) -> String {
    format!("{}/chat/completions", base.trim().trim_end_matches('/'))
}

/// The tools as the API takes them.
pub(super) fn tools(off: &[String]) -> Value {
    let all = crate::agent::tools_without(off);
    all.as_array()
        .into_iter()
        .flatten()
        .map(|tool| json!({ "type": "function", "function": { "name": tool["name"], "description": tool["description"], "parameters": tool["inputSchema"] } }))
        .collect()
}

/// A picture as this API takes it, from the content block the rest of the chat carries it as.
fn picture(block: &Value) -> Value {
    let (kind, data) = (block.pointer("/source/media_type").and_then(Value::as_str), block.pointer("/source/data").and_then(Value::as_str));
    json!({ "type": "image_url", "image_url": { "url": format!("data:{};base64,{}", kind.unwrap_or("image/png"), data.unwrap_or_default()) } })
}

/// A message of the user's: the text alone, or the text with each pasted picture right behind its name, and
/// then the attached pictures.
fn user(text: &str, pictures: &[Value]) -> Value {
    use super::files::Piece;
    if pictures.is_empty() {
        return json!({ "role": "user", "content": super::files::flat(text) });
    }
    let mut parts = Vec::new();
    let mut pasted = 0;
    for piece in super::files::pieces(text, pictures.len()) {
        match piece {
            Piece::Words(words) => parts.push(json!({ "type": "text", "text": words })),
            Piece::Picture(which) => {
                parts.extend(pictures.get(which).map(picture));
                pasted += 1;
            }
        }
    }
    parts.extend(pictures[pasted..].iter().map(picture));
    json!({ "role": "user", "content": parts })
}

/// The request for the conversation `history`; `counted` asks for the usage to come with the stream.
fn request(job: &Job<'_>, history: &[Value], counted: bool) -> Value {
    let mut messages = vec![json!({ "role": "system", "content": super::brief(true, job.instructions) })];
    messages.extend(history.iter().cloned());
    let mut body = json!({ "model": job.model, "stream": true, "tools": tools(job.off), "messages": messages });
    if counted {
        body["stream_options"] = json!({ "include_usage": true });
    }
    body
}

/// The addresses that refused a request asking for its usage, until the app closes.
// Human: a few providers that otherwise speak this API refuse a field they don't know. Such a provider is
// asked once per run of the app; after that its requests go without the question, and its answers get no line.
static UNCOUNTED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Whether requests to `endpoint` ask for their usage.
fn counts(endpoint: &str) -> bool {
    !lock(&UNCOUNTED).iter().any(|refused| refused == endpoint)
}

/// Remembers that `endpoint` refuses requests that ask for their usage, and says so once in the log.
// Human: the log names the provider's host only; the rest of an address may be the user's own.
fn uncounted(endpoint: &str) {
    let mut refused = lock(&UNCOUNTED);
    if refused.iter().any(|known| known == endpoint) {
        return;
    }
    refused.push(endpoint.to_owned());
    let host = endpoint.split_once("://").map_or(endpoint, |(_, rest)| rest).split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.rsplit_once('@').map_or(host, |(_, host)| host);
    log::warn!("{host} refuses requests that ask for their usage; its answers are not counted until the app closes");
}

/// Sends one request to `endpoint` through `send`, which is told whether the request asks for its usage.
// Human: a request refused with "bad request" may be refused for asking about its usage. It is sent once
// more without the question. If that goes through, the address is remembered; if not, the refusal has
// another reason (pictures, say) and later requests ask again.
fn asked(endpoint: &str, send: &dyn Fn(bool) -> Result<Turn, (u16, String)>) -> Result<Turn, (u16, String)> {
    if !counts(endpoint) {
        return send(false);
    }
    let first = send(true);
    if !matches!(first, Err((400 | 422, _))) {
        return first;
    }
    let plain = send(false);
    if plain.is_ok() {
        uncounted(endpoint);
    }
    plain
}

/// Takes every picture out of `history`, leaving a note in its place. Returns whether there was one.
// Human: a model that reads no pictures refuses the whole request; without them the run can go on.
fn without_pictures(history: &mut [Value]) -> bool {
    let mut found = false;
    for parts in history.iter_mut().filter_map(|message| message.get_mut("content").and_then(Value::as_array_mut)) {
        let before = parts.len();
        parts.retain(|part| part["type"] != "image_url");
        if parts.len() < before {
            found = true;
            parts.push(json!({ "type": "text", "text": NO_PICTURES }));
        }
    }
    found
}

/// Whether a conversation that fills `filled` tokens of a context of `room` is due for shortening: from two
/// thirds on. Never when the room isn't known.
fn crowded(filled: u64, room: u64) -> bool {
    room > 0 && filled.saturating_mul(3) >= room.saturating_mul(2)
}

/// Makes `history` smaller: of everything before the model's latest answers, a long tool result keeps its
/// start and a tool's picture is dropped, each with a note saying so. Returns whether anything changed.
// Human: the brief and the tools can't be made smaller and the model's own words are short; what fills a
// context over a long piece of work is what tools gave: screenshots, layers read, skills. The model has used
// the old ones already and can call a tool again. What the user wrote and attached is never touched. A
// message is changed once and stays as it is then, so the start of the conversation a provider has cached
// stays the same from one request to the next.
fn shorten(history: &mut [Value]) -> bool {
    let answers = history.iter().enumerate().filter(|(_, message)| message["role"] == "assistant").map(|(at, _)| at);
    let Some(kept) = answers.rev().nth(KEPT_ANSWERS - 1) else { return false };
    let mut changed = false;
    for message in history.iter_mut().take(kept) {
        if message["role"] == "tool" {
            let Some(said) = message["content"].as_str().filter(|said| said.len() > SHORT) else { continue };
            let end = said.char_indices().map(|(at, _)| at).take_while(|at| *at <= HEAD).last().unwrap_or(0);
            let start = said.get(..end).unwrap_or_default();
            message["content"] = json!(format!("{start}{DROPPED}"));
            changed = true;
        } else if message["role"] == "user" && message.pointer("/content/0/text").and_then(Value::as_str) == Some(TOOL_PICTURE) {
            message["content"] = json!(PICTURE_DROPPED);
            changed = true;
        }
    }
    changed
}

/// One tool call of an answer, as the stream builds it.
#[derive(Default)]
pub(super) struct ToolCall {
    pub(super) id: String,
    pub(super) name: String,
    /// The JSON of its input, as written so far.
    pub(super) arguments: String,
}

/// What opens a model's thinking where it is written among its words.
const THINKS: &str = "<think>";
/// What closes it.
const THOUGHT: &str = "</think>";

/// Where an answer's words stand to thinking written among them.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Aside {
    /// Nothing but the start of a possible [`THINKS`] has come.
    #[default]
    Open,
    /// The answer began with [`THINKS`]: its words are thoughts until [`THOUGHT`].
    Thinking,
    /// The answer's words are the answer, unless a [`THOUGHT`] still shows that they were thoughts.
    Said,
    /// The thinking is over: everything from here on is the answer.
    Closed,
}

/// One answer of the model as it comes in.
#[derive(Default)]
pub(super) struct Turn {
    pub(super) text: String,
    pub(super) calls: Vec<ToolCall>,
    pub(super) finish: Option<String>,
    /// What the answer took, when the provider said.
    pub(super) used: Option<Report>,
    /// How many tokens of the model's context the conversation fills with this answer; 0 when nobody said.
    pub(super) filled: u64,
    /// Whether the words coming in are the answer or thinking written among them.
    aside: Aside,
}

impl Turn {
    /// Takes `more` of the answer's words and adds to `told` what the chat hears of them.
    // Human: a model's thinking should come beside its words, but not every model server tells the two apart.
    // Some write it among the words between <think> and </think>; and a small model in Ollama now and then
    // has its thinking arrive as words, with only the closing </think> to show it. Left alone, the chat shows
    // pages of thinking as the answer and the conversation carries them along in every later request. So:
    // words from a <think> on are thoughts, and when a </think> comes with nothing before it to open it,
    // what was said until then is taken back (`Event::Unsaid`). Only the answer after it is kept.
    pub(super) fn say(&mut self, more: &str, told: &mut Vec<Event>) {
        let said = self.text.len();
        self.text.push_str(more);
        if self.aside == Aside::Open {
            let begun = self.text.trim_start();
            if THINKS.starts_with(begun) {
                return;
            }
            // Everything held back so far is told now, as a thought or as words.
            if let Some(thought) = begun.strip_prefix(THINKS) {
                (self.aside, self.text) = (Aside::Thinking, thought.to_owned());
                return self.aside(0, told);
            }
            self.aside = Aside::Said;
            return self.aside(0, told);
        }
        self.aside(said, told);
    }

    /// Tells the chat about the words from byte `said` of the text on, which it hasn't heard of yet.
    fn aside(&mut self, said: usize, told: &mut Vec<Event>) {
        let new = self.text.get(said..).unwrap_or_default().to_owned();
        if self.aside == Aside::Closed {
            told.push(Event::Text(new));
            return;
        }
        // The closing may have come in pieces: it is looked for from a little before the new words.
        let from = (0..=said.saturating_sub(THOUGHT.len())).rev().find(|at| self.text.is_char_boundary(*at)).unwrap_or(0);
        let closed = self.text.get(from..).and_then(|tail| tail.find(THOUGHT)).map(|at| from + at);
        let Some(closed) = closed else {
            told.push(if self.aside == Aside::Thinking { Event::Thought(new) } else { Event::Text(new) });
            return;
        };
        let heard = self.text.get(..said).unwrap_or_default();
        if self.aside == Aside::Said && !heard.is_empty() {
            told.push(Event::Unsaid(heard.to_owned()));
        }
        let answer = self.text.get(closed + THOUGHT.len()..).unwrap_or_default().trim_start().to_owned();
        (self.aside, self.text) = (Aside::Closed, answer.clone());
        if !answer.is_empty() {
            told.push(Event::Text(answer));
        }
    }

    /// Tells the chat what the answer took, and keeps how full the model's context is with it. Thinking that
    /// was never closed is no answer and is dropped.
    pub(super) fn report(&mut self, tell: &dyn Fn(Event) -> bool) {
        if self.aside == Aside::Thinking {
            self.text.clear();
        }
        if let Some(report) = self.used.take() {
            self.filled = report.used.read().saturating_add(report.used.output);
            tell(Event::Used(report));
        }
    }

    /// Takes one event of the stream. Returns what the chat is told, or what went wrong.
    // Human: a tool call comes in pieces, each naming its place in the answer; a provider that leaves the
    // place out starts a new call with every piece that has an id.
    fn take(&mut self, event: &Value) -> Result<Vec<Event>, String> {
        if let Some(error) = event.get("error").filter(|error| !error.is_null()) {
            return Err(api::stream_error(error));
        }
        // Human: the usage comes in a chunk of its own with no choices, after the answer; some providers send
        // it with every chunk, growing. The last one said counts.
        if let Some(report) = event.get("usage").and_then(usage::completions) {
            self.used = Some(report);
        }
        let mut told = Vec::new();
        let text = |value: &Value, key: &str| value.get(key).and_then(Value::as_str).filter(|text| !text.is_empty()).map(str::to_owned);
        let Some(choice) = event.pointer("/choices/0") else { return Ok(told) };
        // Human: providers that let a model's thinking be read send it beside the answer, under one of two names.
        let delta = choice.get("delta");
        if let Some(more) = delta.and_then(|delta| text(delta, "reasoning_content").or_else(|| text(delta, "reasoning"))) {
            told.push(Event::Thought(more));
        }
        if let Some(more) = choice.get("delta").and_then(|delta| text(delta, "content")) {
            self.say(&more, &mut told);
        }
        for piece in choice.pointer("/delta/tool_calls").and_then(Value::as_array).into_iter().flatten() {
            let id = text(piece, "id");
            let at = match piece.get("index").and_then(Value::as_u64).and_then(|index| usize::try_from(index).ok()) {
                Some(index) => index,
                None if id.is_some() => self.calls.len(),
                None => self.calls.len().saturating_sub(1),
            };
            if at >= MOST_CALLS {
                return Err("the answer's tool calls came out of order".to_owned());
            }
            while self.calls.len() <= at {
                self.calls.push(ToolCall::default());
            }
            let call = &mut self.calls[at];
            // Human: the chat follows a call by its id; a provider that gives none gets one made here.
            match id {
                Some(id) => call.id = id,
                None if call.id.is_empty() => call.id = format!("call_{at}"),
                None => {}
            }
            let function = piece.get("function").unwrap_or(&Value::Null);
            if let Some(name) = text(function, "name").filter(|_| call.name.is_empty()) {
                call.name.clone_from(&name);
                told.push(Event::Tool { id: call.id.clone(), name });
            }
            call.arguments.push_str(function.get("arguments").and_then(Value::as_str).unwrap_or_default());
        }
        if let Some(reason) = text(choice, "finish_reason") {
            self.finish = Some(reason);
        }
        Ok(told)
    }

    /// The answer as it goes back to the API with the next request.
    pub(super) fn message(&self) -> Value {
        let calls: Vec<Value> = self
            .calls
            .iter()
            .map(|call| {
                let arguments = if call.arguments.trim().is_empty() { "{}" } else { call.arguments.as_str() };
                json!({ "id": call.id, "type": "function", "function": { "name": call.name, "arguments": arguments } })
            })
            .collect();
        let mut message = json!({ "role": "assistant", "content": if self.text.is_empty() { Value::Null } else { json!(self.text) } });
        if !calls.is_empty() {
            message["tool_calls"] = Value::Array(calls);
        }
        message
    }
}

/// Sends one request and takes in the streamed answer.
///
/// # Errors
/// The provider's status (0 when there is none) and what went wrong; nothing to say when the run was stopped.
// Agent: BLOCKS for as long as the model writes; READS the stop flag between events.
fn ask(
    agent: &ureq::Agent,
    job: &Job<'_>,
    bearer: Option<&str>,
    body: &Value,
    stop: &AtomicBool,
    tell: &dyn Fn(Event) -> bool,
) -> Result<Turn, (u16, String)> {
    let headers: Vec<(&str, &str)> = bearer.iter().map(|bearer| ("authorization", *bearer)).collect();
    let mut response = api::post(agent, job.endpoint, &headers, &body.to_string(), stop)?;
    let request = api::request_id(&response);
    let named = |status: u16, problem: String| (status, api::named(problem, request.as_deref()));
    let mut turn = Turn::default();
    for line in BufReader::new(response.body_mut().as_reader()).lines() {
        if stop.load(Ordering::Relaxed) {
            return Err((0, String::new()));
        }
        let line = line.map_err(|error| named(api::BROKE_OFF, format!("The connection broke while the model was answering: {error}")))?;
        let Some(data) = line.strip_prefix("data:").map(str::trim) else { continue };
        if data == "[DONE]" {
            break;
        }
        let Ok(event) = serde_json::from_str::<Value>(data) else { continue };
        for told in turn.take(&event).map_err(|error| named(api::broke(&event), error))? {
            if !tell(told) {
                return Err((0, String::new()));
            }
        }
    }
    turn.report(tell);
    Ok(turn)
}

/// Runs the tools an answer called and adds their results to `history`: each tool's words as its result, and
/// the pictures they made in a message after them (a tool's result is text only in this API).
fn use_tools(job: &Job<'_>, turn: &Turn, history: &mut Vec<Value>, blind: bool, stop: &AtomicBool, tell: &dyn Fn(Event) -> bool) -> bool {
    let mut pictures = Vec::new();
    for call in &turn.calls {
        if stop.load(Ordering::Relaxed) {
            return false;
        }
        let input = serde_json::from_str::<Value>(if call.arguments.trim().is_empty() { "{}" } else { &call.arguments });
        let mut failed = true;
        let said = match input {
            Ok(input) if input.is_object() => {
                tell(Event::ToolInput { id: call.id.clone(), input: input.clone() });
                match api::call_tool(job, &call.name, &input) {
                    Ok(content) => {
                        failed = false;
                        let mut words: Vec<&str> = content.iter().filter_map(|item| item["text"].as_str()).collect();
                        let made: Vec<Value> = content
                            .iter()
                            .filter(|item| item["type"] == "image")
                            .map(|item| json!({ "source": { "media_type": item["mimeType"], "data": item["data"] } }))
                            .collect();
                        if blind && !made.is_empty() {
                            words.push(NO_PICTURES);
                        }
                        pictures.extend(made);
                        words.join("\n")
                    }
                    Err(error) => error,
                }
            }
            _ => "The tool's input didn't arrive as complete JSON. Send the call again.".to_owned(),
        };
        tell(Event::ToolEnd { id: call.id.clone(), failed });
        history.push(json!({ "role": "tool", "tool_call_id": call.id, "content": said }));
    }
    if !blind && !pictures.is_empty() {
        history.push(user(TOOL_PICTURE, &pictures));
    }
    true
}

/// Adds the messages the user sent while the model worked to `history` and tells the chat that the model has
/// them now. Returns whether there was one.
fn heard(job: &Job<'_>, history: &mut Vec<Value>, tell: &dyn Fn(Event) -> bool) -> bool {
    let taken = std::mem::take(&mut *lock(job.steering));
    let any = !taken.is_empty();
    for steer in taken {
        history.push(user(&steer.prompt, &steer.pictures));
        tell(Event::Heard(steer.message));
    }
    any
}

/// What a request says to show the job's key, if the provider has one.
///
/// # Errors
/// A sentence for the user when the key couldn't be read.
pub(super) fn bearer(job: &Job<'_>) -> Result<Option<String>, String> {
    match job.key {
        KeySource::None => Ok(None),
        source => api::key(source).map(|key| Some(format!("Bearer {}", key.expose()))),
    }
}

/// Answers `job`, telling the chat what the model says and does. Returns when the model is done.
pub fn run(job: &Job<'_>, stop: &AtomicBool, tell: &dyn Fn(Event) -> bool) {
    let bearer = match bearer(job) {
        Ok(bearer) => bearer,
        Err(error) => {
            tell(Event::Problem(error));
            return;
        }
    };
    let agent = api::agent();
    converse(job, job.window, stop, tell, &|history| {
        asked(job.endpoint, &|counted| ask(&agent, job, bearer.as_deref(), &request(job, history, counted), stop, tell))
    });
}

/// Sends the conversation as it stands and gives the model's answer, or the provider's status (0 when there is
/// none) and what went wrong.
pub(super) type Ask<'a> = dyn Fn(&[Value]) -> Result<Turn, (u16, String)> + 'a;

/// Leads the conversation of `job`: `ask` sends it as it stands and gives the model's answer, the tools the
/// answer calls are run, and so on until the model has nothing more to do. `room` is how many tokens the
/// model reads at most (0 when that isn't known): a conversation that gets near it is shortened.
// Human: a model on one's own computer reads a few ten thousand tokens, and a long piece of work fills that
// with screenshots and tool results; the run then ended in the middle. See `shorten` for what goes.
// Human: a request the provider declines while pictures are in the conversation is sent once more without
// them: many models read no pictures, and the run would otherwise end at the first screenshot.
pub(super) fn converse(job: &Job<'_>, room: u64, stop: &AtomicBool, tell: &dyn Fn(Event) -> bool, ask: &Ask<'_>) {
    let mut history = job.history.clone();
    history.push(user(job.prompt, job.pictures));
    let mut blind = false;
    let mut tries = 0;
    for _ in 0..MOST_ROUNDS {
        let turn = match ask(&history) {
            Ok(turn) => turn,
            Err((status, error)) => {
                if stop.load(Ordering::Relaxed) || error.is_empty() {
                    return;
                }
                if !blind && matches!(status, 400 | 404 | 415 | 422) && without_pictures(&mut history) {
                    blind = true;
                    continue;
                }
                if api::again(status, &mut tries, stop, tell) {
                    continue;
                }
                tell(Event::Problem(error));
                break;
            }
        };
        tries = 0;
        if !turn.text.is_empty() || !turn.calls.is_empty() {
            history.push(turn.message());
        }
        if !turn.calls.is_empty() {
            if !use_tools(job, &turn, &mut history, blind, stop, tell) {
                return;
            }
            if crowded(turn.filled, room) && shorten(&mut history) {
                log::info!("the conversation filled {} of {room} tokens: older tool results were shortened", turn.filled);
            }
            tell(Event::Break);
            heard(job, &mut history, tell);
            continue;
        }
        match turn.finish.as_deref() {
            // Human: an answer that ends for its length ended because the context was full, where that is
            // nearly so. Then the conversation is shortened and the model asked again; only when there is
            // nothing left to shorten does the run end, and the user is told what was full.
            Some("length") if crowded(turn.filled, room) => {
                if shorten(&mut history) {
                    continue;
                }
                tell(Event::Problem(format!(
                    "The model ran out of room: the conversation fills all {room} tokens it can read here. Start a new chat, or ask for smaller steps."
                )));
                break;
            }
            Some("length") => {
                tell(Event::Problem("The answer was cut off at its length limit. Ask for the rest, or for smaller steps.".to_owned()));
                break;
            }
            Some("content_filter") => {
                tell(Event::Problem("The provider held this answer back.".to_owned()));
                break;
            }
            _ if heard(job, &mut history, tell) => {}
            _ => break,
        }
    }
    tell(Event::History(history));
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::*;
    use crate::chat::{BRIEF, Waiting};

    #[test]
    fn a_streamed_answer_becomes_words_and_whole_tool_calls() {
        let mut turn = Turn::default();
        let mut told = Vec::new();
        for event in [
            json!({ "choices": [{ "index": 0, "delta": { "role": "assistant", "content": "" } }] }),
            // What the model thinks comes under one of two names, where a provider lets it be read.
            json!({ "choices": [{ "index": 0, "delta": { "reasoning_content": "A card" } }] }),
            json!({ "choices": [{ "index": 0, "delta": { "reasoning": "." } }] }),
            json!({ "choices": [{ "index": 0, "delta": { "content": "On " } }] }),
            json!({ "choices": [{ "index": 0, "delta": { "content": "it." } }] }),
            json!({ "choices": [{ "index": 0, "delta": { "tool_calls": [{ "index": 0, "id": "c1", "type": "function", "function": { "name": "edit_layers", "arguments": "" } }] } }] }),
            json!({ "choices": [{ "index": 0, "delta": { "tool_calls": [{ "index": 0, "function": { "arguments": "{\"operations\":" } }] } }] }),
            json!({ "choices": [{ "index": 0, "delta": { "tool_calls": [{ "index": 0, "function": { "arguments": "[]}" } }] } }] }),
            json!({ "choices": [{ "index": 0, "delta": { "tool_calls": [{ "index": 1, "id": "c2", "function": { "name": "get_state" } }] } }] }),
            // A provider that names no place: an id starts a new call.
            json!({ "choices": [{ "index": 0, "delta": { "tool_calls": [{ "function": { "name": "screenshot", "arguments": "{}" } }] } }] }),
            json!({ "choices": [{ "index": 0, "delta": { "tool_calls": [{ "id": "c3", "function": { "name": "read_layers", "arguments": "{}" } }] } }] }),
            json!({ "choices": [{ "index": 0, "delta": {}, "finish_reason": "tool_calls" }] }),
            json!({ "choices": [], "usage": {} }),
        ] {
            told.extend(turn.take(&event).unwrap().iter().map(|e| format!("{e:?}")));
        }
        assert_eq!(
            told,
            vec![
                r#"Thought("A card")"#,
                r#"Thought(".")"#,
                r#"Text("On ")"#,
                r#"Text("it.")"#,
                r#"Tool { id: "c1", name: "edit_layers" }"#,
                r#"Tool { id: "c2", name: "get_state" }"#,
                r#"Tool { id: "c3", name: "read_layers" }"#
            ]
        );
        assert_eq!(turn.finish.as_deref(), Some("tool_calls"));
        let message = turn.message();
        assert_eq!((&message["role"], &message["content"]), (&json!("assistant"), &json!("On it.")));
        let calls = message["tool_calls"].as_array().unwrap();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0], json!({ "id": "c1", "type": "function", "function": { "name": "edit_layers", "arguments": "{\"operations\":[]}" } }));
        // A call without input goes back as an empty object, the way the API wants it.
        assert_eq!((&calls[1]["function"]["arguments"], &calls[2]["id"]), (&json!("{}"), &json!("c3")));
        assert_eq!(
            Turn::default().take(&json!({ "error": { "message": "Rate limited", "code": 429 } })).unwrap_err(),
            "The provider reported an error while the model was answering (429): Rate limited"
        );
        // Words alone go back without a list of calls.
        let plain = Turn { text: "Done.".to_owned(), ..Turn::default() };
        assert!(plain.message().get("tool_calls").is_none());
    }

    #[test]
    fn the_streams_last_chunk_says_what_the_answer_took() {
        let mut turn = Turn::default();
        for event in [
            json!({ "choices": [{ "index": 0, "delta": { "content": "Hi" } }] }),
            json!({ "choices": [{ "index": 0, "delta": {}, "finish_reason": "stop" }] }),
            json!({ "choices": [], "usage": { "prompt_tokens": 1000, "completion_tokens": 50, "prompt_tokens_details": { "cached_tokens": 800 } } }),
        ] {
            turn.take(&event).unwrap();
        }
        let report = turn.used.unwrap();
        assert_eq!(report.used, crate::chat::usage::Used { input: 200, cached: 800, written: 0, output: 50 });
        // A provider that says nothing, or an empty usage, reports nothing.
        let mut silent = Turn::default();
        silent.take(&json!({ "choices": [], "usage": {} })).unwrap();
        silent.take(&json!({ "choices": [], "usage": null })).unwrap();
        assert!(silent.used.is_none());
        // An address that refused the question once isn't asked it again.
        assert!(counts("https://a.example/v1/chat/completions"));
        uncounted("https://a.example/v1/chat/completions");
        assert!(!counts("https://a.example/v1/chat/completions") && counts("https://b.example/v1/chat/completions"));
    }

    #[test]
    fn a_request_refused_for_asking_its_usage_is_sent_once_more_without() {
        use std::cell::RefCell;
        // What the provider answers to each request in turn, and whether each request asked for its usage.
        let answers = |all: Vec<Result<(), u16>>| RefCell::new(all.into_iter().rev().collect::<Vec<_>>());
        let counted_ones = RefCell::new(Vec::new());
        let send = |from: &RefCell<Vec<Result<(), u16>>>, counted: bool| {
            counted_ones.borrow_mut().push(counted);
            match from.borrow_mut().pop() {
                Some(Ok(())) => Ok(Turn::default()),
                Some(Err(status)) => Err((status, format!("refused with {status}"))),
                None => Err((0, "nothing more to answer".to_owned())),
            }
        };
        let taken = || std::mem::take(&mut *counted_ones.borrow_mut());
        // Refused with the question, taken without: the address is remembered and no longer asked.
        let shy = "https://shy.asked.test/v1/chat/completions";
        let from = answers(vec![Err(400), Ok(()), Ok(())]);
        assert!(asked(shy, &|counted| send(&from, counted)).is_ok());
        assert_eq!((taken(), counts(shy)), (vec![true, false], false));
        assert!(asked(shy, &|counted| send(&from, counted)).is_ok());
        assert_eq!(taken(), vec![false]);
        // Refused both ways: the reason was another one. The second refusal is told and the next request asks again.
        let blind = "https://blind.asked.test/v1/chat/completions";
        let from = answers(vec![Err(422), Err(400), Ok(())]);
        assert_eq!(asked(blind, &|counted| send(&from, counted)).err(), Some((400, "refused with 400".to_owned())));
        assert_eq!((taken(), counts(blind)), (vec![true, false], true));
        assert!(asked(blind, &|counted| send(&from, counted)).is_ok());
        assert_eq!(taken(), vec![true]);
        // Any other refusal, and an answer, are taken as they are: nothing is sent again.
        let plain = "https://plain.asked.test/v1/chat/completions";
        let from = answers(vec![Err(500), Err(404), Ok(())]);
        assert_eq!(asked(plain, &|counted| send(&from, counted)).err().map(|(status, _)| status), Some(500));
        assert_eq!(asked(plain, &|counted| send(&from, counted)).err().map(|(status, _)| status), Some(404));
        assert!(asked(plain, &|counted| send(&from, counted)).is_ok());
        assert_eq!((taken(), counts(plain)), (vec![true, true, true], true));
        // Remembering an address twice keeps it once.
        uncounted(shy);
        assert_eq!(lock(&UNCOUNTED).iter().filter(|known| *known == shy).count(), 1);
    }

    #[test]
    fn requests_carry_the_brief_the_tools_and_pictures_the_apis_way() {
        let (tools, _calls) = channel();
        let inbox = api::Inbox::default();
        let job = Job {
            endpoint: "",
            official: false,
            model: "some-model",
            window: 0,
            key: &KeySource::None,
            instructions: "",
            off: &["export_layers".to_owned()],
            prompt: "",
            pictures: &[],
            history: Vec::new(),
            tools: &tools,
            wake: &|| {},
            chat: 1,
            steering: &inbox,
        };
        let seen = json!({ "type": "image", "source": { "type": "base64", "media_type": "image/jpeg", "data": "Zm9v" } });
        let mut history = vec![user("Like this", std::slice::from_ref(&seen)), user("and plain", &[])];
        let body = request(&job, &history, true);
        // The request asks for the usage to come with the stream, unless the provider won't have that.
        assert_eq!(body["stream_options"], json!({ "include_usage": true }));
        assert!(request(&job, &history, false).get("stream_options").is_none());
        assert_eq!((&body["model"], &body["stream"], &body["messages"][0]["role"]), (&json!("some-model"), &json!(true), &json!("system")));
        assert!(body["messages"][0]["content"].as_str().is_some_and(|system| system.starts_with(BRIEF)));
        assert_eq!(body["messages"][1]["content"][1], json!({ "type": "image_url", "image_url": { "url": "data:image/jpeg;base64,Zm9v" } }));
        assert_eq!(body["messages"][2]["content"], json!("and plain"));
        let all = body["tools"].as_array().unwrap();
        assert!(
            all.len() >= 8
                && all.iter().all(|tool| tool["function"]["name"] != "export_layers")
                && all.iter().all(|tool| tool["type"] == "function" && tool["function"]["parameters"]["type"] == "object")
        );
        // Without pictures the words stay and a note says what is missing; a second pass finds none.
        assert!(without_pictures(&mut history));
        assert_eq!(history[0]["content"], json!([{ "type": "text", "text": "Like this" }, { "type": "text", "text": NO_PICTURES }]));
        assert!(!without_pictures(&mut history));
        // A message sent meanwhile joins the conversation once.
        let message = Waiting { shown: "Wider".to_owned(), told: "Wider".to_owned(), files: Vec::new(), pasted: Vec::new(), pointed: Vec::new() };
        lock(&inbox).push(api::Steer { message, prompt: "note\n\nWider".to_owned(), pictures: Vec::new() });
        assert!(heard(&job, &mut history, &|_| true) && !heard(&job, &mut history, &|_| true));
        assert_eq!(history[2], json!({ "role": "user", "content": "note\n\nWider" }));
        assert_eq!(endpoint(" https://api.example.com/v1/ "), "https://api.example.com/v1/chat/completions");
    }

    /// Has a model in Ollama's cloud use a tool that makes a picture and say what is in it: the whole way a
    /// message goes, with the key Quill keeps in the keychain. Run by hand, naming that key and a model that
    /// reads pictures:
    /// `OLLAMA_CLOUD_KEY_NAME=provider-… OLLAMA_CLOUD_MODEL=… cargo test -- --ignored a_model_in_ollamas_cloud`.
    #[test]
    #[ignore = "talks to ollama.com on the key of the person running it"]
    fn a_model_in_ollamas_cloud_calls_tools_and_reads_their_pictures() {
        let key = KeySource::Keychain(std::env::var("OLLAMA_CLOUD_KEY_NAME").unwrap());
        let model = std::env::var("OLLAMA_CLOUD_MODEL").unwrap();
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(96, 96, image::Rgba([220, 30, 30, 255])).write_to(&mut png, image::ImageFormat::Png).unwrap();
        let picture = crate::agent::base64(&png.into_inner());
        let (tools, calls) = channel::<crate::link::Call>();
        // The app's part: every tool call is answered with a line of text and a red square.
        let app = std::thread::spawn(move || {
            for call in calls {
                let made = json!({ "type": "image", "mimeType": "image/png", "data": picture });
                let _gone = call.reply.send(Ok(vec![json!({ "type": "text", "text": "A picture of the document, 96 by 96." }), made]));
            }
        });
        let inbox = api::Inbox::default();
        let told = std::sync::Mutex::new(Vec::new());
        let job = Job {
            endpoint: &endpoint("https://ollama.com/v1"),
            official: false,
            model: &model,
            window: 0,
            key: &key,
            instructions: "",
            off: &[],
            prompt: "Call the screenshot tool once. Then tell me in one sentence which one colour fills the picture it made.",
            pictures: &[],
            history: Vec::new(),
            tools: &tools,
            wake: &|| {},
            chat: 1,
            steering: &inbox,
        };
        run(&job, &AtomicBool::new(false), &|event| {
            lock(&told).push(format!("{event:?}"));
            true
        });
        drop(tools);
        app.join().unwrap();
        let told = told.into_inner().unwrap();
        let words: String = told.iter().filter_map(|event| event.strip_prefix("Text(\"")).map(|word| word.trim_end_matches("\")")).collect();
        let kinds: Vec<&str> = told.iter().map(|event| event.split(['(', ' ']).next().unwrap_or_default()).collect();
        println!("{model}: {words}\n{kinds:?}");
        assert!(told.iter().all(|event| !event.starts_with("Problem")), "{told:?}");
        assert!(kinds.contains(&"Tool") && kinds.contains(&"Used") && kinds.last() == Some(&"History"), "{kinds:?}");
        // The model saw the picture itself, and its thinking is no part of what it said.
        assert!(words.to_lowercase().contains("red") && !words.contains("think>"), "{words}");
        assert!(told.last().is_some_and(|event| event.contains("image_url")), "the picture is in the conversation");
    }

    /// What the chat hears, and what is kept as the answer, when the model's words come in these pieces.
    fn said(pieces: &[&str]) -> (Vec<String>, String) {
        let mut turn = Turn::default();
        let mut told = Vec::new();
        for piece in pieces {
            turn.say(piece, &mut told);
        }
        turn.report(&|_| true);
        (told.iter().map(|event| format!("{event:?}")).collect(), turn.text)
    }

    #[test]
    fn thinking_among_the_words_is_no_part_of_the_answer() {
        let heard = |pieces: &[&str]| said(pieces).0.join(" ");
        // Plain words are the answer, also when they begin like a tag or hold one that opens nothing.
        assert_eq!(said(&["A ", "card."]), (vec![r#"Text("A ")"#.to_owned(), r#"Text("card.")"#.to_owned()], "A card.".to_owned()));
        assert_eq!(said(&["<", "b> is bold"]), (vec![r#"Text("<b> is bold")"#.to_owned()], "<b> is bold".to_owned()));
        // From <think> to </think> the words are thoughts, however the tags are cut; the answer follows.
        let (told, kept) = said(&["\n<thi", "nk>Hm, a ", "card.</th", "ink>\n\nA card", "."]);
        assert_eq!(kept, "A card.");
        assert_eq!(told.join(" "), r#"Thought("Hm, a ") Thought("card.</th") Text("A card") Text(".")"#);
        // Thinking that never closes leaves no answer.
        assert_eq!(said(&["<think>", "Hm"]).1, "");
        // A closing with nothing to open it: what was said until then is taken back, tag in pieces or not.
        let (told, kept) = said(&["So I call ", "get_state.\n</thi", "nk>", "Done."]);
        assert_eq!(kept, "Done.");
        assert_eq!(told.join(" "), r#"Text("So I call ") Text("get_state.\n</thi") Unsaid("So I call get_state.\n</thi") Text("Done.")"#);
        assert_eq!(heard(&["Hm.</think>"]), "");
        assert_eq!(heard(&["Hm.", "</think>The answer"]), r#"Text("Hm.") Unsaid("Hm.") Text("The answer")"#);
        // Only the first closing counts: a later one is the answer's own.
        assert_eq!(said(&["a</think>b", "</think>c"]).1, "b</think>c");
    }

    /// A conversation with three answers of the model: a long result and a picture after the first, a short
    /// result after the second.
    fn worked() -> Vec<Value> {
        let call = |id: &str| json!({ "role": "assistant", "content": "", "tool_calls": [{ "id": id, "type": "function", "function": { "name": "t", "arguments": "{}" } }] });
        let seen = json!({ "source": { "media_type": "image/png", "data": "Zm9v" } });
        vec![
            user("Make a card", std::slice::from_ref(&seen)),
            call("a"),
            json!({ "role": "tool", "tool_call_id": "a", "content": "ä".repeat(400) }),
            user(TOOL_PICTURE, std::slice::from_ref(&seen)),
            call("b"),
            json!({ "role": "tool", "tool_call_id": "b", "content": "ok" }),
            call("c"),
            json!({ "role": "tool", "tool_call_id": "c", "content": "x".repeat(900) }),
            user(TOOL_PICTURE, std::slice::from_ref(&seen)),
        ]
    }

    #[test]
    fn an_old_tool_result_is_shortened_and_the_latest_kept() {
        // Two thirds of the room is where shortening starts; an unknown room never is crowded.
        assert!(crowded(20, 30) && !crowded(19, 30) && !crowded(1_000_000, 0));
        let mut history = worked();
        let before = history.clone();
        assert!(shorten(&mut history));
        // The first answer's long result keeps its start, cut between letters, and its picture goes.
        let said = history[2]["content"].as_str().unwrap();
        assert!(said.starts_with(&"ä".repeat(120)) && said.ends_with(DROPPED) && said.len() < 400);
        assert_eq!(history[3], json!({ "role": "user", "content": PICTURE_DROPPED }));
        // What the user sent, the model's own words and everything from the last two answers on are as they were.
        for at in [0, 1, 4, 5, 6, 7, 8] {
            assert_eq!(history[at], before[at], "{at}");
        }
        // Once shortened there is nothing more to take; nor is there with fewer answers than are kept.
        assert!(!shorten(&mut history));
        assert!(!shorten(&mut before.into_iter().skip(5).collect::<Vec<_>>()));
    }

    #[test]
    fn a_full_context_is_shortened_and_the_model_asked_again() {
        let (tools, _calls) = channel::<crate::link::Call>();
        let inbox = api::Inbox::default();
        let job = Job {
            endpoint: "",
            official: false,
            model: "some-model",
            window: 0,
            key: &KeySource::None,
            instructions: "",
            off: &[],
            prompt: "Go on",
            pictures: &[],
            history: worked(),
            tools: &tools,
            wake: &|| {},
            chat: 1,
            steering: &inbox,
        };
        let run = |room: u64| {
            let asked = std::cell::Cell::new(0);
            let told = std::sync::Mutex::new(Vec::new());
            // The model's context is full every time it is asked.
            let ask = |_: &[Value]| {
                asked.set(asked.get() + 1);
                Ok(Turn { finish: Some("length".to_owned()), filled: 1000, ..Turn::default() })
            };
            converse(
                &job,
                room,
                &AtomicBool::new(false),
                &|event| {
                    lock(&told).push(format!("{event:?}"));
                    true
                },
                &ask,
            );
            (asked.get(), told.into_inner().unwrap())
        };
        // With a known room the conversation is shortened once and the model asked again; then the user is told.
        let (asked, told) = run(1000);
        assert_eq!(asked, 2);
        assert!(
            told.iter().any(|event| event.starts_with("Problem(\"The model ran out of room: the conversation fills all 1000 tokens")),
            "{told:?}"
        );
        assert!(told.last().is_some_and(|event| event.starts_with("History") && event.contains("dropped to save room")));
        // Without one, or far from full, a cut-off answer ends the run as before.
        for room in [0, 100_000] {
            let (asked, told) = run(room);
            assert_eq!(asked, 1);
            assert!(told.iter().any(|event| event.starts_with("Problem(\"The answer was cut off")), "{told:?}");
        }
    }
}
