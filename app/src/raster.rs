//! A small software rasteriser for the effects egui can't draw itself: blurs and the shadows made from them.
//!
//! The canvas is drawn by egui as triangles, which can't be blurred. An effect instead has its triangles drawn
//! here into a [`Bitmap`], blurred, and handed back to the canvas as a texture.

use egui::epaint::{Mesh, WHITE_UV};
use egui::{Color32, ColorImage};

/// A premultiplied colour, red, green, blue and alpha in 0–1.
pub type Pixel = [f32; 4];

/// Nothing at all.
const CLEAR: Pixel = [0.0; 4];
/// A blur weaker than this many pixels changes nothing worth the work.
const LEAST_SIGMA: f32 = 0.2;
/// Below this many pixels a blur is worked out exactly; three boxes can't be that narrow.
const EXACT_BELOW: f32 = 2.0;

/// From this wide (its standard deviation, in pixels) a blur is worked out on a smaller picture.
const COARSE_FROM: f32 = 8.0;
/// The smallest that smaller picture is, as a share of the full one's side: an eighth.
const COARSEST: usize = 8;

/// A rectangle of pixels.
#[derive(Clone)]
pub struct Bitmap {
    /// Width in pixels.
    pub w: usize,
    /// Height in pixels.
    pub h: usize,
    /// The pixels, row by row from the top.
    pub px: Vec<Pixel>,
}

/// A rectangle in pixel coordinates: left, top, right, bottom.
pub type Span = [f32; 4];

/// A whole number of pixels as a coordinate.
#[expect(clippy::cast_precision_loss, reason = "bitmaps are a few thousand pixels a side at most")]
fn coordinate(v: usize) -> f32 {
    v as f32
}

/// The two of `count` pixels in a row that the place `at` lies between (`None` for one past either end, which
/// counts as clear), and how far it is from the first to the second, 0–1.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "the floor is checked to be within 0..count before the cast")]
fn between(at: f32, count: usize) -> (Option<usize>, Option<usize>, f32) {
    let low = at.floor();
    let inside = |v: f32| (v >= 0.0 && v < coordinate(count)).then_some(v as usize);
    (inside(low), inside(low + 1.0), at - low)
}

/// The pixel a coordinate falls in, within `0..=limit`.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to 0..=limit before the cast")]
fn index(v: f32, limit: usize) -> usize {
    if v.is_nan() { 0 } else { v.clamp(0.0, coordinate(limit)) as usize }
}

/// A colour channel as a byte.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to the u8 range before the cast")]
fn byte(v: f32) -> u8 {
    (v * 255.0).round().clamp(0.0, 255.0) as u8
}

/// An egui colour (premultiplied bytes) as a pixel.
#[must_use]
pub fn pixel(c: Color32) -> Pixel {
    [f32::from(c.r()) / 255.0, f32::from(c.g()) / 255.0, f32::from(c.b()) / 255.0, f32::from(c.a()) / 255.0]
}

/// `over` painted on top of `under`.
fn blend(under: &mut Pixel, over: Pixel) {
    let keep = 1.0 - over[3];
    for (u, o) in under.iter_mut().zip(over) {
        *u = o + *u * keep;
    }
}

/// Twice the signed area of a → b → p, worked out the same way whichever end of the edge comes first, so two
/// triangles sharing an edge agree exactly about which side a point is on.
fn edge(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> f32 {
    let cross = |a: [f32; 2], b: [f32; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    if forward(a, b) { cross(a, b) } else { -cross(b, a) }
}

/// Whether an edge runs from its smaller end to its larger; a point exactly on an edge belongs to the triangle
/// that sees the edge this way round.
fn forward(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0], a[1]) < (b[0], b[1])
}

/// The distance from `p` to the edge of a rounded box centred on the origin; negative inside.
fn box_distance(p: [f32; 2], half: [f32; 2], radius: f32) -> f32 {
    let radius = radius.clamp(0.0, half[0].min(half[1]));
    let q = [p[0].abs() - half[0] + radius, p[1].abs() - half[1] + radius];
    q[0].max(q[1]).min(0.0) + q[0].max(0.0).hypot(q[1].max(0.0)) - radius
}

/// The least a pixel is covered to count as the rim of a shape an outline is drawn round.
const RIM: f32 = 0.2;

/// A squared distance no pixel of a picture is from another.
const FAR: f32 = 1e20;

