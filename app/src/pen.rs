//! The pen tool: a path drawn point by point, and the layer it becomes.
// Human: a click places a corner point, a press
// and pull places a smooth point whose two handles mirror each other, a click on the first point closes the
// path. A finished path's points are edited as [`Contour`]s: moved, bent by their handles, removed, added,
// made smooth or sharp, and cut.

use std::fmt::Write as _;

use pen_doc::geom::{Affine, Point};
use pen_doc::model::{Kind, Node, Paint, Paints, StrokeWidth};
use pen_doc::path::{self, Segment};
use pen_doc::value::Num;

/// The stroke colour of a new path.
const STROKE: &str = "#000000";
/// The stroke width of a new path.
const STROKE_WIDTH: f32 = 1.0;
/// Points nearer than this are one point, and a handle this near its point is no handle.
const CLOSE: f32 = 1e-3;
/// How many places along each line are tried to find the one nearest the pointer.
const SAMPLES: u8 = 48;
/// The step ⇧ holds a new point's direction to, from the point before it (45°).
const STEP: f32 = std::f32::consts::FRAC_PI_4;

/// One point of the path, with the handles that bend the lines into and out of it; all in document coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    /// Where the point is.
    pub at: Point,
    /// The handle bending the line that arrives here.
    pub before: Option<Point>,
    /// The handle bending the line that leaves from here.
    pub after: Option<Point>,
}

/// What a press with the pen did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pressed {
    /// A point was added.
    Placed,
    /// The press was on the first point: the path is to be closed.
    Closes,
}

/// The path being drawn; empty while none is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Draft {
    anchors: Vec<Anchor>,
}

impl Draft {
    /// Whether no path is being drawn.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// The points placed so far.
    #[must_use]
    pub fn anchors(&self) -> &[Anchor] {
        &self.anchors
    }

    /// Whether a press at `at` would close the path: there are two points or more and `at` is within `reach`
    /// (document units) of the first.
    #[must_use]
    pub fn closes(&self, at: Point, reach: f32) -> bool {
        self.anchors.len() >= 2 && self.anchors.first().is_some_and(|first| distance(first.at, at) <= reach)
    }

    /// Where a new point lands for a pointer at `at`: there, or with `held` (⇧) on the nearest 45° line from
    /// the last point.
    #[must_use]
    pub fn place(&self, at: Point, held: bool) -> Point {
        match self.anchors.last() {
            Some(last) if held => constrain(last.at, at),
            _ => at,
        }
    }

    /// A press at `at`: closes the path when on the first point (see [`Draft::closes`]), else adds a corner point.
    pub fn press(&mut self, at: Point, reach: f32) -> Pressed {
        if self.closes(at, reach) {
            return Pressed::Closes;
        }
        self.anchors.push(Anchor { at, before: None, after: None });
        Pressed::Placed
    }

    /// Pulls the last point's handles: the one after it to `to` (with `held`, on a 45° line), the one before it
    /// mirrored, which makes the point smooth. A pull back onto the point makes it a corner again.
    pub fn pull(&mut self, to: Point, held: bool) {
        let Some(last) = self.anchors.last_mut() else { return };
        let to = if held { constrain(last.at, to) } else { to };
        if distance(last.at, to) < f32::EPSILON {
            (last.before, last.after) = (None, None);
        } else {
            last.after = Some(to);
            last.before = Some(Point::new(2.0 * last.at.x - to.x, 2.0 * last.at.y - to.y));
        }
    }

    /// Takes the last point away again; `false` when there was none.
    pub fn pop(&mut self) -> bool {
        self.anchors.pop().is_some()
    }

    /// Ends the drawing and gives the points drawn.
    pub fn take(&mut self) -> Vec<Anchor> {
        std::mem::take(&mut self.anchors)
    }
}

