//! SVG path data (`geometry` of a `path` node) read into absolute drawing commands.

use crate::geom::{Point, Rect};

/// One absolute drawing command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Segment {
    /// Starts a new subpath.
    MoveTo(Point),
    /// A straight line.
    LineTo(Point),
    /// A quadratic Bézier: control, end.
    QuadTo(Point, Point),
    /// A cubic Bézier: two controls, end.
    CubicTo(Point, Point, Point),
    /// Closes the subpath back to its start.
    Close,
}

/// Why path data could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathError {
    /// Byte offset of the problem in the path data.
    pub offset: usize,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unreadable SVG path data at byte {}", self.offset)
    }
}

impl std::error::Error for PathError {}

/// Reads SVG path data (`M`, `L`, `H`, `V`, `C`, `S`, `Q`, `T`, `A`, `Z`, absolute and relative).
///
/// # Errors
/// [`PathError`] at the first command or number that can't be read; a path drawn up to there is not returned.
pub fn parse(data: &str) -> Result<Vec<Segment>, PathError> {
    let mut lexer = Lexer { bytes: data.as_bytes(), pos: 0 };
    let mut out = Vec::new();
    let mut current = Point::default();
    let mut start = Point::default();
    // The last control point, for the smooth `S` and `T` commands.
    let mut last_cubic: Option<Point> = None;
    let mut last_quad: Option<Point> = None;
    let mut command = None;
    while lexer.skip_separators() {
        if let Some(c) = lexer.command() {
            command = Some(c);
        }
        let Some(c) = command else { return Err(PathError { offset: lexer.pos }) };
        let relative = c.is_ascii_lowercase();
        let base = if relative { current } else { Point::default() };
        let at = |p: Point| Point::new(p.x + base.x, p.y + base.y);
        match c.to_ascii_uppercase() {
            b'M' => {
                current = at(lexer.point()?);
                start = current;
                out.push(Segment::MoveTo(current));
                // Human: numbers after a move are line-tos (SVG spec).
                command = Some(if relative { b'l' } else { b'L' });
                last_cubic = None;
                last_quad = None;
            }
            b'L' => {
                current = at(lexer.point()?);
                out.push(Segment::LineTo(current));
                (last_cubic, last_quad) = (None, None);
            }
            b'H' => {
                let x = lexer.number()?;
                current = Point::new(if relative { current.x + x } else { x }, current.y);
                out.push(Segment::LineTo(current));
                (last_cubic, last_quad) = (None, None);
            }
            b'V' => {
                let y = lexer.number()?;
                current = Point::new(current.x, if relative { current.y + y } else { y });
                out.push(Segment::LineTo(current));
                (last_cubic, last_quad) = (None, None);
            }
            b'C' => {
                let (c1, c2, end) = (at(lexer.point()?), at(lexer.point()?), at(lexer.point()?));
                out.push(Segment::CubicTo(c1, c2, end));
                current = end;
                (last_cubic, last_quad) = (Some(c2), None);
            }
            b'S' => {
                let c1 = last_cubic.map_or(current, |c| reflect(c, current));
                let (c2, end) = (at(lexer.point()?), at(lexer.point()?));
                out.push(Segment::CubicTo(c1, c2, end));
                current = end;
                (last_cubic, last_quad) = (Some(c2), None);
            }
            b'Q' => {
                let (c1, end) = (at(lexer.point()?), at(lexer.point()?));
                out.push(Segment::QuadTo(c1, end));
                current = end;
                (last_cubic, last_quad) = (None, Some(c1));
            }
            b'T' => {
                let c1 = last_quad.map_or(current, |c| reflect(c, current));
                let end = at(lexer.point()?);
                out.push(Segment::QuadTo(c1, end));
                current = end;
                (last_cubic, last_quad) = (None, Some(c1));
            }
            b'A' => {
                let (rx, ry, rotation) = (lexer.number()?, lexer.number()?, lexer.number()?);
                let (large, sweep) = (lexer.flag()?, lexer.flag()?);
                let end = at(lexer.point()?);
                arc_to_cubics(current, end, rx, ry, rotation, large, sweep, &mut out);
                current = end;
                (last_cubic, last_quad) = (None, None);
            }
            b'Z' => {
                out.push(Segment::Close);
                current = start;
                (last_cubic, last_quad) = (None, None);
                command = None;
            }
            _ => return Err(PathError { offset: lexer.pos }),
        }
    }
    Ok(out)
}

