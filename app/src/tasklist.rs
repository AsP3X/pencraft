//! The task list at the top of the chat panel: the tasks of a long job, and how far the assistant has got.
//!
//! The list ([`crate::chat::tasks`]) stands under the panel's header, above the conversation, and stays there
//! while the conversation scrolls; no line parts it from either, the conversation fades out under it. A finished task has a tick, the task at work a small blob of liquid that
//! breathes, with a frame a light runs around; the tasks still to come wait with an empty ring. A bar shows how
//! far the list is. A click on the header folds the list to one line and opens it again.
//!
//! A new list pours in ([`crate::liquid`]): liquid pools under the header and one drop falls for each task.
//! When a task is finished its mark bursts into a tick and a drop runs on to the next task. Tasks added to a
//! list pour in at their rows. A list that is worked through leaves: its line melts into one drop, which falls
//! into the conversation and pours out there as the note the list is filed as ([`filed`]). With "Animations &
//! effects" off in the settings the list simply shows what is so.

use std::sync::Arc;

use egui::text::{LayoutJob, TextWrapping};
use egui::{Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2, pos2, vec2};

use crate::chat::tasks::{State, Tasks};
use crate::chat::{Chat, Entry};
use crate::icons;
use crate::liquid::{self, Pour, Rows};
use crate::theme::{
    LIQUID, LIQUID_FINE, LIQUID_GLOW, LIQUID_REACH, LIQUID_RIM, PROBLEM, Palette, SIDEBAR_TEXT_SIZE, TASKS_BAR, TASKS_BAR_GAP, TASKS_BAR_TIME,
    TASKS_BLOB_PULSE, TASKS_BLOB_TURN, TASKS_BLOB_WARP, TASKS_BURST, TASKS_FLARE, TASKS_FRAME_TURN, TASKS_GLIDE, TASKS_HAND_MELT, TASKS_HEAD,
    TASKS_MARK, TASKS_MARK_GAP, TASKS_PAD, TASKS_ROW, TASKS_ROW_REACH, TASKS_ROWS_GAP, TASKS_STEP_NEXT, TASKS_STEP_TIME,
};

/// The icon before the word "Tasks".
const LIST_ICON: &str = "list-checks";
/// The tick of a finished task.
const TICK_ICON: &str = "check";
/// The mark of a task whose run was stopped or broke off.
const HALTED_ICON: &str = "circle-x";
/// The arrows that fold the list and open it.
const FOLD_ICON: &str = "chevron-up";
/// See [`FOLD_ICON`].
const OPEN_ICON: &str = "chevron-down";
/// How many points the outline of the blob has.
const BLOB_POINTS: u8 = 40;

/// What the panel remembers between frames to move the list: it lives in egui's memory, not in the chat.
#[derive(Clone, Default)]
struct Motion {
    /// The panel has shown a list of this chat before.
    seen: bool,
    /// The list it showed last ([`Tasks::made`]).
    made: u64,
    /// When that list first showed, in egui's time; long ago for a list that simply appeared.
    born: f64,
    /// How the list pours in, while it does.
    pour: Option<Arc<Pour>>,
    /// How many tasks were finished when the list was last drawn, and since when that many are.
    done: usize,
    stepped: f64,
    /// Folded to one line or open, once the user has chosen.
    folded: Option<bool>,
    /// How high the list is drawn right now.
    height: f32,
    /// The picture of the liquid, while there is some.
    picture: Option<TextureHandle>,
    /// The list has poured in: it doesn't again.
    poured: bool,
    /// The tasks added to the list last, which pour in: where the first stands and how many they are; and the
    /// number of the addition the panel has seen ([`Tasks::added`]).
    fresh: Option<(usize, usize)>,
    grown: u64,
    /// How many of this chat's lists the panel has seen filed into the conversation, and when the last one left.
    handed: u64,
    left: f64,
}

/// Where the motion of the list of the chat `key` is kept.
fn motion_id(key: u64) -> egui::Id {
    egui::Id::unique(("tasklist-motion", key))
}

/// Where the conversation last drew the note of a filed list: a list that leaves the top of the panel lands there.
fn landing_id() -> egui::Id {
    egui::Id::unique("tasklist-landing")
}

/// How high the list is with `tasks` tasks: open, folded to one line, or folded with every task finished.
#[expect(clippy::cast_precision_loss, reason = "a list has at most twelve tasks")]
fn tall(tasks: usize, folded: bool, finished: bool) -> f32 {
    let head = TASKS_PAD + TASKS_HEAD;
    match (folded, finished) {
        (true, true) => head + TASKS_PAD,
        (true, false) => head + TASKS_BAR_GAP + TASKS_BAR + TASKS_PAD,
        (false, _) => head + TASKS_BAR_GAP + TASKS_BAR + TASKS_ROWS_GAP + TASKS_ROW * tasks as f32 + TASKS_PAD,
    }
}

