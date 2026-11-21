//! The next steps the assistant offers, beside the message box.
//!
//! The steps ([`crate::chat::Chat::offers`]) are collected behind one round button at the message box's right
//! edge. Open, they stand above it as small pills, the newest nearest the button: a click on a pill sends that
//! step as the user's message, the cross that shows under the pointer drops it, and "Do all" beside the button
//! sends every step as one message. Folded, the button shows how many there are.
//!
//! New steps rise out of the button one after the other and glow for a moment; the pills already there glide up
//! to make room. A step that is sent lifts, flies into the conversation and becomes the message there, while
//! the others close the gap. When the last one is gone the button shows a tick and goes. With "Animations &
//! effects" off in the settings everything simply is where it belongs.

use std::sync::Arc;

use egui::text::{LayoutJob, TextWrapping};
use egui::{Color32, FontId, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

use crate::icons;
use crate::tasklist::{smooth, tint};
use crate::theme::{
    CHAT_ACCENT, CHAT_BUBBLE_RADIUS, ISLAND_GAP, OFFER_BADGE, OFFER_BORN_APART, OFFER_BUTTON, OFFER_CROSS, OFFER_DONE_TIME, OFFER_DROP_TIME,
    OFFER_FLY_APART, OFFER_FLY_TIME, OFFER_GAP, OFFER_GLIDE, OFFER_GLOW_TIME, OFFER_MOST_WIDTH, OFFER_NARROWEST, OFFER_OPEN_APART, OFFER_OPEN_TIME,
    OFFER_PAD, OFFER_PILL, OFFER_PLUS_TIME, OFFER_RING_TIME, OFFER_ROWS, Palette, SIDEBAR_TEXT_SIZE,
};

/// The button's icon while steps wait behind it.
const BULB_ICON: &str = crate::chat::steps::SUGGESTED_ICON;
/// Its icon while the steps show (a click folds them), and the cross that drops one step.
const CLOSE_ICON: &str = "x";
/// Its icon for a moment after the last step was taken.
const DONE_ICON: &str = "check";
/// A time before anything happened.
const LONG_AGO: f64 = f64::NEG_INFINITY;

/// What the user did with the steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// Send the step at this place.
    Take(usize),
    /// Send every step as one message.
    TakeAll,
    /// Drop the step at this place.
    Drop(usize),
}

/// Where the steps are drawn, and how.
#[derive(Clone, Copy)]
pub struct Look {
    /// The message box: the button stands at its right edge, on its bottom line.
    pub anchor: Rect,
    /// The colours of the islands.
    pub palette: Palette,
    /// Things move ("Animations & effects" in the settings).
    pub animate: bool,
    /// There is room beside the button for "Do all"; without it, "Do all" stands on top of the pills.
    // Human: under the open sidebar the canvas is free beside the button. Beside the folded message bar the
    // zoom control stands close by, so there the button keeps to itself.
    pub beside: bool,
    /// How much room there is to the right of the button before the next island or the window's edge: a
    /// pill is no wider than that, and its words wrap sooner where it is tight.
    pub room: f32,
}

impl Look {
    /// The widest a pill may be here.
    fn widest(self) -> f32 {
        self.room.clamp(OFFER_NARROWEST, OFFER_MOST_WIDTH)
    }
}

/// One step's pill as it is drawn.
#[derive(Clone)]
struct Pill {
    text: String,
    /// Where its top edge is right now; it glides to its place.
    top: f32,
    /// When it shows, in egui's time: steps that come together show one after the other.
    born: f64,
}

/// A pill on its way somewhere: into the conversation (sent) or away (dropped).
#[derive(Clone)]
struct Leaving {
    text: String,
    /// Where it stood.
    from: Rect,
    /// When it sets off.
    starts: f64,
}

