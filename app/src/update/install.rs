//! Puts a newer Quill in the place of the running one.
//!
//! The installer of a release is downloaded into the temporary folder and compared with the checksum the
//! release gives for it; once the app knows a key ([`KEYS`]), the checksums themselves must be signed with it.
//! On Windows the installer is the setup wizard, which is then run without its pages; on a Mac it is the disk
//! image, whose app is copied beside the running one and swapped in once the app has quit. Either way the new
//! version starts by itself. A copy that can't be replaced this way has no [`Place`], and the card sends the
//! user to the browser instead.

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use minisign_verify::{PublicKey, Signature};

use super::{API, PATIENCE, ReleaseFile, quiet};

/// The public halves of the keys a release's checksums are signed with, as `minisign -G` prints them (the
/// second line of the `.pub` file). The secret halves are kept away from GitHub: docs/RELEASING.md.
// Human: whoever can change a release on GitHub can change the installer and its checksums together, so the
// checksums prove nothing by themselves. A signature made with a key GitHub never sees does. Two keys, so
// one that is lost or stolen can be left out of the next version while the other still signs it.
// Agent: while this is EMPTY nothing is checked and updates work as they did before; with a key in it, a
// release without a valid `SHA256SUMS.txt.minisig` is not offered and never installed.
pub(super) const KEYS: &[&str] = &[];

/// The file the setup wizard leaves beside the program it installed.
const UNINSTALLER: &str = "unins000.exe";
/// The name of the app inside the disk image.
const BUNDLE: &str = "Pencraft.app";
/// What `codesign` asks of the app in an update: signed through Apple, by Quill's own Developer ID team
/// (`2G3936C5F4`, the team of "Developer ID Application: Niklas Vorberg"). The `=` makes it the requirement
/// itself rather than the name of a file that holds one.
// Human: `--verify --strict` alone lets any valid signature through, an ad hoc one too: a disk image built by
// anyone would pass. Only a copy signed by the team that signs Quill's releases is installed.
const SIGNED_BY_US: &str = r#"-R=anchor apple generic and certificate leaf[subject.OU] = "2G3936C5F4""#;
/// The largest installer the app is willing to download.
const MOST: u64 = 1024 * 1024 * 1024;
/// How the setup wizard is run: a small progress window instead of its pages, no questions, no restart of
/// Windows, and Quill started again at the end. `/RELAUNCH` is read by `installer/pencraft.iss`, and so is
/// the `/WAITPID=<process>` that follows these: the wizard waits for this Quill to be gone, because a
/// silent one that finds it still running gives up.
const SETUP_ARGS: [&str; 4] = ["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/RELAUNCH"];
/// Waits until the app (`$1`) has quit, puts the new bundle (`$3`) in the place of the old one (`$2`), and
/// opens it. When the new one can't be moved in, the old one is put back and opened.
const SWAP: &str = r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done
rm -rf "$2.old"
if mv "$2" "$2.old"; then
  if mv "$3" "$2"; then rm -rf "$2.old"; else mv "$2.old" "$2"; fi
fi
/usr/bin/open "$2""#;

/// Whether a release must come with signed checksums: it must as soon as the app knows a key.
#[must_use]
pub fn checks_signatures() -> bool {
    demanded(KEYS)
}

fn demanded(keys: &[&str]) -> bool {
    !keys.is_empty()
}

/// What the signature of a release's checksums has to say besides being ours: the version it was made for.
/// It is the signature's trusted comment (`minisign -S -t "Quill 1.2.3"`).
// Human: without it, the signed checksums of an old release could be handed out again with the old installer
// under a new version's number, and bring back a bug that was fixed since.
fn said(version: &str) -> String {
    format!("Quill {version}")
}

/// Makes sure `signature` (the text of a `.minisig` file) was made over `sums` with one of `keys`, for `version`.
fn signed(sums: &[u8], signature: &str, keys: &[&str], version: &str) -> Result<(), String> {
    let refused = || "The release's checksums are not signed by Quill's makers, so the update was not installed.".to_owned();
    let signature = Signature::decode(signature).map_err(|_| refused())?;
    // Human: a signature made the old way, over the file itself rather than its hash, is not taken.
    let ours = keys.iter().filter_map(|key| PublicKey::from_base64(key).ok()).any(|key| key.verify(sums, &signature, false).is_ok());
    if !ours {
        return Err(refused());
    }
    if signature.trusted_comment() == said(version) {
        Ok(())
    } else {
        Err("The release's signature was made for another version, so the update was not installed.".to_owned())
    }
}

