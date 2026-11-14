//! Looks for a newer Pencraft among the releases on GitHub and asks the user whether they want it.
//!
//! The look runs on a thread of its own when the app starts and once a day after that. It never holds the
//! window up, and when GitHub can't be reached nothing is shown: the next look is a day later. A newer version
//! is offered in a small card with what is new in it; the user downloads it, skips that version, or decides later.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use egui::{Align2, RichText, Stroke, vec2};
use serde::Deserialize;

use crate::theme::{ISLAND_PADDING, ISLAND_RADIUS, Palette, SIDEBAR_TEXT_SIZE, UPDATE_GAP, UPDATE_MARGIN, UPDATE_NOTES_HEIGHT, UPDATE_WIDTH};

/// The GitHub repository whose releases are the updates.
pub const REPOSITORY: &str = "AsP3X/pencraft";
/// The version of the running app, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Where GitHub answers questions about repositories.
const API: &str = "https://api.github.com";
/// How long the app waits between two looks while it stays open.
const EVERY: Duration = Duration::from_hours(24);
/// How long one look may take before it gives up.
const PATIENCE: Duration = Duration::from_secs(20);

/// A release that is newer than the running app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// The version without the tag's leading `v`.
    pub version: String,
    /// What the release says is new, as its author wrote it (Markdown).
    pub notes: String,
    /// The address the browser is sent to: the installer for this system, or the release's page without one.
    pub download: String,
}

/// What the user chose in the card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Download it: open this address in the browser.
    Get(String),
    /// Never offer this version again.
    Skip(String),
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Deserialize)]
struct ApiAsset {
    name: String,
    #[serde(default)]
    browser_download_url: String,
}

/// Reads what GitHub says about a release.
///
/// # Errors
/// When the answer isn't a release.
pub fn parse_release(json: &str) -> Result<Release, String> {
    let api: ApiRelease = serde_json::from_str(json).map_err(|error| format!("unexpected answer from GitHub: {error}"))?;
    let installer = api.assets.iter().find(|asset| is_installer_for_this_system(&asset.name)).map(|asset| asset.browser_download_url.as_str());
    // Human: the browser is only ever sent to this repository's own pages, whatever the answer says.
    let download =
        [installer, Some(api.html_url.as_str())].into_iter().flatten().find(|address| is_ours(address)).map_or_else(releases_page, str::to_owned);
    Ok(Release { version: api.tag_name.trim_start_matches(['v', 'V']).to_owned(), notes: api.body.unwrap_or_default().trim().to_owned(), download })
}

/// The page that lists every release, for when a release names no page of its own.
fn releases_page() -> String {
    format!("https://github.com/{REPOSITORY}/releases/latest")
}

/// Whether an address is a page or a file of this repository on GitHub, and plain enough to hand to a browser.
fn is_ours(address: &str) -> bool {
    let start = format!("https://github.com/{REPOSITORY}/");
    let plain = address.chars().all(|c| c.is_ascii_alphanumeric() || "/:.-_".contains(c));
    plain && address.get(..start.len()).is_some_and(|head| head.eq_ignore_ascii_case(&start))
}

/// Whether a release file is the installer for 64-bit Windows: `pencraft-<version>-setup.exe`.
#[must_use]
pub fn is_windows_installer(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with("-setup.exe")
}

/// Whether a release file is the disk image for a Mac: `pencraft-<version>-macos-universal.dmg`.
#[must_use]
pub fn is_mac_image(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".dmg")
}

fn is_installer_for_this_system(name: &str) -> bool {
    (cfg!(all(windows, target_arch = "x86_64")) && is_windows_installer(name)) || (cfg!(target_os = "macos") && is_mac_image(name))
}

/// "1.2.3" or "v1.2.3" as numbers. What follows a `-` or `+` is ignored.
#[must_use]
pub fn parse_version(text: &str) -> Option<[u64; 3]> {
    let text = text.trim().trim_start_matches(['v', 'V']);
    let core = text.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let mut numbers = [0u64; 3];
    for number in &mut numbers {
        *number = parts.next()?.parse().ok()?;
    }
    parts.next().is_none().then_some(numbers)
}

/// Whether `candidate` is a higher version than `current`. A tag that isn't a version is never newer.
#[must_use]
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

/// Asks GitHub for the latest release, with a sign-in or without; gives the status and the text of the answer.
type Fetch<'a> = &'a dyn Fn(Option<&str>) -> Result<(u16, String), String>;