/// What is remembered between frames to move things; it lives in egui's memory, one for each chat.
#[derive(Clone)]
struct Motion {
    /// The steps show above the button.
    open: bool,
    pills: Vec<Pill>,
    /// Pills flying into the conversation, and pills falling away.
    sent: Vec<Leaving>,
    dropped: Vec<Leaving>,
    /// When steps last came and how many, for the ring and the "+3" over the button.
    came: (f64, usize),
    /// When the last step was taken, for the tick.
    done: f64,
}

impl Default for Motion {
    fn default() -> Self {
        Motion { open: false, pills: Vec::new(), sent: Vec::new(), dropped: Vec::new(), came: (LONG_AGO, 0), done: LONG_AGO }
    }
}

impl Motion {
    /// Takes in the steps as they are now: new ones get a pill that starts at the button `home`, steps that
    /// are gone lose theirs.
    // Human: the first steps of a chat open by themselves, so they are seen. Steps that join others the user
    // has folded away stay folded; the button says that they came.
    fn take_in(&mut self, offered: &[String], now: f64, home: f32, animate: bool) {
        let mut fresh = 0;
        let were_none = self.pills.is_empty();
        let mut pills = Vec::with_capacity(offered.len());
        for text in offered {
            let pill = self.pills.iter().find(|pill| &pill.text == text).cloned().unwrap_or_else(|| {
                let born = if animate { now + f64::from(OFFER_BORN_APART) * f64::from(fresh) } else { LONG_AGO };
                fresh += 1;
                Pill { text: text.clone(), top: home, born }
            });
            pills.push(pill);
        }
        self.pills = pills;
        if fresh > 0 {
            self.came = (if animate { now } else { LONG_AGO }, usize::try_from(fresh).unwrap_or(0));
            self.open |= were_none;
        }
        if offered.is_empty() {
            self.open = false;
        }
    }

    /// Whether anything is still on its way at `now`.
    fn moving(&self, now: f64) -> bool {
        let recent = |since: f64, time: f32| now - since < f64::from(time);
        !self.sent.is_empty()
            || !self.dropped.is_empty()
            || recent(self.came.0, OFFER_PLUS_TIME)
            || recent(self.done, OFFER_DONE_TIME + OFFER_OPEN_TIME)
            || self.pills.iter().any(|pill| recent(pill.born, OFFER_GLOW_TIME))
    }
}

/// Where the motion of the chat `key`'s steps is kept.
fn motion_id(key: u64) -> egui::Id {
    egui::Id::unique(("offers", key))
}

/// Where the conversation says where it drew its last message of the user's: a step that is sent flies there.
fn landing_id() -> egui::Id {
    egui::Id::unique("offers-landing")
}

/// Where it is kept which message is on its way into the conversation, and when it lands.
fn flying_id() -> egui::Id {
    egui::Id::unique("offers-flying")
}

/// Tells where the conversation drew a message of the user's; the last one told in a frame is the newest.
pub fn lands_at(ui: &Ui, bubble: Rect) {
    ui.data_mut(|data| data.insert_temp(landing_id(), bubble));
}

/// The message that is flying into the conversation right now, as the conversation shows it, and when it
/// lands (in egui's time): the conversation shows that message only then.
#[must_use]
pub fn flying(ctx: &egui::Context) -> Option<(String, f64)> {
    let now = ctx.input(|input| input.time);
    ctx.data(|data| data.get_temp::<(String, f64)>(flying_id())).filter(|(_, lands)| *lands > now)
}

/// The seconds from `since` to `now` as a share of `time`, from 0 to 1.
#[expect(clippy::cast_possible_truncation, reason = "a share between 0 and 1")]
fn share(now: f64, since: f64, time: f32) -> f32 {
    ((now - since) / f64::from(time)).clamp(0.0, 1.0) as f32
}

/// `share` eased: slow away, fast through the middle, slow into place.
fn eased(share: f32) -> f32 {
    smooth(0.0, 1.0, share)
}

/// `share` eased so that it runs a little past its end and settles back, like something on a spring.
fn sprung(share: f32) -> f32 {
    let (back, t) = (1.70158_f32, share.clamp(0.0, 1.0) - 1.0);
    1.0 + (back + 1.0) * t.powi(3) + back * t.powi(2)
}

