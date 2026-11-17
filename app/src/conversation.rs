//! The conversation in the chat panel: the user's messages, the assistant's words, and a note for each thing
//! it does, drawn with a little motion so what is new can be followed.
//!
//! A new line fades in and settles into place; the assistant's words flow in at an even pace however unevenly
//! they arrive; a tool at work turns a spinner beside what it is doing; three dots wait while the model thinks.
//! With "Animations & effects" off in the settings everything simply appears.

use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, FontId, Layout, RichText, Sense, Ui, Vec2, pos2, vec2};

use crate::chat::steps::{self, State};
use crate::chat::{Chat, Entry, Role};
use crate::icons;
use crate::theme::{
    self, CHAT_APPEAR, CHAT_BUBBLE_SHARE, CHAT_DOT, CHAT_DOT_GAP, CHAT_DOT_LIFT, CHAT_DOT_PACE, CHAT_GAP, CHAT_ICON_AT_WORK, CHAT_ICON_BREATH,
    CHAT_ICON_BURST, CHAT_ICON_FROM_OWN, CHAT_ICON_LEAST, CHAT_ICON_RING, CHAT_ICON_SETTLE, CHAT_ICON_SHAKE, CHAT_ICON_TAIL, CHAT_ICON_TURN,
    CHAT_NOTE_GAP, CHAT_NOTE_ICON, CHAT_RISE, CHAT_STATUS_HEIGHT, CHAT_STATUS_HOLD, CHAT_STATUS_MORPH, CHAT_STATUS_PAD, CHAT_WRITE_CATCH_UP,
    CHAT_WRITE_PACE, CHIP_CORNER, CHIP_ICON, CHIP_ICON_AT, MESSAGE_TEXT_SIZE, PROBLEM, Palette, SIDEBAR_TEXT_SIZE,
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

/// Draws the conversation of the chat `key`. `animate` is the user's "Animations & effects" setting.
// Human: the newest line stays in view while the assistant writes, unless the user has scrolled up to read.
// Agent: MAIN-THREAD; asks for another frame while something moves or the assistant is at work.
pub fn show(ui: &mut Ui, chat: &Chat, key: u64, scale: f32, palette: Palette, animate: bool) {
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
            ui.scope(|ui| {
                ui.set_opacity(arrived);
                line(ui, still_at_work.as_ref().unwrap_or(entry), shown, scale, palette, clock);
            });
        }
        if chat.running() && !chat.entries.last().is_some_and(at_work) {
            ui.add_space(CHAT_GAP);
            // Human: under its own words the dots alone say that more is coming.
            let label = if chat.entries.last().is_some_and(|entry| entry.role == Role::Assistant) { "" } else { "Thinking…" };
            waiting(ui, label, now, palette, animate);
        }
        // Human: a message written while the assistant works stands greyed at the end until the assistant
        // has taken it: at its next step when it is steered, or when the run is over.
        for (text, steered) in chat.pending() {
            ui.add_space(CHAT_GAP);
            let width = ui.available_width() * CHAT_BUBBLE_SHARE;
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                egui::Frame::new().fill(palette.hover).corner_radius(10.0).inner_margin(egui::Margin::symmetric(11, 7)).show(ui, |ui| {
                    ui.set_max_width(width);
                    ui.add(egui::Label::new(RichText::new(&text).size(MESSAGE_TEXT_SIZE * scale).color(palette.muted)).wrap());
                });
            });
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
    }
    ui.data_mut(|data| data.insert_temp(motion_id(key), motion));
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

