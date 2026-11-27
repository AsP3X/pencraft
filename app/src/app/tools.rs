//! Doing what an AI model asks for through the tools of [`crate::agent`]: the calls the local server
//! ([`crate::link`]) hands over are answered here, on the main thread, where the documents are.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pen_doc::geom::Rect;
use pen_doc::layout;
use pen_doc::scene::SceneNode;
use pen_doc::vars::Vars;
use serde_json::{Value, json};

use super::talk::{Home, home};
use super::{App, text_measure};
use crate::activity::{Turn, What};
use crate::agent;
use crate::editor::{Editor, find_in};
use crate::link::Call;

/// How long a change waits for the user to finish a drag on the canvas before the model is told to try again.
/// A finished picture waits as long to be put on its layer (`App::finish_pictures`).
pub(super) const BUSY_PATIENCE: Duration = Duration::from_secs(10);
/// What `get_state` says when the assistant may not make pictures.
const SERVICES_OFF: &str = "Generating and editing pictures is turned off for AI models in Quill's settings.";
/// The longest side of a picture given to a model, pixels: larger ones are drawn at a smaller scale.
const MOST_PICTURE_SIDE: f32 = 2000.0;
/// What the model's cursor is labelled with when the settings name nothing else.
const AGENT_NAME: &str = "Agent";
/// How many parts of a design the reviewer is shown enlarged, and the fewest layers such a part holds.
const MOST_PARTS: usize = 4;
/// How many pixels a logo's shorter side has in the small proof that goes with its picture.
// Source: a favicon is 16 or 32 pixels, a tab or list icon 24; a mark that holds at 24 holds on a sign.
const SMALL_PROOF: f32 = 24.0;
/// How many times the small proof is enlarged, pixel by pixel, so a model sees what is left of the logo.
const PROOF_ENLARGED: usize = 6;
/// See [`MOST_PARTS`].
const PART_LAYERS: usize = 8;
/// The fewest layers a model has to have written since the last review for a reviewer to be asked.
const WORTH_A_REVIEW: usize = 30;
/// How long a call waits for the faces of a document's text to be made ready.
const FACE_PATIENCE: Duration = Duration::from_secs(3);
/// How long laying out the document after each operation of a call may take in all, for the canvas to show
/// the call an operation at a time.
const STAGING_MOST: Duration = Duration::from_millis(250);
/// The tools that change a document.
const CHANGING: [&str; 2] = ["edit_layers", "select_layers"];
/// The tools that work on the design, which an answer that has offered next steps may not use any more.
const WORKING: [&str; 3] = ["edit_layers", "generate_image", "edit_image"];
/// The tools whose call waits for a picture to be made ([`App::start_picture`]).
const PICTURING: [&str; 2] = ["generate_image", "edit_image"];

/// A tool's answer as text content: `value` as JSON.
fn text(value: &Value) -> Vec<Value> {
    vec![json!({ "type": "text", "text": serde_json::to_string_pretty(value).unwrap_or_default() })]
}

/// The scene nodes with the ids `wanted`, or every top-level one without any.
fn scenes<'a>(editor: &'a Editor, wanted: &[String]) -> Result<Vec<&'a SceneNode>, String> {
    if wanted.is_empty() {
        return Ok(editor.scene.iter().collect());
    }
    wanted.iter().map(|id| editor.scene_node(id).ok_or_else(|| format!("There is no layer with the id \"{id}\"."))).collect()
}

/// For a font family that isn't installed, the family drawn in its place.
fn stands_in(family: &str) -> Option<String> {
    (!crate::faces::installed(family)).then(|| crate::faces::shown_as(family).unwrap_or("the interface font").to_owned())
}

/// Whether `scene` is the layer `id` or holds it.
pub(super) fn holds(scene: &SceneNode, id: &str) -> bool {
    scene.id == id || scene.children.iter().any(|child| holds(child, id))
}

/// How many texts one answer measures the letters of. A logo or a headline has a few; a page of text gains
/// nothing from it and would pay for every line.
const MOST_INKS: usize = 24;

/// The box round the letters of the text layer `scene` as they are drawn, as a tool's answer gives it.
fn ink_of(scene: &SceneNode, editor: &Editor, ctx: &egui::Context) -> Option<Value> {
    let ink = crate::render::ink_box(ctx, scene, Vars::new(&editor.doc))?;
    let round = |v: f32| (f64::from(v) * 10.0).round() / 10.0;
    Some(json!({ "x": round(ink.x), "y": round(ink.y), "width": round(ink.w), "height": round(ink.h) }))
}

/// The boxes round the letters of the text layers among `touched` and in them, by id; `None` without one.
fn inks_of(editor: &Editor, touched: &[String], ctx: &egui::Context) -> Option<Value> {
    let inks: serde_json::Map<String, Value> = touched
        .iter()
        .filter_map(|id| editor.scene_node(id))
        .flat_map(|scene| texts_in(scene))
        .take(MOST_INKS)
        .filter_map(|scene| ink_of(scene, editor, ctx).map(|ink| (scene.id.clone(), ink)))
        .collect();
    (!inks.is_empty()).then_some(Value::Object(inks))
}

/// The text layers in `scene`, itself included: a text put in with its group is measured too.
fn texts_in(scene: &SceneNode) -> Vec<&SceneNode> {
    let mut found = Vec::new();
    scene.walk(
        &mut |layer, _| {
            if layer.node.kind == pen_doc::model::Kind::Text {
                found.push(layer);
            }
        },
        0,
    );
    found
}

/// Adds to every text layer in `value` (what `read_layers` answers) the box round its letters, as `inkBox`.
fn add_ink(value: &mut Value, editor: &Editor, ctx: &egui::Context, left: &mut usize) {
    match value {
        Value::Object(object) => {
            let ink = (*left > 0 && object.get("type").and_then(Value::as_str) == Some("text"))
                .then(|| object.get("id").and_then(Value::as_str).and_then(|id| editor.scene_node(id)).and_then(|scene| ink_of(scene, editor, ctx)))
                .flatten();
            if let Some(ink) = ink {
                object.insert("inkBox".to_owned(), ink);
                *left -= 1;
            }
            object.values_mut().for_each(|inner| add_ink(inner, editor, ctx, left));
        }
        Value::Array(list) => list.iter_mut().for_each(|inner| add_ink(inner, editor, ctx, left)),
        _ => {}
    }
}

/// `picture` with every pixel drawn `times` across and down: a small picture made large without smoothing,
/// so what is left of a design at that size can be seen.
fn enlarged(picture: &crate::export::Picture, times: usize) -> crate::export::Picture {
    let (w, h) = (picture.w * times, picture.h * times);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let at = ((y / times) * picture.w + x / times) * 4;
            rgba.extend_from_slice(picture.rgba.get(at..at + 4).unwrap_or(&[0; 4]));
        }
    }
    crate::export::Picture { w, h, rgba }
}

