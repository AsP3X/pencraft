//! Layers pasted into the message box: copied on the canvas and pasted into the text, they become a small chip
//! that stands for them, so the user can point at layers in the middle of a sentence.
//!
//! In the text a chip is a short run of characters (its token) that no one types by accident; the message box
//! draws it as a chip. Sent to the assistant, the token becomes the layers' ids and names.

use crate::editor::Editor;

/// The space before a chip's name, where its icon is drawn: this many no-break spaces.
const ICON_ROOM: usize = 5;
/// A space that doesn't break a line, so a chip never wraps in its middle.
const HARD_SPACE: char = '\u{a0}';
/// The most characters of a layer's name a chip shows.
const MOST_NAME: usize = 18;

/// The most chips with the same name one message can hold.
const MOST_ALIKE: usize = 99;

/// One pasted reference to layers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip {
    /// What stands for it in the text.
    pub token: String,
    /// What it shows: the layer's name, or "3 layers".
    pub label: String,
    /// The icon in front: the layer's kind, or a stack for several.
    pub icon: &'static str,
    /// The layers it stands for: id and name.
    pub layers: Vec<(String, String)>,
}

impl Chip {
    /// What the assistant reads in the token's place: each layer's id with its name.
    #[must_use]
    pub fn told(&self) -> String {
        self.layers.iter().map(|(id, name)| format!("{id} ({})", serde_json::Value::String(name.clone()))).collect::<Vec<_>>().join(", ")
    }

    /// The names of its layers, one a line, for the hint under the pointer.
    #[must_use]
    pub fn names(&self) -> String {
        self.layers.iter().map(|(_, name)| name.as_str()).collect::<Vec<_>>().join("\n")
    }
}

