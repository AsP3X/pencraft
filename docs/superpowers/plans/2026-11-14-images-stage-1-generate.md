# Pictures, stage 1: Generate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pencraft can make a picture from a description with an image model service the user has added, both through a new `generate_image` tool for AI models and through a "Generate Image…" dialog in the editor.

**Architecture:** A new `imagery` module holds everything that is not egui: the request and failure types, one file per kind of service (Gemini, OpenAI) that turns a request into an HTTP body and an answer into bytes, a store that writes the picture beside the document with a credits file, and a job that runs all of it on a worker thread. The app parks a `generate_image` tool call until its job ends, exactly as it already parks calls while the user drags. Settings get an Images page; the dialog reuses the same job.

**Tech Stack:** Rust 2024, egui 0.37 (storytold fork), `ureq` 3.5 (blocking), `image` 0.25, `serde_json`, `keyring` through `secrets::Vault`, `craft-atomic`. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-11-14-images-design.md`

---

## Read this first

All paths are relative to `app/` (the Cargo workspace root). Run every command from `app/`.

**House rules (`app/AGENTS.md`), which the lints enforce:**

- No `unwrap`/`expect`/`panic!` outside `#[cfg(test)]`. Return `Result<_, String>` with a sentence a person can read.
- No `#[allow]`. `dead_code` is **denied** and may not be silenced.
- `///` on every public item, and on private ones where the neighbours have them. Non-trivial code gets a `// Human:` comment (what and why, plain words) and, where there is a trap, an `// Agent:` comment.
- Colours and sizes of the interface come from `src/theme.rs` only.
- egui is a fork: use `egui::Id::unique(..)` and `ui.scope_id()`, never `Id::new` or `Ui::id`. When an egui call in this plan does not compile, copy the form the nearest existing code uses (`src/settings.rs`, `src/find.rs`).
- **Nothing is committed unless the user has asked for commits.** Each task ends with a "Checkpoint" step: run the named tests; commit only if the user said so.

**The dead-code trap.** Code added in Tasks 1 to 7 is used only by its own tests until Task 8 wires it into the app. `cargo test` passes during that time; `cargo clippy` and a plain `cargo build` do **not** (unused code is an error). So Tasks 1 to 7 are verified with `cargo test -p pencraft <filter>` only, and `scripts/check.sh` must pass from Task 8 on. Do not "fix" the lint in between.

**Two deviations from the spec, on purpose:**

1. The spec names a `Service` trait. This plan uses an enum `Kind` with a `make` function per kind and passes the work to the job as a closure, so tests can fake a service without a trait object. A further kind is one more enum variant and one more file.
2. `Request` has no "source pictures" field yet. Stage 2 adds it; an unused field would be dead code today.

3. The spec asks for tests of the whole tool with a fake service. `App` can't be built in a test (it needs an eframe window), so the plan tests every piece the tool is made of (`request`, `pick`, `Job`, `takes_picture`, `fill_with`, the store) and checks the parked call itself by hand in Task 8, Step 8.

**Facts checked while planning:**

- craft-settings keeps unknown keys in `config.toml`, so removing `generation.image_provider` needs no migration.
- `link.rs` waits 120 s for a tool's answer (`PATIENCE`), the chat's own engine 130 s (`TOOL_PATIENCE` in `chat/api.rs`). A job's limit is 100 s.
- The API shapes below are from the providers' documentation as read on 2026-11-14. Error and refusal shapes were not documented; Tasks 3 and 4 start by capturing real answers when a key is at hand.

## Files

| File | Responsibility |
|---|---|
| `src/agent.rs` (modify) | `unbase64`; the `generate_image` tool's schema; the guide's "Pictures" section |
| `src/imagery.rs` (create) | Module root: `Kind`, `Ready`, `KeyFrom`, which services work, picking one, reading a tool call into a `Request` |
| `src/imagery/service.rs` (create) | `Request`, `Aspect`, `Made`, `Failure`, the shared HTTP `post` |
| `src/imagery/gemini.rs` (create) | Gemini: body, answer, `make` |
| `src/imagery/openai.rs` (create) | OpenAI Images: body, answer, `make` |
| `src/imagery/store.rs` (create) | Writing a picture and its credit; the small copy a model is shown |
| `src/imagery/jobs.rs` (create) | A request on a worker thread with a time limit |
| `src/imagery/dialog.rs` (create) | The "Generate Image…" window |
| `src/prefs.rs` (modify) | `[images]` section; `generation.image_provider` removed |
| `src/app/pictures.rs` (create) | The app's side: parked tool calls, the dialog's job, placing the result |
| `src/app/tools.rs`, `src/app.rs` (modify) | Dispatch, state, fields, the command |
| `src/chat/steps.rs` (modify) | The chat's words for the tool |
| `src/settings.rs` (modify) | Images page; the placeholder row removed |
| `src/menus.rs`, `src/palette.rs` (modify) | The menu item and the action |
| `AI.md`, `MECHANICS.md` (modify) | Docs |

---

### Task 1: Reading base64

**Files:**
- Modify: `src/agent.rs` (after `pub fn base64`, about line 657; tests at the end of `mod tests`)

- [ ] **Step 1: Write the failing test**

Add inside `mod tests` in `src/agent.rs`:

```rust
    #[test]
    fn unbase64_reads_what_base64_writes_and_refuses_what_is_not() {
        for bytes in [&b""[..], b"f", b"fo", b"foo", b"foob", &[0, 255, 16, 128, 7]] {
            assert_eq!(unbase64(&base64(bytes)).as_deref(), Some(bytes));
        }
        // Line breaks and missing padding are fine; providers send both.
        assert_eq!(unbase64("Zm9v\nYmFy").as_deref(), Some(&b"foobar"[..]));
        assert_eq!(unbase64("Zm8").as_deref(), Some(&b"fo"[..]));
        assert_eq!(unbase64("Zm9v!"), None);
        assert_eq!(unbase64("Z"), None);
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p pencraft unbase64`
Expected: does not compile, `cannot find function unbase64`.

- [ ] **Step 3: Write the function**

Add after `pub fn base64` in `src/agent.rs`:

```rust
/// The bytes base64 text stands for, or `None` when it isn't base64. White space and missing `=` are fine.
#[must_use]
pub fn unbase64(text: &str) -> Option<Vec<u8>> {
    let value = |letter: u8| match letter {
        b'A'..=b'Z' => Some(u32::from(letter - b'A')),
        b'a'..=b'z' => Some(u32::from(letter - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(letter - b'0') + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    };
    let letters: Vec<u32> =
        text.bytes().filter(|letter| !letter.is_ascii_whitespace() && *letter != b'=').map(value).collect::<Option<_>>()?;
    let mut out = Vec::with_capacity(letters.len() / 4 * 3 + 2);
    for group in letters.chunks(4) {
        // Human: four letters carry three bytes; a last group of three carries two, of two carries one, and a
        // single letter carries nothing whole, so it is refused.
        if group.len() == 1 {
            return None;
        }
        let bits = group.iter().fold(0_u32, |bits, letter| (bits << 6) | letter) << (6 * (4 - group.len()));
        let bytes = bits.to_be_bytes();
        out.extend_from_slice(&bytes[1..group.len()]);
    }
    Some(out)
}
```

- [ ] **Step 4: Run the test**

Run: `cargo test -p pencraft unbase64`
Expected: 1 passed.

- [ ] **Step 5: Checkpoint**

Run: `cargo test -p pencraft base64` — expected: 2 passed. Commit only if the user asked for commits (`git add app/src/agent.rs && git commit -m "Read base64"`).

---

### Task 2: The request, the failure, and the HTTP call

**Files:**
- Create: `src/imagery.rs`, `src/imagery/service.rs`
- Modify: `src/main.rs` (the `mod` list, alphabetical: after `mod icons;`)

- [ ] **Step 1: Declare the module**

In `src/main.rs` add between `mod icons;` and `mod images;`:

```rust
mod imagery;
```

Create `src/imagery.rs`:

```rust
//! Pictures made by an image model service: which services there are, asking one for a picture, and keeping
//! the result as a file beside the document.
//!
//! Nothing here draws. [`service`] is what a request and a failure are; [`gemini`] and [`openai`] each speak
//! one provider's API; [`store`] writes the picture and where it came from; [`jobs`] does all of that on a
//! thread of its own, because a picture takes up to a minute.

pub mod service;
```

- [ ] **Step 2: Write the failing tests**

Create `src/imagery/service.rs` with only the tests for now:

```rust
//! What is asked of an image service and what can go wrong, the same for every provider.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_aspect_is_one_of_the_known_ones() {
        assert_eq!(Aspect::parse("16:9"), Some(Aspect(16, 9)));
        assert_eq!(Aspect::parse(" 1:1 "), Some(Aspect(1, 1)));
        assert_eq!(Aspect::parse("7:5"), None);
        assert_eq!(Aspect::parse("wide"), None);
        assert_eq!(Aspect(3, 4).text(), "3:4");
        assert_eq!(Aspect::default(), Aspect(1, 1));
    }

    #[test]
    fn a_status_becomes_the_failure_a_person_can_act_on() {
        let said = r#"{"error":{"message":"Bad key."}}"#;
        assert_eq!(Failure::of(401, said), Failure::Key("Bad key.".to_owned()));
        assert_eq!(Failure::of(403, said), Failure::Key("Bad key.".to_owned()));
        assert_eq!(Failure::of(429, said), Failure::Limit("Bad key.".to_owned()));
        assert_eq!(Failure::of(500, "<html>oops</html>"), Failure::Other("500: <html>oops</html>".to_owned()));
    }

    #[test]
    fn a_failure_names_the_service_and_what_to_do() {
        assert_eq!(
            Failure::Key("Bad key.".to_owned()).sentence("My Gemini"),
            "My Gemini didn't accept its API key: Bad key. Check the key in Settings → Images."
        );
        assert_eq!(Failure::Refused("No people.".to_owned()).sentence("My Gemini"), "My Gemini refused the description: No people.");
        assert_eq!(Failure::Limit("Slow down.".to_owned()).sentence("X"), "X has reached a limit: Slow down.");
        assert_eq!(Failure::Network("timed out".to_owned()).sentence("X"), "X couldn't be reached or took too long: timed out");
        assert_eq!(Failure::Other("500: oops".to_owned()).sentence("X"), "X answered with an error: 500: oops");
    }
}
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test -p pencraft imagery::service`
Expected: does not compile (`Aspect`, `Failure` not found).

- [ ] **Step 4: Write the types and the call**

Put above the tests in `src/imagery/service.rs`:

```rust
use std::time::Duration;

use serde_json::Value;

/// The aspect ratios a picture can be asked for in, as width to height.
pub const ASPECTS: [&str; 7] = ["1:1", "16:9", "9:16", "4:3", "3:4", "3:2", "2:3"];
/// How long a service may take to answer; a little under a job's own limit ([`super::jobs::LIMIT`]).
const ANSWER_TIME: Duration = Duration::from_secs(95);
/// The largest answer taken, in bytes: a large picture as base64 is well over ureq's own 10 MB limit.
const MOST_ANSWER: u64 = 64 * 1024 * 1024;
/// How much of an answer that isn't JSON is shown.
const MOST_SAID: usize = 300;

/// A picture's shape: width to height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aspect(pub u32, pub u32);

impl Default for Aspect {
    fn default() -> Self {
        Aspect(1, 1)
    }
}

impl Aspect {
    /// The aspect `text` names ("16:9"), when it is one of [`ASPECTS`].
    #[must_use]
    pub fn parse(text: &str) -> Option<Aspect> {
        let text = text.trim();
        let (width, height) = ASPECTS.contains(&text).then(|| text.split_once(':')).flatten()?;
        Some(Aspect(width.parse().ok()?, height.parse().ok()?))
    }

    /// "16:9".
    #[must_use]
    pub fn text(self) -> String {
        format!("{}:{}", self.0, self.1)
    }
}

/// One picture to make.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// What the picture shows.
    pub prompt: String,
    /// The model, as the service names it.
    pub model: String,
    /// Its shape.
    pub aspect: Aspect,
    /// About 2048 pixels on the long side instead of about 1024.
    pub large: bool,
    /// A transparent background.
    pub transparent: bool,
}

/// A picture a service made: the bytes of a PNG, JPEG or WebP file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Made(pub Vec<u8>);

/// Why a service made no picture. Each holds the service's own words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The service won't make this picture.
    Refused(String),
    /// The key is missing or wasn't accepted.
    Key(String),
    /// Too many requests, or the account's quota is used up.
    Limit(String),
    /// The service wasn't reached or didn't answer in time.
    Network(String),
    /// Anything else.
    Other(String),
}

impl Failure {
    /// The failure an answer with `status` and the body `said` is.
    #[must_use]
    pub fn of(status: u16, said: &str) -> Failure {
        let message = message_of(said);
        match status {
            401 | 403 => Failure::Key(message),
            429 => Failure::Limit(message),
            _ => Failure::Other(format!("{status}: {message}")),
        }
    }

    /// The failure as a sentence for a person or a model, naming the service.
    #[must_use]
    pub fn sentence(&self, service: &str) -> String {
        match self {
            Failure::Refused(why) => format!("{service} refused the description: {why}"),
            Failure::Key(why) => format!("{service} didn't accept its API key: {why} Check the key in Settings → Images."),
            Failure::Limit(why) => format!("{service} has reached a limit: {why}"),
            Failure::Network(why) => format!("{service} couldn't be reached or took too long: {why}"),
            Failure::Other(why) => format!("{service} answered with an error: {why}"),
        }
    }
}

/// What a service said went wrong: the `error.message` of a JSON answer, or the start of anything else.
#[must_use]
pub fn message_of(said: &str) -> String {
    serde_json::from_str::<Value>(said)
        .ok()
        .and_then(|value| value.pointer("/error/message").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_else(|| said.chars().take(MOST_SAID).collect())
}

/// Sends `body` as JSON to `url` and gives the answer's status and text.
///
/// # Errors
/// [`Failure::Network`] when the service wasn't reached, didn't answer in time, or its answer couldn't be read.
// Agent: BLOCKS for up to ANSWER_TIME; call it on a worker thread only. `headers` carry the key: never log them.
pub fn post(url: &str, headers: &[(&str, &str)], body: &Value) -> Result<(u16, String), Failure> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_recv_response(Some(ANSWER_TIME))
        .build()
        .into();
    let mut request = agent.post(url).header("content-type", "application/json");
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let mut response = request.send(body.to_string().as_str()).map_err(|error| Failure::Network(error.to_string()))?;
    let status = response.status().as_u16();
    let said = response.body_mut().with_config().limit(MOST_ANSWER).read_to_string().map_err(|error| Failure::Network(error.to_string()))?;
    Ok((status, said))
}
```

