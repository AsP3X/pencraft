# Print

Draw print in points: Quill's PDF has one unit to the point (1/72 inch; 1 mm is 2.835 points), keeps text
as text and shapes as shapes, and so prints sharp at any size. Paper sizes are standards; the working rules
on bleed, margins and minimums are common to printers' own pages but differ a little from one to the next,
so the printer's template, when the user has one, wins.

## Paper

| Sheet | Millimetres | Points | Pixels at 300 per inch |
|---|---|---|---|
| A6 | 105 x 148 | 298 x 420 | 1240 x 1748 |
| A5 | 148 x 210 | 420 x 595 | 1748 x 2480 |
| A4 | 210 x 297 | 595 x 842 | 2480 x 3508 |
| A3 | 297 x 420 | 842 x 1191 | 3508 x 4961 |
| A2 | 420 x 594 | 1191 x 1684 | 4961 x 7016 |
| A1 | 594 x 841 | 1684 x 2384 | 7016 x 9933 |
| A0 | 841 x 1189 | 2384 x 3370 | 9933 x 14043 |
| Letter | 216 x 279 (8.5 x 11 in) | 612 x 792 | 2550 x 3300 |
| Legal | 216 x 356 (8.5 x 14 in) | 612 x 1008 | 2550 x 4200 |
| Tabloid | 279 x 432 (11 x 17 in) | 792 x 1224 | 3300 x 5100 |
| Half Letter | 140 x 216 (5.5 x 8.5 in) | 396 x 612 | 1650 x 2550 |
| Postcard | 4 x 6 in; 5 x 7 in | 288 x 432; 360 x 504 | 1200 x 1800; 1500 x 2100 |
| Poster, US | 18 x 24 in; 24 x 36 in | 1296 x 1728; 1728 x 2592 | 5400 x 7200; 7200 x 10800 |
| Envelope DL, C5 | 110 x 220; 162 x 229 | 312 x 624; 459 x 649 | |

Europe and most of the world print on A sizes; the United States and Canada on Letter and its relatives.
Ask, or go by where the user is.

## Business cards

| Where | Millimetres | Points |
|---|---|---|
| Europe, United Kingdom | 85 x 55 | 241 x 156 |
| United States, Canada | 88.9 x 50.8 (3.5 x 2 in) | 252 x 144 |
| Australia | 90 x 55 | 255 x 156 |
| Japan | 91 x 55 | 258 x 156 |
| China | 90 x 54 | 255 x 153 |

Contact details 8 points or larger, the name 10 to 14. A code to scan is 20 mm (57 points) square or more,
with clear paper around it. Two sides are two frames.

## Bleed, trim and safe margin

- Trim is the size of the finished piece. The knife is never exact.
- Bleed: the ground and any picture that touches an edge run 3 mm (8.5 points) past the trim on every side.
  The frame is drawn at trim plus bleed: an 85 x 55 mm card is 258 x 173 points. (A few printers ask for
  1.5 mm, packaging for 5.)
- Safe margin: nothing that is read lies within 3 to 5 mm (8.5 to 14 points) of the trim. So the frame's
  `padding` is the bleed plus the margin: 22.5 for a card, 40 to 60 for a flyer or poster.
- Tell the user the trim size and the bleed with the file: Quill's PDF carries no crop marks.

## Minimums

| What | Smallest |
|---|---|
| Dark text on light | 6 points; 8 for a thin or delicate face |
| Light text on a dark or coloured ground | 8 points, and not a thin weight |
| A line | 0.25 points; 0.5 to be safe, and 0.5 when it is light on dark |
| A picture | 300 pixels per inch at its printed size: a picture 10 cm wide needs about 1200 pixels |

Colours are sRGB and the printer converts them: the brightest oranges, greens and blues come out duller.
Say so once, and choose inks that survive it. Black text is `#000000` or near it, never a tinted grey at
small sizes.

## A business card

85 x 55 mm with 3 mm of bleed: a frame of 258 x 173 with 22.5 of padding (the bleed and a 5 mm margin).
Quill's review counts text under 11 as too small, because it takes a unit for a pixel. On a print piece
a unit is a point and 8 is right: that remark can stand.

