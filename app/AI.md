# Letting an AI model work in Pencraft

Pencraft speaks the Model Context Protocol (MCP), so any AI client that can use MCP tools can read and change
the documents open in Pencraft: Claude Code, Claude Desktop, Cursor, Codex, Gemini CLI, and others.

## The chat inside Pencraft

Type in the message box and press Return. The assistant answers in the chat panel and works on the document
its chat belongs to (the one in front when you send); what you have selected is what "this" means. The menu beside the send button picks who answers:

- **Claude Code** (when the `claude` program is installed): uses your Claude sign-in, no key needed. If it
  fails to answer, run `claude` once in a terminal to check that it is signed in.
- **Anthropic API**: start Pencraft with `ANTHROPIC_API_KEY` set, or add a provider in Settings → Agents with
  the API "Anthropic Messages", the address `https://api.anthropic.com` and your key (kept in the keychain).
- **Another provider that speaks the Anthropic Messages API**: add it the same way with its own address.

To point at layers in the middle of a sentence, copy them on the canvas (⌘C) and paste into the message
box (⌘V): they become a chip, and the assistant works on exactly those layers.

When the assistant has finished a design, a second model that didn't make it looks at a picture of it and
judges it like a strict art director; the assistant then works in what the reviewer wants changed (up to two
rounds). That makes a design take longer and cost more; Settings → Agents → "Have designs reviewed" turns it off.

A few words mean something definite to the assistant when you ask it to change a design that is there:
"bolder" (one decisive move with what the design already owns, not more effects), "quieter", "simpler"
(take away), "polish" (alignment, spacing and consistency, no new ideas), and "what do you think?" (a review
that changes nothing). In each case it leaves alone what you did not name.

A mock of a phone or tablet app ("a mock of an iPhone gallery app") comes inside the device: the phone's body
and corners, with the status bar, the Dynamic Island or camera and the home indicator. Say "the screen alone"
or "no device" to get the bare screen.

The send button turns into a stop button while the assistant works. "New" starts a new chat.

Every open design has a chat of its own: switching tabs switches the chat, and a message half written in one
waits there. The chats work side by side. An assistant at work in one design keeps to that design while you
look at another and talk to its chat, and it doesn't bring its design to the front; its tab shows a dot while
it works. A chat that opens or starts another design goes on in that design. Closing a design ends its chat.

The rest of this page is about AI clients outside Pencraft.

## Connect a client

The client starts `pencraft mcp` and talks to it. Use the full path of the Pencraft program:

```
/path/to/Pencraft/app/target/release/pencraft
```

Claude Code:

```bash
claude mcp add pencraft -- /path/to/Pencraft/app/target/release/pencraft mcp
```

Codex (`~/.codex/config.toml`):

```toml
[mcp_servers.pencraft]
command = "/path/to/Pencraft/app/target/release/pencraft"
args = ["mcp"]
```

Cursor, Claude Desktop and most others (their MCP settings file):

```json
{ "mcpServers": { "pencraft": { "command": "/path/to/Pencraft/app/target/release/pencraft", "args": ["mcp"] } } }
```

A client that speaks MCP over HTTP can skip `pencraft mcp`: the address and the secret to send as
`Authorization: Bearer <token>` are in `~/.pencraft/agent.json` while Pencraft runs (both change with every start).

## What happens

- If Pencraft isn't running, `pencraft mcp` starts it behind your other windows, without taking the keyboard.
- Pencraft doesn't need to be in front. It answers while it is behind other windows, minimised or hidden.
- When you look at it, you see the model work: every layer it touches flashes, and a cursor with the model's
  name moves to where it is working. A change's description shows at the bottom of the window.
- Each `edit_layers` call is one undo step: ⌘Z takes back what the model just did.
- Nothing is saved unless the model calls `save_document` or you save.

## The tools

