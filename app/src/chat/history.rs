//! What was said in a design's chat, kept from one run to the next.
// Human: a design is worked on over days; the conversation about it belongs to it. What the user and the
// assistant said is kept under `~/.pencraft/chats`, in a file named after the design's own file, and is back in
// the chat panel when the design is opened again. The assistant is told the last of it again, as any engine is
// that wasn't there (`Chat::retold`). The design's own file is not touched.
// Agent: READS and WRITES small files under `~/.pencraft/chats`; fast enough for the window's thread.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::{Entry, Pointed, Role};

/// The most lines kept of one conversation: the newest.
pub const MOST: usize = 200;

/// The folder the conversations are kept in.
// Human: the tests keep out of the home folder.
fn root() -> PathBuf {
    if cfg!(test) {
        return std::env::temp_dir().join(format!("pencraft-chats-test-{}", std::process::id()));
    }
    crate::imagery::store::home().map_or_else(|| std::env::temp_dir().join("pencraft-chats"), |home| home.join(".pencraft").join("chats"))
}

/// The file the conversation about the design at `design` is kept in: named by a number worked out from the
/// design's path (FNV-1a, 64 bits), so every design has one of its own wherever it lies.
fn file_in(root: &Path, design: &Path) -> PathBuf {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in design.to_string_lossy().bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    root.join(format!("{hash:016x}.json"))
}

/// The lines of `entries` that are kept: what the user and the assistant said, the newest [`MOST`].
fn kept(entries: &[Entry]) -> Vec<Value> {
    let said: Vec<Value> = entries
        .iter()
        .filter_map(|entry| match entry.role {
            Role::User => {
                let mut line = json!({ "role": "user", "text": entry.text });
                let pictures = pictures_of(entry);
                if !pictures.is_empty() {
                    line["pictures"] = Value::Array(pictures);
                }
                Some(line)
            }
            Role::Assistant => Some(json!({ "role": "assistant", "text": entry.text })),
            Role::Step | Role::Problem => None,
        })
        .collect();
    let from = said.len().saturating_sub(MOST);
    said[from..].to_vec()
}

/// The pictures pasted into the user's message `entry`, as they are kept: where each one's name stands in the
/// text, the name, and its file.
// Human: a message with a picture in it still shows the picture when its design is opened another day.
fn pictures_of(entry: &Entry) -> Vec<Value> {
    entry
        .pointed
        .iter()
        .filter_map(|pointed| match (&pointed.at, &pointed.picture) {
            (Some(at), Some(file)) => Some(json!({ "from": at.start, "to": at.end, "name": pointed.name, "file": file.display().to_string() })),
            _ => None,
        })
        .collect()
}

/// The pictures kept with a line of the user's whose words are `text`; one whose place isn't in the words is
/// left out.
fn pictures_in(line: &Value, text: &str) -> Vec<Pointed> {
    let place = |picture: &Value, key: &str| picture.get(key).and_then(Value::as_u64).and_then(|at| usize::try_from(at).ok());
    line.get("pictures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|picture| {
            let at = place(picture, "from")?..place(picture, "to")?;
            let name = picture.get("name").and_then(Value::as_str)?;
            let file = PathBuf::from(picture.get("file").and_then(Value::as_str)?);
            (text.get(at.clone()) == Some(name)).then(|| Pointed { at: Some(at), icon: "image", name: name.to_owned(), picture: Some(file) })
        })
        .collect()
}

/// Keeps the conversation `entries` for the design at `design`; one in which nothing was said keeps no file.
///
/// # Errors
/// A sentence when the file can't be written.
pub fn keep(design: &Path, entries: &[Entry]) -> Result<(), String> {
    keep_in(&root(), design, entries)
}

