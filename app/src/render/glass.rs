//! Materials: a layer whose background is what lies behind it, frosted or seen through liquid glass.
//!
//! Like the other effects, a material is baked into a picture: what is painted before the layer is drawn into
//! a bitmap, blurred in linear light, and then either tinted on the CPU (frosted) or handed to the glass
//! shader on the GPU, which bends it at the layer's rim. Without a GPU, glass is drawn as the frosted
//! material is, with its own numbers: blur, tint and a lit edge, but no bending.
//!
//! The glass layers of a glass group (a frame with `glassGroup`) are one pane: their outlines are joined with
//! a smooth minimum, so layers closer than the group's spacing flow into each other, and none of them has
//! another one behind it.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, OnceLock};

use lyon_tessellation::FillRule;
use pen_doc::color::Rgba;
use pen_doc::geom::{Point, Rect};
use pen_doc::layout;
use pen_doc::model::Kind;
use pen_doc::scene::SceneNode;
use serde_json::Value;

use super::bake::{Grid, Look, Made, REACH, Seen, Shadow, expand, extent, length, sigma};
use super::{Brush, Builder, Item, Memo, rounded_rect};
use crate::gpu;
use crate::material::{Lookup, Material, Params};
use crate::raster::{self, Bitmap, Pixel};
use crate::shader::{self, Program};

/// The glass shader's source.
const SHADER: &str = include_str!("glass.frag");
/// The shader's sampler for what is behind the layer.
const BACKDROP: &str = "u_backdrop";
/// The shader's sampler for the same, only lightly blurred, which the rim shows.
const RIM: &str = "u_rim";
/// How much of a glass's blur its rim keeps.
const RIM_BLUR: f32 = 0.2;
/// How many outlines one pane can join (the shader's `u_box0` to `u_box7`).
pub(super) const MOST_SHAPES: usize = 8;
/// The width of the bright line along a material's edge, in document units (the shader's `HAIRLINE`).
const HAIRLINE: f32 = 1.1;
/// A backdrop darker than this (linear light, 0–1) gets the dark material when the appearance is `auto`.
// Human: around this luminance black text and white text are equally hard to read; below it the dark
// material with light content on it is the legible one.
const DARK_BELOW: f32 = 0.22;
/// From this many pixels of blur on, what is behind is painted at half the size, and at a quarter from twice
/// that: a blur that wide hides the difference, and the bitmap is a quarter or a sixteenth of the work.
const COARSE_FROM: f32 = 4.0;
/// How dark the tight shadow right under a material's edge is, as a share of its soft shadow.
const CONTACT: f32 = 0.5;

/// A layer's material with what the renderer needs to draw it.
pub(super) struct Pane {
    /// The material as the file writes it.
    material: Material,
    /// The size of the layer the material's numbers are worked out for.
    size: (f32, f32),
    /// The outline in numbers, when it can be worked out for every pixel; else the fill's shape is used.
    form: Option<Form>,
    /// The outline as items, for a layer that has no fill to show it.
    pub mask: Vec<Item>,
    /// The scene ids of the layers this pane is the material of (several for a glass group).
    ids: Vec<String>,
    /// The part of the document the pane covers.
    frame: Rect,
}

/// One outline: a rounded box or an ellipse.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shape {
    /// Its centre as an offset from the form's centre, in the form's own directions.
    offset: [f32; 2],
    /// Half its width and height.
    half: [f32; 2],
    /// Corner radii: top left, top right, bottom right, bottom left.
    radii: [f32; 4],
    /// Whether it is an ellipse.
    oval: bool,
}

impl Shape {
    /// The distance from `p` (an offset from the form's centre) to the outline; negative inside.
    // Agent: the same two formulas as `box_distance` and `oval_distance` in glass.frag; keep them alike.
    fn distance(&self, p: [f32; 2]) -> f32 {
        let p = [p[0] - self.offset[0], p[1] - self.offset[1]];
        let [hx, hy] = self.half;
        if self.oval {
            let k0 = (p[0] / hx).hypot(p[1] / hy);
            let k1 = (p[0] / (hx * hx)).hypot(p[1] / (hy * hy));
            return if k1 > 1e-6 { k0 * (k0 - 1.0) / k1 } else { -hx.min(hy) };
        }
        let radius = match (p[0] < 0.0, p[1] < 0.0) {
            (true, true) => self.radii[0],
            (false, true) => self.radii[1],
            (false, false) => self.radii[2],
            (true, false) => self.radii[3],
        };
        let q = [p[0].abs() - hx + radius, p[1].abs() - hy + radius];
        q[0].max(q[1]).min(0.0) + q[0].max(0.0).hypot(q[1].max(0.0)) - radius
    }
}

