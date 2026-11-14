# Glass components

Each block is one layer to insert with `edit_layers` (`{"op":"insert","node":<block>}`), as a reusable
component. Insert the ones the design needs **once**, on a board beside the screens (for example a frame
named "Glass components" at x -600 with `"layout":"vertical","gap":40,"fill":"#8a93a3"`; a mid grey shows
both light and dark glass), then use them in screens as instances:

```json
{"type":"ref","ref":"glass-tab-bar","x":21,"y":769,
 "descendants":{"glass-tab-1-label":{"content":"Today"},"glass-tab-1-icon":{"icon":"sun"}}}
```

Rules that hold for all of them:

- They use the variables of `references/tokens.md`; set those up first.
- Labels use `$on-glass` and selections `$selection-on-glass`, which follow the screen's `Mode`: in a
  frame with `"theme":{"Mode":"Dark"}` they are white without any override. Panes with a pinned
  `"appearance":"light"` (sheet, alert, menu, sidebar) need `"appearance":"dark"` in a dark screen: override
  `material` on the instance. Where `check_glass` reports a glass that came out the other way than its screen
  (a bar over a dark photo in a light screen), override that instance's labels to `$on-dark-glass` through
  `descendants`, or give the instance `"theme":{"Mode":"Dark"}`.
- The texts carry no `fontFamily`, so they are drawn in the default face: add the design's `fontFamily` to
  each text when you insert a component (or through `descendants`).
- Spacing follows steps of 4. Change sizes in those steps.
- Ids start with `glass-`; if the document already has that id, the component is already there: instance it.
- In a screen, place bars with `x`/`y` in a `"layout":"none"` frame, after the content in the layer order.

## Tab bar with search

A glass group: the tab capsule and the round search button are one piece of glass. The selected tab is a
fill; the others have none. 351 wide for a 393 screen at x 21, y 769.

```json
{"type":"frame","id":"glass-tab-bar","name":"Tab bar","reusable":true,"layout":"horizontal","gap":8,"alignItems":"center","glassGroup":8,"children":[
  {"type":"frame","id":"glass-tabs","name":"Tabs","width":281,"height":62,"cornerRadius":31,"padding":4,"layout":"horizontal","justifyContent":"space_between","alignItems":"center","material":{"type":"liquid_glass","depth":"bar"},"stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
    {"type":"frame","id":"glass-tab-1","width":88,"height":54,"cornerRadius":27,"layout":"vertical","gap":4,"justifyContent":"center","alignItems":"center","fill":"$selection-on-glass","children":[
      {"type":"icon","id":"glass-tab-1-icon","icon":"house","width":20,"height":20,"fill":"$on-glass"},
      {"type":"text","id":"glass-tab-1-label","content":"Home","fontSize":11,"fontWeight":"600","fill":"$on-glass"}]},
    {"type":"frame","id":"glass-tab-2","width":88,"height":54,"cornerRadius":27,"layout":"vertical","gap":4,"justifyContent":"center","alignItems":"center","children":[
      {"type":"icon","id":"glass-tab-2-icon","icon":"images","width":20,"height":20,"fill":"$on-glass"},
      {"type":"text","id":"glass-tab-2-label","content":"Library","fontSize":11,"fontWeight":"600","fill":"$on-glass"}]},
    {"type":"frame","id":"glass-tab-3","width":88,"height":54,"cornerRadius":27,"layout":"vertical","gap":4,"justifyContent":"center","alignItems":"center","children":[
      {"type":"icon","id":"glass-tab-3-icon","icon":"heart","width":20,"height":20,"fill":"$on-glass"},
      {"type":"text","id":"glass-tab-3-label","content":"Saved","fontSize":11,"fontWeight":"600","fill":"$on-glass"}]}]},
  {"type":"frame","id":"glass-tab-search","name":"Search tab","width":62,"height":62,"cornerRadius":31,"justifyContent":"center","alignItems":"center","material":{"type":"liquid_glass","depth":"bar"},"stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
    {"type":"icon","id":"glass-tab-search-icon","icon":"search","width":24,"height":24,"fill":"$on-glass"}]}]}
```

