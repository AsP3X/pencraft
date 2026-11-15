//! What is asked of an image service and what can go wrong, the same for every provider.

use std::time::Duration;

use serde_json::Value;

/// The aspect ratios a picture can be asked for in, as width to height.
pub const ASPECTS: [&str; 7] = ["1:1", "16:9", "9:16", "4:3", "3:4", "3:2", "2:3"];
/// How long a service may take to answer; a little under a job's own limit ([`super::jobs::LIMIT`]).
const ANSWER_TIME: Duration = Duration::from_secs(95);
/// The largest answer taken, in bytes: a large picture as base64 is well over ureq's own 10 MB limit.
const MOST_ANSWER: u64 = 64 * 1024 * 1024;
/// How much of an answer that isn't JSON is shown.
const MOST_SAID: usize = 300;

/// A picture's shape: width to height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aspect(pub u32, pub u32);

impl Default for Aspect {
    fn default() -> Self {
        Aspect(1, 1)
    }
}

impl Aspect {
    /// The aspect `text` names ("16:9"), when it is one of [`ASPECTS`].
    #[must_use]
    pub fn parse(text: &str) -> Option<Aspect> {
        let text = text.trim();
        let (width, height) = ASPECTS.contains(&text).then(|| text.split_once(':')).flatten()?;
        Some(Aspect(width.parse().ok()?, height.parse().ok()?))
    }

    /// "16:9".
    #[must_use]
    pub fn text(self) -> String {
        format!("{}:{}", self.0, self.1)
    }
}

/// A picture sent with a request: the one to change, or one to go by.
#[derive(Clone, PartialEq, Eq)]
pub struct Source {
    /// The bytes of a JPEG or PNG file.
    pub bytes: Vec<u8>,
    /// "image/jpeg" or "image/png".
    pub kind: &'static str,
    /// This is the picture to change; the others are references.
    pub change: bool,
}

// Human: a picture is shown by its kind and size, so the user's pictures never end up in a log (a `Request`
// prints its sources this way too).
impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Source({}, {} bytes{})", self.kind, self.bytes.len(), if self.change { ", to change" } else { "" })
    }
}

/// One picture to make.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// What the picture shows, or for an edit what to change.
    pub prompt: String,
    /// The model, as the service names it.
    pub model: String,
    /// Its shape; `None` for the shape of the picture to change (a plain generation always has one).
    pub aspect: Option<Aspect>,
    /// About 2048 pixels on the long side instead of about 1024.
    pub large: bool,
    /// A transparent background.
    pub transparent: bool,
    /// The pictures sent with it, the one to change first; empty for a plain generation.
    pub sources: Vec<Source>,
}

/// A picture a service made: the bytes of a PNG, JPEG or WebP file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Made(pub Vec<u8>);

/// Why a service made no picture. Each holds the service's own words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The service won't make this picture.
    Refused(String),
    /// The service didn't accept the key.
    Key(String),
    /// The key couldn't be found at all; the words say where it was looked for and what to do.
    NoKey(String),
    /// Too many requests, or the account's quota is used up.
    Limit(String),
    /// The service wasn't reached or didn't answer in time.
    Network(String),
    /// Anything else.
    Other(String),
}

impl Failure {
    /// The failure an answer with `status` and the body `said` is.
    #[must_use]
    pub fn of(status: u16, said: &str) -> Failure {
        let message = message_of(said);
        match status {
            401 | 403 => Failure::Key(message),
            429 => Failure::Limit(message),
            _ => Failure::Other(format!("{status}: {message}")),
        }
    }

    /// The failure as a sentence for a person or a model, naming the service.
    #[must_use]
    pub fn sentence(&self, service: &str) -> String {
        match self {
            Failure::Refused(why) => format!("{service} refused to make this picture: {}", ended(why)),
            Failure::Key(why) => format!("{service} didn't accept its API key: {} Check the key in Settings → Images.", ended(why)),
            Failure::NoKey(why) => format!("{service}'s API key couldn't be found: {}", ended(why)),
            Failure::Limit(why) => format!("{service} has reached a limit: {}", ended(why)),
            Failure::Network(why) => format!("{service} couldn't be reached or took too long: {}", ended(why)),
            Failure::Other(why) => format!("{service} answered with an error: {}", ended(why)),
        }
    }
}

/// A service's words `why` as the end of a sentence: with a full stop when they have none, and "no reason
/// given" when there are none.
fn ended(why: &str) -> String {
    let why = why.trim();
    if why.is_empty() {
        "no reason given.".to_owned()
    } else if why.ends_with(['.', '!', '?']) {
        why.to_owned()
    } else {
        format!("{why}.")
    }
}

