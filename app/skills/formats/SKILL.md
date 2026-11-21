---
name: formats
description: Make a design at the exact size its destination needs, with what matters inside the area that stays visible - social posts, stories and carousels (Instagram, TikTok, LinkedIn, X, Facebook, Pinterest, YouTube thumbnails and banners), link preview and Open Graph images, display ads, app store screenshots and listing graphics (App Store, Google Play, Steam, Chrome Web Store, GitHub), favicons, podcast covers, print (business cards, flyers, posters, postcards, paper sizes with bleed and safe margins) and a picture of an email. Use when the user names one of these, a platform, a paper size or a pixel size, or asks for a post, story, banner, cover, thumbnail, ad, flyer, poster, card or newsletter. Do not use for the screens of an app or site (the ui-components and devices skills), a logo (the logos skill), a chart, or presentation slides.
license: Apache-2.0 with Commons Clause (see LICENSE in the Quill repository)
metadata:
  product: pencraft
  version: "1"
---

# Formats in Quill

A design made for a place is judged in that place: in a feed between other posts, under a platform's
buttons, cut by a printer's knife, an inch wide in a search result. Most of what goes wrong is not taste. It
is a wrong size, a headline under the like button, text that is too small where it is shown, a logo the
circle cut. This skill is the sizes, the areas that stay visible, and the few rules every one of these
shares. The look is still yours to design, by the guide's "Designing well".

Work with Quill's tools (`get_state`, `read_layers`, `edit_layers`, `screenshot`, `export_layers`). The
reference files named below are read with `read_skill` (name `formats`, `file` as given).

Part of this skill is firm and part is a starting point. It says which, and the difference matters:

- Firm: the list "Every time" below, and every size and visible area in the reference files that a
  destination fixes. Do these without being asked, on every design this skill is used for. Only the user's
  own words overrule them; then do what was asked and say in one line what it costs.
- A starting point: where the sender, the message and the facts stand, how many words a piece carries, and
  the whole look of the templates. Begin there, and change it when the design has a reason. They are still
  the expected way: leave one only with a reason, which can be short (the product has two plans, not three;
  the face has no such weight) and needs nobody's leave.
- A design that already has a look, a brand or a system keeps it. Take from this skill what the design
  lacks, and nothing that would replace what it has.
- The examples in the reference files show how a thing is built in Quill and at what size. Their colours,
  faces, words and arrangement are one answer among many, not the look to reproduce.
- The checklist and "When it looks wrong" at the end are there to be gone through every time, as a check
  of your own work. A line in them that speaks of a starting point is a starting point still.

Every time:

1. The size comes first. Look it up (section 1) and make the top-level frame exactly that size before
   anything else. Never design at a size that "looks about right" and scale later.
2. What matters stays inside the safe area: the frame's `padding` is that area, so nothing that is read
   or recognised is placed outside the layout. Backgrounds and pictures run to the frame's edge and past
   the trim.
3. Type is sized for how small the piece is shown, not for how large the frame is (section 3).
4. Sizes that were not read at the platform's own page are marked so in the reference files. Say so in one
   line when you use one, and name the platform's upload page as the place to check.
5. One frame per file to deliver, named for what it is ("Instagram post 4:5"). A series is frames side by
   side, never one tall frame.

Starting points (not part of the list above: begin here, and change one when the user or the design
gives a reason):

- One message per piece: one headline, at most one line under it, one thing to do, the sender. A post is
  not a page.

## 1. Find the size

| The user asks for | Read |
|---|---|
| A post, story, reel cover, carousel, banner, profile picture, thumbnail, a link preview or Open Graph image, a display ad | `references/social.md` |
| App Store or Google Play screenshots, a feature graphic, a Steam capsule, a Chrome Web Store tile, a GitHub preview, a favicon set, a podcast cover | `references/stores.md` |
| A business card, flyer, poster, postcard, a sheet of A4 or Letter, anything a printer cuts | `references/print.md` |
| A newsletter, a transactional email | `references/email.md` |

If the user gives a size in pixels, millimetres or inches, that size wins over the table. If they name a
platform and not a format, take the format the tables mark as the default and say which you took. If the
piece is for several places, make one frame per place: a 4:5 post, a 9:16 story and a 1.91:1 preview are
three compositions of one idea, not one picture cropped three ways.

## 2. The frame

- A top-level frame of the exact size, with `"clip":true`, a `fill` (never transparent, unless the format
  asks for it: an app icon for Google Play, a favicon), and the safe area as its `padding`. Its layout is
  vertical with `"justifyContent":"space_between"` for most pieces: the sender on top, the message in the
  middle, the facts or the thing to do at the bottom.
- A background that should reach the edge is the frame's own fill, or a layer of the frame's size with
  `"layoutPosition":"absolute"` listed first.
- The colours and the face are variables. A document without any gets a small set (below) to start from;
  change every value to the design's own. A document that has them keeps them.

```json
{"op":"set_variables",
 "variables":{
  "paper":{"type":"color","value":"#f3f5f1"},
  "ground":{"type":"color","value":"#e3e7e0"},
  "ink":{"type":"color","value":"#13201a"},
  "ink-soft":{"type":"color","value":"#42524a"},
  "line":{"type":"color","value":"#13201a24"},
  "accent":{"type":"color","value":"#c7361a"},
  "on-accent":{"type":"color","value":"#fff7f2"},
  "font":{"type":"string","value":"Segoe UI Variable"}}}
```

