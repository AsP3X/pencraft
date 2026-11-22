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

/// How many tokens of a conversation the model is asked to read at least, where the model can read that many.
// Human: the brief and the tools are about 20 000 tokens; this leaves some room for the work after them on
// any computer. Where more is known to fit (see `fitting`), more is asked for.
const CONTEXT: u64 = 32_768;
/// The most tokens asked for however much memory there is: beyond this the memory is better left to the user.
const MOST_CONTEXT: u64 = 131_072;
/// Contexts are asked for in steps of this many tokens.
const STEP: u64 = 4_096;
/// The memory left to everything else on the computer when a context is sized to it, in bytes.
const SPARE: u64 = 2 << 30;
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
                Some(parts) => out.extend(beside(&message["role"], parts)),
                None => out.push(message.clone()),
            },
        }
    }
    out
}

/// A message whose content is `parts` (words and pictures in the Chat Completions shape) as Ollama takes it:
/// the words with the pictures beside them.
// Human: Ollama takes a message's pictures beside its words, not among them. So a message with pictures
// in the middle of its words goes as several, cut behind each run of pictures: every picture then still
// follows the words that name it ("like [Image 1]", its picture, "but coloured like [Image 2]", its picture).
fn beside(role: &Value, parts: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    let (mut words, mut pictures): (Vec<&str>, Vec<&str>) = (Vec::new(), Vec::new());
    let mut close = |words: &mut Vec<&str>, pictures: &mut Vec<&str>| {
        let mut told = json!({ "role": role, "content": words.join("\n") });
        if !pictures.is_empty() {
            told["images"] = json!(pictures);
        }
        out.push(told);
        words.clear();
        pictures.clear();
    };
    for part in parts {
        if let Some(text) = part["text"].as_str() {
            if !pictures.is_empty() {
                close(&mut words, &mut pictures);
            }
            words.push(text);
        } else if let Some((_, data)) = part.pointer("/image_url/url").and_then(Value::as_str).and_then(|url| url.split_once("base64,")) {
            pictures.push(data);
        }
    }
    close(&mut words, &mut pictures);
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
        return Err(api::stream_error(error));
    }
    let mut told = Vec::new();
    if let Some(more) = event.pointer("/message/thinking").and_then(Value::as_str).filter(|more| !more.is_empty()) {
        told.push(Event::Thought(more.to_owned()));
    }
    if let Some(more) = event.pointer("/message/content").and_then(Value::as_str).filter(|more| !more.is_empty()) {
        turn.say(more, &mut told);
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
    if event["done"] == true {
        turn.used = super::usage::ollama(event);
    }
    Ok(told)
}

/// How many tokens `model` is asked to read: the number the user set for it in the settings, or as many as
/// suit this computer ([`suggested`]) where they set none.
// Agent: BLOCKS on two short questions to Ollama and on `sysctl`; any failure gives the usual number.
fn context(agent: &ureq::Agent, job: &Job<'_>, headers: &[(&str, &str)], stop: &AtomicBool) -> u64 {
    let base = job.endpoint.trim_end_matches("/api/chat");
    let Ok(mut response) = api::post(agent, &format!("{base}/api/show"), headers, &json!({ "model": job.model }).to_string(), stop) else {
        return CONTEXT;
    };
    let said = response.body_mut().read_to_string().unwrap_or_default();
    asked(job.window, most(&said), suggested(agent, base, job.model, &said))
}

/// What is asked for a model that reads `most` tokens at most, with `window` set in the settings and
/// `suggested` for this computer.
// Human: the settings' "Context window" is the user's to set, and what they typed is what Ollama is asked
// for. But the form used to fill in all a model can read (262 144 tokens, say), and a row added by hand
// starts with a number larger still; asking for that would take tens of gigabytes. So a number that is all
// the model can read, or more, counts as not set.
fn asked(window: u64, most: Option<u64>, suggested: u64) -> u64 {
    match most {
        Some(most) if window > 0 && window < most => window,
        _ => suggested,
    }
}

/// How many tokens suit a conversation with `model`, which the Ollama at `base` shows as `shown`: as many as
/// fit beside the model in this computer's memory, [`CONTEXT`] at least, and never more than the model can.
/// The settings' form fills this in for a model, and it is asked for where the user set nothing else.
// Human: 32 768 tokens was the same on every computer, and a third of it was left for the work; a model that
// thinks at length ran out in the middle of a thought. A Mac with more memory holds a larger context, so the
// number follows the memory. Only an Ollama on this computer is sized: of another machine nothing is known.
// Agent: BLOCKS on one short question to Ollama and on `sysctl`.
pub(super) fn suggested(agent: &ureq::Agent, base: &str, model: &str, shown: &str) -> u64 {
    let fits = super::catalog::is_local_ollama(base)
        .then(|| fitting(memory()?, weights(agent, base, model)?, per_token(shown)?))
        .flatten()
        .unwrap_or(CONTEXT);
    most(shown).map_or(fits, |most| most.min(fits))
}

