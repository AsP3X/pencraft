# Charts, ready to change

Each block is one layer to insert with `edit_layers` (`{"op":"insert","node":<block>}`). They use the
variables of `references/colours.md`: set those first. Every height, point and angle in them was worked out
from the values named beside it, by the rules of section 4 of the skill: change the values, then work the
sizes out again. Their look (colours, corners, weights, spacing) is a plain default: restyle them to the
design they go into. What should survive a restyling is the arithmetic. Ids start with `chart-`; give a second chart of the same kind ids of its own.

A chart looked at by itself is labels of one size, and several series are several colours. Pencraft's
review remarks on both (type without a hierarchy, more than three accent hues). In a card or a screen the
title gives the hierarchy, and the colours of a categorical chart are its meaning: those two remarks can
stand for a chart.

The examples share one set of data, a ferry company's ticket sales: 23,960 tickets from June to November
2026, 3,140 of them in November.

## Bar chart

Tickets sold per month: 4,180, 5,240, 4,910, 3,620, 2,870, 3,140. The top tick is 6,000 and the plot 200 high, so
a bar is `value / 6000 * 200` high: 139, 175, 164, 121, 96, 105. Three steps of
2,000. The plot is 432 wide with a gap of 24 and 12 beside, which makes six bars of 48.

```json
{"type":"frame","id":"chart-bar","name":"Bar chart","layout":"vertical","gap":12,"children":[
  {"type":"frame","id":"chart-bar-body","name":"Body","gap":8,"children":[
    {"type":"frame","id":"chart-bar-y","name":"Y axis","width":40,"height":216,"layout":"vertical","justifyContent":"space_between","children":[
      {"type":"text","id":"chart-bar-y-0","content":"6,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-bar-y-1","content":"4,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-bar-y-2","content":"2,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-bar-y-3","content":"0","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333}]},
    {"type":"frame","id":"chart-bar-main","layout":"vertical","gap":2,"children":[
      {"type":"frame","id":"chart-bar-plot","name":"Plot","width":432,"height":216,"padding":[8,12],"gap":24,"alignItems":"end","children":[
        {"type":"rectangle","id":"chart-bar-grid-0","name":"Baseline","width":432,"height":1,"fill":"$border","layoutPosition":"absolute","x":0,"y":208},
        {"type":"rectangle","id":"chart-bar-grid-1","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":141},
        {"type":"rectangle","id":"chart-bar-grid-2","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":75},
        {"type":"rectangle","id":"chart-bar-grid-3","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":8},
        {"type":"rectangle","id":"chart-bar-bar-1-1-1","width":"fill_container","height":139,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
        {"type":"rectangle","id":"chart-bar-bar-2-1-1","width":"fill_container","height":175,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
        {"type":"rectangle","id":"chart-bar-bar-3-1-1","width":"fill_container","height":164,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
        {"type":"rectangle","id":"chart-bar-bar-4-1-1","width":"fill_container","height":121,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
        {"type":"rectangle","id":"chart-bar-bar-5-1-1","width":"fill_container","height":96,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
        {"type":"rectangle","id":"chart-bar-bar-6-1-1","width":"fill_container","height":105,"fill":"$chart-1","cornerRadius":[4,4,0,0]}]},
      {"type":"frame","id":"chart-bar-x","name":"X axis","width":432,"padding":[0,12],"gap":24,"children":[
        {"type":"frame","id":"chart-bar-x-1","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-bar-x-1-label","content":"Jun","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-bar-x-2","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-bar-x-2-label","content":"Jul","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-bar-x-3","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-bar-x-3-label","content":"Aug","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-bar-x-4","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-bar-x-4-label","content":"Sep","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-bar-x-5","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-bar-x-5-label","content":"Oct","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-bar-x-6","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-bar-x-6-label","content":"Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]}]}]}]}]}
```

For values written on the bars, put each bar into a vertical frame with a label of 12 in `$foreground` over
it, 4 apart, and leave the value axis out: a chart needs one or the other.

## Grouped bars

Online and desk sales per month: online 2,950, 3,810, 3,520, 2,480, 1,930, 2,210; at the desk
1,230, 1,430, 1,390, 1,140, 940, 930. Top tick 4,000, so a bar is `value / 4000 * 200` high. The bars of a
month are 4 apart; months are a bar's width apart or more. The legend is under the chart, centred.

```json
{"type":"frame","id":"chart-grouped","name":"Grouped bar chart","layout":"vertical","gap":12,"children":[
  {"type":"frame","id":"chart-grouped-body","name":"Body","gap":8,"children":[
    {"type":"frame","id":"chart-grouped-y","name":"Y axis","width":40,"height":216,"layout":"vertical","justifyContent":"space_between","children":[
      {"type":"text","id":"chart-grouped-y-0","content":"4,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-grouped-y-1","content":"3,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-grouped-y-2","content":"2,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-grouped-y-3","content":"1,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-grouped-y-4","content":"0","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333}]},
    {"type":"frame","id":"chart-grouped-main","layout":"vertical","gap":2,"children":[
      {"type":"frame","id":"chart-grouped-plot","name":"Plot","width":432,"height":216,"padding":[8,12],"gap":24,"alignItems":"end","children":[
        {"type":"rectangle","id":"chart-grouped-grid-0","name":"Baseline","width":432,"height":1,"fill":"$border","layoutPosition":"absolute","x":0,"y":208},
        {"type":"rectangle","id":"chart-grouped-grid-1","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":158},
        {"type":"rectangle","id":"chart-grouped-grid-2","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":108},
        {"type":"rectangle","id":"chart-grouped-grid-3","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":58},
        {"type":"rectangle","id":"chart-grouped-grid-4","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":8},
        {"type":"frame","id":"chart-grouped-group-1","name":"Category","width":"fill_container","gap":4,"alignItems":"end","children":[
          {"type":"rectangle","id":"chart-grouped-bar-1-1-1","width":"fill_container","height":148,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-grouped-bar-1-2-1","width":"fill_container","height":62,"fill":"$chart-2","cornerRadius":[4,4,0,0]}]},
        {"type":"frame","id":"chart-grouped-group-2","name":"Category","width":"fill_container","gap":4,"alignItems":"end","children":[
          {"type":"rectangle","id":"chart-grouped-bar-2-1-1","width":"fill_container","height":190,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-grouped-bar-2-2-1","width":"fill_container","height":72,"fill":"$chart-2","cornerRadius":[4,4,0,0]}]},
        {"type":"frame","id":"chart-grouped-group-3","name":"Category","width":"fill_container","gap":4,"alignItems":"end","children":[
          {"type":"rectangle","id":"chart-grouped-bar-3-1-1","width":"fill_container","height":176,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-grouped-bar-3-2-1","width":"fill_container","height":70,"fill":"$chart-2","cornerRadius":[4,4,0,0]}]},
        {"type":"frame","id":"chart-grouped-group-4","name":"Category","width":"fill_container","gap":4,"alignItems":"end","children":[
          {"type":"rectangle","id":"chart-grouped-bar-4-1-1","width":"fill_container","height":124,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-grouped-bar-4-2-1","width":"fill_container","height":57,"fill":"$chart-2","cornerRadius":[4,4,0,0]}]},
        {"type":"frame","id":"chart-grouped-group-5","name":"Category","width":"fill_container","gap":4,"alignItems":"end","children":[
          {"type":"rectangle","id":"chart-grouped-bar-5-1-1","width":"fill_container","height":96,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-grouped-bar-5-2-1","width":"fill_container","height":47,"fill":"$chart-2","cornerRadius":[4,4,0,0]}]},
        {"type":"frame","id":"chart-grouped-group-6","name":"Category","width":"fill_container","gap":4,"alignItems":"end","children":[
          {"type":"rectangle","id":"chart-grouped-bar-6-1-1","width":"fill_container","height":110,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-grouped-bar-6-2-1","width":"fill_container","height":46,"fill":"$chart-2","cornerRadius":[4,4,0,0]}]}]},
      {"type":"frame","id":"chart-grouped-x","name":"X axis","width":432,"padding":[0,12],"gap":24,"children":[
        {"type":"frame","id":"chart-grouped-x-1","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-grouped-x-1-label","content":"Jun","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-grouped-x-2","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-grouped-x-2-label","content":"Jul","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-grouped-x-3","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-grouped-x-3-label","content":"Aug","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-grouped-x-4","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-grouped-x-4-label","content":"Sep","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-grouped-x-5","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-grouped-x-5-label","content":"Oct","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-grouped-x-6","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-grouped-x-6-label","content":"Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]}]}]}]},
  {"type":"frame","id":"chart-grouped-legend-row","width":"fill_container","justifyContent":"center","children":[
    {"type":"frame","id":"chart-grouped-legend","name":"Legend","gap":16,"alignItems":"center","children":[
      {"type":"frame","id":"chart-grouped-legend-1","name":"Item","gap":6,"alignItems":"center","children":[
        {"type":"rectangle","id":"chart-grouped-legend-1-swatch","width":8,"height":8,"fill":"$chart-1","cornerRadius":2},
        {"type":"text","id":"chart-grouped-legend-1-label","content":"Online","fontFamily":"$font","fontSize":12,"fill":"$foreground"}]},
      {"type":"frame","id":"chart-grouped-legend-2","name":"Item","gap":6,"alignItems":"center","children":[
        {"type":"rectangle","id":"chart-grouped-legend-2-swatch","width":8,"height":8,"fill":"$chart-2","cornerRadius":2},
        {"type":"text","id":"chart-grouped-legend-2-label","content":"At the desk","fontFamily":"$font","fontSize":12,"fill":"$foreground"}]}]}]}]}
```

