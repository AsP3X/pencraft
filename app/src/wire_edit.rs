//! Connectors on the canvas: drawing one with the connector tool, and the handles of a selected one.
//!
//! A connector has no box to pull at. Selected, it shows a round handle at each end (drag it onto another
//! layer, onto one of that layer's sides, or off into the open), a square one at each waypoint (drag to move, a
//! double click takes it out) and a small dot in the middle of each stretch (drag it to add a waypoint there).

use egui::{CursorIcon, Pos2, Rect as ScreenRect, Stroke};
use pen_doc::connector::{LINE, POINTS};
use pen_doc::edit;
use pen_doc::geom::{Affine, Point, Rect};
use pen_doc::layout;
use pen_doc::model::{Kind, Node, Paint, Paints, StrokeWidth};
use pen_doc::scene::SceneNode;
use pen_doc::value::{Num, narrow};
use pen_doc::vars::Vars;
use serde_json::{Value, json};

use crate::editor::{Editor, Tool};
use crate::render::{doc_point, screen};
use crate::theme::{ACCENT, WIRE_END, WIRE_MID, WIRE_POINT, WIRE_REACH};

/// Stroke of a connector drawn with the tool when the document has no connector to take the look from.
const WIRE_STROKE: &str = "#8B93A1";
/// See [`WIRE_STROKE`].
const WIRE_WIDTH: f32 = 2.0;
/// The part in the middle of a layer, as a share of each side, where a dropped end picks no side of its own.
const MIDDLE: f32 = 0.34;
/// The look a new connector takes from one the document already has, so that all its wires match.
const LOOK: [&str; 10] =
    ["route", "curvature", "stub", "dash", "strokeLinecap", "strokeLinejoin", "startMarker", "endMarker", "markerSize", "cornerRadius"];

/// Which handle of a connector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spot {
    /// Its first end (`from`) or its last (`to`).
    End(bool),
    /// The waypoint at this place in `via`.
    Via(usize),
    /// The middle of the stretch before the waypoint at this place: a waypoint dropped there lands at it.
    Middle(usize),
}

/// The handle of the selected connector a drag holds.
pub struct Grab {
    id: String,
    spot: Spot,
}

/// A connector being drawn with the tool: where it starts.
pub struct Draft {
    from: Point,
}

impl Draft {
    /// A connector starting where the button went down, `at` in the document.
    #[must_use]
    pub fn start(at: Point) -> Draft {
        Draft { from: at }
    }

    /// Makes the connector from the start to `to`, selects it, and goes back to the Move tool.
    // Human: each end holds on to the layer under it, or stays a free point where there is none. The connector
    // goes into the frame both layers share, right under the lower of the two, so it runs beneath them.
    // Agent: WRITES editor.doc (one checkpoint), editor.selection and editor.tool.
    pub fn finish(self, editor: &mut Editor, to: Point, snap: bool) {
        let (first, last) = (target(editor, self.from), target(editor, to));
        if first.is_some() && first.as_ref().map(|(id, _)| id) == last.as_ref().map(|(id, _)| id) {
            return;
        }
        let ids: Vec<&str> = [&first, &last].into_iter().flatten().map(|(id, _)| id.as_str()).collect();
        let (parent, index) = home(editor, &ids, self.from);
        let to_parent = parent.as_deref().and_then(|p| editor.scene_node(p)).and_then(|p| p.to_doc.invert()).unwrap_or(Affine::IDENTITY);
        let id = editor.fresh_id();
        let mut node = Node::new(Kind::Connector, id.clone());
        node.stroke = Some(Paints::One(Paint::Solid(WIRE_STROKE.to_owned())));
        node.stroke_width = Some(StrokeWidth::All(Num::new(WIRE_WIDTH)));
        if let Some(like) = first_connector(&editor.doc.children) {
            node.stroke.clone_from(&like.stroke);
            node.stroke_width.clone_from(&like.stroke_width);
            node.corner_radius.clone_from(&like.corner_radius);
            for key in LOOK {
                if let Some(value) = like.extra.get(key) {
                    node.extra.insert(key.to_owned(), value.clone());
                }
            }
        }
        node.extra.insert("from".to_owned(), end_value(first.as_ref(), self.from, &to_parent, snap));
        node.extra.insert("to".to_owned(), end_value(last.as_ref(), to, &to_parent, snap));
        editor.checkpoint();
        edit::insert(&mut editor.doc, parent.as_deref(), index, node);
        editor.touched();
        editor.selection = vec![id];
        editor.tool = Tool::Move;
    }
}

