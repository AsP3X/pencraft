---
name: ui-components
description: Design standard interface components in Quill the way the common component libraries build them - buttons, inputs, selects, checkboxes, switches, tabs, cards, tables, dialogs, menus, popovers, tooltips, toasts, drawers and command palettes, a sidebar and top bar, calendars and date fields, data tables, the parts of an AI chat, and whole screens such as a dashboard, a sign-in page or an inbox, with their sizes, tokens, states and placement. Use when the user asks for a component, a component set, a design system or UI kit, a form, a settings screen or dialog, a dashboard, an admin or app screen, a chat with an assistant, names shadcn/ui, Radix, Base UI, Sonner, Vaul or cmdk, or asks for standard, default or plain controls, or for the states of a control. Do not use for glass or translucent surfaces (the liquid-glass skill), nor for posters, landing pages and other designs with a look of their own.
license: Apache-2.0 with Commons Clause (see LICENSE in the Quill repository)
metadata:
  product: pencraft
  version: "1"
---

# Standard components in Quill

A standard component is not designed, it is built to a specification people already know by hand: where the
label sits, how tall the control is, what a menu does at its edge. The libraries that most interfaces are
built from agree on nearly all of it. This skill is that agreement, as Quill layers. Where the user
asks for standard or names a library, follow it closely: that is what the request means. Where the design
has a look of its own, keep the anatomy (heights, spacing, states, where things sit) and change the skin
through the tokens.

Work with Quill's tools (`get_state`, `read_layers`, `edit_layers`, `screenshot`). The reference files
named below are read with `read_skill` (name `ui-components`, `file` as given).

Part of this skill is firm and part is a starting point. It says which, and the difference matters:

- Firm: the list "Every time" below, and the numbers of a library whenever the user named that library. Do
  these without being asked, on every design this skill is used for. Only the user's own words overrule
  them; then do what was asked and say in one line what it costs.
- A starting point: the presets when no library was named. In a design with a look of its own the numbers in
  rules 3, 4 and 6 may change as a set (another ladder of heights, other radii, colours and faces), as long
  as they are chosen once and then kept; the rules themselves stay. Also the sample copy, widths and
  arrangement of the examples. The whole screens in `references/screens.md` are worked examples, not layouts
  to reproduce. Begin there, and change it when the design has a reason. They are still the expected way:
  leave one only with a reason, which can be short and needs nobody's leave.
- A design that already has a look, a brand or a system keeps it. Take from this skill what the design
  lacks, and nothing that would replace what it has.
- The examples in the reference files show how a thing is built in Quill and at what size. Their colours,
  faces, words and arrangement are one answer among many, not the look to reproduce.
- The checklist and "When it looks wrong" at the end are there to be gone through every time, as a check
  of your own work. A line in them that speaks of a starting point is a starting point still.

Every time:

1. Build what was named, all of it, and nothing else. No backdrop, heading or extra variants around the
   work. (The board that keeps the reusable components is not the work and stays out of its pictures:
   section 7.) Where the request itself breaks a rule of this skill (a switch beside a Save button, a button
   called "Save"), the request wins: build it, and say so in one line.
2. Every colour, corner and face comes from a variable. No hex in a component, except white on a slider knob.
3. One control height per row, from the ladder (on a phone: 44). Never a 34 beside a 36.
4. A line is 1 wide. Elevation is a hairline plus a soft shadow, and only on what floats.
5. A control that can be used shows its states when states were asked for; a state is a frame, not a note.
6. Text in a control is 14 at weight 500 or 400 (on a phone 16). Nothing in a component is larger than 18;
   the title of a page is the page's own: 24 at weight 500 unless the design has a scale.

## 1. Read the request

Write down what was named: each component, each variant, each state, each size, light or dark. Then the
reference:

