# Pencraft's design library today (working tree of 20 November 2026, HEAD 9e79a84, v0.3.26)

All sources are files in the repository, cited as `path:line`. Paths are relative to
`C:\Users\asp3x\Documents\development\pencraft`. "Not verified" means I did not open that text; see each Gaps list.
Read fully: `app/src/craft.rs`, `app/src/skills.rs`, `app/MECHANICS.md`, `app/src/agent.rs:1-330` (the whole GUIDE),
`app/skills/ui-components/SKILL.md`, `reports/UI component library lessons.md`, `app/src/chat/critic.rs:1-90`.
Read by headings and targeted grep only: the reference files of all skills, the SKILL.md bodies of liquid-glass,
logos and devices, `review.rs`, `AI.md`, `CHANGELOG.md`, `model.rs`, `connector.rs`, `path.rs`, `pattern.rs`.

## 1. What each of the four skills contains, what it excludes, and which requests fall to no skill

### Takeaway
The library is four skills (16 files, 2,840 lines): one material (liquid-glass), one neutral component kit
(ui-components), one device frame (devices, a single iPhone template) and one identity method (logos). Everything
that "has a look of its own" but is not a logo (posters, landing pages, illustration, charts, slides, diagrams,
social graphics, icon sets) and every named design system other than shadcn/Radix/Base UI falls back to the general
guide alone.

### Cited Findings

Registration and size
- Four skills are registered, in this order: liquid-glass (4 files), ui-components (6 files), devices (1 file), logos (5 files) — `app/src/skills.rs:31-63`.
- File sizes in lines: devices/SKILL.md 143; liquid-glass SKILL.md 220, components.md 247, design-language.md 80, tokens.md 116; logos SKILL.md 242, construction.md 134, detail.md 180, styles.md 213, wordmarks.md 89; ui-components SKILL.md 215, controls.md 270, data.md 186, overlays.md 224, styles.md 134, tokens.md 147; total 2,840 (`wc -l` over `app/skills`).
- Skill commits: liquid-glass with 0.1.0 (94bf7ce, 14 Nov); ui-components (16fab0c, 14 Nov); devices in 0.2.0 (6ecc9a4, 15 Nov); a touch in 0ff8a8d (17 Nov, connectors/states/patterns); logos in faf47ea (18 Nov) — `git log -- app/skills`.

liquid-glass
- Description: Liquid Glass of iOS 26 / macOS 26 / visionOS for floating bars and controls, plus frosted panels; triggers on "liquid glass, glass UI, glassmorphism, frosted or translucent panels ... make this look like glass". It has no "Do not use for" clause — `app/skills/liquid-glass/SKILL.md:3`.
- SKILL.md sections: 1 what each surface is made of; 2 something to bend; 3 apply the material; 4 what a live system would adapt; 5 shape; 6 bars; 7 glass group; 8 glass over scrolling content; 9 colour and legibility; 10 workflow; 11 when it looks wrong — `app/skills/liquid-glass/SKILL.md:23,46,64,88,105,119,142,157,177,190,208`.
- Component headings in `references/components.md`: Tab bar with search (:28); Toolbar: back button, grouped actions, primary action (:54); Round button and prominent button (:73); Segmented control (:87); Search field (:101); Slider and toggle (:112); Half-height sheet (:134); Alert (:149); Menu, popover, action sheet (:166); Sidebar (tablet and desktop) (:187); Media transport (clear glass over artwork) (:209); Toast (:227); A frosted panel in content (:235).
- `references/tokens.md`: The variables (:6), The four looks (:45), Retuning a preset (:71), The numbers (`advanced`) (:84), Sizes that fit this design language (:106): desktop window 1280 x 800 with 32-36 toolbar controls and a 240-260 sidebar; phone 393 x 852, tab bar 62 high, titles 34/17 (:108-116).
- `references/design-language.md` headings: Two layers, What makes it read as glass, What a live system adapts, Shape, Bars, Sheets/menus/alerts, Scroll edge effect, Colour and type, What Pencraft does not do (:7-75). Not done: motion, touch glow, corner smoothing, variable blur, glass on shapes other than frame/rectangle/whole ellipse (:75-80).
- Its own check tool is `check_glass` — `app/src/agent.rs:291-293`, `app/MECHANICS.md:265`.

