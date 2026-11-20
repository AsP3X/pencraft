//! Answering by calling a provider's Chat Completions API: the one `OpenAI` introduced and most providers speak.
//!
//! The run is the same as with the Messages API ([`super::api`]): the model answers with words and tool calls,
//! the app runs the tools and sends their results back, until the model has nothing more to do. Only the shape
//! of the requests and of the streamed answers differs.

use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};

use super::api::{self, Job, MOST_ROUNDS};
use super::{Event, KeySource};
use crate::app::lock;

/// The most tool calls one answer may hold; a stream naming more is broken.
pub(super) const MOST_CALLS: usize = 64;
/// What stands in for a picture the provider's model can't take.
const NO_PICTURES: &str = "(A picture belongs here, but this model doesn't take pictures. Check your work with read_layers instead.)";

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

/// A message of the user's: the text alone, or the text and then the pictures.
fn user(text: &str, pictures: &[Value]) -> Value {
    if pictures.is_empty() {
        return json!({ "role": "user", "content": text });
    }
    let mut parts = vec![json!({ "type": "text", "text": text })];
    parts.extend(pictures.iter().map(picture));
    json!({ "role": "user", "content": parts })
}

/// The request for the conversation `history`.
fn request(job: &Job<'_>, history: &[Value]) -> Value {
    let mut messages = vec![json!({ "role": "system", "content": super::brief(true, job.instructions) })];
    messages.extend(history.iter().cloned());
    json!({ "model": job.model, "stream": true, "tools": tools(job.off), "messages": messages })
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

/// One tool call of an answer, as the stream builds it.
#[derive(Default)]
pub(super) struct ToolCall {
    pub(super) id: String,
    pub(super) name: String,
    /// The JSON of its input, as written so far.
    pub(super) arguments: String,
}

/// One answer of the model as it comes in.
#[derive(Default)]
pub(super) struct Turn {
    pub(super) text: String,
    pub(super) calls: Vec<ToolCall>,
    pub(super) finish: Option<String>,
}

impl Turn {
    /// Takes one event of the stream. Returns what the chat is told, or what went wrong.
    // Human: a tool call comes in pieces, each naming its place in the answer; a provider that leaves the
    // place out starts a new call with every piece that has an id.
    fn take(&mut self, event: &Value) -> Result<Vec<Event>, String> {
        if let Some(error) = event.get("error").filter(|error| !error.is_null()) {
            return Err(api::stream_error(error));
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
            self.text.push_str(&more);
            told.push(Event::Text(more));
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
    let named = |problem: String| (0, api::named(problem, request.as_deref()));
    let mut turn = Turn::default();
    for line in BufReader::new(response.body_mut().as_reader()).lines() {
        if stop.load(Ordering::Relaxed) {
            return Err((0, String::new()));
        }
        let line = line.map_err(|error| named(format!("The connection broke while the model was answering: {error}")))?;
        let Some(data) = line.strip_prefix("data:").map(str::trim) else { continue };
        if data == "[DONE]" {
            break;
        }
        let Ok(event) = serde_json::from_str::<Value>(data) else { continue };
        for event in turn.take(&event).map_err(named)? {
            if !tell(event) {
                return Err((0, String::new()));
            }
        }
    }
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
        history.push(user("The picture the tool made:", &pictures));
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
    converse(job, stop, tell, &|history| ask(&agent, job, bearer.as_deref(), &request(job, history), stop, tell));
}

/// Sends the conversation as it stands and gives the model's answer, or the provider's status (0 when there is
/// none) and what went wrong.
pub(super) type Ask<'a> = dyn Fn(&[Value]) -> Result<Turn, (u16, String)> + 'a;

/// Leads the conversation of `job`: `ask` sends it as it stands and gives the model's answer, the tools the
/// answer calls are run, and so on until the model has nothing more to do.
// Human: a request the provider declines while pictures are in the conversation is sent once more without
// them: many models read no pictures, and the run would otherwise end at the first screenshot.
pub(super) fn converse(job: &Job<'_>, stop: &AtomicBool, tell: &dyn Fn(Event) -> bool, ask: &Ask<'_>) {
    let mut history = job.history.clone();
    history.push(user(job.prompt, job.pictures));
    let mut blind = false;
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
                tell(Event::Problem(error));
                break;
            }
        };
        if !turn.text.is_empty() || !turn.calls.is_empty() {
            history.push(turn.message());
        }
        if !turn.calls.is_empty() {
            if !use_tools(job, &turn, &mut history, blind, stop, tell) {
                return;
            }
            tell(Event::Break);
            heard(job, &mut history, tell);
            continue;
        }
        match turn.finish.as_deref() {
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
    fn requests_carry_the_brief_the_tools_and_pictures_the_apis_way() {
        let (tools, _calls) = channel();
        let inbox = api::Inbox::default();
        let job = Job {
            endpoint: "",
            official: false,
            model: "some-model",
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
        let body = request(&job, &history);
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
        let message = Waiting { shown: "Wider".to_owned(), told: "Wider".to_owned(), files: Vec::new(), pointed: Vec::new() };
        lock(&inbox).push(api::Steer { message, prompt: "note\n\nWider".to_owned(), pictures: Vec::new() });
        assert!(heard(&job, &mut history, &|_| true) && !heard(&job, &mut history, &|_| true));
        assert_eq!(history[2], json!({ "role": "user", "content": "note\n\nWider" }));
        assert_eq!(endpoint(" https://api.example.com/v1/ "), "https://api.example.com/v1/chat/completions");
    }
}
