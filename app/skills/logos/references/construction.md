# Constructing a mark

A mark is one `path` layer, sometimes two or three. Its `geometry` is SVG path data in the coordinates of its
`viewBox`; `width` and `height` scale it. Draw in a `viewBox` of `[0, 0, 240, 240]` and give the layer the
same `width` and `height`, so one unit is one unit while you work. Resize the group at the end, never the
numbers.

`build_path` does the arithmetic of this file for you: give it shapes (`circle`, `ring`, `arc`, `polygon`,
`star`, `rect`, `wedge`, `leaf`, `points`), path data of your own or a `text`, with a `transform`, a `mirror`
and a `repeat`, and a `combine` that joins a part to the ones before it or cuts it out of them, and it answers with the path data and the box it fills. The numbers below say what it calculates
and let you check it; use them by hand only for a single point. `measure_layers` then tells you what you
drew: boxes, gaps and symmetry.

```json
{"parts":[
  {"shape":"ring","cx":240,"cy":240,"r":240,"inner":200},
  {"shape":"wedge","cx":240,"cy":240,"from":110,"to":190,"angle":-90,"spread":5,"innerSpread":9,"repeat":{"count":24,"about":[240,240]}},
  {"path":"M240 150 C290 150 330 190 330 240 H240 Z","mirror":{"axis":"vertical","at":240}}
]}
```
is a ring 40 thick, twenty-four rays turned in equal steps, and a half shape with its mirror image, in one
path for a `viewBox` of `[0, 0, 480, 480]`.

Exact does not mean square. Decide the mark's own measures first (a unit, one or two angles of any size, one
or two radii, the weights), work the coordinates out on paper as a list of points, then write the path.
Every number comes from those measures or is calculated by one of the rules below; a grid of 20 is a help
for marks made of bars. Round calculated numbers to two decimals.

## The numbers

**A point on a circle.** With the centre (cx, cy), the radius r and the angle a measured from the right,
turning clockwise on the screen (y runs down): x = cx + r·cos a, y = cy + r·sin a.

| a | cos a | sin a |
|---|---|---|
| 0 | 1 | 0 |
| 15 | 0.9659 | 0.2588 |
| 30 | 0.8660 | 0.5 |
| 45 | 0.7071 | 0.7071 |
| 60 | 0.5 | 0.8660 |
| 75 | 0.2588 | 0.9659 |
| 90 | 0 | 1 |

For other quarters change the signs: 90 to 180 has cos negative, 180 to 270 both negative, 270 to 360 sin
negative (up is negative y).

**An arc.** `A r r 0 large sweep x y` draws from where the pen is to (x, y) along a circle of radius r.
`sweep` 1 turns clockwise on the screen, 0 counter-clockwise. `large` 1 takes the long way round (more than
half the circle), 0 the short way. Both ends must lie on the same circle: check that each is r away from the
centre you mean, or the arc is silently drawn larger.

**A whole circle** is two half arcs: `M cx (cy-r) A r r 0 0 1 cx (cy+r) A r r 0 0 1 cx (cy-r) Z`.

**A ring** is two circles in one path with `"fillRule": "evenodd"`; the stroke of the ring is the difference
of the radii. The same goes for any hole: the outer shape, then the hole as a second subpath.

**A smooth join.** An arc leaves a straight edge without a kink when the circle's centre lies on the line
through the join that is square to the edge. A corner of radius r between a line going right and a line going
down: `H (x-r) A r r 0 0 1 x (y+r) V ...`. Concentric corners: the inner radius is the outer one minus the
stroke, with the same centre.

**A thick arc as a filled shape** (when it has to join other filled shapes): the outer arc from angle a to b
with `sweep` 1, a line to the inner circle at b, the inner arc back to a with `sweep` 0, `Z`. As a line it is
simpler: one arc with a `strokeWidth`.

**A rotated copy.** To turn a point (x, y) by t degrees clockwise about (cx, cy): dx = x - cx, dy = y - cy;
x' = cx + dx·cos t - dy·sin t; y' = cy + dx·sin t + dy·cos t. For three copies use t = 120 and 240 (cos -0.5,
sin 0.8660 and -0.8660); for six, steps of 60. Write all copies as subpaths of one path. Do not use the
layer's `rotation` for this: it turns about the top left corner.

