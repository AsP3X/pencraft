//! What the assistant asks the user, and the next steps it offers: the `ask_user` tool.
//!
//! The model calls the tool with one to three questions, each with a few answers to pick from, or with a few
//! suggestions for what to do next. The call answers at once; the chat shows the question (in the conversation,
//! and as a dialog over the message bar while the sidebar is folded away). The user ticks as many of the
//! answers as apply, writes one of their own beside or instead of them, and sends; what they chose goes to the
//! assistant as their next message.

use serde_json::Value;

/// The tool's name.
pub const TOOL: &str = "ask_user";
/// The most questions one call asks.
pub const MOST_QUESTIONS: usize = 3;
/// The most answers a question offers, and the most suggestions one call makes.
pub const MOST_OPTIONS: usize = 4;
/// The most letters of a question that are shown.
const MOST_QUESTION: usize = 240;
/// The most letters of an answer or a suggestion that are shown.
const MOST_OPTION: usize = 90;

/// What the model is told when its questions are on the screen.
pub const ASKED: &str = "The user now sees your questions with the answers to pick from; they may pick several and add words of their own. End your turn with one short sentence and change nothing meanwhile: their answer arrives as their next message.";
/// What the model is told when its suggestions are on the screen.
pub const SUGGESTED: &str = "The user now sees your suggestions to tick. Finish your reply as usual; if they pick some, or write something else, it arrives as their next message.";
/// What a caller outside the app's own chat is told.
pub const NO_CHAT: &str = "Only the assistant in Pencraft's own chat can ask the user this way. Ask in your reply instead.";

/// One question, with the answers offered for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    /// What is asked; empty for suggestions, which need no question.
    pub text: String,
    /// The answers to pick from. With none, the user types one.
    pub options: Vec<String>,
}

/// What one call of the tool put before the user, and how far the user has got with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    /// The questions, in the order they are asked; for suggestions one question without words whose answers
    /// are the suggestions.
    pub questions: Vec<Question>,
    /// These are next steps the assistant offers, not questions it needs answered.
    pub suggests: bool,
    /// The user's answers so far, one for each question from the first.
    pub answers: Vec<String>,
    /// The answers ticked for the question the user is at, not sent yet.
    pub picked: Vec<String>,
    /// What the user has written as an answer of their own to that question, not sent yet.
    pub own: String,
    /// Nothing more is taken: every question is answered, or the user closed it or wrote something else.
    pub closed: bool,
}

/// `text` on one line, cut after `most` letters.
fn tidy(text: &str, most: usize) -> String {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.chars().count() > most { line.chars().take(most - 1).chain(['…']).collect() } else { line }
}

/// The answers or suggestions in `value`: a list of strings, each tidied, none twice.
// Human: a model sometimes writes an option as an object with a label; that label is taken.
fn options(value: Option<&Value>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in value.and_then(Value::as_array).into_iter().flatten() {
        let Some(text) = item.as_str().or_else(|| item.get("label").and_then(Value::as_str)) else { continue };
        let text = tidy(text, MOST_OPTION);
        if !text.is_empty() && !out.contains(&text) && out.len() < MOST_OPTIONS {
            out.push(text);
        }
    }
    out
}

/// What a call of the tool with `input` asks.
///
/// # Errors
/// What is wrong with the input, said so that the model can put it right.
pub fn parse(input: &Value) -> Result<Asked, String> {
    let questions: Vec<Question> = input
        .get("questions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            // A question may also be written as its words alone.
            let text = tidy(item.get("question").unwrap_or(item).as_str()?, MOST_QUESTION);
            (!text.is_empty()).then(|| Question { text, options: options(item.get("options")) })
        })
        .take(MOST_QUESTIONS)
        .collect();
    let suggestions = options(input.get("suggestions"));
    let asked = |questions, suggests| Asked { questions, suggests, answers: Vec::new(), picked: Vec::new(), own: String::new(), closed: false };
    match (questions.is_empty(), suggestions.is_empty()) {
        (false, true) => Ok(asked(questions, false)),
        (true, false) => Ok(asked(vec![Question { text: String::new(), options: suggestions }], true)),
        (false, false) => {
            Err("Give `questions` or `suggestions`, not both: a question waits for its answer before anything else is offered.".to_owned())
        }
        (true, true) => Err("Give `questions` (each with a `question` and a few `options`) or `suggestions` (a few short next steps).".to_owned()),
    }
}

