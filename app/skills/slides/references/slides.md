# Slides, ready to change

Each block is one slide, 1920 by 1080, to insert with `edit_layers` (`{"op":"insert","node":<block>}`) at
the top level, 96 beside the one before it. Ids start with `slide-`; a deck with two slides of one kind
gives the second ids of its own. The first eight are one presented deck: a harbour master's proposal to her
board. Change every word: the content is there to show the sizes and the kinds, and a slide copied with
its words is a template. Change the look as well: these grounds, faces and placements are one deck's
direction, and another deck should have its own.

Its titles, in order, are the argument:

1. The rota should not take a Sunday.
2. Filling next week's shifts takes 38 minutes on the phone. Every Thursday.
3. Open shifts are found too late.
4. Most calls go to people who could never have taken the shift.
5. The same week, planned two ways.
6. "The rota is out on Thursday now, and nobody calls me about it."
7. A four-week trial costs nothing and starts with one desk.
8. One decision today: start the trial on Monday 7 December.

## A set of variables to start from

Two grounds (`paper` and its opposite, `ink`), a softer ink for each, a line, one accent, two faces.
Replace every value with the direction you chose. The display face here ships with Windows; set both to
families that are installed.

```json
{"op":"set_variables",
 "variables":{
  "paper":{"type":"color","value":"#f4f7f6"},
  "ink":{"type":"color","value":"#0f2a2e"},
  "ink-soft":{"type":"color","value":"#47595c"},
  "on-ink-soft":{"type":"color","value":"#a9bcbf"},
  "line":{"type":"color","value":"#0f2a2e33"},
  "accent":{"type":"color","value":"#f2c230"},
  "font-display":{"type":"string","value":"Bahnschrift"},
  "font":{"type":"string","value":"Segoe UI Variable"}}}
```

Every slide has the same padding (80 on top, 96 at the sides, 56 at the bottom) and the same footer, and is
a vertical frame with `"justifyContent":"space_between"`: what leads at the top, what closes at the bottom.

## Title

The deck's claim, large, low on the slide; who is speaking above it, and one line with what this is, when
and by whom.

```json
{"type":"frame","id":"slide-title","name":"01 Title","width":1920,"height":1080,"layout":"vertical","padding":[80,96,56,96],"justifyContent":"space_between","fill":"$paper","clip":true,"children":[
  {"type":"text","id":"slide-title-who","name":"Who","content":"Sotra Havn","fontFamily":"$font-display","fontSize":40,"fontWeight":"600","fill":"$ink"},
  {"type":"frame","id":"slide-title-main","width":"fill_container","layout":"vertical","gap":40,"children":[
    {"type":"text","id":"slide-title-headline","name":"Headline","content":"The rota should not take a Sunday.","fontFamily":"$font-display","fontSize":136,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":1400,"lineHeight":0.98,"letterSpacing":-3},
    {"type":"text","id":"slide-title-line","name":"What, when, who","content":"A proposal to the harbour board · Thursday 3 December 2026 · Turid Hauge","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"frame","id":"slide-title-foot","name":"Footer","width":"fill_container","justifyContent":"space_between","alignItems":"end","children":[
    {"type":"text","id":"slide-title-foot-deck","content":"Sotra Havn · The rota","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"},
    {"type":"text","id":"slide-title-foot-page","name":"Page","content":"1","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"}]}]}
```

## Statement

One sentence carries the slide. Under it, where the figure comes from.

```json
{"type":"frame","id":"slide-statement","name":"02 Statement","width":1920,"height":1080,"layout":"vertical","padding":[80,96,56,96],"justifyContent":"space_between","fill":"$paper","clip":true,"children":[
  {"type":"text","id":"slide-statement-text","name":"Statement","content":"Filling next week's shifts takes 38 minutes on the phone. Every Thursday.","fontFamily":"$font-display","fontSize":112,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":1560,"lineHeight":1.02,"letterSpacing":-2},
  {"type":"text","id":"slide-statement-note","name":"Where it comes from","content":"Measured over the eight weeks from 5 October.","fontFamily":"$font","fontSize":44,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"},
  {"type":"frame","id":"slide-statement-foot","name":"Footer","width":"fill_container","justifyContent":"space_between","alignItems":"end","children":[
    {"type":"text","id":"slide-statement-foot-deck","content":"Sotra Havn · The rota","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"},
    {"type":"text","id":"slide-statement-foot-page","name":"Page","content":"2","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"}]}]}
```

