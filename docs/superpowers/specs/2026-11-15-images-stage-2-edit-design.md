# Pictures, stage 2: edit by description

Date: 2026-11-15. Status: design, waiting for review. Builds on stage 1
(`2026-11-14-images-design.md`, built and on master); its "Rules every stage follows" hold here unchanged.

## Goal

Change a picture by saying what should be different: "make it dusk", "remove the background", "put this logo
on that mug". An image model service gets the instruction and the pictures and sends back one new picture.

Three ways in: the assistant's new `edit_image` tool; an edit mode of the Generate Image dialog; and a
one-line field on an image fill in the properties panel.

## What goes to the service

An instruction, and one to five pictures in two roles:

- **The picture to change**: at most one. It is either
  - a layer's image fill, read from its file, or
  - any layers **as drawn**: Pencraft renders them together to a picture (the same renderer `screenshot`
    and export use), or
  - a picture file named by its full path (tool only).
- **Reference pictures**: up to four. Each is a layer (its image fill's file when it has one, otherwise the
  layer as drawn) or a picture file from disk.

With references but no picture to change, the request is a generation guided by the references, and its
result is placed as stage 1 places a generated picture.

Before sending, a picture larger than 2048 pixels on its long side is scaled down to that. It is sent as JPEG,
or as PNG when it has transparency. The result is always an opaque picture. When there is a picture to change and references, the instruction is
sent with one sentence before it: "The first picture is the one to change; the others are references."

## Where the result goes

| The picture to change was | Result |
|---|---|
| a layer's image fill | A new file. The fill's image paint gets the new `url`; its `mode` and the fill's other paints stay. One undo step; the old file stays on disk. In the dialog, a switch "Replace the picture" (on) can be turned off to get a new layer instead. |
| layers as drawn | A new rectangle with the picture as its fill, the size of what was rendered, placed to the right of the top-level layer it is in, with a gap of 40 units, clear of other layers. Selected when the person asked for it in the dialog; the assistant's change leaves the selection alone. The layers themselves are untouched. One undo step. |
| a file (tool only) | Nothing in the document changes; the answer has the new `url`. |

The shape of the result is the shape of the picture to change unless another is asked for (the request then
leaves the aspect out, which the service reads as "same as the input"; to be confirmed live).

`images/credits.json` gets `"made": "edited"`, the service and model, the instruction, and `"from"`: the
file names of the pictures that were sent (`"drawn"` for layers that were rendered).

## Services

Stage 2 edits with **Google Gemini only**. The same Interactions endpoint as generation; each picture is one
more item in `input`: `{"type":"image","mime_type":"image/jpeg","data":"<base64>"}`, after the text.

OpenAI's edit endpoint is documented in two contradicting forms, and there is no key to tell which is true.
`Kind` gets `edits()`: true for Gemini, false for OpenAI Images. Asked to edit with a service that can't,
Pencraft says "Editing isn't possible with this kind of service yet; use a Google Gemini service." `get_state`
lists `"edits": true|false` for each service. OpenAI editing is added when a key allows a live check.

**To find out with live calls, as the plan's first task** (a few cents on a billed key; the answers become
test fixtures, as in stage 1):

1. One picture to change on the default model `gemini-3.1-flash-lite-image`: does it work, and does leaving
   the aspect out keep the input's shape?