/// The smaller of two distances, rounded off where they are within `k` of each other, so two outlines closer
/// than `k` flow into one.
// Agent: the shader's `smooth_min`; keep them alike.
fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    if k <= 1e-3 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

/// A pane's outline in numbers: where it is, how it is turned, and the shapes it joins.
#[derive(Clone, Debug, PartialEq)]
struct Form {
    /// The document point of its centre.
    centre: Point,
    /// Turns a document offset into the form's own: x' = a·x + c·y, y' = b·x + d·y.
    turn: [f32; 4],
    /// The outlines, at most [`MOST_SHAPES`].
    shapes: Vec<Shape>,
    /// Within this distance of each other the outlines flow together.
    merge: f32,
}

impl Form {
    /// A document point as an offset from the form's centre in the form's own directions.
    fn local(&self, doc: Point) -> [f32; 2] {
        let (x, y) = (doc.x - self.centre.x, doc.y - self.centre.y);
        [self.turn[0] * x + self.turn[2] * y, self.turn[1] * x + self.turn[3] * y]
    }

    /// The distance from `p` (the form's own offset) to the pane's edge; negative inside.
    fn distance(&self, p: [f32; 2]) -> f32 {
        self.shapes.iter().map(|shape| shape.distance(p)).reduce(|a, b| smooth_min(a, b, self.merge)).unwrap_or(f32::MAX)
    }

    /// The way out of the pane at `p`, as a unit vector in document space.
    fn outward(&self, p: [f32; 2]) -> [f32; 2] {
        const STEP: f32 = 0.25;
        let slope = [
            self.distance([p[0] + STEP, p[1]]) - self.distance([p[0] - STEP, p[1]]),
            self.distance([p[0], p[1] + STEP]) - self.distance([p[0], p[1] - STEP]),
        ];
        let steep = slope[0].hypot(slope[1]);
        if steep < 1e-5 {
            return [0.0; 2];
        }
        let (x, y) = (slope[0] / steep, slope[1] / steep);
        [self.turn[0] * x + self.turn[1] * y, self.turn[2] * x + self.turn[3] * y]
    }
}

/// How many steps the tables of the colour curves have between 0 and 1.
const CURVE_STEPS: usize = 4096;

/// A colour curve from 0 to 1, worked out once at [`CURVE_STEPS`] places and read between them.
// Human: every pixel of every material goes through both curves, and a power is slow to work out: the two
// were a sixth of the time a design with glass took to draw. Read from the table they are off by less than a
// ten-thousandth, a twentieth of the smallest step a colour on screen can take.
struct Curve([f32; CURVE_STEPS + 2]);

impl Curve {
    fn new(exact: fn(f32) -> f32) -> Curve {
        #[expect(clippy::cast_precision_loss, reason = "at most 4097, far below f32's exact range")]
        Curve(std::array::from_fn(|step| exact((step.min(CURVE_STEPS) as f32) / CURVE_STEPS as f32)))
    }

    fn at(&self, c: f32) -> f32 {
        #[expect(clippy::cast_precision_loss, reason = "4096 is exact in f32")]
        let place = if c.is_nan() { 0.0 } else { c.clamp(0.0, 1.0) * CURVE_STEPS as f32 };
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to 0..=4096 just above")]
        let step = place as usize;
        #[expect(clippy::cast_precision_loss, reason = "at most 4096")]
        let part = place - step as f32;
        let (low, high) = (self.0[step.min(CURVE_STEPS)], self.0[(step + 1).min(CURVE_STEPS + 1)]);
        low + (high - low) * part
    }
}

/// A gamma-encoded channel in linear light.
fn to_linear(c: f32) -> f32 {
    static TABLE: std::sync::OnceLock<Curve> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| Curve::new(|c| if c <= 0.040_45 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) })).at(c)
}

/// A linear-light channel, gamma-encoded.
fn to_gamma(c: f32) -> f32 {
    static TABLE: std::sync::OnceLock<Curve> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| Curve::new(|c| if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 })).at(c)
}

/// A premultiplied pixel with its colour run through `curve`.
fn recode(pixel: Pixel, curve: fn(f32) -> f32) -> Pixel {
    let alpha = pixel[3];
    if alpha <= 1e-4 {
        return [0.0; 4];
    }
    [curve(pixel[0] / alpha) * alpha, curve(pixel[1] / alpha) * alpha, curve(pixel[2] / alpha) * alpha, alpha]
}

/// A colour in linear light, opaque.
fn linear(color: Rgba) -> [f32; 3] {
    [color.r, color.g, color.b].map(|c| to_linear(f32::from(c) / 255.0))
}

/// How light a linear, premultiplied pixel is on white, 0–1.
fn lightness(pixel: Pixel) -> f32 {
    0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2] + (1.0 - pixel[3])
}

