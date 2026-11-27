//! Sizes and positions every scene node: flex rows and columns, `fit_content` and `fill_container`, text growth.
//!
//! The layout works one axis at a time (see `MECHANICS.md`): every width is decided first (fit-content
//! containers grow to their children, then fill-container children share what is left), then text heights are
//! measured at those widths, then every height the same way, and finally positions are placed. A child's size
//! in a layout is the bounding box of its rotated box.

use crate::geom::{Affine, Point, Rect};
use crate::model::{Align, Justify, Kind, LayoutMode, Node, Padding};
use crate::path;
use crate::scene::SceneNode;
use crate::value::{Num, Sizing, narrow};
use crate::vars::{ThemeContext, Vars};

/// Font size when a text node sets none.
pub const DEFAULT_FONT_SIZE: f32 = 14.0;
/// Font family when a text node sets none.
pub const DEFAULT_FONT_FAMILY: &str = "Inter";
/// Sticky note: least width.
pub const NOTE_MIN_WIDTH: f32 = 250.0;
/// Sticky note: least height.
pub const NOTE_MIN_HEIGHT: f32 = 219.0;
/// Sticky note: padding on every side.
pub const NOTE_PADDING: f32 = 24.0;
/// Sticky note: corner radius.
pub const NOTE_RADIUS: f32 = 24.0;
/// Sticky note: text size.
pub const NOTE_FONT_SIZE: f32 = 15.0;
/// Icon size when it sets none.
pub const DEFAULT_ICON_SIZE: f32 = 24.0;
/// Size of a `ref` whose component is missing.
pub const BROKEN_REF_SIZE: (f32, f32) = (120.0, 90.0);
/// Least size a `fill_container` child gets.
pub const MIN_FILL_SIZE: f32 = 1.0;

/// A text block to measure or draw, in canvas units.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    /// The text, variables already applied.
    pub content: String,
    /// Font family as written; drawing may fall back to another face.
    pub family: String,
    /// Font size.
    pub size: f32,
    /// Line height in canvas units, `round(lineHeight × fontSize)`; `None` uses the font's own.
    pub line_px: Option<f32>,
    /// Extra space between letters.
    pub letter_spacing: f32,
    /// 100–900.
    pub weight: u16,
    /// Italic.
    pub italic: bool,
    /// Underlined.
    pub underline: bool,
    /// Struck through.
    pub strikethrough: bool,
    /// Horizontal alignment.
    pub align: TextAlign,
    /// Vertical alignment inside a fixed-height box.
    pub valign: TextVAlign,
    /// Wrap width; `None` never wraps.
    pub wrap: Option<f32>,
}

/// Horizontal text alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    /// Left.
    Left,
    /// Centre.
    Center,
    /// Right.
    Right,
    /// Both edges (drawn left-aligned in this version).
    Justify,
}

/// Vertical text alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextVAlign {
    /// Top.
    Top,
    /// Middle.
    Middle,
    /// Bottom.
    Bottom,
}

/// Measures a text block: returns its width and height in canvas units.
pub type Measure<'a> = dyn Fn(&TextStyle) -> (f32, f32) + 'a;

/// Lays out the whole scene.
// Human: widths first (fit, then fill), text heights at those widths, heights (fit, then
// fill), positions. Each pass walks the whole tree, so a parent sees its children's final width before heights.
// Agent: WRITES SceneNode::local and ::to_doc for every node; CALLS measure about twice per text node.
pub fn layout(scene: &mut [SceneNode], vars: Vars<'_>, measure: &Measure<'_>) {
    layout_roots(scene, vars, measure);
    crate::connector::route(scene, vars);
}

/// Lays out the top-level nodes `scene` and what is in them, without routing connectors: each is laid out by
/// itself, so some of a scene's can be laid out again while the others stay as they are. Call
/// [`crate::connector::route`] on the whole scene afterwards when it has connectors.
// Agent: WRITES SceneNode::local and ::to_doc for every node; CALLS measure about twice per text node.
pub fn layout_roots(scene: &mut [SceneNode], vars: Vars<'_>, measure: &Measure<'_>) {
    layout_under(scene, &Affine::IDENTITY, vars, measure);
}