## Stacked bars

The same two series stacked, so each bar's height is the month's total and the first chart's bars again.
The lower part is drawn first from the baseline; the parts are 2 apart, and only the top one is rounded.
Their heights are worked out part by part, and the 2 of the gap is taken off the lower part, so the
stack still ends where its total says.

```json
{"type":"frame","id":"chart-stacked","name":"Stacked bar chart","layout":"vertical","gap":12,"children":[
  {"type":"frame","id":"chart-stacked-body","name":"Body","gap":8,"children":[
    {"type":"frame","id":"chart-stacked-y","name":"Y axis","width":40,"height":216,"layout":"vertical","justifyContent":"space_between","children":[
      {"type":"text","id":"chart-stacked-y-0","content":"6,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-stacked-y-1","content":"4,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-stacked-y-2","content":"2,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-stacked-y-3","content":"0","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333}]},
    {"type":"frame","id":"chart-stacked-main","layout":"vertical","gap":2,"children":[
      {"type":"frame","id":"chart-stacked-plot","name":"Plot","width":432,"height":216,"padding":[8,12],"gap":24,"alignItems":"end","children":[
        {"type":"rectangle","id":"chart-stacked-grid-0","name":"Baseline","width":432,"height":1,"fill":"$border","layoutPosition":"absolute","x":0,"y":208},
        {"type":"rectangle","id":"chart-stacked-grid-1","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":141},
        {"type":"rectangle","id":"chart-stacked-grid-2","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":75},
        {"type":"rectangle","id":"chart-stacked-grid-3","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":8},
        {"type":"frame","id":"chart-stacked-stack-1-1","name":"Stack","width":"fill_container","layout":"vertical","gap":2,"children":[
          {"type":"rectangle","id":"chart-stacked-bar-1-1-2","width":"fill_container","height":41,"fill":"$chart-2","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-stacked-bar-1-1-1","width":"fill_container","height":96,"fill":"$chart-1"}]},
        {"type":"frame","id":"chart-stacked-stack-2-1","name":"Stack","width":"fill_container","layout":"vertical","gap":2,"children":[
          {"type":"rectangle","id":"chart-stacked-bar-2-1-2","width":"fill_container","height":48,"fill":"$chart-2","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-stacked-bar-2-1-1","width":"fill_container","height":125,"fill":"$chart-1"}]},
        {"type":"frame","id":"chart-stacked-stack-3-1","name":"Stack","width":"fill_container","layout":"vertical","gap":2,"children":[
          {"type":"rectangle","id":"chart-stacked-bar-3-1-2","width":"fill_container","height":46,"fill":"$chart-2","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-stacked-bar-3-1-1","width":"fill_container","height":115,"fill":"$chart-1"}]},
        {"type":"frame","id":"chart-stacked-stack-4-1","name":"Stack","width":"fill_container","layout":"vertical","gap":2,"children":[
          {"type":"rectangle","id":"chart-stacked-bar-4-1-2","width":"fill_container","height":38,"fill":"$chart-2","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-stacked-bar-4-1-1","width":"fill_container","height":81,"fill":"$chart-1"}]},
        {"type":"frame","id":"chart-stacked-stack-5-1","name":"Stack","width":"fill_container","layout":"vertical","gap":2,"children":[
          {"type":"rectangle","id":"chart-stacked-bar-5-1-2","width":"fill_container","height":31,"fill":"$chart-2","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-stacked-bar-5-1-1","width":"fill_container","height":62,"fill":"$chart-1"}]},
        {"type":"frame","id":"chart-stacked-stack-6-1","name":"Stack","width":"fill_container","layout":"vertical","gap":2,"children":[
          {"type":"rectangle","id":"chart-stacked-bar-6-1-2","width":"fill_container","height":31,"fill":"$chart-2","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-stacked-bar-6-1-1","width":"fill_container","height":72,"fill":"$chart-1"}]}]},
      {"type":"frame","id":"chart-stacked-x","name":"X axis","width":432,"padding":[0,12],"gap":24,"children":[
        {"type":"frame","id":"chart-stacked-x-1","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-stacked-x-1-label","content":"Jun","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-stacked-x-2","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-stacked-x-2-label","content":"Jul","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-stacked-x-3","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-stacked-x-3-label","content":"Aug","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-stacked-x-4","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-stacked-x-4-label","content":"Sep","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-stacked-x-5","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-stacked-x-5-label","content":"Oct","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"chart-stacked-x-6","width":"fill_container","justifyContent":"center","children":[
          {"type":"text","id":"chart-stacked-x-6-label","content":"Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]}]}]}]},
  {"type":"frame","id":"chart-stacked-legend-row","width":"fill_container","justifyContent":"center","children":[
    {"type":"frame","id":"chart-stacked-legend","name":"Legend","gap":16,"alignItems":"center","children":[
      {"type":"frame","id":"chart-stacked-legend-1","name":"Item","gap":6,"alignItems":"center","children":[
        {"type":"rectangle","id":"chart-stacked-legend-1-swatch","width":8,"height":8,"fill":"$chart-1","cornerRadius":2},
        {"type":"text","id":"chart-stacked-legend-1-label","content":"Online","fontFamily":"$font","fontSize":12,"fill":"$foreground"}]},
      {"type":"frame","id":"chart-stacked-legend-2","name":"Item","gap":6,"alignItems":"center","children":[
        {"type":"rectangle","id":"chart-stacked-legend-2-swatch","width":8,"height":8,"fill":"$chart-2","cornerRadius":2},
        {"type":"text","id":"chart-stacked-legend-2-label","content":"At the desk","fontFamily":"$font","fontSize":12,"fill":"$foreground"}]}]}]}]}
```

