//! Looks for a newer Pencraft among the releases on GitHub and asks the user whether they want it.
//!
//! The look runs on a thread of its own when the app starts and then as often as the settings say, also while
//! the window is left alone. It never holds the window up, and when GitHub can't be reached nothing is shown:
//! the next look comes in its turn. A newer version is offered in a small card with what is new in it; the
//! user takes it, skips that version, decides later (the next look asks again), or folds the card away into a
//! small pill. Taking it downloads the installer, installs it and starts the new version ([`install`]); a copy
//! that can't replace itself sends the user to the browser instead.
//!
//! A release can be marked as required (a line `<!-- required -->` in its notes). Then everyone on an older
//! version is asked in the middle of the window, and can't skip, postpone or fold away: only update, or quit.
//! When the download fails, the user may go on for now, so nobody is locked out of their work by a network.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::time::{Duration, Instant};

use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Align2, FontId, Layout, Rect, RichText, Sense, Stroke, Ui, Vec2, vec2};
use serde::Deserialize;

pub use install::Prepared;
use install::{Place, Progress};

mod install;

use crate::icons;
use crate::theme::{
    CHIP_RADIUS, CONTROL_RADIUS, DIALOG_BACKDROP, ICON_SIZE, ISLAND_PADDING, ISLAND_RADIUS, Palette, SIDEBAR_TEXT_SIZE, UPDATE_BADGE, UPDATE_BAR,
    UPDATE_BAR_GLIDE, UPDATE_BAR_PIECE, UPDATE_BAR_SWEEP, UPDATE_BULLET, UPDATE_BUTTON_HEIGHT, UPDATE_BUTTON_PADDING, UPDATE_GAP, UPDATE_MARGIN,
    UPDATE_NOTE_GAP, UPDATE_NOTES_HEIGHT, UPDATE_PILL, UPDATE_PILL_PADDING, UPDATE_TITLE_SIZE, UPDATE_WIDTH,
};

/// The GitHub repository whose releases are the updates.
pub const REPOSITORY: &str = "AsP3X/pencraft";
/// The version of the running app, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Where GitHub answers questions about repositories.
const API: &str = "https://api.github.com";
/// How often the app looks while it stays open, as the settings name the choices; the first is the default.
// Human: a look is one small question to GitHub, two when there is a newer release. GitHub answers 60 an hour
// to one address without a sign-in, so every 5 minutes (at most 24 an hour) stays well inside that.
pub const EVERY: [(&str, Duration); 4] = [
    ("Every 5 minutes", Duration::from_mins(5)),
    ("Every 15 minutes", Duration::from_mins(15)),
    ("Every hour", Duration::from_hours(1)),
    ("Every day", Duration::from_hours(24)),
];
/// The line in a release's notes that says nobody may stay on an older version.
pub const REQUIRED: &str = "<!-- required -->";
/// How many of the latest releases are looked through for a required one the user would otherwise skip over.
const RECENT: usize = 30;
/// How long one look may take before it gives up.
const PATIENCE: Duration = Duration::from_secs(20);

/// The name of the release file that lists the checksums of the others.
const CHECKSUMS: &str = "SHA256SUMS.txt";
/// How often the card looks at a download to show how far it is.
const GLANCE: Duration = Duration::from_millis(100);

/// A file of a release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseFile {
    /// GitHub's number for the file; the download asks for it by this.
    pub id: u64,
    /// The file's name, as the release's checksums list it.
    pub name: String,
    /// The file's size in bytes, 0 when GitHub didn't say.
    pub size: u64,
}

/// A release that is newer than the running app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// The version without the tag's leading `v`.
    pub version: String,
    /// What the release says is new, as its author wrote it (Markdown).
    pub notes: String,
    /// The address the browser is sent to: the installer for this system, or the release's page without one.
    pub download: String,
    /// The installer for this system, when the release has one.
    pub installer: Option<ReleaseFile>,
    /// The file with the checksums of the release's files, when the release has one.
    pub checksums: Option<ReleaseFile>,
    /// Nobody may stay on an older version: this release, or one between it and the running app, says so.
    pub required: bool,
}

/// What the user chose in the card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Download it: open this address in the browser.
    Get(String),
    /// Quit, install this downloaded update, and start again.
    Install(Prepared),
    /// Never offer this version again.
    Skip(String),
    /// Quit the app: the answer to a required update the user doesn't take now.
    Quit,
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
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// How long the app waits between two looks, for the settings' choice `name`.
#[must_use]
pub fn every(name: &str) -> Duration {
    EVERY.iter().find(|(known, _)| *known == name).map_or(EVERY[0].1, |(_, every)| *every)
}

/// Whether notes say their release is required: the marker on a line of its own.
fn says_required(notes: &str) -> bool {
    notes.lines().any(|line| line.trim() == REQUIRED)
}

/// Whether one of the releases GitHub lists in `json` is required and newer than `current`: then the user has
/// to update even when the latest release itself is an ordinary one.
// Human: someone three versions behind must not get past a required release because a later one is optional.
#[must_use]
pub fn any_required(current: &str, json: &str) -> bool {
    let Ok(releases) = serde_json::from_str::<Vec<ApiRelease>>(json) else { return false };
    releases.iter().any(|release| {
        !release.draft && !release.prerelease && is_newer(&release.tag_name, current) && release.body.as_deref().is_some_and(says_required)
    })
}

#[derive(Deserialize)]
struct ApiAsset {
    #[serde(default)]
    id: u64,
    name: String,
    #[serde(default)]
    size: u64,
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
    // Human: a file without a number can't be asked for, so it counts as not there.
    let file = |wanted: &dyn Fn(&str) -> bool| {
        let asset = api.assets.iter().find(|asset| asset.id != 0 && wanted(&asset.name))?;
        Some(ReleaseFile { id: asset.id, name: asset.name.clone(), size: asset.size })
    };
    let body = api.body.unwrap_or_default();
    // Human: the marker is for the app; the notes the user reads are without it.
    let notes: Vec<&str> = body.lines().filter(|line| line.trim() != REQUIRED).collect();
    Ok(Release {
        version: api.tag_name.trim_start_matches(['v', 'V']).to_owned(),
        notes: notes.join("\n").trim().to_owned(),
        required: says_required(&body),
        download,
        installer: file(&is_installer_for_this_system),
        checksums: file(&|name| name == CHECKSUMS),
    })
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

/// A command that reads nothing and, on Windows, shows no terminal window beside the app.
fn quiet(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    command.stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        // Human: CREATE_NO_WINDOW.
        command.creation_flags(0x0800_0000);
    }
    command
}

/// A GitHub sign-in on this computer: `GH_TOKEN`, `GITHUB_TOKEN`, or the one the GitHub CLI holds.
// Human: only people who may see a private repository have one that helps; everyone else gets "no news".
// Agent: RUNS `gh auth token` (no window on Windows); the token is never logged or saved.
fn token() -> Option<String> {
    let filled = |text: String| Some(text.trim().to_owned()).filter(|text| !text.is_empty());
    if let Some(token) = ["GH_TOKEN", "GITHUB_TOKEN"].into_iter().find_map(|name| std::env::var(name).ok().and_then(filled)) {
        return Some(token);
    }
    let mut command = quiet("gh");
    command.args(["auth", "token"]).stderr(std::process::Stdio::null());
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

/// What is known about a newer version, as the settings window says it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seen {
    /// A look is under way.
    pub looking: bool,
    /// How long ago the last look started; `None` when there was none in this run.
    pub ago: Option<Duration>,
    /// The newer version that is offered, if one is.
    pub offered: Option<String>,
}