/// How the running copy can be replaced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    /// It was installed by the setup wizard on Windows, which can install over it.
    Setup,
    /// It is this `Pencraft.app` on a Mac, in a folder that can be written to.
    Bundle(PathBuf),
}

/// How the running copy can be replaced, or `None` when it can't: the MSI, the plain program, a development
/// build, or a Mac app that is not in a bundle or sits where nothing can be written.
// Agent: READS the folder of the running program; on a Mac MAKES and removes an empty folder beside the bundle.
#[must_use]
pub fn place() -> Option<Place> {
    let program = std::env::current_exe().ok()?;
    if cfg!(all(windows, target_arch = "x86_64")) {
        program.with_file_name(UNINSTALLER).is_file().then_some(Place::Setup)
    } else if cfg!(target_os = "macos") {
        bundle_of(&program).filter(|bundle| bundle.parent().is_some_and(writable)).map(Place::Bundle)
    } else {
        None
    }
}

/// The bundle a Mac program runs from: `<somewhere>/X.app` for `<somewhere>/X.app/Contents/MacOS/x`.
fn bundle_of(program: &Path) -> Option<PathBuf> {
    let inner = program.parent()?;
    let contents = inner.parent()?;
    let bundle = contents.parent()?;
    let named = |path: &Path, name: &str| path.file_name().is_some_and(|found| found == name);
    (named(inner, "MacOS") && named(contents, "Contents") && bundle.extension().is_some_and(|ending| ending == "app")).then(|| bundle.to_owned())
}

/// Whether something can be put into a folder, found out by doing it.
fn writable(folder: &Path) -> bool {
    let probe = folder.join(format!(".pencraft-update-{}", std::process::id()));
    let made = std::fs::create_dir(&probe).is_ok();
    if made {
        let _ = std::fs::remove_dir(&probe);
    }
    made
}

/// How far a download is. Shared between the thread that downloads and the card that shows it.
#[derive(Debug, Default)]
pub struct Progress {
    done: AtomicU64,
    total: AtomicU64,
    stop: AtomicBool,
}

impl Progress {
    /// A download of a file this large (0 when that isn't known yet).
    #[must_use]
    pub fn of(total: u64) -> Progress {
        Progress { total: AtomicU64::new(total), ..Progress::default() }
    }

    /// Says that `done` bytes have arrived.
    pub fn arrive(&self, done: u64) {
        self.done.store(done, Ordering::Relaxed);
    }

    /// Whether the download was told to stop.
    #[must_use]
    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    /// How many bytes have arrived, and how many there are in all (0 when that isn't known).
    #[must_use]
    pub fn sizes(&self) -> (u64, u64) {
        (self.done.load(Ordering::Relaxed), self.total.load(Ordering::Relaxed))
    }

    /// How much has arrived, from 0 to 1.
    #[must_use]
    pub fn fraction(&self) -> f32 {
        let (done, total) = (self.done.load(Ordering::Relaxed), self.total.load(Ordering::Relaxed));
        if total == 0 {
            return 0.0;
        }
        let thousandths = u16::try_from(done.min(total).saturating_mul(1000) / total).unwrap_or(1000);
        f32::from(thousandths) / 1000.0
    }

    /// Tells the download to stop. It notices before its next piece.
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// A downloaded and checked update, ready to be installed as the app quits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Prepared {
    /// The setup wizard, for Windows.
    Setup(PathBuf),
    /// The new bundle, copied beside the one it replaces.
    Bundle {
        /// The new bundle.
        staged: PathBuf,
        /// The running bundle.
        bundle: PathBuf,
    },
}

impl Prepared {
    /// The command that installs the update and starts the new version. `app` is the running app's process
    /// number; the install waits for it to end.
    fn command(&self, app: u32) -> Command {
        match self {
            Prepared::Setup(setup) => {
                let mut command = Command::new(setup);
                command.args(SETUP_ARGS).arg(format!("/WAITPID={app}"));
                command
            }
            Prepared::Bundle { staged, bundle } => {
                let mut command = Command::new("/bin/sh");
                command.args(["-c", SWAP, "sh"]).arg(app.to_string()).arg(bundle).arg(staged);
                command
            }
        }
    }

    /// Starts the install. Call it as the app quits: the install waits for the app to be gone.
    ///
    /// # Errors
    /// When the installer couldn't be started.
    // Agent: SPAWNS a process that outlives the app; on Windows it may ask for administrator rights.
    pub fn start(&self) -> Result<(), String> {
        let mut command = self.command(std::process::id());
        command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        command.spawn().map(|_| ()).map_err(|error| format!("The update couldn't be started: {error}"))
    }
}

