//! The app's side of making pictures: `generate_image` calls that wait for their picture, the dialog's own
//! picture, and putting a finished one into the document.

use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use pen_doc::model::Kind as LayerKind;
use serde_json::{Value, json};

use super::App;
use super::tools::BUSY_PATIENCE;
use crate::agent;
use crate::editor::Editor;
use crate::imagery;
use crate::imagery::dialog::{Dialog, Did};
use crate::imagery::jobs::{self, Done, Job, Order};
use crate::imagery::store::Saved;
use crate::link::Call;
use crate::palette::Command;

/// How often the app looks at pictures being made, so one that takes too long is given up on in time.
const LOOK_AGAIN: Duration = Duration::from_secs(1);
/// The most pictures being made for models at once.
// Human: the dialog's own picture is not counted: it makes one at a time (its Generate button is off while
// one is being made), so at most one more than this is ever being made, and a person is never turned away
// because models are busy.
const MOST_AT_ONCE: usize = 3;
/// Told when the dialog's picture was to fill a layer that was removed while it was made.
const LAYER_GONE: &str = "The layer was gone, so the picture was added as a new layer.";

/// A `generate_image` call that waits for its picture.
pub(super) struct Waiting {
    call: Call,
    job: Job,
    /// The id of the document the call is about ([`Editor::id`]), which stays the same while its tab moves,
    /// its file is saved under a new name, or another tab comes to the front.
    doc: u64,
    /// The folder of the document's file when the picture was asked for: a `./images/…` url is relative to it.
    folder: Option<PathBuf>,
    /// The picture, once it is there but its layer couldn't be filled yet (the user was dragging), and since
    /// when it has been waiting so.
    held: Option<(Done, Instant)>,
}

/// The "Generate Image…" dialog, the document it was opened for, and the picture it is having made.
#[derive(Default)]
pub(super) struct Generating {
    /// The window.
    dialog: Dialog,
    /// The id of the document the dialog was opened for ([`Editor::id`]), which stays the same while its tab
    /// moves or another tab comes to the front.
    doc: Option<u64>,
    /// The picture being made, with the folder of the document's file when it was asked for (a `./images/…`
    /// url is relative to it).
    job: Option<(Job, Option<PathBuf>)>,
}

impl Generating {
    /// Whether the dialog shows.
    pub(super) fn is_open(&self) -> bool {
        self.dialog.open
    }
}

/// What becomes of a finished picture that is to go on a layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hold {
    /// It is put on its layer now.
    Fill,
    /// It waits for the user's drag to end.
    Keep,
    /// The drag went on too long: the model gets the picture, and the layer is left alone.
    LeaveLayer,
}

/// What becomes of a finished picture for a layer, given whether the user is `dragging` on the canvas and
/// `since` when the picture has waited for that drag to end.
// Human: a model's own change waits as long for a drag (`BUSY_PATIENCE`); after that the model is told why its
// layer wasn't changed, and can put the picture in itself.
// Agent: a job ends within its 100 s limit (`jobs::LIMIT`); 10 s more of waiting here stays under the 120 s
// the local server waits for an answer (`link::PATIENCE`), so the model always hears of its picture.
fn hold(dragging: bool, since: Instant, now: Instant) -> Hold {
    if !dragging {
        Hold::Fill
    } else if now.saturating_duration_since(since) < BUSY_PATIENCE {
        Hold::Keep
    } else {
        Hold::LeaveLayer
    }
}

/// Whether another picture can be started for a model while `making` are being made for models; the dialog's
/// own picture is not among them (see [`MOST_AT_ONCE`]).
fn room(making: usize) -> Result<(), String> {
    if making < MOST_AT_ONCE { Ok(()) } else { Err(format!("Pencraft is already making {MOST_AT_ONCE} pictures. Wait for one to finish.")) }
}

/// The url a fill is to give for the picture `saved`: as it was made when the document's folder is still the
/// one it was made for (`started`), else the picture's full path, which goes on working wherever the
/// document is saved. `now` is the document's folder now: `None` when it has no file or was closed.
fn lasting_url(saved: &Saved, started: Option<&Path>, now: Option<&Path>) -> String {
    if started == now { saved.url.clone() } else { saved.path.to_string_lossy().replace('\\', "/") }
}

/// The `advice` in an `edit_layers` answer's content, if it has any.
fn advice_in(content: &[Value]) -> Option<Value> {
    content
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .filter_map(|text| serde_json::from_str::<Value>(text).ok())
        .find_map(|answer| answer.get("advice").cloned())
}

