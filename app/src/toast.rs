//! The message line: one short sentence shown over the bottom of the window for a few seconds (what a model
//! just changed, what a button could not do), on a capsule that opens out of a dot, changes its shape when the
//! next sentence comes, and closes into the dot again before it goes.

use std::time::{Duration, Instant};

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

use crate::theme::{
    self, AGENT, TOAST_CLOSING, TOAST_DOT, TOAST_FLASH, TOAST_GAP, TOAST_HEIGHT, TOAST_LEAVE, TOAST_LIFT, TOAST_MARGIN, TOAST_MORPH, TOAST_PADDING,
    TOAST_RISE, TOAST_TEXT, TOAST_TIME, TOAST_WIDTH,
};

/// The sentence shown.
struct Toast {
    /// Whose it is (a model's name), shown before it with a dot in the colour of a model at work; empty for
    /// the app's own.
    who: String,
    text: String,
    /// When this sentence was said.
    said: Instant,
    /// The size the capsule had when this sentence took the place of another; `None` when it came by itself.
    from: Option<Vec2>,
    /// The size it was last drawn at.
    size: Vec2,
}

/// What the capsule looks like at one moment.
#[derive(Debug, PartialEq)]
struct Look {
    size: Vec2,
    /// How much of the whole is seen (0 = nothing).
    alpha: f32,
    /// How much of the text is seen, on top of `alpha`.
    text: f32,
    /// How strong the flash is (1 = just said, 0 = over).
    flash: f32,
    /// How far under its place it is, points.
    sunk: f32,
}

/// `share` eased: fast away, slow into place.
fn eased(share: f32) -> f32 {
    1.0 - (1.0 - share.clamp(0.0, 1.0)).powi(3)
}

impl Toast {
    /// How it looks at `now`, when at rest it is `full` large.
    // Human: a sentence that comes by itself opens out of a dot as wide as one line's capsule is high, rising
    // a little and fading in; one that takes the place of another starts from that one's shape. The text
    // comes once the capsule is most of the way open, and a flash runs over it.
    // Human: when its time is over it goes the way it came: the text goes first, the capsule closes into the
    // dot again (the first `TOAST_CLOSING` of `TOAST_LEAVE`), and then the dot fades away.
    fn look(&self, full: Vec2, now: Instant) -> Look {
        let age = now.saturating_duration_since(self.said);
        let open = eased(age.as_secs_f32() / TOAST_MORPH.as_secs_f32());
        let dot = Vec2::splat(TOAST_HEIGHT.min(full.y));
        let start = self.from.unwrap_or(dot);
        let left = (self.said + TOAST_TIME).saturating_duration_since(now);
        let leaving = 1.0 - (left.as_secs_f32() / TOAST_LEAVE.as_secs_f32()).clamp(0.0, 1.0);
        let closed = eased(leaving / TOAST_CLOSING);
        let come = if self.from.is_some() { 1.0 } else { open };
        let opened = start + (full - start) * open;
        Look {
            size: opened + (dot - opened) * closed,
            alpha: (come * 3.0).min(1.0) * ((1.0 - leaving) / (1.0 - TOAST_CLOSING)).clamp(0.0, 1.0),
            text: ((open - 0.4) / 0.6).clamp(0.0, 1.0) * (1.0 - closed * 2.5).clamp(0.0, 1.0),
            flash: (1.0 - age.as_secs_f32() / TOAST_FLASH.as_secs_f32()).clamp(0.0, 1.0),
            sunk: (1.0 - come) * TOAST_RISE,
        }
    }
}

/// The message line of the window.
#[derive(Default)]
pub struct Toasts {
    now: Option<Toast>,
}

impl Toasts {
    /// Shows `text` from now on, in place of what is shown. `who` is the model that says it; empty for the app.
    pub fn say(&mut self, who: &str, text: &str, now: Instant) {
        // Human: while a sentence is still there, in whatever shape, the next one starts from that shape.
        let from = self.now.as_ref().filter(|toast| now.saturating_duration_since(toast.said) < TOAST_TIME).map(|toast| toast.size);
        self.now = Some(Toast { who: who.trim().to_owned(), text: text.trim().to_owned(), said: now, from, size: from.unwrap_or_default() });
    }

