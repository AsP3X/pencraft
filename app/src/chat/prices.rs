//! The prices of the providers' models, from the open list at models.dev that the `OpenCode` project keeps.
//!
//! The list names each provider, the address it is reached at, and every model's prices in dollars per
//! million tokens. It is fetched once a day, cut down to addresses and prices, and kept in the home folder.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use super::usage::{self, Prices, Rates, Terms};
use crate::app::lock;

/// Where the list is.
pub const SOURCE: &str = "https://models.dev/api.json";

/// The list's name for each provider the settings window knows, by the address Quill reaches it at.
// Human: the list gives an address for some providers only, and for Meta and Kimi a different one than the
// one Quill uses; so the known providers are named here. Looked up on 2026-11-20.
const KNOWN: [(&str, &str); 15] = [
    ("https://api.openai.com/v1", "openai"),
    ("https://api.anthropic.com", "anthropic"),
    ("https://api.x.ai/v1", "xai"),
    ("https://generativelanguage.googleapis.com/v1beta/openai", "google"),
    ("https://opencode.ai/inference/go/openai/v1", "opencode-go"),
    ("https://openrouter.ai/api/v1", "openrouter"),
    ("https://ollama.com/v1", "ollama-cloud"),
    ("https://api.llama.com/compat/v1", "meta"),
    ("https://api.moonshot.ai/v1", "moonshotai"),
    ("https://api.kimi.com/coding/v1", "kimi-code-plan-global"),
    ("https://api.deepseek.com", "deepseek"),
    ("https://api.together.ai/v1", "togetherai"),
    ("https://api.fireworks.ai/inference/v1", "fireworks-ai"),
    ("https://api.z.ai/api/paas/v4", "zai"),
    ("https://opencode.ai/inference/openai/v1", "opencode"),
];

/// The addresses of the known providers that are paid by subscription, not by the token.
const PLANS: [&str; 3] = ["https://opencode.ai/inference/go/openai/v1", "https://api.kimi.com/coding/v1", "https://ollama.com/v1"];

/// An address as it is compared: without spaces around it, a slash at its end or one `/v1` before that, and
/// with the scheme and the host in lower case (the path is case-sensitive and stays as it is).
// Human: the same provider is typed as "https://API.anthropic.com/v1/", "https://api.deepseek.com/v1" or
// "https://ollama.com" (Ollama's own wire has no /v1); all of those are the address the table has. Both sides of
// a comparison go through here, so the table's addresses that end in /v1 are cut the same way.
fn plain(address: &str) -> String {
    let address = address.trim().trim_end_matches('/');
    let (scheme, rest) = address.split_once("://").unwrap_or(("", address));
    let (host, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let path = path.strip_suffix("/v1").unwrap_or(path);
    let start = if scheme.is_empty() { String::new() } else { format!("{}://", scheme.to_lowercase()) };
    format!("{start}{}{path}", host.to_lowercase())
}

/// The list's name for the known provider reached at `base`.
pub(super) fn known(base: &str) -> Option<&'static str> {
    let base = plain(base);
    KNOWN.iter().find(|(address, _)| plain(address) == base).map(|(_, name)| *name)
}

/// How the provider reached at `base` is paid: by subscription for the known plans, else by the token.
#[must_use]
pub fn terms(base: &str) -> Terms {
    let base = plain(base);
    if PLANS.iter().any(|plan| plain(plan) == base) { Terms::Plan } else { Terms::PerToken }
}

/// One provider of the list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Provider {
    /// The address the list gives for it; empty when it gives none.
    api: String,
    /// Its models' prices, by the model's name.
    models: BTreeMap<String, Rates>,
}

/// The price list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Catalogue {
    /// The providers by the list's name for them.
    providers: BTreeMap<String, Provider>,
}

/// The four prices in a `cost` of the list (or in one of its tiers); `None` without an input and an output price.
// Human: a model without cache prices charges cached input as input.
fn prices(cost: &Value) -> Option<Prices> {
    let price = |name: &str| match cost.get(name) {
        Some(Value::Number(dollars)) => usage::scaled(&dollars.to_string(), 6),
        _ => None,
    };
    let (input, output) = (price("input")?, price("output")?);
    Some(Prices { input, output, cached: price("cache_read").unwrap_or(input), written: price("cache_write").unwrap_or(input) })
}