/// Where GitHub hands out a release file of `repository`.
// Human: built from the repository the release was found in and the file's number, so nothing in GitHub's
// answer can send the download anywhere else.
fn file_address(api: &str, repository: &str, id: u64) -> String {
    format!("{api}/repos/{repository}/releases/assets/{id}")
}

/// The checksum `SHA256SUMS.txt` gives for a file: 64 hex digits, in lowercase.
fn checksum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, file) = line.trim().split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name && is_hash(hash)).then(|| hash.to_ascii_lowercase())
    })
}

fn is_hash(text: &str) -> bool {
    text.len() == 64 && text.chars().all(|c| c.is_ascii_hexdigit())
}

/// The checksum in what the system's tool printed: `shasum` starts a line with it, `certutil` gives it a line
/// of its own, on old systems with spaces between the bytes.
fn hash_in(printed: &str) -> Option<String> {
    printed.lines().find_map(|line| {
        let first = line.split_whitespace().next().unwrap_or_default().to_owned();
        let joined: String = line.split_whitespace().collect();
        [first, joined].into_iter().find(|text| is_hash(text)).map(|hash| hash.to_ascii_lowercase())
    })
}

/// A program that comes with the system, by its full path so nothing else of that name is run.
fn tool(name: &str) -> Command {
    if cfg!(windows) {
        let system = std::env::var_os("SystemRoot").map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        quiet(system.join("System32").join(format!("{name}.exe")))
    } else {
        quiet(Path::new("/usr/bin").join(name))
    }
}

/// Runs a command to its end; a failure comes back with what the command said about it.
// Agent: BLOCKS until the command ends.
fn run(command: &mut Command) -> Result<String, String> {
    let name = command.get_program().to_string_lossy().into_owned();
    let output = command.output().map_err(|error| format!("{name} couldn't be started: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!("{name} failed: {}", String::from_utf8_lossy(&output.stderr).trim()))
    }
}

/// The SHA-256 checksum of a file, worked out by the system's own tool.
// Agent: RUNS `certutil` on Windows and `shasum` elsewhere; BLOCKS while it reads the file.
fn file_hash(path: &Path) -> Result<String, String> {
    let mut command = if cfg!(windows) {
        let mut command = tool("certutil");
        command.arg("-hashfile").arg(path).arg("SHA256");
        command
    } else {
        let mut command = tool("shasum");
        command.args(["-a", "256"]).arg(path);
        command
    };
    hash_in(&run(&mut command)?).ok_or_else(|| "The downloaded file's checksum couldn't be worked out.".to_owned())
}

/// Asks GitHub for a release file itself rather than its description.
// Agent: BLOCKS; SENDS the sign-in only to `address` itself (ureq drops it when GitHub redirects to its file store).
fn ask(address: &str, token: Option<&str>) -> Result<ureq::http::Response<ureq::Body>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(PATIENCE))
        .timeout_recv_response(Some(PATIENCE))
        .timeout_per_read(Some(PATIENCE))
        .build()
        .into();
    let mut request = agent
        .get(address)
        .header("User-Agent", concat!("pencraft/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/octet-stream")
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    request.call().map_err(|error| format!("GitHub couldn't be reached: {error}"))
}

/// Downloads `address` into the file `to`, telling `progress` how far it is and stopping when it says so.
/// `token` finds a sign-in; it is only asked when GitHub says there is nothing to see without one.
// Agent: BLOCKS for the whole download; WRITES `to`, which is left behind half-written on an error.
fn download(address: &str, to: &Path, progress: &Progress, token: impl FnOnce() -> Option<String>) -> Result<(), String> {
    let mut response = ask(address, None)?;
    if response.status().as_u16() == 404
        && let Some(token) = token()
    {
        response = ask(address, Some(&token))?;
    }
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("GitHub answered {status}"));
    }
    if let Some(length) = response.body().content_length() {
        progress.total.store(length, Ordering::Relaxed);
    }
    let unwritten = |error: std::io::Error| format!("The download couldn't be written: {error}");
    let mut file = std::fs::File::create(to).map_err(unwritten)?;
    let mut reader = response.body_mut().as_reader();
    let mut piece = vec![0u8; 64 * 1024];
    let mut done = 0u64;
    loop {
        if progress.stopped() {
            return Err("The download was stopped.".to_owned());
        }
        let size = reader.read(&mut piece).map_err(|error| format!("The download broke off: {error}"))?;
        if size == 0 {
            break;
        }
        done = done.saturating_add(u64::try_from(size).unwrap_or(u64::MAX));
        if done > MOST {
            return Err("The download is larger than an installer can be.".to_owned());
        }
        file.write_all(piece.get(..size).unwrap_or_default()).map_err(unwritten)?;
        progress.arrive(done);
    }
    file.flush().map_err(unwritten)
}