/// Finds the latest release and gives it when it is newer than `current`.
///
/// `fetch` asks GitHub for the latest release, with a sign-in or without, and gives the status and the text of
/// the answer. `token` finds a sign-in; it is only asked when GitHub says there is nothing to see without one,
/// which is what it says about a private repository.
///
/// # Errors
/// When GitHub couldn't be reached, refused, or answered something that isn't a release.
pub fn latest(current: &str, fetch: Fetch<'_>, token: impl FnOnce() -> Option<String>) -> Result<Option<Release>, String> {
    let (mut status, mut body) = fetch(None)?;
    if status == 404
        && let Some(token) = token()
    {
        (status, body) = fetch(Some(&token))?;
    }
    // Human: nothing released yet, or a repository this computer may not see: not an error, just no news.
    if status == 404 {
        return Ok(None);
    }
    if !(200..300).contains(&status) {
        return Err(format!("GitHub answered {status}"));
    }
    let release = parse_release(&body)?;
    Ok(is_newer(&release.version, current).then_some(release))
}

/// Asks for `address` the way GitHub's API wants to be asked; gives the status and the text of the answer.
// Agent: BLOCKS for up to PATIENCE; SENDS the sign-in only to `address` itself (ureq drops it when redirected elsewhere).
fn get(address: &str, token: Option<&str>) -> Result<(u16, String), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(PATIENCE)).build().into();
    let mut request = agent
        .get(address)
        .header("User-Agent", concat!("pencraft/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let mut response = request.call().map_err(|error| format!("GitHub couldn't be reached: {error}"))?;
    let status = response.status().as_u16();
    let body = response.body_mut().read_to_string().map_err(|error| format!("GitHub's answer couldn't be read: {error}"))?;
    Ok((status, body))
}

/// A GitHub sign-in on this computer: `GH_TOKEN`, `GITHUB_TOKEN`, or the one the GitHub CLI holds.
// Human: only people who may see a private repository have one that helps; everyone else gets "no news".
// Agent: RUNS `gh auth token` (no window on Windows); the token is never logged or saved.
fn token() -> Option<String> {
    let filled = |text: String| Some(text.trim().to_owned()).filter(|text| !text.is_empty());
    if let Some(token) = ["GH_TOKEN", "GITHUB_TOKEN"].into_iter().find_map(|name| std::env::var(name).ok().and_then(filled)) {
        return Some(token);
    }
    let mut command = std::process::Command::new("gh");
    command.args(["auth", "token"]).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // Human: CREATE_NO_WINDOW, so no terminal window flashes up beside the app.
        command.creation_flags(0x0800_0000);
    }
    let output = command.output().ok()?;
    output.status.success().then(|| String::from_utf8(output.stdout).ok()).flatten().and_then(filled)
}

/// Opens an address in the user's browser.
///
/// # Errors
/// When the system's opener couldn't be started.
pub fn open_in_browser(address: &str) -> Result<(), String> {
    let mut command = if cfg!(windows) {
        let mut command = std::process::Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler");
        command
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open")
    } else {
        std::process::Command::new("xdg-open")
    };
    command.arg(address).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    command.spawn().map(|_| ()).map_err(|error| format!("The browser couldn't be opened: {error}"))
}

/// What one look found: a newer release, nothing newer, or why it couldn't look.
type Found = Result<Option<Release>, String>;

/// The app's knowledge about a newer version, and the card that offers it.
pub struct Updates {
    /// Whether this run may look at all: not a picture, a private or a background run.
    allowed: bool,
    /// When the last look started.
    looked: Option<Instant>,
    /// A look is under way.
    looking: bool,
    /// The user asked for the look under way, so its answer is said whatever it is.
    asked: bool,
    /// The newer release the card shows.
    offered: Option<Release>,
    answers: Receiver<Found>,
    send: Sender<Found>,
}

impl Updates {
    /// `allowed` is false for runs that must not go online or show anything by themselves. `posed` shows the
    /// card with a made-up release, for a picture of it (`--update-prompt`).
    #[must_use]
    pub fn new(allowed: bool, posed: bool) -> Updates {
        let (send, answers) = channel();
        let mut updates = Updates { allowed, looked: None, looking: false, asked: false, offered: None, answers, send };
        if posed {
            updates.pose();
        }
        updates
    }

