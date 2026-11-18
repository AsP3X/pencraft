//! What an AI model working on a document looks like on the canvas: what it changes is shown a step at a
//! time, every layer it touches flashes, and a cursor with the model's name moves to where it works.

use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use egui::{Color32, Pos2, Rect as ScreenRect, Stroke};
use pen_doc::geom::{Point, Rect};
use pen_doc::scene::SceneNode;

use crate::editor::{Editor, find_in};
use crate::render::screen;
use crate::theme::{
    AGENT, AGENT_CURSOR, AGENT_CURSOR_FADE, AGENT_CURSOR_STAY, AGENT_CURSOR_TRAVEL, AGENT_FLASH_GROW, AGENT_FLASH_STROKE, AGENT_FLASH_TIME,
    AGENT_LABEL_TEXT, AGENT_STEP, AGENT_STEPS_MOST,
};

/// What a step of a model's change did to its layer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum What {
    /// The layer is new: the canvas leaves it out until its turn.
    Made,
    /// The layer was there and is changed.
    Changed,
    /// The layer is gone.
    Removed,
}

/// One small step of a model's change, to be shown at its turn.
pub struct Turn {
    /// The layer (by scene id); empty for a change to no one layer (the document's variables).
    pub id: String,
    /// What was done to it.
    pub what: What,
    /// Where the layer is in the document (for one that is gone, where it was); the cursor goes there.
    pub place: Option<Rect>,
    /// What the document looks like from this step on, when that differs from the step before: the document
    /// laid out as it was after this step's operation.
    pub scene: Option<Vec<SceneNode>>,
}

/// A [`Turn`] waiting.
struct Step {
    turn: Turn,
    /// When its turn is.
    at: Instant,
    /// The name beside the cursor that goes to it.
    label: String,
}

/// A layer flashing.
struct Flash {
    /// The layer (by scene id).
    id: String,
    since: Instant,
    /// Where a layer that is gone was: the flash is drawn there.
    ghost: Option<Rect>,
}

/// The AI's cursor: on its way from one document point to another.
struct Cursor {
    from: Point,
    to: Point,
    /// When it set off.
    moved: Instant,
    /// When the AI last did something.
    seen: Instant,
    /// The name shown next to it; empty for none.
    label: String,
}

impl Cursor {
    /// Where it is at `now`: it eases out towards where it is going.
    fn place(&self, now: Instant) -> Point {
        let t = (now.saturating_duration_since(self.moved).as_secs_f32() / AGENT_CURSOR_TRAVEL.as_secs_f32()).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(3);
        Point::new(self.from.x + (self.to.x - self.from.x) * eased, self.from.y + (self.to.y - self.from.y) * eased)
    }
}

/// The AI activity shown on one document.
#[derive(Default)]
pub struct Activity {
    flashes: Vec<Flash>,
    cursor: Option<Cursor>,
    /// The steps of the model's changes that are still to be shown, in their order.
    steps: VecDeque<Step>,
    /// The new layers whose turn hasn't come: the canvas leaves them out until then.
    hidden: HashSet<String>,
    /// What the canvas shows while steps are waiting: the document as it was after the operation the last
    /// step taken belongs to. `None` shows the document as it is.
    stage: Option<Vec<SceneNode>>,
    /// Counts the changes of `stage`, so drawing kept from one frame to the next knows when to start over.
    stamp: u64,
    /// The document's generation ([`Editor::generation`]) the waiting steps are for.
    generation: u64,
    /// A picture of the app (`--activity`) shows the selected layers as a model works on them.
    pub posed: bool,
}

impl Activity {
    /// Lets the layers `ids` flash, starting now.
    pub fn flash(&mut self, ids: &[String], now: Instant) {
        self.flashes.retain(|flash| now.saturating_duration_since(flash.since) < AGENT_FLASH_TIME && !ids.contains(&flash.id));
        self.flashes.extend(ids.iter().map(|id| Flash { id: id.clone(), since: now, ghost: None }));
    }

    /// Sends the cursor to the middle of `target` (it stays where it is without one) under the name `label`.
    // Human: the cursor appears where it first works, and from then on travels from where it is.
    pub fn point(&mut self, target: Option<Rect>, label: &str, now: Instant) {
        let from =
            self.cursor.as_ref().filter(|c| now.saturating_duration_since(c.seen) < AGENT_CURSOR_STAY + AGENT_CURSOR_FADE).map(|c| c.place(now));
        let Some(to) = target.map(|b| b.center()).or(from) else { return };
        self.cursor = Some(Cursor { from: from.unwrap_or(to), to, moved: now, seen: now, label: label.to_owned() });
    }

