# A table that is worked in, and the stat tile

What the plain table of `references/data.md` lacks once people sort, filter, select and page through it, and
the tile that shows one figure. They use the variables of `references/tokens.md`, and the search input,
select and badge of `references/controls.md`: insert those first.

The table's bars and the tile are measured at shadcn/ui's 2026 dashboard block. The sign of a sorted column
and the bulk action bar were not read there: they are conventions, built from this kit's tokens.

## Data table

Three parts, 16 apart: a toolbar, the table in a box, a footer.

- Toolbar: controls 32 high, 8 apart. What narrows the rows starts at the leading edge (a search input of
  240 to 320, then filters as outline buttons with `list-filter`); what changes the view, and the page's
  action, end at the trailing edge.
- Table: in a `$card` box with a 1 wide `$border` outline and radius `$radius-lg`, clipped. The head row is 40 high
  on `$muted`, labels at weight 500. Rows 44 high, a `$border` line under each but the last.
- Selecting: a column of 16 checkboxes leads, and the head row's box selects all (checked when every row
  is, a `minus` in it when some are). A selected row has `$muted-half`.
- Sorting: the sorted column's label is followed by `arrow-up` or `arrow-down` of 14, 6 after it; a column
  that can be sorted but is not shows nothing until the pointer is on it. One column is sorted at a time,
  and the rows are in that order.
- A row's own actions are a ghost `ellipsis` square of 32 at the trailing edge.
- Footer: at the leading edge, in `$muted-foreground`, how many rows are selected of how many there are; at
  the trailing edge, 32 apart: "Rows per page" with a select 72 wide, "Page 1 of 7", and four outline
  squares of 32 for first, previous, next and last, 8 apart. The ones that lead nowhere have `"opacity":0.5`.
- The numbers agree: rows on the page, rows per page, pages and the total all fit one another.

