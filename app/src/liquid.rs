//! The liquid the task list pours in with: where its drops are at each moment, and the picture of them.
//!
//! A new task list is not simply there. Liquid pools under the chat's header, and one drop falls from it for
//! every task, each from another place and at its own pace. Where a drop lands, its row pours out to both
//! sides and soaks away, and the task stands there. When a task is finished a small drop runs on to the next.
//!
//! The liquid is a set of rounded shapes that melt into each other where they come close. This module works
//! out the shapes for a moment in time ([`Pour::blobs`], [`step`]) and paints them into a picture
//! ([`picture`]): an orange body, a thin lit rim along which a white light travels, and a glow around it.
//! Nothing here knows of the window; the panel ([`crate::tasklist`]) shows the pictures.

use egui::{Color32, ColorImage};

use crate::theme::{
    LIQUID, LIQUID_BLOOM, LIQUID_CORE, LIQUID_CORE_DEPTH, LIQUID_GLOW, LIQUID_HALO, LIQUID_LIGHT_PACE, LIQUID_LIGHT_SHARE, LIQUID_MELT, LIQUID_REACH,
    LIQUID_RIM, LIQUID_RIM_WIDTH, TAB_POUR_RIM, TASKS_HAND_FALL, TASKS_HAND_MELT, TASKS_HAND_SOAK, TASKS_POOL, TASKS_POUR_BAND, TASKS_POUR_GAP,
    TASKS_POUR_GAP_MORE, TASKS_POUR_LAND, TASKS_POUR_SOAK,
};

/// One rounded shape of liquid: a box with fully rounded ends, so a square one is a circle and a long one a
/// capsule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blob {
    /// Its middle.
    pub centre: [f32; 2],
    /// Half its width and half its height.
    pub half: [f32; 2],
}

impl Blob {
    /// A circle.
    fn round(x: f32, y: f32, radius: f32) -> Blob {
        Blob { centre: [x, y], half: [radius, radius] }
    }

    /// How far the point `at` is from the shape's edge: negative inside it.
    fn distance(&self, at: [f32; 2]) -> f32 {
        let radius = self.half[0].min(self.half[1]);
        let (qx, qy) = ((at[0] - self.centre[0]).abs() - (self.half[0] - radius), (at[1] - self.centre[1]).abs() - (self.half[1] - radius));
        qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
    }

    /// Whether the shape is too far from `at` to be seen there, glow and all.
    fn far(&self, at: [f32; 2]) -> bool {
        (at[0] - self.centre[0]).abs() > self.half[0] + LIQUID_REACH || (at[1] - self.centre[1]).abs() > self.half[1] + LIQUID_REACH
    }
}

/// The smaller of two distances, rounded off where they are close: two shapes near each other grow together.
// Source: the polynomial smooth minimum, Inigo Quilez, "Smooth minimum" (iquilezles.org/articles/smin).
fn melted(a: f32, b: f32) -> f32 {
    let share = (LIQUID_MELT - (a - b).abs()).max(0.0) / LIQUID_MELT;
    a.min(b) - share * share * LIQUID_MELT * 0.25
}

/// 0 below `from`, 1 above `to`, and a smooth rise between.
fn smooth(from: f32, to: f32, value: f32) -> f32 {
    let share = ((value - from) / (to - from)).clamp(0.0, 1.0);
    share * share * (3.0 - 2.0 * share)
}

/// `a` to `b`, `share` of the way.
fn mix(a: f32, b: f32, share: f32) -> f32 {
    a + (b - a) * share
}

/// The colour `share` of the way from `a` to `b`, as three numbers from 0 to 255.
fn blend(a: [f32; 3], b: [f32; 3], share: f32) -> [f32; 3] {
    [mix(a[0], b[0], share), mix(a[1], b[1], share), mix(a[2], b[2], share)]
}

fn rgb(color: Color32) -> [f32; 3] {
    [f32::from(color.r()), f32::from(color.g()), f32::from(color.b())]
}

/// Where the light that travels along the liquid's rim is at `time` seconds, in a picture `width` wide: it
/// comes in from the left, crosses, and comes in again.
#[must_use]
pub fn light_at(time: f32, width: f32) -> f32 {
    let share = (time / LIQUID_LIGHT_PACE).rem_euclid(1.0);
    mix(-LIQUID_LIGHT_SHARE * width, (1.0 + LIQUID_LIGHT_SHARE) * width, share)
}

