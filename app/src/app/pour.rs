//! A new tab pours out of the title bar's "new file" button, and a closed one drains away.
//!
//! A new tab is not simply there. The button swells into a drop of the tab's own colour and shoots off to the
//! right, to where it will stand. Behind it the tab's body follows more slowly, a round swell at its front, and
//! between the two a thread of liquid is drawn out. The thread thins, breaks into two beads and lets go; the
//! drop at the button and the beads are gone; the body reaches its width, its swell sinks into it, and it
//! settles into the tab's shape while its name slides in. The liquid is a shade stronger while it is fresh,
//! and a light runs along its rim once.
//!
//! A closed tab goes another way, and quickly: its name is gone at once, it draws itself together into one
//! round bead, as a drop does on a surface that sheds it, and the bead bursts into two small drops that fly
//! apart and are gone.
//!
//! This module says where the liquid is at each moment ([`shapes`], [`beading`]) and shows the picture of it
//! ([`Pours::paint`]); [`crate::liquid::plain`] paints the picture.

use std::time::Instant;

use egui::{Color32, Pos2, Rect, TextureHandle, TextureOptions, Ui, pos2, vec2};

use crate::liquid::{self, Slab};
use crate::theme::{
    CONTROL_RADIUS, Palette, TAB_DRAIN_BEAD, TAB_DRAIN_SPRAY, TAB_DRAIN_TIME, TAB_POUR_BEAD, TAB_POUR_DROP, TAB_POUR_MELT, TAB_POUR_ROOM,
    TAB_POUR_SLIDE, TAB_POUR_THREAD, TAB_POUR_TIME,
};

/// 0 below `from`, 1 above `to`, and a smooth rise between.
fn eased(from: f32, to: f32, value: f32) -> f32 {
    let share = ((value - from) / (to - from)).clamp(0.0, 1.0);
    share * share * (3.0 - 2.0 * share)
}

/// A swing that dies away: 1 at the start, through nothing and a little past it, and at rest by the end.
/// `pace` is how fast it swings and `rest` how fast it dies.
fn swing(share: f32, pace: f32, rest: f32) -> f32 {
    if share >= 1.0 { 0.0 } else { (-rest * share.max(0.0)).exp() * (pace * share.max(0.0)).cos() * (1.0 - share.max(0.0).powi(4)) }
}

/// How a tab that pours in or drains away is drawn at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// How wide it is, as a share of its full width; past 1 for a moment while a new tab springs back.
    pub width: f32,
    /// How strongly its icon and name show, 0 to 1.
    pub shown: f32,
    /// How far its icon and name are from their place, in points to the right.
    pub slide: f32,
}

/// A tab at `share` of pouring in: its place runs past its width and springs back, and its icon and name slide
/// in from the left once the body is mostly there.
#[must_use]
pub fn pouring(share: f32) -> Look {
    let shown = eased(0.5, 0.92, share);
    Look { width: 1.0 - swing(share, 9.0, 6.5), shown, slide: -TAB_POUR_SLIDE * (1.0 - shown) }
}

/// A tab at `share` of draining away: its icon and name go first, and its place closes smoothly.
#[must_use]
pub fn draining(share: f32) -> Look {
    Look { width: 1.0 - eased(0.0, 0.8, share), shown: 1.0 - eased(0.0, 0.2, share), slide: 0.0 }
}

