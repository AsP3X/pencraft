# Colours for charts

Add these once per document with one `edit_layers` call. If the document already has a `Mode` axis, leave
`themes` out.

```json
{"op":"set_variables",
 "themes":{"Mode":["Light","Dark"]},
 "variables":{
  "chart-1":{"type":"color","value":[{"value":"#1868db"},{"value":"#669df1","theme":{"Mode":"Dark"}}]},
  "chart-2":{"type":"color","value":[{"value":"#6a9a23"},{"value":"#b3df72","theme":{"Mode":"Dark"}}]},
  "chart-3":{"type":"color","value":[{"value":"#af59e1"},{"value":"#d8a0f7","theme":{"Mode":"Dark"}}]},
  "chart-4":{"type":"color","value":[{"value":"#e06c00"},{"value":"#fbc828","theme":{"Mode":"Dark"}}]},
  "chart-5":{"type":"color","value":[{"value":"#123263"},{"value":"#6cc3e0","theme":{"Mode":"Dark"}}]},
  "chart-muted":{"type":"color","value":[{"value":"#7d818a"},{"value":"#96999e","theme":{"Mode":"Dark"}}]},
  "chart-grid":{"type":"color","value":[{"value":"#e5e5e580"},{"value":"#ffffff0d","theme":{"Mode":"Dark"}}]},
  "chart-good":{"type":"color","value":[{"value":"#6a9a23"},{"value":"#94c748","theme":{"Mode":"Dark"}}]},
  "chart-warn":{"type":"color","value":[{"value":"#e06c00"},{"value":"#fca700","theme":{"Mode":"Dark"}}]},
  "chart-bad":{"type":"color","value":[{"value":"#c9372c"},{"value":"#f15b50","theme":{"Mode":"Dark"}}]},
  "chart-up":{"type":"color","value":"#26a69a"},
  "chart-down":{"type":"color","value":"#ef5350"}}}
```

| Variable | For |
|---|---|
| `chart-1` to `chart-5` | Series, in this order: blue, green, purple, orange, and a deep blue (a light teal in the dark theme). Each stands 3:1 or better against `$card` in its theme, and neighbours differ in lightness as well as hue |
| `chart-muted` | What is only there to compare; "Other"; data that is missing |
| `chart-grid` | Gridlines: the border colour at half strength |
| `chart-good`, `chart-warn`, `chart-bad` | A judgement in the data: on time, late, failed |
| `chart-up`, `chart-down` | A price that closed higher or lower than it opened, in a candlestick chart; the same in both themes |

The series and state colours are the categorical and status chart tokens of Atlassian's design system, read
from its published token package (light and dark); the gridline is shadcn/ui's; up and down are the
defaults of TradingView's Lightweight Charts.

## The roles a chart writes with

A chart's text and surfaces use the roles of the design it stands in. The ui-components skill's tokens have
them all. In a document without them, add:

```json
{"op":"set_variables",
 "themes":{"Mode":["Light","Dark"]},
 "variables":{
  "background":{"type":"color","value":[{"value":"#ffffff"},{"value":"#0a0a0a","theme":{"Mode":"Dark"}}]},
  "card":{"type":"color","value":[{"value":"#ffffff"},{"value":"#171717","theme":{"Mode":"Dark"}}]},
  "foreground":{"type":"color","value":[{"value":"#0a0a0a"},{"value":"#fafafa","theme":{"Mode":"Dark"}}]},
  "muted":{"type":"color","value":[{"value":"#f5f5f5"},{"value":"#262626","theme":{"Mode":"Dark"}}]},
  "muted-foreground":{"type":"color","value":[{"value":"#5c5c5c"},{"value":"#a8a8a8","theme":{"Mode":"Dark"}}]},
  "border":{"type":"color","value":[{"value":"#e5e5e5"},{"value":"#ffffff1a","theme":{"Mode":"Dark"}}]},
  "font":{"type":"string","value":"Inter"}}}
```

Change `font` to a family that is installed (`get_guide` lists them) before you send it.

## In a design with a palette of its own

- One series takes the design's accent. Check it stands 3:1 against the card.
- Several series: the accent first, then colours that differ from it and from each other in lightness as
  well as hue, so they hold in greyscale and for the one man in twelve who does not tell red from green.
  Set them as `chart-1` to `chart-5` with a value for each theme; never write a hex into a mark.
- Parts of one thing that have an order (low to high, new to old) are steps of the accent from light to
  dark, not five hues.
- In the dark theme take a lighter step of each colour, not the same one: a mid blue that is right on white
  is dull on near-black.
- shadcn/ui's neutral theme sets its five chart colours to five greys, the same in both themes. The
  lightest is about 1.5:1 on a white card and the darkest nearly invisible on a dark one. When the user
  asks for that look, start the series at the second grey in the light theme, stop at the third in the dark
  one, and write the values beside the marks.
