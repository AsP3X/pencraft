//! Pencraft's own mark in a document: the `pencraft` key.
//!
//! A `.pen` file carries the schema version of the program the format comes from, which says nothing about
//! what Pencraft added to it. The stamp says that: `format`, the number of Pencraft's own format, and
//! `features`, the names of what the document uses that only Pencraft draws. A reader that doesn't know the key
//! keeps or drops it; a Pencraft that reads a higher number or a name it doesn't know can say so.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::model::Document;

/// The number of Pencraft's own format this version writes. It goes up when a document can hold something an
/// older Pencraft would draw wrongly or not at all.
pub const FORMAT: u64 = 1;

/// Something a document can hold that only Pencraft draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Feature {
    /// Component states (`states` on a component, `state` on an instance).
    States,
    /// Connector layers.
    Connector,
    /// Materials (`material`: frosted, liquid glass).
    Material,
    /// Masks (`mask`).
    Mask,
    /// Pattern paints.
    Pattern,
    /// Text set on a circle (`onCircle`).
    TextOnCircle,
    /// Text set along a line (`onPath`).
    TextOnPath,
}

impl Feature {
    /// Every feature, in the order their names sort.
    pub const ALL: [Feature; 7] =
        [Feature::States, Feature::Connector, Feature::Material, Feature::Mask, Feature::Pattern, Feature::TextOnCircle, Feature::TextOnPath];

    /// The name the stamp lists it by.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Feature::States => "componentStates",
            Feature::Connector => "connector",
            Feature::Material => "material",
            Feature::Mask => "mask",
            Feature::Pattern => "patternPaint",
            Feature::TextOnCircle => "textOnCircle",
            Feature::TextOnPath => "textOnPath",
        }
    }

    /// What a sentence calls it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Feature::States => "component states",
            Feature::Connector => "connectors",
            Feature::Material => "materials",
            Feature::Mask => "masks",
            Feature::Pattern => "pattern fills",
            Feature::TextOnCircle => "text on a circle",
            Feature::TextOnPath => "text along a line",
        }
    }

    fn named(name: &str) -> Option<Feature> {
        Feature::ALL.into_iter().find(|feature| feature.name() == name)
    }
}

/// What of a document's stamp this version doesn't know.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Newer {
    /// The format number, when it is higher than [`FORMAT`].
    pub format: Option<u64>,
    /// The listed features this version has no name for.
    pub features: Vec<String>,
}

/// The features `doc` uses, found in its layers and in what instances override.
// Human: read from the document as JSON, so a mask or a material an instance overrides counts like one on a
// layer, and so does one inside a kind of layer this version doesn't know.
#[must_use]
pub fn features(doc: &Document) -> Vec<Feature> {
    let mut found = BTreeSet::new();
    if let Ok(layers) = serde_json::to_value(&doc.children) {
        look(&layers, &mut found);
    }
    found.into_iter().collect()
}

fn look(value: &Value, found: &mut BTreeSet<Feature>) {
    match value {
        Value::Object(object) => {
            let kind = object.get("type").and_then(Value::as_str);
            let set = |key: &str| object.get(key).is_some_and(|value| !matches!(value, Value::Null | Value::Bool(false)));
            let uses = [
                (Feature::States, object.get("states").is_some_and(Value::is_object) || (kind == Some("ref") && set("state"))),
                (Feature::Connector, kind == Some("connector")),
                (Feature::Material, set("material")),
                (Feature::Mask, set("mask")),
                (Feature::Pattern, kind == Some("pattern")),
                (Feature::TextOnCircle, set("onCircle")),
                (Feature::TextOnPath, set("onPath")),
            ];
            found.extend(uses.into_iter().filter(|(_, used)| *used).map(|(feature, _)| feature));
            object.values().for_each(|inner| look(inner, found));
        }
        Value::Array(list) => list.iter().for_each(|inner| look(inner, found)),
        _ => {}
    }
}

/// The names the stamp of `doc` lists.
fn listed(doc: &Document) -> Vec<&str> {
    let list = doc.pencraft.as_ref().and_then(|stamp| stamp.get("features")).and_then(Value::as_array);
    list.map(|list| list.iter().filter_map(Value::as_str).collect()).unwrap_or_default()
}

/// The format number the stamp of `doc` says, if it has one.
fn format_of(doc: &Document) -> Option<u64> {
    doc.pencraft.as_ref().and_then(|stamp| stamp.get("format")).and_then(Value::as_u64)
}

/// What the stamp of `doc` says that this version doesn't know: a higher format number, or features without a
/// name here. `None` for a document this version knows all of, and for one without a stamp.
#[must_use]
pub fn newer(doc: &Document) -> Option<Newer> {
    let format = format_of(doc).filter(|format| *format > FORMAT);
    let features: Vec<String> = listed(doc).into_iter().filter(|name| Feature::named(name).is_none()).map(str::to_owned).collect();
    (format.is_some() || !features.is_empty()).then_some(Newer { format, features })
}