## Line chart

Departures on time per week, in per cent, for two lines of the company: the harbour line
96.2, 95.1, 97.4, 93.8, 91.2, 94.6, 96.9, 97.8; the island line 92.4, 93.1, 90.6, 88.9, 91.7, 89.4, 92.8, 93.5. A rate
need not start at zero: the axis runs from 85% to 100% in steps of 5 and says so in its labels. The chart
is about the harbour line, so that one has the colour and the other is `$chart-muted`. Each is named at
its end with its last value, which leaves 96 at the trailing side of the plot for the names.

```json
{"type":"frame","id":"chart-line","name":"Line chart","layout":"vertical","gap":12,"children":[
  {"type":"frame","id":"chart-line-body","name":"Body","gap":8,"children":[
    {"type":"frame","id":"chart-line-y","name":"Y axis","width":40,"height":216,"layout":"vertical","justifyContent":"space_between","children":[
      {"type":"text","id":"chart-line-y-0","content":"100%","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-line-y-1","content":"95%","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-line-y-2","content":"90%","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-line-y-3","content":"85%","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333}]},
    {"type":"frame","id":"chart-line-main","layout":"vertical","gap":2,"children":[
      {"type":"frame","id":"chart-line-plot","name":"Plot","width":432,"height":216,"layout":"none","children":[
        {"type":"rectangle","id":"chart-line-grid-0","name":"Baseline","width":432,"height":1,"fill":"$border","layoutPosition":"absolute","x":0,"y":208},
        {"type":"rectangle","id":"chart-line-grid-1","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":141},
        {"type":"rectangle","id":"chart-line-grid-2","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":75},
        {"type":"rectangle","id":"chart-line-grid-3","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":8},
        {"type":"path","id":"chart-line-island","name":"Island line","width":432,"height":216,"viewBox":[0,0,432,216],"geometry":"M24.0 109.3 L65.1 100.0 L106.3 133.3 L147.4 156.0 L188.6 118.7 L229.7 149.3 L270.9 104.0 L312.0 94.7","stroke":"$chart-muted","strokeWidth":2,"strokeLinejoin":"round","strokeLinecap":"round","layoutPosition":"absolute","x":0,"y":0},
        {"type":"path","id":"chart-line-harbour","name":"Harbour line","width":432,"height":216,"viewBox":[0,0,432,216],"geometry":"M24.0 58.7 L65.1 73.3 L106.3 42.7 L147.4 90.7 L188.6 125.3 L229.7 80.0 L270.9 49.3 L312.0 37.3","stroke":"$chart-1","strokeWidth":2,"strokeLinejoin":"round","strokeLinecap":"round","layoutPosition":"absolute","x":0,"y":0},
        {"type":"ellipse","id":"chart-line-harbour-end","name":"End point","width":8,"height":8,"fill":"$chart-1","layoutPosition":"absolute","x":308.0,"y":33.3},
        {"type":"text","id":"chart-line-harbour-label","content":"Harbour line 97.8%","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground","layoutPosition":"absolute","x":322.0,"y":29.299999999999997},
        {"type":"text","id":"chart-line-island-label","content":"Island line 93.5%","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","layoutPosition":"absolute","x":322.0,"y":86.7}]},
      {"type":"frame","id":"chart-line-x","name":"X axis","width":432,"height":16,"layout":"none","children":[
        {"type":"text","id":"chart-line-x-1","content":"W40","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":0.0,"y":0},
        {"type":"text","id":"chart-line-x-2","content":"W41","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":41.099999999999994,"y":0},
        {"type":"text","id":"chart-line-x-3","content":"W42","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":82.3,"y":0},
        {"type":"text","id":"chart-line-x-4","content":"W43","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":123.4,"y":0},
        {"type":"text","id":"chart-line-x-5","content":"W44","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":164.6,"y":0},
        {"type":"text","id":"chart-line-x-6","content":"W45","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":205.7,"y":0},
        {"type":"text","id":"chart-line-x-7","content":"W46","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":246.89999999999998,"y":0},
        {"type":"text","id":"chart-line-x-8","content":"W47","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":288.0,"y":0}]}]}]}]}
```

## Area chart

Passengers per day for two weeks: 610, 580, 640, 720, 980, 1,340, 1,210, 590, 560, 630, 700, 1,010, 1,420, 1,180. Saturdays and Sundays are
high, which is what makes it look like a ferry. An area stands on zero; top tick 1,500. The area is the
line's points, then down to the baseline and back.

```json
{"type":"frame","id":"chart-area","name":"Area chart","layout":"vertical","gap":12,"children":[
  {"type":"frame","id":"chart-area-body","name":"Body","gap":8,"children":[
    {"type":"frame","id":"chart-area-y","name":"Y axis","width":40,"height":216,"layout":"vertical","justifyContent":"space_between","children":[
      {"type":"text","id":"chart-area-y-0","content":"1,500","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-area-y-1","content":"1,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-area-y-2","content":"500","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
      {"type":"text","id":"chart-area-y-3","content":"0","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333}]},
    {"type":"frame","id":"chart-area-main","layout":"vertical","gap":2,"children":[
      {"type":"frame","id":"chart-area-plot","name":"Plot","width":432,"height":216,"layout":"none","children":[
        {"type":"rectangle","id":"chart-area-grid-0","name":"Baseline","width":432,"height":1,"fill":"$border","layoutPosition":"absolute","x":0,"y":208},
        {"type":"rectangle","id":"chart-area-grid-1","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":141},
        {"type":"rectangle","id":"chart-area-grid-2","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":75},
        {"type":"rectangle","id":"chart-area-grid-3","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":8},
        {"type":"path","id":"chart-area-fill","name":"Area","width":432,"height":216,"viewBox":[0,0,432,216],"geometry":"M24.0 126.7 L53.5 130.7 L83.1 122.7 L112.6 112.0 L142.2 77.3 L171.7 29.3 L201.2 46.7 L230.8 129.3 L260.3 133.3 L289.8 124.0 L319.4 114.7 L348.9 73.3 L378.5 18.7 L408.0 50.7 L408.0 208 L24.0 208 Z","fill":{"type":"color","color":"$chart-1","opacity":0.3},"layoutPosition":"absolute","x":0,"y":0},
        {"type":"path","id":"chart-area-line","name":"Line","width":432,"height":216,"viewBox":[0,0,432,216],"geometry":"M24.0 126.7 L53.5 130.7 L83.1 122.7 L112.6 112.0 L142.2 77.3 L171.7 29.3 L201.2 46.7 L230.8 129.3 L260.3 133.3 L289.8 124.0 L319.4 114.7 L348.9 73.3 L378.5 18.7 L408.0 50.7","stroke":"$chart-1","strokeWidth":2,"strokeLinejoin":"round","strokeLinecap":"round","layoutPosition":"absolute","x":0,"y":0}]},
      {"type":"frame","id":"chart-area-x","name":"X axis","width":432,"height":16,"layout":"none","children":[
        {"type":"text","id":"chart-area-x-1","content":"9 Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":0.0,"y":0},
        {"type":"text","id":"chart-area-x-4","content":"12 Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":88.6,"y":0},
        {"type":"text","id":"chart-area-x-7","content":"15 Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":177.2,"y":0},
        {"type":"text","id":"chart-area-x-10","content":"18 Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":265.8,"y":0},
        {"type":"text","id":"chart-area-x-13","content":"21 Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"textAlign":"center","layoutPosition":"absolute","x":354.5,"y":0}]}]}]}]}
```

## Sparkline