**A mirrored copy.** About the vertical centre line of a 240 box: x' = 240 - x, and every arc's `sweep` flips.

**A wave** is equal arcs with alternating `sweep`, ends on one line: `M40 120 A40 40 0 0 1 120 120 A40 40 0 0
0 200 120` is one crest and one trough of half circles. For a shallower wave keep the same end points and
make the radius larger (r = 0.7071 × the distance between ends gives quarter circles). Never a hand-drawn
`Q` wobble.

**A shield** that looks right is a flat top, straight sides to a third of the height, and two arcs whose
radius is the shield's width, each centred on the opposite side's end point: for a width of 200 from x 20 to
220 with sides ending at y 100, the arcs meet at x 120, y 100 + 0.8660 × 200 = 273.2. It narrows quickly, so
it suits a tall charge. For a wide or square charge take a flat top with a half-round base instead: straight
sides down to y, then one half circle whose radius is half the width (`M20 20 H220 V160 A100 100 0 0 1 20 160
Z`), or a circle.

**A line tangent to two circles** of the same radius r that passes between them (the spine of an S): put
the centres on one vertical line. For a spine at the angle a to that line, the centres are 2r / sin a apart
and the spine is 2r / tan a long: at 45 degrees 2.8284 r apart with a spine of 2r, at 30 degrees 4r apart
with a spine of 3.4641 r (a longer, steeper spine). The line touches each circle at the point a round from
the horizontal: (cx ± r·cos a, cy ± r·sin a). Choose the angle first and calculate the centres, not the
other way round.

**Any angle.** For an angle the table lacks, calculate: a line of the length L at the angle a ends L·cos a
to the right and L·sin a down. Useful values: 10 degrees cos 0.9848 sin 0.1736; 20 degrees 0.9397 and 0.3420;
36 degrees 0.8090 and 0.5878; 72 degrees 0.3090 and 0.9511. Use one such angle throughout, and its mirror.

**A free curve** is a cubic curve, `C x1 y1 x2 y2 x y`: from where the pen is to (x, y), leaving toward
(x1, y1) and arriving from (x2, y2). It is exact when it is disciplined: a point at each extreme of the
shape (leftmost, topmost and so on) with its handles level or upright there; at a smooth point both handles
on one straight line; handles about a third as long as the stretch they shape; and as few points as carry
the shape, usually two to four. A quarter circle of the radius r is one `C` with handles 0.5523 r long. A
curve that should feel fast has a long handle at one end and a short one at the other. Look at `scale: 3`
and remove every bump; a bump is a point too many or a handle off its line.

**A stroke that tapers** is a filled shape between two free curves that start apart and meet in a point:
draw the outer edge from the blunt end to the tip, then the inner edge back. Give both edges the same kind
of handles so the width shrinks evenly. The blunt end is as wide as the mark's heavy weight.

**A slanted bar** of the thickness t, at the angle a to the horizontal, is t / sin a wide measured along a
horizontal line: 1.1547 t at 60 degrees, 1.4142 t at 45. Use that width so it looks as heavy as an upright
stem of t.

**Flat ends on a curved line.** A stroked arc with square ends is cut square to the curve, which is rarely
level with the grid. Where an end has to be exactly level or upright, draw the mark as a filled outline
instead (outer arc, end, inner arc back), or give the line round ends.

**A wave as a band** (filled, with a thickness t) is two wave lines with one centre for each crest: the
outer edge has the radius r + t/2 and the inner r - t/2, so r must be larger than t. If the inner radius
would fall under 20, make the wave shallower (a larger r over the same ends) or draw it as a line with a
`strokeWidth`.

**An equal gap.** Two shapes side by side are separated by the same distance everywhere: straight edges are
parallel, curved edges are concentric (same centre, radii differing by the gap).

## Worked marks

These show the method, not shapes to reuse: a mark for the user's brief comes from its own idea.

