//! Pictures made by an image model service: which services there are, asking one for a picture, and keeping
//! the result as a file beside the document.
//!
//! Nothing here draws. [`service`] is what a request and a failure are; [`gemini`] and [`openai`] each speak
//! one provider's API; [`store`] writes the picture and where it came from; [`jobs`] does all of that on a
//! thread of its own, because a picture takes up to a minute.

pub mod dialog;
pub mod gemini;
pub mod jobs;
pub mod openai;
pub mod service;
pub mod sources;
pub mod store;

use std::path::{Path, PathBuf};

use craft_ai_core::SecretString;
use serde_json::{Value, json};

use crate::prefs::{ImageService, Images};
use crate::secrets::Vault;
use service::{ASPECTS, Aspect, Failure, Made, Request};

/// What a model or a person is told when no service is set up.
pub const NO_SERVICE: &str =
    "No image service is set up. Add one with its API key in Settings → Images, or start Quill with GEMINI_API_KEY or OPENAI_API_KEY set.";
/// What a model is told when a picture it names is on a network share.
pub const ON_A_SHARE: &str = "Pictures on a network share can't be sent; copy it to this computer first.";
/// What a model or a person is told when there are services but none can edit a picture.
pub const NO_EDITING: &str =
    "None of the image services can edit pictures. Add a Google Gemini service in Settings → Images, or start Quill with GEMINI_API_KEY set.";

/// The kinds of service Quill can talk to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Google's Gemini image models.
    Gemini,
    /// `OpenAI`'s image models, and services that speak the same API.
    OpenAi,
}

impl Kind {
    /// Every kind, in the order the settings offer them.
    pub const ALL: [Kind; 2] = [Kind::Gemini, Kind::OpenAi];

    /// The kind's name, as the settings show and save it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Kind::Gemini => "Google Gemini",
            Kind::OpenAi => "OpenAI Images",
        }
    }

    /// The kind called `name`.
    #[must_use]
    pub fn named(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.name() == name)
    }

    /// Where the kind's own service is reached.
    #[must_use]
    pub fn base(self) -> &'static str {
        match self {
            Kind::Gemini => gemini::BASE,
            Kind::OpenAi => openai::BASE,
        }
    }

    /// The model a new service of this kind starts with.
    #[must_use]
    pub fn model(self) -> &'static str {
        match self {
            Kind::Gemini => gemini::MODEL,
            Kind::OpenAi => openai::MODEL,
        }
    }

    /// Whether the kind can make a transparent background.
    #[must_use]
    pub fn transparent(self) -> bool {
        matches!(self, Kind::OpenAi)
    }

    /// Whether the kind can change a picture that is sent to it.
    // Human: only Gemini for now; OpenAI's edit endpoint waits for a live check (stage 2's spec).
    #[must_use]
    pub fn edits(self) -> bool {
        matches!(self, Kind::Gemini)
    }

    /// The environment variable that gives a key for this kind, and what such a service is called.
    fn environment(self) -> (&'static str, &'static str) {
        match self {
            Kind::Gemini => ("GEMINI_API_KEY", "Gemini"),
            Kind::OpenAi => ("OPENAI_API_KEY", "OpenAI"),
        }
    }
}

/// Where a service's key is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyFrom {
    /// This environment variable.
    Environment(&'static str),
    /// The keychain, under this name.
    Keychain(String),
}

/// A service that can be asked: it has a kind Quill knows and a key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ready {
    /// What the user calls it.
    pub name: String,
    /// The API it speaks.
    pub kind: Kind,
    /// Where it is reached.
    pub base_url: String,
    /// The model it is asked for.
    pub model: String,
    /// Where its key is.
    pub key: KeyFrom,
}

