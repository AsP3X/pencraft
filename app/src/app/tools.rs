//! Doing what an AI model asks for through the tools of [`crate::agent`]: the calls the local server
//! ([`crate::link`]) hands over are answered here, on the main thread, where the documents are.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pen_doc::geom::Rect;
use pen_doc::layout;
use pen_doc::scene::SceneNode;
use pen_doc::vars::Vars;
use serde_json::{Value, json};

use super::{App, text_measure};
use crate::agent;
use crate::editor::Editor;
use crate::link::Call;

/// How long a change waits for the user to finish a drag on the canvas before the model is told to try again.
const BUSY_PATIENCE: Duration = Duration::from_secs(10);
/// The longest side of a picture given to a model, pixels: larger ones are drawn at a smaller scale.
const MOST_PICTURE_SIDE: f32 = 2000.0;
/// How many parts of a design the reviewer is shown enlarged, and the fewest layers such a part holds.
const MOST_PARTS: usize = 4;
/// See [`MOST_PARTS`].
const PART_LAYERS: usize = 8;
/// The fewest layers a model has to have written since the last review for a reviewer to be asked.
const WORTH_A_REVIEW: usize = 30;
/// How long a call waits for the faces of a document's text to be made ready.
const FACE_PATIENCE: Duration = Duration::from_secs(3);
/// The tools that change a document.
const CHANGING: [&str; 2] = ["edit_layers", "select_layers"];

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
fn holds(scene: &SceneNode, id: &str) -> bool {
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
        let Ok(tab) = self.document(&call.arguments) else { return false };
        let Some(editor) = self.tabs.get_mut(tab) else { return false };
        editor.refresh(&text_measure(ctx));
        crate::fonts::pending()
    }

    /// The tab a call is about: the one its `document` names (by path, file name or title), or the one in front.
    fn document(&self, args: &Value) -> Result<usize, String> {
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
        self.active.or((!self.tabs.is_empty()).then_some(0)).ok_or_else(|| "No document is open. Open one with open_document.".to_owned())
    }

    /// The name shown beside the model's cursor, as the settings want it.
    fn cursor_label(&self, client: &str) -> String {
        if self.prefs.get().agents.cursor_label == "None" { String::new() } else { client.to_owned() }
    }

    /// Does one tool call and gives its answer's content.
    fn tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let args = &call.arguments;
        match call.tool.as_str() {
            "get_guide" => Ok(vec![json!({ "type": "text", "text": crate::craft::guide() })]),
            "list_skills" => Ok(text(&crate::skills::list())),
            "read_skill" => Ok(vec![json!({ "type": "text", "text": crate::skills::read(args)? })]),
            "get_state" => Ok(text(&self.state(ctx))),
            "open_document" => {
                match full_path(args, "path")? {
                    Some(path) => self.try_open(&path)?,
                    None => self.new_file(),
                }
                Ok(text(&self.state(ctx)))
            }
            "save_document" => {
                let tab = self.document(args)?;
                let path = full_path(args, "path")?;
                let editor = self.tabs.get_mut(tab).ok_or("The document was closed.")?;
                if let Some(path) = path {
                    editor.path =
                        Some(if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pen")) { path } else { path.with_extension("pen") });
                }
                editor.save().map_err(|error| format!("Couldn't save: {error}. A document without a file needs `path`."))?;
                let saved = editor.path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
                self.prefs.update(|p| p.files.touch(&saved));
                self.remember_tabs();
                Ok(text(&json!({ "saved": saved })))
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
        let tab = self.document(&call.arguments)?;
        let label = self.cursor_label(&call.client);
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

    // Human: one call is one undo step, made only when the call worked. Every layer it touched flashes, and the
    // model's cursor goes to them.
    fn edit_tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let args = &call.arguments;
        let (tab, _, label) = self.laid_out(call, ctx)?;
        let editing = self.canvas.editing.clone();
        let view = self.canvas_view;
        let editor = self.tabs.get_mut(tab).ok_or("The document was closed.")?;
        let before = editor.doc.clone();
        let (results, touched) = agent::edit(editor, args)?;
        let after = std::mem::replace(&mut editor.doc, before);
        editor.checkpoint();
        editor.doc = after;
        editor.refresh(&text_measure(ctx));
        let touched: Vec<String> = touched.into_iter().filter(|id| editor.scene_node(id).is_some()).collect();
        let selection: Vec<String> = editor.selection.iter().filter(|id| editor.scene_node(id).is_some()).cloned().collect();
        editor.selection = selection;
        let now = Instant::now();
        let place = touched.iter().filter_map(|id| editor.scene_node(id)).map(layout::doc_bounds).reduce(|a, b| a.union(&b));
        editor.activity.flash(&touched, now);
        editor.activity.point(place, &label, now);
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
            match self.designed.iter_mut().find(|(at, id, _)| *at == tab && *id == top.id) {
                Some((_, _, count)) => *count += layers,
                None => self.designed.push((tab, top.id.clone(), layers)),
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
        if editing.is_some_and(|id| editor.scene_node(&id).is_none()) {
            self.canvas.editing = None;
        }
        // Human: while the chat's own assistant works, its note in the chat (and in the one line over the
        // message bar) already says what a change is; a second message over it would only be in the way.
        if let Some(said) = args.get("description").and_then(Value::as_str).filter(|said| !said.trim().is_empty() && !self.chat.running()) {
            let said = format!("{}: {}", call.client, said.trim());
            self.say(said);
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
        let notes = crate::review::review(&layers, Vars::new(&editor.doc), &stands_in);
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

    /// The reviewer's picture: the top-level layers changed since it last looked, with what the app counted in
    /// them. Nothing when little was written: a new headline or one more row is no design to review, and the
    /// user who asked for it didn't ask for the page around it to be judged.
    fn review_tool(&mut self, call: &Call, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let designed = std::mem::take(&mut self.designed);
        let Some((tab, _, _)) = designed.last() else { return Ok(Vec::new()) };
        let Some(editor) = self.tabs.get_mut(*tab) else { return Ok(Vec::new()) };
        editor.refresh(&text_measure(ctx));
        let ids: Vec<String> =
            designed.iter().filter(|(at, id, _)| at == tab && editor.scene_node(id).is_some()).map(|(_, id, _)| id.clone()).collect();
        let written: usize = designed.iter().filter(|(at, _, _)| at == tab).map(|(_, _, count)| count).sum();
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
            Call { tool: "screenshot".to_owned(), arguments, client: call.client.clone(), reply }
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
        self.active = Some(tab);
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

    /// What `get_state` answers: the open documents, and the one in front in detail.
    fn state(&mut self, ctx: &egui::Context) -> Value {
        let front = self.active.or((!self.tabs.is_empty()).then_some(0));
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
        json!({ "documents": documents, "front": front.and_then(|tab| self.tabs.get(tab)).map(agent::state) })
    }
}
