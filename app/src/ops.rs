//! Editing commands on the selection: the actions behind the shortcuts and menus.
//!
//! Every command records one undo step and leaves the document valid; a command that doesn't apply (nothing
//! selected, nodes with different parents) changes nothing.

use pen_doc::edit::{self, Slot};
use pen_doc::geom::{Point, Rect};
use pen_doc::layout;
use pen_doc::model::{Kind, LayoutMode, Node, Padding};
use pen_doc::scene::SceneNode;
use pen_doc::value::{Flag, Num, number};

use serde_json::Value;

use crate::editor::{Editor, Role};

/// How far an arrow key moves the selection.
pub const NUDGE: f32 = 1.0;
/// How far ⇧ + arrow moves it.
pub const BIG_NUDGE: f32 = 10.0;
/// Gap between a duplicated top-level frame and what is already on its right.
pub const DUPLICATE_FRAME_GAP: f32 = 40.0;
/// Gap and padding "Add flex layout" uses when it has nothing to measure.
pub const FLEX_DEFAULT_SPACING: f32 = 10.0;
/// How close two sizes must be to count as equal when guessing a flex layout.
const FLEX_TOLERANCE: f32 = 0.5;

/// Which edge or centre to align to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignTo {
    /// Left edges (⌥A).
    Left,
    /// Horizontal centres (⌥H).
    Center,
    /// Right edges (⌥D).
    Right,
    /// Top edges (⌥W).
    Top,
    /// Vertical middles (⌥V).
    Middle,
    /// Bottom edges (⌥S).
    Bottom,
}

/// Where in its parent's stack to move a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// One step forward (⌘]).
    Forward,
    /// One step back (⌘[).
    Backward,
    /// To the front (]).
    Front,
    /// To the back ([).
    Back,
}

/// A clipboard node on its way into the document.
struct Incoming {
    /// The node as copied.
    node: Node,
    /// The layer it was copied from, when that is still in this document.
    source: Option<String>,
    /// Its stored x/y.
    stored: Point,
    /// Where that origin is in document coordinates.
    origin: Point,
    /// Its box in document coordinates.
    bounds: Rect,
}

/// Where one pasted copy goes.
struct Placement {
    /// The copy.
    node: Node,
    /// The container it goes into; `None` = the canvas.
    parent: Option<String>,
    /// Its origin in document coordinates.
    origin: Point,
    /// Its box in document coordinates.
    bounds: Rect,
    /// Whether it left its original's place, so its x/y must be written.
    moved: bool,
    /// Whether it sits on the canvas itself and may still be moved to the middle of the view.
    loose: bool,
}

/// A parent (`None` = top level) and its selected children as (index, id), in stack order.
type Siblings = (Option<String>, Vec<(usize, String)>);

/// A node's own x/y as stored (0 when missing).
#[must_use]
pub fn stored_position(node: &Node) -> (f32, f32) {
    let read = |n: &Option<serde_json::Number>| n.as_ref().and_then(serde_json::Number::as_f64).map_or(0.0, pen_doc::value::narrow);
    (read(&node.x), read(&node.y))
}

/// Sets a node's stored x/y.
pub fn set_position(node: &mut Node, x: f32, y: f32) {
    node.x = Some(number(x));
    node.y = Some(number(y));
}

/// Whether the node with `id` is placed by its own x/y (its parent has no flex layout, or it is absolute).
#[must_use]
pub fn is_free(editor: &Editor, id: &str) -> bool {
    let Some(slot) = edit::slot(&editor.doc, id) else { return false };
    let absolute = edit::find(&editor.doc, id).is_some_and(Node::is_absolute);
    match slot.parent.as_deref().and_then(|p| edit::find(&editor.doc, p)) {
        None => true,
        Some(parent) => absolute || parent.layout_mode() == LayoutMode::None,
    }
}

/// A colour as the file writes it: `#rrggbb`, with its alpha behind it when it isn't fully solid.
#[must_use]
pub fn hex_of(color: pen_doc::color::Rgba) -> String {
    if color.a == 255 { format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b) } else { color.to_hex() }
}

/// The property that locks a layer: `locked: true`.
pub const LOCKED: &str = "locked";

/// Whether `node` says it is locked.
#[must_use]
pub fn is_locked(node: &Node) -> bool {
    node.extra.get(LOCKED) == Some(&serde_json::Value::Bool(true))
}

impl Editor {
    /// Whether the layer with `id` is locked: it says so itself, or a layer it is in does.
    // Human: a locked layer stays where it is. A click on the canvas goes through it to what is behind, so a
    // background, a guide picture or a finished part can't be picked up and moved by accident; it is still
    // selected from the layers list, where it is unlocked again. Locking a frame locks everything in it.
    #[must_use]
    pub fn locked(&self, id: &str) -> bool {
        let root = id.split('/').next().unwrap_or(id);
        edit::ancestry(&self.doc, root).iter().any(|layer| edit::find(&self.doc, layer).is_some_and(is_locked))
            || self.scene_node(id).is_some_and(|scene| is_locked(&scene.node))
    }

    /// Whether any of the layers `ids` is locked, as [`Editor::locked`] says of each.
    // Human: the canvas asks this of the whole selection several times a frame. Asking layer by layer goes
    // through the document for each one; here the document is gone through once, carrying down whether a
    // layer on the way was locked.
    // Agent: READS the document as it is now (not the scene, which an edit in this frame may have left behind).
    #[must_use]
    pub fn any_locked(&self, ids: &[String]) -> bool {
        fn visit(nodes: &[Node], held: bool, wanted: &std::collections::HashSet<&str>) -> bool {
            nodes.iter().any(|node| {
                let locked = held || is_locked(node);
                (locked && wanted.contains(node.id.as_str())) || visit(node.kids(), locked, wanted)
            })
        }
        if let [id] = ids {
            return self.locked(id);
        }
        let wanted = ids.iter().map(|id| id.split('/').next().unwrap_or(id)).collect();
        visit(&self.doc.children, false, &wanted) || ids.iter().any(|id| self.scene_node(id).is_some_and(|scene| is_locked(&scene.node)))
    }

    /// Gives the selected layers the colour `color`: as their fill, or as their stroke when `stroke` says so or
    /// the layer is a line or a connector, which have no fill. One undo step.
    pub fn drop_color(&mut self, color: pen_doc::color::Rgba, stroke: bool) {
        // Human: a colour that is fully solid is written the short way, without its alpha.
        let hex = hex_of(color);
        self.set_on_selection(|node| {
            let paint = Some(pen_doc::model::Paints::One(pen_doc::model::Paint::Solid(hex.clone())));
            if stroke || matches!(node.kind, Kind::Line | Kind::Connector) {
                node.stroke = paint;
            } else {
                node.fill = paint;
            }
        });
    }

    /// Locks the selected layers, or unlocks them when every one of them is locked already (⇧⌘L).
    pub fn toggle_lock(&mut self) {
        let unlock = !self.selection.is_empty() && self.selection.iter().all(|id| self.scene_node(id).is_some_and(|scene| is_locked(&scene.node)));
        self.set_on_selection(|node| lock(node, !unlock));
    }

    /// The selection's common parent and the selected slots in stack order, when all share one parent.
    fn siblings(&self) -> Option<Siblings> {
        let mut parent: Option<Option<String>> = None;
        let mut slots = Vec::new();
        for id in &self.selection {
            let slot = edit::slot(&self.doc, id)?;
            match &parent {
                None => parent = Some(slot.parent.clone()),
                Some(p) if *p != slot.parent => return None,
                Some(_) => {}
            }
            slots.push((slot.index, id.clone()));
        }
        slots.sort();
        Some((parent?, slots))
    }

    /// Deletes the selection (⌫).
    pub fn delete_selection(&mut self) {
        // Human: while a path's points are edited, Delete takes out the selected points, never the layer.
        if self.delete_path_points() || self.selection.is_empty() {
            return;
        }
        self.checkpoint();
        // Human: what changes is the layer each deleted one lay in, or the instance that now hides it.
        let mut changed: Vec<Option<String>> = Vec::new();
        for id in std::mem::take(&mut self.selection) {
            // Human: a layer inside an instance belongs to the component and can't be taken out of one instance;
            // the instance hides it instead (an override of `enabled` with `false`).
            // What the instance put into a slot itself is its own to remove.
            let owner = edit::owner(&self.doc, &id);
            let own = owner.clone().zip(id.rsplit('/').next().map(str::to_owned));
            if let Some(((instance, _), last)) = own
                && edit::remove_override_child(&mut self.doc, &instance, &last)
            {
                changed.push(Some(instance));
                continue;
            }
            if self.role(&id) == Role::Inside {
                self.change(&id, &|node| node.enabled = Some(Flag::Value(false)));
                changed.push(owner.map(|(instance, _)| instance));
            } else if let Some((_, slot)) = edit::remove(&mut self.doc, &id) {
                changed.push(slot.parent);
            }
        }
        self.changed_only(&changed);
    }

    /// Adds `nodes` to the frame with scene id `slot` inside an instance, as content of that instance alone, and
    /// selects them. Returns whether the frame was found.
    // Human: an instance fills a slot by listing the slot's children itself (a `children` override); the
    // component's slot stays empty for every other instance. In a slot without a layout the new layers are
    // centred.
    // Agent: the caller makes the undo step. What the slot shows from the component (normally nothing) is
    // replaced the first time, since an overridden children list is the whole list.
    pub fn fill_slot(&mut self, slot: &str, nodes: Vec<Node>) -> bool {
        let Some((instance, key)) = edit::owner(&self.doc, slot) else { return false };
        let Some(frame) = self.scene_node(slot) else { return false };
        let (free, room) = (frame.node.layout_mode() == LayoutMode::None, (frame.local.w, frame.local.h));
        let mut children = edit::override_children(&self.doc, &instance, &key).unwrap_or_default();
        let mut added = Vec::new();
        for mut node in nodes {
            let size = self.scene_node(&node.id).map_or((0.0, 0.0), |scene| (scene.local.w, scene.local.h));
            self.link_components(&mut node);
            self.renew_ids(&mut node);
            // Ids must also differ from layers this instance already holds in its slots, which aren't document layers.
            while added.contains(&node.id) || children.iter().any(|c| c.get("id").and_then(Value::as_str) == Some(node.id.as_str())) {
                node.id.push('x');
            }
            node.layout_position = None;
            if free {
                set_position(&mut node, ((room.0 - size.0) / 2.0).max(0.0).round(), ((room.1 - size.1) / 2.0).max(0.0).round());
            } else {
                (node.x, node.y) = (None, None);
            }
            let Ok(value) = serde_json::to_value(&node) else { continue };
            added.push(node.id);
            children.push(value);
        }
        let mut change = serde_json::Map::new();
        change.insert("children".to_owned(), Value::Array(children));
        if !edit::set_overrides(&mut self.doc, &instance, Some(&key), &change) {
            return false;
        }
        // A filled layer's scene id is the slot's with the layer's own id at the end.
        let prefix = slot.rsplit_once('/').map_or(instance.as_str(), |(prefix, _)| prefix);
        self.selection = added.iter().map(|id| format!("{prefix}/{id}")).collect();
        true
    }

