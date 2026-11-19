//! Snapping to objects: a box that is moved or resized near another layer's edge or middle lands on it, and
//! says so with a guide line.
// Human: lining a layer up with its neighbours is most of what moving a layer is for. Within a few screen
// points of a neighbour's edge or middle the box goes the rest of the way itself, and a line through both
// shows what it lined up with.
// Agent: PURE; no window, no document. Everything is in document coordinates; the caller gives the reach.

use pen_doc::geom::Rect;

/// How close two lines are to count as one, in document units.
const SAME: f32 = 0.01;

/// A line drawn while a box is snapped, in document coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Guide {
    /// The line runs down the screen at an x (`true`) or across it at a y.
    pub vertical: bool,
    /// The x of a vertical line, the y of a horizontal one.
    pub at: f32,
    /// Where the line starts along its own direction.
    pub from: f32,
    /// Where the line ends along its own direction.
    pub to: f32,
}

/// Which of a box's three lines along one axis may snap: its start (left or top), its middle, its end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lines {
    /// The left or top edge.
    pub start: bool,
    /// The middle.
    pub middle: bool,
    /// The right or bottom edge.
    pub end: bool,
}

impl Lines {
    /// Every line: a box that is moved.
    pub const ALL: Lines = Lines { start: true, middle: true, end: true };

    /// The one edge a resize pulls: the start for −1, the end for 1, none for 0.
    #[must_use]
    pub fn pulled(side: i8) -> Lines {
        Lines { start: side < 0, middle: false, end: side > 0 }
    }
}

/// How far a box goes to line up, and the lines to draw for it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snap {
    /// How far the box (or its pulled edge) goes along x.
    pub dx: f32,
    /// How far along y.
    pub dy: f32,
    /// The lines through what it lined up with.
    pub guides: Vec<Guide>,
    /// The gaps that are equal now, each as a line across it: from one layer's side to the next one's.
    pub gaps: Vec<Guide>,
}

/// One axis of a box: its start, middle and end, and its span along the other axis.
#[derive(Clone, Copy)]
struct Side {
    lines: [f32; 3],
    across: (f32, f32),
}

fn side(rect: &Rect, vertical: bool) -> Side {
    if vertical {
        Side { lines: [rect.x, rect.x + rect.w / 2.0, rect.right()], across: (rect.y, rect.bottom()) }
    } else {
        Side { lines: [rect.y, rect.y + rect.h / 2.0, rect.bottom()], across: (rect.x, rect.right()) }
    }
}

/// The lines of `moving` and of a target that may meet: an edge meets an edge, a middle a middle.
// Human: an edge that caught on every neighbour's middle too would make a box hard to place freely; edges
// belong with edges and middles with middles.
fn pairs(lines: Lines) -> impl Iterator<Item = (usize, usize)> {
    let take = [lines.start, lines.middle, lines.end];
    [(0, 0), (0, 2), (1, 1), (2, 0), (2, 2)].into_iter().filter(move |(mine, _)| take[*mine])
}

/// How far `moving` goes along one axis to the nearest target line within `reach`, if any.
fn shift(moving: &Rect, targets: &[Rect], reach: f32, lines: Lines, vertical: bool) -> Option<f32> {
    let mine = side(moving, vertical);
    let mut best: Option<f32> = None;
    for target in targets {
        let theirs = side(target, vertical);
        for (m, t) in pairs(lines) {
            let d = theirs.lines[t] - mine.lines[m];
            if d.abs() <= reach && best.is_none_or(|b| d.abs() < b.abs()) {
                best = Some(d);
            }
        }
    }
    best
}

/// The guide lines along one axis for `placed`, the box where it now is: one for each of its lines that lies on
/// a target's, as long as the box and those targets together.
fn guides(placed: &Rect, targets: &[Rect], lines: Lines, vertical: bool, into: &mut Vec<Guide>) {
    let mine = side(placed, vertical);
    for (m, at) in mine.lines.iter().enumerate() {
        let mut span: Option<(f32, f32)> = None;
        for target in targets {
            let theirs = side(target, vertical);
            if pairs(lines).any(|(a, t)| a == m && (theirs.lines[t] - at).abs() <= SAME) {
                let (from, to) = span.unwrap_or(mine.across);
                span = Some((from.min(theirs.across.0), to.max(theirs.across.1)));
            }
        }
        if let Some((from, to)) = span {
            into.push(Guide { vertical, at: *at, from, to });
        }
    }
}