/// The path layer for `anchors` (document coordinates) inside a parent whose coordinates `to_parent` leads to,
/// and where its box starts there. `None` for fewer than two points: one point draws nothing.
#[must_use]
pub fn layer(anchors: &[Anchor], closed: bool, to_parent: &Affine, id: String) -> Option<(Node, Point)> {
    if anchors.len() < 2 {
        return None;
    }
    let local: Vec<Anchor> = anchors
        .iter()
        .map(|a| Anchor { at: to_parent.apply(a.at), before: a.before.map(|p| to_parent.apply(p)), after: a.after.map(|p| to_parent.apply(p)) })
        .collect();
    // Agent: the box is measured with the same reader and bounds the renderer fits a path with, so the layer
    // draws exactly where it was drawn.
    let bounds = path::bounds(&path::parse(&geometry(&local, closed, Point::default())).ok()?)?;
    let origin = Point::new(bounds.x, bounds.y);
    let mut node = Node::new(Kind::Path, id);
    node.geometry = Some(geometry(&local, closed, origin));
    node.width = Some(Num::new(bounds.w));
    node.height = Some(Num::new(bounds.h));
    node.stroke = Some(Paints::One(Paint::Solid(STROKE.to_owned())));
    node.stroke_width = Some(StrokeWidth::All(Num::new(STROKE_WIDTH)));
    Some((node, origin))
}

/// SVG path data through `anchors`, written relative to `origin`: straight lines between corner points, cubic
/// curves where a handle bends the line.
#[must_use]
pub fn geometry(anchors: &[Anchor], closed: bool, origin: Point) -> String {
    let mut data = String::new();
    write_outline(&mut data, anchors, closed, origin);
    data
}

/// SVG path data for all of `contours`, in their own coordinates.
#[must_use]
pub fn data(contours: &[Contour]) -> String {
    let mut data = String::new();
    for contour in contours {
        if !data.is_empty() {
            data.push(' ');
        }
        write_outline(&mut data, &contour.anchors, contour.closed, Point::default());
    }
    data
}

fn write_outline(data: &mut String, anchors: &[Anchor], closed: bool, origin: Point) {
    let Some(first) = anchors.first() else { return };
    let pair = |p: Point| format!("{} {}", number(p.x - origin.x), number(p.y - origin.y));
    let _ = write!(data, "M{}", pair(first.at));
    let ends = anchors.iter().skip(1).chain(closed.then_some(first));
    for (from, to) in anchors.iter().zip(ends) {
        if from.after.is_none() && to.before.is_none() {
            // A straight way back to the start is what `Z` draws by itself.
            if !std::ptr::eq(to, first) {
                let _ = write!(data, " L{}", pair(to.at));
            }
        } else {
            let _ = write!(data, " C{} {} {}", pair(from.after.unwrap_or(from.at)), pair(to.before.unwrap_or(to.at)), pair(to.at));
        }
    }
    if closed {
        data.push_str(" Z");
    }
}

/// One outline of a path: its points in order, and whether the last joins the first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Contour {
    /// The points.
    pub anchors: Vec<Anchor>,
    /// Whether a line runs from the last point back to the first.
    pub closed: bool,
}

/// Which of a point's two handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The handle of the line arriving at the point.
    Before,
    /// The handle of the line leaving it.
    After,
}

/// A point of a path: which outline, which point in it.
pub type Spot = (usize, usize);

