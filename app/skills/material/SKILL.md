---
name: material
description: Design Android and Material Design interfaces in Pencraft to Google's Material 3 Expressive specification - colour roles, the type scale, shape, elevation and state layers, and components at their real sizes (buttons and button groups, FAB, icon buttons, chips, cards, text fields, switches, checkboxes, sliders, lists, top app bar, navigation bar and rail, search bar, tabs, dialogs, bottom sheets, snackbars), with a ready Android phone screen. Use when the user asks for an Android app or screen, names Material, Material Design, Material 3, Material You, M3 Expressive, Jetpack Compose, Pixel or Google's design system, or wants a FAB, a navigation bar, chips or a snackbar. Do not use for iPhone or Apple designs (the liquid-glass and devices skills), nor for web components in the manner of shadcn or Radix (the ui-components skill).
license: Apache-2.0 with Commons Clause (see LICENSE in the Pencraft repository)
metadata:
  product: pencraft
  version: "1"
---

# Material 3 in Pencraft

Material is a specification: a screen is right when its sizes, roles and shapes are the published ones.
People who use Android every day see a 48 high button or a square FAB at once. This skill is the numbers
of Material 3 Expressive, the current version, as Pencraft layers. Where the user asks for Material, they
are what make a screen read as Material; the colour (from one seed), the content, the composition and the
emphasis are yours to design.

Work with Pencraft's tools (`get_state`, `read_layers`, `edit_layers`, `screenshot`). The reference files
named below are read with `read_skill` (name `material`, `file` as given).

Part of this skill is firm and part is a starting point. It says which, and the difference matters:

- Firm: the list "Every time" below, and Material's published sizes, type roles, colour roles and shapes,
  whenever the user asked for Material or Android. Do these without being asked, on every design this skill
  is used for. Only the user's own words overrule them; then do what was asked and say in one line what it
  costs.
- A starting point: the scheme and its seed colour, which components a screen uses, how expressive it is,
  the content and its order, and the paddings this kit marks as its own. When the user wants something only
  in the manner of Material, the whole specification becomes a starting point. Begin there, and change it
  when the design has a reason. They are still the expected way: leave one only with a reason, which can be
  short and needs nobody's leave.
- A design that already has a look, a brand or a system keeps it. Take from this skill what the design
  lacks, and nothing that would replace what it has.
- The examples in the reference files show how a thing is built in Pencraft and at what size. Their colours,
  faces, words and arrangement are one answer among many, not the look to reproduce.
- The checklist and "When it looks wrong" at the end are there to be gone through every time, as a check
  of your own work. A line in them that speaks of a starting point is a starting point still.

Every time:

1. Colour is a role, never a hex: every fill and text colour is one of the role variables, and what is
   written on a surface is that surface's own "on" colour (`$on-primary` on `$primary`).
2. Type is one of the fifteen roles (section 3): its size, line height, letter spacing and weight together.
   No size in between.
3. Sizes are the specification's (section 5 and `references/components.md`). One unit is one dp. Everything
   pressed has 48 by 48 to be hit, even when it is drawn smaller.
4. Depth is the surface's tone first: a container one step lighter or darker. A shadow is only for what
   floats (a FAB, a dialog, a menu, a snackbar).
5. Corners come from the shape scale, by component: 4, 8, 12, 16, 20, 28, 32, 48, or fully round.
6. Build what was named and nothing around it. A component shown alone gets no phone and no screen.

## 1. Read the request

| The user said | Take |
|---|---|
| Material, Material 3, Material You, Android, nothing more | Material 3 Expressive, as here |
| "Expressive" | The same, and use its marks: large buttons where one action leads, button groups, emphasized type, the floating toolbar |
| Material 2, "classic Material", an old app to match | Not this skill's numbers: say so, and match the screens the user shows |
| A phone, a tablet, a foldable | The window size class of section 6 decides bar or rail |
| A brand colour | The scheme is built from it (section 2), not painted over the baseline |

If the document already has Material role variables, they are the system: use them and add nothing.
Do not put this skill's variables into a document that carries the ui-components skill's tokens: both have a
`primary` and a `secondary`, with different meanings.

## 2. Set the roles once

Run the `set_variables` call in `references/tokens.md` before the first layer. It gives the roles of the
baseline scheme (the purple one) for light and dark. Change `font` in that call to Roboto or Google Sans
Flex when one is installed (`get_guide` lists the families); otherwise the system's sans-serif.

- A role's tone is fixed: `primary` is tone 40 of the primary palette in light and tone 80 in dark, its
  container 90 and 30, and so on (the table in `references/tokens.md`). A tone is a lightness from 0 (black)
  to 100 (white), so a scheme for another colour keeps every role's lightness and changes only hue and
  strength.
- For a brand colour, build the scheme by that table: the brand's hue for the primary palette, the same hue
  at a third of its strength for secondary, a neighbouring hue for tertiary, and the same hue at almost no
  strength for the neutrals. Say that this is worked out by eye: Google's tool does it by an algorithm
  Pencraft does not have.