/// The colour `share` of the way from `from` to `to`.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "colour channels, kept between 0 and 255")]
pub fn tint(from: Color32, to: Color32, share: f32) -> Color32 {
    let share = share.clamp(0.0, 1.0);
    let channel = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * share).round() as u8;
    Color32::from_rgba_premultiplied(channel(from.r(), to.r()), channel(from.g(), to.g()), channel(from.b(), to.b()), channel(from.a(), to.a()))
}

/// 0 below `from`, 1 above `to`, and a smooth rise between.
pub fn smooth(from: f32, to: f32, value: f32) -> f32 {
    let share = ((value - from) / (to - from)).clamp(0.0, 1.0);
    share * share * (3.0 - 2.0 * share)
}

/// The seconds from `since` to `now` in egui's time, as the small number an animation counts with.
#[expect(clippy::cast_possible_truncation, reason = "seconds of an animation; a long time ago is simply large")]
fn seconds(now: f64, since: f64) -> f32 {
    (now - since).min(1e6) as f32
}

/// `time` as the share of a turn that takes `turn` seconds, from 0 to 1.
#[expect(clippy::cast_possible_truncation, reason = "a share between 0 and 1")]
pub fn turned(time: f64, turn: f32) -> f32 {
    (time / f64::from(turn)).rem_euclid(1.0) as f32
}

/// `text` on one line in the panel's small type, cut short with an ellipsis where it is wider than `width`.
fn words(ui: &Ui, text: &str, color: Color32, width: f32) -> Arc<egui::Galley> {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), FontId::proportional(SIDEBAR_TEXT_SIZE), color);
    job.wrap = TextWrapping::truncate_at_width(width.max(0.0));
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

/// The empty ring of a task that waits, or of the task at work while nothing works on it.
fn ring(painter: &egui::Painter, slot: Rect, color: Color32) {
    painter.circle_stroke(slot.center(), slot.width() / 2.0 - 0.75, Stroke::new(1.5, color));
}

/// The tick of a finished task. `burst` is how far it is through bursting into a tick (0 to 1), for the task
/// that was just finished: it flashes white, a ring widens around it, and the tick pops up.
fn ticked(painter: &egui::Painter, slot: Rect, palette: Palette, burst: Option<f32>, opacity: f32) {
    let (centre, radius) = (slot.center(), slot.width() / 2.0);
    let mut fill = palette.hover;
    let mut size = 10.0;
    if let Some(share) = burst {
        fill = if share < 0.35 { tint(Color32::WHITE, LIQUID_RIM, share / 0.35) } else { tint(LIQUID_RIM, palette.hover, (share - 0.35) / 0.65) };
        let widened = smooth(0.0, 0.6, share);
        painter.circle_stroke(centre, radius + 10.0 * widened, Stroke::new(2.0, LIQUID_GLOW.gamma_multiply((1.0 - widened).powi(2))));
        // The usual "back" ease-out: the tick grows a little past its size and settles.
        let (back, t) = (1.70158_f32, (share / 0.5).min(1.0) - 1.0);
        size *= 1.0 + (back + 1.0) * t.powi(3) + back * t.powi(2);
    }
    painter.circle_filled(centre, radius, fill.gamma_multiply(opacity));
    icons::paint(painter, Rect::from_center_size(centre, Vec2::splat(size.max(0.0))), TICK_ICON, palette.foreground.gamma_multiply(opacity));
}

/// The blob of the task at work, around `centre`: a drop of the liquid that swells and shrinks, slowly goes
/// through shapes that are not quite round, and has a light running around its rim. Without `time`
/// ("Animations & effects" is off) it is a still, round drop.
pub fn blob(painter: &egui::Painter, centre: Pos2, radius: f32, time: Option<f64>) {
    let pulse = time.map_or(1.0, |time| 0.95 + 0.13 * (turned(time, TASKS_BLOB_PULSE) * std::f32::consts::TAU).sin());
    let warp = time.map_or(0.0, |time| turned(time, TASKS_BLOB_WARP) * std::f32::consts::TAU);
    let give = if time.is_some() { 1.0 } else { 0.0 };
    let angle = |point: u8| f32::from(point) / f32::from(BLOB_POINTS) * std::f32::consts::TAU;
    let at = |point: u8| {
        let angle = angle(point);
        let out = radius * pulse * (1.0 + give * (0.07 * (2.0 * angle + warp).sin() + 0.05 * (3.0 * angle - 1.7 * warp).sin()));
        centre + out * vec2(angle.cos(), angle.sin())
    };
    for (reach, strength) in [(1.9, 0.07), (1.5, 0.12), (1.2, 0.2)] {
        painter.circle_filled(centre, radius * pulse * reach, LIQUID_GLOW.gamma_multiply(strength));
    }
    painter.add(Shape::convex_polygon((0..BLOB_POINTS).map(at).collect(), LIQUID, Stroke::NONE));
    painter.circle_filled(centre + radius * pulse * vec2(-0.28, -0.3), radius * pulse * 0.42, LIQUID_RIM.gamma_multiply(0.6));
    let head = time.map(|time| turned(time, TASKS_BLOB_TURN) * std::f32::consts::TAU);
    for point in 0..BLOB_POINTS {
        let lit = head.map_or(0.0, |head| {
            let apart = (angle(point) - head).rem_euclid(std::f32::consts::TAU);
            1.0 - smooth(0.0, 1.4, apart.min(std::f32::consts::TAU - apart))
        });
        painter.line_segment([at(point), at((point + 1) % BLOB_POINTS)], Stroke::new(1.3, tint(LIQUID_RIM, Color32::WHITE, lit)));
    }
}