/// The outlines `segments` draw, as points with handles. Quadratic curves become cubic ones.
#[must_use]
pub fn contours(segments: &[Segment]) -> Vec<Contour> {
    let handle = |at: Point, to: Point| (distance(at, to) > CLOSE).then_some(to);
    let mut all: Vec<Contour> = Vec::new();
    for segment in segments {
        match *segment {
            Segment::MoveTo(p) => all.push(Contour { anchors: vec![corner(p)], closed: false }),
            Segment::Close => {
                if let Some(contour) = all.last_mut() {
                    close(contour);
                }
            }
            Segment::LineTo(p) => {
                if let Some(contour) = open(&mut all) {
                    contour.anchors.push(corner(p));
                }
            }
            Segment::QuadTo(c, p) => {
                if let Some(contour) = open(&mut all)
                    && let Some(last) = contour.anchors.last_mut()
                {
                    // A quadratic curve's one control point, as the two of the same cubic curve.
                    let third = |from: Point| Point::new(from.x + (c.x - from.x) * 2.0 / 3.0, from.y + (c.y - from.y) * 2.0 / 3.0);
                    last.after = handle(last.at, third(last.at));
                    contour.anchors.push(Anchor { at: p, before: handle(p, third(p)), after: None });
                }
            }
            Segment::CubicTo(c1, c2, p) => {
                if let Some(contour) = open(&mut all)
                    && let Some(last) = contour.anchors.last_mut()
                {
                    last.after = handle(last.at, c1);
                    contour.anchors.push(Anchor { at: p, before: handle(p, c2), after: None });
                }
            }
        }
    }
    all.retain(|c| !c.anchors.is_empty());
    all
}

/// The outline a drawing command adds to: the last one, or after a `Z` a new one starting where that one did.
fn open(all: &mut Vec<Contour>) -> Option<&mut Contour> {
    if all.last().is_some_and(|c| c.closed) {
        let start = all.last().and_then(|c| c.anchors.first()).map(|a| a.at)?;
        all.push(Contour { anchors: vec![corner(start)], closed: false });
    }
    all.last_mut()
}

/// Closes an outline; a last point drawn back onto the first is that first point.
fn close(contour: &mut Contour) {
    contour.closed = true;
    if contour.anchors.len() > 2
        && let (Some(first), Some(last)) = (contour.anchors.first().copied(), contour.anchors.last().copied())
        && distance(first.at, last.at) <= CLOSE
    {
        contour.anchors.pop();
        contour.anchors[0].before = last.before;
    }
}

fn corner(at: Point) -> Anchor {
    Anchor { at, before: None, after: None }
}

/// Moves the points at `spots` by (`dx`, `dy`), their handles with them.
pub fn shift(contours: &mut [Contour], spots: &[Spot], dx: f32, dy: f32) {
    let by = |p: Point| Point::new(p.x + dx, p.y + dy);
    for &(c, a) in spots {
        if let Some(anchor) = contours.get_mut(c).and_then(|c| c.anchors.get_mut(a)) {
            *anchor = Anchor { at: by(anchor.at), before: anchor.before.map(by), after: anchor.after.map(by) };
        }
    }
}

/// Puts one handle of the point at `spot` at `to`; with `mirrored` the other handle keeps opposite it, at its
/// own length (a smooth point stays smooth).
pub fn bend(contours: &mut [Contour], spot: Spot, side: Side, to: Point, mirrored: bool) {
    let Some(anchor) = contours.get_mut(spot.0).and_then(|c| c.anchors.get_mut(spot.1)) else { return };
    let other = match side {
        Side::Before => anchor.after,
        Side::After => anchor.before,
    };
    let reach = distance(anchor.at, to);
    let opposite = match other {
        Some(other) if mirrored && reach > CLOSE => {
            let scale = distance(anchor.at, other) / reach;
            Some(Point::new(anchor.at.x - (to.x - anchor.at.x) * scale, anchor.at.y - (to.y - anchor.at.y) * scale))
        }
        other => other,
    };
    let pulled = (reach > CLOSE).then_some(to);
    (anchor.before, anchor.after) = match side {
        Side::Before => (pulled, opposite),
        Side::After => (opposite, pulled),
    };
}

/// Whether the point's two handles lie on one line through it, so pulling one should turn the other.
#[must_use]
pub fn is_smooth(anchor: &Anchor) -> bool {
    let (Some(before), Some(after)) = (anchor.before, anchor.after) else { return false };
    let (bx, by, ax, ay) = (before.x - anchor.at.x, before.y - anchor.at.y, after.x - anchor.at.x, after.y - anchor.at.y);
    let (cross, dot) = (bx * ay - by * ax, bx * ax + by * ay);
    dot < 0.0 && cross.abs() <= 0.02 * bx.hypot(by) * ax.hypot(ay)
}

