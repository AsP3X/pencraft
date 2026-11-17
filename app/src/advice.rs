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
/// The most texts a container around a screen has beside it: a caption and a line under it.
const MOST_CAPTIONS: usize = 2;
/// The least room a container leaves around the one frame in it on every side.
const ROOM: f32 = 8.0;
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
    let mut under = under;
    for child in &scene.children {
        look(child, under, changed, touched, vars, found);
        // Human: a layer that lies over the whole frame (a scrim under a dialog, a picture) is what the
        // layers after it are seen against, and it has no single colour to compare with.
        if veils(child, scene, vars) {
            under = None;
        }
    }
}

/// Whether `layer` lies over all of `frame` with a fill of its own, however see-through.
fn veils(layer: &SceneNode, frame: &SceneNode, vars: Vars<'_>) -> bool {
    let shown = !matches!(&layer.node.enabled, Some(Flag::Value(false)));
    let painted = layer.node.fill.as_ref().and_then(|paints| paints.as_slice().last()).is_some_and(|top| match top {
        Paint::Solid(text) => vars.color(text, &layer.theme).is_some_and(|colour| colour.a > 0),
        Paint::Object(_) => true,
    });
    shown && painted && layer.local.w >= frame.local.w - 1.0 && layer.local.h >= frame.local.h - 1.0
}

/// Collects the icon layers in the changed part of `scene` that name no bundled icon, each as its label, the
/// name it asked for and the nearest bundled one.
// Human: such a layer is drawn as a small rounded box, which reads as data that is fine and looks like a
// mistake on the canvas.
fn unknown_icons(scene: &SceneNode, changed: bool, touched: &[String], out: &mut Vec<String>) {
    if matches!(&scene.node.enabled, Some(Flag::Value(false))) {
        return;
    }
    let changed = changed || touched.contains(&scene.id);
    if changed && matches!(scene.node.kind, Kind::Icon | Kind::IconFont) {
        let old = |key: &str| scene.node.extra.get(key).and_then(|value| value.as_str());
        let library = scene.node.library.as_deref().or_else(|| old("iconFontFamily")).unwrap_or("lucide");
        let name = scene.node.icon.as_deref().or_else(|| old("iconFontName")).unwrap_or_default();
        if crate::icons::outline(library, name).is_none() {
            let instead = crate::icons::nearest(name).map_or_else(String::new, |near| format!(" (nearest: \"{near}\")"));
            out.push(format!("\"{}\" ({}) asks for \"{name}\" from {library}{instead}", scene.node.display_name(), scene.id));
        }
    }
    for child in &scene.children {
        unknown_icons(child, changed, touched, out);
    }
}

/// Whether `scene` or a layer in it is one of `touched`.
fn holds_touched(scene: &SceneNode, touched: &[String]) -> bool {
    touched.contains(&scene.id) || scene.children.iter().any(|child| holds_touched(child, touched))
}

/// The top-level frame `top` when it is only a container around one screen. Gives the frame in it.
// Human: the canvas writes a top-level frame's name above it, so a board around a screen puts a box nobody
// asked for around the work, and a caption on it shows the name twice. Two builds are certain enough to
// name: a frame and a caption that repeats the container's name or the frame's own, and a frame alone
// with room around it on every side. A caption that says something else may be a page's heading, and a
// frame that fills its parent is the screen's own content, so neither is remarked on.
fn wrapped(top: &SceneNode) -> Option<&SceneNode> {
    let shown = |layer: &&SceneNode| !matches!(&layer.node.enabled, Some(Flag::Value(false)));
    let mut inner = top.children.iter().filter(shown).filter(|layer| layer.node.kind != Kind::Text);
    let screen = inner.next().filter(|layer| matches!(layer.node.kind, Kind::Frame | Kind::Ref))?;
    if top.node.kind != Kind::Frame || inner.next().is_some() {
        return None;
    }
    let captions: Vec<&str> =
        top.children.iter().filter(shown).filter(|layer| layer.node.kind == Kind::Text).filter_map(|layer| layer.node.content.as_deref()).collect();
    let names = [top.node.name.as_deref(), screen.node.name.as_deref()];
    let repeats = captions.len() <= MOST_CAPTIONS
        && captions.iter().any(|text| names.iter().flatten().any(|name| !name.trim().is_empty() && text.trim().eq_ignore_ascii_case(name.trim())));
    let (frame, part) = (top.local, screen.local);
    let inset = part.x >= ROOM && part.y >= ROOM && frame.w - (part.x + part.w) >= ROOM && frame.h - (part.y + part.h) >= ROOM;
    (repeats || (captions.is_empty() && inset)).then_some(screen)
}

