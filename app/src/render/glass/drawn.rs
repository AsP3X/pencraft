//! Tests that draw whole documents with materials: on the CPU for the frosted material, and through the glass
//! shader where the machine has a GPU.

use std::fmt::Write as _;
use std::path::PathBuf;

use pen_doc::vars::Vars;

use crate::editor::Editor;
use crate::render::{DisplayList, Seen, Survey};

/// The first top-level layer of `json`, drawn at `scale` pixels per unit.
fn survey(json: &str, scale: f32) -> Survey {
    let mut editor = Editor::new(pen_doc::read(json).unwrap(), None, 1);
    editor.refresh(&|_| (10.0, 10.0));
    let ctx = egui::Context::default();
    DisplayList::default().survey(&[&editor.scene[0]], scale, Vars::new(&editor.doc), &ctx, None).unwrap()
}

/// The pixel of `survey` at a document point: premultiplied bytes.
#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a pixel position inside the picture")]
fn pixel(survey: &Survey, x: f32, y: f32) -> [u8; 4] {
    let (px, py) = (((x - survey.origin.x) * survey.scale) as usize, ((y - survey.origin.y) * survey.scale) as usize);
    *survey.picture.rgba[(py * survey.picture.w + px) * 4..].first_chunk::<4>().unwrap()
}

/// A 200 × 100 frame, black on its left half and white on its right, with `pane` over the middle.
fn halves(pane: &str) -> String {
    format!(
        r##"{{"version":"2.22","children":[{{"type":"frame","id":"page","x":0,"y":0,"width":200,"height":100,"layout":"none","fill":"#ffffff","children":[
          {{"type":"rectangle","id":"dark","x":0,"y":0,"width":100,"height":100,"fill":"#000000"}},
          {pane}]}}]}}"##
    )
}

#[test]
fn a_frosted_pane_blurs_and_tints_what_is_behind_it_and_leaves_the_rest_alone() {
    let pane = r#"{"type":"frame","id":"pane","x":40,"y":20,"width":120,"height":60,"cornerRadius":20,"material":{"type":"frosted","thickness":"ultra_thin","appearance":"light"}}"#;
    let survey = survey(&halves(pane), 1.0);
    // Outside the pane the halves are as they were, also in the pane's rounded-off corner.
    assert_eq!(pixel(&survey, 10.0, 50.0), [0, 0, 0, 255]);
    assert_eq!(pixel(&survey, 190.0, 50.0), [255, 255, 255, 255]);
    assert_eq!(pixel(&survey, 41.0, 21.0), [0, 0, 0, 255]);
    // Inside, the edge between black and white is a soft slope, lifted by the white tint.
    let [left, middle, right] = [60.0, 100.0, 140.0].map(|x| pixel(&survey, x, 50.0)[0]);
    assert!(left > 60 && left < middle && middle < right, "{left} {middle} {right}");
    assert_eq!(pixel(&survey, 100.0, 50.0)[3], 255);
    assert_eq!(survey.materials.len(), 1);
    assert!(!survey.materials[0].dark && survey.materials[0].spread > 0.9);
}

#[test]
fn the_appearance_follows_what_is_behind_unless_it_is_pinned() {
    let pane = |x: u32, appearance: &str| {
        format!(r#"{{"type":"frame","id":"p{x}","x":{x},"y":30,"width":40,"height":40,"material":{{"type":"frosted","appearance":"{appearance}"}}}}"#)
    };
    let survey = survey(&halves(&format!("{},{},{}", pane(20, "auto"), pane(140, "auto"), pane(150, "dark"))), 1.0);
    let dark = |id: &str| survey.materials.iter().find(|seen| seen.id == id).unwrap().dark;
    assert!(dark("p20") && !dark("p140") && dark("p150"));
    // Dark over black stays dark, light over white stays light.
    assert!(pixel(&survey, 30.0, 50.0)[0] < 60 && pixel(&survey, 145.0, 50.0)[0] > 240);
}

#[test]
fn what_is_in_a_pane_is_drawn_sharp_on_top_of_it() {
    let pane = r##"{"type":"frame","id":"pane","x":40,"y":20,"width":120,"height":60,"material":"frosted","layout":"none","stroke":"#ff0000","strokeWidth":2,"strokeAlignment":"inner","children":[
        {"type":"rectangle","id":"dot","x":50,"y":20,"width":20,"height":20,"fill":"#00ff00"}]}"##;
    let survey = survey(&halves(pane), 1.0);
    assert_eq!(pixel(&survey, 100.5, 50.5), [0, 255, 0, 255]);
    assert_eq!(pixel(&survey, 100.5, 20.5), [255, 0, 0, 255]);
}