If `with_config().limit(..)` does not exist under that name in ureq 3.5, open `~/.cargo/registry/src/*/ureq-3.5.0/src/body/mod.rs` and use what it offers for a larger read limit; do not fall back to the plain `read_to_string()` (10 MB), which a large picture exceeds.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p pencraft imagery::service`
Expected: 3 passed.

- [ ] **Step 6: Checkpoint**

Commit only if the user asked (`git add app/src/main.rs app/src/imagery.rs app/src/imagery/service.rs`).

---

### Task 3: Gemini

**Files:**
- Create: `src/imagery/gemini.rs`
- Modify: `src/imagery.rs` (add `pub mod gemini;` above `pub mod service;`)

- [ ] **Step 1: Capture real answers, if a key is at hand**

If `GEMINI_API_KEY` is set in the shell, run (bash):

```bash
mkdir -p ../research_notes/imagery && curl -sS -o ../research_notes/imagery/gemini-ok.json -w "%{http_code}\n" https://generativelanguage.googleapis.com/v1beta/interactions -H "x-goog-api-key: $GEMINI_API_KEY" -H "content-type: application/json" -d '{"model":"gemini-3.1-flash-lite-image","input":[{"type":"text","text":"A red apple on a white table"}],"response_format":{"type":"image","mime_type":"image/png","aspect_ratio":"1:1","image_size":"1K"}}'
```

```bash
curl -sS -o ../research_notes/imagery/gemini-badkey.json -w "%{http_code}\n" https://generativelanguage.googleapis.com/v1beta/interactions -H "x-goog-api-key: wrong" -H "content-type: application/json" -d '{"model":"gemini-3.1-flash-lite-image","input":[{"type":"text","text":"A red apple"}]}'
```

Look at both files (the first is large: read only its keys, e.g. `python -c "import json,sys;d=json.load(open(sys.argv[1]));print(json.dumps(d,indent=1)[:1500])" ../research_notes/imagery/gemini-ok.json`). Compare with the fixtures in Step 2: the path to the picture (`steps[]` with `type == "model_output"`, then `content[]` with `type == "image"`, then `data`) and the error's `error.message`. **If the real shape differs, change the fixtures and `read` below to the real shape and say so in your report.** If no key is set, go on with the documented shapes and say in your report that Gemini is unverified against the live service.

- [ ] **Step 2: Write the failing tests**

Create `src/imagery/gemini.rs` with only:

```rust
//! Google's Gemini image models ("Nano Banana"), through the Interactions API.

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::imagery::service::Aspect;

    fn request() -> Request {
        Request { prompt: "A red fox".to_owned(), model: "gemini-3.1-flash-lite-image".to_owned(), aspect: Aspect(16, 9), large: false, transparent: false }
    }

    #[test]
    fn the_body_names_the_model_the_words_and_the_shape() {
        assert_eq!(
            body(&request()),
            json!({
                "model": "gemini-3.1-flash-lite-image",
                "input": [{ "type": "text", "text": "A red fox" }],
                "response_format": { "type": "image", "mime_type": "image/png", "aspect_ratio": "16:9", "image_size": "1K" }
            })
        );
        assert_eq!(body(&Request { large: true, ..request() })["response_format"]["image_size"], "2K");
    }

    #[test]
    fn the_picture_is_read_out_of_the_models_output() {
        let said = json!({ "id": "i1", "steps": [
            { "type": "thought", "content": [{ "type": "text", "text": "thinking" }] },
            { "type": "model_output", "content": [{ "type": "text", "text": "Here you go." }, { "type": "image", "data": "Zm9v" }] }
        ] })
        .to_string();
        assert_eq!(read(200, &said), Ok(Made(b"foo".to_vec())));
    }

    #[test]
    fn an_answer_without_a_picture_is_a_refusal_in_the_models_words() {
        let said = json!({ "steps": [{ "type": "model_output", "content": [{ "type": "text", "text": "I can't draw that." }] }] }).to_string();
        assert_eq!(read(200, &said), Err(Failure::Refused("I can't draw that.".to_owned())));
        assert_eq!(read(200, "{}"), Err(Failure::Refused("It sent no picture and no reason.".to_owned())));
    }

    #[test]
    fn errors_and_nonsense_are_failures() {
        assert_eq!(read(403, r#"{"error":{"message":"API key not valid."}}"#), Err(Failure::Key("API key not valid.".to_owned())));
        assert_eq!(read(200, "not json"), Err(Failure::Other("The answer couldn't be read: not json".to_owned())));
        let broken = json!({ "steps": [{ "type": "model_output", "content": [{ "type": "image", "data": "!!!" }] }] }).to_string();
        assert_eq!(read(200, &broken), Err(Failure::Other("The picture in the answer couldn't be read.".to_owned())));
    }

    #[test]
    fn the_address_takes_a_base_with_or_without_a_slash() {
        assert_eq!(endpoint("https://generativelanguage.googleapis.com/"), "https://generativelanguage.googleapis.com/v1beta/interactions");
        assert_eq!(endpoint(BASE), "https://generativelanguage.googleapis.com/v1beta/interactions");
    }
}
```

Add `pub mod gemini;` to `src/imagery.rs`.

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test -p pencraft imagery::gemini`
Expected: does not compile (`body`, `read`, `endpoint`, `BASE` not found).

- [ ] **Step 4: Write the service**

Put above the tests in `src/imagery/gemini.rs`:

```rust
use craft_ai_core::SecretString;
use serde_json::{Value, json};

use super::service::{Failure, Made, Request, post};

/// Where the service is reached unless the user gave another address.
pub const BASE: &str = "https://generativelanguage.googleapis.com";
/// The model a new service starts with: the fastest and cheapest ("Nano Banana 2 Lite").
pub const MODEL: &str = "gemini-3.1-flash-lite-image";

/// The address requests go to.
fn endpoint(base: &str) -> String {
    format!("{}/v1beta/interactions", base.trim().trim_end_matches('/'))
}

/// The request as the Interactions API takes it.
fn body(request: &Request) -> Value {
    json!({
        "model": request.model,
        "input": [{ "type": "text", "text": request.prompt }],
        "response_format": {
            "type": "image",
            "mime_type": "image/png",
            "aspect_ratio": request.aspect.text(),
            "image_size": if request.large { "2K" } else { "1K" },
        }
    })
}

/// The picture in an answer with `status` and the body `said`.
// Human: the answer is a list of steps; the model's output is the step of type "model_output", and the
// picture is the block of type "image" in it, as base64. An answer that came through but holds no picture is
// the model declining, and what it wrote instead is the reason.
fn read(status: u16, said: &str) -> Result<Made, Failure> {
    if !(200..300).contains(&status) {
        return Err(Failure::of(status, said));
    }
    let answer: Value =
        serde_json::from_str(said).map_err(|_| Failure::Other(format!("The answer couldn't be read: {}", said.chars().take(200).collect::<String>())))?;
    let blocks: Vec<&Value> = answer["steps"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|step| step["type"] == "model_output")
        .flat_map(|step| step["content"].as_array().into_iter().flatten())
        .collect();
    if let Some(data) = blocks.iter().find(|block| block["type"] == "image").and_then(|block| block["data"].as_str()) {
        return crate::agent::unbase64(data).filter(|bytes| !bytes.is_empty()).map(Made).ok_or_else(|| Failure::Other("The picture in the answer couldn't be read.".to_owned()));
    }
    let words: Vec<&str> = blocks.iter().filter_map(|block| block["text"].as_str()).collect();
    Err(Failure::Refused(if words.is_empty() { "It sent no picture and no reason.".to_owned() } else { words.join(" ") }))
}

/// Asks the service at `base` for the picture `request` describes.
///
/// # Errors
/// Why there is no picture.
// Agent: BLOCKS for one answer (up to 95 s).
pub fn make(base: &str, request: &Request, key: &SecretString) -> Result<Made, Failure> {
    let (status, said) = post(&endpoint(base), &[("x-goog-api-key", key.expose())], &body(request))?;
    read(status, &said)
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p pencraft imagery::gemini`
Expected: 5 passed.

- [ ] **Step 6: Checkpoint**

Commit only if the user asked (`git add app/src/imagery.rs app/src/imagery/gemini.rs`).

---

### Task 4: OpenAI Images

**Files:**
- Create: `src/imagery/openai.rs`
- Modify: `src/imagery.rs` (add `pub mod openai;` after `pub mod gemini;`)

- [ ] **Step 1: Capture real answers, if a key is at hand**

If `OPENAI_API_KEY` is set (bash):

```bash
mkdir -p ../research_notes/imagery && curl -sS -o ../research_notes/imagery/openai-ok.json -w "%{http_code}\n" https://api.openai.com/v1/images/generations -H "Authorization: Bearer $OPENAI_API_KEY" -H "content-type: application/json" -d '{"model":"gpt-image-2.5-sunburst","prompt":"A red apple on a white table","size":"1024x576","quality":"auto","background":"auto","output_format":"png","n":1}'
```

```bash
curl -sS -o ../research_notes/imagery/openai-badkey.json -w "%{http_code}\n" https://api.openai.com/v1/images/generations -H "Authorization: Bearer wrong" -H "content-type: application/json" -d '{"model":"gpt-image-2.5-sunburst","prompt":"A red apple"}'
```

Check: the picture is at `data[0].b64_json`; an error has `error.message` and `error.code`; and the service accepted the size `1024x576`. **If `1024x576` is refused as too small, the smallest accepted size decides: change `size` in Step 4 so the short side is never under what the error names, change the test to match, and say so in your report.** Without a key, go on and report OpenAI as unverified against the live service.

- [ ] **Step 2: Write the failing tests**

Create `src/imagery/openai.rs` with only:

```rust
//! OpenAI's image models, and every service that speaks the same Images API.

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::imagery::service::Aspect;

    fn request() -> Request {
        Request { prompt: "A red fox".to_owned(), model: "gpt-image-2.5-sunburst".to_owned(), aspect: Aspect(16, 9), large: false, transparent: false }
    }

    #[test]
    fn the_size_has_the_long_side_asked_for_and_edges_in_sixteens() {
        assert_eq!(size(Aspect(1, 1), false), "1024x1024");
        assert_eq!(size(Aspect(16, 9), false), "1024x576");
        assert_eq!(size(Aspect(9, 16), false), "576x1024");
        assert_eq!(size(Aspect(3, 2), false), "1024x688");
        assert_eq!(size(Aspect(4, 3), true), "2048x1536");
    }

    #[test]
    fn the_body_names_the_model_the_words_the_size_and_the_background() {
        assert_eq!(
            body(&request()),
            json!({ "model": "gpt-image-2.5-sunburst", "prompt": "A red fox", "size": "1024x576", "quality": "auto", "background": "auto", "output_format": "png", "n": 1 })
        );
        assert_eq!(body(&Request { transparent: true, ..request() })["background"], "transparent");
    }

    #[test]
    fn the_picture_is_the_first_of_the_data() {
        assert_eq!(read(200, &json!({ "created": 1, "data": [{ "b64_json": "Zm9v" }] }).to_string()), Ok(Made(b"foo".to_vec())));
    }

    #[test]
    fn a_blocked_description_is_a_refusal_and_other_errors_keep_their_kind() {
        let blocked = json!({ "error": { "message": "Your request was rejected.", "code": "moderation_blocked", "type": "image_generation_user_error" } }).to_string();
        assert_eq!(read(400, &blocked), Err(Failure::Refused("Your request was rejected.".to_owned())));
        assert_eq!(read(401, r#"{"error":{"message":"Incorrect API key."}}"#), Err(Failure::Key("Incorrect API key.".to_owned())));
        assert_eq!(read(200, r#"{"data":[]}"#), Err(Failure::Other("The answer holds no picture.".to_owned())));
        assert_eq!(read(200, r#"{"data":[{"b64_json":"!!!"}]}"#), Err(Failure::Other("The picture in the answer couldn't be read.".to_owned())));
    }

    #[test]
    fn the_address_takes_a_base_with_or_without_the_version() {
        assert_eq!(endpoint(BASE), "https://api.openai.com/v1/images/generations");
        assert_eq!(endpoint("https://api.openai.com/v1/"), "https://api.openai.com/v1/images/generations");
        assert_eq!(endpoint("http://localhost:8080"), "http://localhost:8080/v1/images/generations");
    }
}
```

Add `pub mod openai;` to `src/imagery.rs`.

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test -p pencraft imagery::openai`
Expected: does not compile.

- [ ] **Step 4: Write the service**

Put above the tests in `src/imagery/openai.rs`:

```rust
use craft_ai_core::SecretString;
use serde_json::{Value, json};

use super::service::{Aspect, Failure, Made, Request, post};

/// Where the service is reached unless the user gave another address.
pub const BASE: &str = "https://api.openai.com";
/// The model a new service starts with.
pub const MODEL: &str = "gpt-image-2.5-sunburst";
/// The long side of a picture in pixels: normal and large.
const SIDES: (u32, u32) = (1024, 2048);
/// Both edges are a multiple of this, as the API asks.
const STEP: u32 = 16;

/// The address requests go to. A base may end in `/v1` or not.
fn endpoint(base: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    let base = base.strip_suffix("/v1").unwrap_or(base);
    format!("{base}/v1/images/generations")
}

/// "1024x576": the long side as asked, the short one from the aspect, rounded to the API's step.
fn size(aspect: Aspect, large: bool) -> String {
    let long = if large { SIDES.1 } else { SIDES.0 };
    let (wide, tall) = (aspect.0.max(1), aspect.1.max(1));
    let short = |less: u32, more: u32| ((long * less + more * STEP / 2) / more / STEP * STEP).max(STEP);
    if wide >= tall { format!("{long}x{}", short(tall, wide)) } else { format!("{}x{long}", short(wide, tall)) }
}

/// The request as the Images API takes it.
fn body(request: &Request) -> Value {
    json!({
        "model": request.model,
        "prompt": request.prompt,
        "size": size(request.aspect, request.large),
        "quality": "auto",
        "background": if request.transparent { "transparent" } else { "auto" },
        "output_format": "png",
        "n": 1,
    })
}

/// The picture in an answer with `status` and the body `said`.
fn read(status: u16, said: &str) -> Result<Made, Failure> {
    if !(200..300).contains(&status) {
        let answer: Value = serde_json::from_str(said).unwrap_or(Value::Null);
        // Human: the service says a description broke its rules with this code; that is a refusal the person
        // can act on by wording it differently, not a fault.
        if answer.pointer("/error/code").and_then(Value::as_str) == Some("moderation_blocked") {
            return Err(Failure::Refused(super::service::message_of(said)));
        }
        return Err(Failure::of(status, said));
    }
    let answer: Value =
        serde_json::from_str(said).map_err(|_| Failure::Other(format!("The answer couldn't be read: {}", said.chars().take(200).collect::<String>())))?;
    let data = answer.pointer("/data/0/b64_json").and_then(Value::as_str).ok_or_else(|| Failure::Other("The answer holds no picture.".to_owned()))?;
    crate::agent::unbase64(data).filter(|bytes| !bytes.is_empty()).map(Made).ok_or_else(|| Failure::Other("The picture in the answer couldn't be read.".to_owned()))
}

/// Asks the service at `base` for the picture `request` describes.
///
/// # Errors
/// Why there is no picture.
// Agent: BLOCKS for one answer (up to 95 s).
pub fn make(base: &str, request: &Request, key: &SecretString) -> Result<Made, Failure> {
    let bearer = format!("Bearer {}", key.expose());
    let (status, said) = post(&endpoint(base), &[("authorization", bearer.as_str())], &body(request))?;
    read(status, &said)
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p pencraft imagery::openai`
Expected: 5 passed. (If `size(Aspect(3, 2), false)` gives `1024x680` instead of `1024x688`, the rounding is wrong: 1024 × 2 / 3 = 682.67, and the nearest multiple of 16 is 688.)

- [ ] **Step 6: Checkpoint**

Commit only if the user asked (`git add app/src/imagery.rs app/src/imagery/openai.rs`).

---

### Task 5: Writing the picture and its credit

**Files:**
- Create: `src/imagery/store.rs`
- Modify: `src/imagery.rs` (add `pub mod store;` after `pub mod service;`)

- [ ] **Step 1: Write the failing tests**

Create `src/imagery/store.rs` with only:

```rust
//! Keeping a made picture: a file in `images/` beside the document, and a line in `images/credits.json`
//! saying where it came from.

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(width, height, image::Rgba([200, 30, 30, 255])).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        bytes.into_inner()
    }

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("pencraft-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_name_is_made_of_the_first_plain_words() {
        assert_eq!(stem("A red fox, in SNOW!"), "a-red-fox-in-snow");
        assert_eq!(stem("  ¿¡ "), "picture");
        assert_eq!(stem(&"word ".repeat(30)).len(), 39);
    }

    #[test]
    fn a_picture_lands_beside_the_document_with_its_credit_and_never_replaces_another() {
        let folder = folder("beside");
        let document = folder.join("site.pen");
        let credit = json!({ "made": "generated", "service": "My Gemini", "model": "m", "prompt": "A red fox" });
        let first = save(Some(&document), None, &png(4, 2), "A red fox", &credit).unwrap();
        assert_eq!((first.width, first.height), (4, 2));
        assert!(first.url.starts_with("./images/a-red-fox-") && first.url.ends_with(".png"), "{}", first.url);
        assert_eq!(first.path, folder.join("images").join(first.url.trim_start_matches("./images/")));
        assert!(first.path.is_file());
        let second = save(Some(&document), None, &png(4, 2), "A red fox", &credit).unwrap();
        assert_ne!(first.path, second.path);
        let credits: Value = serde_json::from_str(&std::fs::read_to_string(folder.join("images").join(CREDITS)).unwrap()).unwrap();
        let names: Vec<&String> = credits.as_object().unwrap().keys().collect();
        assert_eq!(names.len(), 2);
        assert_eq!(credits[first.path.file_name().unwrap().to_str().unwrap()], credit);
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_document_without_a_file_gets_a_full_path_under_the_home_folder() {
        let home = folder("home");
        let saved = save(None, Some(&home), &png(1, 1), "x", &json!({})).unwrap();
        assert_eq!(saved.path.parent().unwrap(), home.join(".pencraft").join("images"));
        assert_eq!(saved.url, saved.path.to_string_lossy().replace('\\', "/"));
        assert!(Path::new(&saved.url).is_absolute());
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn what_is_not_a_picture_is_refused_and_nothing_is_written() {
        let folder = folder("refused");
        let document = folder.join("site.pen");
        assert_eq!(save(Some(&document), None, b"<html>", "x", &json!({})).unwrap_err(), "The service sent something that isn't a picture.");
        assert!(!folder.join("images").exists());
        assert_eq!(save(None, None, &png(1, 1), "x", &json!({})).unwrap_err(), "There is no folder to keep the picture in: save the document first.");
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_copy_for_a_model_is_a_png_no_larger_than_asked() {
        let small = shown(&png(40, 20), 10).unwrap();
        let picture = image::load_from_memory(&small).unwrap();
        assert_eq!((picture.width(), picture.height()), (10, 5));
        assert_eq!(image::guess_format(&small).unwrap(), image::ImageFormat::Png);
        let same = shown(&png(4, 2), 10).unwrap();
        assert_eq!(image::load_from_memory(&same).unwrap().width(), 4);
    }
}
```

Add `pub mod store;` to `src/imagery.rs`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p pencraft imagery::store`
Expected: does not compile.

- [ ] **Step 3: Write the store**

Put above the tests in `src/imagery/store.rs`:

```rust
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{Map, Value};

/// The folder pictures go into, beside the document.
const FOLDER: &str = "images";
/// The file in that folder that says where each picture came from.
pub const CREDITS: &str = "credits.json";
/// The most letters of a description that go into a file's name.
const MOST_NAME: usize = 40;

/// Only one picture is filed at a time, so two jobs ending together don't lose a line of the credits.
static FILING: Mutex<()> = Mutex::new(());

/// A picture that has been written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    /// What an image fill's `url` is to say: `./images/name.png` beside a document, a full path otherwise.
    pub url: String,
    /// The file.
    pub path: PathBuf,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

/// The start of a file's name for a picture described as `prompt`: its first plain words, joined by `-`.
fn stem(prompt: &str) -> String {
    let mut name = String::new();
    for word in prompt.split(|letter: char| !letter.is_ascii_alphanumeric()).filter(|word| !word.is_empty()) {
        if name.len() + word.len() + 1 > MOST_NAME {
            break;
        }
        if !name.is_empty() {
            name.push('-');
        }
        name.push_str(&word.to_ascii_lowercase());
    }
    if name.is_empty() { "picture".to_owned() } else { name }
}

/// The user's home folder.
#[must_use]
pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from)
}

/// Writes the picture `bytes` (described as `prompt`) and files `credit` under its name.
///
/// `document` is the document's `.pen` file; without one the picture goes to `.pencraft/images` under `home`.
///
/// # Errors
/// A sentence when the bytes aren't a picture or the folder can't be written; nothing is left behind then.
// Agent: WRITES two files; BLOCKS on decoding the picture, so call it on a worker thread.
pub fn save(document: Option<&Path>, home: Option<&Path>, bytes: &[u8], prompt: &str, credit: &Value) -> Result<Saved, String> {
    const NOT_A_PICTURE: &str = "The service sent something that isn't a picture.";
    let ending = match image::guess_format(bytes) {
        Ok(image::ImageFormat::Png) => "png",
        Ok(image::ImageFormat::Jpeg) => "jpg",
        Ok(image::ImageFormat::WebP) => "webp",
        _ => return Err(NOT_A_PICTURE.to_owned()),
    };
    let (width, height) = image::load_from_memory(bytes).map(|picture| (picture.width(), picture.height())).map_err(|_| NOT_A_PICTURE.to_owned())?;
    let beside = document.and_then(Path::parent).map(|folder| folder.join(FOLDER));
    let folder = beside
        .clone()
        .or_else(|| home.map(|home| home.join(".pencraft").join(FOLDER)))
        .ok_or("There is no folder to keep the picture in: save the document first.")?;
    let _one_at_a_time = FILING.lock().map_err(|_| "Filing pictures stopped working; restart Pencraft.".to_owned())?;
    std::fs::create_dir_all(&folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    // Human: a few letters from the clock tell two pictures of the same description apart; a name that is
    // taken all the same gets the next number.
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.subsec_nanos());
    let (name, path) = (0..u32::MAX)
        .map(|n| format!("{}-{:04x}.{ending}", stem(prompt), nanos.wrapping_add(n) & 0xffff))
        .map(|name| (folder.join(&name), name))
        .find(|(path, _)| !path.exists())
        .map(|(path, name)| (name, path))
        .ok_or("No free name for the picture.")?;
    craft_atomic::atomic_write(&path, bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    let credits = folder.join(CREDITS);
    let mut filed: Map<String, Value> = std::fs::read_to_string(&credits).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
    filed.insert(name.clone(), credit.clone());
    let text = serde_json::to_string_pretty(&filed).map_err(|error| error.to_string())?;
    craft_atomic::atomic_write(&credits, text.as_bytes()).map_err(|error| format!("{}: {error}", credits.display()))?;
    let url = if beside.is_some() { format!("./{FOLDER}/{name}") } else { path.to_string_lossy().replace('\\', "/") };
    Ok(Saved { url, path, width, height })
}

/// A PNG copy of the picture `bytes`, no wider or taller than `side` pixels, for a model to look at.
///
/// # Errors
/// A sentence when the bytes aren't a picture.
pub fn shown(bytes: &[u8], side: u32) -> Result<Vec<u8>, String> {
    let picture = image::load_from_memory(bytes).map_err(|error| error.to_string())?;
    let picture = if picture.width() > side || picture.height() > side { picture.thumbnail(side, side) } else { picture };
    let mut out = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut out, image::ImageFormat::Png).map_err(|error| error.to_string())?;
    Ok(out.into_inner())
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p pencraft imagery::store`
Expected: 5 passed. (`stem(&"word ".repeat(30)).len()` is 39: eight words of four letters and seven dashes.)

- [ ] **Step 5: Checkpoint**

Commit only if the user asked (`git add app/src/imagery.rs app/src/imagery/store.rs`).

---

### Task 6: The settings and which services work

**Files:**
- Modify: `src/prefs.rs`, `src/imagery.rs`, `src/settings.rs:464-472`

- [ ] **Step 1: Write the failing tests**

Append to `src/imagery.rs`:

```rust
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::prefs::{ImageService, Images};

    fn images() -> Images {
        Images {
            services: vec![
                ImageService { name: "Studio".to_owned(), kind: "OpenAI Images".to_owned(), base_url: String::new(), model: String::new(), key_ref: Some("k1".to_owned()) },
                ImageService { name: "No key".to_owned(), kind: "Google Gemini".to_owned(), base_url: String::new(), model: String::new(), key_ref: None },
                ImageService { name: "Odd".to_owned(), kind: "Something else".to_owned(), base_url: String::new(), model: String::new(), key_ref: Some("k2".to_owned()) },
            ],
            default: String::new(),
            assistant: true,
        }
    }

    #[test]
    fn the_working_services_are_the_saved_ones_with_a_key_and_the_ones_the_environment_gives() {
        let ready = ready(&images(), &|name| (name == "GEMINI_API_KEY").then(|| "g".to_owned()));
        let told: Vec<(&str, Kind, &str, &str)> = ready.iter().map(|r| (r.name.as_str(), r.kind, r.base_url.as_str(), r.model.as_str())).collect();
        assert_eq!(
            told,
            [("Studio", Kind::OpenAi, openai::BASE, openai::MODEL), ("Gemini", Kind::Gemini, gemini::BASE, gemini::MODEL)]
        );
        assert_eq!(ready[0].key, KeyFrom::Keychain("k1".to_owned()));
        assert_eq!(ready[1].key, KeyFrom::Environment("GEMINI_API_KEY"));
        assert!(super::ready(&Images::default(), &|_| None).is_empty());
    }

    #[test]
    fn one_is_picked_by_name_or_by_default_or_first() {
        let ready = ready(&images(), &|name| (name == "GEMINI_API_KEY").then(|| "g".to_owned()));
        assert_eq!(pick(&ready, Some("gemini"), "").map(|r| r.name.as_str()), Ok("Gemini"));
        assert_eq!(pick(&ready, None, "Gemini").map(|r| r.name.as_str()), Ok("Gemini"));
        assert_eq!(pick(&ready, None, "Gone").map(|r| r.name.as_str()), Ok("Studio"));
        assert_eq!(pick(&ready, Some("Nope"), ""), Err("There is no image service called \"Nope\". The ones there are: Studio, Gemini.".to_owned()));
        assert_eq!(pick(&[], None, ""), Err(NO_SERVICE.to_owned()));
    }

    #[test]
    fn get_state_lists_them_with_what_they_can_do() {
        let ready = ready(&images(), &|_| None);
        assert_eq!(
            listed(&ready, ""),
            json!([{ "name": "Studio", "kind": "OpenAI Images", "model": openai::MODEL, "default": true, "transparent": true }])
        );
    }

    #[test]
    fn a_tool_call_becomes_a_request_or_says_what_is_wrong() {
        let ready = ready(&images(), &|name| (name == "GEMINI_API_KEY").then(|| "g".to_owned()));
        let asked = request(&json!({ "prompt": " A fox ", "aspect": "16:9", "size": "large", "transparent": true }), &ready[0]);
        assert_eq!(
            asked,
            Ok(Request { prompt: "A fox".to_owned(), model: openai::MODEL.to_owned(), aspect: Aspect(16, 9), large: true, transparent: true })
        );
        assert_eq!(request(&json!({ "prompt": "A fox" }), &ready[1]).map(|r| (r.aspect, r.large, r.transparent)), Ok((Aspect(1, 1), false, false)));
        assert_eq!(request(&json!({ "prompt": "  " }), &ready[0]), Err("`prompt` is missing: say what the picture shows.".to_owned()));
        assert_eq!(
            request(&json!({ "prompt": "x", "aspect": "5:1" }), &ready[0]),
            Err("`aspect` must be one of 1:1, 16:9, 9:16, 4:3, 3:4, 3:2, 2:3.".to_owned())
        );
        assert_eq!(request(&json!({ "prompt": "x", "size": "huge" }), &ready[0]), Err("`size` must be \"normal\" or \"large\".".to_owned()));
        assert_eq!(
            request(&json!({ "prompt": "x", "transparent": true }), &ready[1]),
            Err("Gemini can't make a transparent background. Leave `transparent` out, or use a service that can (get_state lists them).".to_owned())
        );
    }

    #[test]
    fn the_settings_start_with_no_service_and_the_assistant_allowed() {
        let images = Images::default();
        assert!(images.services.is_empty() && images.default.is_empty() && images.assistant);
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p pencraft imagery::tests`
Expected: does not compile.

- [ ] **Step 3: Add the settings**

In `src/prefs.rs`:

a) In `struct Prefs`, after the `generation` field, add:

```rust
    /// The image model services and who may use them.
    pub images: Images,
```

b) Remove the field `image_provider` (with its doc comment) from `struct Generation`, and change `Default for Generation` to:

```rust
impl Default for Generation {
    fn default() -> Self {
        Generation { html_metadata: false, svg_provider: "Pencraft".to_owned() }
    }
}
```

and the struct's doc comment and the field comment in `Prefs` from "Importing pages and generating pictures." to "Importing pages and generating drawings."

c) After `impl Default for Generation`, add:

```rust
/// The image model services and who may use them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Images {
    /// The services the user added. Their API keys are in the keychain, not here.
    pub services: Vec<ImageService>,
    /// The name of the service used when none is asked for; empty for the first one that works.
    pub default: String,
    /// AI models may generate pictures (each one costs money on the user's key).
    pub assistant: bool,
}

impl Default for Images {
    fn default() -> Self {
        Images { services: Vec::new(), default: String::new(), assistant: true }
    }
}

/// One account at one image model provider.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageService {
    /// What the user calls it.
    pub name: String,
    /// The kind of API it speaks, as [`crate::imagery::Kind::name`] gives it.
    pub kind: String,
    /// Where it is reached; empty for the kind's own address.
    pub base_url: String,
    /// The model; empty for the kind's usual one.
    pub model: String,
    /// The name its API key is kept under in the keychain; `None` when it has no key. Never the key itself.
    pub key_ref: Option<String>,
}
```

In `src/settings.rs` remove the placeholder row and one divider, so lines 464-472 become:

```rust
    section(ui, "Import & generation");
    let generation = &mut prefs.generation;
    row(ui, "Include HTML metadata when importing pages", None, palette, |ui| switch(ui, &mut generation.html_metadata, palette));
    divider(ui, palette);
    row(ui, "Default SVG generation provider", None, palette, |ui| choice(ui, "svg-provider", &mut generation.svg_provider, &["Pencraft"], palette));
```

