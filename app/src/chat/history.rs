//! What was said in the chats, kept from one run to the next: every conversation in a file of its own.
// Human: a design is worked on over days; the conversation about it belongs to it. What the user and the
// assistant said is kept under `~/.pencraft/chats` and is back in the chat panel when the design is opened
// again. The assistant is told the last of it again, as any engine is that wasn't there (`Chat::retold`).
// The design's own file is not touched.
// Human: there used to be one file per design, and a new chat wrote over it: the conversation before was
// gone. Now every conversation has a file of its own, with its title (the first thing the user said), when
// it was last added to, and the document it was about, and the History view lists them. The newest
// [`MOST_CHATS`] are kept. The files of before (named by a number made from the design's path, without a
// title or a date) are read as they are: their title is their first message, their date the file's.
// Agent: READS and WRITES small files under `~/.pencraft/chats`; fast enough for the window's thread.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::usage::Spent;
use super::{Entry, Pointed, Role};

/// The most lines kept of one conversation: the newest.
pub const MOST: usize = 200;
/// The most conversations kept: the ones last added to. Older ones are removed when a new one is kept.
pub const MOST_CHATS: usize = 100;
/// The most letters of a conversation's first message that make its title.
const TITLE_MOST: usize = 80;

/// A conversation that is kept, as the History view lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Past {
    /// Its file.
    pub file: PathBuf,
    /// The first thing the user said in it, on one line and cut short.
    pub title: String,
    /// The name of the document it was about; empty when it was about none.
    pub name: String,
    /// That document's file, if it had one.
    pub design: Option<PathBuf>,
    /// When it was last added to, in seconds since 1970.
    pub changed: u64,
}

/// The document a conversation is about: its file, if it has one, and its name.
#[derive(Clone, Copy)]
pub struct About<'a> {
    /// The document's file.
    pub design: Option<&'a Path>,
    /// The name its tab shows.
    pub name: &'a str,
}

/// The folder the conversations are kept in.
// Human: the tests keep out of the home folder.
fn root() -> PathBuf {
    if cfg!(test) {
        return std::env::temp_dir().join(format!("pencraft-chats-test-{}", std::process::id()));
    }
    crate::imagery::store::home().map_or_else(|| std::env::temp_dir().join("pencraft-chats"), |home| home.join(".pencraft").join("chats"))
}

/// Now, in seconds since 1970.
fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

/// A file in `root` no conversation has yet, named after the moment `at`.
fn new_file(root: &Path, at: u64) -> PathBuf {
    (0_u32..10_000).map(|turn| root.join(format!("chat-{at:012x}-{turn}.json"))).find(|file| !file.exists()).unwrap_or_else(|| root.join("chat.json"))
}

/// The title of a conversation whose lines are `said`: the first thing the user said, on one line, cut short.
fn title_of(said: &[Value]) -> String {
    let first =
        said.iter().find(|line| line.get("role").and_then(Value::as_str) == Some("user")).and_then(|line| line.get("text")).and_then(Value::as_str);
    let line = first.unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" ");
    if line.is_empty() {
        return "Chat".to_owned();
    }
    if line.chars().count() <= TITLE_MOST {
        return line;
    }
    format!("{}…", line.chars().take(TITLE_MOST).collect::<String>().trim_end())
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
            Role::Assistant => {
                let mut line = json!({ "role": "assistant", "text": entry.text });
                if let Some(spent) = &entry.spent {
                    line["spent"] = spent.kept();
                }
                Some(line)
            }
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

/// Keeps the conversation `entries`, which is `about` a document, with what it took in all (`total`): in
/// `file` when it has one already, else in a new file. Gives the file it is kept in; `None` when nothing was
/// said, which keeps nothing.
///
/// # Errors
/// A sentence when the file can't be written.
// Agent: WRITES one file; a new one also REMOVES the conversations past [`MOST_CHATS`], oldest first.
pub fn keep(file: Option<&Path>, about: About<'_>, entries: &[Entry], total: &Spent) -> Result<Option<PathBuf>, String> {
    keep_in(&root(), file, about, entries, total, now())
}

fn keep_in(root: &Path, file: Option<&Path>, about: About<'_>, entries: &[Entry], total: &Spent, at: u64) -> Result<Option<PathBuf>, String> {
    let said = kept(entries);
    if said.is_empty() {
        return Ok(None);
    }
    std::fs::create_dir_all(root).map_err(|error| format!("{}: {error}", root.display()))?;
    // Only a file of this folder is written: a path from anywhere else gets a new file here.
    let known = file.filter(|file| file.parent() == Some(root));
    let target = known.map_or_else(|| new_file(root, at), Path::to_path_buf);
    let started = known.and_then(whole).and_then(|was| was.get("started").and_then(Value::as_u64)).unwrap_or(at);
    let mut whole = json!({ "title": title_of(&said), "name": about.name, "started": started, "changed": at, "said": said });
    if let Some(design) = about.design {
        whole["design"] = Value::String(design.display().to_string());
    }
    if !total.is_empty() {
        whole["total"] = total.kept();
    }
    craft_atomic::atomic_write(&target, whole.to_string().as_bytes()).map_err(|error| format!("{}: {error}", target.display()))?;
    if known.is_none() {
        prune(root, &target);
    }
    Ok(Some(target))
}