impl Asked {
    /// The question the user is at, with its place among the questions; `None` when nothing more is taken.
    #[must_use]
    pub fn current(&self) -> Option<(usize, &Question)> {
        if self.closed {
            return None;
        }
        self.questions.get(self.answers.len()).map(|question| (self.answers.len(), question))
    }

    /// Takes `answer` for the question at `at`. With the last answer it gives the user's message: as the panel
    /// shows it, and as the assistant reads it.
    // Human: a second click on an answer that was already taken (the dialog and the conversation both show the
    // question) is for a question the user is past, and does nothing.
    pub fn answer(&mut self, at: usize, answer: &str) -> Option<(String, String)> {
        if self.closed || at != self.answers.len() || at >= self.questions.len() {
            return None;
        }
        self.answers.push(answer.to_owned());
        self.picked.clear();
        self.own.clear();
        if self.answers.len() < self.questions.len() {
            return None;
        }
        self.closed = true;
        let shown = self.answers.join("\n");
        Some(if self.suggests { (shown.clone(), shown) } else { (shown, self.told()) })
    }

    /// Ticks the answer `option` of the question at `at`, or takes its tick away again.
    pub fn toggle(&mut self, at: usize, option: &str) {
        let Some((_, question)) = self.current().filter(|(current, _)| *current == at) else { return };
        if !question.options.iter().any(|offered| offered == option) {
            return;
        }
        match self.picked.iter().position(|picked| picked == option) {
            Some(place) => {
                self.picked.remove(place);
            }
            None => self.picked.push(option.to_owned()),
        }
    }

    /// Takes `text` as what the user has written so far as their own answer to the question at `at`.
    pub fn write(&mut self, at: usize, text: &str) {
        if self.current().is_some_and(|(current, _)| current == at) {
            text.clone_into(&mut self.own);
        }
    }

    /// The answer to the question the user is at as it stands: what is ticked, in the order it is offered,
    /// and then what the user wrote. `None` while there is neither.
    // Human: several answers to a question read as one line ("Warm, Dark, with a bit of green"); several next
    // steps are each a request of their own, so each has its line.
    #[must_use]
    pub fn chosen(&self) -> Option<String> {
        let (_, question) = self.current()?;
        let ticked = question.options.iter().filter(|option| self.picked.contains(option)).map(String::as_str);
        let parts: Vec<&str> = ticked.chain(Some(self.own.trim()).filter(|own| !own.is_empty())).collect();
        (!parts.is_empty()).then(|| parts.join(if self.suggests { "\n" } else { ", " }))
    }

    /// Sends what is chosen for the question at `at` as its answer ([`Asked::answer`]). Nothing happens while
    /// nothing is chosen.
    pub fn confirm(&mut self, at: usize) -> Option<(String, String)> {
        let answer = self.chosen()?;
        self.answer(at, &answer)
    }

