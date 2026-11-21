# Navigation and the frame of an app

The sidebar, the top bar, and what moves someone between pages: a navigation menu, pagination, an accordion, a
stepper and the header of a page. They use the variables of `references/tokens.md`; the avatar is `ui-avatar`
from `references/data.md` and the buttons are those of `references/controls.md`: insert those first. Tabs and
the breadcrumb are in `references/controls.md` and `references/data.md`.

These are the common build, to start from: a product with its own navigation (a rail of pictures, a top
bar only, a sidebar on the trailing side) keeps it, and takes from here only the measures it lacks.

Measured at shadcn/ui's 2026 sidebar and blocks (Carbon's shell has the same 48 header, 256 and 48 side
navigation). The stepper and the page header are not components of those libraries: they are conventions,
built from this kit's tokens, and are marked so below.

## Sidebar

256 wide (288 beside a dashboard; 48 as a rail of icons; on a phone a sheet of 288 over a scrim). Its ground
is a shade off the page (`$sidebar`) with a `$border` line on the inner edge, no
shadow. From the top: a header with the workspace (48 high: a 32 tile, the name at weight 500 and one line of
12 under it), the groups, and a footer with the person signed in. Everything has 8 around it.

A row is 32 high with 8 inside, a 16 icon and 8 to its label, radius `$radius-md`. A group starts with a
label of 12 at weight 500 in `$muted-foreground` in a row of its own, 32 high; rows are 4 apart. The row is
a component with two states: `hover` and `active` (the page that is open: `$accent` and the label at weight
500). Exactly one row is active.

```json
{"type":"frame","id":"ui-nav-row","name":"Sidebar row","reusable":true,"width":240,"height":32,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-md","states":{"hover":{"fill":"$accent"},"active":{"fill":"$accent","descendants":{"ui-nav-row-label":{"fontWeight":"500"}}}},"children":[
  {"type":"icon","id":"ui-nav-row-icon","name":"Icon","library":"lucide","icon":"calendar-days","width":16,"height":16,"fill":"$foreground"},
  {"type":"text","id":"ui-nav-row-label","name":"Label","content":"Rota","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"}]}
```