The last twelve weeks of ticket sales, ending on November's 3,140: 2,710, 2,840, 2,620, 2,930, 3,080, 2,890, 2,760, 3,010, 3,190, 2,980, 2,870, 3,140.
No axis and no zero: the lowest value is at the bottom and the highest at the top, 4 from the edges.

```json
{"type":"path","id":"chart-spark","name":"Sparkline","width":112,"height":48,"viewBox":[0,0,112,48],"geometry":"M2.0 37.7 L11.8 28.6 L21.6 44.0 L31.5 22.2 L41.3 11.7 L51.1 25.1 L60.9 34.2 L70.7 16.6 L80.5 4.0 L90.4 18.7 L100.2 26.5 L110.0 7.5","stroke":"$chart-1","strokeWidth":2,"strokeLinejoin":"round","strokeLinecap":"round"}
```

## Donut

November's 3,140 tickets by kind: Single 1,412, Return 942, Season pass 471, Group 315.
A slice sweeps `value / 3140 * 360` degrees: 161.9, 108.0, 54.0, 36.1.
They run clockwise from the top, largest first, each 1 degree short at both ends. The total is in the
centre, and the rows beside it give each value and its share.

```json
{"type":"frame","id":"chart-donut","name":"Donut chart","gap":32,"alignItems":"center","children":[
  {"type":"frame","id":"chart-donut-ring","name":"Ring","width":160,"height":160,"layout":"none","children":[
    {"type":"ellipse","id":"chart-donut-slice-1","name":"Single","width":160,"height":160,"innerRadius":0.72,"startAngle":-70.9,"sweepAngle":159.9,"fill":"$chart-1","layoutPosition":"absolute","x":0,"y":0},
    {"type":"ellipse","id":"chart-donut-slice-2","name":"Return","width":160,"height":160,"innerRadius":0.72,"startAngle":-178.9,"sweepAngle":106.0,"fill":"$chart-2","layoutPosition":"absolute","x":0,"y":0},
    {"type":"ellipse","id":"chart-donut-slice-3","name":"Season pass","width":160,"height":160,"innerRadius":0.72,"startAngle":-232.9,"sweepAngle":52.0,"fill":"$chart-3","layoutPosition":"absolute","x":0,"y":0},
    {"type":"ellipse","id":"chart-donut-slice-4","name":"Group","width":160,"height":160,"innerRadius":0.72,"startAngle":-269.0,"sweepAngle":34.1,"fill":"$chart-4","layoutPosition":"absolute","x":0,"y":0},
    {"type":"frame","id":"chart-donut-centre","name":"Centre","width":160,"height":160,"layout":"vertical","gap":2,"justifyContent":"center","alignItems":"center","layoutPosition":"absolute","x":0,"y":0,"children":[
      {"type":"text","id":"chart-donut-total","name":"Total","content":"3,140","fontFamily":"$font","fontSize":24,"fontWeight":"600","fill":"$foreground"},
      {"type":"text","id":"chart-donut-caption","name":"Caption","content":"tickets","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]}]},
  {"type":"frame","id":"chart-donut-rows","name":"Legend","width":200,"layout":"vertical","gap":10,"children":[
    {"type":"frame","id":"chart-donut-row-1","name":"Row","width":"fill_container","gap":8,"alignItems":"center","children":[
      {"type":"rectangle","id":"chart-donut-row-1-swatch","width":8,"height":8,"fill":"$chart-1","cornerRadius":2},
      {"type":"text","id":"chart-donut-row-1-label","content":"Single","fontFamily":"$font","fontSize":12,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"chart-donut-row-1-value","content":"1,412","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":40,"textAlign":"right"},
      {"type":"text","id":"chart-donut-row-1-share","content":"45%","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":32,"textAlign":"right"}]},
    {"type":"frame","id":"chart-donut-row-2","name":"Row","width":"fill_container","gap":8,"alignItems":"center","children":[
      {"type":"rectangle","id":"chart-donut-row-2-swatch","width":8,"height":8,"fill":"$chart-2","cornerRadius":2},
      {"type":"text","id":"chart-donut-row-2-label","content":"Return","fontFamily":"$font","fontSize":12,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"chart-donut-row-2-value","content":"942","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":40,"textAlign":"right"},
      {"type":"text","id":"chart-donut-row-2-share","content":"30%","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":32,"textAlign":"right"}]},
    {"type":"frame","id":"chart-donut-row-3","name":"Row","width":"fill_container","gap":8,"alignItems":"center","children":[
      {"type":"rectangle","id":"chart-donut-row-3-swatch","width":8,"height":8,"fill":"$chart-3","cornerRadius":2},
      {"type":"text","id":"chart-donut-row-3-label","content":"Season pass","fontFamily":"$font","fontSize":12,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"chart-donut-row-3-value","content":"471","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":40,"textAlign":"right"},
      {"type":"text","id":"chart-donut-row-3-share","content":"15%","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":32,"textAlign":"right"}]},
    {"type":"frame","id":"chart-donut-row-4","name":"Row","width":"fill_container","gap":8,"alignItems":"center","children":[
      {"type":"rectangle","id":"chart-donut-row-4-swatch","width":8,"height":8,"fill":"$chart-4","cornerRadius":2},
      {"type":"text","id":"chart-donut-row-4-label","content":"Group","fontFamily":"$font","fontSize":12,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"chart-donut-row-4-value","content":"315","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":40,"textAlign":"right"},
      {"type":"text","id":"chart-donut-row-4-share","content":"10%","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":32,"textAlign":"right"}]}]}]}
```

## Gauge

94 of the 120 places on the car deck are taken: 78%. The ring is open at the bottom over 270 degrees; the
value covers `270 * 94 / 120` = 211.5 of them from the lower left.

```json
{"type":"frame","id":"chart-gauge","name":"Gauge","width":160,"height":160,"layout":"none","children":[
  {"type":"ellipse","id":"chart-gauge-track","name":"Track","width":160,"height":160,"innerRadius":0.86,"startAngle":-45,"sweepAngle":270,"fill":"$muted","layoutPosition":"absolute","x":0,"y":0},
  {"type":"ellipse","id":"chart-gauge-value","name":"Value","width":160,"height":160,"innerRadius":0.86,"startAngle":13.5,"sweepAngle":211.5,"fill":"$chart-1","layoutPosition":"absolute","x":0,"y":0},
  {"type":"frame","id":"chart-gauge-centre","name":"Centre","width":160,"height":160,"layout":"vertical","gap":2,"justifyContent":"center","alignItems":"center","layoutPosition":"absolute","x":0,"y":0,"children":[
    {"type":"text","id":"chart-gauge-figure","name":"Figure","content":"78%","fontFamily":"$font","fontSize":28,"fontWeight":"600","fill":"$foreground"},
    {"type":"text","id":"chart-gauge-caption","name":"Caption","content":"94 of 120 car places","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]}]}
```

## Progress ring

A small one for a row or a tile: 64 across with a ring 6 wide, 62% of the way round from the top.

```json
{"type":"frame","id":"chart-ring","name":"Progress ring","width":64,"height":64,"layout":"none","children":[
  {"type":"ellipse","id":"chart-ring-track","name":"Track","width":64,"height":64,"innerRadius":0.8125,"fill":"$muted","layoutPosition":"absolute","x":0,"y":0},
  {"type":"ellipse","id":"chart-ring-value","name":"Value","width":64,"height":64,"innerRadius":0.8125,"startAngle":-133.2,"sweepAngle":223.2,"fill":"$chart-1","layoutPosition":"absolute","x":0,"y":0},
  {"type":"frame","id":"chart-ring-centre","width":64,"height":64,"justifyContent":"center","alignItems":"center","layoutPosition":"absolute","x":0,"y":0,"children":[
    {"type":"text","id":"chart-ring-figure","name":"Figure","content":"62%","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]}]}
```

