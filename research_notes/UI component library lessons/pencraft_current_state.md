# Pencraft today: everything that shapes the quality of AI-made designs

Scope: read-only study of the source at `C:\Users\asp3x\Documents\development\Pencraft` (master, commit f9e8ad1), 14 Nov 2026. Paths are relative to the repo root; "file:line" are line numbers in the working tree. No `.pen` file was read. Not read in full: `docs/liquid-glass/TASK.md` (349 lines, one heading), `app/src/conversation.rs` (694 lines; a search found no prompt text in it), `app/src/material.rs`, `app/src/theme.rs` beyond its constants, `references/design-language.md` (headings only), the tests at the bottom of `review.rs` (from line 1177).

Map of the pieces (who is told what, and when):

| Piece | Where | Reaches the model |
|---|---|---|
| Format guide `GUIDE` | `app/src/agent.rs:21-114` | `get_guide` (all MCP clients); in the system prompt of the API chat route only |
| Skills index | `app/src/skills.rs:76-88` | `get_guide` only |
| Design method `CRAFT` ("Designing well") | `app/src/craft.rs:20-169` | `get_guide`; system prompt of both chat routes |
| Installed fonts + figure advice | `app/src/craft.rs:242-252` | `get_guide`; system prompt of both chat routes |
| Chat brief `BRIEF` + `PLAIN` | `app/src/chat.rs:42-55` | system prompt of the in-app chat |
| Dealt directions (3 worlds, a ground, a lead face) | `app/src/craft.rs:173-238` | first user message of each in-app conversation |
| `advice` after each edit | `app/src/advice.rs`, `review.rs:1156` | answer of every `edit_layers` |
| Counted `review` + art-director questions | `app/src/review.rs`, `craft.rs:262-280` | answer of every `screenshot` |
| `check_glass` | `app/src/glass_check.rs` | on demand |
| Second-model reviewer (`BRIEF`, `PROOF`) | `app/src/chat/critic.rs:31-52` | in-app chat only, after a run that wrote 30+ layers |
| Skill `liquid-glass` | `app/skills/liquid-glass/` | `read_skill`, or installed to the client |
| MCP `initialize` instructions | `app/src/link.rs:266` | every MCP client |

## 1. The guide returned by get_guide: where it lives, its structure, every rule

### Takeaway
`get_guide` returns one concatenated text, `agent::GUIDE` + skills index + `craft::CRAFT` + installed fonts (`app/src/craft.rs:256-258`, served at `app/src/app/tools.rs:174`). It is strong on method (decide a direction, set variables, hierarchy, anti-patterns, self-critique) and on file format, and contains almost no component-level knowledge: the only control dimensions are button padding 8-12, chip padding 4, 8 clearance, and "about 44 high" on a phone.

### Cited Findings

Assembly and delivery
- `guide()` is `format!("{}\n{}\n{CRAFT}\n{}", agent::GUIDE, skills::index(), fonts())` — `app/src/craft.rs:256-258`.
- The MCP server's `initialize` instructions tell clients: "Call get_state first, read get_guide before writing layers ... act on the review that comes with the picture." — `app/src/link.rs:266`.
- The `get_guide` tool description: "How a Pencraft document is built ... how to design well in it, and which fonts are installed. Read it once before writing layers." — `app/src/agent.rs:129-131`.
- `craft.rs` header names its sources: Anthropic's "Improving frontend design through skills", the frontend-design skill, "Harness design for long-running application development", the Impeccable catalogue of anti-patterns (skill v4.6) and the Vercel web interface guidelines — `app/src/craft.rs:13-19`. No component library is cited anywhere in the source.

Part A, `GUIDE` "Working on a Pencraft document" (`app/src/agent.rs:21-114`), section by section
- Intro (23-24): read with `read_layers`, change with `edit_layers`, look with `screenshot`; "Call `get_state` first."
- Layers (26-46): common properties `name, x, y, width, height, rotation, opacity, enabled, fill, stroke, strokeWidth, effect`; types frame, group, rectangle, ellipse, polygon, line, path, text, icon, note, ref. Frame layout is "none" | "horizontal" | "vertical" (default horizontal), with `gap`, `padding` (1, 2 or 4 values), `justifyContent` (start, center, end, space_between, space_around), `alignItems` (start, center, end), `cornerRadius` "(number or four numbers)", `clip`. Text: `content, fontFamily, fontSize, fontWeight ("100" to "900"), fontStyle ("italic"), lineHeight (multiple of size), letterSpacing, textAlign, textGrowth (auto | fixed-width | fixed-width-height)`; "The text colour is its `fill`." Icon: `library` ("lucide"), `icon` name, sized by width/height, coloured by `fill`.
- Size and position (48-52): numbers, "fill_container" or "fit_content"; "Prefer layouts over hand-placed positions; they keep spacing even."
- Fill and stroke (54-61): `#rrggbb`/`#rrggbbaa`; one paint or a list; gradient (linear | radial | angular, rotation, colour stops) and image (`url`, mode cover | contain | stretch). `strokeWidth` is a number or `{top,right,bottom,left}`; `strokeAlignment` "inner" | "center" | "outer".
- Effects (63-66): shadow (`shadowType` outer | inner, color, offset, blur), `blur` (layer), `background_blur`; "one effect or a list".
- Materials (68-78): `frosted` (thickness ultra_thin..thick) and `liquid_glass` (variant regular | clear), `appearance`, `tint`, `depth`, `glassGroup`; "read the `liquid-glass` skill (`read_skill`) before using one, and run `check_glass` on the result."
- Variables and themes (80-86): types color | number | string | boolean; `"$name"`; theme axes such as `{"Mode":["Light","Dark"]}`; per-theme values; a frame with `"theme":{"Mode":"Dark"}` shows its subtree in that theme. "Reuse the variables a document already has."
- Components (88-95): `"reusable": true`; `{"type":"ref","ref":id}`; overrides on the ref or under `descendants`; an entry with `type` replaces the layer, with `children` fills a slot. "Look for existing components before building the same thing twice."
- Working well (97-113): read before writing; one `edit_layers` call per meaningful step; `$0`/`$1` references; screenshot after a change and "put it right"; "Make what was asked for and nothing around it" (no backdrop, board, card or coloured frame behind a requested thing); "Never give a layer the fill of what is behind it"; nothing is saved until `save_document`.
- Omitted from `GUIDE` although the model supports them (see section 4): `layoutPosition: "absolute"`, `underline`, `strikethrough`, `textAlignVertical`, `textAlign: "justify"`, `flipX/flipY`, ellipse `innerRadius/startAngle/sweepAngle`, the `shader` effect, the `context` field, `slot`.

