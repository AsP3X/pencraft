//! One open document: its file, the document itself, the laid-out scene, selection, camera and undo history.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use pen_doc::edit::{self, IdMaker};
use pen_doc::geom::{Point, Rect};
use pen_doc::layout::{self, Measure};
use pen_doc::model::Kind;
use pen_doc::scene::{self, SceneNode};
use pen_doc::vars::Vars;
use pen_doc::{Document, Node};
use serde_json::{Map, Value};

/// How many steps undo goes back.
// Human: undo stores whole documents, so it keeps the newest 100 to bound memory.
pub const UNDO_LIMIT: usize = 100;
/// Least and most zoom.
pub const MIN_ZOOM: f32 = 0.02;
/// See [`MIN_ZOOM`].
pub const MAX_ZOOM: f32 = 256.0;
/// Margin around the design when zooming to fit, screen points.
pub const FIT_PADDING: f32 = 40.0;

/// The tools of the floating toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Select and move (V).
    Move,
    /// Pan the canvas (H).
    Hand,
    /// Draw a frame (F or A).
    Frame,
    /// Draw a rectangle (R).
    Rectangle,
    /// Draw an ellipse (O).
    Ellipse,
    /// Draw a polygon.
    Polygon,
    /// Place an icon.
    Icon,
    /// Scale the selection with everything in it (K).
    Scale,
    /// Place a text (T).
    Text,
    /// Place a sticky note (N).
    Note,
    /// Draw a path point by point (P).
    Pen,
    /// Draw a connector from one layer to another (C).
    Connector,
    /// Take the colour under the pointer for the selected layers (I).
    Eyedropper,
}

/// What a layer is to components.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// An ordinary layer.
    Plain,
    /// A reusable component: the origin its instances show.
    Component,
    /// An instance of a component (a `ref` layer).
    Instance,
    /// A layer inside an instance; it belongs to the component, and changes to it are overrides.
    Inside,
}

/// Where the canvas looks: the document point at the centre of the view, and the zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Document point shown at the centre of the canvas.
    pub center: Point,
    /// Screen points per document unit.
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Camera { center: Point::new(0.0, 0.0), zoom: 1.0 }
    }
}

/// The number the next opened document gets.
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// The node with the scene id `id` among `nodes` and what is in them.
#[must_use]
pub fn find_in<'a>(nodes: &'a [SceneNode], id: &str) -> Option<&'a SceneNode> {
    nodes.iter().find_map(|n| if n.id == id { Some(n) } else { find_in(&n.children, id) })
}

/// Where each node of a scene lies in it, by its scene id: the place of each node on the way down to it, from
/// the top level.
// Human: the canvas asks for the scene node of every selected layer several times a frame. Going through the
// whole scene for each was a second's work for a thousand selected layers in a large design; looked up here
// it is a few steps down the tree.
// Agent: only `Editor::refresh` writes it, in the same step that writes `Editor::scene`, so it is never older
// than the scene. A path that doesn't lead to its id (which can't happen while that holds) is not trusted:
// `Editor::scene_node` then searches as it did before.
#[derive(Default)]
struct Places {
    at: HashMap<String, Box<[u32]>>,
    /// An id was there twice. The first one in drawing order is the one found, which only building the
    /// whole map in that order keeps true.
    doubled: bool,
}

impl Places {
    /// The map for the whole of `scene`.
    fn of(scene: &[SceneNode]) -> Places {
        let mut places = Places::default();
        for (at, root) in scene.iter().enumerate() {
            places.add(root, at);
        }
        places
    }

    /// Takes in `root`, the top-level node at `at`, and everything in it.
    fn add(&mut self, root: &SceneNode, at: usize) {
        self.add_at(root, &[u32::try_from(at).unwrap_or(u32::MAX)]);
    }

    /// Takes in `node`, which lies at `path`, and everything in it.
    fn add_at(&mut self, node: &SceneNode, path: &[u32]) {
        fn visit(places: &mut Places, node: &SceneNode, path: &mut Vec<u32>) {
            match places.at.entry(node.id.clone()) {
                std::collections::hash_map::Entry::Occupied(_) => places.doubled = true,
                std::collections::hash_map::Entry::Vacant(free) => {
                    free.insert(path.as_slice().into());
                }
            }
            for (index, child) in node.children.iter().enumerate() {
                path.push(u32::try_from(index).unwrap_or(u32::MAX));
                visit(places, child, path);
                path.pop();
            }
        }
        visit(self, node, &mut path.to_vec());
    }

    /// Forgets `root`, which was the top-level node at `at`, and everything in it.
    fn remove(&mut self, root: &SceneNode, at: usize) {
        self.remove_at(root, &[u32::try_from(at).unwrap_or(u32::MAX)]);
    }

    /// Forgets `node`, which lay at `path`, and everything in it.
    fn remove_at(&mut self, node: &SceneNode, path: &[u32]) {
        node.walk(
            &mut |inner, _| {
                if self.at.get(&inner.id).is_some_and(|known| known.starts_with(path)) {
                    self.at.remove(&inner.id);
                }
            },
            0,
        );
    }

    /// [`Places::follow`] to change the node.
    fn follow_mut<'a>(scene: &'a mut [SceneNode], path: &[u32]) -> Option<&'a mut SceneNode> {
        let (first, rest) = path.split_first()?;
        let mut node = scene.get_mut(usize::try_from(*first).ok()?)?;
        for index in rest {
            node = node.children.get_mut(usize::try_from(*index).ok()?)?;
        }
        Some(node)
    }

    /// The node `path` leads to in `scene`.
    fn follow<'a>(scene: &'a [SceneNode], path: &[u32]) -> Option<&'a SceneNode> {
        let (first, rest) = path.split_first()?;
        let mut node = scene.get(usize::try_from(*first).ok()?)?;
        for index in rest {
            node = node.children.get(usize::try_from(*index).ok()?)?;
        }
        Some(node)
    }
}

/// A number no scene and no top-level node of one has had yet, in any open document.
fn next_number() -> u64 {
    static LAST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    LAST.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
}

/// The ids the instances in `node` show (their `ref`), `node` itself included.
fn shown_by(node: &Node) -> Vec<String> {
    let mut shown: Vec<String> = node.reference.iter().cloned().collect();
    shown.extend(node.kids().iter().flat_map(shown_by));
    shown
}

/// Whether `node` or anything in it is a component, or is among `shown` (the ids instances show).
fn holds_shown(node: &Node, shown: &HashSet<&str>) -> bool {
    node.reusable == Some(true) || shown.contains(node.id.as_str()) || node.kids().iter().any(|kid| holds_shown(kid, shown))
}

/// A copy of a document that shares what it can with other copies of it: each top-level node is kept once,
/// however many copies hold it.
// Human: undo used to keep a whole document for every step, a hundred of them: on a design of 11,000 layers
// that was two gigabytes. An edit changes one screen of dozens, so a copy made after it now holds that one
// screen anew and the others together with the copy before.
// Agent: nothing in a `Kept` is ever changed in place; a changed node is another `Arc`. `rest` is the document
// without its top-level nodes (`children` empty). Can go to another thread (recovery writes from one).
#[derive(Clone)]
pub struct Kept {
    rest: Arc<Document>,
    roots: Vec<Arc<Node>>,
}

impl Kept {
    /// A copy of `doc` that shares what `with` (an earlier copy) holds unchanged: a top-level node that is
    /// as it was there, at the same place or under the same id, is taken from it, and only the others are
    /// copied. Without `with` it shares nothing yet.
    // Agent: READS every node of `doc` once to compare it; ALLOCATES only for what changed.
    #[must_use]
    pub fn sharing(doc: &Document, with: Option<&Kept>) -> Kept {
        // Human: named field by field, so a field the document gets later can't be left out without the
        // compiler saying so.
        let Document { version, pencraft, children, themes, imports, variables, extra } = doc;
        let same_rest = |held: &&Kept| {
            let Document {
                version: was_version,
                pencraft: was_pencraft,
                children: _,
                themes: was_themes,
                imports: was_imports,
                variables: was_variables,
                extra: was_extra,
            } = &*held.rest;
            (was_version, was_pencraft, was_themes, was_imports, was_variables, was_extra) == (version, pencraft, themes, imports, variables, extra)
        };
        let rest = with.filter(same_rest).map_or_else(
            || {
                Arc::new(Document {
                    version: version.clone(),
                    pencraft: pencraft.clone(),
                    children: Vec::new(),
                    themes: themes.clone(),
                    imports: imports.clone(),
                    variables: variables.clone(),
                    extra: extra.clone(),
                })
            },
            |held| held.rest.clone(),
        );
        let was: &[Arc<Node>] = with.map_or(&[], |held| &held.roots);
        // The nodes of the earlier copy by their ids, made when a node isn't found at its place.
        let mut by_id: Option<HashMap<&str, &Arc<Node>>> = None;
        let share = |(at, node): (usize, &Node)| {
            match was.get(at) {
                Some(held) if **held == *node => return held.clone(),
                // The same node, changed: it is nowhere else.
                Some(held) if held.id == node.id => return Arc::new(node.clone()),
                _ => {}
            }
            let by_id = by_id.get_or_insert_with(|| was.iter().map(|held| (held.id.as_str(), held)).collect());
            match by_id.get(node.id.as_str()) {
                Some(held) if ***held == *node => Arc::clone(*held),
                _ => Arc::new(node.clone()),
            }
        };
        Kept { rest, roots: children.iter().enumerate().map(share).collect() }
    }

    /// The document this is a copy of, as one of its own.
    // Agent: ALLOCATES the whole document.
    #[must_use]
    pub fn document(&self) -> Document {
        let mut doc = Document::clone(&self.rest);
        doc.children = self.roots.iter().map(|node| Node::clone(node)).collect();
        doc
    }
}

/// One open document.
pub struct Editor {
    /// A number of its own among the documents opened in this run; it stays the same however its tab moves.
    pub id: u64,
    /// Where it is saved, a `.quill` file ([`crate::bundle`]), or the `.pen` file or old bundle it was opened
    /// from, which Save leaves for the `.quill` file of its name; `None` until first saved.
    pub path: Option<PathBuf>,
    /// The folder a bundle's files lie unpacked in; `None` for a `.pen` file.
    work: Option<crate::bundle::Work>,
    /// The parts its `.quill` file had that this version doesn't know; they are written back when it is saved
    /// as one ([`crate::bundle::pack`]).
    kept: Vec<crate::bundle::quill::Entry>,
    /// The document as the user made it.
    pub doc: Document,
    /// The document with instances expanded and laid out; rebuilt by [`Editor::refresh`].
    pub scene: Vec<SceneNode>,
    /// Bumped whenever the scene is rebuilt, so cached drawing knows to rebuild.
    pub generation: u64,
    /// Selected document node ids, in selection order.
    pub selection: Vec<String>,
    /// The view.
    pub camera: Camera,
    /// The active tool.
    pub tool: Tool,
    /// The theme value chosen for each theme axis in the document panel (`Mode: Dark`), on top of the defaults.
    pub theme_choice: Vec<(String, String)>,
    /// A part of the document the canvas is asked to bring into view; taken by [`Editor::take_focus`].
    pub focus: Option<Rect>,
    /// The layers find layers has found, outlined on the canvas, and the one its arrows are on.
    pub spotlight: (Vec<String>, Option<String>),
    /// The path whose points are being edited, if one is.
    pub path_edit: Option<crate::path_edit::PathEdit>,
    /// The path the pen tool is drawing in this document; empty while none is.
    pub pen: crate::pen::Draft,
    /// How much of the canvas the floating islands cover at its left, top, right and bottom edge, in screen
    /// points; the app sets it each frame and zoom-to-fit keeps the design out from under them.
    pub covered: [f32; 4],
    /// What the Export section is set to make, and whether its button was pressed (the app then asks where to).
    pub export: (crate::export::Options, bool),
    /// How far the Scale tool has scaled the selection so far (1 = as it was), and the selection that is for.
    pub scaled: (f32, Vec<String>),
    /// What an AI model working on this document shows on the canvas.
    pub activity: crate::activity::Activity,
    /// The documents to step back to, each with the number its state had ([`Editor::state`]).
    undo: VecDeque<(Kept, u64)>,
    /// The documents to step forward to again, each with its state's number.
    redo: Vec<(Kept, u64)>,
    /// The last copy made of the document: the next one shares with it whatever has not changed since.
    held: Option<Kept>,
    /// The number of the state `held` is a copy of ([`Editor::state`]); `None` when it is of none.
    // Human: while the document is still in that state, the copy is the document: a step back, a step
    // forward and the scene after either take it as it is and compare nothing.
    held_state: Option<u64>,
    /// What was known of where the document changed when the last undo step was made, until something else
    /// changes the document: the edit the step was made for can then say which layers it changed
    /// ([`Editor::changed_only`]).
    before_step: Option<Tracked>,
    /// A number for the document as it is now; every change gives it one no state of this document had.
    // Human: this is how the editor knows whether there are unsaved changes: the file holds the state with
    // the number `saved`. Undoing back to that state, or redoing forward to it, lands on its number again
    // and the document counts as saved; a new edit after an undo throws the states ahead away, and with
    // them the saved one, which then can't be reached any more.
    state: u64,
    /// The last number given out for a state of this document.
    last_state: u64,
    /// The number of the state the file holds; `None` when no state that can be reached is the file's.
    saved: Option<u64>,
    ids: IdMaker,
    needs_layout: bool,
    needs_fit: bool,
    /// What the scene was last built from, to tell which of its top-level nodes an edit changed.
    laid: Laid,
    /// Where each node of [`Editor::scene`] lies in it.
    places: Places,
    /// Where the document changed since the scene was last brought up to date.
    tracked: Tracked,
    /// The document as it was when the scene was last brought up to date, to check what was tracked against.
    #[cfg(debug_assertions)]
    checked: Option<Kept>,
}

