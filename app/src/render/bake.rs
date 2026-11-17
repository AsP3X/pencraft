//! Effects: drop and inner shadows, layer blur and background blur.
//!
//! egui draws triangles and can't blur them. So an effect's source (the layer's shape, the layer itself, or
//! what lies behind it) is drawn into a bitmap on the CPU, blurred there, and put on the canvas as a picture.
//! Pictures are kept from one build of the display list to the next and found again by what they were made
//! from, so moving a layer or editing another one bakes nothing anew.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use egui::epaint::{ClippedShape, Mesh, Primitive, TextShape, Vertex};
use egui::{Color32, ColorImage, Pos2, TextureHandle, TextureId, TextureOptions};
use pen_doc::color::Rgba;
use pen_doc::geom::{Point, Rect};
use pen_doc::layout;
use pen_doc::scene::SceneNode;
use pen_doc::value::{Flag, Num, narrow};
use serde_json::Value;

use super::glass::Pane;
use super::{Builder, Item, TextItem, color32, glyph_scale, intersect, text_anchor, text_job, to_u8};
use crate::effects::{self, EffectKind};
use crate::raster::{self, Bitmap, Span};
use crate::shader::{self, Program, Role, Source, Type};
use crate::{gpu, images};

/// How far a blur reaches, in standard deviations (a shadow's layer is grown by `sigma * 3`).
pub(super) const REACH: f32 = 3.0;
/// Most pixels one baked picture has; a larger one is baked coarser and stretched.
const MAX_PIXELS: f32 = 1_000_000.0;
/// Most pixels of pictures kept although the last build didn't use them (64 MB).
const KEPT_PIXELS: usize = 16_000_000;
/// Longest side of a baked picture in pixels.
const MAX_SIDE: f32 = 2048.0;
/// Pictures up to this many pixels are painted at twice the size and halved, for smooth edges.
const FINE_PIXELS: usize = 400_000;
/// Positions are compared in steps of this fraction of a pixel when looking for a picture baked before.
const KEY_STEPS: f32 = 8.0;

/// The standard deviation of the Gaussian an effect's `blur` or `radius` asks for (half of it).
pub(super) fn sigma(radius: f32) -> f32 {
    radius.max(0.0) / 2.0
}

/// A sampler's block member, and the scale and offset that put its texture coordinates on its picture.
type SamplerMap = (String, [f32; 4]);

/// One shadow, variables looked up.
pub(super) struct Shadow {
    /// How far the shadow lies from its shape.
    pub offset: Point,
    /// The `blur` as written, which is the width egui fades a box's shadow over.
    pub blur: f32,
    /// Its colour.
    pub color: Rgba,
}

/// The effects of a node that are switched on and can be drawn.
#[derive(Default)]
pub(super) struct Effects {
    drops: Vec<Shadow>,
    inners: Vec<Shadow>,
    /// Standard deviation of the layer blur.
    layer_blur: Option<f32>,
    /// Standard deviation of the background blur.
    background_blur: Option<f32>,
    /// The shader effects, in the order they run.
    shaders: Vec<Value>,
}

impl Effects {
    pub(super) fn is_empty(&self) -> bool {
        self.drops.is_empty() && self.inners.is_empty() && self.layer_blur.is_none() && self.background_blur.is_none() && self.shaders.is_empty()
    }
}

/// How the node an effect belongs to is drawn.
pub(super) struct Look {
    /// The node's opacity, its ancestors' included.
    pub opacity: f32,
    /// Whether the node's fill hides everything under it.
    pub opaque: bool,
    /// What the node is clipped to.
    pub clip: Option<Rect>,
    /// The corner radius when the node is a plain box or a whole ellipse.
    pub radius: Option<f32>,
}

/// One baked picture.
struct Picture {
    handle: TextureHandle,
    /// The same pixels, for an effect that blurs this picture in turn.
    pixels: Arc<ColorImage>,
    /// The build that last used it.
    round: u64,
}

/// What baking a layer's material found out about it.
#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    /// The layer's id in the scene.
    pub id: String,
    /// Whether the material came out dark (`auto` decides by what is behind the layer).
    pub dark: bool,
    /// How light what is behind the layer is on average, 0–1 in linear light.
    pub lightness: f32,
    /// How far apart the lightest and the darkest parts behind it are, 0–1.
    pub spread: f32,
    /// How much of the material's own tint lies over what is behind, 0–1.
    pub tint_opacity: f32,
}

/// The pictures effects were baked into, by a hash of what each was made from.
// Agent: MAIN-THREAD only (RefCell); a picture not used by a build is dropped at the end of that build.
#[derive(Default)]
pub(super) struct Baked {
    pictures: RefCell<HashMap<u64, Picture>>,
    round: Cell<u64>,
    /// What the materials drawn since the last [`Baked::seen`] came to.
    seen: RefCell<Vec<Seen>>,
    /// What baking each material's picture found out, by the picture's key; the layer's id is left empty.
    learned: RefCell<HashMap<u64, Seen>>,
    /// The part of the document this build bakes pictures for; `None` bakes them all.
    cover: Cell<Option<Rect>>,
    /// Whether this build left a picture out because it lies outside `cover`.
    skipped: Cell<bool>,
    /// The clear picture that stands in for one left out.
    blank: RefCell<Option<TextureHandle>>,
}

impl Baked {
    /// Starts a build of the display list that bakes every picture.
    pub(super) fn begin(&self) {
        self.begin_within(None);
    }

    /// Starts a build of the display list. With `cover`, only the pictures that reach into that part of the
    /// document are baked; the others are left clear, and [`Baked::skipped`] says so afterwards.
    // Human: baking is by far the most of a build (a blur is worked out pixel by pixel on the CPU), and the
    // pixels grow fourfold with every doubling of the zoom. Zoomed in, most of a design is off screen: its
    // effects are left until the view comes near them.
    pub(super) fn begin_within(&self, cover: Option<Rect>) {
        self.round.set(self.round.get().wrapping_add(1));
        self.seen.borrow_mut().clear();
        self.cover.set(cover);
        self.skipped.set(false);
    }

    /// How many pixels the pictures held now have between them.
    pub(super) fn pixels_held(&self) -> usize {
        self.pictures.borrow().values().map(|picture| picture.pixels.size[0] * picture.pixels.size[1]).sum()
    }