/// A model's prices from its `cost` in the list; `None` when it has none, or all are zero.
// Human: the list writes subscription plans and free models as all zeros; neither is a price per token.
fn rates(cost: &Value) -> Option<Rates> {
    let base = prices(cost)?;
    let tiers = cost
        .get("tiers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|tier| tier.pointer("/tier/type").and_then(Value::as_str) == Some("context"))
        .filter_map(|tier| Some((tier.pointer("/tier/size")?.as_u64()?, prices(tier)?)))
        .collect();
    let rates = Rates { base, tiers };
    (!rates.free()).then_some(rates)
}

impl Catalogue {
    /// The list written in `text`: the whole one from models.dev, or the cut of it Quill keeps.
    #[must_use]
    pub fn parse(text: &str) -> Option<Catalogue> {
        let list = serde_json::from_str::<Value>(text).ok()?;
        let providers = list
            .as_object()?
            .iter()
            .map(|(name, provider)| {
                let models = provider.get("models").and_then(Value::as_object).into_iter().flatten();
                let models = models.filter_map(|(id, model)| Some((id.clone(), rates(model.get("cost")?)?))).collect();
                (name.clone(), Provider { api: provider.get("api").and_then(Value::as_str).map(plain).unwrap_or_default(), models })
            })
            .collect();
        Some(Catalogue { providers })
    }

    /// Whether the list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// The prices of the models of the provider reached at `base`, by name; none when the list doesn't know it.
    #[must_use]
    pub fn models(&self, base: &str) -> BTreeMap<String, Rates> {
        let base = plain(base);
        let found = match known(&base) {
            Some(name) => self.providers.get(name),
            None => self.providers.values().find(|provider| !provider.api.is_empty() && provider.api == base),
        };
        found.map(|provider| provider.models.clone()).unwrap_or_default()
    }
}

/// The list in `text` cut down to what Quill reads: each provider's address and its models' costs.
// Human: the whole list is nearly seven megabytes, most of it descriptions.
#[must_use]
pub fn cut(text: &str) -> Option<String> {
    let list = serde_json::from_str::<Value>(text).ok()?;
    let mut out = serde_json::Map::new();
    for (name, provider) in list.as_object()? {
        let mut models = serde_json::Map::new();
        for (id, model) in provider.get("models").and_then(Value::as_object).into_iter().flatten() {
            if let Some(cost) = model.get("cost") {
                models.insert(id.clone(), json!({ "cost": cost }));
            }
        }
        out.insert(name.clone(), json!({ "api": provider.get("api"), "models": models }));
    }
    Some(Value::Object(out).to_string())
}

/// How old the kept list may get before it is fetched again.
const EVERY: Duration = Duration::from_hours(24);
/// The most the list may weigh: a little over twice what it was (6.8 megabytes in November 2026).
const MOST_BYTES: u64 = 16 * 1024 * 1024;

/// The file the cut list is kept in; `None` without a home folder, and in tests.
// Human: the tests keep out of the home folder.
fn file() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    crate::imagery::store::home().map(|home| home.join(".pencraft").join("prices.json"))
}

/// Whether the list kept in `file` is missing or older than a day.
fn stale(file: &Path) -> bool {
    std::fs::metadata(file).and_then(|facts| facts.modified()).ok().and_then(|written| written.elapsed().ok()).is_none_or(|age| age > EVERY)
}

/// The list kept in `file`; empty when there is none or it can't be read.
fn read(file: &Path) -> Catalogue {
    std::fs::read_to_string(file).ok().and_then(|text| Catalogue::parse(&text)).unwrap_or_default()
}

/// Fetches the whole list.
///
/// # Errors
/// A sentence when models.dev couldn't be reached or didn't answer with the list.
// Human: a plain request for a public file: no key, nothing about the user or their chats.
// Agent: BLOCKS on the network for a download of several megabytes.
fn fetch() -> Result<String, String> {
    let mut response = super::api::agent().get(SOURCE).call().map_err(|error| format!("models.dev couldn't be reached: {error}"))?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("models.dev answered with {status}"));
    }
    response.body_mut().with_config().limit(MOST_BYTES).read_to_string().map_err(|error| format!("the price list couldn't be read: {error}"))
}

