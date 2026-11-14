//! Property values as the `.pen` file stores them: a literal or a `$variable` reference.
//!
//! The raw form is kept exactly as read, so a file written back keeps `"fit_content(100)"`, `"$gap"` or `4`
//! as they were; the meaning is read through methods.

use serde::{Deserialize, Serialize};
use serde_json::Number;

/// A number, or text standing for one: `$name` binds the property to a variable, and sizes also accept
/// `fit_content`, `fill_container` and their `(fallback)` forms.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Num {
    /// A literal number, kept as written (`4` stays `4`, `4.5` stays `4.5`).
    Value(Number),
    /// `$name`, a sizing keyword, or a number written as text (`"500"` for a font weight).
    Text(String),
}

impl Num {
    /// A literal number.
    #[must_use]
    pub fn new(value: f32) -> Self {
        Num::Value(number(value))
    }

    /// The variable this value is bound to, without its `$`.
    #[must_use]
    pub fn variable(&self) -> Option<&str> {
        match self {
            Num::Text(text) => text.strip_prefix('$'),
            Num::Value(_) => None,
        }
    }

    /// The literal number, also when it was written as text. `None` for variables and keywords.
    #[must_use]
    pub fn literal(&self) -> Option<f32> {
        match self {
            Num::Value(value) => value.as_f64().map(narrow),
            Num::Text(text) => text.trim().parse::<f64>().ok().map(narrow),
        }
    }

    /// How a `width` or `height` value sizes its node.
    #[must_use]
    pub fn sizing(&self) -> Sizing {
        let Num::Text(text) = self else {
            return Sizing::Fixed(self.clone());
        };
        let (keyword, fallback) = split_fallback(text);
        match keyword {
            "fit_content" => Sizing::FitContent(fallback),
            "fill_container" => Sizing::FillContainer(fallback),
            _ => Sizing::Fixed(self.clone()),
        }
    }
}

/// `fit_content(100)` → (`fit_content`, `Some(100)`); `fit_content` → (`fit_content`, `None`).
fn split_fallback(text: &str) -> (&str, Option<f32>) {
    let text = text.trim();
    match text.split_once('(') {
        Some((keyword, rest)) => (keyword.trim(), rest.trim_end_matches(')').trim().parse::<f64>().ok().map(narrow)),
        None => (text, None),
    }
}

/// How one side of a node is sized (the schema's `SizingBehavior`).
#[derive(Clone, Debug, PartialEq)]
pub enum Sizing {
    /// A number or a variable holding one.
    Fixed(Num),
    /// The combined size of the children; the fallback applies without children.
    FitContent(Option<f32>),
    /// The parent's size; the fallback applies outside a layout.
    FillContainer(Option<f32>),
}

/// A boolean or a `$variable` reference.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Flag {
    /// A literal boolean.
    Value(bool),
    /// `$name`.
    Variable(String),
}

impl Flag {
    /// The variable this flag is bound to, without its `$`.
    #[must_use]
    pub fn variable(&self) -> Option<&str> {
        match self {
            Flag::Variable(text) => text.strip_prefix('$'),
            Flag::Value(_) => None,
        }
    }
}

/// A JSON number for an editor value: whole numbers without a fraction, others rounded to 1/100.
// Human: Values the editor writes (drags, typed sizes) are rounded to hundredths so files don't fill up with
// float noise like 267.8010819142764; values read from files are never rounded.
// Agent: NaN or infinity become 0 (JSON can't hold them).
#[must_use]
pub fn number(value: f32) -> Number {
    let rounded = (f64::from(value) * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 && rounded.abs() < 1e15 {
        return Number::from(whole(rounded));
    }
    Number::from_f64(rounded).unwrap_or_else(|| Number::from(0))
}

/// A whole f64 below 1e15 as i64.
#[expect(clippy::cast_possible_truncation, reason = "callers pass whole values below 1e15, exact in i64")]
fn whole(value: f64) -> i64 {
    value as i64
}

/// Document coordinates are stored as JSON doubles but laid out and drawn in `f32`.
// Human: Canvas positions stay far below f32's exact-integer range (16.7 million), so the narrowing loses
// nothing a designer could see.
// Agent: the ONE f64 → f32 conversion in the crate; NaN stays NaN and is rejected by callers that need finite values.
#[must_use]
#[expect(clippy::cast_possible_truncation, reason = "canvas coordinates fit f32; see the comment above")]
pub fn narrow(value: f64) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizing_keywords_and_fallbacks() {
        assert_eq!(Num::Text("fit_content(100)".into()).sizing(), Sizing::FitContent(Some(100.0)));
        assert_eq!(Num::Text("fill_container".into()).sizing(), Sizing::FillContainer(None));
        assert_eq!(Num::new(4.0).sizing(), Sizing::Fixed(Num::new(4.0)));
        assert_eq!(serde_json::to_string(&Num::new(4.0)).unwrap(), "4");
        assert_eq!(serde_json::to_string(&Num::new(2.505)).unwrap(), "2.51");
        assert_eq!(Num::Text("$w".into()).variable(), Some("w"));
        assert_eq!(Num::Text("500".into()).literal(), Some(500.0));
    }
}
