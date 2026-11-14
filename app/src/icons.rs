//! Icon outlines for `icon` and `icon_font` nodes.
//!
//! The Lucide set (ISC licence) comes from the `lucide-static-svg` crate as SVG text; its few element kinds are
//! turned into the same drawing commands a `path` node uses. `feather` names are looked up in Lucide, which
//! grew out of Feather and keeps most of its names. Other libraries aren't bundled and return `None`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use lucide_static_svg::Icon;
use pen_doc::geom::Point;
use pen_doc::path::{self as svg, Segment};

use crate::app::lock;
use crate::theme::ICON_LINE;

/// Side of the square every bundled icon is drawn in.
pub const VIEW_BOX: f32 = 24.0;
/// Line width of the bundled icons, in view-box units.
pub const LINE_WIDTH: f32 = 2.0;
/// Names Lucide used in earlier releases (and Feather still uses), with the bundled icon that replaced each.
// Agent: every right-hand name MUST exist in the bundled set; a test checks it.
const RENAMED: [(&str, &str); 20] = [
    ("home", "house"),
    ("trash-2", "trash"),
    ("edit", "square-pen"),
    ("edit-2", "pencil"),
    ("edit-3", "pencil"),
    ("more-horizontal", "ellipsis"),
    ("more-vertical", "ellipsis-vertical"),
    ("filter", "funnel"),
    ("help-circle", "circle-question-mark"),
    ("circle-help", "circle-question-mark"),
    ("check-circle", "circle-check-big"),
    ("check-circle-2", "circle-check"),
    ("x-circle", "circle-x"),
    ("bar-chart", "chart-bar"),
    ("triangle-alert", "triangle-exclamation-point"),
    ("alert-triangle", "triangle-exclamation-point"),
    ("circle-alert", "circle-exclamation-point"),
    ("alert-circle", "circle-exclamation-point"),
    ("octagon-alert", "octagon-exclamation-point"),
    ("alert-octagon", "octagon-exclamation-point"),
];
/// Straight pieces a curve of an interface icon is drawn with.
const CURVE_STEPS: u8 = 10;
/// The lines of one icon in its 24 × 24 box; `true` marks a line that closes back to its start.
type Lines = Arc<Vec<(Vec<[f32; 2]>, bool)>>;
// Human: 0.5523 places cubic control points so a quarter circle is within 0.03 % of round.
const KAPPA: f32 = 0.552_284_8;

/// The outline of `name` in `library`, as open and closed lines in a 24 × 24 box, or `None` when the library
/// isn't bundled or has no icon of that name.
#[must_use]
pub fn outline(library: &str, name: &str) -> Option<Vec<Segment>> {
    static NAMES: OnceLock<HashMap<&'static str, Icon>> = OnceLock::new();
    if !matches!(library.trim().to_ascii_lowercase().as_str(), "lucide" | "feather") {
        return None;
    }
    let names = NAMES.get_or_init(|| Icon::all().iter().map(|icon| (icon.name(), *icon)).collect());
    let name = name.trim().to_ascii_lowercase();
    let renamed = RENAMED.iter().find(|(old, _)| *old == name).map(|(_, new)| *new);
    let icon = names.get(name.as_str()).or_else(|| renamed.and_then(|new| names.get(new)))?;
    let segments = segments(icon.svg_str());
    (!segments.is_empty()).then_some(segments)
}

/// How many single-letter changes turn `a` into `b`.
fn changes(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut before = row[0];
        row[0] = i + 1;
        for (j, y) in b.iter().enumerate() {
            let kept = row[j + 1];
            row[j + 1] = (before + usize::from(x != y)).min(row[j] + 1).min(kept + 1);
            before = kept;
        }
    }
    row[b.len()]
}