/// The services that can be asked: the saved ones that have a key, then one for each key in the environment
/// (`variable` reads it) whose name no saved one has.
#[must_use]
pub fn ready(images: &Images, variable: &dyn Fn(&str) -> Option<String>) -> Vec<Ready> {
    let or = |text: &str, usual: &str| if text.trim().is_empty() { usual.to_owned() } else { text.trim().to_owned() };
    let mut ready: Vec<Ready> = images
        .services
        .iter()
        .filter_map(|saved| {
            let kind = Kind::named(&saved.kind)?;
            let key = KeyFrom::Keychain(saved.key_ref.clone()?);
            Some(Ready { name: saved.name.clone(), kind, base_url: or(&saved.base_url, kind.base()), model: or(&saved.model, kind.model()), key })
        })
        .collect();
    for kind in Kind::ALL {
        let (name, called) = kind.environment();
        if variable(name).is_some_and(|key| !key.trim().is_empty()) && !ready.iter().any(|r| r.name.eq_ignore_ascii_case(called)) {
            ready.push(Ready {
                name: called.to_owned(),
                kind,
                base_url: kind.base().to_owned(),
                model: kind.model().to_owned(),
                key: KeyFrom::Environment(name),
            });
        }
    }
    ready
}

/// The service to use: the one `asked` for by name, else the one the settings call the default, else the first.
/// `saved` are the services in the settings, so one asked for that has no key yet is told apart from one that
/// isn't there.
///
/// # Errors
/// A sentence when there is none, none by that name, or the one by that name has no key yet.
pub fn pick<'a>(ready: &'a [Ready], saved: &[ImageService], asked: Option<&str>, default: &str) -> Result<&'a Ready, String> {
    let named = |name: &str| ready.iter().find(|r| r.name.eq_ignore_ascii_case(name.trim()));
    let keyless =
        |name: &str| saved.iter().find(|s| s.name.eq_ignore_ascii_case(name.trim()) && s.key_ref.is_none() && Kind::named(&s.kind).is_some());
    match asked.filter(|name| !name.trim().is_empty()) {
        Some(name) => named(name).ok_or_else(|| {
            if let Some(service) = keyless(name) {
                format!("\"{}\" has no API key yet. Add it in Settings → Images.", service.name)
            } else if ready.is_empty() {
                NO_SERVICE.to_owned()
            } else {
                format!(
                    "There is no image service called \"{name}\". The ones there are: {}.",
                    ready.iter().map(|r| r.name.as_str()).collect::<Vec<_>>().join(", ")
                )
            }
        }),
        None => named(default).or_else(|| ready.first()).ok_or_else(|| NO_SERVICE.to_owned()),
    }
}

/// What `get_state` says about the services.
#[must_use]
pub fn listed(ready: &[Ready], default: &str) -> Value {
    let usual = pick(ready, &[], None, default).ok().map(|r| r.name.clone());
    ready
        .iter()
        .map(|r| {
            json!({
                "name": r.name,
                "kind": r.kind.name(),
                "model": r.model,
                "default": Some(&r.name) == usual.as_ref(),
                "transparent": r.kind.transparent(),
                "edits": r.kind.edits(),
            })
        })
        .collect()
}

/// The argument `name` of a call, unless it was left out or sent as null.
// Human: an argument left out (or sent as null) takes its usual value; one of the wrong type is a mistake.
fn given<'a>(args: &'a Value, name: &str) -> Option<&'a Value> {
    args.get(name).filter(|value| !value.is_null())
}

/// The `aspect` a call asks for, if it asks for one.
fn aspect_of(args: &Value) -> Result<Option<Aspect>, String> {
    given(args, "aspect")
        .map(|value| value.as_str().and_then(Aspect::parse).ok_or_else(|| format!("`aspect` must be one of {}.", ASPECTS.join(", "))))
        .transpose()
}

/// Whether a call asks for the large `size`.
fn large_of(args: &Value) -> Result<bool, String> {
    match given(args, "size").map(Value::as_str) {
        None | Some(Some("normal")) => Ok(false),
        Some(Some("large")) => Ok(true),
        Some(_) => Err("`size` must be \"normal\" or \"large\".".to_owned()),
    }
}

