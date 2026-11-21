//! The welcome film's five beats, each drawn from the seconds played alone. The stage is 1280 by 800 points;
//! every place and moment below is the browser film's (`design/onboarding/splash.js`).
//!
//! - "Describe it.": the message box grows, a request is typed and sent.
//! - "Watch it drawn.": the assistant draws a bookshop's page on a board and ticks off its task list.
//! - "Make it yours.": a person restyles the page: font, fill, position, theme.
//! - The page joins a wall of other designs, which folds into a point.
//! - The mark draws itself, the name closes up, and two buttons come.

use egui::{Color32, Stroke};

use super::draw::{Face, Pen, both, curve, glide, held, into, mix, out, out_hard, past, through};
use super::{BOARD, COVER, HERO, Pictures, WALL};
use crate::theme::{
    ACCENT, AGENT, SPLASH_COVERS, SPLASH_FILLS, SPLASH_HEADING, SPLASH_INK, SPLASH_INK_EDGE, SPLASH_INK_ISLAND, SPLASH_INK_MUTED, SPLASH_NAME,
    SPLASH_PAGE_DARK, SPLASH_PAGE_MUTED, SPLASH_PAPER, SPLASH_PAPER_EDGE, SPLASH_PAPER_FAINT, SPLASH_PAPER_MUTED, SPLASH_READY, SPLASH_TITLE,
};

/// When "Watch it drawn." begins, in seconds.
pub const WATCH: f32 = 1.75;
/// When "Make it yours." begins.
pub const YOURS: f32 = 4.2;
/// When the page leaves for the wall.
const LEAVES: f32 = 6.3;
/// When the name's beat begins.
pub const NAME: f32 = 7.5;
/// When each of the page's five parts is drawn.
const BUILT: [f32; 5] = [2.05, 2.35, 2.65, 2.95, 3.25];
/// The assistant's task list: one line for each part.
const TASKS: [&str; 5] = ["Navigation", "Headline", "Call to action", "Hero picture", "Book covers"];
/// The tool each part is drawn with, as its place in the toolbar.
const TOOLS: [u8; 5] = [1, 2, 2, 3, 1];
/// Where each part lies on the page (x, y, width, height), in the page's 800 by 500 points.
const BOX: [[f32; 4]; 5] =
    [[16.0, 8.0, 768.0, 40.0], [20.0, 100.0, 400.0, 122.0], [20.0, 228.0, 330.0, 120.0], [440.0, 76.0, 332.0, 264.0], [20.0, 356.0, 760.0, 136.0]];
/// What the user asks for in the first beat.
const REQUEST: &str = "A landing page for a bookshop on Mill Lane";
/// Other things to ask for, shown under the message box.
const IDEAS: [(&str, f32); 3] =
    [("A pricing page with three plans", 0.0), ("A sign-in screen for a banking app", 300.0), ("A dashboard for a bike rental fleet", 628.0)];
/// Where the two buttons at the end are pressed: "Start designing", and "Set up an assistant" under it.
pub const BUTTONS: [[f32; 4]; 2] = [[540.0, 674.0, 200.0, 56.0], [540.0, 740.0, 200.0, 36.0]];
/// The line the mark's quill draws: start, two handles, end, in the icon's 1024 points.
const LINE: [[f32; 2]; 4] = [[182.0, 880.0], [380.0, 968.0], [650.0, 936.0], [856.0, 772.0]];
/// The two edges of the quill's vane, from its base to its tip.
const VANE: [[[f32; 2]; 4]; 2] =
    [[[300.0, 772.0], [318.0, 520.0], [540.0, 250.0], [836.0, 188.0]], [[836.0, 188.0], [836.0, 470.0], [600.0, 716.0], [300.0, 772.0]]];
/// How many straight pieces a curve is drawn in.
const PIECES: u8 = 24;