/// Makes the point at `spot` sharp when it has handles, else smooth: handles along the line between its
/// neighbours, a third of the way to each.
pub fn toggle_smooth(contours: &mut [Contour], spot: Spot) {
    let Some(contour) = contours.get_mut(spot.0) else { return };
    let (previous, next) = neighbours(contour, spot.1);
    let Some(anchor) = contour.anchors.get_mut(spot.1) else { return };
    if anchor.before.is_some() || anchor.after.is_some() {
        (anchor.before, anchor.after) = (None, None);
        return;
    }
    let at = anchor.at;
    let (from, to) = (previous.unwrap_or(at), next.unwrap_or(at));
    let length = distance(from, to);
    if length <= CLOSE {
        return;
    }
    let (ux, uy) = ((to.x - from.x) / length, (to.y - from.y) / length);
    let arm = |toward: Option<Point>, sign: f32| {
        toward.map(|p| {
            let reach = distance(at, p) / 3.0;
            Point::new(at.x + sign * ux * reach, at.y + sign * uy * reach)
        })
    };
    (anchor.before, anchor.after) = (arm(previous, -1.0), arm(next, 1.0));
}

/// The points before and after the one at `index` along the outline, where there are any.
fn neighbours(contour: &Contour, index: usize) -> (Option<Point>, Option<Point>) {
    let count = contour.anchors.len();
    let at = |i: usize| contour.anchors.get(i).map(|a| a.at);
    let previous = if index > 0 {
        at(index - 1)
    } else if contour.closed && count > 1 {
        at(count - 1)
    } else {
        None
    };
    let next = if index + 1 < count {
        at(index + 1)
    } else if contour.closed && count > 1 {
        at(0)
    } else {
        None
    };
    (previous, next)
}

/// Takes the points at `spots` out. An outline left with fewer than two points goes with them.
pub fn remove(contours: &mut Vec<Contour>, spots: &[Spot]) {
    for (c, contour) in contours.iter_mut().enumerate() {
        let mut index = 0;
        contour.anchors.retain(|_| {
            index += 1;
            !spots.contains(&(c, index - 1))
        });
        if contour.anchors.len() < 3 {
            contour.closed = false;
        }
    }
    contours.retain(|c| c.anchors.len() >= 2);
}

/// How many lines the outline has: one fewer than its points, or as many when it is closed.
fn stretches(contour: &Contour) -> usize {
    if contour.closed { contour.anchors.len() } else { contour.anchors.len().saturating_sub(1) }
}

/// The two ends of the outline's line number `index`, with the place of the second in the outline.
fn stretch(contour: &Contour, index: usize) -> Option<(Anchor, Anchor, usize)> {
    let to = (index + 1) % contour.anchors.len().max(1);
    (index < stretches(contour)).then(|| Some((*contour.anchors.get(index)?, *contour.anchors.get(to)?, to))).flatten()
}

/// The four points of the cubic curve between two path points (a straight line has its handles on its ends).
fn cubic(from: &Anchor, to: &Anchor) -> [Point; 4] {
    [from.at, from.after.unwrap_or(from.at), to.before.unwrap_or(to.at), to.at]
}

fn mix(a: Point, b: Point, t: f32) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn cubic_at(curve: &[Point; 4], t: f32) -> Point {
    let (ab, bc, cd) = (mix(curve[0], curve[1], t), mix(curve[1], curve[2], t), mix(curve[2], curve[3], t));
    mix(mix(ab, bc, t), mix(bc, cd, t), t)
}