/// The request a `generate_image` call (or the dialog, in the same form) asks of the service `ready`.
///
/// # Errors
/// A sentence saying which argument is wrong.
pub fn request(args: &Value, ready: &Ready) -> Result<Request, String> {
    let prompt = args.get("prompt").and_then(Value::as_str).map(str::trim).filter(|prompt| !prompt.is_empty());
    let prompt = prompt.ok_or("`prompt` is missing: say what the picture shows.")?.to_owned();
    let aspect = aspect_of(args)?.unwrap_or_default();
    let large = large_of(args)?;
    let transparent = match given(args, "transparent") {
        Some(value) => value.as_bool().ok_or("`transparent` must be true or false.")?,
        None => false,
    };
    if transparent && !ready.kind.transparent() {
        return Err(format!(
            "{} can't make a transparent background. Leave `transparent` out, or use a service that can (get_state lists them).",
            ready.name
        ));
    }
    Ok(Request { prompt, model: ready.model.clone(), aspect: Some(aspect), large, transparent, sources: Vec::new() })
}

/// What an `edit_image` call names as its pictures, before anything is read or drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    /// The picture to change, if any.
    pub change: Option<Pick>,
    /// The reference pictures, at most [`sources::MOST_REFERENCES`].
    pub references: Vec<Pick>,
}

/// One picture a call names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    /// A layer's image fill (`layer`), or a reference that is a layer id.
    Layer(String),
    /// Layers to draw together (`layers`).
    Drawn(Vec<String>),
    /// A picture file by its full path (`file`, or a reference that is a path).
    File(PathBuf),
}

/// Whether `path` names a file on a network share (`\\server\share\…` or `//server/share/…`).
// Human: reading a share can hang for as long as the network takes, and a path to one is never the user's own
// picture on this computer; it is told apart by how it is written, without asking the filesystem.
fn on_a_share(path: &str) -> bool {
    path.starts_with("\\\\") || path.starts_with("//")
}

/// The pictures an `edit_image` call names.
///
/// # Errors
/// A sentence when more than one of `layer`, `layers`, `file` is given, when there are more than four
/// references, when a path isn't a full one or is on a network share, or when the call names no picture at all.
pub fn named(args: &Value) -> Result<Named, String> {
    let ways = ["layer", "layers", "file"].into_iter().filter(|way| given(args, way).is_some()).count();
    if ways > 1 {
        return Err("Give one of `layer`, `layers` and `file`, not several: one picture is changed at a time.".to_owned());
    }
    let change = if let Some(layer) = given(args, "layer") {
        Some(Pick::Layer(layer.as_str().ok_or("`layer` must be a layer's id.")?.trim().to_owned()))
    } else if let Some(layers) = given(args, "layers") {
        let layers: Option<Vec<String>> =
            layers.as_array().and_then(|layers| layers.iter().map(|id| id.as_str().map(|id| id.trim().to_owned())).collect());
        let layers = layers.ok_or("`layers` must be a list of layer ids.")?;
        if layers.is_empty() {
            return Err("`layers` is empty: name the layers to draw.".to_owned());
        }
        Some(Pick::Drawn(layers))
    } else if let Some(file) = given(args, "file") {
        let file = file.as_str().ok_or("`file` must be a picture file's full path.")?.trim();
        if on_a_share(file) {
            return Err(ON_A_SHARE.to_owned());
        }
        if !Path::new(file).is_absolute() {
            return Err(format!("`file` must be a full path, starting at the root: \"{file}\" isn't."));
        }
        Some(Pick::File(PathBuf::from(file)))
    } else {
        None
    };
    let references: Option<Vec<&str>> =
        given(args, "references").map_or(Some(Vec::new()), |list| list.as_array().and_then(|list| list.iter().map(Value::as_str).collect()));
    let references = references.ok_or("`references` must be a list of layer ids and full file paths.")?;
    if references.len() > sources::MOST_REFERENCES {
        return Err(format!("At most {} reference pictures can be sent; {} were named.", sources::MOST_REFERENCES, references.len()));
    }
    if references.iter().any(|reference| on_a_share(reference.trim())) {
        return Err(ON_A_SHARE.to_owned());
    }
    // Human: a reference that is a full path is a file; anything else is a layer's id.
    let references: Vec<Pick> = references
        .into_iter()
        .map(str::trim)
        .map(|reference| if Path::new(reference).is_absolute() { Pick::File(PathBuf::from(reference)) } else { Pick::Layer(reference.to_owned()) })
        .collect();
    if change.is_none() && references.is_empty() {
        return Err(
            "There is no picture to change and none to go by. Name `layer`, `layers` or `file`, or use generate_image to make a picture from words alone."
                .to_owned(),
        );
    }
    Ok(Named { change, references })
}

