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
/// Most connections served at once. One more takes the place of the connection that has waited longest
/// between requests; when every one is in the middle of a request, it is told the server is busy and closed.
const MOST_CONNECTIONS: usize = 16;
/// How long a request may take to arrive, from its first byte to the end of its body, and how long the server
/// waits for a caller to take more of an answer.
const REQUEST_PATIENCE: Duration = Duration::from_secs(10);
/// How long a connection may stay open with no request on its way: before its first one, and between two.
const IDLE_PATIENCE: Duration = Duration::from_mins(5);
/// The server's own pace.
const PACE: Pace = Pace { request: REQUEST_PATIENCE, idle: IDLE_PATIENCE, most: MOST_CONNECTIONS };
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
// Agent: SPAWNS one thread that accepts, and one per connection up to MOST_CONNECTIONS; the accepting one
// ends with the process, a connection's when its caller goes, stalls or is idle too long.
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
    std::thread::Builder::new().name("ai-link".to_owned()).spawn(move || listen(&listener, &shared, PACE))?;
    Ok(Server { port, token, off })
}

/// Takes the connections that come in on `listener` and serves each on a thread of its own, at `pace`.
// Agent: BLOCKS for as long as the listener lives; SPAWNS one thread per connection, `pace.most` at most.
fn listen(listener: &TcpListener, shared: &Arc<Shared>, pace: Pace) {
    let lobby = Arc::new(Lobby::new(pace.most));
    for mut stream in listener.incoming().flatten() {
        // Human: a caller that opens connection after connection can't make the app start a thread for each,
        // and can't keep others out by holding connections it says nothing on: those are the first to go.
        let Some(seat) = Lobby::enter(&lobby, &stream) else {
            // A caller that doesn't take the answer isn't waited for: nothing else is accepted meanwhile.
            let _ = stream.set_write_timeout(Some(REFUSED_DRAIN.1));
            let _ = respond(&mut stream, "503 Service Unavailable", Some(&json!({ "error": "too many connections at once" })), false);
            continue;
        };
        let shared = shared.clone();
        // A connection that can't get a thread is dropped, and its place with it; the client tries again.
        let _ = std::thread::Builder::new().name("ai-link-client".to_owned()).spawn(move || serve(stream, &shared, &seat, pace));
    }
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

/// How patient the server is with a connection, and how many it serves at once.
// Human: two kinds of silence look alike on the wire and mean different things. A client that keeps its
// connection open between tool calls (Claude Code does, for a whole run, while the model thinks) is waiting
// for its next turn and gets minutes. A request that has started and doesn't finish is stuck or hostile and
// gets seconds. One short limit for both would cut real clients off; none at all lets a silent caller keep a
// place forever.
#[derive(Clone, Copy)]
struct Pace {
    /// How long a request may take to arrive, from its first byte to the end of its body.
    request: Duration,
    /// How long a connection may stay open with no request on its way.
    idle: Duration,
    /// Most connections served at once.
    most: usize,
}

/// One connection being served, as the [`Lobby`] knows it.
struct Guest {
    /// Its number, by which its [`Place`] finds it again.
    id: u64,
    /// The connection itself (another handle to it), to close it when its place is given away.
    stream: TcpStream,
    /// Since when it has had no request on its way; `None` while one arrives or is answered.
    idle: Option<Instant>,
}

/// The connections being served: at most `most`.
struct Lobby {
    most: usize,
    /// The next guest's number, and the guests.
    guests: Mutex<(u64, Vec<Guest>)>,
}

impl Lobby {
    /// An empty lobby for `most` connections.
    fn new(most: usize) -> Lobby {
        Lobby { most, guests: Mutex::new((0, Vec::new())) }
    }

    /// A place for the connection `stream`. When all are taken, the connection that has been idle longest is
    /// closed and gives up its place; `None` when every connection is in the middle of a request.
    // Human: a client whose idle connection is closed just opens another for its next request, so nothing is
    // lost; a request under way is never cut off for a newcomer.
    // Agent: CLOSES the given-up connection's socket, which wakes the thread that waits on it.
    fn enter(lobby: &Arc<Lobby>, stream: &TcpStream) -> Option<Place> {
        let handle = stream.try_clone().ok()?;
        let mut guests = lock(&lobby.guests);
        if guests.1.len() >= lobby.most {
            let longest = guests.1.iter().enumerate().filter_map(|(at, guest)| Some((guest.idle?, at))).min().map(|(_, at)| at)?;
            let gone = guests.1.swap_remove(longest);
            // Already closed is fine.
            let _ = gone.stream.shutdown(std::net::Shutdown::Both);
        }
        let id = guests.0;
        guests.0 += 1;
        guests.1.push(Guest { id, stream: handle, idle: Some(Instant::now()) });
        Some(Place { lobby: Arc::clone(lobby), id })
    }
}

/// One connection's place in the [`Lobby`]; given back when dropped.
struct Place {
    lobby: Arc<Lobby>,
    id: u64,
}

impl Place {
    /// Notes that a request is on its way (`idle` false) or that the connection waits for its next one. `false`
    /// when the place was given to another connection meanwhile: this one is closed and must end.
    fn set_idle(&self, idle: bool) -> bool {
        let mut guests = lock(&self.lobby.guests);
        let Some(guest) = guests.1.iter_mut().find(|guest| guest.id == self.id) else { return false };
        guest.idle = idle.then(Instant::now);
        true
    }
}

impl Drop for Place {
    fn drop(&mut self) {
        lock(&self.lobby.guests).1.retain(|guest| guest.id != self.id);
    }
}

/// A connection's incoming side, read against the clock.
// Agent: a socket's own read timeout is per read, so a caller that sends a byte every few seconds would never
// run into it. This gives each read only what is left until `until`.
struct Paced {
    stream: TcpStream,
    /// When the request being read must be complete; `None` while waiting for one to begin.
    until: Option<Instant>,
}

impl Paced {
    /// Waits for the next request to begin for `patience` at most.
    fn wait(&mut self, patience: Duration) -> std::io::Result<()> {
        self.until = None;
        self.stream.set_read_timeout(Some(patience))
    }

    /// Everything read from now on must have arrived within `patience`.
    fn hurry(&mut self, patience: Duration) {
        self.until = Some(Instant::now() + patience);
    }
}

impl Read for Paced {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if let Some(until) = self.until {
            // A timeout of zero is refused by the socket: no time left is the end.
            let left = until.checked_duration_since(Instant::now()).filter(|left| !left.is_zero()).ok_or(std::io::ErrorKind::TimedOut)?;
            self.stream.set_read_timeout(Some(left))?;
        }
        self.stream.read(buffer)
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
fn head_line(reader: &mut BufReader<Paced>, line: &mut String) -> Option<()> {
    let limit = u64::try_from(MOST_LINE + 1).unwrap_or(u64::MAX);
    let read = reader.by_ref().take(limit).read_line(line).ok()?;
    (read > 0 && read <= MOST_LINE).then_some(())
}

/// Reads the head of one request from `reader`, not its body; `None` when the connection ended or sent
/// something unreadable or too long.
// Human: the body is read only once the request has been let in, so a caller without the secret can't make
// the app take in 32 MB; nor can it send one endless header line, or endless headers.
fn read_head(reader: &mut BufReader<Paced>) -> Option<Head> {
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
fn refuse(out: &mut TcpStream, reader: &mut BufReader<Paced>, status: &str, why: &str) {
    if respond(out, status, Some(&json!({ "error": why })), false).is_err() {
        return;
    }
    let _ = out.shutdown(std::net::Shutdown::Write);
    let (most, patience) = REFUSED_DRAIN;
    reader.get_mut().hurry(patience);
    let _ = std::io::copy(&mut reader.by_ref().take(most), &mut std::io::sink());
}

/// Serves one connection until it closes, stalls in the middle of a request, or has been idle too long.
// Human: only a caller that shows the secret is served, and never a web page: a browser sends an `Origin`
// header with what a page asks for, and a page on the internet must not be able to reach into the app.
// Both are decided from the head, before the body is read.
// Human: between requests the connection may be quiet for `pace.idle`; from a request's first byte on, the
// whole of it must be there within `pace.request`. While the app works on the answer no clock runs here (the
// app has its own, PATIENCE), and the connection can't lose its place.
// Agent: BLOCKS until the connection ends; `seat` says when a request is under way, so the lobby never
// closes a connection in the middle of one.
fn serve(stream: TcpStream, shared: &Shared, seat: &Place, pace: Pace) {
    let Ok(mut out) = stream.try_clone() else { return };
    // A caller that stops taking an answer is let go too. If the limit can't be set the connection is served
    // without it.
    let _ = out.set_write_timeout(Some(pace.request));
    let mut reader = BufReader::new(Paced { stream, until: None });
    let secret = format!("Bearer {}", shared.token);
    loop {
        // The next request's first byte, however long the client keeps quiet within the idle time.
        if reader.get_mut().wait(pace.idle).is_err() || !reader.fill_buf().is_ok_and(|waiting| !waiting.is_empty()) || !seat.set_idle(false) {
            return;
        }
        reader.get_mut().hurry(pace.request);
        let Some(head) = read_head(&mut reader) else { return };
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
        if result.is_err() || !seat.set_idle(true) {
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
        std::thread::spawn(move || listen(&listener, &Arc::new(shared), PACE));
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
        std::thread::spawn(move || listen(&listener, &Arc::new(shared), PACE));
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

    /// A server at `pace` on a port of its own; returns the port.
    fn serving_at(pace: Pace) -> u16 {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (shared, _calls) = shared();
        std::thread::spawn(move || listen(&listener, &Arc::new(shared), pace));
        port
    }

    /// A connection to the server at `port` that gives up reading after five seconds, so a test fails rather
    /// than hangs.
    fn connect(port: u16) -> BufReader<TcpStream> {
        let stream = TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        BufReader::new(stream)
    }

    /// Sends a ping on the kept-open connection `client` and returns the answer's status line and body.
    fn ping_on(client: &mut BufReader<TcpStream>) -> (String, Value) {
        let request = format!("POST /mcp HTTP/1.1\r\nAuthorization: Bearer secret\r\nContent-Length: {}\r\n\r\n{PING}", PING.len());
        client.get_mut().write_all(request.as_bytes()).unwrap();
        let mut status = String::new();
        client.read_line(&mut status).unwrap();
        let mut length = 0;
        loop {
            let mut header = String::new();
            client.read_line(&mut header).unwrap();
            if header.trim_end().is_empty() {
                break;
            }
            if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        client.read_exact(&mut body).unwrap();
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    /// Whether the server has closed `client`: reading it ends instead of running into the test's own limit.
    fn closed(client: &mut BufReader<TcpStream>) -> bool {
        match client.get_mut().read(&mut [0; 16]) {
            Ok(read) => read == 0,
            Err(error) => !matches!(error.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut),
        }
    }

    #[test]
    fn a_request_that_stalls_part_way_is_closed_long_before_the_idle_time() {
        let port = serving_at(Pace { request: Duration::from_millis(200), idle: Duration::from_secs(60), most: 4 });
        // Half a head, then nothing; and a whole head whose body never comes in full.
        for part in ["POST /mcp HTTP/1.1\r\nAuthoriz", "POST /mcp HTTP/1.1\r\nAuthorization: Bearer secret\r\nContent-Length: 50\r\n\r\n{\"jsonrpc\""]
        {
            let started = Instant::now();
            let mut client = connect(port);
            client.get_mut().write_all(part.as_bytes()).unwrap();
            assert!(closed(&mut client), "{part}");
            assert!(started.elapsed() < Duration::from_secs(4), "{part}");
        }
        // A caller that sends a byte now and then doesn't get a new clock with each: the limit is for the whole request.
        let started = Instant::now();
        let mut client = connect(port);
        let drip = std::thread::spawn({
            let mut sender = client.get_ref().try_clone().unwrap();
            move || {
                for byte in b"POST /mcp HTTP/1.1\r\nX-Slow: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" {
                    if sender.write_all(&[*byte]).is_err() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        });
        assert!(closed(&mut client) && started.elapsed() < Duration::from_secs(2));
        drip.join().unwrap();
    }

    #[test]
    fn a_connection_kept_open_between_requests_still_works_within_the_idle_time() {
        // The idle time is far longer than a request may take, as it is for real.
        let port = serving_at(Pace { request: Duration::from_millis(150), idle: Duration::from_secs(60), most: 4 });
        let mut client = connect(port);
        let pong = json!({ "jsonrpc": "2.0", "id": 0, "result": {} });
        let (status, answer) = ping_on(&mut client);
        assert!(status.starts_with("HTTP/1.1 200") && answer == pong, "{status}");
        // Quiet for several times what a request may take, as a client is while its model thinks.
        std::thread::sleep(Duration::from_millis(600));
        let (status, answer) = ping_on(&mut client);
        assert!(status.starts_with("HTTP/1.1 200") && answer == pong, "{status}");
        // Past the idle time the connection is closed: also one that never said anything.
        let port = serving_at(Pace { request: Duration::from_secs(60), idle: Duration::from_millis(200), most: 4 });
        let (mut used, mut silent) = (connect(port), connect(port));
        assert!(ping_on(&mut used).0.starts_with("HTTP/1.1 200"));
        assert!(closed(&mut used) && closed(&mut silent));
    }

    #[test]
    fn a_full_server_lets_the_longest_idle_connection_go_for_a_new_one() {
        let port = serving_at(Pace { request: Duration::from_secs(5), idle: Duration::from_secs(60), most: 2 });
        // Two callers take both places and say nothing: the first has been idle longest.
        let mut first = connect(port);
        std::thread::sleep(Duration::from_millis(50));
        let mut second = connect(port);
        std::thread::sleep(Duration::from_millis(50));
        let mut third = connect(port);
        let (status, _) = ping_on(&mut third);
        assert!(status.starts_with("HTTP/1.1 200"), "{status}");
        assert!(closed(&mut first));
        // The other one kept its place and is served.
        assert!(ping_on(&mut second).0.starts_with("HTTP/1.1 200"));
    }

    #[test]
    fn a_connection_in_the_middle_of_a_request_never_loses_its_place() {
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).unwrap();
        let port = listener.local_addr().unwrap().port();
        let caller = || TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).unwrap();
        let lobby = Arc::new(Lobby::new(2));
        let (a, b, c) = (caller(), caller(), caller());
        let first = Lobby::enter(&lobby, &a).unwrap();
        let second = Lobby::enter(&lobby, &b).unwrap();
        // Both busy: a third caller finds no place.
        assert!(first.set_idle(false) && second.set_idle(false));
        assert!(Lobby::enter(&lobby, &c).is_none());
        // One waits for its next request again: it gives its place up, and is told so when it looks.
        assert!(second.set_idle(true));
        let third = Lobby::enter(&lobby, &c).unwrap();
        assert!(!second.set_idle(false) && first.set_idle(true) && third.set_idle(false));
        // A place given back is free again.
        drop((first, second, third));
        assert!(lock(&lobby.guests).1.is_empty() && Lobby::enter(&lobby, &a).is_some());
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
