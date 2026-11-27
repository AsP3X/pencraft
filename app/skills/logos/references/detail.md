# Building a detailed logo

For the styles `references/styles.md` marks as detailed: a badge, a crest, a seal, a mascot, an engraving, a
dimensional mark. Everything in `references/construction.md` still holds (a grid, calculated points, arcs,
exact symmetry). What is added is order, hierarchy and a few constructions.

## The order of work
A detailed logo is built in passes, and looked at after each one. Never start with the small parts.
1. The skeleton: the outer shape and its zones (ring, field, band, scroll; or a mascot's envelope and its
   big masses) as plain shapes in black. This is what the rough marks show.
2. The silhouette: fill everything in one ink. It has to be told apart from other logos already.
3. The big shapes inside: the charge, the picture's masses, the bands. Two or three flat colours.
4. The lines: outer contour first, then inner lines, then fine detail, each in its own weight.
5. Tone: one shadow tone per colour, then a few highlights, all from one light.
6. Type: rim text, the name, the motto.
7. Fine detail last, and only where the eye should stop. Then take one thing away.

## Hierarchy
- Three line weights at most, with a clear step between them: the contour or container (heaviest), the
  structure inside (about half), fine detail (about a quarter, and never under 0.5% of the logo's width: at
  a width of 480 for example 8, 4 and 2.4, or the contour the style's entry gives with half and a quarter
  of it). Write the three numbers down before the first line and use nothing else.
- Two or three tones per colour: the base, one shadow (the same hue darker and a little more saturated,
  never grey), sometimes one highlight. Shadows are flat shapes with hard edges unless the style is
  dimensional.
- One light. Decide it once (top left for engraving and mascots, straight above for dimensional work) and
  put every shadow on the far side of every part.
- One centre line. In a badge, a crest or a seal every part is centred on it or mirrored about it.
- A small set of angles, radii and gaps, repeated. Before adding a part, ask which existing angle and
  radius it reuses.
- A resting place. Detail gathers in one or two areas; the rest stays plain so the eye has somewhere to go.
- Everything inside the container keeps a margin from its edge: at least the heaviest line's width.

These tiers are a ceiling. A style that is flat by its rules (a vintage badge, a seal, a mon) uses two line
weights and no shadow tones; its richness is in its parts and its type.

## Contours and keylines
The outline round a filled shape of any form is the same path once more, behind it, with a `stroke` of twice
the outline's width in the outline's colour and `"strokeLinejoin": "round"` (or "miter" for sharp work):
half of that stroke shows outside the shape. A second, wider copy behind that one is the keyline. Give all
three the same `geometry` and `viewBox`, with room in the `viewBox` for the widest stroke (a `viewBox` of
`[-20, -20, 520, 520]` on a layer 520 wide keeps one unit one unit). A mitred corner sharper than about 40
degrees shoots far out: use round joins there, or blunt the point.
One filled path may hold several shapes only when they do not overlap: where two of its subpaths overlap,
the overlap can come out as a hole. Parts that overlap (a body, a neck, a wing) are separate layers of the
same fill.

## Cutting to a shape: masks
A shape layer with `"mask": true` is drawn as it is and cuts every layer after it in the same group to its
outline. This is how anything is put inside a shape: a wavy base and a charge inside a shield, a divided
field (two rectangles of different tinctures after the shield as mask), a shadow tone (a plain shape in
the shade colour laid over the dark side of a masked mass, its far edge cut by the mask), hatching at any
angle (a rotated group of ruled lines after the mask), a highlight. Build it as a group: the mask first,
then what it cuts. What must stay whole comes after that group: the contour or keyline drawn over the
edge, text. A second mask layer in the same group ends the first and cuts what follows it, so several
masked parts can lie in one group, each mask followed by what it holds.
Without a mask (an older Quill answers that the property is unknown), a shadow is drawn as a shape of
its own that shares the edge of what it shades. A `viewBox` may start
anywhere (`[60, 0, 120, 240]`), and `path` layers take `strokeLinecap` and `strokeLinejoin`.

## Text on a circle
A `text` layer with `"onCircle": "top"` or `"bottom"` runs along the circle that fits its box: give it
`"textGrowth": "fixed-width-height"` and the same `width` and `height`, the circle's diameter. The baseline
is that circle. At the top the text is centred on the highest point, reads clockwise, and its letters stand
outward; at the bottom it is centred on the lowest point, reads left to right, and its letters stand inward.
`"circleAngle"` turns the middle of the text clockwise, in degrees.
- To sit in the middle of a ring between the radii r1 (inner) and r2 (outer) with capitals c high (0.7 ×
  `fontSize`, more for some faces): the top text's circle has the radius (r1 + r2 - c) / 2, the bottom
  text's circle (r1 + r2 + c) / 2. The layer's `x` and `y` are the centre minus that radius.
