---
name: charts
description: Draw charts and data graphics in Quill that are true to their numbers - bar, grouped and stacked bar, line, area, sparkline, donut, gauge, progress ring, ranked bar list, uptime strip and candlestick (a price chart as in trading apps), with axes, gridlines, legend, tooltip and the card around them - built from values, not by eye. Use when the user asks for a chart, graph, plot, diagram of data, data visualisation, analytics, statistics, a report or a dashboard that shows figures over time or by category, a KPI with a trend, a share, crypto or currency price, or names a chart type. Also for how figures are written: separators, decimals, units, currency, percentages, changes and dates. Do not use for a flow chart, an org chart or any diagram of boxes and arrows (those are connectors), nor for a table (the ui-components skill).
license: Apache-2.0 with Commons Clause (see LICENSE in the Quill repository)
metadata:
  product: pencraft
  version: "1"
---

# Charts in Quill

A chart is a claim about numbers. In a design it is drawn by hand, so nothing stops a bar from being the
wrong height or a total from disagreeing with its parts, and a reader sees that at once: it is what makes a
mock-up chart look fake. This skill makes the chart from its values: decide what it says, write the data
down, work out every size from it, then draw.

Work with Quill's tools (`get_state`, `read_layers`, `edit_layers`, `screenshot`). The reference files
named below are read with `read_skill` (name `charts`, `file` as given).

Part of this skill is firm and part is a starting point. It says which, and the difference matters:

- Firm: the list "Every time" below, and truth to the numbers everywhere in it: sizes worked out from
  values, totals that agree, real dates. Do these without being asked, on every design this skill is used
  for. Only the user's own words overrule them; then do what was asked and say in one line what it costs.
- A starting point: how a chart looks (the palette, the corner of a bar, the weight of a line, gridlines,
  where the legend stands) and the measures of section 3. Begin there, and change it when the design has a
  reason. They are still the expected way: leave one only with a reason, which can be short (the product has
  two plans, not three; the face has no such weight) and needs nobody's leave.
- A design that already has a look, a brand or a system keeps it. Take from this skill what the design
  lacks, and nothing that would replace what it has.
- The examples in the reference files show how a thing is built in Quill and at what size. Their colours,
  faces, words and arrangement are one answer among many, not the look to reproduce.
- The checklist and "When it looks wrong" at the end are there to be gone through every time, as a check
  of your own work. A line in them that speaks of a starting point is a starting point still.

Every time:

1. Write the data down before the first layer: every value, its unit, the period. Every size in the chart
   is worked out from it (section 4). Never place a mark by eye.
2. Bars and areas stand on zero. A line may start elsewhere, and then the axis says so.
3. What is shown twice agrees: parts sum to their total, a change matches the two values it compares, the
   figure in a tile is the last point of its line.
4. No chart that is only decoration. If the request has no numbers, ask for them or make up a believable
   set and say so (section 6).

Starting points (not part of the list above: begin here, and change one when the user or the design
gives a reason):

- The data is the loudest thing. Gridlines are faint, there is no axis line and no tick mark, labels are
  12 and quiet.
- Colour means something or is left out. One series is one colour; grey is for what is only there to
  compare. A colour per bar says the bars are different things: use it when they are.

## 1. Decide what the chart says

Write one sentence the reader should take away ("Ticket sales peaked in July"). That sentence is the chart's
title; the line under it says what is measured, in which unit, over which period. Then choose the form:

| The reader should see | Use | Not |
|---|---|---|
| How categories compare | Bars, sorted by value unless the categories have an order of their own | A donut with more than five slices |
| A count per period (sales per month) | Bars, in time order | A line: each bar is a sum, not a reading |
| How something changes over time | A line (two to five; more only if all but one are grey) | Bars for a rate or a level |
| A total over time, and how full it is | An area, on zero | A line with a decorative fill |
| Two or three series per category | Grouped bars | Two charts side by side with different scales |
| Parts of a total, per category | Stacked bars | Stacks of more than four parts |
| Two to five parts of one whole | A donut with the total in its centre, or one stacked bar | A pie of many thin slices, 3D, an exploded slice |
| A ranking with names | A bar list: horizontal bars with the name on the bar | Vertical bars with slanted labels |
| One value against a limit | A gauge or a progress ring | A gauge for something without a limit |
| The trend alone, beside a figure | A sparkline: no axes, no labels | A small chart with axes |
| Good and bad days at a glance | A tracker: one block per day | A line of a yes or no |
| A price through each period: where it opened, how far it went, where it closed | A candlestick chart, with the volume traded under it | Candles for anything that has no open, high, low and close |
| One number | A stat tile (the ui-components skill has one), not a chart | |

