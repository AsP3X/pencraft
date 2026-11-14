//! How keyboard shortcuts are written for the person: the Mac's signs on a Mac, key names elsewhere.
//!
//! Shortcuts are written once in the code, the Mac way (`⇧⌘S`). On a Mac they are shown as they are; on
//! Windows and Linux the signs become the names on those keyboards (`Ctrl+Shift+S`). The keys themselves
//! already agree: egui's `command` modifier is ⌘ on a Mac and Ctrl elsewhere.

use std::borrow::Cow;

/// Whether shortcuts are shown with the Mac's signs.
const MAC: bool = cfg!(target_os = "macos");

/// A modifier key's place in a shortcut off the Mac (Ctrl, Alt, Shift) and its name there; `None` for any
/// other sign.
// Human: ⌘ and ⌃ are both Ctrl there, as egui's `command` is.
fn modifier(sign: char) -> Option<(usize, &'static str)> {
    match sign {
        '⌘' | '⌃' => Some((0, "Ctrl")),
        '⌥' => Some((1, "Alt")),
        '⇧' => Some((2, "Shift")),
        _ => None,
    }
}

/// The name off the Mac of a key the Mac shows as a sign; `None` for a key written the same everywhere.
fn named(sign: char) -> Option<&'static str> {
    match sign {
        '⌫' => Some("Del"),
        '↵' => Some("Enter"),
        _ => None,
    }
}

/// `text` with the shortcuts in it as this system writes them: unchanged on a Mac, and elsewhere
/// `Save (⌘S)` reads `Save (Ctrl+S)`.
#[must_use]
pub fn text(text: &str) -> Cow<'_, str> {
    if MAC { Cow::Borrowed(text) } else { Cow::Owned(spelled(text)) }
}

/// `text` with every Mac key sign replaced by the key's name, modifiers in the order Ctrl, Alt, Shift.
fn spelled(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    let mut held = [false; 3];
    for sign in text.chars() {
        if let Some((place, _)) = modifier(sign) {
            held[place] = true;
            continue;
        }
        for (place, name) in ["Ctrl", "Alt", "Shift"].into_iter().enumerate() {
            if std::mem::take(&mut held[place]) {
                out.push_str(name);
                out.push('+');
            }
        }
        match named(sign) {
            Some(name) => out.push_str(name),
            None => out.push(sign),
        }
    }
    out
}

/// A shortcut's keys, one for each key cap, as this system writes them: unchanged on a Mac, and elsewhere
/// `⌥ ⌘ G` is `Ctrl Alt G`.
#[must_use]
pub fn caps(keys: &[&'static str]) -> Vec<&'static str> {
    if MAC { keys.to_vec() } else { named_caps(keys) }
}

/// The caps of `keys` by name: the modifiers first, in the order Ctrl, Alt, Shift.
fn named_caps(keys: &[&'static str]) -> Vec<&'static str> {
    let sign = |key: &str| {
        let mut signs = key.chars();
        signs.next().filter(|_| signs.next().is_none())
    };
    let mut modifiers: Vec<(usize, &'static str)> = keys.iter().filter_map(|key| sign(key).and_then(modifier)).collect();
    modifiers.sort_unstable();
    modifiers.dedup();
    let others = keys.iter().filter(|key| sign(key).and_then(modifier).is_none()).map(|key| sign(key).and_then(named).unwrap_or(key));
    modifiers.into_iter().map(|(_, name)| name).chain(others).collect()
}

/// A menu line written the Mac way, `Save     ⌘S`: the command's name, some room, the shortcut. Split into
/// the name and the shortcut; a line without a shortcut is all name.
#[must_use]
pub fn menu_line(line: &str) -> (&str, &str) {
    line.split_once("  ").map_or((line, ""), |(name, keys)| (name, keys.trim_start()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_become_names_in_the_usual_order() {
        assert_eq!(spelled("Home (⌘N)"), "Home (Ctrl+N)");
        assert_eq!(spelled("⇧⌘S"), "Ctrl+Shift+S");
        assert_eq!(spelled("Frame selection (⌥⌘G)"), "Frame selection (Ctrl+Alt+G)");
        assert_eq!(spelled("⌃⇧PageUp"), "Ctrl+Shift+PageUp");
        assert_eq!(spelled("Flip vertical (⇧V)"), "Flip vertical (Shift+V)");
        assert_eq!(spelled("⌫"), "Del");
        assert_eq!(spelled("⌘\\"), "Ctrl+\\");
        assert_eq!(spelled("No shortcut here…"), "No shortcut here…");
    }

    #[test]
    fn caps_are_named_with_the_modifiers_first() {
        assert_eq!(named_caps(&["⌥", "⌘", "G"]), ["Ctrl", "Alt", "G"]);
        assert_eq!(named_caps(&["⌘", "⇧", "S"]), ["Ctrl", "Shift", "S"]);
        assert_eq!(named_caps(&["⇧", "↵"]), ["Shift", "Enter"]);
        assert_eq!(named_caps(&["⌫"]), ["Del"]);
    }

    #[test]
    fn a_menu_line_splits_into_name_and_shortcut() {
        assert_eq!(menu_line("Save                 ⌘S"), ("Save", "⌘S"));
        assert_eq!(menu_line("Add flex layout  ⇧A"), ("Add flex layout", "⇧A"));
        assert_eq!(menu_line("Open…"), ("Open…", ""));
    }
}