/// What is known of where the document changed since its scene was last brought up to date.
// Human: a drag changes one layer, frame after frame. Finding that layer by comparing the whole document
// with what it was, and then building its whole screen again, was most of what a drag in a large screen
// cost. The edits that know which layers they change say so, and only those layers are built again.
// Agent: `Layers` is a promise: nothing changed outside those layers and what is in them. Every way of
// changing the document that doesn't make it leaves `All`, and the scene is found by comparing, as before.
#[derive(Default)]
enum Tracked {
    /// Nothing changed.
    #[default]
    Nothing,
    /// Only these layers changed, and what is in them.
    Layers(Vec<String>),
    /// Anything may have changed.
    All,
}

impl Tracked {
    /// What is known when, on top of this, `ids` and nothing else changed.
    fn with(self, ids: &[String]) -> Tracked {
        match self {
            Tracked::Nothing => Tracked::Layers(ids.to_vec()),
            Tracked::Layers(mut layers) => {
                layers.extend(ids.iter().filter(|id| !layers.contains(id)).cloned().collect::<Vec<_>>());
                Tracked::Layers(layers)
            }
            Tracked::All => Tracked::All,
        }
    }
}

/// A top-level node of the scene of which only some layers were built again when the scene was last brought
/// up to date.
// Human: the canvas keeps what it drew of the node and draws only those layers anew. It can tell that what it
// drew is what the layers were taken out of: the node had the stamp `from` then, and has `to` now.
pub struct Rebuilt {
    /// The node's place in the scene.
    pub root: usize,
    /// The node's stamp before, and after ([`Editor::stamps`]).
    pub from: u64,
    /// See `from`.
    pub to: u64,
    /// Where the layers built again lie in the scene: each one's place among the nodes on the way down to it,
    /// from the top level. None lies in another.
    pub paths: Vec<Vec<u32>>,
}

/// What a scene was built from, and what is known of each of its top-level nodes.
// Human: a design is dozens of screens side by side, and an edit changes one of them. Building and laying out
// only that one, and drawing only that one anew, is what keeps a large design quick to work on.
#[derive(Default)]
struct Laid {
    /// The document without its top-level nodes: its variables, themes and imports.
    rest: Option<Arc<Document>>,
    /// The top-level nodes, as they were; shared with the copies undo keeps ([`Kept`]).
    nodes: Vec<Arc<Node>>,
    /// The theme that was in force at the top level.
    theme: pen_doc::vars::ThemeContext,
    /// A number for each top-level scene node, another one each time it is built again.
    stamps: Vec<u64>,
    /// The ids each top-level node's instances show (`ref`), anywhere in it.
    shows: Vec<Vec<String>>,
    /// Whether each top-level scene node has a connector in it.
    wired: Vec<bool>,
    /// Whether a part of each top-level scene node was built again by itself since `nodes` was written: that
    /// node is then no longer what its scene node was built from, and counts as changed when compared.
    loose: Vec<bool>,
    /// The top-level nodes of which only some layers were built again the last time ([`Editor::rebuilt`]).
    rebuilt: Vec<Rebuilt>,
    /// Whether everything is to be built again whatever changed (fonts became ready, the theme changed).
    all: bool,
}

impl Editor {
    /// An editor for `doc`, saved at `path` if it has one.
    #[must_use]
    pub fn new(doc: Document, path: Option<PathBuf>, seed: u64) -> Editor {
        Editor {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            path,
            work: None,
            kept: Vec::new(),
            doc,
            scene: Vec::new(),
            generation: 0,
            selection: Vec::new(),
            camera: Camera::default(),
            tool: Tool::Move,
            theme_choice: Vec::new(),
            focus: None,
            spotlight: (Vec::new(), None),
            path_edit: None,
            pen: crate::pen::Draft::default(),
            covered: [0.0; 4],
            scaled: (1.0, Vec::new()),
            export: (crate::export::Options::default(), false),
            activity: crate::activity::Activity::default(),
            undo: VecDeque::new(),
            redo: Vec::new(),
            held: None,
            held_state: None,
            before_step: None,
            state: 0,
            last_state: 0,
            saved: Some(0),
            ids: IdMaker::new(seed),
            needs_layout: true,
            needs_fit: true,
            laid: Laid::default(),
            places: Places::default(),
            tracked: Tracked::All,
            #[cfg(debug_assertions)]
            checked: None,
        }
    }

    /// An editor for the design file at `path`: a `.pen` document, or a bundle, which is unpacked. With it, a
    /// sentence for what the user should know about the file: a newer Quill wrote it and this one doesn't
    /// know all of it, or a file in it was damaged and left out.
    ///
    /// # Errors
    /// A sentence when the file can't be read or isn't a design file.
    // Agent: READS `path`; for a bundle WRITES its working folder. BLOCKS on both.
    pub fn open(path: &Path, seed: u64) -> Result<(Editor, Option<String>), String> {
        let (doc, work, layout, told, kept, later) = if crate::bundle::is_bundle(path) {
            let opened = crate::bundle::open(path)?;
            (opened.doc, Some(opened.work), opened.newer, opened.told, opened.kept, opened.later)
        } else {
            let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
            // Human: a packed design under a `.pen` name would be read as text and fail with words about JSON.
            if crate::bundle::looks_packed(&bytes) {
                return Err("it is a packed design: give its name the ending .quill".to_owned());
            }
            let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
            (pen_doc::read(&text).map_err(|e| e.to_string())?, None, None, Vec::new(), Vec::new(), false)
        };
        let said: Vec<String> = newer_than_this(&doc, layout, later).into_iter().chain(told).collect();
        let mut editor = Editor::new(doc, Some(path.to_path_buf()), seed);
        editor.work = work;
        editor.kept = kept;
        Ok((editor, (!said.is_empty()).then(|| said.join(" "))))
    }

    /// The folder the document's relative paths start in: that of its `.pen` file, or where its bundle lies
    /// unpacked. `None` until first saved.
    #[must_use]
    pub fn folder(&self) -> Option<&Path> {
        match self.unpacked() {
            Some(work) => Some(work.folder()),
            None => self.path.as_deref().and_then(Path::parent),
        }
    }

    /// The document's file as its pictures see it: the `.pen` file itself, or a bundle's document in its
    /// working folder. `None` until first saved.
    #[must_use]
    pub fn anchor(&self) -> Option<PathBuf> {
        match self.unpacked() {
            Some(work) => Some(work.document()),
            None => self.path.clone(),
        }
    }

    /// The working folder, while the document is saved as a bundle.
    // Human: a document whose file is a `.pen` file has none: its paths start beside that file.
    fn unpacked(&self) -> Option<&crate::bundle::Work> {
        self.work.as_ref().filter(|_| self.path.as_deref().is_some_and(crate::bundle::is_bundle))
    }

    /// Whether the document has a bundle's working folder: it is saved as a bundle, or was in this run.
    #[must_use]
    pub fn bundled(&self) -> bool {
        self.work.is_some()
    }

    /// Says the document is to be saved at `path` from now on, and gives the [`Editor::anchor`] it had. Going
    /// to a bundle, it gets a working folder when it has none.
    ///
    /// # Errors
    /// A sentence when a working folder can't be made; the document then stays where it was.
    // Agent: WRITES a working folder when the document had none and `path` is a bundle.
    pub fn move_to(&mut self, path: PathBuf) -> Result<Option<PathBuf>, String> {
        let was = self.anchor();
        if crate::bundle::is_bundle(&path) && self.work.is_none() {
            self.work = Some(crate::bundle::Work::fresh()?);
        }
        self.path = Some(path);
        Ok(was)
    }

    /// A sentence naming what the document uses that only Quill draws, for a document saved as a `.pen`
    /// file other programs open too; `None` when it uses nothing of the kind.
    #[must_use]
    pub fn only_here(&self) -> Option<String> {
        let labels: Vec<&str> = pen_doc::stamp::features(&self.doc).into_iter().map(pen_doc::stamp::Feature::label).collect();
        let list = match labels.as_slice() {
            [] => return None,
            [one] => (*one).to_owned(),
            [first @ .., last] => format!("{} and {last}", first.join(", ")),
        };
        Some(format!("Only Quill draws its {list}; other programs that open .pen files leave them out."))
    }

    /// Lets go of the working folder, when the document is closed.
    // Agent: WRITES (removes the folder).
    pub fn close(&mut self) {
        if let Some(work) = self.work.take() {
            work.discard();
        }
    }

    /// The document's name: its file's name without the ending (`site` for `site.quill` and for `site.pen`), or
    /// "Untitled".
    #[must_use]
    pub fn title(&self) -> String {
        self.path.as_ref().and_then(|p| p.file_stem()).map_or_else(|| "Untitled".to_owned(), |s| s.to_string_lossy().into_owned())
    }

    /// What the file's tab shows: the whole file name with its ending, or "Untitled".
    #[must_use]
    pub fn tab_name(&self) -> String {
        self.path.as_ref().and_then(|p| p.file_name()).map_or_else(|| "Untitled".to_owned(), |s| s.to_string_lossy().into_owned())
    }

    /// Records the document before a change, for undo. Call once per user action, before changing `doc`.
    pub fn checkpoint(&mut self) {
        let now = self.keep();
        self.undo.push_back((now, self.state));
        if self.undo.len() > UNDO_LIMIT {
            self.undo.pop_front();
        }
        self.redo.clear();
        self.new_state();
        self.needs_layout = true;
        self.before_step = Some(std::mem::replace(&mut self.tracked, Tracked::All));
    }

    /// Says, after the change an undo step was made for ([`Editor::checkpoint`]), that it was made to the
    /// layers `ids` and what is in them, and to nothing else: only they are built again. A layer that
    /// comes, goes or changes places is a change to the layer it lies in; `None` stands for the top level,
    /// whose layers are compared to find which came and went.
    // Human: one edit (a delete, a paste, what a model asked for) changes a few layers of thousands. Without
    // this the whole document is compared with what it was, and the whole screen the layers are in is built.
    // Agent: the same promise as [`Editor::touched_only`], checked the same way in debug builds. It counts
    // only when nothing else changed the document since the step was made; otherwise everything is
    // compared, as before. A layer that isn't in the scene (a new one) is found by comparing, too.
    pub fn changed_only(&mut self, ids: &[Option<String>]) {
        if let Some(known) = self.before_step.take()
            && let Some(ids) = ids.iter().cloned().collect::<Option<Vec<String>>>()
        {
            self.tracked = known.with(&ids);
        }
    }