## Big number

On the opposite ground. The title says what the figure means; the figure is in the accent with, beside it,
what it counts.

```json
{"type":"frame","id":"slide-number","name":"03 Big number","width":1920,"height":1080,"layout":"vertical","padding":[80,96,56,96],"justifyContent":"space_between","fill":"$ink","clip":true,"children":[
  {"type":"text","id":"slide-number-title","name":"Title","content":"Open shifts are found too late.","fontFamily":"$font-display","fontSize":64,"fontWeight":"600","fill":"$paper","textGrowth":"fixed-width","width":1500,"lineHeight":1.08,"letterSpacing":-1},
  {"type":"frame","id":"slide-number-main","width":"fill_container","gap":72,"alignItems":"end","children":[
    {"type":"text","id":"slide-number-value","name":"Figure","content":"14","fontFamily":"$font-display","fontSize":420,"fontWeight":"600","fill":"$accent","lineHeight":0.9,"letterSpacing":-12},
    {"type":"text","id":"slide-number-label","name":"What it counts","content":"shifts last quarter were filled the night before, or not at all.","fontFamily":"$font","fontSize":64,"fill":"$paper","textGrowth":"fixed-width","width":760,"lineHeight":1.15}]},
  {"type":"frame","id":"slide-number-foot","name":"Footer","width":"fill_container","justifyContent":"space_between","alignItems":"end","children":[
    {"type":"text","id":"slide-number-foot-deck","content":"Sotra Havn · The rota","fontFamily":"$font","fontSize":24,"fill":"$on-ink-soft"},
    {"type":"text","id":"slide-number-foot-page","name":"Page","content":"3","fontFamily":"$font","fontSize":24,"fill":"$on-ink-soft"}]}]}
```

## Evidence

The title is what the chart shows (3.4 of the 6 calls). Four bars, worked out from their values (the longest, 2.1, is 760 wide;
the others `value / 2.1 * 760`), sorted, with the one that matters in the accent; the values add up to the
six calls the source line names. Labels are 40.

