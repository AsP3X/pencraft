//! Crash recovery: documents with unsaved changes are written beside the user's files while they are worked
//! on, and offered again at the next start when the app didn't close properly.
// Human: a crash, a power cut or a forced quit must not cost the work since the last save. Every few seconds
// the documents that have changes are written to a folder of this run under `~/.pencraft/recovery`. A run that
// ends properly takes its folder with it; one that doesn't leaves it, and the next start finds it.
// Agent: the files are written and removed by one thread of this module's own, in the order asked for, so a
// document saved right after it was written for recovery never leaves a stale copy. The window's thread only
// copies the document.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use pen_doc::Document;
use serde_json::{Value, json};

/// The file in a run's folder that the running app keeps locked.
const HELD: &str = ".held";
/// The ending of a kept document, and of the few facts kept beside it.
const DOCUMENT: &str = "pen";
const FACTS: &str = "json";
/// A changed document is written again this long after it was last written, at the earliest.
pub const EVERY: Duration = Duration::from_secs(5);
/// A run's folder without its lock file is taken for a crashed run's only when it is this old: a run that is
/// just starting has made the folder and not the lock yet.
const SETTLED_AFTER: Duration = Duration::from_secs(10);

/// The folder the runs' folders are made in.
// Human: the tests keep out of the home folder.
fn root() -> PathBuf {
    if cfg!(test) {
        return std::env::temp_dir().join(format!("pencraft-recovery-test-{}", std::process::id()));
    }
    crate::imagery::store::home().map_or_else(|| std::env::temp_dir().join("pencraft-recovery"), |home| home.join(".pencraft").join("recovery"))
}

/// What the writing thread is asked to do.
enum Job {
    /// Write the document with this number ([`crate::editor::Editor::id`]), with the file it belongs to.
    Keep { id: u64, doc: Box<Document>, path: Option<PathBuf>, title: String },
    /// Remove what was written for the document with this number.
    Forget(u64),
}

/// What the keeper is told of an open document when it looks at it.
#[derive(Clone, Copy, Debug)]
pub struct Seen<'a> {
    /// Its number among the documents of this run ([`crate::editor::Editor::id`]).
    pub id: u64,
    /// Whether it has unsaved changes.
    pub dirty: bool,
    /// A number that changes whenever the document does ([`crate::editor::Editor::generation`]).
    pub scene: u64,
    /// The file it belongs to, if it has one.
    pub path: Option<&'a Path>,
    /// Its name, as its tab shows it.
    pub title: &'a str,
}

/// Keeps the changed documents of this run on disk until they are saved or closed.
pub struct Keeper {
    folder: PathBuf,
    held: File,
    jobs: Sender<Job>,
    writer: JoinHandle<()>,
    /// For each document kept: the scene it had when it was last handed over, and when that was.
    kept: HashMap<u64, (u64, Instant)>,
    /// What went wrong with the writing thread's last job, if it failed; `None` once one works again.
    trouble: Arc<Mutex<Option<String>>>,
}

impl Keeper {
    /// Makes this run's folder, in `root`, and starts the thread that writes into it.
    ///
    /// # Errors
    /// A sentence when the folder can't be made or the thread can't be started.
    // Agent: WRITES a folder under `~/.pencraft/recovery` and holds a lock on a file in it for as long as it lives.
    pub fn start() -> Result<Keeper, String> {
        Keeper::start_in(&root())
    }