    /// Marks the document changed without a new undo step (continuing a drag).
    pub fn touched(&mut self) {
        self.new_state();
        self.needs_layout = true;
        self.tracked = Tracked::All;
        self.before_step = None;
    }

    /// [`Editor::touched`] for a change that is made to the layers `ids` and what is in them, and to nothing
    /// else: only they are built again. A layer that comes, goes or changes places is a change to the layer
    /// it lies in.
    // Agent: a promise the caller makes. In debug builds [`Editor::refresh`] checks it against a comparison
    // of the whole document; in release builds a change outside `ids` is not seen until something else is
    // built again.
    pub fn touched_only(&mut self, ids: &[String]) {
        let known = std::mem::take(&mut self.tracked);
        self.touched();
        self.tracked = known.with(ids);
    }

    /// [`Editor::touched_only`] for what [`Editor::change`] wrote for the layer with `id`: the layer itself,
    /// or, for an instance's inside, the instance, whose overrides were written.
    pub fn touched_layer(&mut self, id: &str) {
        let written = if edit::find(&self.doc, id).is_some() { None } else { edit::owner(&self.doc, id) };
        self.touched_only(&[written.map_or_else(|| id.to_owned(), |(instance, _)| instance)]);
    }

    /// The document is in a state it has not been in: it gets a number of its own.
    fn new_state(&mut self) {
        self.last_state += 1;
        self.state = self.last_state;
    }

    /// Whether there are changes not yet saved: the document is not in the state its file holds.
    #[must_use]
    pub fn dirty(&self) -> bool {
        self.saved != Some(self.state)
    }

    /// Counts the document as unsaved in every state it can reach: no file holds any of them (a document
    /// brought back from a run that didn't end properly, which has no file).
    pub fn mark_unsaved(&mut self) {
        self.saved = None;
    }

    /// Puts `doc`, kept from a run that didn't end properly, in the place of the document: its changes count as
    /// unsaved, and one step back is the document as its file has it.
    pub fn restore(&mut self, doc: Document) {
        self.checkpoint();
        self.doc = doc;
        self.after_history();
    }

    /// Steps back one change; while the pen is drawing a path, that is its last point.
    pub fn undo(&mut self) {
        if self.pen.pop() {
            return;
        }
        if let Some((previous, state)) = self.undo.pop_back() {
            let now = self.keep();
            self.put(&now, previous);
            self.redo.push((now, std::mem::replace(&mut self.state, state)));
            self.held_state = Some(state);
            self.after_history();
        }
    }

    /// Steps forward again after an undo.
    pub fn redo(&mut self) {
        if let Some((next, state)) = self.redo.pop() {
            let now = self.keep();
            self.put(&now, next);
            self.undo.push_back((now, std::mem::replace(&mut self.state, state)));
            self.held_state = Some(state);
            self.after_history();
        }
    }

    /// A copy of the document as it is now, sharing with the last copy made whatever is as it was then.
    // Agent: READS every node to compare it with the last copy; ALLOCATES only the top-level nodes that changed.
    #[must_use]
    pub fn kept(&self) -> Kept {
        Kept::sharing(&self.doc, self.held.as_ref())
    }

    /// How many top-level nodes the steps back and forward hold between them, each counted once however many
    /// steps share it, and how many layers those are with everything in them.
    #[cfg(test)]
    pub(crate) fn history_holds(&self) -> (usize, usize) {
        fn layers(node: &Node) -> usize {
            1 + node.kids().iter().map(layers).sum::<usize>()
        }
        let mut seen: HashMap<*const Node, usize> = HashMap::new();
        for root in self.undo.iter().chain(&self.redo).flat_map(|(past, _)| &past.roots) {
            seen.entry(Arc::as_ptr(root)).or_insert_with(|| layers(root));
        }
        (seen.len(), seen.values().sum())
    }

    /// [`Editor::kept`], remembered as the last copy made.
    // Human: the last copy is taken as it is while the document is in the state it was made of: every
    // change gives the document another state, so nothing can have changed.
    // Agent: debug builds compare all the same, and stop at a change that gave the document no new state.
    fn keep(&mut self) -> Kept {
        if let Some(held) = self.held.as_ref().filter(|_| self.held_state == Some(self.state)) {
            #[cfg(debug_assertions)]
            {
                let now = Kept::sharing(&self.doc, Some(held));
                let same = Arc::ptr_eq(&now.rest, &held.rest)
                    && now.roots.len() == held.roots.len()
                    && now.roots.iter().zip(&held.roots).all(|(is, was)| Arc::ptr_eq(is, was));
                debug_assert!(same, "the document changed without getting a new state");
            }
            return held.clone();
        }
        let now = self.kept();
        self.held = Some(now.clone());
        self.held_state = Some(self.state);
        now
    }

    /// Makes `to` the document, which `now` is a copy of as it is at this moment.
    // Human: a step back or forward changes one screen of dozens. The top-level nodes both copies share
    // stay in the document as they are; only the others are copied out of `to`.
    fn put(&mut self, now: &Kept, to: Kept) {
        let mut have: Vec<Option<Node>> = std::mem::take(&mut self.doc.children).into_iter().map(Some).collect();
        if !Arc::ptr_eq(&now.rest, &to.rest) {
            self.doc = Document::clone(&to.rest);
        }
        // Where each node of `now` lies, by where it is kept; made when a node isn't found at its place.
        let mut places: Option<HashMap<*const Node, usize>> = None;
        let mut take = |(at, kept): (usize, &Arc<Node>)| {
            let from = if now.roots.get(at).is_some_and(|here| Arc::ptr_eq(here, kept)) {
                Some(at)
            } else {
                let places = places.get_or_insert_with(|| now.roots.iter().enumerate().map(|(at, here)| (Arc::as_ptr(here), at)).collect());
                places.get(&Arc::as_ptr(kept)).copied()
            };
            from.and_then(|at| have.get_mut(at)).and_then(Option::take).unwrap_or_else(|| Node::clone(kept))
        };
        self.doc.children = to.roots.iter().enumerate().map(&mut take).collect();
        self.held = Some(to);
        // The caller gives the document the state `to` is a copy of.
        self.held_state = None;
    }

    fn after_history(&mut self) {
        self.needs_layout = true;
        self.tracked = Tracked::All;
        self.before_step = None;
        let doc = &self.doc;
        // A layer inside an instance stays selected while its instance is still there.
        self.selection.retain(|id| edit::find(doc, id).is_some() || edit::owner(doc, id).is_some());
    }

    /// Whether a zoom-to-fit is waiting; clears the request.
    pub fn take_fit_request(&mut self) -> bool {
        std::mem::take(&mut self.needs_fit)
    }

    /// The part of the document the canvas should bring into view, if one is waiting; clears the request.
    pub fn take_focus(&mut self) -> Option<Rect> {
        self.focus.take()
    }

    /// The theme in force at the top level: the document's defaults with the document panel's choices.
    #[must_use]
    pub fn root_theme(&self) -> pen_doc::vars::ThemeContext {
        let mut theme = pen_doc::vars::ThemeContext::defaults(&self.doc);
        for (axis, value) in &self.theme_choice {
            theme.set(axis, value);
        }
        theme
    }

    /// Rebuilds the scene and its layout if the document changed.
    // Agent: MAIN-THREAD; ALLOCATES the scene; CALLS measure for every text node.
    // Human: only the top-level nodes that changed are built and laid out again, when that is all that
    // changed (`changed_roots`); the others keep their scene nodes and their stamps.
    pub fn refresh(&mut self, measure: &Measure<'_>) {
        if !self.needs_layout {
            return;
        }
        self.needs_layout = false;
        // Human: the numbers are not this document's own but every open document's together: the canvas of a
        // window draws whichever document's tab is in front, and tells by these numbers what it has drawn.
        self.generation = next_number();
        let theme = self.root_theme();
        self.laid.rebuilt.clear();
        if let Tracked::Layers(ids) = std::mem::take(&mut self.tracked)
            && let Some(patched) = self.patch(&ids, &theme, measure)
        {
            self.route_wires();
            #[cfg(debug_assertions)]
            self.check_tracked(&patched, measure);
            #[cfg(not(debug_assertions))]
            drop(patched);
            return;
        }
        let now = self.keep();
        #[cfg(debug_assertions)]
        {
            self.checked = Some(now.clone());
        }
        let Some(plan) = self.changed_roots(&theme, &now) else {
            self.scene = self.laid_out(measure);
            self.places = Places::of(&self.scene);
            let laid = &mut self.laid;
            (laid.nodes, laid.rest, laid.theme, laid.all) = (now.roots, Some(now.rest), theme, false);
            laid.stamps = self.scene.iter().map(|_| next_number()).collect();
            laid.shows = laid.nodes.iter().map(|node| shown_by(node)).collect();
            laid.wired = self.scene.iter().map(pen_doc::connector::has_connector).collect();
            laid.loose = vec![false; laid.nodes.len()];
            return;
        };
        let vars = Vars::new(&self.doc);
        let changed: Vec<usize> = plan.iter().enumerate().filter(|(_, from)| from.is_none()).map(|(at, _)| at).collect();
        let mut built = scene::build_some(&self.doc, &theme, &changed);
        layout::layout_roots(&mut built, vars, measure);
        let mut built = built.into_iter();
        // Human: when every node that stays is where it was (an edit inside nodes, or nodes that come and go
        // at the end), the map of where the layers lie is put right for the nodes built; when nodes changed
        // places it is made anew, since every layer's way down starts at its top-level node's place.
        let in_place = plan.iter().enumerate().all(|(at, from)| from.is_none_or(|from| from == at));
        let mut scene: Vec<Option<SceneNode>> = std::mem::take(&mut self.scene).into_iter().map(Some).collect();
        if in_place {
            for (at, gone) in scene.iter().enumerate().filter(|(at, _)| plan.get(*at).is_none_or(Option::is_none)) {
                if let Some(gone) = gone {
                    self.places.remove(gone, at);
                }
            }
        }
        let was = std::mem::take(&mut self.laid);
        let laid = &mut self.laid;
        (laid.rest, laid.theme, laid.all) = (was.rest, was.theme, false);
        for (at, from) in plan.iter().enumerate() {
            let kept =
                from.and_then(|from| Some((scene.get_mut(from)?.take()?, *was.stamps.get(from)?, was.shows.get(from)?, *was.wired.get(from)?)));
            let Some(node) = now.roots.get(at) else { continue };
            let (root, stamp, shows, wired) = match (kept, from) {
                (Some((root, stamp, shows, wired)), _) => (root, stamp, shows.clone(), wired),
                (None, None) => {
                    let Some(root) = built.next() else { continue };
                    if in_place {
                        self.places.add(&root, at);
                    }
                    let wired = pen_doc::connector::has_connector(&root);
                    (root, next_number(), shown_by(node), wired)
                }
                // Not to be: a node to keep that the scene doesn't have. Everything is built again.
                (None, Some(_)) => {
                    laid.all = true;
                    continue;
                }
            };
            self.scene.push(root);
            laid.nodes.push(node.clone());
            laid.stamps.push(stamp);
            laid.shows.push(shows);
            laid.wired.push(wired);
            laid.loose.push(false);
        }
        if self.laid.all || self.scene.len() != now.roots.len() {
            self.laid.all = true;
            self.needs_layout = true;
            self.tracked = Tracked::All;
            return self.refresh(measure);
        }
        if !in_place {
            self.places = Places::of(&self.scene);
        }
        self.route_wires();
        if cfg!(test) {
            debug_assert!(self.scene == self.laid_out(measure), "the scene built in part is not the scene built whole");
        }
    }

