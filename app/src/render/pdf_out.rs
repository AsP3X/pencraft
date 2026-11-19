//! Writes layers as a PDF page of vectors: shapes as shapes and text as text in its own font, so the page
//! stays sharp at any size and its words can be selected and searched.
//!
//! What a PDF has a plain word for is written as that: solid, linear and radial fills, picture fills, strokes,
//! opacity, clipping, masks, icons and text. A layer that needs something it has none for (a shadow, a blur, a
//! pattern, an angular gradient, text along a circle or a line, an outline round text, a font that isn't
//! installed) is put on the page as a picture of itself, in its place. A design that shows what is behind a
//! layer through it (a material, a background blur, a shader) can't be taken apart that way, and is not written
//! here at all: its page stays one picture (`crate::export`).
// Agent: the PDF itself is written by craft-pdf's writer over Krilla (`craft_pdf::write::backend`); this file
// only says what to draw. MAIN-THREAD: text is laid out through the egui context, as on the canvas.

use std::collections::HashMap;
use std::path::{Path as FilePath, PathBuf};

use craft_pdf::write::backend::color::rgb;
use craft_pdf::write::backend::geom::{Path as PdfPath, PathBuilder, Point as PdfPoint, Size, Transform};
use craft_pdf::write::backend::image::Image;
use craft_pdf::write::backend::num::NormalizedF32;
use craft_pdf::write::backend::page::PageSettings;
use craft_pdf::write::backend::paint::{
    Fill, FillRule as PdfRule, LineCap as PdfCap, LineJoin as PdfJoin, LinearGradient, Paint as PdfPaint, RadialGradient, SpreadMethod, Stop,
    Stroke as PdfStroke,
};
use craft_pdf::write::backend::surface::Surface;
use craft_pdf::write::backend::text::{Font, TextDirection};
use craft_pdf::write::{self, Metadata, Writer};
use egui::Color32;
use lyon_tessellation::path::{Event, Path};
use lyon_tessellation::{FillRule, LineCap, LineJoin};
use pen_doc::color::Rgba;
use pen_doc::geom::{Affine, Point, Rect};
use pen_doc::layout::{self, NOTE_PADDING, NOTE_RADIUS, TextStyle};
use pen_doc::model::{Kind, Paint, StrokeWidth};
use pen_doc::scene::SceneNode;
use pen_doc::value::narrow;
use serde_json::Value;

use super::{
    Brush, Builder, GradientKind, NOTE_COLOR, NOTE_TEXT_CONTRAST, OnCircle, Pen, Track, build_path, pen_for, rounded_rect, text_corner, text_job,
};
use crate::effects::{self, EffectKind};
use crate::{faces, icons};

/// How many times its size a text is laid out at to find where its words stand: the rows' places are whole
/// pixels of that layout.
const LAID: f32 = 4.0;
/// How far a slanted upright leans: its letters move right by this share of their height.
const SLANT: f32 = 0.2;
/// The sharpest corner a stroke still draws as a point, as SVG has it.
const MITER_LIMIT: f32 = 4.0;

/// Why a page couldn't be written.
struct Failed(String);

impl From<write::WriteError> for Failed {
    fn from(error: write::WriteError) -> Self {
        Failed(error.to_string())
    }
}

/// A layer drawn as a picture, with where it lies in the document.
struct Shot {
    /// Four bytes a pixel, colour not multiplied by alpha.
    rgba: Vec<u8>,
    pixels: (u32, u32),
    at: Rect,
}

/// What a page being written has ready: the layers that go on it as pictures, by scene id, and the fonts and
/// picture files read so far (`None` for one that couldn't be read).
#[derive(Default)]
struct Kit {
    shots: HashMap<String, Shot>,
    fonts: HashMap<(PathBuf, u32, u16), Option<Font>>,
    files: HashMap<PathBuf, Option<Image>>,
}

