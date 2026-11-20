# Cards, tables and the smaller parts

Insert each block with `edit_layers` as the other reference files say. They use the variables of
`references/tokens.md`; buttons and badges inside them are instances of `ui-button` and `ui-badge` from
`references/controls.md`: insert those first.

## Card

A card is a header, content and an optional footer, 24 apart, with 24 around (Compact 16). The title is 16
at weight 500, the description 14 in `$muted-foreground`, 4 under it. A card has the hairline and, at most,
the faintest shadow. No card inside a card: group inside one with space or a `$border` line.

```json
{"type":"frame","id":"ui-card","name":"Card","reusable":true,"width":384,"layout":"vertical","gap":24,"padding":24,"cornerRadius":"$radius-xl","fill":"$card","stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","effect":{"type":"shadow","offset":{"x":0,"y":1},"blur":2,"color":"#0000000d"},"children":[
  {"type":"frame","id":"ui-card-header","name":"Header","width":"fill_container","layout":"vertical","gap":4,"children":[
    {"type":"text","id":"ui-card-title","name":"Title","content":"Weekend cover","fontFamily":"$font","fontSize":16,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-card-description","name":"Description","content":"Two shifts on Saturday still need someone.","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.45,"fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-card-content","name":"Content","width":"fill_container","layout":"vertical","gap":12,"children":[
    {"type":"text","id":"ui-card-content-text","content":"Saturday 08:00 to 14:00, harbour office. Saturday 14:00 to 20:00, ferry desk.","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.45,"fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-card-footer","name":"Footer","width":"fill_container","gap":8,"justifyContent":"end","children":[
    {"type":"ref","id":"ui-card-action","ref":"ui-button","descendants":{"ui-button-label":{"content":"Assign shifts"}}}]}]}
```

- Fill the content through `descendants` with `children` on `ui-card-content`.
- An action that belongs to the whole card (a menu, "See all") sits in the header at the trailing edge, level
  with the title.
- Compact: padding and gap 16, no shadow, and the footer may be a band: `$muted-half`, a `$border` line above
  it, edge to edge.
- Cards side by side share one height and one padding; 16 or 24 apart.

A statistic in a card: the label 14 in `$muted-foreground`, the figure under it 24 at weight 600, and at most
one line of 14 under that. Do not line up three of these as the whole page.

## Table

Text 14. The head row is 40 high with labels at weight 500 in `$foreground` (not grey, not capitals); body
rows 40 to 48 high with 8 beside each cell; a `$border` line under every row except the last. Text columns
start on the left; figures, amounts and dates end on the right at one fixed width with the same decimals.
The row under the pointer is `$muted-half`; a chosen row `$muted`.

```json
{"type":"frame","id":"ui-table","name":"Table","width":640,"layout":"vertical","children":[
  {"type":"frame","id":"ui-table-head","name":"Head row","width":"fill_container","height":40,"padding":[0,8],"gap":16,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
    {"type":"text","id":"ui-table-head-1","content":"Teammate","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-table-head-2","content":"Role","textGrowth":"fixed-width","width":120,"fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-table-head-3","content":"Hours","textGrowth":"fixed-width","width":72,"textAlign":"right","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"ui-table-row-1","name":"Row","width":"fill_container","height":44,"padding":[0,8],"gap":16,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
    {"type":"text","id":"ui-table-row-1-1","content":"Maren Solheim","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-table-row-1-2","content":"Scheduler","textGrowth":"fixed-width","width":120,"fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"},
    {"type":"text","id":"ui-table-row-1-3","content":"37.5","textGrowth":"fixed-width","width":72,"textAlign":"right","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-table-row-2","name":"Row, hovered","width":"fill_container","height":44,"padding":[0,8],"gap":16,"alignItems":"center","fill":"$muted-half","stroke":"$border","strokeWidth":{"bottom":1},"children":[
    {"type":"text","id":"ui-table-row-2-1","content":"Aksel Bjørnstad-Lie","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-table-row-2-2","content":"Deckhand","textGrowth":"fixed-width","width":120,"fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"},
    {"type":"text","id":"ui-table-row-2-3","content":"112.0","textGrowth":"fixed-width","width":72,"textAlign":"right","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-table-row-3","name":"Row","width":"fill_container","height":44,"padding":[0,8],"gap":16,"alignItems":"center","children":[
    {"type":"text","id":"ui-table-row-3-1","content":"Ingrid Vang","textGrowth":"fixed-width","width":"fill_container","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-table-row-3-2","content":"Ticket desk","textGrowth":"fixed-width","width":120,"fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"},
    {"type":"text","id":"ui-table-row-3-3","content":"9.0","textGrowth":"fixed-width","width":72,"textAlign":"right","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}]}
```

