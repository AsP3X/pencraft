//! A look at a design for what makes it read as made by a model on its defaults instead of designed: the
//! stock palette, the stock grid, text nobody can read, sizes and spacings that follow no system.
//!
//! Everything here is counted from the layers, not judged: each note names what was found and where, so the
//! model that made the design can put it right in its next step. The notes go out with every `screenshot`,
//! the moment the model looks at its work.

use pen_doc::color::Rgba;
use pen_doc::layout;
use pen_doc::model::{Corners, Kind, Padding, Paint, StrokeWidth};
use pen_doc::scene::SceneNode;
use pen_doc::value::{Flag, Num};
use pen_doc::vars::Vars;
use serde_json::Value;

/// How many notes one review carries.
const MOST_NOTES: usize = 10;
/// How many layers one note names.
const MOST_NAMED: usize = 4;
/// The least contrast between body text and its background (WCAG AA), and for large text.
// Source: WCAG 2.1, success criterion 1.4.3.
const CONTRAST: f32 = 4.5;
/// See [`CONTRAST`].
const CONTRAST_LARGE: f32 = 3.0;
/// From this size on text counts as large for contrast.
const LARGE_TEXT: f32 = 24.0;
/// Text under this size needs more contrast than the least WCAG asks for: thin strokes of a few pixels fade
/// long before 4.5:1, most of all light on dark.
const SMALL_TEXT: f32 = 13.0;
/// See [`SMALL_TEXT`].
const CONTRAST_SMALL: f32 = 6.0;
/// Text smaller than this is hard to read on a screen.
const SMALLEST_TEXT: f32 = 11.0;
/// How many font sizes, corner radii and spacings a design's scale may have before it stops being one.
const MOST_SIZES: usize = 7;
/// See [`MOST_SIZES`].
const MOST_RADII: usize = 4;
/// See [`MOST_SIZES`].
const MOST_SPACINGS: usize = 8;
/// The fewest text layers that make a layer a design with a hierarchy to speak of.
const A_DESIGN: usize = 6;
/// The most characters a line of running text should have.
const LONGEST_LINE: f32 = 80.0;
/// A fill is an accent (not a grey) from this saturation on, and a colour that is neither near black nor white.
const ACCENT_SATURATION: f32 = 0.35;
/// How many colours may be written out as hex before they should be variables.
const MOST_RAW_COLOURS: usize = 8;
/// The families a model reaches for when it chooses none.
const DEFAULT_FAMILIES: [&str; 9] = ["inter", "arial", "roboto", "helvetica", "helvetica neue", "segoe ui", "open sans", "system-ui", "sans-serif"];
/// The families a model reaches for next, once the ones above are ruled out.
// Source: the Impeccable skill's list of training-data defaults (reference/new-work.md, "Commit the world").
const NEXT_FAMILIES: [&str; 15] = [
    "fraunces",
    "playfair display",
    "cormorant",
    "cormorant garamond",
    "lora",
    "newsreader",
    "syne",
    "space grotesk",
    "space mono",
    "ibm plex sans",
    "ibm plex mono",
    "dm sans",
    "dm serif display",
    "outfit",
    "plus jakarta sans",
];
/// The tightest letter spacing a headline takes, as a part of its size.
// Source: the Impeccable craft floor ("tracking floor -0.04em").
const TIGHTEST_SPACING: f32 = -0.04;
/// A figure is this many times its label's size, or more, in the stock row of statistics.
const A_FIGURE: f32 = 1.8;
/// Words that sell instead of saying something.
const SALES_WORDS: [&str; 12] = [
    "unlock",
    "elevate",
    "supercharge",
    "seamless",
    "effortless",
    "revolutioni",
    "next-level",
    "all-in-one",
    "game-chang",
    "cutting-edge",
    "empower",
    "unleash",
];
/// Copy that stands in for copy.
const PLACEHOLDERS: [&str; 10] =
    ["lorem ipsum", "john doe", "jane doe", "acme", "example.com", "your text here", "card title", "feature one", "feature 1", "placeholder"];

/// One text layer, as far as the review needs it.
struct Text {
    /// Its name and id, as a note names it.
    label: String,
    family: String,
    size: f32,
    weight: f32,
    /// Its letter spacing, in the units of its size.
    spacing: f32,
    colour: Option<Rgba>,
    /// The covering colour behind it, when there is a single one.
    behind: Option<Rgba>,
    centred: bool,
    gradient: bool,
    content: String,
    /// The width it wraps at, when it wraps.
    wraps_at: Option<f32>,
    /// Where its box starts and ends across the page, and whether its lines start at the left of the box.
    left: f32,
    right: f32,
    starts_left: bool,
    /// Where its box starts down the page; `None` for the label of a small box of its own (a button, a chip,
    /// a tab), which that box places and no column does.
    top: Option<f32>,
}

/// What was counted in the layers looked at.
#[derive(Default)]
struct Seen {
    texts: Vec<Text>,
    /// Every solid colour used, with whether it was written out as hex.
    colours: Vec<(Rgba, bool)>,
    /// The gradients' colours, each with the label of its layer.
    gradients: Vec<(String, Vec<Rgba>)>,
    spacings: Vec<f32>,
    radii: Vec<f32>,
    /// Layers that stick out of the frame that lays them out.
    spilling: Vec<String>,
    /// The deepest run of cards inside cards, and the innermost one.
    nested: (usize, String),
    /// Frames with an outline and a shadow at once.
    doubled: Vec<String>,
    /// Frames that hold three or four cards of the same build side by side.
    grids: Vec<String>,
    /// Small spaced capitals right above a much larger line.
    eyebrows: usize,
    blurs: usize,
    /// Cards with a thick stroke on one side only.
    stripes: Vec<String>,
    /// Lists whose rows have a part that lines up on neither edge from row to row.
    ragged: Vec<String>,
    /// Ends that are left loose: a list ending right on a bar, a line under a last row.
    loose: Vec<String>,
    /// Buttons and chips whose label has hardly any room in them, or that sit right against a line of text.
    squeezed: Vec<String>,
    /// Layers with a coloured shadow that has no offset: a glow.
    glows: Vec<String>,
    /// Layers with a shadow that has an offset and no blur: a block shadow.
    blocks: Vec<String>,
    /// Frames that hold three or four big figures, each over a small label, side by side.
    stats: Vec<String>,
}

fn label(scene: &SceneNode) -> String {
    format!("\"{}\" ({})", scene.node.display_name(), scene.id)
}

fn number(scene: &SceneNode, num: Option<&Num>, vars: Vars<'_>) -> Option<f32> {
    num.and_then(|num| vars.number(num, &scene.theme))
}

/// The paints of a fill or stroke as (colour, written out as hex) for solid ones and colour lists for gradients.
fn paints(scene: &SceneNode, paints: Option<&pen_doc::model::Paints>, vars: Vars<'_>) -> (Vec<(Rgba, bool)>, Vec<Vec<Rgba>>) {
    let (mut solid, mut gradients) = (Vec::new(), Vec::new());
    for paint in paints.map(pen_doc::model::Paints::as_slice).unwrap_or_default() {
        match paint {
            Paint::Solid(text) => solid.extend(vars.color(text, &scene.theme).map(|colour| (colour, text.starts_with('#')))),
            Paint::Object(object) => match object.get("type").and_then(Value::as_str) {
                Some("color") => {
                    let text = object.get("color").and_then(Value::as_str).unwrap_or_default();
                    solid.extend(vars.color(text, &scene.theme).map(|colour| (colour, text.starts_with('#'))));
                }
                Some("gradient") => {
                    let stops = object.get("colors").and_then(Value::as_array).into_iter().flatten();
                    gradients.push(
                        stops
                            .filter_map(|stop| stop.get("color").and_then(Value::as_str))
                            .filter_map(|text| vars.color(text, &scene.theme))
                            .collect(),
                    );
                }
                _ => {}
            },
        }
    }
    (solid, gradients)
}

/// A colour's hue in degrees and its saturation and lightness, 0 to 1.
fn hsl(colour: Rgba) -> (f32, f32, f32) {
    let (r, g, b) = (f32::from(colour.r) / 255.0, f32::from(colour.g) / 255.0, f32::from(colour.b) / 255.0);
    let (most, least) = (r.max(g).max(b), r.min(g).min(b));
    let (light, spread) = (f32::midpoint(most, least), most - least);
    if spread < f32::EPSILON {
        return (0.0, 0.0, light);
    }
    let saturation = spread / (1.0 - (2.0 * light - 1.0).abs()).max(f32::EPSILON);
    let hue = if (most - r).abs() < f32::EPSILON {
        ((g - b) / spread).rem_euclid(6.0)
    } else if (most - g).abs() < f32::EPSILON {
        (b - r) / spread + 2.0
    } else {
        (r - g) / spread + 4.0
    };
    (hue * 60.0, saturation.min(1.0), light)
}

/// Whether a colour has a hue to speak of, however transparent it is: saturated, and neither nearly black nor
/// nearly white.
fn vivid(colour: Rgba) -> bool {
    let (_, saturation, light) = hsl(colour);
    saturation >= ACCENT_SATURATION && (0.15..=0.9).contains(&light)
}

/// Whether a colour is an accent: vivid, and covering.
fn accent(colour: Rgba) -> bool {
    colour.a >= 128 && vivid(colour)
}

