//! A second look at a design by a model that didn't make it.
//!
//! A model asked whether its own design is good says yes. So when the assistant has finished a design, a
//! picture of what it changed goes to a reviewer: the same engine in a conversation of its own, told to judge
//! like a strict art director. What the reviewer wants changed goes back to the assistant, which works it in.

use std::process::Child;
use std::sync::Mutex;
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use serde_json::{Value, json};

use super::code::Flavor;
use super::{Engine, Wire};
use crate::link::Call;

/// How many times a design is reviewed and sent back at most.
// Source: research on revising from pictures finds most of the gain in the first two rounds and worse results
// after three (ReLook, arXiv 2510.11498); Anthropic's harness for long-running apps reviews with a separate,
// strict evaluator ("Harness design for long-running application development"). The Impeccable skill sets
// the same ceiling of two rounds, and its modes are why the brief asks first what kind of thing is judged.
pub const ROUNDS: usize = 2;
/// The name the review has among the tools: what the app is asked for the picture by, and the chat's note.
pub const TOOL: &str = "review_design";
/// How long the app may take to draw the picture.
const PATIENCE: Duration = Duration::from_secs(60);
/// The name beside the cursor while the picture is taken.
const CURSOR_NAME: &str = "Reviewer";

/// What the reviewer is told about its job.
const BRIEF: &str = "You are an art director reviewing a design that another designer made in a design tool. You get a picture of the whole design, then pictures of its largest parts enlarged, what the tool counted in its layers, and what the user asked for. You did not make it and gain nothing from liking it. Most work you are shown is competent and generic; say so when it is. Judge only what the picture shows.

First say to yourself what kind of thing it is, because that decides what good means here. Something that has to win a person over (a landing page, a poster, a cover): the design itself is judged, and its look may own the whole. A screen someone works in (an app, a dashboard, settings, a table, a form): it has to read at once as a tool of its kind, with standard navigation and controls where people expect them; its look shows only in type, palette, density and one signature move, and a working screen dressed up as a poster, a newspaper or a terminal is a fault, not an idea. A page someone reads (an article, documentation): the frame may carry the look, the reading column stays calm. Components or a standard kit (the user asked for a button, a dialog, a set of controls, or named a library, a product or a style to follow, or asked for standard or plain components): the named reference is the specification, and looking like that library is the job done, not a default to mark down. A logo (a mark, a wordmark, a monogram, an emblem, a crest, a mascot, an app icon), simple or richly detailed: judged as an identity designer judges one, by the paragraph on logos below instead of the four scores' usual meaning.

Score each of these from 1 to 10. The first two weigh double.
1. Design quality: is it one whole with a mood of its own? 4: parts placed next to each other, it could be any template. 6: tidy and consistent, but without an idea. 8: one clear idea carried through type, colour and layout. For a working screen or a reading page, 8 is: as clean as the best tool or document of its kind, and recognisably this product's through its type, palette and one move.
2. Originality: was each choice made for this subject, or could another product use this design unchanged? Defaults are a violet or indigo gradient, a row of three equal cards with icon, title and text, a centred hero with two buttons, a row of big numbers over small labels, one typeface in one weight, a small spaced-capitals label over a headline, sections numbered 01 02 03, glows; and the looks models land on next: cream with a serif headline and terracotta, near black with one neon accent, newspaper hairlines with an italic serif and small mono labels, flat loud colour with black boxes and block shadows. 4: several defaults. 6: none, but nothing one would remember an hour later. 8: someone clearly decided things. On a working screen, familiar structure is not a default to mark down; a grey screen with one accent colour is. For components or a standard kit this score is not originality but fidelity: does it match the reference the user named (its face, its neutral palette, its sizes, corners and lines), is every component, variant and state that was asked for there, and is the same thing built the same way everywhere? 4: it only resembles the reference, or asked-for pieces are missing. 6: complete, with sizes or states that drift from piece to piece. 8: someone who knows the reference would take it for the real thing. Never mark such a design down for its familiar face, its grey palette or its hairline with a soft shadow on a floating surface.
3. Craft: one clear order of what is read first, second and third; shared edges; spacing that groups, with more room above a heading than below; text that can be read; shadows that read as depth; nothing cut off, overlapping, cramped, floating or garbled. Check the enlarged parts as a print proof is checked: figures in a column end on one edge; the same kind of thing has the same size, weight and colour everywhere; a colour means one thing only; a label doesn't touch what it labels; panels side by side share top and bottom edges; no large area is left empty by accident; small text and controls that are switched off can still be seen clearly against their ground; nothing runs over the edge of the page that the page doesn't cut off. 6: two or three such slips. 8: you find none.
4. Function: it is what the user asked for, what a visitor needs is found at once, and there is one main action, not several of equal weight or more than about four choices side by side.