fn reflect(control: Point, about: Point) -> Point {
    Point::new(2.0 * about.x - control.x, 2.0 * about.y - control.y)
}

/// The tight bounding box of the drawn path (curves sampled), `None` for an empty path.
#[must_use]
pub fn bounds(segments: &[Segment]) -> Option<Rect> {
    let mut bounds: Option<Rect> = None;
    let mut add = |p: Point| {
        let r = Rect::new(p.x, p.y, 0.0, 0.0);
        bounds = Some(bounds.map_or(r, |b| b.union(&r)));
    };
    let mut current = Point::default();
    for segment in segments {
        match *segment {
            Segment::MoveTo(p) | Segment::LineTo(p) => {
                add(p);
                current = p;
            }
            Segment::QuadTo(c, p) => {
                for t in sample_steps() {
                    add(quad_at(current, c, p, t));
                }
                current = p;
            }
            Segment::CubicTo(c1, c2, p) => {
                for t in sample_steps() {
                    add(cubic_at(current, c1, c2, p, t));
                }
                current = p;
            }
            Segment::Close => {}
        }
    }
    bounds
}

/// t = 1/16 … 1, enough for a bounding box within a fraction of a unit on icon-sized curves.
/// The path as runs of straight lines, one per subpath, each curve cut into `steps` pieces.
#[must_use]
pub fn flatten(segments: &[Segment], steps: u8) -> Vec<Vec<Point>> {
    let mut runs: Vec<Vec<Point>> = Vec::new();
    let at = |i: u8| f32::from(i) / f32::from(steps.max(1));
    for segment in segments {
        let current = runs.last().and_then(|run| run.last()).copied().unwrap_or_default();
        match *segment {
            Segment::MoveTo(p) => runs.push(vec![p]),
            Segment::LineTo(p) => runs.last_mut().into_iter().for_each(|run| run.push(p)),
            Segment::QuadTo(c, p) => {
                runs.last_mut().into_iter().for_each(|run| run.extend((1..=steps.max(1)).map(|i| quad_at(current, c, p, at(i)))));
            }
            Segment::CubicTo(c1, c2, p) => {
                runs.last_mut().into_iter().for_each(|run| run.extend((1..=steps.max(1)).map(|i| cubic_at(current, c1, c2, p, at(i)))));
            }
            Segment::Close => {
                if let Some(run) = runs.last_mut()
                    && let Some(first) = run.first().copied()
                {
                    run.push(first);
                }
            }
        }
    }
    runs
}

fn sample_steps() -> impl Iterator<Item = f32> {
    (1..=16u8).map(|i| f32::from(i) / 16.0)
}

fn quad_at(p0: Point, c: Point, p1: Point, t: f32) -> Point {
    let u = 1.0 - t;
    Point::new(u * u * p0.x + 2.0 * u * t * c.x + t * t * p1.x, u * u * p0.y + 2.0 * u * t * c.y + t * t * p1.y)
}

#[expect(clippy::many_single_char_names, reason = "the Bézier formula's usual names")]
fn cubic_at(p0: Point, c1: Point, c2: Point, p1: Point, t: f32) -> Point {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Point::new(a * p0.x + b * c1.x + c * c2.x + d * p1.x, a * p0.y + b * c1.y + c * c2.y + d * p1.y)
}

