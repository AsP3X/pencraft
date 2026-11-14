//! Keeping a made picture: a file in `images/` beside the document, and a line in `images/credits.json`
//! saying where it came from.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{Map, Value};

/// The folder pictures go into, beside the document.
const FOLDER: &str = "images";
/// The file in that folder that says where each picture came from.
pub const CREDITS: &str = "credits.json";
/// The most letters of a description that go into a file's name.
const MOST_NAME: usize = 40;

/// Only one picture is filed at a time in this process, so two jobs ending together don't lose a line of the
/// credits.
static FILING: Mutex<()> = Mutex::new(());

/// A picture that has been written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    /// What an image fill's `url` is to say: `./images/name.png` beside a document, a full path otherwise.
    pub url: String,
    /// The file.
    pub path: PathBuf,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// The start of a file's name for a picture described as `prompt`: its first plain words, joined by `-`; a
/// first word longer than a name may be is cut short.
fn stem(prompt: &str) -> String {
    let mut name = String::new();
    for word in prompt.split(|letter: char| !letter.is_ascii_alphanumeric()).filter(|word| !word.is_empty()) {
        if name.is_empty() && word.len() > MOST_NAME {
            // The word is plain ASCII, so any length is a place it can be cut.
            name.push_str(&word.get(..MOST_NAME).unwrap_or(word).to_ascii_lowercase());
            break;
        }
        if name.len() + word.len() + 1 > MOST_NAME {
            break;
        }
        if !name.is_empty() {
            name.push('-');
        }
        name.push_str(&word.to_ascii_lowercase());
    }
    if name.is_empty() { "picture".to_owned() } else { name }
}

/// The user's home folder.
#[must_use]
pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from)
}

/// Writes the picture `bytes` (described as `prompt`) and files `credit` under its name.
///
/// `document` is the document's `.pen` file; without one the picture goes to `.pencraft/images` under `home`.
///
/// # Errors
/// A sentence when the bytes aren't a picture, the credits can't be read, or the folder can't be written;
/// nothing is left behind then.
// Human: the credits are read before the picture is written, so a credits file that can't be kept up to date
// stops the save while nothing has changed yet; one that fails to be written takes the new picture with it.
// Agent: WRITES two files; BLOCKS on decoding the picture, so call it on a worker thread.
pub fn save(document: Option<&Path>, home: Option<&Path>, bytes: &[u8], prompt: &str, credit: &Value) -> Result<Saved, String> {
    const NOT_A_PICTURE: &str = "The service sent something that isn't a picture.";
    let ending = match image::guess_format(bytes) {
        Ok(image::ImageFormat::Png) => "png",
        Ok(image::ImageFormat::Jpeg) => "jpg",
        Ok(image::ImageFormat::WebP) => "webp",
        _ => return Err(NOT_A_PICTURE.to_owned()),
    };
    let (width, height) = image::load_from_memory(bytes).map(|picture| (picture.width(), picture.height())).map_err(|_| NOT_A_PICTURE.to_owned())?;
    let beside = document.and_then(Path::parent).map(|folder| folder.join(FOLDER));
    let folder = beside
        .clone()
        .or_else(|| home.map(|home| home.join(".pencraft").join(FOLDER)))
        .ok_or("There is no folder to keep the picture in: save the document first.")?;
    // A save that panicked can't have left the folder half-filed (every step undoes itself), so its lock is fine.
    let _one_at_a_time = FILING.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let not_saved = |error: &dyn std::fmt::Display| format!("The picture couldn't be saved in {}: {error}.", folder.display());
    std::fs::create_dir_all(&folder).map_err(|error| not_saved(&error))?;
    let credits = folder.join(CREDITS);
    let mut filed = filed(&credits)?;
    // Human: a few letters from the clock tell two pictures of the same description apart; a name that is
    // taken all the same gets the next number.
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.subsec_nanos());
    let (name, path) = write_new(&folder, &stem(prompt), ending, nanos, bytes).map_err(|error| not_saved(&error))?;
    filed.insert(name.clone(), credit.clone());
    let written = serde_json::to_string_pretty(&filed)
        .map_err(|error| error.to_string())
        .and_then(|text| craft_atomic::atomic_write(&credits, text.as_bytes()).map_err(|error| error.to_string()));
    if let Err(error) = written {
        // The picture without its credit would be a file nobody knows the source of; it goes again.
        let _ = std::fs::remove_file(&path);
        return Err(not_saved(&error));
    }
    let url = if beside.is_some() { format!("./{FOLDER}/{name}") } else { path.to_string_lossy().replace('\\', "/") };
    Ok(Saved { url, path, width, height })
}

