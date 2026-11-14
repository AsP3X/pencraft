# Liquid Glass: running notes

Re-read this file and `TASK.md` whenever work resumes.

## Status

- 2026-11-14: **all five phases done**; `scripts/check.sh` passes (fmt, clippy -D warnings, 28 + 187 tests,
  dead-code grep). Work is in the git worktree `.claude/worktrees/liquid-glass` on branch `liquid-glass`
  (from master 95289d6). Never pushed, not merged.
- Engine: `material` key (`app/src/material.rs`), bake + shader (`app/src/render/glass.rs`, `glass.frag`),
  glass groups (`glassGroup`), inspector section, validation in `edit_layers`, SVG stand-in, golden tests
  (`app/golden/glass/`; `PENCRAFT_BLESS=1` keeps new ones, `PENCRAFT_REQUIRE_GPU` makes a missing adapter
  fail), `check_glass` (`app/src/glass_check.rs`), rounded clipping (`Builder::round_off` in `bake.rs`).
- Skill: `app/skills/liquid-glass/` (SKILL.md + references), `app/src/skills.rs` (`list_skills`,
  `read_skill`, guide index, `pencraft skills install`). Not installed into the user's real home folder.
- Tests with fresh agents: `docs/liquid-glass/tests/` (prompts, photos, `pc.py`, `start.sh`, `out/N`,
  `renders/`). Round 1: tests 1-4; fixes; round 2: tests 1 and 3 again (`1b`, `3b`) and test 5. Every run
  ended with `check_glass` ok.
- Measured (release build, this Windows machine, Vulkan, before the rounded-clip change): 12 glass panes over
  a photo bake in about 122 ms once (56 ms of that is the same screen without materials); an unchanged
  canvas does 0 ms of work; a rebuild after an edit elsewhere 0.2 ms; a zoom step to 2x re-bakes in 212 ms.
  Not measured: frame times while panning in the window, integrated graphics, macOS.
- Baseline: the nine `checks/design-review/pens` exports are pixel-identical to before the work, checked
  three times (after the engine, after rounded clipping, at the end).
- Asked for by the user on the way (through the release session): content showed past a clipping frame's
  rounded corners. Fixed: a clipping frame cuts to its rounded outline; exports take no room for cut-off
  content.

## Open / follow-ups (not done)

- Not run: macOS, Metal, a software adapter; the frosted fallback for glass without a GPU is tested only
  through the frosted material's own tests.
- Code export does not exist in Pencraft, so the task's code-export mapping has no target.
- No corner smoothing, no blur that varies along its length (scroll edge is a gradient fade), no motion.
- Seen by test agents, outside this task: the existing review judges text over a sibling photo against the
  frame's fill (false "too faint"); a missing image file is drawn as a grey box without any advice; one
  agent saw `fontFamily` change nothing in its instance (not reproduced); `save_document` refuses a
  `/c/...` path; `screenshot` reduces its scale for large areas (documented, but surprising).
- The pictures in `tests/photos` are synthetic; no reference screenshots of iOS 26 were available.

## Phase 1 findings (verified in the checkout on `windows-support-and-chat-notes`, uncommitted work included)

1. **Skill system: none.** No `SKILL.md`, no `read_skill`, no per-harness files anywhere. Design guidance is one
   text: `agent::GUIDE` (`app/src/agent.rs:21`, the format) + `craft::CRAFT` (`app/src/craft.rs:20`, how to design)
   + `craft::fonts()`, joined by `craft::guide()` (`craft.rs:256`).
2. **Harness delivery.** Only two routes: the MCP tool `get_guide` (`app/src/app/tools.rs:174`) plus the
   `initialize` instructions (`app/src/link.rs:266`), and the built-in chat's brief (`app/src/chat.rs:59`). MCP has
   `initialize`, `tools/list`, `tools/call` only (no resources, no prompts). `app/AI.md` documents connecting
   Claude Code, Codex, Cursor, Claude Desktop. Nothing is written to any harness's skill or rules folder.
3. **Rendering capabilities.**
   - Yes: background blur (`background_blur`, one per layer, Gaussian only, no saturation), layer blur, linear /
     radial / angular gradients on fills and strokes (per-vertex colours, `render.rs:324`), inner shadows, stacked
     shadows, per-paint opacity, image fills, variables and theme axes (`pen-doc/src/vars.rs`), components with
     instance overrides (`ref` + `descendants`), custom fragment shaders as an effect (`{"type":"shader","url":…}`).
   - No: blend modes (key survives a save, is not drawn), masks (only rectangular `clip`, not rounded), noise or
     texture fills, mesh gradients (first colour only), corner smoothing, component variants, displacement as a
     primitive.
