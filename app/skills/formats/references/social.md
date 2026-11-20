# Social posts, link previews and ads

Sizes in pixels. "Official" was read at the platform's own page for this skill; "third-party" comes from
guides that track the platforms (Sked Social and Sprout Social, read in autumn 2026) and was not checked at
the platform; "disputed" means the guides disagree, and both figures are given. Platforms change these
without notice: when a size matters, the upload page is the authority.

## Sizes

| Place | Piece | Size | Source |
|---|---|---|---|
| Instagram | Feed post, the default | 1080 x 1350 (4:5) | Third-party |
| | Feed post, tall, posted from the app only | 1080 x 1440 (3:4) | Third-party |
| | Square | 1080 x 1080 | Third-party |
| | Landscape | 1080 x 566 (1.91:1) | Third-party |
| | Story, reel | 1080 x 1920 (9:16) | Third-party |
| | Reel cover | 420 x 654 | Third-party |
| | Feed ad | 1440 x 1800 (4:5; 3:4 is not taken) | Third-party |
| | Profile picture | 320 x 320, shown in a circle | Third-party |
| Facebook | Feed image | 1080 x 1080, or 1440 x 1800 (4:5) | Third-party |
| | Page cover | 851 x 315, shown 640 x 360 on a phone | Third-party |
| | Story | 1080 x 1920 | Third-party |
| Threads | Post | 1080 x 1080, or 1440 x 1800 (4:5) | Third-party |
| X | Post image | 1600 x 900, or 1200 x 675 (both 16:9) | Disputed |
| | Header | 1500 x 500 (3:1) | Third-party |
| | Profile picture | 400 x 400, shown in a circle | Third-party |
| LinkedIn | Post image | 1200 x 1200, or 1200 x 627 | Third-party |
| | Document post (a PDF that is swiped) | 1080 x 1350 a page, or 1080 x 1080 | Convention |
| | Personal cover | 1584 x 396 | Third-party |
| | Company cover | 1512 x 256 | Third-party |
| TikTok | Photo post, video cover | 1080 x 1920 (9:16) | Third-party |
| | In-feed ad | 9:16, at least 540 x 960 | Official |
| YouTube | Thumbnail | 3840 x 2160; 1280 x 720 is the same shape and stays under the 2 MB a phone upload allows | Official |
| | Shorts thumbnail | 2160 x 3840 (9:16) | Official |
| | Channel banner | 2560 x 1440, text and logo inside the middle 1235 x 338 | Third-party |
| Pinterest | Pin | 1000 x 1500 (2:3) | Third-party |
| Bluesky | Post image | 1080 x 1350 or 1080 x 1080 | Third-party; Bluesky publishes none |
| Snapchat | Story, ad | 1080 x 1920 | Third-party |
| Apple Podcasts | Show cover | 3000 x 3000, no transparency | Official |

A square of 1080 is shown uncut as a feed post on Instagram, Facebook, LinkedIn, Bluesky and Threads: the
choice when one picture must serve them all.

## What stays visible

| Piece | Keep inside | Why |
|---|---|---|
| Instagram 4:5 post | The middle 1012 across: 34 lost at each side. Use 72 of padding | The profile grid shows posts 3:4 since January 2025 |
| 9:16 for Instagram, TikTok, Shorts, Snapchat | 270 from the top, 660 from the bottom, 120 from each side: an area of 840 x 990 | The platform's own name, caption and buttons lie over the rest |
| YouTube banner | The middle 1235 x 338 of 2560 x 1440 | Phones show only that strip |
| Anything shown in a circle | The circle that fits the square | About a fifth of the square is cut away |
| X header, LinkedIn cover | The right two thirds, away from the lower left | The profile picture overlaps the lower left; the exact area is not published |

The 9:16 figures are a house rule: the larger of two sets. TikTok's own ad template measures 240 top, 660
bottom and 120 at the sides on 1080 x 1920, with a further column 180 wide at the lower right for its
buttons. Meta's ads guide, as quoted by Sked, gives 14% top (269), 35% bottom (672) and 6% at the sides
for Instagram. Other guides give smaller figures. Drawing to 270, 660 and 120 holds on both.

## A post

The default feed piece: 1080 x 1350 with 72 around. Here the sender is on top, the message in the middle
and the facts at the bottom in one block of colour: one arrangement, to show the frame and its padding.
A picture that fills the frame, type as the whole image, the facts on top: all as good, when the design
calls for it.