/// The place on the path nearest to `p`: the outline, the line in it, how far along that line (0 – 1), and the
/// distance. `through` leads from the path's coordinates to the ones `p` is in (the screen's).
#[must_use]
pub fn nearest_stretch(contours: &[Contour], through: &Affine, p: Point) -> Option<(usize, usize, f32, f32)> {
    let mut best: Option<(usize, usize, f32, f32)> = None;
    for (c, contour) in contours.iter().enumerate() {
        for index in 0..stretches(contour) {
            let Some((from, to, _)) = stretch(contour, index) else { continue };
            let curve = cubic(&from, &to).map(|q| through.apply(q));
            for step in 0..=SAMPLES {
                let t = f32::from(step) / f32::from(SAMPLES);
                let far = distance(cubic_at(&curve, t), p);
                if best.is_none_or(|(_, _, _, nearest)| far < nearest) {
                    best = Some((c, index, t, far));
                }
            }
        }
    }
    best
}

/// Adds a point on line `index` of outline `contour`, `t` of the way along, without changing the path's
/// shape; gives the new point's place.
pub fn insert(contours: &mut [Contour], contour: usize, index: usize, t: f32) -> Option<Spot> {
    let outline = contours.get_mut(contour)?;
    let (from, to, end) = stretch(outline, index)?;
    let t = t.clamp(0.01, 0.99);
    let mut added = corner(mix(from.at, to.at, t));
    if from.after.is_some() || to.before.is_some() {
        // Human: a curve is split the usual way (de Casteljau), which leaves its shape as it was.
        let curve = cubic(&from, &to);
        let (ab, bc, cd) = (mix(curve[0], curve[1], t), mix(curve[1], curve[2], t), mix(curve[2], curve[3], t));
        let (abc, bcd) = (mix(ab, bc, t), mix(bc, cd, t));
        added = Anchor { at: mix(abc, bcd, t), before: Some(abc), after: Some(bcd) };
        outline.anchors[index].after = Some(ab);
        outline.anchors[end].before = Some(cd);
    }
    outline.anchors.insert(index + 1, added);
    Some((contour, index + 1))
}

/// Cuts the path open at the point at `spot`: a closed outline opens there, an open one falls into two. The
/// ends of an open outline can't be cut; `false` then.
pub fn cut(contours: &mut Vec<Contour>, spot: Spot) -> bool {
    let Some(outline) = contours.get_mut(spot.0) else { return false };
    let Some(at) = outline.anchors.get(spot.1).copied() else { return false };
    let (start, end) = (Anchor { before: None, ..at }, Anchor { after: None, ..at });
    if outline.closed {
        outline.anchors.rotate_left(spot.1);
        outline.anchors[0] = start;
        outline.anchors.push(end);
        outline.closed = false;
        return true;
    }
    if spot.1 == 0 || spot.1 + 1 >= outline.anchors.len() {
        return false;
    }
    let mut rest = outline.anchors.split_off(spot.1);
    outline.anchors.push(end);
    rest[0] = start;
    contours.insert(spot.0 + 1, Contour { anchors: rest, closed: false });
    true
}

/// `to` moved onto the nearest line through `from` at a multiple of 45°, keeping its distance.
#[must_use]
pub fn constrain(from: Point, to: Point) -> Point {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let angle = (dy.atan2(dx) / STEP).round() * STEP;
    let length = dx.hypot(dy);
    Point::new(from.x + angle.cos() * length, from.y + angle.sin() * length)
}

