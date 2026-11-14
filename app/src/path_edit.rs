//! Editing the points of a finished path: which path and points, what the pointer does to them, how they show.
// Human: a double click (or Enter) on
// a path shows its points; they are selected, dragged, bent by their handles, removed, added with ⌥ on a line,
// made smooth or sharp with a double click, and cut open with X. Enter or Escape leaves.

use egui::{Color32, CursorIcon, Pos2, Rect as ScreenRect, Stroke};
use pen_doc::edit;
use pen_doc::geom::{Affine, Point, Rect};
use pen_doc::model::Kind;
use pen_doc::path;
use pen_doc::value::Num;

use crate::editor::{Editor, Tool};
use crate::ops::{is_free, set_position, stored_position};
use crate::pen::{self, Contour, Side, Spot};
use crate::theme::{ACCENT, PEN_ANCHOR, PEN_HANDLE, PEN_REACH, PEN_STRETCH_REACH};

/// The path whose points are being edited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathEdit {
    /// The path layer's id.
    pub id: String,
    /// The selected points.
    pub selected: Vec<Spot>,
    /// Whether the next click cuts the path open (X) instead of selecting.
    pub cutting: bool,
}

/// What the pointer holds while it drags in a path being edited.
#[derive(Clone, Debug)]
pub enum Grab {
    /// Points being moved: where the drag began (document), the path as it was, the points.
    Points { start: Point, original: Vec<Contour>, spots: Vec<Spot> },
    /// One handle being pulled: the path as it was, the point, which handle, and whether the other one turns
    /// with it.
    Handle { original: Vec<Contour>, spot: Spot, side: Side, mirrored: bool },
}

/// What of the path is under the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Hit {
    Handle(Spot, Side),
    Anchor(Spot),
    /// A line of the path: outline, line, how far along it.
    Stretch(usize, usize, f32),
}

/// The path being edited as it is now: its outlines, the transform from their coordinates to the document's,
/// and the box and scale that transform was made from.
struct Shape {
    contours: Vec<Contour>,
    to_doc: Affine,
    view: Rect,
    scale: (f32, f32),
}

impl Editor {
    /// Whether the layer with `id` is a path whose points can be edited: a path of the document itself (not one
    /// inside an instance) with readable path data.
    #[must_use]
    pub fn can_edit_path(&self, id: &str) -> bool {
        edit::find(&self.doc, id)
            .is_some_and(|n| n.kind == Kind::Path && n.geometry.as_deref().is_some_and(|g| path::parse(g).is_ok_and(|s| !s.is_empty())))
    }

    /// Starts editing the points of the path with `id`.
    pub fn edit_path(&mut self, id: String) {
        if self.can_edit_path(&id) {
            self.selection = vec![id.clone()];
            self.tool = Tool::Move;
            self.path_edit = Some(PathEdit { id, selected: Vec::new(), cutting: false });
        }
    }

    /// For a picture of the app (`--points`): with `on`, the selected path's points are edited, all selected.
    pub fn pose_points(&mut self, on: bool) {
        if let (true, [id]) = (on, self.selection.as_slice()) {
            self.edit_path(id.clone());
            self.select_all_path_points();
        }
    }

    /// Ends the editing when its path is gone, no longer the selection, or another tool was taken; forgets
    /// selected points that are no longer there (after an undo). Called every frame.
    pub fn keep_path_edit(&mut self) {
        let Some(editing) = &self.path_edit else { return };
        let still = self.tool == Tool::Move && self.selection.as_slice() == [editing.id.clone()] && self.can_edit_path(&editing.id);
        let counts: Option<Vec<usize>> = self.shape().map(|s| s.contours.iter().map(|c| c.anchors.len()).collect());
        match (&mut self.path_edit, counts) {
            (Some(editing), Some(counts)) if still => editing.selected.retain(|&(c, a)| counts.get(c).is_some_and(|&count| a < count)),
            // Not laid out yet: nothing to compare the selected points with.
            (Some(_), None) if still => {}
            _ => self.path_edit = None,
        }
    }

    /// One step out of path editing (Escape): first the selected points and the cutting are let go, then the
    /// editing itself; `false` when no path is being edited.
    pub fn leave_path_edit(&mut self) -> bool {
        match &mut self.path_edit {
            Some(editing) if editing.cutting || !editing.selected.is_empty() => {
                editing.cutting = false;
                editing.selected.clear();
            }
            Some(_) => self.path_edit = None,
            None => return false,
        }
        true
    }

