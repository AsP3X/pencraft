//! Draws the laid-out scene on the canvas.
//!
//! Shapes are tessellated once per scene change into meshes in document coordinates (a display list); each
//! frame only moves their vertices to the screen. Text is laid out each frame at the zoomed size, which egui
//! caches.

use egui::epaint::text::VariationCoords;
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId, Painter, Pos2, Shape, Stroke};
use lyon_tessellation::math::point;
use lyon_tessellation::path::Path;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, LineCap, LineJoin, StrokeOptions, StrokeTessellator, StrokeVertex,
    VertexBuffers,
};
use pen_doc::color::Rgba;
use pen_doc::geom::{Affine, Point, Rect};
use pen_doc::layout::{self, NOTE_PADDING, NOTE_RADIUS, TextAlign, TextStyle, TextVAlign};
use pen_doc::model::{Corners, Kind, Node, Paint, StrokeWidth};
use pen_doc::path::{self as svg, Segment};
use pen_doc::scene::SceneNode;
use pen_doc::value::{Num, narrow};
use pen_doc::vars::{ThemeContext, Vars};
use serde_json::Value;

use crate::editor::Camera;
use crate::icons;
use crate::images::{ImageStore, Picture};

mod bake;
mod glass;
mod svg_out;

pub use bake::Seen;

/// Default sticky-note colour.
pub const NOTE_COLOR: Rgba = Rgba { r: 0xfe, g: 0xe4, b: 0x84, a: 0xff };
/// Contrast below which note text turns white.
const NOTE_TEXT_CONTRAST: f32 = 4.6;
/// Font families registered at start-up (see `fonts.rs`).
pub const BOLD_FAMILY: &str = "pencraft-bold";
/// See [`BOLD_FAMILY`].
pub const ITALIC_FAMILY: &str = "pencraft-italic";
/// Corner radius of a browser node that sets none.
pub const BROWSER_RADIUS: f32 = 16.0;
/// Glyphs are laid out no larger than this many screen points (larger ones would overflow egui's font atlas);
/// text that is to be larger is laid out at this size and its triangles are scaled up.
const MAX_GLYPH_PX: f32 = 600.0;

/// The scale to lay text of `size` out at when it is drawn at `z` screen points per unit, and how much its
/// triangles have to grow afterwards (1 when it fits the font atlas as it is).
fn glyph_scale(size: f32, z: f32) -> (f32, f32) {
    let laid = z.min(MAX_GLYPH_PX / size.max(f32::EPSILON));
    (laid, z / laid)
}

/// One thing to draw, in document coordinates.
#[derive(Clone)]
enum Item {
    /// Filled triangles (shapes, strokes).
    Mesh { mesh: Mesh, bounds: Rect, clip: Option<Rect> },
    /// A text block.
    Text { text: Box<TextItem>, bounds: Rect, clip: Option<Rect> },
    /// A soft drop shadow under an unrotated box that hides what is under it.
    Shadow { rect: Rect, radius: f32, offset: Point, blur: f32, color: Color32, clip: Option<Rect> },
}

#[derive(Clone)]
struct TextItem {
    style: TextStyle,
    color: Color32,
    to_doc: Affine,
    size: (f32, f32),
}

/// A picture of layers with what is needed to measure it.
pub struct Survey {
    /// The picture.
    pub picture: crate::export::Picture,
    /// The document point of its top left corner.
    pub origin: Point,
    /// Its pixels per document unit.
    pub scale: f32,
    /// What each material in it came to.
    pub materials: Vec<Seen>,
}

impl Item {
    /// What the item is cut to.
    fn clip(&self) -> Option<Rect> {
        match self {
            Item::Mesh { clip, .. } | Item::Text { clip, .. } | Item::Shadow { clip, .. } => *clip,
        }
    }

    /// The item, cut to `to` instead.
    fn clipped_to(mut self, to: Rect) -> Item {
        match &mut self {
            Item::Mesh { clip, .. } | Item::Text { clip, .. } | Item::Shadow { clip, .. } => *clip = Some(to),
        }
        self
    }
}

/// The tessellated scene of one editor, rebuilt when its scene or zoom level changes.
#[derive(Default)]
pub struct DisplayList {
    generation: u64,
    zoom_bucket: i32,
    /// The document's folder when the list was built; relative image paths start there.
    folder: Option<std::path::PathBuf>,
    /// The canvas colour the list was built on; a background blur blurs it too.
    background: Color32,
    items: Vec<Item>,
    images: ImageStore,
    /// The pictures effects were baked into, kept from one build to the next.
    baked: bake::Baked,
}

impl DisplayList {
    /// Rebuilds the list when the scene changed or the zoom moved to another power of two.
    // Human: Curves are flattened to a tolerance that depends on the zoom, so zooming far in re-tessellates
    // once per doubling instead of showing facets.
    // Human: Effects (blurs, and the shadows egui can't draw) are baked into pictures at the pixel size of that
    // power of two, so they stay sharp while zooming in; `background` is the canvas colour behind the design.
    #[expect(clippy::too_many_arguments, reason = "the scene, what it was built from, and where its pictures live")]
    pub fn update(
        &mut self,
        scene: &[SceneNode],
        generation: u64,
        zoom: f32,
        vars: Vars<'_>,
        ctx: &egui::Context,
        folder: Option<&std::path::Path>,
        background: Color32,
    ) {
        let bucket = zoom_bucket(zoom);
        if !self.items.is_empty()
            && self.generation == generation
            && self.zoom_bucket == bucket
            && self.folder.as_deref() == folder
            && self.background == background
        {
            return;
        }
        self.folder = folder.map(std::path::Path::to_path_buf);
        self.items.clear();
        self.generation = generation;
        self.zoom_bucket = bucket;
        self.background = background;
        self.baked.begin();
        let builder = Builder {
            vars,
            tolerance: (0.25 / zoom).clamp(0.002, 4.0),
            images: &self.images,
            ctx,
            folder,
            baked: &self.baked,
            background,
            scale: 2.0_f32.powi(bucket) * ctx.pixels_per_point(),
            merged: std::cell::Cell::new(false),
        };
        for node in scene {
            builder.node(node, 1.0, None, &mut self.items);
        }
        self.baked.finish();
    }

    /// `scene` (a layer with everything in it) drawn as a picture at `scale` pixels per unit, with its shadows
    /// and whatever else reaches past its box. `None` when it is empty or too large.
    // Agent: MAIN-THREAD; uses its own effect pictures, dropped when it returns.
    pub fn snapshot(
        &self,
        scenes: &[&SceneNode],
        scale: f32,
        vars: Vars<'_>,
        ctx: &egui::Context,
        folder: Option<&std::path::Path>,
    ) -> Option<crate::export::Picture> {
        let baked = bake::Baked::default();
        baked.begin();
        self.builder(scale, vars, ctx, folder, &baked).snapshot(scenes)
    }

    /// [`DisplayList::snapshot`] for measuring: the picture with where it lies and what its materials came to.
    // Agent: MAIN-THREAD; bakes every effect anew, so each material is seen.
    pub fn survey(&self, scenes: &[&SceneNode], scale: f32, vars: Vars<'_>, ctx: &egui::Context, folder: Option<&std::path::Path>) -> Option<Survey> {
        let baked = bake::Baked::default();
        baked.begin();
        let (picture, origin) = self.builder(scale, vars, ctx, folder, &baked).snapshot_at(scenes)?;
        Some(Survey { picture, origin, scale, materials: baked.seen() })
    }

    /// `scenes` together as one SVG file, or `None` when they have no size.
    pub fn svg(&self, scenes: &[&SceneNode], vars: Vars<'_>, ctx: &egui::Context, folder: Option<&std::path::Path>) -> Option<String> {
        let baked = bake::Baked::default();
        self.builder(1.0, vars, ctx, folder, &baked).svg(scenes)
    }

