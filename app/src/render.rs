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
mod pattern;
mod pdf_out;
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
/// Glyphs are laid out no larger than this many pixels of the screen (larger ones would fill egui's font atlas);
/// text that is to be larger is laid out at this size and its triangles are scaled up.
// Human: the font atlas is one picture all letters share. When it is four fifths full egui empties it and
// draws every letter again, the interface's too; a letter 1200 pixels high fills it after thirty letters.
const MAX_GLYPH_PX: f32 = 384.0;
/// Letters up to this many pixels of the screen high are laid out at exactly the size they are drawn at.
const EXACT_GLYPH_PX: f32 = 64.0;
/// How many sizes larger letters are laid out at between one size and its double.
const GLYPH_STEPS: f32 = 4.0;
/// The longest side egui's font atlas may have, in pixels (see [`glyph_scale`]); the app sets it each frame.
// Human: egui makes the atlas as wide as the largest picture the graphics card takes (8192 pixels here) and
// lets it grow as high: a quarter of a gigabyte, sent to the graphics card whole each time it doubles, which
// stalled the window. At 4096 it is a quarter of that at most. Pictures in designs are no larger either.
pub const MAX_ATLAS_SIDE: usize = 4096;

/// The scale to lay text of `size` out at when it is drawn at `z` screen points per unit on a screen of
/// `per_point` pixels a point, and how much its triangles have to be scaled afterwards (1 when it is laid
/// out at the size it is drawn at). `steady` is whether the zoom is at rest.
// Human: every size a letter is drawn at is another picture of it in the font atlas. A zoom on the move is
// another size with every frame, which filled the atlas within a second and stalled the window each time the
// atlas was sent to the graphics card again. So while the zoom moves, text is laid out at the power of two
// nearest the zoom and its triangles are scaled to fit: a little soft for a moment, and sharp again at rest.
// Human: at rest, text up to 64 pixels high is laid out at its very size, where every pixel of a letter
// counts. Larger letters are the ones that fill the atlas, and they look the same drawn a little smaller
// than they were laid out: they are laid out at the next of four sizes to each doubling and scaled down by
// a sixth at most. Resting at one zoom after another then comes back to the same few sizes.
fn glyph_scale(size: f32, z: f32, per_point: f32, steady: bool) -> (f32, f32) {
    let step_up = |v: f32, steps: f32| ((v.max(f32::MIN_POSITIVE).log2() * steps).ceil() / steps).exp2();
    let high = size * z * per_point;
    let wanted = if !steady {
        z.max(f32::MIN_POSITIVE).log2().round().exp2()
    } else if high <= EXACT_GLYPH_PX {
        z
    } else {
        z * step_up(high, GLYPH_STEPS) / high
    };
    let laid = wanted.min(MAX_GLYPH_PX / (size * per_point).max(f32::EPSILON));
    (laid, z / laid)
}

/// One thing to draw, in document coordinates.
#[derive(Clone)]
enum Item {
    /// Filled triangles (shapes, strokes), shared with the jobs that paint them into effect pictures. `memo`
    /// keeps what was worked out from the mesh (it never changes).
    Mesh { mesh: std::sync::Arc<Mesh>, bounds: Rect, clip: Option<Rect>, memo: Memo },
    /// A text block.
    Text { text: Box<TextItem>, bounds: Rect, clip: Option<Rect> },
    /// A soft drop shadow under an unrotated box that hides what is under it.
    Shadow { rect: Rect, radius: f32, offset: Point, blur: f32, color: Color32, clip: Option<Rect> },
}

/// What was worked out from a mesh item's triangles, kept with the item.
// Human: a design with hundreds of glass panes asks of every mesh behind each pane where it lies and what it
// looks like. Going through a mesh's corners for every pane that asks was most of a build of a large design.
#[derive(Clone, Default)]
struct Memo {
    /// The part of the document the mesh paints on.
    extent: std::cell::Cell<Option<Rect>>,
    /// A hash of the mesh's look at so many painted pixels per unit (the bits of that number, then the hash).
    look: std::cell::Cell<Option<(u32, u64)>>,
}

#[derive(Clone)]
struct TextItem {
    style: TextStyle,
    color: Color32,
    to_doc: Affine,
    size: (f32, f32),
    /// The circle the text runs along instead of a straight line, when it has one.
    circle: Option<OnCircle>,
    /// The line the text runs along instead of a straight one, when it has one (and no circle).
    track: Option<std::sync::Arc<Track>>,
    /// The outlines round its letters, the outermost (drawn first) first: each one's colour and how far it
    /// reaches out of the letters.
    outlines: Vec<(Color32, f32)>,
    /// A number for the letters it shows ([`letters_key`]).
    key: u64,
    /// The face it is drawn in, chosen when the item was made.
    face: FaceChoice,
}

/// A number that texts with the same letters in the same face and size share: what one of them has brought
/// into the font atlas serves the others.
fn letters_key(style: &TextStyle) -> u64 {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (&style.content, &style.family, style.size.to_bits(), style.weight, style.italic).hash(&mut hasher);
    hasher.finish()
}

/// Which letters are in the font atlas at which sizes, as far as the canvas has drawn them.
// Human: text drawn at a size for the first time has its letters drawn into the font atlas, which takes a
// few milliseconds a paragraph of large letters. A zoom that reaches a new step, or comes to rest, wants
// that for every text on screen in the same frame: a frame of 10 to 30 ms. So each frame brings in new
// sizes for a few milliseconds only; the texts it didn't get to are drawn at the size they had, scaled,
// and get their turn in the frames after.
#[derive(Default)]
struct Warmth {
    /// The letters and sizes drawn so far.
    known: std::cell::RefCell<std::collections::HashSet<(u64, u32)>>,
    /// The size each text's letters were last laid out at.
    last: std::cell::RefCell<std::collections::HashMap<u64, f32>>,
    /// When this frame's drawing began, and whether it has brought in a new size yet.
    frame: std::cell::Cell<Option<(std::time::Instant, bool)>>,
    /// How full the font atlas was at the last frame; less than that, and it was emptied.
    fill: std::cell::Cell<f32>,
}

/// How long one frame draws letters into the font atlas for sizes new to them, when it can draw them at
/// their last size instead.
const TEXT_BUDGET: std::time::Duration = std::time::Duration::from_millis(3);
/// How many letter sizes are remembered before the memory starts over.
const MOST_KNOWN_SIZES: usize = 50_000;

impl Warmth {
    /// Starts a frame.
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "the method's path is not general over the font view's lifetime and doesn't compile"
    )]
    fn begin(&self, ctx: &egui::Context) {
        let fill = ctx.fonts(|fonts| fonts.font_atlas_fill_ratio());
        // Human: egui empties the atlas when it is nearly full; nothing is in it then.
        if fill < self.fill.get() || self.known.borrow().len() > MOST_KNOWN_SIZES {
            self.known.borrow_mut().clear();
            self.last.borrow_mut().clear();
        }
        self.fill.set(fill);
        self.frame.set(Some((std::time::Instant::now(), false)));
    }

    /// The size to lay the text with the letters `key` out at, when `wanted` is the size it should have: that
    /// size, or the one it had when this frame has no time left to bring in another. Says with `false` that
    /// the text is still to get its size.
    fn size(&self, key: u64, wanted: f32) -> (f32, bool) {
        let at = (key, wanted.to_bits());
        if !self.known.borrow().contains(&at) {
            let late = self.frame.get().is_some_and(|(began, brought)| brought && began.elapsed() >= TEXT_BUDGET);
            if let Some(had) = self.last.borrow().get(&key).copied().filter(|_| late) {
                return (had, false);
            }
            self.known.borrow_mut().insert(at);
            self.frame.set(self.frame.get().map(|(began, _)| (began, true)));
        }
        self.last.borrow_mut().insert(key, wanted);
        (wanted, true)
    }
}

/// The property of a text layer that sets it along a line: SVG path data in the layer's own coordinates.
pub const ON_PATH: &str = "onPath";
/// How far the straight pieces a text's line is followed by may lie from its curves, in document units.
const TRACK_TOLERANCE: f32 = 0.02;

/// The line a text runs along: its corners in the layer's box, each with how far along the line it lies.
// Human: a name on a waving scroll, along an arch, up a slope. The line is the baseline; the text reads the
// way the line was drawn, its letters standing on the left of that direction (upright on a line drawn from
// left to right). Where on the line the text sits is its `textAlign`: at the start, in the middle, at the end.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    corners: Vec<(Point, f32)>,
}

impl Track {
    /// The line a text layer says it runs along; `None` for a text without one, or with one that is no line.
    #[must_use]
    pub fn of(node: &Node) -> Option<Self> {
        use lyon_tessellation::path::iterator::PathIterator;
        let segments = svg::parse(node.extra.get(ON_PATH).and_then(Value::as_str)?).ok()?;
        // Written in the layer's own coordinates: a unit of the path is a unit of the layer.
        let shape = build_path(&segments, Rect::new(0.0, 0.0, 1.0, 1.0), 1.0, 1.0);
        let mut corners: Vec<(Point, f32)> = Vec::new();
        for event in shape.iter().flattened(TRACK_TOLERANCE) {
            match event {
                // Human: the first outline is the line; a path with several has no one direction to read in.
                lyon_tessellation::path::Event::Begin { at } if corners.is_empty() => corners.push((Point::new(at.x, at.y), 0.0)),
                lyon_tessellation::path::Event::Line { to, .. } => {
                    let Some(&(last, along)) = corners.last() else { break };
                    let step = (to.x - last.x).hypot(to.y - last.y);
                    if step > 1e-4 {
                        corners.push((Point::new(to.x, to.y), along + step));
                    }
                }
                lyon_tessellation::path::Event::Begin { .. } | lyon_tessellation::path::Event::End { .. } => {
                    if corners.len() > 1 {
                        break;
                    }
                }
                lyon_tessellation::path::Event::Quadratic { .. } | lyon_tessellation::path::Event::Cubic { .. } => {}
            }
        }
        (corners.len() > 1).then_some(Self { corners })
    }

    /// How long the line is.
    #[must_use]
    pub fn length(&self) -> f32 {
        self.corners.last().map_or(0.0, |(_, along)| *along)
    }

    /// The point `along` the line from its start, and the angle the line runs at there. Before its start and
    /// past its end the line goes on straight, so a text longer than its line is not piled up at the ends.
    #[must_use]
    pub fn at(&self, along: f32) -> (Point, f32) {
        let next = self.corners.partition_point(|(_, reached)| *reached < along).clamp(1, self.corners.len().saturating_sub(1));
        let (Some(&(from, start)), Some(&(to, _))) = (self.corners.get(next.saturating_sub(1)), self.corners.get(next)) else {
            return (Point::new(0.0, 0.0), 0.0);
        };
        let angle = (to.y - from.y).atan2(to.x - from.x);
        let by = along - start;
        (Point::new(from.x + by * angle.cos(), from.y + by * angle.sin()), angle)
    }
}

/// Whether the text layer `node` runs along a circle or a line instead of straight.
#[must_use]
pub fn runs_along(node: &Node) -> bool {
    node.extra.contains_key(ON_CIRCLE) || node.extra.contains_key(ON_PATH)
}

/// The property that makes a layer a mask: the layers after it in its group or frame are cut to its shape.
pub const MASK: &str = "mask";

/// Whether `node` is a mask for the layers that follow it.
#[must_use]
pub fn is_mask(node: &Node) -> bool {
    node.extra.get(MASK).and_then(Value::as_bool) == Some(true)
}

/// The box round the letters of the text layer `scene` as they are drawn, in the document: what a model
/// needs to centre a name on a mark, since a text's own box is taller than its letters. `None` for a text
/// that is empty, turned or set on a circle.
// Agent: lays the text out through the egui context; MAIN-THREAD.
#[must_use]
pub fn ink_box(ctx: &egui::Context, scene: &SceneNode, vars: Vars<'_>) -> Option<Rect> {
    let node = &scene.node;
    if node.kind != Kind::Text || runs_along(node) {
        return None;
    }
    let mut style = layout::text_style(node, vars, &scene.theme);
    if style.content.trim().is_empty() {
        return None;
    }
    if matches!(node.text_growth.as_deref(), Some("fixed-width" | "fixed-width-height")) {
        style.wrap = Some(scene.local.w);
    }
    // Laid out so its letters are about `INK_LETTER` pixels high: their edges are then known to well under
    // a hundredth of the text's size, and small text doesn't fill the font picture with large letters.
    let laid = (INK_LETTER / style.size.max(f32::EPSILON)).clamp(0.5, INK_SCALE);
    let galley = ctx.fonts_mut(|fonts| fonts.layout_job(text_job(&style, Color32::WHITE, laid)));
    let text = TextItem {
        key: letters_key(&style),
        face: FaceChoice::of(&style),
        style,
        color: Color32::WHITE,
        to_doc: scene.to_doc,
        size: (scene.local.w, scene.local.h),
        circle: None,
        track: None,
        outlines: Vec::new(),
    };
    let (anchor, angle) = text_anchor(&text, galley.size().y / laid);
    if angle.abs() > 1e-3 {
        return None;
    }
    let mut ink: Option<egui::Rect> = None;
    for row in &galley.rows {
        let letters = row.row.visuals.mesh_bounds;
        if letters.is_finite() && letters.width() > 0.0 {
            let letters = letters.translate(row.pos.to_vec2());
            ink = Some(ink.map_or(letters, |all| all.union(letters)));
        }
    }
    let ink = ink?;
    Some(Rect::new(anchor.x + ink.min.x / laid, anchor.y + ink.min.y / laid, ink.width() / laid, ink.height() / laid))
}

/// The most a text is enlarged to find where its letters lie.
const INK_SCALE: f32 = 4.0;
/// The height of letters, in pixels, a text is laid out at for that.
const INK_LETTER: f32 = 192.0;

/// The property of a text layer that sets it along a circle: `"top"` or `"bottom"`.
pub const ON_CIRCLE: &str = "onCircle";
/// The property that turns such a text about the circle's centre, in degrees clockwise.
pub const CIRCLE_ANGLE: &str = "circleAngle";