/// The liquid of a tab at `share` of draining away, over `tab` (as wide as its place is at this moment): the
/// body drawing itself together into a bead, the two drops the bead bursts into, and how far apart they still
/// melt into each other.
// Human: surface tension, played fast. The body's corners go round and it gets lower as it gets narrower, until
// it is one round bead at the left of its place. In the second half the bead shrinks and throws a small drop to
// each side; the drops fly apart, let go of the bead, and all three are gone.
#[must_use]
pub fn beading(share: f32, tab: Rect) -> ([Slab; 3], f32) {
    let share = share.clamp(0.0, 1.0);
    let burst = eased(0.5, 1.0, share);
    let bead = TAB_DRAIN_BEAD * (1.0 - burst.powi(2));
    let high = tab.height() / 2.0;
    let half = [(tab.width() / 2.0).max(bead), high + (bead - high) * eased(0.1, 0.6, share)];
    let (round, square) = (half[0].min(half[1]), f32::from(CONTROL_RADIUS).min(half[0].min(half[1])));
    let centre = [tab.left() + half[0], tab.center().y];
    let body = Slab { centre, half, corner: square + (round - square) * eased(0.0, 0.3, share) };
    let spray = TAB_DRAIN_SPRAY * (std::f32::consts::PI * burst).sin();
    let away = bead + 3.0 + 20.0 * burst;
    let drop = |side: f32| Slab { centre: [centre[0] + side * away, centre[1]], half: [spray, spray], corner: spray };
    ([body, drop(-1.0), drop(1.0)], TAB_POUR_BEAD * 3.0 * (1.0 - burst))
}

/// How much of its place the tab's body fills at `share` of the pour: the body follows the button, which
/// shoots ahead, and catches up with it.
fn caught_up(share: f32) -> f32 {
    0.5 + 0.5 * eased(0.2, 0.85, share)
}

/// How much stronger than a tab's own colour the liquid is at `share` of the pour, 0 to 1 of the way to the
/// title bar's text colour: fresh liquid is stronger, and it thins to the tab's colour.
#[must_use]
pub fn freshness(share: f32) -> f32 {
    0.2 * (1.0 - eased(0.0, 0.75, share))
}

/// The liquid at `share` of the pour, over the place `tab` (as wide as it is at this moment): the body, the
/// swell at its front, the thread drawn out behind the button, the drop at the button, whose middle is `head`,
/// and the two beads the thread breaks into; and how far apart they still melt into each other. Without a
/// button to pour from there are the body and its swell alone, filling the place.
// Human: the place opens fast, and the button with it. The body follows: half the place at first, all of it by
// the end, thin while it is pulled and round at its corners until it settles. Its front is a round swell that
// sinks into it. What lies between the body and the drop at the button is the thread: as thick as a finger at
// first, thinner as it is drawn out, gone before the body arrives; where it was, two beads show for a moment.
// The drop at the button is drawn out sideways and shrinks to nothing.
#[must_use]
pub fn shapes(share: f32, tab: Rect, head: Option<Pos2>) -> ([Slab; 6], f32) {
    let share = share.clamp(0.0, 1.0);
    let (y, high) = (tab.center().y, tab.height() / 2.0);
    let wide = tab.width() * if head.is_some() { caught_up(share) } else { 1.0 };
    let half = [wide / 2.0, high * (1.0 - 0.28 * swing(share, 10.0, 4.5))];
    let round = half[0].min(half[1]);
    let corner = round + (f32::from(CONTROL_RADIUS).min(round) - round) * eased(0.5, 1.0, share);
    let body = Slab { centre: [tab.left() + half[0], y], half, corner };
    let front = (high * (1.0 + 0.22 * swing(share, 8.0, 3.0)) * (1.0 - eased(0.6, 1.0, share))).min(half[0]);
    let swell = Slab { centre: [tab.left() + wide - front, y], half: [front, front], corner: front };
    let nothing = Slab { centre: [tab.right(), y], half: [0.0, 0.0], corner: 0.0 };
    let Some(head) = head else { return ([body, swell, nothing, nothing, nothing, nothing], TAB_POUR_MELT) };
    let size = TAB_POUR_DROP * (1.0 - eased(0.5, 0.95, share));
    let drawn = [size * (1.0 + 0.5 * (1.0 - share)), size * (1.0 - 0.15 * (1.0 - share))];
    let drop = Slab { centre: [head.x, head.y], half: drawn, corner: drawn[0].min(drawn[1]) };
    let (from, to) = (tab.left() + wide - 2.0, head.x - drawn[0] + 2.0);
    let thick = if to > from { TAB_POUR_THREAD * (1.0 - eased(0.3, 0.72, share)) } else { 0.0 };
    let thread = Slab { centre: [f32::midpoint(from, to), y], half: [(to - from) / 2.0, thick], corner: thick };
    let bead = |along: f32, size: f32, [start, end]: [f32; 2]| {
        let left = if to > from { size * (std::f32::consts::PI * eased(start, end, share)).sin() } else { 0.0 };
        Slab { centre: [from + (to - from) * along, y], half: [left, left], corner: left }
    };
    let beads = [bead(0.36, TAB_POUR_BEAD, [0.5, 0.92]), bead(0.72, TAB_POUR_BEAD * 0.65, [0.55, 0.88])];
    ([body, swell, thread, drop, beads[0], beads[1]], TAB_POUR_MELT * (1.0 - eased(0.45, 0.85, share)))
}

