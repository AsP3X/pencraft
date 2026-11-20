# Dates and the larger inputs

A calendar and the date field that opens it, a combobox, a field of chips, a one-time code, an input with a
prefix and suffix, a number input, a toggle group and a place to drop files. They use the variables of
`references/tokens.md` and stand beside the inputs of `references/controls.md`, with the same height in a
row (36; 32 in Compact; 44 on a phone) and the same skin: `$control` with a 1 wide `$input` border.

Measured at shadcn/ui's 2026 components. The number input and the file drop are not components of that
library: they are conventions, built from this kit's tokens, and are marked so below.

## Calendar

Days are squares of 32 (28 in Compact) with radius `$radius-md`, seven across with no gap, weeks 8 apart, 12
around the whole (8 in Compact). The caption is one row as high as a day: the month and year at weight 500
in the middle, a `chevron-left` and a `chevron-right` button at its ends. Weekday names are 12 in
`$muted-foreground`. Today has a `$muted` fill; the chosen day `$primary` with `$primary-foreground`; days
of the neighbouring months are `$muted-foreground`; a day that cannot be chosen has `"opacity":0.5`.

```json
{"type":"frame","id":"ui-calendar","name":"Calendar","reusable":true,"layout":"vertical","gap":16,"padding":12,"cornerRadius":"$radius-lg","fill":"$popover","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"color":"#0000000f"},{"type":"shadow","offset":{"x":0,"y":6},"blur":14,"color":"#00000014"}],"stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"ui-calendar-caption","name":"Caption","width":"fill_container","height":32,"justifyContent":"space_between","alignItems":"center","children":[
    {"type":"frame","id":"ui-calendar-previous","name":"Previous","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
      {"type":"icon","id":"ui-calendar-previous-icon","library":"lucide","icon":"chevron-left","width":16,"height":16,"fill":"$foreground"}]},
    {"type":"text","id":"ui-calendar-month","name":"Month","content":"November 2026","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"frame","id":"ui-calendar-next","name":"Next","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
      {"type":"icon","id":"ui-calendar-next-icon","library":"lucide","icon":"chevron-right","width":16,"height":16,"fill":"$foreground"}]}]},
  {"type":"frame","id":"ui-calendar-grid","name":"Grid","layout":"vertical","gap":8,"children":[
    {"type":"frame","id":"ui-calendar-weekdays","name":"Weekdays","children":[
      {"type":"frame","id":"ui-calendar-weekday-0","width":32,"height":20,"justifyContent":"center","alignItems":"center","children":[
        {"type":"text","id":"ui-calendar-weekday-0-label","content":"Mo","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-weekday-1","width":32,"height":20,"justifyContent":"center","alignItems":"center","children":[
        {"type":"text","id":"ui-calendar-weekday-1-label","content":"Tu","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-weekday-2","width":32,"height":20,"justifyContent":"center","alignItems":"center","children":[
        {"type":"text","id":"ui-calendar-weekday-2-label","content":"We","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-weekday-3","width":32,"height":20,"justifyContent":"center","alignItems":"center","children":[
        {"type":"text","id":"ui-calendar-weekday-3-label","content":"Th","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-weekday-4","width":32,"height":20,"justifyContent":"center","alignItems":"center","children":[
        {"type":"text","id":"ui-calendar-weekday-4-label","content":"Fr","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-weekday-5","width":32,"height":20,"justifyContent":"center","alignItems":"center","children":[
        {"type":"text","id":"ui-calendar-weekday-5-label","content":"Sa","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-weekday-6","width":32,"height":20,"justifyContent":"center","alignItems":"center","children":[
        {"type":"text","id":"ui-calendar-weekday-6-label","content":"Su","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]}]},
    {"type":"frame","id":"ui-calendar-week-0","name":"Week","children":[
      {"type":"frame","id":"ui-calendar-day-10-26","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-10-26-label","content":"26","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-10-27","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-10-27-label","content":"27","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-10-28","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-10-28-label","content":"28","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-10-29","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-10-29-label","content":"29","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-10-30","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-10-30-label","content":"30","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-10-31","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-10-31-label","content":"31","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-1","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-1-label","content":"1","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}]},
    {"type":"frame","id":"ui-calendar-week-1","name":"Week","children":[
      {"type":"frame","id":"ui-calendar-day-11-2","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-2-label","content":"2","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-3","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-3-label","content":"3","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-4","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-4-label","content":"4","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-5","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-5-label","content":"5","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-6","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-6-label","content":"6","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-7","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-7-label","content":"7","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-8","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-8-label","content":"8","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}]},
    {"type":"frame","id":"ui-calendar-week-2","name":"Week","children":[
      {"type":"frame","id":"ui-calendar-day-11-9","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-9-label","content":"9","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-10","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-10-label","content":"10","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-11","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-11-label","content":"11","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-12","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-12-label","content":"12","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-13","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-13-label","content":"13","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-14","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-14-label","content":"14","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-15","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-15-label","content":"15","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}]},
    {"type":"frame","id":"ui-calendar-week-3","name":"Week","children":[
      {"type":"frame","id":"ui-calendar-day-11-16","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-16-label","content":"16","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-17","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-17-label","content":"17","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-18","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-18-label","content":"18","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-19","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-19-label","content":"19","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-20","name":"Day, today","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$muted","children":[
        {"type":"text","id":"ui-calendar-day-11-20-label","content":"20","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-21","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-21-label","content":"21","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-22","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-22-label","content":"22","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}]},
    {"type":"frame","id":"ui-calendar-week-4","name":"Week","children":[
      {"type":"frame","id":"ui-calendar-day-11-23","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-23-label","content":"23","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-24","name":"Day, selected","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$primary","children":[
        {"type":"text","id":"ui-calendar-day-11-24-label","content":"24","fontFamily":"$font","fontSize":14,"fill":"$primary-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-25","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-25-label","content":"25","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-26","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-26-label","content":"26","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-27","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-27-label","content":"27","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-28","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-28-label","content":"28","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-11-29","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-29-label","content":"29","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}]},
    {"type":"frame","id":"ui-calendar-week-5","name":"Week","children":[
      {"type":"frame","id":"ui-calendar-day-11-30","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-11-30-label","content":"30","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-calendar-day-12-1","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-12-1-label","content":"1","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-12-2","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-12-2-label","content":"2","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-12-3","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-12-3-label","content":"3","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-12-4","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-12-4-label","content":"4","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-12-5","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-12-5-label","content":"5","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-calendar-day-12-6","name":"Day, outside","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-calendar-day-12-6-label","content":"6","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]}]}]}]}
```

