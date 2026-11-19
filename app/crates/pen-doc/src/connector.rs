//! Connectors: a line between two layers that follows them when they move (a wire in a node editor, an arrow
//! in a flow chart).
//!
//! A `connector` node names its two ends (`from`, `to`) and how the line runs (`route`). Its path is worked out
//! here after layout, when every layer has its place, and written into the scene node as an ordinary path.

use std::collections::HashMap;
use std::fmt::Write as _;

use serde_json::Value;

use crate::geom::{Affine, Point, Rect};
use crate::layout::doc_bounds;
use crate::model::{Corners, Kind};
use crate::path;
use crate::scene::SceneNode;
use crate::value::narrow;
use crate::vars::Vars;

/// The size of an arrow head when the connector doesn't say (`markerSize`), in canvas units.
const MARKER_SIZE: f32 = 8.0;
/// How far a right-angled line runs straight out of a layer before it may turn back.
const STUB: f32 = 16.0;
/// The shortest and longest reach of a curved line's handles.
const HANDLE: (f32, f32) = (24.0, 240.0);

/// The side of a layer a connector leaves from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

/// One end of a connector, in document coordinates.
#[derive(Clone, Copy, Debug)]
enum End {
    /// On a layer: its box, the side (`None` picks the side facing the other end) and how far along that side.
    Layer { rect: Rect, side: Option<Side>, at: f32 },
    /// A free point.
    Free(Point),
}

impl End {
    fn center(&self) -> Point {
        match self {
            End::Layer { rect, .. } => rect.center(),
            End::Free(p) => *p,
        }
    }

    /// The box the line must not run through: the layer's, or nothing but the point itself.
    fn rect(&self) -> Rect {
        match self {
            End::Layer { rect, .. } => *rect,
            End::Free(p) => Rect::new(p.x, p.y, 0.0, 0.0),
        }
    }

    /// Where the line meets this end and the direction it leaves in, given where the `other` end is.
    fn anchor(&self, other: Point) -> (Point, Point) {
        let own = self.center();
        let (dx, dy) = (other.x - own.x, other.y - own.y);
        // The direction of the larger distance: a line between layers side by side leaves sideways.
        let facing = if dx.abs() >= dy.abs() {
            Point::new(if dx >= 0.0 { 1.0 } else { -1.0 }, 0.0)
        } else {
            Point::new(0.0, if dy >= 0.0 { 1.0 } else { -1.0 })
        };
        let (rect, side, at) = match *self {
            End::Free(p) => return (p, facing),
            End::Layer { rect, side, at } => (rect, side, at),
        };
        let side = side.unwrap_or(match (facing.x, facing.y) {
            (x, _) if x > 0.0 => Side::Right,
            (x, _) if x < 0.0 => Side::Left,
            (_, y) if y > 0.0 => Side::Bottom,
            _ => Side::Top,
        });
        match side {
            Side::Left => (Point::new(rect.x, rect.y + rect.h * at), Point::new(-1.0, 0.0)),
            Side::Right => (Point::new(rect.right(), rect.y + rect.h * at), Point::new(1.0, 0.0)),
            Side::Top => (Point::new(rect.x + rect.w * at, rect.y), Point::new(0.0, -1.0)),
            Side::Bottom => (Point::new(rect.x + rect.w * at, rect.bottom()), Point::new(0.0, 1.0)),
            Side::Center => (own, facing),
        }
    }
}

/// How the line runs between its ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    Straight,
    Curved,
    Elbow,
}

/// Works out the path of every connector in a laid-out scene.
// Human: called at the end of layout. A connector whose end names a layer that isn't there draws nothing.
// Agent: WRITES `geometry`, `view_box`, `local` and `to_doc` of every `Kind::Connector` scene node; READS the
// `from`, `to`, `route`, `startMarker`, `endMarker`, `markerSize` keys in `node.extra` and `cornerRadius`.
pub fn route(scene: &mut [SceneNode], vars: Vars<'_>) {
    // A scene without connectors (most are) is only looked through, at no cost in memory.
    if !scene.iter().any(has_connector) {
        return;
    }
    let mut boxes = HashMap::new();
    for root in scene.iter() {
        root.walk(
            &mut |node, _| {
                boxes.insert(node.id.clone(), doc_bounds(node));
            },
            0,
        );
    }
    for root in scene.iter_mut() {
        connect(root, &Affine::IDENTITY, &boxes, vars);
    }
}

