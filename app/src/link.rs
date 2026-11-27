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
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::app::lock;

/// The MCP version this server speaks when the client names none it knows.
const PROTOCOL: &str = "2025-03-26";
/// Largest message taken, in bytes.
const MOST_BYTES: usize = 32 * 1024 * 1024;
/// Longest line of a request's head (its first line or one header) the server reads.
const MOST_LINE: usize = 8 * 1024;
/// Most headers a request may have.
const MOST_HEADERS: usize = 64;
/// Most connections served at once; one more is told the server is busy and closed.
const MOST_CONNECTIONS: usize = 16;
/// Most of a refused request's body read and thrown away before its connection closes (so the refusal isn't
/// lost to a reset), and how long that may take.
const REFUSED_DRAIN: (u64, Duration) = (64 * 1024, Duration::from_secs(1));
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
    Some(own_folder()?.join("agent.json"))
}

/// Quill's folder in the user's home (`~/.pencraft`), which only the user can write into; `None` without a home.
#[must_use]
pub fn own_folder() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".pencraft"))
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
// Agent: SPAWNS one thread that accepts, and one per connection up to MOST_CONNECTIONS; they end with the
// process.
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
        let open = Arc::new(AtomicUsize::new(0));
        for mut stream in listener.incoming().flatten() {
            // Human: a caller that opens connection after connection can't make the app start a thread for each.
            let Some(slot) = Slot::take(&open) else {
                let _ = respond(&mut stream, "503 Service Unavailable", Some(&json!({ "error": "too many connections at once" })), false);
                continue;
            };
            let shared = shared.clone();
            // A connection that can't get a thread is dropped, and its slot with it; the client tries again.
            let _ = std::thread::Builder::new().name("ai-link-client".to_owned()).spawn(move || {
                let _held = slot;
                serve(stream, &shared);
            });
        }
    })?;
    Ok(Server { port, token, off })
}

/// Writes where this app's server is, and its secret, into the address file.
fn write_info(port: u16, token: &str) -> std::io::Result<()> {
    let path = info_file().ok_or_else(|| std::io::Error::other("no home folder to write the address into"))?;
    write_info_to(&path, port, token)
}

/// Writes where this app's server is, and its secret, into the address file at `path`.
fn write_info_to(path: &Path, port: u16, token: &str) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let info = json!({ "url": format!("http://127.0.0.1:{port}/mcp"), "port": port, "token": token, "pid": std::process::id() });
    // Human: the file holds the secret, so only the user may read it, from the moment it is there: it is made
    // readable by the user alone (0600) and only then filled. A file an older Quill left more open is closed
    // first, as a replaced file keeps the permissions it had. On Windows the user's own folder already keeps
    // other users out, so the file needs nothing more there.
    // Agent: craft-atomic writes it beside itself and renames it into place, so a crash never leaves half of it.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        match std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
    }
    craft_atomic::WriteOptions::default()
        .with_new_file_permissions(craft_atomic::NewFilePermissions::Private)
        .atomic_write(path, &serde_json::to_vec_pretty(&info).unwrap_or_default())
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

/// One of the [`MOST_CONNECTIONS`] connections served at once; given back when dropped.
struct Slot(Arc<AtomicUsize>);