## Bar list

November's tickets by route, largest first: Bergen to Askøy 1,284, Bergen to Sotra 846, Askøy to Sotra 512, Bergen to Fedje 318, All other routes 180.
The longest bar is the full 360; the others are `value / 1284 * 360` wide. The names stand on the bars,
the values end on one edge outside them, and what is left over is one grey row at the end.

```json
{"type":"frame","id":"chart-list","name":"Bar list","layout":"vertical","gap":6,"children":[
  {"type":"frame","id":"chart-list-1","name":"Row","gap":12,"alignItems":"center","children":[
    {"type":"frame","id":"chart-list-1-track","width":360,"height":32,"layout":"none","children":[
      {"type":"rectangle","id":"chart-list-1-bar","name":"Bar","width":360,"height":32,"fill":{"type":"color","color":"$chart-1","opacity":0.25},"cornerRadius":4,"layoutPosition":"absolute","x":0,"y":0},
      {"type":"text","id":"chart-list-1-label","content":"Bergen to Askøy","fontFamily":"$font","fontSize":14,"fill":"$foreground","layoutPosition":"absolute","x":8,"y":7}]},
    {"type":"text","id":"chart-list-1-value","content":"1,284","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":48,"textAlign":"right"}]},
  {"type":"frame","id":"chart-list-2","name":"Row","gap":12,"alignItems":"center","children":[
    {"type":"frame","id":"chart-list-2-track","width":360,"height":32,"layout":"none","children":[
      {"type":"rectangle","id":"chart-list-2-bar","name":"Bar","width":237,"height":32,"fill":{"type":"color","color":"$chart-1","opacity":0.25},"cornerRadius":4,"layoutPosition":"absolute","x":0,"y":0},
      {"type":"text","id":"chart-list-2-label","content":"Bergen to Sotra","fontFamily":"$font","fontSize":14,"fill":"$foreground","layoutPosition":"absolute","x":8,"y":7}]},
    {"type":"text","id":"chart-list-2-value","content":"846","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":48,"textAlign":"right"}]},
  {"type":"frame","id":"chart-list-3","name":"Row","gap":12,"alignItems":"center","children":[
    {"type":"frame","id":"chart-list-3-track","width":360,"height":32,"layout":"none","children":[
      {"type":"rectangle","id":"chart-list-3-bar","name":"Bar","width":144,"height":32,"fill":{"type":"color","color":"$chart-1","opacity":0.25},"cornerRadius":4,"layoutPosition":"absolute","x":0,"y":0},
      {"type":"text","id":"chart-list-3-label","content":"Askøy to Sotra","fontFamily":"$font","fontSize":14,"fill":"$foreground","layoutPosition":"absolute","x":8,"y":7}]},
    {"type":"text","id":"chart-list-3-value","content":"512","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":48,"textAlign":"right"}]},
  {"type":"frame","id":"chart-list-4","name":"Row","gap":12,"alignItems":"center","children":[
    {"type":"frame","id":"chart-list-4-track","width":360,"height":32,"layout":"none","children":[
      {"type":"rectangle","id":"chart-list-4-bar","name":"Bar","width":89,"height":32,"fill":{"type":"color","color":"$chart-1","opacity":0.25},"cornerRadius":4,"layoutPosition":"absolute","x":0,"y":0},
      {"type":"text","id":"chart-list-4-label","content":"Bergen to Fedje","fontFamily":"$font","fontSize":14,"fill":"$foreground","layoutPosition":"absolute","x":8,"y":7}]},
    {"type":"text","id":"chart-list-4-value","content":"318","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":48,"textAlign":"right"}]},
  {"type":"frame","id":"chart-list-5","name":"Row","gap":12,"alignItems":"center","children":[
    {"type":"frame","id":"chart-list-5-track","width":360,"height":32,"layout":"none","children":[
      {"type":"rectangle","id":"chart-list-5-bar","name":"Bar","width":50,"height":32,"fill":{"type":"color","color":"$chart-muted","opacity":0.25},"cornerRadius":4,"layoutPosition":"absolute","x":0,"y":0},
      {"type":"text","id":"chart-list-5-label","content":"All other routes","fontFamily":"$font","fontSize":14,"fill":"$foreground","layoutPosition":"absolute","x":8,"y":7}]},
    {"type":"text","id":"chart-list-5-value","content":"180","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":48,"textAlign":"right"}]}]}
```

## Tracker

One block per day for thirty days: on time, some departures late, or a day with cancelled sailings. The
count beside the title says the same as the blocks (27 of 30 are `$chart-good`).

```json
{"type":"frame","id":"chart-tracker","name":"Tracker","width":480,"layout":"vertical","gap":8,"children":[
  {"type":"frame","id":"chart-tracker-head","width":"fill_container","gap":8,"alignItems":"center","children":[
    {"type":"text","id":"chart-tracker-title","content":"Departures on time, last 30 days","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"chart-tracker-figure","content":"27 of 30 days","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"chart-tracker-strip","name":"Strip","width":"fill_container","gap":2,"children":[
    {"type":"rectangle","id":"chart-tracker-1","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":[4,1,1,4]},
    {"type":"rectangle","id":"chart-tracker-2","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-3","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-4","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-5","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-6","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-7","width":"fill_container","height":32,"fill":"$chart-warn","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-8","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-9","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-10","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-11","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-12","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-13","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-14","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-15","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-16","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-17","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-18","width":"fill_container","height":32,"fill":"$chart-bad","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-19","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-20","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-21","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-22","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-23","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-24","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-25","width":"fill_container","height":32,"fill":"$chart-warn","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-26","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-27","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-28","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-29","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":1},
    {"type":"rectangle","id":"chart-tracker-30","width":"fill_container","height":32,"fill":"$chart-good","cornerRadius":[1,4,4,1]}]},
  {"type":"frame","id":"chart-tracker-ends","width":"fill_container","justifyContent":"space_between","children":[
    {"type":"text","id":"chart-tracker-from","content":"23 Oct","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"},
    {"type":"text","id":"chart-tracker-to","content":"21 Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]}]}
```

## Candlestick chart

Twenty trading days of a made-up share, from 26 October to 20 November 2026 (Mondays to Fridays). Each day
has four prices: open, high, low and close. The last day opened at 80.85, went as high as 82.15 and as
low as 80.65, and closed at 81.80: 0.65 above the close before it (81.15), which is 0.80%.
The axis runs from 80 to 86 in steps of 2 and stands at the trailing side; a price `p` lies
`200 * (1 - (p - 80) / 6)` under the top tick. The plot is 432 wide, so each of the twenty bands is
21.6 and a body 13. The volume strip has its own scale: the busiest day is 40 high.