    /// Whether the build since the last [`Baked::begin_within`] left pictures out.
    pub(super) fn skipped(&self) -> bool {
        self.skipped.get()
    }

    /// Ends a build: pictures it didn't use are let go, the longest unused first, down to [`KEPT_PIXELS`].
    // Human: the ones kept are those of the zoom step just left and of the part of the canvas just scrolled
    // away from, so going back to either costs nothing.
    pub(super) fn finish(&self) {
        let round = self.round.get();
        let mut pictures = self.pictures.borrow_mut();
        let mut unused: Vec<(u64, u64, usize)> = pictures
            .iter()
            .filter(|(_, picture)| picture.round != round)
            .map(|(key, picture)| (*key, picture.round, picture.pixels.size[0] * picture.pixels.size[1]))
            .collect();
        unused.sort_by_key(|(_, last, _)| std::cmp::Reverse(*last));
        let mut kept = 0usize;
        for (key, _, size) in unused {
            kept += size;
            if kept > KEPT_PIXELS {
                pictures.remove(&key);
            }
        }
        self.learned.borrow_mut().retain(|key, _| pictures.contains_key(key));
    }

    /// The texture for `key`, baked with `make` unless an earlier build already did. `area` is the part of the
    /// document the picture shows; one outside what this build covers is not baked and comes back clear.
    pub(super) fn texture(&self, ctx: &egui::Context, key: u64, area: Rect, make: impl FnOnce() -> Bitmap) -> TextureId {
        if let Some(picture) = self.pictures.borrow_mut().get_mut(&key) {
            picture.round = self.round.get();
            return picture.handle.id();
        }
        if self.cover.get().is_some_and(|cover| !cover.intersects(&area)) {
            self.skipped.set(true);
            return self
                .blank
                .borrow_mut()
                .get_or_insert_with(|| ctx.load_texture("effect-left-out", ColorImage::filled([1, 1], Color32::TRANSPARENT), TextureOptions::LINEAR))
                .id();
        }
        // `make` reads other pictures, so nothing is borrowed while it runs.
        let image = make().image();
        let handle = ctx.load_texture(format!("effect-{key:016x}"), image.clone(), TextureOptions::LINEAR);
        let id = handle.id();
        self.pictures.borrow_mut().insert(key, Picture { handle, pixels: Arc::new(image), round: self.round.get() });
        id
    }

    /// Keeps what baking the material picture `key` found out.
    pub(super) fn learn(&self, key: u64, found: Seen) {
        self.learned.borrow_mut().insert(key, found);
    }

    /// Notes that the layer `id` is drawn with the material picture `key`, baked now or kept from before.
    pub(super) fn note(&self, key: u64, id: &str) {
        if let Some(found) = self.learned.borrow().get(&key) {
            self.seen.borrow_mut().push(Seen { id: id.to_owned(), ..found.clone() });
        }
    }

    /// What the materials baked so far came to; the list starts anew.
    pub(super) fn seen(&self) -> Vec<Seen> {
        std::mem::take(&mut self.seen.borrow_mut())
    }

    /// The pixels of a baked picture.
    fn pixels(&self, texture: TextureId) -> Option<Arc<ColorImage>> {
        self.pictures.borrow().values().find(|picture| picture.handle.id() == texture).map(|picture| picture.pixels.clone())
    }
}

/// A count of pixels as a length.
#[expect(clippy::cast_precision_loss, reason = "pictures are at most a few thousand pixels a side")]
pub(super) fn length(pixels: usize) -> f32 {
    pixels as f32
}

/// A length as a count of pixels, at least one.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to 1..=MAX_SIDE before the cast")]
fn pixels(length: f32) -> usize {
    // Human: a hair is taken off before rounding up, so a size that is a whole number of pixels doesn't
    // become one more through rounding noise (which would make a moved layer bake anew).
    (length - 1e-3).ceil().clamp(1.0, MAX_SIDE) as usize
}

/// A length as a count of whole pixels, rounded up; `None` for no length at all or one beyond any picture.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "checked to be within 1..=100000 before the cast")]
fn pixels_up(length: f32) -> Option<usize> {
    let rounded = (length - 1e-3).ceil();
    (rounded.is_finite() && (1.0..=100_000.0).contains(&rounded)).then_some(rounded as usize)
}

/// A pixel position in steps of 1 / [`KEY_STEPS`], for comparing.
#[expect(clippy::cast_possible_truncation, reason = "positions are within a few thousand pixels of the picture")]
fn step(v: f32) -> i32 {
    if v.is_finite() { (v * KEY_STEPS).round().clamp(-1e8, 1e8) as i32 } else { 0 }
}

pub(super) fn expand(rect: &Rect, by: f32) -> Rect {
    Rect::new(rect.x - by, rect.y - by, rect.w + 2.0 * by, rect.h + 2.0 * by)
}

/// The pixels a picture is baked on: a part of the document at some scale.
#[derive(Clone, Copy)]
pub(super) struct Grid {
    /// The document point of the picture's top left corner.
    pub origin: Point,
    /// Picture pixels per document unit.
    pub scale: f32,
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
    /// 2 when painted at twice the size and halved, else 1.
    pub fine: usize,
}

impl Grid {
    /// A grid over `region`, cut down to within `pad` of `limit` (the clip), at up to `scale` pixels per unit.
    // Human: the cut leaves `pad` around the clip because a blur at the cut's edge is wrong (it sees nothing
    // beyond it); with the padding that wrong band is clipped away.
    pub(super) fn new(region: Rect, limit: Option<Rect>, pad: f32, scale: f32, smooth: bool) -> Option<Grid> {
        let region = limit.map_or(region, |limit| intersect(&region, &expand(&limit, pad)));
        let sound = [region.x, region.y, region.w, region.h, scale].iter().all(|v| v.is_finite());
        if !sound || region.w <= 0.0 || region.h <= 0.0 || scale <= 0.0 {
            return None;
        }
        let mut scale = scale.min(MAX_SIDE / region.w.max(region.h));
        let area = region.w * scale * region.h * scale;
        if area > MAX_PIXELS {
            scale *= (MAX_PIXELS / area).sqrt();
        }
        let (w, h) = (pixels(region.w * scale), pixels(region.h * scale));
        Some(Grid { origin: Point::new(region.x, region.y), scale, w, h, fine: if smooth && w * h <= FINE_PIXELS { 2 } else { 1 } })
    }