    /// The questions answered so far with their answers, as the assistant reads them.
    #[must_use]
    pub fn told(&self) -> String {
        self.questions
            .iter()
            .zip(&self.answers)
            .map(|(question, answer)| format!("{}\nAnswer: {answer}", question.text))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_call_asks_questions_or_makes_suggestions() {
        let asked = parse(&json!({ "questions": [
            { "question": " Which mood\n should it have? ", "options": ["Warm", " Warm ", "Dark", { "label": "Playful" }, "", 4] },
            "And for which screen?",
            { "options": ["no question"] },
        ] }))
        .unwrap();
        assert!(!asked.suggests && !asked.closed);
        assert_eq!(
            asked.questions[0],
            Question { text: "Which mood should it have?".to_owned(), options: vec!["Warm".into(), "Dark".into(), "Playful".into()] }
        );
        assert_eq!(asked.questions[1], Question { text: "And for which screen?".to_owned(), options: Vec::new() });
        assert_eq!(asked.questions.len(), 2);
        // Suggestions are one question without words; no more than four are taken.
        let offered = parse(&json!({ "suggestions": ["a", "b", "c", "d", "e"] })).unwrap();
        assert!(offered.suggests && offered.questions.len() == 1 && offered.questions[0].text.is_empty());
        assert_eq!(offered.questions[0].options, ["a", "b", "c", "d"]);
        // Neither, or both, is said so.
        assert!(parse(&json!({})).is_err_and(|why| why.contains("`questions`")));
        assert!(parse(&json!({ "questions": ["a?"], "suggestions": ["b"] })).is_err_and(|why| why.contains("not both")));
        // Long words are cut.
        let long = parse(&json!({ "questions": ["x".repeat(500)], })).unwrap();
        assert!(long.questions[0].text.chars().count() == MOST_QUESTION && long.questions[0].text.ends_with('…'));
    }

    #[test]
    fn answers_are_taken_in_turn_and_the_last_one_makes_the_message() {
        let mut asked = parse(&json!({ "questions": [{ "question": "Mood?", "options": ["Warm", "Dark"] }, { "question": "Screen?" }] })).unwrap();
        assert_eq!(asked.current().map(|(at, question)| (at, question.text.as_str())), Some((0, "Mood?")));
        // An answer for another question than the one the user is at is not taken.
        assert_eq!((asked.answer(1, "Phone"), asked.answer(0, "Dark")), (None, None));
        assert_eq!(asked.answer(0, "Warm"), None);
        assert_eq!(asked.current().map(|(at, _)| at), Some(1));
        assert_eq!(asked.answer(1, "Phone"), Some(("Dark\nPhone".to_owned(), "Mood?\nAnswer: Dark\nScreen?\nAnswer: Phone".to_owned())));
        assert!(asked.closed && asked.current().is_none() && asked.answer(1, "Tablet").is_none());
        // Several answers and words of the user's own are one answer, in the order the answers are offered.
        let mut several = parse(&json!({ "questions": [{ "question": "Mood?", "options": ["Warm", "Dark", "Bright"] }, "Screen?"] })).unwrap();
        assert_eq!((several.chosen(), several.confirm(0)), (None, None));
        several.toggle(0, "Bright");
        several.toggle(0, "Warm");
        several.toggle(0, "Dark");
        several.toggle(0, "Dark");
        several.toggle(0, "Not offered");
        several.toggle(1, "Warm");
        several.write(1, "ignored: not the question the user is at");
        several.write(0, "  with a bit of green ");
        assert_eq!(several.chosen().as_deref(), Some("Warm, Bright, with a bit of green"));
        assert_eq!(several.confirm(0), None);
        // The next question starts with nothing ticked and nothing written; words alone answer it.
        assert!(several.picked.is_empty() && several.own.is_empty() && several.current().is_some_and(|(at, _)| at == 1));
        several.write(1, "Phone");
        let (shown, told) = several.confirm(1).unwrap();
        assert_eq!(shown, "Warm, Bright, with a bit of green\nPhone");
        assert!(told.ends_with("Screen?\nAnswer: Phone") && several.closed);
        // Several next steps are each a line of the message.
        let mut steps = parse(&json!({ "suggestions": ["Add a dark version", "Export it"] })).unwrap();
        steps.toggle(0, "Export it");
        steps.toggle(0, "Add a dark version");
        steps.write(0, "and make the logo larger");
        assert_eq!(steps.confirm(0).map(|(shown, _)| shown).as_deref(), Some("Add a dark version\nExport it\nand make the logo larger"));
        // A suggestion the user picks is their message as it stands.
        let mut offered = parse(&json!({ "suggestions": ["Add a dark version", "Export it"] })).unwrap();
        assert_eq!(offered.answer(0, "Export it"), Some(("Export it".to_owned(), "Export it".to_owned())));
    }
}