```json
{"type":"frame","id":"chart-candle","name":"Candlestick chart","layout":"vertical","gap":16,"children":[
  {"type":"frame","id":"chart-candle-header","name":"Header","layout":"vertical","gap":4,"children":[
    {"type":"frame","id":"chart-candle-name-row","gap":8,"alignItems":"center","children":[
      {"type":"text","id":"chart-candle-ticker","name":"Ticker","content":"NORF","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
      {"type":"text","id":"chart-candle-company","name":"Name","content":"Nordfjord Ferries","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
    {"type":"frame","id":"chart-candle-price-row","gap":8,"alignItems":"end","children":[
      {"type":"text","id":"chart-candle-price","name":"Last price","content":"81.80","fontFamily":"$font","fontSize":24,"fontWeight":"600","fill":"$foreground"},
      {"type":"text","id":"chart-candle-currency","name":"Currency","content":"NOK","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"},
      {"type":"frame","id":"chart-candle-change-box","name":"Change","gap":4,"alignItems":"center","children":[
        {"type":"icon","id":"chart-candle-change-icon","library":"lucide","icon":"trending-up","width":16,"height":16,"fill":"$chart-up"},
        {"type":"text","id":"chart-candle-change","content":"+0.65 (+0.80%)","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]}]},
    {"type":"text","id":"chart-candle-day","name":"Day","content":"20 Nov   O 80.85   H 82.15   L 80.65   C 81.80   Vol 413k","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"chart-candle-body","name":"Body","gap":8,"children":[
    {"type":"frame","id":"chart-candle-main","layout":"vertical","gap":8,"children":[
      {"type":"frame","id":"chart-candle-plot","name":"Plot","width":432,"height":216,"layout":"none","children":[
        {"type":"rectangle","id":"chart-candle-grid-0","name":"Baseline","width":432,"height":1,"fill":"$border","layoutPosition":"absolute","x":0,"y":208},
        {"type":"rectangle","id":"chart-candle-grid-1","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":141},
        {"type":"rectangle","id":"chart-candle-grid-2","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":75},
        {"type":"rectangle","id":"chart-candle-grid-3","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":8},
        {"type":"rectangle","id":"chart-candle-1-wick","name":"Wick","width":1,"height":100.0,"fill":"$chart-down","layoutPosition":"absolute","x":10.3,"y":56.3},
        {"type":"rectangle","id":"chart-candle-1-body","name":"Body, down","width":13,"height":68.3,"fill":"$chart-down","layoutPosition":"absolute","x":4.3,"y":69.7},
        {"type":"rectangle","id":"chart-candle-2-wick","name":"Wick","width":1,"height":41.7,"fill":"$chart-up","layoutPosition":"absolute","x":31.9,"y":106.3},
        {"type":"rectangle","id":"chart-candle-2-body","name":"Body, up","width":13,"height":15.0,"fill":"$chart-up","layoutPosition":"absolute","x":25.9,"y":129.7},
        {"type":"rectangle","id":"chart-candle-3-wick","name":"Wick","width":1,"height":20.0,"fill":"$chart-up","layoutPosition":"absolute","x":53.5,"y":129.7},
        {"type":"rectangle","id":"chart-candle-3-body","name":"Body, up","width":13,"height":1.7,"fill":"$chart-up","layoutPosition":"absolute","x":47.5,"y":138.0},
        {"type":"rectangle","id":"chart-candle-4-wick","name":"Wick","width":1,"height":73.3,"fill":"$chart-down","layoutPosition":"absolute","x":75.1,"y":103.0},
        {"type":"rectangle","id":"chart-candle-4-body","name":"Body, down","width":13,"height":43.4,"fill":"$chart-down","layoutPosition":"absolute","x":69.1,"y":126.3},
        {"type":"rectangle","id":"chart-candle-5-wick","name":"Wick","width":1,"height":61.7,"fill":"$chart-up","layoutPosition":"absolute","x":96.7,"y":126.3},
        {"type":"rectangle","id":"chart-candle-5-body","name":"Body, up","width":13,"height":48.3,"fill":"$chart-up","layoutPosition":"absolute","x":90.7,"y":129.7},
        {"type":"rectangle","id":"chart-candle-6-wick","name":"Wick","width":1,"height":36.6,"fill":"$chart-up","layoutPosition":"absolute","x":118.3,"y":109.7},
        {"type":"rectangle","id":"chart-candle-6-body","name":"Body, up","width":13,"height":15.0,"fill":"$chart-up","layoutPosition":"absolute","x":112.3,"y":126.3},
        {"type":"rectangle","id":"chart-candle-7-wick","name":"Wick","width":1,"height":45.0,"fill":"$chart-down","layoutPosition":"absolute","x":139.9,"y":111.3},
        {"type":"rectangle","id":"chart-candle-7-body","name":"Body, down","width":13,"height":20.0,"fill":"$chart-down","layoutPosition":"absolute","x":133.9,"y":126.3},
        {"type":"rectangle","id":"chart-candle-8-wick","name":"Wick","width":1,"height":58.3,"fill":"$chart-down","layoutPosition":"absolute","x":161.5,"y":104.7},
        {"type":"rectangle","id":"chart-candle-8-body","name":"Body, down","width":13,"height":26.6,"fill":"$chart-down","layoutPosition":"absolute","x":155.5,"y":134.7},
        {"type":"rectangle","id":"chart-candle-9-wick","name":"Wick","width":1,"height":48.3,"fill":"$chart-up","layoutPosition":"absolute","x":183.1,"y":143.0},
        {"type":"rectangle","id":"chart-candle-9-body","name":"Body, up","width":13,"height":10.0,"fill":"$chart-up","layoutPosition":"absolute","x":177.1,"y":156.3},
        {"type":"rectangle","id":"chart-candle-10-wick","name":"Wick","width":1,"height":88.4,"fill":"$chart-up","layoutPosition":"absolute","x":204.7,"y":86.3},
        {"type":"rectangle","id":"chart-candle-10-body","name":"Body, up","width":13,"height":31.7,"fill":"$chart-up","layoutPosition":"absolute","x":198.7,"y":116.3},
        {"type":"rectangle","id":"chart-candle-11-wick","name":"Wick","width":1,"height":101.6,"fill":"$chart-up","layoutPosition":"absolute","x":226.3,"y":69.7},
        {"type":"rectangle","id":"chart-candle-11-body","name":"Body, up","width":13,"height":25.0,"fill":"$chart-up","layoutPosition":"absolute","x":220.3,"y":103.0},
        {"type":"rectangle","id":"chart-candle-12-wick","name":"Wick","width":1,"height":65.0,"fill":"$chart-up","layoutPosition":"absolute","x":247.9,"y":68.0},
        {"type":"rectangle","id":"chart-candle-12-body","name":"Body, up","width":13,"height":25.0,"fill":"$chart-up","layoutPosition":"absolute","x":241.9,"y":88.0},
        {"type":"rectangle","id":"chart-candle-13-wick","name":"Wick","width":1,"height":75.0,"fill":"$chart-down","layoutPosition":"absolute","x":269.5,"y":63.0},
        {"type":"rectangle","id":"chart-candle-13-body","name":"Body, down","width":13,"height":38.3,"fill":"$chart-down","layoutPosition":"absolute","x":263.5,"y":98.0},
        {"type":"rectangle","id":"chart-candle-14-wick","name":"Wick","width":1,"height":50.0,"fill":"$chart-down","layoutPosition":"absolute","x":291.1,"y":119.7},
        {"type":"rectangle","id":"chart-candle-14-body","name":"Body, down","width":13,"height":11.7,"fill":"$chart-down","layoutPosition":"absolute","x":285.1,"y":146.3},
        {"type":"rectangle","id":"chart-candle-15-wick","name":"Wick","width":1,"height":23.4,"fill":"$chart-down","layoutPosition":"absolute","x":312.7,"y":151.3},
        {"type":"rectangle","id":"chart-candle-15-body","name":"Body, down","width":13,"height":15.0,"fill":"$chart-down","layoutPosition":"absolute","x":306.7,"y":156.3},
        {"type":"rectangle","id":"chart-candle-16-wick","name":"Wick","width":1,"height":28.3,"fill":"$chart-down","layoutPosition":"absolute","x":334.3,"y":169.7},
        {"type":"rectangle","id":"chart-candle-16-body","name":"Body, down","width":13,"height":11.6,"fill":"$chart-down","layoutPosition":"absolute","x":328.3,"y":179.7},
        {"type":"rectangle","id":"chart-candle-17-wick","name":"Wick","width":1,"height":38.4,"fill":"$chart-up","layoutPosition":"absolute","x":355.9,"y":151.3},
        {"type":"rectangle","id":"chart-candle-17-body","name":"Body, up","width":13,"height":15.0,"fill":"$chart-up","layoutPosition":"absolute","x":349.9,"y":164.7},
        {"type":"rectangle","id":"chart-candle-18-wick","name":"Wick","width":1,"height":23.3,"fill":"$chart-down","layoutPosition":"absolute","x":377.5,"y":149.7},
        {"type":"rectangle","id":"chart-candle-18-body","name":"Body, down","width":13,"height":10.0,"fill":"$chart-down","layoutPosition":"absolute","x":371.5,"y":156.3},
        {"type":"rectangle","id":"chart-candle-19-wick","name":"Wick","width":1,"height":38.3,"fill":"$chart-up","layoutPosition":"absolute","x":399.1,"y":143.0},
        {"type":"rectangle","id":"chart-candle-19-body","name":"Body, up","width":13,"height":3.3,"fill":"$chart-up","layoutPosition":"absolute","x":393.1,"y":169.7},
        {"type":"rectangle","id":"chart-candle-20-wick","name":"Wick","width":1,"height":50.0,"fill":"$chart-up","layoutPosition":"absolute","x":420.7,"y":136.3},
        {"type":"rectangle","id":"chart-candle-20-body","name":"Body, up","width":13,"height":31.7,"fill":"$chart-up","layoutPosition":"absolute","x":414.7,"y":148.0},
        {"type":"rectangle","id":"chart-candle-last-line","name":"Last price","width":432,"height":1,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":0,"y":148.0}]},
      {"type":"frame","id":"chart-candle-volumes","name":"Volume","width":432,"height":40,"layout":"none","children":[
        {"type":"rectangle","id":"chart-candle-1-volume","name":"Volume","width":13,"height":40,"fill":{"type":"color","color":"$chart-down","opacity":0.5},"layoutPosition":"absolute","x":4.3,"y":0},
        {"type":"rectangle","id":"chart-candle-2-volume","name":"Volume","width":13,"height":32,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":25.9,"y":8},
        {"type":"rectangle","id":"chart-candle-3-volume","name":"Volume","width":13,"height":16,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":47.5,"y":24},
        {"type":"rectangle","id":"chart-candle-4-volume","name":"Volume","width":13,"height":35,"fill":{"type":"color","color":"$chart-down","opacity":0.5},"layoutPosition":"absolute","x":69.1,"y":5},
        {"type":"rectangle","id":"chart-candle-5-volume","name":"Volume","width":13,"height":30,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":90.7,"y":10},
        {"type":"rectangle","id":"chart-candle-6-volume","name":"Volume","width":13,"height":33,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":112.3,"y":7},
        {"type":"rectangle","id":"chart-candle-7-volume","name":"Volume","width":13,"height":24,"fill":{"type":"color","color":"$chart-down","opacity":0.5},"layoutPosition":"absolute","x":133.9,"y":16},
        {"type":"rectangle","id":"chart-candle-8-volume","name":"Volume","width":13,"height":33,"fill":{"type":"color","color":"$chart-down","opacity":0.5},"layoutPosition":"absolute","x":155.5,"y":7},
        {"type":"rectangle","id":"chart-candle-9-volume","name":"Volume","width":13,"height":24,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":177.1,"y":16},
        {"type":"rectangle","id":"chart-candle-10-volume","name":"Volume","width":13,"height":37,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":198.7,"y":3},
        {"type":"rectangle","id":"chart-candle-11-volume","name":"Volume","width":13,"height":27,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":220.3,"y":13},
        {"type":"rectangle","id":"chart-candle-12-volume","name":"Volume","width":13,"height":25,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":241.9,"y":15},
        {"type":"rectangle","id":"chart-candle-13-volume","name":"Volume","width":13,"height":28,"fill":{"type":"color","color":"$chart-down","opacity":0.5},"layoutPosition":"absolute","x":263.5,"y":12},
        {"type":"rectangle","id":"chart-candle-14-volume","name":"Volume","width":13,"height":21,"fill":{"type":"color","color":"$chart-down","opacity":0.5},"layoutPosition":"absolute","x":285.1,"y":19},
        {"type":"rectangle","id":"chart-candle-15-volume","name":"Volume","width":13,"height":30,"fill":{"type":"color","color":"$chart-down","opacity":0.5},"layoutPosition":"absolute","x":306.7,"y":10},
        {"type":"rectangle","id":"chart-candle-16-volume","name":"Volume","width":13,"height":22,"fill":{"type":"color","color":"$chart-down","opacity":0.5},"layoutPosition":"absolute","x":328.3,"y":18},
        {"type":"rectangle","id":"chart-candle-17-volume","name":"Volume","width":13,"height":25,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":349.9,"y":15},
        {"type":"rectangle","id":"chart-candle-18-volume","name":"Volume","width":13,"height":35,"fill":{"type":"color","color":"$chart-down","opacity":0.5},"layoutPosition":"absolute","x":371.5,"y":5},
        {"type":"rectangle","id":"chart-candle-19-volume","name":"Volume","width":13,"height":17,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":393.1,"y":23},
        {"type":"rectangle","id":"chart-candle-20-volume","name":"Volume","width":13,"height":32,"fill":{"type":"color","color":"$chart-up","opacity":0.5},"layoutPosition":"absolute","x":414.7,"y":8}]},
      {"type":"frame","id":"chart-candle-x","name":"Time axis","width":432,"height":16,"layout":"none","children":[
        {"type":"text","id":"chart-candle-x-1","content":"26 Oct","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","layoutPosition":"absolute","x":4.3,"y":0},
        {"type":"text","id":"chart-candle-x-6","content":"2 Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","layoutPosition":"absolute","x":112.3,"y":0},
        {"type":"text","id":"chart-candle-x-11","content":"9 Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","layoutPosition":"absolute","x":220.3,"y":0},
        {"type":"text","id":"chart-candle-x-16","content":"16 Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","layoutPosition":"absolute","x":328.3,"y":0}]}]},
    {"type":"frame","id":"chart-candle-y","name":"Price axis","width":48,"height":216,"layout":"vertical","justifyContent":"space_between","children":[
      {"type":"text","id":"chart-candle-y-0","content":"86.00","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"lineHeight":1.3333},
      {"type":"text","id":"chart-candle-y-1","content":"84.00","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"lineHeight":1.3333},
      {"type":"text","id":"chart-candle-y-2","content":"82.00","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"lineHeight":1.3333,"opacity":0},
      {"type":"text","id":"chart-candle-y-3","content":"80.00","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":48,"lineHeight":1.3333},
      {"type":"frame","id":"chart-candle-tag","name":"Last price","height":18,"padding":[0,4],"alignItems":"center","cornerRadius":2,"fill":"$foreground","layoutPosition":"absolute","x":-4,"y":139.0,"children":[
        {"type":"text","id":"chart-candle-tag-text","content":"81.80","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$background"}]}]}]}]}
```

