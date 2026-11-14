//! Variables (`$name`) and themes: which value a property bound to a variable has for a node.

use serde_json::{Map, Value};

use crate::color::Rgba;
use crate::model::Document;
use crate::value::{Flag, Num, narrow};

/// How deep `$a → $b → …` chains are followed before giving up (a cycle guard).
const MAX_CHAIN: usize = 8;

/// The theme axis values in force for a node: the document's defaults, overridden by each ancestor's `theme`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThemeContext {
    axes: Vec<(String, String)>,
}

impl ThemeContext {
    /// The defaults: the first value of every theme axis the document declares.
    #[must_use]
    pub fn defaults(doc: &Document) -> ThemeContext {
        let mut context = ThemeContext::default();
        for (axis, values) in doc.themes.iter().flatten() {
            if let Some(first) = values.as_array().and_then(|v| v.first()).and_then(Value::as_str) {
                context.set(axis, first);
            }
        }
        context
    }

    /// This context with a node's own `theme` laid over it.
    #[must_use]
    pub fn with(&self, theme: Option<&Map<String, Value>>) -> ThemeContext {
        let mut context = self.clone();
        for (axis, value) in theme.into_iter().flatten() {
            if let Some(value) = value.as_str() {
                context.set(axis, value);
            }
        }
        context
    }

    /// Sets one axis.
    pub fn set(&mut self, axis: &str, value: &str) {
        match self.axes.iter_mut().find(|(a, _)| a == axis) {
            Some(entry) => value.clone_into(&mut entry.1),
            None => self.axes.push((axis.to_owned(), value.to_owned())),
        }
    }

    /// The value of one axis.
    #[must_use]
    pub fn get(&self, axis: &str) -> Option<&str> {
        self.axes.iter().find(|(a, _)| a == axis).map(|(_, v)| v.as_str())
    }

    /// Whether every axis a variable entry names has that value here.
    fn matches(&self, theme: &Map<String, Value>) -> bool {
        theme.iter().all(|(axis, value)| value.as_str().is_some_and(|v| self.get(axis) == Some(v)))
    }
}

/// Reads property values through the document's variables.
#[derive(Clone, Copy, Debug)]
pub struct Vars<'a> {
    variables: Option<&'a Map<String, Value>>,
}

impl<'a> Vars<'a> {
    /// The variables of `doc`.
    #[must_use]
    pub fn new(doc: &'a Document) -> Vars<'a> {
        Vars { variables: doc.variables.as_ref() }
    }

    /// The raw value of variable `name` for `context`, following `$other` chains.
    // Human: A variable's value is either one value, or a list of `{value, theme}` entries. The last entry
    // whose theme matches wins, so the unthemed default (listed first) gives way to `Mode: Dark` when dark.
    // Agent: RETURNS None for unknown names, cycles longer than MAX_CHAIN, or no matching entry.
    #[must_use]
    pub fn lookup(&self, name: &str, context: &ThemeContext) -> Option<&'a Value> {
        let mut name = name;
        for _ in 0..MAX_CHAIN {
            let variable = self.variables?.get(name)?;
            let value = variable.get("value")?;
            let chosen = match value.as_array() {
                Some(entries) if entries.iter().all(|e| e.get("value").is_some()) => {
                    entries.iter().rfind(|e| e.get("theme").and_then(Value::as_object).is_none_or(|t| context.matches(t)))?.get("value")?
                }
                _ => value,
            };
            match chosen.as_str().and_then(|s| s.strip_prefix('$')) {
                Some(next) => name = next,
                None => return Some(chosen),
            }
        }
        None
    }

    /// A number property's value.
    #[must_use]
    pub fn number(&self, num: &Num, context: &ThemeContext) -> Option<f32> {
        match num.variable() {
            Some(name) => {
                let value = self.lookup(name, context)?;
                value.as_f64().map(narrow).or_else(|| value.as_str().and_then(|s| s.trim().parse::<f64>().ok()).map(narrow))
            }
            None => num.literal(),
        }
    }

    /// A colour property's value (`#hex` or `$name`).
    #[must_use]
    pub fn color(&self, text: &str, context: &ThemeContext) -> Option<Rgba> {
        match text.strip_prefix('$') {
            Some(name) => self.lookup(name, context).and_then(Value::as_str).and_then(Rgba::parse),
            None => Rgba::parse(text),
        }
    }

    /// A boolean property's value.
    #[must_use]
    pub fn flag(&self, flag: &Flag, context: &ThemeContext) -> Option<bool> {
        match (flag, flag.variable()) {
            (Flag::Value(value), _) => Some(*value),
            (_, Some(name)) => self.lookup(name, context).and_then(Value::as_bool),
            (Flag::Variable(_), None) => None,
        }
    }

    /// A text property's value (`$name` or literal text).
    #[must_use]
    pub fn text(&self, text: &str, context: &ThemeContext) -> String {
        match text.strip_prefix('$').and_then(|name| self.lookup(name, context)) {
            Some(Value::String(value)) => value.clone(),
            Some(Value::Number(value)) => value.to_string(),
            _ => text.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> Document {
        serde_json::from_str(
            r##"{"version":"2.22","children":[],"themes":{"Mode":["Light","Dark"]},
            "variables":{"--bg":{"type":"color","value":[{"value":"#ffffff"},{"value":"#000000","theme":{"Mode":"Dark"}}]},
            "gap":{"type":"number","value":"$base"},"base":{"type":"number","value":8},
            "loop":{"type":"number","value":"$loop"}}}"##,
        )
        .unwrap()
    }

    #[test]
    fn themed_values_follow_the_context() {
        let doc = doc();
        let vars = Vars::new(&doc);
        let light = ThemeContext::defaults(&doc);
        assert_eq!(light.get("Mode"), Some("Light"));
        assert_eq!(vars.color("$--bg", &light), Some(Rgba::WHITE));
        let mut dark_theme = Map::new();
        dark_theme.insert("Mode".into(), Value::from("Dark"));
        let dark = light.with(Some(&dark_theme));
        assert_eq!(vars.color("$--bg", &dark), Some(Rgba::BLACK));
    }

    #[test]
    fn chains_resolve_and_cycles_stop() {
        let doc = doc();
        let vars = Vars::new(&doc);
        let context = ThemeContext::defaults(&doc);
        assert_eq!(vars.number(&Num::Text("$gap".into()), &context), Some(8.0));
        assert_eq!(vars.number(&Num::Text("$loop".into()), &context), None);
        assert_eq!(vars.number(&Num::Text("$missing".into()), &context), None);
    }
}