/// The picture of `blobs`: the part that starts at `origin` and is `size` texels large, with `scale` texels to
/// a point. `light` is where the travelling light is, in points from the left of the whole liquid, and `width`
/// how wide that whole is.
// Human: one texel is worked out from how far it is from the liquid's edge. Inside: the body, darker towards
// the middle, and a thin rim at the edge that is white where the light is. Outside: a soft orange glow, and a
// white bloom beside the lit part of the rim.
// Agent: PURE; costs one distance for each texel and each shape near it.
#[must_use]
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "colour channels, kept between 0 and 255")]
#[expect(clippy::cast_precision_loss, reason = "texel positions, far below where f32 loses whole numbers")]
pub fn picture(blobs: &[Blob], origin: [f32; 2], size: [usize; 2], scale: f32, light: f32, width: f32) -> ColorImage {
    let (body, core, rim, glow) = (rgb(LIQUID), rgb(LIQUID_CORE), rgb(LIQUID_RIM), rgb(LIQUID_GLOW));
    let white = [255.0; 3];
    let reach = LIQUID_LIGHT_SHARE * width;
    let mut pixels = vec![Color32::TRANSPARENT; size[0] * size[1]];
    for (row, line) in pixels.chunks_mut(size[0].max(1)).enumerate() {
        let y = origin[1] + (row as f32 + 0.5) / scale;
        for (column, pixel) in line.iter_mut().enumerate() {
            let at = [origin[0] + (column as f32 + 0.5) / scale, y];
            let mut distance = f32::INFINITY;
            for blob in blobs.iter().filter(|blob| !blob.far(at)) {
                distance = melted(distance, blob.distance(at));
            }
            if distance > LIQUID_REACH {
                continue;
            }
            let lit = 1.0 - smooth(0.0, reach, (at[0] - light).abs());
            let cover = (0.5 - distance * scale).clamp(0.0, 1.0);
            let edge = 1.0 - smooth(LIQUID_RIM_WIDTH - 1.0, LIQUID_RIM_WIDTH, -distance);
            let inside = blend(blend(body, core, smooth(0.0, LIQUID_CORE_DEPTH, -distance)), blend(rim, white, lit), edge);
            let out = distance.max(0.0);
            let (bloom, halo) = (lit * (-out / LIQUID_BLOOM).exp() * 0.9, (-out / LIQUID_HALO).exp() * 0.4);
            let around = (bloom + halo).min(1.0);
            let alpha = cover + (1.0 - cover) * around;
            let channel = |index: usize| {
                let lights = (white[index] * bloom + glow[index] * halo).min(255.0 * around);
                (inside[index] * cover + lights * (1.0 - cover)).round().clamp(0.0, 255.0) as u8
            };
            *pixel = Color32::from_rgba_premultiplied(channel(0), channel(1), channel(2), (alpha * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    ColorImage::new(size, pixels)
}

/// A rounded box of liquid whose corners are rounded by `corner`; a [`Blob`]'s ends are always fully round.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slab {
    /// Its middle.
    pub centre: [f32; 2],
    /// Half its width and half its height.
    pub half: [f32; 2],
    /// How far its corners are rounded.
    pub corner: f32,
}

impl Slab {
    /// How far the point `at` is from the shape's edge: negative inside it. A slab of no size is nowhere.
    fn distance(&self, at: [f32; 2]) -> f32 {
        if self.half[0] <= 0.0 || self.half[1] <= 0.0 {
            return f32::INFINITY;
        }
        let corner = self.corner.clamp(0.0, self.half[0].min(self.half[1]));
        let (qx, qy) = ((at[0] - self.centre[0]).abs() - (self.half[0] - corner), (at[1] - self.centre[1]).abs() - (self.half[1] - corner));
        qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - corner
    }
}

/// The picture of `slabs` in one colour, `colors[0]`: the part that starts at `origin` and is `size` texels
/// large, with `scale` texels to a point. Slabs closer than `melt` grow together. A thin rim along the edge is
/// lit in `colors[1]` near `light`, which is in points from the left like `origin`.
// Human: the plain cousin of [`picture`], for liquid that is part of the interface and has its colours: no
// glow and no bloom, only the body and one light on its rim.
// Agent: PURE; costs one distance for each texel and slab.
#[must_use]
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "colour channels, kept between 0 and 255")]
#[expect(clippy::cast_precision_loss, reason = "texel positions, far below where f32 loses whole numbers")]
pub fn plain(slabs: &[Slab], melt: f32, origin: [f32; 2], size: [usize; 2], scale: f32, colors: [Color32; 2], light: f32) -> ColorImage {
    let (body, lit) = (rgb(colors[0]), rgb(colors[1]));
    let [rim, reach] = TAB_POUR_RIM;
    let mut pixels = vec![Color32::TRANSPARENT; size[0] * size[1]];
    for (row, line) in pixels.chunks_mut(size[0].max(1)).enumerate() {
        let y = origin[1] + (row as f32 + 0.5) / scale;
        for (column, pixel) in line.iter_mut().enumerate() {
            let at = [origin[0] + (column as f32 + 0.5) / scale, y];
            let mut distance = f32::INFINITY;
            for slab in slabs {
                let other = slab.distance(at);
                let share = if melt > 0.0 { (melt - (distance - other).abs()).max(0.0) / melt } else { 0.0 };
                distance = distance.min(other) - share * share * melt * 0.25;
            }
            let cover = (0.5 - distance * scale).clamp(0.0, 1.0);
            if cover <= 0.0 {
                continue;
            }
            let edge = 1.0 - smooth(rim - 1.0, rim, -distance);
            let glint = edge * (1.0 - smooth(0.0, reach, (at[0] - light).abs())) * 0.4;
            let color = blend(body, lit, glint);
            let channel = |index: usize| (color[index] * cover).round().clamp(0.0, 255.0) as u8;
            *pixel = Color32::from_rgba_premultiplied(channel(0), channel(1), channel(2), (cover * 255.0).round().clamp(0.0, 255.0) as u8);
        }
    }
    ColorImage::new(size, pixels)
}

/// Where the rows of a task list are, for the liquid to pour them: in points, from the top left of the list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rows {
    /// How wide the list is.
    pub width: f32,
    /// The left and right end of a row.
    pub span: [f32; 2],
    /// Where the first row's top is.
    pub top: f32,
    /// How high a row is.
    pub height: f32,
}