/// An SVG elliptical arc as cubic Béziers (SVG implementation notes, appendix B.2.4).
#[expect(clippy::too_many_arguments, reason = "the arc's parameters are exactly the SVG command's")]
fn arc_to_cubics(from: Point, to: Point, rx: f32, ry: f32, rotation_deg: f32, large: bool, sweep: bool, out: &mut Vec<Segment>) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < f32::EPSILON || ry < f32::EPSILON || (from.x - to.x).abs() + (from.y - to.y).abs() < f32::EPSILON {
        out.push(Segment::LineTo(to));
        return;
    }
    let (sin, cos) = rotation_deg.to_radians().sin_cos();
    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let x1 = cos * dx + sin * dy;
    let y1 = -sin * dx + cos * dy;
    // Radii too small for the endpoints grow until they fit.
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let num = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut k = if den > 0.0 { (num / den).sqrt() } else { 0.0 };
    if large == sweep {
        k = -k;
    }
    let cx1 = k * rx * y1 / ry;
    let cy1 = -k * ry * x1 / rx;
    let cx = cos * cx1 - sin * cy1 + f32::midpoint(from.x, to.x);
    let cy = sin * cx1 + cos * cy1 + f32::midpoint(from.y, to.y);
    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let a = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
        if a.is_finite() { a } else { 0.0 }
    };
    let theta = angle(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
    if !sweep && delta > 0.0 {
        delta -= std::f32::consts::TAU;
    } else if sweep && delta < 0.0 {
        delta += std::f32::consts::TAU;
    }
    // One cubic per quarter turn at most.
    let pieces = (delta.abs() / std::f32::consts::FRAC_PI_2).ceil().max(1.0);
    let step = delta / pieces;
    let alpha = 4.0 / 3.0 * (step / 4.0).tan();
    let point = |t: f32| {
        let (s, c) = t.sin_cos();
        Point::new(cx + rx * c * cos - ry * s * sin, cy + rx * c * sin + ry * s * cos)
    };
    let derivative = |t: f32| {
        let (s, c) = t.sin_cos();
        Point::new(-rx * s * cos - ry * c * sin, -rx * s * sin + ry * c * cos)
    };
    let mut t = theta;
    let mut piece = 0.0;
    while piece < pieces {
        let next = t + step;
        let (p0, p1) = (point(t), if piece + 1.0 >= pieces { to } else { point(next) });
        let (d0, d1) = (derivative(t), derivative(next));
        out.push(Segment::CubicTo(Point::new(p0.x + alpha * d0.x, p0.y + alpha * d0.y), Point::new(p1.x - alpha * d1.x, p1.y - alpha * d1.y), p1));
        t = next;
        piece += 1.0;
    }
}

struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Lexer<'_> {
    /// Skips whitespace and commas; `false` at the end of the data.
    fn skip_separators(&mut self) -> bool {
        while let Some(&b) = self.bytes.get(self.pos) {
            if b.is_ascii_whitespace() || b == b',' {
                self.pos += 1;
            } else {
                return true;
            }
        }
        false
    }

    fn command(&mut self) -> Option<u8> {
        let b = *self.bytes.get(self.pos)?;
        if b.is_ascii_alphabetic() && b != b'e' && b != b'E' {
            self.pos += 1;
            return Some(b);
        }
        None
    }

    fn number(&mut self) -> Result<f32, PathError> {
        self.skip_separators();
        let start = self.pos;
        let mut seen_dot = false;
        let mut seen_exp = false;
        if matches!(self.bytes.get(self.pos), Some(b'-' | b'+')) {
            self.pos += 1;
        }
        while let Some(&b) = self.bytes.get(self.pos) {
            match b {
                b'0'..=b'9' => self.pos += 1,
                // Human: `0.5.5` is two numbers (0.5 and .5), as SVG writers compress them.
                b'.' if !seen_dot && !seen_exp => {
                    seen_dot = true;
                    self.pos += 1;
                }
                b'e' | b'E' if !seen_exp => {
                    seen_exp = true;
                    self.pos += 1;
                    if matches!(self.bytes.get(self.pos), Some(b'-' | b'+')) {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
        std::str::from_utf8(self.bytes.get(start..self.pos).unwrap_or_default())
            .ok()
            .and_then(|text| text.parse::<f32>().ok())
            .ok_or(PathError { offset: start })
    }

    /// An arc flag: a single `0` or `1`, which SVG lets touch the next number.
    fn flag(&mut self) -> Result<bool, PathError> {
        self.skip_separators();
        let flag = match self.bytes.get(self.pos) {
            Some(b'0') => false,
            Some(b'1') => true,
            _ => return Err(PathError { offset: self.pos }),
        };
        self.pos += 1;
        Ok(flag)
    }

    fn point(&mut self) -> Result<Point, PathError> {
        Ok(Point::new(self.number()?, self.number()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_arrow_of_relative_lines() {
        // An arrow: relative lines after a move.
        let path = parse("M7 0l0 14m7-7l-7 7-7-7").unwrap();
        assert_eq!(path.len(), 5);
        assert_eq!(path[1], Segment::LineTo(Point::new(7.0, 14.0)));
        assert_eq!(path[4], Segment::LineTo(Point::new(0.0, 7.0)));
        assert_eq!(bounds(&path), Some(Rect::new(0.0, 0.0, 14.0, 14.0)));
    }

    #[test]
    fn arcs_and_compressed_numbers() {
        let path = parse("M0 0a5 5 0 1010 0z M1.5.5L2-3").unwrap();
        assert!(path.iter().any(|s| matches!(s, Segment::CubicTo(..))));
        assert!(path.contains(&Segment::LineTo(Point::new(2.0, -3.0))));
        assert!(parse("M0 0 L x").is_err());
    }
}
