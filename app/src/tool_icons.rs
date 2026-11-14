//! The moving pictures of the assistant's tools at work, beside their notes in the chat.
//!
//! Reading is a book whose pages turn, looking at the design is an eye whose pupil wanders (and that blinks),
//! and changing the design is a pencil that writes a line. Each is drawn here line by line, in the same
//! 24-unit box and line width as the bundled icons, so it sits among them as one of them.

use egui::{Color32, Pos2, Rect, Stroke, vec2};

use crate::theme::{ICON_LINE, TOOL_BLINK_EVERY, TOOL_BLINK_TIME, TOOL_PAGE_TIME, TOOL_WRITE_TIME};

/// What a tool does, as far as its moving picture goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// It reads: a book, its pages turning.
    Reading,
    /// It looks at the design: an eye, its pupil moving.
    Looking,
    /// It changes the design: a pencil, writing.
    Changing,
}

/// The side of the box the pictures are drawn in, as the bundled icons are.
const BOX: f32 = 24.0;
/// The bundled pencil icon's tip, in its own box.
const PENCIL_TIP: [f32; 2] = [2.6, 21.4];
/// How large the pencil is drawn, so that it has room to move.
const PENCIL_SIZE: f32 = 0.7;

/// 0 to 1, slow at both ends.
fn eased(share: f32) -> f32 {
    let share = share.clamp(0.0, 1.0);
    share * share * (3.0 - 2.0 * share)
}

/// The share of the way through a loop of `seconds` at `now`.
#[expect(clippy::cast_possible_truncation, reason = "a share between 0 and 1")]
fn looped(now: f64, seconds: f32) -> f32 {
    (now / f64::from(seconds)).rem_euclid(1.0) as f32
}

/// The book's covers, lying open: the left one, the right one, each from the spine's top around to its foot.
const COVERS: [[[f32; 2]; 4]; 2] = [[[12.0, 7.5], [3.0, 6.0], [3.0, 18.0], [12.0, 19.5]], [[12.0, 7.5], [21.0, 6.0], [21.0, 18.0], [12.0, 19.5]]];

/// A page `share` of the way from lying on the right cover (0) to lying on the left one (1): from the spine's
/// top, along its top edge, down its outer edge, back to the spine's foot. On the way it stands up.
fn page(share: f32) -> [[f32; 2]; 4] {
    let turned = eased(share) * std::f32::consts::PI;
    let (reach, lift) = (9.0 * turned.cos(), 3.0 * turned.sin());
    [[12.0, 7.5], [12.0 + reach, 6.0 - lift], [12.0 + reach, 18.0 - lift], [12.0, 19.5]]
}

/// How far open the eye is at `now`: open, but for a quick blink every few seconds.
#[expect(clippy::cast_possible_truncation, reason = "a share between 0 and 1")]
fn open(now: f64) -> f32 {
    let since = now.rem_euclid(f64::from(TOOL_BLINK_EVERY)) as f32;
    if since >= TOOL_BLINK_TIME {
        return 1.0;
    }
    1.0 - 0.92 * (since / TOOL_BLINK_TIME * std::f32::consts::PI).sin()
}

/// Where the middle of the pupil is at `now`: it wanders from side to side, and a little up and down.
#[expect(clippy::cast_possible_truncation, reason = "waves between -1 and 1")]
fn pupil(now: f64) -> [f32; 2] {
    [12.0 + 3.4 * (now * 2.3).sin() as f32, 12.0 + 0.9 * (now * 1.7 + 1.0).sin() as f32]
}

/// A lid of the eye, `open` wide: the upper one (`up`) or the lower, from the left corner to the right.
fn lid(open: f32, up: bool) -> Vec<[f32; 2]> {
    (0..=12_u8)
        .map(|step| {
            let along = f32::from(step) / 12.0;
            let bulge = 6.5 * open * (along * std::f32::consts::PI).sin();
            [2.0 + 20.0 * along, if up { 12.0 - bulge } else { 12.0 + bulge }]
        })
        .collect()
}

/// The share of a loop the pencil writes for; the rest of it lifts and goes back to the start.
const WRITING: f32 = 0.72;

/// Where the pencil's tip is `share` of the way along the line it writes: left to right, in small waves.
fn written(share: f32) -> [f32; 2] {
    [3.0 + 8.0 * share, 20.4 - 1.1 * (share * std::f32::consts::TAU * 1.5).sin()]
}

/// The pencil `share` of the way through its loop: where its tip is, how much of the line is written, and how
/// much of that line still shows (it fades while the pencil goes back).
fn pencil(share: f32) -> ([f32; 2], f32, f32) {
    if share < WRITING {
        let along = share / WRITING;
        return (written(along), along, 1.0);
    }
    let back = eased((share - WRITING) / (1.0 - WRITING));
    let [from_x, from_y] = written(1.0);
    let [to_x, to_y] = written(0.0);
    let lifted = 3.0 * (back * std::f32::consts::PI).sin();
    ([from_x + (to_x - from_x) * back, from_y + (to_y - from_y) * back - lifted], 1.0, 1.0 - back)
}