impl Rows {
    /// The middle of the row `index`, top to bottom.
    #[expect(clippy::cast_precision_loss, reason = "a list has at most twelve rows")]
    fn middle(&self, index: usize) -> f32 {
        self.top + (index as f32 + 0.5) * self.height
    }
}

/// A number from 0 to 1 that follows from `state`, which it moves on: the same start gives the same numbers.
// Source: SplitMix64, Sebastiano Vigna (prng.di.unimi.it/splitmix64.c).
#[expect(clippy::cast_precision_loss, reason = "24 bits, which f32 holds exactly")]
fn chance(state: &mut u64) -> f32 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut mixed = *state;
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((mixed ^ (mixed >> 31)) >> 40) as f32 / (1_u64 << 24) as f32
}

/// The value at `share` on a line through `points`, each a place from 0 to 1 and the value there.
fn along(points: &[(f32, f32)], share: f32) -> f32 {
    let mut before = points.first().copied().unwrap_or_default();
    for &point in points {
        if share <= point.0 {
            let span = (point.0 - before.0).max(f32::EPSILON);
            return mix(before.1, point.1, ((share - before.0) / span).clamp(0.0, 1.0));
        }
        before = point;
    }
    before.1
}

/// One drop of a pour: the task it makes, and how it falls.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Drip {
    /// When it lets go of the pool, seconds into the pour.
    lets_go: f32,
    /// Where it falls, from the left.
    x: f32,
    /// How wide it is.
    size: f32,
    /// How long it falls.
    fall: f32,
}

impl Drip {
    /// When it lands on its row.
    fn lands(&self) -> f32 {
        self.lets_go + TASKS_POUR_LAND * self.fall
    }
}

/// A bulge on the underside of the pool.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Lobe {
    x: f32,
    size: f32,
    /// How fast it bobs, and where in its bob it starts.
    pace: f32,
    phase: f32,
}

/// One pouring in of a task list: where and when each task's drop falls. No two pours are alike.
#[derive(Clone, Debug, PartialEq)]
pub struct Pour {
    rows: Rows,
    /// The row the first drop makes: 0 for a new list, further down for tasks added to one.
    first: usize,
    drips: Vec<Drip>,
    lobes: Vec<Lobe>,
    /// When the last row has poured out and the pool draws back, seconds into the pour.
    drained: f32,
}