```json
{"type":"frame","id":"format-card","name":"Business card 85 x 55 mm, with bleed","width":258,"height":173,"layout":"vertical","padding":22.5,"justifyContent":"space_between","fill":"$paper","clip":true,"children":[
  {"type":"frame","id":"format-card-top","name":"Name","layout":"vertical","gap":2,"children":[
    {"type":"text","id":"format-card-name","name":"Name","content":"Ingrid Vang","fontFamily":"$font","fontSize":13,"fontWeight":"700","fill":"$ink","letterSpacing":-0.2},
    {"type":"text","id":"format-card-role","name":"Role","content":"Volunteer coordinator","fontFamily":"$font","fontSize":8,"fill":"$ink-soft"}]},
  {"type":"frame","id":"format-card-bottom","width":"fill_container","justifyContent":"space_between","alignItems":"end","children":[
    {"type":"frame","id":"format-card-lines","name":"Contact","layout":"vertical","children":[
      {"type":"text","id":"format-card-line-1","content":"Lantern Repair Café","fontFamily":"$font","fontSize":8,"fontWeight":"600","fill":"$ink","lineHeight":1.35},
      {"type":"text","id":"format-card-line-2","content":"Markveien 58, 0554 Oslo","fontFamily":"$font","fontSize":8,"fill":"$ink","lineHeight":1.35},
      {"type":"text","id":"format-card-line-3","content":"ingrid@lantern-repair.example","fontFamily":"$font","fontSize":8,"fill":"$ink","lineHeight":1.35},
      {"type":"text","id":"format-card-line-4","content":"+47 22 00 01 58","fontFamily":"$font","fontSize":8,"fill":"$ink","lineHeight":1.35}]},
    {"type":"frame","id":"format-card-tile","name":"Mark","width":26,"height":26,"justifyContent":"center","alignItems":"center","cornerRadius":6,"fill":"$accent","children":[
      {"type":"icon","id":"format-card-icon","library":"lucide","icon":"wrench","width":14,"height":14,"fill":"$on-accent"}]}]}]}
```

## A poster

A3 with 3 mm of bleed: 859 x 1208, padding 56.5 (the bleed and a 17 mm margin). A poster is read in three
steps: from across the street one thing (here the headline), from a few steps the date and place, up close
the rest. Size each for its distance: about 5 mm of capital height for every metre, which is some 20
points of type size a metre. On this sheet the headline holds from 6 m, the time from 2, the small text
from a step away.

```json
{"type":"frame","id":"format-poster","name":"Poster A3, with bleed","width":859,"height":1208,"layout":"vertical","padding":56.5,"justifyContent":"space_between","fill":"$accent","clip":true,"children":[
  {"type":"frame","id":"format-poster-head","width":"fill_container","justifyContent":"space_between","children":[
    {"type":"text","id":"format-poster-kicker","name":"Organiser","content":"Lantern Repair Café","fontFamily":"$font","fontSize":30,"fontWeight":"600","fill":"$on-accent"},
    {"type":"text","id":"format-poster-date","name":"Date","content":"Sat 28 Nov","fontFamily":"$font","fontSize":30,"fontWeight":"600","fill":"$on-accent"}]},
  {"type":"text","id":"format-poster-title","name":"Headline","content":"Bring it broken. Take it home working.","fontFamily":"$font","fontSize":132,"fontWeight":"800","fill":"$on-accent","textGrowth":"fixed-width","width":"fill_container","lineHeight":0.98,"letterSpacing":-3},
  {"type":"frame","id":"format-poster-foot","name":"Facts","width":"fill_container","justifyContent":"space_between","alignItems":"end","children":[
    {"type":"text","id":"format-poster-what","name":"What","content":"Toasters, lamps, bikes, zips and phones. Volunteers fix it with you, for free. Coffee is on us.","fontFamily":"$font","fontSize":30,"fill":"$on-accent","textGrowth":"fixed-width","width":420,"lineHeight":1.3},
    {"type":"frame","id":"format-poster-where","name":"Where","layout":"vertical","gap":4,"alignItems":"end","children":[
      {"type":"text","id":"format-poster-time","content":"11:00 to 15:00","fontFamily":"$font","fontSize":44,"fontWeight":"700","fill":"$on-accent"},
      {"type":"text","id":"format-poster-place","content":"Markveien 58, Oslo","fontFamily":"$font","fontSize":30,"fill":"$on-accent"}]}]}]}
```

- A flyer (A5 or A6) is the same three steps at hand distance: a headline of 40 to 60, facts at 12 to 14,
  the rest at 9 to 10.
- Not researched for this skill, so ask the printer or the user for the sizes: folded leaflets, book
  covers and spines, labels, stickers, packaging, T-shirts (a common front print area is 12 x 16 inches).
