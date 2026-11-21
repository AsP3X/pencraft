//! The app's side of making pictures: `generate_image` and `edit_image` calls that wait for their picture, the
//! dialog's own picture, and putting a finished one into the document.

use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

use pen_doc::geom::Rect;
use pen_doc::layout;
use pen_doc::model::{Kind as LayerKind, Paint, Paints};
use pen_doc::vars::Vars;
use serde_json::{Value, json};

use super::App;
use super::talk::Talk;
use super::tools::{BUSY_PATIENCE, holds};
use crate::agent;
use crate::editor::Editor;
use crate::imagery::dialog::{Dialog, Did, Offered, Piece, PieceOf};
use crate::imagery::jobs::{self, Done, Job, Order};
use crate::imagery::sources::{self, Take};
use crate::imagery::store::{self, Saved};
use crate::imagery::{self, Pick};
use crate::link::Call;
use crate::palette::Command;
use crate::panels::{PictureAsk, Pictures};
use crate::theme::PIECE_PIXELS;

/// How often the app looks at pictures being made, so one that takes too long is given up on in time.
const LOOK_AGAIN: Duration = Duration::from_secs(1);
/// The most pictures being made for models at once.
// Human: the dialog's own picture is not counted: it makes one at a time (its Generate button is off while
// one is being made), so at most one more than this is ever being made, and a person is never turned away
// because models are busy.
const MOST_AT_ONCE: usize = 3;
/// Told when the dialog's picture was to fill a layer that was removed while it was made.
const LAYER_GONE: &str = "The layer was gone, so the picture was added as a new layer.";
/// How far to the right of the layers beside it a picture made from drawn layers is put, in canvas units.
const BESIDE_GAP: f32 = 40.0;
/// The most layers drawn to be sent are enlarged: a small design is drawn at up to twice its size, so its text
/// stays readable to the service.
const MOST_ENLARGED: f32 = 2.0;
/// The name of the layer a picture made from drawn layers becomes.
const BESIDE_NAME: &str = "Edited picture";
/// Said when the layers to send can't be drawn.
const NOTHING_DRAWN: &str = "The layers are empty or too large to draw, so there is nothing to send.";
/// Told when the settings' switch for the assistant is off.
const ASSISTANT_OFF: &str =
    "Generating and editing pictures is turned off for AI models in Quill's settings (Images: \"Let the assistant generate and edit pictures\").";
/// Told the person when the picture they had changed was replaced on its layer, taken off it, or its layer
/// removed, while the change was made.
const PICTURE_CHANGED: &str = "The layer's picture was changed or removed meanwhile, so the result was added as a new layer.";
/// Told when the picture an edit changed was replaced on its layer, or taken off it, while the edit was made.
const CHANGED_MEANWHILE: &str =
    "The layer's picture was changed while this one was being made, so it was left as it is. Don't put this picture there unless the user asks.";
/// How a model puts a changed picture on its layer itself.
const REPLACE_YOURSELF: &str = "replace the `url` of the layer's image paint with this one with edit_layers, keeping the rest of the fill.";
/// Told a person when a layer they picked to send is no longer in the document.
const PERSON_GONE: &str = "A layer to send is no longer in the document.";
/// Told a person when the layer whose picture they would change has no picture any more.
const PERSON_NO_PICTURE: &str = "This layer has no picture any more.";
/// Told a person when the picture to change is that of a layer inside a component instance.
const PERSON_IN_INSTANCE: &str = "The picture of a layer inside a component instance can't be changed this way. Send it as a reference instead.";

/// Where a finished picture goes, as the call was understood when it started.
// Human: what the call named is looked up once, when it starts; the arguments are not read again afterwards.
#[derive(Clone, Debug, PartialEq)]
enum Goes {
    /// Only into the answer, which tells the model to put the `url` in an image fill: a generation without
    /// `layer`, or an edit that goes by references alone.
    Answer,
    /// Only into the answer, with nothing more suggested: an edit of a picture file.
    File,
    /// Onto the layer as its fill: a generation with `layer`.
    Fill(String),
    /// In place of the picture of the layer's topmost image paint, whose `url` was `was` when the call started:
    /// an edit with `layer`.
    Replace {
        /// The layer.
        layer: String,
        /// Its image paint's `url` when the call started.
        was: String,
    },
    /// Onto a new layer beside layers drawn: an edit with `layers`.
    Beside {
        /// The box the layers drawn took when the call started.
        drawn: Rect,
        /// The box of the top-level layers they are in, then.
        tops: Rect,
    },
}

impl Goes {
    /// Whether the picture is to go onto a layer, and so waits for a drag to end.
    fn to_layer(&self) -> bool {
        matches!(self, Goes::Fill(_) | Goes::Replace { .. } | Goes::Beside { .. })
    }
}

/// A `generate_image` or `edit_image` call that waits for its picture.
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
    /// What the credit calls the pictures sent with the request, in the order sent; empty for a generation.
    from: Vec<String>,
    /// Where the picture goes.
    goes: Goes,
}

/// A picture the person asked for, in the dialog or the quick field, while it is made.
struct Making {
    /// The job making it.
    job: Job,
    /// The id of the document it is for ([`Editor::id`]).
    doc: u64,
    /// The folder of the document's file when it was asked for: a `./images/…` url is relative to it.
    folder: Option<PathBuf>,
    /// Where it goes.
    goes: Goes,
    /// The layer of the quick field that asked for it, and its name then; `None` when the dialog did.
    quick: Option<(String, String)>,
    /// The picture, once it is there but couldn't be placed yet: the person was dragging on the canvas or
    /// editing text on it.
    held: Option<Done>,
}

impl Making {
    /// Whether the quick field asked for it.
    fn is_quick(&self) -> bool {
        self.quick.is_some()
    }

    /// Whether the quick field asked for it for the layer `layer` of the document `doc`.
    fn is_quick_for(&self, layer: &str, doc: u64) -> bool {
        self.doc == doc && self.quick.as_ref().is_some_and(|(id, _)| id == layer)
    }
}

/// What to do with the person's picture this frame.
#[derive(Debug, PartialEq, Eq)]
enum Step<T> {
    /// It isn't there yet.
    Wait,
    /// It is there, but the canvas is busy (a drag, or text being edited): keep it and look again.
    Hold(T),
    /// Place it, or say why it failed.
    Place(Result<T, String>),
}

/// What becomes of the person's picture: the one `held` from before, else what the job gave this frame
/// (`polled`); a finished picture waits while the canvas is `busy`.
// Human: a person's own picture is never put into the document under a drag or while text is being typed on
// the canvas; unlike a model's, nobody waits for an answer, so it waits as long as the canvas is busy. A held
// picture is either held again or placed, never both and never dropped; a failure is said at once.
fn step<T>(held: Option<T>, polled: Option<Result<T, String>>, busy: bool) -> Step<T> {
    match held.map(Ok).or(polled) {
        None => Step::Wait,
        Some(Ok(done)) if busy => Step::Hold(done),
        Some(result) => Step::Place(result),
    }
}

/// What the message line says of the quick field's picture for the layer called `name`, once it is placed
/// (`placed`, with what else to tell) or failed: nothing while the layer is the one selected in the tab in
/// front (`shown`: the field says it), and the note alone when there is one.
fn quick_told(name: &str, shown: bool, placed: &Result<Option<&'static str>, String>) -> Option<String> {
    match placed {
        Ok(Some(note)) => Some((*note).to_owned()),
        Ok(None) | Err(_) if shown => None,
        Ok(None) => Some(format!("The picture of \"{name}\" was changed.")),
        Err(why) => Some(format!("The picture of \"{name}\" couldn't be changed: {why}")),
    }
}

/// The quick field's state for the layer whose image paint it sits under.
#[derive(Default, Debug, PartialEq, Eq)]
pub(super) struct Quick {
    /// The layer it was typed for.
    pub layer: String,
    /// The id of that layer's document ([`Editor::id`]).
    pub doc: u64,
    /// The words.
    pub text: String,
    /// What went wrong the last time.
    pub problem: Option<String>,
    /// The words and problem of the last layer left with words typed or a problem shown, kept for when it is
    /// selected again.
    pub kept: Option<Box<Quick>>,
}

impl Quick {
    /// Ties the field to the layer `layer` of the document `doc`: another layer than the one it was typed for
    /// starts it empty, or with what was kept for it. What was typed for the layer left is kept.
    // Human: a click on another layer and back must not throw away half a sentence; one layer's words are kept
    // (the last left), and are dropped once its picture has been changed (`App::finish_making`).
    fn tie(&mut self, layer: &str, doc: u64) {
        if self.layer == layer && self.doc == doc {
            return;
        }
        let (back, earlier) = match self.kept.take() {
            Some(kept) if kept.layer == layer && kept.doc == doc => (Some(kept), None),
            earlier => (None, earlier),
        };
        let left = Quick {
            layer: std::mem::take(&mut self.layer),
            doc: self.doc,
            text: std::mem::take(&mut self.text),
            problem: self.problem.take(),
            kept: None,
        };
        let kept = if left.text.trim().is_empty() && left.problem.is_none() { earlier } else { Some(Box::new(left)) };
        *self = match back {
            Some(back) => Quick { kept, ..*back },
            None => Quick { layer: layer.to_owned(), doc, kept, ..Quick::default() },
        };
    }

    /// Forgets the words and problem of the layer `layer` of the document `doc`, shown or kept: its picture was
    /// changed.
    fn done_with(&mut self, layer: &str, doc: u64) {
        if self.layer == layer && self.doc == doc {
            self.text.clear();
            self.problem = None;
        }
        if self.kept.as_ref().is_some_and(|kept| kept.layer == layer && kept.doc == doc) {
            self.kept = None;
        }
    }

    /// Shows `why` with the words of the layer `layer` of the document `doc`, shown or kept.
    fn failed(&mut self, layer: &str, doc: u64, why: String) {
        if self.layer == layer && self.doc == doc {
            self.problem = Some(why);
        } else if let Some(kept) = self.kept.as_mut().filter(|kept| kept.layer == layer && kept.doc == doc) {
            kept.problem = Some(why);
        }
    }
}

/// The "Generate Image…" dialog, the document it was opened for, the quick field, and the one picture the
/// person is having made by either.
// Human: a person makes one picture at a time: the dialog and the quick field share `job`, and each is off
// while the other's picture is being made.
#[derive(Default)]
pub(super) struct Generating {
    /// The window.
    dialog: Dialog,
    /// The id of the document the dialog was opened for ([`Editor::id`]), which stays the same while its tab
    /// moves or another tab comes to the front.
    doc: Option<u64>,
    /// The picture being made.
    job: Option<Making>,
    /// The quick field on an image fill.
    quick: Quick,
    /// The small pictures of files in the dialog's row being made, each with the file it is of.
    thumbs: Vec<(PathBuf, Receiver<Option<egui::ColorImage>>)>,
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
    if making < MOST_AT_ONCE { Ok(()) } else { Err(format!("Quill is already making {MOST_AT_ONCE} pictures. Wait for one to finish.")) }
}

/// Whether anybody still waits for the answer to a call from the chat `chat`: a call from outside the app
/// (`None`), or from a chat that is there and running.
// Human: a call from a chat that was stopped or closed is dropped unanswered: its run has ended, so nobody
// reads the reply, and no picture is made or kept for it.
fn heard(talks: &[Talk], chat: Option<u64>) -> bool {
    chat.is_none_or(|id| talks.iter().any(|talk| talk.id == id && talk.chat.running()))
}

/// The url a fill is to give for the picture `saved`: as it was made when the document's folder is still the
/// one it was made for (`started`), else the picture's full path, which goes on working wherever the
/// document is saved. `now` is the document's folder now: `None` when it has no file or was closed.
fn lasting_url(saved: &Saved, started: Option<&Path>, now: Option<&Path>) -> String {
    if started == now { saved.url.clone() } else { saved.path.to_string_lossy().replace('\\', "/") }
}

/// The id of the first layer an `edit_layers` answer's content says was made or changed.
fn made_in(content: &[Value]) -> Option<String> {
    content
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .filter_map(|text| serde_json::from_str::<Value>(text).ok())
        .find_map(|answer| answer.get("results")?.get(0)?.get("id")?.as_str().map(str::to_owned))
}

/// The `advice` in an `edit_layers` answer's content, if it has any.
fn advice_in(content: &[Value]) -> Option<Value> {
    content
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .filter_map(|text| serde_json::from_str::<Value>(text).ok())
        .find_map(|answer| answer.get("advice").cloned())
}

/// Whether `id`, which is no layer of the document itself, is `instanceId/layerId`: a layer inside a component
/// instance, which is changed by an override.
fn in_instance(editor: &Editor, id: &str) -> bool {
    let instance = id.split_once('/').and_then(|(instance, _)| pen_doc::edit::find(&editor.doc, instance));
    instance.is_some_and(|instance| matches!(instance.kind, LayerKind::Ref))
}