/// Lays out `node` and what is in it by itself, as a child of a node whose local → document transform is
/// `parent`: what laying out the whole scene gives for it, when [`stands_alone`] holds for it there.
// Agent: WRITES SceneNode::local and ::to_doc for `node` and every node in it; connectors are not routed.
pub fn layout_within(node: &mut SceneNode, parent: &Affine, vars: Vars<'_>, measure: &Measure<'_>) {
    layout_under(std::slice::from_mut(node), parent, vars, measure);
}

/// Whether the child `node` (with `theme` in force for it) of `parent` can be laid out without `parent` and
/// its other children, and they without it: `parent` neither takes its size from its children nor places them.
// Human: a group is as big as its children and moves them all when one moves; a frame with a horizontal or
// vertical layout places the children that take part in it one after the other, and may take its size from
// them or give them theirs. In a frame without a layout, and for a child that keeps out of the layout
// (absolute, or switched off), a child's box is its own x, y, width and height.
#[must_use]
pub fn stands_alone(parent: &SceneNode, node: &Node, theme: &ThemeContext, vars: Vars<'_>) -> bool {
    if parent.node.kind == Kind::Group {
        return false;
    }
    let enabled = node.enabled.as_ref().and_then(|flag| vars.flag(flag, theme)).unwrap_or(true);
    let takes_part = enabled && !node.is_absolute() && node.kind != Kind::Connector;
    Engine::direction(parent).is_none() || !takes_part
}

/// [`layout_roots`] for nodes that lie in a node whose local → document transform is `parent`.
fn layout_under(scene: &mut [SceneNode], parent: &Affine, vars: Vars<'_>, measure: &Measure<'_>) {
    let engine = Engine { vars, measure };
    for root in scene.iter_mut() {
        engine.init(root);
    }
    for axis in [Axis::X, Axis::Y] {
        if axis == Axis::Y {
            for root in scene.iter_mut() {
                engine.text_heights(root);
            }
        }
        for root in scene.iter_mut() {
            engine.fit(root, axis);
            engine.fill(root, axis);
        }
    }
    for root in scene.iter_mut() {
        let (x, y) = position(&root.node);
        let offset = engine.group_offset(root);
        root.local.x = x + offset.x;
        root.local.y = y + offset.y;
        engine.positions(root);
    }
    for root in scene.iter_mut() {
        place(root, parent);
    }
}

/// The text style of a text or note node.
#[must_use]
pub fn text_style(node: &Node, vars: Vars<'_>, theme: &ThemeContext) -> TextStyle {
    let number = |n: &Option<Num>| n.as_ref().and_then(|n| vars.number(n, theme));
    let flag = |f: &Option<crate::value::Flag>| f.as_ref().and_then(|f| vars.flag(f, theme)).unwrap_or(false);
    let is_note = node.kind == Kind::Note;
    let size = if is_note { NOTE_FONT_SIZE } else { number(&node.font_size).unwrap_or(DEFAULT_FONT_SIZE).max(0.5) };
    TextStyle {
        content: node.content.as_deref().map(|c| vars.text(c, theme)).unwrap_or_default(),
        family: node.font_family.as_deref().map_or_else(|| DEFAULT_FONT_FAMILY.to_owned(), |f| vars.text(f, theme)),
        size,
        line_px: number(&node.line_height).filter(|l| *l != 0.0).map(|l| (l * size).round()),
        letter_spacing: number(&node.letter_spacing).unwrap_or(0.0),
        weight: number(&node.font_weight).map_or(400, weight_class),
        italic: node.font_style.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("italic")),
        underline: flag(&node.underline),
        strikethrough: flag(&node.strikethrough),
        align: match node.text_align.as_deref() {
            Some("center") => TextAlign::Center,
            Some("right") => TextAlign::Right,
            Some("justify") => TextAlign::Justify,
            _ => TextAlign::Left,
        },
        valign: match node.text_align_vertical.as_deref() {
            Some("middle") => TextVAlign::Middle,
            Some("bottom") => TextVAlign::Bottom,
            _ => TextVAlign::Top,
        },
        wrap: None,
    }
}

/// A CSS font weight rounded into 100–900.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to 100..=900 before the cast")]
fn weight_class(weight: f32) -> u16 {
    weight.clamp(100.0, 900.0).round() as u16
}