- [ ] **Step 4: Write the module root's code**

In `src/imagery.rs`, between the `pub mod` lines and the tests, add:

```rust
use craft_ai_core::SecretString;
use serde_json::{Value, json};

use crate::prefs::Images;
use crate::secrets::Vault;
use service::{ASPECTS, Aspect, Failure, Made, Request};

/// What a model or a person is told when no service is set up.
pub const NO_SERVICE: &str = "No image service is set up. Add one with its API key in Settings → Images, or start Pencraft with GEMINI_API_KEY or OPENAI_API_KEY set.";

/// The kinds of service Pencraft can talk to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Google's Gemini image models.
    Gemini,
    /// OpenAI's image models, and services that speak the same API.
    OpenAi,
}

impl Kind {
    /// Every kind, in the order the settings offer them.
    pub const ALL: [Kind; 2] = [Kind::Gemini, Kind::OpenAi];

    /// The kind's name, as the settings show and save it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Kind::Gemini => "Google Gemini",
            Kind::OpenAi => "OpenAI Images",
        }
    }

    /// The kind called `name`.
    #[must_use]
    pub fn named(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|kind| kind.name() == name)
    }

    /// Where the kind's own service is reached.
    #[must_use]
    pub fn base(self) -> &'static str {
        match self {
            Kind::Gemini => gemini::BASE,
            Kind::OpenAi => openai::BASE,
        }
    }

    /// The model a new service of this kind starts with.
    #[must_use]
    pub fn model(self) -> &'static str {
        match self {
            Kind::Gemini => gemini::MODEL,
            Kind::OpenAi => openai::MODEL,
        }
    }

    /// Whether the kind can make a transparent background.
    #[must_use]
    pub fn transparent(self) -> bool {
        matches!(self, Kind::OpenAi)
    }

    /// The environment variable that gives a key for this kind, and what such a service is called.
    fn environment(self) -> (&'static str, &'static str) {
        match self {
            Kind::Gemini => ("GEMINI_API_KEY", "Gemini"),
            Kind::OpenAi => ("OPENAI_API_KEY", "OpenAI"),
        }
    }
}

/// Where a service's key is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyFrom {
    /// This environment variable.
    Environment(&'static str),
    /// The keychain, under this name.
    Keychain(String),
}

/// A service that can be asked: it has a kind Pencraft knows and a key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ready {
    /// What the user calls it.
    pub name: String,
    /// The API it speaks.
    pub kind: Kind,
    /// Where it is reached.
    pub base_url: String,
    /// The model it is asked for.
    pub model: String,
    /// Where its key is.
    pub key: KeyFrom,
}

/// The services that can be asked: the saved ones that have a key, then one for each key in the environment
/// (`variable` reads it) whose name no saved one has.
#[must_use]
pub fn ready(images: &Images, variable: &dyn Fn(&str) -> Option<String>) -> Vec<Ready> {
    let or = |text: &str, usual: &str| if text.trim().is_empty() { usual.to_owned() } else { text.trim().to_owned() };
    let mut ready: Vec<Ready> = images
        .services
        .iter()
        .filter_map(|saved| {
            let kind = Kind::named(&saved.kind)?;
            let key = KeyFrom::Keychain(saved.key_ref.clone()?);
            Some(Ready { name: saved.name.clone(), kind, base_url: or(&saved.base_url, kind.base()), model: or(&saved.model, kind.model()), key })
        })
        .collect();
    for kind in Kind::ALL {
        let (name, called) = kind.environment();
        if variable(name).is_some_and(|key| !key.trim().is_empty()) && !ready.iter().any(|r| r.name.eq_ignore_ascii_case(called)) {
            ready.push(Ready { name: called.to_owned(), kind, base_url: kind.base().to_owned(), model: kind.model().to_owned(), key: KeyFrom::Environment(name) });
        }
    }
    ready
}

/// The service to use: the one `asked` for by name, else the one the settings call the default, else the first.
///
/// # Errors
/// A sentence when there is none, or none by that name.
pub fn pick<'a>(ready: &'a [Ready], asked: Option<&str>, default: &str) -> Result<&'a Ready, String> {
    let named = |name: &str| ready.iter().find(|r| r.name.eq_ignore_ascii_case(name.trim()));
    match asked.filter(|name| !name.trim().is_empty()) {
        Some(name) => named(name).ok_or_else(|| {
            if ready.is_empty() {
                NO_SERVICE.to_owned()
            } else {
                format!("There is no image service called \"{name}\". The ones there are: {}.", ready.iter().map(|r| r.name.as_str()).collect::<Vec<_>>().join(", "))
            }
        }),
        None => named(default).or_else(|| ready.first()).ok_or_else(|| NO_SERVICE.to_owned()),
    }
}

/// What `get_state` says about the services.
#[must_use]
pub fn listed(ready: &[Ready], default: &str) -> Value {
    let usual = pick(ready, None, default).ok().map(|r| r.name.clone());
    ready
        .iter()
        .map(|r| json!({ "name": r.name, "kind": r.kind.name(), "model": r.model, "default": Some(&r.name) == usual.as_ref(), "transparent": r.kind.transparent() }))
        .collect()
}

/// The request a `generate_image` call (or the dialog, in the same form) asks of the service `ready`.
///
/// # Errors
/// A sentence saying which argument is wrong.
pub fn request(args: &Value, ready: &Ready) -> Result<Request, String> {
    let prompt = args.get("prompt").and_then(Value::as_str).map(str::trim).filter(|prompt| !prompt.is_empty());
    let prompt = prompt.ok_or("`prompt` is missing: say what the picture shows.")?.to_owned();
    let aspect = match args.get("aspect").and_then(Value::as_str) {
        Some(text) => Aspect::parse(text).ok_or_else(|| format!("`aspect` must be one of {}.", ASPECTS.join(", ")))?,
        None => Aspect::default(),
    };
    let large = match args.get("size").and_then(Value::as_str) {
        None | Some("normal") => false,
        Some("large") => true,
        Some(_) => return Err("`size` must be \"normal\" or \"large\".".to_owned()),
    };
    let transparent = args.get("transparent").and_then(Value::as_bool).unwrap_or(false);
    if transparent && !ready.kind.transparent() {
        return Err(format!(
            "{} can't make a transparent background. Leave `transparent` out, or use a service that can (get_state lists them).",
            ready.name
        ));
    }
    Ok(Request { prompt, model: ready.model.clone(), aspect, large, transparent })
}

/// The key of the service `ready`.
///
/// # Errors
/// [`Failure::Key`] when it is no longer where it was.
pub fn key(ready: &Ready, vault: &dyn Vault) -> Result<SecretString, Failure> {
    match &ready.key {
        KeyFrom::Environment(name) => std::env::var(name)
            .ok()
            .filter(|key| !key.trim().is_empty())
            .map(|key| SecretString::new(key.trim()))
            .ok_or_else(|| Failure::Key(format!("The {name} environment variable is no longer set."))),
        KeyFrom::Keychain(name) => vault.get(name).map_err(|error| Failure::Key(format!("The key couldn't be read from the keychain: {error}."))),
    }
}

/// Asks the service `ready` for the picture `request` describes.
///
/// # Errors
/// Why there is no picture.
// Agent: BLOCKS for one answer (up to 95 s).
pub fn make(ready: &Ready, request: &Request, key: &SecretString) -> Result<Made, Failure> {
    match ready.kind {
        Kind::Gemini => gemini::make(&ready.base_url, request, key),
        Kind::OpenAi => openai::make(&ready.base_url, request, key),
    }
}
```

`SecretString::new` takes what `secrets.rs` passes it (`String` or `&str`); if `key.trim()` (a `&str`) is refused, pass `key.trim().to_owned()`.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p pencraft imagery`
Expected: all `imagery::` tests pass (23 so far), and `cargo test -p pencraft settings` still passes.

- [ ] **Step 6: Checkpoint**

Commit only if the user asked (`git add app/src/prefs.rs app/src/settings.rs app/src/imagery.rs`).

---

### Task 7: The job

**Files:**
- Create: `src/imagery/jobs.rs`
- Modify: `src/imagery.rs` (add `pub mod jobs;` after `pub mod gemini;`)

- [ ] **Step 1: Write the failing tests**

Create `src/imagery/jobs.rs` with only:

```rust
//! One picture being made, on a thread of its own: asking the service, writing the file, and making the small
//! copy a model is shown. The app asks now and then whether it is done.

