# Controls

Each block is one layer to insert with `edit_layers` (`{"op":"insert","node":<block>}`), most of them as a
reusable component. Insert the ones the request needs once, then use instances:

```json
{"type":"ref","ref":"ui-button","descendants":{"ui-button-label":{"content":"Send invite"}}}
```

Rules that hold for all of them:

- They use the variables of `references/tokens.md`; set those up first.
- These are the Classic preset: controls 36 high. For Compact set `height` 32 on buttons, inputs, selects and
  tab lists, and `cornerRadius` `$radius-lg`.
- Sizes of a control: 24 (padding 8, text 12, icon 12), 32, 36 (the default), 40. Override `height` and
  `padding` on the instance; all controls in one row take the same size. On a phone: `height` 44,
  `padding` `[0,12]` (a select `[0,10,0,12]`, a textarea `[10,12]` at its own height), and `"fontSize":16`
  on the text of inputs, selects, textareas and buttons. Field labels and descriptions stay 14.
- Something inside an instance that is itself inside an instance is reached by nesting `descendants`:
  `"descendants":{"ui-dialog-confirm":{"descendants":{"ui-button-label":{"content":"Save"}}}}`. An entry may
  set properties and give `children` at once (`"ui-dialog-body":{"gap":24,"children":[...]}`).
- An override replaces a property; to take a fill away use `"fill":"#00000000"`.
- Ids start with `ui-`; if the document already has that id, the component is there: instance it.
- Part names follow Base UI: Trigger, Popup, Item, Indicator, Track, Thumb, Label, Description, Error.

## Button

```json
{"type":"frame","id":"ui-button","name":"Button","reusable":true,"height":36,"padding":[0,10],"gap":6,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$primary","children":[
  {"type":"text","id":"ui-button-label","name":"Label","content":"Save changes","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$primary-foreground"}]}
```

The six variants are overrides on an instance:

| Variant | On the `ref` | Label (`descendants`) | For |
|---|---|---|---|
| Primary | nothing | nothing | The one main action of a view |
| Secondary | `"fill":"$secondary"` | `"fill":"$foreground"` | A second action of less weight |
| Outline | `"fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner"` | `"fill":"$foreground"` | Cancel, neutral actions, toolbars |
| Ghost | `"fill":"#00000000"` | `"fill":"$foreground"` | Actions inside rows and toolbars that should not draw the eye |
| Destructive | `"fill":"$destructive-tint"` | `"fill":"$destructive"` | Delete, remove: a tint, never solid red |
| Link | `"fill":"#00000000","padding":0` | `"fill":"$primary","underline":true` | An action inside a sentence |

One primary button per view. In a footer the main action is last (on the right), 8 from the one before it.

States: hover `"fill":"$primary-hover"` (secondary `$secondary-hover`, destructive
`$destructive-tint-hover`, outline and ghost `$muted`); pressed as hover and 1 lower; disabled
`"opacity":0.5` on the instance, once; focus in the ring wrapper below.

With a leading icon the padding on the icon's side is 8, and the icon is 16:

```json
{"type":"frame","id":"ui-button-icon-leading","name":"Button, leading icon","reusable":true,"height":36,"padding":[0,10,0,8],"gap":6,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$primary","children":[
  {"type":"icon","id":"ui-button-icon-leading-icon","name":"Icon","library":"lucide","icon":"plus","width":16,"height":16,"fill":"$primary-foreground"},
  {"type":"text","id":"ui-button-icon-leading-label","name":"Label","content":"New project","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$primary-foreground"}]}
```