/// Draws the tool doing `act` at work in `slot`, as it looks at `now` (egui's time, in seconds).
pub fn paint(painter: &egui::Painter, slot: Rect, act: Act, now: f64, color: Color32) {
    let unit = slot.width() / BOX;
    let at = |[x, y]: [f32; 2]| -> Pos2 { slot.min + vec2(x, y) * unit };
    let stroke = Stroke::new(ICON_LINE, color);
    let line = |points: &[[f32; 2]], stroke: Stroke| painter.add(egui::Shape::line(points.iter().copied().map(at).collect(), stroke));
    match act {
        Act::Reading => {
            for cover in &COVERS {
                line(cover, stroke);
            }
            line(&[[12.0, 7.5], [12.0, 19.5]], stroke);
            // Human: two pages are on their way at any time, half a turn apart, so the book never rests.
            let share = looped(now, TOOL_PAGE_TIME);
            for share in [share, (share + 0.5).fract()] {
                // A page is faint as it leaves one cover and as it lands on the other.
                let strength = (share * std::f32::consts::PI).sin().max(0.0);
                line(&page(share), Stroke::new(ICON_LINE, color.gamma_multiply(0.35 + 0.65 * strength)));
            }
        }
        Act::Looking => {
            let open = open(now);
            line(&lid(open, true), stroke);
            line(&lid(open, false), stroke);
            // A closing eye hides its pupil.
            if open > 0.45 {
                let middle = at(pupil(now));
                painter.circle_stroke(middle, 3.0 * unit, stroke);
                painter.circle_filled(middle, 1.2 * unit, color);
            }
        }
        Act::Changing => {
            let (tip, along, showing) = pencil(looped(now, TOOL_WRITE_TIME));
            let trace: Vec<[f32; 2]> = (0..=16_u8).map(|step| written(along * f32::from(step) / 16.0)).collect();
            line(&trace, Stroke::new(ICON_LINE, color.gamma_multiply(0.75 * showing)));
            // The bundled pencil, placed so that its tip is where the line is being written.
            let size = slot.size() * PENCIL_SIZE;
            let corner = at(tip) - vec2(PENCIL_TIP[0], PENCIL_TIP[1]) * unit * PENCIL_SIZE;
            crate::icons::paint(painter, Rect::from_min_size(corner, size), "pencil", color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3
    }

    #[test]
    fn a_page_goes_from_the_right_cover_over_the_top_to_the_left_one() {
        assert_eq!(page(0.0).map(|[x, y]| [x.round(), y.round()]), COVERS[1].map(|[x, y]| [x.round(), y.round()]));
        assert!(close(page(1.0)[1], COVERS[0][1]) && close(page(1.0)[2], COVERS[0][2]));
        // Half way it stands on the spine, above the covers.
        let standing = page(0.5);
        assert!((standing[1][0] - 12.0).abs() < 1e-3 && standing[1][1] < 4.0);
        // It never leaves the box.
        for step in 0..=20_u8 {
            assert!(page(f32::from(step) / 20.0).iter().all(|[x, y]| (0.0..=BOX).contains(x) && (0.0..=BOX).contains(y)));
        }
    }

    #[test]
    fn the_eye_is_open_but_for_a_blink_and_its_pupil_stays_between_the_lids() {
        assert!(open(f64::from(TOOL_BLINK_TIME) / 2.0) < 0.2);
        assert!((open(f64::from(TOOL_BLINK_TIME) + 0.5) - 1.0).abs() < f32::EPSILON);
        assert!((open(0.0) - 1.0).abs() < 1e-3);
        for tenth in 0..200_u8 {
            let [x, y] = pupil(f64::from(tenth) / 10.0);
            assert!((8.0..=16.0).contains(&x) && (10.5..=13.5).contains(&y), "{x} {y}");
        }
        // The lids meet at the corners and are widest in the middle.
        let upper = lid(1.0, true);
        assert!(close(upper[0], [2.0, 12.0]) && close(upper[12], [22.0, 12.0]) && (upper[6][1] - 5.5).abs() < 1e-3);
        assert!(lid(0.0, false).iter().all(|[_, y]| (y - 12.0).abs() < 1e-3));
    }

    #[test]
    fn the_pencil_writes_a_line_then_lifts_and_goes_back() {
        let (start, none, _) = pencil(0.0);
        assert!(close(start, written(0.0)) && none.abs() < f32::EPSILON);
        let (end, all, showing) = pencil(WRITING - 1e-4);
        assert!((end[0] - 11.0).abs() < 0.01 && all > 0.99 && (showing - 1.0).abs() < f32::EPSILON);
        // Going back it is off the paper, and the line fades.
        let (lifted, _, fading) = pencil(WRITING + (1.0 - WRITING) / 2.0);
        assert!(lifted[1] < 18.5 && fading < 0.6 && fading > 0.4);
        let (back, _, gone) = pencil(1.0);
        assert!(close(back, written(0.0)) && gone.abs() < 1e-3);
        // The whole pencil stays within reach of its box.
        assert!(written(1.0)[0] + (BOX - PENCIL_TIP[0]) * PENCIL_SIZE <= BOX + 2.5);
    }
}
