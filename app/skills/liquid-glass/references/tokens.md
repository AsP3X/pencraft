# Tokens for glass designs

Set these up once per document with one `edit_layers` call (`set_variables`), then use them everywhere. Reuse
variables the document already has for the same purpose instead of adding near-duplicates.

## The variables

```json
{"op":"set_variables",
 "themes":{"Mode":["Light","Dark"],"Glass":["Clear","Tinted","Reduced transparency","Increased contrast"]},
 "variables":{
  "on-glass":{"type":"color","value":[{"value":"#000000"},{"value":"#ffffff","theme":{"Mode":"Dark"}}]},
  "selection-on-glass":{"type":"color","value":[{"value":"#0000001f"},{"value":"#ffffff24","theme":{"Mode":"Dark"}}]},
  "on-light-glass":{"type":"color","value":"#000000"},
  "on-dark-glass":{"type":"color","value":"#ffffff"},
  "accent":{"type":"color","value":"#0060df"},
  "glass-outline":{"type":"color","value":[{"value":"#00000000"},{"value":"#000000","theme":{"Glass":"Increased contrast"}},{"value":"#ffffff","theme":{"Glass":"Increased contrast","Mode":"Dark"}}]},
  "glass-regular-tint-opacity":{"type":"number","value":[
    {"value":0.6,"theme":{"Glass":"Tinted"}},
    {"value":0.9,"theme":{"Glass":"Reduced transparency"}},
    {"value":1,"theme":{"Glass":"Increased contrast"}}]},
  "glass-regular-blur":{"type":"number","value":[
    {"value":40,"theme":{"Glass":"Reduced transparency"}},
    {"value":40,"theme":{"Glass":"Increased contrast"}}]},
  "glass-regular-refraction":{"type":"number","value":[
    {"value":0.4,"theme":{"Glass":"Reduced transparency"}},
    {"value":0,"theme":{"Glass":"Increased contrast"}}]}}}
```

- `on-glass`: the label and icon colour on glass, following the screen: black in a light screen, white in a
  frame with `"theme":{"Mode":"Dark"}`. The components use it, so a dark screen needs no overrides. If the
  document already has a `Mode` axis, keep its values and leave `Mode` out of `themes`.
