//! Answering by calling Ollama's own chat API, the one a model server on the user's computer speaks.
//!
//! Ollama also speaks the Chat Completions API ([`super::openai`]), but there it reads only as much of a
//! conversation as its default context holds (4096 tokens, less than the app's brief and tools alone) and takes
//! no word on that from the request. Its own API does, so the chat uses it. The run and the conversation's shape
//! are the same as with Chat Completions; only the requests and the streamed answers are translated.

use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};

use super::Event;
use super::api::{self, Job};
use super::openai::{self, MOST_CALLS, ToolCall, Turn};

/// How many tokens of a conversation the model is asked to read, where the model can read that many.
// Human: the brief and the tools are about 17 000 tokens; this leaves room for the work after them without
// asking a home computer for the memory a model's whole context would take.
const CONTEXT: u64 = 32_768;
/// Said when the model server doesn't answer at all.
pub(super) const NOT_RUNNING: &str = "Ollama isn't running. Open the Ollama app or run `ollama serve`, then try again.";

/// Where Ollama is, from the address `base` the user wrote: without the "/v1" its Chat Completions API has.
pub(super) fn root(base: &str) -> &str {
    let base = base.trim().trim_end_matches('/');
    base.strip_suffix("/v1").unwrap_or(base)
}

/// The address chat requests are posted to for an Ollama reached at `base`.
#[must_use]
pub fn endpoint(base: &str) -> String {
    format!("{}/api/chat", root(base))
}

/// The conversation `history`, kept in the Chat Completions shape, as Ollama's API takes it: a call's input
/// as an object, a tool's result under the tool's name, and pictures beside a message's words.
fn native(history: &[Value]) -> Vec<Value> {
    let mut names: HashMap<&str, &str> = HashMap::new();
    let mut out = Vec::with_capacity(history.len());
    for message in history {
        match message["role"].as_str() {
            Some("assistant") => {
                let mut calls = Vec::new();
                for call in message["tool_calls"].as_array().into_iter().flatten() {
                    let name = call.pointer("/function/name").and_then(Value::as_str).unwrap_or_default();
                    names.insert(call["id"].as_str().unwrap_or_default(), name);
                    let written = call.pointer("/function/arguments").and_then(Value::as_str).unwrap_or("{}");
                    let input = serde_json::from_str::<Value>(written).ok().filter(Value::is_object).unwrap_or_else(|| json!({}));
                    calls.push(json!({ "function": { "name": name, "arguments": input } }));
                }
                let mut told = json!({ "role": "assistant", "content": message["content"].as_str().unwrap_or_default() });
                if !calls.is_empty() {
                    told["tool_calls"] = Value::Array(calls);
                }
                out.push(told);
            }
            Some("tool") => {
                let name = names.get(message["tool_call_id"].as_str().unwrap_or_default()).copied().unwrap_or_default();
                out.push(json!({ "role": "tool", "tool_name": name, "content": message["content"] }));
            }
            _ => match message["content"].as_array() {
                Some(parts) => {
                    let words: Vec<&str> = parts.iter().filter_map(|part| part["text"].as_str()).collect();
                    let pictures: Vec<&str> = parts
                        .iter()
                        .filter_map(|part| part.pointer("/image_url/url").and_then(Value::as_str))
                        .filter_map(|url| url.split_once("base64,").map(|(_, data)| data))
                        .collect();
                    let mut told = json!({ "role": message["role"], "content": words.join("\n") });
                    if !pictures.is_empty() {
                        told["images"] = json!(pictures);
                    }
                    out.push(told);
                }
                None => out.push(message.clone()),
            },
        }
    }
    out
}

/// The request for the conversation `history`, read with a context of `context` tokens.
fn request(job: &Job<'_>, history: &[Value], context: u64) -> Value {
    let mut messages = vec![json!({ "role": "system", "content": super::brief(true, job.instructions) })];
    messages.extend(native(history));
    json!({ "model": job.model, "stream": true, "tools": openai::tools(job.off), "messages": messages, "options": { "num_ctx": context } })
}