/// Takes back a picture [`save`] wrote: removes the file at `path` and its line in the credits beside it. A
/// credits file left with no line at all goes too.
///
/// # Errors
/// A sentence when the file or the credits couldn't be changed.
// Human: for a picture nobody waits for any more, which would otherwise be a file nobody uses.
// Agent: WRITES the two files `save` wrote, under the same lock, so a picture being filed meanwhile keeps its
// line. A file that is already gone is fine.
pub fn forget(path: &Path) -> Result<(), String> {
    let (Some(folder), Some(name)) = (path.parent(), path.file_name().and_then(|name| name.to_str())) else { return Ok(()) };
    // A save that panicked can't have left the folder half-filed (every step undoes itself), so its lock is fine.
    let _one_at_a_time = FILING.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let not_removed = |error: &dyn std::fmt::Display| format!("The picture {} couldn't be removed: {error}.", path.display());
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(not_removed(&error)),
    }
    let credits = folder.join(CREDITS);
    let mut filed = filed(&credits)?;
    if filed.remove(name).is_none() {
        return Ok(());
    }
    if filed.is_empty() {
        return std::fs::remove_file(&credits).map_err(|error| not_removed(&error));
    }
    serde_json::to_string_pretty(&filed)
        .map_err(|error| error.to_string())
        .and_then(|text| craft_atomic::atomic_write(&credits, text.as_bytes()).map_err(|error| error.to_string()))
        .map_err(|error| not_removed(&error))
}

/// What the credits file `credits` holds: nothing when there is no such file.
///
/// # Errors
/// A sentence when the file is there but can't be read, or isn't a JSON object.
fn filed(credits: &Path) -> Result<Map<String, Value>, String> {
    let unreadable =
        |why: &dyn std::fmt::Display| format!("{FOLDER}/{CREDITS} couldn't be read ({why}). Fix or remove it; the picture was not saved.");
    let text = match std::fs::read_to_string(credits) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(error) => return Err(unreadable(&error)),
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(filed)) => Ok(filed),
        Ok(_) => Err(unreadable(&"it isn't a JSON object")),
        Err(error) => Err(unreadable(&error)),
    }
}

/// Writes `bytes` to a new file in `folder` named `{stem}-{number}.{ending}`, trying the numbers from `first` on
/// until one is free, and gives the name and the file.
///
/// # Errors
/// The system's reason when no file could be made or written; a file that was made is removed again.
// Human: the name is claimed by making the file (`create_new`), which fails when it is already there, so two
// savers can't both pick the same free name; a name that is taken gets the next number.
// Agent: WRITES one file in `folder`; tries at most 65536 names.
fn write_new(folder: &Path, stem: &str, ending: &str, first: u32, bytes: &[u8]) -> Result<(String, PathBuf), std::io::Error> {
    use std::io::Write as _;
    for n in 0..=0xffff_u32 {
        let name = format!("{stem}-{:04x}.{ending}", first.wrapping_add(n) & 0xffff);
        let path = folder.join(&name);
        let mut file = match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        return Ok((name, path));
    }
    Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "every name for it is taken"))
}

