//! The effects a layer can have, as the `.pen` file writes them in its `effect` key: one object, or a list.
//!
//! Effects stay JSON so that keys this version doesn't know (blend modes, shader uniforms) survive a save.

use serde_json::{Map, Value, json};

/// The kinds of effect, in the order the add menu lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    /// A shadow behind the layer.
    DropShadow,
    /// A shadow inside the layer's shape.
    InnerShadow,
    /// The layer itself, blurred.
    LayerBlur,
    /// What is behind the layer, blurred.
    BackgroundBlur,
    /// A fragment shader run over the drawn layer.
    Shader,
}

impl EffectKind {
    /// Every kind, in menu order.
    pub const ALL: [EffectKind; 5] =
        [EffectKind::DropShadow, EffectKind::InnerShadow, EffectKind::LayerBlur, EffectKind::BackgroundBlur, EffectKind::Shader];

    /// The kind of a stored effect.
    #[must_use]
    pub fn of(effect: &Value) -> Option<EffectKind> {
        match (effect.get("type").and_then(Value::as_str)?, effect.get("shadowType").and_then(Value::as_str)) {
            ("shadow", Some("inner")) => Some(EffectKind::InnerShadow),
            ("shadow", _) => Some(EffectKind::DropShadow),
            ("blur", _) => Some(EffectKind::LayerBlur),
            ("background_blur", _) => Some(EffectKind::BackgroundBlur),
            ("shader", _) => Some(EffectKind::Shader),
            _ => None,
        }
    }

    /// Its name in the interface.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            EffectKind::DropShadow => "Drop Shadow",
            EffectKind::InnerShadow => "Inner Shadow",
            EffectKind::LayerBlur => "Layer Blur",
            EffectKind::BackgroundBlur => "Background Blur",
            EffectKind::Shader => "Shader",
        }
    }

    /// The Lucide icon that stands for it.
    #[must_use]
    pub fn icon(self) -> &'static str {
        match self {
            EffectKind::DropShadow => "copy",
            EffectKind::InnerShadow => "square-round-corner",
            EffectKind::LayerBlur => "circle-dot-dashed",
            EffectKind::BackgroundBlur => "square-dashed",
            EffectKind::Shader => "square-activity",
        }
    }

    /// Whether a layer can have only one of this kind.
    // Human: Layer Blur and Background Blur are greyed out in the add menu once the layer has one; shadows
    // and shaders can be stacked.
    #[must_use]
    pub fn single(self) -> bool {
        matches!(self, EffectKind::LayerBlur | EffectKind::BackgroundBlur)
    }

    /// A new effect of this kind, as it is written to the file.
    // Human: shadows are black at 25 %, offset 0 / 4, blur 4; blurs have radius 4; a shader starts
    // with no file. The file keeps `enabled` only when it is off.
    #[must_use]
    pub fn fresh(self) -> Value {
        match self {
            EffectKind::DropShadow => {
                json!({ "type": "shadow", "shadowType": "outer", "color": "#00000040", "offset": { "x": 0, "y": 4 }, "blur": 4 })
            }
            EffectKind::InnerShadow => {
                json!({ "type": "shadow", "shadowType": "inner", "color": "#00000040", "offset": { "x": 0, "y": 4 }, "blur": 4 })
            }
            EffectKind::LayerBlur => json!({ "type": "blur", "radius": 4 }),
            EffectKind::BackgroundBlur => json!({ "type": "background_blur", "radius": 4 }),
            EffectKind::Shader => json!({ "type": "shader", "url": "" }),
        }
    }
}

/// A layer's effects as a list, bottom first.
#[must_use]
pub fn list(effect: Option<&Value>) -> Vec<Value> {
    match effect {
        Some(Value::Array(list)) => list.clone(),
        Some(Value::Null) | None => Vec::new(),
        Some(one) => vec![one.clone()],
    }
}