/// Takes one line of the streamed answer into `turn`. Returns what the chat is told, or what went wrong.
// Human: unlike Chat Completions, a tool call arrives whole, its input already an object, and often without an
// id; the chat follows a call by its id, so one is made here.
fn take(turn: &mut Turn, event: &Value) -> Result<Vec<Event>, String> {
    if let Some(error) = event.get("error").filter(|error| !error.is_null()) {
        return Err(error.as_str().or_else(|| error["message"].as_str()).unwrap_or("Ollama reported an error").to_owned());
    }
    let mut told = Vec::new();
    if let Some(more) = event.pointer("/message/thinking").and_then(Value::as_str).filter(|more| !more.is_empty()) {
        told.push(Event::Thought(more.to_owned()));
    }
    if let Some(more) = event.pointer("/message/content").and_then(Value::as_str).filter(|more| !more.is_empty()) {
        turn.text.push_str(more);
        told.push(Event::Text(more.to_owned()));
    }
    for call in event.pointer("/message/tool_calls").and_then(Value::as_array).into_iter().flatten() {
        if turn.calls.len() >= MOST_CALLS {
            return Err("the answer holds more tool calls than can be run".to_owned());
        }
        let name = call.pointer("/function/name").and_then(Value::as_str).unwrap_or_default().to_owned();
        let given = call.get("id").and_then(Value::as_str).filter(|id| !id.is_empty());
        let id = given.map_or_else(|| format!("call_{}", turn.calls.len()), str::to_owned);
        let arguments = match call.pointer("/function/arguments") {
            Some(Value::String(written)) => written.clone(),
            Some(input) => input.to_string(),
            None => String::new(),
        };
        told.push(Event::Tool { id: id.clone(), name: name.clone() });
        turn.calls.push(ToolCall { id, name, arguments });
    }
    if let Some(reason) = event.get("done_reason").and_then(Value::as_str).filter(|reason| !reason.is_empty()) {
        turn.finish = Some(reason.to_owned());
    }
    Ok(told)
}

/// How many tokens `model` is asked to read: [`CONTEXT`], or all the model can where that is less.
// Agent: BLOCKS on one short question to Ollama; any failure gives the usual number.
fn context(agent: &ureq::Agent, job: &Job<'_>, headers: &[(&str, &str)], stop: &AtomicBool) -> u64 {
    let url = format!("{}/api/show", job.endpoint.trim_end_matches("/api/chat"));
    let Ok(mut response) = api::post(agent, &url, headers, &json!({ "model": job.model }).to_string(), stop) else { return CONTEXT };
    let said = response.body_mut().read_to_string().unwrap_or_default();
    most(&said).map_or(CONTEXT, |most| most.min(CONTEXT))
}

/// The longest conversation a model can read, from what Ollama says about it.
fn most(shown: &str) -> Option<u64> {
    let shown = serde_json::from_str::<Value>(shown).ok()?;
    let facts = shown.get("model_info")?.as_object()?;
    facts.iter().find(|(name, _)| name.ends_with(".context_length")).and_then(|(_, length)| length.as_u64()).filter(|length| *length > 0)
}