fn keep_in(root: &Path, design: &Path, entries: &[Entry]) -> Result<(), String> {
    let (file, said) = (file_in(root, design), kept(entries));
    if said.is_empty() {
        return match std::fs::remove_file(&file) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {error}", file.display())),
            _ => Ok(()),
        };
    }
    std::fs::create_dir_all(root).map_err(|error| format!("{}: {error}", root.display()))?;
    let text = json!({ "design": design.display().to_string(), "said": said }).to_string();
    craft_atomic::atomic_write(&file, text.as_bytes()).map_err(|error| format!("{}: {error}", file.display()))
}

/// The conversation kept for the design at `design`, oldest line first; empty when there is none, or its file
/// can't be read.
#[must_use]
pub fn read(design: &Path) -> Vec<Entry> {
    read_in(&root(), design)
}

fn read_in(root: &Path, design: &Path) -> Vec<Entry> {
    let Some(kept) = std::fs::read(file_in(root, design)).ok().and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok()) else {
        return Vec::new();
    };
    kept.get("said")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|line| {
            let role = match line.get("role").and_then(Value::as_str)? {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                _ => return None,
            };
            let text = line.get("text").and_then(Value::as_str)?;
            Some(Entry { pointed: pictures_in(line, text), ..Entry::new(role, text) })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_was_said_comes_back_for_its_design_and_no_other() {
        let root = root().join("said");
        drop(std::fs::remove_dir_all(&root));
        let (shop, blog) = (Path::new("/designs/shop.pen"), Path::new("/designs/blog.pen"));
        let said = [
            Entry::new(Role::User, "Make the header taller"),
            Entry::new(Role::Step, "Changed 2 layers"),
            Entry::new(Role::Assistant, "The header is 96 high now."),
            Entry::new(Role::Problem, "The provider didn't answer."),
        ];
        keep_in(&root, shop, &said).unwrap();
        // The words come back in order; notes on tools and problems are not kept.
        let back = read_in(&root, shop);
        assert_eq!(back, vec![said[0].clone(), said[2].clone()]);
        assert_eq!(read_in(&root, blog), Vec::new());
        // A conversation started over leaves no file.
        keep_in(&root, shop, &[]).unwrap();
        assert_eq!(read_in(&root, shop), Vec::new());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
        keep_in(&root, blog, &[]).unwrap();
    }

    #[test]
    fn a_pasted_picture_comes_back_in_its_place() {
        let root = root().join("pictures");
        drop(std::fs::remove_dir_all(&root));
        let design = Path::new("/designs/shop.pen");
        let picture = Pointed { at: Some(5..12), icon: "image", name: "Image 1".to_owned(), picture: Some(PathBuf::from("/p/shot.png")) };
        // A layer that was pointed at is words again another day; the picture is still a picture.
        let layer = Pointed { at: Some(17..21), icon: "frame", name: "Card".to_owned(), picture: None };
        let said = [Entry { pointed: vec![picture.clone(), layer], ..Entry::new(Role::User, "Like Image 1 for Card") }];
        keep_in(&root, design, &said).unwrap();
        let back = read_in(&root, design);
        assert_eq!((back[0].text.as_str(), &back[0].pointed), ("Like Image 1 for Card", &vec![picture]));
        // A place that doesn't hold the picture's name (a file changed by hand) is no chip.
        let line = json!({ "pictures": [{ "from": 0, "to": 4, "name": "Image 1", "file": "/p/shot.png" }] });
        assert_eq!(pictures_in(&line, "Like Image 1"), Vec::new());
        keep_in(&root, design, &[]).unwrap();
    }

    #[test]
    fn only_the_newest_lines_are_kept() {
        let root = root().join("newest");
        drop(std::fs::remove_dir_all(&root));
        let design = Path::new("/designs/long.pen");
        let said: Vec<Entry> = (0..MOST + 30).map(|line| Entry::new(Role::User, format!("line {line}"))).collect();
        keep_in(&root, design, &said).unwrap();
        let back = read_in(&root, design);
        assert_eq!((back.len(), back[0].text.as_str(), back[MOST - 1].text.clone()), (MOST, "line 30", format!("line {}", MOST + 29)));
        keep_in(&root, design, &[]).unwrap();
    }
}
