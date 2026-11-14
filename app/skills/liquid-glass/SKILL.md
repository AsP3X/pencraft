---
name: liquid-glass
description: Design Liquid Glass interfaces in Pencraft - the translucent, refractive material of iOS 26, macOS 26 and visionOS for bars and controls that float above content - and frosted panels for structure inside content. Use when the user asks for liquid glass, glass UI, glassmorphism, frosted or translucent panels, an iOS 26 / macOS 26 / visionOS look, a floating glass tab bar, toolbar, sidebar, sheet or menu, or says "make this look like glass". Covers which surfaces get glass, the `material` property, glass groups, legibility, tokens, components and the check_glass tool.
license: Apache-2.0 with Commons Clause (see LICENSE in the Pencraft repository)
metadata:
  product: pencraft
  version: "1"
---

# Liquid glass in Pencraft

Glass is a material, not a colour: a layer made of it shows what is **behind** it, blurred, and bends it at
the rim like a thick lens. It has no look of its own. Over a flat background it is a grey box; over a photo,
a coloured header or a scrolling feed it reads as glass. So every decision here starts from what is behind.

This is Pencraft's own glass material in the style Apple introduced in 2025. It is not Apple's system
material and does not match it pixel for pixel; never say it does, and never use Apple's icons or wallpapers.

Work with Pencraft's tools (`get_state`, `get_guide`, `read_layers`, `edit_layers`, `screenshot`,
`check_glass`). The reference files named below are read with `read_skill` (name `liquid-glass`, `file` as
given), or from this folder when you have it on disk.

## 1. Decide what each surface is made of

Every screen has two layers. Sort each surface into one before you draw it.

| The surface is | It gets | Why |
|---|---|---|
| Navigation or a control floating above content: tab bar, toolbar button, floating button, sidebar, sheet, popover, menu, alert | `"material": "liquid_glass"` | Bending at its rim is how the eye reads it as a separate plane above the content |
| Structure inside the content: a panel, a card over a picture, a grouped box | `"material": "frosted"` | It needs separation, but lensing inside content is noise |
| Content itself: lists, text, pictures, feed cards, table rows | no material; ordinary fills | Glass content has nothing left to float above |

Keep glass to the few elements that carry navigation and the main actions. "Glass everywhere" is the most
common failure.

Then the variant of liquid glass, one per screen, never both:
- **regular** (the default): blurs and evens out what is behind; legible over anything. Use it whenever
  there is text on the glass (bars with labels, sidebars, menus, alerts).
- **clear** (`"variant": "clear"`): nearly see-through. Only when all three hold: it lies over a photo or
  video; a dimming layer over that media does no harm; what is on the glass is bold and bright (white icons;
  keep text off clear glass). Over bright media put a `#00000059` (35 % black) rectangle between the media
  and the glass. That is a starting value: where `check_glass` still fails, darken locally under that control
  (a gradient scrim, up to about 60 %) instead of darkening the whole picture. Dimming dulls bright artwork;
  if the user wants the picture to stay brilliant, say so and use regular glass with dark labels instead.

## 2. Make sure there is something to bend

Before placing glass, look at what will be under it. Content runs **behind** bars, edge to edge: a feed
scrolls under the tab bar and the toolbar, a hero picture continues under the sidebar. Do not give a bar a
strip of plain background to sit on, and do not put padding in the content frame where the bars are (pad the
last row instead). If the screen really is flat, say so and use an ordinary fill, or give the content layer
imagery or colour first. Pencraft's general guide tells you to end a list above a bar and not to let a bar
cut a row; under glass bars that rule is reversed on purpose, and the review's remark about it can stand.
Over a plain text feed, glass has little to bend and reads mostly by its rim and shadow; that is correct,
but let a picture, a coloured header or a card pass under at least one bar when the content has one.

## 3. Apply the material

`material` goes on a `frame`, `rectangle` or `ellipse`. The material is the layer's background: leave `fill`
off, and put labels and icons in it as children; they are drawn sharp on top.

```json
{"type":"frame","name":"Tab bar","height":62,"cornerRadius":31,"material":"liquid_glass","children":[...]}
{"type":"frame","name":"Done","width":44,"height":44,"cornerRadius":22,"material":{"type":"liquid_glass","tint":"$accent"}}
{"type":"frame","name":"Details","cornerRadius":20,"material":{"type":"frosted","thickness":"thin"}}
```