- A mark or picture that the platform shows in a circle (profile pictures, some ad logos) keeps everything
  inside the circle that fits the square: the corners are lost, about a fifth of the area.
- Never draw the platform's own interface (the like button, the status bar, a browser bar) into a piece
  that will be uploaded. To show the piece in its place, make a second frame for that and say it is a
  mock-up.

## 3. Type for the size it is shown at

A piece is drawn large and seen small. What counts is the size on the screen or the page. These floors are
worked out from the width a piece is shown at (a phone shows 1080 across about 375 points), not taken from
a platform's page: no platform publishes type sizes.

| Piece | Frame | Smallest text | Body | Headline |
|---|---|---|---|---|
| Feed post, story, carousel slide | 1080 wide | 32 | 40 to 52 | 96 to 140, three lines at most |
| Link preview | 1200 by 630, often shown 500 wide or less | 28 | 32 to 36 | 60 to 80, two lines |
| Video thumbnail | 1280 by 720, shown as small as 170 wide | 64 | - | 120 to 180, four words at most |
| Display ad, small tile | Under 400 on its long side | 12 | 14 to 16 | 20 to 28 |
| App Store screenshot | 1320 wide | 40 | - | 100 to 130 for the caption, two lines |
| Print, held in the hand | In points | 6 (8 light on dark) | 8 to 10 | By the piece |
| Poster, read from across a room | In points | 24 | 28 to 36 | 5 mm of capital height per metre of distance: about 100 for 5 m |
| Email | 600 wide | 13 | 16 | 28 to 36 |

Count the words too. A feed post carries about a dozen words, a thumbnail four, a story's headline six.
What does not fit belongs in the caption, the description or the page the piece links to.

## 4. A series

Carousel slides, a set of store screenshots, a card's front and back: frames of one size side by side, 48
apart, tops level, named with their number ("Slide 2 of 5"). They share margins, type sizes and the place of
the recurring parts (the mark, the page count), so the eye stays put when the slide changes. The first one
must stand by itself: it is the one a grid, a store listing or a preview shows. Each slide says one thing;
the last one says what to do.

## 5. Export

`export_layers` with the frame's `id` writes the file; the ending picks the format.

| For | Format | Note |
|---|---|---|
| Social posts, previews, ads, thumbnails | `.png` at `scale` 1, or `.jpg` when the platform caps the file (a YouTube thumbnail 2 MB on a phone, a display ad 150 KB) | The frame's size is the file's size |
| Store screenshots | `.png` or `.jpg`, no transparency | The frame has a fill, so there is none |
| An icon with transparent corners | `.png` | The one case where the frame has no fill |
| Print | `.pdf`: one unit is one point, text stays text, shapes stay shapes | Draw print in points (`references/print.md`) |
| Print, when the printer wants a picture | `.png` at `scale` 3 from a frame in points is 216 dots per inch; for 300, draw the frame at the pixel size in the print file's table | A picture holds at most 64 million pixels: A1 and larger at 300 dots per inch do not fit |

Export one frame per file. Tell the user the file's size in pixels (or millimetres) and where it is.

## 6. What Quill cannot do, and what to do instead

| Missing | Instead |
|---|---|
| CMYK, spot colours, rich black | Colours are sRGB. Say that the printer converts them, and keep away from the brightest greens, oranges and blues, which shift most |
| Crop marks, a bleed box in the PDF | Draw the frame at the size with bleed and tell the user the trim size and the bleed, so the printer can set them |
| A PDF of several pages (a card's two sides, a folded flyer) | One file per page |
| Motion and sound (a reel, an animated ad) | The cover or the key frame as a picture; say that it is one frame |
| A real email (HTML that mail programs take) | A picture of the email, to brief whoever builds it |
| Checking a file's weight before export | Export, then tell the user the limit the platform sets |

## 7. Checklist

- The frame has the size the table or the user gave, to the pixel; its name says what it is.
- Everything read or recognised is inside the padding; only ground and picture reach the edge.
- One message; the word count is inside the piece's limit; nothing is under the smallest size.
- The sender can be told (a mark or a name), once.
- Dates, times, places and addresses are real and agree across the pieces of a set.
- A size that is third-party or disputed was named as such to the user.
- Looked at small: `screenshot` the frame, then judge it at the width it is shown at, not at full size.

## 8. When it looks wrong

| It looks like | Cause | Fix |
|---|---|---|
| A web page in a square | Navigation, three points, a paragraph, a footer | One headline, one fact, the sender |
| Fine here, unreadable in the feed | Type sized for the frame | The floors of section 3 |
| Cut off | Text in the lowest third of a 9:16 piece, a headline at the very edge of a 4:5 post, a logo in a corner of a circle | The safe area as padding |
| A template | The stock gradient with centred white text | A direction by "Designing well"; the format fixes the size, not the look |
| A blurry print | A small picture stretched, a PNG where a PDF was wanted | Vector PDF; pictures at 300 dots per inch at their printed size |
| A white sliver at the edge of a printed card | No bleed | The ground runs 3 mm past the trim on every side |