    /// What every way of building part of the scene again ends with: the map of where its nodes lie is put
    /// right, and the connectors are routed.
    fn route_wires(&mut self) {
        // Human: an id that is there twice is found where it comes first in the whole scene, which the map
        // only keeps to when it is built whole.
        if self.places.doubled {
            self.places = Places::of(&self.scene);
        }
        // Human: a connector follows layers that may lie in another top-level node, so every one is routed
        // again, and every top-level node with a connector in it counts as changed.
        if self.laid.wired.contains(&true) {
            pen_doc::connector::route(&mut self.scene, Vars::new(&self.doc));
            for (stamp, _) in self.laid.stamps.iter_mut().zip(&self.laid.wired).filter(|(_, wired)| **wired) {
                *stamp = next_number();
            }
        }
    }

    /// Builds again only what lies in the layers `ids`, which is all that changed since the scene was built,
    /// and gives the places of the top-level nodes they lie in; `None`, with the scene as it was, when
    /// that can't be done and the document has to be compared instead.
    // Human: each layer is built again by itself where its box is its own affair. Where it is not (it lies
    // in a group, or takes part in a layout that places it and may take its size from it), the layer it
    // lies in is built again instead, and so on upwards; a layout thus follows a drag inside it.
    // Human: it can't be done when the scene is to be built whole anyway, when a layer isn't found where
    // the scene has it (it is new, or layers changed places), when the layer to build again would be a
    // top-level node (those are compared, as before), or when it holds a component or anything an instance
    // shows, or lies in one: the instances have to follow.
    // Agent: what can't be done is found before anything is written, so `None` leaves the scene as it was. (A
    // top-level node that was written to all the same is marked `loose`, and the comparison builds it again.)
    fn patch(&mut self, ids: &[String], theme: &pen_doc::vars::ThemeContext, measure: &Measure<'_>) -> Option<Vec<usize>> {
        let laid = &self.laid;
        let count = self.doc.children.len();
        if laid.all
            || laid.theme != *theme
            || self.places.doubled
            || [laid.nodes.len(), laid.stamps.len(), laid.shows.len(), laid.wired.len(), laid.loose.len(), self.scene.len()] != [count; 6]
        {
            return None;
        }
        let vars = Vars::new(&self.doc);
        let shown: HashSet<&str> = laid.shows.iter().flatten().map(String::as_str).collect();
        let mut paths: Vec<Vec<u32>> = Vec::new();
        for id in ids {
            let path = self.places.at.get(id)?;
            let (doc_nodes, scene_nodes) = Self::along(&self.doc, &self.scene, path)?;
            // What an instance that is new in the layer shows counts too.
            let new = *doc_nodes.last()?;
            let new_shows = shown_by(new);
            let known: HashSet<&str> = shown.iter().copied().chain(new_shows.iter().map(String::as_str)).collect();
            // The layers it lies in: none may be a component or shown by an instance.
            let (_, above) = doc_nodes.split_last()?;
            if above.iter().any(|node| node.reusable == Some(true) || known.contains(node.id.as_str())) {
                return None;
            }
            // How far up the layer to build again lies: as far as its box is not its own affair.
            let mut depth = doc_nodes.len() - 1;
            loop {
                if depth == 0 {
                    return None;
                }
                let (parent, was, is) = (scene_nodes[depth - 1], scene_nodes[depth], doc_nodes[depth]);
                let theme_now = parent.theme.with(is.theme.as_ref());
                if layout::stands_alone(parent, &was.node, &was.theme, vars) && layout::stands_alone(parent, is, &theme_now, vars) {
                    break;
                }
                depth -= 1;
            }
            let (was, is) = (scene_nodes[depth], doc_nodes[depth]);
            let mut held = false;
            was.walk(&mut |node, _| held |= node.node.reusable == Some(true) || node.doc_id.as_deref().is_some_and(|id| known.contains(id)), 0);
            if held || holds_shown(is, &known) {
                return None;
            }
            paths.push(path.get(..=depth)?.to_vec());
        }
        // A layer that lies in another one built again comes with it.
        paths.sort();
        paths.dedup_by(|path, before| path.starts_with(before));
        let mut patched: Vec<usize> = Vec::new();
        for path in &paths {
            let (at, above) = (usize::try_from(*path.first()?).ok()?, path.get(..path.len() - 1)?);
            let (doc_nodes, _) = Self::along(&self.doc, &self.scene, path)?;
            let parent = Places::follow(&self.scene, above)?;
            let mut built = scene::build_node(&self.doc, doc_nodes.last()?, &parent.theme);
            layout::layout_within(&mut built, &parent.to_doc, vars, measure);
            let instances = !shown_by(doc_nodes.last()?).is_empty();
            let place = Places::follow_mut(&mut self.scene, path)?;
            let had_instances = {
                let mut any = false;
                place.walk(&mut |node, _| any |= node.node.kind == Kind::Ref, 0);
                any
            };
            self.places.remove_at(place, path);
            self.places.add_at(&built, path);
            self.laid.wired[at] |= pen_doc::connector::has_connector(&built);
            *place = built;
            if instances || had_instances {
                self.laid.shows[at] = self.doc.children.get(at).map(shown_by).unwrap_or_default();
            }
            let from = std::mem::replace(&mut self.laid.stamps[at], next_number());
            let to = self.laid.stamps[at];
            self.laid.loose[at] = true;
            match self.laid.rebuilt.iter_mut().find(|some| some.root == at) {
                Some(some) => {
                    some.to = to;
                    some.paths.push(path.clone());
                }
                None => self.laid.rebuilt.push(Rebuilt { root: at, from, to, paths: vec![path.clone()] }),
            }
            if !patched.contains(&at) {
                patched.push(at);
            }
        }
        Some(patched)
    }