    /// A builder for exports: on a clear background, at `scale` pixels per unit.
    fn builder<'a>(
        &'a self,
        scale: f32,
        vars: Vars<'a>,
        ctx: &'a egui::Context,
        folder: Option<&'a std::path::Path>,
        baked: &'a bake::Baked,
    ) -> Builder<'a> {
        Builder {
            vars,
            tolerance: (0.25 / scale).clamp(0.002, 4.0),
            images: &self.images,
            ctx,
            folder,
            baked,
            background: Color32::TRANSPARENT,
            scale,
            merged: std::cell::Cell::new(false),
        }
    }

    /// Draws everything visible in `view` (screen rectangle) through `camera`.
    // Agent: HOT-PATH per frame; ALLOCATES one transformed mesh per visible item (egui takes ownership).
    pub fn draw(&self, painter: &Painter, camera: &Camera, view: egui::Rect) {
        let visible = view_to_doc(camera, view);
        let to_screen = |p: Pos2| screen(camera, view, Point::new(p.x, p.y));
        for item in &self.items {
            match item {
                Item::Mesh { mesh, bounds, clip } => {
                    // Human: a mesh without triangles (a picture fill on a frame with no height) must not
                    // reach the GPU, which refuses to draw nothing and would end the app.
                    if !bounds.intersects(&visible) || mesh.indices.is_empty() {
                        continue;
                    }
                    let mut moved = mesh.clone();
                    for v in &mut moved.vertices {
                        v.pos = to_screen(v.pos);
                    }
                    clipped(painter, camera, view, *clip).add(Shape::mesh(moved));
                }
                Item::Text { text, bounds, clip } => {
                    if bounds.intersects(&visible) {
                        draw_text(&clipped(painter, camera, view, *clip), camera, view, text);
                    }
                }
                Item::Shadow { rect, radius, offset, blur, color, clip } => {
                    if !rect.intersects(&visible) {
                        continue;
                    }
                    let z = camera.zoom;
                    let min = screen(camera, view, Point::new(rect.x, rect.y));
                    let r = egui::Rect::from_min_size(min, egui::vec2(rect.w * z, rect.h * z));
                    let shadow =
                        egui::epaint::Shadow { offset: [to_i8(offset.x * z), to_i8(offset.y * z)], blur: to_u8(*blur * z), spread: 0, color: *color };
                    clipped(painter, camera, view, *clip).add(shadow.as_shape(r, egui::CornerRadius::same(to_u8(*radius * z))));
                }
            }
        }
    }
}

/// A painter clipped to `clip` (document coordinates) within the canvas.
fn clipped(painter: &Painter, camera: &Camera, view: egui::Rect, clip: Option<Rect>) -> Painter {
    match clip {
        None => painter.clone(),
        Some(c) => {
            let min = screen(camera, view, Point::new(c.x, c.y));
            let max = screen(camera, view, Point::new(c.right(), c.bottom()));
            painter.with_clip_rect(egui::Rect::from_min_max(min, max).intersect(view))
        }
    }
}

/// log2 of the zoom, rounded: the display list is rebuilt when this changes.
#[expect(clippy::cast_possible_truncation, reason = "log2 of a zoom clamped to 0.02..=256 is within -6..=8")]
fn zoom_bucket(zoom: f32) -> i32 {
    zoom.max(1e-3).log2().round() as i32
}

/// A screen offset as egui's shadow offset (−128..=127 points).
#[expect(clippy::cast_possible_truncation, reason = "clamped to the i8 range before the cast")]
fn to_i8(v: f32) -> i8 {
    v.round().clamp(-128.0, 127.0) as i8
}

/// A screen length as egui's shadow and corner sizes (0..=255 points).
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to the u8 range before the cast")]
pub fn to_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Document point → screen point.
#[must_use]
pub fn screen(camera: &Camera, view: egui::Rect, p: Point) -> Pos2 {
    let c = view.center();
    Pos2::new(c.x + (p.x - camera.center.x) * camera.zoom, c.y + (p.y - camera.center.y) * camera.zoom)
}

/// Screen point → document point.
#[must_use]
pub fn doc_point(camera: &Camera, view: egui::Rect, p: Pos2) -> Point {
    let c = view.center();
    Point::new(camera.center.x + (p.x - c.x) / camera.zoom, camera.center.y + (p.y - c.y) / camera.zoom)
}

/// The document area the canvas shows.
#[must_use]
pub fn view_to_doc(camera: &Camera, view: egui::Rect) -> Rect {
    Rect::from_corners(doc_point(camera, view, view.min), doc_point(camera, view, view.max))
}

/// An egui colour from a straight-alpha colour.
#[must_use]
pub fn color32(c: Rgba) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}

/// How a fill paints a point of the node, in coordinates normalised to its box (0–1).
enum Brush {
    Solid(Rgba),
    Gradient {
        kind: GradientKind,
        center: Point,
        size: Point,
        rotation: f32,
        stops: Vec<(f32, Rgba)>,
        opacity: f32,
    },
    /// A picture. `crop` maps a point of the crop's 0–1 box to the picture's 0–1 coordinates.
    Image {
        picture: Picture,
        fit: Fit,
        crop: Affine,
        opacity: f32,
    },
}

/// How a picture is placed in a box of another shape.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fit {
    /// Fills the box, cutting off what sticks out.
    Cover,
    /// Fits inside the box whole, leaving the rest empty.
    Contain,
    /// Fills the box exactly, changing the picture's proportions.
    Stretch,
}

impl Fit {
    /// Where the picture (or its crop) of `picture` size lands in a box of `size`, in the box's coordinates.
    fn place(self, picture: (f32, f32), size: (f32, f32)) -> Rect {
        let (pw, ph) = (picture.0.max(1e-4), picture.1.max(1e-4));
        let scale = match self {
            Fit::Stretch => return Rect::new(0.0, 0.0, size.0, size.1),
            Fit::Cover => (size.0 / pw).max(size.1 / ph),
            Fit::Contain => (size.0 / pw).min(size.1 / ph),
        };
        let (w, h) = (pw * scale, ph * scale);
        Rect::new((size.0 - w) / 2.0, (size.1 - h) / 2.0, w, h)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GradientKind {
    Linear,
    Radial,
    Angular,
}

impl Brush {
    fn at(&self, u: f32, v: f32) -> Rgba {
        match self {
            Brush::Solid(c) => *c,
            // Human: a picture's own colours come from its texture; the vertex colour only carries the opacity.
            Brush::Image { opacity, .. } => Rgba::WHITE.with_opacity(*opacity),
            Brush::Gradient { kind, center, size, rotation, stops, opacity } => {
                let (dx, dy) = (u - center.x, v - center.y);
                let t = match kind {
                    // Human: 0° points up and angles turn counter-clockwise (schema); the gradient runs from the
                    // far end of its length (stop 0) to the near end (stop 1), like CSS `linear-gradient(0deg)`.
                    GradientKind::Linear => {
                        let (sin, cos) = rotation.to_radians().sin_cos();
                        let dir = (-sin, -cos);
                        (dx * dir.0 + dy * dir.1) / size.y.max(1e-4) + 0.5
                    }
                    GradientKind::Radial => ((dx / (size.x / 2.0).max(1e-4)).powi(2) + (dy / (size.y / 2.0).max(1e-4)).powi(2)).sqrt(),
                    GradientKind::Angular => {
                        let angle = (-dx).atan2(-dy).to_degrees() - rotation;
                        angle.rem_euclid(360.0) / 360.0
                    }
                };
                gradient_at(stops, t).with_opacity(*opacity)
            }
        }
    }

    /// Whether the colour changes across the shape in a way flat triangles can't show.
    fn is_gradient(&self) -> bool {
        matches!(self, Brush::Gradient { .. })
    }
}

fn gradient_at(stops: &[(f32, Rgba)], t: f32) -> Rgba {
    let Some(first) = stops.first() else { return Rgba::default() };
    if t <= first.0 {
        return first.1;
    }
    for pair in stops.windows(2) {
        let [(p0, c0), (p1, c1)] = [pair[0], pair[1]];
        if t <= p1 {
            let span = (p1 - p0).max(1e-6);
            return c0.lerp(c1, (t - p0) / span);
        }
    }
    stops.last().map_or(first.1, |s| s.1)
}

struct Builder<'a> {
    vars: Vars<'a>,
    tolerance: f32,
    images: &'a ImageStore,
    ctx: &'a egui::Context,
    folder: Option<&'a std::path::Path>,
    /// Where effects' pictures are kept.
    baked: &'a bake::Baked,
    /// The canvas colour behind the design.
    background: Color32,
    /// Pixels per document unit that effects are baked at.
    scale: f32,
    /// Whether the node about to be built has its material drawn by the glass group it is in.
    merged: std::cell::Cell<bool>,
}

/// The line a stroke is drawn with.
#[derive(Clone, Copy)]
struct Pen {
    width: f32,
    cap: LineCap,
    join: LineJoin,
}

impl Builder<'_> {
    fn num(&self, value: Option<&Num>, theme: &ThemeContext) -> Option<f32> {
        value.and_then(|v| self.vars.number(v, theme))
    }