/// How a text runs along the circle that fits its layer's box.
// Human: a seal, a roundel or a badge has its name set round its rim. The baseline is the circle itself. At
// the top the text reads clockwise and its letters stand outward; at the bottom it reads from left to right
// as well, so it runs the other way round and its letters stand inward, heads toward the centre.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OnCircle {
    /// Whether the text is centred on the lowest point of the circle instead of the highest.
    pub bottom: bool,
    /// How far the text's middle is turned from there, clockwise, in radians.
    pub turned: f32,
}

impl OnCircle {
    /// What a text layer says about running along a circle; `None` for ordinary text.
    #[must_use]
    pub fn of(node: &Node) -> Option<Self> {
        let bottom = match node.extra.get(ON_CIRCLE).and_then(Value::as_str)? {
            "top" => false,
            "bottom" => true,
            _ => return None,
        };
        let turned = node.extra.get(CIRCLE_ANGLE).and_then(Value::as_f64).map_or(0.0, narrow).to_radians();
        Some(Self { bottom, turned })
    }

    /// Where on the circle (the angle from its right, clockwise on screen) the point lies that is `along`
    /// radians of text from the text's middle, and the angle a letter there is drawn at.
    #[must_use]
    pub fn at(self, along: f32) -> (f32, f32) {
        let quarter = std::f32::consts::FRAC_PI_2;
        if self.bottom {
            let place = quarter - along + self.turned;
            (place, place - quarter)
        } else {
            let place = along - quarter + self.turned;
            (place, place + quarter)
        }
    }
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
    /// Which of the items belong to which top-level node of the scene; empty when that isn't known (a list
    /// with pictures still missing), and nothing can be taken over from it.
    parts: Vec<Part>,
    /// The design's pictures and the pictures its effects were baked into, kept from one build to the next;
    /// behind a lock, since a build beside the window uses them too.
    works: std::sync::Arc<std::sync::Mutex<Works>>,
    /// How many pixels the baked pictures had between them after the last build.
    held: usize,
    /// The copies of the scene's top-level nodes that builds beside the window work on, each with the
    /// stamp its node had when it was copied: a node is copied again only when its stamp is another one.
    frozen: Vec<(u64, std::sync::Arc<SceneNode>)>,
    /// The build that is being made beside the window.
    running: Option<Running>,
    /// The part of the document whose effects are baked, when the last build left some out; `None` when it
    /// baked them all.
    cover: Option<Rect>,
    /// The zoom at the last look, and when it last differed from the look before: a zoom still on the move
    /// isn't followed step by step.
    zoom_seen: f32,
    /// See `zoom_seen`.
    zoom_moved: Option<std::time::Instant>,
    /// What was on screen at the last look, and when that last changed (or effects were last added to).
    view_seen: Rect,
    /// See `view_seen`.
    view_moved: Option<std::time::Instant>,
    /// How far past the screen the last build baked effects, as a share of the screen's width and height.
    margin: f32,
    /// The layers the list was built without (see [`crate::activity::Activity::hidden`]).
    hidden: std::collections::HashSet<String>,
    /// Which stage of a model's change the list was built from (see [`crate::activity::Activity::stage`]).
    stage: u64,
    /// Whether the zoom was on the move at the last look: text is then drawn at the nearest zoom step.
    zooming: bool,
    /// Which letters the canvas has drawn at which sizes.
    warmth: Warmth,
    /// The meshes as they were last drawn, in screen coordinates.
    moved: std::cell::RefCell<Moved>,
}

/// The meshes of the last frame in screen coordinates, kept for the next one.
// Human: most frames show the design where the frame before showed it (a chat animates, a cursor blinks).
// Moving every mesh to the screen anew each time was most of such a frame, so the moved meshes are kept, one
// per item, for as long as the camera and the canvas's place in the window stay as they are.
// Human: while the camera moves (a pan, a zoom) every frame is another look, and meshes kept for it would be
// thrown away with the next: measured, that made a panning frame half again as slow. So nothing is kept in
// the first frame of a look, only from the second one on, when the look has shown that it stays.
// Agent: a slot holds the mesh it was moved from too, and counts only while the item still has that very
// mesh (a rebuilt item has another). An item that is not drawn in a frame gives its slot up, so no more is
// kept than is on screen.
#[derive(Default)]
struct Moved {
    /// The camera and view the slots were moved for.
    seen: Option<(Camera, egui::Rect)>,
    /// Whether this frame's meshes are kept: the look is the one of the frame before.
    keeping: bool,
    /// For each item of the list: the mesh it was moved from, and the moved one.
    slots: Vec<Option<(std::sync::Arc<Mesh>, std::sync::Arc<Mesh>)>>,
}

impl Moved {
    /// Starts a frame of a list of `len` items seen through `camera` in `view`: another look empties it.
    fn begin(&mut self, camera: &Camera, view: egui::Rect, len: usize) {
        self.keeping = self.seen == Some((*camera, view));
        if !self.keeping {
            self.slots.clear();
            self.seen = Some((*camera, view));
            return;
        }
        self.slots.resize_with(len, || None);
    }

    /// Item `at` is not drawn as a mesh in this frame.
    fn skip(&mut self, at: usize) {
        if let Some(slot) = self.slots.get_mut(at) {
            *slot = None;
        }
    }

    /// `mesh` (item `at`) in screen coordinates: the one kept, or made with `to_screen` and kept.
    fn of(&mut self, at: usize, mesh: &std::sync::Arc<Mesh>, to_screen: impl Fn(Pos2) -> Pos2) -> std::sync::Arc<Mesh> {
        if let Some(Some((from, there))) = self.slots.get(at)
            && std::sync::Arc::ptr_eq(from, mesh)
        {
            return there.clone();
        }
        let mut there = Mesh::clone(mesh);
        for v in &mut there.vertices {
            v.pos = to_screen(v.pos);
        }
        let there = std::sync::Arc::new(there);
        if self.keeping
            && let Some(slot) = self.slots.get_mut(at)
        {
            *slot = Some((mesh.clone(), there.clone()));
        }
        there
    }
}

/// What a build needs besides the scene: the design's pictures, and the pictures effects were baked into.
#[derive(Default)]
struct Works {
    images: ImageStore,
    baked: bake::Baked,
}

/// A scene as it was when a build beside the window began: its top-level nodes, their stamps (see
/// [`DisplayList::update`]) and the document's variables.
// Human: the build can't read the scene the window has, which changes with the next edit. Its top-level
// nodes are copied for it, each one once: an edit changes one of dozens, and only that one is copied again.
struct Frozen {
    scene: Vec<std::sync::Arc<SceneNode>>,
    stamps: Vec<u64>,
    variables: Option<serde_json::Map<String, Value>>,
}

/// What the list holds for one top-level node of the scene: a run of its items.
// Human: a design is dozens of screens side by side, and an edit changes one. The list remembers which of its
// items came from which top-level node, so the nodes an edit left alone keep theirs.
#[derive(Clone)]
struct Part {
    /// The node's stamp when its items were built.
    stamp: u64,
    /// How many items it has; the parts' items follow each other in the list.
    len: usize,
    /// The part of the document it reads to draw what is behind it (glass, a background blur), if it does:
    /// it is built again when a node under it changes there.
    reads: Option<Rect>,
    /// The part of the document its items paint on.
    extent: Option<Rect>,
    /// Which of its items each layer in it brought, in the order the layers are built ([`Span`]).
    spans: Vec<Span>,
}

/// The items one layer brought, with everything in it: a run of the items of its top-level node.
// Human: a drag moves one layer of a screen with thousands. The screen's items are kept, and only the moved
// layer's are made anew and put in the place of its old ones; for that the list has to know which they are.
// Agent: the spans of a top-level node follow each other the way its layers are built: a layer, then each
// layer in it with all of its own. `nodes` is how many spans a layer has with everything in it, so the next
// layer beside it is that many further on. Only right for a layer none of whose parents gathers its
// children's items to work on them ([`Builder::plain`]); for others it is never asked.
#[derive(Clone, Copy)]
struct Span {
    /// Where its items begin and end, counted from the first item of its top-level node.
    start: usize,
    end: usize,
    nodes: usize,
    /// Whether it handed its children's items on as they were made ([`Builder::plain`]) when it was built.
    plain: bool,
}

/// The spans of the layers a builder goes through, while it is asked to keep them.
#[derive(Default)]
struct Spans {
    on: std::cell::Cell<bool>,
    list: std::cell::RefCell<Vec<Span>>,
}

impl Spans {
    /// A layer's items begin at item `at`; gives the layer's place among the spans.
    fn begin(&self, at: usize) -> Option<usize> {
        if !self.on.get() {
            return None;
        }
        let mut list = self.list.borrow_mut();
        list.push(Span { start: at, end: at, nodes: 1, plain: false });
        Some(list.len() - 1)
    }

    /// The layer at `place` and everything in it ended before item `at`; `plain` says whether it handed its
    /// children's items on as they were made.
    fn end(&self, place: Option<usize>, at: usize, plain: bool) {
        let mut list = self.list.borrow_mut();
        let count = list.len();
        if let Some(place) = place
            && let Some(span) = list.get_mut(place)
        {
            (span.end, span.nodes, span.plain) = (at, count - place, plain);
        }
    }

    /// Keeps spans from now on, and gives the ones kept so far, counted from item `from`.
    fn take(&self, from: usize) -> Vec<Span> {
        self.on.set(true);
        let mut spans = std::mem::take(&mut *self.list.borrow_mut());
        for span in &mut spans {
            (span.start, span.end) = (span.start.saturating_sub(from), span.end.saturating_sub(from));
        }
        spans
    }
}

/// What a build is made for.
#[derive(Clone)]
struct Order {
    /// The zoom, for how finely curves are flattened.
    zoom: f32,
    /// The zoom step.
    bucket: i32,
    /// The part of the document effects are baked for; `None` bakes them all.
    cover: Option<Rect>,
    folder: Option<std::path::PathBuf>,
    background: Color32,
    hidden: std::collections::HashSet<String>,
}

/// A build being made beside the window.
struct Running {
    generation: u64,
    stage: u64,
    bucket: i32,
    cover: Rect,
    /// Set to call the build off.
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Where its lists arrive: unfinished ones as pictures come in, then the whole one.
    arrives: std::sync::mpsc::Receiver<Built>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// A list as a build beside the window made it.
struct Built {
    items: Vec<Item>,
    parts: Vec<Part>,
    /// Whether every picture in it is there; else some are still being made and a later list follows.
    whole: bool,
    /// Whether effects outside the part built for were left out.
    skipped: bool,
    /// How many pixels the baked pictures have between them.
    held: usize,
    /// Pictures the build no longer keeps; the list on screen may still show them, so they are let go
    /// only once this list has taken its place.
    let_go: Vec<egui::TextureHandle>,
}

/// The list on screen as a walk takes it over: its items and parts, and the top-level nodes of which only
/// some layers were built again since ([`crate::editor::Editor::rebuilt`]).
#[derive(Default)]
struct Taken<'a> {
    items: Vec<Item>,
    parts: Vec<Part>,
    rebuilt: &'a [crate::editor::Rebuilt],
}

/// One walk through `scene`: its items, with the effects' pictures taken from `works` or started there, and
/// which of them belong to which top-level node. `stamps` are the scene's (see [`DisplayList::update`]).
/// With `kept` (the list on screen and its parts, built for the same order), a node whose stamp is the one
/// its part was built at keeps that part's items, and only the others are built; of a node `rebuilt` names,
/// only the layers it names are, where that can be done ([`Builder::rebuild`]).
// Human: a kept part is still built again when it draws what is behind it (glass, a background blur) and
// a node under it changed there, in what it painted before or in what it paints now.
fn walk(
    works: &Works,
    (scene, stamps): (&[&SceneNode], &[u64]),
    vars: Vars<'_>,
    ctx: &egui::Context,
    order: &Order,
    stop: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    kept: Option<Taken<'_>>,
) -> (Vec<Item>, Vec<Part>) {
    works.baked.begin_within(order.cover, stop);
    let builder = Builder {
        vars,
        tolerance: (0.25 / order.zoom).clamp(0.002, 4.0),
        images: &works.images,
        ctx,
        folder: order.folder.as_deref(),
        baked: &works.baked,
        background: order.background,
        scale: 2.0_f32.powi(order.bucket) * ctx.pixels_per_point(),
        merged: std::cell::Cell::new(false),
        hidden: Some(&order.hidden),
        reads: std::cell::Cell::new(None),
        spans: Spans::default(),
    };
    let taking_over = kept.is_some();
    let Taken { items: was, parts: had, rebuilt } = kept.unwrap_or_default();
    let mut was = was.into_iter();
    let (mut items, mut parts) = (Vec::new(), Vec::with_capacity(scene.len()));
    // Where the nodes built again painted before and paint now.
    let mut changed: Vec<Rect> = Vec::new();
    for (at, node) in scene.iter().copied().enumerate() {
        let stamp = stamps.get(at).copied().unwrap_or_default();
        let part = had.get(at);
        let holds = |part: &&Part| part.stamp == stamp && !part.reads.is_some_and(|area| changed.iter().any(|there| there.intersects(&area)));
        if let Some(part) = part.filter(holds) {
            items.extend(was.by_ref().take(part.len));
            parts.push(part.clone());
            continue;
        }
        let start = items.len();
        if let Some(part) = part {
            let mut old: Vec<Item> = was.by_ref().take(part.len).collect();
            // Human: only some layers of the node were built again (a drag, a number being pulled): their
            // items are made anew and take the place of their old ones; the node's other items stay.
            let paths = rebuilt.iter().find(|some| some.root == at && some.from == part.stamp && some.to == stamp).map(|some| &some.paths);
            if let Some(paths) = paths
                && let Some((spans, reads, touched)) = builder.rebuild(node, part, &mut old, paths)
            {
                changed.extend(touched);
                let extent = old.iter().filter_map(bake::extent).reduce(|a, b| a.union(&b));
                parts.push(Part { stamp, len: old.len(), reads, extent, spans });
                items.extend(old);
                continue;
            }
            changed.extend(part.extent);
        }
        builder.reads.set(None);
        drop(builder.spans.take(0));
        builder.node(node, 1.0, None, &mut items);
        let spans = builder.spans.take(start);
        let extent = items.get(start..).unwrap_or_default().iter().filter_map(bake::extent).reduce(|a, b| a.union(&b));
        changed.extend(extent);
        parts.push(Part { stamp, len: items.len() - start, reads: builder.reads.get(), extent, spans });
    }
    if taking_over {
        // The pictures of the parts taken over were not asked for by this walk; they are still in use.
        let textured = |item: &Item| match item {
            Item::Mesh { mesh, .. } if mesh.texture_id != egui::TextureId::default() => Some(mesh.texture_id),
            _ => None,
        };
        works.baked.hold(&items.iter().filter_map(textured).collect());
    }
    (items, parts)
}

