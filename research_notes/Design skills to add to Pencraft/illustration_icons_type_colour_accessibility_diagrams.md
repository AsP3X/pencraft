# Foundational craft knowledge beyond components: icons, illustration, type, colour, accessibility, diagrams (state: November 2026)

Reading notes for the writer:
- Everything under "Cited Findings" was read from the linked page (or from a search-result summary of it, flagged "via search") during this session (20 Nov 2026).
- Everything I could not source this session is in "Gaps". Where I know the commonly quoted value from training knowledge, I list it there, explicitly labelled UNVERIFIED, so it is not mistaken for a sourced fact.
- Several official pages are JavaScript-rendered and returned no body text to the fetch tool: m3.material.io (icons, type scale, colour), developer.apple.com HIG (SF Symbols, typography), fonts.google.com/knowledge, ibm.com/design/language. Their numbers are therefore sourced from older static mirrors or secondary pages, or left in Gaps.

---

## 1. Interface icon design: grids, keylines, stroke, extending a set, common faults

### Takeaway
The outline icon sets a model is most likely to sit beside (Lucide, Tabler) share one recipe: 24x24 canvas, 2 px centred stroke, round caps and joins, at least 1 px edge padding, 2 px corner radius, 2 px minimum gap between elements; Material differs (20 dp live area, squared terminals, variable axes) and Heroicons uses a 1.5 px stroke. A new icon matches a set when it copies those numbers exactly, is sized against the set's keyline shapes (square smaller than circle), and passes a "same visual weight as the set's circle and square" comparison.

### Cited Findings