    /// The brushes of a `fill` (or `stroke`) list, bottom first; disabled and unknown paints are left out.
    fn brushes(&self, paints: &[Paint], theme: &ThemeContext) -> Vec<Brush> {
        let mut out = Vec::new();
        for paint in paints {
            match paint {
                Paint::Solid(text) => out.extend(self.vars.color(text, theme).map(Brush::Solid)),
                Paint::Object(object) => {
                    let enabled = object.get("enabled").and_then(|e| match e {
                        Value::Bool(b) => Some(*b),
                        Value::String(s) => self.vars.flag(&pen_doc::value::Flag::Variable(s.clone()), theme),
                        _ => None,
                    });
                    if enabled == Some(false) {
                        continue;
                    }
                    out.extend(self.object_brush(object, theme));
                }
            }
        }
        out
    }

    fn object_brush(&self, object: &serde_json::Map<String, Value>, theme: &ThemeContext) -> Option<Brush> {
        let number = |v: Option<&Value>| -> Option<f32> {
            match v? {
                Value::Number(n) => n.as_f64().map(narrow),
                Value::String(s) => self.vars.number(&Num::Text(s.clone()), theme),
                _ => None,
            }
        };
        let opacity = number(object.get("opacity")).unwrap_or(1.0);
        match object.get("type").and_then(Value::as_str) {
            Some("color") => self.vars.color(object.get("color")?.as_str()?, theme).map(Brush::Solid),
            Some("gradient") => {
                let kind = match object.get("gradientType").and_then(Value::as_str) {
                    Some("radial") => GradientKind::Radial,
                    Some("angular") => GradientKind::Angular,
                    _ => GradientKind::Linear,
                };
                let center = object
                    .get("center")
                    .map_or(Point::new(0.5, 0.5), |c| Point::new(number(c.get("x")).unwrap_or(0.5), number(c.get("y")).unwrap_or(0.5)));
                let size = object
                    .get("size")
                    .map_or(Point::new(1.0, 1.0), |s| Point::new(number(s.get("width")).unwrap_or(1.0), number(s.get("height")).unwrap_or(1.0)));
                let mut stops: Vec<(f32, Rgba)> = object
                    .get("colors")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|s| Some((number(s.get("position")).unwrap_or(0.0), self.vars.color(s.get("color")?.as_str()?, theme)?)))
                    .collect();
                stops.sort_by(|a, b| a.0.total_cmp(&b.0));
                Some(Brush::Gradient { kind, center, size, rotation: number(object.get("rotation")).unwrap_or(0.0), stops, opacity })
            }
            Some("image") => {
                // Human: a fill without a readable picture (no path, a web address, a missing file) shows as a
                // neutral grey placeholder so the layout stays visible.
                let url = object.get("url").and_then(Value::as_str);
                let Some(picture) = url.and_then(|url| self.images.get(self.ctx, self.folder, url)) else {
                    return Some(Brush::Solid(Rgba { r: 0xd9, g: 0xd9, b: 0xd9, a: 0xff }.with_opacity(opacity)));
                };
                // Human: `cover` is the schema's default; older files say `fill`, which means the same.
                let fit = match object.get("mode").and_then(Value::as_str) {
                    Some("contain" | "fit") => Fit::Contain,
                    Some("stretch") => Fit::Stretch,
                    _ => Fit::Cover,
                };
                let crop = object.get("transform").and_then(affine_of).and_then(|t| t.invert()).unwrap_or(Affine::IDENTITY);
                Some(Brush::Image { picture, fit, crop, opacity })
            }
            Some("mesh_gradient") => {
                let first = object.get("colors")?.as_array()?.first()?.as_str()?;
                self.vars.color(first, theme).map(|c| Brush::Solid(c.with_opacity(opacity)))
            }
            _ => None,
        }
    }

    /// Adds the items for `scene` and its children.
    #[expect(clippy::too_many_lines, reason = "one arm per node kind; splitting it would scatter the drawing rules")]
    fn node(&self, scene: &SceneNode, opacity: f32, clip: Option<Rect>, out: &mut Vec<Item>) {
        let node = &scene.node;
        let theme = &scene.theme;
        // Whether the glass group this node is in draws its material; said by `children` just before.
        let merged = self.merged.replace(false);
        if node.enabled.as_ref().and_then(|e| self.vars.flag(e, theme)) == Some(false) {
            return;
        }
        let opacity = opacity * self.num(node.opacity.as_ref(), theme).unwrap_or(1.0).clamp(0.0, 1.0);
        let (w, h) = (scene.local.w, scene.local.h);
        let bounds = layout::doc_bounds(scene);
        let fills = node.fill.as_ref().map(|f| self.brushes(f.as_slice(), theme)).unwrap_or_default();
        let strokes = node.stroke.as_ref().map(|f| self.brushes(f.as_slice(), theme)).unwrap_or_default();
        let fill_item = |shape: &Path, brush: &Brush, fill_rule: FillRule| {
            self.fill_mesh(shape, brush, fill_rule, (w, h), &scene.to_doc, opacity).map(|mesh| Item::Mesh { mesh, bounds, clip })
        };
        // Human: effects need to know which of the node's items are its fill, its stroke and its children: a
        // shadow is cast by the fill and stroke, an inner shadow falls on the fill, under the stroke.
        let effects = self.effects_of(scene);
        let start = out.len();
        let mut fill_end = start;
        match &node.kind {
            Kind::Frame | Kind::Rectangle | Kind::Browser => {
                let radii = self.corner_radii(node, theme, w, h);
                let shape = rounded_rect(0.0, 0.0, w, h, radii);
                if node.kind == Kind::Browser && fills.is_empty() {
                    out.extend(fill_item(&shape, &Brush::Solid(Rgba::WHITE), FillRule::NonZero));
                }
                for brush in &fills {
                    out.extend(fill_item(&shape, brush, FillRule::NonZero));
                }
                fill_end = out.len();
                self.rect_strokes(scene, &strokes, radii, opacity, clip, out);
                if node.extra.get("slot").is_some_and(Value::is_array) && scene.children.is_empty() {
                    self.slot_hatch(scene, opacity, clip, out);
                }
            }
            Kind::Note => {
                let color = node.color.as_deref().and_then(|c| self.vars.color(c, theme)).unwrap_or(NOTE_COLOR);
                let shape = rounded_rect(0.0, 0.0, w, h, [NOTE_RADIUS; 4]);
                out.extend(fill_item(&shape, &Brush::Solid(Rgba { a: 255, ..color }), FillRule::NonZero));
                let mut style = layout::text_style(node, self.vars, theme);
                style.wrap = Some(w - 2.0 * NOTE_PADDING);
                // Human: dark notes get white text (contrast below 4.6 against black).
                let text_color =
                    if color.contrast(Rgba::BLACK) < NOTE_TEXT_CONTRAST { Rgba::WHITE } else { Rgba { r: 0x1c, g: 0x1c, b: 0x1c, a: 255 } };
                let to_doc = scene.to_doc.then(&Affine::translate(NOTE_PADDING, NOTE_PADDING));
                let size = (w - 2.0 * NOTE_PADDING, h - 2.0 * NOTE_PADDING);
                out.push(Item::Text {
                    text: Box::new(TextItem { style, color: color32(text_color.with_opacity(opacity)), to_doc, size }),
                    bounds,
                    clip,
                });
                fill_end = out.len();
            }
            Kind::Ellipse => {
                let shape = self.ellipse(node, theme, w, h);
                for brush in &fills {
                    out.extend(fill_item(&shape, brush, FillRule::EvenOdd));
                }
                fill_end = out.len();
                for brush in &strokes {
                    self.stroke(&shape, brush, scene, opacity, bounds, clip, out);
                }
            }
            Kind::Polygon => {
                let count = self.num(node.polygon_count.as_ref(), theme).unwrap_or(3.0).clamp(3.0, 64.0);
                // Human: a polygon's `cornerRadius` is one number that rounds every corner (schema `Polygon`).
                let radius = match &node.corner_radius {
                    Some(Corners::All(r)) => self.num(Some(r), theme),
                    Some(Corners::Each(each)) => self.num(each.first(), theme),
                    None => None,
                };
                let shape = polygon(w, h, count, radius.unwrap_or(0.0).max(0.0));
                for brush in &fills {
                    out.extend(fill_item(&shape, brush, FillRule::NonZero));
                }
                fill_end = out.len();
                for brush in &strokes {
                    self.stroke(&shape, brush, scene, opacity, bounds, clip, out);
                }
            }
            Kind::Path => {
                let Some(shape) = node.geometry.as_deref().and_then(|g| svg::parse(g).ok()).map(|segments| fit_path(&segments, node, w, h)) else {
                    return;
                };
                let rule = if node.fill_rule.as_deref() == Some("evenodd") { FillRule::EvenOdd } else { FillRule::NonZero };
                for brush in &fills {
                    out.extend(fill_item(&shape, brush, rule));
                }
                fill_end = out.len();
                for brush in &strokes {
                    self.stroke(&shape, brush, scene, opacity, bounds, clip, out);
                }
            }
            Kind::Line => {
                let mut b = Path::builder();
                b.begin(point(0.0, 0.0));
                b.line_to(point(w, h));
                b.end(false);
                let shape = b.build();
                for brush in &strokes {
                    self.stroke(&shape, brush, scene, opacity, bounds, clip, out);
                }
            }
            Kind::Text => {
                let mut style = layout::text_style(node, self.vars, theme);
                if matches!(node.text_growth.as_deref(), Some("fixed-width" | "fixed-width-height")) {
                    style.wrap = Some(w);
                }
                let color = fills.iter().find_map(|b| if let Brush::Solid(c) = b { Some(*c) } else { None }).unwrap_or(Rgba::BLACK);
                out.push(Item::Text {
                    text: Box::new(TextItem { style, color: color32(color.with_opacity(opacity)), to_doc: scene.to_doc, size: (w, h) }),
                    bounds,
                    clip,
                });
                fill_end = out.len();
            }
            Kind::Icon | Kind::IconFont => {
                let color = fills.iter().find_map(|b| if let Brush::Solid(c) = b { Some(*c) } else { None }).unwrap_or(Rgba::BLACK);
                // Human: newer files say `library` and `icon`; older `icon_font` nodes say `iconFontFamily` and
                // `iconFontName`. A node without a library is looked up in Lucide.
                let old = |key: &str| node.extra.get(key).and_then(Value::as_str);
                let library = node.library.as_deref().or_else(|| old("iconFontFamily")).unwrap_or("lucide");
                let name = node.icon.as_deref().or_else(|| old("iconFontName"));
                if let Some(segments) = name.and_then(|name| icons::outline(library, name)) {
                    let shape = build_path(&segments, Rect::new(0.0, 0.0, icons::VIEW_BOX, icons::VIEW_BOX), w, h);
                    let pen = Pen { width: icons::LINE_WIDTH * w.min(h) / icons::VIEW_BOX, cap: LineCap::Round, join: LineJoin::Round };
                    self.stroke_pen(&shape, &Brush::Solid(color), pen, scene, opacity, bounds, clip, out);
                } else {
                    // Human: only Lucide (and Feather through it) is bundled; an icon from another library, or with
                    // an unknown name, shows as a small rounded outline in its colour so layouts keep their rhythm.
                    let inset = w.min(h) * 0.2;
                    let shape = rounded_rect(inset, inset, (w - 2.0 * inset).max(0.0), (h - 2.0 * inset).max(0.0), [w.min(h) * 0.15; 4]);
                    let pen = (w.min(h) * 0.08).max(0.5);
                    self.stroke_with(&shape, &Brush::Solid(color), pen, scene, opacity, bounds, clip, out);
                }
                // An icon's lines are what it is filled with.
                fill_end = out.len();
            }
            Kind::Ref | Kind::Group | Kind::Script | Kind::Other(_) => {
                if !scene.resolved {
                    let shape = rounded_rect(0.0, 0.0, w.max(1.0), h.max(1.0), [0.0; 4]);
                    self.stroke_with(&shape, &Brush::Solid(Rgba { r: 0xef, g: 0x44, b: 0x44, a: 0xff }), 1.0, scene, opacity, bounds, clip, out);
                }
            }
        }
        let own_end = out.len();
        self.children(scene, opacity, bounds, clip, out);
        let pane = self.pane_of(scene, merged, clip);
        if !effects.is_empty() || pane.is_some() {
            let opaque = opacity >= 0.999 && fills.iter().any(|b| matches!(b, Brush::Solid(c) if c.a == 255));
            let look = bake::Look { opacity, opaque, clip, radius: self.box_radius(scene) };
            self.apply_effects(scene, &effects, pane.as_ref(), [start, fill_end.clamp(start, own_end), own_end], &look, out);
        }
    }

    /// Marks an empty slot: diagonal lines across the frame, so the place an instance can fill is seen.
    // Human: only a slot that has nothing in it is hatched.
    fn slot_hatch(&self, scene: &SceneNode, opacity: f32, clip: Option<Rect>, out: &mut Vec<Item>) {
        const SPACING: f32 = 8.0;
        let (w, h) = (scene.local.w, scene.local.h);
        if w <= 0.0 || h <= 0.0 || (w + h) / SPACING > 2000.0 {
            return;
        }
        // Lines x + y = c, each cut to the box.
        let mut builder = Path::builder();
        let mut c = SPACING;
        while c < w + h {
            builder.begin(point(c.min(w), (c - w).max(0.0)));
            builder.line_to(point((c - h).max(0.0), c.min(h)));
            builder.end(false);
            c += SPACING;
        }
        let pen = Pen { width: 1.0, cap: LineCap::Butt, join: LineJoin::Miter };
        let color = Rgba { r: 0xd9, g: 0x46, b: 0xef, a: 0x66 };
        self.stroke_pen(&builder.build(), &Brush::Solid(color), pen, scene, opacity, layout::doc_bounds(scene), clip, out);
    }

    /// The corner radius of a node that is a plain box or a whole ellipse, whose shadow egui can draw itself.
    fn box_radius(&self, scene: &SceneNode) -> Option<f32> {
        let (node, theme) = (&scene.node, &scene.theme);
        let (w, h) = (scene.local.w, scene.local.h);
        match node.kind {
            Kind::Frame | Kind::Rectangle | Kind::Browser => {
                let radii = self.corner_radii(node, theme, w, h);
                radii.iter().all(|r| (r - radii[0]).abs() < 1e-3).then_some(radii[0])
            }
            Kind::Ellipse => {
                let ring = self.num(node.inner_radius.as_ref(), theme).unwrap_or(0.0) > 0.0;
                let whole = self.num(node.sweep_angle.as_ref(), theme).unwrap_or(360.0).abs() >= 359.99;
                (whole && !ring).then_some(w.min(h) / 2.0)
            }
            _ => None,
        }
    }

    /// Adds the items of `scene`'s children, clipped to its box when it clips.
    fn children(&self, scene: &SceneNode, opacity: f32, bounds: Rect, clip: Option<Rect>, out: &mut Vec<Item>) {
        let clips = scene.node.clip.as_ref().and_then(|c| self.vars.flag(c, &scene.theme)).unwrap_or(false);
        let child_clip = if clips { Some(clip.map_or(bounds, |c| intersect(&c, &bounds))) } else { clip };
        // Human: a frame that clips cuts its children to its own outline, rounded corners included. Only an
        // unturned box has corners that can be cut this way; a turned one cuts to the box around it.
        let (w, h) = (scene.local.w, scene.local.h);
        let boxed = matches!(scene.node.kind, Kind::Frame | Kind::Rectangle | Kind::Browser) && scene.to_doc.is_axis_aligned();
        let radii = if clips && boxed { self.corner_radii(&scene.node, &scene.theme, w, h) } else { [0.0; 4] };
        let rounded = radii.iter().any(|radius| *radius > 0.5) && !scene.children.is_empty();
        let mut inside = Vec::new();
        let mut joined = 0;
        for child in &scene.children {
            // Human: a glass group draws the glass of its children as one pane, so they don't draw their own.
            let merges = joined < glass::MOST_SHAPES && self.merges(scene, child);
            joined += usize::from(merges);
            self.merged.set(merges);
            self.node(child, opacity, child_clip, if rounded { &mut inside } else { &mut *out });
        }
        if let (true, Some(cut)) = (rounded, child_clip) {
            let outline = self
                .fill_mesh(&rounded_rect(0.0, 0.0, w, h, radii), &Brush::Solid(Rgba::WHITE), FillRule::NonZero, (w, h), &scene.to_doc, 1.0)
                .map(|mesh| Item::Mesh { mesh, bounds, clip: None });
            match outline {
                Some(outline) => self.round_off(&outline, radii, bounds, cut, inside, out),
                None => out.extend(inside),
            }
        }
    }

    #[expect(clippy::many_single_char_names, reason = "x, y, w, h are the box's position and size; b is the path builder")]
    fn corner_radii(&self, node: &Node, theme: &ThemeContext, w: f32, h: f32) -> [f32; 4] {
        let max = w.min(h) / 2.0;
        let n = |v: &Num| self.vars.number(v, theme).unwrap_or(0.0).clamp(0.0, max);
        match &node.corner_radius {
            // Human: browser nodes default to 16.
            None if node.kind == Kind::Browser => [BROWSER_RADIUS.min(max); 4],
            None => [0.0; 4],
            Some(Corners::All(all)) => [n(all); 4],
            Some(Corners::Each(each)) => match each.as_slice() {
                [a, b, c, d] => [n(a), n(b), n(c), n(d)],
                [all] => [n(all); 4],
                _ => [0.0; 4],
            },
        }
    }

    #[expect(clippy::many_single_char_names, reason = "x, y, w, h are the box's position and size; b is the path builder")]
    fn ellipse(&self, node: &Node, theme: &ThemeContext, w: f32, h: f32) -> Path {
        let inner = self.num(node.inner_radius.as_ref(), theme).unwrap_or(0.0).clamp(0.0, 1.0);
        let start = self.num(node.start_angle.as_ref(), theme).unwrap_or(0.0);
        let sweep = self.num(node.sweep_angle.as_ref(), theme).unwrap_or(360.0).clamp(-360.0, 360.0);
        let (cx, cy, rx, ry) = (w / 2.0, h / 2.0, w / 2.0, h / 2.0);
        // Human: angles are counter-clockwise from the right on screen, so y goes up as the angle grows.
        let at = |deg: f32, scale: f32| {
            let (s, c) = deg.to_radians().sin_cos();
            point(cx + rx * scale * c, cy - ry * scale * s)
        };
        let steps = 72.0f32;
        let mut b = Path::builder();
        let full = sweep.abs() >= 359.99;
        let step = sweep / steps;
        b.begin(at(start, 1.0));
        let mut i = 1.0;
        while i <= steps {
            b.line_to(at(start + step * i, 1.0));
            i += 1.0;
        }
        if full {
            b.end(true);
            if inner > 0.0 {
                b.begin(at(start, inner));
                let mut j = 1.0;
                while j <= steps {
                    b.line_to(at(start - step * j, inner));
                    j += 1.0;
                }
                b.end(true);
            }
        } else if inner > 0.0 {
            let mut j = steps;
            while j >= 0.0 {
                b.line_to(at(start + step * j, inner));
                j -= 1.0;
            }
            b.end(true);
        } else {
            b.line_to(point(cx, cy));
            b.end(true);
        }
        b.build()
    }

    /// Tessellates a filled shape (local coordinates) into a document-space mesh coloured by `brush`.
    fn fill_mesh(&self, shape: &Path, brush: &Brush, rule: FillRule, size: (f32, f32), to_doc: &Affine, opacity: f32) -> Option<Mesh> {
        let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
        let options = FillOptions::tolerance(self.tolerance_for(to_doc)).with_fill_rule(rule);
        FillTessellator::new()
            .tessellate_path(shape, &options, &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| v.position().to_array()))
            .ok()?;
        if brush.is_gradient() {
            subdivide(&mut buffers, size.0.max(size.1) / 24.0);
        }
        Some(colored_mesh(&buffers, brush, size, to_doc, opacity))
    }

    /// Curve tolerance in local units: the screen tolerance divided by how much the node is scaled.
    fn tolerance_for(&self, to_doc: &Affine) -> f32 {
        let scale = (to_doc.m[0] * to_doc.m[3] - to_doc.m[1] * to_doc.m[2]).abs().sqrt().max(1e-3);
        self.tolerance / scale
    }

    #[expect(clippy::too_many_arguments, reason = "a stroke needs its shape, brush, node, opacity, bounds and clip")]
    fn stroke(&self, shape: &Path, brush: &Brush, scene: &SceneNode, opacity: f32, bounds: Rect, clip: Option<Rect>, out: &mut Vec<Item>) {
        let width = self.stroke_width(scene);
        if width > 0.0 {
            self.stroke_with(shape, brush, width, scene, opacity, bounds, clip, out);
        }
    }

    fn stroke_width(&self, scene: &SceneNode) -> f32 {
        match &scene.node.stroke_width {
            None => 1.0,
            Some(StrokeWidth::All(w)) => self.vars.number(w, &scene.theme).unwrap_or(1.0).max(0.0),
            Some(StrokeWidth::Sides(sides)) => sides.values().filter_map(Value::as_f64).map(narrow).fold(0.0, f32::max),
        }
    }

    #[expect(clippy::too_many_arguments, reason = "a stroke needs its shape, brush, width, node, opacity, bounds and clip")]
    fn stroke_with(
        &self,
        shape: &Path,
        brush: &Brush,
        width: f32,
        scene: &SceneNode,
        opacity: f32,
        bounds: Rect,
        clip: Option<Rect>,
        out: &mut Vec<Item>,
    ) {
        let node = &scene.node;
        let cap = match node.extra.get("strokeLinecap").and_then(Value::as_str) {
            Some("round") => LineCap::Round,
            Some("square") => LineCap::Square,
            _ => LineCap::Butt,
        };
        let join = match node.extra.get("strokeLinejoin").and_then(Value::as_str) {
            Some("round") => LineJoin::Round,
            Some("bevel") => LineJoin::Bevel,
            _ => LineJoin::Miter,
        };
        self.stroke_pen(shape, brush, Pen { width, cap, join }, scene, opacity, bounds, clip, out);
    }

    #[expect(clippy::too_many_arguments, reason = "a stroke needs its shape, brush, pen, node, opacity, bounds and clip")]
    fn stroke_pen(
        &self,
        shape: &Path,
        brush: &Brush,
        pen: Pen,
        scene: &SceneNode,
        opacity: f32,
        bounds: Rect,
        clip: Option<Rect>,
        out: &mut Vec<Item>,
    ) {
        let options =
            StrokeOptions::tolerance(self.tolerance_for(&scene.to_doc)).with_line_width(pen.width).with_line_cap(pen.cap).with_line_join(pen.join);
        let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
        if StrokeTessellator::new()
            .tessellate_path(shape, &options, &mut BuffersBuilder::new(&mut buffers, |v: StrokeVertex| v.position().to_array()))
            .is_ok()
        {
            out.push(Item::Mesh { mesh: colored_mesh(&buffers, brush, (scene.local.w, scene.local.h), &scene.to_doc, opacity), bounds, clip });
        }
    }

    /// Box strokes: aligned inside by default; per-side widths draw each side as its own strip.
    #[expect(clippy::many_single_char_names, reason = "x, y, w, h are the box's position and size; b is the path builder")]
    fn rect_strokes(&self, scene: &SceneNode, strokes: &[Brush], radii: [f32; 4], opacity: f32, clip: Option<Rect>, out: &mut Vec<Item>) {
        let (w, h) = (scene.local.w, scene.local.h);
        let bounds = layout::doc_bounds(scene);
        if let Some(StrokeWidth::Sides(sides)) = &scene.node.stroke_width {
            let side = |key: &str| sides.get(key).and_then(Value::as_f64).map_or(0.0, narrow).max(0.0);
            let (t, r, b, l) = (side("top"), side("right"), side("bottom"), side("left"));
            let strips = [Rect::new(0.0, 0.0, w, t), Rect::new(w - r, 0.0, r, h), Rect::new(0.0, h - b, w, b), Rect::new(0.0, 0.0, l, h)];
            for brush in strokes {
                for strip in strips.iter().filter(|s| s.w > 0.0 && s.h > 0.0) {
                    let shape = rounded_rect(strip.x, strip.y, strip.w, strip.h, [0.0; 4]);
                    if let Some(mesh) = self.fill_mesh(&shape, brush, FillRule::NonZero, (w, h), &scene.to_doc, opacity) {
                        out.push(Item::Mesh { mesh, bounds, clip });
                    }
                }
            }
            return;
        }
        let width = self.stroke_width(scene);
        if width <= 0.0 || strokes.is_empty() {
            return;
        }
        // Human: centred by default.
        let offset = match scene.node.stroke_alignment.as_deref() {
            Some("inner") => width / 2.0,
            Some("outer") => -width / 2.0,
            _ => 0.0,
        };
        let shape = rounded_rect(offset, offset, (w - 2.0 * offset).max(0.0), (h - 2.0 * offset).max(0.0), radii.map(|r| (r - offset).max(0.0)));
        for brush in strokes {
            self.stroke_with(&shape, brush, width, scene, opacity, bounds, clip, out);
        }
    }
}