ui-components
- Description: buttons, inputs, selects, checkboxes, switches, tabs, cards, tables, dialogs, menus, popovers, tooltips, toasts, drawers, command palettes; triggers on a component, component set, design system or UI kit, a form, a settings screen or dialog, shadcn/ui, Radix, Base UI, Sonner, Vaul, cmdk, "standard, default or plain controls", states. "Do not use for glass or translucent surfaces (the liquid-glass skill), nor for posters, landing pages and other designs with a look of their own." — `app/skills/ui-components/SKILL.md:3`.
- Six hard rules (build what was named; every value a variable; one height per row; 1-wide lines; states as frames; text 14, nothing above 18) — `SKILL.md:20-31`.
- Two presets only: Classic (36 high, shadcn Vega) and Compact (32, Nova) — `SKILL.md:58-61`; request-to-preset table — `SKILL.md:38-45`.
- Sections: read the request, tokens, sizes and type, lines/corners/elevation, states, overlays in a still picture, workflow, checklist, what Pencraft cannot draw, when it looks wrong — `SKILL.md:33,50,63,86,106,135,161,180,190,202`.
- `references/controls.md` headings: Button (:26), Input (:69), Select (:96), Checkbox, radio, switch (:108), Slider (:142), Badge (:153), Tabs (:166), Field (:186), Focus and invalid: the ring wrapper (:205), State board (:236).
- `references/overlays.md` headings: Menu (:8), Popover (:47), Tooltip (:62), Dialog (:75), Drawer (:131), Toast (:156), Command palette (:184).
- `references/data.md` headings: Card (:7), Table (:34), List row (:66), Avatar (:84), Alert (:97), Empty state (:115), Skeleton, progress, separator (:136), Kbd and breadcrumb (:161).
- `references/tokens.md` headings: The variables (:7), What each is for (:46), Radius (:80), Elevation (:95), Spacing (:126), Where these numbers come from (:140).
- `references/styles.md` headings: shadcn/ui (:8, eight styles), Radix Themes (:75, sizes 1-4), Base UI (:115), Sonner, Vaul, cmdk (:123).
- Ready JSON blocks with stable `ui-` ids exist for (root ids found by grep over the reference files): ui-button, ui-button-icon-leading, ui-icon-button, ui-input, ui-search-input, ui-textarea, ui-select, ui-checkbox, ui-radio, ui-switch, ui-slider, ui-badge, ui-tabs, ui-field-example, ui-focus-example, ui-invalid-example, ui-states-example, ui-menu, ui-popover, ui-tooltip, ui-dialog, ui-dialog-example, ui-drawer, ui-toast, ui-command, ui-card, ui-table (head + 3 rows x 3 cells), ui-list-row, ui-avatar, ui-alert, ui-empty, ui-skeleton-*, ui-progress, ui-kbd, ui-breadcrumb.
- Described in one or two sentences of prose only, with no layers: sub-menu (`overlays.md:40`), hover card (`overlays.md:60`), alert dialog (`overlays.md:123`), sheet (`overlays.md:128`), context menu (shares the Menu build, `overlays.md:10`), spinner and separator (`data.md:158`), pagination and accordion (`data.md:183-186`), error and no-results states (`data.md:133`).
- Checks: an 7-point checklist (`SKILL.md:180-188`) and an 8-row symptom table (`SKILL.md:204-213`).

devices
- Description: an iPhone with real status bar, Dynamic Island, home indicator and side buttons, "an Android phone or an iPad"; use for any mock/mockup/prototype/screen of a phone or tablet app. "Do not use for an app icon, a logo, one component on its own, or when the user asks for the screen alone." — `app/skills/devices/SKILL.md:3`.
- Sections: 1 Insert the device first (:17, the iPhone 16 template), 2 Every iPhone since the XR (:67, a table; the unit test needs at least 19 rows, `app/src/skills.rs:426-431`), 3 Several screens (:122, side by side 96 apart), 4 Other devices (:127), 5 Check (:139).
- "There is no template for these yet": the Android phone (Pixel, 436 x 939 body, 412 x 915 screen) and the iPad 11 inch (860 x 1220 body, 820 x 1180 screen) are numbers in prose only — `app/skills/devices/SKILL.md:127-137`.
- Template numbers also recorded in `app/MECHANICS.md:262`.

logos
- Description lists styles (flat geometric, monoline, negative space, vintage badge and roundel, heraldic crest, seal, engraved, mascot, hand-lettered, luxury monogram, gradient and dimensional, app icon, retro, art deco, mon, pixel) and triggers (logo, mark, wordmark, monogram, emblem, crest, coat of arms, badge, seal, signet, mascot, favicon, app icon). "Do not use for an illustration, a poster, an interface icon set, or a screen that merely shows a logo." — `app/skills/logos/SKILL.md:3` (metadata version "2").
- SKILL.md sections 1-12: brief in four lines, choose the style, find the idea, three rough marks, build exactly, type and lockup, emblem, app icon or favicon, small version, colour, proofs, what is delivered; then "What gives a generated logo away" — `app/skills/logos/SKILL.md:37,53,73,97,112,143,159,180,187,198,206,224,231`.
- `references/styles.md` style headings: Flat geometric (:31), Monoline (:39), Negative space (:48), Vintage badge (:54), Heraldic crest and coat of arms (:76), Seal (:99), Engraved, woodcut and linocut (:107), Mascot and sports (:118), Illustrative and hand-drawn (:137), Hand-lettered script and custom lettering (:142), Luxury monogram and serif wordmark (:151), Gradient and dimensional (:158), Grid, pattern and isometric (:171), Retro (:176), Art deco (:183), Mon (:188), Playful rounded (:194), Psychedelic and maximalist (:199), Pixel (:203), Brutalist (:207), Stencil (:211); 21 styles.
- `references/construction.md`: The numbers (:11), Worked marks (:90), Checking the construction (:129). `references/detail.md`: order of work (:7), Hierarchy (:18), Contours and keylines (:37), Cutting to a shape: masks (:48), Text on a circle (:61), Text along a line (:78), Constructions (:87), Worked logos (:157). `references/wordmarks.md`: choosing the face (:3), case/weight/spacing (:24), where capitals sit (:34), horizontal lockup (:51), stacked lockup (:64), name over two lines (:73), second line (:79), checking (:84).
- Logos have their own counted review (`logo_notes`, limits of 12 shapes, 10 colours, 3 weights, 2 faces, 24 px smallest mark) and their own judge text — `app/src/review.rs:1316-1327,1422`, `app/src/craft.rs:407`.