/// A node's own `x`/`y`.
fn position(node: &Node) -> (f32, f32) {
    let coordinate = |n: &Option<serde_json::Number>| n.as_ref().and_then(serde_json::Number::as_f64).map_or(0.0, narrow);
    (coordinate(&node.x), coordinate(&node.y))
}

/// Padding as `[top, right, bottom, left]`.
fn padding(node: &Node, vars: Vars<'_>, theme: &ThemeContext) -> [f32; 4] {
    let px = |v: &Num| vars.number(v, theme).unwrap_or(0.0).max(0.0);
    match &node.padding {
        None => [0.0; 4],
        Some(Padding::All(all)) => [px(all); 4],
        Some(Padding::Sides(sides)) => match sides.as_slice() {
            [vertical, horizontal] => [px(vertical), px(horizontal), px(vertical), px(horizontal)],
            [top, right, bottom, left] => [px(top), px(right), px(bottom), px(left)],
            [all] => [px(all); 4],
            _ => [0.0; 4],
        },
    }
}

/// One of the two layout axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

/// One side's sizing, resolved to numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Side {
    Fixed(f32),
    Fit(Option<f32>),
    Fill(Option<f32>),
}

impl Side {
    /// The size before layout: the number, or the fallback (0 without one).
    fn start(self) -> f32 {
        match self {
            Side::Fixed(v) => v,
            Side::Fit(f) | Side::Fill(f) => f.unwrap_or(0.0),
        }
    }
}

struct Engine<'a, 'm> {
    vars: Vars<'a>,
    measure: &'m Measure<'m>,
}

impl Engine<'_, '_> {
    fn side(&self, scene: &SceneNode, axis: Axis) -> Side {
        let value = match axis {
            Axis::X => scene.node.width.as_ref(),
            Axis::Y => scene.node.height.as_ref(),
        };
        match value.map(Num::sizing) {
            None => Side::Fit(None),
            Some(Sizing::Fixed(num)) => self.vars.number(&num, &scene.theme).map_or(Side::Fit(None), |v| Side::Fixed(v.max(0.0))),
            Some(Sizing::FitContent(fallback)) => Side::Fit(fallback),
            Some(Sizing::FillContainer(fallback)) => Side::Fill(fallback),
        }
    }

    fn enabled(&self, scene: &SceneNode) -> bool {
        scene.node.enabled.as_ref().and_then(|f| self.vars.flag(f, &scene.theme)).unwrap_or(true)
    }

    /// Whether a child takes part in its parent's flex layout (enabled and not absolute).
    // Human: a connector never does: it lies wherever its two ends are.
    fn affects(&self, child: &SceneNode) -> bool {
        self.enabled(child) && !child.node.is_absolute() && child.node.kind != Kind::Connector
    }

    /// The flex direction of a node: `Some` only for frames with a horizontal or vertical layout.
    fn direction(scene: &SceneNode) -> Option<Axis> {
        if scene.node.kind != Kind::Frame {
            return None;
        }
        match scene.node.layout_mode() {
            LayoutMode::None => None,
            LayoutMode::Horizontal => Some(Axis::X),
            LayoutMode::Vertical => Some(Axis::Y),
        }
    }

    /// The text style measured at the node's current width, as its growth mode says.
    fn wrapped_style(&self, scene: &SceneNode) -> TextStyle {
        let mut style = text_style(&scene.node, self.vars, &scene.theme);
        match scene.node.kind {
            Kind::Note => style.wrap = Some(scene.local.w - 2.0 * NOTE_PADDING),
            _ if matches!(scene.node.text_growth.as_deref(), Some("fixed-width" | "fixed-width-height")) => style.wrap = Some(scene.local.w),
            _ => {}
        }
        style
    }

