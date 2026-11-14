# Liquid Glass: the task as the user wrote it

Task: add a Liquid Glass design skill to Pencraft
Context
Pencraft is my Rust design tool: AI agents create and edit designs on a canvas through tools, guided by skills that Pencraft ships. It is meant to work with every major agent harness (Claude Code, Codex, Cursor, Gemini CLI, OpenCode, Copilot, and any MCP client), not just one.
I want a new skill, `liquid-glass`, that lets any connected agent produce convincing Liquid Glass style interfaces in Pencraft: the translucent, refractive, light-reactive material Apple introduced in 2025 for the control and navigation layer of its platforms.
"Done" means: an agent on any supported harness, asked for "a liquid glass tab bar over this photo", loads the skill and produces a result in Pencraft that reads as real glass, stays legible, and uses reusable components and tokens instead of one-off values.
You have not seen this codebase before in this session. Do not assume how skills, the document format, or the renderer work. Find out first.
The Reference section at the end of this prompt is the research this task is built on: Apple's iOS 26 design rules, how Apple builds both of its glass materials, and the optics for the shader. Read it before planning.
Scope
In scope: a static, correct rendering of the material in every place Pencraft draws a design (canvas, thumbnails, image export), the document properties and agent tools to control it, the skill, and delivery to every harness.
Out of scope unless Pencraft already has prototyping or animation: gel-like flexing, touch glow, device-motion highlights, and animated morphing. The skill shows those states as separate static frames instead (Reference F).
Platforms: I work on Windows and also use a Mac, so the renderer changes must work on both. Do not rely on a feature of one GPU backend. In the final report, say which platforms and backends you actually ran and which are untested.
Working method

* Work on a new git branch, commit at the end of each phase with a clear message, and do not push.
* This task is larger than one context window. Keep a running notes file at `docs/liquid-glass/NOTES.md` (or wherever the repo keeps such docs): Phase 1 findings, the approved plan, decisions and their reasons, current status, and what is next. Update it as you go and re-read it whenever you resume.
* Real screenshots of iOS 26 are the ground truth for the look. Check whether `reference/liquid-glass/` exists and contains images. If it does, compare your renders against them at every visual checkpoint. If it does not, tell me in the Phase 1 report so I can add some, and continue with the Reference section's descriptions in the meantime. Do not commit Apple screenshots or assets into the repo's shipped files.

Phase 1: Explore, then report (no code changes yet)
Read the repo and answer these, citing file paths:

1. Skill system. How are existing skills defined, stored, registered, and served to agents (embedded in the binary, files on disk, an MCP tool such as `read_skill`, exported files per harness)? Take one existing skill as your reference and note its structure, length, and tone.
2. Harness delivery. For each supported harness, how does a skill reach the agent today? List any harness where delivery is missing or handled differently.
3. Rendering capabilities. Which of these can the document format and renderer express right now: background blur (blurring what is behind a node, not the node itself), layer blur, linear/radial/angular gradients on fills and strokes, inner shadows, multiple stacked shadows, blend modes, per-fill opacity, masks or clipping, noise or texture fills, displacement/refraction, variables and themes, reusable components with variants?
4. Render pipeline. What draws the canvas (wgpu, Skia, Vello, a CPU rasterizer, something else), which shading language and backends are in play, whether custom shaders or effect passes already exist, and whether a node can sample the already-rendered content behind it (a backdrop texture or layer snapshot).
5. Verification. How can you render a design to an image headlessly (CLI, test harness, screenshot tool) so you can look at your own output? Does that path use the same renderer as the canvas? Does CI have a GPU, and how do existing image tests stay deterministic?
6. Agent surface. Which tools and schema do agents use to create and style nodes, and where is that surface documented for them? How are effects and their parameters named there today?
7. Editor UI. Is there an inspector or property panel where a person edits effects, and how does a new property get a control there?
8. Export paths. Which exports exist (PNG, SVG, PDF, code generation for web, SwiftUI, or others) and how does each handle effects it cannot express?
9. Document mechanics. How are format versions and migrations handled, and how do undo/redo, copy/paste, and component overrides pick up a new property?

If any foundation this prompt assumes is missing, do not improvise around it silently. Put it in the plan as its own item and stop for my go-ahead:

* no skill system or no existing skill to model on: propose a format (the open Agent Skills `SKILL.md` layout is the default choice) and where it lives
* no headless render path: propose the smallest command that renders a document to PNG with the real renderer, because every later check depends on it
* no backdrop access in the pipeline: say what restructuring it needs and what it costs

Then give me a short plan with a gap list: which glass ingredients are available as-is, which can be approximated by layering existing primitives, and which need engine work. Recommend the smallest engine change set that renders the material correctly. Background blur is the one ingredient the look cannot do without. Edge refraction (lensing) is what separates Liquid Glass from plain frosted glass: if layering existing primitives cannot produce it, plan a dedicated glass shader as described in Phase 2, and say so in the plan.
Stop here and wait for my go-ahead if the plan involves changes to the renderer or the document format. If the skill can be built entirely from existing primitives, continue without waiting.
Phase 2: Engine work (only what the plan approved)