/// What proves that a release's checksums are ours.
struct Proof<'a> {
    /// Where `SHA256SUMS.txt.minisig` is handed out.
    signature: &'a str,
    /// The keys it may be signed with.
    keys: &'a [&'a str],
    /// The version the release says it is.
    version: &'a str,
}

/// One download of a release file and of the release's checksums.
struct Order<'a> {
    /// Where the file is handed out.
    file: &'a str,
    /// Where `SHA256SUMS.txt` is handed out.
    sums: &'a str,
    /// The file's name in `SHA256SUMS.txt`.
    name: &'a str,
    /// What the checksums must be proved with, `None` while the app knows no key.
    proof: Option<Proof<'a>>,
}

/// Downloads a release file to `to` and makes sure it is the file the release's checksums describe, and that
/// the checksums are signed when there is something to prove them with. A file that isn't is deleted.
// Human: the signature is looked at before the installer is asked for, so nothing large is downloaded from a
// release that isn't ours.
// Agent: BLOCKS; WRITES `to`, and `SHA256SUMS.txt` and `SHA256SUMS.txt.minisig` beside it.
fn receive(order: &Order<'_>, to: &Path, progress: &Progress, token: &dyn Fn() -> Option<String>) -> Result<(), String> {
    let list = to.with_file_name("SHA256SUMS.txt");
    download(order.sums, &list, &Progress::default(), token)?;
    let unread = |error: std::io::Error| format!("The release's checksums couldn't be read: {error}");
    let listed = std::fs::read(&list).map_err(unread)?;
    if let Some(proof) = &order.proof {
        let note = to.with_file_name("SHA256SUMS.txt.minisig");
        download(proof.signature, &note, &Progress::default(), token)?;
        signed(&listed, &std::fs::read_to_string(&note).map_err(unread)?, proof.keys, proof.version)?;
    }
    let sums = String::from_utf8_lossy(&listed);
    let wanted = checksum_for(&sums, order.name).ok_or_else(|| format!("The release gives no checksum for {}.", order.name))?;
    let checked = download(order.file, to, progress, token).and_then(|()| file_hash(to)).and_then(|found| {
        if found == wanted { Ok(()) } else { Err("The downloaded file is not the one the release describes, so it was not installed.".to_owned()) }
    });
    if checked.is_err() {
        let _ = std::fs::remove_file(to);
    }
    checked
}

/// Copies the app out of a disk image to beside the bundle it will replace, and gives where it put it.
// Agent: RUNS hdiutil, ditto and codesign; BLOCKS for a few seconds; WRITES `.Pencraft-update.app` beside `bundle`.
fn stage(image: &Path, bundle: &Path) -> Result<PathBuf, String> {
    let mount = image.with_file_name("mount");
    // Human: a mount left over from an update that was interrupted would be in the way.
    let _ = run(tool("hdiutil").arg("detach").arg(&mount).arg("-force"));
    std::fs::create_dir_all(&mount).map_err(|error| format!("The update couldn't be opened: {error}"))?;
    run(tool("hdiutil").args(["attach", "-nobrowse", "-readonly", "-noautoopen", "-mountpoint"]).arg(&mount).arg(image))?;
    let staged = bundle.with_file_name(".Pencraft-update.app");
    let _ = std::fs::remove_dir_all(&staged);
    let copied = run(tool("ditto").arg(mount.join(BUNDLE)).arg(&staged))
        .and_then(|_| run(tool("codesign").args(["--verify", "--strict", SIGNED_BY_US]).arg(&staged)));
    let _ = run(tool("hdiutil").arg("detach").arg(&mount).arg("-force"));
    match copied {
        Ok(_) => Ok(staged),
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staged);
            Err(error)
        }
    }
}

/// The files of a release an update in place needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Files {
    /// The installer for this system.
    pub installer: ReleaseFile,
    /// The checksums of the release's files.
    pub checksums: ReleaseFile,
    /// The signature of the checksums, when the release has one.
    pub signature: Option<ReleaseFile>,
    /// The version the release says it is.
    pub version: String,
}

