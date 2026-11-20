# Material components

Each block is one layer to insert with `edit_layers` (`{"op":"insert","node":<block>}`), most of them as a
reusable component. Their sizes are Material's; their colours come from whatever scheme the roles hold, and
their content and the way they are put together are the design's own. They use the roles of `references/tokens.md`: set those first. Sizes are those of the
Material 3 Expressive token files of Jetpack Compose; a padding marked "this kit's" is not in those files.
Ids start with `m3-`; if the document already has one, instance it. Icons are Lucide stand-ins.

## Button

40 high (the small size, the default), a pill, 16 beside the label, Label Large. An icon is 20 with 8 to
the label: set `"enabled":true` on `m3-button-icon`. States: `hover`, `pressed` (the corner becomes 8) and
`disabled`.

```json
{"type":"frame","id":"m3-button","name":"Button","reusable":true,"height":40,"padding":[0,16],"gap":8,"justifyContent":"center","alignItems":"center","cornerRadius":20,"fill":"$primary","states":{"hover":{"fill":["$primary",{"type":"color","color":"$on-primary","opacity":0.08}]},"pressed":{"fill":["$primary",{"type":"color","color":"$on-primary","opacity":0.1}],"cornerRadius":8},"disabled":{"fill":{"type":"color","color":"$on-surface","opacity":0.1},"descendants":{"m3-button-label":{"fill":"$on-surface","opacity":0.38},"m3-button-icon":{"fill":"$on-surface","opacity":0.38}}}},"children":[
  {"type":"icon","id":"m3-button-icon","name":"Icon","library":"lucide","icon":"plus","width":20,"height":20,"fill":"$on-primary","enabled":false},
  {"type":"text","id":"m3-button-label","name":"Label","content":"Book a repair","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-primary"}]}
```

The five kinds are overrides on an instance:

| Kind | On the `ref` | Label | For |
|---|---|---|---|
| Filled | nothing | `$on-primary` | The one main action of a screen |
| Tonal | `"fill":"$secondary-container"` | `$on-secondary-container` | An important action that is not the main one |
| Elevated | `"fill":"$surface-container-low"` and the level 1 shadow | `$primary` | A tonal button that must stand off a busy ground |
| Outlined | no fill, a 1 wide `$outline-variant` line | `$on-surface-variant` | A second action beside a filled one |
| Text | no fill, 12 beside the label | `$primary` | The least weight: in dialogs, cards, snackbars |

In the example below they stand in that order: filled, tonal, elevated, outlined, text.

```json
{"type":"frame","id":"m3-buttons","name":"Button variants","gap":12,"alignItems":"center","children":[
  {"type":"ref","id":"m3-buttons-filled","name":"Button / filled","ref":"m3-button","descendants":{"m3-button-label":{"content":"Book a repair","fill":"$on-primary"}}},
  {"type":"ref","id":"m3-buttons-tonal","name":"Button / tonal","ref":"m3-button","descendants":{"m3-button-label":{"content":"Save café","fill":"$on-secondary-container"}},"fill":"$secondary-container"},
  {"type":"ref","id":"m3-buttons-elevated","name":"Button / elevated","ref":"m3-button","descendants":{"m3-button-label":{"content":"Share","fill":"$primary"}},"fill":"$surface-container-low","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":2,"color":"#0000004d"},{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"spread":1,"color":"#00000026"}]},
  {"type":"ref","id":"m3-buttons-outlined","name":"Button / outlined","ref":"m3-button","descendants":{"m3-button-label":{"content":"Directions","fill":"$on-surface-variant"}},"fill":"#00000000","stroke":"$outline-variant","strokeWidth":1,"strokeAlignment":"inner"},
  {"type":"ref","id":"m3-buttons-text","name":"Button / text","ref":"m3-button","descendants":{"m3-button-label":{"content":"Details","fill":"$primary"}},"fill":"#00000000","padding":[0,12]}]}
```

- Other sizes: 32 (16 beside, icon 20, square corner 12), 56 (24 beside, icon 24, Title Medium, square
  corner 16), 96 (48 beside, icon 32, Headline Small, square corner 28) and 136 (64 beside, icon 40,
  Headline Large, square corner 28). The label's type for the sizes above 40 is this kit's choice.
- A square button has the corner of its size instead of the pill; pressed, both take the smaller corner
  (8 up to 40, 12 at 56, 16 above).
- The outlined button's colours are the Expressive ones as this kit recalls them; the older style had a
  `$outline` line and a `$primary` label.

## Icon button

40 across with a 24 icon, no fill at rest. Selected (a toggle that is on) it is filled and square. Sizes
32, 56, 96 and 136 as for buttons.

