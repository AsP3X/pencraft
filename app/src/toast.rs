//! The message lines: short sentences shown over the bottom of the window for a few seconds (what a model
//! just changed, what a button could not do), each on a capsule that opens out of a dot and closes into the
//! dot again before it goes. Up to [`TOAST_MOST`] show at once, the newest at the bottom; a problem looks
//! different from a note and stays longer; the pointer over a message holds it there.

use std::time::{Duration, Instant};

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

use crate::theme::{
    self, AGENT, PROBLEM, TOAST_CLOSING, TOAST_DOT, TOAST_FLASH, TOAST_GAP, TOAST_HEIGHT, TOAST_ICON, TOAST_LEAVE, TOAST_LIFT, TOAST_MARGIN,
    TOAST_MORPH, TOAST_MOST, TOAST_PADDING, TOAST_PROBLEM_TIME, TOAST_RISE, TOAST_STACK_GAP, TOAST_TEXT, TOAST_TIME, TOAST_WIDTH,
};

/// The icon before a problem.
const PROBLEM_ICON: &str = "triangle-alert";

/// What kind of sentence a message is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Something that happened or was done.
    Note,
    /// Something that went wrong.
    Problem,
}

impl Kind {
    /// How long a message of this kind shows.
    // Human: a problem is the sentence that must not be missed, and it is often longer (it says why), so it
    // stays twice as long as a note. It does not stay until it is clicked away: it lies over the canvas,
    // and a message that never goes would be in the way of the work it reports on.
    fn time(self) -> Duration {
        match self {
            Kind::Note => TOAST_TIME,
            Kind::Problem => TOAST_PROBLEM_TIME,
        }
    }
}

/// What kind of sentence `text` is, going by its words: a problem when it says something couldn't be done,
/// failed, or isn't there.
// Human: for the sentences whose maker doesn't say which they are. A wrong guess only changes how a message
// looks and how long it stays, never what it says.
#[must_use]
pub fn kind_of(text: &str) -> Kind {
    const TELLS: [&str; 10] = ["couldn't", "can't", "cannot", "didn't", "wasn't", "weren't", "isn't there", "failed", "error", "problem"];
    let lower = text.to_lowercase().replace('’', "'");
    if TELLS.iter().any(|tell| lower.contains(tell)) { Kind::Problem } else { Kind::Note }
}

/// One sentence shown.
struct Toast {
    /// A number of its own among this window's messages (its name for a screen reader's tree).
    serial: u64,
    /// Whose it is (a model's name), shown before it with a dot in the colour of a model at work; empty for
    /// the app's own.
    who: String,
    text: String,
    kind: Kind,
    /// How long it has shown, not counting the time the pointer was over it.
    age: Duration,
    /// When it was last drawn; `None` before its first frame.
    drawn: Option<Instant>,
    /// Where its capsule was last drawn, for the pointer to be over.
    place: Option<Rect>,
    /// The pointer holds it there: its time stands still.
    held: bool,
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
    /// How it looks now, when at rest it is `full` large. `moving` is the "Animations and effects" setting.
    // Human: a sentence opens out of a dot as wide as one line's capsule is high, rising a little and fading
    // in. The text comes once the capsule is most of the way open, and a flash runs over it.
    // Human: when its time is over it goes the way it came: the text goes first, the capsule closes into the
    // dot again (the first `TOAST_CLOSING` of `TOAST_LEAVE`), and then the dot fades away.
    // Human: with animations off it is simply there for its time, whole and still, and then gone.
    fn look(&self, full: Vec2, moving: bool) -> Look {
        if !moving {
            return Look { size: full, alpha: 1.0, text: 1.0, flash: 0.0, sunk: 0.0 };
        }
        let age = self.age;
        let open = eased(age.as_secs_f32() / TOAST_MORPH.as_secs_f32());
        let dot = Vec2::splat(TOAST_HEIGHT.min(full.y));
        let left = self.kind.time().saturating_sub(age);
        let leaving = 1.0 - (left.as_secs_f32() / TOAST_LEAVE.as_secs_f32()).clamp(0.0, 1.0);
        let closed = eased(leaving / TOAST_CLOSING);
        let opened = dot + (full - dot) * open;
        Look {
            size: opened + (dot - opened) * closed,
            alpha: (open * 3.0).min(1.0) * ((1.0 - leaving) / (1.0 - TOAST_CLOSING)).clamp(0.0, 1.0),
            text: ((open - 0.4) / 0.6).clamp(0.0, 1.0) * (1.0 - closed * 2.5).clamp(0.0, 1.0),
            flash: (1.0 - age.as_secs_f32() / TOAST_FLASH.as_secs_f32()).clamp(0.0, 1.0),
            sunk: (1.0 - open) * TOAST_RISE,
        }
    }