/// One tab that is pouring in, or draining away.
pub struct Pour {
    /// Which tab, by its place among the tabs; for one that drains, the place it had.
    tab: usize,
    began: Instant,
    /// The name of a tab that drains: it is no longer among the tabs.
    gone: Option<String>,
    picture: Option<TextureHandle>,
}

impl Pour {
    /// How far it is through pouring in, or through draining away, 0 to 1. `None` once it is over.
    fn share(&self) -> Option<f32> {
        let time = if self.gone.is_some() { TAB_DRAIN_TIME } else { TAB_POUR_TIME };
        let share = self.began.elapsed().as_secs_f32() / time;
        (share < 1.0).then_some(share)
    }

    /// Puts the liquid's picture into the place kept for it under the tab (`under`): `tab` is where the tab is
    /// at this moment and `button` where the "new file" button is, when the tab is the one beside it.
    // Agent: MAIN-THREAD; paints a small picture (the tab and the button, with some room) anew for every frame,
    // and asks for the next frame.
    fn paint(&mut self, ui: &Ui, under: egui::layers::ShapeIdx, tab: Rect, button: Option<Rect>, palette: Palette) {
        let Some(share) = self.share() else { return };
        let right = button.map_or(tab.right(), |button| button.right());
        // Human: a tab that drains throws its drops to both sides of its place, so its picture has room there.
        let reach = if self.gone.is_some() { TAB_DRAIN_BEAD + 30.0 } else { TAB_POUR_ROOM };
        let part = Rect::from_min_max(tab.min, pos2(right, tab.bottom())).expand2(vec2(reach, TAB_POUR_ROOM));
        let scale = ui.ctx().pixels_per_point();
        let texels = |points: f32| {
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a strip of the title bar, a few hundred texels")]
            let texels = (points * scale).ceil().clamp(1.0, 4096.0) as usize;
            texels
        };
        let size = [texels(part.width()), texels(part.height())];
        // Human: the light crosses a new tab once, from before its left end to past its right; a tab that
        // drains has none.
        let (slabs, melt, light, color) = if self.gone.is_some() {
            let (slabs, melt) = beading(share, tab);
            (slabs.to_vec(), melt, f32::NEG_INFINITY, palette.field)
        } else {
            let (slabs, melt) = shapes(share, tab, button.map(|button| button.center()));
            (slabs.to_vec(), melt, part.left() + part.width() * (share * 1.3 - 0.15), palette.field.lerp_to_gamma(palette.strong, freshness(share)))
        };
        let image = liquid::plain(&slabs, melt, [part.left(), part.top()], size, scale, [color, Color32::WHITE], light);
        let picture = match &mut self.picture {
            Some(picture) => {
                picture.set(image, TextureOptions::LINEAR);
                picture.id()
            }
            None => self.picture.insert(ui.ctx().load_texture("tab pour", image, TextureOptions::LINEAR)).id(),
        };
        #[expect(clippy::cast_precision_loss, reason = "a few hundred texels")]
        let shown = Rect::from_min_size(part.min, vec2(size[0] as f32, size[1] as f32) / scale);
        let whole = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
        ui.painter().set(under, egui::Shape::image(picture, shown, whole, Color32::WHITE));
        ui.ctx().request_repaint();
    }
}

