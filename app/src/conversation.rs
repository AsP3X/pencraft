//! The conversation in the chat panel: the user's messages, the assistant's words, and a note for each thing
//! it does, drawn with a little motion so what is new can be followed.
//!
//! A message of the user's rises out of the message box. The assistant's words flow in at an even pace however
//! unevenly they arrive, the newest ones fading in behind a small drop. Each tool call is a note that slides in
//! under the one before it: a light runs through its words while the tool works, and when it is done the note
//! says how long it took. Once the assistant goes on, the notes of that stretch of work fold into one line
//! ("5 steps · 8 s") that opens again with a click. A drop waits while the model thinks, beside what the wait is
//! for, how long it has lasted, and the model's latest thought where it can be read. Under an answer stands what
//! it took and a button that copies it. With "Animations & effects" off in the settings everything simply appears.

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Align, Color32, FontId, Layout, Rect, RichText, Sense, Stroke, Ui, UiBuilder, Vec2, pos2, vec2};

pub use crate::asking::Pick;
use crate::chat::asks::Asked;
use crate::chat::steps::{self, State};
use crate::chat::usage::Spent;
use crate::chat::{Chat, Entry, Pause, Role};
use crate::icons;
use crate::tasklist::{blob, smooth, tint, turned};
use crate::theme::{
    CHAT_ACCENT, CHAT_APPEAR, CHAT_BUBBLE_RADIUS, CHAT_BUBBLE_SHARE, CHAT_BUBBLE_TAIL, CHAT_CARET, CHAT_COPIED_STAYS, CHAT_DOT, CHAT_DOT_GAP,
    CHAT_DOT_LIFT, CHAT_DOT_PACE, CHAT_FADE, CHAT_FOOT_GAP, CHAT_GAP, CHAT_ICON_AT_WORK, CHAT_ICON_BREATH, CHAT_ICON_BURST, CHAT_ICON_FROM_OWN,
    CHAT_ICON_LEAST, CHAT_ICON_RING, CHAT_ICON_SETTLE, CHAT_ICON_SHAKE, CHAT_ICON_TAIL, CHAT_ICON_TURN, CHAT_NOTE_GAP, CHAT_NOTE_ICON, CHAT_NOTE_ROW,
    CHAT_NOTE_TIMED, CHAT_RISE, CHAT_SCROLL_BAR, CHAT_SCROLL_HELD, CHAT_SEND_RISE, CHAT_SEND_TIME, CHAT_SHIMMER_BAND, CHAT_SHIMMER_TURN,
    CHAT_STATUS_HEIGHT, CHAT_STATUS_HOLD, CHAT_STATUS_MORPH, CHAT_STATUS_PAD, CHAT_STEPS_FOLD, CHAT_STEPS_HEAD, CHAT_STEPS_ICON, CHAT_STEPS_LAP,
    CHAT_STEPS_TIME, CHAT_THOUGHT_GLIDE, CHAT_THOUGHT_ROWS, CHAT_WAIT_COUNTED, CHAT_WRITE_CATCH_UP, CHAT_WRITE_FADE, CHAT_WRITE_PACE, CHIP_CORNER,
    CHIP_ICON, CHIP_ICON_AT, CHIP_THUMB, ISLAND_RADIUS, MESSAGE_TEXT_SIZE, PROBLEM, Palette, SIDEBAR_TEXT_SIZE,
};

/// What the panel remembers between frames to move things: it lives in egui's memory, not in the chat.
#[derive(Clone, Default)]
struct Motion {
    /// When each line of the conversation first showed, in egui's time; long ago for lines that were there
    /// when the conversation was first drawn.
    born: Vec<f64>,
    /// The line the assistant is writing, and how many of its letters show so far.
    writing: Option<(usize, f32)>,
    /// For each line, the state of its tool call (none for a line that isn't a note) and since when it is so.
    states: Vec<(Option<State>, f64)>,
    /// For each line, how many seconds its tool call took, when the panel watched it at work.
    took: Vec<Option<f32>>,
}

impl Motion {
    /// Takes in what has changed in `entries` since the last frame: new lines, and tool calls that have
    /// started, are done or have failed.
    // Human: a message that is on its way into the conversation as a pill (`flying`: its words, and when it
    // lands) shows when the pill is nearly there, so the two are never seen side by side.
    fn take_in(&mut self, entries: &[Entry], now: f64, animate: bool, flying: Option<&(String, f64)>) {
        // Human: a conversation that is already there when the panel first shows it doesn't move in; only what
        // is added while one watches does.
        let known = self.born.is_empty() && entries.len() > 1;
        let long_ago = f64::NEG_INFINITY;
        self.born.truncate(entries.len());
        while self.born.len() < entries.len() {
            // Human: the note of a task list that was just worked through shows when the drop that carries the
            // list down from the top of the panel has landed on it.
            let entry = &entries[self.born.len()];
            let carried = !entry.finished.is_empty();
            let lands =
                flying.filter(|(text, _)| entry.role == Role::User && entry.text == *text).map(|(_, lands)| lands - f64::from(CHAT_SEND_TIME) * 0.8);
            let shows = if carried { now + f64::from(crate::liquid::HAND_SHOWS) } else { lands.unwrap_or(now) };
            self.born.push(if known || !animate { long_ago } else { shows });
        }
        // Human: a note's icon plays a short piece when its tool call changes state (starts, is done, fails).
        self.states.truncate(entries.len());
        self.took.resize(entries.len(), None);
        for (index, entry) in entries.iter().enumerate() {
            let state = entry.step.as_ref().map(|step| step.state);
            match self.states.get_mut(index) {
                Some(kept) if kept.0 != state => {
                    // Human: a note that stands for several calls in a row says how long they took together.
                    if kept.0 == Some(State::Running) && kept.1.is_finite() {
                        self.took[index] = Some(self.took[index].unwrap_or(0.0) + seconds(now - kept.1));
                    }
                    *kept = (state, now);
                }
                Some(_) => {}
                None => self.states.push((state, if known || !animate { long_ago } else { now })),
            }
        }
    }
}

/// Where the motion of the chat `key` is kept: each chat has its own, so one that is brought back to the
/// front goes on where it was.
fn motion_id(key: u64) -> egui::Id {
    egui::Id::unique(("conversation-motion", key))
}

/// Forgets what moved in the chat `key`: the conversation is empty, and the next one starts fresh.
pub fn forget(ctx: &egui::Context, key: u64) {
    ctx.data_mut(|data| data.remove::<Motion>(motion_id(key)));
}

/// Draws the conversation of the chat `key`. `animate` is the user's "Animations & effects" setting. Returns
/// what the user did with a question of the assistant's, if anything.
// Human: the newest line stays in view while the assistant writes, unless the user has scrolled up to read.
// Agent: MAIN-THREAD; asks for another frame while something moves or the assistant is at work.
pub fn show(ui: &mut Ui, chat: &Chat, key: u64, scale: f32, palette: Palette, animate: bool) -> Option<Pick> {
    let now = ui.input(|input| input.time);
    let step = ui.input(|input| input.stable_dt).min(0.1);
    let mut motion: Motion = ui.data(|data| data.get_temp(motion_id(key))).unwrap_or_default();
    motion.take_in(&chat.entries, now, animate, crate::offers::flying(ui.ctx()).as_ref());
    let mut moving = false;
    let mut pick = None;
    // Human: only the question that waits for the user can be answered; an older one is history.
    let open = crate::chat::asking(&chat.entries).map(|(at, _)| at);
    let stretches = stretches(&chat.entries);
    let ordinary = ui.visuals().widgets.clone();
    ui.scope(|ui| {
        thin_scroll_bar(ui, palette);
        egui::ScrollArea::vertical().id_salt(("conversation", key)).auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
            ui.visuals_mut().widgets = ordinary;
            ui.add_space(12.0);
            ui.spacing_mut().item_spacing.y = 0.0;
            let last = chat.entries.len().saturating_sub(1);
            let mut index = 0;
            while let Some(entry) = chat.entries.get(index) {
                if index > 0 {
                    ui.add_space(CHAT_GAP);
                }
                // Human: the notes on one stretch of work are drawn together, so they can fold into one line.
                if let Some(stretch) = stretches.iter().find(|stretch| stretch.start == index) {
                    let over = stretch.end < chat.entries.len() || !chat.running();
                    let notes = Notes { entries: &chat.entries, motion: &motion, over, key, scale, palette, now, animate };
                    moving |= notes.show(ui, stretch.clone());
                    index = stretch.end;
                    continue;
                }
                // Human: a message of the user's comes up from the message box; everything else settles in.
                let (rise, time) = if entry.role == Role::User { (CHAT_SEND_RISE, CHAT_SEND_TIME) } else { (CHAT_RISE, CHAT_APPEAR) };
                let arrived = arrival(now, motion.born[index], time);
                moving |= arrived < 1.0;
                // Human: the assistant's newest words are let out letter by letter at an even pace.
                let newest = animate && index == last && entry.role == Role::Assistant;
                let (shown, done) = if newest { written(&mut motion, index, entry, chat.running(), step) } else { (entry.text.as_str(), true) };
                moving |= !done;
                // Human: what an answer took stands under the last words of a run, once the run is over.
                let closing = !(index == last && chat.running()) && chat.entries.get(index + 1).is_none_or(|next| next.role == Role::User);
                let looks = Looks {
                    scale,
                    palette,
                    live: open == Some(index),
                    at: index,
                    total: &chat.total,
                    caret: (newest && (chat.running() || !done)).then_some(now),
                    closing,
                };
                ui.add_space((1.0 - arrived) * rise);
                let picked = ui.scope(|ui| {
                    ui.set_opacity(arrived);
                    line(ui, entry, shown, looks)
                });
                if picked.inner.is_some() {
                    pick = picked.inner;
                }
                index += 1;
            }
            // Human: under its own words the drop after them says that more is coming; without motion, dots do.
            let own_words = animate && chat.pause() == Some(Pause::Writing) && chat.entries.last().is_some_and(|entry| entry.role == Role::Assistant);
            if let Some(pause) = chat.pause().filter(|_| !own_words && !chat.entries.last().is_some_and(at_work)) {
                ui.add_space(CHAT_GAP);
                let wait = (pause != Pause::Writing).then(|| Wait {
                    words: pause.words(chat.answering().unwrap_or_default()),
                    lasted: chat.paused(),
                    thought: flow(chat.thought()),
                });
                waiting(ui, wait.as_ref(), scale, palette, animate.then_some(now));
            }
            pick = queued(ui, chat, scale, palette).or(pick.take());
            ui.add_space(12.0);
        });
    });
    // Human: something arriving or growing moves at full rate; what only shows the assistant is at work (dots,
    // shimmer) is drawn at the idle rate. New messages wake the window themselves.
    if animate && moving {
        ui.request_repaint();
    } else if animate && chat.running() {
        ui.ctx().request_repaint_after(crate::theme::IDLE_FRAME);
    } else if chat.running() {
        // Human: nothing moves, but the seconds of a wait still count up.
        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
    }
    ui.data_mut(|data| data.insert_temp(motion_id(key), motion));
    pick
}

