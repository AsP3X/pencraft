//! Answering with an agent program installed on this computer: Claude Code, or xAI's Grok.
//!
//! The program is run once per message, without a window of its own (`claude -p`, `grok -p`). It signs in the
//! way the user has set it up, so no key is needed here. Both write the same stream of JSON lines (Grok calls
//! it `streaming-messages-json`); a later message continues the same session.
//!
//! Claude Code is given this app as its one tool server on the command line and none of its own tools. Grok
//! takes no tool server on the command line: it reaches the app through `pencraft mcp`, which the user adds to
//! Grok's MCP servers once (Settings → Agents), and only that server's tools are approved for it.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};

use super::Event;
use crate::app::lock;

/// The name the app's tool server has for Claude Code.
const SERVER: &str = "pencraft";
/// How much of what the program wrote to its error output is shown when it fails.
const MOST_ERROR_TEXT: usize = 600;

/// Which program answers; they differ in their command lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flavor {
    /// Anthropic's Claude Code.
    Claude,
    /// xAI's Grok.
    Grok,
}

impl Flavor {
    /// What the program is called in a sentence.
    pub(super) fn name(self) -> &'static str {
        match self {
            Flavor::Claude => "Claude Code",
            Flavor::Grok => "Grok",
        }
    }
}

/// One message to answer.
pub struct Job<'a> {
    /// Which program it is.
    pub flavor: Flavor,
    /// Where the program is.
    pub program: &'a Path,
    /// The model it is asked to use; empty for the program's own default.
    pub model: &'a str,
    /// What the user wants kept to in every conversation (the settings' custom instructions).
    pub instructions: &'a str,
    /// The user's message, with the app's note before it.
    pub prompt: &'a str,
    /// The pictures attached to it, as content blocks.
    pub pictures: &'a [Value],
    /// The session to continue, after the first message.
    pub session: Option<&'a str>,
    /// The app's local server: its port and secret.
    pub server: Option<&'a (u16, String)>,
    /// The chat's id in the app, which the address of the tool server carries.
    pub chat: u64,
}

/// The folder the chat's programs run in.
// Human: the programs keep their sessions by the folder they run in; one folder of its own keeps Pencraft's
// chats together and away from the instructions of whatever project the app was started in.
pub(super) fn folder() -> Option<std::path::PathBuf> {
    let folder = std::env::temp_dir().join("pencraft-chat");
    std::fs::create_dir_all(&folder).is_ok().then_some(folder)
}

/// The end of what a program wrote to its error output, short enough to show.
pub(super) fn tail(said: &str) -> &str {
    let said = said.trim();
    said.char_indices().rev().nth(MOST_ERROR_TEXT).map_or(said, |(at, _)| &said[at..])
}

/// Collects what `running` writes to its error output, on a thread of its own.
pub(super) fn collect_errors(running: &mut Child) -> Option<std::thread::JoinHandle<String>> {
    running.stderr.take().map(|mut errors| {
        std::thread::spawn(move || {
            let mut text = String::new();
            // What can't be read isn't shown.
            let _ = errors.read_to_string(&mut text);
            text
        })
    })
}