**A letter and a thing sharing one contour.** A square H whose lower counter is an arch: a bridge. One closed
path, stems 60 wide, all points on the 20 grid, the arch a half circle of radius 40 centred on (120, 180).

```json
{"type":"path","name":"Mark","x":0,"y":0,"width":240,"height":240,"fill":"#111111","viewBox":[0,0,240,240],"geometry":"M20 20 H80 V100 H160 V20 H220 V220 H160 V180 A40 40 0 0 0 80 180 V220 H20 Z"}
```

**A mark made of lines.** Three quarter circles about one centre (40, 200), radii 60, 110 and 160, one
`strokeWidth` of 30, so every gap is 20. Round ends, chosen once.

```json
{"type":"path","name":"Mark","x":0,"y":0,"width":240,"height":240,"stroke":"#111111","strokeWidth":30,"strokeLinecap":"round","viewBox":[0,0,240,240],"geometry":"M100 200 A60 60 0 0 0 40 140 M150 200 A110 110 0 0 0 40 90 M200 200 A160 160 0 0 0 40 40"}
```

**A part repeated by rotation.** One leaf between (120, 100) and (120, 20), made of two arcs of radius 50;
the other two are the same points turned by 120 and 240 degrees about (120, 120).

```json
{"type":"path","name":"Mark","x":0,"y":0,"width":240,"height":240,"fill":"#111111","viewBox":[0,0,240,240],"geometry":"M120 100 A50 50 0 0 1 120 20 A50 50 0 0 1 120 100 Z M137.32 130 A50 50 0 0 1 206.6 170 A50 50 0 0 1 137.32 130 Z M102.68 130 A50 50 0 0 1 33.4 170 A50 50 0 0 1 102.68 130 Z"}
```

**A crest with one charge cut out of it.** The shield from two arcs, and one tower as a hole (`evenodd`), so
the ground shows through. The tower is 80 of the shield's 200 wide here only because its three merlons need
20 each; a simpler charge would be larger. No outline, no inner rule, no second colour.

```json
{"type":"path","name":"Mark","x":0,"y":0,"width":240,"height":300,"fill":"#111111","fillRule":"evenodd","viewBox":[0,0,240,300],"geometry":"M20 20 H220 V100 A200 200 0 0 1 120 273.2 A200 200 0 0 1 20 100 Z M90 200 V110 H80 V70 H100 V90 H110 V70 H130 V90 H140 V70 H160 V110 H150 V200 Z"}
```

**A sweep that tapers.** One blade, 50 wide at its foot and running out in a point at the top right: the
outer edge leaves (40, 200) straight up and arrives at the tip level; the inner edge returns and arrives at
the foot straight down. Two points, four handles.

```json
{"type":"path","name":"Mark","x":0,"y":0,"width":240,"height":240,"fill":"#111111","viewBox":[0,0,240,240],"geometry":"M40 200 C40 100 120 30 220 30 C150 60 90 120 90 200 Z"}
```

**A ring with a part taken out.** Two circles, radii 100 and 60, about (120, 120): a ring 40 thick.

```json
{"type":"path","name":"Mark","x":0,"y":0,"width":240,"height":240,"fill":"#111111","fillRule":"evenodd","viewBox":[0,0,240,240],"geometry":"M120 20 A100 100 0 0 1 120 220 A100 100 0 0 1 120 20 Z M120 60 A60 60 0 0 0 120 180 A60 60 0 0 0 120 60 Z"}
```

## Checking the construction
- Read the path back as a list of points: is every x mirrored by 240 - x where the mark is symmetric?
- For each arc: are both ends r from the centre? Is the centre square to the edge it joins?
- Measure every bar and every gap: one value for bars, or two that differ plainly (one to three or more),
  and one for gaps. Two values that are nearly alike are a mistake.
- Cover the mark's inside and look at the silhouette alone: is it this mark's, and is one part clearly the
  largest?
- Look at `scale: 3`. A kink where a curve meets an edge is a wrong centre; a part that looks heavier than
  the rest is a wrong width; a gap that closes at `scale: 0.1` is too narrow.
