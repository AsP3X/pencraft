//! Exact geometry for a model that draws: path data built by calculation, and drawn layers measured.
//!
//! A model asked for twelve rays round a centre, a wreath or a mirrored half works the sines out in its head
//! and gets some of them wrong, and it reads gaps and weights off a picture, where a few pixels are noise.
//! `build_path` does the arithmetic (shapes, rotation, mirroring, repetition) and answers with path data;
//! `measure_layers` reads the outlines as they are drawn and says how large, how far apart and how
//! symmetric they are.

use pen_doc::geom::Point;
use pen_doc::model::Kind;
use pen_doc::path::Segment;
use pen_doc::scene::SceneNode;
use pen_doc::value::{Flag, narrow};
use serde_json::{Value, json};

/// A point of a path under construction.
type P = (f64, f64);

/// One drawing command of a path under construction.
#[derive(Clone, Copy)]
enum Seg {
    Move(P),
    Line(P),
    Cubic(P, P, P),
    Close,
}

/// How many parts one call builds, copies one part is repeated into, and commands the answer holds.
const MOST_PARTS: usize = 64;
const MOST_COPIES: u32 = 360;
const MOST_COMMANDS: usize = 20_000;

/// A map of the plane: x' = a·x + c·y + e, y' = b·x + d·y + f.
#[derive(Clone, Copy)]
struct Map([f64; 6]);