```json
{"type":"frame","id":"ui-sidebar","name":"Sidebar","reusable":true,"width":256,"height":720,"layout":"vertical","fill":"$sidebar","stroke":"$border","strokeWidth":{"right":1},"children":[
  {"type":"frame","id":"ui-sidebar-header","name":"Header","width":"fill_container","padding":8,"children":[
    {"type":"frame","id":"ui-sidebar-workspace","name":"Workspace","width":"fill_container","height":48,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-md","children":[
      {"type":"frame","id":"ui-sidebar-logo","name":"Logo","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-lg","fill":"$primary","children":[
        {"type":"icon","id":"ui-sidebar-logo-icon","library":"lucide","icon":"ship","width":16,"height":16,"fill":"$primary-foreground"}]},
      {"type":"frame","id":"ui-sidebar-workspace-text","width":"fill_container","layout":"vertical","gap":2,"children":[
        {"type":"text","id":"ui-sidebar-workspace-name","name":"Name","content":"Harbour Rota","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"ui-sidebar-workspace-plan","name":"Plan","content":"Bergen office","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]},
      {"type":"icon","id":"ui-sidebar-workspace-icon","library":"lucide","icon":"chevrons-up-down","width":16,"height":16,"fill":"$muted-foreground"}]}]},
  {"type":"frame","id":"ui-sidebar-content","name":"Content","width":"fill_container","height":"fill_container","layout":"vertical","children":[
    {"type":"frame","id":"ui-sidebar-planning","name":"Group","width":"fill_container","layout":"vertical","padding":8,"children":[
      {"type":"frame","id":"ui-sidebar-planning-label","name":"Group label","width":"fill_container","height":32,"padding":[0,8],"alignItems":"center","children":[
        {"type":"text","id":"ui-sidebar-planning-label-text","content":"Planning","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-sidebar-planning-menu","name":"Menu","width":"fill_container","layout":"vertical","gap":4,"children":[
        {"type":"ref","id":"ui-sidebar-row-1","ref":"ui-nav-row","width":"fill_container","descendants":{"ui-nav-row-icon":{"icon":"calendar-days"},"ui-nav-row-label":{"content":"Rota"}},"state":"active"},
        {"type":"ref","id":"ui-sidebar-row-2","ref":"ui-nav-row","width":"fill_container","descendants":{"ui-nav-row-icon":{"icon":"clock"},"ui-nav-row-label":{"content":"Shifts"}}},
        {"type":"ref","id":"ui-sidebar-row-3","ref":"ui-nav-row","width":"fill_container","descendants":{"ui-nav-row-icon":{"icon":"users"},"ui-nav-row-label":{"content":"Teammates"}}},
        {"type":"ref","id":"ui-sidebar-row-4","ref":"ui-nav-row","width":"fill_container","descendants":{"ui-nav-row-icon":{"icon":"sun"},"ui-nav-row-label":{"content":"Time off"}}}]}]},
    {"type":"frame","id":"ui-sidebar-office","name":"Group","width":"fill_container","layout":"vertical","padding":8,"children":[
      {"type":"frame","id":"ui-sidebar-office-label","name":"Group label","width":"fill_container","height":32,"padding":[0,8],"alignItems":"center","children":[
        {"type":"text","id":"ui-sidebar-office-label-text","content":"Office","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-sidebar-office-menu","name":"Menu","width":"fill_container","layout":"vertical","gap":4,"children":[
        {"type":"ref","id":"ui-sidebar-row-5","ref":"ui-nav-row","width":"fill_container","descendants":{"ui-nav-row-icon":{"icon":"file-text"},"ui-nav-row-label":{"content":"Reports"}}},
        {"type":"frame","id":"ui-sidebar-sub","name":"Sub-menu","width":"fill_container","padding":[0,0,0,14],"children":[
          {"type":"frame","id":"ui-sidebar-sub-list","width":"fill_container","layout":"vertical","gap":4,"padding":[2,0,2,10],"stroke":"$border","strokeWidth":{"left":1},"children":[
            {"type":"frame","id":"ui-sidebar-sub-1","name":"Sub-row","width":"fill_container","height":28,"padding":[0,8],"alignItems":"center","cornerRadius":"$radius-md","children":[
              {"type":"text","id":"ui-sidebar-sub-1-label","content":"Hours by desk","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"}]},
            {"type":"frame","id":"ui-sidebar-sub-2","name":"Sub-row","width":"fill_container","height":28,"padding":[0,8],"alignItems":"center","cornerRadius":"$radius-md","children":[
              {"type":"text","id":"ui-sidebar-sub-2-label","content":"Overtime","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"}]}]}]},
        {"type":"ref","id":"ui-sidebar-row-6","ref":"ui-nav-row","width":"fill_container","descendants":{"ui-nav-row-icon":{"icon":"settings"},"ui-nav-row-label":{"content":"Settings"}}}]}]}]},
  {"type":"frame","id":"ui-sidebar-footer","name":"Footer","width":"fill_container","padding":8,"children":[
    {"type":"frame","id":"ui-sidebar-user","name":"User","width":"fill_container","height":48,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-md","children":[
      {"type":"ref","id":"ui-sidebar-user-avatar","ref":"ui-avatar"},
      {"type":"frame","id":"ui-sidebar-user-text","width":"fill_container","layout":"vertical","gap":2,"children":[
        {"type":"text","id":"ui-sidebar-user-name","name":"Name","content":"Maren Solheim","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"ui-sidebar-user-mail","name":"Email","content":"maren@harbour.example","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]},
      {"type":"icon","id":"ui-sidebar-user-icon","library":"lucide","icon":"ellipsis-vertical","width":16,"height":16,"fill":"$muted-foreground"}]}]}]}
```

- Change a row through `descendants` (`ui-nav-row-icon`, `ui-nav-row-label`) and pick the open page with
  `"state":"active"`. Every row has an icon, or none has.