Dates are centred in their squares, so one-figure and two-figure dates begin at different places; the
review's remark on nearly aligned text can stand for a calendar.

The month must be a real one: the right weekday under each date, the right number of days. Write the dates
out from a calendar, never from memory. Two months stand side by side, 16 apart, each with its own caption.

A range: the first and last day are `$primary` and rounded; a `$muted` band runs behind them and through
the days between, square where it continues and rounded only at its two outer ends.

```json
{"type":"frame","id":"ui-range-week","name":"Week with a range","children":[
  {"type":"frame","id":"ui-range-23","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"text","id":"ui-range-23-label","content":"23","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-range-24","name":"Day, range start","width":32,"height":32,"cornerRadius":["$radius-md",0,0,"$radius-md"],"fill":"$muted","children":[
    {"type":"frame","id":"ui-range-24-end","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$primary","children":[
      {"type":"text","id":"ui-range-24-label","content":"24","fontFamily":"$font","fontSize":14,"fill":"$primary-foreground"}]}]},
  {"type":"frame","id":"ui-range-25","name":"Day, in range","width":32,"height":32,"justifyContent":"center","alignItems":"center","fill":"$muted","children":[
    {"type":"text","id":"ui-range-25-label","content":"25","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-range-26","name":"Day, in range","width":32,"height":32,"justifyContent":"center","alignItems":"center","fill":"$muted","children":[
    {"type":"text","id":"ui-range-26-label","content":"26","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-range-27","name":"Day, in range","width":32,"height":32,"justifyContent":"center","alignItems":"center","fill":"$muted","children":[
    {"type":"text","id":"ui-range-27-label","content":"27","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-range-28","name":"Day, range end","width":32,"height":32,"cornerRadius":[0,"$radius-md","$radius-md",0],"fill":"$muted","children":[
    {"type":"frame","id":"ui-range-28-end","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$primary","children":[
      {"type":"text","id":"ui-range-28-label","content":"28","fontFamily":"$font","fontSize":14,"fill":"$primary-foreground"}]}]},
  {"type":"frame","id":"ui-range-29","name":"Day","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
    {"type":"text","id":"ui-range-29-label","content":"29","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}]}
```

