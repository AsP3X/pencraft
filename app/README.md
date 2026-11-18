# Pencraft

A canvas design editor for `.pen` files, written in Rust as a Craft app (egui on the storytold fork, with the
storytold craft-libs for saving, settings, dialogs and quitting). Runs on macOS and Windows; Linux next.

On a Mac the menus are in the menu bar and shortcuts use ⌘; on Windows the menus are behind ☰ in the tab strip and
the same shortcuts use Ctrl (⌥ is Alt, full screen is F11). The documentation writes shortcuts the Mac way.

- `cargo run` — open the app. `cargo run -- design.pen` opens a file.
- `cargo run -- --screenshot out.png` — draw ~120 frames, save a picture of the window, quit.
  Add `--dark` for the dark look and `--sidebar history|chat|layers|hidden|moving|morphing|question|suggestions|asked|pinned` to choose what the sidebar shows,
  `--settings general|canvas|agents|provider|cli|chat|images|image-service|permissions|mcp` to show the settings window on that page (`provider`, `cli` and `image-service` are the forms that add a provider, a command-line agent and an image service), `--generate-image` to show the Generate Image dialog open for the selection, `--select ID` to select a layer, `--points` to show that layer's path points being edited, `--actions` to show the action search, `--find TEXT` to show find layers with that text searched for, `--attach FILE` to show a file attached to the chat message, `--update-prompt` to show the card that offers a newer version, `--update-required` to show the card of an update that is required, `--update-collapsed` to show the card folded away into its pill, `--update-download [PERCENT]` to show that card in the middle of a made-up download (without `--screenshot` the download goes on by itself, to watch its bar move; nothing is fetched), `--home` to show the home screen with the named files as its recent files, and `--export FILE` to also export the selected layer (the ending picks the format).
- `cargo run -- --no-update-check` — open the app without asking GitHub for a newer version (`--private` and `--background` runs never ask).
- `scripts/check.sh` (or `check.ps1`) — format, clippy with warnings as errors, tests, dead-code grep.

## AI models

The chat panel talks to Claude through the installed Claude Code program or a provider's API with your key.
Any AI client that speaks MCP can read and change open documents through `pencraft mcp`, also while the app is in
the background; touched layers flash and the model's cursor shows where it works. Setup: [AI.md](AI.md).

Skills teach a model one kind of design each; they are in [`skills/`](skills/) and reach every client through
`read_skill` or `pencraft skills install`. So far: [`liquid-glass`](skills/liquid-glass/SKILL.md) (Liquid Glass and
frosted materials, with the `material` layer property and the `check_glass` tool),
[`ui-components`](skills/ui-components/SKILL.md), [`devices`](skills/devices/SKILL.md) and
[`logos`](skills/logos/SKILL.md) (a mark from one idea, constructed from a few exact paths, with its wordmark).

## Layout

| Path | Role |
|---|---|
| `crates/pen-doc` | The document without a window: model and JSON round-trip, variables and themes, component instances, flex layout, SVG paths, editing by id. `examples/pen_check.rs` checks files. |
| `src/` | The app: `app` (tabs, home, files, shortcuts), `editor` (one open document, undo), `canvas` (pointer tools), `render` (drawing), `sidebar` (the left sidebar's islands and the tool rail), `conversation` (the chat's lines, tool notes and their motion), `pen` and `path_edit` (drawing paths and editing their points), `settings` (the settings window), `panels` (layers list, properties), `ops` (editing commands), `palette` (the action search and the list of commands), `find` (find layers), `menus` (the macOS menu bar), `keys` (shortcuts written the way each system does: ⌘S or Ctrl+S), `export` and `render/svg_out` (exporting layers to PNG, JPEG, WebP, PDF and SVG), `effects` (a layer's effects as the file stores them), `raster` and `render/bake` (shadows and blurs, painted on the CPU), `shader` and `gpu` (shader effects: reading WebGL shader files and running them on the GPU), `chat` (the chat's engines; `chat/critic` has a second model review a finished design), `craft` (what a model is told about designing well) and `review` (what the app counts in a design when the model looks at it), `update` (asking GitHub for a newer release and the card that offers it; `update/install` downloads, checks and installs it), `theme`, `prefs`, `fonts` and `faces` (the installed font families a design's text is drawn in), `screenshot`, `samples`. |
| `MECHANICS.md`, `mechanics.json` | Every rule and number Pencraft uses. |

Installing, building installers and releasing: the [repository's README](../README.md), [docs/BUILDING.md](../docs/BUILDING.md)
and [docs/RELEASING.md](../docs/RELEASING.md).

Coding rules: the user's Rust rules (pedantic clippy, no `allow`, no panics in shipped code, exact pins,
Human/Agent comments) — see `AGENTS.md`.