impl Map {
    const SAME: Map = Map([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn apply(self, (x, y): P) -> P {
        let m = self.0;
        (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
    }

    /// This map, then `next`.
    fn then(self, next: Map) -> Map {
        let (m, n) = (self.0, next.0);
        Map([
            n[0] * m[0] + n[2] * m[1],
            n[1] * m[0] + n[3] * m[1],
            n[0] * m[2] + n[2] * m[3],
            n[1] * m[2] + n[3] * m[3],
            n[0] * m[4] + n[2] * m[5] + n[4],
            n[1] * m[4] + n[3] * m[5] + n[5],
        ])
    }

    /// A turn by `degrees` clockwise on the screen (y runs down) about `about`.
    fn turn(degrees: f64, (cx, cy): P) -> Map {
        let (sin, cos) = degrees.to_radians().sin_cos();
        Map([cos, sin, -sin, cos, cx - cos * cx + sin * cy, cy - sin * cx - cos * cy])
    }

    fn shift((dx, dy): P) -> Map {
        Map([1.0, 0.0, 0.0, 1.0, dx, dy])
    }

    fn scale((sx, sy): P, (cx, cy): P) -> Map {
        Map([sx, 0.0, 0.0, sy, cx - sx * cx, cy - sy * cy])
    }

    /// A mirror image about the upright line x = `at` (`upright`) or the level line y = `at`.
    fn mirror(upright: bool, at: f64) -> Map {
        if upright { Map([-1.0, 0.0, 0.0, 1.0, 2.0 * at, 0.0]) } else { Map([1.0, 0.0, 0.0, -1.0, 0.0, 2.0 * at]) }
    }
}

fn mapped(segs: &[Seg], map: Map) -> Vec<Seg> {
    segs.iter()
        .map(|seg| match *seg {
            Seg::Move(p) => Seg::Move(map.apply(p)),
            Seg::Line(p) => Seg::Line(map.apply(p)),
            Seg::Cubic(a, b, p) => Seg::Cubic(map.apply(a), map.apply(b), map.apply(p)),
            Seg::Close => Seg::Close,
        })
        .collect()
}

/// The number `key` of `part`, or why it is missing.
fn number(part: &Value, key: &str) -> Result<f64, String> {
    part.get(key).and_then(Value::as_f64).filter(|n| n.is_finite()).ok_or_else(|| format!("`{key}` is missing or not a number"))
}

fn number_or(part: &Value, key: &str, default: f64) -> Result<f64, String> {
    if part.get(key).is_none_or(Value::is_null) { Ok(default) } else { number(part, key) }
}

/// The point `[x, y]` under `key` of `part`.
fn point(part: &Value, key: &str) -> Result<P, String> {
    let pair = part.get(key).and_then(Value::as_array).filter(|pair| pair.len() == 2);
    let read = |at: usize| pair.and_then(|pair| pair[at].as_f64()).filter(|n| n.is_finite());
    read(0).zip(read(1)).ok_or_else(|| format!("`{key}` must be a point: [x, y]"))
}

fn point_or(part: &Value, key: &str, default: P) -> Result<P, String> {
    if part.get(key).is_none_or(Value::is_null) { Ok(default) } else { point(part, key) }
}

/// The point at `angle` degrees on the circle of the radius `r` about `c`: 0 is to the right, and the angle
/// grows clockwise on the screen.
fn on_circle((cx, cy): P, r: f64, angle: f64) -> P {
    let (sin, cos) = angle.to_radians().sin_cos();
    (cx + r * cos, cy + r * sin)
}

/// An arc about `c` from the angle `from` to `to` (degrees, clockwise when `to` is larger), as cubic curves
/// that start where the pen is: pieces of a quarter turn at most.
fn arc(c: P, r: f64, from: f64, to: f64, out: &mut Vec<Seg>) {
    let pieces = ((to - from).abs() / 90.0).ceil().max(1.0);
    let step = (to - from) / pieces;
    // The handle of a piece that turns by `step`: 4/3 · tan(step / 4) radii long, along the tangent.
    let handle = 4.0 / 3.0 * (step.to_radians() / 4.0).tan() * r;
    let mut at = from;
    while (at - to).abs() > 1e-9 {
        let next = if (at + step - to).abs() < 1e-9 { to } else { at + step };
        let (start, end) = (on_circle(c, r, at), on_circle(c, r, next));
        let (s0, c0) = at.to_radians().sin_cos();
        let (s1, c1) = next.to_radians().sin_cos();
        out.push(Seg::Cubic((start.0 - handle * s0, start.1 + handle * c0), (end.0 + handle * s1, end.1 - handle * c1), end));
        at = next;
    }
}

/// A whole circle, drawn clockwise or the other way round (a hole in a clockwise shape).
fn circle(c: P, r: f64, clockwise: bool, out: &mut Vec<Seg>) {
    out.push(Seg::Move(on_circle(c, r, -90.0)));
    arc(c, r, -90.0, if clockwise { 270.0 } else { -450.0 }, out);
    out.push(Seg::Close);
}

fn closed(points: &[P], out: &mut Vec<Seg>) {
    for (at, p) in points.iter().enumerate() {
        out.push(if at == 0 { Seg::Move(*p) } else { Seg::Line(*p) });
    }
    out.push(Seg::Close);
}

/// A count under `key`: a whole number from `least` to `most`.
fn count(part: &Value, key: &str, least: u32, most: u32) -> Result<u32, String> {
    part.get(key)
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| (least..=most).contains(n))
        .ok_or_else(|| format!("`{key}` must be a whole number from {least} to {most}"))
}

/// The corners of a star or a regular polygon about `c`: `n` points on the radius `r`, with a point on
/// `inner` between each two when there is one. The first stands at `turn` degrees, the top by default.
fn corners(c: P, r: f64, inner: Option<f64>, n: u32, turn: f64) -> Vec<P> {
    let step = 360.0 / f64::from(n);
    (0..n)
        .flat_map(|i| {
            let at = turn + step * f64::from(i);
            std::iter::once(on_circle(c, r, at)).chain(inner.map(|inner| on_circle(c, inner, at + step / 2.0)))
        })
        .collect()
}

/// A rectangle with corners rounded by `radius`.
fn rect(part: &Value, out: &mut Vec<Seg>) -> Result<(), String> {
    let (left, top) = (number(part, "x")?, number(part, "y")?);
    let (right, bottom) = (left + number(part, "width")?, top + number(part, "height")?);
    let radius = number_or(part, "radius", 0.0)?.clamp(0.0, (right - left).min(bottom - top) / 2.0);
    if radius <= 0.0 {
        closed(&[(left, top), (right, top), (right, bottom), (left, bottom)], out);
        return Ok(());
    }
    out.push(Seg::Move((left + radius, top)));
    let centres =
        [(right - radius, top + radius), (right - radius, bottom - radius), (left + radius, bottom - radius), (left + radius, top + radius)];
    for (corner, from) in centres.into_iter().zip([-90.0, 0.0, 90.0, 180.0]) {
        out.push(Seg::Line(on_circle(corner, radius, from)));
        arc(corner, radius, from, from + 90.0, out);
    }
    out.push(Seg::Close);
    Ok(())
}

/// A leaf: two equal arcs from `from` to `to` that meet in points, `width` apart at the middle.
fn leaf(part: &Value, out: &mut Vec<Seg>) -> Result<(), String> {
    let (a, b, width) = (point(part, "from")?, point(part, "to")?, number(part, "width")?);
    let chord = (b.0 - a.0).hypot(b.1 - a.1);
    let sag = width / 2.0;
    if chord <= 0.0 || sag <= 0.0 || sag > chord / 2.0 {
        return Err("a leaf needs two different points and a `width` between 0 and the distance between them".to_owned());
    }
    let r = (chord * chord / 4.0 + sag * sag) / (2.0 * sag);
    let (ux, uy) = ((b.0 - a.0) / chord, (b.1 - a.1) / chord);
    let middle = (f64::midpoint(a.0, b.0), f64::midpoint(a.1, b.1));
    out.push(Seg::Move(a));
    for side in [1.0, -1.0] {
        // Each arc's centre lies on the far side of the chord from its bulge.
        let c = (middle.0 - side * uy * (r - sag), middle.1 + side * ux * (r - sag));
        let angle = |p: P| (p.1 - c.1).atan2(p.0 - c.0).to_degrees();
        let (start, end) = if side > 0.0 { (a, b) } else { (b, a) };
        let (from, mut to) = (angle(start), angle(end));
        while to - from > 180.0 {
            to -= 360.0;
        }
        while to - from < -180.0 {
            to += 360.0;
        }
        arc(c, r, from, to, out);
    }
    out.push(Seg::Close);
    Ok(())
}

/// The commands of one part's `shape`, before it is moved or repeated.
fn shape(part: &Value, kind: &str) -> Result<Vec<Seg>, String> {
    let mut out = Vec::new();
    let centre = || Ok::<P, String>((number(part, "cx")?, number(part, "cy")?));
    match kind {
        "circle" => circle(centre()?, number(part, "r")?, true, &mut out),
        "ring" => {
            let (c, r, inner) = (centre()?, number(part, "r")?, number(part, "inner")?);
            circle(c, r, true, &mut out);
            circle(c, inner, false, &mut out);
        }
        "arc" => {
            let (c, r, from, to) = (centre()?, number(part, "r")?, number(part, "from")?, number(part, "to")?);
            out.push(Seg::Move(on_circle(c, r, from)));
            arc(c, r, from, to, &mut out);
            // With an inner radius it is a band to fill; without one a line to stroke.
            if let Some(inner) = part.get("inner").and_then(Value::as_f64) {
                out.push(Seg::Line(on_circle(c, inner, to)));
                arc(c, inner, to, from, &mut out);
                out.push(Seg::Close);
            }
        }
        "polygon" => {
            closed(&corners(centre()?, number(part, "r")?, None, count(part, "sides", 3, 360)?, number_or(part, "rotation", -90.0)?), &mut out);
        }
        "star" => {
            let r = number(part, "r")?;
            let inner = number_or(part, "inner", r * 0.382)?;
            closed(&corners(centre()?, r, Some(inner), count(part, "points", 3, 360)?, number_or(part, "rotation", -90.0)?), &mut out);
        }
        "rect" => rect(part, &mut out)?,
        "wedge" => {
            // One ray: between two radii, `spread` degrees wide at the outer end and `innerSpread` at the inner.
            let (c, from, to, angle) = (centre()?, number(part, "from")?, number(part, "to")?, number(part, "angle")?);
            let outer = number(part, "spread")? / 2.0;
            let inner = number_or(part, "innerSpread", outer * 2.0)? / 2.0;
            closed(
                &[
                    on_circle(c, from, angle - inner),
                    on_circle(c, to, angle - outer),
                    on_circle(c, to, angle + outer),
                    on_circle(c, from, angle + inner),
                ],
                &mut out,
            );
        }
        "leaf" => leaf(part, &mut out)?,
        "points" => {
            let points = part
                .get("points")
                .and_then(Value::as_array)
                .filter(|points| points.len() >= 2)
                .ok_or("`points` must be a list of at least two [x, y]")?;
            for (at, p) in points.iter().enumerate() {
                let p = point(&json!({ "p": p }), "p").map_err(|_| "each of `points` must be [x, y]".to_owned())?;
                out.push(if at == 0 { Seg::Move(p) } else { Seg::Line(p) });
            }
            if part.get("closed").and_then(Value::as_bool) != Some(false) {
                out.push(Seg::Close);
            }
        }
        other => return Err(format!("unknown shape \"{other}\": circle, ring, arc, polygon, star, rect, wedge, leaf or points")),
    }
    Ok(out)
}

/// Path data as commands; every curve becomes a cubic one.
fn read(data: &str) -> Result<Vec<Seg>, String> {
    let wide = |p: Point| (f64::from(p.x), f64::from(p.y));
    let mut at = (0.0, 0.0);
    let mut out = Vec::new();
    for segment in pen_doc::path::parse(data).map_err(|error| error.to_string())? {
        match segment {
            Segment::MoveTo(p) => out.push(Seg::Move(wide(p))),
            Segment::LineTo(p) => out.push(Seg::Line(wide(p))),
            Segment::QuadTo(c, p) => {
                let (c, p) = (wide(c), wide(p));
                let third = |from: P| (from.0 + 2.0 / 3.0 * (c.0 - from.0), from.1 + 2.0 / 3.0 * (c.1 - from.1));
                out.push(Seg::Cubic(third(at), third(p), p));
            }
            Segment::CubicTo(a, b, p) => out.push(Seg::Cubic(wide(a), wide(b), wide(p))),
            Segment::Close => out.push(Seg::Close),
        }
        if let Some(Seg::Move(p) | Seg::Line(p) | Seg::Cubic(_, _, p)) = out.last() {
            at = *p;
        }
    }
    Ok(out)
}

/// Whether `value` names the upright axis (a mirror image left to right).
fn upright(value: Option<&Value>) -> Result<bool, String> {
    match value.and_then(Value::as_str) {
        Some("vertical") => Ok(true),
        Some("horizontal") => Ok(false),
        _ => Err("a mirror's axis is \"vertical\" (left becomes right, about x = `at`) or \"horizontal\" (about y = `at`)".to_owned()),
    }
}

/// One step of a part's `transform`.
fn step(step: &Value) -> Result<Map, String> {
    if let Some(degrees) = step.get("rotate").and_then(Value::as_f64) {
        Ok(Map::turn(degrees, point_or(step, "about", (0.0, 0.0))?))
    } else if step.get("move").is_some() {
        Ok(Map::shift(point(step, "move")?))
    } else if let Some(scale) = step.get("scale") {
        let by = match scale.as_f64() {
            Some(both) => (both, both),
            None => point(step, "scale")?,
        };
        Ok(Map::scale(by, point_or(step, "about", (0.0, 0.0))?))
    } else if step.get("mirror").is_some() {
        Ok(Map::mirror(upright(step.get("mirror"))?, number(step, "at")?))
    } else {
        Err("a transform step is {\"rotate\": degrees, \"about\": [x, y]}, {\"move\": [dx, dy]}, {\"scale\": n or [sx, sy], \"about\": [x, y]} or {\"mirror\": \"vertical\" | \"horizontal\", \"at\": n}".to_owned())
    }
}

/// Draws a letter's outline into commands, each letter moved to where it stands on the line.
struct LetterPen<'a> {
    out: &'a mut Vec<Seg>,
    /// Where the letter's origin lies: the pen's place on the baseline.
    origin: P,
}

impl LetterPen<'_> {
    /// A font's y runs up, the canvas's down.
    fn at(&self, x: f32, y: f32) -> P {
        (self.origin.0 + f64::from(x), self.origin.1 - f64::from(y))
    }
}