/// Draws the frame at `time` seconds.
pub fn frame(pen: &Pen, pictures: &Pictures, time: f32) {
    if time < WATCH {
        describe(pen, time);
    } else if time < YOURS {
        watch(pen, pictures, time);
    } else if time < LEAVES {
        yours(pen, pictures, time);
    } else if time < NAME {
        wall(pen, pictures, time);
    } else {
        name(pen, time);
    }
}

/// A line of big type whose words rise out of a mask one after another, and leave upward just before `until`.
fn heading(pen: &Pen, time: f32, [from, until]: [f32; 2], words: &str, color: Color32, size: f32) {
    let (base, spacing) = (64.0 + size * 0.86, -size * 0.04);
    let leave = into(through(time, until - 0.16, 0.16));
    let (mut x, mut delay) = (64.0, 0.0);
    for word in words.split(' ') {
        let rise = out_hard(through(time, from + delay, 0.45));
        let width = pen.width(word, size, Face::Bold, spacing);
        let mask = pen.cut(x - 6.0, base - size * 0.9, width + 16.0, size * 1.14);
        mask.text(word, [x, base + (1.0 - rise) * size * 1.2 - leave * size * 1.2], [size, spacing], Face::Bold, color);
        x += width + size * 0.24;
        delay += 0.07;
    }
}

/// The three marks that say which beat is playing, and how to skip.
fn marks(pen: &Pen, dark: bool, beat: u8) {
    let (lit, unlit, quiet) =
        if dark { (SPLASH_PAPER, SPLASH_INK_EDGE, SPLASH_INK_MUTED) } else { (SPLASH_INK, SPLASH_PAPER_EDGE, SPLASH_PAPER_MUTED) };
    for mark in 0..3u8 {
        pen.fill([64.0 + f32::from(mark) * 50.0, 758.0, 42.0, 4.0], 2.0, if mark < beat { lit } else { unlit });
    }
    pen.text("Esc to skip", [1140.0, 766.0], [15.0, 0.0], Face::Regular, quiet);
}

/// The canvas's dot grid under a beat, and its marks.
fn ground(pen: &Pen, dark: bool, beat: u8) {
    let dot = if dark { Color32::from_white_alpha(18) } else { Color32::from_black_alpha(26) };
    for row in 0..25u8 {
        for column in 0..40u8 {
            pen.fill([24.0 + f32::from(column) * 32.0, 24.0 + f32::from(row) * 32.0, 2.0, 2.0], 0.0, dot);
        }
    }
    marks(pen, dark, beat);
}

/// The floating toolbar, with the tool at `active` lit.
fn tools(pen: &Pen, dark: bool, active: u8, show: f32) {
    if show <= 0.0 {
        return;
    }
    let pen = pen.faded(show);
    let (island, edge, ink) =
        if dark { (SPLASH_INK_ISLAND, SPLASH_INK_EDGE, SPLASH_INK_MUTED) } else { (Color32::WHITE, SPLASH_PAPER_EDGE, SPLASH_PAPER_MUTED) };
    let bar = [64.0, 628.0 + (1.0 - show) * 16.0, 268.0, 60.0];
    pen.fill(bar, 16.0, island);
    pen.outline(bar, 16.0, edge, 1.0);
    for (place, icon) in (0..5u8).zip(["mouse-pointer-2", "frame", "type", "image", "pen-tool"]) {
        let (x, y) = (bar[0] + 38.0 + f32::from(place) * 48.0, bar[1] + 30.0);
        if place == active {
            pen.fill([x - 20.0, y - 20.0, 40.0, 40.0], 10.0, ACCENT);
        }
        pen.icon(icon, x, y, 20.0, if place == active { Color32::WHITE } else { ink });
    }
}

/// A pointer with its tip at (`x`, `y`), and under it the name of whose it is.
fn pointer(pen: &Pen, [x, y]: [f32; 2], fill: Color32, edge: Color32, label: Option<&str>) {
    let arrow = [[0.0, 0.0], [0.0, 25.6], [7.2, 19.2], [18.4, 18.4]].map(|[right, down]| [x + right, y + down]);
    pen.shape(&arrow, fill, Stroke::new(1.5 * pen.view.scale, edge));
    if let Some(label) = label {
        let width = pen.width(label, 13.0, Face::Bold, 0.0) + 16.0;
        pen.fill([x + 16.0, y + 24.0, width, 24.0], 8.0, fill);
        pen.text(label, [x + 24.0, y + 41.0], [13.0, 0.0], Face::Bold, Color32::WHITE);
    }
}

