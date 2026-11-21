//! One picture being made, on a thread of its own: asking the service, writing the file, and making the small
//! copy a model is shown. The app asks now and then whether it is done.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::service::Request;
use super::sources::{self, Take};
use super::store::{self, Saved};
use super::{Ready, key, make};
use crate::secrets::Keychain;

/// How long a picture may take. Under the 120 seconds the local server waits for a tool's answer
/// (`link::PATIENCE`), so a model is always told what happened.
pub const LIMIT: Duration = Duration::from_secs(100);
/// The longest side of the copy a model is shown, in pixels.
const SHOWN_SIDE: u32 = 1024;
/// What the work of a job given up on ends with; nobody hears it.
const GIVEN_UP: &str = "Nobody waits for the picture any more; it was not saved.";

/// What to make and where to keep it.
#[derive(Clone, Debug)]
pub struct Order {
    /// The service to ask.
    pub ready: Ready,
    /// The picture to ask for.
    pub request: Request,
    /// The document's `.pen` file, if it has one: the picture goes beside it.
    pub document: Option<PathBuf>,
    /// The pictures to send with the request, got ready on the job's thread; empty for a plain generation.
    pub takes: Vec<Take>,
}

impl Order {
    /// An order to make the picture `request` describes from words alone, with no pictures sent.
    #[must_use]
    pub fn new(ready: Ready, request: Request, document: Option<PathBuf>) -> Order {
        Order { ready, request, document, takes: Vec::new() }
    }
}

/// A picture that was made and written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Done {
    /// The file and what a fill calls it.
    pub saved: Saved,
    /// A small PNG copy, for a model to look at.
    pub shown: Vec<u8>,
    /// The service that made it.
    pub service: String,
    /// The model that made it.
    pub model: String,
}

/// What a job's work and the app tell each other while a picture is made.
#[derive(Debug, Default)]
pub struct Flags {
    /// Set once nobody waits for the result any more: the limit passed, or the `Job` was dropped.
    given_up: AtomicBool,
    /// Set by the work just before it decides whether to save.
    saving: AtomicBool,
    /// Set when the `Job` is dropped: nobody will ever take the result. A lock rather than a flag, so the
    /// result is handed over either wholly before the drop or not at all (see `Job::start_with`).
    dropped: Mutex<bool>,
}

impl Flags {
    /// Says the work is about to save the picture, and whether it may: `false` once nobody waits for it.
    // Human: once this has said yes, the job waits for the save even past its time limit (see `Job::poll`).
    // Agent: `saving` is set before `given_up` is read, all `SeqCst`; `Job::poll` does the two the other way
    // round, so at least one side sees the other's flag.
    pub fn may_save(&self) -> bool {
        self.saving.store(true, Ordering::SeqCst);
        !self.given_up.load(Ordering::SeqCst)
    }

    /// Whether nobody waits for the result any more: the limit passed, or the `Job` was dropped.
    pub fn given_up(&self) -> bool {
        self.given_up.load(Ordering::SeqCst)
    }