/// Whether the layer `id` can be given a picture as its fill.
fn takes_picture(editor: &Editor, id: &str) -> Result<(), String> {
    let Some(node) = pen_doc::edit::find(&editor.doc, id) else {
        // Human: `instanceId/layerId` is a layer inside a component instance, which is changed by an override.
        let instance = id.split_once('/').and_then(|(instance, _)| pen_doc::edit::find(&editor.doc, instance));
        if instance.is_some_and(|instance| matches!(instance.kind, LayerKind::Ref)) {
            return Err(format!(
                "\"{id}\" is a layer inside a component instance: a picture can't be put on it this way. Leave out `layer`, then set its fill with edit_layers, using the `url` of the picture."
            ));
        }
        return Err(format!("There is no layer with the id \"{id}\"."));
    };
    if matches!(node.kind, LayerKind::Rectangle | LayerKind::Ellipse | LayerKind::Frame) {
        Ok(())
    } else {
        Err(format!("The layer \"{id}\" is a {}: a picture goes on a rectangle, an ellipse or a frame.", node.kind.name()))
    }
}

/// Where the dialog's picture goes in `editor`: the layer `target` it was to fill while that is still there,
/// else a new layer (`None`); and what to tell the person when the layer it was for is gone.
fn placing(target: Option<String>, editor: &Editor) -> (Option<String>, Option<&'static str>) {
    match target {
        Some(layer) if pen_doc::edit::find(&editor.doc, &layer).is_some() => (Some(layer), None),
        Some(_) => (None, Some(LAYER_GONE)),
        None => (None, None),
    }
}

/// The `edit_layers` arguments that give the layer `id` the picture at `url` as its fill.
fn fill_edit(id: &str, url: &str) -> Value {
    json!({
        "description": "Put the generated picture on the layer",
        "operations": [{ "op": "update", "id": id, "properties": { "fill": { "type": "image", "url": url, "mode": "cover" } } }]
    })
}

/// Gives the layer `id` the picture at `url` as its fill, as one undo step.
// Human: the same order as a model's change (`App::edit_in`): the step is kept only when the change worked.
// `checkpoint` marks the document changed and in need of layout.
pub(super) fn fill_with(editor: &mut Editor, id: &str, url: &str) -> Result<(), String> {
    let before = editor.doc.clone();
    agent::edit(editor, &fill_edit(id, url))?;
    let after = std::mem::replace(&mut editor.doc, before);
    editor.checkpoint();
    editor.doc = after;
    Ok(())
}

impl App {
    /// The image services that can be asked right now, and the settings' default.
    pub(super) fn image_services(&self) -> (Vec<imagery::Ready>, String) {
        let images = &self.prefs.get().images;
        (imagery::ready(images, &|name| std::env::var(name).ok()), images.default.clone())
    }

    /// Starts a `generate_image` call: answers at once when it can't be done, otherwise keeps it until its
    /// picture is there.
    pub(super) fn start_picture(&mut self, call: Call, ctx: &egui::Context) {
        match self.picture_order(&call).and_then(|ordered| room(self.picturing.len()).map(|()| ordered)) {
            Ok((order, doc)) => {
                let folder = order.document.as_deref().and_then(Path::parent).map(Path::to_path_buf);
                let wake = ctx.clone();
                let job = Job::start(move |flags| jobs::work(&order, flags), move || wake.request_repaint());
                self.picturing.push(Waiting { call, job, doc, folder, held: None });
                ctx.request_repaint_after(LOOK_AGAIN);
            }
            Err(why) => {
                // A client that stopped waiting has gone; there is no one to tell.
                let _ = call.reply.send(Err(why));
            }
        }
    }

    /// What a `generate_image` call asks for, and the id of the document it is about.
    // Human: the document is found the way every other tool finds it (`App::document`: the one the call
    // names, else its chat's own, else the one in front), once, now; the picture goes beside its file. That
    // there is no service is said before anything about the document, so a model isn't sent to open one first.
    fn picture_order(&self, call: &Call) -> Result<(Order, u64), String> {
        if !self.prefs.get().images.assistant {
            return Err("Generating pictures is turned off for AI models in Pencraft's settings (Images: \"Let the assistant generate pictures\")."
                .to_owned());
        }
        let args = &call.arguments;
        let (ready, default) = self.image_services();
        let saved = &self.prefs.get().images.services;
        let chosen = imagery::pick(&ready, saved, args.get("service").and_then(Value::as_str), &default)?.clone();
        let tab = self.document(call)?;
        let editor = self.tabs.get(tab).ok_or("The document was closed.")?;
        if let Some(layer) = args.get("layer").and_then(Value::as_str) {
            takes_picture(editor, layer)?;
        }
        let request = imagery::request(args, &chosen)?;
        Ok((Order { ready: chosen, request, document: editor.path.clone() }, editor.id))
    }

