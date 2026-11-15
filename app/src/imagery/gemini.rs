//! Google's Gemini image models ("Nano Banana"), through the Interactions API.

use craft_ai_core::SecretString;
use serde_json::{Value, json};

use super::service::{Failure, Made, Request, post};

/// Where the service is reached unless the user gave another address.
pub const BASE: &str = "https://generativelanguage.googleapis.com";
/// The model a new service starts with: the fastest and cheapest ("Nano Banana 2 Lite").
pub const MODEL: &str = "gemini-3.1-flash-lite-image";
/// A model that also makes large pictures ("Nano Banana 2"); the lite one makes the normal size only.
const LARGE_MODEL: &str = "gemini-3.1-flash-image";
/// The reason Google gives in an error's details when it doesn't know the key.
const BAD_KEY: &str = "API_KEY_INVALID";

/// The address requests go to.
fn endpoint(base: &str) -> String {
    format!("{}/v1beta/interactions", base.trim().trim_end_matches('/'))
}

/// Said before the words when a request has both a picture to change and references.
const ROLES: &str = "The first picture is the one to change; the others are references.";

/// The request as the Interactions API takes it.
// Human: the pictures follow the words, the one to change first (`sources::prepare` puts it there). Without an
// aspect the service keeps the shape of the picture to change (live service, 2026-11-15).
fn body(request: &Request) -> Value {
    let sources = &request.sources;
    let roles = sources.iter().any(|source| source.change) && sources.iter().any(|source| !source.change);
    let text = if roles { format!("{ROLES} {}", request.prompt) } else { request.prompt.clone() };
    let mut input = vec![json!({ "type": "text", "text": text })];
    input.extend(sources.iter().map(|source| json!({ "type": "image", "mime_type": source.kind, "data": crate::agent::base64(&source.bytes) })));
    let mut format = serde_json::Map::new();
    format.insert("type".to_owned(), json!("image"));
    // Human: asked of the live service on 2026-11-14, the lite model makes JPEG only ("The value
    // 'image/png' is not supported"), and every model makes JPEG.
    format.insert("mime_type".to_owned(), json!("image/jpeg"));
    if let Some(aspect) = request.aspect {
        format.insert("aspect_ratio".to_owned(), json!(aspect.text()));
    }
    format.insert("image_size".to_owned(), json!(if request.large { "2K" } else { "1K" }));
    json!({ "model": request.model, "input": input, "response_format": format })
}

/// The picture in an answer with `status` and the body `said`.
// Human: the answer is a list of steps; the model's output is the step of type "model_output", and the
// picture is the block of type "image" in it, as base64. An answer that came through but holds no picture is
// the model declining, and what it wrote instead is the reason.
fn read(status: u16, said: &str) -> Result<Made, Failure> {
    if !(200..300).contains(&status) {
        // Human: Google answers a key it doesn't know with 400, like any bad request, and says which it is
        // only in the error's reason.
        if said.contains(BAD_KEY) {
            return Err(Failure::Key(super::service::message_of(said)));
        }
        // Human: a picture the model won't make from the pictures it was given comes as 400 with a code that
        // starts with `image_` ("image_recitation", live service, 2026-11-15): a refusal, in the service's words.
        let code =
            serde_json::from_str::<Value>(said).ok().and_then(|answer| answer.pointer("/error/code").and_then(Value::as_str).map(str::to_owned));
        if status == 400 && code.is_some_and(|code| code.starts_with("image_")) {
            return Err(Failure::Refused(super::service::message_of(said)));
        }
        return Err(Failure::of(status, said));
    }
    let answer: Value = serde_json::from_str(said)
        .map_err(|_| Failure::Other(format!("The answer couldn't be read: {}", said.chars().take(200).collect::<String>())))?;
    let blocks: Vec<&Value> = answer["steps"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|step| step["type"] == "model_output")
        .flat_map(|step| step["content"].as_array().into_iter().flatten())
        .collect();
    if let Some(data) = blocks.iter().find(|block| block["type"] == "image").and_then(|block| block["data"].as_str()) {
        return crate::agent::unbase64(data)
            .filter(|bytes| !bytes.is_empty())
            .map(Made)
            .ok_or_else(|| Failure::Other("The picture in the answer couldn't be read.".to_owned()));
    }
    let words: Vec<&str> = blocks.iter().filter_map(|block| block["text"].as_str()).collect();
    Err(Failure::Refused(if words.is_empty() { "It sent no picture and no reason.".to_owned() } else { words.join(" ") }))
}

