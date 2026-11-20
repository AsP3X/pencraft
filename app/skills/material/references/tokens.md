# Material roles, type, shape and elevation

Set the roles once per document with one `edit_layers` call. These are the baseline scheme's values (the
purple one every Material kit starts from), for light and dark. If the document already has a `Mode` axis,
leave `themes` out.

```json
{"op":"set_variables",
 "themes":{"Mode":["Light","Dark"]},
 "variables":{
  "primary":{"type":"color","value":[{"value":"#6750a4"},{"value":"#d0bcff","theme":{"Mode":"Dark"}}]},
  "on-primary":{"type":"color","value":[{"value":"#ffffff"},{"value":"#381e72","theme":{"Mode":"Dark"}}]},
  "primary-container":{"type":"color","value":[{"value":"#eaddff"},{"value":"#4f378b","theme":{"Mode":"Dark"}}]},
  "on-primary-container":{"type":"color","value":[{"value":"#21005d"},{"value":"#eaddff","theme":{"Mode":"Dark"}}]},
  "secondary":{"type":"color","value":[{"value":"#625b71"},{"value":"#ccc2dc","theme":{"Mode":"Dark"}}]},
  "on-secondary":{"type":"color","value":[{"value":"#ffffff"},{"value":"#332d41","theme":{"Mode":"Dark"}}]},
  "secondary-container":{"type":"color","value":[{"value":"#e8def8"},{"value":"#4a4458","theme":{"Mode":"Dark"}}]},
  "on-secondary-container":{"type":"color","value":[{"value":"#1d192b"},{"value":"#e8def8","theme":{"Mode":"Dark"}}]},
  "tertiary":{"type":"color","value":[{"value":"#7d5260"},{"value":"#efb8c8","theme":{"Mode":"Dark"}}]},
  "on-tertiary":{"type":"color","value":[{"value":"#ffffff"},{"value":"#492532","theme":{"Mode":"Dark"}}]},
  "tertiary-container":{"type":"color","value":[{"value":"#ffd8e4"},{"value":"#633b48","theme":{"Mode":"Dark"}}]},
  "on-tertiary-container":{"type":"color","value":[{"value":"#31111d"},{"value":"#ffd8e4","theme":{"Mode":"Dark"}}]},
  "error":{"type":"color","value":[{"value":"#b3261e"},{"value":"#f2b8b5","theme":{"Mode":"Dark"}}]},
  "on-error":{"type":"color","value":[{"value":"#ffffff"},{"value":"#601410","theme":{"Mode":"Dark"}}]},
  "error-container":{"type":"color","value":[{"value":"#f9dedc"},{"value":"#8c1d18","theme":{"Mode":"Dark"}}]},
  "on-error-container":{"type":"color","value":[{"value":"#410e0b"},{"value":"#f9dedc","theme":{"Mode":"Dark"}}]},
  "surface":{"type":"color","value":[{"value":"#fef7ff"},{"value":"#141218","theme":{"Mode":"Dark"}}]},
  "on-surface":{"type":"color","value":[{"value":"#1d1b20"},{"value":"#e6e0e9","theme":{"Mode":"Dark"}}]},
  "on-surface-variant":{"type":"color","value":[{"value":"#49454f"},{"value":"#cac4d0","theme":{"Mode":"Dark"}}]},
  "surface-container-lowest":{"type":"color","value":[{"value":"#ffffff"},{"value":"#0f0d13","theme":{"Mode":"Dark"}}]},
  "surface-container-low":{"type":"color","value":[{"value":"#f7f2fa"},{"value":"#1d1b20","theme":{"Mode":"Dark"}}]},
  "surface-container":{"type":"color","value":[{"value":"#f3edf7"},{"value":"#211f26","theme":{"Mode":"Dark"}}]},
  "surface-container-high":{"type":"color","value":[{"value":"#ece6f0"},{"value":"#2b2930","theme":{"Mode":"Dark"}}]},
  "surface-container-highest":{"type":"color","value":[{"value":"#e6e0e9"},{"value":"#36343b","theme":{"Mode":"Dark"}}]},
  "outline":{"type":"color","value":[{"value":"#79747e"},{"value":"#938f99","theme":{"Mode":"Dark"}}]},
  "outline-variant":{"type":"color","value":[{"value":"#cac4d0"},{"value":"#49454f","theme":{"Mode":"Dark"}}]},
  "inverse-surface":{"type":"color","value":[{"value":"#322f35"},{"value":"#e6e0e9","theme":{"Mode":"Dark"}}]},
  "inverse-on-surface":{"type":"color","value":[{"value":"#f5eff7"},{"value":"#322f35","theme":{"Mode":"Dark"}}]},
  "inverse-primary":{"type":"color","value":[{"value":"#d0bcff"},{"value":"#6750a4","theme":{"Mode":"Dark"}}]},
  "scrim":{"type":"color","value":[{"value":"#00000052"},{"value":"#00000052","theme":{"Mode":"Dark"}}]},
  "font":{"type":"string","value":"Roboto"}}}
```