/// The box around `layers` on the canvas.
fn around(layers: &[&SceneNode]) -> Option<Rect> {
    layers.iter().map(|scene| layout::doc_bounds(scene)).reduce(|a, b| a.union(&b))
}

/// Whether the document point `at` shows on the canvas `view`, clear of the islands floating over its edges.
fn in_sight(editor: &Editor, view: egui::Rect, at: pen_doc::geom::Point) -> bool {
    let [left, top, right, bottom] = editor.covered;
    let zoom = editor.camera.zoom.max(f32::EPSILON);
    let (w, h) = (view.width() / zoom, view.height() / zoom);
    let (x, y) = (editor.camera.center.x - w / 2.0, editor.camera.center.y - h / 2.0);
    at.x >= x + left / zoom && at.x <= x + w - right / zoom && at.y >= y + top / zoom && at.y <= y + h - bottom / zoom
}

/// A full path a model named, or why it can't be used.
fn full_path(args: &Value, key: &str) -> Result<Option<PathBuf>, String> {
    let Some(text) = args.get(key).and_then(Value::as_str).filter(|text| !text.trim().is_empty()) else { return Ok(None) };
    // Human: a network path would make Windows sign in to another computer; a model is not let name one.
    if crate::images::network_path(text) {
        return Err(format!("`{key}` must be a file on this computer: \"{text}\" is a network path."));
    }
    let path = PathBuf::from(text);
    if path.is_absolute() { Ok(Some(path)) } else { Err(format!("`{key}` must be a full path, starting at the root: \"{text}\" isn't.")) }
}

/// Whether `a` and `b` name the same file: the same path, or the same file once links and `..` are followed.
fn same_file(a: &Path, b: &Path) -> bool {
    a == b || matches!((std::fs::canonicalize(a), std::fs::canonicalize(b)), (Ok(a), Ok(b)) if a == b)
}

/// The path `save_document` saves under, from the `path` it was given: a `.quill` file's, with that ending put
/// after a name that has none.
// Human: a document is saved as a `.quill` file only. A `.pen` file is written by `export_document`, which the
// refusal names; a `.pencraft` file by nothing any more.
fn save_path(path: PathBuf) -> Result<PathBuf, String> {
    if crate::bundle::is_old(&path) {
        return Err("A .pencraft file is no longer written: give `path` the ending .quill.".to_owned());
    }
    if path.extension().is_some_and(|ending| ending.eq_ignore_ascii_case(crate::bundle::PLAIN)) {
        return Err("A document is saved as a .quill file: give `path` the ending .quill, or write a .pen file with `export_document`.".to_owned());
    }
    crate::bundle::ended(path, crate::bundle::ENDING).map_err(|error| format!("Couldn't save: {error}."))
}

/// `export_document`: writes the document of `editor` as a `.pen` file at the `path` in `args`, with the files
/// it names beside it, and answers with where it was written and what there is to know about the file.
// Human: the same export as File › Export .pen File… (`pictures::export_pen`): the document keeps its file, its
// working folder and its unsaved changes.
// Agent: READS and WRITES files; BLOCKS on them.
fn export_document(editor: &Editor, args: &Value, home: Option<&Path>) -> Result<Value, String> {
    let path = full_path(args, "path")?.ok_or("`export_document` needs the `path` of the .pen file to write.")?;
    if crate::bundle::is_old(&path) {
        return Err("A .pencraft file is no longer written: `export_document` writes a .pen file, `save_document` a .quill file.".to_owned());
    }
    if crate::bundle::is_bundle(&path) {
        return Err("`export_document` writes a .pen file: a .quill file is saved with `save_document`.".to_owned());
    }
    let target = crate::bundle::ended(path, crate::bundle::PLAIN).map_err(|error| format!("Couldn't export: {error}."))?;
    // Human: nobody is asked here before a file is replaced, as the file dialog asks the person: a file that
    // is there, also the `.pen` file the document was opened from, is not written over.
    if target.exists() {
        return Err(format!("{} is there already: `export_document` doesn't replace a file. Choose another name.", target.display()));
    }
    let notes = super::pictures::export_pen(editor, &target, home).map_err(|error| format!("Couldn't export: {error}."))?;
    let mut answer = json!({ "exported": target.display().to_string() });
    if !notes.is_empty() {
        answer["notes"] = json!(notes);
    }
    Ok(answer)
}

/// Puts what `get_state` says about image services into `state`: the `listed` ones, or, when the assistant may
/// not make pictures (`assistant` is off), none and why.
fn tell_services(state: &mut Value, assistant: bool, listed: Value) {
    if assistant {
        state["imageServices"] = listed;
    } else {
        state["imageServices"] = json!([]);
        state["imageServicesNote"] = json!(SERVICES_OFF);
    }
}

/// Asks for the face of every text layer written out in `value` (a tool call's arguments).
fn want_faces(value: &Value) {
    match value {
        Value::Object(object) => {
            if let Some(family) = object.get("fontFamily").and_then(Value::as_str) {
                let weight =
                    object.get("fontWeight").and_then(|weight| weight.as_str().and_then(|text| text.parse().ok()).or_else(|| weight.as_u64()));
                let weight = weight.and_then(|weight| u16::try_from(weight).ok()).unwrap_or(400);
                let italic = object.get("fontStyle").and_then(Value::as_str).is_some_and(|style| style.eq_ignore_ascii_case("italic"));
                crate::fonts::want(family, weight, italic);
            }
            object.values().for_each(want_faces);
        }
        Value::Array(list) => list.iter().for_each(want_faces),
        _ => {}
    }
}