/// The unit vector toward a light `angle` degrees clockwise from straight above, in document space (y down).
fn toward_light(angle: f32) -> [f32; 2] {
    let (sin, cos) = angle.to_radians().sin_cos();
    [sin, -cos]
}

/// The compiled glass shader, or `None` when it doesn't compile (said once in the log).
fn program() -> Option<Arc<Program>> {
    static PROGRAM: OnceLock<Option<Arc<Program>>> = OnceLock::new();
    PROGRAM
        .get_or_init(|| match shader::compile(SHADER) {
            Ok(program) => Some(Arc::new(program)),
            Err(error) => {
                log::error!("the glass shader doesn't compile: {error}");
                None
            }
        })
        .clone()
}

/// What is behind a material, ready to be drawn through it: blurred, in linear light, on its own grid.
struct Behind {
    grid: Grid,
    bitmap: Bitmap,
    /// The same, only lightly blurred, for a glass's rim.
    rim: Option<Bitmap>,
}

impl Behind {
    /// The colour behind a document point.
    fn at(&self, doc: Point) -> Pixel {
        let [x, y] = self.grid.place(doc);
        self.bitmap.sample(x, y)
    }
}

/// How light `bitmap` (linear light, on `grid`) is under `frame`: on average, and how far its lightest and
/// darkest parts are apart (leaving out a two-hundredth at either end as noise). A clear backdrop counts as white.
fn levels(bitmap: &Bitmap, grid: &Grid, frame: &Rect) -> (f32, f32) {
    let [x0, y0, x1, y1] = grid.span(frame);
    let mut found = Vec::new();
    for y in 0..bitmap.h {
        for x in 0..bitmap.w {
            let (cx, cy) = (length(x) + 0.5, length(y) + 0.5);
            if cx >= x0 && cx <= x1 && cy >= y0 && cy <= y1 {
                found.push(bitmap.px.get(y * bitmap.w + x).copied().map_or(1.0, lightness));
            }
        }
    }
    if found.is_empty() {
        return (1.0, 0.0);
    }
    let mean = found.iter().sum::<f32>() / length(found.len());
    found.sort_by(f32::total_cmp);
    let edge = found.len() / 200;
    let (low, high) = (found.get(edge).copied().unwrap_or(mean), found.get(found.len() - 1 - edge).copied().unwrap_or(mean));
    (mean, high - low)
}

/// Frost, tint and edge light for one pixel: `behind` in linear light, the edge's `cover` of the pixel, how far
/// `inside` the edge it is and which way is `outward` there (`None` for a shape without a worked-out edge).
// Agent: the CPU's side of glass.frag's frost and hairline; keep the two alike.
fn frosted_pixel(behind: Pixel, params: &Params, tint: [f32; 3], cover: f32, edge: Option<(f32, [f32; 2])>) -> Pixel {
    let grey = 0.2126 * behind[0] + 0.7152 * behind[1] + 0.0722 * behind[2];
    let alpha = behind[3];
    let mut rgb = [behind[0], behind[1], behind[2]].map(|c| (grey + (c - grey) * params.saturation).max(0.0));
    rgb = rgb.map(|c| (c + params.brightness * alpha).clamp(0.0, 1.0));
    let t = params.tint_opacity;
    let mut rgb = [0, 1, 2].map(|i| rgb[i] + (tint[i] - rgb[i]) * t);
    let mut alpha = alpha + (1.0 - alpha) * t;
    if let Some((inside, outward)) = edge {
        let along = (inside / HAIRLINE).clamp(0.0, 1.0);
        let hairline = 1.0 - along * along * (3.0 - 2.0 * along);
        let lit = if params.glass {
            let light = toward_light(params.light_angle);
            let facing = outward[0] * light[0] + outward[1] * light[1];
            0.16 + 0.84 * (facing.max(0.0).powi(2) + 0.45 * (-facing).max(0.0).powi(2))
        } else {
            1.0
        };
        let light = params.highlight * hairline * lit;
        rgb = rgb.map(|c| c + light * (1.0 - c));
        alpha += light * (1.0 - alpha);
    }
    if alpha <= 1e-4 {
        return [0.0; 4];
    }
    let out = alpha * cover;
    [to_gamma(rgb[0] / alpha) * out, to_gamma(rgb[1] / alpha) * out, to_gamma(rgb[2] / alpha) * out, out]
}