For a logo the four scores mean this. A logo may be simple (a flat mark of a few shapes) or detailed (a badge with text round its rim, a crest, a seal, a mascot, an engraving, a dimensional mark): first name its rendering style, and judge it by that style's rules; richness is not a fault and neither is restraint, the wrong one for the brief is. With its picture comes a small proof: the layers drawn 24 pixels on their shorter side and enlarged pixel by pixel. 1, the idea and the style: can the idea be said in one sentence and is it seen without being told; is the style the one this brief calls for, carried through every part? 4: a literal picture of the subject put together from rectangles, or parts held together by a shield, a circle or a ribbon that nothing called for, or the same flat mark any brief would get when the user asked for something rich. 6: tidy, with a stock symbol or a plain initial, or a style only half kept. 8: one idea, in one style kept throughout. 2, whose it is: could a competitor use it unchanged, is it the first picture anyone would draw for the subject, does it look like a known company's mark? 3, craft: built, not assembled: one centre line, exact symmetry, curves that are arcs meeting edges without a kink, equal gaps, nothing colliding or crossing an outline, holes that show the ground. In a simple mark one line weight and nothing that fills in or vanishes in the small proof. In a detailed logo at most three line weights with clear steps, two or three tones per colour, all shading and every gradient from one light, one thing read first and a place to rest, a main picture that fills its field, rim text upright on its arc, centred and not colliding, and beside it a small version that is one clear shape in the small proof (the detailed logo itself only has to keep its silhouette there). 4, the name and the request: a face chosen to answer the mark and the style (not a typewriter or slab face only for tradition), spaced and placed with intent, every text readable at the logo's full size, every piece the user named there, and no part that neither the style nor the brief called for (a board behind it, stars, a year nobody gave). When the idea or the style scores under 7, say REVISE and name in one sentence a different idea or the style to take; do not ask for the weak one to be polished. When the user asked for detail or a rich style and got a flat mark, that is broken. Never ask a simple mark for decoration, and never ask a detailed logo to be flattened unless the brief asked for simple.

Then check the design against the request, whatever kind of thing it is: go through what the user named, piece by piece. A named piece that is missing, and a piece nobody asked for that changes what the user gets, each count as broken. When the user asked for a mock of a phone or tablet app or named a device (an iPhone, an iPad, an Android phone), the device is a named piece: a screen shown as a bare rectangle, without the device's body and corners around it, its status bar, its Dynamic Island or camera and its home indicator, is broken, unless the user asked for the screen alone.

What the user asked for outright is never a fault, even when it is a cliché. Do not ask for more sections, photographs, motion or anything else the user did not ask for, and do not ask a working screen for more decoration.