The selected tab is marked by its fill; its label keeps the colour of the others, because an accent colour
rarely holds 4.5:1 on glass. Two or four tabs: change the tabs' `width` so they fill the 273 inside the
capsule. An accessory (a mini player) is a 48-high
capsule with the same material placed 8 above the bar, in a vertical glass group with it.

## Toolbar: back button, grouped actions, primary action

Three separate pieces of glass. The grouped actions share one capsule; the tinted primary action stands
alone at the trailing end (a glass group gives all its children the first child's material, so a tinted
button never goes inside one). 361 wide for a 393 screen at x 16, y 58.

```json
{"type":"frame","id":"glass-toolbar","name":"Toolbar","reusable":true,"width":361,"layout":"horizontal","justifyContent":"space_between","alignItems":"center","children":[
  {"type":"frame","id":"glass-toolbar-back","name":"Back","width":44,"height":44,"cornerRadius":22,"justifyContent":"center","alignItems":"center","material":"liquid_glass","stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
    {"type":"icon","id":"glass-toolbar-back-icon","icon":"chevron-left","width":22,"height":22,"fill":"$on-glass"}]},
  {"type":"frame","id":"glass-toolbar-trailing","layout":"horizontal","gap":8,"alignItems":"center","children":[
    {"type":"frame","id":"glass-toolbar-actions","name":"Actions","height":44,"cornerRadius":22,"padding":[0,16],"gap":16,"layout":"horizontal","alignItems":"center","material":"liquid_glass","stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
      {"type":"icon","id":"glass-toolbar-action-1","icon":"share","width":22,"height":22,"fill":"$on-glass"},
      {"type":"icon","id":"glass-toolbar-action-2","icon":"bookmark","width":22,"height":22,"fill":"$on-glass"},
      {"type":"icon","id":"glass-toolbar-action-3","icon":"ellipsis","width":22,"height":22,"fill":"$on-glass"}]},
    {"type":"frame","id":"glass-toolbar-primary","name":"Done","width":44,"height":44,"cornerRadius":22,"justifyContent":"center","alignItems":"center","material":{"type":"liquid_glass","tint":"$accent"},"children":[
      {"type":"icon","id":"glass-toolbar-primary-icon","icon":"check","width":22,"height":22,"fill":"#ffffff"}]}]}]}
```

## Round button and prominent button

```json
{"type":"frame","id":"glass-round-button","name":"Round button","reusable":true,"width":44,"height":44,"cornerRadius":22,"justifyContent":"center","alignItems":"center","material":"liquid_glass","stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"icon","id":"glass-round-button-icon","icon":"plus","width":22,"height":22,"fill":"$on-glass"}]}
```

```json
{"type":"frame","id":"glass-prominent-button","name":"Prominent button","reusable":true,"height":50,"cornerRadius":25,"padding":[0,24],"justifyContent":"center","alignItems":"center","material":{"type":"liquid_glass","tint":"$accent"},"children":[
  {"type":"text","id":"glass-prominent-button-label","content":"Continue","fontSize":17,"fontWeight":"600","fill":"#ffffff"}]}
```

One prominent button per screen. A floating action button is the round button at 56 x 56 (radius 28).

## Segmented control

One glass capsule; the selected segment is a fill, 3 inside the capsule, radius 18 - 3 = 15.

```json
{"type":"frame","id":"glass-segmented","name":"Segmented control","reusable":true,"height":36,"cornerRadius":18,"padding":3,"layout":"horizontal","alignItems":"center","material":"liquid_glass","stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"glass-segment-1","width":96,"height":30,"cornerRadius":15,"justifyContent":"center","alignItems":"center","fill":"$selection-on-glass","children":[
    {"type":"text","id":"glass-segment-1-label","content":"Day","fontSize":13,"fontWeight":"600","fill":"$on-glass"}]},
  {"type":"frame","id":"glass-segment-2","width":96,"height":30,"cornerRadius":15,"justifyContent":"center","alignItems":"center","children":[
    {"type":"text","id":"glass-segment-2-label","content":"Week","fontSize":13,"fontWeight":"600","fill":"$on-glass"}]},
  {"type":"frame","id":"glass-segment-3","width":96,"height":30,"cornerRadius":15,"justifyContent":"center","alignItems":"center","children":[
    {"type":"text","id":"glass-segment-3-label","content":"Month","fontSize":13,"fontWeight":"600","fill":"$on-glass"}]}]}
```