/// The first connector in the document, if it has one.
fn first_connector(nodes: &[Node]) -> Option<&Node> {
    nodes.iter().find_map(|node| if node.kind == Kind::Connector { Some(node) } else { first_connector(node.kids()) })
}

/// Where a new connector between the layers `ids` goes: the frame they share (`None` for the top level; for no
/// layers at all, the frame under `at`) and the place in it right under the lower of them.
fn home(editor: &Editor, ids: &[&str], at: Point) -> (Option<String>, usize) {
    let lines: Vec<Vec<String>> = ids.iter().map(|id| edit::ancestry(&editor.doc, id)).collect();
    let Some(first) = lines.first() else { return (crate::canvas::container_at(editor, at), usize::MAX) };
    // What is above each layer, and shared by all of them.
    let above = |line: &Vec<String>| line.len().saturating_sub(1);
    let mut shared = above(first);
    for line in &lines {
        shared = shared.min(above(line)).min(line.iter().zip(first).take_while(|(a, b)| a == b).count());
    }
    let parent = shared.checked_sub(1).and_then(|last| first.get(last)).cloned();
    let index = lines.iter().filter_map(|line| line.get(shared)).filter_map(|branch| edit::slot(&editor.doc, branch)).map(|slot| slot.index).min();
    (parent, index.unwrap_or(usize::MAX))
}

/// The layer an end dropped at `p` holds on to: the deepest document layer there, with its box.
// Human: a board with layers on it is no layer to connect to: its empty ground leaves the end free.
fn target(editor: &Editor, p: Point) -> Option<(String, Rect)> {
    let chain = crate::canvas::hit_chain(&editor.scene, p, Vars::new(&editor.doc), 0.0);
    let (depth, id) = chain.iter().enumerate().rev().find(|(_, id)| edit::find(&editor.doc, id).is_some())?;
    let scene = editor.scene_node(id)?;
    if depth == 0 && scene.node.kind == Kind::Frame && !scene.children.is_empty() {
        return None;
    }
    Some((id.clone(), layout::doc_bounds(scene)))
}

/// A number as the file takes it: two decimals at most.
fn number(value: f32) -> Value {
    json!((f64::from(value) * 100.0).round() / 100.0)
}

/// The side of `rect` nearest to `p`, or `None` in the middle of it, where the connector picks the side itself.
fn nearest_side(rect: &Rect, p: Point) -> Option<&'static str> {
    let (u, v) = ((p.x - rect.x) / rect.w.max(1e-3), (p.y - rect.y) / rect.h.max(1e-3));
    let middle = (0.5 - MIDDLE / 2.0)..=(0.5 + MIDDLE / 2.0);
    if middle.contains(&u) && middle.contains(&v) {
        return None;
    }
    let sides = [("left", p.x - rect.x), ("right", rect.right() - p.x), ("top", p.y - rect.y), ("bottom", rect.bottom() - p.y)];
    sides.into_iter().min_by(|a, b| a.1.total_cmp(&b.1)).map(|(side, _)| side)
}

/// An end as the file writes it: the layer `on` with the side nearest `at`, or the free point `at` in the
/// coordinates of the connector's parent.
fn end_value(on: Option<&(String, Rect)>, at: Point, to_parent: &Affine, snap: bool) -> Value {
    match on {
        Some((id, rect)) => match nearest_side(rect, at) {
            Some(side) => json!({ "id": id, "side": side }),
            None => json!(id),
        },
        None => point_value(at, to_parent, snap),
    }
}

/// A free point as the file writes it, in the coordinates of the connector's parent.
fn point_value(at: Point, to_parent: &Affine, snap: bool) -> Value {
    let p = to_parent.apply(at);
    let whole = |v: f32| if snap { v.round() } else { v };
    json!({ "x": number(whole(p.x)), "y": number(whole(p.y)) })
}

/// The points a connector runs through, in the document: its first end, its waypoints, its last end.
fn stops(node: &SceneNode) -> Vec<Point> {
    let read = |point: &Value| Some(Point::new(narrow(point.get(0)?.as_f64()?), narrow(point.get(1)?.as_f64()?)));
    node.node.extra.get(POINTS).and_then(Value::as_array).into_iter().flatten().filter_map(read).collect()
}