* Follow the existing architecture and conventions for adding an effect or property; do not introduce a parallel mechanism.
* Every new property must round-trip through save/load, appear in the schema or docs agents read, validate in the tool layer with error messages an agent can act on, work with undo/redo, copy/paste, and component overrides, get a control in the editor's inspector if one exists, and degrade sensibly in any export path that cannot express it.
* Keep older documents loading and rendering unchanged. Before touching the renderer, capture renders of a few existing documents and assert they are pixel-identical afterwards. Handle the format version the way the repo already does.
* Add tests in the style the repo already uses.

What agents and people set: a material, not a pile of parameters
Design the document property as a named material with a small set of knobs, because an agent that has to set a dozen raw shader values per node will get them wrong and burn tokens doing it:

* `material`: none, frosted (with a thickness: ultra-thin, thin, regular, thick), or liquid glass (with a variant: regular or clear)
* `appearance`: auto, light, or dark. Auto resolves from the backdrop under the node at render time (Reference F); the other two pin it.
* `tint`: optional color, for primary actions
* `depth` or size class: how thick the glass behaves (control, bar, panel), defaulting from the node's size
* an advanced block for the raw parameters, which presets fill in and experts may override

Presets resolve to the raw parameters in one place in the code, and their values are variables so a theme can retune them. Use the repo's naming conventions; the names above only show the shape.
Image export uses the real shader. Code export maps the material to the target's native equivalent where one exists (on Apple platforms the system glass effect with the same variant, tint, and grouping; on the web a backdrop blur with saturation, tint, and rim highlight) and says in a comment what was approximated.
The glass shader (write it if existing primitives cannot render the material correctly)
Write it in the shading language and effect architecture the renderer already uses, as a first-class effect or material a node can carry, not a special case bolted onto one node type. What it has to compute, per pixel of the glass shape:

* Shape. A signed distance field for the node's shape (rounded rectangle and capsule at minimum, with per-corner radii if the format has them), antialiased at the edge. Everything else derives from the distance to the edge.
* Surface normal. A height profile across a bevel band near the edge (flat in the middle, curving down at the rim, like a thick lens), with the normal taken from the SDF gradient and that profile. Bevel width and glass thickness are parameters.
* Refraction. Sample the backdrop at coordinates displaced along the normal, scaled by thickness and an index-of-refraction style strength, so content bends and magnifies at the rim and stays nearly undistorted in the center. Clamp or mirror at backdrop edges so nothing samples garbage.
* Dispersion. Optional small per-channel offset of the refraction for a chromatic fringe at the rim, off by default or very low.
* Frost. Blur of the refracted backdrop with adjustable radius, done efficiently (downsampled multi-pass or mip-based, not a huge single-pass kernel), plus a saturation and brightness adjustment.
* Tint. A color and opacity mixed over the backdrop result, driven by the node's fill so themes and variables keep working.
* Specular and rim light. A highlight from a light direction parameter against the normal, concentrated on the rim, plus a Fresnel-style brightening toward the edge and a faint opposite-side secondary highlight.
* Inner shadow and drop shadow. Either in the shader or composed from the existing shadow effects, whichever fits the architecture.
* Dither. A little noise before output so gradients do not band.

Around the shader:

* Backdrop capture. The glass node needs the content rendered beneath it as a texture. Add this to the pipeline in the least invasive way: render in z-order, snapshot only when a glass node is encountered, only the region it covers plus the blur and refraction margin. Glass that sits above other glass should see the lower glass in its backdrop.
* What counts as the backdrop. Everything lower in z-order within the same artboard, including the artboard's own fill. Where nothing is behind the node, the backdrop is the artboard or canvas background color, never uninitialized or transparent black. In an export with a transparent background, glass over empty space renders its tint, highlights, and shadow only.
* The node's own content. The material is the node's background. Its children (labels, icons) and its own stroke draw on top afterwards, sharp, and are never blurred or refracted by their own parent.
* Shapes. Rounded rectangles with per-corner radii, capsules, and ellipses get analytic distance fields. For any other shape (paths, boolean results) generate a distance field from the rasterized mask, or fall back to the frosted material and say so in validation. Respect corner smoothing if the format has it.
* Grouping. Give the format a way to mark a frame as a glass group with a spacing value. Its glass children render in one pass, their distance fields joined with a smooth minimum whose radius is the spacing, so close shapes merge and none samples another. Do this after single-node glass works; it is the second milestone, not part of the first.
* Color and alpha. Do the blur, refraction sampling, and tint mixing in linear light with premultiplied alpha, and convert once at the end, in the way the rest of the renderer handles color. Blurring in gamma space darkens edges and is the most common reason a frosted panel looks dirty.
* Parameters. Expose blur, refraction strength, thickness or bevel width, dispersion, tint, saturation, highlight intensity, and light direction as the advanced block under the material property, with sane defaults and clamped ranges, bindable to variables. Light direction defaults from one document-level setting so a screen stays consistent.
* Correctness under transform. It must look right at every zoom level, on high-DPI displays, when rotated, when clipped by a parent, and when partially off-canvas. Effect sizes are in document units, not screen pixels.
* Performance. Set a number and measure it: a screen with twelve glass nodes over a photo holds the display's refresh rate while panning and zooming on integrated graphics, and an idle canvas does no glass work at all. Re-render a glass node only when it or the backdrop region under it changes, blur on a downsampled copy, skip offscreen nodes. Report frame times before and after.
* Fallback. On any backend or export path that cannot run the shader (CPU rasterizer, SVG, generated code), fall back to blur plus tint plus gradient stroke.
* Tests. Golden-image tests for the shader over a fixed checkerboard and a photo-like backdrop, covering both variants, light and dark, and at least two zoom levels. A checkerboard makes refraction errors obvious: lines must bend smoothly at the rim with no seams, tearing, or hard steps. GPUs differ in the last bits, so follow the repo's existing approach to deterministic image tests; if there is none, compare with a small per-pixel tolerance and run on a software adapter in CI.