/// How many tokens fit in a computer with `memory` bytes beside a model of `weights` bytes whose context
/// takes `per_token` bytes a token: between [`CONTEXT`] and [`MOST_CONTEXT`], in whole [`STEP`]s.
// Human: on a Mac the graphics chip may use about two thirds of the memory; a model that needs more is run
// partly on the processor and gets much slower. So two thirds, less what the rest of the computer needs.
fn fitting(memory: u64, weights: u64, per_token: u64) -> Option<u64> {
    let room = (memory / 3 * 2).saturating_sub(SPARE).saturating_sub(weights);
    let tokens = room.checked_div(per_token)? / STEP * STEP;
    Some(tokens.clamp(CONTEXT, MOST_CONTEXT))
}

/// The bytes one token of context takes in memory, from what Ollama says about a model: a key and a value
/// (two bytes a number) for each of its attention heads that keep them, in each of its layers.
fn per_token(shown: &str) -> Option<u64> {
    let shown = serde_json::from_str::<Value>(shown).ok()?;
    let facts = shown.get("model_info")?.as_object()?;
    let fact = |name: &str| facts.iter().find(|(key, _)| key.ends_with(name)).and_then(|(_, value)| value.as_u64()).filter(|count| *count > 0);
    let numbers = fact(".attention.key_length")?.checked_add(fact(".attention.value_length")?)?;
    fact(".block_count")?.checked_mul(fact(".attention.head_count_kv")?)?.checked_mul(numbers)?.checked_mul(2)
}

/// The size of `model`'s file in bytes, from Ollama's list of its models.
// Agent: BLOCKS on one short question to Ollama.
fn weights(agent: &ureq::Agent, base: &str, model: &str) -> Option<u64> {
    let mut response = agent.get(format!("{base}/api/tags")).call().ok()?;
    let listed = serde_json::from_str::<Value>(&response.body_mut().read_to_string().ok()?).ok()?;
    let found = listed["models"].as_array()?.iter().find(|known| known["name"] == model || known["model"] == model)?;
    found["size"].as_u64().filter(|size| *size > 0)
}

/// The memory of this computer in bytes, where a model's context shares it with everything else: a Mac with
/// Apple's own chip. `None` elsewhere, where a graphics card's own memory decides and isn't known here.
// Agent: RUNS `sysctl -n hw.memsize`, which answers at once.
fn memory() -> Option<u64> {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return None;
    }
    let told = std::process::Command::new("/usr/sbin/sysctl").args(["-n", "hw.memsize"]).output().ok()?;
    String::from_utf8_lossy(&told.stdout).trim().parse().ok().filter(|bytes| *bytes > 0)
}