```json
{"type":"frame","id":"slide-chart","name":"04 Evidence","width":1920,"height":1080,"layout":"vertical","padding":[80,96,56,96],"justifyContent":"space_between","fill":"$paper","clip":true,"children":[
  {"type":"text","id":"slide-chart-title","name":"Title","content":"Most calls go to people who could never have taken the shift.","fontFamily":"$font-display","fontSize":64,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":1500,"lineHeight":1.08,"letterSpacing":-1},
  {"type":"frame","id":"slide-chart-bars","name":"Chart","layout":"vertical","gap":36,"children":[
    {"type":"frame","id":"slide-chart-bar-1","name":"Row","gap":32,"alignItems":"center","children":[
      {"type":"text","id":"slide-chart-bar-1-label","content":"Could not take it: rest time too short","fontFamily":"$font","fontSize":40,"fill":"$ink","textGrowth":"fixed-width","width":800},
      {"type":"frame","id":"slide-chart-bar-1-track","width":760,"height":48,"children":[
        {"type":"rectangle","id":"slide-chart-bar-1-mark","name":"Bar","width":760,"height":48,"fill":"$ink","cornerRadius":6}]},
      {"type":"text","id":"slide-chart-bar-1-value","content":"2.1","fontFamily":"$font","fontSize":40,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":88,"textAlign":"right"}]},
    {"type":"frame","id":"slide-chart-bar-2","name":"Row","gap":32,"alignItems":"center","children":[
      {"type":"text","id":"slide-chart-bar-2-label","content":"Did not answer","fontFamily":"$font","fontSize":40,"fill":"$ink","textGrowth":"fixed-width","width":800},
      {"type":"frame","id":"slide-chart-bar-2-track","width":760,"height":48,"children":[
        {"type":"rectangle","id":"slide-chart-bar-2-mark","name":"Bar","width":579,"height":48,"fill":"$ink","cornerRadius":6}]},
      {"type":"text","id":"slide-chart-bar-2-value","content":"1.6","fontFamily":"$font","fontSize":40,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":88,"textAlign":"right"}]},
    {"type":"frame","id":"slide-chart-bar-3","name":"Row","gap":32,"alignItems":"center","children":[
      {"type":"text","id":"slide-chart-bar-3-label","content":"No licence for that desk","fontFamily":"$font","fontSize":40,"fill":"$ink","textGrowth":"fixed-width","width":800},
      {"type":"frame","id":"slide-chart-bar-3-track","width":760,"height":48,"children":[
        {"type":"rectangle","id":"slide-chart-bar-3-mark","name":"Bar","width":470,"height":48,"fill":"$ink","cornerRadius":6}]},
      {"type":"text","id":"slide-chart-bar-3-value","content":"1.3","fontFamily":"$font","fontSize":40,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":88,"textAlign":"right"}]},
    {"type":"frame","id":"slide-chart-bar-4","name":"Row","gap":32,"alignItems":"center","children":[
      {"type":"text","id":"slide-chart-bar-4-label","content":"Took the shift","fontFamily":"$font","fontSize":40,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":800},
      {"type":"frame","id":"slide-chart-bar-4-track","width":760,"height":48,"children":[
        {"type":"rectangle","id":"slide-chart-bar-4-mark","name":"Bar","width":362,"height":48,"fill":"$accent","cornerRadius":6}]},
      {"type":"text","id":"slide-chart-bar-4-value","content":"1.0","fontFamily":"$font","fontSize":40,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":88,"textAlign":"right"}]}]},
  {"type":"text","id":"slide-chart-source","name":"Source","content":"Calls made for one open shift, average of 22 shifts in October. Six calls for one yes.","fontFamily":"$font","fontSize":32,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"},
  {"type":"frame","id":"slide-chart-foot","name":"Footer","width":"fill_container","justifyContent":"space_between","alignItems":"end","children":[
    {"type":"text","id":"slide-chart-foot-deck","content":"Sotra Havn · The rota","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"},
    {"type":"text","id":"slide-chart-foot-page","name":"Page","content":"4","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"}]}]}
```

## Comparison

Two columns with the same three rows, each row named in both. The side being argued for stands on the
accent and is set heavier.