    /// The nodes of `doc` and of `scene` that `path` (a place in the scene) leads through, from the top-level
    /// node to the one it ends at; `None` when the document doesn't have the same nodes there.
    // Human: inside an instance the scene has the component's layers, which are no nodes of the document;
    // no path leads through one for this.
    fn along<'a>(doc: &'a Document, scene: &'a [SceneNode], path: &[u32]) -> Option<(Vec<&'a Node>, Vec<&'a SceneNode>)> {
        let (mut doc_nodes, mut scene_nodes) = (Vec::with_capacity(path.len()), Vec::with_capacity(path.len()));
        let (mut kids, mut children): (&[Node], &[SceneNode]) = (&doc.children, scene);
        for (step, index) in path.iter().enumerate() {
            let index = usize::try_from(*index).ok()?;
            let (node, scene_node) = (kids.get(index)?, children.get(index)?);
            let last = step + 1 == path.len();
            if scene_node.doc_id.as_deref() != Some(node.id.as_str())
                || (!last && (node.kind == Kind::Ref || node.kids().len() != scene_node.children.len()))
            {
                return None;
            }
            doc_nodes.push(node);
            scene_nodes.push(scene_node);
            (kids, children) = (node.kids(), &scene_node.children);
        }
        Some((doc_nodes, scene_nodes))
    }

    /// Checks what was tracked against a comparison of the whole document: every top-level node that
    /// changed since the scene was last brought up to date must be among `patched`, and nothing else of the
    /// document may have changed. In tests the scene must also be what building it whole gives.
    // Human: an edit that says it changed one layer and changed another would leave that other one on the
    // canvas as it was. A debug build stops at such an edit, where it is found at once.
    #[cfg(debug_assertions)]
    fn check_tracked(&mut self, patched: &[usize], measure: &Measure<'_>) {
        let now = Kept::sharing(&self.doc, self.checked.as_ref());
        if let Some(was) = &self.checked {
            debug_assert!(Arc::ptr_eq(&was.rest, &now.rest), "a tracked edit changed the document's variables, themes or imports");
            debug_assert_eq!(was.roots.len(), now.roots.len(), "a tracked edit added or removed a top-level node");
            for (at, (before, after)) in was.roots.iter().zip(&now.roots).enumerate() {
                debug_assert!(Arc::ptr_eq(before, after) || patched.contains(&at), "a tracked edit changed {} without saying so", after.id);
            }
        }
        self.checked = Some(now);
        if cfg!(test) {
            debug_assert!(self.scene == self.laid_out(measure), "the scene built in part is not the scene built whole");
        }
    }

    /// The top-level nodes of which only some layers were built again when the scene was last brought up to
    /// date; of every other node whose stamp is another one, everything was.
    #[must_use]
    pub fn rebuilt(&self) -> &[Rebuilt] {
        &self.laid.rebuilt
    }

    /// A number for each top-level node of [`Editor::scene`] that is another one each time that node is built
    /// again: what was drawn for a node whose number is the same still holds.
    #[must_use]
    pub fn stamps(&self) -> &[u64] {
        &self.laid.stamps
    }

    /// Where each top-level node of the document comes from: its place in the scene as it is, when it is as
    /// it was when the scene was built, or `None` when it changed or is new and has to be built. `None` for
    /// all of it when the whole scene has to be built again.
    // Human: everything is built again when what all nodes share changed (variables, themes, the theme
    // chosen, the fonts), and when a node that changed, came or went holds a component or anything else an
    // instance somewhere shows: that instance has to follow.
    // Human: top-level nodes that come, go or change places are found by what they are, not by where they
    // are: a pasted screen, or one that is deleted, leaves every other screen's scene as it is.
    // Agent: `now` is the document as it is ([`Editor::keep`]). A node that is as it was is the same `Arc`
    // in `now` as in what was laid out; one that is not the same `Arc` counts as changed, which at worst
    // builds a node again that didn't need it.
    fn changed_roots(&self, theme: &pen_doc::vars::ThemeContext, now: &Kept) -> Option<Vec<Option<usize>>> {
        let laid = &self.laid;
        let count = laid.nodes.len();
        if laid.all
            || laid.theme != *theme
            || [laid.stamps.len(), laid.shows.len(), laid.wired.len(), laid.loose.len(), self.scene.len()] != [count; 5]
            || !laid.rest.as_ref().is_some_and(|rest| Arc::ptr_eq(rest, &now.rest))
        {
            return None;
        }
        let usable =
            |from: usize, is: &Arc<Node>| laid.loose.get(from) == Some(&false) && laid.nodes.get(from).is_some_and(|was| Arc::ptr_eq(was, is));
        // The nodes as they were by where they are kept; made when one isn't found at its place.
        let mut by_address: Option<HashMap<*const Node, usize>> = None;
        let mut used = vec![false; count];
        let mut plan: Vec<Option<usize>> = Vec::with_capacity(now.roots.len());
        for (at, is) in now.roots.iter().enumerate() {
            let from = if usable(at, is) {
                Some(at)
            } else if laid.nodes.get(at).is_some_and(|was| was.id == is.id) {
                // The same node, changed: it is nowhere else.
                None
            } else {
                let places = by_address.get_or_insert_with(|| laid.nodes.iter().enumerate().map(|(from, was)| (Arc::as_ptr(was), from)).collect());
                places.get(&Arc::as_ptr(is)).copied().filter(|from| usable(*from, is))
            };
            let from = from.filter(|from| used.get_mut(*from).is_some_and(|used| !std::mem::replace(used, true)));
            plan.push(from);
        }
        let shown: HashSet<&str> = laid.shows.iter().flatten().map(String::as_str).collect();
        let gone = laid.nodes.iter().zip(&used).filter(|(_, used)| !**used).map(|(was, _)| was);
        let new = now.roots.iter().zip(&plan).filter(|(_, from)| from.is_none()).map(|(is, _)| is);
        if gone.chain(new).any(|node| holds_shown(node, &shown)) {
            return None;
        }
        Some(plan)
    }

    /// The document as it is now, with instances expanded and laid out; [`Editor::scene`] is not touched.
    // Agent: MAIN-THREAD; ALLOCATES a scene; CALLS measure for every text node.
    #[must_use]
    pub fn laid_out(&self, measure: &Measure<'_>) -> Vec<SceneNode> {
        let mut scene = scene::build(&self.doc, &self.root_theme());
        layout::layout(&mut scene, Vars::new(&self.doc), measure);
        scene
    }

    /// Whether the document changed, or the faces of its text did, since it was last laid out.
    #[must_use]
    pub fn stale(&self) -> bool {
        self.needs_layout
    }

    /// Forces a relayout of everything (after the theme or the fonts changed).
    pub fn relayout(&mut self) {
        self.needs_layout = true;
        self.laid.all = true;
        self.tracked = Tracked::All;
        // Whoever asks for everything to be built again may have written to the document: it is compared
        // with the last copy made, not taken for it.
        self.held_state = None;
        self.before_step = None;
    }

    /// The laid-out scene node for an id: a document node's id, or `instance/inner` for a node inside an instance.
    #[must_use]
    // Agent: HOT-PATH (several times a frame for every selected layer); a few steps by `Editor::places`.
    pub fn scene_node(&self, id: &str) -> Option<&SceneNode> {
        let path = self.places.at.get(id)?;
        Places::follow(&self.scene, path).filter(|node| node.id == id).or_else(|| find_in(&self.scene, id))
    }

    /// The scene node that holds the one with `id`.
    #[must_use]
    pub fn scene_parent(&self, id: &str) -> Option<&SceneNode> {
        let path = self.places.at.get(id)?;
        let (_, above) = path.split_last()?;
        Places::follow(&self.scene, above).filter(|parent| parent.children.iter().any(|child| child.id == id))
    }

    /// What the layer with `id` is to components.
    #[must_use]
    pub fn role(&self, id: &str) -> Role {
        match edit::find(&self.doc, id) {
            Some(node) if node.kind == Kind::Ref => Role::Instance,
            Some(node) if node.reusable == Some(true) => Role::Component,
            None if edit::owner(&self.doc, id).is_some() => Role::Inside,
            Some(_) | None => Role::Plain,
        }
    }

    /// Applies `change` to the layer with `id`, wherever it lives. Returns whether the layer was found.
    // Human: a plain layer or a component is changed itself. An instance, or a layer inside one, isn't a layer
    // of its own: it shows the component. So the change is made to a copy of what is shown, and whatever
    // differs afterwards is written to the instance as an override; the component and its other instances
    // stay as they are.
    // Agent: READS the scene as last laid out; WRITES self.doc only. The caller makes the undo step.
    pub fn change(&mut self, id: &str, change: &dyn Fn(&mut Node)) -> bool {
        if let Some(node) = edit::find_mut(&mut self.doc, id)
            && node.kind != Kind::Ref
        {
            change(node);
            return true;
        }
        let Some(shown) = self.scene_node(id).map(|scene| scene.node.clone()) else { return false };
        let mut changed = shown.clone();
        change(&mut changed);
        let (Ok(Value::Object(before)), Ok(Value::Object(after))) = (serde_json::to_value(&shown), serde_json::to_value(&changed)) else {
            return false;
        };
        let mut overrides: Map<String, Value> =
            after.iter().filter(|(key, value)| before.get(*key) != Some(value)).map(|(k, v)| (k.clone(), v.clone())).collect();
        overrides.extend(before.keys().filter(|key| !after.contains_key(*key)).map(|key| (key.clone(), Value::Null)));
        if overrides.is_empty() {
            return true;
        }
        match edit::owner(&self.doc, id) {
            Some((instance, key)) => edit::set_overrides(&mut self.doc, &instance, Some(&key), &overrides),
            None => edit::set_overrides(&mut self.doc, id, None, &overrides),
        }
    }

    /// The document-space bounds of the selection.
    #[must_use]
    pub fn selection_bounds(&self) -> Option<Rect> {
        self.selection.iter().filter_map(|id| self.scene_node(id)).map(layout::doc_bounds).reduce(|a, b| a.union(&b))
    }

    /// The bounds of everything on the canvas.
    #[must_use]
    pub fn content_bounds(&self) -> Option<Rect> {
        self.scene.iter().map(layout::doc_bounds).reduce(|a, b| a.union(&b))
    }

    /// A fresh id for a new node.
    pub fn fresh_id(&mut self) -> String {
        let mut taken = edit::ids(&self.doc);
        self.ids.fresh(&mut taken)
    }

    /// `node` (and its descendants) with fresh ids, for pasting and duplicating.
    pub fn renew_ids(&mut self, node: &mut Node) {
        let mut taken: HashSet<String> = edit::ids(&self.doc);
        self.ids.renew(node, &mut taken);
    }

    /// Asks `new` about the `url` of every image paint in the document, once per `url`, and writes the one it
    /// gives in its place; `None` leaves a `url` as it is. The documents undo and redo go to get the same new
    /// `url`s, so stepping back doesn't bring an old one back.
    // Human: not an undo step and not a change to save: it is done while saving, to say where a picture lies now.
    pub fn repicture(&mut self, new: &mut dyn FnMut(&str) -> Option<String>) {
        // The copy of the document as it is before anything is written: it is written like undo's copies below.
        self.held = Some(self.kept());
        // What is written below is written into that copy too, so it stays the document in this state.
        self.held_state = Some(self.state);
        let mut moved: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();
        image_urls(&mut self.doc, &mut |url| {
            if let Some(to) = moved.entry(url.clone()).or_insert_with(|| new(url)) {
                url.clone_from(to);
            }
        });
        if moved.values().all(Option::is_none) {
            return;
        }
        // Human: the copies undo keeps share their nodes, and must go on sharing them: a node is written
        // anew once, and every copy that held the old one gets that new one. The copy of the document as
        // it is now is among them, so the next step shares with these.
        // Agent: `done` holds each old node until the end, so no other node can take its address meanwhile.
        let mut done: HashMap<*const Node, (Arc<Node>, Arc<Node>)> = HashMap::new();
        let copies = self.undo.iter_mut().chain(&mut self.redo).map(|(past, _)| past).chain(&mut self.held);
        for root in copies.flat_map(|past| &mut past.roots) {
            let (_, new) = done.entry(Arc::as_ptr(root)).or_insert_with(|| {
                let (mut written, mut any) = (Node::clone(root), false);
                node_image_urls(&mut written, &mut |url| {
                    if let Some(Some(to)) = moved.get(url.as_str()) {
                        url.clone_from(to);
                        any = true;
                    }
                });
                (root.clone(), if any { Arc::new(written) } else { root.clone() })
            });
            *root = new.clone();
        }
        self.needs_layout = true;
        self.tracked = Tracked::All;
    }

    /// Writes the document to its file, crash-safely: a `.quill` file with the files the document names. Stamps it as Quill's first ([`pen_doc::stamp`]). Gives a sentence for what the user
    /// should know about the file that was written.
    ///
    /// # Errors
    /// When there is no path yet, the path is not a `.quill` file's (a `.pen` document and an old `.pencraft`
    /// bundle are opened, not saved to), the document can't be written, or the file can't be.
    // Human: a document is saved as a `.quill` file and as nothing else. A `.pen` file is made by an export
    // (`pictures::export_pen` in the app), which leaves the document where it is.
    // Agent: WRITES the file; READS the files it names, whole. BLOCKS on both.
    pub fn save(&mut self) -> Result<Vec<String>, String> {
        let Some(path) = self.path.clone() else { return Err("the document has no file yet".to_owned()) };
        // Human: nothing writes the old format any more; whoever saves gives the file the new ending first.
        if crate::bundle::is_old(&path) {
            return Err("a .pencraft file is no longer written: save it as a .quill file".to_owned());
        }
        let Some(work) = self.unpacked() else {
            return Err("this file is opened, not saved to: save the document as a .quill file".to_owned());
        };
        let folder = work.folder().to_path_buf();
        pen_doc::stamp::apply(&mut self.doc);
        // The stamp is written without a new state: the last copy made is no longer the document.
        self.held_state = None;
        let mut told = Vec::new();
        let packed = crate::bundle::pack(&self.doc, &folder, &self.kept)?;
        if !packed.missing.is_empty() {
            told.push(format!("These files the document names weren't found and aren't in the .quill file: {}.", packed.missing.join(", ")));
        }
        craft_atomic::atomic_write(&path, &packed.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        // Human: the file now holds the document as it is; stepping back or forward from here leaves it.
        self.saved = Some(self.state);
        Ok(told)
    }

    /// Whether the document holds parts of its `.quill` file that this version doesn't know: they are written
    /// back into a `.quill` file, and a `.pen` file has no place for them.
    #[must_use]
    pub fn keeps_parts(&self) -> bool {
        !self.kept.is_empty()
    }

    /// Where the document is now, to go back to when a save under another name fails
    /// ([`Editor::save_or_return`]). Taken before [`Editor::move_to`].
    #[must_use]
    pub fn place(&self) -> Place {
        Place { path: self.path.clone(), anchor: self.anchor(), unpacked: self.work.is_some(), doc: self.doc.clone() }
    }

    /// Saves as [`Editor::save`] does, for a document that was moved ([`Editor::move_to`]) from `from` only so
    /// that this save writes it under another name: when the save fails, its file is the one it had again, and
    /// a working folder the move made is removed.
    ///
    /// # Errors
    /// As [`Editor::save`]; when the document could not go back, the sentence says under which name it is now,
    /// unwritten.
    // Human: when the new file can't be written the document must not point at a file that isn't there. It
    // goes back only when nothing in it was changed for the new place, or the new place is where its pictures
    // lay already (an old bundle saved under the ending `.quill`): pictures that were taken into a working
    // folder and named from there (`pictures::settle`) would not be found from the old file. Such a document
    // stays at the name it was given, where saving again writes it.
    // Agent: WRITES and READS as [`Editor::save`]; BLOCKS on both. WRITES (removes a working folder).
    pub fn save_or_return(&mut self, from: Option<Place>) -> Result<Vec<String>, String> {
        let moved = from.is_some();
        let back = from.filter(|from| from.anchor == self.anchor() || from.doc == self.doc);
        let saved = self.save();
        let Err(error) = saved else { return saved };
        match back {
            Some(back) => {
                self.path = back.path;
                if !back.unpacked {
                    self.close();
                }
                Err(error)
            }
            // Human: the user must know that the tab's name is now one no file has.
            None if moved => Err(format!("{error}. The document is now {} and is not written yet", self.tab_name())),
            None => Err(error),
        }
    }
}

/// Where a document was before it was moved to be saved under another name ([`Editor::place`]).
pub struct Place {
    /// The file it had.
    path: Option<PathBuf>,
    /// Its [`Editor::anchor`].
    anchor: Option<PathBuf>,
    /// Whether it had a working folder.
    unpacked: bool,
    /// The document as it was, to see whether the move changed how it names its pictures.
    doc: Document,
}

/// A sentence for a file a newer Quill wrote: its stamp says a higher format number or features this
/// version has no name for, or its bundle a higher layout number or a kind of part this version doesn't know
/// (`layout`). `None` for a file this version knows all of.
// Human: what this version doesn't know is kept: unknown keys in the document, unknown parts of a `.quill` file.
// Only of a `.quill` file in a later layout (`later`) that can't be promised: it is written in this version's.
fn newer_than_this(doc: &Document, layout: Option<u64>, later: bool) -> Option<String> {
    let stamp = pen_doc::stamp::newer(doc);
    if stamp.is_none() && layout.is_none() {
        return None;
    }
    let unknown = stamp.map(|stamp| stamp.features).unwrap_or_default();
    let what = if unknown.is_empty() { String::new() } else { format!(" ({})", unknown.join(", ")) };
    let saved = if later {
        ". Saving it here writes this version's layout, so parts this version doesn't know may be left out."
    } else {
        "; what it doesn't know is kept when you save."
    };
    Some(format!("A newer Quill wrote this file. This version may not draw all of it{what}{saved}"))
}

/// Shows `visit` the `url` of every image paint in `doc`: in fills and strokes, and in what instances override.
// Human: fills and strokes are read as paints; an instance's overrides are kept as plain JSON, so every
// object in them that is an image paint counts.
fn image_urls(doc: &mut Document, visit: &mut dyn FnMut(&mut String)) {
    for top in &mut doc.children {
        node_image_urls(top, visit);
    }
}