```json
{"type":"frame","id":"m3-icon-button","name":"Icon button","reusable":true,"width":40,"height":40,"justifyContent":"center","alignItems":"center","cornerRadius":20,"states":{"hover":{"fill":{"type":"color","color":"$on-surface-variant","opacity":0.08}},"pressed":{"fill":{"type":"color","color":"$on-surface-variant","opacity":0.1}},"selected":{"fill":"$primary","cornerRadius":12,"descendants":{"m3-icon-button-icon":{"fill":"$on-primary"}}}},"children":[
  {"type":"icon","id":"m3-icon-button-icon","name":"Icon","library":"lucide","icon":"heart","width":24,"height":24,"fill":"$on-surface-variant"}]}
```

Filled: `"fill":"$primary"` with the icon in `$on-primary`; tonal: `$secondary-container`; outlined: a 1
wide `$outline-variant` line.

## FAB

The screen's one most important action. 56 with corner 16 and a 24 icon; extended, with a label, 56 high
with 16 beside and 8 between. `$primary-container`, level 3. Also 80 (corner 20, icon 28) and 96 (corner
28, icon 32). Without a label: leave the label out and give it `"width":56`.

```json
{"type":"frame","id":"m3-fab","name":"Extended FAB","reusable":true,"height":56,"padding":[0,16],"gap":8,"justifyContent":"center","alignItems":"center","cornerRadius":16,"fill":"$primary-container","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"color":"#0000004d"},{"type":"shadow","offset":{"x":0,"y":4},"blur":8,"spread":3,"color":"#00000026"}],"children":[
  {"type":"icon","id":"m3-fab-icon","name":"Icon","library":"lucide","icon":"plus","width":24,"height":24,"fill":"$on-primary-container"},
  {"type":"text","id":"m3-fab-label","name":"Label","content":"Add a café","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-primary-container"}]}
```

## Connected button group

Buttons that choose one of a few: 40 high, 2 apart, the outer ends round, the inner corners 8. The chosen
one is filled and fully round.

```json
{"type":"frame","id":"m3-group","name":"Connected button group","gap":2,"children":[
  {"type":"frame","id":"m3-group-1","name":"Segment","height":40,"padding":[0,16],"justifyContent":"center","alignItems":"center","cornerRadius":[20,8,8,20],"fill":"$secondary-container","children":[
    {"type":"text","id":"m3-group-1-label","content":"Day","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-secondary-container"}]},
  {"type":"frame","id":"m3-group-2","name":"Segment, selected","height":40,"padding":[0,16],"justifyContent":"center","alignItems":"center","cornerRadius":20,"fill":"$primary","children":[
    {"type":"text","id":"m3-group-2-label","content":"Week","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-primary"}]},
  {"type":"frame","id":"m3-group-3","name":"Segment","height":40,"padding":[0,16],"justifyContent":"center","alignItems":"center","cornerRadius":[8,20,20,8],"fill":"$secondary-container","children":[
    {"type":"text","id":"m3-group-3-label","content":"Month","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-secondary-container"}]}]}
```

A standard group is separate buttons 12 apart. A split button is a button and a narrow second part with a
`chevron-down`, 2 apart, inner corners 4.

## Chips

32 high, corner 8, Label Large, a 1 wide `$outline-variant` line; an icon is 18 with 8 to the label and 8
before it. A filter chip that is on loses its line, takes `$secondary-container` and a `check`. Chips
stand in one row, 8 apart, and scroll sideways when they do not fit: let the row run off the trailing edge.

```json
{"type":"frame","id":"m3-chips","name":"Filter chips","gap":8,"alignItems":"center","children":[
  {"type":"frame","id":"m3-chip-1","name":"Filter chip, selected","height":32,"padding":[0,16,0,8],"gap":8,"alignItems":"center","cornerRadius":8,"fill":"$secondary-container","children":[
    {"type":"icon","id":"m3-chip-1-check","library":"lucide","icon":"check","width":18,"height":18,"fill":"$on-secondary-container"},
    {"type":"text","id":"m3-chip-1-label","content":"Open now","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-secondary-container"}]},
  {"type":"frame","id":"m3-chip-2","name":"Filter chip","height":32,"padding":[0,16],"gap":8,"alignItems":"center","cornerRadius":8,"stroke":"$outline-variant","strokeWidth":1,"strokeAlignment":"inner","children":[
    {"type":"text","id":"m3-chip-2-label","content":"Bikes","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-surface-variant"}]},
  {"type":"frame","id":"m3-chip-3","name":"Filter chip","height":32,"padding":[0,16],"gap":8,"alignItems":"center","cornerRadius":8,"stroke":"$outline-variant","strokeWidth":1,"strokeAlignment":"inner","children":[
    {"type":"text","id":"m3-chip-3-label","content":"Electronics","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-surface-variant"}]},
  {"type":"frame","id":"m3-chip-4","name":"Filter chip","height":32,"padding":[0,16],"gap":8,"alignItems":"center","cornerRadius":8,"stroke":"$outline-variant","strokeWidth":1,"strokeAlignment":"inner","children":[
    {"type":"text","id":"m3-chip-4-label","content":"Clothes","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-surface-variant"}]}]}
```