- A tick label that the last-price tag would touch gets `"opacity":0`: the tag takes its place. (It stays
  in the axis so the other labels keep their places.)
- A day that closed where it opened has a body 1 high: a line across its wick.
- Trading apps often draw up candles hollow (an outline of 1 in `$chart-up`, no fill) so that direction
  does not hang on colour alone; do it for all up candles or none.
- A range switch (1D, 1W, 1M, 1Y) is the tab list of the ui-components skill over the plot's trailing
  edge; the candles then stand for the period it names (an hour, a day, a week).
- A moving average is a 1 wide line in `$chart-muted` through the closes, named at its end.
- On a phone: 12 to 16 candles across the screen, the volume strip 32 high, the header over the plot.

## Legend

```json
{"type":"frame","id":"chart-legend","name":"Legend","gap":16,"alignItems":"center","children":[
  {"type":"frame","id":"chart-legend-1","name":"Item","gap":6,"alignItems":"center","children":[
    {"type":"rectangle","id":"chart-legend-1-swatch","width":8,"height":8,"fill":"$chart-1","cornerRadius":2},
    {"type":"text","id":"chart-legend-1-label","content":"Online","fontFamily":"$font","fontSize":12,"fill":"$foreground"}]},
  {"type":"frame","id":"chart-legend-2","name":"Item","gap":6,"alignItems":"center","children":[
    {"type":"rectangle","id":"chart-legend-2-swatch","width":8,"height":8,"fill":"$chart-2","cornerRadius":2},
    {"type":"text","id":"chart-legend-2-label","content":"At the desk","fontFamily":"$font","fontSize":12,"fill":"$foreground"}]}]}
```

