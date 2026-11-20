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
        Some((mantissa, written)) => (mantissa, exponent(written)?),
        None => (text, 0),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{whole}{fraction}");
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let number: u128 = digits.parse().ok()?;
    // Human: an exponent no number of this world has must not wrap around: the sum stops at the ends of its
    // range, where the number is too large to hold (`None`) or too small to count (nothing).
    let shift = exponent.saturating_add(i64::from(places)).saturating_sub(i64::try_from(fraction.len()).ok()?);
    let power = |by: u64| u32::try_from(by).ok().and_then(|by| 10_u128.checked_pow(by));
    let out = if shift >= 0 { number.checked_mul(power(shift.unsigned_abs())?)? } else { power(shift.unsigned_abs()).map_or(0, |by| number / by) };
    u64::try_from(out).ok()
}

/// The exponent written in `text` (a sign, then digits); one too large to hold is the largest there is.
/// `None` for anything else.
fn exponent(text: &str) -> Option<i64> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let size = digits.parse::<i64>().unwrap_or(i64::MAX);
    Some(if negative { -size } else { size })
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
        [Basis::Billed, Basis::Own, Basis::Listed, Basis::Worth, Basis::Local, Basis::Mixed]
            .into_iter()
            .find(|basis| basis.name() == name)
            .unwrap_or_default()
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
        if tenths.is_multiple_of(10) { format!("{}{letter}", tenths / 10) } else { format!("{}.{}{letter}", tenths / 10, tenths % 10) }
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
    let take = |name: &str, into: &mut u64| {
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
// Providers name them in one of three ways.
// Human: some providers leave a model's thinking out of the completion's tokens and count it only in the
// total; what was written is then the total less the prompt, when that is more.
#[must_use]
pub fn completions(usage: &Value) -> Option<Report> {
    let count = |pointer: &str| usage.pointer(pointer).and_then(Value::as_u64);
    let (prompt, written) = (count("/prompt_tokens").unwrap_or(0), count("/completion_tokens").unwrap_or(0));
    let output = count("/total_tokens").map_or(written, |total| written.max(total.saturating_sub(prompt)));
    let cached = count("/prompt_tokens_details/cached_tokens")
        .or_else(|| count("/prompt_cache_hit_tokens"))
        .or_else(|| count("/cached_tokens"))
        .unwrap_or(0)
        .min(prompt);
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
// Human: the chat starts one program per message and resumes the session, and Claude Code's `total_cost_usd` and
// `modelUsage` then add up the whole session, not this message: the second answer of a session would show the
// cost of both. Its `model_usage_since_last_result` is this message's alone, so the cost is read from there;
// `usage` is this message's already. A run that used nothing has the object, empty: that is no bill, not the
// session's `total_cost_usd`. Grok has no such object at all, and its `total_cost_usd` is per message.
#[must_use]
pub fn program(line: &Value) -> Option<Report> {
    let mut used = Used::default();
    messages(&line["usage"], &mut used);
    let since = line.get("model_usage_since_last_result").and_then(Value::as_object);
    let billed = match since {
        Some(models) => models.values().filter_map(|model| billed(model.get("costUSD"))).reduce(u64::saturating_add),
        None => billed(line.get("total_cost_usd")),
    };
    let named = since.filter(|models| !models.is_empty()).or_else(|| line.get("modelUsage").and_then(Value::as_object));
    let model = named.filter(|models| models.len() == 1).and_then(|models| models.keys().next().cloned());
    let report = Report { used, model, billed };
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

/// Whether `named` is the model `asked`: by that very name, or by it with a date after it ("big-20261001",
/// "big@20261001").
// Human: a name that only begins like the asked one ("big-mini") is another model, with other prices; and
// a run that asked for no model by name can't be told from any.
fn dated(named: &str, asked: &str) -> bool {
    if asked.is_empty() {
        return false;
    }
    named.strip_prefix(asked).is_some_and(|rest| {
        let mut rest = rest.chars();
        match rest.next() {
            None => true,
            Some('-' | '@') => rest.next().is_some_and(|first| first.is_ascii_digit()),
            Some(_) => false,
        }
    })
}

impl Pricing {
    /// What `report` cost: by the provider's bill, else the user's prices, else the list, else unknown.
    // Human: a provider may answer under a dated name of the model that was asked ("big-20261001"); that is
    // still the asked model. Another model (a hand-over) is priced as itself, from the list.
    #[must_use]
    pub fn price(&self, report: &Report) -> Spent {
        let model = report.model.as_deref().unwrap_or(&self.model);
        // Human: a report that names no model is about the one that was asked.
        let asked = report.model.as_deref().is_none_or(|named| dated(named, &self.model));
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
        // An exponent past what a number can hold neither wraps nor panics: too large is no number, too small is nothing.
        assert_eq!(
            (scaled("1e9223372036854775807", 6), scaled("1e99999999999999999999", 6), scaled("0.5e9223372036854775807", 6)),
            (None, None, None)
        );
        assert_eq!(
            (scaled("1e-9223372036854775808", 6), scaled("1.5e-9223372036854775807", 6), scaled("7e-99999999999999999999", 6)),
            (Some(0), Some(0), Some(0))
        );
        assert_eq!(
            (scaled("2e+3", 0), scaled("1e", 6), scaled("1e--5", 6), scaled("1e+-5", 6), scaled("1e5x", 6)),
            (Some(2000), None, None, None, None)
        );
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
        assert_eq!(
            tiered.cost(Used { input: 150_000, cached: 50_001, output: 1, ..Used::default() }),
            150_000 * 16_000_000 + 50_001 * 400_000 + 60_000_000
        );
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

    #[test]
    fn each_apis_usage_is_brought_to_one_shape() {
        // Messages API: the start of an answer says what was read, its end what was written.
        let mut used = Used::default();
        messages(&json!({ "input_tokens": 12, "cache_creation_input_tokens": 300, "cache_read_input_tokens": 9000, "output_tokens": 1 }), &mut used);
        messages(&json!({ "output_tokens": 420 }), &mut used);
        messages(&json!({}), &mut used);
        assert_eq!(used, Used { input: 12, cached: 9000, written: 300, output: 420 });
        // Chat Completions counts cached tokens inside the prompt's; they are taken off.
        let report =
            completions(&json!({ "prompt_tokens": 1000, "completion_tokens": 50, "prompt_tokens_details": { "cached_tokens": 800 } })).unwrap();
        assert_eq!((report.used, report.billed), (Used { input: 200, cached: 800, written: 0, output: 50 }, None));
        // Deepseek names them differently; OpenRouter says what it charged.
        let deepseek = completions(&json!({ "prompt_tokens": 1000, "completion_tokens": 50, "prompt_cache_hit_tokens": 600 })).unwrap();
        assert_eq!(deepseek.used, Used { input: 400, cached: 600, written: 0, output: 50 });
        let router = completions(&json!({ "prompt_tokens": 10, "completion_tokens": 5, "cost": 0.000_012_5 })).unwrap();
        assert_eq!(router.billed, Some(12_500_000));
        assert_eq!((completions(&json!({})), completions(&json!(null))), (None, None));
        // Thinking counted only in the total is output too; a total that is the plain sum changes nothing,
        // and one smaller than the prompt takes nothing away.
        let output = |usage: Value| completions(&usage).map(|report| report.used.output);
        assert_eq!(output(json!({ "prompt_tokens": 100, "completion_tokens": 9, "total_tokens": 429 })), Some(329));
        assert_eq!(output(json!({ "prompt_tokens": 1000, "completion_tokens": 50, "total_tokens": 1050 })), Some(50));
        assert_eq!(output(json!({ "prompt_tokens": 1000, "completion_tokens": 50, "total_tokens": 7 })), Some(50));
        // Cached tokens named at the top level are taken off the prompt's as well.
        let flat = completions(&json!({ "prompt_tokens": 1000, "completion_tokens": 50, "cached_tokens": 300 })).unwrap();
        assert_eq!(flat.used, Used { input: 700, cached: 300, written: 0, output: 50 });
        // Ollama says both on its last line.
        assert_eq!(
            ollama(&json!({ "done": true, "prompt_eval_count": 17_000, "eval_count": 90 })).unwrap().used,
            Used { input: 17_000, output: 90, ..Used::default() }
        );
        assert_eq!(ollama(&json!({ "done": true })), None);
        assert_eq!(program(&json!({ "type": "result", "result": "Hello" })), None);
    }

    fn captured(text: &str) -> Report {
        program(&serde_json::from_str(text).unwrap()).unwrap()
    }

    #[test]
    fn a_resumed_claude_session_costs_this_message_not_the_session() {
        let first = captured(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"hi","total_cost_usd":0.0784642,"usage":{"input_tokens":2,"cache_creation_input_tokens":4227,"cache_read_input_tokens":53281,"output_tokens":4},"modelUsage":{"claude-fable-5-2":{"inputTokens":2,"outputTokens":4,"cacheReadInputTokens":53281,"cacheCreationInputTokens":4227,"costUSD":0.0784642,"costBasis":"list"}},"model_usage_since_last_result":{"claude-fable-5-2":{"inputTokens":2,"outputTokens":4,"cacheReadInputTokens":53281,"cacheCreationInputTokens":4227,"costUSD":0.0784642,"costBasis":"list"}}}"#,
        );
        assert_eq!(first.used, Used { input: 2, cached: 53_281, written: 4227, output: 4 });
        assert_eq!((first.billed, first.model.as_deref()), (Some(78_464_200_000), Some("claude-fable-5-2")));
        // Resumed: `total_cost_usd` and `modelUsage` are the session's (164_880_400_000); the message's is the other object.
        let second = captured(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"yo","total_cost_usd":0.16488039999999998,"usage":{"input_tokens":2,"cache_creation_input_tokens":4724,"cache_read_input_tokens":53281,"output_tokens":4},"modelUsage":{"claude-fable-5-2":{"inputTokens":4,"outputTokens":8,"cacheReadInputTokens":106562,"cacheCreationInputTokens":8951,"costUSD":0.16488039999999998}},"model_usage_since_last_result":{"claude-fable-5-2":{"inputTokens":2,"outputTokens":4,"cacheReadInputTokens":53281,"cacheCreationInputTokens":4724,"costUSD":0.0864162,"costBasis":"list"}}}"#,
        );
        assert_eq!(second.used, Used { input: 2, cached: 53_281, written: 4724, output: 4 });
        assert_eq!((second.billed, second.model.as_deref()), (Some(86_416_200_000), Some("claude-fable-5-2")));
    }

    #[test]
    fn a_resumed_run_that_used_nothing_is_not_charged_the_session() {
        let nothing = r#"{"type":"result","is_error":true,"result":"limit","total_cost_usd":0.16,"usage":{"input_tokens":0,"output_tokens":0},"model_usage_since_last_result":{}}"#;
        assert_eq!(program(&serde_json::from_str(nothing).unwrap()), None);
        let some = r#"{"type":"result","total_cost_usd":0.16,"usage":{"input_tokens":3,"output_tokens":5},"model_usage_since_last_result":{}}"#;
        let report = captured(some);
        assert_eq!((report.used, report.billed), (Used { input: 3, output: 5, ..Used::default() }, None));
    }

    #[test]
    fn grok_reports_its_message_in_total_cost() {
        let first = captured(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"hi","total_cost_usd":0.01320424,"usage":{"input_tokens":18918,"output_tokens":28,"cache_read_input_tokens":1664,"cache_creation_input_tokens":0},"modelUsage":{"grok-4.7-build":{"inputTokens":18918,"outputTokens":28,"cacheReadInputTokens":1664,"cacheCreationInputTokens":0,"costUSD":0.01320424}}}"#,
        );
        assert_eq!(first.used, Used { input: 18_918, cached: 1664, written: 0, output: 28 });
        assert_eq!((first.billed, first.model.as_deref()), (Some(13_204_240_000), Some("grok-4.7-build")));
        let again = captured(
            r#"{"type":"result","total_cost_usd":0.00366928,"usage":{"input_tokens":225,"output_tokens":17,"cache_read_input_tokens":20480,"cache_creation_input_tokens":0},"modelUsage":{"grok-4.7-build":{"inputTokens":225,"outputTokens":17,"cacheReadInputTokens":20480,"cacheCreationInputTokens":0,"costUSD":0.00366928}}}"#,
        );
        assert_eq!(again.used, Used { input: 225, cached: 20_480, written: 0, output: 17 });
        assert_eq!((again.billed, again.model.as_deref()), (Some(3_669_280_000), Some("grok-4.7-build")));
    }

    #[test]
    fn a_program_that_names_a_model_keeps_its_bill_when_no_model_was_asked() {
        let report = Report {
            used: Used { input: 2, output: 4, ..Used::default() },
            model: Some("claude-fable-5-2".to_owned()),
            billed: Some(86_416_200_000),
        };
        let pricing = Pricing { model: String::new(), terms: Terms::Plan, ..Pricing::default() };
        let spent = pricing.price(&report);
        assert_eq!((spent.basis, spent.cost, spent.model.as_str()), (Basis::Worth, Some(86_416_200_000), "claude-fable-5-2"));
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
        let report = |model: Option<&str>, billed| Report {
            used: Used { input: 1000, output: 100, ..Used::default() },
            model: model.map(str::to_owned),
            billed,
        };
        // The list, by the model that was asked.
        let spent = pricing.price(&report(None, None));
        assert_eq!((spent.cost, spent.basis, spent.model.as_str()), (Some(1000 * 1_000_000 + 100 * 2_000_000), Basis::Listed, "big"));
        // A dated name of the asked model is the asked model; another model is priced as itself.
        assert_eq!(pricing.price(&report(Some("big-20261001"), None)).basis, Basis::Listed);
        assert_eq!(pricing.price(&report(Some("small"), None)).cost, Some(1000 * 500_000 + 100 * 1_000_000));
        assert_eq!(pricing.price(&report(Some("other"), None)).basis, Basis::Unknown);
        assert_eq!(pricing.price(&report(Some("big@20261001"), None)).basis, Basis::Listed);
        // A name that only begins like the asked one is another model: not in the list, so without a price.
        assert_eq!(pricing.price(&report(Some("big-mini"), None)).basis, Basis::Unknown);
        assert_eq!(
            (pricing.price(&report(Some("bigger"), None)).basis, pricing.price(&report(Some("big-"), None)).basis),
            (Basis::Unknown, Basis::Unknown)
        );
        // The user's own prices win over the list, and a bill over both.
        let own = Pricing { own: Rates::typed("10", "10"), ..pricing.clone() };
        assert_eq!(own.price(&report(None, None)).basis, Basis::Own);
        // They are the asked model's: its dated name has them, another model that begins the same has not.
        assert_eq!(
            (own.price(&report(Some("big-20261001"), None)).basis, own.price(&report(Some("big-mini"), None)).basis),
            (Basis::Own, Basis::Unknown)
        );
        // A run that asked for no model by name: whatever answers by name is not known to be it.
        let unnamed = Pricing { model: String::new(), ..own.clone() };
        assert_eq!(
            (unnamed.price(&report(Some("small"), None)).basis, unnamed.price(&report(Some("x"), None)).basis),
            (Basis::Listed, Basis::Unknown)
        );
        assert_eq!((own.price(&report(None, Some(77))).cost, own.price(&report(None, Some(77))).basis), (Some(77), Basis::Billed));
        // A subscription's cost is a worth; a model on this computer has none, whatever is reported.
        let plan = Pricing { terms: Terms::Plan, ..pricing.clone() };
        assert_eq!((plan.price(&report(None, Some(77))).basis, plan.price(&report(Some("other"), None)).basis), (Basis::Worth, Basis::Unknown));
        let local = Pricing { terms: Terms::Local, ..pricing };
        assert_eq!((local.price(&report(None, Some(77))).cost, local.price(&report(None, None)).basis), (None, Basis::Local));
    }
}