/// Whether `scene` or anything in it is a connector.
#[must_use]
pub fn has_connector(scene: &SceneNode) -> bool {
    scene.node.kind == Kind::Connector || scene.children.iter().any(has_connector)
}

fn connect(scene: &mut SceneNode, parent: &Affine, boxes: &HashMap<String, Rect>, vars: Vars<'_>) {
    if scene.node.kind == Kind::Connector {
        draw(scene, parent, boxes, vars);
    }
    let own = scene.to_doc;
    for child in &mut scene.children {
        connect(child, &own, boxes, vars);
    }
}

/// Reads one end: a layer's id, or `{"id":..,"side":..,"at":0..1}`, or a free point `{"x":..,"y":..}` in the
/// coordinates of the connector's parent.
// Human: inside a component instance a layer id means the layer of that same instance first.
fn end(value: Option<&Value>, own_id: &str, parent: &Affine, boxes: &HashMap<String, Rect>) -> Option<End> {
    let value = value?;
    let number = |key: &str| value.get(key).and_then(Value::as_f64).map(narrow);
    let Some(id) = value.as_str().or_else(|| value.get("id").and_then(Value::as_str)) else {
        return Some(End::Free(parent.apply(Point::new(number("x")?, number("y")?))));
    };
    let inside = own_id.rsplit_once('/').and_then(|(prefix, _)| boxes.get(&format!("{prefix}/{id}")));
    let rect = *inside.or_else(|| boxes.get(id))?;
    let side = match value.get("side").and_then(Value::as_str) {
        Some("left") => Some(Side::Left),
        Some("right") => Some(Side::Right),
        Some("top") => Some(Side::Top),
        Some("bottom") => Some(Side::Bottom),
        Some("center") => Some(Side::Center),
        _ => None,
    };
    Some(End::Layer { rect, side, at: number("at").unwrap_or(0.5).clamp(0.0, 1.0) })
}

/// The key under which a connector's scene node carries the filled shapes at its ends (triangles, dots,
/// diamonds) as path data in the same box as its `geometry`. Never in a file: worked out with the line.
pub const MARKERS: &str = "markerGeometry";

/// The key under which a connector's scene node carries the points it runs through, in document coordinates:
/// where it meets its first end, each waypoint, where it meets its last end, as `[[x, y], ..]`. Never in a
/// file; the canvas puts its handles there.
pub const POINTS: &str = "connectorPoints";
/// The key under which a connector's scene node carries its line without dashes and markers, as path data in
/// document coordinates. Never in a file; the canvas finds the line under the pointer with it.
pub const LINE: &str = "connectorLine";

/// What sits at an end of the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Marker {
    None,
    /// An open arrow head, drawn with the line.
    Arrow,
    /// A short line across the end.
    Bar,
    /// A filled arrow head.
    Triangle,
    /// A filled dot.
    Dot,
    /// A filled diamond.
    Diamond,
}

impl Marker {
    fn read(value: Option<&Value>) -> Marker {
        match value.and_then(Value::as_str) {
            Some("arrow") => Marker::Arrow,
            Some("bar") => Marker::Bar,
            Some("triangle") => Marker::Triangle,
            Some("dot") => Marker::Dot,
            Some("diamond") => Marker::Diamond,
            _ => Marker::None,
        }
    }

    /// How much shorter the line is at this end, so that it stops where a filled shape begins.
    fn takes(self, size: f32) -> f32 {
        match self {
            Marker::Triangle | Marker::Diamond => size,
            Marker::Dot => size / 2.0,
            Marker::None | Marker::Arrow | Marker::Bar => 0.0,
        }
    }

