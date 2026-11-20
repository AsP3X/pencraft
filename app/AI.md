# Letting an AI model work in Pencraft

Pencraft speaks the Model Context Protocol (MCP), so any AI client that can use MCP tools can read and change
the documents open in Pencraft: Claude Code, Claude Desktop, Cursor, Codex, Gemini CLI, and others.

## The chat inside Pencraft

Type in the message box and press Return. The assistant answers in the chat panel and works on the document
its chat belongs to (the one in front when you send); what you have selected is what "this" means. The menu beside the send button picks who answers:

- **Claude Code** (when the `claude` program is installed): uses your Claude sign-in, no key needed. If it
  fails to answer, run `claude` once in a terminal to check that it is signed in.
- **Grok** (when xAI's `grok` program is installed): uses your Grok sign-in (`grok login`). Grok takes its tool
  servers from its own settings, so add Pencraft there once: Settings → Agents → Grok → "Add Pencraft to
  Grok" (it runs `grok mcp add pencraft -- <pencraft> mcp`). Only that server's tools are approved for the
  chat. Grok reaches Pencraft the way an outside client does, so it works on the design in front, and pictures
  attached to a message are not passed on to it.
- **Any other command-line agent** (Codex, Gemini CLI, OpenCode and the like): Settings → Agents → "Add
  command-line agent". Give its program and the arguments that make it answer one prompt and exit, with
  `{prompt}` where the message goes (for example `exec {prompt}`); what it prints is shown as its answer. Add
  `pencraft mcp` to that program's own MCP servers (Settings → MCP has the lines) and allow it to use them
  without asking. Such a program keeps no session here, so each message retells the conversation so far.
- **Anthropic API**: start Pencraft with `ANTHROPIC_API_KEY` set, or click the Anthropic card in Settings →
  Agents and enter your key (kept in the keychain).
- **Another known provider** (OpenAI, xAI, Google Gemini, OpenRouter, DeepSeek, Moonshot, Kimi For Coding,
  Together, Fireworks, Z.AI, Meta, OpenCode Zen and Go): click its card in Settings → Agents. Its address and
  API type are filled in; enter your key, press "Choose models…", tick the models you want in the window that opens, and
  save. The card then says "Ready in the chat" and its models are in the assistant menu.
- **Ollama** (models that run on your own computer): click the Ollama card in Settings → Agents. Its address
  (`http://localhost:11434`) and the API type "Ollama" are filled in and no key is needed; with Ollama
  running, press "Choose models…", tick the models you have pulled, and save. Change the address if
  Ollama runs on another machine. Pick a model that can call tools. Pencraft asks Ollama to read up to
  32 768 tokens of the conversation (its brief and tools alone are about 17 000), which takes more memory
  than Ollama's own default; a model that can read less than that will lose the start of the conversation.
  Ollama's cloud models work here too once you are signed in (`ollama signin`) and have pulled one.
- **Ollama Cloud** (Ollama's hosted models, without Ollama on your computer): click the Ollama Cloud card,
  enter an API key from ollama.com (Settings → Keys there), load the models and save. Both cards can be
  set up side by side.
  "Choose models…" opens a window with the provider's own list. Type a few letters of a model's name to find
  it, in order but not necessarily side by side ("ds4p" finds deepseek-v4-pro); the arrow keys move through
  the list and Return ticks a model. A model the provider doesn't list is added with "Add by ID".
  Where the provider says how much a model reads and writes, its context window and max tokens are filled in
  for the models you choose (Ollama is asked about each one and says how much it reads); numbers you typed
  yourself are left alone.
- **Any other provider**: "Add custom provider", with the API type "OpenAI Completions" (what most providers
  and other local model servers speak; a key is optional there), "Anthropic Messages", or "Ollama" for an
  Ollama on another machine.

Settings → Agents lists the agent programs under "Agents on this computer": whether each is installed, where
it is (a path of your own if it isn't found by itself), which models it is offered with, and a switch to take
it out of the chat's menu. Cursor and GitHub Copilot have no API another app can use with a key, so their
cards can't be connected; the "OpenAI Responses" API type and the two sign-in buttons don't work yet either.
A model that reads no pictures still works: the screenshots the assistant takes to check its work are then
left out. Finished designs are reviewed only with Claude Code and providers that speak the Messages API.

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

A logo ("a logo for my bakery", "a family crest", "a mascot for our team", "an app icon") is made another way
than a page. The assistant chooses a rendering style from what you ask for (say "vintage badge", "a full coat
of arms", "minimal", "3D", "detailed" to choose it yourself), looks for one idea, draws three rough marks in
black, keeps the best and builds it exactly; a detailed logo is built in passes, from the silhouette to the
fine lines, and comes with a small version redrawn for favicon sizes. Each picture it takes of a logo comes
with a second one, the logo drawn 24 pixels small and enlarged, and with what the app counted in it (lights
that disagree, more than three line weights, a detailed logo without its small version). A logo is reviewed
by the second model like a larger design, as a logo of its style.

A shape can cut what follows it: a path, rectangle, ellipse or polygon with `"mask": true` is drawn as it is
and cuts every layer after it in the same group or frame to its outline (water inside a shield, shading
inside a letter); a second mask in the group ends the first. A `stroke` on a text draws an outline round its
letters, outside them, and several strokes draw outlines one round the other. Both are drawn on the canvas
and in every export. In the properties panel a shape has "Mask Layers Above" under Appearance, a text takes
its outline in the Stroke section, and the Text section has Straight, Circle Top and Circle Bottom with the
angle the text is turned by. `edit_layers` and
`read_layers` now also say where a text's letters lie (`inkBoxes`, `inkBox`), an `update` that sets nothing
fails instead of answering as done, and the advice names a font weight a family doesn't have.

Text can also follow any line (a waving scroll, an arch, a slope): a text layer with `"onPath"` holding SVG
path data in the layer's own coordinates runs along it, placed by its alignment. The assistant can set
the line, and so can you: in the properties panel choose "Line" in the Text section (the text gets a gentle
arch to start from), then "Edit Line" or Enter shows the line's points on the canvas. They are moved, bent
by their handles, added and removed like a path's, and the text follows; Enter or Escape ends it. A double
click on the text still edits its words. In the layers
list a mask layer is marked "mask".

Text can run round a circle, for the rim of a seal or a badge: a text layer with `"onCircle": "top"` or
`"bottom"` follows the circle that fits its box (`circleAngle` turns it). It is drawn so on the canvas and in
PNG, PDF and SVG exports. Its words are edited on the canvas in one line in the middle of the circle, and the
text round the rim follows as you type.

When a request could go two ways, the assistant asks instead of guessing: a question with a few answers to
pick from. In the chat it is a card; with the sidebar folded away, the status line over the message bar grows
into the question; with another view open (the layers, say), it sits under that view. Tick the answers that apply (as many as you like), write your own in the field under them if none fits or to add to them, and press Next or Send; Return in the field sends too. Several questions come
one after the other, and your answers go to the assistant together. The cross closes a question unanswered.
When it has finished something, the assistant may offer next steps the same way: tick one or several, or write something else, and send them as your
message. A finished design is not reviewed while a question waits for you.

The send button turns into a stop button while the assistant works. "New" starts a new chat.

You can keep writing while the assistant works. A message sent meanwhile shows greyed at the end of the chat
and is sent when the assistant is done. With Settings → General → Agents → "Always steer messages" on, an API
assistant gets it at its next step instead (after the tool it is using), so you can correct it on the way; a
command-line agent reads its message only when it starts, so there the message still waits. Stopping sends
none of the waiting messages. "Chat text size" sets the size of the conversation's text, and "Show agent input
on dashboard" decides whether the home screen has its box to describe a design in.

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
- When you look at it, you see the model work: what a call changes is shown a step at a time (a new layer
  appears, a change is made, a deleted layer goes), every layer it touches flashes at its turn, and a cursor moves to where it is working. The cursor is labelled "Agent"; Settings → General → Agents → "Agent cursor label" changes that
  to the model's name (the chat's model, or the name an outside client gave) or to nothing, and "Agent cursor
  size" makes it larger. A change's description shows at the bottom of the window.
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
| `save_document` | Saves the document, or saves it under a new path; a path ending in `.pencraft` saves a bundle with its pictures and shader files |
| `export_layers` | Writes layers to a PNG, JPEG, WebP, PDF or SVG file |
| `generate_image` | Has an image model make a picture from a description and gives back the file to use in an image fill, or puts it on a layer. Needs an image service (see Pictures) |
| `search_web` | Web results for a query: titles, addresses and the lines under them |
| `read_web_page` | A web page as plain text, to take real content from it |
| `search_images` | Photos and illustrations that may be reused, each with its maker, licence and credit line |
| `download_image` | Fetches a picture into the `images` folder beside the document, for an image fill |
| `edit_image` | Has an image model change a picture by an instruction (a layer's picture, layers as they look, or a picture file), or make one that goes by up to four reference pictures; puts the result in place of the layer's picture or beside the layers drawn. Needs a Google Gemini service (see Pictures) |
| `ask_user` | Asks you one to three questions, each with answers to tick, or offers next steps. Only the chat inside Pencraft has it |

## The web

Ask the assistant for real content or a real photo ("use a real picture of the Elbphilharmonie here") and it
searches, fetches the picture into the `images` folder beside your `.pen` file, and fills the layer with it.
The picture is filed in `images/credits.json` like a generated one, with the address it came from and its
credit line. A document that has no file yet keeps the picture in Pencraft's own folder until it is saved.

- Nothing needs a key. Web results come from DuckDuckGo's plain page, pictures from Openverse and Wikimedia
  Commons. Your queries go to those services, and a fetched page or picture to its own site.
- More pictures with a free key of your own: Settings → MCP → "Picture sources" takes a Pexels and a Pixabay
  API key (kept in the keychain). Their stock photos then appear in every picture search beside the others.
- Pictures found this way may be reused under the licence each result names, and each result says how to
  credit it. Check the licence before you publish: some forbid commercial use or changes.
- A site may limit how often it is asked ("429"). Pencraft waits a few seconds and asks again, twice; after
  that the assistant is told to take a picture from another site. Every search mixes pictures from all
  sources for that reason. Wikimedia's pictures are fetched as its standard 1920-wide copies, not as the
  originals, as Wikimedia asks of apps. Pencraft never pretends to be a browser to get more.
- A layer never loads a web address itself; only pictures fetched through `download_image` are shown.
- Addresses on this computer or its local network are never fetched, also not when a site sends the request
  on to one. What a page says is given to the assistant as information; it is told not to take instructions
  from it.
- Settings → MCP → "Let AI models use the web" turns the four tools off together; each also has its own switch.

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

| [`logos`](skills/logos/SKILL.md) | Logos of every kind and rendering style: flat marks, monoline, vintage badges and roundels with text round the rim, heraldic crests, seals, engravings, mascots, luxury monograms, dimensional marks and app icons. Choosing the style from the brief, finding one idea, three rough marks, exact construction (arcs, cut-outs, rotation), building detail in passes (line weights, tones, one light), the wordmark and its lockup, the small version, and the proofs |

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

## Settings for assistants

- **Agent names**: your own name for each assistant (shown in the chat's menu and, with the cursor label set to
  the model's name, beside its cursor), and what a cursor labelled "Agent" says instead.
- **Chat**: the default assistant, and custom instructions every conversation is given with the app's own
  brief (your brand, your taste, the language to answer in). They apply from the next message.
- **MCP**: whether the local server runs and where, the lines above with this copy's path ready to copy, the
  web switch and picture sources, and a switch for every tool. A tool that is off isn't offered to any client
  or to the chat, and a call to it is refused. A client that is already connected keeps the list it was given
  until it asks again (most do when they reconnect); the chat's next message has the new list.

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
