//! The conversation in the chat panel: the user's messages, the assistant's words, and a note for each thing
//! it does, drawn with a little motion so what is new can be followed.
//!
//! A new line fades in and settles into place; the assistant's words flow in at an even pace however unevenly
//! they arrive; a tool at work turns a spinner beside what it is doing; three dots wait while the model thinks,
//! beside what the wait is for, how long it has lasted, and the model's latest thought where it can be read.
//! With "Animations & effects" off in the settings everything simply appears.

use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, FontId, Layout, RichText, Sense, Ui, Vec2, pos2, vec2};

pub use crate::asking::Pick;
use crate::chat::asks::Asked;
use crate::chat::steps::{self, State};
use crate::chat::{Chat, Entry, Pause, Role};
use crate::icons;
use crate::theme::{
    self, CHAT_APPEAR, CHAT_BUBBLE_SHARE, CHAT_COPIED_STAYS, CHAT_DOT, CHAT_DOT_GAP, CHAT_DOT_LIFT, CHAT_DOT_PACE, CHAT_GAP, CHAT_ICON_AT_WORK,
    CHAT_ICON_BREATH, CHAT_ICON_BURST, CHAT_ICON_FROM_OWN, CHAT_ICON_LEAST, CHAT_ICON_RING, CHAT_ICON_SETTLE, CHAT_ICON_SHAKE, CHAT_ICON_TAIL,
    CHAT_ICON_TURN, CHAT_NOTE_GAP, CHAT_NOTE_ICON, CHAT_RISE, CHAT_STATUS_HEIGHT, CHAT_STATUS_HOLD, CHAT_STATUS_MORPH, CHAT_STATUS_PAD,
    CHAT_THOUGHT_GLIDE, CHAT_THOUGHT_ROWS, CHAT_WAIT_COUNTED, CHAT_WRITE_CATCH_UP, CHAT_WRITE_PACE, CHIP_CORNER, CHIP_ICON, CHIP_ICON_AT,
    ISLAND_RADIUS, MESSAGE_TEXT_SIZE, PROBLEM, Palette, SIDEBAR_TEXT_SIZE,
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
    // Human: a conversation that is already there when the panel first shows it doesn't move in; only what
    // is added while one watches does.
    let known = motion.born.is_empty() && chat.entries.len() > 1;
    motion.born.truncate(chat.entries.len());
    while motion.born.len() < chat.entries.len() {
        motion.born.push(if known || !animate { f64::NEG_INFINITY } else { now });
    }
    // Human: a note's icon plays a short piece when its tool call changes state (starts, is done, fails).
    motion.states.truncate(chat.entries.len());
    for (index, entry) in chat.entries.iter().enumerate() {
        let state = entry.step.as_ref().map(|step| step.state);
        match motion.states.get_mut(index) {
            Some(kept) if kept.0 != state => *kept = (state, now),
            Some(_) => {}
            None => motion.states.push((state, if known || !animate { f64::NEG_INFINITY } else { now })),
        }
    }
    let mut moving = false;
    let mut pick = None;
    // Human: only the question that waits for the user can be answered; an older one is history.
    let open = crate::chat::asking(&chat.entries).map(|(at, _)| at);
    egui::ScrollArea::vertical().id_salt(("conversation", key)).auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
        ui.add_space(12.0);
        ui.spacing_mut().item_spacing.y = 0.0;
        let last = chat.entries.len().saturating_sub(1);
        for (index, entry) in chat.entries.iter().enumerate() {
            if index > 0 {
                let notes = entry.role == Role::Step && chat.entries[index - 1].role == Role::Step;
                ui.add_space(if notes { CHAT_NOTE_GAP } else { CHAT_GAP });
            }
            let arrived = arrival(now, motion.born[index]);
            moving |= arrived < 1.0;
            // Human: the assistant's newest words are let out letter by letter at an even pace.
            let shown = if animate && index == last && entry.role == Role::Assistant {
                let (letters, done) = written(&mut motion, index, entry, chat.running(), step);
                moving |= !done;
                letters
            } else {
                entry.text.as_str()
            };
            // Human: looking at the document or reading the guide takes the app no time at all. Its note still
            // shows the tool at work for a moment, so the eye is seen to look and the book to turn a page.
            let shown_until = motion.born[index] + f64::from(CHAT_ICON_LEAST);
            let still_at_work = held(entry, animate && now < shown_until);
            moving |= still_at_work.is_some();
            let clock = animate.then(|| Clock { now, since: motion.states[index].1.max(shown_until) });
            moving |= clock.is_some_and(|clock| clock.settling() < 1.0);
            ui.add_space((1.0 - arrived) * CHAT_RISE);
            let picked = ui.scope(|ui| {
                ui.set_opacity(arrived);
                line(ui, still_at_work.as_ref().unwrap_or(entry), shown, Looks { scale, palette, clock, live: open == Some(index) })
            });
            if picked.inner.is_some() {
                pick = picked.inner;
            }
        }
        if let Some(pause) = chat.pause().filter(|_| !chat.entries.last().is_some_and(at_work)) {
            ui.add_space(CHAT_GAP);
            // Human: under its own words the dots alone say that more is coming.
            let wait = (pause != Pause::Writing).then(|| Wait {
                words: pause.words(chat.answering().unwrap_or_default()),
                lasted: chat.paused(),
                thought: flow(chat.thought()),
            });
            waiting(ui, wait.as_ref(), scale, palette, animate.then_some(now));
        }
        // Human: a message written while the assistant works stands greyed at the end until the assistant
        // has taken it: at its next step when it is steered, or when the run is over.
        for (text, steered) in chat.pending() {
            ui.add_space(CHAT_GAP);
            bubble(ui, palette, |ui| flowing(ui, &text, MESSAGE_TEXT_SIZE * scale, palette.muted));
            ui.add_space(CHAT_NOTE_GAP);
            let says = if steered { "Goes to the assistant at its next step" } else { "Queued until the assistant is done" };
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                ui.add(egui::Label::new(RichText::new(says).size((SIDEBAR_TEXT_SIZE - 1.0) * scale).color(palette.muted)).selectable(false));
            });
        }
        ui.add_space(12.0);
    });
    if animate && (moving || chat.running()) {
        ui.request_repaint();
    } else if chat.running() {
        // Human: nothing moves, but the seconds of a wait still count up.
        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
    }
    ui.data_mut(|data| data.insert_temp(motion_id(key), motion));
    pick
}