What reaches a model outside the skills (the general guide)
- `get_guide` = GUIDE + skills index + CRAFT + installed fonts — `app/src/craft.rs:399-401`. The chat brief carries the same index, CRAFT and fonts — `app/src/chat.rs:98`.
- CRAFT sections: preamble and "A named reference is the brief" (`craft.rs:22-34`), 1 Decide before you draw (:36), 2 Set the system first (:72), 3 Build with a hierarchy (:90), A screen on a device (:133), A logo or a mark (:145), 4 What gives a generated design away (:165), 5 Look and judge (:188), 6 Changing a design that is already there (:201).
- CRAFT knows three kinds of surface: something that has to win a person over (landing page, poster, cover, invitation), a screen someone works in, a page someone reads — `craft.rs:42-58`.
- Illustration in the whole guide is one bullet: "Draw one confident picture for it from shapes and `path` layers ... large and in one place" — `craft.rs:117-120`.

Requests that no skill covers (keyword counts are case-insensitive greps over GUIDE `agent.rs:21-220`, CRAFT `craft.rs:20-217` and `app/skills`)
- Posters and landing pages: explicitly excluded by ui-components and logos (`ui-components/SKILL.md:3`, `logos/SKILL.md:3`); only CRAFT's kind-of-thing paragraph applies (`craft.rs:43-46`).
- Illustration: excluded by logos (`logos/SKILL.md:3`); one CRAFT bullet (`craft.rs:117-120`).
- Interface icon sets: excluded by logos (`logos/SKILL.md:3`); CRAFT says only "`icon` layers from one set at one size per role" (`craft.rs:116`).
- Charts: CRAFT mentions them once, as a tell ("little charts and progress rings that show nothing real", `craft.rs:171-172`); ui-components mentions only chart colours of shadcn's neutral theme (`ui-components/references/styles.md:64`); GUIDE's one hit is "flow chart" in the connector entry (`agent.rs:41-42`).
- Diagrams and flow charts: GUIDE teaches the `connector` layer (`agent.rs:41-62`); no skill mentions "connector" (0 hits in all four skills).
- Slides / presentations: "presentation" 0 hits anywhere; "slide" hits in skills are slider/slide-in wording, none in GUIDE or CRAFT.
- Social graphics, infographic, banner, business card, resume, website/web page, responsive, breakpoint: 0 hits in GUIDE, CRAFT and all skills.
- Wireframes: one hit, as a fault ("A wireframe" row, `ui-components/SKILL.md:206`); Base UI's black-and-white documentation look is the nearest thing (`ui-components/references/styles.md:115-121`).
- Material Design and other systems: `names_a_reference` recognises "material design", "material ui", "material 3", "bootstrap", "tailwind", "ant design", "fluent", "carbon design", "human interface", "chakra", "mantine" (`craft.rs:289-312`) and CRAFT says "If a skill covers the reference, read it first" (`craft.rs:32-33`), but "Material" has 0 hits in `app/skills/ui-components` and `styles.md` has only shadcn/ui, Radix Themes, Base UI, Sonner/Vaul/cmdk (`styles.md:8,75,115,123`).
- Android app content and tablets: devices gives Pixel and iPad body numbers only (`devices/SKILL.md:127-137`).
- Desktop app window or browser frame: no template; only liquid-glass's "desktop window 1280 x 800 ... three neutral dots" line (`liquid-glass/references/tokens.md:108-111`).
- Dashboards: 1 hit in CRAFT (the list of working screens, `craft.rs:47`), 0 in GUIDE, 0 in skills.

### Inferences
- Because a named reference switches off the dealt directions and the review's default-face remark (`chat.rs:1020`, `review.rs:1007`) and tells the model to keep "that reference's face, palette, measures and states" (`craft.rs:28-34`), a request for Material 3, Fluent, Carbon, Ant or Apple HIG (non-glass) today removes Pencraft's originality machinery while supplying no numbers: the model must draw the system from memory.
- liquid-glass is the only skill with platform-native (iOS) component anatomy; ui-components is web/desktop-library anatomy with a short phone column (`ui-components/SKILL.md:79`). A plain, non-glass native iOS or Android screen has no component source.
- The exclusion clauses of ui-components and logos name exactly the kinds that have no skill (posters, landing pages, illustration, icon sets), so they are the most natural names for new skills.

### Gaps
- I did not read the bodies of the liquid-glass, logos and devices SKILL.md files or any reference file end to end; component lists come from headings and ids, so sub-variants inside a section (for example button variants and sizes in `controls.md:26-68`) are not enumerated here.
- Which of the `ui-` blocks carry `"reusable": true` was not checked one by one; the unit test only asserts at least 20 reusable components (`skills.rs:371`).
- A `research_notes/imagery` folder exists and was not read.

## 2. Standard components still missing from ui-components

### Takeaway
ui-components covers form controls, overlays and a few data parts, but has no navigation components, no date or
advanced inputs, no data-table features and no page compositions at all; about seven items exist only as a sentence
of prose.