## Search field

```json
{"type":"frame","id":"glass-search-field","name":"Search field","reusable":true,"width":361,"height":44,"cornerRadius":22,"padding":[0,16],"gap":8,"layout":"horizontal","alignItems":"center","material":"liquid_glass","stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"icon","id":"glass-search-field-icon","icon":"search","width":20,"height":20,"fill":"$on-glass"},
  {"type":"text","id":"glass-search-field-text","content":"Search","fontSize":17,"fontWeight":"400","fill":"$on-glass","opacity":0.6}]}
```

`check_glass` measures a text's colour, not its `opacity`: quiet text made with `opacity` is unchecked, so
judge it by eye and leave the opacity off when the backdrop is busy.

## Slider and toggle: plain at rest, a glass knob while pressed

These live in the content, so at rest they are ordinary fills. Only the frame that shows the control being
moved has a glass knob, slightly larger than the resting one.

```json
{"type":"frame","id":"glass-slider","name":"Slider","reusable":true,"width":280,"height":28,"layout":"none","children":[
  {"type":"rectangle","id":"glass-slider-track","x":0,"y":11,"width":280,"height":6,"cornerRadius":3,"fill":"#7878803d"},
  {"type":"rectangle","id":"glass-slider-fill","x":0,"y":11,"width":150,"height":6,"cornerRadius":3,"fill":"$accent"},
  {"type":"ellipse","id":"glass-slider-knob","x":136,"y":0,"width":28,"height":28,"fill":"#ffffff","effect":{"type":"shadow","shadowType":"outer","color":"#00000033","offset":{"x":0,"y":2},"blur":6}}]}
```

```json
{"type":"frame","id":"glass-slider-pressed","name":"Slider, pressed","reusable":true,"width":280,"height":36,"layout":"none","children":[
  {"type":"rectangle","id":"glass-slider-pressed-track","x":0,"y":15,"width":280,"height":6,"cornerRadius":3,"fill":"#7878803d"},
  {"type":"rectangle","id":"glass-slider-pressed-fill","x":0,"y":15,"width":150,"height":6,"cornerRadius":3,"fill":"$accent"},
  {"type":"frame","id":"glass-slider-pressed-knob","x":126,"y":0,"width":48,"height":36,"cornerRadius":18,"material":"liquid_glass"}]}
```

A toggle is the same idea: a 51 x 31 capsule track (`$accent` when on, `#7878803d` when off) with a white
27-high knob; in the pressed frame the knob is a 38 x 31 glass capsule.

## Half-height sheet

Inset 8 from the screen edges, so content shows around it; with a dimming rectangle over the content when
it is modal. A large pane: pin its `appearance` to the screen's theme.

```json
{"type":"frame","id":"glass-sheet","name":"Sheet","reusable":true,"width":377,"height":420,"cornerRadius":38,"padding":[12,20,24,20],"gap":16,"layout":"vertical","alignItems":"center","material":{"type":"liquid_glass","appearance":"light"},"stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"rectangle","id":"glass-sheet-grabber","width":36,"height":5,"cornerRadius":2.5,"fill":"#3c3c434d"},
  {"type":"text","id":"glass-sheet-title","content":"Share with","fontSize":17,"fontWeight":"600","fill":"$on-glass"},
  {"type":"frame","id":"glass-sheet-content","width":"fill_container","height":"fill_container","layout":"vertical","gap":12}]}
```

Fill `glass-sheet-content` through `descendants` (an entry with `children`). At full height a sheet is more
opaque: add `"advanced":{"tintOpacity":0.8}`.

## Alert

