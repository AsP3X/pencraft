//! The door AI models come through: a small server on this computer that speaks the Model Context Protocol
//! (MCP), and a bridge for clients that start a program and talk to it over its input and output.
//!
//! The app listens on a port of the local machine only. A client sends a JSON-RPC message per HTTP POST:
//! `initialize`, `tools/list`, or `tools/call`. A tool call is handed to the app's main thread, which owns the
//! documents, and the answer travels back the same way. Where the server listens and the secret a caller must
//! show are written to a file in the user's home folder; `pencraft mcp` reads it and passes messages between
//! its standard input and output and the server, starting the app (in the background) when it isn't running.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::app::lock;

/// The MCP version this server speaks when the client names none it knows.
const PROTOCOL: &str = "2025-03-26";
/// Largest message taken, in bytes.
const MOST_BYTES: usize = 32 * 1024 * 1024;
/// How long the server waits for the app to answer a tool call.
const PATIENCE: Duration = Duration::from_secs(120);
/// How long the bridge waits for the app to start.
const START_PATIENCE: Duration = Duration::from_secs(20);
/// How often a running app looks whether the address file still leads to a running Quill.
const NOTE_CHECK: Duration = Duration::from_secs(2);
/// The message that asks a server whether it is there.
const PING: &str = r#"{"jsonrpc":"2.0","id":0,"method":"ping"}"#;
/// The name clients are shown.
const SERVER_NAME: &str = "pencraft";
/// The message a second Quill sends the running one to have it open files ([`hand_over`]), and the name
/// its [`Call`] has in the app. No tool of this name is offered to models.
pub const OPEN: &str = "pencraft/open";

/// One tool call on its way to the app.
pub struct Call {
    /// The tool's name.
    pub tool: String,
    /// What it was called with.
    pub arguments: Value,
    /// The client's name, as it introduced itself ("Claude Code", "Cursor"), for the cursor on the canvas.
    pub client: String,
    /// The chat inside Quill the call comes from, which decides the document it works on; `None` for a
    /// client outside the app, which works on the document in front.
    pub chat: Option<u64>,
    /// Where the answer goes: the content of the tool's result, or what went wrong.
    pub reply: Sender<Result<Vec<Value>, String>>,
}

/// Windows process creation flag: the started program gets no terminal window (`CREATE_NO_WINDOW`).
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Windows process creation flag: the started program is in a group of its own, so Ctrl+C in the terminal
/// that started it doesn't reach it (`CREATE_NEW_PROCESS_GROUP`).
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
/// Windows process creation flag: the started program doesn't share its starter's terminal (`DETACHED_PROCESS`).
#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;

/// The running server, as the app holds it.
pub struct Server {
    /// The port it listens on.
    pub port: u16,
    /// The secret a caller must show.
    pub token: String,
    /// The tools the user has turned off, which clients aren't told of; the app keeps it as the settings say.
    pub off: Arc<Mutex<Vec<String>>>,
}

/// Where the server's address and secret are written: `~/.pencraft/agent.json`.
fn info_file() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".pencraft").join("agent.json"))
}

/// A secret for this run, made from the operating system's randomness.
fn secret() -> String {
    use std::hash::{BuildHasher, Hasher};
    // Human: each `RandomState` is seeded by the system's random numbers; four of them give 256 bits.
    use std::fmt::Write as _;
    (0..4_u64).fold(String::new(), |mut out, n| {
        // Writing into a String can't fail.
        let _ = write!(out, "{:016x}", std::collections::hash_map::RandomState::new().build_hasher().finish() ^ n);
        out
    })
}

