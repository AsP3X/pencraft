//! Materials: what a layer is made of, as the `.pen` file writes it in its `material` key.
//!
//! A material is the layer's background: what lies behind the layer shows through it, blurred (`frosted`) or
//! bent at the rim like a thick piece of glass (`liquid_glass`). A file names a material and a few choices;
//! the numbers the renderer draws with are worked out here, in [`Material::resolve`], and nowhere else.
//!
//! This is an original glass material. It follows the published behaviour of such materials (see
//! `docs/liquid-glass/TASK.md`), not anyone's implementation, and makes no claim to match a system material.

use pen_doc::color::Rgba;
use serde_json::{Map, Value, json};

/// The `type` of a frosted material.
pub const FROSTED: &str = "frosted";
/// The `type` of a liquid glass material.
pub const LIQUID_GLASS: &str = "liquid_glass";
/// The variable that sets the light's direction for every glass layer of a document.
pub const LIGHT_VARIABLE: &str = "glass-light-angle";
/// Where the light comes from unless the document says otherwise: degrees clockwise from straight above, so
/// this is the upper left.
const LIGHT_ANGLE: f32 = -45.0;
/// A layer up to this short side is a control (a button, a knob).
const CONTROL_SIDE: f32 = 52.0;
/// A layer up to this short side is a bar; a larger one is a panel.
const BAR_SIDE: f32 = 110.0;
/// How strongly a `tint` colours liquid glass, at least.
const GLASS_TINT: f32 = 0.9;
/// The near-black a dark material is tinted with.
const DARK: Rgba = Rgba { r: 0x16, g: 0x16, b: 0x18, a: 0xff };

/// How thick a frosted material is: thicker hides more of what is behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thickness {
    /// Keeps the most of what is behind it.
    UltraThin,
    /// A light veil.
    Thin,
    /// The default.
    Regular,
    /// The most opaque; for fine text.
    Thick,
}

/// Which liquid glass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    /// Blurs and evens out what is behind it; legible over anything.
    Regular,
    /// Nearly clear; only over media, with bold content on it.
    Clear,
}

/// The material itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A blur with a tint: structure inside the content.
    Frosted(Thickness),
    /// Glass that bends what is behind it at its rim: controls and navigation above the content.
    Glass(Variant),
}

/// Whether the material is the light or the dark one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    /// Light over light content, dark over dark content: decided from what is behind the layer.
    Auto,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

/// How thick the glass behaves; larger layers are thicker glass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    /// A button, a knob.
    Control,
    /// A tab bar, a toolbar.
    Bar,
    /// A sheet, a sidebar, a menu.
    Panel,
}

/// One number of a material that a file may set in `advanced`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Knob {
    /// How far the blur of what is behind reaches, as a `background_blur`'s `radius`.
    Blur,
    /// How much stronger colours are behind the material; 1 leaves them.
    Saturation,
    /// How much lighter (or, below 0, darker) what is behind becomes.
    Brightness,
    /// How much of the tint colour lies over what is behind, 0–1.
    TintOpacity,
    /// How strongly the rim bends what is behind it; 1 is glass, 0 a flat pane.
    Refraction,
    /// The width of the curved rim.
    Bevel,
    /// How thick the glass is; thicker glass bends further.
    Thickness,
    /// How far colours split at the rim, 0–1.
    Dispersion,
    /// How bright the light on the rim is, 0–1.
    Highlight,
    /// Where the light comes from, in degrees clockwise from straight above.
    LightAngle,
    /// How dark the shadow under the material is, 0–1.
    Shadow,
}

impl Knob {
    /// Every knob, in the order the inspector lists them.
    pub const ALL: [Knob; 11] = [
        Knob::Blur,
        Knob::Saturation,
        Knob::Brightness,
        Knob::TintOpacity,
        Knob::Refraction,
        Knob::Bevel,
        Knob::Thickness,
        Knob::Dispersion,
        Knob::Highlight,
        Knob::LightAngle,
        Knob::Shadow,
    ];