/// The longest conversation a model can read, from what Ollama says about it.
pub(super) fn most(shown: &str) -> Option<u64> {
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
        let line = line.map_err(|error| (api::BROKE_OFF, format!("The connection broke while the model was answering: {error}")))?;
        let Ok(event) = serde_json::from_str::<Value>(line.trim()) else { continue };
        for told in take(&mut turn, &event).map_err(|error| (api::broke(&event), error))? {
            if !tell(told) {
                return Err((0, String::new()));
            }
        }
        if event["done"] == true {
            break;
        }
    }
    turn.report(tell);
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
    openai::converse(job, context, stop, tell, &|history| ask(&agent, job, &headers, &request(job, history, context), stop, tell));
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
            json!({ "role": "user", "content": [
                { "type": "text", "text": "Like [Image 1]" },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,b25l" } },
                { "type": "text", "text": " but coloured like [Image 2]" },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,dHdv" } },
                { "type": "text", "text": ", please" },
                { "type": "image_url", "image_url": { "url": "data:image/png;base64,Zm9v" } },
            ] }),
        ];
        let told = native(&history);
        // Pictures among the words: the message is cut behind each, so every picture follows its name.
        assert_eq!(
            told[5..],
            [
                json!({ "role": "user", "content": "Like [Image 1]", "images": ["b25l"] }),
                json!({ "role": "user", "content": " but coloured like [Image 2]", "images": ["dHdv"] }),
                json!({ "role": "user", "content": ", please", "images": ["Zm9v"] }),
            ]
        );
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
            window: 0,
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
        assert_eq!((&body["messages"][0]["role"], body["messages"].as_array().map(Vec::len)), (&json!("system"), Some(9)));
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
            json!({ "message": { "role": "assistant", "content": "" }, "done": true, "done_reason": "stop", "prompt_eval_count": 17000, "eval_count": 90 }),
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
        // The last line says what the answer took.
        assert_eq!(turn.used.as_ref().map(|report| (report.used.input, report.used.output)), Some((17_000, 90)));
        // The answer goes back into the conversation the way every other one does.
        assert_eq!(turn.message()["tool_calls"][1]["function"]["arguments"], json!("{\"operations\":[]}"));
        assert_eq!(
            take(&mut Turn::default(), &json!({ "error": "model 'x' not found" })).unwrap_err(),
            "The provider reported an error while the model was answering: model 'x' not found"
        );
        // A model reads what it can, and no more than the app asks for.
        assert_eq!(most(r#"{"model_info":{"general.architecture":"qwen3","qwen3.context_length":40960}}"#), Some(40960));
        assert_eq!((most(r#"{"model_info":{}}"#), most("no")), (None, None));
    }

    #[test]
    fn the_context_follows_the_memory() {
        // A model of 36 layers with 8 heads that keep a key and a value of 128 numbers each, two bytes a number.
        let shown =
            r#"{"model_info":{"m.block_count":36,"m.attention.head_count_kv":8,"m.attention.key_length":128,"m.attention.value_length":128}}"#;
        assert_eq!(per_token(shown), Some(147_456));
        assert_eq!((per_token(r#"{"model_info":{"m.block_count":36}}"#), per_token("no")), (None, None));
        let gib = |count: u64| count << 30;
        // 18 GB hold a 3.3 GB model with 49 152 tokens, 16 GB with 36 864, 36 GB with as many as are ever asked for.
        assert_eq!(fitting(gib(18), 3_300_000_000, 147_456), Some(49_152));
        assert_eq!(fitting(gib(16), 3_300_000_000, 147_456), Some(36_864));
        assert_eq!(fitting(gib(36), 3_300_000_000, 147_456), Some(MOST_CONTEXT));
        // Too little memory, or a model larger than it, still gets the usual context; a model that says nothing gets none.
        assert_eq!((fitting(gib(8), 3_300_000_000, 147_456), fitting(gib(8), gib(20), 147_456)), (Some(CONTEXT), Some(CONTEXT)));
        assert_eq!(fitting(gib(18), 3_300_000_000, 0), None);
        // What the user set in the settings is asked for; all the model can read, more than that or nothing
        // counts as not set, and then it is what suits the computer.
        assert_eq!((asked(65_536, Some(262_144), 49_152), asked(8_192, Some(262_144), 49_152)), (65_536, 8_192));
        assert_eq!(
            (asked(262_144, Some(262_144), 49_152), asked(512_000, Some(262_144), 49_152), asked(0, Some(262_144), 49_152)),
            (49_152, 49_152, 49_152)
        );
        assert_eq!(asked(65_536, None, 32_768), 32_768);
    }

    /// Runs one message through an Ollama on this computer, with its first model, and gives what the chat was told.
    fn told_by_ollama(prompt: &str) -> Vec<String> {
        let base = "http://localhost:11434";
        let models = crate::chat::catalog::list_models(crate::chat::OLLAMA_API, base, None).unwrap();
        let (tools, calls) = channel::<crate::link::Call>();
        let mut png = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(96, 96, image::Rgba([220, 30, 30, 255])).write_to(&mut png, image::ImageFormat::Png).unwrap();
        let picture = crate::agent::base64(&png.into_inner());
        // The app's part: every tool call is answered with a line of text, and the screenshot tool with a red
        // square as well.
        let app = std::thread::spawn(move || {
            for call in calls {
                let mut made = vec![json!({ "type": "text", "text": "One document is open: Untitled, 1 frame, nothing selected." })];
                if call.tool == "screenshot" {
                    made.push(json!({ "type": "image", "mimeType": "image/png", "data": picture }));
                }
                let _gone = call.reply.send(Ok(made));
            }
        });
        let inbox = api::Inbox::default();
        let told = std::sync::Mutex::new(Vec::new());
        let job = Job {
            endpoint: &endpoint(base),
            official: false,
            model: &models[0].id,
            window: 0,
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

    /// A tool's picture reaches a model that reads pictures, and a model that doesn't works on without it.
    #[test]
    #[ignore = "talks to an Ollama on this computer that has a model with tool calling"]
    fn a_model_in_ollama_is_given_a_tools_picture() {
        let told = told_by_ollama("Call the screenshot tool once. Then tell me in one sentence which one colour fills the picture it made.");
        let words: String = told.iter().filter_map(|event| event.strip_prefix("Text(\"")).map(|word| word.trim_end_matches("\")")).collect();
        println!("{words}");
        assert!(told.iter().all(|event| !event.starts_with("Problem")), "{told:?}");
        assert!(told.iter().any(|event| event.starts_with("Tool {")) && !words.is_empty() && !words.contains("think>"));
        let history = told.last().filter(|event| event.starts_with("History")).unwrap();
        // The picture is in the conversation and the model named its colour, or a note stands in its place.
        assert!(history.contains("image_url") && words.to_lowercase().contains("red") || history.contains("doesn't take pictures"), "{words}");
    }
}
