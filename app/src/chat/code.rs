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
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use serde_json::{Value, json};

use super::Event;
use super::usage::{self, Used};
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
// Human: the programs keep their sessions by the folder they run in; one folder of its own keeps Quill's
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
// time, since Quill has none to lend it.
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

/// The MCP settings Claude Code is started with for `job`: Quill's server at `port`, reached with `secret`.
fn servers(job: &Job<'_>, port: u16, secret: &str) -> Value {
    json!({ "mcpServers": { SERVER: { "type": "http", "url": crate::link::address(port, job.chat), "headers": { "Authorization": format!("Bearer {secret}") } } } })
}

/// A file one run of a program is started with: Claude Code's MCP settings, or the message for Grok. Deleted
/// when dropped.
// Human: the settings hold the secret of Quill's server, and the message holds the conversation and the
// design. On the command line every program on the computer could read them in the list of processes, and in
// the shared temporary folder so could other users; in a file only the user can read, they are private, and
// they are there only while the run lasts: also when the run fails or is stopped.
struct RunFile(PathBuf);

impl RunFile {
    /// Writes `content` for the chat `chat` into a new file in Quill's own folder, readable by the user alone.
    /// The file is called `kind` with the run's numbers and ends in `ending`.
    ///
    /// # Errors
    /// When there is no home folder, or the file can't be made or written.
    // Agent: WRITES `~/.pencraft/<kind>-<pid>-<chat>-<n>.<ending>`, made new (never through a file that was
    // there) with mode 0600 on Unix; a number of its own per run, so a run winding down never deletes the next
    // one's.
    fn write(kind: &str, ending: &str, chat: u64, content: &[u8]) -> std::io::Result<RunFile> {
        static RUNS: AtomicU64 = AtomicU64::new(0);
        let folder = crate::link::own_folder().ok_or_else(|| std::io::Error::other("there is no home folder"))?;
        std::fs::create_dir_all(&folder)?;
        let run = RUNS.fetch_add(1, Ordering::Relaxed);
        let path = folder.join(format!("{kind}-{}-{chat}-{run}.{ending}", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&path)?;
        // Made: from here on it is deleted when this is dropped, also when writing it fails.
        let made = RunFile(path);
        file.write_all(content)?;
        Ok(made)
    }

    /// The MCP settings `servers` for a run of Claude Code in the chat `chat`.
    ///
    /// # Errors
    /// As [`RunFile::write`].
    fn config(chat: u64, servers: &Value) -> std::io::Result<RunFile> {
        RunFile::write("mcp", "json", chat, servers.to_string().as_bytes())
    }

    /// The message `text` for a run of Grok in the chat `chat`.
    ///
    /// # Errors
    /// As [`RunFile::write`].
    fn message(chat: u64, text: &str) -> std::io::Result<RunFile> {
        RunFile::write("message", "txt", chat, text.as_bytes())
    }
}

impl Drop for RunFile {
    fn drop(&mut self) {
        // Already gone is fine.
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The command that answers `job` with Claude Code, its MCP settings in the file `config`.
// Human: the program gets no tools but Quill's (no shell, no files, no web): in the chat panel it is a
// designer, not a coding agent, and nothing it does needs asking the user for permission.
fn claude(job: &Job<'_>, config: &Path) -> Command {
    let mut command = Command::new(job.program);
    command
        .args(["-p", "--output-format", "stream-json", "--verbose", "--include-partial-messages"])
        .args(["--tools", "", "--strict-mcp-config", "--mcp-config"])
        .arg(config)
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
pub fn once(
    program: &Path,
    model: &str,
    system: &str,
    content: &[Value],
    child: &Mutex<Option<Child>>,
    used: &dyn Fn(super::usage::Report),
) -> Result<String, String> {
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
            if let Some(report) = super::usage::program(&value) {
                used(report);
            }
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
fn read_line(line: &str, wrote: &mut bool, meter: &mut Meter) -> Vec<Event> {
    let Ok(value) = serde_json::from_str::<Value>(line) else { return Vec::new() };
    let mut told: Vec<Event> = meter.take(&value).into_iter().collect();
    told.extend(events_of(&value, wrote));
    told
}

/// The tokens a program's run has taken so far, by the requests its output shows.
// Human: the program says what a run took only on its last line. Until then the chat counts along: each of the
// run's requests says at its start what the model read and at its end what it wrote.
#[derive(Default)]
struct Meter {
    /// The requests that are over.
    done: Used,
    /// The request under way.
    now: Used,
}

impl Meter {
    /// Takes in one line of the program's output. Returns the run's count so far when the line changed it.
    fn take(&mut self, line: &Value) -> Option<Event> {
        let event = line.get("event").filter(|_| line["type"] == "stream_event")?;
        let before = self.so_far();
        match event["type"].as_str() {
            Some("message_start") => {
                self.done.add(self.now);
                self.now = Used::default();
                usage::messages(&event["message"]["usage"], &mut self.now);
            }
            Some("message_delta") => usage::messages(&event["usage"], &mut self.now),
            _ => return None,
        }
        let so_far = self.so_far();
        (so_far != before).then_some(Event::Meanwhile(so_far))
    }

    /// What the requests so far took together.
    fn so_far(&self) -> Used {
        let mut sum = self.done;
        sum.add(self.now);
        sum
    }
}

/// What to look at when the provider doesn't accept the program's key.
const SIGN_IN_HINT: &str = "Its key or sign-in isn't accepted: run `claude` in a terminal to check, and look at where Claude Code is set up to \
                            send its requests (ANTHROPIC_BASE_URL in its settings, or a router in front of it and that router's key).";

/// What the line `value` of the program's output means for the chat, its tokens aside.
fn events_of(value: &Value, wrote: &mut bool) -> Vec<Event> {
    let text = |pointer: &str| value.pointer(pointer).and_then(Value::as_str);
    match (text("/type"), text("/subtype")) {
        (Some("system"), Some("init")) => text("/session_id").map(|id| Event::Session(id.to_owned())).into_iter().collect(),
        // Human: the program tries a request again by itself when its provider can't take it just now. A
        // provider that doesn't accept its key will not accept it the tenth time either, and the tries take
        // minutes in which the chat would only say that it waits: that is a problem to tell at once.
        (Some("system"), Some("api_retry")) => {
            let status = value.get("error_status").and_then(Value::as_u64);
            let why = text("/error").map(|why| format!(", {}", why.replace('_', " "))).unwrap_or_default();
            status
                .filter(|status| matches!(status, 401 | 403))
                .map(|status| Event::Problem(format!("Claude Code's provider turned it away ({status}{why}). {SIGN_IN_HINT}")))
                .into_iter()
                .collect()
        }
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
        (Some("assistant"), _) => parts(value, "tool_use")
            .map(|part| Event::ToolInput { id: part["id"].as_str().unwrap_or_default().to_owned(), input: part["input"].clone() })
            .collect(),
        (Some("user"), _) => parts(value, "tool_result")
            .map(|part| Event::ToolEnd { id: part["tool_use_id"].as_str().unwrap_or_default().to_owned(), failed: part["is_error"] == true })
            .collect(),
        // Human: the last line says how the run went, and what it took and cost.
        (Some("result"), _) => {
            let mut told: Vec<Event> = super::usage::program(value).map(Event::Used).into_iter().collect();
            if value.get("is_error").and_then(Value::as_bool) == Some(true) {
                told.push(Event::Problem(text("/result").filter(|said| !said.is_empty()).unwrap_or("Claude Code ended with an error.").to_owned()));
            }
            told
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
    // Agent: lives until this function returns, after the program has ended, however it ended (answered,
    // failed, stopped, or never started); its file goes with it.
    let mut run_file = None;
    let mut command = match (job.flavor, job.server) {
        (Flavor::Claude, Some((port, secret))) => match RunFile::config(job.chat, &servers(job, *port, secret)) {
            Ok(config) => claude(job, &run_file.insert(config).0),
            Err(error) => {
                tell(Event::Problem(format!("Claude Code couldn't be given Quill's tools: {error}")));
                return;
            }
        },
        (Flavor::Claude, None) => {
            tell(Event::Problem(
                "Claude Code reaches the document through Quill's local server, which didn't start. Restart Quill, or pick an API assistant."
                    .to_owned(),
            ));
            return;
        }
        (Flavor::Grok, _) => {
            if !job.pictures.is_empty() {
                tell(Event::Problem("Grok is given the message without its pictures: its program takes none this way.".to_owned()));
            }
            match RunFile::message(job.chat, &super::files::flat(job.prompt)) {
                Ok(message) => grok(job, &run_file.insert(message).0),
                Err(error) => {
                    tell(Event::Problem(format!("The message couldn't be handed to Grok: {error}")));
                    return;
                }
            }
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
    let mut meter = Meter::default();
    for line in output.into_iter().flat_map(|output| BufReader::new(output).lines()) {
        let Ok(line) = line else { break };
        ended |= line.contains("\"type\":\"result\"");
        for event in read_line(&line, &mut wrote, &mut meter) {
            failed |= matches!(event, Event::Problem(_));
            if !tell(event) {
                break;
            }
        }
        // Human: a problem told before the program's last line means it would only go on trying: it is ended.
        if failed && !ended {
            if let Some(mut running) = lock(child).take() {
                // A program that has already ended can't be ended again; either way it is gone.
                let _ = running.kill();
                let _ = running.wait();
            }
            break;
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
        (true, Flavor::Grok) => "Run `grok login` in a terminal once, and add Quill's tools to Grok in Settings → Agents.",
    };
    tell(Event::Problem(format!("{} ended without an answer. {hint}", job.flavor.name())));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_programs_lines_become_events() {
        let mut wrote = false;
        let mut meter = Meter::default();
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
            r#"{"type":"result","subtype":"success","is_error":false,"result":"yo","total_cost_usd":0.16488039999999998,"usage":{"input_tokens":2,"cache_creation_input_tokens":4724,"cache_read_input_tokens":53281,"output_tokens":4},"modelUsage":{"claude-fable-5-2":{"inputTokens":4,"outputTokens":8,"cacheReadInputTokens":106562,"cacheCreationInputTokens":8951,"costUSD":0.16488039999999998}},"model_usage_since_last_result":{"claude-fable-5-2":{"inputTokens":2,"outputTokens":4,"cacheReadInputTokens":53281,"cacheCreationInputTokens":4724,"costUSD":0.0864162,"costBasis":"list"}}}"#,
            r#"{"type":"result","is_error":true,"result":"Not logged in"}"#,
        ] {
            events.extend(read_line(line, &mut wrote, &mut meter));
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
                r#"Used(Report { used: Used { input: 2, cached: 53281, written: 4724, output: 4 }, model: Some("claude-fable-5-2"), billed: Some(86416200000) })"#,
                r#"Problem("Not logged in")"#
            ]
        );
    }

    #[test]
    fn a_provider_that_turns_the_program_away_is_a_problem_at_once() {
        let events = |line: &str| read_line(line, &mut false, &mut Meter::default());
        let turned_away =
            events(r#"{"type":"system","subtype":"api_retry","attempt":3,"max_retries":10,"error_status":401,"error":"authentication_failed"}"#);
        let [Event::Problem(said)] = turned_away.as_slice() else { panic!("{turned_away:?}") };
        assert!(said.starts_with("Claude Code's provider turned it away (401, authentication failed). Its key or sign-in isn't accepted"), "{said}");
        // A provider that is only busy is waited for, as before.
        assert!(events(r#"{"type":"system","subtype":"api_retry","attempt":1,"max_retries":10,"error_status":529,"error":"overloaded"}"#).is_empty());
        assert!(events(r#"{"type":"system","subtype":"api_retry","attempt":1,"max_retries":10,"error":"timeout"}"#).is_empty());
    }

    #[test]
    fn a_runs_tokens_are_counted_along_from_its_requests() {
        let (mut wrote, mut meter) = (false, Meter::default());
        let mut counts = Vec::new();
        for line in [
            r#"{"type":"stream_event","event":{"type":"message_start","message":{"usage":{"input_tokens":2,"cache_read_input_tokens":5000,"cache_creation_input_tokens":300,"output_tokens":1}}}}"#,
            r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}}"#,
            r#"{"type":"stream_event","event":{"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":40}}}"#,
            // The same count again changes nothing and is not told twice.
            r#"{"type":"stream_event","event":{"type":"message_delta","delta":{},"usage":{"output_tokens":40}}}"#,
            // The run's next request counts on top of the first.
            r#"{"type":"stream_event","event":{"type":"message_start","message":{"usage":{"input_tokens":10,"cache_read_input_tokens":5300,"output_tokens":1}}}}"#,
            r#"{"type":"stream_event","event":{"type":"message_delta","delta":{},"usage":{"output_tokens":7}}}"#,
            // A whole message written out after its stream is not counted a second time.
            r#"{"type":"assistant","message":{"content":[],"usage":{"input_tokens":10,"output_tokens":7}}}"#,
        ] {
            for event in read_line(line, &mut wrote, &mut meter) {
                if let Event::Meanwhile(used) = event {
                    counts.push((used.input, used.cached, used.written, used.output));
                }
            }
        }
        assert_eq!(counts, vec![(2, 5000, 300, 1), (2, 5000, 300, 40), (12, 10_300, 300, 41), (12, 10_300, 300, 47)]);
    }

    #[test]
    fn the_program_gets_only_quills_tools_and_continues_its_session() {
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
        let command = claude(&job, Path::new("/home/me/.pencraft/mcp.json"));
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
        let with: Vec<String> =
            claude(&seen, Path::new("/home/me/.pencraft/mcp.json")).get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(with.iter().position(|a| a == "--input-format").and_then(|at| with.get(at + 1)).map(String::as_str), Some("stream-json"));
        let line: Value = serde_json::from_str(input(&seen).trim_end()).unwrap();
        assert_eq!(
            (&line["type"], &line["message"]["content"][0], &line["message"]["content"][1]["text"]),
            (&json!("user"), &picture[0], &json!("hi"))
        );
        // The secret isn't on the command line: the settings are in a file, which only the user can read and
        // which goes when the run is over.
        assert_eq!(after("--mcp-config").as_deref(), Some("/home/me/.pencraft/mcp.json"));
        assert!(!args.iter().any(|arg| arg.contains("s3cret")));
        let servers = servers(&job, 4242, "s3cret");
        assert_eq!(servers["mcpServers"]["pencraft"]["url"], "http://127.0.0.1:4242/mcp?chat=3");
        assert_eq!(servers["mcpServers"]["pencraft"]["headers"]["Authorization"], "Bearer s3cret");
        let config = RunFile::config(3, &servers).unwrap();
        let path = config.0.clone();
        assert_eq!(serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap()).unwrap(), servers);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        drop(config);
        assert!(!path.exists());
        // Grok's message is kept the same way: private, in Quill's folder, and gone with the run.
        let message = RunFile::message(3, "Make the card blue").unwrap();
        let path = message.0.clone();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "Make the card blue");
        assert_eq!(path.parent(), crate::link::own_folder().as_deref());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        drop(message);
        assert!(!path.exists());
    }

    #[test]
    fn grok_reads_the_message_from_a_file_and_may_use_only_quills_tools() {
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
