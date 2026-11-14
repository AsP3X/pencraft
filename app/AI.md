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
pictures in `~/.pencraft/images` until then. If `credits.json` can't be read (it is not valid JSON), the picture
is not saved and the message says so; fix or remove the file and ask again. Pencraft never replaces it.

A picture can take a minute or two. After 100 seconds Pencraft gives up and keeps nothing; the service may still
have charged for it. A picture is also not kept when you cancel the dialog or stop the chat that asked for it.
At most 3 pictures are made at once for assistants; a fourth is turned away until one is done.

When the assistant asks for a picture to go on a layer and you are dragging something on the canvas, the
picture waits up to 10 seconds for you to let go. If you are still dragging, the layer is left as it is and the
assistant is told to place the picture itself.

`get_state` lists the services an assistant can use under `imageServices`. It is empty, with a note, when
"Let the assistant generate pictures" is off.

## Skills

A skill teaches a model one kind of design in Pencraft: what to decide, how to build it with Pencraft's
layers, and how to check the result. Skills are folders in the open Agent Skills format, in
[`skills/`](skills/), and are built into the program.

| Skill | For |
|---|---|
| [`liquid-glass`](skills/liquid-glass/SKILL.md) | Liquid Glass interfaces (the translucent, light-bending material of iOS 26 and macOS 26) and frosted panels: which surfaces get glass, the `material` property, glass groups, legibility, tokens, ready-made components |
| [`ui-components`](skills/ui-components/SKILL.md) | Standard components the way the common libraries build them (shadcn/ui, Radix, Base UI, Sonner, Vaul, cmdk): buttons, fields, menus, dialogs, drawers, toasts, a command palette, cards and tables, with sizes, tokens for light and dark, states and placement, and the numbers of named looks |

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

Making a picture sends its description, and nothing else of the document, to the image service you chose, and
is paid for on your key. Settings → Images → "Let the assistant generate pictures" stops AI models from doing
it; the Generate Image dialog still works.