    /// Its key in `advanced`.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Knob::Blur => "blur",
            Knob::Saturation => "saturation",
            Knob::Brightness => "brightness",
            Knob::TintOpacity => "tintOpacity",
            Knob::Refraction => "refraction",
            Knob::Bevel => "bevel",
            Knob::Thickness => "thickness",
            Knob::Dispersion => "dispersion",
            Knob::Highlight => "highlight",
            Knob::LightAngle => "lightAngle",
            Knob::Shadow => "shadow",
        }
    }

    /// The end of the name of the variable that retunes it for a preset (`glass-regular-blur`).
    #[must_use]
    pub fn variable(self) -> &'static str {
        match self {
            Knob::TintOpacity => "tint-opacity",
            Knob::LightAngle => "light-angle",
            other => other.key(),
        }
    }

    /// Its name in the interface.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Knob::Blur => "Blur",
            Knob::Saturation => "Saturation",
            Knob::Brightness => "Brightness",
            Knob::TintOpacity => "Tint opacity",
            Knob::Refraction => "Refraction",
            Knob::Bevel => "Bevel",
            Knob::Thickness => "Thickness",
            Knob::Dispersion => "Dispersion",
            Knob::Highlight => "Highlight",
            Knob::LightAngle => "Light angle",
            Knob::Shadow => "Shadow",
        }
    }

    /// The least and the most it can be.
    #[must_use]
    pub fn range(self) -> (f32, f32) {
        match self {
            Knob::Blur | Knob::Bevel | Knob::Thickness => (0.0, 200.0),
            Knob::Saturation | Knob::Refraction => (0.0, 3.0),
            Knob::Brightness => (-0.5, 0.5),
            Knob::TintOpacity | Knob::Dispersion | Knob::Highlight | Knob::Shadow => (0.0, 1.0),
            Knob::LightAngle => (-360.0, 360.0),
        }
    }

    /// Whether a frosted material has it too (the others belong to glass alone).
    #[must_use]
    pub fn frosted(self) -> bool {
        matches!(self, Knob::Blur | Knob::Saturation | Knob::Brightness | Knob::TintOpacity | Knob::Highlight | Knob::Shadow)
    }
}

/// A layer's material as its file writes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    /// Frosted or glass, and which.
    pub kind: Kind,
    /// Light, dark, or decided from what is behind.
    pub appearance: Appearance,
    /// The colour the material is tinted with (`#hex` or `$variable`), for a primary action.
    pub tint: Option<String>,
    /// How thick the glass behaves, when the file says; else it follows the layer's size.
    pub depth: Option<Depth>,
    /// The numbers the file sets itself, by [`Knob::key`].
    pub advanced: Map<String, Value>,
}

/// The numbers a material is drawn with, in document units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// Whether the rim bends what is behind it (liquid glass), or the material only blurs.
    pub glass: bool,
    /// Whether this is the dark material.
    pub dark: bool,
    /// The blur's reach, as a `background_blur`'s `radius` (twice the Gaussian's standard deviation).
    pub blur: f32,
    /// Colour strength behind the material; 1 leaves it.
    pub saturation: f32,
    /// Added to what is behind, in linear light.
    pub brightness: f32,
    /// The colour laid over what is behind.
    pub tint: Rgba,
    /// How much of it, 0–1.
    pub tint_opacity: f32,
    /// How strongly the rim bends, 0 for none.
    pub refraction: f32,
    /// The width of the curved rim.
    pub bevel: f32,
    /// The thickness of the glass.
    pub thickness: f32,
    /// Colour split at the rim, 0–1.
    pub dispersion: f32,
    /// The rim light's strength, 0–1.
    pub highlight: f32,
    /// Where the light comes from, degrees clockwise from straight above.
    pub light_angle: f32,
    /// The shadow's opacity, 0–1.
    pub shadow: f32,
    /// How far the shadow's blur reaches.
    pub shadow_blur: f32,
    /// How far down the shadow falls.
    pub shadow_drop: f32,
}

impl Params {
    /// The number `knob` came to.
    #[must_use]
    pub fn number(&self, knob: Knob) -> f32 {
        match knob {
            Knob::Blur => self.blur,
            Knob::Saturation => self.saturation,
            Knob::Brightness => self.brightness,
            Knob::TintOpacity => self.tint_opacity,
            Knob::Refraction => self.refraction,
            Knob::Bevel => self.bevel,
            Knob::Thickness => self.thickness,
            Knob::Dispersion => self.dispersion,
            Knob::Highlight => self.highlight,
            Knob::LightAngle => self.light_angle,
            Knob::Shadow => self.shadow,
        }
    }
}