/// Whether the layer `id` can be given a picture as its fill.
fn takes_picture(editor: &Editor, id: &str) -> Result<(), String> {
    let Some(node) = pen_doc::edit::find(&editor.doc, id) else {
        if in_instance(editor, id) {
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
    edit_once(editor, &fill_edit(id, url)).map(|_| ())
}

/// Makes the change the `edit_layers` arguments `arguments` describe, as one undo step; what was made or changed.
// Human: the same order as a model's change (`App::edit_in`): the step is kept only when the change worked.
// `checkpoint` marks the document changed and in need of layout.
fn edit_once(editor: &mut Editor, arguments: &Value) -> Result<Vec<Value>, String> {
    let before = editor.doc.clone();
    let (results, _) = agent::edit(editor, arguments)?;
    let after = std::mem::replace(&mut editor.doc, before);
    editor.checkpoint();
    editor.doc = after;
    Ok(results)
}

/// The layers `ids` of `editor` as pictures of the dialog's row, each named as the layers list names it; a layer
/// that isn't there is left out.
fn pieces_of(editor: &Editor, ids: &[String]) -> Vec<Piece> {
    ids.iter()
        .filter_map(|id| {
            let name = match pen_doc::edit::find(&editor.doc, id) {
                Some(node) => node.display_name(),
                None => editor.scene_node(id)?.node.display_name(),
            };
            Some(Piece::layer(id, &name, image_paint(editor, id).is_ok()))
        })
        .collect()
}

/// Whether files chosen with "Add file…" for the dialog of the document `asked_for` still go to the dialog: it
/// is `open`, not `busy` making its picture, and still that document's (`doc`).
// Human: the file window can stay up while the dialog is closed and opened again for another document; a
// picture chosen then must not land in a row it wasn't chosen for.
fn for_the_dialog(open: bool, busy: bool, doc: Option<u64>, asked_for: u64) -> bool {
    open && !busy && doc == Some(asked_for)
}

/// What the dialog opens with for the selected layers `ids` of `editor`: the layer the picture can fill (the
/// stage 1 target), and the pictures of the row.
// Human: several layers all go into the row, to be sent unless taken out (the "Sends N pictures" line says so
// before anything is sent). A lone layer goes in only when it has a picture as its fill (it is then the
// picture to change) or children (a design, to be sent as drawn): an empty shape, a frame without children or a
// text has nothing worth sending, so the dialog is the stage 1 one, which offers to fill the layer. A lone
// layer in the row is never also the target: a picture put in a design's fill would lie under its children,
// and one put in a layer's fill in place of its picture would drop the fill's other paints.
fn opening(editor: &Editor, ids: &[String]) -> (Option<String>, Vec<Piece>) {
    match ids {
        [only] => {
            let children = editor.scene_node(only).is_some_and(|scene| !scene.children.is_empty())
                || pen_doc::edit::find(&editor.doc, only).and_then(|node| node.children.as_ref()).is_some_and(|children| !children.is_empty());
            if children || image_paint(editor, only).is_ok() {
                (None, pieces_of(editor, ids))
            } else {
                (takes_picture(editor, only).is_ok().then(|| only.clone()), Vec::new())
            }
        }
        _ => (None, pieces_of(editor, ids)),
    }
}

/// What the quick field asks for the layer `layer`: what the dialog asks with that layer alone, marked, no
/// references, the picture's own shape and the normal size, from the service `service`.
// Human: built by a dialog, so the field and the dialog can't drift apart in what they send.
fn quick_asked(text: &str, layer: &str, service: &str) -> Value {
    let mut dialog = Dialog::default();
    dialog.open_for(None, &[Offered { name: service.to_owned(), edits: true }], service);
    dialog.set_pieces(vec![Piece::layer(layer, layer, true)]);
    text.clone_into(&mut dialog.prompt);
    dialog.asked()
}

/// Where a changed picture goes beside the layers `ids` instead of in place of one: right of the top-level
/// layers they are in, at their height and size; `None` when they aren't drawn.
fn beside_of(editor: &Editor, ids: &[String]) -> Option<Goes> {
    let drawn = ids.iter().filter_map(|id| editor.scene_node(id)).map(layout::doc_bounds).reduce(|a, b| a.union(&b))?;
    Some(Goes::Beside { drawn, tops: tops_holding(editor, ids).unwrap_or(drawn) })
}

/// Brings the pictures of the document `editor` to where its file lies, before that is written: a picture kept
/// under the home folder `home` because the document had no file when it was made is copied into `images/`
/// beside it and named from there, and when the document goes to another folder than that of `was`, the file
/// it had, the pictures named from there are copied along. A sentence for each picture that couldn't be
/// brought; that one keeps its path. A bundle takes in every picture file the document shows, also one named
/// by a full path or from outside its folder, and the other files the document names come along into and out
/// of a bundle (`bundle::carry`).
// Human: done on every save, not only the first: a picture can still land under the home folder after it (one
// that was being made while the document was saved), and the next save brings that one too. Without this a
// saved document names pictures by a full path into one person's home folder, and shows none of them anywhere
// else. A path that leaves the document's folder (`../shared/a.png`) is left as it is.
// Agent: READS and WRITES picture files and credits (`store::adopt`, `store::bring`); BLOCKS on them, on the
// thread that saves, which the document's own file is written on too. Changes `url`s in the document and its
// undo steps without a new undo step.
pub(super) fn settle(editor: &mut Editor, was: Option<&Path>, home: Option<&Path>) -> Vec<String> {
    let Some(document) = editor.anchor() else { return Vec::new() };
    let folder = document.parent().unwrap_or(Path::new("")).to_path_buf();
    let left = was.and_then(Path::parent).filter(|left| *left != folder);
    let waiting = home.map(store::waiting);
    let packs = editor.path.as_deref().is_some_and(crate::bundle::is_bundle);
    let mut told = Vec::new();
    editor.repicture(&mut |url| {
        let done = match crate::images::resolve(None, url) {
            Some(whole) if waiting.as_deref().is_some_and(|waiting| whole.starts_with(waiting)) => store::adopt(&whole, &document).map(Some),
            Some(whole) if packs && whole.is_file() => store::adopt(&whole, &document).map(Some),
            Some(_) => Ok(None),
            None => {
                let source = crate::images::resolve(Some(left.unwrap_or(&folder)), url).filter(|source| source.is_file());
                match source.zip(crate::images::resolve(Some(&folder), url)) {
                    // A path that leaves the folder: a bundle takes the picture in, a `.pen` file leaves it.
                    Some((source, target)) if target.components().any(|part| part == Component::ParentDir) => {
                        if packs {
                            store::adopt(&source, &document).map(Some)
                        } else {
                            Ok(None)
                        }
                    }
                    Some((source, target)) if left.is_some() => store::bring(&source, &target).map(|()| None),
                    _ => Ok(None),
                }
            }
        };
        done.unwrap_or_else(|why| {
            told.push(why);
            None
        })
    });
    if let Some(left) = left.filter(|_| editor.bundled()) {
        told.extend(crate::bundle::carry(&editor.doc, left, &folder));
    }
    told
}

/// Writes the document of `editor` as a `.pen` file at `target`, with the files it names beside it, and gives a
/// sentence for each thing the user should know about the file: a file that couldn't be copied or was there
/// already as another one, a picture named from outside the folder, what in it only Quill draws, parts a
/// newer Quill added that a `.pen` file has no place for.
///
/// # Errors
/// A sentence when the document or the file can't be written.
// Human: an export. It is made from a copy of the document, in which a picture kept under the home folder is
// named from beside `target` as in a document saved there ([`settle`]), and the files the document names inside
// its folder are copied to the same place beside `target`. A file that is there already is never written
// over: the same file is used; when it is another one, a picture is copied under a free name and the copy of
// the document names that one, and for a shader file or an imported document, which are named in more places
// than a picture is, the export says that the exported file shows the one that was there. The document in
// the tab is not touched: it keeps its file, its working folder, its paths and its unsaved changes.
// Agent: READS the files the document names; WRITES `target` and the copies beside it. BLOCKS on them.
pub(super) fn export_pen(editor: &Editor, target: &Path, home: Option<&Path>) -> Result<Vec<String>, String> {
    let mut copy = Editor::new(editor.doc.clone(), Some(target.to_path_buf()), 0);
    let mut told = settle(&mut copy, None, home);
    if let (Some(from), Some(to)) = (editor.folder(), target.parent())
        && from != to
    {
        told.extend(bring_along(&mut copy, from, to, target));
    }
    pen_doc::stamp::apply(&mut copy.doc);
    let text = pen_doc::write(&copy.doc).map_err(|error| error.to_string())?;
    craft_atomic::atomic_write(target, text.as_bytes()).map_err(|error| format!("{}: {error}", target.display()))?;
    told.extend(copy.only_here());
    if editor.keeps_parts() {
        told.push("Parts a newer Quill added to the .quill file are not in the .pen file.".to_owned());
    }
    Ok(told)
}

/// Copies the files the document `copy`, to be written at `target` in the folder `to`, names inside the folder
/// `from` to the same place in `to`, and gives a sentence for each thing to know: a file that couldn't be
/// copied, a picture that got another name because another file had its own, a file that was left because
/// another one is there, a picture named by a path that leaves the folder.
// Agent: READS under `from` and `to`; WRITES under `to`. BLOCKS on both.
fn bring_along(copy: &mut Editor, from: &Path, to: &Path, target: &Path) -> Vec<String> {
    let same = |one: &Path, other: &Path| std::fs::read(one).ok().zip(std::fs::read(other).ok()).is_some_and(|(one, other)| one == other);
    let mut told = Vec::new();
    copy.repicture(&mut |url| {
        let Some(name) = crate::bundle::relative(url) else {
            // Human: a path that leaves the folder (`../shared/a.png`) is not followed by an export, as a
            // `.pen` file's never was by a save; from the exported file it leads somewhere else.
            if crate::images::resolve(None, url).is_none() && crate::images::resolve(Some(from), url).is_some_and(|file| file.is_file()) {
                told.push(format!(
                    "The picture {url} lies outside the document's folder: the exported file names it by the same path, which starts at {} there.",
                    to.display()
                ));
            }
            return None;
        };
        let (source, there) = (from.join(&name), to.join(&name));
        if !source.is_file() || same(&source, &there) {
            return None;
        }
        let done = if there.exists() {
            store::adopt(&source, target).inspect(|new| {
                told.push(format!(
                    "Another picture lies at {name} in {}: this one was copied as {new}, and the exported file names it so.",
                    to.display()
                ));
            })
        } else {
            store::bring(&source, &there).map(|()| url.to_owned())
        };
        match done {
            Ok(new) => Some(new).filter(|new| new != url),
            Err(why) => {
                told.push(why);
                None
            }
        }
    });
    // Human: what is left are the shader files and the imported documents; the pictures are there by now.
    for name in crate::bundle::named(&copy.doc) {
        let (source, there) = (from.join(&name), to.join(&name));
        if !source.is_file() || same(&source, &there) {
            continue;
        }
        if there.exists() {
            told.push(format!("Another file lies at {name} in {}: it was left as it is, and the exported file uses that one.", to.display()));
            continue;
        }
        let copied = there.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::copy(&source, &there));
        if let Err(error) = copied {
            told.push(format!("The file {} couldn't be brought to {}: {error}.", source.display(), to.display()));
        }
    }
    told
}

/// A small picture of the picture file at `path`, at most [`PIECE_PIXELS`] a side; `None` when it isn't a PNG,
/// JPEG, WebP or GIF file that can be read.
// Human: opened as a file to send is (`sources::open`): the size cap, the format the first bytes say and the
// decoder's limits. Nothing of the file is logged.
// Agent: READS `path`; BLOCKS on decoding: never on the main thread (`App::make_thumbs` runs it on its own).
fn file_thumb(path: &Path) -> Option<egui::ColorImage> {
    let picture = sources::open(path).ok()?;
    // Human: a picture already small is shown as it is, not enlarged.
    let small = if picture.width().max(picture.height()) > PIECE_PIXELS { picture.thumbnail(PIECE_PIXELS, PIECE_PIXELS) } else { picture };
    let small = small.to_rgba8();
    let size = [usize::try_from(small.width()).ok()?, usize::try_from(small.height()).ok()?];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, small.as_raw()))
}

/// Whether `paint` is an image paint.
fn is_image(paint: &Paint) -> bool {
    matches!(paint, Paint::Object(object) if object.get("type").and_then(Value::as_str) == Some("image"))
}

/// The fill of the layer `id`, the place in it of its topmost image paint, and that paint's `url`; or why
/// there is none.
// Human: paints are listed bottom first, so the topmost is the last.
fn image_paint<'a>(editor: &'a Editor, id: &str) -> Result<(&'a Paints, usize, &'a str), String> {
    let node = pen_doc::edit::find(&editor.doc, id).ok_or_else(|| format!("There is no layer with the id \"{id}\"."))?;
    let no_picture = || format!("The layer \"{id}\" has no picture as its fill.");
    let fill = node.fill.as_ref().ok_or_else(no_picture)?;
    let at = fill.as_slice().iter().rposition(is_image).ok_or_else(no_picture)?;
    let url = match fill.as_slice().get(at) {
        Some(Paint::Object(paint)) => paint.get("url").and_then(Value::as_str).unwrap_or_default(),
        _ => "",
    };
    Ok((fill, at, url))
}

/// Whether a changed picture may go onto the layer `layer`: its topmost image paint still has the `url` it
/// had when the edit started (`was`). Not when the user (or a model) put another picture there meanwhile, or
/// took the picture or the layer away.
fn still_as_started(editor: &Editor, layer: &str, was: &str) -> bool {
    image_paint(editor, layer).is_ok_and(|(_, _, url)| url == was)
}

/// The file a layer's image fill shows, or why it has none that can be read.
// Human: whether the file is there is not asked here: that is the filesystem, and it is asked on the job's
// thread (`sources::prepare`), which names the layer when it isn't.
fn image_of(editor: &Editor, id: &str) -> Result<PathBuf, String> {
    let (_, _, url) = image_paint(editor, id)?;
    let folder = editor.folder();
    crate::images::resolve(folder, url).ok_or_else(|| {
        // Human: a path that would be found beside a saved document is one relative to a document without a file.
        let unsaved = folder.is_none() && crate::images::resolve(Some(Path::new(".")), url).is_some();
        let why = if unsaved { ": the document isn't saved yet, so the path isn't relative to anything" } else { "" };
        format!("The picture of the layer \"{id}\" is \"{url}\", which can't be read from disk{why}.")
    })
}

/// The `fill` of the layer `id` with its image paint's `url` replaced by `url`: the paint's other keys
/// (`mode`…) and the fill's other paints stay as they are.
fn fill_replaced(editor: &Editor, id: &str, url: &str) -> Result<Value, String> {
    let (fill, at, _) = image_paint(editor, id)?;
    let mut fill = serde_json::to_value(fill).map_err(|error| error.to_string())?;
    let paint = match &mut fill {
        Value::Array(paints) => paints.get_mut(at),
        one => Some(one),
    };
    if let Some(Value::Object(paint)) = paint {
        paint.insert("url".to_owned(), json!(url));
    }
    Ok(fill)
}

/// The `edit_layers` arguments that give the layer `id` the fill `fill`, its picture changed.
fn replace_edit(id: &str, fill: &Value) -> Value {
    json!({
        "description": "Put the changed picture on the layer",
        "operations": [{ "op": "update", "id": id, "properties": { "fill": fill } }]
    })
}

/// How a picture a call names is got.
#[derive(Debug, PartialEq, Eq)]
enum Got {
    /// From a file, read on the job's thread.
    Ready(Take),
    /// By drawing these layers as they look, on the main thread, before the job starts.
    Draw(Vec<String>),
}

/// How the picture `pick` names in `editor` is got; `change` when it is the one to change.
// Human: a reference may be any layer: one with a picture as its fill sends that picture's file, any other is
// drawn as it looks (a layer inside a component instance too). The picture to change named by `layer` must be
// a picture, since that picture is replaced; a layer without one is changed as it looks through `layers`,
// which the answer says.
// Agent: asks nothing of the filesystem: a file named is read, or found missing, on the job's thread.
fn take(editor: &Editor, pick: &Pick, change: bool) -> Result<Got, Refused> {
    let id = match pick {
        Pick::Drawn(ids) => return Ok(Got::Draw(ids.clone())),
        Pick::File(path) => return Ok(Got::Ready(Take::file(path.clone(), change, None))),
        Pick::Layer(id) => id,
    };
    if pen_doc::edit::find(&editor.doc, id).is_none() {
        return if change && in_instance(editor, id) {
            Err(Refused::InInstance(id.clone()))
        } else if !change && editor.scene_node(id).is_some() {
            Ok(Got::Draw(vec![id.clone()]))
        } else {
            Err(Refused::Gone(id.clone()))
        };
    }
    match image_of(editor, id) {
        Ok(path) => Ok(Got::Ready(Take::file(path, change, Some(id.clone())))),
        Err(_) if !change && image_paint(editor, id).is_err() => Ok(Got::Draw(vec![id.clone()])),
        Err(why) if image_paint(editor, id).is_err() => Err(Refused::NoPicture(why)),
        Err(why) => Err(Refused::Other(why)),
    }
}