/// What a service said went wrong: the `error.message` of a JSON answer, or the start of anything else.
#[must_use]
pub fn message_of(said: &str) -> String {
    // Human: Google wraps some errors (a key it doesn't know) in a list of one; the error is looked for there too.
    serde_json::from_str::<Value>(said)
        .ok()
        .and_then(|value| {
            let error = value.pointer("/error/message").or_else(|| value.pointer("/0/error/message"));
            error.and_then(Value::as_str).map(str::to_owned)
        })
        .unwrap_or_else(|| said.chars().take(MOST_SAID).collect())
}

/// Sends `body` as JSON to `url` and gives the answer's status and text.
///
/// # Errors
/// [`Failure::Network`] when the service wasn't reached, didn't answer in time, or its answer couldn't be read.
// Human: the global limit covers the whole call, reading the answer's body included: a large picture is tens of
// megabytes, and a service that sends it slowly must not keep the job past its own limit.
// Agent: BLOCKS for up to ANSWER_TIME; call it on a worker thread only. `headers` carry the key: never log them.
pub fn post(url: &str, headers: &[(&str, &str)], body: &Value) -> Result<(u16, String), Failure> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_global(Some(ANSWER_TIME))
        .build()
        .into();
    let mut request = agent.post(url).header("content-type", "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut response = request.send(body.to_string().as_str()).map_err(|error| Failure::Network(error.to_string()))?;
    let status = response.status().as_u16();
    let said = response.body_mut().with_config().limit(MOST_ANSWER).read_to_string().map_err(|error| Failure::Network(error.to_string()))?;
    Ok((status, said))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_aspect_is_one_of_the_known_ones() {
        assert_eq!(Aspect::parse("16:9"), Some(Aspect(16, 9)));
        assert_eq!(Aspect::parse(" 1:1 "), Some(Aspect(1, 1)));
        assert_eq!(Aspect::parse("7:5"), None);
        assert_eq!(Aspect::parse("wide"), None);
        assert_eq!(Aspect(3, 4).text(), "3:4");
        assert_eq!(Aspect::default(), Aspect(1, 1));
    }

    #[test]
    fn a_status_becomes_the_failure_a_person_can_act_on() {
        let said = r#"{"error":{"message":"Bad key."}}"#;
        assert_eq!(Failure::of(401, said), Failure::Key("Bad key.".to_owned()));
        assert_eq!(Failure::of(403, said), Failure::Key("Bad key.".to_owned()));
        assert_eq!(Failure::of(429, said), Failure::Limit("Bad key.".to_owned()));
        assert_eq!(Failure::of(500, "<html>oops</html>"), Failure::Other("500: <html>oops</html>".to_owned()));
    }

    #[test]
    fn a_failure_names_the_service_and_what_to_do() {
        assert_eq!(
            Failure::Key("Bad key.".to_owned()).sentence("My Gemini"),
            "My Gemini didn't accept its API key: Bad key. Check the key in Settings → Images."
        );
        assert_eq!(Failure::Refused("No people.".to_owned()).sentence("My Gemini"), "My Gemini refused to make this picture: No people.");
        assert_eq!(Failure::Limit("Slow down.".to_owned()).sentence("X"), "X has reached a limit: Slow down.");
        assert_eq!(Failure::Network("timed out".to_owned()).sentence("X"), "X couldn't be reached or took too long: timed out.");
        assert_eq!(Failure::Other("500: oops".to_owned()).sentence("X"), "X answered with an error: 500: oops.");
    }

    #[test]
    fn the_services_words_always_end_a_sentence() {
        assert_eq!(Failure::Limit(String::new()).sentence("X"), "X has reached a limit: no reason given.");
        assert_eq!(Failure::Limit("  ".to_owned()).sentence("X"), "X has reached a limit: no reason given.");
        assert_eq!(Failure::Key("Bad key".to_owned()).sentence("X"), "X didn't accept its API key: Bad key. Check the key in Settings → Images.");
        assert_eq!(Failure::Key(String::new()).sentence("X"), "X didn't accept its API key: no reason given. Check the key in Settings → Images.");
        assert_eq!(Failure::Refused("Why not?".to_owned()).sentence("X"), "X refused to make this picture: Why not?");
        assert_eq!(Failure::NoKey("it is gone".to_owned()).sentence("X"), "X's API key couldn't be found: it is gone.");
    }

    #[test]
    fn a_picture_sent_is_printed_by_its_kind_and_size_never_its_bytes() {
        let source = Source { bytes: b"secret".to_vec(), kind: "image/png", change: true };
        assert_eq!(format!("{source:?}"), "Source(image/png, 6 bytes, to change)");
        let request = Request {
            prompt: "x".to_owned(),
            model: "m".to_owned(),
            aspect: None,
            large: false,
            transparent: false,
            sources: vec![source, Source { bytes: vec![1], kind: "image/jpeg", change: false }],
        };
        let printed = format!("{request:?}");
        assert!(printed.contains("Source(image/jpeg, 1 bytes)") && !printed.contains("115"), "{printed}");
    }
}
