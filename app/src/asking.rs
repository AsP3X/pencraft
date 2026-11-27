//! How a question of the assistant's is drawn: its words, the answers to tick as a column of rows, a field for
//! an answer of the user's own, a button that sends what is chosen, and a cross to close it. The conversation
//! shows it as a card, and the folded sidebar as a dialog over the message bar; both are this one sheet.
//!
//! A sheet is laid out before it is drawn, so the dialog's island knows how large to grow.

use std::sync::Arc;

use egui::text::LayoutJob;
use egui::{Color32, FontId, Galley, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};

use crate::chat::asks::Asked;
use crate::chat::steps;
use crate::icons;
use crate::theme::{
    self, CHAT_ASK_CLOSE, CHAT_ASK_GAP, CHAT_ASK_OPTION, CHAT_ASK_OPTION_GAP, CHAT_ASK_OPTION_PAD, CHAT_ASK_OPTION_RADIUS, CHAT_ASK_PAD,
    CHAT_ASK_SEND, CHAT_ASK_TICK, CHAT_NOTE_GAP, CHAT_NOTE_ICON, CHIP_RADIUS, MESSAGE_TEXT_SIZE, Palette, SIDEBAR_TEXT_SIZE,
};

/// The room between an icon and the words beside it.
const LEAD_GAP: f32 = 7.0;
/// The most lines an answer's words take.
const OPTION_ROWS: usize = 2;
/// The most lines a question's words take.
const QUESTION_ROWS: usize = 6;

/// What the user did with a question.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    /// Ticked `answer` of the question at `question`, or took its tick away.
    Toggle {
        /// The question's place among the questions asked together.
        question: usize,
        /// The answer's words.
        answer: String,
    },
    /// Wrote `text` as an answer of their own to the question at `question`.
    Write {
        /// The question's place among the questions asked together.
        question: usize,
        /// All that is written so far.
        text: String,
    },
    /// Sent what is ticked and written as the answer to the question at `question`.
    Send {
        /// The question's place among the questions asked together.
        question: usize,
    },
    /// Closed it without answering.
    Dismiss,
    /// Took back the message at this place of those that wait to be sent: not an answer to a question, but
    /// something the user did in the conversation that the chat takes the same way.
    Unqueue(usize),
}

/// How a sheet is laid out.
#[derive(Clone, Copy)]
pub struct Style {
    /// How much larger than small its text is (the settings' chat text size).
    pub scale: f32,
    /// The questions already answered stand over the one the user is at, each with its answer: the
    /// conversation keeps them, the dialog shows only what is still to answer.
    pub history: bool,
    /// The colours.
    pub palette: Palette,
}