/// The app's knowledge about a newer version, and the card that offers it.
#[expect(clippy::struct_excessive_bools, reason = "each says one independent thing about the look or the card")]
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
    /// How this copy can be replaced by a newer one; `None` sends the user to the browser.
    place: Option<Place>,
    /// How far taking the offered release is.
    stage: Stage,
    /// The card is folded away into its pill.
    collapsed: bool,
    /// The last download of the offered release failed: a required update may then be put off.
    failed: bool,
    /// The sending end of a made-up download's result (`--update-download`), kept so the card goes on
    /// waiting for it: nothing is ever sent.
    staged: Option<Sender<Result<Prepared, String>>>,
    answers: Receiver<Found>,
    send: Sender<Found>,
}

/// How far taking the offered release is.
enum Stage {
    /// The user hasn't decided.
    Offered,
    /// The installer is being downloaded on a thread of its own.
    Fetching {
        /// How far it is, and the way to stop it.
        progress: Arc<Progress>,
        /// Where the thread says how it ended.
        result: Receiver<Result<Prepared, String>>,
    },
    /// The installer has just arrived and is checked: quit and install it without another click.
    Arrived(Prepared),
    /// The installer is there, and the user kept the app open; it waits for a click.
    Ready(Prepared),
}

/// What the user clicked in the card.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Click {
    /// The first button: take the update, or restart into it.
    Take,
    Skip,
    Later,
    /// Stop the download.
    Cancel,
    /// Fold the card away into its pill.
    Collapse,
    /// Open the card again from its pill.
    Expand,
    /// Quit the app instead of taking a required update.
    Quit,
}

impl Updates {
    /// `allowed` is false for runs that must not go online or show anything by themselves. `posed` shows the
    /// card with a made-up release, for a picture of it (`--update-prompt`).
    #[must_use]
    pub fn new(allowed: bool, posed: bool) -> Updates {
        let (send, answers) = channel();
        let (place, stage) = (install::place(), Stage::Offered);
        let mut updates = Updates {
            allowed,
            looked: None,
            looking: false,
            asked: false,
            offered: None,
            place,
            stage,
            collapsed: false,
            failed: false,
            staged: None,
            answers,
            send,
        };
        if posed {
            updates.pose();
        }
        updates
    }

    /// Puts a made-up release into the card.
    fn pose(&mut self) {
        let notes = "A sharper canvas, and exports that keep their colours.\n\n### Added\n\n- **Export to PDF.** Every top-level layer becomes a page.\n- Find layers by the text they hold.\n\n### Fixed\n\n- Shadows no longer cut off at the edge of a frame.";
        self.offered = Some(Release {
            version: "9.1.0".to_owned(),
            notes: notes.to_owned(),
            download: releases_page(),
            installer: None,
            checksums: None,
            required: false,
        });
    }

    /// Sets the card up as the command line asks, for a picture or to watch it: in the middle of a made-up
    /// `download` (how far in per cent, and whether it goes on by itself: `--update-download`), as the card of
    /// an update that is `required` (`--update-required`), and `collapsed` into its pill (`--update-collapsed`).
    pub fn pose_as(&mut self, download: Option<(u8, bool)>, required: bool, collapsed: bool) {
        if let Some((percent, live)) = download {
            self.pose_download(percent, live);
        }
        if (required || collapsed) && self.offered.is_none() {
            self.pose();
        }
        if let Some(release) = self.offered.as_mut().filter(|_| required) {
            release.required = true;
        }
        self.collapsed = collapsed;
    }

    /// Puts the card into the middle of a made-up download that has come `percent` far (`--update-download`):
    /// at 100 the file is "being made ready". Nothing is fetched. With `live` the download goes on by itself,
    /// from the start to the end and over again, so the bar can be watched moving; without, it stands where
    /// it is, for a picture.
    // Human: a real download only shows in an installed copy that is offered a newer version, which a
    // development build never is. This is the way to see the bar without one.
    // Agent: with `live` SPAWNS one thread, which ends when the card's Cancel is clicked or the app quits.
    fn pose_download(&mut self, percent: u8, live: bool) {
        self.pose();
        let progress = Arc::new(Progress::of(STAGED_SIZE));
        progress.arrive(STAGED_SIZE / 100 * u64::from(percent.min(100)));
        let (keep, result) = channel();
        self.staged = Some(keep);
        self.stage = Stage::Fetching { progress: Arc::clone(&progress), result };
        if live {
            let started = std::thread::Builder::new().name("staged-download".to_owned()).spawn(move || staged_download(&progress));
            if let Err(error) = started {
                log::warn!("the made-up download stands still: {error}");
            }
        }
    }

    /// Whether it is time to look: at the start, and then once every `every` while the app stays open.
    fn due(&self, wanted: bool, every: Duration, now: Instant) -> bool {
        self.allowed && wanted && !self.looking && self.looked.is_none_or(|last| now.duration_since(last) >= every)
    }

    /// How long until the next look is due, counted from `now`; `None` when none is coming.
    fn until_due(&self, wanted: bool, every: Duration, now: Instant) -> Option<Duration> {
        let last = self.looked.filter(|_| self.allowed && wanted && !self.looking)?;
        Some(every.saturating_sub(now.duration_since(last)))
    }