Part B, skills index (`app/src/skills.rs:76-88`): heading "## Skills", one sentence ("When a request fits one, read it with `read_skill` before you design, and follow it."), then one bullet per skill with its front-matter description. Today that is one bullet, `liquid-glass`.

Part C, `CRAFT` "# Designing well" (`app/src/craft.rs:20-169`)
- Preamble (22-26): applies to anything "with a look of its own"; for a small change "skip this and match what is there"; "What the user asks for outright always comes first: if they want a violet gradient, they get one."
- 1. Decide before you draw (28-57): classify the surface as (a) something that has to win a person over, (b) a screen someone works in, (c) a page someone reads. For working screens the look "lends four things only: type, palette, density and one signature move" and "never supplies the layout, the navigation or the controls: those are the standard ones people already know" (35-38); "One quiet family can carry everything, in steps of about 1.2" (38-39); "a grey screen with one accent is the stock tool without its polish, not restraint" (40). Find a direction from "five to seven real things the audience knows by heart" (49); write three lines before the first layer (54-56); "Dark or light is never a default" (56-57).
- 2. Set the system first, as variables (59-75): existing variables/components are the system; otherwise `set_variables` first. Colour strategy: restrained, committed, full palette, drenched (62-67). Roles: "a background, a surface, text, quieter text, a line, and the accent" (68); "One family of greys, tinted slightly toward the dominant colour"; "Red, amber and green mean states and never decorate" (70-71). Type: "at most two families"; "A scale of five to seven sizes and nothing between them" (72-74). Space: "steps of 4 (4, 8, 12, 16, 24, 32, 48, 64, 96). Corners: one radius for controls, one for cards." (75)
- 3. Build with a hierarchy (77-116): size jumps of two to three times, weights far apart (300 vs 700/800) (78-80); headlines line height 1.0-1.15, tracking never tighter than 4% of size; running text 15-18 px, line height 1.4-1.6, 45-75 characters (81-83); shared edges, centre only short lone things, "More space above a heading than below it" (84-86); "One main action per view"; "at most four choices side by side" (87-88); control room: "8 to 12 above and below a button's label (4 in a small chip), 8 or more between a button and the text around it, and on a phone whatever is tapped is about 44 high. What someone is meant to press looks like a button" (89-91); optical centring of initials/figures in circles, check at `scale: 2` (91-92); "Set a surface apart in one way only: a line, or a shadow, or a tint. Not all three, and no card inside a card. A shadow has an offset and a soft blur, in a dark shade of the ground." (95-96); real copy with uneven numbers, buttons that name their action, "one state worth seeing (an overdue row, a warning, a very long name)" (97-100); "Icons: `icon` layers from one set at one size per role. Never emoji." (101); one confident drawn picture, never a grey photo placeholder (102-105); print-proof finish: figures "one fixed width and `textAlign: "right"`", same kind of thing same size/weight/colour, "a colour means one thing", panels share top and bottom edges, lists end 16 or more above a bar with no rule under the last row, no half-empty panel (106-116).
- 4. What gives a generated design away (118-138): violet/indigo gradients, gradient text, glowing blobs, coloured glow, decorative glass blur, hard block shadow; three equal cards; centred hero with two buttons; big number over small label x3; fake charts; spaced-capitals eyebrow; 01/02/03 numbering; one coloured word in a headline; icon in a rounded square over every heading; coloured side stripe; Inter/Roboto/Arial by default; monospace to look technical; the "next defaults" in faces (Fraunces, Playfair Display, Cormorant, Lora, Space Grotesk, Syne, DM Sans, Outfit, Plus Jakarta Sans, IBM Plex) and four stock looks; sales words; Lorem ipsum, John Doe, Acme.
- 5. Look, and judge it as someone else's work (140-151): one screenshot of the whole; fix the counted `review` first; four criteria (whole with a mood; made for this subject; craft; function), "the first two weighing most" but for a working screen function "weighs most" (147-148); "Two looks are the ceiling"; "Before you finish, take one thing away."
- 6. Changing a design that is already there (153-168): see section 2 below.

Part D, fonts (`app/src/craft.rs:242-252`): up to 160 installed families with weights (`MOST_FAMILIES`, line 10), plus figure advice: "prices, amounts and anything in a column need lining figures of one width"; Georgia, Constantia, Corbel, Candara have old-style figures; fixed-width faces only for small figures, codes and times (249).

### Inferences
- The guide's only token scale that is spelled out numerically is spacing (4..96). Type scale, radius scale, shadow levels and colour steps are given as counts or rules of thumb ("five to seven sizes", "one radius for controls, one for cards"), not as values.
- The guide tells a model that working screens must use "the standard" navigation and controls (craft.rs:37-38) but nowhere says what those standard controls look like; that is the slot external component-library knowledge would fill.
- A model that reads only the tool descriptions and skips `get_guide` gets none of the design method; the in-app chat avoids this by putting `CRAFT` in the system prompt.

### Gaps
- I did not check how long the full `get_guide` text is at run time (it depends on the installed fonts). `craft.rs` is 23.8 KB including code; `CRAFT` itself is about 150 lines.

## 2. The in-app chat's system prompt: understanding, clarifying, vocabulary, following the request

### Takeaway
The chat's system prompt is `BRIEF` + `CRAFT` + fonts (+ `GUIDE` on the API route) + a closing no-Markdown reminder. Clarifying is one sentence ("ask one short question instead of guessing"); the request-comes-first rule is repeated in several places; "bolder / quieter / simpler / polish / what do you think" each have a fixed meaning in `CRAFT` section 6.

