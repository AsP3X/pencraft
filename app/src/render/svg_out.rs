//! Writes a layer as an SVG file: the shapes as shapes, so the result stays sharp and can be edited elsewhere.
//!
//! Each layer becomes one or more SVG elements under the layer's own transform. What SVG has a plain word for is
//! written (solid, linear and radial fills, strokes, corner radii, opacity, clipping, text, drop shadows, layer
//! blur); what it has none for is left out and listed in `MECHANICS.md`.

use std::fmt::Write as _;

use pen_doc::color::Rgba;
use pen_doc::geom::Rect;
use pen_doc::layout::{self, NOTE_PADDING, NOTE_RADIUS, TextAlign};
use pen_doc::model::{Kind, Paint};
use pen_doc::scene::SceneNode;
use pen_doc::value::{Num, narrow};
use serde_json::Value;

use super::{Brush, Builder, GradientKind, NOTE_COLOR, polygon_points};
use crate::effects::{self, EffectKind};
use crate::icons;

/// The file being written: the definitions (gradients, clips, filters) and the drawing, and a counter for ids.
#[derive(Default)]
struct Sheet {
    defs: String,
    body: String,
    next: usize,
}

/// How much of what a material leaves clear an SVG fills in, since it can't blur what is behind.
const FROST_STAND_IN: f32 = 0.35;

impl Sheet {
    fn id(&mut self, kind: &str) -> String {
        self.next += 1;
        format!("{kind}{}", self.next)
    }
}

/// Text made safe to stand in an attribute or between tags.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A number as SVG writes it: at most three decimals, no trailing zeros.
fn n(v: f32) -> String {
    let rounded = (f64::from(v) * 1000.0).round() / 1000.0;
    if rounded == 0.0 { "0".to_owned() } else { format!("{rounded}") }
}

/// A colour as `fill="…"`-style attributes named `what` (`fill` or `stroke`), with its opacity when it has one.
fn color(what: &str, c: Rgba) -> String {
    let mut out = format!(" {what}=\"#{:02x}{:02x}{:02x}\"", c.r, c.g, c.b);
    if c.a < 255 {
        let _ = write!(out, " {what}-opacity=\"{}\"", n(f32::from(c.a) / 255.0));
    }
    out
}