/// The points of the outline of `rect` with corners of `radius`, clockwise from the top left corner's end, a
/// few points apart, so a light can be moved along it.
fn outline(rect: Rect, radius: f32) -> Vec<Pos2> {
    const CORNER: u8 = 6;
    const APART: f32 = 8.0;
    let radius = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let mut points = Vec::new();
    let side = |from: Pos2, to: Pos2, points: &mut Vec<Pos2>| {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a handful of pieces along one side")]
        let pieces = ((to - from).length() / APART).ceil().max(1.0) as u16;
        points.extend((0..pieces).map(|piece| from + (to - from) * (f32::from(piece) / f32::from(pieces))));
    };
    let corner = |centre: Pos2, from: f32, points: &mut Vec<Pos2>| {
        points.extend((0..CORNER).map(|piece| {
            let angle = from + f32::from(piece) / f32::from(CORNER) * std::f32::consts::FRAC_PI_2;
            centre + radius * vec2(angle.cos(), angle.sin())
        }));
    };
    let (left, right, top, bottom) = (rect.left() + radius, rect.right() - radius, rect.top() + radius, rect.bottom() - radius);
    side(pos2(left, rect.top()), pos2(right, rect.top()), &mut points);
    corner(pos2(right, top), -std::f32::consts::FRAC_PI_2, &mut points);
    side(pos2(rect.right(), top), pos2(rect.right(), bottom), &mut points);
    corner(pos2(right, bottom), 0.0, &mut points);
    side(pos2(right, rect.bottom()), pos2(left, rect.bottom()), &mut points);
    corner(pos2(left, bottom), std::f32::consts::FRAC_PI_2, &mut points);
    side(pos2(rect.left(), bottom), pos2(rect.left(), top), &mut points);
    corner(pos2(left, top), std::f32::consts::PI, &mut points);
    points
}

/// The frame of the task at work: a faint tint of the liquid behind the row and a thin rim, along which a
/// light runs. `flare` is how much of the flash is left that the frame makes when its task comes to be at work
/// (1 to 0). Without `time` the rim is still.
fn frame(painter: &egui::Painter, rect: Rect, palette: Palette, time: Option<f64>, flare: f32) {
    const RADIUS: f32 = 10.0;
    painter.rect_filled(rect, RADIUS, tint(palette.island, LIQUID, 0.07));
    let Some(time) = time else {
        painter.rect_stroke(rect, RADIUS, Stroke::new(1.0, LIQUID_RIM.gamma_multiply(0.45)), StrokeKind::Inside);
        return;
    };
    if flare > 0.0 {
        painter.rect_stroke(rect.expand(2.0), RADIUS + 2.0, Stroke::new(3.0, LIQUID_GLOW.gamma_multiply(0.5 * flare)), StrokeKind::Middle);
    }
    let points = outline(rect.shrink(0.5), RADIUS);
    let lengths: Vec<f32> = (0..points.len()).map(|at| (points[(at + 1) % points.len()] - points[at]).length()).collect();
    let around: f32 = lengths.iter().sum();
    let head = turned(time, TASKS_FRAME_TURN) * around;
    let mut along = 0.0;
    for (at, length) in lengths.iter().enumerate() {
        let apart = (along + length / 2.0 - head).rem_euclid(around);
        let lit = (1.0 - smooth(0.0, 0.14 * around, apart.min(around - apart))).max(flare);
        let color = tint(LIQUID_RIM, Color32::WHITE, lit).gamma_multiply(0.4 + 0.6 * lit);
        painter.line_segment([points[at], points[(at + 1) % points.len()]], Stroke::new(1.0, color));
        along += length;
    }
}

/// The mark of a task in `slot`, for how far the task has got.
#[derive(Clone, Copy)]
struct Marks {
    palette: Palette,
    /// egui's time when things move; `None` with "Animations & effects" off.
    time: Option<f64>,
    /// The assistant is working.
    running: bool,
    /// The run was stopped or broke off at the task at work.
    halted: bool,
    /// How far the task that was just finished is through its burst, with that task's place in the list.
    burst: Option<(usize, f32)>,
}

impl Marks {
    fn paint(self, painter: &egui::Painter, slot: Rect, index: usize, state: State, opacity: f32) {
        match state {
            State::Waiting => ring(painter, slot, self.palette.muted.gamma_multiply(0.45 * opacity)),
            State::Done => ticked(painter, slot, self.palette, self.burst.filter(|burst| burst.0 == index).map(|burst| burst.1), opacity),
            State::Doing if self.halted => icons::paint(painter, slot, HALTED_ICON, PROBLEM),
            State::Doing if self.running => blob(painter, slot.center(), slot.width() / 2.0 - 1.5, self.time),
            State::Doing => ring(painter, slot, LIQUID),
        }
    }
}