impl App {
    /// Answers the tool calls that have come in since the last time.
    // Human: a change waits while the user is dragging something on the canvas (the drag would otherwise work on
    // a document that changed under it); everything else is answered at once.
    // Agent: MAIN-THREAD; runs from `logic`, so also while the window is hidden or behind others.
    pub(super) fn serve_tools(&mut self, ctx: &egui::Context) {
        self.finish_pictures(ctx);
        let mut calls = std::mem::take(&mut self.waiting);
        calls.extend(self.tool_calls.try_iter().map(|call| (call, Instant::now())));
        if calls.is_empty() {
            return;
        }
        // Human: the faces the new layers name are made ready before anything is measured, so the boxes the
        // model is told are those of the text in its own face.
        for (call, _) in &calls {
            want_faces(&call.arguments);
        }
        self.settle_fonts(ctx);
        for (call, since) in calls {
            // Human: files a second Quill passes on are the user's own doing, whatever the settings say
            // about AI models: they open in tabs and the window comes to the front.
            if call.tool == crate::link::OPEN {
                for path in call.arguments.get("paths").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
                    self.open_path(Path::new(path));
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                ctx.request_repaint();
                // A second Quill that stopped waiting opens the files itself; there is no one to tell.
                let _ = call.reply.send(Ok(Vec::new()));
                continue;
            }
            let on = self.prefs.get().agents.tools;
            // Human: a tool the user turned off in the settings is neither offered nor run.
            if on && self.prefs.get().agents.tools_off().contains(&call.tool) {
                let refused = format!("The tool `{}` is turned off in Quill's settings (MCP). Go on without it.", call.tool);
                // A client that stopped waiting has gone; there is no one to tell.
                let _ = call.reply.send(Err(refused));
                continue;
            }
            // Human: the web can be slow; its tools answer from a thread of their own, and the window goes on
            // meanwhile. A picture goes beside the document the call is about.
            if on && crate::web::is_web(&call.tool) {
                let file = self.document(&call).ok().and_then(|tab| self.tabs.get(tab)).and_then(crate::editor::Editor::anchor);
                let wake = ctx.clone();
                let keyed = self.prefs.get().agents.picture_keys.clone();
                crate::web::answer(call, file, keyed, move || wake.request_repaint());
                continue;
            }
            // Human: a picture takes up to a minute, made or changed; its call is kept and answered when the
            // picture is there.
            // Human: next steps the assistant offers are the user's to pick. An answer that has offered some
            // and goes on working is told to stop; the step it wanted is done when the user picks it.
            if WORKING.contains(&call.tool.as_str())
                && let Some(talk) = call.chat.and_then(|id| self.talk_mut(id)).filter(|talk| talk.chat.awaits_choice())
            {
                let why = crate::chat::asks::NOT_CHOSEN;
                log::warn!("the tool {} was refused: {why}", call.tool);
                talk.chat.tool_failed(&call.tool, why);
                // A client that stopped waiting has gone; there is no one to tell.
                let _ = call.reply.send(Err(why.to_owned()));
                continue;
            }
            if PICTURING.contains(&call.tool.as_str()) && self.prefs.get().agents.tools {
                // Human: layers an edit sends as they look are drawn when it starts, so it waits for the faces
                // of the document's text as a screenshot does, and goes on without them after as long.
                if self.draws(&call) && self.faces_coming(&call, ctx) && since.elapsed() < FACE_PATIENCE {
                    self.waiting.push((call, since));
                    continue;
                }
                self.start_picture(call, ctx);
                continue;
            }
            let result = if !self.prefs.get().agents.tools {
                Err("AI tools are turned off in Quill's settings (Agents: \"Let AI models work on documents\").".to_owned())
            } else if self.canvas.dragging() && CHANGING.contains(&call.tool.as_str()) {
                if since.elapsed() < BUSY_PATIENCE {
                    self.waiting.push((call, since));
                    continue;
                }
                Err("The user is moving something on the canvas right now. Try again in a moment.".to_owned())
            } else if self.faces_coming(&call, ctx) && since.elapsed() < FACE_PATIENCE {
                // Human: a face the document's text asks for is still being made ready (a frame or two): until
                // then text is measured and drawn in a stand-in, and a picture or a box given now would be wrong.
                self.waiting.push((call, since));
                continue;
            } else {
                self.tool(&call, ctx)
            };
            // Human: a call that failed says why in the app's log and in the chat it came from.
            if let Err(why) = &result {
                log::warn!("the tool {} failed: {why}", call.tool);
                if let Some(talk) = call.chat.and_then(|id| self.talk_mut(id)) {
                    talk.chat.tool_failed(&call.tool, why);
                }
            }
            // A client that stopped waiting has gone; there is no one to tell.
            let _ = call.reply.send(result);
        }
        if self.waiting.is_empty() {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    /// Whether the document a call is about has text in a face that is not ready yet. Asks for those faces.
    fn faces_coming(&mut self, call: &Call, ctx: &egui::Context) -> bool {
        let Ok(tab) = self.document(call) else { return false };
        let Some(editor) = self.tabs.get_mut(tab) else { return false };
        editor.refresh(&text_measure(ctx));
        crate::fonts::pending()
    }

    /// The tab a call works on when it names no document: the document of the chat it comes from, or the one
    /// in front for a client outside the app.
    // Human: the chats of several documents work at the same time. Each keeps to its own document, also while
    // the user has another one in front; and when its document is closed it is told so, instead of going on in
    // whichever document is in front then.
    fn own(&self, call: &Call) -> Result<usize, String> {
        match home(&self.talks, call.chat) {
            Home::Front => {
                self.active.or((!self.tabs.is_empty()).then_some(0)).ok_or_else(|| "No document is open. Open one with open_document.".to_owned())
            }
            Home::Doc(doc) => {
                self.tabs.iter().position(|tab| tab.id == doc).ok_or_else(|| {
                    "The document this chat belongs to was closed. get_state lists the open ones; name one with `document`.".to_owned()
                })
            }
            Home::Gone => Err("This chat was closed with its document.".to_owned()),
        }
    }

    /// Whether a call may change which document is in front: it comes from outside the app, or from the chat
    /// the user is looking at.
    fn leads(&self, call: &Call) -> bool {
        call.chat.is_none_or(|id| self.talk(id).is_some_and(|talk| talk.doc == self.front_doc()))
    }

    /// The tab a call is about: the one its `document` names (by path, file name or title), or its own.
    pub(super) fn document(&self, call: &Call) -> Result<usize, String> {
        let args = &call.arguments;
        if let Some(wanted) = args.get("document").and_then(Value::as_str).filter(|name| !name.is_empty()) {
            let path = Path::new(wanted);
            return self
                .tabs
                .iter()
                .position(|tab| tab.path.as_deref() == Some(path))
                .or_else(|| {
                    self.tabs
                        .iter()
                        .position(|tab| tab.title() == wanted || tab.path.as_ref().and_then(|p| p.file_name()).is_some_and(|name| name == wanted))
                })
                .ok_or_else(|| format!("No open document is called \"{wanted}\". get_state lists the open ones."));
        }
        self.own(call)
    }

    /// `open_document`: opens the file `path`, or starts an empty document.
    // Human: the chat that opens a document goes on in it. A chat the user is not looking at opens it behind
    // the document in front, so the user is not pulled away from what they are doing.
    fn open_tool(&mut self, call: &Call) -> Result<(), String> {
        let (leads, before) = (self.leads(call), self.front_doc());
        match full_path(&call.arguments, "path")? {
            Some(path) => self.try_open(&path)?,
            None => self.new_file(),
        }
        if let (Some(chat), Some(opened)) = (call.chat, self.front_doc()) {
            self.follow(chat, opened);
        }
        if !leads {
            self.active = before.and_then(|doc| self.tabs.iter().position(|tab| tab.id == doc));
        }
        Ok(())
    }

    /// The name shown beside the model's cursor, as the settings want it.
    // Human: "Agent" for every model, or the model's own name: the chat's assistant is known by its model, an
    // outside client by the name it introduced itself with.
    fn cursor_label(&self, call: &Call) -> String {
        match self.prefs.get().agents.cursor_label.as_str() {
            "None" => String::new(),
            "Model name" => call.chat.and_then(|id| self.talk(id)).and_then(|talk| talk.chat.answering()).unwrap_or(&call.client).to_owned(),
            _ => Some(self.prefs.get().agents.agent_label.trim()).filter(|label| !label.is_empty()).unwrap_or(AGENT_NAME).to_owned(),
        }
    }

    /// Does one tool call and gives its answer's content.
    fn tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let args = &call.arguments;
        match call.tool.as_str() {
            "get_guide" => Ok(vec![json!({ "type": "text", "text": crate::craft::guide() })]),
            "list_skills" => Ok(text(&crate::skills::list())),
            "read_skill" => Ok(vec![json!({ "type": "text", "text": crate::skills::read(args)? })]),
            "get_state" => Ok(text(&self.state(call, ctx))),
            "open_document" => {
                self.open_tool(call)?;
                Ok(text(&self.state(call, ctx)))
            }
            "save_document" => {
                let tab = self.document(call)?;
                let path = full_path(args, "path")?;
                let editor = self.tabs.get_mut(tab).ok_or("The document was closed.")?;
                let mut was = editor.anchor();
                // Human: when the file can't be written under a new name, the document has the file it had again.
                let before = editor.place();
                let (mut moved, mut left) = (false, None);
                if let Some(path) = path {
                    let target = save_path(path)?;
                    // Human: as `export_document`: a file that is there is not written over, unless it is the
                    // document's own file, which saving writes anyway.
                    if target.exists() && !editor.path.as_deref().is_some_and(|own| same_file(own, &target)) {
                        return Err(format!("{} is there already: `save_document` doesn't replace a file. Choose another name.", target.display()));
                    }
                    was = editor.move_to(target).map_err(|error| format!("Couldn't save: {error}."))?;
                    moved = true;
                } else if let Some(next) = editor.path.as_deref().and_then(crate::bundle::successor) {
                    // Human: as when the person saves a `.pen` document or an old bundle: it becomes a `.quill`
                    // file beside it.
                    let old = editor.tab_name();
                    if next.exists() {
                        return Err(format!("{} is there already: give `path` to say where {old} is saved as a .quill file.", next.display()));
                    }
                    was = editor.move_to(next).map_err(|error| format!("Couldn't save: {error}."))?;
                    (moved, left) = (true, Some(old));
                }
                // Human: as when the person saves: the pictures come to where the document is saved.
                let stuck = super::pictures::settle(editor, was.as_deref(), crate::imagery::store::home().as_deref());
                let from = moved.then_some(before);
                let mut notes =
                    editor.save_or_return(from).map_err(|error| format!("Couldn't save: {error}. A document without a file needs `path`."))?;
                if let Some(left) = left {
                    notes.push(format!("The design was opened from {left}. It is saved as {} now; {left} is left as it is.", editor.tab_name()));
                }
                let saved = editor.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
                self.prefs.update(|p| p.files.touch(&saved));
                self.remember_tabs();
                let mut answer = json!({ "saved": saved });
                if !stuck.is_empty() {
                    answer["picturesNotBrought"] = json!(stuck);
                }
                if !notes.is_empty() {
                    answer["notes"] = json!(notes);
                }
                Ok(text(&answer))
            }
            "read_layers" => self.read_tool(call, ctx),
            "edit_layers" => self.edit_tool(call, ctx),
            "screenshot" => self.picture_tool(call, ctx),
            "check_glass" => self.glass_tool(call, ctx),
            "build_path" => Ok(text(&crate::construct::build(args)?)),
            "measure_layers" => {
                let (tab, editor, label) = self.laid_out(call, ctx)?;
                let layers = scenes(editor, &agent::ids(args, "ids"))?;
                let (bounds, measured) = (around(&layers), crate::construct::measure(&layers));
                if let Some(editor) = self.tabs.get_mut(tab) {
                    editor.activity.point(bounds, &label, Instant::now());
                }
                Ok(text(&measured))
            }
            crate::chat::critic::TOOL => self.review_tool(call, ctx),
            "select_layers" => self.select_tool(call, ctx),
            crate::chat::asks::TOOL => self.ask_tool(call),
            crate::chat::tasks::PLAN | crate::chat::tasks::ADD | crate::chat::tasks::FINISH => self.task_tool(call),
            "export_document" => {
                let tab = self.document(call)?;
                let editor = self.tabs.get(tab).ok_or("The document was closed.")?;
                Ok(text(&export_document(editor, args, crate::imagery::store::home().as_deref())?))
            }
            "export_layers" => self.export_tool(call, ctx),
            other => Err(format!("There is no tool called \"{other}\".")),
        }
    }

    /// `ask_user`: says whether the question can be put to the user. The chat itself shows it, from the call
    /// as its run reports it.
    // Human: the call answers at once. The user's answer is their next message, so the assistant doesn't hold
    // a tool call open for as long as the user takes to decide.
    fn ask_tool(&mut self, call: &Call) -> Result<Vec<Value>, String> {
        let asked = crate::chat::asks::parse(&call.arguments)?;
        let Some(talk) = call.chat.and_then(|id| self.talk_mut(id)) else {
            return Err(crate::chat::asks::NO_CHAT.to_owned());
        };
        // Human: from an offer of next steps on, the answer changes nothing more: the user chooses.
        if asked.suggests {
            talk.chat.hold_for_choice();
        }
        let said = if asked.suggests { crate::chat::asks::SUGGESTED } else { crate::chat::asks::ASKED };
        Ok(vec![json!({ "type": "text", "text": said })])
    }

    /// `plan_tasks`, `add_tasks` and `finish_task`: the task list of the chat the call comes from.
    // Human: the list is the chat's, so the panel shows it; the order of the tasks is kept here, where the call
    // is answered, and not left to the model.
    fn task_tool(&mut self, call: &Call) -> Result<Vec<Value>, String> {
        let chat = &mut call.chat.and_then(|id| self.talk_mut(id)).ok_or(crate::chat::tasks::NO_CHAT)?.chat;
        Ok(vec![json!({ "type": "text", "text": chat.task_call(&call.tool, &call.arguments)? })])
    }

    /// The document a call is about, laid out, with its tab and the name for the model's cursor.
    fn laid_out(&mut self, call: &Call, ctx: &egui::Context) -> Result<(usize, &mut Editor, String), String> {
        let tab = self.document(call)?;
        let label = self.cursor_label(call);
        let editor = self.tabs.get_mut(tab).ok_or("The document was closed.")?;
        editor.refresh(&text_measure(ctx));
        Ok((tab, editor, label))
    }

    fn read_tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let args = &call.arguments;
        let (_, editor, label) = self.laid_out(call, ctx)?;
        let mut read = agent::read(editor, args)?;
        add_ink(&mut read, editor, ctx, &mut MOST_INKS.clone());
        // The cursor goes to the layers the model reads by name.
        let looked = scenes(editor, &agent::ids(args, "ids")).ok().filter(|_| args.get("ids").is_some()).and_then(|layers| around(&layers));
        editor.activity.point(looked, &label, Instant::now());
        Ok(text(&read))
    }