impl Kit {
    /// The font of `face` for text of `weight`, read and made ready once.
    // Agent: READS the font file whole the first time. A variable face is set to the weight, within its range.
    fn font(&mut self, face: &faces::Face, weight: u16) -> Option<Font> {
        let set = face.weights.map(|(lightest, heaviest)| f32::from(weight).clamp(lightest, heaviest));
        let key = (face.path.clone(), face.index, if set.is_some() { weight } else { 0 });
        self.fonts
            .entry(key)
            .or_insert_with(|| {
                let bytes = std::fs::read(&face.path).ok()?;
                let coords = set.map(|weight| [(*b"wght", weight)]);
                write::font(&bytes, face.index, coords.as_ref().map(<[_; 1]>::as_slice)).ok()
            })
            .clone()
    }

    /// The picture in the file at `path`, by what its first bytes say it is.
    // Agent: READS the file whole the first time.
    fn picture(&mut self, path: &FilePath) -> Option<Image> {
        self.files
            .entry(path.to_path_buf())
            .or_insert_with(|| {
                let bytes = std::fs::read(path).ok()?;
                let read = match bytes.as_slice() {
                    [0x89, b'P', b'N', b'G', ..] => Image::from_png(bytes.into(), true),
                    [0xff, 0xd8, ..] => Image::from_jpeg(bytes.into(), true),
                    [b'G', b'I', b'F', ..] => Image::from_gif(bytes.into(), true),
                    [b'R', b'I', b'F', b'F', ..] => Image::from_webp(bytes.into(), true),
                    _ => return None,
                };
                read.ok()
            })
            .clone()
    }
}

/// A share between 0 and 1 as the PDF writer takes it.
fn share(value: f32) -> NormalizedF32 {
    NormalizedF32::new(value.clamp(0.0, 1.0)).unwrap_or(NormalizedF32::ONE)
}

/// A colour as a PDF paint and how solid it is.
fn ink(c: Rgba) -> (PdfPaint, NormalizedF32) {
    (rgb::Color::new(c.r, c.g, c.b).into(), share(f32::from(c.a) / 255.0))
}

/// A transform as the PDF writer takes it.
fn transform(affine: &Affine) -> Transform {
    let m = affine.m;
    Transform::from_row(m[0], m[1], m[2], m[3], m[4], m[5])
}

/// An outline as the PDF writer takes it, curves kept as curves; `None` for an outline without a piece.
fn outline(shape: &Path) -> Option<PdfPath> {
    let mut builder = PathBuilder::new();
    for event in shape {
        match event {
            Event::Begin { at } => builder.move_to(at.x, at.y),
            Event::Line { to, .. } => builder.line_to(to.x, to.y),
            Event::Quadratic { ctrl, to, .. } => builder.quad_to(ctrl.x, ctrl.y, to.x, to.y),
            Event::Cubic { ctrl1, ctrl2, to, .. } => builder.cubic_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y),
            Event::End { close, .. } => {
                if close {
                    builder.close();
                }
            }
        }
    }
    builder.finish()
}

fn rule_of(rule: FillRule) -> PdfRule {
    if rule == FillRule::EvenOdd { PdfRule::EvenOdd } else { PdfRule::NonZero }
}