/// Paints the picture of the liquid `blobs` over the list. `origin` and `size` are the part of the list the
/// picture covers, in points from the list's top left corner `corner`; `lit` is where the travelling light is.
// Human: the picture is worked out anew for every frame while liquid is there, at no more texels than the eye
// needs; a list is small, and a pour lasts about two seconds.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss, reason = "the size of a small picture, in texels")]
fn pour_over(ui: &Ui, painter: &egui::Painter, picture: &mut Option<TextureHandle>, blobs: &[liquid::Blob], part: Rect, corner: Pos2, lit: [f32; 2]) {
    let scale = ui.ctx().pixels_per_point().clamp(1.0, LIQUID_FINE);
    let size = [(part.width() * scale).ceil().max(1.0) as usize, (part.height() * scale).ceil().max(1.0) as usize];
    let image = liquid::picture(blobs, [part.left(), part.top()], size, scale, lit[0], lit[1]);
    let shown = vec2(size[0] as f32, size[1] as f32) / scale;
    let texture = match picture {
        Some(texture) => {
            texture.set(image, TextureOptions::LINEAR);
            texture.id()
        }
        None => picture.insert(ui.ctx().load_texture("task list liquid", image, TextureOptions::LINEAR)).id(),
    };
    let place = Rect::from_min_size(corner + part.min.to_vec2(), shown);
    painter.image(texture, place, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
}

/// What the header line shows, and whether a click on it folded or opened the list.
struct Head<'a> {
    tasks: &'a Tasks,
    folded: bool,
    marks: Marks,
}

impl Head<'_> {
    /// Draws the header in `line`: open, the word "Tasks"; folded, the task at work (or that every task is
    /// finished). At its right end how many tasks are finished, and the arrow that folds and opens.
    fn paint(&self, ui: &Ui, painter: &egui::Painter, line: Rect) {
        let palette = self.marks.palette;
        let slot = Rect::from_center_size(pos2(line.left() + TASKS_MARK / 2.0, line.center().y), Vec2::splat(TASKS_MARK));
        let arrow = Rect::from_center_size(pos2(line.right() - 7.0, line.center().y), Vec2::splat(14.0));
        icons::paint(painter, arrow, if self.folded { OPEN_ICON } else { FOLD_ICON }, palette.muted);
        let finished = self.tasks.all_done();
        let count = if finished && self.folded { String::new() } else { format!("{} of {}", self.tasks.done(), self.tasks.len()) };
        let count = words(ui, &count, palette.muted, line.width());
        let count_left = arrow.left() - 8.0 - count.size().x;
        painter.galley(pos2(count_left, line.center().y - count.size().y / 2.0), count.clone(), palette.muted);
        let from = slot.right() + TASKS_MARK_GAP - 1.0;
        let room = count_left - 10.0 - from;
        let (text, color) = match (self.folded, self.tasks.current()) {
            (false, _) => {
                icons::paint(painter, slot, LIST_ICON, palette.foreground);
                ("Tasks".to_owned(), palette.primary)
            }
            (true, Some(current)) => {
                self.marks.paint(painter, slot, self.tasks.done(), State::Doing, 1.0);
                (current.to_owned(), palette.primary)
            }
            (true, None) => {
                self.marks.paint(painter, slot, self.tasks.len().saturating_sub(1), State::Done, 1.0);
                (format!("All {} tasks done", self.tasks.len()), palette.foreground)
            }
        };
        let text = words(ui, &text, color, room);
        painter.galley(pos2(from, line.center().y - text.size().y / 2.0), text, color);
    }
}

/// The bar that shows how far the list is, in `track`: `share` of it is filled, and while the assistant works
/// a light runs along the filled part.
fn bar(painter: &egui::Painter, track: Rect, share: f32, palette: Palette, time: Option<f64>) {
    painter.rect_filled(track, TASKS_BAR / 2.0, palette.hover);
    let filled = Rect::from_min_max(track.min, pos2(track.left() + track.width() * share.clamp(0.0, 1.0), track.bottom()));
    if filled.width() < 0.5 {
        return;
    }
    painter.rect_filled(filled, TASKS_BAR / 2.0, LIQUID);
    if let Some(time) = time {
        let middle = track.left() + (turned(time, 2.2) * 1.6 - 0.3) * track.width();
        let light = Rect::from_center_size(pos2(middle, track.center().y), vec2(track.width() * 0.18, TASKS_BAR));
        painter.with_clip_rect(filled).rect_filled(light, TASKS_BAR / 2.0, Color32::from_white_alpha(150));
    }
}