/// The bundled icon whose name is nearest to `name`: the one sharing most of its words ("circle-loader" for
/// "loader-circle"), then the one fewest letters away. `None` when nothing bundled is close.
// Human: a model writes icon names from memory, and Lucide renames icons; an answer that names the icon it
// most likely meant saves it a guess.
#[must_use]
pub fn nearest(name: &str) -> Option<&'static str> {
    let name = name.trim().to_ascii_lowercase();
    let words: Vec<&str> = name.split('-').collect();
    let shared = |known: &str| known.split('-').filter(|word| words.contains(word)).count();
    Icon::all()
        .iter()
        .map(|icon| icon.name())
        .min_by_key(|known| (words.len().saturating_sub(shared(known)), changes(&name, known)))
        .filter(|known| shared(known) > 0 || changes(&name, known) <= 2)
}

/// Draws the Lucide icon `name` into `rect` as thin lines, for the interface's own buttons.
// Human: each icon's lines are worked out once and kept; drawing only scales them into place.
// Agent: MAIN-THREAD; LOCKS the icon cache briefly; an unknown name draws nothing.
pub fn paint(painter: &egui::Painter, rect: egui::Rect, name: &'static str, color: egui::Color32) {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, Lines>>> = OnceLock::new();
    let lines = lock(CACHE.get_or_init(Mutex::default))
        .entry(name)
        .or_insert_with(|| Arc::new(polylines(&outline("lucide", name).unwrap_or_default())))
        .clone();
    let scale = rect.width().min(rect.height()) / VIEW_BOX;
    let stroke = egui::Stroke::new(ICON_LINE, color);
    for (points, closed) in lines.iter() {
        let points: Vec<egui::Pos2> = points.iter().map(|[x, y]| rect.min + egui::vec2(x * scale, y * scale)).collect();
        painter.add(if *closed { egui::Shape::closed_line(points, stroke) } else { egui::Shape::line(points, stroke) });
    }
}

/// Drawing commands as runs of straight lines, with curves cut into [`CURVE_STEPS`] pieces.
fn polylines(segments: &[Segment]) -> Vec<(Vec<[f32; 2]>, bool)> {
    let mut out: Vec<(Vec<[f32; 2]>, bool)> = Vec::new();
    let mut at = Point::new(0.0, 0.0);
    let steps = || (1..=CURVE_STEPS).map(|i| f32::from(i) / f32::from(CURVE_STEPS));
    for segment in segments {
        match *segment {
            Segment::MoveTo(p) => {
                out.push((vec![[p.x, p.y]], false));
                at = p;
            }
            Segment::LineTo(p) => {
                if let Some((line, _)) = out.last_mut() {
                    line.push([p.x, p.y]);
                }
                at = p;
            }
            Segment::QuadTo(c, p) => {
                if let Some((line, _)) = out.last_mut() {
                    line.extend(steps().map(|t| {
                        let u = 1.0 - t;
                        [u * u * at.x + 2.0 * u * t * c.x + t * t * p.x, u * u * at.y + 2.0 * u * t * c.y + t * t * p.y]
                    }));
                }
                at = p;
            }
            Segment::CubicTo(c1, c2, p) => {
                if let Some((line, _)) = out.last_mut() {
                    line.extend(steps().map(|t| {
                        let rest = 1.0 - t;
                        let weights = [rest * rest * rest, 3.0 * rest * rest * t, 3.0 * rest * t * t, t * t * t];
                        let mix = |v: [f32; 4]| v.iter().zip(weights).map(|(v, w)| v * w).sum::<f32>();
                        [mix([at.x, c1.x, c2.x, p.x]), mix([at.y, c1.y, c2.y, p.y])]
                    }));
                }
                at = p;
            }
            Segment::Close => {
                if let Some((line, closed)) = out.last_mut() {
                    *closed = true;
                    if let Some([x, y]) = line.first() {
                        at = Point::new(*x, *y);
                    }
                }
            }
        }
    }
    out
}