/// The point `share` of the way from `from` to `to`.
fn between(from: Rect, to: Rect, share: f32) -> Rect {
    Rect::from_min_max(from.min + (to.min - from.min) * share, from.max + (to.max - from.max) * share)
}

/// `text` on one line in the panel's small type, cut short with an ellipsis where it is wider than `width`.
fn words(ctx: &egui::Context, text: &str, color: Color32, width: f32) -> Arc<egui::Galley> {
    let mut job = LayoutJob::simple_singleline(text.to_owned(), FontId::proportional(SIDEBAR_TEXT_SIZE), color);
    job.wrap = TextWrapping::truncate_at_width(width.max(0.0));
    ctx.fonts_mut(|fonts| fonts.layout_job(job))
}

/// The words of a step in its pill: on as many lines as they need, up to [`OFFER_ROWS`], so a step can be read
/// whole; only one longer than that is cut short.
fn wrapped(ctx: &egui::Context, text: &str, color: Color32, widest: f32) -> Arc<egui::Galley> {
    let mut job = LayoutJob::simple(text.to_owned(), FontId::proportional(SIDEBAR_TEXT_SIZE), color, widest - OFFER_PAD - OFFER_CROSS);
    job.wrap.max_rows = OFFER_ROWS;
    ctx.fonts_mut(|fonts| fonts.layout_job(job))
}

/// A pill as it is laid out: its words, how high it is, and how far its top edge stands above the button's.
struct Laid {
    text: Arc<egui::Galley>,
    high: f32,
    rise: f32,
}

/// Lays out the pills of `steps` above the button, the first nearest it: a pill is as high as its lines need.
fn laid_out<'a>(ctx: &egui::Context, steps: impl Iterator<Item = &'a str>, look: Look) -> Vec<Laid> {
    let mut rise = 0.0;
    steps
        .map(|step| {
            let text = wrapped(ctx, step, look.palette.foreground, look.widest());
            let high = (text.size().y + OFFER_PILL / 2.0).max(OFFER_PILL);
            rise += high + OFFER_GAP;
            Laid { text, high, rise }
        })
        .collect()
}

/// How far the pill at `place` of `count` has come out of the button when the steps are `open` this far: the
/// pills nearest the button set off first.
#[expect(clippy::cast_precision_loss, reason = "a handful of pills")]
fn fanned(open: f32, place: usize, count: usize) -> f32 {
    let late = OFFER_OPEN_APART * place as f32;
    let span = 1.0 + OFFER_OPEN_APART * count.saturating_sub(1) as f32;
    (open * span - late).clamp(0.0, 1.0)
}

/// A pill: an island with the corners of a one-line pill and `text` in it, at `opacity`.
fn pill(painter: &egui::Painter, rect: Rect, text: &Arc<egui::Galley>, colors: (Color32, Color32, Color32), opacity: f32) {
    let (fill, edge, ink) = colors;
    let round = (OFFER_PILL / 2.0).min(rect.height() / 2.0);
    painter.rect(rect, round, fill.gamma_multiply(opacity), Stroke::new(1.0, edge.gamma_multiply(opacity)), StrokeKind::Inside);
    let at = pos2(rect.left() + OFFER_PAD, rect.center().y - text.size().y / 2.0);
    let room = Rect::from_min_max(rect.min, pos2(rect.right() - OFFER_PAD / 2.0, rect.bottom()));
    painter.with_clip_rect(room.intersect(painter.clip_rect())).galley(at, Arc::clone(text), ink.gamma_multiply(opacity));
}

/// Where everything stands in the frame being drawn.
struct Frame {
    /// The round button.
    button: Rect,
    /// egui's time, and the time since the last frame.
    now: f64,
    step: f32,
    /// How far the steps are open, 0 to 1.
    open: f32,
}