    /// Sizes before any layout: numbers, fallbacks, measured text, geometry.
    fn init(&self, scene: &mut SceneNode) {
        let (sx, sy) = (self.side(scene, Axis::X), self.side(scene, Axis::Y));
        let (mut w, mut h) = (sx.start(), sy.start());
        match &scene.node.kind {
            // Human: `textLayoutBounds`: auto text is as wide as its longest line, rounded up; every text's height
            // is its paragraph's, rounded up; fixed-width-height keeps its stored height.
            Kind::Text if !matches!(scene.node.text_growth.as_deref(), Some("fixed-width" | "fixed-width-height")) => {
                let (mw, mh) = (self.measure)(&text_style(&scene.node, self.vars, &scene.theme));
                w = mw.ceil();
                h = mh.ceil();
            }
            Kind::Note => w = w.max(NOTE_MIN_WIDTH),
            Kind::Path if scene.node.width.is_none() || scene.node.height.is_none() => {
                let natural = scene.node.geometry.as_deref().and_then(|g| path::parse(g).ok()).and_then(|p| path::bounds(&p));
                if let Some(b) = natural {
                    w = if scene.node.width.is_none() { b.w } else { w };
                    h = if scene.node.height.is_none() { b.h } else { h };
                }
            }
            Kind::Icon | Kind::IconFont => {
                w = if scene.node.width.is_none() { DEFAULT_ICON_SIZE } else { w };
                h = if scene.node.height.is_none() { DEFAULT_ICON_SIZE } else { h };
            }
            Kind::Ref if !scene.resolved => {
                w = if scene.node.width.is_none() { BROKEN_REF_SIZE.0 } else { w };
                h = if scene.node.height.is_none() { BROKEN_REF_SIZE.1 } else { h };
            }
            _ => {}
        }
        scene.local = Rect::new(0.0, 0.0, w.max(0.0), h.max(0.0));
        for child in &mut scene.children {
            self.init(child);
        }
    }

    /// Text and note heights at their final widths (between the width and height passes).
    fn text_heights(&self, scene: &mut SceneNode) {
        match scene.node.kind {
            Kind::Text if scene.node.text_growth.as_deref() == Some("fixed-width") => {
                scene.local.h = (self.measure)(&self.wrapped_style(scene)).1.ceil();
            }
            Kind::Note => {
                let (_, text_h) = (self.measure)(&self.wrapped_style(scene));
                scene.local.h = (text_h.ceil() + 2.0 * NOTE_PADDING).max(NOTE_MIN_HEIGHT);
            }
            _ => {}
        }
        for child in &mut scene.children {
            self.text_heights(child);
        }
    }

    /// The node's size along `axis`.
    fn size_of(scene: &SceneNode, axis: Axis) -> f32 {
        match axis {
            Axis::X => scene.local.w,
            Axis::Y => scene.local.h,
        }
    }

    fn set_size(scene: &mut SceneNode, axis: Axis, value: f32) {
        match axis {
            Axis::X => scene.local.w = value,
            Axis::Y => scene.local.h = value,
        }
    }

    /// The child's footprint in its parent: the bounding box of its rotated and flipped box.
    fn outer(scene: &SceneNode) -> Rect {
        linear(&scene.node).bounds(&Rect::new(0.0, 0.0, scene.local.w, scene.local.h))
    }

    fn outer_size(scene: &SceneNode, axis: Axis) -> f32 {
        let b = Self::outer(scene);
        match axis {
            Axis::X => b.w,
            Axis::Y => b.h,
        }
    }

    /// Fit-content containers take the size of their children along `axis` (children first).
    // Human: Children that fill along this axis, are hidden or absolute don't count, and gaps are counted only
    // between the children that do. With none of them the container keeps the size it had (its fallback).
    fn fit(&self, scene: &mut SceneNode, axis: Axis) {
        for child in &mut scene.children {
            self.fit(child, axis);
        }
        if scene.node.kind == Kind::Group {
            // Human: a group is always as big as its children (`resizesWithChildren`).
            let union = self.children_union(scene);
            Self::set_size(scene, axis, if axis == Axis::X { union.w } else { union.h });
            return;
        }
        let Some(direction) = Self::direction(scene) else { return };
        if !matches!(self.side(scene, axis), Side::Fit(_)) {
            return;
        }
        let mut total = 0.0f32;
        let mut count = 0usize;
        for child in &scene.children {
            if !self.affects(child) || matches!(self.side(child, axis), Side::Fill(_)) {
                continue;
            }
            count += 1;
            let size = Self::outer_size(child, axis);
            total = if direction == axis { total + size } else { total.max(size) };
        }
        if count == 0 {
            return;
        }
        let (spacing, pad) = (self.spacing(scene), padding(&scene.node, self.vars, &scene.theme));
        if direction == axis {
            total += spacing * count_f32(count - 1);
        }
        let pad_sum = if axis == Axis::X { pad[1] + pad[3] } else { pad[0] + pad[2] };
        Self::set_size(scene, axis, total + pad_sum);
    }