/// A chip for the layers with the ids `copied` that are in `editor`'s document, or `None` when none of them
/// is. `text` is what the message box holds: the chip's token is made different from every one in it.
// Human: only layers of the document in front can be pointed at; layers copied from another document have
// nothing here that the assistant could work on.
#[must_use]
pub fn chip(editor: &Editor, copied: &[String], text: &str) -> Option<Chip> {
    let found: Vec<(&str, String, &'static str)> = copied
        .iter()
        .filter_map(|id| editor.scene_node(id).map(|scene| (id.as_str(), scene.node.display_name(), crate::panels::kind_icon(&scene.node.kind))))
        .collect();
    let (label, icon) = match found.as_slice() {
        [] => return None,
        [(_, name, icon)] => (name.clone(), *icon),
        many => (format!("{} layers", many.len()), "layers"),
    };
    let short: String = if label.chars().count() > MOST_NAME { label.chars().take(MOST_NAME - 1).chain(['…']).collect() } else { label };
    let token_for = |shown: &str| {
        let name: String = shown.chars().map(|c| if c.is_whitespace() { HARD_SPACE } else { c }).collect();
        format!("{}{name}{HARD_SPACE}", HARD_SPACE.to_string().repeat(ICON_ROOM))
    };
    // The same layer pasted twice gets "Card", then "Card 2", so each chip is found again in the text.
    let (label, token) = (1..=MOST_ALIKE)
        .map(|n| if n == 1 { short.clone() } else { format!("{short} {n}") })
        .map(|label| (token_for(&label), label))
        .find(|(token, _)| !text.contains(token))
        .map(|(token, label)| (label, token))?;
    Some(Chip { token, label, icon, layers: found.into_iter().map(|(id, name, _)| (id.to_owned(), name)).collect() })
}

/// Where the chips are in `text`: each one's place (in bytes) and which chip it is, from left to right.
#[must_use]
pub fn places(text: &str, chips: &[Chip]) -> Vec<(std::ops::Range<usize>, usize)> {
    let mut out: Vec<(std::ops::Range<usize>, usize)> =
        chips.iter().enumerate().filter_map(|(index, chip)| text.find(&chip.token).map(|at| (at..at + chip.token.len(), index))).collect();
    out.sort_by_key(|(place, _)| place.start);
    out
}

/// `text` with every chip's token replaced by what `shown` gives for the chip.
#[must_use]
pub fn written(text: &str, chips: &[Chip], shown: impl Fn(&Chip) -> String) -> String {
    let mut out = String::new();
    let mut from = 0;
    for (place, index) in places(text, chips) {
        out.push_str(&text[from..place.start]);
        out.push_str(&shown(&chips[index]));
        from = place.end;
    }
    out.push_str(&text[from..]);
    out.trim().to_owned()
}

/// Keeps the chips whole after the user edited the text from `before` to `text`: a chip the edit cut into is
/// taken out altogether, and chips that are gone from the text are forgotten. Returns where the text cursor
/// belongs (in characters) when a chip was taken out.
// Human: a chip is one thing: a backspace behind it removes all of it, not its last letter.
pub fn settle(before: &str, text: &mut String, chips: &mut Vec<Chip>) -> Option<usize> {
    let mut cursor = None;
    if let Some(chip) = chips.iter().find(|chip| !text.contains(&chip.token))
        && let Some(at) = before.find(&chip.token)
    {
        let end = at + chip.token.len();
        // The part of `before` the edit changed lies between what the two texts share at their start and end.
        let shortest = before.len().min(text.len());
        let start_same = before.bytes().zip(text.bytes()).take_while(|(a, b)| a == b).count();
        let end_same = before.bytes().rev().zip(text.bytes().rev()).take_while(|(a, b)| a == b).count().min(shortest - start_same);
        let changed = start_same..before.len() - end_same;
        let whole = changed.start <= at && changed.end >= end;
        if !whole {
            *text = format!("{}{}", &before[..at], &before[end..]);
            cursor = Some(before[..at].chars().count());
        }
    }
    chips.retain(|chip| text.contains(&chip.token));
    cursor
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> Editor {
        let doc = pen_doc::read(
            r#"{"version":"2.22","children":[
              {"type":"frame","id":"card","name":"Card","x":0,"y":0,"width":100,"height":80,"children":[
                {"type":"text","id":"title","name":"A very long title for a layer","content":"Hi"}]},
              {"type":"ellipse","id":"dot","x":200,"y":0,"width":10,"height":10}]}"#,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 1);
        editor.refresh(&|_| (10.0, 10.0));
        editor
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|id| (*id).to_owned()).collect()
    }

    #[test]
    fn pasted_layers_become_a_chip_and_the_assistant_reads_their_ids() {
        let editor = editor();
        let one = chip(&editor, &ids(&["card"]), "").unwrap();
        assert_eq!((one.label.as_str(), one.icon, one.told().as_str()), ("Card", "frame", "card (\"Card\")"));
        assert!(one.token.ends_with("Card\u{a0}") && one.token.starts_with('\u{a0}') && !one.token.contains(' '));
        // Several layers are one chip; ones that aren't in the document are left out, and none at all is no chip.
        let many = chip(&editor, &ids(&["card", "gone", "dot"]), "").unwrap();
        assert_eq!((many.label.as_str(), many.icon, many.layers.len()), ("2 layers", "layers", 2));
        assert_eq!(many.names(), "Card\nEllipse");
        assert!(chip(&editor, &ids(&["gone"]), "").is_none());
        // A long name is cut, and the same layer pasted again gets a token of its own.
        assert_eq!(chip(&editor, &ids(&["title"]), "").unwrap().label, "A very long title…");
        let text = format!("Make {} wider", one.token);
        let again = chip(&editor, &ids(&["card"]), &text).unwrap();
        assert_eq!(again.label, "Card 2");
        let text = format!("{text} like {}", again.token);
        let chips = vec![one, again];
        assert_eq!(written(&text, &chips, Chip::told), "Make card (\"Card\") wider like card (\"Card\")");
        assert_eq!(written(&text, &chips, |chip| chip.label.clone()), "Make Card wider like Card 2");
    }

    #[test]
    fn a_chip_is_removed_whole() {
        let editor = editor();
        let one = chip(&editor, &ids(&["card"]), "").unwrap();
        let before = format!("Make {} wider", one.token);
        // A backspace behind the chip takes its last character; all of it goes, and the cursor to its place.
        let mut text = before.replacen("Card\u{a0}", "Card", 1);
        let mut chips = vec![one.clone()];
        assert_eq!(settle(&before, &mut text, &mut chips), Some(5));
        assert_eq!((text.as_str(), chips.len()), ("Make  wider", 0));
        // Typing elsewhere leaves it alone.
        let mut text = format!("{before}!");
        let mut chips = vec![one.clone()];
        assert_eq!(settle(&before, &mut text, &mut chips), None);
        assert_eq!((text, chips.len()), (format!("{before}!"), 1));
        // Replacing a stretch that holds the whole chip keeps what was typed.
        let mut text = "Make it wider".to_owned();
        let mut chips = vec![one];
        assert_eq!(settle(&before, &mut text, &mut chips), None);
        assert_eq!((text.as_str(), chips.len()), ("Make it wider", 0));
    }
}