/// A connector's line in the document as straight pieces, whatever its dashes.
#[must_use]
pub fn line(node: &SceneNode) -> Vec<Point> {
    let data = node.node.extra.get(LINE).and_then(Value::as_str).unwrap_or_default();
    pen_doc::path::parse(data).map(|segments| pen_doc::path::flatten(&segments, 24)).unwrap_or_default().into_iter().next().unwrap_or_default()
}

/// How far `p` is from the piece of line between `a` and `b`.
fn distance(p: Point, a: Point, b: Point) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx * dx + dy * dy;
    let t = if length > 1e-9 { (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0.0, 1.0) } else { 0.0 };
    (p.x - (a.x + dx * t)).hypot(p.y - (a.y + dy * t))
}

/// Whether `p` is within `reach` of a connector's line.
#[must_use]
pub fn near(node: &SceneNode, p: Point, reach: f32) -> bool {
    let line = line(node);
    line.windows(2).any(|pair| distance(p, pair[0], pair[1]) <= reach)
}

/// The middle of each stretch of `line` between one of `stops` and the next, measured along the line.
fn middles(line: &[Point], stops: &[Point]) -> Vec<Point> {
    // Where on the line each stop is: the nearest corner of the line, never before the stop before it.
    let mut at = 0usize;
    let places: Vec<usize> = stops
        .iter()
        .map(|stop| {
            let nearest = line
                .iter()
                .enumerate()
                .skip(at)
                .min_by(|a, b| (a.1.x - stop.x).hypot(a.1.y - stop.y).total_cmp(&(b.1.x - stop.x).hypot(b.1.y - stop.y)));
            at = nearest.map_or(at, |(index, _)| index);
            at
        })
        .collect();
    places
        .windows(2)
        .filter_map(|pair| {
            let piece = line.get(pair[0]..=pair[1])?;
            let lengths: Vec<f32> = piece.windows(2).map(|ends| (ends[1].x - ends[0].x).hypot(ends[1].y - ends[0].y)).collect();
            let mut left = lengths.iter().sum::<f32>() / 2.0;
            for (ends, length) in piece.windows(2).zip(&lengths) {
                if left <= *length {
                    let t = if *length > 1e-6 { left / length } else { 0.0 };
                    return Some(Point::new(ends[0].x + (ends[1].x - ends[0].x) * t, ends[0].y + (ends[1].y - ends[0].y) * t));
                }
                left -= length;
            }
            piece.first().copied()
        })
        .collect()
}

/// The connector that is selected alone and can be edited: a document layer, not one inside an instance.
fn selected(editor: &Editor) -> Option<(&str, &SceneNode)> {
    let [id] = editor.selection.as_slice() else { return None };
    edit::find(&editor.doc, id).filter(|node| node.kind == Kind::Connector)?;
    Some((id.as_str(), editor.scene_node(id)?))
}

/// The waypoints of the connector `id` as the file has them.
fn via_of(editor: &Editor, id: &str) -> Vec<Value> {
    edit::find(&editor.doc, id).and_then(|node| node.extra.get("via")).and_then(Value::as_array).cloned().unwrap_or_default()
}

/// The handles of a connector on screen, the ones to try first at the front: ends, waypoints, middles.
// Human: a waypoint that is a layer has no handle here; it goes where its layer goes.
fn handles(editor: &Editor, view: ScreenRect, id: &str, node: &SceneNode) -> Vec<(Spot, Pos2)> {
    let stops = stops(node);
    let Some((first, last)) = stops.first().zip(stops.last()) else { return Vec::new() };
    let place = |p: Point| screen(&editor.camera, view, p);
    let mut out = vec![(Spot::End(true), place(*first)), (Spot::End(false), place(*last))];
    let via = via_of(editor, id);
    // A waypoint whose layer is gone is left out of the line; the handles then wouldn't match the list.
    if via.len() + 2 != stops.len() {
        return out;
    }
    for (index, (entry, stop)) in via.iter().zip(&stops[1..]).enumerate() {
        if entry.is_object() && entry.get("id").is_none() {
            out.push((Spot::Via(index), place(*stop)));
        }
    }
    out.extend(middles(&line(node), &stops).into_iter().enumerate().map(|(index, middle)| (Spot::Middle(index), place(middle))));
    out
}

/// The handle under `p`, if any.
fn spot_at(editor: &Editor, view: ScreenRect, p: Pos2) -> Option<(Spot, Pos2)> {
    let (id, node) = selected(editor)?;
    handles(editor, view, id, node).into_iter().find(|(_, at)| at.distance(p) <= WIRE_REACH)
}