#[test]
fn a_pane_over_nothing_is_only_its_tint() {
    let json = r#"{"version":"2.22","children":[{"type":"frame","id":"pane","x":0,"y":0,"width":80,"height":80,"material":{"type":"frosted","thickness":"thin","advanced":{"highlight":0}}}]}"#;
    let survey = survey(json, 1.0);
    let [r, g, b, a] = pixel(&survey, 40.0, 40.0);
    // An export has a clear ground: thin frost is a 35 % white veil, premultiplied.
    assert!((i32::from(a) - 89).abs() <= 2 && r == a && g == a && b == a, "{r} {g} {b} {a}");
}

#[test]
fn a_glass_group_is_one_pane_for_all_its_glass_children() {
    let json = r##"{"version":"2.22","children":[{"type":"frame","id":"page","x":0,"y":0,"width":200,"height":100,"layout":"none","fill":"#c0c0c0","children":[
        {"type":"frame","id":"row","x":20,"y":30,"layout":"horizontal","gap":8,"glassGroup":8,"children":[
          {"type":"frame","id":"a","width":40,"height":40,"cornerRadius":20,"material":"liquid_glass"},
          {"type":"frame","id":"b","width":40,"height":40,"cornerRadius":20,"material":"liquid_glass"}]}]}]}"##;
    let survey = survey(json, 1.0);
    let ids: Vec<&str> = survey.materials.iter().map(|seen| seen.id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);
    assert_eq!(survey.materials[0], Seen { id: "a".to_owned(), ..survey.materials[1].clone() });
    // Light glass over flat light grey: lighter than the grey inside each child, the grey itself around them.
    assert!(pixel(&survey, 40.0, 50.0)[0] > 195 && pixel(&survey, 88.0, 50.0)[0] > 195);
    assert_eq!(pixel(&survey, 10.0, 10.0), [192, 192, 192, 255]);
}

#[test]
fn a_clipping_frame_cuts_its_children_to_its_rounded_corners_and_takes_no_room_for_what_it_cuts_off() {
    let json = r##"{"version":"2.22","children":[{"type":"frame","id":"phone","x":0,"y":0,"width":100,"height":100,"cornerRadius":[30,30,0,30],"clip":true,"layout":"none","children":[
        {"type":"rectangle","id":"photo","x":-20,"y":-20,"width":200,"height":200,"fill":"#ff0000"},
        {"type":"rectangle","id":"bar","x":10,"y":40,"width":80,"height":20,"fill":"#0000ff"}]}]}"##;
    let survey = survey(json, 1.0);
    // The picture is the frame, not the child that sticks out of it.
    assert_eq!((survey.picture.w, survey.picture.h), (100, 100));
    // Rounded corners are cut clear; the square one, the edges and the middle are filled; order is kept.
    for (x, y) in [(2.5, 2.5), (97.5, 2.5), (2.5, 97.5)] {
        assert_eq!(pixel(&survey, x, y)[3], 0, "the corner at {x}, {y} isn't cut");
    }
    for (x, y) in [(97.5, 97.5), (50.5, 1.5), (1.5, 50.5), (50.5, 98.5), (20.5, 20.5), (50.5, 80.5)] {
        assert_eq!(pixel(&survey, x, y), [255, 0, 0, 255], "at {x}, {y}");
    }
    assert_eq!(pixel(&survey, 50.5, 50.5), [0, 0, 255, 255]);
    // Along the rounded edge the cut is smooth, not a step.
    let edge = pixel(&survey, 8.5, 8.5)[3];
    assert!(edge > 0 && edge < 255, "{edge}");
}

/// A screen of four glass panes (regular and clear, light and dark) over `backdrop` layers.
fn panes_over(backdrop: &str) -> String {
    let mut panes = String::new();
    for (index, (variant, appearance)) in [("regular", "light"), ("regular", "dark"), ("clear", "light"), ("clear", "dark")].iter().enumerate() {
        let (x, y) = (20 + 190 * (index % 2), 20 + 80 * (index / 2));
        let _ = write!(
            panes,
            r#",{{"type":"frame","id":"g{index}","x":{x},"y":{y},"width":170,"height":60,"cornerRadius":30,"material":{{"type":"liquid_glass","variant":"{variant}","appearance":"{appearance}"}}}}"#
        );
    }
    format!(
        r##"{{"version":"2.22","children":[{{"type":"frame","id":"page","x":0,"y":0,"width":400,"height":180,"layout":"none","clip":true,"fill":"#f3f4f6","children":[{backdrop}{panes}]}}]}}"##
    )
}

/// A checkerboard of 20-unit squares.
fn checkerboard() -> String {
    let mut squares = Vec::new();
    for row in 0..9 {
        for column in 0..20 {
            if (row + column) % 2 == 0 {
                squares.push(format!(
                    r##"{{"type":"rectangle","id":"s{row}x{column}","x":{},"y":{},"width":20,"height":20,"fill":"#1f2937"}}"##,
                    column * 20,
                    row * 20
                ));
            }
        }
    }
    squares.join(",")
}

