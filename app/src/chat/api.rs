//! Answering by calling a provider's Messages API directly, with the user's key. What the two API engines
//! share (the connection, running a tool, the user's messages sent meanwhile) is here too.
//!
//! One message can take many requests: the model answers with words and tool calls, the app runs the tools
//! and sends their results back, until the model has nothing more to do. Answers are streamed, so the words
//! appear as they are written.

use std::io::{BufRead, BufReader};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use craft_ai_core::SecretString;
use serde_json::{Value, json};

use super::usage::{self, Report, Used};
use super::{Event, KeySource, MODELS, Waiting};
use crate::app::lock;
use crate::link::Call;
use crate::secrets::{Keychain, Vault};

/// The API version the requests are written for.
pub(super) const VERSION: &str = "2026-10-21";
/// The most the model may write in one answer. Streaming keeps a long answer from timing out.
const MAX_TOKENS: u32 = 64_000;
/// How hard the model thinks: enough for design work, quick enough for a chat.
const EFFORT: &str = "medium";
/// The beta that lets Anthropic's API hand a declined request to another model inside the same call.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
/// The models that take the `fallbacks` field.
const WITH_FALLBACK: [&str; 3] = ["claude-opus-5-5", "claude-fable-5-2", "claude-sonnet-5-5"];
/// The most requests one message may take, so a model going in circles ends.
pub(super) const MOST_ROUNDS: usize = 80;
/// How long a tool may take before the model is told it didn't answer.
const TOOL_PATIENCE: Duration = Duration::from_secs(130);
/// How often a request the provider couldn't take just now (too many requests, overloaded) is sent again.
const RETRIES: u32 = 2;
/// The name the model's cursor on the canvas gets.
const CURSOR_NAME: &str = "Assistant";

/// The most output tokens of a single question's answer ([`once`]).
const ONCE_TOKENS: u32 = 4000;

/// A message of the user's for a run that is already going.
pub struct Steer {
    /// The message, to show once the model has it, or to send later if the run ends first.
    pub message: Waiting,
    /// What the model reads: the app's note and the message.
    pub prompt: String,
    /// The pictures attached to it, as content blocks.
    pub pictures: Vec<Value>,
}

/// The messages handed to a run that it hasn't taken yet, oldest first.
pub type Inbox = Mutex<Vec<Steer>>;

/// One message to answer.
pub struct Job<'a> {
    /// The address messages are posted to.
    pub endpoint: &'a str,
    /// Whether that is Anthropic's own API.
    pub official: bool,
    /// The model.
    pub model: &'a str,
    /// Where the key is.
    pub key: &'a KeySource,
    /// What the user wants kept to in every conversation (the settings' custom instructions).
    pub instructions: &'a str,
    /// The tools the user has turned off, which the model isn't offered.
    pub off: &'a [String],
    /// The user's message, with the app's note before it.
    pub prompt: &'a str,
    /// The pictures attached to it, as content blocks.
    pub pictures: &'a [Value],
    /// The conversation so far, as the API takes it.
    pub history: Vec<Value>,
    /// Where tool calls go to be run on the main thread.
    pub tools: &'a Sender<Call>,
    /// Gets the app's attention after a tool call is sent.
    pub wake: &'a (dyn Fn() + Send + Sync),
    /// The chat's id in the app, which its tool calls carry.
    pub chat: u64,
    /// Messages the user sends while the model works.
    pub steering: &'a Inbox,
}

/// The key for `source`.
pub(super) fn key(source: &KeySource) -> Result<SecretString, String> {
    match source {
        KeySource::None => Err("This provider has no API key; add one in Settings → Agents.".to_owned()),
        KeySource::Environment => std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty())
            .map(|key| SecretString::new(key.trim()))
            .ok_or_else(|| "The ANTHROPIC_API_KEY environment variable is no longer set.".to_owned()),
        KeySource::Keychain(name) => {
            Keychain.get(name).map_err(|error| format!("The provider's API key couldn't be read from the keychain: {error}"))
        }
    }
}