/// The request an `edit_image` call asks of the service `ready`: `instruction` as the prompt; `size` and
/// `service` as in [`request`]. `aspect` as given; without one, the shape of the picture to change when there is
/// one (`changes`), else 1:1 as for a generation.
///
/// # Errors
/// A sentence saying which argument is wrong, or that this kind of service can't edit.
pub fn edit_request(args: &Value, ready: &Ready, changes: bool) -> Result<Request, String> {
    let instruction = args.get("instruction").and_then(Value::as_str).map(str::trim).filter(|instruction| !instruction.is_empty());
    let prompt = instruction.ok_or("`instruction` is missing: say what to change.")?.to_owned();
    if !ready.kind.edits() {
        return Err(format!(
            "{} is an {} service, and editing isn't possible with that kind yet; use a Google Gemini service (get_state lists which can edit).",
            ready.name,
            ready.kind.name()
        ));
    }
    // Human: with references alone there is no shape to keep, and the service makes a square when asked none.
    let aspect = aspect_of(args)?.or_else(|| (!changes).then(Aspect::default));
    Ok(Request { prompt, model: ready.model.clone(), aspect, large: large_of(args)?, transparent: false, sources: Vec::new() })
}

/// The service to edit with: the one `asked` for by name (as [`pick`] finds it, whether it can edit or not:
/// [`edit_request`] says why it can't), else the settings' `default` when it can edit, else the first that can.
///
/// # Errors
/// What [`pick`] says of a name; [`NO_SERVICE`] when there is no service, [`NO_EDITING`] when none can edit.
pub fn pick_editing<'a>(ready: &'a [Ready], saved: &[ImageService], asked: Option<&str>, default: &str) -> Result<&'a Ready, String> {
    if asked.is_some_and(|name| !name.trim().is_empty()) {
        return pick(ready, saved, asked, default);
    }
    let edits = |r: &&Ready| r.kind.edits();
    pick(ready, saved, None, default)
        .ok()
        .filter(edits)
        .or_else(|| ready.iter().find(edits))
        .ok_or_else(|| if ready.is_empty() { NO_SERVICE } else { NO_EDITING }.to_owned())
}

/// The key of the service `ready`.
///
/// # Errors
/// [`Failure::NoKey`] when it is no longer where it was.
// Human: a key from the environment is set where Quill was started, so the sentence names the variable and
// nothing about Settings; a key from the keychain was added in Settings, so that is where to add it again.
pub fn key(ready: &Ready, vault: &dyn Vault) -> Result<SecretString, Failure> {
    match &ready.key {
        KeyFrom::Environment(name) => std::env::var(name)
            .ok()
            .filter(|key| !key.trim().is_empty())
            .map(|key| SecretString::new(key.trim()))
            .ok_or_else(|| Failure::NoKey(format!("the {name} environment variable is no longer set."))),
        KeyFrom::Keychain(name) => vault.get(name).map_err(|error| {
            Failure::NoKey(format!("the keychain answered \"{}\". Add it again in Settings → Images.", error.trim().trim_end_matches('.')))
        }),
    }
}

