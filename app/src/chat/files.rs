//! Files attached to a chat message: pictures the assistant looks at, and text files it reads.
// Human: "Attach context files": a PNG/JPG, DESIGN.md or any text file, dropped on the window or
// picked with +. A text file travels inside the message; a picture as a picture, which both the API and the
// Claude Code program take.

use std::fmt::Write as _;
use std::path::Path;

use serde_json::{Value, json};

/// The largest picture taken, in bytes: as base64 text it stays under the API's 5 MB for one picture.
pub const MOST_PICTURE: u64 = 3_700_000;
/// The largest text file taken, in bytes.
pub const MOST_TEXT: u64 = 256_000;
/// How many files one message can carry.
pub const MOST_FILES: usize = 10;

/// A file as the assistant gets it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Attachment {
    /// A text file: its name and what it says.
    Text {
        /// The file's name.
        name: String,
        /// Its text.
        content: String,
    },
    /// A picture: its name, its media type and its bytes as base64 text.
    Picture {
        /// The file's name.
        name: String,
        /// `image/png` and the like.
        kind: &'static str,
        /// The bytes, as base64.
        data: String,
    },
}

/// The media type of a picture file the assistants take, by the file's ending; `None` for anything else.
#[must_use]
pub fn picture_kind(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// The file's name without its folders, for showing.
#[must_use]
pub fn name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned())
}

/// Reads the file at `path` as something to attach.
///
/// # Errors
/// A sentence for the user when the file can't be read, is too large, or is neither a picture nor text.
pub fn read(path: &Path) -> Result<Attachment, String> {
    let name = name(path);
    let size = std::fs::metadata(path).map_err(|error| format!("Couldn't read {name}: {error}"))?.len();
    let kind = picture_kind(path);
    let most = if kind.is_some() { MOST_PICTURE } else { MOST_TEXT };
    if size > most {
        let what = if kind.is_some() { "A picture" } else { "A text file" };
        return Err(format!("{name} is too large to attach. {what} can be up to {} KB.", most / 1000));
    }
    let bytes = std::fs::read(path).map_err(|error| format!("Couldn't read {name}: {error}"))?;
    match kind {
        Some(kind) => Ok(Attachment::Picture { name, kind, data: crate::agent::base64(&bytes) }),
        None => match String::from_utf8(bytes) {
            Ok(content) => Ok(Attachment::Text { name, content }),
            Err(_) => Err(format!("{name} can't be attached: it is neither a picture (PNG, JPG, GIF, WebP) nor a text file.")),
        },
    }
}

/// `prompt` with the attached text files after it, each in a block that names it, and a line naming the
/// attached pictures so the assistant can tell them apart.
#[must_use]
pub fn written(prompt: &str, files: &[Attachment]) -> String {
    let mut all = prompt.to_owned();
    let mut pictures = Vec::new();
    for file in files {
        match file {
            Attachment::Text { name, content } => {
                let _ = write!(all, "\n\n<attached_file name=\"{name}\">\n{content}\n</attached_file>");
            }
            Attachment::Picture { name, .. } => pictures.push(name.as_str()),
        }
    }
    if !pictures.is_empty() {
        let _ = write!(all, "\n\n<attached_pictures>{}</attached_pictures>", pictures.join(", "));
    }
    all
}

/// The attached pictures as the content blocks the Messages API takes, in the order they were attached.
#[must_use]
pub fn pictures(files: &[Attachment]) -> Vec<Value> {
    files
        .iter()
        .filter_map(|file| match file {
            Attachment::Picture { kind, data, .. } => {
                Some(json!({ "type": "image", "source": { "type": "base64", "media_type": kind, "data": data } }))
            }
            Attachment::Text { .. } => None,
        })
        .collect()
}

/// A user message's content: the text alone, or the pictures and then the text.
#[must_use]
pub fn content(prompt: &str, pictures: &[Value]) -> Value {
    if pictures.is_empty() {
        return json!(prompt);
    }
    let mut blocks = pictures.to_vec();
    blocks.push(json!({ "type": "text", "text": prompt }));
    Value::Array(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!("pencraft-attach-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn pictures_and_text_files_are_read_and_anything_else_is_turned_away() {
        let folder = folder("read");
        let (notes, shot, blob) = (folder.join("DESIGN.md"), folder.join("Shot.PNG"), folder.join("font.bin"));
        std::fs::write(&notes, "# Colours\nBlue.").unwrap();
        std::fs::write(&shot, b"foo").unwrap();
        std::fs::write(&blob, [0xff, 0xfe, 0x00, 0x80]).unwrap();
        assert_eq!(read(&notes), Ok(Attachment::Text { name: "DESIGN.md".to_owned(), content: "# Colours\nBlue.".to_owned() }));
        // A picture is known by its ending, in any case, and travels as base64.
        assert_eq!(read(&shot), Ok(Attachment::Picture { name: "Shot.PNG".to_owned(), kind: "image/png", data: "Zm9v".to_owned() }));
        assert!(read(&blob).is_err_and(|why| why.starts_with("font.bin can't be attached")));
        assert!(read(&folder.join("gone.txt")).is_err_and(|why| why.starts_with("Couldn't read gone.txt")));
        // A text file over the limit is turned away with the limit named.
        let long = folder.join("long.txt");
        std::fs::write(&long, vec![b'a'; usize::try_from(MOST_TEXT).unwrap() + 1]).unwrap();
        assert_eq!(read(&long), Err("long.txt is too large to attach. A text file can be up to 256 KB.".to_owned()));
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn text_files_go_into_the_message_and_pictures_beside_it() {
        let files = [
            Attachment::Text { name: "DESIGN.md".to_owned(), content: "Blue.".to_owned() },
            Attachment::Picture { name: "shot.png".to_owned(), kind: "image/png", data: "Zm9v".to_owned() },
        ];
        let prompt = written("Make it match.", &files);
        assert_eq!(
            prompt,
            "Make it match.\n\n<attached_file name=\"DESIGN.md\">\nBlue.\n</attached_file>\n\n<attached_pictures>shot.png</attached_pictures>"
        );
        let blocks = pictures(&files);
        assert_eq!(blocks, [json!({ "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": "Zm9v" } })]);
        // Without pictures the content is the text as it always was; with them the pictures come first.
        assert_eq!(content("hi", &[]), json!("hi"));
        assert_eq!(content("hi", &blocks), json!([blocks[0], { "type": "text", "text": "hi" }]));
        assert_eq!(written("hi", &[]), "hi");
    }
}