    /// The same grid painted at its own size.
    fn coarse(self) -> Grid {
        Grid { fine: 1, ..self }
    }

    /// Painted pixels per document unit.
    fn density(&self) -> f32 {
        self.scale * length(self.fine)
    }

    /// A document point in painted pixels.
    pub(super) fn place(&self, p: Point) -> [f32; 2] {
        [(p.x - self.origin.x) * self.density(), (p.y - self.origin.y) * self.density()]
    }

    /// A document rectangle in painted pixels.
    pub(super) fn span(&self, rect: &Rect) -> Span {
        let ([x0, y0], [x1, y1]) = (self.place(Point::new(rect.x, rect.y)), self.place(Point::new(rect.right(), rect.bottom())));
        [x0, y0, x1, y1]
    }

    /// The part of the document the picture covers.
    pub(super) fn rect(&self) -> Rect {
        Rect::new(self.origin.x, self.origin.y, length(self.w) / self.scale, length(self.h) / self.scale)
    }

    /// A clear bitmap to paint on.
    pub(super) fn bitmap(&self) -> Bitmap {
        Bitmap::new(self.w * self.fine, self.h * self.fine)
    }

    /// A painted bitmap at the picture's own size.
    pub(super) fn settle(&self, bitmap: Bitmap) -> Bitmap {
        if self.fine == 2 { bitmap.halved() } else { bitmap }
    }
}

/// egui's font picture, which text meshes are textured with.
#[expect(clippy::redundant_closure_for_method_calls, reason = "the method's path is not general over the font view's lifetime and doesn't compile")]
fn font_picture(ctx: &egui::Context) -> ColorImage {
    ctx.fonts(|fonts| fonts.image())
}

/// Whether an item paints anything.
fn shows(item: &Item) -> bool {
    match item {
        Item::Mesh { mesh, .. } => mesh.vertices.iter().any(|v| v.color.a() > 0),
        Item::Text { text, .. } => !text.style.content.is_empty(),
        Item::Shadow { .. } => false,
    }
}

/// The part of the document an item paints on.
pub(super) fn extent(item: &Item) -> Option<Rect> {
    match item {
        Item::Mesh { mesh, .. } => {
            let first = mesh.vertices.first()?.pos;
            let (mut min, mut max) = (first, first);
            for v in &mesh.vertices {
                (min, max) = (min.min(v.pos), max.max(v.pos));
            }
            Some(Rect::new(min.x, min.y, max.x - min.x, max.y - min.y))
        }
        Item::Text { bounds, .. } => Some(*bounds),
        Item::Shadow { rect, offset, blur, .. } => Some(expand(&rect.translate(offset.x, offset.y), blur / 2.0)),
    }
}

/// The part of the document `items` paint on together.
fn extent_of(items: &[&Item]) -> Option<Rect> {
    items.iter().filter_map(|item| extent(item)).reduce(|a, b| a.union(&b))
}

/// Feeds what makes `item` look the way it does on `grid` to `hasher`. Positions go in relative to the grid, so
/// the same layer somewhere else on the canvas hashes the same.
fn hash_item(item: &Item, grid: &Grid, hasher: &mut DefaultHasher) {
    let at = |p: Point| grid.place(p).map(step);
    let clip_of = |clip: &Option<Rect>| clip.map(|c| grid.span(&c).map(step));
    match item {
        Item::Mesh { mesh, clip, .. } => {
            (0_u8, mesh.texture_id, clip_of(clip), &mesh.indices).hash(hasher);
            for v in &mesh.vertices {
                (at(Point::new(v.pos.x, v.pos.y)), v.color.to_array(), v.uv.x.to_bits(), v.uv.y.to_bits()).hash(hasher);
            }
        }
        Item::Text { text, clip, .. } => {
            let m = text.to_doc.m;
            (1_u8, format!("{:?}", text.style), text.color.to_array(), clip_of(clip)).hash(hasher);
            ([m[0], m[1], m[2], m[3], text.size.0, text.size.1].map(f32::to_bits), at(Point::new(m[4], m[5]))).hash(hasher);
        }
        Item::Shadow { rect, radius, offset, blur, color, clip } => {
            (2_u8, grid.span(rect).map(step), color.to_array(), clip_of(clip)).hash(hasher);
            [*radius, offset.x, offset.y, *blur].map(|v| step(v * grid.scale)).hash(hasher);
        }
    }
}