/// Starts the server. Tool calls go to `send`, for the app to answer; `wake` is called after one is queued, to get the app's attention (also while
/// its window is hidden).
///
/// # Errors
/// When no port can be opened or the address file can't be written.
// Agent: SPAWNS one thread that accepts, and one per connection; they end with the process.
pub fn start(send: Sender<Call>, wake: impl Fn() + Send + Sync + 'static) -> std::io::Result<Server> {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))?;
    let port = listener.local_addr()?.port();
    let token = secret();
    write_info(port, &token)?;
    // Human: the note can go stale under a running app: a second Quill writes its own and is then closed, or
    // is ended without tidying up. The app that is still there puts its own note back, so an AI client finds
    // it instead of starting yet another window.
    let again = token.clone();
    std::thread::Builder::new().name("ai-link-note".to_owned()).spawn(move || {
        loop {
            std::thread::sleep(NOTE_CHECK);
            if !serving() {
                // A note that can't be written now is tried again at the next check.
                let _ = write_info(port, &again);
            }
        }
    })?;
    let off = Arc::new(Mutex::new(Vec::new()));
    let shared = Arc::new(Shared {
        token: token.clone(),
        send: Mutex::new(send),
        client: Mutex::new("AI".to_owned()),
        off: off.clone(),
        wake: Box::new(wake),
    });
    std::thread::Builder::new().name("ai-link".to_owned()).spawn(move || {
        for stream in listener.incoming().flatten() {
            let shared = shared.clone();
            // A connection that can't get a thread is dropped; the client tries again.
            let _ = std::thread::Builder::new().name("ai-link-client".to_owned()).spawn(move || serve(stream, &shared));
        }
    })?;
    Ok(Server { port, token, off })
}