Read from the colour, palette, type, shape, elevation and state token files of Jetpack Compose's Material 3
library (the Expressive versions). Five values (`on-secondary-container` and `on-secondary`, and the
tertiary containers and their "on" colours) are the baseline scheme's as widely published, not re-read at
the source for this skill.

## Roles and their tones

A tone is a lightness from 0 (black) to 100 (white) within one palette. P is the primary palette, S
secondary, T tertiary, E error, N neutral, NV neutral variant.

| Role | Light | Dark | For |
|---|---|---|---|
| `primary` / `on-primary` | P40 / P100 | P80 / P20 | The main action, the active state |
| `primary-container` / `on-primary-container` | P90 / P10 | P30 / P90 | A FAB, a highlighted block |
| `secondary` / `on-secondary` | S40 / S100 | S80 / S20 | Quieter accents |
| `secondary-container` / `on-secondary-container` | S90 / S10 | S30 / S90 | A tonal button, a selected chip, the navigation indicator |
| `tertiary` / `on-tertiary`, and its container pair | T40 / T100, T90 / T10 | T80 / T20, T30 / T90 | A contrasting touch |
| `error` / `on-error`, and its container pair | E40 / E100, E90 / E10 | E80 / E20, E30 / E90 | Errors |
| `surface` / `on-surface` | N98 / N10 | N6 / N90 | The page and its text |
| `on-surface-variant` | NV30 | NV80 | Supporting text, icons |
| `surface-container-lowest` to `-highest` | N100, N96, N94, N92, N90 | N4, N10, N12, N17, N22 | Surfaces on the page, by how much they stand out |
| `outline` / `outline-variant` | NV50 / NV80 | NV60 / NV30 | A border that must be seen / a divider |
| `inverse-surface`, `inverse-on-surface`, `inverse-primary` | N20, N95, P80 | N90, N20, P40 | A snackbar |
| `scrim` | Black at 32% | Black at 32% | Under a dialog, sheet or drawer |

A scheme for another colour gives each role the same tone in its own palettes. In practice: take the brand's
hue; `primary` is that hue at lightness 40 (80 in dark), strong; its container the same hue at 90 (30),
pale; the secondary palette the same hue at about a third of the strength; the neutrals the same hue with
only a breath of colour, so greys lean toward the brand. Check `on-` pairs: each must read 4.5:1 on its
surface, which the tone gaps of 50 and more give by themselves.

## Elevation as shadows

Levels are 0, 1, 3, 6, 8 and 12 dp. A picture needs them as shadows; these are the ones Material's own
web components draw (written from memory of that library, not re-read): a tight one and a soft one that
spreads.

Level 1, an elevated card or chip:

```json
{"type":"rectangle","name":"Level 1","width":120,"height":64,"cornerRadius":12,"fill":"$surface-container-low","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":2,"color":"#0000004d"},{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"spread":1,"color":"#00000026"}]}
```

Level 2, a menu, a top app bar over content:

```json
{"type":"rectangle","name":"Level 2","width":120,"height":64,"cornerRadius":4,"fill":"$surface-container","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":2,"color":"#0000004d"},{"type":"shadow","offset":{"x":0,"y":2},"blur":6,"spread":2,"color":"#00000026"}]}
```

Level 3, a FAB, a dialog, a snackbar, a search bar:

```json
{"type":"rectangle","name":"Level 3","width":120,"height":64,"cornerRadius":16,"fill":"$surface-container-high","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"color":"#0000004d"},{"type":"shadow","offset":{"x":0,"y":4},"blur":8,"spread":3,"color":"#00000026"}]}
```

In dark the shadows are nearly invisible on the dark page; the lighter container is what lifts a surface.
Keep them anyway: they are right, and they show on lighter content.

## States

| State | Layer over the container, in the content's colour |
|---|---|
| Under the pointer | 8% |
| Focused | 10% |
| Pressed | 10%, and a button takes its pressed corner |
| Dragged | 16% |
| Disabled | Container `$on-surface` at 10%, content `$on-surface` at 38% |

A layer is a second paint in the fill: `["$secondary-container", {"type":"color","color":"$on-secondary-container","opacity":0.08}]`.
On a container without a fill (a text button, an icon button) the layer is the only paint.

## Window size classes

| Class | Width in dp | What it is | Navigation |
|---|---|---|---|
| Compact | Under 600 | A phone upright | Navigation bar |
| Medium | 600 to 839 | A small tablet upright, an unfolded phone | Navigation rail |
| Expanded | 840 to 1199 | A tablet lying down | Rail or drawer; two panes |
| Large | 1200 to 1599 | A large tablet, a window on a desktop | The same |
| Extra large | 1600 and up | A desktop | The same, three panes at most |

By height: compact under 480 (a phone lying down), medium 480 to 899, expanded from 900. The widths are
Android's own; the navigation for each is the common mapping.