    /// Puts an instance of the component with id `component` into the slot with scene id `slot`.
    pub fn fill_slot_with(&mut self, slot: &str, component: &str) {
        // Human: handed over as a copy of the component, which `fill_slot` turns into an instance of it and
        // centres by the component's size, like a pasted copy.
        let Some(mut copy) = edit::find(&self.doc, component).filter(|node| node.reusable == Some(true)).cloned() else { return };
        copy.children = None;
        self.checkpoint();
        self.fill_slot(slot, vec![copy]);
    }

    /// Gives the slot with scene id `slot` back to its component: the instance's own content for it is dropped.
    pub fn clear_slot(&mut self, slot: &str) {
        let Some((instance, key)) = edit::owner(&self.doc, slot) else { return };
        if edit::override_children(&self.doc, &instance, &key).is_some() {
            self.checkpoint();
            edit::clear_override(&mut self.doc, &instance, &key, "children");
            self.selection = vec![slot.to_owned()];
        }
    }

    /// The selected layers as they are now, to turn from (see [`Editor::rotate_from`]). `None` when nothing
    /// that can be turned is selected.
    // Human: one layer turns around the middle of its own box; several turn together around the middle of
    // the selection. A layer inside an instance, or inside another selected layer, isn't turned by itself.
    #[must_use]
    pub fn rotate_start(&self) -> Option<RotateStart> {
        let mut items = Vec::new();
        for id in &self.selection {
            let (Some(node), Some(shown)) = (edit::find(&self.doc, id), self.scene_node(id)) else { continue };
            if edit::ancestry(&self.doc, id).iter().any(|a| a != id && self.selection.contains(a)) {
                continue;
            }
            let turned = node.rotation.as_ref().map_or(Some(0.0), Num::literal);
            // A rotation bound to a variable is the variable's to set.
            if let Some(rotation) = turned {
                items.push((id.clone(), rotation, shown.to_doc.apply(Point::new(0.0, 0.0))));
            }
        }
        let centre = match items.as_slice() {
            [] => return None,
            [(id, ..)] => self.scene_node(id).map(|shown| shown.to_doc.apply(Point::new(shown.local.w / 2.0, shown.local.h / 2.0)))?,
            _ => self.selection_bounds()?.center(),
        };
        Some(RotateStart { items, centre })
    }

    /// Turns the layers of `start` by `degrees` (counter-clockwise) from where they were then.
    // Human: a layer's `rotation` turns it around its top left corner (schema). To turn it around another point
    // the rotation changes by the angle and the corner itself travels around that point by the same angle.
    // A layer placed by a layout only gets the rotation; the layout puts it in its place.
    // Agent: always starts from `start`, so a drag can call this every frame. The caller makes the undo step.
    pub fn rotate_from(&mut self, start: &RotateStart, degrees: f32) {
        let turn = pen_doc::geom::Affine::rotate_ccw(degrees);
        for (id, rotation, origin) in &start.items {
            let arm = turn.apply(Point::new(origin.x - start.centre.x, origin.y - start.centre.y));
            let moved = Point::new(start.centre.x + arm.x, start.centre.y + arm.y);
            let parent = edit::slot(&self.doc, id).and_then(|slot| slot.parent);
            let local = parent.as_deref().and_then(|p| self.scene_node(p)).and_then(|p| layout::to_local(p, moved)).unwrap_or(moved);
            let free = is_free(self, id);
            let Some(node) = edit::find_mut(&mut self.doc, id) else { continue };
            // Kept within −180…180 and to two decimals, like a typed value.
            let mut angle = (rotation + degrees).rem_euclid(360.0);
            if angle > 180.0 {
                angle -= 360.0;
            }
            let angle = (angle * 100.0).round() / 100.0;
            node.rotation = (angle.abs() > f32::EPSILON).then(|| Num::new(angle));
            if free {
                set_position(node, local.x, local.y);
            }
        }
    }

    /// Turns the selection by `degrees` (counter-clockwise) as one undo step.
    pub fn rotate_selection(&mut self, degrees: f32) {
        if let Some(start) = self.rotate_start() {
            self.checkpoint();
            self.rotate_from(&start, degrees);
        }
    }

    /// The document as it is now, to scale from (see [`Editor::scale_from`]). `None` with nothing selected.
    #[must_use]
    pub fn scale_start(&self) -> Option<ScaleStart> {
        Some(ScaleStart { bounds: self.selection_bounds()?, doc: self.doc.clone(), scene: self.scene.clone() })
    }