impl Pour {
    /// A pour of `tasks` rows from the row `first` on, laid out as `rows` says, with the places and paces that
    /// follow from `seed`.
    // Human: each drop falls from another place along the pool, never close to the one before it, and each
    // has its own size, its own speed and its own wait after the drop before.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "a list has at most twelve rows")]
    pub fn new(seed: u64, tasks: usize, first: usize, rows: Rows) -> Pour {
        let mut state = seed;
        let lobes = (0..5_u8)
            .map(|index| Lobe {
                x: rows.width * (0.08 + f32::from(index) * 0.21 + chance(&mut state) * 0.14),
                size: 26.0 + chance(&mut state) * 14.0,
                pace: std::f32::consts::TAU / (0.84 + chance(&mut state) * 0.64),
                phase: chance(&mut state) * std::f32::consts::TAU,
            })
            .collect();
        let (from, room) = (rows.span[0] + 6.0, (rows.span[1] - rows.span[0] - 38.0).max(1.0));
        let (mut lets_go, mut before, mut drained) = (TASKS_POOL, f32::NEG_INFINITY, 0.0_f32);
        let mut drips = Vec::with_capacity(tasks);
        for index in 0..tasks {
            let mut x = from + chance(&mut state) * room;
            for _ in 0..8 {
                if (x - before).abs() >= room * 0.22 {
                    break;
                }
                x = from + chance(&mut state) * room;
            }
            before = x;
            let drip = Drip { lets_go, x, size: 18.0 + chance(&mut state) * 9.0, fall: 0.15 + index as f32 * 0.022 + chance(&mut state) * 0.07 };
            drained = drained.max(drip.lands() + TASKS_POUR_SOAK);
            lets_go += TASKS_POUR_GAP + chance(&mut state) * TASKS_POUR_GAP_MORE;
            drips.push(drip);
        }
        Pour { rows, first, drips, lobes, drained }
    }

    /// The row the first drop makes.
    #[must_use]
    pub fn first(&self) -> usize {
        self.first
    }

    /// When nothing of the pour is left to see, seconds into it.
    #[must_use]
    pub fn over(&self) -> f32 {
        self.drained + 0.45
    }

    /// When the list stands and the first task is shown at work.
    #[must_use]
    pub fn settled(&self) -> f32 {
        self.drained + 0.24
    }

    /// How much of the row `index` shows `time` seconds into the pour: nothing until its drop has landed and
    /// its liquid begins to soak away, then all of it.
    #[must_use]
    pub fn shown(&self, index: usize, time: f32) -> f32 {
        let drip = index.checked_sub(self.first).and_then(|place| self.drips.get(place));
        drip.map_or(1.0, |drip| ((time - drip.lands() - 0.23) / 0.2).clamp(0.0, 1.0))
    }

    /// How far down the pool hangs under the list's top edge at `time`: it fills with a little too much,
    /// settles, and draws back when the last row is poured.
    fn pool(&self, time: f32) -> f32 {
        let filled = along(&[(0.0, 0.0), (0.55, 1.2), (0.78, 0.94), (1.0, 1.0)], ((time - 0.04) / 0.5).clamp(0.0, 1.0));
        let left = 1.0 - smooth(0.0, 0.3, time - self.drained);
        TASKS_POUR_BAND * filled * left
    }