/// The price list as it is kept, and, when `look` says so and the kept one is a day old, a fetch of a new one
/// that replaces it once it is there.
// Human: a failed fetch keeps the old list and is tried again the next time the app starts.
// Agent: READS a small file; SPAWNS a thread that fetches and WRITES `~/.pencraft/prices.json`.
#[must_use]
pub fn start(look: bool) -> Arc<Mutex<Catalogue>> {
    let file = file();
    let shared = Arc::new(Mutex::new(file.as_deref().map(read).unwrap_or_default()));
    let Some(file) = file.filter(|file| look && stale(file)) else { return shared };
    let fresh = shared.clone();
    let started = std::thread::Builder::new().name("prices".to_owned()).spawn(move || {
        let kept = fetch().and_then(|text| cut(&text).ok_or_else(|| "the price list isn't the JSON it was".to_owned()));
        match kept.as_deref().map(|kept| (kept, Catalogue::parse(kept))) {
            Ok((kept, Some(list))) if !list.is_empty() => {
                let written = file.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| craft_atomic::atomic_write(&file, kept.as_bytes()));
                if let Err(error) = written {
                    log::warn!("the price list couldn't be kept: {error}");
                }
                *lock(&fresh) = list;
            }
            Ok(_) => log::warn!("the price list from models.dev is empty"),
            Err(problem) => log::warn!("prices: {problem}"),
        }
    });
    if let Err(error) = started {
        log::warn!("the price list couldn't be fetched: {error}");
    }
    shared
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cut of what models.dev answered on 2026-11-20, with the fields Quill doesn't read left in.
    const LIST: &str = r#"{
      "anthropic": { "id": "anthropic", "name": "Anthropic", "doc": "https://docs", "models": {
        "claude-fable-5-2": { "id": "claude-fable-5-2", "name": "Claude Fable 5.2", "limit": { "context": 1000000, "output": 128000 },
          "cost": { "input": 8, "output": 40, "cache_read": 0.2, "cache_write": 10 } } } },
      "xai": { "id": "xai", "name": "xAI", "models": {
        "grok-4.5": { "id": "grok-4.5", "cost": { "input": 2, "output": 6, "cache_read": 0.3,
          "tiers": [{ "input": 4, "output": 12, "cache_read": 0.6, "tier": { "type": "context", "size": 200000 } }],
          "context_over_200k": { "input": 4, "output": 12, "cache_read": 0.6 } } } } },
      "togetherai": { "id": "togetherai", "name": "Together AI", "models": {
        "openai/gpt-oss-120b": { "id": "openai/gpt-oss-120b", "cost": { "input": 0.15, "output": 0.6 } } } },
      "kimi-code-plan-global": { "id": "kimi-code-plan-global", "api": "https://api.kimi.ai/coding/v1", "models": {
        "k3": { "id": "k3", "cost": { "input": 0, "output": 0, "cache_read": 0, "cache_write": 0 } } } },
      "some-host": { "id": "some-host", "api": "https://llm.example.com/v1/", "models": {
        "m": { "id": "m", "cost": { "input": 1, "output": 2 } }, "unpriced": { "id": "unpriced" } } }
    }"#;

    #[test]
    fn the_list_gives_each_providers_prices_by_the_address_it_is_reached_at() {
        let list = Catalogue::parse(LIST).unwrap();
        let fable = &list.models("https://api.anthropic.com/")["claude-fable-5-2"];
        assert_eq!(fable.base, Prices { input: 8_000_000, cached: 200_000, written: 10_000_000, output: 40_000_000 });
        // A tier, and a missing cache-write price charged as input.
        let grok = &list.models("https://api.x.ai/v1")["grok-4.5"];
        assert_eq!(grok.tiers, vec![(200_000, Prices { input: 4_000_000, cached: 600_000, written: 4_000_000, output: 12_000_000 })]);
        assert_eq!(grok.base.written, 2_000_000);
        // No cache prices at all: cached input costs what input costs.
        let oss = &list.models("https://api.together.ai/v1")["openai/gpt-oss-120b"];
        assert_eq!((oss.base.input, oss.base.cached, oss.base.output), (150_000, 150_000, 600_000));
        // A plan's zeros are no price; a model without a cost has none.
        assert!(list.models("https://api.kimi.com/coding/v1").is_empty());
        // A provider the user added is found by the address the list gives for it.
        let own = list.models(" https://llm.example.com/v1 ");
        assert_eq!(own.keys().collect::<Vec<_>>(), ["m"]);
        assert!(list.models("https://nobody.example.com").is_empty() && list.models("").is_empty());
        // The case of scheme and host, and a /v1 at either end, make no difference; another host or path does.
        assert!(list.models("https://API.anthropic.com/v1/").contains_key("claude-fable-5-2"));
        assert!(list.models("HTTPS://Api.X.AI").contains_key("grok-4.5"));
        assert_eq!(list.models("https://LLM.example.com").keys().collect::<Vec<_>>(), ["m"]);
        assert_eq!(list.models("https://llm.example.com/v1/").keys().collect::<Vec<_>>(), ["m"]);
        assert!(list.models("https://api.anthropic.org").is_empty() && list.models("https://llm.example.com/v2").is_empty());
        assert!(list.models("https://llm.example.com/V1/x").is_empty() && list.models("https://api.anthropic.com/other").is_empty());
        assert!(Catalogue::parse("not json").is_none() && Catalogue::parse("[]").is_none());
        assert!(Catalogue::default().is_empty() && !list.is_empty());
    }

    #[test]
    fn the_cut_reads_the_same_as_the_whole_list() {
        let kept = cut(LIST).unwrap();
        assert!(kept.len() < LIST.len() && !kept.contains("Claude Fable 5.2") && !kept.contains("limit"));
        assert_eq!(Catalogue::parse(&kept), Catalogue::parse(LIST));
        assert!(cut("not json").is_none());
    }

    #[test]
    fn plans_are_known_by_their_address() {
        assert_eq!((terms("https://ollama.com/v1/"), terms("https://api.kimi.com/coding/v1")), (Terms::Plan, Terms::Plan));
        assert_eq!((terms("https://api.openai.com/v1"), terms("https://llm.example.com")), (Terms::PerToken, Terms::PerToken));
        assert!(PLANS.iter().all(|plan| known(plan).is_some()));
        // Ollama Cloud over its own wire has no /v1; a typed address may differ in case and in a /v1.
        assert_eq!((terms("https://ollama.com"), terms("HTTPS://Ollama.com/V1")), (Terms::Plan, Terms::PerToken));
        assert_eq!(terms("https://OLLAMA.com/v1/"), Terms::Plan);
        assert_eq!(known("https://ollama.com"), Some("ollama-cloud"));
        assert_eq!(known("https://api.deepseek.com/v1"), Some("deepseek"));
        assert_eq!(known("https://API.anthropic.com/v1/"), Some("anthropic"));
        assert_eq!(known("https://api.openai.com"), Some("openai"));
        assert_eq!(known("https://api.deepseek.org"), None);
    }

    #[test]
    fn no_two_known_addresses_read_the_same() {
        for (at, (address, name)) in KNOWN.iter().enumerate() {
            assert_eq!(known(address), Some(*name), "{address} finds another provider");
            assert!(KNOWN.iter().skip(at + 1).all(|(other, _)| plain(other) != plain(address)), "{address} collapses onto another");
        }
        assert_ne!(known("https://opencode.ai/inference/go/openai/v1"), known("https://opencode.ai/inference/openai/v1"));
    }

    #[test]
    fn a_list_is_fetched_again_once_it_is_a_day_old() {
        let folder = std::env::temp_dir().join(format!("pencraft-prices-test-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let file = folder.join("prices.json");
        drop(std::fs::remove_file(&file));
        // No file yet: fetch. A file just written: don't.
        assert!(stale(&file));
        std::fs::write(&file, cut(LIST).unwrap()).unwrap();
        assert!(!stale(&file));
        assert_eq!(read(&file), Catalogue::parse(LIST).unwrap());
        // A file that can't be read is an empty list, not an error.
        std::fs::write(&file, "half a li").unwrap();
        assert!(read(&file).is_empty());
        drop(std::fs::remove_dir_all(&folder));
    }

    #[test]
    #[ignore = "reaches out to models.dev"]
    fn the_real_list_knows_every_provider_named_here() {
        let text = fetch().unwrap();
        let list = Catalogue::parse(&cut(&text).unwrap()).unwrap();
        for (address, name) in KNOWN {
            assert!(list.providers.contains_key(name), "{name} is gone from the list");
            // Kimi's plan is all zeros; every other provider has priced models.
            assert!(name == "kimi-code-plan-global" || !list.models(address).is_empty(), "{name} has no prices");
        }
    }
}