/// For every place along a line, the least of `far[q] + (p - q)²` over all places `q`: the squared distance
/// to the nearest painted pixel, when `far` holds how far each place already is from one.
// Human: each place stands for a parabola; the answer is the lower edge of them all, found in one sweep
// that keeps only the parabolas that are lowest somewhere.
fn nearest(far: &[f32], out: &mut [f32]) {
    let n = far.len();
    // The parabolas kept, by where they stand, and where each begins to be the lowest.
    let mut kept: Vec<usize> = Vec::with_capacity(n);
    let mut from: Vec<f32> = Vec::with_capacity(n + 1);
    let meet = |p: usize, q: usize| {
        let (fp, fq, p, q) = (far[p], far[q], coordinate(p), coordinate(q));
        // Written so that no large squares are subtracted from each other: a picture is thousands of pixels wide.
        f32::midpoint(p, q) + (fq - fp) / (2.0 * (q - p))
    };
    for (q, height) in far.iter().enumerate() {
        if *height >= FAR {
            continue;
        }
        while let (Some(&last), Some(&since)) = (kept.last(), from.last()) {
            if meet(last, q) > since {
                break;
            }
            kept.pop();
            from.pop();
        }
        from.push(kept.last().map_or(f32::NEG_INFINITY, |&last| meet(last, q)));
        kept.push(q);
    }
    let mut at = 0;
    for (p, out) in out.iter_mut().enumerate().take(n) {
        while at + 1 < kept.len() && from[at + 1] <= coordinate(p) {
            at += 1;
        }
        *out = kept.get(at).map_or(FAR, |&q| far[q] + (coordinate(p) - coordinate(q)).powi(2));
    }
}

impl Bitmap {
    /// A clear bitmap.
    #[must_use]
    pub fn new(w: usize, h: usize) -> Bitmap {
        Bitmap { w, h, px: vec![CLEAR; w * h] }
    }

    /// Paints every pixel `color`.
    pub fn fill(&mut self, color: Pixel) {
        self.px.fill(color);
    }

    /// The rows and columns `span` (or the whole bitmap) covers, cut down to `clip`.
    fn range(&self, span: Span, clip: Option<Span>) -> Option<[usize; 4]> {
        let [mut x0, mut y0, mut x1, mut y1] = span;
        if let Some([cx0, cy0, cx1, cy1]) = clip {
            (x0, y0, x1, y1) = (x0.max(cx0), y0.max(cy0), x1.min(cx1), y1.min(cy1));
        }
        // A pixel is painted when its centre is inside, so the first is the one whose centre is at or after x0.
        let first = |v: f32, limit: usize| index((v - 0.5).ceil(), limit);
        let last = |v: f32, limit: usize| index((v - 0.5).floor() + 1.0, limit);
        let range = [first(x0, self.w), first(y0, self.h), last(x1, self.w), last(y1, self.h)];
        (range[0] < range[2] && range[1] < range[3]).then_some(range)
    }