### Cited Findings
Case-insensitive grep counts over `app/skills/ui-components` (all six files):
- Zero mentions: date picker, stepper, navigation menu, navbar, segmented (control), number input, OTP / one-time, file upload, dropzone, tag input, chips, carousel, timeline, rating, toggle, toggle group, sort, KPI, pricing, sign in, login, dashboard, collapsible, scroll area, resizable, menubar, color picker, list-detail, onboarding, chat / message bubble, code block, banner, radio group, password.
- Calendar: only as icon names `calendar-plus` and `calendar` (`data.md:124`, `overlays.md:201`); no calendar or date picker component.
- Combobox and date field: named once, only to say they are "drawn closed, showing a value" (`SKILL.md:151`); no build.
- Sidebar: only inside sample copy "Pinned to the sidebar" (`overlays.md:28`); no sidebar or app-shell component. (liquid-glass has a glass Sidebar, `liquid-glass/references/components.md:187`.)
- Tree: one unrelated hit in `styles.md` (Figtree face, `styles.md:27`); no tree view.
- Chart: only "Chart colours of the neutral theme" (`styles.md:64`).
- Pagination and accordion: two sentences of prose, no layers (`data.md:183-186`).
- Context menu: "For a dropdown menu, a context menu and the list of a select" share one Menu build (`overlays.md:10`). Sub-menu: one bullet (`overlays.md:40`).
- Sheet: one sentence ("a dialog fixed to a side: full height, 384 wide", `overlays.md:128`). Hover card: one sentence (`overlays.md:60`). Alert dialog: one sentence (`overlays.md:123`).
- Textarea: present (`ui-textarea`, mentioned 5 times in `controls.md`).
- Table: head and three rows of three cells (`ui-table-*` ids, `data.md:34`); no sort indicator, filter bar, row selection, pagination row or bulk-action bar ("sort" 0 hits; "filter" only in empty-state prose, `data.md:133`).
- Statistic: the only trace is "600 is for a figure in a statistic and for nothing else" (`SKILL.md:83`) and the figure-column checklist line (`SKILL.md:187`); no stat/KPI card.
- Settings: appears in the description (`SKILL.md:3`) and in dialog sample copy (4 hits in `overlays.md`); no settings-page composition. Auth form, pricing card, dashboard composition: none.
- Tab bar / bottom navigation: only as something a toast must sit above (`SKILL.md:148-150`); no component.
- Segmented control exists only in glass form (`liquid-glass/references/components.md:87`).
- The older report's planned contents that did not arrive: "combobox with chips", "toggle and toggle group", "sidebar" as components, and "accordion", "pagination" as builds (`reports/UI component library lessons.md:182`).

### Inferences
- The missing set groups cleanly into four reference files a later report could propose: navigation (sidebar, navbar/app bar, navigation menu, menubar, bottom tab bar, stepper, pagination, breadcrumb already there), dates and advanced inputs (calendar, date picker, combobox, tag input, number input, OTP, file upload, rating, segmented control, toggle group), data display (table features, stat card, chart, timeline, tree, carousel, accordion) and compositions (settings page, auth form, dashboard, pricing card).
- The table build as shipped cannot answer "a data table with sorting and selection" without the model inventing the parts.

### Gaps
- Grep proves absence of the words, not of an unnamed equivalent; I did not read `controls.md`, `overlays.md` and `data.md` line by line.

## 3. The older report's plan: done and open

### Takeaway
Nearly all P0 "fidelity" items and the ui-components skill are done; of the six other planned skills none exists as
a skill (states were folded into ui-components); almost all P1/P2 review checks are open; three engine gaps closed
(shadow spread, component states, masks) but the ui-components skill still tells the model they are missing.

### Cited Findings

Fidelity changes (plan table at `reports/UI component library lessons.md:154-166`)
- Named-reference rule: DONE — `app/src/craft.rs:28-34`; the dealt note ends "None of this note applies when the user named a library ..." (`craft.rs:278`); word list `craft.rs:289-312`.
- Do not deal directions for component/working-screen requests: PARTLY — not dealt when the first message names a reference or a logo (`app/src/chat.rs:1019-1022`); otherwise still dealt, with ground and face downgraded to "a first suggestion only" for working screens, reading pages and components (`craft.rs:278`).
- Request ledger: DONE in CRAFT — `craft.rs:37-40`.
- Critic class "components or a standard kit" with fidelity instead of originality, plus a request check: DONE — `app/src/chat/critic.rs:34,38,44`. A fifth class, logo, was added too (`critic.rs:34,42`).
- Declared conventions passed to the counted review: DONE for the face — `review(..., follows)` (`app/src/review.rs:1230`), set from the chat's request (`app/src/app/tools.rs:614`), used at `review.rs:1007`. Whether gradient/effect notes are silenced when effects were requested: not verified.
- Skills index in the chat brief: DONE — `app/src/chat.rs:98`.
- Starter document replaced: DONE in substance — `app/src/samples.rs:33-34` is a "Starter kit" of "Five components built from the document's variables" with a Mode switch and `$--font`.
- Vocabulary for "denser", "add states", "dark mode", "accessible", "options": OPEN — CRAFT section 6 still defines only bolder, quieter/calmer, simpler/cleaner, polish, review (`craft.rs:201-216`); "dark mode|dark theme" and "accessib" have 0 hits in CRAFT.
- Named library with several looks: pick the default, say so, offer the other: DONE — `craft.rs:33-34`, `ui-components/SKILL.md:41`.
- Per-document record of accepted decisions: no trace found; not verified by a dedicated search.
- Art-director review for MCP clients: OPEN — `review_design` occurs only in `app/src/chat/critic.rs:25` and `app/src/chat/steps.rs:98`; the tool list has 19 names without it (get_state, get_guide, list_skills, read_skill, read_layers, edit_layers, screenshot, check_glass, select_layers, open_document, save_document, export_layers, generate_image, edit_image, search_web, read_web_page, search_images, download_image, ask_user — `app/src/agent.rs`, `"name":` entries).

