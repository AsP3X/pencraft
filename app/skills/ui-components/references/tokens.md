# Tokens for standard components

Set these up once per document with one `edit_layers` call, then use them everywhere. If the document already
has variables for the same roles, use those and add only the missing ones. Before you send the call, change
the value of `font` to a family that is installed (section 2 of the skill says which), and `font-mono`, the
face of code, to an installed fixed-width one (Cascadia Mono or Consolas on Windows, SF Mono or Menlo on a
Mac, otherwise DejaVu Sans Mono).

## The variables

```json
{"op":"set_variables",
 "themes":{"Mode":["Light","Dark"]},
 "variables":{
  "background":{"type":"color","value":[{"value":"#ffffff"},{"value":"#0a0a0a","theme":{"Mode":"Dark"}}]},
  "foreground":{"type":"color","value":[{"value":"#0a0a0a"},{"value":"#fafafa","theme":{"Mode":"Dark"}}]},
  "card":{"type":"color","value":[{"value":"#ffffff"},{"value":"#171717","theme":{"Mode":"Dark"}}]},
  "popover":{"type":"color","value":[{"value":"#ffffff"},{"value":"#171717","theme":{"Mode":"Dark"}}]},
  "ground":{"type":"color","value":[{"value":"#f5f5f5"},{"value":"#0a0a0a","theme":{"Mode":"Dark"}}]},
  "sidebar":{"type":"color","value":[{"value":"#fafafa"},{"value":"#171717","theme":{"Mode":"Dark"}}]},
  "primary":{"type":"color","value":[{"value":"#171717"},{"value":"#e5e5e5","theme":{"Mode":"Dark"}}]},
  "primary-foreground":{"type":"color","value":[{"value":"#fafafa"},{"value":"#171717","theme":{"Mode":"Dark"}}]},
  "primary-hover":{"type":"color","value":[{"value":"#171717cc"},{"value":"#e5e5e5cc","theme":{"Mode":"Dark"}}]},
  "secondary":{"type":"color","value":[{"value":"#f5f5f5"},{"value":"#262626","theme":{"Mode":"Dark"}}]},
  "secondary-hover":{"type":"color","value":[{"value":"#e8e8e8"},{"value":"#333333","theme":{"Mode":"Dark"}}]},
  "muted":{"type":"color","value":[{"value":"#f5f5f5"},{"value":"#262626","theme":{"Mode":"Dark"}}]},
  "muted-half":{"type":"color","value":[{"value":"#f5f5f580"},{"value":"#26262680","theme":{"Mode":"Dark"}}]},
  "muted-foreground":{"type":"color","value":[{"value":"#5c5c5c"},{"value":"#a8a8a8","theme":{"Mode":"Dark"}}]},
  "accent":{"type":"color","value":[{"value":"#f5f5f5"},{"value":"#262626","theme":{"Mode":"Dark"}}]},
  "destructive":{"type":"color","value":[{"value":"#c40d1f"},{"value":"#ff6467","theme":{"Mode":"Dark"}}]},
  "destructive-tint":{"type":"color","value":[{"value":"#c40d1f1a"},{"value":"#ff646733","theme":{"Mode":"Dark"}}]},
  "destructive-tint-hover":{"type":"color","value":[{"value":"#c40d1f26"},{"value":"#ff646740","theme":{"Mode":"Dark"}}]},
  "border":{"type":"color","value":[{"value":"#e5e5e5"},{"value":"#ffffff1a","theme":{"Mode":"Dark"}}]},
  "input":{"type":"color","value":[{"value":"#d4d4d4"},{"value":"#ffffff26","theme":{"Mode":"Dark"}}]},
  "control":{"type":"color","value":[{"value":"#ffffff00"},{"value":"#ffffff0b","theme":{"Mode":"Dark"}}]},
  "hairline":{"type":"color","value":[{"value":"#0a0a0a1a"},{"value":"#fafafa1a","theme":{"Mode":"Dark"}}]},
  "ring":{"type":"color","value":[{"value":"#a1a1a1"},{"value":"#737373","theme":{"Mode":"Dark"}}]},
  "ring-soft":{"type":"color","value":[{"value":"#a1a1a180"},{"value":"#73737380","theme":{"Mode":"Dark"}}]},
  "scrim":{"type":"color","value":[{"value":"#00000033"},{"value":"#00000099","theme":{"Mode":"Dark"}}]},
  "radius-xs":{"type":"number","value":4},
  "radius-sm":{"type":"number","value":6},
  "radius-md":{"type":"number","value":8},
  "radius-lg":{"type":"number","value":10},
  "radius-xl":{"type":"number","value":14},
  "font":{"type":"string","value":"Inter"},
  "font-mono":{"type":"string","value":"Consolas"}}}
```

If the document already has a `Mode` axis, keep its values and leave `themes` out.

## What each is for

A surface and the colour written on it come as a pair. Use the pair; never put `$muted-foreground` on
`$primary`.

