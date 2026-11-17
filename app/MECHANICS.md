# Pencraft mechanics sheet

Every rule and number Pencraft uses.

## File format (`.pen`)

| Rule | Value | Note |
|---|---|---|
| Container | JSON, two-space indent, no trailing newline |  |
| Schema version for new files | `2.22` |  |
| Node kinds | frame, group, rectangle, ellipse, polygon, path, text, note, icon, script, browser, ref (+ older `icon_font`, `line`) |  |
| Unknown keys and kinds | kept and written back | Pencraft's own rule (round-trip) |
| Variables | `$name`; a value list picks the last entry whose `theme` matches |  |
| Instances (`ref`) | instance properties override the component; `descendants` by id path: with `type` = replace, else merge |  |

`cargo run -p pen-doc --example pen_check -- FILES` checks that documents read, write back stably, resolve their
components and lay out with finite numbers.

## Node defaults

`enabled true, x 0, y 0, width 0, height 0, rotation 0, opacity 1, clip false, layoutMode None, justify Start,
align Start, sizing Fixed`; **frames**: `layoutMode Horizontal, width and height FitContent`; text: `fontSize 14,
fontFamily "Inter", fontWeight normal, letterSpacing 0, lineHeight 0 (font's own), textAlign left,
textAlignVertical top, textGrowth auto`; **browser** `cornerRadius 16`; **missing component** 120 × 90.
Document loading: a frame's `layout` defaults to horizontal, `gap` 0, justify/align start.

## Layout

Per axis, widths first then heights: fit content (x), fill container (x), fit content (y), fill container (y), then positions.

| Step | Rule |
|---|---|
| Fit content | A flex frame sized `fit_content` along an axis becomes the sum (main) or max (cross) of its children's footprints, plus gaps between the counted children, plus padding. Children that fill along that axis, hidden or absolute children don't count. With none counted it keeps its size (fallback, else 0). Frames without a layout never grow. |
| Fill container | Main axis: (inner size − non-fill footprints − gap × (children − 1)) ÷ fill children. Cross axis: the whole inner size. At least 1. |
| Positions | Free space = inner − footprints (not clamped). start; end: +free − gap·(n−1); center: half that; space_between: gap = free ÷ (n−1); space_around: gap = free ÷ n, start + gap ÷ 2. Cross: start / (inner − size) ÷ 2 / inner − size. |
| Footprint | a child's rotated, flipped bounding box; its corner lands on the slot |
| Local matrix | translate(x, y) · rotate · scale(flipX ? −1 : 1, flipY ? −1 : 1), around the node's origin |
| Groups | as big as their children; scaling a group in a layout scales its children (not in this version yet) |

## Text

| Rule | Value |
|---|---|
| Auto width | `ceil(maxIntrinsicWidth)` |
| Height | `ceil(paragraph height)`; fixed-width-height keeps its height |
| Line height | `round(lineHeight × fontSize)` when lineHeight ≠ 0, half leading, forced strut |
| Paragraph x | −letterSpacing ÷ 2 |
| Vertical align | only for fixed-width and fixed-width-height text |
| Resizing text | auto → fixed-width (horizontal) or fixed-width-height (vertical) |

## Drawing

