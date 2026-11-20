# Tokens and Price of an Answer: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Under every answered chat message, show the tokens it took and what it cost, for every engine and provider, by that provider's price for the model.

**Architecture:** Each engine reads the usage its provider already reports and tells the chat with a new `Event::Used`. The chat prices each report with a `Pricing` it was handed when the run started (billed cost, then the user's own prices, then the models.dev list), sums the run, and puts the sum on the run's last line. Two new modules carry the logic without any window code: `chat/usage.rs` (counts, prices, words) and `chat/prices.rs` (the models.dev list, cached on disk).

**Tech Stack:** Rust, serde_json, ureq 3.5, egui (storytold fork). No new dependency.

**Spec:** `docs/superpowers/specs/2026-11-20-token-usage-and-price-design.md`

**House rules (read `app/AGENTS.md` first):**

- All commands run from `app/`.
- No `unwrap`/`expect`/`panic!` outside tests, no `#[allow]`, `///` on every public item, `// Human:` / `// Agent:` comments on non-trivial code.
- Clippy runs pedantic with warnings as errors. Money and token counts are whole numbers (`u64`) throughout, so no float casts are needed; keep it that way.
- An item nothing uses yet fails clippy's dead-code check. Until Task 10 wires everything together, verify each task with its `cargo test` filter; run the full gate `scripts/check.sh` only in Task 14.
- **Do not commit.** The project rule is that nothing is committed without the user asking. There are no commit steps in this plan.

## File map

| File | Responsibility |
|---|---|
| `src/chat/usage.rs` (new) | `Used`, `Prices`, `Rates`, `Report`, `Spent`, `Basis`, `Terms`, `Pricing`; reading each API's usage; the line's words |
| `src/chat/prices.rs` (new) | The models.dev list: parse, cut, find a provider by address, disk cache, daily refresh |
| `src/chat.rs` | `Event::Used`, `Entry::spent`, `Ask::pricing`, `Chat::total`, summing a run, `pricing()` |
| `src/chat/api.rs`, `openai.rs`, `ollama.rs`, `code.rs`, `critic.rs` | Tell `Event::Used` |
| `src/chat/history.rs` | Keep `spent` per line and the conversation's `total` |
| `src/prefs.rs` | `ProviderModel::{input_price, output_price}`, `Agents::prices` |
| `src/app.rs`, `src/app/talk.rs` | Hold the list, hand `Pricing` to a run, keep and read the total |
| `src/conversation.rs`, `src/theme.rs` | Paint the line and its hover |
| `src/settings.rs` | The switch and the two price fields |

---

### Task 1: Counts, prices and exact money (`usage.rs`, part 1)

**Files:**
- Create: `src/chat/usage.rs`
- Modify: `src/chat.rs:22-33` (module list)

- [ ] **Step 1: Declare the module**

In `src/chat.rs`, add after `pub mod steps;`:

```rust
pub mod usage;
```

- [ ] **Step 2: Write the file with failing tests**

Create `src/chat/usage.rs`:

```rust
//! What an answer took and what it cost: the tokens a model read and wrote, the prices of a model, and the
//! words the chat says about both.
// Human: money is kept in whole numbers. A price is in millionths of a dollar per million tokens, which is the
// same as millionths of a millionth of a dollar per token; a cost is tokens times a price, in that small unit.
// A `u64` of it holds 18 million dollars. Dollars appear only in the words.

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// The number written in `text` (digits, a point, an exponent) times ten to the `places`, rounded down.
/// `None` for anything that isn't a number of zero or more.
// Human: prices and costs arrive as decimal numbers ("0.075", "1.2e-5"). Reading their digits keeps them
// exact; a float would not.
#[must_use]
pub fn scaled(text: &str, places: u32) -> Option<u64> {
    let text = text.trim();
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i64>().ok()?),
        None => (text, 0),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{whole}{fraction}");
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let number: u128 = digits.parse().ok()?;
    let shift = exponent + i64::from(places) - i64::try_from(fraction.len()).ok()?;
    let power = |by: i64| u32::try_from(by).ok().and_then(|by| 10_u128.checked_pow(by));
    let out = if shift >= 0 { number.checked_mul(power(shift)?)? } else { power(-shift).map_or(0, |by| number / by) };
    u64::try_from(out).ok()
}

/// The tokens of one request, or of several added up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Used {
    /// Read by the model and not taken from the provider's cache.
    pub input: u64,
    /// Read from the provider's cache.
    pub cached: u64,
    /// Read and written to the provider's cache.
    pub written: u64,
    /// Written by the model, its thinking included.
    pub output: u64,
}

impl Used {
    /// Everything the model read.
    #[must_use]
    pub fn read(self) -> u64 {
        self.input.saturating_add(self.cached).saturating_add(self.written)
    }

    /// Whether nothing was counted.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self == Used::default()
    }

    /// Adds `other` to these.
    pub fn add(&mut self, other: Used) {
        self.input = self.input.saturating_add(other.input);
        self.cached = self.cached.saturating_add(other.cached);
        self.written = self.written.saturating_add(other.written);
        self.output = self.output.saturating_add(other.output);
    }
}

/// What a model's tokens cost, each in millionths of a dollar per million tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Prices {
    /// Input not from the cache.
    pub input: u64,
    /// Input read from the cache.
    pub cached: u64,
    /// Input written to the cache.
    pub written: u64,
    /// Output.
    pub output: u64,
}

/// A model's prices: the usual ones, and others for requests that read more than a size.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rates {
    /// The usual prices.
    pub base: Prices,
    /// The prices of a request that reads more than this many tokens.
    pub tiers: Vec<(u64, Prices)>,
}

impl Rates {
    /// The prices a user typed in as dollars per million tokens; `None` unless both are numbers.
    // Human: a user's own prices have no cache prices; cached input is charged as input.
    #[must_use]
    pub fn typed(input: &str, output: &str) -> Option<Rates> {
        let (input, output) = (scaled(input, 6)?, scaled(output, 6)?);
        Some(Rates { base: Prices { input, cached: input, written: input, output }, tiers: Vec::new() })
    }

    /// Whether every price is nothing: how a price list writes a subscription plan or a free model.
    #[must_use]
    pub fn free(&self) -> bool {
        self.base == Prices::default() && self.tiers.iter().all(|(_, prices)| *prices == Prices::default())
    }

    /// What the one request `used` costs.
    #[must_use]
    pub fn cost(&self, used: Used) -> u64 {
        let read = used.read();
        let prices = self.tiers.iter().filter(|(size, _)| read > *size).max_by_key(|(size, _)| *size).map_or(self.base, |(_, prices)| *prices);
        let part = |tokens: u64, price: u64| tokens.saturating_mul(price);
        part(used.input, prices.input)
            .saturating_add(part(used.cached, prices.cached))
            .saturating_add(part(used.written, prices.written))
            .saturating_add(part(used.output, prices.output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_numbers_are_read_exactly() {
        assert_eq!((scaled("8", 6), scaled("0.075", 6), scaled(" 2.5 ", 6)), (Some(8_000_000), Some(75_000), Some(2_500_000)));
        // A cost of 0.000012 dollars, as a JSON number may be written.
        assert_eq!((scaled("1.2e-5", 12), scaled("0.0812", 12)), (Some(12_000_000), Some(81_200_000_000)));
        assert_eq!((scaled("1e-300", 6), scaled("0", 6), scaled(".5", 2)), (Some(0), Some(0), Some(50)));
        assert_eq!((scaled("", 6), scaled("abc", 6), scaled("-1", 6), scaled("1.2.3", 6), scaled("1e400", 6)), (None, None, None, None, None));
    }

    #[test]
    fn a_request_costs_its_tokens_at_the_models_prices() {
        // Eight dollars a million in, forty out, cache at 0.2 and 10.
        let base = Prices { input: 8_000_000, cached: 200_000, written: 10_000_000, output: 40_000_000 };
        let rates = Rates { base, tiers: Vec::new() };
        let million = 1_000_000;
        assert_eq!(rates.cost(Used { input: million, ..Used::default() }), 8_000_000_000_000);
        let used = Used { input: 1000, cached: 2000, written: 3000, output: 4000 };
        assert_eq!(rates.cost(used), 1000 * 8_000_000 + 2000 * 200_000 + 3000 * 10_000_000 + 4000 * 40_000_000);
        // Past a tier's size the whole request is charged at the tier's prices; at the size it is not.
        let dear = Prices { input: 16_000_000, cached: 400_000, written: 20_000_000, output: 60_000_000 };
        let tiered = Rates { base, tiers: vec![(200_000, dear)] };
        assert_eq!(tiered.cost(Used { input: 200_000, output: 1, ..Used::default() }), 200_000 * 8_000_000 + 40_000_000);
        assert_eq!(tiered.cost(Used { input: 150_000, cached: 50_001, output: 1, ..Used::default() }), 150_000 * 16_000_000 + 50_001 * 400_000 + 60_000_000);
        assert!(Rates::default().free() && !rates.free());
    }

    #[test]
    fn typed_prices_charge_cached_input_as_input() {
        let rates = Rates::typed("3", " 15.5 ").unwrap();
        assert_eq!(rates.base, Prices { input: 3_000_000, cached: 3_000_000, written: 3_000_000, output: 15_500_000 });
        assert!(Rates::typed("", "15").is_none() && Rates::typed("3", "lots").is_none());
        let mut sum = Used { input: 1, cached: 2, written: 3, output: 4 };
        sum.add(Used { input: 10, cached: 20, written: 30, output: 40 });
        assert_eq!((sum, sum.read(), Used::default().is_empty()), (Used { input: 11, cached: 22, written: 33, output: 44 }, 66, true));
    }
}
```

`BTreeMap`, `Value` and `json` are imported for Tasks 2 and 3; if the compiler complains about them now, leave them out until then.

- [ ] **Step 3: Run the tests**

Run: `cargo test usage::`
Expected: 3 tests pass.

---

### Task 2: A run's sum and its words (`usage.rs`, part 2)

**Files:**
- Modify: `src/chat/usage.rs`

- [ ] **Step 1: Add the failing tests**

Add to the `tests` module:

```rust
    fn spent(input: u64, output: u64, cost: Option<u64>, basis: Basis) -> Spent {
        Spent { used: Used { input, output, ..Used::default() }, cost, partial: false, basis, model: "m".to_owned() }
    }

    #[test]
    fn the_line_says_tokens_and_what_is_known_of_the_price() {
        let mut eight = spent(10_200, 1_800, Some(80_000_000_000), Basis::Listed);
        eight.used.cached = 2_200;
        assert_eq!(eight.line(), "12.4k in · 1.8k out · $0.08");
        assert_eq!(spent(940, 12, None, Basis::Unknown).line(), "940 in · 12 out");
        assert_eq!(spent(940, 12, None, Basis::Local).line(), "940 in · 12 out");
        assert_eq!(spent(1_200_000, 999_950, Some(1_234_500_000_000_000), Basis::Billed).line(), "1.2M in · 1M out · $1234.50");
        assert_eq!(spent(100, 10, Some(9_999_999_999), Basis::Own).line(), "100 in · 10 out · < $0.01");
        assert_eq!(spent(100, 10, Some(0), Basis::Listed).line(), "100 in · 10 out · $0.00");
        assert_eq!(spent(999_949, 1000, Some(14_900_000_000), Basis::Worth).line(), "999.9k in · 1k out · ≈ $0.01");
    }

    #[test]
    fn sums_keep_what_is_known_and_say_when_a_part_is_missing() {
        let mut sum = Spent::default();
        sum.add(&spent(100, 10, Some(5), Basis::Listed));
        assert_eq!(sum, spent(100, 10, Some(5), Basis::Listed));
        sum.add(&spent(50, 5, Some(7), Basis::Listed));
        assert_eq!((sum.used.input, sum.cost, sum.partial, sum.basis), (150, Some(12), false, Basis::Listed));
        // A request without a known price leaves the sum a lower bound.
        sum.add(&spent(1, 1, None, Basis::Unknown));
        assert_eq!((sum.cost, sum.partial, sum.basis), (Some(12), true, Basis::Mixed));
        assert!(sum.line().ends_with("· ≥ $0.00"));
        // A model on this computer costs nothing and hides nothing.
        let mut local = spent(1, 1, None, Basis::Local);
        local.add(&spent(1, 1, None, Basis::Local));
        assert_eq!((local.cost, local.partial, local.basis), (None, false, Basis::Local));
        // Nothing added changes nothing.
        let before = sum.clone();
        sum.add(&Spent::default());
        assert_eq!(sum, before);
    }

    #[test]
    fn the_hover_says_the_parts_and_a_kept_sum_comes_back() {
        let mut one = spent(10_200, 1_800, Some(80_000_000_000), Basis::Listed);
        one.used.cached = 2_200;
        let total = spent(90_000, 9_000, Some(610_000_000_000), Basis::Listed);
        assert_eq!(
            one.details(&total),
            vec![
                "Input: 10.2k",
                "Read from cache: 2.2k",
                "Output: 1.8k",
                "Model: m",
                "Price: from the list at models.dev",
                "This conversation: 90k in · 9k out · $0.61"
            ]
        );
        // The only answer of a conversation doesn't repeat itself as the total.
        assert_eq!(one.details(&one).len(), 5);
        assert_eq!(Spent::from_kept(&one.kept()), Some(one.clone()));
        let partial = Spent { partial: true, basis: Basis::Worth, cost: None, ..one };
        assert_eq!(Spent::from_kept(&partial.kept()), Some(partial));
        assert_eq!(Spent::from_kept(&json!({ "text": "hi" })), None);
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test usage::`
Expected: does not compile, `Spent` and `Basis` not found.

- [ ] **Step 3: Implement**

Add above the `tests` module:

```rust
/// How a cost is known.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Basis {
    /// It isn't: no price is known for the model.
    #[default]
    Unknown,
    /// The provider said what it charged for the request.
    Billed,
    /// Worked out from the prices the user typed in.
    Own,
    /// Worked out from the price list.
    Listed,
    /// The provider or program is paid by subscription: what the tokens are worth, not a bill.
    Worth,
    /// The model runs on this computer and costs nothing.
    Local,
    /// A sum of costs known in different ways.
    Mixed,
}

impl Basis {
    /// The name this is kept under in a chat's file.
    fn name(self) -> &'static str {
        match self {
            Basis::Unknown => "unknown",
            Basis::Billed => "billed",
            Basis::Own => "own",
            Basis::Listed => "listed",
            Basis::Worth => "worth",
            Basis::Local => "local",
            Basis::Mixed => "mixed",
        }
    }

    /// The basis kept under `name`.
    fn of(name: &str) -> Basis {
        [Basis::Billed, Basis::Own, Basis::Listed, Basis::Worth, Basis::Local, Basis::Mixed].into_iter().find(|basis| basis.name() == name).unwrap_or_default()
    }

    /// What the hover says about it.
    fn says(self) -> &'static str {
        match self {
            Basis::Unknown => "No price is known for this model",
            Basis::Billed => "Price: as the provider billed it",
            Basis::Own => "Price: from the prices typed in for this model",
            Basis::Listed => "Price: from the list at models.dev",
            Basis::Worth => "Price: what the tokens are worth at list prices; a subscription isn't billed by the token",
            Basis::Local => "The model runs on this computer",
            Basis::Mixed => "Price: from more than one source",
        }
    }
}

/// What a request, a run or a conversation took and cost.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Spent {
    /// The tokens.
    pub used: Used,
    /// The cost, as far as it is known.
    pub cost: Option<u64>,
    /// Some of what was used has no known cost: `cost` is the least it was.
    pub partial: bool,
    /// How the cost is known.
    pub basis: Basis,
    /// The model that answered.
    pub model: String,
}

/// A count of tokens in few letters: `940`, `12.4k`, `1.2M`.
fn count(tokens: u64) -> String {
    let short = |unit: u64, letter: char| {
        let tenths = tokens.saturating_add(unit / 20) / (unit / 10);
        if tenths % 10 == 0 { format!("{}{letter}", tenths / 10) } else { format!("{}.{}{letter}", tenths / 10, tenths % 10) }
    };
    match tokens {
        0..=999 => tokens.to_string(),
        1000..=999_949 => short(1000, 'k'),
        _ => short(1_000_000, 'M'),
    }
}

/// A cost in dollars and cents.
fn dollars(cost: u64) -> String {
    const CENT: u64 = 10_000_000_000;
    match cost {
        0 => "$0.00".to_owned(),
        less if less < CENT => "< $0.01".to_owned(),
        _ => {
            let cents = cost.saturating_add(CENT / 2) / CENT;
            format!("${}.{:02}", cents / 100, cents % 100)
        }
    }
}

impl Spent {
    /// Whether there is nothing to say.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.used.is_empty() && self.cost.is_none()
    }

    /// Adds `other` to this sum.
    // Human: a sum of a known and an unknown cost is the least the whole cost; a model on this computer has
    // no cost to be unknown.
    pub fn add(&mut self, other: &Spent) {
        if other.is_empty() {
            return;
        }
        if self.is_empty() {
            *self = other.clone();
            return;
        }
        let hidden = |spent: &Spent| spent.cost.is_none() && spent.basis != Basis::Local;
        self.partial |= other.partial || (hidden(self) != hidden(other) && (self.cost.is_some() || other.cost.is_some()));
        self.cost = match (self.cost, other.cost) {
            (Some(one), Some(more)) => Some(one.saturating_add(more)),
            (one, more) => one.or(more),
        };
        if self.basis != other.basis {
            self.basis = Basis::Mixed;
        }
        self.used.add(other.used);
    }

    /// The one line under an answer: `12.4k in · 1.8k out · $0.08`.
    #[must_use]
    pub fn line(&self) -> String {
        let tokens = format!("{} in · {} out", count(self.used.read()), count(self.used.output));
        let mark = match (self.partial, self.basis) {
            (true, _) => "≥ ",
            (false, Basis::Worth) => "≈ ",
            _ => "",
        };
        // Human: "at least less than a cent" says nothing; a lower bound under a cent is written as zero.
        let sum = |cost: u64| if self.partial && cost < 10_000_000_000 { "$0.00".to_owned() } else { dollars(cost) };
        match self.cost {
            Some(cost) => format!("{tokens} · {mark}{}", sum(cost)),
            None => tokens,
        }
    }

    /// What the hover of the line says, line by line; `total` is the whole conversation's sum.
    #[must_use]
    pub fn details(&self, total: &Spent) -> Vec<String> {
        let mut lines = vec![format!("Input: {}", count(self.used.input))];
        if self.used.cached > 0 {
            lines.push(format!("Read from cache: {}", count(self.used.cached)));
        }
        if self.used.written > 0 {
            lines.push(format!("Written to cache: {}", count(self.used.written)));
        }
        lines.push(format!("Output: {}", count(self.used.output)));
        if !self.model.is_empty() {
            lines.push(format!("Model: {}", self.model));
        }
        lines.push(self.basis.says().to_owned());
        if total != self && !total.is_empty() {
            lines.push(format!("This conversation: {}", total.line()));
        }
        lines
    }

    /// This sum as a chat's file keeps it.
    #[must_use]
    pub fn kept(&self) -> Value {
        json!({
            "in": self.used.input, "cached": self.used.cached, "written": self.used.written, "out": self.used.output,
            "cost": self.cost, "partial": self.partial, "basis": self.basis.name(), "model": self.model,
        })
    }

    /// The sum a chat's file keeps as `kept`; `None` when that is no sum.
    #[must_use]
    pub fn from_kept(kept: &Value) -> Option<Spent> {
        let count = |name: &str| kept.get(name).and_then(Value::as_u64);
        Some(Spent {
            used: Used { input: count("in")?, cached: count("cached").unwrap_or(0), written: count("written").unwrap_or(0), output: count("out")? },
            cost: count("cost"),
            partial: kept.get("partial").and_then(Value::as_bool).unwrap_or(false),
            basis: Basis::of(kept.get("basis").and_then(Value::as_str).unwrap_or_default()),
            model: kept.get("model").and_then(Value::as_str).unwrap_or_default().to_owned(),
        })
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test usage::`
Expected: 6 tests pass.

---

### Task 3: Reading each API's usage, and pricing a report (`usage.rs`, part 3)

**Files:**
- Modify: `src/chat/usage.rs`

- [ ] **Step 1: Add the failing tests**

```rust
    #[test]
    fn each_apis_usage_is_brought_to_one_shape() {
        // Messages API: the start of an answer says what was read, its end what was written.
        let mut used = Used::default();
        messages(&json!({ "input_tokens": 12, "cache_creation_input_tokens": 300, "cache_read_input_tokens": 9000, "output_tokens": 1 }), &mut used);
        messages(&json!({ "output_tokens": 420 }), &mut used);
        messages(&json!({}), &mut used);
        assert_eq!(used, Used { input: 12, cached: 9000, written: 300, output: 420 });
        // Chat Completions counts cached tokens inside the prompt's; they are taken off.
        let report = completions(&json!({ "prompt_tokens": 1000, "completion_tokens": 50, "prompt_tokens_details": { "cached_tokens": 800 } })).unwrap();
        assert_eq!((report.used, report.billed), (Used { input: 200, cached: 800, written: 0, output: 50 }, None));
        // Deepseek names them differently; OpenRouter says what it charged.
        let deepseek = completions(&json!({ "prompt_tokens": 1000, "completion_tokens": 50, "prompt_cache_hit_tokens": 600 })).unwrap();
        assert_eq!(deepseek.used, Used { input: 400, cached: 600, written: 0, output: 50 });
        let router = completions(&json!({ "prompt_tokens": 10, "completion_tokens": 5, "cost": 0.0000125 })).unwrap();
        assert_eq!(router.billed, Some(12_500_000));
        assert_eq!((completions(&json!({})), completions(&json!(null))), (None, None));
        // Ollama says both on its last line.
        assert_eq!(ollama(&json!({ "done": true, "prompt_eval_count": 17_000, "eval_count": 90 })).unwrap().used, Used { input: 17_000, output: 90, ..Used::default() });
        assert_eq!(ollama(&json!({ "done": true })), None);
        // A program's last line carries the run's sum and its cost.
        let line = json!({ "type": "result", "total_cost_usd": 0.0812, "usage": { "input_tokens": 5, "cache_read_input_tokens": 40, "output_tokens": 7 } });
        let report = program(&line).unwrap();
        assert_eq!((report.used, report.billed), (Used { input: 5, cached: 40, written: 0, output: 7 }, Some(81_200_000_000)));
        assert_eq!(program(&json!({ "type": "result", "result": "Hello" })), None);
    }

    #[test]
    fn a_report_is_priced_by_the_best_that_is_known() {
        let listed = Rates { base: Prices { input: 1_000_000, cached: 100_000, written: 1_000_000, output: 2_000_000 }, tiers: Vec::new() };
        let pricing = Pricing {
            model: "big".to_owned(),
            own: None,
            listed: BTreeMap::from([("big".to_owned(), listed.clone()), ("small".to_owned(), Rates::typed("0.5", "1").unwrap())]),
            terms: Terms::PerToken,
        };
        let report = |model: Option<&str>, billed| Report { used: Used { input: 1000, output: 100, ..Used::default() }, model: model.map(str::to_owned), billed };
        // The list, by the model that was asked.
        let spent = pricing.price(&report(None, None));
        assert_eq!((spent.cost, spent.basis, spent.model.as_str()), (Some(1000 * 1_000_000 + 100 * 2_000_000), Basis::Listed, "big"));
        // A dated name of the asked model is the asked model; another model is priced as itself.
        assert_eq!(pricing.price(&report(Some("big-20261001"), None)).basis, Basis::Listed);
        assert_eq!(pricing.price(&report(Some("small"), None)).cost, Some(1000 * 500_000 + 100 * 1_000_000));
        assert_eq!(pricing.price(&report(Some("other"), None)).basis, Basis::Unknown);
        // The user's own prices win over the list, and a bill over both.
        let own = Pricing { own: Rates::typed("10", "10"), ..pricing.clone() };
        assert_eq!(own.price(&report(None, None)).basis, Basis::Own);
        assert_eq!((own.price(&report(None, Some(77))).cost, own.price(&report(None, Some(77))).basis), (Some(77), Basis::Billed));
        // A subscription's cost is a worth; a model on this computer has none, whatever is reported.
        let plan = Pricing { terms: Terms::Plan, ..pricing.clone() };
        assert_eq!((plan.price(&report(None, Some(77))).basis, plan.price(&report(Some("other"), None)).basis), (Basis::Worth, Basis::Unknown));
        let local = Pricing { terms: Terms::Local, ..pricing };
        assert_eq!((local.price(&report(None, Some(77))).cost, local.price(&report(None, None)).basis), (None, Basis::Local));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test usage::`
Expected: does not compile, `messages`, `completions`, `ollama`, `program`, `Pricing`, `Terms`, `Report` not found.

- [ ] **Step 3: Implement**

Add above the `tests` module:

```rust
/// What an engine reports about one request (a program: about its whole run).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The tokens.
    pub used: Used,
    /// The model that answered, when the provider names it.
    pub model: Option<String>,
    /// What the provider says it charged, when it says.
    pub billed: Option<u64>,
}

/// A number of dollars in `value` as a cost; `None` when there is none.
fn billed(value: Option<&Value>) -> Option<u64> {
    match value {
        Some(Value::Number(dollars)) => scaled(&dollars.to_string(), 12),
        _ => None,
    }
}

/// Takes what a Messages API `usage` says into `used`; a count it doesn't name stays as it was.
// Human: the start of a streamed answer says what the model read, the end what it wrote, and newer versions
// repeat everything at the end. Each count is the latest one said.
pub fn messages(usage: &Value, used: &mut Used) {
    let mut take = |name: &str, into: &mut u64| {
        if let Some(count) = usage.get(name).and_then(Value::as_u64) {
            *into = count;
        }
    };
    take("input_tokens", &mut used.input);
    take("cache_read_input_tokens", &mut used.cached);
    take("cache_creation_input_tokens", &mut used.written);
    take("output_tokens", &mut used.output);
}

/// What a Chat Completions `usage` says; `None` when it says nothing.
// Human: this API counts the tokens read from the cache inside the prompt's tokens, so they are taken off.
// Providers name them in one of two ways.
#[must_use]
pub fn completions(usage: &Value) -> Option<Report> {
    let count = |pointer: &str| usage.pointer(pointer).and_then(Value::as_u64);
    let (prompt, output) = (count("/prompt_tokens").unwrap_or(0), count("/completion_tokens").unwrap_or(0));
    let cached = count("/prompt_tokens_details/cached_tokens").or_else(|| count("/prompt_cache_hit_tokens")).unwrap_or(0).min(prompt);
    let report = Report { used: Used { input: prompt - cached, cached, written: 0, output }, model: None, billed: billed(usage.get("cost")) };
    (!report.used.is_empty() || report.billed.is_some()).then_some(report)
}

/// What the last line of an Ollama answer says; `None` when it says nothing.
#[must_use]
pub fn ollama(line: &Value) -> Option<Report> {
    let count = |name: &str| line.get(name).and_then(Value::as_u64).unwrap_or(0);
    let used = Used { input: count("prompt_eval_count"), output: count("eval_count"), ..Used::default() };
    (!used.is_empty()).then_some(Report { used, model: None, billed: None })
}

/// What the `result` line of Claude Code or Grok says about the run; `None` when it says nothing.
#[must_use]
pub fn program(line: &Value) -> Option<Report> {
    let mut used = Used::default();
    messages(&line["usage"], &mut used);
    let report = Report { used, model: None, billed: billed(line.get("total_cost_usd")) };
    (!report.used.is_empty() || report.billed.is_some()).then_some(report)
}

/// How a provider is paid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Terms {
    /// By the token.
    #[default]
    PerToken,
    /// By subscription: a cost is what the tokens are worth.
    Plan,
    /// Not at all: the model runs on this computer.
    Local,
}

/// What a run needs to know to price what it used.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pricing {
    /// The model the run asks.
    pub model: String,
    /// The prices the user typed in for that model.
    pub own: Option<Rates>,
    /// The price list's prices of the provider's models, by name.
    pub listed: BTreeMap<String, Rates>,
    /// How the provider is paid.
    pub terms: Terms,
}

impl Pricing {
    /// What `report` cost: by the provider's bill, else the user's prices, else the list, else unknown.
    // Human: a provider may answer under a dated name of the model that was asked ("big-20261001"); that is
    // still the asked model. Another model (a hand-over) is priced as itself, from the list.
    #[must_use]
    pub fn price(&self, report: &Report) -> Spent {
        let model = report.model.as_deref().unwrap_or(&self.model);
        let asked = model.starts_with(&self.model);
        let own = self.own.as_ref().filter(|_| asked);
        let listed = self.listed.get(model).or_else(|| self.listed.get(&self.model).filter(|_| asked));
        let (cost, basis) = match (self.terms, report.billed, own, listed) {
            (Terms::Local, ..) => (None, Basis::Local),
            (_, Some(billed), ..) => (Some(billed), Basis::Billed),
            (_, None, Some(own), _) => (Some(own.cost(report.used)), Basis::Own),
            (_, None, None, Some(listed)) => (Some(listed.cost(report.used)), Basis::Listed),
            (_, None, None, None) => (None, Basis::Unknown),
        };
        let basis = if self.terms == Terms::Plan && cost.is_some() { Basis::Worth } else { basis };
        Spent { used: report.used, cost, partial: false, basis, model: model.to_owned() }
    }
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test usage::`
Expected: 8 tests pass.

---

### Task 4: The Messages API tells what it used

**Files:**
- Modify: `src/chat.rs` (`Event`, around line 172)
- Modify: `src/chat/api.rs` (`Turn`, `Turn::take`, `ask`, `once`)
- Modify: `src/chat/critic.rs:97-108`

- [ ] **Step 1: Add the event**

In `src/chat.rs`, in `pub enum Event`, before `Problem`:

```rust
    /// What a request took (a program: what its whole run took).
    Used(usage::Report),
```

In `Chat::hear`, extend the first `match` so the report doesn't count as the model writing:

```rust
            Event::Session(_) | Event::History(_) | Event::Used(_) => {}
```

and in the second `match`, before `Event::Done => {}`, add a line that Task 10 replaces:

```rust
            Event::Used(_) => {}
```

- [ ] **Step 2: Write the failing test**

In `src/chat/api.rs`, add to the `tests` module:

```rust
    #[test]
    fn a_streamed_answer_says_what_it_took_and_which_model_answered() {
        let mut turn = Turn::default();
        for event in [
            json!({ "type": "message_start", "message": { "model": "claude-fable-5-2", "usage": { "input_tokens": 12, "cache_creation_input_tokens": 300, "cache_read_input_tokens": 9000, "output_tokens": 1 } } }),
            json!({ "type": "content_block_start", "index": 0, "content_block": { "type": "text", "text": "" } }),
            json!({ "type": "content_block_delta", "index": 0, "delta": { "type": "text_delta", "text": "Hi" } }),
            json!({ "type": "content_block_stop", "index": 0 }),
            json!({ "type": "message_delta", "delta": { "stop_reason": "end_turn" }, "usage": { "output_tokens": 420 } }),
        ] {
            turn.take(&event).unwrap();
        }
        let report = turn.report().unwrap();
        assert_eq!(report.used, crate::chat::usage::Used { input: 12, cached: 9000, written: 300, output: 420 });
        assert_eq!((report.model.as_deref(), report.billed), (Some("claude-fable-5-2"), None));
        // An answer that says nothing about usage reports nothing.
        assert!(Turn::default().report().is_none());
    }
```

- [ ] **Step 3: Run it to see it fail**

Run: `cargo test chat::api::`
Expected: does not compile, no method `report`.

- [ ] **Step 4: Implement in `api.rs`**

Add `use super::usage::{self, Report, Used};` to the imports.

In `struct Turn`, add:

```rust
    /// The tokens the answer took, as far as the stream has said.
    used: Used,
    /// The model the stream says is answering.
    model: Option<String>,
```

Add to `impl Turn`:

```rust
    /// What the answer took; `None` when the provider said nothing about it.
    fn report(&self) -> Option<Report> {
        (!self.used.is_empty()).then(|| Report { used: self.used, model: self.model.clone(), billed: None })
    }
```

In `Turn::take`, add an arm before `Some("message_delta")`:

```rust
            Some("message_start") => {
                usage::messages(&event["message"]["usage"], &mut self.used);
                self.model = text("/message/model").map(str::to_owned);
                Ok(None)
            }
```

and in the `Some("message_delta")` arm, before `Ok(None)`:

```rust
                usage::messages(&event["usage"], &mut self.used);
```

In `fn ask`, replace the final `Ok(turn)` with:

```rust
    // Human: an answer that broke off has returned above; its tokens are not counted, though the provider
    // may charge for them.
    if let Some(report) = turn.report() {
        tell(Event::Used(report));
    }
    Ok(turn)
```

- [ ] **Step 5: Let the single question report too**

`api::once` answers the reviewer's one question. Change its signature and end:

```rust
pub fn once(endpoint: &str, source: &KeySource, model: &str, system: &str, content: &[Value], used: &dyn Fn(Report)) -> Result<String, String> {
```

and before its final `Ok(...)`:

```rust
    let mut taken = Used::default();
    usage::messages(&answer["usage"], &mut taken);
    if !taken.is_empty() {
        used(Report { used: taken, model: answer["model"].as_str().map(str::to_owned), billed: None });
    }
```

In `src/chat/critic.rs`, give `judge` the same last parameter and pass it on (the Claude Code branch gets it in Task 7; for now it takes `used` only on the API branch):

```rust
pub fn judge(
    engine: &Engine,
    asked: &str,
    mut picture: Vec<Value>,
    proof: bool,
    child: &Mutex<Option<Child>>,
    used: &dyn Fn(super::usage::Report),
) -> Result<Option<String>, String> {
```

```rust
        Engine::Api { wire: Wire::Messages, endpoint, model, key, .. } => super::api::once(endpoint, key, model, brief, &picture, used)?,
```

In `src/chat.rs`, in `Chat::send`, change the call:

```rust
                let verdict = critic::judge(&ask.engine, &files::flat(&ask.text), picture, round == critic::ROUNDS, &held, &|report| {
                    tell(Event::Used(report));
                });
```

- [ ] **Step 6: Run the tests**

Run: `cargo test chat::`
Expected: all chat tests pass, the new one included. (`used` is unused on the Claude Code branch of `judge` until Task 7; that is a warning under `cargo test`, not an error.)

---

### Task 5: Chat Completions asks for usage and tells it

**Files:**
- Modify: `src/chat/openai.rs` (`request`, `Turn`, `Turn::take`, `ask`, `run`, tests)
- Modify: `src/chat/ollama.rs:107` only if it calls `openai::request` (it does not; it has its own)

- [ ] **Step 1: Write the failing tests**

In `src/chat/openai.rs` tests, in `requests_carry_the_brief_the_tools_and_pictures_the_apis_way`, change `request(&job, &history)` to `request(&job, &history, true)` and add after `let body = ...`:

```rust
        // The request asks for the usage to come with the stream, unless the provider won't have that.
        assert_eq!(body["stream_options"], json!({ "include_usage": true }));
        assert!(request(&job, &history, false).get("stream_options").is_none());
```

Add a new test:

```rust
    #[test]
    fn the_streams_last_chunk_says_what_the_answer_took() {
        let mut turn = Turn::default();
        for event in [
            json!({ "choices": [{ "index": 0, "delta": { "content": "Hi" } }] }),
            json!({ "choices": [{ "index": 0, "delta": {}, "finish_reason": "stop" }] }),
            json!({ "choices": [], "usage": { "prompt_tokens": 1000, "completion_tokens": 50, "prompt_tokens_details": { "cached_tokens": 800 } } }),
        ] {
            turn.take(&event).unwrap();
        }
        let report = turn.used.unwrap();
        assert_eq!(report.used, crate::chat::usage::Used { input: 200, cached: 800, written: 0, output: 50 });
        // A provider that says nothing, or an empty usage, reports nothing.
        let mut silent = Turn::default();
        silent.take(&json!({ "choices": [], "usage": {} })).unwrap();
        silent.take(&json!({ "choices": [], "usage": null })).unwrap();
        assert!(silent.used.is_none());
        // An address that refused the question once isn't asked it again.
        assert!(counts("https://a.example/v1/chat/completions"));
        uncounted("https://a.example/v1/chat/completions");
        assert!(!counts("https://a.example/v1/chat/completions") && counts("https://b.example/v1/chat/completions"));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test chat::openai::`
Expected: does not compile.

- [ ] **Step 3: Implement**

Add imports: `use std::cell::Cell;`, `use std::sync::Mutex;`, `use super::usage::{self, Report};`.

Replace `fn request`:

```rust
/// The request for the conversation `history`; `counted` asks for the usage to come with the stream.
fn request(job: &Job<'_>, history: &[Value], counted: bool) -> Value {
    let mut messages = vec![json!({ "role": "system", "content": super::brief(true, job.instructions) })];
    messages.extend(history.iter().cloned());
    let mut body = json!({ "model": job.model, "stream": true, "tools": tools(job.off), "messages": messages });
    if counted {
        body["stream_options"] = json!({ "include_usage": true });
    }
    body
}

/// The addresses that refused a request asking for its usage, until the app closes.
// Human: a few providers that otherwise speak this API refuse a field they don't know. Such a provider is
// asked once per run of the app; after that its requests go without the question, and its answers get no line.
static UNCOUNTED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Whether requests to `endpoint` ask for their usage.
fn counts(endpoint: &str) -> bool {
    !lock(&UNCOUNTED).iter().any(|refused| refused == endpoint)
}

/// Remembers that `endpoint` refuses requests that ask for their usage.
fn uncounted(endpoint: &str) {
    lock(&UNCOUNTED).push(endpoint.to_owned());
}
```

In `pub(super) struct Turn`, add:

```rust
    /// What the answer took, when the provider said.
    pub(super) used: Option<Report>,
```

In `Turn::take`, directly after the `error` check and before `let mut told`:

```rust
        // Human: the usage comes in a chunk of its own with no choices, after the answer; some providers send
        // it with every chunk, growing. The last one said counts.
        if let Some(report) = event.get("usage").and_then(usage::completions) {
            self.used = Some(report);
        }
```

In `fn ask`, replace the final `Ok(turn)`:

```rust
    if let Some(report) = turn.used.take() {
        tell(Event::Used(report));
    }
    Ok(turn)
```

Replace the last line of `pub fn run`:

```rust
    let agent = api::agent();
    // Human: a request refused with "bad request" may be refused for asking about its usage. It is sent once
    // more without the question. If that goes through, the address is remembered; if not, the refusal has
    // another reason (pictures, say) and later requests ask again.
    let counted = Cell::new(counts(job.endpoint));
    converse(job, stop, tell, &|history| {
        let asked = ask(&agent, job, bearer.as_deref(), &request(job, history, counted.get()), stop, tell);
        if !counted.get() || !matches!(asked, Err((400 | 422, _))) {
            return asked;
        }
        let plain = ask(&agent, job, bearer.as_deref(), &request(job, history, false), stop, tell);
        if plain.is_ok() {
            counted.set(false);
            uncounted(job.endpoint);
        }
        plain
    });
```

- [ ] **Step 4: Run the tests**

Run: `cargo test chat::openai::`
Expected: all pass.

---

### Task 6: Ollama tells what it used

**Files:**
- Modify: `src/chat/ollama.rs` (`take`, `ask`, tests)

- [ ] **Step 1: Write the failing test**

In the test `a_streamed_answer_becomes_words_and_whole_tool_calls`, change the last event of the list to carry counts:

```rust
            json!({ "message": { "role": "assistant", "content": "" }, "done": true, "done_reason": "stop", "prompt_eval_count": 17000, "eval_count": 90 }),
```

and add after the `assert_eq!((turn.finish...` line:

```rust
        // The last line says what the answer took.
        assert_eq!(turn.used.as_ref().map(|report| (report.used.input, report.used.output)), Some((17_000, 90)));
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test chat::ollama::`
Expected: FAIL, `turn.used` is `None`.

- [ ] **Step 3: Implement**

In `fn take`, before `Ok(told)`:

```rust
    if event["done"] == true {
        turn.used = super::usage::ollama(event);
    }
```

In `fn ask`, replace the final `Ok(turn)`:

```rust
    if let Some(report) = turn.used.take() {
        tell(Event::Used(report));
    }
    Ok(turn)
```

- [ ] **Step 4: Run the tests**

Run: `cargo test chat::ollama::`
Expected: all pass.

---

### Task 7: Claude Code and Grok tell what a run took

**Files:**
- Modify: `src/chat/code.rs` (`read_line`, `once`, tests)
- Modify: `src/chat/critic.rs:101`

- [ ] **Step 1: Write the failing test**

In `the_programs_lines_become_events`, add a line to the list before the two `result` lines:

```rust
            r#"{"type":"result","subtype":"success","is_error":false,"result":"Hello","total_cost_usd":0.0812,"usage":{"input_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":40,"output_tokens":7}}"#,
```

and add to the expected list, before `r#"Problem("Not logged in")"#`:

```rust
                r#"Used(Report { used: Used { input: 5, cached: 40, written: 0, output: 7 }, model: None, billed: Some(81200000000) })"#,
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test chat::code::`
Expected: FAIL, the `Used` line is missing.

- [ ] **Step 3: Implement**

In `read_line`, replace the `(Some("result"), _) if ... => { ... }` arm:

```rust
        // Human: the last line says how the run went, and what it took and cost.
        (Some("result"), _) => {
            let mut told: Vec<Event> = super::usage::program(&value).map(Event::Used).into_iter().collect();
            if value.get("is_error").and_then(Value::as_bool) == Some(true) {
                told.push(Event::Problem(text("/result").filter(|said| !said.is_empty()).unwrap_or("Claude Code ended with an error.").to_owned()));
            }
            told
        }
```

`code::once` (the reviewer's one question): add a last parameter `used: &dyn Fn(super::usage::Report)` and, inside its loop where it handles `value["type"] == "result"`, add:

```rust
            if let Some(report) = super::usage::program(&value) {
                used(report);
            }
```

In `src/chat/critic.rs`, pass it on:

```rust
        Engine::Code { flavor: Flavor::Claude, program, model } => super::code::once(program, model, brief, &picture, child, used)?,
```

- [ ] **Step 4: Run the tests**

Run: `cargo test chat::`
Expected: all pass.

---

### Task 8: The price list: reading it and finding a provider (`prices.rs`, part 1)

**Files:**
- Create: `src/chat/prices.rs`
- Modify: `src/chat.rs` (module list)
- Modify: `src/chat/catalog.rs` (one test)

- [ ] **Step 1: Declare the module**

In `src/chat.rs`, add after `pub mod plain;`:

```rust
pub mod prices;
```

- [ ] **Step 2: Write the file with failing tests**

Create `src/chat/prices.rs`:

```rust
//! The prices of the providers' models, from the open list at models.dev that the OpenCode project keeps.
//!
//! The list names each provider, the address it is reached at, and every model's prices in dollars per
//! million tokens. It is fetched once a day, cut down to addresses and prices, and kept in the home folder.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::usage::{self, Prices, Rates, Terms};

/// Where the list is.
pub const SOURCE: &str = "https://models.dev/api.json";

/// The list's name for each provider the settings window knows, by the address Pencraft reaches it at.
// Human: the list gives an address for some providers only, and for Meta and Kimi a different one than the
// one Pencraft uses; so the known providers are named here. Looked up on 2026-11-20.
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

/// An address as it is compared: without spaces around it or a slash at its end.
fn plain(address: &str) -> &str {
    address.trim().trim_end_matches('/')
}

/// The list's name for the known provider reached at `base`.
pub(super) fn known(base: &str) -> Option<&'static str> {
    KNOWN.iter().find(|(address, _)| *address == plain(base)).map(|(_, name)| *name)
}

/// How the provider reached at `base` is paid: by subscription for the known plans, else by the token.
#[must_use]
pub fn terms(base: &str) -> Terms {
    if PLANS.contains(&plain(base)) { Terms::Plan } else { Terms::PerToken }
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
    /// The list written in `text`: the whole one from models.dev, or the cut of it Pencraft keeps.
    #[must_use]
    pub fn parse(text: &str) -> Option<Catalogue> {
        let list = serde_json::from_str::<Value>(text).ok()?;
        let providers = list
            .as_object()?
            .iter()
            .map(|(name, provider)| {
                let models = provider.get("models").and_then(Value::as_object).into_iter().flatten();
                let models = models.filter_map(|(id, model)| Some((id.clone(), rates(model.get("cost")?)?))).collect();
                (name.clone(), Provider { api: provider.get("api").and_then(Value::as_str).map(plain).unwrap_or_default().to_owned(), models })
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
        let found = match known(base) {
            Some(name) => self.providers.get(name),
            None => self.providers.values().find(|provider| !provider.api.is_empty() && provider.api == plain(base)),
        };
        found.map(|provider| provider.models.clone()).unwrap_or_default()
    }
}

/// The list in `text` cut down to what Pencraft reads: each provider's address and its models' costs.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A cut of what models.dev answered on 2026-11-20, with the fields Pencraft doesn't read left in.
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
```

The ignored test calls `fetch`, which Task 9 adds; leave that one test out until then if the file is to compile now.

- [ ] **Step 3: Tie the table to the settings window's providers**

In `src/chat/catalog.rs`, add to the test `known_providers_come_with_their_address_and_api`:

```rust
        // Every provider that takes a key is one the price list is asked about.
        assert!(PRESETS.iter().filter(|(name, _)| *name != OLLAMA).all(|(_, p)| super::super::prices::known(p.base_url).is_some()));
```

- [ ] **Step 4: Run the tests**

Run: `cargo test chat::prices:: && cargo test chat::catalog::`
Expected: all pass.

---

### Task 9: The price list on disk, fetched once a day (`prices.rs`, part 2)

**Files:**
- Modify: `src/chat/prices.rs`
- Modify: `src/prefs.rs` (`Agents`)
- Modify: `src/app.rs` (`App`, `App::new`)

- [ ] **Step 1: Add the switch to the preferences**

In `src/prefs.rs`, `pub struct Agents`, add after `review`:

```rust
    /// Fetch the public price list of the providers' models once a day, to say what an answer cost.
    pub prices: bool,
```

and in `impl Default for Agents`, after `review: true,`:

```rust
            prices: true,
```

- [ ] **Step 2: Write the failing test**

In `src/chat/prices.rs` tests:

```rust
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
```

- [ ] **Step 3: Run it to see it fail**

Run: `cargo test chat::prices::`
Expected: does not compile, `stale` and `read` not found.

- [ ] **Step 4: Implement**

Add imports at the top of `src/chat/prices.rs`:

```rust
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::app::lock;
```

Add above the `tests` module:

```rust
/// How old the kept list may get before it is fetched again.
const EVERY: Duration = Duration::from_secs(24 * 60 * 60);
/// The most the list may weigh; it was 6.8 megabytes in November 2026.
const MOST_BYTES: u64 = 64 * 1024 * 1024;

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
```

Check two things against the code as it is: `craft_atomic::atomic_write`'s error type must convert to or match `std::io::Error` for the `and_then` (it is used with `map_err` in `src/chat/history.rs:106`; if its error is not `io::Error`, write the two steps as separate `if let Err` blocks), and `with_config().limit(..)` against ureq 3.5.0's `Body` documentation.

If Task 8 left the ignored test out, add it now.

- [ ] **Step 5: Hold the list in the app**

In `src/app.rs`, `pub struct App`, add:

```rust
    /// The price list of the providers' models, which a fetch on another thread may replace.
    prices: std::sync::Arc<std::sync::Mutex<crate::chat::prices::Catalogue>>,
```

In `App::new`, before the `App { ... }` literal is built, work out whether to look (the preferences are in scope there; use the name they have):

```rust
        // Human: the list is looked up for someone who pays a provider by the token: a provider with a key,
        // or a key in the environment. A picture of the app fetches nothing.
        let agents = &prefs.get().agents;
        let look = agents.prices
            && launch.screenshot.is_none()
            && (agents.providers.iter().any(|provider| provider.key_ref.is_some()) || crate::chat::key_in_environment());
```

and in the literal:

```rust
            prices: crate::chat::prices::start(look),
```

- [ ] **Step 6: Run the tests**

Run: `cargo test chat::prices:: && cargo test prefs::`
Expected: all pass.

Run: `cargo test chat::prices:: -- --ignored`
Expected: `the_real_list_knows_every_provider_named_here` passes (needs the network). If a provider is gone from the list, fix `KNOWN` and the table in the spec.

---

### Task 10: The chat sums a run and puts the sum on its last line

**Files:**
- Modify: `src/prefs.rs` (`ProviderModel`)
- Modify: `src/chat.rs` (`Entry`, `Ask`, `Run`, `Chat`, `choices`, new `pricing`)
- Modify: `src/app/talk.rs:387-425` (`start_chat`)
- Modify: `src/settings.rs` (only to keep it compiling: the `ProviderModel` literals)

- [ ] **Step 1: Add the user's own prices to a model**

In `src/prefs.rs`, `pub struct ProviderModel`, add:

```rust
    /// What a million tokens of input cost in dollars, as the user typed it; empty to go by the price list.
    pub input_price: String,
    /// The same for output.
    pub output_price: String,
```

In `src/settings.rs`, the two `ProviderModel { id: ..., context_window: ..., max_tokens: ... }` literals (in `ModelRow::model` near line 224 and the test near line 2627) get `..ProviderModel::default()` as their last field. Task 13 fills them properly.

- [ ] **Step 2: Write the failing tests**

In `src/chat.rs` tests, add (the module already has helpers for building a `Chat`; this test drives `hear` and `close` directly, which need no run):

```rust
    #[test]
    fn a_runs_requests_are_summed_onto_its_last_line() {
        use usage::{Basis, Pricing, Rates, Report, Used};
        let pricing = Pricing { model: "m".to_owned(), own: Rates::typed("1", "2"), ..Pricing::default() };
        let report = |input, output| Report { used: Used { input, output, ..Used::default() }, model: None, billed: None };
        let mut chat = Chat::default();
        chat.entries.push(Entry::new(Role::User, "Make a card"));
        let from = chat.entries.len();
        chat.entries.push(Entry::new(Role::Assistant, "On it."));
        chat.entries.push(Entry::new(Role::Step, "Changed 2 layers"));
        chat.entries.push(Entry::new(Role::Assistant, "Done."));
        chat.count(&pricing, &report(1000, 100));
        chat.count(&pricing, &report(2000, 50));
        chat.close(from);
        let spent = chat.entries[3].spent.clone().unwrap();
        assert_eq!((spent.used.input, spent.used.output, spent.cost, spent.basis), (3000, 150, Some(3000 * 1_000_000 + 150 * 2_000_000), Basis::Own));
        assert!(chat.entries[1].spent.is_none());
        assert_eq!(chat.total, spent);
        // The next run ends in a problem; its line is under the problem, and the total grows.
        chat.entries.push(Entry::new(Role::User, "Again"));
        let from = chat.entries.len();
        chat.entries.push(Entry::new(Role::Problem, "The provider didn't answer."));
        chat.count(&pricing, &report(500, 0));
        chat.close(from);
        assert_eq!(chat.entries[5].spent.as_ref().map(|spent| spent.used.input), Some(500));
        assert_eq!(chat.total.used.input, 3500);
        // A run that used nothing, and one that left no line of its own, mark nothing.
        chat.close(chat.entries.len());
        chat.count(&pricing, &report(7, 0));
        chat.close(chat.entries.len());
        assert_eq!((chat.total.used.input, chat.entries.iter().filter(|entry| entry.spent.is_some()).count()), (3507, 2));
        // A new conversation starts at nothing.
        chat.clear();
        assert!(chat.total.is_empty());
    }

    #[test]
    fn a_choice_knows_how_its_provider_is_priced() {
        use crate::prefs::ProviderModel;
        use usage::Terms;
        let list = prices::Catalogue::parse(
            r#"{ "xai": { "models": { "grok-4.5": { "cost": { "input": 2, "output": 6 } } } },
                 "ollama-cloud": { "models": { "kimi-k3": { "cost": { "input": 1, "output": 4 } } } } }"#,
        )
        .unwrap();
        let model = |id: &str, input: &str, output: &str| ProviderModel {
            id: id.to_owned(),
            input_price: input.to_owned(),
            output_price: output.to_owned(),
            ..ProviderModel::default()
        };
        let provider = |name: &str, api: &str, base: &str, models: Vec<ProviderModel>| CustomProvider {
            name: name.to_owned(),
            api: api.to_owned(),
            base_url: base.to_owned(),
            models,
            key_ref: Some("k".to_owned()),
        };
        let agents = Agents {
            local: Vec::new(),
            providers: vec![
                provider("Grok", COMPLETIONS_API, "https://api.x.ai/v1", vec![model("grok-4.5", "", ""), model("grok-mini", "0.5", "1")]),
                provider("Cloud", COMPLETIONS_API, "https://ollama.com/v1", vec![model("kimi-k3", "", "")]),
                provider("Here", OLLAMA_API, "http://localhost:11434", vec![model("qwen3", "", "")]),
            ],
            ..Agents::default()
        };
        let all = choices(&agents, &|_| None, false);
        let of = |id: &str| pricing(chosen(&all, id).unwrap(), &agents, &list);
        let grok = of("api/Grok/grok-4.5");
        assert_eq!((grok.model.as_str(), grok.own.is_none(), grok.listed.len(), grok.terms), ("grok-4.5", true, 1, Terms::PerToken));
        assert!(of("api/Grok/grok-mini").own.is_some());
        assert_eq!(of("api/Cloud/kimi-k3").terms, Terms::Plan);
        assert_eq!((of("api/Here/qwen3").terms, of("api/Here/qwen3").listed.len()), (Terms::Local, 0));
        // With the look-up turned off the list is not used; the user's own prices still are.
        let off = Agents { prices: false, ..agents.clone() };
        let quiet = |id: &str| pricing(chosen(&all, id).unwrap(), &off, &list);
        assert!(quiet("api/Grok/grok-4.5").listed.is_empty() && quiet("api/Grok/grok-mini").own.is_some());
    }
```

`chosen` takes the id the test gives; if its matching differs from an exact id (read `chosen` at `src/chat.rs:573`), find the choice with `all.iter().find(|choice| choice.id == id)` instead.

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test chat::tests::`
Expected: does not compile.

- [ ] **Step 4: Implement in `src/chat.rs`**

`Entry` gains a field, and `Entry::new` sets it:

```rust
    /// For the last line of a run: what the run took and cost.
    pub spent: Option<usage::Spent>,
```

```rust
        Entry { role, text: text.into(), pointed: Vec::new(), step: None, asked: None, about: Vec::new(), spent: None }
```

`Ask` gains:

```rust
    /// How what the run uses is priced.
    pub pricing: usage::Pricing,
```

`Run` gains:

```rust
    /// How what the run uses is priced.
    pricing: usage::Pricing,
    /// How many lines the conversation had when the run began: its own lines come after.
    from: usize,
```

`Chat` gains:

```rust
    /// What the run under way has taken so far.
    spent: usage::Spent,
    /// What the whole conversation has taken.
    pub total: usage::Spent,
```

In `Chat::send`: read `let from = self.entries.len();` right after the user's entry is pushed, take `let pricing = ask.pricing.clone();` next to `let (model, via) = ...`, and put both into the `Run { ... }` literal.

Add to `impl Chat`:

```rust
    /// Adds what one request took, priced by `pricing`, to the run's sum.
    fn count(&mut self, pricing: &usage::Pricing, report: &usage::Report) {
        self.spent.add(&pricing.price(report));
    }

    /// The run whose lines begin at `from` is over: its sum goes on its last line and into the conversation's.
    // Human: the last line is the assistant's last words, or the problem the run ended with. A run that left
    // neither (it was stopped before it wrote) still counts in the conversation's sum.
    fn close(&mut self, from: usize) {
        let spent = std::mem::take(&mut self.spent);
        if spent.is_empty() {
            return;
        }
        self.total.add(&spent);
        let last = self.entries.iter().rposition(|entry| matches!(entry.role, Role::Assistant | Role::Problem)).filter(|at| *at >= from);
        if let Some(entry) = last.and_then(|at| self.entries.get_mut(at)) {
            entry.spent = Some(spent);
        }
    }
```

In `Chat::hear`, replace the placeholder arm from Task 4:

```rust
            Event::Used(report) => {
                if let Some(pricing) = self.run.as_ref().map(|run| run.pricing.clone()) {
                    self.count(&pricing, &report);
                }
            }
```

In `Chat::poll`, replace the `if over { ... }` block's first statement so the run's start is known:

```rust
        if over {
            let run = self.run.take();
            let from = run.as_ref().map_or(self.entries.len(), |run| run.from);
            // Human: a message handed to the run that it ended without taking goes first among the waiting ones.
            if let Some(inbox) = run.and_then(|run| run.steering) {
                let unheard: Vec<Waiting> = std::mem::take(&mut *lock(&inbox)).into_iter().map(|steer| steer.message).collect();
                self.waiting.splice(0..0, unheard);
            }
            self.thought_over(now);
            self.quiet = None;
            self.settle();
            self.close(from);
        }
```

In `Chat::stop`, after `self.settle();`:

```rust
        self.close(run.from);
```

Pull the environment's address out of `choices` so `pricing` can use it too:

```rust
/// Where a key in the environment belongs: `ANTHROPIC_BASE_URL`, as for Anthropic's own tools, or Anthropic's API.
fn environment_base() -> String {
    std::env::var("ANTHROPIC_BASE_URL").ok().filter(|base| !base.trim().is_empty()).unwrap_or_else(|| ANTHROPIC.to_owned())
}
```

and in `choices` replace the `let base = ...` line with `let base = environment_base();`.

Add after `chosen`:

```rust
/// How what `choice` uses is priced: the user's own prices for its model, the price list's for its provider,
/// and how the provider is paid.
// Human: a program (Claude Code, Grok) says itself what a run cost, and doesn't say whether the user pays by
// the token or by plan; its cost is shown as a worth. A model on this computer costs nothing.
#[must_use]
pub fn pricing(choice: &Choice, agents: &Agents, list: &prices::Catalogue) -> usage::Pricing {
    match &choice.engine {
        Engine::Code { model, .. } => usage::Pricing { model: model.clone(), terms: usage::Terms::Plan, ..usage::Pricing::default() },
        Engine::Plain { model, .. } => usage::Pricing { model: model.clone(), ..usage::Pricing::default() },
        Engine::Api { model, .. } => {
            let provider = agents.providers.iter().find(|provider| choice.id == format!("api/{}/{model}", provider.name));
            let base = provider.map_or_else(environment_base, |provider| provider.base_url.clone());
            let typed = provider.and_then(|provider| provider.models.iter().find(|known| known.id == *model));
            let local = catalog::is_local_ollama(&base);
            usage::Pricing {
                model: model.clone(),
                own: typed.and_then(|typed| usage::Rates::typed(&typed.input_price, &typed.output_price)),
                listed: if agents.prices && !local { list.models(&base) } else { std::collections::BTreeMap::new() },
                terms: if local { usage::Terms::Local } else { prices::terms(&base) },
            }
        }
    }
}
```

Every test in the crate that builds an `Ask` literal now needs `pricing: usage::Pricing::default(),`; the compiler lists them.

- [ ] **Step 5: Hand the pricing to a run**

In `src/app/talk.rs`, `start_chat`: `picked` also carries the pricing.

```rust
        let picked = self.assistant().map(|choice| {
            let pricing = chat::pricing(choice, &self.prefs.get().agents, &crate::app::lock(&self.prices));
            (choice.engine.clone(), choice.model_name().to_owned(), pricing)
        });
        let (Ok(attached), None, Some((engine, model, pricing))) = (attached, &refusal, picked) else {
```

and add `pricing,` to the `Ask { ... }` literal.

- [ ] **Step 6: Run the tests**

Run: `cargo test chat::`
Expected: all pass.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean, or only complaints about what Tasks 11 to 13 still add (`Chat::total` and `Entry::spent` are read there). Fix anything else now.

---

### Task 11: The sums are kept with the design's chat

**Files:**
- Modify: `src/chat/history.rs`
- Modify: `src/app/talk.rs:113` and `:217`

- [ ] **Step 1: Write the failing test**

In `src/chat/history.rs` tests:

```rust
    #[test]
    fn what_an_answer_took_and_the_conversations_sum_come_back() {
        use crate::chat::usage::{Basis, Spent, Used};
        let root = root().join("spent");
        drop(std::fs::remove_dir_all(&root));
        let design = Path::new("/designs/shop.pen");
        let spent = Spent { used: Used { input: 10_200, cached: 2_200, written: 0, output: 1_800 }, cost: Some(80_000_000_000), partial: false, basis: Basis::Listed, model: "m".to_owned() };
        let total = Spent { used: Used { input: 90_000, output: 9_000, ..Used::default() }, cost: Some(610_000_000_000), partial: true, basis: Basis::Mixed, model: "m".to_owned() };
        let said = [Entry::new(Role::User, "Make a card"), Entry { spent: Some(spent.clone()), ..Entry::new(Role::Assistant, "Done.") }];
        keep_in(&root, design, &said, &total).unwrap();
        let (back, sum) = read_in(&root, design);
        assert_eq!((back[1].spent.as_ref(), back[0].spent.as_ref(), &sum), (Some(&spent), None, &total));
        // A file from before sums were kept reads as it did, with nothing spent.
        let file = file_in(&root, design);
        std::fs::write(&file, r#"{"design":"/designs/shop.pen","said":[{"role":"assistant","text":"Done."}]}"#).unwrap();
        let (back, sum) = read_in(&root, design);
        assert_eq!((back.len(), back[0].spent.as_ref(), sum.is_empty()), (1, None, true));
        keep_in(&root, design, &[], &Spent::default()).unwrap();
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test chat::history::`
Expected: does not compile.

- [ ] **Step 3: Implement**

Add `use super::usage::Spent;` to the imports.

In `fn kept`, replace the assistant's arm:

```rust
            Role::Assistant => {
                let mut line = json!({ "role": "assistant", "text": entry.text });
                if let Some(spent) = &entry.spent {
                    line["spent"] = spent.kept();
                }
                Some(line)
            }
```

`keep` and `keep_in` take the conversation's sum and write it beside what was said:

```rust
pub fn keep(design: &Path, entries: &[Entry], total: &Spent) -> Result<(), String> {
    keep_in(&root(), design, entries, total)
}

fn keep_in(root: &Path, design: &Path, entries: &[Entry], total: &Spent) -> Result<(), String> {
```

```rust
    let mut whole = json!({ "design": design.display().to_string(), "said": said });
    if !total.is_empty() {
        whole["total"] = total.kept();
    }
    let text = whole.to_string();
```

`read` and `read_in` give the sum back too:

```rust
/// The conversation kept for the design at `design`, oldest line first, and what it took in all; nothing when
/// there is none, or its file can't be read.
#[must_use]
pub fn read(design: &Path) -> (Vec<Entry>, Spent) {
    read_in(&root(), design)
}

fn read_in(root: &Path, design: &Path) -> (Vec<Entry>, Spent) {
    let Some(kept) = std::fs::read(file_in(root, design)).ok().and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok()) else {
        return (Vec::new(), Spent::default());
    };
    let said = kept
        .get("said")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|line| {
            let role = match line.get("role").and_then(Value::as_str)? {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                _ => return None,
            };
            let text = line.get("text").and_then(Value::as_str)?;
            Some(Entry { pointed: pictures_in(line, text), spent: line.get("spent").and_then(Spent::from_kept), ..Entry::new(role, text) })
        })
        .collect();
    (said, kept.get("total").and_then(Spent::from_kept).unwrap_or_default())
}
```

Update the three existing tests in this file to the new signatures: `keep_in(&root, design, &said, &Spent::default())` and `read_in(&root, design).0`.

- [ ] **Step 4: Update the two callers**

`src/app/talk.rs`, in `front_talk`:

```rust
                (talk.chat.entries, talk.chat.total) = chat::history::read(&file);
```

and where the history is kept:

```rust
            if let Err(problem) = chat::history::keep(&now.0, &talk.chat.entries, &talk.chat.total) {
```

- [ ] **Step 5: Run the tests**

Run: `cargo test chat::history:: && cargo test app::`
Expected: all pass.

---

### Task 12: The line under an answer

**Files:**
- Modify: `src/theme.rs` (near `CHAT_NOTE_GAP`, line 145)
- Modify: `src/conversation.rs` (`Looks`, `line`, the call in `show` near line 106, and any other caller of `line`)

- [ ] **Step 1: Add the sizes to the theme**

In `src/theme.rs`, after `CHAT_NOTE_GAP`:

```rust
/// The room between an answer and the line saying what it took.
pub const CHAT_SPENT_GAP: f32 = 3.0;
/// How faint that line is, as a share of the muted text's strength.
pub const CHAT_SPENT_INK: f32 = 0.7;
```

- [ ] **Step 2: Paint the line**

In `src/conversation.rs`, add `CHAT_SPENT_GAP, CHAT_SPENT_INK` to the `use crate::theme::{...}` list and `use crate::chat::usage::Spent;` to the imports.

`Looks` learns the conversation's sum. It becomes `Looks<'a>`:

```rust
/// How a line of the conversation is drawn.
#[derive(Clone, Copy)]
struct Looks<'a> {
    /// How much larger than small the text is (the settings' chat text size).
    scale: f32,
    palette: Palette,
    /// The time a note's icon moves by; `None` when nothing moves.
    clock: Option<Clock>,
    /// The line is the question that waits for the user: its answers can be picked.
    live: bool,
    /// What the whole conversation took, for the hover of an answer's own sum.
    total: &'a Spent,
}
```

Add:

```rust
/// What the run that ended with this line took and cost, one faint line; the hover says the parts.
// Human: the number is there for whoever wants it and out of the way of the conversation: smaller and fainter
// than a note, and nothing that can be clicked.
fn spent_line(ui: &mut Ui, spent: &Spent, total: &Spent, scale: f32, palette: Palette) {
    ui.add_space(CHAT_SPENT_GAP);
    let words = RichText::new(spent.line()).size((SIDEBAR_TEXT_SIZE - 1.0) * scale).color(palette.muted.gamma_multiply(CHAT_SPENT_INK));
    ui.add(egui::Label::new(words).selectable(false)).on_hover_text(spent.details(total).join("\n"));
}
```

In `fn line`, destructure `total` too (`let Looks { scale, palette, clock, live, total } = looks;`) and change the two arms:

```rust
        Role::Assistant => {
            ui.add(words(shown, palette.foreground));
            // Human: the sum waits until the answer's words are all out.
            if let Some(spent) = entry.spent.as_ref().filter(|_| shown.len() == entry.text.len()) {
                spent_line(ui, spent, total, scale, palette);
            }
        }
```

```rust
        Role::Problem => {
            ui.add(words(shown, PROBLEM));
            trouble(ui, entry, scale, palette);
            if let Some(spent) = &entry.spent {
                spent_line(ui, spent, total, scale, palette);
            }
        }
```

At the call near line 106, pass the chat's sum: `Looks { scale, palette, clock, live: open == Some(index), total: &chat.total }`. Search the file for other `Looks {` literals and `line(` calls (`pinned` at line 295 draws a question; it has `chat` too) and give each `total: &chat.total`.

- [ ] **Step 3: Build and look**

Run: `cargo test conversation::`
Expected: all pass.

Run: `cargo run`, open a design, pick an assistant with an API key, send "Add a blue rectangle". When the answer is done, a faint line such as `17.3k in · 212 out · $0.14` stands under it; hovering it lists input, cache, output, model, the price's source. Send a second message and check that the second hover ends with "This conversation: …" and the first line is unchanged. Close and reopen the design: both lines are back.

If no key is at hand, the chat's sample conversation (`Chat::sample`, used by the screenshot launch) can carry a made-up `spent` on its last assistant line to look at the painting: add it there only if the project's golden pictures are regenerated in the same change (`golden/`), otherwise check with a real answer.

---

### Task 13: The switch and the two price fields in the settings

**Files:**
- Modify: `src/settings.rs` (search table near line 162, hints near line 82, `ModelRow` near 206, `Form::of` near 344, row near 1577, `model_rows` near 2167, tests near 2627)

- [ ] **Step 1: Write the failing tests**

In the settings tests, next to the test that checks `ModelRow` validation (near line 2640, the one with `change(|f| f.models[0].max_tokens = "0".to_owned()).is_none()`), add in the same style:

```rust
        // Prices are optional, but one alone or a word is no price.
        assert!(change(|f| f.models[0].input_price = "3".to_owned()).is_none());
        assert!(change(|f| (f.models[0].input_price, f.models[0].output_price) = ("3".to_owned(), "lots".to_owned())).is_none());
        let priced = change(|f| (f.models[0].input_price, f.models[0].output_price) = (" 3 ".to_owned(), "15.5".to_owned())).unwrap();
        assert_eq!((priced.models[0].input_price.as_str(), priced.models[0].output_price.as_str()), ("3", "15.5"));
```

(`change` is that test's own helper returning the saved provider; if it returns something else, read the fields from what it returns.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test settings::`
Expected: does not compile, `ModelRow` has no `input_price`.

- [ ] **Step 3: Implement the fields**

`ModelRow` and its two constructors:

```rust
struct ModelRow {
    id: String,
    context_window: String,
    max_tokens: String,
    input_price: String,
    output_price: String,
}

impl ModelRow {
    fn empty() -> ModelRow {
        ModelRow {
            id: String::new(),
            context_window: DEFAULT_TOKENS.to_owned(),
            max_tokens: DEFAULT_TOKENS.to_owned(),
            input_price: String::new(),
            output_price: String::new(),
        }
    }

    /// The model this row describes, if its name and both counts are usable, and its prices are both there
    /// as numbers or both left empty.
    fn model(&self) -> Option<ProviderModel> {
        let count = |text: &str| text.trim().parse::<u32>().ok().filter(|n| *n > 0);
        let id = self.id.trim();
        (!id.is_empty()).then_some(())?;
        let (input, output) = (self.input_price.trim(), self.output_price.trim());
        let priced = crate::chat::usage::Rates::typed(input, output).is_some();
        (priced || (input.is_empty() && output.is_empty())).then_some(())?;
        Some(ProviderModel {
            id: id.to_owned(),
            context_window: count(&self.context_window)?,
            max_tokens: count(&self.max_tokens)?,
            input_price: input.to_owned(),
            output_price: output.to_owned(),
        })
    }
}
```

In `Form::of`, the `.map(...)` builds the row with the kept prices:

```rust
            .map(|m| ModelRow {
                id: m.id.clone(),
                context_window: m.context_window.to_string(),
                max_tokens: m.max_tokens.to_string(),
                input_price: m.input_price.clone(),
                output_price: m.output_price.clone(),
            })
```

Other `ModelRow { ... }` literals (the tests near line 2827, the picker's `None => self.models.push(ModelRow { id: id.to_owned(), ..ModelRow::empty() })`) already use `..ModelRow::empty()` or need the two fields added as empty strings; the compiler lists them.

In `model_rows`, five fields share the row:

```rust
    // Human: five fields side by side and a bin; only the first row carries the fields' names. The prices are
    // dollars for a million tokens and may stay empty: the price list is asked then.
    let gap = 8.0;
    let bin = 30.0;
    let fields = ui.available_width() - bin - 5.0 * gap;
    let widths = [fields * 0.34, fields * 0.18, fields * 0.18, fields * 0.15, fields * 0.15];
```

```rust
        for (text, name, example, width) in [
            (&mut model.id, "Model ID", "my-model-id", widths[0]),
            (&mut model.context_window, "Context window", DEFAULT_TOKENS, widths[1]),
            (&mut model.max_tokens, "Max tokens", DEFAULT_TOKENS, widths[2]),
            (&mut model.input_price, "Input $/M", "from list", widths[3]),
            (&mut model.output_price, "Output $/M", "from list", widths[4]),
        ] {
```

- [ ] **Step 4: Implement the switch**

Next to `REVIEW_HINT`:

```rust
/// What the "Look up model prices" row says under its name.
const PRICES_HINT: &str = "To say what an answer cost, the public price list at models.dev is fetched once a day. Nothing about you or your chats is sent. Turned off, a price shows only where the provider bills each answer or you typed prices in for a model.";
```

In the search table, after the `"Have finished designs reviewed"` entry:

```rust
    (Page::Chat, "Conversation", "Look up model prices"),
```

After the review row (near line 1577):

```rust
        rows.row("Look up model prices", Some(PRICES_HINT), Lead::Nothing, |ui| switch(ui, &mut agents.prices, palette));
```

- [ ] **Step 5: Run the tests and look**

Run: `cargo test settings::`
Expected: all pass.

Run: `cargo run -- --settings` (or open Settings in the running app), find the switch under Chat → Conversation, open a provider's form and check that the five fields fit at the window's smallest width and that Save is refused with only one price typed.

---

### Task 14: Settle the open points, then the full gate

**Files:**
- Modify: tests in `src/chat/openai.rs`, `src/chat/code.rs` as the captures require
- Modify: `docs/superpowers/specs/2026-11-20-token-usage-and-price-design.md` (the "Open points" section)
- Modify: `CHANGELOG.md`

Each open point of the spec is settled by one real answer. Ask the user which keys and programs they have; do the ones that can be done and leave the others listed as open in the spec.

- [ ] **Step 1: Chat Completions providers and `stream_options`**

For each connected Chat Completions provider, run the app with `RUST_LOG=debug` (or add a temporary `log::debug!` of the final chunk in `openai::ask`), send one message, and check a line appears under the answer. A provider with no line either refused the field (the fallback worked; nothing to do) or sends usage under names `usage::completions` doesn't read: add those names there with a test case in `each_apis_usage_is_brought_to_one_shape`.

- [ ] **Step 2: OpenRouter's cost**

With an OpenRouter key, send one message and hover the line: it must say "as the provider billed it". If it says "from the list at models.dev", OpenRouter no longer sends `usage.cost` unasked: add `"usage": { "include": true }` to the request body in `openai::request` when `counted` and the endpoint starts with `https://openrouter.ai/`, with a test.

- [ ] **Step 3: Claude Code's and Grok's result line**

Run: `claude -p "Say hi" --output-format stream-json --verbose | tail -n 1`
Compare the last line's field names with what `usage::program` reads (`usage.input_tokens`, `usage.cache_read_input_tokens`, `usage.cache_creation_input_tokens`, `usage.output_tokens`, `total_cost_usd`). Replace the made-up line in `the_programs_lines_become_events` with the captured one (shortened), and fix `program` if a name differs. Do the same for Grok (`grok -p "Say hi" --output-format streaming-messages-json`); if its last line carries nothing, Grok simply gets no line, and the spec says so.

- [ ] **Step 4: Meta's model names**

With a Meta key, load the provider's models in Settings and compare a few names with the `meta` entry of `~/.pencraft/prices.json`. If they differ, Meta's models have no price until the list catches up; note it in the spec. Do not map names by hand.

- [ ] **Step 5: Write down what was found**

In the spec, replace the "Open points to settle while building" section with what each check showed, and set the status line to "built". Add to `CHANGELOG.md`, in the style of its newest entry, one entry: the chat says what each answer took in tokens and what it cost; prices come from the provider's bill, the user's own prices, or the public list at models.dev (a switch in Settings turns the look-up off).

- [ ] **Step 6: The full gate**

Run: `scripts/check.sh`
Expected: `check: all passed`. Fix every format, clippy (pedantic, dead code included) and test failure; do not silence a lint.

Run: `cargo test -- --ignored the_real_list`
Expected: PASS.

- [ ] **Step 7: Report**

Tell the user what was built, which open points were settled and which were not, and that nothing is committed.