/// Writes where this app's server is, and its secret, into the address file.
fn write_info(port: u16, token: &str) -> std::io::Result<()> {
    let path = info_file().ok_or_else(|| std::io::Error::other("no home folder to write the address into"))?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let info = json!({ "url": format!("http://127.0.0.1:{port}/mcp"), "port": port, "token": token, "pid": std::process::id() });
    std::fs::write(&path, serde_json::to_vec_pretty(&info).unwrap_or_default())?;
    // Human: the file holds the secret, so only the user may read it. On Windows the user's own folder
    // already keeps other users out, so the file needs nothing more there.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Whether a Quill is running whose server the address file leads to.
#[must_use]
pub fn serving() -> bool {
    running().is_some_and(|(port, token)| post(port, &token, PING).is_ok())
}

/// Hands `files` to the Quill that is running, which opens them in tabs and comes to the front; whether
/// one took them.
// Human: a design double-clicked in the Explorer, or named on the command line, starts a second Quill. With
// one running already the file belongs in that one's window, beside the designs that are open; the second
// passes it on and ends without opening a window.
// Agent: BLOCKS on the running app's answer (it opens the files first); `false` when none answers.
#[must_use]
pub fn hand_over(files: &[PathBuf]) -> bool {
    let Some((port, token)) = running() else { return false };
    let paths: Vec<String> = files.iter().map(|file| std::path::absolute(file).unwrap_or_else(|_| file.clone()).display().to_string()).collect();
    let message = json!({ "jsonrpc": "2.0", "id": 1, "method": OPEN, "params": { "paths": paths } });
    post(port, &token, &message.to_string())
        .ok()
        .and_then(|body| serde_json::from_slice::<Value>(&body).ok())
        .is_some_and(|answer| answer.get("result").is_some())
}

/// Removes the address file when the app ends, so a bridge doesn't try a server that is gone.
pub fn forget() {
    // Human: only this app's own note is removed: a second Quill started later has written its own.
    let ours = |info: Value| info.get("pid").and_then(Value::as_u64) == Some(u64::from(std::process::id()));
    if let Some(path) = info_file()
        && std::fs::read(&path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).is_some_and(ours)
    {
        // Nothing to do if it is already gone.
        let _ = std::fs::remove_file(path);
    }
}

/// What the connection threads share.
struct Shared {
    token: String,
    send: Mutex<Sender<Call>>,
    /// The name the last client gave when it introduced itself.
    client: Mutex<String>,
    /// The tools the user has turned off.
    off: Arc<Mutex<Vec<String>>>,
    wake: Box<dyn Fn() + Send + Sync>,
}

/// One HTTP request: its method, path, the headers this server reads, and its body.
struct Request {
    method: String,
    path: String,
    authorization: Option<String>,
    origin: bool,
    body: Vec<u8>,
}

/// Reads one request from `reader`; `None` when the connection ended or sent something unreadable.
fn read_request(reader: &mut BufReader<TcpStream>) -> Option<Request> {
    let mut line = String::new();
    if reader.read_line(&mut line).ok()? == 0 {
        return None;
    }
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next()?.to_owned(), parts.next()?.to_owned());
    let (mut length, mut authorization, mut origin) = (0_usize, None, false);
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 {
            return None;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let Some((name, value)) = header.split_once(':') else { continue };
        match name.trim().to_ascii_lowercase().as_str() {
            "content-length" => length = value.trim().parse().ok()?,
            "authorization" => authorization = Some(value.trim().to_owned()),
            "origin" => origin = true,
            _ => {}
        }
    }
    if length > MOST_BYTES {
        return None;
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    Some(Request { method, path, authorization, origin, body })
}

/// Writes an HTTP answer with a JSON body (or none).
fn respond(stream: &mut TcpStream, status: &str, body: Option<&Value>) -> std::io::Result<()> {
    let body = body.map(|b| serde_json::to_vec(b).unwrap_or_default()).unwrap_or_default();
    let head = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n", body.len());
    stream.write_all(head.as_bytes())?;
    stream.write_all(&body)?;
    stream.flush()
}

/// Serves one connection until it closes.
// Human: only a caller that shows the secret is served, and never a web page: a browser sends an `Origin`
// header with what a page asks for, and a page on the internet must not be able to reach into the app.
fn serve(stream: TcpStream, shared: &Shared) {
    let Ok(mut out) = stream.try_clone() else { return };
    let mut reader = BufReader::new(stream);
    while let Some(request) = read_request(&mut reader) {
        let allowed = request.authorization.as_deref() == Some(&format!("Bearer {}", shared.token));
        let result = if request.origin {
            respond(&mut out, "403 Forbidden", Some(&json!({ "error": "requests from web pages are refused" })))
        } else if !allowed {
            respond(&mut out, "401 Unauthorized", Some(&json!({ "error": "the secret from ~/.pencraft/agent.json is missing or wrong" })))
        } else if request.method != "POST" || request.path.split('?').next() != Some("/mcp") {
            respond(&mut out, "405 Method Not Allowed", Some(&json!({ "error": "POST JSON-RPC messages to /mcp" })))
        } else {
            match serde_json::from_slice::<Value>(&request.body) {
                Err(error) => respond(&mut out, "200 OK", Some(&failure(&Value::Null, -32700, &format!("not JSON: {error}")))),
                Ok(message) => match answer(&message, chat_of(&request.path), shared) {
                    Some(reply) => respond(&mut out, "200 OK", Some(&reply)),
                    None => respond(&mut out, "202 Accepted", None),
                },
            }
        };
        if result.is_err() {
            return;
        }
    }
}

/// A JSON-RPC error answer.
fn failure(id: &Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// The address the chat `chat` inside Quill reaches the server at.
#[must_use]
pub fn address(port: u16, chat: u64) -> String {
    format!("http://127.0.0.1:{port}/mcp?chat={chat}")
}

/// The chat a request's `path` names (`/mcp?chat=3`), if it names one.
// Human: the app's own chats run side by side, one per open document. Each is given an address of its own, so
// the app knows which document a call is for even while another one is in front.
fn chat_of(path: &str) -> Option<u64> {
    let (_, query) = path.split_once('?')?;
    query.split('&').find_map(|pair| pair.strip_prefix("chat=")).and_then(|id| id.parse().ok())
}

/// The answer to one JSON-RPC message that came in for `chat`; `None` for a notification, which gets none.
fn answer(message: &Value, chat: Option<u64>, shared: &Shared) -> Option<Value> {
    let id = message.get("id").filter(|id| !id.is_null())?;
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let result = match message.get("method").and_then(Value::as_str) {
        Some("initialize") => {
            if let Some(name) = params.pointer("/clientInfo/name").and_then(Value::as_str) {
                name.clone_into(&mut lock(&shared.client));
            }
            let version = params.get("protocolVersion").and_then(Value::as_str).unwrap_or(PROTOCOL);
            json!({
                "protocolVersion": version,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
                "instructions": "Quill is a canvas design editor. Call get_state first, read get_guide before writing layers (it also says how to design well here, which fonts are installed and which skills there are; read a skill with read_skill before the kind of design it is for), change the design with edit_layers and check it with screenshot; act on the review that comes with the picture."
            })
        }
        Some("ping") => json!({}),
        Some(OPEN) => {
            let (reply, answered) = channel();
            let call = Call { tool: OPEN.to_owned(), arguments: params, client: String::new(), chat: None, reply };
            if lock(&shared.send).send(call).is_err() {
                return Some(failure(id, -32000, "Quill is closing"));
            }
            (shared.wake)();
            match answered.recv_timeout(PATIENCE) {
                Ok(Ok(_)) => json!({ "opened": true }),
                Ok(Err(error)) => return Some(failure(id, -32000, &error)),
                Err(_) => return Some(failure(id, -32000, "Quill didn't answer in time.")),
            }
        }
        Some("tools/list") => {
            // Human: only the app's own chat has a place to put a question to the user; a client outside the app
            // asks in its own window, and is not offered the tool.
            let mut off = lock(&shared.off).clone();
            // Human: the task list stands in the app's chat panel too.
            if chat.is_none() {
                off.extend(
                    [crate::chat::asks::TOOL, crate::chat::tasks::PLAN, crate::chat::tasks::ADD, crate::chat::tasks::FINISH].map(str::to_owned),
                );
            }
            json!({ "tools": crate::agent::tools_without(&off) })
        }
        Some("tools/call") => {
            let Some(tool) = params.get("name").and_then(Value::as_str) else { return Some(failure(id, -32602, "tools/call needs the tool's name")) };
            let (reply, answered) = channel();
            let call = Call {
                tool: tool.to_owned(),
                arguments: params.get("arguments").cloned().unwrap_or_else(|| json!({})),
                client: lock(&shared.client).clone(),
                chat,
                reply,
            };
            if lock(&shared.send).send(call).is_err() {
                return Some(failure(id, -32000, "Quill is closing"));
            }
            (shared.wake)();
            // Human: a tool that fails is still an answer (the model reads the error and tries again), marked
            // as an error; only a broken message is a protocol error.
            match answered.recv_timeout(PATIENCE) {
                Ok(Ok(content)) => json!({ "content": content, "isError": false }),
                Ok(Err(error)) => json!({ "content": [{ "type": "text", "text": error }], "isError": true }),
                Err(_) => json!({ "content": [{ "type": "text", "text": "Quill didn't answer in time." }], "isError": true }),
            }
        }
        Some(other) => return Some(failure(id, -32601, &format!("unknown method {other}"))),
        None => return Some(failure(id, -32600, "the message has no method")),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

/// Where a running app's server is, and its secret, from the address file.
fn running() -> Option<(u16, String)> {
    let info: Value = serde_json::from_slice(&std::fs::read(info_file()?).ok()?).ok()?;
    Some((u16::try_from(info.get("port")?.as_u64()?).ok()?, info.get("token")?.as_str()?.to_owned()))
}

/// Sends one JSON-RPC message to the app's server and returns its answer's body (empty for a notification).
fn post(port: u16, token: &str, body: &str) -> std::io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))?;
    let head = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    let mut reader = BufReader::new(stream);
    let mut length = 0_usize;
    let mut line = String::new();
    reader.read_line(&mut line)?;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header.trim_end().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().unwrap_or(0);
        }
    }
    let mut answer = vec![0; length.min(MOST_BYTES)];
    reader.read_exact(&mut answer)?;
    Ok(answer)
}