/// How long a build beside the window sleeps between looks at whether it can go on.
const BUILD_NAP: std::time::Duration = std::time::Duration::from_millis(2);

/// Builds the list for `order` from `frozen`, beside the window, and sends it to `out`: after each walk when
/// `showing` (the list on screen is of the same zoom step, so one with pictures still missing is the better
/// one to show), and the whole one at the end. Ends early once `stop` is set.
// Human: a walk through a large design takes tens of milliseconds, and one that hands pictures out to be made
// has to be gone through again when they arrive, several times where glass lies behind glass. All of that
// is done here, away from the window, which only swaps the finished list in.
// Agent: runs on a thread of its own; HOLDS the lock on `works` for the length of one walk, never while it waits.
fn build_apart(
    works: &std::sync::Mutex<Works>,
    frozen: &Frozen,
    ctx: &egui::Context,
    order: &Order,
    showing: bool,
    stop: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    out: &std::sync::mpsc::Sender<Built>,
) {
    use std::sync::atomic::Ordering::Relaxed;
    let scene: Vec<&SceneNode> = frozen.scene.iter().map(std::convert::AsRef::as_ref).collect();
    loop {
        let (built, under_way, more) = {
            let works = crate::app::lock(works);
            let vars = Vars::of(frozen.variables.as_ref());
            let (items, parts) = walk(&works, (&scene, &frozen.stamps), vars, ctx, order, Some(stop.clone()), None);
            if stop.load(Relaxed) {
                return;
            }
            let whole = !works.baked.late();
            let let_go = if whole { works.baked.finish() } else { Vec::new() };
            let (under_way, more) = works.baked.waiting();
            (Built { items, parts, whole, skipped: works.baked.skipped(), held: works.baked.pixels_held(), let_go }, under_way, more)
        };
        let whole = built.whole;
        if (whole || showing) && out.send(built).is_ok() {
            ctx.request_repaint();
        }
        if whole {
            return;
        }
        while !bake::go_on(under_way.load(Relaxed), more) {
            if stop.load(Relaxed) {
                return;
            }
            std::thread::sleep(BUILD_NAP);
        }
    }
}

/// A mesh or a shadow smaller than this many pixels of the screen, both ways, is not drawn.
// Human: measured on a design of 11,000 layers shown whole on a sharp screen: a quarter of everything drawn
// is smaller than this, and without it a frame takes 6.5 ms of the processor's time instead of 10. At half
// a pixel next to nothing was left out; at two pixels icons went missing that could still be told.
const SMALLEST_DRAWN: f32 = 1.0;

/// Whether something `long` document units long on its longer side is too small to draw at `shown_px` pixels
/// of the screen per unit.
fn too_small(long: f32, shown_px: f32) -> bool {
    long * shown_px < SMALLEST_DRAWN
}
/// How long the zoom must hold still before the list is rebuilt for its new step.
const ZOOM_SETTLE: std::time::Duration = std::time::Duration::from_millis(120);
/// How far past the visible part of the canvas effects are baked at first, as a share of its width and height.
const BAKE_MARGIN: f32 = 0.5;
/// How long the view must rest before effects further out are baked, a step at a time.
const BAKE_ON_AFTER: std::time::Duration = std::time::Duration::from_millis(300);
/// Effects further out are only baked while the pictures held are fewer pixels than this (128 MB).
const BAKE_ON_BELOW: usize = 32_000_000;
impl DisplayList {
    /// Rebuilds the list when the scene changed, when the zoom has come to rest on another power of two, or when
    /// `visible` (the part of the document on screen) has left the part whose effects are baked.
    // Human: Curves are flattened to a tolerance that depends on the zoom, so zooming far in re-tessellates
    // once per doubling instead of showing facets.
    // Human: Effects (blurs, and the shadows egui can't draw) are baked into pictures at the pixel size of that
    // power of two, so they stay sharp while zooming in; `background` is the canvas colour behind the design.
    // Human: a build costs up to seconds on a design with many effects, nearly all of it baking. So it waits
    // until a zoom has held still for a moment (the list drawn meanwhile is the old one, scaled), and it bakes
    // only what is on screen or near it. Zooming out or panning past that part rebuilds at once: effects
    // that were left out would otherwise be seen missing. And while the view rests, the part that is baked
    // grows a step at a time, until it is the whole design or the pictures would take too much memory: after
    // a short rest panning costs nothing.
    // Human: even so, a build for a new zoom step or a new part of the canvas took from a fifth of a second
    // to several seconds in one go, with the window frozen. A build for a scene that is on screen already is
    // therefore made beside the window, on a thread of its own, from a copy of the scene; the window goes on
    // drawing the list it has. At the same zoom step it shows the build's list after each walk, with the
    // pictures that have arrived so far. For another zoom step the list on screen stays, scaled, until the
    // new one is whole, so effects are never seen going away and coming back. A scene that changed (an
    // edit, a file just opened) is still built here in one go: an edit must not be seen without its effects.
    // Human: `hidden` are layers a model made that are not shown yet (it shows its work a layer at a time): they
    // are left out with everything in them, and the list is built again when they change. `stage` says which
    // step of such a change `scene` is: it has the document's `generation`, and is another scene each time.
    // Human: `stamps` has a number for each top-level node that is another one when the node was built again,
    // and `rebuilt` says of which nodes only some layers were ([`crate::editor::Editor::rebuilt`]).
    // Agent: CALLS ctx.request_repaint_after while a rebuild is waiting for the zoom to settle or the view to
    // rest; a build beside the window asks for a frame when it has sent a list. BLOCKS on the lock a build
    // beside the window holds only when the scene changed, after calling that build off.
    #[expect(clippy::too_many_arguments, reason = "the scene, what it was built from, where its pictures live, and what is on screen")]
    pub fn update(
        &mut self,
        scene: &[SceneNode],
        generation: u64,
        zoom: f32,
        visible: Rect,
        doc: &pen_doc::Document,
        ctx: &egui::Context,
        folder: Option<&std::path::Path>,
        background: Color32,
        hidden: &std::collections::HashSet<String>,
        stage: u64,
        (stamps, rebuilt): (&[u64], &[crate::editor::Rebuilt]),
    ) {
        let bucket = zoom_bucket(zoom);
        let now = std::time::Instant::now();
        if (zoom - self.zoom_seen).abs() > f32::EPSILON {
            // The very first zoom is no move: the first build is made at once.
            self.zoom_moved = (!self.items.is_empty()).then_some(now);
            self.zoom_seen = zoom;
        }
        if visible != self.view_seen {
            self.view_seen = visible;
            self.view_moved = Some(now);
        }
        let wait = self.zoom_moved.and_then(|moved| ZOOM_SETTLE.checked_sub(now.duration_since(moved)));
        // Text drawn at a zoom step is drawn sharp once the zoom rests.
        self.zooming = wait.is_some();
        if let Some(wait) = wait {
            ctx.request_repaint_after(wait);
        }
        let same = !self.items.is_empty()
            && self.generation == generation
            && self.folder.as_deref() == folder
            && self.background == background
            && self.hidden == *hidden
            && self.stage == stage;
        let margin_of = |margin: f32| {
            Rect::new(
                visible.x - visible.w * margin,
                visible.y - visible.h * margin,
                visible.w * (1.0 + 2.0 * margin),
                visible.h * (1.0 + 2.0 * margin),
            )
        };
        if !same {
            // Human: the scene changed: a build beside the window is of the old one. It is called off (it
            // lets go of the pictures within a moment), and the new scene is built here and now.
            self.running = None;
            // Human: an edit leaves most of a design as it was. When nothing else differs from the list on
            // screen (same zoom step, same part baked), only the top-level nodes with another stamp are
            // built, and the rest of the list is taken over as it is.
            let alike = !self.items.is_empty()
                && self.folder.as_deref() == folder
                && self.background == background
                && self.hidden == *hidden
                && self.stage == stage;
            let covered = self.cover.is_none_or(|cover| cover.holds(&visible));
            let kept = (alike && covered && self.zoom_bucket == bucket && self.parts.len() == scene.len() && stamps.len() == scene.len())
                .then(|| (std::mem::take(&mut self.items), std::mem::take(&mut self.parts)));
            let taken_over = kept.is_some();
            let cover = if taken_over {
                self.cover
            } else {
                self.margin = BAKE_MARGIN;
                self.view_moved = Some(now);
                Some(margin_of(self.margin))
            };
            let order = Order { zoom, bucket, cover, folder: folder.map(std::path::Path::to_path_buf), background, hidden: hidden.clone() };
            let works = crate::app::lock(&self.works);
            let roots: Vec<&SceneNode> = scene.iter().collect();
            (self.items, self.parts) =
                walk(&works, (&roots, stamps), Vars::new(doc), ctx, &order, None, kept.map(|(items, parts)| Taken { items, parts, rebuilt }));
            drop(works.baked.finish());
            self.held = works.baked.pixels_held();
            // A list taken over keeps the part that was baked for it.
            self.cover = if taken_over { cover } else { cover.filter(|_| works.baked.skipped()) };
            drop(works);
            (self.folder, self.background, self.hidden) = (order.folder, background, order.hidden);
            (self.generation, self.stage, self.zoom_bucket) = (generation, stage, bucket);
            if self.cover.is_some() {
                ctx.request_repaint_after(BAKE_ON_AFTER);
            }
            return;
        }
        self.take_in(ctx, now);
        let rest = self.view_moved.and_then(|moved| BAKE_ON_AFTER.checked_sub(now.duration_since(moved)));
        let covered = self.cover.is_none_or(|cover| cover.holds(&visible));
        let current = covered && self.zoom_bucket == bucket;
        // Effects were left out, there is room for more, and nothing is moving: bake on.
        let more = current && self.cover.is_some() && self.held < BAKE_ON_BELOW;
        if current && !(more && rest.is_none()) {
            // A build for a zoom step the zoom has left again is of no use.
            if self.running.as_ref().is_some_and(|running| running.bucket != bucket) {
                self.running = None;
            }
            if let Some(rest) = rest.filter(|_| more && wait.is_none() && self.running.is_none()) {
                ctx.request_repaint_after(rest);
            }
            return;
        }
        if covered && wait.is_some() {
            return;
        }
        // The build being made is the one wanted: it sends its list when it has one.
        if self.running.as_ref().is_some_and(|running| running.bucket == bucket && running.cover.holds(&visible)) {
            return;
        }
        // The next step out waits for another rest.
        self.view_moved = Some(now);
        self.margin = if current { self.margin * 2.0 + BAKE_MARGIN } else { BAKE_MARGIN };
        let cover = margin_of(self.margin);
        let order = Order { zoom, bucket, cover: Some(cover), folder: folder.map(std::path::Path::to_path_buf), background, hidden: hidden.clone() };
        self.build_beside((scene, stamps), doc, (generation, stage, cover), order, ctx);
    }

    /// Starts the build for `order` beside the window; with it come the scene's generation and stage
    /// and `cover` the part of the document it bakes effects for.
    fn build_beside(
        &mut self,
        (scene, stamps): (&[SceneNode], &[u64]),
        doc: &pen_doc::Document,
        (generation, stage, cover): (u64, u64, Rect),
        order: Order,
        ctx: &egui::Context,
    ) {
        let bucket = order.bucket;
        // Human: the build works on copies of the scene's top-level nodes, since the ones here change with
        // the next edit. A node whose stamp is the one its copy was taken at keeps that copy.
        let stamped = stamps.len() == scene.len();
        let copy = |(at, node): (usize, &SceneNode)| match (stamps.get(at), self.frozen.get(at)) {
            (Some(stamp), Some((taken, copy))) if stamped && stamp == taken => copy.clone(),
            _ => std::sync::Arc::new(node.clone()),
        };
        let copies: Vec<std::sync::Arc<SceneNode>> = scene.iter().enumerate().map(copy).collect();
        // A scene without stamps (a stage of a model's change) is copied whole each time.
        self.frozen = if stamped { stamps.iter().copied().zip(copies.iter().cloned()).collect() } else { Vec::new() };
        let frozen = Frozen { scene: copies, stamps: stamps.to_vec(), variables: doc.variables.clone() };
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (send, arrives) = std::sync::mpsc::channel();
        let (works, wake, showing, called_off) = (self.works.clone(), ctx.clone(), self.zoom_bucket == bucket, stop.clone());
        let at = bucket;
        let build = move || build_apart(&works, &frozen, &wake, &order, showing, &called_off, &send);
        match std::thread::Builder::new().name("pencraft-build".to_owned()).spawn(build) {
            // Starting it calls off the build that was being made, if there was one.
            Ok(_) => self.running = Some(Running { generation, stage, bucket: at, cover, stop, arrives }),
            Err(error) => {
                log::warn!("no thread to build the canvas on; it is tried again: {error}");
                ctx.request_repaint_after(BAKE_ON_AFTER);
            }
        }
    }