Reference C describes how Apple structures its own implementation and Reference E gives the optics model with formulas; build on both. Three structural lessons from Apple's version are worth copying: the shape is rendered once to a distance-field texture that the refraction pass and the highlight passes all read; highlights are separate passes lit from two opposite angles; and glass shapes that belong together are drawn in one shared pass so they can merge (smooth union of their distance fields) and so none of them has to sample another.
Judge the shader by looking at renders, not by the code compiling. Compare against reference imagery of the real material and iterate until the rim reads as a curved, thick piece of glass.
Phase 3: Write the skill
Match the format, frontmatter, and conventions of the existing skills exactly. Write it for an agent that is capable but has never thought about glass materials; explain why each rule exists so it can handle cases the skill does not list. Keep the main file focused and move long reference material (token tables, full component recipes) into linked reference files if the skill system supports that.
The trigger description should fire on: liquid glass, glass UI, glassmorphism, frosted or translucent panels, iOS 26 / macOS 26 style, visionOS style, and "make this look like glass".
Content the skill needs to cover, expressed in Pencraft's actual node properties and tool calls:
What the material is. Glass is a functional layer that floats above content: tab bars, toolbars, sidebars, sheets, popovers, floating controls. Content itself (lists, text, media, cards in a feed) is not glass. The effect only exists when there is something rich behind it, so the skill must tell the agent to make sure there is a backdrop worth refracting (imagery, color, scrolling content) before placing glass.
Choosing the material. One decision the agent makes for every surface: functional layer gets liquid glass, structure inside the content layer gets frosted, content itself gets neither. Then the variant: regular by default, clear only when all three conditions in Reference B3 hold, never both in one context.
How to apply it. Set the material property and its few knobs. The skill teaches the preset-level API first and mentions the advanced parameters only as an escape hatch. If the engine ended up without the shader, the skill instead gives the ordered layer recipe (backdrop blur with saturation lift, tint fill, gradient rim stroke, inner shadow, ambient drop shadow) with starting values and tuning ranges.
Deciding what a live system would adapt. Pencraft designs are static, so the skill tells the agent how to make the calls iOS makes at runtime (Reference B2 and F): light or dark appearance from the backdrop under small elements, with the foreground set to the opposite; large panes keep the theme's appearance and are more opaque; thickness, lensing, and shadow grow with element size; shadow is stronger over text and busy content.
Shape and geometry. The three shape types and their radius formulas (Reference A2). Capsules for bars and buttons, concentric radii for anything nested, margins from the screen edge instead of edge-to-edge bars, corner smoothing on.
Structure. How bars are built in this design language (Reference A4): no bar backgrounds, items grouped by function on shared glass, text buttons and symbol buttons in separate groups, the primary action separate and tinted, a floating capsule tab bar with search split off, inset half-height sheets, menus and action sheets that originate from their trigger.
Grouping. Adjacent glass controls go in one glass group, with the spacing set to the layout gap. Never glass on glass: what sits on a glass surface is a fill, a vibrant label, or an icon.
Scroll edge effect. Wherever glass overlaps scrollable content, add one progressive blur-and-fade band to the content at that edge: soft by default, hard for dense desktop layouts, never stacked.
Color. Monochrome foregrounds on glass; tint only the backing of a primary action or a status; brand color lives in the content layer (Reference A3 and B3).
Legibility and accessibility. At least 4.5:1 for text and icons, checked against the brightest and darkest regions of the backdrop under the element, not an average. Clear glass over bright media gets the 35% dimming layer. Provide these as theme or component variants: the Tinted look (more opaque, more contrast), reduced transparency (much frostier and more opaque, still a material), and increased contrast (near black or white surface with a contrasting border).
Tokens and components. Glass and frosted values as variables with light and dark theme values. Reusable components, each with its variants: tab bar (with search tab and optional accessory), top toolbar with grouped items, floating round button, prominent tinted button, segmented control, slider and toggle (plain at rest, glass knob in the pressed variant), search field, half-height sheet, alert, menu or popover, action sheet anchored to a control, sidebar, toast. The skill tells agents to instantiate these instead of rebuilding them, and how to create them when the document has none. If Pencraft has a library or template mechanism, ship the components through it so agents do not regenerate them in every document.
Workflow. Check what the document already has; make sure there is a backdrop worth refracting; set up tokens and components; compose the screen; render a screenshot and inspect it against a short checklist (backdrop visibly bent at the rims, text legible over the worst region, no glass on glass, radii concentric, one light direction, one variant per context, scroll edge effect where glass meets scrolling content). Fix and re-render before reporting done.
Common failures and their fixes. A short table: reads as a gray box (no backdrop behind it, or blur without saturation); muddy (tint too strong, or blur in the wrong color space); illegible label (clear variant over bright content without dimming); looks like plastic (highlight too strong or lit from several directions); glass everywhere (content layer turned into glass).
Size. The main skill file stays about as long as Pencraft's existing skills; an agent reads it in full on every use. Token tables, per-component recipes, and the design-language background go into reference files it opens when needed. Do not paste this prompt's Reference section into the skill; distill it.
Scope and honesty. This is an original implementation of a glass material style. Do not copy Apple's assets, icons, or wallpapers, and do not claim pixel parity with the system material.
Frosted glass for the content layer. Apple keeps Liquid Glass out of the content layer and uses standard blur materials there instead. The skill needs a second, simpler recipe for those (four thicknesses, see Reference D) and a clear rule for which of the two an element gets.
The Reference section is the factual basis for all of the above. Each fact there carries a confidence label. Where you can reach Apple's current documentation, re-check the items marked official and prefer what Apple says now if it differs. Target the iOS 26 look; Apple revised the material again for iOS 27, so do not mix the two.
Phase 4: Deliver to every harness