impl Builder<'_> {
    /// `scenes` (layers with everything in them, back to front) as one picture at this builder's scale: their
    /// boxes and whatever their items paint outside them (shadows, strokes, children). `None` when there is
    /// nothing or it is too large.
    // Human: painted in bands of a few rows at twice the size and halved, so edges are smooth without holding
    // the whole picture at four times its pixels.
    pub(super) fn snapshot(&self, scenes: &[&SceneNode]) -> Option<crate::export::Picture> {
        self.snapshot_at(scenes).map(|(picture, _)| picture)
    }

    /// [`Builder::snapshot`], with the document point of the picture's top left corner.
    pub(super) fn snapshot_at(&self, scenes: &[&SceneNode]) -> Option<(crate::export::Picture, Point)> {
        /// Rows painted at a time.
        const BAND: usize = 128;
        /// Most pixels an exported picture has.
        const MOST: usize = 64_000_000;
        let mut items = Vec::new();
        for scene in scenes {
            self.node(scene, 1.0, None, &mut items);
        }
        let items: Vec<&Item> = items.iter().collect();
        // Human: several layers make one picture: each where it is on the canvas, in a picture that holds them all.
        let frame = scenes.iter().map(|scene| layout::doc_bounds(scene)).reduce(|a, b| a.union(&b))?;
        // Human: what a clipping frame cuts off is not in the picture, so it takes no room in it either.
        let painted = items
            .iter()
            .filter_map(|item| extent(item).map(|e| item.clip().map_or(e, |clip| intersect(&e, &clip))))
            .filter(|e| e.w > 0.0 && e.h > 0.0)
            .reduce(|a, b| a.union(&b));
        let area = painted.map_or(frame, |painted| painted.union(&frame));
        // The picture starts and ends on whole pixels.
        let scale = self.scale;
        let (left, top) = ((area.x * scale).floor() / scale, (area.y * scale).floor() / scale);
        let (w, h) = (pixels_up((area.right() - left) * scale)?, pixels_up((area.bottom() - top) * scale)?);
        if w.checked_mul(h)? > MOST {
            return None;
        }
        let mut rgba = Vec::with_capacity(w * h * 4);
        let mut row = 0;
        while row < h {
            let rows = BAND.min(h - row);
            let grid = Grid { origin: Point::new(left, top + length(row) / scale), scale, w, h: rows, fine: 2 };
            let mut bitmap = grid.bitmap();
            self.paint(&grid, &items, false, &mut bitmap);
            rgba.extend(grid.settle(bitmap).bytes());
            row += rows;
        }
        Some((crate::export::Picture { w, h, rgba }, Point::new(left, top)))
    }

    /// Cuts `inside` (the items of a clipping frame's children, already cut to its box `cut`) to the frame's
    /// rounded `outline`, and adds them to `out`.
    // Human: the canvas can only cut to upright rectangles. So the frame is taken apart: three bands that
    // have no rounded corner in them (the top one between the top corners, the middle one, the bottom one)
    // keep their items as they are, each cut to its band, and so stay sharp at any zoom; each corner is a small
    // picture of what lies there, with everything outside the outline rubbed out.
    // Agent: `radii` are top left, top right, bottom right, bottom left; `bounds` is the frame's box.
    pub(super) fn round_off(&self, outline: &Item, radii: [f32; 4], bounds: Rect, cut: Rect, inside: Vec<Item>, out: &mut Vec<Item>) {
        let (top, bottom) = (radii[0].max(radii[1]), radii[2].max(radii[3]));
        let bands = [
            Rect::new(bounds.x + radii[0], bounds.y, bounds.w - radii[0] - radii[1], top),
            Rect::new(bounds.x, bounds.y + top, bounds.w, bounds.h - top - bottom),
            Rect::new(bounds.x + radii[3], bounds.bottom() - bottom, bounds.w - radii[3] - radii[2], bottom),
        ];
        let corners = [
            Rect::new(bounds.x, bounds.y, radii[0], top),
            Rect::new(bounds.right() - radii[1], bounds.y, radii[1], top),
            Rect::new(bounds.right() - radii[2], bounds.bottom() - bottom, radii[2], bottom),
            Rect::new(bounds.x, bounds.bottom() - bottom, radii[3], bottom),
        ];
        // The part of `area` an item paints on, if any.
        let reaches = |item: &Item, area: &Rect| {
            let part = intersect(&item.clip().unwrap_or(cut), area);
            (part.w > 0.0 && part.h > 0.0 && extent(item).is_some_and(|e| e.intersects(&part))).then_some(part)
        };
        for corner in corners.iter().map(|corner| intersect(corner, &cut)).filter(|corner| corner.w > 0.0 && corner.h > 0.0) {
            let here: Vec<&Item> = inside.iter().filter(|item| reaches(item, &corner).is_some()).collect();
            let Some(grid) = Grid::new(corner, None, 0.0, self.scale, true).filter(|_| !here.is_empty()) else { continue };
            let key = Self::key("corner", &grid, &[(&here, false), (&[outline], true)], &[], Color32::TRANSPARENT);
            let texture = self.baked.texture(self.ctx, key, grid.rect(), || {
                let mut bitmap = grid.bitmap();
                self.paint(&grid, &here, false, &mut bitmap);
                let mut bitmap = grid.settle(bitmap);
                let mut solid = grid.bitmap();
                self.paint(&grid, &[outline], true, &mut solid);
                for (pixel, cover) in bitmap.px.iter_mut().zip(&grid.settle(solid).px) {
                    *pixel = pixel.map(|c| c * cover[3]);
                }
                bitmap
            });
            out.push(Self::picture(texture, &grid, Point::default(), 1.0, Some(corner)));
        }
        for item in inside {
            let parts: Vec<Rect> = bands.iter().filter_map(|band| reaches(&item, band)).collect();
            let last = parts.len().saturating_sub(1);
            let mut item = Some(item);
            for (index, part) in parts.into_iter().enumerate() {
                // The item itself goes into its last band; the bands before get a copy.
                let whole = if index == last { item.take() } else { item.clone() };
                out.extend(whole.map(|whole| whole.clipped_to(part)));
            }
        }
    }

    /// The effects of `scene` that are switched on, variables looked up.
    // Human: a shadow is its offset, colour (black when unset) and `blur`; `spread` is
    // not read at all.
    pub(super) fn effects_of(&self, scene: &SceneNode) -> Effects {
        let mut found = Effects::default();
        let theme = &scene.theme;
        let number = |v: Option<&Value>| -> f32 {
            match v {
                Some(Value::Number(n)) => n.as_f64().map_or(0.0, narrow),
                Some(Value::String(s)) => self.vars.number(&Num::Text(s.clone()), theme).unwrap_or(0.0),
                _ => 0.0,
            }
        };
        for effect in effects::list(scene.node.effect.as_ref()) {
            let off = match effect.get("enabled") {
                Some(Value::Bool(on)) => !on,
                Some(Value::String(name)) => self.vars.flag(&Flag::Variable(name.clone()), theme) == Some(false),
                _ => false,
            };
            if off {
                continue;
            }
            match EffectKind::of(&effect) {
                Some(kind @ (EffectKind::DropShadow | EffectKind::InnerShadow)) => {
                    let color = effect.get("color").and_then(Value::as_str).and_then(|c| self.vars.color(c, theme)).unwrap_or(Rgba::BLACK);
                    if color.a == 0 {
                        continue;
                    }
                    let offset = effect.get("offset").map_or(Point::default(), |o| Point::new(number(o.get("x")), number(o.get("y"))));
                    let shadow = Shadow { offset, blur: number(effect.get("blur")).max(0.0), color };
                    if kind == EffectKind::DropShadow { found.drops.push(shadow) } else { found.inners.push(shadow) }
                }
                Some(EffectKind::LayerBlur) => found.layer_blur = Some(sigma(number(effect.get("radius")))).filter(|s| *s > 0.0),
                Some(EffectKind::BackgroundBlur) => found.background_blur = Some(sigma(number(effect.get("radius")))).filter(|s| *s > 0.0),
                Some(EffectKind::Shader) => found.shaders.push(effect),
                None => {}
            }
        }
        found
    }

    /// Adds `scene`'s effects to its items. `marks` are where in `out` its fills start, its strokes start and
    /// its children start; everything from the first mark on is the node.
    // Human: the order is: background blur, drop shadows, fill, inner
    // shadows, stroke, children. A node without a fill has its children as its shape: they cast its shadow, and
    // its inner shadows fall on them. A layer blur blurs all of it but the background blur. Shader effects run
    // last, over the node with its other effects, each on the picture the one before it made.
    // Human: a material is the node's background: it comes after the node's shadows and before its fill, and a
    // node that has no fill takes its shape from its outline instead of from its children.
    pub(super) fn apply_effects(
        &self,
        scene: &SceneNode,
        effects: &Effects,
        pane: Option<&Pane>,
        marks: [usize; 3],
        look: &Look,
        out: &mut Vec<Item>,
    ) {
        let [start, fill_end, own_end] = marks;
        let kids: Vec<Item> = out.drain(own_end..).collect();
        let strokes: Vec<Item> = out.drain(fill_end..).collect();
        let fills: Vec<Item> = out.drain(start..).collect();
        let has_fill = fills.iter().any(shows);
        let outline: Vec<&Item> = pane.filter(|_| !has_fill).map(|pane| pane.mask.iter().collect()).unwrap_or_default();
        let interior: Vec<&Item> = if !outline.is_empty() {
            outline.clone()
        } else if has_fill {
            fills.iter().collect()
        } else {
            kids.iter().collect()
        };
        let full: Vec<&Item> = interior.iter().copied().chain(&strokes).collect();
        let backdrop = effects.background_blur.and_then(|sigma| self.backdrop(&out.iter().collect::<Vec<_>>(), &interior, sigma, look));
        // Human: egui draws the soft shadow of a box itself, which costs nothing. That shadow also lies under the
        // box, so it only serves a box that hides it; any other shape gets a baked shadow with the shape cut out.
        let plain = look.radius.filter(|_| look.opaque && has_fill && effects.layer_blur.is_none() && scene.to_doc.is_axis_aligned());
        let mut layer: Vec<Item> = effects
            .drops
            .iter()
            .filter_map(|shadow| match plain {
                Some(radius) => Some(Item::Shadow {
                    rect: layout::doc_bounds(scene),
                    radius,
                    offset: shadow.offset,
                    blur: shadow.blur,
                    color: color32(shadow.color.with_opacity(look.opacity)),
                    clip: look.clip,
                }),
                None => self.drop_shadow(&full, shadow, look),
            })
            .collect();
        let mut inner: Vec<Item> = effects.inners.iter().filter_map(|shadow| self.inner_shadow(&interior, shadow, look)).collect();
        out.extend(backdrop);
        // A material needs a shape: its outline, or what its fill paints.
        if let Some(pane) = pane.filter(|_| has_fill || !outline.is_empty()) {
            layer.extend(self.material(scene, pane, &interior, &out.iter().collect::<Vec<_>>(), look));
        }
        layer.extend(fills);
        if has_fill {
            layer.append(&mut inner);
        }
        layer.extend(strokes);
        layer.extend(kids);
        layer.append(&mut inner);
        if let Some(sigma) = effects.layer_blur
            && let Some(picture) = self.layer_blur(&layer.iter().collect::<Vec<_>>(), sigma, look)
        {
            layer = vec![picture];
        }
        if !effects.shaders.is_empty()
            && let Some(picture) = self.shaded(scene, &effects.shaders, &layer.iter().collect::<Vec<_>>(), &out.iter().collect::<Vec<_>>(), look)
        {
            layer = vec![picture];
        }
        out.extend(layer);
    }

    /// One shader run over `object`: its samplers are given the picture each asks for, and the GPU draws a new
    /// picture of the same size. `on_node` maps texture coordinates onto the node's box in `object` and
    /// `background`; `block` makes the uniform bytes once the samplers' maps are known.
    fn pass(
        &self,
        program: &Program,
        sources: &[(String, Source)],
        object: &Bitmap,
        background: Option<&[u8]>,
        on_node: [f32; 4],
        block: &dyn Fn(&[SamplerMap]) -> Vec<u8>,
    ) -> Result<Bitmap, String> {
        const WHOLE: [f32; 4] = [1.0, 1.0, 0.0, 0.0];
        // A picture file fills texture coordinates 0–1 itself and is only turned upright.
        const UPRIGHT: [f32; 4] = [1.0, -1.0, 0.0, 1.0];
        let whole = |pixels: usize| u32::try_from(pixels).map_err(|e| e.to_string());
        let size = (whole(object.w)?, whole(object.h)?);
        let now = object.bytes();
        let clear = [0_u8; 4];
        let files: Vec<Option<Arc<ColorImage>>> = sources
            .iter()
            .map(|(_, source)| match source {
                Source::File(url) => self.images.get(self.ctx, self.folder, url).and_then(|p| self.images.pixels(p.texture)),
                _ => None,
            })
            .collect();
        let mut maps = Vec::new();
        let mut inputs = Vec::new();
        for ((name, source), file) in sources.iter().zip(&files) {
            let nothing = (gpu::Input { rgba: &clear, w: 1, h: 1, mirror: false }, WHOLE);
            let (input, map) = match (source, background, file) {
                (Source::Object, ..) => (gpu::Input { rgba: &now, w: size.0, h: size.1, mirror: false }, on_node),
                (Source::Background, Some(back), _) => (gpu::Input { rgba: back, w: size.0, h: size.1, mirror: true }, on_node),
                (Source::File(_), _, Some(file)) => {
                    (gpu::Input { rgba: file.as_raw(), w: whole(file.size[0])?, h: whole(file.size[1])?, mirror: false }, UPRIGHT)
                }
                _ => nothing,
            };
            maps.push((shader::map_member(name), map));
            inputs.push(input);
        }
        let bytes = gpu::run(program, size, &block(&maps), &inputs)?;
        Bitmap::from_bytes(object.w, object.h, &bytes).ok_or_else(|| "the GPU returned a picture of the wrong size".to_owned())
    }

    /// `layer` run through the node's shader effects, as one picture. `None` when no shader can run (the file
    /// is missing or doesn't compile, or there is no GPU); the layer is then drawn as it is.
    // Human: the node with its other effects is the `pencil:object` picture,
    // what is behind it `pencil:background` (mirrored past its edges); `gl_FragCoord` and `@resolution` are in
    // the node's own units, y up from its bottom edge, and texture coordinates 0–1 span the node's box, so the
    // picture's margin (shadows, strokes) is reached by going outside 0–1. `@time` and `@mouse` stay 0.
    // Agent: a rotated node is given the box around it, since pictures are baked upright.
    fn shaded(&self, scene: &SceneNode, shaders: &[Value], layer: &[&Item], behind: &[&Item], look: &Look) -> Option<Item> {
        let frame = layout::doc_bounds(scene);
        let pad = 1.0 / self.scale;
        let region = extent_of(layer).map_or(frame, |e| e.union(&frame));
        let grid = Grid::new(expand(&region, pad), look.clip, pad, self.scale, false)?;
        let passes: Vec<(Arc<Program>, &Value)> = shaders
            .iter()
            .filter_map(|effect| {
                let url = effect.get("url").and_then(Value::as_str).unwrap_or("");
                let path = images::resolve(self.folder, url)?;
                match shader::load(&path) {
                    Ok(program) => Some((program, effect)),
                    Err(error) => {
                        log::warn!("shader effect: {}: {error}", path.display());
                        None
                    }
                }
            })
            .collect();
        if passes.is_empty() {
            return None;
        }
        let variable = |name: &str| self.vars.lookup(name, &scene.theme).cloned();
        let given = |effect: &Value, name: &str| effect.get("uniforms").and_then(|u| u.get(name)).cloned();
        let value = |effect: &Value, uniform: &shader::Uniform| match uniform.role {
            Role::Resolution => [frame.w, frame.h, 0.0, 0.0],
            Role::Time | Role::Mouse | Role::Sdf => [0.0; 4],
            Role::Plain => shader::numbers(uniform, given(effect, &uniform.name).as_ref(), &variable),
        };
        let sources = |program: &Program, effect: &Value| -> Vec<(String, Source)> {
            let samplers = program.uniforms.iter().filter(|u| u.ty == Type::Sampler);
            samplers.map(|u| (u.name.clone(), shader::source(u, given(effect, &u.name).as_ref()))).collect()
        };
        let uses_background = passes.iter().any(|(program, effect)| sources(program, effect).iter().any(|(_, s)| *s == Source::Background));
        let area = grid.rect();
        let behind: Vec<&Item> = if uses_background {
            behind.iter().copied().filter(|item| extent(item).is_some_and(|e| e.intersects(&area))).collect()
        } else {
            Vec::new()
        };
        // Where the node's box lies on the picture: its left, top, right and bottom edges in pixels.
        let [left, top, right, bottom] = grid.span(&frame);
        let (w, h) = (length(grid.w), length(grid.h));

        let mut hasher = DefaultHasher::new();
        for (program, effect) in &passes {
            program.glsl.hash(&mut hasher);
            for uniform in &program.uniforms {
                value(effect, uniform).map(f32::to_bits).hash(&mut hasher);
            }
            for (_, source) in sources(program, effect) {
                source.hash(&mut hasher);
            }
        }
        let tag = format!("shader-{:016x}", hasher.finish());
        let key = Self::key(&tag, &grid, &[(layer, false), (&behind, false)], &[left, top, right, bottom], self.background);
        let texture = self.baked.texture(self.ctx, key, grid.rect(), || {
            let mut object = grid.bitmap();
            self.paint(&grid, layer, false, &mut object);
            let background = uses_background.then(|| {
                let mut back = grid.bitmap();
                back.fill(raster::pixel(self.background));
                self.paint(&grid, &behind, false, &mut back);
                back.bytes()
            });
            // Human: texture coordinates 0–1 span the node's box with v = 0 at its bottom; on the picture that
            // is this scale and offset.
            let on_node = [(right - left) / w, (top - bottom) / h, left / w, bottom / h];
            for (program, effect) in &passes {
                let block = |maps: &[SamplerMap]| program.block([left, bottom, grid.scale, 0.0], maps, |uniform| value(effect, uniform));
                match self.pass(program, &sources(program, effect), &object, background.as_deref(), on_node, &block) {
                    Ok(drawn) => object = drawn,
                    Err(error) => log::warn!("shader effect: {error}"),
                }
            }
            object
        });
        // The items already carry the node's opacity.
        Some(Self::picture(texture, &grid, Point::default(), 1.0, look.clip))
    }

    /// Paints `items` onto `bitmap`, which covers `grid`. `solid` paints their shapes, fully opaque.
    pub(super) fn paint(&self, grid: &Grid, items: &[&Item], solid: bool, bitmap: &mut Bitmap) {
        // Human: texts are laid out first, so the font picture read after that holds all their letters.
        // Human: a letter new to the font picture can make the picture grow, and a mesh made before that points
        // at where its letters were. So the texts are laid out twice: once to bring every letter into the
        // picture, then, with the picture settled, for the meshes that are painted.
        let lay_out = || -> Vec<Vec<Mesh>> {
            items.iter().map(|item| if let Item::Text { text, .. } = item { self.text_meshes(text, grid) } else { Vec::new() }).collect()
        };
        let first = lay_out();
        let texts = if first.iter().any(|meshes| !meshes.is_empty()) { lay_out() } else { first };
        let letters = texts.iter().any(|meshes| !meshes.is_empty()).then(|| font_picture(self.ctx));
        for (item, text) in items.iter().zip(&texts) {
            match item {
                Item::Mesh { mesh, clip, .. } => {
                    let picture = if mesh.texture_id == TextureId::default() {
                        None
                    } else {
                        self.baked.pixels(mesh.texture_id).or_else(|| self.images.pixels(mesh.texture_id))
                    };
                    bitmap.triangles(mesh, |p| grid.place(Point::new(p.x, p.y)), clip.map(|c| grid.span(&c)), picture.as_deref(), solid);
                }
                Item::Text { clip, .. } => {
                    for mesh in text {
                        bitmap.triangles(mesh, |p| [p.x, p.y], clip.map(|c| grid.span(&c)), letters.as_ref(), solid);
                    }
                }
                // A box's own soft shadow is no part of its shape.
                Item::Shadow { rect, radius, offset, blur, color, clip } if !solid => {
                    let span = grid.span(&rect.translate(offset.x, offset.y));
                    bitmap.soft_box(span, radius * grid.density(), blur * grid.density(), raster::pixel(*color), clip.map(|c| grid.span(&c)));
                }
                Item::Shadow { .. } => {}
            }
        }
    }

    /// A text as triangles in `grid`'s painted pixels, textured with egui's font picture.
    // Agent: lays the text out through the egui context (its letters join the font picture) and tessellates it
    // there; MAIN-THREAD.
    fn text_meshes(&self, text: &TextItem, grid: &Grid) -> Vec<Mesh> {
        let per_point = self.ctx.pixels_per_point();
        // Points per document unit at which one painted pixel is one physical pixel.
        let z = grid.density() / per_point;
        if text.style.content.is_empty() || text.style.size * z < 1.0 {
            return Vec::new();
        }
        // Human: text larger than the font atlas holds is laid out smaller and its triangles are scaled up
        // (see `glyph_scale`); a poster's headline must not vanish from the picture of it.
        let (laid, grow) = glyph_scale(text.style.size, z);
        let galley = self.ctx.fonts_mut(|fonts| fonts.layout_job(text_job(&text.style, text.color, laid)));
        let (anchor, angle) = text_anchor(text, galley.size().y / laid);
        let [x, y] = grid.place(anchor);
        let shape = TextShape::new(Pos2::ZERO, galley, text.color).with_angle(angle);
        self.ctx
            .tessellate(vec![ClippedShape::new(egui::Rect::EVERYTHING, shape)], per_point)
            .into_iter()
            .filter_map(|clipped| match clipped.primitive {
                Primitive::Mesh(mut mesh) => {
                    for v in &mut mesh.vertices {
                        v.pos = Pos2::new(v.pos.x * grow * per_point + x, v.pos.y * grow * per_point + y);
                    }
                    Some(mesh)
                }
                Primitive::Callback(_) => None,
            })
            .collect()
    }

    /// A hash of everything a picture is made from: the effect, its numbers and colour, and each layer of items
    /// with whether it is painted solid.
    pub(super) fn key(tag: &str, grid: &Grid, layers: &[(&[&Item], bool)], numbers: &[f32], color: Color32) -> u64 {
        let mut hasher = DefaultHasher::new();
        (tag, grid.w, grid.h, grid.fine, grid.scale.to_bits(), color.to_array()).hash(&mut hasher);
        for number in numbers {
            step(*number).hash(&mut hasher);
        }
        for (items, solid) in layers {
            (items.len(), solid).hash(&mut hasher);
            for item in *items {
                hash_item(item, grid, &mut hasher);
            }
        }
        hasher.finish()
    }

    /// The picture `texture`, baked on `grid`, as an item: moved by `shift` and faded to `opacity`.
    pub(super) fn picture(texture: TextureId, grid: &Grid, shift: Point, opacity: f32, clip: Option<Rect>) -> Item {
        let bounds = grid.rect().translate(shift.x, shift.y);
        let level = to_u8(opacity.clamp(0.0, 1.0) * 255.0);
        let color = Color32::from_rgba_premultiplied(level, level, level, level);
        let corner = |x: f32, y: f32, u: f32, v: f32| Vertex { pos: Pos2::new(x, y), uv: Pos2::new(u, v), color };
        let vertices = vec![
            corner(bounds.x, bounds.y, 0.0, 0.0),
            corner(bounds.right(), bounds.y, 1.0, 0.0),
            corner(bounds.right(), bounds.bottom(), 1.0, 1.0),
            corner(bounds.x, bounds.bottom(), 0.0, 1.0),
        ];
        Item::Mesh { mesh: Mesh { indices: vec![0, 1, 2, 0, 2, 3], vertices, texture_id: texture }, bounds, clip }
    }

    /// The shadow `shape` casts: its blurred outline in the shadow's colour, moved by the offset.
    // Human: the shape is drawn solid, whatever its fill's opacity, and blurred; when the shape
    // doesn't hide what is under it, the shape itself is cut out of the shadow, so a see-through layer shows no
    // shadow through its fill.
    pub(super) fn drop_shadow(&self, shape: &[&Item], shadow: &Shadow, look: &Look) -> Option<Item> {
        let sigma = sigma(shadow.blur);
        let pad = sigma * REACH + 1.0 / self.scale;
        // The shadow is clipped where it lands, which is `offset` away from the shape it is made from.
        let limit = look.clip.map(|c| c.translate(-shadow.offset.x, -shadow.offset.y));
        let grid = Grid::new(expand(&extent_of(shape)?, pad), limit, pad, self.scale, sigma * self.scale < 1.0)?;
        let (sigma, offset) = (sigma * grid.scale, [shadow.offset.x * grid.scale, shadow.offset.y * grid.scale]);
        let color = color32(shadow.color);
        let cut = !look.opaque;
        let key = Self::key(if cut { "drop-cut" } else { "drop" }, &grid, &[(shape, true)], &[sigma, offset[0], offset[1]], color);
        let texture = self.baked.texture(self.ctx, key, grid.rect(), || {
            let mut solid = grid.bitmap();
            self.paint(&grid, shape, true, &mut solid);
            let solid = grid.settle(solid);
            let mut soft = solid.clone();
            soft.blur(sigma);
            let tint = raster::pixel(color);
            for y in 0..soft.h {
                for x in 0..soft.w {
                    // This pixel of the shadow lands `offset` further on, where the shape may be.
                    let covered = if cut { solid.sample(length(x) + 0.5 + offset[0], length(y) + 0.5 + offset[1])[3] } else { 0.0 };
                    if let Some(pixel) = soft.px.get_mut(y * soft.w + x) {
                        let strength = pixel[3] * (1.0 - covered);
                        *pixel = tint.map(|c| c * strength);
                    }
                }
            }
            soft
        });
        Some(Self::picture(texture, &grid, shadow.offset, look.opacity, look.clip))
    }

    /// The shadow that falls inside `shape` from its own edge.
    // Human: the shape in the shadow's colour, with the shape's blurred outline, moved by the
    // offset, cut out of it. What is left is the rim the moved outline no longer covers.
    fn inner_shadow(&self, shape: &[&Item], shadow: &Shadow, look: &Look) -> Option<Item> {
        let sigma = sigma(shadow.blur);
        let pad = sigma * REACH + shadow.offset.x.abs().max(shadow.offset.y.abs()) + 1.0 / self.scale;
        let grid = Grid::new(expand(&extent_of(shape)?, 1.0 / self.scale), look.clip, pad, self.scale, true)?;
        let (sigma, offset) = (sigma * grid.scale, [shadow.offset.x * grid.scale, shadow.offset.y * grid.scale]);
        let color = color32(shadow.color);
        let key = Self::key("inner", &grid, &[(shape, true)], &[sigma, offset[0], offset[1]], color);
        let texture = self.baked.texture(self.ctx, key, grid.rect(), || {
            let mut solid = grid.bitmap();
            self.paint(&grid, shape, true, &mut solid);
            let mut solid = grid.settle(solid);
            let mut soft = solid.clone();
            soft.blur(sigma);
            let tint = raster::pixel(color);
            for y in 0..solid.h {
                for x in 0..solid.w {
                    let lit = soft.sample(length(x) + 0.5 - offset[0], length(y) + 0.5 - offset[1])[3];
                    if let Some(pixel) = solid.px.get_mut(y * solid.w + x) {
                        let strength = pixel[3] * (1.0 - lit);
                        *pixel = tint.map(|c| c * strength);
                    }
                }
            }
            solid
        });
        Some(Self::picture(texture, &grid, Point::default(), look.opacity, look.clip))
    }

    /// `layer` (a node with its shadows and children) as one blurred picture.
    fn layer_blur(&self, layer: &[&Item], sigma: f32, look: &Look) -> Option<Item> {
        let pad = sigma * REACH + 1.0 / self.scale;
        let grid = Grid::new(expand(&extent_of(layer)?, pad), look.clip, pad, self.scale, sigma * self.scale < 1.0)?;
        let sigma = sigma * grid.scale;
        let key = Self::key("layer", &grid, &[(layer, false)], &[sigma], Color32::TRANSPARENT);
        let texture = self.baked.texture(self.ctx, key, grid.rect(), || {
            let mut bitmap = grid.bitmap();
            self.paint(&grid, layer, false, &mut bitmap);
            let mut bitmap = grid.settle(bitmap);
            bitmap.blur(sigma);
            bitmap
        });
        // The items already carry the node's opacity.
        Some(Self::picture(texture, &grid, Point::default(), 1.0, look.clip))
    }

    /// What `behind` paints (on the canvas colour), blurred and cut to `shape`.
    // Human: everything painted before the node is what lies behind it, since items are in paint order.
    fn backdrop(&self, behind: &[&Item], shape: &[&Item], sigma: f32, look: &Look) -> Option<Item> {
        let pad = sigma * REACH + 1.0 / self.scale;
        let grid = Grid::new(expand(&extent_of(shape)?, pad), look.clip, pad, self.scale, true)?;
        let area = grid.rect();
        let behind: Vec<&Item> = behind.iter().copied().filter(|item| extent(item).is_some_and(|e| e.intersects(&area))).collect();
        let sigma = sigma * grid.scale;
        let key = Self::key("backdrop", &grid, &[(&behind, false), (shape, true)], &[sigma], self.background);
        let texture = self.baked.texture(self.ctx, key, grid.rect(), || {
            let mut back = grid.coarse().bitmap();
            back.fill(raster::pixel(self.background));
            self.paint(&grid.coarse(), &behind, false, &mut back);
            back.blur(sigma);
            let mut solid = grid.bitmap();
            self.paint(&grid, shape, true, &mut solid);
            let solid = grid.settle(solid);
            for (pixel, cover) in back.px.iter_mut().zip(&solid.px) {
                *pixel = pixel.map(|c| c * cover[3]);
            }
            back
        });
        Some(Self::picture(texture, &grid, Point::default(), look.opacity, look.clip))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_covers_its_region_and_shrinks_to_the_pixel_budget() {
        let grid = Grid::new(Rect::new(10.0, 20.0, 100.0, 50.0), None, 0.0, 2.0, true).unwrap();
        assert_eq!((grid.w, grid.h, grid.fine), (200, 100, 2));
        assert_eq!(grid.place(Point::new(11.0, 21.0)), [4.0, 4.0]);
        assert_eq!(grid.coarse().place(Point::new(11.0, 21.0)), [2.0, 2.0]);
        let covered = grid.rect();
        assert!((covered.w - 100.0).abs() < 1e-4 && (covered.h - 50.0).abs() < 1e-4);
        // A huge region is baked coarser instead of larger.
        let huge = Grid::new(Rect::new(0.0, 0.0, 10_000.0, 10_000.0), None, 0.0, 4.0, true).unwrap();
        assert!(length(huge.w) * length(huge.h) <= MAX_PIXELS * 1.01 && huge.fine == 1);
        assert!(huge.rect().w >= 10_000.0 - 1e-2);
        assert!(Grid::new(Rect::new(0.0, 0.0, 0.0, 5.0), None, 0.0, 1.0, false).is_none());
        assert!(Grid::new(Rect::new(0.0, 0.0, f32::NAN, 5.0), None, 0.0, 1.0, false).is_none());
    }

    #[test]
    fn a_grid_is_cut_to_the_clip_with_room_for_the_blur() {
        let grid = Grid::new(Rect::new(0.0, 0.0, 500.0, 500.0), Some(Rect::new(100.0, 100.0, 50.0, 50.0)), 10.0, 1.0, false).unwrap();
        assert_eq!((grid.origin, grid.w, grid.h), (Point::new(90.0, 90.0), 70, 70));
    }

    #[test]
    fn a_moved_layer_hashes_the_same_and_a_changed_one_does_not() {
        let square = |x: f32, alpha: u8| {
            let color = Color32::from_rgba_premultiplied(0, 0, 0, alpha);
            let corner = |x: f32, y: f32| Vertex { pos: Pos2::new(x, y), uv: egui::epaint::WHITE_UV, color };
            let vertices = vec![corner(x, 0.0), corner(x + 10.0, 0.0), corner(x + 10.0, 10.0), corner(x, 10.0)];
            Item::Mesh {
                mesh: Mesh { indices: vec![0, 1, 2, 0, 2, 3], vertices, ..Default::default() },
                bounds: Rect::new(x, 0.0, 10.0, 10.0),
                clip: None,
            }
        };
        let key = |item: &Item| {
            let grid = Grid::new(extent(item).unwrap(), None, 0.0, 2.0, true).unwrap();
            Builder::key("test", &grid, &[(&[item], true)], &[1.0], Color32::BLACK)
        };
        assert_eq!(key(&square(0.0, 255)), key(&square(1234.5, 255)));
        assert_ne!(key(&square(0.0, 255)), key(&square(0.0, 128)));
    }
}