2. A picture and two references on the lite model (Google calls it "not optimized for multiple reference
   inputs") and on `gemini-3.1-flash-image`. If the lite model fails or ignores them, the failure is given
   the stage 1 treatment: a sentence that names the model to set in Settings → Images.
3. What a picture too large, or too many pictures, is answered with.

## The assistant's tool

```
edit_image
  instruction   required   What to change, in plain words.
  layer                    The layer whose image fill is the picture to change.
  layers                   Layers to render together, as drawn, as the picture to change.
  file                     A picture file's full path as the picture to change.
                           At most one of layer / layers / file.
  references               Up to four: layer ids or full file paths.
  document, service        As in generate_image.
  aspect, size             As in generate_image; without `aspect`, the shape of the picture to change.
```

Without `layer`, `layers`, `file` and `references` it is an error that points to `generate_image`. The answer
is as `generate_image`'s (the picture made smaller, `url`, pixel size, service, model), plus what was done in
the document (`layer` whose fill was replaced, or the `id` of the new layer).

Everything stage 1 built around a parked call applies unchanged: the 100-second limit, at most three
pictures at once (generating and editing together), a held fill while the user drags, Stop dropping the
call, the switches. The switch is renamed "Let the assistant generate and edit pictures" and covers both
tools; with it off, `edit_image` is refused like `generate_image`.

The guide's "Pictures" section and the tool's description gain: edit when the user asks for a change to a
picture that is there, not to "improve" pictures unasked; `layers` sends a picture of the user's design to
the service, so use it only when the user asks for that; references only when the user names them.

The chat shows "Editing a picture" / "Edited a picture" / "Couldn't edit the picture", with the start of the
instruction.

## In the editor

**The dialog.** Opened with layers selected, the dialog shows them as a row of small pictures. One can be
marked "Change this" (an image-fill layer selected alone is marked at once); the others are references. Each
has a cross to take it out; "Add file…" brings in reference pictures from disk (craft-dialogs). At most one to
change and four references; a fifth reference is refused with a sentence. With a picture to change the title
is "Edit image", the description field's hint asks for the change, and Shape offers "Same as the picture"
(the default). With none, it is the stage 1 dialog. Under the row one line says what will leave the computer:
"Sends 3 pictures to Gemini." A service that can't edit is not offered while there are pictures.
A layer selected alone goes into the row only when it has a picture as its fill or has children (a design);
any other lone layer (an empty shape or frame, a text) leaves the row empty, so the dialog is the stage 1 one
with its target and "Use as the fill of the selection", which is never offered for a lone layer in the row.
With several layers selected they all go into the row and are sent unless taken out, which the "Sends N
pictures" line states before the button is pressed.

**"Edit with AI…"** on an image paint's row in the properties panel opens the dialog with that layer marked.

**The quick field.** Under an image paint's row: a one-line field, "Describe a change…", and a button; Return
runs it. It uses the default service (the first that can edit), no references, the picture's own shape, and
replaces the fill. While it works the row shows progress and a Cancel; a failure is shown under the field,
with the words kept. Its tooltip says the picture is sent to the service by name. With no service that can
edit, the field is replaced by one muted line that says where to add one.

A person makes one picture at a time: the dialog and the quick field share stage 1's single job, and each is
disabled while the other's picture is being made. Editor shortcuts stay blocked while the dialog is open.

All colours and sizes come from `theme.rs`; the gap of 40 and the limits (2048 pixels, four references) go
into `MECHANICS.md`.

## Code

| Where | What |
|---|---|
| `imagery/service.rs` | `Source { bytes, kind, change: bool }`; `Request` gains `sources: Vec<Source>` and `aspect` becomes optional |
| `imagery/sources.rs` (new) | Getting a picture ready to send: read a file, scale down, choose JPEG or PNG; runs on the worker thread. Plain functions, tested without a network |
| `imagery/gemini.rs` | The body with pictures; the sentence about roles; failures explained |
| `imagery.rs` | `Kind::edits()`; `listed` says it; reading an `edit_image` call into a request; the refusal for a kind that can't |
| `imagery/jobs.rs` | `Order` carries what to prepare (file paths, and rendered pictures already as bytes); the credit for an edit |
| `imagery/store.rs` | The `"edited"` credit with `"from"` |
| `app/pictures.rs` | `edit_image` parked calls; rendering layers on the main thread before the job starts; replacing an image paint; placing beside |
| `imagery/dialog.rs` | The row of pictures, roles, "Add file…", the edit title and the line about what is sent |
| `panels.rs` | The "Edit with AI…" button and the quick field on an image paint |

Rendering layers needs the window's renderer and so happens on the main thread when the call or the dialog
starts the job; reading and scaling files happens on the worker.

## Privacy and cost

This is the first stage that sends the user's own pictures, and with "as drawn" a picture of part of their
design, to a service. It happens only on a person's own action (the dialog, the quick field) or through the
tool while the assistant switch is on. The dialog says how many pictures go to which service before the
button is pressed. `AI.md` says all of this under "Safety". Each edit costs money on the user's key, like a
generated picture.

## Failures

Stage 1's table holds. New rows:

| What happens | What is said |
|---|---|
| The service's kind can't edit | That, and to use a Google Gemini service |
| The layer has no image fill, or its file is missing or not a picture | Which layer, and what is wrong with it |
| The layers are empty or too large to draw | That there is nothing to send |
| More than one picture to change, or more than four references | The limit |
| The model can't take the references (if the live check shows such an answer) | The model to set in Settings → Images |

Nothing is written and nothing in the document changes when an edit fails.

## Tests

- `sources.rs`: scaling down, the choice of JPEG or PNG, a file that isn't a picture.
- Gemini: the body with one picture and with references, in order, with the sentence about roles; answers
  from the live fixtures.
- Reading an `edit_image` call: each source form, the limits, a kind that can't edit, no source at all.
- Replacing an image paint keeps `mode` and the other paints and is one undo step; placing beside puts the
  new layer 40 units to the right at the rendered size.
- The dialog's roles: marking, the limits, opening on a lone image-fill layer.
- The `"edited"` credit.
- One `#[ignore]` live test that edits a picture with the real Gemini service.
- `scripts/check.sh` passes.

## Not in this stage

- Editing with OpenAI Images.
- Masks or regions ("change only this part"), and several results per request.
- Editing in turns that remember the last result (`previous_interaction_id`).
- Searching online (stage 3) and adjustments by hand (stage 4).