* Register the skill through whatever single source of truth Pencraft uses, so all harnesses receive the same content. Do not fork per-harness copies by hand.
* If Pencraft exports skill files for harnesses, generate them for each supported one. Check each harness's current documentation for its skill or instruction-file location and format instead of relying on memory; these conventions change often.
* If any tool names differ between harnesses, follow the mapping approach existing skills use.
* Update the skill index, README, and any docs that list available skills.
* For each harness, check the result as far as you can from here: the file is where that harness looks, the frontmatter parses, the description is within that harness's length limit. In the final report, separate the harnesses you loaded the skill in from the ones where you only generated files.

Phase 5: Prove it works
You wrote the engine and the skill, so you cannot judge whether the skill is enough on its own. Test it with fresh agents: for each design below, start a subagent that gets only the user request, the skill as an outside agent would receive it, and Pencraft's public agent tools. Give it no hints from this session. Read its transcript afterwards: every place it guessed, got a validation error, or needed several attempts is a hole in the skill or the tool surface. Fix that, then rerun with a new subagent.
Test designs:

1. a mobile screen with a photo-heavy feed, a floating glass tab bar with a search tab, and a top toolbar with grouped buttons and one tinted primary action
2. the same screen over a light, text-heavy feed, and again over a dark one, to exercise the light and dark appearance choice
3. a media player with clear-variant controls over bright artwork
4. a desktop window with a glass sidebar, a popover menu, and a frosted panel inside the content area, in dark theme
5. screen 1 in the Tinted, reduced-transparency, and increased-contrast variants

Checks on every render:

* measured, not eyeballed: contrast of each label and icon on glass against the worst backdrop region under it is at least 4.5:1
* a walk of the document tree finds no glass node whose backdrop is another glass node outside its group, and no nested radius that breaks the concentric formula
* with the shader in place, the rim visibly bends the backdrop: the render differs from a frosted-only render of the same node along the bevel band
* by eye, against the reference screenshots if present: does it read as thick curved glass, or as a flat gray translucent box? If the latter, the recipe is wrong: fix the skill or the engine, not just the test design.

If those tree checks are cheap to keep, add them as a validation command or tool that agents can run on their own designs, and have the skill's workflow call it.
Save the final renders and the subagent prompts under `docs/liquid-glass/` so I can look at them and rerun the tests.
Constraints

* Follow the repo's existing style, error handling, and module layout. No new dependencies without telling me why.
* Run the formatter, clippy, and the full test suite before you finish; everything passes.
* Do not change the behavior of existing skills or existing documents.
* The repositories listed under Sources are for understanding the technique. Do not copy code from them; their licenses vary and some rely on Apple's private APIs. Implement from the math in Reference E.
* Nothing in Pencraft may call or depend on Apple's private APIs. The private names in Reference C only explain how Apple structures its version.
* If something about the codebase contradicts an assumption in this prompt, trust the codebase and tell me.

Final report
Tell me: what you found in Phase 1, which engine changes you made and why, measured frame times before and after, which platforms and GPU backends you ran, where the skill lives and how each harness gets it (loaded versus only generated), what the fresh-agent tests exposed and how you fixed it, the paths to the test renders, and anything that is approximated, untested, or still missing.
Reference: Apple's iOS 26 design language and how its glass materials are built
Researched 14 November 2026. Confidence labels:

* [official] stated by Apple in the Human Interface Guidelines, developer documentation, or WWDC25 sessions
* [reverse-engineered] found by third parties inspecting Apple's private frameworks; names and values may change between OS releases
* [community] measured or built by third parties; a good approximation, not Apple's numbers
* [starting value] my suggestion for Pencraft, to be tuned by eye against renders

Apple publishes behavior and rules for Liquid Glass, not a numeric recipe. Anything numeric about the glass itself is therefore reverse-engineered, community, or a starting value.
A. The design language
A1. Principles and layer model

