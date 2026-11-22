//! The providers the settings window knows by name: where each is reached and which API it speaks, so that
//! connecting one takes only a key. Also asks a provider which models it has.
//!
//! The addresses are the ones each provider documents for OpenAI-compatible clients (looked up in November
//! 2026; Meta's is from third parties, its own page was gone). The user can change them in the form.

use craft_ai_core::SecretString;
use serde_json::Value;

use super::{COMPLETIONS_API, MESSAGES_API, MODELS, OLLAMA_API, Wire, api, ollama};

/// The name of the model server that runs on the user's own computer and takes no key.
pub const OLLAMA: &str = "Ollama";
/// The name of Ollama's hosted service, which runs the models on its own machines and takes a key.
pub const OLLAMA_CLOUD: &str = "Ollama Cloud";
/// Where Ollama answers on the computer it runs on.
const OLLAMA_LOCAL: &str = "http://localhost:11434";

/// Said when Ollama runs on this computer but has no model to offer.
const NO_LOCAL_MODELS: &str =
    "Ollama is running but has no models yet. Download one first, for example with `ollama pull` and a model's name, then load again.";

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

/// The known providers the chat can reach, by the name the settings window shows.
// Human: Cursor and GitHub Copilot are not here: neither has an API that another app can use with a key.
// Ollama is the one that runs on the user's own computer: it takes no key and is reached unencrypted there,
// by its own API, which alone lets the chat say how much of a conversation the model reads.
// Its hosted service speaks the same API at ollama.com and takes a key like the others (looked at in
// November 2026: it lists its models to anyone and answers a chat without a key with 401).
const PRESETS: [(&str, Preset); 16] = [
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
    (OLLAMA, Preset { api: OLLAMA_API, base_url: OLLAMA_LOCAL, models_url: None }),
    (OLLAMA_CLOUD, completions("https://ollama.com/v1")),
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

/// Whether the provider reached at `base` is a known one that answers only with an API key.
#[must_use]
pub fn wants_key(base: &str) -> bool {
    let base = base.trim().trim_end_matches('/');
    PRESETS.iter().any(|(_, preset)| preset.base_url == base && preset.base_url != OLLAMA_LOCAL)
}

/// Whether `base` is where Ollama answers on this computer.
#[must_use]
pub fn is_local_ollama(base: &str) -> bool {
    ollama::root(base) == OLLAMA_LOCAL
}

/// Where a provider with the API type `api`, reached at `base`, lists its models.
fn models_url(api: &str, base: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    if let Some(url) = PRESETS.iter().find(|(_, preset)| preset.base_url == base).and_then(|(_, preset)| preset.models_url) {
        return url.to_owned();
    }
    match Wire::of(api) {
        Some(Wire::Ollama) => format!("{}/api/tags", ollama::root(base)),
        Some(Wire::Messages) if !base.ends_with("/v1") => format!("{base}/v1/models"),
        _ => format!("{base}/models"),
    }
}

/// What a provider says one of its models can take, as far as it says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Limits {
    /// The most tokens of conversation the model reads.
    pub context: Option<u64>,
    /// The most tokens it writes in one answer.
    pub output: Option<u64>,
}

/// A model a provider lists, with what the list says about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offered {
    /// The name the API knows the model by.
    pub id: String,
    /// What the list says the model can take.
    pub limits: Limits,
}

/// The names providers give a model's context window in their lists, likeliest first.
// Human: there is no standard: OpenRouter and Together say `context_length`, Groq `context_window`, Mistral
// `max_context_length`, Anthropic `max_input_tokens`, Google `inputTokenLimit`, vLLM `max_model_len`.
const CONTEXT_NAMES: [&str; 8] = [
    "/context_length",
    "/context_window",
    "/max_context_length",
    "/max_input_tokens",
    "/inputTokenLimit",
    "/input_token_limit",
    "/max_model_len",
    "/top_provider/context_length",
];
/// The names they give the most a model writes in one answer.
const OUTPUT_NAMES: [&str; 7] = [
    "/max_output_tokens",
    "/max_completion_tokens",
    "/outputTokenLimit",
    "/output_token_limit",
    "/top_provider/max_completion_tokens",
    "/max_output_length",
    "/max_tokens",
];

/// What the entry `model` of a provider's list says the model can take.
fn limits_of(model: &Value) -> Limits {
    let first = |names: &[&str]| names.iter().find_map(|name| model.pointer(name).and_then(Value::as_u64).filter(|count| *count > 0));
    Limits { context: first(&CONTEXT_NAMES), output: first(&OUTPUT_NAMES) }
}