/// A new material as a file writes it: liquid glass, or frosted.
#[must_use]
pub fn fresh(glass: bool) -> Value {
    json!({ "type": if glass { LIQUID_GLASS } else { FROSTED } })
}

/// The stored material with `change` applied to its keys. A material written as one word is spelled out
/// first; keys this version doesn't know stay.
#[must_use]
pub fn with(stored: &Value, change: impl FnOnce(&mut Map<String, Value>)) -> Value {
    let mut keys = match stored {
        Value::Object(keys) => keys.clone(),
        Value::String(kind) => Map::from_iter([("type".to_owned(), Value::String(kind.clone()))]),
        _ => Map::new(),
    };
    change(&mut keys);
    if keys.get("advanced").and_then(Value::as_object).is_some_and(Map::is_empty) {
        keys.remove("advanced");
    }
    Value::Object(keys)
}

/// How a file's values are looked up while a material is resolved.
pub struct Lookup<'a> {
    /// The number a variable holds, by its name without the `$`.
    pub number: &'a dyn Fn(&str) -> Option<f32>,
    /// The colour of a `#hex` or `$variable` text.
    pub color: &'a dyn Fn(&str) -> Option<Rgba>,
}

fn word<'a>(object: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    object.get(key).and_then(Value::as_str)
}

impl Thickness {
    /// Every thickness, thinnest first.
    pub const ALL: [Thickness; 4] = [Thickness::UltraThin, Thickness::Thin, Thickness::Regular, Thickness::Thick];

    /// Its name in a file.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Thickness::UltraThin => "ultra_thin",
            Thickness::Thin => "thin",
            Thickness::Regular => "regular",
            Thickness::Thick => "thick",
        }
    }

    fn of(text: &str) -> Option<Thickness> {
        Thickness::ALL.into_iter().find(|t| t.key() == text)
    }
}

impl Variant {
    /// Both variants.
    pub const ALL: [Variant; 2] = [Variant::Regular, Variant::Clear];

    /// Its name in a file.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Variant::Regular => "regular",
            Variant::Clear => "clear",
        }
    }

    fn of(text: &str) -> Option<Variant> {
        Variant::ALL.into_iter().find(|v| v.key() == text)
    }
}

impl Appearance {
    /// Every appearance.
    pub const ALL: [Appearance; 3] = [Appearance::Auto, Appearance::Light, Appearance::Dark];

    /// Its name in a file.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Appearance::Auto => "auto",
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        }
    }

    fn of(text: &str) -> Option<Appearance> {
        Appearance::ALL.into_iter().find(|a| a.key() == text)
    }
}

impl Depth {
    /// Every depth, thinnest first.
    pub const ALL: [Depth; 3] = [Depth::Control, Depth::Bar, Depth::Panel];

    /// Its name in a file.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Depth::Control => "control",
            Depth::Bar => "bar",
            Depth::Panel => "panel",
        }
    }

    fn of(text: &str) -> Option<Depth> {
        Depth::ALL.into_iter().find(|d| d.key() == text)
    }

    /// The depth a layer of `size` has by itself: its short side decides.
    #[must_use]
    pub fn of_size(size: (f32, f32)) -> Depth {
        let short = size.0.min(size.1);
        if short <= CONTROL_SIDE {
            Depth::Control
        } else if short <= BAR_SIDE {
            Depth::Bar
        } else {
            Depth::Panel
        }
    }
}

/// The names in `all`, quoted and joined, for a message.
fn choices(all: &[&str]) -> String {
    all.iter().map(|name| format!("\"{name}\"")).collect::<Vec<_>>().join(", ")
}

impl Material {
    /// The material a stored `material` value names, or `None` when it names none this version draws. What
    /// is wrong with the value is in [`problems`]; reading is forgiving, so a file with a mistake still shows.
    #[must_use]
    pub fn read(value: &Value) -> Option<Material> {
        let empty = Map::new();
        let (kind, object) = match value {
            Value::String(kind) => (kind.as_str(), &empty),
            Value::Object(object) => (word(object, "type")?, object),
            _ => return None,
        };
        let kind = match kind {
            FROSTED => Kind::Frosted(word(object, "thickness").and_then(Thickness::of).unwrap_or(Thickness::Regular)),
            LIQUID_GLASS => Kind::Glass(word(object, "variant").and_then(Variant::of).unwrap_or(Variant::Regular)),
            _ => return None,
        };
        Some(Material {
            kind,
            appearance: word(object, "appearance").and_then(Appearance::of).unwrap_or(Appearance::Auto),
            tint: word(object, "tint").map(str::to_owned),
            depth: word(object, "depth").and_then(Depth::of),
            advanced: object.get("advanced").and_then(Value::as_object).cloned().unwrap_or_default(),
        })
    }