/// The messages that haven't gone to the assistant yet, at the end of the conversation. Returns
/// [`Pick::Unqueue`] when the user takes one back.
// Human: a message written while the assistant works stands greyed at the end until the assistant has taken
// it: at its next step when it is steered, or when the run is over. Under it stands which of the two, and a
// button that takes the message back, for the one sent too soon or no longer wanted.
fn queued(ui: &mut Ui, chat: &Chat, scale: f32, palette: Palette) -> Option<Pick> {
    let mut taken_back = None;
    for (place, (text, steered)) in chat.pending().into_iter().enumerate() {
        ui.add_space(CHAT_GAP);
        bubble(ui, palette, |ui| flowing(ui, &text, MESSAGE_TEXT_SIZE * scale, palette.muted));
        ui.add_space(CHAT_NOTE_GAP);
        let says = if steered { "Goes to the assistant at its next step" } else { "Queued until the assistant is done" };
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let font = FontId::proportional((SIDEBAR_TEXT_SIZE - 1.0) * scale);
            let words = ui.painter().layout_no_wrap("Remove".to_owned(), font.clone(), palette.muted);
            let size = vec2(CHIP_ICON_AT + CHIP_ICON / 2.0 + 4.0 + words.size().x + 8.0, words.size().y + 6.0);
            let (chip, response) = ui.allocate_exact_size(size, Sense::click());
            let ink = if response.hovered() { palette.foreground } else { palette.muted };
            ui.painter().rect_stroke(chip, CHIP_CORNER, Stroke::new(1.0, palette.outline), egui::StrokeKind::Inside);
            icons::paint(
                ui.painter(),
                Rect::from_center_size(pos2(chip.left() + CHIP_ICON_AT, chip.center().y), Vec2::splat(CHIP_ICON - 1.0)),
                "x",
                ink,
            );
            ui.painter().galley(pos2(chip.left() + CHIP_ICON_AT + CHIP_ICON / 2.0 + 4.0, chip.center().y - words.size().y / 2.0), words, ink);
            if response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Don't send this message").clicked() {
                taken_back = Some(Pick::Unqueue(place));
            }
            ui.add(egui::Label::new(RichText::new(says).font(font).color(palette.muted)).selectable(false));
        });
    }
    taken_back
}

/// The seconds `time` as the small number an animation counts with.
#[expect(clippy::cast_possible_truncation, reason = "the seconds a tool call took")]
fn seconds(time: f64) -> f32 {
    time.clamp(0.0, 1e6) as f32
}

/// The conversation's scroll bar: a thin rounded handle without a track, faint until the conversation is
/// scrolled or the pointer is over it, and in the liquid's colour while it is dragged.
// Agent: the handle takes its colour from the widgets' text colours, which the conversation sets back for what
// it draws inside ([`show`]).
fn thin_scroll_bar(ui: &mut Ui, palette: Palette) {
    let style = ui.style_mut();
    style.spacing.scroll = egui::style::ScrollStyle {
        bar_width: CHAT_SCROLL_HELD,
        floating_width: CHAT_SCROLL_BAR,
        bar_inner_margin: 2.0,
        handle_min_length: 36.0,
        dormant_handle_opacity: 0.3,
        active_handle_opacity: 0.55,
        interact_handle_opacity: 0.9,
        dormant_background_opacity: 0.0,
        active_background_opacity: 0.0,
        interact_background_opacity: 0.0,
        fade: egui::style::ScrollFadeStyle { strength: 1.0, size: CHAT_FADE },
        ..egui::style::ScrollStyle::floating()
    };
    let round = egui::CornerRadius::same(3);
    for (widget, color) in [
        (&mut style.visuals.widgets.inactive, palette.muted),
        (&mut style.visuals.widgets.hovered, palette.muted),
        (&mut style.visuals.widgets.active, CHAT_ACCENT),
    ] {
        widget.fg_stroke.color = color;
        widget.corner_radius = round;
    }
}

/// Whether `entry` is a note on something the assistant did: a tool call, not a question that is still a card
/// and not a task list that was worked through.
fn is_note(entry: &Entry) -> bool {
    entry.role == Role::Step && entry.finished.is_empty() && entry.asked.as_ref().is_none_or(|asked| asked.suggests && asked.closed)
}

/// The stretches of work in `entries`: each run of notes that stand one after the other.
fn stretches(entries: &[Entry]) -> Vec<std::ops::Range<usize>> {
    let mut found = Vec::new();
    let mut from = None;
    for (index, entry) in entries.iter().enumerate() {
        match (is_note(entry), from) {
            (true, None) => from = Some(index),
            (false, Some(start)) => {
                found.push(start..index);
                from = None;
            }
            _ => {}
        }
    }
    found.extend(from.map(|start| start..entries.len()));
    found
}

/// What the one line of a folded stretch says: how many tool calls it stands for, and how long they took when
/// that is known and worth saying ("5 steps · 8 s").
fn steps_say(calls: u32, took: Option<f32>) -> String {
    let steps = if calls == 1 { "1 step".to_owned() } else { format!("{calls} steps") };
    match took.filter(|took| *took >= 1.0) {
        Some(took) => format!("{steps}  ·  {}", crate::chat::span(std::time::Duration::from_secs_f32(took))),
        None => steps,
    }
}

/// The notes on one stretch of work, and what is needed to draw them.
#[derive(Clone, Copy)]
struct Notes<'a> {
    entries: &'a [Entry],
    motion: &'a Motion,
    /// The assistant has gone on to something else, or the run is over: the stretch folds.
    over: bool,
    key: u64,
    scale: f32,
    palette: Palette,
    now: f64,
    animate: bool,
}

impl Notes<'_> {
    /// `on` as a share that glides between 0 and 1, remembered under `id`.
    fn share(&self, ui: &Ui, id: egui::Id, on: bool) -> f32 {
        if self.animate {
            eased(ui.ctx().animate_bool_with_time(id, on, CHAT_STEPS_TIME))
        } else if on {
            1.0
        } else {
            0.0
        }
    }

    /// Draws the notes `stretch`. Returns whether something in them still moves.
    // Human: while the assistant is at this stretch every note shows, each sliding in under the one before
    // it. Once it goes on, a stretch of two or more notes folds into one line saying how many steps it was;
    // a click on that line opens the notes again.
    fn show(&self, ui: &mut Ui, stretch: std::ops::Range<usize>) -> bool {
        let Notes { entries, motion, scale, palette, now, animate, .. } = *self;
        let id = egui::Id::unique(("chat-steps", self.key, stretch.start));
        let mut opened: bool = ui.data(|data| data.get_temp(id)).unwrap_or(false);
        let folded = self.share(ui, id.with("folded"), self.over && stretch.len() >= CHAT_STEPS_FOLD);
        let open = self.share(ui, id.with("open"), opened);
        if folded > 0.0 && self.head(ui, id, &entries[stretch.clone()], stretch.start, (folded, open)) {
            opened = !opened;
            ui.data_mut(|data| data.insert_temp(id, opened));
        }
        let shown = 1.0 - folded * (1.0 - open);
        if shown <= 0.0 {
            return false;
        }
        let grown: Vec<f32> = stretch.clone().map(|index| arrival(now, motion.born[index], CHAT_APPEAR)).collect();
        let lead = CHAT_NOTE_GAP * folded;
        let rows: f32 = grown.iter().enumerate().map(|(place, grown)| note_height(place > 0, scale) * grown).sum();
        let (place, _) = ui.allocate_exact_size(vec2(ui.available_width(), (lead + rows) * shown), Sense::hover());
        let whole = Rect::from_min_size(place.min, vec2(place.width(), lead + rows));
        let mut inside = ui.new_child(UiBuilder::new().max_rect(whole).layout(Layout::top_down(Align::Min)));
        // Human: a ring that bursts around an icon reaches a little past its row.
        let window = if shown < 1.0 { place.expand2(vec2(CHAT_ICON_BURST + 1.0, 0.0)) } else { place.expand(CHAT_ICON_BURST + 1.0) };
        inside.set_clip_rect(window.intersect(ui.clip_rect()));
        inside.multiply_opacity(shown);
        inside.spacing_mut().item_spacing.y = 0.0;
        inside.add_space(lead);
        let mut moving = false;
        for (place, index) in stretch.enumerate() {
            let entry = &entries[index];
            // Human: looking at the document or reading the guide takes the app no time at all. Its note still
            // shows the tool at work for a moment, so the eye is seen to look and the book to turn a page.
            let shown_until = motion.born[index] + f64::from(CHAT_ICON_LEAST);
            let still_at_work = held(entry, animate && now < shown_until);
            let clock = animate.then(|| Clock { now, since: motion.states[index].1.max(shown_until) });
            moving |= still_at_work.is_some() || grown[place] < 1.0 || clock.is_some_and(|clock| clock.settling() < 1.0);
            let took = motion.took.get(index).copied().flatten();
            note(&mut inside, still_at_work.as_ref().unwrap_or(entry), Row { scale, palette, clock, took, grown: grown[place], linked: place > 0 });
        }
        moving
    }

    /// The one line of the folded stretch `notes`, which starts at the line `at` of the conversation: the
    /// icons of its tools, how many steps it was, and an arrow. `shares` are how far the stretch is folded and
    /// how far the user has opened it again. Returns whether it was clicked.
    fn head(&self, ui: &mut Ui, id: egui::Id, notes: &[Entry], at: usize, shares: (f32, f32)) -> bool {
        let (folded, open) = shares;
        let palette = self.palette;
        let high = CHAT_STEPS_HEAD * self.scale;
        let (place, _) = ui.allocate_exact_size(vec2(ui.available_width(), high * folded), Sense::hover());
        let line = Rect::from_min_size(place.min, vec2(place.width(), high));
        let painter = ui.painter().with_clip_rect(place.expand2(vec2(2.0, 0.0)).intersect(ui.clip_rect()));
        let calls: u32 = notes.iter().map(|entry| entry.step.as_ref().map_or(1, |step| step.times.max(1))).sum();
        let took = (at..at + notes.len()).filter_map(|index| self.motion.took.get(index).copied().flatten()).reduce(|sum, took| sum + took);
        let words = painter.layout_no_wrap(steps_say(calls, took), FontId::proportional((SIDEBAR_TEXT_SIZE - 1.0) * self.scale), palette.muted);
        // The icons of the first few different tools, each lying a little over the one before it.
        let mut tools: Vec<&str> = Vec::new();
        for entry in notes {
            let icon = entry.step.as_ref().map_or(steps::PLAIN_ICON, |step| steps::icon(&step.tool));
            if !tools.contains(&icon) && tools.len() < 3 {
                tools.push(icon);
            }
        }
        #[expect(clippy::cast_precision_loss, reason = "at most three icons")]
        let icons_end = line.left() + CHAT_STEPS_ICON + (CHAT_STEPS_ICON - CHAT_STEPS_LAP) * tools.len().saturating_sub(1) as f32;
        let words_at = pos2(icons_end + LEAD_GAP, line.center().y - words.size().y / 2.0);
        let arrow = Rect::from_center_size(pos2(words_at.x + words.size().x + 11.0, line.center().y + 0.5), Vec2::splat(12.0));
        let pressed = ui.interact(Rect::from_min_max(place.min, pos2(arrow.right() + 4.0, place.bottom())), id.with("head"), Sense::click());
        let ink = if pressed.hovered() { palette.foreground } else { palette.muted }.gamma_multiply(folded);
        let mut centre = pos2(line.left() + CHAT_STEPS_ICON / 2.0, line.center().y);
        for icon in tools {
            painter.circle(centre, CHAT_STEPS_ICON / 2.0, palette.island, Stroke::new(1.0, palette.outline.gamma_multiply(folded)));
            icons::paint(&painter, Rect::from_center_size(centre, Vec2::splat(CHAT_STEPS_ICON * 0.58)), icon, palette.muted.gamma_multiply(folded));
            centre.x += CHAT_STEPS_ICON - CHAT_STEPS_LAP;
        }
        painter.galley(words_at, words, ink);
        icons::paint(&painter, arrow, if open > 0.5 { "chevron-up" } else { "chevron-down" }, ink);
        pressed.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    }
}

/// How far a line born at `born` has come in, when coming in takes `time` seconds: 0 just born, 1 in place;
/// it slows down as it gets there.
#[expect(clippy::cast_possible_truncation, reason = "a share between 0 and 1")]
fn arrival(now: f64, born: f64, time: f32) -> f32 {
    let share = ((now - born) / f64::from(time)).clamp(0.0, 1.0) as f32;
    1.0 - (1.0 - share).powi(3)
}

