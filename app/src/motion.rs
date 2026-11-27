//! Whether the computer asks for less motion (macOS: System Settings › Accessibility › Display › Reduce
//! motion), asked once when the app starts.
//!
// Human: the app has its own "Animations and effects" switch. Until the user has set it, it follows the
// computer: someone who told the system to keep things still should not be met by a film with music on first
// launch, before any switch can be reached.
// Human: neither egui, winit nor accesskit (the versions this app builds with) say what the system asks for.
// On macOS the setting can be read with the system's own `defaults` program, which needs no unsafe code and
// no new library. On Windows and Linux there is no way that clean, so the system is not asked there and the
// app's own switch decides alone.
// Agent: SPAWNS one short-lived thread (macOS only), which RUNS `/usr/bin/defaults read` twice; nothing is
// written. The answer is taken on the main thread with `System::answer`.

use std::sync::mpsc;
use std::time::{Duration, Instant};

/// How long the app waits for the answer before it goes on as if the system asked for nothing.
const PATIENCE: Duration = Duration::from_millis(600);

/// The question to the system, and its answer once it is there.
pub struct System {
    asked: Instant,
    arrives: Option<mpsc::Receiver<bool>>,
    /// The answer has been taken ([`System::answer`]) or given up on.
    settled: bool,
}

impl System {
    /// Asks the system, on a thread of its own where there is a way to ask; `wake` is told when the answer is in.
    #[must_use]
    pub fn ask(wake: &egui::Context) -> System {
        System { asked: Instant::now(), arrives: asking(wake), settled: false }
    }

    /// A question nobody was asked: there is no answer to wait for (a picture of the app, a test).
    #[must_use]
    pub fn unasked() -> System {
        System { asked: Instant::now(), arrives: None, settled: true }
    }

    /// Whether the answer is still on its way, so what would move should wait a moment.
    #[must_use]
    pub fn pending(&self) -> bool {
        !self.settled && self.arrives.is_some() && self.asked.elapsed() < PATIENCE
    }

    /// The answer, the one time it arrives: whether the system asks for less motion. `None` before that,
    /// after it was given once, and where the system can't be asked.
    pub fn answer(&mut self) -> Option<bool> {
        if self.settled {
            return None;
        }
        let Some(arrives) = &self.arrives else {
            self.settled = true;
            return None;
        };
        match arrives.try_recv() {
            Ok(reduced) => {
                self.settled = true;
                Some(reduced)
            }
            Err(mpsc::TryRecvError::Empty) if self.asked.elapsed() < PATIENCE => None,
            // Too late or never: the app's own switch decides.
            Err(_) => {
                self.settled = true;
                None
            }
        }
    }
}

/// Whether what `defaults read` printed for a switch says the switch is on.
fn on(printed: &str) -> bool {
    matches!(printed.trim(), "1" | "true" | "YES")
}

/// Starts the thread that asks macOS; `None` when it can't be started.
#[cfg(target_os = "macos")]
fn asking(wake: &egui::Context) -> Option<mpsc::Receiver<bool>> {
    /// Where macOS keeps the switch, by version: the older place, then the newer one.
    const PLACES: [(&str, &str); 2] = [("com.apple.universalaccess", "reduceMotion"), ("com.apple.Accessibility", "ReduceMotionEnabled")];
    let (send, arrives) = mpsc::channel();
    let wake = wake.clone();
    let work = move || {
        let read = |(domain, key): &(&str, &str)| {
            // A key that was never set is not there: `defaults` then fails, which means "off".
            std::process::Command::new("/usr/bin/defaults")
                .args(["read", domain, key])
                .stdin(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .output()
                .is_ok_and(|done| done.status.success() && on(&String::from_utf8_lossy(&done.stdout)))
        };
        // The app may have gone on without the answer; then nobody takes it, which is fine.
        let _unheard = send.send(PLACES.iter().any(read));
        wake.request_repaint();
    };
    match std::thread::Builder::new().name("pencraft-motion".to_owned()).spawn(work) {
        Ok(_) => Some(arrives),
        Err(error) => {
            log::warn!("couldn't ask the system whether it wants less motion: {error}");
            None
        }
    }
}

/// On Windows and Linux the system isn't asked (see the top of this file).
#[cfg(not(target_os = "macos"))]
fn asking(_wake: &egui::Context) -> Option<mpsc::Receiver<bool>> {
    None
}

/// The "Animations and effects" switch as the settings have it.
#[derive(Clone, Copy, Debug)]
pub struct Switch {
    /// Things move.
    pub on: bool,
    /// The user has set it themselves.
    pub chosen: bool,
    /// It is off because the system asked for less motion when Quill last started.
    pub off_for_system: bool,
}

/// What "Animations and effects" is to be changed to, given whether the system asks for `reduced` motion:
/// off when it does, and on again when it no longer does and Quill had turned it off for that. `None` when
/// it stays as it is.
// Human: what the user set themselves stands. So does an "off" that was there before Quill followed the
// system (a settings file of an earlier version): nobody asked for that to be turned on again.
#[must_use]
pub fn follow(now: Switch, reduced: bool) -> Option<bool> {
    match (now.chosen, reduced, now.on) {
        (false, true, true) => Some(false),
        (false, false, false) if now.off_for_system => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_systems_switch_is_read_from_what_defaults_prints() {
        assert!(on("1\n") && on("true") && on(" YES "));
        assert!(!on("0\n") && !on("") && !on("The domain/default pair of (x, y) does not exist"));
    }

    #[test]
    fn the_apps_switch_follows_the_system_until_the_user_sets_it() {
        let switch = |on: bool, chosen: bool, off_for_system: bool| Switch { on, chosen, off_for_system };
        // The system asks for less motion: off. It stops asking: on again, since Quill turned it off.
        assert_eq!(follow(switch(true, false, false), true), Some(false));
        assert_eq!(follow(switch(false, false, true), true), None);
        assert_eq!(follow(switch(false, false, true), false), Some(true));
        assert_eq!(follow(switch(true, false, false), false), None);
        // An "off" from an earlier version's settings file stays off, whatever the system says.
        assert_eq!(follow(switch(false, false, false), false), None);
        assert_eq!(follow(switch(false, false, false), true), None);
        // What the user set themselves stands, on or off.
        for on in [true, false] {
            for reduced in [true, false] {
                assert_eq!(follow(switch(on, true, false), reduced), None);
            }
        }
    }

    #[test]
    fn the_answer_is_given_once_and_a_question_nobody_was_asked_is_not_waited_for() {
        let mut none = System::unasked();
        assert!(!none.pending() && none.answer().is_none());
        let (send, arrives) = mpsc::channel();
        let mut system = System { asked: Instant::now(), arrives: Some(arrives), settled: false };
        assert!(system.pending() && system.answer().is_none() && system.pending());
        send.send(true).unwrap();
        assert_eq!(system.answer(), Some(true));
        assert!(!system.pending() && system.answer().is_none());
        // An answer that doesn't come in time is not waited for any longer.
        let (_send, arrives) = mpsc::channel::<bool>();
        let long_ago = Instant::now().checked_sub(PATIENCE * 2).unwrap();
        let mut late = System { asked: long_ago, arrives: Some(arrives), settled: false };
        assert!(!late.pending() && late.answer().is_none() && late.settled);
    }
}