    /// Answers the `generate_image` calls whose picture is there, or failed, or took too long.
    // Human: a picture that is to fill a layer waits while the user is dragging something on the canvas, as a
    // model's own change does (`App::serve_tools`); it is put in when the drag has ended, or handed to the
    // model without touching the layer when the drag goes on too long (`hold`). Stopping a chat stops the
    // pictures it asked for.
    // Agent: MAIN-THREAD; runs from `serve_tools`, so also while the window is hidden. A kept picture is not
    // polled again: its job's result has already been taken.
    pub(super) fn finish_pictures(&mut self, ctx: &egui::Context) {
        // Agent: a call from a chat that is gone or no longer running is dropped unanswered: its run has
        // ended, so nobody reads the reply. Dropping it drops its `Job`, which marks the job as given up, so
        // the picture is not kept: not saved, or removed again when it was saved already (`jobs::Job`).
        let talks = &self.talks;
        self.picturing.retain(|waiting| waiting.call.chat.is_none_or(|id| talks.iter().any(|talk| talk.id == id && talk.chat.running())));
        if self.picturing.is_empty() {
            return;
        }
        let now = Instant::now();
        for mut waiting in std::mem::take(&mut self.picturing) {
            let (result, since) = match waiting.held.take() {
                Some((done, since)) => (Some(Ok(done)), since),
                None => (waiting.job.poll(), now),
            };
            let to_layer = waiting.call.arguments.get("layer").and_then(Value::as_str).is_some();
            match result {
                None => self.picturing.push(waiting),
                Some(Ok(done)) => match if to_layer { hold(self.canvas.dragging(), since, now) } else { Hold::Fill } {
                    Hold::Keep => {
                        waiting.held = Some((done, since));
                        self.picturing.push(waiting);
                    }
                    ending => {
                        let answer = self.picture_answer(&waiting, &done, ending == Hold::LeaveLayer, ctx);
                        // A client that stopped waiting has gone; there is no one to tell.
                        let _ = waiting.call.reply.send(Ok(answer));
                    }
                },
                Some(Err(why)) => {
                    // A client that stopped waiting has gone; there is no one to tell.
                    let _ = waiting.call.reply.send(Err(why));
                }
            }
        }
        if !self.picturing.is_empty() {
            ctx.request_repaint_after(LOOK_AGAIN);
        }
    }