    /// Lets the time since it was last drawn pass for it, unless the pointer is `over` it while it is at
    /// rest. Returns whether the pointer holds it.
    // Human: someone reading a message, or about to, holds the pointer on it; its time stands still meanwhile.
    // Only a message that is open and not yet leaving is held: one that is opening or closing goes on, so
    // a pointer that happens to lie where a message appears never stops it half open.
    fn pass(&mut self, now: Instant, over: bool) -> bool {
        let since = self.drawn.map_or(Duration::ZERO, |drawn| now.saturating_duration_since(drawn));
        self.drawn = Some(now);
        let resting = self.age >= TOAST_MORPH.max(TOAST_FLASH) && self.age < self.kind.time().saturating_sub(TOAST_LEAVE);
        let held = over && resting;
        if !held {
            self.age += since;
        }
        held
    }
}

/// A problem that lasts (settings that can't be written, a recovery copy that can't be kept): said once, and
/// again only after it went away and came back.
// Human: a failure that repeats every second would otherwise cover the window with the same sentence.
#[derive(Default)]
pub struct Lasting {
    said: bool,
}

impl Lasting {
    /// The problem is there now; whether to say it (the first time since it last went away).
    pub fn fails(&mut self) -> bool {
        !std::mem::replace(&mut self.said, true)
    }

    /// It worked: the next failure is said again.
    pub fn works(&mut self) {
        self.said = false;
    }
}

/// The message lines of the window.
#[derive(Default)]
pub struct Toasts {
    /// The messages showing, the oldest first.
    list: Vec<Toast>,
    /// How many messages were said so far.
    said: u64,
}

impl Toasts {
    /// The message lines, saying `problem` from the start when there is one (found before the window opened).
    #[must_use]
    pub fn saying(problem: Option<&str>) -> Toasts {
        let mut toasts = Toasts::default();
        if let Some(text) = problem {
            toasts.say("", text, Kind::Problem);
        }
        toasts
    }

    /// Shows `text` from now on, under the messages that are showing. `who` is the model that says it; empty
    /// for the app.
    // Human: three messages at most; a fourth pushes the oldest out. A sentence that is said again while it
    // is the newest one showing is not shown twice: its time starts over.
    pub fn say(&mut self, who: &str, text: &str, kind: Kind) {
        let (who, text) = (who.trim(), text.trim());
        if let Some(last) = self.list.last_mut().filter(|last| last.who == who && last.text == text && last.kind == kind) {
            last.age = last.age.min(TOAST_MORPH);
            return;
        }
        self.said += 1;
        self.list.push(Toast {
            serial: self.said,
            who: who.to_owned(),
            text: text.to_owned(),
            kind,
            age: Duration::ZERO,
            drawn: None,
            place: None,
            held: false,
        });
        let over = self.list.len().saturating_sub(TOAST_MOST);
        self.list.drain(..over);
    }