Guide additions (plan text at `reports/...lessons.md:168-174`)
- A ~25-line "standard controls" block in CRAFT: NOT added; CRAFT still has only the one control bullet (`craft.rs:102-105`). The ladder lives only in the skill (`ui-components/SKILL.md:63-81`).
- Floating-surface exception to "one way only": DONE — `craft.rs:108-111`. "The face of a library the user named was chosen": DONE — `craft.rs:176`.
- GUIDE properties: `layoutPosition: "absolute"` DONE (`agent.rs:103-105`); `textAlignVertical`, `underline`, `strikethrough` DONE (`agent.rs:65-66`); ellipse `innerRadius`, `startAngle`, `sweepAngle` DONE (`agent.rs:37-38`); structural icon names and nearest-name answer DONE (`agent.rs:83-87`); slot only in passing (`agent.rs:152`); `context` field not in GUIDE (it exists in the model, `app/crates/pen-doc/src/model.rs:233`).

New skills (plan table at `reports/...lessons.md:180-188`)
- `ui-components` (P0): DONE, with the omissions listed in section 2.
- `component-states` (P1): folded into ui-components (`SKILL.md:106-133`, `controls.md:236`); no separate file; own-state list is three lines (`SKILL.md:120-122`).
- `data-and-numbers` (P1): OPEN. Only the figure paragraph in the fonts note (`craft.rs:392`), CRAFT's "figures get one fixed width" (`craft.rs:122-123`) and one checklist line (`ui-components/SKILL.md:187`).
- `app-screens` (P1): OPEN (0 hits for dashboard, login, sign in in the skill).
- `stress-states` (P2): OPEN. `ai-product-ui` (P2): OPEN (0 hits for chat/message bubble, code block). `landing-effects` (P2): OPEN.
- Added although not in that plan: `devices` and `logos` (`skills.rs:52-62`).
- Process: "every fenced JSON through the counted review" is DONE for ui-components (light and dark, `skills.rs:345-376`) and the device template (`skills.rs:378-403,422-486`); liquid-glass and logos blocks are only parsed and checked for material and glass-group fields (`skills.rs:283-317`). Fresh-agent component scenarios: DONE, four of the five proposed (`docs/ui-components/tests/README.md`, "The tests"; no dark dashboard). A component sheet in `checks/design-review/`: OPEN (still the nine cases, `checks/design-review/pens`).

Review checks (plan table at `reports/...lessons.md:194-213`)
- Unknown icon name answered with the nearest name: DONE — `agent.rs:86-87`.
- Hairline of 1 at 20% or less not counted as outline-plus-shadow: DONE — `review.rs:44-46,327`.
- Radius limit 5 when all are variables: DONE — `review.rs:37-40`. Concentric-radius check outside glass: not found in the check list of `MECHANICS.md:260` (only `check_glass` has it, `MECHANICS.md:265`).
- Default family silent for a named library or a variable-set face: DONE — `review.rs:1007` (`follows || seen.unbound_faces == 0`); test `review.rs:1664`.
- P1 checks (unequal control heights in a row, control ladder, side padding of a control, icon discipline, target size, shadow vocabulary, heavy lines, dark-theme edges, both themes, mixed decimals in figure columns): OPEN as far as the documented check list shows — `MECHANICS.md:260` lists none of them; `controls()` checks only 4 above and below a label and 4 between a control and a text line (`review.rs:580-650`).
- P2 checks (off-token values, main actions, effects budget): not in `MECHANICS.md:260`. State coverage by the critic: partly, as "every component, variant and state that was asked for there" (`critic.rs:38`).
- New since the plan: bare-screen-without-device note (`review.rs:1248-1300`, `MECHANICS.md:262`), logo counts (`review.rs:1316-1422`), list ending on a bar and trailing rule (`review.rs:652-734`), container around a screen (`agent.rs:204-209`).

Engine gaps (plan table at `reports/...lessons.md:219-230`)
- Shadow `spread`: DONE in 0.3.21 — `MECHANICS.md:71-72`, `CHANGELOG.md:149`. Not taught: GUIDE's Effects section lists colour, offset, blur only (`agent.rs:122-125`), and ui-components still says "A shadow that spreads" is missing (`SKILL.md:195`) and "They have no spread, which Pencraft does not have" (`references/tokens.md:97`).
- Tabular numerals / OpenType features: OPEN — no `tabular`, `tnum` or `fontFeature` in `app/src` or `pen-doc/src` except a world's wording (`craft.rs:222`); `SKILL.md:194`.
- Variants and states: DONE as component `states` in 0.3.8 — `MECHANICS.md:13`, `agent.rs:154-159`, `CHANGELOG.md:474`. ui-components still lists "Variants and states on a component" as missing (`SKILL.md:197`), builds states "as overrides on an instance" (`SKILL.md:108`) and contains no `"states"` key (0 hits in all skills).
- Colour variable with alpha at the use site: DONE — `{"type":"color","color":"$name","opacity":..}` (`agent.rs:109-110`, `MECHANICS.md:14`).
- Truncation, max lines, min/max size, wrapping layout: OPEN — no such field in `model.rs:219-380`; `SKILL.md:198`.
- Several strokes on one layer: `stroke` takes a list of paints (`agent.rs:108`) and a text takes a list of outlines (`agent.rs:78-80`); separate widths per stroke on a shape not seen; skill still says missing (`SKILL.md:196`).
- Dashed strokes: only on connectors (`MECHANICS.md:12`; `dash` occurs in `pen-doc/src` only in `connector.rs`); `SKILL.md:199`.
- Masks: DONE in 0.3.11 — `MECHANICS.md:190`, `agent.rs:91-97`. Blend modes: OPEN — "effects' blend modes" under Not in this version (`MECHANICS.md:307`).
- Corner smoothing: OPEN — `liquid-glass/references/design-language.md:77`. OKLCH input: not verified.

