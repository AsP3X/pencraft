//! The providers the settings window knows by name: where each is reached and which API it speaks, so that
//! connecting one takes only a key. Also asks a provider which models it has.
//!
//! The addresses are the ones each provider documents for OpenAI-compatible clients (looked up in November
//! 2026; Meta's is from third parties, its own page was gone). The user can change them in the form.

use craft_ai_core::SecretString;
use serde_json::Value;

use super::{COMPLETIONS_API, MESSAGES_API, MODELS, Wire, api};

/// What is known about a provider beforehand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preset {
    /// The API type, as the settings window names it.
    pub api: &'static str,
    /// Where it is reached.
    pub base_url: &'static str,
    /// Where it lists its models, when that isn't `models` under the base address.
    models_url: Option<&'static str>,
}

const fn completions(base_url: &'static str) -> Preset {
    Preset { api: COMPLETIONS_API, base_url, models_url: None }
}

/// The known providers that take an API key, by the name the settings window shows.
// Human: Cursor and GitHub Copilot are not here: neither has an API that another app can use with a key.
const PRESETS: [(&str, Preset); 14] = [
    ("OpenAI ChatGPT", completions("https://api.openai.com/v1")),
    ("Anthropic Claude Code", Preset { api: MESSAGES_API, base_url: "https://api.anthropic.com", models_url: None }),
    ("xAI", completions("https://api.x.ai/v1")),
    ("Google Gemini", completions("https://generativelanguage.googleapis.com/v1beta/openai")),
    (
        "OpenCode Go",
        Preset {
            api: COMPLETIONS_API,
            base_url: "https://opencode.ai/inference/go/openai/v1",
            models_url: Some("https://opencode.ai/inference/go/v1/models"),
        },
    ),
    ("Open Router", completions("https://openrouter.ai/api/v1")),
    ("Meta", completions("https://api.llama.com/compat/v1")),
    ("Moonshot AI", completions("https://api.moonshot.ai/v1")),
    ("Kimi For Coding", completions("https://api.kimi.com/coding/v1")),
    ("Deepseek", completions("https://api.deepseek.com")),
    ("Together AI", completions("https://api.together.ai/v1")),
    ("Fireworks AI", completions("https://api.fireworks.ai/inference/v1")),
    ("Z.AI", completions("https://api.z.ai/api/paas/v4")),
    (
        "OpenCode Zen",
        Preset {
            api: COMPLETIONS_API,
            base_url: "https://opencode.ai/inference/openai/v1",
            models_url: Some("https://opencode.ai/inference/v1/models"),
        },
    ),
];

/// What is known about the provider called `name`; `None` for one that can't be connected with a key.
#[must_use]
pub fn preset(name: &str) -> Option<Preset> {
    PRESETS.iter().find(|(known, _)| *known == name).map(|(_, preset)| *preset)
}

/// The models a new connection to the provider called `name` starts with; none where only the provider knows.
// Human: model names change faster than this app; only Anthropic's are known here (the chat offers them
// anyway), every other list is loaded from the provider.
#[must_use]
pub fn starting_models(name: &str) -> Vec<&'static str> {
    if name == "Anthropic Claude Code" { MODELS.iter().map(|(id, _)| *id).collect() } else { Vec::new() }
}

/// Where a provider with the API type `api`, reached at `base`, lists its models.
fn models_url(api: &str, base: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    if let Some(url) = PRESETS.iter().find(|(_, preset)| preset.base_url == base).and_then(|(_, preset)| preset.models_url) {
        return url.to_owned();
    }
    if Wire::of(api) == Some(Wire::Messages) && !base.ends_with("/v1") { format!("{base}/v1/models") } else { format!("{base}/models") }
}

/// The model names in a provider's answer: the `data` list every OpenAI-compatible API gives, or a bare list.
fn ids(body: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(body) else { return Vec::new() };
    let list = value.get("data").or_else(|| value.get("models")).unwrap_or(&value);
    let mut ids: Vec<String> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|model| model.get("id").or_else(|| model.get("name")).and_then(Value::as_str).or_else(|| model.as_str()))
        .map(str::to_owned)
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// Asks the provider which models it has.
///
/// # Errors
/// A sentence for the user when the provider couldn't be reached, refused, or listed nothing.
// Agent: BLOCKS on the network; call it from a thread of its own.
pub fn list_models(api: &str, base: &str, key: Option<&SecretString>) -> Result<Vec<String>, String> {
    let url = models_url(api, base);
    let mut request = api::agent().get(&url);
    match (Wire::of(api), key) {
        (Some(Wire::Messages), Some(key)) => request = request.header("anthropic-version", api::VERSION).header("x-api-key", key.expose()),
        (_, Some(key)) => request = request.header("authorization", format!("Bearer {}", key.expose())),
        (_, None) => {}
    }
    let mut response = request.call().map_err(|error| format!("The provider couldn't be reached: {error}"))?;
    let status = response.status().as_u16();
    let body = response.body_mut().read_to_string().unwrap_or_default();
    if status != 200 {
        return Err(api::refusal_of(status, &body));
    }
    let found = ids(&body);
    if found.is_empty() { Err("The provider listed no models; type their names in by hand.".to_owned()) } else { Ok(found) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_providers_come_with_their_address_and_api() {
        assert_eq!(preset("xAI"), Some(completions("https://api.x.ai/v1")));
        assert_eq!(preset("Anthropic Claude Code").map(|p| p.api), Some(MESSAGES_API));
        assert!(preset("Cursor").is_none() && preset("GitHub Copilot").is_none() && preset("Mine").is_none());
        // Every address is encrypted and written without a slash at its end, as the form and the chat expect.
        assert!(PRESETS.iter().all(|(_, p)| p.base_url.starts_with("https://") && !p.base_url.ends_with('/') && Wire::of(p.api).is_some()));
        assert_eq!(starting_models("Anthropic Claude Code").len(), MODELS.len());
        assert_eq!(starting_models("xAI"), [] as [&str; 0]);
    }

    #[test]
    fn models_are_listed_where_each_api_has_them() {
        assert_eq!(models_url(COMPLETIONS_API, "https://api.example.com/v1/"), "https://api.example.com/v1/models");
        assert_eq!(models_url(MESSAGES_API, "https://api.anthropic.com"), "https://api.anthropic.com/v1/models");
        assert_eq!(models_url(MESSAGES_API, "https://llm.example.com/v1"), "https://llm.example.com/v1/models");
        assert_eq!(models_url(COMPLETIONS_API, "https://opencode.ai/inference/openai/v1"), "https://opencode.ai/inference/v1/models");
        assert_eq!(ids(r#"{"object":"list","data":[{"id":"b"},{"id":"a"},{"id":"b"},{"object":"model"}]}"#), ["a", "b"]);
        assert_eq!(ids(r#"[{"id":"x"},"y"]"#), ["x", "y"]);
        assert_eq!(ids(r#"{"models":[{"name":"models/g"}]}"#), ["models/g"]);
        assert!(ids("not json").is_empty() && ids(r#"{"error":{"message":"no"}}"#).is_empty());
    }
}