    /// Takes in what a look found. `skipped` is the version the user never wants offered. Gives a sentence for
    /// the user when they asked for the look themselves; a look the app started on its own stays quiet.
    fn take(&mut self, found: Found, skipped: &str) -> Option<String> {
        self.looking = false;
        let asked = std::mem::take(&mut self.asked);
        match found {
            Ok(Some(release)) => {
                // Human: a download under way or done is not thrown away by a later look. A required release
                // is offered whatever was skipped. A card the user folded away stays folded for the same
                // release, and opens again for another one or one that has become required.
                if matches!(self.stage, Stage::Offered) && (asked || release.required || release.version != skipped) {
                    let same = self.offered.as_ref().is_some_and(|shown| shown.version == release.version && shown.required == release.required);
                    if !same {
                        self.collapsed = false;
                        self.failed = false;
                    }
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
            let mut found = latest(VERSION, &|token| get(&address, token), token);
            // Human: the newest release may be an ordinary one while a required one lies between it and this
            // copy. The list of recent releases says so; when it can't be had, the update stays optional.
            if let Ok(Some(release)) = &mut found
                && !release.required
            {
                let list = format!("{API}/repos/{REPOSITORY}/releases?per_page={RECENT}");
                let listed = |sign_in: Option<&str>| get(&list, sign_in).ok().filter(|(status, _)| *status == 200);
                let answer = listed(None).or_else(|| token().and_then(|token| listed(Some(&token))));
                release.required = answer.is_some_and(|(_, body)| any_required(VERSION, &body));
            }
            // Human: the app may have quit meanwhile; then nobody waits for the answer.
            let _ = send.send(found);
            ctx.request_repaint();
        };
        if let Err(error) = std::thread::Builder::new().name("update-check".to_owned()).spawn(work) {
            log::info!("update check: no thread to look on: {error}");
            self.looking = false;
        }
    }

    /// What the settings window says about a newer version.
    #[must_use]
    pub fn seen(&self) -> Seen {
        Seen { looking: self.looking, ago: self.looked.map(|at| at.elapsed()), offered: self.offered.as_ref().map(|release| release.version.clone()) }
    }

    /// Looks now because the user asked to, whatever the setting says; the answer is always said.
    pub fn look_now(&mut self, ctx: &egui::Context) {
        if !self.looking {
            self.look(ctx);
        }
        self.asked = true;
    }

    /// Once a frame: takes in a finished look and starts one when it is time. `wanted` and `every` are the
    /// settings (whether to look, and how often), `skipped` the version the user never wants offered. Gives a
    /// sentence to show the user, if there is one.
    // Human: a window that is left alone draws no frames, and would never look again. So the next look's time
    // is given to the window as the latest moment to draw the next frame.
    pub fn tick(&mut self, ctx: &egui::Context, wanted: bool, every: Duration, skipped: &str) -> Option<String> {
        let mut say = None;
        while let Ok(found) = self.answers.try_recv() {
            say = self.take(found, skipped).or(say);
        }
        let now = Instant::now();
        if self.due(wanted, every, now) {
            self.look(ctx);
        } else if let Some(wait) = self.until_due(wanted, every, now) {
            ctx.request_repaint_after(wait.max(Duration::from_secs(1)));
        }
        self.fetched().or(say)
    }

    /// The installer and the checksums of the offered release, when this copy can install them itself.
    fn installable(&self) -> Option<(ReleaseFile, ReleaseFile, Place)> {
        let release = self.offered.as_ref()?;
        Some((release.installer.clone()?, release.checksums.clone()?, self.place.clone()?))
    }

    /// Starts downloading the offered release's installer on a thread of its own.
    // Agent: SPAWNS one thread that lives as long as the download; WAKES the window through `ctx` when it ends.
    fn fetch(&mut self, ctx: &egui::Context) {
        let Some((installer, checksums, place)) = self.installable() else { return };
        let progress = Arc::new(Progress::of(installer.size));
        let (send, result) = channel();
        let (shared, ctx) = (Arc::clone(&progress), ctx.clone());
        let work = move || {
            // Human: a download the user stopped has nobody waiting for its answer.
            let _ = send.send(install::fetch(&installer, &checksums, &place, &shared));
            ctx.request_repaint();
        };
        match std::thread::Builder::new().name("update-download".to_owned()).spawn(work) {
            Ok(_) => self.stage = Stage::Fetching { progress, result },
            Err(error) => log::info!("update: no thread to download on: {error}"),
        }
    }

    /// Takes in a finished download. Gives a sentence for the user when it failed.
    fn fetched(&mut self) -> Option<String> {
        let Stage::Fetching { result, .. } = &self.stage else { return None };
        let ended = match result.try_recv() {
            Ok(ended) => ended,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("The download ended without an answer.".to_owned()),
        };
        match ended {
            Ok(prepared) => {
                self.stage = Stage::Arrived(prepared);
                None
            }
            Err(error) => {
                log::info!("update download: {error}");
                self.stage = Stage::Offered;
                self.failed = true;
                Some(format!("The update couldn't be downloaded. {error}"))
            }
        }
    }

    /// What follows a click in the card.
    fn clicked(&mut self, click: Click, ctx: &egui::Context) -> Option<Answer> {
        match (click, &self.stage) {
            (Click::Take, Stage::Arrived(prepared) | Stage::Ready(prepared)) => Some(Answer::Install(prepared.clone())),
            (Click::Take, Stage::Fetching { .. }) | (Click::Cancel, Stage::Offered | Stage::Arrived(_) | Stage::Ready(_)) => None,
            (Click::Take, Stage::Offered) if self.installable().is_some() => {
                self.fetch(ctx);
                None
            }
            // Human: a copy that can't replace itself sends the user to the browser. An ordinary offer is done
            // with that; a required one stays, since the old version is still the one running.
            (Click::Take, Stage::Offered) if self.offered.as_ref().is_some_and(|release| release.required) => {
                self.offered.as_ref().map(|release| Answer::Get(release.download.clone()))
            }
            (Click::Take, Stage::Offered) => self.offered.take().map(|release| Answer::Get(release.download)),
            (Click::Skip, _) => {
                self.stage = Stage::Offered;
                self.collapsed = false;
                self.offered.take().map(|release| Answer::Skip(release.version))
            }
            // Human: "Later" takes the card away until the next look, which asks again.
            (Click::Later, _) => {
                self.offered = None;
                self.stage = Stage::Offered;
                self.collapsed = false;
                self.failed = false;
                None
            }
            (Click::Collapse, _) => {
                self.collapsed = true;
                None
            }
            (Click::Expand, _) => {
                self.collapsed = false;
                None
            }
            (Click::Quit, _) => Some(Answer::Quit),
            (Click::Cancel, Stage::Fetching { progress, .. }) => {
                progress.cancel();
                self.stage = Stage::Offered;
                None
            }
        }
    }

    /// Draws the card while a newer version is on offer, and gives what the user chose in it.
    pub fn show(&mut self, ctx: &egui::Context, palette: Palette, animate: bool) -> Option<Answer> {
        if let Stage::Arrived(prepared) = &self.stage {
            let prepared = prepared.clone();
            self.stage = Stage::Ready(prepared.clone());
            return Some(Answer::Install(prepared));
        }
        let release = self.offered.as_ref()?;
        let frame = egui::Frame::new()
            .fill(palette.island)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(ISLAND_RADIUS)
            .inner_margin(ISLAND_PADDING);
        let mut click = None;
        if release.required {
            // Human: a required update is asked in the middle of the window, over everything, and the rest
            // waits for the answer. Escape and a click beside it change nothing.
            let id = egui::Id::unique("update-required");
            egui::Modal::new(id)
                .area(egui::Modal::default_area(id).fade_in(false))
                .frame(frame)
                .backdrop_color(egui::Color32::from_black_alpha(DIALOG_BACKDROP))
                .show(ctx, |ui| click = self.card(ui, release, palette, animate));
        } else if self.collapsed {
            egui::Area::new(egui::Id::unique("update-pill"))
                .anchor(Align2::RIGHT_BOTTOM, vec2(-UPDATE_MARGIN, -UPDATE_MARGIN))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| click = self.pill(ui, release, palette, animate));
        } else {
            egui::Area::new(egui::Id::unique("update"))
                .anchor(Align2::RIGHT_BOTTOM, vec2(-UPDATE_MARGIN, -UPDATE_MARGIN))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    frame.show(ui, |ui| click = self.card(ui, release, palette, animate));
                });
        }
        self.clicked(click?, ctx)
    }

    /// Draws the card: its head, what is new, and its last part. Gives what was clicked.
    fn card(&self, ui: &mut Ui, release: &Release, palette: Palette, animate: bool) -> Option<Click> {
        // Human: in a narrow window the card takes what there is, and the bar with it.
        let room = ui.ctx().content_rect().width() - 2.0 * (UPDATE_MARGIN + ISLAND_PADDING);
        ui.set_width(UPDATE_WIDTH.min(room).max(0.0));
        let mut click = head(ui, release, palette).then_some(Click::Collapse);
        if !release.notes.is_empty() {
            rule(ui, palette);
            egui::ScrollArea::vertical().max_height(UPDATE_NOTES_HEIGHT).auto_shrink([false, true]).show(ui, |ui| {
                notes(ui, &release.notes, palette);
            });
        }
        rule(ui, palette);
        click = self.foot(ui, palette, animate).or(click);
        click
    }

    /// What the folded card says in a few words: the offer, how far the download is, or that it is ready.
    fn in_short(&self, release: &Release) -> String {
        match &self.stage {
            Stage::Offered => format!("Pencraft {} is available", release.version),
            Stage::Fetching { progress, .. } => {
                let (done, total) = progress.sizes();
                let arrived = progress.fraction();
                if total == 0 || arrived >= 1.0 { said(arrived, arrived, done, total) } else { format!("Downloading… {:.0} %", arrived * 100.0) }
            }
            Stage::Arrived(_) | Stage::Ready(_) => "Restart to update".to_owned(),
        }
    }

    /// Draws the folded card: a pill with an icon and a few words, and a thin bar under them while a download
    /// runs. A click opens the card again.
    // Human: the card covers a corner of the canvas. Folded away it still says that there is an update, and
    // how far its download is, without being in the way.
    fn pill(&self, ui: &mut Ui, release: &Release, palette: Palette, animate: bool) -> Option<Click> {
        let text = self.in_short(release);
        let words = ui.painter().layout_no_wrap(text.clone(), FontId::proportional(SIDEBAR_TEXT_SIZE), palette.foreground);
        let size = vec2(UPDATE_PILL_PADDING * 2.0 + ICON_SIZE + UPDATE_NOTE_GAP + words.size().x, UPDATE_PILL);
        let (pill, response) = ui.allocate_exact_size(size, Sense::click());
        let on = response.hovered() || response.has_focus();
        ui.painter().rect(
            pill,
            UPDATE_PILL / 2.0,
            if on { palette.hover } else { palette.island },
            Stroke::new(1.0, palette.outline),
            egui::StrokeKind::Inside,
        );
        let icon = Rect::from_center_size(egui::pos2(pill.left() + UPDATE_PILL_PADDING + ICON_SIZE / 2.0, pill.center().y), Vec2::splat(ICON_SIZE));
        icons::paint(ui.painter(), icon, "download", palette.foreground);
        ui.painter().galley(egui::pos2(icon.right() + UPDATE_NOTE_GAP, pill.center().y - words.size().y / 2.0), words, palette.foreground);
        if let Stage::Fetching { progress, .. } = &self.stage {
            // The download's bar, as a hairline along the pill's lower edge.
            let (_, total) = progress.sizes();
            let line = Rect::from_min_max(
                egui::pos2(pill.left() + UPDATE_PILL / 2.0, pill.bottom() - 3.0),
                egui::pos2(pill.right() - UPDATE_PILL / 2.0, pill.bottom() - 1.0),
            );
            ui.painter().rect_filled(line, 1.0, palette.input);
            let part = if total > 0 { progress.fraction() } else { 0.0 };
            ui.painter().rect_filled(Rect::from_min_size(line.min, vec2(line.width() * part, line.height())), 1.0, palette.primary);
            ui.ctx().request_repaint_after(if animate { GLANCE } else { GLANCE * 5 });
        }
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &text));
        response.on_hover_text("Show the update").clicked().then_some(Click::Expand)
    }

    /// Draws the card's last part, which depends on how far taking the update is: the choices, the download's
    /// progress, or the restart. Gives what was clicked.
    fn foot(&self, ui: &mut Ui, palette: Palette, animate: bool) -> Option<Click> {
        let mut click = None;
        let required = self.offered.as_ref().is_some_and(|release| release.required);
        match &self.stage {
            Stage::Offered => {
                let take = if self.installable().is_some() { "Update and restart" } else { "Download" };
                if self.failed {
                    let why =
                        if required { "The download didn't work. Try again, or go on for now." } else { "The download didn't work. Try again." };
                    ui.label(RichText::new(why).size(SIDEBAR_TEXT_SIZE).color(palette.muted));
                    ui.add_space(UPDATE_NOTE_GAP);
                }
                ui.horizontal(|ui| {
                    click = primary(ui, take, palette).then_some(Click::Take);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if required {
                            // Human: a required update is taken or the app is left. Only when its download
                            // failed may the user go on, so a network that is down locks nobody out.
                            click = quiet_button(ui, "Quit Pencraft", palette).then_some(Click::Quit).or(click);
                            if self.failed {
                                click = quiet_button(ui, "Not now", palette).then_some(Click::Later).or(click);
                            }
                        } else {
                            click = quiet_button(ui, "Skip this version", palette).then_some(Click::Skip).or(click);
                            click = quiet_button(ui, "Later", palette).then_some(Click::Later).or(click);
                        }
                    });
                });
            }
            Stage::Fetching { progress, .. } => {
                let (done, total) = progress.sizes();
                let arrived = progress.fraction();
                // Human: the download arrives in pieces; the bar and the number glide to where it is, so
                // they move evenly instead of in jumps. With animations off they show it as it is.
                let part = if animate && total > 0 {
                    ui.ctx().animate_value_with_time(egui::Id::unique("update-download"), arrived, UPDATE_BAR_GLIDE)
                } else {
                    arrived
                };
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(said(part, arrived, done, total)).size(SIDEBAR_TEXT_SIZE).color(palette.muted));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        click = quiet_button(ui, "Cancel", palette).then_some(Click::Cancel);
                    });
                });
                // After the last piece the file is still checked, and on a Mac unpacked: no length to show.
                bar(ui, (total > 0 && arrived < 1.0).then_some(part), palette, animate);
                if animate {
                    ui.ctx().request_repaint();
                } else {
                    ui.ctx().request_repaint_after(GLANCE);
                }
            }
            Stage::Arrived(_) | Stage::Ready(_) => {
                ui.label(RichText::new("Downloaded and checked. Pencraft restarts to install it.").size(SIDEBAR_TEXT_SIZE).color(palette.muted));
                ui.add_space(UPDATE_NOTE_GAP);
                ui.horizontal(|ui| {
                    click = primary(ui, "Restart to update", palette).then_some(Click::Take);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (other, does) = if required { ("Quit Pencraft", Click::Quit) } else { ("Later", Click::Later) };
                        click = quiet_button(ui, other, palette).then_some(does).or(click);
                    });
                });
            }
        }
        click
    }
}