impl Slot {
    /// A slot from the `open` count, unless all are taken.
    fn take(open: &Arc<AtomicUsize>) -> Option<Slot> {
        open.try_update(Ordering::AcqRel, Ordering::Acquire, |taken| (taken < MOST_CONNECTIONS).then_some(taken + 1))
            .ok()
            .map(|_| Slot(Arc::clone(open)))
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// The head of one HTTP request: its method, path, the headers this server reads, and how long its body is.
struct Head {
    method: String,
    path: String,
    authorization: Option<String>,
    origin: bool,
    length: usize,
}

/// Reads one line of a request's head into `line`; `None` when the connection ended, the line is longer than
/// [`MOST_LINE`] or isn't text.
fn head_line(reader: &mut BufReader<TcpStream>, line: &mut String) -> Option<()> {
    let limit = u64::try_from(MOST_LINE + 1).unwrap_or(u64::MAX);
    let read = reader.by_ref().take(limit).read_line(line).ok()?;
    (read > 0 && read <= MOST_LINE).then_some(())
}

/// Reads the head of one request from `reader`, not its body; `None` when the connection ended or sent
/// something unreadable or too long.
// Human: the body is read only once the request has been let in, so a caller without the secret can't make
// the app take in 32 MB; nor can it send one endless header line, or endless headers.
fn read_head(reader: &mut BufReader<TcpStream>) -> Option<Head> {
    let mut line = String::new();
    head_line(reader, &mut line)?;
    let mut parts = line.split_whitespace();
    let (method, path) = (parts.next()?.to_owned(), parts.next()?.to_owned());
    let (mut length, mut authorization, mut origin) = (0_usize, None, false);
    for _ in 0..=MOST_HEADERS {
        let mut header = String::new();
        head_line(reader, &mut header)?;
        let header = header.trim_end();
        if header.is_empty() {
            return Some(Head { method, path, authorization, origin, length });
        }
        let Some((name, value)) = header.split_once(':') else { continue };
        match name.trim().to_ascii_lowercase().as_str() {
            "content-length" => length = value.trim().parse().ok()?,
            "authorization" => authorization = Some(value.trim().to_owned()),
            "origin" => origin = true,
            _ => {}
        }
    }
    None
}

/// Writes an HTTP answer with a JSON body (or none); `keep` says whether the connection stays open after it.
fn respond(stream: &mut TcpStream, status: &str, body: Option<&Value>, keep: bool) -> std::io::Result<()> {
    let body = body.map(|b| serde_json::to_vec(b).unwrap_or_default()).unwrap_or_default();
    let connection = if keep { "keep-alive" } else { "close" };
    let head = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: {connection}\r\n\r\n", body.len());
    stream.write_all(head.as_bytes())?;
    stream.write_all(&body)?;
    stream.flush()
}

/// Answers a request that isn't let in, and ends its connection: its body was never read, so nothing more
/// on it can be understood.
// Agent: reads and throws away at most REFUSED_DRAIN of what the caller still sends, so closing doesn't reset
// the connection before the caller has read the answer.
fn refuse(out: &mut TcpStream, reader: &mut BufReader<TcpStream>, status: &str, why: &str) {
    if respond(out, status, Some(&json!({ "error": why })), false).is_err() {
        return;
    }
    let _ = out.shutdown(std::net::Shutdown::Write);
    let (most, patience) = REFUSED_DRAIN;
    let _ = reader.get_ref().set_read_timeout(Some(patience));
    let _ = std::io::copy(&mut reader.by_ref().take(most), &mut std::io::sink());
}

/// Serves one connection until it closes.
// Human: only a caller that shows the secret is served, and never a web page: a browser sends an `Origin`
// header with what a page asks for, and a page on the internet must not be able to reach into the app.
// Both are decided from the head, before the body is read.
fn serve(stream: TcpStream, shared: &Shared) {
    let Ok(mut out) = stream.try_clone() else { return };
    let mut reader = BufReader::new(stream);
    let secret = format!("Bearer {}", shared.token);
    while let Some(head) = read_head(&mut reader) {
        if head.origin {
            return refuse(&mut out, &mut reader, "403 Forbidden", "requests from web pages are refused");
        }
        if head.authorization.as_deref() != Some(secret.as_str()) {
            return refuse(&mut out, &mut reader, "401 Unauthorized", "the secret from ~/.pencraft/agent.json is missing or wrong");
        }
        if head.length > MOST_BYTES {
            return refuse(&mut out, &mut reader, "413 Content Too Large", "a message may be 32 MB at most");
        }
        let mut body = vec![0; head.length];
        if reader.read_exact(&mut body).is_err() {
            return;
        }
        let result = if head.method != "POST" || head.path.split('?').next() != Some("/mcp") {
            respond(&mut out, "405 Method Not Allowed", Some(&json!({ "error": "POST JSON-RPC messages to /mcp" })), true)
        } else {
            match serde_json::from_slice::<Value>(&body) {
                Err(error) => respond(&mut out, "200 OK", Some(&failure(&Value::Null, -32700, &format!("not JSON: {error}"))), true),
                Ok(message) => match answer(&message, chat_of(&head.path), shared) {
                    Some(reply) => respond(&mut out, "200 OK", Some(&reply), true),
                    None => respond(&mut out, "202 Accepted", None, true),
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

    #[cfg(unix)]
    #[test]
    fn the_address_file_is_only_the_users_even_when_an_older_one_was_open() {
        use std::os::unix::fs::PermissionsExt as _;
        let folder = std::env::temp_dir().join(format!("pencraft-link-info-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let path = folder.join("agent.json");
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        write_info_to(&path, 1234, "secret").unwrap();
        assert_eq!(mode(&path), 0o600);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_info_to(&path, 5678, "again").unwrap();
        assert_eq!(mode(&path), 0o600);
        let info: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!((info["port"].as_u64(), info["token"].as_str()), (Some(5678), Some("again")));
        std::fs::remove_dir_all(&folder).unwrap();
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
    fn a_caller_without_the_secret_is_refused_before_its_body_and_long_heads_are_cut_off() {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (shared, _calls) = shared();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                serve(stream, &shared);
            }
        });
        let status = |request: &str| {
            let mut stream = TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).unwrap();
            stream.write_all(request.as_bytes()).unwrap();
            let mut status = String::new();
            let _ = BufReader::new(stream).read_line(&mut status);
            status
        };
        // A wrong secret is answered from the head alone: the 32 MB it says are coming are never waited for.
        let big = status("POST /mcp HTTP/1.1\r\nAuthorization: Bearer wrong\r\nContent-Length: 33554432\r\n\r\n");
        assert!(big.starts_with("HTTP/1.1 401"), "{big}");
        // With the secret, a body over the limit is refused rather than read.
        let over = status("POST /mcp HTTP/1.1\r\nAuthorization: Bearer secret\r\nContent-Length: 33554433\r\n\r\n");
        assert!(over.starts_with("HTTP/1.1 413"), "{over}");
        // A header line that never ends, or headers without end, get no answer: the connection closes.
        let long = format!("POST /mcp HTTP/1.1\r\nX-Long: {}\r\n\r\n", "a".repeat(MOST_LINE + 10));
        assert_eq!(status(&long), "");
        let many = format!("POST /mcp HTTP/1.1\r\n{}\r\n", "X-A: b\r\n".repeat(MOST_HEADERS + 5));
        assert_eq!(status(&many), "");
    }

    #[test]
    fn only_so_many_connections_are_served_at_once() {
        let open = Arc::new(AtomicUsize::new(0));
        let held: Vec<Slot> = std::iter::from_fn(|| Slot::take(&open)).take(MOST_CONNECTIONS + 3).collect();
        assert_eq!((held.len(), open.load(Ordering::Acquire)), (MOST_CONNECTIONS, MOST_CONNECTIONS));
        assert!(Slot::take(&open).is_none());
        drop(held);
        assert_eq!(open.load(Ordering::Acquire), 0);
        assert!(Slot::take(&open).is_some());
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
