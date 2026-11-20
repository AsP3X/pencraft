//! Files attached to a chat message: pictures the assistant looks at, and text files it reads.
// Human: "Attach context files": a PNG/JPG, DESIGN.md or any text file, dropped on the window or
// picked with +. A text file travels inside the message; a picture as a picture, which both the API and the
// Claude Code program take.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// The largest picture taken, in bytes: as base64 text it stays under the API's 5 MB for one picture.
pub const MOST_PICTURE: u64 = 3_700_000;
/// The largest text file taken, in bytes.
pub const MOST_TEXT: u64 = 256_000;
/// How many files one message can carry.
pub const MOST_FILES: usize = 10;
/// What stands before and after the name of a picture pasted into a message, in the message as the assistant's
/// engine gets it: where the picture belongs. No one types it (the "object replacement character").
// Agent: never reaches a model; `pieces` and `flat` turn each pair into the name in brackets.
pub const MARK: char = '\u{fffc}';

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
        /// The file it was read from.
        path: PathBuf,
        /// Whether it was pasted into the message's text, where its name marks its place.
        inline: bool,
    },
}

/// The name of a picture pasted into a message, as it stands in the message handed to an engine: marked,
/// so the picture can be put in that very place.
#[must_use]
pub fn marker(name: &str) -> String {
    format!("{MARK}{name}{MARK}")
}

/// A stretch of a message: words, or the place of a pasted picture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// Words, with the names of the pictures in them in brackets.
    Words(String),
    /// The pasted picture with this number (from 0), which stands right after the words before it.
    Picture(usize),
}

/// `prompt` cut at its pasted pictures, of which there are `pictures`: the words up to each picture's name
/// (the name in brackets is their end), the picture, and so on to the words after the last one.
// Human: "make it look like [Image 1] but with the colours of [Image 2]": each picture goes to the model
// right behind its name, so the model knows which one a sentence means.
#[must_use]
pub fn pieces(prompt: &str, pictures: usize) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut words = String::new();
    let mut seen = 0;
    let mut rest = prompt;
    while let Some((before, after)) = rest.split_once(MARK) {
        words.push_str(before);
        rest = after;
        let Some((name, after)) = after.split_once(MARK) else { continue };
        rest = after;
        let _ = write!(words, "[{name}]");
        if seen < pictures {
            out.push(Piece::Words(std::mem::take(&mut words)));
            out.push(Piece::Picture(seen));
            seen += 1;
        }
    }
    words.push_str(rest);
    out.push(Piece::Words(words));
    out.retain(|piece| !matches!(piece, Piece::Words(words) if words.trim().is_empty()));
    out
}

/// `prompt` as plain text: each pasted picture as its name in brackets.
#[must_use]
pub fn flat(prompt: &str) -> String {
    pieces(prompt, 0).into_iter().filter_map(|piece| if let Piece::Words(words) = piece { Some(words) } else { None }).collect()
}

/// `prompt` as plain text for a program that takes no pictures, with where each picture of `files` lies on
/// this computer, so a program that opens files can look at them.
#[must_use]
pub fn with_paths(prompt: &str, files: &[Attachment]) -> String {
    let mut all = flat(prompt);
    let lines: Vec<String> = files
        .iter()
        .filter_map(|file| match file {
            Attachment::Picture { name, path, .. } => Some(format!("{name}: {}", path.display())),
            Attachment::Text { .. } => None,
        })
        .collect();
    if !lines.is_empty() {
        let _ = write!(
            all,
            "\n\n<picture_files>\nThe pictures are files on this computer; open them to look at them.\n{}\n</picture_files>",
            lines.join("\n")
        );
    }
    all
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
        Some(kind) => Ok(Attachment::Picture { name, kind, data: crate::agent::base64(&bytes), path: path.to_path_buf(), inline: false }),
        None => match String::from_utf8(bytes) {
            Ok(content) => Ok(Attachment::Text { name, content }),
            Err(_) => Err(format!("{name} can't be attached: it is neither a picture (PNG, JPG, GIF, WebP) nor a text file.")),
        },
    }
}

/// Reads the picture pasted into a message under the name `label` from its file at `path`.
///
/// # Errors
/// A sentence for the user when the file can't be read or is too large.
pub fn read_pasted(label: &str, path: &Path) -> Result<Attachment, String> {
    match read(path) {
        Ok(Attachment::Picture { kind, data, path, .. }) => Ok(Attachment::Picture { name: label.to_owned(), kind, data, path, inline: true }),
        Ok(Attachment::Text { .. }) => Err(format!("{label} is not a picture.")),
        Err(_) => Err(format!("{label} can't be sent: its picture is no longer there.")),
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
            // A pasted picture is named where it stands in the message.
            Attachment::Picture { name, inline: false, .. } => pictures.push(name.as_str()),
            Attachment::Picture { .. } => {}
        }
    }
    if !pictures.is_empty() {
        let _ = write!(all, "\n\n<attached_pictures>{}</attached_pictures>", pictures.join(", "));
    }
    all
}