    /// `edit_layers`: the change a call asks for, in the document it is about.
    // Human: a run that keeps changing the design without a task list is told, once, to make one.
    fn edit_tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let tab = self.document(call)?;
        let mut answer = self.edit_in(tab, call, ctx)?;
        if let Some(advice) = call.chat.and_then(|id| self.talk_mut(id)).and_then(|talk| talk.chat.tasks.changed()) {
            answer.push(json!({ "type": "text", "text": advice }));
        }
        Ok(answer)
    }

    /// The change an `edit_layers` call asks for, in the document at `tab`.
    // Human: one call is one undo step, made only when the call worked. What it changed is shown a small step
    // at a time, in the order of the call's operations: the canvas shows the document as it was after each
    // one, so a change is seen when it is made and a deleted layer goes when it is deleted. A layer that was
    // made is followed by each layer in it, parents first, and appears at its turn. At its turn a layer
    // flashes (one that is gone, where it was) and the model's cursor goes to it.
    // Human: laying the document out after every operation costs time the model waits for. After
    // `STAGING_MOST` the rest of the call is shown on the finished document: its new layers still appear one
    // by one, its changes and deletions are then seen at once.
    // Agent: a finished picture is put on its layer through this with the tab found by its document's id
    // (`App::fill_picture`), so two documents of the same name can't be mistaken for each other.
    pub(super) fn edit_in(&mut self, tab: usize, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let args = &call.arguments;
        let label = self.cursor_label(call);
        self.tabs.get_mut(tab).ok_or("The document was closed.")?.refresh(&text_measure(ctx));
        let editing = self.canvas.editing.clone();
        let view = self.canvas_view;
        let editor = self.tabs.get_mut(tab).ok_or("The document was closed.")?;
        let before = editor.doc.clone();
        let was = pen_doc::edit::ids(&before);
        let first = editor.scene.clone();
        let count = args.get("operations").and_then(Value::as_array).map_or(0, Vec::len);
        let began = Instant::now();
        let measure = text_measure(ctx);
        // What the document looked like after each operation but the last, which leaves it as it is.
        let mut staged: Vec<Option<Vec<SceneNode>>> = Vec::new();
        let (results, touched) = agent::edit_staged(editor, args, &mut |editor| {
            let worth = staged.len() + 1 < count && began.elapsed() < STAGING_MOST;
            staged.push(worth.then(|| editor.laid_out(&measure)));
        })?;
        let after = std::mem::replace(&mut editor.doc, before);
        editor.checkpoint();
        editor.doc = after;
        editor.refresh(&text_measure(ctx));
        let touched: Vec<String> = touched.into_iter().filter(|id| editor.scene_node(id).is_some()).collect();
        let selection: Vec<String> = editor.selection.iter().filter(|id| editor.scene_node(id).is_some()).cloned().collect();
        editor.selection = selection;
        let now = Instant::now();
        let place = touched.iter().filter_map(|id| editor.scene_node(id)).map(layout::doc_bounds).reduce(|a, b| a.union(&b));
        // The insides of an instance are no layers of the document: they come with their instance.
        let mut turns: Vec<Vec<Turn>> = Vec::new();
        let mut listed = std::collections::HashSet::new();
        let at = |index: usize| staged.get(index).and_then(Option::as_deref).unwrap_or(&editor.scene);
        for (index, result) in results.iter().enumerate() {
            let mut own = Vec::new();
            if let Some(id) = result.get("deleted").and_then(Value::as_str) {
                let had = if index == 0 { &first[..] } else { at(index - 1) };
                let place = find_in(had, id).map(layout::doc_bounds);
                own.push(Turn { id: id.to_owned(), what: What::Removed, place, scene: None });
            } else if let Some(top) = result.get("id").and_then(Value::as_str).and_then(|id| find_in(at(index), id)) {
                top.walk(
                    &mut |layer, depth| {
                        let new = layer.doc_id.as_ref().is_some_and(|id| !was.contains(id));
                        let fresh = listed.insert(layer.id.clone());
                        if depth == 0 || (new && fresh) {
                            let what = if new && fresh { What::Made } else { What::Changed };
                            own.push(Turn { id: layer.id.clone(), what, place: Some(layout::doc_bounds(layer)), scene: None });
                        }
                    },
                    0,
                );
            }
            // An operation on no layer to be seen (the variables) still has its turn: what it changed shows then.
            if own.is_empty() {
                own.push(Turn { id: String::new(), what: What::Changed, place: None, scene: None });
            }
            turns.push(own);
        }
        // Each operation's first step brings what the document looked like after it; from the first one that
        // wasn't kept on, that is the document as it is.
        let mut finished = false;
        for (index, own) in turns.iter_mut().enumerate() {
            let scene =
                staged.get_mut(index).and_then(Option::take).or_else(|| (!std::mem::replace(&mut finished, true)).then(|| editor.scene.clone()));
            if let Some(turn) = own.first_mut() {
                turn.scene = scene;
            }
        }
        editor.activity.play(turns.into_iter().flatten().collect(), &label, now, editor.generation);
        // Human: a change out of sight would flash unseen. When the middle of what was touched isn't on the
        // part of the canvas the islands leave free, the view goes there, as it does for "select layers".
        if let Some(place) = place
            && self.active == Some(tab)
            && !in_sight(editor, view, place.center())
        {
            editor.focus = Some(place);
        }
        // Human: what was changed is remembered by the top-level layer it is in, for the reviewer's picture.
        for top in &editor.scene {
            let written: Vec<&SceneNode> = touched.iter().filter(|id| holds(top, id)).filter_map(|id| editor.scene_node(id)).collect();
            if written.is_empty() {
                continue;
            }
            let layers = crate::review::size(&written);
            match self.designed.iter_mut().find(|(at, id, _)| *at == editor.id && *id == top.id) {
                Some((_, _, count)) => *count += layers,
                None => self.designed.push((editor.id, top.id.clone(), layers)),
            }
        }
        let mut answer = json!({ "results": results, "layoutBoxes": agent::boxes(editor, &touched) });
        // Human: a text's box is taller than its letters; where the letters are is what a model needs to set
        // a name level with a mark.
        if let Some(inks) = inks_of(editor, &touched, ctx) {
            answer["inkBoxes"] = inks;
        }
        // Human: the app looks at what the model made and says what it would not want to leave that way.
        let mut advice = crate::advice::advice(editor, &touched);
        // Human: the top-level layers changed are looked at whole: text is judged against the frame behind
        // it, which is often not the layer that was changed.
        let changed: Vec<&SceneNode> = editor.scene.iter().filter(|top| touched.iter().any(|id| holds(top, id))).collect();
        advice.extend(crate::review::mistakes_in(&changed, Vars::new(&editor.doc), &stands_in));
        if !advice.is_empty() {
            answer["advice"] = json!(advice);
        }
        // Human: the text being edited is in the document in front; a change to another document leaves it be.
        if self.active == Some(tab) && editing.is_some_and(|id| editor.scene_node(&id).is_none()) {
            self.canvas.editing = None;
        }
        // Human: what a chat's own assistant changes, its note in the chat (and in the one line over the
        // message bar) already says; a second message over it would only be in the way.
        if let Some(said) = args.get("description").and_then(Value::as_str).filter(|said| !said.trim().is_empty() && call.chat.is_none()) {
            self.say_from(&call.client, said);
        }
        Ok(text(&answer))
    }

    fn picture_tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let args = &call.arguments;
        let (tab, _, label) = self.laid_out(call, ctx)?;
        let editor = self.tabs.get(tab).ok_or("The document was closed.")?;
        let layers = scenes(editor, &agent::ids(args, "ids"))?;
        let bounds = around(&layers).ok_or("The document is empty: there is nothing to show.")?;
        #[expect(clippy::cast_possible_truncation, reason = "a scale between 0.1 and 3")]
        let asked = args.get("scale").and_then(Value::as_f64).map_or(1.0, |scale| scale.clamp(0.1, 3.0) as f32);
        // Human: a very large picture is drawn smaller, so it stays something a model can take in.
        let scale = asked.min(MOST_PICTURE_SIDE / bounds.w.max(bounds.h).max(1.0));
        let folder = editor.folder();
        // Human: the picture holds what the layers paint, which is more than their boxes where a shadow or an
        // outline reaches out of them and less where a mask cuts something away; the note says where the
        // picture itself lies, so a place measured in it is the same place on the canvas.
        let drawn =
            self.canvas.list().survey(&layers, scale, Vars::new(&editor.doc), ctx, folder).ok_or("The layers are empty or too large to draw.")?;
        let (picture, origin) = (drawn.picture, drawn.origin);
        #[expect(clippy::cast_precision_loss, reason = "a picture is at most a few thousand pixels a side")]
        let note = format!(
            "{} × {} pixels at {scale:.2} pixels per unit, showing the canvas from ({:.0}, {:.0}), {:.0} × {:.0} units.",
            picture.w,
            picture.h,
            origin.x,
            origin.y,
            picture.w as f32 / scale,
            picture.h as f32 / scale
        );
        let png = picture.encode(crate::export::Format::Png, 100)?;
        // Human: the moment the model looks at its work is the moment to tell it what the app counted in
        // these layers, and what to ask of the picture before calling it done.
        // Human: while the chat's assistant works on a request that names what to follow (a library, plain
        // standard components), the review says nothing about what that reference calls for.
        let asked = |test: fn(&str) -> bool| {
            call.chat
                .and_then(|id| self.talk(id))
                .is_some_and(|talk| talk.chat.entries.iter().any(|entry| entry.role == crate::chat::Role::User && test(&entry.text)))
        };
        let follows = asked(crate::craft::names_a_reference);
        // Human: a logo is judged as a logo: when the chat's request asks for one, or (for a client outside,
        // whose request the app never sees) when the layers shown are named as one, like "Holm logo". What
        // the app can count in a logo comes first, and the picture is shown once more as small as a logo
        // has to work, which is where a mark made of many parts gives itself away.
        let named_one = |layer: &&SceneNode| crate::craft::names_a_logo(&layer.node.display_name());
        let logo = asked(crate::craft::names_a_logo) || layers.iter().any(named_one);
        // Human: what the review counts in a page (cards in cards, a scale of sizes and spacings, a default
        // face) is not what a logo is made of; of a logo only the plain mistakes are told, then what is
        // counted in a logo.
        let mut notes = if logo {
            crate::review::mistakes_in(&layers, Vars::new(&editor.doc), &stands_in)
        } else {
            crate::review::review(&layers, Vars::new(&editor.doc), &stands_in, follows)
        };
        let mut small = None;
        if logo {
            // Human: a detailed logo's fine parts are right when a small version stands beside it.
            let has_small = editor.scene.iter().any(|top| crate::review::names_small(&top.node.display_name()));
            let counted = crate::review::logo_notes(&layers, Vars::new(&editor.doc), has_small);
            notes.splice(0..0, counted);
            let shorter = bounds.w.min(bounds.h).max(1.0);
            let proof = SMALL_PROOF / shorter;
            // A picture that is already this small is its own proof.
            if proof < scale * 0.5 {
                small = self
                    .canvas
                    .list()
                    .snapshot(&layers, proof, Vars::new(&editor.doc), ctx, folder)
                    .map(|picture| enlarged(&picture, PROOF_ENLARGED));
            }
        }
        // Human: while the chat's request names a phone or a tablet, a phone screen standing bare on the canvas
        // is the first thing to put right. It is judged by the top-level layers the picture shows or lies in,
        // so a picture of the screen inside a device doesn't count as a bare screen.
        let tops: Vec<&SceneNode> = editor.scene.iter().filter(|top| layers.iter().any(|layer| holds(top, &layer.id))).collect();
        if asked(crate::craft::names_a_device)
            && let Some(note) = crate::review::device_note(&tops)
        {
            notes.insert(0, note);
        }
        // Human: a container around a screen is seen best in the picture, and a design made before the app
        // pointed it out is only ever looked at, not changed, so the review names it too.
        if let Some(note) = crate::advice::container_note(&tops) {
            notes.insert(0, note);
        }
        notes.truncate(crate::review::MOST_NOTES);
        let mut answer =
            vec![json!({ "type": "image", "data": agent::base64(&png), "mimeType": "image/png" }), json!({ "type": "text", "text": note })];
        if let Some(png) = small.and_then(|small| small.encode(crate::export::Format::Png, 100).ok()) {
            answer.push(json!({ "type": "image", "data": agent::base64(&png), "mimeType": "image/png" }));
            answer.push(json!({ "type": "text", "text": format!("The small proof: the same layers drawn {SMALL_PROOF:.0} pixels on their shorter side, the size a logo has to hold at, shown {PROOF_ENLARGED} times enlarged so you see each pixel. What has filled in, broken up or vanished here is too small or too thin in the mark.") }));
        }
        answer.extend(crate::craft::with_picture(&notes, crate::review::size(&layers), logo).map(|said| json!({ "type": "text", "text": said })));
        if let Some(editor) = self.tabs.get_mut(tab) {
            editor.activity.point(Some(bounds), &label, Instant::now());
        }
        Ok(answer)
    }

    /// `check_glass`: the layers drawn once more with everything written on a material left out, so what
    /// each label lies on can be measured, and the checks' answer.
    fn glass_tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let args = &call.arguments;
        let (tab, _, label) = self.laid_out(call, ctx)?;
        let editor = self.tabs.get(tab).ok_or("The document was closed.")?;
        let layers = scenes(editor, &agent::ids(args, "ids"))?;
        let bounds = around(&layers).ok_or("The document is empty: there is nothing to check.")?;
        let scale = (MOST_PICTURE_SIDE / bounds.w.max(bounds.h).max(1.0)).min(1.0);
        let folder = editor.folder();
        let vars = Vars::new(&editor.doc);
        let bare: Vec<SceneNode> = layers.iter().map(|scene| crate::glass_check::without_labels(scene)).collect();
        let survey = self.canvas.list().survey(&bare.iter().collect::<Vec<_>>(), scale, vars, ctx, folder);
        let answer = crate::glass_check::check(&layers, vars, survey.as_ref());
        if let Some(editor) = self.tabs.get_mut(tab) {
            editor.activity.point(Some(bounds), &label, Instant::now());
        }
        Ok(text(&answer))
    }

    /// The reviewer's picture: the top-level layers changed in the chat's document since it last looked, with
    /// what the app counted in them. Nothing when little was written: a new headline or one more row is no design to review, and the
    /// user who asked for it didn't ask for the page around it to be judged.
    fn review_tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        // Human: each chat's review is of its own document; what another chat changed elsewhere waits for
        // that chat's review.
        let own = call.chat.and_then(|id| self.talk(id)).and_then(|talk| talk.doc);
        let Some(doc) = own.or_else(|| self.designed.last().map(|(doc, _, _)| *doc)) else { return Ok(Vec::new()) };
        let (designed, others): (Vec<_>, Vec<_>) = std::mem::take(&mut self.designed).into_iter().partition(|(at, _, _)| *at == doc);
        self.designed = others;
        // Human: a logo is a handful of layers, and the design most in need of a second pair of eyes: it is
        // reviewed however little was written.
        let logo = call.chat.and_then(|id| self.talk(id)).is_some_and(|talk| {
            talk.chat.entries.iter().any(|entry| entry.role == crate::chat::Role::User && crate::craft::names_a_logo(&entry.text))
        });
        let Some(editor) = self.tabs.iter_mut().find(|tab| tab.id == doc) else { return Ok(Vec::new()) };
        editor.refresh(&text_measure(ctx));
        let ids: Vec<String> = designed.iter().filter(|(_, id, _)| editor.scene_node(id).is_some()).map(|(_, id, _)| id.clone()).collect();
        let written: usize = designed.iter().map(|(_, _, count)| count).sum();
        // Human: a design that was reviewed and changed is looked at again however little was changed, and
        // so is a logo.
        let again = call.arguments.get("again").and_then(Value::as_bool) == Some(true);
        if ids.is_empty() || written < if again || logo { 1 } else { WORTH_A_REVIEW } {
            return Ok(Vec::new());
        }
        let document = editor.path.as_ref().map_or_else(|| editor.title(), |path| path.display().to_string());
        // Human: craft is judged in the details, which a picture of a whole page is too small to show. The
        // largest parts of the design are shown again, enlarged.
        let mut parts: Vec<&SceneNode> = ids
            .iter()
            .filter_map(|id| editor.scene_node(id))
            .flat_map(|top| &top.children)
            .filter(|part| crate::review::size(&[*part]) >= PART_LAYERS)
            .collect();
        parts.sort_by(|a, b| (b.local.w * b.local.h).total_cmp(&(a.local.w * a.local.h)));
        let parts: Vec<String> = parts.iter().take(MOST_PARTS).map(|part| part.id.clone()).collect();
        let picture = |arguments: Value| {
            let (reply, _) = std::sync::mpsc::channel();
            Call { tool: "screenshot".to_owned(), arguments, client: call.client.clone(), chat: call.chat, reply }
        };
        let mut content = self.picture_tool(&picture(json!({ "ids": ids, "document": document })), ctx)?;
        for part in parts {
            // A part that can't be drawn is left out; the whole is there.
            let enlarged = self.picture_tool(&picture(json!({ "ids": [part], "document": document, "scale": 2 })), ctx).unwrap_or_default();
            content.extend(enlarged.into_iter().filter(|item| item["type"] == "image"));
        }
        Ok(content)
    }

    fn select_tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let (tab, editor, label) = self.laid_out(call, ctx)?;
        let wanted = agent::ids(&call.arguments, "ids");
        let bounds = if wanted.is_empty() { None } else { around(&scenes(editor, &wanted)?) };
        editor.selection.clone_from(&wanted);
        editor.focus = bounds;
        editor.activity.point(bounds, &label, Instant::now());
        // Human: a chat the user is not looking at selects in its document without bringing it to the front.
        if self.leads(call) {
            self.active = Some(tab);
        }
        Ok(text(&json!({ "selection": wanted })))
    }

    fn export_tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let args = &call.arguments;
        let target = full_path(args, "path")?.ok_or("`export_layers` needs the `path` of the file to write.")?;
        let ending = target.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).map(|e| if e == "jpeg" { "jpg".to_owned() } else { e });
        let format = crate::export::Format::ALL
            .into_iter()
            .find(|format| ending.as_deref() == Some(format.extension()))
            .ok_or("The path must end in .png, .jpg, .webp, .pdf or .svg.")?;
        // Human: as `export_document`: nobody is asked before a file is replaced, so one that is there is left.
        if target.exists() {
            return Err(format!("{} is there already: `export_layers` doesn't replace a file. Choose another name.", target.display()));
        }
        let scale = args.get("scale").and_then(Value::as_u64).and_then(|scale| u8::try_from(scale.clamp(1, 3)).ok()).unwrap_or(1);
        let (tab, editor, label) = self.laid_out(call, ctx)?;
        let wanted = agent::ids(args, "ids");
        scenes(editor, &wanted)?;
        // Human: an export takes the selection; the layers the model named stand in for it while the file is
        // made, and the user's selection is as it was afterwards.
        let kept = std::mem::replace(&mut editor.selection, wanted);
        let options = crate::export::Options { format, scale, ..Default::default() };
        let editor = self.tabs.get(tab).ok_or("The document was closed.")?;
        let made = crate::export::run(self.canvas.list(), editor, ctx, options, &target);
        if let Some(editor) = self.tabs.get_mut(tab) {
            editor.selection = kept;
            editor.activity.point(None, &label, Instant::now());
        }
        made?;
        Ok(text(&json!({ "exported": target.display().to_string() })))
    }

    /// What `get_state` answers: the open documents, and in detail the one the call works on (for a chat its
    /// own document, otherwise the one in front).
    fn state(&mut self, call: &Call, ctx: &egui::Context) -> Value {
        let front = self.own(call).ok();
        let measure = text_measure(ctx);
        if let Some(editor) = front.and_then(|tab| self.tabs.get_mut(tab)) {
            editor.refresh(&measure);
        }
        let documents: Vec<Value> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(tab, editor)| {
                json!({ "name": editor.title(), "file": editor.path.as_ref().map(|p| p.display().to_string()), "inFront": Some(tab) == front, "unsavedChanges": editor.dirty() })
            })
            .collect();
        let (ready, default) = self.image_services();
        let mut state = json!({
            "documents": documents,
            "front": front.and_then(|tab| self.tabs.get(tab)).map(agent::state),
        });
        tell_services(&mut state, self.prefs.get().images.assistant, crate::imagery::listed(&ready, &default));
        state
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_model_cant_name_a_network_path() {
        for text in [r"\\server\share\a.quill", "//server/share/a.quill", r"\/server/a.png"] {
            let said = full_path(&json!({ "path": text }), "path");
            assert!(said.as_ref().is_err_and(|said| said.contains("network path")), "{text}: {said:?}");
        }
        let home = std::env::temp_dir().join("a.quill");
        assert_eq!(full_path(&json!({ "path": home.display().to_string() }), "path"), Ok(Some(home)));
    }

    #[cfg(windows)]
    #[test]
    fn a_model_cant_name_a_windows_network_path() {
        for text in [r"\\?\UNC\server\share\a.quill", r"\\.\pipe\x"] {
            assert!(full_path(&json!({ "path": text }), "path").is_err(), "{text}");
        }
    }

    #[test]
    fn the_same_file_is_found_through_a_different_spelling() {
        let folder = std::env::temp_dir().join(format!("pencraft-tools-same-{}", std::process::id()));
        std::fs::create_dir_all(folder.join("sub")).unwrap();
        let file = folder.join("a.quill");
        std::fs::write(&file, b"x").unwrap();
        assert!(same_file(&file, &file));
        assert!(same_file(&file, &folder.join("sub").join("..").join("a.quill")));
        assert!(!same_file(&file, &folder.join("b.quill")));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_document_is_exported_as_a_pen_file_and_saved_only_as_a_quill_file() {
        let folder = std::env::temp_dir().join(format!("pencraft-tools-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("out")).unwrap();
        let doc = r#"{"version":"2.22","children":[{"type":"rectangle","id":"a","fill":{"type":"image","url":"./images/x.png"}}]}"#;
        let mut editor = Editor::new(pen_doc::read(doc).unwrap(), None, 7);
        editor.move_to(folder.join("site.quill")).unwrap();
        let unpacked = editor.folder().unwrap().to_path_buf();
        std::fs::create_dir_all(unpacked.join("images")).unwrap();
        std::fs::write(unpacked.join("images/x.png"), b"x").unwrap();
        editor.save().unwrap();
        editor.mark_unsaved();
        // A name without the ending gets it; the picture comes to lie beside the file; the document stays.
        let path = |name: &str| json!({ "path": folder.join("out").join(name).display().to_string() });
        let answer = export_document(&editor, &path("copy.v2"), None).unwrap();
        assert_eq!(answer, json!({ "exported": folder.join("out").join("copy.v2.pen").display().to_string() }));
        let written = std::fs::read_to_string(folder.join("out/copy.v2.pen")).unwrap();
        assert!(written.starts_with("{\n  \"version\": \"2.22\"") && written.contains("\"./images/x.png\""), "{written}");
        assert_eq!(std::fs::read(folder.join("out/images/x.png")).unwrap(), b"x");
        assert_eq!((editor.path.as_ref(), editor.folder(), editor.dirty()), (Some(&folder.join("site.quill")), Some(unpacked.as_path()), true));
        // A `.pen` file that is there is not replaced, under the name given or the name with the ending added.
        let before = std::fs::read(folder.join("out/copy.v2.pen")).unwrap();
        assert!(export_document(&editor, &path("copy.v2.pen"), None).unwrap_err().contains("is there already: `export_document` doesn't replace"));
        assert!(export_document(&editor, &path("copy.v2"), None).unwrap_err().contains("copy.v2.pen is there already"));
        assert_eq!(std::fs::read(folder.join("out/copy.v2.pen")).unwrap(), before);
        // Another picture that lies there under the name is left, and the exported file names its own copy.
        std::fs::write(folder.join("out/images/x.png"), b"another").unwrap();
        let answer = export_document(&editor, &path("copy.pen"), None).unwrap();
        assert!(answer["notes"][0].as_str().is_some_and(|note| note.contains("images/x.png") && note.contains("x-0000.png")), "{answer}");
        assert_eq!(std::fs::read(folder.join("out/images/x.png")).unwrap(), b"another");
        assert!(std::fs::read_to_string(folder.join("out/copy.pen")).unwrap().contains("\"./images/x-0000.png\""));
        // What it refuses: another kind of design file, a path that isn't whole or names no file, no path.
        let refused = |args: Value| export_document(&editor, &args, None).unwrap_err();
        assert!(refused(path("copy.quill")).contains("`save_document`") && refused(path("copy.QUILL")).contains("`save_document`"));
        assert!(refused(path("copy.pencraft")).contains("no longer written"));
        assert!(refused(path("..")).contains("names no file"));
        assert!(refused(json!({ "path": "copy.pen" })).contains("full path") && refused(json!({})).contains("needs the `path`"));
        assert!(!folder.join("out/copy.quill").exists() && !folder.join("out/copy.pencraft").exists());
        // `save_document` saves a `.quill` file and nothing else.
        assert_eq!(save_path(folder.join("home.v2")), Ok(folder.join("home.v2.quill")));
        assert_eq!(save_path(folder.join("home.QUILL")), Ok(folder.join("home.QUILL")));
        assert!(save_path(folder.join("home.pen")).is_err_and(|said| said.contains("`export_document`") && said.contains(".quill")));
        assert!(save_path(folder.join("home.pencraft")).is_err_and(|said| said.contains("no longer written")));
        assert!(save_path(folder.join("..")).is_err_and(|said| said.contains("names no file")));
        editor.close();
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn get_state_lists_no_image_service_when_the_assistant_may_not_make_pictures() {
        let listed = json!([{ "name": "Studio" }]);
        let mut on = json!({ "documents": [] });
        tell_services(&mut on, true, listed.clone());
        assert_eq!(on, json!({ "documents": [], "imageServices": listed }));
        let mut off = json!({ "documents": [] });
        tell_services(&mut off, false, listed);
        assert_eq!(off, json!({ "documents": [], "imageServices": [], "imageServicesNote": SERVICES_OFF }));
        assert_eq!(SERVICES_OFF, "Generating and editing pictures is turned off for AI models in Quill's settings.");
    }
}
