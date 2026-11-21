//! What the welcome film is drawn with: a pen that works in the stage's own points, and the curves its
//! movements follow.

use std::sync::Arc;

use egui::epaint::{Mesh, PathShape, PathStroke, RectShape, Shadow, Vertex};
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontFamily, FontId, Galley, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, TextureHandle, Vec2, pos2, vec2};

use crate::render::BOLD_FAMILY;

/// The family of the page's first headline.
pub const SERIF: &str = "Georgia";
/// The family its headline is changed to.
pub const HEAVY: &str = "Arial Black";

/// The face a piece of the film's text is set in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// The interface's face.
    Regular,
    /// Its bold.
    Bold,
    /// [`SERIF`] in bold; the interface's bold where the computer has no such face.
    Serif,
    /// [`HEAVY`]; the interface's bold where the computer has no such face.
    Heavy,
}

impl Face {
    /// The egui family to draw with.
    fn family(self) -> FontFamily {
        let bold = || FontFamily::Name(BOLD_FAMILY.into());
        match self {
            Face::Regular => FontFamily::Proportional,
            Face::Bold => bold(),
            Face::Serif => crate::fonts::choose(SERIF, 700, false).map_or_else(bold, |chosen| chosen.family),
            Face::Heavy => crate::fonts::choose(HEAVY, 900, false).map_or_else(bold, |chosen| chosen.family),
        }
    }

    /// How far above its baseline the face's line begins, as a share of its size.
    // Agent: the ascent of Arial, Georgia and Arial Black; text is placed by its baseline, as in the browser film.
    fn ascent(self) -> f32 {
        match self {
            Face::Regular | Face::Bold => 0.905,
            Face::Serif => 0.917,
            Face::Heavy => 1.1,
        }
    }
}

/// `value` held between 0 and 1.
#[must_use]
pub fn held(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

/// How far `time` is through the `length` seconds that start at `from`, 0 to 1.
#[must_use]
pub fn through(time: f32, from: f32, length: f32) -> f32 {
    held((time - from) / length)
}

/// Fast away, slow into place.
#[must_use]
pub fn out(share: f32) -> f32 {
    1.0 - (1.0 - held(share)).powi(3)
}

/// The same, with a harder start and a longer landing.
#[must_use]
pub fn out_hard(share: f32) -> f32 {
    1.0 - (1.0 - held(share)).powi(5)
}

/// Slow away, fast at the end: for what leaves.
#[must_use]
pub fn into(share: f32) -> f32 {
    held(share).powi(3)
}

/// Slow at both ends: for what travels.
#[must_use]
pub fn both(share: f32) -> f32 {
    let share = held(share);
    if share < 0.5 { 4.0 * share.powi(3) } else { 1.0 - (-2.0 * share + 2.0).powi(3) / 2.0 }
}

/// Past its place by a little, then back.
#[must_use]
pub fn past(share: f32) -> f32 {
    let share = held(share) - 1.0;
    1.0 + 2.7 * share.powi(3) + 1.7 * share.powi(2)
}

/// `from` to `to` by `share`.
#[must_use]
pub fn mix(from: f32, to: f32, share: f32) -> f32 {
    from + (to - from) * share
}

/// The point at `share` of the curve through `points` (start, two handles, end).
#[must_use]
pub fn curve(points: [[f32; 2]; 4], share: f32) -> [f32; 2] {
    let rest = 1.0 - share;
    let weights = [rest.powi(3), 3.0 * rest.powi(2) * share, 3.0 * rest * share.powi(2), share.powi(3)];
    let along = |axis: usize| points.iter().zip(weights).map(|(point, weight)| point[axis] * weight).sum();
    [along(0), along(1)]
}

/// Where a pointer is at `time`: it glides from each of `stops` (seconds, x, y) to the next.
#[must_use]
pub fn glide(stops: &[[f32; 3]], time: f32) -> [f32; 2] {
    let Some(first) = stops.first() else { return [0.0, 0.0] };
    if time <= first[0] {
        return [first[1], first[2]];
    }
    for pair in stops.windows(2) {
        if let [from, to] = pair
            && time < to[0]
        {
            let share = both((time - from[0]) / (to[0] - from[0]));
            return [mix(from[1], to[1], share), mix(from[2], to[2], share)];
        }
    }
    stops.last().map_or([0.0, 0.0], |last| [last[1], last[2]])
}

/// A space with its own points: where its origin is in the window, and how large one of its points is there.
#[derive(Clone, Copy, Debug)]
pub struct View {
    /// The window's place of this space's (0, 0).
    pub origin: Pos2,
    /// Window points to one point of this space.
    pub scale: f32,
}

impl View {
    /// The window's place of this space's (`x`, `y`).
    #[must_use]
    pub fn at(self, x: f32, y: f32) -> Pos2 {
        self.origin + vec2(x, y) * self.scale
    }

    /// The window's rectangle of this space's.
    #[must_use]
    pub fn rect(self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.at(x, y), vec2(w.max(0.0), h.max(0.0)) * self.scale)
    }
}

