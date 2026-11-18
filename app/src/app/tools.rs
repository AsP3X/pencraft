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
const SERVICES_OFF: &str = "Generating and editing pictures is turned off for AI models in Pencraft's settings.";
/// The longest side of a picture given to a model, pixels: larger ones are drawn at a smaller scale.
const MOST_PICTURE_SIDE: f32 = 2000.0;
/// What the model's cursor is labelled with when the settings name nothing else.
const AGENT_NAME: &str = "Agent";
/// How many parts of a design the reviewer is shown enlarged, and the fewest layers such a part holds.
const MOST_PARTS: usize = 4;
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
    let path = PathBuf::from(text);
    if path.is_absolute() { Ok(Some(path)) } else { Err(format!("`{key}` must be a full path, starting at the root: \"{text}\" isn't.")) }
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
            let on = self.prefs.get().agents.tools;
            // Human: a tool the user turned off in the settings is neither offered nor run.
            if on && self.prefs.get().agents.tools_off().contains(&call.tool) {
                let refused = format!("The tool `{}` is turned off in Pencraft's settings (MCP). Go on without it.", call.tool);
                // A client that stopped waiting has gone; there is no one to tell.
                let _ = call.reply.send(Err(refused));
                continue;
            }
            // Human: the web can be slow; its tools answer from a thread of their own, and the window goes on
            // meanwhile. A picture goes beside the document the call is about.
            if on && crate::web::is_web(&call.tool) {
                let file = self.document(&call).ok().and_then(|tab| self.tabs.get(tab)).and_then(|editor| editor.path.clone());
                let wake = ctx.clone();
                let keyed = self.prefs.get().agents.picture_keys.clone();
                crate::web::answer(call, file, keyed, move || wake.request_repaint());
                continue;
            }
            // Human: a picture takes up to a minute, made or changed; its call is kept and answered when the
            // picture is there.
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
                Err("AI tools are turned off in Pencraft's settings (Agents: \"Let AI models work on documents\").".to_owned())
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
                let was = editor.path.clone();
                if let Some(path) = path {
                    editor.path =
                        Some(if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pen")) { path } else { path.with_extension("pen") });
                }
                // Human: as when the person saves: the pictures come to where the document is saved.
                let stuck = super::pictures::settle(editor, was.as_deref(), crate::imagery::store::home().as_deref());
                editor.save().map_err(|error| format!("Couldn't save: {error}. A document without a file needs `path`."))?;
                let saved = editor.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
                self.prefs.update(|p| p.files.touch(&saved));
                self.remember_tabs();
                let mut answer = json!({ "saved": saved });
                if !stuck.is_empty() {
                    answer["picturesNotBrought"] = json!(stuck);
                }
                Ok(text(&answer))
            }
            "read_layers" => self.read_tool(call, ctx),
            "edit_layers" => self.edit_tool(call, ctx),
            "screenshot" => self.picture_tool(call, ctx),
            "check_glass" => self.glass_tool(call, ctx),
            crate::chat::critic::TOOL => self.review_tool(call, ctx),
            "select_layers" => self.select_tool(call, ctx),
            "export_layers" => self.export_tool(call, ctx),
            other => Err(format!("There is no tool called \"{other}\".")),
        }
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
        let read = agent::read(editor, args)?;
        // The cursor goes to the layers the model reads by name.
        let looked = scenes(editor, &agent::ids(args, "ids")).ok().filter(|_| args.get("ids").is_some()).and_then(|layers| around(&layers));
        editor.activity.point(looked, &label, Instant::now());
        Ok(text(&read))
    }

    /// `edit_layers`: the change a call asks for, in the document it is about.
    fn edit_tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let tab = self.document(call)?;
        self.edit_in(tab, call, ctx)
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
        let folder = editor.path.as_deref().and_then(Path::parent);
        let picture =
            self.canvas.list().snapshot(&layers, scale, Vars::new(&editor.doc), ctx, folder).ok_or("The layers are empty or too large to draw.")?;
        let note = format!(
            "{} × {} pixels at {scale:.2} pixels per unit, showing the canvas from ({:.0}, {:.0}), {:.0} × {:.0} units.",
            picture.w, picture.h, bounds.x, bounds.y, bounds.w, bounds.h
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
        let mut notes = crate::review::review(&layers, Vars::new(&editor.doc), &stands_in, follows);
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
        answer.extend(crate::craft::with_picture(&notes, crate::review::size(&layers)).map(|said| json!({ "type": "text", "text": said })));
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
        let folder = editor.path.as_deref().and_then(Path::parent);
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
        let Some(editor) = self.tabs.iter_mut().find(|tab| tab.id == doc) else { return Ok(Vec::new()) };
        editor.refresh(&text_measure(ctx));
        let ids: Vec<String> = designed.iter().filter(|(_, id, _)| editor.scene_node(id).is_some()).map(|(_, id, _)| id.clone()).collect();
        let written: usize = designed.iter().map(|(_, _, count)| count).sum();
        // Human: a design that was reviewed and changed is looked at again however little was changed.
        let again = call.arguments.get("again").and_then(Value::as_bool) == Some(true);
        if ids.is_empty() || written < if again { 1 } else { WORTH_A_REVIEW } {
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
                json!({ "name": editor.title(), "file": editor.path.as_ref().map(|p| p.display().to_string()), "inFront": Some(tab) == front, "unsavedChanges": editor.dirty })
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
    fn get_state_lists_no_image_service_when_the_assistant_may_not_make_pictures() {
        let listed = json!([{ "name": "Studio" }]);
        let mut on = json!({ "documents": [] });
        tell_services(&mut on, true, listed.clone());
        assert_eq!(on, json!({ "documents": [], "imageServices": listed }));
        let mut off = json!({ "documents": [] });
        tell_services(&mut off, false, listed);
        assert_eq!(off, json!({ "documents": [], "imageServices": [], "imageServicesNote": SERVICES_OFF }));
        assert_eq!(SERVICES_OFF, "Generating and editing pictures is turned off for AI models in Pencraft's settings.");
    }
}