/// Something like a photo: a field of colour with soft and hard shapes on it.
const PHOTO: &str = r##"{"type":"rectangle","id":"sky","x":0,"y":0,"width":400,"height":180,"fill":{"type":"gradient","gradientType":"linear","rotation":30,"colors":[{"color":"#ff7a18","position":0},{"color":"#af002d","position":0.5},{"color":"#319197","position":1}]}},
    {"type":"ellipse","id":"sun","x":40,"y":-30,"width":120,"height":120,"fill":"#ffd166"},
    {"type":"ellipse","id":"leaf","x":230,"y":60,"width":160,"height":160,"fill":"#06d6a0"},
    {"type":"rectangle","id":"bar","x":150,"y":100,"width":120,"height":14,"fill":"#111111"},
    {"type":"ellipse","id":"dot","x":300,"y":10,"width":50,"height":50,"fill":"#ffffff"}"##;

/// Compares `survey` with the kept picture `name`; with `PENCRAFT_BLESS=1` it keeps the new one instead.
// Human: GPUs differ in the last bits, so a pixel may be a little off, and a few pixels (an edge that falls on
// the other side of a pixel's centre) a lot.
fn matches_golden(survey: &Survey, name: &str) {
    const NEAR: u8 = 10;
    const MOST_FAR: f64 = 0.004;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden").join("glass").join(format!("{name}.png"));
    let (w, h) = (u32::try_from(survey.picture.w).unwrap(), u32::try_from(survey.picture.h).unwrap());
    if std::env::var_os("PENCRAFT_BLESS").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbaImage::from_raw(w, h, survey.picture.rgba.clone()).unwrap().save(&path).unwrap();
        return;
    }
    let kept = image::open(&path).unwrap_or_else(|error| panic!("{}: {error}; run the tests once with PENCRAFT_BLESS=1", path.display())).to_rgba8();
    assert_eq!((kept.width(), kept.height()), (w, h), "{name} has another size");
    let (old, new) = (kept.as_raw().as_chunks::<4>().0, survey.picture.rgba.as_chunks::<4>().0);
    let far = old.iter().zip(new).filter(|(a, b)| a.iter().zip(*b).any(|(a, b)| a.abs_diff(*b) > NEAR)).count();
    let share = f64::from(u32::try_from(far).unwrap()) / f64::from(w * h);
    assert!(share <= MOST_FAR, "{name}: {far} pixels ({:.2} %) differ from the kept picture by more than {NEAR}", share * 100.0);
}

/// Whether the glass shader can run here. A machine without a GPU adapter skips the shader's tests, unless
/// `PENCRAFT_REQUIRE_GPU` is set, as it should be where the tests must not be skipped.
fn shader_runs() -> bool {
    let ready = crate::gpu::install_for_tests();
    assert!(ready || std::env::var_os("PENCRAFT_REQUIRE_GPU").is_none(), "no GPU adapter was found for the glass shader's tests");
    ready
}

#[test]
fn glass_over_a_checkerboard_and_over_a_photo_matches_the_kept_pictures_at_two_zooms() {
    if !shader_runs() {
        return;
    }
    for (name, backdrop) in [("checkerboard", checkerboard()), ("photo", PHOTO.to_owned())] {
        for (zoom, scale) in [("1x", 1.0), ("2x", 2.0)] {
            matches_golden(&survey(&panes_over(&backdrop), scale), &format!("{name}-{zoom}"));
        }
    }
}

#[test]
fn the_rim_bends_what_is_behind_it_and_the_middle_does_not() {
    if !shader_runs() {
        return;
    }
    let page = |refraction: u8| {
        format!(
            r##"{{"version":"2.22","children":[{{"type":"frame","id":"page","x":0,"y":0,"width":400,"height":180,"layout":"none","fill":"#f3f4f6","children":[{},
              {{"type":"frame","id":"pane","x":100,"y":50,"width":200,"height":100,"cornerRadius":50,"material":{{"type":"liquid_glass","variant":"clear","appearance":"light","advanced":{{"refraction":{refraction},"highlight":0,"blur":0}}}}}}]}}]}}"##,
            checkerboard()
        )
    };
    let (bent, flat) = (survey(&page(1), 2.0), survey(&page(0), 2.0));
    // How many places in a box differ clearly between the two.
    let differing = |x0: u16, y0: u16, x1: u16, y1: u16| {
        let mut count = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                let at = (f32::from(x) + 0.25, f32::from(y) + 0.25);
                count += u32::from(pixel(&bent, at.0, at.1)[0].abs_diff(pixel(&flat, at.0, at.1)[0]) > 40);
            }
        }
        count
    };
    // The pane's top edge lies halfway down a row of squares: the band along it shows the next row, which a
    // flat pane doesn't; the middle shows the same in both.
    assert!(differing(150, 51, 250, 59) > 100, "the rim doesn't bend: {}", differing(150, 51, 250, 59));
    assert_eq!(differing(150, 90, 250, 110), 0);
    // Outside the pane nothing changes.
    assert_eq!(differing(0, 0, 400, 30), 0);
}