/// The pictures of `files` as the content blocks the Messages API takes: the ones pasted into the message
/// first, in the order they stand in it, then the attached ones in the order they were attached.
#[must_use]
pub fn pictures(files: &[Attachment]) -> Vec<Value> {
    let of = |pasted: bool| {
        files.iter().filter_map(move |file| match file {
            Attachment::Picture { kind, data, inline, .. } if *inline == pasted => {
                Some(json!({ "type": "image", "source": { "type": "base64", "media_type": kind, "data": data } }))
            }
            _ => None,
        })
    };
    of(true).chain(of(false)).collect()
}

/// A user message as content blocks: the attached pictures, then the words with each pasted picture right
/// behind its name.
#[must_use]
pub fn blocks(prompt: &str, pictures: &[Value]) -> Vec<Value> {
    let pieces = pieces(prompt, pictures.len());
    let pasted = pieces.iter().filter(|piece| matches!(piece, Piece::Picture(_))).count();
    let mut blocks: Vec<Value> = pictures[pasted..].to_vec();
    for piece in pieces {
        match piece {
            Piece::Words(words) => blocks.push(json!({ "type": "text", "text": words })),
            Piece::Picture(which) => blocks.extend(pictures.get(which).cloned()),
        }
    }
    blocks
}

/// A user message's content: the text alone, or the pictures and the text ([`blocks`]).
#[must_use]
pub fn content(prompt: &str, pictures: &[Value]) -> Value {
    if pictures.is_empty() {
        return json!(flat(prompt));
    }
    Value::Array(blocks(prompt, pictures))
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
        assert_eq!(
            read(&shot),
            Ok(Attachment::Picture { name: "Shot.PNG".to_owned(), kind: "image/png", data: "Zm9v".to_owned(), path: shot.clone(), inline: false })
        );
        // A pasted picture is the same file under the name it has in the message.
        assert_eq!(
            read_pasted("Image 1", &shot),
            Ok(Attachment::Picture { name: "Image 1".to_owned(), kind: "image/png", data: "Zm9v".to_owned(), path: shot.clone(), inline: true })
        );
        assert_eq!(read_pasted("Image 2", &folder.join("gone.png")), Err("Image 2 can't be sent: its picture is no longer there.".to_owned()));
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
            Attachment::Picture {
                name: "shot.png".to_owned(),
                kind: "image/png",
                data: "Zm9v".to_owned(),
                path: PathBuf::from("/s/shot.png"),
                inline: false,
            },
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

    #[test]
    fn a_pasted_picture_goes_to_the_model_right_behind_its_name() {
        let picture = |data: &str, inline: bool| Attachment::Picture {
            name: if inline { format!("Image {data}") } else { "shot.png".to_owned() },
            kind: "image/png",
            data: data.to_owned(),
            path: PathBuf::from(format!("/p/{data}.png")),
            inline,
        };
        // The attached picture was added first; the pasted ones still lead, in the order they stand in the text.
        let files = [picture("0", false), picture("2", true), picture("1", true)];
        let told = format!("note\n\nLike {} but coloured like {}, please", marker("Image 2"), marker("Image 1"));
        let prompt = written(&told, &files);
        let seen = pictures(&files);
        let data = |block: &Value| block.pointer("/source/data").and_then(Value::as_str).unwrap_or_default().to_owned();
        assert_eq!(seen.iter().map(data).collect::<Vec<_>>(), ["2", "1", "0"]);
        let blocks = blocks(&prompt, &seen);
        let read: Vec<String> =
            blocks.iter().map(|block| block["text"].as_str().map_or_else(|| format!("<{}>", data(block)), str::to_owned)).collect();
        assert_eq!(
            read,
            [
                "<0>",
                "note\n\nLike [Image 2]",
                "<2>",
                " but coloured like [Image 1]",
                "<1>",
                ", please\n\n<attached_pictures>shot.png</attached_pictures>"
            ]
        );
        // A message that is one picture and nothing else has no empty words around it.
        assert_eq!(pieces(&marker("Image 1"), 1), [Piece::Words("[Image 1]".to_owned()), Piece::Picture(0)]);
        // As plain text every picture is its name, and a program that opens files is told where they lie.
        assert_eq!(flat(&told), "note\n\nLike [Image 2] but coloured like [Image 1], please");
        assert_eq!(content(&told, &[]), json!(flat(&told)));
        let paths = with_paths(&told, &files[1..2]);
        assert!(paths.starts_with("note\n\nLike [Image 2] but") && paths.contains("\nImage 2: ") && paths.ends_with("</picture_files>"), "{paths}");
        assert_eq!(with_paths("hi", &[]), "hi");
        // A mark without its pair is dropped; more names than pictures leaves the last ones as words.
        assert_eq!(pieces(&format!("a{MARK}b"), 3), [Piece::Words("ab".to_owned())]);
        assert_eq!(
            pieces(&format!("{} and {}", marker("A"), marker("B")), 1),
            [Piece::Words("[A]".to_owned()), Piece::Picture(0), Piece::Words(" and [B]".to_owned())]
        );
    }
}