/// Draws the steps `offered` in the chat `key` beside the message box. Returns what the user did.
// Agent: MAIN-THREAD; a floating layer over the canvas; asks for another frame while something moves.
pub fn show(ctx: &egui::Context, offered: &[String], key: u64, look: Look) -> Option<Act> {
    let now = ctx.input(|input| input.time);
    let id = motion_id(key);
    let mut motion: Motion = ctx.data(|data| data.get_temp(id)).unwrap_or_default();
    let button = Rect::from_min_size(pos2(look.anchor.right() + ISLAND_GAP, look.anchor.bottom() - OFFER_BUTTON), Vec2::splat(OFFER_BUTTON));
    motion.take_in(offered, now, button.top(), look.animate);
    let there = !offered.is_empty() || motion.moving(now);
    if !there {
        ctx.data_mut(|data| data.remove::<Motion>(id));
        return None;
    }
    let open = if look.animate {
        eased(ctx.animate_bool_with_time(id.with("open"), motion.open, OFFER_OPEN_TIME))
    } else if motion.open {
        1.0
    } else {
        0.0
    };
    let frame = Frame { button, now, step: ctx.input(|input| input.stable_dt).min(0.1), open };
    let stack = laid_out(ctx, offered.iter().map(String::as_str), look).last().map_or(0.0, |top| top.rise);
    let tall = stack + if look.beside { 0.0 } else { OFFER_PILL + OFFER_GAP };
    let reach =
        Rect::from_min_max(pos2(button.left(), button.top() - tall * open), pos2(button.left() + look.widest().max(OFFER_BUTTON), button.bottom()));
    let mut act = None;
    egui::Area::new(id.with("area")).fade_in(false).fixed_pos(reach.min).order(egui::Order::Foreground).show(ctx, |ui| {
        // Folded, only the button is in the way of the canvas; open, the room its pills stand in.
        ui.allocate_rect(if open > 0.0 { reach } else { button }, Sense::hover());
        act = pills(ui, &mut motion, id, &frame, look);
        act = act.or(the_button(ui, &mut motion, id, &frame, look));
    });
    // Human: what is sent says so to the conversation, which shows the message when the pill has landed.
    let sending = match act {
        Some(Act::Take(place)) => offered.get(place).cloned(),
        Some(Act::TakeAll) => Some(crate::chat::asks::all_shown(offered)),
        _ => None,
    };
    if let Some(text) = sending.filter(|_| look.animate) {
        let last = motion.sent.iter().map(|pill| pill.starts).fold(now, f64::max);
        ctx.data_mut(|data| data.insert_temp(flying_id(), (text, last + f64::from(OFFER_FLY_TIME) * 0.86)));
    }
    leaving(ctx, &mut motion, &frame, look);
    let opening = open > 0.0 && open < 1.0;
    if look.animate && (motion.moving(now) || opening || motion.pills.iter().any(|pill| pill.born > now)) {
        ctx.request_repaint();
    }
    ctx.data_mut(|data| data.insert_temp(id, motion));
    act
}