/// The line across the gap from the end of `a` to the start of `b`, half way up what the two share.
fn gap(a: &Side, b: &Side, vertical: bool) -> Guide {
    let at = f32::midpoint(a.across.0.max(b.across.0), a.across.1.min(b.across.1));
    Guide { vertical: !vertical, at, from: a.lines[2], to: b.lines[0] }
}

/// How far `moving` goes along one axis to stand at equal gaps with the layers in its row (or column), if such
/// a place is within `reach`, and the gaps that are then equal.
// Human: three things in a row want the same room between them. A layer moved between two neighbours lands
// half way; moved to the end of a row it lands as far from the last one as that one is from the one before.
// Only layers that lie beside it count (they share some of its height, for a row), and only the nearest on
// each side with the one behind it; the frame around it is no neighbour.
fn spacing(moving: &Rect, targets: &[Rect], reach: f32, vertical: bool) -> Option<(f32, Vec<Guide>)> {
    let mine = side(moving, vertical);
    let (start, width) = (mine.lines[0], mine.lines[2] - mine.lines[0]);
    let beside: Vec<Side> =
        targets.iter().map(|target| side(target, vertical)).filter(|t| t.across.0 < mine.across.1 && t.across.1 > mine.across.0).collect();
    let before = |edge: f32| beside.iter().filter(|t| t.lines[2] <= edge).copied().max_by(|a, b| a.lines[2].total_cmp(&b.lines[2]));
    let after = |edge: f32| beside.iter().filter(|t| t.lines[0] >= edge).copied().min_by(|a, b| a.lines[0].total_cmp(&b.lines[0]));
    let placed = |at: f32| Side { lines: [at, at + width / 2.0, at + width], across: mine.across };
    let (left, right) = (before(start + reach), after(mine.lines[2] - reach));
    // Each place it could stand at, with the two gaps that are equal there.
    let mut offers: Vec<(f32, [(Side, Side); 2])> = Vec::new();
    if let (Some(left), Some(right)) = (left, right) {
        let room = right.lines[0] - left.lines[2] - width;
        if room > 0.0 {
            let at = left.lines[2] + room / 2.0;
            offers.push((at, [(left, placed(at)), (placed(at), right)]));
        }
    }
    if let Some(left) = left
        && let Some(far) = before(left.lines[0])
        && left.lines[0] > far.lines[2]
    {
        let at = left.lines[2] + (left.lines[0] - far.lines[2]);
        offers.push((at, [(far, left), (left, placed(at))]));
    }
    if let Some(right) = right
        && let Some(far) = after(right.lines[2])
        && far.lines[0] > right.lines[2]
    {
        let at = right.lines[0] - (far.lines[0] - right.lines[2]) - width;
        offers.push((at, [(placed(at), right), (right, far)]));
    }
    let (at, gaps) =
        offers.into_iter().filter(|(at, _)| (at - start).abs() <= reach).min_by(|a, b| (a.0 - start).abs().total_cmp(&(b.0 - start).abs()))?;
    Some((at - start, gaps.iter().map(|(a, b)| gap(a, b, vertical)).collect()))
}

/// The shorter of two ways to go; the first when they are as long.
fn nearer(first: Option<f32>, second: Option<f32>) -> Option<f32> {
    match (first, second) {
        (Some(a), Some(b)) => Some(if b.abs() < a.abs() { b } else { a }),
        (a, b) => a.or(b),
    }
}