    /// Writes the marker with its tip at `tip`, pointing along `heading`: into `stroked` or `filled`.
    fn write(self, tip: Point, heading: Point, size: f32, stroked: &mut String, filled: &mut String) {
        let along = |d: f32| Point::new(tip.x - heading.x * d, tip.y - heading.y * d);
        let (wx, wy) = (-heading.y * size, heading.x * size);
        let beside = |from: Point, w: f32| format!("{} {}", n(from.x + wx * w), n(from.y + wy * w));
        let _ = match self {
            Marker::None => Ok(()),
            Marker::Arrow => write!(stroked, "M{}L{} {}L{}", beside(along(size), 0.6), n(tip.x), n(tip.y), beside(along(size), -0.6)),
            Marker::Bar => write!(stroked, "M{}L{}", beside(tip, 0.6), beside(tip, -0.6)),
            Marker::Triangle => write!(filled, "M{} {}L{}L{}Z", n(tip.x), n(tip.y), beside(along(size), 0.5), beside(along(size), -0.5)),
            Marker::Diamond => {
                let (middle, far) = (along(size / 2.0), along(size));
                write!(filled, "M{} {}L{}L{} {}L{}Z", n(tip.x), n(tip.y), beside(middle, 0.35), n(far.x), n(far.y), beside(middle, -0.35))
            }
            Marker::Dot => {
                // A circle as four curves.
                let (c, r, k) = (along(size / 2.0), size / 2.0, size / 2.0 * 0.5523);
                write!(
                    filled,
                    "M{} {}C{} {} {} {} {} {}C{} {} {} {} {} {}C{} {} {} {} {} {}C{} {} {} {} {} {}Z",
                    n(c.x + r),
                    n(c.y),
                    n(c.x + r),
                    n(c.y + k),
                    n(c.x + k),
                    n(c.y + r),
                    n(c.x),
                    n(c.y + r),
                    n(c.x - k),
                    n(c.y + r),
                    n(c.x - r),
                    n(c.y + k),
                    n(c.x - r),
                    n(c.y),
                    n(c.x - r),
                    n(c.y - k),
                    n(c.x - k),
                    n(c.y - r),
                    n(c.x),
                    n(c.y - r),
                    n(c.x + k),
                    n(c.y - r),
                    n(c.x + r),
                    n(c.y - k),
                    n(c.x + r),
                    n(c.y)
                )
            }
        };
    }
}

/// How far a curved line's handle reaches from `a` toward `b` when it leaves `a` along `d`.
// Human: half the way along the direction the line leaves in, so a wire between two layers side by side is a
// gentle S; a wire that has to turn back still bulges out a little.
fn reach(a: Point, b: Point, d: Point) -> f32 {
    let (dx, dy) = ((b.x - a.x).abs(), (b.y - a.y).abs());
    let (along, across) = if d.x == 0.0 { (dy, dx) } else { (dx, dy) };
    (along * 0.5).max(across * 0.25).clamp(HANDLE.0.min(dx.hypot(dy) / 2.0), HANDLE.1)
}

/// A smooth line through `points` that leaves the first along `d0` and arrives at the last against `d1`.
// Human: between the ends the line passes through every point in the direction from the point before it to
// the point after it (a Catmull-Rom spline). `curvature` scales every handle: 1 is the usual S, 0 is straight
// lines, more than 1 swings wider.
fn curve(data: &mut String, points: &[Point], d0: Point, d1: Point, curvature: f32) {
    let Some((first, last)) = points.first().zip(points.last()) else { return };
    let count = points.len();
    if count < 2 {
        return;
    }
    // The handle at each point, as the step from the point to its control point on the way out.
    let handle = |i: usize| -> Point {
        let h = if i == 0 {
            let r = reach(*first, points[1], d0);
            Point::new(d0.x * r, d0.y * r)
        } else if i == count - 1 {
            let r = reach(*last, points[count - 2], d1);
            Point::new(-d1.x * r, -d1.y * r)
        } else {
            Point::new((points[i + 1].x - points[i - 1].x) / 6.0, (points[i + 1].y - points[i - 1].y) / 6.0)
        };
        Point::new(h.x * curvature, h.y * curvature)
    };
    let _ = write!(data, "M{} {}", n(first.x), n(first.y));
    for i in 0..count - 1 {
        let (a, b, out, into) = (points[i], points[i + 1], handle(i), handle(i + 1));
        let _ = write!(data, "C{} {} {} {} {} {}", n(a.x + out.x), n(a.y + out.y), n(b.x - into.x), n(b.y - into.y), n(b.x), n(b.y));
    }
}