```json
{"type":"frame","id":"format-post","name":"Instagram post 4:5","width":1080,"height":1350,"layout":"vertical","padding":[72,72],"justifyContent":"space_between","fill":"$paper","clip":true,"children":[
  {"type":"frame","id":"format-post-brand","name":"Brand","gap":29,"alignItems":"center","children":[
    {"type":"frame","id":"format-post-brand-tile","width":72,"height":72,"justifyContent":"center","alignItems":"center","cornerRadius":18,"fill":"$accent","children":[
      {"type":"icon","id":"format-post-brand-icon","library":"lucide","icon":"wrench","width":40,"height":40,"fill":"$on-accent"}]},
    {"type":"text","id":"format-post-brand-name","name":"Name","content":"Lantern Repair Café","fontFamily":"$font","fontSize":36,"fontWeight":"600","fill":"$ink"}]},
  {"type":"frame","id":"format-post-message","name":"Message","width":"fill_container","layout":"vertical","gap":40,"children":[
    {"type":"text","id":"format-post-title","name":"Headline","content":"Bring it broken. Take it home working.","fontFamily":"$font","fontSize":124,"fontWeight":"700","fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.02,"letterSpacing":-2},
    {"type":"text","id":"format-post-detail","name":"Detail","content":"Toasters, lamps, bikes, zips. Our volunteers fix it with you, for free.","fontFamily":"$font","fontSize":44,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.3}]},
  {"type":"frame","id":"format-post-facts","name":"Facts","width":"fill_container","padding":[36,44],"justifyContent":"space_between","alignItems":"end","cornerRadius":28,"fill":"$accent","children":[
    {"type":"frame","id":"format-post-when","layout":"vertical","gap":6,"children":[
      {"type":"text","id":"format-post-when-label","content":"Saturday 28 November","fontFamily":"$font","fontSize":44,"fontWeight":"600","fill":"$on-accent"},
      {"type":"text","id":"format-post-when-time","content":"11:00 to 15:00","fontFamily":"$font","fontSize":44,"fill":"$on-accent"}]},
    {"type":"text","id":"format-post-where","name":"Place","content":"Markveien 58","fontFamily":"$font","fontSize":44,"fontWeight":"600","fill":"$on-accent"}]}]}
```

## A story

1080 x 1920 with the safe area as padding. The ground fills the whole frame; everything else lives in the
middle 840 x 990. A story asks for one thing (a reply, a tap), and that is the last element.

```json
{"type":"frame","id":"format-story","name":"Story 9:16","width":1080,"height":1920,"layout":"vertical","padding":[270,120,660,120],"justifyContent":"space_between","fill":"$paper","clip":true,"children":[
  {"type":"frame","id":"format-story-brand","name":"Brand","gap":29,"alignItems":"center","children":[
    {"type":"frame","id":"format-story-brand-tile","width":72,"height":72,"justifyContent":"center","alignItems":"center","cornerRadius":18,"fill":"$accent","children":[
      {"type":"icon","id":"format-story-brand-icon","library":"lucide","icon":"wrench","width":40,"height":40,"fill":"$on-accent"}]},
    {"type":"text","id":"format-story-brand-name","name":"Name","content":"Lantern Repair Café","fontFamily":"$font","fontSize":36,"fontWeight":"600","fill":"$ink"}]},
  {"type":"frame","id":"format-story-message","name":"Message","width":"fill_container","layout":"vertical","gap":40,"children":[
    {"type":"text","id":"format-story-title","name":"Headline","content":"Bring it broken.","fontFamily":"$font","fontSize":132,"fontWeight":"700","fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.0,"letterSpacing":-2},
    {"type":"text","id":"format-story-detail","name":"Detail","content":"Free repairs this Saturday, 11:00 to 15:00, Markveien 58.","fontFamily":"$font","fontSize":52,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.3}]},
  {"type":"frame","id":"format-story-cue","name":"Cue","padding":[28,44],"cornerRadius":56,"fill":"$accent","children":[
    {"type":"text","id":"format-story-cue-text","content":"What can we fix? Reply and ask","fontFamily":"$font","fontSize":40,"fontWeight":"600","fill":"$on-accent"}]}]}
```

## Carousels

- One shape for all slides; Instagram takes up to 20 in the app (10 through its interface for programs), a
  LinkedIn document many more, a TikTok carousel ad 2 to 35.
- The first slide is the post in the grid and must make someone swipe: the claim, not the introduction.
- A picture that runs across slides is drawn once at the full width (three slides of 1080 are 3240) and cut
  at the slide edges; nothing that is read may lie within 72 of a cut.
- A small page count or arrow in the same corner of every slide, and the thing to do on the last.

## Link preview