### Cited Findings
- `brief(guide)` is `format!("{BRIEF}\n\n{}\n{}\n{guide}\n{PLAIN}", CRAFT, fonts())` — `app/src/chat.rs:59-63`. The API route calls `brief(true)` as the `system` field (`app/src/chat/api.rs:104`); the Claude Code route calls `brief(false)` through `--append-system-prompt` (`app/src/chat/code.rs:48`). The skills index is not part of the brief; skills are named only inside `GUIDE` (Materials, agent.rs:77-78) and `CRAFT` (craft.rs:122).
- `BRIEF` (`app/src/chat.rs:42-50`): tool order (get_state, read, edit with description, screenshot before saying done); "When edit_layers answers with advice, or a screenshot comes with a review, act on it."; follow "Designing well"; "Never fall back on your defaults (Inter or Arial, a violet gradient, three equal cards, everything centred)."; reuse variables and components; "Make what was asked for and nothing around it"; pasted layer chips mean "work on exactly those layers"; `<pencraft_context>` gives the document and selection, and "this" means the selection; replies short and plain, no Markdown.
- Clarifying: "If a request is unclear in a way that changes the design, ask one short question instead of guessing." — `app/src/chat.rs:50`. Also in `CRAFT` 6: "if the design's own system cannot do what is asked, ask before adding a colour, a face or a radius" — `app/src/craft.rs:155-156`.
- Following the request: "What the user asks for outright always comes first" (craft.rs:25-26); "The user's own words can earn any of these" anti-patterns (craft.rs:119); "A change keeps what is there: its look, its copy, its variables, and everything the user did not name. Touch only what was named" (craft.rs:154-155); the reviewer's follow-up says "the user's own words come first where the two differ" (`app/src/chat/critic.rs:107`).
- Vocabulary (`app/src/craft.rs:157-168`): "Bolder" = bring the part up to the design's own strongest moves, "make one decisive move completely and quiet what is around it ... Not more effects."; "Quieter"/"calmer" = fewer colours, less saturation, lighter weights, more room, "quiet is not grey"; "Simpler"/"cleaner" = "take away"; "Polish"/"tidy up" = "no new ideas. Shared edges, the spacing and type scales, the same thing looking the same everywhere, contrast, real copy"; "What do you think?"/"review this" = "change nothing", name three things that matter most. The same list is documented for users in `app/AI.md:24-27`.
- Per-message context: `<pencraft_context>` with the document in front and the selection — `app/src/app/talk.rs:62-68`.
- First message of a conversation also carries `<pencraft_directions>`: three of 24 "worlds" (craft.rs:173-198), one of 5 grounds (202-208) and one of 6 lead-face kinds (210-217), picked from the clock (`chat.rs:377-379`). The ground and lead face are "not yours to choose this time"; "Only the user's own words or the document's existing variables overrule them." — `app/src/craft.rs:230`.
- The API route offers every tool from `agent::tools()` (`app/src/chat/api.rs:80-81`), which lists twelve (`app/src/agent.rs:676-688`); `app/MECHANICS.md:236` still says "the nine tools".

### Inferences
- The dealt directions apply to every first message, including requests for a plain component or a working screen; the text limits them ("on a screen someone works in ... it lends only type, palette, density and one signature move") but a forced ground and lead face may still pull a simple "make me a settings dialog" away from conventional component styling.
- There is no instruction about restating or checking understanding of a request beyond the single clarifying question, and no vocabulary entries for component-level asks ("make it denser", "more compact", "add states", "make it accessible", "dark mode").

### Gaps
- `app/src/conversation.rs` was not read; a text search for prompt strings in it found none.

## 3. The review: every check, measured or model-judged, with thresholds

### Takeaway
Three automatic layers exist: `advice` on every edit (same-colour and hard mistakes), a counted `review` on every `screenshot` (about 35 checks, capped at 10 notes), and a second-model art-director review after large chat runs (two rounds plus a proofread). All counted checks work on static layer geometry and colours; none knows what a component is beyond "a small filled box with one text" (a control) and "a rounded or outlined frame at least 120 x 64" (a card).

### Cited Findings

A. `advice` in the `edit_layers` answer
- Same-colour check (`app/src/advice.rs`): a touched layer whose top fill is within 3 per channel (`SAME_COLOUR`, line 13) of the nearest covering frame fill behind it (alpha >= 128, `COVERS`, line 15); icons, paths and lines are skipped (line 76); layers on a material are skipped (line 86). Text gets "can't be read"; shapes get "don't stand out". At most 5 names per note and 8 notes (lines 17-19).
- Plus `review::mistakes_in` on the changed top-level layers (`app/src/app/tools.rs:272-279`, `app/src/review.rs:1156-1165`): the "mistakes" group below.

B. Counted `review` with each `screenshot` (`app/src/review.rs:1138-1151`; attached in `app/src/app/tools.rs:312-315`). Order: mistakes, colour, build, type; truncated to 10 notes (`MOST_NOTES`, line 17), 4 names per note (line 19).

Mistakes (`review.rs:793-851`), always reported:
- Font family not installed, with the stand-in drawn (797-803).
- Contrast of text against the nearest covering frame fill: 4.5:1; 3:1 from 24 px; 6:1 under 13 px (constants 22-31; check 804-825). Text on a gradient, image or material is not measured (413-418).
- Layers that do not fit: sticking more than 1.5 out of the parent's padded box (383-396); reaching more than 1.5 over a filled, non-clipping top-level frame (432-462); a text overlapping a neighbour (across > 1.5 and down > 25% of the smaller height) (506-508); two texts in a layout touching, gap between -1.5 and 2 (510-514).
- Loose ends: content ending less than 16 above a bottom bar (`ROOM_ABOVE_A_BAR`, 592; a bar is a filled frame at least 90% wide, at the page bottom, at most 30% of its height, 632-633); a vertical list of 3+ rows where every row including the last has a bottom rule (659-670).
- Squeezed controls: a control is a filled or outlined frame at most 64 high (`TALLEST_CONTROL`, 522) holding one text and at most icons, up to 3 parts (531-541); flagged when the label (13 px or larger) has less than 4 above and below (`ROOM_OF_A_CONTROL`, 520; 563-568), or when a control sits less than 4 from a text in the next row (573-588). Note text: "8 to 12 in a button; on a phone whatever is tapped is about 44 high ... stands 8 or more clear" (836-838).
- Ragged columns: 3+ rows of equal part count where one part aligns on neither left, right nor middle (spread > 1.5 each) (674-697).
- Text under 11 px (`SMALLEST_TEXT`, 33; 847-850).

Colour notes (`review.rs:989-1044`):
- Violet or indigo gradient: any stop that is an accent (saturation >= 0.35, lightness 0.15-0.9, alpha >= 128) with hue 235-295 degrees (229-231, 990-996); otherwise more than 2 gradients (997).
- Gradient-filled text (1003-1006).
- Grey text (saturation < 0.12, lightness 0.3-0.85) on an accent-coloured surface (1008-1015).
- For designs of 6+ texts: more than 3 accent hues in 30-degree steps (1020-1034); more than 8 distinct colours written as raw hex instead of variables (`MOST_RAW_COLOURS`, 47; 1035-1043).

Build notes (`review.rs:1047-1108`):
- Stock feature grid: a horizontal frame of 3-4 boxed children of identical build, each with an icon and 2+ texts (700-721).
- Stock statistics row: 3-4 children each holding only a figure at least 1.8 times its label's size (`A_FIGURE`, 73; 730-753).
- Coloured glow: a vivid shadow with no offset and blur > 0.5 (257-264); block shadow: offset with blur < 0.5, reported from 2 layers (265-267, 1066).
- Cards nested 3 deep (card = frame with children, outline or own fill, rounded or outlined, at least 120 x 64; 273-287; 1073).
- Outline and shadow at once on 2+ frames (324-326, 1079).
- Side stripe: a one-sided stroke of 3 or more on a layer at least 40 high (327-332).
- 3+ `background_blur` effects (1092).
- Spacing: more than 8 distinct gaps/paddings (`MOST_SPACINGS`, 39) or more than one odd value (1095-1102).
- More than 4 distinct corner radii, pills excluded (`MOST_RADII`, 37; 316-319; 1103-1106); the note adds "an inner corner smaller than the one around it".
- Copy: emoji, stand-in copy (10 phrases, 90-91), sales words (12 stems, 75-88) (1111-1133).

