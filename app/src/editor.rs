//! One open document: its file, the document itself, the laid-out scene, selection, camera and undo history.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

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
        visit(self, root, &mut vec![u32::try_from(at).unwrap_or(u32::MAX)]);
    }

    /// Forgets `root`, which was the top-level node at `at`, and everything in it.
    fn remove(&mut self, root: &SceneNode, at: usize) {
        let at = u32::try_from(at).unwrap_or(u32::MAX);
        root.walk(
            &mut |node, _| {
                if self.at.get(&node.id).is_some_and(|path| path.first() == Some(&at)) {
                    self.at.remove(&node.id);
                }
            },
            0,
        );
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
    undo: Vec<(Document, u64)>,
    /// The documents to step forward to again, each with its state's number.
    redo: Vec<(Document, u64)>,
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
}

/// What a scene was built from, and what is known of each of its top-level nodes.
// Human: a design is dozens of screens side by side, and an edit changes one of them. Building and laying out
// only that one, and drawing only that one anew, is what keeps a large design quick to work on.
#[derive(Default)]
struct Laid {
    /// The document without its top-level nodes: its variables, themes and imports.
    rest: Option<Document>,
    /// The top-level nodes, as they were.
    nodes: Vec<Node>,
    /// The theme that was in force at the top level.
    theme: pen_doc::vars::ThemeContext,
    /// A number for each top-level scene node, another one each time it is built again.
    stamps: Vec<u64>,
    /// The ids each top-level node's instances show (`ref`), anywhere in it.
    shows: Vec<Vec<String>>,
    /// Whether each top-level scene node has a connector in it.
    wired: Vec<bool>,
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
            undo: Vec::new(),
            redo: Vec::new(),
            state: 0,
            last_state: 0,
            saved: Some(0),
            ids: IdMaker::new(seed),
            needs_layout: true,
            needs_fit: true,
            laid: Laid::default(),
            places: Places::default(),
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
        self.undo.push((self.doc.clone(), self.state));
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.new_state();
        self.needs_layout = true;
    }

    /// Marks the document changed without a new undo step (continuing a drag).
    pub fn touched(&mut self) {
        self.new_state();
        self.needs_layout = true;
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
        if let Some((previous, state)) = self.undo.pop() {
            self.redo.push((std::mem::replace(&mut self.doc, previous), std::mem::replace(&mut self.state, state)));
            self.after_history();
        }
    }

    /// Steps forward again after an undo.
    pub fn redo(&mut self) {
        if let Some((next, state)) = self.redo.pop() {
            self.undo.push((std::mem::replace(&mut self.doc, next), std::mem::replace(&mut self.state, state)));
            self.after_history();
        }
    }

    fn after_history(&mut self) {
        self.needs_layout = true;
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
        let Some(changed) = self.changed_roots(&theme) else {
            self.scene = self.laid_out(measure);
            self.places = Places::of(&self.scene);
            let laid = &mut self.laid;
            let mut rest = self.doc.clone();
            laid.nodes = std::mem::take(&mut rest.children);
            (laid.rest, laid.theme, laid.all) = (Some(rest), theme, false);
            laid.stamps = self.scene.iter().map(|_| next_number()).collect();
            laid.shows = laid.nodes.iter().map(shown_by).collect();
            laid.wired = self.scene.iter().map(pen_doc::connector::has_connector).collect();
            return;
        };
        let vars = Vars::new(&self.doc);
        let mut built = scene::build_some(&self.doc, &theme, &changed);
        layout::layout_roots(&mut built, vars, measure);
        for (at, root) in changed.iter().copied().zip(built) {
            let (Some(node), Some(place)) = (self.doc.children.get(at), self.scene.get_mut(at)) else { continue };
            self.laid.wired[at] = pen_doc::connector::has_connector(&root);
            self.places.remove(place, at);
            self.places.add(&root, at);
            *place = root;
            self.laid.nodes[at] = node.clone();
            self.laid.shows[at] = shown_by(node);
            self.laid.stamps[at] = next_number();
        }
        // Human: an id that is there twice is found where it comes first in the whole scene, which the map
        // only keeps to when it is built whole.
        if self.places.doubled {
            self.places = Places::of(&self.scene);
        }
        // Human: a connector follows layers that may lie in another top-level node, so every one is routed
        // again, and every top-level node with a connector in it counts as changed.
        if self.laid.wired.contains(&true) {
            pen_doc::connector::route(&mut self.scene, vars);
            for (stamp, _) in self.laid.stamps.iter_mut().zip(&self.laid.wired).filter(|(_, wired)| **wired) {
                *stamp = next_number();
            }
        }
    }

    /// A number for each top-level node of [`Editor::scene`] that is another one each time that node is built
    /// again: what was drawn for a node whose number is the same still holds.
    #[must_use]
    pub fn stamps(&self) -> &[u64] {
        &self.laid.stamps
    }

    /// The top-level nodes that changed since the scene was built, by their place; `None` when the whole
    /// scene has to be built again.
    // Human: everything is built again when what all nodes share changed (variables, themes, the theme
    // chosen, the fonts), when top-level nodes came, went or changed places, and when a changed node holds
    // a component or anything else an instance somewhere shows: that instance has to follow.
    fn changed_roots(&mut self, theme: &pen_doc::vars::ThemeContext) -> Option<Vec<usize>> {
        let laid = &self.laid;
        let count = self.doc.children.len();
        if laid.all
            || laid.theme != *theme
            || [laid.nodes.len(), laid.stamps.len(), laid.shows.len(), laid.wired.len(), self.scene.len()] != [count; 5]
        {
            return None;
        }
        // The document is compared without its children, which are compared one by one below.
        let children = std::mem::take(&mut self.doc.children);
        let same_rest = self.laid.rest.as_ref() == Some(&self.doc);
        self.doc.children = children;
        if !same_rest {
            return None;
        }
        let laid = &self.laid;
        let changed: Vec<usize> = (0..count).filter(|at| laid.nodes.get(*at) != self.doc.children.get(*at)).collect();
        let shown: HashSet<&str> = laid.shows.iter().flatten().map(String::as_str).collect();
        for at in &changed {
            let (Some(was), Some(is)) = (laid.nodes.get(*at), self.doc.children.get(*at)) else { return None };
            if was.id != is.id || holds_shown(was, &shown) || holds_shown(is, &shown) {
                return None;
            }
        }
        Some(changed)
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
        let mut moved: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();
        image_urls(&mut self.doc, &mut |url| {
            if let Some(to) = moved.entry(url.clone()).or_insert_with(|| new(url)) {
                url.clone_from(to);
            }
        });
        if moved.values().all(Option::is_none) {
            return;
        }
        for (past, _) in self.undo.iter_mut().chain(&mut self.redo) {
            image_urls(past, &mut |url| {
                if let Some(Some(to)) = moved.get(url.as_str()) {
                    url.clone_from(to);
                }
            });
        }
        self.needs_layout = true;
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
    for top in &mut doc.children {
        in_node(top, visit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