- A sub-menu hangs on a 1 wide `$border` line, 14 in from the row above it, its rows 28 high without icons
  and 10 from the line. Show it open only under the row it belongs to; one level, never a tree of three.
- A count beside a row (unread, open) is 12 at weight 500 at the trailing edge, in a box 20 high, 4 from the
  row's edge. A row's own action is a 20 square ghost icon in the same place, shown on the hovered row only.
- The rail of icons: the same rows as 32 squares centred in 48, labels gone, the workspace as its tile alone.
  A tooltip names the row under the pointer.
- Inset: the page takes the sidebar's ground, and the content is a `$background` panel with `$radius-xl`, 8
  from the top, right and bottom edges, with the faintest shadow.

## Top bar

48 high (56 or 64 where the page has room to spare; one height in a product), a `$border` line under it, 24
beside. From the leading edge: the sidebar's toggle (`panel-left`), a separator 16 high, then the page's
title at 16 and weight 500, or a breadcrumb. Actions end at the trailing edge, 8 apart, the main one last.
Controls in the bar are 32 high.

```json
{"type":"frame","id":"ui-topbar","name":"Top bar","reusable":true,"width":960,"height":48,"padding":[0,24,0,20],"gap":8,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
  {"type":"frame","id":"ui-topbar-toggle","name":"Sidebar toggle","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"icon","id":"ui-topbar-toggle-icon","library":"lucide","icon":"panel-left","width":16,"height":16,"fill":"$foreground"}]},
  {"type":"rectangle","id":"ui-topbar-separator","width":1,"height":16,"fill":"$border"},
  {"type":"frame","id":"ui-topbar-title-box","width":"fill_container","padding":[0,0,0,8],"alignItems":"center","children":[
    {"type":"text","id":"ui-topbar-title","name":"Title","content":"Rota","fontFamily":"$font","fontSize":16,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"ui-topbar-actions","name":"Actions","gap":8,"alignItems":"center","children":[
    {"type":"frame","id":"ui-topbar-export","height":32,"padding":[0,10],"gap":6,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
      {"type":"text","id":"ui-topbar-export-label","content":"Export","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
    {"type":"ref","id":"ui-topbar-new","ref":"ui-button-icon-leading","height":32,"descendants":{"ui-button-icon-leading-label":{"content":"New shift"}}}]}]}
```

Replace the title with a breadcrumb when the page lies more than one level down. Search in the bar is an
input 32 high and 240 to 320 wide, before the actions.

## Navigation menu

The row of links across the top of a site. Each is 36 high with 16 beside the label, 14 at weight 500, no gap
between them; one that opens a panel has a `chevron-down` of 12, 4 after its label. Under the pointer:
`$muted`. Its panel is a popover (`references/overlays.md`) with radius `$radius-lg` and 8 inside, 4 below the
row, holding links of 14 with 8 around each.

```json
{"type":"frame","id":"ui-nav-menu","name":"Navigation menu","alignItems":"center","children":[
  {"type":"frame","id":"ui-nav-menu-1","name":"Trigger, open","height":36,"padding":[0,16],"gap":4,"alignItems":"center","cornerRadius":"$radius-md","fill":"$muted","children":[
    {"type":"text","id":"ui-nav-menu-1-label","content":"Product","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"icon","id":"ui-nav-menu-1-icon","library":"lucide","icon":"chevron-down","width":12,"height":12,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-nav-menu-2","name":"Trigger","height":36,"padding":[0,16],"gap":4,"alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"text","id":"ui-nav-menu-2-label","content":"Timetables","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"icon","id":"ui-nav-menu-2-icon","library":"lucide","icon":"chevron-down","width":12,"height":12,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-nav-menu-3","name":"Link","height":36,"padding":[0,16],"gap":4,"alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"text","id":"ui-nav-menu-3-label","content":"Prices","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"ui-nav-menu-4","name":"Link","height":36,"padding":[0,16],"gap":4,"alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"text","id":"ui-nav-menu-4-label","content":"Help","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]}]}
```

