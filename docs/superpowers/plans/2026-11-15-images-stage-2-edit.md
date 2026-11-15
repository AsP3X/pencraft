# Pictures, stage 2: Edit by description — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A picture in a Pencraft document can be changed by an instruction: the assistant's `edit_image` tool, an edit mode of the Generate Image dialog, and a one-line field on an image fill.

**Architecture:** Stage 1's request gains pictures to send (`Source`). A new `imagery/sources.rs` gets each picture ready on the worker thread (read, scale down, encode); layers "as drawn" are rendered on the main thread before the job starts and travel as bytes. The Gemini body carries the pictures after the text. Everything around a job (parked calls, limits, giving up, saving, credits) is stage 1's and is reused, not copied.

**Tech Stack:** Rust 2024, egui 0.37 (storytold fork), `ureq`, `image`, `serde_json`, craft-dialogs for the file picker. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-11-15-images-stage-2-edit-design.md`. Stage 1's spec and code are the base: `docs/superpowers/specs/2026-11-14-images-design.md`, `app/src/imagery*`, `app/src/app/pictures.rs`.

---

## Read this first

All paths are relative to `app/` (the Cargo workspace root). Run every command from `app/`.

**House rules (`app/AGENTS.md`), enforced by lints:** no `unwrap`/`expect`/`panic!` outside tests; no `#[allow]`; `dead_code` is denied and never silenced; `///` on public items; `// Human:` / `// Agent:` comments on non-trivial code; UI colours and sizes only from `src/theme.rs`; the egui fork's forms (`egui::Id::unique`, `ui.scope_id()`); native dialogs only through craft-dialogs. **Nothing is committed unless the user has asked.**

**The gate passes after every task.** `bash scripts/check.sh` must end with `check: all passed` at the end of each task. Stage 1 learned that code used only by tests fails the gate (`dead_code`), so every task here is a slice that is used by the app when the task ends. If you find yourself wanting `#[cfg(test)]` on shipped code to get past the lint, the slice is cut wrong: stop and report.

**How this plan is written.** Stage 1's plan gave every line of code, and much of it had to be changed by the people building it, because it was written without the code in front of it. This plan gives, for each task: the exact types and signatures that later tasks depend on, the tests as code where the behaviour is plain logic, the behaviour in words where it is interface, and what "done" means. Read the named files before each task and follow their existing patterns; where this plan and the code disagree about a name, the code wins and you report it.

**What stage 1 already gives you (read before Task 2):**

- `imagery/service.rs`: `Request { prompt, model, aspect: Aspect, large, transparent }`, `Made(Vec<u8>)`, `Failure`, `post`.
- `imagery/gemini.rs`: `body`, `read`, `make`, `explained` (a failure with what the service left unsaid).
- `imagery.rs`: `Kind` (`ALL`, `name`, `named`, `base`, `model`, `transparent`), `Ready`, `ready`, `pick`, `listed`, `request`, `key`, `make`.
- `imagery/jobs.rs`: `Order { ready, request, document }`, `Done`, `Flags`, `work(order, flags)`, `Job`.
- `imagery/store.rs`: `save(document, home, bytes, prompt, credit)`, `shown`, `forget`.
- `app/pictures.rs`: parked calls (`Waiting`, `start_picture`, `picture_order`, `finish_pictures`, `picture_answer`, `fill_picture`), the dialog's side (`Generating`, `open_picture_dialog`, `show_picture_dialog`, `dialog_order`, `place_picture`), helpers (`takes_picture`, `fill_with`, `lasting_url`, `hold`, `room`).
- `imagery/dialog.rs`: `Dialog`, `Did`.

## Files

| File | Responsibility in this stage |
|---|---|
| `src/imagery/service.rs` (modify) | `Source`; `Request.sources`; `Request.aspect` becomes `Option<Aspect>` |
| `src/imagery/sources.rs` (create) | Getting a picture ready to send: `Take`, `prepare` |
| `src/imagery/gemini.rs` (modify) | The body with pictures; the sentence about roles |
| `src/imagery/openai.rs` (modify) | Refuses a request that has sources |
| `src/imagery.rs` (modify) | `Kind::edits`; `listed` says it; `edit_request` |
| `src/imagery/jobs.rs` (modify) | `Order.takes`; `work` prepares them; the `"edited"` credit |
| `src/app/pictures.rs` (modify) | `edit_image` calls; rendering layers; replacing an image paint; placing beside; the dialog's and quick field's jobs |
| `src/agent.rs`, `src/chat/steps.rs`, `src/app/tools.rs` (modify) | The tool's schema, guide text, chat words, dispatch |
| `src/imagery/dialog.rs` (modify) | The row of pictures and roles; the edit title; what is sent |
| `src/panels.rs` (modify) | "Edit with AI…" and the quick field on an image paint |
| `src/settings.rs` (modify) | The switch's new name |
| `AI.md`, `MECHANICS.md` (modify) | Docs |