/// Why a picture named can't be got: said one way to a model, which names layers by id and arguments, and
/// another to a person, who picked them in the dialog or the properties panel.
// Human: the checks are made once (`take`, `App::drawn`); only the words differ, so a sentence written for a
// model ("name it in `layers` instead") never reaches a person.
#[derive(Debug, PartialEq, Eq)]
enum Refused {
    /// There is no layer with this id.
    Gone(String),
    /// The layer to change has no picture as its fill: why, as `image_of` says it.
    NoPicture(String),
    /// The layer to change is inside a component instance.
    InInstance(String),
    /// Anything else, said the same way to both.
    Other(String),
}

impl Refused {
    /// The sentence for a model.
    fn for_model(self) -> String {
        match self {
            Refused::Gone(id) => format!("There is no layer with the id \"{id}\"."),
            Refused::NoPicture(why) => format!("{why} To change the layer as it looks, name it in `layers` instead."),
            Refused::InInstance(id) => format!(
                "\"{id}\" is a layer inside a component instance: its picture can't be changed this way. Name it in `layers` to change it as it looks, then set its fill with edit_layers, using the `url` of the picture."
            ),
            Refused::Other(why) => why,
        }
    }

    /// The sentence for a person.
    fn for_person(self) -> String {
        match self {
            Refused::Gone(_) => PERSON_GONE.to_owned(),
            Refused::NoPicture(_) => PERSON_NO_PICTURE.to_owned(),
            Refused::InInstance(_) => PERSON_IN_INSTANCE.to_owned(),
            Refused::Other(why) => why,
        }
    }

    /// The sentence for whoever asked: a model when `model`, else a person.
    fn said(self, model: bool) -> String {
        if model { self.for_model() } else { self.for_person() }
    }
}

/// How many pixels a canvas unit becomes when layers taking `bounds` are drawn to be sent: as many as keep the
/// picture within `sources::MOST_SIDE`, and at most [`MOST_ENLARGED`].
fn drawing_scale(bounds: Rect) -> f32 {
    #[expect(clippy::cast_precision_loss, reason = "2048 is exact in an f32")]
    let most = sources::MOST_SIDE as f32;
    (most / bounds.w.max(bounds.h).max(1.0)).min(MOST_ENLARGED)
}

/// Whether the boxes `a` and `b` share some area (touching edges don't).
fn overlap(a: &Rect, b: &Rect) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}

/// Where the picture of layers drawn in `drawn` goes: as large as `drawn` and at its height, [`BESIDE_GAP`] to
/// the right of `tops` (the top-level layers they are in), then on to the right past each of the top-level
/// layers `others` it would lie on.
// Human: a card drawn inside a page gets its picture beside the page, not over it.
// Agent: each step moves the box past at least one layer it can't meet again, so `others.len()` steps end it.
fn beside_box(drawn: Rect, tops: Rect, others: &[Rect]) -> Rect {
    let mut place = Rect { x: tops.right() + BESIDE_GAP, y: drawn.y, w: drawn.w, h: drawn.h };
    for _ in 0..others.len() {
        let Some(past) = others.iter().filter(|other| overlap(other, &place)).map(Rect::right).reduce(f32::max) else { break };
        place.x = past + BESIDE_GAP;
    }
    place
}

/// The layer a picture made from drawn layers becomes: a rectangle filling `place`, with the picture at `url`
/// as its fill.
fn beside(place: Rect, url: &str, name: &str) -> Value {
    json!({
        "type": "rectangle",
        "name": name,
        "x": place.x,
        "y": place.y,
        "width": place.w,
        "height": place.h,
        "fill": { "type": "image", "url": url, "mode": "cover" }
    })
}

/// The `edit_layers` arguments that add the picture at `url` at the top level, filling `place`.
fn beside_edit(place: Rect, url: &str) -> Value {
    json!({
        "description": "Put the changed picture beside what was drawn",
        "operations": [{ "op": "insert", "node": beside(place, url, BESIDE_NAME) }]
    })
}

/// The box of the top-level layers of `editor` that hold any of the layers `ids`.
fn tops_holding(editor: &Editor, ids: &[String]) -> Option<Rect> {
    editor.scene.iter().filter(|top| ids.iter().any(|id| holds(top, id))).map(layout::doc_bounds).reduce(|a, b| a.union(&b))
}

/// A model's answer about its picture: the small copy, and what it is `told`.
fn answer(done: &Done, told: &Value) -> Vec<Value> {
    vec![
        json!({ "type": "image", "data": agent::base64(&done.shown), "mimeType": "image/png" }),
        json!({ "type": "text", "text": serde_json::to_string_pretty(told).unwrap_or_default() }),
    ]
}

impl App {
    /// The image services that can be asked right now, and the settings' default.
    pub(super) fn image_services(&self) -> (Vec<imagery::Ready>, String) {
        let images = &self.prefs.get().images;
        (imagery::ready(images, &|name| std::env::var(name).ok()), images.default.clone())
    }

    /// Starts a `generate_image` or `edit_image` call: answers at once when it can't be done, otherwise keeps
    /// it until its picture is there.
    // Human: both kinds count towards the same limit (`room`): a picture changed costs as much as one made. A
    // call whose chat was stopped meanwhile (it waited for faces, or for its turn) is dropped as
    // `finish_pictures` drops one: nothing is drawn, sent or made for it.
    pub(super) fn start_picture(&mut self, call: Call, ctx: &egui::Context) {
        if !heard(&self.talks, call.chat) {
            return;
        }
        let ordered = if call.tool == "edit_image" { self.edit_order(&call, ctx) } else { self.picture_order(&call) };
        match ordered.and_then(|ordered| room(self.picturing.len()).map(|()| ordered)) {
            Ok((order, doc, goes)) => {
                let folder = order.document.as_deref().and_then(Path::parent).map(Path::to_path_buf);
                let from = sources::in_order(&order.takes).into_iter().map(|take| take.name.clone()).collect();
                let wake = ctx.clone();
                let job = Job::start(move |flags| jobs::work(&order, flags), move || wake.request_repaint());
                self.picturing.push(Waiting { call, job, doc, folder, held: None, from, goes });
                ctx.request_repaint_after(LOOK_AGAIN);
            }
            Err(why) => {
                // A client that stopped waiting has gone; there is no one to tell.
                let _ = call.reply.send(Err(why));
            }
        }
    }

    /// The service a picture call is to go to (one that can edit, for `editing`), and the tab of the document
    /// it is about.
    // Human: the switch, then the service, then the document: that there is no service is said before anything
    // about the document, so a model isn't sent to open one first. The document is found the way every other
    // tool finds it (`App::document`: the one the call names, else its chat's own, else the one in front), once,
    // now; the picture goes beside its file.
    fn service_and_document(&self, call: &Call, editing: bool) -> Result<(imagery::Ready, usize), String> {
        if !self.prefs.get().images.assistant {
            return Err(ASSISTANT_OFF.to_owned());
        }
        let (ready, default) = self.image_services();
        let saved = &self.prefs.get().images.services;
        let asked = call.arguments.get("service").and_then(Value::as_str);
        let chosen = if editing { imagery::pick_editing(&ready, saved, asked, &default) } else { imagery::pick(&ready, saved, asked, &default) };
        let chosen = chosen?.clone();
        Ok((chosen, self.document(call)?))
    }

    /// What a `generate_image` call asks for, the id of the document it is about, and where its picture goes.
    fn picture_order(&self, call: &Call) -> Result<(Order, u64, Goes), String> {
        let (chosen, tab) = self.service_and_document(call, false)?;
        let editor = self.tabs.get(tab).ok_or("The document was closed.")?;
        let args = &call.arguments;
        let goes = match args.get("layer").and_then(Value::as_str).map(str::trim) {
            Some(layer) => {
                takes_picture(editor, layer)?;
                Goes::Fill(layer.to_owned())
            }
            None => Goes::Answer,
        };
        let request = imagery::request(args, &chosen)?;
        Ok((Order::new(chosen, request, editor.anchor()), editor.id, goes))
    }

    /// What an `edit_image` call asks for, the id of the document it is about, and where its picture goes.
    // Human: as `picture_order`: the switch, then the service, then the document, found once, now. The pictures
    // named are found in the document now; their files are read on the job's thread. Layers to send as they
    // look are drawn here, last, once everything else about the call is known to be right: only the layers the
    // call names are drawn, and only when the call can go.
    // Agent: MAIN-THREAD (drawing needs the window's renderer); asks nothing of the filesystem. The faces of the
    // document's text are made ready before this is called: `App::serve_tools` holds a call that `draws` while
    // they come, as it holds a screenshot.
    fn edit_order(&mut self, call: &Call, ctx: &egui::Context) -> Result<(Order, u64, Goes), String> {
        let (chosen, tab) = self.service_and_document(call, true)?;
        self.edit_parts(&call.arguments, chosen, tab, true, ctx)
    }

    /// What `edit_image` arguments `args` ask of the service `chosen` for the document in tab `tab`: the order,
    /// the document's id, and where the picture goes. A model's call (`model`: counted among the pictures made
    /// for models, so drawing waits for room, and told what is wrong in a model's words), the dialog and the
    /// quick field all go through here.
    // Agent: MAIN-THREAD (drawing needs the window's renderer); asks nothing of the filesystem.
    fn edit_parts(
        &mut self,
        args: &Value,
        chosen: imagery::Ready,
        tab: usize,
        model: bool,
        ctx: &egui::Context,
    ) -> Result<(Order, u64, Goes), String> {
        let editor = self.tabs.get(tab).ok_or("The document was closed.")?;
        let named = imagery::named(args)?;
        let picks = named.change.iter().map(|pick| (pick, true)).chain(named.references.iter().map(|pick| (pick, false)));
        let got = picks
            .map(|(pick, change)| take(editor, pick, change).map(|got| (got, change)))
            .collect::<Result<Vec<_>, Refused>>()
            .map_err(|refused| refused.said(model))?;
        let request = imagery::edit_request(args, &chosen, named.change.is_some())?;
        let mut goes = match &named.change {
            Some(Pick::Layer(layer)) => Goes::Replace {
                layer: layer.clone(),
                was: image_paint(editor, layer).map_err(|why| Refused::NoPicture(why).said(model))?.2.to_owned(),
            },
            Some(Pick::File(_)) => Goes::File,
            // Human: put in its place once the layers are drawn, below.
            Some(Pick::Drawn(_)) | None => Goes::Answer,
        };
        let (document, doc) = (editor.anchor(), editor.id);
        // Human: nothing is drawn for a call that would be turned away for want of room.
        if model && got.iter().any(|(got, _)| matches!(got, Got::Draw(_))) {
            room(self.picturing.len())?;
        }
        let mut takes = Vec::new();
        for (got, change) in got {
            match got {
                Got::Ready(take) => takes.push(take),
                Got::Draw(ids) => {
                    let (picture, drawn) = self.drawn(tab, &ids, ctx).map_err(|refused| refused.said(model))?;
                    if change {
                        let tops = self.tabs.get(tab).and_then(|editor| tops_holding(editor, &ids)).unwrap_or(drawn);
                        goes = Goes::Beside { drawn, tops };
                    }
                    let (width, height) =
                        (u32::try_from(picture.w).map_err(|e| e.to_string())?, u32::try_from(picture.h).map_err(|e| e.to_string())?);
                    takes.push(Take::pixels(width, height, picture.rgba, change));
                }
            }
        }
        Ok((Order { ready: chosen, request, document, takes }, doc, goes))
    }

    /// Whether an `edit_image` call sends layers drawn as they look: its `layers`, or a reference that is a
    /// layer without a picture as its fill. Such a call waits for the faces of its document's text, as a
    /// screenshot does (`App::serve_tools`).
    pub(super) fn draws(&self, call: &Call) -> bool {
        if call.tool != "edit_image" || !self.prefs.get().images.assistant {
            return false;
        }
        let Ok(named) = imagery::named(&call.arguments) else { return false };
        let Some(editor) = self.document(call).ok().and_then(|tab| self.tabs.get(tab)) else { return false };
        let mut picks = named.change.iter().map(|pick| (pick, true)).chain(named.references.iter().map(|pick| (pick, false)));
        picks.any(|(pick, change)| matches!(take(editor, pick, change), Ok(Got::Draw(_))))
    }

    /// The layers `ids` of the document in tab `tab`, drawn together as they look, no larger than
    /// `sources::MOST_SIDE` a side (give or take the renderer's rounding), with the box they take on the canvas.
    // Human: drawn by the renderer `screenshot` and export use, so what is sent is what the user sees. The pixels
    // go to the job as they are: encoding them, and scaling down a picture a pixel over the side or with a shadow
    // reaching past the box, is done on the job's thread (`sources::prepare`).
    // Agent: MAIN-THREAD (the renderer); called before the job starts.
    fn drawn(&mut self, tab: usize, ids: &[String], ctx: &egui::Context) -> Result<(crate::export::Picture, Rect), Refused> {
        let nothing = || Refused::Other(NOTHING_DRAWN.to_owned());
        let editor = self.tabs.get_mut(tab).ok_or_else(|| Refused::Other("The document was closed.".to_owned()))?;
        editor.refresh(&super::text_measure(ctx));
        let editor = &*editor;
        let layers = ids.iter().map(|id| editor.scene_node(id).ok_or_else(|| Refused::Gone(id.clone()))).collect::<Result<Vec<_>, _>>()?;
        let bounds = layers
            .iter()
            .map(|scene| layout::doc_bounds(scene))
            .reduce(|a, b| a.union(&b))
            .filter(|bounds| bounds.w > 0.0 && bounds.h > 0.0)
            .ok_or_else(nothing)?;
        let folder = editor.folder();
        let picture = self.canvas.list().snapshot(&layers, drawing_scale(bounds), Vars::new(&editor.doc), ctx, folder).ok_or_else(nothing)?;
        Ok((picture, bounds))
    }