    /// Shows a change a step at a time: `turns` get their turn one after another in their order, starting now
    /// or when the steps already waiting are through. `generation` is the document's, with the whole change
    /// in it.
    // Human: the document itself is changed at once and whole, so the model's answer and its next call see the
    // finished change. Only the canvas takes its time: it shows the document as it was after each operation,
    // and leaves a new layer out until its turn. So a design is seen being built layer by layer, a change is
    // seen when it is made and a deleted layer goes when it is deleted, instead of everything at once.
    // Human: a call with many steps doesn't take longer than `AGENT_STEPS_MOST`: they then follow each other
    // faster.
    pub fn play(&mut self, turns: Vec<Turn>, label: &str, now: Instant, generation: u64) {
        self.generation = generation;
        if turns.is_empty() {
            self.point(None, label, now);
            return;
        }
        let gap = AGENT_STEP.min(AGENT_STEPS_MOST / u32::try_from(turns.len()).unwrap_or(u32::MAX));
        let mut at = self.steps.back().map_or(now, |last| (last.at + AGENT_STEP).max(now));
        for turn in turns {
            if turn.what == What::Made {
                self.hidden.insert(turn.id.clone());
            }
            self.steps.push_back(Step { turn, at, label: label.to_owned() });
            at += gap;
        }
    }

    /// Takes the steps whose turn has come by `now`: the canvas shows the document as of each, its layer is
    /// shown from then on and flashes, and the cursor goes to it. `generation` is the document's now.
    // Human: when the document has changed since the steps were made (the person edited it, or took the
    // change back), there is nothing left to show them on: the canvas shows the document as it is at once.
    // Agent: called once a frame before the canvas is drawn; `hidden`, `stage` and `stamp` change here and in
    // `play` only.
    pub fn advance(&mut self, now: Instant, generation: u64) {
        if generation != self.generation && (!self.steps.is_empty() || self.stage.is_some()) {
            self.steps.clear();
            self.hidden.clear();
            self.stage = None;
            self.stamp += 1;
            return;
        }
        while self.steps.front().is_some_and(|step| step.at <= now) {
            let Some(Step { turn, at, label }) = self.steps.pop_front() else { break };
            if let Some(scene) = turn.scene {
                self.stage = Some(scene);
                self.stamp += 1;
            }
            // A layer waiting for a second turn (made, then changed again) is shown at its first.
            self.hidden.remove(&turn.id);
            if turn.place.is_some() {
                self.flashes.retain(|flash| now.saturating_duration_since(flash.since) < AGENT_FLASH_TIME && flash.id != turn.id);
                self.flashes.push(Flash { id: turn.id, since: at, ghost: turn.place.filter(|_| turn.what == What::Removed) });
            }
            self.point(turn.place, &label, now);
        }
        if self.steps.is_empty() && self.stage.take().is_some() {
            self.stamp += 1;
        }
    }

    /// The layers the canvas leaves out for now (by scene id): made by a model, their turn to be shown not
    /// yet come.
    #[must_use]
    pub fn hidden(&self) -> &HashSet<String> {
        &self.hidden
    }

    /// What the canvas shows in place of the document as it is, while steps of a change are waiting; and a
    /// number that changes whenever this does.
    #[must_use]
    pub fn stage(&self) -> (Option<&[SceneNode]>, u64) {
        (self.stage.as_deref(), self.stamp)
    }

    /// The top-level layers on the canvas right now, of a document whose scene is `scene`.
    // Human: while a model's change is shown a step at a time, a board whose turn hasn't come is not there yet
    // (it has no name over it either), and one that is gone stays until it goes.
    pub fn shown<'a>(&'a self, scene: &'a [SceneNode]) -> impl Iterator<Item = &'a SceneNode> {
        self.stage.as_deref().unwrap_or(scene).iter().filter(|node| !self.hidden.contains(&node.id))
    }

    /// How long until nothing of this is left to show, from `now`; `None` when nothing is showing.
    fn remaining(&self, now: Instant) -> Option<Duration> {
        let flash = self.flashes.iter().map(|flash| (flash.since + AGENT_FLASH_TIME).saturating_duration_since(now)).max();
        let cursor = self.cursor.as_ref().map(|c| (c.seen + AGENT_CURSOR_STAY + AGENT_CURSOR_FADE).saturating_duration_since(now));
        flash.max(cursor).filter(|left| !left.is_zero())
    }