### Inferences
- The cheapest high-value fix is a revision of ui-components itself: its "cannot draw" table (`SKILL.md:190-200`) is wrong on two of seven rows since 0.3.8/0.3.21, so models are steered away from component `states` and ring shadows that the engine now has.
- Because the skills' own text is compiled in and tested (`skills.rs:345-376`), moving the state recipes to real `states` would be verified automatically in light and dark.

### Gaps
- `BRIEF` in `chat.rs` and the test bodies of `review.rs` were not read; "open" for P1/P2 checks rests on `MECHANICS.md:260`, the function list of `review.rs` and the `controls()` body, not on a full read of `review.rs`.
- `samples.rs` was only grepped (first matches); that it is free of the old indigo gradient is likely but not confirmed line by line.

## 4. What the engine can draw that no skill or guide section teaches, and "Not in this version"

### Takeaway
GUIDE documents nearly every drawable property, but only as syntax; connectors, pattern fills, component states,
arcs, picture generation and web pictures are used by no skill, masks and text on a path only by logos, and shadow
spread and shader effects are taught nowhere.

### Cited Findings
- Node kinds: frame, group, rectangle, ellipse, polygon, path, connector, text, note, icon, script, browser, ref (+ older icon_font, line) — `MECHANICS.md:11`.
- Connectors (from/to a layer id, side, `at`, free points, `via` waypoints, routes curved/elbow/straight, `dash`, markers arrow/bar/triangle/dot/diamond) — `MECHANICS.md:12`, `app/crates/pen-doc/src/connector.rs:1-5`; taught in GUIDE as "a wire in a node editor, an arrow in a flow chart" (`agent.rs:41-62`). Skills: 0 mentions.
- Component states — `MECHANICS.md:13`, `agent.rs:154-159`. Skills: 0 uses.
- Pattern paint (dots, grid, lines; spacing, size, direction, offset; at most 20,000 marks) — `MECHANICS.md:15`, `app/src/render/pattern.rs:1-5`; GUIDE `agent.rs:113-118` ("the dot grid of a canvas or editor, graph paper, a ruled sheet"). ui-components and devices: 0 mentions of "pattern"; liquid-glass 2 and logos 3 word hits (logos has a "Grid, pattern and isometric" style, `logos/references/styles.md:171`; whether they use the paint was not checked).
- Masks — `MECHANICS.md:190`, GUIDE `agent.rs:91-97`; taught in a skill only by logos (`logos/references/detail.md:48`).
- Text on a circle and along a line — `MECHANICS.md:187-188`, GUIDE `agent.rs:68-77`; only logos (`detail.md:61,78`; 7 `onCircle` and 3 `onPath` hits).
- Text outline (stroke on text, stacked outlines) and letters' ink box — `MECHANICS.md:189,191`, GUIDE `agent.rs:78-82`.
- Shadow spread on drop and inner shadows — `MECHANICS.md:71-72`; absent from GUIDE's Effects (`agent.rs:122-125`); "spread" hits: logos 1, ui-components 2 (both saying it does not exist).
- Shader effects (WebGL 1.0 fragment shader file per layer, `pencil:object`, `pencil:background`, uniforms) — `MECHANICS.md:76-77`; the word "shader" does not occur in GUIDE (`agent.rs:21-220`, read in full).
- Gradients linear, radial, angular; image fills with cover/contain/stretch — `agent.rs:110-113`. Ellipse rings and arcs ("a spinner, a progress ring") — `agent.rs:37-38`. Polygon with corner radius — `agent.rs:39`, `MECHANICS.md:111`.
- Paths: SVG path data with move, line, quadratic and further segments, `viewBox`, `fillRule` — `app/crates/pen-doc/src/path.rs:1-12`, `agent.rs:40`.
- Materials frosted and liquid glass, glass groups — `MECHANICS.md:79-82`; taught by liquid-glass.
- Theme axes of any name (not only Mode) — `agent.rs:142-145`; ui-components uses only `Mode` (`SKILL.md:52-54`), liquid-glass has its own variables (`liquid-glass/references/tokens.md:6`).
- Pictures: `generate_image` (seven aspects, normal/large, transparent on OpenAI only) and `edit_image` (Gemini only) — `MECHANICS.md:229,235-236`, `agent.rs:162-179,321-330`. Web: `search_web`, `read_web_page`, `search_images`, `download_image` (DuckDuckGo, Openverse, Wikimedia Commons, optional Pexels and Pixabay keys) — `agent.rs:181-188`, `app/AI.md:192-214`. Skills: 0 mentions of `generate_image` or `search_images`.
- `ask_user`: the assistant can ask a question with tickable answers — tool list in `agent.rs`, `MECHANICS.md:301`, `CHANGELOG.md:300`.
- Icons: Lucide only (all of it), Feather names mapped — `MECHANICS.md:227`, `agent.rs:83-87`.
- Export: PNG, JPEG, WebP, SVG and a vector PDF with real text and embedded fonts, one page sized to the layers — `MECHANICS.md:181-185`. Bundles (`.pencraft`) — `MECHANICS.md:19`.
- Model fields that exist but GUIDE does not name: `context`, `flipX`/`flipY` — `model.rs:233,248-251`; `locked` — `MECHANICS.md:127`.
- "Not in this version" (verbatim list): chips for skills, files and web elements; chat modes; Codex and OpenAI-style providers as the chat's assistant; Markdown in the assistant's words; the generating effect on placeholder frames; live browser nodes; scripts; shader fills; in shader effects `@time`, `@mouse`, `@sdf`, `@mipmap`; effects' blend modes; pictures from web addresses; icons from libraries other than Lucide and Feather (Material Symbols, Phosphor); icon `weight`; script layers; Figma import; snapshots; copy-as and code export; SVG groups, transforms and per-shape colours on import; comments; opening files from the Finder on a Mac; group scaling inside a layout — `MECHANICS.md:305-310`.
- Further limits stated elsewhere: no tabular figures, no ellipsis truncation, no dashed shape strokes, no motion (`ui-components/SKILL.md:190-200`); PDF is one page (`MECHANICS.md:185`); effects are CPU-drawn pictures (`MECHANICS.md:78`).