    /// Puts a made-up release into the card.
    fn pose(&mut self) {
        let notes = "A sharper canvas, and exports that keep their colours.\n\n### Added\n\n- **Export to PDF.** Every top-level layer becomes a page.\n- Find layers by the text they hold.\n\n### Fixed\n\n- Shadows no longer cut off at the edge of a frame.";
        self.offered = Some(Release { version: "9.1.0".to_owned(), notes: notes.to_owned(), download: releases_page() });
    }

    /// Whether it is time to look: at the start, and once a day while the app stays open.
    fn due(&self, wanted: bool, now: Instant) -> bool {
        self.allowed && wanted && !self.looking && self.looked.is_none_or(|last| now.duration_since(last) >= EVERY)
    }

    /// Takes in what a look found. `skipped` is the version the user never wants offered. Gives a sentence for
    /// the user when they asked for the look themselves; a look the app started on its own stays quiet.
    fn take(&mut self, found: Found, skipped: &str) -> Option<String> {
        self.looking = false;
        let asked = std::mem::take(&mut self.asked);
        match found {
            Ok(Some(release)) => {
                if asked || release.version != skipped {
                    self.offered = Some(release);
                }
                None
            }
            Ok(None) => asked.then(|| format!("Pencraft {VERSION} is the latest version.")),
            Err(error) => {
                log::info!("update check: {error}");
                asked.then(|| format!("Couldn't look for a new version. {error}"))
            }
        }
    }

    /// Starts a look on a thread of its own.
    // Agent: SPAWNS one short-lived thread; WAKES the window through `ctx` when the answer is there.
    fn look(&mut self, ctx: &egui::Context) {
        self.looked = Some(Instant::now());
        self.looking = true;
        let (send, ctx) = (self.send.clone(), ctx.clone());
        let work = move || {
            let address = format!("{API}/repos/{REPOSITORY}/releases/latest");
            // Human: the app may have quit meanwhile; then nobody waits for the answer.
            let _ = send.send(latest(VERSION, &|token| get(&address, token), token));
            ctx.request_repaint();
        };
        if let Err(error) = std::thread::Builder::new().name("update-check".to_owned()).spawn(work) {
            log::info!("update check: no thread to look on: {error}");
            self.looking = false;
        }
    }

    /// Looks now because the user asked to, whatever the setting says; the answer is always said.
    pub fn look_now(&mut self, ctx: &egui::Context) {
        if !self.looking {
            self.look(ctx);
        }
        self.asked = true;
    }

    /// Once a frame: takes in a finished look and starts one when it is time. `wanted` is the setting,
    /// `skipped` the version the user never wants offered. Gives a sentence to show the user, if there is one.
    pub fn tick(&mut self, ctx: &egui::Context, wanted: bool, skipped: &str) -> Option<String> {
        let mut say = None;
        while let Ok(found) = self.answers.try_recv() {
            say = self.take(found, skipped).or(say);
        }
        if self.due(wanted, Instant::now()) {
            self.look(ctx);
        }
        say
    }

    /// Draws the card while a newer version is on offer, and gives what the user chose in it.
    pub fn show(&mut self, ctx: &egui::Context, palette: Palette) -> Option<Answer> {
        let release = self.offered.as_ref()?;
        let (mut get, mut skip, mut later) = (false, false, false);
        egui::Area::new(egui::Id::unique("update"))
            .anchor(Align2::RIGHT_BOTTOM, vec2(-UPDATE_MARGIN, -UPDATE_MARGIN))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(palette.island)
                    .stroke(Stroke::new(1.0, palette.outline))
                    .corner_radius(ISLAND_RADIUS)
                    .inner_margin(ISLAND_PADDING)
                    .show(ui, |ui| {
                        ui.set_width(UPDATE_WIDTH);
                        ui.label(RichText::new(format!("Pencraft {} is available", release.version)).strong());
                        ui.label(RichText::new(format!("You have {VERSION}.")).size(SIDEBAR_TEXT_SIZE).color(palette.muted));
                        if !release.notes.is_empty() {
                            ui.add_space(UPDATE_GAP);
                            egui::ScrollArea::vertical().max_height(UPDATE_NOTES_HEIGHT).auto_shrink([false, true]).show(ui, |ui| {
                                notes(ui, &release.notes, palette);
                            });
                        }
                        ui.add_space(UPDATE_GAP);
                        ui.horizontal(|ui| {
                            let download = egui::Button::new(RichText::new("Download").color(palette.primary_text)).fill(palette.primary);
                            get = ui.add(download).clicked();
                            skip = ui.button("Skip this version").clicked();
                            later = ui.button("Later").clicked();
                        });
                    });
            });
        if !(get || skip || later) {
            return None;
        }
        let release = self.offered.take()?;
        if get {
            Some(Answer::Get(release.download))
        } else if skip {
            Some(Answer::Skip(release.version))
        } else {
            None
        }
    }
}