/// The card's head: an icon, the new version and the one that is running, and for an update that isn't
/// required the button that folds the card away. Gives whether that button was clicked.
fn head(ui: &mut Ui, release: &Release, palette: Palette) -> bool {
    let mut fold = false;
    ui.horizontal(|ui| {
        let (badge, _) = ui.allocate_exact_size(Vec2::splat(UPDATE_BADGE), Sense::hover());
        ui.painter().rect_filled(badge, CHIP_RADIUS, palette.hover);
        icons::paint(ui.painter(), Rect::from_center_size(badge.center(), Vec2::splat(ICON_SIZE)), "download", palette.foreground);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let (title, under) = if release.required {
                (format!("Pencraft {} is required", release.version), format!("You have {VERSION}. This update has to be installed to go on."))
            } else {
                (format!("Pencraft {} is available", release.version), format!("You have {VERSION}"))
            };
            ui.label(RichText::new(title).size(UPDATE_TITLE_SIZE).color(palette.foreground).strong());
            ui.label(RichText::new(under).size(SIDEBAR_TEXT_SIZE).color(palette.muted));
        });
        if !release.required {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                fold = crate::sidebar::icon_button(ui, "chevron-down", Vec2::splat(UPDATE_BUTTON_HEIGHT - 4.0), false, palette)
                    .on_hover_text("Fold away")
                    .clicked();
            });
        }
    });
    fold
}

