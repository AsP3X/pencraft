# Charts, dashboards / app shells, and components beyond the basic set (drawable specs, November 2026)

Retrieved 2026-11-20. All pixel values assume Tailwind's 4 px spacing unit and shadcn's radius base of 10
(`--radius: 0.625rem`; steps 0.6 / 0.8 / 1 / 1.4 = 6 / 8 / 10 / 14 for `sm / md / lg / xl`, as already used in
Pencraft's `ui-components/references/tokens.md`). "Vega" = shadcn classic (36 px controls), "Nova" = shadcn
compact and current default (32 px controls). Source shorthand used in links:

- SHADCN-VEGA = `apps/v4/registry/styles/style-vega.css` on `main` (last commit to that file 2026-10-14)
- SHADCN-NOVA = `apps/v4/registry/styles/style-nova.css`
- Values read by script from raw source files are "verified at source". Anything marked (unverified) was not
  confirmed at the source in this session. Several documentation sites (m3.material.io, atlassian.design,
  developer.apple.com HTML, polaris-react.shopify.com) do not serve content to a fetcher; where possible the
  underlying repo or JSON was read instead, and that is said in each bullet.

## 1. Charts: which types, exact anatomy and defaults, colour rules, shadcn chart tokens

### Takeaway
A product chart in the shadcn/Tremor idiom is very plain: horizontal-only 1 px solid gridlines in the border
colour at half strength, no axis line, no tick marks, 12 px muted tick labels, 2 px lines, bars with 4 to 8 px
corners, a 12 px tooltip card, and an 8 px square legend swatch. The current shadcn neutral theme's
`chart-1..5` are five greys (light to dark, identical in light and dark mode), and coloured themes use five
steps of one hue; the multi-hue orange/teal/navy/yellow set seen in most screenshots is the legacy 2025 palette.

### Cited Findings

**shadcn/ui chart container (Recharts wrapper), same in every style**
- Container is `aspect-video` (16:9) with `text-xs` (12 px); axis tick text uses `fill-muted-foreground`;
  cartesian grid lines are recoloured to `stroke-border/50` (the border colour at 50%); polar grid and reference
  lines use `stroke-border`; the line-chart hover cursor is `stroke-border`; the bar-chart hover cursor is a
  rectangle `fill-muted`; a radial bar's background track is `fill-muted`; dot and sector strokes that Recharts
  draws white are made transparent — [chart.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/chart.tsx)
- Tooltip card: `border-border/50 bg-background gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs shadow-xl`,
  plus `grid min-w-32 items-start` = radius 10, 1 px border at 50%, padding 10 beside and 6 above/below, 12 px
  text, min width 128, 6 px between label and rows, large shadow. Identical in Vega and Nova —
  [SHADCN-VEGA](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-vega.css),
  [SHADCN-NOVA](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-nova.css),
  [chart.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/chart.tsx)
- Tooltip rows: label line `font-medium`; each row `flex gap-2`; indicator is one of: dot = 10 x 10 with radius
  2; line = 4 wide, full row height, radius 2; dashed = a 1.5 px dashed border, no fill. Series name in
  `text-muted-foreground`; value in `font-mono font-medium text-foreground tabular-nums`, pushed to the right —
  [chart.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/chart.tsx)
- Legend: one centred row, `gap-4` (16) between items, `pt-3` (12) when under the chart or `pb-3` when above;
  each item `gap-1.5` (6) with an 8 x 8 swatch of radius 2 followed by the label at the container's 12 px —
  [chart.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/chart.tsx)

**shadcn chart examples (per type)**
- Bar, single series: `<CartesianGrid vertical={false} />` (horizontal lines only, solid), X axis with
  `tickLine={false}`, `tickMargin={10}`, month names cut to 3 letters, no Y axis drawn at all; bar
  `radius={8}` on all four corners; tooltip without label — [chart-bar-default.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-bar-default.tsx)
- Bar, two series side by side: `radius={4}`, tooltip indicator "dashed" — [chart-bar-multiple.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-bar-multiple.tsx)
- Bar, stacked: bottom segment `radius={[0, 0, 4, 4]}`, top segment `radius={[4, 4, 0, 0]}` (only the outer
  corners of the stack are rounded), legend shown — [chart-bar-stacked.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-bar-stacked.tsx)
- Bar with value labels: chart `margin top 20`; label above the bar at `offset={12}`, `fontSize={12}`,
  `fill-foreground` — [chart-bar-label.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-bar-label.tsx)
- Line: `strokeWidth={2}`, `type="natural"` (smoothed), chart margin left/right 12, `tickMargin={8}`; dots
  filled in the series colour, hovered dot `r: 6` — [chart-line-dots.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-line-dots.tsx)
- Area: `fillOpacity={0.4}`, `type="natural"`, stacked; the gradient variant fills with a vertical gradient of
  the series colour from 0.8 opacity at 5% to 0.1 opacity at 95% — [chart-area-gradient.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-area-gradient.tsx)
- Large "interactive" area chart in a card: chart is `aspect-auto h-[250px] w-full`; card header has a bottom
  border and `py-5` (20) with title/description left and a 160 wide select (range "Last 3 months / 30 days /
  7 days") right; content padding 24 (8 on small screens); legend below — [chart-area-interactive.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-area-interactive.tsx)
- Donut with centre text: square, `max-h-[250px]`; `innerRadius={60}`, `strokeWidth={5}` on the slices;
  centre shows the total in `text-3xl font-bold` (30 px) `fill-foreground` and a caption 24 px lower in
  `fill-muted-foreground` (12 px); five slices using chart-1..5 — [chart-pie-donut-text.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-pie-donut-text.tsx)
- Radial / gauge with text: `innerRadius={80}`, `outerRadius={90}` (a 10 px ring), `startAngle={0}`,
  `endAngle={250}`, `cornerRadius={10}` (round caps), muted background track, centre value `text-4xl
  font-bold` (36 px) plus muted caption 24 px lower, no ticks or axis — [chart-radial-text.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-radial-text.tsx)
- Chart cards end with a two-line footer at 14 px: line 1 `font-medium` with a 16 px trending icon ("Trending
  up by 5.2% this month"), line 2 `text-muted-foreground` ("Showing total visitors for the last 6 months"),
  gap 8 — [chart-bar-default.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-bar-default.tsx)
- The registry holds 70 chart examples in six families: area (10), bar (10), line (10), pie/donut (11), radar
  (14), radial (6), plus 9 tooltip variants. There is no scatter, heatmap or funnel example —
  [charts directory](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/new-york-v4/charts)

**shadcn `chart-1..5` tokens, current values**
- Neutral theme, light AND dark are the same five greys: chart-1 `oklch(0.87 0 0)`, chart-2 `oklch(0.556 0 0)`,
  chart-3 `oklch(0.439 0 0)`, chart-4 `oklch(0.371 0 0)`, chart-5 `oklch(0.269 0 0)` —
  [themes.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/themes.ts)
- The other neutral bases (stone, zinc, mauve, olive, mist, taupe) use the same five lightness steps with a
  faint tint, e.g. zinc `oklch(0.871 0.006 286.286)` ... `oklch(0.274 0.006 286.033)` — [themes.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/themes.ts)
- Coloured themes use five steps of the one hue, light and dark alike; blue: chart-1 `oklch(0.809 0.105
  251.813)`, chart-2 `oklch(0.623 0.214 259.815)`, chart-3 `oklch(0.546 0.245 262.881)`, chart-4 `oklch(0.488
  0.243 264.376)`, chart-5 `oklch(0.424 0.199 265.638)`. Themes available: amber, blue, cyan, emerald, fuchsia,
  green, indigo, lime, orange, pink, purple, red, rose, sky, teal, violet, yellow —
  [themes.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/themes.ts)
- The shadcn site's own stylesheet sets `--chart-1..5` to Tailwind `blue-300, blue-500, blue-600, blue-700,
  blue-800` in both light and dark blocks — [globals.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/app/globals.css)

**Tremor (source of the `tremorlabs/tremor` repo)**
- Bar, line and area charts default to `h-80 w-full` (320 high); gridlines `stroke-gray-200 stroke-1`
  (`gray-800` in dark), drawn only when `showGridLines` (default true); both axes `tickLine={false}`
  `axisLine={false}`; tick text `text-xs fill-gray-500` (same in dark); `yAxisWidth = 56`; minimum gap between
  X ticks `tickGap = 5`; chart margin top 5; optional axis titles in `text-sm font-medium fill-gray-800`
  (offset -20 for X, -15 for Y); `legendPosition = "right"` (legend sits above the plot, right-aligned) —
  [BarChart.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/BarChart/BarChart.tsx),
  [LineChart.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/LineChart/LineChart.tsx)
- Legend item: `gap-1.5 px-2 py-1 rounded-sm`, label `text-xs`; swatch is `size-2 rounded-xs` (8 x 8 square)
  for bar charts and `h-[3px] w-3.5 rounded-full` (a 14 x 3 dash) for line and area charts; non-hovered series
  drop to 40% opacity — [BarChart.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/BarChart/BarChart.tsx),
  [LineChart.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/LineChart/LineChart.tsx)
- Tooltip: `rounded-md border text-sm shadow-md`; a header block `border-b px-4 py-2` with the category label,
  then rows in `space-y-1 px-4 py-2`, each with the same swatch as the legend, name left and value right —
  [BarChart.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/BarChart/BarChart.tsx)
- Hover cursor: bar charts a band `fill #d1d5db` at opacity 0.15; line and area charts a vertical line
  `stroke #d1d5db`, 1 px; the active dot is `r={5}` with a white (dark: `gray-950`) stroke —
  [LineChart.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/LineChart/LineChart.tsx)
- Lines use `type="linear"` (straight segments) and `connectNulls = false` by default —
  [LineChart.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/LineChart/LineChart.tsx)
- Area fill is a vertical gradient of the series colour from 5% to 95% ending at opacity 0 —
  [AreaChart.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/AreaChart/AreaChart.tsx)
- Donut: `h-40 w-40` (160), `innerRadius "75%"` (pie = 0%), `outerRadius "100%"`, starts at 12 o'clock
  (`startAngle={90}`, `endAngle={-270}`), slices separated by a white (dark `gray-950`) stroke, centre label
  `fill-gray-700`, hovered slice dims others to 0.3; tooltip swatch is an 8 px circle —
  [DonutChart.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/DonutChart/DonutChart.tsx)
- Sparkline (spark area / line / bar): `h-12 w-28` (112 x 48), `strokeWidth={2}`, no axes; area gradient 0.4
  at 5% to 0 at 95%, or a flat 0.3 fill — [SparkChart.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/SparkChart/SparkChart.tsx)
- Default categorical order (Tailwind 500 steps): blue, emerald, violet, amber, gray, cyan, pink, lime,
  fuchsia (nine colours) — [chartColors.ts](https://github.com/tremorlabs/tremor/blob/main/src/utils/chartColors.ts)
- Tracker (uptime strip): `h-8` row of blocks, each block radius 1 with 0.5 px side padding, the strip's outer
  ends radius 4 — [Tracker.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/Tracker/Tracker.tsx)
- CategoryBar (segmented meter): bar `h-2` fully rounded, segments separated by `gap-0.5` (2), value labels
  above in `text-sm` within a 20 high row with 8 below; marker is a `h-4 w-1` rounded bar with a 2 px ring —
  [CategoryBar.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/CategoryBar/CategoryBar.tsx)
- BarList (ranked horizontal bars with the label inside): rows `h-8`, `space-y-1.5` (6) apart, bars
  `rounded-sm`, label `text-sm` inset 8 from the left on the bar, value `text-sm` right-aligned outside —
  [BarList.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/BarList/BarList.tsx)
- ProgressCircle: `radius = 32` (64 px across), `strokeWidth = 6`, a lighter track of the same hue behind —
  [ProgressCircle.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/ProgressCircle/ProgressCircle.tsx)

**Shopify Polaris Viz (constants in source)**
- `FONT_SIZE = 11` (touch 12), `LINE_HEIGHT = 14`; `DEFAULT_BORDER_RADIUS = 3` for bars (top corners only on
  vertical bars); `DEFAULT_LINE_WIDTH = 2`; lines are splines with an area under them by default
  (`DEFAULT_LINE_HAS_SPLINE = true`, `DEFAULT_LINE_HAS_AREA = true`); `DEFAULT_ARC_CORNER_RADIUS = 2`;
  horizontal gridlines on, `DEFAULT_GRID_HORIZONTAL_MARGIN = 16`; crosshair width 1; `STACKED_BAR_GAP = 2`;
  `MIN_BAR_HEIGHT = 2`; `DEFAULT_CHART_MIN_HEIGHT = 200`, `SMALL_CHART_HEIGHT = 125`, spark chart min height
  40; legends margin 16 above and below; `Y_AXIS_CHART_SPACING = 20`; faded (non-hovered) series opacity 0.3;
  `BAR_SPACING = 0.5`; horizontal bar label height 12 with offset 10 —
  [polaris-viz-core constants.ts](https://github.com/Shopify/polaris-viz/blob/main/packages/polaris-viz-core/src/constants.ts)

**IBM Carbon**
- Categorical palette order (light theme, 14 colours): Purple 70 `#6929c4`, Cyan 50 `#1192e8`, Teal 70
  `#005d5d`, Magenta 70 `#9f1853`, Red 50 `#fa4d56`, Red 90 `#570408`, Green 60 `#198038`, Blue 80 `#002d9c`,
  Magenta 50 `#ee538b`, Yellow 50 `#b28600`, Teal 50 `#009d9a`, Cyan 90 `#012749`, Orange 70 `#8a3800`,
  Purple 50 `#a56eff`; the sequence is ordered to maximise contrast between neighbours; one-colour options are
  Purple 70, Blue 80, Cyan 50 or Teal 70 — [Carbon colour palettes](https://carbondesignsystem.com/data-visualization/color-palettes/)
- Sequential (monochromatic) blue: 10 `#edf5ff`, 20 `#d0e2ff`, 30 `#a6c8ff`, 40 `#78a9ff`, 50 `#4589ff`, 60
  `#0f62fe`, 70 `#0043ce`, 80 `#002d9c`, 90 `#001d6c`, 100 `#001141`. In light themes the darkest step is the
  largest value; in dark themes the lightest step is the largest value —
  [Carbon colour palettes](https://carbondesignsystem.com/data-visualization/color-palettes/)
- Diverging palettes (same in light and dark): red-cyan for hot/cold data (Red 80 `#750e13` ... Red 10
  `#fff1f1` | Cyan 10 `#e5f6ff` ... Cyan 80 `#003a6d`) and purple-teal for data with no temperature meaning
  (Purple 80 `#491d8b` ... Purple 10 `#f6f2ff` | Teal 10 `#d9fbfb` ... Teal 80 `#004144`) —
  [Carbon colour palettes](https://carbondesignsystem.com/data-visualization/color-palettes/)
- Alert colours: Red 60 `#da1e28` (danger), Orange 40 `#ff832b` (serious warning), Yellow 30 `#f1c21b`
  (warning), Green 60 `#198038` (success). Gradients must never stand in for a sequential palette —
  [Carbon colour palettes](https://carbondesignsystem.com/data-visualization/color-palettes/)
- "Always start numerical axes at zero for part-to-whole and comparisons charts, such as bar and area chart";
  line charts and scatter plots may start elsewhere. Never interpolate across a gap in the data. When data
  crosses into a new day, month or year, that axis label is set semibold. An axis break is drawn as a wavy
  line, 16 px minimum — [Carbon axes and labels](https://carbondesignsystem.com/data-visualization/axes-and-labels/)
- Carbon Charts' built-in constants: default number of axis ticks 7 (labels rotate when a tick has under 30 px);
  area fill opacity 0.4 selected and 0.05 unselected; line opacity 1 selected and 0.3 unselected, weight 2
  selected and 1 unselected (key names read from a filtered listing); pie `innerRadius: 2`, `padAngle: 0.007`,
  slices under 5 degrees get a callout label; radar 4 Y ticks, dot radius 5; legend items 12 apart
  horizontally and 24 vertically, checkbox radius 6.5; meter bar height 8 (16 for proportional); chart toolbar
  buttons 32 with 20 icons; tooltip offset 4; histogram 10 bins —
  [configuration-non-customizable.ts](https://github.com/carbon-design-system/carbon-charts/blob/master/packages/core/src/configuration-non-customizable.ts)

**Atlassian chart colour tokens** (hex read by script from the published `@atlaskit/tokens` package, latest on
unpkg; the docs page itself did not render for the fetcher)
- Light: `color.chart.categorical.1` `#1868DB` (blue), `.2` `#6A9A23` (lime), `.3` `#AF59E1` (purple), `.4`
  `#E06C00` (orange), `.5` `#123263` (dark blue), `.6` `#803FA5` (dark purple), `.7` `#2898BD` (teal), `.8`
  `#7A3B00` (dark orange); `color.chart.neutral` `#7D818A`, `.brand` `#1868DB`, `.danger` `#C9372C`,
  `.warning` `#E06C00`, `.success` `#6A9A23`, `.discovery` `#AF59E1`, `.information` `#1868DB` —
  [@atlaskit/tokens atlassian-light](https://unpkg.com/@atlaskit/tokens/dist/esm/artifacts/tokens-raw/atlassian-light.js)
- Dark: `.1` `#669DF1`, `.2` `#B3DF72`, `.3` `#D8A0F7`, `.4` `#FBC828`, `.5` `#1868DB`, `.6` `#AF59E1`, `.7`
  `#6CC3E0`, `.8` `#FBD779`; neutral `#96999E`, danger `#F15B50`, warning `#FCA700`, success `#94C748` —
  [@atlaskit/tokens atlassian-dark](https://unpkg.com/@atlaskit/tokens/dist/esm/artifacts/tokens-raw/atlassian-dark.js)
- The categorical tokens are meant to be applied in numbered order, eight exist, and the page asks to keep a
  chart to five or six colours (from a search summary of the page, not read directly) —
  [Atlassian data visualisation colour](https://atlassian.design/foundations/color-new/data-visualization-color)

**Apple HIG, Charts** (read from the page's JSON data)
- Bar marks suit comparing categories and parts of a whole, and time series where each value is a sum (steps
  per day); line marks show change over time and the slope shows the size of change; point marks show
  individual values, outliers and clusters; points may be added on a line to stress single values —
  [Apple HIG Charts](https://developer.apple.com/design/human-interface-guidelines/charts)
- Bar charts work well with zero as the lower bound; a zero bound can hide meaningful differences elsewhere
  (heart rate). Use familiar tick sequences ("0, 5, 10", not "1, 6, 11"). Too many grid lines overwhelm, too
  few make values hard to estimate; with interactive inspection use fewer grid lines and light label colours —
  [Apple HIG Charts](https://developer.apple.com/design/human-interface-guidelines/charts)
- Give the chart a title and subtitle that state the main message (Weather: the expected precipitation for the
  next hour). The data is the most prominent element; descriptions and axes must not compete with it. In a
  compact space keep Y labels as short as possible and put units in the title. Align the chart's leading edge
  with surrounding views; Y axis and its labels may sit on the trailing side so the leading edge stays clean.
  Do not rely on colour alone (add shapes or patterns) and add visual separation between touching areas of
  colour — [Apple HIG Charts](https://developer.apple.com/design/human-interface-guidelines/charts)

**Datawrapper**
- Line charts are for values that develop over time; use bars for differences between categories. Use grey for
  comparison/context lines and colour, width or dashes only for the important ones. Label lines directly in
  the line's own colour instead of a legend. Lines need not start at zero, but include the zero baseline when
  values come close to it, and extend to 100% for shares near 100%. Point symbols are usually unnecessary with
  evenly spaced dates. Smoothed curves can distort the data. Annotate, and mark a highlight range where values
  turn into projections — [Datawrapper: line charts](https://www.datawrapper.de/blog/line-charts)
- Palette building: start with 3 to 10 distinct colours; give them different lightness so they survive
  greyscale; check them as thin lines (3 to 10 px) and as large areas; define at least one grey for context,
  missing and less important data, kept low in saturation; non-data greys are tiered: gridlines lightest, axis
  labels darker, titles darkest; sequential scales should span a wide lightness range; a diverging scale is
  two sequential scales joined at their lightest ends; use more saturated colours for small marks and less
  saturated for large areas; about 8% of men are colour-blind —
  [Datawrapper: colours for data vis style guides](https://www.datawrapper.de/blog/colors-for-data-vis-style-guides)

**WCAG 1.4.11 Non-text Contrast**
- Graphical objects "required to understand the content" need "a contrast ratio of at least 3:1 against
  adjacent color(s)"; 2.999:1 fails. Pie chart: slices must contrast with each other and the background unless
  visible labels and values carry the same information, in which case the criterion does not apply; a darker
  border around a light slice is an accepted fix. Line chart: each line needs 3:1 against the background but
  lines need not contrast with each other or with gridlines. Heat maps fall under the "essential" exception —
  [W3C Understanding 1.4.11](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html)

**KPI / stat tile with delta (shadcn dashboard-01 `section-cards`)**
- A stat card is: description line (`CardDescription`, 14 px muted, e.g. "Total Revenue"); the value as
  `CardTitle` in `text-2xl font-semibold tabular-nums` (24 px, 30 px once the card is at least 250 wide); an
  outline badge in the header's action corner holding a trend icon and the delta ("+12.5%"); a footer at 14 px
  with `gap-1.5`: a `font-medium` line with a 16 px trend icon, then a muted line. Cards carry a faint upward
  gradient from `primary` at 5% to the card colour and the faintest shadow —
  [section-cards.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/dashboard-01/components/section-cards.tsx)

### Inferences
- Approximate hex for the neutral tokens (they are Tailwind's neutral 300, 500, 600, 700, 800 lightnesses):
  chart-1 `#d4d4d4`, chart-2 `#737373`, chart-3 `#525252`, chart-4 `#404040`, chart-5 `#262626`. Blue theme:
  `#8ec5ff`, `#2b7fff`, `#155dfc`, `#1447e6`, `#193cb8` (Tailwind v4 blue 300/500/600/700/800). Conversions are
  mine, not from the source.
- Because the neutral chart tokens do not change with the mode, chart-1 (`#d4d4d4`) is about 1.5:1 on a white
  card and chart-5 (`#262626`) is close to invisible on the dark card `#171717`: both fail the 3:1 of WCAG
  1.4.11 unless values are labelled. A Pencraft charts skill should therefore not copy the tokens blindly:
  in light mode start a single series at chart-2 or darker; in dark mode start at chart-1 and stop at chart-3;
  or direct-label the values.
- A safe drawable default that agrees with shadcn, Tremor and Polaris Viz at once: plot height 200 to 320;
  horizontal gridlines only, 1 px solid, border colour at 50% (shadcn) or `gray-200` (Tremor), 4 to 5 lines
  ("0, 5, 10"-style round steps per Apple; Carbon's code default of 7 ticks is the upper end); no axis line, no
  tick marks; tick labels 12 px (11 in Polaris) muted, 8 to 10 px from the plot; line 2 px; hover dot radius
  5 to 6; bar corner radius 4 (3 Polaris, 8 only for a single wide series); stacked segments separated by 2 px
  (Polaris) with only the outer corners rounded; area fill at 0.4 or a gradient from 0.8/0.4 down to 0.1/0;
  donut ring = 25% of the radius (Tremor) with a 2 to 5 px gap in the background colour between slices; centre
  figure 30 px bold with a 12 px caption 24 px below; sparkline 112 x 48 with a 2 px line and no axes; legend
  swatch 8 x 8 radius 2 (bars/areas) or a 14 x 3 dash (lines), 12 px labels, 16 px between items.
- Bar width to gap: the shadcn examples do not set `barCategoryGap`, so Recharts' default applies; from memory
  that default is a 10% gap on each side of a category (bars about 80% of the band) and 4 px between bars of a
  group (unverified this session, the grep of Recharts' source returned nothing). Polaris Viz's
  `BAR_SPACING = 0.5` suggests a 2:1 bar-to-gap ratio there, but how the constant is applied was not read.
- Chart choice, combining Apple, Carbon and Datawrapper: bar for comparing categories and for per-period sums;
  grouped bar for 2 to 3 series per category; stacked bar for parts of a total over categories; line for change
  over time (2 to 5 coloured lines, the rest grey); area for one cumulative total or a stacked part-to-whole
  over time (zero baseline required); sparkline for trend without values inside a tile or table row;
  donut for 2 to 5 parts of one whole with the total in the centre; scatter for the relation of two measures;
  heatmap for a value over two categorical axes using a sequential palette; radial/gauge for one value against
  a fixed maximum (battery, quota); funnel for sequential drop-off; KPI tile for one number with its change.
- Series limits that the sources support: shadcn has 5 tokens; Atlassian 8 tokens but advises 5 to 6; Tremor 9;
  Carbon 14; Datawrapper 3 to 10. A mock-up should stay at 5 or fewer and fold the remainder into "Other" in
  the neutral grey.
- Dark mode: Atlassian swaps every categorical colour for a lighter step (600/500 to 500/400); Carbon flips
  the direction of sequential scales (lightest = largest); Tremor keeps the same 500 steps and only changes
  the grid to `gray-800`; shadcn keeps the same values. So "lighten one or two steps in dark, keep the grid
  at the dark border colour" is the common practice, and keeping identical colours is also defensible.

### Gaps
- Financial Times chart guidance (Visual Vocabulary, FT chart style) was not fetched; nothing in these notes
  is sourced from the FT.
- Material 3 has no fetched data-visualisation guidance here (m3.material.io returned 404 to the fetcher), and
  it is not certain that M3 publishes chart specifications at all.
- Legacy shadcn chart colours (2025 "new-york", multi-hue) were not re-read at the source; `_legacy-themes.ts`
  contained no chart tokens. From memory (unverified): light `hsl(12 76% 61%)`, `hsl(173 58% 39%)`, `hsl(197
  37% 24%)`, `hsl(43 74% 66%)`, `hsl(27 87% 67%)` (about `#e76e50`, `#2a9d90`, `#274754`, `#e8c468`,
  `#f4a462`); dark `hsl(220 70% 50%)`, `hsl(160 60% 45%)`, `hsl(30 80% 55%)`, `hsl(280 65% 60%)`, `hsl(340 75%
  55%)`.
- Recharts' default bar gap, default dot radius and default tick count were not confirmed at source.
- Tremor's default line `strokeWidth` in LineChart/AreaChart (only the spark charts' 2 px was confirmed) and
  the main area gradient's starting opacity were not captured.
- No source was found giving exact scatter point sizes, heatmap cell sizes/gaps or funnel anatomy for product
  UI; Carbon's code only gives `heatmap.minCellDividerDimension: 16`.
- Number-formatting rules (K/M abbreviation, decimals, currency) were not found in any fetched page; the
  shadcn examples only show `toLocaleString()` for totals and 3-letter month ticks.
- Carbon's dark-theme categorical hex values were not separable from the page text.
- Colour-blind safe palette specifics (e.g. Okabe-Ito values) were not sourced.

## 2. Common chart mistakes that make a mock-up look fake, and realistic sample data

### Takeaway
The fetched sources give rules that double as a fake-detector (zero-based bars, no interpolation across gaps,
smoothed curves distort, familiar tick steps, a title that states the message), and shadcn's own sample data
shows what "realistic" looks like: irregular integers with visible dips, not a rising ramp. No source was found
that discusses fake-looking mock-up charts as such.

### Cited Findings
- Smoothed curves can distort the data; Datawrapper advises its plain "Curved" over "natural" or "cardinal"
  interpolation — [Datawrapper: line charts](https://www.datawrapper.de/blog/line-charts)
- Tremor draws straight segments (`type="linear"`) by default, while shadcn's examples use `type="natural"` and
  Polaris Viz splines by default — [LineChart.tsx](https://github.com/tremorlabs/tremor/blob/main/src/components/LineChart/LineChart.tsx),
  [chart-line-dots.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-line-dots.tsx),
  [polaris-viz constants](https://github.com/Shopify/polaris-viz/blob/main/packages/polaris-viz-core/src/constants.ts)
- Bars and areas must start at zero; never interpolate across periods with no data, and label both ends of
  such a gap — [Carbon axes and labels](https://carbondesignsystem.com/data-visualization/axes-and-labels/)
- Tick labels should follow familiar sequences (0, 5, 10); units can go in the title when the axis is tight; a
  chart needs a title/subtitle summarising its message — [Apple HIG Charts](https://developer.apple.com/design/human-interface-guidelines/charts)
- shadcn's six-month sample series: desktop 186, 305, 237, 73, 209, 214; mobile 80, 200, 120, 190, 130, 140
  (January to June). It rises and collapses (305 to 73) instead of trending smoothly —
  [chart-bar-multiple.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-bar-multiple.tsx)
- shadcn's donut sample: Chrome 275, Safari 200, Firefox 287, Edge 173, Other 190, and the centre total is
  computed from the same rows (so it reads 1,125) — [chart-pie-donut-text.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-pie-donut-text.tsx)
- shadcn's 91-day sample (2024-04-01 to 2024-06-30) uses daily values between about 59 and 530 with large
  day-to-day swings, and the card offers 90 / 30 / 7 day ranges that filter the same rows —
  [chart-area-interactive.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-area-interactive.tsx)
- A chart card's caption quotes a figure tied to the data ("Trending up by 5.2% this month", "January - June
  2024") — [chart-bar-default.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/charts/chart-bar-default.tsx)

### Inferences
- A checklist a model can apply, each item derived from the findings above: (1) every number shown twice must
  agree: donut centre = sum of slices, stacked bar total = sum of segments, KPI delta = (current - previous) /
  previous for the two values the sparkline ends on, percentages sum to 100; (2) bar heights are computed from
  the values and a zero baseline, not eyeballed; (3) axis ticks are round steps that bracket the data and
  carry a unit or currency somewhere (axis, title or tooltip); (4) time axes use real, evenly spaced dates
  with 3-letter months and a stated range; (5) lines are straight segments or a mild curve that passes
  through every point and never dips below the baseline or overshoots between points; (6) data has a
  believable shape: noise, one or two dips, weekday/weekend rhythm for daily data, not a monotonic ramp or a
  perfect sine; (7) values are not round (186, not 200) and not all the same magnitude; (8) one hover state
  at most (a cursor line + tooltip whose values match the point under it); (9) no chart smaller than its
  labels can support: if there is no room for axes, draw a sparkline, not a miniature chart with axes.
- Realistic sample data recipe: pick the end value from the KPI tile, walk backwards with a small drift
  (0.5 to 2% per step) plus noise of about 10 to 25% of the value, add one outlier, then round to integers
  (or to cents for money). For two series keep a stable ratio with independent noise (shadcn's mobile series
  runs at roughly 45 to 90% of desktop). This recipe is mine; no fetched source prescribes it.

### Gaps
- No authoritative article on "AI-generated / fake-looking charts in mock-ups" was located; this section rests
  on general chart rules plus inference.
- No source on how design teams produce sample data (tools, conventions) was fetched.

## 3. Dashboard and app-shell layouts: dimensions and structure

### Takeaway
shadcn's sidebar is 256 expanded, 48 as an icon rail and 288 on mobile; its dashboard block overrides that to
288 with a 48 high header, 24 px page gutters and a 4-column KPI row with 16 px gaps. Carbon uses a 48 header
with a 256 / 48 side nav, Polaris a 240 navigation with a 662 primary column, Material a 64 top bar with an
80 rail or 360 drawer.

### Cited Findings

**shadcn sidebar component**
- `SIDEBAR_WIDTH = "16rem"` (256), `SIDEBAR_WIDTH_MOBILE = "18rem"` (288), `SIDEBAR_WIDTH_ICON = "3rem"` (48),
  keyboard shortcut "b"; variants `sidebar | floating | inset`; floating and inset add `p-2` (8) around the
  panel so the collapsed width becomes 48 + 16 (+2 for the border); the plain variant has a 1 px border on its
  inner edge; the resize rail is a 16 wide hit area showing a 2 px line on hover —
  [sidebar.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/sidebar.tsx)
- Inset variant: the content panel gets `m-2` (8, none on the sidebar side), `rounded-xl` (14) and the faintest
  shadow, on a page coloured like the sidebar; floating variant: the sidebar itself is `rounded-lg` (10) with
  a 1 px ring and faint shadow — [SHADCN-VEGA](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-vega.css)
- Anatomy and sizes (Vega and Nova agree on all of these): header and footer `p-2 gap-2`; content `gap-2`;
  each group `p-2`; group label `h-8 px-2 text-xs font-medium` at 70% of the sidebar text colour, radius 8;
  menu `gap-1` (4) between rows; menu button `h-8 p-2 gap-2 text-sm rounded-md` (32 high, 8 padding, 8 between
  the 16 icon and the label, radius 8), hover and active fill `sidebar-accent`, active label `font-medium`;
  sizes sm `h-7 text-xs` and lg `h-12` (the workspace / user row); collapsed: the button becomes 32 x 32 inside
  the 48 rail; sub-menu `mx-3.5 border-l px-2.5 py-0.5 gap-1` (a 1 px guide line 14 in, children indented 10
  more) with sub-buttons `h-7 px-2`; menu badge `h-5 min-w-5 px-1 text-xs font-medium` at the right, 4 from
  the edge; row action button 20 square, 4 from the right, 6 from the top; search input `h-8`; separator has
  `mx-2` — [SHADCN-VEGA](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-vega.css),
  [SHADCN-NOVA](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-nova.css)
- Sidebar colours (shadcn site, neutral): light `--sidebar oklch(0.985 0 0)` (about `#fafafa`), accent
  `oklch(0.97 0 0)` (`#f5f5f5`), border `oklch(0.922 0 0)` (`#e5e5e5`), primary `oklch(0.205 0 0)`; dark
  `--sidebar oklch(0.205 0 0)` (`#171717`), accent `oklch(0.269 0 0)` (`#262626`), border white at 10% —
  [globals.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/app/globals.css)

**shadcn dashboard-01 block**
- Page sets `--sidebar-width: calc(var(--spacing) * 72)` (288) and `--header-height: calc(var(--spacing) *
  12)` (48); main column is `flex flex-col gap-4 py-4 md:gap-6 md:py-6` (sections 24 apart, 24 top and bottom
  on desktop; 16 on small) with every section padded `px-4 lg:px-6` (24 gutters from 1024 up) —
  [dashboard-01 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/dashboard-01/page.tsx)
- Header: `h-(--header-height)` with a bottom border; inside `px-4 lg:px-6`, `gap-1 lg:gap-2`: sidebar toggle
  (pulled 4 left), a vertical separator 16 high with 8 margin each side, then the page title `text-base
  font-medium` (16/500) — [site-header.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/dashboard-01/components/site-header.tsx)
- KPI row: `grid grid-cols-1 gap-4`, 2 columns from a 576 wide container (`@xl/main`), 4 columns from 1024
  (`@5xl/main`); four cards ("Total Revenue", "New Customers", ...) —
  [section-cards.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/dashboard-01/components/section-cards.tsx)
- Order of the page: KPI row, one full-width chart card (250 high plot), then a tabbed data table —
  [dashboard-01 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/dashboard-01/page.tsx)
- Table toolbar row: `flex items-center justify-between px-4 lg:px-6` with the view tabs left and buttons
  ("Customize Columns", "Add Section") right, gap 8; the table sits in `overflow-hidden rounded-lg border`
  with a `sticky top-0 bg-muted` header; first column 32 wide for the drag handle/checkbox —
  [data-table.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/dashboard-01/components/data-table.tsx)

**Other shadcn sidebar blocks**
- Header heights: 64 (`h-16`) in sidebar-07 and sidebar-08, shrinking to 48 when the sidebar is collapsed to
  icons (sidebar-07); 56 (`h-14`) in sidebar-10 and sidebar-15; header content `px-4` (or `px-3`), `gap-2`:
  toggle, 16 high vertical separator, breadcrumb — [sidebar-07 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/sidebar-07/page.tsx),
  [sidebar-10 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/sidebar-10/page.tsx)
- Generic content: `flex flex-col gap-4 p-4 pt-0`, a 3-column grid with `gap-4` of 16:9 panels with radius 14,
  then one tall panel — [sidebar-08 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/sidebar-08/page.tsx)
- Document-style content: centred column `max-w-3xl` (768) with `px-4 py-10` —
  [sidebar-10 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/sidebar-10/page.tsx)
- Workspace switcher / brand row: a 32 square logo tile with radius 10 in `sidebar-primary` holding a 16 icon,
  then two lines (`font-medium` 14 name, 12 plan), inside an `lg` (48 high) menu button —
  [sidebar-09 app-sidebar.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/sidebar-09/components/app-sidebar.tsx)

**Carbon UI shell**
- Header height 48; menu trigger and header action buttons 48 x 48; product name 14 px semibold with 16 left
  and 32 right padding; header links 14 px regular with 16 side padding; header has a 1 px bottom border in
  `$border-subtle`; a selected link has a bottom border in `$border-interactive` —
  [Carbon UI shell header style](https://carbondesignsystem.com/components/UI-shell-header/style/)
- Side nav: collapsed rail `mini-units(6)`, expanded `mini-units(32)`, starts 48 below the top; items
  `block-size: mini-units(4)`; sub-items indented `mini-units(4)` (with icon `mini-units(9)`); icons 16; the
  active item has a 3 px bar on its leading edge; panel top padding 16 —
  [_side-nav.scss](https://github.com/carbon-design-system/carbon/blob/main/packages/styles/scss/components/ui-shell/side-nav/_side-nav.scss)

**Shopify Polaris (admin frame, CSS variables in source)**
- `--pg-navigation-width: 240px`; primary column 480 min to 662 max (`--pg-layout-width-primary-min/max`);
  secondary column 240 min to 320 max; one-half column base 450; one-third base 240; inner spacing 16; outer
  spacing min 20; the frame's contextual sidebar is 356 + 16 —
  [Polaris AppProvider global.css](https://github.com/Shopify/polaris/blob/main/polaris-react/src/components/AppProvider/global.css),
  [Frame.module.css](https://github.com/Shopify/polaris/blob/main/polaris-react/src/components/Frame/Frame.module.css)

**Material 3 (token files of material-web, v0_192; the m3.material.io pages did not load)**
- Navigation rail container 80 wide, active indicator 56 x 32 pill, icons 24; navigation drawer 360 wide with a
  336 x 56 pill indicator; small top app bar 64 high with 24 icons; bottom navigation bar 80 high with a
  64 x 32 indicator — [material-web tokens](https://github.com/material-components/material-web/tree/main/tokens/versions/v0_192)
- Window size classes: compact under 600, medium 600 to 839, expanded 840 to 1199, large 1200 to 1599,
  extra-large 1600 and up (dp); height classes compact under 480, medium 480 to 899, expanded 900 and up —
  [Android window size classes](https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes)

**Material date/segmented tokens that affect shell toolbars**
- Outlined segmented button 40 high, fully rounded, 1 px outline, 18 px icon —
  [material-web tokens](https://github.com/material-components/material-web/tree/main/tokens/versions/v0_192)

### Inferences
- Carbon's `mini-units(n)` is n x 8 px in Carbon's convention (not re-read): rail 48, expanded side nav 256,
  nav item 32 high, sub-item indent 32 (72 with icon). That makes shadcn (256 / 48, 32 px rows) and Carbon
  (256 / 48, 32 px rows) agree exactly; Polaris is 240 and M3's drawer 360.
- Drawable desktop shell for a 1440 x 900 frame in the shadcn idiom: sidebar 256 (or 288 for a dashboard), top
  bar 48 (compact) / 56 / 64, content gutters 24, section gap 24, card gap 16; KPI row of 4 cards across
  (each (1440 - 256 - 48 - 48) / 4 = 272 wide at 256 sidebar); then a full-width chart card with a 250 high
  plot; then a table card. Tablet (below 1024): 2 KPI columns, gutters 16. Mobile: sidebar becomes a 288 sheet.
- Breakpoints a model can cite: Tailwind's 640 / 768 / 1024 / 1280 / 1536 (used by all shadcn blocks: `md`
  shows the sidebar, `lg` widens gutters) alongside Material's 600 / 840 / 1200 / 1600.
- Page header anatomy is not a shadcn component; the blocks only show "toggle | separator | title or
  breadcrumb" in the top bar, with actions pushed to the right (`ml-auto`). A title + description + actions
  header inside the content area (title 24/600, description 14 muted 4 below, actions right-aligned, 24 below
  it) is convention, not sourced.
- Filter bar, by analogy with the dashboard-01 table toolbar: one row, 32 or 36 high controls, 8 between
  them, search input left (about 240 to 320 wide), filter selects next, view options and the primary action
  right; 16 between the bar and the table.

### Gaps
- Linear- and Vercel-style shell measurements (sidebar widths, header heights) were not sourced; no public
  spec was fetched.
- Material 3 canonical layouts (list-detail, supporting pane, feed) pane widths, margins and spacers could not
  be read: m3.material.io did not serve the page and the search found no confirming source. The M3 Expressive
  (2025) rail/drawer changes were not checked; the token values above come from the material-web v0_192 token
  set, which may predate them.
- Polaris' top bar height and default page max width (998 px from memory) were not confirmed: the CSS refers to
  variables whose definitions were not in the files read.
- Carbon's 16-column 2x grid gutters and breakpoints, and the right panel / switcher widths, were not fetched.
- Content max widths for dashboards (most are fluid) are not specified by any source read; only shadcn's 768
  document column was found.

## 4. Components missing from the current skill: anatomy and measurements

### Takeaway
Nearly every missing component exists in shadcn's 2026 registry, with exact classes per style, including a new
chat/AI set (message, bubble, attachment, marker, trace, questionnaire, code-block). Vega values are given
first and Nova's differences in brackets; stepper, timeline, rating, tree view, dropzone and banner are not
shadcn components and come from Ant Design, Radix Themes or Material tokens, or are gaps.

### Cited Findings
All Vega values: [SHADCN-VEGA](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-vega.css);
Nova values: [SHADCN-NOVA](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-nova.css).
The component list (63 files, including `attachment`, `bubble`, `code-block`, `marker`, `message`,
`message-scroller`, `questionnaire`, `trace`, `input-group`, `item`, `button-group`, `native-select`, `spinner`,
`empty`, `field`) is at [bases/base/ui](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/ui).

**Calendar / date picker**
- Vega: padding 12, day cell 32 x 32, cell radius 8. Nova: padding 8, cell 28 — SHADCN-VEGA, SHADCN-NOVA
- Structure: months side by side with `gap-4` (16) (stacked on small screens); inside a month `gap-4` between
  caption and grid; caption row is one cell high, centred, `text-sm font-medium`; previous/next buttons are
  ghost icon buttons one cell in size at the two ends of the caption row with 16 px chevrons; weekday header
  `text-[0.8rem]` (12.8 px) regular in `muted-foreground`; week rows `mt-2` (8) apart; cells are square and
  share the width equally — [calendar.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/calendar.tsx)
- States: today = `bg-muted` with `text-foreground`, radius 8; single selected day = `bg-primary
  text-primary-foreground`; range start and end = rounded only on the outer side with a `bg-muted` band behind
  them; range middle = square corners; days outside the month = `text-muted-foreground`; disabled = muted at
  50% — [calendar.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/calendar.tsx)
- Month/year dropdown caption: `h-8 pl-2 pr-1` bordered with faint shadow (Nova `h-6 pl-1.5`), 14.5 px chevron
  (`size-3.5`) in muted — SHADCN-VEGA, [calendar.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/calendar.tsx)
- Material 3 modal date picker for comparison: container 360 x 568 with extra-large corners, header 120 high,
  day cells 40 x 40 circles — [material-web tokens](https://github.com/material-components/material-web/tree/main/tokens/versions/v0_192)

**Combobox, multi-select with chips (tag input)**
- Popup: `min-w-36` (144), `max-h-72` (288), radius 8 [Nova 10], 1 px ring of foreground at 10%, medium shadow;
  an optional search input group inside: 32 high, `m-1` with no bottom margin, tinted `input/30`, no shadow;
  list padding 4 — SHADCN-VEGA
- Item: `py-1.5 pl-2 pr-8 gap-2 text-sm rounded-sm` (6 above/below, 8 left, 32 right for the check; radius 6)
  [Nova `py-1 pl-1.5`, radius 8]; highlighted = `bg-accent`; check indicator 16 at `right-2`; group label
  `px-2 py-1.5 text-xs` muted; separator 1 px with 4 above/below, bleeding 4 into the padding; empty text 14
  muted centred with `py-2` — SHADCN-VEGA
- Chips field (multi-select / tag input): `min-h-9` (36) [Nova 32], `px-2.5 py-1.5` [Nova `py-1`], `gap-1.5`
  [Nova 4], radius 8, input border; when it contains chips the side padding drops to 6 [Nova 4]. Chip: height
  22 [Nova 21], `px-1.5`, `gap-1`, `rounded-sm` (6), `bg-muted`, `text-xs font-medium`; with a remove button the
  right padding is 0 and the "x" sits at 50% opacity — SHADCN-VEGA, SHADCN-NOVA

**Pagination**
- A centred row with `gap-1` (4) [Nova 2]; page links are icon-size buttons (36 [Nova 32]): the current page
  uses the outline variant, others ghost; Previous / Next are default-size buttons with a chevron and the word
  (word hidden on small screens), inner side padding reduced to 8; ellipsis is a 36 [Nova 32] square holding a
  16 px "more" icon — SHADCN-VEGA, [pagination.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/pagination.tsx)
- Ant Design: item size = control height 32 (24 small, 40 large); quick-jump input 50 wide —
  [antd pagination tokens](https://github.com/ant-design/ant-design/blob/master/components/pagination/style/index.ts)

**Accordion / collapsible**
- Item has a bottom border except the last; trigger `py-4 text-sm font-medium` (16 above/below), text left, a
  16 px chevron in `muted-foreground` at the far right, underline on hover; content `text-sm` with `pb-4`
  [Nova trigger `py-2.5`, content `pb-2.5`, radius 10] — SHADCN-VEGA, SHADCN-NOVA

**Segmented control / toggle group**
- shadcn Toggle: sizes sm 32, default 36, lg 40 high with the same min width [Nova 28 / 32 / 36], `px-2.5`,
  `gap-1`, radius 8, 14/500, 16 px icons; pressed = `bg-muted`; outline variant has the input border and faint
  shadow. Toggle group with spacing 0: items share borders, only the outer ends are rounded (8), inner padding
  8 — SHADCN-VEGA, SHADCN-NOVA
- shadcn Tabs list as a segmented control [Nova]: list radius 10 with 3 px padding, trigger `px-1.5 py-0.5`
  radius 8, 14/500 — SHADCN-NOVA
- Radix Themes SegmentedControl: size 1 height `--space-5` (24) with font size 1 (12), item padding
  `--space-3` (12) and gap 4; size 2 height `--space-6` (32), font size 2 (14), padding `--space-4` (16);
  the selected indicator is inset 1 px with radius one less than the track; 1 px separators between unselected
  items; items are equal width; selected label is medium weight, others regular —
  [segmented-control.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/segmented-control.css)
- Ant Design Segmented: track padding 2 (`lineWidthBold`), track `colorBgLayout`, selected item
  `colorBgElevated` — [antd segmented tokens](https://github.com/ant-design/ant-design/blob/master/components/segmented/style/index.ts)
- Material 3 outlined segmented button: 40 high, pill, 1 px outline, 18 px icon —
  [material-web tokens](https://github.com/material-components/material-web/tree/main/tokens/versions/v0_192)

**Button group, input group (prefix/suffix, number stepper, prompt bar)**
- Button group: buttons joined, only the outer corners rounded (8); a text segment is `bg-muted border px-2.5
  text-sm font-medium`; the separator between joined buttons uses the `input` colour; nested groups sit 8
  apart — SHADCN-VEGA
- Input group: 36 high [Nova 32], radius 8 [Nova 10], input border; inline addon (icon, text, kbd, button):
  `muted-foreground`, 14/500, 16 px icons, 8 from the edge, and the input's padding on that side drops to 6;
  buttons inside: xs 24 high with `px-1.5` and 14 px icons, icon-xs 24 square, icon-sm 32 square, radius
  (base - 5) = 5 — SHADCN-VEGA
- Block addon (a row above or below the field): the group becomes a column with automatic height; addon
  `px-2.5` and 8 toward the outer edge; the text area inside has `py-2` and no border of its own. This is the
  prompt-bar construction: text area on top, a tool row below — SHADCN-VEGA

**OTP input**
- Slots are joined squares 36 x 36 [Nova 32], 14 px text, input border on top/bottom/right with the first
  slot adding the left border and 8 radius on the group's outer corners [Nova 10]; the active slot gets the
  ring colour border plus a 3 px halo; caret is a 1 x 16 line in the foreground colour; groups are 8 apart and
  the separator is a 16 px dash/minus icon — SHADCN-VEGA, [input-otp.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/input-otp.tsx)

**Textarea**
- `px-2.5 py-2`, radius 8 [Nova 10], input border, 14 px text on desktop (16 on mobile), faint shadow [none in
  Nova] — SHADCN-VEGA, SHADCN-NOVA

**Context menu, hover card, sheet, navigation menu**
- Context menu: same as the dropdown menu: `min-w-36`, padding 4, radius 8 [Nova 10], hairline ring 10%,
  medium shadow; item `px-2 py-1.5 gap-2 text-sm rounded-sm` [Nova `px-1.5 py-1 gap-1.5`, radius 8]; inset
  items `pl-8` [Nova 28]; checkbox/radio items `pr-8 pl-2` with the mark at `right-2`; label `px-2 py-1.5
  text-xs font-medium` muted; shortcut `text-xs tracking-widest` muted, right-aligned; sub-menu `min-w-32`
  with a larger shadow — SHADCN-VEGA, SHADCN-NOVA
- Hover card: 256 wide (`w-64`), padding 16 [Nova 10], radius 10, hairline ring, medium shadow, 14 px text —
  SHADCN-VEGA, SHADCN-NOVA
- Sheet (side panel): `bg-popover`, large shadow, 1 px border on the inner edge, width 75% of the screen up
  to `sm:max-w-sm` (384) for left/right; overlay black at 10%; header `p-4 gap-1.5` [Nova gap 2], title
  `font-medium`, description 14 muted; footer `p-4 gap-2`; close button `top-4 right-4`; sections 16 apart —
  SHADCN-VEGA
- Navigation menu (top nav with mega-menu): trigger `px-4 py-2 text-sm font-medium rounded-md` [Nova `px-2.5
  py-1.5`, radius 10] with a 12 px chevron 4 to its right, hover `bg-muted`, open `bg-muted/50`; no gap
  between triggers; panel radius 10 with hairline ring and shadow, content padding 8 (10 on the right); links
  in the panel `p-2 gap-1.5 text-sm rounded-sm` — SHADCN-VEGA, SHADCN-NOVA

**Item (list/master row), table, data-table features**
- Item (the 2026 list-row primitive): default `px-4 py-3.5 gap-3.5` [Nova `px-3 py-2.5 gap-2.5`]; sm `px-3
  py-2.5 gap-2.5`; xs `px-2.5 py-2 gap-2`; radius 8; variants default (no border), outline (`border`), muted
  (`bg-muted/50`); media: icon 16, or image 40 square radius 6 (sm 32, xs 24); title `text-sm font-medium
  leading-snug`; description `text-sm` muted (xs: 12); content gap 4; actions gap 8; groups 16 apart (sm 10,
  xs 8) — SHADCN-VEGA, SHADCN-NOVA
- Table (Vega and Nova equal): header cell `h-10 px-2 font-medium text-foreground`, 14 px; body cell `p-2`
  (so rows are about 37 high with one line of 14 px text); rows have a bottom border, hover `bg-muted/50`,
  selected `bg-muted`; footer `bg-muted/50 border-t font-medium`; a cell holding a checkbox has no right
  padding; caption 14 muted, 16 above — SHADCN-VEGA
- Data-table footer (dashboard-01): left, muted 14 px "N of M row(s) selected."; right group with `gap-8`
  (32): "Rows per page" label 14/500 + a small select 80 wide (gap 8); "Page 1 of 7" 14/500; four 32 square
  outline icon buttons (first, previous, next, last) with gap 8; footer padded `px-4`; header is `sticky top-0
  bg-muted`; the table has a 1 px border and radius 10; a row-action "more" button is a 32 square ghost
  button; drag handle 28 square — [data-table.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/dashboard-01/components/data-table.tsx)

**Carousel**
- Previous/next buttons are fully round icon buttons (shadcn); Ant Design: dots 16 x 3 with a 4 gap, 12 from
  the bottom edge, arrows 16 — SHADCN-VEGA, [antd carousel tokens](https://github.com/ant-design/ant-design/blob/master/components/carousel/style/index.ts)

**Stepper / wizard, timeline, rating, upload (Ant Design component tokens)**
- Steps: `iconSize = controlHeight`, `iconSizeSM = fontSizeHeading3`, `iconFontSize = fontSize`,
  `titleLineHeight = controlHeight`, `dotSize = controlHeight / 4`, `dotCurrentSize = controlHeightLG / 4`;
  waiting step icon uses a neutral fill with label-colour text —
  [antd steps tokens](https://github.com/ant-design/ant-design/blob/master/components/steps/style/index.ts)
- Timeline: `itemHeadSize: 10`, `tailWidth = lineWidthBold`, `dotBorderWidth = lineWidthBold`,
  `itemPaddingBottom = padding * 1.25`, tail colour `colorSplit` —
  [antd timeline tokens](https://github.com/ant-design/ant-design/blob/master/components/timeline/style/index.ts)
- Rate: `starSize = controlHeight * 0.625` (small and large use the SM and LG control heights), `starColor =
  yellow6`, empty star `colorFillContent`, hover scale 1.1 —
  [antd rate tokens](https://github.com/ant-design/ant-design/blob/master/components/rate/style/index.ts)
- Upload: `pictureCardSize = controlHeightLG * 2.55`, thumbnail = `fontSizeHeading3 * 2` —
  [antd upload tokens](https://github.com/ant-design/ant-design/blob/master/components/upload/style/index.ts)

**Chat / AI message UI (shadcn 2026 registry)**
- Message scroller: messages `gap-8` (32) apart [Nova 24] — SHADCN-VEGA, SHADCN-NOVA
- Message: row with `gap-2`, 14 px; avatar column min 32 wide; content column `gap-2.5`; header and footer
  (name, time, actions) `text-xs font-medium` muted with `px-3` so they line up with the bubble's text (0 for
  ghost bubbles); `align=end` pushes the user's message to the right; grouped messages 8 apart — SHADCN-VEGA,
  [message.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/message.tsx)
- Bubble: max width 80% of the thread (ghost: full width); content `rounded-xl` (14) `px-3 py-2 text-sm
  leading-relaxed` (12 beside, 8 above/below, 14 px with 1.625 line height). Variants: default = `bg-primary`
  + `primary-foreground`; secondary; muted = `bg-muted`; tinted = the primary hue at lightness 0.93 and 40% of
  its chroma (dark: 0.30); outline = background + border; ghost = no fill, no padding, no radius (the usual
  assistant answer); destructive = destructive at 10% (dark 20%) with destructive text. Reactions: a pill
  overlapping the bubble's edge by three quarters of its height, 12 in from the side, `bg-muted px-1.5 py-0.5`
  with a 3 px ring in the card colour — SHADCN-VEGA, [bubble.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/bubble.tsx)
- Attachment (file chip): radius 14; horizontal form min 160 wide, `px-2.5 py-2 gap-2`, media tile 40 square
  radius 10 in `bg-muted` with a 16 icon (sm 32, xs 28 with radius 8 and 14 icon), title 14/500, description 12
  with 2 above; vertical (thumbnail) form 96 wide, or 120 with a caption, icon 24, actions at top-right 12 in;
  a group of attachments has gap 12; images are shown at 60% opacity while uploading — SHADCN-VEGA
- Marker (system line / date divider in a thread): 14 px muted, 16 icon, gap 8; separator variant = text
  centred between two 1 px lines; border variant = bottom border with 8 padding — SHADCN-VEGA
- Trace (tool calls / reasoning steps): steps 8 apart; each line `gap-2 text-sm` with a 16 icon; a collapsible
  step has a trigger with gap 4 and a chevron that turns 90 degrees; its content is indented under a 1 px left
  line: margin-left 7, padding-left 16, 8 above and 4 below, relaxed line height — SHADCN-VEGA,
  [trace.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/trace.tsx)
- Code block: radius 10, input-colour border, 14 px mono (sm 12); content `px-4 py-4` [Nova 12]; header `px-4
  pt-4` with a 14/500 sans title and 16 icon, actions (copy) at the right; without a header the copy button
  floats 12 from the top-right; numbered lines have a 24 wide right-aligned muted gutter and 12 gap;
  highlighted and added lines `bg-muted` (added shows "+"), removed lines `destructive/10` with a red "-" —
  SHADCN-VEGA, SHADCN-NOVA
- Questionnaire (the assistant asks the user to choose): block `gap-6`; progress text 12; title `text-base
  font-semibold` (16/600) with 20 below; choices 12 apart; a choice is a bordered row `px-4 py-3.5 gap-3
  rounded-md` [Nova `px-3 py-2.5`, radius 10] with a 16 px indicator (radius 4; checked = primary fill with a
  14 check or an 8 dot), a 20 square keyboard-shortcut hint (mono 10 px, border, radius 8), label 14 and muted
  description; checked row = `bg-muted` with the border at primary 40%; actions row min 36 high, gap 8 —
  SHADCN-VEGA, SHADCN-NOVA

### Inferences
- Ant Design's component tokens above resolve, with Ant's default seed tokens (control height 32, small 24,
  large 40, font size 14, heading-3 24, padding 16, bold line 2; defaults from memory, not re-read), to: step
  circle 32 (small 24) with a 14 px number, dot steps 8 (current 10); timeline dot 10 with a 2 px line and 20
  between items; rating star 20 (small 15, large 25) in yellow (`#fadb14` from memory); upload picture card
  102 square, list thumbnail 48.
- Radix SegmentedControl size 3 is presumably 40 high with 16 px text (pattern from sizes 1 and 2; the lines
  were cut off in the listing) (unverified).
- Number input / stepper: no shadcn component; build from the input group (36 high [32], radius 8) with two
  icon-xs buttons (24 square, minus and plus) as inline addons, or joined as a button group
  `[-] [ value ] [+]` with 36 square outline buttons at the ends. Base UI's NumberField is unstyled, so this
  is a composition, not a sourced spec.
- Tag input = the combobox chips field without the popup (chip 22 high, 12/500, radius 6, muted fill).
- File dropzone, in the kit's own tokens (not sourced): a dashed 1 px `input`-colour border, radius 10 to 14,
  padding 24 to 32, centred column: 40 square muted icon tile, 14/500 line ("Drag files here or browse"), 12
  muted line with the limits; uploaded files listed below as shadcn Attachments or Items with a progress bar.
- Stepper in shadcn terms (not sourced as a shadcn component): 24 to 32 circles (done = primary fill with a
  check, current = primary border or fill with the number, upcoming = muted fill with muted number), 14/500
  titles, 12 muted descriptions, connector 1 to 2 px in `border` (done part in primary), 8 between circle and
  text.
- Timeline / activity feed in shadcn terms: Marker and Trace give the vocabulary: 16 icon or a 10 dot on a
  1 px `border` line, entry text 14, timestamp 12 muted, 16 to 20 between entries.
- Bulk action bar and banner are not in the shadcn registry. A bulk bar is commonly a floating row (radius
  10, popover fill, hairline, floating shadow) at the bottom centre: "3 selected", a separator, ghost action
  buttons 32 high, a close icon (convention, unsourced). A banner is a full-width row above the top bar, 40 to
  48 high, 14 px text centred with one link/action and a dismiss icon at the right (convention, unsourced).
- Sort indicator and column filter were not read line by line in data-table.tsx; the long-standing shadcn
  pattern is a ghost header button with the label and a 16 px up/down arrow icon 8 to its right (unverified
  this session).
- Tree view: no shadcn component; the sidebar's sub-menu is the closest drawable pattern: rows 28 to 32 high,
  16 chevron + 16 icon + label with gap 8, children under a 1 px guide line indented about 14 + 10.

### Gaps
- React Aria / Spectrum, Base UI docs and Ant Design docs pages were not fetched; only Ant's token source was
  read, and its tree and input-number token blocks returned nothing usable (tree `titleHeight`, input-number
  handle width not captured).
- Tree view, file dropzone, banner, bulk action bar, stepper and timeline have no measured shadcn source; the
  values in Inferences are compositions.
- Date range picker presets column, time picker and the popover that holds the calendar were not measured
  (the calendar inside a popover simply drops its own background).
- Carousel slide gaps and indicator styling in shadcn were not read (only the round buttons).
- Polaris Banner and M3 banner specs were not fetched.
- The exact size of the icon-size button used for pagination links in each style was taken from the existing
  Pencraft notes (36 Vega / 32 Nova), not re-read here.
- shadcn's `sidebar-01..16` were sampled (07, 08, 09, 10, 15 read); the others were not opened.

## 5. Page compositions with measurements

### Takeaway
Only sign-in / sign-up and the inbox (master-detail) composition are measured in shadcn's blocks; settings and
detail pages can lean on Polaris' column widths. Pricing, checkout, onboarding, search results and error pages
have no measured source in this research and are given as clearly labelled conventions.

### Cited Findings

**Sign-in / sign-up (shadcn login-01..05, signup-01..05)**
- Centred card: page `flex min-h-svh items-center justify-center p-6 md:p-10` (24, 40 from 768); column
  `w-full max-w-sm` (384); form stack `gap-6` (24); the password label row holds a right-aligned "Forgot your
  password?" link (14 px, underline on hover); a centred 14 px muted line under the buttons ("Don't have an
  account? Sign up") — [login-01 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/login-01/page.tsx),
  [login-form.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/login-01/components/login-form.tsx)
- Split screen: `grid min-h-svh lg:grid-cols-2`; left half `p-6 md:p-10` with the brand at top-left (a 24
  square primary tile, radius 8, 16 icon, then the name in `font-medium`, gap 8) and the form centred in the
  remaining height at `max-w-xs` (320); right half `bg-muted` with a cover image (hidden below 1024) —
  [login-02 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/login-02/page.tsx)
- Muted page with brand above the card: page `bg-muted`, column `max-w-sm` with `gap-6`: centred brand row,
  then the card — [login-03 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/login-03/page.tsx)
- Wide two-column card: container `max-w-sm md:max-w-4xl` (896); the card has no padding and two equal
  columns: form `p-6 md:p-8` (32) and a muted image; heading `text-2xl font-bold` (24) centred with a muted
  line under it (gap 8); a text separator ("Or continue with"); three social buttons in `grid grid-cols-3
  gap-4`; legal line under the card, centred, `px-6` — [login-04 login-form.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/login-04/components/login-form.tsx),
  [login-04 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/login-04/page.tsx)
- Sign-up uses the identical frame (`p-6 md:p-10`, `max-w-sm`) —
  [signup-01 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/signup-01/page.tsx)

**Inbox / master-detail (shadcn sidebar-09)**
- `--sidebar-width: 350px` in total: a 48 (+1 border) icon rail and a list pane filling the rest (about 301);
  list header `p-4 gap-3.5` with a bottom border: title `text-base font-medium` left and an "Unreads" label
  with a switch right, then a search input; each mail row `p-4 gap-2 text-sm leading-tight` with a bottom
  border (none on the last): line 1 sender name with the date right-aligned in 12 px, line 2 subject
  `font-medium`, line 3 a two-line preview in 12 px limited to 260 wide; hover `sidebar-accent`; the detail
  pane header is sticky with `p-4` and a bottom border — [sidebar-09 app-sidebar.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/sidebar-09/components/app-sidebar.tsx),
  [sidebar-09 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/sidebar-09/page.tsx)

**Column widths usable for settings and detail pages (Polaris)**
- Two-column page: primary 480 to 662, secondary 240 to 320; annotated (settings) layout: a one-third
  description column of base 240 beside the cards; halves base 450; spacing between columns 16 —
  [Polaris AppProvider global.css](https://github.com/Shopify/polaris/blob/main/polaris-react/src/components/AppProvider/global.css)

**Reading column and dashboard content (shadcn)**
- Document / detail text column `max-w-3xl` (768) centred with `py-10` (40) —
  [sidebar-10 page.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/sidebar-10/page.tsx)

### Inferences
(All conventions composed from the measured tokens above; none is a sourced spec.)
- Settings page: inside the app shell, a content column of at most 768 to 960; either a 200 to 240 secondary
  nav at the left (ghost rows 32 high) with section cards at the right, or Polaris' annotated layout (240
  description column + 662 card column, 16 to 24 between). Section card: title 16/500, description 14 muted,
  fields 24 apart, a footer band with the save button right-aligned; destructive "danger zone" card last.
- Pricing table: 3 cards (sometimes 4) on a 1080 to 1200 container with 24 gaps, each about 320 to 360 wide,
  padding 24 to 32; plan name 16/500, price 36 to 48 bold with "/month" 14 muted, one-line description, a
  full-width button, then a feature list of 14 px rows with 16 check icons, 8 to 12 apart; the recommended
  plan is marked by a primary button plus a badge or a ring, not by scaling; a monthly/yearly segmented
  control centred above, 32 to 48 above the cards.
- Checkout: two columns on a 1040 to 1120 container: form about 600 to 662 (contact, shipping, payment
  sections 32 apart, fields 16 to 24 apart, two-up fields with 16 gap) and an order summary of 320 to 400 on
  a muted panel (item rows with 48 to 64 thumbnails, subtotal/shipping/tax rows 14 px, total 16 to 18 /600
  above a 1 px separator); pay button full width of the form column.
- Onboarding steps: a centred column of 480 to 560 with a stepper or "Step 2 of 4" text plus progress bar at
  the top, title 24/600, description 14 muted, one question per step (shadcn's Questionnaire choices fit
  exactly: 16 px padded bordered rows 12 apart), footer row with Back (ghost/outline) left and Continue
  (primary) right.
- Search results: search field at the top (max about 640), a muted 14 px count line, optional 240 filter
  column, results as Items (title 14 to 16 /500, snippet 14 muted two lines, meta 12 muted) separated by 1 px
  lines or 16 gaps; pagination centred below.
- Detail page: breadcrumb in the top bar; page header with title 24/600, status badge, meta line 14 muted and
  actions right; body as primary (about 662 to 768) + secondary (240 to 320) columns per Polaris: main cards
  left, properties/activity right.
- 404 / error: everything centred in the viewport in a column of at most 384 to 448, gap 24: the code or a 48
  icon tile, title 24 to 30 /600, one muted sentence at 14 to 16, then one primary and one outline/ghost
  button with gap 8. shadcn's `empty` component is the natural base (not measured here).
- Master-detail beyond mail: list pane 300 to 360 (shadcn 350 including the rail; M3 drawer 360), detail pane
  fluid with a 768 reading column.

### Gaps
- No measured source for pricing, checkout, onboarding, search results, detail or 404 pages was fetched;
  shadcn's registry on `main` contains only dashboard, login, signup, sidebar and "preview" blocks.
- shadcn's `empty` component classes (for error and empty pages) and the `preview`, `preview-02`,
  `preview-03` blocks were not opened.
- login-05 and signup-02..05 were not read individually; they are assumed to mirror login-02..05.
- Material 3 list-detail and supporting-pane widths could not be verified (see section 3).