    /// Draws the flashes and the cursor over the canvas. `scale` is the cursor's size (1 = small).
    // Agent: MAIN-THREAD; asks for the next frame while something is moving.
    pub fn draw(&self, ui: &egui::Ui, painter: &egui::Painter, editor: &Editor, view: ScreenRect, scale: f32) {
        let now = Instant::now();
        if self.steps.is_empty() && self.remaining(now).is_none() {
            return;
        }
        ui.ctx().request_repaint();
        let cam = &editor.camera;
        let shown = self.stage.as_deref().unwrap_or(&editor.scene);
        for Flash { id, since, ghost } in &self.flashes {
            let t = now.saturating_duration_since(*since).as_secs_f32() / AGENT_FLASH_TIME.as_secs_f32();
            if t >= 1.0 {
                continue;
            }
            if let Some(was) = ghost {
                flash(painter, was.corners().map(|c| screen(cam, view, c)), t);
            } else if let Some(node) = find_in(shown, id) {
                let corners = Rect::new(0.0, 0.0, node.local.w, node.local.h).corners().map(|c| screen(cam, view, node.to_doc.apply(c)));
                flash(painter, corners, t);
            }
        }
        if let Some(cursor) = &self.cursor {
            let idle = now.saturating_duration_since(cursor.seen);
            let fade = 1.0 - (idle.saturating_sub(AGENT_CURSOR_STAY).as_secs_f32() / AGENT_CURSOR_FADE.as_secs_f32()).clamp(0.0, 1.0);
            if fade > 0.0 {
                pointer(painter, screen(cam, view, cursor.place(now)), &cursor.label, scale, fade);
            }
        }
    }
}

/// One layer's flash at `t` (0 = just touched, 1 = over): an outline that starts wide and closes in on the
/// layer, a light wash, and a band of light sweeping down over it.
// Human: the flash on a node an agent changed: a 2-point outline that contracts onto the
// node while a scan line runs over it, in the agent's colour, fading out.
fn flash(painter: &egui::Painter, corners: [Pos2; 4], t: f32) {
    let fade = (2.0 * (1.0 - t)).min(1.0);
    let centre = Pos2::new(corners.iter().map(|c| c.x).sum::<f32>() / 4.0, corners.iter().map(|c| c.y).sum::<f32>() / 4.0);
    // The outline is pushed out from the middle, less and less.
    let grow = AGENT_FLASH_GROW * (1.0 - t).powi(3);
    let outline: Vec<Pos2> = corners
        .iter()
        .map(|c| {
            let away = *c - centre;
            let length = away.length().max(1.0);
            *c + away * (grow / length)
        })
        .collect();
    painter.add(egui::Shape::convex_polygon(corners.to_vec(), AGENT.gamma_multiply(0.10 * fade), Stroke::NONE));
    painter.add(egui::Shape::closed_line(outline, Stroke::new(AGENT_FLASH_STROKE, AGENT.gamma_multiply(fade))));
    // The band of light: clear at its top, brightest at its lower edge, running from above the layer to below it.
    let bounds = ScreenRect::from_points(&corners);
    let band = (bounds.height() * 0.35).clamp(12.0, 160.0);
    let sweep = (t * 1.6).min(1.0);
    let bottom = bounds.top() + (bounds.height() + band) * sweep;
    if sweep < 1.0 {
        let bright = AGENT.gamma_multiply(0.45 * fade);
        let mut mesh = egui::Mesh::default();
        mesh.colored_vertex(Pos2::new(bounds.left(), bottom - band), Color32::TRANSPARENT);
        mesh.colored_vertex(Pos2::new(bounds.right(), bottom - band), Color32::TRANSPARENT);
        mesh.colored_vertex(Pos2::new(bounds.right(), bottom), bright);
        mesh.colored_vertex(Pos2::new(bounds.left(), bottom), bright);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.with_clip_rect(bounds.intersect(painter.clip_rect())).add(egui::Shape::mesh(mesh));
    }
}