/// A thin line across the card, with room above and below.
fn rule(ui: &mut Ui, palette: Palette) {
    ui.add_space(UPDATE_GAP);
    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(line, 0.0, palette.border);
    ui.add_space(UPDATE_GAP);
}

/// The card's main button. Gives whether it was clicked.
fn primary(ui: &mut Ui, text: &str, palette: Palette) -> bool {
    ui.spacing_mut().button_padding.x = UPDATE_BUTTON_PADDING;
    let text = RichText::new(text).size(SIDEBAR_TEXT_SIZE).color(palette.primary_text);
    ui.add(egui::Button::new(text).fill(palette.primary).corner_radius(CONTROL_RADIUS).min_size(vec2(0.0, UPDATE_BUTTON_HEIGHT))).clicked()
}

/// One of the card's lesser buttons: only words until the pointer is on it. Gives whether it was clicked.
fn quiet_button(ui: &mut Ui, text: &str, palette: Palette) -> bool {
    let size = vec2(0.0, UPDATE_BUTTON_HEIGHT);
    let (rect, response) = {
        let galley = ui.painter().layout_no_wrap(text.to_owned(), FontId::proportional(SIDEBAR_TEXT_SIZE), palette.muted);
        ui.allocate_exact_size(vec2(galley.size().x + UPDATE_BUTTON_PADDING * 1.5, size.y), Sense::click())
    };
    let on = response.hovered() || response.has_focus();
    if on {
        ui.painter().rect_filled(rect, CONTROL_RADIUS, palette.hover);
    }
    let color = if on { palette.foreground } else { palette.muted };
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, FontId::proportional(SIDEBAR_TEXT_SIZE), color);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, text));
    response.clicked()
}

/// The size of the made-up download (`--update-download`), about that of a real installer.
const STAGED_SIZE: u64 = 31_000_000;
/// How often a piece of the made-up download arrives, and how long its file is "made ready" at the end.
const STAGED_PIECE: Duration = Duration::from_millis(220);
/// See [`STAGED_PIECE`].
const STAGED_READY: Duration = Duration::from_secs(3);

/// How large the made-up download's piece number `step` is: uneven, as a real connection brings them, and
/// now and then nothing at all.
fn staged_piece(step: u64) -> u64 {
    match step % 7 {
        3 => 0,
        5 => STAGED_SIZE / 25,
        odd if odd % 2 == 1 => STAGED_SIZE / 60,
        _ => STAGED_SIZE / 110,
    }
}

/// Lets the made-up download `progress` arrive piece by piece, over and over, until it is told to stop.
// Agent: BLOCKS its own thread between pieces; WRITES only `progress`.
fn staged_download(progress: &Progress) {
    let (mut done, _) = progress.sizes();
    let mut step = 0;
    while !progress.stopped() {
        std::thread::sleep(STAGED_PIECE);
        step += 1;
        done = (done + staged_piece(step)).min(STAGED_SIZE);
        progress.arrive(done);
        if done == STAGED_SIZE {
            // The file is "made ready" for a while, then the download starts over.
            let ready = Instant::now();
            while ready.elapsed() < STAGED_READY && !progress.stopped() {
                std::thread::sleep(STAGED_PIECE);
            }
            done = 0;
            progress.arrive(done);
        }
    }
}

/// What the card says over the bar: how far the download is in per cent and megabytes, or that the file is
/// being made ready. `part` is what the bar shows, `arrived` what has really come.
fn said(part: f32, arrived: f32, done: u64, total: u64) -> String {
    // Shown with one decimal: megabytes as tenths, worked out in whole numbers.
    let megabytes = |bytes: u64| {
        let tenths = bytes / 100_000;
        format!("{}.{}", tenths / 10, tenths % 10)
    };
    if total == 0 {
        return if done == 0 { "Downloading…".to_owned() } else { format!("Downloading… {} MB", megabytes(done)) };
    }
    // Human: after the last piece the file is still checked, and on a Mac unpacked.
    if arrived >= 1.0 {
        return "Getting ready…".to_owned();
    }
    format!("Downloading… {:.0} %  ·  {} of {} MB", part * 100.0, megabytes(done), megabytes(total))
}

/// Where something that goes back and forth once every two `period`s is at `time`: from 0 to 1 and back,
/// slow at the ends.
fn back_and_forth(time: f64, period: f32) -> f32 {
    let turn = (time / f64::from(period)).rem_euclid(2.0);
    #[expect(clippy::cast_possible_truncation, reason = "a number between 0 and 1")]
    let along = if turn > 1.0 { 2.0 - turn } else { turn } as f32;
    along * along * (3.0 - 2.0 * along)
}

/// The bar that shows how far a download is: `part` from 0 to 1, or `None` while there is no length to show
/// (the size isn't known, or the file is being checked).
// Human: a filled part that a light runs over says "at work" also while no new piece arrives; without a
// length a piece of the bar travels back and forth. With animations off the bar stands still: filled as far
// as the download is, or dimmed all over while there is no length.
fn bar(ui: &mut Ui, part: Option<f32>, palette: Palette, animate: bool) {
    ui.add_space(UPDATE_NOTE_GAP);
    let (track, _) = ui.allocate_exact_size(vec2(ui.available_width(), UPDATE_BAR), Sense::hover());
    let round = UPDATE_BAR / 2.0;
    ui.painter().rect_filled(track, round, palette.input);
    let now = ui.input(|input| input.time);
    let Some(part) = part else {
        if animate {
            let piece = track.width() * UPDATE_BAR_PIECE;
            let left = track.left() + (track.width() - piece) * back_and_forth(now, UPDATE_BAR_SWEEP);
            ui.painter().rect_filled(Rect::from_min_size(egui::pos2(left, track.top()), vec2(piece, UPDATE_BAR)), round, palette.primary);
        } else {
            ui.painter().rect_filled(track, round, palette.primary.gamma_multiply(0.45));
        }
        return;
    };
    let done = Rect::from_min_size(track.min, vec2((track.width() * part.clamp(0.0, 1.0)).max(UPDATE_BAR), UPDATE_BAR));
    if part <= 0.0 {
        return;
    }
    ui.painter().rect_filled(done, round, palette.primary);
    if animate {
        // The light: a soft band that runs from the bar's start to where the download is, again and again.
        let band = (done.width() * 0.35).clamp(UPDATE_BAR * 2.0, 60.0);
        #[expect(clippy::cast_possible_truncation, reason = "a number between 0 and 1")]
        let along = (now / f64::from(UPDATE_BAR_SWEEP)).rem_euclid(1.0) as f32;
        let left = done.left() - band + (done.width() + band) * along;
        let light = Rect::from_min_size(egui::pos2(left, done.top()), vec2(band, UPDATE_BAR));
        ui.painter().with_clip_rect(done.shrink2(vec2(round * 0.5, 0.0))).rect_filled(light, round, palette.primary_text.gamma_multiply(0.28));
    }
}