/// The part of the assistant's line `entry` that shows this frame, and whether that is all of it.
// Human: the model's words arrive in bursts. They are let out at a steady pace, and faster the further the
// shown text is behind, so the reader never waits long for what is already there.
#[expect(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "letter counts of a chat message")]
fn written<'a>(motion: &mut Motion, index: usize, entry: &'a Entry, running: bool, step: f32) -> (&'a str, bool) {
    let all = entry.text.chars().count() as f32;
    let so_far = match motion.writing {
        Some((line, letters)) if line == index => letters.min(all),
        // A line first seen while it is being written starts empty; one that is finished is simply there.
        _ if running => 0.0,
        _ => all,
    };
    let letters = (so_far + (CHAT_WRITE_PACE + (all - so_far) * CHAT_WRITE_CATCH_UP) * step).min(all);
    motion.writing = Some((index, letters));
    let end = entry.text.char_indices().nth(letters as usize).map_or(entry.text.len(), |(at, _)| at);
    (&entry.text[..end], letters >= all)
}

/// The note `entry` as it is shown while `hold` keeps its finished tool call looking at work; `None` when it
/// is shown as it is.
fn held(entry: &Entry, hold: bool) -> Option<Entry> {
    let step = entry.step.as_ref().filter(|step| hold && step.state == State::Done)?;
    let mut shown = entry.clone();
    shown.step = Some(crate::chat::steps::Step { state: State::Running, ..step.clone() });
    Some(shown)
}

/// Whether `entry` is the note of a tool that is at work now.
fn at_work(entry: &Entry) -> bool {
    entry.step.as_ref().is_some_and(|step| step.state == State::Running)
}

/// How a line of the conversation is drawn.
#[derive(Clone, Copy)]
struct Looks<'a> {
    /// How much larger than small the text is (the settings' chat text size).
    scale: f32,
    palette: Palette,
    /// The line is the question that waits for the user: its answers can be picked.
    live: bool,
    /// The line's place in the conversation.
    at: usize,
    /// What the whole conversation took, for the hover of an answer's own sum.
    total: &'a Spent,
    /// The assistant is writing this line: egui's time, by which the drop after its words moves.
    caret: Option<f64>,
    /// The line is the last of a run that is over: what the run took stands under it.
    closing: bool,
}

/// What a run took and cost, in faint words; the hover says the parts.
fn spent_words(ui: &mut Ui, spent: &Spent, total: &Spent, scale: f32, palette: Palette) {
    let words = RichText::new(spent.line()).size((SIDEBAR_TEXT_SIZE - 1.0) * scale).color(palette.quiet);
    ui.add(egui::Label::new(words).selectable(false)).on_hover_text(spent.details(total).join("\n"));
}

/// The line under an answer: what the run took, where that was counted, and a button that copies the answer.
// Human: the numbers are there for whoever wants them and out of the way of the conversation: smaller and
// fainter than a note. The button turns into a tick for a moment when it has copied.
fn answer_foot(ui: &mut Ui, entry: &Entry, looks: Looks) {
    let Looks { scale, palette, at, total, .. } = looks;
    ui.add_space(CHAT_FOOT_GAP);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if let Some(spent) = &entry.spent {
            spent_words(ui, spent, total, scale, palette);
        }
        let now = ui.input(|input| input.time);
        let id = egui::Id::unique(("chat-answer-copied", at));
        let since: Option<f64> = ui.data(|data| data.get_temp(id));
        let copied = since.is_some_and(|since| now - since < CHAT_COPIED_STAYS);
        let (slot, response) = ui.allocate_exact_size(Vec2::splat(CHAT_NOTE_ICON), Sense::click());
        let ink = if response.hovered() || copied { palette.foreground } else { palette.quiet };
        let icon = if copied { steps::ANSWERED_ICON } else { steps::COPY_ICON };
        icons::paint(ui.painter(), Rect::from_center_size(slot.center(), Vec2::splat(CHIP_ICON)), icon, ink);
        if response.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(if copied { "Copied" } else { "Copy the answer" }).clicked() {
            ui.ctx().copy_text(entry.text.clone());
            ui.data_mut(|data| data.insert_temp(id, now));
        }
        if copied {
            // The button turns back by itself.
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        }
    });
}

/// The assistant's words while it writes them: the newest letters fade in, and a small drop stands after the
/// last one. `more` says that words which have arrived are still to be let out.
// Human: the drop is the one of the task list, so the eye knows it: something is being made here.
// Agent: painted as one laid-out text, not a label; while it is written the line can't be selected.
fn streaming(ui: &mut Ui, shown: &str, more: bool, looks: Looks, time: f64) {
    let Looks { scale, palette, .. } = looks;
    let font = FontId::proportional(MESSAGE_TEXT_SIZE * scale);
    let format = |color: Color32| TextFormat { font_id: font.clone(), color, ..TextFormat::default() };
    let letters = shown.chars().count();
    let fading = if more { CHAT_WRITE_FADE.min(letters) } else { 0 };
    let cut = shown.char_indices().nth(letters - fading).map_or(shown.len(), |(at, _)| at);
    let mut job = LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    job.append(&shown[..cut], 0.0, format(palette.foreground));
    // Each of the last letters is a little fainter than the one before it.
    let step = 1.0 / (f32::from(u8::try_from(fading).unwrap_or(u8::MAX)) + 1.0);
    let mut strength = 1.0;
    for (at, letter) in shown[cut..].char_indices() {
        strength -= step;
        job.append(&shown[cut + at..cut + at + letter.len_utf8()], 0.0, format(palette.foreground.gamma_multiply(strength.max(0.0))));
    }
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let row = ui.fonts_mut(|fonts| fonts.row_height(&font));
    let (place, _) = ui.allocate_exact_size(vec2(ui.available_width(), galley.size().y.max(row)), Sense::hover());
    let end = galley.pos_from_cursor(egui::text::CCursor::new(letters));
    ui.painter().galley(place.min, galley, palette.foreground);
    let centre = place.min + vec2(end.right() + CHAT_CARET + 4.0, end.top() + row / 2.0);
    blob(ui.painter(), centre, CHAT_CARET, Some(time));
}

/// What the hover of the counter says while an answer is under way.
const SPENDING_HINT: &str = "What this answer has taken so far. Its sum and price follow when it is done.";

/// What the counter by the message box says, and what its hover says: the answer under way so far, or what
/// the last answer took with its parts and the conversation's sum. `None` when nothing was counted.
// Human: while the assistant works the counter counts up in a pill between the conversation and the message
// box, where it is seen without scrolling; the finished sum stands under its answer. Folded away, the
// conversation can't be seen, so the pill over the bar keeps the last sum.
fn counter_words(chat: &Chat) -> Option<(String, String)> {
    let (spent, under_way) = chat.counter()?;
    let hover = if under_way { SPENDING_HINT.to_owned() } else { spent.details(&chat.total).join("\n") };
    Some((spent.line(), hover))
}

/// How much room the counter's words take; `None` when nothing was counted, or, with `under_way`, when no
/// answer is being counted right now.
#[must_use]
pub fn counter_size(ctx: &egui::Context, chat: &Chat, under_way: bool) -> Option<Vec2> {
    if under_way && !chat.counter().is_some_and(|(_, counting)| counting) {
        return None;
    }
    let (words, _) = counter_words(chat)?;
    Some(ctx.fonts_mut(|fonts| fonts.layout_no_wrap(words, FontId::proportional(SIDEBAR_TEXT_SIZE - 1.0), Color32::WHITE)).size())
}

/// Paints the counter into `slot`, its words in the middle, at `opacity`; the hover says the parts.
// Agent: MAIN-THREAD; nothing in it can be clicked.
pub fn counter(ui: &mut Ui, slot: egui::Rect, chat: &Chat, palette: Palette, opacity: f32) {
    let Some((words, hover)) = counter_words(chat) else { return };
    let ink = palette.quiet.gamma_multiply(opacity);
    let words = ui.painter().layout_no_wrap(words, FontId::proportional(SIDEBAR_TEXT_SIZE - 1.0), ink);
    let place = egui::Rect::from_center_size(slot.center(), words.size());
    ui.allocate_rect(slot, Sense::hover()).on_hover_text(hover);
    ui.painter().galley(place.min, words, ink);
}

/// One line of the conversation; `shown` is as much of its text as shows. Returns what the user did with it,
/// when it is a question of the assistant's.
fn line(ui: &mut Ui, entry: &Entry, shown: &str, looks: Looks) -> Option<Pick> {
    let Looks { scale, palette, live, at, total, caret, closing } = looks;
    let words = |text: &str, color: Color32| egui::Label::new(RichText::new(text).size(MESSAGE_TEXT_SIZE * scale).color(color)).wrap();
    match entry.role {
        Role::User => bubble(ui, palette, |ui| {
            if entry.pointed.is_empty() {
                flowing(ui, shown, MESSAGE_TEXT_SIZE * scale, palette.foreground);
            } else {
                pointing(ui, entry, scale, palette);
            }
        }),
        Role::Assistant => {
            match caret {
                Some(time) => streaming(ui, shown, shown.len() < entry.text.len(), looks, time),
                None => {
                    ui.add(words(shown, palette.foreground));
                }
            }
            // Human: the line under an answer waits until the answer's words are all out.
            if closing && shown.len() == entry.text.len() {
                answer_foot(ui, entry, looks);
            }
        }
        // Human: a question is a card with its answers; suggestions nobody can pick any more are one quiet note.
        // Human: a task list that was worked through is a note that opens to show its tasks.
        Role::Step if !entry.finished.is_empty() => crate::tasklist::filed(ui, entry, at, scale, palette),
        Role::Step => match &entry.asked {
            Some(asked) if !(asked.suggests && asked.closed) => return question(ui, asked, scale, palette, live),
            // A note is drawn with the others of its stretch ([`Notes`]); one that stands alone here is at rest.
            _ => note(ui, entry, Row { scale, palette, clock: None, took: None, grown: 1.0, linked: false }),
        },
        Role::Problem => {
            ui.add(words(shown, PROBLEM));
            trouble(ui, entry, scale, palette);
            if let Some(spent) = &entry.spent {
                ui.add_space(CHAT_FOOT_GAP);
                spent_words(ui, spent, total, scale, palette);
            }
        }
    }
    None
}

/// What is known about the problem `entry` under its words, and a button that copies all of it.
// Human: "Internal Server Error" alone can't be acted on or reported. Under it stand who was answering and
// through what, what the assistant was doing, how far into the answer and when; the button puts these, the
// problem's words and the app's version on the clipboard as text to paste into a report.
fn trouble(ui: &mut Ui, entry: &Entry, scale: f32, palette: Palette) {
    let font = FontId::proportional((SIDEBAR_TEXT_SIZE - 1.0) * scale);
    let quiet = palette.quiet;
    ui.add_space(5.0);
    for (name, fact) in &entry.about {
        let mut row = LayoutJob::default();
        row.append(&format!("{name}  "), 0.0, TextFormat { font_id: font.clone(), color: quiet, ..TextFormat::default() });
        row.append(fact, 0.0, TextFormat { font_id: font.clone(), color: palette.muted, ..TextFormat::default() });
        ui.add(egui::Label::new(row).wrap());
        ui.add_space(2.0);
    }
    ui.add_space(4.0);
    let now = ui.input(|input| input.time);
    let id = egui::Id::unique(("chat-problem-copied", &entry.text, &entry.about));
    let since: Option<f64> = ui.data(|data| data.get_temp(id));
    let copied = since.is_some_and(|since| now - since < CHAT_COPIED_STAYS);
    let (icon, says) = if copied { (steps::ANSWERED_ICON, "Copied") } else { (steps::COPY_ICON, "Copy details") };
    let words = ui.painter().layout_no_wrap(says.to_owned(), font.clone(), palette.muted);
    let size = vec2(CHIP_ICON_AT + CHIP_ICON / 2.0 + 5.0 + words.size().x + 8.0, words.size().y + 8.0);
    let (chip, response) = ui.allocate_exact_size(size, Sense::click());
    let ink = if response.hovered() || copied { palette.foreground } else { palette.muted };
    ui.painter().rect_stroke(chip, CHIP_CORNER, egui::Stroke::new(1.0, palette.outline), egui::StrokeKind::Inside);
    icons::paint(ui.painter(), egui::Rect::from_center_size(pos2(chip.left() + CHIP_ICON_AT, chip.center().y), Vec2::splat(CHIP_ICON)), icon, ink);
    ui.painter().galley(pos2(chip.left() + CHIP_ICON_AT + CHIP_ICON / 2.0 + 5.0, chip.center().y - words.size().y / 2.0), words, ink);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.clicked() {
        ui.ctx().copy_text(entry.report());
        ui.data_mut(|data| data.insert_temp(id, now));
    }
    if copied {
        // The button turns back by itself.
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
    }
}