    fn start_in(root: &Path) -> Result<Keeper, String> {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_nanos());
        let folder = root.join(format!("{}-{stamp}", std::process::id()));
        let made = || -> std::io::Result<File> {
            std::fs::create_dir_all(&folder)?;
            let held = File::create(folder.join(HELD))?;
            held.lock()?;
            Ok(held)
        };
        let held = made().map_err(|error| format!("the recovery folder {} couldn't be made: {error}", folder.display()))?;
        let (jobs, asked) = channel();
        let into = folder.clone();
        let trouble = Arc::new(Mutex::new(None));
        let told = Arc::clone(&trouble);
        let writer = std::thread::Builder::new()
            .name("recovery".to_owned())
            .spawn(move || write_all(&into, &asked, &told))
            .map_err(|error| error.to_string())?;
        Ok(Keeper { folder, held, jobs, writer, kept: HashMap::new(), trouble })
    }

    /// What went wrong when the writing thread last failed, while its jobs still fail; `None` once one works.
    // Agent: MAIN-THREAD; a short lock shared with the writing thread.
    #[must_use]
    pub fn trouble(&self) -> Option<String> {
        self.trouble.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Looks at one open document: one with unsaved changes that differs from what was last kept is handed to
    /// the writing thread, at most every [`EVERY`]; one without changes is no longer kept.
    ///
    /// Returns whether it has changes that wait for their turn, so the caller looks again after [`EVERY`].
    // Agent: MAIN-THREAD; copies the document when it is handed over, nothing else. Call it while no drag is
    // under way: a drag changes the document with every frame.
    pub fn look(&mut self, seen: &Seen<'_>, doc: &Document, now: Instant) -> bool {
        if !seen.dirty {
            self.forget(seen.id);
            return false;
        }
        let (changed, due) = match self.kept.get(&seen.id) {
            Some((kept, at)) => (*kept != seen.scene, now.saturating_duration_since(*at) >= EVERY),
            None => (true, true),
        };
        if !changed || !due {
            return changed;
        }
        let job = Job::Keep { id: seen.id, doc: Box::new(doc.clone()), path: seen.path.map(Path::to_path_buf), title: seen.title.to_owned() };
        if self.jobs.send(job).is_ok() {
            self.kept.insert(seen.id, (seen.scene, now));
        }
        false
    }

    /// Stops keeping the document with the number `id`: it was saved, or closed by the user's choice.
    pub fn forget(&mut self, id: u64) {
        if self.kept.remove(&id).is_some() && self.jobs.send(Job::Forget(id)).is_err() {
            log::warn!("recovery: the writing thread is gone");
        }
    }

    /// Ends this run's keeping the proper way: what is still to be written is written, then the folder goes.
    // Agent: BLOCKS until the writing thread is done; call it as the app quits.
    pub fn finish(self) {
        let Keeper { folder, held, jobs, writer, .. } = self;
        drop(jobs);
        if writer.join().is_err() {
            log::warn!("recovery: the writing thread failed");
        }
        drop(held);
        if let Err(error) = std::fs::remove_dir_all(&folder) {
            log::warn!("recovery folder {}: {error}", folder.display());
        }
    }
}

/// Does the jobs as they come, until nobody can ask for more, and says in `trouble` whether the last one failed.
// Human: the window tells the user once when unsaved work can't be kept for recovery (the disk is full, the
// folder went away), and again only if it fails after having worked.
fn write_all(folder: &Path, asked: &Receiver<Job>, trouble: &Mutex<Option<String>>) {
    for job in asked {
        let failed = match job {
            Job::Keep { id, doc, path, title } => write(folder, id, &doc, path.as_deref(), &title).err().map(|error| {
                log::warn!("recovery: {title} wasn't kept: {error}");
                format!("Unsaved changes to “{title}” couldn't be kept for recovery: {error}")
            }),
            Job::Forget(id) => {
                let mut failed = None;
                for ending in [FACTS, DOCUMENT] {
                    let file = folder.join(format!("{id}.{ending}"));
                    if file.exists()
                        && let Err(error) = std::fs::remove_file(&file)
                    {
                        log::warn!("recovery: {}: {error}", file.display());
                        failed = Some(format!("A recovery copy couldn't be removed: {error}"));
                    }
                }
                failed
            }
        };
        *trouble.lock().unwrap_or_else(PoisonError::into_inner) = failed;
    }
}