    /// Removes the selected points (Delete); `true` while a path is being edited, whether or not any were.
    pub fn delete_path_points(&mut self) -> bool {
        let Some(editing) = &self.path_edit else { return false };
        let spots = editing.selected.clone();
        if let Some(mut shape) = self.shape().filter(|_| !spots.is_empty()) {
            self.checkpoint();
            pen::remove(&mut shape.contours, &spots);
            self.write_path(&shape.contours);
            if let Some(editing) = &mut self.path_edit {
                editing.selected.clear();
            }
        }
        true
    }

    /// Moves the selected points by a step in the document (arrow keys); `true` while a path is being edited.
    pub fn nudge_path_points(&mut self, dx: f32, dy: f32) -> bool {
        let Some(editing) = &self.path_edit else { return false };
        let spots = editing.selected.clone();
        if let Some(mut shape) = self.shape().filter(|_| !spots.is_empty())
            && let Some(inverse) = shape.to_doc.invert()
        {
            self.checkpoint();
            let (gx, gy) = linear(&inverse, dx, dy);
            pen::shift(&mut shape.contours, &spots, gx, gy);
            self.write_path(&shape.contours);
        }
        true
    }

    /// Selects every point of the path (⌘A); `true` while a path is being edited.
    pub fn select_all_path_points(&mut self) -> bool {
        // Agent: read from the document alone, so it also works before the first layout (the `--points` picture).
        let data = self.path_edit.as_ref().and_then(|e| edit::find(&self.doc, &e.id)).and_then(|n| n.geometry.as_deref());
        let contours = data.and_then(|d| path::parse(d).ok()).map(|s| pen::contours(&s)).unwrap_or_default();
        let all: Vec<Spot> = contours.iter().enumerate().flat_map(|(c, contour)| (0..contour.anchors.len()).map(move |a| (c, a))).collect();
        match &mut self.path_edit {
            Some(editing) => editing.selected = all,
            None => return false,
        }
        true
    }

    /// The path being edited, read from the document as it is now.
    fn shape(&self) -> Option<Shape> {
        let id = &self.path_edit.as_ref()?.id;
        let (node, scene) = (edit::find(&self.doc, id)?, self.scene_node(id)?);
        let segments = path::parse(node.geometry.as_deref()?).ok()?;
        let narrow = pen_doc::value::narrow;
        let view = node.view_box.map(|v| Rect::new(narrow(v[0]), narrow(v[1]), narrow(v[2]), narrow(v[3]))).or_else(|| path::bounds(&segments))?;
        // Agent: the same rule the renderer fits a path with: a box without width or height is not stretched.
        let along = |size: f32, of: f32| if of > 1e-6 { size / of } else { 1.0 };
        let scale = (along(scene.local.w, view.w), along(scene.local.h, view.h));
        let to_own = Affine::scale(scale.0, scale.1).then(&Affine::translate(-view.x, -view.y));
        Some(Shape { contours: pen::contours(&segments), to_doc: scene.to_doc.then(&to_own), view, scale })
    }

    /// Writes the edited outlines to the path layer; its box follows the outline unless the path has a view box
    /// of its own. With no outline left the layer goes. No undo step is made here.
    fn write_path(&mut self, contours: &[Contour]) {
        let Some(shape) = self.shape() else { return };
        let Some(id) = self.path_edit.as_ref().map(|e| e.id.clone()) else { return };
        let data = pen::data(contours);
        let Some(bounds) = path::parse(&data).ok().and_then(|s| path::bounds(&s)) else {
            edit::remove(&mut self.doc, &id);
            self.selection.clear();
            self.path_edit = None;
            self.touched();
            return;
        };
        // Human: the layer's corner moves along the layer's own sides; its stored place is in the parent's.
        let parent = edit::slot(&self.doc, &id).and_then(|s| s.parent).and_then(|p| self.scene_node(&p)).map_or(Affine::IDENTITY, |p| p.to_doc);
        let own = self.scene_node(&id).map_or(Affine::IDENTITY, |s| s.to_doc);
        let from_own = parent.invert().unwrap_or(Affine::IDENTITY).then(&own);
        let free = is_free(self, &id);
        let Some(node) = edit::find_mut(&mut self.doc, &id) else { return };
        node.geometry = Some(data);
        if node.view_box.is_none() {
            let (sx, sy) = shape.scale;
            node.width = Some(Num::new(bounds.w * sx));
            node.height = Some(Num::new(bounds.h * sy));
            if free {
                let (px, py) = linear(&from_own, (bounds.x - shape.view.x) * sx, (bounds.y - shape.view.y) * sy);
                let (x, y) = stored_position(node);
                set_position(node, x + px, y + py);
            }
        }
        self.touched();
    }
}