/// The frame the editor draws round what is selected, with its four handles.
fn selection(pen: &Pen, [x, y, w, h]: [f32; 4]) {
    pen.outline([x, y, w, h], 0.0, ACCENT, 1.5);
    for [right, down] in [[0.0, 0.0], [w, 0.0], [0.0, h], [w, h]] {
        let handle = [x + right - 4.0, y + down - 4.0, 8.0, 8.0];
        pen.fill(handle, 0.0, Color32::WHITE);
        pen.outline(handle, 0.0, ACCENT, 1.5);
    }
}

/// How the bookshop's page looks at one moment.
struct Page {
    /// How far each of its five parts is drawn, 0 to 1.
    drawn: [f32; 5],
    /// Seconds since its covers began to come.
    covers: f32,
    /// Its headline is set in the heavy face.
    heavy: bool,
    /// It has its dark theme.
    dark: bool,
    /// The fill behind its picture.
    fill: Color32,
    /// Where its button's left edge is.
    button: f32,
}

impl Page {
    /// The page at `time`.
    fn at(time: f32) -> Page {
        Page {
            drawn: BUILT.map(|built| out(through(time, built, 0.22))),
            covers: time - BUILT[4],
            heavy: time >= 4.62,
            dark: time >= 6.0,
            fill: if time < 5.0 {
                ACCENT
            } else if (5.14..5.28).contains(&time) {
                SPLASH_FILLS[1]
            } else {
                SPLASH_FILLS[0]
            },
            button: mix(28.0, 222.0, both(through(time, 5.5, 0.4))),
        }
    }

    /// Its text, its quiet text and its ground.
    fn colors(&self) -> [Color32; 3] {
        if self.dark { [SPLASH_PAPER, SPLASH_PAGE_MUTED[1], SPLASH_PAGE_DARK] } else { [SPLASH_INK, SPLASH_PAGE_MUTED[0], Color32::WHITE] }
    }

    /// A pen for the part at `index`, which is wiped in from the left while it is being drawn; `None` before.
    fn part(&self, pen: &Pen, index: usize) -> Option<Pen> {
        let drawn = self.drawn.get(index).copied().unwrap_or(1.0);
        let [x, y, w, h] = BOX.get(index).copied().unwrap_or_default();
        match drawn {
            drawn if drawn <= 0.0 => None,
            drawn if drawn < 1.0 => Some(pen.cut(x - 8.0, y - 8.0, (w + 16.0) * drawn, h + 16.0)),
            _ => Some(pen.faded(1.0)),
        }
    }
}

/// How much larger than its own points the page is drawn: it comes slowly nearer all the while.
fn nearness(time: f32) -> f32 {
    1.0 + 0.035 * through(time, 1.8, 4.5)
}

/// The stage's place of the page's (`x`, `y`).
fn on_page(time: f32, x: f32, y: f32) -> [f32; 2] {
    let scale = nearness(time);
    [820.0 + (x - 400.0) * scale, 500.0 + (y - 250.0) * scale]
}

/// A pen for the page's own points, the page `lift` under its place.
fn page_pen(pen: &Pen, time: f32, lift: f32) -> Pen {
    let scale = nearness(time);
    pen.inside(820.0 - 400.0 * scale, 500.0 + lift - 250.0 * scale, scale)
}