impl skrifa::outline::OutlinePen for LetterPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.out.push(Seg::Move(self.at(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.out.push(Seg::Line(self.at(x, y)));
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let from = match self.out.last() {
            Some(Seg::Move(p) | Seg::Line(p) | Seg::Cubic(_, _, p)) => *p,
            _ => self.origin,
        };
        let (control, to) = (self.at(cx, cy), self.at(x, y));
        let third = |end: P| (end.0 + 2.0 / 3.0 * (control.0 - end.0), end.1 + 2.0 / 3.0 * (control.1 - end.1));
        self.out.push(Seg::Cubic(third(from), third(to), to));
    }

    fn curve_to(&mut self, ax: f32, ay: f32, bx: f32, by: f32, x: f32, y: f32) {
        self.out.push(Seg::Cubic(self.at(ax, ay), self.at(bx, by), self.at(x, y)));
    }

    fn close(&mut self) {
        self.out.push(Seg::Close);
    }
}

/// How long a text one part turns into outlines may be.
const MOST_LETTERS: usize = 200;

/// One line of `text` as the outlines of its letters, shaped as Quill draws it (kerning and ligatures of
/// the face), its baseline starting at `x`, `y`.
// Agent: READS the font file of the installed face.
fn letters(part: &Value, text: &str) -> Result<Vec<Seg>, String> {
    use skrifa::MetadataProvider as _;
    if text.is_empty() || text.chars().count() > MOST_LETTERS || text.contains('\n') {
        return Err(format!("`text` is one line of 1 to {MOST_LETTERS} characters; give each line a part of its own"));
    }
    let family = part.get("fontFamily").and_then(Value::as_str).ok_or("a text needs `fontFamily`, an installed family")?;
    let size = number(part, "fontSize")?;
    let weight = match part.get("fontWeight") {
        None | Some(Value::Null) => 400.0,
        Some(Value::String(weight)) => weight.parse::<f64>().map_err(|_| "`fontWeight` is a number from 100 to 900".to_owned())?,
        Some(_) => number(part, "fontWeight")?,
    };
    let italic = part.get("fontStyle").and_then(Value::as_str) == Some("italic");
    let spacing = number_or(part, "letterSpacing", 0.0)?;
    let (x, y) = (number_or(part, "x", 0.0)?, number_or(part, "y", 0.0)?);
    // A weight is a whole number from 1 to 1000 once clamped, so it is read back as one.
    let asked = format!("{:.0}", weight.clamp(1.0, 1000.0)).parse::<u16>().unwrap_or(400);
    let face = crate::faces::pick(family, asked, italic)
        .filter(|_| crate::faces::installed(family))
        .ok_or_else(|| format!("the family \"{family}\" is not installed: take one from the guide's list of fonts"))?;
    let bytes = std::fs::read(&face.path).map_err(|error| format!("the font file of \"{family}\" couldn't be read: {error}"))?;
    let font = skrifa::FontRef::from_index(&bytes, face.index).map_err(|error| format!("the font file of \"{family}\" couldn't be read: {error}"))?;
    // A variable font is set to the weight asked for, within what it can take.
    let set = face.weights.map(|(lightest, heaviest)| narrow(weight).clamp(lightest, heaviest));
    let location = font.axes().location(set.map(|weight| ("wght", weight)));
    let instance =
        set.map(|value| harfrust::ShaperInstance::from_variations(&font, [harfrust::Variation { tag: harfrust::Tag::new(b"wght"), value }]));
    let data = harfrust::ShaperData::new(&font);
    let shaper = data.shaper(&font).instance(instance.as_ref()).build();
    let mut buffer = harfrust::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(harfrust::Direction::LeftToRight);
    buffer.guess_segment_properties();
    let glyphs = shaper.shape(buffer, harfrust::ShapeOptions::new());
    let per_em = f64::from(font.metrics(skrifa::instance::Size::unscaled(), skrifa::instance::LocationRef::default()).units_per_em);
    if per_em <= 0.0 {
        return Err(format!("the font file of \"{family}\" has no size of its own"));
    }
    let scale = size / per_em;
    let outlines = font.outline_glyphs();
    let settings = || skrifa::outline::DrawSettings::unhinted(skrifa::instance::Size::new(narrow(size)), &location);
    let (mut out, mut pen, mut missing) = (Vec::new(), x, 0_u32);
    for (info, place) in glyphs.glyph_infos().iter().zip(glyphs.glyph_positions()) {
        let origin = (pen + f64::from(place.x_offset) * scale, y - f64::from(place.y_offset) * scale);
        // Glyph 0 is the face's "no such letter" box.
        match outlines.get(skrifa::GlyphId::new(info.glyph_id)).filter(|_| info.glyph_id != 0) {
            Some(outline) => {
                outline
                    .draw(settings(), &mut LetterPen { out: &mut out, origin })
                    .map_err(|error| format!("a letter of \"{family}\" couldn't be drawn: {error}"))?;
            }
            None => missing += 1,
        }
        pen += f64::from(place.x_advance) * scale + spacing;
    }
    if missing > 0 {
        return Err(format!("\"{family}\" has no letter for {missing} of the characters in \"{text}\": take another family or other characters"));
    }
    Ok(out)
}

/// One part's commands: its shape, path or text, moved, then mirrored, then repeated.
fn part(part: &Value) -> Result<Vec<Seg>, String> {
    let mut segs =
        match (part.get("shape").and_then(Value::as_str), part.get("path").and_then(Value::as_str), part.get("text").and_then(Value::as_str)) {
            (Some(kind), None, None) => shape(part, kind)?,
            (None, Some(data), None) => read(data)?,
            (None, None, Some(text)) => letters(part, text)?,
            _ => return Err("a part needs a `shape`, a `path` or a `text`, one of the three".to_owned()),
        };
    let mut map = Map::SAME;
    for one in part.get("transform").and_then(Value::as_array).into_iter().flatten() {
        map = map.then(step(one)?);
    }
    segs = mapped(&segs, map);
    if let Some(mirror) = part.get("mirror") {
        let copy = mapped(&segs, Map::mirror(upright(mirror.get("axis"))?, number(mirror, "at")?));
        segs.extend(copy);
    }
    if let Some(repeat) = part.get("repeat") {
        let copies = count(repeat, "count", 1, MOST_COPIES)?;
        let one = segs.clone();
        for copy in 1..copies {
            let nth = f64::from(copy);
            let map = if repeat.get("about").is_some() {
                Map::turn(number_or(repeat, "angle", 360.0 / f64::from(copies))? * nth, point(repeat, "about")?)
            } else {
                let (dx, dy) = point(repeat, "move")
                    .map_err(|_| "`repeat` needs `about`: [x, y] (copies turned about a point) or `move`: [dx, dy]".to_owned())?;
                Map::shift((dx * nth, dy * nth))
            };
            segs.extend(mapped(&one, map));
        }
    }
    Ok(segs)
}

/// Commands as the path a set of closed shapes is combined from: a shape left open is closed.
fn outline_of(segs: &[Seg]) -> kurbo::BezPath {
    let mut path = kurbo::BezPath::new();
    let mut open = false;
    for seg in segs {
        match *seg {
            Seg::Move(p) => {
                if open {
                    path.close_path();
                }
                path.move_to(p);
                open = true;
            }
            Seg::Line(p) => path.line_to(p),
            Seg::Cubic(a, b, p) => path.curve_to(a, b, p),
            Seg::Close => {
                path.close_path();
                open = false;
            }
        }
    }
    if open {
        path.close_path();
    }
    path
}

/// What is `drawn` so far and the part `with` as one outline: their union, `drawn` with the part taken
/// out, what both cover, or what only one covers. `evenodd` says how both are filled.
fn combined(drawn: &[Seg], with: &[Seg], how: &str, evenodd: bool) -> Result<Vec<Seg>, String> {
    let op = match how {
        "union" => linesweeper::BinaryOp::Union,
        "subtract" => linesweeper::BinaryOp::Difference,
        "intersect" => linesweeper::BinaryOp::Intersection,
        "exclude" => linesweeper::BinaryOp::Xor,
        other => return Err(format!("unknown `combine` \"{other}\": union, subtract, intersect or exclude")),
    };
    if drawn.is_empty() {
        return Err("`combine` works on the parts before it: the first part has nothing to be combined with".to_owned());
    }
    let rule = if evenodd { linesweeper::FillRule::EvenOdd } else { linesweeper::FillRule::NonZero };
    let (a, b) = (outline_of(drawn), outline_of(with));
    // Human: the library checks its own work with assertions; a shape it cannot handle must not end the app.
    let contours = std::panic::catch_unwind(|| linesweeper::binary_op(&a, &b, rule, op))
        .map_err(|_| "these shapes couldn't be combined: move one a little, so no two edges lie exactly on each other".to_owned())?
        .map_err(|error| format!("these shapes couldn't be combined: {error:?}"))?;
    let mut out = Vec::new();
    for contour in contours.contours() {
        let mut at = (0.0, 0.0);
        for element in contour.path.elements() {
            let point = |p: kurbo::Point| (p.x, p.y);
            // Human: where two outlines met, the library leaves pieces without any length; they are noise in
            // path data a person or a model reads.
            let here = |p: kurbo::Point| (p.x - at.0).abs() < 0.005 && (p.y - at.1).abs() < 0.005;
            match *element {
                kurbo::PathEl::LineTo(p) if here(p) => {}
                kurbo::PathEl::CurveTo(a, b, p) if here(a) && here(b) && here(p) => {}
                kurbo::PathEl::MoveTo(p) => out.push(Seg::Move(point(p))),
                kurbo::PathEl::LineTo(p) => out.push(Seg::Line(point(p))),
                kurbo::PathEl::QuadTo(c, p) => {
                    let (c, p) = (point(c), point(p));
                    let third = |end: P| (end.0 + 2.0 / 3.0 * (c.0 - end.0), end.1 + 2.0 / 3.0 * (c.1 - end.1));
                    out.push(Seg::Cubic(third(at), third(p), p));
                }
                kurbo::PathEl::CurveTo(a, b, p) => out.push(Seg::Cubic(point(a), point(b), point(p))),
                kurbo::PathEl::ClosePath => out.push(Seg::Close),
            }
            if let Some(Seg::Move(p) | Seg::Line(p) | Seg::Cubic(_, _, p)) = out.last() {
                at = *p;
            }
        }
    }
    Ok(out)
}

/// A coordinate as it is written: two decimals at most, no trailing zeros.
fn written(n: f64) -> String {
    let rounded = (n * 100.0).round() / 100.0;
    // Minus zero is written as zero.
    let text = format!("{:.2}", if rounded == 0.0 { 0.0 } else { rounded });
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// `build_path`: the path data of `args.parts`, with the box it fills.
///
/// # Errors
/// What is wrong with a part, said so that the model can put it right.
pub fn build(args: &Value) -> Result<Value, String> {
    let parts = args.get("parts").and_then(Value::as_array).filter(|parts| !parts.is_empty()).ok_or("`parts` must be a list of at least one part")?;
    if parts.len() > MOST_PARTS {
        return Err(format!("at most {MOST_PARTS} parts in one call"));
    }
    let evenodd = args.get("fillRule").and_then(Value::as_str) == Some("evenodd");
    let mut segs = Vec::new();
    for (at, one) in parts.iter().enumerate() {
        let made = part(one).map_err(|error| format!("part {at}: {error}"))?;
        match one.get("combine").and_then(Value::as_str) {
            None => segs.extend(made),
            Some(how) => segs = combined(&segs, &made, how, evenodd).map_err(|error| format!("part {at}: {error}"))?,
        }
        if segs.len() > MOST_COMMANDS {
            return Err(format!("the path would have more than {MOST_COMMANDS} commands: build it in several layers"));
        }
    }
    let mut data = String::new();
    let mut subpaths = 0_u32;
    for seg in &segs {
        let (letter, points): (&str, Vec<P>) = match *seg {
            Seg::Move(p) => ("M", vec![p]),
            Seg::Line(p) => ("L", vec![p]),
            Seg::Cubic(a, b, p) => ("C", vec![a, b, p]),
            Seg::Close => ("Z", Vec::new()),
        };
        subpaths += u32::from(letter == "M");
        if !data.is_empty() {
            data.push(' ');
        }
        data.push_str(letter);
        for (x, y) in points {
            data.push_str(&written(x));
            data.push(' ');
            data.push_str(&written(y));
            data.push(' ');
        }
        if data.ends_with(' ') {
            data.pop();
        }
    }
    // The box of the curves themselves, not of their handles.
    let bounds = pen_doc::path::parse(&data).ok().and_then(|segments| pen_doc::path::bounds(&segments)).unwrap_or(pen_doc::geom::Rect::ZERO);
    Ok(json!({
        "geometry": data,
        "bounds": [rounded(bounds.x), rounded(bounds.y), rounded(bounds.w), rounded(bounds.h)],
        "subpaths": subpaths,
    }))
}

/// One outline as drawn, in the document's coordinates.
struct Outline {
    /// The layer and which of its shapes this is, as the answer names it.
    name: String,
    points: Vec<Point>,
}

/// How many outlines are compared with each other, and how many gaps are told.
const MOST_OUTLINES: usize = 60;
const MOST_GAPS: usize = 12;
/// Into how many pieces a curve is cut to be measured.
const CURVE_STEPS: u8 = 24;

fn rounded(n: f32) -> f64 {
    (f64::from(n) * 100.0).round() / 100.0
}

/// The outlines of `scene` in its own coordinates: a path's shapes, or the layer's box or ellipse.
fn local_outlines(scene: &SceneNode) -> Vec<Vec<Point>> {
    let (w, h) = (scene.local.w, scene.local.h);
    match scene.node.kind {
        Kind::Path => {
            let segments = scene.node.geometry.as_deref().and_then(|data| pen_doc::path::parse(data).ok()).unwrap_or_default();
            let view = scene.node.view_box.map(|v| (narrow(v[0]), narrow(v[1]), narrow(v[2]), narrow(v[3])));
            let (vx, vy, vw, vh) = view.or_else(|| pen_doc::path::bounds(&segments).map(|b| (b.x, b.y, b.w, b.h))).unwrap_or((0.0, 0.0, w, h));
            let (sx, sy) = (if vw > 1e-6 { w / vw } else { 1.0 }, if vh > 1e-6 { h / vh } else { 1.0 });
            pen_doc::path::flatten(&segments, CURVE_STEPS)
                .into_iter()
                .map(|run| run.into_iter().map(|p| Point::new((p.x - vx) * sx, (p.y - vy) * sy)).collect())
                .collect()
        }
        Kind::Ellipse => {
            let around = (0..=64_u8).map(|i| {
                let (sin, cos) = (f32::from(i) / 64.0 * std::f32::consts::TAU).sin_cos();
                Point::new(w / 2.0 * (1.0 + cos), h / 2.0 * (1.0 + sin))
            });
            vec![around.collect()]
        }
        Kind::Rectangle | Kind::Polygon | Kind::Line => {
            vec![vec![Point::new(0.0, 0.0), Point::new(w, 0.0), Point::new(w, h), Point::new(0.0, h), Point::new(0.0, 0.0)]]
        }
        _ => Vec::new(),
    }
}

/// The box round `points`: left, top, right, bottom.
fn around(points: &[Point]) -> [f32; 4] {
    points.iter().fold([f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY], |b, p| {
        [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)]
    })
}

/// What `scene` and the layers in it are, with their outlines gathered into `outlines`.
fn gather(scene: &SceneNode, outlines: &mut Vec<Outline>, layers: &mut Vec<Value>) {
    if matches!(&scene.node.enabled, Some(Flag::Value(false))) {
        return;
    }
    let runs: Vec<Vec<Point>> = local_outlines(scene)
        .into_iter()
        .map(|run| run.into_iter().map(|p| scene.to_doc.apply(p)).collect::<Vec<_>>())
        .filter(|run| run.len() > 1)
        .collect();
    if !runs.is_empty() {
        let name = scene.node.display_name();
        let all: Vec<Point> = runs.iter().flatten().copied().collect();
        let [left, top, right, bottom] = around(&all);
        let shapes: Vec<Value> = runs
            .iter()
            .map(|run| {
                let [l, t, r, b] = around(run);
                // The shoelace sum: twice the area the outline encloses.
                let twice: f32 = run.windows(2).map(|pair| pair[0].x * pair[1].y - pair[1].x * pair[0].y).sum();
                json!({ "box": [rounded(l), rounded(t), rounded(r - l), rounded(b - t)], "area": rounded(twice.abs() / 2.0) })
            })
            .collect();
        layers.push(json!({
            "id": scene.id,
            "name": name,
            "box": [rounded(left), rounded(top), rounded(right - left), rounded(bottom - top)],
            "shapes": shapes,
        }));
        let many = runs.len() > 1;
        for (at, points) in runs.into_iter().enumerate() {
            let name = if many { format!("{name} (shape {})", at + 1) } else { name.clone() };
            outlines.push(Outline { name, points });
        }
    }
    for child in &scene.children {
        gather(child, outlines, layers);
    }
}

/// How far `p` is from the line between `a` and `b`.
fn off_line(p: Point, a: Point, b: Point) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx * dx + dy * dy;
    let along = if length > 0.0 { (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0.0, 1.0) } else { 0.0 };
    (p.x - a.x - along * dx).hypot(p.y - a.y - along * dy)
}