    /// Takes in what the build beside the window has sent: the newest of its lists that can be shown.
    fn take_in(&mut self, ctx: &egui::Context, now: std::time::Instant) {
        let Some(running) = &self.running else { return };
        if (running.generation, running.stage) != (self.generation, self.stage) {
            self.running = None;
            return;
        }
        let mut newest = None;
        let ended = loop {
            match running.arrives.try_recv() {
                Ok(built) => {
                    // An unfinished list of another zoom step has holes where the one on screen has pictures.
                    if built.whole || running.bucket == self.zoom_bucket {
                        newest = Some(built);
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break false,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break true,
            }
        };
        let (bucket, cover) = (running.bucket, running.cover);
        // Human: a whole list is the build's last word. The build ends a moment after sending it, and a look in
        // that moment would show the new list while still counting a build as under way; so the list itself
        // says the build is over, not only the build's end.
        let ended = ended || newest.as_ref().is_some_and(|built| built.whole);
        if let Some(built) = newest {
            self.items = built.items;
            // From a list with pictures still missing nothing is taken over.
            self.parts = if built.whole { built.parts } else { Vec::new() };
            self.zoom_bucket = bucket;
            self.held = built.held;
            self.cover = (built.skipped || !built.whole).then_some(cover);
            // The list that showed them is gone now.
            drop(built.let_go);
            if built.whole {
                // The next step out waits for a rest from now.
                self.view_moved = Some(now);
                if self.cover.is_some() {
                    ctx.request_repaint_after(BAKE_ON_AFTER);
                }
            }
        }
        if ended {
            self.running = None;
        }
    }

    /// `scene` (a layer with everything in it) drawn as a picture at `scale` pixels per unit, with its shadows
    /// and whatever else reaches past its box. `None` when it is empty or too large.
    // Agent: MAIN-THREAD; uses its own effect pictures, dropped when it returns. WAITS for a walk a build
    // beside the window is in the middle of.
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
        let works = crate::app::lock(&self.works);
        Self::builder(scale, vars, ctx, folder, &works.images, &baked).snapshot(scenes)
    }

    /// [`DisplayList::snapshot`] for measuring: the picture with where it lies and what its materials came to.
    // Agent: MAIN-THREAD; bakes every effect anew, so each material is seen.
    pub fn survey(&self, scenes: &[&SceneNode], scale: f32, vars: Vars<'_>, ctx: &egui::Context, folder: Option<&std::path::Path>) -> Option<Survey> {
        let baked = bake::Baked::default();
        baked.begin();
        let works = crate::app::lock(&self.works);
        let (picture, origin) = Self::builder(scale, vars, ctx, folder, &works.images, &baked).snapshot_at(scenes)?;
        Some(Survey { picture, origin, scale, materials: baked.seen() })
    }

    /// `scenes` together as a PDF page of vectors, with the layers a PDF has no word for as pictures at the
    /// export's scale; `None` when the design shows what is behind a layer through it, which only a picture
    /// of the whole page shows.
    ///
    /// # Errors
    /// A sentence when the layers have no size or the PDF can't be made.
    // Agent: MAIN-THREAD; lays text out through `ctx` and READS the fonts and pictures the layers name.
    pub fn pdf(
        &self,
        scenes: &[&SceneNode],
        vars: Vars<'_>,
        ctx: &egui::Context,
        folder: Option<&std::path::Path>,
    ) -> Option<Result<Vec<u8>, String>> {
        let baked = bake::Baked::default();
        baked.begin();
        let works = crate::app::lock(&self.works);
        Self::builder(crate::export::PDF_SCALE, vars, ctx, folder, &works.images, &baked).pdf(scenes)
    }

    /// `scenes` together as one SVG file, or `None` when they have no size.
    pub fn svg(&self, scenes: &[&SceneNode], vars: Vars<'_>, ctx: &egui::Context, folder: Option<&std::path::Path>) -> Option<String> {
        let baked = bake::Baked::default();
        let works = crate::app::lock(&self.works);
        Self::builder(1.0, vars, ctx, folder, &works.images, &baked).svg(scenes)
    }

    /// A builder for exports: on a clear background, at `scale` pixels per unit.
    fn builder<'a>(
        scale: f32,
        vars: Vars<'a>,
        ctx: &'a egui::Context,
        folder: Option<&'a std::path::Path>,
        images: &'a ImageStore,
        baked: &'a bake::Baked,
    ) -> Builder<'a> {
        Builder {
            vars,
            tolerance: (0.25 / scale).clamp(0.002, 4.0),
            images,
            ctx,
            folder,
            baked,
            background: Color32::TRANSPARENT,
            scale,
            merged: std::cell::Cell::new(false),
            hidden: None,
            reads: std::cell::Cell::new(None),
            spans: Spans::default(),
        }
    }

    /// Draws everything visible in `view` (screen rectangle) through `camera`.
    // Agent: HOT-PATH per frame; ALLOCATES a transformed mesh per visible item only in the first frame of a
    // look (see [`Moved`]); the frames after hand egui the ones kept.
    pub fn draw(&self, painter: &Painter, camera: &Camera, view: egui::Rect) {
        let visible = view_to_doc(camera, view);
        // Pixels of the screen per document unit.
        let shown_px = camera.zoom * painter.ctx().pixels_per_point();
        self.warmth.begin(painter.ctx());
        let to_screen = |p: Pos2| screen(camera, view, Point::new(p.x, p.y));
        let mut moved = self.moved.borrow_mut();
        moved.begin(camera, view, self.items.len());
        for (at, item) in self.items.iter().enumerate() {
            match item {
                Item::Mesh { mesh, bounds, clip, .. } => {
                    // Human: a mesh without triangles (a picture fill on a frame with no height) must not
                    // reach the GPU, which refuses to draw nothing and would end the app.
                    if !bounds.intersects(&visible) || mesh.indices.is_empty() {
                        moved.skip(at);
                        continue;
                    }
                    // Human: zoomed far out, most of a large design's layers are smaller than a pixel: an
                    // icon is a speck, and thousands of specks cost as much to draw as thousands of icons.
                    // A mesh under a pixel both ways is left out. (A hairline is long one way and stays.)
                    if bake::extent(item).is_some_and(|painted| too_small(painted.w.max(painted.h), shown_px)) {
                        moved.skip(at);
                        continue;
                    }
                    clipped(painter, camera, view, *clip).add(Shape::Mesh(moved.of(at, mesh, to_screen)));
                }
                Item::Text { text, bounds, clip } => {
                    moved.skip(at);
                    if bounds.intersects(&visible) {
                        draw_text(&clipped(painter, camera, view, *clip), camera, view, text, !self.zooming, &self.warmth);
                    }
                }
                Item::Shadow { rect, radius, offset, blur, color, clip } => {
                    moved.skip(at);
                    if !rect.intersects(&visible) || too_small(rect.w.max(rect.h) + blur, shown_px) {
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
    /// A mark repeated across the layer (see [`pattern`]).
    Pattern(pattern::Pattern),
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
            // Only where a pattern can't be drawn as its marks (a stroke): its colour.
            Brush::Pattern(pattern) => pattern.color,
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
    /// The layers left out (by scene id); a picture for a file or a model leaves none out.
    hidden: Option<&'a std::collections::HashSet<String>>,
    /// The part of the document whose items the nodes built so far read to draw what is behind them
    /// (glass, a background blur); `None` while none did. The list takes it down for each top-level node.
    reads: std::cell::Cell<Option<Rect>>,
    /// Which items each layer brought, while the list asks for them.
    spans: Spans,
}

/// The line a stroke is drawn with.
#[derive(Clone, Copy)]
struct Pen {
    width: f32,
    cap: LineCap,
    join: LineJoin,
}

/// The pen `node`'s stroke of `width` is drawn with.
// Human: a connector's line ends and turns round unless it says otherwise; every other stroke is cut off square
// and turns in a point, as SVG does.
fn pen_for(node: &Node, width: f32) -> Pen {
    let wire = node.kind == Kind::Connector;
    let cap = match node.extra.get("strokeLinecap").and_then(Value::as_str) {
        Some("round") => LineCap::Round,
        Some("square") => LineCap::Square,
        None if wire => LineCap::Round,
        Some(_) | None => LineCap::Butt,
    };
    let join = match node.extra.get("strokeLinejoin").and_then(Value::as_str) {
        Some("round") => LineJoin::Round,
        Some("bevel") => LineJoin::Bevel,
        None if wire => LineJoin::Round,
        Some(_) | None => LineJoin::Miter,
    };
    Pen { width, cap, join }
}

impl Builder<'_> {
    /// Notes that what is being built draws what lies behind it in `area`.
    fn reads_behind(&self, area: Rect) {
        self.reads.set(Some(self.reads.get().map_or(area, |read| read.union(&area))));
    }

    /// Whether `scene` is left out for now.
    fn hides(&self, scene: &SceneNode) -> bool {
        self.hidden.is_some_and(|hidden| hidden.contains(&scene.id))
    }

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
            Some("color") => self.vars.paint_color(object, theme).map(Brush::Solid),
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
            Some("pattern") => {
                let color = self.vars.paint_color(object, theme)?;
                Some(Brush::Pattern(pattern::Pattern::read(object, color, &number)))
            }
            Some("mesh_gradient") => {
                let first = object.get("colors")?.as_array()?.first()?.as_str()?;
                self.vars.color(first, theme).map(|c| Brush::Solid(c.with_opacity(opacity)))
            }
            _ => None,
        }
    }

    /// Adds the items for `scene` and its children.
    fn node(&self, scene: &SceneNode, opacity: f32, clip: Option<Rect>, out: &mut Vec<Item>) {
        let place = self.spans.begin(out.len());
        let plain = self.node_items(scene, opacity, clip, out);
        self.spans.end(place, out.len(), plain);
    }

    /// The opacity `scene` and what is in it are drawn at, in a layer drawn at `opacity`.
    fn opacity_in(&self, scene: &SceneNode, opacity: f32) -> f32 {
        opacity * self.num(scene.node.opacity.as_ref(), &scene.theme).unwrap_or(1.0).clamp(0.0, 1.0)
    }

    /// Whether `scene` cuts its children to its box, and what they are cut to, when `scene` is cut to `clip`.
    fn clip_in(&self, scene: &SceneNode, bounds: Rect, clip: Option<Rect>) -> (bool, Option<Rect>) {
        let clips = scene.node.clip.as_ref().and_then(|c| self.vars.flag(c, &scene.theme)).unwrap_or(false);
        (clips, if clips { Some(clip.map_or(bounds, |c| intersect(&c, &bounds))) } else { clip })
    }

    /// The corners `scene` cuts its children's items round, when it does: it clips, and has round corners.
    fn rounds(&self, scene: &SceneNode, clips: bool) -> Option<[f32; 4]> {
        let (w, h) = (scene.local.w, scene.local.h);
        let boxed = matches!(scene.node.kind, Kind::Frame | Kind::Rectangle | Kind::Browser) && scene.to_doc.is_axis_aligned();
        let radii = if clips && boxed { self.corner_radii(&scene.node, &scene.theme, w, h) } else { [0.0; 4] };
        (radii.iter().any(|radius| *radius > 0.5) && !scene.children.is_empty()).then_some(radii)
    }

    /// Whether `scene`, cut to `clip`, hands its children's items on as they are made, one layer after the
    /// other: it is drawn, and neither it nor a mask in it gathers them to make something else of them (an
    /// effect, glass, a cut to round corners or to a mask's shape).
    fn plain(&self, scene: &SceneNode, clip: Option<Rect>) -> bool {
        let (node, theme) = (&scene.node, &scene.theme);
        let (clips, _) = self.clip_in(scene, layout::doc_bounds(scene), clip);
        !self.hides(scene)
            && node.enabled.as_ref().and_then(|e| self.vars.flag(e, theme)) != Some(false)
            && self.effects_of(scene).is_empty()
            && self.pane_of(scene, false, clip).is_none()
            && self.rounds(scene, clips).is_none()
            && !scene.children.iter().any(|child| is_mask(&child.node))
    }

    /// Makes the items of the layers at `paths` in `root` anew and puts them in the place of their old ones in
    /// `items`, the items of `root` as `part` has them. Gives the spans of `root` as they are now, what it
    /// reads of what is behind it, and where the layers painted and paint; `None`, with `items` to be thrown
    /// away, when the whole node has to be built.
    // Human: that is so when a layer the changed one lies in works on its children's items ([`Builder::plain`])
    // or draws their glass as one pane, when the node reads what is behind it (glass, a background blur: it
    // may be the changed layer it reads), and when the changed layer does so itself: built alone, it has
    // nothing behind it.
    // Agent: a path is a layer's place among the scene's nodes, from the top-level node down; the layers
    // above the changed one are as they were when `part` was built, which is what makes the old spans right.
    // Whether such a layer worked on its children's items depends on those children too (a mask among them,
    // glass it draws for them), so it must hold both for what was built then and for what is there now.
    fn rebuild(&self, root: &SceneNode, part: &Part, items: &mut Vec<Item>, paths: &[Vec<u32>]) -> Option<(Vec<Span>, Option<Rect>, Vec<Rect>)> {
        if part.reads.is_some() || self.baked.stopped() {
            return None;
        }
        let mut spans = part.spans.clone();
        let mut touched = Vec::new();
        for path in paths {
            let (mut scene, mut place, mut opacity, mut clip) = (root, 0_usize, 1.0_f32, None);
            let mut above = Vec::with_capacity(path.len());
            for index in path.get(1..)? {
                if !spans.get(place)?.plain || !self.plain(scene, clip) {
                    return None;
                }
                let child = scene.children.get(usize::try_from(*index).ok()?)?;
                if self.merges(scene, child) {
                    return None;
                }
                opacity = self.opacity_in(scene, opacity);
                (_, clip) = self.clip_in(scene, layout::doc_bounds(scene), clip);
                // The child's span: past the spans of the children before it.
                let end = place + spans.get(place)?.nodes;
                above.push(place);
                place += 1;
                for _ in 0..*index {
                    place += spans.get(place)?.nodes;
                }
                if place >= end {
                    return None;
                }
                scene = child;
            }
            let Span { start, end, nodes, .. } = *spans.get(place)?;
            let old = items.get(start..end)?;
            touched.extend(old.iter().filter_map(bake::extent).reduce(|a, b| a.union(&b)));
            // The layer's own items, made as its parent would have them made.
            self.reads.set(None);
            drop(self.spans.take(0));
            self.merged.set(false);
            let mut new = Vec::new();
            self.node(scene, opacity, clip, &mut new);
            let mut within = self.spans.take(0);
            if self.reads.get().is_some() || self.baked.stopped() {
                return None;
            }
            touched.extend(new.iter().filter_map(bake::extent).reduce(|a, b| a.union(&b)));
            // The spans after it move by as many items as it has more or fewer now; the ones it lies in grow.
            let (was_long, is_long) = (end - start, new.len());
            for span in &mut within {
                (span.start, span.end) = (span.start + start, span.end + start);
            }
            // (A span that is never asked for, see [`Span`], may hold any numbers: none may make this fail.)
            let moved = |at: usize| (at + is_long).saturating_sub(was_long);
            for span in spans.get_mut(place + nodes..)? {
                (span.start, span.end) = (moved(span.start), moved(span.end));
            }
            for parent in above {
                let span = spans.get_mut(parent)?;
                (span.end, span.nodes) = (moved(span.end), (span.nodes + within.len()).saturating_sub(nodes));
            }
            spans.splice(place..place + nodes, within);
            items.splice(start..end, new);
        }
        Some((spans, None, touched))
    }

    /// [`Builder::node`]'s work; gives whether the children's items were handed on as they were made
    /// ([`Builder::plain`]).
    #[expect(clippy::too_many_lines, reason = "one arm per node kind; splitting it would scatter the drawing rules")]
    fn node_items(&self, scene: &SceneNode, opacity: f32, clip: Option<Rect>, out: &mut Vec<Item>) -> bool {
        let node = &scene.node;
        let theme = &scene.theme;
        // Whether the glass group this node is in draws its material; said by `children` just before.
        let merged = self.merged.replace(false);
        // A build that was called off ends its walk here; what it has so far is thrown away.
        if self.hides(scene) || self.baked.stopped() {
            return false;
        }
        if node.enabled.as_ref().and_then(|e| self.vars.flag(e, theme)) == Some(false) {
            return false;
        }
        let opacity = self.opacity_in(scene, opacity);
        let (w, h) = (scene.local.w, scene.local.h);
        let bounds = layout::doc_bounds(scene);
        let fills = node.fill.as_ref().map(|f| self.brushes(f.as_slice(), theme)).unwrap_or_default();
        let strokes = node.stroke.as_ref().map(|f| self.brushes(f.as_slice(), theme)).unwrap_or_default();
        let fill_item = |shape: &Path, brush: &Brush, fill_rule: FillRule| {
            self.fill_mesh(shape, brush, fill_rule, (w, h), &scene.to_doc, opacity).map(|mesh| Item::Mesh {
                mesh: mesh.into(),
                bounds,
                clip,
                memo: Memo::default(),
            })
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
                    text: Box::new(TextItem {
                        key: letters_key(&style),
                        face: FaceChoice::of(&style),
                        style,
                        color: color32(text_color.with_opacity(opacity)),
                        to_doc,
                        size,
                        circle: None,
                        track: None,
                        outlines: Vec::new(),
                    }),
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
                    return false;
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
            // Human: a connector's line was worked out by the layout (`pen_doc::connector`); it is only ever
            // stroked, since the space an open line encloses is not a shape anybody meant.
            Kind::Connector => {
                if let Some(shape) = node.geometry.as_deref().and_then(|g| svg::parse(g).ok()).map(|segments| fit_path(&segments, node, w, h)) {
                    for brush in &strokes {
                        self.stroke(&shape, brush, scene, opacity, bounds, clip, out);
                    }
                }
                // The filled shapes at its ends (a triangle, a dot) take the line's colour.
                let markers = node.extra.get(pen_doc::connector::MARKERS).and_then(Value::as_str);
                if let Some(shape) = markers.and_then(|g| svg::parse(g).ok()).map(|segments| fit_path(&segments, node, w, h)) {
                    for brush in &strokes {
                        out.extend(fill_item(&shape, brush, FillRule::NonZero));
                    }
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
                let circle = OnCircle::of(node);
                let track = if circle.is_none() { Track::of(node).map(std::sync::Arc::new) } else { None };
                if circle.is_some() || track.is_some() {
                    // Text on a circle or along a line is one line, however long.
                    style.wrap = None;
                }
                let color = fills.iter().find_map(|b| if let Brush::Solid(c) = b { Some(*c) } else { None }).unwrap_or(Rgba::BLACK);
                let outlines: Vec<(Color32, f32)> =
                    self.text_outlines(scene, &strokes).into_iter().map(|(ink, reach)| (color32(ink.with_opacity(opacity)), reach)).collect();
                let reach = outlines.first().map_or(0.0, |(_, reach)| *reach);
                let bounds = Rect::new(bounds.x - reach, bounds.y - reach, bounds.w + 2.0 * reach, bounds.h + 2.0 * reach);
                out.push(Item::Text {
                    text: Box::new(TextItem {
                        key: letters_key(&style),
                        face: FaceChoice::of(&style),
                        style,
                        color: color32(color.with_opacity(opacity)),
                        to_doc: scene.to_doc,
                        size: (w, h),
                        circle,
                        track,
                        outlines,
                    }),
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
        let straight = self.children(scene, opacity, bounds, clip, out);
        let pane = self.pane_of(scene, merged, clip);
        if !effects.is_empty() || pane.is_some() {
            let opaque = opacity >= 0.999 && fills.iter().any(|b| matches!(b, Brush::Solid(c) if c.a == 255));
            let look = bake::Look { opacity, opaque, clip, radius: self.box_radius(scene) };
            self.apply_effects(scene, &effects, pane.as_ref(), [start, fill_end.clamp(start, own_end), own_end], &look, out);
            return false;
        }
        straight
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

    /// The outlines round the letters of the text `scene`, the outermost first: each one's colour and how far
    /// it reaches out of the letters.
    // Human: a text's `stroke` is an outline round its letters, `strokeWidth` wide, drawn outside them so the
    // letters keep their weight: the name on a team's crest, a sticker's edge. Several strokes are outlines
    // one round the other, each `strokeWidth` wide; like fills, the first is at the back, so it is the
    // outermost (a white keyline round a dark outline is `["#ffffff", "$ink"]`).
    pub(super) fn text_outlines(&self, scene: &SceneNode, strokes: &[Brush]) -> Vec<(Rgba, f32)> {
        let width = self.stroke_width(scene);
        let solid: Vec<Rgba> = strokes.iter().filter_map(|b| if let Brush::Solid(c) = b { Some(*c) } else { None }).collect();
        if width <= 0.0 {
            return Vec::new();
        }
        #[expect(clippy::cast_precision_loss, reason = "a handful of strokes")]
        solid.iter().enumerate().map(|(at, ink)| (*ink, width * (solid.len() - at) as f32)).collect()
    }

    /// The outline of a layer that has a shape of its own (a box, an ellipse, a polygon, a path), and how its
    /// inside is told from its outside.
    fn shape_of(&self, scene: &SceneNode) -> Option<(Path, FillRule)> {
        let (node, theme) = (&scene.node, &scene.theme);
        let (w, h) = (scene.local.w, scene.local.h);
        match node.kind {
            Kind::Frame | Kind::Rectangle => Some((rounded_rect(0.0, 0.0, w, h, self.corner_radii(node, theme, w, h)), FillRule::NonZero)),
            Kind::Ellipse => Some((self.ellipse(node, theme, w, h), FillRule::EvenOdd)),
            Kind::Polygon => {
                let count = self.num(node.polygon_count.as_ref(), theme).unwrap_or(3.0).clamp(3.0, 64.0);
                let radius = match &node.corner_radius {
                    Some(Corners::All(r)) => self.num(Some(r), theme),
                    Some(Corners::Each(each)) => self.num(each.first(), theme),
                    None => None,
                };
                Some((polygon(w, h, count, radius.unwrap_or(0.0).max(0.0)), FillRule::NonZero))
            }
            Kind::Path => {
                let shape = node.geometry.as_deref().and_then(|g| svg::parse(g).ok()).map(|segments| fit_path(&segments, node, w, h))?;
                Some((shape, if node.fill_rule.as_deref() == Some("evenodd") { FillRule::EvenOdd } else { FillRule::NonZero }))
            }
            _ => None,
        }
    }

    /// The outline of a mask layer as an item that covers its shape, and the box of that shape.
    fn mask_layer(&self, scene: &SceneNode) -> Option<(Item, Rect)> {
        if !is_mask(&scene.node) {
            return None;
        }
        let (shape, rule) = self.shape_of(scene)?;
        let bounds = layout::doc_bounds(scene);
        let mesh = self.fill_mesh(&shape, &Brush::Solid(Rgba::WHITE), rule, (scene.local.w, scene.local.h), &scene.to_doc, 1.0)?;
        Some((Item::Mesh { mesh: mesh.into(), bounds, clip: None, memo: Memo::default() }, bounds))
    }

    /// Adds the items of `scene`'s children, clipped to its box when it clips. Gives whether they were added
    /// as they were made: none was gathered to be cut round the corners or to a mask's shape.
    fn children(&self, scene: &SceneNode, opacity: f32, bounds: Rect, clip: Option<Rect>, out: &mut Vec<Item>) -> bool {
        let (clips, child_clip) = self.clip_in(scene, bounds, clip);
        // Human: a frame that clips cuts its children to its own outline, rounded corners included. Only an
        // unturned box has corners that can be cut this way; a turned one cuts to the box around it.
        let (w, h) = (scene.local.w, scene.local.h);
        let radii = self.rounds(scene, clips);
        let (rounded, radii) = (radii.is_some(), radii.unwrap_or([0.0; 4]));
        let mut inside = Vec::new();
        // Human: a layer with `"mask": true` is drawn as it is, and cuts the layers that follow it in the same
        // group or frame to its shape: water inside a shield, shading inside a letter. Those layers are
        // gathered apart and become one picture, cut to the mask's outline.
        let mut mask: Option<(Item, Rect, Vec<Item>)> = None;
        let mut straight = !rounded;
        let mut joined = 0;
        for child in &scene.children {
            // Human: a glass group draws the glass of its children as one pane, so they don't draw their own.
            let merges = joined < glass::MOST_SHAPES && self.merges(scene, child);
            joined += usize::from(merges);
            self.merged.set(merges);
            let drawn: &mut Vec<Item> = if rounded { &mut inside } else { &mut *out };
            // Human: a second mask layer ends what the first one cuts: it is drawn whole, and cuts what follows.
            let next = self.mask_layer(child);
            straight &= next.is_none();
            if next.is_some()
                && let Some((outline, area, masked)) = mask.take()
            {
                self.mask_off(&outline, area, child_clip, &masked, drawn);
            }
            if let Some((_, _, masked)) = &mut mask {
                self.node(child, opacity, child_clip, masked);
            } else {
                self.node(child, opacity, child_clip, drawn);
                mask = next.map(|(outline, area)| (outline, area, Vec::new()));
            }
        }
        if let Some((outline, area, masked)) = mask {
            self.mask_off(&outline, area, child_clip, &masked, if rounded { &mut inside } else { &mut *out });
        }
        if let (true, Some(cut)) = (rounded, child_clip) {
            let outline = self
                .fill_mesh(&rounded_rect(0.0, 0.0, w, h, radii), &Brush::Solid(Rgba::WHITE), FillRule::NonZero, (w, h), &scene.to_doc, 1.0)
                .map(|mesh| Item::Mesh { mesh: mesh.into(), bounds, clip: None, memo: Memo::default() });
            match outline {
                Some(outline) => self.round_off(&outline, radii, bounds, cut, inside, out),
                None => out.extend(inside),
            }
        }
        straight
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
        if let Brush::Pattern(pattern) = brush {
            return Some(pattern.mesh(&buffers, size, self.tolerance_for(to_doc), to_doc, opacity));
        }
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
        self.stroke_pen(shape, brush, pen_for(&scene.node, width), scene, opacity, bounds, clip, out);
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
            out.push(Item::Mesh {
                mesh: colored_mesh(&buffers, brush, (scene.local.w, scene.local.h), &scene.to_doc, opacity).into(),
                bounds,
                clip,
                memo: Memo::default(),
            });
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
                        out.push(Item::Mesh { mesh: mesh.into(), bounds, clip, memo: Memo::default() });
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

/// The face a text is drawn in, worked out once and kept with the text.
// Human: every text on the canvas asked for its face every frame: a lock shared by all of them, the family's
// name in small letters, a look through the installed families and a name put together for egui. The answer
// only changes when a face becomes ready, so it is kept, with the number the faces had when it was given.
#[derive(Clone)]
pub struct FaceChoice {
    family: FontFamily,
    coords: VariationCoords,
    /// The letters are slanted, because the family has no italic of its own.
    italics: bool,
    /// [`crate::fonts::changes`] when this was worked out.
    as_of: u64,
}

impl FaceChoice {
    /// The face for text in `style`'s family, weight and slant, as the faces are now.
    // Agent: TAKES the fonts' lock; asks for the face when it isn't ready yet, as `fonts::choose` does.
    #[must_use]
    pub fn of(style: &TextStyle) -> FaceChoice {
        // Read before the choice is made: a face that becomes ready meanwhile makes this choice an old one.
        let as_of = crate::fonts::changes();
        // Human: the face the text names, when it is installed and ready; until then, and for a family nothing
        // installed can stand in for, the interface face in its regular, bold or italic.
        let (family, coords, italics) = match crate::fonts::choose(&style.family, style.weight, style.italic) {
            Some(chosen) => (chosen.family, chosen.coords, chosen.slant),
            None if style.italic => (FontFamily::Name(ITALIC_FAMILY.into()), VariationCoords::default(), false),
            None if style.weight >= 600 => (FontFamily::Name(BOLD_FAMILY.into()), VariationCoords::default(), false),
            None if style.family.to_ascii_lowercase().contains("mono") => (FontFamily::Monospace, VariationCoords::default(), false),
            None => (FontFamily::Proportional, VariationCoords::default(), false),
        };
        FaceChoice { family, coords, italics, as_of }
    }

    /// This choice while it still holds, or the choice as the faces are now.
    fn now(&self, style: &TextStyle) -> std::borrow::Cow<'_, FaceChoice> {
        if self.as_of == crate::fonts::changes() { std::borrow::Cow::Borrowed(self) } else { std::borrow::Cow::Owned(FaceChoice::of(style)) }
    }
}

/// The egui text job for a style at `scale` screen points per canvas unit.
#[must_use]
pub fn text_job(style: &TextStyle, color: Color32, scale: f32) -> LayoutJob {
    text_job_in(style, &FaceChoice::of(style), color, scale)
}

/// [`text_job`] with the face already chosen (`face` must be the choice for `style`'s family, weight and slant).
// Agent: HOT-PATH (every visible text, every frame); takes no lock.
fn text_job_in(style: &TextStyle, face: &FaceChoice, color: Color32, scale: f32) -> LayoutJob {
    let size = style.size * scale;
    let format = TextFormat {
        font_id: FontId::new(size, face.family.clone()),
        color,
        extra_letter_spacing: style.letter_spacing * scale,
        line_height: style.line_px.map(|l| l * scale),
        coords: face.coords.clone(),
        italics: face.italics,
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

/// Draws `text` through `camera`; `steady` is whether the zoom is at rest (see [`glyph_scale`]), and `warmth`
/// knows which sizes its letters have been drawn at.
fn draw_text(painter: &Painter, camera: &Camera, view: egui::Rect, text: &TextItem, steady: bool, warmth: &Warmth) {
    let z = camera.zoom;
    if text.style.content.is_empty() || text.style.size * z < 1.0 {
        return;
    }
    let (wanted, _) = glyph_scale(text.style.size, z, painter.ctx().pixels_per_point(), steady);
    let (laid, there) = warmth.size(text.key, wanted);
    if !there {
        // The text gets its size in one of the next frames.
        painter.ctx().request_repaint();
    }
    let grow = z / laid;
    let fine = z * painter.ctx().pixels_per_point();
    for (corner, paragraph, angle) in text_parts(text, laid, fine, &mut |job| painter.layout_job(job)) {
        paint_galley(painter, screen(camera, view, corner), paragraph, text.color, angle, grow);
    }
}

/// How far apart, in screen pixels, the copies of a text lie along the ring that makes its outline.
const OUTLINE_STEP: f32 = 1.0;
/// How many copies one ring of an outline has at most.
const MOST_COPIES: f32 = 256.0;

/// Where the copies of a text lie that together make an outline reaching `width` out of its letters, as
/// offsets in document units, drawn at `z` screen pixels per unit.
// Human: letters come from the font picture, which has no outlines to widen. Copies of the text in the
// outline's colour, set round a ring of the outline's width (and round a second ring half way in when it is
// wide, for dots and serifs smaller than the outline), fill the same area a widened letter would, with round
// corners. A letter's square corner shows every gap between two copies as a step, so the copies lie a pixel
// apart; the graphics card draws a few hundred small letters without effort. (A picture is painted another
// way, from distances: see `Bitmap::outline`.)
fn outline_ring(width: f32, z: f32) -> Vec<(f32, f32)> {
    let mut places = Vec::new();
    let px = width * z;
    let rings: &[f32] = if px > 4.0 { &[1.0, 0.5] } else { &[1.0] };
    for part in rings {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to 8..=256 before the cast")]
        let count = (std::f32::consts::TAU * px * part / OUTLINE_STEP).ceil().clamp(8.0, MOST_COPIES) as usize;
        for at in 0..count {
            #[expect(clippy::cast_precision_loss, reason = "at most 256 steps")]
            let angle = std::f32::consts::TAU * at as f32 / count as f32;
            places.push((width * part * angle.cos(), width * part * angle.sin()));
        }
    }
    places
}

/// `text` in `color`, as paragraphs with the document point of their top left corner and the angle they are
/// turned by: the whole text, or its letters one by one when it is set on a circle. `laid` is the scale it
/// is laid out at; `lay` lays a paragraph out.
fn text_placed(text: &TextItem, color: Color32, laid: f32, lay: &mut dyn FnMut(LayoutJob) -> std::sync::Arc<egui::Galley>) -> Vec<Letter> {
    let face = text.face.now(&text.style);
    let galley = lay(text_job_in(&text.style, &face, color, laid));
    // Human: where the letter whose middle is `at` along a text `long` stands, and the angle it is turned by.
    let (w, h) = text.size;
    if let Some(circle) = text.circle {
        let radius = w.min(h) / 2.0;
        if radius <= 0.0 {
            return Vec::new();
        }
        return letters_along(text, &galley, color, laid, lay, &|at, long| {
            let (place, angle) = circle.at((at - long / 2.0) / radius);
            (Point::new(w / 2.0 + radius * place.cos(), h / 2.0 + radius * place.sin()), angle)
        });
    }
    if let Some(track) = &text.track {
        let room = track.length();
        return letters_along(text, &galley, color, laid, lay, &|at, long| {
            let start = match text.style.align {
                TextAlign::Left | TextAlign::Justify => 0.0,
                TextAlign::Center => (room - long) / 2.0,
                TextAlign::Right => room - long,
            };
            track.at(start + at)
        });
    }
    let (anchor, angle) = text_anchor(text, galley.size().y / laid);
    vec![(anchor, galley, angle)]
}

/// Everything of `text` that the canvas draws, back to front: the copies that make its outline, then the
/// text itself. `z` is the scale it is drawn at, in screen pixels per unit.
fn text_parts(text: &TextItem, laid: f32, z: f32, lay: &mut dyn FnMut(LayoutJob) -> std::sync::Arc<egui::Galley>) -> Vec<Letter> {
    let mut parts = Vec::new();
    for (ink, width) in text.outlines.iter().copied() {
        let ring = outline_ring(width, z);
        for (corner, paragraph, angle) in text_placed(text, ink, laid, lay) {
            parts.extend(ring.iter().map(|(dx, dy)| (Point::new(corner.x + dx, corner.y + dy), paragraph.clone(), angle)));
        }
    }
    parts.extend(text_placed(text, text.color, laid, lay));
    parts
}

/// A letter of a text set on a circle: the document point of the top left corner of its own small
/// paragraph, that paragraph, and the angle it is drawn at about that corner.
type Letter = (Point, std::sync::Arc<egui::Galley>, f32);

/// The letters of `galley` (the text of `text` laid out as one line at `laid` pixels per unit), each placed to
/// stand on the circle that fits the layer's box. `lay` lays one letter out.
// Human: the line as a whole says where each letter's middle lies along the text, so the spacing between
// letters is the face's own; each letter is then drawn by itself, turned to stand on the circle there.
fn letters_along(
    text: &TextItem,
    galley: &egui::Galley,
    color: Color32,
    laid: f32,
    lay: &mut dyn FnMut(LayoutJob) -> std::sync::Arc<egui::Galley>,
    place: &dyn Fn(f32, f32) -> (Point, f32),
) -> Vec<Letter> {
    let Some(row) = galley.rows.first() else { return Vec::new() };
    let (Some(first), Some(last)) = (row.row.glyphs.first(), row.row.glyphs.last()) else { return Vec::new() };
    // The space added after each letter is not part of the last letter.
    let spacing = text.style.letter_spacing * laid;
    let length = last.pos.x + last.advance_width - spacing - first.pos.x;
    let turn = text.to_doc.m[1].atan2(text.to_doc.m[0]);
    // One letter is set in the face of the whole text.
    let face = text.face.now(&text.style);
    let mut single = text.style.clone();
    single.letter_spacing = 0.0;
    single.wrap = None;
    let mut letters = Vec::new();
    for glyph in &row.row.glyphs {
        if glyph.chr.is_whitespace() {
            continue;
        }
        let middle = glyph.pos.x - first.pos.x + (glyph.advance_width - spacing) / 2.0;
        let (stands, angle) = place(middle / laid, length / laid);
        let on_circle = text.to_doc.apply(stands);
        single.content = glyph.chr.to_string();
        let letter = lay(text_job_in(&single, &face, color, laid));
        let Some(drawn) = letter.rows.first().and_then(|row| row.row.glyphs.first().map(|glyph| (row.pos, glyph.pos, glyph.advance_width))) else {
            continue;
        };
        // The point of the letter that goes on the circle: the middle of its width, on its baseline.
        let (foot_x, foot_y) = ((drawn.0.x + drawn.1.x + drawn.2 / 2.0) / laid, (drawn.0.y + drawn.1.y) / laid);
        let angle = angle + turn;
        let (sin, cos) = angle.sin_cos();
        let corner = Point::new(on_circle.x - (foot_x * cos - foot_y * sin), on_circle.y - (foot_x * sin + foot_y * cos));
        letters.push((corner, letter, angle));
    }
    letters
}

/// Paints `galley` with its top left corner at `pos`, turned by `angle` about that corner and scaled `grow`
/// times (1 when it was laid out at the size it is drawn at).
fn paint_galley(painter: &Painter, pos: Pos2, galley: std::sync::Arc<egui::Galley>, color: Color32, angle: f32, grow: f32) {
    if (grow - 1.0).abs() < 1e-4 {
        painter.add(egui::epaint::TextShape::new(pos, galley, color).with_angle(angle));
        return;
    }
    // Human: a headline zoomed past what the font atlas holds is laid out at the largest size that fits, and
    // text on a moving zoom at the nearest step; its triangles are scaled around where the text starts. It
    // is a little soft, and it is there.
    let shape = egui::epaint::TextShape::new(Pos2::ZERO, galley, color).with_angle(angle);
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
    // Human: the angle of the node's local x axis on screen (rotation; flips are not mirrored for text).
    (text.to_doc.apply(text_corner(&text.style, text.size, text_h)), text.to_doc.m[1].atan2(text.to_doc.m[0]))
}

/// Where the paragraph of a text in `style` starts in its own box of `size`, the paragraph being `text_h` high.
fn text_corner(style: &TextStyle, size: (f32, f32), text_h: f32) -> Point {
    let (w, h) = size;
    // Human: the paragraph starts half a letter-spacing to the left (`x = -letterSpacing / 2`),
    // and vertical alignment applies only to fixed-size text (not `auto`).
    let anchor_x = -style.letter_spacing / 2.0
        + match style.align {
            TextAlign::Left | TextAlign::Justify => 0.0,
            TextAlign::Center => w / 2.0,
            TextAlign::Right => w,
        };
    let top = match (style.wrap.is_some(), style.valign) {
        (false, _) | (true, TextVAlign::Top) => 0.0,
        (true, TextVAlign::Middle) => (h - text_h) / 2.0,
        (true, TextVAlign::Bottom) => h - text_h,
    };
    Point::new(anchor_x, top)
}

/// For the timings (`crate::timing`): nanoseconds to make one text's job, with its face chosen each time or kept.
#[cfg(test)]
pub fn timing_text_job(style: &TextStyle, kept: bool) -> f64 {
    const TIMES: u32 = 200_000;
    let face = FaceChoice::of(style);
    let start = std::time::Instant::now();
    for _ in 0..TIMES {
        let style = std::hint::black_box(style);
        std::hint::black_box(if kept { text_job_in(style, &face.now(style), Color32::BLACK, 0.25) } else { text_job(style, Color32::BLACK, 0.25) });
    }
    start.elapsed().as_secs_f64() * 1e9 / f64::from(TIMES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moved_meshes_are_kept_while_the_look_and_the_item_stay() {
        use std::sync::Arc;
        let corner = |x: f32| egui::epaint::Vertex { pos: Pos2::new(x, 0.0), uv: Pos2::ZERO, color: Color32::WHITE };
        let mesh = Arc::new(Mesh { indices: vec![0, 1, 2], vertices: vec![corner(0.0), corner(1.0), corner(2.0)], ..Default::default() });
        let view = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 100.0));
        let camera = Camera { center: Point::new(0.0, 0.0), zoom: 2.0 };
        let shift = |by: f32| move |p: Pos2| Pos2::new(p.x + by, p.y);
        let mut moved = Moved::default();
        // The first frame of a look keeps nothing: the look may be gone with the next frame.
        moved.begin(&camera, view, 2);
        let first = moved.of(0, &mesh, shift(10.0));
        assert_eq!(moved.slots.len(), 0);
        // The second keeps what it moves, and the third is handed the very same mesh.
        moved.begin(&camera, view, 2);
        let second = moved.of(0, &mesh, shift(10.0));
        moved.begin(&camera, view, 2);
        let third = moved.of(0, &mesh, shift(10.0));
        assert!(Arc::ptr_eq(&second, &third) && !Arc::ptr_eq(&first, &second));
        assert!((third.vertices[1].pos.x - 11.0).abs() < 1e-6 && (mesh.vertices[1].pos.x - 1.0).abs() < 1e-6);
        // An item with another mesh (built again) is moved anew, though its place in the list is the same.
        let rebuilt = Arc::new(Mesh::clone(&mesh));
        moved.begin(&camera, view, 2);
        assert!(!Arc::ptr_eq(&moved.of(0, &rebuilt, shift(10.0)), &third));
        // An item that isn't drawn gives its mesh up.
        moved.begin(&camera, view, 2);
        moved.skip(0);
        assert!(moved.slots.iter().all(Option::is_none));
        // Another look (a pan, a zoom, the canvas somewhere else in the window) empties everything.
        moved.begin(&camera, view, 2);
        moved.of(1, &mesh, shift(10.0));
        let panned = Camera { center: Point::new(5.0, 0.0), zoom: 2.0 };
        moved.begin(&panned, view, 2);
        assert_eq!(moved.slots.len(), 0);
        assert!((moved.of(1, &mesh, shift(20.0)).vertices[0].pos.x - 20.0).abs() < 1e-6);
        // A shorter list lets go of the slots past its end.
        moved.begin(&panned, view, 2);
        moved.of(1, &mesh, shift(20.0));
        moved.begin(&panned, view, 1);
        assert_eq!(moved.slots.len(), 1);
    }

    #[test]
    fn text_is_laid_out_at_few_sizes_and_no_larger_than_the_atlas_takes() {
        // At rest text up to 64 pixels high is laid out at the size it is drawn at.
        assert_eq!(glyph_scale(16.0, 1.3, 2.0, true), (1.3, 1.0));
        // Larger text at rest is laid out at the next of four sizes to a doubling, and scaled down a little.
        let mut sizes = std::collections::BTreeSet::new();
        for step in 0..=100_u8 {
            let zoom = 4.0 + f32::from(step) * 0.04;
            let (laid, shrink) = glyph_scale(16.0, zoom, 2.0, true);
            assert!((laid * shrink - zoom).abs() < 1e-4 && shrink <= 1.0 + 1e-6 && shrink > 0.84, "{zoom}: {laid} × {shrink}");
            sizes.insert((laid * 1000.0).round().to_bits());
        }
        // A hundred zooms between 400 % and 800 %: five sizes.
        assert_eq!(sizes.len(), 5);
        // On the move every zoom between two steps shares one size, and the triangles make up the difference.
        for zoom in [1.5_f32, 1.7, 2.0, 2.4, 2.8] {
            let (laid, grow) = glyph_scale(16.0, zoom, 2.0, false);
            assert!((laid - 2.0).abs() < 1e-6 && (laid * grow - zoom).abs() < 1e-5, "{zoom}: {laid} × {grow}");
        }
        assert!((glyph_scale(16.0, 0.3, 2.0, false).0 - 0.25).abs() < 1e-6);
        // A letter is never laid out higher than MAX_GLYPH_PX pixels of the screen, at rest or on the move.
        for steady in [true, false] {
            let (laid, grow) = glyph_scale(100.0, 64.0, 2.0, steady);
            assert!((laid * 100.0 * 2.0 - MAX_GLYPH_PX).abs() < 1e-2 && (laid * grow - 64.0).abs() < 1e-3);
        }
    }

    #[test]
    fn letters_new_to_a_size_are_brought_in_a_few_a_frame_and_drawn_at_their_last_size_meanwhile() {
        let warmth = Warmth::default();
        let long_ago = std::time::Instant::now().checked_sub(std::time::Duration::from_secs(1)).unwrap();
        // A frame with time left: every text gets the size it wants.
        warmth.frame.set(Some((std::time::Instant::now(), false)));
        assert_eq!((warmth.size(1, 1.0), warmth.size(2, 1.0)), ((1.0, true), (1.0, true)));
        // A frame whose time is up: the first text new to a size still gets it, the next keeps its last one,
        // and a text never drawn before is drawn, having no other size to be drawn at.
        warmth.frame.set(Some((long_ago, false)));
        assert_eq!((warmth.size(1, 2.0), warmth.size(2, 2.0), warmth.size(3, 2.0)), ((2.0, true), (1.0, false), (2.0, true)));
        // A size known already costs nothing and is always given.
        assert_eq!(warmth.size(1, 1.0), (1.0, true));
        // The frame after brings in the text that was left.
        warmth.frame.set(Some((long_ago, false)));
        assert_eq!(warmth.size(2, 2.0), (2.0, true));
    }

    /// A page with three blurred layers side by side, laid out. Each has a size of its own: layers that look
    /// the same share one picture.
    fn blurred() -> crate::editor::Editor {
        let layer = |n: u8| {
            format!(
                r##"{{"type":"ellipse","id":"e{n}","x":{},"y":20,"width":{},"height":40,"fill":"#ff0000","effect":{{"type":"blur","radius":4}}}}"##,
                20 + 60 * n,
                30 + 5 * n
            )
        };
        let layers = [layer(0), layer(1), layer(2)].join(",");
        let json = format!(
            r##"{{"version":"2.22","children":[{{"type":"frame","id":"page","x":0,"y":0,"width":200,"height":80,"layout":"none","fill":"#ffffff","children":[{layers}]}}]}}"##
        );
        let mut editor = crate::editor::Editor::new(pen_doc::read(&json).unwrap(), None, 1);
        editor.refresh(&|_| (10.0, 10.0));
        editor
    }

    /// The pictures (textured meshes) among the list's items.
    fn pictures(list: &DisplayList) -> Vec<egui::TextureId> {
        let textured = |item: &Item| match item {
            Item::Mesh { mesh, .. } if mesh.texture_id != egui::TextureId::default() => Some(mesh.texture_id),
            _ => None,
        };
        list.items.iter().filter_map(textured).collect()
    }

    #[test]
    fn a_pdf_is_vectors_with_pictures_only_for_what_it_has_no_word_for() {
        let open = |layers: &str| {
            let json = format!(
                r##"{{"version":"2.22","children":[{{"type":"frame","id":"page","x":0,"y":0,"width":200,"height":80,"layout":"none","clip":true,"fill":"#ffffff","children":[{layers}]}}]}}"##
            );
            let mut editor = crate::editor::Editor::new(pen_doc::read(&json).unwrap(), None, 1);
            editor.refresh(&|_| (10.0, 10.0));
            editor
        };
        let ctx = egui::Context::default();
        let write = |editor: &crate::editor::Editor| {
            let scenes: Vec<&SceneNode> = editor.scene.iter().collect();
            DisplayList::default().pdf(&scenes, Vars::new(&editor.doc), &ctx, None)
        };
        // The layers that go on the page as pictures of themselves, by id.
        let pictured = |editor: &crate::editor::Editor| {
            let scenes: Vec<&SceneNode> = editor.scene.iter().collect();
            let (list, baked) = (DisplayList::default(), bake::Baked::default());
            baked.begin();
            let works = crate::app::lock(&list.works);
            DisplayList::builder(crate::export::PDF_SCALE, Vars::new(&editor.doc), &ctx, None, &works.images, &baked).pdf_pictured(&scenes)
        };
        // Shapes, a gradient, a stroke and a faded layer: a page of vectors, without a single picture.
        let plain = open(
            r##"{"type":"rectangle","id":"r","x":10,"y":10,"width":60,"height":40,"cornerRadius":8,"stroke":"#000000","strokeWidth":2,
                 "fill":{"type":"gradient","gradientType":"linear","colors":[{"position":0,"color":"#ff0000"},{"position":1,"color":"#0000ff"}]}},
                {"type":"ellipse","id":"e","x":90,"y":10,"width":50,"height":50,"fill":"#00aa00","opacity":0.5}"##,
        );
        assert!(write(&plain).unwrap().unwrap().starts_with(b"%PDF-"));
        assert_eq!(pictured(&plain), Vec::<String>::new());
        // A blurred layer, one with a pattern and one with an angular gradient are pictures of themselves; the
        // plain one beside them stays a shape.
        let mixed = open(
            r##"{"type":"rectangle","id":"r","x":10,"y":10,"width":60,"height":40,"fill":"#ff0000"},
                {"type":"ellipse","id":"blur","x":90,"y":10,"width":50,"height":50,"fill":"#00aa00","effect":{"type":"blur","radius":4}},
                {"type":"rectangle","id":"dots","x":150,"y":10,"width":20,"height":20,"fill":{"type":"pattern","shape":"dots","color":"#000000"}},
                {"type":"rectangle","id":"sweep","x":150,"y":40,"width":20,"height":20,
                 "fill":{"type":"gradient","gradientType":"angular","colors":[{"position":0,"color":"#ff0000"},{"position":1,"color":"#0000ff"}]}}"##,
        );
        assert_eq!(pictured(&mixed), vec!["blur".to_owned(), "dots".to_owned(), "sweep".to_owned()]);
        let (with, without) = (write(&mixed).unwrap().unwrap(), write(&plain).unwrap().unwrap());
        assert!(with.starts_with(b"%PDF-") && with.len() > without.len());
        // A material shows what is behind it: no page of vectors is written, and the page stays one picture.
        let glass = open(r#"{"type":"rectangle","id":"g","x":10,"y":10,"width":60,"height":40,"material":"frosted"}"#);
        assert!(write(&glass).is_none());
    }

    #[test]
    fn another_zoom_step_is_built_beside_the_window_while_the_list_on_screen_stays() {
        let editor = blurred();
        let ctx = egui::Context::default();
        let hidden = std::collections::HashSet::new();
        let visible = Rect::new(0.0, 0.0, 200.0, 80.0);
        let mut list = DisplayList::default();
        let look = |list: &mut DisplayList, zoom: f32| {
            // The zoom is looked at as one that has come to rest.
            list.zoom_seen = zoom;
            list.zoom_moved = None;
            list.update(&editor.scene, editor.generation, zoom, visible, &editor.doc, &ctx, None, Color32::WHITE, &hidden, 0, (&[], &[]));
        };
        // Looks until the build beside the window has sent its whole list; the list on screen is `shown` until then.
        let until_whole = |list: &mut DisplayList, zoom: f32, shown: &[egui::TextureId], step: i32| {
            for _ in 0..5000 {
                look(list, zoom);
                if list.zoom_bucket == step && list.running.is_none() {
                    return;
                }
                assert_eq!(pictures(list), shown);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            panic!("the build beside the window never sent its list");
        };
        // The first build is made here, in one go.
        look(&mut list, 1.0);
        let first = pictures(&list);
        assert_eq!((first.len(), list.running.is_some(), list.zoom_bucket), (3, false, 0));
        assert_eq!(first.iter().collect::<std::collections::HashSet<_>>().len(), 3);
        // Zoomed in two steps: the look starts a build beside the window and leaves the list as it is.
        look(&mut list, 4.0);
        assert_eq!((pictures(&list), list.zoom_bucket, list.running.is_some()), (first.clone(), 0, true));
        until_whole(&mut list, 4.0, &first, 2);
        // The new list is whole and shown, with pictures of its own.
        let second = pictures(&list);
        assert_eq!(second.len(), 3);
        assert!(second.iter().all(|picture| !first.contains(picture)));
        assert_eq!(second.iter().collect::<std::collections::HashSet<_>>().len(), 3);
        // Going back builds beside the window too, and finds the pictures of the step before.
        look(&mut list, 1.0);
        until_whole(&mut list, 1.0, &second, 0);
        assert_eq!(pictures(&list), first);
    }

    #[test]
    fn a_changed_scene_is_built_at_once_and_calls_off_the_build_beside_the_window() {
        let mut editor = blurred();
        let ctx = egui::Context::default();
        let hidden = std::collections::HashSet::new();
        let visible = Rect::new(0.0, 0.0, 200.0, 80.0);
        let mut list = DisplayList::default();
        list.update(&editor.scene, editor.generation, 1.0, visible, &editor.doc, &ctx, None, Color32::WHITE, &hidden, 0, (&[], &[]));
        let first = pictures(&list);
        // A build for another zoom step is started beside the window.
        list.zoom_seen = 4.0;
        list.update(&editor.scene, editor.generation, 4.0, visible, &editor.doc, &ctx, None, Color32::WHITE, &hidden, 0, (&[], &[]));
        let called_off = list.running.as_ref().map(|running| running.stop.clone()).unwrap();
        // Every layer gets another colour: all three pictures are made anew, here and at once, at the zoom
        // the window is at, and the build of the scene as it was is called off.
        for n in 0..3 {
            editor.change(&format!("e{n}"), &|node| node.fill = serde_json::from_str(r##""#0000ff""##).ok());
        }
        editor.touched();
        editor.refresh(&|_| (10.0, 10.0));
        list.update(&editor.scene, editor.generation, 4.0, visible, &editor.doc, &ctx, None, Color32::WHITE, &hidden, 0, (&[], &[]));
        let second = pictures(&list);
        assert_eq!((second.len(), list.running.is_some(), list.zoom_bucket), (3, false, 2));
        assert!(second.iter().all(|picture| !first.contains(picture)));
        assert!(called_off.load(std::sync::atomic::Ordering::Relaxed));
    }

    /// Two boards side by side, each with a blurred dot, and a third board with a glass pane that lies over
    /// the first board alone. Laid out.
    fn boards() -> crate::editor::Editor {
        let board = |id: &str, x: u16, inside: &str| {
            format!(
                r##"{{"type":"frame","id":"{id}","x":{x},"y":0,"width":100,"height":100,"layout":"none","fill":"#ffffff","children":[{inside}]}}"##
            )
        };
        let dot = |id: &str| {
            format!(
                r##"{{"type":"ellipse","id":"{id}","x":20,"y":20,"width":40,"height":30,"fill":"#ff0000","effect":{{"type":"blur","radius":4}}}}"##
            )
        };
        let pane = r#"{"type":"frame","id":"pane","x":20,"y":40,"width":60,"height":40,"cornerRadius":8,"material":{"type":"frosted","thickness":"ultra_thin","appearance":"light"}}"#;
        let json = format!(
            r#"{{"version":"2.22","children":[{},{},{{"type":"frame","id":"over","x":0,"y":0,"width":100,"height":100,"layout":"none","children":[{pane}]}}]}}"#,
            board("left", 0, &dot("a")),
            board("right", 200, &dot("b"))
        );
        let mut editor = crate::editor::Editor::new(pen_doc::read(&json).unwrap(), None, 1);
        editor.refresh(&|_| (10.0, 10.0));
        editor
    }

    /// What `list` draws, item by item, in words: two lists that say the same draw the same.
    fn told(list: &DisplayList) -> Vec<String> {
        let one = |item: &Item| match item {
            Item::Mesh { mesh, bounds, clip, .. } => {
                let corners: Vec<(u32, u32, [u8; 4])> =
                    mesh.vertices.iter().map(|v| (v.pos.x.to_bits(), v.pos.y.to_bits(), v.color.to_array())).collect();
                format!("mesh {bounds:?} {clip:?} {:?} {corners:?} {}", mesh.indices, mesh.texture_id == egui::TextureId::default())
            }
            Item::Text { text, bounds, clip } => {
                format!("text {bounds:?} {clip:?} {:?} {:?} {:?} {}", text.to_doc, text.size, text.color, text.style.content)
            }
            Item::Shadow { rect, radius, offset, blur, color, clip } => format!("shadow {rect:?} {radius} {offset:?} {blur} {color:?} {clip:?}"),
        };
        list.items.iter().map(one).collect()
    }

    /// The meshes of `list` by where they are kept: a mesh that is still the same one was not made anew.
    fn kept_meshes(list: &DisplayList) -> std::collections::HashSet<usize> {
        let held = |item: &Item| match item {
            Item::Mesh { mesh, .. } => Some(std::sync::Arc::as_ptr(mesh) as usize),
            _ => None,
        };
        list.items.iter().filter_map(held).collect()
    }

    #[test]
    fn a_layer_built_again_by_itself_brings_only_its_own_items_anew() {
        let json = r##"{"version":"2.22","children":[
            {"type":"frame","id":"plain","x":0,"y":0,"width":400,"height":300,"layout":"none","fill":"#ffffff","clip":true,"children":[
                {"type":"rectangle","id":"a","x":10,"y":10,"width":40,"height":20,"fill":"#ff0000"},
                {"type":"frame","id":"faint","x":100,"y":20,"width":200,"height":200,"layout":"none","opacity":0.5,"clip":true,"children":[
                    {"type":"rectangle","id":"b","x":5,"y":5,"width":20,"height":20,"fill":"#00ff00","stroke":{"thickness":2,"fill":"#003300"}},
                    {"type":"frame","id":"deep","x":40,"y":40,"width":80,"height":80,"layout":"none","fill":"#eeeeee","children":[
                        {"type":"ellipse","id":"c","x":5,"y":5,"width":20,"height":20,"fill":"#0000ff"},
                        {"type":"text","id":"t","x":5,"y":40,"content":"Word","fontSize":12,"fill":"#111111"}]},
                    {"type":"rectangle","id":"last","x":150,"y":150,"width":20,"height":20,"fill":"#ff00ff"}]},
                {"type":"frame","id":"cut","x":10,"y":150,"width":80,"height":80,"layout":"none","children":[
                    {"type":"ellipse","id":"m","mask":true,"x":0,"y":0,"width":60,"height":60,"fill":"#000000"},
                    {"type":"rectangle","id":"under","x":10,"y":10,"width":70,"height":70,"fill":"#00ffff"}]},
                {"type":"rectangle","id":"z","x":300,"y":250,"width":40,"height":20,"fill":"#ffff00"}]},
            {"type":"frame","id":"round","x":500,"y":0,"width":200,"height":200,"layout":"none","fill":"#ffffff","clip":true,"cornerRadius":20,"children":[
                {"type":"rectangle","id":"r","x":10,"y":10,"width":40,"height":20,"fill":"#ff0000"}]},
            {"type":"frame","id":"soft","x":800,"y":0,"width":200,"height":200,"layout":"none","fill":"#ffffff","effect":{"type":"blur","radius":4},"children":[
                {"type":"rectangle","id":"s","x":10,"y":10,"width":40,"height":20,"fill":"#ff0000"}]}
        ]}"##;
        let mut editor = crate::editor::Editor::new(pen_doc::read(json).unwrap(), None, 1);
        editor.refresh(&|_| (10.0, 10.0));
        let ctx = egui::Context::default();
        let hidden = std::collections::HashSet::new();
        let visible = Rect::new(0.0, 0.0, 1000.0, 300.0);
        let look = |list: &mut DisplayList, editor: &crate::editor::Editor| {
            list.update(
                &editor.scene,
                editor.generation,
                1.0,
                visible,
                &editor.doc,
                &ctx,
                None,
                Color32::WHITE,
                &hidden,
                0,
                (editor.stamps(), editor.rebuilt()),
            );
        };
        let mut list = DisplayList::default();
        look(&mut list, &editor);
        // Changes `id` saying so, looks, and gives how many meshes were made anew. What the list draws
        // must be what a list built from nothing draws.
        let mut change = |editor: &mut crate::editor::Editor, id: &str, change: &dyn Fn(&mut Node)| {
            let before = kept_meshes(&list);
            assert!(editor.change(id, change));
            editor.touched_layer(id);
            editor.refresh(&|_| (10.0, 10.0));
            look(&mut list, editor);
            let mut whole = DisplayList::default();
            look(&mut whole, editor);
            assert_eq!(told(&list), told(&whole), "after changing {id}");
            kept_meshes(&list).difference(&before).count()
        };
        let x = |x: u32| move |node: &mut Node| node.x = Some(x.into());
        // A layer straight in the board, one in a faint clipping frame, one three frames deep: one mesh each
        // (a fill and its outline are one), of the board's nine.
        assert_eq!(change(&mut editor, "a", &x(30)), 1);
        assert_eq!(change(&mut editor, "b", &x(9)), 1);
        assert_eq!(change(&mut editor, "c", &x(9)), 1);
        // A text has no mesh.
        assert_eq!(change(&mut editor, "t", &x(9)), 0);
        // A frame with what is in it: its own mesh and its child's.
        assert_eq!(change(&mut editor, "deep", &x(50)), 2);
        // A layer that has fewer items than before, then more: the ones after it are still the right ones.
        assert_eq!(change(&mut editor, "b", &|node| node.stroke = None), 1);
        assert_eq!(change(&mut editor, "a", &|node| node.fill = None), 0);
        assert_eq!(change(&mut editor, "a", &|node| node.fill = serde_json::from_str("[\"#ff0000\",\"#00ff0080\"]").ok()), 2);
        assert_eq!(change(&mut editor, "last", &x(160)), 1);
        assert_eq!(change(&mut editor, "c", &x(12)), 1);
        // A layer that is switched off, and on again.
        assert_eq!(change(&mut editor, "deep", &|node| node.enabled = Some(pen_doc::value::Flag::Value(false))), 0);
        assert_eq!(change(&mut editor, "deep", &|node| node.enabled = None), 2);
        // A board that cuts its children round its corners, and one with an effect, work on their children's
        // items: they are built whole (the cut and the effect are pictures, so no mesh says so here; that
        // the list is the one built from nothing is what counts).
        change(&mut editor, "r", &x(30));
        change(&mut editor, "s", &x(30));
        // So does a frame with a mask in it, and still when the changed layer is the mask that stops being one:
        // what it cut before is no longer cut.
        change(&mut editor, "under", &x(20));
        change(&mut editor, "m", &|node| drop(node.extra.insert(MASK.to_owned(), false.into())));
        assert_eq!(change(&mut editor, "under", &x(15)), 1);
        change(&mut editor, "m", &|node| drop(node.extra.insert(MASK.to_owned(), true.into())));
        change(&mut editor, "under", &x(12));
        assert_eq!(change(&mut editor, "z", &x(310)), 1);
    }

    /// The textures of the pictures in each part of the list.
    fn pictures_by_part(list: &DisplayList) -> Vec<Vec<egui::TextureId>> {
        let mut start = 0;
        let mut found = Vec::new();
        for part in &list.parts {
            let textured = |item: &Item| match item {
                Item::Mesh { mesh, .. } if mesh.texture_id != egui::TextureId::default() => Some(mesh.texture_id),
                _ => None,
            };
            found.push(list.items[start..start + part.len].iter().filter_map(textured).collect());
            start += part.len;
        }
        found
    }

    #[test]
    fn an_edit_builds_only_the_top_level_nodes_it_changed_and_the_glass_over_them() {
        let mut editor = boards();
        let ctx = egui::Context::default();
        let hidden = std::collections::HashSet::new();
        let visible = Rect::new(0.0, 0.0, 300.0, 100.0);
        let mut list = DisplayList::default();
        let look = |list: &mut DisplayList, editor: &crate::editor::Editor| {
            list.update(
                &editor.scene,
                editor.generation,
                1.0,
                visible,
                &editor.doc,
                &ctx,
                None,
                Color32::WHITE,
                &hidden,
                0,
                (editor.stamps(), editor.rebuilt()),
            );
        };
        let recolour = |editor: &mut crate::editor::Editor, id: &str, fill: &str| {
            editor.change(id, &|node| node.fill = serde_json::from_str(&format!("\"{fill}\"")).ok());
            editor.touched();
            editor.refresh(&|_| (10.0, 10.0));
        };
        look(&mut list, &editor);
        let first = pictures_by_part(&list);
        assert_eq!(first.iter().map(Vec::len).collect::<Vec<_>>(), [1, 1, 1]);
        let stamps = editor.stamps().to_vec();
        // The dot on the right board gets another colour: only that board is built again, in the scene and
        // in the list. The left board and the pane over it keep their items and their pictures.
        recolour(&mut editor, "b", "#0000ff");
        assert_eq!((editor.stamps()[0], editor.stamps()[2]), (stamps[0], stamps[2]));
        assert_ne!(editor.stamps()[1], stamps[1]);
        look(&mut list, &editor);
        let second = pictures_by_part(&list);
        assert_eq!((&second[0], &second[2]), (&first[0], &first[2]));
        assert_ne!(second[1], first[1]);
        // The dot on the left board changes: its board is built again, and so is the pane, which shows a
        // blur of what is under it; the right board is left alone.
        recolour(&mut editor, "a", "#00ff00");
        look(&mut list, &editor);
        let third = pictures_by_part(&list);
        assert_eq!(third[1], second[1]);
        assert_ne!(third[0], second[0]);
        assert_ne!(third[2], second[2]);
        // What the list has now is what a list built from nothing has: as many items in each part.
        let mut whole = DisplayList::default();
        look(&mut whole, &editor);
        let lens = |list: &DisplayList| list.parts.iter().map(|part| part.len).collect::<Vec<_>>();
        assert_eq!((lens(&list), list.items.len()), (lens(&whole), whole.items.len()));
        // The pictures of the parts taken over are kept alive: every picture in the list is still held.
        let works = crate::app::lock(&list.works);
        assert!(third.iter().flatten().all(|picture| works.baked.holds(*picture)));
    }

    #[test]
    fn what_is_smaller_than_a_pixel_both_ways_is_not_drawn() {
        // A 16-unit icon on a sharp screen: drawn at 5 %, a speck that is left out at 3 %.
        assert!(!too_small(16.0, 0.05 * 2.0) && too_small(16.0, 0.03 * 2.0));
        // A hairline is as long as its longer side: it stays however thin it is.
        assert!(!too_small(400.0_f32.max(0.5), 0.03 * 2.0));
        // At 100 % nothing a design holds is left out.
        assert!(!too_small(1.0, 1.0));
    }

    #[test]
    fn text_on_a_circle_reads_from_left_to_right_at_the_top_and_at_the_bottom() {
        let quarter = std::f32::consts::FRAC_PI_2;
        let node = |json: &str| serde_json::from_str::<Node>(json).unwrap();
        assert_eq!(OnCircle::of(&node(r#"{"type":"text","id":"a","content":"A"}"#)), None);
        let top = OnCircle::of(&node(r#"{"type":"text","id":"a","content":"A","onCircle":"top"}"#)).unwrap();
        // The middle of the text is at the highest point, upright; what follows lies to its right, turned on.
        assert_eq!(top.at(0.0), (-quarter, 0.0));
        let (place, angle) = top.at(0.5);
        assert!(place.cos() > 0.0 && angle > 0.0);
        let bottom = OnCircle::of(&node(r#"{"type":"text","id":"a","content":"A","onCircle":"bottom","circleAngle":90}"#)).unwrap();
        assert!(bottom.bottom && (bottom.turned - quarter).abs() < 1e-6);
        let plain = OnCircle { bottom: true, turned: 0.0 };
        assert_eq!(plain.at(0.0), (quarter, 0.0));
        // At the bottom what follows also lies to the right, turned the other way.
        let (place, angle) = plain.at(0.5);
        assert!(place.cos() > 0.0 && angle < 0.0);
    }

    #[test]
    fn a_text_follows_its_line_from_the_start_and_goes_on_straight_past_its_ends() {
        let node = |json: &str| serde_json::from_str::<Node>(json).unwrap();
        assert_eq!(Track::of(&node(r#"{"type":"text","id":"a","content":"A"}"#)), None);
        assert_eq!(Track::of(&node(r#"{"type":"text","id":"a","content":"A","onPath":"M0 0"}"#)), None);
        // Right for 100, then down for 50; a second outline is no part of the line.
        let track = Track::of(&node(r#"{"type":"text","id":"a","content":"A","onPath":"M0 10 H100 V60 M500 500 H600"}"#)).unwrap();
        assert!((track.length() - 150.0).abs() < 1e-3);
        let near =
            |(p, angle): (Point, f32), x: f32, y: f32, turned: f32| (p.x - x).abs() < 1e-3 && (p.y - y).abs() < 1e-3 && (angle - turned).abs() < 1e-3;
        let quarter = std::f32::consts::FRAC_PI_2;
        assert!(near(track.at(40.0), 40.0, 10.0, 0.0));
        assert!(near(track.at(120.0), 100.0, 30.0, quarter));
        assert!(near(track.at(-20.0), -20.0, 10.0, 0.0) && near(track.at(170.0), 100.0, 80.0, quarter));
        // A curve is followed closely: a half circle of radius 50 is about 157 long.
        let arch = Track::of(&node(r#"{"type":"text","id":"a","content":"A","onPath":"M0 50 A50 50 0 0 1 100 50"}"#)).unwrap();
        assert!((arch.length() - 157.08).abs() < 0.2, "{}", arch.length());
        assert!(runs_along(&node(r#"{"type":"text","id":"a","onPath":"M0 0 H9"}"#)) && !runs_along(&node(r#"{"type":"text","id":"a"}"#)));
    }

    #[test]
    fn an_outline_is_a_ring_of_copies_and_a_mask_is_told_by_its_property() {
        // A hairline outline takes the fewest copies; a wide one more, and a second ring half way in.
        assert_eq!(outline_ring(1.0, 1.0).len(), 8);
        let wide = outline_ring(8.0, 2.0);
        assert!((120..=200).contains(&wide.len()) && wide.iter().all(|(x, y)| x.hypot(*y) <= 8.001), "{}", wide.len());
        assert!(wide.iter().any(|(x, y)| (x.hypot(*y) - 4.0).abs() < 0.01));
        let node = |json: &str| serde_json::from_str::<Node>(json).unwrap();
        assert!(is_mask(&node(r#"{"type":"path","id":"a","mask":true}"#)));
        assert!(!is_mask(&node(r#"{"type":"path","id":"a","mask":false}"#)) && !is_mask(&node(r#"{"type":"path","id":"a"}"#)));
    }

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
