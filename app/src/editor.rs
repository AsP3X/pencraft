//! One open document: its file, the document itself, the laid-out scene, selection, camera and undo history.

use std::collections::HashSet;
use std::path::PathBuf;

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

/// One open document.
pub struct Editor {
    /// Where it is saved; `None` until first saved.
    pub path: Option<PathBuf>,
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
}

impl Editor {
    /// An editor for `doc`, saved at `path` if it has one.
    #[must_use]
    pub fn new(doc: Document, path: Option<PathBuf>, seed: u64) -> Editor {
        Editor {
            path,
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
    pub fn refresh(&mut self, measure: &Measure<'_>) {
        if !self.needs_layout {
            return;
        }
        self.needs_layout = false;
        let mut scene = scene::build(&self.doc, &self.root_theme());
        layout::layout(&mut scene, Vars::new(&self.doc), measure);
        self.scene = scene;
        self.generation += 1;
    }

    /// Whether the document changed, or the faces of its text did, since it was last laid out.
    #[must_use]
    pub fn stale(&self) -> bool {
        self.needs_layout
    }

    /// Forces a relayout (after the theme changed).
    pub fn relayout(&mut self) {
        self.needs_layout = true;
    }

    /// The laid-out scene node for an id: a document node's id, or `instance/inner` for a node inside an instance.
    #[must_use]
    pub fn scene_node(&self, id: &str) -> Option<&SceneNode> {
        fn search<'a>(nodes: &'a [SceneNode], id: &str) -> Option<&'a SceneNode> {
            nodes.iter().find_map(|n| if n.id == id { Some(n) } else { search(&n.children, id) })
        }
        search(&self.scene, id)
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

    /// Writes the document to its file, crash-safely.
    ///
    /// # Errors
    /// When there is no path yet, the JSON can't be made, or the file can't be written.
    pub fn save(&mut self) -> Result<(), String> {
        let Some(path) = self.path.clone() else { return Err("the document has no file yet".to_owned()) };
        let text = pen_doc::write(&self.doc).map_err(|e| e.to_string())?;
        craft_atomic::atomic_write(&path, text.as_bytes()).map_err(|e| format!("{}: {e}", path.display()))?;
        self.dirty = false;
        Ok(())
    }
}
