# Rendering styles

A logo has a kind (wordmark, monogram, mark with a name, emblem, mascot, app icon) and a rendering style. Each
style below is a set of rules; a logo keeps to the rules of one. For each: when it fits, its palette, its
lines, how it shades, its type, its layout with numbers, how it is built in Pencraft, what its small version
is, and what gives a poor one away. Numbers are in parts of the logo's width (W) unless they say otherwise.

Styles marked *detailed* are built in the order `references/detail.md` gives and are delivered with a small
version (SKILL.md, section 9).

## Choosing
The user's word decides ("vintage", "a crest", "mascot", "minimal", "3D", "luxury", "hand-drawn"). Without one,
go by tone first (formal or playful, old or new, loud or quiet), then by where it lives:

| The brief | Styles to consider |
|---|---|
| Software, a tool, a service for business | flat geometric, monoline, negative space, gradient and dimensional |
| An app that needs its icon | dimensional app icon, or flat geometric on a tile |
| Food, drink, a workshop, outdoors, a shop with a history | vintage badge, engraved, hand-lettered, monoline |
| A university, a law firm, a club, a town, a family | heraldic crest, seal, luxury serif |
| A team, a league, a school, a gaming channel | mascot, with bold type and a shield or roundel |
| Fashion, beauty, a hotel, a restaurant with tablecloths | luxury monogram and serif wordmark, art deco |
| Children, games, sweets, a friendly consumer app | playful rounded, mascot, pixel |
| Music, a festival, streetwear, a gallery | retro, psychedelic, brutalist, art deco |
| Building, freight, tools, security | stencil, heavy geometric, flat |
| Quiet, crafted, Japanese or Nordic restraint | mon, monoline, negative space |

If the logo will mostly be seen under 64 pixels (a favicon, an app, a tab), choose a simple style, or plan the
small version first and the detailed one as its dress uniform.

## Flat geometric
- When: the default of modern identity; anything that must work everywhere.
- Palette: one or two flat colours. Lines: none; filled shapes from circles, squares and a few fixed angles.
- Sub-kinds worth knowing: pure geometry; optical effect (overlap, rotation, a field of lines); built from a
  letter.
- Type: a grotesque or geometric sans, one weight. Small version: the same mark.
- Tells: gaps that are nearly equal, radii chosen by feel, two ideas in one mark, one heavy spot (squint).

## Monoline
- When: craft, cafes, software, anything light and friendly. Works as a simple mark or as a *detailed*
  line illustration.
- One stroke width everywhere, 4% to 6% of the mark's height; gaps at least one stroke wide. One
  `strokeLinecap` and `strokeLinejoin` for all (round reads friendly, square reads technical).
- No shading; depth only by a line stopping short of the line it passes behind (a gap of one stroke).
- Type: a sans or script whose stems match the stroke. Small version: thicker stroke, fewer lines, open ends.
- Tells: two widths, mixed caps, lines closer than a stroke, a closed shape that fills in when small.

## Negative space
- When: the idea is two things in one. One or two flat colours.
- The hidden shape could stand alone as a shape, and is seen within a second without being told.
- No sliver thinner than the mark's thinnest stroke. Small version: often loses the hidden shape; test it.
- Tells: the second picture needs explaining; it disappears at 32 pixels.

## Vintage badge *(detailed)*
- When: roasters, breweries, barbers, outfitters, workshops; "heritage", "stamp", "label", "roundel".
- Palette: two to four flat colours, typically one dark ink, a paper, one accent.
- Layout of a roundel: outer circle W; the ring that carries the text is 12% to 17% of W on each side; a
  thin inner rule in the light line weight set a few units inside the ring; the centre field inside it is 55% to 70% of W
  and holds one picture, which fills about four fifths of it. The name runs on the top arc, a place, trade or year on the bottom arc, with a star or
  dot at 9 and 3 o'clock between them. The name's capitals are 50% to 60% of the ring's width and sit in its middle; the second
  line's are smaller, 35% to 45%.
- Other layouts: a lozenge or shield; a picture with the name arched over it and a ribbon across its lower
  third; stacked lines of type of different sizes filling a shape.
- Lines: a heavy weight for outlines and rings and a light one (about 0.5% of W, no less than 2 at W 480)
  for inner rules and picture detail; a middle one only if the picture needs it. Shading: flat colours, no
  shadow tones; at most a few hatch lines or paper-coloured cuts in the picture (feathers, planks, ripples).
- Where the brief gives a year or a place and the picture leaves room, they may flank it in the field
  ("20 | 11" either side of its foot) instead of going on the rim.
- Type: two families at most: wide or condensed capitals for the rim, and a script, a slab or a serif for
  one large word. Spaced capitals on the arcs (10% to 20% of the size).