/// One piece of release notes as the card shows it. The text keeps Markdown's `**` around what is bold.
#[derive(Debug, PartialEq, Eq)]
enum Note {
    /// A heading, without its `#`.
    Heading(String),
    /// A paragraph.
    Text(String),
    /// An item of a list, without its dash.
    Item(String),
}

/// Release notes as headings, paragraphs and list items. A changelog breaks its lines to fit an editor; here
/// the lines of a paragraph or an item are joined again, so the card can break them to fit itself. Code loses
/// its backticks.
fn note_blocks(notes: &str) -> Vec<Note> {
    let mut blocks: Vec<Note> = Vec::new();
    // Whether the next plain line carries on the last block.
    let mut open = false;
    for line in notes.lines().map(|line| line.trim().replace('`', "")) {
        if line.is_empty() {
            open = false;
        } else if line.starts_with('#') {
            blocks.push(Note::Heading(line.trim_start_matches('#').trim().to_owned()));
            open = false;
        } else if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            blocks.push(Note::Item(item.trim().to_owned()));
            open = true;
        } else if let (true, Some(Note::Text(text) | Note::Item(text))) = (open, blocks.last_mut()) {
            text.push(' ');
            text.push_str(&line);
        } else {
            blocks.push(Note::Text(line));
            open = true;
        }
    }
    blocks
}

/// A line of notes cut where `**` switches bold on and off: each piece with whether it is bold.
fn spans(text: &str) -> Vec<(bool, &str)> {
    text.split("**").enumerate().map(|(index, piece)| (index % 2 == 1, piece)).filter(|(_, piece)| !piece.is_empty()).collect()
}

/// A paragraph or item of the notes as words to draw: quiet, with what was bold in the main text colour.
fn words(text: &str, palette: Palette) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (bold, piece) in spans(text) {
        let color = if bold { palette.foreground } else { palette.muted };
        job.append(piece, 0.0, TextFormat { font_id: FontId::proportional(SIDEBAR_TEXT_SIZE), color, ..TextFormat::default() });
    }
    job
}