```json
{"type":"frame","id":"ui-data-table","name":"Data table","width":880,"layout":"vertical","gap":16,"children":[
  {"type":"frame","id":"ui-data-table-toolbar","name":"Toolbar","width":"fill_container","justifyContent":"space_between","alignItems":"center","children":[
    {"type":"frame","id":"ui-data-table-filters","name":"Filters","gap":8,"alignItems":"center","children":[
      {"type":"ref","id":"ui-data-table-search","ref":"ui-search-input","width":240,"height":32,"descendants":{"ui-search-input-value":{"content":"Filter shifts"}}},
      {"type":"frame","id":"ui-data-table-status","height":32,"padding":[0,10,0,8],"gap":6,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
        {"type":"icon","id":"ui-data-table-status-icon","library":"lucide","icon":"list-filter","width":16,"height":16,"fill":"$foreground"},
        {"type":"text","id":"ui-data-table-status-label","content":"Status","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]}]},
    {"type":"frame","id":"ui-data-table-columns","height":32,"padding":[0,10,0,8],"gap":6,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
      {"type":"icon","id":"ui-data-table-columns-icon","library":"lucide","icon":"settings-2","width":16,"height":16,"fill":"$foreground"},
      {"type":"text","id":"ui-data-table-columns-label","content":"Columns","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]}]},
  {"type":"frame","id":"ui-data-table-box","name":"Table","width":"fill_container","layout":"vertical","cornerRadius":"$radius-lg","fill":"$card","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","clip":true,"children":[
    {"type":"frame","id":"ui-data-table-head","name":"Head row","width":"fill_container","height":40,"padding":[0,8,0,12],"gap":16,"alignItems":"center","fill":"$muted","stroke":"$border","strokeWidth":{"bottom":1},"children":[
      {"type":"frame","id":"ui-data-table-head-check","width":24,"height":16,"alignItems":"center","children":[
        {"type":"frame","id":"ui-data-table-head-box","name":"Checkbox, some","width":16,"height":16,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-xs","fill":"$primary","children":[
          {"type":"icon","id":"ui-data-table-head-box-mark","library":"lucide","icon":"minus","width":12,"height":12,"fill":"$primary-foreground"}]}]},
      {"type":"frame","id":"ui-data-table-head-shift","name":"Sorted column","width":"fill_container","gap":6,"alignItems":"center","children":[
        {"type":"text","id":"ui-data-table-head-shift-label","content":"Shift","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
        {"type":"icon","id":"ui-data-table-head-shift-icon","library":"lucide","icon":"arrow-up","width":14,"height":14,"fill":"$muted-foreground"}]},
      {"type":"text","id":"ui-data-table-head-mate","content":"Teammate","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":176},
      {"type":"text","id":"ui-data-table-head-status","content":"Status","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":104},
      {"type":"text","id":"ui-data-table-head-hours","content":"Hours","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":56,"textAlign":"right"},
      {"type":"frame","id":"ui-data-table-head-action","width":32,"height":32,"children":[]}]},
    {"type":"frame","id":"ui-data-table-row-1","name":"Row","width":"fill_container","height":44,"padding":[0,8,0,12],"gap":16,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
      {"type":"frame","id":"ui-data-table-row-1-check","width":24,"height":16,"alignItems":"center","children":[
        {"type":"frame","id":"ui-data-table-row-1-box","name":"Checkbox","width":16,"height":16,"cornerRadius":"$radius-xs","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[]}]},
      {"type":"text","id":"ui-data-table-row-1-shift","content":"Ferry desk, Saturday 08:00","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"ui-data-table-row-1-mate","content":"Aksel Bjørnstad-Lie","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":176},
      {"type":"frame","id":"ui-data-table-row-1-status","width":104,"alignItems":"center","children":[
        {"type":"ref","id":"ui-data-table-row-1-badge","ref":"ui-badge","descendants":{"ui-badge-label":{"content":"Covered"}},"fill":"#00000000","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner"}]},
      {"type":"text","id":"ui-data-table-row-1-hours","content":"6.0","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":56,"textAlign":"right"},
      {"type":"frame","id":"ui-data-table-row-1-action","name":"Row actions","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"icon","id":"ui-data-table-row-1-action-icon","library":"lucide","icon":"ellipsis","width":16,"height":16,"fill":"$muted-foreground"}]}]},
    {"type":"frame","id":"ui-data-table-row-2","name":"Row, selected","width":"fill_container","height":44,"padding":[0,8,0,12],"gap":16,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"fill":"$muted-half","children":[
      {"type":"frame","id":"ui-data-table-row-2-check","width":24,"height":16,"alignItems":"center","children":[
        {"type":"frame","id":"ui-data-table-row-2-box","name":"Checkbox, checked","width":16,"height":16,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-xs","fill":"$primary","children":[
          {"type":"icon","id":"ui-data-table-row-2-box-mark","library":"lucide","icon":"check","width":12,"height":12,"fill":"$primary-foreground"}]}]},
      {"type":"text","id":"ui-data-table-row-2-shift","content":"Harbour office, Saturday 14:00","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"ui-data-table-row-2-mate","content":"Maren Solheim","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":176},
      {"type":"frame","id":"ui-data-table-row-2-status","width":104,"alignItems":"center","children":[
        {"type":"ref","id":"ui-data-table-row-2-badge","ref":"ui-badge","descendants":{"ui-badge-label":{"content":"Covered"}},"fill":"#00000000","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner"}]},
      {"type":"text","id":"ui-data-table-row-2-hours","content":"6.0","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":56,"textAlign":"right"},
      {"type":"frame","id":"ui-data-table-row-2-action","name":"Row actions","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"icon","id":"ui-data-table-row-2-action-icon","library":"lucide","icon":"ellipsis","width":16,"height":16,"fill":"$muted-foreground"}]}]},
    {"type":"frame","id":"ui-data-table-row-3","name":"Row, selected","width":"fill_container","height":44,"padding":[0,8,0,12],"gap":16,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"fill":"$muted-half","children":[
      {"type":"frame","id":"ui-data-table-row-3-check","width":24,"height":16,"alignItems":"center","children":[
        {"type":"frame","id":"ui-data-table-row-3-box","name":"Checkbox, checked","width":16,"height":16,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-xs","fill":"$primary","children":[
          {"type":"icon","id":"ui-data-table-row-3-box-mark","library":"lucide","icon":"check","width":12,"height":12,"fill":"$primary-foreground"}]}]},
      {"type":"text","id":"ui-data-table-row-3-shift","content":"Ticket desk, Sunday 09:30","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"ui-data-table-row-3-mate","content":"Nobody yet","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":176},
      {"type":"frame","id":"ui-data-table-row-3-status","width":104,"alignItems":"center","children":[
        {"type":"ref","id":"ui-data-table-row-3-badge","ref":"ui-badge","descendants":{"ui-badge-label":{"content":"Open","fill":"$primary-foreground"}},"fill":"$primary"}]},
      {"type":"text","id":"ui-data-table-row-3-hours","content":"7.5","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":56,"textAlign":"right"},
      {"type":"frame","id":"ui-data-table-row-3-action","name":"Row actions","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"icon","id":"ui-data-table-row-3-action-icon","library":"lucide","icon":"ellipsis","width":16,"height":16,"fill":"$muted-foreground"}]}]},
    {"type":"frame","id":"ui-data-table-row-4","name":"Row","width":"fill_container","height":44,"padding":[0,8,0,12],"gap":16,"alignItems":"center","children":[
      {"type":"frame","id":"ui-data-table-row-4-check","width":24,"height":16,"alignItems":"center","children":[
        {"type":"frame","id":"ui-data-table-row-4-box","name":"Checkbox","width":16,"height":16,"cornerRadius":"$radius-xs","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[]}]},
      {"type":"text","id":"ui-data-table-row-4-shift","content":"Night watch, Sunday 22:00","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"ui-data-table-row-4-mate","content":"Ingrid Vang","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":176},
      {"type":"frame","id":"ui-data-table-row-4-status","width":104,"alignItems":"center","children":[
        {"type":"ref","id":"ui-data-table-row-4-badge","ref":"ui-badge","descendants":{"ui-badge-label":{"content":"Swap asked"}}}]},
      {"type":"text","id":"ui-data-table-row-4-hours","content":"10.0","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":56,"textAlign":"right"},
      {"type":"frame","id":"ui-data-table-row-4-action","name":"Row actions","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"icon","id":"ui-data-table-row-4-action-icon","library":"lucide","icon":"ellipsis","width":16,"height":16,"fill":"$muted-foreground"}]}]}]},
  {"type":"frame","id":"ui-data-table-footer","name":"Footer","width":"fill_container","gap":16,"alignItems":"center","children":[
    {"type":"text","id":"ui-data-table-count","name":"Selection","content":"2 of 68 shifts selected.","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"frame","id":"ui-data-table-paging","name":"Paging","gap":32,"alignItems":"center","children":[
      {"type":"frame","id":"ui-data-table-size","name":"Rows per page","gap":8,"alignItems":"center","children":[
        {"type":"text","id":"ui-data-table-size-label","content":"Rows per page","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
        {"type":"ref","id":"ui-data-table-size-select","ref":"ui-select","width":72,"height":32,"descendants":{"ui-select-value":{"content":"10"}}}]},
      {"type":"text","id":"ui-data-table-page","name":"Page","content":"Page 1 of 7","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
      {"type":"frame","id":"ui-data-table-buttons","name":"Page buttons","gap":8,"alignItems":"center","children":[
        {"type":"frame","id":"ui-data-table-first","name":"First page","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","opacity":0.5,"children":[
          {"type":"icon","id":"ui-data-table-first-icon","library":"lucide","icon":"chevrons-left","width":16,"height":16,"fill":"$foreground"}]},
        {"type":"frame","id":"ui-data-table-previous","name":"Previous page","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","opacity":0.5,"children":[
          {"type":"icon","id":"ui-data-table-previous-icon","library":"lucide","icon":"chevron-left","width":16,"height":16,"fill":"$foreground"}]},
        {"type":"frame","id":"ui-data-table-next","name":"Next page","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
          {"type":"icon","id":"ui-data-table-next-icon","library":"lucide","icon":"chevron-right","width":16,"height":16,"fill":"$foreground"}]},
        {"type":"frame","id":"ui-data-table-last","name":"Last page","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
          {"type":"icon","id":"ui-data-table-last-icon","library":"lucide","icon":"chevrons-right","width":16,"height":16,"fill":"$foreground"}]}]}]}]}]}
```