### Inferences
- Engine features with a guide entry but no skill map directly onto absent skills: connectors + pattern grid -> diagrams/flow charts/node editors; arcs, polygons, paths, pattern lines -> charts and data graphics; masks + patterns + gradients + text on a path -> illustration and poster work; `generate_image`/`search_images` + image fills + masks -> photo-led pages (hero, product, editorial); vector PDF with embedded fonts -> print pieces (but single page, no bleed or page-size concept seen).
- "Material Symbols, Phosphor" being unavailable (`MECHANICS.md:308`) limits a faithful Material 3 skill to Lucide stand-ins; shadcn styles that use Hugeicons or Tabler have the same limit (`ui-components/references/styles.md:27`).
- Multi-page output (a slide deck, a carousel) has to be several top-level frames side by side (`agent.rs:204-209`) exported one file per selection (`MECHANICS.md:193`); there is no page or presentation concept ("presentation" is among left-out actions, `MECHANICS.md:175`).

### Gaps
- `AI.md` and `CHANGELOG.md` were read by section headings and targeted lines, `model.rs` by its field list, `connector.rs`, `path.rs` and `pattern.rs` by their module headers only; MECHANICS and GUIDE were read in full and are the basis of this section.
- Whether shapes can carry several strokes of different widths, and whether `oklch()` is accepted, was not verified.

## 5. Worlds, grounds, lead faces, and what the test folders show

### Takeaway
The 24 worlds are all printed or physical artefacts (timetable, poster, chart, label), dealt only for look-of-its-own
requests; tested design kinds are nine general cases, five glass scenes and four component scenes, with no recorded
fresh-agent tests for logos or devices.

### Cited Findings
- 24 worlds: railway timetable; Swiss exhibition poster; field guide to birds; nautical chart; 1960s record sleeve; broadsheet front page; engineering drawing; museum wall label; airport wayfinding sign; 1970s paperback cover; laboratory notebook; cookbook page; concert handbill; bank note; seed packet; ski resort trail map; letterpress business card; control panel; film title card; atlas index; market stall price board; library catalogue card; mid-century annual report; weather map — `app/src/craft.rs:221-246`.
- 5 grounds: light warm paper; dark with light type; one strong colour as ground of the main part; cool pale with one deep ink; white with colour only in large blocks — `craft.rs:250-256`.
- 6 lead faces: serif with character; condensed sans; plain grotesque in two far-apart weights; slab or typewriter; wide or geometric sans; humanist sans with a serif for reading — `craft.rs:258-265`.
- Dealing: three worlds by seed (`seed + step*7 mod 24`), one ground (`seed/3 mod 5`), one lead (`seed/5 mod 6`), only on the first message, not for a logo or a named reference — `craft.rs:274-285`, `chat.rs:1019-1022`. Ground and face bind only "for something that has to win a person over" (`craft.rs:278`).
- Banned defaults that bound the worlds: violet gradients, three equal cards, centred hero, spaced-caps eyebrow, Inter/Roboto/Arial by default, ten "next default" faces and four "next default" looks — `craft.rs:168-184`.
- `checks/design-review/`: nine pictures and `.pen` files — 1 coffee landing, 2 habit tracker, 3 repair cafe poster, 4 product page, 5 banking home, 6 bakery dashboard, 7 settings screen, 8 pricing page, 9 weather screen — plus three banking runs (run-a/b/c) (directory listing of `checks/design-review` and `checks/design-review/pens`, `banking-runs`). `checks/round-0` and `round-1` hold one screenshot each.
- `docs/liquid-glass/tests/`: prompts 1, 1b, 2, 3, 3b, 4, 5 and renders named photo feed (and rerun), dark feed, light feed, media player (and first run), desktop window, accessibility looks; harness `pc.py`, `start.sh`, `stop.sh` (directory listing).
- `docs/ui-components/tests/README.md` ("The tests"): 1 a settings dialog "in shadcn style" over a dimmed page; 2 a state sheet for buttons (five variants, five states, light and dark); 3 a command palette "like Linear or Raycast but plain" in three moments; 4 a dark phone screen with a list, a bottom drawer with a select and a note, and a toast; 2b and 4b are second rounds; "These last fixes have not been tried by a third round."
- Known open frictions from those tests: `edit_layers` boxes a few units off `read_layers`; `screenshot` shrinks silently; text faded by `opacity` is not measured; no tool lists icon names — `docs/ui-components/tests/README.md`, "Not fixed, and worth knowing".
- `docs/` holds only `liquid-glass`, `ui-components`, `superpowers`, BUILDING.md, RELEASING.md: no test folder for logos or devices (directory listing). Their coverage is unit tests: device template and every iPhone row (`skills.rs:378-486`), bare-screen note (`review.rs:1894`), logo counts (`review.rs:1551`), logo word match (`craft.rs:482-499`).

