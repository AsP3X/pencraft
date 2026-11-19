//! One open document: its file, the document itself, the laid-out scene, selection, camera and undo history.

use std::collections::HashSet;
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
    /// Where it is saved; `None` until first saved. A `.pen` file or a bundle ([`crate::bundle`]).
    pub path: Option<PathBuf>,
    /// The folder a bundle's files lie unpacked in; `None` for a `.pen` file.
    work: Option<crate::bundle::Work>,
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
    /// Whether there are changes not yet saved.
    pub dirty: bool,
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
    undo: Vec<Document>,
    redo: Vec<Document>,
    ids: IdMaker,
    needs_layout: bool,
    needs_fit: bool,
    /// What the scene was last built from, to tell which of its top-level nodes an edit changed.
    laid: Laid,
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
            doc,
            scene: Vec::new(),
            generation: 0,
            selection: Vec::new(),
            camera: Camera::default(),
            tool: Tool::Move,
            dirty: false,
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
            ids: IdMaker::new(seed),
            needs_layout: true,
            needs_fit: true,
            laid: Laid::default(),
        }
    }

    /// An editor for the design file at `path`: a `.pen` document, or a bundle, which is unpacked. With it, a
    /// sentence when a newer Pencraft wrote the file and this one doesn't know all of it.
    ///
    /// # Errors
    /// A sentence when the file can't be read or isn't a design file.
    // Agent: READS `path`; for a bundle WRITES its working folder. BLOCKS on both.
    pub fn open(path: &Path, seed: u64) -> Result<(Editor, Option<String>), String> {
        let (doc, work, layout) = if crate::bundle::is_bundle(path) {
            let opened = crate::bundle::open(path)?;
            (opened.doc, Some(opened.work), opened.newer)
        } else {
            let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            (pen_doc::read(&text).map_err(|e| e.to_string())?, None, None)
        };
        let newer = newer_than_this(&doc, layout);
        let mut editor = Editor::new(doc, Some(path.to_path_buf()), seed);
        editor.work = work;
        Ok((editor, newer))
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
    // Human: a document that went from a bundle to a `.pen` file keeps the folder until it is closed, but
    // its paths start beside the `.pen` file from then on.
    fn unpacked(&self) -> Option<&crate::bundle::Work> {
        self.work.as_ref().filter(|_| self.path.as_deref().is_some_and(crate::bundle::is_bundle))
    }

    /// Whether the document has a bundle's working folder: it is saved as a bundle, or was in this run.
    #[must_use]
    pub fn bundled(&self) -> bool {
        self.work.is_some()
    }

    /// Says the document is to be saved at `path` from now on, and gives the [`Editor::anchor`] it had. Going
    /// to a bundle, it gets a working folder; going from one to a `.pen` file, it keeps the folder until it is
    /// closed.
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

    /// A sentence naming what the document uses that only Pencraft draws, for a document saved as a `.pen`
    /// file other programs open too; `None` when it uses nothing of the kind.
    #[must_use]
    pub fn only_here(&self) -> Option<String> {
        let labels: Vec<&str> = pen_doc::stamp::features(&self.doc).into_iter().map(pen_doc::stamp::Feature::label).collect();
        let list = match labels.as_slice() {
            [] => return None,
            [one] => (*one).to_owned(),
            [first @ .., last] => format!("{} and {last}", first.join(", ")),
        };
        Some(format!("Only Pencraft draws its {list}; other programs that open .pen files leave them out."))
    }

    /// Lets go of the working folder, when the document is closed.
    // Agent: WRITES (removes the folder).
    pub fn close(&mut self) {
        if let Some(work) = self.work.take() {
            work.discard();
        }
    }

    /// The tab title: the file name without `.pen`, or "Untitled".
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
        self.undo.push(self.doc.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
        self.needs_layout = true;
    }

    /// Marks the document changed without a new undo step (continuing a drag).
    pub fn touched(&mut self) {
        self.dirty = true;
        self.needs_layout = true;
    }

    /// Steps back one change; while the pen is drawing a path, that is its last point.
    pub fn undo(&mut self) {
        if self.pen.pop() {
            return;
        }
        if let Some(previous) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc, previous));
            self.after_history();
        }
    }

    /// Steps forward again after an undo.
    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc, next));
            self.after_history();
        }
    }

    fn after_history(&mut self) {
        self.dirty = true;
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
            *place = root;
            self.laid.nodes[at] = node.clone();
            self.laid.shows[at] = shown_by(node);
            self.laid.stamps[at] = next_number();
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
    pub fn scene_node(&self, id: &str) -> Option<&SceneNode> {
        find_in(&self.scene, id)
    }

    /// The scene node that holds the one with `id`.
    #[must_use]
    pub fn scene_parent(&self, id: &str) -> Option<&SceneNode> {
        fn search<'a>(nodes: &'a [SceneNode], id: &str) -> Option<&'a SceneNode> {
            nodes.iter().find_map(|n| if n.children.iter().any(|c| c.id == id) { Some(n) } else { search(&n.children, id) })
        }
        search(&self.scene, id)
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
        for past in self.undo.iter_mut().chain(&mut self.redo) {
            image_urls(past, &mut |url| {
                if let Some(Some(to)) = moved.get(url.as_str()) {
                    url.clone_from(to);
                }
            });
        }
        self.needs_layout = true;
    }

    /// Writes the document to its file, crash-safely: a `.pen` file as JSON, a bundle with the files the
    /// document names. Stamps it as Pencraft's first ([`pen_doc::stamp`]). Gives a sentence for what the user
    /// should know about the file that was written.
    ///
    /// # Errors
    /// When there is no path yet, the JSON can't be made, or the file can't be written.
    // Agent: WRITES the file; for a bundle READS the files it names, whole. BLOCKS on both.
    pub fn save(&mut self) -> Result<Vec<String>, String> {
        let Some(path) = self.path.clone() else { return Err("the document has no file yet".to_owned()) };
        pen_doc::stamp::apply(&mut self.doc);
        let mut told = Vec::new();
        let bytes = match self.unpacked() {
            Some(work) => {
                let packed = crate::bundle::pack(&self.doc, work.folder())?;
                if !packed.missing.is_empty() {
                    told.push(format!("These files the document names weren't found and aren't in the bundle: {}.", packed.missing.join(", ")));
                }
                packed.bytes
            }
            None => pen_doc::write(&self.doc).map_err(|e| e.to_string())?.into_bytes(),
        };
        craft_atomic::atomic_write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        self.dirty = false;
        Ok(told)
    }
}

/// A sentence for a file a newer Pencraft wrote: its stamp says a higher format number or features this
/// version has no name for, or its bundle a higher layout number (`layout`). `None` for a file this version
/// knows all of.
fn newer_than_this(doc: &Document, layout: Option<u64>) -> Option<String> {
    let stamp = pen_doc::stamp::newer(doc);
    if stamp.is_none() && layout.is_none() {
        return None;
    }
    let unknown = stamp.map(|stamp| stamp.features).unwrap_or_default();
    let what = if unknown.is_empty() { String::new() } else { format!(" ({})", unknown.join(", ")) };
    Some(format!("A newer Pencraft wrote this file. This version may not draw all of it{what}; what it doesn't know is kept when you save."))
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