/// The page's card, its navigation, its headline, and its two lines with the button.
fn page_words(pen: &Pen, page: &Page) {
    let [ink, quiet, ground] = page.colors();
    let card = [0.0, 0.0, 800.0, 500.0];
    pen.shadow(card, 10.0);
    pen.fill(card, 10.0, ground);
    pen.outline(card, 10.0, if page.dark { SPLASH_INK_EDGE } else { SPLASH_PAPER_EDGE }, 1.0);
    if let Some(pen) = page.part(pen, 0) {
        pen.fill([28.0, 17.0, 18.0, 22.0], 3.0, ink);
        pen.text("Mill Lane Books", [56.0, 34.0], [15.0, 0.0], Face::Bold, ink);
        for (link, x) in [("Shop", 520.0), ("Events", 582.0), ("About", 644.0)] {
            pen.text(link, [x, 34.0], [14.0, 0.0], Face::Regular, quiet);
        }
        pen.outline([712.0, 14.0, 60.0, 28.0], 14.0, ink, 1.2);
        pen.text("Visit", [727.0, 33.0], [13.0, 0.0], Face::Bold, ink);
    }
    if let Some(pen) = page.part(pen, 1) {
        let (face, size) = if page.heavy { (Face::Heavy, [44.0, -1.5]) } else { (Face::Serif, [52.0, -1.0]) };
        pen.text("Books worth", [28.0, 150.0], size, face, ink);
        pen.text("the walk.", [28.0, 208.0], size, face, ink);
    }
    if let Some(pen) = page.part(pen, 2) {
        pen.text("An independent bookshop on Mill Lane,", [28.0, 248.0], [16.0, 0.0], Face::Regular, quiet);
        pen.text("open late on Thursdays.", [28.0, 270.0], [16.0, 0.0], Face::Regular, quiet);
        pen.fill([page.button, 296.0, 150.0, 44.0], 22.0, ink);
        pen.text("Plan a visit", [page.button + 32.0, 324.0], [15.0, 0.0], Face::Bold, ground);
    }
}

/// The page's picture of books on its fill, and its row of four covers.
fn page_pictures(pen: &Pen, pictures: &Pictures, page: &Page) {
    if let Some(pen) = page.part(pen, 3) {
        pen.fill([440.0, 84.0, 332.0, 256.0], 14.0, page.fill);
        pen.picture(pictures.get(HERO), [448.0, 92.0, 316.0, 237.0], 0.0);
    }
    if let Some(pen) = page.part(pen, 4) {
        pen.text("New this week", [28.0, 376.0], [13.0, 0.0], Face::Bold, page.colors()[0]);
        for (cover, under) in (0..4u8).zip(SPLASH_COVERS) {
            let come = out_hard(through(page.covers, f32::from(cover) * 0.07, 0.3));
            if come > 0.0 {
                let tile = [28.0 + f32::from(cover) * 184.0, 388.0 + (1.0 - come) * 16.0, 170.0, 96.0];
                let pen = pen.faded(come);
                pen.fill(tile, 8.0, under);
                pen.picture(pictures.get(COVER + usize::from(cover)), tile, 8.0);
            }
        }
    }
}

/// The first `share` of `text`, by letters.
fn typed(text: &str, share: f32) -> &str {
    let letters = u16::try_from(text.chars().count()).map_or(f32::MAX, f32::from);
    let mut counted = 0.0;
    for (at, _) in text.char_indices() {
        if counted >= (share * letters).floor() {
            return text.get(..at).unwrap_or(text);
        }
        counted += 1.0;
    }
    text
}