/// Where the task list is drawn, and how.
#[derive(Clone, Copy)]
pub struct Look {
    /// The room of the conversation; the list takes the top of it.
    pub area: Rect,
    /// How far the panel goes on to each side of `area`: the liquid uses the panel's whole width.
    pub reach: f32,
    /// The panel's colours.
    pub palette: Palette,
    /// Things move ("Animations & effects" in the settings).
    pub animate: bool,
    /// Holds the list's motion at this many seconds into it, for a picture of it: into its pouring in while no
    /// task is finished, else into the step from the task finished last to the next.
    pub held: Option<f32>,
}

/// How far under the list's top edge its first row starts.
const ROWS_TOP: f32 = TASKS_PAD + TASKS_HEAD + TASKS_BAR_GAP + TASKS_BAR + TASKS_ROWS_GAP;

/// What of the list's motion is where in the frame being drawn.
struct Moment {
    /// The list across the panel's whole width, as high as it is drawn right now.
    whole: Rect,
    /// Seconds since the list first showed, and since a task was last finished.
    since_born: f32,
    since_step: f32,
    /// The list is folded to one line.
    folded: bool,
    /// How the list pours in, while it does.
    pour: Option<Arc<Pour>>,
    /// Every row is there: the first task can be shown at work.
    settled: bool,
}

/// The row of the task `index` in a list at the top of `area`.
#[expect(clippy::cast_precision_loss, reason = "a list has at most twelve tasks")]
fn row_of(area: Rect, index: usize) -> Rect {
    Rect::from_min_size(pos2(area.left(), area.top() + ROWS_TOP + TASKS_ROW * index as f32), vec2(area.width(), TASKS_ROW))
}

/// Takes in what has changed since the last frame: another list, a task finished, a moment to be held.
// Human: a list that is there when the panel first shows it doesn't pour in; only one that is made while one
// watches does.
fn take_in(motion: &mut Motion, chat: &Chat, now: f64, look: Look) {
    let tasks = &chat.tasks;
    let long_ago = f64::NEG_INFINITY;
    if !motion.seen || motion.made != tasks.made() {
        let pours = look.animate && chat.running() && tasks.done() == 0;
        *motion = Motion {
            seen: true,
            made: tasks.made(),
            born: if pours { now } else { long_ago },
            done: tasks.done(),
            stepped: long_ago,
            grown: tasks.added().0,
            handed: tasks.handed(),
            left: long_ago,
            ..Motion::default()
        };
        motion.height = if pours { 0.0 } else { -1.0 };
    }
    // Human: tasks added to the list pour in at their rows, as a new list does.
    let (grown, at, count) = tasks.added();
    if grown != motion.grown {
        motion.grown = grown;
        if look.animate && chat.running() {
            (motion.born, motion.pour, motion.poured, motion.fresh) = (now, None, false, Some((at, count)));
        }
    }
    if motion.done != tasks.done() {
        motion.stepped = if look.animate && tasks.done() == motion.done + 1 { now } else { long_ago };
        motion.done = tasks.done();
    }
    if let Some(held) = look.held.filter(|_| look.animate) {
        let then = now - f64::from(held);
        if tasks.done() == 0 {
            motion.born = then;
        } else if grown > 0 {
            (motion.born, motion.fresh) = (then, Some((at, count)));
        } else {
            motion.stepped = then;
        }
    }
}

/// The tasks, one row each: its mark, its words, and around the task at work its frame.
fn rows(ui: &Ui, painter: &egui::Painter, chat: &Chat, look: Look, moment: &Moment, marks: Marks) {
    let (tasks, palette) = (&chat.tasks, look.palette);
    for (index, (text, state)) in tasks.each().enumerate() {
        let opacity = moment.pour.as_ref().map_or(1.0, |pour| pour.shown(index, moment.since_born));
        if opacity <= 0.0 {
            continue;
        }
        // Human: the task that comes to be at work waits until the drop from the finished one has reached it.
        let arriving = state == State::Doing && (!moment.settled || (index > 0 && moment.since_step < TASKS_STEP_NEXT));
        let state = if arriving { State::Waiting } else { state };
        let row = row_of(look.area, index);
        if state == State::Doing && !tasks.halted() {
            let since = match (index, &moment.pour) {
                (0, Some(pour)) if pour.first() == 0 => moment.since_born - pour.settled(),
                (0, _) => f32::MAX,
                _ => moment.since_step - TASKS_STEP_NEXT,
            };
            let flare = if look.animate { 1.0 - smooth(0.0, TASKS_FLARE, since) } else { 0.0 };
            frame(painter, row.expand2(vec2(TASKS_ROW_REACH, -1.0)), palette, marks.time.filter(|_| chat.running()), flare);
        }
        let slot = Rect::from_center_size(pos2(row.left() + TASKS_MARK / 2.0, row.center().y), Vec2::splat(TASKS_MARK));
        marks.paint(painter, slot, index, state, opacity);
        let color = match state {
            State::Done => palette.quiet,
            State::Doing => palette.primary,
            State::Waiting => palette.muted,
        }
        .gamma_multiply(opacity);
        let from = slot.right() + TASKS_MARK_GAP;
        let text = words(ui, text, color, row.right() - from);
        painter.galley(pos2(from, row.center().y - text.size().y / 2.0), text, color);
    }
}