/// The cursor: an arrow with its tip at `tip`, and the name beside it on a pill.
fn pointer(painter: &egui::Painter, tip: Pos2, label: &str, scale: f32, fade: f32) {
    let arrow: Vec<Pos2> = AGENT_CURSOR.iter().map(|(x, y)| tip + egui::vec2(*x, *y) * scale).collect();
    painter.add(egui::Shape::convex_polygon(arrow, AGENT.gamma_multiply(fade), Stroke::new(1.0, Color32::WHITE.gamma_multiply(fade))));
    if label.is_empty() {
        return;
    }
    let text =
        painter.layout_no_wrap(label.to_owned(), egui::FontId::proportional(AGENT_LABEL_TEXT * scale.sqrt()), Color32::WHITE.gamma_multiply(fade));
    let pad = egui::vec2(6.0, 3.0);
    let corner = tip + egui::vec2(12.0, 16.0) * scale;
    let pill = ScreenRect::from_min_size(corner, text.size() + 2.0 * pad);
    painter.rect_filled(pill, pill.height() / 2.0, AGENT.gamma_multiply(fade));
    painter.galley(corner + pad, text, Color32::WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flashes_end_and_the_cursor_travels_then_leaves() {
        let start = Instant::now();
        let mut activity = Activity::default();
        assert!(activity.remaining(start).is_none());
        activity.flash(&["a".to_owned()], start);
        activity.point(Some(Rect::new(0.0, 0.0, 100.0, 50.0)), "Claude", start);
        // It appears where it first works.
        let cursor = activity.cursor.as_ref().unwrap();
        assert_eq!(cursor.place(start), Point::new(50.0, 25.0));
        // Sent on, it sets off from where it is and arrives.
        let later = start + Duration::from_millis(100);
        activity.point(Some(Rect::new(200.0, 0.0, 100.0, 50.0)), "Claude", later);
        let cursor = activity.cursor.as_ref().unwrap();
        assert_eq!(cursor.place(later), Point::new(50.0, 25.0));
        let halfway = cursor.place(later + AGENT_CURSOR_TRAVEL / 2).x;
        assert!(halfway > 150.0 && halfway < 250.0, "{halfway}");
        assert_eq!(cursor.place(later + AGENT_CURSOR_TRAVEL), Point::new(250.0, 25.0));
        // A call without a place leaves it where it is, and keeps it shown.
        activity.point(None, "Claude", later + AGENT_CURSOR_TRAVEL);
        assert_eq!(activity.cursor.as_ref().unwrap().to, Point::new(250.0, 25.0));
        // The same layer touched again flashes once, from the start.
        activity.flash(&["a".to_owned()], later);
        assert_eq!(activity.flashes.len(), 1);
        assert!(activity.remaining(later).is_some());
        assert!(activity.remaining(later + Duration::from_secs(60)).is_none());
    }

    fn turn(id: &str, what: What, x: f32) -> Turn {
        Turn { id: id.to_owned(), what, place: Some(Rect::new(x, 0.0, 100.0, 50.0)), scene: None }
    }

    #[test]
    fn a_change_is_shown_a_step_at_a_time() {
        let start = Instant::now();
        let mut activity = Activity::default();
        let first = Turn { scene: Some(Vec::new()), ..turn("a", What::Made, 0.0) };
        let last = Turn { scene: Some(Vec::new()), ..turn("old", What::Removed, 0.0) };
        activity.play(vec![first, turn("b", What::Made, 200.0), turn("c", What::Changed, 0.0), last], "Claude", start, 7);
        // New layers are left out until their turn; one that was there stays shown.
        assert!(activity.hidden().contains("a") && activity.hidden().contains("b") && !activity.hidden().contains("c"));
        assert!(activity.stage().0.is_none() && activity.stage().1 == 0);
        activity.advance(start, 7);
        assert!(!activity.hidden().contains("a") && activity.hidden().contains("b"));
        // The canvas shows the document as it was after the first operation.
        assert!(activity.stage().0.is_some() && activity.stage().1 == 1);
        assert_eq!(activity.flashes.len(), 1);
        assert_eq!(activity.cursor.as_ref().unwrap().to, Point::new(50.0, 25.0));
        activity.advance(start + AGENT_STEP, 7);
        assert!(!activity.hidden().contains("b"));
        assert_eq!(activity.stage().1, 1);
        assert_eq!(activity.cursor.as_ref().unwrap().to, Point::new(250.0, 25.0));
        // The layer that is gone flashes where it was; with the last step the document is shown as it is.
        activity.advance(start + AGENT_STEP * 3, 7);
        assert!(activity.hidden().is_empty() && activity.steps.is_empty());
        assert!(activity.flashes.len() == 4 && activity.flashes[3].ghost.is_some() && activity.flashes[2].ghost.is_none());
        assert!(activity.stage().0.is_none() && activity.stage().1 == 3);
        // The next call's steps wait for the ones before; many steps share the longest time a call may take.
        let many: Vec<Turn> = (0..1000).map(|n| turn(&format!("n{n}"), What::Made, 0.0)).collect();
        activity.play(vec![turn("a", What::Changed, 0.0)], "Claude", start, 8);
        activity.play(many, "Claude", start, 9);
        assert_eq!(activity.steps[1].at, start + AGENT_STEP);
        assert!(activity.steps.back().unwrap().at <= start + AGENT_STEP + AGENT_STEPS_MOST);
        activity.advance(start + AGENT_STEP + AGENT_STEPS_MOST, 9);
        assert!(activity.hidden().is_empty());
        // A document changed by someone else meanwhile is shown as it is at once.
        activity.play(vec![Turn { scene: Some(Vec::new()), ..turn("x", What::Made, 0.0) }, turn("y", What::Made, 0.0)], "Claude", start, 9);
        activity.advance(start, 9);
        assert!(activity.stage().0.is_some() && activity.hidden().contains("y"));
        activity.advance(start, 10);
        assert!(activity.stage().0.is_none() && activity.hidden().is_empty() && activity.steps.is_empty());
        // A call that touched nothing to show keeps the cursor where it is.
        activity.play(Vec::new(), "Claude", start, 10);
        assert!(activity.steps.is_empty());
    }
}