- `selection-on-glass`: the fill of a selected tab, segment or row, following the screen the same way.
- `on-light-glass` / `on-dark-glass`: fixed black and white, for the single layer whose glass came out the
  other way than its screen (a bar over a dark photo in a light screen). Which glass a layer has is reported
  by `check_glass` (`appearance`). Over photos, put the screen (or the bar's instance) in `Mode` Dark or
  override to `$on-dark-glass`.
- `accent`: the tint of the primary action and the selected tab's icon. It must hold 4.5:1 against white.
- `glass-outline`: put `"stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner"` on glass
  components; it is invisible except in the Increased contrast look.
- The three `glass-regular-...` variables have **no** plain value on purpose: outside the themes they name,
  each layer keeps the number its size gives it. A plain value would make buttons, bars and panels all the
  same thickness.

## The four looks

The `Glass` theme axis is the system-wide setting a person can choose. A frame with
`"theme":{"Glass":"Tinted"}` shows everything in it in that look; to present the looks side by side, place
copies (or instances) of the screen in frames that differ only in `theme`.

| Look | What changes | Why |
|---|---|---|
| Clear (default) | nothing | The material as designed |
| Tinted | more of the glass's own white or dark over the backdrop | More opacity and contrast for people who find clear glass hard to read |
| Reduced transparency | much frostier and nearly opaque, hardly any bending | Still a material, but the backdrop no longer competes with the labels |
| Increased contrast | a solid white or black surface with a contrasting 1-unit border | Maximum separation; the label colour stays the opposite of the surface |

A document with a `Mode` axis (Light, Dark) keeps it; the two axes combine.

Three things to check when you build the looks:
- A layer's own `advanced` beats the look's variables. A bar with `advanced.tintOpacity` stays as it is in
  every look: remove that key in the copies (or avoid it in the first place; `"depth":"panel"` gives a more
  opaque glass without blocking the looks).
- `check_glass` lists each material's `tintOpacity` as it came out: in the Tinted copy it should read 0.6, in
  Reduced transparency 0.9, in Increased contrast 1. If a copy shows the original's number, something on
  the layer overrides the look.
- `glass-outline` is black in a light screen and white in a `Mode` Dark one. A bar whose glass came out dark
  inside a light screen (over a dark photo) needs its `stroke` overridden to `$on-dark-glass` in the
  Increased contrast copy, like its labels.

## Retuning a preset

A number of a material comes from, in this order: the layer's own `advanced`, a document variable named
after the preset, the preset itself. Variable names are `<preset>-<number>`:

- presets: `glass-regular`, `glass-clear`, `frosted-ultra-thin`, `frosted-thin`, `frosted-regular`,
  `frosted-thick`
- numbers: `blur`, `saturation`, `brightness`, `tint-opacity`, `refraction`, `bevel`, `thickness`,
  `dispersion`, `highlight`, `light-angle`, `shadow`

`glass-light-angle` sets the light for every glass layer of the document (degrees clockwise from straight
above; -45, the upper left, by default). Keep one light per document.

## The numbers (`advanced`)

Only when a preset is wrong for one layer. Keys are camelCase in `advanced`.

| Key | Range | Regular glass (control / bar / panel) | Clear glass | Frosted (ultra thin ... thick) |
|---|---|---|---|---|
| `blur` | 0-200 | 14 / 20 / 36 | 2 / 3 / 5 | 24 / 40 / 60 / 80 |
| `saturation` | 0-3 | 1.4 | 1.15 | 1.6 / 1.7 / 1.8 / 1.8 |
| `brightness` | -0.5-0.5 | 0.03 light, -0.01 dark | 0 | 0.02 |
| `tintOpacity` | 0-1 | light 0.34 / 0.42 / 0.62, dark 0.50 / 0.58 / 0.76 | 0.02 / 0.03 / 0.05 | 0.20 / 0.35 / 0.55 / 0.75 |
| `refraction` | 0-3 | 1 | 1 | - |
| `bevel` | 0-200 | 14 / 20 / 26 (never more than half the short side) | same | - |
| `thickness` | 0-200 | 22 / 30 / 40 | same | - |
| `dispersion` | 0-1 | 0 | 0 | - |
| `highlight` | 0-1 | 0.6 light, 0.42 dark | same | 0.16 light, 0.09 dark |
| `lightAngle` | -360-360 | -45 | -45 | -45 |
| `shadow` | 0-1 | 0.12 / 0.16 / 0.22 | same | 0 |

`blur` means what a `background_blur` effect's `radius` means. `refraction` 0 turns glass into a flat pane;
lowering it step by step in separate frames shows a control appearing ("materializing"). `dispersion` splits
colours at the rim; keep it at or near 0 for interfaces.

## Sizes that fit this design language

Desktop window 1280 x 800 or larger, radius 24: toolbar controls 32-36 high, sidebar rows 32 high with 13 px
labels and 16 px icons, sidebar 240-260 wide and inset 8, menus 220-260 wide with 32-high rows. Window
controls are three neutral dots (`#ffffff47` in a dark window, `#0000002e` in a light one), never a
platform's own artwork. Choose insets so that nested radii land on few values (for example 24, 16, 8).

Phone screen 393 x 852. Margins 16; floating bars 16-21 from the edges. Bar buttons 44 x 44 (round) or 44
high (capsule). Tab bar 62 high, tab label 11 px (never smaller: Pencraft's review flags text under 11).
Titles 34 bold (large) or 17 semibold (inline); body 17; secondary 15; captions 13. Sheet radius 34-40, alert
radius 30, menu radius 22, cards in content 20-28. Icons 22-24 from the `lucide` library.