| Rule | Value | Note |
|---|---|---|
| Stroke alignment default | **center** |  |
| Browser corner radius | 16 |  |
| Find layers | ⌘F, ⇧⌘F, the magnifier on the tool rail or the action "Find layers" open a small island beside the rail and clear the selection. It looks through every layer's name (as the layers list shows it), then its text, then its id, ignoring case, parents before their children; at most 200 characters. The first match is the current one; ↑ and ↓ step through and go round; a click on a line makes it current. All matches are outlined on the canvas, the current one more strongly, and it is brought into view. ↵ selects the current match (or the first) and closes; esc, the close button or a click elsewhere closes. The counter reads "2 of 7" or "7 matches"; with nothing typed "Searches names, text, and IDs"; without a match "No layers match “…”". Field row 44, lines 32, 10 lines before it scrolls, 6 above and below | **Estimated:** the island's width (300) and place, the outlines' look, and how a match is brought into view (the view moves to it, and zooms out only when it doesn't fit). **Left out:** layers inside component instances aren't searched |
| Corner radius fields | one field sets all four corners and reads "Mixed" when they differ; the button beside it ("Edit corners individually", lit while on) shows four fields under it: top left, top right, then bottom left, bottom right. Not for polygons. The file lists the corners clockwise from the top left; no value is below 0. Equal corners are written as one value, none at all for 0 (16 for a browser) | The four fields start open for a layer whose corners already differ. The corner marks in front of the fields have an **estimated** look |
| Drop shadow | offset, colour (black when unset) and blur; Gaussian sigma = blur ÷ 2; `spread` is not read. Cast by the layer's shape drawn solid (fill and stroke; the children when it has no fill), whatever the fill's opacity; when the shape doesn't hide what is under it, the shape is cut out of the shadow |  |
| Inner shadow | the fill's shape in the shadow's colour, minus that shape blurred (sigma = blur ÷ 2) and moved by the offset; drawn over the fill and under the stroke, or over the children when there is no fill |  |
| Layer blur, background blur | Gaussian sigma = radius ÷ 2; one of each per layer at most |  |
| Effect order | background blur, drop shadows, fill, inner shadows, stroke, children; a layer blur blurs all but the background blur | layer blur over the shadows is **Pencraft's own rule** |
| New effect | shadows: `#00000040`, offset 0 / 4, blur 4; blurs: radius 4; shader: no file. Listed newest first; the file writes one effect as an object, several as a list, `enabled` only when off, and leaves out an offset of 0 / 0 and a blur of 0 |  |
| Shader effect | a WebGL 1.0 fragment shader file, run last over the layer with its other effects; several run in list order, each on the one before. `pencil:object` is that picture, `pencil:background` what is behind the layer (mirrored past its edges); a picture file is read upright. `gl_FragCoord` and `@resolution` are in the node's own units, y up from its bottom edge; texture coordinates 0–1 span the node's box. A uniform's value is the layer's `uniforms` entry, else its `@default`, else 0 |  |
| How shaders are run | the file is rewritten to GLSL 450 (uniforms in one block, each sampler split into texture and sampler, `main` wrapped), checked, run once on the window's GPU over the layer's picture and read back; a shader that is missing or doesn't compile leaves the layer unshaded. A rotated layer is given the upright box around it; node units, not screen pixels, are an **assumption** | **Pencraft's own rule** (wgpu reads GLSL 450 only) |
| How effects are drawn | on the CPU: the source is painted into a bitmap at the zoom's power of two (at most 1 000 000 pixels, 2048 a side), blurred (three box blurs; exact below sigma 2), and shown as a picture, kept until what it was made from changes. A plain opaque unrotated box keeps egui's own soft shadow | **Pencraft's own rule** (egui can't blur) |
| Material | `material` on a frame, rectangle or whole ellipse: `"frosted"` (thickness `ultra_thin`, `thin`, `regular`, `thick`) or `"liquid_glass"` (variant `regular`, `clear`), with `appearance` (`auto`, `light`, `dark`), `tint`, `depth` (`control`, `bar`, `panel`; by the short side: up to 52, up to 110, larger) and `advanced` numbers. The material is the layer's background: after its shadows, before its fill; children and stroke are drawn on top. A text, icon, line, group or note can't have one. | **Pencraft's own rule** (the look follows Apple's published 2025 guidance, the numbers are tuned by eye) |
| How a material is drawn | what is painted before the layer (on the canvas colour) is drawn a blur's reach beyond it, turned into linear light and blurred (at half or a quarter of the size for wide blurs). Frosted: saturation, a little light, the tint and a 1.1-unit lit edge, on the CPU. Liquid glass: a built-in shader (`src/render/glass.frag`, through the shader effect's path) bends that picture inward along a squircle-profiled rim by Snell's law (index 1.5), shows the rim from a lightly blurred copy (a fifth of the blur), adds light from one direction and weaker from the opposite one, and dithers; without a GPU, and for any other shape, glass is drawn like frosted. `auto` is dark when what is behind is darker than 0.22 on average (linear light). Kept like other effect pictures; an idle canvas bakes nothing | **Pencraft's own rule** |
| Material numbers | each from the layer's `advanced`, else a variable named after the preset (`glass-regular-blur`, `frosted-thin-tint-opacity`), else the preset; `glass-light-angle` turns the light for the whole document (degrees clockwise from above, -45 by default). `blur` is a background blur's `radius`. The table is `Material::preset_number` in `src/material.rs` and in the skill's `references/tokens.md` | **Pencraft's own rule** |
| Glass group | a frame with `glassGroup` (a spacing, 0 to 200) draws the liquid glass of up to 8 direct children as one pane with the first one's material: their outlines are joined by a smooth minimum, so children closer than the spacing flow together. Unturned frames and children only | **Pencraft's own rule** |
| Sticky note | colour `#FEE484`, 250 × 219 min, padding 24, radius 24, text 15, white text below contrast 4.6 |  |

## Tools

| Rule | Value | Note |
|---|---|---|
| Rectangle / ellipse / polygon fill | `#CCCCCC` (polygon count 3) |  |
| New frame | fill `#FFFFFF`, **clip on**, no layout, fixed size, name `Frame N` |  |
| Polygon tool | drawn like a rectangle: fill `#CCCCCC`, 3 corners |  |
| Icon tool | a click places a 24 × 24 Lucide `heart`, centred, black or white by contrast with what is under it |  |
| Pen tool (P) | a click places a corner point, a press and pull a smooth point whose two handles mirror each other; ⇧ holds the new point (or the handle) to 45° lines from the last point; a click within 8 pt of the first point closes the path. Enter ends the path and goes back to Move, Escape or another tool ends it too; fewer than two points make nothing. ⌘Z takes the last point away. The layer: `path`, stroke `#000000` 1, no fill, its box the drawn outline's, in the frame under the first point. Dots 8, handle squares 9, lines 1 pt in the selection blue | The layer is made when the path ends and each outline is its own layer. Joining outlines and snapping to other points are left out; the points of a finished path are edited as the next row says |
| Path points | a double click on a path, or Enter with it selected, shows its points instead of its box; Enter, Escape (first letting selected points go) or a double click beside the path ends it. A click selects a point (⇧ adds or takes away), a click on a line its two ends; ⌘A all. Dragging moves the selected points with their handles, on whole pixels when snapping is on; arrows nudge them; Delete takes them out (an outline under two points goes, the layer with its last outline). The handles of selected points and their neighbours show; pulling one turns the other while the point is smooth, ⌥ bends it alone. A double click on a point makes it smooth (handles along the line between its neighbours, a third of the way) or sharp. ⌥ and a click on a line adds a point without changing the shape. X takes the cut: a click on a point or line opens a closed outline there or splits an open one, then the pointer is back (V too). The layer's box follows the outline unless the path has a view box of its own; a stretched path is edited in its own coordinates. Reach 8 pt for points and handles, 10 pt for lines | The smooth handle length (a third) is **estimated**. **Left out:** the pen adding to a path being edited (P leaves the editing), joining outlines, dragging a box around points, bending a line by dragging it with ⌘, copy and paste of points, snapping to other points and guides, editing rectangles, ellipses and polygons as paths |
| Scale tool (K) | factor 0.01 – 100. A selected layer moves from the anchor and everything in it scales: positions, fixed sizes, font size, letter spacing, gap, padding, stroke width, corner radius, shadows' and blurs' sizes. Layers placed by a layout keep their place; fill / fit sizes and variables are left alone. A handle scales away from the opposite side; the number field scales around the middle and restarts at 1 with a new selection |  |
| Tool menus | Move menu: Move (V), Scale (K). Shape menu: Rectangle (R), Ellipse (O), Polygon, Icon, Script, Import Image or SVG…, Import Figma; the tool in use is ticked |  |
| Import image or SVG | a picture becomes a rectangle its own size (at most 80 % of the view) filled with it, the file named relative to the document's folder when inside it; an SVG becomes one path layer in its view box. Both land in the middle of the view | placement and the SVG reading are **Pencraft's own rule** |
| Shapes need a drag | a click without dragging, or a 0 size, creates nothing |  |
| After drawing | tool → Move, new layer selected; a new frame takes in the layers it covers |  |
| Pixel snapping | pointer positions rounded while drawing (on by default) |  |
| Text tool | font size 16, empty text, black or white by contrast with the pixel under the click; a drag over 30 × 10 screen points sets its box; an empty text is deleted when editing ends |  |
| Sticky-note tool | 250 × 219 centred on the click, inside the frame under it, then editing |  |