/// One line of the conversation; `shown` is as much of its text as shows.
/// `scale` is how much larger than small the text is (the settings' chat text size).
fn line(ui: &mut Ui, entry: &Entry, shown: &str, scale: f32, palette: Palette, clock: Option<Clock>) {
    let words = |text: &str, color: Color32| egui::Label::new(RichText::new(text).size(MESSAGE_TEXT_SIZE * scale).color(color)).wrap();
    match entry.role {
        Role::User => {
            let width = ui.available_width() * CHAT_BUBBLE_SHARE;
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                egui::Frame::new().fill(palette.hover).corner_radius(10.0).inner_margin(egui::Margin::symmetric(11, 7)).show(ui, |ui| {
                    ui.set_max_width(width);
                    if entry.pointed.is_empty() {
                        ui.add(words(shown, palette.foreground));
                    } else {
                        pointing(ui, entry, scale, palette);
                    }
                });
            });
        }
        Role::Assistant => {
            ui.add(words(shown, palette.foreground));
        }
        Role::Step => note(ui, entry, scale, palette, clock),
        Role::Problem => {
            ui.add(words(shown, PROBLEM));
        }
    }
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
    /// The model is thinking, or writing its answer.
    Waiting(&'static str),
    /// The answer is there to read.
    Answered,
    /// The run ended with this problem.
    Problem(&'a str),
}

/// What that line says: `running` while a message is being answered, and for a while after (`fresh`).
fn status(entries: &[Entry], running: bool, fresh: bool) -> Option<Status<'_>> {
    let last = entries.last()?;
    if running {
        return Some(match entries.iter().rev().find(|entry| at_work(entry)) {
            Some(note) => Status::Tool(note),
            None if last.role == Role::Assistant => Status::Waiting("Writing…"),
            None => Status::Waiting("Thinking…"),
        });
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
    /// The dots and a word beside them.
    Waiting(&'static str),
    /// The answer is there to read.
    Answered,
    /// The run ended with this problem.
    Problem(String),
}

impl From<Status<'_>> for Face {
    fn from(status: Status<'_>) -> Self {
        match status {
            Status::Tool(entry) => Face::Tool(entry.clone()),
            Status::Waiting(label) => Face::Waiting(label),
            Status::Answered => Face::Answered,
            Status::Problem(text) => Face::Problem(text.to_owned()),
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
    /// What it showed before, and how wide the island was when that gave way; `None` when the line was not
    /// there before.
    from: Option<(Face, f32)>,
    /// How wide the island was drawn last.
    width: f32,
}

impl Morph {
    /// The line at rest showing `face`: nothing moves ("Animations & effects" is off).
    fn still(face: Face) -> Self {
        Morph { face, since: f64::NEG_INFINITY, from: None, width: 0.0 }
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
            Some(kept) => Morph { from: Some((kept.face, kept.width)), face: wanted, since: now, width: kept.width },
            None => Morph { face: wanted, since: now, from: None, width: 0.0 },
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

/// Where and how a face of the status line is painted.
#[derive(Clone, Copy)]
struct Spot {
    /// The island as it is drawn now.
    island: egui::Rect,
    /// The widest the island gets.
    most_width: f32,
    /// How much of the face shows.
    opacity: f32,
    /// egui's time, by which icons and dots move; `None` when nothing moves.
    time: Option<f64>,
}

/// Paints `face` in the middle of the island at `place`.
fn paint_face(ui: &mut Ui, face: &Face, place: Spot, palette: Palette) {
    let Spot { island, most_width, opacity, time } = place;
    if opacity <= 0.0 {
        return;
    }
    let (lead, words) = laid_out(ui.ctx(), face, most_width, palette);
    let left = island.center().x - (lead + LEAD_GAP + words.size().x) / 2.0;
    let slot = egui::Rect::from_min_size(pos2(left, island.center().y - CHAT_NOTE_ICON / 2.0), vec2(lead, CHAT_NOTE_ICON));
    ui.scope(|ui| {
        ui.multiply_opacity(opacity);
        // Human: while the island is narrower than the words, they show through it as through a window.
        ui.set_clip_rect(island.shrink(1.0).intersect(ui.clip_rect()));
        match face {
            Face::Tool(entry) => note_icon(ui, slot, entry, palette, time.map(Clock::settled)),
            Face::Waiting(_) => dots(ui, slot, time),
            Face::Answered => icons::paint(ui.painter(), slot, steps::ANSWERED_ICON, theme::AGENT),
            Face::Problem(_) => icons::paint(ui.painter(), slot, steps::FAILED_ICON, PROBLEM),
        }
        let at = pos2(slot.right() + LEAD_GAP, island.center().y - words.size().y / 2.0);
        ui.painter().galley(at, words, palette.muted);
    });
}

fn status_id() -> egui::Id {
    egui::Id::unique("chat-status-morph")
}

/// Draws the chat's one-line status as a small island: what the assistant is doing right now, or that its
/// answer is there. Returns whether it was clicked (the sidebar then opens the chat).
// Human: with the sidebar folded away the conversation can't be seen, so without this line a message sent
// from the bar would get no sign of life until the design changes. It is one island all through a run: when
// what it says changes, it grows or shrinks to the new words while the old ones fade and the new ones come.
// Agent: MAIN-THREAD; a floating layer over the canvas; asks for the next frame while the assistant works or
// the island turns.
pub fn status_line(ctx: &egui::Context, line: &Line, chat: &Chat, palette: Palette, animate: bool) -> bool {
    let Line { foot, most_width, fresh, opacity } = *line;
    let Some(status) = status(&chat.entries, chat.running(), fresh) else {
        ctx.data_mut(|data| data.remove::<Morph>(status_id()));
        return false;
    };
    let now = ctx.input(|input| input.time);
    let mut morph = if animate { Morph::next(ctx.data(|data| data.get_temp(status_id())), status.into(), now) } else { Morph::still(status.into()) };
    let share = morph.share(now);
    let (lead, words) = laid_out(ctx, &morph.face, most_width, palette);
    let full = (2.0 * CHAT_STATUS_PAD + lead + LEAD_GAP + words.size().x).min(most_width);
    // Human: a line that was not there before simply fades in at its size.
    let (width, appeared) = match &morph.from {
        Some((_, from)) => (from + (full - from) * eased(share), 1.0),
        None => (full, share),
    };
    morph.width = width;
    let place = egui::Rect::from_min_size(pos2(foot.x - width / 2.0, foot.y - CHAT_STATUS_HEIGHT), vec2(width, CHAT_STATUS_HEIGHT));
    let time = animate.then_some(now);
    let shown = egui::Area::new(egui::Id::unique("chat-status")).fade_in(false).fixed_pos(place.min).order(egui::Order::Foreground).show(ctx, |ui| {
        ui.multiply_opacity(opacity * appeared);
        let (island, response) = ui.allocate_exact_size(place.size(), Sense::click());
        crate::sidebar::island(ui, island, CHAT_STATUS_HEIGHT / 2.0, palette);
        let (old, new) = if morph.from.is_some() { crossfade(share) } else { (0.0, 1.0) };
        if let Some((before, _)) = &morph.from {
            paint_face(ui, before, Spot { island, most_width, opacity: old, time }, palette);
        }
        paint_face(ui, &morph.face, Spot { island, most_width, opacity: new, time }, palette);
        response
    });
    if animate && (chat.running() || share < 1.0) {
        ctx.request_repaint();
    }
    ctx.data_mut(|data| data.insert_temp(status_id(), morph));
    let response = shown.inner;
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
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

/// The dots and `label` beside them: the model is thinking.
fn waiting(ui: &mut Ui, label: &str, now: f64, palette: Palette, animate: bool) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = LEAD_GAP;
        let (slot, _) = ui.allocate_exact_size(vec2(DOTS_WIDTH, CHAT_NOTE_ICON), Sense::hover());
        dots(ui, slot, animate.then_some(now));
        if !label.is_empty() {
            ui.add(egui::Label::new(RichText::new(label).size(SIDEBAR_TEXT_SIZE - 1.0).color(palette.muted)).selectable(false));
        }
    });
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
        assert_eq!(status(&[], true, true), None);
        assert_eq!(status(&entries, true, false), Some(Status::Waiting("Thinking…")));
        entries.extend(crate::chat::sample().into_iter().filter(|entry| entry.step.is_some()).take(2));
        // A finished tool is no news; one at work is.
        assert_eq!(status(&entries, true, false), Some(Status::Waiting("Thinking…")));
        if let Some(step) = entries[2].step.as_mut() {
            step.state = State::Running;
        }
        assert!(matches!(status(&entries, true, false), Some(Status::Tool(note)) if note == &entries[2]));
        entries.push(Entry::new(Role::Assistant, "A dog."));
        if let Some(step) = entries[2].step.as_mut() {
            step.state = State::Done;
        }
        assert_eq!(status(&entries, true, false), Some(Status::Waiting("Writing…")));
        // After the run the line says the answer is there, for a while.
        assert_eq!((status(&entries, false, true), status(&entries, false, false)), (Some(Status::Answered), None));
        entries.push(Entry::new(Role::Problem, "No connection"));
        assert_eq!(status(&entries, false, true), Some(Status::Problem("No connection")));
    }

    #[test]
    fn the_status_line_turns_from_one_thing_into_the_next() {
        let mut note = crate::chat::sample().into_iter().find(|entry| entry.step.is_some()).unwrap();
        if let Some(step) = note.step.as_mut() {
            step.state = State::Running;
        }
        let (thinking, tool) = (Face::Waiting("Thinking…"), Face::Tool(note));
        // A line that wasn't there comes from nothing; one that says the same stays as it is.
        let first = Morph::next(None, thinking.clone(), 10.0);
        assert_eq!((first.from.as_ref(), first.since), (None, 10.0));
        let first = Morph { width: 90.0, ..first };
        assert_eq!(Morph::next(Some(first.clone()), thinking.clone(), 11.0), first);
        // A tool takes over at once, from the width the island had.
        let at_work = Morph::next(Some(first), tool.clone(), 12.0);
        assert_eq!((&at_work.face, at_work.from.as_ref(), at_work.since), (&tool, Some(&(thinking.clone(), 90.0)), 12.0));
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
    fn only_a_tool_at_work_hides_the_thinking_dots() {
        let mut entries = crate::chat::sample();
        assert!(!entries.iter().any(at_work));
        if let Some(step) = entries[2].step.as_mut() {
            step.state = State::Running;
        }
        assert!(at_work(&entries[2]) && !at_work(&entries[0]));
    }
}
