//! Pattern fills: a mark (a dot, a line) repeated across a layer at a fixed spacing, for the dot grid of a
//! canvas or the ruling of a sheet, without a layer for every mark.
//!
//! The marks are cut to the layer's own shape, whatever it is: the shape arrives here as the triangles it was
//! filled with, and every mark is clipped against each of them.

use egui::Pos2;
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use lyon_tessellation::VertexBuffers;
use pen_doc::color::Rgba;
use pen_doc::geom::{Affine, Point};
use serde_json::Value;

use super::color32;

/// The most marks one fill draws; a denser pattern is thinned to every second, third, ... mark.
// Human: marks this close can't be told apart at any size the canvas shows whole, and would only slow drawing.
const MOST_MARKS: f32 = 20_000.0;
/// The closest marks may lie, in canvas units.
const LEAST_SPACING: f32 = 2.0;

/// What is repeated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mark {
    /// A round dot, `size` across.
    Dots,
    /// Lines both ways, `size` thick.
    Grid,
    /// Lines from left to right.
    Rows,
    /// Lines from top to bottom.
    Columns,
}

/// A `{"type":"pattern",..}` paint, read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Pattern {
    pub mark: Mark,
    pub color: Rgba,
    /// A dot's diameter or a line's thickness.
    pub size: f32,
    /// The distance from one mark to the next, across and down.
    pub spacing: Point,
    /// Where the first mark is, from the layer's top left corner.
    pub offset: Point,
}

impl Pattern {
    /// Reads the paint; `color` is its colour with its opacity, `number` reads a number or a `$variable`.
    // Human: `spacing` is one number or `[across, down]`; without an `offset` the first mark sits half a
    // spacing in, so the pattern is centred in its cells and no mark is cut in half by the layer's edge.
    pub fn read(paint: &serde_json::Map<String, Value>, color: Rgba, number: &dyn Fn(Option<&Value>) -> Option<f32>) -> Pattern {
        let mark = match (paint.get("shape").and_then(Value::as_str), paint.get("direction").and_then(Value::as_str)) {
            (Some("grid"), _) => Mark::Grid,
            (Some("lines"), Some("vertical")) => Mark::Columns,
            (Some("lines"), _) => Mark::Rows,
            _ => Mark::Dots,
        };
        let spacing = match paint.get("spacing") {
            Some(Value::Array(both)) => Point::new(number(both.first()).unwrap_or(24.0), number(both.get(1)).unwrap_or(24.0)),
            one => {
                let one = number(one).unwrap_or(24.0);
                Point::new(one, one)
            }
        };
        let spacing = Point::new(spacing.x.max(LEAST_SPACING), spacing.y.max(LEAST_SPACING));
        let offset = paint.get("offset").map_or(Point::new(spacing.x / 2.0, spacing.y / 2.0), |offset| {
            Point::new(number(offset.get("x")).unwrap_or(0.0), number(offset.get("y")).unwrap_or(0.0))
        });
        let fallback = if mark == Mark::Dots { 2.0 } else { 1.0 };
        Pattern { mark, color, size: number(paint.get("size")).unwrap_or(fallback).max(0.0), spacing, offset }
    }

    /// The spacing actually drawn in a `w` × `h` box: the pattern's own, or a multiple of it when that would
    /// be more than [`MOST_MARKS`].
    fn drawn_spacing(&self, w: f32, h: f32) -> Point {
        let marks = (w / self.spacing.x) * (h / self.spacing.y);
        let thin = (marks / MOST_MARKS).sqrt().ceil().max(1.0);
        Point::new(self.spacing.x * thin, self.spacing.y * thin)
    }

    /// The marks in a `w` × `h` box as convex outlines, before they are cut to the shape.
    fn marks(&self, w: f32, h: f32, tolerance: f32) -> Vec<Vec<[f32; 2]>> {
        let spacing = self.drawn_spacing(w, h);
        let half = self.size / 2.0;
        // The positions of the marks along one axis: the first at or after the start, to the end.
        let along = |offset: f32, step: f32, length: f32| {
            let mut at = offset.rem_euclid(step);
            if at - half > 0.0 && at - step + half > 0.0 {
                at -= step;
            }
            std::iter::from_fn(move || {
                let here = at;
                at += step;
                (here - half < length).then_some(here)
            })
        };
        let mut out = Vec::new();
        if self.size <= 0.0 {
            return out;
        }
        match self.mark {
            Mark::Dots => {
                // As many corners as it takes for the dot to look round at this zoom.
                let sides = if half > tolerance { (std::f32::consts::PI / (1.0 - tolerance / half).acos()).ceil().clamp(6.0, 32.0) } else { 6.0 };
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to 6..=32 just above")]
                let sides = sides as usize;
                #[expect(clippy::cast_precision_loss, reason = "at most 32")]
                let ring: Vec<(f32, f32)> = (0..sides).map(|i| (i as f32 / sides as f32 * std::f32::consts::TAU).sin_cos()).collect();
                for y in along(self.offset.y, spacing.y, h) {
                    for x in along(self.offset.x, spacing.x, w) {
                        out.push(ring.iter().map(|(sin, cos)| [x + cos * half, y + sin * half]).collect());
                    }
                }
            }
            Mark::Grid | Mark::Rows | Mark::Columns => {
                if self.mark != Mark::Columns {
                    out.extend(along(self.offset.y, spacing.y, h).map(|y| vec![[0.0, y - half], [w, y - half], [w, y + half], [0.0, y + half]]));
                }
                if self.mark != Mark::Rows {
                    out.extend(along(self.offset.x, spacing.x, w).map(|x| vec![[x - half, 0.0], [x + half, 0.0], [x + half, h], [x - half, h]]));
                }
            }
        }
        out
    }