/// A vector through the turning and scaling of `t`, without its move.
#[expect(clippy::many_single_char_names, reason = "a, b, c, d are the matrix entries' standard names")]
fn linear(t: &Affine, dx: f32, dy: f32) -> (f32, f32) {
    let [a, b, c, d, _, _] = t.m;
    (a * dx + c * dy, b * dx + d * dy)
}

/// The transform from the path's coordinates to the screen's.
fn to_screen(shape: &Shape, editor: &Editor, view: ScreenRect) -> Affine {
    let (zoom, centre, at) = (editor.camera.zoom, view.center(), editor.camera.center);
    Affine::translate(centre.x - at.x * zoom, centre.y - at.y * zoom).then(&Affine::scale(zoom, zoom)).then(&shape.to_doc)
}

fn pos(p: Point) -> Pos2 {
    Pos2::new(p.x, p.y)
}

/// The points whose handles show: the selected ones and the ones next to them along the path.
fn shown(contours: &[Contour], selected: &[Spot]) -> Vec<Spot> {
    let mut all = Vec::new();
    for &(c, a) in selected {
        let Some(contour) = contours.get(c) else { continue };
        let count = contour.anchors.len();
        let previous = if a > 0 { Some(a - 1) } else { (contour.closed && count > 1).then(|| count - 1) };
        let next = if a + 1 < count { Some(a + 1) } else { (contour.closed && count > 1).then_some(0) };
        for spot in [Some(a), previous, next].into_iter().flatten().map(|a| (c, a)) {
            if !all.contains(&spot) {
                all.push(spot);
            }
        }
    }
    all
}

/// What of the path is at the screen point `p`: a showing handle first, then a point, then a line.
fn hit(contours: &[Contour], selected: &[Spot], through: &Affine, p: Pos2) -> Option<Hit> {
    let near = |q: Point| pos(through.apply(q)).distance(p) <= PEN_REACH;
    for (c, a) in shown(contours, selected) {
        let anchor = contours[c].anchors[a];
        for (handle, side) in [(anchor.before, Side::Before), (anchor.after, Side::After)] {
            if handle.is_some_and(near) {
                return Some(Hit::Handle((c, a), side));
            }
        }
    }
    for (c, contour) in contours.iter().enumerate() {
        if let Some(a) = contour.anchors.iter().position(|anchor| near(anchor.at)) {
            return Some(Hit::Anchor((c, a)));
        }
    }
    let (c, index, t, far) = pen::nearest_stretch(contours, through, Point::new(p.x, p.y))?;
    (far <= PEN_STRETCH_REACH).then_some(Hit::Stretch(c, index, t))
}

/// The two ends of line `index` of outline `c`.
fn ends(contours: &[Contour], c: usize, index: usize) -> Vec<Spot> {
    let count = contours.get(c).map_or(1, |contour| contour.anchors.len().max(1));
    vec![(c, index), (c, (index + 1) % count)]
}