/// How far a line born at `born` has come in: 0 just born, 1 in place; it slows down as it gets there.
#[expect(clippy::cast_possible_truncation, reason = "a share between 0 and 1")]
fn arrival(now: f64, born: f64) -> f32 {
    let share = ((now - born) / f64::from(CHAT_APPEAR)).clamp(0.0, 1.0) as f32;
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
struct Looks {
    /// How much larger than small the text is (the settings' chat text size).
    scale: f32,
    palette: Palette,
    /// The time a note's icon moves by; `None` when nothing moves.
    clock: Option<Clock>,
    /// The line is the question that waits for the user: its answers can be picked.
    live: bool,
}

/// One line of the conversation; `shown` is as much of its text as shows. Returns what the user did with it,
/// when it is a question of the assistant's.
fn line(ui: &mut Ui, entry: &Entry, shown: &str, looks: Looks) -> Option<Pick> {
    let Looks { scale, palette, clock, live } = looks;
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
            ui.add(words(shown, palette.foreground));
        }
        // Human: a question is a card with its answers; suggestions nobody can pick any more are one quiet note.
        Role::Step => match &entry.asked {
            Some(asked) if !(asked.suggests && asked.closed) => return question(ui, asked, scale, palette, live),
            _ => note(ui, entry, scale, palette, clock),
        },
        Role::Problem => {
            ui.add(words(shown, PROBLEM));
            trouble(ui, entry, scale, palette);
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
    let quiet = palette.muted.gamma_multiply(0.7);
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
        egui::Frame::new().fill(palette.hover).corner_radius(10.0).inner_margin(egui::Margin::symmetric(11, 7)).show(ui, |ui| {
            ui.set_max_width(width);
            inside(ui);
        });
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
    let mut pasted: Vec<(&std::ops::Range<usize>, &'static str)> = entry
        .pointed
        .iter()
        .filter_map(|layer| layer.at.as_ref().map(|at| (at, layer.icon)))
        .filter(|(at, _)| entry.text.get(at.start..at.end).is_some())
        .collect();
    pasted.sort_by_key(|(at, _)| at.start);
    let mut job = LayoutJob::default();
    // Where each chip is in what is laid out, in letters, with its icon.
    let mut chips = Vec::new();
    let (mut from, mut letters) = (0, 0);
    for (at, icon) in pasted {
        if at.start < from {
            continue;
        }
        let before = &entry.text[from..at.start];
        job.append(before, 0.0, plain.clone());
        letters += before.chars().count();
        let chip = crate::chips::padded(&entry.text[at.start..at.end]);
        job.append(&chip, 0.0, tinted.clone());
        chips.push((letters, letters + chip.chars().count(), icon));
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
        for (start, end, icon) in chips {
            let (start, end) = (galley.pos_from_cursor(egui::text::CCursor::new(start)), galley.pos_from_cursor(egui::text::CCursor::new(end)));
            // A chip too long for its line is left as tinted text.
            if (start.top() - end.top()).abs() > 1.0 {
                continue;
            }
            let chip = egui::Rect::from_min_max(start.left_top(), end.left_bottom()).translate(place.min.to_vec2());
            ui.painter().rect_stroke(chip, CHIP_CORNER, egui::Stroke::new(1.0, palette.outline), egui::StrokeKind::Inside);
            let icon_at = egui::Rect::from_center_size(pos2(chip.left() + CHIP_ICON_AT, chip.center().y), Vec2::splat(CHIP_ICON));
            icons::paint(ui.painter(), icon_at, icon, palette.muted);
        }
    });
}

/// What a note says: what the tool does or did, and in quieter letters what the call was about and how often
/// it was made.
fn note_words(entry: &Entry, scale: f32, palette: Palette) -> LayoutJob {
    let font = FontId::proportional((SIDEBAR_TEXT_SIZE - 1.0) * scale);
    let quiet = palette.muted.gamma_multiply(0.7);
    let mut text = LayoutJob::default();
    let mut say = |words: &str, color: Color32| text.append(words, 0.0, TextFormat { font_id: font.clone(), color, ..TextFormat::default() });
    match &entry.step {
        Some(step) if step.state == State::Running => {
            let doing = if step.named { entry.text.clone() } else { steps::doing(&step.tool) };
            say(&format!("{doing}…"), palette.foreground.gamma_multiply(0.85));
        }
        _ => say(&entry.text, palette.muted),
    }
    if let Some(step) = &entry.step {
        if !step.detail.is_empty() {
            say(&format!("  ·  {}", step.detail), quiet);
        }
        if step.times > 1 {
            say(&format!("  ×{}", step.times), quiet);
        }
    }
    text
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
fn comet(painter: &egui::Painter, centre: egui::Pos2, radius: f32, head: f32, color: Color32) {
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
// widens and fades as the icon pops up to its full size. Failed, a red cross takes its place and shakes its
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
                ring(0.0, theme::AGENT.gamma_multiply(0.35));
                icons::paint(painter, sized(CHAT_ICON_AT_WORK, 0.0), steps::icon(&step.tool), theme::AGENT);
                return;
            };
            // Human: reading, looking and changing have moving pictures of their own (a book, an eye, a pencil).
            if let Some(act) = steps::act(&step.tool) {
                crate::tool_icons::paint(painter, slot, act, clock.now, theme::AGENT);
                return;
            }
            let breath = (clock.now * f64::from(CHAT_ICON_BREATH)).sin() as f32;
            let head = (clock.now * std::f64::consts::TAU / f64::from(CHAT_ICON_TURN)).rem_euclid(std::f64::consts::TAU) as f32;
            ring(0.0, theme::AGENT.gamma_multiply(0.14));
            comet(painter, centre, radius, head, theme::AGENT);
            icons::paint(
                painter,
                sized(CHAT_ICON_AT_WORK + 0.04 * breath, 0.0),
                steps::icon(&step.tool),
                theme::AGENT.gamma_multiply(0.8 + 0.2 * breath),
            );
        }
        State::Done => {
            if settling < 1.0 {
                ring(CHAT_ICON_BURST * settling, theme::AGENT.gamma_multiply((1.0 - settling).powi(2)));
            }
            let from = if clock.is_some() && steps::act(&step.tool).is_some() { CHAT_ICON_FROM_OWN } else { CHAT_ICON_AT_WORK };
            icons::paint(painter, sized(pop(settling, from), 0.0), steps::icon(&step.tool), theme::AGENT);
        }
        State::Failed => {
            if settling < 1.0 {
                ring(CHAT_ICON_BURST * settling, PROBLEM.gamma_multiply((1.0 - settling).powi(2)));
            }
            icons::paint(painter, sized(pop(settling, CHAT_ICON_AT_WORK), shake(settling)), steps::FAILED_ICON, PROBLEM);
        }
    }
}