    /// Makes the document `start` with the selection scaled by `factor` around `anchor` (document coordinates).
    // Human: the Scale tool: the factor is kept between 0.01 and 100; a selected layer moves
    // away from or towards the anchor and everything in it scales with it: positions, fixed sizes, font size
    // and letter spacing, gap, padding, stroke width, corner radius, and shadows' and blurs' sizes. A layer placed
    // by a layout keeps its place; sizes that fill or fit, and values bound to variables, are left alone.
    // Agent: always starts from `start`, so a drag can call this every frame without the error adding up. The
    // caller makes the undo step, and says the document changed: the layers written are returned for that
    // ([`Editor::touched_only`]).
    // Human: only the scaled layers are taken from `start` again, not the whole document: copying a large
    // design on every frame of the drag, and comparing all of it afterwards, was most of what a step cost.
    // Nothing else changes between `start` and the end of the drag.
    pub fn scale_from(&mut self, start: &ScaleStart, factor: f32, anchor: Point) -> Vec<String> {
        fn find<'a>(nodes: &'a [SceneNode], id: &str) -> Option<&'a SceneNode> {
            nodes.iter().find_map(|n| if n.id == id { Some(n) } else { find(&n.children, id) })
        }
        let factor = if factor.is_finite() { factor.clamp(SCALE_MIN, SCALE_MAX) } else { 1.0 };
        let selected: Vec<String> = self.selection.clone();
        let mut written = Vec::new();
        for id in &selected {
            // A layer inside another selected one is scaled with that one.
            let chain = edit::ancestry(&self.doc, id);
            if chain.iter().any(|a| a != id && selected.contains(a)) {
                continue;
            }
            let Some(shown) = find(&start.scene, id) else { continue };
            let origin = shown.to_doc.apply(Point::new(0.0, 0.0));
            let moved = Point::new(anchor.x + (origin.x - anchor.x) * factor, anchor.y + (origin.y - anchor.y) * factor);
            let parent = edit::slot(&self.doc, id).and_then(|slot| slot.parent);
            let local = parent.as_deref().and_then(|p| find(&start.scene, p)).and_then(|p| layout::to_local(p, moved)).unwrap_or(moved);
            let (Some(was), Some(node)) = (edit::find(&start.doc, id), edit::find_mut(&mut self.doc, id)) else { continue };
            node.clone_from(was);
            let free = is_free(self, id);
            let Some(node) = edit::find_mut(&mut self.doc, id) else { continue };
            if free {
                set_position(node, local.x, local.y);
            }
            scale_node(node, factor, false);
            written.push(id.clone());
        }
        written
    }

    /// Scales the selection to `factor` of what it was when it was selected, around its middle (the Scale
    /// tool's number field).
    pub fn scale_selection_to(&mut self, factor: f32) {
        if self.scaled.1 != self.selection {
            self.scaled = (1.0, self.selection.clone());
        }
        let factor = factor.clamp(SCALE_MIN, SCALE_MAX);
        let Some(start) = self.scale_start() else { return };
        self.scale_from(&start, factor / self.scaled.0, start.bounds.center());
        self.scaled.0 = factor;
    }

    /// Adds the picture or SVG file at `path` as a new layer in the middle of `visible` and selects it.
    ///
    /// # Errors
    /// A sentence when the file can't be read as a picture or an SVG.
    // Human: a picture becomes a rectangle its own size with the picture as its fill (shrunk to fit the view
    // when larger); the fill names the file relative to the document's folder when it lies in it. An SVG's
    // shapes become one path layer in the SVG's own box.
    pub fn import_file(&mut self, path: &std::path::Path, visible: Rect) -> Result<(), String> {
        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned());
        let is_svg = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
        let mut node = if is_svg {
            let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            crate::icons::svg_layer(&text, self.fresh_id()).ok_or("the SVG has no shapes this version can read")?
        } else {
            let (w, h) = image::image_dimensions(path).map_err(|e| e.to_string())?;
            let folder = self.folder();
            // Human: a path inside the document's folder is written from there, with `/` between its parts on
            // every system, so the document shows its pictures on a Mac and on Windows alike.
            let parts = |rel: &std::path::Path| rel.iter().map(|part| part.to_string_lossy()).collect::<Vec<_>>().join("/");
            let url = folder.and_then(|f| path.strip_prefix(f).ok()).map_or_else(|| path.display().to_string(), |rel| format!("./{}", parts(rel)));
            self.picture_layer(&url, (w, h), visible)
        };
        node.name = name;
        let size = |n: &Option<Num>| n.as_ref().and_then(Num::literal).unwrap_or(0.0);
        let centre = visible.center();
        let (w, h) = (size(&node.width), size(&node.height));
        set_position(&mut node, (centre.x - w / 2.0).round(), (centre.y - h / 2.0).round());
        self.checkpoint();
        self.selection = vec![node.id.clone()];
        edit::insert(&mut self.doc, None, usize::MAX, node);
        Ok(())
    }

    /// A new rectangle filled with the picture an image fill names as `url`, as large as the picture's `pixels`,
    /// shrunk to fit the `visible` part of the canvas when it is larger.
    fn picture_layer(&mut self, url: &str, pixels: (u32, u32), visible: Rect) -> Node {
        let mut node = Node::new(Kind::Rectangle, self.fresh_id());
        let (w, h) = (pen_doc::value::narrow(f64::from(pixels.0)), pen_doc::value::narrow(f64::from(pixels.1)));
        let fit = (visible.w * IMPORT_ROOM / w).min(visible.h * IMPORT_ROOM / h).min(1.0);
        (node.width, node.height) = (Some(Num::new((w * fit).round().max(1.0))), Some(Num::new((h * fit).round().max(1.0))));
        node.fill = Some(pen_doc::model::Paints::One(pen_doc::model::Paint::Object(
            serde_json::json!({ "type": "image", "url": url }).as_object().cloned().unwrap_or_default(),
        )));
        node
    }

    /// Pastes a picture that was on the clipboard and is now the file an image fill names as `url`: a rectangle
    /// of the picture's size (`pixels`, shrunk to fit the view) with the picture as its fill, placed as any
    /// paste is and selected.
    pub fn paste_picture(&mut self, url: &str, pixels: (u32, u32), visible: Rect) {
        let mut node = self.picture_layer(url, pixels, visible);
        node.name = Some("Pasted picture".to_owned());
        set_position(&mut node, 0.0, 0.0);
        self.paste(&[node], visible);
    }

    /// Pastes SVG text as one path layer, placed as any paste is and selected; `false` when the text is no SVG
    /// or has no shape this version reads.
    // Human: an icon copied from a website or another design tool arrives as the SVG's text. Pasted as a text
    // layer it would be a page of angle brackets; it is drawn instead, the way "Import Image or SVG" draws a file.
    pub fn paste_svg(&mut self, text: &str, visible: Rect) -> bool {
        let start = text.trim_start();
        let looks_like_one = start.starts_with("<svg") || (start.starts_with("<?xml") || start.starts_with("<!DOCTYPE")) && start.contains("<svg");
        let Some(mut node) = looks_like_one.then(|| crate::icons::svg_layer(text, self.fresh_id())).flatten() else { return false };
        node.name = Some("Pasted SVG".to_owned());
        set_position(&mut node, 0.0, 0.0);
        self.paste(&[node], visible);
        true
    }

    /// Turns every component in `copy` (a copy of document layers, still with their ids) into an instance of it.
    // Human: copying a component on the canvas makes an instance of it. A copy
    // of a component is never a second component; it shows the first one. A component that is no longer in the
    // document (cut, or copied from another file) is pasted as it is.
    fn link_components(&self, copy: &mut Node) {
        let is_component = copy.reusable == Some(true) && edit::find(&self.doc, &copy.id).is_some_and(|n| n.reusable == Some(true));
        if is_component {
            let mut instance = Node::new(Kind::Ref, copy.id.clone());
            instance.reference = Some(copy.id.clone());
            (instance.x, instance.y) = (copy.x.take(), copy.y.take());
            instance.layout_position = copy.layout_position.take();
            *copy = instance;
            return;
        }
        for child in copy.children.iter_mut().flatten() {
            self.link_components(child);
        }
    }

    /// Puts a new instance of the component with id `component` in the middle of the view and selects it.
    pub fn insert_instance(&mut self, component: &str) {
        let Some(bounds) = self.scene_node(component).map(layout::doc_bounds) else { return };
        self.checkpoint();
        let mut instance = Node::new(Kind::Ref, self.fresh_id());
        instance.reference = Some(component.to_owned());
        set_position(&mut instance, (self.camera.center.x - bounds.w / 2.0).round(), (self.camera.center.y - bounds.h / 2.0).round());
        self.selection = vec![instance.id.clone()];
        edit::insert(&mut self.doc, None, usize::MAX, instance);
    }

    /// Selects the component the selected instance shows and brings it into view.
    pub fn go_to_component(&mut self) {
        let Some(id) = self.selection.first() else { return };
        let instance = edit::owner(&self.doc, id).map_or_else(|| id.clone(), |(instance, _)| instance);
        let Some(component) = edit::find(&self.doc, &instance).and_then(|node| node.reference.clone()) else { return };
        if let Some(bounds) = self.scene_node(&component).map(layout::doc_bounds) {
            self.focus = Some(bounds);
            self.selection = vec![component];
        }
    }

    /// Copies of the selected nodes, top-most last.
    #[must_use]
    pub fn copy_selection(&self) -> Vec<Node> {
        let mut nodes: Vec<(Option<Slot>, Node)> =
            self.selection.iter().filter_map(|id| edit::find(&self.doc, id).map(|n| (edit::slot(&self.doc, id), n.clone()))).collect();
        nodes.sort_by_key(|(slot, _)| slot.as_ref().map_or(0, |s| s.index));
        nodes.into_iter().map(|(_, n)| n).collect()
    }

    /// Pastes `nodes` (⌘V) and selects the copies. `visible` is the part of the document the canvas shows.
    // Human: a paste is never offset. With a container selected on screen (or a
    // layer inside one) the copies go into it, centred, unless they came from it; otherwise they land exactly on
    // their originals when one of those is on screen; otherwise in the middle of the view, inside a layout-free
    // frame there if one wholly holds them.
    // Agent: positions are planned from the scene as it is before the change, then applied in one undo step. A
    // copy that stays with its original's parent keeps its stored x/y untouched (no rounding).
    pub fn paste(&mut self, nodes: &[Node], visible: Rect) {
        // Human: with a frame inside an instance selected (a slot, usually) the copies go into it, as that
        // instance's own content.
        if let [id] = self.selection.as_slice()
            && self.role(id) == Role::Inside
            && self.scene_node(id).is_some_and(|scene| matches!(scene.node.kind, Kind::Frame | Kind::Group))
        {
            let id = id.clone();
            self.checkpoint();
            self.fill_slot(&id, nodes.to_vec());
            return;
        }
        let incoming: Vec<Incoming> = nodes.iter().map(|n| self.incoming(n)).collect();
        let Some(all) = incoming.iter().map(|i| i.bounds).reduce(|a, b| a.union(&b)) else { return };
        let sources: Vec<String> = incoming.iter().filter_map(|i| i.source.clone()).collect();
        let source_on_screen = sources.iter().any(|id| self.scene_node(id).is_some_and(|s| visible.intersects(&layout::doc_bounds(s))));
        let targets = self.paste_targets(&sources, visible);
        let mut plan: Vec<Placement> = Vec::new();
        for item in incoming {
            let home = item.source.as_deref().and_then(|id| edit::slot(&self.doc, id)).map(|slot| slot.parent);
            if targets.is_empty() {
                let parent = if source_on_screen { home.flatten() } else { None };
                let origin = if source_on_screen { item.origin } else { item.stored };
                let bounds = item.bounds.translate(origin.x - item.origin.x, origin.y - item.origin.y);
                plan.push(Placement { loose: parent.is_none(), node: item.node, parent, origin, bounds, moved: false });
                continue;
            }
            for target in &targets {
                let stays = home.as_ref().is_some_and(|parent| parent.as_deref() == Some(target.as_str()));
                let shift = match self.scene_node(target) {
                    Some(scene) if !stays => {
                        let (to, from) = (layout::doc_bounds(scene).center(), all.center());
                        Point::new(to.x - from.x, to.y - from.y)
                    }
                    _ => Point::new(0.0, 0.0),
                };
                plan.push(Placement {
                    node: item.node.clone(),
                    parent: Some(target.clone()),
                    origin: Point::new(item.origin.x + shift.x, item.origin.y + shift.y),
                    bounds: item.bounds.translate(shift.x, shift.y),
                    moved: !stays,
                    loose: false,
                });
            }
        }
        if let Some(loose) = plan.iter().filter(|p| p.loose).map(|p| p.bounds).reduce(|a, b| a.union(&b))
            && (sources.is_empty() || !visible.intersects(&loose))
        {
            let (to, from) = (visible.center(), loose.center());
            let (dx, dy) = (to.x - from.x, to.y - from.y);
            let frame = self.plain_frame_holding(&loose.translate(dx, dy), &sources);
            for p in plan.iter_mut().filter(|p| p.loose) {
                p.origin = Point::new(p.origin.x + dx, p.origin.y + dy);
                p.parent.clone_from(&frame);
                p.moved = true;
            }
        }
        self.checkpoint();
        // Human: what changes is the layer each copy is put into.
        let (mut pasted, mut changed) = (Vec::new(), Vec::new());
        for Placement { mut node, parent, origin, moved, .. } in plan {
            self.link_components(&mut node);
            self.renew_ids(&mut node);
            if moved {
                let local = parent.as_deref().and_then(|id| self.scene_node(id)).and_then(|scene| layout::to_local(scene, origin)).unwrap_or(origin);
                set_position(&mut node, local.x, local.y);
            }
            pasted.push(node.id.clone());
            edit::insert(&mut self.doc, parent.as_deref(), usize::MAX, node);
            changed.push(parent);
        }
        self.changed_only(&changed);
        self.selection = pasted;
    }

    /// A clipboard node with where it would sit in the document as it is now.
    fn incoming(&self, node: &Node) -> Incoming {
        let (x, y) = stored_position(node);
        let stored = Point::new(x, y);
        let size = |n: &Option<Num>| match n {
            Some(Num::Value(v)) => v.as_f64().map_or(0.0, pen_doc::value::narrow),
            _ => 0.0,
        };
        let Some(scene) = self.scene_node(&node.id) else {
            let bounds = Rect::new(x, y, size(&node.width), size(&node.height));
            return Incoming { node: node.clone(), source: None, stored, origin: stored, bounds };
        };
        let parent = edit::slot(&self.doc, &node.id).and_then(|slot| slot.parent);
        let origin = parent.as_deref().and_then(|id| self.scene_node(id)).map_or(stored, |p| p.to_doc.apply(stored));
        Incoming { node: node.clone(), source: Some(node.id.clone()), stored, origin, bounds: layout::doc_bounds(scene) }
    }

    /// The containers a paste goes into: for each selected layer on screen that isn't one of the copied ones, the
    /// nearest of itself and its parents that holds children.
    fn paste_targets(&self, sources: &[String], visible: Rect) -> Vec<String> {
        let mut targets: Vec<String> = Vec::new();
        for id in &self.selection {
            if sources.contains(id) || !self.scene_node(id).is_some_and(|s| visible.intersects(&layout::doc_bounds(s))) {
                continue;
            }
            let mut at = Some(id.clone());
            while let Some(current) = at {
                if edit::find(&self.doc, &current).is_some_and(|n| n.kind.is_container()) {
                    if !targets.contains(&current) {
                        targets.push(current);
                    }
                    break;
                }
                at = edit::slot(&self.doc, &current).and_then(|slot| slot.parent);
            }
        }
        targets
    }

    /// The deepest frame without a flex layout under the centre of `bounds` that wholly holds it, skipping `exclude`.
    fn plain_frame_holding(&self, bounds: &Rect, exclude: &[String]) -> Option<String> {
        fn visit(doc: &pen_doc::Document, node: &SceneNode, bounds: &Rect, exclude: &[String], found: &mut Option<String>) {
            let Some(id) = node.doc_id.as_deref() else { return };
            let Some(doc_node) = edit::find(doc, id) else { return };
            let own = layout::doc_bounds(node);
            if exclude.iter().any(|e| e == id) || !doc_node.kind.is_container() || !own.contains(bounds.center()) {
                return;
            }
            if doc_node.kind == Kind::Frame && doc_node.layout_mode() == LayoutMode::None && own.holds(bounds) {
                *found = Some(id.to_owned());
            }
            for child in &node.children {
                visit(doc, child, bounds, exclude, found);
            }
        }
        let mut found = None;
        for root in &self.scene {
            visit(&self.doc, root, bounds, exclude, &mut found);
        }
        found
    }

    /// Duplicates the selection (⌘D).
    // Human: each copy goes right above its original, in the same place; a single
    // top-level frame instead goes to free space on its right, 40 units past whatever is there.
    pub fn duplicate(&mut self) {
        let ids = self.selection.clone();
        if ids.is_empty() {
            return;
        }
        let lone_board = match ids.as_slice() {
            [id] => edit::slot(&self.doc, id).is_some_and(|s| s.parent.is_none()) && edit::find(&self.doc, id).is_some_and(|n| n.kind == Kind::Frame),
            _ => false,
        };
        let board_spot = if lone_board { ids.first().and_then(|id| self.empty_space_right(id)) } else { None };
        self.checkpoint();
        // Human: what changes is the layer each copy is put into.
        let (mut copies, mut changed) = (Vec::new(), Vec::new());
        for id in ids {
            let (Some(slot), Some(mut copy)) = (edit::slot(&self.doc, &id), edit::find(&self.doc, &id).cloned()) else { continue };
            self.link_components(&mut copy);
            self.renew_ids(&mut copy);
            if let Some((x, y)) = board_spot {
                set_position(&mut copy, x, y);
            }
            copies.push(copy.id.clone());
            edit::insert(&mut self.doc, slot.parent.as_deref(), slot.index + 1, copy);
            changed.push(slot.parent);
        }
        self.changed_only(&changed);
        self.selection = copies;
    }

    /// Where a copy of top-level node `id` fits on its right:
    /// start just past the node, then step past every top-level node it would overlap, left to right.
    fn empty_space_right(&self, id: &str) -> Option<(f32, f32)> {
        let own = self.local_box(id)?;
        let (sx, sy) = edit::find(&self.doc, id).map(stored_position)?;
        let mut spot = Rect::new(own.right() + DUPLICATE_FRAME_GAP, own.y, own.w, own.h);
        let mut others: Vec<Rect> = self.scene.iter().filter(|n| n.doc_id.as_deref() != Some(id)).map(pen_doc::layout::doc_bounds).collect();
        others.sort_by(|a, b| a.x.total_cmp(&b.x));
        for other in others {
            if spot.intersects(&other) {
                spot.x = spot.x.max(other.right() + DUPLICATE_FRAME_GAP);
            }
        }
        // Human: from the box back to the stored position (they differ for groups and rotated frames).
        Some((sx + (spot.x - own.x), sy + (spot.y - own.y)))
    }

    /// The scene box of a document node in its parent's coordinates.
    fn local_box(&self, id: &str) -> Option<Rect> {
        self.scene_node(id).map(|s| s.local)
    }

    /// Wraps the selection in a group (⌘G) or a plain frame (⌥⌘G).
    // Human: a group sits at the top-left of what it holds; a frame gets that box as a fixed
    // size, the name "Frame N", no fill, no clipping and no flex layout. The wrapper takes the stack place of the
    // top-most wrapped layer.
    pub fn wrap_selection(&mut self, kind: &Kind) {
        let Some((parent, slots)) = self.siblings() else { return };
        let boxes: Vec<(String, Rect)> = slots.iter().filter_map(|(_, id)| self.local_box(id).map(|b| (id.clone(), b))).collect();
        let Some(bounds) = boxes.iter().map(|(_, b)| *b).reduce(|a, b| a.union(&b)) else { return };
        let mut container = Node::new(kind.clone(), self.fresh_id());
        set_position(&mut container, bounds.x, bounds.y);
        if *kind == Kind::Frame {
            container.name = Some(format!("Frame {}", next_frame_number(&self.doc.children)));
            container.layout = Some("none".to_owned());
            container.width = Some(Num::new(bounds.w));
            container.height = Some(Num::new(bounds.h));
        }
        self.wrap(parent.as_deref(), &slots, container, &boxes, bounds, None);
    }

    /// Moves the wrapped layers into `container`, keeping where they are (or in `order` for a flex frame).
    fn wrap(
        &mut self,
        parent: Option<&str>,
        slots: &[(usize, String)],
        mut container: Node,
        boxes: &[(String, Rect)],
        bounds: Rect,
        order: Option<&[String]>,
    ) {
        let top_index = slots.last().map_or(0, |s| s.0).saturating_sub(slots.len().saturating_sub(1));
        self.checkpoint();
        let mut children = Vec::new();
        for (id, _) in boxes {
            if let Some((mut node, _)) = edit::remove(&mut self.doc, id) {
                let (sx, sy) = stored_position(&node);
                // Human: shift by the box's offset from the stored position, so groups and rotated layers stay put.
                set_position(&mut node, sx - bounds.x, sy - bounds.y);
                children.push(node);
            }
        }
        if let Some(order) = order {
            children.sort_by_key(|n| order.iter().position(|id| *id == n.id).unwrap_or(usize::MAX));
        }
        container.children = Some(children);
        let id = container.id.clone();
        edit::insert(&mut self.doc, parent, top_index, container);
        self.selection = vec![id];
    }

    /// Ungroups the selected groups and frames: their children take their place (⇧⌘G).
    pub fn ungroup(&mut self) {
        let targets: Vec<String> = self
            .selection
            .iter()
            .filter(|id| edit::find(&self.doc, id).is_some_and(|n| n.kind.is_container() && !n.kids().is_empty()))
            .cloned()
            .collect();
        if targets.is_empty() {
            return;
        }
        self.checkpoint();
        let mut released = Vec::new();
        for id in targets {
            let Some(container_box) = self.local_box(&id) else { continue };
            let child_boxes: Vec<(String, Rect)> =
                self.scene_node(&id).map(|s| s.children.iter().filter_map(|c| Some((c.doc_id.clone()?, c.local))).collect()).unwrap_or_default();
            let Some((container, slot)) = edit::remove(&mut self.doc, &id) else { continue };
            for (offset, mut child) in container.children.unwrap_or_default().into_iter().enumerate() {
                if let Some((_, b)) = child_boxes.iter().find(|(cid, _)| *cid == child.id) {
                    set_position(&mut child, container_box.x + b.x, container_box.y + b.y);
                }
                released.push(child.id.clone());
                edit::insert(&mut self.doc, slot.parent.as_deref(), slot.index + offset, child);
            }
        }
        self.selection = released;
    }

    /// Turns on a flex layout (⇧A): a lone frame gets one fitted to its children, anything else is wrapped in a
    /// new flex frame. Direction, gap, padding, alignment and stretching are read from how the layers lie.
    pub fn add_flex_layout(&mut self) {
        if let [id] = self.selection.as_slice()
            && let Some(node) = edit::find(&self.doc, id)
            && node.kind == Kind::Frame
            && node.layout_mode() == LayoutMode::None
        {
            let id = id.clone();
            let Some(scene) = self.scene_node(&id) else { return };
            let frame = Rect::new(0.0, 0.0, scene.local.w, scene.local.h);
            let items: Vec<Item> = scene.children.iter().filter_map(|c| Some(Item::new(c.doc_id.clone()?, c))).collect();
            let plan = infer_flex(&items, Some(frame));
            self.checkpoint();
            if let Some(node) = edit::find_mut(&mut self.doc, &id) {
                plan.apply_to_frame(node);
                plan.apply_to_children(node.children.get_or_insert_with(Vec::new));
            }
            return;
        }
        let Some((parent, slots)) = self.siblings() else { return };
        let items: Vec<Item> = slots.iter().filter_map(|(_, id)| self.scene_node(id).map(|s| Item::new(id.clone(), s))).collect();
        if items.is_empty() {
            return;
        }
        let plan = infer_flex(&items, None);
        let bounds = plan.bounds;
        let mut container = Node::new(Kind::Frame, self.fresh_id());
        container.name = Some(format!("Frame {}", next_frame_number(&self.doc.children)));
        set_position(&mut container, bounds.x, bounds.y);
        plan.apply_to_frame(&mut container);
        if plan.stretch {
            let (w, h) = (bounds.w, bounds.h);
            if plan.horizontal { container.height = Some(Num::new(h)) } else { container.width = Some(Num::new(w)) }
        }
        let boxes: Vec<(String, Rect)> = items.iter().map(|i| (i.id.clone(), i.outer)).collect();
        self.wrap(parent.as_deref(), &slots, container, &boxes, bounds, Some(&plan.order));
        if let Some(frame) = self.selection.first().cloned()
            && let Some(node) = edit::find_mut(&mut self.doc, &frame)
        {
            plan.apply_to_children(node.children.get_or_insert_with(Vec::new));
        }
    }

    /// Turns a frame's flex layout off (⌥⇧A), keeping every child where it is.
    pub fn remove_flex_layout(&mut self) {
        let frames: Vec<String> =
            self.selection.iter().filter(|id| edit::find(&self.doc, id).is_some_and(|n| n.kind == Kind::Frame)).cloned().collect();
        if frames.is_empty() {
            return;
        }
        self.checkpoint();
        for id in frames {
            let boxes: Vec<(String, Rect)> =
                self.scene_node(&id).map(|s| s.children.iter().filter_map(|c| Some((c.doc_id.clone()?, c.local))).collect()).unwrap_or_default();
            let size = self.local_box(&id);
            if let Some(node) = edit::find_mut(&mut self.doc, &id) {
                node.layout = Some("none".to_owned());
                if let Some(size) = size {
                    node.width = Some(Num::new(size.w));
                    node.height = Some(Num::new(size.h));
                }
                for child in node.children.iter_mut().flatten() {
                    if let Some((_, b)) = boxes.iter().find(|(cid, _)| *cid == child.id) {
                        set_position(child, b.x, b.y);
                    }
                }
            }
        }
    }

    /// Moves the selection in its parent's stack.
    pub fn reorder(&mut self, order: Order) {
        let Some((parent, slots)) = self.siblings() else { return };
        let Some(len) = edit::children_mut(&mut self.doc.clone(), parent.as_deref()).map(|l| l.len()) else { return };
        self.checkpoint();
        let ids: Vec<String> = slots.iter().map(|(_, id)| id.clone()).collect();
        let iter: Box<dyn Iterator<Item = &String>> = match order {
            Order::Forward | Order::Front => Box::new(ids.iter().rev()),
            Order::Backward | Order::Back => Box::new(ids.iter()),
        };
        for (n, id) in iter.enumerate() {
            let Some((node, slot)) = edit::remove(&mut self.doc, id) else { continue };
            let index = match order {
                Order::Forward => (slot.index + 1).min(len - 1),
                Order::Backward => slot.index.saturating_sub(1),
                Order::Front => len - 1 - n,
                Order::Back => n,
            };
            edit::insert(&mut self.doc, parent.as_deref(), index, node);
        }
    }

    /// Mirrors the selection (⇧H, ⇧V).
    pub fn flip(&mut self, horizontal: bool) {
        if self.selection.is_empty() {
            return;
        }
        self.checkpoint();
        for id in self.selection.clone() {
            if let Some(node) = edit::find_mut(&mut self.doc, &id) {
                let field = if horizontal { &mut node.flip_x } else { &mut node.flip_y };
                let on = matches!(field, Some(Flag::Value(true)));
                *field = if on { None } else { Some(Flag::Value(true)) };
            }
        }
    }

    /// Moves freely placed selected nodes by (dx, dy) in their parent's units.
    pub fn nudge(&mut self, dx: f32, dy: f32) {
        if self.nudge_path_points(dx, dy) {
            return;
        }
        let ids: Vec<String> = self.selection.iter().filter(|id| is_free(self, id) && !self.locked(id)).cloned().collect();
        if ids.is_empty() {
            return;
        }
        self.checkpoint();
        for id in ids {
            if let Some(node) = edit::find_mut(&mut self.doc, &id) {
                let (x, y) = stored_position(node);
                set_position(node, x + dx, y + dy);
            }
        }
    }

    /// Aligns the selection: several nodes to their common bounds, one node within its parent.
    pub fn align(&mut self, to: AlignTo) {
        let ids: Vec<String> = self.selection.iter().filter(|id| is_free(self, id) && !self.locked(id)).cloned().collect();
        let boxes: Vec<(String, Rect)> = ids.iter().filter_map(|id| self.local_box(id).map(|b| (id.clone(), b))).collect();
        let target = if boxes.len() > 1 {
            boxes.iter().map(|(_, b)| *b).reduce(|a, b| a.union(&b))
        } else {
            boxes
                .first()
                .and_then(|(id, _)| edit::slot(&self.doc, id))
                .and_then(|s| s.parent)
                .and_then(|p| self.local_box(&p))
                .map(|b| Rect::new(0.0, 0.0, b.w, b.h))
        };
        let Some(target) = target else { return };
        self.checkpoint();
        for (id, b) in boxes {
            let (x, y) = match to {
                AlignTo::Left => (target.x, b.y),
                AlignTo::Center => (target.x + (target.w - b.w) / 2.0, b.y),
                AlignTo::Right => (target.right() - b.w, b.y),
                AlignTo::Top => (b.x, target.y),
                AlignTo::Middle => (b.x, target.y + (target.h - b.h) / 2.0),
                AlignTo::Bottom => (b.x, target.bottom() - b.h),
            };
            if let Some(node) = edit::find_mut(&mut self.doc, &id) {
                let (sx, sy) = stored_position(node);
                // Human: stored positions can differ from the laid-out box (groups); move by the difference.
                set_position(node, sx + (x - b.x), sy + (y - b.y));
            }
        }
    }

    /// Spreads three or more selected layers so the gaps between them are equal, `across` the page or down it:
    /// the two outermost stay and the others go between them in the order they lie in.
    // Human: with less room than the layers take, the equal gap is negative and they overlap by the same amount.
    // Layers a layout places are left alone, as in aligning, and so is anything when fewer than three can move.
    pub fn distribute(&mut self, across: bool) {
        let ids: Vec<String> = self.selection.iter().filter(|id| is_free(self, id) && !self.locked(id)).cloned().collect();
        let span = |b: &Rect| if across { (b.x, b.w) } else { (b.y, b.h) };
        let mut boxes: Vec<(String, f32, f32)> =
            ids.iter().filter_map(|id| self.local_box(id).map(|b| (id.clone(), span(&b).0, span(&b).1))).collect();
        if boxes.len() < 3 {
            return;
        }
        boxes.sort_by(|a, b| a.1.total_cmp(&b.1).then((a.1 + a.2).total_cmp(&(b.1 + b.2))));
        let Some(start) = boxes.first().map(|b| b.1) else { return };
        let end = boxes.iter().map(|b| b.1 + b.2).fold(f32::MIN, f32::max);
        let taken: f32 = boxes.iter().map(|b| b.2).sum();
        let gaps = u16::try_from(boxes.len() - 1).map_or(1.0, f32::from);
        let gap = (end - start - taken) / gaps;
        self.checkpoint();
        let mut at = start;
        for (id, was, size) in boxes {
            if let Some(node) = edit::find_mut(&mut self.doc, &id) {
                let (sx, sy) = stored_position(node);
                if across {
                    set_position(node, sx + (at - was), sy);
                } else {
                    set_position(node, sx, sy + (at - was));
                }
            }
            at += size + gap;
        }
    }

    /// Sets `textAlign` on selected text (⌥⌘L/T/R/J).
    pub fn text_align(&mut self, align: &str) {
        self.set_on_selection(|n| {
            if n.kind == Kind::Text {
                n.text_align = Some(align.to_owned());
            }
        });
    }

    /// Applies `change` to every selected node as one undo step.
    pub fn set_on_selection(&mut self, change: impl Fn(&mut Node)) {
        if self.selection.is_empty() {
            return;
        }
        self.checkpoint();
        for id in self.selection.clone() {
            self.change(&id, &change);
        }
    }

    /// Makes the selection reusable components (⌥⌘K).
    // Human: only a layer of its own can become one; an instance, or a layer inside one, shows another
    // component.
    pub fn create_component(&mut self) {
        let plain: Vec<String> = self.selection.iter().filter(|id| self.role(id) == Role::Plain).cloned().collect();
        if plain.is_empty() {
            return;
        }
        self.checkpoint();
        for id in plain {
            if let Some(node) = edit::find_mut(&mut self.doc, &id) {
                node.reusable = Some(true);
            }
        }
    }

    /// Makes the selected components plain layers again; their instances keep showing them.
    pub fn detach_components(&mut self) {
        let components: Vec<String> = self.selection.iter().filter(|id| self.role(id) == Role::Component).cloned().collect();
        if components.is_empty() {
            return;
        }
        self.checkpoint();
        for id in components {
            if let Some(node) = edit::find_mut(&mut self.doc, &id) {
                node.reusable = None;
            }
        }
    }

    /// Replaces selected instances with plain copies of what they show (⌥⌘X).
    pub fn detach_instances(&mut self) {
        let instances: Vec<String> =
            self.selection.iter().filter(|id| edit::find(&self.doc, id).is_some_and(|n| n.kind == Kind::Ref)).cloned().collect();
        if instances.is_empty() {
            return;
        }
        let rebuilt: Vec<(String, Node)> = instances.iter().filter_map(|id| self.scene_node(id).map(|s| (id.clone(), rebuild(s)))).collect();
        self.checkpoint();
        for (id, mut node) in rebuilt {
            let Some(slot) = edit::slot(&self.doc, &id) else { continue };
            let original = edit::find(&self.doc, &id).cloned();
            for child in node.children.iter_mut().flatten() {
                self.renew_ids(child);
            }
            node.id.clone_from(&id);
            if let Some(original) = original {
                node.x = original.x;
                node.y = original.y;
            }
            edit::remove(&mut self.doc, &id);
            edit::insert(&mut self.doc, slot.parent.as_deref(), slot.index, node);
        }
    }

    /// Selects every sibling of the selection, or every top-level node (⌘A).
    pub fn select_all(&mut self) {
        if self.select_all_path_points() {
            return;
        }
        let parent = self.selection.first().and_then(|id| edit::slot(&self.doc, id)).and_then(|s| s.parent);
        let list = match parent.as_deref().and_then(|p| edit::find(&self.doc, p)) {
            Some(parent) => parent.kids(),
            None => &self.doc.children,
        };
        self.selection = list.iter().map(|n| n.id.clone()).collect();
    }

    /// Selects the children of the selected node (↵).
    pub fn select_children(&mut self) {
        if let [id] = self.selection.as_slice()
            && let Some(node) = edit::find(&self.doc, id)
            && !node.kids().is_empty()
        {
            self.selection = node.kids().iter().map(|n| n.id.clone()).collect();
        }
    }

    /// Selects the parent of the selection (⇧↵).
    pub fn select_parent(&mut self) {
        if let Some(parent) = self.selection.first().and_then(|id| edit::slot(&self.doc, id)).and_then(|s| s.parent) {
            self.selection = vec![parent];
        }
    }

    /// The part of the document a canvas of `width` × `height` screen points shows.
    #[must_use]
    pub fn visible_area(&self, width: f32, height: f32) -> Rect {
        let (w, h) = (width / self.camera.zoom, height / self.camera.zoom);
        Rect::new(self.camera.center.x - w / 2.0, self.camera.center.y - h / 2.0, w, h)
    }
}