4. **Render pipeline.** egui (storytold fork) on wgpu 30 draws tessellated triangles (lyon) with 4x MSAA
   (`render.rs`, `main.rs:105`). Effects are **baked on the CPU**: `render/bake.rs` paints the source items into a
   `raster::Bitmap`, blurs (three box blurs), uploads as an egui texture, cached by a position-independent hash
   and re-baked per power-of-two zoom bucket; at most 1,000,000 px / 2048 a side per picture. Shader effects:
   WebGL 1 GLSL rewritten to GLSL 450 (`shader.rs`), run once on the window's wgpu device with CPU upload and
   readback (`gpu.rs`), `Rgba8Unorm` premultiplied. **Backdrop access exists**: items are in paint order, so
   "everything before the node" is its backdrop (`bake.rs:750` `backdrop`, and `pencil:background` for shaders,
   `bake.rs:517`, mirrored at the edges, canvas colour underneath). The `@sdf` sampler role is declared but not
   implemented (`shader.rs:86`). Colour is blended in gamma space throughout (egui's way).
5. **Verification.** `pencraft FILE.pen --screenshot OUT.png --sidebar hidden --select ID --export FILE.png`
   works on Windows (ran it: 2.6 s, Vulkan). `--screenshot` is the real window (GPU triangles); `--export`, the
   MCP `screenshot` and `export_layers` use `DisplayList::snapshot` (`render.rs:138`): the same item list and the
   same baked effect pictures, painted by the CPU rasteriser. No CI, no image tests, no golden files; unit tests
   have no GPU device and no window.
6. **Agent surface.** MCP tools in `agent::tools()` (`agent.rs:104`), executed in `app/tools.rs`; layers are raw
   `.pen` JSON. `effect` is a free JSON value (`model.rs:281`): **nothing validates effects**, an unknown effect
   type is silently not drawn. Documented in `agent::GUIDE` "Effects" and `app/AI.md`.
7. **Inspector.** `panels.rs:972` `effects()` lists effects, `effect_fields` (`panels.rs:1068`) has one arm per
   `EffectKind` (`effects.rs`); shader uniforms get controls from `@label/@range/@color` notes. An "Appearance"
   section holds opacity, radius, clip. A new property needs a section function in the list at `panels.rs:435`.
8. **Exports.** PNG / JPEG / WebP / PDF (a picture) through `snapshot`, effects and shaders included, transparent
   background. SVG (`render/svg_out.rs`): drop shadow and layer blur only; background blur, inner shadow and
   shaders are left out and listed in `MECHANICS.md` (Export, SVG). **No code export**: the Code section is a stub
   (`panels.rs:1337`). No thumbnails.
9. **Document mechanics.** No migrations; `version` is the schema string ("2.22"), unknown keys
   round-trip through `extra` (`model.rs:360`). Undo stores whole documents (`editor.rs:168`), copy/paste and
   overrides work on whole nodes / JSON merges (`pen-doc/src/edit.rs:74`, `scene.rs:93`), so a new node key is
   picked up by all three without extra work.

Other facts: `reference/liquid-glass/` does not exist. `craft::CRAFT` lists "glass blur as decoration" as a tell
and `review.rs:1090` flags three or more background blurs; both need a rule for deliberate glass. The gate was
not run in Phase 1.

## Decisions

- **Branch**: a worktree on a new branch, because the session rules forbid switching the shared checkout and
  the task wants a new branch. Nothing is pushed.
- **Shader**: GPU, through the existing `shader.rs`/`gpu.rs` path (`shader::compile` on an embedded WebGL-1
  file). Frosted is all CPU; glass without a GPU falls back to the CPU frost with a lit edge.
- **Rim**: the shader gets two pictures of the backdrop, the frosted one and a lightly blurred one
  (`RIM_BLUR` = 0.2 of the blur); the bevel shows the sharp one, else a 12-unit blur hides all bending.
- **Units**: `blur` means what `background_blur.radius` means here (sigma = radius / 2), so the frosted presets
  are 24/40/60/80, twice the task's starting values (which read as sigmas).
- **Auto appearance**: mean linear luminance of the blurred backdrop under the layer; dark below 0.22.
- **Tint**: `material.tint` (colour or `$variable`); the layer's `fill` still paints on top of the material.
- **Retuning by variables**: `glass-regular-blur`, `frosted-thin-tint-opacity`, ...; `glass-light-angle` for the
  whole document. No new document key was needed for the light.
- **No format version bump**: the repo has no migrations; `material` is an additive key older readers keep.
- **Reference screenshots**: none present; working from the task's descriptions.
- **Skill format**: Agent Skills `SKILL.md` under `app/skills/<name>/`, embedded in the binary. Spec-strict
  front matter (name = folder, description under 1024) so every client loads it.
- **Harness delivery** (docs checked 14 Nov 2026 by a research subagent): all six clients load SKILL.md
  folders; `~/.agents/skills` covers Codex, Cursor, Gemini CLI, OpenCode, Copilot; `~/.claude/skills` covers
  Claude Code. `pencraft skills install` writes both.
- **Glass groups**: `glassGroup: <spacing>` on a plain frame; its liquid-glass children (max 8, unturned)
  are one pane with the first child's material; a tinted button must stay outside.
- **Accessibility looks**: a `Glass` theme axis whose themed variables (`glass-regular-tint-opacity`, ...)
  have no plain value, so depth presets stay intact in the default look.
- **Review/advice**: text on a material is no longer judged against a frame colour; `check_glass` measures it.