/// Handles the pointer while a path's points are edited. `grab` is what a drag holds, kept by the canvas;
/// `snap` puts moved points on whole pixels of the document.
// Agent: MAIN-THREAD; WRITES editor.doc (through checkpoint/write_path) and editor.path_edit.
#[expect(clippy::too_many_lines, reason = "the pointer state machine reads best as one match, as the canvas's does")]
pub fn pointer(ui: &egui::Ui, editor: &mut Editor, view: ScreenRect, response: &egui::Response, grab: &mut Option<Grab>, snap: bool) {
    let (Some(shape), Some(editing)) = (editor.shape(), editor.path_edit.clone()) else { return };
    let Some(to_path) = shape.to_doc.invert() else { return };
    let through = to_screen(&shape, editor, view);
    let modifiers = ui.input(|i| i.modifiers);
    let pointer = response.interact_pointer_pos().or_else(|| ui.input(|i| i.pointer.hover_pos()));
    let in_doc = |p: Pos2| crate::render::doc_point(&editor.camera, view, p);
    let whole = |p: Point| if snap { Point::new(p.x.round(), p.y.round()) } else { p };
    let under = |p: Pos2| hit(&shape.contours, &editing.selected, &through, p);
    if editing.cutting {
        ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
    } else if grab.is_none() && pointer.and_then(under).is_some_and(|h| !matches!(h, Hit::Stretch(..))) {
        ui.ctx().set_cursor_icon(CursorIcon::Move);
    }
    let mut selected = editing.selected.clone();
    let mut cutting = editing.cutting;
    if response.double_clicked() {
        match pointer.and_then(under) {
            // Human: a double click on a point makes it smooth, or sharp again.
            Some(Hit::Anchor(spot)) => {
                let mut contours = shape.contours.clone();
                pen::toggle_smooth(&mut contours, spot);
                editor.checkpoint();
                editor.write_path(&contours);
                selected = vec![spot];
            }
            Some(_) => {}
            // A double click beside the path ends the editing.
            None => {
                editor.path_edit = None;
                return;
            }
        }
    } else if response.drag_started() {
        let origin = ui.input(|i| i.pointer.press_origin()).or(pointer);
        let moving = |spots: Vec<Spot>| Grab::Points { start: origin.map(in_doc).unwrap_or_default(), original: shape.contours.clone(), spots };
        match origin.and_then(under).filter(|_| !cutting) {
            Some(Hit::Handle(spot, side)) => {
                // Human: a smooth point's other handle turns along; ⌥ bends this one alone.
                let mirrored = pen::is_smooth(&shape.contours[spot.0].anchors[spot.1]) && !modifiers.alt;
                *grab = Some(Grab::Handle { original: shape.contours.clone(), spot, side, mirrored });
            }
            Some(Hit::Anchor(spot)) => {
                if !selected.contains(&spot) {
                    if !modifiers.shift {
                        selected.clear();
                    }
                    selected.push(spot);
                }
                *grab = Some(moving(selected.clone()));
            }
            Some(Hit::Stretch(c, index, _)) => {
                selected = ends(&shape.contours, c, index);
                *grab = Some(moving(selected.clone()));
            }
            None => {
                if !modifiers.shift {
                    selected.clear();
                }
            }
        }
        if grab.is_some() {
            editor.checkpoint();
        }
    } else if response.dragged()
        && let Some(p) = pointer
    {
        match grab.as_ref() {
            Some(Grab::Points { start, original, spots }) => {
                let moved = whole(Point::new(in_doc(p).x - start.x, in_doc(p).y - start.y));
                let (dx, dy) = linear(&to_path, moved.x, moved.y);
                let mut contours = original.clone();
                pen::shift(&mut contours, spots, dx, dy);
                editor.write_path(&contours);
            }
            Some(Grab::Handle { original, spot, side, mirrored }) => {
                let mut contours = original.clone();
                pen::bend(&mut contours, *spot, *side, to_path.apply(whole(in_doc(p))), *mirrored);
                editor.write_path(&contours);
            }
            None => {}
        }
    } else if response.clicked()
        && let Some(p) = pointer
    {
        let mut contours = shape.contours.clone();
        match under(p) {
            // Human: with the cut (X) a click on a point or a line opens the path there, and the cut is done
            // (the pointer then moves points again).
            Some(at @ (Hit::Anchor(_) | Hit::Stretch(..))) if cutting => {
                let spot = match at {
                    Hit::Stretch(c, index, t) => pen::insert(&mut contours, c, index, t),
                    Hit::Anchor(spot) => Some(spot),
                    Hit::Handle(..) => None,
                };
                if let Some(spot) = spot
                    && pen::cut(&mut contours, spot)
                {
                    editor.checkpoint();
                    editor.write_path(&contours);
                    selected.clear();
                    cutting = false;
                }
            }
            Some(Hit::Anchor(spot)) => {
                if !modifiers.shift {
                    selected = vec![spot];
                } else if let Some(index) = selected.iter().position(|s| *s == spot) {
                    selected.remove(index);
                } else {
                    selected.push(spot);
                }
            }
            // Human: ⌥ and a click on a line adds a point there; a plain click selects
            // the line's two ends.
            Some(Hit::Stretch(c, index, t)) if modifiers.alt => {
                if let Some(spot) = pen::insert(&mut contours, c, index, t) {
                    editor.checkpoint();
                    editor.write_path(&contours);
                    selected = vec![spot];
                }
            }
            Some(Hit::Stretch(c, index, _)) => {
                if !modifiers.shift {
                    selected.clear();
                }
                for spot in ends(&shape.contours, c, index) {
                    if !selected.contains(&spot) {
                        selected.push(spot);
                    }
                }
            }
            Some(Hit::Handle(..)) => {}
            None => {
                if !modifiers.shift {
                    selected.clear();
                }
            }
        }
    }
    if response.drag_stopped() {
        *grab = None;
    }
    if let Some(editing) = &mut editor.path_edit {
        (editing.selected, editing.cutting) = (selected, cutting);
    }
}