fn intersect(a: &Rect, b: &Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    Rect::new(x, y, (a.right().min(b.right()) - x).max(0.0), (a.bottom().min(b.bottom()) - y).max(0.0))
}

/// Splits triangles until no edge is longer than `max_len`, so per-vertex gradient colours look smooth.
fn subdivide(buffers: &mut VertexBuffers<[f32; 2], u32>, max_len: f32) {
    let max_len = max_len.max(0.5);
    for _ in 0..4 {
        let mut next = Vec::with_capacity(buffers.indices.len() * 2);
        let mut changed = false;
        for &[a, b, c] in buffers.indices.as_chunks::<3>().0 {
            let p = |i: u32| buffers.vertices.get(usize::try_from(i).unwrap_or(usize::MAX)).copied().unwrap_or([0.0, 0.0]);
            let (pa, pb, pc) = (p(a), p(b), p(c));
            let long = |u: [f32; 2], v: [f32; 2]| ((u[0] - v[0]).powi(2) + (u[1] - v[1]).powi(2)).sqrt() > max_len;
            if !(long(pa, pb) || long(pb, pc) || long(pc, pa)) {
                next.extend_from_slice(&[a, b, c]);
                continue;
            }
            changed = true;
            let mut mid = |u: [f32; 2], v: [f32; 2]| {
                buffers.vertices.push([f32::midpoint(u[0], v[0]), f32::midpoint(u[1], v[1])]);
                u32::try_from(buffers.vertices.len() - 1).unwrap_or(0)
            };
            let (ab, bc, ca) = (mid(pa, pb), mid(pb, pc), mid(pc, pa));
            next.extend_from_slice(&[a, ab, ca, ab, b, bc, ca, bc, c, ab, bc, ca]);
        }
        buffers.indices = next;
        if !changed || buffers.indices.len() > 200_000 {
            break;
        }
    }
}