/// Whether a colour is a covering mid grey: no hue, and neither a white nor a black.
fn grey(colour: Rgba) -> bool {
    let (_, saturation, light) = hsl(colour);
    colour.a >= 250 && saturation < 0.12 && (0.3..=0.85).contains(&light)
}

/// Whether a colour is the violet to indigo a model reaches for by default.
fn violet(colour: Rgba) -> bool {
    accent(colour) && (235.0..=295.0).contains(&hsl(colour).0)
}

/// `over` drawn on `under`, as one opaque colour.
fn blend(over: Rgba, under: Rgba) -> Rgba {
    Rgba { a: 255, ..under.lerp(Rgba { a: 255, ..over }, f32::from(over.a) / 255.0) }
}

/// The effects of a layer of one kind, inner shadows left out.
fn effects<'a>(scene: &'a SceneNode, kind: &'a str) -> impl Iterator<Item = &'a Value> {
    let list: &[Value] = match &scene.node.effect {
        Some(Value::Array(list)) => list,
        Some(effect) => std::slice::from_ref(effect),
        None => &[],
    };
    list.iter().filter(move |effect| {
        effect.get("type").and_then(Value::as_str) == Some(kind) && effect.get("shadowType").and_then(Value::as_str) != Some("inner")
    })
}

fn has_effect(scene: &SceneNode, kind: &str) -> bool {
    effects(scene, kind).next().is_some()
}

/// Notes the shadows of a layer that are no depth: a coloured glow, a block without blur.
// Source: the Impeccable craft floor ("shadows carry an offset and a soft blur; a zero-offset colored halo is
// decoration", and hard offset shadows as a costume outside a neo-brutalist design).
fn shadows(scene: &SceneNode, vars: Vars<'_>, seen: &mut Seen) {
    for shadow in effects(scene, "shadow") {
        let along = |axis: &str| shadow.get("offset").and_then(|offset| offset.get(axis)).and_then(Value::as_f64).unwrap_or(0.0);
        let moved = along("x").abs() > 0.5 || along("y").abs() > 0.5;
        let blur = shadow.get("blur").and_then(Value::as_f64).unwrap_or(0.0);
        let colour = shadow.get("color").and_then(Value::as_str).and_then(|text| vars.color(text, &scene.theme));
        if !moved && blur > 0.5 && colour.is_some_and(vivid) {
            seen.glows.push(label(scene));
        } else if moved && blur < 0.5 {
            seen.blocks.push(label(scene));
        }
    }
}

/// Whether a frame is a card: a box of its own (outline, or a fill other than what is behind it) with rounded
/// corners or an outline, large enough to hold content rather than be a button or a tag.
fn is_card(scene: &SceneNode, fill: Option<Rgba>, behind: Option<Rgba>, vars: Vars<'_>) -> bool {
    let rounded = match &scene.node.corner_radius {
        Some(Corners::All(radius)) => vars.number(radius, &scene.theme).is_some_and(|radius| radius > 0.0),
        Some(Corners::Each(_)) => true,
        None => false,
    };
    let outlined = scene.node.stroke.is_some();
    let own_fill = fill.is_some() && fill != behind;
    scene.node.kind == Kind::Frame
        && !scene.children.is_empty()
        && (outlined || own_fill)
        && (rounded || outlined)
        && scene.local.h >= 64.0
        && scene.local.w >= 120.0
}

/// How a layer is built, to tell cards of the same build: the kinds of everything in it, in order.
fn build(scene: &SceneNode, out: &mut String) {
    out.push(match scene.node.kind {
        Kind::Text => 't',
        Kind::Icon => 'i',
        Kind::Frame | Kind::Group => 'f',
        _ => 's',
    });
    if !scene.children.is_empty() {
        out.push('(');
        scene.children.iter().for_each(|child| build(child, out));
        out.push(')');
    }
}

/// Notes the spacing, corners, effects and side strokes of one layer.
fn measures(scene: &SceneNode, vars: Vars<'_>, seen: &mut Seen) {
    let node = &scene.node;
    if !scene.children.is_empty() {
        seen.spacings.extend(number(scene, node.gap.as_ref(), vars).filter(|gap| *gap > 0.0));
        let sides: &[Num] = match &node.padding {
            Some(Padding::All(all)) => std::slice::from_ref(all),
            Some(Padding::Sides(sides)) => sides,
            None => &[],
        };
        seen.spacings.extend(sides.iter().filter_map(|side| vars.number(side, &scene.theme)).filter(|side| *side > 0.0));
    }
    if let Some(Corners::All(radius)) = &node.corner_radius {
        // A radius of half the height or more is a pill, whatever its number.
        seen.radii.extend(vars.number(radius, &scene.theme).filter(|radius| *radius > 0.0 && *radius < scene.local.h.min(scene.local.w) / 2.0));
    }
    if has_effect(scene, "background_blur") {
        seen.blurs += 1;
    }
    shadows(scene, vars, seen);
    if node.stroke.is_some() && has_effect(scene, "shadow") && node.kind == Kind::Frame {
        seen.doubled.push(label(scene));
    }
    if let Some(StrokeWidth::Sides(sides)) = &node.stroke_width {
        let wide: Vec<&String> = sides.iter().filter(|(_, width)| width.as_f64().is_some_and(|width| width > 0.0)).map(|(side, _)| side).collect();
        if wide.len() == 1 && sides.get(wide[0]).and_then(Value::as_f64).is_some_and(|width| width >= 3.0) && scene.local.h >= 40.0 {
            seen.stripes.push(label(scene));
        }
    }
}

/// Notes one text layer.
fn text(scene: &SceneNode, parent: Option<&SceneNode>, behind: Option<Rgba>, vars: Vars<'_>, seen: &mut Seen) {
    let node = &scene.node;
    let style = layout::text_style(node, vars, &scene.theme);
    let (solid, gradients) = paints(scene, node.fill.as_ref(), vars);
    let in_centred_column =
        parent.is_some_and(|parent| parent.node.layout.as_deref() == Some("vertical") && parent.node.align_items.as_deref() == Some("center"));
    seen.texts.push(Text {
        label: label(scene),
        family: style.family.clone(),
        size: style.size,
        weight: f32::from(style.weight),
        spacing: style.letter_spacing,
        colour: solid.last().map(|(colour, _)| *colour),
        behind,
        centred: node.text_align.as_deref() == Some("center") || in_centred_column,
        gradient: !gradients.is_empty(),
        wraps_at: node.text_growth.as_deref().is_some_and(|growth| growth.starts_with("fixed")).then_some(scene.local.w),
        left: layout::doc_bounds(scene).x,
        right: layout::doc_bounds(scene).x + layout::doc_bounds(scene).w,
        // A text centred by its column starts wherever its width puts it; that is no edge to compare.
        starts_left: !matches!(node.text_align.as_deref(), Some("center" | "right")) && !in_centred_column,
        top: (!parent.is_some_and(|parent| {
            parent.children.len() <= 2 && (parent.node.fill.is_some() || parent.node.stroke.is_some()) && parent.local.h <= 72.0
        }))
        .then(|| layout::doc_bounds(scene).y),
        content: style.content,
    });
}

/// Walks `scene`, counting into `seen`. `behind` is the covering colour under the layer, `cards` how many
/// cards it sits in.
fn look(scene: &SceneNode, parent: Option<&SceneNode>, behind: Option<Rgba>, cards: usize, vars: Vars<'_>, seen: &mut Seen) {
    let node = &scene.node;
    if matches!(&node.enabled, Some(Flag::Value(false))) {
        return;
    }
    let (fills, fill_gradients) = paints(scene, node.fill.as_ref(), vars);
    let (strokes, _) = paints(scene, node.stroke.as_ref(), vars);
    seen.colours.extend(fills.iter().chain(&strokes).filter(|(colour, _)| colour.a >= 128));
    if node.kind == Kind::Text {
        text(scene, parent, behind, vars, seen);
    } else {
        seen.gradients.extend(fill_gradients.iter().map(|colours| (label(scene), colours.clone())));
    }
    measures(scene, vars, seen);
    // Human: a layer placed by its frame's layout should lie inside that frame; one that doesn't was given a
    // size the frame has no room for.
    if let Some(parent) = parent.filter(|parent| parent.node.kind == Kind::Frame && parent.node.layout.as_deref() != Some("none"))
        && node.layout_position.as_deref() != Some("absolute")
    {
        // The room the frame gives is what its padding leaves: a layer in the padding has run over as well.
        let (inner, outer, [top, right, bottom, left]) = (layout::doc_bounds(scene), layout::doc_bounds(parent), padding(parent, vars));
        let turned = parent.node.rotation.is_some() || node.rotation.is_some();
        let over = (inner.x + inner.w - outer.x - outer.w + right)
            .max(outer.x + left - inner.x)
            .max(inner.y + inner.h - outer.y - outer.h + bottom)
            .max(outer.y + top - inner.y);
        if over > 1.5 && !turned {
            seen.spilling.push(format!("{} sticks out of {} by {over:.0}", label(scene), label(parent)));
        }
    }
    overlaps(scene, seen);
    controls(scene, vars, seen);
    crowding(scene, seen);
    trailing_rule(scene, seen);
    columns(scene, seen);
    let fill = fills.last().map(|(colour, _)| *colour).filter(|colour| colour.a >= 128);
    let card = is_card(scene, fill, behind, vars);
    let cards = cards + usize::from(card);
    if card && cards > seen.nested.0 {
        seen.nested = (cards, label(scene));
    }
    grid(scene, seen);
    stats(scene, vars, seen);
    eyebrows(scene, vars, seen);
    // Human: what is behind a layer is the nearest frame around it with a covering fill; a frame filled with a
    // gradient has no single colour, and nothing is said about the contrast of what lies on it.
    let under = match (&node.kind, fill) {
        // What lies on a material is measured on a picture (`check_glass`), not against a colour.
        _ if node.material.is_some() => None,
        (Kind::Frame, Some(colour)) => Some(colour),
        (Kind::Frame, None) if !fill_gradients.is_empty() => None,
        _ => behind,
    };
    for child in &scene.children {
        look(child, Some(scene), under, cards, vars, seen);
    }
}