    /// Whether this is liquid glass.
    #[must_use]
    pub fn is_glass(&self) -> bool {
        matches!(self.kind, Kind::Glass(_))
    }

    /// The start of the names of the variables that retune this material's preset (`glass-regular`).
    #[must_use]
    pub fn preset(&self) -> String {
        match self.kind {
            Kind::Frosted(thickness) => format!("frosted-{}", thickness.key().replace('_', "-")),
            Kind::Glass(variant) => format!("glass-{}", variant.key()),
        }
    }

    /// The preset's own number for `knob` on a layer of `depth`, before a file or a variable changes it.
    // Human: the one table of the materials. Frosted follows the four system thicknesses (blur, saturation,
    // tint: thicker is more opaque); glass grows thicker, more opaque and deeper in shadow with the layer's
    // size, because a large pane behaves like thick glass. All of it was tuned by eye on renders.
    fn preset_number(&self, knob: Knob, depth: Depth, dark: bool) -> f32 {
        let by_depth = |control: f32, bar: f32, panel: f32| match depth {
            Depth::Control => control,
            Depth::Bar => bar,
            Depth::Panel => panel,
        };
        match self.kind {
            Kind::Frosted(thickness) => {
                let (blur, saturation, tint) = match thickness {
                    Thickness::UltraThin => (24.0, 1.6, 0.20),
                    Thickness::Thin => (40.0, 1.7, 0.35),
                    Thickness::Regular => (60.0, 1.8, 0.55),
                    Thickness::Thick => (80.0, 1.8, 0.75),
                };
                match knob {
                    Knob::Blur => blur,
                    Knob::Saturation => saturation,
                    Knob::Brightness => 0.02,
                    Knob::TintOpacity => tint,
                    Knob::Highlight => {
                        if dark {
                            0.09
                        } else {
                            0.16
                        }
                    }
                    Knob::LightAngle => LIGHT_ANGLE,
                    Knob::Refraction | Knob::Bevel | Knob::Thickness | Knob::Dispersion | Knob::Shadow => 0.0,
                }
            }
            Kind::Glass(variant) => {
                let clear = variant == Variant::Clear;
                match knob {
                    Knob::Blur => {
                        if clear {
                            by_depth(2.0, 3.0, 5.0)
                        } else {
                            by_depth(14.0, 20.0, 36.0)
                        }
                    }
                    Knob::Saturation => {
                        if clear {
                            1.15
                        } else {
                            1.4
                        }
                    }
                    Knob::Brightness => match (clear, dark) {
                        (true, _) => 0.0,
                        (false, false) => 0.03,
                        (false, true) => -0.01,
                    },
                    Knob::TintOpacity => match (clear, dark) {
                        (true, _) => by_depth(0.02, 0.03, 0.05),
                        // Human: regular glass has to carry a label over anything, a sunset included, so it
                        // keeps well over a third of its own white or dark; clear glass is the see-through one.
                        (false, false) => by_depth(0.34, 0.42, 0.62),
                        (false, true) => by_depth(0.50, 0.58, 0.76),
                    },
                    Knob::Refraction => 1.0,
                    Knob::Bevel => by_depth(14.0, 20.0, 26.0),
                    Knob::Thickness => by_depth(22.0, 30.0, 40.0),
                    Knob::Dispersion => 0.0,
                    Knob::Highlight => {
                        if dark {
                            0.42
                        } else {
                            0.6
                        }
                    }
                    Knob::LightAngle => LIGHT_ANGLE,
                    Knob::Shadow => by_depth(0.12, 0.16, 0.22),
                }
            }
        }
    }