| Tool | What it does |
|---|---|
| `get_state` | The open documents; for the one in front its top-level layers, selection, components, variables |
| `get_guide` | How a document is built (layer types, layout, fills, effects, components), how to design well in it, and the installed fonts |
| `list_skills`, `read_skill` | The skills Pencraft has for one kind of design each (see below), and a skill's text or one of its reference files |
| `read_layers` | Layers with their properties and their boxes on the canvas; or a search by name, text or type |
| `edit_layers` | Insert, update, delete, move, copy and replace layers, set variables; all or none |
| `screenshot` | A picture of layers as Pencraft draws them, with a review of what makes them read as generated or broken, so the model can check and improve its work |
| `check_glass` | Measures a design that uses glass materials: which appearance each material came to, the contrast of every label on glass against the worst of what is under it, and how the glass is built |
| `select_layers` | Selects layers for you and brings them into view |
| `open_document` | Opens a `.pen` file, or starts an empty document |
| `save_document` | Saves the document, or saves it under a new path |
| `export_layers` | Writes layers to a PNG, JPEG, WebP, PDF or SVG file |
| `generate_image` | Has an image model make a picture from a description and gives back the file to use in an image fill, or puts it on a layer. Needs an image service (see Pictures) |
| `edit_image` | Has an image model change a picture by an instruction (a layer's picture, layers as they look, or a picture file), or make one that goes by up to four reference pictures; puts the result in place of the layer's picture or beside the layers drawn. Needs a Google Gemini service (see Pictures) |

## Pictures

Pencraft can have an image model make a picture: File → Generate Image… (also in the action search), or ask
the assistant ("put a photo of a red fox on this card"). It needs an image service: add one with its API key in
Settings → Images (Google Gemini, OpenAI Images, or another service that speaks the OpenAI Images API), or start
Pencraft with `GEMINI_API_KEY` or `OPENAI_API_KEY` set. The model and the address can be changed there. A
service saved without a key is not used; asking for it by name says it has no key yet.

A Gemini key must belong to a project with billing set up in Google AI Studio: the free tier makes no
pictures, and a Google AI Pro subscription does not count for the API. Gemini's usual model here
(`gemini-3.1-flash-lite-image`) makes the normal size only; for large pictures set the model to
`gemini-3.1-flash-image` in Settings → Images.

A picture is saved as a file in an `images` folder beside the document, with a line in `images/credits.json`
saying which service and model made it from which description. A document that has no file yet keeps its
pictures in `~/.pencraft/images` until then; when it is saved, they are copied into `images` beside it with
their credits, and the document names them from there. Saving a document under a new name in another folder
copies the pictures it names from its own folder along. If `credits.json` can't be read (it is not valid JSON), the picture
is not saved and the message says so; fix or remove the file and ask again. Pencraft never replaces it.

A picture can take a minute or two. After 100 seconds Pencraft gives up and keeps nothing; the service may still
have charged for it. A picture is also not kept when you cancel the dialog or stop the chat that asked for it.
At most 3 pictures are made at once for assistants; a fourth is turned away until one is done.

When the assistant asks for a picture to go on a layer and you are dragging something on the canvas, the
picture waits up to 10 seconds for you to let go. If you are still dragging, the layer is left as it is and the
assistant is told to place the picture itself. A picture you asked for yourself waits as long as you are
dragging or typing text on the canvas, and goes in when you stop; once it has come back, Cancel no longer
throws it away.

`get_state` lists the services an assistant can use under `imageServices`, with `edits: true` for those that
can change pictures. It is empty, with a note, when "Let the assistant generate and edit pictures" is off.

### Changing a picture

Pencraft can also have a picture changed by description ("make it dusk", "take the people out"), or have a new
one made that goes by pictures you give it. There are three ways:

- **Ask the assistant.** It uses the `edit_image` tool, and sends only the pictures you name.
- **The Generate Image dialog.** Select layers and open File → Generate Image…. The selected layers show as a
  row of small pictures. Click "Reference" under one to mark it "Change this": it is then the picture to
  change, and the dialog is called "Edit image". The others are references, pictures for the service to go
  by. "Add file…" adds picture files as references, and the cross takes a picture out of the row. A layer
  selected alone goes into the row only when it has a picture as its fill (it is then marked at once) or has
  layers inside it. Any other lone layer, such as an empty rectangle, gets the plain dialog, which offers to
  use the new picture as that layer's fill. Under the row the dialog says how many pictures it sends, and to
  which service, before you press the button.
- **On an image fill.** When the selected layer has a picture as its fill, the properties panel shows "Edit
  with AI…" (the wand) on the picture's row, which opens the dialog with that picture marked, and under the
  row a field, "Describe a change…". Type the change and press Return or the arrow: the picture and your
  words go to the default service when it can change pictures, else to the first that can, and the result
  takes the picture's place. You can go on working meanwhile. If another layer or tab is in front when it is
  done, the message line says what became of the picture. Words you typed and left stay with that layer
  until its picture is changed (for the last layer you left with words in the field).

The picture to change is the picture in a layer's fill (the topmost one when the fill has several), layers as
they look on the canvas ("as drawn", for a design or anything without a picture), or for the assistant a
picture file. A reference is any layer (its picture when its fill has one, otherwise how it looks) or a
picture file. One picture to change and four references at most go in one request. With references alone,
the service makes a new picture from your description, guided by them.

Where the result goes:

- **In place of a layer's picture.** Only the picture is replaced: the fill's other paints and the picture's
  mode stay, and it is one undo step. This happens only while the layer still shows the picture that was sent.
  If the picture was changed or removed meanwhile, the result becomes a new layer instead, and you are told.
  In the dialog, "Replace the picture" turned off puts the result beside the layer instead.
- **Beside.** Layers changed as drawn are left alone. The result is a new layer, "Edited picture", as large as
  what was drawn and at its height, to the right of the top-level layer it is in and clear of the other
  top-level layers. It is selected when you asked for it yourself.
- **Elsewhere.** From the dialog, a picture made from references alone becomes a new layer in the middle of the
  view. The assistant gets that picture, or a changed picture file, as a file to use; nothing in the document
  changes.

The dialog's result brings its document's tab to the front; the field's does not. The result is never
transparent: a picture with a transparent background comes back with a filled one.

Only Google Gemini services can change pictures for now. With only an OpenAI Images service, the dialog says
so when pictures are in its row, and the field on an image fill says where to add a Gemini one.

The pictures sent are scaled down to at most 2048 pixels on their longer side. A picture file is read only
when it is a PNG, JPEG, WebP or GIF file of at most 100 MB (its first bytes say which, not its name). All the
pictures of one request together may come to 15 MB. Changing a picture takes as long as making one, with the
same 100 second limit.

## Skills

A skill teaches a model one kind of design in Pencraft: what to decide, how to build it with Pencraft's
layers, and how to check the result. Skills are folders in the open Agent Skills format, in
[`skills/`](skills/), and are built into the program.

| Skill | For |
|---|---|
| [`liquid-glass`](skills/liquid-glass/SKILL.md) | Liquid Glass interfaces (the translucent, light-bending material of iOS 26 and macOS 26) and frosted panels: which surfaces get glass, the `material` property, glass groups, legibility, tokens, ready-made components |
| [`ui-components`](skills/ui-components/SKILL.md) | Standard components the way the common libraries build them (shadcn/ui, Radix, Base UI, Sonner, Vaul, cmdk): buttons, fields, menus, dialogs, drawers, toasts, a command palette, cards and tables, with sizes, tokens for light and dark, states and placement, and the numbers of named looks |
| [`devices`](skills/devices/SKILL.md) | Screens shown inside the phone or tablet they are for: a ready iPhone template measured from the real one (status bar with the time and the signal, Wi-Fi and battery glyphs, Dynamic Island, home indicator, side buttons), and the numbers of an Android phone and an iPad |

A model gets a skill in one of two ways, with the same text either way:

- **Through Pencraft's tools** (every MCP client, and the chat inside Pencraft): the guide lists the skills,
  and `read_skill` gives a skill or one of its files. Nothing to install.
- **As a skill of the client itself**, so the client offers it whenever a request fits, even before
  Pencraft's tools are called:

  ```bash
  pencraft skills install
  ```

  writes the skills to `~/.agents/skills/` (read by Codex, Cursor, Gemini CLI, OpenCode and GitHub Copilot)
  and `~/.claude/skills/` (read by Claude Code). `pencraft skills install --project DIR` writes them into a
  project's `.agents/skills/` and `.claude/skills/` instead; `--dir DIR` into any one folder (for example
  `~/.cursor/skills`). Run it again after updating Pencraft. `pencraft skills` lists the skills;
  `pencraft skills show liquid-glass` prints one.

The skill still needs Pencraft's tools to do anything: connect the client as described above.

## Safety

The server listens on this computer only, serves only callers that show the secret from `~/.pencraft/agent.json`
(readable by you alone), and refuses requests from web pages. Turn it off in Settings → Agents →
"Let AI models work on documents".

Making a picture from a description sends that description, and nothing else of the document, to the image
service you chose. Changing a picture sends more: the description, and every picture named. That is a
layer's picture file, a picture file added as a reference, and for layers sent as drawn a picture of that
part of your design. Pictures are sent only when you ask for it yourself (in the Generate Image dialog or the
field on an image fill), or when the assistant calls `edit_image` while Settings → Images → "Let the assistant
generate and edit pictures" is on. Both are paid for on your key. That switch stops AI models from making and
changing pictures; the dialog and the field still work.

A layer's picture is replaced only if the layer still shows the picture that was sent. Cancel in the dialog or
the field, or Stop in the chat, keeps anything that has not been sent yet from being sent; a request already
sent may still be charged.