/// The corners of a right-angled line through `via`, leaving `p0` along `d0` and arriving at `p1` against `d1`.
// Human: two points that share an x or a y are joined by one straight run. Otherwise a leg makes one turn: it
// first runs the way the line left its layer (sideways from a left or right side), and the last leg turns so
// that it arrives the way the far side faces. So waypoints are the corners or the lanes the line is to take.
fn elbow_through(p0: Point, d0: Point, via: &[Point], p1: Point, d1: Point) -> Vec<Point> {
    let mut points = vec![p0];
    let mut leg = |to: Point, sideways_first: bool| {
        let from = points.last().copied().unwrap_or(p0);
        if (from.x - to.x).abs() > 0.01 && (from.y - to.y).abs() > 0.01 {
            points.push(if sideways_first { Point::new(to.x, from.y) } else { Point::new(from.x, to.y) });
        }
        points.push(to);
    };
    for point in via {
        leg(*point, d0.x != 0.0);
    }
    leg(p1, d1.x == 0.0);
    points
}

/// Cuts a line into dashes: `pattern` is the lengths drawn and left out in turn.
fn dashed(data: &str, pattern: &[f32]) -> String {
    let mut out = String::new();
    let Ok(segments) = path::parse(data) else { return out };
    for run in path::flatten(&segments, 48) {
        let (mut index, mut left, mut drawing) = (0usize, pattern[0], true);
        let mut pen_down = false;
        for pair in run.windows(2) {
            let (mut from, to) = (pair[0], pair[1]);
            let mut length = (to.x - from.x).hypot(to.y - from.y);
            while length > 1e-4 {
                let step = left.min(length);
                let t = step / length;
                let next = Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
                if drawing {
                    if !pen_down {
                        let _ = write!(out, "M{} {}", n(from.x), n(from.y));
                        pen_down = true;
                    }
                    let _ = write!(out, "L{} {}", n(next.x), n(next.y));
                }
                (from, length, left) = (next, length - step, left - step);
                if left <= 1e-4 {
                    index = (index + 1) % pattern.len();
                    (left, drawing, pen_down) = (pattern[index], !drawing, false);
                }
            }
        }
    }
    out
}