    /// Paints the triangles of `mesh`, each corner placed by `place`, inside `clip`. `texture` is the picture the
    /// mesh's texture coordinates point into; `solid` paints the shape itself, fully opaque, whatever its colours.
    // Human: one sample at each pixel's centre, and a point on a shared edge goes to exactly one triangle, so a
    // mesh has neither seams nor doubled pixels. Smooth edges come from painting at twice the size and halving.
    // Agent: HOT-PATH for effects; ALLOCATES one placed copy of the corners.
    pub fn triangles(&mut self, mesh: &Mesh, place: impl Fn(egui::Pos2) -> [f32; 2], clip: Option<Span>, texture: Option<&ColorImage>, solid: bool) {
        let corners: Vec<([f32; 2], Pixel, egui::Pos2)> = mesh
            .vertices
            .iter()
            .map(|v| {
                let color = pixel(v.color);
                let color = if !solid {
                    color
                } else if color[3] > 0.0 {
                    [1.0; 4]
                } else {
                    CLEAR
                };
                (place(v.pos), color, v.uv)
            })
            .collect();
        let textured = texture.filter(|_| mesh.vertices.iter().any(|v| v.uv != WHITE_UV));
        let corner = |i: u32| usize::try_from(i).ok().and_then(|i| corners.get(i));
        for &[a, b, c] in mesh.indices.as_chunks::<3>().0 {
            let (Some(&(pa, ca, ta)), Some(&(mut pb, mut cb, mut tb)), Some(&(mut pc, mut cc, mut tc))) = (corner(a), corner(b), corner(c)) else {
                continue;
            };
            let mut area = edge(pa, pb, pc);
            if area < 0.0 {
                (pb, pc, cb, cc, tb, tc) = (pc, pb, cc, cb, tc, tb);
                area = -area;
            }
            if area.is_nan() || area < 1e-9 {
                continue;
            }
            let span = [pa[0].min(pb[0]).min(pc[0]), pa[1].min(pb[1]).min(pc[1]), pa[0].max(pb[0]).max(pc[0]), pa[1].max(pb[1]).max(pc[1])];
            let Some([x0, y0, x1, y1]) = self.range(span, clip) else { continue };
            let edges = [(pb, pc), (pc, pa), (pa, pb)];
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = [coordinate(x) + 0.5, coordinate(y) + 0.5];
                    let weights = edges.map(|(from, to)| (edge(from, to, p), forward(from, to)));
                    if weights.iter().any(|&(w, forward)| w < 0.0 || (w == 0.0 && !forward)) {
                        continue;
                    }
                    let [wa, wb, wc] = weights.map(|(w, _)| w / area);
                    let mut color = [0, 1, 2, 3].map(|i| ca[i] * wa + cb[i] * wb + cc[i] * wc);
                    if let Some(texture) = textured {
                        let texel = texel(texture, ta.x * wa + tb.x * wb + tc.x * wc, ta.y * wa + tb.y * wb + tc.y * wc);
                        color = [0, 1, 2, 3].map(|i| color[i] * texel[i]);
                    }
                    if let Some(under) = self.px.get_mut(y * self.w + x) {
                        blend(under, color);
                    }
                }
            }
        }
    }

    /// Paints an outline in `color` round what `shape` shows: everything within `reach` pixels of it, inside
    /// `clip`. `shape` is this bitmap with `pad` more pixels on every side, so letters just outside this bitmap
    /// (a picture is painted in strips) still have their outline in it.
    // Human: an outline round letters, for a picture. The distance from every pixel to the nearest painted one
    // is worked out exactly in two sweeps (first down each column, then along each row), and a pixel is in
    // the outline as far as that distance is under `reach`. One pass over the pixels, whatever the reach;
    // painting the letters again and again round a ring, as the canvas does, takes a hundred times as long here.
    // Source: Felzenszwalb and Huttenlocher, "Distance Transforms of Sampled Functions" (2012).
    pub fn outline(&mut self, shape: &Bitmap, pad: usize, reach: f32, color: Pixel, clip: Option<Span>) {
        if shape.w != self.w + 2 * pad || shape.h != self.h + 2 * pad || reach.is_nan() || reach <= 0.0 {
            return;
        }
        // The part of `shape` the outline can reach: around what is painted, by `reach`.
        let (mut x0, mut y0, mut x1, mut y1) = (shape.w, shape.h, 0, 0);
        for (at, pixel) in shape.px.iter().enumerate() {
            if pixel[3] > RIM {
                let (x, y) = (at % shape.w, at / shape.w);
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
        if x0 >= x1 {
            return;
        }
        let around = index(reach.ceil() + 1.0, shape.w.max(shape.h));
        let (x0, y0, x1, y1) = (x0.saturating_sub(around), y0.saturating_sub(around), (x1 + around).min(shape.w), (y1 + around).min(shape.h));
        let (w, h) = (x1 - x0, y1 - y0);
        // How far each pixel is from the letters, squared: nothing for a painted one, up to a pixel for one on
        // their rim (so the outline's edge is as smooth as the letters'), and out of reach for a clear one.
        let mut far: Vec<f32> = Vec::with_capacity(w * h);
        for y in y0..y1 {
            for x in x0..x1 {
                let cover = shape.px.get(y * shape.w + x).map_or(0.0, |pixel| pixel[3]);
                // The letters' edge is where they cover half a pixel; the faint rim beyond it is no letter.
                far.push(if cover >= 0.5 {
                    0.0
                } else if cover > RIM {
                    (1.0 - 2.0 * cover).powi(2)
                } else {
                    FAR
                });
            }
        }
        let mut line = vec![0.0; w.max(h)];
        let mut swept = vec![0.0; w.max(h)];
        for x in 0..w {
            for y in 0..h {
                line[y] = far[y * w + x];
            }
            nearest(&line[..h], &mut swept[..h]);
            for y in 0..h {
                far[y * w + x] = swept[y];
            }
        }
        for y in 0..h {
            nearest(&far[y * w..(y + 1) * w], &mut swept[..w]);
            far[y * w..(y + 1) * w].copy_from_slice(&swept[..w]);
        }
        // The part of this bitmap that lies in the region worked out, inside `clip`.
        let own = |v: usize| coordinate(v.saturating_sub(pad));
        let Some([cx0, cy0, cx1, cy1]) = self.range([own(x0), own(y0), own(x1), own(y1)], clip) else { return };
        for y in cy0..cy1 {
            for x in cx0..cx1 {
                let Some(&far) = (y + pad)
                    .checked_sub(y0)
                    .zip((x + pad).checked_sub(x0))
                    .and_then(|(row, column)| far.get(row * w + column).filter(|_| column < w))
                else {
                    continue;
                };
                let cover = (reach + 0.5 - far.sqrt()).clamp(0.0, 1.0);
                if cover > 0.0
                    && let Some(under) = self.px.get_mut(y * self.w + x)
                {
                    blend(under, color.map(|c| c * cover));
                }
            }
        }
    }

    /// Paints a rounded box whose edge fades over `blur` pixels, half inside and half outside it.
    // Human: this is how egui draws the soft shadow of a box; here it is worked out for each pixel from its
    // distance to the box's edge.
    pub fn soft_box(&mut self, rect: Span, radius: f32, blur: f32, color: Pixel, clip: Option<Span>) {
        let blur = blur.max(1.0);
        let grow = blur / 2.0;
        let Some([x0, y0, x1, y1]) = self.range([rect[0] - grow, rect[1] - grow, rect[2] + grow, rect[3] + grow], clip) else { return };
        let centre = [f32::midpoint(rect[0], rect[2]), f32::midpoint(rect[1], rect[3])];
        let half = [(rect[2] - rect[0]) / 2.0, (rect[3] - rect[1]) / 2.0];
        for y in y0..y1 {
            for x in x0..x1 {
                let p = [coordinate(x) + 0.5 - centre[0], coordinate(y) + 0.5 - centre[1]];
                let cover = (0.5 - box_distance(p, half, radius) / blur).clamp(0.0, 1.0);
                if cover > 0.0
                    && let Some(under) = self.px.get_mut(y * self.w + x)
                {
                    blend(under, color.map(|c| c * cover));
                }
            }
        }
    }

    /// The bitmap at half the size, each pixel the average of four.
    #[must_use]
    pub fn halved(&self) -> Bitmap {
        let (w, h) = (self.w / 2, self.h / 2);
        let mut out = Bitmap::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let mut sum = CLEAR;
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let p = self.px.get((2 * y + dy) * self.w + 2 * x + dx).copied().unwrap_or(CLEAR);
                    sum = [0, 1, 2, 3].map(|i| sum[i] + p[i] / 4.0);
                }
                if let Some(slot) = out.px.get_mut(y * w + x) {
                    *slot = sum;
                }
            }
        }
        out
    }

    /// The pixel at `x`, `y`, or nothing outside the bitmap.
    fn at(&self, x: isize, y: isize) -> Pixel {
        match (usize::try_from(x), usize::try_from(y)) {
            (Ok(x), Ok(y)) if x < self.w && y < self.h => self.px.get(y * self.w + x).copied().unwrap_or(CLEAR),
            _ => CLEAR,
        }
    }

    /// The colour at a point between pixels (pixel centres are at halves), mixed from the four around it.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "the floor of a pixel coordinate, far inside the isize range")]
    pub fn sample(&self, x: f32, y: f32) -> Pixel {
        let (fx, fy) = (x - 0.5, y - 0.5);
        if !(fx.is_finite() && fy.is_finite()) || fx < -1.0 || fy < -1.0 || fx > coordinate(self.w) || fy > coordinate(self.h) {
            return CLEAR;
        }
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let (ix, iy) = (x0 as isize, y0 as isize);
        let (p00, p10, p01, p11) = (self.at(ix, iy), self.at(ix + 1, iy), self.at(ix, iy + 1), self.at(ix + 1, iy + 1));
        [0, 1, 2, 3].map(|i| (p00[i] * (1.0 - tx) + p10[i] * tx) * (1.0 - ty) + (p01[i] * (1.0 - tx) + p11[i] * tx) * ty)
    }

    /// Blurs the bitmap as a Gaussian of standard deviation `sigma` pixels would; what lies outside is clear.
    // Human: three box blurs in a row come within a few percent of a true Gaussian and cost the same however
    // wide the blur is (Kovesi, "Fast almost-Gaussian filtering"). A narrow blur, which boxes of whole pixels
    // can't match, is weighed out exactly instead; it has few weights, so that is cheap too.
    // Human: a wide blur leaves nothing in the picture finer than itself, so it is worked out on a picture
    // shrunk by up to a quarter of its width (2, 4 or 8 times) and stretched back: the same picture to the
    // eye for a fraction of the pixels, which is what makes glass and soft shadows cheap when zoomed in.
    pub fn blur(&mut self, sigma: f32) {
        if !sigma.is_finite() || sigma < LEAST_SIGMA || self.w == 0 || self.h == 0 {
            return;
        }
        let mut step = 1;
        while sigma >= COARSE_FROM * coordinate(step) && step < COARSEST {
            step *= 2;
        }
        if step > 1 {
            // Shrinking and stretching blur a little themselves (about a quarter of the step, squared).
            let scale = coordinate(step);
            let mut small = self.shrunk(step);
            small.blur_whole((sigma * sigma - scale * scale / 4.0).max(1.0).sqrt() / scale);
            self.stretch_from(&small, step);
            return;
        }
        self.blur_whole(sigma);
    }

    /// The bitmap at a `step`th of its size, each pixel the average of the `step` × `step` it stands for
    /// (what lies outside counts as clear).
    fn shrunk(&self, step: usize) -> Bitmap {
        let (w, h) = (self.w.div_ceil(step), self.h.div_ceil(step));
        let mut px = vec![CLEAR; w * h];
        for (y, row) in self.px.chunks(self.w).enumerate() {
            for (x, pixel) in row.iter().enumerate() {
                if let Some(sum) = px.get_mut(y / step * w + x / step) {
                    *sum = [0, 1, 2, 3].map(|c| sum[c] + pixel[c]);
                }
            }
        }
        let share = 1.0 / coordinate(step * step);
        for pixel in &mut px {
            *pixel = pixel.map(|c| c * share);
        }
        Bitmap { w, h, px }
    }

    /// Fills the bitmap from `small`, the same picture at a `step`th of the size, reading between its pixels.
    fn stretch_from(&mut self, small: &Bitmap, step: usize) {
        let scale = 1.0 / coordinate(step);
        // Where each of this bitmap's columns lies among the small one's: the two columns and the share of the second.
        let across: Vec<(Option<usize>, Option<usize>, f32)> = (0..self.w).map(|x| between((coordinate(x) + 0.5) * scale - 0.5, small.w)).collect();
        for (y, row) in self.px.chunks_mut(self.w).enumerate() {
            let (top, bottom, down) = between((coordinate(y) + 0.5) * scale - 0.5, small.h);
            let line = |at: Option<usize>| at.and_then(|at| small.px.get(at * small.w..(at + 1) * small.w));
            let (upper, lower) = (line(top), line(bottom));
            for (pixel, (left, right, along)) in row.iter_mut().zip(&across) {
                let read = |line: Option<&[Pixel]>, at: &Option<usize>| line.zip(*at).and_then(|(line, at)| line.get(at)).copied().unwrap_or(CLEAR);
                let (a, b, c, d) = (read(upper, left), read(upper, right), read(lower, left), read(lower, right));
                *pixel = [0, 1, 2, 3].map(|i| {
                    let (over, under) = (a[i] + (b[i] - a[i]) * along, c[i] + (d[i] - c[i]) * along);
                    over + (under - over) * down
                });
            }
        }
    }

    /// [`Bitmap::blur`] at the bitmap's own size.
    fn blur_whole(&mut self, sigma: f32) {
        let mut other = vec![CLEAR; self.px.len()];
        if sigma < EXACT_BELOW {
            let weights = bell(sigma);
            weighted_pass(&self.px, &mut other, self.w, self.h, &weights, false);
            weighted_pass(&other, &mut self.px, self.w, self.h, &weights, true);
            return;
        }
        for reach in box_reaches(sigma) {
            if reach == 0 {
                continue;
            }
            box_pass(&self.px, &mut other, self.w, self.h, reach, false);
            box_pass(&other, &mut self.px, self.w, self.h, reach, true);
        }
    }

    /// The bitmap as premultiplied RGBA bytes, four a pixel, for the GPU.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        self.px.iter().flat_map(|p| p.map(byte)).collect()
    }

    /// A bitmap from premultiplied RGBA bytes, or `None` when there aren't four for each pixel.
    #[must_use]
    pub fn from_bytes(w: usize, h: usize, bytes: &[u8]) -> Option<Bitmap> {
        (bytes.len() == w * h * 4).then(|| Bitmap { w, h, px: bytes.as_chunks::<4>().0.iter().map(|p| p.map(|c| f32::from(c) / 255.0)).collect() })
    }

    /// The bitmap as an egui picture.
    #[must_use]
    pub fn image(&self) -> ColorImage {
        let pixels = self
            .px
            .iter()
            .map(|p| {
                let a = byte(p[3]);
                // A premultiplied channel is never above its alpha.
                Color32::from_rgba_premultiplied(byte(p[0]).min(a), byte(p[1]).min(a), byte(p[2]).min(a), a)
            })
            .collect();
        ColorImage::new([self.w, self.h], pixels)
    }
}