/// Least and most the Scale tool scales by.
pub const SCALE_MIN: f32 = 0.01;
/// See [`SCALE_MIN`].
pub const SCALE_MAX: f32 = 100.0;
/// The part of the view an imported picture may fill at most.
const IMPORT_ROOM: f32 = 0.8;

/// Locks `node`, or unlocks it: an unlocked layer has no such property at all.
pub fn lock(node: &mut Node, locked: bool) {
    if locked {
        node.extra.insert(LOCKED.to_owned(), serde_json::Value::Bool(true));
    } else {
        node.extra.remove(LOCKED);
    }
}

/// The selected layers as they were when a turn began: each one's id, rotation and top left corner (document
/// coordinates), and the point they turn around.
pub struct RotateStart {
    items: Vec<(String, f32, Point)>,
    /// The point the layers turn around, in document coordinates.
    pub centre: Point,
}

impl RotateStart {
    /// The layers a turn from here changes.
    #[must_use]
    pub fn layers(&self) -> Vec<String> {
        self.items.iter().map(|(id, ..)| id.clone()).collect()
    }
}

/// The document, its laid-out scene and the selection's bounds as they were when a scaling began.
pub struct ScaleStart {
    doc: pen_doc::Document,
    scene: Vec<SceneNode>,
    /// The selection's bounds in document coordinates.
    pub bounds: Rect,
}