Answer in plain text without Markdown. The first line is the one word PASS when design quality, originality (or fidelity) and function are 7 or more, craft is 8 or more and nothing is broken, otherwise the one word REVISE. When only craft holds a design back, list only the slips, up to eight of them, each with where it is and what to set, and leave the idea alone. The second line gives the four scores. After REVISE, list at most four changes, the most important first, one sentence each, naming the part and what to do instead, concretely (\"set the plan names in 28 px and drop the icons\", not \"improve the hierarchy\"). A missing piece the user named comes first. If the direction itself is generic, say which direction to take instead; for components or a standard kit never ask for a new direction.";

/// What the reviewer is told for the last look: the idea is settled, only the execution is read, as a proof is.
// Human: two reviews settle what the design is; what is left then are small slips, which a review that also
// weighs the idea names too few of. The last look is for those alone.
const PROOF: &str = "You are proofreading a design another designer made in a design tool, the way a print proof is read before it goes to press. You get a picture of the whole design, then pictures of its parts enlarged, what the tool counted in its layers, and what the user asked for. The idea, the palette, the typefaces and the layout are settled: do not comment on them and do not ask for anything to be added, removed or redesigned.

Go through every enlarged part, edge by edge and line by line, and look for slips of execution only: text that overlaps, touches or is cut off; a missing space between a figure and a word; figures in a column that do not end on one edge; a thing that sticks a few units past the edge its neighbours keep; the same kind of thing set in different sizes, weights or colours; rows or panels side by side with different heights or with an empty pocket; a label touching what it labels; a button or chip whose label has hardly any room above and below it, or that sits right against a line of text; initials or a label sitting off-centre in its circle, badge or button; small or quiet text too faint for its ground; a control that is switched off and can hardly be seen; a list whose last row or rule sits right against the bar or footer below it; a number that contradicts another (a bar that does not match its caption, a sum that is wrong).

Answer in plain text without Markdown. The first line is the one word PASS when you find nothing, otherwise the one word REVISE. After REVISE, list up to ten slips, the most visible first, one sentence each: where it is (name the part and the text) and exactly what to set instead.";

/// A picture of what the assistant designed since the last review, with what the app counted in it, as the
/// content of a message; `chat` is the chat whose design it is. `None` when it changed nothing worth a review.
// Agent: BLOCKS until the app's main thread has drawn the picture.
#[must_use]
pub fn picture(tools: &Sender<Call>, wake: &(dyn Fn() + Send + Sync), chat: u64, again: bool) -> Option<Vec<Value>> {
    let (reply, answered) = channel();
    tools.send(Call { tool: TOOL.to_owned(), arguments: json!({ "again": again }), client: CURSOR_NAME.to_owned(), chat: Some(chat), reply }).ok()?;
    wake();
    let content = answered.recv_timeout(PATIENCE).ok()?.ok()?;
    let blocks: Vec<Value> = content
        .iter()
        .map(|item| match item["type"].as_str() {
            Some("image") => json!({ "type": "image", "source": { "type": "base64", "media_type": item["mimeType"], "data": item["data"] } }),
            _ => json!({ "type": "text", "text": item["text"] }),
        })
        .collect();
    blocks.iter().any(|block| block["type"] == "image").then_some(blocks)
}

/// What the reviewer wants changed, from its answer; `None` when the design passes.
fn changes(answer: &str) -> Option<String> {
    // Human: the verdict is the first line that says PASS or REVISE; a model sometimes puts a word before it.
    let mut lines = answer.trim().lines();
    let verdict = lines.by_ref().map(str::to_uppercase).find(|line| line.contains("REVISE") || line.contains("PASS"))?;
    let rest = lines.collect::<Vec<_>>().join(
        "
",
    );
    // An answer that is neither is no review; the design is left as it is.
    (verdict.contains("REVISE") && !rest.trim().is_empty()).then(|| rest.trim().to_owned())
}

/// Asks the reviewer about the design in `picture`, made for the request `asked`. Gives what it wants changed,
/// or `None` when the design passes.
///
/// # Errors
/// When the reviewer couldn't be asked.
// Agent: BLOCKS for one model answer; `child` holds the Claude Code program meanwhile so the chat can end it.
pub fn judge(engine: &Engine, asked: &str, mut picture: Vec<Value>, proof: bool, child: &Mutex<Option<Child>>) -> Result<Option<String>, String> {
    let brief = if proof { PROOF } else { BRIEF };
    picture.push(json!({ "type": "text", "text": format!("The user asked for:\n{asked}") }));
    let answer = match engine {
        Engine::Code { flavor: Flavor::Claude, program, model } => super::code::once(program, model, brief, &picture, child)?,
        Engine::Api { wire: Wire::Messages, endpoint, model, key, .. } => super::api::once(endpoint, key, model, brief, &picture)?,
        // No reviewer speaks through the other engines; the design goes to the user as it is.
        _ => return Ok(None),
    };
    log::info!("review: {}", answer.trim());
    Ok(changes(&answer))
}

/// Whether a reviewer can be asked through `engine`: Claude Code and the Messages API take the one question
/// with a picture that a review is.
#[must_use]
pub fn can_judge(engine: &Engine) -> bool {
    matches!(engine, Engine::Code { flavor: Flavor::Claude, .. } | Engine::Api { wire: Wire::Messages, .. })
}

/// The message that brings the reviewer's `changes` to the assistant.
#[must_use]
pub fn follow_up(changes: &str) -> String {
    format!(
        "<pencraft_review>\nBefore your design goes to the user, a reviewer who did not make it looked at a picture of it. This is what the reviewer wants changed:\n{changes}\n</pencraft_review>\nMake these changes now. The reviewer is strict and usually right about the design, but the user's own words come first where the two differ. Check the result with a screenshot. Then tell the user in two or three plain sentences what the design is now, without mentioning the review."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_review_passes_or_says_what_to_change() {
        assert_eq!(changes("PASS\n8 7 8 9"), None);
        assert_eq!(changes("  pass"), None);
        assert_eq!(changes("REVISE\n6 5 7 8\n1. Drop the icons."), Some("6 5 7 8\n1. Drop the icons.".to_owned()));
        // Neither word, or nothing to change: the design stays.
        assert_eq!(changes("I like it."), None);
        assert_eq!(
            changes(
                "Here is my review.

REVISE
Drop the icons."
            ),
            Some("Drop the icons.".to_owned())
        );
        assert_eq!(changes("REVISE"), None);
        assert!(follow_up("Drop the icons.").contains("Drop the icons.\n</pencraft_review>"));
    }

    #[test]
    fn the_picture_comes_from_the_app_as_message_content() {
        let (tools, calls) = channel::<Call>();
        let app = std::thread::spawn(move || {
            let call = calls.recv().unwrap();
            assert_eq!((call.tool.as_str(), call.chat), (TOOL, Some(4)));
            call.reply
                .send(Ok(vec![json!({ "type": "image", "data": "QQ==", "mimeType": "image/png" }), json!({ "type": "text", "text": "review" })]))
                .unwrap();
            // A run that changed nothing gets no picture, and so no review.
            calls.recv().unwrap().reply.send(Ok(Vec::new())).unwrap();
        });
        let content = picture(&tools, &|| {}, 4, false).unwrap();
        assert_eq!(content[0]["source"]["data"], "QQ==");
        assert_eq!(content[1]["text"], "review");
        assert_eq!(picture(&tools, &|| {}, 4, true), None);
        app.join().unwrap();
    }
}