/// "Describe it.": the message box grows across the stage, the request is typed, and the send button swells
/// over the whole window, which is the cut into the next beat.
fn describe(pen: &Pen, time: f32) {
    heading(pen, time, [0.0, 1.6], "Describe it.", SPLASH_PAPER, SPLASH_TITLE);
    let (grow, fold) = (out_hard(through(time, 0.12, 0.5)), both(through(time, 1.44, 0.2)));
    let (x, y, width) = (64.0 + 1048.0 * fold, 440.0, mix(104.0, 1152.0, grow) - 1048.0 * fold);
    for (place, (idea, left)) in (0..3u8).zip(IDEAS) {
        let come = out_hard(through(time, 0.4 + f32::from(place) * 0.07, 0.35)) * (1.0 - fold);
        if come > 0.0 {
            let pen = pen.faded(come);
            let pill = [64.0 + left, 574.0 + (1.0 - come) * 14.0, pen.width(idea, 18.0, Face::Regular, 0.0) + 40.0, 44.0];
            pen.outline(pill, 22.0, SPLASH_INK_EDGE, 1.5);
            pen.text(idea, [pill[0] + 20.0, pill[1] + 28.0], [18.0, 0.0], Face::Regular, SPLASH_INK_MUTED);
        }
    }
    if time > 0.1 {
        pen.fill([x, y, width, 104.0], 52.0, SPLASH_INK_ISLAND);
        pen.outline([x, y, width, 104.0], 52.0, SPLASH_INK_EDGE, 1.5);
        let words = pen.cut(x + 36.0, y, width - 130.0, 104.0).faded(1.0 - fold);
        let said = typed(REQUEST, through(time, 0.55, 0.7));
        let end = words.text(said, [x + 44.0, y + 64.0], [36.0, 0.0], Face::Regular, SPLASH_PAPER);
        if time < 1.36 && (time * 4.0).rem_euclid(2.0) < 1.0 {
            words.fill([x + 47.0 + end, y + 32.0, 3.0, 42.0], 0.0, ACCENT);
        }
        let press = 1.0 - 0.16 * (std::f32::consts::PI * through(time, 1.3, 0.16)).sin();
        let radius = 36.0 * (grow * 1.6).min(1.0) * press;
        pen.dot(x + width - 52.0, y + 52.0, radius, ACCENT);
        pen.icon("arrow-up", x + width - 52.0, y + 52.0, radius * 0.95, Color32::WHITE);
    }
    marks(pen, true, 1);
    let iris = into(through(time, 1.56, 0.19));
    if iris > 0.0 {
        pen.dot(1164.0, 492.0, 36.0 + iris * 1300.0, ACCENT);
    }
}

/// The assistant's task list. Returns the tool it has in hand.
fn tasks(pen: &Pen, time: f32) -> u8 {
    let mut tool = 0;
    let mut before = 1.95 - 0.22;
    for (place, ((name, built), with)) in (0..5u8).zip(TASKS.into_iter().zip(BUILT).zip(TOOLS)) {
        if time >= built {
            tool = with;
        }
        let show = out_hard(through(time, 1.9 + f32::from(place) * 0.05, 0.3));
        let (done, active) = (time >= built + 0.22, time < built + 0.22 && time >= before + 0.22);
        before = built;
        if show <= 0.0 {
            continue;
        }
        let pen = pen.faded(show);
        let y = 330.0 + f32::from(place) * 50.0 + (1.0 - show) * 14.0;
        if done {
            pen.dot(76.0, y - 8.0, 11.0 * past(through(time, built + 0.22, 0.25)), ACCENT);
            pen.icon("check", 76.0, y - 8.0, 14.0, Color32::WHITE);
        } else {
            pen.ring(76.0, y - 8.0, 10.0, SPLASH_PAPER_FAINT, 2.0);
        }
        if active {
            let arc: Vec<[f32; 2]> =
                (0..=8u8).map(|step| time * 9.0 + f32::from(step) * 0.24).map(|a| [76.0 + 10.0 * a.cos(), y - 8.0 + 10.0 * a.sin()]).collect();
            pen.line(&arc, ACCENT, 2.6);
        }
        let color = if done {
            SPLASH_PAPER_MUTED
        } else if active {
            SPLASH_INK
        } else {
            SPLASH_PAPER_FAINT
        };
        pen.text(name, [102.0, y], [22.0, 0.0], if done || active { Face::Bold } else { Face::Regular }, color);
    }
    if time >= BUILT[4] + 0.3 { 0 } else { tool }
}