/// What to say about the top-level layers of `tops` that are only a container around one screen; `None`
/// when there is none.
#[must_use]
pub fn container_note(tops: &[&SceneNode]) -> Option<String> {
    let boards: Vec<String> = tops
        .iter()
        .filter_map(|top| {
            wrapped(top)
                .map(|screen| format!("\"{}\" ({}) around \"{}\" ({})", top.node.display_name(), top.id, screen.node.display_name(), screen.id))
        })
        .collect();
    (!boards.is_empty()).then(|| {
        let more = if boards.len() > MOST_NAMED { format!(" and {} more", boards.len() - MOST_NAMED) } else { String::new() };
        format!(
            "A screen needs no container around it: {}{more}. The canvas already writes a top-level frame's name above it, so the container is a box around the work, and a caption on it shows the name twice. Unless the user asked for a board or a background behind the screen, move the frame inside to the top level at the container's place (a move without `parent`), give it the container's name and theme, and delete the container.",
            boards.iter().take(MOST_NAMED).map(String::as_str).collect::<Vec<_>>().join("; ")
        )
    })
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
    let mut icons = Vec::new();
    for top in &editor.scene {
        unknown_icons(top, false, touched, &mut icons);
    }
    if !icons.is_empty() {
        let more = if icons.len() > MOST_NAMED { format!(" and {} more", icons.len() - MOST_NAMED) } else { String::new() };
        out.push(format!(
            "Icons that don't exist are drawn as a small empty box: {}{more}. Only the Lucide set is bundled (library \"lucide\"); use an icon name from it.",
            icons.iter().take(MOST_NAMED).map(String::as_str).collect::<Vec<_>>().join("; ")
        ));
    }
    let tops: Vec<&SceneNode> = editor.scene.iter().filter(|top| holds_touched(top, touched)).collect();
    out.extend(container_note(&tops));
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

    #[test]
    fn a_dialog_over_a_scrim_is_not_compared_with_the_page_under_it() {
        let json = r##"{"version":"2.22","children":[{"type":"frame","id":"screen","x":0,"y":0,"width":400,"height":300,"layout":"none","fill":"#ffffff","children":[
          {"type":"rectangle","id":"scrim","x":0,"y":0,"width":400,"height":300,"fill":"#00000033"},
          {"type":"frame","id":"dialog","name":"Dialog","x":100,"y":100,"width":200,"height":100,"fill":"#ffffff"}]}]}"##;
        assert_eq!(advice(&editor(json), &["screen".to_owned()]), Vec::<String>::new());
        // Without the scrim the white dialog on the white screen is pointed out.
        let bare = json.replace("\"fill\":\"#00000033\"", "\"fill\":\"#00000000\"");
        assert_eq!(advice(&editor(&bare), &["screen".to_owned()]).len(), 1);
    }

    #[test]
    fn a_container_that_only_names_the_screen_in_it_is_pointed_out() {
        let json = r##"{"version":"2.22","children":[
          {"type":"frame","id":"board","name":"1 · General","x":0,"y":0,"layout":"vertical","fill":"#8b8b90","children":[
            {"type":"text","id":"caption","content":"1 · General","fill":"#ffffff"},
            {"type":"frame","id":"win","name":"Settings window","width":200,"height":100,"fill":"#ffffff"}]},
          {"type":"frame","id":"parts","name":"Parts","x":900,"y":0,"layout":"vertical","fill":"#8b8b90","children":[
            {"type":"text","id":"title","content":"Parts","fill":"#ffffff"},
            {"type":"frame","id":"a","width":50,"height":50,"fill":"#ffffff"},
            {"type":"frame","id":"b","width":50,"height":50,"fill":"#ffffff"}]},
          {"type":"frame","id":"page","name":"Home","x":1800,"y":0,"layout":"vertical","fill":"#ffffff","children":[
            {"type":"text","id":"heading","content":"Welcome","fill":"#000000"},
            {"type":"frame","id":"hero","width":50,"height":50,"fill":"#eeeeee"}]}]}"##;
        let said = advice(&editor(json), &["win".to_owned(), "parts".to_owned(), "page".to_owned()]);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].contains("\"1 · General\" (board) around \"Settings window\" (win)"), "{}", said[0]);
        // A change elsewhere says nothing about it.
        assert_eq!(advice(&editor(json), &["parts".to_owned()]), Vec::<String>::new());
        // A caption that repeats the name of the frame in it counts too.
        let own = json.replace(r#""content":"1 · General""#, r#""content":"Settings window""#);
        assert_eq!(advice(&editor(&own), &["board".to_owned()]).len(), 1);
    }

    #[test]
    fn a_frame_that_only_holds_a_smaller_frame_is_a_container_and_a_screen_with_its_content_is_not() {
        let json = r##"{"version":"2.22","children":[
          {"type":"frame","id":"board","name":"General","x":0,"y":0,"layout":"vertical","padding":48,"fill":"#8b8b90","children":[
            {"type":"frame","id":"win","name":"Settings window","width":200,"height":100,"fill":"#ffffff"}]},
          {"type":"frame","id":"screen","name":"Home","x":900,"y":0,"width":400,"height":300,"layout":"vertical","fill":"#ffffff","children":[
            {"type":"frame","id":"content","name":"Content","width":"fill_container","height":"fill_container","fill":"#eeeeee"}]}]}"##;
        let editor = editor(json);
        let tops: Vec<&SceneNode> = editor.scene.iter().collect();
        let said = container_note(&tops).unwrap_or_default();
        assert!(said.contains("\"General\" (board) around \"Settings window\" (win)") && !said.contains("Home"), "{said}");
    }

    #[test]
    fn an_icon_that_does_not_exist_is_named_with_the_nearest_one() {
        let editor = editor(
            r#"{"version":"2.22","children":[{"type":"frame","id":"bar","x":0,"y":0,"children":[
              {"type":"icon","id":"ok","icon":"check","width":16,"height":16},
              {"type":"icon","id":"old","icon":"home","width":16,"height":16},
              {"type":"icon","id":"bad","name":"Spinner","icon":"loader-circl","width":16,"height":16}]}]}"#,
        );
        let named = advice(&editor, &["bar".to_owned()]);
        assert_eq!(named.len(), 1, "{named:?}");
        assert!(named[0].contains("\"Spinner\" (bad) asks for \"loader-circl\" from lucide (nearest: \"loader-circle\")"), "{}", named[0]);
        assert!(!named[0].contains("(ok)") && !named[0].contains("(old)"));
    }
}