---

### Task 1: Ask the live service what an edit looks like

No code. Needs `GEMINI_API_KEY` set in the shell to a key on a billed project. If it is not set, stop and report `NEEDS_CONTEXT`; do not go on with guesses.

**Files:**
- Create: `../research_notes/imagery/gemini-edit.md` (what was asked and what came back, without picture data and without the key)

- [ ] **Step 1: Make two small test pictures**

In your scratchpad folder (never in the repo), with Python or any tool at hand: `subject.jpg` (a 1024×768 photo-like picture: generate one with the stage 1 request, `"A white mug on a wooden table"`, model `gemini-3.1-flash-lite-image`, `aspect_ratio` `"4:3"`, `image_size` `"1K"`, `mime_type` `"image/jpeg"`), and `logo.png` (a 256×256 flat red circle on white, drawn locally).

- [ ] **Step 2: One picture to change, lite model, no aspect**

POST `https://generativelanguage.googleapis.com/v1beta/interactions` with header `x-goog-api-key`, body:

```json
{"model":"gemini-3.1-flash-lite-image",
 "input":[{"type":"text","text":"Make it night, lit by a candle."},
          {"type":"image","mime_type":"image/jpeg","data":"<base64 of subject.jpg>"}],
 "response_format":{"type":"image","mime_type":"image/jpeg","image_size":"1K"}}
```

Record: status; where the picture is in the answer (stage 1: the `image` block of the `model_output` step); the result's pixel size (is it the input's 4:3?). Look at the picture: is it the same mug, at night?

- [ ] **Step 3: The same with `"aspect_ratio":"1:1"`** — record the size, and whether the content was kept.

- [ ] **Step 4: A picture and a reference, on both models**

Text: `"The first picture is the one to change; the others are references. Put the red circle from the second picture on the mug as a logo."`, then `subject.jpg`, then `logo.png` (`"mime_type":"image/png"`). Once with `gemini-3.1-flash-lite-image`, once with `gemini-3.1-flash-image`. Record status and message for each, and look at the results.

- [ ] **Step 5: The edges**

(a) Six pictures in one request (subject + five copies of the logo) on `gemini-3.1-flash-image`: status and message. (b) `logo.png` sent with `"mime_type":"image/jpeg"` (a wrong kind): status and message. (c) A 4096×3072 JPEG as the subject (scale `subject.jpg` up locally): status, message, time taken.

- [ ] **Step 6: Write it down**

`../research_notes/imagery/gemini-edit.md`: one short section per call: the request body with `"data"` cut to `"<…>"`, the status, the answer's shape (keys only for a picture; the whole `error` object for a failure), what you saw in the picture. End with a list: **what Tasks 2 to 4 must do differently from this plan**, if anything (for example: the lite model refuses references → `explained` must say so; aspect left out does not keep the shape → send the nearest of the seven aspects).

- [ ] **Step 7: Report** what was found. If anything contradicts the spec's "What goes to the service" or "Services", say so first.

---

### Task 2: Pictures in a request (the module, used by the app with no pictures yet)

**Files:**
- Modify: `src/imagery/service.rs`, `src/imagery/gemini.rs`, `src/imagery/openai.rs`, `src/imagery.rs`, `src/imagery/jobs.rs`, `src/imagery/store.rs` (only if the credit needs it), and the callers that build a `Request` or an `Order` (`src/app/pictures.rs`, tests)
- Create: `src/imagery/sources.rs`

Read Task 1's notes first; where they say this task must differ, they win.

- [ ] **Step 1: Types (the contract the later tasks build on)**

In `service.rs`:

```rust
/// A picture sent with a request: the one to change, or one to go by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    /// The bytes of a JPEG or PNG file.
    pub bytes: Vec<u8>,
    /// "image/jpeg" or "image/png".
    pub kind: &'static str,
    /// This is the picture to change; the others are references.
    pub change: bool,
}
```

`Request` gains `pub sources: Vec<Source>` (empty for a plain generation) and `aspect` becomes `Option<Aspect>` (`None`: the shape of the picture to change; a plain generation always has `Some`).

In `sources.rs`:

```rust
/// A picture to get ready for sending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Take {
    /// Where its bytes come from.
    pub from: From,
    /// This is the picture to change.
    pub change: bool,
    /// What the credit calls it: the file's name, or "drawn".
    pub name: String,
}

/// Where a picture to send comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum From {
    /// A picture file, read on the worker thread.
    File(PathBuf),
    /// Layers already drawn on the main thread, as PNG bytes.
    Drawn(Vec<u8>),
}

/// The longest side of a picture that is sent, in pixels.
pub const MOST_SIDE: u32 = 2048;
/// The most reference pictures in one request.
pub const MOST_REFERENCES: usize = 4;

/// Gets `takes` ready: reads, scales down to `MOST_SIDE`, and encodes each as JPEG, or as PNG when it has
/// transparency. The picture to change comes first.
///
/// # Errors
/// A sentence naming the picture that can't be read or isn't one.
pub fn prepare(takes: &[Take]) -> Result<Vec<Source>, String>;
```

In `jobs.rs`: `Order` gains `pub takes: Vec<Take>`. `work` calls `sources::prepare(&order.takes)` first, puts the result into a clone of the request, and for an order with takes writes the credit `{"made":"edited","service","model","prompt","from":[names…]}` in place of `"generated"`.

In `imagery.rs`: `pub fn edits(self) -> bool` on `Kind` (Gemini true, OpenAi false); `listed` adds `"edits"` to each service.

- [ ] **Step 2: Tests first** (write them, see them fail to compile, then implement)

`sources.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32, alpha: u8) -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(width, height, image::Rgba([200, 30, 30, alpha])).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        bytes.into_inner()
    }

    fn drawn(bytes: Vec<u8>, change: bool) -> Take {
        Take { from: From::Drawn(bytes), change, name: "drawn".to_owned() }
    }

    #[test]
    fn a_large_picture_is_scaled_down_and_a_small_one_is_left_its_size() {
        let ready = prepare(&[drawn(png(4096, 1024, 255), true), drawn(png(64, 32, 255), false)]).unwrap();
        let sizes: Vec<(u32, u32)> = ready.iter().map(|s| image::load_from_memory(&s.bytes).map(|p| (p.width(), p.height())).unwrap()).collect();
        assert_eq!(sizes, [(2048, 512), (64, 32)]);
    }

    #[test]
    fn an_opaque_picture_goes_as_jpeg_and_one_with_transparency_as_png() {
        let ready = prepare(&[drawn(png(8, 8, 255), false), drawn(png(8, 8, 120), false)]).unwrap();
        assert_eq!(ready.iter().map(|s| s.kind).collect::<Vec<_>>(), ["image/jpeg", "image/png"]);
        assert_eq!(image::guess_format(&ready[0].bytes).unwrap(), image::ImageFormat::Jpeg);
        assert_eq!(image::guess_format(&ready[1].bytes).unwrap(), image::ImageFormat::Png);
    }

    #[test]
    fn the_picture_to_change_comes_first_whatever_the_order_given() {
        let ready = prepare(&[drawn(png(8, 8, 255), false), drawn(png(16, 16, 255), true)]).unwrap();
        assert_eq!(ready.iter().map(|s| s.change).collect::<Vec<_>>(), [true, false]);
        assert_eq!(image::load_from_memory(&ready[0].bytes).unwrap().width(), 16);
    }

    #[test]
    fn a_file_that_is_missing_or_not_a_picture_is_named() {
        let missing = std::env::temp_dir().join("pencraft-no-such-picture.png");
        let take = Take { from: From::File(missing.clone()), change: true, name: "pencraft-no-such-picture.png".to_owned() };
        let why = prepare(&[take]).unwrap_err();
        assert!(why.contains("pencraft-no-such-picture.png"), "{why}");
        let why = prepare(&[drawn(b"<html>".to_vec(), true)]).unwrap_err();
        assert!(why.contains("isn't a picture"), "{why}");
    }
}
```

`gemini.rs` (add; adapt the first assertion's `mime_type` and anything else to Task 1's notes):

```rust
    #[test]
    fn pictures_follow_the_words_and_roles_are_said_only_when_both_are_there() {
        let source = |change| Source { bytes: b"foo".to_vec(), kind: "image/jpeg", change };
        let one = body(&Request { sources: vec![source(true)], aspect: None, ..request() });
        assert_eq!(one["input"], json!([{ "type": "text", "text": "A red fox" }, { "type": "image", "mime_type": "image/jpeg", "data": "Zm9v" }]));
        assert!(one["response_format"].get("aspect_ratio").is_none(), "no shape asked for: the picture's own");
        let both = body(&Request { sources: vec![source(true), source(false)], ..request() });
        assert_eq!(both["input"][0]["text"], "The first picture is the one to change; the others are references. A red fox");
        assert_eq!(both["input"].as_array().unwrap().len(), 3);
        let only_references = body(&Request { sources: vec![source(false)], ..request() });
        assert_eq!(only_references["input"][0]["text"], "A red fox");
    }
```