/// The tools as the API takes them.
// Human: the tools' inputs are streamed as they are written (a whole board of layers is a long input), so
// the app checks each input itself before running the tool.
fn tools(official: bool, off: &[String]) -> Value {
    let all = crate::agent::tools_without(off);
    all.as_array()
        .into_iter()
        .flatten()
        .map(|tool| {
            let mut out = json!({ "name": tool["name"], "description": tool["description"], "input_schema": tool["inputSchema"] });
            if official {
                out["eager_input_streaming"] = json!(true);
            }
            out
        })
        .collect()
}

/// The request for the conversation `history`.
// Human: Anthropic's own API also gets what only it takes: caching of the unchanging start of the request
// (the brief, the guide, the tools), a level of effort, and for the models that have it a hand-over to
// another model when a request is declined. Other providers get the plain request.
fn request(job: &Job<'_>, history: &[Value]) -> Value {
    let mut body = json!({
        "model": job.model,
        "max_tokens": MAX_TOKENS,
        "stream": true,
        "system": super::brief(true, job.instructions),
        "tools": tools(job.official, job.off),
        "messages": history,
    });
    if job.official {
        body["cache_control"] = json!({ "type": "ephemeral" });
        if MODELS.iter().any(|(id, _)| *id == job.model) {
            body["output_config"] = json!({ "effort": EFFORT });
        }
        if WITH_FALLBACK.contains(&job.model) {
            body["fallbacks"] = json!("default");
        }
    }
    body
}

/// One answer of the model as it comes in: its blocks, kept whole so they can be sent back unchanged.
#[derive(Default)]
struct Turn {
    blocks: Vec<Value>,
    /// The JSON of each tool call's input as written so far, by block.
    inputs: Vec<String>,
    /// The tool calls whose input didn't come out as JSON.
    broken: Vec<String>,
    stop_reason: Option<String>,
    /// The tokens the answer took, as far as the stream has said.
    used: Used,
    /// The model the stream says is answering.
    model: Option<String>,
}

impl Turn {
    /// What the answer took; `None` when the provider said nothing about it.
    fn report(&self) -> Option<Report> {
        (!self.used.is_empty()).then(|| Report { used: self.used, model: self.model.clone(), billed: None })
    }