    /// Whether the `Job` was dropped.
    fn dropped(&self) -> bool {
        *self.dropped.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Saves with `save` when `flags` allow it ([`Flags::may_save`]), and takes the save back with `unsave` when
/// the job was dropped while it saved.
///
/// # Errors
/// What `save` said, or [`GIVEN_UP`] when the job was given up on before the save or dropped during it.
// Human: a time limit that passes during the save doesn't take it back: the job waits for a save it let start
// (`Job::poll`), so its caller gets the picture. Only a dropped job (the dialog cancelled, the chat stopped)
// has nobody left to give it to.
fn keep(flags: &Flags, save: impl FnOnce() -> Result<Saved, String>, unsave: impl FnOnce(&Saved)) -> Result<Saved, String> {
    if !flags.may_save() {
        return Err(GIVEN_UP.to_owned());
    }
    let saved = save()?;
    if flags.dropped() {
        unsave(&saved);
        return Err(GIVEN_UP.to_owned());
    }
    Ok(saved)
}

/// Takes back the picture at `path` that nobody will take; a failure is only logged, as nobody is there to tell.
fn discard(path: &Path) {
    if let Err(why) = store::forget(path) {
        log::warn!("a picture nobody waits for stayed: {why}");
    }
}

/// Does `order` from start to end: gets its pictures ready, reads the key, asks the service, writes the picture
/// and its credit. A job given up on before the service is asked asks nothing; once `flags` say nobody waits
/// any more, nothing is written; a job dropped during the save takes it back.
///
/// # Errors
/// A sentence for a person or a model; nothing is left half-written.
// Human: the caller of a job given up on was told no picture was kept, so a picture that arrives after that
// is not kept: it would be a file nobody asked for any more.
// Agent: BLOCKS up to 95 s on the network; READS the keychain; WRITES `images/` only when `may_save` allows,
// and removes what it wrote again when the job was dropped meanwhile (`keep`). The small copy is made first: it
// writes nothing, so when it fails nothing has been written. Worker thread only.
pub fn work(order: &Order, flags: &Flags) -> Result<Done, String> {
    let name = &order.ready.name;
    // Human: the pictures are got ready before the key is read, so one that can't be read is said without
    // asking the service anything.
    let request = Request { sources: sources::prepare(&order.takes)?, ..order.request.clone() };
    // Human: a job stopped while its pictures were got ready (the chat stopped, the dialog cancelled) sends
    // nothing: the user's pictures don't leave the computer for a picture nobody waits for.
    if flags.given_up() {
        return Err(GIVEN_UP.to_owned());
    }
    let key = key(&order.ready, &Keychain).map_err(|failure| failure.sentence(name))?;
    let made = make(&order.ready, &request, &key).map_err(|failure| failure.sentence(name))?;
    let shown = store::shown(&made.0, SHOWN_SIDE)?;
    let names: Vec<&str> = sources::in_order(&order.takes).into_iter().map(|take| take.name.as_str()).collect();
    let credit = credit(order, &names);
    let saved = keep(
        flags,
        || store::save(order.document.as_deref(), store::home().as_deref(), &made.0, &order.request.prompt, &credit),
        |saved| discard(&saved.path),
    )?;
    Ok(Done { saved, shown, service: name.clone(), model: order.request.model.clone() })
}

/// What `images/credits.json` says of the picture `order` made: generated, or edited from the pictures `names`
/// (in the order they were sent).
fn credit(order: &Order, names: &[&str]) -> Value {
    let (service, model, prompt) = (&order.ready.name, &order.request.model, &order.request.prompt);
    if names.is_empty() {
        json!({ "made": "generated", "service": service, "model": model, "prompt": prompt })
    } else {
        json!({ "made": "edited", "service": service, "model": model, "prompt": prompt, "from": names })
    }
}

/// A picture being made.
pub struct Job {
    got: Receiver<Result<Done, String>>,
    since: Instant,
    limit: Duration,
    /// What the job and its work tell each other.
    flags: Arc<Flags>,
}

/// Calls the function it holds when it is dropped: when a job's thread ends, however it ends.
struct Waking<F: Fn()>(Arc<F>);

impl<F: Fn()> Drop for Waking<F> {
    fn drop(&mut self) {
        (self.0)();
    }
}

/// What a model or a person is told when a picture took longer than `limit`.
// Human: not "nothing was made": the service may have made the picture (and charged for it) all the same.
fn too_long(limit: Duration) -> String {
    match limit.as_secs() {
        0 => "The picture took too long; none was kept.".to_owned(),
        1 => "The picture took too long (over 1 second); none was kept.".to_owned(),
        seconds => format!("The picture took too long (over {seconds} seconds); none was kept."),
    }
}

impl Job {
    /// Starts `work` on a thread of its own; `wake` is called when it has ended. `work` is handed [`Flags`]
    /// and writes only when [`Flags::may_save`] allows it.
    pub fn start(work: impl FnOnce(&Flags) -> Result<Done, String> + Send + 'static, wake: impl Fn() + Send + Sync + 'static) -> Job {
        Job::start_with(work, wake, LIMIT)
    }

    /// [`Job::start`] with another time limit.
    // Human: `wake` is called however the thread ends (with a result, or by a panic in the work), and at once
    // when no thread could be started, so the app always looks again.
    // Agent: SPAWNS one thread per job. A job given up on (limit passed, or the `Job` dropped) lets its thread
    // run to the end of its network call; its flags say so then, so the work keeps nothing. The result is
    // handed over under the `dropped` lock: either it is in the channel before the drop takes that lock (and the
    // drop takes the picture back, `Drop for Job`), or the drop came first and the picture is taken back here.
    // So a dropped job leaves no picture, however the two meet.
    fn start_with(
        work: impl FnOnce(&Flags) -> Result<Done, String> + Send + 'static,
        wake: impl Fn() + Send + Sync + 'static,
        limit: Duration,
    ) -> Job {
        let (send, got) = channel();
        let flags = Arc::new(Flags::default());
        let wake = Arc::new(wake);
        let (shared, waker) = (Arc::clone(&flags), Arc::clone(&wake));
        let started = std::thread::Builder::new().name("make-picture".to_owned()).spawn(move || {
            // The guard comes first, so it is dropped last: the channel is closed before the app is woken.
            let _waking = Waking(waker);
            let send = send;
            let result = work(&shared);
            let unheard = {
                let dropped = shared.dropped.lock().unwrap_or_else(PoisonError::into_inner);
                if *dropped {
                    Some(result)
                } else {
                    // The `Job` and its receiver live until the drop has taken the lock, so this arrives.
                    let _sent = send.send(result);
                    None
                }
            };
            if let Some(Ok(done)) = unheard {
                discard(&done.saved.path);
            }
        });
        if let Err(error) = started {
            log::warn!("couldn't start making a picture: {error}");
            wake();
        }
        Job { got, since: Instant::now(), limit, flags }
    }

    /// The result, once there is one; `None` while the picture is still being made.
    // Human: a picture that is being saved when the limit passes is waited for: the local save is short, and
    // telling the caller none was kept while its file and credit are being written would be untrue.
    // Agent: the work sets `saving`, then reads `given_up` (`Flags::may_save`); this sets `given_up`, then
    // reads `saving`; all `SeqCst`, so both can't miss the other. Either the work read `given_up` first, saw it
    // unset and saves: its `saving` came before that, so this sees it and waits for the result. Or this set
    // `given_up` first: the work sees it and keeps nothing; if this saw `saving` all the same and waited, the
    // work's `GIVEN_UP` answer is told as the time limit. 100 s and a short save stay under the 120 s the
    // local server waits (`link::PATIENCE`).
    // The whole rule: the time limit passing before the work's `may_save` keeps nothing; passing during the
    // save, it is waited for and the picture is handed back. A dropped `Job` keeps nothing whenever it is
    // dropped: before the save `may_save` says no, during it `keep` takes it back, after it the handover in
    // `start_with` or `Drop for Job` does. Not guaranteed: a picture whose save is cut off by Quill quitting,
    // or whose removal fails (logged); and the service may have charged for a picture that isn't kept.
    #[must_use]
    pub fn poll(&self) -> Option<Result<Done, String>> {
        match self.got.try_recv() {
            Ok(Err(why)) if why == GIVEN_UP => Some(Err(too_long(self.limit))),
            Ok(result) => Some(result),
            Err(TryRecvError::Disconnected) => Some(Err("Making the picture stopped unexpectedly.".to_owned())),
            Err(TryRecvError::Empty) if self.since.elapsed() > self.limit => {
                self.flags.given_up.store(true, Ordering::SeqCst);
                if self.flags.saving.load(Ordering::SeqCst) { None } else { Some(Err(too_long(self.limit))) }
            }
            Err(TryRecvError::Empty) => None,
        }
    }
}

impl Drop for Job {
    // Human: a job is dropped when nobody will take its picture any more (the dialog cancelled, the chat
    // stopped); a picture that is there already but was never taken is removed with its credit.
    // Agent: MAIN-THREAD; may WRITE `images/` (one file removed, the credits rewritten), which is short. After
    // the lock is let go the work sends nothing (`Job::start_with`), so what `try_recv` misses is never sent.
    fn drop(&mut self) {
        *self.flags.dropped.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.flags.given_up.store(true, Ordering::SeqCst);
        if let Ok(Ok(done)) = self.got.try_recv() {
            discard(&done.saved.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn done() -> Done {
        Done {
            saved: Saved { url: "./images/a.png".to_owned(), path: PathBuf::from("/d/images/a.png"), width: 4, height: 2 },
            shown: vec![1, 2, 3],
            service: "Studio".to_owned(),
            model: "m".to_owned(),
        }
    }

    fn wait(job: &Job) -> Result<Done, String> {
        for _ in 0..500 {
            if let Some(result) = job.poll() {
                return result;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the job never ended");
    }

    /// A flag a job's `wake` sets, and the `wake` that sets it.
    fn woken() -> (Arc<AtomicBool>, impl Fn() + Send + Sync + 'static) {
        let woken = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&woken);
        (woken, move || flag.store(true, Ordering::SeqCst))
    }

    /// Waits up to five seconds for `flag` to be set; whether it was.
    fn until(flag: &AtomicBool) -> bool {
        let since = Instant::now();
        while !flag.load(Ordering::SeqCst) && since.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(10));
        }
        flag.load(Ordering::SeqCst)
    }

    #[test]
    fn a_job_hands_back_what_its_work_made_and_wakes_the_app() {
        let (woken, wake) = woken();
        let job = Job::start(|_| Ok(done()), wake);
        assert_eq!(wait(&job), Ok(done()));
        assert!(until(&woken));
        let job = Job::start(|_| Err("Studio refused to make this picture: no.".to_owned()), || {});
        assert_eq!(wait(&job), Err("Studio refused to make this picture: no.".to_owned()));
    }

    #[test]
    fn a_job_that_takes_too_long_says_so_and_its_work_keeps_nothing() {
        let wrote = Arc::new(AtomicBool::new(false));
        let (woken, wake) = woken();
        let kept = Arc::clone(&wrote);
        // Human: the work stands for `work`: it waits past the limit, then "saves" unless it was given up on.
        let slow = Job::start_with(
            move |flags: &Flags| {
                std::thread::sleep(Duration::from_millis(800));
                if !flags.may_save() {
                    return Err(GIVEN_UP.to_owned());
                }
                kept.store(true, Ordering::SeqCst);
                Ok(done())
            },
            wake,
            Duration::from_millis(300),
        );
        assert_eq!(slow.poll(), None);
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(slow.poll(), Some(Err("The picture took too long; none was kept.".to_owned())));
        assert!(until(&woken), "the work ended");
        assert!(!wrote.load(Ordering::SeqCst), "nothing was saved after the job was given up on");
    }

    #[test]
    fn a_picture_being_saved_when_the_limit_passes_is_still_handed_back() {
        let (woken, wake) = woken();
        // Human: the work stands for `work`: it may save before the limit, and its save outlasts the limit.
        let saving = Job::start_with(
            |flags: &Flags| {
                std::thread::sleep(Duration::from_millis(100));
                if !flags.may_save() {
                    return Err(GIVEN_UP.to_owned());
                }
                std::thread::sleep(Duration::from_millis(600));
                Ok(done())
            },
            wake,
            Duration::from_millis(300),
        );
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(saving.poll(), None, "the limit passed while saving: the result is waited for");
        assert_eq!(wait(&saving), Ok(done()));
        assert!(until(&woken));
    }

    #[test]
    fn work_that_was_told_to_keep_nothing_is_told_as_the_time_limit() {
        let flags = Flags::default();
        assert!(flags.may_save());
        flags.given_up.store(true, Ordering::SeqCst);
        assert!(!flags.may_save());
        // Human: the work saw the limit pass while the job waited for its save: the caller hears the limit.
        let given_up = Job::start_with(|_| Err(GIVEN_UP.to_owned()), || {}, Duration::from_secs(5));
        assert_eq!(wait(&given_up), Err("The picture took too long (over 5 seconds); none was kept.".to_owned()));
    }

    #[test]
    fn a_job_dropped_tells_its_work_to_keep_nothing() {
        let saw = Arc::new(AtomicBool::new(false));
        let (woken, wake) = woken();
        let seen = Arc::clone(&saw);
        let job = Job::start(
            move |flags: &Flags| {
                seen.store(until(&flags.given_up), Ordering::SeqCst);
                Err(GIVEN_UP.to_owned())
            },
            wake,
        );
        drop(job);
        assert!(until(&woken));
        assert!(saw.load(Ordering::SeqCst));
    }

    /// A real picture saved in a folder of its own named after `name`, as `work` would save it.
    fn saved(name: &str) -> (PathBuf, Saved) {
        let folder = std::env::temp_dir().join(format!("pencraft-jobs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255])).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        let saved = store::save(Some(&folder.join("site.pen")), None, &bytes.into_inner(), "A fox", &json!({})).unwrap();
        (folder, saved)
    }

    #[test]
    fn a_save_is_taken_back_only_when_the_job_was_dropped_during_it() {
        let ok = || Ok(done().saved);
        // Human: the time limit passing during the save keeps the picture: its caller waits for it.
        let flags = Flags::default();
        let kept = keep(
            &flags,
            || {
                flags.given_up.store(true, Ordering::SeqCst);
                ok()
            },
            |_| panic!("nothing is taken back"),
        );
        assert_eq!(kept, Ok(done().saved));
        // Dropped during the save: taken back.
        let flags = Flags::default();
        let unsaved = AtomicBool::new(false);
        let dropped = keep(
            &flags,
            || {
                *flags.dropped.lock().unwrap() = true;
                ok()
            },
            |_| unsaved.store(true, Ordering::SeqCst),
        );
        assert_eq!(dropped, Err(GIVEN_UP.to_owned()));
        assert!(unsaved.load(Ordering::SeqCst));
        // Given up on before: no save at all.
        let flags = Flags::default();
        flags.given_up.store(true, Ordering::SeqCst);
        assert_eq!(keep(&flags, || panic!("nothing is saved"), |_| panic!("nothing is taken back")), Err(GIVEN_UP.to_owned()));
    }

    #[test]
    fn a_picture_handed_over_but_never_taken_goes_with_its_job() {
        let (folder, picture) = saved("untaken");
        let path = picture.path.clone();
        let (woken, wake) = woken();
        let job = Job::start(move |_| Ok(Done { saved: picture, ..done() }), wake);
        assert!(until(&woken), "the picture is in the channel");
        assert!(path.is_file());
        drop(job);
        assert!(!path.exists() && !folder.join("images").join(store::CREDITS).exists());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_picture_finished_after_its_job_was_dropped_is_taken_back() {
        let (folder, picture) = saved("late");
        let path = picture.path.clone();
        let (woken, wake) = woken();
        let job = Job::start(
            move |flags: &Flags| {
                // Human: stands for a save that ends just after the drop, past `keep`'s look.
                while !flags.dropped() {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(Done { saved: picture, ..done() })
            },
            wake,
        );
        drop(job);
        assert!(until(&woken));
        assert!(!path.exists() && !folder.join("images").join(store::CREDITS).exists());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_job_whose_work_dies_says_so_and_still_wakes_the_app() {
        let (woken, wake) = woken();
        let dead = Job::start(|_| panic!("boom"), wake);
        assert_eq!(wait(&dead), Err("Making the picture stopped unexpectedly.".to_owned()));
        assert!(until(&woken));
    }

    #[test]
    fn the_time_limit_is_said_in_whole_seconds_only_when_there_are_some() {
        assert_eq!(too_long(Duration::from_millis(300)), "The picture took too long; none was kept.");
        assert_eq!(too_long(Duration::from_millis(1500)), "The picture took too long (over 1 second); none was kept.");
        assert_eq!(too_long(LIMIT), "The picture took too long (over 100 seconds); none was kept.");
    }

    /// An order for the Gemini service from the environment, to change pictures or make one.
    fn gemini_order(request: Request, takes: Vec<Take>, document: Option<PathBuf>) -> Order {
        use crate::imagery::{KeyFrom, Kind};

        let kind = Kind::Gemini;
        Order {
            ready: Ready {
                name: "Gemini".to_owned(),
                kind,
                base_url: kind.base().to_owned(),
                model: kind.model().to_owned(),
                key: KeyFrom::Environment("GEMINI_API_KEY"),
            },
            request,
            document,
            takes,
        }
    }

    #[test]
    fn work_given_up_on_while_its_pictures_were_got_ready_asks_no_service() {
        use crate::imagery::service::Aspect;

        let request = Request {
            prompt: "x".to_owned(),
            model: "m".to_owned(),
            aspect: Some(Aspect(1, 1)),
            large: false,
            transparent: false,
            sources: Vec::new(),
        };
        let mut order = gemini_order(request, vec![Take::pixels(2, 2, vec![255; 16], true)], None);
        // Human: a key that isn't there: had the work gone on, it would have said so.
        order.ready.key = crate::imagery::KeyFrom::Environment("PENCRAFT_TEST_NO_SUCH_KEY");
        let flags = Flags::default();
        assert!(work(&order, &flags).is_err_and(|why| why.contains("PENCRAFT_TEST_NO_SUCH_KEY")), "not given up on: it goes on to the key");
        flags.given_up.store(true, Ordering::SeqCst);
        assert!(flags.given_up());
        assert_eq!(work(&order, &flags), Err(GIVEN_UP.to_owned()));
    }

    #[test]
    fn an_edit_is_credited_with_the_pictures_it_was_made_from() {
        use crate::imagery::service::Aspect;

        let request = Request {
            prompt: "Make it dusk".to_owned(),
            model: "m".to_owned(),
            aspect: Some(Aspect(1, 1)),
            large: false,
            transparent: false,
            sources: Vec::new(),
        };
        let order = gemini_order(request, Vec::new(), None);
        assert_eq!(credit(&order, &[]), json!({ "made": "generated", "service": "Gemini", "model": "m", "prompt": "Make it dusk" }));
        assert_eq!(
            credit(&order, &["photo.jpg", "drawn"]),
            json!({ "made": "edited", "service": "Gemini", "model": "m", "prompt": "Make it dusk", "from": ["photo.jpg", "drawn"] })
        );
    }

    /// Has the real Gemini service change a picture file made here and keeps the result beside a document in the
    /// temporary folder, the whole way a job goes. Run by hand, with a key that may make pictures:
    /// `GEMINI_API_KEY=… cargo test -p pencraft -- --ignored the_real_gemini_service_changes`.
    #[test]
    #[ignore = "calls the real service and costs money on the key of the person running it"]
    fn the_real_gemini_service_changes_a_picture() {
        use crate::imagery::Kind;

        let folder = std::env::temp_dir().join(format!("pencraft-live-gemini-edit-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_fn(512, 384, |x, y| {
            if (x / 64 + y / 64) % 2 == 0 { image::Rgba([230, 60, 40, 255]) } else { image::Rgba([250, 240, 220, 255]) }
        })
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
        std::fs::write(folder.join("squares.png"), bytes.into_inner()).unwrap();
        let take = Take::file(folder.join("squares.png"), true, None);
        let request = Request {
            prompt: "Make the whole picture blue.".to_owned(),
            model: Kind::Gemini.model().to_owned(),
            aspect: None,
            large: false,
            transparent: false,
            sources: Vec::new(),
        };
        let order = gemini_order(request, vec![take], Some(folder.join("live.pen")));
        let done = work(&order, &Flags::default()).unwrap();
        assert!(done.saved.path.is_file() && done.saved.width > done.saved.height, "{} x {}", done.saved.width, done.saved.height);
        let credits: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(folder.join("images").join(store::CREDITS)).unwrap()).unwrap();
        let name = done.saved.path.file_name().unwrap().to_str().unwrap();
        assert_eq!(credits[name]["made"], "edited");
        assert_eq!(credits[name]["from"], json!(["squares.png"]));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    /// Has the real Gemini service make a picture and keeps it beside a document in the temporary folder, the
    /// whole way a job goes. Run by hand, with a key that may make pictures:
    /// `GEMINI_API_KEY=… cargo test -p pencraft -- --ignored the_real_gemini`.
    #[test]
    #[ignore = "calls the real service and costs money on the key of the person running it"]
    fn the_real_gemini_service_makes_a_picture_that_is_kept_with_its_credit() {
        use crate::imagery::service::Aspect;
        use crate::imagery::{KeyFrom, Kind};

        let folder = std::env::temp_dir().join(format!("pencraft-live-gemini-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let kind = Kind::Gemini;
        let ready = Ready {
            name: "Gemini".to_owned(),
            kind,
            base_url: kind.base().to_owned(),
            model: kind.model().to_owned(),
            key: KeyFrom::Environment("GEMINI_API_KEY"),
        };
        let request = Request {
            prompt: "A red apple on a white table".to_owned(),
            model: kind.model().to_owned(),
            aspect: Some(Aspect(16, 9)),
            large: false,
            transparent: false,
            sources: Vec::new(),
        };
        let order = Order::new(ready, request, Some(folder.join("live.pen")));
        let done = work(&order, &Flags::default()).unwrap();
        assert!(done.saved.url.starts_with("./images/a-red-apple-on-a-white-table-"), "{}", done.saved.url);
        assert!(done.saved.path.is_file() && done.saved.width > done.saved.height, "{} x {}", done.saved.width, done.saved.height);
        assert_eq!(image::guess_format(&done.shown).unwrap(), image::ImageFormat::Png);
        let credits: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(folder.join("images").join(store::CREDITS)).unwrap()).unwrap();
        let name = done.saved.path.file_name().unwrap().to_str().unwrap();
        assert_eq!(
            credits[name],
            json!({ "made": "generated", "service": "Gemini", "model": kind.model(), "prompt": "A red apple on a white table" })
        );
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