#[cfg(test)]
mod tests {
    use super::*;

    fn done() -> Done {
        Done {
            saved: Saved { url: "./images/a.png".to_owned(), path: PathBuf::from("/d/images/a.png"), width: 4, height: 2 },
            shown: vec![1, 2, 3],
            service: "Studio".to_owned(),
            model: "m".to_owned(),
        }
    }

    fn wait(job: &Job) -> Result<Done, String> {
        for _ in 0..500 {
            if let Some(result) = job.poll() {
                return result;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("the job never ended");
    }

    #[test]
    fn a_job_hands_back_what_its_work_made_and_wakes_the_app() {
        let woken = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&woken);
        let job = Job::start(|| Ok(done()), move || flag.store(true, Ordering::Relaxed));
        assert_eq!(wait(&job), Ok(done()));
        assert!(woken.load(Ordering::Relaxed));
        let job = Job::start(|| Err("Studio refused the description: no.".to_owned()), || {});
        assert_eq!(wait(&job), Err("Studio refused the description: no.".to_owned()));
    }

    #[test]
    fn a_job_that_takes_too_long_or_dies_says_so() {
        let slow = Job::start_with(|| { std::thread::sleep(Duration::from_secs(2)); Ok(done()) }, || {}, Duration::from_millis(20));
        assert_eq!(slow.poll(), None);
        std::thread::sleep(Duration::from_millis(40));
        assert_eq!(slow.poll(), Some(Err("The picture took too long (over 0 seconds); nothing was made.".to_owned())));
        let dead = Job::start(|| panic!("boom"), || {});
        assert_eq!(wait(&dead), Err("Making the picture stopped unexpectedly.".to_owned()));
    }
}
```

Add `pub mod jobs;` to `src/imagery.rs`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p pencraft imagery::jobs`
Expected: does not compile.

- [ ] **Step 3: Write the job**

Put above the tests in `src/imagery/jobs.rs`:

```rust
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::{Duration, Instant};

use serde_json::json;

use super::service::Request;
use super::store::{self, Saved};
use super::{Ready, key, make};
use crate::secrets::Keychain;

#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

/// How long a picture may take. Under the 120 seconds the local server waits for a tool's answer
/// (`link::PATIENCE`), so a model is always told what happened.
pub const LIMIT: Duration = Duration::from_secs(100);
/// The longest side of the copy a model is shown, in pixels.
const SHOWN_SIDE: u32 = 1024;

/// What to make and where to keep it.
#[derive(Clone, Debug)]
pub struct Order {
    /// The service to ask.
    pub ready: Ready,
    /// The picture to ask for.
    pub request: Request,
    /// The document's `.pen` file, if it has one: the picture goes beside it.
    pub document: Option<PathBuf>,
}

/// A picture that was made and written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Done {
    /// The file and what a fill calls it.
    pub saved: Saved,
    /// A small PNG copy, for a model to look at.
    pub shown: Vec<u8>,
    /// The service that made it.
    pub service: String,
    /// The model that made it.
    pub model: String,
}

/// Does `order` from start to end: reads the key, asks the service, writes the picture and its credit.
///
/// # Errors
/// A sentence for a person or a model; nothing is left half-written.
// Agent: BLOCKS up to 95 s on the network; READS the keychain; WRITES `images/`. Worker thread only.
pub fn work(order: &Order) -> Result<Done, String> {
    let name = &order.ready.name;
    let key = key(&order.ready, &Keychain).map_err(|failure| failure.sentence(name))?;
    let made = make(&order.ready, &order.request, &key).map_err(|failure| failure.sentence(name))?;
    let credit = json!({ "made": "generated", "service": name, "model": order.request.model, "prompt": order.request.prompt });
    let saved = store::save(order.document.as_deref(), store::home().as_deref(), &made.0, &order.request.prompt, &credit)?;
    let shown = store::shown(&made.0, SHOWN_SIDE)?;
    Ok(Done { saved, shown, service: name.clone(), model: order.request.model.clone() })
}

/// A picture being made.
pub struct Job {
    got: Receiver<Result<Done, String>>,
    since: Instant,
    limit: Duration,
}

impl Job {
    /// Starts `work` on a thread of its own; `wake` is called when it has ended.
    pub fn start(work: impl FnOnce() -> Result<Done, String> + Send + 'static, wake: impl Fn() + Send + 'static) -> Job {
        Job::start_with(work, wake, LIMIT)
    }

    /// [`Job::start`] with another time limit.
    // Agent: SPAWNS one thread per job. A job given up on (limit passed, or the `Job` dropped) lets its thread
    // run to the end of its network call; its result is then thrown away, but a picture it made stays on disk.
    fn start_with(work: impl FnOnce() -> Result<Done, String> + Send + 'static, wake: impl Fn() + Send + 'static, limit: Duration) -> Job {
        let (send, got) = channel();
        let started = std::thread::Builder::new().name("make-picture".to_owned()).spawn(move || {
            // Nobody may be listening any more (the call was given up on); that is fine.
            let _unheard = send.send(work());
            wake();
        });
        if let Err(error) = started {
            log::warn!("couldn't start making a picture: {error}");
        }
        Job { got, since: Instant::now(), limit }
    }

    /// The result, once there is one; `None` while the picture is still being made.
    #[must_use]
    pub fn poll(&self) -> Option<Result<Done, String>> {
        match self.got.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Disconnected) => Some(Err("Making the picture stopped unexpectedly.".to_owned())),
            Err(TryRecvError::Empty) if self.since.elapsed() > self.limit => {
                Some(Err(format!("The picture took too long (over {} seconds); nothing was made.", self.limit.as_secs())))
            }
            Err(TryRecvError::Empty) => None,
        }
    }
}
```

The `wake` closure is called by `spawn`'s closure after `send`; when the thread could not start, the sender is dropped and `poll` reports "stopped unexpectedly".

- [ ] **Step 4: Run the tests**

Run: `cargo test -p pencraft imagery::jobs`
Expected: 2 passed (the panicking job prints a panic message to the test output; that is expected).

- [ ] **Step 5: Checkpoint**

Run: `cargo test -p pencraft imagery` — all pass. Commit only if the user asked (`git add app/src/imagery.rs app/src/imagery/jobs.rs`).

---

### Task 8: The `generate_image` tool

After this task `scripts/check.sh` must pass.

**Files:**
- Modify: `src/agent.rs` (tool list, guide, tests), `src/chat/steps.rs`, `src/app.rs` (field, `mod`), `src/app/tools.rs` (dispatch, state)
- Create: `src/app/pictures.rs`

- [ ] **Step 1: Write the failing tests**

a) In `src/agent.rs` `mod tests`, in `every_tool_has_a_name_a_description_and_a_schema`, add `"generate_image"` to the expected names after `"export_layers"`:

```rust
                "export_layers",
                "generate_image"
```

and add a test:

```rust
    #[test]
    fn the_guide_says_when_to_generate_a_picture() {
        assert!(GUIDE.contains("## Pictures") && GUIDE.contains("generate_image") && GUIDE.contains("imageServices"));
    }
```

b) In `src/chat/steps.rs` `mod tests`, add:

```rust
    #[test]
    fn generating_a_picture_is_named_with_the_start_of_its_description() {
        assert_eq!((doing("generate_image"), did("generate_image")), ("Generating a picture".to_owned(), "Generated a picture".to_owned()));
        assert_eq!(failed("generate_image"), "Couldn't generate the picture");
        assert_eq!(icon("generate_image"), "image-plus");
        assert_eq!(detail("generate_image", &json!({ "prompt": "A red fox in fresh snow at dawn, seen from the side" })), "“A red fox in fresh snow at dawn…”");
        assert_eq!(detail("generate_image", &json!({ "prompt": "A fox" })), "“A fox”");
    }
```

c) Create `src/app/pictures.rs` with only its tests for the two plain functions:

```rust
//! The app's side of making pictures: `generate_image` calls that wait for their picture, the dialog's own
//! picture, and putting a finished one into the document.

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn editor() -> Editor {
        let doc = pen_doc::read(
            r##"{"version":"2.22","children":[
              {"type":"rectangle","id":"hero","x":0,"y":0,"width":400,"height":300,"fill":"#eeeeee"},
              {"type":"text","id":"title","x":20,"y":20,"content":"Hello","fill":"#000000"}]}"##,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 7);
        editor.refresh(&|_| (10.0, 10.0));
        editor
    }

    #[test]
    fn only_a_shape_or_a_frame_takes_a_picture() {
        let editor = editor();
        assert_eq!(takes_picture(&editor, "hero"), Ok(()));
        assert_eq!(takes_picture(&editor, "title"), Err("The layer \"title\" is a text: a picture goes on a rectangle, an ellipse or a frame.".to_owned()));
        assert_eq!(takes_picture(&editor, "gone"), Err("There is no layer with the id \"gone\".".to_owned()));
    }

    #[test]
    fn filling_a_layer_is_one_undo_step() {
        let mut editor = editor();
        let undo_before = editor.undo.len();
        fill_with(&mut editor, "hero", "./images/a.png").unwrap();
        let node = pen_doc::edit::find(&editor.doc, "hero").unwrap();
        assert_eq!(serde_json::to_value(&node.fill).unwrap(), json!({ "type": "image", "url": "./images/a.png", "mode": "cover" }));
        assert_eq!(editor.undo.len(), undo_before + 1);
        assert!(fill_with(&mut editor, "gone", "./images/a.png").is_err());
        assert_eq!(editor.undo.len(), undo_before + 1);
    }
}
```

If `editor.undo` is private to `editor.rs`, assert on behaviour instead: call the editor's undo method (the one `Command::Undo` uses; find it with `grep -n "fn undo" src/editor.rs src/ops.rs`) once and check the fill is `"#eeeeee"` again.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p pencraft generate` and `cargo test -p pencraft app::pictures`
Expected: failures and compile errors (`takes_picture`, `fill_with` not found; `"generate_image"` missing).

- [ ] **Step 3: The tool's schema and the guide**

In `src/agent.rs` `tools()`, add after the `export_layers` entry (mind the comma after its closing brace):

```rust
        {
            "name": "generate_image",
            "description": "Have an image model make a picture from a description: a photograph or an illustration a design needs (a hero, a product shot, an avatar). Answers with the picture and the `url` to put in an image fill, or gives it to `layer` as its fill. Takes up to a minute and costs the user money each time: one picture per need, and look at it before asking for another. Never for interface, text, icons or logos; draw those as layers. `get_state` lists the services under `imageServices`; with none there, tell the user to add one in Settings → Images.",
            "inputSchema": { "type": "object", "required": ["prompt"], "properties": {
                "document": document,
                "prompt": { "type": "string", "description": "What the picture shows: subject, setting, light, framing and style, in plain sentences." },
                "service": { "type": "string", "description": "A service's name from `imageServices`. Leave out for the user's default." },
                "aspect": { "type": "string", "enum": ["1:1", "16:9", "9:16", "4:3", "3:4", "3:2", "2:3"], "description": "Width to height (default 1:1). Pick the one nearest the layer it is for." },
                "size": { "type": "string", "enum": ["normal", "large"], "description": "normal: about 1024 pixels on the long side (default). large: about 2048, for a picture shown big." },
                "transparent": { "type": "boolean", "description": "A transparent background, for a thing cut out. Only services listed with `transparent: true` can." },
                "layer": { "type": "string", "description": "A rectangle, ellipse or frame to give the picture to as its fill (one undo step). Without it nothing in the document changes." }
            } }
        }
```

`document` is moved into the first use above it; write `"document": document.clone()` in the `export_layers` entry if the compiler says the value was moved.

In `GUIDE`, add before `## Working well`:

```text
## Pictures
A photograph or an illustration is a file shown by an image fill (see "Fill and stroke"). When the user asks
for one, or a design plainly needs one (a hero, a product shot, a portrait), `generate_image` makes it:
describe subject, setting, light, framing and style; pick the `aspect` nearest the layer; give `layer` to put
it straight onto a rectangle, ellipse or frame, or use the `url` it answers with in a fill. Each picture costs
the user money and up to a minute: one per need, and look at it before asking for another. `get_state` lists
the services under `imageServices`; with none, say so and go on without a picture (a plain fill, not a
stand-in drawn to look like a photo). Interface, text, icons and logos are never generated: draw them.
```

- [ ] **Step 4: The chat's words**

In `src/chat/steps.rs`: change `const TOOLS: [Words; 13]` to `[Words; 14]` and add before the `review_design` entry:

```rust
    Words {
        tool: "generate_image",
        icon: "image-plus",
        doing: "Generating a picture",
        did: "Generated a picture",
        failed: "Couldn't generate the picture",
    },
```

Add above `fn detail`:

```rust
/// The most letters of a picture's description a note shows.
const MOST_PROMPT: usize = 36;
```

and in `detail`'s `match`, before `_ => String::new()`:

```rust
        "generate_image" => {
            let prompt = input.get("prompt").and_then(Value::as_str).map(str::trim).unwrap_or_default();
            if prompt.chars().count() > MOST_PROMPT {
                // Human: cut at the last whole word that fits.
                let cut: String = prompt.chars().take(MOST_PROMPT).collect();
                format!("“{}…”", cut.rsplit_once(' ').map_or(cut.as_str(), |(whole, _)| whole).trim_end_matches(','))
            } else if prompt.is_empty() {
                String::new()
            } else {
                format!("“{prompt}”")
            }
        }
```

Confirm `image-plus` exists in the Lucide set: `grep -rn "image-plus" ~/.cargo/registry/src/*/lucide-static-svg-1.72.0/ | head -1`. If not, use `"image"`, and change the test.