/// Lines `moving` up with the nearest of `targets` within `reach` on each axis. `xs` and `ys` say which of its
/// lines take part: all of them for a move ([`Lines::ALL`]), the pulled edge for a resize ([`Lines::pulled`]).
/// With `resizing` the shift moves only the pulled edges, so the guides are worked out for the box as resized.
/// A moved box also lands where its gaps to the layers beside it are equal ([`spacing`]); an edge or middle as
/// near wins.
#[must_use]
pub fn line_up(moving: &Rect, targets: &[Rect], reach: f32, xs: Lines, ys: Lines, resizing: bool) -> Snap {
    let spaced = |vertical: bool| if resizing { None } else { spacing(moving, targets, reach, vertical).map(|(d, _)| d) };
    let dx = nearer(shift(moving, targets, reach, xs, true), spaced(true)).unwrap_or(0.0);
    let dy = nearer(shift(moving, targets, reach, ys, false), spaced(false)).unwrap_or(0.0);
    let placed = if resizing { pulled(moving, xs, ys, dx, dy) } else { moving.translate(dx, dy) };
    let mut lines = Vec::new();
    guides(&placed, targets, xs, true, &mut lines);
    guides(&placed, targets, ys, false, &mut lines);
    // The gaps that are equal where the box now stands, whichever way it got there.
    let mut gaps = Vec::new();
    if !resizing {
        for vertical in [true, false] {
            gaps.extend(spacing(&placed, targets, SAME, vertical).into_iter().flat_map(|(_, gaps)| gaps));
        }
    }
    Snap { dx, dy, guides: lines, gaps }
}

