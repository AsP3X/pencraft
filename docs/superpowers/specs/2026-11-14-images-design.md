# Pictures in Pencraft: generate, edit, search

Date: 2026-11-14. Status: design, waiting for review. Nothing here is built yet.

## Goal

Pencraft can today only show pictures that already lie on disk. It should be able to

1. **generate** a picture from a description,
2. **edit** a picture, both by a described change (an image model) and by hand (crop, rotate, adjust),
3. **search** for pictures online and place one,

and each of these should work two ways: for the **assistant** (as tools, for the chat inside Pencraft and for
every MCP client) and for the **person** (in the editor's own interface).

Image model services are configurable: the person adds the ones they have a key for, and the assistant picks
among those that are added and working.

## The four stages

This is too much for one build. It is split into four stages; each gets its own plan and is finished (tool,
interface, tests, docs) before the next starts. **This document specifies stage 1 in full** and fixes the
decisions the later stages depend on; stages 2 to 4 get a short spec of their own when their turn comes.

| Stage | What | Assistant tool | In the editor |
|---|---|---|---|
| 1 | Generate, and the shared foundation | `generate_image` | "Generate Image…" dialog; Settings → Images |
| 2 | Edit by description | `edit_image` | A prompt field on a selected image fill |
| 3 | Search online | `search_images` | A search panel with a grid of results |
| 4 | Edit by hand | `adjust_image` | Crop, rotate/flip, brightness/contrast/saturation |

Generation is first because it forces the foundation (services, keys, saving, credits, long-running tool
calls) that the other three reuse.

## Rules every stage follows

- **A picture is a file.** A generated, edited or downloaded picture is written to disk and a layer's image
  fill names it (`{"type":"image","url":"./images/…"}`), exactly as an imported picture is today. The `.pen`
  format does not change, and a document made here opens anywhere the format is read.
- **Where the file goes.** Into `images/` beside the `.pen` file, named relative to the document
  (`./images/name.png`). A document that has no file yet gets its pictures in Pencraft's data folder under
  a full path. (Moving those beside the document on first save is out of scope; see "Not in this stage".)
- **Nothing is overwritten.** Every result is a new file. An edit changes a layer's fill to the new file,
  which is one undo step, and the picture before it is still on disk. This also sidesteps `ImageStore` not
  reloading a file that changed.
- **Where a picture came from is kept.** `images/credits.json` holds one entry per file: how it was made
  (`generated`, `edited`, `found`), the service and model, the prompt or instruction, the file it was made
  from, and for found pictures the source, author, licence and page address. Written with craft-atomic.
- **Keys are in the keychain**, through `secrets::Vault`, never in `config.toml`; the same as chat providers.
- **Network work never runs on the main thread.** Blocking `ureq` on a worker thread, as the chat does.
- **No new dependencies.** `ureq`, `image`, `serde_json`, `keyring` and craft-atomic cover it. A base64
  decoder is written beside the encoder in `agent.rs`; a multipart body (stage 2, if needed) by hand.

## Stage 1: generate

### Services

A service is one account at one image model provider. Stage 1 ships two kinds:

| Kind | Endpoint | Auth | Default model |
|---|---|---|---|
| Google Gemini | `POST {base}/v1beta/interactions` | `x-goog-api-key` | `gemini-3.1-flash-lite-image` ("Nano Banana 2 Lite") |
| OpenAI Images | `POST {base}/v1/images/generations` | `Authorization: Bearer` | `gpt-image-2.5-sunburst` |

The model is a text field with the default filled in, so a newer model needs no new Pencraft. The address is
editable too, which lets any service that speaks the OpenAI Images API be added as kind "OpenAI Images".

Request and answer shapes, as the providers' documentation gave them on 2026-11-14:

- **Gemini**: body `{"model", "input": [{"type":"text","text":…}], "response_format": {"type":"image",
  "mime_type":"image/png", "aspect_ratio":"16:9", "image_size":"1K"|"2K"}}`. The picture is the base64 `data`
  of the block with `type == "image"` in the `content` of a step with `type == "model_output"`.
- **OpenAI**: body `{"model", "prompt", "size", "quality":"auto", "background":"auto"|"transparent",
  "output_format":"png", "n":1}`. The picture is `data[0].b64_json`.

The tool's `aspect` and `size` are turned into each service's own terms: for Gemini `aspect_ratio` as given
and `image_size` "1K" (normal) or "2K" (large); for OpenAI a `size` of `WIDTHxHEIGHT` with the long side 1024
(normal) or 2048 (large) and the short side from the aspect, rounded to a multiple of 16.

Neither provider's pages documented the shape of an error or a refusal. The plan's first task for each
service is therefore a live call with a real key (one that works, one with a bad key, one prompt that is
refused) whose answers become the test fixtures. Until then the rule is: a status other than 2xx is a
failure and the service's own message is shown; a 2xx answer without a picture is a refusal and the text the
model sent with it is shown.

**ElevenLabs and other kinds** come after stage 1 as one more implementation of the same interface each,
once their public API has been read. They are not promised here.

### Code

A new module `src/imagery.rs` with small files, none of which knows egui except the dialog:

| File | Does | Depends on |
|---|---|---|
| `imagery/service.rs` | `Request` (prompt, model, aspect, size, transparent, source pictures: empty in stage 1), `Made` (bytes, kind), `Failure` (refused, bad key, limit, network, other), and the `Service` trait: `fn make(&self, request, key, agent) -> Result<Made, Failure>` | `ureq`, `SecretString` |
| `imagery/gemini.rs`, `imagery/openai.rs` | One `Service` each: build the body, send, read the answer. Body building and answer reading are plain functions on JSON, tested without a network | `service.rs` |
| `imagery/store.rs` | Checks the bytes decode as a picture, names the file from the prompt (`a-red-fox-3f9a.png`), writes it, adds the entry to `credits.json`, returns the `url` for a fill and the pixel size | `image`, craft-atomic |
| `imagery/jobs.rs` | Starts a request on a worker thread and hands the result back through a channel; the app polls it from `logic`. A job has a time limit of 100 seconds | `service.rs`, `store.rs` |
| `imagery/dialog.rs` | The "Generate Image…" window | egui, `theme.rs` |

`prefs.rs` gets an `[images]` section: `services: Vec<ImageService>` (`name`, `kind`, `base_url`, `model`,
`key_ref`), `default: String` (a service's name), and `assistant: bool` (see "Cost"). The placeholder
`generation.image_provider` ("Nano Banana Lite", which nothing reads) is removed; if craft-settings refuses
unknown keys in an old `config.toml`, the schema version goes to 2 with a migration that drops the key.

Started with `GEMINI_API_KEY` or `OPENAI_API_KEY` set, Pencraft offers that service without anything being
added, as it does for `ANTHROPIC_API_KEY` in the chat.

### The assistant's tool

```
generate_image
  prompt       required   What the picture shows.
  document                As in the other tools.
  service                 A service's name from get_state; the default one without it.
  aspect                  "1:1" (default) | "16:9" | "9:16" | "4:3" | "3:4" | "3:2" | "2:3"
  size                    "normal" (about 1024 px on the long side, default) | "large" (about 2048 px)
  transparent             true asks for a transparent background; an error on a service that can't.
  layer                   A rectangle, ellipse or frame to give the picture to as its fill (one undo step).
```

It answers with the fill's `url`, the pixel size, the service and model used, and the picture itself (made
smaller to at most 1024 px a side) so the model sees what it got. Without `layer` it changes nothing in the
document; the model places the picture with `edit_layers`, as it would any file.

`get_state` gains `imageServices`: for each working service its `name`, `kind`, `model`, whether it is the
default, and whether it can do transparent backgrounds. With none, `generate_image` fails with a sentence
that says where to add one (Settings → Images).

**Long calls.** Tools are answered on the main thread in one go today; a picture takes 5 to 60 seconds. A
`generate_image` call starts a job and is parked (as calls already are in `App::waiting` while the user
drags), and is answered when its job finishes. The job's 100-second limit stays under the link's 120-second
patience (`link.rs`), so the model always gets an answer from the tool and not a dropped call.

The chat shows the step as "Generating a picture" / "Generated a picture" / "Couldn't generate the picture"
(`chat/steps.rs`), with the prompt's first words as its detail.

The guide (`agent.rs` `GUIDE`) gets a short "Pictures" section: generate a photograph or an illustration
when the user asks for one or a design plainly needs one (a hero, a product shot, an avatar); never generate
interface, text, icons or logos, which are drawn as layers; describe subject, setting, light and framing;
one picture per need, looked at before another is asked for.

### In the editor

- **Menu**: "Generate Image…" beside "Import Image/SVG/Figma…", and in the command palette.
- **Dialog**: a description field, the service (when more than one), aspect, size, and "Generate". While it
  works the button shows progress and "Cancel" drops the result. A failure is shown in the dialog in the
  service's words, with the description kept. On success the dialog closes and the picture is placed the way
  an imported file is (`Editor::import_file`): a rectangle its own size in the middle of the view, selected.
  If exactly one rectangle, ellipse or frame was selected when the dialog opened, a checkbox "Use as the
  fill of the selection" (on by default) gives it the picture instead.
- **Settings → Images**: the list of services (add, change, remove; kind, name, address, model, key), which
  one is the default, and the switch below. Built like the provider list under Agents.

All colours and sizes come from `theme.rs`.

### Cost and privacy

- A generated picture costs money on the person's own key. The switch "Let the assistant generate pictures"
  (on by default) turns `generate_image` off for models while leaving the dialog working.
- Stage 1 sends a service the description and nothing else: no layers, no screenshot, no file.
- `AI.md` gets the new tool in its table and a paragraph on both points under "Safety".

### Failures

| What happens | What the person or model is told |
|---|---|
| No service added | Where to add one |
| The key is missing or refused | Which service, and to check its key in Settings → Images |
| The service refuses the description | The service's reason, unchanged |
| Rate or quota limit | The service's message; nothing is retried automatically |
| No answer within 100 seconds, or no network | That it took too long or couldn't be reached |
| The bytes aren't a picture | That the service sent something that isn't a picture; nothing is written |
| `images/` can't be written | The folder and the system's reason |

None of these panics, and none leaves a half-written file or a half-made undo step.

### Tests

- Each service: the body built for a set of requests, and the answer read from fixtures (a picture, a
  refusal, an error), with no network.
- `store.rs`: naming, no overwriting, the relative `url` on Windows and elsewhere, `credits.json` read back,
  a document without a file.
- The tool, with a fake `Service`: success with and without `layer` (one undo step, undone by ⌘Z), every row
  of the failure table, the assistant switch, a parked call answered when its job ends.
- One `#[ignore]` test per service that makes a real picture with a real key, run by hand.
- `scripts/check.sh` passes.

### Not in this stage

- Editing, searching and hand adjustments (stages 2 to 4).
- Several pictures per call, streaming previews, and a history of generated pictures.
- Moving the pictures of an unsaved document beside it on first save.
- ElevenLabs and further kinds of service.

## What stages 2 to 4 are already decided to be

- **Stage 2, edit by description.** `Request` carries source pictures; Gemini takes them as
  `{"type":"image","mime_type","data"}` items in `input`, OpenAI through `/v1/images/edits`. `edit_image`
  takes a layer or a file and an instruction, writes a new file and, for a layer, swaps its fill. In the
  editor: a prompt field in the fill's panel. This is the first stage that sends a picture of the person's
  to a service; the prompt field says so, naming the service.
- **Stage 3, search.** Sources behind one interface: Openverse (no key), Unsplash and Pexels (free keys),
  and a general web search (Brave is the likely one; needs a key). `search_images` gives thumbnails with
  author, licence and page; placing one downloads it into `images/` and records the credit. Results from the
  general web search are labelled "licence unknown" for the person and the model alike.
- **Stage 4, edit by hand.** Crop, rotate, flip, brightness, contrast and saturation, worked out locally
  with the `image` crate into a new file. No service, no key.