A table is not a component: copy the row for every record, and give every row the same parts at the same
widths. A state in a row is a badge; an action is a ghost icon button (`ellipsis`) at the trailing edge. Show
real records with one worth seeing (a long name, a large figure). An empty table shows the head row and the
empty state below in place of rows.

## List row

For a list on a phone, and for any list that is not a table: rows 64 high with two lines (56 with one), 16
beside, a `$border` line under every row except the last. The first line is 16 at weight 500 on a phone (14
on a desktop), the second 14 in `$muted-foreground`; what sits at the trailing edge (a time, an amount, a
badge, a `chevron-right`) has one fixed width in every row. The chosen row is `$muted`.

```json
{"type":"frame","id":"ui-list-row","name":"List row","reusable":true,"width":390,"height":64,"padding":[0,16],"gap":12,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
  {"type":"frame","id":"ui-list-row-text","name":"Text","width":"fill_container","layout":"vertical","gap":2,"children":[
    {"type":"text","id":"ui-list-row-title","name":"Title","content":"Ferry desk","fontFamily":"$font","fontSize":16,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-list-row-description","name":"Description","content":"Aksel Bjørnstad-Lie","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
  {"type":"text","id":"ui-list-row-value","name":"Value","content":"10:00 to 18:00","textGrowth":"fixed-width","width":112,"textAlign":"right","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}
```

On the last row of a list override `"strokeWidth":{"bottom":0}`. A list runs on under whatever covers it (a
drawer, a toast): do not shorten it to fit.

## Avatar

A circle of 32 (24 small, 40 large). Without a picture: `$muted` with one or two initials at weight 500.

```json
{"type":"frame","id":"ui-avatar","name":"Avatar","reusable":true,"width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":16,"fill":"$muted","children":[
  {"type":"text","id":"ui-avatar-initials","name":"Fallback","content":"MS","fontFamily":"$font","fontSize":13,"fontWeight":"500","fill":"$foreground"}]}
```

With a picture the frame gets an image fill and `"enabled":false` on the initials. In a group each avatar
overlaps the one before by 8 and has a 2 wide `$background` stroke on the outside; after four comes a count
("+3") in the same circle. Look at initials at `scale: 2` and nudge them until they sit centred.

## Alert

A message that belongs to a place on the page and stays there. Radius `$radius-lg`, a `$border` outline,
12 above and below, 16 beside; a 16 icon, then a title at weight 500 and a description in
`$muted-foreground` 2 under it.

```json
{"type":"frame","id":"ui-alert","name":"Alert","reusable":true,"width":480,"padding":[12,16],"gap":10,"cornerRadius":"$radius-lg","fill":"$card","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"ui-alert-icon-box","height":20,"alignItems":"center","children":[
    {"type":"icon","id":"ui-alert-icon","name":"Icon","library":"lucide","icon":"info","width":16,"height":16,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-alert-content","name":"Content","width":"fill_container","layout":"vertical","gap":2,"children":[
    {"type":"text","id":"ui-alert-title","name":"Title","content":"The winter timetable starts on 4 November","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-alert-description","name":"Description","content":"Shifts after that date follow the new sailing times.","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.45,"fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]}]}
```

Destructive: the icon (`triangle-exclamation-point`), the title and the description in `$destructive`, the
surface unchanged. No red fill.

## Empty state

What a list, a table or a page shows when there is nothing yet: centred, an icon tile of 40 in `$muted`, a
title of 18 at weight 500, one sentence in `$muted-foreground` at most 384 wide, and the action that fills
it. It says what will be here and how to start, not "No data".