## Selection and editing

| Rule | Value | Note |
|---|---|---|
| Polygon shape | corners on the ellipse that fills the box, the first at the top; `cornerRadius` rounds every corner, at most half the shortest side |  |
| Hitting a shape | an ellipse or polygon is hit on its shape, not on the empty parts of its box |  |
| Selection look | the box, and for an ellipse or polygon the shape's outline too; a square handle at each corner (edges resize without a handle); the size label under the box | handle and label sizes **estimated** |
| Polygon selection box | the box, its handles and its size label fit the polygon's corners, not the layer's own box (a triangle leaves a quarter of that empty under its base); resizing pulls that fitted box and the layer's box follows in proportion. | Pencraft's own rule |
| Turning | a press just outside a corner of the selection box (within 22 points) turns the layer around the middle of its box, several layers around the middle of the selection; ⇧ turns in steps of 15°; the label shows the angle while turning. "Rotate 90° right / left" are in the action search. A layer placed by a layout gets only the rotation | The reach and the arrow by the pointer are **Pencraft's own rule** |
| Rotation handle | a round dot on a short stem, 26 points off the middle of the selection box's top side (turned with the layer; above the selection's bounds for several layers); dragging it turns the selection like the corner zones do. | Pencraft's own rule |
| What a press grabs | decided by where the button went down, not where the pointer is once the drag is recognised | Pencraft's own rule |
| Handles on turned layers | handles sit on the turned box's corners and resize along the layer's own sides, the opposite side staying in place; mirrored layers have none | Pencraft's own rule |
| Several layers selected | one box is drawn around them all with its size under it, and none is outlined by itself; the rotation handle stands on that box | Pencraft's own rule |
| Corner-radius handles | a selected upright rectangle or frame at least 56 points on its shorter side shows a round handle inside each corner, 12 points in or at the radius; pulling one sets all four corners, up to half the shorter side | the numbers are **estimated** |
| Click picking | ⌘ = deepest. A top-level frame passes to the child under the pointer; its background picks it only with a layout or no children. Nested frames and groups are picked whole until selected, then the click goes inside. |  |
| Duplicate | copy in place right above the original; a single top-level frame goes to free space on its right, 40 past any neighbour |  |
| Frame selection (⌥⌘G) | `Frame N`, no fill, no clip, no layout, fixed size of the bounds; group at the bounds' top-left |  |
| Add flex layout (⇧A) | direction from overlap, else wider centre spread; gap = median gap (10 for one); padding from frame edges (wrapping: 0, or 10 for one); align by least distance; ≥ 2 full-cross children stretch (cross fixed); a lone text centred |  |
| Nudge / big nudge | 1 / 10 |  |
| Paste (⌘V) | never offset. Into each selected container on screen (or the container of a selected layer), centred, unless copied from it; else exactly on the originals when one is on screen; else centred in the view, inside a layout-free frame there that wholly holds it |  |
| Pasting plain text | a text layer: font size 12, "Inter", fill `#000000`, then placed like any paste |  |
| Undo steps | the newest 100 are kept (whole documents are stored) | Pencraft's own limit |

## Components

| Rule | Value | Note |
|---|---|---|
| Override keys | a `descendants` key is the id of a node anywhere in the component, or its name when only one node has it; `nestedInstanceId/innerId` reaches into a nested instance. With `type` the node is replaced; with `children` its children are; an instance's own `children` replace the component's |  |
| Editing an instance | an instance and the layers inside it can be selected and edited; what differs from the component is written to the instance (its own keys, or `descendants`), never to the component |  |
| Stepping in | an instance is picked whole until it is selected, then a click goes to the layer inside it; a double-click steps in | Pencraft's own rule, following the frame and group rule |
| Inside an instance | layers can't be moved, reordered, resized by dragging or made components; delete hides the layer (`enabled: false`) |  |
| Copying a component | a duplicate, ⌥-drag copy or paste of a component that is still in the document is an instance of it |  |
| Create / detach | Create Component only on plain layers; Detach Component clears `reusable`; Detach Instance turns the instance into plain layers |  |
| Go to component | selects the component and brings it into view with the fit margin, at most 100 % |  |
| Slots | an empty frame inside a component can be made a slot (`slot: []`) and unmade; an empty slot is hatched | Hatch spacing 8 and colour are **estimated** |
| Filling a slot | with a frame inside an instance selected, the Slot section adds an instance of a component (the slot's recommended ones, else every component, at most 12, never the instance's own), and paste adds the copies; both write a `children` override on that frame. New layers are centred in a slot without a layout. Deleting one takes it out of the override; Reset drops the override | the section and its limits are **Pencraft's own rule** |
| Colours | component: magenta box and name, always shown; instance and its insides: violet when selected | exact colours **estimated** |

