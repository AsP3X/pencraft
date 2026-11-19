//! Effects: drop and inner shadows, layer blur and background blur.
//!
//! egui draws triangles and can't blur them. So an effect's source (the layer's shape, the layer itself, or
//! what lies behind it) is drawn into a bitmap on the CPU, blurred there, and put on the canvas as a picture.
//! Pictures are kept from one build of the display list to the next and found again by what they were made
//! from, so moving a layer or editing another one bakes nothing anew.
//!
//! Making a picture has two halves. Getting it ready needs the window (text is laid out through it, and it
//! holds the pictures a picture is made from): [`Builder::plan`]. Painting and blurring needs only what was
//! got ready, and is the bulk of the work: a [`Job`], which a build that must not stall the window hands to
//! another thread ([`Baked::texture`]).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};

use egui::epaint::{ClippedShape, Mesh, Primitive, TextShape, Vertex};
use egui::{Color32, ColorImage, Pos2, TextureHandle, TextureId, TextureOptions};
use pen_doc::color::Rgba;
use pen_doc::geom::{Point, Rect};
use pen_doc::layout;
use pen_doc::scene::SceneNode;
use pen_doc::value::{Flag, Num, narrow};
use serde_json::Value;

use super::glass::Pane;
use super::{Builder, Item, Memo, TextItem, color32, glyph_scale, intersect, to_u8};
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
    /// How far the shape is grown (shrunk, when negative) before it casts the shadow; an inner shadow reaches
    /// that much further in.
    pub spread: f32,
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

/// How a tile of a mask's box lies to the mask's outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Cover {
    /// Wholly inside: what is in it shows as it is.
    Inside,
    /// Wholly outside: nothing of it shows.
    Outside,
    /// The outline's edge runs through it: it is drawn as a picture, cut to the outline.
    Edge,
}

/// The side of a mask's tile, in picture pixels.
// Human: small enough that the pictures along the outline's edge are a small part of the mask's area, large
// enough that a shield on screen is a few dozen tiles.
const MASK_TILE: f32 = 256.0;
/// How many tiles a mask's box is laid out in at most; past that the tiles grow.
const MOST_MASK_TILES: f32 = 4096.0;
/// How many tiles' answers are kept.
const KEPT_COVERS: usize = 16_384;

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
// Agent: one thread at a time (RefCell): the display list keeps it behind a lock. A picture not used by a
// build is let go at the end of that build.
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
    /// How each tile of a mask asked about so far lies to its outline, by a hash of the two.
    tiles: RefCell<HashMap<u64, Cover>>,
    /// Whether this build left a picture out because it is still being made.
    late: Cell<bool>,
    /// Whether this build had more pictures to hand out than the threads' line takes.
    cut: Cell<bool>,
    /// For a build that has its pictures made on other threads and goes on without them: the flag that
    /// calls it off. `None` for a build that makes its pictures itself.
    stop: RefCell<Option<Arc<AtomicBool>>>,
    /// The pictures being made on other threads, by key: where each arrives, and the build that last asked.
    pending: RefCell<HashMap<u64, Awaited>>,
    /// How many of them are not made yet (the threads count it down).
    under_way: Arc<AtomicUsize>,
    /// The pictures whose making failed; they stay clear instead of being tried with every build.
    failed: RefCell<HashSet<u64>>,
    /// A copy of egui's font picture, with the pass it was taken in and how full the font picture was then
    /// (see [`Builder::letters`]).
    font: RefCell<Option<(u64, f32, Arc<ColorImage>)>>,
}

/// A picture being made on another thread: where it arrives (`None` when its build was called off before the
/// job's turn came), and the build that last asked for it.
type Awaited = (Receiver<Option<Made>>, u64);

/// A picture as a thread made it, with what making it found out about its material.
pub(super) struct Made {
    /// The picture.
    pub bitmap: Bitmap,
    /// What a material's picture found out about what is behind it.
    pub seen: Option<Seen>,
}

/// The making of one picture: everything it needs is in it, so it can run on any thread.
pub(super) type Job = Box<dyn FnOnce() -> Made + Send>;

/// A job that makes a plain picture.
pub(super) fn job(make: impl FnOnce() -> Bitmap + Send + 'static) -> Job {
    Box::new(move || Made { bitmap: make(), seen: None })
}

/// How many pictures are made at the same time: a thread each, one processor left for the window.
fn most_at_once() -> usize {
    std::thread::available_parallelism().map_or(2, |threads| threads.get().saturating_sub(1)).clamp(1, 6)
}

/// How many pictures wait to be made, at most, for each thread that makes them.
// Human: long enough that a large design is handed out in a few goes (each go walks the whole design), and
// short enough that the copies of what the waiting jobs paint don't fill the memory.
const WAITING_A_THREAD: usize = 32;

/// Whether a build that left pictures out should be gone through again: `under_way` of its pictures are
/// still being made, and `cut` says it had more to hand out than the threads' line took.
// Human: going through a large design again takes tens of milliseconds, so it is done as seldom as will do:
// when every picture asked for has been made (the list can then be whole), or when there is more work to
// hand out and the threads are about to run out of it.
pub(super) fn go_on(under_way: usize, cut: bool) -> bool {
    under_way == 0 || (cut && under_way <= most_at_once())
}