    /// The shapes of the liquid `time` seconds into the pour.
    #[must_use]
    pub fn blobs(&self, time: f32) -> Vec<Blob> {
        let rows = self.rows;
        let depth = self.pool(time);
        let full = (depth / TASKS_POUR_BAND).clamp(0.0, 1.0);
        let mut blobs = Vec::new();
        if depth > 0.5 {
            // The pool hangs from above the list's top edge, so its upper half is never seen.
            let above = 40.0;
            blobs.push(Blob { centre: [rows.width / 2.0, (depth - above) / 2.0], half: [rows.width / 2.0 + 30.0, f32::midpoint(depth, above)] });
            for lobe in &self.lobes {
                let bob = (time * lobe.pace + lobe.phase).sin() * 5.5;
                blobs.push(Blob::round(lobe.x, depth - lobe.size * 0.32 + bob, lobe.size * 0.5 * full));
            }
        }
        for (index, drip) in self.drips.iter().enumerate() {
            let row = rows.middle(self.first + index);
            // Human: before a drop lets go, the pool swells there and stretches down; then it snaps back.
            let swell = (time - (drip.lets_go - 0.25)) / 0.34;
            if (0.0..1.0).contains(&swell) && depth > 0.5 {
                let wide = along(&[(0.0, 0.0), (0.35, 1.0), (0.7, 0.9), (0.85, 0.6), (1.0, 0.5)], swell);
                let long = along(&[(0.0, 0.0), (0.35, 1.0), (0.7, 1.7), (0.85, 1.8), (1.0, 0.4)], swell);
                let down = along(&[(0.0, 0.0), (0.35, 4.0), (0.7, 14.0), (0.85, 18.0), (1.0, 0.0)], swell);
                let top = depth - 16.0 + down;
                blobs.push(Blob { centre: [drip.x + 11.0, top + 18.0 * long], half: [16.0 * wide, 18.0 * long] });
            }
            // The drop: long while it falls, flat for a moment where it lands.
            let fallen = (time - drip.lets_go) / drip.fall;
            if (0.0..1.0).contains(&fallen) {
                let way = (fallen / TASKS_POUR_LAND).clamp(0.0, 1.0);
                let y = mix(depth, row, way * way);
                let (wide, long) = if fallen < TASKS_POUR_LAND { (0.7, 1.9) } else { (1.5, 0.55) };
                let gone = 1.0 - smooth(0.92, 1.0, fallen);
                blobs.push(Blob { centre: [drip.x + 11.0, y], half: [drip.size * 0.5 * wide * gone, (drip.size * 0.5 + 1.0) * long * gone] });
            }
            // The row: it pours out from where the drop landed to both sides, sloshes, and soaks away.
            let poured = (time - drip.lands()) / TASKS_POUR_SOAK;
            if (0.0..1.0).contains(&poured) {
                let spread = (poured / 0.58).clamp(0.0, 1.0);
                let (left, right) = (mix(drip.x, rows.span[0], spread), mix(drip.x + 26.0, rows.span[1], spread));
                let high = along(&[(0.0, 1.0), (0.25, 1.15), (0.42, 0.92), (0.58, 1.12), (0.72, 0.95), (1.0, 0.0)], poured) * 11.0;
                if high > 0.4 {
                    blobs.push(Blob { centre: [f32::midpoint(left, right), row], half: [(right - left) / 2.0, high] });
                }
                // A wave runs ahead of each end while the row spreads.
                let wave = along(&[(0.0, 15.0), (0.35, 13.0), (0.58, 5.0), (0.7, 0.0)], poured);
                if wave > 0.4 {
                    blobs.push(Blob::round(left + 8.0, row, wave));
                    blobs.push(Blob::round(right - 8.0, row, wave));
                }
            }
            // Two droplets fly out to the sides where it lands.
            let flown = (time - drip.lands()) / 0.4;
            if (0.0..1.0).contains(&flown) {
                let out = along(&[(0.0, 0.0), (0.4, 1.0), (1.0, 0.4)], flown);
                let up = along(&[(0.0, 0.0), (0.4, -11.0), (1.0, 2.0)], flown);
                let size = along(&[(0.0, 0.0), (0.4, 4.0), (1.0, 0.0)], flown);
                for side in [-20.0, 20.0].into_iter().filter(|_| size > 0.4) {
                    blobs.push(Blob::round(drip.x + 13.0 + side * out, row + up, size));
                }
            }
        }
        blobs
    }
}

/// The shapes of the small drop that runs from a finished task's mark at `from` to the next task's mark, one
/// row of `row` points below it, `share` of the way through its run (0 to 1): it stretches down, lets go, lands
/// flat and is gone, with two droplets flying out where it lands.
#[must_use]
pub fn step(from: [f32; 2], row: f32, share: f32) -> Vec<Blob> {
    let mut blobs = Vec::new();
    let long = along(&[(0.0, 10.0), (0.4, 25.0), (0.75, 10.0), (1.0, 10.0)], share);
    let down = along(&[(0.0, 0.0), (0.4, 15.0), (0.75, row), (1.0, row)], share);
    let wide = along(&[(0.0, 10.0), (0.4, 8.0), (0.75, 14.0), (0.9, 9.0), (1.0, 0.0)], share);
    let flat = along(&[(0.0, 1.0), (0.4, 1.0), (0.75, 0.7), (0.9, 1.0), (1.0, 0.0)], share);
    if wide > 0.4 {
        blobs.push(Blob { centre: [from[0], from[1] + down], half: [wide, long * flat] });
    }
    let flown = (share - 0.66) / 0.34;
    if (0.0..1.0).contains(&flown) {
        let out = along(&[(0.0, 0.0), (0.4, 1.0), (1.0, 0.4)], flown);
        let up = along(&[(0.0, 0.0), (0.4, -9.0), (1.0, 2.0)], flown);
        let size = along(&[(0.0, 0.0), (0.4, 3.5), (1.0, 0.0)], flown);
        for side in [-14.0, 18.0].into_iter().filter(|_| size > 0.4) {
            blobs.push(Blob::round(from[0] + side * out, from[1] + row + up, size));
        }
    }
    blobs
}