- Three accents with jobs: primary for the main action and what is active, secondary for quieter parts
  (a selected chip, the active navigation item's pill), tertiary for a contrasting touch. Most screens need
  only primary, the secondary container and the surfaces.
- Surfaces in five steps, `surface-container-lowest` to `-highest`: the page is `surface`; a navigation
  bar `surface-container`; a search bar and a dialog `surface-container-high`; a filled card and a text
  field `surface-container-highest`. In dark the steps get lighter as they rise.
- Lines: `outline` for what must be seen (a text field's border), `outline-variant` for dividers and quiet
  borders.

## 3. Type

Fifteen roles in five families. A text takes all four values of its role.

| Role | Size | Line height (as a multiple, and in dp) | Letter spacing | Weight | Emphasized |
|---|---|---|---|---|---|
| Display large | 57 | 1.1228 (64) | -0.25 | 400 | 500 |
| Display medium | 45 | 1.1556 (52) | 0 | 400 | 500 |
| Display small | 36 | 1.2222 (44) | 0 | 400 | 500 |
| Headline large | 32 | 1.25 (40) | 0 | 400 | 500 |
| Headline medium | 28 | 1.2857 (36) | 0 | 400 | 500 |
| Headline small | 24 | 1.3333 (32) | 0 | 400 | 500 |
| Title large | 22 | 1.2727 (28) | 0 | 400 | 500 |
| Title medium | 16 | 1.5 (24) | 0.15 | 500 | 700 |
| Title small | 14 | 1.4286 (20) | 0.1 | 500 | 700 |
| Body large | 16 | 1.5 (24) | 0.5 | 400 | 500 |
| Body medium | 14 | 1.4286 (20) | 0.25 | 400 | 500 |
| Body small | 12 | 1.3333 (16) | 0.4 | 400 | 500 |
| Label large | 14 | 1.4286 (20) | 0.1 | 500 | 700 |
| Label medium | 12 | 1.3333 (16) | 0.5 | 500 | 700 |
| Label small | 11 | 1.4545 (16) | 0.5 | 500 | 700 |

- Display and headline for the few large things (a figure, a greeting, a page's title in a large app bar);
  title for bars, cards and dialogs; body for reading; label for everything pressed and for captions.
- Emphasized (Expressive) keeps the size and raises the weight: for the selected item, the main figure, the
  headline that leads. Not for whole paragraphs.
- Buttons, chips and tabs are Label Large or Title Small: 14 at weight 500. Sentence case, never capitals.
- Body text that is read is Body Large; Body Medium for supporting lines.

## 4. Shape, elevation, states

| Shape | Corner | For |
|---|---|---|
| Extra small | 4 | Text field, snackbar, the inner corners of joined parts |
| Small | 8 | Chip, a button when pressed |
| Medium | 12 | Card, small FAB, a square button of 32 or 40 |
| Large | 16 | FAB, extended FAB, navigation drawer, a square button of 56 |
| Large increased | 20 | Medium FAB |
| Extra large | 28 | Dialog, bottom sheet (top corners), search view, large FAB |
| Extra large increased, extra extra large | 32, 48 | The largest containers of an expressive layout |
| Full | Half the height | Round buttons, the navigation indicator, search bar, switch |

- A button is round (a pill) or square (the corner of its size); pressed, either becomes the smaller
  corner; a toggle that is selected swaps round for square. In a still picture: draw the pressed or
  selected one with its other shape.
- Elevation has six levels. In a picture a level is a tone and, from level 1, a shadow; the shadows are
  written out in `references/tokens.md`. Level 0: the page, a filled or outlined card, a top app bar at
  rest. Level 1: an elevated card or chip, a bottom sheet. Level 2: a menu, a top app bar with content
  scrolled under it. Level 3: a FAB, a dialog, a snackbar, a search bar.
- A state is a layer of the content's colour over the container: 8% under the pointer, 10% focused or
  pressed, 16% dragged. As a fill: `["$primary", {"type":"color","color":"$on-primary","opacity":0.08}]`.
  Disabled: the container is `$on-surface` at 10%, its content `$on-surface` at 38%.
- What covers the page (a dialog, a sheet, a drawer) has `$scrim` under it, over everything else.

## 5. Sizes that are easy to get wrong

| Part | Material 3 Expressive | The older value a memory may hold |
|---|---|---|
| Button | 40 high by default with 16 beside the label; five sizes: 32, 40, 56, 96, 136 | 40 with 24 beside |
| Icon in a button | 20 (24 from the 56 size) with 8 to the label | 18 |
| Icon button | 40 (also 32, 56, 96, 136), icon 24 | 48 |
| FAB | 56 with corner 16; 80 with 20; 96 with 28; level 3 | Round |
| Navigation bar | 64 high, indicator 56 by 32, label Label Medium, 4 under the indicator | 80 high |
| Navigation rail | 96 wide (80 narrow), items as in the bar | 80 |
| Top app bar | 64 high; title Title Large; icons 24 in 40; 4 at the sides | 56 |
| Large top app bar | 120 (152 with a subtitle), title Display Small | 152 with Headline Medium |
| Chip | 32 high, corner 8, icon 18, Label Large | A pill |
| Card | Corner 12 | 4 or 8 |
| Text field | 56 high, corner 4, Body Large | 48 |
| Switch | Track 52 by 32; handle 24 on, 16 off | 34 by 14 with a thumb that overhangs |
| Checkbox, radio | Box 18 with corner 2; radio 20; both inside 40 | |
| Slider | Track 16 high, handle 4 by 44 with 6 beside it | A thin line with a round thumb |
| List item | 56, 72 or 88 high for one, two or three lines; 16 at the sides | 48 |
| Dialog | Corner 28, level 3, headline Headline Small | Corner 4 |
| Snackbar | 48 high (68 with two lines), corner 4, inverse colours | |
| Search bar | 56 high, a pill | 48 with corner 4 |
| Tabs | 48 high, indicator 3 thick | 2 |

Paddings the token files do not carry (inside a card, a dialog, between a screen's edge and its content)
are this kit's: 16 at a phone's edges and inside cards, 24 inside a dialog and at the edges from a tablet
up. Say so when a user asks where they come from.

## 6. The screen

- One unit is one dp. A phone is 412 by 915 (a Pixel; a small one 360 by 800).
- The app draws edge to edge: its ground runs behind the status bar on top and the gesture handle at the
  bottom. Android publishes no heights for them. This kit draws the status bar 36 high (the time at the
  leading side, signal and battery at the trailing) and the gesture area 24 high with a handle of 108 by 4,
  on the colour of whatever is behind them, never on a bar of their own.
- By width: under 600 (a phone upright) a navigation bar at the bottom; 600 to 839 (a small tablet, an
  unfolded phone) a navigation rail; from 840 a rail or a drawer, and two panes side by side; from 1200
  the same with more room. Three to five destinations in a bar.
- From the top: status bar, top app bar, content, navigation bar, gesture area. A FAB floats 16 from the
  trailing edge and 16 over the navigation bar. A snackbar stands 16 over the bar or the FAB's baseline,
  never over the FAB.
- Around the phone itself, if the user wants the device: the `devices` skill.

`references/components.md` ends with a whole screen built from the components.

## 7. Icons

Material's own icons (Material Symbols) are not in Pencraft; the Lucide set stands in. Use one size per
role (24 in bars, lists and buttons of 56; 20 in buttons of 32 and 40; 18 in chips) and tell the user in
one line that the icons are stand-ins. Material fills the icon of the active destination; Lucide has
outlines only, so the indicator pill and the label carry the active state.

## 8. Workflow

1. `get_state`, `read_layers`: which variables and components are there.
2. The roles (`references/tokens.md`), once; the scheme for the brand colour if there is one.
3. Insert the components the request needs from `references/components.md`, as reusable components on a
   board named "Material components" well to the left of the work; ids start with `m3-`. If one is there
   already, instance it.
4. Compose the work with instances and overrides. Pick states with `"state"`.
5. `screenshot` with the work's `id`, put right what the review counted, then go through the checklist.
6. Tell the user in two sentences what is there, that the icons are Lucide, and anything they asked for
   that Pencraft cannot draw.

## 9. Checklist

- No hex in a layer; every text on its surface's own "on" role.
- Every text is one of the fifteen roles.
- Heights: buttons 40 (or the size chosen for all of a row), bars 64, list rows 56, 72 or 88.
- One FAB at most; one filled button a screen for the main action, the others tonal, outlined or text.
- Shadows only on what floats; no line and shadow together.
- The ground runs under the status bar and the gesture handle.
- In dark: the dark values of the roles, surfaces lighter as they rise, no pure black page.

## 10. What Pencraft cannot draw, and what to do instead

| Missing | Instead |
|---|---|
| Material Symbols, filled or with their weights | Lucide; the active state by pill and label |
| A scheme worked out from a seed colour | The tone table and a scheme by eye, said as such |
| Shape morphing, ripples, the wavy progress bar in motion | One frame per state; the pressed shape drawn |
| The font's width and roundness axes (Google Sans Flex) | The family at its weight |
| Text cut off with an ellipsis | The shortened text written with "…", at a fixed width |

## 11. When it looks wrong

| It looks like | Cause | Fix |
|---|---|---|
| An iPhone app in purple | A tab bar with a blur, thin separators, centred titles, pills everywhere | A navigation bar with indicator pills, tonal surfaces, leading titles |
| Material from 2018 | Capitals on buttons, a round FAB, shadows on every card, an 80 high bar, a coloured app bar | Sentence case, corner 16, tonal cards, 64, `$surface` |
| A web page | 36 high buttons, 14 body text, hairlines round everything | 40 and 56, Body Large, tone instead of lines |
| Flat and grey | Only `surface` and one accent | The container steps and the secondary container |
| Garish | Primary on every surface | Primary for the one action and what is active; containers for the rest |