fn draw(scene: &mut SceneNode, parent: &Affine, boxes: &HashMap<String, Rect>, vars: Vars<'_>) {
    scene.node.geometry = None;
    for key in [MARKERS, POINTS, LINE] {
        scene.node.extra.remove(key);
    }
    scene.local = Rect::ZERO;
    let extra = &scene.node.extra;
    let (Some(from), Some(to)) = (end(extra.get("from"), &scene.id, parent, boxes), end(extra.get("to"), &scene.id, parent, boxes)) else {
        return;
    };
    let number = |key: &str| extra.get(key).and_then(Value::as_f64).map(narrow);
    // Human: `via` are the points the line passes through on its way, in order: a point of the parent, or a
    // layer (its middle), which then carries the line with it when it moves.
    let via: Vec<Point> = extra
        .get("via")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|point| end(Some(point), &scene.id, parent, boxes))
        .map(|point| point.center())
        .collect();
    let route = match extra.get("route").and_then(Value::as_str) {
        Some("straight") => Route::Straight,
        Some("elbow") => Route::Elbow,
        _ => Route::Curved,
    };
    let size = number("markerSize").unwrap_or(MARKER_SIZE).max(0.0);
    let (mark0, mark1) = (Marker::read(extra.get("startMarker")), Marker::read(extra.get("endMarker")));
    let radius = match &scene.node.corner_radius {
        Some(Corners::All(radius)) => vars.number(radius, &scene.theme).unwrap_or(0.0).max(0.0),
        _ => 0.0,
    };
    // Each end faces the next point of the line: the first waypoint, or the other end.
    let (tip0, d0) = from.anchor(via.first().copied().unwrap_or_else(|| to.center()));
    let (tip1, d1) = to.anchor(via.last().copied().unwrap_or_else(|| from.center()));
    // The way the line is going when it reaches each end.
    let toward = |tip: Point, d: Point, next: Point| {
        if route == Route::Straight { unit(Point::new(tip.x - next.x, tip.y - next.y)) } else { Point::new(-d.x, -d.y) }
    };
    let head0 = toward(tip0, d0, via.first().copied().unwrap_or(tip1));
    let head1 = toward(tip1, d1, via.last().copied().unwrap_or(tip0));
    // The line itself stops where a filled marker begins.
    let (p0, p1) = (
        Point::new(tip0.x - head0.x * mark0.takes(size), tip0.y - head0.y * mark0.takes(size)),
        Point::new(tip1.x - head1.x * mark1.takes(size), tip1.y - head1.y * mark1.takes(size)),
    );
    let through: Vec<Point> = std::iter::once(p0).chain(via.iter().copied()).chain(std::iter::once(p1)).collect();

    let mut data = String::new();
    match route {
        Route::Straight => line(&mut data, &through, radius),
        Route::Curved => curve(&mut data, &through, d0, d1, number("curvature").unwrap_or(1.0).max(0.0)),
        Route::Elbow if via.is_empty() => {
            let stub = number("stub").unwrap_or(STUB).max(0.0);
            line(&mut data, &elbow((p0, d0, from.rect()), (p1, d1, to.rect()), stub), radius);
        }
        Route::Elbow => line(&mut data, &elbow_through(p0, d0, &via, p1, d1), radius),
    }
    // Human: `dash` is one length (drawn and left out alike) or a list of lengths drawn and left out in turn.
    let pattern: Vec<f32> = match extra.get("dash") {
        Some(Value::Array(lengths)) => lengths.iter().filter_map(Value::as_f64).map(narrow).collect(),
        Some(one) => one.as_f64().map(narrow).into_iter().collect(),
        None => Vec::new(),
    };
    let whole_line = data.clone();
    let stops: Vec<Value> = std::iter::once(tip0)
        .chain(via.iter().copied())
        .chain(std::iter::once(tip1))
        .map(|p| Value::from(vec![f64::from(p.x), f64::from(p.y)]))
        .collect();
    if !pattern.is_empty() && pattern.iter().all(|length| *length > 0.01) {
        data = dashed(&data, &pattern);
    }
    let mut filled = String::new();
    mark0.write(tip0, head0, size, &mut data, &mut filled);
    mark1.write(tip1, head1, size, &mut data, &mut filled);

    // The path was written in document coordinates; the layer's box is the path's, and the path moves into it.
    let whole = format!("{data}{filled}");
    let Some(bounds) = path::parse(&whole).ok().and_then(|segments| path::bounds(&segments)) else { return };
    scene.node.geometry = Some(data);
    scene.node.extra.insert(POINTS.to_owned(), Value::Array(stops));
    scene.node.extra.insert(LINE.to_owned(), Value::String(whole_line));
    if !filled.is_empty() {
        scene.node.extra.insert(MARKERS.to_owned(), Value::String(filled));
    }
    scene.node.view_box = Some([f64::from(bounds.x), f64::from(bounds.y), f64::from(bounds.w), f64::from(bounds.h)]);
    let corner = parent.invert().map_or(Point::new(bounds.x, bounds.y), |inverse| inverse.apply(Point::new(bounds.x, bounds.y)));
    scene.local = Rect::new(corner.x, corner.y, bounds.w, bounds.h);
    scene.to_doc = Affine::translate(bounds.x, bounds.y);
}

fn unit(v: Point) -> Point {
    let length = v.x.hypot(v.y);
    if length > 1e-6 { Point::new(v.x / length, v.y / length) } else { Point::new(1.0, 0.0) }
}