/// The drawing commands of every shape element in `text` (an SVG document without groups or transforms).
fn segments(text: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    for element in text.split('<').skip(1) {
        let (tag, rest) = element.split_once(|c: char| c.is_whitespace() || c == '>' || c == '/').unwrap_or((element, ""));
        let number = |name: &str| attribute(rest, name).and_then(|v| v.trim().parse::<f32>().ok());
        let or_zero = |name: &str| number(name).unwrap_or(0.0);
        match tag {
            "path" => out.extend(attribute(rest, "d").and_then(|d| svg::parse(d).ok()).unwrap_or_default()),
            "circle" => {
                if let Some(r) = number("r") {
                    ellipse(or_zero("cx"), or_zero("cy"), r, r, &mut out);
                }
            }
            "ellipse" => {
                if let (Some(rx), Some(ry)) = (number("rx"), number("ry")) {
                    ellipse(or_zero("cx"), or_zero("cy"), rx, ry, &mut out);
                }
            }
            "line" => {
                out.push(Segment::MoveTo(Point::new(or_zero("x1"), or_zero("y1"))));
                out.push(Segment::LineTo(Point::new(or_zero("x2"), or_zero("y2"))));
            }
            "rect" => {
                if let (Some(w), Some(h)) = (number("width"), number("height")) {
                    // Human: SVG lets one corner radius stand in for the other.
                    let rx = number("rx").or_else(|| number("ry")).unwrap_or(0.0);
                    let ry = number("ry").unwrap_or(rx);
                    rect(or_zero("x"), or_zero("y"), w, h, rx.min(w / 2.0), ry.min(h / 2.0), &mut out);
                }
            }
            "polyline" | "polygon" => {
                let values: Vec<f32> =
                    attribute(rest, "points").unwrap_or("").split(|c: char| c.is_whitespace() || c == ',').filter_map(|v| v.parse().ok()).collect();
                for (i, pair) in values.as_chunks::<2>().0.iter().enumerate() {
                    let p = Point::new(pair[0], pair[1]);
                    out.push(if i == 0 { Segment::MoveTo(p) } else { Segment::LineTo(p) });
                }
                if tag == "polygon" && values.len() >= 4 {
                    out.push(Segment::Close);
                }
            }
            _ => {}
        }
    }
    out
}

/// Drawing commands as SVG path data (`M… L… C… Z`).
#[must_use]
pub fn path_data(segments: &[Segment]) -> String {
    use std::fmt::Write as _;
    let mut data = String::new();
    for segment in segments {
        let p = |p: Point| format!("{} {}", p.x, p.y);
        // Writing to a String can't fail.
        let _ = match *segment {
            Segment::MoveTo(to) => write!(data, "M{} ", p(to)),
            Segment::LineTo(to) => write!(data, "L{} ", p(to)),
            Segment::QuadTo(c, to) => write!(data, "Q{} {} ", p(c), p(to)),
            Segment::CubicTo(c1, c2, to) => write!(data, "C{} {} {} ", p(c1), p(c2), p(to)),
            Segment::Close => write!(data, "Z "),
        };
    }
    data.trim_end().to_owned()
}

/// An SVG file as one path layer with id `id`: every shape it has, in the SVG's view box (or the shapes' own
/// bounds), filled or stroked as the `<svg>` element says. `None` when it has no shape this reads.
// Human: this reads what the bundled icons need: paths, circles, ellipses, lines, rects and polygons, without
// groups' transforms, styles or gradients. Colours other than hex ones (`currentColor`, names) become black.
#[must_use]
pub fn svg_layer(text: &str, id: String) -> Option<pen_doc::Node> {
    use pen_doc::model::{Kind, Paint, Paints, StrokeWidth};
    use pen_doc::value::Num;
    let shapes = segments(text);
    if shapes.is_empty() {
        return None;
    }
    let root = text.split("<svg").nth(1).and_then(|rest| rest.split('>').next()).unwrap_or("");
    let numbers: Vec<f32> =
        attribute(root, "viewBox").unwrap_or("").split(|c: char| c.is_whitespace() || c == ',').filter_map(|v| v.parse().ok()).collect();
    let area = match numbers.as_slice() {
        [x, y, w, h] if *w > 0.0 && *h > 0.0 => pen_doc::geom::Rect::new(*x, *y, *w, *h),
        _ => svg::bounds(&shapes)?,
    };
    let color = |name: &str| match attribute(root, name).map(str::trim) {
        Some("none") => None,
        Some(hex) if hex.starts_with('#') => Some(hex.to_owned()),
        Some(_) => Some("#000000".to_owned()),
        None => (name == "fill").then(|| "#000000".to_owned()),
    };
    let mut node = pen_doc::Node::new(Kind::Path, id);
    node.geometry = Some(path_data(&shapes));
    node.view_box = Some([area.x, area.y, area.w, area.h].map(f64::from));
    (node.width, node.height) = (Some(Num::new(area.w)), Some(Num::new(area.h)));
    node.fill = color("fill").map(|c| Paints::One(Paint::Solid(c)));
    node.stroke = color("stroke").map(|c| Paints::One(Paint::Solid(c)));
    if node.stroke.is_some() {
        let width = attribute(root, "stroke-width").and_then(|w| w.trim().parse::<f32>().ok()).unwrap_or(1.0);
        node.stroke_width = Some(StrokeWidth::All(Num::new(width)));
        for (key, name) in [("strokeLinecap", "stroke-linecap"), ("strokeLinejoin", "stroke-linejoin")] {
            if let Some(value) = attribute(root, name) {
                node.extra.insert(key.to_owned(), serde_json::Value::String(value.trim().to_owned()));
            }
        }
    }
    Some(node)
}