/// Draws the path being edited: its outline, the handles of the selected points and their neighbours, and
/// every point's dot (filled when selected, larger under the pointer).
pub fn overlay(ui: &egui::Ui, painter: &egui::Painter, editor: &Editor, view: ScreenRect, dragging: bool) {
    let (Some(shape), Some(editing)) = (editor.shape(), editor.path_edit.as_ref()) else { return };
    let through = to_screen(&shape, editor, view);
    let place = |p: Point| pos(through.apply(p));
    let line = Stroke::new(1.0, ACCENT);
    for contour in &shape.contours {
        let count = contour.anchors.len();
        let lines = if contour.closed { count } else { count.saturating_sub(1) };
        for index in 0..lines {
            let (from, to) = (contour.anchors[index], contour.anchors[(index + 1) % count]);
            stretch_line(painter, place(from.at), from.after.map(place), to.before.map(place), place(to.at), line);
        }
    }
    let hover = if dragging { None } else { ui.input(|i| i.pointer.hover_pos()).filter(|p| view.contains(*p)) };
    let under = hover.and_then(|p| hit(&shape.contours, &editing.selected, &through, p));
    for (c, a) in shown(&shape.contours, &editing.selected) {
        let anchor = shape.contours[c].anchors[a];
        for (handle, side) in [(anchor.before, Side::Before), (anchor.after, Side::After)] {
            let Some(handle) = handle.map(place) else { continue };
            painter.line_segment([place(anchor.at), handle], line);
            let half = if under == Some(Hit::Handle((c, a), side)) { PEN_HANDLE / 2.0 + 1.5 } else { PEN_HANDLE / 2.0 };
            let corners = [egui::vec2(0.0, -half), egui::vec2(half, 0.0), egui::vec2(0.0, half), egui::vec2(-half, 0.0)];
            painter.add(egui::Shape::convex_polygon(corners.map(|v| handle + v).to_vec(), Color32::WHITE, line));
        }
    }
    for (c, contour) in shape.contours.iter().enumerate() {
        for (a, anchor) in contour.anchors.iter().enumerate() {
            let radius = if under == Some(Hit::Anchor((c, a))) { PEN_ANCHOR / 2.0 + 1.5 } else { PEN_ANCHOR / 2.0 };
            let (fill, edge) = if editing.selected.contains(&(c, a)) { (ACCENT, Color32::WHITE) } else { (Color32::WHITE, ACCENT) };
            painter.circle(place(anchor.at), radius, fill, Stroke::new(1.0, edge));
        }
    }
}