```json
{"type":"frame","id":"glass-alert","name":"Alert","reusable":true,"width":300,"cornerRadius":30,"padding":20,"gap":16,"layout":"vertical","material":{"type":"liquid_glass","appearance":"light"},"stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"glass-alert-text","width":"fill_container","layout":"vertical","gap":6,"children":[
    {"type":"text","id":"glass-alert-title","content":"Delete this album?","fontSize":17,"fontWeight":"700","fill":"$on-glass","width":"fill_container","textGrowth":"fixed-width"},
    {"type":"text","id":"glass-alert-message","content":"Its 24 photos stay in your library.","fontSize":15,"fontWeight":"400","fill":"$on-glass","width":"fill_container","textGrowth":"fixed-width"}]},
  {"type":"frame","id":"glass-alert-buttons","width":"fill_container","layout":"horizontal","gap":8,"children":[
    {"type":"frame","id":"glass-alert-cancel","width":"fill_container","height":46,"cornerRadius":23,"justifyContent":"center","alignItems":"center","fill":"$selection-on-glass","children":[
      {"type":"text","id":"glass-alert-cancel-label","content":"Cancel","fontSize":17,"fontWeight":"600","fill":"$on-glass"}]},
    {"type":"frame","id":"glass-alert-confirm","width":"fill_container","height":46,"cornerRadius":23,"justifyContent":"center","alignItems":"center","fill":"$accent","children":[
      {"type":"text","id":"glass-alert-confirm-label","content":"Delete","fontSize":17,"fontWeight":"600","fill":"#ffffff"}]}]}]}
```

The buttons on an alert are fills, not glass: they sit on glass. Put a `#00000033` rectangle over the screen
behind the alert.

## Menu, popover, action sheet

One pane next to the control that opened it (touching it or 8 away), never overlapping other glass. Rows are
plain; a separator is a 1-high fill.

```json
{"type":"frame","id":"glass-menu","name":"Menu","reusable":true,"width":240,"cornerRadius":22,"padding":[8,0],"layout":"vertical","material":{"type":"liquid_glass","appearance":"light","depth":"panel"},"stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"glass-menu-row-1","width":"fill_container","height":44,"padding":[0,16],"gap":12,"layout":"horizontal","alignItems":"center","children":[
    {"type":"icon","id":"glass-menu-row-1-icon","icon":"copy","width":20,"height":20,"fill":"$on-glass"},
    {"type":"text","id":"glass-menu-row-1-label","content":"Duplicate","fontSize":17,"fontWeight":"400","fill":"$on-glass"}]},
  {"type":"frame","id":"glass-menu-row-2","width":"fill_container","height":44,"padding":[0,16],"gap":12,"layout":"horizontal","alignItems":"center","children":[
    {"type":"icon","id":"glass-menu-row-2-icon","icon":"folder","width":20,"height":20,"fill":"$on-glass"},
    {"type":"text","id":"glass-menu-row-2-label","content":"Move to...","fontSize":17,"fontWeight":"400","fill":"$on-glass"}]},
  {"type":"rectangle","id":"glass-menu-separator","width":"fill_container","height":1,"fill":"#3c3c4333"},
  {"type":"frame","id":"glass-menu-row-3","width":"fill_container","height":44,"padding":[0,16],"gap":12,"layout":"horizontal","alignItems":"center","children":[
    {"type":"icon","id":"glass-menu-row-3-icon","icon":"trash-2","width":20,"height":20,"fill":"#d70015"},
    {"type":"text","id":"glass-menu-row-3-label","content":"Delete","fontSize":17,"fontWeight":"400","fill":"#d70015"}]}]}
```

An action sheet is this menu, wider (up to 361), placed directly above or below its trigger.

## Sidebar (tablet and desktop)

An inset floating pane: 8-12 from the window's edges, content running behind it. Pin its appearance. Its
corner radius is the window's radius less the inset.

