//! Checks on a design that uses materials: what each material came to, whether what is written on it can be
//! read, and whether the layers are built the way glass is meant to be used.
//!
//! The tree checks need only the scene. The measured ones need a picture of it ([`crate::render::Survey`]):
//! contrast is taken from the pixels under each label, against the lightest and the darkest of them, never an
//! average, because a label over a busy backdrop is only as legible as its worst spot.

use pen_doc::color::Rgba;
use pen_doc::geom::Rect;
use pen_doc::layout;
use pen_doc::model::{Corners, Kind, Paint};
use pen_doc::scene::SceneNode;
use pen_doc::value::Flag;
use pen_doc::vars::Vars;
use serde_json::{Value, json};

use crate::material::{self, Material, Variant};
use crate::render::Survey;

/// The least contrast between a label and what it lies on (WCAG AA for text).
pub const LEAST_CONTRAST: f32 = 4.5;
/// The key that makes a frame a glass group; its value is the spacing its glass children merge within.
pub const GROUP: &str = "glassGroup";
/// How far a nested corner's radius may be off the concentric one, in units, before it is said.
const RADIUS_SLACK: f32 = 1.5;
/// A backdrop whose lightest and darkest parts differ by less than this has nothing for glass to bend.
const FLAT_BELOW: f32 = 0.008;
/// How many of the lightest and of the darkest pixels under a label are left out as noise, in a thousand.
const TRIM: usize = 30;
/// How many problems one answer carries.
const MOST_PROBLEMS: usize = 24;

/// A layer that has a material, as the checks see it.
struct Pane<'a> {
    scene: &'a SceneNode,
    material: Material,
    bounds: Rect,
    /// The glass group it is in: the id of its parent frame.
    group: Option<&'a str>,
    /// The nearest layer above it that has a material too.
    on: Option<&'a str>,
    /// Which of the checked top-level layers it is in.
    root: usize,
}

/// Something written or drawn: a text or an icon.
struct Label<'a> {
    scene: &'a SceneNode,
    /// The pane it lies on, as an index into the panes; `None` for one that lies on the content.
    pane: Option<usize>,
    color: Rgba,
}

/// The least contrast large text needs (WCAG AA from 24 px).
const LEAST_LARGE: f32 = 3.0;
/// From this size on text is large.
const LARGE_TEXT: f32 = 24.0;

/// A layer's name and id, for a sentence.
fn named(scene: &SceneNode) -> String {
    format!("\"{}\" ({})", scene.node.display_name(), scene.id)
}

/// The spacing of a glass group, when `scene` is one.
#[must_use]
pub fn group_spacing(scene: &SceneNode) -> Option<f32> {
    scene.node.extra.get(GROUP).and_then(Value::as_f64).map(pen_doc::value::narrow).filter(|spacing| *spacing >= 0.0)
}

/// What is wrong with a stored `glassGroup` value, as a sentence; `None` when it is sound.
#[must_use]
pub fn group_problem(kind: Option<&Kind>, value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    if kind.is_some_and(|kind| *kind != Kind::Frame) {
        return Some(format!("`{GROUP}` belongs on a frame: the frame around the glass layers that should merge"));
    }
    (!value.as_f64().is_some_and(|spacing| (0.0..=200.0).contains(&spacing))).then(|| {
        format!("`{GROUP}` must be a number from 0 to 200: the spacing within which the frame's glass children merge (use the frame's `gap`)")
    })
}

/// The one radius all four corners of a box have, in units, when they have one.
fn radius_of(scene: &SceneNode, vars: Vars<'_>) -> Option<f32> {
    let most = scene.local.w.min(scene.local.h) / 2.0;
    let number = |num| vars.number(num, &scene.theme).unwrap_or(0.0).clamp(0.0, most);
    match (&scene.node.kind, &scene.node.corner_radius) {
        (Kind::Ellipse, _) => Some(most),
        (Kind::Frame | Kind::Rectangle, Some(Corners::All(all))) => Some(number(all)),
        (Kind::Frame | Kind::Rectangle, Some(Corners::Each(each))) => {
            let radii: Vec<f32> = each.iter().map(number).collect();
            radii.first().copied().filter(|first| radii.iter().all(|r| (r - first).abs() < 0.01))
        }
        (Kind::Frame | Kind::Rectangle, None) => Some(0.0),
        _ => None,
    }
}