/// Asks the service at `base` for the picture `request` describes.
///
/// # Errors
/// Why there is no picture.
// Agent: BLOCKS for one answer (up to 95 s).
pub fn make(base: &str, request: &Request, key: &SecretString) -> Result<Made, Failure> {
    let (status, said) = post(&endpoint(base), &[("x-goog-api-key", key.expose())], &body(request))?;
    read(status, &said).map_err(|failure| explained(failure, status, request))
}

/// A failure with what the service left unsaid.
// Human: asked for a large picture, the lite model answers 404 "Requested entity was not found." and nothing
// more (live service, 2026-11-14); the larger models make one. The same answer comes for a model name the
// service doesn't know, so both are said.
fn explained(failure: Failure, status: u16, request: &Request) -> Failure {
    match failure {
        Failure::Other(_) if status == 404 && request.large => Failure::Other(format!(
            "The model {} can't make a large picture, or the service doesn't know it. Ask for the normal size, or set a model that can, such as {LARGE_MODEL}, in Settings → Images.",
            request.model
        )),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::imagery::service::{Aspect, Source};

    fn request() -> Request {
        Request {
            prompt: "A red fox".to_owned(),
            model: "gemini-3.1-flash-lite-image".to_owned(),
            aspect: Some(Aspect(16, 9)),
            large: false,
            transparent: false,
            sources: Vec::new(),
        }
    }

    #[test]
    fn the_body_names_the_model_the_words_and_the_shape() {
        assert_eq!(
            body(&request()),
            json!({
                "model": "gemini-3.1-flash-lite-image",
                "input": [{ "type": "text", "text": "A red fox" }],
                "response_format": { "type": "image", "mime_type": "image/jpeg", "aspect_ratio": "16:9", "image_size": "1K" }
            })
        );
        assert_eq!(body(&Request { large: true, ..request() })["response_format"]["image_size"], "2K");
    }

    #[test]
    fn the_picture_is_read_out_of_the_models_output() {
        let said = json!({ "id": "i1", "steps": [
            { "type": "thought", "content": [{ "type": "text", "text": "thinking" }] },
            { "type": "model_output", "content": [{ "type": "text", "text": "Here you go." }, { "type": "image", "data": "Zm9v" }] }
        ] })
        .to_string();
        assert_eq!(read(200, &said), Ok(Made(b"foo".to_vec())));
    }

    #[test]
    fn an_answer_without_a_picture_is_a_refusal_in_the_models_words() {
        let said = json!({ "steps": [{ "type": "model_output", "content": [{ "type": "text", "text": "I can't draw that." }] }] }).to_string();
        assert_eq!(read(200, &said), Err(Failure::Refused("I can't draw that.".to_owned())));
        assert_eq!(read(200, "{}"), Err(Failure::Refused("It sent no picture and no reason.".to_owned())));
    }

    #[test]
    fn errors_and_nonsense_are_failures() {
        assert_eq!(read(403, r#"{"error":{"message":"API key not valid."}}"#), Err(Failure::Key("API key not valid.".to_owned())));
        // Human: the three answers below are the live service's own, taken on 2026-11-14.
        let unknown_key = r#"[{"error":{"code":400,"message":"API key not valid. Please pass a valid API key.","status":"INVALID_ARGUMENT","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"API_KEY_INVALID","domain":"googleapis.com"}]}}]"#;
        assert_eq!(read(400, unknown_key), Err(Failure::Key("API key not valid. Please pass a valid API key.".to_owned())));
        let free_tier = r#"{"error":{"message":"Rate limit exceeded for model gemini-3.1-flash-lite-image (limit: 0 input tokens per minute on Free Tier). Please upgrade your tier at https://ai.dev/rate-limit.","code":"too_many_requests"}}"#;
        assert!(matches!(read(429, free_tier), Err(Failure::Limit(said)) if said.contains("Free Tier")));
        let no_model = r#"{"error":{"message":"Model 'no-such-model' not found.","code":"not_found"}}"#;
        assert_eq!(read(404, no_model), Err(Failure::Other("404: Model 'no-such-model' not found.".to_owned())));
        assert_eq!(read(200, "not json"), Err(Failure::Other("The answer couldn't be read: not json".to_owned())));
        let broken = json!({ "steps": [{ "type": "model_output", "content": [{ "type": "image", "data": "!!!" }] }] }).to_string();
        assert_eq!(read(200, &broken), Err(Failure::Other("The picture in the answer couldn't be read.".to_owned())));
    }

    #[test]
    fn a_large_picture_the_model_cant_make_is_said_with_what_to_do() {
        let not_found = || read(404, r#"{"error":{"message":"Requested entity was not found.","code":"not_found"}}"#).unwrap_err();
        let large = Request { large: true, ..request() };
        assert_eq!(
            explained(not_found(), 404, &large),
            Failure::Other(
                "The model gemini-3.1-flash-lite-image can't make a large picture, or the service doesn't know it. Ask for the normal size, or set a model that can, such as gemini-3.1-flash-image, in Settings → Images."
                    .to_owned()
            )
        );
        // The normal size, and any other failure, are told as the service told them.
        assert_eq!(explained(not_found(), 404, &request()), not_found());
        assert_eq!(explained(Failure::Limit("Slow down.".to_owned()), 429, &large), Failure::Limit("Slow down.".to_owned()));
    }

    #[test]
    fn pictures_follow_the_words_and_roles_are_said_only_when_both_are_there() {
        let source = |change| Source { bytes: b"foo".to_vec(), kind: "image/jpeg", change };
        let one = body(&Request { sources: vec![source(true)], aspect: None, ..request() });
        assert_eq!(one["input"], json!([{ "type": "text", "text": "A red fox" }, { "type": "image", "mime_type": "image/jpeg", "data": "Zm9v" }]));
        assert!(one["response_format"].get("aspect_ratio").is_none(), "no shape asked for: the picture's own");
        let both = body(&Request { sources: vec![source(true), source(false)], ..request() });
        assert_eq!(both["input"][0]["text"], "The first picture is the one to change; the others are references. A red fox");
        assert_eq!(both["input"].as_array().unwrap().len(), 3);
        let only_references = body(&Request { sources: vec![source(false)], ..request() });
        assert_eq!(only_references["input"][0]["text"], "A red fox");
    }

    #[test]
    fn a_picture_the_service_wont_make_from_the_pictures_given_is_a_refusal() {
        // Human: the live service's own answer, taken on 2026-11-15 (research_notes/imagery/gemini-edit.md).
        let recitation = r#"{"error":{"message":"Image generation blocked due to copyright/recitation. Please modify your input and retry.","code":"image_recitation"}}"#;
        assert_eq!(
            read(400, recitation),
            Err(Failure::Refused("Image generation blocked due to copyright/recitation. Please modify your input and retry.".to_owned()))
        );
        // Another 400 is still an error, not a refusal.
        let invalid = r#"{"error":{"message":"Invalid value.","code":"invalid_request"}}"#;
        assert_eq!(read(400, invalid), Err(Failure::Other("400: Invalid value.".to_owned())));
    }

    #[test]
    fn the_address_takes_a_base_with_or_without_a_slash() {
        assert_eq!(endpoint("https://generativelanguage.googleapis.com/"), "https://generativelanguage.googleapis.com/v1beta/interactions");
        assert_eq!(endpoint(BASE), "https://generativelanguage.googleapis.com/v1beta/interactions");
    }
}