/// A number as path data: two decimals at most.
fn n(value: f32) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// The corners of a right-angled line between two ends, each given as where the line starts, the direction it
/// leaves in, and the box of its layer.
// Human: two ends that face each other meet halfway with two turns; an end that would have to run back through
// its own layer first steps out by a stub and goes around both layers: between them where there is room, else
// past whichever side is the shorter way. A sideways end and an upright end meet in one corner.
fn elbow((p0, d0, r0): (Point, Point, Rect), (p1, d1, r1): (Point, Point, Rect), stub: f32) -> Vec<Point> {
    let (flat0, flat1) = (d0.x != 0.0, d1.x != 0.0);
    if flat0 != flat1 {
        let corner = if flat0 { Point::new(p1.x, p0.y) } else { Point::new(p0.x, p1.y) };
        return vec![p0, corner, p1];
    }
    // Work as if both ends were sideways; upright ends are the same with x and y swapped.
    let swap = |p: Point| if flat0 { p } else { Point::new(p.y, p.x) };
    let (a, da, b, db) = (swap(p0), swap(d0), swap(p1), swap(d1));
    // The room each layer takes across the line's direction.
    let across = |r: Rect| if flat0 { (r.y, r.bottom()) } else { (r.x, r.right()) };
    let (room_a, room_b) = (across(r0), across(r1));
    let ahead = (b.x - a.x) * da.x > 2.0 * stub && (a.x - b.x) * db.x > 2.0 * stub;
    let points = if ahead {
        let middle = f32::midpoint(a.x, b.x);
        vec![a, Point::new(middle, a.y), Point::new(middle, b.y), b]
    } else if (da.x > 0.0) == (db.x > 0.0) {
        // Both leave the same way (right to right): out past the further one and back.
        let out = if da.x > 0.0 { a.x.max(b.x) + stub } else { a.x.min(b.x) - stub };
        vec![a, Point::new(out, a.y), Point::new(out, b.y), b]
    } else {
        let (out_a, out_b) = (a.x + da.x * stub, b.x + db.x * stub);
        let middle = if room_a.1 + 2.0 * stub <= room_b.0 {
            f32::midpoint(room_a.1, room_b.0)
        } else if room_b.1 + 2.0 * stub <= room_a.0 {
            f32::midpoint(room_b.1, room_a.0)
        } else {
            let (over, under) = (room_a.0.min(room_b.0) - stub, room_a.1.max(room_b.1) + stub);
            let way = |lane: f32| (a.y - lane).abs() + (b.y - lane).abs();
            if way(over) <= way(under) { over } else { under }
        };
        vec![a, Point::new(out_a, a.y), Point::new(out_a, middle), Point::new(out_b, middle), Point::new(out_b, b.y), b]
    };
    points.into_iter().map(swap).collect()
}

/// Writes a line through `points`, its corners rounded by `radius` (never more than half a leg).
fn line(data: &mut String, points: &[Point], radius: f32) {
    let mut points: Vec<Point> = points.to_vec();
    points.dedup_by(|b, a| (a.x - b.x).abs() < 0.01 && (a.y - b.y).abs() < 0.01);
    let Some(first) = points.first() else { return };
    let _ = write!(data, "M{} {}", n(first.x), n(first.y));
    for i in 1..points.len() {
        let here = points[i];
        let Some(next) = points.get(i + 1).filter(|_| radius > 0.0) else {
            let _ = write!(data, "L{} {}", n(here.x), n(here.y));
            continue;
        };
        let before = points[i - 1];
        let (leg_in, leg_out) = ((here.x - before.x).hypot(here.y - before.y), (next.x - here.x).hypot(next.y - here.y));
        let cut = radius.min(leg_in / 2.0).min(leg_out / 2.0);
        let (toward, away) = (unit(Point::new(before.x - here.x, before.y - here.y)), unit(Point::new(next.x - here.x, next.y - here.y)));
        let (start, stop) = (Point::new(here.x + toward.x * cut, here.y + toward.y * cut), Point::new(here.x + away.x * cut, here.y + away.y * cut));
        let _ = write!(data, "L{} {}Q{} {} {} {}", n(start.x), n(start.y), n(here.x), n(here.y), n(stop.x), n(stop.y));
    }
}

#[cfg(test)]
mod tests {
    use crate::layout::{self, TextStyle};
    use crate::model::Document;
    use crate::scene::{self, SceneNode};
    use crate::vars::{ThemeContext, Vars};

