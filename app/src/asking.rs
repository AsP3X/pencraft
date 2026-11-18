//! How a question of the assistant's is drawn: its words, the answers to pick from as a column of buttons, and
//! a cross to close it. The conversation shows it as a card, and the folded sidebar as a dialog over the
//! message bar; both are this one sheet.
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
    CHAT_NOTE_GAP, CHAT_NOTE_ICON, CHIP_RADIUS, MESSAGE_TEXT_SIZE, Palette, SIDEBAR_TEXT_SIZE,
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
    /// Picked `answer` for the question at `question`.
    Answer {
        /// The question's place among the questions asked together.
        question: usize,
        /// The answer's words.
        answer: String,
    },
    /// Closed it without answering.
    Dismiss,
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
    /// An answer to pick.
    Answer { rect: Rect, galley: Arc<Galley>, answer: String },
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

    /// Adds the answers of a question as a column of buttons.
    fn answers(&mut self, options: &[String], size: f32, palette: Palette) {
        for option in options {
            let inside = self.width - 2.0 * (CHAT_ASK_PAD + CHAT_ASK_OPTION_PAD);
            let galley = self.laid_out(option, size, palette.foreground, inside, OPTION_ROWS);
            let height = (galley.size().y + 2.0 * CHAT_ASK_OPTION_GAP).max(CHAT_ASK_OPTION);
            let rect = Rect::from_min_size(pos2(CHAT_ASK_PAD, self.y), vec2(self.width - 2.0 * CHAT_ASK_PAD, height));
            self.items.push(Item::Answer { rect, galley, answer: option.clone() });
            self.y += height + CHAT_ASK_OPTION_GAP;
        }
        if !options.is_empty() {
            self.y += CHAT_ASK_GAP - CHAT_ASK_OPTION_GAP;
        }
    }
}

/// The line under a question's answers: where the user is among the questions, and that typing answers too.
fn foot(at: usize, of: usize, options: bool) -> String {
    let typing = if options { "or type your own answer below" } else { "Type your answer below" };
    if of > 1 {
        format!("{} of {of}  ·  {typing}", at + 1)
    } else if options {
        "Or type your own answer below".to_owned()
    } else {
        typing.to_owned()
    }
}

/// Lays `asked` out as a sheet `width` wide.
// Human: top to bottom: the questions already answered with their answers (in the conversation), the question
// the user is at with its answers to pick, and a quiet line saying that typing answers too. Suggestions are a
// title and the steps to pick. The cross stands at the top right while there is something to answer.
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
            column.answers(&question.options, small, palette);
        }
        Some((at, question)) => {
            column.words(Some((steps::icon(crate::chat::asks::TOOL), theme::AGENT)), &question.text, large, palette.foreground, spared);
            column.y += CHAT_ASK_GAP;
            column.answers(&question.options, small, palette);
            let says = foot(at, asked.questions.len(), !question.options.is_empty());
            column.words(None, &says, small - 1.0, palette.muted.gamma_multiply(0.8), 0.0);
            column.y += CHAT_ASK_GAP;
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
    Sheet { items: column.items, size: vec2(width, height), question: current.map_or(0, |(at, _)| at) }
}

/// Paints `sheet` with its top-left corner at `origin`. While `live`, its answers and its cross can be
/// clicked; returns what the user did.
// Agent: MAIN-THREAD; the buttons take their clicks through the `ui` they are painted in, clipped as it is.
pub fn paint(ui: &Ui, sheet: &Sheet, origin: Pos2, live: bool, palette: Palette) -> Option<Pick> {
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
            Item::Answer { rect, galley, answer } => {
                let rect = rect.translate(by);
                let response = pressed(rect);
                if response.as_ref().is_some_and(egui::Response::hovered) {
                    ui.painter().rect_filled(rect, CHAT_ASK_OPTION_RADIUS, palette.hover);
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                ui.painter().rect_stroke(rect, CHAT_ASK_OPTION_RADIUS, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
                let at = pos2(rect.left() + CHAT_ASK_OPTION_PAD, rect.center().y - galley.size().y / 2.0);
                ui.painter().galley(at, Arc::clone(galley), palette.foreground);
                if response.is_some_and(|response| response.clicked()) {
                    pick = Some(Pick::Answer { question: sheet.question, answer: answer.clone() });
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
        // The second question has no answers to pick: the sheet is shorter and says to type.
        asked.answer(0, "Bright");
        let second = laid(&asked, false);
        assert!(answers(&second).is_empty() && second.question == 1 && second.size.y < first.size.y);
        // The conversation also keeps what was answered, so its sheet is taller than the dialog's.
        assert!(laid(&asked, true).size.y > second.size.y);
        // Answered, nothing is left to pick and nothing to close.
        asked.answer(1, "Phone");
        let done = laid(&asked, true);
        assert!(answers(&done).is_empty() && !done.items.iter().any(|item| matches!(item, Item::Close(_))));
        assert_eq!(
            (foot(0, 3, true), foot(0, 1, true), foot(1, 2, false)),
            (
                "1 of 3  ·  or type your own answer below".to_owned(),
                "Or type your own answer below".to_owned(),
                "2 of 2  ·  Type your answer below".to_owned()
            )
        );
    }
}