- Capitals 50% to 60% of the ring's width; `letterSpacing` 10% to 20% of the size; one family.
- The two arcs must not meet: each takes at most about 150 degrees. To know a text's length, insert it once
  as an ordinary straight `text` layer, read its `width` from the answer's `layoutBoxes`, and delete it; the
  angle it takes on the circle is that length / radius × 57.3 degrees. The same picture shows how tall its
  capitals really are (0.68 to 0.78 × `fontSize`, by the face). Shorten, shrink or
  re-space until both fit, and put a star or a dot at 9 and 3 o'clock.
- Look at it at `scale: 2`: letters upright on their arc, both arcs ending symmetrically.

## Text along a line
A `text` layer with `"onPath"` runs along any line: SVG path data in the layer's own coordinates, with
`"textGrowth": "fixed-width-height"` and a box that holds the line. The line is the baseline, the text reads
the way the line is drawn and stands on its left, and `"textAlign": "center"` puts it in the line's middle.
For a name on a waving scroll, the line is the scroll's middle line moved down by about a third of the
capitals' height, so the letters sit in the middle of the band: build the scroll's top and bottom edges as
the same curve at two heights and give the text that curve again. Leave the first and last tenth of the
line free of text. A name on an arch that is part of a circle is simpler with `onCircle`.

## Constructions
`build_path` makes every repeated or mirrored part below from one unit: rays as one `wedge` with a `repeat`
about the centre, a ring of beads or teeth as one shape repeated, a wreath as one `leaf` repeated with an
`angle` per step and mirrored, a star as `star`, mantling and supporters as one half with a `mirror`. Write
one unit and let it calculate the rest; the recipes say what the unit is.

**Rays of a sunburst.** n rays between the angles a0 and a1 about a centre, from the radius r1 to r2: each
ray is a wedge whose two sides lie on angles half a ray-width either side of its middle, closed by arcs of
r1 and r2. Give the rays 40% to 50% of each step (the rest is gap). All four corners are points on circles
(`references/construction.md`); never draw rays as rotated rectangles placed by eye.

**A five-pointed star** of the outer radius R about (cx, cy): ten points alternating between R and 0.382 R,
at steps of 36 degrees starting straight up (-90). Stars in a logo are small separators, all the same size.

**A ring of beads, leaves or teeth.** One unit drawn at the top, then turned in equal steps about the
centre by the rotation rule, all as subpaths of one path. Count them: an even number mirrors cleanly.

**A laurel or wreath** is one leaf (two arcs meeting in points) repeated along an arc, each turned to
follow it, mirrored to the other side. Seven to nine leaves a side; leave the top open.