/// What `brush` paints a shape of `size` with, and how solid; `None` for a brush a PDF has no word for (those
/// layers are pictures) and for a picture, which is drawn as itself.
// Human: a gradient is said in shares of the layer's box, as the file has it, and stretched onto the box: a
// gradient across a wide box is squeezed the same way the canvas squeezes it.
fn paint_of(brush: &Brush, size: (f32, f32)) -> Option<(PdfPaint, NormalizedF32)> {
    match brush {
        Brush::Solid(c) => Some(ink(*c)),
        Brush::Pattern(_) | Brush::Image { .. } | Brush::Gradient { kind: GradientKind::Angular, .. } => None,
        Brush::Gradient { kind, center, size: reach, rotation, stops, opacity } => {
            let first = stops.first()?;
            if stops.len() == 1 {
                return Some(ink(first.1.with_opacity(*opacity)));
            }
            // Stops the file puts outside 0 to 1 are pulled in; the colours before the first and after the
            // last go on to the edges, as on the canvas.
            let stops: Vec<Stop> = stops
                .iter()
                .map(|(at, c)| Stop {
                    offset: share(*at),
                    color: rgb::Color::new(c.r, c.g, c.b).into(),
                    opacity: share(f32::from(c.a) / 255.0 * opacity),
                })
                .collect();
            let onto_box = Affine { m: [size.0.max(1e-4), 0.0, 0.0, size.1.max(1e-4), 0.0, 0.0] };
            let paint: PdfPaint = if matches!(kind, GradientKind::Linear) {
                let (sin, cos) = rotation.to_radians().sin_cos();
                let (dx, dy) = (-sin * reach.y / 2.0, -cos * reach.y / 2.0);
                LinearGradient {
                    x1: center.x - dx,
                    y1: center.y - dy,
                    x2: center.x + dx,
                    y2: center.y + dy,
                    transform: transform(&onto_box),
                    spread_method: SpreadMethod::Pad,
                    stops,
                    anti_alias: false,
                }
                .into()
            } else {
                // A circle of radius 1 round the origin, stretched to the gradient's reach about its centre.
                let round = Affine { m: [(reach.x / 2.0).max(1e-4), 0.0, 0.0, (reach.y / 2.0).max(1e-4), center.x, center.y] };
                RadialGradient {
                    fx: 0.0,
                    fy: 0.0,
                    fr: 0.0,
                    cx: 0.0,
                    cy: 0.0,
                    cr: 1.0,
                    transform: transform(&onto_box.then(&round)),
                    spread_method: SpreadMethod::Pad,
                    stops,
                    anti_alias: false,
                }
                .into()
            };
            Some((paint, NormalizedF32::ONE))
        }
    }
}

/// Fills `shape` on `surface` with `paint`.
fn fill(surface: &mut Surface<'_>, shape: &Path, rule: FillRule, paint: (PdfPaint, NormalizedF32)) {
    let Some(path) = outline(shape) else { return };
    surface.set_stroke(None);
    surface.set_fill(Some(Fill { paint: paint.0, opacity: paint.1, rule: rule_of(rule) }));
    surface.draw_path(&path);
}

/// Draws the line of `shape` on `surface` with `pen` in `paint`.
fn stroke(surface: &mut Surface<'_>, shape: &Path, pen: Pen, paint: (PdfPaint, NormalizedF32)) {
    let Some(path) = outline(shape) else { return };
    let line_cap = match pen.cap {
        LineCap::Round => PdfCap::Round,
        LineCap::Square => PdfCap::Square,
        LineCap::Butt => PdfCap::Butt,
    };
    let line_join = match pen.join {
        LineJoin::Round => PdfJoin::Round,
        LineJoin::Bevel => PdfJoin::Bevel,
        LineJoin::Miter | LineJoin::MiterClip => PdfJoin::Miter,
    };
    surface.set_fill(None);
    surface.set_stroke(Some(PdfStroke {
        paint: paint.0,
        width: pen.width,
        miter_limit: MITER_LIMIT,
        line_cap,
        line_join,
        opacity: paint.1,
        dash: None,
    }));
    surface.draw_path(&path);
}