    /// The numbers to draw this material with on a layer of `size`; `dark` is what `auto` came to.
    ///
    /// Each number is the file's own (`advanced`), else the document's variable for the preset
    /// (`glass-regular-blur`), else the preset's; the light's direction can also be set once for the whole
    /// document ([`LIGHT_VARIABLE`]).
    #[must_use]
    pub fn resolve(&self, size: (f32, f32), dark: bool, lookup: &Lookup<'_>) -> Params {
        let dark = match self.appearance {
            Appearance::Auto => dark,
            Appearance::Light => false,
            Appearance::Dark => true,
        };
        let depth = self.depth.unwrap_or_else(|| Depth::of_size(size));
        let preset = self.preset();
        let number = |knob: Knob| -> f32 {
            let own = match self.advanced.get(knob.key()) {
                Some(Value::Number(n)) => n.as_f64().map(pen_doc::value::narrow),
                Some(Value::String(name)) => name.strip_prefix('$').and_then(|name| (lookup.number)(name)),
                _ => None,
            };
            let shared = || (lookup.number)(&format!("{preset}-{}", knob.variable()));
            let document = || (knob == Knob::LightAngle).then(|| (lookup.number)(LIGHT_VARIABLE)).flatten();
            let (least, most) = knob.range();
            let value = own.or_else(shared).or_else(document).unwrap_or_else(|| self.preset_number(knob, depth, dark));
            if value.is_finite() { value.clamp(least, most) } else { least.max(0.0) }
        };
        let own_tint = self.tint.as_deref().and_then(|text| (lookup.color)(text));
        let base = if dark { DARK } else { Rgba::WHITE };
        let short = size.0.min(size.1).max(0.0);
        let mut tint_opacity = number(Knob::TintOpacity);
        // Human: a tinted glass is coloured glass: mostly its colour, the rest what is behind it. A file that
        // sets the opacity itself is taken at its word.
        if own_tint.is_some() && self.is_glass() && !self.advanced.contains_key(Knob::TintOpacity.key()) {
            tint_opacity = tint_opacity.max(GLASS_TINT);
        }
        let tint = own_tint.unwrap_or(base);
        let shadow_reach = match depth {
            Depth::Control => (14.0, 4.0),
            Depth::Bar => (26.0, 8.0),
            Depth::Panel => (44.0, 14.0),
        };
        Params {
            glass: self.is_glass(),
            dark,
            blur: number(Knob::Blur),
            saturation: number(Knob::Saturation),
            // Coloured glass keeps its colour: it isn't lightened or darkened with the appearance.
            brightness: if own_tint.is_some() && self.is_glass() && !self.advanced.contains_key(Knob::Brightness.key()) {
                0.0
            } else {
                number(Knob::Brightness)
            },
            tint: Rgba { a: 255, ..tint },
            tint_opacity: (tint_opacity * f32::from(tint.a) / 255.0).clamp(0.0, 1.0),
            refraction: number(Knob::Refraction),
            // A rim can't be wider than half the layer.
            bevel: number(Knob::Bevel).min(short / 2.0),
            thickness: number(Knob::Thickness),
            dispersion: number(Knob::Dispersion),
            highlight: number(Knob::Highlight),
            light_angle: number(Knob::LightAngle),
            shadow: number(Knob::Shadow),
            shadow_blur: shadow_reach.0,
            shadow_drop: shadow_reach.1,
        }
    }
}