/// Downloads the installer of a release, checks it, and gets it ready to be installed in `place`.
///
/// # Errors
/// When the download fails or is stopped, when the checksums are not signed with one of our keys for this
/// version (once the app knows a key), when the file is not the one the checksums describe, or when the
/// Mac's new bundle couldn't be put beside the old one.
// Agent: BLOCKS for the whole download, so call it on a thread of its own; WRITES into the temporary folder.
pub fn fetch(files: &Files, place: &Place, progress: &Progress) -> Result<Prepared, String> {
    let Files { installer, checksums, signature, version } = files;
    let signature = signature.as_ref().map(|file| file_address(API, file.repository, file.id));
    let proof = match &signature {
        Some(signature) if checks_signatures() => Some(Proof { signature, keys: KEYS, version }),
        None if checks_signatures() => return Err("The release's checksums are not signed, so the update was not installed.".to_owned()),
        _ => None,
    };
    let folder = std::env::temp_dir().join("pencraft-update");
    std::fs::create_dir_all(&folder).map_err(|error| format!("No folder to download into: {error}"))?;
    let to = folder.join(match place {
        Place::Setup => "pencraft-setup.exe",
        Place::Bundle(_) => "pencraft.dmg",
    });
    let file = file_address(API, installer.repository, installer.id);
    let order = Order { file: &file, sums: &file_address(API, checksums.repository, checksums.id), name: &installer.name, proof };
    receive(&order, &to, progress, &super::token)?;
    match place {
        Place::Setup => Ok(Prepared::Setup(to)),
        Place::Bundle(bundle) => Ok(Prepared::Bundle { staged: stage(&to, bundle)?, bundle: bundle.clone() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Answers one request after another with `replies`, and hands back what was asked, in lowercase.
    fn serve(replies: Vec<Vec<u8>>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let mut asked = Vec::new();
            for reply in replies {
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
                // Human: a download that was stopped hangs up; that is no failure of the server.
                let _ = stream.write_all(&reply);
                asked.push(String::from_utf8_lossy(&seen).to_lowercase());
            }
            asked
        });
        (address, server)
    }

    fn reply(status: &str, body: &[u8]) -> Vec<u8> {
        let mut reply = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes();
        reply.extend_from_slice(body);
        reply
    }

    /// An empty folder of this test's own in the temporary folder.
    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("pencraft-update-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    // SHA-256 of the three bytes "abc".
    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn a_mac_program_knows_its_bundle() {
        let bundle = Path::new("/Applications/Pencraft.app");
        assert_eq!(bundle_of(&bundle.join("Contents/MacOS/pencraft")).as_deref(), Some(bundle));
        assert_eq!(bundle_of(Path::new("/Users/me/pencraft/app/target/release/pencraft")), None);
        assert_eq!(bundle_of(Path::new("/Applications/Pencraft.app/Contents/Resources/pencraft")), None);
        assert_eq!(bundle_of(Path::new("/Applications/Pencraft/Contents/MacOS/pencraft")), None);
        assert_eq!(bundle_of(Path::new("pencraft")), None);
    }

    #[test]
    fn a_development_build_cant_replace_itself() {
        // Human: the test program was put there by cargo, not by the setup wizard or into a bundle.
        assert_eq!(place(), None);
    }

    #[test]
    fn a_folder_that_takes_files_is_writable() {
        let folder = folder("writable");
        assert!(writable(&folder));
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
        assert!(!writable(&folder.join("missing")));
    }

    #[test]
    fn progress_is_a_part_of_the_whole() {
        let progress = Progress::of(200);
        assert!(progress.fraction().abs() < f32::EPSILON);
        progress.done.store(50, Ordering::Relaxed);
        assert!((progress.fraction() - 0.25).abs() < f32::EPSILON);
        progress.done.store(900, Ordering::Relaxed);
        assert!((progress.fraction() - 1.0).abs() < f32::EPSILON);
        // Human: a size nobody knows yet is no progress, and no division by zero.
        assert!(Progress::default().fraction().abs() < f32::EPSILON);
        assert!(!progress.stopped());
        progress.cancel();
        assert!(progress.stopped());
    }

    #[test]
    fn a_release_file_is_only_ever_asked_for_in_this_repository() {
        assert_eq!(file_address(API, "AsP3X/pencraft", 42), "https://api.github.com/repos/AsP3X/pencraft/releases/assets/42");
        assert_eq!(file_address(API, "AsP3X/quill", 42), "https://api.github.com/repos/AsP3X/quill/releases/assets/42");
    }

    #[test]
    fn a_checksum_is_found_by_the_files_name() {
        let sums = format!("{}  pencraft-9.1.0-x64.msi\n{ABC}  pencraft-9.1.0-setup.exe\n{} *pencraft-9.1.0.dmg\n", "0".repeat(64), "F".repeat(64));
        assert_eq!(checksum_for(&sums, "pencraft-9.1.0-setup.exe").as_deref(), Some(ABC));
        assert_eq!(checksum_for(&sums, "pencraft-9.1.0.dmg"), Some("f".repeat(64)));
        assert_eq!(checksum_for(&sums, "setup.exe"), None);
        assert_eq!(checksum_for("not-a-hash  pencraft-9.1.0-setup.exe", "pencraft-9.1.0-setup.exe"), None);
        assert_eq!(checksum_for("", "pencraft-9.1.0-setup.exe"), None);
    }

    #[test]
    fn a_checksum_is_read_from_what_the_systems_tool_prints() {
        let certutil = format!(
            "SHA256 hash of C:\\Temp\\pencraft-setup.exe:\r\n{}\r\nCertUtil: -hashfile command completed successfully.\r\n",
            ABC.to_uppercase()
        );
        assert_eq!(hash_in(&certutil).as_deref(), Some(ABC));
        let spaced: Vec<&str> = (0..32).map(|byte| &ABC[byte * 2..byte * 2 + 2]).collect();
        assert_eq!(hash_in(&format!("SHA256 hash of file x:\r\n{}\r\n", spaced.join(" "))).as_deref(), Some(ABC));
        assert_eq!(hash_in(&format!("{ABC}  /tmp/pencraft.dmg\n")).as_deref(), Some(ABC));
        assert_eq!(hash_in("certutil: file not found"), None);
    }

    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn the_system_works_out_a_files_checksum() {
        let file = folder("hash").join("abc.txt");
        std::fs::write(&file, "abc").unwrap();
        assert_eq!(file_hash(&file).unwrap(), ABC);
        assert!(file_hash(&file.with_file_name("missing")).is_err());
    }

    #[test]
    fn a_download_arrives_whole_and_says_how_far_it_is() {
        let to = folder("download").join("file");
        let (address, server) = serve(vec![reply("200 OK", b"abc")]);
        let progress = Progress::default();
        download(&address, &to, &progress, || -> Option<String> { panic!("no sign-in is needed") }).unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"abc");
        assert!((progress.fraction() - 1.0).abs() < f32::EPSILON);
        let asked = server.join().unwrap();
        assert!(asked[0].contains("accept: application/octet-stream"));
        assert!(!asked[0].contains("authorization"));
    }

    #[test]
    fn a_download_asks_with_a_sign_in_when_there_is_nothing_to_see_without_one() {
        let to = folder("private").join("file");
        let (address, server) = serve(vec![reply("404 Not Found", b"no"), reply("200 OK", b"abc")]);
        download(&address, &to, &Progress::default(), || Some("secret".to_owned())).unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"abc");
        let asked = server.join().unwrap();
        assert!(!asked[0].contains("authorization"));
        assert!(asked[1].contains("authorization: bearer secret"));

        // Human: without a sign-in, or when GitHub refuses, the user is told what GitHub answered.
        let (address, server) = serve(vec![reply("404 Not Found", b"no")]);
        assert_eq!(download(&address, &to, &Progress::default(), || None).unwrap_err(), "GitHub answered 404");
        server.join().unwrap();
    }

    #[test]
    fn a_stopped_download_writes_nothing_more() {
        let to = folder("stopped").join("file");
        let (address, server) = serve(vec![reply("200 OK", b"abc")]);
        let progress = Progress::default();
        progress.cancel();
        assert_eq!(download(&address, &to, &progress, || None).unwrap_err(), "The download was stopped.");
        assert_eq!(std::fs::read(&to).unwrap(), b"");
        server.join().unwrap();
    }

    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn only_the_file_the_release_describes_is_kept() {
        let to = folder("receive").join("pencraft-setup.exe");
        let sums = format!("{ABC}  pencraft-9.1.0-setup.exe\n");
        let (address, server) = serve(vec![reply("200 OK", sums.as_bytes()), reply("200 OK", b"abc")]);
        let order = Order { file: &address, sums: &address, name: "pencraft-9.1.0-setup.exe", proof: None };
        receive(&order, &to, &Progress::default(), &|| None).unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"abc");
        server.join().unwrap();

        // Human: another file under the same name is thrown away.
        let (address, server) = serve(vec![reply("200 OK", sums.as_bytes()), reply("200 OK", b"abd")]);
        let order = Order { file: &address, sums: &address, name: "pencraft-9.1.0-setup.exe", proof: None };
        assert!(receive(&order, &to, &Progress::default(), &|| None).unwrap_err().starts_with("The downloaded file is not the one"));
        assert!(!to.exists());
        server.join().unwrap();

        // Human: a release that says nothing about the file gets nothing downloaded.
        let (address, server) = serve(vec![reply("200 OK", b"")]);
        let order = Order { file: &address, sums: &address, name: "pencraft-9.1.0-setup.exe", proof: None };
        assert_eq!(receive(&order, &to, &Progress::default(), &|| None).unwrap_err(), "The release gives no checksum for pencraft-9.1.0-setup.exe.");
        assert_eq!(server.join().unwrap().len(), 1);
    }

    // Human: two keys made for these tests and a signature the first made over the checksums below, as
    // `minisign -S -t "Quill 9.1.0"` writes it. Their secret halves were thrown away.
    const KEY: &str = "RWRRdWlsbFRzQUkgZ4DaxaamZh4LiHmEirzY5ZF+AMXa1F9hvKfiWDlJ";
    const OTHER_KEY: &str = "RWRRdWlsbFRzQnXk9XwybfG50Ao+hMED9PvAaLOyGCzQ+VyYL6oSh01c";
    const SIGNATURE: &str = "untrusted comment: signature from a key made for Quill's tests\n\
        RURRdWlsbFRzQU9+Jg/TPpKs+pttu/HUuoRYvLjgFFoa4dSKQJvYAOnZV2cCZLAcrLjUN2BLX9fGmIfDicMbBy/YTlpt91g6IwE=\n\
        trusted comment: Quill 9.1.0\n\
        gFfbVc/GLLneJGKyiAuAS6DpfmdqJmFWTEv1GiVSxTwHpGSw25XGtcNMhChGIgHDeQmE7H7VDiq8UPuKrbz1Aw==\n";

    fn signed_sums() -> String {
        format!("{ABC}  pencraft-9.1.0-setup.exe\n")
    }

    #[test]
    fn checksums_count_only_when_one_of_our_keys_signed_them_for_this_version() {
        let sums = signed_sums();
        assert_eq!(signed(sums.as_bytes(), SIGNATURE, &[KEY], "9.1.0"), Ok(()));
        // Human: either of the two keys will do, and one that can't be read is passed over.
        assert_eq!(signed(sums.as_bytes(), SIGNATURE, &["not a key", OTHER_KEY, KEY], "9.1.0"), Ok(()));
        // Human: Windows line ends in the signature file change nothing.
        assert_eq!(signed(sums.as_bytes(), &SIGNATURE.replace('\n', "\r\n"), &[KEY], "9.1.0"), Ok(()));

        let not_ours = "The release's checksums are not signed by Quill's makers, so the update was not installed.";
        assert_eq!(signed(sums.as_bytes(), SIGNATURE, &[OTHER_KEY], "9.1.0").unwrap_err(), not_ours);
        assert_eq!(signed(sums.as_bytes(), SIGNATURE, &[], "9.1.0").unwrap_err(), not_ours);
        // Human: other checksums under the same signature: what someone who changed the installer would send.
        let changed = sums.replace("ba78", "ba79");
        assert_eq!(signed(changed.as_bytes(), SIGNATURE, &[KEY], "9.1.0").unwrap_err(), not_ours);
        assert_eq!(signed(sums.as_bytes(), "", &[KEY], "9.1.0").unwrap_err(), not_ours);
        assert_eq!(signed(sums.as_bytes(), "not a signature\nat all\n", &[KEY], "9.1.0").unwrap_err(), not_ours);
        // Human: a comment changed after signing breaks the signature, it doesn't pass for the other version.
        let relabelled = SIGNATURE.replace("Quill 9.1.0", "Quill 9.2.0");
        assert_eq!(signed(sums.as_bytes(), &relabelled, &[KEY], "9.2.0").unwrap_err(), not_ours);

        // Human: an old release's signed checksums, handed out again as a newer version.
        assert_eq!(
            signed(sums.as_bytes(), SIGNATURE, &[KEY], "9.2.0").unwrap_err(),
            "The release's signature was made for another version, so the update was not installed."
        );
    }

    #[test]
    fn signatures_are_asked_for_as_soon_as_the_app_knows_a_key() {
        assert!(!demanded(&[]));
        assert!(demanded(&[KEY]));
        assert_eq!(checks_signatures(), demanded(KEYS));
        // Human: a key that was pasted in wrong would refuse every release.
        for key in KEYS {
            assert!(PublicKey::from_base64(key).is_ok(), "{key} is not a minisign public key");
        }
    }

    #[cfg(any(windows, target_os = "macos"))]
    #[test]
    fn an_installer_is_only_asked_for_after_the_checksums_proved_to_be_ours() {
        let to = folder("signed").join("pencraft-setup.exe");
        let sums = signed_sums();
        let order = |address: &'static str, keys: &'static [&'static str], version: &'static str| Order {
            file: address,
            sums: address,
            name: "pencraft-9.1.0-setup.exe",
            proof: Some(Proof { signature: address, keys, version }),
        };
        let (address, server) = serve(vec![reply("200 OK", sums.as_bytes()), reply("200 OK", SIGNATURE.as_bytes()), reply("200 OK", b"abc")]);
        let address: &'static str = address.leak();
        receive(&order(address, &[KEY], "9.1.0"), &to, &Progress::default(), &|| None).unwrap();
        assert_eq!(std::fs::read(&to).unwrap(), b"abc");
        assert_eq!(server.join().unwrap().len(), 3);
        std::fs::remove_file(&to).unwrap();

        // Human: signed with a key the app doesn't know: the installer is never downloaded.
        let (address, server) = serve(vec![reply("200 OK", sums.as_bytes()), reply("200 OK", SIGNATURE.as_bytes())]);
        let address: &'static str = address.leak();
        assert!(receive(&order(address, &[OTHER_KEY], "9.1.0"), &to, &Progress::default(), &|| None).unwrap_err().contains("not signed by"));
        assert_eq!(server.join().unwrap().len(), 2);
        assert!(!to.exists());

        // Human: a release whose signature can't be had is no release to install.
        let (address, server) = serve(vec![reply("200 OK", sums.as_bytes()), reply("404 Not Found", b"no")]);
        let address: &'static str = address.leak();
        assert_eq!(receive(&order(address, &[KEY], "9.1.0"), &to, &Progress::default(), &|| None).unwrap_err(), "GitHub answered 404");
        server.join().unwrap();
        assert!(!to.exists());
    }

    #[test]
    fn the_setup_wizard_runs_without_its_pages_and_starts_the_app_again() {
        let command = Prepared::Setup(PathBuf::from("pencraft-setup.exe")).command(7);
        assert_eq!(command.get_program(), "pencraft-setup.exe");
        assert_eq!(command.get_args().collect::<Vec<_>>(), ["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/RELAUNCH", "/WAITPID=7"]);
    }

    #[test]
    fn the_mac_swap_waits_for_the_app_and_opens_the_new_one() {
        let prepared =
            Prepared::Bundle { staged: PathBuf::from("/Applications/.Pencraft-update.app"), bundle: PathBuf::from("/Applications/Pencraft.app") };
        let command = prepared.command(7);
        assert_eq!(command.get_program(), "/bin/sh");
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["-c", SWAP, "sh", "7", "/Applications/Pencraft.app", "/Applications/.Pencraft-update.app"]
        );
        assert!(SWAP.starts_with(r#"while kill -0 "$1""#));
        assert!(SWAP.ends_with(r#"/usr/bin/open "$2""#));
    }

    /// Runs the swap script on folders that stand in for the bundles, with `open` left out.
    #[cfg(unix)]
    #[test]
    fn the_swap_puts_the_new_bundle_in_the_old_ones_place() {
        let folder = folder("swap");
        let (bundle, staged) = (folder.join("Pencraft.app"), folder.join(".Pencraft-update.app"));
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::create_dir_all(&staged).unwrap();
        std::fs::write(bundle.join("version"), "old").unwrap();
        std::fs::write(staged.join("version"), "new").unwrap();
        let script = SWAP.replace("/usr/bin/open", "true");
        // Human: a number no process has stands for the app that has already quit.
        let status = Command::new("/bin/sh").args(["-c", &script, "sh", "4194000"]).arg(&bundle).arg(&staged).status().unwrap();
        assert!(status.success());
        assert_eq!(std::fs::read_to_string(bundle.join("version")).unwrap(), "new");
        assert!(!staged.exists());
        assert!(!folder.join("Pencraft.app.old").exists());
    }
}