/// Whether a frame cuts its children off at its edge.
fn clips(scene: &SceneNode) -> bool {
    matches!(&scene.node.clip, Some(Flag::Value(true)))
}

/// Notes what reaches over the edge of a page (a filled frame at the top) that doesn't cut it off: a shape
/// meant to run off the sheet shows beyond it, in the picture and in every export.
fn bleed(page: &SceneNode, seen: &mut Seen) {
    fn furthest<'a>(scene: &'a SceneNode, page: &pen_doc::geom::Rect, worst: &mut Option<(f32, &'a SceneNode)>) {
        for child in &scene.children {
            if matches!(&child.node.enabled, Some(Flag::Value(false))) {
                continue;
            }
            let inner = layout::doc_bounds(child);
            let over = (inner.x + inner.w - page.x - page.w).max(page.x - inner.x).max(inner.y + inner.h - page.y - page.h).max(page.y - inner.y);
            if over > 1.5 && worst.is_none_or(|(most, _)| over > most) {
                *worst = Some((over, child));
            }
            // What a clipping frame holds is cut off there and can't reach the page's edge.
            if !clips(child) {
                furthest(child, page, worst);
            }
        }
    }
    if page.node.kind != Kind::Frame || page.node.fill.is_none() || clips(page) {
        return;
    }
    let mut worst = None;
    furthest(page, &layout::doc_bounds(page), &mut worst);
    if let Some((over, layer)) = worst {
        seen.spilling.push(format!(
            "{} reaches {over:.0} over the edge of {}, which doesn't cut it off, so it shows beyond the page (set `clip` to true on {} if the edge should cut it)",
            label(layer),
            label(page),
            label(page)
        ));
    }
}

/// A frame's padding as top, right, bottom, left.
fn padding(scene: &SceneNode, vars: Vars<'_>) -> [f32; 4] {
    let side = |num: &Num| vars.number(num, &scene.theme).unwrap_or(0.0);
    match &scene.node.padding {
        Some(Padding::All(all)) => [side(all); 4],
        Some(Padding::Sides(sides)) => match sides.as_slice() {
            [vertical, horizontal] => [side(vertical), side(horizontal), side(vertical), side(horizontal)],
            [top, right, bottom, left] => [side(top), side(right), side(bottom), side(left)],
            _ => [0.0; 4],
        },
        None => [0.0; 4],
    }
}

/// Notes text layers that a layout placed on top of their neighbours: a negative gap, or a line height so
/// small that one line of text runs into the next layer.
fn overlaps(scene: &SceneNode, seen: &mut Seen) {
    if !matches!(scene.node.kind, Kind::Frame | Kind::Group) {
        return;
    }
    // Human: in a layout neighbours are compared (a negative gap, a line height too small); where layers are
    // placed by hand, every text is compared with every other text beside it, since two texts set at nearly
    // the same place are the usual slip there.
    let by_hand = scene.node.kind == Kind::Group || scene.node.layout.as_deref() == Some("none");
    let placed: Vec<&SceneNode> = scene
        .children
        .iter()
        .filter(|child| child.node.layout_position.as_deref() != Some("absolute") && !matches!(&child.node.enabled, Some(Flag::Value(false))))
        .filter(|child| !by_hand || child.node.kind == Kind::Text)
        .collect();
    let mut pairs: Vec<(&SceneNode, &SceneNode)> = Vec::new();
    for (at, first) in placed.iter().enumerate() {
        let others = if by_hand { &placed[at + 1..] } else { &placed[(at + 1).min(placed.len())..(at + 2).min(placed.len())] };
        pairs.extend(others.iter().map(|second| (*first, *second)));
    }
    for (first, second) in pairs.into_iter().take(400) {
        if first.node.kind != Kind::Text && second.node.kind != Kind::Text {
            continue;
        }
        let (a, b) = (layout::doc_bounds(first), layout::doc_bounds(second));
        let (across, down) = ((a.x + a.w).min(b.x + b.w) - a.x.max(b.x), (a.y + a.h).min(b.y + b.h) - a.y.max(b.y));
        // Lines of text have room above and below their letters; boxes that only graze each other there are fine.
        if across > 1.5 && down > a.h.min(b.h) * 0.25 {
            seen.spilling.push(format!("{} lies over {} by {:.0}", label(first), label(second), across.min(down)));
        }
        // Human: two texts set side by side with nothing between them read as one word ("€212.40on last Saturday").
        let between = b.x - (a.x + a.w);
        let both_text = first.node.kind == Kind::Text && second.node.kind == Kind::Text;
        if both_text && !by_hand && down > a.h.min(b.h) * 0.5 && (-1.5..2.0).contains(&between) {
            seen.spilling.push(format!("{} touches {} (give their frame a gap of 4 or more)", label(first), label(second)));
        }
    }
}

/// A label with less than this above and below it inside its button or chip looks squeezed in, and so does a
/// button or chip with less than this between it and a line of text above or below it.
const ROOM_OF_A_CONTROL: f32 = 4.0;
/// The tallest box that is still taken for a button or a chip.
const TALLEST_CONTROL: f32 = 64.0;

/// Whether a layer is shown.
fn shown(scene: &SceneNode) -> bool {
    !matches!(&scene.node.enabled, Some(Flag::Value(false)))
}

/// The label of a small control, when `scene` is one: a filled or outlined box that holds one text and at most
/// icons beside it (a button, a chip, a pill). A row with a line along one side is not one.
fn control_label(scene: &SceneNode) -> Option<&SceneNode> {
    let node = &scene.node;
    let boxed = node.fill.is_some() || (node.stroke.is_some() && !matches!(&node.stroke_width, Some(StrokeWidth::Sides(_))));
    let parts: Vec<&SceneNode> = scene.children.iter().filter(|child| shown(child)).collect();
    let plain = parts.iter().all(|part| matches!(part.node.kind, Kind::Text | Kind::Icon));
    let mut texts = parts.iter().filter(|part| part.node.kind == Kind::Text);
    let (label, more) = (texts.next(), texts.next());
    (node.kind == Kind::Frame && boxed && plain && parts.len() <= 3 && more.is_none() && scene.local.h <= TALLEST_CONTROL)
        .then_some(label.copied())
        .flatten()
}

/// The controls (`wanted`) or the texts outside of controls in `scene`, itself included, two levels down.
fn parts_of<'a>(scene: &'a SceneNode, wanted: bool, depth: usize, out: &mut Vec<&'a SceneNode>) {
    if !shown(scene) {
        return;
    }
    let control = control_label(scene).is_some();
    if control == wanted && (control || scene.node.kind == Kind::Text) {
        out.push(scene);
    }
    if !control && depth > 0 {
        for child in &scene.children {
            parts_of(child, wanted, depth - 1, out);
        }
    }
}

/// Notes buttons and chips that are squeezed: a label with hardly any room above and below it in its box, and
/// a box set right under or over a line of text in the next row of a column.
fn controls(scene: &SceneNode, vars: Vars<'_>, seen: &mut Seen) {
    // Human: a badge in small type may be tight; from reading size on, a label needs air to read as a control.
    if let Some(text) = control_label(scene).filter(|text| layout::text_style(&text.node, vars, &text.theme).size >= SMALL_TEXT) {
        let (outer, inner) = (layout::doc_bounds(scene), layout::doc_bounds(text));
        let room = (inner.y - outer.y).min(outer.y + outer.h - inner.y - inner.h);
        if room < ROOM_OF_A_CONTROL - 0.5 {
            seen.squeezed.push(format!("{} leaves its label {:.0} above and below", label(scene), room.max(0.0)));
        }
    }
    if scene.node.kind != Kind::Frame || scene.node.layout.as_deref() != Some("vertical") {
        return;
    }
    let rows: Vec<&SceneNode> = scene.children.iter().filter(|row| row.node.layout_position.as_deref() != Some("absolute")).collect();
    for pair in rows.windows(2) {
        for (texts_in, controls_in) in [(0, 1), (1, 0)] {
            let (mut texts, mut boxes) = (Vec::new(), Vec::new());
            parts_of(pair[texts_in], false, 2, &mut texts);
            parts_of(pair[controls_in], true, 2, &mut boxes);
            for (text, control) in texts.iter().flat_map(|text| boxes.iter().map(move |control| (*text, *control))).take(200) {
                let (a, b) = (layout::doc_bounds(text), layout::doc_bounds(control));
                let across = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
                let between = if texts_in == 0 { b.y - a.y - a.h } else { a.y - b.y - b.h };
                if across > 1.5 && (-1.5..ROOM_OF_A_CONTROL - 0.5).contains(&between) {
                    seen.squeezed.push(format!("{} sits {:.0} from {}", label(control), between.max(0.0), label(text)));
                }
            }
        }
    }
}