impl Builder<'_> {
    /// The outline of `scene` as a path and as a shape around the origin, when it is a box or a whole ellipse.
    fn outline_of(&self, scene: &SceneNode) -> Option<(lyon_tessellation::path::Path, Shape)> {
        let (node, theme) = (&scene.node, &scene.theme);
        let (w, h) = (scene.local.w, scene.local.h);
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let shape = |radii: [f32; 4], oval: bool| Shape { offset: [0.0; 2], half: [w / 2.0, h / 2.0], radii, oval };
        match node.kind {
            Kind::Frame | Kind::Rectangle | Kind::Browser => {
                let radii = self.corner_radii(node, theme, w, h);
                Some((rounded_rect(0.0, 0.0, w, h, radii), shape(radii, false)))
            }
            Kind::Ellipse if self.box_radius(scene).is_some() => Some((self.ellipse(node, theme, w, h), shape([0.0; 4], true))),
            _ => None,
        }
    }

    /// `scene`'s outline as an item that paints it.
    fn mask_of(&self, scene: &SceneNode, path: &lyon_tessellation::path::Path, clip: Option<Rect>) -> Option<Item> {
        self.fill_mesh(path, &Brush::Solid(Rgba::WHITE), FillRule::NonZero, (scene.local.w, scene.local.h), &scene.to_doc, 1.0)
            .map(|mesh| Item::Mesh { mesh: mesh.into(), bounds: layout::doc_bounds(scene), clip, memo: Memo::default() })
    }

    /// Whether `child`'s glass is drawn by the glass group `parent` it is in, not by itself: both lie straight
    /// on the canvas and the child is liquid glass with an outline that can be worked out.
    pub(super) fn merges(&self, parent: &SceneNode, child: &SceneNode) -> bool {
        !self.hides(child)
            && crate::glass_check::group_spacing(parent).is_some()
            && parent.to_doc.is_axis_aligned()
            && child.to_doc.is_axis_aligned()
            && child.node.material.as_ref().and_then(Material::read).is_some_and(|material| material.is_glass())
            && self.outline_of(child).is_some()
    }

    /// `scene`'s material with its outline, when it has one; `merged` says that its parent's glass group draws
    /// it. A glass group has the material of its first glass child, as one pane for all of them.
    // Human: a box and a whole ellipse have an outline that can be worked out for every pixel, which is what
    // glass bends light along; any other shape is frosted inside whatever its fill paints.
    pub(super) fn pane_of(&self, scene: &SceneNode, merged: bool, clip: Option<Rect>) -> Option<Pane> {
        let members: Vec<&SceneNode> = scene.children.iter().filter(|child| self.merges(scene, child)).take(MOST_SHAPES).collect();
        if let (Some(first), Some(merge)) = (members.first(), crate::glass_check::group_spacing(scene)) {
            let frame = members.iter().map(|member| layout::doc_bounds(member)).reduce(|a, b| a.union(&b))?;
            let centre = frame.center();
            let mut shapes = Vec::new();
            let mut mask = Vec::new();
            for member in &members {
                let (path, shape) = self.outline_of(member)?;
                let at = layout::doc_bounds(member).center();
                shapes.push(Shape { offset: [at.x - centre.x, at.y - centre.y], ..shape });
                mask.extend(self.mask_of(member, &path, clip));
            }
            return Some(Pane {
                material: first.node.material.as_ref().and_then(Material::read)?,
                size: (first.local.w, first.local.h),
                form: Some(Form { centre, turn: [1.0, 0.0, 0.0, 1.0], shapes, merge }),
                mask,
                ids: members.iter().map(|member| member.id.clone()).collect(),
                frame,
            });
        }
        if merged {
            return None;
        }
        let material = scene.node.material.as_ref().and_then(Material::read)?;
        let (w, h) = (scene.local.w, scene.local.h);
        let outline = self.outline_of(scene);
        let back = scene.to_doc.invert().map(|back| back.m);
        let form = outline.as_ref().zip(back).map(|((_, shape), back)| Form {
            centre: scene.to_doc.apply(Point::new(w / 2.0, h / 2.0)),
            turn: [back[0], back[1], back[2], back[3]],
            shapes: vec![*shape],
            merge: 0.0,
        });
        let mask = outline.and_then(|(path, _)| self.mask_of(scene, &path, clip)).into_iter().collect();
        Some(Pane { material, size: (w, h), form, mask, ids: vec![scene.id.clone()], frame: layout::doc_bounds(scene) })
    }

    /// The numbers of `pane`'s material, read through `scene`'s variables and theme.
    pub(super) fn material_params(&self, scene: &SceneNode, pane: &Pane, dark: bool) -> Params {
        let theme = &scene.theme;
        let number = |name: &str| match self.vars.lookup(name, theme)? {
            Value::Number(n) => n.as_f64().map(pen_doc::value::narrow),
            Value::String(text) => text.trim().parse().ok(),
            _ => None,
        };
        let color = |text: &str| self.vars.color(text, theme);
        pane.material.resolve(pane.size, dark, &Lookup { number: &number, color: &color })
    }

    /// The items of `scene`'s material: its shadow, then what is behind it seen through the material. `shape`
    /// is what the pane's outline is painted with, `behind` everything painted before the layer.
    pub(super) fn material(&self, scene: &SceneNode, pane: &Pane, shape: &[&Item], behind: &[&Item], look: &Look) -> Vec<Item> {
        let mut items = Vec::new();
        let light = self.material_params(scene, pane, false);
        if light.shadow > 0.0 {
            // Human: a soft shadow under the pane and a tight one right at its edge; both are cut out under the
            // pane itself, so they never show through it.
            let cast = Look { opaque: false, ..*look };
            let shadow = |drop: f32, blur: f32, strength: f32| Shadow {
                offset: Point::new(0.0, drop),
                blur,
                color: Rgba::BLACK.with_opacity(light.shadow * strength),
            };
            items.extend(self.drop_shadow(shape, &shadow(light.shadow_drop, light.shadow_blur, 1.0), &cast));
            items.extend(self.drop_shadow(shape, &shadow(1.0, 4.0, CONTACT), &cast));
        }
        items.extend(self.pane(scene, pane, shape, behind, look));
        items
    }

    /// What is behind `pane`, seen through its material, as one picture.
    fn pane(&self, scene: &SceneNode, pane: &Pane, shape: &[&Item], behind: &[&Item], look: &Look) -> Option<Item> {
        let frame = pane.frame;
        let pad = 1.0 / self.scale;
        let grid = Grid::new(expand(&frame, pad), look.clip, pad, self.scale, false)?;
        // Both appearances are worked out: which one is drawn depends on what is behind, known only once that
        // is painted, and the picture's key has to be known before.
        let params = [false, true].map(|dark| self.material_params(scene, pane, dark));
        let blur = sigma(params[0].blur);
        // Human: what is behind is painted a blur's reach beyond the pane, so the blur has something to mix in
        // at the pane's edge, and coarser the wider the blur is.
        let reach = blur * REACH + 2.0 * pad;
        let glass = params[0].glass && pane.form.is_some();
        let finest = if glass { blur * RIM_BLUR } else { blur };
        let coarse = match finest * grid.scale {
            wide if wide >= 2.0 * COARSE_FROM => 4.0,
            wide if wide >= COARSE_FROM => 2.0,
            _ => 1.0,
        };
        let back = Grid::new(expand(&grid.rect(), reach), None, 0.0, grid.scale / coarse, false)?;
        let area = back.rect();
        let behind: Vec<&Item> = behind.iter().copied().filter(|item| extent(item).is_some_and(|e| e.intersects(&area))).collect();
        let shader = if glass && gpu::ready() { program() } else { None };

        let mut hasher = DefaultHasher::new();
        (format!("{params:?}"), shader.is_some(), back.w, back.h, back.scale.to_bits()).hash(&mut hasher);
        if let Some(form) = &pane.form {
            // Where the form lies is in the key through the picture's grid; its own numbers go in here.
            (format!("{:?}", form.shapes), form.turn.map(f32::to_bits), form.merge.to_bits()).hash(&mut hasher);
        }
        let tag = format!("material-{:016x}", hasher.finish());
        let [left, top, right, bottom] = grid.span(&frame);
        let key = Self::key(&tag, &grid, &[(&behind, false), (shape, true)], &[left, top, right, bottom], self.background);
        let ground = raster::pixel(self.background);
        let form = pane.form.clone();
        let texture = self.baked.texture(self.ctx, key, grid.rect().union(&back.rect()), || {
            let behind = self.plan(&back, &behind, false);
            // A shape without a worked-out edge is cut out with what it paints, at twice the size and halved.
            let fine = Grid { fine: 2, ..grid };
            let painted = form.is_none().then(|| self.plan(&fine, shape, true));
            if behind.starved || painted.as_ref().is_some_and(|painted| painted.starved) {
                return None;
            }
            Some(Box::new(move || {
                let mut bitmap = back.bitmap();
                bitmap.fill(ground);
                behind.onto(&mut bitmap);
                for pixel in &mut bitmap.px {
                    *pixel = recode(*pixel, to_linear);
                }
                let (lightness, spread) = levels(&bitmap, &back, &frame);
                let params = if lightness < DARK_BELOW { params[1] } else { params[0] };
                let seen = Some(Seen { id: String::new(), dark: params.dark, lightness, spread, tint_opacity: params.tint_opacity });
                let rim = glass.then(|| {
                    let mut rim = bitmap.clone();
                    rim.blur(blur * RIM_BLUR * back.scale);
                    rim
                });
                bitmap.blur(blur * back.scale);
                let behind = Behind { grid: back, bitmap, rim };
                if let (Some(program), Some(form)) = (&shader, &form) {
                    match Self::glass(program, &grid, &behind, form, &params) {
                        Ok(bitmap) => return Made { bitmap, seen },
                        Err(error) => log::warn!("liquid glass: {error}; drawn frosted instead"),
                    }
                }
                let cut = painted.map(|painted| {
                    let mut solid = fine.bitmap();
                    painted.onto(&mut solid);
                    fine.settle(solid)
                });
                Made { bitmap: frosted(&grid, &behind, form.as_ref(), cut.as_ref(), &params), seen }
            }))
        });
        for id in &pane.ids {
            self.baked.note(key, id);
        }
        Some(Self::picture(texture, &grid, Point::default(), look.opacity, look.clip))
    }

    /// The glass shader run over `behind`, as a picture on `grid`.
    fn glass(program: &Program, grid: &Grid, behind: &Behind, form: &Form, params: &Params) -> Result<Bitmap, String> {
        let whole = |pixels: usize| u32::try_from(pixels).map_err(|e| e.to_string());
        let [cx, cy] = grid.place(form.centre);
        let area = behind.grid.rect();
        // Human: the shader reads the backdrop at an offset from the pane's centre in document units; this
        // scale and offset turn that into a place on the backdrop's picture.
        let map = [1.0 / area.w, 1.0 / area.h, (form.centre.x - area.x) / area.w, (form.centre.y - area.y) / area.h];
        let tint = linear(params.tint);
        let light = toward_light(params.light_angle);
        let maps = [(shader::map_member(BACKDROP), map), (shader::map_member(RIM), map)];
        // A rim can't be wider than half the narrowest outline.
        let narrowest = form.shapes.iter().map(|shape| shape.half[0].min(shape.half[1])).fold(f32::MAX, f32::min);
        let block = program.block([cx, cy, grid.scale, 0.0], &maps, |uniform| {
            let name = uniform.name.as_str();
            let numbered =
                |prefix: &str| name.strip_prefix(prefix).and_then(|index| index.parse::<usize>().ok()).and_then(|index| form.shapes.get(index));
            if let Some(shape) = numbered("u_box") {
                return [shape.offset[0], shape.offset[1], shape.half[0], shape.half[1]];
            }
            if let Some(shape) = numbered("u_round") {
                // An ellipse is told apart by a first radius below zero.
                return if shape.oval { [-1.0, 0.0, 0.0, 0.0] } else { shape.radii };
            }
            match name {
                "u_count" => [length(form.shapes.len()), 0.0, 0.0, 0.0],
                "u_merge" => [form.merge, 0.0, 0.0, 0.0],
                "u_turn" => form.turn,
                "u_scale" => [grid.scale, 0.0, 0.0, 0.0],
                "u_bevel" => [params.bevel.min(narrowest), 0.0, 0.0, 0.0],
                "u_thickness" => [params.thickness, 0.0, 0.0, 0.0],
                "u_refraction" => [params.refraction, 0.0, 0.0, 0.0],
                "u_dispersion" => [params.dispersion, 0.0, 0.0, 0.0],
                "u_saturation" => [params.saturation, 0.0, 0.0, 0.0],
                "u_brightness" => [params.brightness, 0.0, 0.0, 0.0],
                "u_tint" => [tint[0], tint[1], tint[2], params.tint_opacity],
                "u_highlight" => [params.highlight, 0.0, 0.0, 0.0],
                "u_light" => [light[0], light[1], 0.0, 0.0],
                _ => [0.0; 4],
            }
        });
        let encode = |bitmap: &Bitmap| Bitmap { w: bitmap.w, h: bitmap.h, px: bitmap.px.iter().map(|p| recode(*p, to_gamma)).collect() }.bytes();
        let (soft, sharp) = (encode(&behind.bitmap), encode(behind.rim.as_ref().unwrap_or(&behind.bitmap)));
        let (w, h) = (whole(behind.bitmap.w)?, whole(behind.bitmap.h)?);
        let inputs = [gpu::Input { rgba: &soft, w, h, mirror: true }, gpu::Input { rgba: &sharp, w, h, mirror: true }];
        let bytes = gpu::run(program, (whole(grid.w)?, whole(grid.h)?), &block, &inputs)?;
        Bitmap::from_bytes(grid.w, grid.h, &bytes).ok_or_else(|| "the GPU returned a picture of the wrong size".to_owned())
    }
}