    /// Takes one event of the stream. Returns what the chat is told, or what went wrong.
    // Human: a thinking block comes with a signature and must go back to the API exactly as it came, so every
    // kind of block is kept as the stream builds it, not only the ones shown.
    fn take(&mut self, event: &Value) -> Result<Option<Event>, String> {
        let text = |pointer: &str| event.pointer(pointer).and_then(Value::as_str);
        let index = event.get("index").and_then(Value::as_u64).and_then(|i| usize::try_from(i).ok()).unwrap_or(usize::MAX);
        match text("/type") {
            Some("content_block_start") => {
                let block = event.get("content_block").cloned().unwrap_or(Value::Null);
                if index != self.blocks.len() {
                    return Err("the answer's parts came out of order".to_owned());
                }
                let word = |field: &str| block[field].as_str().unwrap_or_default().to_owned();
                let tool = (block["type"] == "tool_use").then(|| Event::Tool { id: word("id"), name: word("name") });
                // Human: a model whose thoughts are kept from the app still says that it thinks.
                let thinks = matches!(block["type"].as_str(), Some("thinking" | "redacted_thinking")).then(|| Event::Thought(word("thinking")));
                self.blocks.push(block);
                self.inputs.push(String::new());
                Ok(tool.or(thinks))
            }
            Some("content_block_delta") => {
                let (Some(block), Some(input)) = (self.blocks.get_mut(index), self.inputs.get_mut(index)) else { return Ok(None) };
                let append = |block: &mut Value, field: &str, more: &str| {
                    let so_far = block[field].as_str().unwrap_or_default().to_owned();
                    block[field] = Value::String(so_far + more);
                };
                match text("/delta/type") {
                    Some("text_delta") => {
                        let more = text("/delta/text").unwrap_or_default();
                        append(block, "text", more);
                        return Ok(Some(Event::Text(more.to_owned())));
                    }
                    Some("input_json_delta") => input.push_str(text("/delta/partial_json").unwrap_or_default()),
                    Some("thinking_delta") => {
                        let more = text("/delta/thinking").unwrap_or_default();
                        append(block, "thinking", more);
                        return Ok(Some(Event::Thought(more.to_owned())));
                    }
                    Some("signature_delta") => block["signature"] = json!(text("/delta/signature").unwrap_or_default()),
                    _ => {}
                }
                Ok(None)
            }
            Some("content_block_stop") => {
                let (Some(block), Some(input)) = (self.blocks.get_mut(index), self.inputs.get(index)) else { return Ok(None) };
                if block["type"] == "tool_use" {
                    // A call without input writes nothing; anything else must be one whole JSON object.
                    match serde_json::from_str::<Value>(if input.trim().is_empty() { "{}" } else { input }) {
                        Ok(parsed) if parsed.is_object() => block["input"] = parsed,
                        _ => {
                            block["input"] = json!({});
                            self.broken.push(block["id"].as_str().unwrap_or_default().to_owned());
                        }
                    }
                    // The chat's note on the call can now say what it is about.
                    return Ok(Some(Event::ToolInput { id: block["id"].as_str().unwrap_or_default().to_owned(), input: block["input"].clone() }));
                }
                Ok(None)
            }
            Some("message_start") => {
                usage::messages(&event["message"]["usage"], &mut self.used);
                self.model = text("/message/model").map(str::to_owned);
                // Human: what the model reads is known as the answer starts; the chat counts along from there.
                Ok((!self.used.is_empty()).then_some(Event::Meanwhile(self.used)))
            }
            Some("message_delta") => {
                usage::messages(&event["usage"], &mut self.used);
                if let Some(reason) = text("/delta/stop_reason") {
                    self.stop_reason = Some(reason.to_owned());
                }
                Ok(None)
            }
            Some("error") => Err(stream_error(&event["error"])),
            _ => Ok(None),
        }
    }
}

/// What a provider's `error` in the middle of a streamed answer says, as a sentence for the user: its words,
/// and the kind or code it gives.
// Human: the provider's words alone ("Internal Server Error") don't say who said them or when; a report of
// the problem needs both.
pub(super) fn stream_error(error: &Value) -> String {
    let said = error.as_str().or_else(|| error["message"].as_str()).map(str::trim).filter(|said| !said.is_empty()).unwrap_or("no reason given");
    let kind = [&error["type"], &error["code"]].into_iter().find_map(|kind| match kind {
        Value::String(kind) if !kind.is_empty() => Some(kind.clone()),
        Value::Number(code) => Some(code.to_string()),
        _ => None,
    });
    let kind = kind.map(|kind| format!(" ({kind})")).unwrap_or_default();
    format!("The provider reported an error while the model was answering{kind}: {said}")
}

/// The status an answer is given that began and then broke off: the provider reported an error in the middle
/// of the stream, or the connection broke. No provider answers with it.
pub(super) const BROKE_OFF: u16 = 1;

/// The status for what went wrong while the stream's `event` was taken in: [`BROKE_OFF`] when the event is the
/// provider reporting an error, 0 when the app couldn't make sense of the answer.
pub(super) fn broke(event: &Value) -> u16 {
    if event.get("error").is_some_and(|error| !error.is_null()) { BROKE_OFF } else { 0 }
}

