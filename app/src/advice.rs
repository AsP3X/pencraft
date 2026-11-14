//! What the app tells an AI model about the design it just changed: mistakes that are easy to make when
//! writing layers as data and plain to see on the canvas, so the model can put them right in its next step.

use pen_doc::color::Rgba;
use pen_doc::model::{Kind, Paint};
use pen_doc::scene::SceneNode;
use pen_doc::value::Flag;
use pen_doc::vars::Vars;

use crate::editor::Editor;

/// How far two colours may differ in a channel and still look the same.
const SAME_COLOUR: u8 = 3;
/// The least opacity (of 255) at which a fill covers what is behind it.
const COVERS: u8 = 128;
/// How many layers one piece of advice names.
const MOST_NAMED: usize = 5;
/// How many pieces of advice one answer carries.
const MOST_ADVICE: usize = 8;

/// What a layer is filled with, as far as telling it from its background goes.
enum Fill {
    /// Nothing, or something see-through: what is behind shows.
    Clear,
    /// One colour that covers.
    Colour(Rgba),
    /// A gradient or a picture: no single colour.
    Mixed,
}

/// The fill on top of a layer's fills.
fn fill(scene: &SceneNode, vars: Vars<'_>) -> Fill {
    let Some(top) = scene.node.fill.as_ref().and_then(|paints| paints.as_slice().last()) else { return Fill::Clear };
    let colour = match top {
        Paint::Solid(text) => vars.color(text, &scene.theme),
        Paint::Object(object) if object.get("type").and_then(|t| t.as_str()) == Some("color") => {
            object.get("color").and_then(|c| c.as_str()).and_then(|text| vars.color(text, &scene.theme))
        }
        Paint::Object(_) => return Fill::Mixed,
    };
    match colour {
        Some(colour) if colour.a >= COVERS => Fill::Colour(colour),
        _ => Fill::Clear,
    }
}

fn same(a: Rgba, b: Rgba) -> bool {
    a.r.abs_diff(b.r) <= SAME_COLOUR && a.g.abs_diff(b.g) <= SAME_COLOUR && a.b.abs_diff(b.b) <= SAME_COLOUR
}