/// How far `p` is from the outline `points`.
fn off_outline(p: Point, points: &[Point]) -> f32 {
    points.windows(2).map(|pair| off_line(p, pair[0], pair[1])).fold(f32::INFINITY, f32::min)
}

/// The narrowest place between two outlines: how wide, and where on the first.
fn gap(a: &[Point], b: &[Point]) -> (f32, Point) {
    let near = |from: &[Point], to: &[Point]| {
        from.iter().map(|p| (off_outline(*p, to), *p)).fold((f32::INFINITY, Point::default()), |best, one| if one.0 < best.0 { one } else { best })
    };
    let (there, back) = (near(a, b), near(b, a));
    if there.0 <= back.0 { there } else { back }
}

/// `measure_layers`: what `layers` measure as drawn: each layer's box and shapes, the narrowest gaps between
/// outlines, and how far the whole is from a mirror image of itself.
#[must_use]
pub fn measure(layers: &[&SceneNode]) -> Value {
    let (mut outlines, mut told) = (Vec::new(), Vec::new());
    for layer in layers {
        gather(layer, &mut outlines, &mut told);
    }
    if outlines.is_empty() {
        return json!({ "layers": [], "note": "No shapes in these layers: paths, rectangles, ellipses and polygons are measured; a text's letters are told by `inkBox` in read_layers." });
    }
    let left_out = outlines.len().saturating_sub(MOST_OUTLINES);
    outlines.truncate(MOST_OUTLINES);
    let all: Vec<Point> = outlines.iter().flat_map(|outline| outline.points.iter().copied()).collect();
    let [left, top, right, bottom] = around(&all);
    let mut gaps: Vec<(f32, Point, &str, &str)> = Vec::new();
    for (at, a) in outlines.iter().enumerate() {
        for b in &outlines[at + 1..] {
            let (wide, at) = gap(&a.points, &b.points);
            gaps.push((wide, at, &a.name, &b.name));
        }
    }
    gaps.sort_by(|a, b| a.0.total_cmp(&b.0));
    let gaps: Vec<Value> = gaps
        .iter()
        .take(MOST_GAPS)
        .map(|(wide, at, a, b)| json!({ "between": [a, b], "gap": rounded(*wide), "at": [rounded(at.x), rounded(at.y)] }))
        .collect();
    // How far each point's mirror image lies from the nearest outline, about the upright middle line.
    let middle = f32::midpoint(left, right);
    let (off, at) = all
        .iter()
        .map(|p| (outlines.iter().map(|outline| off_outline(Point::new(2.0 * middle - p.x, p.y), &outline.points)).fold(f32::INFINITY, f32::min), *p))
        .fold((0.0_f32, Point::default()), |worst, one| if one.0 > worst.0 { one } else { worst });
    let mut answer = json!({
        "box": [rounded(left), rounded(top), rounded(right - left), rounded(bottom - top)],
        "layers": told,
        "narrowestGaps": gaps,
        "mirror": { "about": format!("x = {}", rounded(middle)), "off": rounded(off), "worstAt": [rounded(at.x), rounded(at.y)] },
        "note": "Canvas units. A gap of 0 means the two outlines touch or cross; a gap between a shape and a hole in it is the wall between them. `mirror.off` is how far the farthest point's mirror image lies from any outline: about 0 for a symmetric mark (curves are measured in short straight pieces, so up to 0.3 is rounding), and large for one that is not meant to be symmetric.",
    });
    if left_out > 0 {
        answer["leftOut"] = json!(format!("{left_out} more outlines were not compared: measure fewer layers at once."));
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;

    fn geometry(args: &Value) -> String {
        build(args).unwrap()["geometry"].as_str().unwrap().to_owned()
    }

    #[test]
    fn shapes_are_calculated_turned_mirrored_and_repeated() {
        // A triangle's corners on a circle, the first at the top.
        assert_eq!(
            geometry(&json!({ "parts": [{ "shape": "polygon", "cx": 120, "cy": 120, "r": 100, "sides": 3 }] })),
            "M120 20 L206.6 170 L33.4 170 Z"
        );
        // Three copies of a bar turned about the centre: the worked numbers of the skill.
        let turned = geometry(&json!({ "parts": [{ "path": "M120 100 L120 20", "repeat": { "count": 3, "about": [120, 120] } }] }));
        assert_eq!(turned, "M120 100 L120 20 M137.32 130 L206.6 170 M102.68 130 L33.4 170");
        // A mirror image is added beside the part, about x = 120.
        let both = geometry(
            &json!({ "parts": [{ "shape": "points", "points": [[20, 20], [60, 20], [40, 80]], "mirror": { "axis": "vertical", "at": 120 } }] }),
        );
        assert_eq!(both, "M20 20 L60 20 L40 80 Z M220 20 L180 20 L200 80 Z");
        // Steps of a transform come in their order: moved, then doubled about the origin.
        let moved = geometry(&json!({ "parts": [{ "path": "M0 0 H10", "transform": [{ "move": [5, 0] }, { "scale": 2 }] }] }));
        assert_eq!(moved, "M10 0 L30 0");
        // A quarter circle is one curve with handles 0.5523 radii long, and a ring's hole runs the other way.
        let quarter = geometry(&json!({ "parts": [{ "shape": "arc", "cx": 0, "cy": 0, "r": 100, "from": 0, "to": 90 }] }));
        assert_eq!(quarter, "M100 0 C100 55.23 55.23 100 0 100");
        let ring = build(&json!({ "parts": [{ "shape": "ring", "cx": 120, "cy": 120, "r": 100, "inner": 60 }] })).unwrap();
        assert_eq!((ring["subpaths"].as_u64(), &ring["bounds"]), (Some(2), &json!([20.0, 20.0, 200.0, 200.0])));
        // Twelve rays, a star and a leaf come out closed, and a wrong part says which and why.
        let rays = build(
            &json!({ "parts": [{ "shape": "wedge", "cx": 240, "cy": 240, "from": 100, "to": 200, "angle": -90, "spread": 6, "repeat": { "count": 12, "about": [240, 240] } }] }),
        );
        assert_eq!(rays.unwrap()["subpaths"], 12);
        assert!(
            geometry(&json!({ "parts": [{ "shape": "star", "cx": 0, "cy": 0, "r": 100, "points": 5 }] }))
                .starts_with("M0 -100 L22.45 -30.9 L95.11 -30.9")
        );
        let leaf = build(&json!({ "parts": [{ "shape": "leaf", "from": [0, 0], "to": [100, 0], "width": 40 }] })).unwrap();
        // It is as wide at its middle as asked, to the hundredth the curves are written in.
        let (top, tall) = (leaf["bounds"][1].as_f64().unwrap(), leaf["bounds"][3].as_f64().unwrap());
        assert!((top + 20.0).abs() < 0.02 && (tall - 40.0).abs() < 0.02 && leaf["bounds"][2] == 100.0, "{leaf}");
        let wrong = build(&json!({ "parts": [{ "shape": "circle", "cx": 0, "cy": 0, "r": 5 }, { "shape": "blob" }] })).unwrap_err();
        assert!(wrong.starts_with("part 1: unknown shape \"blob\""), "{wrong}");
    }

    #[test]
    fn shapes_are_combined_into_one_outline_and_letters_become_outlines() {
        let square = |x: f64, combine: Option<&str>| {
            let mut part = json!({ "shape": "rect", "x": x, "y": 0, "width": 100, "height": 100 });
            if let Some(how) = combine {
                part["combine"] = json!(how);
            }
            part
        };
        let built = |how: &str| build(&json!({ "parts": [square(0.0, None), square(50.0, Some(how))] })).unwrap();
        // Two squares that overlap by half: one outline of 150 by 100, the left half, the middle, or both ends.
        let (union, left, middle, ends) = (built("union"), built("subtract"), built("intersect"), built("exclude"));
        assert_eq!((union["subpaths"].as_u64(), &union["bounds"]), (Some(1), &json!([0.0, 0.0, 150.0, 100.0])), "{union}");
        assert_eq!((left["subpaths"].as_u64(), &left["bounds"]), (Some(1), &json!([0.0, 0.0, 50.0, 100.0])), "{left}");
        assert_eq!((middle["subpaths"].as_u64(), &middle["bounds"]), (Some(1), &json!([50.0, 0.0, 50.0, 100.0])), "{middle}");
        assert_eq!((ends["subpaths"].as_u64(), &ends["bounds"]), (Some(2), &json!([0.0, 0.0, 150.0, 100.0])), "{ends}");
        // A disc taken out of a disc keeps its curves: a ring of two outlines, not a polygon of hundreds of points.
        let ring = build(&json!({ "parts": [
            { "shape": "circle", "cx": 120, "cy": 120, "r": 100 },
            { "shape": "circle", "cx": 120, "cy": 120, "r": 60, "combine": "subtract" }
        ] }))
        .unwrap();
        let data = ring["geometry"].as_str().unwrap();
        assert_eq!(ring["subpaths"], 2, "{ring}");
        assert!(data.contains('C') && data.len() < 1200, "{data}");
        // No piece of the answer is without length.
        let joined = build(&json!({ "parts": [
            { "path": "M316 0 H356 A60 60 0 0 1 356 120 H316 Z" },
            { "shape": "rect", "x": 348, "y": 26, "width": 30, "height": 68, "combine": "subtract" }
        ] }))
        .unwrap();
        let numbers: Vec<&str> =
            joined["geometry"].as_str().unwrap().split(['M', 'L', 'C', 'Z']).map(str::trim).filter(|piece| !piece.is_empty()).collect();
        assert!(
            numbers.windows(2).all(|pair| !pair[1].split(' ').collect::<Vec<_>>().chunks(2).all(|xy| pair[0].ends_with(&xy.join(" ")))),
            "{joined}"
        );
        // Nothing to combine with, and a word that is no way of combining.
        assert!(build(&json!({ "parts": [square(0.0, Some("union"))] })).unwrap_err().contains("the first part has nothing"));
        assert!(build(&json!({ "parts": [square(0.0, None), square(5.0, Some("glue"))] })).unwrap_err().contains("unknown `combine` \"glue\""));
        // A family that isn't installed is said so; an installed one gives closed outlines on the baseline.
        let text = |family: &str| {
            build(&json!({ "parts": [{ "text": "NV", "fontFamily": family, "fontSize": 100, "fontWeight": "700", "x": 10, "y": 100 }] }))
        };
        assert!(text("No Such Family 123").unwrap_err().contains("is not installed"));
        if let Some(family) = ["Helvetica", "Arial", "DejaVu Sans"].into_iter().find(|family| crate::faces::installed(family)) {
            let set = text(family).unwrap();
            let bounds = &set["bounds"];
            // N is one outline and V another; they stand on the baseline at y 100, right of x 10.
            assert_eq!(set["subpaths"], 2, "{set}");
            assert!(bounds[0].as_f64().unwrap() >= 10.0 && (bounds[1].as_f64().unwrap() + bounds[3].as_f64().unwrap() - 100.0).abs() < 2.0, "{set}");
        }
    }

    #[test]
    fn drawn_layers_are_measured() {
        let mut editor = Editor::new(
            pen_doc::read(
                r##"{"version":"2.22","children":[{"type":"group","id":"logo","x":100,"y":0,"children":[
                {"type":"path","id":"bars","name":"Bars","x":0,"y":0,"width":240,"height":240,"viewBox":[0,0,240,240],"fill":"#111111",
                 "geometry":"M20 20 H80 V220 H20 Z M160 20 H220 V220 H160 Z"},
                {"type":"ellipse","id":"dot","name":"Dot","x":100,"y":100,"width":40,"height":40,"fill":"#111111"}]}]}"##,
            )
            .unwrap(),
            None,
            1,
        );
        editor.refresh(&|style| (style.size * 5.0, style.size * 1.2));
        let layers: Vec<&SceneNode> = editor.scene.iter().collect();
        let measured = measure(&layers);
        assert_eq!(measured["box"], json!([120.0, 20.0, 200.0, 200.0]));
        assert_eq!(measured["layers"][0]["shapes"][0], json!({ "box": [120.0, 20.0, 60.0, 200.0], "area": 12000.0 }));
        // The dot stands 20 from each bar, and the two bars 80 apart.
        let gaps = measured["narrowestGaps"].as_array().unwrap();
        assert_eq!((gaps[0]["gap"].as_f64(), gaps[1]["gap"].as_f64(), gaps[2]["gap"].as_f64()), (Some(20.0), Some(20.0), Some(80.0)));
        assert_eq!(gaps[2]["between"], json!(["Bars (shape 1)", "Bars (shape 2)"]));
        // The whole is its own mirror image about its middle.
        assert_eq!(measured["mirror"]["about"], "x = 220");
        assert!(measured["mirror"]["off"].as_f64().unwrap() < 0.3, "{measured}");
    }
}