    /// Fill-container children share the space left along the main axis, or take the whole inner size
    /// across it; never less than 1. Parents first.
    fn fill(&self, scene: &mut SceneNode, axis: Axis) {
        if let Some(direction) = Self::direction(scene) {
            let pad = padding(&scene.node, self.vars, &scene.theme);
            let pad_sum = if axis == Axis::X { pad[1] + pad[3] } else { pad[0] + pad[2] };
            let mut inner = Self::size_of(scene, axis) - pad_sum;
            let share = if direction == axis {
                let (mut count, mut fills) = (0usize, 0usize);
                for child in scene.children.iter().filter(|c| self.affects(c)) {
                    count += 1;
                    if matches!(self.side(child, axis), Side::Fill(_)) {
                        fills += 1;
                    } else {
                        inner -= Self::outer_size(child, axis);
                    }
                }
                inner -= count_f32(count.saturating_sub(1)) * self.spacing(scene);
                if fills == 0 { 0.0 } else { inner / count_f32(fills) }
            } else {
                inner
            };
            let share = share.max(MIN_FILL_SIZE);
            for i in 0..scene.children.len() {
                let child = &scene.children[i];
                if self.affects(child) && matches!(self.side(child, axis), Side::Fill(_)) {
                    Self::set_size(&mut scene.children[i], axis, share);
                }
            }
        }
        for child in &mut scene.children {
            self.fill(child, axis);
        }
    }

    fn spacing(&self, scene: &SceneNode) -> f32 {
        scene.node.gap.as_ref().and_then(|g| self.vars.number(g, &scene.theme)).unwrap_or(0.0)
    }

    /// The union of a group's children's footprints, in the group's stored coordinates.
    fn children_union(&self, scene: &SceneNode) -> Rect {
        let mut union: Option<Rect> = None;
        for child in scene.children.iter().filter(|c| self.enabled(c) && c.node.kind != Kind::Connector) {
            let (x, y) = position(&child.node);
            let o = Self::outer(child);
            let r = Rect::new(x + o.x, y + o.y, o.w, o.h);
            union = Some(union.map_or(r, |u| u.union(&r)));
        }
        union.unwrap_or(Rect::ZERO)
    }

    /// How far a group's box starts from its stored origin (its top-left child); zero for other kinds.
    fn group_offset(&self, scene: &SceneNode) -> Point {
        if scene.node.kind == Kind::Group {
            let u = self.children_union(scene);
            Point::new(u.x, u.y)
        } else {
            Point::default()
        }
    }