/// Writes one document and the facts about it, each crash-safely; the facts last, so a document is only ever
/// found whole.
fn write(folder: &Path, id: u64, doc: &Document, path: Option<&Path>, title: &str) -> Result<(), String> {
    let text = pen_doc::write(doc).map_err(|error| error.to_string())?;
    craft_atomic::atomic_write(&folder.join(format!("{id}.{DOCUMENT}")), text.as_bytes()).map_err(|error| error.to_string())?;
    let facts = json!({ "path": path.map(|path| path.display().to_string()), "title": title });
    craft_atomic::atomic_write(&folder.join(format!("{id}.{FACTS}")), facts.to_string().as_bytes()).map_err(|error| error.to_string())
}

/// A document an earlier run left behind with unsaved changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Left {
    /// The file it was kept in.
    file: PathBuf,
    /// The file it belongs to; `None` for one that was never saved.
    pub path: Option<PathBuf>,
    /// Its name, as its tab showed it.
    pub title: String,
}

impl Left {
    /// Reads the document as it was last kept.
    ///
    /// # Errors
    /// A sentence when the kept file can't be read.
    // Agent: READS the kept file whole. BLOCKS.
    pub fn document(&self) -> Result<Document, String> {
        let text = std::fs::read_to_string(&self.file).map_err(|error| error.to_string())?;
        pen_doc::read(&text).map_err(|error| error.to_string())
    }
}

/// The documents the runs that didn't end properly left behind, oldest run first.
// Human: a run's folder is a crashed run's when no running Quill holds the lock in it. A folder with nothing
// to offer in it is cleared away on the way.
// Agent: READS and WRITES under the recovery root; BLOCKS on small files only (the documents are not read).
#[must_use]
pub fn left() -> Vec<Left> {
    left_in(&root())
}

fn left_in(root: &Path) -> Vec<Left> {
    let Ok(folders) = std::fs::read_dir(root) else { return Vec::new() };
    let mut folders: Vec<PathBuf> = folders.filter_map(Result::ok).map(|entry| entry.path()).filter(|path| path.is_dir()).collect();
    folders.sort();
    let mut found = Vec::new();
    for folder in folders {
        let free = match File::open(folder.join(HELD)) {
            Ok(held) => held.try_lock().is_ok(),
            Err(_) => std::fs::metadata(&folder)
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|at| at.elapsed().ok())
                .is_some_and(|age| age > SETTLED_AFTER),
        };
        if !free {
            continue;
        }
        let here = left_by(&folder);
        if here.is_empty()
            && let Err(error) = std::fs::remove_dir_all(&folder)
        {
            log::warn!("recovery folder {}: {error}", folder.display());
        }
        found.extend(here);
    }
    found
}

/// The documents kept in one run's folder: those whose facts and document are both there.
fn left_by(folder: &Path) -> Vec<Left> {
    let Ok(files) = std::fs::read_dir(folder) else { return Vec::new() };
    let mut facts: Vec<PathBuf> =
        files.filter_map(Result::ok).map(|entry| entry.path()).filter(|path| path.extension().is_some_and(|ending| ending == FACTS)).collect();
    facts.sort();
    facts
        .into_iter()
        .filter_map(|facts| {
            let file = facts.with_extension(DOCUMENT);
            let read: Value = serde_json::from_slice(&std::fs::read(&facts).ok()?).ok()?;
            let title = read.get("title").and_then(Value::as_str).unwrap_or("Untitled").to_owned();
            let path = read.get("path").and_then(Value::as_str).map(PathBuf::from);
            file.is_file().then_some(Left { file, path, title })
        })
        .collect()
}