    /// Draws the messages over everything else, while there are any. `moving` is the "Animations and effects"
    /// setting: without it a message is there at once and gone at once.
    // Human: a sentence stays on one line while it fits `TOAST_WIDTH` (or the window), takes a second line
    // when it doesn't, and ends in "…" when two are not enough; the capsule is as wide as its text.
    // Agent: MAIN-THREAD, once a frame; asks for the next frame while something moves, and for the one the
    // next message starts to leave at otherwise. Tells screen readers each message as a live region.
    pub fn show(&mut self, ctx: &egui::Context, dark: bool, moving: bool) {
        let now = Instant::now();
        let hover = ctx.input(|input| input.pointer.hover_pos());
        for toast in &mut self.list {
            let over = hover.is_some_and(|at| toast.place.is_some_and(|place| place.contains(at)));
            toast.held = toast.pass(now, over);
        }
        self.list.retain(|toast| toast.age < toast.kind.time());
        if self.list.is_empty() {
            return;
        }
        let palette = theme::palette(dark);
        let screen = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("message")));
        // The newest lies lowest; the others stand on it.
        let mut ground = screen.bottom() - TOAST_LIFT;
        let (mut busy, mut next) = (false, None::<Duration>);
        let mut told = Vec::new();
        for toast in self.list.iter_mut().rev() {
            let mark = match (toast.kind, toast.who.is_empty()) {
                (Kind::Problem, _) => TOAST_ICON + TOAST_GAP,
                (Kind::Note, false) => TOAST_DOT + TOAST_GAP,
                (Kind::Note, true) => 0.0,
            };
            let room = (TOAST_WIDTH.min(screen.width() - 2.0 * TOAST_MARGIN) - 2.0 * TOAST_PADDING - mark).max(TOAST_HEIGHT);
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
            let full = egui::vec2(measured.size().x + mark + 2.0 * TOAST_PADDING, (measured.size().y + TOAST_PADDING).max(TOAST_HEIGHT));
            let look = toast.look(full, moving);
            let centre = Pos2::new(screen.center().x, ground - full.y / 2.0 + look.sunk);
            // Human: a message that comes or goes takes and gives its room as it shows, so the ones above
            // it move to their place and don't jump there.
            ground -= (full.y + TOAST_STACK_GAP) * look.alpha;
            let capsule = Rect::from_center_size(centre, look.size);
            toast.place = Some(capsule);
            told.push((
                toast.serial,
                capsule,
                toast.kind,
                if toast.who.is_empty() { toast.text.clone() } else { format!("{}: {}", toast.who, toast.text) },
            ));
            let radius = (look.size.y / 2.0).min(TOAST_HEIGHT / 2.0);
            let shadow = egui::epaint::Shadow { offset: [0, 6], blur: 22, spread: 0, color: Color32::BLACK.gamma_multiply(0.22 * look.alpha) };
            #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a radius of at most half the capsule's height")]
            painter.add(shadow.as_shape(capsule, egui::CornerRadius::same(radius as u8)));
            // Human: a problem is told apart by more than its colour: a warning sign stands before it, and
            // its capsule has the problem colour's edge.
            let edge = if toast.kind == Kind::Problem { PROBLEM } else { palette.outline };
            painter.rect(
                capsule,
                radius,
                palette.island.gamma_multiply(look.alpha),
                Stroke::new(1.0, edge.gamma_multiply(look.alpha)),
                StrokeKind::Inside,
            );
            // What is on the capsule keeps its place while the capsule opens around it.
            let inside = painter.with_clip_rect(capsule.shrink2(egui::vec2(radius / 2.0, 0.0)));
            let seen = look.alpha * look.text;
            let content = Rect::from_center_size(centre, egui::vec2(full.x - 2.0 * TOAST_PADDING, measured.size().y));
            let first = measured.rows.first().map_or(measured.size().y, |row| row.height());
            match (toast.kind, toast.who.is_empty()) {
                (Kind::Problem, _) => {
                    let sign =
                        Rect::from_center_size(Pos2::new(content.left() + TOAST_ICON / 2.0, content.top() + first / 2.0), Vec2::splat(TOAST_ICON));
                    crate::icons::paint(&inside, sign, PROBLEM_ICON, PROBLEM.gamma_multiply(seen));
                }
                (Kind::Note, false) => {
                    let middle = Pos2::new(content.left() + TOAST_DOT / 2.0, content.top() + first / 2.0);
                    inside.circle_filled(middle, TOAST_DOT / 2.0, AGENT.gamma_multiply(seen));
                }
                (Kind::Note, true) => {}
            }
            inside.galley(Pos2::new(content.left() + mark, content.top()), lay(seen), Color32::TRANSPARENT);
            let wash = match (toast.kind, toast.who.is_empty()) {
                (Kind::Problem, _) => PROBLEM,
                (Kind::Note, false) => AGENT,
                (Kind::Note, true) => palette.foreground,
            };
            flash(&painter, capsule, radius, wash, look.flash * look.alpha);
            busy |= look.flash > 0.0 || look.alpha < 1.0 || look.size != full;
            let rest = toast.kind.time().saturating_sub(if moving { TOAST_LEAVE } else { Duration::ZERO }).saturating_sub(toast.age);
            // A message the pointer holds waits for the pointer, not for a time.
            next = next.into_iter().chain((!toast.held).then_some(rest)).min();
        }
        tell(ctx, screen, &told);
        if busy {
            ctx.request_repaint();
        } else if let Some(next) = next {
            ctx.request_repaint_after(next + Duration::from_millis(1));
        }
    }
}

