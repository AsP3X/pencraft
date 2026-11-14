# Overlays: what floats above the page

Menus, popovers, tooltips, dialogs, drawers, toasts and the command palette. They share one build: the
`$popover` fill, the `$hairline`, a shadow from `references/tokens.md`, and a fixed distance to what opened
them. Section 6 of the skill says where each goes in a screen. Buttons inside them are instances of
`ui-button` from `references/controls.md`: insert that first.

## Menu

For a dropdown menu, a context menu and the list of a select. Padding 4; rows 32 high with 8 inside, 8
between icon and label, radius `$radius-sm`; no gap between rows. At least 128 wide (a select's list: as wide
as its trigger).

```json
{"type":"frame","id":"ui-menu","name":"Menu","reusable":true,"width":224,"layout":"vertical","padding":4,"cornerRadius":"$radius-md","fill":"$popover","stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"color":"#0000000f"},{"type":"shadow","offset":{"x":0,"y":6},"blur":14,"color":"#00000014"}],"children":[
  {"type":"frame","id":"ui-menu-label","name":"Group label","width":"fill_container","height":28,"padding":[0,8],"alignItems":"center","children":[
    {"type":"text","id":"ui-menu-label-text","content":"Project","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-menu-item-1","name":"Item, highlighted","width":"fill_container","height":32,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-sm","fill":"$accent","children":[
    {"type":"icon","id":"ui-menu-item-1-icon","library":"lucide","icon":"pencil","width":16,"height":16,"fill":"$muted-foreground"},
    {"type":"text","id":"ui-menu-item-1-label","content":"Rename","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fill":"$foreground"},
    {"type":"text","id":"ui-menu-item-1-shortcut","content":"F2","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-menu-item-2","name":"Item","width":"fill_container","height":32,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-sm","children":[
    {"type":"icon","id":"ui-menu-item-2-icon","library":"lucide","icon":"copy","width":16,"height":16,"fill":"$muted-foreground"},
    {"type":"text","id":"ui-menu-item-2-label","content":"Duplicate","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fill":"$foreground"},
    {"type":"text","id":"ui-menu-item-2-shortcut","content":"Ctrl D","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-menu-item-3","name":"Item, checked","width":"fill_container","height":32,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-sm","children":[
    {"type":"icon","id":"ui-menu-item-3-icon","library":"lucide","icon":"star","width":16,"height":16,"fill":"$muted-foreground"},
    {"type":"text","id":"ui-menu-item-3-label","content":"Pinned to the sidebar","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fill":"$foreground"},
    {"type":"icon","id":"ui-menu-item-3-indicator","name":"Indicator","library":"lucide","icon":"check","width":16,"height":16,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-menu-separator","name":"Separator","width":"fill_container","height":9,"padding":[4,0],"children":[
    {"type":"rectangle","id":"ui-menu-separator-line","width":"fill_container","height":1,"fill":"$border"}]},
  {"type":"frame","id":"ui-menu-item-4","name":"Item, destructive","width":"fill_container","height":32,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-sm","children":[
    {"type":"icon","id":"ui-menu-item-4-icon","library":"lucide","icon":"trash","width":16,"height":16,"fill":"$destructive"},
    {"type":"text","id":"ui-menu-item-4-label","content":"Delete project","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fill":"$destructive"}]}]}
```

- Highlighted (the pointer or the keyboard is on the row) is the `$accent` fill: one row at most. Checked is
  the mark at the trailing edge. A row can be both; they are different things.
- A destructive row is last, under a separator, in `$destructive`; highlighted it takes `$destructive-tint`.
- A row that opens a sub-menu ends in a `chevron-right`; the sub-menu sits to the right of the row, its first
  row level with it, overlapping the menu by 4, with the large shadow.
- A disabled row has `"opacity":0.5`. Either every row has a leading icon or none has.
- Shortcuts are plain 12 text in `$muted-foreground`, written as the keyboard says them ("Ctrl D").
- A select's list has no icons and no group label; the chosen option carries the check.
- Compact: rows 28 high with 6 inside, menu radius `$radius-lg`.

## Popover

Free content next to its trigger, 288 wide, padding 16, parts 16 apart; 8 from the trigger when it has an
arrow or content of its own, otherwise 4.

```json
{"type":"frame","id":"ui-popover","name":"Popover","reusable":true,"width":288,"layout":"vertical","gap":16,"padding":16,"cornerRadius":"$radius-md","fill":"$popover","stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"color":"#0000000f"},{"type":"shadow","offset":{"x":0,"y":6},"blur":14,"color":"#00000014"}],"children":[
  {"type":"frame","id":"ui-popover-header","name":"Header","width":"fill_container","layout":"vertical","gap":4,"children":[
    {"type":"text","id":"ui-popover-title","name":"Title","content":"Share this board","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-popover-description","name":"Description","content":"Anyone with the link can view it.","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.45,"fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
  {"type":"ref","id":"ui-popover-action","ref":"ui-button","width":"fill_container","descendants":{"ui-button-label":{"content":"Copy link"}}}]}
```