## Date field

A date field is drawn closed, showing a date written the way its readers write it ("24 November 2026", not
"2026-11-24", unless the product is for people who think in the second). A `calendar` icon leads. Open, the
calendar is a popover 4 below it, start edges aligned, and the field shows its `focus` state. A range field
shows both dates with an en dash: "24 – 28 November 2026".

```json
{"type":"frame","id":"ui-date-field","name":"Date field","reusable":true,"width":280,"height":36,"padding":[0,10],"gap":8,"alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","states":{"focus":{"fill":["$background","$control"],"stroke":"$ring","effect":{"type":"shadow","offset":{"x":0,"y":0},"blur":0,"spread":3,"color":"$ring-soft"}}},"children":[
  {"type":"icon","id":"ui-date-field-icon","name":"Icon","library":"lucide","icon":"calendar","width":16,"height":16,"fill":"$muted-foreground"},
  {"type":"text","id":"ui-date-field-value","name":"Value","content":"24 November 2026","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"}]}
```

## Combobox

A select whose list can be searched: use it from about eight choices up. The trigger is the select of
`references/controls.md` with `chevrons-up-down` as its icon. The list has a search row on top (36 high, a
line under it), then the rows of a menu: 32 high, radius `$radius-sm`, group labels of 12. Highlighted (where
the keyboard is) is `$accent`; chosen is a `check` at the trailing edge; they are different things and one
row can be both. Drawn with a query typed, the list shows only what matches, in a believable order, and
groups without a match are gone with their labels. No match at all: one line of 14 in `$muted-foreground`,
centred, 8 above and below, naming the query.

```json
{"type":"frame","id":"ui-combobox","name":"Combobox list","reusable":true,"width":280,"layout":"vertical","cornerRadius":"$radius-md","fill":"$popover","effect":[{"type":"shadow","offset":{"x":0,"y":1},"blur":3,"color":"#0000000f"},{"type":"shadow","offset":{"x":0,"y":6},"blur":14,"color":"#00000014"}],"stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"ui-combobox-search","name":"Search","width":"fill_container","height":36,"padding":[0,12],"gap":8,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
    {"type":"icon","id":"ui-combobox-search-icon","library":"lucide","icon":"search","width":16,"height":16,"fill":"$muted-foreground"},
    {"type":"text","id":"ui-combobox-search-value","name":"Value","content":"sol","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"frame","id":"ui-combobox-list","name":"List","width":"fill_container","layout":"vertical","padding":4,"children":[
    {"type":"frame","id":"ui-combobox-heading","name":"Group label","width":"fill_container","height":28,"padding":[0,8],"alignItems":"center","children":[
      {"type":"text","id":"ui-combobox-heading-text","content":"Teammates","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$muted-foreground"}]},
    {"type":"frame","id":"ui-combobox-item-1","name":"Item, highlighted","width":"fill_container","height":32,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-sm","fill":"$accent","children":[
      {"type":"text","id":"ui-combobox-item-1-label","content":"Maren Solheim","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"icon","id":"ui-combobox-item-1-indicator","name":"Indicator","library":"lucide","icon":"check","width":16,"height":16,"fill":"$foreground"}]},
    {"type":"frame","id":"ui-combobox-item-2","name":"Item","width":"fill_container","height":32,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-sm","children":[
      {"type":"text","id":"ui-combobox-item-2-label","content":"Solveig Aune","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"}]},
    {"type":"frame","id":"ui-combobox-item-3","name":"Item","width":"fill_container","height":32,"padding":[0,8],"gap":8,"alignItems":"center","cornerRadius":"$radius-sm","children":[
      {"type":"text","id":"ui-combobox-item-3-label","content":"Aksel Bjørnstad-Lie Solli","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"}]}]}]}
```

## Chips field

For choosing several, and for tags. The field is the input's skin with 6 inside; a chip is 22 high with
radius `$radius-sm`, `$muted`, 12 at weight 500, and an `x` of 12 that removes it. What is typed next follows
the last chip. Pencraft's layouts do not wrap: when the chips need a second line, make the field a vertical
frame of rows, each 22 high and 6 apart, with 6 above and below.