fn colored_mesh(buffers: &VertexBuffers<[f32; 2], u32>, brush: &Brush, size: (f32, f32), to_doc: &Affine, opacity: f32) -> Mesh {
    if let Brush::Image { picture, fit, crop, opacity: own } = brush {
        return image_mesh(buffers, *picture, *fit, crop, size, to_doc, opacity * own);
    }
    let (w, h) = (size.0.max(1e-4), size.1.max(1e-4));
    let vertices = buffers
        .vertices
        .iter()
        .map(|&[x, y]| {
            let color = brush.at(x / w, y / h).with_opacity(opacity);
            let p = to_doc.apply(Point::new(x, y));
            Vertex { pos: Pos2::new(p.x, p.y), uv: WHITE_UV, color: color32(color) }
        })
        .collect();
    Mesh { indices: buffers.indices.clone(), vertices, texture_id: egui::TextureId::default() }
}

/// A six-number `AffineTransform` from the file.
fn affine_of(value: &Value) -> Option<Affine> {
    let numbers = value.as_array()?;
    let mut m = [0.0; 6];
    if numbers.len() != m.len() {
        return None;
    }
    for (slot, number) in m.iter_mut().zip(numbers) {
        *slot = narrow(number.as_f64()?);
    }
    Some(Affine { m })
}