/// Content that ends closer than this above a bar below it looks cut short.
const ROOM_ABOVE_A_BAR: f32 = 16.0;

/// The bottom edge of the lowest text, icon or rule in `scene`, on the page: where its content ends.
fn lowest_text(scene: &SceneNode) -> Option<f32> {
    if matches!(&scene.node.enabled, Some(Flag::Value(false))) {
        return None;
    }
    // A thin rectangle is a rule drawn between rows; one under the last row ends the list like its text does.
    let ends = match scene.node.kind {
        Kind::Text | Kind::Icon | Kind::Line => true,
        Kind::Rectangle => scene.local.h <= 2.0,
        // A row with a line along its bottom ends where that line is, whatever padding its text has.
        Kind::Frame => {
            scene.node.stroke.is_some()
                && matches!(&scene.node.stroke_width, Some(StrokeWidth::Sides(sides)) if sides.get("bottom").and_then(Value::as_f64).is_some_and(|width| width > 0.0))
        }
        _ => false,
    };
    let own = ends.then(|| {
        let bounds = layout::doc_bounds(scene);
        bounds.y + bounds.h
    });
    scene.children.iter().filter_map(lowest_text).chain(own).reduce(f32::max)
}

/// Notes content whose last line of text ends right on top of a bar below it: a list running into a tab bar
/// or a footer, with no room and no sign that it goes on.
fn crowding(scene: &SceneNode, seen: &mut Seen) {
    if !matches!(scene.node.kind, Kind::Frame | Kind::Group) {
        return;
    }
    let page = layout::doc_bounds(scene);
    for (at, above) in scene.children.iter().enumerate() {
        let Some(bottom) = lowest_text(above) else { continue };
        let top = layout::doc_bounds(above);
        for bar in scene.children.iter().skip(at + 1) {
            let bounds = layout::doc_bounds(bar);
            // A bar is a filled frame across (nearly) the whole width along the bottom of its page (a tab bar, a
            // footer); a panel further up that follows a status bar is not one. A row that only has a line
            // along its bottom is the next row of a list, not a bar.
            let at_the_bottom = (bounds.y + bounds.h - page.y - page.h).abs() <= 2.0 && bounds.h <= page.h * 0.3;
            let is_bar = bar.node.kind == Kind::Frame && bar.node.fill.is_some() && bounds.w >= page.w * 0.9 && at_the_bottom;
            // Human: content that runs on behind the bar is cut off where the bar begins, in the middle of a row:
            // in a still picture that reads as a mistake, as does content that ends within a few units of it.
            let visible = bottom.min(top.y + top.h);
            if is_bar && bounds.y >= top.y + top.h * 0.5 && (-1.5..ROOM_ABOVE_A_BAR).contains(&(bounds.y - visible)) {
                seen.loose.push(format!(
                    "{} runs up to {:.0} above {}: show whole rows only and leave 16 or more below the last one (fewer rows, or a \"See all\" line)",
                    label(above),
                    (bounds.y - bottom).max(0.0),
                    label(bar)
                ));
                return;
            }
        }
    }
}

/// Whether a frame has a line along its bottom only.
fn ruled_below(scene: &SceneNode) -> bool {
    scene.node.kind == Kind::Frame
        && scene.node.stroke.is_some()
        && matches!(&scene.node.stroke_width, Some(StrokeWidth::Sides(sides)) if sides.get("bottom").and_then(Value::as_f64).is_some_and(|width| width > 0.0))
}

/// Notes a list of three or more rows in which every row, the last one too, has a line along its bottom: the
/// last line divides the list from nothing and hangs under it.
fn trailing_rule(scene: &SceneNode, seen: &mut Seen) {
    let rows = &scene.children;
    let listed =
        scene.node.kind == Kind::Frame && scene.node.layout.as_deref() == Some("vertical") && rows.len() >= 3 && rows.iter().all(ruled_below);
    if let Some(last) = rows.last().filter(|_| listed) {
        seen.loose.push(format!(
            "the last row of {} ({}) has a line under it with no row below: lines go between rows, so take the stroke off the last one",
            label(scene),
            label(last)
        ));
    }
}

/// Notes a list whose rows are built alike but have a part that lines up on neither its left nor its right
/// edge nor its middle from row to row: figures of different widths set one after the other.
fn columns(scene: &SceneNode, seen: &mut Seen) {
    if scene.node.kind != Kind::Frame || scene.node.layout.as_deref() != Some("vertical") {
        return;
    }
    let rows: Vec<&SceneNode> = scene
        .children
        .iter()
        .filter(|row| row.node.kind == Kind::Frame && matches!(row.node.layout.as_deref(), None | Some("horizontal")) && row.children.len() >= 2)
        .collect();
    let Some(parts) = rows.first().map(|row| row.children.len()) else { return };
    if rows.len() < 3 || rows.iter().any(|row| row.children.len() != parts) {
        return;
    }
    let spread = |values: Vec<f32>| values.iter().copied().fold(f32::MIN, f32::max) - values.iter().copied().fold(f32::MAX, f32::min);
    for part in 0..parts {
        let boxes: Vec<pen_doc::geom::Rect> = rows.iter().map(|row| layout::doc_bounds(&row.children[part])).collect();
        let (left, right) = (spread(boxes.iter().map(|b| b.x).collect()), spread(boxes.iter().map(|b| b.x + b.w).collect()));
        let middle = spread(boxes.iter().map(|b| b.x + b.w / 2.0).collect());
        if left > 1.5 && right > 1.5 && middle > 1.5 {
            seen.ragged.push(format!("part {} of the rows in {}", part + 1, label(scene)));
            return;
        }
    }
}

/// Notes a row of three or four cards of the same build that each hold an icon and two or more texts.
fn grid(scene: &SceneNode, seen: &mut Seen) {
    if scene.node.kind != Kind::Frame
        || !matches!(scene.node.layout.as_deref(), None | Some("horizontal"))
        || !(3..=4).contains(&scene.children.len())
    {
        return;
    }
    let builds: Vec<String> = scene
        .children
        .iter()
        .map(|child| {
            let mut out = String::new();
            build(child, &mut out);
            out
        })
        .collect();
    let stock = |build: &str| build.contains('i') && build.matches('t').count() >= 2;
    let boxed = scene.children.iter().all(|child| child.node.kind == Kind::Frame && (child.node.fill.is_some() || child.node.stroke.is_some()));
    if builds.windows(2).all(|pair| pair[0] == pair[1]) && builds.first().is_some_and(|build| stock(build)) && boxed {
        seen.grids.push(label(scene));
    }
}

/// Whether a text is a figure: digits with at most a unit beside them ("1,284", "98%", "4.2k", "$12M").
fn is_figure(content: &str) -> bool {
    let content = content.trim();
    content.chars().any(|c| c.is_ascii_digit()) && content.chars().filter(|c| c.is_alphabetic()).count() <= 2 && content.chars().count() <= 10
}

/// Whether a layer holds nothing but a big figure and its small label.
fn is_stat(scene: &SceneNode, vars: Vars<'_>) -> bool {
    let texts: Vec<layout::TextStyle> = scene
        .children
        .iter()
        .filter(|child| child.node.kind == Kind::Text)
        .map(|child| layout::text_style(&child.node, vars, &child.theme))
        .collect();
    let figure = texts.iter().find(|text| is_figure(&text.content));
    (2..=3).contains(&texts.len())
        && texts.len() == scene.children.len()
        && figure.is_some_and(|figure| texts.iter().any(|text| !is_figure(&text.content) && figure.size >= text.size * A_FIGURE))
}

/// Notes a row of three or four big figures, each over a small label.
// Source: the Impeccable craft floor ("the hero-metric template: big number, small label, supporting stats").
fn stats(scene: &SceneNode, vars: Vars<'_>, seen: &mut Seen) {
    if scene.node.kind == Kind::Frame
        && matches!(scene.node.layout.as_deref(), None | Some("horizontal"))
        && (3..=4).contains(&scene.children.len())
        && scene.children.iter().all(|child| is_stat(child, vars))
    {
        seen.stats.push(label(scene));
    }
}

/// Counts small spaced capitals that sit right above a line more than twice their size.
fn eyebrows(scene: &SceneNode, vars: Vars<'_>, seen: &mut Seen) {
    for pair in scene.children.windows(2) {
        if pair[0].node.kind != Kind::Text || pair[1].node.kind != Kind::Text {
            continue;
        }
        let (small, large) = (layout::text_style(&pair[0].node, vars, &pair[0].theme), layout::text_style(&pair[1].node, vars, &pair[1].theme));
        let letters: Vec<char> = small.content.chars().filter(|c| c.is_alphabetic()).collect();
        if letters.len() >= 3 && letters.iter().all(|c| c.is_uppercase()) && small.letter_spacing > 0.0 && large.size >= small.size * 2.0 {
            seen.eyebrows += 1;
        }
    }
}

/// The values of `all` that differ, smallest first.
fn distinct(all: &[f32]) -> Vec<f32> {
    let mut values: Vec<f32> = all.iter().map(|value| (value * 2.0).round() / 2.0).collect();
    values.sort_by(f32::total_cmp);
    values.dedup();
    values
}