/// The pills above the button, the newest nearest it. Returns what the user did with one.
fn pills(ui: &Ui, motion: &mut Motion, id: egui::Id, frame: &Frame, look: Look) -> Option<Act> {
    let (palette, now) = (look.palette, frame.now);
    let count = motion.pills.len();
    let mut act = None;
    let mut gliding = false;
    let laid = laid_out(ui.ctx(), motion.pills.iter().map(|pill| pill.text.as_str()), look);
    for (place, Laid { text, high, rise }) in laid.into_iter().enumerate() {
        let home = frame.button.top() - rise;
        let pill_now = &mut motion.pills[place];
        // The pill glides to its place: when steps come or go, the others make room or close the gap.
        let apart = home - pill_now.top;
        pill_now.top = if look.animate && apart.abs() > 0.3 { pill_now.top + apart * (1.0 - (-OFFER_GLIDE * frame.step).exp()) } else { home };
        gliding |= (home - pill_now.top).abs() > 0.3;
        let out = fanned(frame.open, place, count);
        let born = share(now, pill_now.born, 0.24);
        if out <= 0.0 || now < pill_now.born {
            continue;
        }
        let hover_id = id.with(("pill", place));
        let wide = OFFER_PAD + text.size().x + OFFER_CROSS;
        // Folded, every pill is in the button; it comes out of it going up and growing to its width.
        let folded_top = frame.button.center().y - high / 2.0;
        let spring = if look.animate { sprung(out) } else { out };
        let top = folded_top + (pill_now.top - folded_top) * spring;
        let rect = Rect::from_min_size(pos2(frame.button.left(), top), vec2(OFFER_BUTTON + (wide - OFFER_BUTTON) * out.min(1.0), high));
        let opacity = out.min(born);
        let glow = share(now, pill_now.born, OFFER_GLOW_TIME);
        if glow < 1.0 && opacity > 0.0 {
            let ring = Stroke::new(2.0, CHAT_ACCENT.gamma_multiply((1.0 - glow).powi(2) * opacity));
            ui.painter().rect_stroke(rect.expand(1.0 + 4.0 * glow), OFFER_PILL, ring, StrokeKind::Outside);
        }
        // Human: the pill has two parts that don't overlap: its words, which send the step, and at its right
        // end a cross that is always there and as high as the pill, which drops it. The pointer over either
        // lights the pill's edge.
        let ready = out >= 0.98 && born >= 1.0;
        let (words_part, cross_part) = rect.split_left_right_at_x(rect.right() - OFFER_CROSS);
        let over = ui.ctx().animate_bool_with_time(hover_id.with("over"), ready && ui.rect_contains_pointer(rect), 0.14);
        pill(ui.painter(), rect, &text, (palette.island, tint(palette.outline, CHAT_ACCENT, over), palette.foreground), opacity);
        let cross = pos2(cross_part.center().x - 3.0, cross_part.center().y);
        let on_cross = ready && ui.rect_contains_pointer(cross_part);
        if on_cross {
            ui.painter().circle_filled(cross, OFFER_CROSS / 2.0 - 3.0, palette.hover.gamma_multiply(opacity));
        }
        let ink = if on_cross { palette.foreground } else { palette.muted.gamma_multiply(0.75) };
        icons::paint(ui.painter(), Rect::from_center_size(cross, Vec2::splat(11.0)), CLOSE_ICON, ink.gamma_multiply(opacity));
        if !ready {
            continue;
        }
        let hand = egui::CursorIcon::PointingHand;
        let dropped = ui.interact(cross_part, hover_id.with("drop"), Sense::click()).on_hover_cursor(hand).on_hover_text("Remove this suggestion");
        let taken = ui.interact(words_part, hover_id, Sense::click()).on_hover_cursor(hand).on_hover_text(pill_now.text.clone());
        let leaves = Leaving { text: pill_now.text.clone(), from: rect, starts: now };
        if dropped.clicked() {
            motion.dropped.push(leaves);
            act = Some(Act::Drop(place));
        } else if taken.clicked() {
            motion.sent.push(leaves);
            act = Some(Act::Take(place));
        }
    }
    if gliding {
        ui.ctx().request_repaint();
    }
    act
}