/// A shape's triangles textured with a picture: placed by `fit`, cropped by `crop`, and cut off where a
/// contained picture ends so nothing smears past its edge.
fn image_mesh(
    buffers: &VertexBuffers<[f32; 2], u32>,
    picture: Picture,
    fit: Fit,
    crop: &Affine,
    size: (f32, f32),
    to_doc: &Affine,
    opacity: f32,
) -> Mesh {
    // Human: the crop's size in picture pixels is the picture's size times how far the crop's unit box reaches
    // in picture coordinates.
    let cropped = (picture.width * crop.m[0].hypot(crop.m[1]), picture.height * crop.m[2].hypot(crop.m[3]));
    let place = fit.place(cropped, size);
    let (vertices, indices) = if fit == Fit::Contain { clip_triangles(buffers, &place) } else { (buffers.vertices.clone(), buffers.indices.clone()) };
    let color = color32(Rgba::WHITE.with_opacity(opacity));
    let (pw, ph) = (place.w.max(1e-4), place.h.max(1e-4));
    let vertices = vertices
        .iter()
        .map(|&[x, y]| {
            let uv = crop.apply(Point::new((x - place.x) / pw, (y - place.y) / ph));
            let p = to_doc.apply(Point::new(x, y));
            Vertex { pos: Pos2::new(p.x, p.y), uv: Pos2::new(uv.x, uv.y), color }
        })
        .collect();
    Mesh { indices, vertices, texture_id: picture.texture }
}

/// The triangles of `buffers` cut down to the part inside `rect` (Sutherland–Hodgman, one triangle at a time).
fn clip_triangles(buffers: &VertexBuffers<[f32; 2], u32>, rect: &Rect) -> (Vec<[f32; 2]>, Vec<u32>) {
    // Each side as (axis, limit, keep-below): a point stays when its coordinate is on the kept side of the limit.
    let sides = [(0, rect.x, false), (0, rect.right(), true), (1, rect.y, false), (1, rect.bottom(), true)];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let corner = |i: u32| buffers.vertices.get(usize::try_from(i).unwrap_or(usize::MAX)).copied().unwrap_or([0.0, 0.0]);
    for &[a, b, c] in buffers.indices.as_chunks::<3>().0 {
        let mut polygon = vec![corner(a), corner(b), corner(c)];
        for (axis, limit, below) in sides {
            let inside = |p: &[f32; 2]| if below { p[axis] <= limit } else { p[axis] >= limit };
            let mut next = Vec::with_capacity(polygon.len() + 2);
            for (i, p) in polygon.iter().enumerate() {
                let q = &polygon[(i + 1) % polygon.len()];
                if inside(p) {
                    next.push(*p);
                }
                if inside(p) != inside(q) {
                    let t = (limit - p[axis]) / (q[axis] - p[axis]);
                    next.push([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]);
                }
            }
            polygon = next;
            if polygon.len() < 3 {
                break;
            }
        }
        if polygon.len() < 3 {
            continue;
        }
        let Ok(first) = u32::try_from(vertices.len()) else { break };
        let Ok(count) = u32::try_from(polygon.len()) else { continue };
        vertices.extend_from_slice(&polygon);
        for i in 1..count - 1 {
            indices.extend_from_slice(&[first, first + i, first + i + 1]);
        }
    }
    (vertices, indices)
}