fn listed(values: &[f32]) -> String {
    values.iter().map(|value| format!("{value}")).collect::<Vec<_>>().join(", ")
}

/// The first few of `labels`, and how many more there are.
fn named<'a>(labels: impl Iterator<Item = &'a str>) -> String {
    let all: Vec<&str> = labels.collect();
    let more = if all.len() > MOST_NAMED { format!(" and {} more", all.len() - MOST_NAMED) } else { String::new() };
    format!("{}{more}", all.iter().take(MOST_NAMED).copied().collect::<Vec<_>>().join(", "))
}

fn is_emoji(c: char) -> bool {
    matches!(u32::from(c), 0x1F300..=0x1FAFF | 0x2600..=0x27BF | 0x2B50..=0x2B55 | 0xFE0F)
}

/// Notes on what can't be read, isn't installed or doesn't fit: mistakes, whatever the design is meant to be.
fn mistakes(seen: &Seen, installed: &dyn Fn(&str) -> Option<String>, out: &mut Vec<String>) {
    let mut families: Vec<&str> = seen.texts.iter().map(|text| text.family.as_str()).collect();
    families.sort_unstable();
    families.dedup();
    for family in families {
        if let Some(shown) = installed(family) {
            out.push(format!(
                "The font \"{family}\" isn't installed on this computer, so its text is drawn in {shown}. Choose a family from the installed ones (get_guide lists them)."
            ));
        }
    }
    let faint: Vec<String> = seen
        .texts
        .iter()
        .filter_map(|text| {
            let (colour, behind) = (text.colour?, text.behind?);
            let ratio = blend(colour, behind).contrast(behind);
            let least = if text.size >= LARGE_TEXT {
                CONTRAST_LARGE
            } else if text.size < SMALL_TEXT {
                CONTRAST_SMALL
            } else {
                CONTRAST
            };
            (ratio < least).then(|| format!("{} {ratio:.1}:1", text.label))
        })
        .collect();
    if !faint.is_empty() {
        out.push(format!(
            "Text too faint against its background (it needs {CONTRAST}:1, {CONTRAST_LARGE}:1 from {LARGE_TEXT} px, {CONTRAST_SMALL}:1 under {SMALL_TEXT} px): {}. Darken or lighten the text, not the size.",
            named(faint.iter().map(String::as_str))
        ));
    }
    if !seen.spilling.is_empty() {
        out.push(format!(
            "Layers that don't fit where their frame's layout puts them: {}. Give them \"fill_container\" or a smaller size, let the text wrap (textGrowth \"fixed-width\"), or give the frame a gap that isn't negative.",
            named(seen.spilling.iter().map(String::as_str))
        ));
    }
    if !seen.loose.is_empty() {
        out.push(format!("Loose ends: {}.", named(seen.loose.iter().map(String::as_str))));
    }
    if !seen.squeezed.is_empty() {
        out.push(format!(
            "Squeezed controls: {}. The label of a button or chip needs 4 or more above and below it (8 to 12 in a button; on a phone whatever is tapped is about 44 high), and a button or chip stands 8 or more clear of the text above and below it.",
            named(seen.squeezed.iter().map(String::as_str))
        ));
    }
    if !seen.ragged.is_empty() {
        out.push(format!(
            "Rows that don't form columns: {} lines up on neither edge from row to row. Give that part one fixed width in every row (with textAlign \"right\" for figures), or let the part before it fill the row (width \"fill_container\"), so the eye can run down a straight edge.",
            named(seen.ragged.iter().map(String::as_str))
        ));
    }
    let tiny: Vec<&str> = seen.texts.iter().filter(|text| text.size < SMALLEST_TEXT).map(|text| text.label.as_str()).collect();
    if !tiny.is_empty() {
        out.push(format!("Text under {SMALLEST_TEXT} px is hard to read: {}.", named(tiny.into_iter())));
    }
}

/// Notes texts that nearly share an edge with others and don't: a few units off a line that three or more
/// texts start or end on. The eye takes "nearly aligned" for a mistake, where a clear step reads as meant.
fn near_misses(texts: &[Text], out: &mut Vec<String>) {
    // Human: a column is made of texts set alike; a tab label that happens to end near the amounts' edge is not in it.
    let sharing = |edge: f32, size: f32, of: &dyn Fn(&Text) -> f32| {
        texts.iter().filter(|text| (of(text) - edge).abs() <= 0.75 && (text.size - size).abs() <= 0.5).count()
    };
    // Human: a text is in a column only if it lies between the first and the last of the texts on that edge;
    // a link in the header that happens to end near the amounts further down is not.
    let within = |text: &Text, edge: f32, of: &dyn Fn(&Text) -> f32| {
        let tops =
            texts.iter().filter(|other| (of(other) - edge).abs() <= 0.75 && (other.size - text.size).abs() <= 0.5).filter_map(|other| other.top);
        let (first, last) = tops.fold((f32::MAX, f32::MIN), |(first, last), top| (first.min(top), last.max(top)));
        text.top.is_some_and(|top| top > first && top < last)
    };
    let left: &dyn Fn(&Text) -> f32 = &|text| text.left;
    let right: &dyn Fn(&Text) -> f32 = &|text| text.right;
    let mut off: Vec<String> = Vec::new();
    for text in texts {
        // A start a few units beside a shared left edge (a wider step is an indent, which is meant).
        let near_left =
            texts.iter().map(|other| other.left).find(|edge| sharing(*edge, text.size, left) >= 3 && (1.5..=6.0).contains(&(text.left - edge).abs()));
        if let Some(edge) = near_left.filter(|edge| text.starts_left && sharing(text.left, text.size, left) == 1 && within(text, *edge, left)) {
            off.push(format!(
                "{} starts {:.0} beside the edge {} others start on",
                text.label,
                (text.left - edge).abs(),
                sharing(edge, text.size, left)
            ));
            continue;
        }
        // An end a little short of or past a shared right edge, for text that isn't set from a left edge it shares.
        let near_right = texts
            .iter()
            .map(|other| other.right)
            .find(|edge| sharing(*edge, text.size, right) >= 3 && (1.5..=16.0).contains(&(text.right - edge).abs()));
        if let Some(edge) = near_right.filter(|edge| {
            text.wraps_at.is_none()
                && !text.centred
                && sharing(text.right, text.size, right) == 1
                && sharing(text.left, text.size, left) == 1
                && within(text, *edge, right)
        }) {
            off.push(format!("{} ends {:.0} off the edge {} others end on", text.label, (text.right - edge).abs(), sharing(edge, text.size, right)));
        }
    }
    if !off.is_empty() {
        out.push(format!(
            "Nearly aligned, which reads as a mistake: {}. Put each on the shared edge (figures in a column of one width with textAlign \"right\"), or move it clearly away.",
            named(off.iter().map(String::as_str))
        ));
    }
}

/// Notes on the type: the family nobody chose, a scale that isn't one, a hierarchy that is flat.
fn type_notes(seen: &Seen, out: &mut Vec<String>) {
    let texts = &seen.texts;
    if texts.len() < A_DESIGN {
        return;
    }
    near_misses(texts, out);
    let mut families: Vec<String> = texts.iter().map(|text| text.family.to_lowercase()).filter(|family| !family.contains("mono")).collect();
    families.sort_unstable();
    families.dedup();
    if families.len() == 1 && DEFAULT_FAMILIES.contains(&families[0].as_str()) {
        out.push(format!(
            "All text is set in {}, the face a model uses when it chooses none. Unless the document's own type system says so, pick a typeface for this subject from the installed ones: a display face with character for the headlines, a quiet one for reading.",
            texts[0].family
        ));
    } else if families.len() > 2 {
        out.push(format!("{} font families ({}). Two are enough: one for display, one for text.", families.len(), families.join(", ")));
    }
    let next: Vec<&str> = families.iter().map(String::as_str).filter(|family| NEXT_FAMILIES.contains(family)).collect();
    if !next.is_empty() {
        out.push(format!(
            "{} is among the faces a model lands on once Inter is ruled out. Unless the user or the document's own type system chose it, pick a face from this subject's world out of the installed ones.",
            next.join(", ")
        ));
    }
    let tight: Vec<&str> = texts.iter().filter(|text| text.spacing < text.size * TIGHTEST_SPACING - 0.01).map(|text| text.label.as_str()).collect();
    if !tight.is_empty() {
        out.push(format!(
            "Letters set tighter than 4% of the font size run into each other: {}. Loosen the letter spacing.",
            named(tight.into_iter())
        ));
    }
    let numbered = ["01", "02", "03"]
        .into_iter()
        .filter(|number| texts.iter().any(|text| text.content.trim().trim_end_matches(['.', '/', ' ']) == *number))
        .count();
    if numbered == 3 {
        out.push(
            "Sections are numbered 01, 02, 03. Keep the numbers only where the order itself tells the reader something (steps to follow); otherwise drop them."
                .to_owned(),
        );
    }
    let sizes = distinct(&texts.iter().map(|text| text.size).collect::<Vec<_>>());
    if sizes.len() > MOST_SIZES {
        out.push(format!("{} different font sizes ({}). Set a scale of five to seven sizes and use only those.", sizes.len(), listed(&sizes)));
    }
    let weights = distinct(&texts.iter().map(|text| text.weight).collect::<Vec<_>>());
    let mut colours: Vec<Rgba> = texts.iter().filter_map(|text| text.colour).collect();
    colours.dedup();
    if let (Some(smallest), Some(largest)) = (sizes.first(), sizes.last())
        && largest / smallest < 1.6
        && weights.len() < 2
        && colours.windows(2).all(|pair| pair[0] == pair[1])
    {
        out.push(format!(
            "The type has no hierarchy: every text is between {smallest} and {largest} px, in one weight and one colour. Decide what is read first, second and third, and show it with size (jumps of two or three times, not 1.2), weight and a quieter colour for what matters less."
        ));
    }
    let centred = texts.iter().filter(|text| text.centred).count();
    if centred * 4 > texts.len() * 3 {
        out.push(format!(
            "{centred} of {} texts are centred. Centre only short, lone lines; align the rest to one edge so the eye has a line to read along.",
            texts.len()
        ));
    }
    let long: Vec<&str> = texts
        .iter()
        .filter(|text| text.wraps_at.is_some_and(|width| width / (text.size * 0.5) > LONGEST_LINE && text.content.chars().count() > 160))
        .map(|text| text.label.as_str())
        .collect();
    if !long.is_empty() {
        out.push(format!("Lines of running text longer than about {LONGEST_LINE} characters: {}. Narrow them to 45 to 75.", named(long.into_iter())));
    }
    if seen.eyebrows >= 2 {
        out.push(format!(
            "{} headlines carry a small label in spaced capitals above them: a template's habit. Delete the labels and let each headline carry itself.",
            seen.eyebrows
        ));
    }
}