```json
{"type":"frame","id":"ui-chips-field","name":"Chips field","width":360,"height":36,"padding":[0,6],"gap":6,"alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"ui-chip-1","name":"Chip","height":22,"padding":[0,0,0,6],"gap":2,"alignItems":"center","cornerRadius":"$radius-sm","fill":"$muted","children":[
    {"type":"text","id":"ui-chip-1-label","content":"Maren Solheim","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground"},
    {"type":"frame","id":"ui-chip-1-remove","name":"Remove","width":20,"height":22,"justifyContent":"center","alignItems":"center","children":[
      {"type":"icon","id":"ui-chip-1-remove-icon","library":"lucide","icon":"x","width":12,"height":12,"fill":"$muted-foreground"}]}]},
  {"type":"frame","id":"ui-chip-2","name":"Chip","height":22,"padding":[0,0,0,6],"gap":2,"alignItems":"center","cornerRadius":"$radius-sm","fill":"$muted","children":[
    {"type":"text","id":"ui-chip-2-label","content":"Ingrid Vang","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground"},
    {"type":"frame","id":"ui-chip-2-remove","name":"Remove","width":20,"height":22,"justifyContent":"center","alignItems":"center","children":[
      {"type":"icon","id":"ui-chip-2-remove-icon","library":"lucide","icon":"x","width":12,"height":12,"fill":"$muted-foreground"}]}]},
  {"type":"text","id":"ui-chips-field-value","name":"Value","content":"Add a teammate","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]}
```

## One-time code

Squares of 36 joined into groups (the outer corners of a group rounded, one line between neighbours), groups
8 apart with a `minus` between them. Six digits are two groups of three. Draw it part filled: the slot the
keyboard is on has the focus ring and a caret 1 wide and 16 high; slots after it are empty.

```json
{"type":"frame","id":"ui-otp","name":"One-time code","gap":8,"alignItems":"center","children":[
  {"type":"frame","id":"ui-otp-group-1","name":"Group","children":[
    {"type":"frame","id":"ui-otp-1","name":"Slot","width":36,"height":36,"justifyContent":"center","alignItems":"center","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","fill":"$control","cornerRadius":["$radius-md",0,0,"$radius-md"],"children":[
      {"type":"text","id":"ui-otp-1-digit","content":"4","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
    {"type":"frame","id":"ui-otp-2","name":"Slot","width":36,"height":36,"justifyContent":"center","alignItems":"center","stroke":"$input","strokeWidth":{"top":1,"right":1,"bottom":1},"strokeAlignment":"inner","fill":"$control","children":[
      {"type":"text","id":"ui-otp-2-digit","content":"8","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
    {"type":"frame","id":"ui-otp-3","name":"Slot","width":36,"height":36,"justifyContent":"center","alignItems":"center","stroke":"$input","strokeWidth":{"top":1,"right":1,"bottom":1},"strokeAlignment":"inner","fill":"$control","cornerRadius":[0,"$radius-md","$radius-md",0],"children":[
      {"type":"text","id":"ui-otp-3-digit","content":"2","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]}]},
  {"type":"icon","id":"ui-otp-separator","name":"Separator","library":"lucide","icon":"minus","width":16,"height":16,"fill":"$muted-foreground"},
  {"type":"frame","id":"ui-otp-group-2","name":"Group","children":[
    {"type":"frame","id":"ui-otp-4","name":"Slot, active","width":36,"height":36,"justifyContent":"center","alignItems":"center","stroke":"$ring","strokeWidth":1,"strokeAlignment":"inner","fill":["$background","$control"],"cornerRadius":["$radius-md",0,0,"$radius-md"],"effect":{"type":"shadow","offset":{"x":0,"y":0},"blur":0,"spread":3,"color":"$ring-soft"},"children":[
      {"type":"rectangle","id":"ui-otp-4-caret","width":1,"height":16,"fill":"$foreground"}]},
    {"type":"frame","id":"ui-otp-5","name":"Slot","width":36,"height":36,"justifyContent":"center","alignItems":"center","stroke":"$input","strokeWidth":{"top":1,"right":1,"bottom":1},"strokeAlignment":"inner","fill":"$control","children":[]},
    {"type":"frame","id":"ui-otp-6","name":"Slot","width":36,"height":36,"justifyContent":"center","alignItems":"center","stroke":"$input","strokeWidth":{"top":1,"right":1,"bottom":1},"strokeAlignment":"inner","fill":"$control","cornerRadius":[0,"$radius-md","$radius-md",0],"children":[]}]}]}
```

## Input group