/// "Watch it drawn.": the assistant's pointer draws the page part by part, each one framed as it lands.
fn watch(pen: &Pen, pictures: &Pictures, time: f32) {
    let open = out_hard(through(time, WATCH, 0.3));
    if open < 1.0 {
        pen.fill([-4000.0, -4000.0, 9280.0, 8800.0], 0.0, ACCENT);
        pen.dot(1164.0, 492.0, open * 1400.0, SPLASH_PAPER);
    }
    if open > 0.6 {
        ground(pen, false, 2);
    }
    heading(pen, time, [1.8, YOURS], "Watch it drawn.", SPLASH_INK, SPLASH_HEADING);
    let tool = tasks(pen, time);
    tools(pen, false, tool, out_hard(through(time, 2.0, 0.4)));
    let enter = out_hard(through(time, 1.82, 0.5));
    let page = Page::at(time);
    let board = page_pen(&pen.faded(enter), time, (1.0 - enter) * 110.0);
    page_words(&board, &page);
    page_pictures(&board, pictures, &page);
    let mut stops = vec![[1.85, 1250.0, 790.0]];
    for (built, [x, y, w, h]) in BUILT.into_iter().zip(BOX) {
        if (built..built + 0.3).contains(&time) {
            selection(&board, [x, y, w * out(through(time, built, 0.22)), h]);
        }
        let (from, to) = (on_page(time, x + 6.0, y + 6.0), on_page(time, x + w - 4.0, y + h - 4.0));
        stops.extend([[built, from[0], from[1]], [built + 0.2, to[0], to[1]]]);
    }
    stops.push([3.9, 1150.0, 716.0]);
    let seen = through(time, 1.9, 0.15) * (1.0 - through(time, 4.0, 0.18));
    pointer(&pen.faded(seen), glide(&stops, time), AGENT, Color32::WHITE, Some("Agent"));
}

/// "Make it yours.": a person's pointer restyles the page, and each change is written down beside it.
fn yours(pen: &Pen, pictures: &Pictures, time: f32) {
    ground(pen, true, 3);
    heading(pen, time, [4.22, LEAVES], "Make it yours.", SPLASH_PAPER, SPLASH_HEADING);
    tools(pen, true, 0, 1.0);
    let page = Page::at(time);
    let fill = format!("#{:02X}{:02X}{:02X}", page.fill.r(), page.fill.g(), page.fill.b());
    let place = format!("X {:.0}", page.button);
    let changes = [("Font", "Arial Black", 4.62), ("Fill", fill.as_str(), 5.0), ("Position", place.as_str(), 5.5), ("Theme", "Dark", 6.0)];
    for (row, (label, value, at)) in (0..4u8).zip(changes) {
        let show = out_hard(through(time, at, 0.3));
        if show > 0.0 {
            let pen = pen.faded(show);
            let y = 330.0 + f32::from(row) * 50.0 + (1.0 - show) * 14.0;
            pen.text(label, [64.0, y], [19.0, 0.0], Face::Regular, SPLASH_INK_MUTED);
            let swatch = if label == "Fill" { 34.0 } else { 0.0 };
            if swatch > 0.0 {
                pen.fill([168.0, y - 19.0, 24.0, 24.0], 6.0, page.fill);
            }
            pen.text(value, [168.0 + swatch, y], [24.0, 0.0], Face::Bold, SPLASH_PAPER);
        }
    }
    let board = page_pen(pen, time, 0.0);
    page_words(&board, &page);
    page_pictures(&board, pictures, &page);
    let chosen = [
        (4.46..4.92, BOX[1]),
        (4.94..5.42, [440.0, 84.0, 332.0, 256.0]),
        (5.46..5.98, [page.button, 296.0, 150.0, 44.0]),
        (6.0..LEAVES, [0.0, 0.0, 800.0, 500.0]),
    ];
    for (when, frame) in chosen {
        if when.contains(&time) {
            selection(&board, frame);
        }
    }
    let [headline, picture, button] = [on_page(time, 230.0, 170.0), on_page(time, 610.0, 220.0), on_page(time, 110.0, 322.0)];
    let way = [
        [4.2, 1180.0, 730.0],
        [4.5, headline[0], headline[1]],
        [4.75, headline[0], headline[1]],
        [4.96, picture[0], picture[1]],
        [5.3, picture[0], picture[1]],
        [5.48, button[0], button[1]],
    ];
    let (held_at, away) = (on_page(time, 304.0, 322.0), on_page(time, 700.0, 30.0));
    let tip = if time >= 5.92 {
        glide(&[[5.92, held_at[0], held_at[1]], [6.04, away[0], away[1]]], time)
    } else if time >= 5.48 {
        on_page(time, page.button + 82.0, 322.0)
    } else {
        glide(&way, time)
    };
    pointer(pen, tip, Color32::WHITE, SPLASH_INK, None);
}