/// How long a finished list takes to move into the conversation ([`hand`]), in seconds.
pub const HAND_TIME: f32 = TASKS_HAND_MELT + TASKS_HAND_FALL + TASKS_HAND_SOAK;
/// How far into that move the note it becomes begins to show.
pub const HAND_SHOWS: f32 = TASKS_HAND_MELT + TASKS_HAND_FALL + 0.2;

/// The shapes of a finished list on its way into the conversation, `time` seconds into the move. Its line,
/// which lies between the ends `span` at the height of `from`, melts into one drop at `from`; the drop falls
/// to the height `to[2]`, where the list is filed, and pours out there between `to[0]` and `to[1]` before it
/// soaks away.
#[must_use]
pub fn hand(from: [f32; 2], span: [f32; 2], to: [f32; 3], time: f32) -> Vec<Blob> {
    let mut blobs = Vec::new();
    let melted = time / TASKS_HAND_MELT;
    if (0.0..1.0).contains(&melted) {
        let right = mix(span[1], from[0] + 13.0, smooth(0.15, 1.0, melted));
        let high = along(&[(0.0, 0.0), (0.2, 12.0), (1.0, 11.0)], melted);
        if high > 0.4 {
            blobs.push(Blob { centre: [f32::midpoint(span[0], right), from[1]], half: [(right - span[0]) / 2.0, high] });
        }
    }
    let fallen = (time - TASKS_HAND_MELT) / TASKS_HAND_FALL;
    if (0.0..1.0).contains(&fallen) {
        blobs.push(Blob { centre: [from[0], mix(from[1], to[2], fallen * fallen)], half: [8.0, 22.0] });
    }
    let landed = time - TASKS_HAND_MELT - TASKS_HAND_FALL;
    let poured = landed / TASKS_HAND_SOAK;
    if (0.0..1.0).contains(&poured) {
        let spread = (poured / 0.5).clamp(0.0, 1.0);
        let (left, right) = (mix(from[0] - 13.0, to[0], spread), mix(from[0] + 13.0, to[1], spread));
        let high = along(&[(0.0, 1.0), (0.25, 1.15), (0.4, 0.92), (0.5, 1.1), (0.65, 0.95), (1.0, 0.0)], poured) * 11.0;
        if high > 0.4 {
            blobs.push(Blob { centre: [f32::midpoint(left, right), to[2]], half: [(right - left) / 2.0, high] });
        }
    }
    let flown = landed / 0.4;
    if (0.0..1.0).contains(&flown) {
        let out = along(&[(0.0, 0.0), (0.4, 1.0), (1.0, 0.4)], flown);
        let up = along(&[(0.0, 0.0), (0.4, -11.0), (1.0, 2.0)], flown);
        let size = along(&[(0.0, 0.0), (0.4, 4.0), (1.0, 0.0)], flown);
        for side in [-20.0, 20.0].into_iter().filter(|_| size > 0.4) {
            blobs.push(Blob::round(from[0] + side * out, to[2] + up, size));
        }
    }
    blobs
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROWS: Rows = Rows { width: 360.0, span: [9.0, 351.0], top: 48.0, height: 28.0 };

    fn alpha(image: &ColorImage, x: usize, y: usize) -> u8 {
        image.pixels[y * image.size[0] + x].a()
    }

    #[test]
    fn shapes_that_are_close_grow_together() {
        let (one, other) = (Blob::round(20.0, 20.0, 8.0), Blob::round(40.0, 20.0, 8.0));
        // Between two circles four points apart there is liquid, though neither reaches the middle.
        let middle = [30.0, 20.0];
        assert!(one.distance(middle) > 0.0 && other.distance(middle) > 0.0);
        assert!(melted(one.distance(middle), other.distance(middle)) < 0.0);
        // Far apart, each is only itself.
        let far = Blob::round(120.0, 20.0, 8.0);
        assert!((melted(one.distance(middle), far.distance(middle)) - one.distance(middle)).abs() < 1e-4);
        // A long shape is a capsule: as far from its side as from its end.
        let capsule = Blob { centre: [50.0, 50.0], half: [30.0, 10.0] };
        assert!((capsule.distance([50.0, 65.0]) - 5.0).abs() < 1e-4 && (capsule.distance([85.0, 50.0]) - 5.0).abs() < 1e-4);
    }

    #[test]
    fn the_picture_has_a_body_a_lit_rim_and_a_glow_that_fades() {
        let blobs = [Blob::round(30.0, 30.0, 14.0)];
        let image = picture(&blobs, [0.0, 0.0], [60, 60], 1.0, 30.0, 60.0);
        let at = |x: usize, y: usize| image.pixels[y * 60 + x];
        // The middle is the body's deeper colour, opaque.
        assert_eq!(at(30, 30), LIQUID_CORE);
        // Under the light the rim is nearly white; the glow beside it is seen and fades to nothing.
        let rim = at(30, 16);
        assert!(rim.a() == 255 && rim.r() > 240 && rim.g() > 225 && rim.b() > 200, "{rim:?}");
        assert!(alpha(&image, 30, 12) > alpha(&image, 30, 6) && alpha(&image, 30, 6) > 0);
        assert_eq!(at(1, 1), Color32::TRANSPARENT);
        // Away from the light the rim keeps the liquid's own lighter orange.
        let dark = picture(&blobs, [0.0, 0.0], [60, 60], 1.0, -200.0, 60.0);
        let quiet = dark.pixels[16 * 60 + 30];
        assert!(quiet.b() < 140 && quiet.r() > 240, "{quiet:?}");
        // A part of the picture is that part: the same texels, wherever it starts.
        let part = picture(&blobs, [20.0, 20.0], [20, 20], 1.0, 30.0, 60.0);
        assert_eq!(part.pixels[10 * 20 + 10], at(30, 30));
        // More texels to a point give a larger picture of the same thing.
        let fine = picture(&blobs, [0.0, 0.0], [120, 120], 2.0, 30.0, 60.0);
        assert_eq!(fine.pixels[60 * 120 + 60], LIQUID_CORE);
    }

    #[test]
    fn a_pour_drops_one_drop_for_each_task_from_places_of_its_own() {
        let pour = Pour::new(7, 6, 0, ROWS);
        assert_eq!(pour.drips.len(), 6);
        // Each lets go after the one before, falls within the rows, and not close to where the last one fell.
        for pair in pour.drips.windows(2) {
            assert!(pair[1].lets_go > pair[0].lets_go);
            assert!((pair[1].x - pair[0].x).abs() >= (ROWS.span[1] - ROWS.span[0] - 38.0) * 0.22, "{pair:?}");
        }
        assert!(pour.drips.iter().all(|drip| drip.x >= ROWS.span[0] && drip.x + 26.0 <= ROWS.span[1]));
        // Another pour falls elsewhere; the same one again falls the same.
        assert_ne!(Pour::new(8, 6, 0, ROWS).drips, pour.drips);
        assert_eq!(Pour::new(7, 6, 0, ROWS), pour);
    }

    #[test]
    fn a_pour_pools_then_makes_its_rows_and_leaves_nothing_behind() {
        let pour = Pour::new(3, 4, 0, ROWS);
        assert_eq!(pour.blobs(0.0), []);
        // While it pools there is liquid at the top and no row yet.
        let pooled = pour.blobs(0.6);
        assert!(pooled.iter().any(|blob| blob.half[0] > ROWS.width / 2.0));
        assert!((0..4).all(|row| pour.shown(row, 0.6) <= 0.0));
        // A row's liquid lies on its row once its drop has landed, and reaches both ends before it soaks away.
        let landed = pour.drips[0].lands();
        let spread = pour.blobs(landed + TASKS_POUR_SOAK * 0.6);
        let row = spread.iter().find(|blob| (blob.centre[1] - ROWS.middle(0)).abs() < 0.01 && blob.half[0] > 100.0).expect("the first row's liquid");
        assert!((row.centre[0] - row.half[0] - ROWS.span[0]).abs() < 0.5 && (row.centre[0] + row.half[0] - ROWS.span[1]).abs() < 0.5);
        // The rows show in their order, each after its drop.
        assert!(pour.shown(0, landed + 0.5) >= 1.0 && pour.shown(3, landed + 0.2) <= 0.0);
        assert!(pour.settled() < pour.over());
        assert!(pour.blobs(pour.over()).is_empty() && (0..4).all(|row| pour.shown(row, pour.over()) >= 1.0));
    }

    #[test]
    fn tasks_added_to_a_list_pour_in_at_their_rows_and_leave_the_others_be() {
        let pour = Pour::new(5, 2, 3, ROWS);
        assert_eq!((pour.first(), pour.drips.len()), (3, 2));
        // The rows that were there show all along; the new ones after their drops.
        assert!(pour.shown(0, 0.0) >= 1.0 && pour.shown(2, 0.0) >= 1.0 && pour.shown(5, 0.0) >= 1.0);
        assert!(pour.shown(3, 0.0) <= 0.0 && pour.shown(4, 0.0) <= 0.0 && pour.shown(4, pour.over()) >= 1.0);
        // The first new row's liquid lies on the fourth row.
        let spread = pour.blobs(pour.drips[0].lands() + TASKS_POUR_SOAK * 0.6);
        assert!(spread.iter().any(|blob| (blob.centre[1] - ROWS.middle(3)).abs() < 0.01 && blob.half[0] > 100.0));
        assert!(!spread.iter().any(|blob| (blob.centre[1] - ROWS.middle(0)).abs() < 0.01 && blob.half[0] > 100.0));
    }

    #[test]
    fn a_finished_list_melts_falls_and_pours_out_where_it_is_filed() {
        let (from, span, to) = ([22.0, 19.0], [9.0, 351.0], [20.0, 220.0, 400.0]);
        assert_eq!(hand(from, span, to, 0.0), []);
        // First its line lies across the list, then it has drawn together at its left end.
        let lying = hand(from, span, to, TASKS_HAND_MELT * 0.1);
        assert!(lying.len() == 1 && lying[0].half[0] > 150.0);
        let drawn = hand(from, span, to, TASKS_HAND_MELT * 0.98);
        assert!(drawn[0].half[0] < 20.0);
        // The drop is on its way between the two places, then the liquid lies where the list is filed.
        let falling = hand(from, span, to, TASKS_HAND_MELT + TASKS_HAND_FALL * 0.7);
        assert!(falling.iter().any(|blob| blob.centre[1] > 100.0 && blob.centre[1] < 399.0));
        let there = hand(from, span, to, TASKS_HAND_MELT + TASKS_HAND_FALL + TASKS_HAND_SOAK * 0.55);
        let filed = there.iter().find(|blob| blob.half[0] > 50.0).expect("the liquid where the list is filed");
        assert!((filed.centre[1] - 400.0).abs() < 0.01 && (filed.centre[0] - filed.half[0] - 20.0).abs() < 0.5);
        assert_eq!(hand(from, span, to, HAND_TIME), []);
    }

    #[test]
    fn the_drop_between_two_tasks_runs_down_one_row_and_is_gone() {
        let start = step([10.0, 100.0], 28.0, 0.0);
        assert_eq!(start, vec![Blob::round(10.0, 100.0, 10.0)]);
        let landed = step([10.0, 100.0], 28.0, 0.8);
        assert!(landed.iter().any(|blob| (blob.centre[1] - 128.0).abs() < 0.01));
        assert_eq!(step([10.0, 100.0], 28.0, 1.0), []);
    }

    #[test]
    fn the_light_crosses_the_liquid_and_comes_in_again() {
        assert!(light_at(0.0, 300.0) < 0.0);
        assert!(light_at(LIQUID_LIGHT_PACE * 0.5, 300.0) > 100.0 && light_at(LIQUID_LIGHT_PACE * 0.5, 300.0) < 200.0);
        assert!(light_at(LIQUID_LIGHT_PACE * 0.99, 300.0) > 300.0);
        assert!((light_at(LIQUID_LIGHT_PACE * 1.25, 300.0) - light_at(LIQUID_LIGHT_PACE * 0.25, 300.0)).abs() < 0.01);
    }
}