/// Scales `node` and everything in it by `factor`. `inner` is set for the layers inside the scaled one, whose
/// positions scale too.
fn scale_node(node: &mut Node, factor: f32, inner: bool) {
    let scale = |value: &mut Option<Num>| {
        if let Some(v) = value.as_ref().and_then(Num::literal) {
            *value = Some(Num::new(v * factor));
        }
    };
    let scale_list = |values: &mut [Num]| {
        for value in values {
            if let Some(v) = value.literal() {
                *value = Num::new(v * factor);
            }
        }
    };
    let scale_json = |value: &mut Value| {
        if let Some(v) = value.as_f64() {
            *value = crate::effects::number(pen_doc::value::narrow(v) * factor);
        }
    };
    if inner && (node.x.is_some() || node.y.is_some()) {
        let (x, y) = stored_position(node);
        set_position(node, x * factor, y * factor);
    }
    for value in [&mut node.width, &mut node.height, &mut node.gap, &mut node.font_size, &mut node.letter_spacing] {
        scale(value);
    }
    match &mut node.stroke_width {
        Some(pen_doc::model::StrokeWidth::All(width)) => scale_list(std::slice::from_mut(width)),
        Some(pen_doc::model::StrokeWidth::Sides(sides)) => sides.values_mut().for_each(scale_json),
        None => {}
    }
    match &mut node.corner_radius {
        Some(pen_doc::model::Corners::All(radius)) => scale_list(std::slice::from_mut(radius)),
        Some(pen_doc::model::Corners::Each(radii)) => scale_list(radii),
        None => {}
    }
    match &mut node.padding {
        Some(Padding::All(padding)) => scale_list(std::slice::from_mut(padding)),
        Some(Padding::Sides(sides)) => scale_list(sides),
        None => {}
    }
    let effects: Vec<&mut Value> = match &mut node.effect {
        Some(Value::Array(list)) => list.iter_mut().collect(),
        Some(one) => vec![one],
        None => Vec::new(),
    };
    for effect in effects {
        for key in ["blur", "radius"] {
            effect.get_mut(key).into_iter().for_each(scale_json);
        }
        if let Some(offset) = effect.get_mut("offset") {
            for key in ["x", "y"] {
                offset.get_mut(key).into_iter().for_each(scale_json);
            }
        }
    }
    for child in node.children.iter_mut().flatten() {
        scale_node(child, factor, true);
    }
}