/// The round button, and beside it "Do all" while the steps show. Returns [`Act::TakeAll`] when that is clicked.
fn the_button(ui: &Ui, motion: &mut Motion, id: egui::Id, frame: &Frame, look: Look) -> Option<Act> {
    let (palette, now, button, open) = (look.palette, frame.now, frame.button, frame.open);
    let count = motion.pills.len();
    let painter = ui.painter();
    // Human: when the last step is taken the button shows a tick for a moment, then goes.
    let done = share(now, motion.done, OFFER_DONE_TIME);
    let leaving = !motion.sent.is_empty();
    let size = if count > 0 || leaving { 1.0 } else { 1.0 - smooth(0.7, 1.0, done) };
    if size <= 0.0 {
        return None;
    }
    let (centre, radius) = (button.center(), OFFER_BUTTON / 2.0 * size);
    let pressed = ui.interact(button, id.with("button"), Sense::click());
    let lit = pressed.hovered() || (count == 0 && !leaving);
    painter.circle(centre, radius - 0.5, palette.island, Stroke::new(1.0, if lit { CHAT_ACCENT } else { palette.outline }));
    let came = share(now, motion.came.0, OFFER_RING_TIME);
    for age in [came, if count == 0 && !leaving { done / 0.8 } else { 1.0 }] {
        if age < 1.0 {
            painter.circle_stroke(centre, radius * (1.0 + 0.8 * age), Stroke::new(2.0, CHAT_ACCENT.gamma_multiply((1.0 - age).powi(2))));
        }
    }
    let icon = |name: &'static str, color: Color32, shown: f32| {
        if shown > 0.0 {
            icons::paint(painter, Rect::from_center_size(centre, Vec2::splat(18.0 * size * (0.6 + 0.4 * shown))), name, color.gamma_multiply(shown));
        }
    };
    if count == 0 && leaving {
        icon(BULB_ICON, CHAT_ACCENT, 1.0);
    } else if count == 0 {
        icon(DONE_ICON, CHAT_ACCENT, sprung(share(now, motion.done, 0.3)).min(1.0));
    } else {
        // The bulb nods when steps come.
        let nod = if came < 1.0 { (came * std::f32::consts::TAU * 1.5).sin() * 2.5 * (1.0 - came) } else { 0.0 };
        if open < 1.0 {
            let place = Rect::from_center_size(centre + vec2(nod, 0.0), Vec2::splat(18.0 * (0.6 + 0.4 * (1.0 - open))));
            icons::paint(painter, place, BULB_ICON, CHAT_ACCENT.gamma_multiply(1.0 - open));
        }
        icon(CLOSE_ICON, palette.muted, open);
    }
    // Folded, a small count sits on the button's shoulder; it pops when the count changes.
    if count > 0 && open < 1.0 {
        let pop = 1.0 + 0.3 * (1.0 - came) * (came * std::f32::consts::PI).sin();
        let at = pos2(button.right() - 4.0, button.top() + 4.0);
        painter.circle_filled(at, OFFER_BADGE / 2.0 * (1.0 - open) * pop, CHAT_ACCENT);
        let ink = Color32::WHITE.gamma_multiply(1.0 - open);
        painter.text(at, egui::Align2::CENTER_CENTER, count.to_string(), FontId::proportional(11.0), ink);
    }
    let plus = share(now, motion.came.0, OFFER_PLUS_TIME);
    if plus < 1.0 && motion.came.1 > 0 && open < 0.5 {
        let at = pos2(centre.x, button.top() - 6.0 - 18.0 * plus);
        let ink = CHAT_ACCENT.gamma_multiply(smooth(0.0, 0.2, plus) * (1.0 - smooth(0.6, 1.0, plus)));
        painter.text(at, egui::Align2::CENTER_CENTER, format!("+{}", motion.came.1), FontId::proportional(12.0), ink);
    }
    if count > 0
        && pressed.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(if motion.open { "Fold away" } else { "Next steps" }).clicked()
    {
        motion.open = !motion.open;
    }
    if count == 0 || open <= 0.0 {
        return None;
    }
    // "Do all" slides out of the button.
    let says = if count == 1 { "Do it".to_owned() } else { format!("Do all {count}") };
    let text = words(ui.ctx(), &says, palette.primary_text, OFFER_MOST_WIDTH);
    let slid = if look.animate { sprung(open) } else { open };
    let size = vec2(text.size().x + 2.0 * OFFER_PAD, OFFER_PILL);
    let all = if look.beside {
        Rect::from_min_size(pos2(button.right() + OFFER_GAP - (1.0 - slid) * 18.0, centre.y - OFFER_PILL / 2.0), size)
    } else {
        // On top of the pills, where it comes out of the button last.
        let stack = laid_out(ui.ctx(), motion.pills.iter().map(|pill| pill.text.as_str()), look).last().map_or(0.0, |top| top.rise);
        let home = button.top() - stack - OFFER_PILL - OFFER_GAP;
        let folded = centre.y - OFFER_PILL / 2.0;
        Rect::from_min_size(pos2(button.left(), folded + (home - folded) * slid), size)
    };
    pill(painter, all, &text, (palette.primary, palette.primary, palette.primary_text), open);
    if open < 0.98 || !ui.interact(all, id.with("all"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
        return None;
    }
    // Every pill sets off for the conversation, the one nearest the button first.
    let laid = laid_out(ui.ctx(), motion.pills.iter().map(|pill| pill.text.as_str()), look);
    for (place, Laid { text, high, rise }) in laid.into_iter().enumerate() {
        let from = Rect::from_min_size(pos2(button.left(), button.top() - rise), vec2(OFFER_PAD + text.size().x + OFFER_CROSS, high));
        #[expect(clippy::cast_precision_loss, reason = "a handful of pills")]
        let starts = now + f64::from(OFFER_FLY_APART) * place as f64;
        motion.sent.push(Leaving { text: motion.pills[place].text.clone(), from, starts });
    }
    Some(Act::TakeAll)
}