**A waving scroll** is the ribbon with its face bent: the top edge is a gentle S (`M50 40 C150 0 330 80 430
40`), the bottom edge the same curve 80 lower, drawn back the other way, and the name runs along the curve
`onPath` between them. Keep the wave shallow (no more than a fifth of the face's height up and down), or the
letters fan apart on the crests.

**A ribbon or scroll.** Three flat tones of one colour: the face, the tails behind it (darker), and the
small folds that join them (darkest). The face is a rectangle (or a band between two arcs of one centre);
each tail is as high as the face, set lower by a quarter of its height, with a V cut into its end; each fold
is a triangle from the face's lower corner to the tail's lower corner. The text on it is capitals about
half the face's height.

**A shield with a keyline.** The shield as a filled path; the keyline is a second, larger shield behind it
in the dark ink (the same construction with every radius and offset grown by the line's width), not a
stroke, so corners stay sharp and the weight is exact.

**A helm**, seen from the front, is a rounded dome on a short neck: a half circle as wide as two fifths of
the shield over a rectangle, a horizontal eye slit cut out with `evenodd`, and a few upright bars under
it. **A torse** (the twisted wreath on the helm) is a row of six slanted lozenges in two alternating
tinctures. **Mantling** is cloth cut into leaves, falling from the helm down both sides of the shield:
build one side as three to five curled leaves, each a crescent between two arcs of different radius that
end in one point, the largest at the top, each tucked behind the one above; colour outside, metal inside.
Then write the other side by mirroring every coordinate (x becomes W - x, every arc's `sweep` flips).
**Supporters** are two animals in profile facing the shield, mirrored, each built like a character's head
below but whole and stylised, standing on a compartment (a low mound or a bar).

**A divided field.** Per pale (left and right), per fess (top and bottom), quartered: the shield as a mask,
then one plain rectangle per part in its tincture; the mask cuts them to the shield.

**Hatching.** Even hatching in an upright or level direction is a pattern fill on the shape itself:
`"fill": ["$paper", {"type":"pattern","shape":"lines","color":"$ink","size":2,"spacing":8}]` (add
`"direction":"vertical"` to turn it). Hatching that follows a form is drawn: one path of many open lines,
each an arc of the form's own curve, closer together in the shadow, one `strokeWidth`.

**Organic curves** (an animal, a leaf, a letter) are not held to the grid's angles or to mirror symmetry (a
head in three quarter view has neither); they are held to this instead. They are cubic curves with discipline: a point at each extreme
of the shape (leftmost, topmost, and so on), handles level or upright at those points, both handles of a
smooth point on one straight line, and as few points as carry the shape. A quarter circle of radius r is
`C` with handles 0.5523 r long. Draw one half and mirror it when the thing is symmetric. Look at `scale: 3`
and fix every bump before going on.

**A character's head** starts as three or four big masses (skull, muzzle, ears) fitted into a circle, drawn
as filled shapes with their points on a grid of 10; then the contour as the stroked copy behind; then eyes
and mouth as two or three shapes each (an eye is a dark shape with a slanted upper edge for the brow and
one small light; fierce is a brow line that falls toward the nose, friendly is a round eye); then the
markings in the second colour (cheeks, muzzle, inner ears); then one shadow shape per mass on the side away
from the light, sharing that mass's edge. Sharp tufts and ear tips are straight lines meeting in points;
the long curves between them are single cubic curves. Look after each of these steps.

**Depth for a dimensional mark.** A linear gradient with `"rotation": 180` runs downward: its first colour
is at the top. Shape: light tone to its deeper neighbour. Under it one shadow: `{"type":"shadow",
"shadowType":"outer","color":"<dark shade of the ground at 40%>","offset":{"x":0,"y":10},"blur":18}`. An
inner shadow (`"shadowType":"inner"`) with an upward offset darkens a shape's lower edge and makes it look
pressed or thick. A soft light on a tile is a layer no larger than the tile with a white gradient that
fades to nothing.

## Worked logos
These show the constructions together, not designs to reuse.

**A roundel badge**, 480 across: ring from radius 168 to 240 (15% of the width), an inner rule, a sun with
nine rays over three bars, stars at 9 and 3 o'clock, the name on the top arc and the place on the bottom
arc. Capitals of 38 are 27 high; the ring's middle is at 204, so the top text's circle has the radius 190
and the bottom text's (capitals 18 high) 213.

```json
{"type":"frame","name":"Nordlicht roundel","x":0,"y":0,"width":480,"height":480,"layout":"none","children":[{"type":"path","name":"Ring","x":0,"y":0,"width":480,"height":480,"fill":"#1F3A5F","fillRule":"evenodd","viewBox":[0,0,480,480],"geometry":"M240 0 A240 240 0 0 1 240 480 A240 240 0 0 1 240 0 Z M240 72 A168 168 0 0 0 240 408 A168 168 0 0 0 240 72 Z"},{"type":"path","name":"Inner rule","x":0,"y":0,"width":480,"height":480,"fill":"#1F3A5F","fillRule":"evenodd","viewBox":[0,0,480,480],"geometry":"M240 80 A160 160 0 0 1 240 400 A160 160 0 0 1 240 80 Z M240 86 A154 154 0 0 0 240 394 A154 154 0 0 0 240 86 Z"},{"type":"path","name":"Sun and rays","x":0,"y":0,"width":480,"height":480,"fill":"#C8452E","viewBox":[0,0,480,480],"geometry":"M170 250 A70 70 0 0 1 310 250 Z M156.43 241.51 L100.72 235.85 A140 140 0 0 1 104.28 215.66 L158.57 229.39 A84 84 0 0 0 156.43 241.51 Z M164.37 213.44 L113.96 189.07 A140 140 0 0 1 124.21 171.31 L170.53 202.78 A84 84 0 0 0 164.37 213.44 Z M181.44 189.78 L142.4 149.63 A140 140 0 0 1 158.11 136.45 L190.86 181.87 A84 84 0 0 0 181.44 189.78 Z M205.57 173.38 L182.61 122.3 A140 140 0 0 1 201.88 115.29 L217.13 169.17 A84 84 0 0 0 205.57 173.38 Z M233.85 166.23 L229.75 110.38 A140 140 0 0 1 250.25 110.38 L246.15 166.23 A84 84 0 0 0 233.85 166.23 Z M262.87 169.17 L278.12 115.29 A140 140 0 0 1 297.39 122.3 L274.43 173.38 A84 84 0 0 0 262.87 169.17 Z M289.14 181.87 L321.89 136.45 A140 140 0 0 1 337.6 149.63 L298.56 189.78 A84 84 0 0 0 289.14 181.87 Z M309.47 202.78 L355.79 171.31 A140 140 0 0 1 366.04 189.07 L315.63 213.44 A84 84 0 0 0 309.47 202.78 Z M321.43 229.39 L375.72 215.66 A140 140 0 0 1 379.28 235.85 L323.57 241.51 A84 84 0 0 0 321.43 229.39 Z"},{"type":"path","name":"Sea","x":0,"y":0,"width":480,"height":480,"fill":"#1F3A5F","viewBox":[0,0,480,480],"geometry":"M108 270 H372 V282 H108 Z M124 296 H356 V308 H124 Z M148 322 H332 V334 H148 Z"},{"type":"path","name":"Stars","x":0,"y":0,"width":480,"height":480,"fill":"#F3EBDD","viewBox":[0,0,480,480],"geometry":"M36 226 L39.14 235.67 L49.31 235.67 L41.09 241.65 L44.23 251.33 L36 245.35 L27.77 251.33 L30.91 241.65 L22.69 235.67 L32.86 235.67 Z M444 226 L447.14 235.67 L457.31 235.67 L449.09 241.65 L452.23 251.33 L444 245.35 L435.77 251.33 L438.91 241.65 L430.69 235.67 L440.86 235.67 Z"},{"type":"text","name":"Name","x":50,"y":50,"width":380,"height":380,"textGrowth":"fixed-width-height","onCircle":"top","content":"NORDLICHT RÖSTEREI","fontFamily":"Futura","fontWeight":"700","fontSize":38,"letterSpacing":5,"fill":"#F3EBDD"},{"type":"text","name":"Place","x":27,"y":27,"width":426,"height":426,"textGrowth":"fixed-width-height","onCircle":"bottom","content":"TROMSØ · SEIT 1998","fontFamily":"Futura","fontWeight":"500","fontSize":26,"letterSpacing":6,"fill":"#F3EBDD"}]}
```

**A dimensional app icon**, light from above: a gradient tile, a soft light across its top half, the mark as
one line with a gradient of its own and one shadow, and a dot in the accent colour.

```json
{"type":"frame","name":"Tide app icon","x":0,"y":0,"width":480,"height":480,"layout":"none","cornerRadius":108,"clip":true,"fill":{"type":"gradient","gradientType":"linear","rotation":180,"colors":[{"color":"#3FA7F5","position":0},{"color":"#1B4FD1","position":1}]},"children":[{"type":"rectangle","name":"Light","x":0,"y":0,"width":480,"height":260,"fill":{"type":"gradient","gradientType":"linear","rotation":180,"colors":[{"color":"#FFFFFF40","position":0},{"color":"#FFFFFF00","position":1}]}},{"type":"path","name":"Mark","x":96,"y":96,"width":288,"height":288,"stroke":{"type":"gradient","gradientType":"linear","rotation":180,"colors":[{"color":"#FFFFFF","position":0},{"color":"#D6E6FF","position":1}]},"strokeWidth":44,"strokeLinecap":"round","viewBox":[0,0,288,288],"effect":{"type":"shadow","shadowType":"outer","color":"#0B2A7A66","offset":{"x":0,"y":10},"blur":18},"geometry":"M22 110 A50 50 0 0 1 72 160 A72 72 0 0 0 216 160 A50 50 0 0 1 266 110"},{"type":"ellipse","name":"Dot","x":212,"y":150,"width":56,"height":56,"fill":"#FFD166","effect":{"type":"shadow","shadowType":"outer","color":"#0B2A7A66","offset":{"x":0,"y":8},"blur":14}}]}
```

**A scroll** in three tones of one red, with its text.

```json
{"type":"frame","name":"Scroll","x":0,"y":0,"width":480,"height":120,"layout":"none","children":[{"type":"path","name":"Tails","x":0,"y":0,"width":480,"height":120,"fill":"#A53622","viewBox":[0,0,480,120],"geometry":"M0 30 H80 V110 H0 L24 70 Z M480 30 H400 V110 H480 L456 70 Z"},{"type":"path","name":"Folds","x":0,"y":0,"width":480,"height":120,"fill":"#6E2214","viewBox":[0,0,480,120],"geometry":"M50 90 L80 110 V90 Z M430 90 L400 110 V90 Z"},{"type":"path","name":"Face","x":0,"y":0,"width":480,"height":120,"fill":"#C8452E","viewBox":[0,0,480,120],"geometry":"M50 10 H430 V90 H50 Z"},{"type":"text","name":"Motto","x":50,"y":22,"width":380,"height":56,"textGrowth":"fixed-width-height","textAlign":"center","textAlignVertical":"middle","content":"SEIT 1998","fontFamily":"Futura","fontWeight":"700","fontSize":40,"letterSpacing":8,"fill":"#F3EBDD"}]}
```