- In Pencraft: rings as `evenodd` paths; rim text as `text` layers with `onCircle`; rays, stars and ribbons
  from `references/detail.md`. Small version: the centre picture or a monogram in the outer ring, no text.
- Tells: bottom text upside down; arc text that does not end symmetrically; three or more faces; parts
  that share no centre line; ring and picture in the same line weight; texture baked in.

## Heraldic crest and coat of arms *(detailed)*
- When: families, towns, universities, clubs, anything that wants lineage.
- A shield alone is a crest in the everyday sense. A full achievement has, from the top: the crest on a
  twisted wreath, the helm, mantling falling from the helm on both sides, the shield, supporters either
  side standing on a compartment, and a motto scroll under the shield. Use only the parts asked for.
- Tinctures: two metals (gold or yellow, silver or white) and five colours (red, blue, green, purple,
  black). Never metal on metal or colour on colour: a charge on a red field is gold or white, never blue.
  Small details (claws, tongues) and things in their natural colour are exempt; a thin edge in a metal
  rescues a clash. It is a rule of contrast.
- Shield about 5 wide to 6 high. Charges are stylised, bold and fill their field; a divided field (per
  pale, per fess, quartered) puts one charge in each part. In a logo the helm and crest together are about
  two thirds as tall as the shield, supporters about the shield's height, and the whole achievement fits a
  box about 3 wide to 4 high: shield 50% of the box's width without supporters, 36% with them. A wavy base
  (water) or a chief (a band across the top) are divisions of the field, not stock pictures.
- Lines: flat tinctures with a dark contour of even weight (about 1.2% of W), inner lines half of that, and
  nothing finer than 0.5% of W; at most one shadow tone.
  Mantling mirrored exactly. One-colour version: tinctures become hatching (gold dots, silver blank, red
  vertical lines, blue horizontal lines, black crossed lines, green diagonal).
- Type: the motto in spaced capitals on the scroll, a serif with some age. Small version: the shield with
  its main charge; then the charge alone.
- Tells: low-contrast tinctures; a small charge floating in an empty field; realistic animals; a motto too
  small to read; lopsided mantling.

## Seal *(detailed)*
- When: institutions, certificates, "official". A roundel with stricter manners.
- Inner circle 65% to 75% of the outer one, so the text ring is 12% to 17% of W each side. Name on the top
  arc, clockwise; second line on the bottom arc, reading left to right; stars or dots between. One or two
  colours. An outer edge of beads or a rope is a ring of small circles placed by rotation.
- Centre: one emblem, a monogram or a shield. Small version: the centre alone in one ring.
- Tells: text touching the ring's edges; the two arcs colliding; a centre that is a tiny picture.

## Engraved, woodcut and linocut *(detailed)*
- When: distillers, bookshops, botanicals, banks; "etched", "old print".
- One ink on paper, a second ink as a spot. Tone is made by lines only: lines follow the form, closer
  together and thicker in shadow; cross-hatching only in the deepest shadow; highlights are bare paper. One
  light, from the top left. Woodcut and linocut are the coarse kind: few, thick, wedge-shaped cuts, large
  solid blacks, two or three values.
- Lines taper: draw each as a thin closed wedge, not a stroke of even width, or use two stroke widths.
- Type: engraved serifs and old display capitals. Small version: the solid silhouette with three to five
  lines kept.
- Tells: even parallel fill that ignores the form; lines too fine and close to hold; light from two sides.

## Mascot and sports *(detailed)*
- When: teams, leagues, schools, gaming, some food brands.
- Palette: a main colour, black or white, one accent; each colour has a base and one shadow tone (a darker
  shade of itself, never grey) and a few small highlights. Hard-edged shapes, no soft gradients.
- Lines: the outer contour is 2 to 3 times the inner lines; often a second outer keyline in white or the
  accent, so the logo sits on any ground. One light, from the top or top front.
- Shape: fits a circle or a shield, so it crops to an avatar; reads as a solid black silhouette; a three
  quarter view with an expression, not a symmetric front face. Head only is the usual choice.