/// The pills that are leaving, over everything else: those that were sent fly into the conversation and turn
/// into the message there; those that were dropped fall away.
fn leaving(ctx: &egui::Context, motion: &mut Motion, frame: &Frame, look: Look) {
    let (palette, now) = (look.palette, frame.now);
    if !look.animate {
        if !motion.sent.is_empty() && motion.pills.len() <= motion.sent.len() {
            motion.done = LONG_AGO;
        }
        motion.sent.clear();
        motion.dropped.clear();
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("offers-leaving")));
    // Where the message is, or, while the conversation can't be seen, the message box it is written in.
    let fallback = Rect::from_center_size(look.anchor.center(), vec2(look.anchor.width() * 0.6, OFFER_PILL));
    let landing: Rect = ctx.data(|data| data.get_temp(landing_id())).unwrap_or(fallback);
    let had = motion.sent.len();
    motion.sent.retain(|pill| now - pill.starts < f64::from(OFFER_FLY_TIME));
    if had > 0 && motion.sent.is_empty() && motion.pills.is_empty() {
        motion.done = now;
    }
    for flying in &motion.sent {
        let went = share(now, flying.starts, OFFER_FLY_TIME);
        // It lifts, crosses over while it takes the message's shape, and fades once it is there.
        let lifted = flying.from.translate(vec2(0.0, -6.0 * smooth(0.0, 0.2, went)));
        let over = eased(((went - 0.2) / 0.66).clamp(0.0, 1.0));
        let rect = between(lifted, landing, over);
        let opacity = 1.0 - smooth(0.86, 1.0, went);
        let radius = OFFER_PILL / 2.0 + (f32::from(CHAT_BUBBLE_RADIUS) - OFFER_PILL / 2.0) * over;
        let fill = tint(palette.island, palette.hover, over).gamma_multiply(opacity);
        painter.rect(rect, radius.min(rect.height() / 2.0), fill, Stroke::new(1.0, CHAT_ACCENT.gamma_multiply(opacity)), StrokeKind::Inside);
        let text = words(ctx, &flying.text, palette.foreground, (rect.width() - 2.0 * OFFER_PAD).max(0.0));
        let at = pos2(rect.left() + OFFER_PAD, rect.center().y - text.size().y / 2.0);
        painter.with_clip_rect(rect).galley(at, text, palette.foreground.gamma_multiply(opacity));
    }
    motion.dropped.retain(|pill| now - pill.starts < f64::from(OFFER_DROP_TIME));
    for falling in &motion.dropped {
        let fell = share(now, falling.starts, OFFER_DROP_TIME);
        let text = words(ctx, &falling.text, palette.foreground, OFFER_MOST_WIDTH);
        let rect = falling.from.translate(vec2(0.0, 10.0 * fell * fell)).shrink(2.0 * fell);
        pill(&painter, rect, &text, (palette.island, palette.outline, palette.foreground), 1.0 - fell);
    }
}