## Canvas

| Value | Number | Note |
|---|---|---|
| Zoom range | 0.02 – 256 |  |
| Zoom in / out (⌘+ / ⌘−) | next power of two: `2^(⌊log₂z⌋+1)`, `2^(⌈log₂z⌉−1)` |  |
| Zoom to fit / selection | 40 pt margin, no zoom cap |  |
| Fit on open | 40 pt margin, at most 100 % |  |
| Fit area | the 40 pt margin is kept from the part of the canvas the floating islands leave free: the tool rail (left), the message bar while the sidebar is folded (bottom), the properties island while something is selected (right). | Pencraft's own rule |
| Component marks | a component's magenta box and name show only while it is selected or the pointer is over it |  |
| Ideas row | shown only for a document with no layers yet |  |
| Tab name | a saved file's tab shows the whole file name with `.pen` |  |
| Wheel with ⌘ / ctrl | zoom += clamp(Δy, ±15 / ±30) × −0.012 × zoom |  |
| Wheel pan | Δ × 1.5, at most 120 pt a step |  |
| Wheel "line" units | 40 pt | **estimated** |

## Action search

| Rule | Value |
|---|---|
| Opening | ⌘P, or the rail's "…" |
| List | actions under their headings in this order (File, Edit, Arrange, Layout, Text, Components, Selection, View, Tools, Application), each with icon, name and shortcut key caps; found by name, heading or keywords; ↑ ↓ move, ↵ runs, esc closes; at most 400 high |
| Left out | actions this version can't do: rename file, snapshots, copy-as and export, presentation, comments, eyedropper, pixel grid, rotate 90°, paste to replace, make/remove slot |

## Export

| Rule | Value | Note |
|---|---|---|
| Formats | PNG, JPEG, WEBP, PDF, SVG |  |
| Scale | 1x, 2x, 3x for PNG, JPEG and WEBP |  |
| JPEG quality | High 95, Medium 75, Low 50 |  |
| WEBP | always without loss | the image library writes lossless WebP only |
| PDF | one page as large as the exported layers in points, holding them as a picture drawn at 2 pixels per unit (JPEG quality 92), not as vectors. | the picture page and the single page are **Pencraft's own rule** |
| Pictures | the layer with everything in it, drawn like the canvas (effects and shaders included), as large as its box plus whatever reaches past it; PNG and WEBP keep transparency, JPEG and PDF put it on white; at most 64 000 000 pixels | **Pencraft's own rule** |
| SVG | shapes, solid, linear and radial fills, picture fills (linked by file path), strokes (centred), corner radii, opacity, clipping, text lines, icons, drop shadows and layer blur. A material is written as its tint (a little more solid) and a lit edge. Left out: inner shadows, background blur, shaders, angular gradients (first colour), stroke alignment, the wrapping of wrapped text | **Pencraft's own rule** |
| What is exported | the selected layers together in one file, each where it is on the canvas, in a picture as large as the box around them all, without what lies unselected between them; with nothing selected, every top-level layer. The file is chosen in a save dialog, named after the layer or the document. | Pencraft's own rule |
| Entry points | the Export section at the foot of the properties island, File › Export Selection to…, and the action search |  |

## Menu bar (macOS)

| Menu | Items | Note |
|---|---|---|
| File | New File ⇧⌘N, Open… ⌘O, Open Recent ▸, Home ⌘N — Import Image/SVG/Figma…, Export Selection to… ▸, Export Snapshot… — Guide: How to Import from Figma… — Install command-line tool… — Save ⌘S, Save As… ⇧⌘S — Close Tab ⌘W |  |
| Edit | Undo ⌘Z, Redo ⇧⌘Z — Cut ⌘X, Copy ⌘C, Paste ⌘V, Delete — Select All ⌘A (macOS adds its own writing items) |  |
| View | Show/Hide UI ⌘\, Chat Text Size ▸ — Toggle Full Screen ⌃⌘F, Search Actions… ⌘P |  |
| Window | Minimize ⌘M, Zoom — Toggle Light/Dark Mode — Show Previous Tab ⇧⌘[, Show Next Tab ⇧⌘], Move Tab Left ⌃⇧Page Up, Move Tab Right ⌃⇧Page Down — Bring All to Front — the open tabs | the Page Up / Page Down keys are **estimated** |
| Help | Documentation, Prompt Gallery & Tips, Website — Cursor Extension, VSCode Extension — Join Our Discord |  |
| App menu | About is left out; Settings… ⌘,, Services, Hide, Quit (asks about unsaved work) | craft-menu's Mac layout |
| Greyed | what this version can't do stays in its place, greyed: the snapshot, the Figma guide, the CLI install, chat text size, every Help item | **Pencraft's own rule** |

## Window

| Value | Number |
|---|---|
| Default size / minimum | 1352 × 878 / 560 × 400 |
| Tab strip height | 50 |
| Colours | defined in `src/theme.rs` (selection `#3b82f6`) |
| Shortcuts | as shown beside each action in the action search and in the menu bar |

## Pictures and icons