/// Whether the same request is sent once more after an answer ended with `status`: it is when the answer broke
/// off and this is not the third time in a row (`tries` counts them). The chat is told, and a moment passes.
// Human: a provider's error in the middle of an answer is nearly always passing (an overloaded server, a model
// that wrote a tool call the provider couldn't read). Nothing of a broken answer was acted on: its tools run
// only once it is whole. So asking again loses nothing, and usually gets the answer.
// Agent: BLOCKS for a few seconds before a retry; READS the stop flag.
pub(super) fn again(status: u16, tries: &mut u32, stop: &AtomicBool, tell: &dyn Fn(Event) -> bool) -> bool {
    if status != BROKE_OFF || *tries >= RETRIES || stop.load(Ordering::Relaxed) {
        return false;
    }
    *tries += 1;
    if !tell(Event::Again(format!("The provider broke off its answer; asking again ({tries} of {RETRIES})"))) {
        return false;
    }
    std::thread::sleep(Duration::from_secs(u64::from(*tries) * 3));
    !stop.load(Ordering::Relaxed)
}

/// The name the provider gave this request, which its support can find it by; `None` when it gave none.
pub(super) fn request_id(response: &ureq::http::Response<ureq::Body>) -> Option<String> {
    ["request-id", "x-request-id"].into_iter().find_map(|name| response.headers().get(name)?.to_str().ok().map(str::to_owned))
}

/// `problem` with the request's name after it, when the provider gave one. A stopped run's empty problem stays empty.
pub(super) fn named(problem: String, request: Option<&str>) -> String {
    match request {
        Some(request) if !problem.is_empty() => format!("{problem} (request {request})"),
        _ => problem,
    }
}

/// What the provider said went wrong, from an answer that isn't a stream.
pub(super) fn refusal_of(status: u16, body: &str) -> String {
    // Human: most providers put what went wrong under "error" as an object with a message; Ollama puts the words there.
    let said = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.pointer("/error/message").or_else(|| value.get("error")).and_then(Value::as_str).map(str::to_owned));
    let said = said.unwrap_or_else(|| body.chars().take(300).collect());
    match status {
        401 | 403 => format!("The provider didn't accept the API key ({status}): {said}"),
        404 => format!("The provider doesn't know this address or model (404): {said}"),
        _ => format!("The provider answered with an error ({status}): {said}"),
    }
}

/// The connection requests are sent over.
pub(super) fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_recv_response(Some(Duration::from_secs(300)))
        .build()
        .into()
}

/// Posts the JSON `text` to `url` and gives the answer once the provider has accepted the request.
///
/// # Errors
/// The provider's status (0 when it wasn't reached) with a sentence for the user.
// Agent: BLOCKS until the provider answers; sleeps between the tries of a request it couldn't take just now.
pub(super) fn post(
    agent: &ureq::Agent,
    url: &str,
    headers: &[(&str, &str)],
    text: &str,
    stop: &AtomicBool,
) -> Result<ureq::http::Response<ureq::Body>, (u16, String)> {
    let mut attempt = 0;
    loop {
        let mut request = agent.post(url).header("content-type", "application/json");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let mut response = request.send(text).map_err(|error| (0, format!("The provider couldn't be reached: {error}")))?;
        let status = response.status().as_u16();
        if status == 200 {
            return Ok(response);
        }
        // Human: "too many requests" and "overloaded" pass; the same request is sent again after a moment.
        if (status == 429 || status >= 500) && attempt < RETRIES && !stop.load(Ordering::Relaxed) {
            attempt += 1;
            std::thread::sleep(Duration::from_secs(u64::from(attempt) * 3));
            continue;
        }
        let request = request_id(&response);
        let said = response.body_mut().read_to_string().unwrap_or_default();
        return Err((status, named(refusal_of(status, &said), request.as_deref())));
    }
}