`openai.rs`: a request with sources is refused before anything is sent: `Failure::Other("Editing isn't possible with this kind of service yet; use a Google Gemini service.")`. Test it on the function that decides (make that function not need a network).

`imagery.rs`: `listed` includes `"edits": true` for Gemini and `false` for OpenAI Images (extend the existing test).

`jobs.rs`: a test of the credit: a small pure function `credit(order, names) -> Value` gives `"generated"` without takes and `"edited"` with `"from"` listing the names in the order sent.

- [ ] **Step 3: Implement** until those tests pass. Update every place that builds a `Request` (now `aspect: Some(..)`, `sources: Vec::new()`) or an `Order` (`takes: Vec::new()`). The existing behaviour of generation must not change: all stage 1 tests pass unmodified except for the two new fields.

- [ ] **Step 4: Gate.** `bash scripts/check.sh` → `check: all passed`. Everything added here is reachable from `jobs::work`, so nothing is dead.

- [ ] **Step 5: Live test.** Add to `jobs.rs` tests, beside stage 1's `the_real_gemini_service_makes_a_picture…`, an `#[ignore]` test `the_real_gemini_service_changes_a_picture`: an `Order` with one `From::Drawn` take (a locally made 512×384 PNG, `change: true`), prompt "Make the whole picture blue.", `aspect: None`; it asserts the result is saved, its credit says `"edited"` with `"from": ["drawn"]`, and the saved picture is wider than tall. If `GEMINI_API_KEY` is set, run it (`cargo test -p pencraft -- --ignored the_real_gemini_service_changes`) and report the result; if not, say it was not run.

- [ ] **Step 6: Checkpoint.** Commit only if the user asked.

---

### Task 3: The `edit_image` tool for image fills and files

**Files:**
- Modify: `src/imagery.rs`, `src/app/pictures.rs`, `src/app/tools.rs`, `src/agent.rs`, `src/chat/steps.rs`, `src/settings.rs`

- [ ] **Step 1: Reading the call** — in `imagery.rs`:

```rust
/// What an `edit_image` call names as its pictures, before anything is read or drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    /// The picture to change, if any.
    pub change: Option<Pick>,
    /// The reference pictures, at most `sources::MOST_REFERENCES`.
    pub references: Vec<Pick>,
}

/// One picture a call names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    /// A layer's image fill (`layer`), or a reference that is a layer id.
    Layer(String),
    /// Layers to draw together (`layers`).
    Drawn(Vec<String>),
    /// A picture file by its full path (`file`, or a reference that is a path).
    File(PathBuf),
}

/// The pictures an `edit_image` call names.
///
/// # Errors
/// A sentence when more than one of `layer`, `layers`, `file` is given, when there are more than four
/// references, when a path isn't a full one, or when the call names no picture at all.
pub fn named(args: &Value) -> Result<Named, String>;

/// The request an `edit_image` call asks of the service `ready`: `instruction` as the prompt; `aspect`
/// `None` unless given; `size` and `service` as in `request`.
///
/// # Errors
/// A sentence saying which argument is wrong, or that this kind of service can't edit.
pub fn edit_request(args: &Value, ready: &Ready) -> Result<Request, String>;
```

A reference string is a file when it is an absolute path (`Path::is_absolute`), otherwise a layer id. Tests (write first), covering each sentence:

```rust
    #[test]
    fn an_edit_call_names_its_pictures_or_says_what_is_wrong() {
        let named = |args: Value| super::named(&args);
        assert_eq!(named(json!({ "layer": "hero" })), Ok(Named { change: Some(Pick::Layer("hero".to_owned())), references: vec![] }));
        assert_eq!(
            named(json!({ "layers": ["a", "b"], "references": ["logo"] })),
            Ok(Named { change: Some(Pick::Drawn(vec!["a".to_owned(), "b".to_owned()])), references: vec![Pick::Layer("logo".to_owned())] })
        );
        let full = if cfg!(windows) { "C:/pictures/a.png" } else { "/pictures/a.png" };
        assert_eq!(named(json!({ "file": full })).map(|n| n.change), Ok(Some(Pick::File(PathBuf::from(full)))));
        assert_eq!(named(json!({ "references": [full] })), Ok(Named { change: None, references: vec![Pick::File(PathBuf::from(full))] }));
        assert_eq!(named(json!({ "layer": "a", "file": full })), Err("Give one of `layer`, `layers` and `file`, not several: one picture is changed at a time.".to_owned()));
        assert_eq!(named(json!({ "file": "a.png" })), Err("`file` must be a full path, starting at the root: \"a.png\" isn't.".to_owned()));
        assert_eq!(named(json!({ "references": ["a", "b", "c", "d", "e"] })), Err("At most 4 reference pictures can be sent; 5 were named.".to_owned()));
        assert_eq!(named(json!({ "layers": [] })), Err("`layers` is empty: name the layers to draw.".to_owned()));
        assert_eq!(
            named(json!({})),
            Err("There is no picture to change and none to go by. Name `layer`, `layers` or `file`, or use generate_image to make a picture from words alone.".to_owned())
        );
    }

    #[test]
    fn an_edit_request_takes_the_instruction_and_the_pictures_own_shape() {
        let ready = ready(&images(), &|name| (name == "GEMINI_API_KEY").then(|| "g".to_owned()));
        let (openai, gemini) = (&ready[0], &ready[1]);
        let asked = edit_request(&json!({ "instruction": " Make it dusk ", "size": "large" }), gemini).unwrap();
        assert_eq!((asked.prompt.as_str(), asked.aspect, asked.large, asked.transparent), ("Make it dusk", None, true, false));
        assert_eq!(edit_request(&json!({ "instruction": "x", "aspect": "16:9" }), gemini).map(|r| r.aspect), Ok(Some(Aspect(16, 9))));
        assert_eq!(edit_request(&json!({}), gemini), Err("`instruction` is missing: say what to change.".to_owned()));
        assert_eq!(
            edit_request(&json!({ "instruction": "x" }), openai),
            Err("Studio is an OpenAI Images service, and editing isn't possible with that kind yet; use a Google Gemini service (get_state lists which can edit).".to_owned())
        );
    }
```

(`images()` is the test module's existing helper; its first ready service is the OpenAI one called "Studio".)

- [ ] **Step 2: Finding the pictures in a document** — in `app/pictures.rs`, plain functions with tests:

```rust
/// The file a layer's image fill shows, and the fill's place in the layer, or why it has none.
fn image_of(editor: &Editor, id: &str) -> Result<PathBuf, String>;

/// The `fill` of the layer `id` with its image paint's `url` replaced by `url`: the paint's other keys
/// (`mode`…) and the fill's other paints stay as they are.
fn fill_replaced(editor: &Editor, id: &str, url: &str) -> Result<Value, String>;
```

`image_of` resolves the paint's `url` with `crate::images::resolve(folder, url)` (the folder of the document's file) and says, for each case: no such layer; the layer has no image fill ("The layer \"title\" has no picture as its fill."); the fill names a web address or an unsaved document's relative path ("…can't be read from disk"); when the fill has several image paints, the topmost one is meant. `fill_replaced` gives a single paint object when the fill was one paint and a list when it was a list. Tests: a layer with `{"type":"image","url":"./a.png","mode":"contain"}` → same object with the new url and `"mode":"contain"`; a layer with `["#ffffff", {image…}]` → the list with only the image's url changed; a layer with a colour fill → the error; an unknown id → "There is no layer with the id …".

- [ ] **Step 3: The parked call.** `edit_image` goes the way `generate_image` goes; reuse, don't copy:

- `serve_tools` (`app/tools.rs`) parks it like `generate_image` (one condition: the tool is either).
- `start_picture` builds the order with a new `edit_order(&self, call) -> Result<(Order, u64), String>`: the assistant switch; `pick`; `document`; `imagery::named(args)`; each `Pick::Layer` → `image_of` → `Take { from: From::File(path), .. }` named by the file's name; each `Pick::File` → checked to exist → `From::File`; **`Pick::Drawn` is refused in this task** with "Drawing layers to send isn't built yet." (Task 4 replaces that line); `imagery::edit_request`.
- When the picture arrives: with `layer` given as the picture to change, the fill is replaced through `edit_in` with `fill_replaced`'s value (same hold-while-dragging, same switches check, same `layerProblem` wording as stage 1, with "changed" where it says "made"); with `file`, nothing changes and `next` tells the model to use the `url`. The answer adds `"from"`: the names sent.
- The cap of three covers both tools.

- [ ] **Step 4: What the model reads and the chat shows.**

`agent.rs` `tools()`, after `generate_image`:

```rust
        {
            "name": "edit_image",
            "description": "Have an image model change a picture that is there, by an instruction (\"make it dusk\", \"remove the background\"), or make one that goes by reference pictures (\"put this logo on that mug\"). The pictures are sent to the image service. With `layer`, that layer's picture is changed and its fill replaced (one undo step); with `layers`, those layers are drawn as they look and the result is put beside them as a new layer; with `file`, nothing in the document changes. Costs the user money and takes up to a minute, like generate_image, and the same rules hold: only when the user asks for it, one at a time, look at the result, and when it fails tell the user why and go on. Only services listed with `edits: true` in `get_state` can do this.",
            "inputSchema": { "type": "object", "required": ["instruction"], "properties": {
                "document": document,
                "instruction": { "type": "string", "description": "What to change, in plain words. Say what should stay as it is when that matters." },
                "layer": { "type": "string", "description": "The layer whose image fill is the picture to change." },
                "layers": ids("Layers to draw together, as they look on the canvas, as the picture to change. This sends a picture of the user's design to the service: only when the user asks for that."),
                "file": { "type": "string", "description": "A picture file's full path, as the picture to change." },
                "references": { "type": "array", "maxItems": 4, "items": { "type": "string" }, "description": "Up to four pictures to go by: layer ids (their picture, or how they look) or full file paths. Only what the user named." },
                "service": { "type": "string", "description": "A service's name from `imageServices` with `edits: true`. Leave out for the first that can." },
                "aspect": { "type": "string", "enum": ["1:1", "16:9", "9:16", "4:3", "3:4", "3:2", "2:3"], "description": "Width to height. Leave out to keep the shape of the picture to change." },
                "size": { "type": "string", "enum": ["normal", "large"], "description": "As in generate_image." }
            } }
        }
```

(`document` and `ids` are the helpers `tools()` already has; mind moved values.) Without `service`, the default is used when it can edit, else the first service that can; add that rule to `pick`'s caller, tested as a plain function.

`GUIDE`'s "## Pictures" section gains, in its voice: `edit_image` changes a picture that is there; use it when the user asks for a change to a picture, never to "improve" pictures unasked; `layers` and `references` send the user's own pictures or design to the service, so only what the user named. Extend the guide test.

`chat/steps.rs`: words `edit_image` → icon `"wand-sparkles"` (check it exists in the Lucide set as stage 1 checked `image-plus`; else `"image"`), "Editing a picture" / "Edited a picture" / "Couldn't edit the picture"; `detail` shows the start of `instruction` by the same cutting as `generate_image`'s prompt (share the code). Test it. Update the tool-names test.

`settings.rs`: the switch reads "Let the assistant generate and edit pictures", its second line names both tools; every sentence elsewhere that quotes the switch's name (`pictures.rs`, `tools.rs`, `AI.md` later) follows. Search for the old name: `grep -rn "Let the assistant generate pictures" src AI.md MECHANICS.md`.

`get_state`: nothing new beyond `edits` from Task 2; check the `imageServicesNote` sentence still reads right for both tools.

- [ ] **Step 5: Gate.** `bash scripts/check.sh` → `check: all passed`.

- [ ] **Step 6: Try it** (only if `GEMINI_API_KEY` is set; otherwise say it was not tried): there is no way to drive the window from here, so write one more `#[ignore]` test only if a seam exists without a window; otherwise list for the person what to try: in the app, select a rectangle with an imported photo, ask the chat "make this picture black and white"; expect "Editing a picture" then the changed picture, `credits.json` with `"edited"`, and Ctrl+Z giving the old picture back.

- [ ] **Step 7: Checkpoint.** Commit only if the user asked.

---

### Task 4: Layers as drawn

**Files:**
- Modify: `src/app/pictures.rs`

- [ ] **Step 1: Drawing layers to bytes.** Read `App::picture_tool` in `app/tools.rs` (how `screenshot` draws layers: `self.canvas.list().snapshot(&layers, scale, Vars::new(&editor.doc), ctx, folder)`, then `.encode(Format::Png, 100)`). Add to `impl App` in `pictures.rs`:

```rust
    /// The layers `ids` of the document in tab `tab`, drawn together as they look, as PNG bytes no larger than
    /// `sources::MOST_SIDE` a side, with the box they take on the canvas.
    // Agent: MAIN-THREAD (the renderer); called before the job starts.
    fn drawn(&mut self, tab: usize, ids: &[String], ctx: &egui::Context) -> Result<(Vec<u8>, Rect), String>;
```

The scale is `(MOST_SIDE / longest side of the bounds).min(2.0)`: a small design is drawn at up to twice its size so text stays readable, a large one is drawn smaller. Errors: an unknown id (the same sentence the other tools use); "The layers are empty or too large to draw, so there is nothing to send." Make sure the faces of the document's text are ready as `picture_tool`'s caller does (`faces_coming` in `serve_tools`): an `edit_image` call with `layers` or layer references waits for faces like `screenshot` does; say how you did it.

- [ ] **Step 2: Where the result goes.** A plain function, tested:

```rust
/// The layer a picture made from drawn layers becomes: a rectangle the size of `bounds`, `BESIDE_GAP` to the
/// right of it, with the picture at `url` as its fill.
fn beside(bounds: Rect, url: &str, name: &str) -> Value;
/// How far to the right of what was drawn its picture is put, in canvas units.
const BESIDE_GAP: f32 = 40.0;
```