Loading: set the icon to `loader-circle`, keep the label, `"opacity":0.5`. A button that has no icon at rest
is shown loading as an instance of this one (icon `loader-circle` in the label's colour); give the rest and
the loading instance the same `width` (the loading one's) so the button does not grow.

An icon button is square, as tall as the controls beside it:

```json
{"type":"frame","id":"ui-icon-button","name":"Icon button","reusable":true,"width":36,"height":36,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"icon","id":"ui-icon-button-icon","name":"Icon","library":"lucide","icon":"ellipsis","width":16,"height":16,"fill":"$foreground"}]}
```

## Input

```json
{"type":"frame","id":"ui-input","name":"Input","reusable":true,"width":280,"height":36,"padding":[0,10],"alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"text","id":"ui-input-value","name":"Value","content":"name@company.com","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]}
```

As inserted it shows a placeholder (`$muted-foreground`). Filled: override the value's `content` and
`"fill":"$foreground"`. Being typed in: filled, in the focus ring, with a caret after the text (a rectangle 1
wide and 16 high in `$foreground`, as a second child, 1 from the text). Read-only: filled, with `"fill":"$muted-half"` on the input. Disabled:
`"opacity":0.5`. A placeholder is an example of what to type, never the label.

With a leading icon (search), the icon is 16 in `$muted-foreground`, 8 from the text:

```json
{"type":"frame","id":"ui-search-input","name":"Search input","reusable":true,"width":280,"height":36,"padding":[0,10],"gap":8,"alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"icon","id":"ui-search-input-icon","name":"Icon","library":"lucide","icon":"search","width":16,"height":16,"fill":"$muted-foreground"},
  {"type":"text","id":"ui-search-input-value","name":"Value","content":"Search projects","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]}
```

A textarea has the same skin, 8 above and below its text, and is at least 64 high:

```json
{"type":"frame","id":"ui-textarea","name":"Textarea","reusable":true,"width":280,"height":80,"padding":[8,10],"alignItems":"start","cornerRadius":"$radius-md","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"text","id":"ui-textarea-value","name":"Value","content":"Tell the team what changed and why.","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.45,"fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]}
```

## Select

The trigger. Its list is the menu in `references/overlays.md`, exactly as wide as the trigger, 4 below it.

```json
{"type":"frame","id":"ui-select","name":"Select","reusable":true,"width":280,"height":36,"padding":[0,8,0,10],"gap":6,"justifyContent":"space_between","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"text","id":"ui-select-value","name":"Value","content":"Editor","fontFamily":"$font","fontSize":14,"fill":"$foreground"},
  {"type":"icon","id":"ui-select-icon","name":"Icon","library":"lucide","icon":"chevron-down","width":16,"height":16,"fill":"$muted-foreground"}]}
```

Nothing chosen yet: the value says what to choose ("Choose a role") in `$muted-foreground`.

## Checkbox, radio, switch

Each is a row: the mark, 8, the label. The label is part of the control. Rows of a group are 12 apart.

```json
{"type":"frame","id":"ui-checkbox","name":"Checkbox","reusable":true,"gap":8,"alignItems":"center","children":[
  {"type":"frame","id":"ui-checkbox-box","name":"Box","width":16,"height":16,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-xs","fill":"$primary","children":[
    {"type":"icon","id":"ui-checkbox-indicator","name":"Indicator","library":"lucide","icon":"check","width":12,"height":12,"fill":"$primary-foreground"}]},
  {"type":"text","id":"ui-checkbox-label","name":"Label","content":"Send a welcome email","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}
```

Unchecked: on the box `"fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner"`, and
`"enabled":false` on the indicator. Indeterminate: the indicator's `icon` is `minus`.

```json
{"type":"frame","id":"ui-radio","name":"Radio","reusable":true,"gap":8,"alignItems":"center","children":[
  {"type":"frame","id":"ui-radio-circle","name":"Circle","width":16,"height":16,"justifyContent":"center","alignItems":"center","cornerRadius":8,"fill":"$primary","children":[
    {"type":"ellipse","id":"ui-radio-indicator","name":"Indicator","width":8,"height":8,"fill":"$primary-foreground"}]},
  {"type":"text","id":"ui-radio-label","name":"Label","content":"Everyone in the workspace","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}
```

Not chosen: the circle as the unchecked box, the indicator off. A chosen radio is a filled disc with a light
dot, not a ring with a dark dot.

```json
{"type":"frame","id":"ui-switch","name":"Switch","reusable":true,"gap":8,"alignItems":"center","children":[
  {"type":"frame","id":"ui-switch-track","name":"Track","width":32,"height":18,"padding":2,"justifyContent":"end","alignItems":"center","cornerRadius":9,"fill":"$primary","children":[
    {"type":"ellipse","id":"ui-switch-thumb","name":"Thumb","width":14,"height":14,"fill":"$background"}]},
  {"type":"text","id":"ui-switch-label","name":"Label","content":"Email me a weekly summary","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}
```

Off: on the track `"fill":"$input","justifyContent":"start"`. A switch takes effect at once; where the user
has to press Save, use a checkbox.

## Slider

```json
{"type":"frame","id":"ui-slider","name":"Slider","reusable":true,"width":240,"height":16,"layout":"none","children":[
  {"type":"rectangle","id":"ui-slider-track","name":"Track","x":0,"y":5,"width":240,"height":6,"cornerRadius":3,"fill":"$muted"},
  {"type":"rectangle","id":"ui-slider-range","name":"Indicator","x":0,"y":5,"width":104,"height":6,"cornerRadius":3,"fill":"$primary"},
  {"type":"ellipse","id":"ui-slider-thumb","name":"Thumb","x":96,"y":0,"width":16,"height":16,"fill":"#ffffff","stroke":"$primary","strokeWidth":1,"strokeAlignment":"inner"}]}
```

The indicator ends under the middle of the thumb. Show the value as text beside or above the slider.

## Badge

A pill 20 high. It says a state or a count; it is not pressed.

```json
{"type":"frame","id":"ui-badge","name":"Badge","reusable":true,"height":20,"padding":[0,8],"gap":4,"justifyContent":"center","alignItems":"center","cornerRadius":10,"fill":"$secondary","children":[
  {"type":"text","id":"ui-badge-label","name":"Label","content":"Draft","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground"}]}
```

Variants: primary (`$primary` with `$primary-foreground`), secondary (as inserted), outline
(`"fill":"#00000000","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner"`), destructive
(`$destructive-tint` with `$destructive`).

## Tabs

A list 36 high in `$muted` with 4 inside (so the tabs are 28 high with `$radius-sm`, which is the list's 10 less
the 4); the selected tab is a `$background` fill with the faint shadow, the
others have no fill.

```json
{"type":"frame","id":"ui-tabs","name":"Tabs","reusable":true,"height":36,"padding":4,"alignItems":"center","cornerRadius":"$radius-lg","fill":"$muted","children":[
  {"type":"frame","id":"ui-tabs-tab-1","name":"Tab, selected","height":28,"padding":[0,10],"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-sm","fill":"$background","effect":{"type":"shadow","offset":{"x":0,"y":1},"blur":2,"color":"#0000001a"},"children":[
    {"type":"text","id":"ui-tabs-tab-1-label","name":"Label","content":"Account","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"ui-tabs-tab-2","name":"Tab","height":28,"padding":[0,10],"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-sm","children":[
    {"type":"text","id":"ui-tabs-tab-2-label","name":"Label","content":"Members","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-tabs-tab-3","name":"Tab","height":28,"padding":[0,10],"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-sm","children":[
    {"type":"text","id":"ui-tabs-tab-3-label","name":"Label","content":"Billing","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$muted-foreground"}]}]}
```

The line variant has no list fill: tabs 16 apart, the selected one in `$foreground` with a 2 high
`$foreground` rectangle under it, and one `$border` line under the whole row. Use the line variant for the
sections of a page, the pill variant for a switch between views of one thing. Two to five tabs.

## Field

A field is a label, its control and at most one line under it, 8 apart. Fields are 24 apart. The label is
always visible, above the control, 14 at weight 500. This block is a composition, not a component: build
each field of a form like it.

```json
{"type":"frame","id":"ui-field-example","name":"Field","width":280,"layout":"vertical","gap":8,"children":[
  {"type":"text","id":"ui-field-example-label","name":"Label","content":"Work email","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
  {"type":"ref","id":"ui-field-example-control","ref":"ui-input","width":"fill_container","descendants":{"ui-input-value":{"content":"maren.solheim@harbourrota.no","fill":"$foreground"}}},
  {"type":"text","id":"ui-field-example-description","name":"Description","content":"We send the invitation to this address.","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.45,"fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]}
```

Order: Label, Control, Description, Error. An error replaces the description, in `$destructive`, and says
what to do ("Enter an address with an @"), not only that something is wrong. Mark the few optional fields
with "(optional)" in `$muted-foreground` after the label instead of starring the required ones. A checkbox or
switch field has its description under its label, starting at the label's left edge (24 in for a checkbox,
40 for a switch). A checkbox or switch row among fields is 24 from the field before it, like any field.

## Focus and invalid: the ring wrapper

Pencraft draws one stroke per layer, so the 3 wide halo is a frame around the control: padding 3, `$radius-lg`
(one step above the control's 8), filled with `$ring-soft`. The control inside gets the `$ring` border and a covering
fill. The wrapper is 6 wider and taller than the control: where it stands in a row, give it
`"layoutPosition":"absolute"` over the place of the control so nothing moves.

```json
{"type":"frame","id":"ui-focus-example","name":"Input / focus","padding":3,"cornerRadius":"$radius-lg","fill":"$ring-soft","children":[
  {"type":"ref","id":"ui-focus-example-control","ref":"ui-input","fill":"$background","stroke":"$ring","descendants":{"ui-input-value":{"content":"maren.solheim@","fill":"$foreground"}}}]}
```

The covering fill by button variant: primary, secondary and their hover keep their own fill; outline and
ghost take `$background`; destructive takes both, the tint over the cover:
`"fill":["$background","$destructive-tint"]`. The wrapper's 3 is added around the control, so parts beside
it are drawn 3 closer (the gap of 4 in the invalid example below is the usual 8 less the ring and a bit).

Invalid is the same wrapper with `"fill":"$destructive-tint"` and `"stroke":"$destructive"` (with the same covering fill) on the control,
and the error under the field:

```json
{"type":"frame","id":"ui-invalid-example","name":"Field / invalid","layout":"vertical","gap":4,"children":[
  {"type":"text","id":"ui-invalid-example-label","name":"Label","content":"Work email","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
  {"type":"frame","id":"ui-invalid-example-ring","name":"Ring","padding":3,"cornerRadius":"$radius-lg","fill":"$destructive-tint","children":[
    {"type":"ref","id":"ui-invalid-example-control","ref":"ui-input","fill":"$background","stroke":"$destructive","descendants":{"ui-input-value":{"content":"maren.solheim","fill":"$foreground"}}}]},
  {"type":"text","id":"ui-invalid-example-error","name":"Error","content":"Enter an address with an @, like name@company.com.","fontFamily":"$font","fontSize":14,"fill":"$destructive"}]}
```

Show an error only after the user has left the field or pressed the form's button, never on an empty form.
A focus ring is drawn on one control per screen: the one the keyboard is on.

## State board

When states are asked for. Every cell is a frame of one `width` per column that holds a frame with 3 of
padding, so the focus cell can carry the halo without anything moving: in the focus cell that inner frame is
the ring wrapper. The headings stand 3 in, over the buttons' edge. Rows start with the variant's name. Every
instance in the board has one `width` (the widest loading button's), so no button grows from one state to
the next. Loading is an instance of `ui-button-icon-leading` with the variant's fill on it and the variant's
colour on its icon and label. Copy the rows for every variant that was named and drop the columns that were
not, then copy the whole board into a frame with `"theme":{"Mode":"Dark"}` for dark, named
"<Component> states, dark". The labels of this board are part of it, not a heading around the work.

```json
{"type":"frame","id":"ui-states-example","name":"Button states","layout":"vertical","gap":16,"padding":32,"fill":"$background","children":[
  {"type":"frame","id":"ui-states-head","name":"States","gap":24,"children":[
    {"type":"frame","id":"ui-states-head-0","width":88,"height":16},
    {"type":"frame","id":"ui-states-head-1","width":160,"padding":[0,3],"children":[{"type":"text","id":"ui-states-head-1-text","content":"Rest","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]},
    {"type":"frame","id":"ui-states-head-2","width":160,"padding":[0,3],"children":[{"type":"text","id":"ui-states-head-2-text","content":"Hover","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]},
    {"type":"frame","id":"ui-states-head-3","width":160,"padding":[0,3],"children":[{"type":"text","id":"ui-states-head-3-text","content":"Focus","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]},
    {"type":"frame","id":"ui-states-head-4","width":160,"padding":[0,3],"children":[{"type":"text","id":"ui-states-head-4-text","content":"Disabled","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]},
    {"type":"frame","id":"ui-states-head-5","width":160,"padding":[0,3],"children":[{"type":"text","id":"ui-states-head-5-text","content":"Loading","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]}]},
  {"type":"frame","id":"ui-states-row-primary","name":"Primary","gap":24,"alignItems":"center","children":[
    {"type":"text","id":"ui-states-row-primary-name","content":"Primary","textGrowth":"fixed-width","width":88,"fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"},
    {"type":"frame","id":"ui-states-primary-rest-cell","name":"Cell","width":160,"children":[{"type":"frame","id":"ui-states-primary-rest-pad","padding":3,"children":[{"type":"ref","id":"ui-states-primary-rest","name":"Button / primary / rest","ref":"ui-button","width":144,"descendants":{"ui-button-label":{"content":"Save changes"}}}]}]},
    {"type":"frame","id":"ui-states-primary-hover-cell","name":"Cell","width":160,"children":[{"type":"frame","id":"ui-states-primary-hover-pad","padding":3,"children":[{"type":"ref","id":"ui-states-primary-hover","name":"Button / primary / hover","ref":"ui-button","width":144,"descendants":{"ui-button-label":{"content":"Save changes"}},"fill":"$primary-hover"}]}]},
    {"type":"frame","id":"ui-states-primary-focus-cell","name":"Cell","width":160,"children":[{"type":"frame","id":"ui-states-primary-focus-pad","padding":3,"children":[{"type":"ref","id":"ui-states-primary-focus","name":"Button / primary / focus","ref":"ui-button","width":144,"descendants":{"ui-button-label":{"content":"Save changes"}},"stroke":"$ring","strokeWidth":1,"strokeAlignment":"inner"}],"name":"Ring","cornerRadius":"$radius-lg","fill":"$ring-soft"}]},
    {"type":"frame","id":"ui-states-primary-disabled-cell","name":"Cell","width":160,"children":[{"type":"frame","id":"ui-states-primary-disabled-pad","padding":3,"children":[{"type":"ref","id":"ui-states-primary-disabled","name":"Button / primary / disabled","ref":"ui-button","width":144,"descendants":{"ui-button-label":{"content":"Save changes"}},"opacity":0.5}]}]},
    {"type":"frame","id":"ui-states-primary-loading-cell","name":"Cell","width":160,"children":[{"type":"frame","id":"ui-states-primary-loading-pad","padding":3,"children":[{"type":"ref","id":"ui-states-primary-loading","name":"Button / primary / loading","ref":"ui-button-icon-leading","width":144,"descendants":{"ui-button-icon-leading-icon":{"icon":"loader-circle"},"ui-button-icon-leading-label":{"content":"Save changes"}},"opacity":0.5}]}]}]},
  {"type":"frame","id":"ui-states-row-destructive","name":"Destructive","gap":24,"alignItems":"center","children":[
    {"type":"text","id":"ui-states-row-destructive-name","content":"Destructive","textGrowth":"fixed-width","width":88,"fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"},
    {"type":"frame","id":"ui-states-destructive-rest-cell","name":"Cell","width":160,"children":[{"type":"frame","id":"ui-states-destructive-rest-pad","padding":3,"children":[{"type":"ref","id":"ui-states-destructive-rest","name":"Button / destructive / rest","ref":"ui-button","width":144,"fill":"$destructive-tint","descendants":{"ui-button-label":{"content":"Delete project","fill":"$destructive"}}}]}]},
    {"type":"frame","id":"ui-states-destructive-hover-cell","name":"Cell","width":160,"children":[{"type":"frame","id":"ui-states-destructive-hover-pad","padding":3,"children":[{"type":"ref","id":"ui-states-destructive-hover","name":"Button / destructive / hover","ref":"ui-button","width":144,"fill":"$destructive-tint-hover","descendants":{"ui-button-label":{"content":"Delete project","fill":"$destructive"}}}]}]},
    {"type":"frame","id":"ui-states-destructive-focus-cell","name":"Cell","width":160,"children":[{"type":"frame","id":"ui-states-destructive-focus-pad","padding":3,"children":[{"type":"ref","id":"ui-states-destructive-focus","name":"Button / destructive / focus","ref":"ui-button","width":144,"fill":["$background","$destructive-tint"],"descendants":{"ui-button-label":{"content":"Delete project","fill":"$destructive"}},"stroke":"$ring","strokeWidth":1,"strokeAlignment":"inner"}],"name":"Ring","cornerRadius":"$radius-lg","fill":"$ring-soft"}]},
    {"type":"frame","id":"ui-states-destructive-disabled-cell","name":"Cell","width":160,"children":[{"type":"frame","id":"ui-states-destructive-disabled-pad","padding":3,"children":[{"type":"ref","id":"ui-states-destructive-disabled","name":"Button / destructive / disabled","ref":"ui-button","width":144,"fill":"$destructive-tint","descendants":{"ui-button-label":{"content":"Delete project","fill":"$destructive"}},"opacity":0.5}]}]},
    {"type":"frame","id":"ui-states-destructive-loading-cell","name":"Cell","width":160,"children":[{"type":"frame","id":"ui-states-destructive-loading-pad","padding":3,"children":[{"type":"ref","id":"ui-states-destructive-loading","name":"Button / destructive / loading","ref":"ui-button-icon-leading","width":144,"fill":"$destructive-tint","descendants":{"ui-button-icon-leading-icon":{"icon":"loader-circle","fill":"$destructive"},"ui-button-icon-leading-label":{"content":"Delete project","fill":"$destructive"}},"opacity":0.5}]}]}]}]}
```