- [ ] **Step 5: The app's side**

In `src/app.rs`: add `mod pictures;` after `mod talk;` (alphabetical: before `talk`), add to `struct App` after `waiting`:

```rust
    /// `generate_image` calls whose picture is still being made.
    picturing: Vec<pictures::Waiting>,
```

and to `App::new`'s struct literal after `waiting: Vec::new(),`:

```rust
            picturing: Vec::new(),
```

Put above the tests in `src/app/pictures.rs`:

```rust
use std::sync::mpsc::channel;
use std::time::Duration;

use pen_doc::model::Kind as LayerKind;
use serde_json::{Value, json};

use super::App;
use crate::agent;
use crate::editor::Editor;
use crate::imagery;
use crate::imagery::jobs::{self, Done, Job, Order};
use crate::link::Call;

/// How often the app looks at pictures being made, so one that takes too long is given up on in time.
const LOOK_AGAIN: Duration = Duration::from_secs(1);

/// A `generate_image` call that waits for its picture.
pub(super) struct Waiting {
    call: Call,
    job: Job,
    /// The document the call is about, as `App::document` finds it again when the picture is there.
    document: Value,
}

/// Whether the layer `id` can be given a picture as its fill.
fn takes_picture(editor: &Editor, id: &str) -> Result<(), String> {
    let node = pen_doc::edit::find(&editor.doc, id).ok_or_else(|| format!("There is no layer with the id \"{id}\"."))?;
    if matches!(node.kind, LayerKind::Rectangle | LayerKind::Ellipse | LayerKind::Frame) {
        Ok(())
    } else {
        let kind = serde_json::to_value(&node.kind).ok().and_then(|kind| kind.as_str().map(str::to_owned)).unwrap_or_else(|| "layer".to_owned());
        Err(format!("The layer \"{id}\" is a {kind}: a picture goes on a rectangle, an ellipse or a frame."))
    }
}

/// The `edit_layers` arguments that give the layer `id` the picture at `url` as its fill.
fn fill_edit(document: &Value, id: &str, url: &str) -> Value {
    json!({
        "document": document,
        "description": "Put the generated picture on the layer",
        "operations": [{ "op": "update", "id": id, "properties": { "fill": { "type": "image", "url": url, "mode": "cover" } } }]
    })
}

/// Gives the layer `id` the picture at `url` as its fill, as one undo step.
// Human: the same order as a model's change (`App::edit_tool`): the step is kept only when the change worked.
pub(super) fn fill_with(editor: &mut Editor, id: &str, url: &str) -> Result<(), String> {
    let before = editor.doc.clone();
    agent::edit(editor, &fill_edit(&Value::Null, id, url))?;
    let after = std::mem::replace(&mut editor.doc, before);
    editor.checkpoint();
    editor.doc = after;
    Ok(())
}

impl App {
    /// The image services that can be asked right now, and the settings' default.
    pub(super) fn image_services(&self) -> (Vec<imagery::Ready>, String) {
        let images = &self.prefs.get().images;
        (imagery::ready(images, &|name| std::env::var(name).ok()), images.default.clone())
    }

    /// Starts a `generate_image` call: answers at once when it can't be done, otherwise keeps it until its
    /// picture is there.
    pub(super) fn start_picture(&mut self, call: Call, ctx: &egui::Context) {
        match self.picture_order(&call.arguments) {
            Ok((order, document)) => {
                let wake = ctx.clone();
                let job = Job::start(move || jobs::work(&order), move || wake.request_repaint());
                self.picturing.push(Waiting { call, job, document });
                ctx.request_repaint_after(LOOK_AGAIN);
            }
            Err(why) => {
                // A client that stopped waiting has gone; there is no one to tell.
                let _ = call.reply.send(Err(why));
            }
        }
    }

    /// What a `generate_image` call asks for, and the document it is about by a name that still finds it later.
    fn picture_order(&self, args: &Value) -> Result<(Order, Value), String> {
        if !self.prefs.get().images.assistant {
            return Err("Generating pictures is turned off for AI models in Pencraft's settings (Images: \"Let the assistant generate pictures\").".to_owned());
        }
        let tab = self.document(args)?;
        let editor = self.tabs.get(tab).ok_or("The document was closed.")?;
        if let Some(layer) = args.get("layer").and_then(Value::as_str) {
            takes_picture(editor, layer)?;
        }
        let (ready, default) = self.image_services();
        let chosen = imagery::pick(&ready, args.get("service").and_then(Value::as_str), &default)?.clone();
        let request = imagery::request(args, &chosen)?;
        // Human: the picture takes a while and the user may bring another tab to the front meanwhile, so the
        // document is remembered by its file, or by its title while it has none.
        let document = json!(editor.path.as_ref().map_or_else(|| editor.title(), |path| path.display().to_string()));
        Ok((Order { ready: chosen, request, document: editor.path.clone() }, document))
    }

    /// Answers the `generate_image` calls whose picture is there, or failed, or took too long.
    // Agent: MAIN-THREAD; runs from `serve_tools`, so also while the window is hidden.
    pub(super) fn finish_pictures(&mut self, ctx: &egui::Context) {
        if self.picturing.is_empty() {
            return;
        }
        for waiting in std::mem::take(&mut self.picturing) {
            match waiting.job.poll() {
                None => self.picturing.push(waiting),
                Some(result) => {
                    let answer = result.and_then(|done| self.picture_answer(&waiting, &done, ctx));
                    let _ = waiting.call.reply.send(answer);
                }
            }
        }
        if !self.picturing.is_empty() {
            ctx.request_repaint_after(LOOK_AGAIN);
        }
    }

    /// What a model is told about its picture; gives it to the layer the call named first.
    fn picture_answer(&mut self, waiting: &Waiting, done: &Done, ctx: &egui::Context) -> Result<Vec<Value>, String> {
        let mut told = json!({
            "url": done.saved.url,
            "width": done.saved.width,
            "height": done.saved.height,
            "service": done.service,
            "model": done.model,
        });
        if let Some(layer) = waiting.call.arguments.get("layer").and_then(Value::as_str) {
            // Human: the fill is set the way a model sets one itself, so the layer flashes, the model's cursor
            // goes to it, and ⌘Z takes it back.
            let (reply, _unheard) = channel();
            let edit = Call { tool: "edit_layers".to_owned(), arguments: fill_edit(&waiting.document, layer, &done.saved.url), client: waiting.call.client.clone(), reply };
            match self.edit_tool(&edit, ctx) {
                Ok(_) => told["layer"] = json!(layer),
                Err(why) => told["layerProblem"] = json!(format!("The picture was made, but the layer wasn't changed: {why} Use the `url` in a fill yourself.")),
            }
        } else {
            told["next"] = json!("Nothing in the document changed. Put the `url` in an image fill with edit_layers.");
        }
        Ok(vec![
            json!({ "type": "image", "data": agent::base64(&done.shown), "mimeType": "image/png" }),
            json!({ "type": "text", "text": serde_json::to_string_pretty(&told).unwrap_or_default() }),
        ])
    }
}
```

`edit_tool` is a private method of `impl App` in `src/app/tools.rs`; `pictures` is a sibling module, so change its signature there from `fn edit_tool` to `pub(super) fn edit_tool`. If `Node::kind` or the `Kind` variants are named differently, follow `src/ops.rs:353` (`Node::new(Kind::Rectangle, ..)`).

In `src/app/tools.rs`:

a) `serve_tools`: the early return must not skip finished pictures. Change the start of the function to:

```rust
    pub(super) fn serve_tools(&mut self, ctx: &egui::Context) {
        self.finish_pictures(ctx);
        let mut calls = std::mem::take(&mut self.waiting);
```

b) In the `for (call, since) in calls` loop, a picture is started and not answered in place. Insert as the first statement of the loop body:

```rust
            // Human: a picture takes up to a minute; its call is kept and answered when the picture is there.
            if call.tool == "generate_image" && self.prefs.get().agents.tools {
                self.start_picture(call, ctx);
                continue;
            }
```

c) `state`: change its last line to add the services:

```rust
        let (ready, default) = self.image_services();
        json!({
            "documents": documents,
            "front": front.and_then(|tab| self.tabs.get(tab)).map(agent::state),
            "imageServices": crate::imagery::listed(&ready, &default),
        })
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p pencraft`
Expected: everything passes.

- [ ] **Step 7: Run the whole gate**

Run: `bash scripts/check.sh`
Expected: `check: all passed`. Dead-code errors now mean something in `imagery` is truly unused: `Kind::ALL`/`Kind::name` are used by `ready`/`listed`; `store::CREDITS` only by tests and `save`. If an item is reported, either it is used only from a later task (Tasks 9 and 10 use nothing new from `imagery` except `Kind::ALL`, `Kind::name`, `Kind::base`, `Kind::model`, which are all used already) or it should be deleted. Do not silence the lint.

- [ ] **Step 8: Try it for real**

With a service key in the environment (`GEMINI_API_KEY` or `OPENAI_API_KEY`), build and start the app (`cargo run --release`), open a document with a rectangle, and from the chat ask: "Put a photo of a red fox in snow on the selected rectangle." Expected: a note "Generating a picture “…”", then the rectangle shows the picture; `images/` beside the `.pen` holds the file and `credits.json`; ⌘Z/Ctrl+Z takes the fill back. Without any key: the assistant reports that no image service is set up. Say in your report which of these you could run.

- [ ] **Step 9: Checkpoint**

Commit only if the user asked (`git add app/src`).

---

### Task 9: Settings → Images

**Files:**
- Modify: `src/settings.rs`

The page is built from the helpers the Agents page already uses: `title`, `section`, `row`, `switch`, `choice`, `divider`, `caption`, `field`, `wide_choice`, `provider_card`.

- [ ] **Step 1: Write the failing tests**

In `src/settings.rs` `mod tests` (it has a fake vault with a `kept` map; use it the way `save_provider`'s tests do), add:

```rust
    #[test]
    fn an_image_service_form_needs_a_name_no_other_service_has() {
        let mut prefs = Prefs::default();
        let mut form = ImageForm::new();
        assert_eq!(form.service(&prefs.images), None);
        form.name = " Studio ".to_owned();
        let service = form.service(&prefs.images).unwrap();
        assert_eq!((service.name.as_str(), service.kind.as_str(), service.base_url.as_str(), service.model.as_str()), ("Studio", "Google Gemini", "", ""));
        prefs.images.services.push(service);
        assert_eq!(form.service(&prefs.images), None, "a second service of the same name");
        let again = ImageForm::of(0, &prefs.images.services[0]);
        assert!(again.service(&prefs.images).is_some(), "the service being changed may keep its name");
    }

    #[test]
    fn an_image_services_key_goes_to_the_vault_and_leaves_with_the_service() {
        let vault = FakeVault::default();
        let mut prefs = Prefs::default();
        let mut form = ImageForm::new();
        form.name = "Studio".to_owned();
        form.key = "secret".to_owned();
        let service = form.service(&prefs.images).unwrap();
        save_image_service(&mut form, service, &mut prefs, &vault).unwrap();
        let name = prefs.images.services[0].key_ref.clone().unwrap();
        assert_eq!(vault.kept.borrow().len(), 1);
        assert!(form.key.is_empty());
        prefs.images.default = "Studio".to_owned();
        remove_image_service(0, &mut prefs, &vault).unwrap();
        assert!(prefs.images.services.is_empty() && prefs.images.default.is_empty());
        assert!(!vault.kept.borrow().contains_key(&name));
    }
```

Use the fake vault's real name and the real type of its `kept` map as they are in the file's existing tests (`grep -n "kept" src/settings.rs`).

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p pencraft settings::tests::an_image`
Expected: does not compile.

- [ ] **Step 3: The form and its saving**

In `src/settings.rs`, change the `use crate::prefs::..` line to also bring in `ImageService` and `Images`, and add after `impl Form { .. }`:

```rust
/// The image service form.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ImageForm {
    /// The saved service being changed; `None` for a new one.
    index: Option<usize>,
    name: String,
    kind: String,
    base_url: String,
    model: String,
    /// A newly typed key; it goes to the keychain when the form is saved, never to the settings.
    key: String,
    /// The name the service's saved key has in the keychain, if it has one.
    key_ref: Option<String>,
}

impl ImageForm {
    fn new() -> ImageForm {
        ImageForm {
            index: None,
            name: String::new(),
            kind: crate::imagery::Kind::ALL[0].name().to_owned(),
            base_url: String::new(),
            model: String::new(),
            key: String::new(),
            key_ref: None,
        }
    }

    fn of(index: usize, service: &ImageService) -> ImageForm {
        ImageForm {
            index: Some(index),
            name: service.name.clone(),
            kind: service.kind.clone(),
            base_url: service.base_url.clone(),
            model: service.model.clone(),
            key: String::new(),
            key_ref: service.key_ref.clone(),
        }
    }

    /// The service the form describes, when it has a name no other saved service has.
    fn service(&self, images: &Images) -> Option<ImageService> {
        let name = self.name.trim();
        let taken = images.services.iter().enumerate().any(|(index, other)| Some(index) != self.index && other.name.eq_ignore_ascii_case(name));
        (!name.is_empty() && !taken).then(|| ImageService {
            name: name.to_owned(),
            kind: self.kind.clone(),
            base_url: self.base_url.trim().to_owned(),
            model: self.model.trim().to_owned(),
            key_ref: self.key_ref.clone(),
        })
    }
}