Test: bounds `(x 100, y 50, w 300, h 200)` → `{"type":"rectangle","name":…,"x":440,"y":50,"width":300,"height":200,"fill":{"type":"image","url":…,"mode":"cover"}}`.

- [ ] **Step 3: Wire it.** In `edit_order`: `Pick::Drawn(ids)` → `self.drawn(..)` → `From::Drawn(bytes)` named `"drawn"`, remembering the bounds in `Waiting`; a `Pick::Layer` reference whose layer has no image fill is drawn too (a reference may be any layer; the picture to change by `layer` must have an image fill, and its error says to use `layers` for a layer that has none). When the picture arrives for a call whose picture to change was drawn: insert `beside(..)` at the top level through `edit_in` (one undo step, flashes, held while dragging like a fill), select nothing, and tell the model the new layer's `id`. `edit_order` now needs `&mut self` and the `ctx`; adjust `start_picture`.

- [ ] **Step 4: Gate**, then **checkpoint**.

---

### Task 5: The dialog's edit mode

**Files:**
- Modify: `src/imagery/dialog.rs`, `src/app/pictures.rs`, `src/panels.rs`, `src/theme.rs`, `src/app.rs` (if a new `Then` for the file dialog is needed)

Read `dialog.rs` and the dialog half of `pictures.rs` fully first, and the pictures in `../research_notes/imagery/generate-image-*.png` to see what the dialog looks like today.

- [ ] **Step 1: The dialog's pictures as plain state**, tested without a window:

```rust
/// A picture in the dialog's row.
pub struct Piece {
    /// The layer it is, or the file.
    pub what: PieceOf,
    /// The name under it.
    pub name: String,
    /// "Change this" is marked on it.
    pub change: bool,
    /// Its small picture, once made (`None` in tests).
    pub thumb: Option<egui::TextureHandle>,
}
pub enum PieceOf { Layer { id: String, has_picture: bool }, File(PathBuf) }

impl Dialog {
    /// Puts the selected layers into the row. A lone layer with an image fill is marked as the one to change.
    pub fn set_pieces(&mut self, pieces: Vec<Piece>);
    /// Marks piece `index` as the one to change, or unmarks it when it was; no other stays marked.
    pub fn mark(&mut self, index: usize);
    /// Adds a file as a reference, or says why not (the limit).
    pub fn add_file(&mut self, path: PathBuf) -> Result<(), String>;
    /// Takes piece `index` out of the row.
    pub fn remove(&mut self, index: usize);
    /// "Sends 3 pictures to Gemini." or `None` when nothing is sent.
    pub fn sends(&self) -> Option<String>;
    /// Whether this is an edit (a piece is marked) rather than a generation.
    pub fn editing(&self) -> bool;
}
```

Rules to test: opening with one image-fill layer selected marks it; with several selected none is marked; marking one unmarks the other; at most `MOST_REFERENCES` pieces unmarked plus one marked: `add_file` past that fails with "At most 4 reference pictures can be sent."; opening with more than five layers selected keeps the first five and says so in `problem`; `sends()` counts all pieces and names the chosen service, singular "Sends 1 picture to Gemini."; with pieces, services that can't edit are not offered, and with none that can the dialog shows "None of your image services can edit pictures yet. Add a Google Gemini service in Settings → Images." with the Open Settings button; `asked()` now gives the `edit_image` argument form when there are pieces (`instruction`, `layer` or `layers` or none, `references`, `service`, `size`, and `aspect` only when Shape isn't "Same as the picture").

- [ ] **Step 2: Drawing it.** Between the description and the Service/Shape/Size row, when there are pieces: a row of small pictures (a theme constant for the size, about 64 px, rounded like the app's other thumbnails), each with its name cut to fit, a small cross to take it out, and under it a toggle "Change this" (marked: the accent colour) or the muted word "Reference"; then an "Add file…" button the same height. Under the row, the muted `sends()` line. The title is "Edit image" while `editing()`, the field's hint "What to change: …", Shape gains "Same as the picture" as its first choice while editing (and is the choice when the dialog opens that way), and the stage 1 fill switch is replaced while editing by "Replace the picture" (shown only when the marked piece is a layer with an image fill; on by default; its second line: "When off, the result is a new layer beside it."). Thumbnails: a layer's via `App::drawn` at a small scale, a file's via the `image` crate, both made once when the piece is added, on the main thread (they are small); a piece whose thumbnail can't be made shows the picture icon.

"Add file…" uses craft-dialogs exactly as `ask_import` in `app.rs` does (a new `Then::Reference` arm), never a blocking dialog.

- [ ] **Step 3: The job.** `dialog_order` builds the order from `asked()` through the same functions the tool uses (`imagery::named`, `edit_request`, the takes from `image_of` / `drawn`), so the dialog and the tool cannot drift apart; factor what `edit_order` does into a function both call, taking where the pictures come from and the document's tab. Placing the result: marked layer with an image fill and "Replace the picture" on → `fill_replaced` as one undo step (a `fill_with`-like helper that takes the fill value); off, or a marked layer that was drawn → `beside`; no marked piece → stage 1's placement. The stage 1 guards stay: shortcuts blocked, Escape and backdrop don't cancel while busy, the fallback when the layer is gone, bringing the tab forward.

- [ ] **Step 4: "Edit with AI…" in the properties panel.** On an image paint's row in `panels.rs` (the `"image" => "Image"` row), a small icon button (the icon chosen in Task 3) with the tooltip "Edit with AI…"; it opens the dialog with that layer as the marked piece. Follow how the panel already hands actions up to the app (find how `PaintEdit` or the panel's other buttons reach `App`); add the smallest new variant.

