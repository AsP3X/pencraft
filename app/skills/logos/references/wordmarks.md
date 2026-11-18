# The wordmark and the lockup

## Choosing the face
Pick from the families installed on this computer (the guide lists them with their weights). Decide by shape,
with the mark beside you:

| The mark is made of | Take a face with | Kinds to look for in the list |
|---|---|---|
| circles and straight bars of one weight | round o, single-storey shapes, even strokes | a geometric sans (Futura, Avenir, Century Gothic, Jost, Poppins) |
| straight bars and right angles, compact | narrow, flat-sided letters | a grotesque or a condensed sans (Helvetica Neue, Akzidenz, DIN, Arial Narrow, Bahnschrift) |
| soft shapes with round ends | round stroke ends | a rounded sans (Arial Rounded, Nunito, SF Pro Rounded, Varela Round) |
| thick and thin strokes, pointed ends | the same contrast | a high-contrast serif (Didot, Bodoni) for fashion and print; a sturdier serif (Baskerville, Garamond, Georgia) for things that last |
| drawn, humane curves | open, calligraphic shapes | a humanist sans (Gill Sans, Optima, Frutiger, Segoe UI, Candara) |

These names are examples; use one only if it is installed, and prefer one you have not used for another logo
in this document. The weight follows the mark's: a heavy mark takes weight 600 to 800 and a mark of fine
lines 300 to 400. The letters' stems stay thinner than the mark's stroke; what has to agree is the
impression, heavy with heavy and light with light. A face is wrong for a wordmark when its letters for this name make an awkward pair (a gap in
"AV", a collision in "rt"): look at the actual name large before you settle.

Not these, unless the user asks: a typewriter or slab face for "heritage", a script for "handmade", a
monospaced face for "tech", a stencil for "industrial", Papyrus, Comic Sans, Copperplate, Impact.

## Case, weight and spacing
`letterSpacing` is in canvas units, added after every letter. As a part of the `fontSize`:
- Capitals: spaced out. 4% to 8% for a bold weight, 8% to 14% for a regular or light one. Wide spacing of
  20% or more is a style of its own: only for a short second line.
- Lowercase or mixed case: 0, or down to -2% for a heavy weight at large size. Never spaced out.
- The text layer adds the spacing after the last letter too, so a centred, spaced name sits half a spacing to
  the left: move it right by half the `letterSpacing`.
One family, one weight, one case for the name. Change one letter only when the idea lives there (a letter
replaced by the mark, a dot in the accent colour), and then exactly once.

## Where capitals sit in a text layer
A text layer's box is taller than its letters. For most faces the baseline lies about 0.95 to 1.0 × the
`fontSize` below the top of the box, capitals are about 0.7 × the `fontSize` tall, and lowercase letters
without ascenders about 0.5. So:
- the middle of the capitals is about 0.63 × `fontSize` below the top of the box;
- the middle of the lowercase x-height is about 0.72 × `fontSize` below it.
Pencraft tells you where the letters really are: `edit_layers` answers with `inkBoxes` for the texts it
touched and `read_layers` gives each text an `inkBox`, the box round its letters as drawn. Set the name by
that, not by its `layoutBox`: for capitals centred on a mark, move the text until the middle of its
`inkBox` (y + height / 2) is the mark's middle; for a gap, measure from the `inkBox`'s left edge. For a
mixed-case name the `inkBox` includes ascenders and descenders, so centre the x-height by the rule below.
Without an `inkBox` (an older Pencraft), these rules of thumb are the start, and faces differ widely (the capitals' middle lies anywhere from 0.55 to 0.8 ×
`fontSize` below the top of the box; Futura sits low in its box and has capitals about 0.76 tall). Look at
`scale: 2`, measure in the picture where the letters really are, and nudge `y` until the letters, not the
box, sit where they should. A name in a stand-in face (the review says when a family is not installed) is
measured again once the face is right.

## Horizontal lockup
With a mark of height H (240 while you construct):
- `fontSize` so the capitals are between H/3 and H/2 tall: between 0.48 × H and 0.7 × H. A long name takes
  the smaller end, a name of three or four letters the larger. H is the mark's height; for a mark much wider
  than it is high go up to 0.95 × H, so the name is not dwarfed.
- The gap between the mark's right edge and the first letter: the capitals' height (0.7 × `fontSize`), a
  little less after a mark that ends in a point or a curve.
- Vertically: an all-capitals name has the capitals' middle on the mark's middle: `y` of the text = the
  mark's middle - 0.63 × `fontSize`. A lowercase or mixed-case name ("Speiche") has its x-height there:
  - 0.72 × `fontSize`. Then check and nudge. If the mark has a strong
  horizontal edge (a bar, a baseline of its own), put the name's baseline or cap line on it instead.
- `textGrowth: "auto"`, so the layer is as wide as the name.

## Stacked lockup
- The name centred under the mark; its width between 1.0 and 1.5 × the mark's width. Set the `fontSize` to
  reach that, and if a short name stays narrow, leave it narrow rather than spacing it wide.
- The gap between the mark's lowest point and the top of the capitals: the capitals' height. In box terms the
  text's `y` = the mark's bottom + gap - 0.28 × `fontSize`.
- Centre on the mark's visual middle: a mark that is heavier on one side is not centred on its box. The
  first letter starts a little inside its box (its side bearing, 2% to 5% of the `fontSize`): to make the
  name exactly as wide as the mark, measure the letters in the picture, not the box.

## A name over two lines
Two text layers, the same family and weight, each with `"lineHeight": 1` so their boxes are no taller than
they need to be, set to the same width by their sizes (the shorter word larger). The gap between the lines'
letters is a quarter of the smaller line's capitals. Check that the weight you asked for was drawn: a
family without that weight or without an italic is drawn in its nearest face, which can be much lighter.

## A second line
Only when the user gave one. Same family, capitals, 22% to 28% of the name's `fontSize`, spaced 12% to 20%,
in the ink colour at full strength, set flush left with the name in a horizontal lockup or centred in a
stacked one, one capitals' height of its own below the name's baseline.

## Checking it
- At `scale: 2`: do the capitals sit centred on the mark; is the gap the one you set; does the first letter
  start where you meant (a round or pointed letter such as O, A or V may move 1 to 3 units toward the mark)?
- At the small proof: can the name still be read when the mark is 24 pixels? If not, the name is too small
  for the mark.
- Cover the mark: does the name alone look set with care? Cover the name: does the mark stand?