**Lucide (the set Pencraft ships)**
- Canvas 24x24 px; at least 1 px padding between strokes and canvas edge (so a 22x22 live area) — [Lucide icon design guide](https://lucide.dev/guide/design/icon-design-guide)
- Stroke width 2 px, never mixed; stroke centred on the path (no inside/outside strokes); round joins only (miter and bevel not allowed); round caps on open paths (butt and square not allowed) — [Lucide icon design guide](https://lucide.dev/guide/design/icon-design-guide)
- Corner radius: 2 px on 90-degree corners of shapes at least 8 px wide or tall; 1 px on shapes smaller than 8 px; 2.41 px (1+sqrt 2) where diagonal lines meet at a right angle; keep corners sharp where more than two lines meet — [Lucide icon design guide](https://lucide.dev/guide/design/icon-design-guide)
- Spacing: at least 2 px between distinct elements and, where possible, inside shapes; test: a 2-px-wide circle must fit in the gap without overlapping — [Lucide icon design guide](https://lucide.dev/guide/design/icon-design-guide)
- Visual weight must match the set's `circle` and `square` icons; blur test: the icon should not look much lighter or darker than the reference; dark blobs under blur mean too much detail (no numeric density limit given) — [Lucide icon design guide](https://lucide.dev/guide/design/icon-design-guide)
- Optical balance: icons visually centred; symmetrical icons stay geometrically centred; compare side by side and stacked with `circle`/`square` — [Lucide icon design guide](https://lucide.dev/guide/design/icon-design-guide)
- Curves: prefer simple arcs and quadratic curves; align cubic control points; avoid extra control points. Pixel alignment: align coordinates, arc centres and sub-elements to the pixel grid, including diagonals where possible — [Lucide icon design guide](https://lucide.dev/guide/design/icon-design-guide)
- Reuse: keep a base icon's geometry, placement and orientation unchanged when deriving variants; add-ons (badges, plus, check) keep consistent placement and size — [Lucide icon design guide](https://lucide.dev/guide/design/icon-design-guide)

**Tabler**
- 24x24 canvas; minimum 1 px padding; 2 px stroke centred on path; round caps and joins; 2 px corner radius on rectangles and similar shapes; 2 px space between separate elements — [Tabler icon design guide](https://docs.tabler.io/icons/design-guide.md)
- Start from four keyline shapes (circle, square, wide rectangle, tall rectangle) to judge optical size (no numeric sizes published) — [Tabler icon design guide](https://docs.tabler.io/icons/design-guide.md)
- Accessibility rule: use shapes with clear contrast and avoid icons that differ only in small details — [Tabler icon design guide](https://docs.tabler.io/icons/design-guide.md)
- Variant conventions: `-off` is a slash crossing the whole icon; `-plus`, `-check` suffix marks sit in the bottom-right corner where the base shape leaves room; logos prefixed `brand-`; main object first in the name (`user-plus`) — [Tabler icon design guide](https://docs.tabler.io/icons/design-guide.md)
- SVG output: `viewBox 0 0 24 24`, `fill="none"`, `stroke="currentColor"`, `stroke-width="2"`, round linecap and linejoin; every shape a path; no groups, transforms, masks, clip paths — [Tabler icon design guide](https://docs.tabler.io/icons/design-guide.md)

**Material (system icons / Material Symbols)**
- Material Symbols variable axes: FILL 0–1 (default 0), wght 100–700 (default 400), GRAD -50 to 200 (default 0), opsz 20–48 dp; optical size adjusts stroke weight as the icon is scaled so it looks consistent across sizes; 24 px is called the preferred icon size; three styles exist (the page names Outlined) — [Google Fonts Material Symbols guide](https://developers.google.com/fonts/docs/material_symbols)
- Classic Material system-icon grid (Material 1 page, static mirror; the M3 page could not be read): icon 24 dp; live area 20x20 dp with padding around it; dense variant 20 dp icon with 16x16 dp live area — [Material Design 1, Style: Icons](https://m1.material.io/style/icons.html)
- Keyline shapes: square 18x18 dp, circle 20 dp diameter, vertical rectangle 16x20 dp, horizontal rectangle 20x16 dp — [Material Design 1, Style: Icons](https://m1.material.io/style/icons.html)
- Stroke 2 dp for all instances (curves, angles, interior, exterior); 1.5 dp allowed only as an optical exception in complex icons; exterior silhouette corner radius 2 dp; interior corners square; do not round strokes of 2 dp width or less; stroke terminals squared (not round) — [Material Design 1, Style: Icons](https://m1.material.io/style/icons.html)
- Place icons on whole-pixel coordinates; touch target 48 dp for a 24 dp icon, 40 dp for a dense 20 dp icon — [Material Design 1, Style: Icons](https://m1.material.io/style/icons.html)
- Padding conflict: the M1 mirror was summarised as "4 dp padding each side" for a 20 dp live area in 24 dp, which is arithmetically wrong (24 - 20 = 2 dp each side); a search summary of the M2 page states 2 dp padding around the perimeter and that content may extend into the padding for extra weight but never past the trim area — [Material Design 1, Style: Icons](https://m1.material.io/style/icons.html); contradicted/corrected by [Material 2 system icons (via search)](https://material.io/design/iconography/system-icons.html)

**Heroicons**
- 316 icons; Outline style is 24x24 with a 1.5 px stroke — [heroicons.com](https://heroicons.com/)
- Four sets by path: `/24/outline`, `/24/solid`, `/20/solid`, `/16/solid`; MIT licence; icons take colour from CSS `color` (currentColor) — [Heroicons GitHub README](https://github.com/tailwindlabs/heroicons)

**Phosphor**
- Six weights: thin, light, regular, bold, fill, duotone; custom icons are designed on a 256x256 grid; duotone uses a second layer at reduced opacity (0.2 in the README example) — [Phosphor React README (via search)](https://www.github.com/phosphor-icons/react)
- Described as "designed at 16 x 16px to read well small and scale up big" — [Phosphor mirror description (via search)](https://sourceforge.net/mirror/phosphor-icons/)

**Carbon (IBM)**
- Artboards 16, 20, 24 and 32 px; 16 px is the typical size inside components; 16 and 20 px icons pair with 14 and 16 px IBM Plex — [Carbon icons usage](https://carbondesignsystem.com/elements/icons/usage/)
- Interactive icons need a touch target "44px or larger" (padding added around the glyph); icons must meet 4.5:1 contrast, the same as type — [Carbon icons usage](https://carbondesignsystem.com/elements/icons/usage/)

**SF Symbols**
- SF Symbols 7: over 6,900 symbols; new Draw On / Draw Off animation presets, Variable Draw, improved Magic Replace, automatic linear gradients generated from a single source colour; introduced at WWDC 2025 for the "26" OS releases — [Apple SF Symbols page (via search)](https://developer.apple.com/sf-symbols); [WWDC25 session 337](https://developer.apple.com/videos/play/wwdc2025/337/)

**What changed recently**
- Material Symbols (variable font with fill/weight/grade/optical-size axes) is the current Google set, replacing fixed-style Material Icons — [Google Fonts Material Symbols guide](https://developers.google.com/fonts/docs/material_symbols)
- SF Symbols 7 (2025) added gradients and draw animations — [WWDC25 session 337](https://developer.apple.com/videos/play/wwdc2025/337/)

### Inferences
- A model-facing "icon-sets" skill can be reduced to a checkable spec because Lucide and Tabler agree on nearly every number: 24 canvas, 1 px padding, 2 px centred stroke, round cap/join, 2 px radius (1 px under 8 px), 2 px minimum gap. Pencraft could count violations automatically: stroke width not equal to the set value, mixed stroke widths, any point outside the 1..23 range, non-round caps/joins, gaps under 2 px, off-grid coordinates (not on 0.5 px multiples).
- Because Pencraft uses Lucide, a custom icon drawn next to Lucide icons should use Lucide's numbers, not Material's (squared terminals, 20 dp live area) or Heroicons' (1.5 px stroke): mixing families is the most visible inconsistency.
- Keyline logic generalises: a square should be drawn smaller than a circle (Material: 18 vs 20 of 24) so they look the same size; tall and wide rectangles use the full long dimension but a shorter short dimension (16x20). For a 24 px Lucide-style icon with 1 px padding the equivalent would be roughly circle 20–22, square 18, rectangles 16x20, but those exact Lucide keyline sizes are not published (see Gaps).
- When scaling a stroked icon the stroke must be re-set, not scaled: Material's optical-size axis and Carbon's separate 16/20/24/32 artboards both exist because a 2 px stroke at 24 px becomes too heavy at 16 px and too thin at 48 px if scaled geometrically.
- Filled/outline pairs: Material treats fill as an axis on the same skeleton (FILL 0–1), Tabler and Heroicons as separate sets. A sensible rule: the filled version keeps the same silhouette and keyline size as the outline one, and interior details become 2 px (or 1.5 px) negative-space cut-outs, which Pencraft can do with boolean subtraction.
- Likely faults in generated icons (inferred from the rules above, not from a study): inconsistent stroke widths within one icon; butt caps beside round caps; shapes touching the canvas edge; gaps under 2 px that fill in at small size; too much detail (fails the blur test); off-centre optical balance; squares drawn at the same bounding size as circles; fill-and-stroke mixed in one glyph; non-pixel-aligned straight lines that render blurry; too many control points producing lumpy curves; perspective or shading in a flat set.

### Gaps
- m3.material.io/styles/icons could not be read (JS-rendered); the current M3 wording of the grid/keyline rules and the M3 corner-radius value are unverified. Numbers above are from the M1 static mirror and a search summary of the M2 page.
- Apple HIG SF Symbols page could not be read. UNVERIFIED from training knowledge: 9 weights (ultralight to black), 3 scales (small, medium, large), rendering modes monochrome / hierarchical / palette / multicolor, variants outline / fill / slash / enclosed; custom symbols are made from an exported template.
- Heroicons names and purposes for the 20 px and 16 px sets ("Mini", "Micro"; for small buttons and tight UI) were not stated on the pages read: UNVERIFIED.
- Phosphor's actual stroke widths per weight on the 256 grid (UNVERIFIED commonly quoted: thin 8, light 12, regular 16, bold 24 units, i.e. 0.75 / 1.125 / 1.5 / 2.25 px at 24 px) were not confirmed.
- Carbon's page did not give stroke weight, padding, corner radius or keylines; those live in IBM's Figma library, not read.
- No published keyline dimensions for Lucide or Tabler.
- No empirical source on "common faults in AI-generated icons"; the fault list is inference.

---

## 2. Vector illustration from simple shapes: style families, construction, spot vs hero, empty states

### Takeaway
Design systems now treat illustration as a small set of functional types (spot, low-fidelity UI, ambient pattern, hero/collage for marketing) with a "first ask whether it is needed" rule; the flat, long-limbed "Corporate Memphis / Alegria" look (2017) is widely regarded as overused, with reporting of a move to 3D-like and more textured styles, but no single successor. Detailed numeric construction rules (palette counts, proportions, angles) are mostly not published by the official systems; treat those as house rules to be chosen, not as citable standards.

### Cited Findings

**Types and when to use (Atlassian, the most complete public guidance read)**
- Types: Spot (a single concept; for empty states, error states and celebrating big wins; colourful spots for emphasis, neutral spots for routine tasks), Low-fidelity UI (simplified UI compositions for onboarding, feature tours and complex workflows; basic shapes, sharp corners, mostly grey tones), Ambient pattern (background texture for messages or promotional areas), Collage (marketing only, not in product), Hero (light and dark banners shown) — [Atlassian Design: Illustrations](https://atlassian.design/foundations/illustrations)
- Principles: decide first whether an illustration is necessary; visuals serve the user's goal, not decoration; illustrations reinforce the copy rather than replace it; overuse adds cognitive load, so consider icons, copy or patterns first — [Atlassian Design: Illustrations](https://atlassian.design/foundations/illustrations)
- Do not place real screenshots inside product screens (confusing screen-within-screen effect); use low-fi UI illustration instead — [Atlassian Design: Illustrations](https://atlassian.design/foundations/illustrations)
- Hero banners are provided in light and dark versions — [Atlassian Design: Illustrations](https://atlassian.design/foundations/illustrations)

**Shopify Polaris**
- Empty-state illustration appears the first time a merchant opens an area, to explain what can be done there; spot illustrations are reserved for rare cases (drawing attention on a crowded page, explaining a technical concept); example of a literal metaphor: scissors cutting a coupon for "no discount codes" — [Polaris illustrations (via search; the page now redirects)](https://polaris.shopify.com/design/illustrations)
- Empty state component: for a whole page, list, table or chart with nothing in it, not small areas; only one primary call-to-action; illustrations are decorative and take an empty alt attribute; about 40 px of white space above the image inside a card, modal or navigation — [Polaris empty state (via search)](https://polaris.shopify.com/components/layout-and-structure/empty-state)
- polaris.shopify.com/design/illustrations returned a 301 redirect to shopify.dev/docs/api/polaris on 20 Nov 2026, and a community thread says the illustration guidelines were being reworked — [Shopify community thread (via search)](https://community.shopify.dev/t/illustration-guidelines-update/26288)

**Empty states (content rules)**
- Three purposes: communicate system status, provide learning cues (say what will appear here and how to fill it), provide direct pathways (a button or link that starts the key task) — [NN/g: Designing Empty States](https://www.nngroup.com/articles/empty-state-interface-design/)
- Don'ts: no totally blank area; no "no records" message while still loading (use a progress indicator); no vague instruction that names an action without saying where it is — [NN/g: Designing Empty States](https://www.nngroup.com/articles/empty-state-interface-design/)
- Size the illustration to the space; if space is limited use text only — [Carbon empty states pattern (via search)](https://carbondesignsystem.com/patterns/empty-states-pattern/)
- Only add an illustration if there is enough room; not on small elements such as tiles or toasts — [SAP Fiori: designing for empty states (via search)](https://www.sap.com/design-system/fiori-design-web/v1-148/foundations/best-practices/global-patterns/designing-for-empty-states)
- Use rich graphics sparingly and pick an illustration that literally matches the context (e.g. a clothing rack for no results) — [eBay Playbook: empty states (via search)](https://playbook.ebay.com/design-system/patterns/empty-states)

**Corporate Memphis and the backlash**
- Features: minimalist, solid bright flat colours, cartoon-like figures with distorted proportions (long elastic arms, small heads, disproportionate torsos), non-realistic skin tones such as purple or blue; an isometric variant exists — [Wikipedia: Corporate Memphis](https://en.wikipedia.org/wiki/Corporate_Memphis)
- Origin: Facebook's "Alegria" system, created by the agency Buck, 2017; stock libraries such as unDraw made it easy to adopt — [Wikipedia: Corporate Memphis](https://en.wikipedia.org/wiki/Corporate_Memphis)
- Criticism: called lazy ("simple shapes [and] untextured colours"), small palette and no depth, homogeneous; 2022 Fast Company reported a return to 3D-like skeuomorphism; 2023 Creative Bloq said its popularity had fallen through oversaturation and it had become parody — [Wikipedia: Corporate Memphis](https://en.wikipedia.org/wiki/Corporate_Memphis)
- "The corporate world is not yet ready to fully commit to a specific new illustration type" — [Webflow: After Corporate Memphis (via search)](https://webflow.com/blog/after-corporate-memphis)
- Counter-view: the pile-on reflects wider anxieties about commercial illustration, and included harassment of illustrators — [AIGA Eye on Design (via search)](https://eyeondesign.aiga.org/what-the-think-pieces-about-corporate-memphis-tell-us-about-the-state-of-illustration/)

### Inferences
- A workable Pencraft taxonomy from the Atlassian model: (a) spot (one concept, roughly icon-like, sits beside a heading and one button), (b) low-fi UI (grey boxes representing the product itself), (c) ambient pattern (repeatable background, which maps directly onto Pencraft's pattern fills), (d) hero (large, scene-level, marketing surfaces only). The logos skill exclusion leaves all four uncovered.
- A model should be told the gate first ("would an icon or better copy do?") and that an empty state needs heading + one-sentence explanation + one primary button regardless of whether an illustration is present; the illustration is decorative.
- Avoid the Alegria signature (tiny heads, rubber limbs, blue/purple skin, flat untextured fills) unless asked: it now reads as dated and generic. Depth cues that the criticism says were missing (one consistent light direction, a second tone per shape, subtle grain or gradient) are cheap to add with Pencraft's gradients, masks and shadows.
- Since official systems publish few numbers, a skill would have to set its own house rules; reasonable ones consistent with what was read: one concept per spot; palette = brand accent + its tint + its shade + one neutral + one optional contrast colour; one stroke weight or none; one light direction; detail level matched to render size (a spot at about 120–200 px cannot carry more detail than a large icon).
- Hero scenes with people, hands, faces, organic textures or realistic depth are where shape-built vector work by a model is weakest; that is the point to switch to Pencraft's generated raster picture, keeping vector for spots, low-fi UI and patterns. (Judgement, not sourced.)

### Gaps
- Could not read: IBM Design Language illustration pages (truncated), the current Shopify illustration guidance (redirect), Mailchimp, Dropbox, GitHub/Primer illustration guidelines, unDraw, Storyset, Humaaans. No facts from them are included.
- No citable source found this session for: a canonical list of style families; palette-size rules (the brief's "3–5 colours with tints"); isometric construction angles; character proportions; spot-illustration pixel sizes. UNVERIFIED practitioner knowledge, offered only as starting points: style families commonly named are flat geometric, monoline/outline, duotone, isometric (true isometric uses 30-degree axes; "2:1 pixel isometric" uses about 26.57 degrees), grainy/noise gradient, paper-cut layered with soft shadows, blob-and-plant spot scenes, 3D-like soft gradients ("clay"), and hand-drawn/sketch; realistic adult figures are about 7–8 heads tall while stylised product figures are often 4–6 heads; spot illustrations are commonly delivered around 120–240 px and heroes at 400 px and up.
- No data on how many companies have actually retired Alegria-style work, and no sourced "2026 dominant style".
- No source located on when to prefer generated raster over vector; the rule above is inference.

---

## 3. Typography: scales and roles, installed families on Windows 11 and macOS, pairing, micro-typography

### Takeaway
Citable anchors: body 15–25 px on screen (10–12 pt print), line spacing 120–145 %, line length 45–90 characters (Bringhurst: 45–75, ideal 66), caps tracked +5–12 %; platform ramps are small (Windows 11: nine styles from 12/16 to 68/92 using only Regular and Semibold; Material 3: fifteen roles in five groups). The installed-font inventories for Windows 11 and macOS Sequoia are documented by Microsoft and Apple and give enough range (optical-size serifs like Sitka, variable sans like Segoe UI Variable and Bahnschrift on Windows; Avenir Next, Helvetica Neue, Charter, Baskerville, Didot, Futura, Optima, Gill Sans on macOS) for real pairings without downloads.

### Cited Findings

**Body-text numbers (Butterick)**
- Body point size 10–12 pt in print, 15–25 px on the web; line spacing 120–145 % of point size; line length 45–90 characters including spaces — [Practical Typography: summary of key rules](https://practicaltypography.com/summary-of-key-rules.html)
- All caps only for less than one line; add 5–12 % letterspacing to all caps and small caps; never fake small caps; kerning always on — [Practical Typography: summary of key rules](https://practicaltypography.com/summary-of-key-rules.html)
- Bold or italic sparingly and never both; never underline (except possibly web links); centred text sparingly; one space between sentences — [Practical Typography: summary of key rules](https://practicaltypography.com/summary-of-key-rules.html)
- Paragraph separation: first-line indent of 1–4 times the point size OR 4–10 pt of space between paragraphs, not both; hyphenation always on with justified text — [Practical Typography: summary of key rules](https://practicaltypography.com/summary-of-key-rules.html)
- Curly quotes not straight; apostrophes curl downward; foot and inch marks stay straight; real hyphens, en and em dashes (never double hyphens); real ellipsis character; real trademark and copyright symbols; non-breaking space after paragraph and section marks — [Practical Typography: summary of key rules](https://practicaltypography.com/summary-of-key-rules.html)

**Line length (Bringhurst)**
- "Anything from 45 to 75 characters is widely regarded as a satisfactory length of line"; the 66-character line (letters and spaces) is regarded as ideal; for multiple columns 40–50 characters; one character averages about 0.5 em, so 33 em is about 66 characters — [The Elements of Typographic Style Applied to the Web, 2.1.2](http://webtypography.net/2.1.2)
- Windows guidance is narrower: keep to 50–60 letters per line; not fewer than 20 or more than 60 — [Microsoft Learn: Typography in Windows](https://learn.microsoft.com/en-us/windows/apps/design/style/typography)

**Modular scales**
- A modular scale is "a prearranged set of harmonious proportions" (Bringhurst): choose the body size at which the text face looks best (examples 18 px, 16 px), choose a ratio (example golden section 1:1.618), multiply and divide to get sizes; a double-stranded scale interleaves two starting numbers on the same ratio to fill gaps; not every value must come from the scale — [A List Apart: More Meaningful Typography (Tim Brown)](https://alistapart.com/article/more-meaningful-typography/)

**Windows 11 type ramp (Fluent)**
- Caption 12/16 (Small), Body 14/20 (Text), Body Strong 14/20 semibold, Body Large 18/24, Body Large Strong 18/24 semibold, Subtitle 20/28 (Display semibold), Title 28/36, Title Large 40/52, Display 68/92; sizes in effective pixels — [Microsoft Learn: Typography in Windows](https://learn.microsoft.com/en-us/windows/apps/design/style/typography)
- Use Regular for most text and Semibold for titles; Bold and Italic are not in the ramp (italic excluded as reducing legibility, particularly for people with dyslexia); minimums 12 px Regular, 14 px Semibold; sentence case for all UI text including titles; left-aligned by default, centre only in rare cases such as text under icons — [Microsoft Learn: Typography in Windows](https://learn.microsoft.com/en-us/windows/apps/design/style/typography)

**Material 3 type roles**
- Fifteen baseline styles: display, headline, title, body, label, each in large/medium/small; since Compose Material3 1.5.0 each has an "Emphasized" counterpart (30 styles in all) — [Android Developers: Compose Material3 Typography](https://developer.android.com/reference/kotlin/androidx/compose/material3/Typography)
- Role intent: headlines for short, important text or numerals (an expressive display face is allowed); titles for medium-emphasis shorter text; body for long-form (serif or sans); labelLarge for buttons, tabs, dialogs, cards; labelMedium/Small used sparingly to annotate imagery or introduce a headline — [Android Developers: Compose Material3 Typography](https://developer.android.com/reference/kotlin/androidx/compose/material3/Typography)
- Confirmed values: displayLarge 57/64, headlineLarge 32/40 — [Android Developers: Compose Material3 Typography (via search)](https://developer.android.com/reference/kotlin/androidx/compose/material3/Typography)
- Secondary (community) sources give body-large 16/24, weight 400, tracking 0.5; label-large 14/20, weight 500, tracking 0.1; display-large tracking -0.25; title-medium 16 px medium — [community M3 typography reference (via search)](https://skills.sh/shelbeely/shelbeely-agent-skills/material-design-3-typography); [Material 3 typography cheatsheet (via search)](https://astordev.github.io/articles/material-typography)

**Apple**
- macOS does not support Dynamic Type (fixed text styles); iOS/iPadOS have a Dynamic Type size table whose first row at xSmall is Large Title 31 pt with 38 pt leading — [Apple HIG Typography (via search)](https://developer.apple.com/design/human-interface-guidelines/typography.md)

**Fonts shipped with Windows 11 (Microsoft list; page dated April 2022, so later feature updates are not reflected)**
- Sans: Segoe UI (Light, Semilight, Regular, Semibold, Bold, Black, each with italic), Segoe UI Variable (Small / Text / Display optical sizes, each Light, Semilight, Regular, Semibold, Bold; new in 11), Bahnschrift (single variable file), Arial (4 styles) and Arial Black, Calibri (Light, Regular, Bold + italics), Candara and Corbel (Light, Regular, Bold + italics), Franklin Gothic Medium (+ italic), Tahoma (Regular, Bold), Trebuchet MS (4), Verdana (4), Impact, Microsoft Sans Serif, Lucida Sans Unicode — [Microsoft Learn: Windows 11 font list](https://learn.microsoft.com/en-us/typography/fonts/windows_11_font_list)
- Serif: Sitka (six optical sizes: Small, Text, Subheading, Heading, Display, Banner; each Regular, Semibold, Bold + italics; variable file; Semibold new in 11), Georgia (4), Cambria (4 + Cambria Math), Constantia (4), Palatino Linotype (4), Times New Roman (4), Sylfaen — [Microsoft Learn: Windows 11 font list](https://learn.microsoft.com/en-us/typography/fonts/windows_11_font_list)
- Mono: Cascadia Code and Cascadia Mono (ExtraLight, Light, SemiLight, Regular, SemiBold, Bold, each with italic; new in 11), Consolas (4), Courier New (4), Lucida Console — [Microsoft Learn: Windows 11 font list](https://learn.microsoft.com/en-us/typography/fonts/windows_11_font_list)
- Script/display: Gabriola, Ink Free, Segoe Print, Segoe Script, Comic Sans MS, MV Boli; icon fonts Segoe Fluent Icons (new in 11) and Segoe MDL2 Assets — [Microsoft Learn: Windows 11 font list](https://learn.microsoft.com/en-us/typography/fonts/windows_11_font_list)
- NOT installed by default (optional "Pan-European Supplemental Fonts" feature): Arial Nova, Georgia Pro, Gill Sans Nova, Neue Haas Grotesk Text Pro, Rockwell Nova, Verdana Pro, each with wide weight ranges and condensed cuts — [Microsoft Learn: Windows 11 font list](https://learn.microsoft.com/en-us/typography/fonts/windows_11_font_list)
- Segoe UI Variable axes: weight 100 (Thin) to 700 (Bold), named instances Light 300, Semilight 350, Regular 400, Semibold 600, Bold 700; optical size automatic, scaling from 8 pt to 36 pt; Bahnschrift has weight and width axes — [Microsoft Learn: Typography in Windows](https://learn.microsoft.com/en-us/windows/apps/design/style/typography)
- Aptos (the Microsoft 365 default since 2023) is a cloud font provisioned to Office apps, not installed as a Windows system font by default, so other applications cannot see it unless the separately downloadable Aptos package is installed locally — [Microsoft Support: Cloud fonts in Office (via search)](https://support.microsoft.com/en-US/Office/fonts/cloud-fonts-in-office); [Microsoft Download Center: Aptos (via search)](https://microsoft.com/en-us/download/details.aspx?id=106087)

**Fonts included with macOS (Apple list for macOS Sequoia; the page does not mark which are pre-installed versus download-on-demand, saying only that most are installed automatically and undownloaded ones are dimmed in Font Book)**
- Sans: Helvetica Neue (UltraLight, Thin, Light, Regular, Medium, Bold + italics, Condensed Bold/Black), Helvetica (Light, Regular, Bold + obliques), Avenir (Light, Book, Roman, Medium, Heavy, Black + obliques), Avenir Next (Ultra Light, Regular, Medium, Demi Bold, Bold, Heavy + italics, and Condensed versions of each), Gill Sans (Light, Regular, SemiBold, Bold + italics, UltraBold), Futura (Medium, Medium Italic, Bold, Condensed Medium, Condensed ExtraBold), Optima (Regular, Italic, Bold, Bold Italic, ExtraBlack), Lucida Grande, Arial family, Verdana, Tahoma, Trebuchet MS — [Apple Support: Fonts included with macOS Sequoia](https://support.apple.com/en-us/120414)
- Serif: Baskerville (Regular, Italic, SemiBold, Bold, Bold Italic), Charter (Roman, Bold, Black + italics), Didot (Regular, Italic, Bold), Bodoni 72 (Book, Italic, Bold; Oldstyle and Smallcaps variants), Cochin (4), Palatino (4), Georgia (4), Hoefler Text (incl. Black), Times New Roman (4), Rockwell, American Typewriter, Copperplate — [Apple Support: Fonts included with macOS Sequoia](https://support.apple.com/en-us/120414)
- Mono: Menlo (4), Monaco, Andale Mono, Courier New — [Apple Support: Fonts included with macOS Sequoia](https://support.apple.com/en-us/120414)
- Script/display: Snell Roundhand, Zapfino, Noteworthy, Marker Felt, Chalkboard, Papyrus, Apple Chancery, Skia — [Apple Support: Fonts included with macOS Sequoia](https://support.apple.com/en-us/120414)
- Downloadable commercial families listed by Apple: Proxima Nova, Graphik, Founders Grotesk, Canela, Druk, Produkt, Publico, Domaine Display — [Apple Support: Fonts included with macOS Sequoia](https://support.apple.com/en-us/120414)
- "Document-support" fonts (e.g. Bebas Neue, Seravek, Times, Garamond) are available only to documents that already use them or apps that request them by name, so are not reliably selectable — [Apple Support: Fonts included with macOS Sequoia](https://support.apple.com/en-us/120414)
- Cross-platform overlap visible in both lists: Arial, Georgia, Times New Roman, Verdana, Trebuchet MS, Tahoma, Courier New, Comic Sans MS, Impact (Palatino on macOS vs Palatino Linotype on Windows) — [Microsoft Learn: Windows 11 font list](https://learn.microsoft.com/en-us/typography/fonts/windows_11_font_list); [Apple Support: Fonts included with macOS Sequoia](https://support.apple.com/en-us/120414)

**Quality ranking of system fonts (Butterick; a print/professional-document view, opinionated)**
- A list, "generally tolerable" and plausible for body text: Avenir, Charter, Gill Sans, Helvetica, Helvetica Neue, Hoefler Text, Iowan Old Style, Optima, Palatino, Seravek, Sitka, Athelas, Franklin Gothic, Garamond, Century Schoolbook, Book Antiqua, and others — [Practical Typography: system fonts](https://practicaltypography.com/system-fonts.html)
- B list, fine in limited doses: Segoe UI, Calibri, Candara, Corbel, Constantia, Futura, Rockwell, Bodoni 72, Big Caslon — [Practical Typography: system fonts](https://practicaltypography.com/system-fonts.html)
- C list, questionable: Georgia (screen-optimised, clunky in print), Baskerville, Cambria, Didot, Cochin, Consolas, Menlo, Times New Roman (overexposed), Century Gothic — [Practical Typography: system fonts](https://practicaltypography.com/system-fonts.html)
- F list: Arial, Verdana, Tahoma, Comic Sans MS, Papyrus, Marker Felt, Segoe Print/Script; he recommends Avenir and Gill Sans as alternatives to Arial and warns Helvetica is overused for distinctive work — [Practical Typography: system fonts](https://practicaltypography.com/system-fonts.html)

**Popular free families**
- Third-party lists based on Google's analytics put Roboto first and Open Sans second, with Lato, Montserrat, Inter and Poppins in the top ten; Roboto Condensed and Roboto Slab also rank; no dated 2026 ranking was found — [Kinsta: best Google Fonts (via search)](https://kinsta.com/blog/best-google-fonts/); [Speckyboy: most popular Google Fonts (via search)](https://speckyboy.com/most-popular-google-fonts/)
- Google Fonts passed 1,800 families — [WhatFontIs blog (via search)](https://www.whatfontis.com/blog/google-fonts-hits-1800-typefaces-what-this-milestone-means-for-designers-in-2026/)

**Quotation marks per language**
- English US “…” then ‘…’; UK ‘…’ then “…”; German „…“ then ‚…‘; French « … » then ‹ … › with a narrow no-break space (U+202F) inside the guillemets; Spanish (Spain) and Italian «…» then “…”; Portuguese (Portugal) «…», Brazil “…”; Dutch “…” (alternative „…”); Polish „…” then »…«; Russian «…» then „…“; Swedish ”…” then ’…’; Danish »…« (alternatives ”…”); Japanese 「…」 then 『…』; Simplified Chinese “…”, Traditional 「…」 — [Wikipedia: Quotation mark](https://en.wikipedia.org/wiki/Quotation_mark)

### Inferences
- The guide's "at most two families, five to seven sizes" is consistent with the platform ramps read: Windows uses seven distinct sizes (12, 14, 18, 20, 28, 40, 68) and two weights; M3's fifteen roles are a ceiling for a whole product, not for one screen. A "type" skill can keep the limit and add named roles: caption, body, body-strong, subtitle, title, title-large, display, with size/line-height pairs.
- Line-height pattern readable from the Windows ramp: about 1.33–1.43 for 12–20 px text, about 1.29–1.35 for 28–68 px. Together with Butterick's 120–145 % this supports a rule a model can apply: body 1.4–1.5, headings 1.1–1.3, tighter as size grows.
- A font-fallback table per OS is feasible from the two official lists. Pairings that use only stock fonts and respect Butterick's ranking (pairings are my suggestions, the availability is cited): Windows — Sitka Heading/Display + Segoe UI Variable Text; Segoe UI Variable Display + Sitka Text; Bahnschrift (condensed widths for headings/labels) + Segoe UI; Georgia + Segoe UI for web-like editorial; Cascadia Mono for code/data. macOS — Avenir Next + Charter; Helvetica Neue + Baskerville or Hoefler Text; Didot or Bodoni 72 display + Avenir Next body; Futura headings + Charter or Palatino body; Optima or Gill Sans as humanist single-family choices; Menlo for code.
- Cross-platform safe choices (present on both lists) are Georgia, Arial, Verdana, Trebuchet MS, Times New Roman, Courier New, Palatino(/Linotype): only Georgia and Palatino escape Butterick's F list, so documents meant to open on both systems are best set in Georgia or Palatino plus a platform sans with a declared fallback.
- Gill Sans, Rockwell and Neue Haas Grotesk exist on Windows only as the optional "Nova/Pro" supplemental feature, and Aptos only if the user installed it: the tool should check the installed list before offering them.
- Using the optical-size cuts correctly is an easy quality win on Windows: Sitka Small/Text for body, Subheading/Heading/Display/Banner as size grows; Segoe UI Variable Small under about 12 px, Text for body, Display from about 20 px (the Windows ramp switches to Display at 20 px Subtitle).
- Tracking rule that is fully sourced: +5–12 % on all caps and small caps. Negative tracking on large display sizes and positive tracking on small labels is consistent with the secondary M3 values (-0.25 at 57 px, +0.5 at 16 px, +0.1 at 14 px) but those are not officially confirmed here.
- Scale ratios for a model: pick a base (14–16 px UI, 16–20 px reading) and one ratio; the golden section (1.618) gives too few usable steps for UI, which is why double-stranded scales or smaller ratios are used (the smaller named ratios are in Gaps as unverified).

### Gaps
- m3.material.io type-scale token page unreadable; the full official 15-role table is UNVERIFIED. Commonly quoted baseline (training knowledge): display 57/64, 45/52, 36/44; headline 32/40, 28/36, 24/32; title 22/28, 16/24 (medium, +0.15), 14/20 (medium, +0.1); body 16/24 (+0.5), 14/20 (+0.25), 12/16 (+0.4); label 14/20 (medium, +0.1), 12/16 (medium, +0.5), 11/16 (medium, +0.5).
- Apple HIG typography page unreadable. UNVERIFIED commonly quoted values: iOS default body 17 pt, minimum 11 pt; macOS default 13 pt, minimum 10 pt; iOS Large size table Large Title 34/41, Title 1 28/34, Title 2 22/28, Title 3 20/25, Headline 17/22 semibold, Body 17/22, Callout 16/21, Subheadline 15/20, Footnote 13/18, Caption 1 12/16, Caption 2 11/13; macOS Large Title 26, Title 1 22, Title 2 17, Title 3 15, Headline 13 bold, Body 13, Callout 12, Subheadline 11, Footnote 10. SF Pro's size-specific tracking table was not retrieved.
- Whether SF Pro, SF Mono and New York appear as user-selectable families in a third-party app's font list on current macOS was not verified (they are not in Apple's "fonts included" list read; they are distributed separately on developer.apple.com/fonts under a licence limited to Apple-platform UI mock-ups — UNVERIFIED).
- The Microsoft font list is dated 2022; whether Windows 11 24H2/25H2 added or removed families was not found. Apple's list read is for Sequoia (macOS 15); a macOS 26 list was not checked.
- Google Fonts "Pairing typefaces" knowledge page unreadable; no cited pairing principles. UNVERIFIED standard principles: pair by contrast of structure (serif with sans) but similarity of proportion (x-height, width, stroke contrast); or stay inside one superfamily; avoid two faces of the same classification; assign roles (display vs text) rather than mixing at one level.
- Named modular ratios other than the golden section (minor third 1.2, major third 1.25, perfect fourth 1.333, augmented fourth 1.414, perfect fifth 1.5) are standard but not confirmed by a source read here.
- No official 2026 Google Fonts usage ranking retrieved (fonts.google.com analytics not read); order beyond "Roboto first, Open Sans second" is unverified.
- Not researched to a citable level: optical margin alignment and hanging punctuation, widow/orphan rules, numerals (tabular vs proportional, lining vs old-style), dash usage per language, Fonts In Use pairings, print-specific scales. UNVERIFIED standard practice: tabular lining figures in tables and anything that updates; old-style figures in running serif text; hang quotation marks and bullets outside the text edge in display settings; avoid a last line of one word and a single paragraph line stranded at the top or bottom of a column; en dash for ranges, spaced en dash (UK) or unspaced em dash (US) for parenthetical breaks, German uses spaced en dash (Halbgeviertstrich).

---

## 4. Colour systems: tonal ramps, roles from one brand colour, dark themes, contrast models

### Takeaway
Current practice builds each hue as an 11–12 step ramp in a perceptual space (Tailwind v4 publishes its palette in OKLCH; Radix assigns each of 12 steps a fixed job and guarantees APCA Lc 60/90 for steps 11/12 on step 2; Material derives five tonal palettes from one source colour in HCT). WCAG 2 ratios remain the only normative contrast test in November 2026: APCA was removed from the WCAG 3 draft in 2023, WCAG 3 has no chosen algorithm and is not expected to be final before about 2028–2030.

### Cited Findings

**Radix: one job per step (12 steps)**
- 1 app background; 2 subtle background (cards, sidebars, code blocks, striped tables); 3 component background (normal); 4 component hover; 5 component pressed/selected; 6 subtle borders and separators on non-interactive elements; 7 borders on interactive components; 8 stronger borders and focus rings; 9 solid backgrounds (purest, highest-chroma step); 10 hover for solid backgrounds; 11 low-contrast text; 12 high-contrast text — [Radix Colors: Understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale)
- Guarantees: step 11 reaches APCA Lc 60 and step 12 Lc 90 on a step 2 background of the same scale — [Radix Colors: Understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale)
- Most step 9 colours take white text; Sky, Mint, Lime, Yellow and Amber take dark text — [Radix Colors: Understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale)
- Neutral pairing: pure `gray` works with anything; otherwise choose the tinted grey closest in hue to the accent: mauve with tomato/red/ruby/crimson/pink/plum/purple/violet; slate with iris/indigo/blue/sky/cyan; sage with mint/teal/jade/green; olive with grass/lime; sand with yellow/amber/orange/brown — [Radix Colors: Composing a palette](https://www.radix-ui.com/colors/docs/palette-composition/composing-a-palette)
- Caution: saturated greys can clash with colourful components, especially as app backgrounds in dark mode — [Radix Colors: Composing a palette](https://www.radix-ui.com/colors/docs/palette-composition/composing-a-palette)
- Semantic hue families: error red/ruby/tomato/crimson; success green/teal/jade/grass/mint; warning yellow/amber/orange; info blue/indigo/sky/cyan — [Radix Colors: Composing a palette](https://www.radix-ui.com/colors/docs/palette-composition/composing-a-palette)
- Text from the accent scale reads more colourful (marketing); text from the grey scale reads more functional (apps) — [Radix Colors: Composing a palette](https://www.radix-ui.com/colors/docs/palette-composition/composing-a-palette)

**Tailwind CSS v4 palette (OKLCH)**
- 26 named hues plus black and white; 11 steps each (50, 100–900, 950); values defined in OKLCH and exposed as CSS variables; neutral-leaning families now number nine: slate, gray, zinc, neutral, stone, taupe, mauve, mist, olive — [Tailwind CSS docs: Colors](https://tailwindcss.com/docs/colors)
- Blue ramp (L% C H): 50 97/0.014/254.6; 100 93.2/0.032/255.6; 200 88.2/0.059/254.1; 300 80.9/0.105/251.8; 400 70.7/0.165/254.6; 500 62.3/0.214/259.8; 600 54.6/0.245/262.9; 700 48.8/0.243/264.4; 800 42.4/0.199/265.6; 900 37.9/0.146/265.5; 950 28.2/0.091/267.9 — [Tailwind CSS docs: Colors](https://tailwindcss.com/docs/colors)
- Pure neutral ramp lightness: 98.5, 97, 92.2, 87, 70.8, 55.6, 43.9, 37.1, 26.9, 20.5, 14.5 (chroma 0) — [Tailwind CSS docs: Colors](https://tailwindcss.com/docs/colors)
- Slate (cool tinted neutral): lightness 98.4 to 12.9 with chroma only 0.003–0.046 at hue about 248–266 — [Tailwind CSS docs: Colors](https://tailwindcss.com/docs/colors)

**OKLCH mechanics**
- `oklch(L C H)`: L 0–1 perceived lightness; C from 0 (grey), in practice always below 0.37 for sRGB and P3, with the true maximum depending on lightness and hue; H 0–360 (red about 20, yellow about 90, green about 140, blue about 220, purple about 320) — [Evil Martians: OKLCH in CSS](https://evilmartians.com/chronicles/oklch-in-css-why-quit-rgb-hsl)
- HSL lightness is not perceptual (equal `l` for blue and red look different), so HSL ramps give unpredictable contrast; in OKLCH changing only L gives predictable tints and shades — [Evil Martians: OKLCH in CSS](https://evilmartians.com/chronicles/oklch-in-css-why-quit-rgb-hsl)
- Not every L/C/H combination exists in sRGB; out-of-gamut colours are mapped by the renderer, either by clipping channels (fast, can shift hue) or by reducing chroma (preserves hue; the CSS Color 4 method); P3 holds roughly 30 % more colours than sRGB and cannot be written as hex; ship an sRGB value as default and P3 behind a gamut media query — [Evil Martians: OKLCH in CSS](https://evilmartians.com/chronicles/oklch-in-css-why-quit-rgb-hsl)
- Fixing L for a role (example 0.87 for light backgrounds) lets text colour be chosen without per-pair contrast checks; OKLCH was designed to fix LCH's hue shift in blues (hue about 270–330) — [Evil Martians: OKLCH in CSS](https://evilmartians.com/chronicles/oklch-in-css-why-quit-rgb-hsl)

**Material 3 (HCT)**
- HCT = hue, chroma, tone (tone is luminance; low is dark); one source colour yields five key colours (primary, secondary, tertiary, neutral, neutral variant), each a tonal palette; specific tones are slotted into roles and the same role names are used for light and dark; container/on-container pairs are tonal pairs with accessible contrast — [Android Developers: colour for mobile design (via search)](https://developer.android.com/design/ui/mobile/guides/styles/color)
- Tones at least 40 apart reach at least 3:1 contrast (the corresponding "50 apart gives 4.5:1" was not retrieved) — [Material blog / developer docs (via search)](https://material.io/blog/tone-based-surface-color-m3)
- 2023 change: surfaces became tone-based (five surface-container roles) instead of elevation overlays; default light surface moved from tone 99 to 98 and neutral-palette chroma rose from 4 to 6 — [Material blog: tone-based surface colour in M3 (via search)](https://material.io/blog/tone-based-surface-color-m3)
- A third-party implementation's defaults: light primary tone 40, primary container 90, on-surface-variant 30, surface variant 90; dark primary tone 80, primary container 30 — [flex_color_scheme tone defaults (via search)](https://github-redirect.dependabot.com/rydmike/flex_color_scheme/blob/ca7278657dd634e7896306ebaa968b95e8ced427/lib/src/flex_tones.dart)

**Contrast-first generation (Leonardo, Adobe)**
- Leonardo generates colours from target contrast ratios, so compliance is the starting point rather than a later audit; results often land slightly above the requested ratio because of RGB quantisation; its demos regenerate a whole UI palette from brightness and contrast controls ("adaptive" palettes) — [adobe/leonardo README](https://github.com/adobe/leonardo)

**APCA versus WCAG 2, and WCAG 3 status**
- APCA Lc levels: 90 preferred for body text (e.g. 14 px/400, 18 px/300); 75 minimum for body columns (18 px/400, 16 px/500, 14 px/700); 60 minimum for other content text (24 px/400, 18 px/600, 16 px/700); 45 minimum for large headlines (36 px/400, 24 px bold) and fine-detail pictograms or small outline icons; 30 absolute minimum for placeholder/disabled text and large solid icons; 15 minimum for non-text such as dividers; below 15 treat as invisible; +15 to each level approximates AAA — [APCA in a Nutshell](https://git.apcacontrast.com/documentation/APCA_in_a_Nutshell.html)
- APCA's own page says WCAG 2 contrast overstates contrast for dark colours and cannot guide dark-mode design, and describes APCA as a WCAG 3 candidate, also developed independently as the "APCA Readability Criterion" — [APCA in a Nutshell](https://git.apcacontrast.com/documentation/APCA_in_a_Nutshell.html)
- But: APCA was removed from the WCAG 3 working draft in July 2023; the April 2026 editor's draft marks the contrast test "Exploratory" and says the algorithm "is yet to be determined"; advice is to keep meeting WCAG 2 and, if APCA is also used, choose colours that pass both — [Adrian Roselli: WCAG3 Contrast as of April 2026](https://adrianroselli.com/2026/04/wcag3-contrast-as-of-april-2026.html); contradicts the "candidate" wording on [APCA in a Nutshell](https://git.apcacontrast.com/documentation/APCA_in_a_Nutshell.html)
- WCAG 3 is "an incomplete draft that will change", latest Working Draft September 2026 (page updated 16 Nov 2026); intended to become a standard "in a few years"; WCAG 2 will not be deprecated for several years after WCAG 3 is final; W3C recommends meeting WCAG 2.2 now — [W3C WAI: WCAG 3 Introduction](https://www.w3.org/WAI/standards-guidelines/wcag/wcag3-intro/)
- Timing estimates differ: candidate recommendation planned Q4 2027 (AbilityNet), "around 2030 at the earliest" (Roselli) — [AbilityNet (via search)](https://abilitynet.org.uk/resources/digital-accessibility/what-expect-wcag-30-web-content-accessibility-guidelines); [Adrian Roselli](https://adrianroselli.com/2026/04/wcag3-contrast-as-of-april-2026.html)

**Colour-blind-safe sets and 60-30-10**
- Okabe-Ito eight-colour qualitative palette: orange #E69F00, sky blue #56B4E9, bluish green #009E73, yellow #F0E442, blue #0072B2, vermillion #D55E00, reddish purple #CC79A7, black #000000; popularised by Bang Wong in Nature Methods (2011); described as distinguishable under protanopia, deuteranopia and tritanopia — [Okabe-Ito reference (via search)](https://conceptviz.app/blog/okabe-ito-palette-hex-codes-complete-reference); [NYU Siegal lab colour palette (via search)](https://siegal.bio.nyu.edu/color-palette/)
- About 8 % of men and 0.5 % of women have a colour-vision deficiency (secondary source) — [Okabe-Ito reference (via search)](https://conceptviz.app/blog/okabe-ito-palette-hex-codes-complete-reference)
- 60-30-10 comes from interior design, is presented by UI tutorials as "a guideline, not a hard rule"; no usability study was found supporting the split — [LogRocket: 60-30-10 rule (via search)](https://blog.logrocket.com/ux-design/60-30-10-rule/); [hype4 academy (via search)](https://hype4.academy/articles/design/60-30-10-rule-in-ui)

**What changed recently**
- Tailwind's palette is published in OKLCH (v4) and now includes extra tinted-neutral families taupe, mauve, mist and olive — [Tailwind CSS docs: Colors](https://tailwindcss.com/docs/colors)
- WCAG 3 drafts were republished in March and September 2026, still without a contrast algorithm — [W3C WAI news, March 2026 (via search)](https://www.w3.org/WAI/news/2026-03-03/wcag3); [W3C WAI: WCAG 3 Introduction](https://www.w3.org/WAI/standards-guidelines/wcag/wcag3-intro/)

### Inferences
- The guide's "six roles" can be extended to a step-per-job ramp without contradiction: Radix's 12 jobs collapse to backgrounds (1–2), component fills (3–5), borders (6–8), solid (9–10), text (11–12). A model can generate this from one brand colour: put the brand at step 9, keep hue, set lightness targets per step, and reduce chroma toward both ends (Tailwind's blue shows the shape: chroma peaks at 500–700 around L 49–62 % and falls to 0.014 at L 97 % and 0.091 at L 28 %; hue also drifts about 13 degrees toward violet as it darkens).
- Lightness targets that can be lifted directly from Tailwind's ramps for an 11-step scale: roughly 98, 96, 92, 87, 71, 56–62, 44–55, 37–49, 27–42, 21–38, 13–28 %; chromatic ramps stay lighter than neutrals in the dark half.
- "Greys tinted toward the dominant colour" can be made numeric: Tailwind's slate uses chroma 0.003–0.046 (a few percent of an accent's 0.2+) at the accent's hue; Radix gives the hue-matching table and the warning not to over-saturate dark-mode backgrounds.
- Dark theme derivation, consistent across Radix (same step jobs), Material (same role names, tones swapped, e.g. primary 40 to 80) and Tailwind's ramps: reuse the same roles and reverse the ramp rather than inverting colours; lower chroma on large dark surfaces; re-check contrast because WCAG 2 is unreliable for dark pairs (APCA's claim) — a design that passes 4.5:1 in dark mode may still read poorly, so targeting Lc 75–90 for body text in addition is a defensible belt-and-braces rule.
- Since Pencraft stores hex, ramps should be computed in OKLCH and converted with chroma reduction (not channel clipping) to stay in sRGB and keep hue; very high-chroma mid-lightness steps of some hues (not blue) will not exist in sRGB.
- The Pencraft automatic check (text 4.5:1) is aligned with the only normative model. APCA could be offered as an advisory second number, never as a replacement.
- Semantic colours: pick error/success/warning/info from the Radix hue families, and shift a semantic hue if it collides with the brand hue (e.g. red brand needs a distinguishable error treatment plus an icon; see 1.4.1 in section 5).
- Data colours: Okabe-Ito is a ready categorical set of eight; never encode a series by hue alone.
- 60-30-10 is best taught as "a dominant neutral area, a secondary area, a small accent area", i.e. a proportion heuristic, with the caveat that role-based ramps, not three colours, are what real interfaces need.

### Gaps
- m3.material.io colour pages unreadable; the official tone-to-role table is UNVERIFIED. Commonly quoted: tonal palettes at tones 0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100 (later extended with 4, 6, 12, 17, 22, 24, 87, 92, 94, 96, 98 for surfaces); light scheme primary 40 / on-primary 100 / primary-container 90 / on-primary-container 10 (newer spec 30); dark scheme primary 80 / on-primary 20 / primary-container 30 / on-primary-container 90; three contrast levels (standard, medium, high); tone difference of 50 gives at least 4.5:1.
- Radix documentation of how dark scales are derived and of alpha variants was not read.
- Leonardo's exact parameters (key colours, background, colour spaces such as CAM02/OKLCH, WCAG 2 vs APCA formula switch, lightness/contrast/saturation theme controls) were not confirmed from the README text retrieved.
- No source read for Refactoring UI's palette advice (UNVERIFIED: 8–10 shades each of grey, primary and several accent colours) or for a reasoned critique of 60-30-10.
- The 8 % / 0.5 % colour-vision-deficiency prevalence is from a commercial page, not a primary study.
- Not researched: sequential and diverging data-palette construction, P3-specific palette design, Figma/Tailwind tooling for OKLCH, Evil Martians' Harmony palette.

---

## 5. Accessibility that can be judged on a still design

### Takeaway
A static design can be checked mechanically against a compact set of WCAG 2.2 numbers: text 4.5:1 (3:1 for large text), non-text and state indicators 3:1, targets 24x24 CSS px (AA) or 44x44 (AAA), a focus indicator at least a 2 px perimeter with 3:1 change, layouts that survive 320 px width and 200 % text, text that survives line-height 1.5 and letter-spacing 0.12 em, and no meaning carried by colour alone. Legally, the European Accessibility Act has applied since 28 June 2025 via EN 301 549 (WCAG 2.1 AA today; a 2026 revision adopting WCAG 2.2 AA is reported as published but its legal citation is unconfirmed).

### Cited Findings

**WCAG 2.2 numbers**
- 1.4.3 Contrast (Minimum), AA: text and images of text 4.5:1; large text 3:1; large text = at least 18 pt, or 14 pt bold; exempt: inactive components, pure decoration, invisible text, incidental text in pictures, logotypes — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 1.4.6 Contrast (Enhanced), AAA: 7:1, large text 4.5:1 — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 1.4.11 Non-text Contrast, AA: 3:1 against adjacent colours for the visual information needed to identify UI components and their states, and for parts of graphics needed to understand content; inactive components exempt — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 1.4.1 Use of Color, A: colour must not be the only visual means of conveying information, indicating an action, prompting a response or distinguishing an element — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 1.4.4 Resize Text, AA: text resizable to 200 % without loss of content or function — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 1.4.5 Images of Text, AA: use real text, not pictures of text, except where the presentation is essential (logotypes) — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 1.4.10 Reflow, AA: no loss of content and no two-dimensional scrolling at 320 CSS px width (or 256 CSS px height for horizontally scrolling content); maps, diagrams, data tables, toolbars and similar are exempt — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 1.4.12 Text Spacing, AA: no loss when users set line height to 1.5 times font size, paragraph spacing 2 times, letter spacing 0.12 times, word spacing 0.16 times — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 1.3.4 Orientation, AA: content not locked to one orientation unless essential — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 2.4.7 Focus Visible, AA: a visible keyboard focus indicator must exist (no size or contrast number in this criterion) — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 2.4.11 Focus Not Obscured (Minimum), AA (new in 2.2): the focused component must not be entirely hidden by author-created content such as sticky headers; 2.4.12 (AAA): no part hidden — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 2.4.13 Focus Appearance, AAA (new in 2.2): indicator area at least as large as a 2 CSS px thick perimeter of the unfocused component, and at least 3:1 contrast between the focused and unfocused states — [WCAG 2.2](https://www.w3.org/TR/WCAG22/)
- 2.5.8 Target Size (Minimum), AA (new in 2.2): targets at least 24 by 24 CSS px, except: Spacing (a 24 px diameter circle centred on each undersized target does not intersect another target or another such circle), Equivalent (another conforming control does the same thing), Inline (target in a sentence or constrained by line height), User-agent control, Essential — [Understanding SC 2.5.8](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)
- 2.5.5 Target Size (Enhanced) uses 44 by 44 CSS px; W3C encourages it for important links and controls — [Understanding SC 2.5.8](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)
- Platform targets for comparison: Material 48 dp for a 24 dp icon (40 dp dense); Carbon 44 px or larger — [Material Design 1, Style: Icons](https://m1.material.io/style/icons.html); [Carbon icons usage](https://carbondesignsystem.com/elements/icons/usage/)

**European Accessibility Act**
- Enforceable from 28 June 2025; conformance in practice means EN 301 549, whose current version 3.2.1 incorporates WCAG 2.1 AA — [Level Access: EAA (via search)](https://www.levelaccess.com/blog/european-accessibility-act/); [CookieYes EAA guide (via search)](https://www.cookieyes.com/blog/european-accessibility-act/)
- Microenterprise exemption (fewer than 10 employees and turnover or balance sheet not above EUR 2 million) is narrow and covers services, not products — [Taylor Wessing: EAA exemptions (via search)](https://www.taylorwessing.com/en/interface/2025/accessibility/key-eu-accessibility-act-exemptions-and-the-challenges-they-pose); [CCPC Ireland microenterprise guidelines (via search)](https://www.ccpc.ie/business/enforcement/accessibility/european-accessibility-act-guidelines-for-microenterprises/)
- EN 301 549 v4.1.1 incorporating WCAG 2.2 AA: one secondary source says it was published on 2 September 2026 with Official Journal citation expected in November 2026; another describes it only as "expected in 2026"; status conflicting and unconfirmed — [CookieYes EAA guide (via search)](https://www.cookieyes.com/blog/european-accessibility-act/); conflicting with [Level Access: EU accessibility requirements (via search)](https://www.levelaccess.com/blog/eu-accessibility-requirements-and-eaa-compliance/)

**Hand-off annotation conventions**
- GitHub Annotation Toolkit (Figma) annotation types: Basic notes, Button, Flow lines, Form element (input, textarea, checkbox...), Heading, Landmark, Link, List, Mobile annotations (iOS and Android), Media (image, video, audio), Metadata (page title and language), Ordering (focus order, arrow stops, reading order), System feedback (states and live regions), Table, User interactions (keyboard shortcuts, gestures) — [github/annotation-toolkit](https://github.com/github/annotation-toolkit)
- It was open-sourced in September 2025 and is a fork of CVS Health's Web Accessibility Annotation Kit (CC-BY 4.0) — [GitHub changelog 2025-09-22 (via search)](https://github.blog/changelog/2025-09-22-open-sourcing-the-github-annotation-toolkit); [Primer: Annotation Toolkit (via search)](https://primer.style/accessibility/tools-and-resources/annotation-toolkit)
- CVS Health kit components: buttons, headings, images, landmarks, links, inputs, focus order, reading order, notes, utilities — [cvs-health/annotations (via search)](https://github.com/cvs-health/annotations)
- Indeed's A11y Annotation Kit: callouts for elements, focus order, keyboard interactions; basic properties heading levels, image alt text, accessible names for icon buttons — [Indeed Design: Building an accessibility library (via search)](https://medium.com/indeed-design/building-an-accessibility-library-e134e9012c17)
- Other kits exist from Microsoft, GitLab, Twitter and Intopia; CVS found none of them complete — [CVS Health tech blog (via search)](https://medium.com/cvs-health-tech-blog/behind-the-scenes-of-creating-a-new-web-accessibility-annotation-kit-1834815544d3)

**What design-time checkers actually check**
- Figma's built-in checker (colour picker): contrast ratio 1:1 to 21:1 of the selected layer against its background; categories Auto, Large text, Normal text, Graphics; thresholds 4.5:1 AA and 7:1 AAA for normal text, 3:1 AA for large text and graphics; can snap to the nearest passing colour; APCA not mentioned — [Figma Help: colour picker](https://help.figma.com/hc/en-us/articles/360041003774-Apply-paints-with-the-color-picker)
- Stark (Figma plugin) tools: Contrast (AA/AAA with ratio), Typography, Vision Simulator (4 colour-blindness types plus other low-vision conditions), Focus Order, Landmarks, Touch Targets, Alt-text annotations; January 2026 release notes add checks on Figma Slides — [Stark for Figma (via search)](https://www.getstark.co/figma); [Stark release notes (via search)](https://getstark.co/release-notes)
- Pencraft-relevant consequence of Carbon's rule: icons are held to 4.5:1 like text in that system, stricter than WCAG's 3:1 for graphics — [Carbon icons usage](https://carbondesignsystem.com/elements/icons/usage/)

### Inferences
- Checks Pencraft could add to its automatic counters, all computable from layer geometry and fills: (1) large-text exception: 3:1 when size is at least 24 px regular or about 18.66 px bold (18 pt / 14 pt at 96 dpi; the px conversion is mine); (2) non-text contrast 3:1 for input borders, icon-only buttons, toggle tracks, chart marks, focus rings; (3) interactive layer smaller than 24x24 with no 24 px clearance circle (warn), under 44x44 on touch frames (advise); (4) a text style whose line height is under about 1.5 for body paragraphs or whose box has no slack for 1.5 line height (advisory proxy for 1.4.12); (5) status conveyed only by fill colour: e.g. error fields with red border but no icon or text, links distinguished from body only by colour, chart series with no label or pattern (heuristic for 1.4.1); (6) text baked into a generated raster picture (1.4.5); (7) form inputs with placeholder only and no persistent label, or an error state without a text message (3.3.x, see Gaps for the exact wording).
- A focus state is a design deliverable: draw at least one focused variant per interactive component with a ring at least 2 px thick, 3:1 against both the component and the surrounding background (meets 2.4.13 AAA and trivially 2.4.7), with an offset so it is not hidden by neighbours.
- Reflow and resize can only be evidenced on a still design by drawing a 320 px wide frame and a 200 % text variant; a guide can tell the model to offer the narrow frame for any web page design and to prefer flex layout with wrapping over fixed positions so the design is reflow-ready.
- An "accessibility annotations" skill fits Pencraft's primitives well: numbered focus-order badges joined by connectors; landmark outlines with labels (header, nav, main, aside, footer); heading-level tags (H1–H6) beside headings; alt-text notes on images ("decorative" or the text); accessible-name notes on icon buttons; page title and language as metadata; state/live-region notes. The category list is taken from the GitHub/CVS kits.
- For EU-facing work, WCAG 2.1 AA is the present legal floor and WCAG 2.2 AA the safe target (2.2 adds target size 2.5.8 and focus-not-obscured 2.4.11, both visible in a still design).

### Gaps
- The fetch of WCAG 2.2 was truncated before 2.5.5 and 3.3.x; exact normative text for 2.5.5 (AAA, 44 by 44 with exceptions), 3.3.1 Error Identification (A: errors identified and described in text), 3.3.2 Labels or Instructions (A), 3.3.3 Error Suggestion (AA), 3.3.7 Redundant Entry (A, new in 2.2) and 3.3.8 Accessible Authentication (AA, new in 2.2) is UNVERIFIED here (levels given from training knowledge).
- EN 301 549 v4.1.1 publication date and Official Journal citation are unconfirmed; EAA transition provisions (UNVERIFIED: services contracts and products already in use may continue until 28 June 2030) were not found in results.
- No statistic was found for the share of accessibility defects that originate in design (the GitHub toolkit page gives none).
- Polypane's checks were not researched; Intopia's kit contents were not confirmed; the visual styling conventions of the annotation kits (colours, badge shapes) were not described in sources read.
- Stark's touch-target thresholds and heading checks were not detailed in results.

---

## 6. Diagrams, flows and wireframes

### Takeaway
The conventions are few and stable: flow diagrams use ovals for start/end, rectangles for screens or steps, diamonds for decisions with every exit labelled, one reading direction, and the expected path drawn as the straight "yes" line; a wireflow replaces boxes with small wireframes and draws each arrow from the exact control that triggers it. Architecture diagrams (C4) need a title, a legend, typed and described elements and specifically labelled one-way arrows; journey maps and service blueprints are fixed row layouts.

### Cited Findings

**Flow diagrams and wireflows**
- Shapes: oval = entry/exit, rectangle = screen/action/process, diamond = decision (usually a yes/no question); arrows show direction; these three are the most common flowchart symbols — [Lucidchart: flowchart symbols (via search)](https://lucid.co/lucidchart/flowchart-symbols); [Figma resource library: flowchart symbols (via search)](https://www.figma.com/resource-library/flowchart-symbols/)
- Keep one flow direction (top-to-bottom or left-to-right); every diamond has at least two labelled exits; draw the primary path from entry to goal first, then edge cases; consistently draw the expected path as the "Yes" branch — [UX Collective: capturing alternative paths (via search)](https://uxdesign.cc/capturing-alternative-paths-to-improve-design-outcomes-f85a9b66dadb); [Optimizely: user flow map (via search)](https://www.optimizely.com/insights/how-to-create-a-user-flow-map-for-your-website--app)
- Sources disagree on the number of terminals: one says exactly two ovals per flow, another allows several starts and ends — [search summary of the same sources](https://www.linearity.io/blog/how-to-make-a-user-flow-diagram/amp/)
- Wireflow = wireframe-style page layouts combined with a simplified flowchart; best for apps with few pages whose content changes dynamically (checkout, mobile apps); flowcharts document complex multi-path logic exhaustively but omit page context; plain wireframes suit mostly static page-based sites — [NN/g: Wireflows](https://www.nngroup.com/articles/wireflows/)
- Wireflow rules: each step is a screen or the same screen in a changed state; on desktop show only the part that changes (dialog, filter panel); each arrow starts at the specific UI element acted on and points to the resulting state; hotspots must be clearly marked where a screen has several targets; show feedback (confirmation, error) as part of the result — [NN/g: Wireflows](https://www.nngroup.com/articles/wireflows/)

**Wireframes and fidelity**
- Fidelity has three independent dimensions: interactivity, visuals, content; low-fidelity visuals show only some or none of the final attributes (e.g. black-and-white sketch); low-fidelity content uses stand-ins such as placeholder images — [NN/g: prototype fidelity](https://www.nngroup.com/articles/ux-prototype-hi-lo-fidelity/)
- Low fidelity: faster, easier to change, invites more honest criticism, signals "unfinished" to stakeholders; high fidelity: tests legibility, affordance, detailed workflows and visual hierarchy — [NN/g: prototype fidelity](https://www.nngroup.com/articles/ux-prototype-hi-lo-fidelity/)
- Balsamiq tips: size the board to the target device; use a grid; keep buttons, tabs and labels consistent via reusable symbols; think in flows and connect screens with arrows; use real (or realistic-length) content early; colour sparingly and consistently, e.g. only to mark interactive elements or the selected item; "functional, not pretty": start from basic boxes; add brief annotations (sticky notes) — [Balsamiq: practical tips for better wireframes](https://balsamiq.com/blog/practical-tips-for-better-wireframes/)
- Image placeholders stand in for logos and featured images when the real asset is missing; a secondary study guide describes them as boxes with X marks that keep attention on layout — [Balsamiq: image guidelines (via search)](https://balsamiq.com/learn/ui-control-guidelines/images/); [Fiveable: wireframing best practices (via search)](https://fiveable.me/lists/wireframing-best-practices)

**Architecture diagrams (C4)**
- Levels: 1 System context, 2 Container, 3 Component, 4 Code; plus system landscape, dynamic and deployment diagrams — [C4 model: notation](https://c4model.com/diagrams/notation)
- Every diagram has a title stating type and scope (e.g. "System Context diagram for My Software System") and a key/legend explaining shapes, colours, border styles, line types and arrowheads; acronyms explained — [C4 model: notation](https://c4model.com/diagrams/notation)
- Every element states its type (Person, Software System, Container, Component), has a short description of its responsibilities, and containers/components state their technology — [C4 model: notation](https://c4model.com/diagrams/notation)
- Every line is one-way and labelled with a specific phrase matching its direction (not just "Uses"); container-to-container lines also name the protocol/technology — [C4 model: notation](https://c4model.com/diagrams/notation)
- C4 prescribes no shapes or colours; keep colour coding consistent within and across diagrams and consider black-and-white printing and colour blindness; the test is that the diagram stands alone without narration — [C4 model: notation](https://c4model.com/diagrams/notation)

**Journey maps and service blueprints**
- Journey map zones: top = actor (one persona per map), scenario and expectations; middle = phases, each with actions, mindsets (ideally real quotes) and a single emotion line running across phases showing highs and lows; bottom = opportunities, insights and internal ownership — [NN/g: Journey Mapping 101](https://www.nngroup.com/articles/journey-mapping-101/)
- Service blueprint rows: customer actions; frontstage actions (visible; human or technology); backstage actions; support processes; with evidence (props and places) attached; separated by the line of interaction (customer vs organisation), line of visibility (frontstage vs backstage) and line of internal interaction (contact staff vs support) — [NN/g: Service Blueprints: Definition](https://www.nngroup.com/articles/service-blueprints-definition/)
- Blueprint arrows: single arrow = one-way exchange; double arrow = agreement or co-dependency; optional rows: time, regulations/policy, emotion, metrics; one blueprint per customer journey — [NN/g: Service Blueprints: Definition](https://www.nngroup.com/articles/service-blueprints-definition/)

### Inferences
- These map directly onto Pencraft primitives: frames with flex layout for swimlane/row grids; rectangles, ellipses (pill terminals) and 4-point polygons (diamonds); connectors between layers for arrows; text labels on connectors. A "diagrams-and-flows" skill can state: one direction; equal node sizes per type; nodes on a grid with consistent gaps; one arrowhead style; orthogonal connectors that do not cross nodes; label every decision exit; main path straight, exceptions branch off to one side; legend when more than one shape or colour meaning is used; colour only to encode a category, never as the sole cue (ties to 1.4.1).
- A "wireframes" skill can state: greyscale only (one accent reserved for interactive/selected), boxes and real-length text, no shadows/gradients/pictures, device-sized frames, annotations visually distinct from the wireframe itself (a different colour and a numbered marker), and wireflow arrows that start on the triggering control.
- C4's checklist (title, legend, typed elements, verb-phrase arrows) is a good generic quality gate for any generated architecture diagram, and is automatable: flag a diagram frame with no title text, connectors without labels, or more than one fill colour without a legend.
- Journey maps and blueprints are essentially tables with a time axis left to right and fixed rows; a model should build them as a grid frame first and only then place content, with the three blueprint lines drawn as full-width dashed rules and the emotion curve as a single bezier path across phases.

### Gaps
- No primary source read this session for: the classic wireframe vocabulary beyond placeholders (UNVERIFIED conventions: crossed rectangle for images, grey bars or squiggles for body text, real text for headings and labels, annotations in a contrasting colour, often blue or red, with numbered callouts); sitemap conventions (tree of page boxes, numbered hierarchy such as 1.0 / 1.1); UML basics (class boxes with three compartments, solid line association, hollow triangle generalisation, diamonds for aggregation/composition; sequence diagram lifelines); Excalidraw/Whimsical/FigJam house styles; storyboards (3–6 panels with captions); design-system documentation boards (do/don't cards with green/red markers, anatomy call-outs with numbered pins, red-line spacing annotations). All UNVERIFIED practitioner knowledge.
- No numeric spacing rules for diagrams (node gaps, arrowhead sizes, line weights) were found in an authoritative source; any numbers in a skill would be house rules.
- ISO 5807 (the formal flowchart symbol standard) was not consulted; shape meanings above come from vendor guides.
- The C4 review checklist page (c4model.com/diagrams/checklist) was not read.
- Balsamiq's article did not cover fonts, greyscale palettes or the X-box explicitly; the X-box description is from a secondary study guide.