/// Keeps a terminal program from opening a terminal window of its own on Windows.
// Human: these are terminal programs; on Windows each would otherwise open a black terminal window every
// time, since Pencraft has none to lend it.
#[cfg_attr(not(windows), expect(clippy::needless_pass_by_ref_mut, reason = "only Windows has a window to keep away"))]
pub(super) fn windowless(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(crate::link::CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

/// The command that answers `job` with Claude Code.
// Human: the program gets no tools but Pencraft's (no shell, no files, no web): in the chat panel it is a
// designer, not a coding agent, and nothing it does needs asking the user for permission.
fn claude(job: &Job<'_>, port: u16, secret: &str) -> Command {
    let servers = json!({ "mcpServers": { SERVER: { "type": "http", "url": crate::link::address(port, job.chat), "headers": { "Authorization": format!("Bearer {secret}") } } } });
    let mut command = Command::new(job.program);
    command
        .args(["-p", "--output-format", "stream-json", "--verbose", "--include-partial-messages"])
        .args(["--tools", "", "--strict-mcp-config", "--mcp-config", &servers.to_string()])
        .args(["--allowedTools", &format!("mcp__{SERVER}"), "--append-system-prompt", &super::brief(false, job.instructions)]);
    if !job.model.is_empty() {
        command.args(["--model", job.model]);
    }
    if let Some(session) = job.session {
        command.args(["--resume", session]);
    }
    // Human: a message with pictures goes in as a JSON line, which can carry them; plain text can't.
    if !job.pictures.is_empty() {
        command.args(["--input-format", "stream-json"]);
    }
    command
}

/// The command that answers `job` with Grok; the message is in the file `prompt`.
// Human: only the tools of the server called "pencraft" in Grok's own settings are approved; anything else
// Grok would ask about is refused, since nobody can answer its question here. The message travels in a file:
// with attached text it can be longer than a command line may be. Grok reaches the app as any outside client
// does, so its tools work on the design in front, not on the design this chat belongs to.
fn grok(job: &Job<'_>, prompt: &Path) -> Command {
    let mut command = Command::new(job.program);
    command
        .args(["--output-format", "streaming-messages-json", "--include-partial-messages"])
        .args(["--allow", &format!("MCPTool({SERVER}__*)"), "--rules", &super::brief(false, job.instructions)])
        .arg("--prompt-file")
        .arg(prompt);
    if !job.model.is_empty() {
        command.args(["--model", job.model]);
    }
    if let Some(session) = job.session {
        command.args(["--resume", session]);
    }
    command
}

/// Asks the program one question with `system` as its whole brief and no tools, and gives its answer.
///
/// # Errors
/// When the program couldn't be started or ended with an error.
// Agent: BLOCKS until the program ends; `child` holds it meanwhile so the chat can end it.
pub fn once(program: &Path, model: &str, system: &str, content: &[Value], child: &Mutex<Option<Child>>) -> Result<String, String> {
    let mut command = Command::new(program);
    command
        .args(["-p", "--output-format", "stream-json", "--verbose", "--input-format", "stream-json"])
        .args(["--tools", "", "--strict-mcp-config", "--mcp-config", r#"{"mcpServers":{}}"#])
        .args(["--model", model, "--system-prompt", system]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(crate::link::CREATE_NO_WINDOW);
    }
    let folder = std::env::temp_dir().join("pencraft-chat");
    if std::fs::create_dir_all(&folder).is_ok() {
        command.current_dir(&folder);
    }
    let mut running = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Claude Code couldn't be started: {error}"))?;
    if let Some(mut input) = running.stdin.take() {
        let message = json!({ "type": "user", "message": { "role": "user", "content": content } });
        // A program that ended at once has closed its input; it then gives no answer, which is told below.
        let _ = input.write_all(format!("{message}\n").as_bytes());
    }
    let output = running.stdout.take();
    *lock(child) = Some(running);
    let mut answer = Err("Claude Code ended without an answer.".to_owned());
    for line in output.into_iter().flat_map(|output| BufReader::new(output).lines()) {
        let Ok(line) = line else { break };
        let Ok(value) = serde_json::from_str::<Value>(&line) else { continue };
        if value["type"] == "result" {
            let said = value["result"].as_str().unwrap_or_default().to_owned();
            answer = if value["is_error"] == true { Err(said) } else { Ok(said) };
        }
    }
    if let Some(mut running) = lock(child).take() {
        // The program has closed its output; it is ending or has ended.
        let _ = running.wait();
    }
    answer
}

/// What is written to the program's input: the message as text, or with pictures one JSON line holding both.
fn input(job: &Job<'_>) -> String {
    if job.pictures.is_empty() {
        return job.prompt.to_owned();
    }
    let message = json!({ "type": "user", "message": { "role": "user", "content": super::files::content(job.prompt, job.pictures) } });
    format!("{message}\n")
}

/// What one line of the program's output means for the chat.
// Human: the lines carry the same events as the API's own stream (words as they are written, tool calls as
// they start), each wrapped with the session's name; the last line says how the run went.
fn read_line(line: &str, wrote: &mut bool) -> Vec<Event> {
    let Ok(value) = serde_json::from_str::<Value>(line) else { return Vec::new() };
    let text = |pointer: &str| value.pointer(pointer).and_then(Value::as_str);
    match (text("/type"), text("/subtype")) {
        (Some("system"), Some("init")) => text("/session_id").map(|id| Event::Session(id.to_owned())).into_iter().collect(),
        (Some("stream_event"), _) => match (text("/event/type"), text("/event/delta/type"), text("/event/content_block/type")) {
            (Some("message_start"), ..) if *wrote => vec![Event::Break],
            (Some("content_block_delta"), Some("text_delta"), _) => {
                *wrote = true;
                text("/event/delta/text").map(|more| Event::Text(more.to_owned())).into_iter().collect()
            }
            (Some("content_block_delta"), Some("thinking_delta"), _) => {
                text("/event/delta/thinking").map(|more| Event::Thought(more.to_owned())).into_iter().collect()
            }
            (Some("content_block_start"), _, Some("thinking" | "redacted_thinking")) => vec![Event::Thought(String::new())],
            (Some("content_block_start"), _, Some("tool_use")) => text("/event/content_block/name")
                .map(|name| Event::Tool { id: text("/event/content_block/id").unwrap_or_default().to_owned(), name: name.to_owned() })
                .into_iter()
                .collect(),
            _ => Vec::new(),
        },
        // Human: after the stream of a turn the program writes the turn out whole: there each tool call has its
        // complete input. The tools' answers come back as a message "from the user", saying whether one failed.
        (Some("assistant"), _) => parts(&value, "tool_use")
            .map(|part| Event::ToolInput { id: part["id"].as_str().unwrap_or_default().to_owned(), input: part["input"].clone() })
            .collect(),
        (Some("user"), _) => parts(&value, "tool_result")
            .map(|part| Event::ToolEnd { id: part["tool_use_id"].as_str().unwrap_or_default().to_owned(), failed: part["is_error"] == true })
            .collect(),
        (Some("result"), _) if value.get("is_error").and_then(Value::as_bool) == Some(true) => {
            vec![Event::Problem(text("/result").filter(|said| !said.is_empty()).unwrap_or("Claude Code ended with an error.").to_owned())]
        }
        _ => Vec::new(),
    }
}

/// The parts of the kind `kind` in the message a line of the program's output holds.
fn parts<'a>(line: &'a Value, kind: &'a str) -> impl Iterator<Item = &'a Value> {
    line.pointer("/message/content").and_then(Value::as_array).into_iter().flatten().filter(move |part| part["type"] == kind)
}

/// Runs the program for `job`, telling the chat what it says. Returns when the program has ended.
// Agent: BLOCKS until the program ends; `child` holds it meanwhile so the chat can end it; SPAWNS a thread
// that collects the program's error output.
pub fn run(job: &Job<'_>, stop: &AtomicBool, child: &Mutex<Option<Child>>, tell: &dyn Fn(Event) -> bool) {
    let folder = folder();
    let mut command = match (job.flavor, job.server) {
        (Flavor::Claude, Some((port, secret))) => claude(job, *port, secret),
        (Flavor::Claude, None) => {
            tell(Event::Problem(
                "Claude Code reaches the document through Pencraft's local server, which didn't start. Restart Pencraft, or pick an API assistant."
                    .to_owned(),
            ));
            return;
        }
        (Flavor::Grok, _) => {
            if !job.pictures.is_empty() {
                tell(Event::Problem("Grok is given the message without its pictures: its program takes none this way.".to_owned()));
            }
            let file = folder.clone().unwrap_or_else(std::env::temp_dir).join(format!("message-{}-{}.txt", std::process::id(), job.chat));
            if let Err(error) = std::fs::write(&file, super::files::flat(job.prompt)) {
                tell(Event::Problem(format!("The message couldn't be handed to Grok: {error}")));
                return;
            }
            grok(job, &file)
        }
    };
    windowless(&mut command);
    if let Some(folder) = &folder {
        command.current_dir(folder);
    }
    let started = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let mut running = match started {
        Ok(running) => running,
        Err(error) => {
            tell(Event::Problem(format!("{} couldn't be started ({}): {error}", job.flavor.name(), job.program.display())));
            return;
        }
    };
    if let Some(mut input) = running.stdin.take()
        && job.flavor == Flavor::Claude
    {
        // A program that ended at once has closed its input; its error output says why.
        let _ = input.write_all(self::input(job).as_bytes());
    }
    let output = running.stdout.take();
    let errors = collect_errors(&mut running);
    *lock(child) = Some(running);
    let (mut wrote, mut failed, mut ended) = (false, false, false);
    for line in output.into_iter().flat_map(|output| BufReader::new(output).lines()) {
        let Ok(line) = line else { break };
        ended |= line.contains("\"type\":\"result\"");
        for event in read_line(&line, &mut wrote) {
            failed |= matches!(event, Event::Problem(_));
            if !tell(event) {
                break;
            }
        }
    }
    let status = lock(child).take().and_then(|mut running| running.wait().ok());
    let said = errors.and_then(|errors| errors.join().ok()).unwrap_or_default();
    // Human: a run that was stopped ends without a result, and that is not an error to report.
    if stop.load(Ordering::Relaxed) || failed || (ended && status.is_some_and(|status| status.success())) {
        return;
    }
    let said = tail(&said);
    let hint = match (said.is_empty(), job.flavor) {
        (false, _) => said,
        (true, Flavor::Claude) => "Run `claude` in a terminal once to check that it is signed in.",
        (true, Flavor::Grok) => "Run `grok login` in a terminal once, and add Pencraft's tools to Grok in Settings → Agents.",
    };
    tell(Event::Problem(format!("{} ended without an answer. {hint}", job.flavor.name())));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_programs_lines_become_events() {
        let mut wrote = false;
        let mut events = Vec::new();
        for line in [
            r#"{"type":"system","subtype":"init","session_id":"abc","tools":[]}"#,
            r#"{"type":"system","subtype":"status","status":"requesting"}"#,
            r#"{"type":"stream_event","event":{"type":"message_start","message":{}}}"#,
            r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}}"#,
            r#"{"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"t1","name":"mcp__pencraft__edit_layers","input":{}}}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello"}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"mcp__pencraft__edit_layers","input":{"description":"add"}}]}}"#,
            r#"{"type":"user","message":{"role":"user","content":[{"tool_use_id":"t1","type":"tool_result","is_error":true,"content":"no"}]}}"#,
            r#"{"type":"stream_event","event":{"type":"message_start","message":{}}}"#,
            r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"hm"}}}"#,
            "not json",
            r#"{"type":"result","subtype":"success","is_error":false,"result":"Hello"}"#,
            r#"{"type":"result","is_error":true,"result":"Not logged in"}"#,
        ] {
            events.extend(read_line(line, &mut wrote));
        }
        let shown: Vec<String> = events.iter().map(|e| format!("{e:?}")).collect();
        assert_eq!(
            shown,
            vec![
                r#"Session("abc")"#,
                r#"Text("Hello")"#,
                r#"Tool { id: "t1", name: "mcp__pencraft__edit_layers" }"#,
                r#"ToolInput { id: "t1", input: Object {"description": String("add")} }"#,
                r#"ToolEnd { id: "t1", failed: true }"#,
                "Break",
                r#"Thought("hm")"#,
                r#"Problem("Not logged in")"#
            ]
        );
    }

    #[test]
    fn the_program_gets_only_pencrafts_tools_and_continues_its_session() {
        let job = Job {
            flavor: Flavor::Claude,
            program: Path::new("/bin/claude"),
            model: "claude-opus-5-5",
            instructions: "Be brief.",
            prompt: "hi",
            pictures: &[],
            session: Some("abc"),
            server: None,
            chat: 3,
        };
        let command = claude(&job, 4242, "s3cret");
        let args: Vec<String> = command.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        let after = |flag: &str| args.iter().position(|a| a == flag).and_then(|at| args.get(at + 1)).cloned();
        assert_eq!(
            (after("--tools"), after("--allowedTools"), after("--model"), after("--resume")),
            (Some(String::new()), Some("mcp__pencraft".to_owned()), Some("claude-opus-5-5".to_owned()), Some("abc".to_owned()))
        );
        assert!(args.contains(&"--strict-mcp-config".to_owned()));
        // Without pictures the message goes in as plain text; with one, as a JSON line the program is told of.
        assert!(!args.contains(&"--input-format".to_owned()) && input(&job) == "hi");
        let picture = [json!({ "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": "Zm9v" } })];
        let seen = Job { pictures: &picture, ..job };
        let with: Vec<String> = claude(&seen, 4242, "s3cret").get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(with.iter().position(|a| a == "--input-format").and_then(|at| with.get(at + 1)).map(String::as_str), Some("stream-json"));
        let line: Value = serde_json::from_str(input(&seen).trim_end()).unwrap();
        assert_eq!(
            (&line["type"], &line["message"]["content"][0], &line["message"]["content"][1]["text"]),
            (&json!("user"), &picture[0], &json!("hi"))
        );
        let servers: Value = serde_json::from_str(&after("--mcp-config").unwrap()).unwrap();
        assert_eq!(servers["mcpServers"]["pencraft"]["url"], "http://127.0.0.1:4242/mcp?chat=3");
        assert_eq!(servers["mcpServers"]["pencraft"]["headers"]["Authorization"], "Bearer s3cret");
    }

    #[test]
    fn grok_reads_the_message_from_a_file_and_may_use_only_pencrafts_tools() {
        let job = Job {
            flavor: Flavor::Grok,
            program: Path::new("/bin/grok"),
            model: "",
            instructions: "Be brief.",
            prompt: "hi",
            pictures: &[],
            session: None,
            server: None,
            chat: 3,
        };
        let args =
            |job: &Job<'_>| -> Vec<String> { grok(job, Path::new("/tmp/m.txt")).get_args().map(|a| a.to_string_lossy().into_owned()).collect() };
        let plain = args(&job);
        let after = |args: &[String], flag: &str| args.iter().position(|a| a == flag).and_then(|at| args.get(at + 1)).cloned();
        assert_eq!(
            (after(&plain, "--output-format"), after(&plain, "--allow"), after(&plain, "--prompt-file")),
            (Some("streaming-messages-json".to_owned()), Some("MCPTool(pencraft__*)".to_owned()), Some("/tmp/m.txt".to_owned()))
        );
        // The user's standing instructions travel in the rules; its own default model unless one is named; a
        // later message continues the session.
        assert!(after(&plain, "--rules").is_some_and(|rules| rules.contains("every conversation:\nBe brief.")));
        assert!(
            plain.contains(&"--include-partial-messages".to_owned())
                && !plain.contains(&"--model".to_owned())
                && !plain.contains(&"--resume".to_owned())
        );
        let later = args(&Job { model: "grok-4.7", session: Some("abc"), ..job });
        assert_eq!((after(&later, "--model"), after(&later, "--resume")), (Some("grok-4.7".to_owned()), Some("abc".to_owned())));
        assert_eq!(tail("  not signed in \n"), "not signed in");
    }
}