/// A scene node back as a document node tree (for detaching an instance).
fn rebuild(scene: &SceneNode) -> Node {
    let mut node = scene.node.clone();
    node.reference = None;
    node.descendants = None;
    node.reusable = None;
    if node.kind == Kind::Ref {
        node.kind = Kind::Frame;
    }
    let children: Vec<Node> = scene.children.iter().map(rebuild).collect();
    node.children = (!children.is_empty()).then_some(children);
    node.extra.retain(|k, _| k != "ref");
    node.extra.remove("descendants");
    node
}

/// One layer as "Add flex layout" sees it: its footprint in the parent and whether it is text.
struct Item {
    id: String,
    outer: Rect,
    text: bool,
}

impl Item {
    fn new(id: String, scene: &SceneNode) -> Item {
        // Human: the footprint is the rotated box in the parent.
        let parent_box = scene.local;
        Item { id, outer: parent_box, text: scene.node.kind == Kind::Text }
    }
}

/// What "Add flex layout" decided.
struct FlexPlan {
    horizontal: bool,
    gap: f32,
    padding: [f32; 4],
    align: &'static str,
    justify_center: bool,
    /// Children that span the whole cross size get `fill_container` across, and the frame a fixed cross size.
    stretch: bool,
    stretched: Vec<String>,
    order: Vec<String>,
    /// The new frame's box when wrapping (children's bounds plus padding).
    bounds: Rect,
}

impl FlexPlan {
    fn apply_to_frame(&self, frame: &mut Node) {
        frame.layout = Some(if self.horizontal { "horizontal" } else { "vertical" }.to_owned());
        frame.gap = Some(Num::new(self.gap));
        let [t, r, b, l] = self.padding;
        let same = |a: f32, b: f32| (a - b).abs() < 1e-3;
        frame.padding = if self.padding.iter().all(|p| same(*p, 0.0)) {
            None
        } else if same(t, b) && same(r, l) {
            if same(t, r) { Some(Padding::All(Num::new(t))) } else { Some(Padding::Sides(vec![Num::new(t), Num::new(r)])) }
        } else {
            Some(Padding::Sides(vec![Num::new(t), Num::new(r), Num::new(b), Num::new(l)]))
        };
        frame.justify_content = self.justify_center.then(|| "center".to_owned());
        frame.align_items = (self.align != "start").then(|| self.align.to_owned());
        // Human: the main axis fits the content; across, fixed when children stretch, else fit.
        let fit = Some(Num::Text("fit_content".to_owned()));
        if self.horizontal {
            frame.width.clone_from(&fit);
            if !self.stretch {
                frame.height = fit;
            }
        } else {
            frame.height.clone_from(&fit);
            if !self.stretch {
                frame.width = fit;
            }
        }
    }

    fn apply_to_children(&self, children: &mut [Node]) {
        children.sort_by_key(|n| self.order.iter().position(|id| *id == n.id).unwrap_or(usize::MAX));
        for child in children.iter_mut() {
            if child.layout_position.as_deref() == Some("absolute") {
                child.layout_position = None;
            }
            if self.stretched.contains(&child.id) {
                // Human: stretch across; a text in a row becomes a fixed box, in a column a fixed width.
                let fill = Some(Num::Text("fill_container".to_owned()));
                if self.horizontal {
                    child.height = fill;
                    if child.kind == Kind::Text {
                        child.text_growth = Some("fixed-width-height".to_owned());
                    }
                } else {
                    child.width = fill;
                    if child.kind == Kind::Text && child.text_growth.as_deref() != Some("fixed-width-height") {
                        child.text_growth = Some("fixed-width".to_owned());
                    }
                }
            }
        }
    }
}

/// Reads a flex layout from how the layers lie. `frame` is the frame's own box when it gets
/// the layout itself (padding then comes from the distances to its edges).
// Human: Direction: if the layers overlap along one axis only, they stack along the other; otherwise along the
// axis their centres spread more. Gap: the median space between neighbours (10 for one layer). Alignment across:
// start, centre or end, whichever puts the layers closest. A lone text is centred both ways.
fn infer_flex(items: &[Item], frame: Option<Rect>) -> FlexPlan {
    let union = items.iter().map(|i| i.outer).reduce(|a, b| a.union(&b)).unwrap_or(Rect::ZERO);
    let padding_from = |frame: Option<Rect>, single: bool| match frame {
        Some(f) => [(union.y - f.y).max(0.0), (f.right() - union.right()).max(0.0), (f.bottom() - union.bottom()).max(0.0), (union.x - f.x).max(0.0)],
        None if single => [FLEX_DEFAULT_SPACING; 4],
        None => [0.0; 4],
    };
    let grow = |p: [f32; 4]| Rect::new(union.x - p[3], union.y - p[0], union.w + p[1] + p[3], union.h + p[0] + p[2]);
    if let [only] = items
        && only.text
    {
        let p = match frame {
            Some(f) => {
                let v = (union.y - f.y).max(0.0).min((f.bottom() - union.bottom()).max(0.0));
                let h = (union.x - f.x).max(0.0).min((f.right() - union.right()).max(0.0));
                [v, h, v, h]
            }
            None => [FLEX_DEFAULT_SPACING; 4],
        };
        return FlexPlan {
            horizontal: false,
            gap: FLEX_DEFAULT_SPACING,
            padding: p,
            align: "center",
            justify_center: true,
            stretch: false,
            stretched: Vec::new(),
            order: vec![only.id.clone()],
            bounds: grow(p),
        };
    }
    let sum_w: f32 = items.iter().map(|i| i.outer.w).sum();
    let sum_h: f32 = items.iter().map(|i| i.outer.h).sum();
    let overlap_x = sum_w > union.w + FLEX_TOLERANCE;
    let overlap_y = sum_h > union.h + FLEX_TOLERANCE;
    let spread = |f: fn(&Rect) -> f32| {
        let values = items.iter().map(|i| f(&i.outer));
        values.clone().fold(f32::MIN, f32::max) - values.fold(f32::MAX, f32::min)
    };
    let horizontal = match (overlap_x, overlap_y) {
        (false, true) => true,
        (true, false) => false,
        _ => spread(|r| r.center().x) > spread(|r| r.center().y),
    };
    // Main and cross ranges per item.
    let range = |r: &Rect| if horizontal { ((r.x, r.right()), (r.y, r.bottom())) } else { ((r.y, r.bottom()), (r.x, r.right())) };
    let mut sorted: Vec<&Item> = items.iter().collect();
    sorted.sort_by(|a, b| range(&a.outer).0.0.total_cmp(&range(&b.outer).0.0));
    let (cross_min, cross_max) = if horizontal { (union.y, union.bottom()) } else { (union.x, union.right()) };
    let cross_span = cross_max - cross_min;
    let spans_all = |i: &Item| {
        let (_, (lo, hi)) = range(&i.outer);
        ((hi - lo) - cross_span).abs() < FLEX_TOLERANCE
    };
    let stretch = items.iter().filter(|i| spans_all(i)).count() >= 2;
    let mut gaps: Vec<f32> = sorted.windows(2).map(|w| (range(&w[1].outer).0.0 - range(&w[0].outer).0.1).max(0.0)).collect();
    gaps.sort_by(f32::total_cmp);
    let gap = match gaps.len() {
        0 => FLEX_DEFAULT_SPACING,
        n if n % 2 == 1 => gaps[n / 2],
        n => f32::midpoint(gaps[n / 2 - 1], gaps[n / 2]),
    };
    let align = if items.len() < 2 {
        "start"
    } else {
        let mid = f32::midpoint(cross_min, cross_max);
        let (mut start, mut centre, mut end) = (0.0, 0.0, 0.0);
        for i in items {
            let (_, (lo, hi)) = range(&i.outer);
            start += (lo - cross_min).abs();
            centre += (f32::midpoint(lo, hi) - mid).abs();
            end += (cross_max - hi).abs();
        }
        if start <= centre && start <= end {
            "start"
        } else if centre <= end {
            "center"
        } else {
            "end"
        }
    };
    let padding = padding_from(frame, items.len() == 1);
    FlexPlan {
        horizontal,
        gap,
        padding,
        align,
        justify_center: false,
        stretch,
        stretched: if stretch { items.iter().filter(|i| spans_all(i)).map(|i| i.id.clone()).collect() } else { Vec::new() },
        order: sorted.iter().map(|i| i.id.clone()).collect(),
        bounds: grow(padding),
    }
}