    fn measure(_: &TextStyle) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn laid_out(json: &str) -> Vec<SceneNode> {
        let doc: Document = serde_json::from_str(json).unwrap();
        let mut scene = scene::build(&doc, &ThemeContext::defaults(&doc));
        layout::layout(&mut scene, Vars::new(&doc), &measure);
        scene
    }

    fn wire(scene: &[SceneNode]) -> &SceneNode {
        let mut found = None;
        for root in scene {
            root.walk(
                &mut |node, _| {
                    if node.node.kind == crate::Kind::Connector {
                        found = Some(node);
                    }
                },
                0,
            );
        }
        found.unwrap()
    }

    fn board(b_x: u32, wire: &str) -> String {
        format!(
            r##"{{"version":"2.22","children":[{{"type":"frame","id":"board","layout":"none","width":800,"height":600,"children":[
            {{"type":"rectangle","id":"a","x":100,"y":100,"width":100,"height":40}},
            {{"type":"rectangle","id":"b","x":{b_x},"y":300,"width":100,"height":40}},
            {{"type":"connector","id":"w","stroke":"#888888","strokeWidth":2,{wire}}}]}}]}}"##
        )
    }

    #[test]
    fn a_wire_runs_between_the_facing_sides_and_follows_a_layer_that_moves() {
        let scene = laid_out(&board(400, r#""from":{"id":"a","side":"right"},"to":{"id":"b","side":"left"}"#));
        let w = wire(&scene);
        // From the middle of a's right side (200, 120) to the middle of b's left side (400, 320).
        assert_eq!(w.node.geometry.as_deref(), Some("M200 120C300 120 300 320 400 320"));
        assert_eq!(layout::doc_bounds(w), crate::geom::Rect::new(200.0, 120.0, 200.0, 200.0));
        let moved = laid_out(&board(500, r#""from":{"id":"a","side":"right"},"to":{"id":"b","side":"left"}"#));
        assert!(wire(&moved).node.geometry.as_deref().unwrap().ends_with("500 320"));
    }

    #[test]
    fn sides_are_picked_when_none_is_named_and_a_missing_layer_draws_nothing() {
        let scene = laid_out(&board(400, r#""from":"a","to":"b","route":"straight","endMarker":"arrow""#));
        // The layers lie further apart sideways than up and down: right side to left side, with an arrow head.
        assert!(wire(&scene).node.geometry.as_deref().unwrap().starts_with("M200 120L400 320M"));
        let scene = laid_out(&board(400, r#""from":"a","to":"gone""#));
        assert_eq!(wire(&scene).node.geometry, None);
        assert_eq!(wire(&scene).local.w, 0.0);
    }

    #[test]
    fn an_elbow_turns_halfway_with_rounded_corners_and_a_free_end_is_a_point_of_the_parent() {
        let scene = laid_out(&board(400, r#""from":{"id":"a","side":"right"},"to":{"id":"b","side":"left"},"route":"elbow","cornerRadius":10"#));
        assert_eq!(wire(&scene).node.geometry.as_deref(), Some("M200 120L290 120Q300 120 300 130L300 310Q300 320 310 320L400 320"));
        let scene = laid_out(&board(400, r#""from":{"id":"a","side":"bottom"},"to":{"x":150,"y":400},"route":"straight""#));
        assert_eq!(wire(&scene).node.geometry.as_deref(), Some("M150 140L150 400"));
    }

    #[test]
    fn an_elbow_that_has_to_turn_back_goes_around_both_layers() {
        // From a's left side to b's right side, with b to the right of a: out, around and back in.
        let scene = laid_out(&board(400, r#""from":{"id":"a","side":"left"},"to":{"id":"b","side":"right"},"route":"elbow""#));
        // a is 100..140 down the page and b 300..340: there is room between them, at 220.
        assert_eq!(wire(&scene).node.geometry.as_deref(), Some("M100 120L84 120L84 220L516 220L516 320L500 320"));
        // Layers level with each other leave no room between: the line passes over them.
        let level = r##"{"version":"2.22","children":[{"type":"frame","id":"board","layout":"none","width":800,"height":600,"children":[
            {"type":"rectangle","id":"a","x":100,"y":100,"width":100,"height":40},
            {"type":"rectangle","id":"b","x":400,"y":110,"width":100,"height":40},
            {"type":"connector","id":"w","stroke":"#888888","from":{"id":"a","side":"left"},"to":{"id":"b","side":"right"},"route":"elbow"}]}]}"##;
        assert_eq!(wire(&laid_out(level)).node.geometry.as_deref(), Some("M100 120L84 120L84 84L516 84L516 130L500 130"));
    }

    #[test]
    fn waypoints_steer_the_line_whatever_its_route() {
        let ends = r#""from":{"id":"a","side":"right"},"to":{"id":"b","side":"left"}"#;
        // Straight: a line from point to point.
        let scene = laid_out(&board(400, &format!(r#"{ends},"route":"straight","via":[{{"x":300,"y":60}}]"#)));
        assert_eq!(wire(&scene).node.geometry.as_deref(), Some("M200 120L300 60L400 320"));
        // Elbow: waypoints are corners; two that share a y make the lane the line runs along, here over the top.
        let scene = laid_out(&board(400, &format!(r#"{ends},"route":"elbow","via":[{{"x":240,"y":60}},{{"x":360,"y":60}}]"#)));
        assert_eq!(wire(&scene).node.geometry.as_deref(), Some("M200 120L240 120L240 60L360 60L360 320L400 320"));
        // Curved: through the point, in the direction from the end before it to the end after it.
        let scene = laid_out(&board(400, &format!(r#"{ends},"via":[{{"x":300,"y":400}}]"#)));
        let data = wire(&scene).node.geometry.clone().unwrap();
        assert!(data.starts_with("M200 120C") && data.contains(" 300 400C") && data.ends_with(" 400 320"), "{data}");
        // A waypoint can be a layer: the line goes through its middle, and follows it.
        let scene = laid_out(&board(400, &format!(r#"{ends},"route":"straight","via":["a"]"#)));
        assert_eq!(wire(&scene).node.geometry.as_deref(), Some("M200 120L150 120L400 320"));
    }

    #[test]
    fn the_line_can_be_dashed_flattened_and_end_in_filled_shapes() {
        let ends = r#""from":{"id":"a","side":"bottom"},"to":{"x":150,"y":240},"route":"straight""#;
        // 100 long, in dashes of 30 with gaps of 20.
        let scene = laid_out(&board(400, &format!(r#"{ends},"dash":[30,20]"#)));
        assert_eq!(wire(&scene).node.geometry.as_deref(), Some("M150 140L150 170M150 190L150 220"));
        // A filled triangle is kept apart from the line, which stops at its base.
        let scene = laid_out(&board(400, &format!(r#"{ends},"endMarker":"triangle","markerSize":10,"startMarker":"bar""#)));
        let w = wire(&scene);
        assert_eq!(w.node.geometry.as_deref(), Some("M150 140L150 230M156 140L144 140"));
        assert_eq!(w.node.extra.get(super::MARKERS).and_then(|v| v.as_str()), Some("M150 240L145 230L155 230Z"));
        // No curvature makes a curved route a straight line.
        let scene = laid_out(&board(400, r#""from":{"id":"a","side":"right"},"to":{"id":"b","side":"left"},"curvature":0"#));
        assert_eq!(wire(&scene).node.geometry.as_deref(), Some("M200 120C200 120 400 320 400 320"));
    }

    #[test]
    fn a_wire_takes_no_room_in_a_flex_layout_or_a_group() {
        let scene = laid_out(
            r##"{"version":"2.22","children":[{"type":"frame","id":"row","layout":"horizontal","gap":50,"children":[
            {"type":"rectangle","id":"a","width":100,"height":40},
            {"type":"connector","id":"w","stroke":"#888888","from":"a","to":"b","route":"straight"},
            {"type":"rectangle","id":"b","width":100,"height":40}]}]}"##,
        );
        assert_eq!(scene[0].local.w, 250.0);
        assert_eq!(wire(&scene).node.geometry.as_deref(), Some("M100 20L150 20"));
    }
}