```json
{"type":"frame","id":"slide-compare","name":"05 Comparison","width":1920,"height":1080,"layout":"vertical","padding":[80,96,56,96],"justifyContent":"space_between","fill":"$paper","clip":true,"children":[
  {"type":"text","id":"slide-compare-title","name":"Title","content":"The same week, planned two ways.","fontFamily":"$font-display","fontSize":64,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":1500,"lineHeight":1.08,"letterSpacing":-1},
  {"type":"frame","id":"slide-compare-columns","name":"Columns","width":"fill_container","gap":48,"children":[
    {"type":"frame","id":"slide-compare-today","name":"Column","width":"fill_container","layout":"vertical","gap":24,"padding":[40,48,24,0],"children":[
      {"type":"text","id":"slide-compare-today-head","content":"Today","fontFamily":"$font-display","fontSize":40,"fontWeight":"600","fill":"$ink-soft"},
      {"type":"frame","id":"slide-compare-today-1","name":"Row","width":"fill_container","height":168,"layout":"vertical","gap":8,"justifyContent":"center","stroke":"$line","strokeWidth":{"top":2},"children":[
        {"type":"text","id":"slide-compare-today-1-label","content":"Who can take a shift","fontFamily":"$font","fontSize":32,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"slide-compare-today-1-text","content":"The harbour master remembers","fontFamily":"$font","fontSize":48,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.15}]},
      {"type":"frame","id":"slide-compare-today-2","name":"Row","width":"fill_container","height":168,"layout":"vertical","gap":8,"justifyContent":"center","stroke":"$line","strokeWidth":{"top":2},"children":[
        {"type":"text","id":"slide-compare-today-2-label","content":"When the rota is out","fontFamily":"$font","fontSize":32,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"slide-compare-today-2-text","content":"Sunday evening","fontFamily":"$font","fontSize":48,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.15}]},
      {"type":"frame","id":"slide-compare-today-3","name":"Row","width":"fill_container","height":168,"layout":"vertical","gap":8,"justifyContent":"center","stroke":"$line","strokeWidth":{"top":2},"children":[
        {"type":"text","id":"slide-compare-today-3-label","content":"A swap between two people","fontFamily":"$font","fontSize":32,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"slide-compare-today-3-text","content":"Three phone calls","fontFamily":"$font","fontSize":48,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.15}]}]},
    {"type":"frame","id":"slide-compare-with","name":"Column","width":"fill_container","layout":"vertical","gap":24,"padding":[40,48,24,48],"cornerRadius":24,"fill":"$accent","children":[
      {"type":"text","id":"slide-compare-with-head","content":"With Harbour Rota","fontFamily":"$font-display","fontSize":40,"fontWeight":"600","fill":"$ink"},
      {"type":"frame","id":"slide-compare-with-1","name":"Row","width":"fill_container","height":168,"layout":"vertical","gap":8,"justifyContent":"center","stroke":"$line","strokeWidth":{"top":2},"children":[
        {"type":"text","id":"slide-compare-with-1-label","content":"Who can take a shift","fontFamily":"$font","fontSize":32,"fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"slide-compare-with-1-text","content":"The rota checks rest, licence and hours","fontFamily":"$font","fontSize":48,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.15}]},
      {"type":"frame","id":"slide-compare-with-2","name":"Row","width":"fill_container","height":168,"layout":"vertical","gap":8,"justifyContent":"center","stroke":"$line","strokeWidth":{"top":2},"children":[
        {"type":"text","id":"slide-compare-with-2-label","content":"When the rota is out","fontFamily":"$font","fontSize":32,"fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"slide-compare-with-2-text","content":"Thursday","fontFamily":"$font","fontSize":48,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.15}]},
      {"type":"frame","id":"slide-compare-with-3","name":"Row","width":"fill_container","height":168,"layout":"vertical","gap":8,"justifyContent":"center","stroke":"$line","strokeWidth":{"top":2},"children":[
        {"type":"text","id":"slide-compare-with-3-label","content":"A swap between two people","fontFamily":"$font","fontSize":32,"fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"slide-compare-with-3-text","content":"One yes or no","fontFamily":"$font","fontSize":48,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.15}]}]}]},
  {"type":"frame","id":"slide-compare-foot","name":"Footer","width":"fill_container","justifyContent":"space_between","alignItems":"end","children":[
    {"type":"text","id":"slide-compare-foot-deck","content":"Sotra Havn · The rota","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"},
    {"type":"text","id":"slide-compare-foot-page","name":"Page","content":"5","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"}]}]}
```

## Quote

On the opposite ground. A real person, with role, place and since when.