/// The value of attribute `name` in an element's text; `name` must be the whole attribute name.
fn attribute<'a>(element: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = element;
    while let Some(at) = rest.find(name) {
        let before_ok = rest[..at].chars().next_back().is_none_or(char::is_whitespace);
        let after = &rest[at + name.len()..];
        if before_ok && let Some(value) = after.strip_prefix("=\"") {
            return value.split_once('"').map(|(v, _)| v);
        }
        rest = after;
    }
    None
}

fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32, out: &mut Vec<Segment>) {
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    let p = Point::new;
    out.push(Segment::MoveTo(p(cx + rx, cy)));
    out.push(Segment::CubicTo(p(cx + rx, cy + ky), p(cx + kx, cy + ry), p(cx, cy + ry)));
    out.push(Segment::CubicTo(p(cx - kx, cy + ry), p(cx - rx, cy + ky), p(cx - rx, cy)));
    out.push(Segment::CubicTo(p(cx - rx, cy - ky), p(cx - kx, cy - ry), p(cx, cy - ry)));
    out.push(Segment::CubicTo(p(cx + kx, cy - ry), p(cx + rx, cy - ky), p(cx + rx, cy)));
    out.push(Segment::Close);
}

#[expect(clippy::many_single_char_names, reason = "x, y, w, h are the box's position and size; p builds a point")]
fn rect(x: f32, y: f32, w: f32, h: f32, rx: f32, ry: f32, out: &mut Vec<Segment>) {
    let p = Point::new;
    let (kx, ky) = (rx * (1.0 - KAPPA), ry * (1.0 - KAPPA));
    let round = rx > 0.0 && ry > 0.0;
    out.push(Segment::MoveTo(p(x + rx, y)));
    let mut side = |end: Point, c1: Point, c2: Point, corner: Point| {
        out.push(Segment::LineTo(end));
        if round {
            out.push(Segment::CubicTo(c1, c2, corner));
        }
    };
    side(p(x + w - rx, y), p(x + w - kx, y), p(x + w, y + ky), p(x + w, y + ry));
    side(p(x + w, y + h - ry), p(x + w, y + h - ky), p(x + w - kx, y + h), p(x + w - rx, y + h));
    side(p(x + rx, y + h), p(x + kx, y + h), p(x, y + h - ky), p(x, y + h - ry));
    side(p(x, y + ry), p(x, y + ky), p(x + kx, y), p(x + rx, y));
    out.push(Segment::Close);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_name_finds_the_icon_most_likely_meant() {
        assert_eq!(nearest("loader-circl"), Some("loader-circle"));
        assert_eq!(nearest("user-circle"), Some("circle-user"));
        assert_eq!(nearest("zzqq"), None);
        // The names the guide gives a model for the parts components are built from.
        for name in [
            "check",
            "x",
            "chevron-down",
            "chevron-right",
            "chevron-up",
            "ellipsis",
            "search",
            "loader-circle",
            "panel-left",
            "circle-check",
            "info",
            "triangle-exclamation-point",
            "octagon-x",
            "plus",
            "house",
            "trash",
        ] {
            assert!(outline("lucide", name).is_some(), "{name}");
            assert!(crate::agent::GUIDE.contains(&format!("`{name}`")), "{name}");
        }
    }

    /// The icons the interface's own buttons use.
    const INTERFACE: [&str; 35] = [
        "paperclip",
        "image",
        "file-text",
        "messages-square",
        "message-circle",
        "layers",
        "presentation",
        "component",
        "library",
        "plus",
        "maximize-2",
        "minimize-2",
        "rocket",
        "chevron-down",
        "arrow-up",
        "panel-left",
        "mouse-pointer-2",
        "square",
        "circle",
        "pen-tool",
        "type",
        "frame",
        "sticky-note",
        "globe",
        "hand",
        "search",
        "ellipsis",
        "house",
        "folder",
        "x",
        "sun",
        "moon",
        "settings",
        "share",
        "play",
    ];

    #[test]
    fn every_bundled_icon_has_an_outline_inside_its_box() {
        for icon in Icon::all() {
            let segments = outline("lucide", icon.name()).unwrap_or_else(|| panic!("{} has no outline", icon.name()));
            let bounds = svg::bounds(&segments).unwrap();
            assert!(bounds.x >= -0.5 && bounds.y >= -0.5 && bounds.right() <= VIEW_BOX + 0.5 && bounds.bottom() <= VIEW_BOX + 0.5, "{}", icon.name());
        }
    }

    #[test]
    fn names_and_libraries_are_matched_loosely_and_unknown_ones_refused() {
        assert!(outline("Lucide", " Chevron-Right ").is_some());
        assert!(outline("feather", "check").is_some());
        assert!(outline("lucide", "no-such-icon").is_none());
        assert!(outline("phosphor", "check").is_none());
    }

    #[test]
    fn earlier_names_lead_to_their_replacements() {
        for (old, new) in RENAMED {
            assert!(outline("lucide", new).is_some(), "{new} is not bundled");
            assert_eq!(outline("lucide", old), outline("lucide", new), "{old}");
        }
    }

    #[test]
    fn every_interface_icon_is_bundled_and_has_lines() {
        for name in INTERFACE {
            let lines = polylines(&outline("lucide", name).unwrap_or_else(|| panic!("{name} is not bundled")));
            assert!(lines.iter().any(|(points, _)| points.len() >= 2), "{name}");
        }
    }

    #[test]
    fn curves_become_straight_pieces_and_closed_lines_are_marked() {
        let lines = polylines(&svg::parse("M0 0 C0 10 10 10 10 0 Z M1 1 L2 2").unwrap());
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0.len(), 1 + usize::from(CURVE_STEPS));
        assert_eq!(lines[0].0.last(), Some(&[10.0, 0.0]));
        assert!(lines[0].1 && !lines[1].1);
    }

    #[test]
    fn shape_elements_become_lines() {
        let text = r#"<svg width="24"><circle cx="12" cy="12" r="10" /><line x1="1" y1="2" x2="3" y2="4" /><rect width="6" height="4" x="2" y="3" rx="1" /><polyline points="1,1 2,2 3,1" /></svg>"#;
        let segments = segments(text);
        assert_eq!(segments.iter().filter(|s| matches!(s, Segment::MoveTo(_))).count(), 4);
        assert_eq!(segments.iter().filter(|s| matches!(s, Segment::Close)).count(), 2);
        assert!(segments.contains(&Segment::LineTo(Point::new(3.0, 4.0))));
        // Human: `rx` must not be read out of `rx`-less attributes such as a circle's `r`, nor `x` out of `rx`.
        assert_eq!(attribute(r#"rx="1" x="2""#, "x"), Some("2"));
        assert_eq!(attribute(r#"cx="5""#, "x"), None);
    }
}