Type notes, only for 6+ texts (`A_DESIGN`, 41; `review.rs:908-986`):
- Near-miss alignment: a text 1.5-6 off a left edge, or 1.5-16 off a right edge, shared by 3+ texts of its size (855-905).
- One family from the default list (inter, arial, roboto, helvetica, helvetica neue, segoe ui, open sans, system-ui, sans-serif; 49) (917-921); more than 2 families (922); any of 15 "next" families (52-68; 925-931).
- Letter spacing tighter than -4% of size (932-938); sections numbered 01/02/03 (939-948); more than 7 font sizes (949-952); flat hierarchy: largest/smallest < 1.6, one weight, one colour (956-964); more than 75% of texts centred (965-971); wrapped lines over about 80 characters in texts longer than 160 characters (972-979); 2+ spaced-capitals eyebrows over a line twice their size (756-767, 980-985).

After the notes, `with_picture` appends the four art-director questions when there is any note or the picture holds 12+ layers (`app/src/craft.rs:262-280`).

C. `check_glass` (`app/src/glass_check.rs`; summary `app/MECHANICS.md:220`): contrast of every text and icon against the lightest and darkest 3% of the pixels under it, 4.5:1, or 3:1 for text from 24 px (`LEAST_CONTRAST` 21, `LEAST_LARGE` 55, `TRIM` 29); glass on glass (208); overlapping glass outside a group (220); regular and clear mixed in one screen (238); nested corners not concentric, slack 1.5 (25, 272); flat backdrop, lightness range under 0.008 (27, 306); glass on an unsupported shape (198). At most 24 problems. `opacity`-dimmed text is not measured (`app/skills/liquid-glass/references/components.md:109-110`).

D. Second-model reviewer, in-app chat only (`app/src/chat/critic.rs`, loop in `app/src/chat.rs:426-449`)
- Trigger: the run wrote 30+ layers since the last review (`WORTH_A_REVIEW`, `app/src/app/tools.rs:27`, 355); later rounds trigger on any change. On by default (`app/src/prefs.rs:136`).
- Input: one picture of the changed top-level layers, up to 4 largest parts (8+ layers each) at scale 2 (`tools.rs:23-25, 361-378`), the counted notes, and the user's request (critic.rs:94).
- `BRIEF` (critic.rs:31-43): classify the surface first; score 1-10 on design quality, originality (both "weigh double"), craft, function, with anchors at 4/6/8; craft list covers hierarchy, shared edges, spacing, readable text, shadows as depth, columns of figures, consistency, "small text and controls that are switched off can still be seen clearly against their ground"; "What the user asked for outright is never a fault"; "Do not ask for more sections, photographs, motion"; PASS needs quality, originality and function >= 7 and craft >= 8; REVISE lists at most four changes (up to eight slips when only craft fails).
- `PROOF` (critic.rs:48-52): last look, execution only, up to ten slips, including "initials or a label sitting off-centre in its circle, badge or button" and "a number that contradicts another".
- Rounds: `ROUNDS = 2` design reviews, then one proofread if the last still asked for changes (critic.rs:22; chat.rs:430-438). The reviewer is the same engine and model with no tools (`app/MECHANICS.md:242`).

### Inferences
- External MCP clients (Claude Code, Cursor, Codex) never get the second-model review; they get only the counted notes and the self-judging questions.
- No counted check looks at: touch-target size (44 is advice text only), focus indication, disabled-state contrast (model-judged only), presence of states, icon size consistency, icon/label gap, input height consistency across a form, shadow consistency (number of distinct shadows), line-height values, border contrast in dark themes, or number formatting.
- The contrast check has a known false positive: text over a sibling photo is judged against the frame's fill (`docs/liquid-glass/NOTES.md:35-36`).
- The radius check allows 4 distinct radii and only its note text mentions nesting; concentric-radius arithmetic is enforced only by `check_glass` for materials.

### Gaps
- The exact contrast formula (`Rgba::contrast`) lives in `pen-doc/src/color.rs`, not read; the code comments cite WCAG 2.1 SC 1.4.3 (`review.rs:21`).

## 4. Document model capabilities relevant to component libraries

### Takeaway
The model is a static flex-box tree: frames with one-direction layout, rectangles/ellipses/polygons/paths, text, Lucide icons, components with overrides and slots, variables with theme axes. It can express per-side strokes, inner/outer/center stroke alignment, stacked and inner shadows, per-corner radii and gradients. It cannot express variants, states, interaction, motion, wrapping/grid layout, min/max sizes, text truncation, OpenType features (no tabular numerals), shadow spread, blend modes or corner smoothing.

### Cited Findings

Layer types
- Kinds: frame, group, rectangle, ellipse, polygon, path, line, text, note, icon, icon_font, script, browser, ref, plus unknown kinds kept by name — `app/crates/pen-doc/src/model.rs:46-77`. Script layers and live browser nodes are listed under "Not in this version" — `app/MECHANICS.md:252-253`.
- Unknown keys round-trip through `extra` — `model.rs:363-365`; `app/MECHANICS.md:12`.

Layout
- `layout` none | vertical | horizontal (frames default horizontal), `gap`, `padding` (1, 2 or 4 values), `justifyContent` (start, center, end, space_between, space_around), `alignItems` (start, center, end only) — `model.rs:252-266, 482-530`.
- Sizes are a number, variable, `fit_content` or `fill_container` — `model.rs:243-248`. `layoutPosition: "absolute"` takes a child out of the flow — `model.rs:237-239, 464-468` (not mentioned in `GUIDE`).
- A text search of `pen-doc/src` for wrap, minWidth, maxWidth, truncat, maxLines, grid found nothing (search run for this note); layout rules in `app/MECHANICS.md:27-38` describe fit, fill and positions only.
- `clip` cuts children at the frame edge, including its rounded outline — `model.rs:249-251`; `docs/liquid-glass/NOTES.md:25-27`.

Fills
- One paint or a list, bottom first; a paint is a colour/variable string or an object of type `color`, `gradient`, `image`, `shader`, `mesh_gradient` — `model.rs:137-166`.
- Gradients: linear, radial, angular, on fills and strokes — `agent.rs:56-59`; `NOTES.md:51-52`. Mesh gradients draw the first colour only; shader fills are not in this version — `NOTES.md:55-56`; `MECHANICS.md:252`.
- Image fills: local path only, modes cover/contain/stretch, optional crop `transform`; unreadable picture is a grey `#d9d9d9` placeholder; nothing is fetched from the network — `MECHANICS.md:193-197`.
- Per-paint opacity exists — `NOTES.md:53`.

