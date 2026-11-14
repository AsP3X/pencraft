//! What an AI model working on a document looks like on the canvas: every layer it touches flashes, and a
//! cursor with the model's name moves to where it works.

use std::time::{Duration, Instant};

use egui::{Color32, Pos2, Rect as ScreenRect, Stroke};
use pen_doc::geom::{Point, Rect};

use crate::editor::Editor;
use crate::render::screen;
use crate::theme::{
    AGENT, AGENT_CURSOR, AGENT_CURSOR_FADE, AGENT_CURSOR_STAY, AGENT_CURSOR_TRAVEL, AGENT_FLASH_GROW, AGENT_FLASH_STROKE, AGENT_FLASH_TIME,
    AGENT_LABEL_TEXT,
};

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
    /// The layers flashing (by scene id), and since when.
    flashes: Vec<(String, Instant)>,
    cursor: Option<Cursor>,
    /// A picture of the app (`--activity`) shows the selected layers as a model works on them.
    pub posed: bool,
}

impl Activity {
    /// Lets the layers `ids` flash, starting now.
    pub fn flash(&mut self, ids: &[String], now: Instant) {
        self.flashes.retain(|(id, since)| now.saturating_duration_since(*since) < AGENT_FLASH_TIME && !ids.contains(id));
        self.flashes.extend(ids.iter().map(|id| (id.clone(), now)));
    }

    /// Sends the cursor to the middle of `target` (it stays where it is without one) under the name `label`.
    // Human: the cursor appears where it first works, and from then on travels from where it is.
    pub fn point(&mut self, target: Option<Rect>, label: &str, now: Instant) {
        let from =
            self.cursor.as_ref().filter(|c| now.saturating_duration_since(c.seen) < AGENT_CURSOR_STAY + AGENT_CURSOR_FADE).map(|c| c.place(now));
        let Some(to) = target.map(|b| b.center()).or(from) else { return };
        self.cursor = Some(Cursor { from: from.unwrap_or(to), to, moved: now, seen: now, label: label.to_owned() });
    }

    /// How long until nothing of this is left to show, from `now`; `None` when nothing is showing.
    fn remaining(&self, now: Instant) -> Option<Duration> {
        let flash = self.flashes.iter().map(|(_, since)| (*since + AGENT_FLASH_TIME).saturating_duration_since(now)).max();
        let cursor = self.cursor.as_ref().map(|c| (c.seen + AGENT_CURSOR_STAY + AGENT_CURSOR_FADE).saturating_duration_since(now));
        flash.max(cursor).filter(|left| !left.is_zero())
    }

    /// Draws the flashes and the cursor over the canvas. `scale` is the cursor's size (1 = small).
    // Agent: MAIN-THREAD; asks for the next frame while something is moving.
    pub fn draw(&self, ui: &egui::Ui, painter: &egui::Painter, editor: &Editor, view: ScreenRect, scale: f32) {
        let now = Instant::now();
        if self.remaining(now).is_none() {
            return;
        }
        ui.ctx().request_repaint();
        let cam = &editor.camera;
        for (id, since) in &self.flashes {
            let t = now.saturating_duration_since(*since).as_secs_f32() / AGENT_FLASH_TIME.as_secs_f32();
            if let (true, Some(node)) = (t < 1.0, editor.scene_node(id)) {
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
}