/// One piece of work for the threads that make pictures.
type Task = Box<dyn FnOnce() + Send>;

/// The line the threads that make pictures take their work from; `None` when no thread could be started.
// Human: a few threads that last as long as the app, each taking the next job when it has finished one. (A
// thread of its own for every picture was hundreds of threads for a large design.)
fn workers() -> Option<&'static std::sync::mpsc::Sender<Task>> {
    static LINE: std::sync::OnceLock<Option<std::sync::mpsc::Sender<Task>>> = std::sync::OnceLock::new();
    LINE.get_or_init(|| {
        let (send, take) = std::sync::mpsc::channel::<Task>();
        let take = Arc::new(std::sync::Mutex::new(take));
        let mut started = 0;
        for _ in 0..most_at_once() {
            let take = take.clone();
            let work = move || {
                loop {
                    // The lock is let go before the job runs, so the others can take theirs.
                    let next = crate::app::lock(&take).recv();
                    match next {
                        Ok(task) => task(),
                        Err(_) => break,
                    }
                }
            };
            match std::thread::Builder::new().name("pencraft-bake".to_owned()).spawn(work) {
                Ok(_) => started += 1,
                Err(error) => log::warn!("no thread to make effect pictures on: {error}"),
            }
        }
        (started > 0).then_some(send)
    })
    .as_ref()
}

impl Baked {
    /// Starts a build of the display list that bakes every picture.
    pub(super) fn begin(&self) {
        self.begin_within(None, None);
    }

    /// Starts a build of the display list. With `cover`, only the pictures that reach into that part of the
    /// document are baked; the others are left clear, and [`Baked::skipped`] says so afterwards.
    // Human: baking is by far the most of a build (a blur is worked out pixel by pixel on the CPU), and the
    // pixels grow fourfold with every doubling of the zoom. Zoomed in, most of a design is off screen: its
    // effects are left until the view comes near them.
    // Human: with `stop`, the build doesn't wait for its pictures. Each one new to it is handed to another
    // thread and left clear for now; [`Baked::late`] says so afterwards, and the build is gone through again
    // when pictures have arrived, until it is whole. `stop` is the flag that calls such a build off: set, the
    // jobs still waiting are dropped and the walk through the design ends early.
    // Human: painting a picture and blurring it is the bulk of the work and needs nothing but what was got
    // ready for it, so several are made at the same time.
    pub(super) fn begin_within(&self, cover: Option<Rect>, stop: Option<Arc<AtomicBool>>) {
        self.round.set(self.round.get().wrapping_add(1));
        self.seen.borrow_mut().clear();
        self.cover.set(cover);
        self.skipped.set(false);
        self.late.set(false);
        self.cut.set(false);
        *self.stop.borrow_mut() = stop;
    }

    /// Whether the build was called off.
    pub(super) fn stopped(&self) -> bool {
        self.stop.borrow().as_ref().is_some_and(|stop| stop.load(Ordering::Relaxed))
    }

    /// Whether this build bakes pictures for `area`; says so afterwards ([`Baked::skipped`]) when it doesn't.
    fn covers(&self, area: &Rect) -> bool {
        let covered = self.cover.get().is_none_or(|cover| cover.intersects(area));
        if !covered {
            self.skipped.set(true);
        }
        covered
    }

    /// What says when an unfinished build should be gone through again: how many pictures are still being
    /// made, and whether the build had more to hand out ([`go_on`]).
    pub(super) fn waiting(&self) -> (Arc<AtomicUsize>, bool) {
        (self.under_way.clone(), self.cut.get())
    }

    /// Counts the pictures `shown` (their textures) as used by this build, so [`Baked::finish`] keeps them: a
    /// build that takes over items of the list before it never asked for their pictures.
    pub(super) fn hold(&self, shown: &HashSet<TextureId>) {
        let round = self.round.get();
        for picture in self.pictures.borrow_mut().values_mut().filter(|picture| shown.contains(&picture.handle.id())) {
            picture.round = round;
        }
    }

    /// Whether the picture with `texture` is still held.
    #[cfg(test)]
    pub(super) fn holds(&self, texture: TextureId) -> bool {
        self.pictures.borrow().values().any(|picture| picture.handle.id() == texture)
    }

    /// Whether `texture` is the clear picture that stands in for one left out.
    fn is_blank(&self, texture: TextureId) -> bool {
        self.blank.borrow().as_ref().is_some_and(|blank| blank.id() == texture)
    }

    /// Whether the build since the last [`Baked::begin_within`] left pictures out because they are still
    /// being made.
    pub(super) fn late(&self) -> bool {
        self.late.get()
    }