* [official] Three stated principles: hierarchy (controls elevate and distinguish the content beneath), harmony (shapes follow the concentric geometry of hardware and software), consistency (platform conventions that adapt across window sizes and displays).
* [official] Two layers. The content layer holds the app's content (lists, text, media, backgrounds). The functional layer floats above it and holds navigation and controls (tab bars, toolbars, sidebars, sheets, popovers, menus, floating buttons). Liquid Glass belongs only to the functional layer.
* [official] Exception: controls in the content layer with a transient moving part (slider knob, toggle knob) turn into glass only while the person is interacting with them, and rest as plain controls.
* [official] For structure inside the content layer, use the standard blur materials (Reference D), not Liquid Glass.
* [official] Apply glass to the control itself, not to views nested inside it. Limit custom glass to the few most important functional elements.
* [official] In a steady state (for example first launch) content and glass should not overlap awkwardly; reposition or scale content so there is separation.

A2. Shape and concentricity

* [official] Three shape types build every layout:
   1. Fixed: a constant corner radius.
   2. Capsule: radius = half the container's height.
   3. Concentric: radius = parent radius minus the padding between parent and child.
* [official] Concentric means radii and margins share one center so shapes nest. Corners that look pinched or flared are errors to fix.
* [official] Views are centered mathematically where that works and nudged for optical balance where it does not.
* [official] Capsules are used for sliders, switches, bars, buttons, and the rounding of grouped lists. On touch layouts capsules are the norm; on dense desktop layouts they are kept for standout actions.
* [official] On a phone, an element near the screen edge is a capsule with extra margin. On iPad and Mac it is a concentric shape aligned to the window edge. A component that may be nested or standalone uses a concentric shape with a fallback radius for the standalone case.
* [official] The default shape of a custom glass element is a capsule; circle, ellipse, rounded rectangle, and container-concentric rectangle are the other standard choices.
* [official] macOS control sizes: mini, small, medium keep rounded rectangles; large uses capsules; a new extra-large size uses Liquid Glass for emphasis. Windows have rounder corners so bars and controls nest into them.
* Well-known, verify: Apple's rounded shapes use continuous-curvature corners (a superellipse-like profile), not circular arcs. If Pencraft has a corner-smoothing property, turn it on for glass components.

A3. Color and typography

* [official] System colors were adjusted slightly across light, dark, and increased-contrast appearances for better hue separation next to glass. Apple warns that the values change between releases, so define tokens, do not hard-code.
* [official] Glass has no color of its own; it takes color from what is behind it.
* [official] Symbols and text on glass are monochrome: dark over light content, light over dark content.
* [official] Color on glass is used sparingly, for a key action or a status. For a prominent button (Done), tint the glass backing, not the label. Brand color belongs in the content layer (for example a colorful header), not spread across the glass.
* [official] A custom color needs light and dark variants plus an increased-contrast option for each.
* [official] Type is bolder and left-aligned in key moments such as alerts and onboarding. Section headers in lists use title-style capitalization, not all caps.
* [community] Large title 34 pt bold, left-aligned; inline title 17 pt semibold; body and list titles 17 pt regular; secondary text 15 pt; captions 13 pt; toolbar labels 17 pt medium; tab bar labels 11 pt (the minimum size).
* Well-known, verify: the full iOS text ramp at default size is Large Title 34, Title 1 28, Title 2 22, Title 3 20, Headline 17 semibold, Body 17, Callout 16, Subheadline 15, Footnote 13, Caption 1 12, Caption 2 11.

A4. Structure and components

* [official] Bars have no background of their own any more. Remove custom backgrounds and borders; hierarchy comes from layout and grouping. Bar buttons sit on glass.
* [official] Bar items are grouped by function and frequency; a group shares one glass background. Do not put a text button and a symbol button in the same group (they read as one button). Do not mix text and icons inside one shared background. Overflow goes into a "more" menu.
* [official] The primary action stays separate from groups and is tinted: on iOS typically a blue checkmark, on macOS a prominent text button.
* [official] Tab bar on iPhone: floats above content, can minimize when scrolling down and re-expand when scrolling back, can carry an accessory view above it (for example playback controls). Search is its own tab, separated at the trailing end. No screen-specific actions in the tab bar.
* [community] The tab bar is a centered capsule, not full width, inset about 21 pt from the left, right, and bottom edges; 2 to 5 tabs; the selected tab uses the accent color. The home-indicator zone is about 21 pt tall and stays free of fixed elements. Minimum touch target 44 × 44 pt. Navigation bar buttons are glass circles holding an icon.
* [official] Sheets have a larger corner radius. A half-height sheet is inset from the display edges so content shows around it, and is glass. Pulled to full height it becomes more opaque. A modal sheet is paired with a dimming layer behind it.
* [official] Action sheets originate from the control that triggered them instead of the bottom edge. Menus pop out of their button and expand in place. Menu items for common actions carry icons.
* [official] Sidebars on iPad and Mac are inset floating glass panes with content flowing behind; inspectors are edge-to-edge glass beside the content. A background-extension effect mirrors and blurs adjacent content under the sidebar so a hero image appears to continue beneath it.
* [official] Lists, tables, and forms have taller rows, more padding, and larger section corner radii to match the controls. They are content, never glass.
* [official] Scroll edge effect: where pinned controls overlap scrolling content, the content is blurred and faded near that edge instead of meeting a divider line. Soft style (default on iOS/iPadOS) fades gradually and suits glass buttons. Hard style (mostly macOS) is a more uniform, more opaque band for text controls without backgrounds and pinned headers. One effect per view, never stacked or mixed, never decorative. When the glass above has flipped to its dark style, the effect dims instead of lightening.
* [official] App icons are built from up to four layer groups on a 1024 × 1024 canvas; the system adds specular highlights, blur, translucency, and shadow, and renders default, dark, clear, and tinted variants. Shadows are chromatic over light backgrounds and neutral for dark and mono variants.