/// The liquid over the list: a new list pours in, and a drop runs from a finished task to the next one.
/// Returns whether there was any.
fn liquid_over(ui: &Ui, painter: &egui::Painter, picture: &mut Option<TextureHandle>, tasks: &Tasks, look: Look, moment: &Moment) -> bool {
    let whole = moment.whole;
    let stepping = look.animate && !moment.folded && tasks.open() && tasks.done() > 0 && moment.since_step < TASKS_STEP_TIME;
    if let Some(pour) = &moment.pour {
        let part = Rect::from_min_size(Pos2::ZERO, vec2(whole.width(), tall(tasks.len(), false, false)));
        let lit = [liquid::light_at(moment.since_born, whole.width()), whole.width()];
        pour_over(ui, painter, picture, &pour.blobs(moment.since_born), part, whole.min, lit);
    } else if stepping {
        let from = row_of(look.area, tasks.done() - 1);
        let mark = pos2(from.left() + TASKS_MARK / 2.0, from.center().y) - whole.min.to_vec2();
        let part = Rect::from_min_max(mark - Vec2::splat(LIQUID_REACH + 14.0), mark + vec2(LIQUID_REACH + 22.0, TASKS_ROW + LIQUID_REACH + 14.0));
        let blobs = liquid::step([mark.x, mark.y], TASKS_ROW, moment.since_step / TASKS_STEP_TIME);
        // The whole drop is lit: it is too small for a light to travel along.
        pour_over(ui, painter, picture, &blobs, part, whole.min, [mark.x, whole.width() * 2.0]);
    } else {
        *picture = None;
        return false;
    }
    true
}

/// Draws the task list of the chat `key` at the top of the room `look` gives it, and gives the room left under
/// it for the conversation.
// Human: a chat without a list shows nothing and gives its whole room on.
// Agent: MAIN-THREAD; asks for another frame while something moves.
pub fn show(ui: &Ui, chat: &Chat, key: u64, look: Look) -> Rect {
    let (tasks, area, palette) = (&chat.tasks, look.area, look.palette);
    let now = ui.input(|input| input.time);
    let pace = ui.input(|input| input.stable_dt).min(0.1);
    let mut motion: Motion = ui.data(|data| data.get_temp(motion_id(key))).unwrap_or_default();
    if tasks.is_empty() {
        return leave(ui, chat, key, look, motion);
    }
    take_in(&mut motion, chat, now, look);
    let (since_born, since_step) = (seconds(now, motion.born), seconds(now, motion.stepped));
    // Human: a list that has just been finished stays open until its last tick has burst, then folds away.
    let folded = motion.folded.unwrap_or(tasks.all_done() && since_step > TASKS_BURST + 0.3);
    let target = tall(tasks.len(), folded, tasks.all_done());
    let gliding = look.animate && motion.height >= 0.0 && (target - motion.height).abs() > 0.5;
    motion.height = if gliding { motion.height + (target - motion.height) * (1.0 - (-TASKS_GLIDE * pace).exp()) } else { target };
    let whole = Rect::from_min_max(pos2(area.left() - look.reach, area.top()), pos2(area.right() + look.reach, area.top() + motion.height));
    let painter = ui.painter().with_clip_rect(whole.intersect(ui.clip_rect()));

    if motion.pour.is_none() && motion.born.is_finite() && !motion.poured {
        // Where the rows are, from the list's top left corner: the liquid pours them there.
        let span = [look.reach - TASKS_ROW_REACH, whole.width() - look.reach + TASKS_ROW_REACH];
        // A held pour is the same one in every picture.
        let seed = if look.held.is_some() { key } else { now.to_bits() ^ key };
        let (first, count) = motion.fresh.unwrap_or((0, tasks.len()));
        motion.pour = Some(Arc::new(Pour::new(seed, count, first, Rows { width: whole.width(), span, top: ROWS_TOP, height: TASKS_ROW })));
        motion.poured = true;
    }
    motion.pour = motion.pour.take().filter(|pour| since_born < pour.over() && !folded);
    // Human: while the list pours in, the first task is not shown at work until every row is there. Tasks
    // added to a list change nothing about the task at work.
    let settled = motion.pour.as_ref().is_none_or(|pour| pour.first() > 0 || since_born >= pour.settled());
    let moment = Moment { whole, since_born, since_step, folded, pour: motion.pour.clone(), settled };

    let time = look.animate.then_some(now);
    let burst = (since_step < TASKS_BURST && tasks.done() > 0).then(|| (tasks.done() - 1, since_step / TASKS_BURST));
    let marks = Marks { palette, time, running: chat.running(), halted: tasks.halted(), burst };

    // The header: a click folds the list or opens it.
    let line = Rect::from_min_size(pos2(area.left(), area.top() + TASKS_PAD), vec2(area.width(), TASKS_HEAD));
    let pressed = ui.interact(line.expand2(vec2(0.0, TASKS_PAD / 2.0)), ui.scope_id().with(("tasklist-head", key)), Sense::click());
    if pressed.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        motion.folded = Some(!folded);
    }
    Head { tasks, folded, marks }.paint(ui, &painter, line);
    if !(folded && tasks.all_done()) {
        #[expect(clippy::cast_precision_loss, reason = "a list has at most twelve tasks")]
        let share = |done: usize| done as f32 / tasks.len().max(1) as f32;
        let before = share(tasks.done().saturating_sub(1));
        let filled = before + (share(tasks.done()) - before) * smooth(0.0, 1.0, since_step / TASKS_BAR_TIME);
        let track = Rect::from_min_size(pos2(area.left(), line.bottom() + TASKS_BAR_GAP), vec2(area.width(), TASKS_BAR));
        bar(&painter, track, filled, palette, time.filter(|_| chat.running() && tasks.open()));
    }
    if !folded {
        rows(ui, &painter, chat, look, &moment, marks);
    }
    let wet = liquid_over(ui, &painter, &mut motion.picture, tasks, look, &moment);

    // Human: a step's burst, a glide and the liquid move at full rate; the bar's light while the assistant works
    // at the idle rate.
    let moving = wet || gliding || since_step < TASKS_BURST + 0.4;
    if look.animate && moving {
        ui.request_repaint();
    } else if look.animate && chat.running() && tasks.open() {
        ui.ctx().request_repaint_after(crate::theme::IDLE_FRAME);
    } else if tasks.all_done() {
        // Human: nothing moves, but the finished list is about to be filed into the conversation.
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
    ui.data_mut(|data| data.insert_temp(motion_id(key), motion));
    Rect::from_min_max(pos2(area.left(), whole.bottom() + 1.0), area.max)
}