```json
{"type":"frame","id":"slide-quote","name":"06 Quote","width":1920,"height":1080,"layout":"vertical","padding":[80,96,56,96],"justifyContent":"space_between","fill":"$ink","clip":true,"children":[
  {"type":"text","id":"slide-quote-text","name":"Quote","content":"“The rota is out on Thursday now, and nobody calls me about it.”","fontFamily":"$font-display","fontSize":104,"fontWeight":"500","fill":"$paper","textGrowth":"fixed-width","width":1560,"lineHeight":1.08,"letterSpacing":-2},
  {"type":"frame","id":"slide-quote-who","name":"Who","gap":32,"alignItems":"center","children":[
    {"type":"frame","id":"slide-quote-avatar","width":96,"height":96,"justifyContent":"center","alignItems":"center","cornerRadius":48,"fill":"$accent","children":[
      {"type":"text","id":"slide-quote-initials","content":"EN","fontFamily":"$font","fontSize":36,"fontWeight":"600","fill":"$ink"}]},
    {"type":"frame","id":"slide-quote-name-box","layout":"vertical","gap":4,"children":[
      {"type":"text","id":"slide-quote-name","content":"Eirik Nes","fontFamily":"$font","fontSize":48,"fontWeight":"600","fill":"$paper"},
      {"type":"text","id":"slide-quote-role","content":"Harbour master, Askøy Ferjer, on Harbour Rota since March","fontFamily":"$font","fontSize":40,"fill":"$on-ink-soft"}]}]},
  {"type":"frame","id":"slide-quote-foot","name":"Footer","width":"fill_container","justifyContent":"space_between","alignItems":"end","children":[
    {"type":"text","id":"slide-quote-foot-deck","content":"Sotra Havn · The rota","fontFamily":"$font","fontSize":24,"fill":"$on-ink-soft"},
    {"type":"text","id":"slide-quote-foot-page","name":"Page","content":"6","fontFamily":"$font","fontSize":24,"fill":"$on-ink-soft"}]}]}
```

## Plan

Three steps with their dates; a bar over each, the last in the accent because it is the decision. The
dates are real ones: 7 December 2026 is a Monday, 7 January 2027 a Thursday.

```json
{"type":"frame","id":"slide-steps","name":"07 Plan","width":1920,"height":1080,"layout":"vertical","padding":[80,96,56,96],"justifyContent":"space_between","fill":"$paper","clip":true,"children":[
  {"type":"text","id":"slide-steps-title","name":"Title","content":"A four-week trial costs nothing and starts with one desk.","fontFamily":"$font-display","fontSize":64,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":1500,"lineHeight":1.08,"letterSpacing":-1},
  {"type":"frame","id":"slide-steps-row","name":"Steps","width":"fill_container","gap":56,"children":[
    {"type":"frame","id":"slide-steps-1","name":"Step","width":"fill_container","layout":"vertical","gap":20,"children":[
      {"type":"rectangle","id":"slide-steps-1-bar","width":"fill_container","height":12,"fill":"$ink","cornerRadius":6},
      {"type":"text","id":"slide-steps-1-when","name":"When","content":"7 to 13 December","fontFamily":"$font","fontSize":40,"fill":"$ink-soft"},
      {"type":"text","id":"slide-steps-1-what","name":"What","content":"The ferry desk only","fontFamily":"$font-display","fontSize":64,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.05},
      {"type":"text","id":"slide-steps-1-note","name":"Note","content":"Paper stays as it is for the other desks.","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.25}]},
    {"type":"frame","id":"slide-steps-2","name":"Step","width":"fill_container","layout":"vertical","gap":20,"children":[
      {"type":"rectangle","id":"slide-steps-2-bar","width":"fill_container","height":12,"fill":"$ink","cornerRadius":6},
      {"type":"text","id":"slide-steps-2-when","name":"When","content":"14 to 27 December","fontFamily":"$font","fontSize":40,"fill":"$ink-soft"},
      {"type":"text","id":"slide-steps-2-what","name":"What","content":"Every desk","fontFamily":"$font-display","fontSize":64,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.05},
      {"type":"text","id":"slide-steps-2-note","name":"Note","content":"Paper is kept as a backup over Christmas.","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.25}]},
    {"type":"frame","id":"slide-steps-3","name":"Step","width":"fill_container","layout":"vertical","gap":20,"children":[
      {"type":"rectangle","id":"slide-steps-3-bar","width":"fill_container","height":12,"fill":"$accent","cornerRadius":6},
      {"type":"text","id":"slide-steps-3-when","name":"When","content":"Thursday 7 January","fontFamily":"$font","fontSize":40,"fill":"$ink-soft"},
      {"type":"text","id":"slide-steps-3-what","name":"What","content":"The board decides","fontFamily":"$font-display","fontSize":64,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.05},
      {"type":"text","id":"slide-steps-3-note","name":"Note","content":"With the hours and the calls counted.","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.25}]}]},
  {"type":"frame","id":"slide-steps-foot","name":"Footer","width":"fill_container","justifyContent":"space_between","alignItems":"end","children":[
    {"type":"text","id":"slide-steps-foot-deck","content":"Sotra Havn · The rota","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"},
    {"type":"text","id":"slide-steps-foot-page","name":"Page","content":"7","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"}]}]}
```