| The user said | Take |
|---|---|
| nothing, or "standard", "plain", "default" | the Classic preset below |
| shadcn, shadcn/ui | Classic; say in one line that Compact is shadcn's 2026 default for new projects and offer it |
| "compact", "dense", Nova, a data-heavy tool | Compact |
| Radix Themes, Base UI, a named shadcn style (Maia, Luma, Sera ...) | Classic changed as `references/styles.md` says |
| Sonner, Vaul, cmdk | the toast, drawer or command palette in `references/overlays.md`, which follow them |
| "like Linear", "like Raycast", another product's command palette | the command palette in `references/overlays.md` in this preset; that product's own traits (`references/styles.md`) only when asked for |

If the document already has variables or components, they are the system: map the roles of section 2 onto
them and add only what is missing.

## 2. Set the tokens once

Run the `set_variables` call in `references/tokens.md` before the first layer. It gives the paired colour
roles (a surface and what is written on it), the lines, the radius scale and the face, with a `Mode` axis for
light and dark. (`font-mono` is for code alone: set it to an installed fixed-width family when a code block is
asked for.) Change `font` in that call to an installed family before you send it (`get_guide` lists them
with their weights): Inter where it is installed; on Windows Segoe UI Variable (plain Segoe UI has no weight
500, which every label is set in); otherwise SF Pro Text or Noto Sans. The family needs 400 and 500.

| Preset | Control height | Button and input padding | Card and dialog padding | Radius base | Menu row |
|---|---|---|---|---|---|
| Classic | 36 | 10 | 24 | 10 (controls 8) | 32 |
| Compact | 32 | 10 | 16 | 10 (controls 10) | 28 |

## 3. Sizes and type

Controls sit on one ladder of heights: 24, 28, 32, 36, 40. A row of controls shares one height. On a phone
whatever is tapped is 44 high and input text is 16.

| Part | Value | Why |
|---|---|---|
| Control label | 14, weight 500 | One size for everything pressed; hierarchy comes from weight and colour |
| Input text, body, description | 14, weight 400; descriptions in `$muted-foreground` | |
| Meta: menu group label, badge, kbd | 12, weight 500 | |
| A shortcut written as text in a menu; a tooltip | 12, weight 400 | |
| Titles: card 16/500, dialog 16/500 or 18/500 (alert), empty 18/500 | never above 18 | Size is the page's tool, not the component's |
| Icon | 16 beside 14 text, 12 beside 12 text; 6 to 8 from its label | |
| Padding in a control | 8 at 24 to 28 high, 10 at 32 to 36, 10 to 16 at 40 | |
| Between label, control and help text | 8 | |
| Between fields | 24 (Compact 16) | |
| On a phone | controls 44 high with 12 beside the text; input, select and button text 16; rows of a list 56 to 64 | A finger needs 44; under 16 a phone zooms into the field |
| Card and dialog padding, and the gap between their parts | 24 (Compact 16) | |
| Popover padding 16; menu padding 4; menu row padding 8 | | |

Weights are 400 and 500. 600 is for a figure in a statistic and for nothing else. No negative letter spacing
in a component, no capitals with spacing (except a shortcut hint).

## 4. Lines, corners, elevation

- A control's border is `$input`, 1 wide, `"strokeAlignment":"inner"`. A layout line (a table row, a
  separator) is `$border`. What floats has `$hairline` instead of a border: the text colour at 10%.
- Corners come from one base: `$radius-sm` 6 (menu rows, checkbox uses 4), `$radius-md` 8 (controls, menus,
  popovers), `$radius-lg` 10 (tab lists, alerts, toasts), `$radius-xl` 14 (cards, dialogs). An inner corner is
  the outer corner minus the gap between them, and not less than 2 where the outer one is rounded: a button 24
  inside a 14 dialog keeps its own 8, a tab 4 inside a 10 tab list has 6. Never make the inner corner
  larger than the outer.
- Three levels of elevation, recipes in `references/tokens.md`. Level 0: controls and cards on the page have
  no shadow (a card may take the faintest one). Level 1: what floats beside its trigger (menu, select list,
  popover, tooltip) has the hairline and the floating shadow. Level 2: what covers the page (dialog, sheet,
  drawer) has the hairline and a scrim behind it; the scrim, not a bigger shadow, is what separates it. A
  toast takes the large shadow.