/// The colour of `texture` at texture coordinates `u`, `v` (0–1), mixed from the four texels around the point.
fn texel(texture: &ColorImage, u: f32, v: f32) -> Pixel {
    let [w, h] = texture.size;
    if w == 0 || h == 0 {
        return CLEAR;
    }
    let (fx, fy) = ((u * coordinate(w) - 0.5).clamp(0.0, coordinate(w - 1)), (v * coordinate(h) - 0.5).clamp(0.0, coordinate(h - 1)));
    let (x0, y0) = (index(fx.floor(), w - 1), index(fy.floor(), h - 1));
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (tx, ty) = (fx - coordinate(x0), fy - coordinate(y0));
    let get = |x: usize, y: usize| texture.pixels.get(y * w + x).copied().map_or(CLEAR, pixel);
    let (p00, p10, p01, p11) = (get(x0, y0), get(x1, y0), get(x0, y1), get(x1, y1));
    [0, 1, 2, 3].map(|i| (p00[i] * (1.0 - tx) + p10[i] * tx) * (1.0 - ty) + (p01[i] * (1.0 - tx) + p11[i] * tx) * ty)
}

/// How far each of three box blurs reaches to either side so that together they blur like a Gaussian of `sigma`.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "box widths are small positive whole numbers")]
fn box_reaches(sigma: f32) -> [usize; 3] {
    const BOXES: f32 = 3.0;
    let ideal = (12.0 * sigma * sigma / BOXES + 1.0).sqrt();
    let mut lower = ideal.floor();
    if lower % 2.0 == 0.0 {
        lower -= 1.0;
    }
    let lower = lower.max(1.0);
    // How many of the boxes take the narrower width.
    let narrow = ((12.0 * sigma * sigma - BOXES * lower * lower - 4.0 * BOXES * lower - 3.0 * BOXES) / (-4.0 * lower - 4.0)).round();
    [0.0, 1.0, 2.0].map(|i| {
        let width = if i < narrow { lower } else { lower + 2.0 };
        ((width - 1.0) / 2.0).clamp(0.0, 4096.0) as usize
    })
}