## The ask

On the accent: the one thing the room should do, and under it what it costs. Nothing else.

```json
{"type":"frame","id":"slide-ask","name":"08 The ask","width":1920,"height":1080,"layout":"vertical","padding":[80,96,56,96],"justifyContent":"space_between","fill":"$accent","clip":true,"children":[
  {"type":"text","id":"slide-ask-text","name":"The ask","content":"One decision today: start the trial on Monday 7 December.","fontFamily":"$font-display","fontSize":120,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":1560,"lineHeight":1.0,"letterSpacing":-3},
  {"type":"frame","id":"slide-ask-facts","name":"What it costs","gap":120,"children":[
    {"type":"frame","id":"slide-ask-fact-1","layout":"vertical","gap":4,"children":[
      {"type":"text","id":"slide-ask-fact-1-value","content":"NOK 0","fontFamily":"$font-display","fontSize":72,"fontWeight":"600","fill":"$ink"},
      {"type":"text","id":"slide-ask-fact-1-label","content":"for the four weeks","fontFamily":"$font","fontSize":40,"fill":"$ink"}]},
    {"type":"frame","id":"slide-ask-fact-2","layout":"vertical","gap":4,"children":[
      {"type":"text","id":"slide-ask-fact-2-value","content":"NOK 4,900","fontFamily":"$font-display","fontSize":72,"fontWeight":"600","fill":"$ink"},
      {"type":"text","id":"slide-ask-fact-2-label","content":"a year after that, for the whole harbour","fontFamily":"$font","fontSize":40,"fill":"$ink"}]}]},
  {"type":"frame","id":"slide-ask-foot","name":"Footer","width":"fill_container","justifyContent":"space_between","alignItems":"end","children":[
    {"type":"text","id":"slide-ask-foot-deck","content":"Sotra Havn · The rota","fontFamily":"$font","fontSize":24,"fill":"$ink"},
    {"type":"text","id":"slide-ask-foot-page","name":"Page","content":"8","fontFamily":"$font","fontSize":24,"fill":"$ink"}]}]}
```

## The same evidence, for a deck that is read

For a file that is sent, the slide must explain itself: the title carries the "because", the chart is
smaller with its full source, and the reading beside it says what follows. Text is 30, the source 22.