/// Asks the service `ready` for the picture `request` describes.
///
/// # Errors
/// Why there is no picture.
// Agent: BLOCKS for one answer (up to 95 s).
pub fn make(ready: &Ready, request: &Request, key: &SecretString) -> Result<Made, Failure> {
    match ready.kind {
        Kind::Gemini => gemini::make(&ready.base_url, request, key),
        Kind::OpenAi => openai::make(&ready.base_url, request, key),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn images() -> Images {
        Images {
            services: vec![
                ImageService {
                    name: "Studio".to_owned(),
                    kind: "OpenAI Images".to_owned(),
                    base_url: String::new(),
                    model: String::new(),
                    key_ref: Some("k1".to_owned()),
                },
                ImageService {
                    name: "No key".to_owned(),
                    kind: "Google Gemini".to_owned(),
                    base_url: String::new(),
                    model: String::new(),
                    key_ref: None,
                },
                ImageService {
                    name: "Odd".to_owned(),
                    kind: "Something else".to_owned(),
                    base_url: String::new(),
                    model: String::new(),
                    key_ref: Some("k2".to_owned()),
                },
            ],
            default: String::new(),
            assistant: true,
        }
    }

    #[test]
    fn the_working_services_are_the_saved_ones_with_a_key_and_the_ones_the_environment_gives() {
        let ready = ready(&images(), &|name| (name == "GEMINI_API_KEY").then(|| "g".to_owned()));
        let told: Vec<(&str, Kind, &str, &str)> = ready.iter().map(|r| (r.name.as_str(), r.kind, r.base_url.as_str(), r.model.as_str())).collect();
        assert_eq!(told, [("Studio", Kind::OpenAi, openai::BASE, openai::MODEL), ("Gemini", Kind::Gemini, gemini::BASE, gemini::MODEL)]);
        assert_eq!(ready[0].key, KeyFrom::Keychain("k1".to_owned()));
        assert_eq!(ready[1].key, KeyFrom::Environment("GEMINI_API_KEY"));
        assert_eq!(super::ready(&Images::default(), &|_| None), []);
    }

    #[test]
    fn one_is_picked_by_name_or_by_default_or_first() {
        let saved = images().services;
        let ready = ready(&images(), &|name| (name == "GEMINI_API_KEY").then(|| "g".to_owned()));
        assert_eq!(pick(&ready, &saved, Some("gemini"), "").map(|r| r.name.as_str()), Ok("Gemini"));
        assert_eq!(pick(&ready, &saved, None, "Gemini").map(|r| r.name.as_str()), Ok("Gemini"));
        assert_eq!(pick(&ready, &saved, None, "Gone").map(|r| r.name.as_str()), Ok("Studio"));
        assert_eq!(
            pick(&ready, &saved, Some("Nope"), ""),
            Err("There is no image service called \"Nope\". The ones there are: Studio, Gemini.".to_owned())
        );
        assert_eq!(pick(&[], &[], None, ""), Err(NO_SERVICE.to_owned()));
    }

    #[test]
    fn a_saved_service_without_a_key_is_named_as_such_and_not_listed() {
        let saved = images().services;
        let ready = ready(&images(), &|_| None);
        assert_eq!(pick(&ready, &saved, Some("no KEY"), ""), Err("\"No key\" has no API key yet. Add it in Settings → Images.".to_owned()));
        assert_eq!(pick(&[], &saved, Some("No key"), ""), Err("\"No key\" has no API key yet. Add it in Settings → Images.".to_owned()));
        // A kind Quill doesn't know is not a service at all, key or none.
        assert_eq!(pick(&ready, &saved, Some("Odd"), ""), Err("There is no image service called \"Odd\". The ones there are: Studio.".to_owned()));
        assert!(listed(&ready, "").as_array().is_some_and(|listed| listed.iter().all(|service| service["name"] != "No key")));
    }

    /// A keychain that has the keys it is given and says `refusal` for any other.
    struct Kept(&'static str);

    impl Vault for Kept {
        fn put(&self, _: &str, _: &SecretString) -> Result<(), String> {
            Ok(())
        }

        fn get(&self, name: &str) -> Result<SecretString, String> {
            if name == "k1" { Ok(SecretString::new("secret")) } else { Err(self.0.to_owned()) }
        }

        fn forget(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn a_key_that_cant_be_found_says_where_it_was_looked_for() {
        let ready = ready(&images(), &|_| None);
        assert_eq!(key(&ready[0], &Kept("No matching entry found.")).map(|key| key.expose().to_owned()), Ok("secret".to_owned()));
        let gone = Ready { key: KeyFrom::Keychain("k9".to_owned()), ..ready[0].clone() };
        assert_eq!(
            key(&gone, &Kept("No matching entry found.")).map_err(|failure| failure.sentence(&gone.name)).map(|_| ()),
            Err("Studio's API key couldn't be found: the keychain answered \"No matching entry found\". Add it again in Settings → Images."
                .to_owned())
        );
        let unset = Ready { name: "Gemini".to_owned(), key: KeyFrom::Environment("PENCRAFT_TEST_NO_SUCH_KEY"), ..ready[0].clone() };
        assert_eq!(
            key(&unset, &Kept("")).map_err(|failure| failure.sentence(&unset.name)).map(|_| ()),
            Err("Gemini's API key couldn't be found: the PENCRAFT_TEST_NO_SUCH_KEY environment variable is no longer set.".to_owned())
        );
    }

    #[test]
    fn get_state_lists_them_with_what_they_can_do() {
        let ready = ready(&images(), &|_| None);
        assert_eq!(
            listed(&ready, ""),
            json!([{ "name": "Studio", "kind": "OpenAI Images", "model": openai::MODEL, "default": true, "transparent": true, "edits": false }])
        );
        let both = super::ready(&images(), &|name| (name == "GEMINI_API_KEY").then(|| "g".to_owned()));
        assert_eq!(
            listed(&both, "Gemini")[1],
            json!({ "name": "Gemini", "kind": "Google Gemini", "model": gemini::MODEL, "default": true, "transparent": false, "edits": true })
        );
        assert!(Kind::Gemini.edits() && !Kind::OpenAi.edits());
    }

    #[test]
    fn a_tool_call_becomes_a_request_or_says_what_is_wrong() {
        let ready = ready(&images(), &|name| (name == "GEMINI_API_KEY").then(|| "g".to_owned()));
        let asked = request(&json!({ "prompt": " A fox ", "aspect": "16:9", "size": "large", "transparent": true }), &ready[0]);
        assert_eq!(
            asked,
            Ok(Request {
                prompt: "A fox".to_owned(),
                model: openai::MODEL.to_owned(),
                aspect: Some(Aspect(16, 9)),
                large: true,
                transparent: true,
                sources: Vec::new()
            })
        );
        assert_eq!(
            request(&json!({ "prompt": "A fox" }), &ready[1]).map(|r| (r.aspect, r.large, r.transparent)),
            Ok((Some(Aspect(1, 1)), false, false))
        );
        assert_eq!(request(&json!({ "prompt": "  " }), &ready[0]), Err("`prompt` is missing: say what the picture shows.".to_owned()));
        assert_eq!(
            request(&json!({ "prompt": "x", "aspect": "5:1" }), &ready[0]),
            Err("`aspect` must be one of 1:1, 16:9, 9:16, 4:3, 3:4, 3:2, 2:3.".to_owned())
        );
        assert_eq!(request(&json!({ "prompt": "x", "size": "huge" }), &ready[0]), Err("`size` must be \"normal\" or \"large\".".to_owned()));
        // Human: an argument of the wrong type is a mistake to tell the model about, not a default to use.
        assert_eq!(
            request(&json!({ "prompt": "x", "aspect": 1.5 }), &ready[0]),
            Err("`aspect` must be one of 1:1, 16:9, 9:16, 4:3, 3:4, 3:2, 2:3.".to_owned())
        );
        assert_eq!(request(&json!({ "prompt": "x", "size": 2 }), &ready[0]), Err("`size` must be \"normal\" or \"large\".".to_owned()));
        assert_eq!(request(&json!({ "prompt": "x", "transparent": "yes" }), &ready[0]), Err("`transparent` must be true or false.".to_owned()));
        assert_eq!(
            request(&json!({ "prompt": "x", "aspect": null, "size": null, "transparent": null }), &ready[0]).map(|r| (
                r.aspect,
                r.large,
                r.transparent
            )),
            Ok((Some(Aspect(1, 1)), false, false))
        );
        assert_eq!(
            request(&json!({ "prompt": "x", "transparent": true }), &ready[1]),
            Err("Gemini can't make a transparent background. Leave `transparent` out, or use a service that can (get_state lists them).".to_owned())
        );
    }

    #[test]
    fn an_edit_call_names_its_pictures_or_says_what_is_wrong() {
        let named = |args: Value| super::named(&args);
        assert_eq!(named(json!({ "layer": "hero" })), Ok(Named { change: Some(Pick::Layer("hero".to_owned())), references: vec![] }));
        assert_eq!(
            named(json!({ "layers": ["a", "b"], "references": ["logo"] })),
            Ok(Named { change: Some(Pick::Drawn(vec!["a".to_owned(), "b".to_owned()])), references: vec![Pick::Layer("logo".to_owned())] })
        );
        let full = if cfg!(windows) { "C:/pictures/a.png" } else { "/pictures/a.png" };
        assert_eq!(named(json!({ "file": full })).map(|n| n.change), Ok(Some(Pick::File(PathBuf::from(full)))));
        assert_eq!(named(json!({ "references": [full] })), Ok(Named { change: None, references: vec![Pick::File(PathBuf::from(full))] }));
        assert_eq!(
            named(json!({ "layer": "a", "file": full })),
            Err("Give one of `layer`, `layers` and `file`, not several: one picture is changed at a time.".to_owned())
        );
        assert_eq!(named(json!({ "file": "a.png" })), Err("`file` must be a full path, starting at the root: \"a.png\" isn't.".to_owned()));
        assert_eq!(
            named(json!({ "references": ["a", "b", "c", "d", "e"] })),
            Err("At most 4 reference pictures can be sent; 5 were named.".to_owned())
        );
        assert_eq!(named(json!({ "layers": [] })), Err("`layers` is empty: name the layers to draw.".to_owned()));
        // Human: a share is refused on every platform, however it is written, as the file or a reference.
        for share in [r"\\server\pictures\a.png", "//server/pictures/a.png"] {
            assert_eq!(named(json!({ "file": share })), Err(ON_A_SHARE.to_owned()), "{share}");
            assert_eq!(named(json!({ "layer": "a", "references": [format!(" {share}")] })), Err(ON_A_SHARE.to_owned()), "{share}");
        }
        assert_eq!(ON_A_SHARE, "Pictures on a network share can't be sent; copy it to this computer first.");
        assert_eq!(
            named(json!({})),
            Err("There is no picture to change and none to go by. Name `layer`, `layers` or `file`, or use generate_image to make a picture from words alone."
                .to_owned())
        );
        // Human: arguments sent as null are left out; ones of the wrong type are mistakes to tell the model about.
        assert_eq!(named(json!({ "layer": "hero", "file": null, "layers": null })).map(|n| n.change), Ok(Some(Pick::Layer("hero".to_owned()))));
        assert_eq!(named(json!({ "layer": 3 })), Err("`layer` must be a layer's id.".to_owned()));
        assert_eq!(named(json!({ "layers": "a" })), Err("`layers` must be a list of layer ids.".to_owned()));
        assert_eq!(
            named(json!({ "layer": "a", "references": [1] })),
            Err("`references` must be a list of layer ids and full file paths.".to_owned())
        );
    }

    #[test]
    fn an_edit_request_takes_the_instruction_and_the_pictures_own_shape() {
        let ready = ready(&images(), &|name| (name == "GEMINI_API_KEY").then(|| "g".to_owned()));
        let (openai, gemini) = (&ready[0], &ready[1]);
        let asked = edit_request(&json!({ "instruction": " Make it dusk ", "size": "large" }), gemini, true).unwrap();
        assert_eq!((asked.prompt.as_str(), asked.aspect, asked.large, asked.transparent), ("Make it dusk", None, true, false));
        assert_eq!(edit_request(&json!({ "instruction": "x", "aspect": "16:9" }), gemini, true).map(|r| r.aspect), Ok(Some(Aspect(16, 9))));
        // Human: references alone have no shape to keep: square, as a generation.
        assert_eq!(edit_request(&json!({ "instruction": "x" }), gemini, false).map(|r| r.aspect), Ok(Some(Aspect(1, 1))));
        assert_eq!(edit_request(&json!({ "instruction": "x", "aspect": "3:4" }), gemini, false).map(|r| r.aspect), Ok(Some(Aspect(3, 4))));
        assert_eq!(edit_request(&json!({}), gemini, true), Err("`instruction` is missing: say what to change.".to_owned()));
        assert_eq!(
            edit_request(&json!({ "instruction": "x" }), openai, true),
            Err("Studio is an OpenAI Images service, and editing isn't possible with that kind yet; use a Google Gemini service (get_state lists which can edit).".to_owned())
        );
        assert_eq!(
            edit_request(&json!({ "instruction": "x", "aspect": "5:1" }), gemini, true),
            Err("`aspect` must be one of 1:1, 16:9, 9:16, 4:3, 3:4, 3:2, 2:3.".to_owned())
        );
    }

    #[test]
    fn an_edit_goes_to_the_default_when_it_can_edit_else_the_first_that_can() {
        let saved = images().services;
        let both = ready(&images(), &|name| (name == "GEMINI_API_KEY").then(|| "g".to_owned()));
        let picked = |ready: &[Ready], asked: Option<&str>, default: &str| pick_editing(ready, &saved, asked, default).map(|r| r.name.clone());
        // Studio, the first, can't edit: Gemini is taken, whether the default is Studio or no default is set.
        assert_eq!(picked(&both, None, ""), Ok("Gemini".to_owned()));
        assert_eq!(picked(&both, None, "Studio"), Ok("Gemini".to_owned()));
        assert_eq!(picked(&both, None, "Gemini"), Ok("Gemini".to_owned()));
        // One asked for by name is that one, even if it can't edit: the request then says why.
        assert_eq!(picked(&both, Some("Studio"), "Gemini"), Ok("Studio".to_owned()));
        let only_openai = ready(&images(), &|_| None);
        assert_eq!(picked(&only_openai, None, ""), Err(NO_EDITING.to_owned()));
        assert_eq!(picked(&[], None, ""), Err(NO_SERVICE.to_owned()));
        assert_eq!(
            NO_EDITING,
            "None of the image services can edit pictures. Add a Google Gemini service in Settings → Images, or start Quill with GEMINI_API_KEY set."
        );
    }

    #[test]
    fn the_settings_start_with_no_service_and_the_assistant_allowed() {
        let images = Images::default();
        assert!(images.services.is_empty() && images.default.is_empty() && images.assistant);
    }
}