/// Notes on the colours: the stock gradient, too many accents, colours that should be variables.
fn colour_notes(seen: &Seen, out: &mut Vec<String>) {
    let violet_gradients: Vec<&str> =
        seen.gradients.iter().filter(|(_, colours)| colours.iter().any(|colour| violet(*colour))).map(|(label, _)| label.as_str()).collect();
    if !violet_gradients.is_empty() {
        out.push(format!(
            "A violet or indigo gradient ({}) is the best-known mark of a generated design. Unless the user asked for it, use one solid colour that belongs to the subject.",
            named(violet_gradients.into_iter())
        ));
    } else if seen.gradients.len() > 2 {
        out.push(format!(
            "{} gradients. Keep a gradient for the one place that needs depth; fill the rest with solid colours.",
            seen.gradients.len()
        ));
    }
    let gradient_text: Vec<&str> = seen.texts.iter().filter(|text| text.gradient).map(|text| text.label.as_str()).collect();
    if !gradient_text.is_empty() {
        out.push(format!("Text filled with a gradient ({}): set it in one colour.", named(gradient_text.into_iter())));
    }
    // Source: the Impeccable craft floor ("on colored surfaces tint secondary text from that hue; never gray").
    let greyed: Vec<&str> =
        seen.texts.iter().filter(|text| text.colour.is_some_and(grey) && text.behind.is_some_and(accent)).map(|text| text.label.as_str()).collect();
    if !greyed.is_empty() {
        out.push(format!(
            "Grey text on a coloured surface looks washed out: {}. Use a shade of that colour, or the main text colour made partly transparent.",
            named(greyed.into_iter())
        ));
    }
    if seen.texts.len() < A_DESIGN {
        return;
    }
    // Human: accents are told apart by hue, in twelve steps around the colour wheel; shades of one hue are one accent.
    let mut hues: Vec<(i32, Rgba)> = Vec::new();
    for (colour, _) in seen.colours.iter().filter(|(colour, _)| accent(*colour)) {
        #[expect(clippy::cast_possible_truncation, reason = "a hue, 0 to 360, in steps of 30")]
        let step = (hsl(*colour).0 / 30.0).round() as i32 % 12;
        if !hues.iter().any(|(had, _)| *had == step) {
            hues.push((step, *colour));
        }
    }
    if hues.len() > 3 {
        out.push(format!(
            "{} accent hues ({}). A design holds together with one dominant colour and one sharp accent; keep red, amber and green for states, never for decoration.",
            hues.len(),
            hues.iter().map(|(_, colour)| format!("#{:02x}{:02x}{:02x}", colour.r, colour.g, colour.b)).collect::<Vec<_>>().join(", ")
        ));
    }
    let mut raw: Vec<Rgba> = seen.colours.iter().filter(|(_, written)| *written).map(|(colour, _)| *colour).collect();
    raw.sort_by_key(|colour| (colour.r, colour.g, colour.b, colour.a));
    raw.dedup();
    if raw.len() > MOST_RAW_COLOURS {
        out.push(format!(
            "{} different colours are written out as hex. Make the few the design needs variables (background, surface, text, muted text, line, accent) with set_variables and use those everywhere, so the palette stays one.",
            raw.len()
        ));
    }
}

/// Notes on the build: the stock grid, boxes in boxes, measures off any scale, and copy that isn't copy.
fn build_notes(seen: &Seen, out: &mut Vec<String>) {
    if !seen.grids.is_empty() {
        out.push(format!(
            "{} holds a row of equal cards, each an icon, a title and a line of text: the stock feature grid. Say the same with a layout that has an order of importance (one large and two small, a list with a picture, a table), or drop the boxes and icons.",
            named(seen.grids.iter().map(String::as_str))
        ));
    }
    if !seen.stats.is_empty() {
        out.push(format!(
            "{} holds a row of big figures over small labels: the stock row of statistics. Give the one figure that matters the room and say the rest in a plain line or a table.",
            named(seen.stats.iter().map(String::as_str))
        ));
    }
    if !seen.glows.is_empty() {
        out.push(format!(
            "A coloured glow without an offset ({}) is decoration, not depth. Give the shadow an offset and a soft blur in a dark shade of the ground, or leave it out.",
            named(seen.glows.iter().map(String::as_str))
        ));
    }
    if seen.blocks.len() >= 2 {
        out.push(format!(
            "{} layers have a block shadow without blur ({}). Unless the whole design is meant to be that raw, use a soft shadow, a line or a tint.",
            seen.blocks.len(),
            named(seen.blocks.iter().map(String::as_str))
        ));
    }
    if seen.nested.0 >= 3 {
        out.push(format!(
            "Cards inside cards inside cards (down to {}). Take the inner boxes away and group with space and a line instead.",
            seen.nested.1
        ));
    }
    if seen.doubled.len() >= 2 {
        out.push(format!(
            "{} frames have an outline and a shadow at once ({}). Set a surface apart in one way: a line, a shadow or a tint.",
            seen.doubled.len(),
            named(seen.doubled.iter().map(String::as_str))
        ));
    }
    if !seen.stripes.is_empty() {
        out.push(format!(
            "A thick coloured stroke down one side of a card ({}) is a template's accent. Leave it out.",
            named(seen.stripes.iter().map(String::as_str))
        ));
    }
    if seen.blurs >= 3 {
        out.push(format!("{} layers blur what is behind them. Keep glass for the one layer that floats over content.", seen.blurs));
    }
    let spacings = distinct(&seen.spacings);
    let odd: Vec<f32> = spacings.iter().copied().filter(|value| value.rem_euclid(2.0) > 0.01).collect();
    if spacings.len() > MOST_SPACINGS || odd.len() > 1 {
        out.push(format!(
            "The gaps and paddings follow no scale ({}). Use steps of 4 (4, 8, 12, 16, 24, 32, 48, 64): small ones inside a group, clearly larger ones between groups.",
            listed(&spacings)
        ));
    }
    let radii = distinct(&seen.radii);
    if radii.len() > MOST_RADII {
        out.push(format!("{} different corner radii ({}). Keep one rule: one radius for controls, one for cards, and an inner corner smaller than the one around it.", radii.len(), listed(&radii)));
    }
    copy_notes(seen, out);
}

/// Notes on the words.
fn copy_notes(seen: &Seen, out: &mut Vec<String>) {
    let with = |words: &[&str]| -> Vec<&str> {
        seen.texts.iter().filter(|text| words.iter().any(|word| text.content.to_lowercase().contains(word))).map(|text| text.label.as_str()).collect()
    };
    let emoji: Vec<&str> = seen.texts.iter().filter(|text| text.content.chars().any(is_emoji)).map(|text| text.label.as_str()).collect();
    if !emoji.is_empty() {
        out.push(format!(
            "Emoji in the text ({}). Use an icon layer where a sign is needed, and none where it only decorates.",
            named(emoji.into_iter())
        ));
    }
    let placeholders = with(&PLACEHOLDERS);
    if !placeholders.is_empty() {
        out.push(format!(
            "Stand-in copy ({}). Write what this product would really say: real names, uneven numbers, lengths that differ.",
            named(placeholders.into_iter())
        ));
    }
    let sales = with(&SALES_WORDS);
    if !sales.is_empty() {
        out.push(format!("Sales words that say nothing ({}). Say plainly what the thing does.", named(sales.into_iter())));
    }
}

/// What a look at `layers` finds, most important first; empty when there is nothing to say. `stands_in` gives,
/// for a font family that isn't installed, the family drawn in its place.
#[must_use]
pub fn review(layers: &[&SceneNode], vars: Vars<'_>, stands_in: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
    let mut seen = Seen::default();
    for layer in layers {
        look(layer, None, None, 0, vars, &mut seen);
        bleed(layer, &mut seen);
    }
    let mut out = Vec::new();
    mistakes(&seen, stands_in, &mut out);
    colour_notes(&seen, &mut out);
    build_notes(&seen, &mut out);
    type_notes(&seen, &mut out);
    out.truncate(MOST_NOTES);
    out
}