## Card

Corner 12, 16 inside (this kit's). Filled: `$surface-container-highest`, no shadow. Elevated:
`$surface-container-low` with the level 1 shadow. Outlined: `$surface` with a 1 wide `$outline-variant`
line. One kind per screen.

```json
{"type":"frame","id":"m3-card","name":"Card","reusable":true,"width":360,"layout":"vertical","gap":24,"padding":16,"cornerRadius":12,"fill":"$surface-container-highest","children":[
  {"type":"frame","id":"m3-card-text","width":"fill_container","layout":"vertical","gap":4,"children":[
    {"type":"text","id":"m3-card-title","name":"Title","content":"Lantern Repair Café","fontFamily":"$font","fontSize":16,"fontWeight":"500","lineHeight":1.5,"letterSpacing":0.15,"fill":"$on-surface","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"m3-card-detail","name":"Supporting text","content":"Volunteers fix toasters, lamps, bikes and zips with you, for free.","fontFamily":"$font","fontSize":14,"fontWeight":"400","lineHeight":1.4286,"letterSpacing":0.25,"fill":"$on-surface-variant","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"frame","id":"m3-card-actions","name":"Actions","width":"fill_container","gap":8,"justifyContent":"end","children":[
    {"type":"ref","id":"m3-card-secondary","ref":"m3-button","fill":"#00000000","stroke":"$outline-variant","strokeWidth":1,"strokeAlignment":"inner","descendants":{"m3-button-label":{"content":"Directions","fill":"$on-surface-variant"}}},
    {"type":"ref","id":"m3-card-primary","ref":"m3-button","descendants":{"m3-button-label":{"content":"Book a repair"}}}]}]}
```

## Text field

56 high, corner 4, a 1 wide `$outline` line, Body Large, 16 beside. The label sits on the line in Body
Small once there is a value; in an empty field it is the placeholder in Body Large and
`$on-surface-variant`. Supporting text under it in Body Small. States: `focus` (the line 2 wide in
`$primary`, the label too) and `error`.

```json
{"type":"frame","id":"m3-text-field","name":"Text field, outlined","reusable":true,"width":328,"layout":"vertical","gap":4,"states":{"focus":{"descendants":{"m3-text-field-box":{"stroke":"$primary","strokeWidth":2},"m3-text-field-label":{"fill":"$primary"}}},"error":{"descendants":{"m3-text-field-box":{"stroke":"$error","strokeWidth":2},"m3-text-field-label":{"fill":"$error"},"m3-text-field-supporting":{"fill":"$error"}}}},"children":[
  {"type":"frame","id":"m3-text-field-box","name":"Container","width":"fill_container","height":56,"padding":[0,16],"alignItems":"center","cornerRadius":4,"stroke":"$outline","strokeWidth":1,"strokeAlignment":"inner","children":[
    {"type":"text","id":"m3-text-field-input","name":"Input","content":"Kettle, does not switch off","fontFamily":"$font","fontSize":16,"fontWeight":"400","lineHeight":1.5,"letterSpacing":0.5,"fill":"$on-surface","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"frame","id":"m3-text-field-label-box","padding":[0,4],"fill":"$surface","layoutPosition":"absolute","x":12,"y":-8,"children":[
      {"type":"text","id":"m3-text-field-label","name":"Label","content":"What needs fixing?","fontFamily":"$font","fontSize":12,"fontWeight":"400","lineHeight":1.3333,"letterSpacing":0.4,"fill":"$on-surface-variant"}]}]},
  {"type":"frame","id":"m3-text-field-support","width":"fill_container","padding":[0,16],"children":[
    {"type":"text","id":"m3-text-field-supporting","name":"Supporting text","content":"One thing per booking","fontFamily":"$font","fontSize":12,"fontWeight":"400","lineHeight":1.3333,"letterSpacing":0.4,"fill":"$on-surface-variant","textGrowth":"fixed-width","width":"fill_container"}]}]}
```

The filled kind has `$surface-container-highest`, top corners of 4 and only a line under it (1, or 2
focused). The label's box takes the colour of what is behind the field: change `m3-text-field-label-box`
when the field stands on a card. (Pencraft's advice about a layer with the fill of what is behind it can
stand for this one box: it is what breaks the line behind the label.)

## Switch, checkbox

A switch is a track of 52 by 32 with a handle of 24 when on. Off (`"state":"off"`): the track is
`$surface-container-highest` with a 2 wide `$outline` line and the handle 16 in `$outline`.

```json
{"type":"frame","id":"m3-switch","name":"Switch","reusable":true,"width":52,"height":32,"padding":4,"justifyContent":"end","alignItems":"center","cornerRadius":16,"fill":"$primary","states":{"off":{"fill":"$surface-container-highest","stroke":"$outline","strokeWidth":2,"strokeAlignment":"inner","justifyContent":"start","padding":8,"descendants":{"m3-switch-handle":{"width":16,"height":16,"cornerRadius":8,"fill":"$outline"}}}},"children":[
  {"type":"frame","id":"m3-switch-handle","name":"Handle","width":24,"height":24,"cornerRadius":12,"fill":"$on-primary","children":[]}]}
```

A checkbox is 18 with corner 2; off, a 2 wide `$on-surface-variant` line. A radio button is a 20 ring of
2 with a 10 dot when chosen. Each has 40 by 40 to itself and its label in Body Large 16 after it.

```json
{"type":"frame","id":"m3-checkbox","name":"Checkbox","reusable":true,"width":18,"height":18,"justifyContent":"center","alignItems":"center","cornerRadius":2,"fill":"$primary","states":{"off":{"fill":"#00000000","stroke":"$on-surface-variant","strokeWidth":2,"strokeAlignment":"inner","descendants":{"m3-checkbox-mark":{"enabled":false}}}},"children":[
  {"type":"icon","id":"m3-checkbox-mark","name":"Mark","library":"lucide","icon":"check","width":14,"height":14,"fill":"$on-primary"}]}
```

## Slider

The Expressive slider: a track 16 high in two parts, `$primary` up to the handle and
`$secondary-container` after it, the handle a bar 4 wide and 44 high with 6 on each side, and a 4 dot at
the end of the track. The corners of the two parts are 8 outside and 2 toward the handle.

```json
{"type":"frame","id":"m3-slider","name":"Slider","width":328,"height":44,"gap":6,"alignItems":"center","children":[
  {"type":"rectangle","id":"m3-slider-active","name":"Active track","width":180,"height":16,"fill":"$primary","cornerRadius":[8,2,2,8]},
  {"type":"rectangle","id":"m3-slider-handle","name":"Handle","width":4,"height":44,"fill":"$primary","cornerRadius":2},
  {"type":"frame","id":"m3-slider-inactive","name":"Inactive track","width":"fill_container","height":16,"padding":[0,6],"justifyContent":"end","alignItems":"center","cornerRadius":[2,8,8,2],"fill":"$secondary-container","children":[
    {"type":"ellipse","id":"m3-slider-stop","name":"Stop","width":4,"height":4,"fill":"$primary"}]}]}
```

## List item

One line 56 high, two lines 72, three 88; 16 at the sides, 12 between the parts. Headline Body Large,
supporting text Body Medium in `$on-surface-variant`, a trailing text Label Small. Leading: an icon of 24,
an avatar of 40, or a picture of 56. No lines between items by default; a divider, where one is wanted, is
1 high in `$outline-variant`, inset 16.

```json
{"type":"frame","id":"m3-list-item","name":"List item, two lines","reusable":true,"width":412,"height":72,"padding":[0,16],"gap":12,"alignItems":"center","children":[
  {"type":"frame","id":"m3-list-item-avatar","name":"Avatar","width":40,"height":40,"justifyContent":"center","alignItems":"center","cornerRadius":20,"fill":"$primary-container","children":[
    {"type":"text","id":"m3-list-item-initial","name":"Initial","content":"L","fontFamily":"$font","fontSize":16,"fontWeight":"500","lineHeight":1.5,"letterSpacing":0.15,"fill":"$on-primary-container"}]},
  {"type":"frame","id":"m3-list-item-text","width":"fill_container","layout":"vertical","children":[
    {"type":"text","id":"m3-list-item-headline","name":"Headline","content":"Lantern Repair Café","fontFamily":"$font","fontSize":16,"fontWeight":"400","lineHeight":1.5,"letterSpacing":0.5,"fill":"$on-surface","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"m3-list-item-supporting","name":"Supporting text","content":"Markveien 58 · Sat 11:00","fontFamily":"$font","fontSize":14,"fontWeight":"400","lineHeight":1.4286,"letterSpacing":0.25,"fill":"$on-surface-variant","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"text","id":"m3-list-item-trailing","name":"Trailing text","content":"350 m","fontFamily":"$font","fontSize":11,"fontWeight":"500","lineHeight":1.4545,"letterSpacing":0.5,"fill":"$on-surface-variant"}]}
```

Expressive lists may be segmented: items 2 apart on `$surface-container`, each with corner 4, the first
and last of the list rounded 16 at their outer ends.

## Top app bar

64 high, 4 at the sides, with no fill of its own (the page's `$surface` shows through); a leading icon button, the title in Title Large starting 4 after
it, then at most three actions. The title is at the leading side, not centred. With content scrolled
under it the bar takes `"fill":"$surface-container"` (level 2 without a shadow). Larger bars for a screen's first
view: 112 with the title in Headline Medium under the icons, or 120 with Display Small.

```json
{"type":"frame","id":"m3-app-bar","name":"Top app bar, small","reusable":true,"width":412,"height":64,"padding":[0,4],"alignItems":"center","children":[
  {"type":"ref","id":"m3-app-bar-leading","ref":"m3-icon-button","descendants":{"m3-icon-button-icon":{"icon":"menu","fill":"$on-surface"}}},
  {"type":"frame","id":"m3-app-bar-title-box","width":"fill_container","padding":[0,4],"children":[
    {"type":"text","id":"m3-app-bar-title","name":"Title","content":"Repair cafés","fontFamily":"$font","fontSize":22,"fontWeight":"400","lineHeight":1.2727,"fill":"$on-surface","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"ref","id":"m3-app-bar-action","ref":"m3-icon-button","descendants":{"m3-icon-button-icon":{"icon":"search"}}},
  {"type":"frame","id":"m3-app-bar-avatar-box","width":40,"height":40,"justifyContent":"center","alignItems":"center","children":[
    {"type":"frame","id":"m3-app-bar-avatar","name":"Avatar","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":16,"fill":"$primary","children":[
      {"type":"text","id":"m3-app-bar-avatar-initial","content":"M","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-primary"}]}]}]}
```

## Navigation bar

64 high on `$surface-container`, three to five destinations of equal width. Each is a 24 icon in an
indicator of 56 by 32 and a label in Label Medium 4 under it. The active one has the indicator filled with
`$secondary-container`, its icon in `$on-secondary-container` and its label emphasized in `$on-surface` (Material's `$secondary`
falls short of the 6:1 Pencraft asks of text this small).

```json
{"type":"frame","id":"m3-nav-bar","name":"Navigation bar","reusable":true,"width":412,"height":64,"padding":[0,8],"alignItems":"center","fill":"$surface-container","children":[
  {"type":"frame","id":"m3-nav-item-1","name":"Destination, active","width":"fill_container","layout":"vertical","gap":4,"alignItems":"center","children":[
    {"type":"frame","id":"m3-nav-item-1-indicator","name":"Indicator","width":56,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":16,"fill":"$secondary-container","children":[
      {"type":"icon","id":"m3-nav-item-1-icon","library":"lucide","icon":"map-pin","width":24,"height":24,"fill":"$on-secondary-container"}]},
    {"type":"text","id":"m3-nav-item-1-label","content":"Nearby","fontFamily":"$font","fontSize":12,"fontWeight":"700","lineHeight":1.3333,"letterSpacing":0.5,"fill":"$on-surface"}]},
  {"type":"frame","id":"m3-nav-item-2","name":"Destination","width":"fill_container","layout":"vertical","gap":4,"alignItems":"center","children":[
    {"type":"frame","id":"m3-nav-item-2-indicator","name":"Indicator","width":56,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":16,"children":[
      {"type":"icon","id":"m3-nav-item-2-icon","library":"lucide","icon":"bookmark","width":24,"height":24,"fill":"$on-surface-variant"}]},
    {"type":"text","id":"m3-nav-item-2-label","content":"Saved","fontFamily":"$font","fontSize":12,"fontWeight":"500","lineHeight":1.3333,"letterSpacing":0.5,"fill":"$on-surface-variant"}]},
  {"type":"frame","id":"m3-nav-item-3","name":"Destination","width":"fill_container","layout":"vertical","gap":4,"alignItems":"center","children":[
    {"type":"frame","id":"m3-nav-item-3-indicator","name":"Indicator","width":56,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":16,"children":[
      {"type":"icon","id":"m3-nav-item-3-icon","library":"lucide","icon":"calendar","width":24,"height":24,"fill":"$on-surface-variant"}]},
    {"type":"text","id":"m3-nav-item-3-label","content":"Bookings","fontFamily":"$font","fontSize":12,"fontWeight":"500","lineHeight":1.3333,"letterSpacing":0.5,"fill":"$on-surface-variant"}]},
  {"type":"frame","id":"m3-nav-item-4","name":"Destination","width":"fill_container","layout":"vertical","gap":4,"alignItems":"center","children":[
    {"type":"frame","id":"m3-nav-item-4-indicator","name":"Indicator","width":56,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":16,"children":[
      {"type":"icon","id":"m3-nav-item-4-icon","library":"lucide","icon":"user","width":24,"height":24,"fill":"$on-surface-variant"}]},
    {"type":"text","id":"m3-nav-item-4-label","content":"You","fontFamily":"$font","fontSize":12,"fontWeight":"500","lineHeight":1.3333,"letterSpacing":0.5,"fill":"$on-surface-variant"}]}]}
```

From 600 wide it is a rail instead: 96 wide at the leading edge, the same items one under the other 4
apart, starting 44 from the top, a FAB above them.

## Search bar

56 high, a pill on `$surface-container-high`, 16 at the sides: a leading icon, the hint in Body Large and
`$on-surface-variant`, an avatar of 30 or an icon at the trailing side.

```json
{"type":"frame","id":"m3-search-bar","name":"Search bar","reusable":true,"width":380,"height":56,"padding":[0,16],"gap":16,"alignItems":"center","cornerRadius":28,"fill":"$surface-container-high","children":[
  {"type":"icon","id":"m3-search-bar-icon","name":"Leading icon","library":"lucide","icon":"search","width":24,"height":24,"fill":"$on-surface"},
  {"type":"text","id":"m3-search-bar-text","name":"Placeholder","content":"Search cafés and repairs","fontFamily":"$font","fontSize":16,"fontWeight":"400","lineHeight":1.5,"letterSpacing":0.5,"fill":"$on-surface-variant","textGrowth":"fixed-width","width":"fill_container"},
  {"type":"frame","id":"m3-search-bar-avatar","name":"Avatar","width":30,"height":30,"justifyContent":"center","alignItems":"center","cornerRadius":15,"fill":"$primary","children":[
    {"type":"text","id":"m3-search-bar-initial","content":"M","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-primary"}]}]}
```

## Tabs

48 high, each tab's label in Title Small. The selected tab's label is `$primary` with an indicator 3
thick under it, its top corners 3; a 1 high `$outline-variant` line runs under the whole row.

```json
{"type":"frame","id":"m3-tabs","name":"Tabs","width":412,"height":48,"fill":"$surface","stroke":"$outline-variant","strokeWidth":{"bottom":1},"children":[
  {"type":"frame","id":"m3-tab-1","name":"Tab, selected","width":"fill_container","height":48,"layout":"vertical","alignItems":"center","children":[
    {"type":"frame","id":"m3-tab-1-box","height":45,"alignItems":"center","children":[
      {"type":"text","id":"m3-tab-1-label","content":"Upcoming","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$primary"}]},
    {"type":"rectangle","id":"m3-tab-1-indicator","name":"Indicator","width":48,"height":3,"fill":"$primary","cornerRadius":[3,3,0,0]}]},
  {"type":"frame","id":"m3-tab-2","name":"Tab","width":"fill_container","height":48,"layout":"vertical","alignItems":"center","children":[
    {"type":"frame","id":"m3-tab-2-box","height":45,"alignItems":"center","children":[
      {"type":"text","id":"m3-tab-2-label","content":"Past","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-surface-variant"}]},
    {"type":"rectangle","id":"m3-tab-2-indicator","name":"Indicator","width":48,"height":3,"fill":"#00000000","cornerRadius":[3,3,0,0]}]},
  {"type":"frame","id":"m3-tab-3","name":"Tab","width":"fill_container","height":48,"layout":"vertical","alignItems":"center","children":[
    {"type":"frame","id":"m3-tab-3-box","height":45,"alignItems":"center","children":[
      {"type":"text","id":"m3-tab-3-label","content":"Cancelled","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-surface-variant"}]},
    {"type":"rectangle","id":"m3-tab-3-indicator","name":"Indicator","width":48,"height":3,"fill":"#00000000","cornerRadius":[3,3,0,0]}]}]}
```

## Dialog

Corner 28 on `$surface-container-high`, level 3, 24 inside (this kit's), 312 wide on a phone (this
kit's). The headline in Headline Small, the text in Body Medium and `$on-surface-variant`, then text
buttons at the trailing edge, 8 apart, the confirming one last. Under it `$scrim` over the whole screen.

```json
{"type":"frame","id":"m3-dialog","name":"Dialog","reusable":true,"width":312,"layout":"vertical","gap":24,"padding":24,"cornerRadius":28,"fill":"$surface-container-high","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"color":"#0000004d"},{"type":"shadow","offset":{"x":0,"y":4},"blur":8,"spread":3,"color":"#00000026"}],"children":[
  {"type":"frame","id":"m3-dialog-text","width":"fill_container","layout":"vertical","gap":16,"children":[
    {"type":"text","id":"m3-dialog-headline","name":"Headline","content":"Cancel this booking?","fontFamily":"$font","fontSize":24,"fontWeight":"400","lineHeight":1.3333,"fill":"$on-surface","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"m3-dialog-body","name":"Supporting text","content":"Your place at 11:40 on Saturday goes to the next person in line.","fontFamily":"$font","fontSize":14,"fontWeight":"400","lineHeight":1.4286,"letterSpacing":0.25,"fill":"$on-surface-variant","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"frame","id":"m3-dialog-actions","name":"Actions","width":"fill_container","gap":8,"justifyContent":"end","children":[
    {"type":"ref","id":"m3-dialog-dismiss","ref":"m3-button","fill":"#00000000","padding":[0,12],"descendants":{"m3-button-label":{"content":"Keep it","fill":"$primary"}}},
    {"type":"ref","id":"m3-dialog-confirm","ref":"m3-button","fill":"#00000000","padding":[0,12],"descendants":{"m3-button-label":{"content":"Cancel booking","fill":"$primary"}}}]}]}
```

A bottom sheet has top corners of 28, `$surface-container-low`, level 1 and a handle of 32 by 4 in
`$on-surface-variant`, 16 (this kit's) from its top edge.

## Snackbar

48 high (68 with two lines), corner 4, `$inverse-surface` with the text in Body Medium and
`$inverse-on-surface`, one action in `$inverse-primary`. 16 from the sides and over the navigation bar.

```json
{"type":"frame","id":"m3-snackbar","name":"Snackbar","reusable":true,"width":380,"height":48,"padding":[0,16],"gap":8,"alignItems":"center","cornerRadius":4,"fill":"$inverse-surface","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"color":"#0000004d"},{"type":"shadow","offset":{"x":0,"y":4},"blur":8,"spread":3,"color":"#00000026"}],"children":[
  {"type":"text","id":"m3-snackbar-text","name":"Text","content":"Booking cancelled","fontFamily":"$font","fontSize":14,"fontWeight":"400","lineHeight":1.4286,"letterSpacing":0.25,"fill":"$inverse-on-surface","textGrowth":"fixed-width","width":"fill_container"},
  {"type":"text","id":"m3-snackbar-action","name":"Action","content":"Undo","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$inverse-primary"}]}
```

## A whole screen

412 by 915. From the top: the status bar (36, this kit's), the top app bar, a row of filter chips, the
list, then the navigation bar with the gesture area (24, this kit's) under it. The two share one frame
that carries the bar's colour, so the bar's instance has its own fill taken away. The
FAB floats 16 from the trailing edge and 16 over the bar, and the list ends above it: a list that runs on
scrolls under the FAB, so leave its last row clear of it in the picture. Insert `m3-icon-button`, `m3-app-bar`,
`m3-list-item`, `m3-nav-bar` and `m3-fab` first.

```json
{"type":"frame","id":"m3-screen","name":"Android screen","width":412,"height":915,"layout":"vertical","fill":"$surface","clip":true,"children":[
  {"type":"frame","id":"m3-screen-status","name":"Status bar","width":"fill_container","height":36,"padding":[0,24],"justifyContent":"space_between","alignItems":"center","children":[
    {"type":"text","id":"m3-screen-status-time","name":"Time","content":"9:41","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-surface"},
    {"type":"frame","id":"m3-screen-status-icons","gap":6,"alignItems":"center","children":[
      {"type":"icon","id":"m3-screen-status-signal","library":"lucide","icon":"signal","width":16,"height":16,"fill":"$on-surface"},
      {"type":"icon","id":"m3-screen-status-wifi","library":"lucide","icon":"wifi","width":16,"height":16,"fill":"$on-surface"},
      {"type":"icon","id":"m3-screen-status-battery","library":"lucide","icon":"battery-full","width":18,"height":18,"fill":"$on-surface"}]}]},
  {"type":"ref","id":"m3-screen-app-bar","ref":"m3-app-bar","width":"fill_container"},
  {"type":"frame","id":"m3-screen-filters","name":"Filters","width":"fill_container","height":48,"padding":[0,16],"alignItems":"center","children":[
    {"type":"frame","id":"m3-screen-chips","name":"Filter chips","gap":8,"alignItems":"center","children":[
      {"type":"frame","id":"m3-screen-chip-1","name":"Filter chip, selected","height":32,"padding":[0,16,0,8],"gap":8,"alignItems":"center","cornerRadius":8,"fill":"$secondary-container","children":[
        {"type":"icon","id":"m3-screen-chip-1-check","library":"lucide","icon":"check","width":18,"height":18,"fill":"$on-secondary-container"},
        {"type":"text","id":"m3-screen-chip-1-label","content":"Open now","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-secondary-container"}]},
      {"type":"frame","id":"m3-screen-chip-2","name":"Filter chip","height":32,"padding":[0,16],"gap":8,"alignItems":"center","cornerRadius":8,"stroke":"$outline-variant","strokeWidth":1,"strokeAlignment":"inner","children":[
        {"type":"text","id":"m3-screen-chip-2-label","content":"Bikes","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-surface-variant"}]},
      {"type":"frame","id":"m3-screen-chip-3","name":"Filter chip","height":32,"padding":[0,16],"gap":8,"alignItems":"center","cornerRadius":8,"stroke":"$outline-variant","strokeWidth":1,"strokeAlignment":"inner","children":[
        {"type":"text","id":"m3-screen-chip-3-label","content":"Phones","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-surface-variant"}]},
      {"type":"frame","id":"m3-screen-chip-4","name":"Filter chip","height":32,"padding":[0,16],"gap":8,"alignItems":"center","cornerRadius":8,"stroke":"$outline-variant","strokeWidth":1,"strokeAlignment":"inner","children":[
        {"type":"text","id":"m3-screen-chip-4-label","content":"Clothes","fontFamily":"$font","fontSize":14,"fontWeight":"500","lineHeight":1.4286,"letterSpacing":0.1,"fill":"$on-surface-variant"}]}]}]},
  {"type":"frame","id":"m3-screen-list","name":"List","width":"fill_container","height":"fill_container","layout":"vertical","clip":true,"children":[
    {"type":"ref","id":"m3-screen-item-1","ref":"m3-list-item","width":"fill_container","descendants":{"m3-list-item-initial":{"content":"L"},"m3-list-item-headline":{"content":"Lantern Repair Café"},"m3-list-item-supporting":{"content":"Markveien 58 · Sat 11:00"},"m3-list-item-trailing":{"content":"350 m"}}},
    {"type":"ref","id":"m3-screen-item-2","ref":"m3-list-item","width":"fill_container","descendants":{"m3-list-item-initial":{"content":"F"},"m3-list-item-headline":{"content":"Fikselaget Tøyen"},"m3-list-item-supporting":{"content":"Hagegata 22 · Sun 12:00"},"m3-list-item-trailing":{"content":"1.2 km"}}},
    {"type":"ref","id":"m3-screen-item-3","ref":"m3-list-item","width":"fill_container","descendants":{"m3-list-item-initial":{"content":"S"},"m3-list-item-headline":{"content":"Sagene Verksted"},"m3-list-item-supporting":{"content":"Arendalsgata 3 · Wed 17:00"},"m3-list-item-trailing":{"content":"1.9 km"}}},
    {"type":"ref","id":"m3-screen-item-4","ref":"m3-list-item","width":"fill_container","descendants":{"m3-list-item-initial":{"content":"R"},"m3-list-item-headline":{"content":"Reparatørene Grünerløkka"},"m3-list-item-supporting":{"content":"Thorvald Meyers gate 41 · Thu 16:30"},"m3-list-item-trailing":{"content":"2.1 km"}}},
    {"type":"ref","id":"m3-screen-item-5","ref":"m3-list-item","width":"fill_container","descendants":{"m3-list-item-initial":{"content":"B"},"m3-list-item-headline":{"content":"Bjølsen Fiksefest"},"m3-list-item-supporting":{"content":"Maridalsveien 120 · Sat 12:00"},"m3-list-item-trailing":{"content":"2.8 km"}}},
    {"type":"ref","id":"m3-screen-item-6","ref":"m3-list-item","width":"fill_container","descendants":{"m3-list-item-initial":{"content":"G"},"m3-list-item-headline":{"content":"Gamlebyen Sykkelkjeller"},"m3-list-item-supporting":{"content":"Schweigaards gate 60 · Tue 18:00"},"m3-list-item-trailing":{"content":"3.4 km"}}},
    {"type":"ref","id":"m3-screen-item-7","ref":"m3-list-item","width":"fill_container","descendants":{"m3-list-item-initial":{"content":"M"},"m3-list-item-headline":{"content":"Majorstuen Syhjørne"},"m3-list-item-supporting":{"content":"Bogstadveien 9 · Mon 17:00"},"m3-list-item-trailing":{"content":"4.0 km"}}},
    {"type":"ref","id":"m3-screen-item-8","ref":"m3-list-item","width":"fill_container","descendants":{"m3-list-item-initial":{"content":"N"},"m3-list-item-headline":{"content":"Nydalen Elektrobord"},"m3-list-item-supporting":{"content":"Gullhaugveien 7 · Sun 13:00"},"m3-list-item-trailing":{"content":"4.6 km"}}}]},
  {"type":"frame","id":"m3-screen-bottom","name":"Bottom","width":"fill_container","layout":"vertical","fill":"$surface-container","children":[
    {"type":"ref","id":"m3-screen-nav","ref":"m3-nav-bar","width":"fill_container","fill":"#00000000"},
    {"type":"frame","id":"m3-screen-gesture","name":"Gesture area","width":"fill_container","height":24,"justifyContent":"center","alignItems":"center","children":[
      {"type":"rectangle","id":"m3-screen-handle","name":"Handle","width":108,"height":4,"fill":"$on-surface","cornerRadius":2}]}]},
  {"type":"frame","id":"m3-screen-fab-box","width":380,"justifyContent":"end","layoutPosition":"absolute","x":16,"y":755,"children":[
    {"type":"ref","id":"m3-screen-fab","ref":"m3-fab"}]}]}
```