A hover card is the same at 256 wide with `$radius-lg`.

## Tooltip

The colours turned round: `$foreground` as the fill, `$background` as the text. 12 text, 12 beside it and 6
above and below; one short line, at most 320 wide. 4 above its trigger, centred. No shadow, no hairline.

```json
{"type":"frame","id":"ui-tooltip","name":"Tooltip","reusable":true,"padding":[6,12],"gap":6,"alignItems":"center","cornerRadius":"$radius-md","fill":"$foreground","children":[
  {"type":"text","id":"ui-tooltip-label","name":"Label","content":"Add to library","fontFamily":"$font","fontSize":12,"fill":"$background"}]}
```

A tooltip names a control that has no visible label. It never holds what someone needs in order to act, and
nothing in it can be pressed.

## Dialog

448 wide (Compact 384), padding 24, parts 24 apart, radius `$radius-xl`, centred over the scrim. The title
says what happens, the buttons name the action. Cancel (outline) comes before the main action, both at the
trailing end, 8 apart. The close mark is level with the title.

```json
{"type":"frame","id":"ui-dialog","name":"Dialog","reusable":true,"width":448,"layout":"vertical","gap":24,"padding":24,"cornerRadius":"$radius-xl","fill":"$popover","stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"ui-dialog-header","name":"Header","width":"fill_container","layout":"vertical","gap":8,"children":[
    {"type":"frame","id":"ui-dialog-title-row","width":"fill_container","gap":16,"alignItems":"center","children":[
      {"type":"text","id":"ui-dialog-title","name":"Title","content":"Invite a teammate","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":16,"fontWeight":"500","fill":"$foreground"},
      {"type":"icon","id":"ui-dialog-close","name":"Close","library":"lucide","icon":"x","width":16,"height":16,"fill":"$muted-foreground"}]},
    {"type":"text","id":"ui-dialog-description","name":"Description","content":"They can open the Harbour Rota workspace as soon as they accept.","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.45,"fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-dialog-body","name":"Body","width":"fill_container","layout":"vertical","gap":8,"children":[
    {"type":"text","id":"ui-dialog-field-label","name":"Label","content":"Work email","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"ref","id":"ui-dialog-field-control","ref":"ui-input","width":"fill_container"}]},
  {"type":"frame","id":"ui-dialog-footer","name":"Footer","width":"fill_container","gap":8,"justifyContent":"end","children":[
    {"type":"ref","id":"ui-dialog-cancel","ref":"ui-button","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","descendants":{"ui-button-label":{"content":"Cancel","fill":"$foreground"}}},
    {"type":"ref","id":"ui-dialog-confirm","ref":"ui-button","descendants":{"ui-button-label":{"content":"Send invite"}}}]}]}
```

In a screen, the scrim of `references/tokens.md` goes under it:

```json
{"type":"frame","name":"Screen","width":1280,"height":800,"layout":"none","clip":true,"fill":"$background","children":[
  <the page>,
  {"type":"rectangle","name":"Scrim","x":0,"y":0,"width":1280,"height":800,"fill":"$scrim"},
  {"type":"ref","ref":"ui-dialog","x":416,"y":240}]}
```

A dialog of your own is an instance with its texts, body and button labels overridden. The body takes any
number of fields, 24 apart; the buttons are instances inside the instance, so their labels are reached by
nesting `descendants`:

```json
{"type":"ref","id":"ui-dialog-example","name":"Workspace settings dialog","ref":"ui-dialog","descendants":{
  "ui-dialog-title":{"content":"Workspace settings"},
  "ui-dialog-description":{"content":"These apply to everyone in Northlight Surveying."},
  "ui-dialog-body":{"gap":24,"children":[
    {"type":"frame","id":"ui-dialog-example-field","name":"Field","width":"fill_container","layout":"vertical","gap":8,"children":[
      {"type":"text","id":"ui-dialog-example-label","name":"Label","content":"Workspace name","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
      {"type":"ref","id":"ui-dialog-example-control","ref":"ui-input","width":"fill_container","descendants":{"ui-input-value":{"content":"Northlight Surveying","fill":"$foreground"}}}]}]},
  "ui-dialog-confirm":{"descendants":{"ui-button-label":{"content":"Save changes"}}}}}
```