A relation between two measures is a scatter of points; a value over two categories is a heat map of cells
in steps of one hue; a drop from step to step is a funnel of bars that shrink. No library measured here
publishes their anatomy: build them from the same axes, labels and colours as below, and say that the
layout is the usual one.

## 2. Set the colours once

Run the `set_variables` call in `references/colours.md` before the first layer. It adds five series colours
(`chart-1` to `chart-5`), a grey for context (`chart-muted`), the gridline (`chart-grid`) and three state
colours, each with a light and a dark value. The chart's text, card and lines use the roles a design
already has (`foreground`, `muted-foreground`, `card`, `muted`, `border`, `font`): the same file gives them
for a document that has none. In a document with a palette of its own, map the series onto it as that file
says and keep the rest.

## 3. Anatomy

| Part | Value | Why |
|---|---|---|
| Plot | 200 high in a card (160 to 320); 120 at least | Below that the steps cannot be told apart: draw a sparkline |
| Gridlines | Horizontal only, 1 high, `$chart-grid`; the zero line in `$border` | They help to read a height and nothing else |
| Number of gridlines | 3 to 5, on round steps that bracket the data (0, 2,000, 4,000, 6,000) | Never 0, 1,300, 2,600 |
| Axis line, tick marks | None | The first gridline is the axis |
| Tick labels | 12 in `$muted-foreground`; the value axis 8 before the plot, ending on one edge; the category axis 2 to 10 under it, centred on its mark | |
| Category labels | Short: "Jun", "W47", "9 Nov"; every second or third when they would touch | Slanted labels are hard to read: shorten or skip some instead |
| Bar | Its band less a gap of a third to a half; top corners 4 (8 when the bar is 48 wide or more) | Square at the baseline: it stands on it |
| Bars of one group | 4 apart; groups a bar's width apart | |
| Stacked parts | 2 apart (taken off the lower part), only the top of the stack rounded | The gap keeps two touching colours apart |
| Line | 2 wide, round joins, straight from point to point | A curve claims values between the points |
| Points | None on a line of evenly spaced dates; one of 8 at the end that is labelled or under the pointer | |
| Area | The series colour at 30%, with its 2 wide line on top | |
| Donut | 160 across (120 to 200), the ring 28% of the radius, slices 2 degrees apart, starting at the top and running clockwise from largest to smallest | |
| Centre of a donut or gauge | The figure at 24 to 28 and weight 600, one line of 12 under it | |
| Gauge | 270 degrees open at the bottom, ring 14% of the radius, track `$muted` | |
| Progress ring | 64 across, ring 6 wide, track `$muted`, starting at the top, clockwise | |
| Sparkline | 112 by 48, a 2 wide line, nothing else | |
| Bar list | Rows 32 high and 6 apart; the bar at 25% of its colour with the name on it, 8 in; the value outside, ending on one edge | |
| Tracker | Blocks 32 high and 2 apart, radius 1, the strip's ends 4 | |
| Candle | A body 60% of its band wide from open to close (1 high at least), a wick 1 wide through its middle from high to low; body and wick in `$chart-up` when it closed at or above its open, `$chart-down` when below; no rounded corners | The body is the day's trade, the wick how far it strayed |
| Price axis | At the trailing side, prices with the decimals the market quotes; not from zero: round steps that bracket the lowest low and highest high | A price chart is read at its newest end |
| Last price | A 1 high line across the plot at half of the last candle's colour, and the price on the axis in a tag (`$foreground` with `$background` text) | |
| Volume | A strip 40 high under the plot, one bar per candle as wide as its body, at half of the candle's colour, on a scale of its own from zero | |
| Legend | Under the chart, centred, 12 from it: an 8 square (radius 2) or, for lines, a 14 by 3 dash; the name at 12, 6 after it; items 16 apart | |
| Tooltip | A card with radius 10 and a `$border` line, 6 above and below and 10 beside, at least 128 wide; the label at weight 500, then a 10 square, the name in `$muted-foreground` and the value at weight 500 at the trailing edge | |