/// `behind` tinted and edged on the CPU, as a picture on `grid`: the frosted material, and glass where no
/// shader can run. A shape without a worked-out edge (`form` is `None`) is cut out with `painted`, what the
/// shape paints on `grid`.
fn frosted(grid: &Grid, behind: &Behind, form: Option<&Form>, painted: Option<&Bitmap>, params: &Params) -> Bitmap {
    let tint = linear(params.tint);
    let mut out = grid.bitmap();
    let area = grid.rect();
    for y in 0..out.h {
        for x in 0..out.w {
            let doc = Point::new(area.x + (length(x) + 0.5) / grid.scale, area.y + (length(y) + 0.5) / grid.scale);
            let (cover, edge) = match (form, painted) {
                (Some(form), _) => {
                    let p = form.local(doc);
                    let distance = form.distance(p);
                    let cover = (0.5 - distance * grid.scale).clamp(0.0, 1.0);
                    // Far from the edge there is no light to work out.
                    let near = cover > 0.0 && -distance < HAIRLINE;
                    (cover, Some(if near { ((-distance).max(0.0), form.outward(p)) } else { (HAIRLINE, [0.0; 2]) }))
                }
                (None, Some(solid)) => (solid.px.get(y * solid.w + x).map_or(0.0, |p| p[3]), None),
                (None, None) => (0.0, None),
            };
            if cover > 0.0
                && let Some(pixel) = out.px.get_mut(y * out.w + x)
            {
                *pixel = frosted_pixel(behind.at(doc), params, tint, cover, edge);
            }
        }
    }
    out
}