    /// The pattern as triangles, cut to the shape that `buffers` holds the triangles of.
    // Agent: `buffers` is the layer's filled shape in its own coordinates; RETURNS a mesh in document
    // coordinates. Work is (marks × triangles) box tests, and a clip only where the boxes overlap.
    pub fn mesh(&self, buffers: &VertexBuffers<[f32; 2], u32>, size: (f32, f32), tolerance: f32, to_doc: &Affine, opacity: f32) -> Mesh {
        let color = color32(self.color.with_opacity(opacity));
        let mut mesh = Mesh::default();
        let triangles: Vec<[[f32; 2]; 3]> = buffers
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .filter_map(|i| {
                Some([*buffers.vertices.get(i[0] as usize)?, *buffers.vertices.get(i[1] as usize)?, *buffers.vertices.get(i[2] as usize)?])
            })
            .collect();
        let boxes: Vec<[f32; 4]> = triangles.iter().map(|triangle| bounds(triangle)).collect();
        for mark in self.marks(size.0, size.1, tolerance) {
            let around = bounds(&mark);
            for (triangle, limits) in triangles.iter().zip(&boxes) {
                if around[0] > limits[2] || around[2] < limits[0] || around[1] > limits[3] || around[3] < limits[1] {
                    continue;
                }
                let piece = clip(&mark, triangle);
                if piece.len() < 3 {
                    continue;
                }
                let first = u32::try_from(mesh.vertices.len()).unwrap_or(0);
                for [x, y] in &piece {
                    let p = to_doc.apply(Point::new(*x, *y));
                    mesh.vertices.push(Vertex { pos: Pos2::new(p.x, p.y), uv: WHITE_UV, color });
                }
                for i in 1..u32::try_from(piece.len()).unwrap_or(0) - 1 {
                    mesh.indices.extend_from_slice(&[first, first + i, first + i + 1]);
                }
            }
        }
        mesh
    }

    /// The pattern as an SVG `<pattern>` body in the layer's own coordinates: its attributes and its marks.
    pub fn svg(&self, id: &str, n: &dyn Fn(f32) -> String) -> String {
        let (w, h, half) = (self.spacing.x, self.spacing.y, self.size / 2.0);
        let paint =
            format!("fill=\"#{:02x}{:02x}{:02x}\" fill-opacity=\"{}\"", self.color.r, self.color.g, self.color.b, n(f32::from(self.color.a) / 255.0));
        let row = format!("<rect x=\"0\" y=\"{}\" width=\"{}\" height=\"{}\" {paint}/>", n(h / 2.0 - half), n(w), n(self.size));
        let column = format!("<rect x=\"{}\" y=\"0\" width=\"{}\" height=\"{}\" {paint}/>", n(w / 2.0 - half), n(self.size), n(h));
        let marks = match self.mark {
            Mark::Dots => format!("<circle cx=\"{}\" cy=\"{}\" r=\"{}\" {paint}/>", n(w / 2.0), n(h / 2.0), n(half)),
            Mark::Grid => format!("{row}{column}"),
            Mark::Rows => row,
            Mark::Columns => column,
        };
        // The tile holds one mark in its middle, so it starts half a spacing before the first mark.
        format!(
            "<pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\">{marks}</pattern>",
            n(self.offset.x - w / 2.0),
            n(self.offset.y - h / 2.0),
            n(w),
            n(h)
        )
    }
}

/// `[left, top, right, bottom]` of some points.
fn bounds(points: &[[f32; 2]]) -> [f32; 4] {
    points.iter().fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, p| [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])])
}