    /// Why the layer the call named is not to get its picture, when it isn't: a switch was turned off while
    /// the picture was made, or the user's drag went on too long (`dragged`).
    fn layer_left(&self, dragged: bool) -> Option<&'static str> {
        let prefs = self.prefs.get();
        if !prefs.agents.tools {
            Some("AI tools were turned off in Pencraft's settings while the picture was being made, so the layer was left alone.")
        } else if !prefs.images.assistant {
            Some(
                "Generating pictures was turned off for AI models in Pencraft's settings while the picture was being made, so the layer was left alone.",
            )
        } else if dragged {
            Some("The user was moving something on the canvas, so the layer was left alone. Put the `url` in a fill with edit_layers.")
        } else {
            None
        }
    }

    /// What a model is told about its picture; gives it to the layer the call named first, unless the user
    /// dragged too long (`dragged`) or a switch was turned off meanwhile.
    fn picture_answer(&mut self, waiting: &Waiting, done: &Done, dragged: bool, ctx: &egui::Context) -> Vec<Value> {
        // Human: a `./images/…` url is relative to the document's folder when the picture was asked for; a
        // document saved elsewhere since, or closed, gets the picture's full path instead.
        let now = self.tabs.iter().find(|editor| editor.id == waiting.doc).and_then(|editor| editor.path.as_deref().and_then(Path::parent));
        let url = lasting_url(&done.saved, waiting.folder.as_deref(), now);
        let mut told = json!({
            "url": url,
            "width": done.saved.width,
            "height": done.saved.height,
            "service": done.service,
            "model": done.model,
        });
        if let Some(layer) = waiting.call.arguments.get("layer").and_then(Value::as_str) {
            if let Some(why) = self.layer_left(dragged) {
                told["layerProblem"] = json!(why);
            } else {
                match self.fill_picture(waiting, layer, &url, ctx) {
                    Ok(advice) => {
                        told["layer"] = json!(layer);
                        if let Some(advice) = advice {
                            told["advice"] = advice;
                        }
                    }
                    Err(why) => {
                        told["layerProblem"] =
                            json!(format!("The picture was made, but the layer wasn't changed: {why} Use the `url` in a fill yourself."));
                    }
                }
            }
        } else {
            told["next"] = json!("Nothing in the document changed. Put the `url` in an image fill with edit_layers.");
        }
        vec![
            json!({ "type": "image", "data": agent::base64(&done.shown), "mimeType": "image/png" }),
            json!({ "type": "text", "text": serde_json::to_string_pretty(&told).unwrap_or_default() }),
        ]
    }

    /// Gives the layer `layer` of the waiting call's document the picture at `url` as its fill; the `advice`
    /// the change came with, if any.
    // Human: the fill is set the way a model sets one itself, so the layer flashes, the model's cursor goes to
    // it, ⌘Z takes it back, and what the app would not leave that way is said.
    // Agent: the tab is found by the document's id, never by its name, which two tabs can share ("Untitled").
    // The inner call comes from the same client and chat as the waiting one.
    fn fill_picture(&mut self, waiting: &Waiting, layer: &str, url: &str, ctx: &egui::Context) -> Result<Option<Value>, String> {
        let tab = self.tabs.iter().position(|tab| tab.id == waiting.doc).ok_or("Its document was closed.")?;
        let (reply, _unheard) = channel();
        let edit = Call {
            tool: "edit_layers".to_owned(),
            arguments: fill_edit(layer, url),
            client: waiting.call.client.clone(),
            chat: waiting.call.chat,
            reply,
        };
        self.edit_in(tab, &edit, ctx).map(|content| advice_in(&content))
    }

    /// Opens the "Generate Image…" dialog for the document in front.
    // Human: with exactly one layer selected that can take a picture, the dialog offers to fill it. The
    // settings' switch for the assistant ("Let the assistant generate pictures") doesn't apply here: a person
    // may always make a picture. While one is being made, the dialog shows again as it is, for its own document.
    pub(super) fn open_picture_dialog(&mut self) {
        let Some(editor) = self.active.and_then(|tab| self.tabs.get(tab)) else { return };
        if self.generating.job.is_none() {
            self.generating.doc = Some(editor.id);
        }
        let target = match editor.selection.as_slice() {
            [only] => takes_picture(editor, only).is_ok().then(|| only.clone()),
            _ => None,
        };
        let (ready, default) = self.image_services();
        let names: Vec<String> = ready.iter().map(|r| r.name.clone()).collect();
        self.generating.dialog.open_for(target, &names, &default);
    }

    /// Draws the dialog, starts its picture, and places the picture when it is there.
    // Agent: MAIN-THREAD; called once a frame from `App::ui`.
    pub(super) fn show_picture_dialog(&mut self, ctx: &egui::Context) {
        self.generating.dialog.orphaned = self.dialog_editor().is_none();
        match self.generating.dialog.show(ctx, crate::theme::palette(self.dark())) {
            Did::Nothing => {}
            Did::Cancel => {
                // Human: a picture already asked for is given up on (only the Cancel button does that while one
                // is being made). Dropping its job means no file and no credit of it stay in `images/`: one not
                // saved yet is never saved, one saved already is removed again (`jobs::Job`), unless Pencraft
                // quits during the save. The service may still make it, and charge for it.
                self.generating.job = None;
                self.generating.dialog.open = false;
                self.generating.dialog.busy = false;
            }
            Did::OpenSettings => {
                self.generating.dialog.open = false;
                self.settings.open_named("images");
            }
            Did::Generate => match self.dialog_order() {
                Ok(order) => {
                    let folder = order.document.as_deref().and_then(Path::parent).map(Path::to_path_buf);
                    let wake = ctx.clone();
                    self.generating.job = Some((Job::start(move |flags| jobs::work(&order, flags), move || wake.request_repaint()), folder));
                    self.generating.dialog.busy = true;
                    self.generating.dialog.problem = None;
                }
                Err(why) => self.generating.dialog.problem = Some(why),
            },
        }
        let Some((job, _)) = &self.generating.job else { return };
        let Some(result) = job.poll() else {
            ctx.request_repaint_after(LOOK_AGAIN);
            return;
        };
        let folder = self.generating.job.take().and_then(|(_, folder)| folder);
        self.generating.dialog.busy = false;
        match result.and_then(|done| self.place_picture(&done, folder.as_deref(), ctx)) {
            Ok(note) => {
                self.generating.dialog.open = false;
                if let Some(note) = note {
                    self.say(note);
                }
            }
            Err(why) => {
                self.generating.dialog.problem = Some(why);
                // Human: a document closed meanwhile leaves the dialog only its Close button.
                self.generating.dialog.orphaned = self.dialog_editor().is_none();
            }
        }
    }

    /// What the dialog asks for, for the document it was opened for.
    fn dialog_order(&self) -> Result<Order, String> {
        let editor = self.dialog_editor().ok_or("The document was closed.")?;
        let asked = self.generating.dialog.asked();
        let (ready, default) = self.image_services();
        let saved = &self.prefs.get().images.services;
        let chosen = imagery::pick(&ready, saved, asked.get("service").and_then(Value::as_str), &default)?.clone();
        let request = imagery::request(&asked, &chosen)?;
        Ok(Order { ready: chosen, request, document: editor.path.clone() })
    }

    /// The document the dialog was opened for, if it is still open.
    fn dialog_editor(&self) -> Option<&Editor> {
        self.tabs.iter().find(|editor| Some(editor.id) == self.generating.doc)
    }

    /// Puts the dialog's finished picture into its document: onto the layer it was opened for, or as a new
    /// layer in the middle of the view, selected; then brings the document's tab to the front. Either is one
    /// undo step. `started` is the folder of the document's file when the picture was asked for. What to tell
    /// the person besides, if anything.
    // Human: both go through `checkpoint`, which marks the document changed and in need of layout; it is laid
    // out at once, so the picture shows without waiting for the canvas. A document saved elsewhere meanwhile
    // gets the picture's full path in its fill (`lasting_url`); a new layer names the file from where the
    // document is now (`Editor::import_file`). A layer removed while the picture was made doesn't lose the
    // picture, which was paid for: it becomes a new layer instead, and the person is told.
    // Agent: the tab is found by the document's id, never by its place: tabs can be closed or moved while the
    // picture is made.
    fn place_picture(&mut self, done: &Done, started: Option<&Path>, ctx: &egui::Context) -> Result<Option<&'static str>, String> {
        let view = self.canvas_view;
        let target = self.generating.dialog.target.clone().filter(|_| self.generating.dialog.fill);
        let doc = self.generating.doc;
        let kept = done.saved.path.display().to_string();
        let (tab, editor) = self
            .tabs
            .iter_mut()
            .enumerate()
            .find(|(_, editor)| Some(editor.id) == doc)
            .ok_or_else(|| format!("The picture was made, but its document was closed. It is kept at {kept}."))?;
        let (layer, note) = placing(target, editor);
        let placed = if let Some(layer) = layer {
            let url = lasting_url(&done.saved, started, editor.path.as_deref().and_then(Path::parent));
            fill_with(editor, &layer, &url)
        } else {
            let visible = editor.visible_area(view.width(), view.height());
            editor.import_file(&done.saved.path, visible)
        };
        editor.refresh(&super::text_measure(ctx));
        placed.map_err(|why| format!("The picture was made, but it couldn't be placed: {why} It is kept at {kept}."))?;
        // Human: the person sees where the picture went: its tab comes to the front as choosing it does.
        self.run(Command::ShowTab(tab), ctx);
        Ok(note)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn editor() -> Editor {
        let doc = pen_doc::read(
            r##"{"version":"2.22","children":[
              {"type":"rectangle","id":"hero","x":0,"y":0,"width":400,"height":300,"fill":"#eeeeee"},
              {"type":"text","id":"title","x":20,"y":20,"content":"Hello","fill":"#000000"},
              {"type":"frame","id":"card","reusable":true,"x":500,"y":0,"width":100,"height":100,"layout":"none","children":[
                {"type":"rectangle","id":"photo","width":100,"height":60}]},
              {"type":"ref","id":"inst","ref":"card","x":500,"y":200}]}"##,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 7);
        editor.refresh(&|_| (10.0, 10.0));
        editor
    }

    #[test]
    fn only_a_shape_or_a_frame_takes_a_picture() {
        let editor = editor();
        assert_eq!(takes_picture(&editor, "hero"), Ok(()));
        assert_eq!(
            takes_picture(&editor, "title"),
            Err("The layer \"title\" is a text: a picture goes on a rectangle, an ellipse or a frame.".to_owned())
        );
        assert_eq!(takes_picture(&editor, "gone"), Err("There is no layer with the id \"gone\".".to_owned()));
        assert_eq!(takes_picture(&editor, "gone/photo"), Err("There is no layer with the id \"gone/photo\".".to_owned()));
    }

    #[test]
    fn a_layer_inside_an_instance_is_sent_to_edit_layers() {
        let editor = editor();
        assert_eq!(
            takes_picture(&editor, "inst/photo"),
            Err("\"inst/photo\" is a layer inside a component instance: a picture can't be put on it this way. Leave out `layer`, then set its fill with edit_layers, using the `url` of the picture.".to_owned())
        );
    }

    #[test]
    fn filling_a_layer_is_one_undo_step() {
        let mut editor = editor();
        fill_with(&mut editor, "hero", "./images/a.png").unwrap();
        let fill = |editor: &Editor| serde_json::to_value(&pen_doc::edit::find(&editor.doc, "hero").unwrap().fill).unwrap();
        assert_eq!(fill(&editor), json!({ "type": "image", "url": "./images/a.png", "mode": "cover" }));
        assert!(fill_with(&mut editor, "gone", "./images/a.png").is_err());
        // Human: the undo stack is private to the editor, so the one step is checked by what undo does.
        editor.undo();
        assert_eq!(fill(&editor), json!("#eeeeee"));
    }

    #[test]
    fn a_picture_for_a_layer_that_is_gone_becomes_a_new_layer_and_says_so() {
        let editor = editor();
        assert_eq!(placing(Some("hero".to_owned()), &editor), (Some("hero".to_owned()), None));
        assert_eq!(placing(Some("gone".to_owned()), &editor), (None, Some(LAYER_GONE)));
        assert_eq!(placing(None, &editor), (None, None));
    }

    #[test]
    fn a_picture_waits_for_a_drag_only_so_long() {
        let since = Instant::now();
        assert_eq!(hold(false, since, since), Hold::Fill);
        assert_eq!(hold(false, since, since + BUSY_PATIENCE * 2), Hold::Fill);
        assert_eq!(hold(true, since, since), Hold::Keep);
        assert_eq!(hold(true, since, since + BUSY_PATIENCE.saturating_sub(Duration::from_millis(1))), Hold::Keep);
        assert_eq!(hold(true, since, since + BUSY_PATIENCE), Hold::LeaveLayer);
        // Human: the 100 s a job may take and the 10 s a picture waits stay under the 120 s the server waits.
        assert!(jobs::LIMIT + BUSY_PATIENCE < Duration::from_secs(120));
    }

    #[test]
    fn only_so_many_pictures_are_made_at_once() {
        assert_eq!(room(0), Ok(()));
        assert_eq!(room(MOST_AT_ONCE - 1), Ok(()));
        assert_eq!(room(MOST_AT_ONCE), Err("Pencraft is already making 3 pictures. Wait for one to finish.".to_owned()));
    }

    #[test]
    fn a_url_beside_a_document_that_moved_becomes_the_full_path() {
        let saved = Saved { url: "./images/a.png".to_owned(), path: PathBuf::from(r"C:\d\images\a.png"), width: 4, height: 2 };
        let folder = Path::new(r"C:\d");
        assert_eq!(lasting_url(&saved, Some(folder), Some(folder)), "./images/a.png");
        // Saved elsewhere since, or closed.
        assert_eq!(lasting_url(&saved, Some(folder), Some(Path::new(r"C:\e"))), "C:/d/images/a.png");
        assert_eq!(lasting_url(&saved, Some(folder), None), "C:/d/images/a.png");
        // A picture for a document without a file has its full path already.
        let home = Saved { url: "/home/me/.pencraft/images/a.png".to_owned(), path: PathBuf::from("/home/me/.pencraft/images/a.png"), ..saved };
        assert_eq!(lasting_url(&home, None, Some(folder)), "/home/me/.pencraft/images/a.png");
        assert_eq!(lasting_url(&home, None, None), "/home/me/.pencraft/images/a.png");
    }

    #[test]
    fn the_advice_of_a_change_is_found_in_its_answer() {
        let answer = |value: &Value| vec![json!({ "type": "text", "text": serde_json::to_string_pretty(value).unwrap() })];
        assert_eq!(advice_in(&answer(&json!({ "results": [], "advice": ["Too faint."] }))), Some(json!(["Too faint."])));
        assert_eq!(advice_in(&answer(&json!({ "results": [] }))), None);
        assert_eq!(advice_in(&[json!({ "type": "image" })]), None);
    }
}