```json
{"type":"frame","id":"ui-empty","name":"Empty state","width":480,"layout":"vertical","gap":16,"padding":48,"alignItems":"center","children":[
  {"type":"frame","id":"ui-empty-media","name":"Media","width":40,"height":40,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-lg","fill":"$muted","children":[
    {"type":"icon","id":"ui-empty-icon","library":"lucide","icon":"calendar-plus","width":24,"height":24,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-empty-header","name":"Header","width":"fill_container","layout":"vertical","gap":8,"alignItems":"center","children":[
    {"type":"text","id":"ui-empty-title","name":"Title","content":"No shifts this week yet","fontFamily":"$font","fontSize":18,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-empty-description","name":"Description","content":"Add the first shift and the rota fills in from there.","textAlign":"center","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
  {"type":"ref","id":"ui-empty-action","ref":"ui-button","descendants":{"ui-button-label":{"content":"Add a shift"}}}]}
```

The other states of a view, which a design of a working screen should name and, when asked, show as frames:
loading (skeletons in the shapes of the content, below), an error (an alert with what went wrong and a
"Try again" button), and no results for a search or filter (one line naming the query and a "Clear filters"
ghost button).

## Skeleton, progress, separator

A skeleton stands in for content that is loading: `$muted` rectangles with `$radius-md`, in the sizes of what
will come (a 16 high bar where a line of 14 text will be, a 32 circle for an avatar). Never a spinner over an
empty page when the layout is known.

```json
{"type":"frame","id":"ui-skeleton-row","name":"Skeleton row","width":320,"gap":12,"alignItems":"center","children":[
  {"type":"ellipse","id":"ui-skeleton-avatar","width":32,"height":32,"fill":"$muted"},
  {"type":"frame","id":"ui-skeleton-lines","width":"fill_container","layout":"vertical","gap":8,"children":[
    {"type":"rectangle","id":"ui-skeleton-line-1","width":160,"height":14,"cornerRadius":"$radius-xs","fill":"$muted"},
    {"type":"rectangle","id":"ui-skeleton-line-2","width":240,"height":14,"cornerRadius":"$radius-xs","fill":"$muted"}]}]}
```

A progress bar is a 6 high `$muted` track with a `$primary` indicator, both with a radius of 3; a label of 14
at weight 500 above its start and the value above its end:

```json
{"type":"frame","id":"ui-progress","name":"Progress","reusable":true,"width":320,"height":6,"cornerRadius":3,"fill":"$muted","children":[
  {"type":"rectangle","id":"ui-progress-indicator","name":"Indicator","width":208,"height":6,"cornerRadius":3,"fill":"$primary"}]}
```

A spinner is the `loader-circle` icon at 16 in `$muted-foreground`. A separator is a rectangle 1 high (or
wide) in `$border`.

## Kbd and breadcrumb

A key: 20 high, at least 20 wide (give a single letter `"width":20`), 4 beside the letter, `$radius-xs`, `$muted`, 12 at weight 500. Several keys
are 4 apart.

```json
{"type":"frame","id":"ui-kbd","name":"Kbd","reusable":true,"height":20,"padding":[0,4],"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-xs","fill":"$muted","children":[
  {"type":"text","id":"ui-kbd-label","content":"Ctrl","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground"}]}
```

A breadcrumb is one row of 14 text, 6 apart: the places before in `$muted-foreground`, a `chevron-right` of
14 between them, the current page last in `$foreground`.

```json
{"type":"frame","id":"ui-breadcrumb","name":"Breadcrumb","gap":6,"alignItems":"center","children":[
  {"type":"text","id":"ui-breadcrumb-1","content":"Harbour Rota","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"},
  {"type":"icon","id":"ui-breadcrumb-sep-1","library":"lucide","icon":"chevron-right","width":14,"height":14,"fill":"$muted-foreground"},
  {"type":"text","id":"ui-breadcrumb-2","content":"Rotas","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"},
  {"type":"icon","id":"ui-breadcrumb-sep-2","library":"lucide","icon":"chevron-right","width":14,"height":14,"fill":"$muted-foreground"},
  {"type":"text","id":"ui-breadcrumb-3","content":"Week 45","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}
```

Both of these are built in `references/navigation.md`. Pagination is a centred row of ghost icon buttons 36 square, 4 apart, the current page as an outline button,
"Previous" and "Next" with a chevron at the ends. An accordion is a list of rows with a `$border` line under
each but the last: a trigger of 14 at weight 500 with 16 above and below and a `chevron-down` at the trailing
edge (`chevron-up` when open), the open one with its content of 14 under it.