    /// The clear picture that stands in for one left out.
    fn blank(&self, ctx: &egui::Context) -> TextureId {
        self.blank
            .borrow_mut()
            .get_or_insert_with(|| ctx.load_texture("effect-left-out", ColorImage::filled([1, 1], Color32::TRANSPARENT), TextureOptions::LINEAR))
            .id()
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
    /// They are handed back: the list on screen may still show them, and whoever shows the new list lets
    /// them go after that.
    // Human: the ones kept are those of the zoom step just left and of the part of the canvas just scrolled
    // away from, so going back to either costs nothing.
    #[must_use]
    pub(super) fn finish(&self) -> Vec<TextureHandle> {
        let round = self.round.get();
        let mut pictures = self.pictures.borrow_mut();
        let mut unused: Vec<(u64, u64, usize)> = pictures
            .iter()
            .filter(|(_, picture)| picture.round != round)
            .map(|(key, picture)| (*key, picture.round, picture.pixels.size[0] * picture.pixels.size[1]))
            .collect();
        unused.sort_by_key(|(_, last, _)| std::cmp::Reverse(*last));
        let mut kept = 0usize;
        let mut let_go = Vec::new();
        for (key, _, size) in unused {
            kept += size;
            if kept > KEPT_PIXELS {
                let_go.extend(pictures.remove(&key).map(|picture| picture.handle));
            }
        }
        self.learned.borrow_mut().retain(|key, _| pictures.contains_key(key));
        // A picture still being made that this build didn't ask for is for a zoom or a scene left behind.
        self.pending.borrow_mut().retain(|_, (_, asked)| *asked == round);
        // The tiles of masks are few; a long session over many designs is what fills this.
        if self.tiles.borrow().len() > KEPT_COVERS {
            self.tiles.borrow_mut().clear();
        }
        let_go
    }

    /// The texture for `key`, made by the job `prepare` gives unless an earlier build already made it. `area`
    /// is the part of the document the picture shows. A picture outside what this build covers and one that
    /// is still being made on another thread come back clear.
    // Human: `prepare` runs here and gathers what the picture is made from; it gives no job when one of those
    // things is itself a picture not made yet. The job it gives is run at once by a build that waits for
    // its pictures (an export, an edit), and on another thread by one that doesn't.
    pub(super) fn texture(&self, ctx: &egui::Context, key: u64, area: Rect, prepare: impl FnOnce() -> Option<Job>) -> TextureId {
        self.texture_from(ctx, key, area, false, prepare)
    }

    /// [`Baked::texture`] for a picture that is made as it is got ready: its job is run at once.
    pub(super) fn texture_here(&self, ctx: &egui::Context, key: u64, area: Rect, make: impl FnOnce() -> Bitmap) -> TextureId {
        self.texture_from(ctx, key, area, true, || {
            let bitmap = make();
            Some(Box::new(move || Made { bitmap, seen: None }))
        })
    }

    fn texture_from(&self, ctx: &egui::Context, key: u64, area: Rect, here: bool, prepare: impl FnOnce() -> Option<Job>) -> TextureId {
        if let Some(picture) = self.pictures.borrow_mut().get_mut(&key) {
            picture.round = self.round.get();
            return picture.handle.id();
        }
        if self.cover.get().is_some_and(|cover| !cover.intersects(&area)) {
            self.skipped.set(true);
            return self.blank(ctx);
        }
        let apart = self.stop.borrow().is_some();
        let under_way = self.pending.borrow_mut().remove(&key);
        if let Some((arrives, _)) = under_way {
            // A build that waits for its pictures waits for this one too: that is sooner than making it again.
            let arrived = if apart { arrives.try_recv() } else { arrives.recv().map_err(|_| TryRecvError::Disconnected) };
            match arrived {
                Ok(Some(made)) => return self.keep(ctx, key, made),
                // Its build was called off before the job's turn came: it is made for this one.
                Ok(None) => {}
                Err(TryRecvError::Empty) => {
                    self.pending.borrow_mut().insert(key, (arrives, self.round.get()));
                    self.late.set(true);
                    return self.blank(ctx);
                }
                Err(TryRecvError::Disconnected) => {
                    log::warn!("an effect's picture could not be made; the effect is left out");
                    self.failed.borrow_mut().insert(key);
                    return self.blank(ctx);
                }
            }
        }
        if self.failed.borrow().contains(&key) || self.stopped() {
            return self.blank(ctx);
        }
        let line = workers().filter(|_| apart && !here);
        if line.is_some() && self.pending.borrow().len() >= most_at_once() * WAITING_A_THREAD {
            // There is more to hand out once the threads have caught up.
            self.late.set(true);
            self.cut.set(true);
            return self.blank(ctx);
        }
        // `prepare` reads other pictures, so nothing is borrowed while it runs.
        let Some(job) = prepare() else { return self.blank(ctx) };
        let Some(line) = line else { return self.keep(ctx, key, job()) };
        self.late.set(true);
        let (send, arrives) = std::sync::mpsc::channel();
        let (under_way, stop) = (self.under_way.clone(), self.stop.borrow().clone());
        under_way.fetch_add(1, Ordering::Relaxed);
        let task: Task = Box::new(move || {
            // A job whose build was called off is not run. The build that asked may also be gone; then
            // nobody takes the picture.
            let called_off = stop.is_some_and(|stop| stop.load(Ordering::Relaxed));
            drop(send.send((!called_off).then(job)));
            under_way.fetch_sub(1, Ordering::Relaxed);
        });
        if line.send(task).is_ok() {
            self.pending.borrow_mut().insert(key, (arrives, self.round.get()));
        } else {
            // The threads are gone; the next go makes the picture itself.
            self.under_way.fetch_sub(1, Ordering::Relaxed);
            self.cut.set(true);
        }
        self.blank(ctx)
    }

    /// Takes in the picture made for `key` and gives its texture.
    fn keep(&self, ctx: &egui::Context, key: u64, made: Made) -> TextureId {
        if let Some(seen) = made.seen {
            self.learned.borrow_mut().insert(key, seen);
        }
        let pixels = Arc::new(made.bitmap.image());
        let handle = ctx.load_texture(format!("effect-{key:016x}"), pixels.clone(), TextureOptions::LINEAR);
        let id = handle.id();
        self.pictures.borrow_mut().insert(key, Picture { handle, pixels, round: self.round.get() });
        id
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

/// How often a painting's texts are laid out again because the font picture was emptied meanwhile.
const MOST_FONT_TRIES: usize = 4;

/// One thing a painting paints, placed in the pixels of its picture.
enum Stroke {
    /// Triangles, textured with `picture` when they have one. `on` is the grid a mesh still in document
    /// coordinates is placed on; a text's meshes are in pixels already.
    Mesh { mesh: Arc<Mesh>, on: Option<Grid>, clip: Option<Span>, picture: Option<Arc<ColorImage>> },
    /// The outlines round a text's letters, the outermost first: each one's colour and how far it reaches
    /// out of the letters, in pixels. `letters` are the text's meshes, `font` the picture they point into.
    Outline { letters: Vec<Mesh>, font: Option<Arc<ColorImage>>, pad: usize, rings: Vec<(raster::Pixel, f32)>, clip: Option<Span> },
    /// The soft shadow of a box.
    SoftBox { span: Span, radius: f32, blur: f32, color: raster::Pixel, clip: Option<Span> },
}

/// Items made ready to be painted onto a bitmap: text laid out, meshes placed, the pictures they show at hand.
// Human: getting items ready needs the window (text is laid out through it, and it has the pictures); painting
// them needs nothing but this, so it can be done on another thread.
pub(super) struct Painting {
    strokes: Vec<Stroke>,
    /// Whether the shapes are painted, fully opaque, instead of their colours.
    solid: bool,
    /// Whether one of the items shows a picture that isn't made yet: the painting would have a hole.
    pub starved: bool,
}

impl Painting {
    /// Paints onto `bitmap`, which covers the grid the painting was made ready for.
    pub(super) fn onto(&self, bitmap: &mut Bitmap) {
        for stroke in &self.strokes {
            match stroke {
                Stroke::Mesh { mesh, on: Some(grid), clip, picture } => {
                    bitmap.triangles(mesh, |p| grid.place(Point::new(p.x, p.y)), *clip, picture.as_deref(), self.solid);
                }
                Stroke::Mesh { mesh, on: None, clip, picture } => bitmap.triangles(mesh, |p| [p.x, p.y], *clip, picture.as_deref(), self.solid),
                Stroke::Outline { letters, font, pad, rings, clip } => {
                    let mut shape = Bitmap::new(bitmap.w + 2 * pad, bitmap.h + 2 * pad);
                    let by = length(*pad);
                    for mesh in letters {
                        shape.triangles(mesh, |p| [p.x + by, p.y + by], None, font.as_deref(), true);
                    }
                    for (color, reach) in rings {
                        bitmap.outline(&shape, *pad, *reach, *color, *clip);
                    }
                }
                Stroke::SoftBox { span, radius, blur, color, clip } => bitmap.soft_box(*span, *radius, *blur, *color, *clip),
            }
        }
    }
}

/// Whether every triangle of `mesh` (letters pointing into the font picture) has ink where it points into `font`.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "texture coordinates are within the picture, and clamped to it")]
fn inked(mesh: &Mesh, font: &ColorImage) -> bool {
    let [w, h] = font.size;
    if w == 0 || h == 0 {
        return false;
    }
    let at = |v: f32, side: usize| ((v * length(side)).floor().max(0.0) as usize).min(side - 1);
    mesh.indices.as_chunks::<3>().0.iter().all(|corners| {
        let uvs = corners.map(|index| usize::try_from(index).ok().and_then(|index| mesh.vertices.get(index)).map_or(Pos2::ZERO, |vertex| vertex.uv));
        let (x0, x1) = (at(uvs[0].x.min(uvs[1].x).min(uvs[2].x), w), at(uvs[0].x.max(uvs[1].x).max(uvs[2].x), w));
        let (y0, y1) = (at(uvs[0].y.min(uvs[1].y).min(uvs[2].y), h), at(uvs[0].y.max(uvs[1].y).max(uvs[2].y), h));
        (y0..=y1).any(|y| font.pixels.get(y * w + x0..=y * w + x1).is_some_and(|row| row.iter().any(|pixel| pixel.a() > 0)))
    })
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
        Item::Mesh { mesh, memo, .. } => {
            if let Some(known) = memo.extent.get() {
                return Some(known);
            }
            let first = mesh.vertices.first()?.pos;
            let (mut min, mut max) = (first, first);
            for v in &mesh.vertices {
                (min, max) = (min.min(v.pos), max.max(v.pos));
            }
            let found = Rect::new(min.x, min.y, max.x - min.x, max.y - min.y);
            memo.extent.set(Some(found));
            Some(found)
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
        // Human: the mesh's own look is hashed once for each density it is asked at, with its corners
        // measured from the corner of its box; where that corner lies on the grid goes in each time.
        Item::Mesh { mesh, clip, bounds, memo } => {
            let density = grid.density();
            let look = match memo.look.get() {
                Some((at, look)) if at == density.to_bits() => look,
                _ => {
                    let mut own = DefaultHasher::new();
                    (mesh.texture_id, &mesh.indices).hash(&mut own);
                    for v in &mesh.vertices {
                        let from_corner = [step((v.pos.x - bounds.x) * density), step((v.pos.y - bounds.y) * density)];
                        (from_corner, v.color.to_array(), v.uv.x.to_bits(), v.uv.y.to_bits()).hash(&mut own);
                    }
                    let look = own.finish();
                    memo.look.set(Some((density.to_bits(), look)));
                    look
                }
            };
            (0_u8, look, at(Point::new(bounds.x, bounds.y)), clip_of(clip)).hash(hasher);
        }
        Item::Text { text, clip, .. } => {
            let m = text.to_doc.m;
            (1_u8, format!("{:?}{:?}{:?}{:?}", text.style, text.circle, text.track, text.outlines), text.color.to_array(), clip_of(clip))
                .hash(hasher);
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
        let whole = scenes.iter().map(|scene| layout::doc_bounds(scene)).reduce(|a, b| a.union(&b))?;
        // Human: a group has no box of its own, only the one round what is in it; what a mask in it cuts
        // away would otherwise leave an empty margin in the picture. A group is as large as what it paints.
        let frame = scenes
            .iter()
            .filter(|scene| scene.node.kind != pen_doc::model::Kind::Group)
            .map(|scene| layout::doc_bounds(scene))
            .reduce(|a, b| a.union(&b));
        // Human: what a clipping frame cuts off is not in the picture, so it takes no room in it either.
        let painted = items
            .iter()
            .filter_map(|item| extent(item).map(|e| item.clip().map_or(e, |clip| intersect(&e, &clip))))
            .filter(|e| e.w > 0.0 && e.h > 0.0)
            .reduce(|a, b| a.union(&b));
        let area = match (painted, frame) {
            (Some(painted), Some(frame)) => painted.union(&frame),
            (Some(area), None) | (None, Some(area)) => area,
            (None, None) => whole,
        };
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
            let texture = self.baked.texture(self.ctx, key, grid.rect(), || self.cut_out(&grid, &here, outline));
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

    /// Cuts `inside` (the items of the layers that follow a mask layer) to the mask's `outline`, whose box is
    /// `area`, and adds what is left to `out`.
    // Human: the canvas can only cut to upright rectangles, and a mask has any shape (a shield, a letter, a
    // bird). So the mask's box is laid out in square tiles of a fixed size on screen. A tile that lies wholly
    // inside the outline keeps its items as they are, each cut to the tile (tiles side by side joined into
    // one band): they stay sharp at any zoom and cost no picture. A tile wholly outside shows nothing. Only
    // a tile the outline's edge runs through becomes a small picture of what lies there, with everything
    // outside the outline rubbed out; and only where the view is, like every other picture.
    pub(super) fn mask_off(&self, outline: &Item, area: Rect, cut: Option<Rect>, inside: &[Item], out: &mut Vec<Item>) {
        let area = cut.map_or(area, |cut| intersect(&area, &cut));
        if area.w <= 0.0 || area.h <= 0.0 || inside.is_empty() || self.scale <= 0.0 {
            return;
        }
        // A tile's side in the document; larger when the box would take more tiles than are worth going through.
        let side = (MASK_TILE / self.scale).max((area.w * area.h / MOST_MASK_TILES).sqrt());
        let touches = |item: &Item, part: &Rect| {
            let reach = item.clip().map_or(*part, |clip| intersect(&clip, part));
            (reach.w > 0.0 && reach.h > 0.0 && extent(item).is_some_and(|e| e.intersects(&reach))).then_some(reach)
        };
        // The bands of tiles wholly inside the outline, each a run of tiles in one row.
        let mut bands: Vec<Rect> = Vec::new();
        let mut y = area.y;
        while y < area.bottom() - 1e-3 {
            let h = side.min(area.bottom() - y);
            let mut run: Option<Rect> = None;
            let mut x = area.x;
            while x < area.right() - 1e-3 {
                let tile = Rect::new(x, y, side.min(area.right() - x), h);
                x += side;
                let whole = self.baked.covers(&tile) && {
                    match self.cover_of(outline, &tile) {
                        Cover::Inside => true,
                        Cover::Outside => false,
                        Cover::Edge => {
                            self.mask_tile(outline, tile, cut, inside, out);
                            false
                        }
                    }
                };
                if whole {
                    run = Some(run.map_or(tile, |run| run.union(&tile)));
                } else {
                    bands.extend(run.take());
                }
            }
            bands.extend(run.take());
            y += side;
        }
        // Bands one under the other with the same ends are one block.
        let mut blocks: Vec<Rect> = Vec::new();
        for band in bands {
            match blocks
                .iter_mut()
                .find(|block| (block.x - band.x).abs() < 1e-3 && (block.w - band.w).abs() < 1e-3 && (block.bottom() - band.y).abs() < 1e-3)
            {
                Some(block) => *block = block.union(&band),
                None => blocks.push(band),
            }
        }
        for block in blocks {
            out.extend(inside.iter().filter_map(|item| touches(item, &block).map(|reach| item.clone().clipped_to(reach))));
        }
    }

    /// Whether `tile` lies inside the mask's `outline`, outside it, or on its edge.
    // Human: the outline alone is painted over the tile, one sample a pixel; that is cheap, and kept, since
    // the same tiles are asked about with every build.
    fn cover_of(&self, outline: &Item, tile: &Rect) -> Cover {
        let Some(grid) = Grid::new(*tile, None, 0.0, self.scale, false) else { return Cover::Outside };
        let key = Self::key("cover", &grid, &[(&[outline], true)], &[], Color32::TRANSPARENT);
        if let Some(known) = self.baked.tiles.borrow().get(&key) {
            return *known;
        }
        let mut solid = grid.bitmap();
        self.paint(&grid, &[outline], true, &mut solid);
        let (mut any, mut all) = (false, true);
        for pixel in &solid.px {
            any |= pixel[3] > 0.0;
            all &= pixel[3] >= 1.0;
        }
        let found = match (any, all) {
            (_, true) => Cover::Inside,
            (false, false) => Cover::Outside,
            (true, false) => Cover::Edge,
        };
        self.baked.tiles.borrow_mut().insert(key, found);
        found
    }

    /// The picture of what `inside` shows in `tile`, cut to the mask's `outline`, added to `out`.
    fn mask_tile(&self, outline: &Item, tile: Rect, cut: Option<Rect>, inside: &[Item], out: &mut Vec<Item>) {
        let here: Vec<&Item> = inside
            .iter()
            .filter(|item| extent(item).is_some_and(|e| e.intersects(&item.clip().map_or(tile, |clip| intersect(&clip, &tile)))))
            .collect();
        let Some(grid) = Grid::new(tile, None, 0.0, self.scale, true).filter(|_| !here.is_empty()) else { return };
        let key = Self::key("mask", &grid, &[(&here, false), (&[outline], true)], &[], Color32::TRANSPARENT);
        let texture = self.baked.texture(self.ctx, key, grid.rect(), || self.cut_out(&grid, &here, outline));
        out.push(Self::picture(texture, &grid, Point::default(), 1.0, cut));
    }

    /// The making of a picture of `inside` on `grid` with everything outside `outline` rubbed out.
    fn cut_out(&self, grid: &Grid, inside: &[&Item], outline: &Item) -> Option<Job> {
        let (inside, outline, grid) = (self.plan(grid, inside, false), self.plan(grid, &[outline], true), *grid);
        if inside.starved || outline.starved {
            return None;
        }
        Some(job(move || {
            let mut bitmap = grid.bitmap();
            inside.onto(&mut bitmap);
            let mut bitmap = grid.settle(bitmap);
            let mut solid = grid.bitmap();
            outline.onto(&mut solid);
            for (pixel, cover) in bitmap.px.iter_mut().zip(&grid.settle(solid).px) {
                *pixel = pixel.map(|c| c * cover[3]);
            }
            bitmap
        }))
    }

    /// The effects of `scene` that are switched on, variables looked up.
    // Human: a shadow is its offset, colour (black when unset), `blur` and `spread`.
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
                    let shadow = Shadow { offset, blur: number(effect.get("blur")).max(0.0), color, spread: number(effect.get("spread")) };
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
                // A box grown by the spread is a box again, its corners rounder by as much.
                Some(radius) => Some(Item::Shadow {
                    rect: expand(&layout::doc_bounds(scene), shadow.spread),
                    radius: (radius + shadow.spread).max(0.0),
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
            self.reads_behind(area);
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
        // A shader reads files and variables as it runs: its picture is made here, not on another thread.
        let texture = self.baked.texture_here(self.ctx, key, grid.rect(), || {
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
        self.plan(grid, items, solid).onto(bitmap);
    }

    /// `items` made ready to be painted onto a bitmap that covers `grid` ([`Painting::onto`]). `solid` paints
    /// their shapes, fully opaque.
    // Agent: MAIN-THREAD (lays text out through the egui context, reads the pictures); meshes are shared.
    pub(super) fn plan(&self, grid: &Grid, items: &[&Item], solid: bool) -> Painting {
        // Human: texts are laid out first, so the font picture read after that holds all their letters.
        // Human: a letter new to the font picture can make the picture grow, and a mesh made before that points
        // at where its letters were. So the texts are laid out twice: once to bring every letter into the
        // picture, then, with the picture settled, for the meshes that are painted.
        let lay_out = || -> Vec<Vec<Mesh>> {
            items.iter().map(|item| if let Item::Text { text, .. } = item { self.text_meshes(text, grid) } else { Vec::new() }).collect()
        };
        // Human: a build beside the window lays text out while the window draws. When the font picture is
        // nearly full the window empties it at the start of a frame, and letters laid out before that point
        // at places that hold other letters afterwards. The font picture only ever gets fuller otherwise, so
        // one that is less full after the texts were laid out and copied was emptied in between: once more.
        // (Without a text among the items the font picture is not asked about at all.)
        let wordy = items.iter().any(|item| matches!(item, Item::Text { .. }));
        let mut tries = 0;
        let (texts, font, settled) = loop {
            let before = if wordy { self.font_fill() } else { 0.0 };
            let first = lay_out();
            let any = first.iter().any(|meshes| !meshes.is_empty());
            let texts = if any { lay_out() } else { first };
            let font = any.then(|| self.letters(&texts));
            tries += 1;
            let settled = !any || self.font_fill() >= before;
            if settled || tries >= MOST_FONT_TRIES {
                break (texts, font, settled);
            }
        };
        // Never settled: the picture waits for the next go.
        let mut painting = Painting { strokes: Vec::new(), solid, starved: !settled };
        for (item, text) in items.iter().zip(texts) {
            match item {
                Item::Mesh { mesh, clip, .. } => {
                    let picture = if mesh.texture_id == TextureId::default() {
                        None
                    } else if self.baked.is_blank(mesh.texture_id) {
                        // A picture left out of this build: whatever is made from it has to wait for it.
                        painting.starved = true;
                        continue;
                    } else {
                        self.baked.pixels(mesh.texture_id).or_else(|| self.images.pixels(mesh.texture_id))
                    };
                    // The mesh is shared, not copied; the job places it on the grid as it paints.
                    painting.strokes.push(Stroke::Mesh { mesh: mesh.clone(), on: Some(*grid), clip: clip.map(|c| grid.span(&c)), picture });
                }
                Item::Text { text: item, clip, .. } => {
                    let clip = clip.map(|c| grid.span(&c));
                    // Human: the outline round letters is everything within its width of them, found from one
                    // painting of the letters; the shape of an outlined text has its outline in it.
                    if let Some((_, widest)) = item.outlines.first() {
                        let rings =
                            item.outlines.iter().map(|(ink, width)| (if solid { [1.0; 4] } else { raster::pixel(*ink) }, width * grid.density()));
                        painting.strokes.push(Stroke::Outline {
                            letters: text.clone(),
                            font: font.clone(),
                            pad: pixels(widest * grid.density() + 2.0),
                            rings: rings.collect(),
                            clip,
                        });
                    }
                    painting.strokes.extend(text.into_iter().map(|mesh| Stroke::Mesh { mesh: mesh.into(), on: None, clip, picture: font.clone() }));
                }
                // A box's own soft shadow is no part of its shape.
                Item::Shadow { rect, radius, offset, blur, color, clip } if !solid => painting.strokes.push(Stroke::SoftBox {
                    span: grid.span(&rect.translate(offset.x, offset.y)),
                    radius: radius * grid.density(),
                    blur: blur * grid.density(),
                    color: raster::pixel(*color),
                    clip: clip.map(|c| grid.span(&c)),
                }),
                Item::Shadow { .. } => {}
            }
        }
        painting
    }

    /// egui's font picture, which the text meshes `texts` are textured with.
    // Human: the font picture is large (tens of megabytes on a sharp screen) and egui only hands out a copy
    // of all of it. Copying it for every picture with text in it cost more than painting the text. So one
    // copy is kept for as long as it is good. Within one pass of the window the font picture only gains
    // letters; a letter the copy was taken before is still clear in it. So the copy is good for these meshes
    // when every letter they show has ink in it, and is taken anew when one hasn't.
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "the method's path is not general over the font view's lifetime and doesn't compile"
    )]
    fn letters(&self, texts: &[Vec<Mesh>]) -> Arc<ColorImage> {
        let (pass, fill) = (self.ctx.cumulative_pass_nr(), self.font_fill());
        // Emptied since the copy was taken (see `plan`), and the copy is of what was there before.
        if let Some((taken, full, copy)) = self.baked.font.borrow().as_ref()
            && *taken == pass
            && fill >= *full
            && texts.iter().flatten().all(|mesh| inked(mesh, copy))
        {
            return copy.clone();
        }
        let copy = Arc::new(self.ctx.fonts(|fonts| fonts.image()));
        *self.baked.font.borrow_mut() = Some((pass, fill, copy.clone()));
        copy
    }

    /// How full egui's font picture is.
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "the method's path is not general over the font view's lifetime and doesn't compile"
    )]
    fn font_fill(&self) -> f32 {
        self.ctx.fonts(|fonts| fonts.font_atlas_fill_ratio())
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
        let (laid, grow) = glyph_scale(text.style.size, z, per_point, true);
        // Human: set on a circle, each letter is a paragraph of its own. (An outline is painted from these.)
        let parts = super::text_placed(text, text.color, laid, &mut |job| self.ctx.fonts_mut(|fonts| fonts.layout_job(job)));
        let mut meshes = Vec::new();
        for (anchor, galley, angle) in parts {
            let [x, y] = grid.place(anchor);
            let shape = TextShape::new(Pos2::ZERO, galley, text.color).with_angle(angle);
            meshes.extend(self.ctx.tessellate(vec![ClippedShape::new(egui::Rect::EVERYTHING, shape)], per_point).into_iter().filter_map(|clipped| {
                match clipped.primitive {
                    Primitive::Mesh(mut mesh) => {
                        for v in &mut mesh.vertices {
                            v.pos = Pos2::new(v.pos.x * grow * per_point + x, v.pos.y * grow * per_point + y);
                        }
                        Some(mesh)
                    }
                    Primitive::Callback(_) => None,
                }
            }));
        }
        meshes
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
        Item::Mesh { mesh: Mesh { indices: vec![0, 1, 2, 0, 2, 3], vertices, texture_id: texture }.into(), bounds, clip, memo: Memo::default() }
    }

    /// The shadow `shape` casts: its blurred outline in the shadow's colour, moved by the offset.
    // Human: the shape is drawn solid, whatever its fill's opacity, and blurred; when the shape
    // doesn't hide what is under it, the shape itself is cut out of the shadow, so a see-through layer shows no
    // shadow through its fill.
    pub(super) fn drop_shadow(&self, shape: &[&Item], shadow: &Shadow, look: &Look) -> Option<Item> {
        let sigma = sigma(shadow.blur);
        let pad = sigma * REACH + shadow.spread.max(0.0) + 1.0 / self.scale;
        // The shadow is clipped where it lands, which is `offset` away from the shape it is made from.
        let limit = look.clip.map(|c| c.translate(-shadow.offset.x, -shadow.offset.y));
        let grid = Grid::new(expand(&extent_of(shape)?, pad), limit, pad, self.scale, sigma * self.scale < 1.0)?;
        let (sigma, offset, grown) = (sigma * grid.scale, [shadow.offset.x * grid.scale, shadow.offset.y * grid.scale], shadow.spread * grid.scale);
        let color = color32(shadow.color);
        let cut = !look.opaque;
        let key = Self::key(if cut { "drop-cut" } else { "drop" }, &grid, &[(shape, true)], &[sigma, offset[0], offset[1], grown], color);
        let texture = self.baked.texture(self.ctx, key, grid.rect(), || {
            let shape = self.plan(&grid, shape, true);
            if shape.starved {
                return None;
            }
            Some(job(move || {
                let mut solid = grid.bitmap();
                shape.onto(&mut solid);
                let solid = grid.settle(solid);
                let mut soft = solid.clone();
                soft.spread(grown);
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
            }))
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
        let (sigma, offset, grown) = (sigma * grid.scale, [shadow.offset.x * grid.scale, shadow.offset.y * grid.scale], shadow.spread * grid.scale);
        let color = color32(shadow.color);
        let key = Self::key("inner", &grid, &[(shape, true)], &[sigma, offset[0], offset[1], grown], color);
        let texture = self.baked.texture(self.ctx, key, grid.rect(), || {
            let shape = self.plan(&grid, shape, true);
            if shape.starved {
                return None;
            }
            Some(job(move || {
                let mut solid = grid.bitmap();
                shape.onto(&mut solid);
                let mut solid = grid.settle(solid);
                // Human: the light that leaves the rim dark comes through a smaller opening the more the
                // shadow spreads.
                let mut soft = solid.clone();
                soft.spread(-grown);
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
            }))
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
            let layer = self.plan(&grid, layer, false);
            if layer.starved {
                return None;
            }
            Some(job(move || {
                let mut bitmap = grid.bitmap();
                layer.onto(&mut bitmap);
                let mut bitmap = grid.settle(bitmap);
                bitmap.blur(sigma);
                bitmap
            }))
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
        self.reads_behind(area);
        let behind: Vec<&Item> = behind.iter().copied().filter(|item| extent(item).is_some_and(|e| e.intersects(&area))).collect();
        let sigma = sigma * grid.scale;
        let key = Self::key("backdrop", &grid, &[(&behind, false), (shape, true)], &[sigma], self.background);
        let ground = raster::pixel(self.background);
        let texture = self.baked.texture(self.ctx, key, grid.rect(), || {
            let (behind, shape) = (self.plan(&grid.coarse(), &behind, false), self.plan(&grid, shape, true));
            if behind.starved || shape.starved {
                return None;
            }
            Some(job(move || {
                let mut back = grid.coarse().bitmap();
                back.fill(ground);
                behind.onto(&mut back);
                back.blur(sigma);
                let mut solid = grid.bitmap();
                shape.onto(&mut solid);
                let solid = grid.settle(solid);
                for (pixel, cover) in back.px.iter_mut().zip(&solid.px) {
                    *pixel = pixel.map(|c| c * cover[3]);
                }
                back
            }))
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
                mesh: Mesh { indices: vec![0, 1, 2, 0, 2, 3], vertices, ..Default::default() }.into(),
                bounds: Rect::new(x, 0.0, 10.0, 10.0),
                clip: None,
                memo: Memo::default(),
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