### Inferences
- Kinds tried at least once: landing page, poster, product page, pricing page, mobile app home/tracker/weather, dashboard, settings screen, glass phone feeds, glass media player, glass desktop window, shadcn dialog, state sheet, command palette, phone drawer. Kinds with no test or check case: logo (fresh-agent), illustration, chart or data graphic, diagram/flow chart, slide, social graphic, icon set, email, document page, tablet or Android screen, a Material or other named-system screen, a multi-screen flow.
- The worlds give print-derived looks; none is a digital-native idiom (an operating system, a game HUD, a terminal is even named as a costume in `craft.rs:50-51`), and none addresses illustration styles, so a later "worlds" extension or an illustration skill would not collide with them.

### Gaps
- The prompt files themselves (`prompts/*.md`) and the pictures were not opened; scenario descriptions come from the README and file names. The liquid-glass `README.md`, `NOTES.md` and `TASK.md` were not read. `.pen` files were not opened, as instructed.

## 6. How a skill is added and tested

### Takeaway
A skill is a folder compiled into the binary and listed in one constant; four generic tests bind its format, and the
description is the only trigger text, sent with every chat brief, so new skills must be few, sharply bounded and
under 1,024 characters of description and 500 lines of SKILL.md.

### Cited Findings
- Layout: `app/skills/<name>/SKILL.md` with `name` and `description` front matter and reference files beside it; registered by adding a `Skill { name, files: &[(MAIN, include_str!(..)), ("references/x.md", include_str!(..))] }` entry to `ALL` — `app/src/skills.rs:1-6,22-63`.
- Front matter in use: `name`, `description`, `license: Apache-2.0 with Commons Clause`, `metadata: product: pencraft, version` — `app/skills/ui-components/SKILL.md:1-8`. `front()` reads single-line `key: value` only — `skills.rs:67-71`.
- The index sentence and one line per skill (`- \`name\`: description`) go into `get_guide` and into the chat brief — `skills.rs:99-112`, `craft.rs:399-401`, `chat.rs:98`.
- Tools: `list_skills` (name, description, files) and `read_skill(name, file?)` with helpful errors — `skills.rs:114-133`, `agent.rs:241-252`. CLI: `pencraft skills`, `show NAME [FILE]`, `install [--project DIR | --dir DIR]` writing to `.agents/skills` and `.claude/skills` — `skills.rs:19-20,171-225`, `app/AI.md:300-332`.
- Format test: front matter opens the file; name equals folder, lowercase letters/digits/single hyphens, at most 64; description 40 to 1,024 characters with no `<` or `>`; main file under 500 lines; every backticked `references/*.md` named in any file must exist; every file must be named in SKILL.md — `skills.rs:255-281`.
- Layer test: every fenced `json` block that is one layer (no `...`, no `<`, one top-level object with a `type`) must deserialize as a `Node`, with valid material and glass-group fields; more than 50 layers overall — `skills.rs:283-317`.
- Per-skill tests: ui-components' tokens plus every block must load as one document, resolve all refs, hold at least 20 reusable components and produce no review mistakes and no advice in Light and in Dark — `skills.rs:319-376`; devices template and every iPhone row must pass `device_note` and `mistakes_in`, with status-bar positions checked — `skills.rs:378-486`.
- A hard-coded expectation must be edited when a skill is added: install order `["liquid-glass", "ui-components", "devices", "logos"]` and `list()["skills"][0]` — `skills.rs:488-505`.
- Other places a skill is wired in today: word lists and per-message notes for references, devices and logos (`craft.rs:289-381`, `chat.rs:1019-1028`); CRAFT sections that send the model to a skill (`craft.rs:133-163`); review switches by request (`app/src/app/tools.rs:614-650`); critic classes (`critic.rs:34-44`); the skills table in `app/AI.md:300-309`; MECHANICS "Skills" row (`MECHANICS.md:264`).
- Conventions the existing skills follow: decision-first SKILL.md with numbered sections, hard rules, a workflow, a checklist and a "when it looks wrong" table; references with ready JSON and stable id prefixes (`ui-`); one tokens file with a `set_variables` call — `ui-components/SKILL.md:20-31,161-213`, `liquid-glass/SKILL.md:190,208`; the plan's authoring advice at `reports/UI component library lessons.md:178`.
- Fresh-agent harness: start a private Pencraft, give a prompt file to a new agent, collect its friction report — `docs/ui-components/tests/README.md` ("Running one").

### Inferences
- Every description is paid for in every conversation (index in the brief, `chat.rs:98`), and descriptions are the only routing signal for MCP clients; skills with a deterministic trigger (devices, logos, named references) additionally get a word list and an injected note. A new skill for, say, charts or diagrams would fit that pattern (word list + note + optional review switch).
- A new skill that ships JSON should get a `document_of`-style test like ui-components' (`skills.rs:319-376`); the helper is already generic over the skill and its tokens file.
- Adding reference files to ui-components (navigation, dates, data, compositions) costs no index tokens and is automatically covered by the light/dark review test, which makes "more reference files" cheaper than "more skills" for standard components.

### Gaps
- Current description lengths in characters were not measured (logos' is the longest and looks close to the 1,024 limit).
- `scripts/check.sh` and how the skill tests run in CI were not looked at.