/// Draws in a [`View`]'s points.
pub struct Pen {
    painter: Painter,
    /// The space it draws in.
    pub view: View,
}

impl Pen {
    /// A pen for `view` on `painter`.
    #[must_use]
    pub fn new(painter: Painter, view: View) -> Pen {
        Pen { painter, view }
    }

    /// A pen for a space inside this one: its (0, 0) at (`x`, `y`), its points `scale` of these.
    #[must_use]
    pub fn inside(&self, x: f32, y: f32, scale: f32) -> Pen {
        Pen { painter: self.painter.clone(), view: View { origin: self.view.at(x, y), scale: self.view.scale * scale } }
    }

    /// The same pen, drawing only inside the rectangle.
    #[must_use]
    pub fn cut(&self, x: f32, y: f32, w: f32, h: f32) -> Pen {
        let cut = self.view.rect(x, y, w, h).intersect(self.painter.clip_rect());
        Pen { painter: self.painter.with_clip_rect(cut), view: self.view }
    }

    /// The same pen, drawing `share` as strongly.
    #[must_use]
    pub fn faded(&self, share: f32) -> Pen {
        let mut painter = self.painter.clone();
        painter.multiply_opacity(held(share));
        Pen { painter, view: self.view }
    }

    /// A filled rectangle with corners rounded by `radius`.
    pub fn fill(&self, [x, y, w, h]: [f32; 4], radius: f32, color: Color32) {
        self.painter.rect_filled(self.view.rect(x, y, w, h), radius.min(w / 2.0).min(h / 2.0).max(0.0) * self.view.scale, color);
    }

    /// The outline of one, `width` wide in window points.
    pub fn outline(&self, [x, y, w, h]: [f32; 4], radius: f32, color: Color32, width: f32) {
        let radius = radius.min(w / 2.0).min(h / 2.0).max(0.0) * self.view.scale;
        self.painter.rect_stroke(self.view.rect(x, y, w, h), radius, Stroke::new(width, color), StrokeKind::Inside);
    }

    /// The soft shadow a card throws downwards.
    pub fn shadow(&self, [x, y, w, h]: [f32; 4], radius: f32) {
        let shadow = Shadow { offset: [0, 14], blur: 36, spread: 0, color: Color32::from_black_alpha(52) };
        self.painter.add(shadow.as_shape(self.view.rect(x, y, w, h), radius * self.view.scale));
    }

    /// A filled circle.
    pub fn dot(&self, x: f32, y: f32, radius: f32, color: Color32) {
        self.painter.circle_filled(self.view.at(x, y), radius.max(0.0) * self.view.scale, color);
    }

    /// A circle's outline, `width` wide in this space's points.
    pub fn ring(&self, x: f32, y: f32, radius: f32, color: Color32, width: f32) {
        self.painter.circle_stroke(self.view.at(x, y), radius * self.view.scale, Stroke::new(width * self.view.scale, color));
    }

    /// A line through `points`, `width` wide in this space's points, with round ends.
    pub fn line(&self, points: &[[f32; 2]], color: Color32, width: f32) {
        let placed: Vec<Pos2> = points.iter().map(|point| self.view.at(point[0], point[1])).collect();
        let width = width * self.view.scale;
        for end in placed.first().into_iter().chain(placed.last()) {
            self.painter.circle_filled(*end, width / 2.0, color);
        }
        self.painter.add(Shape::line(placed, PathStroke::new(width, color)));
    }

    /// A filled shape with no dents, through `points`.
    pub fn shape(&self, points: &[[f32; 2]], color: Color32, edge: Stroke) {
        let placed = points.iter().map(|point| self.view.at(point[0], point[1])).collect();
        self.painter.add(PathShape::convex_polygon(placed, color, edge));
    }

    /// One of the app's icons, `size` large with its middle at (`x`, `y`).
    pub fn icon(&self, name: &'static str, x: f32, y: f32, size: f32, color: Color32) {
        crate::icons::paint(&self.painter, Rect::from_center_size(self.view.at(x, y), Vec2::splat(size * self.view.scale)), name, color);
    }

    /// `text` laid out in `face` at `size`, with `spacing` more between its letters.
    fn laid(&self, text: &str, size: f32, face: Face, color: Color32, spacing: f32) -> Arc<Galley> {
        let format = TextFormat {
            font_id: FontId::new(size * self.view.scale, face.family()),
            color,
            extra_letter_spacing: spacing * self.view.scale,
            ..TextFormat::default()
        };
        self.painter.layout_job(LayoutJob::single_section(text.to_owned(), format))
    }