/// Saves the form's `service` in the settings and a newly typed key in the keychain.
///
/// # Errors
/// A sentence for the user when the keychain refused; the settings are then left as they were.
// Agent: WRITES the keychain first, the settings only after that worked.
fn save_image_service(form: &mut ImageForm, mut service: ImageService, prefs: &mut Prefs, vault: &dyn Vault) -> Result<(), String> {
    let typed = form.key.trim();
    if !typed.is_empty() {
        let name = service.key_ref.clone().unwrap_or_else(secrets::new_name);
        vault.put(&name, &SecretString::new(typed)).map_err(|e| format!("The key couldn't be saved in the keychain: {e}"))?;
        service.key_ref = Some(name);
    }
    form.key.clear();
    match form.index.and_then(|index| prefs.images.services.get_mut(index)) {
        Some(saved) => {
            // Human: the default follows a service that was renamed.
            if prefs.images.default == saved.name {
                prefs.images.default.clone_from(&service.name);
            }
            *saved = service;
        }
        None => prefs.images.services.push(service),
    }
    Ok(())
}

/// Removes the saved image service at `index` and its key.
///
/// # Errors
/// A sentence for the user when the keychain refused; the service then stays.
fn remove_image_service(index: usize, prefs: &mut Prefs, vault: &dyn Vault) -> Result<(), String> {
    let Some(service) = prefs.images.services.get(index) else { return Ok(()) };
    if let Some(name) = &service.key_ref {
        vault.forget(name).map_err(|e| format!("The key couldn't be removed from the keychain: {e}"))?;
    }
    if prefs.images.default == service.name {
        prefs.images.default.clear();
    }
    prefs.images.services.remove(index);
    Ok(())
}
```

The borrow of `saved` and `prefs.images.default` in `save_image_service` overlap; if the compiler refuses, read `let renamed = prefs.images.services.get(index).is_some_and(|saved| saved.name == prefs.images.default);` before the `match` and set the default after it.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p pencraft settings`
Expected: all pass.

- [ ] **Step 5: The page**

a) `enum Page`: add after `Agents`:

```rust
    /// Image model services.
    Images,
```

b) `PAGES`: make it `[(Page, &str, &str); 6]` and add after the Agents line:

```rust
    (Page::Images, "image", "Images"),
```

c) `struct Settings`: add after `form`:

```rust
    /// The image service form, when it is open on the Images page.
    image_form: Option<ImageForm>,
```

Set it to `None` in `Default`, in `open_named`, in `open_general`, and where `pages()` sets `self.form = None` on a click. In `show`, change the scroll area's salt to `("settings-page", self.page, self.form.is_some() || self.image_form.is_some())` and add the arm:

```rust
                                Page::Images => self.images(ui, prefs, palette, &mut notice),
```

d) Add to `impl Settings`, after `fn agents`:

```rust
    /// The Images page: the image model services, or the form for one.
    fn images(&mut self, ui: &mut Ui, prefs: &mut Prefs, palette: Palette, notice: &mut Option<String>) {
        if let Some(form) = &mut self.image_form {
            if image_form(ui, form, prefs, self.vault.as_ref(), palette, notice) {
                self.image_form = None;
            }
            return;
        }
        title(ui, "Images");
        ui.add_space(8.0);
        let about = "Image model services make pictures from a description. Add the ones you have an API key for; each picture is paid for on that key.";
        ui.scope(|ui| {
            ui.set_max_width(450.0);
            ui.add(egui::Label::new(RichText::new(about).size(SIDEBAR_TEXT_SIZE + 0.5).color(palette.muted)).wrap());
        });
        section(ui, "Services");
        ui.spacing_mut().item_spacing.y = 7.5;
        let saved: Vec<(String, String)> = prefs
            .images
            .services
            .iter()
            .map(|s| {
                let kind = crate::imagery::Kind::named(&s.kind);
                let model = if s.model.is_empty() { kind.map_or("", crate::imagery::Kind::model) } else { s.model.as_str() };
                let key = if s.key_ref.is_some() { "" } else { " · no key yet" };
                (s.name.clone(), format!("{} · {model}{key}", s.kind))
            })
            .collect();
        for (index, (name, about)) in saved.iter().enumerate() {
            if provider_card(ui, name, about, false, None, palette).clicked()
                && let Some(service) = prefs.images.services.get(index)
            {
                self.image_form = Some(ImageForm::of(index, service));
            }
        }
        if provider_card(ui, "Add image service", "", false, None, palette).clicked() {
            self.image_form = Some(ImageForm::new());
        }
        section(ui, "Use");
        let names: Vec<String> = std::iter::once("First that works".to_owned()).chain(prefs.images.services.iter().map(|s| s.name.clone())).collect();
        let options: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut chosen = if prefs.images.default.is_empty() { names[0].clone() } else { prefs.images.default.clone() };
        row(ui, "Default service", None, palette, |ui| choice(ui, "image-default", &mut chosen, &options, palette));
        prefs.images.default = if chosen == names[0] { String::new() } else { chosen };
        divider(ui, palette);
        let cost = "AI models may make pictures with the generate_image tool. Each one costs money on your key. The Generate Image dialog works either way.";
        row(ui, "Let the assistant generate pictures", Some(cost), palette, |ui| switch(ui, &mut prefs.images.assistant, palette));
    }
```

e) Add as a free function after `provider_form`:

```rust
/// The image service form. Returns `true` when it is finished: the service was saved or removed.
fn image_form(ui: &mut Ui, form: &mut ImageForm, prefs: &mut Prefs, vault: &dyn Vault, palette: Palette, notice: &mut Option<String>) -> bool {
    let mut finished = false;
    title(ui, if form.index.is_some() { "Image service" } else { "Add image service" });
    let kind = crate::imagery::Kind::named(&form.kind);
    section(ui, "Service");
    ui.columns(2, |columns| {
        caption(&mut columns[0], "Name");
        field(&mut columns[0], &mut form.name, "My image service", false, false, palette);
        caption(&mut columns[1], "Kind");
        let kinds: Vec<&str> = crate::imagery::Kind::ALL.iter().map(|kind| kind.name()).collect();
        wide_choice(&mut columns[1], &mut form.kind, &kinds, palette);
    });
    ui.add_space(8.0);
    caption(ui, "Model");
    field(ui, &mut form.model, kind.map_or("", crate::imagery::Kind::model), true, false, palette);
    ui.add_space(8.0);
    caption(ui, "Address");
    field(ui, &mut form.base_url, kind.map_or("", crate::imagery::Kind::base), true, false, palette);
    if unprotected(&form.base_url) {
        ui.add_space(2.0);
        ui.label(RichText::new("This address isn't encrypted: the key and your descriptions could be read on the way.").color(CHECKING));
    }
    ui.add_space(2.0);
    ui.label(RichText::new("Leave model and address empty for the usual ones. \"OpenAI Images\" also fits other services that speak the same API.").color(palette.muted));
    section(ui, "API key");
    let saved = form.key_ref.is_some();
    field(ui, &mut form.key, if saved { "Saved - type a new key to replace it" } else { "Needed to make pictures" }, true, true, palette);
    ui.add_space(2.0);
    ui.label(RichText::new("The key is kept in your system's keychain.").color(palette.muted));
    ui.add_space(16.0);
    let service = form.service(&prefs.images);
    ui.horizontal(|ui| {
        let label = if form.index.is_some() { "Save service" } else { "Add service" };
        let save = egui::Button::new(RichText::new(label).color(palette.primary_text)).fill(palette.primary).corner_radius(CHIP_RADIUS).min_size(vec2(92.0, FIELD_HEIGHT));
        if ui.add_enabled(service.is_some(), save).clicked()
            && let Some(service) = service
        {
            match save_image_service(form, service, prefs, vault) {
                Ok(()) => finished = true,
                Err(why) => *notice = Some(why),
            }
        }
        if let Some(index) = form.index
            && ui.add(egui::Button::new("Remove").corner_radius(CHIP_RADIUS).min_size(vec2(72.0, FIELD_HEIGHT))).clicked()
        {
            match remove_image_service(index, prefs, vault) {
                Ok(()) => finished = true,
                Err(why) => *notice = Some(why),
            }
        }
        if ui.add(egui::Button::new("Cancel").corner_radius(CHIP_RADIUS).min_size(vec2(72.0, FIELD_HEIGHT))).clicked() {
            finished = true;
        }
    });
    finished
}
```

`provider_card` with `state: None` draws no status; with an empty second text it draws the name alone (as "Add custom provider" does).

- [ ] **Step 6: Look at it**

Run: `cargo run --release -- --settings general --screenshot ../research_notes/imagery/settings.png` (the app's own picture mode; see `Launch::settings` in `src/app.rs`). `open_named` only knows `general`, `agents` and `provider`: add `images` to it so the page can be pictured:

```rust
        self.page = match name {
            "general" => Page::General,
            "images" => Page::Images,
            _ => Page::Agents,
        };
```

Then run with `--settings images` and read the PNG. Expected: the page list shows "Images" between Agents and Agent names; the page shows the text, "Add image service", "Default service", and the switch, with nothing overlapping. Fix spacing with `ui.add_space` only; no new colours or sizes outside `theme.rs`.

- [ ] **Step 7: Run the gate**

Run: `bash scripts/check.sh`
Expected: `check: all passed`.

- [ ] **Step 8: Checkpoint**

Commit only if the user asked (`git add app/src/settings.rs`).

---

### Task 10: The "Generate Image…" dialog

**Files:**
- Create: `src/imagery/dialog.rs`
- Modify: `src/imagery.rs` (`pub mod dialog;` first in the list), `src/palette.rs`, `src/menus.rs`, `src/app.rs`, `src/app/pictures.rs`

- [ ] **Step 1: Write the failing tests**

Create `src/imagery/dialog.rs` with only:

```rust
//! The "Generate Image…" window: a description, a service, a shape and a size, and a button.

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_dialog_asks_in_the_tools_own_form() {
        let mut dialog = Dialog::default();
        dialog.open_for(Some("hero".to_owned()), &["Studio".to_owned(), "Gemini".to_owned()], "Gemini");
        assert!(dialog.open && dialog.service == "Gemini" && dialog.fill && dialog.target.as_deref() == Some("hero"));
        dialog.prompt = "A red fox".to_owned();
        dialog.aspect = "16:9".to_owned();
        dialog.large = true;
        assert_eq!(dialog.asked(), json!({ "prompt": "A red fox", "service": "Gemini", "aspect": "16:9", "size": "large" }));
    }

    #[test]
    fn opening_again_keeps_the_description_and_forgets_the_last_problem() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &["Studio".to_owned()], "");
        assert!(dialog.service == "Studio" && !dialog.fill && dialog.aspect == "1:1");
        dialog.prompt = "A fox".to_owned();
        dialog.problem = Some("no".to_owned());
        dialog.open = false;
        dialog.open_for(None, &["Studio".to_owned()], "");
        assert!(dialog.prompt == "A fox" && dialog.problem.is_none());
        dialog.open_for(None, &[], "");
        assert!(dialog.service.is_empty());
    }
}
```

Add `pub mod dialog;` to `src/imagery.rs`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p pencraft imagery::dialog`
Expected: does not compile.

- [ ] **Step 3: Write the dialog**

Put above the tests in `src/imagery/dialog.rs`:

```rust
use egui::{Color32, RichText, Stroke, vec2};
use serde_json::{Value, json};

use super::service::ASPECTS;
use crate::theme::{CHIP_RADIUS, FIELD_HEIGHT, Palette, SETTINGS_RADIUS};

/// The dialog's width, and the height of its description field in lines.
const WIDTH: f32 = 440.0;
/// See [`WIDTH`].
const LINES: usize = 4;

/// What the person did in the dialog this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Did {
    /// Nothing that concerns the app.
    Nothing,
    /// Pressed "Generate".
    Generate,
    /// Pressed "Cancel" or closed the window while a picture was being made.
    Cancel,
}

/// The dialog's state.
#[derive(Default)]
pub struct Dialog {
    /// Whether the window shows.
    pub open: bool,
    /// The description.
    pub prompt: String,
    /// The chosen service's name; empty when there is none.
    pub service: String,
    /// The chosen shape, one of [`ASPECTS`].
    pub aspect: String,
    /// About 2048 pixels instead of about 1024.
    pub large: bool,
    /// The layer that was selected when the dialog opened and can take a picture.
    pub target: Option<String>,
    /// Give the picture to [`Dialog::target`] instead of adding a layer.
    pub fill: bool,
    /// A picture is being made.
    pub busy: bool,
    /// What went wrong the last time.
    pub problem: Option<String>,
    services: Vec<String>,
}

impl Dialog {
    /// Opens the dialog. `target` is the selected layer when it can take a picture; `services` are the ones
    /// that work and `default` the settings' choice among them.
    pub fn open_for(&mut self, target: Option<String>, services: &[String], default: &str) {
        self.open = true;
        self.problem = None;
        self.busy = false;
        self.fill = target.is_some();
        self.target = target;
        self.services = services.to_vec();
        if !services.contains(&self.service) {
            self.service = services.iter().find(|name| name.as_str() == default).or_else(|| services.first()).cloned().unwrap_or_default();
        }
        if !ASPECTS.contains(&self.aspect.as_str()) {
            self.aspect = ASPECTS[0].to_owned();
        }
    }

    /// What the dialog asks for, in the form of a `generate_image` call's arguments.
    #[must_use]
    pub fn asked(&self) -> Value {
        json!({ "prompt": self.prompt, "service": self.service, "aspect": self.aspect, "size": if self.large { "large" } else { "normal" } })
    }