/// Removes the conversations past the newest [`MOST_CHATS`], never `keep`.
fn prune(root: &Path, keep: &Path) {
    for past in list_in(root).into_iter().filter(|past| past.file != keep).skip(MOST_CHATS.saturating_sub(1)) {
        if let Err(error) = std::fs::remove_file(&past.file) {
            log::warn!("chat history: {}: {error}", past.file.display());
        }
    }
}

/// What the file `file` holds, when it is a conversation's.
fn whole(file: &Path) -> Option<Value> {
    std::fs::read(file)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .filter(|kept| kept.get("said").is_some_and(Value::is_array))
}

/// The conversations kept, the one last added to first.
// Agent: READS every file of the folder (at most [`MOST_CHATS`] and a few), whole.
#[must_use]
pub fn list() -> Vec<Past> {
    list_in(&root())
}

fn list_in(root: &Path) -> Vec<Past> {
    let Ok(files) = std::fs::read_dir(root) else { return Vec::new() };
    let mut found: Vec<Past> = files
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|file| file.extension().is_some_and(|ending| ending == "json"))
        .filter_map(|file| {
            let kept = whole(&file)?;
            let said = kept.get("said").and_then(Value::as_array)?;
            if said.is_empty() {
                return None;
            }
            let text = |key: &str| kept.get(key).and_then(Value::as_str).map(str::to_owned);
            let design = text("design").map(PathBuf::from);
            // A file of before has neither: its first message is its title, its design's file its name.
            let title = text("title").unwrap_or_else(|| title_of(said));
            let name = text("name").or_else(|| design.as_ref().and_then(|design| design.file_name()).map(|name| name.to_string_lossy().into_owned()));
            let written = || std::fs::metadata(&file).and_then(|about| about.modified()).ok().and_then(|at| at.duration_since(UNIX_EPOCH).ok());
            let changed = kept.get("changed").and_then(Value::as_u64).or_else(|| written().map(|since| since.as_secs())).unwrap_or(0);
            Some(Past { file, title, name: name.unwrap_or_default(), design, changed })
        })
        .collect();
    found.sort_by(|a, b| b.changed.cmp(&a.changed).then_with(|| b.file.cmp(&a.file)));
    found
}

/// The conversation last added to about the design at `design`, if one is kept.
#[must_use]
pub fn latest_for(design: &Path) -> Option<PathBuf> {
    list().into_iter().find(|past| past.design.as_deref() == Some(design)).map(|past| past.file)
}

/// Removes the kept conversation in `file`.
///
/// # Errors
/// A sentence when `file` is none of the kept conversations' or can't be removed.
// Agent: REMOVES one file, and only one that lies in the folder of the conversations.
pub fn delete(file: &Path) -> Result<(), String> {
    delete_in(&root(), file)
}

fn delete_in(root: &Path, file: &Path) -> Result<(), String> {
    if file.parent() != Some(root) || file.extension().is_none_or(|ending| ending != "json") {
        return Err(format!("{} is not a kept chat", file.display()));
    }
    std::fs::remove_file(file).map_err(|error| format!("{}: {error}", file.display()))
}