/// The app's server, starting the app in the background when it isn't running.
fn reach() -> Option<(u16, String)> {
    let alive = |(port, token): (u16, String)| post(port, &token, PING).ok().map(|_| (port, token));
    if let Some(found) = running().and_then(alive) {
        return Some(found);
    }
    // Human: the app is started without taking the keyboard or coming to the front, and keeps running after
    // the client that asked for it has gone, like any app the user started.
    let program = std::env::current_exe().ok()?;
    if !start_through_shell(&program) {
        let mut command = std::process::Command::new(program);
        command.arg("--background").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        // Human: in a group of its own, so it isn't ended together with the client that asked for it.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt as _;
            command.process_group(0);
        }
        // Human: the same on Windows, where it also must not share the client's terminal: closing that
        // terminal would close the app with it.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt as _;
            command.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
        }
        command.spawn().ok()?;
    }
    let started = Instant::now();
    while started.elapsed() < START_PATIENCE {
        std::thread::sleep(Duration::from_millis(150));
        if let Some(found) = running().and_then(alive) {
            return Some(found);
        }
    }
    None
}

/// Starts the app at `program` in the background the way the system's shell starts programs; whether it did.
// Human: a program started directly on Windows is handed every pipe its starter holds, so the app would keep
// the AI client's pipes open after this bridge has ended, and the client would wait for them for ever. Started
// through the shell (PowerShell's `Start-Process`) it gets none of them. When that can't be done the app is
// started directly instead.
// Agent: BLOCKS until PowerShell has handed the start over (well under a second); the app itself keeps running.
#[cfg(windows)]
fn start_through_shell(program: &std::path::Path) -> bool {
    use std::os::windows::process::CommandExt as _;
    let Some(windows) = std::env::var_os("SystemRoot") else { return false };
    let shell = PathBuf::from(windows).join("System32").join("WindowsPowerShell").join("v1.0").join("powershell.exe");
    // Human: inside PowerShell's single quotes only the quote itself has to be doubled.
    let quoted = program.display().to_string().replace('\'', "''");
    std::process::Command::new(shell)
        .args(["-NoProfile", "-NonInteractive", "-Command", &format!("Start-Process -FilePath '{quoted}' -ArgumentList '--background'")])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .is_ok_and(|status| status.success())
}