/// The page flies into the middle of a tilted wall of other designs, which then folds into a point.
fn wall(pen: &Pen, pictures: &Pictures, time: f32) {
    let leave = 1.0 - through(time, LEAVES, 0.3);
    ground(&pen.faded(leave), true, 3);
    tools(pen, true, 0, leave);
    let (travel, size) = (both(through(time, LEAVES, 0.6)), mix(1.1, 1.0, out(through(time, LEAVES, 1.2))));
    let angle = -0.105 * travel;
    let (sin, cos) = angle.sin_cos();
    let edge = Color32::from_gray(128).gamma_multiply(0.35);
    for row in 0..3u8 {
        for column in 0..5u8 {
            let (across, down) = (f32::from(column) - 2.0, f32::from(row) - 1.0);
            let far = across.abs() + down.abs();
            let (born, gone) = (through(time, 6.5 + far * 0.06, 0.34), 1.0 - into(through(time, 7.12 + (3.0 - far) * 0.03, 0.3)));
            if far < 0.5 || born <= 0.0 || gone <= 0.0 {
                continue;
            }
            let scale = 0.26 * mix(0.8, 1.0, past(born)) * gone * size;
            let (x, y) = (across * 236.0 * gone * size, down * 158.0 * gone * size);
            let place = usize::from(row * 5 + column);
            let picture = pictures.get(WALL + if place < 7 { place } else { place - 1 });
            pen.faded(born * 2.5).turned(
                picture,
                [640.0 + x * cos - y * sin, 400.0 + x * sin + y * cos],
                [800.0 * scale, 500.0 * scale],
                angle,
                edge,
            );
        }
    }
    let scale = mix(nearness(LEAVES), 0.26 * size, travel) * (1.0 - into(through(time, 7.2, 0.28)));
    pen.turned(pictures.get(BOARD), [mix(820.0, 640.0, travel), mix(500.0, 400.0, travel)], [800.0 * scale, 500.0 * scale], angle, edge);
    if time > 7.3 {
        pen.dot(640.0, 400.0, 7.0 * through(time, 7.3, 0.1), ACCENT);
    }
}

/// The mark: the quill's line draws itself with two fainter copies trailing, and the quill swings down onto
/// its nib.
fn mark(pen: &Pen, time: f32) {
    let scale = 330.0 / 1024.0;
    let pen = pen.inside(640.0 - 512.0 * scale, 262.0 - 512.0 * scale, scale);
    let drawn = out_hard(through(time, NAME + 0.02, 0.5));
    for (strength, lag) in [(0.16, 0.16), (0.3, 0.08), (1.0, 0.0)] {
        let reach = out_hard(through(time, NAME + 0.02 + lag, 0.5));
        if reach > 0.0 && (lag <= 0.0 || drawn < 0.995) {
            let line: Vec<[f32; 2]> = (0..=PIECES).map(|piece| curve(LINE, reach * f32::from(piece) / f32::from(PIECES))).collect();
            pen.faded(strength).line(&line, ACCENT, 46.0);
        }
    }
    let swing = through(time, NAME + 0.22, 0.62);
    if swing <= 0.0 {
        return;
    }
    let (sin, cos) = (-0.95 * (1.0 - past(swing))).sin_cos();
    let turn = |[x, y]: [f32; 2]| [182.0 + (x - 182.0) * cos - (y - 880.0) * sin, 880.0 + (x - 182.0) * sin + (y - 880.0) * cos];
    let pen = pen.faded(swing * 5.0);
    let vane: Vec<[f32; 2]> =
        VANE.iter().flat_map(|side| (0..PIECES).map(|piece| turn(curve(*side, f32::from(piece) / f32::from(PIECES))))).collect();
    pen.shape(&vane, SPLASH_INK, Stroke::NONE);
    pen.shape(&[[182.0, 880.0], [288.0, 759.0], [312.0, 785.0]].map(turn), SPLASH_INK, Stroke::NONE);
    for split in [[[676.0, 660.0], [500.0, 604.0]], [[808.0, 484.0], [634.0, 440.0]], [[338.0, 736.0], [716.0, 316.0]]] {
        pen.line(&split.map(turn), SPLASH_PAPER, 18.0);
    }
}