impl Builder<'_> {
    /// `scenes` (layers, back to front) as a PDF of one page as large as the box around them and whatever
    /// reaches past it. `None` when the design shows what is behind a layer through it, which only a picture of
    /// the whole page can show.
    ///
    /// # Errors
    /// A sentence when the layers have no size or the PDF can't be made.
    pub(super) fn pdf(&self, scenes: &[&SceneNode]) -> Option<Result<Vec<u8>, String>> {
        if scenes.iter().any(|scene| self.pdf_reads_behind(scene)) {
            return None;
        }
        Some(write::guard(|| self.pdf_page(scenes)).map_err(|Failed(why)| why))
    }

    /// The scene ids of the layers of `scenes` that go on the page as pictures.
    #[cfg(test)]
    pub(super) fn pdf_pictured(&self, scenes: &[&SceneNode]) -> Vec<String> {
        let mut kit = Kit::default();
        for scene in scenes {
            self.pdf_shots(scene, &mut kit);
        }
        let mut ids: Vec<String> = kit.shots.into_keys().collect();
        ids.sort();
        ids
    }

    fn pdf_page(&self, scenes: &[&SceneNode]) -> Result<Vec<u8>, Failed> {
        let mut kit = Kit::default();
        for scene in scenes {
            self.pdf_shots(scene, &mut kit);
        }
        let boxes = scenes.iter().map(|scene| layout::doc_bounds(scene)).chain(kit.shots.values().map(|shot| shot.at));
        let area = boxes
            .reduce(|a, b| a.union(&b))
            .filter(|area| area.w > 0.0 && area.h > 0.0)
            .ok_or_else(|| Failed("The layers have no size to export.".to_owned()))?;
        let page = PageSettings::from_wh(area.w, area.h).ok_or_else(|| Failed("The layers have no size to export.".to_owned()))?;
        let about = Metadata { creator: "Pencraft".to_owned(), producer: format!("Pencraft {}", env!("CARGO_PKG_VERSION")), ..Metadata::default() };
        // Human: colours are written as sRGB with its profile in the file, not as "whatever this device shows":
        // a viewer then shows the colours the canvas shows.
        let settings = write::backend::SerializeSettings { no_device_cs: true, ..write::backend::SerializeSettings::default() };
        let mut writer = Writer::new(settings, about);
        {
            let mut page = writer.document_mut().start_page_with(page);
            let mut surface = page.surface();
            surface.push_transform(&Transform::from_translate(-area.x, -area.y));
            for scene in scenes {
                self.pdf_node(scene, &mut surface, &mut kit);
            }
            surface.pop();
            surface.finish();
            page.finish();
        }
        Ok(writer.finish()?)
    }

    fn pdf_off(&self, scene: &SceneNode) -> bool {
        self.hides(scene) || scene.node.enabled.as_ref().and_then(|on| self.vars.flag(on, &scene.theme)) == Some(false)
    }

    /// Whether `scene` or a layer in it draws what lies behind it.
    fn pdf_reads_behind(&self, scene: &SceneNode) -> bool {
        if self.pdf_off(scene) {
            return false;
        }
        let through =
            |effect: &Value| !effects::is_off(effect) && matches!(EffectKind::of(effect), Some(EffectKind::BackgroundBlur | EffectKind::Shader));
        scene.node.material.is_some()
            || effects::list(scene.node.effect.as_ref()).iter().any(through)
            || scene.children.iter().any(|child| self.pdf_reads_behind(child))
    }

    /// Whether `scene` needs something a PDF has no word for, and goes on the page as a picture of itself.
    fn pdf_as_picture(&self, scene: &SceneNode, kit: &mut Kit) -> bool {
        let (node, theme) = (&scene.node, &scene.theme);
        if effects::list(node.effect.as_ref()).iter().any(|effect| !effects::is_off(effect)) {
            return true;
        }
        let fills = node.fill.as_ref().map(|f| self.brushes(f.as_slice(), theme)).unwrap_or_default();
        let strokes = node.stroke.as_ref().map(|f| self.brushes(f.as_slice(), theme)).unwrap_or_default();
        let wordless = |brush: &Brush| matches!(brush, Brush::Pattern(_) | Brush::Gradient { kind: GradientKind::Angular, .. });
        if fills.iter().chain(&strokes).any(wordless) || strokes.iter().any(|brush| matches!(brush, Brush::Image { .. })) {
            return true;
        }
        if !matches!(node.kind, Kind::Text | Kind::Note) {
            return false;
        }
        let style = layout::text_style(node, self.vars, theme);
        let bent =
            node.kind == Kind::Text && (OnCircle::of(node).is_some() || Track::of(node).is_some() || !self.text_outlines(scene, &strokes).is_empty());
        let face = faces::pick(&style.family, style.weight, style.italic);
        bent || !style.content.is_empty() && face.is_none_or(|face| kit.font(face, style.weight).is_none())
    }

    /// Draws the layers of `scene` that go on the page as pictures, and keeps them in `kit`.
    // Agent: each is drawn by the canvas's own renderer at the builder's scale, with its effects.
    fn pdf_shots(&self, scene: &SceneNode, kit: &mut Kit) {
        if self.pdf_off(scene) {
            return;
        }
        if !self.pdf_as_picture(scene, kit) {
            for child in &scene.children {
                self.pdf_shots(child, kit);
            }
            return;
        }
        let Some((picture, origin)) = self.snapshot_at(&[scene]) else { return };
        let (Ok(w), Ok(h)) = (u32::try_from(picture.w), u32::try_from(picture.h)) else { return };
        let at = Rect::new(origin.x, origin.y, crate::export::narrow_len(picture.w) / self.scale, crate::export::narrow_len(picture.h) / self.scale);
        kit.shots.insert(scene.id.clone(), Shot { rgba: picture.straight(), pixels: (w, h), at });
    }

    /// Writes `scene` and what is in it.
    fn pdf_node(&self, scene: &SceneNode, surface: &mut Surface<'_>, kit: &mut Kit) {
        if self.pdf_off(scene) {
            return;
        }
        if let Some(shot) = kit.shots.get(&scene.id) {
            // Human: the picture has the layer's own opacity and everything in it already.
            if let Some(size) = Size::from_wh(shot.at.w, shot.at.h) {
                surface.push_transform(&Transform::from_translate(shot.at.x, shot.at.y));
                surface.draw_image(Image::from_rgba8(shot.rgba.clone(), shot.pixels.0, shot.pixels.1), size);
                surface.pop();
            }
            return;
        }
        let (node, theme) = (&scene.node, &scene.theme);
        let opacity = self.num(node.opacity.as_ref(), theme).unwrap_or(1.0).clamp(0.0, 1.0);
        let faded = opacity < 1.0;
        if faded {
            surface.push_opacity(share(opacity));
        }
        let place = transform(&scene.to_doc);
        surface.push_transform(&place);
        self.pdf_own(scene, surface, kit);
        surface.pop();
        // Children, cut to the frame when it clips.
        let clips = node.clip.as_ref().and_then(|c| self.vars.flag(c, theme)).unwrap_or(false) && !scene.children.is_empty();
        let cut = if clips { self.shape_of(scene).and_then(|(shape, rule)| Some((outline(&shape)?.transform(place)?, rule))) } else { None };
        if let Some((path, rule)) = &cut {
            surface.push_clip_path(path, &rule_of(*rule));
        }
        self.pdf_children(scene, surface, kit);
        if cut.is_some() {
            surface.pop();
        }
        if faded {
            surface.pop();
        }
    }

    /// Writes the children of `scene`; the layers after a mask layer are cut to its shape.
    fn pdf_children(&self, scene: &SceneNode, surface: &mut Surface<'_>, kit: &mut Kit) {
        let mut masked = false;
        for child in &scene.children {
            // A second mask layer ends what the first one cuts: it is written whole, and cuts what follows.
            let mask = if super::is_mask(&child.node) && !self.pdf_off(child) { self.shape_of(child) } else { None };
            let mask = mask.and_then(|(shape, rule)| Some((outline(&shape)?.transform(transform(&child.to_doc))?, rule)));
            if mask.is_some() && std::mem::take(&mut masked) {
                surface.pop();
            }
            self.pdf_node(child, surface, kit);
            if let Some((path, rule)) = mask {
                surface.push_clip_path(&path, &rule_of(rule));
                masked = true;
            }
        }
        if masked {
            surface.pop();
        }
    }

    /// Writes what `scene` draws of its own, in its own coordinates: its fills, its strokes, its words.
    fn pdf_own(&self, scene: &SceneNode, surface: &mut Surface<'_>, kit: &mut Kit) {
        let (node, theme) = (&scene.node, &scene.theme);
        let (w, h) = (scene.local.w, scene.local.h);
        let strokes = node.stroke.as_ref().map(|f| self.brushes(f.as_slice(), theme)).unwrap_or_default();
        let solid = |brushes: &[Brush]| brushes.iter().find_map(|b| if let Brush::Solid(c) = b { Some(*c) } else { None }).unwrap_or(Rgba::BLACK);
        let width = self.stroke_width(scene);
        let lines = |surface: &mut Surface<'_>, shape: &Path| {
            if width > 0.0 {
                for paint in strokes.iter().filter_map(|brush| paint_of(brush, (w, h))) {
                    stroke(surface, shape, pen_for(node, width), paint);
                }
            }
        };
        match node.kind {
            Kind::Frame | Kind::Rectangle | Kind::Browser => {
                let radii = self.corner_radii(node, theme, w, h);
                let shape = rounded_rect(0.0, 0.0, w, h, radii);
                if node.kind == Kind::Browser && node.fill.is_none() {
                    fill(surface, &shape, FillRule::NonZero, ink(Rgba::WHITE));
                }
                self.pdf_fills(scene, &shape, FillRule::NonZero, surface, kit);
                self.pdf_box_strokes(scene, &strokes, radii, surface);
            }
            Kind::Note => {
                let color = node.color.as_deref().and_then(|c| self.vars.color(c, theme)).unwrap_or(NOTE_COLOR);
                fill(surface, &rounded_rect(0.0, 0.0, w, h, [NOTE_RADIUS; 4]), FillRule::NonZero, ink(Rgba { a: 255, ..color }));
                let mut style = layout::text_style(node, self.vars, theme);
                style.wrap = Some(w - 2.0 * NOTE_PADDING);
                let words = if color.contrast(Rgba::BLACK) < NOTE_TEXT_CONTRAST { Rgba::WHITE } else { Rgba { r: 0x1c, g: 0x1c, b: 0x1c, a: 255 } };
                let inner = Rect::new(NOTE_PADDING, NOTE_PADDING, w - 2.0 * NOTE_PADDING, h - 2.0 * NOTE_PADDING);
                Self::pdf_text(self.ctx, &style, words, inner, surface, kit);
            }
            Kind::Ellipse | Kind::Polygon | Kind::Path => {
                let Some((shape, rule)) = self.shape_of(scene) else { return };
                self.pdf_fills(scene, &shape, rule, surface, kit);
                lines(surface, &shape);
            }
            // Human: a connector is only ever stroked; the filled shapes at its ends take the line's colour.
            Kind::Connector => {
                let part =
                    |data: Option<&str>| data.and_then(|data| pen_doc::path::parse(data).ok()).map(|segments| super::fit_path(&segments, node, w, h));
                if let Some(shape) = part(node.geometry.as_deref()) {
                    lines(surface, &shape);
                }
                if let Some(ends) = part(node.extra.get(pen_doc::connector::MARKERS).and_then(Value::as_str)) {
                    for paint in strokes.iter().filter_map(|brush| paint_of(brush, (w, h))) {
                        fill(surface, &ends, FillRule::NonZero, paint);
                    }
                }
            }
            Kind::Line => {
                let mut line = Path::builder();
                line.begin(lyon_tessellation::math::point(0.0, 0.0));
                line.line_to(lyon_tessellation::math::point(w, h));
                line.end(false);
                lines(surface, &line.build());
            }
            Kind::Text => {
                let mut style = layout::text_style(node, self.vars, theme);
                if matches!(node.text_growth.as_deref(), Some("fixed-width" | "fixed-width-height")) {
                    style.wrap = Some(w);
                }
                let fills = node.fill.as_ref().map(|f| self.brushes(f.as_slice(), theme)).unwrap_or_default();
                Self::pdf_text(self.ctx, &style, solid(&fills), Rect::new(0.0, 0.0, w, h), surface, kit);
            }
            Kind::Icon | Kind::IconFont => {
                let fills = node.fill.as_ref().map(|f| self.brushes(f.as_slice(), theme)).unwrap_or_default();
                let old = |key: &str| node.extra.get(key).and_then(Value::as_str);
                let library = node.library.as_deref().or_else(|| old("iconFontFamily")).unwrap_or("lucide");
                let name = node.icon.as_deref().or_else(|| old("iconFontName"));
                if let Some(segments) = name.and_then(|name| icons::outline(library, name)) {
                    let shape = build_path(&segments, Rect::new(0.0, 0.0, icons::VIEW_BOX, icons::VIEW_BOX), w, h);
                    let pen = Pen { width: icons::LINE_WIDTH * w.min(h) / icons::VIEW_BOX, cap: LineCap::Round, join: LineJoin::Round };
                    stroke(surface, &shape, pen, ink(solid(&fills)));
                } else {
                    // An icon that isn't bundled shows as a small rounded outline, as on the canvas.
                    let inset = w.min(h) * 0.2;
                    let shape = rounded_rect(inset, inset, (w - 2.0 * inset).max(0.0), (h - 2.0 * inset).max(0.0), [w.min(h) * 0.15; 4]);
                    stroke(surface, &shape, pen_for(node, (w.min(h) * 0.08).max(0.5)), ink(solid(&fills)));
                }
            }
            Kind::Ref | Kind::Group | Kind::Script | Kind::Other(_) => {}
        }
    }

    /// Fills `shape` with each of the layer's fills in turn; a picture fill is the picture itself.
    fn pdf_fills(&self, scene: &SceneNode, shape: &Path, rule: FillRule, surface: &mut Surface<'_>, kit: &mut Kit) {
        let (node, theme) = (&scene.node, &scene.theme);
        let size = (scene.local.w, scene.local.h);
        for paint in node.fill.iter().flat_map(pen_doc::model::Paints::as_slice) {
            let Some(brush) = self.brushes(std::slice::from_ref(paint), theme).into_iter().next() else { continue };
            if let Some(paint) = paint_of(&brush, size) {
                fill(surface, shape, rule, paint);
                continue;
            }
            let Brush::Image { picture, fit, crop, opacity } = brush else { continue };
            let file = match paint {
                Paint::Object(object) => object.get("url").and_then(Value::as_str).and_then(|url| crate::images::resolve(self.folder, url)),
                Paint::Solid(_) => None,
            };
            // Human: placed as the canvas places it: the crop's box lands where the picture's fit puts it, and
            // the picture's unit square is carried along with it; what lies outside that box or the shape is cut.
            let cropped = (picture.width * crop.m[0].hypot(crop.m[1]), picture.height * crop.m[2].hypot(crop.m[3]));
            let place = fit.place(cropped, size);
            let onto = Affine { m: [place.w, 0.0, 0.0, place.h, place.x, place.y] };
            let (Some(image), Some(whole), Some(cut), Some(unit)) =
                (file.and_then(|file| kit.picture(&file)), crop.invert(), outline(shape), Size::from_wh(1.0, 1.0))
            else {
                fill(surface, shape, rule, ink(Rgba { r: 0xd9, g: 0xd9, b: 0xd9, a: 0xff }.with_opacity(opacity)));
                continue;
            };
            surface.push_clip_path(&cut, &rule_of(rule));
            if let Some(edge) = outline(&rounded_rect(place.x, place.y, place.w, place.h, [0.0; 4])) {
                surface.push_clip_path(&edge, &PdfRule::NonZero);
                surface.push_opacity(share(opacity));
                surface.push_transform(&transform(&onto.then(&whole)));
                surface.draw_image(image, unit);
                surface.pop();
                surface.pop();
                surface.pop();
            }
            surface.pop();
        }
    }

    /// A box's strokes: each side as a strip of its own when the sides have widths of their own, else one line
    /// on the edge, inside it or outside it as the layer says.
    fn pdf_box_strokes(&self, scene: &SceneNode, strokes: &[Brush], radii: [f32; 4], surface: &mut Surface<'_>) {
        let (w, h) = (scene.local.w, scene.local.h);
        if let Some(StrokeWidth::Sides(sides)) = &scene.node.stroke_width {
            let side = |key: &str| sides.get(key).and_then(Value::as_f64).map_or(0.0, narrow).max(0.0);
            let [top, right, bottom, left] = ["top", "right", "bottom", "left"].map(side);
            let strips = [
                Rect::new(0.0, 0.0, w, top),
                Rect::new(w - right, 0.0, right, h),
                Rect::new(0.0, h - bottom, w, bottom),
                Rect::new(0.0, 0.0, left, h),
            ];
            for paint in strokes.iter().filter_map(|brush| paint_of(brush, (w, h))) {
                for strip in strips.iter().filter(|strip| strip.w > 0.0 && strip.h > 0.0) {
                    fill(surface, &rounded_rect(strip.x, strip.y, strip.w, strip.h, [0.0; 4]), FillRule::NonZero, paint.clone());
                }
            }
            return;
        }
        let width = self.stroke_width(scene);
        if width <= 0.0 {
            return;
        }
        let offset = match scene.node.stroke_alignment.as_deref() {
            Some("inner") => width / 2.0,
            Some("outer") => -width / 2.0,
            _ => 0.0,
        };
        let shape = rounded_rect(offset, offset, (w - 2.0 * offset).max(0.0), (h - 2.0 * offset).max(0.0), radii.map(|r| (r - offset).max(0.0)));
        for paint in strokes.iter().filter_map(|brush| paint_of(brush, (w, h))) {
            stroke(surface, &shape, pen_for(&scene.node, width), paint);
        }
    }

    /// Writes a text's words into `room` (the layer's box, or a sticky note's inside) in `color`.
    // Human: the text is laid out as the canvas lays it out, so every line breaks where it breaks there. Each
    // word is written where it stands, in the text's own font, which goes into the file: the words can be
    // selected and searched, and look the same on a computer without the font. With letter spacing every
    // letter is written at its own place. An upright standing in for a missing italic is leant over.
    fn pdf_text(ctx: &egui::Context, style: &TextStyle, color: Rgba, room: Rect, surface: &mut Surface<'_>, kit: &mut Kit) {
        if style.content.is_empty() {
            return;
        }
        let Some(face) = faces::pick(&style.family, style.weight, style.italic) else { return };
        let Some(font) = kit.font(face, style.weight) else { return };
        let leans = style.italic && !face.italic;
        let galley = ctx.fonts_mut(|fonts| fonts.layout_job(text_job(style, Color32::WHITE, LAID)));
        let corner = text_corner(style, (room.w, room.h), galley.size().y / LAID);
        let (paint, solid) = ink(color);
        let spaced = style.letter_spacing.abs() > f32::EPSILON;
        let rule = (style.size / 16.0).max(1.0);
        for row in &galley.rows {
            let at = |glyph: &egui::epaint::text::Glyph| {
                Point::new(room.x + corner.x + (row.pos.x + glyph.pos.x) / LAID, room.y + corner.y + (row.pos.y + glyph.pos.y) / LAID)
            };
            surface.set_stroke(None);
            surface.set_fill(Some(Fill { paint: paint.clone(), opacity: solid, rule: PdfRule::NonZero }));
            let mut word: Option<(Point, String)> = None;
            let write = |surface: &mut Surface<'_>, word: Option<(Point, String)>| {
                let Some((start, text)) = word else { return };
                surface.push_transform(&Transform::from_translate(start.x, start.y));
                if leans {
                    surface.push_transform(&Transform::from_skew(-SLANT, 0.0));
                }
                surface.draw_text(PdfPoint::from_xy(0.0, 0.0), font.clone(), style.size, &text, false, TextDirection::Auto);
                if leans {
                    surface.pop();
                }
                surface.pop();
            };
            for glyph in &row.row.glyphs {
                if glyph.chr.is_whitespace() {
                    write(surface, word.take());
                    continue;
                }
                match &mut word {
                    Some((_, text)) if !spaced => text.push(glyph.chr),
                    _ => {
                        write(surface, word.take());
                        word = Some((at(glyph), glyph.chr.to_string()));
                    }
                }
            }
            write(surface, word.take());
            // The line under or through a row runs from its first letter to the end of its last.
            let (Some(first), Some(last)) = (row.row.glyphs.first(), row.row.glyphs.last()) else { continue };
            let (from, to) = (at(first), at(last).x + last.advance_width / LAID);
            for lift in [style.underline.then_some(style.size * 0.12), style.strikethrough.then_some(-style.size * 0.28)].into_iter().flatten() {
                let line = rounded_rect(from.x, from.y + lift - rule / 2.0, to - from.x, rule, [0.0; 4]);
                fill(surface, &line, FillRule::NonZero, (paint.clone(), solid));
            }
        }
    }
}