/// Sets the waypoints of the connector `id`; an empty list takes the key away.
fn set_via(editor: &mut Editor, id: &str, via: &[Value]) {
    editor.change(id, &|node| {
        if via.is_empty() {
            node.extra.remove("via");
        } else {
            node.extra.insert("via".to_owned(), Value::Array(via.to_vec()));
        }
    });
    editor.touched();
}

/// Handles the pointer on the handles of the selected connector. `grab` is what a drag holds, kept by the
/// canvas. Returns whether the pointer was the connector's this frame; if not, the canvas goes on as usual.
// Agent: MAIN-THREAD; WRITES editor.doc (through checkpoint/change) only while a handle is held or double clicked.
pub fn pointer(ui: &egui::Ui, editor: &mut Editor, view: ScreenRect, response: &egui::Response, grab: &mut Option<Grab>, snap: bool) -> bool {
    let Some(id) = selected(editor).map(|(id, _)| id.to_owned()) else {
        *grab = None;
        return false;
    };
    let pointer = response.interact_pointer_pos().or_else(|| ui.input(|i| i.pointer.hover_pos()));
    let to_parent = crate::canvas::parent_transform(editor, &id).invert().unwrap_or(Affine::IDENTITY);
    if grab.is_none() && pointer.is_some_and(|p| spot_at(editor, view, p).is_some()) {
        ui.ctx().set_cursor_icon(CursorIcon::Move);
    }
    if response.double_clicked() {
        // Human: a double click on a waypoint takes it out.
        let Some((Spot::Via(index), _)) = pointer.and_then(|p| spot_at(editor, view, p)) else { return false };
        let mut via = via_of(editor, &id);
        if index < via.len() {
            via.remove(index);
            editor.checkpoint();
            set_via(editor, &id, &via);
        }
        return true;
    }
    if response.drag_started() {
        let origin = ui.input(|i| i.pointer.press_origin()).or(pointer);
        let Some((spot, at)) = origin.and_then(|p| spot_at(editor, view, p)) else { return false };
        editor.checkpoint();
        let spot = match spot {
            // Human: pulling the dot in the middle of a stretch puts a waypoint there, which the drag then holds.
            Spot::Middle(index) => {
                let mut via = via_of(editor, &id);
                let index = index.min(via.len());
                via.insert(index, point_value(doc_point(&editor.camera, view, at), &to_parent, snap));
                set_via(editor, &id, &via);
                Spot::Via(index)
            }
            other => other,
        };
        *grab = Some(Grab { id: id.clone(), spot });
    }
    let Some(held) = grab.as_ref().filter(|held| held.id == id) else {
        *grab = None;
        return false;
    };
    if response.dragged()
        && let Some(p) = pointer
    {
        let at = doc_point(&editor.camera, view, p);
        match held.spot {
            Spot::End(first) => {
                let value = end_value(target(editor, at).as_ref(), at, &to_parent, snap);
                let key = if first { "from" } else { "to" };
                editor.change(&id, &|node| {
                    node.extra.insert(key.to_owned(), value.clone());
                });
                editor.touched();
            }
            Spot::Via(index) => {
                let mut via = via_of(editor, &id);
                if let Some(entry) = via.get_mut(index) {
                    *entry = point_value(at, &to_parent, snap);
                    set_via(editor, &id, &via);
                }
            }
            Spot::Middle(_) => {}
        }
    }
    if response.drag_stopped() {
        *grab = None;
    }
    true
}