/// What is wrong with a stored `material` value, each as a sentence that says how to put it right. Empty when
/// the value is sound.
#[must_use]
pub fn problems(value: &Value) -> Vec<String> {
    const KEYS: [&str; 7] = ["type", "variant", "thickness", "appearance", "tint", "depth", "advanced"];
    let types = choices(&[FROSTED, LIQUID_GLASS]);
    let object = match value {
        Value::Null => return Vec::new(),
        Value::String(kind) if kind == FROSTED || kind == LIQUID_GLASS => return Vec::new(),
        Value::String(kind) => return vec![format!("`material` \"{kind}\" is not a material; use {types}, or null for none")],
        Value::Object(object) => object,
        _ => return vec![format!("`material` must be {types}, or an object like {{\"type\":\"liquid_glass\"}}, or null for none")],
    };
    let mut found = Vec::new();
    let kind = word(object, "type");
    match kind {
        Some(FROSTED | LIQUID_GLASS) => {}
        Some(other) => found.push(format!("`material.type` \"{other}\" is not a material; use {types}")),
        None => found.push(format!("`material` needs a `type`: {types}")),
    }
    for key in object.keys().filter(|key| !KEYS.contains(&key.as_str())) {
        found.push(format!("`material.{key}` is not a material property; the properties are {}", choices(&KEYS)));
    }
    let mut choice = |key: &str, valid: &[&str], only: Option<&str>| {
        let Some(given) = object.get(key) else { return };
        if let Some(only) = only.filter(|only| kind.is_some_and(|kind| kind != *only)) {
            found.push(format!("`material.{key}` belongs to \"{only}\" only; leave it out of a \"{}\" material", kind.unwrap_or("")));
        } else if !given.as_str().is_some_and(|given| valid.contains(&given)) {
            found.push(format!("`material.{key}` must be one of {}", choices(valid)));
        }
    };
    choice("variant", &Variant::ALL.map(Variant::key), Some(LIQUID_GLASS));
    choice("thickness", &Thickness::ALL.map(Thickness::key), Some(FROSTED));
    choice("appearance", &Appearance::ALL.map(Appearance::key), None);
    choice("depth", &Depth::ALL.map(Depth::key), Some(LIQUID_GLASS));
    match object.get("tint") {
        None | Some(Value::Null) => {}
        Some(Value::String(text)) if text.starts_with('$') || Rgba::parse(text).is_some() => {}
        Some(_) => found.push("`material.tint` must be a colour (\"#rrggbb\" or \"#rrggbbaa\") or a \"$variable\"".to_owned()),
    }
    match object.get("advanced") {
        None | Some(Value::Null) => {}
        Some(Value::Object(advanced)) => {
            for (key, given) in advanced {
                let Some(knob) = Knob::ALL.into_iter().find(|knob| knob.key() == key) else {
                    let keys = Knob::ALL.map(Knob::key);
                    found.push(format!("`material.advanced.{key}` is not a number of a material; the numbers are {}", choices(&keys)));
                    continue;
                };
                let (least, most) = knob.range();
                match given {
                    Value::String(name) if name.starts_with('$') => {}
                    Value::Number(n) if n.as_f64().is_some_and(|n| (f64::from(least)..=f64::from(most)).contains(&n)) => {}
                    _ => found.push(format!("`material.advanced.{key}` must be a number from {least} to {most}, or a \"$variable\"")),
                }
                if kind == Some(FROSTED) && !knob.frosted() {
                    found.push(format!("`material.advanced.{key}` belongs to \"{LIQUID_GLASS}\" only; a frosted material doesn't bend light"));
                }
            }
        }
        Some(_) => found.push("`material.advanced` must be an object of numbers, like {\"blur\": 10}".to_owned()),
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NOTHING: Lookup<'static> = Lookup { number: &|_| None, color: &|text| Rgba::parse(text) };

    #[test]
    fn a_material_is_read_from_a_word_or_an_object() {
        let glass = Material::read(&json!("liquid_glass")).unwrap();
        assert_eq!((glass.kind, glass.appearance, glass.depth), (Kind::Glass(Variant::Regular), Appearance::Auto, None));
        let clear = Material::read(&json!({"type": "liquid_glass", "variant": "clear", "appearance": "dark", "depth": "panel"})).unwrap();
        assert_eq!((clear.kind, clear.appearance, clear.depth), (Kind::Glass(Variant::Clear), Appearance::Dark, Some(Depth::Panel)));
        let frosted = Material::read(&json!({"type": "frosted", "thickness": "ultra_thin"})).unwrap();
        assert_eq!(frosted.kind, Kind::Frosted(Thickness::UltraThin));
        assert_eq!(frosted.preset(), "frosted-ultra-thin");
        assert!(Material::read(&json!("plastic")).is_none());
        assert!(Material::read(&json!(3)).is_none());
    }

    #[test]
    fn glass_grows_thicker_with_the_layer_and_frosted_bends_nothing() {
        let glass = Material::read(&json!("liquid_glass")).unwrap();
        let button = glass.resolve((44.0, 44.0), false, &NOTHING);
        let bar = glass.resolve((320.0, 64.0), false, &NOTHING);
        let sheet = glass.resolve((360.0, 400.0), false, &NOTHING);
        assert!(button.glass && button.refraction > 0.0);
        assert!(button.thickness < bar.thickness && bar.thickness < sheet.thickness);
        assert!(button.tint_opacity < sheet.tint_opacity && button.shadow < sheet.shadow);
        // The rim never takes more than half the layer.
        assert!(glass.resolve((16.0, 16.0), false, &NOTHING).bevel <= 8.0);
        let frosted = Material::read(&json!("frosted")).unwrap().resolve((300.0, 200.0), false, &NOTHING);
        assert!(!frosted.glass && frosted.refraction == 0.0 && frosted.blur > button.blur);
    }

    #[test]
    fn a_number_comes_from_the_file_then_a_variable_then_the_preset() {
        let glass = Material::read(&json!({"type": "liquid_glass", "advanced": {"blur": 3, "highlight": "$shine", "refraction": 99}})).unwrap();
        let number = |name: &str| match name {
            "shine" => Some(0.25),
            "glass-regular-blur" => Some(50.0),
            "glass-regular-saturation" => Some(2.0),
            LIGHT_VARIABLE => Some(30.0),
            _ => None,
        };
        let params = glass.resolve((44.0, 44.0), false, &Lookup { number: &number, color: &|text| Rgba::parse(text) });
        assert_eq!((params.blur, params.highlight, params.saturation, params.light_angle), (3.0, 0.25, 2.0, 30.0));
        // A number beyond its range is brought back into it.
        assert_eq!(params.refraction, Knob::Refraction.range().1);
    }

    #[test]
    fn the_appearance_and_the_tint_decide_the_colour() {
        let auto = Material::read(&json!("liquid_glass")).unwrap();
        assert_eq!(auto.resolve((44.0, 44.0), false, &NOTHING).tint, Rgba::WHITE);
        assert_eq!(auto.resolve((44.0, 44.0), true, &NOTHING).tint, DARK);
        let pinned = Material::read(&json!({"type": "liquid_glass", "appearance": "light"})).unwrap();
        assert!(!pinned.resolve((44.0, 44.0), true, &NOTHING).dark);
        let tinted = Material::read(&json!({"type": "liquid_glass", "tint": "#0a84ff"})).unwrap().resolve((44.0, 44.0), false, &NOTHING);
        assert_eq!(tinted.tint, Rgba { r: 0x0a, g: 0x84, b: 0xff, a: 0xff });
        assert!(tinted.tint_opacity >= GLASS_TINT);
    }

    #[test]
    fn a_stored_material_is_changed_key_by_key() {
        let glass = with(&json!("liquid_glass"), |keys| {
            keys.insert("variant".to_owned(), json!("clear"));
        });
        assert_eq!(glass, json!({"type": "liquid_glass", "variant": "clear"}));
        let kept = with(&json!({"type": "frosted", "future": 1, "advanced": {"blur": 4}}), |keys| {
            keys.insert("advanced".to_owned(), json!({}));
        });
        assert_eq!(kept, json!({"type": "frosted", "future": 1}));
        assert_eq!(fresh(true), json!({"type": "liquid_glass"}));
        let params = Material::read(&fresh(false)).unwrap().resolve((100.0, 100.0), false, &NOTHING);
        assert!((params.number(Knob::Blur) - params.blur).abs() < f32::EPSILON);
    }

    #[test]
    fn problems_say_what_to_write_instead() {
        assert_eq!(problems(&json!("liquid_glass")), Vec::<String>::new());
        assert_eq!(problems(&json!({"type": "frosted", "thickness": "thin", "advanced": {"blur": "$b"}})), Vec::<String>::new());
        assert_eq!(problems(&Value::Null), Vec::<String>::new());
        let wrong = problems(&json!({"type": "glass", "varient": "clear"}));
        assert!(wrong[0].contains("\"liquid_glass\"") && wrong[1].contains("`material.varient`"), "{wrong:?}");
        let mixed = problems(&json!({"type": "frosted", "variant": "clear", "advanced": {"refraction": 1, "blur": 900, "glow": 1}}));
        assert_eq!(mixed.len(), 4, "{mixed:?}");
        assert!(mixed.iter().any(|m| m.contains("from 0 to 200")));
        assert!(problems(&json!({"type": "liquid_glass", "tint": "blue"}))[0].contains("#rrggbb"));
        assert!(problems(&json!(7))[0].contains("null for none"));
    }
}