Strokes
- `strokeWidth` uniform or per side `{top,right,bottom,left}`; a missing side has no stroke — `model.rs:188-196`.
- `strokeAlignment` inner | center | outer; default center — `model.rs:276-278`; `MECHANICS.md:55`.
- Strokes take the same paints as fills (gradients included) — `model.rs:270-272`.
- No dash pattern field exists in `Node`; `strokeLinejoin`/`strokeLinecap` appear only as extra keys on a sample path — `app/src/samples.rs:138-139`.

Effects
- Kinds: drop shadow, inner shadow, layer blur, background blur, shader — `app/src/effects.rs:9-37`.
- Shadows and shaders can be stacked; one layer blur and one background blur per layer — `effects.rs:64-70`; `MECHANICS.md:61`.
- Shadow fields: colour, offset x/y, blur; "`spread` is not read" — `MECHANICS.md:59`.
- Effect order: background blur, drop shadows, fill, inner shadows, stroke, children — `MECHANICS.md:62`.
- Effects are free JSON; nothing validates them and an unknown type is silently not drawn — `NOTES.md:72-74`; `model.rs:279-281`.
- Blend modes are kept on save but not drawn — `NOTES.md:55`; `MECHANICS.md:252`.

Corner radius
- One number or `[top-left, top-right, bottom-right, bottom-left]` — `model.rs:178-186`; polygons take one radius for all corners — `MECHANICS.md:96`.
- "Pencraft has no corner smoothing; plain rounded corners are what you get." — `app/skills/liquid-glass/SKILL.md:109-110`; `NOTES.md:34`.

Text
- Fields: `content`, `textGrowth`, `fontFamily`, `fontSize`, `fontWeight`, `letterSpacing` (canvas units), `fontStyle`, `underline`, `strikethrough`, `lineHeight` (multiple of size), `textAlign` (left, center, right, justify), `textAlignVertical` (top, middle, bottom) — `model.rs:309-344`.
- Defaults: size 14, family "Inter", weight normal, line height the font's own — `MECHANICS.md:22-24`.
- `TextStyle` carries content, family, size, line height, letter spacing, weight, italic — `app/crates/pen-doc/src/layout.rs:38-50, 126-138`. A search of `pen-doc/src` for tabular, fontFeature, openType, tnum, textTransform found nothing. The guide works around this with per-family figure advice (`craft.rs:249`) and fixed-width right-aligned columns (`craft.rs:107-108`).
- One style per text layer (the node holds a single set of font fields; no rich-text runs in `Node`).
- Weight picks the nearest installed face; variable fonts are set to the weight — `MECHANICS.md:218`.

Variables and themes
- Types color, number, string, boolean; `$name` anywhere a value goes; a value list picks the last entry whose `theme` matches; chains `$a -> $b` up to 8 deep — `app/crates/pen-doc/src/vars.rs:9-10, 76-98`; `agent.rs:80-86`.
- Any number of theme axes; a node's `theme` overrides axes for its subtree — `vars.rs:18-41`; the glass skill uses two axes together (`Mode` and `Glass`) — `app/skills/liquid-glass/references/tokens.md:10, 58`.
- A colour variable cannot take an alpha at the use site ("`$bg` cannot take an alpha, so define `bg-clear` and `bg-fade`") — `SKILL.md:165-166`.
- Documents can declare `imports` of other `.pen` files by alias — `model.rs:26-28` (how far this is implemented was not checked).

Components
- `reusable: true` makes a component; `ref` instances it; overrides sit on the ref or in `descendants` keyed by id, unique name, or `nestedInstanceId/innerId`; with `type` the node is replaced, with `children` its children are — `MECHANICS.md:119-120`; `agent.rs:88-95`.
- Slots: an empty frame in a component marked `slot: []`, filled through a `children` override — `MECHANICS.md:126-127`; `app/crates/pen-doc/src/scene.rs:108, 238-256`.
- "No: ... component variants" — `NOTES.md:55-57`. A search of `pen-doc/src` for variant, hover, interaction, prototype, animation, transition found only the glass material's `variant` key.
- States are shown as separate frames: "Pressed, morphing or appearing glass is shown as another frame beside the screen, not animated" — `SKILL.md:94-96`; "no motion" — `NOTES.md:34`. Shader `@time` and `@mouse` stay 0 — `MECHANICS.md:252`.

Icons
- One bundled set: Lucide (ISC), from the `lucide-static-svg` crate, drawn as 2-unit round lines in a 24 box; `feather` names resolve to Lucide; 14 old names are aliased (home -> house, trash-2 -> trash, more-horizontal -> ellipsis, ...) — `app/src/icons.rs:1-38, 46-60`; `MECHANICS.md:199`.
- Other libraries (Material Symbols, Phosphor) and icon `weight` are not supported; an unknown icon or library draws a small rounded outline — `MECHANICS.md:200, 253`.
- The guide gives no list of valid icon names; a wrong name fails silently into the outline placeholder (no advice was found for it in `advice.rs` or `review.rs`).

Fonts
- "Nothing is downloaded or shipped": design text uses families installed on the computer; a missing family falls back to the first installed of Inter, Segoe UI, Helvetica Neue, Helvetica, Arial, Liberation Sans, DejaVu Sans (serif and mono lists likewise) — `app/src/fonts.rs:1-9`; `MECHANICS.md:218`.

Materials
- `frosted` and `liquid_glass` on frames, rectangles and whole ellipses, with `appearance`, `tint`, `depth`, `advanced`; `glassGroup` merges up to 8 children — `MECHANICS.md:67-70`.

Export
- PNG, JPEG, WebP, PDF (as a picture), SVG (subset); no code export — `MECHANICS.md:158-164`; `NOTES.md:80-81`.

### Inferences
- A component library's variant matrix (size x intent x state) has to be laid out in Pencraft as separate components or as instances with overrides on a board; a theme axis can stand in for an axis such as density or state because variables switch per subtree (the glass skill already uses a `Glass` axis this way for accessibility looks).
- Focus rings are expressible (outer-aligned stroke, or a stacked shadow with zero blur), but spread-based rings (the common `box-shadow: 0 0 0 3px`) are not, and a zero-blur offset shadow is flagged as a block shadow from two layers on (`review.rs:1066`), while a coloured zero-offset blurred ring is flagged as a glow (`review.rs:263`).
- Hairline dark-mode borders built from translucent white are expressible (`#ffffff1a` strokes), and inner 1 px highlights via inner shadow are expressible.
- Tabular alignment of changing numbers (NumberFlow-style) can only be approximated by choosing a face with lining tabular figures or a fixed-width right-aligned text box.
- Because there is no wrap, min/max or truncation, responsive behaviour and overflow handling ("very long name") must be drawn by hand as fixed states.