Centre it by hand: `x` is (screen width - 448) / 2, `y` is (screen height - the dialog's height) / 2, the
height read with `read_layers` after inserting.

An alert dialog interrupts to confirm something that cannot be undone: the same shell at 512 wide, an 18
title at weight 500 that asks the question ("Delete the Harbour Rota workspace?"), a description that says
what is lost, no close mark, and two buttons: Cancel, and the action named in full ("Delete workspace") as a
destructive button. It has no other content.

A sheet is a dialog fixed to a side: full height, 384 wide at the right edge, padding 16 in header and
footer, the footer at the bottom, a line only on its inner edge, the large shadow, and the scrim.

## Drawer

The bottom sheet of a phone. As wide as the screen, the top corners 14, a handle 100 by 6 in `$muted` 16
from the top, header and footer with 16 inside, buttons 44 high, as wide as the drawer and 8 apart, the main
action first (on top). Between header and footer is an empty body: put the drawer's fields into it through
`descendants` (`"ui-drawer-body":{"padding":[8,16],"children":[...]}`), 24 apart, their controls 44 high with
16 text.

```json
{"type":"frame","id":"ui-drawer","name":"Drawer","reusable":true,"width":390,"layout":"vertical","alignItems":"center","padding":[16,0,0,0],"cornerRadius":[14,14,0,0],"fill":"$popover","children":[
  {"type":"rectangle","id":"ui-drawer-handle","name":"Handle","width":100,"height":6,"cornerRadius":3,"fill":"$muted"},
  {"type":"frame","id":"ui-drawer-header","name":"Header","width":"fill_container","layout":"vertical","gap":4,"padding":16,"children":[
    {"type":"text","id":"ui-drawer-title","name":"Title","content":"Move to another shift","fontFamily":"$font","fontSize":16,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-drawer-description","name":"Description","content":"Maren keeps her break at 12:30.","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.45,"fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-drawer-body","name":"Body","width":"fill_container","layout":"vertical","gap":24,"padding":[0,16]},
  {"type":"frame","id":"ui-drawer-footer","name":"Footer","width":"fill_container","layout":"vertical","gap":8,"padding":[8,16,24,16],"children":[
    {"type":"ref","id":"ui-drawer-confirm","ref":"ui-button","width":"fill_container","height":44,"padding":[0,12],"descendants":{"ui-button-label":{"content":"Move shift","fontSize":16}}},
    {"type":"ref","id":"ui-drawer-cancel","ref":"ui-button","width":"fill_container","height":44,"fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","padding":[0,12],"descendants":{"ui-button-label":{"content":"Cancel","fontSize":16,"fill":"$foreground"}}}]}]}
```

The drawer has no line and no shadow of its own: its lighter fill and the scrim set it apart (a line along
its top only would square its corners). In a screen it lies at the bottom over the scrim. To show the page behind stepping back, as phones do: the
page frame gets a radius of 8, is drawn 14 lower and 26 narrower (centred) on a black screen. A drawer that
snaps to several heights is shown as one frame per height.

## Toast

356 wide, padding 16, radius `$radius-lg`, the large shadow. A 16 icon, then a title of 13 at weight 500
and, 2 under it, a description of 13 in `$muted-foreground`; an action, if any, is a 24 high button at the
trailing edge in the turned-round colours.

```json
{"type":"frame","id":"ui-toast","name":"Toast","reusable":true,"width":356,"padding":16,"gap":8,"alignItems":"center","cornerRadius":"$radius-lg","fill":"$popover","stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","effect":[{"type":"shadow","offset":{"x":0,"y":2},"blur":6,"color":"#0000000f"},{"type":"shadow","offset":{"x":0,"y":12},"blur":28,"color":"#0000001f"}],"children":[
  {"type":"icon","id":"ui-toast-icon","name":"Icon","library":"lucide","icon":"circle-check","width":16,"height":16,"fill":"$foreground"},
  {"type":"frame","id":"ui-toast-content","name":"Content","width":"fill_container","layout":"vertical","gap":2,"children":[
    {"type":"text","id":"ui-toast-title","name":"Title","content":"Invitation sent","fontFamily":"$font","fontSize":13,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-toast-description","name":"Description","content":"Maren gets an email in a minute.","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":13,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-toast-action","name":"Action","height":24,"padding":[0,8],"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-xs","fill":"$primary","children":[
    {"type":"text","id":"ui-toast-action-label","content":"Undo","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$primary-foreground"}]}]}
```

- Kinds by icon: `circle-check` (done), `info`, `triangle-exclamation-point` (warning), `octagon-x` (failed),
  `loader-circle` (working). The toast stays neutral; only the icon may take a state colour.
- Place: 24 from the bottom right corner of a desktop screen. On a phone it is as wide as the screen less 16
  on each side (override `width`), at the bottom: 16 above the bottom edge, or 16 above the top edge of a tab
  bar or an open drawer. A toast lies above a scrim, never under it, and never over the page's title.
- Without an action: `"descendants":{"ui-toast-action":{"enabled":false}}`.
- A stack shows three at most. The newest is in front; each one behind is 14 higher, 5% narrower (338, then
  320 wide, centred), has the front toast's height and no content. Expanded (the pointer is on the stack)
  they are a plain list, 14 apart.
- A toast reports what just happened and goes away by itself. What needs a decision is a dialog; what
  belongs to a field is the field's error.

## Command palette

640 wide at most (512 in a small window), radius `$radius-xl`. In a screen it sits in the top third over the
scrim; shown by itself (when the palette is what was asked for) it stands alone, with no screen and no scrim.
The input row is 48 high with no border of its own, a line under it, then groups: a 12 heading in
`$muted-foreground`, rows 40 high (32 in Compact) with a 16 icon, the label and a shortcut at the trailing
edge. Exactly one row is selected (`$muted`), by default the first.

```json
{"type":"frame","id":"ui-command","name":"Command palette","reusable":true,"width":560,"layout":"vertical","cornerRadius":"$radius-xl","fill":"$popover","stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","effect":[{"type":"shadow","offset":{"x":0,"y":2},"blur":6,"color":"#0000000f"},{"type":"shadow","offset":{"x":0,"y":16},"blur":48,"color":"#00000029"}],"children":[
  {"type":"frame","id":"ui-command-input","name":"Input","width":"fill_container","height":48,"padding":[0,16],"gap":8,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
    {"type":"icon","id":"ui-command-input-icon","library":"lucide","icon":"search","width":16,"height":16,"fill":"$muted-foreground"},
    {"type":"text","id":"ui-command-input-value","name":"Value","content":"Type a command or search","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-command-list","name":"List","width":"fill_container","layout":"vertical","padding":8,"children":[
    {"type":"frame","id":"ui-command-heading","name":"Group heading","width":"fill_container","height":28,"padding":[0,8],"alignItems":"center","children":[
      {"type":"text","id":"ui-command-heading-text","content":"Suggestions","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]},
    {"type":"frame","id":"ui-command-item-1","name":"Item, selected","width":"fill_container","height":40,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-sm","fill":"$muted","children":[
      {"type":"icon","id":"ui-command-item-1-icon","library":"lucide","icon":"calendar","width":16,"height":16,"fill":"$muted-foreground"},
      {"type":"text","id":"ui-command-item-1-label","content":"Open this week's rota","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fill":"$foreground"},
      {"type":"frame","id":"ui-command-item-1-kbd","name":"Kbd","width":20,"height":20,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-xs","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
        {"type":"text","id":"ui-command-item-1-kbd-text","content":"R","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground"}]}]},
    {"type":"frame","id":"ui-command-item-2","name":"Item","width":"fill_container","height":40,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-sm","children":[
      {"type":"icon","id":"ui-command-item-2-icon","library":"lucide","icon":"user-plus","width":16,"height":16,"fill":"$muted-foreground"},
      {"type":"text","id":"ui-command-item-2-label","content":"Invite a teammate","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
    {"type":"frame","id":"ui-command-item-3","name":"Item","width":"fill_container","height":40,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-sm","children":[
      {"type":"icon","id":"ui-command-item-3-icon","library":"lucide","icon":"settings","width":16,"height":16,"fill":"$muted-foreground"},
      {"type":"text","id":"ui-command-item-3-label","content":"Workspace settings","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}]}]}
```

Draw the three moments when the palette is the subject, as section 5 of the skill lays out moments: as
opened (suggestions, the first row selected); typed (the query in `$foreground` with a caret after it, only
rows that contain the query, the best match first and selected, groups without a match gone); and no match
(the list's children replaced by one centred line of 14 in `$muted-foreground`, and its padding by
`[24,16]`: "No results for rota swap"). Each moment is an instance; the list is refilled through
`descendants` with `children` on `ui-command-list`.

A shortcut in a palette is drawn as keys: one 20 square kbd per key, 4 apart ("Ctrl" then "K"; a word key is
as wide as its text plus 4 on each side). In a menu a shortcut is plain 12 text. The selected row here is
`$muted` and the highlighted row of a menu `$accent`: the same grey in the neutral preset, two roles so a
tinted kit can tell them apart. A footer with key hints (a 40 high band, `$muted-half`, a line above it) is the Raycast
manner: add it only when asked.