    /// Places the children of `scene` (flex placement or their own x/y), then recurses.
    // Human: Free space is the inner size minus the children's footprints, and is not clamped: centred content
    // that is too big sticks out on both sides. Space-between/around replace the gap.
    fn positions(&self, scene: &mut SceneNode) {
        let shift = self.group_offset(scene);
        let direction = Self::direction(scene);
        for child in &mut scene.children {
            let (x, y) = position(&child.node);
            let offset = self.group_offset(child);
            child.local.x = x + offset.x - shift.x;
            child.local.y = y + offset.y - shift.y;
        }
        if let Some(main) = direction
            && !scene.children.is_empty()
        {
            let cross = if main == Axis::X { Axis::Y } else { Axis::X };
            let pad = padding(&scene.node, self.vars, &scene.theme);
            let inner = [scene.local.w - (pad[1] + pad[3]), scene.local.h - (pad[0] + pad[2])];
            let index = |axis: Axis| usize::from(axis == Axis::Y);
            let mut cursor = [pad[3], pad[0]];
            let mut free = inner;
            let mut count = 0usize;
            for child in scene.children.iter().filter(|c| self.affects(c)) {
                count += 1;
                let o = Self::outer(child);
                free[0] -= o.w;
                free[1] -= o.h;
            }
            let mut spacing = self.spacing(scene);
            let n = count_f32(count);
            let m = index(main);
            if count > 0 {
                match scene.node.justify() {
                    Justify::Start => {}
                    Justify::End => cursor[m] += free[m] - spacing * (n - 1.0),
                    Justify::Center => cursor[m] += (free[m] - spacing * (n - 1.0)) / 2.0,
                    Justify::SpaceBetween => spacing = if count > 1 { free[m] / (n - 1.0) } else { 0.0 },
                    Justify::SpaceAround => {
                        spacing = free[m] / n;
                        cursor[m] += spacing / 2.0;
                    }
                }
            }
            let c = index(cross);
            let align = scene.node.align();
            for child in &mut scene.children {
                if !self.affects(child) {
                    continue;
                }
                let o = Self::outer(child);
                let size = [o.w, o.h];
                let mut at = cursor;
                match align {
                    Align::Start => {}
                    Align::End => at[c] += inner[c] - size[c],
                    Align::Center => at[c] += (inner[c] - size[c]) / 2.0,
                }
                // Human: the footprint's corner lands on the slot, whatever the rotation (`layoutPositionToLocalPosition`).
                let offset = self.group_offset(child);
                child.local.x = at[0] - o.x + offset.x;
                child.local.y = at[1] - o.y + offset.y;
                cursor[m] += size[m] + spacing;
            }
        }
        for child in &mut scene.children {
            self.positions(child);
        }
    }
}

/// A count as f32 (child counts stay far below f32's exact range).
#[expect(clippy::cast_precision_loss, reason = "child counts are far below 2^24")]
fn count_f32(count: usize) -> f32 {
    count as f32
}

/// A node's rotation and flips around its own origin (no translation).
fn linear(node: &Node) -> Affine {
    let rotation = node.rotation.as_ref().and_then(Num::literal).unwrap_or(0.0);
    let flip = |f: &Option<crate::value::Flag>| matches!(f, Some(crate::value::Flag::Value(true)));
    let mut t = Affine::IDENTITY;
    if rotation.abs() > f32::EPSILON {
        t = t.then(&Affine::rotate_ccw(rotation));
    }
    if flip(&node.flip_x) || flip(&node.flip_y) {
        t = t.then(&Affine::scale(if flip(&node.flip_x) { -1.0 } else { 1.0 }, if flip(&node.flip_y) { -1.0 } else { 1.0 }));
    }
    t
}

/// Builds each node's local → document transform from its box, rotation and flips.
fn place(scene: &mut SceneNode, parent: &Affine) {
    let node = &scene.node;
    let rotation = node.rotation.as_ref().and_then(Num::literal).unwrap_or(0.0);
    let flip = |f: &Option<crate::value::Flag>| matches!(f, Some(crate::value::Flag::Value(true)));
    let (fx, fy) = (flip(&node.flip_x), flip(&node.flip_y));
    let mut transform = parent.then(&Affine::translate(scene.local.x, scene.local.y));
    if rotation.abs() > f32::EPSILON {
        transform = transform.then(&Affine::rotate_ccw(rotation));
    }
    if fx || fy {
        transform = transform.then(&Affine::scale(if fx { -1.0 } else { 1.0 }, if fy { -1.0 } else { 1.0 }));
    }
    scene.to_doc = transform;
    for child in &mut scene.children {
        place(child, &transform);
    }
}

/// The box of `scene` in document coordinates (the bounds of its rotated box).
#[must_use]
pub fn doc_bounds(scene: &SceneNode) -> Rect {
    scene.to_doc.bounds(&Rect::new(0.0, 0.0, scene.local.w, scene.local.h))
}