### Gaps
- `app/src/material.rs`, `scene.rs` and `layout.rs` were only sampled; details such as whether `imports` resolve components across files were not verified.
- Whether a `ref` can override `theme` per instance was seen used in the skill (`components.md:20-21`) but not traced in `scene.rs`.

## 5. The skills mechanism

### Takeaway
A skill is an Agent Skills folder (`SKILL.md` with front matter, plus reference files) compiled into the binary with `include_str!`; adding one means adding a folder and one entry in a Rust constant. The one existing skill, `liquid-glass`, is about 48 KB over four files and was developed with fresh-agent tests whose friction reports drove fixes.

### Cited Findings
- Registration: `pub const ALL: &[Skill]` with one `Skill { name, files: &[(path, include_str!(...))] }`; the first file must be `SKILL.md` — `app/src/skills.rs:23-39`.
- Front matter read by the app: `name`, `description` (`skills.rs:43-53`). The liquid-glass front matter also has `license` and `metadata` (`app/skills/liquid-glass/SKILL.md:1-8`).
- Three delivery routes with the same text: the guide's index (`skills.rs:76-88`), the tools `list_skills` (92-94) and `read_skill` with optional `file` (100-109), and `pencraft skills install` which writes to `~/.agents/skills` (Codex, Cursor, Gemini CLI, OpenCode, GitHub Copilot) and `~/.claude/skills` (Claude Code), or `--project DIR` / `--dir DIR` (`skills.rs:19-20, 116-130, 169-195`; `app/AI.md:99-116`).
- Format rules enforced by a unit test: file starts with `---`; name equals folder, lowercase letters, digits and single hyphens, at most 64 characters; description 40-1024 characters without `<` or `>`; main file under 500 lines; every `references/*.md` named in any file exists, and every file is named in `SKILL.md` — `skills.rs:232-257`.
- A second test parses every fenced `json` block in every skill file as a Pencraft layer and validates its material and glass group; it requires more than 50 layers in total across skills — `skills.rs:260-293`.
- Sizes of the existing skill: `SKILL.md` 15,449 bytes (213 lines); `references/components.md` 20,121 (247 lines); `references/tokens.md` 7,270 (116 lines); `references/design-language.md` 5,065 (80 lines) — measured with `wc` for this note.
- Structure of `SKILL.md` (`app/skills/liquid-glass/SKILL.md`): 1 decide what each surface is made of (table) (23-44); 2 something to bend (46-55); 3 apply the material (key table) (57-79); 4 what a live system would adapt (81-96); 5 shape: capsule, concentric (inner radius = outer - gap), fixed (98-110); 6 bars (112-133); 7 glass groups (135-148); 8 scroll edge (150-168); 9 colour and legibility (170-181); 10 workflow with a final checklist (183-199); 11 "When it looks wrong" symptom/cause/fix table (201-211).
- `references/components.md` holds ready-to-insert reusable layers with stable `glass-` ids: tab bar with search, toolbar, round and prominent button, segmented control, search field, slider and toggle (rest and pressed as separate frames), half-height sheet, alert, menu/popover/action sheet, sidebar, media transport, toast, frosted panel — headings at lines 28, 54, 73, 87, 101, 112, 134, 149, 166, 187, 209, 227, 235. Rules: components use the token variables; texts carry no `fontFamily`; "Spacing follows steps of 4"; insert once on a board and instance (lines 3-26).
- `references/tokens.md` gives one `set_variables` call with `Mode` and `Glass` axes (8-28), the four accessibility looks (45-56), preset retuning by variable name (71-82), the `advanced` number table (84-100), and concrete sizes: desktop toolbar controls 32-36 high, sidebar rows 32 with 13 px labels and 16 px icons, menus 220-260 wide with 32-high rows; phone 393 x 852, margins 16, bar buttons 44, tab bar 62, tab label 11, titles 34/17, body 17, secondary 15, captions 13, sheet radius 34-40, alert 30, menu 22, cards 20-28, icons 22-24 (106-116).
- The skill explicitly overrides two general rules: lists ending above a bar ("under glass bars that rule is reversed on purpose, and the review's remark about it can stand", `SKILL.md:52-53`) and the gradient count ("Pencraft's review counts these gradients; that remark can stand", `SKILL.md:166`). `CRAFT` carves glass out of its anti-pattern list (`craft.rs:121-122`).
- Testing approach (`docs/liquid-glass/tests/README.md`): each test gives a fresh agent only a user request, the public tools and what they return; the agent must not read the repo (`tests/prompts/1.md:15-22`); it reports "every place where you had to guess, got an error ... needed more than one attempt, found the guide, the skill or a tool unclear" (`prompts/1.md:31-35`). Harness: `pc.py` calls a tool from a shell, `start.sh N`/`stop.sh N` run a private Pencraft per test with its own `PENCRAFT_HOME` (README 13-18). Five scenarios (README 41-46), two rounds; round 1 friction and the fixes it caused are listed (README 50-59). Pass criteria: `check_glass` ok, a unit test on rim bending, and a by-eye check (README 63-68). Outputs kept under `tests/out/N` and `tests/renders/`.
- Process record: `docs/liquid-glass/NOTES.md` has status, open follow-ups (29-39), Phase 1 findings about the codebase (41-89) and decisions (91-116); before this work "Skill system: none" (`NOTES.md:43-45`).
- In the app's own chat composer the hint text reads "/ for skills" (`app/src/sidebar.rs:30`), but "chips for skills" are listed as not in this version (`app/MECHANICS.md:252`).

### Inferences
- A new skill (for example a component-anatomy or app-UI skill) would need: a folder under `app/skills/<name>/`, a `Skill` entry in `ALL`, a row in `app/AI.md` and `MECHANICS.md`, and its JSON examples must parse as real layers. The existing tests would then cover it automatically, but the material-specific assertions add nothing for a non-glass skill; there is no generic check that example layers pass the counted review.
- The 500-line and 1024-character limits, plus the pattern of a short decision-oriented main file with `references/` for components and tokens, define the shape external lessons would have to take. Reference files are read on demand, so a large component catalogue costs tokens only when asked for.
- The skill index line is the only trigger: a skill is read only if its `description` matches the request in the model's judgement. A general "UI components" skill would compete with `CRAFT`, which is always loaded.
- The liquid-glass component file is the only place in Pencraft where concrete control anatomy exists (heights 36/44/50/62, paddings, radii, label sizes), and it is specific to iOS-26-style glass.