- [ ] **Step 5: Look at it.** Extend the `--generate-image` pose so a picture can show the edit mode: with `--select <id>` naming a layer that has an image fill, the pose opens the dialog on it. The starter board has no picture: for the pose, make a temporary `.pen` in your scratchpad with one rectangle whose fill is a small local image, and pass it on the command line. Take light and dark pictures with `GEMINI_API_KEY=dummy` set for that command only (nothing is sent unless the button is pressed), into `../research_notes/imagery/edit-image-*.png`; open and look: row, toggles, the line about what is sent, nothing overlapping; also one of the properties panel showing the new button. Fix what looks wrong; say what you saw.

- [ ] **Step 6: Gate**, then **checkpoint**.

---

### Task 6: The quick field on an image fill

**Files:**
- Modify: `src/panels.rs`, `src/app/pictures.rs`, `src/theme.rs`

- [ ] **Step 1: State and rules**, as plain functions with tests, in `pictures.rs`:

```rust
/// The quick field's state for the layer whose image paint it sits under.
pub(super) struct Quick {
    /// The layer it was typed for.
    pub layer: String,
    /// The words.
    pub text: String,
    /// What went wrong the last time.
    pub problem: Option<String>,
}
```

Rules: one `Quick` per app, tied to a layer id and the document's id; selecting another layer clears the words; while any person's picture is being made (the dialog's or the field's: stage 1's single `Generating` job) the field is disabled, and if this field started it, it shows progress and "Cancel" (dropping the job, as the dialog's Cancel does); Return or the button starts it when the words aren't blank; the order is what the dialog would build for that layer marked, no references, Shape "Same as the picture", normal size, the default service or the first that can edit; the result replaces the fill (one undo step); a failure is kept in `problem` and the words stay; with no service that can edit, the field is replaced by the muted line "Add a Google Gemini service in Settings → Images to change pictures by description."

- [ ] **Step 2: Draw it** under the image paint's row, in the panel's own style (its field helper, `PROPS_ROW`, `PROPS_TEXT`): hint "Describe a change…", the button with the Task 3 icon, tooltip "Sends this picture to <service> and replaces it with the result." The problem line under it in the problem colour, wrapped.

- [ ] **Step 3: Look at it** with the pose from Task 5 Step 5 (`--select` the picture layer, no `--generate-image`): light and dark pictures into `../research_notes/imagery/quick-edit-*.png`; look; fix.

- [ ] **Step 4: Gate**, then **checkpoint**.

---

### Task 7: Documentation and the last check

**Files:**
- Modify: `AI.md`, `MECHANICS.md`, `README.md` (only if a pose flag changed)

- [ ] **Step 1:** Read the code as it now is, then `AI.md` and `MECHANICS.md` whole. `AI.md`: the `edit_image` row in the tools table; in "Pictures": changing a picture three ways (the assistant, the dialog's edit mode, the field on an image fill), what counts as the picture to change and as a reference, where the result goes, that only Gemini services edit for now; in "Safety": that an edit sends the pictures named, and with "as drawn" a picture of part of the design, to the service, on the person's action or through the tool while the switch (by its new name) is on. `MECHANICS.md`: the numbers, each checked against its constant and named in your report: `MOST_SIDE` 2048, `MOST_REFERENCES` 4, `BESIDE_GAP` 40, the thumbnail size, the drawing scale's cap of 2.
- [ ] **Step 2:** `bash scripts/check.sh` → `check: all passed`.
- [ ] **Step 3: Report:** what was verified against the live service and what only against notes from Task 1; which pictures were looked at; what needs a person at the window (a real edit landing, undo, Cancel, the quick field while busy, "Add file…"); everything in this plan that had to change, and why. Commit only if the user asked.