```json
{"type":"frame","id":"glass-sidebar","name":"Sidebar","reusable":true,"width":260,"height":700,"cornerRadius":20,"padding":[16,12],"gap":4,"layout":"vertical","material":{"type":"liquid_glass","appearance":"light"},"stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"text","id":"glass-sidebar-title","content":"Library","fontSize":13,"fontWeight":"600","fill":"$on-glass","opacity":0.6},
  {"type":"frame","id":"glass-sidebar-row-1","width":"fill_container","height":36,"cornerRadius":10,"padding":[0,10],"gap":10,"layout":"horizontal","alignItems":"center","fill":"$selection-on-glass","children":[
    {"type":"icon","id":"glass-sidebar-row-1-icon","icon":"images","width":18,"height":18,"fill":"$accent"},
    {"type":"text","id":"glass-sidebar-row-1-label","content":"All photos","fontSize":15,"fontWeight":"500","fill":"$on-glass"}]},
  {"type":"frame","id":"glass-sidebar-row-2","width":"fill_container","height":36,"cornerRadius":10,"padding":[0,10],"gap":10,"layout":"horizontal","alignItems":"center","children":[
    {"type":"icon","id":"glass-sidebar-row-2-icon","icon":"heart","width":18,"height":18,"fill":"$on-glass"},
    {"type":"text","id":"glass-sidebar-row-2-label","content":"Favourites","fontSize":15,"fontWeight":"500","fill":"$on-glass"}]},
  {"type":"frame","id":"glass-sidebar-row-3","width":"fill_container","height":36,"cornerRadius":10,"padding":[0,10],"gap":10,"layout":"horizontal","alignItems":"center","children":[
    {"type":"icon","id":"glass-sidebar-row-3-icon","icon":"clock","width":18,"height":18,"fill":"$on-glass"},
    {"type":"text","id":"glass-sidebar-row-3-label","content":"Recents","fontSize":15,"fontWeight":"500","fill":"$on-glass"}]}]}
```

For a dark window: put the window in `"theme":{"Mode":"Dark"}` and override the instance's `material` to
`{"type":"liquid_glass","appearance":"dark"}`.

## Media transport (clear glass over artwork)

Not a component, a pattern: over full-bleed artwork the controls are clear glass with white icons and no
text. One glass group holds the transport: the play capsule first (it decides the thickness), the small
round buttons after it, a fixed `gap` equal to `glassGroup`. Nothing is tinted: with clear glass the artwork
is the colour. Title, artist and times are content: they lie on a gradient scrim (clear to `#00000099`) at
the bottom of the artwork, not on glass. The scrubber is a plain white track at 30 % with a white fill.

```json
{"type":"frame","name":"Transport","layout":"horizontal","gap":16,"alignItems":"center","glassGroup":16,"children":[
  {"type":"frame","name":"Play","width":200,"height":72,"cornerRadius":36,"gap":32,"layout":"horizontal","justifyContent":"center","alignItems":"center","material":{"type":"liquid_glass","variant":"clear"},"children":[
    {"type":"icon","icon":"skip-back","width":26,"height":26,"fill":"#ffffff"},
    {"type":"icon","icon":"pause","width":30,"height":30,"fill":"#ffffff"},
    {"type":"icon","icon":"skip-forward","width":26,"height":26,"fill":"#ffffff"}]},
  {"type":"frame","name":"Shuffle","width":48,"height":48,"cornerRadius":24,"justifyContent":"center","alignItems":"center","material":{"type":"liquid_glass","variant":"clear"},"children":[
    {"type":"icon","icon":"shuffle","width":20,"height":20,"fill":"#ffffff"}]}]}
```

## Toast

```json
{"type":"frame","id":"glass-toast","name":"Toast","reusable":true,"height":44,"cornerRadius":22,"padding":[0,16],"gap":8,"layout":"horizontal","alignItems":"center","material":"liquid_glass","stroke":"$glass-outline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"icon","id":"glass-toast-icon","icon":"check","width":18,"height":18,"fill":"$on-glass"},
  {"type":"text","id":"glass-toast-label","content":"Saved to Library","fontSize":15,"fontWeight":"600","fill":"$on-glass"}]}
```

## A frosted panel in content

Not a component, just the pattern: a card or panel inside the content layer that needs to stand off a
picture gets the frosted material and ordinary labels.

```json
{"type":"frame","name":"Details","width":320,"cornerRadius":24,"padding":20,"gap":8,"layout":"vertical","material":{"type":"frosted","thickness":"regular"},"children":[
  {"type":"text","content":"Harbour at dusk","fontSize":20,"fontWeight":"700","fill":"#000000"},
  {"type":"text","content":"34 photos, shared with 3 people","fontSize":15,"fontWeight":"400","fill":"#000000"}]}
```

Thickness by purpose: `ultra_thin` and `thin` over calm pictures with bold labels; `regular` for body text;
`thick` for fine or secondary text. In a dark screen add `"appearance":"dark"` and white labels.