    /// Draws the sentence over everything else, while there is one.
    // Human: the sentence stays on one line while it fits `TOAST_WIDTH` (or the window), takes a second line
    // when it doesn't, and ends in "…" when two are not enough; the capsule is as wide as its text.
    // Agent: MAIN-THREAD, once a frame; asks for the next frame while something moves, and for the one it
    // starts to leave at otherwise.
    pub fn show(&mut self, ctx: &egui::Context, dark: bool) {
        let now = Instant::now();
        let Some(toast) = self.now.as_mut().filter(|toast| now.saturating_duration_since(toast.said) < TOAST_TIME) else {
            self.now = None;
            return;
        };
        let palette = theme::palette(dark);
        let screen = ctx.content_rect();
        let dot = if toast.who.is_empty() { 0.0 } else { TOAST_DOT + TOAST_GAP };
        let room = (TOAST_WIDTH.min(screen.width() - 2.0 * TOAST_MARGIN) - 2.0 * TOAST_PADDING - dot).max(TOAST_HEIGHT);
        let lay = |seen: f32| {
            let font = FontId::proportional(TOAST_TEXT);
            let mut job = LayoutJob::default();
            if !toast.who.is_empty() {
                job.append(
                    &toast.who,
                    0.0,
                    TextFormat { font_id: font.clone(), color: palette.foreground.gamma_multiply(seen), ..TextFormat::default() },
                );
                job.append("  ", 0.0, TextFormat { font_id: font.clone(), ..TextFormat::default() });
            }
            let color = if toast.who.is_empty() { palette.foreground } else { palette.muted };
            job.append(&toast.text, 0.0, TextFormat { font_id: font, color: color.gamma_multiply(seen), ..TextFormat::default() });
            job.wrap.max_width = room;
            job.wrap.max_rows = 2;
            job.wrap.overflow_character = Some('…');
            ctx.fonts_mut(|fonts| fonts.layout_job(job))
        };
        let measured = lay(1.0);
        let full = egui::vec2(measured.size().x + dot + 2.0 * TOAST_PADDING, (measured.size().y + TOAST_PADDING).max(TOAST_HEIGHT));
        let look = toast.look(full, now);
        toast.size = look.size;
        let centre = Pos2::new(screen.center().x, screen.bottom() - TOAST_LIFT - full.y / 2.0 + look.sunk);
        let capsule = Rect::from_center_size(centre, look.size);
        let radius = (look.size.y / 2.0).min(TOAST_HEIGHT / 2.0);
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("message")));
        let shadow = egui::epaint::Shadow { offset: [0, 6], blur: 22, spread: 0, color: Color32::BLACK.gamma_multiply(0.22 * look.alpha) };
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a radius of at most half the capsule's height")]
        painter.add(shadow.as_shape(capsule, egui::CornerRadius::same(radius as u8)));
        painter.rect(
            capsule,
            radius,
            palette.island.gamma_multiply(look.alpha),
            Stroke::new(1.0, palette.outline.gamma_multiply(look.alpha)),
            StrokeKind::Inside,
        );
        // What is on the capsule keeps its place while the capsule opens around it.
        let inside = painter.with_clip_rect(capsule.shrink2(egui::vec2(radius / 2.0, 0.0)));
        let seen = look.alpha * look.text;
        let content = Rect::from_center_size(centre, egui::vec2(full.x - 2.0 * TOAST_PADDING, measured.size().y));
        if dot > 0.0 {
            let first = measured.rows.first().map_or(measured.size().y, |row| row.height());
            inside.circle_filled(
                Pos2::new(content.left() + TOAST_DOT / 2.0, content.top() + first / 2.0),
                TOAST_DOT / 2.0,
                AGENT.gamma_multiply(seen),
            );
        }
        inside.galley(Pos2::new(content.left() + dot, content.top()), lay(seen), Color32::TRANSPARENT);
        flash(&painter, capsule, radius, if dot > 0.0 { AGENT } else { palette.foreground }, look.flash * look.alpha);
        let moving = look.flash > 0.0 || look.alpha < 1.0 || look.size != full;
        if moving {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(
                (toast.said + TOAST_TIME.saturating_sub(TOAST_LEAVE)).saturating_duration_since(now) + Duration::from_millis(1),
            );
        }
    }
}