Copy the row for every record, with the same parts at the same widths. Figures end on one edge with the same
decimals; a state is a badge; an empty cell says what is missing ("Nobody yet") in `$muted-foreground`, not a
dash. With no rows the box shows the head row and the empty state of `references/data.md` under it.

## Bulk action bar

Convention, built from this kit's tokens. It appears while rows are selected: a floating bar (the `$popover`
fill, the hairline and the floating shadow) 48 high with radius `$radius-lg`, centred, 24 above the bottom
of the screen, over the table. The count, a separator, the actions as buttons without a skin, the one that
destroys in `$destructive` and last, and an `x` that clears the selection.

```json
{"type":"frame","id":"ui-bulk-bar","name":"Bulk action bar","height":48,"padding":[0,8,0,16],"gap":8,"alignItems":"center","cornerRadius":"$radius-lg","fill":"$popover","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"color":"#0000000f"},{"type":"shadow","offset":{"x":0,"y":6},"blur":14,"color":"#00000014"}],"stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"text","id":"ui-bulk-count","name":"Count","content":"2 selected","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
  {"type":"rectangle","id":"ui-bulk-separator","width":1,"height":16,"fill":"$border"},
  {"type":"frame","id":"ui-bulk-actions","name":"Actions","alignItems":"center","children":[
    {"type":"frame","id":"ui-bulk-assign","name":"Action","height":32,"padding":[0,10],"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
      {"type":"text","id":"ui-bulk-assign-label","content":"Assign","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
    {"type":"frame","id":"ui-bulk-export","name":"Action","height":32,"padding":[0,10],"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
      {"type":"text","id":"ui-bulk-export-label","content":"Export","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
    {"type":"frame","id":"ui-bulk-remove","name":"Action","height":32,"padding":[0,10],"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
      {"type":"text","id":"ui-bulk-remove-label","content":"Remove","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$destructive"}]}]},
  {"type":"frame","id":"ui-bulk-close","name":"Clear selection","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"icon","id":"ui-bulk-close-icon","library":"lucide","icon":"x","width":16,"height":16,"fill":"$muted-foreground"}]}]}
```