/// The bubble of a message of the user's: it stands at the right of the panel, as wide as what is `inside` it
/// and no wider than a message may be.
fn bubble(ui: &mut Ui, palette: Palette, inside: impl FnOnce(&mut Ui)) {
    let width = ui.available_width() * CHAT_BUBBLE_SHARE;
    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        // Human: the corner at the bottom right is nearly square: the bubble points at the message box it came from.
        let corners = egui::CornerRadius { nw: CHAT_BUBBLE_RADIUS, ne: CHAT_BUBBLE_RADIUS, sw: CHAT_BUBBLE_RADIUS, se: CHAT_BUBBLE_TAIL };
        let drawn = egui::Frame::new().fill(palette.hover).corner_radius(corners).inner_margin(egui::Margin::symmetric(12, 8)).show(ui, |ui| {
            ui.set_max_width(width);
            inside(ui);
        });
        // A step taken from the assistant's suggestions flies to where the newest message stands.
        crate::offers::lands_at(ui, drawn.response.rect);
    });
}

/// The words of a message in its bubble: every line starts at the bubble's left edge.
// Human: the bubble is laid out from the right, to stand at the right of the panel. Words put straight into it
// would be laid out from the right too: each line would end at the right edge and start wherever it happened
// to, which reads badly. So the words are measured first and given a place of their own that fills from the left.
fn flowing(ui: &mut Ui, text: &str, size: f32, color: Color32) {
    let job = LayoutJob::simple(text.to_owned(), FontId::proportional(size), color, ui.available_width());
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    ui.allocate_ui_with_layout(vec2(galley.size().x, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.add(egui::Label::new(galley));
    });
}

/// Draws the question that waits for the user in the chat at the bottom of `body`, for a sidebar view that
/// doesn't show the conversation. Returns the room left over it for the view, and what the user did.
// Human: with the layers or the components open the conversation can't be seen, and no status line stands
// over the message box. The question then sits just over the message box, and the view gives way to it.
// Agent: MAIN-THREAD; draws nothing and returns `body` whole when nothing is asked or the panel is too short.
pub fn pinned(ui: &mut Ui, chat: &Chat, body: egui::Rect, scale: f32, palette: Palette) -> (egui::Rect, Option<Pick>) {
    let Some((_, asked)) = crate::chat::asking(&chat.entries) else { return (body, None) };
    let sheet = crate::asking::sheet(ui.ctx(), asked, body.width(), crate::asking::Style { scale, history: false, palette });
    let card = egui::Rect::from_min_max(pos2(body.left(), body.bottom() - CHAT_GAP - sheet.size.y), pos2(body.right(), body.bottom() - CHAT_GAP));
    let left = egui::Rect::from_min_max(body.min, pos2(body.right(), card.top() - CHAT_GAP));
    if left.height() < sheet.size.y / 2.0 {
        return (body, None);
    }
    ui.painter().rect_stroke(card, ISLAND_RADIUS - 4.0, egui::Stroke::new(1.0, palette.outline), egui::StrokeKind::Inside);
    (left, crate::asking::paint(ui, &sheet, card.min, true, palette))
}

/// A question of the assistant's in the conversation: a card as wide as the panel with the question, its
/// answers to pick from while it is `live`, and the answers already given.
fn question(ui: &mut Ui, asked: &Asked, scale: f32, palette: Palette, live: bool) -> Option<Pick> {
    let sheet = crate::asking::sheet(ui.ctx(), asked, ui.available_width(), crate::asking::Style { scale, history: true, palette });
    let (card, _) = ui.allocate_exact_size(sheet.size, Sense::hover());
    ui.painter().rect_stroke(card, ISLAND_RADIUS - 4.0, egui::Stroke::new(1.0, palette.outline), egui::StrokeKind::Inside);
    crate::asking::paint(ui, &sheet, card.min, live, palette)
}