/// Draws the selected connector's handles, and a connector being drawn with the tool.
pub fn overlay(ui: &egui::Ui, painter: &egui::Painter, editor: &Editor, view: ScreenRect, grab: Option<&Grab>, draft: Option<&Draft>) {
    let edge = Stroke::new(1.5, ACCENT);
    let hover = ui.input(|i| i.pointer.hover_pos()).filter(|p| view.contains(*p));
    let frame = |rect: Rect| {
        let (min, max) =
            (screen(&editor.camera, view, Point::new(rect.x, rect.y)), screen(&editor.camera, view, Point::new(rect.right(), rect.bottom())));
        painter.rect_stroke(ScreenRect::from_min_max(min, max), 0.0, edge, egui::StrokeKind::Outside);
    };
    if let Some(draft) = draft {
        // Human: the line being pulled out, and the layers it would hold on to at both ends.
        for (_, rect) in [Some(draft.from), hover.map(|p| doc_point(&editor.camera, view, p))].into_iter().flatten().filter_map(|p| target(editor, p))
        {
            frame(rect);
        }
        if let Some(p) = hover {
            painter.line_segment([screen(&editor.camera, view, draft.from), p], edge);
        }
    }
    let Some((id, node)) = selected(editor) else { return };
    // An end being dragged shows the layer it would hold on to.
    if let (Some(Grab { spot: Spot::End(_), .. }), Some(p)) = (grab, hover)
        && let Some((_, rect)) = target(editor, doc_point(&editor.camera, view, p))
    {
        frame(rect);
    }
    let under = if grab.is_some() { None } else { hover.and_then(|p| spot_at(editor, view, p)).map(|(spot, _)| spot) };
    let held = grab.map(|grab| grab.spot);
    for (spot, at) in handles(editor, view, id, node).into_iter().rev() {
        let grow = if under == Some(spot) || held == Some(spot) { 1.5 } else { 0.0 };
        match spot {
            Spot::End(_) => {
                painter.circle(at, WIRE_END / 2.0 + grow, crate::theme::HANDLE_FILL, edge);
            }
            Spot::Via(_) => {
                let square = ScreenRect::from_center_size(at, egui::Vec2::splat(WIRE_POINT + 2.0 * grow));
                painter.rect(square, 1.0, crate::theme::HANDLE_FILL, edge, egui::StrokeKind::Inside);
            }
            // The dots between the points are only an offer, and stay out of the way while something is dragged.
            Spot::Middle(_) if grab.is_none() => {
                painter.circle(at, WIRE_MID / 2.0 + grow, ACCENT.gamma_multiply(0.35), Stroke::new(1.0, ACCENT));
            }
            Spot::Middle(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_end_takes_the_nearest_side_of_a_layer_and_none_in_its_middle() {
        let rect = Rect::new(100.0, 100.0, 200.0, 100.0);
        assert_eq!(nearest_side(&rect, Point::new(110.0, 150.0)), Some("left"));
        assert_eq!(nearest_side(&rect, Point::new(290.0, 150.0)), Some("right"));
        assert_eq!(nearest_side(&rect, Point::new(200.0, 105.0)), Some("top"));
        assert_eq!(nearest_side(&rect, Point::new(150.0, 195.0)), Some("bottom"));
        assert_eq!(nearest_side(&rect, Point::new(200.0, 150.0)), None);
        // A free point is written in the parent's coordinates, on whole pixels when snapping is on.
        let to_parent = Affine::translate(-40.0, -10.0);
        assert_eq!(end_value(None, Point::new(100.4, 50.26), &to_parent, true), json!({ "x": 60.0, "y": 40.0 }));
        assert_eq!(end_value(None, Point::new(100.4, 50.26), &to_parent, false), json!({ "x": 60.4, "y": 40.26 }));
        let on = ("node".to_owned(), rect);
        assert_eq!(end_value(Some(&on), Point::new(110.0, 150.0), &to_parent, true), json!({ "id": "node", "side": "left" }));
        assert_eq!(end_value(Some(&on), Point::new(200.0, 150.0), &to_parent, true), json!("node"));
    }

    fn board() -> Editor {
        let doc: pen_doc::Document = serde_json::from_str(
            r##"{"version":"2.22","children":[{"type":"frame","id":"board","x":100,"y":100,"width":800,"height":600,"layout":"none","children":[
            {"type":"rectangle","id":"ground","x":0,"y":0,"width":800,"height":600,"fill":"#111111"},
            {"type":"rectangle","id":"a","x":50,"y":50,"width":100,"height":60,"fill":"#333333"},
            {"type":"rectangle","id":"b","x":400,"y":300,"width":100,"height":60,"fill":"#333333"},
            {"type":"connector","id":"old","from":"a","to":"b","stroke":"#ff0000","strokeWidth":3,"route":"elbow","endMarker":"arrow"}]}]}"##,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        editor
    }

    #[test]
    fn the_tool_draws_a_connector_under_its_layers_that_looks_like_the_others() {
        let mut editor = board();
        editor.tool = Tool::Connector;
        // From the right edge of a (150..250 across, 150..210 down on the canvas) to the middle of b.
        Draft::start(Point::new(245.0, 180.0)).finish(&mut editor, Point::new(550.0, 430.0), true);
        let children = editor.doc.children[0].kids();
        // It lies in the board, right under a, the lower of its two layers, and over the ground.
        let made = &children[1];
        assert_eq!(children.iter().map(|n| n.id.as_str()).collect::<Vec<_>>()[..3], ["ground", made.id.as_str(), "a"]);
        assert_eq!(
            (&made.kind, made.extra.get("from"), made.extra.get("to")),
            (&Kind::Connector, Some(&json!({ "id": "a", "side": "right" })), Some(&json!("b")))
        );
        // It takes the look of the connector that was there.
        assert_eq!((made.extra.get("route"), made.extra.get("endMarker")), (Some(&json!("elbow")), Some(&json!("arrow"))));
        assert_eq!(serde_json::to_value(&made.stroke).unwrap(), json!("#ff0000"));
        assert_eq!((editor.selection.clone(), editor.tool), (vec![made.id.clone()], Tool::Move));
        // One undo takes it away; a pull that ends on the layer it began on, or on open ground twice, differs.
        editor.undo();
        assert_eq!(editor.doc.children[0].kids().len(), 4);
        Draft::start(Point::new(160.0, 160.0)).finish(&mut editor, Point::new(240.0, 200.0), true);
        assert_eq!(editor.doc.children[0].kids().len(), 4);
        // An end on the board's ground is a free point, in the board's coordinates.
        editor.doc.children[0].children.as_mut().unwrap().remove(0);
        editor.touched();
        editor.refresh(&|_| (0.0, 0.0));
        Draft::start(Point::new(200.0, 180.0)).finish(&mut editor, Point::new(700.4, 600.0), true);
        let free = editor.doc.children[0].kids().iter().find(|n| n.extra.get("to").is_some_and(Value::is_object) && n.id != "old").unwrap();
        assert_eq!(free.extra.get("to"), Some(&json!({ "x": 600.0, "y": 500.0 })));
    }

    #[test]
    fn a_connector_is_picked_on_its_line_and_shows_a_handle_for_each_end_waypoint_and_stretch() {
        let mut editor = board();
        let old = |editor: &Editor| editor.scene_node("old").unwrap().clone();
        // The elbow runs from a's right side (250, 180) across to 375, down to 430, and across to b (500, 430).
        assert!(near(&old(&editor), Point::new(300.0, 183.0), 4.0) && near(&old(&editor), Point::new(372.0, 300.0), 4.0));
        assert!(!near(&old(&editor), Point::new(300.0, 300.0), 4.0));
        // Under the pointer the line counts only with a reach; without one the layer beneath is found.
        let chain = |editor: &Editor, reach: f32| crate::canvas::hit_chain(&editor.scene, Point::new(300.0, 181.0), Vars::new(&editor.doc), reach);
        assert_eq!((chain(&editor, 4.0).last().cloned(), chain(&editor, 0.0).last().cloned()), (Some("old".to_owned()), Some("ground".to_owned())));
        editor.selection = vec!["old".to_owned()];
        let view = ScreenRect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 800.0));
        let spots = |editor: &Editor| handles(editor, view, "old", &old(editor)).into_iter().map(|(spot, _)| spot).collect::<Vec<_>>();
        assert_eq!(spots(&editor), vec![Spot::End(true), Spot::End(false), Spot::Middle(0)]);
        // A waypoint adds its own handle and one more stretch; one that is a layer has no handle.
        set_via(&mut editor, "old", &[json!({ "x": 300.0, "y": 20.0 }), json!("ground")]);
        editor.refresh(&|_| (0.0, 0.0));
        assert_eq!(spots(&editor), vec![Spot::End(true), Spot::End(false), Spot::Via(0), Spot::Middle(0), Spot::Middle(1), Spot::Middle(2)]);
        set_via(&mut editor, "old", &[]);
        assert!(edit::find(&editor.doc, "old").unwrap().extra.get("via").is_none());
    }

    #[test]
    fn the_middle_of_a_stretch_is_measured_along_the_line() {
        // An L: 100 across, then 60 down, with a waypoint at the corner.
        let line = [Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(100.0, 60.0)];
        let stops = [Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(100.0, 60.0)];
        assert_eq!(middles(&line, &stops), vec![Point::new(50.0, 0.0), Point::new(100.0, 30.0)]);
        // Without the waypoint the one stretch is 160 long, and its middle 80 along: 80 across.
        assert_eq!(middles(&line, &[stops[0], stops[2]]), vec![Point::new(80.0, 0.0)]);
        assert!((distance(Point::new(50.0, 7.0), line[0], line[1]) - 7.0).abs() < 1e-4);
        assert!((distance(Point::new(-3.0, 4.0), line[0], line[1]) - 5.0).abs() < 1e-4);
    }
}