The picture a link shows when it is shared. 1200 x 630 is the size that works everywhere; it is a habit,
not a rule of the Open Graph protocol, which only names a picture. X's own large card is given as 2:1,
1.91:1 and 16:9 by different guides, so keep what matters inside the middle 1200 x 600 and away from the
edges: 64 above and below, 80 beside. GitHub's repository preview is 1280 x 640 (official).

```json
{"type":"frame","id":"format-preview","name":"Link preview 1200 x 630","width":1200,"height":630,"layout":"vertical","padding":[64,80],"justifyContent":"space_between","fill":"$paper","clip":true,"children":[
  {"type":"frame","id":"format-preview-brand","name":"Brand","gap":22,"alignItems":"center","children":[
    {"type":"frame","id":"format-preview-brand-tile","width":56,"height":56,"justifyContent":"center","alignItems":"center","cornerRadius":14,"fill":"$accent","children":[
      {"type":"icon","id":"format-preview-brand-icon","library":"lucide","icon":"wrench","width":31,"height":31,"fill":"$on-accent"}]},
    {"type":"text","id":"format-preview-brand-name","name":"Name","content":"Lantern Repair Café","fontFamily":"$font","fontSize":28,"fontWeight":"600","fill":"$ink"}]},
  {"type":"frame","id":"format-preview-message","name":"Message","width":"fill_container","layout":"vertical","gap":20,"children":[
    {"type":"text","id":"format-preview-title","name":"Headline","content":"Bring it broken. Take it home working.","fontFamily":"$font","fontSize":76,"fontWeight":"700","fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.05,"letterSpacing":-1},
    {"type":"text","id":"format-preview-detail","name":"Detail","content":"Free repairs every last Saturday of the month","fontFamily":"$font","fontSize":34,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"}]}]}
```

The title is the page's own, shortened to what fits in two lines. No address bar, no date, no "read more".

## Video thumbnail

1280 x 720. It is seen as small as 170 across beside other thumbnails, so: four words at most, very large,
one picture, strong contrast between the two. YouTube lays the video's length over the lower right corner:
keep that corner free of words (this kit leaves 200 across and 80 up; YouTube publishes no figure).

```json
{"type":"frame","id":"format-thumbnail","name":"Video thumbnail 16:9","width":1280,"height":720,"padding":[64,72],"justifyContent":"space_between","alignItems":"center","fill":"$accent","clip":true,"children":[
  {"type":"frame","id":"format-thumbnail-words","name":"Words","width":640,"layout":"vertical","gap":32,"children":[
    {"type":"text","id":"format-thumbnail-title","name":"Headline","content":"Fix a toaster","fontFamily":"$font","fontSize":150,"fontWeight":"800","fill":"$on-accent","textGrowth":"fixed-width","width":"fill_container","lineHeight":0.98,"letterSpacing":-3},
    {"type":"frame","id":"format-thumbnail-tag","name":"Tag","padding":[14,32],"cornerRadius":16,"fill":"$paper","children":[
      {"type":"text","id":"format-thumbnail-tag-text","content":"in 9 minutes","fontFamily":"$font","fontSize":64,"fontWeight":"700","fill":"$ink"}]}]},
  {"type":"frame","id":"format-thumbnail-picture","name":"Picture","width":400,"height":400,"justifyContent":"center","alignItems":"center","cornerRadius":200,"fill":"$ink","children":[
    {"type":"icon","id":"format-thumbnail-icon","library":"lucide","icon":"wrench","width":260,"height":260,"fill":"$on-accent"}]}]}
```

## Display ads

Google's fixed sizes (official), each at most 150 KB as JPG or PNG:

| Name | Size |
|---|---|
| Medium rectangle | 300 x 250 |
| Large rectangle | 336 x 280 |
| Leaderboard | 728 x 90 |
| Large leaderboard | 970 x 90 |
| Wide skyscraper | 160 x 600 |
| Half page | 300 x 600 |
| Mobile leaderboard | 320 x 50 |

Pictures for its responsive ads (official): 1200 x 1200, 1200 x 628 (1.91:1), 960 x 1200 (4:5) and
1080 x 1920 (9:16); a square logo of 1200. The words beside them are typed in, not drawn: a headline of 40
characters, a description of 90.

- Every ad has the same four parts in the same order of weight: the picture or the claim, the brand, one
  line, a button. In 320 x 50 that is a mark, five words and a button.
- An ad on a white page needs an edge of its own: a ground that is not white, or a 1 wide line.
- A set of ads is one design in seven shapes: recompose it for each, never squeeze it.