| Rule | Value | Note |
|---|---|---|
| Image fill `url` | a file path relative to the `.pen` file's folder; absolute paths and `file://` also read |  |
| Image fill `mode` | `cover` (default; older files say `fill`), `contain`, `stretch`; the picture is centred |  |
| Image fill `transform` | crops: maps picture 0–1 coordinates into the crop's 0–1 box; the mode then fits the crop |  |
| Unreadable picture | grey `#d9d9d9` placeholder; nothing is fetched from the network | Pencraft's own rule |
| Picture formats | PNG, JPEG, WebP, GIF (first frame); sides over 4096 px are scaled down | Pencraft's own rule (`image` crate) |
| Icons | `library` + `icon`, or older `iconFontFamily` + `iconFontName`; scaled to the node's box |  |
| Bundled icon set | Lucide (ISC licence) from the `lucide-static-svg` crate, drawn as 2-unit round lines in a 24 box; `feather` names are looked up in Lucide, and 14 earlier names (`home`, `trash-2`, `edit`…) lead to their replacements | Pencraft's own rule |
| Unknown icon or library | small rounded outline in the icon's colour | Pencraft's own rule |
| Made pictures | an image service makes a picture from a description (aspect 1:1, 16:9, 9:16, 4:3, 3:4, 3:2 or 2:3; "normal" about 1024 pixels on the long side, "large" about 2048). It is written to `images/` beside the `.pen` file, named from the first words of the description (40 letters at most) and four hex digits, never over another file; with no file yet, to `~/.pencraft/images`, from where every save copies the pictures the document shows into `images/` beside it (the same name; a new one when another picture has it) and names them from there, in the undo steps too. Saved under a new name in another folder, the pictures named from the document's folder (no `..` in the path) are copied to the same place under the new one, never over a file there. `images/credits.json` gets a line for it (service, model, description); one that can't be read stops the save and is never replaced | Pencraft's own rule |
| Time for a picture | a service may take 95 s to answer, a job 100 s (under the 120 s the local server waits for a tool); after that nothing is kept, though the service may have charged. A picture already being written is waited for. A cancelled dialog or a stopped chat keeps nothing | Pencraft's own rule |
| Pictures at once | 3 for assistants; a fourth call is answered at once with a sentence | Pencraft's own rule |
| Answer size | a service's answer is read up to 64 MB (a large picture as base64 passes the usual 10 MB) | Pencraft's own rule |
| The copy a model sees | a PNG no larger than 1024 pixels on a side; the file itself is kept as made | Pencraft's own rule |
| Picture for a layer | given as the layer's image fill (`cover`), one undo step; while the user drags on the canvas it waits up to 10 s, then the layer is left alone and the model is told to place the picture itself. Switching off AI tools or "Let the assistant generate and edit pictures" meanwhile also leaves the layer alone | Pencraft's own rule |
| Image services | Google Gemini (`GEMINI_API_KEY`) and OpenAI Images (`OPENAI_API_KEY`), or a saved service of either kind with its own address and model; one saved without a key is not used and is named as such when asked for. Only OpenAI Images makes a transparent background; only Google Gemini changes pictures (`edits: true`). `get_state` lists them under `imageServices` (empty, with a note, when "Let the assistant generate and edit pictures" is off). The Generate Image dialog (File menu, action search) and the field on an image fill work with that switch off | Pencraft's own rule |
| Changing a picture | by an instruction, through `edit_image`, the Generate Image dialog with a picture marked "Change this", or the field under an image fill's row. The picture to change: a layer's topmost image paint (`layer`), layers drawn together as they look (`layers`), or a picture file by its full path (`file`, the assistant only). References: layer ids (their picture, else how they look) or full file paths. With references alone the result is a new picture guided by them (shape 1:1 unless asked); with a picture to change it keeps that picture's shape unless asked. The result is never transparent | Pencraft's own rule |
| Pictures in one request | one to change and 4 references at most (`sources::MOST_REFERENCES`); the dialog's row holds 5 and turns a fifth reference away; with more references than that in the row (five layers selected, none marked) its button is off and the line under the row says why instead of what is sent. Network-share paths are refused before anything is read | Pencraft's own rule |
| Getting pictures ready | each scaled to at most 2048 pixels on its longer side (`sources::MOST_SIDE`); sent as JPEG (quality 90) laid on white, or as PNG when more than 1 % of its pixels are less than opaque (`CLEAR_PERCENT`); the picture to change goes first. All of one request together at most 15 MB once ready (`MOST_SENT`), else nothing is sent | Pencraft's own rule |
| Reading a picture file | one way for sending and for the dialog's small pictures (`sources::open`): at most 100 MB on disk (`MOST_FILE`), asked before reading; PNG, JPEG, WebP or GIF by its first bytes, never by its name; decoded within 16384 pixels a side (`MOST_READ_SIDE`) and 256 MB of memory (`MOST_DECODED`). Nothing of a refused file is told | Pencraft's own rule |
| Layers sent as drawn | drawn by the canvas's renderer (as `screenshot` and export draw), as many pixels a unit as keep the longer side within 2048, and enlarged at most 2× (`MOST_ENLARGED`) so a small design's text stays readable; a model's call waits for the faces of their text as a screenshot does | Pencraft's own rule |
| Where a changed picture goes | `layer`: the url of that image paint is replaced, the fill's other paints and the paint's `mode` kept, one undo step, only while the layer's topmost image paint still has the url that was sent (else a person's result becomes a new layer and is said, and a model is told and nothing changes). `layers`: a new rectangle "Edited picture" with the picture as its fill (`cover`), as large as the box drawn and at its height, 40 right of the top-level layers holding them (`BESIDE_GAP`), stepped on past every top-level layer it would lie on, 40 beyond each; selected when a person asked. `file`, or references alone for a model: nothing in the document changes. In the dialog "Replace the picture" off puts a layer's result beside it the same way | Pencraft's own rule |
| The dialog's row | a lone selected layer goes in only with a picture as its fill (marked "Change this") or with layers inside it; any other lone layer leaves the row empty and the dialog offers "Use as the fill of the selection", which is never offered for a lone layer in the row. Several layers all go in, as references. Small pictures 64 points a side, made at 128 pixels (`PIECE_PIXELS`); a layer's drawn at most 2× its size, a file's read on a thread of its own | Pencraft's own rule |
| The field on an image fill | under the row of the selected layer's topmost image paint; Return or the arrow sends the picture and the words to the default service when it can edit, else the first that can, normal size, the picture's own shape; the result replaces the picture. The words left on one layer come back when it is selected again (the last layer left with words), and are dropped once its picture is changed. While it works the field shows progress and Cancel, and other image fields say which layer the picture is for | Pencraft's own rule |
| A person's picture | the dialog's or the field's, one at a time, not counted among the 3 for assistants. It waits, with no time limit, while the user drags on the canvas or types text on it, looked at again every second; then it is placed, and Cancel no longer throws it away. The dialog's brings its document's tab to the front; the field's leaves the tab in front and, when its layer isn't the one selected there, says in the message line what became of it | Pencraft's own rule |