/// Sends one request and takes in the streamed answer.
// Agent: BLOCKS for as long as the model writes; READS the stop flag between events.
fn ask(
    agent: &ureq::Agent,
    job: &Job<'_>,
    key: &SecretString,
    body: &Value,
    stop: &AtomicBool,
    tell: &dyn Fn(Event) -> bool,
) -> Result<Turn, (u16, String)> {
    let mut headers = vec![("anthropic-version", VERSION), ("x-api-key", key.expose())];
    if body.get("fallbacks").is_some() {
        headers.push(("anthropic-beta", FALLBACK_BETA));
    }
    let mut response = post(agent, job.endpoint, &headers, &body.to_string(), stop)?;
    let request = request_id(&response);
    let mut turn = Turn::default();
    for line in BufReader::new(response.body_mut().as_reader()).lines() {
        if stop.load(Ordering::Relaxed) {
            return Err((0, String::new()));
        }
        let line =
            line.map_err(|error| (BROKE_OFF, named(format!("The connection broke while the model was answering: {error}"), request.as_deref())))?;
        let Some(data) = line.strip_prefix("data:") else { continue };
        let Ok(event) = serde_json::from_str::<Value>(data.trim()) else { continue };
        if let Some(told) = turn.take(&event).map_err(|error| (broke(&event), named(error, request.as_deref())))?
            && !tell(told)
        {
            return Err((0, String::new()));
        }
    }
    // Human: an answer that broke off has returned above; its tokens are not counted, though the provider
    // may charge for them.
    if let Some(report) = turn.report() {
        tell(Event::Used(report));
    }
    Ok(turn)
}

/// Asks the model at `endpoint` one question with `system` as its whole brief and no tools, and gives its answer.
///
/// # Errors
/// When the provider couldn't be reached or refused.
// Agent: BLOCKS for one model answer.
pub fn once(endpoint: &str, source: &KeySource, model: &str, system: &str, content: &[Value], used: &dyn Fn(Report)) -> Result<String, String> {
    let key = key(source)?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_recv_response(Some(Duration::from_secs(180)))
        .build()
        .into();
    let body = json!({ "model": model, "max_tokens": ONCE_TOKENS, "system": system, "messages": [{ "role": "user", "content": content }] });
    let mut response = agent
        .post(endpoint)
        .header("content-type", "application/json")
        .header("anthropic-version", VERSION)
        .header("x-api-key", key.expose())
        .send(body.to_string().as_str())
        .map_err(|error| format!("The provider couldn't be reached: {error}"))?;
    let status = response.status().as_u16();
    let said = response.body_mut().read_to_string().unwrap_or_default();
    if status != 200 {
        return Err(refusal_of(status, &said));
    }
    let answer: Value = serde_json::from_str(&said).map_err(|error| format!("The provider's answer couldn't be read: {error}"))?;
    let mut taken = Used::default();
    usage::messages(&answer["usage"], &mut taken);
    if !taken.is_empty() {
        used(Report { used: taken, model: answer["model"].as_str().map(str::to_owned), billed: None });
    }
    Ok(answer["content"].as_array().into_iter().flatten().filter_map(|block| block["text"].as_str()).collect::<Vec<_>>().join(""))
}

/// Runs one tool on the main thread and gives what it answered: text and picture items, or what went wrong.
pub(super) fn call_tool(job: &Job<'_>, name: &str, input: &Value) -> Result<Vec<Value>, String> {
    let (reply, answered) = channel();
    let call = Call { tool: name.to_owned(), arguments: input.clone(), client: CURSOR_NAME.to_owned(), chat: Some(job.chat), reply };
    if job.tools.send(call).is_ok() {
        (job.wake)();
        answered.recv_timeout(TOOL_PATIENCE).unwrap_or_else(|_| Err("Pencraft didn't answer in time.".to_owned()))
    } else {
        Err("Pencraft is closing.".to_owned())
    }
}

/// Runs one tool and gives its result as the API takes it.
fn use_tool(job: &Job<'_>, id: &str, name: &str, input: &Value) -> Value {
    match call_tool(job, name, input) {
        Ok(content) => {
            // A picture travels as base64 with its kind; everything else is text.
            let content: Vec<Value> = content
                .iter()
                .map(|item| match item["type"].as_str() {
                    Some("image") => json!({ "type": "image", "source": { "type": "base64", "media_type": item["mimeType"], "data": item["data"] } }),
                    _ => json!({ "type": "text", "text": item["text"] }),
                })
                .collect();
            json!({ "type": "tool_result", "tool_use_id": id, "content": content })
        }
        Err(error) => json!({ "type": "tool_result", "tool_use_id": id, "content": error, "is_error": true }),
    }
}