/// Sends one request and takes in the streamed answer.
///
/// # Errors
/// Ollama's status (0 when there is none) and what went wrong; nothing to say when the run was stopped.
// Agent: BLOCKS for as long as the model writes; READS the stop flag between lines.
fn ask(
    agent: &ureq::Agent,
    job: &Job<'_>,
    headers: &[(&str, &str)],
    body: &Value,
    stop: &AtomicBool,
    tell: &dyn Fn(Event) -> bool,
) -> Result<Turn, (u16, String)> {
    // Human: no answer at all from a model server on one's own computer nearly always means it isn't running.
    let unreached =
        |(status, said): (u16, String)| if status == 0 && !said.is_empty() { (0, format!("{NOT_RUNNING} ({said})")) } else { (status, said) };
    let mut response = api::post(agent, job.endpoint, headers, &body.to_string(), stop).map_err(unreached)?;
    let mut turn = Turn::default();
    for line in BufReader::new(response.body_mut().as_reader()).lines() {
        if stop.load(Ordering::Relaxed) {
            return Err((0, String::new()));
        }
        let line = line.map_err(|error| (0, format!("The connection broke while the model was answering: {error}")))?;
        let Ok(event) = serde_json::from_str::<Value>(line.trim()) else { continue };
        for event in take(&mut turn, &event).map_err(|error| (0, error))? {
            if !tell(event) {
                return Err((0, String::new()));
            }
        }
        if event["done"] == true {
            break;
        }
    }
    Ok(turn)
}