/// The top of the panel of a chat that has no list: nothing, or what is left of a list that has just been
/// filed into the conversation. Its line "All 6 tasks done" fades while it melts into the drop that carries it
/// down ([`hand_over`]), and the room it took closes.
fn leave(ui: &Ui, chat: &Chat, key: u64, look: Look, mut motion: Motion) -> Rect {
    let (tasks, area) = (&chat.tasks, look.area);
    let now = ui.input(|input| input.time);
    let held = look.held.filter(|_| look.animate && tasks.handed() > 0);
    if look.animate && motion.seen && tasks.handed() > motion.handed {
        motion.left = now;
    }
    if let Some(held) = held {
        (motion.seen, motion.left) = (true, now - f64::from(held));
    }
    motion.handed = tasks.handed();
    let since = seconds(now, motion.left);
    if !motion.seen || since >= liquid::HAND_TIME {
        ui.data_mut(|data| data.remove::<Motion>(motion_id(key)));
        return area;
    }
    let height = tall(0, true, true) * (1.0 - smooth(TASKS_HAND_MELT, TASKS_HAND_MELT + 0.2, since));
    let whole = Rect::from_min_max(pos2(area.left() - look.reach, area.top()), pos2(area.right() + look.reach, area.top() + height));
    let painter = ui.painter().with_clip_rect(whole.intersect(ui.clip_rect()));
    let fading = 1.0 - smooth(0.0, TASKS_HAND_MELT, since);
    let count = chat.entries.iter().rev().find(|entry| !entry.finished.is_empty()).map_or(0, |entry| entry.finished.len());
    if fading > 0.0 && count > 0 {
        let line = Rect::from_min_size(pos2(area.left(), area.top() + TASKS_PAD), vec2(area.width(), TASKS_HEAD));
        let slot = Rect::from_center_size(pos2(line.left() + TASKS_MARK / 2.0, line.center().y), Vec2::splat(TASKS_MARK));
        ticked(&painter, slot, look.palette, None, fading);
        let color = look.palette.foreground.gamma_multiply(fading);
        let from = slot.right() + TASKS_MARK_GAP - 1.0;
        let text = words(ui, &format!("All {count} tasks done"), color, line.right() - from);
        painter.galley(pos2(from, line.center().y - text.size().y / 2.0), text, color);
    }
    ui.request_repaint();
    ui.data_mut(|data| data.insert_temp(motion_id(key), motion));
    Rect::from_min_max(pos2(area.left(), whole.bottom() + if height > 0.5 { 1.0 } else { 0.0 }), area.max)
}