| Key | Values | Use |
|---|---|---|
| `type` | `"liquid_glass"`, `"frosted"` | A bare string is enough when nothing else is set |
| `variant` (glass) | `"regular"`, `"clear"` | Section 1 |
| `thickness` (frosted) | `"ultra_thin"`, `"thin"`, `"regular"`, `"thick"` | Thicker hides more and carries fine text better |
| `appearance` | `"auto"`, `"light"`, `"dark"` | Section 4 |
| `tint` | a colour or `"$variable"` | Only the one primary action (Done, Send), or a status. Tint the glass, never the label. Pick a colour that differs in hue from what is behind the bar, or the tinted button will not stand out |
| `depth` (glass) | `"control"`, `"bar"`, `"panel"` | How thick the glass behaves. It follows the layer's size; set it when a small layer belongs to a bar (a tab bar's search button: `"bar"`) |
| `advanced` | numbers such as `blur`, `refraction`, `highlight` | Escape hatch only. If you reach for it, first check the choices above; `references/tokens.md` lists the numbers |

Shadows come with the material. Do not add `background_blur` or shadow effects to a layer that has one.
An `update` replaces the whole `material` object (also as an instance override): send every key again.

## 4. Decide what a live system would adapt

A phone decides these while you scroll. A design is one frozen moment, so you decide them once.

- **Light or dark glass.** `auto` picks per layer from what is behind it: light glass over light content,
  dark glass over dark content. Small elements (bar buttons, tab bars) stay on `auto`. Large panes (sidebar,
  sheet, menu) do not flip: pin them to the screen's theme with `"appearance":"light"` or `"dark"`.
- **Foreground opposite to the glass.** Labels and icons on glass are one colour: near black on light glass,
  white on dark glass. You cannot guess which glass `auto` chose; `check_glass` tells you (`appearance` per
  layer), and it measures the result. Start with `$on-glass` (black in a light screen, white in a dark one;
  white over photos), then correct single layers from what `check_glass` reports.
- **Thickness follows size.** Buttons are thin glass, bars medium, sheets and sidebars thick and more opaque.
  This is automatic (`depth`); do not flatten it with `advanced`.
- **Other states are separate frames.** Pressed, morphing or appearing glass is shown as another frame beside
  the screen, not animated: for a control that is being pressed (a slider knob, a toggle) show the knob as
  glass in that frame and plain at rest.

## 5. Shape

Three shapes build everything; pick one per layer and compute it, do not eyeball it.

- **Capsule**: `cornerRadius` = half the height. Bars, buttons, search fields, tab selections.
- **Concentric**: a shape inside a rounded shape has radius = outer radius - the gap between their edges. A
  54-high selection 4 inside a 62-high tab bar has radius 31 - 4 = 27. A capsule nests anywhere.
  `check_glass` flags corners that do not nest.
- **Fixed**: one radius for free-standing rectangles (cards 20-28, sheets 34-40).

Glass floats: keep it off the screen's edges. On a phone a bar is a capsule 16-21 from the left, right and
bottom edge, not an edge-to-edge strip. Touch targets are at least 44 x 44. Pencraft has no corner smoothing;
plain rounded corners are what you get.

## 6. Build bars the way this design language does

- No bar backgrounds. A toolbar is not a strip: it is a row of separate glass shapes over the content.
- Group by function: actions that belong together share **one** glass shape (a capsule holding three icons).
  Icons and text buttons never share a shape. More than three or four actions: put the rest behind `ellipsis`.
- The primary action stands alone, at the trailing end, tinted (`tint`), with a white symbol.
- Tab bar: a floating capsule at the bottom with 2-5 tabs, each an icon over an 11-point label; the selected
  tab is a capsule **fill** inside it (`#0000001f` on light glass, `#ffffff33` on dark), not more glass,
  and keeps the same label colour as the others (an accent-coloured label rarely holds 4.5:1 on glass).
  Search is its own round glass button beside the tab capsule.
- Never glass on glass. What sits on a glass surface is a fill, a label or an icon.
- A control whose menu is open is marked with a selection fill inside it, like a selected tab.
- A sidebar floats over the content's ground or over a wide hero picture; put content that must be seen
  (thumbnails, text) beside it, not under it.
- Sheets are inset from the screen edges with a large radius and content visible around them; a modal sheet
  gets a `#00000033` dimming rectangle over the content. Menus and popovers sit next to the control that
  opened them and do not overlap other glass.

Ready-made layers for each of these are in `references/components.md`. Look for existing components in the
document first (`get_state` lists them). For a document with several screens, insert the ones you need from
that file as reusable components on a board and instance them; for a single screen, put the same layers
straight into the screen (without `reusable`) and add no board.

## 7. Merge neighbours with a glass group