/// The conversation kept in `file`, oldest line first, and what it took in all; nothing when the file can't
/// be read.
#[must_use]
pub fn read(file: &Path) -> (Vec<Entry>, Spent) {
    let Some(kept) = whole(file) else {
        return (Vec::new(), Spent::default());
    };
    let said = kept
        .get("said")
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
            Some(Entry { pointed: pictures_in(line, text), spent: line.get("spent").and_then(Spent::from_kept), ..Entry::new(role, text) })
        })
        .collect();
    (said, kept.get("total").and_then(Spent::from_kept).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An empty folder of its own for a test.
    fn folder(name: &str) -> PathBuf {
        let root = root().join(name);
        drop(std::fs::remove_dir_all(&root));
        root
    }

    fn about(design: &Path) -> About<'_> {
        About { design: Some(design), name: "shop.quill" }
    }

    #[test]
    fn every_conversation_has_a_file_of_its_own_and_a_new_one_leaves_the_others() {
        let root = folder("own");
        let shop = Path::new("/designs/shop.quill");
        let said = [
            Entry::new(Role::User, "Make the header\n  taller"),
            Entry::new(Role::Step, "Changed 2 layers"),
            Entry::new(Role::Assistant, "The header is 96 high now."),
            Entry::new(Role::Problem, "The provider didn't answer."),
        ];
        let none = Spent::default();
        let first = keep_in(&root, None, about(shop), &said, &none, 100).unwrap().unwrap();
        // The words come back in order; notes on tools and problems are not kept.
        assert_eq!(read(&first).0, vec![said[0].clone(), said[2].clone()]);
        // A conversation in which nothing was said keeps nothing, and takes nothing away.
        assert_eq!(keep_in(&root, None, about(shop), &[], &none, 150).unwrap(), None);
        assert_eq!(keep_in(&root, Some(&first), about(shop), &[], &none, 150).unwrap(), None);
        // A new chat is another file; the first is as it was.
        let again = [Entry::new(Role::User, "And a footer")];
        let second = keep_in(&root, None, About { design: None, name: "Untitled" }, &again, &none, 200).unwrap().unwrap();
        assert_ne!(first, second);
        assert_eq!(read(&first).0.len(), 2);
        // Listed newest first, each with its title, its document's name and when it was last added to.
        let listed = list_in(&root);
        let shown: Vec<(&str, &str, Option<&Path>, u64)> =
            listed.iter().map(|past| (past.title.as_str(), past.name.as_str(), past.design.as_deref(), past.changed)).collect();
        assert_eq!(shown, [("And a footer", "Untitled", None, 200), ("Make the header taller", "shop.quill", Some(shop), 100)]);
        // Added to, a conversation stays in its file, keeps its title and when it began, and comes first.
        let more = [said[0].clone(), said[2].clone(), Entry::new(Role::User, "Wider, too")];
        assert_eq!(keep_in(&root, Some(&first), about(shop), &more, &none, 300).unwrap().as_ref(), Some(&first));
        let listed = list_in(&root);
        assert_eq!(
            (listed.len(), listed[0].file.as_path(), listed[0].title.as_str(), listed[0].changed),
            (2, first.as_path(), "Make the header taller", 300)
        );
        assert_eq!(whole(&first).unwrap()["started"], 100);
        // The one last added to about a design is the one its chat starts with.
        let listed = list_in(&root);
        assert_eq!(listed.iter().find(|past| past.design.as_deref() == Some(shop)).map(|past| past.file.clone()), Some(first.clone()));
        // Deleted, it is gone from the list; a file that is no kept chat is not touched.
        delete_in(&root, &first).unwrap();
        assert_eq!(list_in(&root).iter().map(|past| past.file.clone()).collect::<Vec<_>>(), [second]);
        let other = root.join("deeper").join("x.json");
        assert!(delete_in(&root, &other).is_err() && delete_in(&root, &root.join("notes.txt")).is_err());
        // A path from somewhere else is never written to: the conversation gets a file here.
        let elsewhere = std::env::temp_dir().join("not-a-chat.json");
        let third = keep_in(&root, Some(&elsewhere), about(shop), &again, &none, 400).unwrap().unwrap();
        assert!(third.parent() == Some(root.as_path()) && !elsewhere.exists());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_long_first_message_is_cut_for_the_title_and_a_chat_without_one_still_has_a_title() {
        let long = "word ".repeat(40);
        let title = title_of(&[json!({ "role": "assistant", "text": "Hello" }), json!({ "role": "user", "text": long })]);
        assert!(title.ends_with('…') && title.chars().count() <= TITLE_MOST + 1 && title.starts_with("word word"));
        assert_eq!(title_of(&[json!({ "role": "assistant", "text": "Hello" })]), "Chat");
        assert_eq!(title_of(&[json!({ "role": "user", "text": "  Größe\tändern \n" })]), "Größe ändern");
    }

    #[test]
    fn a_file_from_before_conversations_had_files_of_their_own_still_loads() {
        let root = folder("before");
        std::fs::create_dir_all(&root).unwrap();
        // As it was written then: named by a number made from the design's path, no title, no date.
        let old = root.join("0123456789abcdef.json");
        let text = r#"{"design":"/designs/shop.pen","said":[{"role":"user","text":"Make a card"},{"role":"assistant","text":"Done."}]}"#;
        std::fs::write(&old, text).unwrap();
        std::fs::write(root.join("broken.json"), "not json").unwrap();
        std::fs::write(root.join("other.json"), r#"{"something":"else"}"#).unwrap();
        let listed = list_in(&root);
        assert_eq!(listed.len(), 1);
        let past = &listed[0];
        assert_eq!(
            (past.title.as_str(), past.name.as_str(), past.design.as_deref()),
            ("Make a card", "shop.pen", Some(Path::new("/designs/shop.pen")))
        );
        assert!(past.changed > 0);
        let (back, sum) = read(&old);
        assert_eq!((back.len(), back[0].text.as_str(), back[1].spent.as_ref(), sum.is_empty()), (2, "Make a card", None, true));
        // Added to, it stays in its file and has a title and a date from then on.
        let more = [back[0].clone(), back[1].clone(), Entry::new(Role::User, "Rounder")];
        let kept =
            keep_in(&root, Some(&old), About { design: Some(Path::new("/designs/shop.pen")), name: "shop.pen" }, &more, &Spent::default(), 500)
                .unwrap();
        assert_eq!(kept.as_deref(), Some(old.as_path()));
        assert_eq!((list_in(&root)[0].changed, list_in(&root)[0].title.as_str()), (500, "Make a card"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn no_more_conversations_are_kept_than_the_most_and_the_oldest_go() {
        let root = folder("most");
        let said = |turn: usize| [Entry::new(Role::User, format!("chat {turn}"))];
        let mut files = Vec::new();
        for turn in 0..MOST_CHATS + 3 {
            let at = 1000 + u64::try_from(turn).unwrap();
            files.push(keep_in(&root, None, About { design: None, name: "" }, &said(turn), &Spent::default(), at).unwrap().unwrap());
        }
        let listed = list_in(&root);
        assert_eq!(
            (listed.len(), listed[0].title.clone(), listed[MOST_CHATS - 1].title.as_str()),
            (MOST_CHATS, format!("chat {}", MOST_CHATS + 2), "chat 3")
        );
        assert!(!files[0].exists() && !files[2].exists() && files[3].exists());
        // Adding to an old one makes it the newest; nothing is removed for that.
        keep_in(&root, Some(&files[3]), About { design: None, name: "" }, &said(3), &Spent::default(), 9000).unwrap();
        assert_eq!((list_in(&root).len(), list_in(&root)[0].file.as_path()), (MOST_CHATS, files[3].as_path()));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_pasted_picture_comes_back_in_its_place() {
        let root = folder("pictures");
        let design = Path::new("/designs/shop.pen");
        let picture = Pointed { at: Some(5..12), icon: "image", name: "Image 1".to_owned(), picture: Some(PathBuf::from("/p/shot.png")) };
        // A layer that was pointed at is words again another day; the picture is still a picture.
        let layer = Pointed { at: Some(17..21), icon: "frame", name: "Card".to_owned(), picture: None };
        let said = [Entry { pointed: vec![picture.clone(), layer], ..Entry::new(Role::User, "Like Image 1 for Card") }];
        let file = keep_in(&root, None, about(design), &said, &Spent::default(), 1).unwrap().unwrap();
        let back = read(&file).0;
        assert_eq!((back[0].text.as_str(), &back[0].pointed), ("Like Image 1 for Card", &vec![picture]));
        // A place that doesn't hold the picture's name (a file changed by hand) is no chip.
        let line = json!({ "pictures": [{ "from": 0, "to": 4, "name": "Image 1", "file": "/p/shot.png" }] });
        assert_eq!(pictures_in(&line, "Like Image 1"), Vec::new());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn only_the_newest_lines_are_kept() {
        let root = folder("newest");
        let said: Vec<Entry> = (0..MOST + 30).map(|line| Entry::new(Role::User, format!("line {line}"))).collect();
        let file = keep_in(&root, None, about(Path::new("/designs/long.pen")), &said, &Spent::default(), 1).unwrap().unwrap();
        let back = read(&file).0;
        assert_eq!((back.len(), back[0].text.as_str(), back[MOST - 1].text.clone()), (MOST, "line 30", format!("line {}", MOST + 29)));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn what_an_answer_took_and_the_conversations_sum_come_back() {
        use crate::chat::usage::{Basis, Used};
        let root = folder("spent");
        let design = Path::new("/designs/shop.pen");
        let spent = Spent {
            used: Used { input: 10_200, cached: 2_200, written: 0, output: 1_800 },
            cost: Some(80_000_000_000),
            partial: false,
            basis: Basis::Listed,
            model: "m".to_owned(),
        };
        let total = Spent {
            used: Used { input: 90_000, output: 9_000, ..Used::default() },
            cost: Some(610_000_000_000),
            partial: true,
            basis: Basis::Mixed,
            model: "m".to_owned(),
        };
        let said = [Entry::new(Role::User, "Make a card"), Entry { spent: Some(spent.clone()), ..Entry::new(Role::Assistant, "Done.") }];
        let file = keep_in(&root, None, about(design), &said, &total, 1).unwrap().unwrap();
        let (back, sum) = read(&file);
        assert_eq!((back[1].spent.as_ref(), back[0].spent.as_ref(), &sum), (Some(&spent), None, &total));
        // A conversation that took nothing keeps no sum.
        keep_in(&root, Some(&file), about(design), &said[..1], &Spent::default(), 2).unwrap();
        assert!(!std::fs::read_to_string(&file).unwrap().contains("total"));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