/// A note on what the assistant did: its icon and its words on one line.
fn note(ui: &mut Ui, entry: &Entry, scale: f32, palette: Palette, clock: Option<Clock>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 7.0;
        let (slot, _) = ui.allocate_exact_size(Vec2::splat(CHAT_NOTE_ICON), Sense::hover());
        note_icon(ui, slot, entry, palette, clock);
        ui.add(egui::Label::new(note_words(entry, scale, palette)).truncate().selectable(false));
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
    /// A tool at work: its note.
    Tool(Entry),
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
            Status::Tool(entry) => Face::Tool(entry.clone()),
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
        Face::Tool(entry) => note_words(entry, 1.0, palette),
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
            Face::Answered => icons::paint(ui.painter(), slot, steps::ANSWERED_ICON, theme::AGENT),
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
    if animate && (chat.running() || share < 1.0) {
        ctx.request_repaint();
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
        ui.painter().circle_filled(at, CHAT_DOT / 2.0, theme::AGENT.gamma_multiply(0.35 + 0.65 * lift));
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
    let quiet = palette.muted.gamma_multiply(0.7);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = LEAD_GAP;
        let (slot, _) = ui.allocate_exact_size(vec2(DOTS_WIDTH, CHAT_NOTE_ICON), Sense::hover());
        dots(ui, slot, time);
        let Some(wait) = wait else { return };
        let mut text = LayoutJob::default();
        let mut say = |words: &str, color: Color32| text.append(words, 0.0, TextFormat { font_id: font.clone(), color, ..TextFormat::default() });
        say(&format!("{}…", wait.words), palette.foreground.gamma_multiply(0.85));
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
    let indent = DOTS_WIDTH + LEAD_GAP;
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
        assert!((arrival(10.0, 10.0)).abs() < f32::EPSILON);
        assert!((arrival(10.0 + f64::from(CHAT_APPEAR), 10.0) - 1.0).abs() < f32::EPSILON);
        assert!((arrival(10.0, f64::NEG_INFINITY) - 1.0).abs() < f32::EPSILON);
        let half = arrival(10.0 + f64::from(CHAT_APPEAR) / 2.0, 10.0);
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
        let (thinking, tool) = (Face::Waiting("Thinking…".to_owned()), Face::Tool(note));
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