/// One thing on a sheet, placed from the sheet's top-left corner.
enum Item {
    /// Words, with an icon before their first line if given.
    Words { at: Pos2, galley: Arc<Galley>, icon: Option<(Rect, &'static str, Color32)> },
    /// An answer to tick.
    Answer { rect: Rect, galley: Arc<Galley>, answer: String, ticked: bool },
    /// The field for an answer of the user's own: what is written in it so far, and what it says while empty.
    Own { rect: Rect, text: String, hint: &'static str, size: f32 },
    /// The button that sends what is chosen; `ready` once something is.
    Send { rect: Rect, galley: Arc<Galley>, ready: bool },
    /// The cross that closes the question.
    Close(Rect),
}

/// A question laid out for a width.
pub struct Sheet {
    items: Vec<Item>,
    /// How large it is.
    pub size: Vec2,
    /// The place of the question the user is at.
    question: usize,
    /// Something is chosen for it, so it can be sent.
    ready: bool,
}

/// What is being laid out: the items so far and how far down they reach.
struct Column<'a> {
    ctx: &'a egui::Context,
    items: Vec<Item>,
    /// The sheet's width.
    width: f32,
    /// Where the next item starts.
    y: f32,
}

impl Column<'_> {
    /// `text` laid out in letters of `size`, at most `width` wide and `rows` lines long.
    fn laid_out(&self, text: &str, size: f32, color: Color32, width: f32, rows: usize) -> Arc<Galley> {
        let mut job = LayoutJob::simple(text.to_owned(), FontId::proportional(size), color, width.max(0.0));
        job.wrap.max_rows = rows;
        self.ctx.fonts_mut(|fonts| fonts.layout_job(job))
    }

    /// Adds a paragraph with `icon` before its first line; `spared` is room left free at its right end.
    fn words(&mut self, icon: Option<(&'static str, Color32)>, text: &str, size: f32, color: Color32, spared: f32) {
        let left = CHAT_ASK_PAD + CHAT_NOTE_ICON + LEAD_GAP;
        let galley = self.laid_out(text, size, color, self.width - left - CHAT_ASK_PAD - spared, QUESTION_ROWS);
        // Human: the icon stands in the middle of the first line, however many lines follow.
        let first = galley.rows.first().map_or(galley.size().y, |row| row.height());
        let height = galley.size().y.max(CHAT_NOTE_ICON);
        let slot = Rect::from_min_size(pos2(CHAT_ASK_PAD, self.y + (first.max(CHAT_NOTE_ICON) - CHAT_NOTE_ICON) / 2.0), Vec2::splat(CHAT_NOTE_ICON));
        let at = pos2(left, self.y + (CHAT_NOTE_ICON - first).max(0.0) / 2.0);
        self.items.push(Item::Words { at, galley, icon: icon.map(|(name, color)| (slot, name, color)) });
        self.y += height;
    }

    /// Adds the answers of a question as a column of rows to tick; the ones in `picked` are ticked.
    fn answers(&mut self, options: &[String], picked: &[String], size: f32, palette: Palette) {
        for option in options {
            let inside = self.width - 2.0 * (CHAT_ASK_PAD + CHAT_ASK_OPTION_PAD) - CHAT_ASK_TICK - LEAD_GAP;
            let galley = self.laid_out(option, size, palette.foreground, inside, OPTION_ROWS);
            let height = (galley.size().y + 2.0 * CHAT_ASK_OPTION_GAP).max(CHAT_ASK_OPTION);
            let rect = Rect::from_min_size(pos2(CHAT_ASK_PAD, self.y), vec2(self.width - 2.0 * CHAT_ASK_PAD, height));
            self.items.push(Item::Answer { rect, galley, answer: option.clone(), ticked: picked.contains(option) });
            self.y += height + CHAT_ASK_OPTION_GAP;
        }
    }

    /// Adds the field for an answer of the user's own, and under it the line that says where the user is among
    /// the questions (`says`) with the button that sends what is chosen (`send`) at its right.
    fn own(&mut self, asked: &Asked, hint: &'static str, says: &str, send: &str, sizes: (f32, f32), palette: Palette) {
        let (small, tiny) = sizes;
        let wide = self.width - 2.0 * CHAT_ASK_PAD;
        let rect = Rect::from_min_size(pos2(CHAT_ASK_PAD, self.y), vec2(wide, CHAT_ASK_OPTION));
        self.items.push(Item::Own { rect, text: asked.own.clone(), hint, size: small });
        self.y += CHAT_ASK_OPTION + CHAT_ASK_GAP;
        let label = self.laid_out(send, small, palette.primary_text, wide, 1);
        let button = vec2(label.size().x + 2.0 * CHAT_ASK_PAD, CHAT_ASK_SEND);
        let rect = Rect::from_min_size(pos2(self.width - CHAT_ASK_PAD - button.x, self.y), button);
        if !says.is_empty() {
            let galley = self.laid_out(says, tiny, palette.quiet, wide - button.x - CHAT_ASK_GAP, 1);
            let at = pos2(CHAT_ASK_PAD, self.y + (CHAT_ASK_SEND - galley.size().y) / 2.0);
            self.items.push(Item::Words { at, galley, icon: None });
        }
        self.items.push(Item::Send { rect, galley: label, ready: asked.chosen().is_some() });
        self.y += CHAT_ASK_SEND + CHAT_ASK_GAP;
    }
}

/// The line beside the button that sends an answer: where the user is among the questions, and with answers to
/// tick that more than one can be. Nothing for a single question without answers to tick.
fn foot(at: usize, of: usize, options: bool) -> String {
    let several = if options { "Pick one or more" } else { "" };
    match (of > 1, options) {
        (true, true) => format!("{} of {of}  ·  {several}", at + 1),
        (true, false) => format!("{} of {of}", at + 1),
        (false, _) => several.to_owned(),
    }
}

/// Lays `asked` out as a sheet `width` wide.
// Human: top to bottom: the questions already answered with their answers (in the conversation), the question
// the user is at with its answers to tick, a field for an answer of the user's own, and the button that sends
// what is chosen ("Next" while questions follow). Suggestions are a title, the steps to tick, the same field
// and button. The cross stands at the top right while there is something to answer.
pub fn sheet(ctx: &egui::Context, asked: &Asked, width: f32, style: Style) -> Sheet {
    let Style { scale, history, palette } = style;
    let (large, small) = (MESSAGE_TEXT_SIZE * scale, (SIDEBAR_TEXT_SIZE - 1.0) * scale);
    let current = asked.current();
    let mut column = Column { ctx, items: Vec::new(), width, y: CHAT_ASK_PAD };
    // The first line leaves room for the cross.
    let mut spared = if current.is_some() { CHAT_ASK_CLOSE } else { 0.0 };
    if current.is_some() {
        let side = CHAT_ASK_CLOSE;
        column.items.push(Item::Close(Rect::from_min_size(pos2(width - CHAT_ASK_PAD - side + 5.0, CHAT_ASK_PAD - 3.0), Vec2::splat(side))));
    }
    if history && !asked.suggests {
        for (question, answer) in asked.questions.iter().zip(&asked.answers) {
            column.words(None, &question.text, small, palette.muted, std::mem::take(&mut spared));
            column.y += CHAT_NOTE_GAP - 2.0;
            column.words(Some((steps::PICKED_ICON, palette.muted)), answer, large, palette.foreground, 0.0);
            column.y += CHAT_ASK_GAP;
        }
    }
    match current {
        Some((_, question)) if asked.suggests => {
            column.words(Some((steps::SUGGESTED_ICON, theme::AGENT)), "Next steps", small, palette.muted, spared);
            column.y += CHAT_ASK_GAP;
            column.answers(&question.options, &asked.picked, small, palette);
            column.own(asked, "Or something else…", "Pick one or more", "Send", (small, small - 1.0), palette);
        }
        Some((at, question)) => {
            column.words(Some((steps::icon(crate::chat::asks::TOOL), theme::AGENT)), &question.text, large, palette.foreground, spared);
            column.y += CHAT_ASK_GAP;
            column.answers(&question.options, &asked.picked, small, palette);
            let says = foot(at, asked.questions.len(), !question.options.is_empty());
            let hint = if question.options.is_empty() { "Your answer…" } else { "Or your own answer…" };
            let send = if at + 1 < asked.questions.len() { "Next" } else { "Send" };
            column.own(asked, hint, &says, send, (small, small - 1.0), palette);
        }
        // Closed before every question was answered: the ones left stand there quietly.
        None if history => {
            for question in asked.questions.iter().skip(asked.answers.len()) {
                column.words(Some((steps::icon(crate::chat::asks::TOOL), palette.muted)), &question.text, large, palette.muted, 0.0);
                column.y += CHAT_ASK_GAP;
            }
        }
        None => {}
    }
    let height = (column.y - CHAT_ASK_GAP + CHAT_ASK_PAD).max(2.0 * CHAT_ASK_PAD);
    Sheet { items: column.items, size: vec2(width, height), question: current.map_or(0, |(at, _)| at), ready: asked.chosen().is_some() }
}

/// Paints `sheet` with its top-left corner at `origin`. While `live`, its answers can be ticked, its field
/// written in, and its button and its cross clicked; returns what the user did.
// Human: Return in the field sends, like the button, so an answer of one's own is typed and sent without the
// mouse.
// Agent: MAIN-THREAD; the buttons take their clicks through the `ui` they are painted in, clipped as it is.
pub fn paint(ui: &mut Ui, sheet: &Sheet, origin: Pos2, live: bool, palette: Palette) -> Option<Pick> {
    let by = origin.to_vec2();
    let mut pick = None;
    for (index, item) in sheet.items.iter().enumerate() {
        let pressed = |rect: Rect| live.then(|| ui.interact(rect, ui.scope_id().with(("ask", index)), Sense::click()));
        match item {
            Item::Words { at, galley, icon } => {
                if let Some((slot, name, color)) = icon {
                    icons::paint(ui.painter(), slot.translate(by), name, *color);
                }
                ui.painter().galley(*at + by, Arc::clone(galley), palette.foreground);
            }
            Item::Answer { rect, galley, answer, ticked } => {
                let rect = rect.translate(by);
                let response = pressed(rect);
                let hovered = response.as_ref().is_some_and(egui::Response::hovered);
                if hovered {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                // Human: a ticked answer keeps a quiet fill and a brighter edge, so what is chosen can be seen
                // at a glance without reading the boxes.
                let fill = match (*ticked, hovered) {
                    (_, true) => palette.hover,
                    (true, false) => palette.hover.gamma_multiply(0.45),
                    (false, false) => Color32::TRANSPARENT,
                };
                let edge = if *ticked { palette.muted } else { palette.outline };
                ui.painter().rect(rect, CHAT_ASK_OPTION_RADIUS, fill, Stroke::new(1.0, edge), StrokeKind::Inside);
                let tick = Rect::from_center_size(
                    pos2(rect.left() + CHAT_ASK_OPTION_PAD + CHAT_ASK_TICK / 2.0, rect.center().y),
                    Vec2::splat(CHAT_ASK_TICK),
                );
                if *ticked {
                    ui.painter().rect_filled(tick, 4.0, palette.primary);
                    icons::paint(ui.painter(), tick.shrink(2.5), steps::ANSWERED_ICON, palette.primary_text);
                } else {
                    ui.painter().rect_stroke(tick, 4.0, Stroke::new(1.0, palette.muted.gamma_multiply(0.6)), StrokeKind::Inside);
                }
                let at = pos2(tick.right() + LEAD_GAP, rect.center().y - galley.size().y / 2.0);
                ui.painter().galley(at, Arc::clone(galley), palette.foreground);
                if response.is_some_and(|response| response.clicked()) {
                    pick = Some(Pick::Toggle { question: sheet.question, answer: answer.clone() });
                }
            }
            Item::Own { rect, text, hint, size } => {
                let rect = rect.translate(by);
                ui.painter().rect_stroke(rect, CHAT_ASK_OPTION_RADIUS, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
                let inside = rect.shrink2(vec2(CHAT_ASK_OPTION_PAD, 0.0));
                let font = FontId::proportional(*size);
                if !live {
                    let (words, ink) = if text.is_empty() { (*hint, palette.quiet) } else { (text.as_str(), palette.foreground) };
                    ui.painter().text(inside.left_center(), egui::Align2::LEFT_CENTER, words, font, ink);
                    continue;
                }
                let mut written = text.clone();
                let field = egui::TextEdit::singleline(&mut written)
                    .id(egui::Id::unique("chat-ask-own"))
                    .frame(egui::Frame::NONE)
                    .font(font.clone())
                    .text_color(palette.foreground)
                    .hint_text(egui::RichText::new(*hint).font(font).color(palette.quiet))
                    .desired_width(inside.width());
                let row = ui.fonts_mut(|fonts| fonts.row_height(&FontId::proportional(*size)));
                let response = ui.put(Rect::from_center_size(inside.center(), vec2(inside.width(), row + 2.0)), field);
                if response.changed() {
                    pick = Some(Pick::Write { question: sheet.question, text: written.clone() });
                }
                // The words just typed count: Return right after them sends them too.
                if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) && (sheet.ready || !written.trim().is_empty()) {
                    pick = pick.or(Some(Pick::Send { question: sheet.question }));
                }
            }
            Item::Send { rect, galley, ready } => {
                let rect = rect.translate(by);
                let response = pressed(rect).filter(|_| *ready);
                let hovered = response.as_ref().is_some_and(egui::Response::hovered);
                if hovered {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                let (fill, ink) = if *ready { (palette.primary, palette.primary_text) } else { (palette.input, palette.muted) };
                ui.painter().rect_filled(rect, CHAT_ASK_OPTION_RADIUS, if hovered { fill.gamma_multiply(0.88) } else { fill });
                let at = rect.center() - galley.size() / 2.0;
                ui.painter().galley_with_override_text_color(at, Arc::clone(galley), ink);
                if response.is_some_and(|response| response.clicked()) {
                    pick = Some(Pick::Send { question: sheet.question });
                }
            }
            Item::Close(rect) => {
                let rect = rect.translate(by);
                let response = pressed(rect);
                let hovered = response.as_ref().is_some_and(egui::Response::hovered);
                if hovered {
                    ui.painter().rect_filled(rect, CHIP_RADIUS, palette.hover.gamma_multiply(0.5));
                }
                let ink = if hovered { palette.foreground } else { palette.muted };
                icons::paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(CHAT_NOTE_ICON - 2.0)), steps::CLOSE_ICON, ink);
                if response.is_some_and(|response| response.on_hover_text("Close without answering").clicked()) {
                    pick = Some(Pick::Dismiss);
                }
            }
        }
    }
    pick
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn asked() -> Asked {
        crate::chat::asks::parse(&json!({ "questions": [
            { "question": "Which mood should the page have?", "options": ["Warm and editorial", "Dark and technical", "Bright"] },
            { "question": "Phone or desktop?" },
        ] }))
        .unwrap()
    }

    /// The sheet of `asked` 400 wide, in the dialog's style or the conversation's.
    fn laid(asked: &Asked, history: bool) -> Sheet {
        let ctx = egui::Context::default();
        let mut sheet = None;
        let mut drawn = ctx.run_ui(egui::RawInput::default(), |ui| {
            sheet = Some(super::sheet(ui.ctx(), asked, 400.0, Style { scale: 1.0, history, palette: theme::palette(false) }));
        });
        drawn.textures_delta.clear();
        sheet.unwrap()
    }

    fn answers(sheet: &Sheet) -> Vec<&str> {
        sheet.items.iter().filter_map(|item| if let Item::Answer { answer, .. } = item { Some(answer.as_str()) } else { None }).collect()
    }

    #[test]
    fn a_sheet_holds_the_question_the_user_is_at_with_its_answers() {
        let mut asked = asked();
        let first = laid(&asked, false);
        assert_eq!((answers(&first), first.question), (vec!["Warm and editorial", "Dark and technical", "Bright"], 0));
        assert!(first.items.iter().any(|item| matches!(item, Item::Close(_))));
        // Everything stands inside the sheet, one under the other.
        let rects: Vec<Rect> = first.items.iter().filter_map(|item| if let Item::Answer { rect, .. } = item { Some(*rect) } else { None }).collect();
        assert!(rects.windows(2).all(|pair| pair[1].top() >= pair[0].bottom()));
        assert!(rects.iter().all(|rect| rect.left() >= CHAT_ASK_PAD && rect.right() <= 400.0 - CHAT_ASK_PAD && rect.bottom() < first.size.y));
        // Every question has a field for an answer of one's own and a button that sends what is chosen: it
        // says "Next" while questions follow, and is ready once something is ticked or written.
        let sends = |sheet: &Sheet| {
            sheet
                .items
                .iter()
                .find_map(|item| if let Item::Send { galley, ready, .. } = item { Some((galley.text().to_owned(), *ready)) } else { None })
        };
        let ticked = |sheet: &Sheet| sheet.items.iter().filter(|item| matches!(item, Item::Answer { ticked: true, .. })).count();
        assert_eq!((sends(&first), ticked(&first), first.ready), (Some(("Next".to_owned(), false)), 0, false));
        assert!(first.items.iter().any(|item| matches!(item, Item::Own { text, .. } if text.is_empty())));
        asked.toggle(0, "Bright");
        asked.toggle(0, "Warm and editorial");
        asked.write(0, "a little green");
        let chosen = laid(&asked, false);
        assert_eq!((sends(&chosen), ticked(&chosen), chosen.ready), (Some(("Next".to_owned(), true)), 2, true));
        assert!(chosen.items.iter().any(|item| matches!(item, Item::Own { text, .. } if text == "a little green")));
        // Ticking and writing change nothing about the sheet's size, so the dialog stands still.
        assert!((chosen.size - first.size).length() < f32::EPSILON);
        // The second question has no answers to tick: the sheet is shorter, and its button sends.
        asked.confirm(0);
        let second = laid(&asked, false);
        assert!(answers(&second).is_empty() && second.question == 1 && second.size.y < first.size.y);
        // The conversation also keeps what was answered, so its sheet is taller than the dialog's.
        assert!(laid(&asked, true).size.y > second.size.y);
        // Answered, nothing is left to pick and nothing to close.
        assert_eq!(sends(&second), Some(("Send".to_owned(), false)));
        asked.answer(1, "Phone");
        let done = laid(&asked, true);
        assert!(answers(&done).is_empty() && !done.items.iter().any(|item| matches!(item, Item::Close(_))));
        assert_eq!(
            (foot(0, 3, true), foot(0, 1, true), foot(1, 2, false), foot(0, 1, false)),
            ("1 of 3  ·  Pick one or more".to_owned(), "Pick one or more".to_owned(), "2 of 2".to_owned(), String::new())
        );
    }
}