/// [`image_urls`] for one node and everything in it.
fn node_image_urls(node: &mut Node, visit: &mut dyn FnMut(&mut String)) {
    use pen_doc::model::{Paint, Paints};
    fn in_paint(object: &mut Map<String, Value>, visit: &mut dyn FnMut(&mut String)) {
        if object.get("type").and_then(Value::as_str) == Some("image")
            && let Some(Value::String(url)) = object.get_mut("url")
        {
            visit(url);
        }
    }
    fn in_paints(paints: &mut Paints, visit: &mut dyn FnMut(&mut String)) {
        let list = match paints {
            Paints::One(one) => std::slice::from_mut(one),
            Paints::Many(many) => many.as_mut_slice(),
        };
        for one in list {
            if let Paint::Object(object) = one {
                in_paint(object, visit);
            }
        }
    }
    fn in_value(value: &mut Value, visit: &mut dyn FnMut(&mut String)) {
        match value {
            Value::Object(object) => {
                in_paint(object, visit);
                object.values_mut().for_each(|inner| in_value(inner, visit));
            }
            Value::Array(list) => list.iter_mut().for_each(|inner| in_value(inner, visit)),
            _ => {}
        }
    }
    fn in_node(node: &mut Node, visit: &mut dyn FnMut(&mut String)) {
        for own in [&mut node.fill, &mut node.stroke].into_iter().flatten() {
            in_paints(own, visit);
        }
        for overridden in node.descendants.iter_mut().flat_map(Map::values_mut) {
            in_value(overridden, visit);
        }
        for child in node.children.iter_mut().flatten() {
            in_node(child, visit);
        }
    }
    in_node(node, visit);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A design to build in part: a board without a layout that holds a rectangle, a text, a frame in a frame,
    /// a row with a layout that is as wide as its children, and a group; and a board that is a layout itself.
    fn parts_editor() -> Editor {
        let json = r##"{"version":"2.22","children":[
            {"type":"frame","id":"plain","x":0,"y":0,"width":800,"height":600,"layout":"none","children":[
                {"type":"rectangle","id":"a","x":10,"y":10,"width":40,"height":20,"fill":"#ff0000"},
                {"type":"text","id":"t","x":10,"y":50,"content":"Hello","fontSize":16},
                {"type":"frame","id":"outer","x":100,"y":100,"width":300,"height":200,"layout":"none","children":[
                    {"type":"frame","id":"inner","x":10,"y":10,"width":100,"height":100,"layout":"none","children":[
                        {"type":"rectangle","id":"deep","x":5,"y":5,"width":20,"height":20,"fill":"#00ff00"}]}]},
                {"type":"frame","id":"row","x":10,"y":400,"layout":"horizontal","gap":8,"padding":4,"children":[
                    {"type":"rectangle","id":"r1","width":30,"height":30,"fill":"#0000ff"},
                    {"type":"rectangle","id":"r2","width":"fill_container","height":30,"fill":"#0000ff"},
                    {"type":"rectangle","id":"float","layoutPosition":"absolute","x":2,"y":2,"width":5,"height":5}]},
                {"type":"group","id":"grp","x":500,"y":100,"children":[
                    {"type":"rectangle","id":"g1","x":0,"y":0,"width":30,"height":30,"fill":"#333333"},
                    {"type":"rectangle","id":"g2","x":50,"y":50,"width":30,"height":30,"fill":"#333333"}]}]},
            {"type":"frame","id":"flow","x":900,"y":0,"layout":"vertical","gap":10,"children":[
                {"type":"rectangle","id":"f1","width":100,"height":40,"fill":"#ff00ff"},
                {"type":"rectangle","id":"f2","width":100,"height":40,"fill":"#ff00ff"}]}
        ]}"##;
        let mut editor = Editor::new(pen_doc::read(json).unwrap(), None, 1);
        editor.refresh(&measure);
        editor
    }

    /// Changes the layers `ids` with `change`, saying that only they change, and brings the scene up to date.
    /// Gives whether the scene was built in part: the first top-level node is then marked as no longer what
    /// was last compared. (That the scene is what building it whole gives is checked by the refresh itself.)
    fn tracked(editor: &mut Editor, ids: &[&str], change: &dyn Fn(&mut Node)) -> bool {
        // A comparison first, so the mark is this change's.
        editor.touched();
        editor.refresh(&measure);
        let ids: Vec<String> = ids.iter().map(|id| (*id).to_owned()).collect();
        editor.touched_only(&ids);
        for id in &ids {
            assert!(editor.change(id, change));
        }
        let stamps = editor.stamps().to_vec();
        editor.refresh(&measure);
        assert_eq!(editor.scene, editor.laid_out(&measure));
        let built = editor.laid.loose.contains(&true);
        // What was built in part has another stamp, so the canvas draws it anew; nothing else has.
        assert_eq!(built, editor.laid.loose.iter().zip(stamps.iter().zip(editor.stamps())).all(|(loose, (was, is))| *loose == (was != is)));
        built
    }

    fn at_x(x: u32) -> impl Fn(&mut Node) {
        move |node| node.x = Some(x.into())
    }

    #[test]
    fn a_layer_whose_box_is_its_own_affair_is_built_again_by_itself() {
        let mut editor = parts_editor();
        // A layer of a frame without a layout, a layer deep in such frames, and a text.
        assert!(tracked(&mut editor, &["a"], &at_x(77)));
        assert!((editor.scene_node("a").unwrap().local.x - 77.0).abs() < 1e-6);
        assert!(tracked(&mut editor, &["deep"], &at_x(33)));
        assert!((layout::doc_bounds(editor.scene_node("deep").unwrap()).x - 143.0).abs() < 1e-6);
        assert!(tracked(&mut editor, &["t"], &|node| node.content = Some("Hello there".to_owned())));
        // Several at once, one of them inside another: the outer one brings the inner one along.
        assert!(tracked(&mut editor, &["deep", "inner", "a"], &at_x(12)));
        // A layer that keeps out of the layout it lies in.
        assert!(tracked(&mut editor, &["float"], &at_x(9)));
        // The other board was never built again.
        assert_eq!(editor.laid.loose, [true, false]);
    }

    #[test]
    fn a_layout_and_a_group_follow_a_layer_that_changes_in_them() {
        let mut editor = parts_editor();
        let width = |editor: &Editor, id: &str| editor.scene_node(id).unwrap().local.w;
        // The row is as wide as its children: it is built again, and it alone, since the board has no layout.
        let before = width(&editor, "row");
        assert!(tracked(&mut editor, &["r1"], &|node| node.width = Some(pen_doc::value::Num::new(90.0))));
        assert!((width(&editor, "row") - before - 60.0).abs() < 1e-4);
        // A group is as big as what is in it.
        assert!(tracked(&mut editor, &["g2"], &at_x(200)));
        assert!((width(&editor, "grp") - 230.0).abs() < 1e-4);
        // A board that is a layout itself is a top-level node: those are compared, as before.
        assert!(!tracked(&mut editor, &["f1"], &|node| node.height = Some(pen_doc::value::Num::new(80.0))));
        assert!((editor.scene_node("f2").unwrap().local.y - 90.0).abs() < 1e-4);
        // A layer that goes into a layout, or comes out of it, moves the others: the layout is built again.
        assert!(tracked(&mut editor, &["float"], &|node| node.layout_position = None));
        assert!(tracked(&mut editor, &["float"], &|node| node.layout_position = Some("absolute".to_owned())));
    }

    #[test]
    fn layers_that_come_and_go_are_a_change_to_the_layer_they_lie_in() {
        let mut editor = parts_editor();
        editor.touched_only(&["inner".to_owned()]);
        let mut new = edit::find(&editor.doc, "deep").unwrap().clone();
        new.id = "new".to_owned();
        assert!(edit::insert(&mut editor.doc, Some("inner"), 0, new));
        editor.refresh(&measure);
        assert!(editor.laid.loose[0] && editor.scene_node("new").is_some() && editor.scene_parent("deep").unwrap().id == "inner");
        editor.touched_only(&["inner".to_owned()]);
        assert!(edit::remove(&mut editor.doc, "deep").is_some());
        editor.refresh(&measure);
        assert!(editor.scene_node("deep").is_none() && editor.scene_node("new").is_some());
        // A layer the scene doesn't know yet is found by comparing.
        editor.touched_only(&["nowhere".to_owned()]);
        editor.refresh(&measure);
        assert_eq!(editor.scene, editor.laid_out(&measure));
    }

    #[test]
    fn an_edit_with_its_own_undo_step_builds_only_the_layers_it_names() {
        let mut editor = parts_editor();
        let stamps = editor.stamps().to_vec();
        // A delete and a duplicate deep in a board: the layer they lay in is built again, alone.
        editor.selection = vec!["deep".to_owned()];
        editor.duplicate();
        editor.refresh(&measure);
        assert!(!editor.rebuilt().is_empty() && editor.stamps()[1] == stamps[1] && edit::find(&editor.doc, "inner").unwrap().kids().len() == 2);
        editor.selection = vec!["deep".to_owned()];
        editor.delete_selection();
        editor.refresh(&measure);
        assert!(!editor.rebuilt().is_empty() && editor.scene_node("deep").is_none());
        assert_eq!(editor.scene, editor.laid_out(&measure));
        // Directly in a board, the board is what changed: it is found by comparing, and built whole.
        editor.selection = vec!["a".to_owned()];
        editor.delete_selection();
        editor.refresh(&measure);
        assert!(editor.rebuilt().is_empty() && editor.scene_node("a").is_none() && editor.stamps()[1] == stamps[1]);
        // Something else changed the document after the step was made: nothing said afterwards counts.
        editor.checkpoint();
        assert!(editor.change("f1", &at_x(7)));
        editor.touched();
        assert!(editor.change("g1", &at_x(3)));
        editor.changed_only(&[Some("g1".to_owned())]);
        editor.refresh(&measure);
        assert_eq!(editor.scene, editor.laid_out(&measure));
        // The top level among what changed: compared, too.
        editor.checkpoint();
        assert!(editor.change("f2", &at_x(9)));
        editor.changed_only(&[None, Some("g1".to_owned())]);
        editor.refresh(&measure);
        assert_eq!(editor.scene, editor.laid_out(&measure));
        // Every step goes back, and forward again.
        for _ in 0..5 {
            editor.undo();
            editor.refresh(&measure);
            assert_eq!(editor.scene, editor.laid_out(&measure));
        }
        assert!(editor.scene_node("deep").is_some() && editor.scene_node("a").is_some());
        for _ in 0..5 {
            editor.redo();
            editor.refresh(&measure);
        }
        assert!(editor.scene_node("deep").is_none() && editor.scene_node("a").is_none());
    }

    #[test]
    fn top_level_nodes_that_come_go_and_change_places_leave_the_others_scenes_as_they_are() {
        let mut editor = parts_editor();
        let (plain, flow) = (editor.stamps()[0], editor.stamps()[1]);
        // A new board at the end, then one in front: the two that were there keep what was built of them.
        let mut copy = edit::find(&editor.doc, "flow").unwrap().clone();
        editor.checkpoint();
        editor.renew_ids(&mut copy);
        let new = copy.id.clone();
        assert!(edit::insert(&mut editor.doc, None, usize::MAX, copy.clone()));
        editor.refresh(&measure);
        assert_eq!((editor.stamps().len(), editor.stamps()[0], editor.stamps()[1]), (3, plain, flow));
        assert!(editor.scene_node(&new).is_some());
        editor.checkpoint();
        editor.renew_ids(&mut copy);
        assert!(edit::insert(&mut editor.doc, None, 0, copy));
        editor.refresh(&measure);
        assert_eq!((editor.stamps().len(), editor.stamps()[1], editor.stamps()[2]), (4, plain, flow));
        assert_places_hold(&editor, "after a board was put in front");
        // The first of the old boards goes; a step back brings it, a step forward takes it again.
        editor.selection = vec!["plain".to_owned()];
        editor.delete_selection();
        editor.refresh(&measure);
        assert_eq!((editor.stamps().len(), editor.stamps()[1]), (3, flow));
        assert!(editor.scene_node("deep").is_none());
        editor.undo();
        editor.refresh(&measure);
        assert_eq!((editor.stamps().len(), editor.stamps()[2]), (4, flow));
        assert!(editor.scene_node("deep").is_some());
        assert_places_hold(&editor, "after a board came back");
        // Two boards change places: both keep what was built of them.
        editor.checkpoint();
        editor.doc.children.swap(1, 2);
        editor.refresh(&measure);
        assert_eq!(editor.stamps()[1], flow);
        assert_eq!(editor.scene, editor.laid_out(&measure));
        assert_places_hold(&editor, "after two boards changed places");
    }

    #[test]
    fn a_copy_of_the_document_is_taken_as_it_is_while_the_document_is_in_its_state() {
        let mut editor = parts_editor();
        editor.checkpoint();
        assert!(editor.change("a", &at_x(30)));
        editor.refresh(&measure);
        let held = editor.held.clone().unwrap();
        // Nothing changed since the scene was built: the next step's copy is that copy.
        editor.checkpoint();
        let (step, _) = editor.undo.back().unwrap();
        assert!(Arc::ptr_eq(&step.rest, &held.rest) && step.roots.iter().zip(&held.roots).all(|(is, was)| Arc::ptr_eq(is, was)));
        // A change gives the document another state, and the next copy holds it.
        assert!(editor.change("a", &at_x(40)));
        editor.checkpoint();
        let (step, _) = editor.undo.back().unwrap();
        assert!(!Arc::ptr_eq(&step.roots[0], &held.roots[0]) && Arc::ptr_eq(&step.roots[1], &held.roots[1]));
        assert_eq!(stored_x(&step.roots[0], "a"), Some(40.0));
    }

    /// The `x` the layer `id` has in `root`.
    fn stored_x(root: &Node, id: &str) -> Option<f32> {
        if root.id == id {
            return root.x.as_ref().and_then(serde_json::Number::as_f64).map(pen_doc::value::narrow);
        }
        root.kids().iter().find_map(|kid| stored_x(kid, id))
    }

    #[test]
    fn a_comparison_after_parts_were_built_sees_what_they_left() {
        let mut editor = parts_editor();
        let first = editor.doc.clone();
        assert!(tracked(&mut editor, &["a"], &at_x(77)));
        // Back to the document that was last compared, without saying where: it is as it was then, and the
        // scene, which is not, has to follow all the same.
        editor.touched();
        editor.doc = first;
        editor.refresh(&measure);
        assert_eq!(editor.scene, editor.laid_out(&measure));
        assert!((editor.scene_node("a").unwrap().local.x - 10.0).abs() < 1e-6 && editor.laid.loose == [false, false]);
        // Undo and redo, which say nothing either.
        editor.checkpoint();
        assert!(tracked(&mut editor, &["deep"], &at_x(40)));
        editor.undo();
        editor.refresh(&measure);
        assert_eq!(editor.scene, editor.laid_out(&measure));
        editor.redo();
        editor.refresh(&measure);
        assert_eq!(editor.scene, editor.laid_out(&measure));
        assert!((editor.scene_node("deep").unwrap().local.x - 40.0).abs() < 1e-6);
    }

    #[test]
    fn instances_are_built_in_part_and_components_bring_their_instances_along() {
        let mut editor = editor();
        // An instance is a layer like another; the connector on the next board is still routed.
        editor.touched_layer("one");
        assert!(editor.change("one", &at_x(60)));
        editor.refresh(&measure);
        assert!(editor.laid.loose[1] && (editor.scene_node("one").unwrap().local.x - 60.0).abs() < 1e-6);
        // The layer a connector starts at: the connector follows it into the other board.
        editor.touched_layer("from");
        assert!(editor.change("from", &at_x(150)));
        let wire = editor.scene_node("wire").unwrap().clone();
        editor.refresh(&measure);
        assert_ne!(*editor.scene_node("wire").unwrap(), wire);
        // A component is shown elsewhere: everything is built, and the instance has its new width.
        editor.touched_layer("chip");
        assert!(editor.change("chip", &|node| node.width = Some(pen_doc::value::Num::new(64.0))));
        editor.refresh(&measure);
        assert!((editor.scene_node("one").unwrap().local.w - 64.0).abs() < 1e-6);
        assert_eq!(editor.laid.loose, [false; 4]);
        assert_eq!(editor.scene, editor.laid_out(&measure));
    }

    #[test]
    #[should_panic(expected = "without saying so")]
    fn a_change_that_is_not_where_it_was_said_to_be_is_caught() {
        let mut editor = parts_editor();
        editor.touched_only(&["a".to_owned()]);
        assert!(editor.change("f1", &at_x(5)));
        editor.refresh(&measure);
    }

    #[test]
    #[should_panic(expected = "not the scene built whole")]
    fn a_change_beside_the_layer_it_was_said_to_be_in_is_caught() {
        let mut editor = parts_editor();
        editor.touched_only(&["a".to_owned()]);
        assert!(editor.change("t", &at_x(5)));
        editor.refresh(&measure);
    }

    #[test]
    fn a_step_back_holds_only_the_top_level_nodes_that_changed() {
        let mut editor = editor();
        let first = editor.doc.clone();
        // Three edits of one layer on one board: each step holds that board anew, the other three once.
        for x in [11, 12, 13] {
            editor.checkpoint();
            assert!(editor.change("word", &|node| node.x = Some(x.into())));
            editor.refresh(&measure);
        }
        assert_eq!(editor.history_holds().0, 4 + 2);
        let shared = |a: &Kept, b: &Kept| a.roots.iter().zip(&b.roots).filter(|(a, b)| Arc::ptr_eq(a, b)).count();
        assert_eq!(shared(&editor.undo[0].0, &editor.undo[1].0), 3);
        assert!(Arc::ptr_eq(&editor.undo[0].0.rest, &editor.undo[2].0.rest));
        // What the scene was laid out from is those same nodes, not copies of its own.
        let now = editor.kept();
        assert_eq!(shared(&now, &editor.undo[2].0), 3);
        assert!(editor.laid.nodes.iter().zip(&now.roots).all(|(laid, now)| Arc::ptr_eq(laid, now)));
        // Each step back and forward is the document as it was then.
        let last = editor.doc.clone();
        editor.undo();
        assert_eq!(edit::find(&editor.doc, "word").and_then(|node| node.x.clone()), Some(12.into()));
        editor.undo();
        editor.undo();
        assert_eq!(editor.doc, first);
        editor.undo();
        assert_eq!(editor.doc, first);
        for _ in 0..3 {
            editor.redo();
        }
        assert_eq!(editor.doc, last);
        editor.redo();
        assert_eq!(editor.doc, last);
        // A copy as a document of its own is the document.
        assert_eq!(editor.kept().document(), last);
        assert_eq!(Kept::sharing(&last, None).document(), last);
    }

    #[test]
    fn steps_back_follow_top_level_nodes_that_come_go_and_change_places() {
        let mut editor = editor();
        let mut states = vec![editor.doc.clone()];
        let mut step = |editor: &mut Editor, change: &dyn Fn(&mut Document)| {
            editor.checkpoint();
            change(&mut editor.doc);
            editor.refresh(&measure);
            states.push(editor.doc.clone());
        };
        // A board goes, two change places, one comes in at the front, the variables change, a layer changes.
        step(&mut editor, &|doc| drop(doc.children.remove(3)));
        step(&mut editor, &|doc| doc.children.swap(0, 2));
        step(&mut editor, &|doc| {
            let mut new = doc.children[1].clone();
            new.id = "new".to_owned();
            new.children = None;
            doc.children.insert(0, new);
        });
        step(&mut editor, &|doc| doc.variables = Some(serde_json::Map::new()));
        step(&mut editor, &|doc| doc.children[2].x = Some(7.into()));
        // A board that only changed its place is held once: by the step before and the step after.
        let (before, after) = (&editor.undo[1].0, &editor.undo[2].0);
        assert!(Arc::ptr_eq(&before.roots[0], &after.roots[2]) && Arc::ptr_eq(&before.roots[2], &after.roots[0]));
        for state in states.iter().rev().skip(1) {
            editor.undo();
            assert_eq!(&editor.doc, state);
            editor.refresh(&measure);
            assert_eq!(editor.scene.len(), state.children.len());
        }
        for state in states.iter().skip(1) {
            editor.redo();
            assert_eq!(&editor.doc, state);
        }
    }

    #[test]
    fn no_more_steps_back_are_kept_than_the_limit() {
        let mut editor = editor();
        for x in 0..u32::try_from(UNDO_LIMIT).unwrap() + 5 {
            editor.checkpoint();
            editor.change("word", &|node| node.x = Some(x.into()));
        }
        assert_eq!(editor.undo.len(), UNDO_LIMIT);
        // The oldest steps are the ones that went: the first one left is the document after five edits.
        for _ in 0..UNDO_LIMIT {
            editor.undo();
        }
        assert_eq!(edit::find(&editor.doc, "word").and_then(|node| node.x.clone()), Some(4.into()));
    }

    #[test]
    fn a_picture_that_moved_is_written_into_every_step_and_the_steps_go_on_sharing() {
        let json = r##"{"version":"2.22","children":[
            {"type":"rectangle","id":"a","fill":{"type":"image","url":"./old.png"}},
            {"type":"rectangle","id":"b","x":0,"fill":"#ff0000"}]}"##;
        let mut editor = Editor::new(pen_doc::read(json).unwrap(), None, 1);
        for x in [1, 2, 3] {
            editor.checkpoint();
            editor.change("b", &|node| node.x = Some(x.into()));
        }
        editor.undo();
        editor.repicture(&mut |url| (url == "./old.png").then(|| "./new.png".to_owned()));
        // The picture's layer is still held once, by every step and by the document's own copy.
        assert_eq!(editor.history_holds().0, 1 + 3);
        let held = editor.held.clone().unwrap();
        assert!(editor.undo.iter().chain(&editor.redo).all(|(past, _)| Arc::ptr_eq(&past.roots[0], &held.roots[0])));
        let url = |doc: &Document| serde_json::to_value(&doc.children[0]).unwrap()["fill"]["url"].clone();
        assert_eq!(url(&editor.doc), "./new.png");
        for _ in 0..2 {
            editor.undo();
            assert_eq!(url(&editor.doc), "./new.png");
        }
        for _ in 0..3 {
            editor.redo();
            assert_eq!(url(&editor.doc), "./new.png");
        }
    }

    /// A design of three boards: one holds a component, one an instance of it, and one a plain rectangle with
    /// a connector to a rectangle on the board before it.
    fn editor() -> Editor {
        let json = r##"{"version":"2.22","children":[
            {"type":"frame","id":"parts","x":0,"y":0,"width":200,"height":100,"layout":"none","children":[
                {"type":"rectangle","id":"chip","reusable":true,"x":10,"y":10,"width":40,"height":20,"fill":"#ff0000"}]},
            {"type":"frame","id":"uses","x":300,"y":0,"width":200,"height":100,"layout":"none","children":[
                {"type":"ref","id":"one","ref":"chip","x":20,"y":20},
                {"type":"rectangle","id":"from","x":100,"y":10,"width":30,"height":30,"fill":"#00ff00"}]},
            {"type":"frame","id":"plain","x":600,"y":0,"width":200,"height":100,"layout":"none","children":[
                {"type":"rectangle","id":"to","x":20,"y":50,"width":30,"height":30,"fill":"#0000ff"},
                {"type":"connector","id":"wire","from":"from","to":"to","stroke":"#000000"}]},
            {"type":"frame","id":"alone","x":900,"y":0,"width":200,"height":100,"layout":"none","children":[
                {"type":"text","id":"word","x":10,"y":10,"content":"Hello","fontSize":16}]}
        ]}"##;
        let mut editor = Editor::new(pen_doc::read(json).unwrap(), None, 1);
        editor.refresh(&measure);
        editor
    }

    #[test]
    fn a_restored_document_counts_as_unsaved_and_one_step_back_is_the_files() {
        let mut editor = editor();
        let (before, mut kept) = (pen_doc::write(&editor.doc).unwrap(), editor.doc.clone());
        kept.children.pop();
        editor.selection = vec!["from".to_owned()];
        assert!(!editor.dirty());
        editor.restore(kept.clone());
        editor.refresh(&|_| (0.0, 0.0));
        assert!(editor.dirty() && editor.scene.len() == kept.children.len());
        editor.undo();
        assert_eq!(pen_doc::write(&editor.doc).unwrap(), before);
        // That is the file's document again: nothing is unsaved.
        assert!(!editor.dirty());
    }

    fn measure(style: &pen_doc::layout::TextStyle) -> (f32, f32) {
        #[expect(clippy::cast_precision_loss, reason = "a short test text")]
        let long = style.content.chars().count() as f32;
        (long * style.size * 0.5, style.size * 1.2)
    }

    /// Changes the layer `id`, lays the document out again, and says which top-level nodes got another
    /// stamp. The scene must be the one a build of the whole document gives.
    fn edit(editor: &mut Editor, id: &str, change: &dyn Fn(&mut Node)) -> Vec<bool> {
        let before = editor.stamps().to_vec();
        assert!(editor.change(id, change));
        editor.touched();
        editor.refresh(&measure);
        assert_eq!(format!("{:?}", editor.scene), format!("{:?}", editor.laid_out(&measure)), "after changing {id}");
        before.iter().zip(editor.stamps()).map(|(was, is)| was != is).collect()
    }

    #[test]
    fn an_edit_lays_out_only_the_top_level_node_it_is_in() {
        let mut editor = editor();
        // A text on a board of its own: that board alone. (The board with the connector counts as changed
        // with every edit, since what a connector follows may lie anywhere.)
        let rebuilt = edit(&mut editor, "word", &|node| node.content = Some("Hello there".to_owned()));
        assert_eq!(rebuilt, [false, false, true, true]);
        let wider = editor.scene_node("word").unwrap().local.w;
        assert!((wider - 11.0 * 8.0).abs() < 1e-3, "{wider}");
        // The layer a connector starts at moves: its own board, and the connector's board follows.
        let rebuilt = edit(&mut editor, "from", &|node| node.x = Some(140.into()));
        assert_eq!(rebuilt, [false, true, true, false]);
        // Nothing changed at all: nothing is built again.
        editor.touched();
        let before = editor.stamps().to_vec();
        editor.refresh(&measure);
        assert_eq!(editor.stamps()[..2], before[..2]);
    }

    /// One edit as the canvas, a panel or a model makes it: an undo step, then the change.
    fn widen(editor: &mut Editor, width: f32) {
        editor.checkpoint();
        assert!(editor.change("chip", &|node| node.width = Some(pen_doc::value::Num::new(width))));
    }

    #[test]
    fn a_document_is_unsaved_exactly_while_it_is_not_in_the_state_its_file_holds() {
        let mut editor = editor();
        assert!(!editor.dirty());
        // Two edits, then back: at the file's state nothing is unsaved; a step forward and there is again.
        widen(&mut editor, 50.0);
        widen(&mut editor, 60.0);
        assert!(editor.dirty());
        editor.undo();
        assert!(editor.dirty());
        editor.undo();
        assert!(!editor.dirty());
        editor.redo();
        assert!(editor.dirty());
        // A drag goes on without a new undo step: the document changed all the same.
        editor.undo();
        editor.touched();
        assert!(editor.dirty());
        // Saved in the middle of its history (as Save, Save As and a model's save do, all through `save`):
        // that state is the file's, and steps away from it in either direction are unsaved.
        let folder = std::env::temp_dir().join(format!("quill-saved-point-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let mut editor = self::editor();
        widen(&mut editor, 50.0);
        widen(&mut editor, 60.0);
        editor.undo();
        editor.move_to(folder.join("first.quill")).unwrap();
        editor.save().unwrap();
        assert!(!editor.dirty());
        editor.undo();
        assert!(editor.dirty());
        editor.redo();
        assert!(!editor.dirty());
        editor.redo();
        assert!(editor.dirty());
        editor.undo();
        assert!(!editor.dirty());
        // Saved under another name from a step back: that step is the saved one now.
        editor.undo();
        editor.move_to(folder.join("second.quill")).unwrap();
        editor.save().unwrap();
        assert!(!editor.dirty());
        editor.redo();
        assert!(editor.dirty());
        // Saved at the last step. A new edit after an undo throws away the steps ahead, the saved one with
        // them: no undo or redo comes back to it.
        editor.redo();
        editor.save().unwrap();
        editor.undo();
        assert!(editor.dirty());
        widen(&mut editor, 70.0);
        editor.redo();
        assert!(editor.dirty());
        for _ in 0..3 {
            editor.undo();
            assert!(editor.dirty());
        }
        for _ in 0..3 {
            editor.redo();
            assert!(editor.dirty());
        }
        // Until it is saved again.
        editor.save().unwrap();
        assert!(!editor.dirty());
        // A document counted as unsaved because no file holds it stays so through undo and redo.
        widen(&mut editor, 80.0);
        editor.mark_unsaved();
        editor.undo();
        assert!(editor.dirty());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    /// Every node of the scene is found by its id where a search of the whole scene finds it, with the node
    /// that holds it, and nothing else is found.
    fn assert_places_hold(editor: &Editor, when: &str) {
        fn parent_of<'a>(nodes: &'a [SceneNode], id: &str) -> Option<&'a SceneNode> {
            nodes.iter().find_map(|n| if n.children.iter().any(|c| c.id == id) { Some(n) } else { parent_of(&n.children, id) })
        }
        let mut ids = HashSet::new();
        for top in &editor.scene {
            top.walk(
                &mut |node, _| {
                    ids.insert(node.id.clone());
                },
                0,
            );
        }
        for id in &ids {
            let (quick, searched) = (editor.scene_node(id), find_in(&editor.scene, id));
            assert!(matches!((quick, searched), (Some(a), Some(b)) if std::ptr::eq(a, b)), "{when}: {id}");
            let (quick, searched) = (editor.scene_parent(id), parent_of(&editor.scene, id));
            assert_eq!(quick.map(std::ptr::from_ref), searched.map(std::ptr::from_ref), "{when}: the parent of {id}");
        }
        assert_eq!(editor.places.at.len(), ids.len(), "{when}: ids that are in no scene are kept");
        assert!(editor.scene_node("nowhere").is_none() && editor.scene_parent("nowhere").is_none(), "{when}");
    }

    #[test]
    fn scene_nodes_are_found_by_id_after_every_kind_of_change() {
        let mut editor = editor();
        assert_places_hold(&editor, "at first");
        assert_eq!(editor.scene_parent("chip").map(|parent| parent.id.as_str()), Some("parts"));
        assert!(editor.scene_parent("parts").is_none());
        // An edit that builds one board again, and one that builds everything again.
        edit(&mut editor, "word", &|node| node.content = Some("Hello there".to_owned()));
        assert_places_hold(&editor, "after an edit on one board");
        edit(&mut editor, "chip", &|node| node.width = Some(pen_doc::value::Num::new(60.0)));
        assert_places_hold(&editor, "after a component changed");
        // A layer goes: its id is found no more. Undo brings it back, redo takes it away again.
        editor.checkpoint();
        assert!(edit::remove(&mut editor.doc, "to").is_some());
        editor.refresh(&measure);
        assert!(editor.scene_node("to").is_none());
        assert_places_hold(&editor, "after a layer was removed");
        editor.undo();
        editor.refresh(&measure);
        assert!(editor.scene_node("to").is_some());
        assert_places_hold(&editor, "after undo");
        editor.redo();
        editor.refresh(&measure);
        assert!(editor.scene_node("to").is_none());
        assert_places_hold(&editor, "after redo");
        // A layer moves to another board, as a model's edit or a drag does.
        editor.checkpoint();
        let (moved, _) = edit::remove(&mut editor.doc, "from").unwrap();
        assert!(edit::insert(&mut editor.doc, Some("alone"), 0, moved));
        editor.refresh(&measure);
        assert_eq!(editor.scene_parent("from").map(|parent| parent.id.as_str()), Some("alone"));
        assert_places_hold(&editor, "after a layer changed boards");
        // The boards change places, and one more comes.
        editor.checkpoint();
        editor.doc.children.swap(0, 3);
        let mut extra = editor.doc.children[1].clone();
        extra.id = "extra".to_owned();
        extra.children = None;
        editor.doc.children.insert(1, extra);
        editor.refresh(&measure);
        assert_places_hold(&editor, "after the boards changed places");
        // Another document takes this one's place (a kept copy restored, a model's whole new document).
        let other = pen_doc::read(r#"{"version":"2.22","children":[{"type":"rectangle","id":"only","width":10,"height":10}]}"#).unwrap();
        editor.restore(other);
        editor.refresh(&measure);
        assert!(editor.scene_node("only").is_some() && editor.scene_node("parts").is_none());
        assert_places_hold(&editor, "after another document");
    }

    #[test]
    fn an_id_that_is_there_twice_is_found_where_it_comes_first() {
        let json = r#"{"version":"2.22","children":[
            {"type":"frame","id":"a","width":100,"height":100,"layout":"none","children":[{"type":"rectangle","id":"twin","width":10,"height":10}]},
            {"type":"frame","id":"b","x":200,"width":100,"height":100,"layout":"none","children":[{"type":"rectangle","id":"twin","width":20,"height":20}]}
        ]}"#;
        let mut editor = Editor::new(pen_doc::read(json).unwrap(), None, 1);
        editor.refresh(&measure);
        let first = |editor: &Editor| std::ptr::eq(editor.scene_node("twin").unwrap(), find_in(&editor.scene, "twin").unwrap());
        assert!(first(&editor));
        // The board the first one is on is built again, then the other: still the first in drawing order.
        for (step, board) in [0, 1, 0].into_iter().enumerate() {
            editor.doc.children[board].y = Some(i32::try_from(step + 1).unwrap().into());
            editor.touched();
            editor.refresh(&measure);
            assert!(first(&editor), "after board {board} was built again");
        }
        // The first one goes: the other is found.
        editor.doc.children[0].children = Some(Vec::new());
        editor.touched();
        editor.refresh(&measure);
        assert!(first(&editor) && (editor.scene_node("twin").unwrap().local.w - 20.0).abs() < 1e-3);
    }

    #[test]
    fn a_selection_holds_a_locked_layer_when_one_of_them_or_what_it_is_in_is_locked() {
        let mut editor = editor();
        let all: Vec<String> = ["parts", "chip", "uses", "one", "from", "plain", "to", "word", "nowhere"].map(str::to_owned).to_vec();
        let agree = |editor: &Editor| {
            for from in 0..all.len() {
                for to in from..=all.len() {
                    let ids = &all[from..to];
                    assert_eq!(editor.any_locked(ids), ids.iter().any(|id| editor.locked(id)), "{ids:?}");
                }
            }
        };
        agree(&editor);
        assert!(!editor.any_locked(&all));
        // A board is locked: so is everything on it, and nothing else.
        assert!(editor.change("uses", &|node| crate::ops::lock(node, true)));
        agree(&editor);
        assert!(editor.any_locked(&["word".to_owned(), "from".to_owned()]) && !editor.any_locked(&["word".to_owned(), "to".to_owned()]));
        // One layer alone is locked.
        assert!(editor.change("uses", &|node| crate::ops::lock(node, false)) && editor.change("to", &|node| crate::ops::lock(node, true)));
        editor.touched();
        editor.refresh(&measure);
        agree(&editor);
    }

    #[test]
    fn a_change_to_a_component_or_to_what_all_nodes_share_builds_everything_again() {
        let mut editor = editor();
        // The component changes: the instance on another board has to follow.
        let rebuilt = edit(&mut editor, "chip", &|node| node.width = Some(pen_doc::value::Num::new(60.0)));
        assert_eq!(rebuilt, [true; 4]);
        assert!((editor.scene_node("one").unwrap().local.w - 60.0).abs() < 1e-3);
        // The fonts became ready, or the theme changed: everything.
        let before = editor.stamps().to_vec();
        editor.relayout();
        editor.refresh(&measure);
        assert!(before.iter().zip(editor.stamps()).all(|(was, is)| was != is));
        // An undo is an edit like any other: back to the scene of the whole document.
        editor.checkpoint();
        edit(&mut editor, "word", &|node| node.content = Some("Bye".to_owned()));
        editor.undo();
        editor.refresh(&measure);
        assert_eq!(format!("{:?}", editor.scene), format!("{:?}", editor.laid_out(&measure)));
        assert_eq!(editor.scene_node("word").unwrap().node.content.as_deref(), Some("Hello"));
    }
}