Glass cannot show other glass through it. Glass shapes that sit side by side (the tab capsule and its search
button, a row of toolbar buttons) go into one plain frame with `"glassGroup": <spacing>`. That frame has no
material and no fill; its glass children are drawn as one piece, and children closer together than the
spacing flow into each other like droplets. Set the spacing to the frame's `gap` so they stay separate at
rest; set it larger (gap x 2) to show them merging in a transition frame. All glass children of a group
share the first child's material, so a tinted button stays outside the group. The first child's size also
decides how thick the whole piece is: put the largest child first, or give the first child a `depth`. The
spacing is a distance, so give the frame a fixed `gap` (not `space_between`).

```json
{"type":"frame","layout":"horizontal","gap":10,"glassGroup":10,"children":[<tab capsule>,<search button>]}
```

## 8. Where glass meets scrolling content

Under a pinned bar, content should fade out instead of colliding with the bar. Put one rectangle over the
content and under the glass, as wide as the screen, from the screen edge to a little past the bar, filled
with a gradient from the content's background colour to clear:

```json
{"type":"rectangle","name":"Scroll edge","x":0,"y":0,"width":393,"height":120,
 "fill":{"type":"gradient","gradientType":"linear","rotation":0,
         "colors":[{"color":"#ffffff00","position":0},{"color":"#ffffffcc","position":1}]}}
```

`rotation` 0 puts position 1 at the top (a top bar); use 180 for a bottom bar. One per edge, never stacked.
Make the band about twice as tall as the distance from the screen edge to the bar's far side, so the bar
sits where the fade is still half clear: a fade that is opaque under the bar leaves the glass nothing to
bend (`check_glass` then reports a flat backdrop). Colours can be variables (`"$bg"` cannot take an alpha,
so define `bg-clear` and `bg-fade`). Pencraft's review counts these gradients; that remark can stand.
Leave the fade out over full-bleed photos, where it would grey the picture. For dense desktop layouts use a
`{"type":"frosted","thickness":"thin"}` band instead (the "hard" style).

## 9. Colour and legibility

- Glass has no colour; do not fill it. Brand colour lives in the content (a header picture, a coloured
  card), not on the bars.
- Labels and icons on glass are monochrome. Only the selected tab may take the accent colour.
- Every label and icon needs 4.5:1 against the **worst** part of what is under it, not the average.
  `check_glass` measures exactly this, for every text and icon in the layers you give it, on glass or on
  the content. A tint must carry its white symbol too: `#0a84ff` gives only 3.6:1; use a deeper accent such
  as `#0060df`. Text that lies straight on a photo needs a scrim (a gradient from clear to 50-60 % black)
  under it or a frosted panel around it.
- Provide the accessibility looks as theme values, not as copies: see `references/tokens.md` for the
  `Glass` theme axis (Clear, Tinted, Reduced transparency, Increased contrast).

## 10. Workflow

1. `get_state`, `read_layers`: what is there, which variables and components exist.
2. Make sure the content layer gives the glass something to bend (section 2).
3. Set up tokens once (`references/tokens.md`): `$on-glass`, `$selection-on-glass`, `$accent`, and the
   `Mode` and `Glass` theme axes. Skip the ones a single screen does not use.
4. Insert or instance the components (`references/components.md`), compose the screen. Glass layers come
   after the content in the layer order, and are placed with `x`/`y` in a `"layout":"none"` screen frame.
5. `screenshot` the screen, then `check_glass` with the screen's id.
6. Fix everything `check_glass` lists, set label colours from the `appearance` it reports, and repeat 5
   until `ok` is true. Then look at the picture once more against this list:
   - the rims visibly bend what is behind them (if not: nothing behind, or a flat backdrop);
   - every label reads over the busiest spot under it;
   - no glass on glass, and neighbours are in a glass group;
   - nested corners are concentric, bars are capsules, nothing glass touches the screen edge;
   - one variant per screen, one accent, one tinted action;
   - a scroll edge fade wherever a bar overlaps content that is not a photo.

## 11. When it looks wrong

| It looks like | Cause | Fix |
|---|---|---|
| A grey box | Nothing behind it, or content stops before the bar | Run the content behind the glass; add imagery or colour to the content layer |
| Muddy, dirty | A `fill` on the glass, a strong tint, or extra blur effects | Remove the fill and the effects; tint only the primary action |
| Label unreadable | Clear glass over bright media, or a label the same lightness as the glass | Add the 35 % dimming layer, switch to regular, or flip the label colour |
| Plastic, shiny | `highlight` raised, or stroke and shadow effects added on top | Remove them; the material lights its own rim |
| A panel that flickers between light and dark across screens | A large pane left on `auto` | Pin `appearance` on sidebars, sheets and menus |
| Everything floats, nothing is content | Lists, cards or text made of glass | Glass for navigation and controls only; frosted for structure; fills for content |
| A blob where two buttons should be | A glass group with spacing larger than the gap | Set `glassGroup` to the gap |

More background on why the rules are what they are: `references/design-language.md`.