/// Draws release notes.
fn notes(ui: &mut Ui, text: &str, palette: Palette) {
    ui.spacing_mut().item_spacing.y = UPDATE_NOTE_GAP;
    for (index, block) in note_blocks(text).into_iter().enumerate() {
        match block {
            Note::Heading(text) => {
                if index > 0 {
                    ui.add_space(UPDATE_NOTE_GAP);
                }
                ui.add(egui::Label::new(RichText::new(text).size(SIDEBAR_TEXT_SIZE).color(palette.foreground).strong()).wrap());
            }
            Note::Text(text) => {
                ui.add(egui::Label::new(words(&text, palette)).wrap());
            }
            Note::Item(text) => {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let line = ui.fonts_mut(|fonts| fonts.row_height(&FontId::proportional(SIDEBAR_TEXT_SIZE)));
                    let (dot, _) = ui.allocate_exact_size(vec2(UPDATE_BULLET, line), Sense::hover());
                    ui.painter().circle_filled(dot.left_center() + vec2(3.0, 0.0), 1.75, palette.muted);
                    ui.add(egui::Label::new(words(&text, palette)).wrap());
                });
            }
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
        Release {
            version: version.to_owned(),
            notes: "- New things".to_owned(),
            download: releases_page(),
            installer: None,
            checksums: None,
            required: false,
        }
    }

    /// An offered release this copy could install itself, with a download under way that `send` ends.
    fn fetching() -> (Updates, Sender<Result<Prepared, String>>, Arc<Progress>) {
        let mut updates = Updates::new(true, false);
        let file = |name: &str| Some(ReleaseFile { id: 1, name: name.to_owned(), size: 3 });
        updates.offered = Some(Release { installer: file("pencraft-9.1.0-setup.exe"), checksums: file(CHECKSUMS), ..release("9.1.0") });
        updates.place = Some(Place::Setup);
        let (send, result) = channel();
        let progress = Arc::new(Progress::of(3));
        updates.stage = Stage::Fetching { progress: Arc::clone(&progress), result };
        (updates, send, progress)
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
        assert_eq!(
            bare,
            Release {
                version: "9.2.0".to_owned(),
                notes: String::new(),
                download: releases_page(),
                installer: None,
                checksums: None,
                required: false
            }
        );
        assert!(parse_release("not json").is_err());
    }

    #[test]
    fn a_release_names_the_files_an_update_in_place_needs() {
        let numbered = NEWER.replace(r#"{"name": "#, r#"{"id": 7, "size": 9, "name": "#);
        let release = parse_release(&numbered).unwrap();
        assert_eq!(release.checksums, Some(ReleaseFile { id: 7, name: CHECKSUMS.to_owned(), size: 9 }));
        if cfg!(all(windows, target_arch = "x86_64")) {
            assert_eq!(release.installer.unwrap().name, "pencraft-9.1.0-setup.exe");
        } else if cfg!(target_os = "macos") {
            assert_eq!(release.installer.unwrap().name, "pencraft-9.1.0-macos-universal.dmg");
        } else {
            assert_eq!(release.installer, None);
        }
        // Human: files GitHub gave no number can't be asked for.
        let release = parse_release(NEWER).unwrap();
        assert_eq!((release.installer, release.checksums), (None, None));
    }

    #[test]
    fn a_copy_that_cant_replace_itself_sends_the_user_to_the_browser() {
        let ctx = egui::Context::default();
        let (mut updates, _send, _progress) = fetching();
        updates.stage = Stage::Offered;
        updates.place = None;
        assert_eq!(updates.clicked(Click::Take, &ctx), Some(Answer::Get(releases_page())));
        assert_eq!(updates.offered, None);

        // Human: so does a release without an installer or without checksums.
        let mut updates = Updates::new(true, false);
        updates.place = Some(Place::Setup);
        updates.offered = Some(release("9.1.0"));
        assert_eq!(updates.clicked(Click::Take, &ctx), Some(Answer::Get(releases_page())));
    }

    #[test]
    fn a_finished_download_restarts_into_the_update_once() {
        let ctx = egui::Context::default();
        let (mut updates, send, _progress) = fetching();
        assert_eq!(updates.fetched(), None);
        let prepared = Prepared::Setup(std::path::PathBuf::from("pencraft-setup.exe"));
        send.send(Ok(prepared.clone())).unwrap();
        assert_eq!(updates.fetched(), None);
        assert_eq!(updates.show(&ctx, crate::theme::palette(true), false), Some(Answer::Install(prepared.clone())));
        // Human: when the user then keeps the app open, the card waits for a click and downloads nothing again.
        assert!(matches!(updates.stage, Stage::Ready(_)));
        assert!(updates.offered.is_some());
        assert_eq!(updates.clicked(Click::Take, &ctx), Some(Answer::Install(prepared)));
        assert_eq!(updates.clicked(Click::Later, &ctx), None);
        assert_eq!(updates.offered, None);
        assert!(matches!(updates.stage, Stage::Offered));
    }

    #[test]
    fn a_failed_or_stopped_download_goes_back_to_the_offer() {
        let ctx = egui::Context::default();
        let (mut updates, send, _progress) = fetching();
        send.send(Err("GitHub answered 500".to_owned())).unwrap();
        assert_eq!(updates.fetched().unwrap(), "The update couldn't be downloaded. GitHub answered 500");
        assert!(matches!(updates.stage, Stage::Offered));
        assert!(updates.offered.is_some());

        // Human: a stopped download's thread is told so, and what it says afterwards reaches nobody.
        let (mut updates, send, _progress) = fetching();
        assert_eq!(updates.clicked(Click::Cancel, &ctx), None);
        assert!(matches!(updates.stage, Stage::Offered));
        assert!(send.send(Err("The download was stopped.".to_owned())).is_err());
        assert_eq!(updates.fetched(), None);

        // Human: a thread that died without a word is a failure too, not a card stuck on "Downloading".
        let (mut updates, send, _progress) = fetching();
        drop(send);
        assert!(updates.fetched().is_some());
    }

    #[test]
    fn the_daily_look_leaves_a_download_alone() {
        let (mut updates, _send, _progress) = fetching();
        assert_eq!(updates.take(Ok(Some(release("9.2.0"))), ""), None);
        assert_eq!(updates.offered.as_ref().map(|release| release.version.as_str()), Some("9.1.0"));
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
    fn the_app_looks_at_the_start_and_then_every_so_often_and_never_in_a_quiet_run() {
        let start = Instant::now();
        let day = Duration::from_hours(24);
        let mut updates = Updates::new(true, false);
        assert!(updates.due(true, day, start));
        // Before the first look there is nothing to wait for: it is due.
        assert_eq!(updates.until_due(true, day, start), None);
        // The setting switches it off.
        assert!(!updates.due(false, day, start));
        updates.looked = Some(start);
        updates.looking = true;
        assert!(!updates.due(true, day, start + day));
        updates.looking = false;
        assert!(!updates.due(true, day, start + Duration::from_hours(23)));
        assert!(updates.due(true, day, start + day));
        // How often is the user's choice: an hour is due where a day is not, and an unknown choice is the default.
        assert!(updates.due(true, every("Every hour"), start + Duration::from_hours(2)));
        // A choice that is no longer offered ("Every 6 hours", the default before) counts as the default.
        assert_eq!((every("Every day"), every("Every 15 minutes")), (day, Duration::from_mins(15)));
        assert_eq!((every("Every 6 hours"), every("whenever")), (Duration::from_mins(5), Duration::from_mins(5)));
        // A window left alone is woken when the next look is due, not before and not never.
        assert_eq!(updates.until_due(true, day, start + Duration::from_hours(23)), Some(Duration::from_hours(1)));
        assert_eq!(updates.until_due(true, day, start + Duration::from_hours(30)), Some(Duration::ZERO));
        assert_eq!(updates.until_due(false, day, start), None);
        // A picture, a private or a background run never looks.
        let quiet = Updates::new(false, false);
        assert!(!quiet.due(true, day, start) && quiet.until_due(true, day, start).is_none());
    }

    #[test]
    fn a_required_release_is_found_in_its_notes_or_among_those_skipped_over() {
        let json = |tag: &str, body: &str| {
            format!(r#"{{"tag_name":"{tag}","html_url":"","body":{},"assets":[]}}"#, serde_json::Value::String(body.to_owned()))
        };
        // The marker on a line of its own says so, and is not part of what the user reads.
        let marked = parse_release(&json("v9.2.0", "Safer saves.\n\n<!-- required -->\n\n- A fix")).unwrap();
        assert!(marked.required);
        assert_eq!(marked.notes, "Safer saves.\n\n\n- A fix");
        let plain = parse_release(&json("v9.2.0", "We talk about <!-- required --> here, in a sentence.")).unwrap();
        assert!(!plain.required && plain.notes.contains("required"));
        // A newer release that is required counts though the newest is not; older ones, drafts and
        // pre-releases don't.
        let list = |entries: &[(&str, &str, &str)]| {
            let all: Vec<String> = entries.iter().map(|(tag, body, more)| format!(r#"{{"tag_name":"{tag}","body":"{body}"{more}}}"#)).collect();
            format!("[{}]", all.join(","))
        };
        assert!(any_required("9.0.0", &list(&[("v9.2.0", "- New", ""), ("v9.1.0", "<!-- required -->", ""), ("v9.0.0", "- Old", "")])));
        assert!(!any_required("9.1.0", &list(&[("v9.2.0", "- New", ""), ("v9.1.0", "<!-- required -->", "")])));
        assert!(!any_required(
            "9.0.0",
            &list(&[("v9.1.0", "<!-- required -->", r#","draft":true"#), ("v9.2.0-rc.1", "<!-- required -->", r#","prerelease":true"#)])
        ));
        assert!(!any_required("9.0.0", "not a list") && !any_required("9.0.0", "[]"));
    }

    #[test]
    fn a_required_update_cant_be_skipped_or_put_off_unless_its_download_failed() {
        let ctx = egui::Context::default();
        let mut updates = Updates::new(true, false);
        let required = Release { required: true, ..release("9.1.0") };
        // It is offered though that version was skipped.
        assert_eq!(updates.take(Ok(Some(required.clone())), "9.1.0"), None);
        assert!(updates.offered.as_ref().is_some_and(|release| release.required));
        // A copy that can't install it itself is sent to the browser, and the card stays.
        updates.place = None;
        assert_eq!(updates.clicked(Click::Take, &ctx), Some(Answer::Get(required.download.clone())));
        assert!(updates.offered.is_some());
        // Not taking it means leaving the app.
        assert_eq!(updates.clicked(Click::Quit, &ctx), Some(Answer::Quit));
        // A download that failed lets the user go on for now; the next look asks again.
        let (send, result) = channel();
        updates.stage = Stage::Fetching { progress: Arc::new(Progress::of(3)), result };
        send.send(Err("offline".to_owned())).unwrap();
        assert!(updates.fetched().is_some_and(|said| said.contains("offline")) && updates.failed);
        assert_eq!(updates.clicked(Click::Later, &ctx), None);
        assert!(updates.offered.is_none() && !updates.failed);
        assert_eq!(updates.take(Ok(Some(required)), ""), None);
        assert!(updates.offered.is_some());
    }

    #[test]
    fn the_card_folds_away_and_opens_again_for_something_new() {
        let ctx = egui::Context::default();
        let mut updates = Updates::new(true, false);
        assert_eq!(updates.take(Ok(Some(release("9.1.0"))), ""), None);
        assert_eq!(updates.clicked(Click::Collapse, &ctx), None);
        assert!(updates.collapsed && updates.offered.is_some());
        // Folded, it says in a few words what there is.
        let shown = updates.offered.clone().unwrap();
        assert_eq!(updates.in_short(&shown), "Pencraft 9.1.0 is available");
        // The next look finds the same release: the card stays folded.
        assert_eq!(updates.take(Ok(Some(release("9.1.0"))), ""), None);
        assert!(updates.collapsed);
        // Another release, or the same one become required, opens it again.
        assert_eq!(updates.take(Ok(Some(release("9.2.0"))), ""), None);
        assert!(!updates.collapsed);
        updates.collapsed = true;
        assert_eq!(updates.take(Ok(Some(Release { required: true, ..release("9.2.0") })), ""), None);
        assert!(!updates.collapsed);
        // A click on the pill opens the card; "Later" takes it away until the next look, unfolded.
        updates.collapsed = true;
        assert_eq!(updates.clicked(Click::Expand, &ctx), None);
        assert!(!updates.collapsed);
        updates.collapsed = true;
        assert_eq!(updates.clicked(Click::Later, &ctx), None);
        assert!(updates.offered.is_none() && !updates.collapsed);
        // Posed for a picture: folded, and required.
        let mut posed = Updates::new(false, false);
        posed.pose_as(Some((40, false)), true, true);
        assert!(posed.collapsed && posed.offered.as_ref().is_some_and(|release| release.required));
        let shown = posed.offered.clone().unwrap();
        assert_eq!(posed.in_short(&shown), "Downloading… 40 %");
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
    fn release_notes_are_put_together_again_from_a_changelogs_short_lines() {
        let notes = "A sharper canvas,\nand more.\n\n\n### Added\n\n- **Export to PDF.** Every top-level\n  layer becomes a page. Uses `--export`.\n- Find layers.\n\nThanks.\n";
        assert_eq!(
            note_blocks(notes),
            vec![
                Note::Text("A sharper canvas, and more.".to_owned()),
                Note::Heading("Added".to_owned()),
                Note::Item("**Export to PDF.** Every top-level layer becomes a page. Uses --export.".to_owned()),
                Note::Item("Find layers.".to_owned()),
                Note::Text("Thanks.".to_owned()),
            ]
        );
        // Human: a line right under a heading starts a paragraph of its own.
        assert_eq!(note_blocks("## Fixed\nShadows."), vec![Note::Heading("Fixed".to_owned()), Note::Text("Shadows.".to_owned())]);
        assert_eq!(note_blocks("\n\n"), Vec::new());
    }

    #[test]
    fn what_the_notes_make_bold_stands_out() {
        assert_eq!(spans("**Export to PDF.** Every layer."), vec![(true, "Export to PDF."), (false, " Every layer.")]);
        assert_eq!(spans("Plain"), vec![(false, "Plain")]);
        assert_eq!(spans("A **b** c **d**"), vec![(false, "A "), (true, "b"), (false, " c "), (true, "d")]);
        assert_eq!(spans(""), Vec::new());
    }

    #[test]
    fn the_card_draws_in_every_stage() {
        let ctx = egui::Context::default();
        let draw = |updates: &mut Updates| {
            let mut answer = None;
            let mut drawn = ctx.run_ui(egui::RawInput::default(), |ui| answer = updates.show(ui.ctx(), crate::theme::palette(true), true));
            // The font picture this frame made is for a screen; there is none here.
            drawn.textures_delta.clear();
            answer
        };
        let (mut updates, send, _progress) = fetching();
        updates.offered.as_mut().unwrap().notes = "Intro.\n\n### Added\n\n- **One.** Two\n  three.".to_owned();
        assert_eq!(draw(&mut updates), None);
        updates.stage = Stage::Offered;
        assert_eq!(draw(&mut updates), None);
        updates.stage = Stage::Ready(Prepared::Setup(std::path::PathBuf::from("pencraft-setup.exe")));
        assert_eq!(draw(&mut updates), None);
        drop(send);
    }

    #[test]
    fn the_download_says_how_far_it_is_and_the_bar_moves_evenly() {
        // Per cent as the bar shows them, megabytes as they have arrived.
        assert_eq!(said(0.42, 0.5, 15_500_000, 31_000_000), "Downloading… 42 %  ·  15.5 of 31.0 MB");
        assert_eq!(said(0.99, 1.0, 31_000_000, 31_000_000), "Getting ready…");
        // A download whose size isn't known says what has come, and nothing before the first piece.
        assert_eq!((said(0.0, 0.0, 0, 0), said(0.0, 0.0, 2_340_000, 0)), ("Downloading…".to_owned(), "Downloading… 2.3 MB".to_owned()));
        // The travelling piece goes there and back, slow where it turns, and never leaves the bar.
        let at = |time: f64| back_and_forth(time, 2.0);
        assert_eq!((at(0.0), at(2.0), at(4.0)), (0.0, 1.0, 0.0));
        assert!((at(1.0) - 0.5).abs() < 1e-6 && (at(3.0) - 0.5).abs() < 1e-6);
        assert!(at(0.2) < 0.1 && at(1.8) > 0.9);
        assert!((0..400).map(|step| at(f64::from(step) * 0.01)).all(|along| (0.0..=1.0).contains(&along)));
        let progress = Progress::of(10);
        assert_eq!(progress.sizes(), (0, 10));
    }

    #[test]
    fn a_made_up_download_stands_where_it_is_asked_to_and_can_be_cancelled() {
        let mut updates = Updates::new(false, false);
        updates.pose_as(Some((42, false)), false, false);
        let Stage::Fetching { progress, .. } = &updates.stage else { panic!("the card isn't downloading") };
        assert_eq!(progress.sizes(), (13_020_000, STAGED_SIZE));
        assert!((progress.fraction() - 0.42).abs() < 0.001 && updates.offered.is_some());
        // Nothing arrives and nothing ends by itself: the card goes on showing the download.
        assert_eq!(updates.fetched(), None);
        assert!(matches!(updates.stage, Stage::Fetching { .. }));
        // At 100 the file is being made ready; more than that is 100.
        updates.pose_as(Some((250, false)), false, false);
        let Stage::Fetching { progress, .. } = &updates.stage else { panic!("the card isn't downloading") };
        assert_eq!(progress.sizes(), (STAGED_SIZE, STAGED_SIZE));
        // The pieces come unevenly, some not at all, and a whole download's worth in a reasonable time.
        let pieces: Vec<u64> = (1..=7).map(staged_piece).collect();
        assert!(pieces.contains(&0) && pieces.iter().max() > pieces.iter().filter(|piece| **piece > 0).min());
        let rounds = STAGED_SIZE / (pieces.iter().sum::<u64>() / 7);
        assert!((40..160).contains(&rounds), "{rounds}");
        // A download that was stopped ends at its next piece.
        let progress = Progress::of(STAGED_SIZE);
        progress.cancel();
        staged_download(&progress);
        assert_eq!(progress.sizes().0, 0);
    }
}