/// A PNG copy of the picture `bytes`, no wider or taller than `side` pixels, for a model to look at.
///
/// # Errors
/// A sentence when the bytes aren't a picture.
pub fn shown(bytes: &[u8], side: u32) -> Result<Vec<u8>, String> {
    let picture = image::load_from_memory(bytes).map_err(|error| error.to_string())?;
    let picture = if picture.width() > side || picture.height() > side { picture.thumbnail(side, side) } else { picture };
    let mut out = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut out, image::ImageFormat::Png).map_err(|error| error.to_string())?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(width, height, image::Rgba([200, 30, 30, 255])).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        bytes.into_inner()
    }

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("pencraft-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_name_is_made_of_the_first_plain_words() {
        assert_eq!(stem("A red fox, in SNOW!"), "a-red-fox-in-snow");
        assert_eq!(stem("  ¿¡ "), "picture");
        assert_eq!(stem(&"word ".repeat(30)).len(), 39);
        assert_eq!(stem(&format!("{} fox", "a".repeat(50))), "a".repeat(MOST_NAME));
    }

    #[test]
    fn a_picture_lands_beside_the_document_with_its_credit_and_never_replaces_another() {
        let folder = folder("beside");
        let document = folder.join("site.pen");
        let credit = json!({ "made": "generated", "service": "My Gemini", "model": "m", "prompt": "A red fox" });
        let first = save(Some(&document), None, &png(4, 2), "A red fox", &credit).unwrap();
        assert_eq!((first.width, first.height), (4, 2));
        assert!(
            first.url.starts_with("./images/a-red-fox-") && Path::new(&first.url).extension().is_some_and(|ending| ending == "png"),
            "{}",
            first.url
        );
        assert_eq!(first.path, folder.join("images").join(first.url.trim_start_matches("./images/")));
        assert!(first.path.is_file());
        let second = save(Some(&document), None, &png(4, 2), "A red fox", &credit).unwrap();
        assert_ne!(first.path, second.path);
        let credits: Value = serde_json::from_str(&std::fs::read_to_string(folder.join("images").join(CREDITS)).unwrap()).unwrap();
        let names: Vec<&String> = credits.as_object().unwrap().keys().collect();
        assert_eq!(names.len(), 2);
        assert_eq!(credits[first.path.file_name().unwrap().to_str().unwrap()], credit);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_document_without_a_file_gets_a_full_path_under_the_home_folder() {
        let home = folder("home");
        let saved = save(None, Some(&home), &png(1, 1), "x", &json!({})).unwrap();
        assert_eq!(saved.path.parent().unwrap(), home.join(".pencraft").join("images"));
        assert_eq!(saved.url, saved.path.to_string_lossy().replace('\\', "/"));
        assert!(Path::new(&saved.url).is_absolute());
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn what_is_not_a_picture_is_refused_and_nothing_is_written() {
        let folder = folder("refused");
        let document = folder.join("site.pen");
        assert_eq!(save(Some(&document), None, b"<html>", "x", &json!({})).unwrap_err(), "The service sent something that isn't a picture.");
        assert!(!folder.join("images").exists());
        assert_eq!(save(None, None, &png(1, 1), "x", &json!({})).unwrap_err(), "There is no folder to keep the picture in: save the document first.");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_picture_taken_back_leaves_neither_its_file_nor_its_credit() {
        let folder = folder("forget");
        let document = folder.join("site.pen");
        let credits = folder.join("images").join(CREDITS);
        let first = save(Some(&document), None, &png(1, 1), "A fox", &json!({ "n": 1 })).unwrap();
        let second = save(Some(&document), None, &png(1, 1), "A fox", &json!({ "n": 2 })).unwrap();
        forget(&second.path).unwrap();
        assert!(!second.path.exists() && first.path.is_file());
        let filed: Value = serde_json::from_str(&std::fs::read_to_string(&credits).unwrap()).unwrap();
        assert_eq!(filed, json!({ first.path.file_name().unwrap().to_str().unwrap(): { "n": 1 } }));
        // Human: taking back one that is gone already changes nothing; the last one takes the credits with it.
        forget(&second.path).unwrap();
        forget(&first.path).unwrap();
        assert!(!first.path.exists() && !credits.exists());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_name_that_is_taken_is_never_written_over_and_the_next_one_is_used() {
        let folder = folder("taken");
        std::fs::write(folder.join("fox-0007.png"), b"mine").unwrap();
        std::fs::write(folder.join("fox-ffff.png"), b"mine").unwrap();
        let (name, path) = write_new(&folder, "fox", "png", 7, b"new").unwrap();
        assert_eq!((name.as_str(), path.clone()), ("fox-0008.png", folder.join("fox-0008.png")));
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(std::fs::read(folder.join("fox-0007.png")).unwrap(), b"mine");
        // The numbers go round: after ffff comes 0000.
        assert_eq!(write_new(&folder, "fox", "png", 0xffff, b"new").unwrap().0, "fox-0000.png");
        assert_eq!(std::fs::read(folder.join("fox-ffff.png")).unwrap(), b"mine");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn credits_that_cant_be_read_stop_the_save_before_anything_is_written() {
        for (name, broken) in [("broken", "{ not json"), ("list", "[1, 2]")] {
            let folder = folder(name);
            let images = folder.join("images");
            std::fs::create_dir_all(&images).unwrap();
            std::fs::write(images.join(CREDITS), broken).unwrap();
            let why = save(Some(&folder.join("site.pen")), None, &png(1, 1), "A fox", &json!({})).unwrap_err();
            assert!(why.starts_with("images/credits.json couldn't be read ("), "{why}");
            assert!(why.ends_with("). Fix or remove it; the picture was not saved."), "{why}");
            assert_eq!(std::fs::read_to_string(images.join(CREDITS)).unwrap(), broken);
            assert_eq!(std::fs::read_dir(&images).unwrap().count(), 1, "only the credits are there");
            std::fs::remove_dir_all(&folder).unwrap();
        }
    }

    #[test]
    fn the_copy_for_a_model_is_a_png_no_larger_than_asked() {
        let small = shown(&png(40, 20), 10).unwrap();
        let picture = image::load_from_memory(&small).unwrap();
        assert_eq!((picture.width(), picture.height()), (10, 5));
        assert_eq!(image::guess_format(&small).unwrap(), image::ImageFormat::Png);
        let same = shown(&png(4, 2), 10).unwrap();
        assert_eq!(image::load_from_memory(&same).unwrap().width(), 4);
    }
}