## 4. Build it from the values

A plot is a frame of fixed size. With `top` the highest tick and `low` the lowest (0 for bars and areas),
and `H` the plot's height:

- A bar's height is `value / top * H`, rounded to a whole number. Nothing else decides it.
- A gridline for tick `t` lies `(t - low) / (top - low) * H` above the baseline.
- A point of a line is `x = inset + i * (W - 2 * inset) / (n - 1)` across and
  `y = H * (1 - (value - low) / (top - low))` down from the top tick.
- A candle in band `i` of `n` has its middle at `(i - 0.5) * W / n`. Its wick runs from `y(high)` to
  `y(low)` and its body from `y` of the higher of open and close to `y` of the lower, with `y` as for a line.
- A slice of a donut sweeps `value / total * 360` degrees. Quill measures angles counter-clockwise from
  the right, so a slice that begins `turned` degrees clockwise from the top has
  `"startAngle": 90 - turned - sweep` and `"sweepAngle": sweep`; take 1 off each end for the gap.
- A gauge that is `share` full: the track has `"startAngle": -45, "sweepAngle": 270`, the value
  `"startAngle": 225 - 270 * share, "sweepAngle": 270 * share`.

How the layers go together (ready examples of each in `references/charts.md`):

- Bars: the plot is a horizontal frame with `"alignItems":"end"`, a `gap`, and half that gap as its side
  padding; each bar is a rectangle with `"width":"fill_container"` and its worked-out height. The category
  axis under it is a second frame with the same padding and gap, so every label is centred under its bar.
  Gridlines are rectangles 1 high with `"layoutPosition":"absolute"`, listed first so they lie behind.
- The value axis is a column as high as the plot with `"justifyContent":"space_between"`: its labels have
  `"lineHeight":1.3333` (16 high), and the plot has 8 of padding above and below, so each label's middle is
  level with its gridline.
- A line is one `path` as large as the plot, its `viewBox` the plot's own size and its `geometry` the points
  joined with `L`: `"M24 122.7 L78.9 137.3 ..."`. It has a `stroke` and no `fill`. An area is a second path
  under it: the same points, then down to the baseline and back, closed with `Z`, filled with the series
  colour at 30% (`{"type":"color","color":"$chart-1","opacity":0.3}`).
- Candles are rectangles placed in a `"layout":"none"` plot: for each period the wick, then the body
  over it. The volume strip and the time axis are frames of the plot's width under it, and the price axis
  stands after them, at the trailing side.
- A donut, a gauge and a ring are ellipses of one size laid over each other in a `"layout":"none"` frame,
  each with `innerRadius`, `startAngle` and `sweepAngle`, and a centred frame of the same size over them
  for the figure.

Check the arithmetic once more against the picture: the tallest bar reaches where its value says, between
the right two gridlines.

## 5. Colour

- One series: `$chart-1`, or the design's own accent. Not a colour per bar.
- Several series take `$chart-1`, `$chart-2` and on in that order, and keep their colour in every chart of
  the screen. Five at most; what is left over becomes "Other" in `$chart-muted`.
- What is only there to compare (last year, the average, the other routes) is `$chart-muted`, and the one
  thing the chart is about has the colour. On a line chart with more than three lines this is the rule,
  not the exception.
- Name a line at its end, in words, instead of a legend where there is room; a legend is for bars and for
  lines that cross too much to label.
- Steps of an amount (a heat map, a map) are steps of one hue from light to dark; in the dark theme the
  lightest step is the largest value. A scale with a meaningful middle (above and below target) is two
  hues that meet in a light step.
- Good, warning and bad are `$chart-good`, `$chart-warn`, `$chart-bad`, and only where the data is a
  judgement (on time or late). In the light theme they are the greens and oranges of `chart-2` and
  `chart-4`: do not use both sets in one chart.
- Up and down in a price chart are `$chart-up` and `$chart-down` (green and red in most of the world; in
  China, Japan and Korea red is up and green is down: follow the market the product is for, and swap the
  two variables, not single candles). The change beside the price carries its sign as well.
- Never colour alone: a line is also named, a state is also a word or a count, two touching areas have a
  gap between them.
