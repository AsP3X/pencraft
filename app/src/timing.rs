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

/// The canvas with the camera at `center` and `zoom`, built and at rest.
fn looking_at(ctx: &egui::Context, canvas: &mut Canvas, editor: &mut Editor, center: (f32, f32), zoom: f32) {
    frame(ctx, |ui| canvas.show(ui, editor, false));
    editor.camera = crate::editor::Camera { center: pen_doc::geom::Point::new(center.0, center.1), zoom };
    for _ in 0..20 {
        settle(ctx, canvas, editor);
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// A design of shapes with many corners each: forty boards of rounded, outlined cards and circles, 9,640 layers.
// Human: the large design is rectangles of four corners, which cost next to nothing to copy. A real design
// is round corners, outlines, circles and icons, and those are what moving meshes to the screen costs.
fn round_design() -> String {
    let mut json = String::from(r#"{"version":"2.22","children":["#);
    for board in 0..BOARDS {
        let (x, y) = ((board % 8) * 1500, (board / 8) * 1100);
        write!(
            json,
            r##"{{"type":"frame","id":"b{board}","x":{x},"y":{y},"width":1400,"height":1000,"layout":"none","fill":"#ffffff","cornerRadius":40,"children":["##
        )
        .unwrap();
        for layer in 0..240 {
            let (lx, ly) = ((layer % 20) * 68 + 20, (layer / 20) * 80 + 20);
            let id = format!("r{board}_{layer}");
            if layer % 2 == 0 {
                write!(json, r##"{{"type":"rectangle","id":"{id}","x":{lx},"y":{ly},"width":60,"height":70,"cornerRadius":16,"fill":"#3b82f6","stroke":{{"thickness":2,"fill":"#1e3a8a"}}}}"##).unwrap();
            } else {
                write!(json, r##"{{"type":"ellipse","id":"{id}","x":{lx},"y":{ly},"width":60,"height":60,"fill":"#f59e0b","stroke":{{"thickness":2,"fill":"#78350f"}}}}"##).unwrap();
            }
            json.push(if layer + 1 < 240 { ',' } else { ']' });
        }
        json.push('}');
        json.push(if board + 1 < BOARDS { ',' } else { ']' });
    }
    json.push('}');
    json
}

/// How many meshes one frame of the canvas hands to egui, and the corners they have between them.
fn meshes_drawn(ctx: &egui::Context, canvas: &mut Canvas, editor: &mut Editor) -> (usize, usize) {
    fn meshes(shape: &egui::Shape) -> (usize, usize) {
        match shape {
            egui::Shape::Mesh(mesh) => (1, mesh.vertices.len()),
            egui::Shape::Vec(shapes) => shapes.iter().map(meshes).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1)),
            _ => (0, 0),
        }
    }
    let input = egui::RawInput { screen_rect: Some(ScreenRect::from_min_size(Pos2::ZERO, WINDOW)), focused: true, ..Default::default() };
    let drawn = ctx.run_ui(input, |ui| canvas.show(ui, editor, false));
    drawn.shapes.iter().map(|clipped| meshes(&clipped.shape)).fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
}

#[test]
#[ignore = "timing: prints frame times"]
fn timing_40_frames_with_the_camera_still_and_moving() {
    let ctx = egui::Context::default();
    for (design, text) in [("rectangles and text", large_design()), ("round shapes", round_design())] {
        let mut editor = Editor::new(pen_doc::read(&text).unwrap(), None, 1);
        editor.refresh(&measure);
        let mut canvas = Canvas::default();
        for (name, center, zoom) in [("the whole design in view", (5950.0, 3250.0), 0.13), ("sixteen boards in view", (2950.0, 2150.0), 0.25)] {
            looking_at(&ctx, &mut canvas, &mut editor, center, zoom);
            let (meshes, corners) = meshes_drawn(&ctx, &mut canvas, &mut editor);
            println!("40 {design}, {name}: {meshes} meshes with {corners} corners");
            let still = median(&ctx, 10, 61, |ui| canvas.show(ui, &mut editor, false));
            println!("40 {design}, {name}, camera still: {}", ms(still));
            // A pan: the camera a little further each frame, within the part whose effects are baked.
            let mut step = 0.0_f32;
            let moving = median(&ctx, 10, 61, |ui| {
                step += 1.0;
                editor.camera.center.x = center.0 + (step % 20.0);
                canvas.show(ui, &mut editor, false);
            });
            println!("40 {design}, {name}, camera moving: {}", ms(moving));
            editor.camera.center.x = center.0;
        }
    }
}

/// How much memory this process holds at the moment, in megabytes, as `ps` tells it.
fn held_mb() -> f64 {
    let told = std::process::Command::new("ps").args(["-o", "rss=", "-p", &std::process::id().to_string()]).output().unwrap();
    String::from_utf8_lossy(&told.stdout).trim().parse::<f64>().unwrap_or(0.0) / 1024.0
}

#[test]
#[ignore = "timing: prints times and memory"]
fn timing_41_a_hundred_undo_steps_on_a_large_design() {
    let mut editor = editor();
    let before = held_mb();
    let mut steps = Duration::ZERO;
    // A hundred edits, each of one layer, on five of the forty boards in turn.
    for edit in 0..100_u16 {
        let id = format!("l{}_{}_{}", edit % 5, edit % 10, (edit % 12) * 2);
        let start = Instant::now();
        editor.checkpoint();
        steps += start.elapsed();
        editor.change(&id, &|node| node.x = Some(serde_json::Number::from(edit)));
        editor.refresh(&measure);
    }
    println!("41 memory for 100 undo steps on {} layers: {:.1} MB", count(&editor.doc.children), held_mb() - before);
    println!("41 one checkpoint: {}", ms(steps / 100));
    let (roots, layers) = editor.history_holds();
    println!("41 the 100 steps hold {roots} top-level nodes with {layers} layers between them");
    let start = Instant::now();
    for _ in 0..100 {
        editor.undo();
    }
    println!("41 one undo: {}", ms(start.elapsed() / 100));
    let start = Instant::now();
    for _ in 0..100 {
        editor.redo();
    }
    println!("41 one redo: {}", ms(start.elapsed() / 100));
    println!("41 memory with the 100 steps taken back and made again: {:.1} MB", held_mb() - before);
}

/// One artboard that holds everything: 440 groups of 24 rectangles and texts in one frame, 11,001 layers.
fn one_board_design() -> String {
    let mut json = String::from(
        r##"{"version":"2.22","children":[{"type":"frame","id":"board","x":0,"y":0,"width":6000,"height":10000,"layout":"none","fill":"#ffffff","children":["##,
    );
    for group in 0..440 {
        let (gx, gy) = ((group % 22) * 270 + 20, (group / 22) * 480 + 20);
        write!(json, r#"{{"type":"frame","id":"g{group}","x":{gx},"y":{gy},"width":250,"height":460,"layout":"none","children":["#).unwrap();
        for layer in 0..IN_GROUP {
            let (lx, ly) = ((layer % 2) * 120 + 5, (layer / 2) * 36 + 5);
            let id = format!("s{group}_{layer}");
            if layer % 2 == 0 {
                write!(json, r##"{{"type":"rectangle","id":"{id}","x":{lx},"y":{ly},"width":110,"height":30,"fill":"#3b82f6"}}"##).unwrap();
            } else {
                write!(json, r##"{{"type":"text","id":"{id}","x":{lx},"y":{ly},"content":"Layer {layer} of {group}","fontSize":16,"fontFamily":"Helvetica","fill":"#111111"}}"##).unwrap();
            }
            json.push(if layer + 1 < IN_GROUP { ',' } else { ']' });
        }
        json.push('}');
        json.push(if group + 1 < 440 { ',' } else { ']' });
    }
    json.push_str("}]}");
    json
}

#[test]
#[ignore = "timing: prints frame times"]
fn timing_42_dragging_one_layer_in_a_large_artboard() {
    let ctx = egui::Context::default();
    let mut editor = Editor::new(pen_doc::read(&one_board_design()).unwrap(), None, 1);
    editor.refresh(&measure);
    println!("42 layers: {}", count(&editor.doc.children));
    let mut canvas = Canvas::default();
    looking_at(&ctx, &mut canvas, &mut editor, (700.0, 500.0), 1.0);
    editor.checkpoint();
    // A drag: the layer a unit further each frame, the scene brought up to date, the canvas drawn. Once with
    // the drag saying which layer it moves, once leaving it to be found by comparing the document.
    for tracked in [true, false] {
        let (mut edits, mut frames) = (Vec::new(), Vec::new());
        for step in 0..60_u16 {
            let start = Instant::now();
            if tracked {
                editor.touched_only(&["s25_4".to_owned()]);
            } else {
                editor.touched();
            }
            editor.change("s25_4", &|node| node.x = Some(serde_json::Number::from(step)));
            editor.refresh(&measure);
            edits.push(start.elapsed());
            frames.push(frame(&ctx, |ui| canvas.show(ui, &mut editor, false)));
        }
        edits.sort();
        frames.sort();
        let how = if tracked { "the layer named" } else { "the document compared" };
        println!("42 one drag step, {how}: the document and its scene brought up to date: {}", ms(edits[30]));
        println!("42 one drag step, {how}: the canvas frame after it: {}", ms(frames[30]));
    }
}

/// One edit of the large design, a hundred times over: how long the edit with its undo step and the scene
/// brought up to date took, and the canvas frame after it (the middle one of each).
fn edits(name: &str, ctx: &egui::Context, canvas: &mut Canvas, editor: &mut Editor, mut edit: impl FnMut(&mut Editor, u16)) {
    let (mut steps, mut frames) = (Vec::new(), Vec::new());
    for turn in 0..100_u16 {
        let start = Instant::now();
        edit(editor, turn);
        editor.refresh(&measure);
        steps.push(start.elapsed());
        frames.push(frame(ctx, |ui| canvas.show(ui, editor, false)));
    }
    steps.sort();
    frames.sort();
    println!("53 {name}: the edit and its scene: {}; the canvas frame after it: {}", ms(steps[50]), ms(frames[50]));
}

#[test]
#[ignore = "timing: prints times"]
fn timing_53_edits_that_know_their_layers() {
    let ctx = egui::Context::default();
    let mut editor = editor();
    let mut canvas = Canvas::default();
    looking_at(&ctx, &mut canvas, &mut editor, (700.0, 500.0), 1.0);
    let visible = pen_doc::geom::Rect::new(0.0, 0.0, 1400.0, 1000.0);
    let layer = |turn: u16| format!("l{}_{}_{}", turn % 5, turn % 10, (turn % 6) * 2);
    edits("duplicate a layer", &ctx, &mut canvas, &mut editor, |editor, turn| {
        editor.selection = vec![layer(turn)];
        editor.duplicate();
    });
    edits("delete a layer", &ctx, &mut canvas, &mut editor, |editor, turn| {
        editor.selection = vec![format!("l1_{}_{}", turn / 10, 12 + turn % 10)];
        editor.delete_selection();
    });
    edits("paste a layer into a frame", &ctx, &mut canvas, &mut editor, |editor, turn| {
        let copy = pen_doc::edit::find(&editor.doc, &layer(turn + 1)).cloned().unwrap();
        editor.selection = vec![format!("g{}_{}", turn % 5, turn % 10)];
        editor.paste(&[copy], visible);
    });
    edits("undo", &ctx, &mut canvas, &mut editor, |editor, _| editor.undo());
    edits("redo", &ctx, &mut canvas, &mut editor, |editor, _| editor.redo());
    edits("the assistant updates a layer and adds one", &ctx, &mut canvas, &mut editor, |editor, turn| {
        let group = format!("g{}_{}", turn % 5, turn % 10);
        let call = serde_json::json!({ "operations": [
            { "op": "update", "id": layer(turn), "properties": { "fill": "#ef4444" } },
            { "op": "insert", "parent": group, "node": { "type": "rectangle", "width": 20, "height": 20, "fill": "#000000" } },
        ] });
        // The order `App::edit_in` keeps: the change, then the undo step for the document as it was.
        let before = editor.doc.clone();
        let (results, _) = crate::agent::edit(editor, &call).unwrap();
        let changed = crate::agent::changed_layers(&before, &editor.doc, &call, &results);
        let after = std::mem::replace(&mut editor.doc, before);
        editor.checkpoint();
        editor.doc = after;
        if let Some(changed) = changed {
            editor.changed_only(&changed);
        }
    });
    editor.selection = vec![layer(3)];
    let start = editor.scale_start().unwrap();
    editor.checkpoint();
    edits("a step of the scale drag", &ctx, &mut canvas, &mut editor, |editor, turn| {
        let written = editor.scale_from(&start, 1.0 + f32::from(turn) / 200.0, start.bounds.center());
        editor.touched_only(&written);
    });
}

/// How many photos the photo design shows, and each one's size in pixels.
const PHOTOS: usize = 24;
const PHOTO: (u32, u32) = (4000, 3000);

/// A design of [`PHOTOS`] large photos, six to a row, written into a new folder with its picture files;
/// the editor for it, and the folder.
// Human: one photo is drawn and encoded once (a JPEG, as a camera writes it) and copied under a name per
// layer, so every layer has a file of its own to read.
fn photo_design() -> (Editor, std::path::PathBuf) {
    let folder = std::env::temp_dir().join(format!("quill-timing-photos-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let photo = image::RgbImage::from_fn(PHOTO.0, PHOTO.1, |x, y| {
        let wave = |v: u32, by: u32| u8::try_from((v / by + (x ^ y) % 23) % 256).unwrap_or(0);
        image::Rgb([wave(x, 16), wave(y, 12), wave(x + y, 28)])
    });
    let first = folder.join("photo-0.jpg");
    photo.save(&first).unwrap();
    let mut json = String::from(r#"{"version":"2.22","children":["#);
    for at in 0..PHOTOS {
        if at > 0 {
            std::fs::copy(&first, folder.join(format!("photo-{at}.jpg"))).unwrap();
            json.push(',');
        }
        let (x, y) = ((at % 6) * 420, (at / 6) * 320);
        write!(
            json,
            r#"{{"type":"rectangle","id":"p{at}","x":{x},"y":{y},"width":400,"height":300,"fill":{{"type":"image","url":"./photo-{at}.jpg","mode":"cover"}}}}"#
        )
        .unwrap();
    }
    json.push_str("]}");
    let mut editor = Editor::new(pen_doc::read(&json).unwrap(), Some(folder.join("photos.pen")), 1);
    editor.refresh(&measure);
    (editor, folder)
}

#[test]
#[ignore = "timing: prints times and memory"]
fn timing_37_opening_a_design_of_many_large_photos() {
    let ctx = egui::Context::default();
    let (mut editor, folder) = photo_design();
    editor.camera = crate::editor::Camera { center: pen_doc::geom::Point::new(1250.0, 630.0), zoom: 0.6 };
    let mut canvas = Canvas::default();
    let before = held_mb();
    let start = Instant::now();
    let first = frame(&ctx, |ui| canvas.show(ui, &mut editor, false));
    println!("37 first frame of {PHOTOS} photos of {} x {}: {}", PHOTO.0, PHOTO.1, ms(first));
    // Frames until every photo is drawn: the longest of them is how long the window stood still.
    let mut longest = first;
    for _ in 0..2000 {
        if canvas.list().pictures().0 == 0 && !ctx.has_requested_repaint() {
            break;
        }
        std::thread::sleep(Duration::from_millis(4));
        longest = longest.max(frame(&ctx, |ui| canvas.show(ui, &mut editor, false)));
    }
    println!("37 every photo drawn after {}; the longest frame until then: {}", ms(start.elapsed()), ms(longest));
    println!("37 memory after opening (this process, without the textures on the graphics card): {:.1} MB", held_mb() - before);
    let (_, pictures, bytes) = canvas.list().pictures();
    #[expect(clippy::cast_precision_loss, reason = "megabytes to show")]
    let megabytes = bytes as f64 / (1024.0 * 1024.0);
    println!("37 the pictures kept: {pictures}, {megabytes:.1} MB of textures and copies for effects");
    std::fs::remove_dir_all(&folder).unwrap();
}