### Gaps
- `docs/liquid-glass/TASK.md` (the original task text) was not read.
- Whether `pencraft skills install` has been run on this machine was not checked (NOTES says it was not installed into the real home folder).

## 6. Samples, built-in components, palette.rs, theme.rs

### Takeaway
Pencraft ships no design-system or component kit for documents: the only built-in document content is one starter landing page with a single reusable button, and that starter itself shows several of the marks the guide and review tell models to avoid. `palette.rs` is the app's command palette, not colours; `theme.rs` holds the editor's own UI tokens and is not exposed to models.

### Cited Findings
- `STARTER` (`app/src/samples.rs:4-153`): a 1200-wide landing page with a nav row, hero (64 px headline, letter spacing -1.5, line height 1.1), two button instances, three equal feature cards, a note and an arrow path. One component: "Button" (`reusable`, padding [12, 22], radius 10, fill `$--accent`, 15 px/600 label) at lines 8-22; the second instance overrides fill, stroke and label to make a secondary button (line 75).
- Starter variables: `--bg`, `--card`, `--fg`, `--muted`, `--border` with Light/Dark values, and `--accent` `#4f46e5` (146-151); themes `Mode: [Light, Dark]` (142-144).
- The starter uses Inter for every text (lines 20-111), three equal cards each with outline plus shadow (88-113), gradient-filled icon shapes including `#6366f1 -> #22d3ee` (91), and an indigo accent.
- `app/src/palette.rs` is "The action search: every command of the app in one list" with 74 actions — `palette.rs:1-2, 149`.
- `app/src/theme.rs`: "The editor's colours and sizes, in one place" (line 1). Sizes include `CONTROL_RADIUS` 6 (65), `ISLAND_RADIUS` 14 (29), `CHIP_RADIUS` 8 (39), `CHIP_HEIGHT` 29 (41), `SIDEBAR_ROW` 28 (37), `ICON_SIZE` 16 (43), `ICON_LINE` 1.3 (45), text 13/13.5 (47-51). Palettes mirror shadcn-style token names in their comments (`--island-bg`, `--accent`, `--input`, `--primary`, `--primary-foreground`; lines 225-236): light background `#fafafa`, muted `#71717a`, border `#e4e4e7`, primary `#18181b`; dark background `#18181b`, island `#27272a`, hover/input `#3f3f46`, outline `#3a3a3e`, primary `#fafafa` (239-266). Accent `#3b82f6` (67); status green `#22c55e` and amber `#f59e0b` (343-345); problem red `#dc2626` (101).
- `design/build_design.py` builds Pencraft's own design file by driving `pencraft mcp` with builder helpers (lines 1-40 read); `design/pencraft.pen` was not opened, as instructed.
- New-layer defaults in the editor: shapes fill `#CCCCCC`, new frame `#FFFFFF` with clip on, pasted plain text 12 px Inter black, text tool 16 px — `app/MECHANICS.md:77-89, 112`.

### Inferences
- A model that opens the starter and follows "existing variables and components are the system" (`craft.rs:60-61`) inherits Inter, an indigo accent and the three-card grid; the review would then flag the starter's own gradient (hue about 239 degrees is inside the 235-295 violet band, `review.rs:229-231`) and its Inter-only type (`review.rs:917-921`).
- The app's own UI is styled on a zinc, shadcn-like token set, so a "neutral app UI" token vocabulary already exists in the codebase, but only as egui constants for the editor, not as document variables or as guidance for models.

### Gaps
- Whether the home screen offers more starters than `STARTER` was not traced beyond `samples.rs` (the file defines only this one constant).
- The contents of `design/pencraft.pen` and the rest of `build_design.py` are unread.

## 7. Gaps: what a model working in Pencraft is not told today

### Takeaway
Pencraft teaches direction, hierarchy, anti-slop and proof-level tidiness, and measures contrast, fit, alignment and scale counts. It does not teach or check the things component libraries encode: anatomy and dimensions of standard controls, state sets, stepped token scales, focus and dark-mode conventions, number formatting, and empty/loading/error states. The only component anatomy in the product is inside the glass skill.

### Cited Findings (what exists, to bound each gap)
- Control sizing in the general guide is limited to: button label room 8-12, chip 4, clearance 8, phone tap height about 44 — `app/src/craft.rs:89-91`; repeated in the review note — `app/src/review.rs:836-838`.
- States in the general guide: "Red, amber and green mean states and never decorate" (`craft.rs:70-71`); "one state worth seeing (an overdue row, a warning, a very long name)" (`craft.rs:98-100`); "a state (pending, overdue) is shown by a word or a mark beside the amount" (`craft.rs:115`). The reviewer mentions switched-off controls must stay visible (`critic.rs:38, 50`). The glass skill says other states are separate frames (`SKILL.md:94-96`) and marks an open menu's control with a selection fill (`SKILL.md:123`).
- Token roles named: background, surface, text, quieter text, line, accent (`craft.rs:68`; `review.rs:1040`). Spacing steps 4..96 (`craft.rs:75`). Radii: "one radius for controls, one for cards" (`craft.rs:75`), "an inner corner smaller than the one around it" (`review.rs:1105`). Shadows: "an offset and a soft blur, in a dark shade of the ground" (`craft.rs:96`). Type: "five to seven sizes" (`craft.rs:73-74`), steps of about 1.2 on working screens (`craft.rs:38-39`).
- Dark mode: "Dark or light is never a default" (`craft.rs:56-57`); theme axes are documented as a mechanism (`agent.rs:83-86`).
- Numbers: uneven realistic numbers (`craft.rs:97-98`); lining figures of one width, old-style and fixed-width cautions (`craft.rs:249`); right-aligned fixed-width figure columns (`craft.rs:107-108`).
- Icons: one set, one size per role, never emoji (`craft.rs:101`); only Lucide exists (`icons.rs:1-5`).
- Accessibility: contrast thresholds only (`review.rs:22-33`); glass accessibility looks (`tokens.md:45-56`).
- Motion: none in the product (`NOTES.md:34`); the reviewer is told not to ask for motion (`critic.rs:41`).
- Component names that appear anywhere in guidance outside the glass skill: button, chip, badge, tab bar, footer, table, list, card, panel (in `craft.rs` and `review.rs` prose). A search of `craft.rs`/`agent.rs` text read for this note found no mention of input, field, select, checkbox, radio, switch, dialog, modal, menu, dropdown, tooltip, toast, drawer, command palette, tabs (as a control), accordion, skeleton, spinner, progress bar, avatar, breadcrumb, pagination or date picker.

### Inferences (the concrete missing items)