/// The models in a provider's answer, by name: the `data` list every OpenAI-compatible API gives, or a bare list.
fn offered(body: &str) -> Vec<Offered> {
    let Ok(value) = serde_json::from_str::<Value>(body) else { return Vec::new() };
    let list = value.get("data").or_else(|| value.get("models")).unwrap_or(&value);
    let mut found: Vec<Offered> = list
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|model| {
            let id = model.get("id").or_else(|| model.get("name")).and_then(Value::as_str).or_else(|| model.as_str())?;
            Some(Offered { id: id.to_owned(), limits: limits_of(model) })
        })
        .collect();
    found.sort_by(|one, other| one.id.cmp(&other.id));
    found.dedup_by(|later, earlier| later.id == earlier.id);
    found
}

/// Whether the provider reached at `base` with the API type `api` is an Ollama, which says what a model can
/// take only when asked about that model ([`limits`]).
#[must_use]
pub fn tells_per_model(api: &str, base: &str) -> bool {
    Wire::of(api) == Some(Wire::Ollama) || ollama::root(base) == "https://ollama.com"
}

/// Asks an Ollama what its model `id` can take. Nothing is known when it can't be asked or doesn't say.
// Human: Ollama's list has names only; the context a model reads is in what it shows about one model. It
// doesn't say how much a model writes.
// Agent: BLOCKS on one short question; call it from a thread of its own.
#[must_use]
pub fn limits(base: &str, key: Option<&SecretString>, id: &str) -> Limits {
    let mut request = api::agent().post(format!("{}/api/show", ollama::root(base))).header("content-type", "application/json");
    if let Some(key) = key {
        request = request.header("authorization", format!("Bearer {}", key.expose()));
    }
    let Ok(mut response) = request.send(serde_json::json!({ "model": id }).to_string().as_str()) else { return Limits::default() };
    if response.status().as_u16() != 200 {
        return Limits::default();
    }
    // Human: of an Ollama on this computer the form is told how much suits the computer, not all the model can
    // read: that number is what the model is then asked for, and all of it would take far too much memory.
    let shown = response.body_mut().read_to_string().unwrap_or_default();
    let most = ollama::most(&shown);
    let context = if is_local_ollama(base) { most.map(|_| ollama::suggested(&api::agent(), ollama::root(base), id, &shown)) } else { most };
    Limits { context, output: None }
}