/// The weights of a Gaussian of `sigma` from its centre outwards, out to three sigmas; with both sides they add
/// up to 1.
fn bell(sigma: f32) -> Vec<f32> {
    let reach = index((sigma * 3.0).ceil(), 64).max(1);
    let weights: Vec<f32> = (0..=reach).map(|i| (-coordinate(i).powi(2) / (2.0 * sigma * sigma)).exp()).collect();
    let total: f32 = weights.iter().sum::<f32>() * 2.0 - weights.first().copied().unwrap_or(1.0);
    weights.iter().map(|w| w / total).collect()
}

/// One weighted blur along the rows (or the columns, when `down`); `weights` run from the centre outwards.
fn weighted_pass(from: &[Pixel], to: &mut [Pixel], w: usize, h: usize, weights: &[f32], down: bool) {
    let (lines, length, step, stride) = if down { (w, h, w, 1) } else { (h, w, 1, w) };
    for line in 0..lines {
        let start = line * stride;
        let get = |i: usize| from.get(start + i * step).copied().unwrap_or(CLEAR);
        for i in 0..length {
            let mut sum = CLEAR;
            for (distance, weight) in weights.iter().enumerate() {
                let mut add = |p: Pixel| sum = [0, 1, 2, 3].map(|c| sum[c] + p[c] * weight);
                if distance <= i {
                    add(get(i - distance));
                }
                if distance > 0 && i + distance < length {
                    add(get(i + distance));
                }
            }
            if let Some(slot) = to.get_mut(start + i * step) {
                *slot = sum;
            }
        }
    }
}