#[cfg(test)]
mod drawn;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Material;
    use serde_json::json;

    fn form(oval: bool) -> Form {
        let shape = Shape { offset: [0.0; 2], half: [40.0, 20.0], radii: [20.0, 20.0, 4.0, 4.0], oval };
        Form { centre: Point::new(100.0, 50.0), turn: [1.0, 0.0, 0.0, 1.0], shapes: vec![shape], merge: 0.0 }
    }

    #[test]
    fn a_box_measures_the_distance_to_its_edge_with_each_corner_its_own_radius() {
        let form = form(false);
        assert!((form.distance([0.0, 0.0]) + 20.0).abs() < 1e-4);
        assert!((form.distance([50.0, 0.0]) - 10.0).abs() < 1e-4);
        // The top left corner is round (radius 20), so the corner point itself is outside; the bottom right
        // corner (radius 4) is nearly square.
        assert!(form.distance([-40.0, -20.0]) > 8.0);
        assert!(form.distance([40.0, 20.0]) < 2.0);
        assert!((form.outward([39.0, 0.0])[0] - 1.0).abs() < 1e-3);
    }

    #[test]
    fn an_oval_is_inside_at_its_centre_and_on_its_edge_at_its_ends() {
        let form = form(true);
        assert!(form.distance([0.0, 0.0]) < 0.0);
        assert!(form.distance([40.0, 0.0]).abs() < 1e-3 && form.distance([0.0, 20.0]).abs() < 1e-3);
        assert!(form.distance([60.0, 0.0]) > 0.0);
    }

    #[test]
    fn a_turned_layer_measures_in_its_own_directions() {
        // Turned a quarter: the layer's long side runs down the document.
        let turned = Form { turn: [0.0, -1.0, 1.0, 0.0], ..form(false) };
        let p = turned.local(Point::new(100.0, 85.0));
        assert!((p[0].abs() - 35.0).abs() < 1e-4 && p[1].abs() < 1e-4);
        assert!(turned.distance(p) < 0.0 && turned.distance(turned.local(Point::new(135.0, 50.0))) > 0.0);
    }

    #[test]
    fn outlines_closer_than_the_spacing_flow_together_and_farther_ones_stay_apart() {
        let circle = |x: f32| Shape { offset: [x, 0.0], half: [20.0, 20.0], radii: [20.0; 4], oval: false };
        let pair = |gap: f32, merge: f32| Form {
            centre: Point::new(0.0, 0.0),
            turn: [1.0, 0.0, 0.0, 1.0],
            shapes: vec![circle(-20.0 - gap / 2.0), circle(20.0 + gap / 2.0)],
            merge,
        };
        // Midway between two circles 8 apart: outside both (4 from each).
        assert!((pair(8.0, 0.0).distance([0.0, 0.0]) - 4.0).abs() < 1e-4);
        // With a spacing of 24 they have a bridge there; with their own gap as the spacing they don't touch.
        assert!(pair(8.0, 24.0).distance([0.0, 0.0]) < 0.0);
        assert!(pair(8.0, 8.0).distance([0.0, 0.0]) > 0.0);
        // Far from the gap nothing changes.
        assert!((pair(8.0, 24.0).distance([-44.0, 0.0]) - pair(8.0, 0.0).distance([-44.0, 0.0])).abs() < 1e-4);
        assert!((smooth_min(3.0, 9.0, 0.0) - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn the_colour_curves_undo_each_other_and_keep_the_alpha() {
        for v in [0.0, 0.002, 0.2, 0.5, 1.0] {
            assert!((to_gamma(to_linear(v)) - v).abs() < 1e-4);
        }
        // Read from their tables the curves are where the formulas put them, also at the steep dark end.
        for step in 0..=1000u16 {
            let v = f32::from(step) / 1000.0;
            let (linear, gamma) = (if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }, 1.055 * v.powf(1.0 / 2.4) - 0.055);
            assert!((to_linear(v) - linear).abs() < 1e-4, "{v}");
            assert!(v <= 0.003_130_8 || (to_gamma(v) - gamma).abs() < 2e-4, "{v}");
        }
        let half = recode([0.25, 0.25, 0.25, 0.5], to_linear);
        assert!((half[3] - 0.5).abs() < 1e-6 && (half[0] - to_linear(0.5) * 0.5).abs() < 1e-6);
        assert_eq!(recode([0.0; 4], to_linear), [0.0; 4]);
    }

    #[test]
    fn frost_keeps_grey_grey_and_a_tint_pulls_toward_its_colour() {
        let material = Material::read(&json!({"type": "frosted", "advanced": {"highlight": 0, "brightness": 0}})).unwrap();
        let lookup = Lookup { number: &|_| None, color: &|text| Rgba::parse(text) };
        let params = material.resolve((200.0, 200.0), false, &lookup);
        let grey = [0.2, 0.2, 0.2, 1.0];
        let out = frosted_pixel(grey, &params, [1.0; 3], 1.0, Some((HAIRLINE, [0.0; 2])));
        // A white tint at 55 % over 0.2 grey, then gamma-encoded.
        let expected = to_gamma(0.2 + 0.8 * params.tint_opacity);
        assert!((out[0] - expected).abs() < 1e-4 && (out[0] - out[1]).abs() < 1e-6 && (out[3] - 1.0).abs() < 1e-6);
        // Half covered at the edge: premultiplied by the cover.
        let edge = frosted_pixel(grey, &params, [1.0; 3], 0.5, None);
        assert!((edge[3] - 0.5).abs() < 1e-6 && (edge[0] - expected * 0.5).abs() < 1e-4);
        // Nothing behind (an export on a clear ground): only the tint is there.
        let clear = frosted_pixel([0.0; 4], &params, [1.0; 3], 1.0, None);
        assert!((clear[3] - params.tint_opacity).abs() < 1e-4);
    }

    #[test]
    fn the_light_comes_from_the_upper_left_by_default() {
        let light = toward_light(-45.0);
        assert!(light[0] < -0.7 && light[1] < -0.7);
        assert_eq!(toward_light(0.0).map(f32::round), [0.0, -1.0]);
    }

    #[test]
    fn levels_give_the_mean_and_the_spread_under_a_frame() {
        let grid = Grid::new(Rect::new(0.0, 0.0, 10.0, 10.0), None, 0.0, 1.0, false).unwrap();
        let mut bitmap = grid.bitmap();
        for (index, pixel) in bitmap.px.iter_mut().enumerate() {
            *pixel = if index % 10 < 5 { [0.0, 0.0, 0.0, 1.0] } else { [1.0, 1.0, 1.0, 1.0] };
        }
        let (mean, spread) = levels(&bitmap, &grid, &Rect::new(0.0, 0.0, 10.0, 10.0));
        assert!((mean - 0.5).abs() < 1e-4 && (spread - 1.0).abs() < 1e-4);
        // Only the dark half.
        let (mean, spread) = levels(&bitmap, &grid, &Rect::new(0.0, 0.0, 5.0, 10.0));
        assert!(mean < 1e-4 && spread < 1e-4);
    }

    #[test]
    fn the_glass_shader_compiles_and_has_every_uniform_the_renderer_sets() {
        let program = shader::compile(SHADER).unwrap();
        let names: Vec<&str> = program.uniforms.iter().map(|u| u.name.as_str()).collect();
        let mut wanted: Vec<String> = [
            BACKDROP,
            RIM,
            "u_count",
            "u_merge",
            "u_turn",
            "u_scale",
            "u_bevel",
            "u_thickness",
            "u_refraction",
            "u_dispersion",
            "u_saturation",
            "u_brightness",
            "u_tint",
            "u_highlight",
            "u_light",
        ]
        .map(str::to_owned)
        .to_vec();
        for index in 0..MOST_SHAPES {
            wanted.extend([format!("u_box{index}"), format!("u_round{index}")]);
        }
        for name in &wanted {
            assert!(names.contains(&name.as_str()), "{name} is missing from the shader");
        }
        assert_eq!(names.len(), wanted.len());
    }
}