/// Elsewhere a started program gets only what it is given, so the app is started directly.
#[cfg(not(windows))]
fn start_through_shell(_program: &std::path::Path) -> bool {
    false
}

/// `pencraft mcp`: passes MCP messages between this program's standard input and output and the app, one JSON
/// message a line, until the input ends.
// Agent: BLOCKS on standard input; WRITES only protocol messages to standard output (a client reads nothing else
// there).
#[expect(clippy::print_stdout, reason = "the MCP stdio transport is messages on standard output")]
pub fn bridge() -> ExitCode {
    let mut server: Option<(u16, String)> = None;
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let id = serde_json::from_str::<Value>(&line).ok().and_then(|m| m.get("id").cloned()).filter(|id| !id.is_null());
        let mut sent = server.as_ref().and_then(|(port, token)| post(*port, token, &line).ok());
        if sent.is_none() {
            // The app was closed (or never started): find or start it, once more.
            server = reach();
            sent = server.as_ref().and_then(|(port, token)| post(*port, token, &line).ok());
        }
        let out = match (sent, id) {
            (Some(body), _) if !body.is_empty() => String::from_utf8_lossy(&body).into_owned(),
            (Some(_), _) | (None, None) => continue,
            (None, Some(id)) => failure(&id, -32000, "Quill isn't running and couldn't be started").to_string(),
        };
        println!("{out}");
        // A closed output means the client has gone.
        if std::io::stdout().flush().is_err() {
            break;
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::Receiver;

    use super::*;

    fn shared() -> (Shared, Receiver<Call>) {
        let (send, calls) = channel();
        let off = Arc::new(Mutex::new(Vec::new()));
        (Shared { token: "secret".to_owned(), send: Mutex::new(send), client: Mutex::new("AI".to_owned()), off, wake: Box::new(|| {}) }, calls)
    }

    #[test]
    fn a_client_is_greeted_and_told_the_tools() {
        let (shared, _calls) = shared();
        let hello = answer(&json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2024-11-05", "clientInfo": { "name": "Some Agent" } } }), None, &shared).unwrap();
        assert_eq!((&hello["result"]["protocolVersion"], &hello["result"]["serverInfo"]["name"]), (&json!("2024-11-05"), &json!("pencraft")));
        assert_eq!(*lock(&shared.client), "Some Agent");
        assert!(answer(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }), None, &shared).is_none());
        let list = answer(&json!({ "jsonrpc": "2.0", "id": "a", "method": "tools/list" }), None, &shared).unwrap();
        assert!(list["result"]["tools"].as_array().is_some_and(|tools| tools.len() >= 9) && list["id"] == "a");
        // Asking the user is offered to the app's own chats only.
        let offered = |chat| {
            let list = answer(&json!({ "jsonrpc": "2.0", "id": "c", "method": "tools/list" }), chat, &shared).unwrap();
            list["result"]["tools"].as_array().unwrap().iter().any(|tool| tool["name"] == crate::chat::asks::TOOL)
        };
        assert!(offered(Some(3)) && !offered(None));
        // A tool the user turned off is left out of the list from then on.
        lock(&shared.off).push("export_layers".to_owned());
        let fewer = answer(&json!({ "jsonrpc": "2.0", "id": "b", "method": "tools/list" }), None, &shared).unwrap();
        let names: Vec<&str> = fewer["result"]["tools"].as_array().unwrap().iter().filter_map(|tool| tool["name"].as_str()).collect();
        assert!(names.len() >= 8 && !names.contains(&"export_layers") && names.contains(&"edit_layers"));
        assert_eq!(answer(&json!({ "jsonrpc": "2.0", "id": 2, "method": "nope" }), None, &shared).unwrap()["error"]["code"], -32601);
    }

    #[test]
    fn a_tool_call_goes_to_the_app_and_its_answer_comes_back() {
        let (shared, calls) = shared();
        let app = std::thread::spawn(move || {
            let call = calls.recv().unwrap();
            assert_eq!((call.tool.as_str(), &call.arguments["ids"][0], call.chat), ("read_layers", &json!("a"), Some(7)));
            call.reply.send(Ok(vec![json!({ "type": "text", "text": "done" })])).unwrap();
            let failing = calls.recv().unwrap();
            failing.reply.send(Err("no such layer".to_owned())).unwrap();
        });
        let call = |arguments: Value| {
            answer(
                &json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "read_layers", "arguments": arguments } }),
                Some(7),
                &shared,
            )
            .unwrap()
        };
        let good = call(json!({ "ids": ["a"] }));
        assert_eq!((&good["result"]["content"][0]["text"], &good["result"]["isError"]), (&json!("done"), &json!(false)));
        let bad = call(json!({}));
        assert_eq!((&bad["result"]["content"][0]["text"], &bad["result"]["isError"]), (&json!("no such layer"), &json!(true)));
        app.join().unwrap();
    }

    #[test]
    fn files_handed_over_reach_the_app_as_a_call_of_their_own() {
        let (shared, calls) = shared();
        let app = std::thread::spawn(move || {
            let call = calls.recv().unwrap();
            assert_eq!((call.tool.as_str(), &call.arguments["paths"], call.chat), (OPEN, &json!(["/designs/shop.pen"]), None));
            call.reply.send(Ok(Vec::new())).unwrap();
            calls.recv().unwrap().reply.send(Err("Quill is busy".to_owned())).unwrap();
        });
        let open = json!({ "jsonrpc": "2.0", "id": 1, "method": OPEN, "params": { "paths": ["/designs/shop.pen"] } });
        assert_eq!(answer(&open, None, &shared).unwrap()["result"], json!({ "opened": true }));
        // An app that can't take them says so, and the second Quill opens them itself.
        assert!(answer(&open, None, &shared).unwrap().get("result").is_none());
        app.join().unwrap();
    }

    #[test]
    fn the_server_serves_callers_with_the_secret_and_no_one_else() {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (shared, _calls) = shared();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                serve(stream, &shared);
            }
        });
        let ping = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let answer: Value = serde_json::from_slice(&post(port, "secret", ping).unwrap()).unwrap();
        assert_eq!(answer, json!({ "jsonrpc": "2.0", "id": 1, "result": {} }));
        let refused: Value = serde_json::from_slice(&post(port, "wrong", ping).unwrap()).unwrap();
        assert!(refused["error"].as_str().is_some_and(|e| e.contains("secret")));
        // A notification gets an empty answer; a web page (it sends an Origin) is refused even with the secret.
        assert_eq!(post(port, "secret", r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).unwrap(), Vec::<u8>::new());
        let mut page = TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).unwrap();
        let request = format!(
            "POST /mcp HTTP/1.1\r\nOrigin: https://example.com\r\nAuthorization: Bearer secret\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{ping}",
            ping.len()
        );
        page.write_all(request.as_bytes()).unwrap();
        let mut status = String::new();
        BufReader::new(page).read_line(&mut status).unwrap();
        assert!(status.starts_with("HTTP/1.1 403"), "{status}");
    }

    #[test]
    fn a_chats_address_names_it() {
        assert_eq!(address(4242, 3), "http://127.0.0.1:4242/mcp?chat=3");
        assert_eq!(chat_of("/mcp?chat=3"), Some(3));
        assert_eq!(chat_of("/mcp?session=a&chat=12"), Some(12));
        // An outside client names none; a name that is no number is none either.
        assert_eq!((chat_of("/mcp"), chat_of("/mcp?chat=x"), chat_of("/mcp?")), (None, None, None));
    }

    #[test]
    fn secrets_are_long_and_differ() {
        let (a, b) = (secret(), secret());
        assert!(a.len() == 64 && a != b && a.bytes().all(|c| c.is_ascii_hexdigit()));
    }
}