/// The one colour a layer is filled with, when it is.
fn solid(scene: &SceneNode, vars: Vars<'_>) -> Option<Rgba> {
    match scene.node.fill.as_ref()?.as_slice().last()? {
        Paint::Solid(text) => vars.color(text, &scene.theme),
        Paint::Object(object) if object.get("type").and_then(Value::as_str) == Some("color") => vars.paint_color(object, &scene.theme),
        Paint::Object(_) => None,
    }
}

fn hidden(scene: &SceneNode) -> bool {
    matches!(scene.node.enabled, Some(Flag::Value(false)))
}

/// Gathers the panes and the labels on them, in paint order.
fn gather<'a>(
    scene: &'a SceneNode,
    root: usize,
    on: Option<usize>,
    parent: Option<&'a SceneNode>,
    vars: Vars<'_>,
    panes: &mut Vec<Pane<'a>>,
    labels: &mut Vec<Label<'a>>,
) {
    if hidden(scene) {
        return;
    }
    let mut on = on;
    if let Some(material) = scene.node.material.as_ref().and_then(Material::read) {
        let group = parent.filter(|parent| group_spacing(parent).is_some()).map(|parent| parent.id.as_str());
        let above = on.and_then(|index| panes.get(index)).map(|pane| pane.scene.id.as_str());
        panes.push(Pane { scene, material, bounds: layout::doc_bounds(scene), group, on: above, root });
        on = Some(panes.len() - 1);
    } else if matches!(scene.node.kind, Kind::Text | Kind::Icon | Kind::IconFont) {
        labels.push(Label { scene, pane: on, color: solid(scene, vars).unwrap_or(Rgba::BLACK) });
    }
    for child in &scene.children {
        gather(child, root, on, Some(scene), vars, panes, labels);
    }
}

/// `scene` with every text and icon hidden, so a picture of it shows what they lie on.
#[must_use]
pub fn without_labels(scene: &SceneNode) -> SceneNode {
    fn strip(scene: &mut SceneNode) {
        for child in &mut scene.children {
            if matches!(child.node.kind, Kind::Text | Kind::Icon | Kind::IconFont) {
                child.node.enabled = Some(Flag::Value(false));
            } else {
                strip(child);
            }
        }
    }
    let mut copy = scene.clone();
    strip(&mut copy);
    copy
}

/// The lightest and the darkest the picture is under `bounds` (WCAG luminance, on white where it is clear),
/// leaving out a few stray pixels at either end. `None` when the box is off the picture.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "pixel positions, clamped to the picture before the cast")]
fn extremes(survey: &Survey, bounds: &Rect) -> Option<(f32, f32)> {
    let picture = &survey.picture;
    let at = |v: f32, origin: f32, limit: u16| usize::from(((v - origin) * survey.scale).round().clamp(0.0, f32::from(limit)) as u16);
    let side = |pixels: usize| u16::try_from(pixels).unwrap_or(u16::MAX);
    let (x0, x1) = (at(bounds.x, survey.origin.x, side(picture.w)), at(bounds.right(), survey.origin.x, side(picture.w)));
    let (y0, y1) = (at(bounds.y, survey.origin.y, side(picture.h)), at(bounds.bottom(), survey.origin.y, side(picture.h)));
    let linear = |byte: f32| if byte <= 0.040_45 { byte / 12.92 } else { ((byte + 0.055) / 1.055).powf(2.4) };
    let mut levels: Vec<f32> = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let Some(&[r, g, b, a]) = picture.rgba.get((y * picture.w + x) * 4..).and_then(|rest| rest.first_chunk::<4>()) else { continue };
            // Premultiplied bytes on white.
            let on_white = |c: u8| (f32::from(c) + f32::from(255 - a)) / 255.0;
            levels.push(0.2126 * linear(on_white(r)) + 0.7152 * linear(on_white(g)) + 0.0722 * linear(on_white(b)));
        }
    }
    if levels.is_empty() {
        return None;
    }
    levels.sort_by(f32::total_cmp);
    let skip = levels.len() * TRIM / 1000;
    Some((*levels.get(skip)?, *levels.get(levels.len() - 1 - skip)?))
}

/// The WCAG contrast of two luminances.
fn contrast(a: f32, b: f32) -> f32 {
    let (a, b) = (a + 0.05, b + 0.05);
    if a > b { a / b } else { b / a }
}