/// A message of the user's that points at layers: what was selected as chips over it, and each layer pasted
/// into the sentence as a chip in its place, as the message box showed them.
// Human: the chips are drawn the way the message box draws them (an outline, the kind's icon, the name), so
// a message looks sent as it looked written.
fn pointing(ui: &mut Ui, entry: &Entry, scale: f32, palette: Palette) {
    /// Space between two chips of what was selected.
    const BETWEEN: f32 = 4.0;
    let font = FontId::proportional(MESSAGE_TEXT_SIZE * scale);
    let plain = TextFormat { font_id: font.clone(), color: palette.foreground, ..TextFormat::default() };
    let tinted = TextFormat { font_id: font, color: palette.muted, background: palette.island, ..TextFormat::default() };
    let mut pasted: Vec<(&std::ops::Range<usize>, &crate::chat::Pointed)> = entry
        .pointed
        .iter()
        .filter_map(|layer| layer.at.as_ref().map(|at| (at, layer)))
        .filter(|(at, _)| entry.text.get(at.start..at.end).is_some())
        .collect();
    pasted.sort_by_key(|(at, _)| at.start);
    let mut job = LayoutJob::default();
    // Where each chip is in what is laid out, in letters, with its icon.
    let mut chips = Vec::new();
    let (mut from, mut letters) = (0, 0);
    for (at, layer) in pasted {
        if at.start < from {
            continue;
        }
        let before = &entry.text[from..at.start];
        job.append(before, 0.0, plain.clone());
        letters += before.chars().count();
        let chip = crate::chips::padded(&entry.text[at.start..at.end]);
        job.append(&chip, 0.0, tinted.clone());
        chips.push((letters, letters + chip.chars().count(), layer));
        letters += chip.chars().count();
        from = at.end;
    }
    job.append(&entry.text[from..], 0.0, plain);
    job.wrap.max_width = ui.available_width();
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    let selected: Vec<&crate::chat::Pointed> = entry.pointed.iter().filter(|layer| layer.at.is_none()).collect();
    // Human: the bubble is as wide as what is in it (the row of chips, or the longest line of the message),
    // like a message without chips, and no wider than a message may be.
    let row: f32 = selected.iter().map(|layer| crate::sidebar::layer_chip_width(ui, &layer.name) + BETWEEN).sum::<f32>() - BETWEEN;
    let words = if entry.text.is_empty() { 0.0 } else { galley.size().x };
    let width = row.max(words).min(ui.available_width());
    ui.allocate_ui_with_layout(vec2(width, 0.0), Layout::top_down(Align::Min), |ui| {
        ui.set_max_width(width);
        ui.spacing_mut().item_spacing = vec2(BETWEEN, 5.0);
        if !selected.is_empty() {
            ui.horizontal(|ui| {
                for layer in &selected {
                    crate::sidebar::layer_chip(ui, layer.icon, &layer.name, palette);
                }
            });
        }
        if entry.text.is_empty() {
            return;
        }
        let (place, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
        ui.painter().galley(place.min, std::sync::Arc::clone(&galley), palette.foreground);
        for (start, end, layer) in chips {
            let (start, end) = (galley.pos_from_cursor(egui::text::CCursor::new(start)), galley.pos_from_cursor(egui::text::CCursor::new(end)));
            // A chip too long for its line is left as tinted text.
            if (start.top() - end.top()).abs() > 1.0 {
                continue;
            }
            let chip = egui::Rect::from_min_max(start.left_top(), end.left_bottom()).translate(place.min.to_vec2());
            ui.painter().rect_stroke(chip, CHIP_CORNER, egui::Stroke::new(1.0, palette.outline), egui::StrokeKind::Inside);
            let middle = pos2(chip.left() + CHIP_ICON_AT, chip.center().y);
            // A pasted picture shows itself, tiny, and larger under the pointer.
            if let Some(file) = &layer.picture
                && crate::chat::pasted::paint_thumb(ui, ui.painter(), egui::Rect::from_center_size(middle, Vec2::splat(CHIP_THUMB)), file)
            {
                ui.interact(chip, egui::Id::unique(("sent-picture", file, chip.min.x.to_bits(), chip.min.y.to_bits())), Sense::hover())
                    .on_hover_ui(|ui| crate::chat::pasted::show_preview(ui, file));
                continue;
            }
            icons::paint(ui.painter(), egui::Rect::from_center_size(middle, Vec2::splat(CHIP_ICON)), layer.icon, palette.muted);
        }
    });
}

/// What a note says: what the tool does or did, and in quieter letters what the call was about and how often
/// it was made. `light` is how far a light has run through the words of a tool at work, from 0 to 1; without
/// it they are plain.
fn note_words(entry: &Entry, scale: f32, palette: Palette, light: Option<f32>) -> LayoutJob {
    let font = FontId::proportional((SIDEBAR_TEXT_SIZE - 1.0) * scale);
    let quiet = palette.quiet;
    let mut text = LayoutJob::default();
    let mut say = |words: &str, color: Color32| text.append(words, 0.0, TextFormat { font_id: font.clone(), color, ..TextFormat::default() });
    match &entry.step {
        Some(step) if step.state == State::Running => {
            let doing = if step.named { entry.text.clone() } else { steps::doing(&step.tool) };
            lit(&format!("{doing}…"), palette.foreground.gamma_multiply(0.85), light, &mut say);
        }
        _ => say(&entry.text, palette.muted),
    }
    if let Some(step) = &entry.step {
        // Human: a call that failed says why, where the tool said so; that tells more than what was wanted.
        let more = if step.why.is_empty() { step.detail.as_str() } else { steps::brief(&step.why) };
        if !more.is_empty() {
            say(&format!("  ·  {more}"), quiet);
        }
        if step.times > 1 {
            say(&format!("  ×{}", step.times), quiet);
        }
    }
    text
}

/// Says `words` in `color` with `say`, letter by letter where a light runs through them: `light` is how far it
/// has come, from 0 before the first letter to 1 after the last.
fn lit(words: &str, color: Color32, light: Option<f32>, say: &mut impl FnMut(&str, Color32)) {
    let Some(light) = light else {
        say(words, color);
        return;
    };
    let letters = words.chars().fold(0.0_f32, |count, _| count + 1.0);
    let head = light * (letters + 2.0 * CHAT_SHIMMER_BAND) - CHAT_SHIMMER_BAND;
    let mut place = 0.0_f32;
    for (at, letter) in words.char_indices() {
        let near = 1.0 - smooth(0.0, CHAT_SHIMMER_BAND, (place - head).abs());
        say(&words[at..at + letter.len_utf8()], tint(color, CHAT_ACCENT, near));
        place += 1.0;
    }
}

/// The time a note's icon moves by: now, and since when its tool call is in the state it is in.
#[derive(Clone, Copy)]
struct Clock {
    now: f64,
    since: f64,
}

impl Clock {
    /// For an icon whose state is not news: it shows as it ends up.
    fn settled(now: f64) -> Self {
        Clock { now, since: f64::NEG_INFINITY }
    }

    /// How far the icon has settled into its state: 0 just changed, 1 at rest.
    #[expect(clippy::cast_possible_truncation, reason = "a share between 0 and 1")]
    fn settling(self) -> f32 {
        ((self.now - self.since) / f64::from(CHAT_ICON_SETTLE)).clamp(0.0, 1.0) as f32
    }
}

/// How large the icon is `share` of the way from at work to done: it grows from the size `from`, a little
/// past its full size, and settles there.
fn pop(share: f32, from: f32) -> f32 {
    // The usual "back" ease-out: it overshoots by about a tenth.
    let (back, t) = (1.70158_f32, share - 1.0);
    let eased = 1.0 + (back + 1.0) * t.powi(3) + back * t.powi(2);
    from + (1.0 - from) * eased
}

/// How far to the side the icon of a call that failed is `share` of the way through its shake: three quick
/// swings, each smaller than the one before.
fn shake(share: f32) -> f32 {
    (share * std::f32::consts::TAU * 3.0).sin() * CHAT_ICON_SHAKE * (1.0 - share)
}

/// An arc around `centre` that ends at the angle `head` and fades out behind it, like a comet's tail.
pub fn comet(painter: &egui::Painter, centre: egui::Pos2, radius: f32, head: f32, color: Color32) {
    const PIECES: u8 = 18;
    let at = |piece: u8| {
        let angle = head - CHAT_ICON_TAIL * (1.0 - f32::from(piece) / f32::from(PIECES));
        centre + radius * vec2(angle.cos(), angle.sin())
    };
    for piece in 0..PIECES {
        let strength = (f32::from(piece) + 1.0) / f32::from(PIECES);
        painter.line_segment([at(piece), at(piece + 1)], egui::Stroke::new(CHAT_ICON_RING, color.gamma_multiply(strength * strength)));
    }
}

/// A note's icon in `slot`, moving with what its tool call is doing.
// Human: at work, the tool's icon is small and breathes while a comet circles it. Done, the circle closes,
// widens and fades as the icon pops up to its full size and goes quiet. Failed, a red cross takes its place and shakes its
// head while a red circle fades. Without a `clock` ("Animations & effects" is off) each state is a still
// picture: a faint circle around the small icon, the icon, the cross.
#[expect(clippy::cast_possible_truncation, reason = "angles and waves")]
fn note_icon(ui: &Ui, slot: egui::Rect, entry: &Entry, palette: Palette, clock: Option<Clock>) {
    let painter = ui.painter();
    let Some(step) = &entry.step else {
        icons::paint(painter, slot, steps::PLAIN_ICON, palette.muted);
        return;
    };
    let centre = slot.center();
    let radius = slot.width() / 2.0 - CHAT_ICON_RING / 2.0;
    let sized = |scale: f32, aside: f32| egui::Rect::from_center_size(centre + vec2(aside, 0.0), slot.size() * scale);
    let ring = |grown: f32, color: Color32| painter.circle_stroke(centre, radius + grown, egui::Stroke::new(CHAT_ICON_RING, color));
    let settling = clock.map_or(1.0, Clock::settling);
    match step.state {
        State::Running => {
            let Some(clock) = clock else {
                ring(0.0, CHAT_ACCENT.gamma_multiply(0.35));
                icons::paint(painter, sized(CHAT_ICON_AT_WORK, 0.0), steps::icon(&step.tool), CHAT_ACCENT);
                return;
            };
            // Human: reading, looking and changing have moving pictures of their own (a book, an eye, a pencil).
            if let Some(act) = steps::act(&step.tool) {
                crate::tool_icons::paint(painter, slot, act, clock.now, CHAT_ACCENT);
                return;
            }
            let breath = (clock.now * f64::from(CHAT_ICON_BREATH)).sin() as f32;
            let head = (clock.now * std::f64::consts::TAU / f64::from(CHAT_ICON_TURN)).rem_euclid(std::f64::consts::TAU) as f32;
            ring(0.0, CHAT_ACCENT.gamma_multiply(0.14));
            comet(painter, centre, radius, head, CHAT_ACCENT);
            icons::paint(
                painter,
                sized(CHAT_ICON_AT_WORK + 0.04 * breath, 0.0),
                steps::icon(&step.tool),
                CHAT_ACCENT.gamma_multiply(0.8 + 0.2 * breath),
            );
        }
        State::Done => {
            if settling < 1.0 {
                ring(CHAT_ICON_BURST * settling, CHAT_ACCENT.gamma_multiply((1.0 - settling).powi(2)));
            }
            let from = if clock.is_some() && steps::act(&step.tool).is_some() { CHAT_ICON_FROM_OWN } else { CHAT_ICON_AT_WORK };
            // Human: the icon of a finished call goes quiet once it has popped; what is at work keeps the colour.
            icons::paint(painter, sized(pop(settling, from), 0.0), steps::icon(&step.tool), tint(CHAT_ACCENT, palette.muted, settling));
        }
        State::Failed => {
            if settling < 1.0 {
                ring(CHAT_ICON_BURST * settling, PROBLEM.gamma_multiply((1.0 - settling).powi(2)));
            }
            icons::paint(painter, sized(pop(settling, CHAT_ICON_AT_WORK), shake(settling)), steps::FAILED_ICON, PROBLEM);
        }
    }
}

/// How a note is drawn.
#[derive(Clone, Copy)]
struct Row {
    /// How much larger than small the text is.
    scale: f32,
    palette: Palette,
    /// The time the note moves by; `None` when nothing moves.
    clock: Option<Clock>,
    /// How many seconds the tool call took, when that is known.
    took: Option<f32>,
    /// How far the note has slid in under the one before it: 0 not there yet, 1 in place.
    grown: f32,
    /// A note stands right before it: a short line links their icons.
    linked: bool,
}

/// How high a note is with the room over it, which it has when another note stands before it.
fn note_height(linked: bool, scale: f32) -> f32 {
    CHAT_NOTE_ROW * scale + if linked { CHAT_NOTE_GAP } else { 0.0 }
}

/// A note on what the assistant did: its icon, its words, and at the right how long the call took.
// Human: a light runs through the words while the tool works. When the call is done or has failed, the new
// words come up into place and the time fades in beside them.
fn note(ui: &mut Ui, entry: &Entry, row: Row) {
    let Row { scale, palette, clock, took, grown, linked } = row;
    let (high, gap) = (CHAT_NOTE_ROW * scale, if linked { CHAT_NOTE_GAP } else { 0.0 });
    let (place, hovered) = ui.allocate_exact_size(vec2(ui.available_width(), note_height(linked, scale) * grown), Sense::hover());
    if grown <= 0.0 {
        return;
    }
    // Human: the line has room for the start of the reason a call failed; the pointer over it shows all of
    // it, and what the assistant wanted to do.
    if let Some(step) = entry.step.as_ref().filter(|step| !step.why.is_empty()) {
        let wanted = if step.detail.is_empty() { String::new() } else { format!("\n\nIt wanted to: {}", step.detail) };
        hovered.on_hover_text(format!("{}{wanted}", step.why));
    }
    let line = Rect::from_min_size(place.min + vec2(0.0, gap * grown + (1.0 - grown) * CHAT_RISE), vec2(place.width(), high));
    let slot = Rect::from_center_size(pos2(line.left() + CHAT_NOTE_ICON / 2.0, line.center().y), Vec2::splat(CHAT_NOTE_ICON));
    ui.scope(|ui| {
        ui.multiply_opacity(grown);
        if linked && grown >= 1.0 {
            let before = slot.top() - gap - (high - CHAT_NOTE_ICON);
            ui.painter().vline(slot.center().x, (before + 3.0)..=(slot.top() - 3.0), Stroke::new(1.0, palette.outline));
        }
        if grown < 1.0 {
            ui.set_clip_rect(place.expand2(vec2(CHAT_ICON_BURST + 1.0, 0.0)).intersect(ui.clip_rect()));
        }
        note_icon(ui, slot, entry, palette, clock);
        // What has just changed comes in; a tool at work simply shows.
        let settled = if at_work(entry) { 1.0 } else { clock.map_or(1.0, Clock::settling) };
        let mut right = line.right();
        if let Some(took) = took.filter(|took| *took >= CHAT_NOTE_TIMED && !at_work(entry)) {
            let ink = palette.quiet.gamma_multiply(settled);
            let time = ui.painter().layout_no_wrap(format!("{took:.1} s"), FontId::proportional((SIDEBAR_TEXT_SIZE - 2.0) * scale), ink);
            right -= time.size().x;
            ui.painter().galley(pos2(right, line.center().y - time.size().y / 2.0), time, ink);
            right -= 8.0;
        }
        let from = slot.right() + LEAD_GAP;
        let light = clock.filter(|_| at_work(entry)).map(|clock| turned(clock.now, CHAT_SHIMMER_TURN));
        let mut words = note_words(entry, scale, palette, light);
        words.wrap = TextWrapping::truncate_at_width((right - from).max(0.0));
        let words = ui.fonts_mut(|fonts| fonts.layout_job(words));
        let at = pos2(from, line.center().y - words.size().y / 2.0 + (1.0 - settled) * 4.0);
        ui.multiply_opacity(0.25 + 0.75 * settled);
        ui.painter().galley(at, words, palette.muted);
    });
}

/// What the chat is doing, for the one line that stands over the message bar while the sidebar is folded away.
#[derive(Debug, PartialEq, Eq)]
enum Status<'a> {
    /// A tool is at work: its note.
    Tool(&'a Entry),
    /// The model is thinking, or writing its answer: what the wait is for.
    Waiting(&'a str),
    /// The answer is there to read.
    Answered,
    /// The run ended with this problem.
    Problem(&'a str),
    /// The assistant asks the user something, or offers next steps: not a line but a dialog.
    Asking(&'a Asked),
}

/// What that line says: `waiting` is what a run waits for while a message is being answered ("Thinking…"),
/// and for a while after the run the line stays (`fresh`).
// Human: a question of the assistant's comes before everything else and stays until it is answered or closed:
// the run that asked it can't go on without the user.
fn status<'a>(entries: &'a [Entry], waiting: Option<&'a str>, fresh: bool) -> Option<Status<'a>> {
    let last = entries.last()?;
    if let Some((_, asked)) = crate::chat::asking(entries) {
        return Some(Status::Asking(asked));
    }
    if let Some(waiting) = waiting {
        return Some(entries.iter().rev().find(|entry| at_work(entry)).map_or(Status::Waiting(waiting), Status::Tool));
    }
    match last.role {
        _ if !fresh => None,
        Role::Problem => Some(Status::Problem(&last.text)),
        Role::Assistant => Some(Status::Answered),
        Role::User | Role::Step => None,
    }
}

/// Where and how the chat's one-line status stands.
#[derive(Clone, Copy)]
pub struct Line {
    /// The bottom middle of the line.
    pub foot: egui::Pos2,
    /// The most room it may take.
    pub most_width: f32,
    /// How wide it is as a dialog, when the assistant asks something.
    pub ask_width: f32,
    /// The run ended only a moment ago: the line still says that the answer is there.
    pub fresh: bool,
    /// How much of it shows; it fades with the bar it stands over.
    pub opacity: f32,
}

/// What the status line shows, kept between frames so it can still be drawn while it gives way to the next.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Face {
    /// A tool at work: its note (boxed: a line of the conversation is far larger than the other faces).
    Tool(Box<Entry>),
    /// The dots and a few words beside them.
    Waiting(String),
    /// The answer is there to read.
    Answered,
    /// The run ended with this problem.
    Problem(String),
    /// A question of the assistant's, or its suggestions, as far as the user has got with it.
    Asking(Asked),
}

impl From<Status<'_>> for Face {
    fn from(status: Status<'_>) -> Self {
        match status {
            Status::Tool(entry) => Face::Tool(Box::new(entry.clone())),
            Status::Waiting(label) => Face::Waiting(label.to_owned()),
            Status::Answered => Face::Answered,
            Status::Problem(text) => Face::Problem(text.to_owned()),
            Status::Asking(asked) => Face::Asking(asked.clone()),
        }
    }
}

/// What the status line remembers between frames to turn from one thing into the next; it lives in egui's
/// memory.
#[derive(Clone, Debug, PartialEq)]
struct Morph {
    /// What the line shows.
    face: Face,
    /// Since when, in egui's time.
    since: f64,
    /// What it showed before, and how large the island was when that gave way; `None` when the line was not
    /// there before.
    from: Option<(Face, Vec2)>,
    /// How large the island was drawn last.
    size: Vec2,
}

impl Morph {
    /// The line at rest showing `face`: nothing moves ("Animations & effects" is off).
    fn still(face: Face) -> Self {
        Morph { face, since: f64::NEG_INFINITY, from: None, size: Vec2::ZERO }
    }

    /// What the line shows at `now`, given what it `kept` from the frame before and what the chat `wanted`.
    // Human: the app's tools are done in no time, so the note of one would give way to "Thinking…" before it
    // could be read. A tool's note stays for a moment, unless the next tool or the end of the run takes over.
    fn next(kept: Option<Morph>, wanted: Face, now: f64) -> Self {
        match kept {
            Some(kept) if kept.face == wanted => kept,
            // Human: ticking an answer or writing one's own changes what the dialog shows, not what it is: it
            // stays as it stands, without turning into itself with every letter.
            Some(kept) if same_question(&kept.face, &wanted) => Morph { face: wanted, ..kept },
            Some(kept) if matches!((&kept.face, &wanted), (Face::Tool(_), Face::Waiting(_))) && now - kept.since < f64::from(CHAT_STATUS_HOLD) => {
                kept
            }
            Some(kept) => Morph { from: Some((kept.face, kept.size)), face: wanted, since: now, size: kept.size },
            None => Morph { face: wanted, since: now, from: None, size: Vec2::ZERO },
        }
    }

    /// How far the turn has come at `now`: 0 just begun, 1 over.
    #[expect(clippy::cast_possible_truncation, reason = "a share between 0 and 1")]
    fn share(&self, now: f64) -> f32 {
        ((now - self.since) / f64::from(CHAT_STATUS_MORPH)).clamp(0.0, 1.0) as f32
    }
}

/// Whether `one` and `other` show the same question of the assistant's at the same point: only what is ticked
/// or written for it differs.
fn same_question(one: &Face, other: &Face) -> bool {
    match (one, other) {
        (Face::Asking(one), Face::Asking(other)) => {
            one.questions == other.questions && one.answers == other.answers && one.closed == other.closed && one.suggests == other.suggests
        }
        _ => false,
    }
}

/// `share` eased: slow away, fast through the middle, slow into place.
fn eased(share: f32) -> f32 {
    if share < 0.5 { 4.0 * share.powi(3) } else { 1.0 - (2.0 - 2.0 * share).powi(3) / 2.0 }
}

/// How much of what the line showed before, and of what it shows now, is seen `share` of the way through a
/// turn: the old goes first, the new comes as the island nears its new width.
fn crossfade(share: f32) -> (f32, f32) {
    ((1.0 - share / 0.45).clamp(0.0, 1.0), ((share - 0.35) / 0.65).clamp(0.0, 1.0))
}

/// The room the dots take.
const DOTS_WIDTH: f32 = 3.0 * CHAT_DOT + 2.0 * CHAT_DOT_GAP;
/// The room between an icon, or the dots, and the words beside it.
const LEAD_GAP: f32 = 7.0;

/// The words of `face`.
fn face_words(face: &Face, palette: Palette) -> LayoutJob {
    let quiet = |text: &str, color: Color32| LayoutJob::simple_singleline(text.to_owned(), FontId::proportional(SIDEBAR_TEXT_SIZE - 1.0), color);
    match face {
        Face::Tool(entry) => note_words(entry, 1.0, palette, None),
        Face::Waiting(label) => quiet(label, palette.muted),
        Face::Answered => quiet("Done  ·  click to read the reply", palette.muted),
        Face::Problem(text) => quiet(text, PROBLEM),
        // A question is not a line: it is laid out as a sheet ([`crate::asking`]).
        Face::Asking(_) => LayoutJob::default(),
    }
}

/// A face laid out for an island at most `most_width` wide: the room its icon (or the dots) takes, and its
/// words, cut short when they are too many.
fn laid_out(ctx: &egui::Context, face: &Face, most_width: f32, palette: Palette) -> (f32, std::sync::Arc<egui::Galley>) {
    let lead = if matches!(face, Face::Waiting(_)) { DOTS_WIDTH } else { CHAT_NOTE_ICON };
    let mut words = face_words(face, palette);
    words.wrap = egui::text::TextWrapping::truncate_at_width((most_width - 2.0 * CHAT_STATUS_PAD - lead - LEAD_GAP).max(0.0));
    (lead, ctx.fonts_mut(|fonts| fonts.layout_job(words)))
}

/// How a question is laid out as the dialog over the message bar: only what is still to answer.
fn dialog(palette: Palette) -> crate::asking::Style {
    crate::asking::Style { scale: 1.0, history: false, palette }
}

/// How large the island is when it shows `face` and nothing else.
fn face_size(ctx: &egui::Context, face: &Face, line: &Line, palette: Palette) -> Vec2 {
    if let Face::Asking(asked) = face {
        return crate::asking::sheet(ctx, asked, line.ask_width, dialog(palette)).size;
    }
    let (lead, words) = laid_out(ctx, face, line.most_width, palette);
    vec2((2.0 * CHAT_STATUS_PAD + lead + LEAD_GAP + words.size().x).min(line.most_width), CHAT_STATUS_HEIGHT)
}

/// Where and how a face of the status line is painted.
#[derive(Clone, Copy)]
struct Spot {
    /// The island as it is drawn now.
    island: egui::Rect,
    /// The widest the island gets as a line, and its width as a dialog.
    widths: (f32, f32),
    /// How much of the face shows.
    opacity: f32,
    /// egui's time, by which icons and dots move; `None` when nothing moves.
    time: Option<f64>,
    /// The face is what the island shows now, not what it showed before: its buttons can be clicked.
    live: bool,
}

/// Paints `face` in the island at `place`: a line in the middle of its bottom strip, a question as its whole
/// sheet standing on the bottom edge. Returns what the user did with a question.
// Human: the island grows upwards from the line it was, so a line stays where it stood while it fades, and a
// question's sheet is laid out for the finished island and shows through the growing one.
fn paint_face(ui: &mut Ui, face: &Face, place: Spot, palette: Palette) -> Option<Pick> {
    let Spot { island, widths: (most_width, ask_width), opacity, time, live } = place;
    if opacity <= 0.0 {
        return None;
    }
    if let Face::Asking(asked) = face {
        let sheet = crate::asking::sheet(ui.ctx(), asked, ask_width, dialog(palette));
        let origin = pos2(island.center().x - sheet.size.x / 2.0, island.bottom() - sheet.size.y);
        return ui
            .scope(|ui| {
                ui.multiply_opacity(opacity);
                ui.set_clip_rect(island.shrink(1.0).intersect(ui.clip_rect()));
                crate::asking::paint(ui, &sheet, origin, live, palette)
            })
            .inner;
    }
    let strip = egui::Rect::from_min_max(pos2(island.left(), island.bottom() - CHAT_STATUS_HEIGHT), island.max);
    let (lead, words) = laid_out(ui.ctx(), face, most_width, palette);
    let left = strip.center().x - (lead + LEAD_GAP + words.size().x) / 2.0;
    let slot = egui::Rect::from_min_size(pos2(left, strip.center().y - CHAT_NOTE_ICON / 2.0), vec2(lead, CHAT_NOTE_ICON));
    ui.scope(|ui| {
        ui.multiply_opacity(opacity);
        // Human: while the island is narrower than the words, they show through it as through a window.
        ui.set_clip_rect(island.shrink(1.0).intersect(ui.clip_rect()));
        match face {
            Face::Tool(entry) => note_icon(ui, slot, entry, palette, time.map(Clock::settled)),
            Face::Waiting(_) => dots(ui, slot, time),
            Face::Answered => icons::paint(ui.painter(), slot, steps::ANSWERED_ICON, CHAT_ACCENT),
            Face::Problem(_) => icons::paint(ui.painter(), slot, steps::FAILED_ICON, PROBLEM),
            Face::Asking(_) => {}
        }
        let at = pos2(slot.right() + LEAD_GAP, strip.center().y - words.size().y / 2.0);
        ui.painter().galley(at, words, palette.muted);
    });
    None
}

/// What the user did with the island over the message bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tap {
    /// Clicked it: the sidebar opens the chat.
    Open,
    /// Answered or closed the question in it.
    Pick(Pick),
}

fn status_id() -> egui::Id {
    egui::Id::unique("chat-status-morph")
}

/// Draws the chat's one-line status as a small island: what the assistant is doing right now, or that its
/// answer is there; and when the assistant asks the user something, the same island grown into a dialog with
/// the question and its answers. Returns what the user did with it: a click on it opens the chat.
// Human: with the sidebar folded away the conversation can't be seen, so without this line a message sent
// from the bar would get no sign of life until the design changes. It is one island all through a run: when
// what it says changes, it grows or shrinks to the new words while the old ones fade and the new ones come.
// A question is no other thing: the line's island grows upwards into the dialog, and shrinks back into a line
// when the question is answered.
// Agent: MAIN-THREAD; a floating layer over the canvas; asks for the next frame while the assistant works or
// the island turns.
pub fn status_line(ctx: &egui::Context, line: &Line, chat: &Chat, palette: Palette, animate: bool) -> Option<Tap> {
    let Line { foot, most_width, ask_width, fresh, opacity } = *line;
    // Human: the line says what the wait is for, as the panel does; the seconds and the thought are the panel's.
    let waiting = chat.pause().map(|pause| format!("{}…", pause.words(chat.answering().unwrap_or_default())));
    let Some(status) = status(&chat.entries, waiting.as_deref(), fresh) else {
        ctx.data_mut(|data| data.remove::<Morph>(status_id()));
        return None;
    };
    let now = ctx.input(|input| input.time);
    let mut morph = if animate { Morph::next(ctx.data(|data| data.get_temp(status_id())), status.into(), now) } else { Morph::still(status.into()) };
    let share = morph.share(now);
    let full = face_size(ctx, &morph.face, line, palette);
    // Human: a line that was not there before simply fades in at its size.
    let (size, appeared) = match &morph.from {
        Some((_, from)) => (*from + (full - *from) * eased(share), 1.0),
        None => (full, share),
    };
    morph.size = size;
    let place = egui::Rect::from_min_size(pos2(foot.x - size.x / 2.0, foot.y - size.y), size);
    let widths = (most_width, ask_width);
    let time = animate.then_some(now);
    let shown = egui::Area::new(egui::Id::unique("chat-status")).fade_in(false).fixed_pos(place.min).order(egui::Order::Foreground).show(ctx, |ui| {
        ui.multiply_opacity(opacity * appeared);
        let (island, response) = ui.allocate_exact_size(place.size(), Sense::click());
        // Human: a line is a pill; grown into a dialog the island has the corners of the other islands.
        crate::sidebar::island(ui, island, ISLAND_RADIUS.min(island.height() / 2.0), palette);
        let (old, new) = if morph.from.is_some() { crossfade(share) } else { (0.0, 1.0) };
        if let Some((before, _)) = &morph.from {
            paint_face(ui, before, Spot { island, widths, opacity: old, time, live: false }, palette);
        }
        let pick = paint_face(ui, &morph.face, Spot { island, widths, opacity: new, time, live: true }, palette);
        (response, pick)
    });
    // Human: the pill's morph runs at full rate; its light while the assistant works at the idle rate.
    if animate && share < 1.0 {
        ctx.request_repaint();
    } else if animate && chat.running() {
        ctx.request_repaint_after(crate::theme::IDLE_FRAME);
    }
    ctx.data_mut(|data| data.insert_temp(status_id(), morph));
    let (response, pick) = shown.inner;
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    pick.map(Tap::Pick).or_else(|| response.clicked().then_some(Tap::Open))
}

/// Three dots that rise one after the other in `slot`; without a `time` they rest.
#[expect(clippy::cast_possible_truncation, reason = "a wave between -1 and 1")]
fn dots(ui: &Ui, slot: egui::Rect, time: Option<f64>) {
    for place in [0.0_f32, 1.0, 2.0] {
        // Each dot is a third of a beat behind the one before it; below the wave's middle it rests.
        let lift = time.map_or(0.4, |now| ((now * f64::from(CHAT_DOT_PACE) - f64::from(place) * 0.9).sin() as f32).max(0.0));
        let at = pos2(slot.left() + CHAT_DOT / 2.0 + place * (CHAT_DOT + CHAT_DOT_GAP), slot.center().y + 1.0 - lift * CHAT_DOT_LIFT);
        ui.painter().circle_filled(at, CHAT_DOT / 2.0, CHAT_ACCENT.gamma_multiply(0.35 + 0.65 * lift));
    }
}

/// A wait of the model's as the panel tells it.
struct Wait {
    /// What the wait is for, without the dots after it.
    words: String,
    /// How long it has lasted.
    lasted: std::time::Duration,
    /// What the model has thought in this wait, as one flow of words; empty when none can be read.
    thought: String,
}

/// `text` without the white space and the Markdown marks around it.
fn tidy(text: &str) -> &str {
    text.trim_matches(|letter: char| letter.is_whitespace() || matches!(letter, '*' | '#' | '`' | '-' | '_'))
}

/// All the model has `thought` since it last did something, as one flow of words: its lines joined, without
/// the Markdown marks around them, and with one space wherever there was white space.
fn flow(thought: &str) -> String {
    thought.lines().map(tidy).flat_map(str::split_whitespace).collect::<Vec<_>>().join(" ")
}

/// How far the thought's lines are moved up so that its last ones show, given how far they `were` moved the
/// frame before, how far they `should` be now and the time `step` since that frame; `None` when nothing glides.
// Human: a new line of the thought doesn't snap into place, it glides up. A thought that starts over (the
// model thinks anew) is simply there.
fn glide(were: f32, should: f32, step: Option<f32>) -> f32 {
    match step {
        Some(step) if should > were && should - were > 0.5 => were + (should - were) * (1.0 - (-CHAT_THOUGHT_GLIDE * step).exp()),
        _ => should,
    }
}

/// The dots that say the model is at it, and what the `wait` is for beside them: its words, how long it has
/// lasted once that is worth saying, and under them the last lines of what the model thinks. Without a `wait`
/// the dots stand alone; without a `time` they rest and nothing glides.
fn waiting(ui: &mut Ui, wait: Option<&Wait>, scale: f32, palette: Palette, time: Option<f64>) {
    let font = FontId::proportional((SIDEBAR_TEXT_SIZE - 1.0) * scale);
    let quiet = palette.quiet;
    // Human: while things move, the wait is the drop of the task list, and a light runs through its words;
    // at rest it is three dots.
    let lead = if time.is_some() { CHAT_NOTE_ICON } else { DOTS_WIDTH };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = LEAD_GAP;
        let (slot, _) = ui.allocate_exact_size(vec2(lead, CHAT_NOTE_ICON), Sense::hover());
        match time {
            Some(now) => blob(ui.painter(), slot.center(), CHAT_CARET + 0.5, Some(now)),
            None => dots(ui, slot, None),
        }
        let Some(wait) = wait else { return };
        let mut text = LayoutJob::default();
        let mut say = |words: &str, color: Color32| text.append(words, 0.0, TextFormat { font_id: font.clone(), color, ..TextFormat::default() });
        let light = time.map(|now| turned(now, CHAT_SHIMMER_TURN));
        lit(&format!("{}…", wait.words), palette.foreground.gamma_multiply(0.85), light, &mut say);
        if wait.lasted.as_secs() >= CHAT_WAIT_COUNTED {
            say(&format!("  ·  {}", crate::chat::span(wait.lasted)), quiet);
        }
        ui.add(egui::Label::new(text).truncate().selectable(false));
    });
    let Some(wait) = wait else { return };
    // Human: the thought stands under the words, not under the dots, in a place that is always two lines high
    // and as wide as the panel, whether the model has thought a word, a page or nothing yet: nothing under or
    // over it moves while the thought streams in. Its last two lines show; when a third begins, the lines
    // glide up by one. It is a glimpse of what the model is at, not something to read through.
    let indent = lead + LEAD_GAP;
    let width = (ui.available_width() - indent).max(0.0);
    let text = if wait.thought.is_empty() { " " } else { wait.thought.as_str() };
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(LayoutJob::simple(text.to_owned(), font, quiet, width)));
    let rows = galley.rows.len().max(1);
    #[expect(clippy::cast_precision_loss, reason = "a handful of lines of text")]
    let (row, shown) = (galley.size().y / rows as f32, CHAT_THOUGHT_ROWS as f32);
    #[expect(clippy::cast_precision_loss, reason = "a handful of lines of text")]
    let should = rows.saturating_sub(CHAT_THOUGHT_ROWS) as f32 * row;
    let id = egui::Id::unique("chat-thought-glide");
    let were: f32 = ui.data(|data| data.get_temp(id)).unwrap_or(should);
    let moved = glide(were, should, time.map(|_| ui.input(|input| input.stable_dt).min(0.1)));
    ui.data_mut(|data| data.insert_temp(id, moved));
    ui.add_space(3.0);
    let (place, _) = ui.allocate_exact_size(vec2(indent + width, row * shown), Sense::hover());
    let window = egui::Rect::from_min_max(place.min + vec2(indent, 0.0), place.max);
    ui.painter().with_clip_rect(window.intersect(ui.clip_rect())).galley(window.min - vec2(0.0, moved), galley, quiet);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_comes_in_fast_then_settles() {
        assert!((arrival(10.0, 10.0, CHAT_APPEAR)).abs() < f32::EPSILON);
        assert!((arrival(10.0 + f64::from(CHAT_APPEAR), 10.0, CHAT_APPEAR) - 1.0).abs() < f32::EPSILON);
        assert!((arrival(10.0, f64::NEG_INFINITY, CHAT_APPEAR) - 1.0).abs() < f32::EPSILON);
        let half = arrival(10.0 + f64::from(CHAT_APPEAR) / 2.0, 10.0, CHAT_APPEAR);
        assert!(half > 0.5 && half < 1.0, "{half}");
    }

    #[test]
    fn the_assistants_words_flow_in_and_catch_up() {
        let entry = Entry::new(Role::Assistant, "Grüße — a card with a soft shadow.");
        let mut motion = Motion::default();
        // Being written, the line starts empty and grows frame by frame, never cutting a letter in two.
        let (first, done) = written(&mut motion, 3, &entry, true, 0.016);
        assert!(!done && first.len() < entry.text.len() && entry.text.starts_with(first));
        let mut frames = 0;
        while !written(&mut motion, 3, &entry, true, 0.016).1 {
            frames += 1;
            assert!(frames < 200, "the text never arrived");
        }
        assert_eq!(written(&mut motion, 3, &entry, false, 0.016), (entry.text.as_str(), true));
        // A finished line seen for the first time is simply there.
        assert_eq!(written(&mut Motion::default(), 0, &entry, false, 0.016), (entry.text.as_str(), true));
    }

    /// How tall `line` draws `entry` with `shown` of its words, as the last line of a run that is over.
    fn height_of(entry: &Entry, shown: &str) -> f32 {
        let ctx = egui::Context::default();
        let total = Spent::default();
        let mut height = 0.0;
        let mut drawn = ctx.run_ui(egui::RawInput::default(), |ui| {
            let looks = Looks { scale: 1.0, palette: crate::theme::palette(false), live: false, at: 0, total: &total, caret: None, closing: true };
            height = ui.scope(|ui| line(ui, entry, shown, looks)).response.rect.height();
        });
        drawn.textures_delta.clear();
        height
    }

    #[test]
    fn what_an_answer_took_stands_under_it_once_its_words_are_out() {
        let answer = Entry::new(Role::Assistant, "A dog.");
        // While the words are still coming nothing stands under them.
        let written = height_of(&answer, &answer.text);
        assert!(written > height_of(&answer, "A do"));
        // An answer that was counted has its sum on that line, which is as high with it as without.
        let mut priced = answer.clone();
        priced.spent = Some(Spent { used: crate::chat::usage::Used { input: 10, output: 5, ..Default::default() }, ..Spent::default() });
        assert!((height_of(&priced, &priced.text) - written).abs() < 4.0);
    }

    #[test]
    fn notes_that_follow_each_other_are_one_stretch_of_work() {
        let note = |tool: &str| {
            let step = steps::Step {
                tool: tool.to_owned(),
                id: String::new(),
                detail: String::new(),
                state: State::Done,
                times: 1,
                named: false,
                why: String::new(),
            };
            Entry { step: Some(step), ..Entry::new(Role::Step, steps::did(tool)) }
        };
        let mut list = Entry::new(Role::Step, "Tasks completed: 2 of 2");
        list.finished = vec!["One".to_owned(), "Two".to_owned()];
        let entries = [
            Entry::new(Role::User, "A card"),
            note("read_layers"),
            note("edit_layers"),
            Entry::new(Role::Assistant, "Done."),
            note("export_layers"),
            // A task list that was worked through is not a note on a tool call: it ends the stretch.
            list,
            note("read_layers"),
        ];
        assert_eq!(stretches(&entries), [1..3, 4..5, 6..7]);
        assert_eq!(stretches(&entries[..1]).len(), 0);
        // A question that still waits for its answer is a card, not a note.
        assert!(crate::chat::sample_asking(false).iter().all(|entry| !is_note(entry) || entry.asked.is_none()));
    }

    #[test]
    fn a_tool_call_that_was_watched_says_how_long_it_took() {
        let call = |state: State| {
            let step = steps::Step {
                tool: "export_layers".to_owned(),
                id: String::new(),
                detail: String::new(),
                state,
                times: 1,
                named: false,
                why: String::new(),
            };
            Entry { step: Some(step), ..Entry::new(Role::Step, "Exported layers") }
        };
        let mut motion = Motion::default();
        motion.take_in(&[call(State::Running)], 10.0, true, None);
        assert_eq!(motion.took, [None]);
        motion.take_in(&[call(State::Done)], 12.5, true, None);
        assert_eq!(motion.took, [Some(2.5)]);
        // A second call told by the same note adds to it.
        motion.take_in(&[call(State::Running)], 13.0, true, None);
        motion.take_in(&[call(State::Failed)], 14.0, true, None);
        assert_eq!(motion.took, [Some(3.5)]);
        // A call that was done before the panel showed it has no time to say.
        let mut later = Motion::default();
        later.take_in(&[call(State::Done)], 20.0, true, None);
        // A message that flies into the conversation as a pill shows when the pill is nearly there.
        let mut flown = Motion::default();
        let flying = ("Export it".to_owned(), 31.0);
        flown.take_in(&[Entry::new(Role::User, "A card")], 30.0, true, Some(&flying));
        flown.take_in(&[Entry::new(Role::User, "A card"), Entry::new(Role::User, "Export it")], 30.0, true, Some(&flying));
        assert!(flown.born[1] > 30.0 && flown.born[1] < 31.0);
        assert_eq!(later.took, [None]);
    }

    #[test]
    fn a_folded_stretch_says_how_many_steps_it_was_and_how_long_they_took() {
        assert_eq!(steps_say(1, None), "1 step");
        assert_eq!(steps_say(5, None), "5 steps");
        // Under a second is not worth saying.
        assert_eq!(steps_say(5, Some(0.4)), "5 steps");
        assert_eq!(steps_say(5, Some(8.2)), "5 steps  ·  8 s");
    }

    #[test]
    fn a_light_runs_through_the_words_of_a_tool_at_work() {
        let (plain, accent) = (Color32::from_gray(200), CHAT_ACCENT);
        let colors = |light: Option<f32>| {
            let mut said = Vec::new();
            lit("Reading layers…", plain, light, &mut |words: &str, color| said.push((words.to_owned(), color)));
            said
        };
        // Without a light the words are said in one piece.
        assert_eq!(colors(None), [("Reading layers…".to_owned(), plain)]);
        // With one, every letter is said, and the letters near the light take its colour.
        let half = colors(Some(0.5));
        assert_eq!(half.iter().map(|(words, _)| words.as_str()).collect::<String>(), "Reading layers…");
        assert!(half[7].1 != plain && half[8].1 != plain && accent != plain);
        assert_eq!((half[0].1, half[14].1), (plain, plain));
        // Before it starts and after it has passed, no letter is lit.
        assert!(colors(Some(0.0)).iter().chain(&colors(Some(1.0))).all(|(_, color)| *color == plain));
    }

    #[test]
    fn the_counter_says_what_the_last_answer_took_and_its_hover_the_parts() {
        use crate::chat::usage::{Spent, Used};
        // A conversation in which nothing was counted has no counter, and takes no room.
        let mut chat = Chat::default();
        chat.entries.push(Entry::new(Role::User, "A dog"));
        chat.entries.push(Entry::new(Role::Assistant, "A dog."));
        assert!(counter_words(&chat).is_none());
        // The last answer that was counted is the one shown, also when words came after it.
        let spent = Spent { used: Used { input: 10_200, cached: 2_200, written: 0, output: 1_800 }, cost: Some(80_000_000_000), ..Spent::default() };
        chat.entries[1].spent = Some(spent.clone());
        chat.entries.push(Entry::new(Role::User, "And a cat"));
        chat.total = spent;
        let (words, hover) = counter_words(&chat).unwrap();
        assert_eq!(words, "12.4k in · 1.8k out · $0.08");
        assert!(hover.starts_with("Input: 10.2k\nRead from cache: 2.2k\nOutput: 1.8k"), "{hover}");
    }

    #[test]
    fn an_icon_pops_into_place_and_a_failed_one_shakes_itself_still() {
        // Done: from the small size at work, a little past full size, to full size.
        assert!((pop(0.0, CHAT_ICON_AT_WORK) - CHAT_ICON_AT_WORK).abs() < 1e-4 && (pop(1.0, CHAT_ICON_AT_WORK) - 1.0).abs() < 1e-4);
        let most = (0..=100_u8).map(|step| pop(f32::from(step) / 100.0, CHAT_ICON_AT_WORK)).fold(0.0, f32::max);
        assert!(most > 1.02 && most < 1.2, "{most}");
        // Failed: it starts and ends in its place, and swings both ways in between, less each time.
        assert!(shake(0.0).abs() < 1e-4 && shake(1.0).abs() < 1e-4);
        let swings: Vec<f32> = (0..=100_u8).map(|step| shake(f32::from(step) / 100.0)).collect();
        let (left, right) = (swings.iter().copied().fold(0.0, f32::min), swings.iter().copied().fold(0.0, f32::max));
        assert!(left < -1.0 && right > 1.0 && right <= CHAT_ICON_SHAKE, "{left} {right}");
        assert!(swings[60..].iter().all(|swing| swing.abs() < right * 0.6));
        // A state that is not news shows at rest; one that just changed has all of its piece to play.
        assert!((Clock::settled(5.0).settling() - 1.0).abs() < f32::EPSILON);
        assert!(Clock { now: 5.0, since: 5.0 }.settling().abs() < f32::EPSILON);
    }

    #[test]
    fn a_tool_that_was_done_at_once_is_still_shown_at_work_for_a_moment() {
        let entries = crate::chat::sample();
        let done = &entries[2];
        assert!(held(done, false).is_none() && !at_work(done));
        assert!(held(done, true).is_some_and(|shown| at_work(&shown) && shown.text == done.text));
        // What isn't a finished tool call is never held: words, and a call that failed.
        assert!(held(&entries[0], true).is_none());
        assert!(
            entries
                .iter()
                .filter(|entry| entry.step.as_ref().is_some_and(|step| step.state == State::Failed))
                .all(|entry| held(entry, true).is_none())
        );
    }

    #[test]
    fn the_folded_chat_says_in_one_line_what_is_going_on() {
        let mut entries = vec![Entry::new(Role::User, "Make a dog")];
        assert_eq!(status(&[], Some("Thinking…"), true), None);
        assert_eq!(status(&entries, Some("Thinking…"), false), Some(Status::Waiting("Thinking…")));
        entries.extend(crate::chat::sample().into_iter().filter(|entry| entry.step.is_some()).take(2));
        // A finished tool is no news; one at work is.
        assert_eq!(status(&entries, Some("Thinking…"), false), Some(Status::Waiting("Thinking…")));
        if let Some(step) = entries[2].step.as_mut() {
            step.state = State::Running;
        }
        assert!(matches!(status(&entries, Some("Thinking…"), false), Some(Status::Tool(note)) if note == &entries[2]));
        entries.push(Entry::new(Role::Assistant, "A dog."));
        if let Some(step) = entries[2].step.as_mut() {
            step.state = State::Done;
        }
        // Whatever the wait is for is said as the chat words it.
        assert_eq!(status(&entries, Some("Writing…"), false), Some(Status::Waiting("Writing…")));
        // After the run the line says the answer is there, for a while.
        assert_eq!((status(&entries, None, true), status(&entries, None, false)), (Some(Status::Answered), None));
        entries.push(Entry::new(Role::Problem, "No connection"));
        assert_eq!(status(&entries, None, true), Some(Status::Problem("No connection")));
    }

    #[test]
    fn the_status_line_turns_from_one_thing_into_the_next() {
        let mut note = crate::chat::sample().into_iter().find(|entry| entry.step.is_some()).unwrap();
        if let Some(step) = note.step.as_mut() {
            step.state = State::Running;
        }
        let (thinking, tool) = (Face::Waiting("Thinking…".to_owned()), Face::Tool(Box::new(note)));
        // A line that wasn't there comes from nothing; one that says the same stays as it is.
        let first = Morph::next(None, thinking.clone(), 10.0);
        assert_eq!((first.from.as_ref(), first.since), (None, 10.0));
        let pill = vec2(90.0, CHAT_STATUS_HEIGHT);
        let first = Morph { size: pill, ..first };
        assert_eq!(Morph::next(Some(first.clone()), thinking.clone(), 11.0), first);
        // A tool takes over at once, from the width the island had.
        let at_work = Morph::next(Some(first), tool.clone(), 12.0);
        assert_eq!((&at_work.face, at_work.from.as_ref(), at_work.since), (&tool, Some(&(thinking.clone(), pill)), 12.0));
        // Its note stays for a moment after the tool is done, then gives way; the end of the run doesn't wait.
        assert_eq!(Morph::next(Some(at_work.clone()), thinking.clone(), 12.5), at_work);
        assert_eq!(Morph::next(Some(at_work.clone()), thinking.clone(), 12.0 + f64::from(CHAT_STATUS_HOLD)).face, thinking);
        assert_eq!(Morph::next(Some(at_work.clone()), Face::Answered, 12.5).face, Face::Answered);
        // The turn: the old words are gone before the new ones are all there, and it ends at rest.
        assert!((at_work.share(12.0)).abs() < f32::EPSILON && (at_work.share(12.0 + f64::from(CHAT_STATUS_MORPH)) - 1.0).abs() < f32::EPSILON);
        assert!((Morph::still(thinking).share(0.0) - 1.0).abs() < f32::EPSILON);
        assert_eq!((crossfade(0.0), crossfade(1.0)), ((1.0, 0.0), (0.0, 1.0)));
        assert!((0..=100_u8).map(|step| crossfade(f32::from(step) / 100.0)).all(|(old, new)| old + new <= 1.0));
        assert!(eased(0.0).abs() < 1e-6 && (eased(0.5) - 0.5).abs() < 1e-6 && (eased(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_question_takes_the_status_lines_place_until_it_is_answered() {
        let mut entries = crate::chat::sample_asking(false);
        // It is there while the assistant still writes, and long after the run: it waits for the user.
        assert!(matches!(status(&entries, Some("Thinking…"), false), Some(Status::Asking(asked)) if asked.questions.len() == 2));
        assert!(matches!(status(&entries, None, false), Some(Status::Asking(_))));
        // The island that said "Thinking…" becomes the dialog, from the size it had; the next question, and
        // the line after the last answer, come out of the dialog the same way.
        let pill = vec2(90.0, CHAT_STATUS_HEIGHT);
        let line = Morph { size: pill, ..Morph::next(None, Face::Waiting("Thinking…".to_owned()), 1.0) };
        let first: Face = status(&entries, Some("Thinking…"), false).unwrap().into();
        let asking = Morph::next(Some(line), first.clone(), 2.0);
        assert_eq!((&asking.face, asking.from.as_ref()), (&first, Some(&(Face::Waiting("Thinking…".to_owned()), pill))));
        let grown = vec2(440.0, 190.0);
        if let Some(asked) = entries[3].asked.as_mut() {
            asked.answer(0, "Dark and technical");
        }
        let second: Face = status(&entries, None, false).unwrap().into();
        let next = Morph::next(Some(Morph { size: grown, ..asking }), second.clone(), 3.0);
        assert_eq!(next.from, Some((first, grown)));
        if let Some(asked) = entries[3].asked.as_mut() {
            asked.answer(1, "Phone");
        }
        assert_eq!(status(&entries, None, false), None);
        entries.push(Entry::new(Role::User, "Dark and technical\nPhone"));
        let after: Face = status(&entries, Some("Thinking…"), false).unwrap().into();
        assert_eq!(Morph::next(Some(Morph { size: grown, ..next }), after, 4.0).from, Some((second, grown)));
    }

    #[test]
    fn a_thought_is_one_flow_of_words_whose_new_lines_glide_into_view() {
        assert_eq!(flow(""), "");
        assert_eq!(
            flow("**Planning the page**\n\nA dark   page first.\n- Grüße an alle, die bauen!"),
            "Planning the page A dark page first. Grüße an alle, die bauen!"
        );
        // A new line comes into view over a few frames, never past its place, and ends exactly there.
        let mut moved = 0.0;
        for _ in 0..120 {
            let next = glide(moved, 16.0, Some(0.016));
            assert!(next >= moved && next <= 16.0, "{next}");
            moved = next;
        }
        assert!((moved - 16.0).abs() < f32::EPSILON);
        assert!(glide(0.0, 16.0, Some(0.016)) < 8.0);
        // With nothing moving it is there at once, and so is a thought that starts over.
        assert!((glide(0.0, 16.0, None) - 16.0).abs() < f32::EPSILON && glide(32.0, 0.0, Some(0.016)).abs() < f32::EPSILON);
    }

    #[test]
    fn only_a_tool_at_work_hides_the_thinking_dots() {
        let mut entries = crate::chat::sample();
        assert!(!entries.iter().any(at_work));
        if let Some(step) = entries[2].step.as_mut() {
            step.state = State::Running;
        }
        assert!(at_work(&entries[2]) && !at_work(&entries[0]));
    }
}