/// The tabs in motion: the one pouring in and the one draining away, if any.
#[derive(Default)]
pub struct Pours {
    born: Option<Pour>,
    gone: Option<Pour>,
}

impl Pours {
    /// The tab at `tab` was just made: it pours in.
    pub fn made(&mut self, tab: usize) {
        self.born = Some(Pour { tab, began: Instant::now(), gone: None, picture: None });
    }

    /// The tab at `tab`, named `title`, was just closed: it drains away where it was, and the one pouring in
    /// keeps its place among the tabs that are left.
    pub fn closed(&mut self, tab: usize, title: String) {
        self.gone = Some(Pour { tab, began: Instant::now(), gone: Some(title), picture: None });
        self.born = self.born.take().filter(|born| born.tab != tab).map(|born| Pour { tab: born.tab - usize::from(born.tab > tab), ..born });
    }

    /// Forgets the pours that are over. Call once a frame, before the tabs are drawn.
    pub fn settle(&mut self) {
        for pour in [&mut self.born, &mut self.gone] {
            if pour.as_ref().is_some_and(|pour| pour.share().is_none()) {
                *pour = None;
            }
        }
    }

    /// How the tab at `tab` is drawn while it pours in; `None` for a tab that is simply there.
    #[must_use]
    pub fn born_at(&self, tab: usize) -> Option<Look> {
        self.born.as_ref().filter(|pour| pour.tab == tab).and_then(Pour::share).map(pouring)
    }

    /// The tab that drains away before the place `tab`: its name, and how it is drawn at this moment.
    #[must_use]
    pub fn gone_at(&self, tab: usize) -> Option<(String, Look)> {
        let pour = self.gone.as_ref().filter(|pour| pour.tab == tab)?;
        Some((pour.gone.clone()?, draining(pour.share()?)))
    }

