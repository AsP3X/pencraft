//! `OpenAI`'s image models, and every service that speaks the same Images API.

use craft_ai_core::SecretString;
use serde_json::{Value, json};

use super::service::{Aspect, Failure, Made, Request, post};

/// Where the service is reached unless the user gave another address.
pub const BASE: &str = "https://api.openai.com";
/// The model a new service starts with.
pub const MODEL: &str = "gpt-image-2.5-sunburst";
/// The long side of a picture in pixels: normal and large.
const SIDES: (u32, u32) = (1024, 2048);
/// Both edges are a multiple of this, as the API asks.
const STEP: u32 = 16;

/// The address requests go to. A base may end in `/v1` or not.
fn endpoint(base: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    let base = base.strip_suffix("/v1").unwrap_or(base);
    format!("{base}/v1/images/generations")
}

/// "1024x576": the long side as asked, the short one from the aspect, rounded to the API's step.
fn size(aspect: Aspect, large: bool) -> String {
    let long = if large { SIDES.1 } else { SIDES.0 };
    let (wide, tall) = (aspect.0.max(1), aspect.1.max(1));
    let short = |less: u32, more: u32| ((long * less + more * STEP / 2) / more / STEP * STEP).max(STEP);
    if wide >= tall { format!("{long}x{}", short(tall, wide)) } else { format!("{}x{long}", short(wide, tall)) }
}

/// Whether `request` is one this API is asked at all: one with pictures to send is refused before anything is
/// sent.
// Human: OpenAI's edit endpoint is documented in two forms that contradict each other, and there was no key to
// tell which is true (stage 2's spec); editing with it waits for a live check.
fn asked(request: &Request) -> Result<(), Failure> {
    if request.sources.is_empty() {
        Ok(())
    } else {
        Err(Failure::Other("Editing isn't possible with this kind of service yet; use a Google Gemini service.".to_owned()))
    }
}

/// The request as the Images API takes it.
fn body(request: &Request) -> Value {
    json!({
        "model": request.model,
        "prompt": request.prompt,
        "size": size(request.aspect.unwrap_or_default(), request.large),
        "quality": "auto",
        "background": if request.transparent { "transparent" } else { "auto" },
        "output_format": "png",
        "n": 1,
    })
}

/// The picture in an answer with `status` and the body `said`.
fn read(status: u16, said: &str) -> Result<Made, Failure> {
    if !(200..300).contains(&status) {
        let answer: Value = serde_json::from_str(said).unwrap_or(Value::Null);
        // Human: the service says a description broke its rules with this code; that is a refusal the person
        // can act on by wording it differently, not a fault.
        if answer.pointer("/error/code").and_then(Value::as_str) == Some("moderation_blocked") {
            return Err(Failure::Refused(super::service::message_of(said)));
        }
        return Err(Failure::of(status, said));
    }
    let answer: Value = serde_json::from_str(said)
        .map_err(|_| Failure::Other(format!("The answer couldn't be read: {}", said.chars().take(200).collect::<String>())))?;
    let data = answer.pointer("/data/0/b64_json").and_then(Value::as_str).ok_or_else(|| Failure::Other("The answer holds no picture.".to_owned()))?;
    crate::agent::unbase64(data)
        .filter(|bytes| !bytes.is_empty())
        .map(Made)
        .ok_or_else(|| Failure::Other("The picture in the answer couldn't be read.".to_owned()))
}

/// Asks the service at `base` for the picture `request` describes.
///
/// # Errors
/// Why there is no picture.
// Agent: BLOCKS for one answer (up to 95 s).
pub fn make(base: &str, request: &Request, key: &SecretString) -> Result<Made, Failure> {
    asked(request)?;
    let bearer = format!("Bearer {}", key.expose());
    let (status, said) = post(&endpoint(base), &[("authorization", bearer.as_str())], &body(request))?;
    read(status, &said)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::imagery::service::{Aspect, Source};

    fn request() -> Request {
        Request {
            prompt: "A red fox".to_owned(),
            model: "gpt-image-2.5-sunburst".to_owned(),
            aspect: Some(Aspect(16, 9)),
            large: false,
            transparent: false,
            sources: Vec::new(),
        }
    }

    #[test]
    fn a_request_with_pictures_is_refused_before_anything_is_sent() {
        assert_eq!(asked(&request()), Ok(()));
        let edit = Request { sources: vec![Source { bytes: b"foo".to_vec(), kind: "image/jpeg", change: true }], ..request() };
        assert_eq!(
            asked(&edit),
            Err(Failure::Other("Editing isn't possible with this kind of service yet; use a Google Gemini service.".to_owned()))
        );
    }

    #[test]
    fn the_size_has_the_long_side_asked_for_and_edges_in_sixteens() {
        assert_eq!(size(Aspect(1, 1), false), "1024x1024");
        assert_eq!(size(Aspect(16, 9), false), "1024x576");
        assert_eq!(size(Aspect(9, 16), false), "576x1024");
        assert_eq!(size(Aspect(3, 2), false), "1024x688");
        assert_eq!(size(Aspect(4, 3), true), "2048x1536");
    }

    #[test]
    fn the_body_names_the_model_the_words_the_size_and_the_background() {
        assert_eq!(
            body(&request()),
            json!({ "model": "gpt-image-2.5-sunburst", "prompt": "A red fox", "size": "1024x576", "quality": "auto", "background": "auto", "output_format": "png", "n": 1 })
        );
        assert_eq!(body(&Request { transparent: true, ..request() })["background"], "transparent");
    }

    #[test]
    fn the_picture_is_the_first_of_the_data() {
        assert_eq!(read(200, &json!({ "created": 1, "data": [{ "b64_json": "Zm9v" }] }).to_string()), Ok(Made(b"foo".to_vec())));
    }

    #[test]
    fn a_blocked_description_is_a_refusal_and_other_errors_keep_their_kind() {
        let blocked =
            json!({ "error": { "message": "Your request was rejected.", "code": "moderation_blocked", "type": "image_generation_user_error" } })
                .to_string();
        assert_eq!(read(400, &blocked), Err(Failure::Refused("Your request was rejected.".to_owned())));
        assert_eq!(read(401, r#"{"error":{"message":"Incorrect API key."}}"#), Err(Failure::Key("Incorrect API key.".to_owned())));
        assert_eq!(read(200, r#"{"data":[]}"#), Err(Failure::Other("The answer holds no picture.".to_owned())));
        assert_eq!(read(200, r#"{"data":[{"b64_json":"!!!"}]}"#), Err(Failure::Other("The picture in the answer couldn't be read.".to_owned())));
    }

    #[test]
    fn the_address_takes_a_base_with_or_without_the_version() {
        assert_eq!(endpoint(BASE), "https://api.openai.com/v1/images/generations");
        assert_eq!(endpoint("https://api.openai.com/v1/"), "https://api.openai.com/v1/images/generations");
        assert_eq!(endpoint("http://localhost:8080"), "http://localhost:8080/v1/images/generations");
    }
}