Component anatomy and sizes
- No size scale for controls (for example small/default/large heights such as 32/36/40, or icon-button squares), no rule that inputs, selects and buttons in one row share a height, no horizontal padding rule, no icon-to-label gap, no icon size per control size.
- No anatomy for inputs (label above, control, helper or error text, leading/trailing adornments, placeholder versus value colour), selects/comboboxes, checkboxes/radios/switches (box sizes, label gap), textareas, sliders outside glass.
- No anatomy for overlays: dialog (max width, padding, title/description/footer, button order and alignment, scrim opacity), popover and dropdown menu (item height, inset, separators, shortcut hints, checked items, destructive items, submenu indicators), tooltip (size, delay is moot but placement and arrow are not), toast (position, stacking, action, dismiss, variants as in Sonner), drawer/bottom sheet (handle, snap heights as in Vaul), command palette (input row, grouped results, selected row, empty row, footer hints as in cmdk).
- No anatomy for navigation and data: tabs, segmented controls, breadcrumbs, pagination, sidebar nav items with active state, tables (header style, row height, numeric alignment is covered, sort indicator, selection, sticky header), cards (header/content/footer padding), badges, avatars, accordions, steppers.
- The only ready-made component layers are glass ones; there is no neutral (non-glass) component reference a model can insert, and the starter has only one button.

State sets
- No list of states a control should be shown in (default, hover, pressed/active, focus-visible, disabled, invalid, loading, selected/checked, indeterminate, read-only, open). No convention for laying states out as a board (the glass skill's "another frame beside the screen" is the only precedent).
- No rule for how each state differs (for example hover as a one-step tint, disabled as lowered opacity with a stated floor, invalid as destructive border plus message, loading as spinner replacing the leading icon with width preserved).
- The document model has no variants, so nothing prompts a model to produce more than the resting state.

Token scales
- Colour: six roles only. No surface elevation steps, no hover/active/subtle steps, no semantic sets (destructive, success, warning, info with their foregrounds), no "on-colour" pairing rule (primary/primary-foreground), no ring/focus colour, no input border versus divider distinction, no 12-step-style scale logic (Radix) or neutral-family choice.
- Radius: no scale (for example 4/6/8/12/16/full) and no nesting formula outside glass (inner = outer - gap exists only in `SKILL.md:103-105`).
- Shadow: no elevation levels (xs..xl), no layered-shadow recipe, no guidance that dark themes rely on borders and lighter surfaces instead of shadows.
- Type: no named roles with sizes, line heights and weights (for example 12/16, 14/20, 16/24, headings), no label/body/caption distinction for UI text, no rule for UI text weight (500 for labels) or for line height of single-line controls.
- Space: the 4-step scale exists; there is no mapping from scale to use (control padding, form field gap, section gap, card padding, page gutters) and no density modes.
- Sizes: no icon scale (16/20/24) tied to text size, no border width convention (1 px hairlines), no max content widths for dialogs/forms beyond the 45-75 character line rule.

Focus, accessibility, dark mode
- Focus rings are never mentioned: no ring width, offset, colour or which control shows it in a mock-up; and the counted review may flag ring recipes as glow or block shadow (`review.rs:263-267`).
- Touch/pointer target minimums are prose only (44 on phones) and not counted; no desktop minimum (24).
- Disabled contrast, placeholder contrast and non-text contrast (3:1 for control borders and icons) are not stated; only text contrast is measured.
- Dark mode has a mechanism but no craft rules: translucent white borders, raised surfaces lighter than the ground, reduced shadow, desaturated accents, avoiding pure black/white, checking both themes. The review does not check the second theme unless a frame in that theme is on the canvas.

Numbers and data
- No number formatting rules: thousands separators, decimal precision consistency, currency and unit placement, percent, compact notation (1.2k), negative values, deltas with sign and colour, dates and relative time, durations, alignment on the decimal point. Tabular numerals cannot be switched on (no OpenType features), and the workaround is described only as font-family cautions.
- No guidance for charts beyond "little charts and progress rings that show nothing real" being a tell (`craft.rs:124-125`).

Empty, loading, error and edge states
- Nothing on empty states (icon, one line, one action), loading (skeleton shapes matching content, spinners, progress), error states (inline field errors, banners, retry), success confirmation, offline, permission, zero-results in search, or overflow/truncation of long content (the model cannot truncate; it must draw it).
- "A working screen shows real data with one state worth seeing" (`craft.rs:98-100`) is the only nudge, and it concerns data rows, not screen states.

Interaction and motion (expressed statically)
- No convention for showing an open menu/popover/dialog over a screen outside glass (scrim value, placement, offset from trigger), for showing a transition as a sequence of frames, or for annotating behaviour with `note` layers or the `context` field.

Showcase and effect libraries (Magic UI, Aceternity UI, 21st.dev)
- The current guidance treats most of their signature moves as tells to remove unless the user asks: gradient text, glows, glowing borders, blurred blobs, decorative glass, bento-style equal cards (`craft.rs:120-128`; `review.rs:989-1006, 1060-1064`). There is no positive guidance for when such effects are requested and how to do them well in static form (shader effects exist, `MECHANICS.md:64`, but are not in `GUIDE`).

Process and checking
- No counted checks for: consistent control heights in a row, icon size consistency, distinct shadow count, missing focus/disabled states, target size, raw (non-variable) radii or spacing, number format consistency, or both themes.
- External MCP clients get no second-model review; only the in-app chat does (`chat.rs:426-449`).
- Icon names are not validated or listed; a wrong Lucide name draws a placeholder outline with no advice (`MECHANICS.md:200`).
- The first-message "dealt directions" force a ground and a lead face on every new conversation (`craft.rs:230`), which may work against a request that is really for conventional, library-style components.
- The starter document contradicts the guide (section 6), so "match what is there" can lead a model to defaults the review then flags.

### Gaps
- These gaps are derived from reading the texts listed above; I did not run the app or observe a model's behaviour, so which of them actually hurt output quality most is not established here.
- Whether any of this knowledge is present in `design/pencraft.pen` as reusable components is unknown (file not opened by instruction).
- A set of nine general design-review cases exists at `checks/design-review/` (PNG renders plus `pens/`: 1-coffee-landing, 2-habit-tracker, 3-repair-cafe-poster, 4-product-page, 5-banking-home, 6-bakery-dashboard, 7-settings-screen, 8-pricing-page, 9-weather-screen; also `banking-runs/run-a..c.png`), used as a pixel baseline in `docs/liquid-glass/NOTES.md:23-24`. I listed the folder but did not open the pictures or the `.pen` files, and found no prompts or scoring notes beside them, so how they were produced and judged is unknown. Three of the nine are working screens (banking home, bakery dashboard, settings); none is a component sheet.
- No automated eval or scoring harness for general (non-glass) design quality was found in the source.