## AI models (tools)

| Mechanic | Value | Note |
|---|---|---|
| How a model reaches the app | Model Context Protocol: `pencraft mcp` passes JSON-RPC messages (one a line) between the client and the running app | Pencraft's own rule |
| The app's server | HTTP on `127.0.0.1`, a free port chosen at start; `POST /mcp` with one JSON-RPC message | Pencraft's own rule (`src/link.rs`) |
| Who is served | callers showing the secret of this run (`Authorization: Bearer …`, 256 bits, in `~/.pencraft/agent.json`, readable by the user only); a request with an `Origin` header (a web page) is refused | Pencraft's own rule |
| App not running | `pencraft mcp` starts it with `--background`: the window opens behind what the user is doing and doesn't take the keyboard; waits up to 20 s | Pencraft's own rule |
| One app | a `--background` launch ends at once when a Pencraft already answers; a running app rewrites `agent.json` within about 2 s when the file no longer leads to a running Pencraft | Pencraft's own rule |
| Picture-only runs | `--screenshot` windows open under all other windows, are moved off screen (macOS keeps a 40-point strip) and show no Dock icon | Pencraft's own rule |
| In the background | tool calls are answered while the window is behind others, minimised or hidden (they run in the app's logic step, which needs no drawing) | Pencraft's own rule; measured about 100 ms a call while hidden |
| Tools | `get_state`, `get_guide`, `list_skills`, `read_skill`, `read_layers`, `edit_layers`, `screenshot`, `check_glass`, `select_layers`, `open_document`, `save_document`, `export_layers`, `generate_image`, `edit_image` | Pencraft's own rule |
| `edit_layers` | operations `insert`, `update`, `delete`, `move`, `copy`, `replace`, `set_variables` (variables and theme axes) in order; layers brought in an instance's overrides get ids like any other; `$0`, `$1`… name the id an earlier operation made; all or none; one undo step | Pencraft's own rule |
| Advice after a change | `edit_layers` answers with `advice` when a changed layer has the fill of the nearest filled frame behind it (channels within 3 of 255; fills under half opacity, gradients, pictures, hidden layers, icons, paths and lines don't count), or text has its background's colour; grouped by the layer behind, 5 names each, 8 pieces at most | Pencraft's own rule (a board must not have the colour of the parts on it) |
| Review with a picture | `screenshot` answers with the picture and a `review` of the layers shown, counted not judged, ten notes at most, mistakes first: a font family that isn't installed (and what is drawn instead); text under 4.5:1 against the nearest filled frame behind it (3:1 from 24 px, 6:1 under 13 px); a layer sticking more than 1.5 out of the room its frame's layout gives it (the frame less its padding), a text lying more than 1.5 over its neighbour in a layout, a list of three or more rows built alike in which one part lines up on neither edge nor its middle from row to row, a layer reaching more than 1.5 over the edge of a filled top-level frame that doesn't clip, or content whose last text, icon, rule or bottom-lined row ends less than 16 above a filled frame that spans nine tenths of the width below it, unless it runs on behind that frame (a list ending right on a tab bar), or a vertical list of three or more rows in which every row, the last too, has a line along its bottom; these mistakes (with the font, the contrast and text under 11 px) also come as `advice` with every `edit_layers`; text under 11 px; a gradient with a hue of 235° to 295° (violet, indigo), more than two gradients, gradient text; more than three accent hues (saturation from 0.35, twelve steps of 30°) and more than eight colours written as hex, in designs of six texts or more; a row of three or four boxed children of the same build with an icon and two texts; cards three deep; two or more frames with outline and shadow; a stroke of 3 or more on one side only; three or more background blurs; more than eight gaps and paddings or two odd ones; more than four corner radii; emoji, stand-in copy and sales words; and for six texts or more: one default family (Inter, Arial, Roboto, Helvetica, Segoe UI, Open Sans), more than two families, more than seven sizes, sizes within 1.6 of each other in one weight and colour, more than three quarters centred, wrapped lines over about 80 characters, two or more spaced-capital labels over a line twice their size, a text starting 1.5 to 6 beside a left edge three or more texts of its size share, or ending 1.5 to 16 off a right edge three or more of its size share. Twelve layers or more, or any note, also bring the four questions to judge the picture by | Pencraft's own rule. Contrast: WCAG 2.1 SC 1.4.3. The marks: Anthropic, "Improving frontend design through skills" and "Harness design for long-running application development"; the Impeccable anti-pattern catalogue; Vercel web interface guidelines |
| Design guide | every engine is told "Designing well" with the brief, and `get_guide` gives it to outside clients: decide a direction for the subject, set colour, type and spacing as variables first, build with a hierarchy, leave out the marks of a generated design, judge the screenshot on wholeness, originality, craft and function (the first two weighing most) and improve it in two or three rounds. With it the installed font families and their weights, 160 at most | Anthropic's published method (see above) |
| A screen on a device | a mock of a phone or tablet app is shown inside the device: the guide ("A screen on a device") sends the model to the `devices` skill, whose iPhone template (the iPhone 16) is inserted as it is: a group of five side buttons (Camera Control among them) and a body 417 × 876 (corners 67, bezel `#0c0c0d`, rim 2 in `#48484a`) around a clipped screen 393 × 852 (corners 55) holding an "App" frame, the status bar (54 high; "9:41" semibold 17 centred 66 from the left; signal 17 × 11, Wi-Fi 16 × 11.3 and battery 27.5 × 12 as solid glyphs, 6 apart, centred about 326 from the left; all level with the island's middle), the Dynamic Island (126 × 37, 11 from the top) and the home indicator (134 × 5, 8 from the bottom). Every other iPhone since the XR (2018) up to the iPhone 18 Pro Max (2026) is the template with the numbers of its row in the skill's table: screen size, corners, island or notch (and its width), status bar height, Action button or ring/silent switch, Camera Control or not; the iPhone SE has its own bezels and home button. A chat message with the words iPhone, iPad, iOS, Android, smartphone, phone, mobile, tablet or App Store (where a word starts) brings a note that says so. While the chat's request has one of them, a `screenshot` review names first every frame 340 to 480 wide and 1.7 to 2.4 times as high, within three levels of the top-level layers shown, that has no such frame inside it at 4 or more from every edge; the reviewer counts a screen without its device as a missing piece | Pencraft's own rule; the status bar measured from the iPhone's own; screen sizes from Apple's and Android's device specifications |
| Text faces | text is drawn in the installed family its `fontFamily` names, in the face nearest its `fontWeight` (a variable font is set to the weight, within its range), italic when the family has one, else slanted. The font folders of the system and the user are read once (family name, weight, slant, weight range from the `name`, `OS/2` and `fvar` tables); a face's file is read when text first asks for it, so the first frame may show the stand-in. A family that isn't installed is drawn in the first installed of: Inter, Segoe UI, Helvetica Neue, Helvetica, Arial, Liberation Sans, DejaVu Sans (serif names: Georgia, Times New Roman, Times, Liberation Serif, DejaVu Serif; mono names: Cascadia Code, Consolas, SF Mono, Menlo, Liberation Mono, DejaVu Sans Mono); with none of them, Arial regular, bold from 600, italic as before | Pencraft's own rule; no fonts are downloaded |
| Skills | folders in the Agent Skills layout under `app/skills/` (`SKILL.md` with `name` and `description`, reference files beside it), built into the program. A model reads them with `list_skills` and `read_skill`; the guide lists them; `pencraft skills install` writes them to `~/.agents/skills` and `~/.claude/skills` (or a project's, or a named folder) | **Pencraft's own rule**; locations from each client's documentation, 14 November 2026 |
| `check_glass` | draws the layers once more without what is written on a material and answers with each material's appearance (light or dark) and the lightness behind it, and with problems: a text or icon under 4.5:1 against the lightest or darkest 3 % of the pixels under it; liquid glass on a material, or overlapping other glass outside one glass group; a nested corner that isn't the outer radius less the gap (capsules pass); regular and clear glass in one top-level layer; glass over a flat backdrop; glass on a shape that is drawn frosted | **Pencraft's own rule** |
| Stroke written as one object | `{"thickness", "fill", "align"}` becomes `strokeWidth`, `stroke`, `strokeAlignment` (inside → inner, outside → outer) | Pencraft's own rule (models often write the older form) |
| Screenshot for a model | PNG at the asked scale (0.1–3, default 1), drawn smaller when its longer side would pass 2000 px | Pencraft's own limit |
| While the user drags | `edit_layers` and `select_layers` wait up to 10 s for the drag to end, then answer "try again" | Pencraft's own rule |
| Flash on a touched layer | 1.2 s in `rgb(96, 125, 255)`: a 2-point outline that starts 12 points outside and closes in, a 10 % wash, and a band of light sweeping down; all fading | times estimated |
| Model's cursor | an arrow with the client's name on a pill, travelling 0.45 s to the middle of what the model touched, read or pictured; stays 8 s after the last call, fades in 0.5 s | times estimated |
| Settings window | at most 1100 × 695; in a smaller window it keeps 16 free at the left and right and 60 at the top and bottom; corner radius 24; the list of pages is 232 wide | the original's settings dialog: `w-[calc(100%-2rem)] max-w-[1100px]`, `h-[min(695px,calc(100vh-120px))]`, `rounded-3xl`, `sm:w-[232px]` |
| Cursor settings | size Small 1×, Medium 1.35×, Large 1.7×; label "Agent" (or the user's own word for it), the model's name, or none | factors estimated |
| Switch | Settings → Agents → "Let AI models work on documents", on by default; off answers every call with an error | Pencraft's own rule |

## Chat with an assistant

| Mechanic | Value | Note |
|---|---|---|
| Who answers | the assistant picked in the message box's menu: "Claude Code · model" when the `claude` program is installed, "Anthropic API · model" when `ANTHROPIC_API_KEY` is set (`ANTHROPIC_BASE_URL` moves it), and every custom provider with the "Anthropic Messages" API and a key; the first there is by default | the API route is Pencraft's own rule |
| Models offered | Opus 5.5 (first), Fable 5.2, Sonnet 5.5, Haiku 5.5; a custom provider's own list | Anthropic's current models |
| Claude Code route | one `claude -p` run per message with streamed JSON output, none of its own tools (`--tools ""`), only Pencraft's tool server (`--strict-mcp-config`, allowed without asking), the model picked, the brief and the design guide appended to its system prompt; later messages `--resume` the session; runs in a folder of its own in the temporary folder | Pencraft's own rule; flags of Claude Code 2.1 |
| API route | `POST <base>/v1/messages`, API version `2026-10-21`, streamed, 64,000 output tokens at most, the brief, the design guide and the document guide as system prompt, the nine tools; tool calls are run and all their results sent back in one message; at most 80 requests a message; "too many requests" and server errors are retried twice | Anthropic Messages API |
| Only to Anthropic's own API | caching of the request's unchanging start, effort `medium`, tool inputs streamed as written, and for Opus 5.5, Fable 5.2 and Sonnet 5.5 a hand-over to another model when a request is declined (`fallbacks: "default"`) | Anthropic API guidance; other providers get the plain request |
| What the model is told with each message | the document in front (name and file) and the selected layers (name and id) | Pencraft's own rule |
| Layers pasted into the message box | layers copied on the canvas and pasted into the text become one chip at the cursor, followed by a space: the kind's icon (12) and the name (18 characters at most) for one layer, a stack icon and "3 layers" for several with the names under the pointer; tinted ground, 1-point edge, corner 4. A chip is deleted whole. Only layers of the document in front count; others give a notice | the chip's text size is the message's, so the cursor stays true |
| What the assistant reads for a chip | each layer's id and, in brackets and quotes, its name: `card2 ("Card")`, several joined by commas; the panel shows the name |  |
| Attaching files | + in the message box asks for files; files dropped on the window are attached too (a dropped `.pen` is opened instead). Pictures (PNG, JPG, GIF, WebP, up to 3.7 MB) and text files (any file that reads as text, up to 256 KB), 10 to a message; anything else is turned away with a sentence. The files show as one chip with a paperclip and their number; a click lists them, a click on a name takes it off. Files alone can be sent. A text file travels inside the message in a block with its name, a picture as a picture: an image block for the API, and for Claude Code one JSON line on its input (`--input-format stream-json`). The panel shows "Attached: …" under the message | The limits, the chip and how the files travel are **Pencraft's own rules** (the picture limit keeps a picture under the API's 5 MB as base64). **Left out:** typing @ to pick a file, pictures shown in the chat panel |
| A reviewer for finished designs | when the assistant's answer is done and it wrote 30 layers or more since the last review (a new headline or one more row is no design to review), a picture of the top-level layers it changed, then its four largest parts of eight layers or more enlarged twice, (with what the app counted in them) and the user's request go to a reviewer: the same engine and model in a conversation of its own, with no tools, briefed as a strict art director. It scores design quality and originality (both weighing double), craft and function from 1 to 10 and answers PASS (craft 8 or more, the others 7 or more) or REVISE with four changes at most; these go back to the assistant as a message, it works them in and answers again. Two rounds at most; when the second still asked for changes, a third look only proofreads the execution (up to ten slips: touching or cut-off text, figures off their column, unequal heights, faint text, numbers that contradict each other) and leaves the idea alone; an answer that is neither word, or a reviewer that can't be reached, leaves the design as it is. The chat shows "A reviewer looked at the design". Settings → Agents → "Have designs reviewed" turns it off (on by default) | Pencraft's own rule, after Anthropic's "Harness design for long-running application development" (a separate, strict evaluator, since a model grades its own work too kindly) and ReLook, arXiv 2510.11498 (most of the gain within two rounds) |
| Replies | plain sentences without Markdown; the brief says so at its start and again as its last words, after the design guide | Pencraft's own rule: said once at the start, the rule was forgotten after long work |
| Changing assistant mid-chat | the one taking over is told the last 24 messages | Pencraft's own rule |
| Keys | a custom provider's key is read from the keychain only when a message is sent to it; never logged or shown | Pencraft's own rule |
| Panel | the user's messages at the right on a tinted bubble (86 % of the width at most), the assistant's words as plain text, one muted line per tool used (the same one in a row said once), problems in red, a spinner while it works; the newest line stays in view | Pencraft's own rule |
| Sending | Return sends, ⇧Return starts a new line; sending opens the chat panel; while the assistant works the send button is a stop button | Pencraft's own rule |
| From the command line | `--ask MESSAGE` sends a message at start, `--assistant ID` picks who answers, `--private` leaves the settings and remembered tabs alone; `RUST_LOG=info` writes the chat's lines to the terminal | Pencraft's own rule |

## Not in this version

Chat history, chips for skills, files and web elements, chat modes, Codex and OpenAI-style providers as the chat's assistant, Markdown in the assistant's words, the generating effect on placeholder frames, live browser nodes, scripts, shader fills, and in shader effects `@time` and `@mouse` (both stay 0), `@sdf` and `@mipmap`; effects' blend modes, pictures from web
addresses, icons from libraries other than Lucide and Feather (Material Symbols, Phosphor), icon `weight`, script layers, Figma import, snapshots, copy-as and code export, SVG groups, transforms and per-shape colours on import, comments, auto-update, opening files
from Finder, group scaling inside a layout, and pasting SVG or pictures. Pasting into
several selected containers puts a plain copy in each.