/// Paints the drop that carries a finished list from the top of the panel into the conversation, over
/// everything the panel has drawn: call it after the conversation. `look` is what [`show`] was given.
// Human: the drop lands on the note the list is filed as, which the conversation shows from then on.
pub fn hand_over(ui: &Ui, key: u64, look: Look) {
    let Some(mut motion) = ui.data(|data| data.get_temp::<Motion>(motion_id(key))).filter(|_| look.animate) else { return };
    let since = seconds(ui.input(|input| input.time), motion.left);
    let Some(landing) = ui.data(|data| data.get_temp::<Rect>(landing_id())).filter(|_| since < liquid::HAND_TIME) else { return };
    let corner = pos2(look.area.left() - look.reach, look.area.top());
    let width = look.area.width() + 2.0 * look.reach;
    let from = [look.reach + TASKS_MARK / 2.0, TASKS_PAD + TASKS_HEAD / 2.0];
    let span = [look.reach - TASKS_ROW_REACH, width - look.reach + TASKS_ROW_REACH];
    let to = [landing.left() - TASKS_ROW_REACH - corner.x, landing.right() + TASKS_ROW_REACH - corner.x, landing.center().y - corner.y];
    let part = Rect::from_min_max(Pos2::ZERO, pos2(width, (to[2] + LIQUID_REACH + 14.0).max(TASKS_HEAD)));
    let painter = ui.painter().with_clip_rect(Rect::from_min_max(corner, look.area.max + vec2(look.reach, 0.0)).intersect(ui.clip_rect()));
    let lit = [liquid::light_at(since * 1.6, width), width];
    pour_over(ui, &painter, &mut motion.picture, &liquid::hand(from, span, to, since), part, corner, lit);
    ui.data_mut(|data| data.insert_temp(motion_id(key), motion));
}

/// The note in the conversation of a task list that was worked through ("Tasks completed: 6 of 6"). A click
/// opens it to show the tasks, and closes it again. `at` is the note's place in the conversation, by which
/// it is remembered whether it is open; `scale` is how much larger than small the conversation's text is.
pub fn filed(ui: &mut Ui, entry: &Entry, at: usize, scale: f32, palette: Palette) {
    let id = egui::Id::unique(("tasklist-filed", at));
    let mut open: bool = ui.data(|data| data.get_temp(id)).unwrap_or(false);
    let size = (SIDEBAR_TEXT_SIZE - 1.0) * scale;
    let head = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 7.0;
            let (slot, _) = ui.allocate_exact_size(Vec2::splat(TASKS_MARK), Sense::hover());
            ticked(ui.painter(), slot, palette, None, 1.0);
            ui.add(egui::Label::new(egui::RichText::new(&entry.text).size(size).color(palette.foreground)).selectable(false));
            let (arrow, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
            icons::paint(ui.painter(), arrow, if open { FOLD_ICON } else { OPEN_ICON }, palette.muted);
        })
        .response;
    if ui.interact(head.rect, id.with("press"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        open = !open;
    }
    ui.data_mut(|data| {
        data.insert_temp(id, open);
        data.insert_temp(landing_id(), head.rect);
    });
    if !open {
        return;
    }
    for task in &entry.finished {
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 7.0;
            ui.add_space(TASKS_MARK + 7.0);
            let (slot, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
            icons::paint(ui.painter(), slot, TICK_ICON, palette.muted);
            ui.add(egui::Label::new(egui::RichText::new(task).size(size).color(palette.muted)).truncate().selectable(false));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_as_high_as_what_it_shows() {
        let open = tall(6, false, false);
        assert!((open - (TASKS_PAD * 2.0 + TASKS_HEAD + TASKS_BAR_GAP + TASKS_BAR + TASKS_ROWS_GAP + 6.0 * TASKS_ROW)).abs() < 0.01);
        // Folded it is one line with the bar under it; with every task finished the bar goes too.
        assert!(tall(6, true, false) < open && tall(6, true, true) < tall(6, true, false));
        assert!((tall(12, true, false) - tall(2, true, false)).abs() < 0.01);
        assert!((tall(3, false, true) - tall(3, false, false)).abs() < 0.01);
    }

    #[test]
    fn every_icon_of_the_list_is_bundled() {
        for name in [LIST_ICON, TICK_ICON, HALTED_ICON, FOLD_ICON, OPEN_ICON] {
            assert!(icons::outline("lucide", name).is_some(), "{name}");
        }
    }

    #[test]
    fn the_outline_of_a_frame_goes_once_around_it() {
        let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(300.0, 26.0));
        let points = outline(rect, 10.0);
        // Every point lies on the frame, and neighbours are close: a light can move along without a jump.
        assert!(points.iter().all(|point| rect.expand(0.01).contains(*point)));
        let around: f32 = (0..points.len()).map(|at| (points[(at + 1) % points.len()] - points[at]).length()).sum();
        let exact = 2.0 * (300.0 - 20.0) + 2.0 * (26.0 - 20.0) + std::f32::consts::TAU * 10.0;
        assert!((around - exact).abs() < 1.0, "{around} {exact}");
        assert!((0..points.len()).all(|at| (points[(at + 1) % points.len()] - points[at]).length() < 9.0));
        // A frame lower than its corners are round still closes.
        assert!(outline(Rect::from_min_size(Pos2::ZERO, vec2(40.0, 12.0)), 10.0).len() > 8);
    }

    #[test]
    fn a_colour_between_two_is_between_them() {
        assert_eq!(tint(Color32::BLACK, Color32::WHITE, 0.0), Color32::BLACK);
        assert_eq!(tint(Color32::BLACK, Color32::WHITE, 1.0), Color32::WHITE);
        assert_eq!(tint(Color32::BLACK, Color32::WHITE, 0.5), Color32::from_rgb(128, 128, 128));
        assert_eq!(tint(Color32::BLACK, Color32::WHITE, 7.0), Color32::WHITE);
    }
}