fn hex(c: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// A layer that can't be told from what is behind it.
struct Found {
    /// The id and name of the layer behind, and its colour.
    behind: (String, String, Rgba),
    /// The layer's name and id.
    layer: String,
    /// Whether the layer is text (it can't be read) or a shape or frame (it doesn't stand out).
    text: bool,
}

/// Walks `scene`, collecting the layers in the changed part that have the colour of what is behind them.
// Human: what is "behind" a layer is the nearest frame around it with a covering fill; a frame filled with a
// gradient or a picture has no single colour, and nothing is said about what lies on it.
fn look<'a>(scene: &'a SceneNode, behind: Option<(&'a SceneNode, Rgba)>, changed: bool, touched: &[String], vars: Vars<'_>, found: &mut Vec<Found>) {
    if matches!(&scene.node.enabled, Some(Flag::Value(false))) {
        return;
    }
    let changed = changed || touched.contains(&scene.id);
    let own = fill(scene, vars);
    if let (true, Fill::Colour(colour), Some((back, back_colour))) = (changed, &own, behind)
        && same(*colour, back_colour)
        && !matches!(scene.node.kind, Kind::Icon | Kind::Path | Kind::Line)
    {
        found.push(Found {
            behind: (back.id.clone(), back.node.display_name(), back_colour),
            layer: format!("\"{}\" ({})", scene.node.display_name(), scene.id),
            text: scene.node.kind == Kind::Text,
        });
    }
    let under = match (&scene.node.kind, own) {
        // A layer made of a material has no colour of its own to compare with.
        _ if scene.node.material.is_some() => None,
        (Kind::Frame, Fill::Colour(colour)) => Some((scene, colour)),
        (Kind::Frame, Fill::Mixed) => None,
        _ => behind,
    };
    for child in &scene.children {
        look(child, under, changed, touched, vars, found);
    }
}

/// Advice on the layers `touched` (and everything in them) after a change, one sentence each; empty when
/// there is nothing to say.
// Human: a part shown on a board of its own colour (a white card on a white board) is the mistake this is
// for: it looks fine as data and is nearly invisible on the canvas. The advice names the layer behind first,
// because changing that one layer is usually the cure.
#[must_use]
pub fn advice(editor: &Editor, touched: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for top in &editor.scene {
        look(top, None, false, touched, Vars::new(&editor.doc), &mut found);
    }
    let mut out: Vec<String> = Vec::new();
    for text in [false, true] {
        let mut backs: Vec<&(String, String, Rgba)> = Vec::new();
        for item in found.iter().filter(|f| f.text == text) {
            if !backs.iter().any(|back| back.0 == item.behind.0) {
                backs.push(&item.behind);
            }
        }
        for back in backs {
            let layers: Vec<&str> = found.iter().filter(|f| f.text == text && f.behind.0 == back.0).map(|f| f.layer.as_str()).collect();
            let named = layers.iter().take(MOST_NAMED).copied().collect::<Vec<_>>().join(", ");
            let more = if layers.len() > MOST_NAMED { format!(" and {} more", layers.len() - MOST_NAMED) } else { String::new() };
            out.push(if text {
                format!("Text in the colour of its background ({}) can't be read: {named}{more}, on \"{}\" ({}). Change the text's fill or the background.", hex(back.2), back.1, back.0)
            } else {
                format!(
                    "{} layer(s) have the same fill ({}) as \"{}\" ({}) behind them, so they don't stand out from it: {named}{more}. Give \"{}\" a different background (for a board that only presents parts, a grey or a tint the parts don't use), or give those layers a fill that differs.",
                    layers.len(),
                    hex(back.2),
                    back.1,
                    back.0,
                    back.1
                )
            });
        }
    }
    out.truncate(MOST_ADVICE);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor(json: &str) -> Editor {
        let mut editor = Editor::new(pen_doc::read(json).unwrap(), None, 1);
        editor.refresh(&|_| (10.0, 10.0));
        editor
    }

    #[test]
    fn a_part_in_the_colour_of_its_board_is_pointed_out_with_the_board_first() {
        let editor = editor(
            r##"{"version":"2.22","themes":{"Mode":["Light","Dark"]},
              "variables":{"bg":{"type":"color","value":[{"value":"#fafafa"},{"value":"#18181b","theme":{"Mode":"Dark"}}]}},
              "children":[
              {"type":"frame","id":"board","name":"Parts","x":0,"y":0,"fill":"$bg","layout":"vertical","children":[
                {"type":"frame","id":"row","children":[
                  {"type":"frame","id":"card","name":"Card","width":50,"height":50,"fill":"#fbfbfb","stroke":"#e4e4e7","strokeWidth":1},
                  {"type":"frame","id":"tinted","name":"Tinted","width":50,"height":50,"fill":"#3b82f6","children":[
                    {"type":"text","id":"hidden","name":"Hidden","content":"x","fill":"#3b82f6"},
                    {"type":"text","id":"plain","content":"x","fill":"#ffffff"}]},
                  {"type":"frame","id":"glass","name":"Glass","width":50,"height":50,"fill":"#fafafa20"},
                  {"type":"frame","id":"off","name":"Off","enabled":false,"width":50,"height":50,"fill":"#fafafa"},
                  {"type":"frame","id":"dark","name":"Dark","theme":{"Mode":"Dark"},"width":50,"height":50,"fill":"$bg"}]}]},
              {"type":"frame","id":"alone","x":900,"y":0,"width":50,"height":50,"fill":"#fafafa"}]}"##,
        );
        let said = advice(&editor, &["board".to_owned(), "alone".to_owned()]);
        assert_eq!(said.len(), 2, "{said:?}");
        // The card on the board of (all but) its own colour; not the see-through, hidden, dark-themed or top-level ones.
        assert!(
            said[0].starts_with("1 layer(s) have the same fill (#fafafa) as \"Parts\" (board)") && said[0].contains("\"Card\" (card)"),
            "{}",
            said[0]
        );
        assert!(!said[0].contains("Glass") && !said[0].contains("Off") && !said[0].contains("Dark"));
        assert!(said[1].contains("can't be read") && said[1].contains("\"Hidden\" (hidden)") && !said[1].contains("plain"), "{}", said[1]);
        // Only what the change touched is looked at.
        assert_eq!(advice(&editor, &["alone".to_owned()]), Vec::<String>::new());
        assert_eq!(advice(&editor, &["card".to_owned()]).len(), 1);
    }
}