    /// Draws the window when it is open.
    // Agent: MAIN-THREAD.
    pub fn show(&mut self, ctx: &egui::Context, palette: Palette) -> Did {
        if !self.open {
            return Did::Nothing;
        }
        let mut did = Did::Nothing;
        let frame = egui::Frame::new().fill(palette.island).stroke(Stroke::new(1.0, palette.outline)).corner_radius(SETTINGS_RADIUS).inner_margin(20);
        let id = egui::Id::unique("generate-image");
        let modal = egui::Modal::new(id).area(egui::Modal::default_area(id).fade_in(false)).frame(frame).backdrop_color(Color32::from_black_alpha(140)).show(ctx, |ui| {
            ui.set_width(WIDTH);
            ui.label(RichText::new("Generate image").size(15.0).color(palette.foreground));
            ui.add_space(10.0);
            if self.services.is_empty() {
                ui.add(egui::Label::new(RichText::new(super::NO_SERVICE).color(palette.muted)).wrap());
                ui.add_space(12.0);
                if ui.button("Close").clicked() {
                    self.open = false;
                }
                return;
            }
            ui.add_enabled(
                !self.busy,
                egui::TextEdit::multiline(&mut self.prompt).hint_text("What the picture shows: subject, setting, light, style").desired_rows(LINES).desired_width(f32::INFINITY),
            );
            ui.add_space(10.0);
            ui.add_enabled_ui(!self.busy, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if self.services.len() > 1 {
                        egui::ComboBox::from_id_salt("generate-service").selected_text(self.service.clone()).show_ui(ui, |ui| {
                            for name in &self.services {
                                ui.selectable_value(&mut self.service, name.clone(), name);
                            }
                        });
                    }
                    egui::ComboBox::from_id_salt("generate-aspect").selected_text(self.aspect.clone()).show_ui(ui, |ui| {
                        for aspect in ASPECTS {
                            ui.selectable_value(&mut self.aspect, aspect.to_owned(), aspect);
                        }
                    });
                    ui.checkbox(&mut self.large, "Large");
                });
                if self.target.is_some() {
                    ui.add_space(4.0);
                    ui.checkbox(&mut self.fill, "Use as the fill of the selection");
                }
            });
            if let Some(problem) = &self.problem {
                ui.add_space(8.0);
                ui.add(egui::Label::new(RichText::new(problem).color(palette.danger)).wrap());
            }
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                let label = if self.busy { "Generating…" } else { "Generate" };
                let go = egui::Button::new(RichText::new(label).color(palette.primary_text)).fill(palette.primary).corner_radius(CHIP_RADIUS).min_size(vec2(96.0, FIELD_HEIGHT));
                if ui.add_enabled(!self.busy && !self.prompt.trim().is_empty(), go).clicked() {
                    did = Did::Generate;
                }
                if ui.add(egui::Button::new("Cancel").corner_radius(CHIP_RADIUS).min_size(vec2(72.0, FIELD_HEIGHT))).clicked() {
                    did = Did::Cancel;
                }
                if self.busy {
                    ui.spinner();
                    ui.label(RichText::new("This can take up to a minute.").color(palette.muted));
                }
            });
        });
        if modal.should_close() && did == Did::Nothing {
            did = Did::Cancel;
        }
        did
    }
}
```

Names to check against `src/theme.rs` before compiling, and what to do when one is missing: `SETTINGS_RADIUS`, `CHIP_RADIUS`, `FIELD_HEIGHT` are used by `settings.rs` (import them from wherever it does). `palette.danger` may have another name (look at `struct Palette` at `src/theme.rs:214` for the colour used for errors; `settings.rs` uses a `CHECKING` colour for warnings). The title's `15.0` must become a size constant from `theme.rs` (`settings.rs` draws "Settings" with `FontId::proportional(15.0)`; if there is no constant, add `pub const DIALOG_TITLE_SIZE: f32 = 15.0;` to `theme.rs` and use it in both places). `Modal::should_close` is the fork's name for "escape or backdrop click"; if it differs, use what `settings.rs` does after its own modal (`src/settings.rs:338-341`).

- [ ] **Step 4: Run the tests**

Run: `cargo test -p pencraft imagery::dialog`
Expected: 2 passed.

- [ ] **Step 5: The command, the menu and the action**

a) `src/palette.rs`, `enum Command`: add after `Import`:

```rust
    /// Open the "Generate Image…" dialog.
    GenerateImage,
```

and in the list of actions after the `Command::Import` line:

```rust
    action(Command::GenerateImage, "image-plus", "Generate image…", "Edit", &[], "ai picture photo illustration make create prompt"),
```

(use the icon name Task 8 settled on).

b) `src/menus.rs`: after `("file.import", Command::Import),` add `("file.generate", Command::GenerateImage),`; after the `item("file.import", ..)` line add:

```rust
            item("file.generate", "Generate Image…", "", doc),
```

and add `"Generate Image…",` after `"Import Image/SVG/Figma…",` in the test that lists the File menu (about line 234).

c) `src/app.rs`: in the long list of app-level commands at about line 1554 add `| Command::GenerateImage` after `| Command::Import`; in `App::run`'s `match` after the `Command::Import` arm add:

```rust
            Command::GenerateImage => {
                if self.active.is_some() {
                    self.open_picture_dialog();
                }
            }
```

add to `struct App` after `picturing`:

```rust
    /// The "Generate Image…" dialog and the picture it is having made.
    picture_dialog: crate::imagery::dialog::Dialog,
    dialog_job: Option<(crate::imagery::jobs::Job, usize)>,
```

with `picture_dialog: crate::imagery::dialog::Dialog::default(), dialog_job: None,` in `App::new`; and right after the line `let notice = self.settings.show(ctx, &mut prefs, dark);` block (about line 1297, once `prefs` has been written back) add:

```rust
        self.show_picture_dialog(ctx);
```

- [ ] **Step 6: The app's side of the dialog**

Add to `impl App` in `src/app/pictures.rs`:

```rust
    /// Opens the dialog for the document in front.
    pub(super) fn open_picture_dialog(&mut self) {
        let (ready, default) = self.image_services();
        let names: Vec<String> = ready.iter().map(|r| r.name.clone()).collect();
        // Human: with exactly one layer selected that can take a picture, the dialog offers to fill it.
        let target = self.editor().and_then(|editor| match editor.selection.as_slice() {
            [only] => takes_picture(editor, only).is_ok().then(|| only.clone()),
            _ => None,
        });
        self.picture_dialog.open_for(target, &names, &default);
    }

    /// Draws the dialog, starts its picture, and places the picture when it is there.
    // Agent: MAIN-THREAD; called once a frame.
    pub(super) fn show_picture_dialog(&mut self, ctx: &egui::Context) {
        use crate::imagery::dialog::Did;
        let palette = crate::theme::palette(self.dark());
        match self.picture_dialog.show(ctx, palette) {
            Did::Nothing => {}
            Did::Cancel => {
                // Human: a picture already asked for is not waited for any longer; if the service still makes
                // it, the file stays in `images/` unused.
                self.dialog_job = None;
                self.picture_dialog.open = false;
                self.picture_dialog.busy = false;
            }
            Did::Generate => match self.dialog_order() {
                Ok((order, tab)) => {
                    let wake = ctx.clone();
                    self.dialog_job = Some((Job::start(move || jobs::work(&order), move || wake.request_repaint()), tab));
                    self.picture_dialog.busy = true;
                    self.picture_dialog.problem = None;
                }
                Err(why) => self.picture_dialog.problem = Some(why),
            },
        }
        let Some((job, tab)) = &self.dialog_job else { return };
        let Some(result) = job.poll() else {
            ctx.request_repaint_after(LOOK_AGAIN);
            return;
        };
        let tab = *tab;
        self.dialog_job = None;
        self.picture_dialog.busy = false;
        match result.and_then(|done| self.place_picture(tab, &done)) {
            Ok(()) => self.picture_dialog.open = false,
            Err(why) => self.picture_dialog.problem = Some(why),
        }
    }

    /// What the dialog asks for, and the tab it is for.
    fn dialog_order(&self) -> Result<(Order, usize), String> {
        let tab = self.active.ok_or("No document is open.")?;
        let editor = self.tabs.get(tab).ok_or("The document was closed.")?;
        let asked = self.picture_dialog.asked();
        let (ready, default) = self.image_services();
        let chosen = imagery::pick(&ready, asked.get("service").and_then(Value::as_str), &default)?.clone();
        let request = imagery::request(&asked, &chosen)?;
        Ok((Order { ready: chosen, request, document: editor.path.clone() }, tab))
    }

    /// Puts the dialog's finished picture into the document in tab `tab`: onto the layer it was opened for,
    /// or as a new layer in the middle of the view.
    fn place_picture(&mut self, tab: usize, done: &Done) -> Result<(), String> {
        let view = self.canvas_view;
        let target = self.picture_dialog.target.clone().filter(|_| self.picture_dialog.fill);
        let editor = self.tabs.get_mut(tab).ok_or("The document was closed; the picture is in its images folder.")?;
        match target {
            Some(layer) => fill_with(editor, &layer, &done.saved.url),
            None => {
                let visible = editor.visible_area(view.width(), view.height());
                editor.import_file(&done.saved.path, visible)
            }
        }
    }
```

After `fill_with`, the editor must lay itself out again and count as changed. Look at how `App::edit_tool` leaves the editor after `agent::edit` (it calls `editor.refresh(..)` with the text measure) and at whether `agent::edit` already marks the document dirty and in need of layout (`grep -n "needs_layout\|dirty" src/agent.rs src/editor.rs`); if `agent::edit` does not, set what `Editor::import_file`'s path sets (it goes through `checkpoint` and `edit::insert`). Verify in Step 8 that the picture shows without touching the canvas and that the tab shows the unsaved mark.

- [ ] **Step 7: Run the gate**

Run: `bash scripts/check.sh`
Expected: `check: all passed`.

- [ ] **Step 8: Try it for real**

`cargo run --release`, with a key in the environment or a service added in Settings → Images. Check each and say in your report which you could run:

1. File → Generate Image… with nothing selected: describe a picture, Generate. A spinner shows; within a minute the dialog closes and a new rectangle with the picture sits in the middle of the view, selected. Ctrl+Z removes it.
2. Select one rectangle first: the checkbox "Use as the fill of the selection" is there and on; the picture lands on the rectangle; Ctrl+Z gives the old fill back.
3. Take the key away (rename the service's key in Settings, or unset the variable and restart): the dialog says no image service is set up.
4. A wrong key: the dialog stays open, the description is kept, and the sentence names the service and Settings → Images.
5. Cancel while it generates: the dialog closes and nothing is placed.
6. The action search (Ctrl+P) finds "Generate image…".

- [ ] **Step 9: Checkpoint**

Commit only if the user asked (`git add app/src`).

---

### Task 11: Documentation and the last check

**Files:**
- Modify: `AI.md`, `MECHANICS.md`, `src/secrets.rs:7`, `src/images.rs:1-4`

- [ ] **Step 1: `AI.md`**

Add to the tools table after the `export_layers` row:

```markdown
| `generate_image` | Has an image model make a picture from a description and gives back the file to use in an image fill, or puts it on a layer. Needs an image service (see below) |
```

Add a section before `## Skills`:

```markdown
## Pictures

Pencraft can have an image model make a picture: File → Generate Image…, or ask the assistant ("put a photo
of a red fox on this card"). It needs an image service: add one with its API key in Settings → Images
(Google Gemini, OpenAI Images, or another service that speaks the OpenAI Images API), or start Pencraft with
`GEMINI_API_KEY` or `OPENAI_API_KEY` set. The model and the address can be changed there.

A picture is saved as a file in an `images` folder beside the document, with a line in `images/credits.json`
saying which service and model made it from which description. A document that has no file yet keeps its
pictures in `~/.pencraft/images` until then.
```

Add to `## Safety`:

```markdown
Making a picture sends its description, and nothing else of the document, to the image service you chose,
and is paid for on your key. Settings → Images → "Let the assistant generate pictures" stops AI models from
doing it; the dialog still works.
```

- [ ] **Step 2: `MECHANICS.md`**

Find the section on images or fills (`grep -n -i "image" MECHANICS.md`) and add, in that file's own style, the numbers this stage introduced: a job's limit of 100 seconds (under the link's 120), the answer limit of 64 MB, the copy shown to a model at most 1024 px a side, normal and large pictures at about 1024 and 2048 px on the long side, and the `images/` folder with `credits.json`. If `mechanics.json` mirrors such numbers (`grep -n "patience\|seconds" mechanics.json`), add them there in the same form; if it holds only canvas mechanics, leave it.

- [ ] **Step 3: Two comments that are now wrong**

`src/secrets.rs` line 7: change "A key is read back only by the chat, when it calls the provider the key belongs to." to:

```rust
//! A key is read back only by the chat and by the maker of pictures ([`crate::imagery`]), when they call the
//! provider the key belongs to.
```

`src/images.rs` lines 3-4 stay true (a fill's `url` is still a file and nothing is fetched to show it); add one line after them:

```rust
//! Pictures an image service made are files like any other by the time they get here ([`crate::imagery`]).
```

- [ ] **Step 4: The whole gate, once more**

Run: `bash scripts/check.sh`
Expected: `check: all passed`.

- [ ] **Step 5: Report**

Say, plainly: which services were verified against the live API (Tasks 3 and 4, Step 1) and which only against the documented shapes; which of the by-hand checks in Tasks 8 and 10 were run and what they showed; anything in this plan that had to be changed to fit the code, and why. Commit only if the user asked.