/// A rectangle with corner radii `[top-left, top-right, bottom-right, bottom-left]`.
#[expect(clippy::many_single_char_names, reason = "x, y, w, h are the box's position and size; b is the path builder")]
fn rounded_rect(x: f32, y: f32, w: f32, h: f32, radii: [f32; 4]) -> Path {
    // Human: 0.5523 places cubic control points so a quarter circle is within 0.03 % of round.
    const KAPPA: f32 = 0.552_284_8;
    let [tl, tr, br, bl] = radii.map(|r| r.clamp(0.0, w.min(h) / 2.0));
    let mut b = Path::builder();
    b.begin(point(x + tl, y));
    b.line_to(point(x + w - tr, y));
    if tr > 0.0 {
        b.cubic_bezier_to(point(x + w - tr + tr * KAPPA, y), point(x + w, y + tr - tr * KAPPA), point(x + w, y + tr));
    }
    b.line_to(point(x + w, y + h - br));
    if br > 0.0 {
        b.cubic_bezier_to(point(x + w, y + h - br + br * KAPPA), point(x + w - br + br * KAPPA, y + h), point(x + w - br, y + h));
    }
    b.line_to(point(x + bl, y + h));
    if bl > 0.0 {
        b.cubic_bezier_to(point(x + bl - bl * KAPPA, y + h), point(x, y + h - bl + bl * KAPPA), point(x, y + h - bl));
    }
    b.line_to(point(x, y + tl));
    if tl > 0.0 {
        b.cubic_bezier_to(point(x, y + tl - tl * KAPPA), point(x + tl - tl * KAPPA, y), point(x + tl, y));
    }
    b.end(true);
    b.build()
}

/// The outline of a regular polygon inscribed in a `w` × `h` box, first corner at the top, its corners rounded
/// by `radius`: the points of a closed line.
// Human: the corners lie on the ellipse that fills the box, starting at the top. A corner
// radius is at most half the shortest side; each corner is then cut back along both its sides by radius ÷ tan
// of half the corner's angle and joined by an arc.
#[must_use]
pub fn polygon_points(w: f32, h: f32, count: f32, radius: f32) -> Vec<[f32; 2]> {
    /// Straight pieces a rounded corner is drawn with.
    const ARC_STEPS: u8 = 8;
    let mut corners = Vec::new();
    let mut i = 0.0;
    while i < count {
        let angle = (i / count * 360.0 - 90.0).to_radians();
        corners.push([w / 2.0 + w / 2.0 * angle.cos(), h / 2.0 + h / 2.0 * angle.sin()]);
        i += 1.0;
    }
    let n = corners.len();
    let shortest =
        (0..n).map(|k| (corners[(k + 1) % n][0] - corners[k][0]).hypot(corners[(k + 1) % n][1] - corners[k][1])).fold(f32::INFINITY, f32::min);
    if n < 3 || radius <= 0.0 || shortest < 1e-6 {
        return corners;
    }
    let radius = radius.min(shortest / 2.0);
    let unit = |from: [f32; 2], to: [f32; 2]| {
        let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
        let length = dx.hypot(dy).max(1e-9);
        [dx / length, dy / length]
    };
    let mut out = Vec::new();
    for k in 0..n {
        let (before, corner, after) = (corners[(k + n - 1) % n], corners[k], corners[(k + 1) % n]);
        let (back, on) = (unit(corner, before), unit(corner, after));
        let half = (back[0] * on[0] + back[1] * on[1]).clamp(-1.0, 1.0).acos() / 2.0;
        if !(1e-6..std::f32::consts::FRAC_PI_2 - 1e-6).contains(&half) {
            out.push(corner);
            continue;
        }
        let cut = radius / half.tan();
        let inward = unit([0.0, 0.0], [back[0] + on[0], back[1] + on[1]]);
        let reach = radius / half.sin();
        let centre = [corner[0] + inward[0] * reach, corner[1] + inward[1] * reach];
        let start = [corner[0] + back[0] * cut, corner[1] + back[1] * cut];
        let end = [corner[0] + on[0] * cut, corner[1] + on[1] * cut];
        let from = (start[1] - centre[1]).atan2(start[0] - centre[0]);
        let mut sweep = (end[1] - centre[1]).atan2(end[0] - centre[0]) - from;
        if sweep > std::f32::consts::PI {
            sweep -= std::f32::consts::TAU;
        }
        if sweep < -std::f32::consts::PI {
            sweep += std::f32::consts::TAU;
        }
        for step in 0..=ARC_STEPS {
            let angle = from + sweep * f32::from(step) / f32::from(ARC_STEPS);
            out.push([centre[0] + radius * angle.cos(), centre[1] + radius * angle.sin()]);
        }
    }
    out
}

/// A regular polygon inscribed in the box, first corner at the top, as a path to fill and stroke.
fn polygon(w: f32, h: f32, count: f32, radius: f32) -> Path {
    let mut builder = Path::builder();
    for (index, [x, y]) in polygon_points(w, h, count, radius).into_iter().enumerate() {
        if index == 0 {
            builder.begin(point(x, y));
        } else {
            builder.line_to(point(x, y));
        }
    }
    builder.end(true);
    builder.build()
}

/// SVG path segments mapped from their view box (or tight bounds) onto the node's box.
fn fit_path(segments: &[Segment], node: &Node, w: f32, h: f32) -> Path {
    let vb = node
        .view_box
        .map(|v| Rect::new(narrow(v[0]), narrow(v[1]), narrow(v[2]), narrow(v[3])))
        .or_else(|| svg::bounds(segments))
        .unwrap_or(Rect::ZERO);
    build_path(segments, vb, w, h)
}

/// Drawing commands mapped from the box `vb` they were written in onto a `w` × `h` box.
#[expect(clippy::many_single_char_names, reason = "w and h are the box's size; b is the path builder; p a point")]
fn build_path(segments: &[Segment], vb: Rect, w: f32, h: f32) -> Path {
    let sx = if vb.w > 1e-6 { w / vb.w } else { 1.0 };
    let sy = if vb.h > 1e-6 { h / vb.h } else { 1.0 };
    let map = |p: Point| point((p.x - vb.x) * sx, (p.y - vb.y) * sy);
    let mut b = Path::builder();
    let mut open = false;
    for segment in segments {
        match *segment {
            Segment::MoveTo(p) => {
                if open {
                    b.end(false);
                }
                b.begin(map(p));
                open = true;
            }
            Segment::LineTo(p) => {
                if open {
                    b.line_to(map(p));
                }
            }
            Segment::QuadTo(c, p) => {
                if open {
                    b.quadratic_bezier_to(map(c), map(p));
                }
            }
            Segment::CubicTo(c1, c2, p) => {
                if open {
                    b.cubic_bezier_to(map(c1), map(c2), map(p));
                }
            }
            Segment::Close => {
                if open {
                    b.end(true);
                    open = false;
                }
            }
        }
    }
    if open {
        b.end(false);
    }
    b.build()
}