    /// Paints the liquid of both: `under` are the places kept for their pictures, `tabs` where the tab pouring
    /// in and the one draining away are at this moment, `button` where the "new file" button is, and `last` the
    /// number of tabs, which says whether a tab is the one beside the button.
    pub fn paint(&mut self, ui: &Ui, under: [egui::layers::ShapeIdx; 2], tabs: [Option<Rect>; 2], button: Rect, last: usize, palette: Palette) {
        if let (Some(pour), Some(tab)) = (&mut self.born, tabs[0]) {
            let beside = (pour.tab + 1 == last).then_some(button);
            pour.paint(ui, under[0], tab, beside, palette);
        }
        if let (Some(pour), Some(tab)) = (&mut self.gone, tabs[1]) {
            pour.paint(ui, under[1], tab, None, palette);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::TITLE_BUTTON;

    fn tab(width: f32) -> Rect {
        Rect::from_min_size(pos2(100.0, 10.0), vec2(width, 28.0))
    }

    #[test]
    fn the_tab_shoots_past_its_width_and_springs_back_and_its_name_comes_late() {
        assert!(pouring(0.0).width.abs() < 1e-3);
        assert!((pouring(1.0).width - 1.0).abs() < 1e-3);
        let widest = (0..=100u8).map(|step| pouring(f32::from(step) / 100.0).width).fold(0.0, f32::max);
        assert!(widest > 1.03 && widest < 1.17, "past its width, but not wildly: {widest}");
        assert!(pouring(0.45).shown.abs() < 1e-4 && pouring(0.45).slide < -5.0, "its name waits to the left");
        assert!((pouring(0.95).shown - 1.0).abs() < 1e-4 && pouring(0.95).slide.abs() < 1e-3, "and is in its place at the end");
    }

    #[test]
    fn a_closed_tab_loses_its_name_at_once_beads_up_and_bursts_into_two_drops() {
        assert_eq!(draining(0.0), Look { width: 1.0, shown: 1.0, slide: 0.0 });
        assert!(draining(0.25).shown.abs() < 1e-4, "the name is gone in the first fifth");
        assert!(draining(0.9).width.abs() < 1e-4, "its place has closed before the drops are gone");

        let ([body, left, right], _) = beading(0.0, tab(120.0));
        assert_eq!(body.half, [60.0, 14.0]);
        assert!((body.corner - f32::from(CONTROL_RADIUS)).abs() < 1e-4, "it starts as the tab");
        assert!(left.half[0].abs() < 1e-4 && right.half[0].abs() < 1e-4, "no drops yet");

        let ([body, ..], _) = beading(0.5, tab(8.0));
        assert!((body.half[0] - TAB_DRAIN_BEAD).abs() < 1e-3, "as wide as the bead");
        assert!((body.corner - body.half[0].min(body.half[1])).abs() < 1e-3, "and round");

        let ([body, left, right], melt) = beading(0.75, tab(0.0));
        assert!(left.half[0] > 2.0 && right.half[0] > 2.0, "two drops");
        assert!(left.centre[0] < body.centre[0] - body.half[0] && right.centre[0] > body.centre[0] + body.half[0], "one to each side of the bead");
        assert!(melt > 0.0);

        let ([body, left, right], melt) = beading(1.0, tab(0.0));
        assert!(body.half[0].abs() < 1e-4 && left.half[0].abs() < 1e-4 && right.half[0].abs() < 1e-4 && melt.abs() < 1e-4, "nothing is left");
    }

    #[test]
    fn the_pour_starts_as_a_drop_at_the_button_and_ends_as_the_tab_alone() {
        let button = Some(pos2(250.0, 24.0));
        let ([body, _, _, drop, first, second], melt) = shapes(0.0, tab(0.0), button);
        assert!(body.half[0].abs() < 1e-4, "no body yet");
        assert!(drop.half[0] > drop.half[1] && drop.half[1] > 6.0, "a drop, drawn out sideways");
        assert!(first.half[0].abs() < 1e-4 && second.half[0].abs() < 1e-4, "no beads yet");
        assert!((melt - TAB_POUR_MELT).abs() < 1e-4);
        assert!(freshness(0.0) > 0.15, "fresh liquid is stronger");

        let (slabs, melt) = shapes(1.0, tab(120.0), button);
        assert_eq!(slabs[0].half, [60.0, 14.0]);
        assert!((slabs[0].corner - f32::from(CONTROL_RADIUS)).abs() < 1e-4, "the tab's own corners");
        assert!(slabs[1..].iter().all(|slab| slab.half[0].min(slab.half[1]).abs() < 1e-4), "everything but the body is gone");
        assert!(melt.abs() < 1e-4 && freshness(1.0).abs() < 1e-4, "nothing melts, and it has the tab's colour");
    }

    #[test]
    fn the_body_follows_the_button_on_a_thread_that_breaks_into_beads() {
        let place = tab(100.0);
        let button = Some(pos2(place.right() + 22.0, 24.0));
        let ([body, swell, thread, drop, ..], _) = shapes(0.25, place, button);
        assert!(body.half[0] * 2.0 < 60.0, "the body has not caught up with its place");
        assert!(swell.half[0] > 10.0 && (swell.centre[0] + swell.half[0] - (body.centre[0] + body.half[0])).abs() < 1e-3, "a swell at its front");
        assert!(thread.half[0] > 15.0 && thread.half[1] > 1.5, "a thread between the body and the drop");
        assert!(thread.centre[0] > body.centre[0] && thread.centre[0] < drop.centre[0]);

        let ([_, _, thread, _, first, second], _) = shapes(0.74, place, button);
        assert!(thread.half[1].abs() < 1e-4, "the thread has let go");
        assert!(first.half[0] > 1.5 && second.half[0] > 1.0 && first.half[0] > second.half[0], "two beads, the nearer one larger");
        let thin = (0..=20u8).map(|step| shapes(f32::from(step) / 100.0, place, button).0[0].half[1]).fold(f32::MAX, f32::min);
        assert!(thin < 11.0, "the body is thin while it is pulled");
    }

    #[test]
    fn a_tab_that_is_not_beside_the_button_fills_its_place_and_has_no_thread() {
        let (slabs, _) = shapes(0.3, tab(60.0), None);
        assert_eq!(slabs[0].half[0], 30.0);
        assert!(slabs[2..].iter().all(|slab| slab.half[0].min(slab.half[1]).abs() < 1e-4));
    }

    #[test]
    fn a_closed_tab_drains_where_it_was_and_the_one_pouring_in_keeps_its_place() {
        let mut pours = Pours::default();
        pours.made(3);
        pours.closed(1, "menu.quill".to_owned());
        assert!(pours.born_at(2).is_some(), "the new tab moved one place down");
        let (title, look) = pours.gone_at(1).unwrap();
        assert_eq!(title, "menu.quill");
        assert!(look.width > 0.9, "it has only begun to drain");
        assert!(pours.gone_at(0).is_none());
        pours.closed(2, "new".to_owned());
        assert!(pours.born_at(2).is_none(), "closing the tab that pours in ends its pour");
    }

    /// Paints `frames` moments of liquid under each other into a file named `name` in the temporary folder.
    fn strip(name: &str, frames: u8, liquid: impl Fn(f32) -> (Vec<Slab>, f32)) {
        let (wide, high, scale) = (240_usize * 3, 44_usize * 3, 3.0_f32);
        let ground = image::Rgba([24, 24, 27, 255]);
        let mut strip = image::RgbaImage::from_pixel(u32::try_from(wide).unwrap(), u32::try_from(high).unwrap() * u32::from(frames), ground);
        for frame in 0..frames {
            let share = f32::from(frame) / f32::from(frames - 1);
            let (slabs, melt) = liquid(share);
            let light = 60.0 + 200.0 * (share * 1.3 - 0.15);
            let picture = liquid::plain(&slabs, melt, [60.0, 2.0], [wide, high], scale, [crate::theme::DARK.field, Color32::WHITE], light);
            for (index, pixel) in picture.pixels.iter().enumerate() {
                let [red, green, blue, alpha] = pixel.to_srgba_unmultiplied();
                let (across, down) =
                    (u32::try_from(index % wide).unwrap(), u32::try_from(index / wide).unwrap() + u32::from(frame) * u32::try_from(high).unwrap());
                let under = strip.get_pixel(across, down).0;
                let over = |top: u8, bottom: u8| {
                    u8::try_from((u32::from(top) * u32::from(alpha) + u32::from(bottom) * (255 - u32::from(alpha))) / 255).unwrap()
                };
                strip.put_pixel(across, down, image::Rgba([over(red, under[0]), over(green, under[1]), over(blue, under[2]), 255]));
            }
        }
        let file = std::env::temp_dir().join(name);
        strip.save(&file).unwrap();
        println!("{}", file.display());
    }

    /// Writes the pour and the drain as strips of pictures to look at. Run by hand:
    /// `cargo test -- --ignored liquid_strips --nocapture`.
    #[test]
    #[ignore = "writes pictures into the temporary folder for a person to look at"]
    fn liquid_strips() {
        strip("quill-tab-pour.png", 16, |share| {
            let now = tab(120.0 * pouring(share).width);
            let (slabs, melt) = shapes(share, now, Some(pos2(now.right() + 8.0 + TITLE_BUTTON / 2.0, 24.0)));
            (slabs.to_vec(), melt)
        });
        strip("quill-tab-drain.png", 10, |share| {
            let (slabs, melt) = beading(share, tab(120.0 * draining(share).width));
            (slabs.to_vec(), melt)
        });
    }
}