| Variable | On it goes | Use |
|---|---|---|
| `background` | `foreground` | The page; an outline button; the knob of a switch |
| `ground` | `foreground` | The page of a screen whose content stands in cards (a dashboard, a sign-in page): a shade under the cards in light, where a card would otherwise have the page's own white; the page itself in dark |
| `card` | `foreground` | A card or panel on the page |
| `sidebar` | `foreground` | A sidebar and the list pane beside it: a shade off the page |
| `popover` | `foreground` | Everything that floats: menu, select list, popover, dialog, sheet, toast |
| `primary` | `primary-foreground` | The one main action; a checked checkbox, radio, switch; the filled part of a slider or progress bar |
| `primary-hover` | `primary-foreground` | `primary` under the pointer (80%) |
| `secondary` | `foreground` | A filled action of less weight; a secondary badge |
| `secondary-hover` | `foreground` | `secondary` under the pointer |
| `muted` | `foreground` or `muted-foreground` | Quiet surfaces: a tab list, a skeleton, a track, an avatar without a picture, a kbd |
| `muted-half` | `foreground` | A table row or list row under the pointer; a footer band |
| `muted-foreground` | - | Descriptions, placeholders, group labels, shortcuts, icons that only accompany |
| `accent` | `foreground` | The row of a menu the pointer is on; a ghost button under the pointer |
| `destructive` | - | The text and icon of a destructive action and of an error; an invalid border |
| `destructive-tint` | `destructive` | The fill of a destructive button or badge; the halo of an invalid control |
| `destructive-tint-hover` | `destructive` | A destructive button under the pointer: only a little stronger than the tint, or its label would no longer hold 4.5:1 |
| `border` | - | Layout lines: table rows, separators, an alert's outline, an outline button |
| `input` | - | The border of a form control; the track of a switch that is off |
| `control` | `foreground` | The fill of a form control: clear in light, a little lighter than the page in dark |
| `hairline` | - | The 1 wide line around what floats, and around a card |
| `ring`, `ring-soft` | - | Focus: the border, and the 3 wide halo around it |
| `scrim` | - | The rectangle between a dialog and the page: black at 20%, and at 60% in the dark theme, where 20% on a near-black page cannot be seen |

Red, amber and green mean states. A success or warning colour is added the same way as `destructive` (a
strong colour for text, a tint of it for the fill) when the design needs one; do not add them unused.

To give the kit an accent, change `primary` and `primary-foreground` (and `primary-hover`, `ring`) and check
that the pair holds 4.5:1. Leave the greys neutral or tint all of them the same slight way.

## Radius

One base, 10, and steps of it: 0.6, 0.8, 1 and 1.4 times.

| Variable | Value | For |
|---|---|---|
| `radius-xs` | 4 | Checkbox, a kbd, the action inside a toast |
| `radius-sm` | 6 | A row inside a menu; a chip |
| `radius-md` | 8 | Buttons, inputs, selects, menus, popovers, tooltips |
| `radius-lg` | 10 | Tab lists, alerts, toasts, the Compact preset's controls |
| `radius-xl` | 14 | Cards, dialogs, a command palette |

A pill is half the height (a badge of 20 has 10, a switch of 18 has 9). To make the whole kit rounder or
squarer, change the five numbers together: base 7 gives 3, 4, 6, 7, 10; base 14 gives 6, 8, 11, 14, 20.

## Elevation

An effect cannot be a variable (its colour can), so these are written out. They get their softness from two
layers and need no `spread`; the one shadow here that spreads is the focus ring (`references/controls.md`).

Faintest, for a card on the page and the selected tab:

```json
{"type":"rectangle","name":"Elevation, faint","width":120,"height":64,"cornerRadius":"$radius-xl","fill":"$card","stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","effect":{"type":"shadow","offset":{"x":0,"y":1},"blur":2,"color":"#0000000d"}}
```

Floating, for menus, select lists, popovers, hover cards:

```json
{"type":"rectangle","name":"Elevation, floating","width":120,"height":64,"cornerRadius":"$radius-md","fill":"$popover","stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"color":"#0000000f"},{"type":"shadow","offset":{"x":0,"y":6},"blur":14,"color":"#00000014"}]}
```

Large, for a toast, a sheet, a sub-menu:

```json
{"type":"rectangle","name":"Elevation, large","width":120,"height":64,"cornerRadius":"$radius-lg","fill":"$popover","stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","effect":[{"type":"shadow","offset":{"x":0,"y":2},"blur":6,"color":"#0000000f"},{"type":"shadow","offset":{"x":0,"y":12},"blur":28,"color":"#0000001f"}]}
```

A dialog needs no shadow of its own: the scrim separates it. The scrim is one rectangle as large as the
screen, under the dialog and over everything else. It has no blur: in a still picture a blurred page reads
as a fault, not as depth.

```json
{"type":"rectangle","name":"Scrim","width":1280,"height":800,"fill":"$scrim"}
```

## Spacing

Steps of 4, each with a job:

| Step | Job |
|---|---|
| 4 | Title to its description; a menu's own padding; between a menu's rows there is none |
| 6 | An icon to its label in a button |
| 8 | An icon to its label in a row; label to control; control to its description; between two buttons |
| 12 | Between a checkbox, radio or switch row and the next |
| 16 | Popover padding; between an avatar and its text; between cards in a tight grid |
| 24 | Card and dialog padding and the gap between header, content and footer; between fields |
| 32 | Between sections of a page |

## Where these numbers come from

shadcn/ui's neutral theme and its classic style, read from its source in November 2026, with three changes
for Pencraft: `muted-foreground` is one step darker than shadcn's `#737373`, so that 12 px labels hold 6:1 on
white and on the muted grey; `input` in the light theme is one step darker than `border`, so a field's edge can be seen;
`destructive` is deepened so its text holds 4.5:1 on its own tint. The hex values are conversions of
shadcn's OKLCH values and are approximate. `references/styles.md` has the exact shadcn values for when exact
fidelity is asked for.
