//! Points, boxes and 2D affine transforms in canvas units (y down).

/// A point or vector.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
    /// Horizontal, right is positive.
    pub x: f32,
    /// Vertical, down is positive.
    pub y: f32,
}

impl Point {
    /// A point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }
}

/// An axis-aligned box.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width, ≥ 0.
    pub w: f32,
    /// Height, ≥ 0.
    pub h: f32,
}

impl Rect {
    /// The empty box at the origin.
    pub const ZERO: Rect = Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };

    /// A box from its top-left corner and size.
    #[must_use]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    /// The box spanning two corners, in any order.
    #[must_use]
    pub fn from_corners(a: Point, b: Point) -> Rect {
        Rect::new(a.x.min(b.x), a.y.min(b.y), (a.x - b.x).abs(), (a.y - b.y).abs())
    }

    /// Right edge.
    #[must_use]
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    /// Bottom edge.
    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    /// Centre.
    #[must_use]
    pub fn center(&self) -> Point {
        Point::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// Whether `p` lies inside (edges included).
    #[must_use]
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x <= self.right() && p.y >= self.y && p.y <= self.bottom()
    }

    /// Whether the two boxes overlap.
    #[must_use]
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x <= other.right() && other.x <= self.right() && self.y <= other.bottom() && other.y <= self.bottom()
    }

    /// Whether `other` lies wholly inside (edges included).
    #[must_use]
    pub fn holds(&self, other: &Rect) -> bool {
        self.x <= other.x && self.y <= other.y && self.right() >= other.right() && self.bottom() >= other.bottom()
    }

    /// The same box moved by (`dx`, `dy`).
    #[must_use]
    pub fn translate(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    /// The smallest box holding both.
    #[must_use]
    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Rect::new(x, y, self.right().max(other.right()) - x, self.bottom().max(other.bottom()) - y)
    }

    /// The four corners, clockwise from top-left.
    #[must_use]
    pub fn corners(&self) -> [Point; 4] {
        [Point::new(self.x, self.y), Point::new(self.right(), self.y), Point::new(self.right(), self.bottom()), Point::new(self.x, self.bottom())]
    }
}

/// A 2D affine transform `[a, b, c, d, tx, ty]`: x' = a·x + c·y + tx, y' = b·x + d·y + ty.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    /// The matrix, in the order the `.pen` schema writes `AffineTransform`.
    pub m: [f32; 6],
}

impl Affine {
    /// No change.
    pub const IDENTITY: Affine = Affine { m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] };

    /// A move.
    #[must_use]
    pub fn translate(x: f32, y: f32) -> Affine {
        Affine { m: [1.0, 0.0, 0.0, 1.0, x, y] }
    }

    /// A rotation by `degrees` counter-clockwise as seen on screen (y down), around the origin.
    // Human: The schema's rotation is counter-clockwise; with y pointing down that is a negative angle in the
    // usual maths convention.
    #[must_use]
    pub fn rotate_ccw(degrees: f32) -> Affine {
        let (sin, cos) = (-degrees.to_radians()).sin_cos();
        Affine { m: [cos, sin, -sin, cos, 0.0, 0.0] }
    }

    /// A scale around the origin.
    #[must_use]
    pub fn scale(x: f32, y: f32) -> Affine {
        Affine { m: [x, 0.0, 0.0, y, 0.0, 0.0] }
    }

    /// `self` after `inner`: applies `inner` first, then `self`.
    #[must_use]
    #[expect(clippy::many_single_char_names, reason = "a, b, c, d, e, f are the matrix entries' standard names")]
    pub fn then(&self, inner: &Affine) -> Affine {
        let [a, b, c, d, e, f] = self.m;
        let [a2, b2, c2, d2, e2, f2] = inner.m;
        Affine { m: [a * a2 + c * b2, b * a2 + d * b2, a * c2 + c * d2, b * c2 + d * d2, a * e2 + c * f2 + e, b * e2 + d * f2 + f] }
    }

    /// Applies the transform to a point.
    #[must_use]
    #[expect(clippy::many_single_char_names, reason = "a, b, c, d, e, f are the matrix entries' standard names")]
    pub fn apply(&self, p: Point) -> Point {
        let [a, b, c, d, e, f] = self.m;
        Point::new(a * p.x + c * p.y + e, b * p.x + d * p.y + f)
    }

    /// The inverse, or `None` when the transform squashes everything flat.
    #[must_use]
    #[expect(clippy::many_single_char_names, reason = "a, b, c, d, e, f are the matrix entries' standard names")]
    pub fn invert(&self) -> Option<Affine> {
        let [a, b, c, d, e, f] = self.m;
        let det = a * d - b * c;
        if det.abs() < 1e-12 {
            return None;
        }
        let inv = 1.0 / det;
        Some(Affine { m: [d * inv, -b * inv, -c * inv, a * inv, (c * f - d * e) * inv, (b * e - a * f) * inv] })
    }

    /// Whether the transform keeps edges horizontal and vertical (no rotation or skew).
    #[must_use]
    pub fn is_axis_aligned(&self) -> bool {
        self.m[1].abs() < 1e-6 && self.m[2].abs() < 1e-6
    }

    /// The bounding box of `rect` after the transform.
    #[must_use]
    pub fn bounds(&self, rect: &Rect) -> Rect {
        let [p0, p1, p2, p3] = rect.corners().map(|p| self.apply(p));
        let min_x = p0.x.min(p1.x).min(p2.x).min(p3.x);
        let min_y = p0.y.min(p1.y).min(p2.y).min(p3.y);
        let max_x = p0.x.max(p1.x).max(p2.x).max(p3.x);
        let max_y = p0.y.max(p1.y).max(p2.y).max(p3.y);
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_is_counter_clockwise_on_screen() {
        // A point to the right of the origin rotated 90° CCW ends up above it (negative y).
        let p = Affine::rotate_ccw(90.0).apply(Point::new(10.0, 0.0));
        assert!(p.x.abs() < 1e-4 && (p.y + 10.0).abs() < 1e-4);
    }

    #[test]
    fn inverse_undoes_the_transform() {
        let t = Affine::translate(5.0, 7.0).then(&Affine::rotate_ccw(30.0)).then(&Affine::scale(2.0, -1.0));
        let p = Point::new(3.0, 4.0);
        let back = t.invert().unwrap().apply(t.apply(p));
        assert!((back.x - p.x).abs() < 1e-4 && (back.y - p.y).abs() < 1e-4);
    }
}