## Stat tile

One figure in a card: a label of 14 in `$muted-foreground`, the figure under it at 24 and weight 600, and
level with the label at the trailing edge a badge with how it changed (`trending-up` or `trending-down` of
12 and the change with its sign). Under them, 24 below, one line of 14 at weight 500 that says what the
change means and one in `$muted-foreground` that says what was counted.

```json
{"type":"frame","id":"ui-stat","name":"Stat tile","reusable":true,"width":272,"layout":"vertical","gap":24,"padding":24,"cornerRadius":"$radius-xl","fill":"$card","effect":{"type":"shadow","offset":{"x":0,"y":1},"blur":2,"color":"#0000000d"},"stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"ui-stat-header","name":"Header","width":"fill_container","gap":8,"children":[
    {"type":"frame","id":"ui-stat-text","width":"fill_container","layout":"vertical","gap":6,"children":[
      {"type":"text","id":"ui-stat-label","name":"Label","content":"Hours rostered","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"ui-stat-value","name":"Value","content":"1,284.5","fontFamily":"$font","fontSize":24,"fontWeight":"600","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"}]},
    {"type":"frame","id":"ui-stat-delta","name":"Change","height":20,"padding":[0,8],"gap":4,"alignItems":"center","cornerRadius":10,"stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
      {"type":"icon","id":"ui-stat-delta-icon","name":"Icon","library":"lucide","icon":"trending-up","width":12,"height":12,"fill":"$foreground"},
      {"type":"text","id":"ui-stat-delta-label","name":"Label","content":"+12.5%","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground"}]}]},
  {"type":"frame","id":"ui-stat-footer","name":"Footer","width":"fill_container","layout":"vertical","gap":4,"children":[
    {"type":"text","id":"ui-stat-note","name":"Note","content":"More than last week","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"ui-stat-detail","name":"Detail","content":"Week 47, all four desks","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]}]}
```

- Four tiles across the top of a dashboard, 16 apart, all one width and height. On a narrow screen two by
  two. Never as the whole page: tiles lead into the chart or table they sum up.
- A figure is written in full with its separators ("1,284.5"), or shortened the same way in every tile
  ("1.3k"). The change carries its sign. Whether up is good depends on what is counted: say it in the note,
  and keep red and green for where a rise or fall is truly bad or good.
- The figures must agree with the table below them: a total is the sum of what the rows show.