/// What is wrong with how the panes are built, each as a sentence that says how to put it right.
fn tree_problems(panes: &[Pane<'_>], scenes: &[&SceneNode], vars: Vars<'_>) -> Vec<String> {
    let mut found = Vec::new();
    for (index, pane) in panes.iter().enumerate() {
        let glass = pane.material.is_glass();
        if !matches!(pane.scene.node.kind, Kind::Frame | Kind::Rectangle | Kind::Browser | Kind::Ellipse) && glass {
            found.push(format!(
                "{} is a {:?}: liquid glass needs a frame, rectangle or ellipse to bend light along its edge, so this one is drawn frosted. Put the material on a frame of that shape instead.",
                named(pane.scene),
                pane.scene.node.kind
            ));
        }
        if !glass {
            continue;
        }
        if let Some(under) = pane.on {
            found.push(format!(
                "{} is glass on the material of \"{under}\". Glass never lies on glass: make it a plain fill (for a selection, `#ffffff33` or `#00000022`), or move it out of that layer.",
                named(pane.scene)
            ));
            continue;
        }
        // Glass painted earlier that this one covers, and that isn't merged with it in a group.
        for lower in panes.iter().take(index).filter(|lower| lower.material.is_glass() && lower.root == pane.root) {
            let apart = pane.group.is_none() || pane.group != lower.group;
            let overlap = (pane.bounds.right().min(lower.bounds.right()) - pane.bounds.x.max(lower.bounds.x))
                .min(pane.bounds.bottom().min(lower.bounds.bottom()) - pane.bounds.y.max(lower.bounds.y));
            if apart && overlap > 1.0 {
                found.push(format!(
                    "{} overlaps the glass of {}; glass can't show other glass through it. Move them apart, or put both in one frame with `{GROUP}`.",
                    named(pane.scene),
                    named(lower.scene)
                ));
            }
        }
    }
    for (root, scene) in scenes.iter().enumerate() {
        let variants: Vec<Variant> = panes
            .iter()
            .filter(|pane| pane.root == root)
            .filter_map(|pane| match pane.material.kind {
                material::Kind::Glass(variant) => Some(variant),
                material::Kind::Frosted(_) => None,
            })
            .collect();
        if variants.contains(&Variant::Regular) && variants.contains(&Variant::Clear) {
            found.push(format!(
                "{} mixes regular and clear glass. Use one variant in one screen: regular by default, clear only over media.",
                named(scene)
            ));
        }
    }
    for pane in panes {
        nested_radii(pane.scene, pane.scene, vars, &mut found);
    }
    found
}

/// Says each box inside `pane` whose corners don't nest in the pane's: a nested radius is the outer radius
/// less the gap between the two edges.
fn nested_radii(pane: &SceneNode, inner: &SceneNode, vars: Vars<'_>, found: &mut Vec<String>) {
    let Some(outer_radius) = radius_of(pane, vars).filter(|radius| *radius > 0.0) else { return };
    let outer = layout::doc_bounds(pane);
    for child in inner.children.iter().filter(|child| !hidden(child)) {
        let bounds = layout::doc_bounds(child);
        // Human: a child nests in a corner of the pane when one of its own corners lies in the square that
        // corner's rounding takes up; a child in the middle of a long bar is near no corner at all.
        let near = |x: f32, y: f32| (x >= 0.0 && y >= 0.0 && x < outer_radius && y < outer_radius).then(|| x.min(y));
        let (left, top) = (bounds.x - outer.x, bounds.y - outer.y);
        let (right, bottom) = (outer.right() - bounds.right(), outer.bottom() - bounds.bottom());
        let gap = [near(left, top), near(right, top), near(right, bottom), near(left, bottom)].into_iter().flatten().reduce(f32::min);
        let painted = child.node.fill.is_some() || child.node.stroke.is_some() || child.node.material.is_some();
        if let (true, Some(gap), Some(radius)) = (painted, gap, radius_of(child, vars)) {
            let wanted = outer_radius - gap;
            // A child too small to reach the wanted radius is as round as it can be, which nests too.
            let roundest = child.local.w.min(child.local.h) / 2.0;
            let reachable = wanted.min(roundest);
            // A capsule is a shape of its own and nests anywhere.
            let capsule = (radius - roundest).abs() < 0.5;
            if !capsule && (radius - reachable).abs() > RADIUS_SLACK {
                found.push(format!(
                    "{} has corner radius {radius:.0} inside {} (radius {outer_radius:.0}, {gap:.0} from its edge): nested corners share a centre, so its radius should be {outer_radius:.0} - {gap:.0} = {reachable:.0}.",
                    named(child),
                    named(pane)
                ));
            }
        }
        if child.node.material.is_none() {
            nested_radii(pane, child, vars, found);
        }
    }
}

/// The checks on `scenes`: what each material is and came to, and every problem found. `survey` is a picture
/// of the same scenes with their labels hidden ([`without_labels`]); without it only the tree is checked.
#[must_use]
pub fn check(scenes: &[&SceneNode], vars: Vars<'_>, survey: Option<&Survey>) -> Value {
    let (mut panes, mut labels) = (Vec::new(), Vec::new());
    for (root, scene) in scenes.iter().enumerate() {
        gather(scene, root, None, None, vars, &mut panes, &mut labels);
    }
    let mut problems = tree_problems(&panes, scenes, vars);
    let mut materials = Vec::new();
    for pane in &panes {
        let seen = survey.and_then(|survey| survey.materials.iter().find(|seen| seen.id == pane.scene.id));
        let mut entry = json!({ "id": pane.scene.id, "name": pane.scene.node.display_name(), "material": pane.material.preset() });
        if let Some(tint) = &pane.material.tint {
            entry["tint"] = json!(tint);
        }
        if let Some(seen) = seen {
            entry["appearance"] = json!(if seen.dark { "dark" } else { "light" });
            entry["backdropLightness"] = json!((f64::from(seen.lightness) * 100.0).round() / 100.0);
            entry["tintOpacity"] = json!((f64::from(seen.tint_opacity) * 100.0).round() / 100.0);
            if pane.material.is_glass() && seen.spread < FLAT_BELOW {
                problems.push(format!(
                    "{} has a flat backdrop: there is nothing behind it for the glass to bend, so it reads as a grey box. If content belongs under it, run that content behind it; on a screen that is plain by design give this layer a plain fill instead of a material. Don't add a gradient, colour field or picture behind it that the user didn't ask for.",
                    named(pane.scene)
                ));
            }
        }
        materials.push(entry);
    }
    // What the pane's material came to, for a sentence about a label on it.
    let came_out = |pane: &SceneNode| match survey.and_then(|survey| survey.materials.iter().find(|seen| seen.id == pane.id)) {
        Some(seen) if seen.dark => ", which came out as dark glass,",
        Some(_) => ", which came out as light glass,",
        None => "",
    };
    let mut lowest: Option<f32> = None;
    let mut measured = 0;
    for label in &labels {
        let pane = label.pane.and_then(|index| panes.get(index));
        let Some((dark, light)) = survey.and_then(|survey| extremes(survey, &layout::doc_bounds(label.scene))) else { continue };
        let own = label.color.luminance();
        let (against_dark, against_light) = (contrast(own, dark), contrast(own, light));
        let worst = against_dark.min(against_light);
        measured += 1;
        lowest = Some(lowest.map_or(worst, |lowest: f32| lowest.min(worst)));
        let size = label.scene.node.font_size.as_ref().and_then(|size| vars.number(size, &label.scene.theme)).unwrap_or(0.0);
        let needed = if label.scene.node.kind == Kind::Text && size >= LARGE_TEXT { LEAST_LARGE } else { LEAST_CONTRAST };
        if worst < needed {
            let too_light = against_light < against_dark;
            // Whether the label is the darker of the two there: then darker still is what helps.
            let darker = own < if too_light { light } else { dark };
            let where_ = if too_light { "the lightest part of what is under it" } else { "the darkest part of what is under it" };
            let clear = pane.is_some_and(|pane| pane.material.kind == material::Kind::Glass(Variant::Clear));
            // Human: the cure depends on what the label lies on: glass can change its appearance, clear
            // glass needs what is behind it dimmed, and a label on the content needs a scrim or another place.
            // A label that is already black or white can't be pushed further; then the surface has to change.
            let spent = if darker { own < 0.03 } else { own > 0.85 };
            let frosted = pane.is_some_and(|pane| !pane.material.is_glass());
            let (on, fix) = match (pane, too_light, clear) {
                (Some(pane), _, false) if spent && frosted => (
                    format!("on {}{}", named(pane.scene), came_out(pane.scene)),
                    "use a thicker frosted material (`thickness` \"thick\"), which hides more of what is behind",
                ),
                (Some(pane), _, false) if spent => (
                    format!("on {}{}", named(pane.scene), came_out(pane.scene)),
                    "the label can't go further, so make the surface under it calmer: remove or weaken a selection fill under the label, give the glass `\"depth\":\"panel\"` (thicker, more opaque glass), or as a last resort raise `advanced.tintOpacity` (0.7 to 0.85)",
                ),
                (Some(pane), true, true) => (
                    format!("on {}{}", named(pane.scene), came_out(pane.scene)),
                    "darken what is behind this control further (a darker dimming rectangle or a gradient scrim under it, between the media and the glass; start at 35 % black and go up to about 60 %), or use the regular variant",
                ),
                (Some(pane), _, _) if darker => (
                    format!("on {}{}", named(pane.scene), came_out(pane.scene)),
                    "use a darker label (black, or a deeper shade of the colour), or pin the material's `appearance` to \"dark\" and make the label white",
                ),
                (Some(pane), _, _) => (
                    format!("on {}{}", named(pane.scene), came_out(pane.scene)),
                    "use a lighter label (white), or pin the material's `appearance` to \"light\" and make the label black",
                ),
                (None, true, _) => (
                    "on the content".to_owned(),
                    "use a darker label, put a dark gradient scrim between the picture and the text, or move the text off the light part",
                ),
                (None, false, _) => (
                    "on the content".to_owned(),
                    "use a lighter label, put a light scrim or a frosted panel behind the text, or move it off the dark part",
                ),
            };
            problems.push(format!(
                "{} ({}) {on} has a contrast of {worst:.1}:1 against {where_}; it needs {needed}:1. To fix: {fix}.",
                named(label.scene),
                label.color.to_hex()
            ));
        }
    }
    let more = problems.len().saturating_sub(MOST_PROBLEMS);
    problems.truncate(MOST_PROBLEMS);
    if more > 0 {
        problems.push(format!("... and {more} more of the same kinds."));
    }
    let mut answer = json!({ "materials": materials, "labelsMeasured": measured, "problems": problems, "ok": problems.is_empty() });
    if let Some(lowest) = lowest {
        answer["lowestContrast"] = json!((f64::from(lowest) * 10.0).round() / 10.0);
    }
    if panes.is_empty() {
        answer["note"] = json!("No layer here has a `material`.");
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;

    fn editor(json: &str) -> Editor {
        let mut editor = Editor::new(pen_doc::read(json).unwrap(), None, 1);
        editor.refresh(&|_| (10.0, 10.0));
        editor
    }

    fn problems(json: &str) -> Vec<String> {
        let editor = editor(json);
        let scenes: Vec<&SceneNode> = editor.scene.iter().collect();
        let answer = check(&scenes, Vars::new(&editor.doc), None);
        answer["problems"].as_array().unwrap().iter().map(|p| p.as_str().unwrap().to_owned()).collect()
    }

    #[test]
    fn glass_on_glass_and_overlapping_glass_are_said_and_a_group_is_not() {
        let said = problems(
            r#"{"version":"2.22","children":[{"type":"frame","id":"screen","x":0,"y":0,"width":400,"height":400,"layout":"none","children":[
              {"type":"frame","id":"bar","x":20,"y":20,"width":300,"height":60,"cornerRadius":30,"material":"liquid_glass","layout":"none","children":[
                {"type":"frame","id":"pill","x":4,"y":4,"width":80,"height":52,"cornerRadius":26,"material":"liquid_glass"}]},
              {"type":"frame","id":"menu","x":200,"y":50,"width":150,"height":200,"cornerRadius":20,"material":"liquid_glass"},
              {"type":"frame","id":"row","x":20,"y":300,"width":200,"height":44,"layout":"horizontal","gap":8,"glassGroup":8,"children":[
                {"type":"frame","id":"a","width":44,"height":44,"cornerRadius":22,"material":"liquid_glass"},
                {"type":"frame","id":"b","width":44,"height":44,"cornerRadius":22,"material":"liquid_glass"}]}]}]}"#,
        );
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said[0].contains("(pill) is glass on the material of \"bar\""), "{}", said[0]);
        assert!(said[1].contains("(menu) overlaps the glass of") && said[1].contains("(bar)"), "{}", said[1]);
    }

    #[test]
    fn nested_corners_are_checked_against_the_outer_radius_less_the_gap() {
        let said = problems(
            r##"{"version":"2.22","children":[
              {"type":"frame","id":"bar","x":0,"y":0,"width":300,"height":64,"cornerRadius":32,"material":"liquid_glass","layout":"none","children":[
                {"type":"frame","id":"good","x":4,"y":4,"width":90,"height":56,"cornerRadius":28,"fill":"#ffffff33"},
                {"type":"frame","id":"bad","x":204,"y":4,"width":92,"height":56,"cornerRadius":8,"fill":"#ffffff33"},
                {"type":"frame","id":"pill","x":100,"y":4,"width":60,"height":20,"cornerRadius":10,"fill":"#ffffff33"},
                {"type":"frame","id":"far","x":120,"y":22,"width":20,"height":20,"cornerRadius":4,"fill":"#ffffff33"},
                {"type":"text","id":"t","x":100,"y":20,"content":"x","fill":"#000000"}]}]}"##,
        );
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].contains("(bad) has corner radius 8") && said[0].contains("32 - 4 = 28"), "{}", said[0]);
    }

    #[test]
    fn mixed_variants_and_glass_on_a_shape_without_an_edge_are_said() {
        let said = problems(
            r##"{"version":"2.22","children":[{"type":"frame","id":"screen","x":0,"y":0,"width":400,"height":400,"layout":"none","children":[
              {"type":"frame","id":"one","x":0,"y":0,"width":100,"height":44,"cornerRadius":22,"material":"liquid_glass"},
              {"type":"frame","id":"two","x":0,"y":100,"width":100,"height":44,"cornerRadius":22,"material":{"type":"liquid_glass","variant":"clear"}},
              {"type":"polygon","id":"tri","x":200,"y":200,"width":80,"height":80,"fill":"#ffffff20","material":"liquid_glass"},
              {"type":"frame","id":"card","x":200,"y":0,"width":150,"height":100,"cornerRadius":16,"material":"frosted"}]}]}"##,
        );
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said[0].contains("(tri) is a Polygon") && said[1].contains("mixes regular and clear"), "{said:?}");
    }

    #[test]
    fn every_text_and_icon_is_hidden_for_the_picture_they_are_measured_on() {
        let editor = editor(
            r##"{"version":"2.22","children":[{"type":"frame","id":"screen","x":0,"y":0,"width":400,"height":400,"layout":"none","children":[
              {"type":"text","id":"title","x":0,"y":0,"content":"Feed","fill":"#000000"},
              {"type":"frame","id":"bar","x":0,"y":100,"width":300,"height":60,"material":"liquid_glass","children":[
                {"type":"frame","id":"tab","children":[{"type":"text","id":"label","content":"Home","fill":"#000000"}]}]}]}]}"##,
        );
        let bare = without_labels(&editor.scene[0]);
        let mut off = Vec::new();
        bare.walk(
            &mut |scene, _| {
                if hidden(scene) {
                    off.push(scene.id.clone());
                }
            },
            0,
        );
        assert_eq!(off, ["title", "label"]);
    }

    #[test]
    fn a_label_is_measured_against_the_worst_pixels_under_it() {
        let editor = editor(
            r##"{"version":"2.22","children":[{"type":"frame","id":"bar","x":0,"y":0,"width":100,"height":40,"material":"frosted","layout":"none","children":[
              {"type":"text","id":"label","x":10,"y":10,"width":40,"height":20,"textGrowth":"fixed-width-height","content":"Hi","fill":"#ffffff"}]}]}"##,
        );
        // A picture that is black, with a light stripe under part of the label.
        let (w, h) = (100, 40);
        let mut rgba = vec![0_u8; w * h * 4];
        for (index, pixel) in rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let light = (20..30).contains(&(index % w));
            pixel.copy_from_slice(if light { &[230, 230, 230, 255] } else { &[0, 0, 0, 255] });
        }
        let survey =
            Survey { picture: crate::export::Picture { w, h, rgba }, origin: pen_doc::geom::Point::new(0.0, 0.0), scale: 1.0, materials: Vec::new() };
        let scenes: Vec<&SceneNode> = editor.scene.iter().collect();
        let answer = check(&scenes, Vars::new(&editor.doc), Some(&survey));
        let said = answer["problems"][0].as_str().unwrap();
        assert!(said.contains("(label)") && said.contains("the lightest part") && said.contains("needs 4.5:1"), "{said}");
        assert_eq!(answer["labelsMeasured"], 1);
        assert_eq!(answer["ok"], false);
        assert!(answer["lowestContrast"].as_f64().unwrap() < 1.5);
    }

    #[test]
    fn a_glass_group_needs_a_frame_and_a_spacing() {
        assert_eq!(group_problem(Some(&Kind::Frame), &json!(12)), None);
        assert_eq!(group_problem(None, &Value::Null), None);
        assert!(group_problem(Some(&Kind::Rectangle), &json!(12)).unwrap().contains("belongs on a frame"));
        assert!(group_problem(Some(&Kind::Frame), &json!("wide")).unwrap().contains("from 0 to 200"));
    }
}