/// One box blur along the rows (or the columns, when `down`): each pixel becomes the average of those within
/// `reach` of it.
// Agent: HOT-PATH; a running sum, so the cost doesn't grow with `reach`.
fn box_pass(from: &[Pixel], to: &mut [Pixel], w: usize, h: usize, reach: usize, down: bool) {
    let (lines, length, step, stride) = if down { (w, h, w, 1) } else { (h, w, 1, w) };
    let scale = 1.0 / coordinate(2 * reach + 1);
    for line in 0..lines {
        let start = line * stride;
        let get = |i: usize| from.get(start + i * step).copied().unwrap_or(CLEAR);
        let mut sum = CLEAR;
        for i in 0..reach.min(length) {
            let p = get(i);
            sum = [0, 1, 2, 3].map(|c| sum[c] + p[c]);
        }
        for i in 0..length {
            if i + reach < length {
                let p = get(i + reach);
                sum = [0, 1, 2, 3].map(|c| sum[c] + p[c]);
            }
            if let Some(slot) = to.get_mut(start + i * step) {
                *slot = sum.map(|c| (c * scale).max(0.0));
            }
            if i >= reach {
                let p = get(i - reach);
                sum = [0, 1, 2, 3].map(|c| sum[c] - p[c]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::epaint::Vertex;
    use egui::pos2;

    fn square(x0: f32, y0: f32, x1: f32, y1: f32, color: Color32) -> Mesh {
        let corner = |x, y| Vertex { pos: pos2(x, y), uv: WHITE_UV, color };
        Mesh { indices: vec![0, 1, 2, 0, 2, 3], vertices: vec![corner(x0, y0), corner(x1, y0), corner(x1, y1), corner(x0, y1)], ..Default::default() }
    }

    #[test]
    fn an_outline_reaches_as_far_as_asked_round_what_is_painted_also_past_the_edge() {
        // One painted pixel in the middle of a padded shape; the outline is a disc of the reach round it.
        let (side, pad) = (41, 6);
        let mut shape = Bitmap::new(side + 2 * pad, side + 2 * pad);
        shape.px[(20 + pad) * shape.w + 20 + pad] = [1.0; 4];
        let mut bitmap = Bitmap::new(side, side);
        bitmap.outline(&shape, pad, 8.0, [1.0; 4], None);
        let alpha = |x: usize, y: usize| bitmap.px[y * side + x][3];
        assert!(alpha(20, 20) >= 1.0 && alpha(27, 20) >= 1.0 && alpha(20, 13) >= 1.0);
        assert!(alpha(29, 20) <= 0.0 && alpha(27, 27) <= 0.0 && alpha(25, 25) >= 1.0);
        // The edge is soft over one pixel: half covered at exactly the reach.
        assert!((alpha(28, 20) - 0.5).abs() < 1e-3);
        // A pixel painted just outside the bitmap, in the padding, still casts its outline into it.
        let mut shape = Bitmap::new(side + 2 * pad, side + 2 * pad);
        shape.px[(20 + pad) * shape.w + 2] = [1.0; 4];
        let mut bitmap = Bitmap::new(side, side);
        bitmap.outline(&shape, pad, 8.0, [1.0; 4], None);
        assert!(bitmap.px[20 * side][3] >= 1.0 && bitmap.px[20 * side + 3][3] >= 1.0 && bitmap.px[20 * side + 6][3] <= 0.0);
        // Distances along a line: the nearest painted place, squared.
        let mut out = [0.0; 6];
        nearest(&[FAR, 0.0, FAR, FAR, FAR, 0.0], &mut out);
        assert_eq!(out, [1.0, 0.0, 1.0, 4.0, 1.0, 0.0]);
    }

    #[test]
    fn a_wide_blur_worked_out_small_is_the_same_picture_as_one_worked_out_whole() {
        // A white square in the middle of a clear picture, blurred wide enough to be worked out at a quarter.
        let mut whole = Bitmap::new(160, 120);
        for y in 40..80 {
            for x in 50..110 {
                whole.px[y * 160 + x] = [1.0; 4];
            }
        }
        let mut small = whole.clone();
        whole.blur_whole(20.0);
        small.blur(20.0);
        let worst = whole.px.iter().zip(&small.px).map(|(a, b)| (a[3] - b[3]).abs()).fold(0.0, f32::max);
        // No pixel is off by more than 2 % (5 of the 255 steps a colour has), on a blur 20 pixels wide.
        assert!(worst < 0.02, "{worst}");
        // Nothing is lost or gained on the way: the light in the picture is the same.
        assert!((alpha_sum(&whole) - alpha_sum(&small)).abs() / alpha_sum(&whole) < 0.01);
        // A narrow blur is worked out whole, as before.
        let (mut a, mut b) = (Bitmap::new(40, 40), Bitmap::new(40, 40));
        (a.px[820], b.px[820]) = ([1.0; 4], [1.0; 4]);
        a.blur(5.0);
        b.blur_whole(5.0);
        assert_eq!(a.px, b.px);
    }

    fn alpha_sum(bitmap: &Bitmap) -> f32 {
        bitmap.px.iter().map(|p| p[3]).sum()
    }

    #[test]
    fn two_triangles_sharing_an_edge_paint_each_pixel_once() {
        // A half-clear square: a pixel painted twice along the diagonal would come out darker than 0.5.
        let mut bitmap = Bitmap::new(8, 8);
        bitmap.triangles(&square(1.0, 1.0, 7.0, 7.0, Color32::from_rgba_premultiplied(0, 0, 0, 128)), |p| [p.x, p.y], None, None, false);
        let painted: Vec<f32> = bitmap.px.iter().map(|p| p[3]).filter(|a| *a > 0.0).collect();
        assert_eq!(painted.len(), 36);
        assert!(painted.iter().all(|a| (a - 128.0 / 255.0).abs() < 1e-4));
    }

    #[test]
    fn solid_painting_ignores_colour_and_clipping_cuts_it_off() {
        let mut bitmap = Bitmap::new(8, 8);
        bitmap.triangles(
            &square(0.0, 0.0, 8.0, 8.0, Color32::from_rgba_premultiplied(10, 0, 0, 20)),
            |p| [p.x, p.y],
            Some([2.0, 2.0, 6.0, 8.0]),
            None,
            true,
        );
        assert!((alpha_sum(&bitmap) - 24.0).abs() < 1e-4);
        assert!((bitmap.px[0][3]).abs() < 1e-6 && (bitmap.px[3 * 8 + 3][3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_blur_spreads_a_dot_without_losing_or_gaining_any_of_it() {
        let mut bitmap = Bitmap::new(41, 41);
        bitmap.px[20 * 41 + 20] = [1.0; 4];
        bitmap.blur(3.0);
        assert!((alpha_sum(&bitmap) - 1.0).abs() < 1e-3);
        let (centre, beside, far) = (bitmap.px[20 * 41 + 20][3], bitmap.px[20 * 41 + 23][3], bitmap.px[20 * 41 + 32][3]);
        assert!(centre > beside && beside > far && far < 1e-3);
        // A Gaussian of sigma 3 has 1 / (2π·9) ≈ 0.0177 at its centre.
        assert!((centre - 0.0177).abs() < 0.004, "{centre}");
    }

    #[test]
    fn the_three_boxes_together_have_the_variance_asked_for() {
        for sigma in [2.0_f32, 4.0, 10.0, 37.5] {
            let variance: f32 = box_reaches(sigma).iter().map(|r| (coordinate(2 * r + 1).powi(2) - 1.0) / 12.0).sum();
            assert!((variance.sqrt() - sigma).abs() < 0.3, "{sigma}: {}", variance.sqrt());
        }
    }

    #[test]
    fn a_narrow_blur_is_exact() {
        // A dot blurred by sigma 0.8 has 1 / (2π·0.64) ≈ 0.249 at its centre (a little more, cut off at three sigmas).
        let mut bitmap = Bitmap::new(21, 21);
        bitmap.px[10 * 21 + 10] = [1.0; 4];
        bitmap.blur(0.8);
        assert!((alpha_sum(&bitmap) - 1.0).abs() < 1e-4);
        assert!((bitmap.px[10 * 21 + 10][3] - 0.249).abs() < 0.01, "{}", bitmap.px[10 * 21 + 10][3]);
        assert!(bitmap.px[10 * 21 + 11][3] > bitmap.px[10 * 21 + 12][3]);
    }

    #[test]
    fn halving_averages_and_sampling_mixes_neighbours() {
        let mut bitmap = Bitmap::new(4, 2);
        bitmap.px[0] = [1.0; 4];
        bitmap.px[1] = [1.0; 4];
        let half = bitmap.halved();
        assert_eq!((half.w, half.h), (2, 1));
        assert!((half.px[0][3] - 0.5).abs() < 1e-6 && half.px[1][3].abs() < 1e-6);
        assert!((bitmap.sample(0.5, 0.5)[3] - 1.0).abs() < 1e-6);
        assert!((bitmap.sample(2.0, 0.5)[3] - 0.5).abs() < 1e-6);
        assert!(bitmap.sample(-5.0, 0.5)[3].abs() < 1e-6);
    }

    #[test]
    fn a_soft_box_is_full_inside_and_fades_across_its_edge() {
        let mut bitmap = Bitmap::new(40, 40);
        bitmap.soft_box([10.0, 10.0, 30.0, 30.0], 0.0, 8.0, [0.0, 0.0, 0.0, 1.0], None);
        let at = |x: usize, y: usize| bitmap.px[y * 40 + x][3];
        assert!((at(20, 20) - 1.0).abs() < 1e-6);
        assert!((at(9, 20) - 0.44).abs() < 0.07 && at(7, 20) > 0.0 && at(7, 20) < at(9, 20));
        assert!(at(4, 20).abs() < 1e-6);
    }

    #[test]
    fn bytes_go_to_the_gpu_and_come_back_the_same() {
        let bytes = [255, 128, 0, 255, 0, 0, 0, 0];
        let bitmap = Bitmap::from_bytes(2, 1, &bytes).unwrap();
        assert_eq!(bitmap.bytes(), bytes);
        assert!(Bitmap::from_bytes(2, 2, &bytes).is_none());
    }

    #[test]
    fn a_bitmap_becomes_a_premultiplied_picture() {
        let mut bitmap = Bitmap::new(1, 1);
        bitmap.px[0] = [0.5, 0.25, 0.0, 0.5];
        let image = bitmap.image();
        assert_eq!(image.pixels[0], Color32::from_rgba_premultiplied(128, 64, 0, 128));
    }
}