/// The last beat: the mark, the name closing up from wide apart, what the app is for in one line, and the two
/// buttons ([`BUTTONS`]).
fn name(pen: &Pen, time: f32) {
    mark(pen, time);
    let gap = mix(90.0, -5.0, out_hard(through(time, NAME + 0.5, 0.7)));
    let letters = ["Q", "u", "i", "l", "l"];
    let widths = letters.map(|letter| pen.width(letter, SPLASH_NAME, Face::Bold, 0.0));
    let whole = widths.iter().sum::<f32>() + gap * 4.0;
    let mut x = 640.0 - whole / 2.0;
    for (place, (letter, width)) in (0..5u8).zip(letters.into_iter().zip(widths)) {
        let seen = through(time, NAME + 0.5 + f32::from(place) * 0.05, 0.2);
        pen.faded(seen).text(letter, [x, 572.0], [SPLASH_NAME, 0.0], Face::Bold, SPLASH_INK);
        x += width + gap;
    }
    let said = out_hard(through(time, 8.55, 0.45));
    let line = "Describe it. Watch it drawn. Make it yours.";
    pen.cut(0.0, 596.0, 1280.0, 44.0).centred(line, [640.0, 628.0 + (1.0 - said) * 44.0], 25.0, Face::Regular, SPLASH_PAPER_MUTED);
    let ready = out_hard(through(time, SPLASH_READY, 0.4));
    if ready > 0.0 {
        let (pen, lift) = (pen.faded(ready), (1.0 - held(ready)) * 14.0);
        let [x, y, w, h] = BUTTONS[0];
        pen.fill([x, y + lift, w, h], h / 2.0, SPLASH_INK);
        pen.centred("Start designing", [640.0, y + lift + 34.0], 18.0, Face::Bold, SPLASH_PAPER);
        pen.centred("Set up an assistant", [640.0, BUTTONS[1][1] + lift + 23.0], 15.0, Face::Regular, SPLASH_PAPER_MUTED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_is_typed_letter_by_letter() {
        assert_eq!(typed("abcd", 0.0), "");
        assert_eq!(typed("abcd", 0.5), "ab");
        assert_eq!(typed("abcd", 1.0), "abcd");
        assert_eq!(typed("é", 1.0), "é");
    }

    #[test]
    fn the_page_is_empty_before_it_is_drawn_and_whole_after() {
        assert_eq!(Page::at(WATCH).drawn, [0.0; 5]);
        assert_eq!(Page::at(YOURS).drawn, [1.0; 5]);
    }

    #[test]
    fn the_page_is_restyled_in_order_font_fill_position_theme() {
        let before = Page::at(4.5);
        assert!(!before.heavy && before.fill == ACCENT && !before.dark);
        assert!((before.button - 28.0).abs() < 1e-3);
        let after = Page::at(LEAVES);
        assert!(after.heavy && after.fill == SPLASH_FILLS[0] && after.dark);
        assert!((after.button - 222.0).abs() < 1e-3);
        assert_eq!(Page::at(5.2).fill, SPLASH_FILLS[1], "the second fill is tried on the way");
    }

    #[test]
    fn the_page_comes_nearer_and_its_points_follow() {
        assert_eq!(on_page(0.0, 400.0, 250.0), [820.0, 500.0]);
        assert!(nearness(LEAVES) > nearness(WATCH));
    }
}