/// Stamps `doc` as this version writes it: the format number (a higher one it came with stays) and the
/// features it uses, by name in order; the list is left out when there are none.
// Human: a feature the stamp listed that this version has no name for stays listed: what it stands for is in
// keys this version keeps without reading, so it is still in the document. Keys of the stamp this version
// doesn't know are kept after its own.
pub fn apply(doc: &mut Document) {
    let mut names: BTreeSet<String> = features(doc).into_iter().map(|feature| feature.name().to_owned()).collect();
    names.extend(listed(doc).into_iter().filter(|name| Feature::named(name).is_none()).map(str::to_owned));
    let mut stamp = Map::new();
    stamp.insert("format".to_owned(), Value::from(format_of(doc).map_or(FORMAT, |format| format.max(FORMAT))));
    if !names.is_empty() {
        stamp.insert("features".to_owned(), Value::from(names.into_iter().collect::<Vec<_>>()));
    }
    if let Some(Value::Object(old)) = doc.pencraft.take() {
        stamp.extend(old.into_iter().filter(|(key, _)| key != "format" && key != "features"));
    }
    doc.pencraft = Some(Value::Object(stamp));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read, write};

    const USES_ALL: &str = r##"{"version":"2.22","children":[
        {"type":"frame","id":"card","reusable":true,"states":{"hover":{"fill":"#fff"}},"material":"frosted","children":[
            {"type":"rectangle","id":"cut","mask":true,"fill":{"type":"pattern","shape":"dots"}},
            {"type":"text","id":"round","onCircle":"top"},
            {"type":"text","id":"along","onPath":"M0 0L10 0"}]},
        {"type":"ref","id":"one","ref":"card","state":"hover"},
        {"type":"connector","id":"wire","from":"card","to":"one"}]}"##;

    #[test]
    fn every_feature_is_found_and_a_plain_document_has_none() {
        assert_eq!(features(&read(USES_ALL).unwrap()), Feature::ALL);
        let plain = r##"{"version":"2.22","children":[{"type":"rectangle","id":"a","mask":false,"fill":"#fff"},{"type":"text","id":"t","content":"mask material connector"}]}"##;
        assert_eq!(features(&read(plain).unwrap()), []);
    }

    #[test]
    fn a_feature_an_instance_overrides_counts() {
        let text = r#"{"version":"2.22","children":[{"type":"ref","id":"i","ref":"c","descendants":{"box":{"material":{"type":"liquid_glass"}}}}]}"#;
        assert_eq!(features(&read(text).unwrap()), [Feature::Material]);
    }

    #[test]
    fn the_stamp_comes_after_the_version_and_a_second_stamp_changes_nothing() {
        let mut doc = read(r#"{"version":"2.22","children":[{"type":"connector","id":"w"},{"type":"rectangle","id":"m","mask":true}]}"#).unwrap();
        apply(&mut doc);
        let written = write(&doc).unwrap();
        assert!(written.starts_with("{\n  \"version\": \"2.22\",\n  \"pencraft\": {\n    \"format\": 1,\n    \"features\": [\n      \"connector\",\n      \"mask\"\n    ]\n  },\n  \"children\""));
        let mut again = read(&written).unwrap();
        assert_eq!(newer(&again), None);
        apply(&mut again);
        assert_eq!(write(&again).unwrap(), written);
        let mut plain = read(r#"{"version":"2.22","children":[]}"#).unwrap();
        apply(&mut plain);
        assert_eq!(write(&plain).unwrap(), "{\n  \"version\": \"2.22\",\n  \"pencraft\": {\n    \"format\": 1\n  },\n  \"children\": []\n}");
    }

    #[test]
    fn what_a_newer_pencraft_wrote_is_told_and_kept() {
        let text = r#"{"version":"2.30","pencraft":{"format":3,"features":["hologram","mask"],"by":"9.0"},"children":[]}"#;
        let mut doc = read(text).unwrap();
        assert_eq!(newer(&doc), Some(Newer { format: Some(3), features: vec!["hologram".to_owned()] }));
        apply(&mut doc);
        // The mask is gone from the layers, so it leaves the list; the unknown name, the number and the key stay.
        assert_eq!(doc.pencraft, Some(serde_json::json!({ "format": 3, "features": ["hologram"], "by": "9.0" })));
        // A stamp that isn't an object is replaced, not a reason to refuse the file.
        let mut odd = read(r#"{"version":"2.22","pencraft":"yes","children":[]}"#).unwrap();
        assert_eq!(newer(&odd), None);
        apply(&mut odd);
        assert_eq!(doc.pencraft.as_ref().map(Value::is_object), Some(true));
        assert_eq!(odd.pencraft, Some(serde_json::json!({ "format": 1 })));
    }
}