/// Gives screen readers the messages: each one a part of the window with its sentence, read out when it
/// comes (a problem at once, a note when the reader is free).
// Agent: an area that takes no clicks, so what is under a message is still reached through it.
fn tell(ctx: &egui::Context, screen: Rect, told: &[(u64, Rect, Kind, String)]) {
    egui::Area::new(egui::Id::unique("messages-told")).order(egui::Order::Tooltip).fixed_pos(screen.min).interactable(false).show(ctx, |ui| {
        for (serial, capsule, kind, text) in told {
            let response = ui.interact(*capsule, ui.scope_id().with(("message", *serial)), egui::Sense::hover());
            let (role, live) = match kind {
                Kind::Note => (egui::Role::Status, egui::accesskit::Live::Polite),
                Kind::Problem => (egui::Role::Alert, egui::accesskit::Live::Assertive),
            };
            response.widget_info(|| egui::WidgetInfo::labeled(role, true, text));
            ctx.accesskit_node_builder(response.id, |node| node.set_live(live));
        }
    });
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
    fn a_lasting_failure_is_said_once_and_again_only_after_it_worked() {
        let mut lasting = Lasting::default();
        assert!(lasting.fails());
        assert!(!lasting.fails() && !lasting.fails());
        lasting.works();
        assert!(lasting.fails());
    }

    /// The newest message, `age` old.
    fn aged(toasts: &mut Toasts, age: Duration) -> &Toast {
        let toast = toasts.list.last_mut().unwrap();
        toast.age = age;
        toast
    }

    #[test]
    fn a_sentence_opens_out_of_a_dot_and_closes_into_it() {
        let full = egui::vec2(300.0, TOAST_HEIGHT);
        let mut toasts = Toasts::default();
        toasts.say("", " Saved ", Kind::Note);
        assert_eq!(toasts.list[0].text, "Saved");
        // It starts as a dot under its place, unseen, with the flash at its strongest.
        let first = aged(&mut toasts, Duration::ZERO).look(full, true);
        assert_eq!((first.size, first.alpha, first.text, first.flash), (Vec2::splat(TOAST_HEIGHT), 0.0, 0.0, 1.0));
        assert!((first.sunk - TOAST_RISE).abs() < 0.01);
        // Open, it is at rest: whole, in place, the flash over.
        let rest = aged(&mut toasts, TOAST_MORPH.max(TOAST_FLASH)).look(full, true);
        assert_eq!(rest, Look { size: full, alpha: 1.0, text: 1.0, flash: 0.0, sunk: 0.0 });
        // At the end the text goes, the capsule closes into the dot again, and then the dot fades.
        let closing = aged(&mut toasts, TOAST_TIME.saturating_sub(TOAST_LEAVE.mul_f32(1.0 - TOAST_CLOSING / 2.0))).look(full, true);
        assert!(closing.size.x < full.x && closing.size.x > TOAST_HEIGHT && closing.alpha > 0.99 && closing.text < 0.01, "{closing:?}");
        let dot = aged(&mut toasts, TOAST_TIME.saturating_sub(TOAST_LEAVE.mul_f32((1.0 - TOAST_CLOSING) / 2.0))).look(full, true);
        assert!(dot.size == Vec2::splat(TOAST_HEIGHT) && dot.alpha > 0.4 && dot.alpha < 0.6 && dot.sunk == 0.0, "{dot:?}");
    }

    #[test]
    fn with_animations_off_a_sentence_is_whole_and_still_for_its_time() {
        let full = egui::vec2(300.0, TOAST_HEIGHT);
        let mut toasts = Toasts::default();
        toasts.say("", "Saved", Kind::Note);
        let still = Look { size: full, alpha: 1.0, text: 1.0, flash: 0.0, sunk: 0.0 };
        for age in [Duration::ZERO, TOAST_MORPH / 2, TOAST_TIME.saturating_sub(TOAST_LEAVE / 2)] {
            assert_eq!(aged(&mut toasts, age).look(full, false), still);
        }
    }

    #[test]
    fn three_sentences_show_at_most_the_newest_last_and_a_repeated_one_starts_over() {
        let mut toasts = Toasts::default();
        for text in ["one", "two", "three", "four"] {
            toasts.say("", text, Kind::Note);
        }
        let texts = |toasts: &Toasts| toasts.list.iter().map(|toast| toast.text.clone()).collect::<Vec<_>>();
        assert_eq!(texts(&toasts), ["two", "three", "four"]);
        assert_eq!(TOAST_MOST, 3);
        // Said again while it is the newest: one line, and its time starts over (it is not opened anew).
        aged(&mut toasts, Duration::from_secs(4));
        toasts.say("", "four", Kind::Note);
        assert_eq!(texts(&toasts), ["two", "three", "four"]);
        assert_eq!(toasts.list[2].age, TOAST_MORPH);
        // The same words as a problem, or from a model, are another sentence.
        toasts.say("", "four", Kind::Problem);
        toasts.say("Claude", "four", Kind::Problem);
        assert_eq!(texts(&toasts), ["four", "four", "four"]);
        assert_eq!(toasts.list.iter().map(|toast| toast.serial).collect::<Vec<_>>(), [4, 5, 6]);
    }

    #[test]
    fn time_stands_still_for_a_sentence_under_the_pointer_and_a_problem_stays_longer() {
        let start = Instant::now();
        let mut toasts = Toasts::default();
        toasts.say("", "Saved", Kind::Note);
        let toast = toasts.list.last_mut().unwrap();
        // Its first frame starts its time.
        toast.pass(start, false);
        assert_eq!(toast.age, Duration::ZERO);
        toast.pass(start + Duration::from_secs(2), false);
        assert_eq!(toast.age, Duration::from_secs(2));
        // Three seconds under the pointer don't count; the time after they do.
        assert!(toast.pass(start + Duration::from_secs(5), true));
        assert_eq!(toast.age, Duration::from_secs(2));
        assert!(!toast.pass(start + Duration::from_secs(6), false));
        assert_eq!(toast.age, Duration::from_secs(3));
        // While it opens and while it leaves the pointer doesn't hold it: it never stops half open.
        toast.age = TOAST_MORPH / 2;
        assert!(!toast.pass(start + Duration::from_secs(7), true));
        assert_eq!(toast.age, TOAST_MORPH / 2 + Duration::from_secs(1));
        toast.age = TOAST_TIME.saturating_sub(TOAST_LEAVE / 2);
        assert!(!toast.pass(start + Duration::from_secs(8), true));
        assert!(toast.age > TOAST_TIME);
        assert!(Kind::Problem.time() > Kind::Note.time());
        assert_eq!(Toasts::saying(Some("The settings file couldn't be read.")).list[0].kind, Kind::Problem);
    }

    #[test]
    fn the_messages_are_drawn_stacked_and_go_when_their_time_is_over() {
        let ctx = egui::Context::default();
        let mut toasts = Toasts::default();
        toasts.say("", "The file is saved", Kind::Note);
        toasts.say("", "Couldn't save: the disk is full", Kind::Problem);
        let frame = |toasts: &mut Toasts, moving: bool| {
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(900.0, 700.0))), ..Default::default() };
            let mut drawn = ctx.run_ui(input, |_| toasts.show(&ctx, false, moving));
            drawn.textures_delta.clear();
        };
        frame(&mut toasts, false);
        let [note, problem] = [toasts.list[0].place.unwrap(), toasts.list[1].place.unwrap()];
        // The newer one lies under the older one, clear of it, both in the middle.
        assert!(problem.top() >= note.bottom() + TOAST_STACK_GAP - 0.5, "{note:?} {problem:?}");
        assert!((note.center().x - 450.0).abs() < 0.5 && (problem.center().x - 450.0).abs() < 0.5);
        assert!((problem.bottom() - (700.0 - TOAST_LIFT)).abs() < 0.5);
        // The note's time is over first; the problem is still there, and then it goes too.
        for toast in &mut toasts.list {
            toast.age = TOAST_TIME;
        }
        frame(&mut toasts, false);
        assert_eq!(toasts.list.iter().map(|toast| toast.kind).collect::<Vec<_>>(), [Kind::Problem]);
        toasts.list[0].age = TOAST_PROBLEM_TIME;
        frame(&mut toasts, true);
        assert!(toasts.list.is_empty());
    }

    #[test]
    fn a_sentence_that_says_something_went_wrong_is_a_problem() {
        for text in ["Couldn't export: the disk is full", "The key couldn’t be saved in the keychain: no access", "The download failed."] {
            assert_eq!(kind_of(text), Kind::Problem, "{text}");
        }
        for text in ["Saved.", "Exported home.png", "Looking for a new version…", "A message can carry 4 files."] {
            assert_eq!(kind_of(text), Kind::Note, "{text}");
        }
    }

    #[test]
    fn the_problem_sign_is_bundled() {
        assert!(crate::icons::outline("lucide", PROBLEM_ICON).is_some());
    }
}