/// Asks the provider which models it has.
///
/// # Errors
/// A sentence for the user when the provider couldn't be reached, refused, or listed nothing.
// Agent: BLOCKS on the network; call it from a thread of its own.
pub fn list_models(api: &str, base: &str, key: Option<&SecretString>) -> Result<Vec<Offered>, String> {
    let url = models_url(api, base);
    let mut request = api::agent().get(&url);
    match (Wire::of(api), key) {
        (Some(Wire::Messages), Some(key)) => request = request.header("anthropic-version", api::VERSION).header("x-api-key", key.expose()),
        (_, Some(key)) => request = request.header("authorization", format!("Bearer {}", key.expose())),
        (_, None) => {}
    }
    // Human: the usual reason Ollama on this computer doesn't answer is that it isn't running; say what to do.
    let unreached = |error: ureq::Error| {
        if is_local_ollama(base) { format!("{} ({error})", ollama::NOT_RUNNING) } else { format!("The provider couldn't be reached: {error}") }
    };
    let mut response = request.call().map_err(unreached)?;
    let status = response.status().as_u16();
    let body = response.body_mut().read_to_string().unwrap_or_default();
    if status != 200 {
        return Err(api::refusal_of(status, &body));
    }
    let found = offered(&body);
    match (found.is_empty(), is_local_ollama(base)) {
        (false, _) => Ok(found),
        (true, true) => Err(NO_LOCAL_MODELS.to_owned()),
        (true, false) => Err("The provider listed no models; type their names in by hand.".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The names of the models in a provider's answer.
    fn ids(body: &str) -> Vec<String> {
        offered(body).into_iter().map(|model| model.id).collect()
    }

    #[test]
    fn a_list_says_what_its_models_can_take_under_whatever_name_the_provider_uses() {
        let limits = |body: &str| offered(body).first().map(|model| (model.limits.context, model.limits.output));
        // OpenRouter: the provider's own numbers come before the model's general ones.
        assert_eq!(
            limits(r#"{"data":[{"id":"a","context_length":200000,"top_provider":{"context_length":128000,"max_completion_tokens":16384}}]}"#),
            Some((Some(200_000), Some(16_384)))
        );
        assert_eq!(limits(r#"{"data":[{"id":"a","context_window":131072,"max_completion_tokens":8192}]}"#), Some((Some(131_072), Some(8_192))));
        assert_eq!(limits(r#"{"data":[{"id":"a","max_input_tokens":1000000,"max_tokens":64000}]}"#), Some((Some(1_000_000), Some(64_000))));
        assert_eq!(
            limits(r#"{"models":[{"name":"models/g","inputTokenLimit":1048576,"outputTokenLimit":65536}]}"#),
            Some((Some(1_048_576), Some(65_536)))
        );
        // A list with names only says nothing, and nothing is made of a zero or of words.
        assert_eq!(limits(r#"{"data":[{"id":"a","owned_by":"x"}]}"#), Some((None, None)));
        assert_eq!(limits(r#"{"data":[{"id":"a","context_length":0,"max_tokens":"many"}]}"#), Some((None, None)));
        assert_eq!(limits(r#"["y"]"#), Some((None, None)));
    }

    #[test]
    fn known_providers_come_with_their_address_and_api() {
        assert_eq!(preset("xAI"), Some(completions("https://api.x.ai/v1")));
        assert_eq!(preset("Anthropic Claude Code").map(|p| p.api), Some(MESSAGES_API));
        assert!(preset("Cursor").is_none() && preset("GitHub Copilot").is_none() && preset("Mine").is_none());
        // Every address is encrypted, or on this computer, and written without a slash at its end, as the form
        // and the chat expect.
        let safe = |url: &str| url.starts_with("https://") || url.starts_with("http://localhost:");
        assert!(PRESETS.iter().all(|(_, p)| safe(p.base_url) && !p.base_url.ends_with('/') && Wire::of(p.api).is_some()));
        assert_eq!(PRESETS.iter().filter(|(_, p)| !p.base_url.starts_with("https://")).map(|(name, _)| *name).collect::<Vec<_>>(), [OLLAMA]);
        assert_eq!(preset(OLLAMA).map(|p| (p.api, p.base_url)), Some((OLLAMA_API, "http://localhost:11434")));
        assert_eq!(preset(OLLAMA_CLOUD), Some(completions("https://ollama.com/v1")));
        // Every provider that takes a key is one the price list is asked about.
        assert!(PRESETS.iter().filter(|(name, _)| *name != OLLAMA).all(|(_, p)| super::super::prices::known(p.base_url).is_some()));
        // Ollama on this computer is the only known provider that answers without a key.
        assert!(wants_key("https://ollama.com/v1/") && wants_key(" https://api.x.ai/v1") && !wants_key(OLLAMA_LOCAL));
        assert!(
            !wants_key("https://llm.example.com/v1") && is_local_ollama("http://localhost:11434/v1/") && !is_local_ollama("https://ollama.com/v1")
        );
        // An Ollama without models answers with no list at all.
        assert_eq!(ids(r#"{"object":"list","data":null}"#), [] as [&str; 0]);
        assert!(tells_per_model(OLLAMA_API, "http://box:11434") && tells_per_model(COMPLETIONS_API, "https://ollama.com/v1/"));
        assert!(!tells_per_model(COMPLETIONS_API, "https://api.x.ai/v1"));
        assert_eq!(models_url(OLLAMA_API, "http://localhost:11434"), "http://localhost:11434/api/tags");
        assert_eq!(models_url(OLLAMA_API, "http://box:11434/v1/"), "http://box:11434/api/tags");
        assert_eq!(ids(r#"{"models":[{"name":"qwen3:0.6b","model":"qwen3:0.6b"}]}"#), ["qwen3:0.6b"]);
        assert_eq!(starting_models("Anthropic Claude Code").len(), MODELS.len());
        assert_eq!(starting_models("xAI"), [] as [&str; 0]);
    }

    #[test]
    #[ignore = "reaches out to ollama.com and to an Ollama on this computer"]
    fn ollama_lists_its_models_here_and_in_its_cloud() {
        // The cloud lists its models to anyone; a chat there takes a key.
        let cloud = list_models(COMPLETIONS_API, "https://ollama.com/v1", None).unwrap();
        assert!(cloud.len() > 3, "{cloud:?}");
        // Asked about one model, it says how much that model reads.
        assert!(limits("https://ollama.com/v1", None, &cloud[0].id).context.is_some_and(|context| context > 1000));
        // On this computer Ollama lists what is pulled, or the form says what is missing.
        match list_models(OLLAMA_API, OLLAMA_LOCAL, None) {
            // Asked about one of them, it says how many tokens suit this computer: never more than are ever asked for.
            Ok(models) => {
                assert_ne!(models, [] as [Offered; 0]);
                let suits = limits(OLLAMA_LOCAL, None, &models[0].id).context;
                println!("{}: {suits:?}", models[0].id);
                assert!(suits.is_some_and(|context| (1000..=131_072).contains(&context)), "{suits:?}");
            }
            Err(why) => assert!(why == NO_LOCAL_MODELS || why.starts_with(ollama::NOT_RUNNING), "{why}"),
        }
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