impl Builder<'_> {
    /// `scenes` (layers, back to front) as one SVG document as large as the box around them, or `None` when
    /// that has no size.
    // Human: the file is as large as the layer's box; a shadow or a stroke reaching past it is drawn but may
    // be cut by a viewer that clips to the box, as with any SVG.
    pub(super) fn svg(&self, scenes: &[&SceneNode]) -> Option<String> {
        let bounds = scenes.iter().map(|scene| layout::doc_bounds(scene)).reduce(|a, b| a.union(&b))?;
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return None;
        }
        let mut sheet = Sheet::default();
        for scene in scenes {
            self.svg_node(scene, &mut sheet);
        }
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{x} {y} {w} {h}\" overflow=\"visible\">\n",
            x = n(bounds.x),
            y = n(bounds.y),
            w = n(bounds.w),
            h = n(bounds.h)
        );
        if !sheet.defs.is_empty() {
            let _ = writeln!(out, "<defs>\n{}</defs>", sheet.defs);
        }
        out.push_str(&sheet.body);
        out.push_str("</svg>\n");
        Some(out)
    }

    /// The fill or stroke attributes for `brush` on a `w` × `h` shape; a gradient is added to the definitions.
    fn svg_paint(what: &str, brush: &Brush, sheet: &mut Sheet) -> String {
        match brush {
            Brush::Solid(c) => color(what, *c),
            // Human: a picture is written as its own `<image>`; here it only keeps the shape from being black.
            Brush::Image { .. } => format!(" {what}=\"none\""),
            Brush::Pattern(pattern) => {
                let id = sheet.id("p");
                let _ = writeln!(sheet.defs, "{}", pattern.svg(&id, &n));
                format!(" {what}=\"url(#{id})\"")
            }
            Brush::Gradient { kind, center, size, rotation, stops, opacity } => {
                let Some(first) = stops.first() else { return format!(" {what}=\"none\"") };
                let id = sheet.id("g");
                let mut stops_text = String::new();
                for (at, c) in stops {
                    let c = c.with_opacity(*opacity);
                    let _ = write!(
                        stops_text,
                        "<stop offset=\"{}\" stop-color=\"#{:02x}{:02x}{:02x}\" stop-opacity=\"{}\"/>",
                        n(*at),
                        c.r,
                        c.g,
                        c.b,
                        n(f32::from(c.a) / 255.0)
                    );
                }
                match kind {
                    // Human: the gradient's line runs through its centre, `size.y` long, turned by `rotation`
                    // (0° points up, counter-clockwise), from the far end to the near one, as the canvas draws it.
                    GradientKind::Linear => {
                        let (sin, cos) = rotation.to_radians().sin_cos();
                        let (dx, dy) = (-sin * size.y / 2.0, -cos * size.y / 2.0);
                        let _ = writeln!(
                            sheet.defs,
                            "<linearGradient id=\"{id}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">{stops_text}</linearGradient>",
                            n(center.x - dx),
                            n(center.y - dy),
                            n(center.x + dx),
                            n(center.y + dy)
                        );
                    }
                    GradientKind::Radial => {
                        let _ = writeln!(
                            sheet.defs,
                            "<radialGradient id=\"{id}\" cx=\"0.5\" cy=\"0.5\" r=\"0.5\" gradientTransform=\"translate({} {}) scale({} {}) translate(-0.5 -0.5)\">{stops_text}</radialGradient>",
                            n(center.x),
                            n(center.y),
                            n(size.x),
                            n(size.y)
                        );
                    }
                    // SVG has no angular gradient: its first colour stands in.
                    GradientKind::Angular => return color(what, first.1.with_opacity(*opacity)),
                }
                format!(" {what}=\"url(#{id})\"")
            }
        }
    }

    /// Path data of `node` as a `<path` element without its closing `/>`, fitted from the node's view box onto
    /// its `w` × `h` box.
    // Human: a path is written in its view box and stretched onto the layer's box. The stretch is worked into
    // the path's own numbers: the element's one `transform` is the layer's place.
    #[expect(clippy::unused_self, reason = "reads nothing of the builder, but belongs with its other shape writers")]
    fn svg_path(&self, node: &pen_doc::Node, data: &str, w: f32, h: f32) -> Option<String> {
        let segments = pen_doc::path::parse(data).ok()?;
        let view =
            node.view_box.map(|v| Rect::new(narrow(v[0]), narrow(v[1]), narrow(v[2]), narrow(v[3]))).or_else(|| pen_doc::path::bounds(&segments))?;
        let (sx, sy) = (if view.w > 1e-6 { w / view.w } else { 1.0 }, if view.h > 1e-6 { h / view.h } else { 1.0 });
        let at = |p: pen_doc::geom::Point| format!("{} {}", n((p.x - view.x) * sx), n((p.y - view.y) * sy));
        let mut data = String::new();
        for segment in &segments {
            let _ = match *segment {
                pen_doc::path::Segment::MoveTo(p) => write!(data, "M{}", at(p)),
                pen_doc::path::Segment::LineTo(p) => write!(data, "L{}", at(p)),
                pen_doc::path::Segment::QuadTo(c, p) => write!(data, "Q{} {}", at(c), at(p)),
                pen_doc::path::Segment::CubicTo(c1, c2, p) => write!(data, "C{} {} {}", at(c1), at(c2), at(p)),
                pen_doc::path::Segment::Close => write!(data, "Z"),
            };
        }
        let rule = if node.fill_rule.as_deref() == Some("evenodd") { " fill-rule=\"evenodd\"" } else { "" };
        Some(format!("<path d=\"{data}\"{rule}"))
    }

    /// The outline of `scene`'s own shape as an SVG element without its closing `/>`, or `None` for a layer
    /// that has none (a group, an instance's root).
    fn svg_shape(&self, scene: &SceneNode) -> Option<String> {
        let (node, theme) = (&scene.node, &scene.theme);
        let (w, h) = (scene.local.w, scene.local.h);
        Some(match node.kind {
            Kind::Frame | Kind::Rectangle | Kind::Browser | Kind::Note => {
                let radii = if node.kind == Kind::Note { [NOTE_RADIUS; 4] } else { self.corner_radii(node, theme, w, h) };
                if radii.iter().all(|r| (r - radii[0]).abs() < 1e-3) {
                    format!("<rect width=\"{}\" height=\"{}\" rx=\"{}\"", n(w), n(h), n(radii[0]))
                } else {
                    let [tl, tr, br, bl] = radii;
                    format!(
                        "<path d=\"M{} 0 H{} A{tr} {tr} 0 0 1 {} {} V{} A{br} {br} 0 0 1 {} {} H{} A{bl} {bl} 0 0 1 0 {} V{} A{tl} {tl} 0 0 1 {} 0 Z\"",
                        n(tl),
                        n(w - tr),
                        n(w),
                        n(tr),
                        n(h - br),
                        n(w - br),
                        n(h),
                        n(bl),
                        n(h - bl),
                        n(tl),
                        n(tl),
                        tl = n(tl),
                        tr = n(tr),
                        br = n(br),
                        bl = n(bl)
                    )
                }
            }
            Kind::Ellipse => format!("<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"", n(w / 2.0), n(h / 2.0), n(w / 2.0), n(h / 2.0)),
            Kind::Polygon => {
                let count = self.num(node.polygon_count.as_ref(), theme).unwrap_or(3.0).clamp(3.0, 64.0);
                let radius = match &node.corner_radius {
                    Some(pen_doc::model::Corners::All(r)) => self.num(Some(r), theme),
                    Some(pen_doc::model::Corners::Each(each)) => self.num(each.first(), theme),
                    None => None,
                };
                let points: Vec<String> =
                    polygon_points(w, h, count, radius.unwrap_or(0.0).max(0.0)).iter().map(|p| format!("{},{}", n(p[0]), n(p[1]))).collect();
                format!("<polygon points=\"{}\"", points.join(" "))
            }
            Kind::Path | Kind::Connector => self.svg_path(node, node.geometry.as_deref()?, w, h)?,
            Kind::Line => format!("<line x1=\"0\" y1=\"0\" x2=\"{}\" y2=\"{}\"", n(w), n(h)),
            _ => return None,
        })
    }

    /// A filter for the layer's drop shadows and layer blur, as a ` filter="…"` attribute; empty without any.
    fn svg_filter(&self, scene: &SceneNode, sheet: &mut Sheet) -> String {
        let theme = &scene.theme;
        let number = |v: Option<&Value>| match v {
            Some(Value::Number(x)) => x.as_f64().map_or(0.0, narrow),
            Some(Value::String(s)) => self.vars.number(&Num::Text(s.clone()), theme).unwrap_or(0.0),
            _ => 0.0,
        };
        let mut steps = String::new();
        for effect in effects::list(scene.node.effect.as_ref()) {
            if effects::is_off(&effect) {
                continue;
            }
            match EffectKind::of(&effect) {
                Some(EffectKind::DropShadow) => {
                    let c = effect.get("color").and_then(Value::as_str).and_then(|c| self.vars.color(c, theme)).unwrap_or(Rgba::BLACK);
                    let offset = effect.get("offset");
                    let _ = write!(
                        steps,
                        "<feDropShadow dx=\"{}\" dy=\"{}\" stdDeviation=\"{}\" flood-color=\"#{:02x}{:02x}{:02x}\" flood-opacity=\"{}\"/>",
                        n(number(offset.and_then(|o| o.get("x")))),
                        n(number(offset.and_then(|o| o.get("y")))),
                        n(number(effect.get("blur")) / 2.0),
                        c.r,
                        c.g,
                        c.b,
                        n(f32::from(c.a) / 255.0)
                    );
                }
                Some(EffectKind::LayerBlur) => {
                    let _ = write!(steps, "<feGaussianBlur stdDeviation=\"{}\"/>", n(number(effect.get("radius")) / 2.0));
                }
                _ => {}
            }
        }
        if steps.is_empty() {
            return String::new();
        }
        let id = sheet.id("f");
        let _ = writeln!(sheet.defs, "<filter id=\"{id}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\">{steps}</filter>");
        format!(" filter=\"url(#{id})\"")
    }

    /// Writes what an SVG can show of `scene`'s material, if it has one.
    // Human: SVG can't show what is behind a layer through it. A material is written as what is left of it:
    // its tint, a little more solid to stand in for the frost, and the light along its edge.
    fn svg_material(&self, scene: &SceneNode, shape: &str, transform: &str, sheet: &mut Sheet) {
        let Some(pane) = scene.node.material.as_ref().and_then(|_| self.pane_of(scene, false, None)) else { return };
        let params = self.material_params(scene, &pane, false);
        let solid = params.tint_opacity + (1.0 - params.tint_opacity) * FROST_STAND_IN;
        let _ = writeln!(sheet.body, "{shape}{} transform=\"{transform}\"/>", color("fill", params.tint.with_opacity(solid)));
        let id = sheet.id("g");
        let stop = |at: f32, strength: f32| {
            format!("<stop offset=\"{}\" stop-color=\"#ffffff\" stop-opacity=\"{}\"/>", n(at), n(params.highlight * strength))
        };
        let _ = writeln!(
            sheet.defs,
            "<linearGradient id=\"{id}\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\">{}{}{}</linearGradient>",
            stop(0.0, 1.0),
            stop(0.5, 0.16),
            stop(1.0, 0.5)
        );
        let _ = writeln!(sheet.body, "{shape} fill=\"none\" stroke=\"url(#{id})\" stroke-width=\"1\" transform=\"{transform}\"/>");
    }

    /// Writes `scene` and what is in it.
    fn svg_node(&self, scene: &SceneNode, sheet: &mut Sheet) {
        let (node, theme) = (&scene.node, &scene.theme);
        if node.enabled.as_ref().and_then(|e| self.vars.flag(e, theme)) == Some(false) {
            return;
        }
        let (w, h) = (scene.local.w, scene.local.h);
        let m = scene.to_doc.m;
        let transform = format!("matrix({} {} {} {} {} {})", n(m[0]), n(m[1]), n(m[2]), n(m[3]), n(m[4]), n(m[5]));
        let opacity = self.num(node.opacity.as_ref(), theme).unwrap_or(1.0).clamp(0.0, 1.0);
        let filter = self.svg_filter(scene, sheet);
        let fade = if opacity < 1.0 { format!(" opacity=\"{}\"", n(opacity)) } else { String::new() };
        let _ = writeln!(sheet.body, "<g{fade}{filter}>");
        let fills = if node.kind == Kind::Note {
            let c = node.color.as_deref().and_then(|c| self.vars.color(c, theme)).unwrap_or(NOTE_COLOR);
            vec![Brush::Solid(Rgba { a: 255, ..c })]
        } else {
            node.fill.as_ref().map(|f| self.brushes(f.as_slice(), theme)).unwrap_or_default()
        };
        let strokes = node.stroke.as_ref().map(|f| self.brushes(f.as_slice(), theme)).unwrap_or_default();
        let shape = self.svg_shape(scene);
        if let Some(shape) = &shape {
            self.svg_material(scene, shape, &transform, sheet);
        }
        if let Some(shape) = &shape {
            if !matches!(node.kind, Kind::Line | Kind::Connector) {
                for brush in &fills {
                    let paint = Self::svg_paint("fill", brush, sheet);
                    let _ = writeln!(sheet.body, "{shape}{paint} transform=\"{transform}\"/>");
                }
                // Human: a picture fill is the picture itself, covering the box and cut to the shape.
                for url in node.fill.iter().flat_map(pen_doc::model::Paints::as_slice).filter_map(|paint| match paint {
                    Paint::Object(object) if object.get("type").and_then(Value::as_str) == Some("image") => object.get("url").and_then(Value::as_str),
                    _ => None,
                }) {
                    let Some(path) = crate::images::resolve(self.folder, url) else { continue };
                    let clip = sheet.id("c");
                    let _ = writeln!(sheet.defs, "<clipPath id=\"{clip}\">{shape}/></clipPath>");
                    let _ = writeln!(
                        sheet.body,
                        "<g transform=\"{transform}\" clip-path=\"url(#{clip})\"><image href=\"file://{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"xMidYMid slice\"/></g>",
                        escape(&path.display().to_string()),
                        n(w),
                        n(h)
                    );
                }
            }
            let width = self.stroke_width(scene);
            if width > 0.0 {
                // Human: a connector's line ends and turns round unless it says otherwise, and the filled shapes
                // at its ends are a path of their own in the line's colour.
                let ends = if node.kind == Kind::Connector {
                    let said = |key: &str| node.extra.get(key).and_then(Value::as_str).unwrap_or("round").to_owned();
                    format!(" stroke-linecap=\"{}\" stroke-linejoin=\"{}\"", escape(&said("strokeLinecap")), escape(&said("strokeLinejoin")))
                } else {
                    String::new()
                };
                let markers = node.extra.get(pen_doc::connector::MARKERS).and_then(Value::as_str).and_then(|data| self.svg_path(node, data, w, h));
                for brush in &strokes {
                    let paint = Self::svg_paint("stroke", brush, sheet);
                    let _ = writeln!(sheet.body, "{shape} fill=\"none\"{paint} stroke-width=\"{}\"{ends} transform=\"{transform}\"/>", n(width));
                    if let Some(markers) = &markers {
                        let paint = Self::svg_paint("fill", brush, sheet);
                        let _ = writeln!(sheet.body, "{markers}{paint} transform=\"{transform}\"/>");
                    }
                }
            }
        }
        match node.kind {
            Kind::Text | Kind::Note => self.svg_text(scene, &fills, &transform, sheet),
            Kind::Icon | Kind::IconFont => {
                let ink = fills.iter().find_map(|b| if let Brush::Solid(c) = b { Some(*c) } else { None }).unwrap_or(Rgba::BLACK);
                let old = |key: &str| node.extra.get(key).and_then(Value::as_str);
                let library = node.library.as_deref().or_else(|| old("iconFontFamily")).unwrap_or("lucide");
                if let Some(segments) = node.icon.as_deref().or_else(|| old("iconFontName")).and_then(|name| icons::outline(library, name)) {
                    let _ = writeln!(
                        sheet.body,
                        "<path d=\"{}\" fill=\"none\"{} stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" transform=\"{transform} scale({} {})\"/>",
                        icons::path_data(&segments),
                        color("stroke", ink),
                        n(icons::LINE_WIDTH),
                        n(w / icons::VIEW_BOX),
                        n(h / icons::VIEW_BOX)
                    );
                }
            }
            _ => {}
        }
        // Children, cut to the frame when it clips.
        let clips = node.clip.as_ref().and_then(|c| self.vars.flag(c, theme)).unwrap_or(false);
        let cut = match (&shape, clips && !scene.children.is_empty()) {
            (Some(shape), true) => {
                let clip = sheet.id("c");
                let _ = writeln!(sheet.defs, "<clipPath id=\"{clip}\">{shape} transform=\"{transform}\"/></clipPath>");
                let _ = writeln!(sheet.body, "<g clip-path=\"url(#{clip})\">");
                true
            }
            _ => false,
        };
        for child in &scene.children {
            self.svg_node(child, sheet);
        }
        if cut {
            sheet.body.push_str("</g>\n");
        }
        sheet.body.push_str("</g>\n");
    }

    /// Writes a text's (or a sticky note's) lines.
    // Human: SVG has no text boxes: each line the text has is written at its place, by the font's size and line
    // height. A text that the app wraps to its width is written unwrapped, since where it wraps depends on
    // the font the viewer has.
    fn svg_text(&self, scene: &SceneNode, fills: &[Brush], transform: &str, sheet: &mut Sheet) {
        let (node, theme) = (&scene.node, &scene.theme);
        let style = layout::text_style(node, self.vars, theme);
        if style.content.is_empty() {
            return;
        }
        let sticky = node.kind == Kind::Note;
        let ink = if sticky {
            Rgba { r: 0x1c, g: 0x1c, b: 0x1c, a: 255 }
        } else {
            fills.iter().find_map(|b| if let Brush::Solid(c) = b { Some(*c) } else { None }).unwrap_or(Rgba::BLACK)
        };
        let pad = if sticky { NOTE_PADDING } else { 0.0 };
        let width = scene.local.w - 2.0 * pad;
        let (x, anchor) = match style.align {
            TextAlign::Left | TextAlign::Justify => (pad, "start"),
            TextAlign::Center => (pad + width / 2.0, "middle"),
            TextAlign::Right => (pad + width, "end"),
        };
        let line = style.line_px.unwrap_or(style.size * 1.2);
        let mut attributes = format!(
            " font-family=\"'{}', sans-serif\" font-size=\"{}\" font-weight=\"{}\" text-anchor=\"{anchor}\"",
            escape(&style.family),
            n(style.size),
            style.weight
        );
        if style.italic {
            attributes.push_str(" font-style=\"italic\"");
        }
        if style.letter_spacing.abs() > f32::EPSILON {
            let _ = write!(attributes, " letter-spacing=\"{}\"", n(style.letter_spacing));
        }
        if style.underline || style.strikethrough {
            let _ = write!(attributes, " text-decoration=\"{}\"", if style.underline { "underline" } else { "line-through" });
        }
        let _ = writeln!(sheet.body, "<text{attributes}{} transform=\"{transform}\">", color("fill", ink));
        let mut baseline = pad + (line - style.size) / 2.0 + style.size * 0.8;
        for text in style.content.lines() {
            let _ = writeln!(sheet.body, "<tspan x=\"{}\" y=\"{}\">{}</tspan>", n(x), n(baseline), escape(text));
            baseline += line;
        }
        sheet.body.push_str("</text>\n");
    }
}