```json
{"type":"frame","id":"slide-read","name":"Reading slide: evidence","width":1920,"height":1080,"layout":"vertical","padding":[80,96,56,96],"justifyContent":"space_between","fill":"$paper","clip":true,"children":[
  {"type":"text","id":"slide-read-title","name":"Title","content":"Most calls go to people who could never have taken the shift, because the rules are checked by memory.","fontFamily":"$font-display","fontSize":48,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":1560,"lineHeight":1.12,"letterSpacing":-0.5},
  {"type":"frame","id":"slide-read-body","name":"Body","width":"fill_container","gap":72,"children":[
    {"type":"frame","id":"slide-read-chart","name":"Chart","width":1000,"layout":"vertical","gap":40,"children":[
      {"type":"frame","id":"slide-read-bars","name":"Chart","layout":"vertical","gap":24,"children":[
        {"type":"frame","id":"slide-read-bar-1","name":"Row","gap":22,"alignItems":"center","children":[
          {"type":"text","id":"slide-read-bar-1-label","content":"Could not take it: rest time too short","fontFamily":"$font","fontSize":28,"fill":"$ink","textGrowth":"fixed-width","width":560},
          {"type":"frame","id":"slide-read-bar-1-track","width":300,"height":32,"children":[
            {"type":"rectangle","id":"slide-read-bar-1-mark","name":"Bar","width":300,"height":32,"fill":"$ink","cornerRadius":6}]},
          {"type":"text","id":"slide-read-bar-1-value","content":"2.1","fontFamily":"$font","fontSize":28,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":62,"textAlign":"right"}]},
        {"type":"frame","id":"slide-read-bar-2","name":"Row","gap":22,"alignItems":"center","children":[
          {"type":"text","id":"slide-read-bar-2-label","content":"Did not answer","fontFamily":"$font","fontSize":28,"fill":"$ink","textGrowth":"fixed-width","width":560},
          {"type":"frame","id":"slide-read-bar-2-track","width":300,"height":32,"children":[
            {"type":"rectangle","id":"slide-read-bar-2-mark","name":"Bar","width":229,"height":32,"fill":"$ink","cornerRadius":6}]},
          {"type":"text","id":"slide-read-bar-2-value","content":"1.6","fontFamily":"$font","fontSize":28,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":62,"textAlign":"right"}]},
        {"type":"frame","id":"slide-read-bar-3","name":"Row","gap":22,"alignItems":"center","children":[
          {"type":"text","id":"slide-read-bar-3-label","content":"No licence for that desk","fontFamily":"$font","fontSize":28,"fill":"$ink","textGrowth":"fixed-width","width":560},
          {"type":"frame","id":"slide-read-bar-3-track","width":300,"height":32,"children":[
            {"type":"rectangle","id":"slide-read-bar-3-mark","name":"Bar","width":186,"height":32,"fill":"$ink","cornerRadius":6}]},
          {"type":"text","id":"slide-read-bar-3-value","content":"1.3","fontFamily":"$font","fontSize":28,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":62,"textAlign":"right"}]},
        {"type":"frame","id":"slide-read-bar-4","name":"Row","gap":22,"alignItems":"center","children":[
          {"type":"text","id":"slide-read-bar-4-label","content":"Took the shift","fontFamily":"$font","fontSize":28,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":560},
          {"type":"frame","id":"slide-read-bar-4-track","width":300,"height":32,"children":[
            {"type":"rectangle","id":"slide-read-bar-4-mark","name":"Bar","width":143,"height":32,"fill":"$accent","cornerRadius":6}]},
          {"type":"text","id":"slide-read-bar-4-value","content":"1.0","fontFamily":"$font","fontSize":28,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":62,"textAlign":"right"}]}]},
      {"type":"text","id":"slide-read-source","name":"Source","content":"Calls made for one open shift, average of 22 shifts, 1 to 31 October 2026. Source: the harbour office's call log.","fontFamily":"$font","fontSize":22,"fill":"$ink-soft","textGrowth":"fixed-width","width":960,"lineHeight":1.35}]},
    {"type":"frame","id":"slide-read-points","name":"Reading","width":"fill_container","layout":"vertical","gap":32,"children":[
      {"type":"text","id":"slide-read-point-1","content":"Over half of the calls reach someone who is not allowed to work the shift: too little rest, or no licence for that desk.","fontFamily":"$font","fontSize":30,"fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.35},
      {"type":"text","id":"slide-read-point-2","content":"The harbour master cannot know this before calling. The rules are in two binders and one head.","fontFamily":"$font","fontSize":30,"fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.35},
      {"type":"text","id":"slide-read-point-3","content":"A rota that knows rest time and licences would offer only the people who can say yes.","fontFamily":"$font","fontSize":30,"fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.35}]}]},
  {"type":"frame","id":"slide-read-foot","name":"Footer","width":"fill_container","justifyContent":"space_between","alignItems":"end","children":[
    {"type":"text","id":"slide-read-foot-deck","content":"Sotra Havn · The rota","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"},
    {"type":"text","id":"slide-read-foot-page","name":"Page","content":"4","fontFamily":"$font","fontSize":24,"fill":"$ink-soft"}]}]}
```