/// The egui text job for a style at `scale` screen points per canvas unit.
#[must_use]
pub fn text_job(style: &TextStyle, color: Color32, scale: f32) -> LayoutJob {
    // Human: the face the text names, when it is installed and ready; until then, and for a family nothing
    // installed can stand in for, the interface face in its regular, bold or italic.
    let (family, coords, italics) = match crate::fonts::choose(&style.family, style.weight, style.italic) {
        Some(chosen) => (chosen.family, chosen.coords, chosen.slant),
        None if style.italic => (FontFamily::Name(ITALIC_FAMILY.into()), VariationCoords::default(), false),
        None if style.weight >= 600 => (FontFamily::Name(BOLD_FAMILY.into()), VariationCoords::default(), false),
        None if style.family.to_ascii_lowercase().contains("mono") => (FontFamily::Monospace, VariationCoords::default(), false),
        None => (FontFamily::Proportional, VariationCoords::default(), false),
    };
    let size = style.size * scale;
    let format = TextFormat {
        font_id: FontId::new(size, family),
        color,
        extra_letter_spacing: style.letter_spacing * scale,
        line_height: style.line_px.map(|l| l * scale),
        coords,
        italics,
        underline: if style.underline { Stroke::new((size / 16.0).max(1.0), color) } else { Stroke::NONE },
        strikethrough: if style.strikethrough { Stroke::new((size / 16.0).max(1.0), color) } else { Stroke::NONE },
        ..Default::default()
    };
    let mut job = LayoutJob::single_section(style.content.clone(), format);
    // Human: a hair of extra width keeps rounding from wrapping a line the layout measured as fitting.
    job.wrap.max_width = style.wrap.map_or(f32::INFINITY, |w| w * scale + 0.5);
    job.halign = match style.align {
        TextAlign::Left | TextAlign::Justify => egui::Align::LEFT,
        TextAlign::Center => egui::Align::Center,
        TextAlign::Right => egui::Align::RIGHT,
    };
    job
}

fn draw_text(painter: &Painter, camera: &Camera, view: egui::Rect, text: &TextItem) {
    let z = camera.zoom;
    if text.style.content.is_empty() || text.style.size * z < 1.0 {
        return;
    }
    let (laid, grow) = glyph_scale(text.style.size, z);
    let galley = painter.layout_job(text_job(&text.style, text.color, laid));
    let (anchor, angle) = text_anchor(text, galley.size().y / laid);
    let pos = screen(camera, view, anchor);
    if grow <= 1.0 {
        painter.add(egui::epaint::TextShape::new(pos, galley, text.color).with_angle(angle));
        return;
    }
    // Human: a headline zoomed past what the font atlas holds is laid out at the largest size that fits, and
    // its triangles are scaled up around where the text starts. It is a little soft, and it is there.
    let shape = egui::epaint::TextShape::new(Pos2::ZERO, galley, text.color).with_angle(angle);
    let per_point = painter.ctx().pixels_per_point();
    for clipped in painter.ctx().tessellate(vec![egui::epaint::ClippedShape::new(egui::Rect::EVERYTHING, shape)], per_point) {
        if let egui::epaint::Primitive::Mesh(mut mesh) = clipped.primitive {
            for vertex in &mut mesh.vertices {
                vertex.pos = pos + vertex.pos.to_vec2() * grow;
            }
            painter.add(Shape::mesh(mesh));
        }
    }
}

/// Where a text's paragraph starts in the document, and the angle it is drawn at; `text_h` is the height of
/// the laid-out paragraph in document units.
fn text_anchor(text: &TextItem, text_h: f32) -> (Point, f32) {
    let (w, h) = text.size;
    // Human: the paragraph starts half a letter-spacing to the left (`x = -letterSpacing / 2`),
    // and vertical alignment applies only to fixed-size text (not `auto`).
    let anchor_x = -text.style.letter_spacing / 2.0
        + match text.style.align {
            TextAlign::Left | TextAlign::Justify => 0.0,
            TextAlign::Center => w / 2.0,
            TextAlign::Right => w,
        };
    let top = match (text.style.wrap.is_some(), text.style.valign) {
        (false, _) | (true, TextVAlign::Top) => 0.0,
        (true, TextVAlign::Middle) => (h - text_h) / 2.0,
        (true, TextVAlign::Bottom) => h - text_h,
    };
    // Human: the angle of the node's local x axis on screen (rotation; flips are not mirrored for text).
    (text.to_doc.apply(Point::new(anchor_x, top)), text.to_doc.m[1].atan2(text.to_doc.m[0]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &Rect, b: &Rect) -> bool {
        [(a.x, b.x), (a.y, b.y), (a.w, b.w), (a.h, b.h)].iter().all(|(u, v)| (u - v).abs() < 1e-3)
    }

    #[test]
    fn a_triangle_sits_in_the_ellipse_of_its_box_and_its_corners_round() {
        // A triangle in a 298 × 298 box: top in the middle, base three quarters down.
        let sharp = polygon_points(298.0, 298.0, 3.0, 0.0);
        let expected = [[149.0, 0.0], [278.04, 223.5], [19.96, 223.5]];
        assert_eq!(sharp.len(), 3);
        for (got, want) in sharp.iter().zip(expected) {
            assert!((got[0] - want[0]).abs() < 0.05 && (got[1] - want[1]).abs() < 0.05, "{got:?}");
        }
        // Rounded: every point is the radius away from its corner's arc centre, and none sticks out of the box.
        let round = polygon_points(298.0, 298.0, 3.0, 20.0);
        assert_eq!(round.len(), 27);
        assert!(round.iter().all(|p| p[1] > 19.9 - 20.0 && p[1] <= 223.6 && p[0] >= 19.9 && p[0] <= 278.1));
        let top = round.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
        assert!((top - 20.0).abs() < 0.5, "{top}");
        // A radius larger than half a side is cut down to it, and a stretched box stretches the polygon.
        assert_eq!(polygon_points(298.0, 298.0, 3.0, 1000.0).len(), 27);
        let diamond = polygon_points(200.0, 100.0, 4.0, 0.0);
        for (got, want) in diamond.iter().zip([[100.0, 0.0], [200.0, 50.0], [100.0, 100.0], [0.0, 50.0]]) {
            assert!((got[0] - want[0]).abs() < 1e-3 && (got[1] - want[1]).abs() < 1e-3, "{got:?}");
        }
    }

    #[test]
    fn pictures_cover_contain_or_stretch_their_box() {
        // A 200 × 100 picture in a 100 × 100 box.
        let (picture, size) = ((200.0, 100.0), (100.0, 100.0));
        assert!(close(&Fit::Cover.place(picture, size), &Rect::new(-50.0, 0.0, 200.0, 100.0)));
        assert!(close(&Fit::Contain.place(picture, size), &Rect::new(0.0, 25.0, 100.0, 50.0)));
        assert!(close(&Fit::Stretch.place(picture, size), &Rect::new(0.0, 0.0, 100.0, 100.0)));
    }

    #[test]
    fn a_contained_picture_is_cut_off_at_its_edges() {
        // One triangle covering the left half of a 100 × 100 box, cut to the band y = 25..75.
        let buffers = VertexBuffers { vertices: vec![[0.0, 0.0], [100.0, 0.0], [0.0, 100.0]], indices: vec![0, 1, 2] };
        let (vertices, indices) = clip_triangles(&buffers, &Rect::new(0.0, 25.0, 100.0, 50.0));
        assert!(!indices.is_empty() && indices.len() % 3 == 0);
        assert!(vertices.iter().all(|v| v[1] >= 25.0 - 1e-3 && v[1] <= 75.0 + 1e-3));
        // A triangle wholly outside leaves nothing.
        let (_, none) = clip_triangles(&buffers, &Rect::new(200.0, 200.0, 10.0, 10.0));
        assert_eq!(none, [] as [u32; 0]);
    }

    #[test]
    fn the_crop_transform_picks_the_right_half() {
        // The schema's example: [2, 0, 0, 1, -1, 0] is the right half of the picture.
        let crop = affine_of(&serde_json::json!([2, 0, 0, 1, -1, 0])).and_then(|t| t.invert()).unwrap();
        let left = crop.apply(Point::new(0.0, 0.0));
        let right = crop.apply(Point::new(1.0, 1.0));
        assert!((left.x - 0.5).abs() < 1e-6 && (right.x - 1.0).abs() < 1e-6 && (right.y - 1.0).abs() < 1e-6);
        assert!(affine_of(&serde_json::json!([1, 0, 0])).is_none());
    }
}