- In the dark theme a surface gets lighter as it rises (page, then card and popover, then muted), lines are
  white at 10% (inputs 15%), and shadows nearly vanish: the lighter fill and the hairline carry the depth.
  Never invert a light design.
- Quill's review remarks on an outline with a shadow; a hairline at 20% or less is not counted. Its remark
  on Inter as a default face is silent when the face comes from the `font` variable.

## 5. States

Every control that can be used has these. A state that many instances share is named once in the
component's `states` and picked on an instance with `"state"` (the buttons and inputs of
`references/controls.md` come with hover, focus, disabled and invalid); a state one instance shows alone is
an override on it:

| State | Recipe |
|---|---|
| Rest | the component |
| Hover | primary `$primary-hover`; secondary `$secondary-hover`; destructive `$destructive-tint-hover`; outline and ghost `$muted`; a menu row `$accent`; a table or list row `$muted-half` |
| Pressed | as hover, drawn 1 lower |
| Focus | border `$ring`, and a 3 wide halo of `$ring-soft` around it: a shadow with `"spread":3`, no offset and no blur (`references/controls.md`) |
| Disabled | `"opacity":0.5` on the control (what is in it fades with it; do not set it twice) |
| Invalid | border `$destructive`, halo `$destructive-tint`, and the message under the control in `$destructive` |
| Loading | `loader-circle` (16, in the label's colour) in place of the leading icon, the label kept, `"opacity":0.5`. A button without an icon gets the spinner before its label; give its rest and loading instances the same `width` so nothing jumps |

And their own: a checkbox has unchecked, checked, indeterminate; an input has placeholder, filled, read-only;
a menu row has highlighted (the pointer is on it) and checked (a mark at its trailing edge), which are
different things; a tab has selected.

When the user asks for states, build a state board as `references/controls.md` ("State board") shows: a
frame named "<Component> states" in `$background` with 32 around; rows are the variants, each starting with
its name; columns are the states, each under its name; labels 12 at weight 500 in `$muted-foreground`; 24
between columns, 16 between rows. Name each instance `Component / variant / state`. Show the dark theme as a
copy of the board with `"theme":{"Mode":"Dark"}`, 48 beside the first. The rule of one focus ring per screen
is for screens, not for this board. A design cannot move: a transition is two frames side by side.

Moments of one component (a palette as opened, typed, empty; a drawer at two heights) are laid out the same
way: side by side, 48 apart, top edges level, a 12 label in `$muted-foreground` 12 above each. Each moment is
an instance of the component; the component itself stays on the board of section 7.

## 6. Overlays in a still picture

A menu, popover or tooltip is drawn open, next to the control that opened it, and that control shows its open
state (the hover fill).

- 4 from the trigger (a popover with content of its own: 8). Below it and aligned to its start edge by
  default; a tooltip sits above, centred.
- At least as wide as the trigger; a select list exactly as wide.
- 10 or more from the edge of the screen; if it would not fit below, draw it above.
- Give the overlay `"layoutPosition":"absolute"` with `x` and `y` in the screen frame, after the content in
  the layer order, so it does not push the layout apart.
- A dialog is centred with the scrim rectangle under it covering the screen (work out its `y` from its
  height, read with `read_layers`). Only one modal at a time.
- A toast sits 24 from the bottom right corner. On a phone it is 16 from the left and right edges, at the
  bottom: 16 above the bottom edge, or 16 above the top edge of whatever holds the bottom (a tab bar, an open
  drawer). It lies above the scrim, and it may cover rows of the page; never put it over the page's title.
- A select, a combobox and a date field are drawn closed, showing a value, unless the open list is what the
  user asked to see.
- Scrim, hairline and shadow are for overlays inside a screen. A component shown by itself (hard rule 1)
  gets no screen and no scrim around it: its own hairline and shadow hold it on the canvas.
- The page under an overlay is an ordinary page that happens to be covered: do not cut its list to fit
  around the overlay. When the user gives no page, draw a plain one (a title and a few rows) and say so. A
  screen has its title; that is part of a page, not a heading added around the work.
- A drawer's `y` is the screen's height less the drawer's height, which is known only after it is filled:
  insert it, read its height with `read_layers`, then set `y`.

## 7. Workflow

1. `get_state`, `read_layers`: which variables and components are there.
2. The tokens (`references/tokens.md`), once.
3. Insert only the components the request needs, as reusable components, on a board named "UI components"
   (`"fill":"#e5e5e5"`, the one hex allowed; vertical, gap 32, padding 32) placed well to the left of the
   work (at `x` -1600 when the work starts at 0; it only has to be clear of it): controls from `references/controls.md`, floating
   things from `references/overlays.md`, cards, tables and the rest from `references/data.md`; a sidebar, top
   bar, pagination, accordion or stepper from `references/navigation.md`; a calendar, date field, combobox,
   chips, one-time code or file drop from `references/inputs.md`; a table with a toolbar, selection and paging,
   and the stat tile, from `references/tables.md`; the parts of a chat with an assistant from
   `references/chat.md`. A whole screen (a dashboard, sign-in, an inbox, settings, pricing, checkout) starts
   from `references/screens.md`, which names the components it needs. The board is
   where the components live, not part of what the user asked for: it carries the references' sample copy,
   and it is left out of every picture and export of the work. Ids start with `ui-`; if one is there
   already, instance it. Only when the user asked for the kit itself is the board the work: then give it
   real copy and a dark copy beside it.
4. Compose the work with instances (`ref`) and `descendants` overrides, also when the work is one component.
   Change sizes only along the ladder.
5. `screenshot` with the `ids` of the work (not the whole canvas), put right what the review counted, then
   go through the checklist.
6. Tell the user in two sentences what is there, and name anything they asked for that Quill cannot draw
   (section 9).

## 8. Checklist

- Everything named is there; nothing unnamed was added.
- One height per row of controls; labels 14; icons 16; one face.
- No hex in a component. Where the user asked for both themes, both are there and both were looked at.
- Floating things: hairline, floating shadow, 4 from their trigger, the trigger in its open state.
- Each requested state is a named frame and visibly different from rest.
- Figures in a column end on one edge, with the same decimals.
- Copy is real: "Send invite", not "Submit"; a real name, an uneven number.

## 9. What Quill cannot draw, and what to do instead

| Missing | Instead |
|---|---|
| Figures of one width (tabular numerals) | Right-align figure columns at a fixed width; avoid faces with old-style figures |
| Two strokes on one layer | A shadow that only spreads for a ring outside (the focus ring); an inner shadow (y 1, blur 0, white at 14%) for a top highlight |
| Variants of a component (primary, outline, ghost) | Overrides on an instance; only states are named on the component |
| Showing a state when the pointer or the keyboard is there | A state is picked by hand: one instance per state, on a state board |
| Text cut off with an ellipsis | Write the shortened text with a real "..." at a fixed width; never shorten a number or a date |
| Dashed lines | A solid 1 line in `$border` |
| Motion | Two frames: before and after |

## 10. When it looks wrong

| It looks like | Cause | Fix |
|---|---|---|
| A wireframe | Opaque grey borders everywhere, no fills | Layout lines `$border`, surfaces `$card`, one `$primary` action |
| Heavy, boxed in | 2 wide strokes, a shadow on every control | 1 wide lines; shadows only on what floats |
| Cheap in the dark theme | Grey opaque borders, black shadows, surfaces all one black | White at 10% lines, lighter surfaces as they rise |
| Uneven rows | Controls of 34, 36 and 40 mixed | One step of the ladder per row |
| A menu that looks pasted on | No gap to its trigger, or wider gap than 8, or no hairline | 4 from the trigger, start edges aligned, hairline and floating shadow |
| Muddy focus | A blurred glow as focus ring | The 3 wide solid halo of `$ring-soft` |
| Shouting | Solid red delete buttons | Destructive is a tint: `$destructive-tint` with `$destructive` text |
| A toy | Label 16 and bold, 12 radius on a 32 control | Label 14/500, radius 8 |

Named looks and what changed in the libraries in 2026: `references/styles.md`.