/// Takes the messages the user sent while the model worked, as content blocks for the next request, and tells
/// the chat that the model has them now.
fn heard(job: &Job<'_>, tell: &dyn Fn(Event) -> bool) -> Vec<Value> {
    let taken = std::mem::take(&mut *lock(job.steering));
    let mut blocks = Vec::new();
    for steer in taken {
        blocks.extend(super::files::blocks(&steer.prompt, &steer.pictures));
        tell(Event::Heard(steer.message));
    }
    blocks
}

/// Answers `job`, telling the chat what the model says and does. Returns when the model is done.
// Human: every tool result of one answer goes back in one message, and the model's answer goes back exactly
// as it came. A stopped run leaves the conversation as it was before the message.
// Human: a message the user sends meanwhile goes to the model with the next request: after the tool results
// when it is using tools, or as a new message when it was about to finish.
pub fn run(job: &Job<'_>, stop: &AtomicBool, tell: &dyn Fn(Event) -> bool) {
    let key = match key(job.key) {
        Ok(key) => key,
        Err(error) => {
            tell(Event::Problem(error));
            return;
        }
    };
    let agent = agent();
    let mut history = job.history.clone();
    history.push(json!({ "role": "user", "content": super::files::content(job.prompt, job.pictures) }));
    let mut tries = 0;
    for _ in 0..MOST_ROUNDS {
        let turn = match ask(&agent, job, &key, &request(job, &history), stop, tell) {
            Ok(turn) => turn,
            Err((status, error)) => {
                if stop.load(Ordering::Relaxed) || error.is_empty() {
                    return;
                }
                if again(status, &mut tries, stop, tell) {
                    continue;
                }
                tell(Event::Problem(error));
                break;
            }
        };
        tries = 0;
        if !turn.blocks.is_empty() {
            history.push(json!({ "role": "assistant", "content": turn.blocks }));
        }
        match turn.stop_reason.as_deref() {
            Some("tool_use") => {
                let mut results = Vec::new();
                for block in turn.blocks.iter().filter(|block| block["type"] == "tool_use") {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }
                    let (id, name) = (block["id"].as_str().unwrap_or_default(), block["name"].as_str().unwrap_or_default());
                    let result = if turn.broken.iter().any(|broken| broken == id) {
                        json!({ "type": "tool_result", "tool_use_id": id, "content": "The tool's input didn't arrive as complete JSON. Send the call again.", "is_error": true })
                    } else {
                        use_tool(job, id, name, &block["input"])
                    };
                    tell(Event::ToolEnd { id: id.to_owned(), failed: result["is_error"] == true });
                    results.push(result);
                }
                tell(Event::Break);
                results.extend(heard(job, tell));
                history.push(json!({ "role": "user", "content": results }));
            }
            // The provider paused a long answer; asking again continues it.
            Some("pause_turn") => {}
            Some("refusal") => {
                tell(Event::Problem("The model declined this request.".to_owned()));
                break;
            }
            Some("max_tokens") => {
                tell(Event::Problem("The answer was cut off at its length limit. Ask for the rest, or for smaller steps.".to_owned()));
                break;
            }
            _ => {
                let more = heard(job, tell);
                if more.is_empty() {
                    break;
                }
                history.push(json!({ "role": "user", "content": more }));
            }
        }
    }
    tell(Event::History(history));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_streamed_answer_is_kept_whole_and_told_as_it_comes() {
        let mut turn = Turn::default();
        let mut told = Vec::new();
        for event in [
            json!({ "type": "message_start", "message": {} }),
            json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "thinking", "thinking": "", "signature": "" } }),
            json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "thinking_delta", "thinking": "A card." } }),
            json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "signature_delta", "signature": "sig" } }),
            json!({ "type": "content_block_stop", "index": 0 }),
            json!({ "type": "content_block_start", "index": 1, "content_block": { "type": "text", "text": "" } }),
            json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "text_delta", "text": "On " } }),
            json!({ "type": "content_block_delta", "index": 1, "delta": { "type": "text_delta", "text": "it." } }),
            json!({ "type": "content_block_stop", "index": 1 }),
            json!({ "type": "content_block_start", "index": 2, "content_block": { "type": "tool_use", "id": "t1", "name": "edit_layers", "input": {} } }),
            json!({ "type": "content_block_delta", "index": 2, "delta": { "type": "input_json_delta", "partial_json": "{\"operations\":[{\"op\":" } }),
            json!({ "type": "content_block_delta", "index": 2, "delta": { "type": "input_json_delta", "partial_json": "\"delete\",\"id\":\"a\"}]}" } }),
            json!({ "type": "content_block_stop", "index": 2 }),
            json!({ "type": "content_block_start", "index": 3, "content_block": { "type": "tool_use", "id": "t2", "name": "get_state", "input": {} } }),
            json!({ "type": "content_block_stop", "index": 3 }),
            json!({ "type": "content_block_start", "index": 4, "content_block": { "type": "tool_use", "id": "t3", "name": "read_layers", "input": {} } }),
            json!({ "type": "content_block_delta", "index": 4, "delta": { "type": "input_json_delta", "partial_json": "{\"ids\":[\"a" } }),
            json!({ "type": "content_block_stop", "index": 4 }),
            json!({ "type": "message_delta", "delta": { "stop_reason": "tool_use" }, "usage": {} }),
            json!({ "type": "message_stop" }),
        ] {
            told.extend(turn.take(&event).unwrap().map(|e| format!("{e:?}")));
        }
        let told: Vec<&str> = told.iter().map(|event| event.split([' ', '(']).next().unwrap_or_default()).collect();
        // That the model thinks is told when it starts to, and then what it thinks. Each call is told when it
        // starts, and again with its input once that is written out.
        assert_eq!(told, vec!["Thought", "Thought", "Text", "Text", "Tool", "ToolInput", "Tool", "ToolInput", "Tool", "ToolInput"]);
        assert_eq!(turn.blocks[0], json!({ "type": "thinking", "thinking": "A card.", "signature": "sig" }));
        assert_eq!(turn.blocks[1], json!({ "type": "text", "text": "On it." }));
        assert_eq!(turn.blocks[2]["input"], json!({ "operations": [{ "op": "delete", "id": "a" }] }));
        assert_eq!(turn.blocks[3]["input"], json!({}));
        // An input cut off before its end is not run.
        assert_eq!((turn.broken.as_slice(), turn.stop_reason.as_deref()), (&["t3".to_owned()][..], Some("tool_use")));
        assert_eq!(
            turn.take(&json!({ "type": "error", "error": { "type": "overloaded_error", "message": "Overloaded" } })).unwrap_err(),
            "The provider reported an error while the model was answering (overloaded_error): Overloaded"
        );
    }

    #[test]
    fn a_streamed_answer_says_what_it_took_and_which_model_answered() {
        let mut turn = Turn::default();
        for event in [
            json!({ "type": "message_start", "message": { "model": "claude-fable-5-2", "usage": { "input_tokens": 12, "cache_creation_input_tokens": 300, "cache_read_input_tokens": 9000, "output_tokens": 1 } } }),
            json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "text", "text": "" } }),
            json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "text_delta", "text": "Hi" } }),
            json!({ "type": "content_block_stop", "index": 0 }),
            json!({ "type": "message_delta", "delta": { "stop_reason": "end_turn" }, "usage": { "output_tokens": 420 } }),
        ] {
            turn.take(&event).unwrap();
        }
        let report = turn.report().unwrap();
        assert_eq!(report.used, crate::chat::usage::Used { input: 12, cached: 9000, written: 300, output: 420 });
        assert_eq!((report.model.as_deref(), report.billed), (Some("claude-fable-5-2"), None));
        // An answer that says nothing about usage reports nothing.
        assert!(Turn::default().report().is_none());
    }

    #[test]
    fn requests_carry_anthropics_extras_only_to_anthropic() {
        let (tools, _calls) = channel();
        let inbox = Inbox::default();
        let job = |official: bool, model: &'static str| Job {
            endpoint: "",
            official,
            model,
            key: &KeySource::Environment,
            instructions: "",
            off: &[],
            prompt: "",
            pictures: &[],
            history: Vec::new(),
            tools: &tools,
            wake: &|| {},
            chat: 1,
            steering: &inbox,
        };
        let history = [json!({ "role": "user", "content": "hi" })];
        let own = request(&job(true, "claude-opus-5-5"), &history);
        assert_eq!(
            (&own["fallbacks"], &own["output_config"]["effort"], &own["cache_control"]["type"]),
            (&json!("default"), &json!("medium"), &json!("ephemeral"))
        );
        assert_eq!((&own["stream"], &own["max_tokens"], &own["messages"][0]["content"]), (&json!(true), &json!(64_000), &json!("hi")));
        assert!(
            own["tools"]
                .as_array()
                .is_some_and(|tools| tools.len() >= 9 && tools.iter().all(|t| t["eager_input_streaming"] == true && t["input_schema"].is_object()))
        );
        assert!(
            own["system"]
                .as_str()
                .is_some_and(|system| system.starts_with(super::super::BRIEF) && system.contains("# Working on a Pencraft document"))
        );
        // No sampling settings, no thinking setting: the current models decide both themselves.
        assert!(own.get("temperature").is_none() && own.get("thinking").is_none());
        let haiku = request(&job(true, "claude-haiku-5-5"), &history);
        assert!(haiku.get("fallbacks").is_none() && haiku["output_config"]["effort"] == "medium");
        let other = request(&job(false, "claude-opus-5-5"), &history);
        assert!(other.get("fallbacks").is_none() && other.get("output_config").is_none() && other.get("cache_control").is_none());
        assert!(other["tools"][0].get("eager_input_streaming").is_none());
        let fewer = Job { off: &["screenshot".to_owned()], ..job(true, "claude-opus-5-5") };
        let offered = request(&fewer, &history);
        assert!(offered["tools"].as_array().is_some_and(|tools| tools.len() + 1 == own["tools"].as_array().map_or(0, Vec::len)));
        assert!(offered["tools"].as_array().is_some_and(|tools| tools.iter().all(|tool| tool["name"] != "screenshot")));
        // A message sent meanwhile is taken once: its pictures, then its words, and the chat is told.
        let picture = json!({ "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": "Zm9v" } });
        let message = Waiting { shown: "Wider".to_owned(), told: "Wider".to_owned(), files: Vec::new(), pasted: Vec::new(), pointed: Vec::new() };
        lock(&inbox).push(Steer { message, prompt: "note\n\nWider".to_owned(), pictures: vec![picture.clone()] });
        let told = std::cell::RefCell::new(Vec::new());
        let tell = |event: Event| {
            told.borrow_mut().push(format!("{event:?}"));
            true
        };
        let job = job(false, "m");
        assert_eq!(heard(&job, &tell), vec![picture, json!({ "type": "text", "text": "note\n\nWider" })]);
        assert!(told.borrow().len() == 1 && told.borrow()[0].starts_with(r#"Heard(Waiting { shown: "Wider""#), "{:?}", told.borrow());
        assert!(heard(&job, &tell).is_empty() && lock(&inbox).is_empty());
    }

    #[test]
    fn errors_say_what_to_look_at() {
        assert!(
            refusal_of(401, r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#)
                .contains("API key (401): invalid x-api-key")
        );
        assert!(refusal_of(404, "nope").contains("address or model (404): nope"));
        assert!(refusal_of(400, r#"{"error":{"message":"bad"}}"#).ends_with("(400): bad"));
    }
}
