//! Timings on a large design, run by hand:
//! `cargo test --release -- --ignored timing_ --nocapture --test-threads 1`.
//!
// Human: `render.rs` speaks of a design of 11,000 layers that frame times were measured on; that file is not
// in the repository, so the design here is built to the same size: forty boards of grouped rectangles and
// texts, and one board with close to a thousand layers of its own, which is what Select All takes.
// Agent: every test here is `#[ignore]`d: they print times and assert nothing about them.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use egui::{Pos2, Rect as ScreenRect};

use crate::canvas::Canvas;
use crate::editor::Editor;

/// The boards of grouped layers, the groups on each, and the layers in a group.
const BOARDS: usize = 40;
const GROUPS: usize = 10;
const IN_GROUP: usize = 24;
/// The layers the one wide board holds itself.
const WIDE: usize = 960;
/// The window the frames are drawn in.
const WINDOW: egui::Vec2 = egui::vec2(1600.0, 1000.0);

/// The design as `.pen` text: 11,001 layers.
pub(crate) fn large_design() -> String {
    let mut json = String::from(r#"{"version":"2.22","children":["#);
    for board in 0..BOARDS {
        let (x, y) = ((board % 8) * 1500, (board / 8) * 1100);
        write!(
            json,
            r##"{{"type":"frame","id":"b{board}","x":{x},"y":{y},"width":1400,"height":1000,"layout":"none","fill":"#ffffff","children":["##
        )
        .unwrap();
        for group in 0..GROUPS {
            let (gx, gy) = ((group % 5) * 270 + 20, (group / 5) * 480 + 20);
            write!(json, r#"{{"type":"frame","id":"g{board}_{group}","x":{gx},"y":{gy},"width":250,"height":460,"layout":"none","children":["#)
                .unwrap();
            for layer in 0..IN_GROUP {
                let (lx, ly) = ((layer % 2) * 120 + 5, (layer / 2) * 36 + 5);
                let id = format!("l{board}_{group}_{layer}");
                if layer % 2 == 0 {
                    write!(json, r##"{{"type":"rectangle","id":"{id}","x":{lx},"y":{ly},"width":110,"height":30,"fill":"#3b82f6"}}"##).unwrap();
                } else {
                    write!(json, r##"{{"type":"text","id":"{id}","x":{lx},"y":{ly},"content":"Layer {layer} of {group}","fontSize":16,"fontFamily":"Helvetica","fill":"#111111"}}"##).unwrap();
                }
                json.push(if layer + 1 < IN_GROUP { ',' } else { ']' });
            }
            json.push('}');
            json.push(if group + 1 < GROUPS { ',' } else { ']' });
        }
        json.push_str("},");
    }
    write!(json, r#"{{"type":"frame","id":"wide","x":0,"y":5500,"width":4800,"height":1000,"layout":"none","children":["#).unwrap();
    for layer in 0..WIDE {
        let (x, y) = ((layer % 48) * 100, (layer / 48) * 50);
        write!(json, r##"{{"type":"rectangle","id":"w{layer}","x":{x},"y":{y},"width":90,"height":40,"fill":"#22c55e"}}"##).unwrap();
        json.push(if layer + 1 < WIDE { ',' } else { ']' });
    }
    json.push_str("}]}");
    json
}

/// The large design open in an editor, laid out.
fn editor() -> Editor {
    let mut editor = Editor::new(pen_doc::read(&large_design()).unwrap(), None, 1);
    editor.refresh(&measure);
    editor
}

fn measure(style: &pen_doc::layout::TextStyle) -> (f32, f32) {
    #[expect(clippy::cast_precision_loss, reason = "a short test text")]
    let long = style.content.chars().count() as f32;
    (long * style.size * 0.5, style.size * 1.2)
}

fn count(nodes: &[pen_doc::Node]) -> usize {
    nodes.iter().map(|node| 1 + count(node.kids())).sum()
}

/// One frame of `draw` in a window of [`WINDOW`]; how long it took.
fn frame(ctx: &egui::Context, mut draw: impl FnMut(&mut egui::Ui)) -> Duration {
    let input = egui::RawInput { screen_rect: Some(ScreenRect::from_min_size(Pos2::ZERO, WINDOW)), focused: true, ..Default::default() };
    let start = Instant::now();
    let mut drawn = ctx.run_ui(input, |ui| draw(ui));
    let took = start.elapsed();
    drawn.textures_delta.clear();
    took
}

/// The middle one of `frames` frame times of `draw`, after `warm` frames that don't count.
fn median(ctx: &egui::Context, warm: usize, frames: usize, mut draw: impl FnMut(&mut egui::Ui)) -> Duration {
    for _ in 0..warm {
        frame(ctx, &mut draw);
    }
    let mut times: Vec<Duration> = (0..frames).map(|_| frame(ctx, &mut draw)).collect();
    times.sort();
    times[times.len() / 2]
}

fn ms(time: Duration) -> String {
    format!("{:.3} ms", time.as_secs_f64() * 1000.0)
}

/// Frames of the canvas until its drawing of the design is built.
fn settle(ctx: &egui::Context, canvas: &mut Canvas, editor: &mut Editor) {
    for _ in 0..400 {
        frame(ctx, |ui| canvas.show(ui, editor, false));
        if !ctx.has_requested_repaint() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
#[ignore = "timing: prints frame times"]
fn timing_27_canvas_overlays_with_a_large_selection() {
    let ctx = egui::Context::default();
    let mut editor = editor();
    println!("layers: {}", count(&editor.doc.children));
    let mut canvas = Canvas::default();
    settle(&ctx, &mut canvas, &mut editor);
    let mut run = |name: &str, selection: Vec<String>, editor: &mut Editor| {
        editor.selection = selection;
        let time = median(&ctx, 5, 41, |ui| canvas.show(ui, editor, false));
        println!("27 canvas frame, {name}: {}", ms(time));
    };
    run("nothing selected", Vec::new(), &mut editor);
    run("one layer selected", vec!["w500".to_owned()], &mut editor);
    run("Select All in the wide board (960)", (0..WIDE).map(|layer| format!("w{layer}")).collect(), &mut editor);
    run("the 40 boards", (0..BOARDS).map(|board| format!("b{board}")).collect(), &mut editor);
    // The look-ups by themselves, as the overlay makes them: every selected layer's scene node.
    editor.selection = (0..WIDE).map(|layer| format!("w{layer}")).collect();
    let start = Instant::now();
    let mut found = 0;
    for _ in 0..20 {
        found += editor.selection.iter().filter_map(|id| editor.scene_node(id)).count();
    }
    println!("27 scene_node for 960 selected layers, once: {} (found {})", ms(start.elapsed() / 20), found / 20);
}

#[test]
#[ignore = "timing: prints frame times"]
fn timing_28_layers_panel() {
    let ctx = egui::Context::default();
    let mut editor = editor();
    let run = |name: &str, selection: Vec<String>, frames: usize, editor: &mut Editor| {
        editor.selection = selection;
        let time = median(&ctx, 2, frames, |ui| crate::panels::layers(ui, editor, false));
        println!("28 layers panel frame, {name}: {}", ms(time));
    };
    run("boards open, nothing selected", Vec::new(), 15, &mut editor);
    run("one deep layer selected", vec!["l20_5_7".to_owned()], 15, &mut editor);
    run("ten layers selected on ten boards", (0..10).map(|board| format!("l{board}_4_3")).collect(), 3, &mut editor);
    // Human: a selected layer opens everything it lies in, so one layer in every group unfolds the whole list.
    // Before the selection's ancestors were worked out once a frame this took minutes a frame; set
    // `TIMING_ALL_ROWS` to run it.
    if std::env::var_os("TIMING_ALL_ROWS").is_some() {
        let spread: Vec<String> = (0..BOARDS).flat_map(|board| (0..GROUPS).map(move |group| format!("l{board}_{group}_3"))).collect();
        run("400 layers selected, every group open (all rows)", spread, 15, &mut editor);
    }
}

/// How many texts one frame of the canvas draws.
fn texts_drawn(ctx: &egui::Context, canvas: &mut Canvas, editor: &mut Editor) -> usize {
    fn texts(shape: &egui::Shape) -> usize {
        match shape {
            egui::Shape::Text(_) => 1,
            egui::Shape::Vec(shapes) => shapes.iter().map(texts).sum(),
            _ => 0,
        }
    }
    let input = egui::RawInput { screen_rect: Some(ScreenRect::from_min_size(Pos2::ZERO, WINDOW)), focused: true, ..Default::default() };
    let drawn = ctx.run_ui(input, |ui| canvas.show(ui, editor, false));
    drawn.shapes.iter().map(|clipped| texts(&clipped.shape)).sum()
}

#[test]
#[ignore = "timing: prints frame times"]
fn timing_36_text_on_the_canvas() {
    let ctx = egui::Context::default();
    crate::fonts::install(&ctx);
    let mut editor = editor();
    let mut canvas = Canvas::default();
    // Sixteen boards in view, their text four pixels high: about 1,900 texts drawn a frame.
    frame(&ctx, |ui| canvas.show(ui, &mut editor, false));
    editor.camera = crate::editor::Camera { center: pen_doc::geom::Point::new(2950.0, 2150.0), zoom: 0.25 };
    for _ in 0..50 {
        settle(&ctx, &mut canvas, &mut editor);
        if crate::fonts::sync(&ctx) {
            editor.relayout();
            editor.refresh(&measure);
        }
    }
    let time = median(&ctx, 10, 61, |ui| canvas.show(ui, &mut editor, false));
    println!("36 canvas frame with {} texts in view: {}", texts_drawn(&ctx, &mut canvas, &mut editor), ms(time));
    let style = pen_doc::layout::TextStyle {
        content: "Layer 7 of 3".to_owned(),
        family: "Helvetica".to_owned(),
        size: 16.0,
        line_px: None,
        letter_spacing: 0.0,
        weight: 400,
        italic: false,
        underline: false,
        strikethrough: false,
        align: pen_doc::layout::TextAlign::Left,
        valign: pen_doc::layout::TextVAlign::Top,
        wrap: None,
    };
    println!("36 text_job (font choice and job), one text: {:.0} ns", crate::render::timing_text_job(&style, false));
    println!("36 text job with the choice kept, one text: {:.0} ns", crate::render::timing_text_job(&style, true));
    // What a kept galley would save on top: making the job and asking egui for the galley it has kept.
    let mut asked = 0.0;
    frame(&ctx, |ui| {
        let start = Instant::now();
        for _ in 0..100_000 {
            let job = crate::render::text_job(&style, egui::Color32::BLACK, 0.25);
            std::hint::black_box(ui.painter().layout_job(job));
        }
        asked = start.elapsed().as_secs_f64() * 1e9 / 100_000.0;
    });
    println!("36 job and egui's own kept galley, one text: {asked:.0} ns (of which the job with its face chosen anew, above)");
}