/// One line of release notes as the card shows it: a heading or plain text, without Markdown's marks.
#[derive(Debug, PartialEq, Eq)]
enum NoteLine {
    Heading(String),
    Text(String),
    Gap,
}

/// Release notes line by line. Headings lose their `#`, bold and code lose their marks, and several empty
/// lines count as one.
fn note_lines(notes: &str) -> Vec<NoteLine> {
    let mut lines: Vec<NoteLine> = Vec::new();
    for line in notes.lines().map(str::trim_end) {
        let plain = line.replace("**", "").replace('`', "");
        let line = if plain.trim().is_empty() {
            NoteLine::Gap
        } else if plain.starts_with('#') {
            NoteLine::Heading(plain.trim_start_matches('#').trim().to_owned())
        } else {
            NoteLine::Text(plain)
        };
        if line != NoteLine::Gap || lines.last().is_some_and(|last| *last != NoteLine::Gap) {
            lines.push(line);
        }
    }
    lines
}

/// Draws release notes.
fn notes(ui: &mut egui::Ui, text: &str, palette: Palette) {
    for line in note_lines(text) {
        match line {
            NoteLine::Heading(text) => {
                ui.add(egui::Label::new(RichText::new(text).size(SIDEBAR_TEXT_SIZE).strong()).wrap());
            }
            NoteLine::Text(text) => {
                ui.add(egui::Label::new(RichText::new(text).size(SIDEBAR_TEXT_SIZE).color(palette.muted)).wrap());
            }
            NoteLine::Gap => ui.add_space(UPDATE_GAP / 2.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::io::{Read as _, Write as _};

    const NEWER: &str = r#"{
        "tag_name": "v9.1.0",
        "html_url": "https://github.com/AsP3X/pencraft/releases/tag/v9.1.0",
        "body": "\n- New things\n",
        "assets": [
            {"name": "pencraft-9.1.0-setup.exe", "browser_download_url": "https://github.com/AsP3X/pencraft/releases/download/v9.1.0/pencraft-9.1.0-setup.exe"},
            {"name": "pencraft-9.1.0-x64.msi", "browser_download_url": "https://github.com/AsP3X/pencraft/releases/download/v9.1.0/pencraft-9.1.0-x64.msi"},
            {"name": "pencraft-9.1.0-macos-universal.dmg", "browser_download_url": "https://github.com/AsP3X/pencraft/releases/download/v9.1.0/pencraft-9.1.0-macos-universal.dmg"},
            {"name": "SHA256SUMS.txt", "browser_download_url": "https://github.com/AsP3X/pencraft/releases/download/v9.1.0/SHA256SUMS.txt"}
        ]
    }"#;

    fn release(version: &str) -> Release {
        Release { version: version.to_owned(), notes: "- New things".to_owned(), download: releases_page() }
    }

    #[test]
    fn versions_compare_by_number() {
        assert_eq!(parse_version("v1.2.3"), Some([1, 2, 3]));
        assert_eq!(parse_version("0.10.0-beta.1"), Some([0, 10, 0]));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("latest"), None);
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(is_newer("v1.0.0", "0.99.99"));
        assert!(!is_newer("0.3.0", "0.3.0"));
        assert!(!is_newer("0.2.9", "0.3.0"));
        // A tag that is not a version never counts as newer.
        assert!(!is_newer("nightly", "0.3.0"));
        assert!(parse_version(VERSION).is_some());
    }

    #[test]
    fn a_release_names_its_version_its_notes_and_the_download_for_this_system() {
        let release = parse_release(NEWER).unwrap();
        assert_eq!(release.version, "9.1.0");
        assert_eq!(release.notes, "- New things");
        if cfg!(all(windows, target_arch = "x86_64")) {
            assert!(release.download.ends_with("/pencraft-9.1.0-setup.exe"));
        } else if cfg!(target_os = "macos") {
            assert!(release.download.ends_with("/pencraft-9.1.0-macos-universal.dmg"));
        } else {
            assert!(release.download.ends_with("/releases/tag/v9.1.0"));
        }
        assert!(is_windows_installer("pencraft-9.1.0-setup.exe"));
        assert!(!is_windows_installer("pencraft-9.1.0-x64.exe"));
        assert!(!is_windows_installer("pencraft-9.1.0-x64.msi"));
        assert!(is_mac_image("pencraft-9.1.0-macos-universal.dmg"));

        // A release without files or notes still reads, and leads to the list of releases.
        let bare = parse_release(r#"{"tag_name": "v9.2.0"}"#).unwrap();
        assert_eq!(bare, Release { version: "9.2.0".to_owned(), notes: String::new(), download: releases_page() });
        assert!(parse_release("not json").is_err());
    }

    #[test]
    fn the_browser_is_only_sent_to_this_repository() {
        assert!(is_ours("https://github.com/AsP3X/pencraft/releases/tag/v9.1.0"));
        assert!(is_ours("https://github.com/asp3x/Pencraft/releases/download/v9.1.0/pencraft-9.1.0-setup.exe"));
        assert!(!is_ours("https://github.com/someone/else/releases/tag/v9.1.0"));
        assert!(!is_ours("https://github.com/AsP3X/pencraft-evil/releases"));
        assert!(!is_ours("http://github.com/AsP3X/pencraft/releases"));
        assert!(!is_ours("https://github.com/AsP3X/pencraft/releases?x=1 & calc.exe"));
        assert!(!is_ours("file:///C:/Windows/System32/calc.exe"));
        let elsewhere = r#"{"tag_name": "v9.1.0", "html_url": "https://example.com/get", "assets": [
            {"name": "pencraft-9.1.0-setup.exe", "browser_download_url": "https://example.com/pencraft-9.1.0-setup.exe"},
            {"name": "pencraft-9.1.0-macos-universal.dmg", "browser_download_url": "https://example.com/pencraft.dmg"}]}"#;
        assert_eq!(parse_release(elsewhere).unwrap().download, releases_page());
    }

    #[test]
    fn only_a_higher_version_is_news() {
        let ok = |_: Option<&str>| Ok((200, NEWER.to_owned()));
        assert_eq!(latest("9.0.9", &ok, || None).unwrap().unwrap().version, "9.1.0");
        assert_eq!(latest("9.1.0", &ok, || None).unwrap(), None);
        assert_eq!(latest("10.0.0", &ok, || None).unwrap(), None);
        // GitHub out of reach, refusing, or talking nonsense is an error for the caller to keep quiet about.
        assert!(latest("0.1.0", &|_| Err("offline".to_owned()), || None).is_err());
        assert_eq!(latest("0.1.0", &|_| Ok((403, "rate limit".to_owned())), || None).unwrap_err(), "GitHub answered 403");
        assert!(latest("0.1.0", &|_| Ok((200, "<html>".to_owned())), || None).is_err());
    }

    #[test]
    fn a_sign_in_is_only_used_when_there_is_nothing_to_see_without_one() {
        // Human: a public repository answers at once, and nobody is asked for a sign-in.
        let asked = RefCell::new(Vec::new());
        let open = |token: Option<&str>| {
            asked.borrow_mut().push(token.map(str::to_owned));
            Ok((200, NEWER.to_owned()))
        };
        let never = || -> Option<String> { panic!("no sign-in is needed") };
        assert!(latest("0.1.0", &open, never).unwrap().is_some());
        assert_eq!(*asked.borrow(), vec![None]);

        // Human: a private one says 404 first, and answers the same question asked with a sign-in.
        let asked = RefCell::new(Vec::new());
        let private = |token: Option<&str>| {
            asked.borrow_mut().push(token.map(str::to_owned));
            Ok(if token == Some("secret") { (200, NEWER.to_owned()) } else { (404, String::new()) })
        };
        assert!(latest("0.1.0", &private, || Some("secret".to_owned())).unwrap().is_some());
        assert_eq!(*asked.borrow(), vec![None, Some("secret".to_owned())]);

        // Human: without a sign-in, or with one that doesn't help, there is simply no news.
        assert_eq!(latest("0.1.0", &private, || None).unwrap(), None);
        assert_eq!(latest("0.1.0", &private, || Some("wrong".to_owned())).unwrap(), None);
    }

    /// Answers one request with `reply` and hands back what was asked, in lowercase.
    fn serve_once(reply: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}/repos/x/y/releases/latest", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut seen = Vec::new();
            let mut chunk = [0u8; 1024];
            while !seen.windows(4).any(|window| window == b"\r\n\r\n") {
                let size = stream.read(&mut chunk).unwrap();
                if size == 0 {
                    break;
                }
                seen.extend_from_slice(&chunk[..size]);
            }
            stream.write_all(reply.as_bytes()).unwrap();
            String::from_utf8_lossy(&seen).to_lowercase()
        });
        (address, server)
    }

    #[test]
    fn a_question_carries_the_apps_name_and_a_sign_in_only_when_given() {
        let (address, server) = serve_once("HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
        assert_eq!(get(&address, Some("secret")).unwrap(), (200, "ok".to_owned()));
        let asked = server.join().unwrap();
        assert!(asked.contains(&format!("user-agent: pencraft/{VERSION}")));
        assert!(asked.contains("accept: application/vnd.github+json"));
        assert!(asked.contains("authorization: bearer secret"));

        // Human: a refusal comes back as an answer, not an error, and without a sign-in none is sent.
        let (address, server) = serve_once("HTTP/1.1 404 Not Found\r\nContent-Length: 4\r\nConnection: close\r\n\r\nnope");
        assert_eq!(get(&address, None).unwrap(), (404, "nope".to_owned()));
        assert!(!server.join().unwrap().contains("authorization"));

        // Human: nobody listening is an error, said in words.
        let gone = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        assert!(get(&format!("http://{gone}/"), None).unwrap_err().starts_with("GitHub couldn't be reached"));
    }

    #[test]
    fn the_app_looks_at_the_start_and_once_a_day_and_never_in_a_quiet_run() {
        let start = Instant::now();
        let mut updates = Updates::new(true, false);
        assert!(updates.due(true, start));
        // The setting switches it off.
        assert!(!updates.due(false, start));
        updates.looked = Some(start);
        updates.looking = true;
        assert!(!updates.due(true, start + EVERY));
        updates.looking = false;
        assert!(!updates.due(true, start + Duration::from_hours(23)));
        assert!(updates.due(true, start + EVERY));
        // A picture, a private or a background run never looks.
        assert!(!Updates::new(false, false).due(true, start));
    }

    #[test]
    fn a_newer_version_is_offered_unless_it_was_skipped() {
        let mut updates = Updates::new(true, false);
        updates.looking = true;
        assert_eq!(updates.take(Ok(Some(release("9.1.0"))), ""), None);
        assert!(!updates.looking);
        assert_eq!(updates.offered, Some(release("9.1.0")));

        let mut updates = Updates::new(true, false);
        assert_eq!(updates.take(Ok(Some(release("9.1.0"))), "9.1.0"), None);
        assert_eq!(updates.offered, None);
        // The version after the skipped one is offered again.
        assert_eq!(updates.take(Ok(Some(release("9.2.0"))), "9.1.0"), None);
        assert_eq!(updates.offered, Some(release("9.2.0")));
    }

    #[test]
    fn a_look_the_app_started_is_quiet_and_one_the_user_asked_for_is_answered() {
        let mut updates = Updates::new(true, false);
        assert_eq!(updates.take(Err("offline".to_owned()), ""), None);
        assert_eq!(updates.take(Ok(None), ""), None);
        assert_eq!(updates.offered, None);

        updates.asked = true;
        assert_eq!(
            updates.take(Err("GitHub couldn't be reached: offline".to_owned()), "").unwrap(),
            "Couldn't look for a new version. GitHub couldn't be reached: offline"
        );
        // Asking counts once.
        assert!(!updates.asked);
        updates.asked = true;
        assert_eq!(updates.take(Ok(None), "").unwrap(), format!("Pencraft {VERSION} is the latest version."));
        // Asking shows a version that was skipped before.
        updates.asked = true;
        assert_eq!(updates.take(Ok(Some(release("9.1.0"))), "9.1.0"), None);
        assert_eq!(updates.offered, Some(release("9.1.0")));
    }

    #[test]
    fn release_notes_are_shown_without_markdowns_marks() {
        let lines = note_lines("A sharper canvas.\n\n\n### Added\n\n- **Export to PDF.** Uses `--export`.\n");
        assert_eq!(
            lines,
            vec![
                NoteLine::Text("A sharper canvas.".to_owned()),
                NoteLine::Gap,
                NoteLine::Heading("Added".to_owned()),
                NoteLine::Gap,
                NoteLine::Text("- Export to PDF. Uses --export.".to_owned()),
            ]
        );
        assert_eq!(note_lines("\n\n"), Vec::new());
    }
}