For lines the mark is a dash 14 wide and 3 high in place of the square.

## Tooltip

For one moment of use: the bar or point under the pointer keeps its colour, the others drop to 40% of
theirs (`{"type":"color","color":"$chart-1","opacity":0.4}`), and the tooltip stands 8 beside it, inside
the plot. Its values are the ones drawn.

```json
{"type":"frame","id":"chart-tooltip","name":"Tooltip","width":152,"layout":"vertical","gap":6,"padding":[6,10],"cornerRadius":10,"fill":"$card","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","effect":[{"type":"shadow","offset":{"x":0,"y":2},"blur":6,"color":"#0000000f"},{"type":"shadow","offset":{"x":0,"y":12},"blur":28,"color":"#0000001f"}],"children":[
  {"type":"text","id":"chart-tooltip-title","name":"Label","content":"July 2026","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground"},
  {"type":"frame","id":"chart-tooltip-1","name":"Row","width":"fill_container","gap":8,"alignItems":"center","children":[
    {"type":"rectangle","id":"chart-tooltip-1-swatch","width":10,"height":10,"fill":"$chart-1","cornerRadius":2},
    {"type":"text","id":"chart-tooltip-1-label","content":"Online","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"chart-tooltip-1-value","content":"3,810","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"chart-tooltip-2","name":"Row","width":"fill_container","gap":8,"alignItems":"center","children":[
    {"type":"rectangle","id":"chart-tooltip-2-swatch","width":10,"height":10,"fill":"$chart-2","cornerRadius":2},
    {"type":"text","id":"chart-tooltip-2-label","content":"At the desk","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"chart-tooltip-2-value","content":"1,430","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"chart-tooltip-total","name":"Total","width":"fill_container","gap":8,"alignItems":"center","children":[
    {"type":"text","id":"chart-tooltip-total-label","content":"Total","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"chart-tooltip-total-value","content":"5,240","fontFamily":"$font","fontSize":12,"fontWeight":"500","fill":"$foreground"}]}]}
```

## Chart card

The title says what the chart shows; the line under it what was measured. The footer's first line is a
change worked out from the bars (5,240 in July to 3,140 in November is 40% less), its second what was
counted (the six bars sum to 23,960).

```json
{"type":"frame","id":"chart-card","name":"Chart card","width":528,"layout":"vertical","gap":24,"padding":24,"cornerRadius":14,"fill":"$card","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"chart-card-header","name":"Header","width":"fill_container","layout":"vertical","gap":4,"children":[
    {"type":"text","id":"chart-card-title","name":"Title","content":"Ticket sales peaked in July","fontFamily":"$font","fontSize":16,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"chart-card-description","name":"Description","content":"Tickets sold per month, June to November 2026","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"frame","id":"chart-card-bar","name":"Bar chart","layout":"vertical","gap":12,"children":[
    {"type":"frame","id":"chart-card-bar-body","name":"Body","gap":8,"children":[
      {"type":"frame","id":"chart-card-bar-y","name":"Y axis","width":40,"height":216,"layout":"vertical","justifyContent":"space_between","children":[
        {"type":"text","id":"chart-card-bar-y-0","content":"6,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
        {"type":"text","id":"chart-card-bar-y-1","content":"4,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
        {"type":"text","id":"chart-card-bar-y-2","content":"2,000","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333},
        {"type":"text","id":"chart-card-bar-y-3","content":"0","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":40,"textAlign":"right","lineHeight":1.3333}]},
      {"type":"frame","id":"chart-card-bar-main","layout":"vertical","gap":2,"children":[
        {"type":"frame","id":"chart-card-bar-plot","name":"Plot","width":432,"height":216,"padding":[8,12],"gap":24,"alignItems":"end","children":[
          {"type":"rectangle","id":"chart-card-bar-grid-0","name":"Baseline","width":432,"height":1,"fill":"$border","layoutPosition":"absolute","x":0,"y":208},
          {"type":"rectangle","id":"chart-card-bar-grid-1","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":141},
          {"type":"rectangle","id":"chart-card-bar-grid-2","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":75},
          {"type":"rectangle","id":"chart-card-bar-grid-3","name":"Gridline","width":432,"height":1,"fill":"$chart-grid","layoutPosition":"absolute","x":0,"y":8},
          {"type":"rectangle","id":"chart-card-bar-bar-1-1-1","width":"fill_container","height":139,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-card-bar-bar-2-1-1","width":"fill_container","height":175,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-card-bar-bar-3-1-1","width":"fill_container","height":164,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-card-bar-bar-4-1-1","width":"fill_container","height":121,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-card-bar-bar-5-1-1","width":"fill_container","height":96,"fill":"$chart-1","cornerRadius":[4,4,0,0]},
          {"type":"rectangle","id":"chart-card-bar-bar-6-1-1","width":"fill_container","height":105,"fill":"$chart-1","cornerRadius":[4,4,0,0]}]},
        {"type":"frame","id":"chart-card-bar-x","name":"X axis","width":432,"padding":[0,12],"gap":24,"children":[
          {"type":"frame","id":"chart-card-bar-x-1","width":"fill_container","justifyContent":"center","children":[
            {"type":"text","id":"chart-card-bar-x-1-label","content":"Jun","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"chart-card-bar-x-2","width":"fill_container","justifyContent":"center","children":[
            {"type":"text","id":"chart-card-bar-x-2-label","content":"Jul","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"chart-card-bar-x-3","width":"fill_container","justifyContent":"center","children":[
            {"type":"text","id":"chart-card-bar-x-3-label","content":"Aug","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"chart-card-bar-x-4","width":"fill_container","justifyContent":"center","children":[
            {"type":"text","id":"chart-card-bar-x-4-label","content":"Sep","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"chart-card-bar-x-5","width":"fill_container","justifyContent":"center","children":[
            {"type":"text","id":"chart-card-bar-x-5-label","content":"Oct","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"chart-card-bar-x-6","width":"fill_container","justifyContent":"center","children":[
            {"type":"text","id":"chart-card-bar-x-6-label","content":"Nov","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]}]}]}]}]},
  {"type":"frame","id":"chart-card-footer","name":"Footer","width":"fill_container","layout":"vertical","gap":4,"children":[
    {"type":"frame","id":"chart-card-note-row","gap":8,"alignItems":"center","children":[
      {"type":"text","id":"chart-card-note","name":"Note","content":"Down 40% from the July peak","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
      {"type":"icon","id":"chart-card-note-icon","library":"lucide","icon":"trending-down","width":16,"height":16,"fill":"$foreground"}]},
    {"type":"text","id":"chart-card-detail","name":"Detail","content":"23,960 tickets in six months","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]}]}
```