fn distance(a: Point, b: Point) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// A number as path data has it: two decimals at most, no trailing zeros, no negative zero.
fn number(v: f32) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    if rounded == 0.0 { "0".to_owned() } else { format!("{rounded}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corner(x: f32, y: f32) -> Anchor {
        Anchor { at: Point::new(x, y), before: None, after: None }
    }

    #[test]
    fn clicks_make_straight_lines_and_a_click_on_the_first_point_closes() {
        let mut draft = Draft::default();
        assert_eq!(draft.press(Point::new(10.0, 10.0), 4.0), Pressed::Placed);
        // One point can't be closed yet: a second press near it adds a point.
        assert!(!draft.closes(Point::new(11.0, 10.0), 4.0));
        assert_eq!(draft.press(Point::new(110.0, 10.0), 4.0), Pressed::Placed);
        assert_eq!(draft.press(Point::new(110.0, 60.0), 4.0), Pressed::Placed);
        assert_eq!(draft.press(Point::new(12.0, 11.0), 4.0), Pressed::Closes);
        assert_eq!(geometry(draft.anchors(), true, Point::new(10.0, 10.0)), "M0 0 L100 0 L100 50 Z");
    }

    #[test]
    fn a_pull_makes_a_smooth_point_with_mirrored_handles() {
        let mut draft = Draft::default();
        draft.press(Point::new(0.0, 0.0), 4.0);
        draft.press(Point::new(100.0, 0.0), 4.0);
        draft.pull(Point::new(130.0, 40.0), false);
        let last = draft.anchors()[1];
        assert_eq!((last.before, last.after), (Some(Point::new(70.0, -40.0)), Some(Point::new(130.0, 40.0))));
        draft.press(Point::new(200.0, 0.0), 4.0);
        assert_eq!(geometry(draft.anchors(), false, Point::default()), "M0 0 C0 0 70 -40 100 0 C130 40 200 0 200 0");
        // Pulled back onto the point it is a corner again.
        draft.pop();
        draft.pull(Point::new(100.0, 0.0), false);
        assert_eq!(draft.anchors()[1], corner(100.0, 0.0));
    }

    #[test]
    fn shift_holds_a_point_to_45_degree_lines() {
        let mut draft = Draft::default();
        draft.press(Point::new(0.0, 0.0), 4.0);
        let placed = draft.place(Point::new(100.0, 8.0), true);
        assert!((placed.y).abs() < 1e-3 && (placed.x - 100.0_f32.hypot(8.0)).abs() < 1e-3);
        let diagonal = constrain(Point::new(0.0, 0.0), Point::new(50.0, 40.0));
        assert!((diagonal.x - diagonal.y).abs() < 1e-3);
        assert_eq!(draft.place(Point::new(100.0, 8.0), false), Point::new(100.0, 8.0));
    }

    #[test]
    fn the_layer_sits_on_its_drawn_box_and_reads_back() {
        let anchors = [corner(20.0, 30.0), corner(120.0, 30.0), corner(120.0, 80.0)];
        let Some((node, origin)) = layer(&anchors, false, &Affine::IDENTITY, "p1".to_owned()) else {
            panic!("three points make a layer");
        };
        assert_eq!(origin, Point::new(20.0, 30.0));
        assert_eq!(node.geometry.as_deref(), Some("M0 0 L100 0 L100 50"));
        assert_eq!((node.width, node.height), (Some(Num::new(100.0)), Some(Num::new(50.0))));
        assert_eq!(node.fill, None);
        let read = path::parse(node.geometry.as_deref().unwrap_or_default());
        assert_eq!(read.map(|segments| segments.len()), Ok(3));
        // One point alone draws nothing.
        assert!(layer(&anchors[..1], false, &Affine::IDENTITY, "p2".to_owned()).is_none());
    }

    fn read(data: &str) -> Vec<Contour> {
        contours(&path::parse(data).unwrap())
    }

    #[test]
    fn path_data_reads_into_points_and_writes_back() {
        // A closed outline drawn back onto its start has three points, not four; a second outline follows.
        let read_back = read("M0 0 L100 0 C100 0 100 30 100 50 L0 0 Z M10 10 L20 20");
        assert_eq!((read_back.len(), read_back[0].anchors.len(), read_back[0].closed, read_back[1].closed), (2, 3, true, false));
        assert_eq!(read_back[0].anchors[2].before, Some(Point::new(100.0, 30.0)));
        assert_eq!(data(&read_back), "M0 0 L100 0 C100 0 100 30 100 50 Z M10 10 L20 20");
        // A quadratic curve is the same curve as a cubic one.
        assert_eq!(data(&read("M0 0 Q30 60 60 0")), "M0 0 C20 40 40 40 60 0");
    }

    #[test]
    fn points_move_bend_and_go() {
        let mut shape = read("M0 0 L100 0 L100 100 L0 100 Z");
        shift(&mut shape, &[(0, 1), (0, 2)], 10.0, 0.0);
        assert_eq!(data(&shape), "M0 0 L110 0 L110 100 L0 100 Z");
        // A sharp point made smooth gets handles along the line between its neighbours, a third of the way.
        toggle_smooth(&mut shape, (0, 0));
        let smooth = shape[0].anchors[0];
        assert!(is_smooth(&smooth), "{smooth:?}");
        // Pulling one handle of a smooth point turns the other, which keeps its length.
        let kept = distance(smooth.at, smooth.before.unwrap());
        bend(&mut shape, (0, 0), Side::After, Point::new(0.0, -60.0), true);
        let bent = shape[0].anchors[0];
        assert!((bent.before.unwrap().x).abs() < 1e-3 && (bent.before.unwrap().y - kept).abs() < 1e-3, "{bent:?}");
        // Made sharp again, the handles are gone.
        toggle_smooth(&mut shape, (0, 0));
        assert_eq!(shape[0].anchors[0], corner(0.0, 0.0));
        // Without two of its points the square is one open line; without one more it is gone.
        remove(&mut shape, &[(0, 0), (0, 3)]);
        assert_eq!(data(&shape), "M110 0 L110 100");
        remove(&mut shape, &[(0, 0)]);
        assert_eq!(shape, Vec::new());
    }

    #[test]
    fn a_point_is_added_where_the_pointer_is_without_changing_the_shape() {
        let mut shape = read("M0 0 C0 100 100 100 100 0 L100 -50");
        let before = cubic_at(&cubic(&shape[0].anchors[0], &shape[0].anchors[1]), 0.25);
        // The pointer is nearest the curve's middle, at its lowest place.
        let (contour, index, t, far) = nearest_stretch(&shape, &Affine::IDENTITY, Point::new(50.0, 80.0)).unwrap();
        assert!((contour, index) == (0, 0) && (t - 0.5).abs() < 0.03 && (far - 5.0).abs() < 0.1, "{t} {far}");
        assert_eq!(insert(&mut shape, 0, 0, 0.5), Some((0, 1)));
        assert_eq!(data(&shape), "M0 0 C0 50 25 75 50 75 C75 75 100 50 100 0 L100 -50");
        // The first half is the same curve as before: its middle is the old curve's quarter.
        let after = cubic_at(&cubic(&shape[0].anchors[0], &shape[0].anchors[1]), 0.5);
        assert!(distance(before, after) < 1e-3);
        // On a straight line the new point is a sharp one.
        assert_eq!(insert(&mut shape, 0, 2, 0.5), Some((0, 3)));
        assert_eq!(shape[0].anchors[3], corner(100.0, -25.0));
    }

    #[test]
    fn a_cut_opens_a_closed_outline_and_splits_an_open_one() {
        let mut shape = read("M0 0 L100 0 L100 100 L0 100 Z");
        assert!(cut(&mut shape, (0, 2)));
        assert_eq!(data(&shape), "M100 100 L0 100 L0 0 L100 0 L100 100");
        // The ends of an open line can't be cut; a point between them splits it in two.
        assert!(!cut(&mut shape, (0, 0)) && !cut(&mut shape, (0, 4)));
        assert!(cut(&mut shape, (0, 2)));
        assert_eq!(data(&shape), "M100 100 L0 100 L0 0 M0 0 L100 0 L100 100");
    }

    #[test]
    fn points_are_written_in_the_parent_frames_coordinates() {
        let anchors = [corner(120.0, 230.0), corner(170.0, 230.0)];
        let into_frame = Affine::translate(-100.0, -200.0);
        let placed = layer(&anchors, false, &into_frame, "p1".to_owned()).map(|(_, origin)| origin);
        assert_eq!(placed, Some(Point::new(20.0, 30.0)));
    }
}