/// `p` (document coordinates) in `scene`'s own coordinates.
#[must_use]
pub fn to_local(scene: &SceneNode, p: Point) -> Option<Point> {
    scene.to_doc.invert().map(|inverse| inverse.apply(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::scene;

    /// Every character is half the font size wide and lines are 1.2 × the size tall.
    fn measure(style: &TextStyle) -> (f32, f32) {
        let per_char = style.size * 0.5;
        let chars = count_f32(style.content.chars().count());
        let line = style.line_px.unwrap_or(style.size * 1.2);
        match style.wrap {
            Some(w) if w > 0.0 => {
                let per_line = (w / per_char).floor().max(1.0);
                (w, (chars / per_line).ceil().max(1.0) * line)
            }
            _ => (chars * per_char, line),
        }
    }

    fn laid_out(json: &str) -> Vec<SceneNode> {
        let doc: Document = serde_json::from_str(json).unwrap();
        let mut scene = scene::build(&doc, &ThemeContext::defaults(&doc));
        layout(&mut scene, Vars::new(&doc), &measure);
        scene
    }

    #[test]
    fn frames_default_to_a_horizontal_fit_content_row() {
        let s = laid_out(
            r#"{"version":"2.22","children":[{"type":"frame","id":"f","x":5,"y":6,"gap":10,"padding":[4,8],"children":[
              {"type":"rectangle","id":"a","width":20,"height":30},{"type":"rectangle","id":"b","width":40,"height":10}]}]}"#,
        );
        let f = &s[0];
        assert_eq!(f.local, Rect::new(5.0, 6.0, 8.0 + 20.0 + 10.0 + 40.0 + 8.0, 4.0 + 30.0 + 4.0));
        assert_eq!(f.children[0].local, Rect::new(8.0, 4.0, 20.0, 30.0));
        assert_eq!(f.children[1].local, Rect::new(38.0, 4.0, 40.0, 10.0));
    }

    #[test]
    fn fill_container_shares_the_space_left() {
        let s = laid_out(
            r#"{"version":"2.22","children":[{"type":"frame","id":"f","width":300,"height":100,"layout":"vertical","alignItems":"center","children":[
              {"type":"rectangle","id":"a","width":"fill_container","height":20},
              {"type":"rectangle","id":"b","width":50,"height":"fill_container"},
              {"type":"rectangle","id":"c","width":10,"height":"fill_container"}]}]}"#,
        );
        let kids = &s[0].children;
        assert_eq!(kids[0].local, Rect::new(0.0, 0.0, 300.0, 20.0));
        assert_eq!(kids[1].local, Rect::new(125.0, 20.0, 50.0, 40.0));
        assert_eq!(kids[2].local, Rect::new(145.0, 60.0, 10.0, 40.0));
    }

    #[test]
    fn justify_modes_place_children() {
        let make = |justify: &str| {
            laid_out(&format!(
                r#"{{"version":"2.22","children":[{{"type":"frame","id":"f","width":100,"height":10,"gap":7,"justifyContent":"{justify}","children":[
                  {{"type":"rectangle","id":"a","width":10,"height":10}},{{"type":"rectangle","id":"b","width":10,"height":10}}]}}]}}"#
            ))[0]
                .children
                .iter()
                .map(|c| c.local.x)
                .collect::<Vec<_>>()
        };
        assert_eq!(make("start"), vec![0.0, 17.0]);
        assert_eq!(make("center"), vec![36.5, 53.5]);
        assert_eq!(make("end"), vec![73.0, 90.0]);
        assert_eq!(make("space_between"), vec![0.0, 90.0]);
        assert_eq!(make("space_around"), vec![20.0, 70.0]);
    }

    #[test]
    fn overflowing_centred_content_sticks_out_on_both_sides() {
        // Placement doesn't clamp the free space: a 60-wide child centred in a 40-wide frame starts at -10.
        let s = laid_out(
            r#"{"version":"2.22","children":[{"type":"frame","id":"f","width":40,"height":10,"justifyContent":"center","children":[
              {"type":"rectangle","id":"a","width":60,"height":10}]}]}"#,
        );
        assert_eq!(s[0].children[0].local.x, -10.0);
    }

    #[test]
    fn fill_children_are_left_out_of_fit_content_and_its_gaps() {
        // Fit-content counts only non-fill children (and the gaps between them); the fill child then gets at least 1.
        let s = laid_out(
            r#"{"version":"2.22","children":[{"type":"frame","id":"f","gap":10,"children":[
              {"type":"rectangle","id":"a","width":30,"height":10},
              {"type":"rectangle","id":"b","width":"fill_container","height":10}]}]}"#,
        );
        assert_eq!(s[0].local.w, 30.0);
        assert_eq!(s[0].children[1].local.w, MIN_FILL_SIZE);
    }

    #[test]
    fn stretched_columns_keep_their_fit_height() {
        let s = laid_out(
            r#"{"version":"2.22","children":[{"type":"frame","id":"row","width":220,"gap":20,"children":[
              {"type":"frame","id":"a","width":"fill_container","layout":"vertical","children":[
                {"type":"text","id":"t","content":"abcdefghijabcdefghijabcdefghij","fontSize":10,"textGrowth":"fixed-width","width":"fill_container"}]},
              {"type":"frame","id":"b","width":"fill_container","height":"fill_container","layout":"vertical"}]}]}"#,
        );
        let row = &s[0];
        let a = &row.children[0];
        assert_eq!((a.local.w, a.local.h), (100.0, 24.0));
        assert_eq!((a.children[0].local.w, a.children[0].local.h), (100.0, 24.0));
        assert_eq!(row.children[1].local, Rect::new(120.0, 0.0, 100.0, 24.0));
        assert_eq!(row.local.h, 24.0);
    }

    #[test]
    fn layout_none_uses_positions_and_absolute_children_leave_the_flow() {
        let s = laid_out(
            r#"{"version":"2.22","children":[{"type":"frame","id":"f","layout":"vertical","children":[
              {"type":"rectangle","id":"a","width":10,"height":10},
              {"type":"rectangle","id":"abs","x":50,"y":60,"width":5,"height":5,"layoutPosition":"absolute"}]},
              {"type":"frame","id":"g","layout":"none","children":[{"type":"rectangle","id":"r","x":30,"y":40,"width":10,"height":10}]}]}"#,
        );
        assert_eq!(s[0].local, Rect::new(0.0, 0.0, 10.0, 10.0));
        assert_eq!(s[0].children[1].local, Rect::new(50.0, 60.0, 5.0, 5.0));
        // Fit-content only grows frames that have a layout: a free frame without a size stays 0 × 0.
        assert_eq!((s[1].local.w, s[1].local.h), (0.0, 0.0));
        assert_eq!(s[1].children[0].local, Rect::new(30.0, 40.0, 10.0, 10.0));
    }

    #[test]
    fn text_growth_line_height_and_notes() {
        let s = laid_out(
            r#"{"version":"2.22","children":[
              {"type":"text","id":"t","content":"abc","fontSize":10,"width":999},
              {"type":"text","id":"w","content":"abcdefgh","fontSize":10,"textGrowth":"fixed-width","width":20},
              {"type":"text","id":"l","content":"a","fontSize":10,"lineHeight":1.26},
              {"type":"note","id":"n","content":"hi"}]}"#,
        );
        // Auto text: measured width rounded up (15 → 15), width ignored.
        assert_eq!((s[0].local.w, s[0].local.h), (15.0, 12.0));
        assert_eq!((s[1].local.w, s[1].local.h), (20.0, 24.0));
        // Line height in whole units: round(1.26 × 10) = 13.
        assert_eq!(s[2].local.h, 13.0);
        assert_eq!((s[3].local.w, s[3].local.h), (NOTE_MIN_WIDTH, NOTE_MIN_HEIGHT));
    }

    #[test]
    fn rotated_children_take_their_bounding_box_in_a_row() {
        let s = laid_out(
            r#"{"version":"2.22","children":[{"type":"frame","id":"f","children":[
              {"type":"rectangle","id":"r","width":10,"height":20,"rotation":90},
              {"type":"rectangle","id":"s","width":5,"height":5}]}]}"#,
        );
        assert!((s[0].local.w - 25.0).abs() < 1e-4 && (s[0].local.h - 10.0).abs() < 1e-4);
        let b = doc_bounds(&s[0].children[0]);
        assert!((b.x).abs() < 1e-4 && (b.y).abs() < 1e-4 && (b.w - 20.0).abs() < 1e-4);
        assert!((s[0].children[1].local.x - 20.0).abs() < 1e-4);
    }

    #[test]
    fn missing_components_get_the_placeholder_size() {
        let s = laid_out(r#"{"version":"2.22","children":[{"type":"ref","id":"r","ref":"gone"}]}"#);
        assert_eq!((s[0].local.w, s[0].local.h), BROKEN_REF_SIZE);
    }
}