/// One line of a path on screen: straight without handles, else a cubic curve.
pub fn stretch_line(painter: &egui::Painter, from: Pos2, after: Option<Pos2>, before: Option<Pos2>, to: Pos2, stroke: Stroke) {
    if after.is_none() && before.is_none() {
        painter.line_segment([from, to], stroke);
    } else {
        let points = [from, after.unwrap_or(from), before.unwrap_or(to), to];
        painter.add(egui::epaint::CubicBezierShape::from_points_stroke(points, false, Color32::TRANSPARENT, stroke));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editing(doc: &str, id: &str) -> Editor {
        let mut editor = Editor::new(serde_json::from_str(doc).unwrap(), None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        editor.edit_path(id.to_owned());
        editor
    }

    const SQUARE: &str = r##"{"version":"2.22","children":[{"type":"frame","id":"f","x":100,"y":200,"width":400,"height":300,"layout":"none","children":[
        {"type":"path","id":"p","x":20,"y":30,"width":100,"height":50,"geometry":"M0 0 L100 0 L100 50 L0 50 Z","stroke":"#000000","strokeWidth":1},
        {"type":"rectangle","id":"r","x":0,"y":0,"width":10,"height":10}]}]}"##;

    fn path_of(editor: &Editor) -> (String, (f32, f32), Option<Num>, Option<Num>) {
        let node = edit::find(&editor.doc, "p").unwrap();
        (node.geometry.clone().unwrap_or_default(), stored_position(node), node.width.clone(), node.height.clone())
    }

    #[test]
    fn only_paths_of_the_document_are_edited_and_the_editing_ends_with_the_selection() {
        let mut editor = editing(SQUARE, "r");
        assert_eq!(editor.path_edit, None);
        editor.edit_path("p".to_owned());
        assert!(editor.path_edit.is_some() && editor.selection == ["p"]);
        // Escape lets the selected points go first, then the editing.
        assert!(editor.select_all_path_points());
        assert_eq!(editor.path_edit.as_ref().map(|e| e.selected.len()), Some(4));
        assert!(editor.leave_path_edit() && editor.path_edit.as_ref().is_some_and(|e| e.selected.is_empty()));
        assert!(editor.leave_path_edit() && editor.path_edit.is_none() && !editor.leave_path_edit());
        // Another selection or tool ends it too.
        editor.edit_path("p".to_owned());
        editor.selection = vec!["r".to_owned()];
        editor.keep_path_edit();
        assert_eq!(editor.path_edit, None);
    }

    #[test]
    fn moved_points_keep_their_place_and_the_box_follows_the_outline() {
        let mut editor = editing(SQUARE, "p");
        // The two left points go 10 to the left and 5 up: the box grows that way and its corner moves along.
        if let Some(e) = &mut editor.path_edit {
            e.selected = vec![(0, 0), (0, 3)];
        }
        assert!(editor.nudge_path_points(-10.0, -5.0));
        assert_eq!(path_of(&editor), ("M-10 -5 L100 0 L100 50 L-10 45 Z".to_owned(), (10.0, 25.0), Some(Num::new(110.0)), Some(Num::new(55.0))));
        // One undo puts it back; the points that are still there stay selected.
        editor.undo();
        editor.refresh(&|_| (0.0, 0.0));
        editor.keep_path_edit();
        assert_eq!(path_of(&editor).0, "M0 0 L100 0 L100 50 L0 50 Z");
        // Delete takes the selected points out; the layer is never deleted while its points are edited.
        assert!(editor.delete_path_points());
        editor.refresh(&|_| (0.0, 0.0));
        assert_eq!(path_of(&editor), ("M100 0 L100 50".to_owned(), (120.0, 30.0), Some(Num::new(0.0)), Some(Num::new(50.0))));
        // Without its last points the layer goes, and the editing with it.
        assert!(editor.select_all_path_points() && editor.delete_path_points());
        assert!(edit::find(&editor.doc, "p").is_none() && editor.path_edit.is_none() && editor.selection.is_empty());
    }

    #[test]
    fn a_stretched_path_is_edited_in_its_own_coordinates() {
        // An icon-sized outline shown four times as large: a step of 8 in the document is 2 in the path data.
        let doc = r#"{"version":"2.22","children":[{"type":"path","id":"p","x":0,"y":0,"width":96,"height":96,"geometry":"M0 0 L24 0 L24 24 Z"}]}"#;
        let mut editor = editing(doc, "p");
        if let Some(e) = &mut editor.path_edit {
            e.selected = vec![(0, 1)];
        }
        editor.nudge_path_points(8.0, 0.0);
        let node = edit::find(&editor.doc, "p").unwrap();
        assert_eq!((node.geometry.as_deref(), node.width.clone()), (Some("M0 0 L26 0 L24 24 Z"), Some(Num::new(104.0))));
    }

    #[test]
    fn what_is_under_the_pointer_is_a_handle_then_a_point_then_a_line() {
        let contours = pen::contours(&path::parse("M0 0 C0 40 60 40 100 0 L100 100").unwrap());
        let same = Affine::IDENTITY;
        assert_eq!(hit(&contours, &[], &same, Pos2::new(2.0, 3.0)), Some(Hit::Anchor((0, 0))));
        // A handle is only there to be hit while its point, or the one next to it, is selected.
        assert_eq!(hit(&contours, &[], &same, Pos2::new(0.0, 40.0)), None);
        assert_eq!(hit(&contours, &[(0, 1)], &same, Pos2::new(0.0, 40.0)), Some(Hit::Handle((0, 0), Side::After)));
        assert!(matches!(hit(&contours, &[], &same, Pos2::new(104.0, 50.0)), Some(Hit::Stretch(0, 1, t)) if (t - 0.5).abs() < 0.03));
        assert_eq!(hit(&contours, &[], &same, Pos2::new(160.0, 50.0)), None);
    }
}