/// `rect` with its pulled edges moved by `dx` and `dy`, the opposite ones staying.
#[must_use]
pub fn pulled(rect: &Rect, xs: Lines, ys: Lines, dx: f32, dy: f32) -> Rect {
    let (mut left, mut right, mut top, mut bottom) = (rect.x, rect.right(), rect.y, rect.bottom());
    if xs.start {
        left += dx;
    }
    if xs.end {
        right += dx;
    }
    if ys.start {
        top += dy;
    }
    if ys.end {
        bottom += dy;
    }
    Rect::new(left.min(right), top.min(bottom), (right - left).abs(), (bottom - top).abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moved_box_lands_on_the_nearest_edge_within_reach_and_is_left_alone_beyond_it() {
        let target = [Rect::new(100.0, 100.0, 50.0, 50.0)];
        // Its left edge is 3 off the target's left edge, its top 40 away from anything.
        let near = line_up(&Rect::new(103.0, 240.0, 20.0, 20.0), &target, 5.0, Lines::ALL, Lines::ALL, false);
        assert!((near.dx + 3.0).abs() < 1e-4 && near.dy.abs() < 1e-4, "{near:?}");
        // One line, down the shared left edge, as long as both boxes together.
        assert_eq!(near.guides, vec![Guide { vertical: true, at: 100.0, from: 100.0, to: 260.0 }]);
        let far = line_up(&Rect::new(108.0, 240.0, 20.0, 20.0), &target, 5.0, Lines::ALL, Lines::ALL, false);
        assert_eq!(far, Snap::default());
    }

    #[test]
    fn middles_meet_middles_and_edges_meet_edges_on_both_axes() {
        let target = [Rect::new(0.0, 0.0, 100.0, 100.0)];
        // A box 20 square whose middle is 2 right of and 1 above the target's middle.
        let got = line_up(&Rect::new(42.0, 39.0, 20.0, 20.0), &target, 5.0, Lines::ALL, Lines::ALL, false);
        assert!((got.dx + 2.0).abs() < 1e-4 && (got.dy - 1.0).abs() < 1e-4, "{got:?}");
        assert_eq!(got.guides.len(), 2);
        assert!(got.guides.iter().any(|g| g.vertical && (g.at - 50.0).abs() < 1e-4));
        assert!(got.guides.iter().any(|g| !g.vertical && (g.at - 50.0).abs() < 1e-4));
        // An edge doesn't catch on a middle: the box's left edge 1 off the target's middle stays.
        let edge = line_up(&Rect::new(51.0, 300.0, 20.0, 20.0), &target, 5.0, Lines::ALL, Lines::ALL, false);
        assert!(edge.dx.abs() < 1e-4, "{edge:?}");
    }

    #[test]
    fn the_nearer_of_two_neighbours_wins_and_a_shared_line_runs_past_both() {
        let targets = [Rect::new(0.0, 0.0, 40.0, 40.0), Rect::new(44.0, 400.0, 40.0, 40.0), Rect::new(0.0, 800.0, 40.0, 40.0)];
        // The right edge at 43 is 3 from the first's right edge (40) and 1 from the second's left edge (44).
        let got = line_up(&Rect::new(23.0, 200.0, 20.0, 20.0), &targets, 5.0, Lines::ALL, Lines::ALL, false);
        assert!((got.dx - 1.0).abs() < 1e-4, "{got:?}");
        // On the left edges of the first and the third at once: one line from the top of one to the foot of the other.
        let both = line_up(&Rect::new(2.0, 200.0, 20.0, 20.0), &targets, 5.0, Lines::ALL, Lines::ALL, false);
        assert_eq!(both.guides, vec![Guide { vertical: true, at: 0.0, from: 0.0, to: 840.0 }]);
    }

    #[test]
    fn a_box_moved_between_two_neighbours_or_to_the_end_of_a_row_lands_at_equal_gaps() {
        // Two boxes in a row 300 apart, and a frame around everything.
        let row = [Rect::new(0.0, 0.0, 100.0, 100.0), Rect::new(400.0, 20.0, 100.0, 100.0), Rect::new(-50.0, -50.0, 1000.0, 600.0)];
        // A box 60 wide between them at 223 would have 123 before it and 117 after: it goes 3 left to 120 each.
        let between = line_up(&Rect::new(223.0, 41.0, 60.0, 30.0), &row, 5.0, Lines::ALL, Lines::ALL, false);
        assert!((between.dx + 3.0).abs() < 1e-4 && between.dy.abs() < 1e-4, "{between:?}");
        assert_eq!(between.gaps.len(), 2);
        assert!(between.gaps.iter().all(|g| !g.vertical && (g.to - g.from - 120.0).abs() < 1e-3), "{:?}", between.gaps);
        // Each mark lies half way up what the two layers share: 41 to 71 with the first, the same with the second.
        assert!(between.gaps.iter().all(|g| (g.at - 56.0).abs() < 1e-3), "{:?}", between.gaps);
        // Past the second box at 803, 303 from it: it goes to 800, as far from it as that is from the first.
        let after = line_up(&Rect::new(803.0, 41.0, 60.0, 30.0), &row, 5.0, Lines::ALL, Lines::ALL, false);
        assert!((after.dx + 3.0).abs() < 1e-4, "{after:?}");
        assert_eq!(after.gaps.iter().map(|g| (g.from, g.to)).collect::<Vec<_>>(), vec![(100.0, 400.0), (500.0, 800.0)]);
        // A box that shares none of their height is in no row with them.
        let below = line_up(&Rect::new(223.0, 300.0, 60.0, 30.0), &row, 5.0, Lines::ALL, Lines::ALL, false);
        assert!(below.dx.abs() < 1e-4 && below.gaps.is_empty(), "{below:?}");
        // An edge as near as the equal gap wins, and a resize knows no gaps.
        let pulled = line_up(&Rect::new(223.0, 41.0, 60.0, 30.0), &row, 5.0, Lines::pulled(1), Lines::pulled(0), true);
        assert_eq!(pulled, Snap::default());
    }

    #[test]
    fn a_resize_snaps_only_the_pulled_edge() {
        let target = [Rect::new(200.0, 0.0, 50.0, 50.0)];
        // Pulling the right edge (at 198) 2 short of the target's left edge: it goes there, the left edge stays.
        let moving = Rect::new(100.0, 100.0, 98.0, 40.0);
        let got = line_up(&moving, &target, 5.0, Lines::pulled(1), Lines::pulled(0), true);
        assert!((got.dx - 2.0).abs() < 1e-4 && got.dy.abs() < 1e-4, "{got:?}");
        assert_eq!(got.guides, vec![Guide { vertical: true, at: 200.0, from: 0.0, to: 140.0 }]);
        // The left edge, not pulled, lies 1 off nothing and takes no part even when a target is there.
        let other = [Rect::new(101.0, 300.0, 10.0, 10.0)];
        assert_eq!(line_up(&moving, &other, 5.0, Lines::pulled(1), Lines::pulled(0), true), Snap::default());
    }
}