B. Liquid Glass: observable behavior
B1. Optics and motion

* [official] Apple calls it a digital meta-material: not a simulation of real glass, but one that bends, shapes, and concentrates light where older materials only scattered (blurred) it.
* [official] Lensing is the defining cue. Content is bent at the edges of the shape, which is how the eye reads the element's outline and its position above the content while it stays transparent.
* [official] Materialization: glass does not fade in or out. It appears and disappears by ramping its light bending up or down.
* [official] It flexes like a gel under touch and "energizes" with light. Elements can lift into glass on touch so the resting state stays calm.
* [official] Morphing: between states the glass reshapes between control sets as one continuous plane; nearby shapes merge and split like droplets.
* [official] Highlights come from light sources in an environment and follow the shape's geometry; in some places they respond to device motion. Light travels around the rim during events such as unlock.
* [official] Shadows are content-aware: stronger when text is underneath, weaker over a plain light background.
* [official] Interaction glow: light starts under the fingertip, spreads through the element, and reaches neighboring glass.
* [official] Interactive custom glass scales on press, bounces, shimmers, and shows touch-point illumination.

B2. Adaptivity

* [official] Every layer of the material adjusts tint, shadow, and dynamic range to what is behind it; there is no fixed light or dark look.
* [official] Small elements (navigation bars, tab bars, bar buttons) flip between a light and a dark appearance depending on the backdrop. Large elements (menus, sidebars) adapt but do not flip, because a flip over a large area is distracting, and they are more opaque.
* [official] Foreground symbols and text flip the opposite way to the glass for contrast.
* [official] Thickness scales with size. A larger element behaves like thicker glass: deeper and richer shadow, stronger lensing and refraction, softer light scattering.
* [official] Large panes pick up colored light from nearby content, which spills onto their surface and into their shadow.
* [official] An unfocused window's glass recedes visually.

B3. Variants, tint, legibility

* [official] Regular: blurs the backdrop and adjusts its luminosity; has all adaptive behaviors; legible over anything at any size. Use it by default and whenever there is much text (alerts, sidebars, popovers).
* [official] Clear: far more transparent, no adaptive behavior. Use only when all three hold: it sits over media-rich content; a dimming layer will not hurt that content; the foreground on the glass is bold and bright. Over bright content add a dark dimming layer at 35% opacity; over already dark content none is needed. For small elements the dimming can be local to the element.
* [official] Never mix regular and clear in the same context.
* [official] Never stack glass on glass. Things placed on a glass surface are thin overlays made with fills, transparency, and vibrancy.
* [official] Tinting behaves like colored glass: one chosen color yields a range of tones mapped to the brightness of the backdrop, shifting hue, brightness, and saturation while staying near the intended color. A solid opaque fill breaks the material; a tint keeps it transparent. Tint only primary actions.
* [community] The regular variant appears to hold roughly 4.5:1 contrast for foreground content; use that as the floor.

B4. Settings that change the material

* [official] Reduce Transparency: frostier glass that hides more of the backdrop.
* [official] Increase Contrast: elements become mostly black or white with a contrasting border.
* [official] Reduce Motion: weaker effects, no elastic behavior.
* [official] Since iOS 26.1 people can choose between Clear (the original look) and Tinted (more opacity, more contrast) for the whole system. Design components so both look right.