    /// Answers the `generate_image` and `edit_image` calls whose picture is there, or failed, or took too long.
    // Human: a picture that is to go on a layer waits while the user is dragging something on the canvas, as a
    // model's own change does (`App::serve_tools`); it is put in when the drag has ended, or handed to the
    // model without touching the layer when the drag goes on too long (`hold`). Stopping a chat stops the
    // pictures it asked for.
    // Agent: MAIN-THREAD; runs from `serve_tools`, so also while the window is hidden. A kept picture is not
    // polled again: its job's result has already been taken.
    pub(super) fn finish_pictures(&mut self, ctx: &egui::Context) {
        // Agent: a call from a chat that is gone or no longer running is dropped unanswered (`heard`). Dropping
        // it drops its `Job`, which marks the job as given up, so nothing more is sent and the picture is not
        // kept: not saved, or removed again when it was saved already (`jobs::Job`).
        let talks = &self.talks;
        self.picturing.retain(|waiting| heard(talks, waiting.call.chat));
        if self.picturing.is_empty() {
            return;
        }
        let now = Instant::now();
        for mut waiting in std::mem::take(&mut self.picturing) {
            let (result, since) = match waiting.held.take() {
                Some((done, since)) => (Some(Ok(done)), since),
                None => (waiting.job.poll(), now),
            };
            match result {
                None => self.picturing.push(waiting),
                Some(Ok(done)) => match if waiting.goes.to_layer() { hold(self.canvas.dragging(), since, now) } else { Hold::Fill } {
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

    /// Why the picture is not to go where `goes` says, when it isn't: a switch was turned off while the picture
    /// was made, or the user's drag went on too long (`dragged`); with what the model can do itself.
    fn layer_left(&self, dragged: bool, goes: &Goes) -> Option<String> {
        let prefs = self.prefs.get();
        let (left, yourself) = match goes {
            Goes::Beside { .. } => ("no layer was added for it", "Add a rectangle with the `url` as its fill with edit_layers.".to_owned()),
            Goes::Replace { .. } => ("the layer was left alone", format!("To put it there, {REPLACE_YOURSELF}")),
            _ => ("the layer was left alone", "Put the `url` in a fill with edit_layers.".to_owned()),
        };
        if !prefs.agents.tools {
            Some(format!("AI tools were turned off in Quill's settings while the picture was being made, so {left}."))
        } else if !prefs.images.assistant {
            Some(format!(
                "Generating and editing pictures was turned off for AI models in Quill's settings while the picture was being made, so {left}."
            ))
        } else if dragged {
            Some(format!("The user was moving something on the canvas, so {left}. {yourself}"))
        } else {
            None
        }
    }

    /// What a model is told about its picture; puts it where the call asked first, unless the user dragged too
    /// long (`dragged`), a switch was turned off meanwhile, or (for a picture replaced) the layer's picture was
    /// changed meanwhile.
    fn picture_answer(&mut self, waiting: &Waiting, done: &Done, dragged: bool, ctx: &egui::Context) -> Vec<Value> {
        // Human: a `./images/…` url is relative to the document's folder when the picture was asked for; a
        // document saved elsewhere since, or closed, gets the picture's full path instead.
        let now = self.tabs.iter().find(|editor| editor.id == waiting.doc).and_then(|editor| editor.folder());
        let url = lasting_url(&done.saved, waiting.folder.as_deref(), now);
        let mut told = json!({
            "url": url,
            "width": done.saved.width,
            "height": done.saved.height,
            "service": done.service,
            "model": done.model,
        });
        if !waiting.from.is_empty() {
            told["from"] = json!(waiting.from);
        }
        let advice = |told: &mut Value, advice: Option<Value>| {
            if let Some(advice) = advice {
                told["advice"] = advice;
            }
        };
        if let Some(why) = self.layer_left(dragged, &waiting.goes).filter(|_| waiting.goes.to_layer()) {
            told["layerProblem"] = json!(why);
            return answer(done, &told);
        }
        match &waiting.goes {
            Goes::Answer => told["next"] = json!("Nothing in the document changed. Put the `url` in an image fill with edit_layers."),
            Goes::File => told["next"] = json!(format!("Nothing in the document changed; the picture is kept at {}.", done.saved.path.display())),
            Goes::Beside { drawn, tops } => {
                let (drawn, tops) = (*drawn, *tops);
                let placed = self.change_for(waiting, ctx, |editor| {
                    let others: Vec<Rect> = editor.scene.iter().map(layout::doc_bounds).collect();
                    Ok(beside_edit(beside_box(drawn, tops, &others), &url))
                });
                match placed.and_then(|content| Ok((made_in(&content).ok_or("The new layer wasn't found after it was added.")?, advice_in(&content))))
                {
                    Ok((id, said)) => {
                        told["newLayer"] = json!(id);
                        told["next"] = json!("The picture is a new layer beside the layers that were drawn, which are unchanged.");
                        advice(&mut told, said);
                    }
                    Err(why) => {
                        told["layerProblem"] = json!(format!(
                            "The picture was changed, but no layer was added for it: {why} Add a rectangle with the `url` as its fill with edit_layers."
                        ));
                    }
                }
            }
            Goes::Fill(layer) => match self.change_for(waiting, ctx, |_| Ok(fill_edit(layer, &url))) {
                Ok(content) => {
                    told["layer"] = json!(layer);
                    advice(&mut told, advice_in(&content));
                }
                Err(why) => {
                    told["layerProblem"] =
                        json!(format!("The picture was made, but the layer wasn't changed: {why} Use the `url` in a fill yourself."));
                }
            },
            Goes::Replace { layer, was } => {
                let changed = self.tabs.iter().find(|editor| editor.id == waiting.doc).is_some_and(|editor| !still_as_started(editor, layer, was));
                if changed {
                    told["layerProblem"] = json!(CHANGED_MEANWHILE);
                } else {
                    match self.change_for(waiting, ctx, |editor| Ok(replace_edit(layer, &fill_replaced(editor, layer, &url)?))) {
                        Ok(content) => {
                            told["layer"] = json!(layer);
                            advice(&mut told, advice_in(&content));
                        }
                        Err(why) => {
                            told["layerProblem"] =
                                json!(format!("The picture was changed, but the layer wasn't: {why} To put it there, {REPLACE_YOURSELF}"));
                        }
                    }
                }
            }
        }
        answer(done, &told)
    }

    /// Makes the change `arguments` builds from the waiting call's document, laid out, as a model's own
    /// `edit_layers` call; that call's answer.
    // Human: the change is made the way a model makes one itself (`App::edit_in`), so it is one undo step, the
    // layers flash, the model's cursor goes to them, ⌘Z takes it back, the selection is left as it is, and what
    // the app would not leave that way is said. A fill replaced takes the fill as it is now: its `mode` and other
    // paints stay. A picture beside drawn layers goes by the box remembered when the call started: layers moved
    // or removed meanwhile don't keep the picture, which was paid for, from a place.
    // Agent: the tab is found by the document's id, never by its name, which two tabs can share ("Untitled").
    // The inner call comes from the same client and chat as the waiting one.
    fn change_for(
        &mut self,
        waiting: &Waiting,
        ctx: &egui::Context,
        arguments: impl FnOnce(&Editor) -> Result<Value, String>,
    ) -> Result<Vec<Value>, String> {
        let tab = self.tabs.iter().position(|tab| tab.id == waiting.doc).ok_or("Its document was closed.")?;
        let editor = self.tabs.get_mut(tab).ok_or("Its document was closed.")?;
        editor.refresh(&super::text_measure(ctx));
        let arguments = arguments(editor)?;
        let (reply, _unheard) = channel();
        let edit = Call { tool: "edit_layers".to_owned(), arguments, client: waiting.call.client.clone(), chat: waiting.call.chat, reply };
        self.edit_in(tab, &edit, ctx)
    }

    /// Opens the "Generate Image…" dialog for the document in front, with its selected layers in the row of
    /// pictures.
    pub(super) fn open_picture_dialog(&mut self) {
        let Some(ids) = self.active.and_then(|tab| self.tabs.get(tab)).map(|editor| editor.selection.clone()) else { return };
        self.open_picture_dialog_on(&ids);
    }

    /// Opens the dialog for the document in front with the layers `ids` in its row of pictures as `opening`
    /// decides: a lone layer with a picture as its fill is marked as the one to change ("Edit with AI…" opens
    /// it so).
    // Human: with exactly one layer that can take a picture and nothing in the row, the dialog offers to fill it.
    // The settings' switch for the assistant ("Let the assistant generate and edit pictures") doesn't apply here: a person may always
    // make a picture. While the dialog's picture is being made, the dialog shows again as it is, for its own
    // document; while the quick field's is, it opens but waits for it (`Dialog::other`).
    pub(super) fn open_picture_dialog_on(&mut self, ids: &[String]) {
        let Some(editor) = self.active.and_then(|tab| self.tabs.get(tab)) else { return };
        let dialogs_own = self.generating.job.as_ref().is_some_and(|making| making.quick.is_none());
        if !dialogs_own {
            self.generating.doc = Some(editor.id);
        }
        let (target, pieces) = opening(editor, ids);
        let (ready, default) = self.image_services();
        let offered: Vec<Offered> = ready.iter().map(|r| Offered { name: r.name.clone(), edits: r.kind.edits() }).collect();
        let dialog = &mut self.generating.dialog;
        dialog.open_for(target, &offered, &default);
        dialog.set_pieces(pieces);
        dialog.other = self.generating.job.as_ref().is_some_and(|making| making.quick.is_some());
    }

    /// Adds the picture files chosen with "Add file…" for the dialog of the document `doc` to its row as
    /// references, as many as fit; says why the others aren't. An answer for a dialog that has closed, is
    /// making its picture, or was opened again for another document is dropped.
    pub(super) fn add_references(&mut self, doc: u64, paths: Vec<PathBuf>) {
        let dialog = &self.generating.dialog;
        if for_the_dialog(dialog.open, dialog.busy, self.generating.doc, doc) {
            self.generating.dialog.add_files(paths);
        }
    }

    /// Gives the pictures of the dialog's row their small pictures: a layer's drawn now, small; a file's made
    /// on a thread of its own and put in when it is there. One that can't be made keeps the picture icon.
    // Human: drawing a layer is the renderer's work and quick at this size; reading a file can be slow (a large
    // picture, a slow disk), so the window never waits for it.
    // Agent: MAIN-THREAD; READS nothing itself.
    fn make_thumbs(&mut self, ctx: &egui::Context) {
        for (path, coming) in std::mem::take(&mut self.generating.thumbs) {
            match coming.try_recv() {
                Ok(image) => {
                    let texture = image.map(|image| ctx.load_texture("reference", image, egui::TextureOptions::LINEAR));
                    for piece in self.generating.dialog.pieces_mut() {
                        if matches!(&piece.what, PieceOf::File(file) if *file == path) {
                            piece.thumb.clone_from(&texture);
                        }
                    }
                }
                Err(TryRecvError::Empty) => self.generating.thumbs.push((path, coming)),
                Err(TryRecvError::Disconnected) => {}
            }
        }
        let wanted: Vec<(usize, PieceOf)> = self
            .generating
            .dialog
            .pieces_mut()
            .iter_mut()
            .enumerate()
            .filter_map(|(index, piece)| (!std::mem::replace(&mut piece.looked, true)).then(|| (index, piece.what.clone())))
            .collect();
        let tab = self.tabs.iter().position(|editor| Some(editor.id) == self.generating.doc);
        // Human: the document is laid out once for all the layers drawn now, not once for each.
        if let Some(editor) = tab.and_then(|tab| self.tabs.get_mut(tab))
            && wanted.iter().any(|(_, what)| matches!(what, PieceOf::Layer { .. }))
        {
            editor.refresh(&super::text_measure(ctx));
        }
        for (index, what) in wanted {
            match what {
                PieceOf::Layer { id, .. } => {
                    let thumb = tab.and_then(|tab| self.layer_thumb(tab, &id, ctx));
                    if let Some(piece) = self.generating.dialog.pieces_mut().get_mut(index) {
                        piece.thumb = thumb;
                    }
                }
                PieceOf::File(path) => {
                    let (send, coming) = channel();
                    let (wake, read) = (ctx.clone(), path.clone());
                    let started = std::thread::Builder::new().name("reference picture".to_owned()).spawn(move || {
                        // The dialog may be gone; then nobody wants the picture.
                        let _ = send.send(file_thumb(&read));
                        wake.request_repaint();
                    });
                    if started.is_ok() {
                        self.generating.thumbs.push((path, coming));
                    }
                }
            }
        }
    }

    /// A small picture of the layer `id` of the document in tab `tab`, drawn as it looks, at most
    /// [`PIECE_PIXELS`] a side.
    // Agent: MAIN-THREAD (the renderer); the document is laid out already (`make_thumbs`).
    fn layer_thumb(&self, tab: usize, id: &str, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        let editor = self.tabs.get(tab)?;
        let scene = editor.scene_node(id)?;
        let bounds = layout::doc_bounds(scene);
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return None;
        }
        #[expect(clippy::cast_precision_loss, reason = "128 is exact in an f32")]
        let pixels = PIECE_PIXELS as f32;
        let scale = (pixels / bounds.w.max(bounds.h)).min(MOST_ENLARGED);
        let folder = editor.folder();
        let picture = self.canvas.list().snapshot(&[scene], scale, Vars::new(&editor.doc), ctx, folder)?;
        let image = egui::ColorImage::from_rgba_premultiplied([picture.w, picture.h], &picture.rgba);
        Some(ctx.load_texture("layer", image, egui::TextureOptions::LINEAR))
    }

    /// Starts the person's picture `order` for the document `doc`, going where `goes` says; `quick` is the layer
    /// of the quick field that asked for it and its name, `None` for the dialog.
    fn start_making(&mut self, order: Order, doc: u64, goes: Goes, quick: Option<(String, String)>, ctx: &egui::Context) {
        let folder = order.document.as_deref().and_then(Path::parent).map(Path::to_path_buf);
        let wake = ctx.clone();
        let job = Job::start(move |flags| jobs::work(&order, flags), move || wake.request_repaint());
        self.generating.job = Some(Making { job, doc, folder, goes, quick, held: None });
    }

    /// Draws the dialog, starts its picture, and places the person's picture (the dialog's or the quick
    /// field's) when it is there.
    // Agent: MAIN-THREAD; called once a frame from `App::ui`.
    pub(super) fn show_picture_dialog(&mut self, ctx: &egui::Context) {
        self.generating.dialog.orphaned = self.dialog_editor().is_none();
        self.generating.dialog.other = self.generating.job.as_ref().is_some_and(Making::is_quick);
        if self.generating.dialog.open {
            self.make_thumbs(ctx);
        }
        match self.generating.dialog.show(ctx, crate::theme::palette(self.dark())) {
            Did::Nothing => {}
            Did::Cancel => {
                // Human: a picture the dialog asked for is given up on (only the Cancel button does that while
                // one is being made). Dropping its job means no file and no credit of it stay in `images/`: one
                // not saved yet is never saved, one saved already is removed again (`jobs::Job`), unless Quill
                // quits during the save. The service may still make it, and charge for it. The quick field's
                // picture is not the dialog's to give up, and one that is there already, waiting for the canvas,
                // is placed when it is free.
                if self.generating.job.as_ref().is_some_and(|making| !making.is_quick() && making.held.is_none()) {
                    self.generating.job = None;
                }
                self.generating.dialog.open = false;
                self.generating.dialog.busy = false;
            }
            Did::OpenSettings => {
                self.generating.dialog.open = false;
                self.settings.open_named("images");
            }
            Did::AddFile => {
                if let Some(doc) = self.generating.doc {
                    self.ask_reference(doc);
                }
            }
            Did::Generate if self.generating.job.is_some() => {}
            Did::Generate => match self.dialog_order(ctx) {
                Ok((order, doc, goes)) => {
                    self.start_making(order, doc, goes, None, ctx);
                    self.generating.dialog.busy = true;
                    self.generating.dialog.problem = None;
                }
                Err(why) => self.generating.dialog.problem = Some(why),
            },
        }
        self.finish_making(ctx);
        // Human: small pictures of files still being made for a dialog that closed, whichever way it closed, are
        // nobody's any more.
        if !self.generating.dialog.open {
            self.generating.thumbs.clear();
        }
    }

    /// Places the person's picture when it is there and the canvas is free, and says what became of it.
    // Agent: a held picture is not polled again: its job's result has already been taken.
    fn finish_making(&mut self, ctx: &egui::Context) {
        let busy = self.canvas.busy();
        let Some(making) = &mut self.generating.job else { return };
        let polled = if making.held.is_some() { None } else { making.job.poll() };
        let result = match step(making.held.take(), polled, busy) {
            Step::Wait => {
                ctx.request_repaint_after(LOOK_AGAIN);
                return;
            }
            Step::Hold(done) => {
                making.held = Some(done);
                ctx.request_repaint_after(LOOK_AGAIN);
                return;
            }
            Step::Place(result) => result,
        };
        let Some(making) = self.generating.job.take() else { return };
        let placed = result.and_then(|done| self.place_picture(&done, &making, ctx));
        match (&making.quick, placed) {
            (None, Ok(note)) => {
                self.generating.dialog.busy = false;
                self.generating.dialog.open = false;
                if let Some(note) = note {
                    self.say(note);
                }
            }
            (None, Err(why)) => {
                self.generating.dialog.busy = false;
                self.generating.dialog.problem = Some(why);
                // Human: a document closed meanwhile leaves the dialog only its Close button.
                self.generating.dialog.orphaned = self.dialog_editor().is_none();
            }
            (Some((layer, name)), placed) => {
                // Human: the field shows what became of its picture while its layer is the one selected in the
                // tab in front; otherwise the message line says it, naming the layer. The tab in front stays.
                let shown = self
                    .active
                    .and_then(|tab| self.tabs.get(tab))
                    .is_some_and(|editor| editor.id == making.doc && editor.selection.as_slice() == std::slice::from_ref(layer));
                if let Some(told) = quick_told(name, shown, &placed) {
                    self.say(told);
                }
                match placed {
                    Ok(_) => self.generating.quick.done_with(layer, making.doc),
                    Err(why) => self.generating.quick.failed(layer, making.doc, why),
                }
            }
        }
    }

    /// What the dialog asks for, for the document it was opened for: the order, the document's id, and where
    /// the picture goes.
    // Human: with pictures to send the dialog asks in `edit_image`'s form, and the order is built by the same
    // functions a model's call goes through (`edit_parts`): the service that can edit as `pick_editing` finds
    // it, the pictures as `take` gets them, layers drawn as `drawn` draws them. "Replace the picture" turned off
    // puts the result beside the layer instead.
    // Agent: MAIN-THREAD (drawing layers needs the renderer).
    fn dialog_order(&mut self, ctx: &egui::Context) -> Result<(Order, u64, Goes), String> {
        let tab = self.tabs.iter().position(|editor| Some(editor.id) == self.generating.doc).ok_or("The document was closed.")?;
        let asked = self.generating.dialog.asked();
        let (ready, default) = self.image_services();
        let saved = &self.prefs.get().images.services;
        let service = asked.get("service").and_then(Value::as_str);
        if !self.generating.dialog.has_pieces() {
            let chosen = imagery::pick(&ready, saved, service, &default)?.clone();
            let request = imagery::request(&asked, &chosen)?;
            let editor = self.tabs.get(tab).ok_or("The document was closed.")?;
            return Ok((Order::new(chosen, request, editor.anchor()), editor.id, Goes::Answer));
        }
        let chosen = imagery::pick_editing(&ready, saved, service, &default)?.clone();
        let (order, doc, goes) = self.edit_parts(&asked, chosen, tab, false, ctx)?;
        let goes = match goes {
            Goes::Replace { layer, .. } if !self.generating.dialog.replace => {
                let editor = self.tabs.get(tab).ok_or("The document was closed.")?;
                beside_of(editor, &[layer]).ok_or(NOTHING_DRAWN)?
            }
            goes => goes,
        };
        Ok((order, doc, goes))
    }

    /// What the properties island shows of the quick field for the document in tab `tab`, tied to its selected
    /// layer.
    pub(super) fn quick_look(&mut self, tab: usize) -> Pictures {
        if let Some(editor) = self.tabs.get(tab)
            && let [only] = editor.selection.as_slice()
        {
            self.generating.quick.tie(only, editor.id);
        }
        let (ready, default) = self.image_services();
        let service = imagery::pick_editing(&ready, &[], None, &default).ok().map(|ready| ready.name.clone());
        let quick = &self.generating.quick;
        let making = self.generating.job.as_ref();
        let ours = making.is_some_and(|making| making.is_quick_for(&quick.layer, quick.doc));
        let making_for = making.and_then(|making| making.quick.as_ref()).map(|(_, name)| name.clone());
        Pictures {
            service,
            text: quick.text.clone(),
            problem: quick.problem.clone(),
            busy: making.is_some(),
            ours,
            making_for,
            dark: self.dark(),
            asked: None,
        }
    }

    /// Takes in what the properties island did with the quick field and "Edit with AI…".
    pub(super) fn quick_asked(&mut self, pictures: Pictures, ctx: &egui::Context) {
        self.generating.quick.text = pictures.text;
        match pictures.asked {
            Some(PictureAsk::Dialog) => {
                let layer = self.generating.quick.layer.clone();
                self.open_picture_dialog_on(&[layer]);
            }
            Some(PictureAsk::Quick) => self.start_quick(ctx),
            // Human: as the dialog's Cancel: the picture is given up on, and nothing of it is kept; one that is there
            // already, waiting for the canvas, is placed when it is free.
            Some(PictureAsk::Cancel) if self.generating.job.as_ref().is_some_and(|making| making.is_quick() && making.held.is_none()) => {
                self.generating.job = None;
            }
            Some(PictureAsk::Cancel) | None => {}
        }
    }

    /// Starts the quick field's picture: the words as the change to the picture of its layer, which the result
    /// replaces.
    // Human: what the dialog would ask for that layer alone, marked (`quick_asked`), from the default service
    // when it can edit, else the first that can; through the same functions as the dialog and the tool.
    fn start_quick(&mut self, ctx: &egui::Context) {
        if self.generating.job.is_some() || self.generating.quick.text.trim().is_empty() {
            return;
        }
        let (layer, doc) = (self.generating.quick.layer.clone(), self.generating.quick.doc);
        let Some(tab) = self.tabs.iter().position(|editor| editor.id == doc) else { return };
        let name =
            self.tabs.get(tab).and_then(|editor| pen_doc::edit::find(&editor.doc, &layer)).map_or_else(|| layer.clone(), pen_doc::Node::display_name);
        let (ready, default) = self.image_services();
        let started = imagery::pick_editing(&ready, &self.prefs.get().images.services, None, &default).cloned().and_then(|chosen| {
            let asked = quick_asked(&self.generating.quick.text, &layer, &chosen.name);
            self.edit_parts(&asked, chosen, tab, false, ctx)
        });
        match started {
            Ok((order, doc, goes)) => {
                self.start_making(order, doc, goes, Some((layer, name)), ctx);
                self.generating.quick.problem = None;
            }
            Err(why) => self.generating.quick.problem = Some(why),
        }
    }

    /// The document the dialog was opened for, if it is still open.
    fn dialog_editor(&self) -> Option<&Editor> {
        self.tabs.iter().find(|editor| Some(editor.id) == self.generating.doc)
    }

    /// Puts the person's finished picture into its document where `making` says, then, for the dialog's,
    /// brings the document's tab to the front; one undo step. What to tell the person besides, if anything.
    // Human: a picture from words goes onto the layer the dialog was opened for, or becomes a new layer in the
    // middle of the view, selected. A changed picture replaces the layer's picture (its `mode` and the fill's
    // other paints stay) only while the layer still shows the picture that was sent; else, and when the layer is
    // gone, it becomes a new layer instead, and the person is told: it was paid for. One placed beside goes right
    // of the top-level layer, clear of the others, and is selected, since the person asked for it. All go
    // through `checkpoint`, which marks the document changed and in need of layout; it is laid out at once, so
    // the picture shows without waiting for the canvas. A document saved elsewhere meanwhile gets the picture's
    // full path (`lasting_url`); a new layer names the file from where the document is now
    // (`Editor::import_file`).
    // Agent: the tab is found by the document's id, never by its place: tabs can be closed or moved while the
    // picture is made.
    fn place_picture(&mut self, done: &Done, making: &Making, ctx: &egui::Context) -> Result<Option<&'static str>, String> {
        let view = self.canvas_view;
        let target = self.generating.dialog.target.clone().filter(|_| self.generating.dialog.fill);
        let kept = done.saved.path.display().to_string();
        let (tab, editor) = self
            .tabs
            .iter_mut()
            .enumerate()
            .find(|(_, editor)| editor.id == making.doc)
            .ok_or_else(|| format!("The picture was made, but its document was closed. It is kept at {kept}."))?;
        let url = lasting_url(&done.saved, making.folder.as_deref(), editor.folder());
        let visible = editor.visible_area(view.width(), view.height());
        let placed = match &making.goes {
            Goes::Replace { layer, was } if still_as_started(editor, layer, was) => {
                fill_replaced(editor, layer, &url).and_then(|fill| edit_once(editor, &replace_edit(layer, &fill))).map(|_| None)
            }
            Goes::Replace { .. } => editor.import_file(&done.saved.path, visible).map(|()| Some(PICTURE_CHANGED)),
            Goes::Beside { drawn, tops } => {
                let others: Vec<Rect> = editor.scene.iter().map(layout::doc_bounds).collect();
                edit_once(editor, &beside_edit(beside_box(*drawn, *tops, &others), &url)).and_then(|results| {
                    let id = results.first().and_then(|made| made.get("id")).and_then(Value::as_str);
                    editor.selection = vec![id.ok_or("The new layer wasn't found after it was added.")?.to_owned()];
                    Ok(None)
                })
            }
            Goes::Answer | Goes::File | Goes::Fill(_) => {
                let (layer, note) = placing(target, editor);
                let placed = match layer {
                    Some(layer) => fill_with(editor, &layer, &url),
                    None => editor.import_file(&done.saved.path, visible),
                };
                placed.map(|()| note)
            }
        };
        editor.refresh(&super::text_measure(ctx));
        let note = placed.map_err(|why| format!("The picture was made, but it couldn't be placed: {why} It is kept at {kept}."))?;
        // Human: the person sees where the dialog's picture went: its tab comes to the front as choosing it does.
        // The quick field's doesn't move the person: they may have gone on working elsewhere meanwhile, and the
        // message line names the layer instead (`finish_making`).
        if !making.is_quick() {
            self.run(Command::ShowTab(tab), ctx);
        }
        Ok(note)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// What `take` says, in a model's words.
    fn taken(editor: &Editor, pick: &Pick, change: bool) -> Result<Got, String> {
        take(editor, pick, change).map_err(Refused::for_model)
    }

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

    /// A document saved in a folder of its own (named after `name`) beside a picture `a.png`, with layers whose
    /// fills name pictures in each way a fill can.
    fn pictured(name: &str) -> (PathBuf, Editor) {
        let folder = std::env::temp_dir().join(format!("pencraft-pictures-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("a.png"), b"a picture").unwrap();
        std::fs::write(folder.join("b.png"), b"a picture").unwrap();
        let doc = pen_doc::read(
            r##"{"version":"2.22","children":[
              {"type":"rectangle","id":"one","width":10,"height":10,"fill":{"type":"image","url":"./a.png","mode":"contain"}},
              {"type":"rectangle","id":"many","width":10,"height":10,"fill":["#ffffff",{"type":"image","url":"./a.png","mode":"cover"},{"type":"image","url":"./b.png"},"#00000010"]},
              {"type":"rectangle","id":"plain","width":10,"height":10,"fill":"#eeeeee"},
              {"type":"rectangle","id":"web","width":10,"height":10,"fill":{"type":"image","url":"https://example.com/a.png"}},
              {"type":"rectangle","id":"lost","width":10,"height":10,"fill":{"type":"image","url":"./gone.png"}}]}"##,
        )
        .unwrap();
        let editor = Editor::new(doc, Some(folder.join("site.pen")), 7);
        (folder, editor)
    }

    /// A PNG picture of one pixel.
    fn dot() -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(1, 1, image::Rgba([200, 30, 30, 255])).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        bytes.into_inner()
    }

    /// An empty folder of this test's own, named after `name`.
    fn empty(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("pencraft-pictures-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    /// A document without a file whose layers show the picture at `url`: as a fill, among several paints, as a
    /// stroke and in an instance's override; with one undo step behind it that shows it too.
    fn unsaved_with(url: &str) -> Editor {
        let text = r##"{"version":"2.22","children":[
              {"type":"rectangle","id":"one","width":10,"height":10,"fill":{"type":"image","url":"URL","mode":"contain"}},
              {"type":"rectangle","id":"many","width":10,"height":10,"fill":["#ffffff",{"type":"image","url":"URL"}],"stroke":{"type":"image","url":"URL"}},
              {"type":"frame","id":"card","reusable":true,"width":10,"height":10,"children":[{"type":"rectangle","id":"photo","width":10,"height":10}]},
              {"type":"ref","id":"inst","ref":"card","descendants":{"photo":{"fill":{"type":"image","url":"URL"}}}}]}"##
            .replace("URL", url);
        let mut editor = Editor::new(pen_doc::read(&text).unwrap(), None, 7);
        editor.checkpoint();
        pen_doc::edit::find_mut(&mut editor.doc, "one").unwrap().name = Some("Hero".to_owned());
        editor
    }

    #[test]
    fn saving_brings_the_pictures_kept_under_the_home_folder_beside_the_document() {
        let (home, folder) = (empty("settle-home"), empty("settle"));
        let kept = crate::imagery::store::save(None, Some(&home), &dot(), "A fox", &json!({ "made": "generated" })).unwrap();
        let mut editor = unsaved_with(&kept.url);
        editor.path = Some(folder.join("site.pen"));
        assert_eq!(settle(&mut editor, None, Some(&home)), Vec::<String>::new());
        let url = format!("./images/{}", kept.path.file_name().unwrap().to_str().unwrap());
        assert!(folder.join(&url).is_file());
        let written = pen_doc::write(&editor.doc).unwrap();
        assert_eq!(written.matches(&url).count(), 4, "{written}");
        assert!(!written.contains(&kept.url), "{written}");
        // Human: the step before the save names the picture beside the document too, so undoing keeps it portable.
        editor.undo();
        let before = pen_doc::write(&editor.doc).unwrap();
        assert!(before.matches(&url).count() == 4 && !before.contains(&kept.url), "{before}");
        // Human: a later save has nothing left to do, and makes no second copy.
        assert_eq!(settle(&mut editor, Some(&folder.join("site.pen")), Some(&home)), Vec::<String>::new());
        assert_eq!(std::fs::read_dir(folder.join("images")).unwrap().count(), 2, "the picture and the credits");
        std::fs::remove_dir_all(&home).unwrap();
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_document_goes_into_a_bundle_and_out_again_with_its_files() {
        let (folder, outside, back) = (empty("bundled"), empty("bundled-outside"), empty("bundled-back"));
        for (file, bytes) in
            [(folder.join("images/a.png"), &b"a"[..]), (folder.join("fx/glow.glsl"), b"void main(){}"), (outside.join("far.png"), b"far")]
        {
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
        std::fs::write(folder.join("images/credits.json"), r#"{"a.png":{"made":"generated"}}"#).unwrap();
        let far = outside.join("far.png").to_string_lossy().replace('\\', "/");
        let doc = pen_doc::read(
            &json!({ "version": "2.22", "children": [
                { "type": "rectangle", "id": "a", "mask": true, "fill": { "type": "image", "url": "./images/a.png" }, "effect": { "type": "shader", "url": "fx/glow.glsl" } },
                { "type": "rectangle", "id": "b", "fill": { "type": "image", "url": far } },
                { "type": "rectangle", "id": "c", "fill": { "type": "image", "url": "images/gone.png" } },
            ] })
            .to_string(),
        )
        .unwrap();
        let mut editor = Editor::new(doc, Some(folder.join("site.pen")), 7);

        // Into a bundle: every picture comes in, also the one named by a full path, and the shader file.
        let bundle = folder.join("site.quill");
        let was = editor.move_to(bundle.clone()).unwrap();
        assert_eq!(was, Some(folder.join("site.pen")));
        assert_eq!(settle(&mut editor, was.as_deref(), None), Vec::<String>::new());
        let told = editor.save().unwrap();
        assert!(told.len() == 1 && told[0].contains("images/gone.png"), "{told:?}");
        let files = crate::bundle::quill::read(&std::fs::read(&bundle).unwrap(), 1 << 20).unwrap().entries;
        let names: Vec<&str> = files.iter().map(|file| file.name.as_str()).collect();
        assert_eq!(names, ["", "fx/glow.glsl", "images/a.png", "images/far.png", "images/credits.json"]);
        let written = pen_doc::write(&pen_doc::binary::read(&files[0].bytes).unwrap()).unwrap();
        assert!(written.contains("\"./images/far.png\"") && !written.contains(&far), "{written}");
        assert!(written.contains("\"pencraft\": {\n    \"format\": 1,\n    \"features\": [\n      \"mask\"\n    ]"), "{written}");
        assert!(!folder.join("site.pen").exists() && outside.join("far.png").is_file());
        let unpacked = editor.folder().unwrap().to_path_buf();
        editor.close();
        assert!(!unpacked.exists());

        // Opened again, its files lie in a working folder, where its paths start.
        let (mut editor, newer) = Editor::open(&bundle, 7).unwrap();
        assert_eq!(newer, None);
        let unpacked = editor.folder().unwrap().to_path_buf();
        assert!(unpacked != folder && unpacked.join("images/far.png").is_file() && unpacked.join("fx/glow.glsl").is_file());
        assert_eq!(editor.anchor(), Some(unpacked.join("document.pen")));
        // Saved again as it is, the bundle is the same byte for byte.
        let first = std::fs::read(&bundle).unwrap();
        let at = editor.anchor();
        assert_eq!(settle(&mut editor, at.as_deref(), None), Vec::<String>::new());
        editor.save().unwrap();
        assert_eq!(std::fs::read(&bundle).unwrap(), first);

        // Exported as a `.pen` file elsewhere: the files come to lie beside that file, and the document stays as
        // it is, in its `.quill` file, with its unsaved changes.
        editor.dirty = true;
        let before = pen_doc::write(&editor.doc).unwrap();
        let told = export_pen(&editor, &back.join("back.pen"), None).unwrap();
        assert_eq!(told, ["Only Quill draws its masks; other programs that open .pen files leave them out."]);
        assert_eq!((editor.path.as_ref(), editor.folder(), editor.dirty), (Some(&bundle), Some(unpacked.as_path()), true));
        assert_eq!(pen_doc::write(&editor.doc).unwrap(), before);
        assert_eq!(std::fs::read(back.join("images/far.png")).unwrap(), b"far");
        assert_eq!(std::fs::read(back.join("fx/glow.glsl")).unwrap(), b"void main(){}");
        let credits = std::fs::read_to_string(back.join("images/credits.json")).unwrap();
        assert!(credits.contains("a.png"), "{credits}");
        let plain = std::fs::read_to_string(back.join("back.pen")).unwrap();
        assert!(plain.starts_with("{\n  \"version\": \"2.22\",\n  \"pencraft\": {") && plain.contains("\"./images/far.png\""), "{plain}");
        // Exported there again, the files that are there are used and nothing is said.
        assert_eq!(export_pen(&editor, &back.join("back.pen"), None).unwrap().len(), 1);
        assert_eq!(std::fs::read_dir(back.join("images")).unwrap().count(), 3, "two pictures and the credits");
        // When other files lie there under those names, none is written over. The picture is copied under a
        // free name and the exported file names that one; of the shader file it is said that the other is used.
        std::fs::write(back.join("images/far.png"), b"another").unwrap();
        std::fs::write(back.join("fx/glow.glsl"), b"another").unwrap();
        let told = export_pen(&editor, &back.join("back.pen"), None).unwrap();
        assert_eq!(told.len(), 3, "{told:?}");
        assert!(told[0].starts_with("Another picture lies at images/far.png in ") && told[0].contains("copied as ./images/far-0000.png"), "{told:?}");
        assert!(told[1].starts_with("Another file lies at fx/glow.glsl in ") && told[1].ends_with("the exported file uses that one."), "{told:?}");
        assert_eq!(std::fs::read(back.join("images/far.png")).unwrap(), b"another");
        assert_eq!(std::fs::read(back.join("images/far-0000.png")).unwrap(), b"far");
        assert_eq!(std::fs::read(back.join("fx/glow.glsl")).unwrap(), b"another");
        let plain = std::fs::read_to_string(back.join("back.pen")).unwrap();
        assert!(plain.contains("\"./images/far-0000.png\"") && !plain.contains("\"./images/far.png\""), "{plain}");
        assert_eq!(pen_doc::write(&editor.doc).unwrap(), before, "the document in the tab names its pictures as before");
        // The exported file opens as a document of its own, with its pictures beside it.
        let (exported, said) = Editor::open(&back.join("back.pen"), 7).unwrap();
        assert_eq!((said, exported.bundled(), exported.folder()), (None, false, Some(back.as_path())));
        editor.close();
        assert!(!unpacked.exists());
        for folder in [folder, outside, back] {
            std::fs::remove_dir_all(folder).unwrap();
        }
    }

    #[test]
    fn a_file_a_newer_quill_wrote_opens_with_a_sentence() {
        let folder = empty("newer");
        let file = folder.join("new.pen");
        std::fs::write(&file, r#"{"version":"2.40","pencraft":{"format":4,"features":["hologram","mask"]},"children":[]}"#).unwrap();
        let (editor, newer) = Editor::open(&file, 7).unwrap();
        assert!(newer.is_some_and(|said| said.contains("A newer Quill") && said.contains("(hologram)")));
        assert!(!editor.bundled() && editor.folder() == Some(folder.as_path()));
        std::fs::write(&file, r#"{"version":"2.22","children":[]}"#).unwrap();
        assert_eq!(Editor::open(&file, 7).unwrap().1, None);
        assert!(Editor::open(&folder.join("missing.quill"), 7).is_err());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_pencraft_file_is_not_written_and_a_packed_file_named_pen_is_not_read_as_text() {
        let folder = empty("old-bundle");
        let mut editor = Editor::new(pen_doc::Document::default(), None, 7);
        editor.move_to(folder.join("old.pencraft")).unwrap();
        assert!(editor.save().unwrap_err().contains(".quill"));
        assert!(!folder.join("old.pencraft").exists());
        // Saved as a `.quill` file, then given the wrong ending by hand.
        editor.move_to(folder.join("new.quill")).unwrap();
        editor.save().unwrap();
        editor.close();
        std::fs::copy(folder.join("new.quill"), folder.join("wrong.pen")).unwrap();
        assert!(Editor::open(&folder.join("wrong.pen"), 7).err().is_some_and(|said| said.contains("ending .quill")));
        let (mut again, said) = Editor::open(&folder.join("new.quill"), 7).unwrap();
        assert_eq!((said, again.bundled()), (None, true));
        again.close();
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_part_a_later_quill_put_in_a_quill_file_is_still_in_it_after_saving() {
        use crate::bundle::quill;
        let folder = empty("later-part");
        let file = folder.join("later.quill");
        let entry = |kind: u8, name: &str, bytes: Vec<u8>| quill::Entry { kind, name: name.to_owned(), bytes };
        let document = pen_doc::binary::write(&pen_doc::read(r#"{"version":"2.22","children":[]}"#).unwrap()).unwrap();
        // Kind 2 is what a later version keeps the chat in; this one has no name for it.
        let chat = entry(2, "chat", "what was said about the design. ".repeat(50).into_bytes());
        let parts = [entry(quill::DOCUMENT, "", document), chat.clone()];
        std::fs::write(&file, quill::write(&parts).unwrap()).unwrap();
        let (mut editor, said) = Editor::open(&file, 7).unwrap();
        assert_eq!(
            said.as_deref(),
            Some("A newer Quill wrote this file. This version may not draw all of it; what it doesn't know is kept when you save.")
        );
        editor.save().unwrap();
        editor.save().unwrap();
        let saved = quill::read(&std::fs::read(&file).unwrap(), 1 << 20).unwrap();
        assert_eq!((saved.unknown, saved.entries.len()), (vec![chat], 1));
        // Exported as a `.pen` file, which has no place for it: that is said, and the document still holds it.
        let told = export_pen(&editor, &folder.join("plain.pen"), None).unwrap();
        assert_eq!(told, ["Parts a newer Quill added to the .quill file are not in the .pen file."]);
        editor.save().unwrap();
        assert_eq!(quill::read(&std::fs::read(&file).unwrap(), 1 << 20).unwrap().unknown.len(), 1);
        editor.close();
        // A file in a later layout opens too, and is told apart: what saving it here loses can't be known.
        let mut later = quill::write(&parts).unwrap();
        later[8..12].copy_from_slice(&2u32.to_le_bytes());
        std::fs::write(&file, later).unwrap();
        let (mut editor, said) = Editor::open(&file, 7).unwrap();
        assert_eq!(
            said.as_deref(),
            Some(
                "A newer Quill wrote this file. This version may not draw all of it. Saving it here writes this version's layout, so parts this version doesn't know may be left out."
            )
        );
        editor.close();
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_save_under_a_new_name_that_fails_leaves_the_document_at_the_file_it_had() {
        let folder = empty("stays");
        // Nothing can be written beside a file that lies in a file.
        std::fs::write(folder.join("blocked"), b"a file, not a folder").unwrap();
        let old = folder.join("blocked").join("site.pencraft");
        let mut editor = Editor::new(pen_doc::Document::default(), Some(old.clone()), 7);
        let before = editor.place();
        editor.move_to(crate::bundle::successor(&old).unwrap()).unwrap();
        assert!(editor.save_or_return(Some(before)).is_err());
        assert_eq!(editor.path.as_ref(), Some(&old));
        editor.close();
        // A document without a file has none again, and a `.pen` document keeps no working folder of the try.
        let mut editor = Editor::new(pen_doc::Document::default(), None, 7);
        let before = editor.place();
        editor.move_to(folder.join("blocked").join("new.quill")).unwrap();
        let made = editor.folder().unwrap().to_path_buf();
        assert!(editor.bundled() && made.is_dir());
        assert!(editor.save_or_return(Some(before)).is_err());
        assert_eq!((editor.path.as_ref(), editor.bundled(), made.exists()), (None, false, false));
        std::fs::write(folder.join("plain.pen"), r#"{"version":"2.22","children":[]}"#).unwrap();
        let (mut editor, _) = Editor::open(&folder.join("plain.pen"), 7).unwrap();
        let before = editor.place();
        editor.move_to(folder.join("blocked").join("plain.quill")).unwrap();
        assert!(editor.save_or_return(Some(before)).is_err());
        assert_eq!((editor.path.clone(), editor.bundled(), editor.folder()), (Some(folder.join("plain.pen")), false, Some(folder.as_path())));
        // A save that works leaves it where it was saved.
        let before = editor.place();
        editor.move_to(folder.join("site.quill")).unwrap();
        assert!(editor.save_or_return(Some(before)).is_ok());
        assert_eq!(editor.path, Some(folder.join("site.quill")));
        editor.close();
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_failed_save_into_a_quill_file_that_took_pictures_in_stays_where_its_pictures_are() {
        let (folder, outside) = (empty("took-in"), empty("took-in-outside"));
        std::fs::write(folder.join("blocked"), b"a file, not a folder").unwrap();
        std::fs::write(outside.join("far.png"), b"far").unwrap();
        let far = outside.join("far.png").to_string_lossy().replace('\\', "/");
        let doc = json!({ "version": "2.22", "children": [{ "type": "rectangle", "id": "b", "fill": { "type": "image", "url": far } }] });
        let mut editor = Editor::new(pen_doc::read(&doc.to_string()).unwrap(), Some(folder.join("site.pen")), 7);
        let before = editor.place();
        let next = folder.join("blocked").join("site.quill");
        let was = editor.move_to(next.clone()).unwrap();
        // The picture named by a full path is now named from the working folder: going back would lose it.
        assert_eq!(settle(&mut editor, was.as_deref(), None), Vec::<String>::new());
        let said = editor.save_or_return(Some(before)).unwrap_err();
        assert!(said.ends_with(". The document is now site.quill and is not written yet"), "{said}");
        assert_eq!(editor.path, Some(next));
        assert!(editor.folder().unwrap().join("images/far.png").is_file());
        editor.close();
        for folder in [folder, outside] {
            std::fs::remove_dir_all(folder).unwrap();
        }
    }

    #[test]
    fn a_pen_document_is_saved_as_a_quill_file_beside_it_with_its_pictures() {
        let folder = empty("pen-to-quill");
        std::fs::create_dir_all(folder.join("images")).unwrap();
        std::fs::write(folder.join("images/x.png"), b"x").unwrap();
        let text = r#"{"version":"2.22","children":[{"type":"rectangle","id":"a","fill":{"type":"image","url":"./images/x.png"}}]}"#;
        std::fs::write(folder.join("site.pen"), text).unwrap();
        let (mut editor, said) = Editor::open(&folder.join("site.pen"), 7).unwrap();
        assert_eq!(said, None);
        // The `.pen` file is opened, not saved to.
        assert_eq!(editor.save().unwrap_err(), "this file is opened, not saved to: save the document as a .quill file");
        assert_eq!(std::fs::read_to_string(folder.join("site.pen")).unwrap(), text);
        // What Save does with it: the `.quill` file of its name, with the picture that lay beside it.
        let (before, was) = (editor.place(), editor.anchor());
        let next = crate::bundle::successor(&folder.join("site.pen")).unwrap();
        editor.move_to(next.clone()).unwrap();
        assert_eq!(settle(&mut editor, was.as_deref(), None), Vec::<String>::new());
        assert_eq!(editor.save_or_return(Some(before)).unwrap(), Vec::<String>::new());
        assert_eq!((editor.path.as_ref(), editor.bundled(), editor.dirty), (Some(&folder.join("site.quill")), true, false));
        let files = crate::bundle::quill::read(&std::fs::read(&next).unwrap(), 1 << 20).unwrap().entries;
        let held: Vec<(&str, usize)> = files.iter().map(|file| (file.name.as_str(), file.bytes.len())).collect();
        assert_eq!((held[1], held.len(), files[1].bytes.as_slice()), (("images/x.png", 1), 2, &b"x"[..]));
        // The `.pen` file and what lies beside it are as they were.
        assert_eq!(std::fs::read_to_string(folder.join("site.pen")).unwrap(), text);
        assert_eq!(std::fs::read_dir(folder.join("images")).unwrap().count(), 1);
        assert_eq!(std::fs::read(folder.join("images/x.png")).unwrap(), b"x");
        editor.close();
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_document_opened_from_a_file_of_another_name_is_saved_as_a_quill_file_and_exported_with_what_it_names() {
        let (folder, out) = (empty("other-name"), empty("other-name-out"));
        std::fs::create_dir_all(folder.join("site/images")).unwrap();
        std::fs::write(folder.join("site/images/x.png"), b"x").unwrap();
        std::fs::write(folder.join("shared.png"), b"shared").unwrap();
        let text = r#"{"version":"2.22","children":[
            {"type":"rectangle","id":"a","fill":{"type":"image","url":"./images/x.png"}},
            {"type":"rectangle","id":"b","fill":{"type":"image","url":"../shared.png"}}]}"#;
        let file = folder.join("site/design.json");
        std::fs::write(&file, text).unwrap();
        let (mut editor, said) = Editor::open(&file, 7).unwrap();
        assert_eq!((said, editor.bundled()), (None, false));
        // Exported from where it lies: the picture in its folder comes along, and of the one named from
        // outside the folder it is said that the path is kept.
        let told = export_pen(&editor, &out.join("copy.pen"), None).unwrap();
        assert_eq!(told.len(), 1, "{told:?}");
        assert!(told[0].starts_with("The picture ../shared.png lies outside the document's folder"), "{told:?}");
        assert_eq!(std::fs::read(out.join("images/x.png")).unwrap(), b"x");
        assert!(std::fs::read_to_string(out.join("copy.pen")).unwrap().contains("\"../shared.png\""));
        // The file itself is not saved to; Save writes its name with `.quill` after it, and takes both
        // pictures in, so nothing in a `.quill` file is named from outside.
        assert!(editor.save().is_err());
        let (before, was) = (editor.place(), editor.anchor());
        let next = crate::bundle::successor(&file).unwrap();
        assert_eq!(next, folder.join("site/design.json.quill"));
        editor.move_to(next.clone()).unwrap();
        assert_eq!(settle(&mut editor, was.as_deref(), None), Vec::<String>::new());
        assert_eq!(editor.save_or_return(Some(before)).unwrap(), Vec::<String>::new());
        assert_eq!((editor.path.as_ref(), editor.title().as_str()), (Some(&next), "design.json"));
        let files = crate::bundle::quill::read(&std::fs::read(&next).unwrap(), 1 << 20).unwrap().entries;
        let names: Vec<&str> = files.iter().map(|file| file.name.as_str()).collect();
        assert_eq!(names, ["", "images/shared.png", "images/x.png"]);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
        assert_eq!(export_pen(&editor, &out.join("again.pen"), None).unwrap(), Vec::<String>::new());
        assert_eq!(std::fs::read(out.join("images/shared.png")).unwrap(), b"shared");
        editor.close();
        for folder in [folder, out] {
            std::fs::remove_dir_all(folder).unwrap();
        }
    }

    #[test]
    fn saving_under_a_new_name_elsewhere_takes_the_documents_pictures_along() {
        let (folder, mut editor) = pictured("moved");
        let other = empty("moved-to");
        std::fs::create_dir_all(folder.join("images")).unwrap();
        std::fs::write(folder.join("images").join("c.png"), b"c").unwrap();
        std::fs::write(folder.join("images").join("credits.json"), r#"{"c.png":{"made":"generated"}}"#).unwrap();
        pen_doc::edit::find_mut(&mut editor.doc, "plain").unwrap().fill =
            Some(Paints::One(Paint::Object(json!({ "type": "image", "url": "./images/c.png" }).as_object().cloned().unwrap())));
        let before = pen_doc::write(&editor.doc).unwrap();
        editor.path = Some(other.join("copy.pen"));
        assert_eq!(settle(&mut editor, Some(&folder.join("site.pen")), None), Vec::<String>::new());
        assert_eq!(pen_doc::write(&editor.doc).unwrap(), before, "the paths still fit, so the document is as it was");
        for picture in ["a.png", "b.png", "images/c.png"] {
            assert_eq!(std::fs::read(other.join(picture)).unwrap(), std::fs::read(folder.join(picture)).unwrap(), "{picture}");
        }
        assert_eq!(std::fs::read_to_string(other.join("images").join("credits.json")).unwrap().matches("c.png").count(), 1);
        assert!(!other.join("gone.png").exists());
        std::fs::remove_dir_all(&folder).unwrap();
        std::fs::remove_dir_all(&other).unwrap();
    }

    #[test]
    fn a_picture_that_cant_be_brought_is_told_and_keeps_its_path() {
        let (home, folder) = (empty("stuck-home"), empty("stuck"));
        let kept = crate::imagery::store::save(None, Some(&home), &dot(), "A fox", &json!({})).unwrap();
        let mut editor = unsaved_with(&kept.url);
        editor.path = Some(folder.join("site.pen"));
        // Human: a file where the folder for pictures would be.
        std::fs::write(folder.join("images"), b"in the way").unwrap();
        let told = settle(&mut editor, None, Some(&home));
        assert_eq!(told.len(), 1, "one sentence for the one picture, however many layers show it: {told:?}");
        assert!(told[0].starts_with("The picture ") && told[0].contains("couldn't be brought"), "{told:?}");
        assert_eq!(pen_doc::write(&editor.doc).unwrap().matches(&kept.url).count(), 4);
        std::fs::remove_dir_all(&home).unwrap();
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_picture_of_a_layer_is_its_topmost_image_paint_on_disk_or_why_not() {
        let (folder, editor) = pictured("of");
        assert_eq!(image_of(&editor, "one"), Ok(folder.join("./a.png")));
        assert_eq!(image_of(&editor, "many"), Ok(folder.join("./b.png")));
        assert_eq!(image_of(&editor, "plain"), Err("The layer \"plain\" has no picture as its fill.".to_owned()));
        assert_eq!(image_of(&editor, "gone"), Err("There is no layer with the id \"gone\".".to_owned()));
        assert_eq!(
            image_of(&editor, "web"),
            Err("The picture of the layer \"web\" is \"https://example.com/a.png\", which can't be read from disk.".to_owned())
        );
        // Human: whether the file is there is asked on the job's thread (`sources::prepare`), not here.
        assert_eq!(image_of(&editor, "lost"), Ok(folder.join("./gone.png")));
        // Human: a relative path in a document that has no file yet points nowhere.
        let unsaved = Editor::new(editor.doc.clone(), None, 7);
        assert_eq!(
            image_of(&unsaved, "one"),
            Err("The picture of the layer \"one\" is \"./a.png\", which can't be read from disk: the document isn't saved yet, so the path isn't relative to anything.".to_owned())
        );
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_replaced_picture_keeps_the_paints_mode_and_the_fills_other_paints() {
        let (folder, editor) = pictured("replaced");
        assert_eq!(fill_replaced(&editor, "one", "./images/new.jpg"), Ok(json!({ "type": "image", "url": "./images/new.jpg", "mode": "contain" })));
        assert_eq!(
            fill_replaced(&editor, "many", "./images/new.jpg"),
            Ok(
                json!(["#ffffff", { "type": "image", "url": "./a.png", "mode": "cover" }, { "type": "image", "url": "./images/new.jpg" }, "#00000010"])
            )
        );
        assert_eq!(fill_replaced(&editor, "plain", "./images/new.jpg"), Err("The layer \"plain\" has no picture as its fill.".to_owned()));
        assert_eq!(fill_replaced(&editor, "gone", "./images/new.jpg"), Err("There is no layer with the id \"gone\".".to_owned()));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_picture_named_by_a_call_is_taken_from_its_file_or_said_to_be_missing() {
        let (folder, editor) = pictured("take");
        assert_eq!(
            taken(&editor, &Pick::Layer("one".to_owned()), true),
            Ok(Got::Ready(Take::file(folder.join("./a.png"), true, Some("one".to_owned()))))
        );
        assert_eq!(taken(&editor, &Pick::Layer("one".to_owned()), true).map(|got| matches!(got, Got::Ready(take) if take.name == "a.png")), Ok(true));
        assert_eq!(taken(&editor, &Pick::File(folder.join("b.png")), false), Ok(Got::Ready(Take::file(folder.join("b.png"), false, None))));
        // Human: a file that isn't there is found so on the job's thread, which names it (`sources::prepare`).
        let missing = folder.join("nothing.png");
        assert_eq!(taken(&editor, &Pick::File(missing.clone()), false), Ok(Got::Ready(Take::file(missing, false, None))));
        assert_eq!(
            taken(&editor, &Pick::Layer("lost".to_owned()), false),
            Ok(Got::Ready(Take::file(folder.join("./gone.png"), false, Some("lost".to_owned()))))
        );
        assert_eq!(
            taken(&editor, &Pick::Drawn(vec!["one".to_owned(), "plain".to_owned()]), true),
            Ok(Got::Draw(vec!["one".to_owned(), "plain".to_owned()]))
        );
        // Human: a reference may be any layer: one without a picture is sent as it looks.
        assert_eq!(taken(&editor, &Pick::Layer("plain".to_owned()), false), Ok(Got::Draw(vec!["plain".to_owned()])));
        // Human: the picture to change named by `layer` must be a picture; the answer says how to send it drawn.
        assert_eq!(
            taken(&editor, &Pick::Layer("plain".to_owned()), true),
            Err("The layer \"plain\" has no picture as its fill. To change the layer as it looks, name it in `layers` instead.".to_owned())
        );
        assert_eq!(taken(&editor, &Pick::Layer("gone".to_owned()), false), Err("There is no layer with the id \"gone\".".to_owned()));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_picture_of_drawn_layers_goes_beside_the_top_level_layers_they_are_in() {
        let page = Rect { x: 0.0, y: 0.0, w: 1000.0, h: 800.0 };
        let card = Rect { x: 100.0, y: 50.0, w: 300.0, h: 200.0 };
        // A top-level layer drawn: 40 to its right, at its height and size.
        assert_eq!(beside_box(card, card, &[card]), Rect { x: 440.0, y: 50.0, w: 300.0, h: 200.0 });
        // A card inside a page: beside the page, not on it.
        assert_eq!(beside_box(card, page, &[page]), Rect { x: 1040.0, y: 50.0, w: 300.0, h: 200.0 });
        // A neighbour where it would go: stepped past it, and past the one after it.
        let neighbour = Rect { x: 1100.0, y: 0.0, w: 200.0, h: 100.0 };
        let next = Rect { x: 1350.0, y: 200.0, w: 100.0, h: 100.0 };
        let below = Rect { x: 1040.0, y: 400.0, w: 500.0, h: 100.0 };
        assert_eq!(beside_box(card, page, &[page, neighbour, next, below]), Rect { x: 1490.0, y: 50.0, w: 300.0, h: 200.0 });
        // Touching a layer's edge is not lying on it.
        let touching = Rect { x: 1040.0, y: 250.0, w: 100.0, h: 100.0 };
        assert_eq!(beside_box(card, page, &[page, touching]).x, 1040.0);
    }

    #[test]
    fn the_top_level_layers_holding_drawn_ones_are_found() {
        let editor = editor();
        assert_eq!(tops_holding(&editor, &["photo".to_owned()]), Some(Rect { x: 500.0, y: 0.0, w: 100.0, h: 100.0 }));
        assert_eq!(tops_holding(&editor, &["hero".to_owned()]), Some(Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 }));
        assert_eq!(tops_holding(&editor, &["gone".to_owned()]), None);
    }

    #[test]
    fn a_changed_picture_goes_on_its_layer_only_when_the_layers_picture_is_still_the_one_sent() {
        let (folder, mut editor) = pictured("meanwhile");
        assert!(still_as_started(&editor, "one", "./a.png"));
        assert!(still_as_started(&editor, "many", "./b.png"), "the topmost image paint is the one that counts");
        assert!(!still_as_started(&editor, "many", "./a.png"));
        // The user put another picture there, or took it away, or removed the layer.
        fill_with(&mut editor, "one", "./other.png").unwrap();
        assert!(!still_as_started(&editor, "one", "./a.png"));
        assert!(!still_as_started(&editor, "plain", ""));
        assert!(!still_as_started(&editor, "gone", "./a.png"));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn only_a_picture_for_a_layer_waits_for_a_drag() {
        let rect = Rect { x: 0.0, y: 0.0, w: 1.0, h: 1.0 };
        assert!(Goes::Fill("a".to_owned()).to_layer() && Goes::Beside { drawn: rect, tops: rect }.to_layer());
        assert!(Goes::Replace { layer: "a".to_owned(), was: String::new() }.to_layer());
        assert!(!Goes::Answer.to_layer() && !Goes::File.to_layer());
    }

    #[test]
    fn a_picture_of_drawn_layers_fills_its_place() {
        let bounds = Rect { x: 440.0, y: 50.0, w: 300.0, h: 200.0 };
        assert_eq!(
            beside(bounds, "./images/new.jpg", "Edited picture"),
            json!({
                "type": "rectangle", "name": "Edited picture", "x": 440.0, "y": 50.0, "width": 300.0, "height": 200.0,
                "fill": { "type": "image", "url": "./images/new.jpg", "mode": "cover" }
            })
        );
    }

    #[test]
    fn a_small_design_is_drawn_larger_and_a_large_one_smaller() {
        assert!((drawing_scale(Rect { x: 0.0, y: 0.0, w: 300.0, h: 200.0 }) - MOST_ENLARGED).abs() < f32::EPSILON);
        assert!((drawing_scale(Rect { x: 0.0, y: 0.0, w: 100.0, h: 4096.0 }) - 0.5).abs() < f32::EPSILON);
        assert!((drawing_scale(Rect { x: 0.0, y: 0.0, w: 1024.0, h: 10.0 }) - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_layer_added_by_a_change_is_found_in_its_answer() {
        let answer = |value: &Value| vec![json!({ "type": "text", "text": serde_json::to_string_pretty(value).unwrap() })];
        assert_eq!(made_in(&answer(&json!({ "results": [{ "id": "k3" }] }))), Some("k3".to_owned()));
        assert_eq!(made_in(&answer(&json!({ "results": [] }))), None);
    }

    #[test]
    fn a_picture_added_beside_layers_is_one_undo_step() {
        let mut editor = editor();
        let count = |editor: &Editor| editor.doc.children.len();
        let before = count(&editor);
        let results = edit_once(&mut editor, &beside_edit(Rect { x: 0.0, y: 0.0, w: 400.0, h: 300.0 }, "./images/a.png")).unwrap();
        let id = results[0]["id"].as_str().unwrap().to_owned();
        let node = pen_doc::edit::find(&editor.doc, &id).unwrap();
        assert_eq!(serde_json::to_value(&node.fill).unwrap(), json!({ "type": "image", "url": "./images/a.png", "mode": "cover" }));
        assert_eq!(count(&editor), before + 1);
        editor.undo();
        assert_eq!(count(&editor), before);
    }

    #[test]
    fn replacing_a_picture_is_one_undo_step() {
        let (folder, mut editor) = pictured("undo");
        let fill = |editor: &Editor| serde_json::to_value(&pen_doc::edit::find(&editor.doc, "one").unwrap().fill).unwrap();
        let before = fill(&editor);
        let replaced = fill_replaced(&editor, "one", "./images/new.jpg").unwrap();
        edit_once(&mut editor, &replace_edit("one", &replaced)).unwrap();
        assert_eq!(fill(&editor), json!({ "type": "image", "url": "./images/new.jpg", "mode": "contain" }));
        editor.undo();
        assert_eq!(fill(&editor), before);
        std::fs::remove_dir_all(&folder).unwrap();
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
        // Human: its picture can't be changed by `layer` either; as a reference it is drawn as it looks.
        assert_eq!(
            taken(&editor, &Pick::Layer("inst/photo".to_owned()), true),
            Err("\"inst/photo\" is a layer inside a component instance: its picture can't be changed this way. Name it in `layers` to change it as it looks, then set its fill with edit_layers, using the `url` of the picture.".to_owned())
        );
        assert_eq!(taken(&editor, &Pick::Layer("inst/photo".to_owned()), false), Ok(Got::Draw(vec!["inst/photo".to_owned()])));
        assert_eq!(taken(&editor, &Pick::Layer("gone/photo".to_owned()), false), Err("There is no layer with the id \"gone/photo\".".to_owned()));
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
        assert_eq!(room(MOST_AT_ONCE), Err("Quill is already making 3 pictures. Wait for one to finish.".to_owned()));
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

    #[test]
    fn the_quick_field_starts_empty_for_another_layer_and_keeps_its_words_for_its_own() {
        let mut quick = Quick::default();
        quick.tie("hero", 7);
        "Make it dusk".clone_into(&mut quick.text);
        quick.problem = Some("no".to_owned());
        quick.tie("hero", 7);
        assert_eq!((quick.text.as_str(), quick.problem.as_deref()), ("Make it dusk", Some("no")));
        quick.tie("card", 7);
        assert_eq!((quick.layer.as_str(), quick.doc, quick.text.as_str(), quick.problem.as_deref()), ("card", 7, "", None));
        // The same id in another document is another layer.
        "x".clone_into(&mut quick.text);
        quick.tie("card", 8);
        assert!(quick.text.is_empty() && quick.doc == 8);
    }

    #[test]
    fn half_typed_words_come_back_when_their_layer_is_selected_again() {
        let mut quick = Quick::default();
        quick.tie("hero", 7);
        "Make it du".clone_into(&mut quick.text);
        // A click on another layer and back.
        quick.tie("card", 7);
        assert_eq!(quick.text, "");
        quick.tie("hero", 7);
        assert_eq!(quick.text, "Make it du");
        // A problem comes back with its words; a layer left empty doesn't push them out.
        quick.problem = Some("no".to_owned());
        quick.tie("card", 7);
        quick.tie("logo", 7);
        quick.tie("hero", 7);
        assert_eq!((quick.text.as_str(), quick.problem.as_deref()), ("Make it du", Some("no")));
        // Only the last layer left with words is kept.
        quick.tie("card", 7);
        "Bluer".clone_into(&mut quick.text);
        quick.tie("logo", 7);
        quick.tie("hero", 7);
        assert_eq!(quick.text, "");
        quick.tie("card", 7);
        assert_eq!(quick.text, "Bluer");
        // Words kept for a layer whose picture was then changed are dropped, shown or kept.
        quick.tie("hero", 7);
        quick.done_with("card", 7);
        quick.tie("card", 7);
        assert_eq!(quick.text, "");
        "Warmer".clone_into(&mut quick.text);
        quick.done_with("card", 7);
        assert_eq!(quick.text, "");
        // A failure is shown with the words it is about, shown or kept.
        "Warmer".clone_into(&mut quick.text);
        quick.tie("hero", 7);
        quick.failed("card", 7, "Too large.".to_owned());
        quick.tie("card", 7);
        assert_eq!((quick.text.as_str(), quick.problem.as_deref()), ("Warmer", Some("Too large.")));
    }

    #[test]
    fn a_lone_layer_goes_into_the_row_only_with_a_picture_or_children() {
        let (folder, pictured) = pictured("opening");
        let ids = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
        let seen = |(target, pieces): (Option<String>, Vec<Piece>)| (target, pieces.into_iter().map(|piece| piece.what).collect::<Vec<_>>());
        // An empty rectangle: the stage 1 dialog, which offers to fill it.
        assert_eq!(seen(opening(&pictured, &ids(&["plain"]))), (Some("plain".to_owned()), vec![]));
        // A rectangle with a picture: in the row, marked as the picture to change, and not also a target.
        let (target, pieces) = opening(&pictured, &ids(&["one"]));
        assert_eq!(target, None);
        let mut dialog = Dialog::default();
        dialog.open_for(target, &[Offered { name: "Gemini".to_owned(), edits: true }], "");
        dialog.set_pieces(pieces);
        assert!(dialog.editing() && dialog.has_pieces());
        // Several layers: all in the row, none a target.
        assert_eq!(
            seen(opening(&pictured, &ids(&["one", "plain"]))),
            (None, vec![PieceOf::Layer { id: "one".to_owned(), has_picture: true }, PieceOf::Layer { id: "plain".to_owned(), has_picture: false }])
        );
        std::fs::remove_dir_all(&folder).unwrap();
        // A frame with children: in the row, sent as drawn (a reference until marked), never a target, since a
        // picture in its fill would lie under its children.
        let editor = editor();
        assert_eq!(seen(opening(&editor, &ids(&["card"]))), (None, vec![PieceOf::Layer { id: "card".to_owned(), has_picture: false }]));
        // A text: nothing in the row, and nothing to fill.
        assert_eq!(seen(opening(&editor, &ids(&["title"]))), (None, vec![]));
        assert_eq!(seen(opening(&editor, &[])), (None, vec![]));
    }

    #[test]
    fn files_chosen_go_only_to_the_dialog_they_were_chosen_for() {
        assert!(for_the_dialog(true, false, Some(7), 7));
        assert!(!for_the_dialog(false, false, Some(7), 7), "closed meanwhile");
        assert!(!for_the_dialog(true, true, Some(7), 7), "making its picture");
        assert!(!for_the_dialog(true, false, Some(8), 7), "opened again for another document");
        assert!(!for_the_dialog(true, false, None, 7));
    }

    #[test]
    fn a_persons_picture_waits_for_the_canvas_and_is_never_lost_or_placed_twice() {
        assert_eq!(step::<u8>(None, None, false), Step::Wait);
        assert_eq!(step::<u8>(None, None, true), Step::Wait);
        // There, while the canvas is busy: held.
        assert_eq!(step(None, Some(Ok(1)), true), Step::Hold(1));
        // Held, and the canvas is still busy: held again; free: placed.
        assert_eq!(step(Some(1), None, true), Step::Hold(1));
        assert_eq!(step(Some(1), None, false), Step::Place(Ok(1)));
        assert_eq!(step(None, Some(Ok(1)), false), Step::Place(Ok(1)));
        // A failure is said at once, busy or not.
        assert_eq!(step::<u8>(None, Some(Err("no".to_owned())), true), Step::Place(Err("no".to_owned())));
    }

    #[test]
    fn the_message_line_names_the_layer_whose_picture_the_quick_field_changed_elsewhere() {
        assert_eq!(quick_told("Hero photo", true, &Ok(None)), None);
        assert_eq!(quick_told("Hero photo", false, &Ok(None)).as_deref(), Some("The picture of \"Hero photo\" was changed."));
        assert_eq!(quick_told("Hero photo", true, &Ok(Some(PICTURE_CHANGED))).as_deref(), Some(PICTURE_CHANGED));
        assert_eq!(quick_told("Hero photo", true, &Err("Too large.".to_owned())), None);
        assert_eq!(
            quick_told("Hero photo", false, &Err("Too large.".to_owned())).as_deref(),
            Some("The picture of \"Hero photo\" couldn't be changed: Too large.")
        );
    }

    #[test]
    fn a_person_is_told_what_is_wrong_without_a_models_words() {
        let (folder, editor) = pictured("person");
        let said = |pick: &Pick, change: bool| take(&editor, pick, change).map_err(Refused::for_person).err();
        assert_eq!(said(&Pick::Layer("plain".to_owned()), true).as_deref(), Some(PERSON_NO_PICTURE));
        assert_eq!(said(&Pick::Layer("gone".to_owned()), false).as_deref(), Some(PERSON_GONE));
        let instance = super::tests::editor();
        assert_eq!(
            take(&instance, &Pick::Layer("inst/photo".to_owned()), true).map_err(Refused::for_person).err().as_deref(),
            Some(PERSON_IN_INSTANCE)
        );
        for sentence in [PERSON_NO_PICTURE, PERSON_GONE, PERSON_IN_INSTANCE] {
            assert!(!sentence.contains('`') && !sentence.contains("edit_layers"), "{sentence}");
        }
        // Human: the model's words are the same as before.
        assert!(Refused::NoPicture("x".to_owned()).said(true).ends_with("name it in `layers` instead."));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_quick_field_asks_what_the_dialog_would_for_its_layer_alone() {
        let asked = quick_asked("Make it dusk", "hero", "Gemini");
        assert_eq!(asked, json!({ "instruction": "Make it dusk", "service": "Gemini", "size": "normal", "layer": "hero", "references": [] }));
        // Human: read by the very functions a model's call goes through: the layer is the picture to change, and
        // without `aspect` the picture keeps its shape.
        let named = imagery::named(&asked).unwrap();
        assert_eq!((named.change, named.references), (Some(Pick::Layer("hero".to_owned())), vec![]));
        let gemini = imagery::Ready {
            name: "Gemini".to_owned(),
            kind: imagery::Kind::Gemini,
            base_url: String::new(),
            model: "m".to_owned(),
            key: imagery::KeyFrom::Environment("GEMINI_API_KEY"),
        };
        assert_eq!(imagery::edit_request(&asked, &gemini, true).map(|request| (request.aspect, request.large)), Ok((None, false)));
    }

    #[test]
    fn the_dialogs_row_holds_the_selected_layers_by_name_and_whether_they_have_a_picture() {
        let (folder, editor) = pictured("row");
        let pieces = pieces_of(&editor, &["one".to_owned(), "plain".to_owned(), "gone".to_owned()]);
        let seen: Vec<(PieceOf, String)> = pieces.into_iter().map(|piece| (piece.what, piece.name)).collect();
        assert_eq!(
            seen,
            [
                (PieceOf::Layer { id: "one".to_owned(), has_picture: true }, "Rectangle".to_owned()),
                (PieceOf::Layer { id: "plain".to_owned(), has_picture: false }, "Rectangle".to_owned())
            ]
        );
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_picture_put_beside_a_layer_goes_right_of_its_top_level_layer() {
        let editor = editor();
        assert_eq!(
            beside_of(&editor, &["photo".to_owned()]),
            Some(Goes::Beside { drawn: Rect { x: 500.0, y: 0.0, w: 100.0, h: 60.0 }, tops: Rect { x: 500.0, y: 0.0, w: 100.0, h: 100.0 } })
        );
        assert_eq!(beside_of(&editor, &["gone".to_owned()]), None);
    }

    #[test]
    fn a_small_picture_of_a_file_is_made_or_found_impossible() {
        let folder = std::env::temp_dir().join(format!("pencraft-pictures-thumb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let wide = folder.join("wide.png");
        image::RgbaImage::from_pixel(512, 256, image::Rgba([10, 20, 30, 255])).save(&wide).unwrap();
        assert_eq!(file_thumb(&wide).map(|image| image.size), Some([128, 64]));
        let small = folder.join("small.png");
        image::RgbaImage::from_pixel(8, 4, image::Rgba([10, 20, 30, 255])).save(&small).unwrap();
        assert_eq!(file_thumb(&small).map(|image| image.size), Some([8, 4]));
        std::fs::write(folder.join("page.png"), b"<html>").unwrap();
        assert!(file_thumb(&folder.join("page.png")).is_none());
        assert!(file_thumb(&folder.join("missing.png")).is_none());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_change_made_at_once_is_one_undo_step_and_says_what_it_made() {
        let mut editor = editor();
        let before = editor.doc.children.len();
        let results = edit_once(&mut editor, &beside_edit(Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, "./images/a.png")).unwrap();
        assert!(results[0]["id"].as_str().is_some());
        assert_eq!(editor.doc.children.len(), before + 1);
        editor.undo();
        assert_eq!(editor.doc.children.len(), before);
        assert!(edit_once(&mut editor, &json!({ "operations": [] })).is_err());
    }
}