An input with something fixed before or after what is typed: a unit, a currency, a part of an address. Those
parts are 14 in `$muted-foreground` inside the field's own border, 6 from the value. A button inside a field
(show a password, clear a search) is a 24 square ghost icon, 6 from the trailing edge.

```json
{"type":"frame","id":"ui-input-group","name":"Input group","width":320,"height":36,"padding":[0,10],"gap":6,"alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"text","id":"ui-input-group-prefix","name":"Prefix","content":"NOK","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"},
  {"type":"text","id":"ui-input-group-value","name":"Value","content":"245","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
  {"type":"text","id":"ui-input-group-suffix","name":"Suffix","content":"an hour","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]}
```

## Number input

Convention, built from this kit's tokens. The value centred between a `minus` and a `plus`, each a square of
the field's height with a line toward the value. At its lowest value `minus` has `"opacity":0.5`. For an
amount of money or a measurement, use a plain input with the unit as a suffix instead.

```json
{"type":"frame","id":"ui-number","name":"Number input","height":36,"cornerRadius":"$radius-md","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","clip":true,"children":[
  {"type":"frame","id":"ui-number-minus","name":"Fewer","width":36,"height":36,"justifyContent":"center","alignItems":"center","stroke":"$input","strokeWidth":{"right":1},"children":[
    {"type":"icon","id":"ui-number-minus-icon","library":"lucide","icon":"minus","width":16,"height":16,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-number-value-box","width":56,"height":36,"justifyContent":"center","alignItems":"center","children":[
    {"type":"text","id":"ui-number-value","name":"Value","content":"8","fontFamily":"$font","fontSize":14,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-number-plus","name":"More","width":36,"height":36,"justifyContent":"center","alignItems":"center","stroke":"$input","strokeWidth":{"left":1},"children":[
    {"type":"icon","id":"ui-number-plus-icon","library":"lucide","icon":"plus","width":16,"height":16,"fill":"$foreground"}]}]}
```

## Toggle group and segmented control

A toggle group is a row of joined buttons, each a square of the control's height (or with 10 beside a label),
one line between neighbours; what is on has `$muted`. For choosing exactly one of two to five views. A
segmented control with labels is the tab list of `references/controls.md` with every item the same width.

```json
{"type":"frame","id":"ui-toggle-group","name":"Toggle group","height":36,"cornerRadius":"$radius-md","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","clip":true,"children":[
  {"type":"frame","id":"ui-toggle-1","name":"Toggle, on","width":36,"height":36,"justifyContent":"center","alignItems":"center","stroke":"$input","strokeWidth":{"right":1},"fill":"$muted","children":[
    {"type":"icon","id":"ui-toggle-1-icon","library":"lucide","icon":"list","width":16,"height":16,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-toggle-2","name":"Toggle","width":36,"height":36,"justifyContent":"center","alignItems":"center","stroke":"$input","strokeWidth":{"right":1},"children":[
    {"type":"icon","id":"ui-toggle-2-icon","library":"lucide","icon":"layout-grid","width":16,"height":16,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-toggle-3","name":"Toggle","width":36,"height":36,"justifyContent":"center","alignItems":"center","children":[
    {"type":"icon","id":"ui-toggle-3-icon","library":"lucide","icon":"calendar","width":16,"height":16,"fill":"$foreground"}]}]}
```

## File drop

Convention, built from this kit's tokens. A panel with a 1 wide `$input` border (the web
draws it dashed; Pencraft's shapes have solid strokes only), radius `$radius-lg`, 32 inside: a 40 tile with
an `upload` icon, one line of 14 at weight 500 saying what to drop, one of 12 with the limits. Files that
were added are listed under it as rows (name, size, a progress bar while one uploads, an `x` to remove it).

```json
{"type":"frame","id":"ui-dropzone","name":"File drop","width":480,"layout":"vertical","gap":12,"padding":32,"alignItems":"center","cornerRadius":"$radius-lg","fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"ui-dropzone-media","name":"Media","width":40,"height":40,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-lg","fill":"$muted","children":[
    {"type":"icon","id":"ui-dropzone-icon","library":"lucide","icon":"upload","width":16,"height":16,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-dropzone-text","layout":"vertical","gap":4,"alignItems":"center","children":[
    {"type":"text","id":"ui-dropzone-title","name":"Title","content":"Drop the timesheets here, or browse","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
    {"type":"text","id":"ui-dropzone-description","name":"Description","content":"CSV or PDF, up to 10 MB each","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]}]}
```