    /// How wide `text` is in this space's points.
    #[must_use]
    pub fn width(&self, text: &str, size: f32, face: Face, spacing: f32) -> f32 {
        if self.view.scale <= 0.0 {
            return 0.0;
        }
        self.laid(text, size, face, Color32::WHITE, spacing).size().x / self.view.scale
    }

    /// Writes `text` with its first letter's left edge at `x` and its baseline at `y`. Returns how wide it is.
    pub fn text(&self, text: &str, [x, y]: [f32; 2], [size, spacing]: [f32; 2], face: Face, color: Color32) -> f32 {
        let laid = self.laid(text, size, face, color, spacing);
        let width = laid.size().x / self.view.scale.max(f32::EPSILON);
        self.painter.galley(self.view.at(x, y - size * face.ascent()), laid, color);
        width
    }

    /// Writes `text` with its middle at `x` and its baseline at `y`.
    pub fn centred(&self, text: &str, [x, y]: [f32; 2], size: f32, face: Face, color: Color32) {
        let width = self.width(text, size, face, 0.0);
        self.text(text, [x - width / 2.0, y], [size, 0.0], face, color);
    }

    /// A picture filling the rectangle, its corners rounded by `radius`.
    pub fn picture(&self, picture: Option<&TextureHandle>, [x, y, w, h]: [f32; 4], radius: f32) {
        let Some(picture) = picture else { return };
        let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        let shape = RectShape::filled(self.view.rect(x, y, w, h), radius * self.view.scale, Color32::WHITE).with_texture(picture.id(), whole);
        self.painter.add(shape);
    }

    /// A picture `size` large, turned by `angle` about its middle at (`x`, `y`), with a thin edge.
    pub fn turned(&self, picture: Option<&TextureHandle>, [x, y]: [f32; 2], size: [f32; 2], angle: f32, edge: Color32) {
        let (sin, cos) = angle.sin_cos();
        let corner = |right: f32, down: f32| {
            let (across, along) = (right * size[0] / 2.0, down * size[1] / 2.0);
            self.view.at(x + across * cos - along * sin, y + across * sin + along * cos)
        };
        let corners = [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)];
        if let Some(picture) = picture {
            let mut mesh = Mesh::with_texture(picture.id());
            for (place, uv) in corners.into_iter().zip([pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(1.0, 1.0), pos2(0.0, 1.0)]) {
                mesh.vertices.push(Vertex { pos: place, uv, color: Color32::WHITE });
            }
            mesh.indices.extend([0, 1, 2, 0, 2, 3]);
            self.painter.add(Shape::mesh(mesh));
        }
        self.painter.add(Shape::closed_line(corners.to_vec(), PathStroke::new(1.0, edge)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_curve_starts_at_nothing_and_ends_in_place() {
        for curve in [out, out_hard, into, both, past] {
            assert!(curve(0.0).abs() < 1e-5);
            assert!((curve(1.0) - 1.0).abs() < 1e-5);
            assert!((curve(7.0) - 1.0).abs() < 1e-5, "held past its end");
        }
        assert!(past(0.7) > 1.0, "goes past its place before it settles");
    }

    #[test]
    fn a_pointer_waits_at_its_first_stop_glides_and_stays_at_its_last() {
        let stops = [[1.0, 0.0, 0.0], [2.0, 10.0, 20.0]];
        assert_eq!(glide(&stops, 0.0), [0.0, 0.0]);
        assert_eq!(glide(&stops, 1.5), [5.0, 10.0]);
        assert_eq!(glide(&stops, 9.0), [10.0, 20.0]);
        assert_eq!(glide(&[], 1.0), [0.0, 0.0]);
    }

    #[test]
    fn a_curve_runs_from_its_start_to_its_end() {
        let points = [[0.0, 0.0], [0.0, 10.0], [10.0, 10.0], [10.0, 0.0]];
        assert_eq!(curve(points, 0.0), [0.0, 0.0]);
        assert_eq!(curve(points, 1.0), [10.0, 0.0]);
        assert_eq!(curve(points, 0.5), [5.0, 7.5]);
    }

    #[test]
    fn a_view_places_its_points_in_the_window() {
        let view = View { origin: pos2(100.0, 50.0), scale: 2.0 };
        assert_eq!(view.at(10.0, 20.0), pos2(120.0, 90.0));
        assert_eq!(view.rect(0.0, 0.0, 5.0, -3.0), Rect::from_min_size(pos2(100.0, 50.0), vec2(10.0, 0.0)));
    }
}
