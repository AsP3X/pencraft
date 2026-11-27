//! Whether the computer asks for less motion, asked once when the app starts: macOS (System Settings ›
//! Accessibility › Display › Reduce motion), Windows (Settings › Accessibility › Visual effects › Animation
//! effects) and Linux (GNOME's `enable-animations`).
//!
// Human: the app has its own "Animations and effects" switch. Until the user has set it, it follows the
// computer: someone who told the system to keep things still should not be met by a film with music on first
// launch, before any switch can be reached.
// Human: neither egui, winit nor accesskit (the versions this app builds with) say what the system asks for.
// Each system has a program of its own that prints the setting, which needs no unsafe code and no new
// library: `defaults` on macOS, `reg query` on Windows, `gsettings` on Linux.
// Human: on Windows the switch is what `SPI_GETCLIENTAREAANIMATION` reports. Microsoft documents that
// call, not where its answer is kept; the place read here (the second bit of the fifth byte of
// `UserPreferencesMask` under `HKCU\Control Panel\Desktop`, set while things move) is the one Wine's
// win32u uses and the one seen to change when the switch is turned. It is the copy written at the last
// change, which is what a program that starts later finds.
// Human: an answer that can't be had (no `gsettings`, a desktop without that key, a value that isn't
// there) is "unknown", never "reduced": the app's own switch then decides alone.
// Agent: SPAWNS one short-lived thread, which RUNS the system's program (twice on macOS); nothing is
// written. The answer is taken on the main thread with `System::answer`; an unknown one is never sent.

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
#[cfg(any(target_os = "macos", test))]
fn on(printed: &str) -> bool {
    matches!(printed.trim(), "1" | "true" | "YES")
}

/// Whether what `reg query "HKCU\Control Panel\Desktop" /v UserPreferencesMask` printed says Windows asks
/// for less motion; `None` when it doesn't say.
// Human: the line reads `    UserPreferencesMask    REG_BINARY    9E1E078012000000`: the bytes in order, two
// letters each. The fifth byte's second bit is set while "Animation effects" is on. A value too short to
// have that byte (an old profile) says nothing.
#[cfg(any(windows, test))]
fn windows_reduced(printed: &str) -> Option<bool> {
    /// Where the byte with the switch starts among the letters, and the switch's bit in it.
    const BYTE: std::ops::Range<usize> = 8..10;
    const MOVING: u8 = 0x02;
    let line = printed.lines().find(|line| line.contains("UserPreferencesMask") && line.contains("REG_BINARY"))?;
    let bytes = line.split_whitespace().last()?;
    let byte = u8::from_str_radix(bytes.get(BYTE)?, 16).ok()?;
    Some(byte & MOVING == 0)
}

/// Whether what `gsettings get org.gnome.desktop.interface enable-animations` printed says the desktop asks
/// for less motion; `None` when it is neither `true` nor `false`.
#[cfg(any(target_os = "linux", test))]
fn gnome_reduced(printed: &str) -> Option<bool> {
    match printed.trim() {
        "true" => Some(false),
        "false" => Some(true),
        _ => None,
    }
}

/// What `program` prints when run with `arguments`, and whether it says it worked; `None` when it can't be run.
// Agent: RUNS a program and waits for it; call it on a thread of its own.
#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
fn printed(program: &str, arguments: &[&str]) -> Option<(String, bool)> {
    let mut command = std::process::Command::new(program);
    command.args(arguments).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    // Human: without this a console window flashes up for the moment the program runs.
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut command, crate::link::CREATE_NO_WINDOW);
    let done = command.output().ok()?;
    Some((String::from_utf8_lossy(&done.stdout).into_owned(), done.status.success()))
}

/// Asks macOS; `None` only when `defaults` can't be run at all.
#[cfg(target_os = "macos")]
fn reduced() -> Option<bool> {
    /// Where macOS keeps the switch, by version: the older place, then the newer one.
    const PLACES: [(&str, &str); 2] = [("com.apple.universalaccess", "reduceMotion"), ("com.apple.Accessibility", "ReduceMotionEnabled")];
    let mut any = false;
    for (domain, key) in PLACES {
        // A key that was never set is not there: `defaults` then fails, which means "off".
        let (text, worked) = printed("/usr/bin/defaults", &["read", domain, key])?;
        any |= worked && on(&text);
    }
    Some(any)
}

/// Asks Windows; a value that isn't there gives no answer.
#[cfg(windows)]
fn reduced() -> Option<bool> {
    let (text, worked) = printed("reg", &["query", r"HKCU\Control Panel\Desktop", "/v", "UserPreferencesMask"])?;
    worked.then(|| windows_reduced(&text)).flatten()
}

/// Asks a GNOME desktop; another desktop has no `gsettings` or not this key, and gives no answer.
#[cfg(target_os = "linux")]
fn reduced() -> Option<bool> {
    let (text, worked) = printed("gsettings", &["get", "org.gnome.desktop.interface", "enable-animations"])?;
    worked.then(|| gnome_reduced(&text)).flatten()
}

/// Starts the thread that asks the system; `None` when it can't be started.
#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
fn asking(wake: &egui::Context) -> Option<mpsc::Receiver<bool>> {
    let (send, arrives) = mpsc::channel();
    let wake = wake.clone();
    let work = move || {
        // Human: with no answer nothing is sent, and the app sees that the question is over when this
        // thread ends. The app may also have gone on without the answer; then nobody takes it, which is fine.
        if let Some(reduced) = reduced() {
            let _unheard = send.send(reduced);
        }
        drop(send);
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

/// On any other system there is nothing to ask.
#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
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
    fn windows_switch_is_the_second_bit_of_the_fifth_byte_reg_prints() {
        let told = |bytes: &str| format!("\r\nHKEY_CURRENT_USER\\Control Panel\\Desktop\r\n    UserPreferencesMask    REG_BINARY    {bytes}\r\n\r\n");
        // "Animation effects" on (things move), then off, as Windows 11 writes them.
        assert_eq!(windows_reduced(&told("9E1E078012000000")), Some(false));
        assert_eq!(windows_reduced(&told("9012078010000000")), Some(true));
        assert_eq!(windows_reduced(&told("9e1e078002")), Some(false));
        // Too short to hold the switch, not bytes, no such line, or nothing at all: unknown.
        assert_eq!(windows_reduced(&told("9E1E0780")), None);
        assert_eq!(windows_reduced(&told("9E1E0780ZZ000000")), None);
        assert_eq!(windows_reduced("    MenuShowDelay    REG_SZ    400\r\n"), None);
        assert_eq!(windows_reduced("ERROR: The system was unable to find the specified registry key or value."), None);
        assert_eq!(windows_reduced(""), None);
    }

    #[test]
    fn gnomes_switch_is_read_from_what_gsettings_prints_and_anything_else_is_unknown() {
        assert_eq!(gnome_reduced("true\n"), Some(false));
        assert_eq!(gnome_reduced("false\n"), Some(true));
        assert_eq!(gnome_reduced(""), None);
        assert_eq!(gnome_reduced("No such schema \u{201c}org.gnome.desktop.interface\u{201d}"), None);
        assert_eq!(gnome_reduced("@b true"), None);
    }

    #[test]
    fn an_answer_the_system_never_gives_is_not_waited_for() {
        let (send, arrives) = mpsc::channel::<bool>();
        let mut system = System { asked: Instant::now(), arrives: Some(arrives), settled: false };
        drop(send);
        assert!(system.answer().is_none() && system.settled && !system.pending());
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