/// The mistakes in `layers`, whatever the design is meant to be: text too faint to read or too small, layers
/// that don't fit their frame, a font that isn't installed. Told with every change, so they don't pile up.
#[must_use]
pub fn mistakes_in(layers: &[&SceneNode], vars: Vars<'_>, stands_in: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
    let mut seen = Seen::default();
    for layer in layers {
        look(layer, None, None, 0, vars, &mut seen);
        bleed(layer, &mut seen);
    }
    let mut out = Vec::new();
    mistakes(&seen, stands_in, &mut out);
    out
}

/// How many layers `layers` hold, themselves included.
#[must_use]
pub fn size(layers: &[&SceneNode]) -> usize {
    let mut count = 0;
    for layer in layers {
        layer.walk(&mut |_, _| count += 1, 0);
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;

    fn notes(json: &str, missing: &[&str]) -> Vec<String> {
        let mut editor = Editor::new(pen_doc::read(json).unwrap(), None, 1);
        editor.refresh(&|style| (style.size * 0.5 * 10.0, style.size * 1.2));
        let layers: Vec<&SceneNode> = editor.scene.iter().collect();
        review(&layers, Vars::new(&editor.doc), &|family| missing.contains(&family).then(|| "Segoe UI".to_owned()))
    }

    #[test]
    fn the_marks_of_a_generated_design_are_named() {
        let card = |id: &str| {
            format!(
                r##"{{"type":"frame","id":"{id}","name":"Card","layout":"vertical","width":300,"height":200,"padding":23,"gap":13,"cornerRadius":16,"fill":"#ffffff","stroke":"#e5e7eb","effect":{{"type":"shadow","blur":8}},"children":[
                  {{"type":"icon","id":"{id}i","icon":"zap","width":24,"height":24,"fill":"#22c55e"}},
                  {{"type":"text","id":"{id}t","content":"Seamless sync 🚀","fontSize":16,"fill":"#111827"}},
                  {{"type":"text","id":"{id}b","content":"Lorem ipsum dolor sit amet","fontSize":15,"fill":"#e5e7eb"}}]}}"##
            )
        };
        let json = format!(
            r##"{{"version":"2.22","children":[
              {{"type":"frame","id":"page","name":"Page","x":0,"y":0,"width":1200,"layout":"vertical","alignItems":"center","gap":32,"padding":64,"fill":"#ffffff","children":[
                {{"type":"frame","id":"hero","name":"Hero","width":1400,"height":300,"fill":{{"type":"gradient","gradientType":"linear","colors":[{{"color":"#6366f1","position":0}},{{"color":"#a855f7","position":1}}]}}}},
                {{"type":"text","id":"h","name":"Headline","content":"Unlock your potential","fontSize":17,"fill":"#111827"}},
                {{"type":"frame","id":"row","name":"Features","gap":24,"children":[{},{},{}]}}]}}]}}"##,
            card("a"),
            card("b"),
            card("c")
        );
        let said = notes(&json, &["Inter"]);
        let has = |part: &str| said.iter().any(|note| note.contains(part));
        assert!(has("\"Inter\" isn't installed"), "{said:#?}");
        assert!(has("too faint") && has("(ab) 1.2:1"), "{said:#?}");
        assert!(has("\"Hero\" (hero) sticks out of \"Page\" (page) by 164"), "{said:#?}");
        assert!(has("violet or indigo gradient"), "{said:#?}");
        assert!(has("stock feature grid") && has("\"Features\" (row)"), "{said:#?}");
        assert!(has("outline and a shadow at once"), "{said:#?}");
        assert!(has("follow no scale (13, 23, 24, 32, 64)"), "{said:#?}");
        assert!(has("Emoji") && has("Stand-in copy") && has("Sales words"), "{said:#?}");
        assert!(said.len() <= MOST_NOTES);
    }

    #[test]
    fn a_design_with_a_system_gets_no_notes() {
        let json = r##"{"version":"2.22","variables":{"ink":{"type":"color","value":"#1c1917"},"quiet":{"type":"color","value":"#57534e"},"paper":{"type":"color","value":"#f5f5f4"},"accent":{"type":"color","value":"#b45309"}},"children":[
          {"type":"frame","id":"page","x":0,"y":0,"width":900,"layout":"vertical","gap":24,"padding":48,"fill":"$paper","children":[
            {"type":"text","id":"t1","content":"Ferry times, Oban to Mull","fontFamily":"Georgia","fontSize":56,"fontWeight":"700","fill":"$ink"},
            {"type":"text","id":"t2","content":"Winter timetable, valid until 28 March","fontFamily":"Georgia","fontSize":18,"fill":"$quiet"},
            {"type":"frame","id":"list","layout":"vertical","gap":8,"width":"fill_container","children":[
              {"type":"text","id":"r1","content":"07:25  Craignure","fontFamily":"Georgia","fontSize":18,"fill":"$ink"},
              {"type":"text","id":"r2","content":"09:55  Craignure","fontFamily":"Georgia","fontSize":18,"fill":"$ink"},
              {"type":"text","id":"r3","content":"12:10  Craignure, via Lismore","fontFamily":"Georgia","fontSize":18,"fill":"$ink"},
              {"type":"text","id":"r4","content":"Book the 12:10","fontFamily":"Georgia","fontSize":18,"fontWeight":"700","fill":"$accent"}]}]}]}"##;
        assert_eq!(notes(json, &[]), Vec::<String>::new());
        // The same in the face nobody chose, all one size and centred, is told so.
        let plain = json
            .replace("\"fontFamily\":\"Georgia\",", "")
            .replace("\"fontSize\":56,\"fontWeight\":\"700\"", "\"fontSize\":18")
            .replace(",\"fontWeight\":\"700\",\"fill\":\"$accent\"", ",\"fill\":\"$ink\"")
            .replace("$quiet", "$ink")
            .replace("\"gap\":24,", "\"gap\":24,\"alignItems\":\"center\",");
        let said = notes(&plain, &[]);
        assert!(said.iter().any(|note| note.contains("the face a model uses when it chooses none")), "{said:#?}");
        assert!(said.iter().any(|note| note.contains("no hierarchy")), "{said:#?}");
    }

    #[test]
    fn the_newer_marks_are_named_too() {
        let stat = |id: &str, figure: &str, word: &str| {
            format!(
                r##"{{"type":"frame","id":"{id}","layout":"vertical","gap":4,"children":[
                  {{"type":"text","id":"{id}n","content":"{figure}","fontFamily":"Fraunces","fontSize":48,"letterSpacing":-4,"fill":"#1c1917"}},
                  {{"type":"text","id":"{id}l","content":"{word}","fontFamily":"Fraunces","fontSize":14,"fill":"#57534e"}}]}}"##
            )
        };
        let json = format!(
            r##"{{"version":"2.22","children":[
              {{"type":"frame","id":"page","name":"Page","x":0,"y":0,"width":1200,"layout":"vertical","gap":32,"padding":64,"fill":"#f5f5f4","children":[
                {{"type":"frame","id":"row","name":"Numbers","gap":48,"children":[{},{},{}]}},
                {{"type":"text","id":"n1","content":"01","fontFamily":"Fraunces","fontSize":14,"fill":"#1c1917"}},
                {{"type":"text","id":"n2","content":"02 /","fontFamily":"Fraunces","fontSize":14,"fill":"#1c1917"}},
                {{"type":"text","id":"n3","content":"03.","fontFamily":"Fraunces","fontSize":14,"fill":"#1c1917"}},
                {{"type":"frame","id":"band","name":"Band","width":600,"height":120,"padding":24,"fill":"#b45309","effect":[
                    {{"type":"shadow","color":"#f59e0b80","offset":{{"x":0,"y":0}},"blur":40}},
                    {{"type":"shadow","color":"#000000","offset":{{"x":6,"y":6}},"blur":0}}],"children":[
                  {{"type":"text","id":"g","name":"Aside","content":"Sailings may change at short notice","fontFamily":"Fraunces","fontSize":16,"fill":"#a8a29e"}}]}},
                {{"type":"frame","id":"tag","name":"Tag","width":80,"height":32,"fill":"#ffffff","effect":{{"type":"shadow","color":"#000000","offset":{{"x":4,"y":4}},"blur":0}}}}]}}]}}"##,
            stat("a", "1,284", "crossings"),
            stat("b", "98%", "on time"),
            stat("c", "4.2k", "cars carried")
        );
        let said = notes(&json, &[]);
        let has = |part: &str| said.iter().any(|note| note.contains(part));
        assert!(has("stock row of statistics") && has("\"Numbers\" (row)"), "{said:#?}");
        assert!(has("coloured glow") && has("\"Band\" (band)"), "{said:#?}");
        assert!(has("2 layers have a block shadow"), "{said:#?}");
        assert!(has("Grey text on a coloured surface") && has("\"Aside\" (g)"), "{said:#?}");
        assert!(has("fraunces is among the faces"), "{said:#?}");
        assert!(has("tighter than 4%") && has("(an)"), "{said:#?}");
        assert!(has("numbered 01, 02, 03"), "{said:#?}");
        assert!(is_figure("$12M") && !is_figure("12 months") && !is_figure("Craignure"));
    }

    #[test]
    fn rows_whose_figures_line_up_on_no_edge_are_named() {
        let row = |id: &str, width: u32, gap: u32| {
            format!(
                r##"{{"type":"frame","id":"{id}","width":300,"gap":{gap},"children":[
                  {{"type":"rectangle","id":"{id}n","width":100,"height":20,"fill":"#1c1917"}},
                  {{"type":"rectangle","id":"{id}v","width":{width},"height":20,"fill":"#1c1917"}}]}}"##
            )
        };
        let list = |rows: [String; 3]| {
            format!(
                r#"{{"version":"2.22","children":[{{"type":"frame","id":"list","name":"Done","x":0,"y":0,"layout":"vertical","gap":8,"children":[{}]}}]}}"#,
                rows.join(",")
            )
        };
        // Values of three widths after gaps of three sizes: no edge is shared.
        let said = notes(&list([row("a", 40, 10), row("b", 55, 20), row("c", 31, 37)]), &[]);
        assert!(said.iter().any(|note| note.contains("part 2 of the rows in \"Done\" (list)")), "{said:#?}");
        // The same values starting on one edge are a column.
        assert_eq!(notes(&list([row("a", 40, 10), row("b", 55, 10), row("c", 31, 10)]), &[]), Vec::<String>::new());
    }

    #[test]
    fn a_shape_running_off_a_page_that_does_not_cut_it_is_named() {
        let page = |clip: &str| {
            format!(
                r##"{{"version":"2.22","children":[{{"type":"frame","id":"sheet","name":"Poster","x":0,"y":0,"width":400,"height":600,"layout":"none","fill":"#1e3fb8"{clip},"children":[
                  {{"type":"ellipse","id":"sun","name":"Disc","x":250,"y":200,"width":300,"height":300,"fill":"#ffd02e"}}]}}]}}"##
            )
        };
        let said = notes(&page(""), &[]);
        assert!(said.iter().any(|note| note.contains("\"Disc\" (sun) reaches 150 over the edge of \"Poster\" (sheet)")), "{said:#?}");
        assert_eq!(notes(&page(r#","clip":true"#), &[]), Vec::<String>::new());
    }

    #[test]
    fn texts_set_on_top_of_each_other_by_hand_are_named() {
        let page = |x: u32| {
            format!(
                r##"{{"version":"2.22","children":[{{"type":"frame","id":"card","x":0,"y":0,"width":400,"height":100,"layout":"none","children":[
                  {{"type":"text","id":"a","name":"Times","x":20,"y":20,"content":"22:00 to 07:30","fontSize":20,"fill":"#1c1917"}},
                  {{"type":"text","id":"b","name":"Length","x":{x},"y":22,"content":"9 h 30 min","fontSize":20,"fill":"#1c1917"}}]}}]}}"##
            )
        };
        let said = notes(&page(100), &[]);
        assert!(said.iter().any(|note| note.contains("\"Times\" (a) lies over \"Length\" (b)")), "{said:#?}");
        assert_eq!(notes(&page(200), &[]), Vec::<String>::new());
    }

    #[test]
    fn a_button_squeezed_under_a_line_of_text_is_named() {
        let card = |padding: u32, gap: u32| {
            format!(
                r##"{{"version":"2.22","children":[{{"type":"frame","id":"card","x":0,"y":0,"width":200,"layout":"vertical","gap":{gap},"alignItems":"end","children":[
                  {{"type":"text","id":"due","name":"Due","content":"Tomorrow","fontFamily":"Georgia","fontSize":14,"fill":"#1c1917"}},
                  {{"type":"frame","id":"renew","name":"Renew","padding":[{padding},10],"stroke":"#1c1917","strokeWidth":1,"cornerRadius":8,"children":[
                    {{"type":"text","id":"label","content":"Renew","fontFamily":"Georgia","fontSize":14,"fill":"#1c1917"}}]}}]}}]}}"##
            )
        };
        let said = notes(&card(2, 2), &[]);
        assert!(said.iter().any(|note| note.contains("\"Renew\" (renew) leaves its label 2 above and below")), "{said:#?}");
        assert!(said.iter().any(|note| note.contains("\"Renew\" (renew) sits 2 from \"Due\" (due)")), "{said:#?}");
        assert_eq!(notes(&card(8, 8), &[]), Vec::<String>::new());
    }

    #[test]
    fn two_texts_side_by_side_with_nothing_between_them_are_named() {
        let row = |gap: u32| {
            format!(
                r##"{{"version":"2.22","children":[{{"type":"frame","id":"row","x":0,"y":0,"gap":{gap},"children":[
                  {{"type":"text","id":"a","name":"Figure","content":"212.40","fontFamily":"Georgia","fontSize":20,"fill":"#1c1917"}},
                  {{"type":"text","id":"b","name":"Note","content":"on last Saturday","fontFamily":"Georgia","fontSize":20,"fill":"#1c1917"}}]}}]}}"##
            )
        };
        let said = notes(&row(0), &[]);
        assert!(said.iter().any(|note| note.contains("\"Figure\" (a) touches \"Note\" (b)")), "{said:#?}");
        assert_eq!(notes(&row(8), &[]), Vec::<String>::new());
    }

    #[test]
    fn a_figure_that_ends_a_little_short_of_the_others_is_named() {
        let page = |x: u32| {
            let amount = |id: &str, at: u32, y: u32| {
                format!(
                    r##"{{"type":"text","id":"{id}","name":"{id}","x":{at},"y":{y},"content":"12345","fontFamily":"Georgia","fontSize":20,"fontWeight":"700","fill":"#1c1917"}}"##
                )
            };
            let label = |id: &str, y: u32| {
                format!(r##"{{"type":"text","id":"l{id}","x":20,"y":{y},"content":"Rent","fontFamily":"Georgia","fontSize":40,"fill":"#57534e"}}"##)
            };
            format!(
                r##"{{"version":"2.22","children":[{{"type":"frame","id":"card","x":0,"y":0,"width":400,"height":300,"layout":"none","fill":"#f5f5f4","children":[{},{},{},{},{},{},{},{}]}}]}}"##,
                label("1", 20),
                label("2", 80),
                label("3", 140),
                label("4", 200),
                amount("a", 280, 20),
                amount("d", x, 80),
                amount("c", 280, 140),
                amount("b", 280, 200)
            )
        };
        let said = notes(&page(268), &[]);
        assert!(said.iter().any(|note| note.contains("\"d\" (d) ends 12 off the edge 3 others end on")), "{said:#?}");
        assert_eq!(notes(&page(280), &[]), Vec::<String>::new());
    }

    #[test]
    fn a_list_that_ends_right_on_a_bar_below_it_is_named() {
        let screen = |height: u32| {
            format!(
                r##"{{"version":"2.22","children":[{{"type":"frame","id":"screen","x":0,"y":0,"width":390,"height":300,"layout":"none","fill":"#f5f5f4","clip":true,"children":[
                  {{"type":"frame","id":"list","name":"Activity","x":0,"y":0,"width":390,"height":{height},"layout":"vertical","justifyContent":"end","children":[
                    {{"type":"text","id":"row","content":"Payroll","fontFamily":"Georgia","fontSize":20,"fill":"#1c1917"}}]}},
                  {{"type":"frame","id":"tabs","name":"Tab bar","x":0,"y":240,"width":390,"height":60,"fill":"#ffffff"}}]}}]}}"##
            )
        };
        let said = notes(&screen(238), &[]);
        assert!(said.iter().any(|note| note.contains("\"Activity\" (list) runs up to 2 above \"Tab bar\" (tabs)")), "{said:#?}");
        assert_eq!(notes(&screen(220), &[]), Vec::<String>::new());
    }

    #[test]
    fn a_line_under_the_last_row_of_a_list_is_named() {
        let list = |last: &str| {
            let row = |id: &str, stroke: &str| format!(r#"{{"type":"frame","id":"{id}","name":"Row","width":300,"height":40{stroke}}}"#);
            let ruled = r##","stroke":"#d6d3d1","strokeWidth":{"bottom":1}"##;
            format!(
                r#"{{"version":"2.22","children":[{{"type":"frame","id":"rows","name":"Activity","x":0,"y":0,"layout":"vertical","children":[{},{},{}]}}]}}"#,
                row("a", ruled),
                row("b", ruled),
                row("c", last)
            )
        };
        let said = notes(&list(r##","stroke":"#d6d3d1","strokeWidth":{"bottom":1}"##), &[]);
        assert!(said.iter().any(|note| note.contains("the last row of \"Activity\" (rows) (\"Row\" (c)) has a line under it")), "{said:#?}");
        assert_eq!(notes(&list(""), &[]), Vec::<String>::new());
    }

    #[test]
    fn colours_are_told_apart_by_hue() {
        let (hue, saturation, light) = hsl(Rgba { r: 99, g: 102, b: 241, a: 255 });
        assert!((238.0..240.0).contains(&hue) && saturation > 0.8 && (0.6..0.7).contains(&light));
        assert!(violet(Rgba { r: 168, g: 85, b: 247, a: 255 }) && !violet(Rgba { r: 180, g: 83, b: 9, a: 255 }));
        assert!(!accent(Rgba { r: 250, g: 250, b: 249, a: 255 }) && !accent(Rgba::BLACK));
    }
}