- Type: a heavy sans in capitals, in an italic or condensed cut when the family has one, or arched over the
  head with `onCircle`; the club's name may run over two lines in two sizes. The name carries the
  character's contour: a `stroke` on the text in the contour's colour, `strokeWidth` the contour's width
  (a text's stroke lies outside its letters, so add twice its width to the `letterSpacing` and to the gap
  between two lines, or neighbours' outlines run together). A keyline round that is a second stroke in
  front of it in the list: `"stroke": ["$logo-paper", "$logo-ink"]`. Small version: the head redrawn with fewer shapes and no
  shadow tone, without the name.
- A white keyline cannot be seen on an empty canvas: check it over a dark rectangle put behind for the
  picture and deleted again.
- Tells: detail that turns to mush at avatar size; a weak silhouette; outlines of one weight; grey shadows.

## Illustrative and hand-drawn
- When: cafes, studios, children's things, makers. One to four colours.
- Lines of varying width with round ends; the same slight irregularity everywhere, or none.
- Small version: the main shape as a solid. Tells: wobble in some parts only; too many points.

## Hand-lettered script and custom lettering
- When: the name is the logo and should have a voice: bakeries, bars, sport, music.
- An installed script or display face is the start; make it this logo's with one change (a swash under the
  name as a path, a joined pair, a tail), an upward baseline of 5 to 10 degrees (`rotation`), and a weight
  that holds small.
- If letters are drawn as paths: points at the extremes of each curve, handles horizontal or vertical,
  thick downstrokes and thin upstrokes, one slant throughout.
- Tells: an untouched script font; a wavering slant; hairlines that vanish.

## Luxury monogram and serif wordmark
- When: fashion, jewellery, hotels, perfume, law. One colour: black, white, or one metal tone.
- A high-contrast serif in capitals, spaced wide (10% to 30% of the size), set small with much room; or two
  initials interlocked or mirrored into a symmetric monogram with the same gap at every crossing.
- No ornament added to "feel premium". Small version: the monogram.
- Tells: default spacing; hairlines that break when small; decoration.

## Gradient and dimensional *(detailed)*
- When: software, media, app icons; "3D", "glossy", "modern", "glass".
- One light, from the top: every gradient runs light at the top to dark at the bottom, in two or three
  stops of neighbouring hues (blue to deeper blue, orange to red), never a rainbow.
- Depth from at most three things: a gradient on the shape, one soft shadow under it (offset down, blur
  about equal to the offset, in a dark shade of the ground, 20% to 40%), and one highlight (a lighter edge or
  a soft light spot at the top). Folded or overlapping shapes: the part underneath is darker where the upper
  part crosses it.
- App icon: a tile with a gradient ground, one bold foreground shape with round corners, no thin lines,
  seen straight on. For Apple's icons do not draw bevels or gloss on the tile; the system adds them.
- Small version: the flat one-colour shape. Tells: gradients against the light; a rainbow; a glossy bevel;
  a muddy middle stop; hairlines.

## Grid, pattern and isometric
- Units on a strict grid of cells with equal gutters; or a solid drawn at 30 degrees with three face tones
  (top lightest, one side middle, the other darkest).
- Tells: unequal gutters; mixed angles; too many cells to read small.

## Retro
- 1970s: three to six parallel stripes of equal width (gaps a quarter to a half of a stripe), straight or
  arcing, in earthy tones; heavy rounded type. 1980s: neon on dark, chrome letters (a light sky tone at the
  top, a hard dark band across the middle, a warm tone below), a grid horizon. 1990s: shapes off their axis,
  hard offset shadows, squiggles.
- Tells: a face from the wrong decade; unequal stripes; fake wear.

## Art deco
- Strict mirror symmetry; stepped forms, chevrons, fans and sunbursts with rays at equal angles; two colours
  (gold on black, cream on navy); two line weights, a thick frame with thin parallel lines inside it.
- Type: tall geometric capitals. Tells: any asymmetry; flowing plant curves; unequal rays.

## Mon
- One colour, and its inverse works as well. Only circles and straight lines, drawn as with compass and
  ruler; a plant, a bird, a wave or an object reduced to that, often inside a ring 6% to 10% of W thick;
  turning symmetry of two, three or five. No type in the mark.
- Tells: freehand curves; colour doing the work; near-symmetry.

## Playful rounded
- Every end and corner round; soft, fat shapes; two to four bright or pastel colours; one thick even
  outline or none; a face is two dots and a curve; one small highlight.
- Tells: sharp and round corners mixed; a detailed face.

## Psychedelic and maximalist *(detailed)*
- Strong complementary pairs; type warped to fill a shape; ornament everywhere, with one calm area kept for
  the name. Tells: a name nobody can read; no small version.

## Pixel
- A grid of 16 or 32 squares; every square on the grid, one square size; 4 to 16 colours; no curves. Draw
  each colour as one path of squares. Tells: half squares; a smooth curve among pixels.

## Brutalist
- Black, white and at most one raw colour; a plain grotesque or system face, tight; visible boxes and rules;
  nothing decorative. The awkwardness is a system, not an accident.

## Stencil
- One colour; every enclosed part is joined to the outside by a bridge, as if cut from sheet metal; bridges
  of one width (15% to 25% of the stem) at one angle. Tells: bridges missing in some letters.