/// A list of effects as the file stores it: nothing, the one effect, or a list.
#[must_use]
pub fn stored(mut list: Vec<Value>) -> Option<Value> {
    match list.len() {
        0 => None,
        1 => Some(list.remove(0)),
        _ => Some(Value::Array(list)),
    }
}

/// Whether an effect is switched off by a plain `false` (a variable is left to the canvas to look up).
#[must_use]
pub fn is_off(effect: &Value) -> bool {
    effect.get("enabled") == Some(&Value::Bool(false))
}

/// The effect with `change` applied to its keys; anything but an object comes back untouched.
#[must_use]
pub fn with(effect: &Value, change: impl FnOnce(&mut Map<String, Value>)) -> Value {
    let mut effect = effect.clone();
    if let Some(keys) = effect.as_object_mut() {
        change(keys);
    }
    effect
}

/// A number for the file: whole numbers without a fraction.
#[must_use]
#[expect(clippy::cast_possible_truncation, reason = "only whole numbers within the i64 range take this branch")]
pub fn number(v: f32) -> Value {
    // Human: two decimals are as fine as the number fields go; rounding also drops f32 noise such as 0.1 → 0.100000001.
    let rounded = (f64::from(v) * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 && rounded.abs() < 1e15 { json!(rounded as i64) } else { json!(rounded) }
}

/// The name a shader effect shows: its file's name, or "Shader" before it has one.
#[must_use]
pub fn shader_name(effect: &Value) -> String {
    let url = effect.get("url").and_then(Value::as_str).unwrap_or("").trim();
    let name = url.rsplit(['/', '\\']).next().unwrap_or("");
    if name.is_empty() { EffectKind::Shader.label().to_owned() } else { name.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_effect_is_of_its_own_kind_and_is_switched_on() {
        for kind in EffectKind::ALL {
            let effect = kind.fresh();
            assert_eq!(EffectKind::of(&effect), Some(kind));
            assert!(!is_off(&effect));
        }
        // A shadow without a `shadowType` is a drop shadow (older files).
        assert_eq!(EffectKind::of(&json!({ "type": "shadow" })), Some(EffectKind::DropShadow));
        assert_eq!(EffectKind::of(&json!({ "type": "glow" })), None);
    }

    #[test]
    fn effects_are_stored_as_none_one_or_a_list() {
        let blur = EffectKind::LayerBlur.fresh();
        let shadow = EffectKind::DropShadow.fresh();
        assert!(stored(Vec::new()).is_none());
        assert_eq!(stored(vec![blur.clone()]), Some(blur.clone()));
        let both = stored(vec![blur.clone(), shadow.clone()]).unwrap();
        assert_eq!(list(Some(&both)), vec![blur.clone(), shadow]);
        assert_eq!(list(Some(&blur)), vec![blur]);
        assert_eq!(list(None), [] as [Value; 0]);
    }

    #[test]
    fn changing_an_effect_keeps_the_keys_it_does_not_touch() {
        let effect = json!({ "type": "shadow", "blendMode": "multiply", "blur": 4 });
        let changed = with(&effect, |keys| {
            keys.insert("blur".to_owned(), number(6.5));
            keys.insert("enabled".to_owned(), Value::Bool(false));
        });
        assert_eq!(changed, json!({ "type": "shadow", "blendMode": "multiply", "blur": 6.5, "enabled": false }));
        assert!(is_off(&changed));
    }

    #[test]
    fn numbers_are_written_whole_when_they_are_whole() {
        assert_eq!(number(4.0).to_string(), "4");
        assert_eq!(number(-2.0).to_string(), "-2");
        assert_eq!(number(0.1).to_string(), "0.1");
        assert_eq!(number(2.345).to_string(), "2.35");
    }

    #[test]
    fn a_shader_is_named_after_its_file() {
        assert_eq!(shader_name(&json!({ "type": "shader", "url": "./fx/glow.glsl" })), "glow.glsl");
        assert_eq!(shader_name(&json!({ "type": "shader", "url": "" })), "Shader");
    }
}