- A mark must stand 3:1 against what it lies on, unless its value is written beside it. The series colours
  here do on `$card` in both themes; a library's light grey first series does not.
- No gradients on bars, no glow, no shadow on a mark, no 3D.

## 6. Numbers that hold up

Before you draw, and again before you answer:

- Parts sum to the total shown (the centre of a donut, the top of a stack, the footer of a card), and
  shares sum to 100.
- A change is worked out from the two values it compares: "down 40% from July" is (5,240 - 3,140) / 5,240.
- The figure in a tile is the last point of the line beside it. The same quantity has the same value
  everywhere on the screen.
- Ticks are round and bracket the data; the unit is in the axis, the title or the line under it.
- Dates are real and evenly spaced: the right weekday, the right number of days in the month.
- In a price chart every candle obeys its own numbers: the high is the highest of the four and the low the
  lowest, a day opens at or near the close before it, and there is no candle on a day the market is shut
  (no Saturdays and Sundays for shares; every day for crypto). The price in the header is the last close.
- Made-up data has the shape of real data: uneven values (4,180, not 4,000), a dip, a season or a weekend
  rhythm, one value that stands out. Not a steady climb, not a perfect wave. Start from the figure the
  screen already shows and walk back from it.
- At most one moment of use: one bar or point under the pointer, with a tooltip whose values are the ones
  drawn there.

How figures are written (separators, decimals, units, changes, dates): `references/figures.md`.

## 7. Where a chart stands

- In a card: a title that says what the chart shows, one line of 14 in `$muted-foreground` with measure,
  unit and period, the plot, and under it at most two lines: what changed, at weight 500, and what was
  counted. `references/charts.md` has the card.
- Align the plot's leading edge with the card's text; the value axis may stand at the trailing side
  instead, to keep that edge clean.
- Two charts side by side that are to be compared share one scale and one height.
- In a stat tile a sparkline stands at the trailing edge, level with the figure. A tile with a figure and
  no room for 112 by 48 gets no chart.
- On a phone a chart is as wide as the screen less 16 on each side, 160 to 200 high, with three gridlines
  and every second category label.
- With the ui-components skill: its tokens are the roles this skill uses, and the stat tile and the
  dashboard are there. Add the chart variables to them.

## 8. Workflow

1. `get_state`, `read_layers`: which variables are there, how wide the chart may be.
2. The sentence and the form (section 1). The data, written out (section 6).
3. The variables (`references/colours.md`), once.
4. Work out the ticks, then every height, point and angle (section 4).
5. Insert the chart from the nearest example in `references/charts.md`, with your values and sizes.
6. `screenshot` with the chart's `id`; put right what the review counted; then check section 6 against the
   picture.
7. Tell the user in two sentences what the chart shows and, if you made the data up, that you did.

## 9. What Quill cannot draw, and what to do instead

| Missing | Instead |
|---|---|
| Figures of one width (tabular numerals) | Give each column of figures one fixed width and `"textAlign":"right"`; avoid faces with old-style figures |
| Dashed strokes on shapes (a projection, a target line, the last-price line of a trading chart) | A solid 1 wide line in `$chart-muted` with its name at its end; a projected part of a series at 40% of its colour |
| Round ends on an arc | Square ends; never fake them with dots |
| A layout that wraps | A legend too long for one row is a column beside the chart |
| Hover, zoom, animation | One frame with one tooltip; a second frame for a second moment |
| A chart from a data file | Read the values first and write them into the layers |

## 10. When it looks wrong

| It looks like | Cause | Fix |
|---|---|---|
| A fake | Round values, an even climb, heights by eye | Uneven values with a dip; work the heights out |
| A spreadsheet default | Axis lines, tick marks, a box round the plot, a grey ground | Take them away: gridlines and labels only |
| Noise | Vertical gridlines, a label on every point, seven colours | Horizontal lines, labels at the ends, grey for context |
| A toy | Bars 8 corners round on thin bars, bars thicker than their gaps by far, a gradient on each | Radius 4, a gap of a third to a half, flat colour |
| It lies | Bars that start above zero; two scales on one chart | Zero baseline; two charts |
| Cramped | A chart with axes in 100 by 60 | A sparkline |
| Unreadable in the dark | Light-theme colours on a dark card, black gridlines | The dark values of the variables; white at 5% gridlines |