B5. Grouping semantics (from Apple's API, useful as a model for Pencraft components)

* [official] Glass cannot sample other glass, so related glass shapes are placed in one container that renders them in a single pass.
* [official] The container has a spacing value: shapes closer than it start to blend into each other. Set it to match the layout gap so shapes are separate at rest and merge during transitions.
* [official] A union operation joins several elements into one glass shape even when they are apart. Do not nest containers.
* [official] Transitions are either matched geometry (shape morphs from one element to another) or materialize (lensing ramps in).

C. How Apple implements Liquid Glass

* [reverse-engineered] It lives in Core Animation (QuartzCore), composited by the system render server, not drawn by the app. Two new private layer filters exist, `glassBackground` and `glassForeground`, each with many parameters, plus a distance-field effect family (`CASDFEffect`) with subclasses including `CASDFGlassHighlightEffect` and `CASDFGlassDisplacementEffect`.
* [reverse-engineered] Layer tree of one glass view on macOS 26:

```
host view
└── SDF wrapper layer
    ├── CABackdropLayer            captures what is behind; runs glassBackground
    │   └── CASDFLayer "@0"        outputs the shape as a distance-field texture
    │       └── container → CASDFElementLayer   (the shape itself)
    ├── CASDFLayer "@1"            edge highlight, one light angle
    │   └── portal → same shape element
    └── CASDFLayer "@2"            edge highlight, opposite angle
        └── portal → same shape element

```

* [reverse-engineered] The backdrop layer's `glassBackground` filter is pointed at sublayer "@0" for its shape and does refraction, blur, vibrancy, and tone mapping in one filter. The two highlight layers reuse the same shape through portals, apply the highlight effect from opposing directions, and are toned by a 4 × 5 color matrix (`vibrantColorMatrix`).
* [reverse-engineered] Parameter names observed on the filter include inner and outer refraction amounts, refraction opacity, aberration amount, and a key/fill highlight amount. Values seen on system controls in one third-party inspection: inner refraction −80, outer refraction about 10, key/fill highlight 0.4. Read these as "strong inward bend at the rim, slight outward bend outside it, moderate highlight," not as numbers to copy.
* [reverse-engineered] The backdrop only contains layers behind the glass; anything parented above it never shows up in the refraction.
* Consequences for Pencraft: (1) render the shape to a distance field once and feed it to every pass; (2) refraction, blur, saturation, and tone mapping can be one pass over the captured backdrop; (3) highlights are separate, cheap passes over the same distance field; (4) chromatic aberration is a real parameter of the system material, kept small; (5) the backdrop is a snapshot of lower z-order content only.

D. How frosted glass (standard materials) is implemented

* [official] A visual effect view captures the content behind it and applies filters to that capture. Four thicknesses on iOS: ultra-thin, thin, regular (default), thick. Thicker is more opaque and gives better contrast for fine text; thinner keeps more context visible. Choose by purpose, not by apparent color, because system settings change the result.
* [official] Content on a material uses vibrant colors: four label levels (primary to quaternary; avoid quaternary on thin and ultra-thin), three fill levels, one separator level.
* [reverse-engineered] Layer structure: a container holding a backdrop layer (the blur) and a tint layer above it.
* [reverse-engineered] The backdrop is sampled at quarter resolution (scale 0.25) and samples a margin beyond its bounds (a bleed of about 10 pt in one replica) so the blur has data at the edges, while the visible result is clipped to the layer. Backdrops that share a group name composite as one seamless surface.
* [reverse-engineered] Filter chain on the capture, in order: saturation boost, Gaussian blur with normalized edges, small brightness lift. Example values from a faithful replica: blur radius 30, saturation 1.8, brightness +0.02. The saturation boost is what keeps a strong blur from looking gray.
* [reverse-engineered] The tint is a solid color layer composited with a blend mode: a light tint with a darken blend for light materials, a dark tint with a lighten blend for dark materials. That clamps the backdrop toward the tint instead of simply washing over it, which keeps contrast steady. Example: white at about 10% with 50% alpha.
* [reverse-engineered] Each material is a stored recipe per state and appearance (active, inactive, light, dark, high contrast). Inactive usually means lower saturation. Reduce Transparency swaps blur and tint for an opaque fill. Increase Contrast makes the rim fully opaque at 1 pt.
* [reverse-engineered] Progressive blur (the kind behind scroll edge effects) is a variable blur: one maximum radius plus a gradient mask whose alpha sets the radius per pixel, from full blur to none.
* [starting value] Pencraft frosted tokens (blur radius / saturation / tint opacity, light theme with a white tint, dark theme with a near-black tint): ultra-thin 12 / 1.6 / 0.20; thin 20 / 1.7 / 0.35; regular 30 / 1.8 / 0.55; thick 40 / 1.8 / 0.75. Add a 1 px inner stroke of white at 10 to 20% in light and 6 to 12% in dark.
* Renderer notes: do the blur on a downsampled copy (quarter resolution matches Apple) with a multi-pass kernel (dual Kawase or separable Gaussian), sample beyond the node bounds by the blur radius, apply saturation before the blur and the tint after it.

E. Optics model for the Liquid Glass shader
All [community] unless marked; this is the technique the best public recreations converge on.

* Simplifying assumptions: air has index 1, glass about 1.5 (crown glass 1.5168 is a common choice); the view ray is perpendicular to the screen; one refraction event per ray; the backdrop plane sits directly under the glass.
* The glass is a flat slab with a curved bevel at the rim. Its height profile is a function of normalized distance from the edge, x from 0 (outer edge) to 1 (where the bevel meets the flat top):
   * convex circle: `h(x) = sqrt(1 − (1 − x)²)`. Abrupt meeting with the flat top, harsher refraction edge.
   * convex squircle: `h(x) = (1 − (1 − x)⁴)^(1/4)`. Smooth meeting with the flat top; this is the profile that reads as Apple's glass and should be the default.
   * concave: `1 − convex(x)`. Pushes samples outside the shape; avoid for panels.
   * lip: blend of convex and concave by a smootherstep of x. A raised rim with a shallow dish, used for switch-knob style lenses.
* Normal: take the slope of the profile (numerically, with a step near 0.001, or analytically); the surface normal in the cross-section is `(−slope, 1)` normalized, and its screen-space direction is the gradient of the shape's distance field.
* Refraction: bend the view ray `(0, 0, −1)` at the surface with Snell's law (`refract(I, N, 1/ior)` in shader terms) and follow it down to the backdrop plane: `offset = R.xy / (−R.z) × (h × thickness + base)`. Sample the backdrop at the pixel position plus that offset. For a convex bevel the offset points inward, toward the shape's center, so the rim shows a compressed, magnified slice of the content further in.
* The offset depends only on distance from the edge, so it can be precomputed along one radius (127 samples fits an 8-bit map) and stored normalized by its maximum; a single scale parameter then restores pixel distances. Ramping that scale from 0 is exactly Apple's "materialize" transition.
* If the offset decays faster than one pixel per pixel the field folds and the same backdrop content appears twice. Clamp the slope unless a swirl is wanted.
* Sample the displaced backdrop with bilinear filtering; nearest-neighbor sampling shows stair-steps along the bent rim.
* Dispersion: compute the offset with a slightly different index per color channel, strongest at the rim. Keep it barely visible for UI glass.
* Rim light: brightness from the angle between the surface normal's screen direction and a fixed light direction (a common default is light from the upper left, about −60°), confined to the bevel band, plus a weaker second highlight from the opposite angle to match Apple's two highlight layers. A Fresnel term (Schlick) brightens the extreme edge.
* Shadow: a soft drop shadow plus a tight ambient edge shadow, both derived from the same distance field.
* Merging: combine neighboring shapes' distance fields with a smooth minimum whose radius is the container's spacing; the merged field then drives refraction and highlights so the bridge between droplets is lit and refracts correctly.
* Typical parameter ranges in public implementations, for a control about 44 to 60 px tall: bevel width 20 to 45 px, thickness 40 to 80 px, index 1.5, rim highlight opacity 0.2 to 0.5, blur 0 to 2 px for a pure lens and more for the regular variant.
* [starting value] Regular variant: backdrop blur 8 to 14 px with saturation 1.5 to 1.8, luminosity pulled toward the tint, tint opacity 0.10 to 0.25, squircle bevel, dispersion near zero. Clear variant: blur 0 to 3 px, tint opacity 0 to 0.06, same lensing, plus the 35% dimming layer over bright media.

F. Translating adaptive behavior into a static design tool
Apple's glass adapts live. In a design file the agent has to decide the state once:

* Sample the luminance of the backdrop region under each small glass element and choose the light or dark glass variant from it, then set the foreground to the opposite. Large panes keep the theme's variant.
* Scale thickness, lensing strength, and shadow depth with element size: bar buttons thinnest, bars medium, menus, sheets, and sidebars thickest and most opaque.
* Raise shadow opacity when the element sits over text or busy content, lower it over plain light areas.
* Provide Clear and Tinted looks, a reduced-transparency look, and an increased-contrast look as theme or component variants.
* Add the scroll edge effect (a progressive blur and fade band on the content) wherever a glass bar overlaps scrollable content, once per edge.
* Show transitions as separate frames: merged droplet shapes for morphing, lower lensing for a materializing element.

Sources
Apple:

* Human Interface Guidelines, Materials: https://developer.apple.com/design/human-interface-guidelines/materials
* Human Interface Guidelines, Color: https://developer.apple.com/design/human-interface-guidelines/color
* Adopting Liquid Glass: https://developer.apple.com/documentation/technologyoverviews/adopting-liquid-glass
* Applying Liquid Glass to custom views: https://developer.apple.com/documentation/SwiftUI/Applying-Liquid-Glass-to-custom-views
* WWDC25 219, Meet Liquid Glass: https://developer.apple.com/videos/play/wwdc2025/219/
* WWDC25 356, Get to know the new design system: https://developer.apple.com/videos/play/wwdc2025/356/
* WWDC25 323, Build a SwiftUI app with the new design: https://developer.apple.com/videos/play/wwdc2025/323/
* Creating your app icon using Icon Composer: https://developer.apple.com/documentation/xcode/creating-your-app-icon-using-icon-composer

Reverse engineering:

* ShatteredGlass (layer tree of macOS 26 glass): https://github.com/AlexStrNik/ShatteredGlass
* Guilherme Rambo on the private filters: https://mastodon.social/@_inside/114658957054004337
* LiquidGlassFallback (observed filter parameters): https://github.com/AotianNeo/LiquidGlassFallback
* Oskar Groth, Reverse Engineering NSVisualEffectView: https://oskargroth.com/blog/reverse-engineering-nsvisualeffectview
* Variable blur via backdrop layer: https://designcode.io/swiftui-handbook-progressive-blur

Optics and recreations:

* kube.io, Liquid Glass in the Browser: https://kube.io/blog/liquid-glass-css-svg/
* liquid-glass-refraction-shader: https://github.com/m2-md/liquid-glass-refraction-shader
* the-best-liquid-glass: https://github.com/lzq20121022-tech/the-best-liquid-glass
* hyalite: https://github.com/VII-Cae/hyalite--liquid-glass
* LiquidGlass for iOS 15+ (Metal): https://github.com/anuero/LiquidGlass

Metrics and settings:

* Learn UI Design, iOS 26 design guidelines: https://www.learnui.design/blog/ios-design-guidelines-templates.html
* iOS 26.1 Clear/Tinted setting: https://www.macrumors.com/2025/10/20/ios-26-1-liquid-glass-toggle/
* Liquid Glass reference (accessibility and behavior notes): https://www.conor.fyi/writing/liquid-glass-reference