/// Clears away what was kept of `handled`: they were restored, or the user doesn't want them. A run's folder
/// that is empty afterwards goes too.
// Agent: WRITES under the recovery root.
pub fn clear(handled: &[Left]) {
    for left in handled {
        for file in [left.file.with_extension(FACTS), left.file.clone()] {
            if let Err(error) = std::fs::remove_file(&file) {
                log::warn!("recovery: {}: {error}", file.display());
            }
        }
        if let Some(folder) = left.file.parent()
            && left_by(folder).is_empty()
            && let Err(error) = std::fs::remove_dir_all(folder)
        {
            log::warn!("recovery folder {}: {error}", folder.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A root of its own for one test.
    fn fresh(name: &str) -> PathBuf {
        let root = root().join(name);
        drop(std::fs::remove_dir_all(&root));
        root
    }

    fn doc(width: u32) -> Document {
        pen_doc::read(&format!(r#"{{"version":"2.22","children":[{{"type":"rectangle","id":"r","x":0,"y":0,"width":{width},"height":10}}]}}"#))
            .unwrap()
    }

    fn seen<'a>(id: u64, dirty: bool, scene: u64, path: Option<&'a Path>, title: &'a str) -> Seen<'a> {
        Seen { id, dirty, scene, path, title }
    }

    /// Ends a keeper the way a crash does: its lock goes, its files stay.
    fn crash(keeper: Keeper) {
        let Keeper { held, jobs, writer, .. } = keeper;
        drop(jobs);
        writer.join().unwrap();
        drop(held);
    }

    #[test]
    fn a_changed_document_is_found_after_a_crash_and_not_after_a_proper_end() {
        let root = fresh("crash");
        let now = Instant::now();
        let mut keeper = Keeper::start_in(&root).unwrap();
        assert!(!keeper.look(&seen(1, true, 10, Some(Path::new("/designs/shop.pen")), "shop"), &doc(40), now));
        let _ = keeper.look(&seen(2, true, 11, None, "Untitled"), &doc(50), now);
        // A document without changes is not kept.
        let _ = keeper.look(&seen(3, false, 12, None, "Untitled"), &doc(60), now);
        // While the run lives, its folder is nobody's to take.
        assert_eq!(left_in(&root), Vec::new());
        crash(keeper);
        let found = left_in(&root);
        assert_eq!(
            found.iter().map(|left| (left.path.clone(), left.title.as_str())).collect::<Vec<_>>(),
            vec![(Some(PathBuf::from("/designs/shop.pen")), "shop"), (None, "Untitled")]
        );
        assert_eq!(pen_doc::write(&found[0].document().unwrap()).unwrap(), pen_doc::write(&doc(40)).unwrap());
        // Handled, they are gone, and so is the run's folder.
        clear(&found);
        assert_eq!(left_in(&root), Vec::new());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        // A run that ends properly leaves nothing.
        let mut keeper = Keeper::start_in(&root).unwrap();
        let _ = keeper.look(&seen(1, true, 10, None, "Untitled"), &doc(40), now);
        keeper.finish();
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    }

    #[test]
    fn a_document_is_written_again_only_when_it_changed_and_time_has_passed_and_forgotten_once_saved() {
        let root = fresh("again");
        let now = Instant::now();
        let mut keeper = Keeper::start_in(&root).unwrap();
        let _ = keeper.look(&seen(1, true, 10, None, "Untitled"), &doc(40), now);
        // Changed a second later: too soon. Unchanged after ten: nothing to write. Changed after ten: written.
        assert!(keeper.look(&seen(1, true, 11, None, "Untitled"), &doc(41), now + Duration::from_secs(1)));
        let _ = keeper.look(&seen(1, true, 10, None, "Untitled"), &doc(42), now + Duration::from_secs(10));
        assert_eq!(keeper.kept.get(&1).map(|kept| kept.0), Some(10));
        let _ = keeper.look(&seen(1, true, 12, None, "Untitled"), &doc(43), now + Duration::from_secs(10));
        assert_eq!(keeper.kept.get(&1).map(|kept| kept.0), Some(12));
        // A second document is saved (no changes left): what was kept of it goes, the first stays as last written.
        let _ = keeper.look(&seen(2, true, 20, None, "Untitled"), &doc(70), now);
        let _ = keeper.look(&seen(2, false, 20, None, "Untitled"), &doc(70), now);
        crash(keeper);
        let found = left_in(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(pen_doc::write(&found[0].document().unwrap()).unwrap(), pen_doc::write(&doc(43)).unwrap());
        clear(&found);
    }
}