/// Answers `job`, telling the chat what the model says and does. Returns when the model is done.
pub fn run(job: &Job<'_>, stop: &AtomicBool, tell: &dyn Fn(Event) -> bool) {
    let bearer = match openai::bearer(job) {
        Ok(bearer) => bearer,
        Err(error) => {
            tell(Event::Problem(error));
            return;
        }
    };
    let headers: Vec<(&str, &str)> = bearer.iter().map(|bearer| ("authorization", bearer.as_str())).collect();
    let agent = api::agent();
    let context = context(&agent, job, &headers, stop);
    openai::converse(job, stop, tell, &|history| ask(&agent, job, &headers, &request(job, history, context), stop, tell));
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::*;
    use crate::chat::KeySource;

    #[test]
    fn a_conversation_is_told_ollamas_way() {
        let history = vec![
            json!({ "role": "user", "content": [
                { "type": "text", "text": "Like this" },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,Zm9v" } },
            ] }),
            json!({ "role": "assistant", "content": null, "tool_calls": [
                { "id": "c1", "type": "function", "function": { "name": "get_state", "arguments": "{\"deep\":true}" } },
                { "id": "c2", "type": "function", "function": { "name": "screenshot", "arguments": "not json" } },
            ] }),
            json!({ "role": "tool", "tool_call_id": "c1", "content": "One document." }),
            json!({ "role": "tool", "tool_call_id": "c2", "content": "A picture." }),
            json!({ "role": "user", "content": "and plain" }),
        ];
        let told = native(&history);
        assert_eq!(told[0], json!({ "role": "user", "content": "Like this", "images": ["Zm9v"] }));
        assert_eq!(told[1]["content"], json!(""));
        assert_eq!(told[1]["tool_calls"][0], json!({ "function": { "name": "get_state", "arguments": { "deep": true } } }));
        assert_eq!(told[1]["tool_calls"][1]["function"]["arguments"], json!({}));
        assert_eq!(told[2], json!({ "role": "tool", "tool_name": "get_state", "content": "One document." }));
        assert_eq!((&told[3]["tool_name"], &told[4]), (&json!("screenshot"), &history[4]));
        // The request asks for a context that holds the brief, and carries the tools.
        let (tools, _calls) = channel();
        let inbox = api::Inbox::default();
        let job = Job {
            endpoint: "",
            official: false,
            model: "some-model",
            key: &KeySource::None,
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
        let body = request(&job, &history, 8192);
        assert_eq!((&body["model"], &body["stream"], &body["options"]["num_ctx"]), (&json!("some-model"), &json!(true), &json!(8192)));
        assert_eq!((&body["messages"][0]["role"], body["messages"].as_array().map(Vec::len)), (&json!("system"), Some(6)));
        assert!(body["tools"].as_array().is_some_and(|all| all.len() >= 8));
        assert_eq!(endpoint(" http://localhost:11434/v1/ "), "http://localhost:11434/api/chat");
        assert_eq!(endpoint("http://box:11434"), "http://box:11434/api/chat");
    }

    #[test]
    fn a_streamed_answer_becomes_words_and_whole_tool_calls() {
        let mut turn = Turn::default();
        let mut told = Vec::new();
        for event in [
            json!({ "message": { "role": "assistant", "content": "", "thinking": "Hm" }, "done": false }),
            json!({ "message": { "role": "assistant", "content": "On " }, "done": false }),
            json!({ "message": { "role": "assistant", "content": "it." }, "done": false }),
            json!({ "message": { "role": "assistant", "content": "", "tool_calls": [
                { "function": { "name": "get_state", "arguments": {} } },
                { "id": "c9", "function": { "name": "edit_layers", "arguments": { "operations": [] } } },
            ] }, "done": false }),
            json!({ "message": { "role": "assistant", "content": "" }, "done": true, "done_reason": "stop" }),
        ] {
            told.extend(take(&mut turn, &event).unwrap().iter().map(|e| format!("{e:?}")));
        }
        assert_eq!(
            told,
            vec![
                r#"Thought("Hm")"#,
                r#"Text("On ")"#,
                r#"Text("it.")"#,
                r#"Tool { id: "call_0", name: "get_state" }"#,
                r#"Tool { id: "c9", name: "edit_layers" }"#
            ]
        );
        assert_eq!((turn.finish.as_deref(), turn.text.as_str()), (Some("stop"), "On it."));
        // The answer goes back into the conversation the way every other one does.
        assert_eq!(turn.message()["tool_calls"][1]["function"]["arguments"], json!("{\"operations\":[]}"));
        assert_eq!(take(&mut Turn::default(), &json!({ "error": "model 'x' not found" })).unwrap_err(), "model 'x' not found");
        // A model reads what it can, and no more than the app asks for.
        assert_eq!(most(r#"{"model_info":{"general.architecture":"qwen3","qwen3.context_length":40960}}"#), Some(40960));
        assert_eq!((most(r#"{"model_info":{}}"#), most("no")), (None, None));
    }

    /// Runs one message through an Ollama on this computer, with its first model, and gives what the chat was told.
    fn told_by_ollama(prompt: &str) -> Vec<String> {
        let base = "http://localhost:11434";
        let models = crate::chat::catalog::list_models(crate::chat::OLLAMA_API, base, None).unwrap();
        let (tools, calls) = channel::<crate::link::Call>();
        // The app's part: every tool call is answered with a line of text.
        let app = std::thread::spawn(move || {
            for call in calls {
                let _gone =
                    call.reply.send(Ok(vec![json!({ "type": "text", "text": "One document is open: Untitled, 1 frame, nothing selected." })]));
            }
        });
        let inbox = api::Inbox::default();
        let told = std::sync::Mutex::new(Vec::new());
        let job = Job {
            endpoint: &endpoint(base),
            official: false,
            model: &models[0],
            key: &KeySource::None,
            instructions: "",
            off: &[],
            prompt,
            pictures: &[],
            history: Vec::new(),
            tools: &tools,
            wake: &|| {},
            chat: 1,
            steering: &inbox,
        };
        run(&job, &AtomicBool::new(false), &|event| {
            crate::app::lock(&told).push(format!("{event:?}"));
            true
        });
        drop(tools);
        app.join().unwrap();
        told.into_inner().unwrap()
    }

    #[test]
    #[ignore = "talks to an Ollama on this computer that has a model with tool calling"]
    fn a_model_in_ollama_answers_and_calls_tools() {
        let told = told_by_ollama("Call the get_state tool once, then tell me in one sentence what it answered.");
        let words: String = told.iter().filter_map(|event| event.strip_prefix("Text(\"")).map(|word| word.trim_end_matches("\")")).collect();
        println!("{words}");
        assert!(told.iter().all(|event| !event.starts_with("Problem")), "{told:?}");
        assert!(told.iter().any(|event| event.starts_with("Tool {")) && !words.is_empty());
        assert!(told.last().is_some_and(|event| event.starts_with("History")));
    }
}