/// The flash on a capsule just said or just changed, `strength` strong (1 = at its start): a wash of `color`
/// that fades, and a band of light running over it from left to right.
fn flash(painter: &egui::Painter, capsule: Rect, radius: f32, color: Color32, strength: f32) {
    if strength <= 0.0 {
        return;
    }
    painter.rect_filled(capsule, radius, color.gamma_multiply(0.22 * strength * strength));
    // The band stays between the round ends, so it never shows outside them.
    let track = capsule.shrink2(egui::vec2(radius, 1.0));
    let band = (track.width() * 0.4).max(1.0);
    let middle = track.left() - band / 2.0 + (track.width() + band) * (1.0 - strength);
    let bright = color.gamma_multiply(0.30 * strength);
    let mut mesh = egui::Mesh::default();
    for (x, tint) in [(middle - band / 2.0, Color32::TRANSPARENT), (middle, bright), (middle + band / 2.0, Color32::TRANSPARENT)] {
        mesh.colored_vertex(Pos2::new(x, track.top()), tint);
        mesh.colored_vertex(Pos2::new(x, track.bottom()), tint);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    mesh.add_triangle(2, 3, 4);
    mesh.add_triangle(3, 4, 5);
    painter.with_clip_rect(track.intersect(painter.clip_rect())).add(egui::Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sentence_opens_out_of_a_dot_closes_into_it_and_the_next_takes_its_shape() {
        let start = Instant::now();
        let full = egui::vec2(300.0, TOAST_HEIGHT);
        let mut toasts = Toasts::default();
        toasts.say("", " Saved ", start);
        let toast = toasts.now.as_ref().unwrap();
        assert_eq!(toast.text, "Saved");
        // It starts as a dot under its place, unseen, with the flash at its strongest.
        let first = toast.look(full, start);
        assert_eq!((first.size, first.alpha, first.text, first.flash), (Vec2::splat(TOAST_HEIGHT), 0.0, 0.0, 1.0));
        assert!((first.sunk - TOAST_RISE).abs() < 0.01);
        // Open, it is at rest: whole, in place, the flash over.
        let rest = toast.look(full, start + TOAST_MORPH.max(TOAST_FLASH));
        assert_eq!(rest, Look { size: full, alpha: 1.0, text: 1.0, flash: 0.0, sunk: 0.0 });
        // At the end the text goes, the capsule closes into the dot again, and then the dot fades.
        let closing = toast.look(full, start + TOAST_TIME.saturating_sub(TOAST_LEAVE.mul_f32(1.0 - TOAST_CLOSING / 2.0)));
        assert!(closing.size.x < full.x && closing.size.x > TOAST_HEIGHT && closing.alpha > 0.99 && closing.text < 0.01, "{closing:?}");
        let dot = toast.look(full, start + TOAST_TIME.saturating_sub(TOAST_LEAVE.mul_f32((1.0 - TOAST_CLOSING) / 2.0)));
        assert!(dot.size == Vec2::splat(TOAST_HEIGHT) && dot.alpha > 0.4 && dot.alpha < 0.6 && dot.sunk == 0.0, "{dot:?}");
        // The next sentence starts from the shape the capsule has, already seen and in place, and flashes again.
        toasts.now.as_mut().unwrap().size = full;
        let later = start + Duration::from_secs(1);
        toasts.say("Claude", "Made the header", later);
        let next = toasts.now.as_ref().unwrap();
        let wider = egui::vec2(400.0, TOAST_HEIGHT);
        let turned = next.look(wider, later);
        assert_eq!((turned.size, turned.alpha, turned.flash, turned.sunk), (full, 1.0, 1.0, 0.0));
        assert_eq!(next.look(wider, later + TOAST_MORPH).size, wider);
        // One that comes while the last is closing starts from the shape it has; after that it comes by itself.
        toasts.say("", "Later", later + TOAST_TIME.saturating_sub(TOAST_LEAVE / 2));
        assert!(toasts.now.as_ref().unwrap().from.is_some());
        toasts.say("", "Much later", later + TOAST_TIME * 3);
        assert!(toasts.now.as_ref().unwrap().from.is_none());
    }
}