/// One more than the highest "Frame N" name in the document.
#[must_use]
pub fn next_frame_number(nodes: &[Node]) -> u32 {
    fn highest(nodes: &[Node], max: &mut u32) {
        for node in nodes {
            if node.kind == Kind::Frame
                && let Some(n) = node.name.as_deref().and_then(|name| name.strip_prefix("Frame ")).and_then(|n| n.parse::<u32>().ok())
            {
                *max = (*max).max(n);
            }
            highest(node.kids(), max);
        }
    }
    let mut max = 0;
    highest(nodes, &mut max);
    max + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, x: f32, y: f32, w: f32, h: f32) -> Item {
        Item { id: id.to_owned(), outer: Rect::new(x, y, w, h), text: false }
    }

    #[test]
    fn three_layers_are_spread_at_equal_gaps_between_the_two_outermost() {
        let doc: pen_doc::Document = serde_json::from_str(
            r#"{"version":"2.22","children":[
                {"type":"rectangle","id":"c","x":300,"y":90,"width":100,"height":10},
                {"type":"rectangle","id":"a","x":0,"y":0,"width":40,"height":10},
                {"type":"rectangle","id":"b","x":50,"y":20,"width":60,"height":30}]}"#,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        let place = |editor: &Editor, id: &str| edit::find(&editor.doc, id).map(stored_position).unwrap();
        // Two layers have no gap to make equal.
        editor.selection = vec!["a".to_owned(), "c".to_owned()];
        editor.distribute(true);
        assert_eq!((place(&editor, "a"), place(&editor, "c")), ((0.0, 0.0), (300.0, 90.0)));
        // Across: 400 wide, 200 of it taken, two gaps of 100. "b" goes to 140; the outer two and every y stay.
        editor.selection = vec!["c".to_owned(), "a".to_owned(), "b".to_owned()];
        editor.distribute(true);
        assert_eq!((place(&editor, "a"), place(&editor, "b"), place(&editor, "c")), ((0.0, 0.0), (140.0, 20.0), (300.0, 90.0)));
        // Down: 100 high, 50 taken, two gaps of 25. "b" goes to 35. One step back undoes it.
        editor.refresh(&|_| (0.0, 0.0));
        editor.distribute(false);
        assert_eq!(place(&editor, "b"), (140.0, 35.0));
        editor.undo();
        assert_eq!(place(&editor, "b"), (140.0, 20.0));
    }

    #[test]
    fn flex_guess_reads_direction_median_gap_and_alignment() {
        // Three boxes side by side, gaps 10, 20 and 30: a row, median gap 20, bottoms aligned → end.
        let items =
            [item("a", 0.0, 10.0, 10.0, 10.0), item("b", 20.0, 0.0, 10.0, 20.0), item("c", 50.0, 5.0, 10.0, 15.0), item("d", 90.0, 15.0, 10.0, 5.0)];
        let plan = infer_flex(&items, None);
        assert!(plan.horizontal);
        assert!((plan.gap - 20.0).abs() < 1e-4);
        assert_eq!(plan.align, "end");
        assert_eq!(plan.order, vec!["a", "b", "c", "d"]);
        assert_eq!(plan.padding, [0.0; 4]);
    }

    #[test]
    fn flex_guess_stacks_overlapping_columns_and_stretches_full_width_items() {
        // Two full-width bars stacked: a column; both span the cross size, so they stretch.
        let items = [item("a", 0.0, 0.0, 100.0, 10.0), item("b", 0.0, 30.0, 100.0, 10.0)];
        let plan = infer_flex(&items, Some(Rect::new(-8.0, -4.0, 116.0, 50.0)));
        assert!(!plan.horizontal);
        assert!(plan.stretch);
        assert_eq!(plan.stretched.len(), 2);
        assert_eq!(plan.padding, [4.0, 8.0, 6.0, 8.0]);
    }

    #[test]
    fn a_lone_text_is_centred_with_default_padding() {
        let plan = infer_flex(&[Item { id: "t".into(), outer: Rect::new(0.0, 0.0, 40.0, 12.0), text: true }], None);
        assert!(plan.justify_center && plan.align == "center" && !plan.horizontal);
        assert_eq!(plan.padding, [FLEX_DEFAULT_SPACING; 4]);
    }

    #[test]
    fn frame_numbers_continue_from_the_highest() {
        let doc = pen_doc::read(r#"{"version":"2.22","children":[{"type":"frame","id":"a","name":"Frame 7","children":[{"type":"frame","id":"b","name":"Frame 2"}]},{"type":"frame","id":"c","name":"Hero"}]}"#)
            .unwrap();
        assert_eq!(next_frame_number(&doc.children), 8);
        assert_eq!(next_frame_number(&[]), 1);
    }

    /// A board at (100, 100) holding a box at (10, 20), and a second, empty board at (600, 100).
    fn with_component() -> Editor {
        let doc: pen_doc::Document = serde_json::from_str(
            r##"{"version":"2.22","children":[
              {"type":"frame","id":"btn","reusable":true,"x":0,"y":0,"width":100,"height":40,"layout":"none","fill":"#000000","children":[
                {"type":"text","id":"label","content":"OK","fill":"#ffffff"},
                {"type":"frame","id":"hole","x":60,"y":10,"width":20,"height":20,"layout":"none"}]},
              {"type":"ref","id":"one","ref":"btn","x":200,"y":0}]}"##,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        editor
    }

    #[test]
    fn editing_inside_an_instance_writes_an_override_and_leaves_the_component() {
        let mut editor = with_component();
        assert_eq!(
            (editor.role("btn"), editor.role("one"), editor.role("one/label"), editor.role("label")),
            (Role::Component, Role::Instance, Role::Inside, Role::Plain)
        );
        assert!(editor.change("one/label", &|n| n.content = Some("Cancel".to_owned())));
        assert!(editor.change("one", &|n| n.opacity = Some(Num::new(0.5))));
        // A change that changes nothing writes nothing.
        assert!(editor.change("one/hole", &|_| {}));
        let one = serde_json::to_value(edit::find(&editor.doc, "one").unwrap()).unwrap();
        assert_eq!(
            one,
            serde_json::json!({"type":"ref","id":"one","x":200,"y":0,"opacity":0.5,"ref":"btn","descendants":{"label":{"content":"Cancel"}}})
        );
        assert_eq!(edit::find(&editor.doc, "label").unwrap().content.as_deref(), Some("OK"));
        editor.touched();
        editor.refresh(&|_| (0.0, 0.0));
        assert_eq!(editor.scene_node("one/label").unwrap().node.content.as_deref(), Some("Cancel"));
    }

    #[test]
    fn a_copy_of_a_component_is_an_instance_of_it() {
        let mut editor = with_component();
        editor.selection = vec!["btn".to_owned()];
        editor.duplicate();
        let copy = edit::find(&editor.doc, &editor.selection[0]).unwrap();
        assert_eq!((copy.kind.clone(), copy.reference.as_deref(), copy.reusable, copy.children.is_none()), (Kind::Ref, Some("btn"), None, true));
        // Pasting the copied component gives an instance too; making components skips instances and their insides.
        let copied = {
            editor.selection = vec!["btn".to_owned()];
            editor.copy_selection()
        };
        editor.paste(&copied, Rect::new(0.0, 0.0, 500.0, 500.0));
        assert_eq!(edit::find(&editor.doc, &editor.selection[0]).unwrap().reference.as_deref(), Some("btn"));
        editor.selection = vec!["one".to_owned(), "one/label".to_owned()];
        editor.create_component();
        assert_eq!(edit::find(&editor.doc, "one").unwrap().reusable, None);
    }

    #[test]
    fn deleting_inside_an_instance_hides_and_an_instance_can_be_inserted() {
        let mut editor = with_component();
        editor.selection = vec!["one/hole".to_owned()];
        editor.delete_selection();
        let one = edit::find(&editor.doc, "one").unwrap();
        assert_eq!(serde_json::to_value(one.descendants.as_ref().unwrap()).unwrap(), serde_json::json!({"hole":{"enabled":false}}));
        assert!(edit::find(&editor.doc, "hole").is_some());
        editor.insert_instance("btn");
        let new = edit::find(&editor.doc, &editor.selection[0]).unwrap();
        assert_eq!((new.kind.clone(), new.reference.as_deref(), stored_position(new)), (Kind::Ref, Some("btn"), (-50.0, -20.0)));
        editor.selection = vec!["one/label".to_owned()];
        editor.go_to_component();
        assert_eq!((editor.selection.clone(), editor.take_focus()), (vec!["btn".to_owned()], Some(Rect::new(0.0, 0.0, 100.0, 40.0))));
    }

    #[test]
    fn an_instance_fills_a_slot_removes_from_it_and_gives_it_back() {
        let mut editor = with_component();
        // A second component to put into the first one's slot.
        let chip: Node =
            serde_json::from_str(r##"{"type":"rectangle","id":"chip","reusable":true,"x":0,"y":80,"width":10,"height":6,"fill":"#ff0000"}"##)
                .unwrap();
        edit::insert(&mut editor.doc, None, usize::MAX, chip);
        editor.touched();
        editor.refresh(&|_| (0.0, 0.0));
        editor.fill_slot_with("one/hole", "chip");
        let [first] = editor.selection.clone().try_into().unwrap();
        // Pasting with the slot selected adds to it; the copy of a plain layer stays a plain layer.
        editor.selection = vec!["one/hole".to_owned()];
        let label = vec![edit::find(&editor.doc, "label").unwrap().clone()];
        editor.paste(&label, Rect::new(0.0, 0.0, 500.0, 500.0));
        let [second] = editor.selection.clone().try_into().unwrap();
        let children = edit::override_children(&editor.doc, "one", "hole").unwrap();
        assert_eq!(children.len(), 2);
        // The instance of the 10 × 6 chip is centred in the 20 × 20 slot.
        assert_eq!(
            (&children[0]["type"], &children[0]["ref"], &children[0]["x"], &children[0]["y"]),
            (&serde_json::json!("ref"), &serde_json::json!("chip"), &serde_json::json!(5), &serde_json::json!(7))
        );
        assert_eq!(children[1]["content"], serde_json::json!("OK"));
        editor.refresh(&|_| (0.0, 0.0));
        assert_eq!((editor.role(&first), editor.scene_node(&first).map(|s| s.node.kind.clone())), (Role::Inside, Some(Kind::Rectangle)));
        assert!(editor.scene_node(&second).is_some() && first.starts_with("one/") && second.starts_with("one/"));
        // The component's own slot stays empty.
        assert_eq!(edit::find(&editor.doc, "hole").unwrap().kids().len(), 0);
        // Deleting a filled layer takes it out; deleting a layer of the component only hides it.
        editor.selection = vec![first];
        editor.delete_selection();
        assert_eq!(edit::override_children(&editor.doc, "one", "hole").map(|c| c.len()), Some(1));
        editor.clear_slot("one/hole");
        assert_eq!((edit::override_children(&editor.doc, "one", "hole"), editor.selection.clone()), (None, vec!["one/hole".to_owned()]));
    }

    #[test]
    fn scaling_scales_a_layer_with_everything_in_it_around_the_anchor() {
        let doc: pen_doc::Document = serde_json::from_str(
            r##"{"version":"2.22","children":[
              {"type":"frame","id":"card","x":100,"y":100,"width":200,"height":100,"layout":"none","cornerRadius":8,"padding":[4,8],"gap":6,
               "stroke":"#000000","strokeWidth":{"top":1,"left":2},"effect":{"type":"shadow","blur":4,"offset":{"x":0,"y":2}},"children":[
                {"type":"text","id":"title","x":10,"y":20,"content":"Hi","fontSize":16,"letterSpacing":0.5,"width":"fill_container"},
                {"type":"rectangle","id":"bar","x":10,"y":60,"width":"$w","height":10}]}]}"##,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        editor.selection = vec!["card".to_owned()];
        // Twice the size, away from the card's bottom right corner (300, 200).
        let start = editor.scale_start().unwrap();
        editor.scale_from(&start, 2.0, Point::new(300.0, 200.0));
        let card = serde_json::to_value(edit::find(&editor.doc, "card").unwrap()).unwrap();
        assert_eq!(
            (&card["x"], &card["y"], &card["width"], &card["height"]),
            (&serde_json::json!(-100), &serde_json::json!(0), &serde_json::json!(400), &serde_json::json!(200))
        );
        assert_eq!(
            (&card["cornerRadius"], &card["padding"], &card["gap"]),
            (&serde_json::json!(16), &serde_json::json!([8, 16]), &serde_json::json!(12))
        );
        assert_eq!(card["strokeWidth"], serde_json::json!({"top":2,"left":4}));
        assert_eq!(card["effect"], serde_json::json!({"type":"shadow","blur":8,"offset":{"x":0,"y":4}}));
        let title = &card["children"][0];
        assert_eq!(
            (&title["x"], &title["y"], &title["fontSize"], &title["letterSpacing"]),
            (&serde_json::json!(20), &serde_json::json!(40), &serde_json::json!(32), &serde_json::json!(1))
        );
        // A size that fills, and one bound to a variable, stay as they are.
        assert_eq!(
            (&title["width"], &card["children"][1]["width"], &card["children"][1]["height"]),
            (&serde_json::json!("fill_container"), &serde_json::json!("$w"), &serde_json::json!(20))
        );
        // Scaling again starts from the same beginning, and the factor is kept in bounds.
        editor.scale_from(&start, 1000.0, Point::new(300.0, 200.0));
        assert_eq!(edit::find(&editor.doc, "card").unwrap().width.as_ref().and_then(Num::literal), Some(200.0 * SCALE_MAX));
        // The number field scales around the middle and remembers how far it has gone.
        editor.scale_from(&start, 1.0, Point::new(0.0, 0.0));
        editor.touched();
        editor.refresh(&|_| (0.0, 0.0));
        editor.scale_selection_to(0.5);
        assert_eq!((stored_position(edit::find(&editor.doc, "card").unwrap()), editor.scaled.0), ((150.0, 125.0), 0.5));
    }

    #[test]
    fn a_picture_file_comes_in_as_a_rectangle_filled_with_it() {
        let folder = std::env::temp_dir().join(format!("pencraft-import-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        image::RgbaImage::from_pixel(40, 20, image::Rgba([255, 0, 0, 255])).save(folder.join("red.png")).unwrap();
        std::fs::write(
            folder.join("ring.svg"),
            r##"<svg viewBox="0 0 24 24" fill="none" stroke="#ff0000" stroke-width="2"><circle cx="12" cy="12" r="10"/></svg>"##,
        )
        .unwrap();
        let mut editor = with_component();
        editor.path = Some(folder.join("doc.pen"));
        let visible = Rect::new(0.0, 0.0, 400.0, 300.0);
        editor.import_file(&folder.join("red.png"), visible).unwrap();
        let picture = serde_json::to_value(edit::find(&editor.doc, &editor.selection[0]).unwrap()).unwrap();
        assert_eq!(
            (&picture["type"], &picture["name"], &picture["x"], &picture["y"]),
            (&serde_json::json!("rectangle"), &serde_json::json!("red"), &serde_json::json!(180), &serde_json::json!(140))
        );
        assert_eq!(
            (&picture["width"], &picture["height"], &picture["fill"]),
            (&serde_json::json!(40), &serde_json::json!(20), &serde_json::json!({"type":"image","url":"./red.png"}))
        );
        editor.import_file(&folder.join("ring.svg"), visible).unwrap();
        let ring = edit::find(&editor.doc, &editor.selection[0]).unwrap();
        assert_eq!(
            (ring.kind.clone(), ring.view_box, ring.fill.is_none(), ring.stroke.is_some()),
            (Kind::Path, Some([0.0, 0.0, 24.0, 24.0]), true, true)
        );
        assert!(ring.geometry.as_deref().is_some_and(|d| d.starts_with("M22 12 C")));
        assert!(editor.import_file(&folder.join("missing.png"), visible).is_err());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn turning_a_layer_keeps_its_middle_where_it_is() {
        let mut editor = two_boards();
        let middle = |editor: &Editor, id: &str| editor.scene_node(id).map(layout::doc_bounds).unwrap().center();
        // A layer inside a frame, turned a quarter turn around its own middle.
        editor.selection = vec!["box".to_owned()];
        let before = middle(&editor, "box");
        editor.rotate_selection(90.0);
        editor.refresh(&|_| (0.0, 0.0));
        let after = middle(&editor, "box");
        assert!((after.x - before.x).abs() < 1e-3 && (after.y - before.y).abs() < 1e-3, "{after:?}");
        assert_eq!(edit::find(&editor.doc, "box").unwrap().rotation.as_ref().and_then(Num::literal), Some(90.0));
        // The 40 × 30 box now stands 30 wide and 40 high.
        let turned = editor.scene_node("box").map(layout::doc_bounds).unwrap();
        assert!((turned.w - 30.0).abs() < 1e-3 && (turned.h - 40.0).abs() < 1e-3);
        // Turning back from a start goes back exactly, and angles stay within a half turn.
        let start = editor.rotate_start().unwrap();
        editor.rotate_from(&start, 150.0);
        assert_eq!(edit::find(&editor.doc, "box").unwrap().rotation.as_ref().and_then(Num::literal), Some(-120.0));
        editor.rotate_from(&start, -90.0);
        editor.touched();
        assert_eq!((edit::find(&editor.doc, "box").unwrap().rotation.clone(), position(&editor, "box")), (None, (10.0, 20.0)));
        // Two boards turn together around the middle of both.
        editor.selection = vec!["one".to_owned(), "two".to_owned()];
        let all = editor.selection_bounds().unwrap().center();
        editor.rotate_selection(180.0);
        editor.touched();
        editor.refresh(&|_| (0.0, 0.0));
        let swapped = editor.selection_bounds().unwrap().center();
        assert!((swapped.x - all.x).abs() < 1e-2 && (swapped.y - all.y).abs() < 1e-2);
        assert!(middle(&editor, "one").x > middle(&editor, "two").x);
    }

    fn two_boards() -> Editor {
        let doc = pen_doc::read(
            r#"{"version":"2.22","children":[
            {"type":"frame","id":"one","x":100,"y":100,"width":300,"height":200,"layout":"none","children":[
              {"type":"rectangle","id":"box","x":10,"y":20,"width":40,"height":30}]},
            {"type":"frame","id":"two","x":600,"y":100,"width":300,"height":200,"layout":"none"}]}"#,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        editor
    }

    fn position(editor: &Editor, id: &str) -> (f32, f32) {
        stored_position(edit::find(&editor.doc, id).unwrap())
    }

    #[test]
    fn pasted_svg_text_is_a_path_and_a_pasted_picture_a_rectangle_filled_with_it() {
        let mut editor = two_boards();
        let visible = Rect::new(0.0, 0.0, 1000.0, 400.0);
        // SVG text becomes one path layer, selected; one undo step takes it away again.
        let layers = |editor: &Editor| pen_doc::write(&editor.doc).unwrap().matches("\"id\"").count();
        let before = layers(&editor);
        assert!(editor.paste_svg(r#"<?xml version="1.0"?><svg viewBox="0 0 24 24"><path d="M2 2 L22 22"/></svg>"#, visible));
        let [pasted] = editor.selection.clone().try_into().unwrap();
        let node = edit::find(&editor.doc, &pasted).unwrap();
        assert_eq!((&node.kind, node.name.as_deref()), (&Kind::Path, Some("Pasted SVG")));
        editor.undo();
        assert_eq!(layers(&editor), before);
        // Words that only mention SVG, and an SVG without a shape, are left to be pasted as text.
        assert!(!editor.paste_svg("Export it as <svg> please", visible));
        assert!(!editor.paste_svg("<svg viewBox=\"0 0 24 24\"></svg>", visible));
        // A picture 4000 by 1000 is shrunk to fit four fifths of the view (800 by 200) and named.
        editor.selection.clear();
        editor.paste_picture("./images/pasted-picture-0001.png", (4000, 1000), visible);
        let [pasted] = editor.selection.clone().try_into().unwrap();
        let node = edit::find(&editor.doc, &pasted).unwrap();
        let size = (node.width.as_ref().and_then(Num::literal), node.height.as_ref().and_then(Num::literal));
        assert_eq!((&node.kind, node.name.as_deref(), size), (&Kind::Rectangle, Some("Pasted picture"), (Some(800.0), Some(200.0))));
        assert!(pen_doc::write(&editor.doc).unwrap().contains("./images/pasted-picture-0001.png"));
    }

    #[test]
    fn a_locked_layer_stays_where_it_is_and_a_dropped_colour_goes_to_the_selection() {
        let mut editor = two_boards();
        editor.selection = vec!["box".to_owned()];
        let was = position(&editor, "box");
        editor.toggle_lock();
        editor.refresh(&|_| (0.0, 0.0));
        assert!(editor.locked("box") && !editor.locked("one"));
        // The arrow keys and aligning leave it alone.
        editor.nudge(5.0, 0.0);
        editor.align(AlignTo::Right);
        assert_eq!(position(&editor, "box"), was);
        // Locking the board it is on locks it too, also once it is unlocked itself; an unlocked layer says nothing.
        editor.toggle_lock();
        editor.refresh(&|_| (0.0, 0.0));
        assert!(!editor.locked("box") && !pen_doc::write(&editor.doc).unwrap().contains("locked"));
        editor.selection = vec!["one".to_owned()];
        editor.toggle_lock();
        editor.refresh(&|_| (0.0, 0.0));
        assert!(editor.locked("box") && editor.locked("one"));
        editor.toggle_lock();
        // A colour dropped on the selection is its fill, or its stroke when asked; a solid one is written short.
        editor.selection = vec!["box".to_owned()];
        editor.drop_color(pen_doc::color::Rgba { r: 255, g: 0, b: 16, a: 255 }, false);
        editor.drop_color(pen_doc::color::Rgba { r: 0, g: 0, b: 0, a: 128 }, true);
        let written = pen_doc::write(&editor.doc).unwrap();
        assert!(written.contains("\"fill\": \"#ff0010\"") && written.contains("\"stroke\": \"#00000080\""), "{written}");
    }

    #[test]
    fn paste_lands_on_its_original_when_that_is_on_screen() {
        let mut editor = two_boards();
        editor.selection = vec!["box".to_owned()];
        let copied = editor.copy_selection();
        editor.paste(&copied, Rect::new(0.0, 0.0, 1000.0, 400.0));
        let [pasted] = editor.selection.as_slice() else { panic!("one copy expected") };
        assert_ne!(pasted, "box");
        assert_eq!(edit::slot(&editor.doc, pasted).unwrap().parent.as_deref(), Some("one"));
        assert_eq!(position(&editor, pasted), (10.0, 20.0));
    }

    #[test]
    fn paste_goes_centred_into_a_selected_container() {
        let mut editor = two_boards();
        editor.selection = vec!["box".to_owned()];
        let copied = editor.copy_selection();
        editor.selection = vec!["two".to_owned()];
        editor.paste(&copied, Rect::new(0.0, 0.0, 1000.0, 400.0));
        let [pasted] = editor.selection.as_slice() else { panic!("one copy expected") };
        assert_eq!(edit::slot(&editor.doc, pasted).unwrap().parent.as_deref(), Some("two"));
        // Centre of the 300 × 200 board minus half the 40 × 30 box.
        assert_eq!(position(&editor, pasted), (130.0, 85.0));
    }

    #[test]
    fn paste_far_from_its_original_goes_to_the_middle_of_the_view() {
        let mut editor = two_boards();
        editor.selection = vec!["box".to_owned()];
        let copied = editor.copy_selection();
        editor.selection.clear();
        // Agent: the view shows empty canvas only, so the copy is centred on (2100, 2050) at the top level.
        editor.paste(&copied, Rect::new(2000.0, 2000.0, 200.0, 100.0));
        let [pasted] = editor.selection.as_slice() else { panic!("one copy expected") };
        assert_eq!(edit::slot(&editor.doc, pasted).unwrap().parent, None);
        assert_eq!(position(&editor, pasted), (2080.0, 2035.0));
        // Agent: with the second board in the middle of the view, the copy goes into it (it wholly holds it).
        editor.selection.clear();
        editor.paste(&copied, Rect::new(2000.0, 2000.0, 200.0, 100.0).translate(-1350.0, -1850.0));
        let [pasted] = editor.selection.as_slice() else { panic!("one copy expected") };
        assert_eq!(edit::slot(&editor.doc, pasted).unwrap().parent.as_deref(), Some("two"));
        assert_eq!(position(&editor, pasted), (130.0, 85.0));
    }
}