## Pagination

A centred row, 4 apart: "Previous" and "Next" with a chevron (8 on the chevron's side, 10 on the other), page
numbers as 36 squares (32 in Compact) without a skin, the current page as an outline, an `ellipsis` where
pages are left out. Show the first, the last, and the pages around the current one. On the first page
"Previous" has `"opacity":0.5`. (Looked at alone it is text of one size, and Quill's review says so:
that remark can stand for a control.)

```json
{"type":"frame","id":"ui-pagination","name":"Pagination","gap":4,"alignItems":"center","children":[
  {"type":"frame","id":"ui-pagination-previous","name":"Previous","height":36,"padding":[0,10,0,8],"gap":4,"alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"icon","id":"ui-pagination-previous-icon","library":"lucide","icon":"chevron-left","width":16,"height":16,"fill":"$foreground"},
    {"type":"text","id":"ui-pagination-previous-label","content":"Previous","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"ui-pagination-1","name":"Page","width":36,"height":36,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"text","id":"ui-pagination-1-label","content":"1","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"ui-pagination-2","name":"Page, current","width":36,"height":36,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
    {"type":"text","id":"ui-pagination-2-label","content":"2","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"ui-pagination-3","name":"Page","width":36,"height":36,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"text","id":"ui-pagination-3-label","content":"3","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"ui-pagination-more","name":"Ellipsis","width":36,"height":36,"justifyContent":"center","alignItems":"center","children":[
    {"type":"icon","id":"ui-pagination-more-icon","library":"lucide","icon":"ellipsis","width":16,"height":16,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-pagination-4","name":"Page","width":36,"height":36,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"text","id":"ui-pagination-4-label","content":"12","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"ui-pagination-next","name":"Next","height":36,"padding":[0,8,0,10],"gap":4,"alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"text","id":"ui-pagination-next-label","content":"Next","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"icon","id":"ui-pagination-next-icon","library":"lucide","icon":"chevron-right","width":16,"height":16,"fill":"$foreground"}]}]}
```

## Accordion

Rows with a `$border` line under each but the last. The trigger is 14 at weight 500 with 16 above and below
(10 in Compact) and a chevron in `$muted-foreground` at the trailing edge: `chevron-down` closed,
`chevron-up` open. The open row has its content of 14 under the trigger, 16 above the line.

```json
{"type":"frame","id":"ui-accordion","name":"Accordion","width":480,"layout":"vertical","children":[
  {"type":"frame","id":"ui-accordion-1","name":"Item, open","width":"fill_container","layout":"vertical","stroke":"$border","strokeWidth":{"bottom":1},"children":[
    {"type":"frame","id":"ui-accordion-1-trigger","name":"Trigger","width":"fill_container","padding":[16,0],"gap":16,"alignItems":"center","children":[
      {"type":"text","id":"ui-accordion-1-title","content":"Can two people share one shift?","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"icon","id":"ui-accordion-1-icon","library":"lucide","icon":"chevron-up","width":16,"height":16,"fill":"$muted-foreground"}]},
    {"type":"frame","id":"ui-accordion-1-content","name":"Content","width":"fill_container","padding":[0,32,16,0],"children":[
      {"type":"text","id":"ui-accordion-1-text","content":"Yes. Split the shift at any quarter hour and give each half to one teammate; the rota counts the hours apiece.","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.45}]}]},
  {"type":"frame","id":"ui-accordion-2","name":"Item","width":"fill_container","layout":"vertical","stroke":"$border","strokeWidth":{"bottom":1},"children":[
    {"type":"frame","id":"ui-accordion-2-trigger","name":"Trigger","width":"fill_container","padding":[16,0],"gap":16,"alignItems":"center","children":[
      {"type":"text","id":"ui-accordion-2-title","content":"What happens to a shift nobody takes?","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"icon","id":"ui-accordion-2-icon","library":"lucide","icon":"chevron-down","width":16,"height":16,"fill":"$muted-foreground"}]}]},
  {"type":"frame","id":"ui-accordion-3","name":"Item","width":"fill_container","layout":"vertical","children":[
    {"type":"frame","id":"ui-accordion-3-trigger","name":"Trigger","width":"fill_container","padding":[16,0],"gap":16,"alignItems":"center","children":[
      {"type":"text","id":"ui-accordion-3-title","content":"Who can see the hours I worked?","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"icon","id":"ui-accordion-3-icon","library":"lucide","icon":"chevron-down","width":16,"height":16,"fill":"$muted-foreground"}]}]}]}
```

## Stepper

Convention, built from this kit's tokens (not a component of the libraries above). Steps in a row: a 24
circle and a label of 14 at weight 500, 8 apart, joined by a 1 high line. Done: `$primary` with a `check`,
and the line after it in `$primary`. Current: `$primary` with its number. To come: `$muted` with the number
and the label in `$muted-foreground`. With more than five steps, or on a phone, write "Step 2 of 6" over a
progress bar instead.

```json
{"type":"frame","id":"ui-stepper","name":"Stepper","width":560,"gap":12,"alignItems":"center","children":[
  {"type":"frame","id":"ui-stepper-1","name":"Step, done","gap":8,"alignItems":"center","children":[
    {"type":"frame","id":"ui-stepper-1-dot","name":"Indicator","width":24,"height":24,"justifyContent":"center","alignItems":"center","cornerRadius":12,"fill":"$primary","children":[
      {"type":"icon","id":"ui-stepper-1-mark","library":"lucide","icon":"check","width":14,"height":14,"fill":"$primary-foreground"}]},
    {"type":"text","id":"ui-stepper-1-label","content":"Details","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"rectangle","id":"ui-stepper-line-1","width":"fill_container","height":1,"fill":"$primary"},
  {"type":"frame","id":"ui-stepper-2","name":"Step, current","gap":8,"alignItems":"center","children":[
    {"type":"frame","id":"ui-stepper-2-dot","name":"Indicator","width":24,"height":24,"justifyContent":"center","alignItems":"center","cornerRadius":12,"fill":"$primary","children":[
      {"type":"text","id":"ui-stepper-2-mark","content":"2","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$primary-foreground"}]},
    {"type":"text","id":"ui-stepper-2-label","content":"Schedule","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"rectangle","id":"ui-stepper-line-2","width":"fill_container","height":1,"fill":"$border"},
  {"type":"frame","id":"ui-stepper-3","name":"Step","gap":8,"alignItems":"center","children":[
    {"type":"frame","id":"ui-stepper-3-dot","name":"Indicator","width":24,"height":24,"justifyContent":"center","alignItems":"center","cornerRadius":12,"fill":"$muted","children":[
      {"type":"text","id":"ui-stepper-3-mark","content":"3","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]},
    {"type":"text","id":"ui-stepper-3-label","content":"Review","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$muted-foreground"}]}]}
```

## Page header

Convention, built from this kit's tokens. Under the top bar, 24 above the content: the title at 24 and weight
500, one line of 14 in `$muted-foreground` 4 under it, and the page's actions on the same row, ending at the
trailing edge and level with the last line of text.

```json
{"type":"frame","id":"ui-page-header","name":"Page header","width":960,"gap":24,"alignItems":"end","children":[
  {"type":"frame","id":"ui-page-header-text","width":"fill_container","layout":"vertical","gap":4,"children":[
    {"type":"text","id":"ui-page-header-title","name":"Title","content":"Week 48","fontFamily":"$font","fontSize":24,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"ui-page-header-description","name":"Description","content":"23 to 29 November. Two shifts still need someone.","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"frame","id":"ui-page-header-actions","name":"Actions","gap":8,"alignItems":"center","children":[
    {"type":"frame","id":"ui-page-header-copy","height":36,"padding":[0,10],"gap":6,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
      {"type":"text","id":"ui-page-header-copy-label","content":"Copy last week","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
    {"type":"ref","id":"ui-page-header-publish","ref":"ui-button","descendants":{"ui-button-label":{"content":"Publish rota"}}}]}]}
```