/// The part of a convex outline that lies inside a triangle (Sutherland–Hodgman).
fn clip(outline: &[[f32; 2]], triangle: &[[f32; 2]; 3]) -> Vec<[f32; 2]> {
    let [a, b, c] = *triangle;
    // Which way the triangle winds decides which side of an edge is inside.
    let winding = ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).signum();
    let mut piece = outline.to_vec();
    for (from, to) in [(a, b), (b, c), (c, a)] {
        let side = |p: [f32; 2]| ((to[0] - from[0]) * (p[1] - from[1]) - (to[1] - from[1]) * (p[0] - from[0])) * winding;
        let mut kept = Vec::with_capacity(piece.len() + 1);
        for (i, &here) in piece.iter().enumerate() {
            let next = piece[(i + 1) % piece.len()];
            let (s0, s1) = (side(here), side(next));
            if s0 >= 0.0 {
                kept.push(here);
            }
            if (s0 >= 0.0) != (s1 >= 0.0) {
                let t = s0 / (s0 - s1);
                kept.push([here[0] + (next[0] - here[0]) * t, here[1] + (next[1] - here[1]) * t]);
            }
        }
        piece = kept;
        if piece.len() < 3 {
            break;
        }
    }
    piece
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(json: &str) -> Pattern {
        let paint: serde_json::Map<String, Value> = serde_json::from_str(json).unwrap();
        Pattern::read(&paint, Rgba::WHITE, &|value| value.and_then(Value::as_f64).map(pen_doc::value::narrow))
    }

    /// A `w` × `h` box as the two triangles it is filled with.
    fn sheet(w: f32, h: f32) -> VertexBuffers<[f32; 2], u32> {
        let mut buffers = VertexBuffers::new();
        buffers.vertices = vec![[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]];
        buffers.indices = vec![0, 1, 2, 0, 2, 3];
        buffers
    }

    fn area(mesh: &Mesh) -> f32 {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|i| {
                let [a, b, c] = [mesh.vertices[i[0] as usize].pos, mesh.vertices[i[1] as usize].pos, mesh.vertices[i[2] as usize].pos];
                ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() / 2.0
            })
            .sum()
    }

    #[test]
    fn dots_sit_in_the_middle_of_their_cells_and_lines_run_the_whole_way() {
        let dots = pattern(r#"{"type":"pattern","shape":"dots","spacing":20,"size":4}"#);
        let marks = dots.marks(100.0, 60.0, 0.25);
        assert_eq!(marks.len(), 15);
        let first = bounds(&marks[0]);
        assert!((first[0] - 8.0).abs() < 0.3 && (first[2] - 12.0).abs() < 0.3);
        let rows = pattern(r#"{"type":"pattern","shape":"lines","spacing":[20,10],"size":1}"#);
        assert_eq!(rows.marks(100.0, 60.0, 0.25).len(), 6);
        let grid = pattern(r#"{"type":"pattern","shape":"grid","spacing":[20,10],"size":1,"offset":{"x":0,"y":0}}"#);
        // Lines at 0, 10, ... 60 down and at 0, 20, ... 100 across; those on the edge are there, cut in half.
        assert_eq!(grid.marks(100.0, 60.0, 0.25).len(), 7 + 6);
        assert!((area(&grid.mesh(&sheet(100.0, 60.0), (100.0, 60.0), 0.25, &Affine::IDENTITY, 1.0)) - (6.0 * 100.0 + 5.0 * 60.0)).abs() < 0.01);
    }

    #[test]
    fn marks_are_cut_to_the_shape_and_a_dense_pattern_is_thinned() {
        let rows = pattern(r#"{"type":"pattern","shape":"lines","spacing":10,"size":2}"#);
        // A triangle that is half the box holds half of every line.
        let mut half = sheet(100.0, 100.0);
        half.indices.truncate(3);
        let (whole, cut) = (
            rows.mesh(&sheet(100.0, 100.0), (100.0, 100.0), 0.25, &Affine::IDENTITY, 1.0),
            rows.mesh(&half, (100.0, 100.0), 0.25, &Affine::IDENTITY, 1.0),
        );
        assert!((area(&whole) - 2000.0).abs() < 0.01 && (area(&cut) - 1000.0).abs() < 0.01);
        let dense = pattern(r#"{"type":"pattern","shape":"dots","spacing":2,"size":1}"#);
        assert!(dense.marks(4000.0, 4000.0, 0.25).len() <= 20_100);
    }

    #[test]
    fn an_svg_pattern_tiles_one_mark_from_half_a_spacing_before_the_first() {
        let dots = pattern(r#"{"type":"pattern","spacing":24,"size":2}"#);
        let text = dots.svg("p1", &|v| format!("{v}"));
        assert!(
            text.starts_with(r#"<pattern id="p1" patternUnits="userSpaceOnUse" x="0" y="0" width="24" height="24"><circle cx="12" cy="12" r="1""#)
        );
    }
}