/// The point the steps' button has its middle at, for a message box at `anchor`.
#[cfg(test)]
fn button_at(anchor: Rect) -> egui::Pos2 {
    pos2(anchor.right() + ISLAND_GAP + OFFER_BUTTON / 2.0, anchor.bottom() - OFFER_BUTTON / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| (*text).to_owned()).collect()
    }

    #[test]
    fn the_first_steps_open_by_themselves_and_later_ones_leave_a_folded_list_folded() {
        let mut motion = Motion::default();
        motion.take_in(&steps(&["Add a dark version", "Export it"]), 10.0, 500.0, true);
        assert!(motion.open && motion.came == (10.0, 2));
        // They show one after the other, and start at the button.
        assert!(motion.pills[1].born > motion.pills[0].born && motion.pills.iter().all(|pill| (pill.top - 500.0).abs() < f32::EPSILON));
        // The user folds them away; a step that joins them doesn't open them again.
        motion.open = false;
        motion.pills[0].top = 300.0;
        motion.take_in(&steps(&["Make it bolder", "Add a dark version", "Export it"]), 20.0, 500.0, true);
        assert!(!motion.open && motion.came == (20.0, 1));
        // A pill that was there keeps its place; only the new one starts at the button.
        let tops: Vec<f32> = motion.pills.iter().map(|pill| pill.top).collect();
        assert_eq!(tops, [500.0, 300.0, 500.0]);
        // With nothing left the list is folded, ready for the next first steps.
        motion.open = true;
        motion.take_in(&[], 30.0, 500.0, true);
        assert!(!motion.open && motion.pills.is_empty());
    }

    #[test]
    fn without_motion_steps_are_simply_there() {
        let mut motion = Motion::default();
        motion.take_in(&steps(&["Add a dark version"]), 10.0, 500.0, false);
        assert!(motion.open && !motion.moving(10.0));
        assert!(motion.pills[0].born < 0.0);
    }

    #[test]
    fn pills_come_out_of_the_button_one_after_the_other() {
        // Folded, none is out; open, all are.
        assert!((0..4).all(|place| fanned(0.0, place, 4) <= 0.0));
        assert!((0..4).all(|place| (fanned(1.0, place, 4) - 1.0).abs() < f32::EPSILON));
        // On the way, the pill nearest the button is furthest out.
        assert!(fanned(0.4, 0, 4) > fanned(0.4, 1, 4) && fanned(0.4, 1, 4) > fanned(0.4, 3, 4));
        // A spring runs past its end and settles there.
        assert!(sprung(0.7) > 1.0 && (sprung(1.0) - 1.0).abs() < 1e-6 && sprung(0.0).abs() < 1e-6);
    }

    #[test]
    fn the_button_stands_at_the_message_boxs_right_edge_on_its_bottom_line() {
        let anchor = Rect::from_min_max(pos2(8.0, 600.0), pos2(385.0, 691.0));
        let middle = button_at(anchor);
        assert!((middle.x - (385.0 + ISLAND_GAP + OFFER_BUTTON / 2.0)).abs() < f32::EPSILON);
        assert!((middle.y + OFFER_BUTTON / 2.0 - 691.0).abs() < f32::EPSILON);
        // What flies into the conversation goes all the way from where it stood to where the message is.
        let (from, to) = (Rect::from_min_size(pos2(393.0, 500.0), vec2(180.0, 32.0)), Rect::from_min_size(pos2(120.0, 300.0), vec2(250.0, 40.0)));
        assert_eq!((between(from, to, 0.0), between(from, to, 1.0)), (from, to));
        assert!(icons::outline("lucide", BULB_ICON).is_some() && icons::outline("lucide", CLOSE_ICON).is_some());
        assert!(icons::outline("lucide", DONE_ICON).is_some());
    }
}
