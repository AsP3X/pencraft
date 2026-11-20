# Stores and listings

Sizes in pixels, read at each store's own page in autumn 2026 unless marked.

## Apple App Store

| Piece | Size | Note |
|---|---|---|
| iPhone screenshot, large | 1320 x 2868 (also 1290 x 2796, 1260 x 2736) | Draw this one: the store scales it down for smaller phones |
| iPhone screenshot, medium | 1206 x 2622 (also 1179 x 2556) | |
| iPad 13 inch | 2064 x 2752 (also 2048 x 2732) | Needed if the app runs on an iPad |
| iPad 11 inch | 1668 x 2420 | |
| Mac | 2880 x 1800, 2560 x 1600, 1440 x 900 or 1280 x 800 (16:10) | |
| Apple TV | 1920 x 1080 or 3840 x 2160 | |
| Vision Pro | 3840 x 2160 | |
| Apple Watch | 422 x 514, 410 x 502, 416 x 496 | |
| App icon | 1024 x 1024, square corners, no transparency | Not re-read for this skill; the long-standing figure |

One to ten screenshots a size, JPEG or PNG, no transparency. Which iPhone class Apple requires is read
differently by different guides (the large one, or the medium one): draw the large one and check in App
Store Connect. Landscape is the same sizes turned.

## Google Play

| Piece | Size | Note |
|---|---|---|
| App icon | 512 x 512 PNG, transparency allowed | The store rounds the corners itself |
| Feature graphic | 1024 x 500, no transparency | Logo, name and line near the middle: the edges are cut on some screens |
| Phone screenshots | 1080 x 1920 or 1920 x 1080; 2 to 8; each side 320 to 3840, the long side at most 2.3 times the short | |
| Tablet screenshots | Sides 1080 to 7680, at least 4 | |
| Wear OS | 384 x 384 or larger, square | |
| TV banner | 1280 x 720 | |

Play asks for strong colour and against pure white, black or dark grey grounds (they vanish into the
store's own), against pictures of devices, store badges, and words such as "Best", "#1" or "Free".

```json
{"type":"frame","id":"format-feature","name":"Play feature graphic","width":1024,"height":500,"gap":40,"justifyContent":"center","alignItems":"center","fill":"$accent","clip":true,"children":[
  {"type":"frame","id":"format-feature-tile","name":"Icon","width":128,"height":128,"justifyContent":"center","alignItems":"center","cornerRadius":28,"fill":"$ink","children":[
    {"type":"icon","id":"format-feature-icon","library":"lucide","icon":"wrench","width":72,"height":72,"fill":"$on-accent"}]},
  {"type":"frame","id":"format-feature-words","layout":"vertical","gap":4,"children":[
    {"type":"text","id":"format-feature-name","name":"Name","content":"Lantern","fontFamily":"$font","fontSize":84,"fontWeight":"700","fill":"$on-accent","letterSpacing":-1},
    {"type":"text","id":"format-feature-line","name":"Line","content":"Find a repair café near you","fontFamily":"$font","fontSize":34,"fill":"$on-accent"}]}]}
```

## A store screenshot

A screenshot in a store is an advertisement with a screen in it: a caption of a few words on top, the screen
under it, running off the bottom edge. 1320 x 2868, the caption 200 from the top and 110 from the sides,
100 to 130 in size and two lines at most; the screen as wide as the frame less 110 at each side. A set of
five tells five things the app does, one a frame, with one ground colour or a quiet change between them.
For the phone itself around the screen, use the `devices` skill; for the screen's content, the
ui-components skill. On Google Play leave the device away.

```json
{"type":"frame","id":"format-shot","name":"App Store screenshot","width":1320,"height":2868,"layout":"vertical","gap":120,"padding":[200,110,0,110],"fill":"$accent","clip":true,"children":[
  {"type":"frame","id":"format-shot-caption","name":"Caption","width":"fill_container","padding":[0,40],"children":[
    {"type":"text","id":"format-shot-title","name":"Headline","content":"Find a repair café near you","fontFamily":"$font","fontSize":116,"fontWeight":"700","fill":"$on-accent","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.04,"letterSpacing":-2,"textAlign":"center"}]},
  {"type":"frame","id":"format-shot-screen","name":"Screen","width":"fill_container","height":"fill_container","layout":"vertical","gap":24,"padding":[120,72,0,72],"cornerRadius":[96,96,0,0],"fill":"$paper","clip":true,"children":[
    {"type":"text","id":"format-shot-screen-title","name":"Title","content":"Near you","fontFamily":"$font","fontSize":84,"fontWeight":"700","fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"frame","id":"format-shot-list","name":"List","width":"fill_container","layout":"vertical","children":[
      {"type":"frame","id":"format-shot-row-1","name":"Row","width":"fill_container","padding":[48,0],"gap":24,"alignItems":"center","stroke":"$line","strokeWidth":{"bottom":2},"children":[
        {"type":"frame","id":"format-shot-row-1-text","width":"fill_container","layout":"vertical","gap":8,"children":[
          {"type":"text","id":"format-shot-row-1-title","content":"Lantern Repair Café","fontFamily":"$font","fontSize":52,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"format-shot-row-1-detail","content":"Markveien 58 · Sat 11:00","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"}]},
        {"type":"icon","id":"format-shot-row-1-icon","library":"lucide","icon":"chevron-right","width":44,"height":44,"fill":"$ink-soft"}]},
      {"type":"frame","id":"format-shot-row-2","name":"Row","width":"fill_container","padding":[48,0],"gap":24,"alignItems":"center","stroke":"$line","strokeWidth":{"bottom":2},"children":[
        {"type":"frame","id":"format-shot-row-2-text","width":"fill_container","layout":"vertical","gap":8,"children":[
          {"type":"text","id":"format-shot-row-2-title","content":"Fikselaget Tøyen","fontFamily":"$font","fontSize":52,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"format-shot-row-2-detail","content":"Hagegata 22 · Sun 12:00","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"}]},
        {"type":"icon","id":"format-shot-row-2-icon","library":"lucide","icon":"chevron-right","width":44,"height":44,"fill":"$ink-soft"}]},
      {"type":"frame","id":"format-shot-row-3","name":"Row","width":"fill_container","padding":[48,0],"gap":24,"alignItems":"center","stroke":"$line","strokeWidth":{"bottom":2},"children":[
        {"type":"frame","id":"format-shot-row-3-text","width":"fill_container","layout":"vertical","gap":8,"children":[
          {"type":"text","id":"format-shot-row-3-title","content":"Sagene Verksted","fontFamily":"$font","fontSize":52,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"format-shot-row-3-detail","content":"Arendalsgata 3 · Wed 17:00","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"}]},
        {"type":"icon","id":"format-shot-row-3-icon","library":"lucide","icon":"chevron-right","width":44,"height":44,"fill":"$ink-soft"}]},
      {"type":"frame","id":"format-shot-row-4","name":"Row","width":"fill_container","padding":[48,0],"gap":24,"alignItems":"center","stroke":"$line","strokeWidth":{"bottom":2},"children":[
        {"type":"frame","id":"format-shot-row-4-text","width":"fill_container","layout":"vertical","gap":8,"children":[
          {"type":"text","id":"format-shot-row-4-title","content":"Reparatørene Grünerløkka","fontFamily":"$font","fontSize":52,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"format-shot-row-4-detail","content":"Thorvald Meyers gate 41 · Thu 16:30","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"}]},
        {"type":"icon","id":"format-shot-row-4-icon","library":"lucide","icon":"chevron-right","width":44,"height":44,"fill":"$ink-soft"}]},
      {"type":"frame","id":"format-shot-row-5","name":"Row","width":"fill_container","padding":[48,0],"gap":24,"alignItems":"center","stroke":"$line","strokeWidth":{"bottom":2},"children":[
        {"type":"frame","id":"format-shot-row-5-text","width":"fill_container","layout":"vertical","gap":8,"children":[
          {"type":"text","id":"format-shot-row-5-title","content":"Bjølsen Fiksefest","fontFamily":"$font","fontSize":52,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"format-shot-row-5-detail","content":"Maridalsveien 120 · Sat 12:00","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"}]},
        {"type":"icon","id":"format-shot-row-5-icon","library":"lucide","icon":"chevron-right","width":44,"height":44,"fill":"$ink-soft"}]},
      {"type":"frame","id":"format-shot-row-6","name":"Row","width":"fill_container","padding":[48,0],"gap":24,"alignItems":"center","stroke":"$line","strokeWidth":{"bottom":2},"children":[
        {"type":"frame","id":"format-shot-row-6-text","width":"fill_container","layout":"vertical","gap":8,"children":[
          {"type":"text","id":"format-shot-row-6-title","content":"Gamlebyen Sykkelkjeller","fontFamily":"$font","fontSize":52,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"format-shot-row-6-detail","content":"Schweigaards gate 60 · Tue 18:00","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"}]},
        {"type":"icon","id":"format-shot-row-6-icon","library":"lucide","icon":"chevron-right","width":44,"height":44,"fill":"$ink-soft"}]},
      {"type":"frame","id":"format-shot-row-7","name":"Row","width":"fill_container","padding":[48,0],"gap":24,"alignItems":"center","stroke":"$line","strokeWidth":{"bottom":2},"children":[
        {"type":"frame","id":"format-shot-row-7-text","width":"fill_container","layout":"vertical","gap":8,"children":[
          {"type":"text","id":"format-shot-row-7-title","content":"Majorstuen Syhjørne","fontFamily":"$font","fontSize":52,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"format-shot-row-7-detail","content":"Bogstadveien 9 · Mon 17:00","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"}]},
        {"type":"icon","id":"format-shot-row-7-icon","library":"lucide","icon":"chevron-right","width":44,"height":44,"fill":"$ink-soft"}]},
      {"type":"frame","id":"format-shot-row-8","name":"Row","width":"fill_container","padding":[48,0],"gap":24,"alignItems":"center","children":[
        {"type":"frame","id":"format-shot-row-8-text","width":"fill_container","layout":"vertical","gap":8,"children":[
          {"type":"text","id":"format-shot-row-8-title","content":"Nydalen Elektrobord","fontFamily":"$font","fontSize":52,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"format-shot-row-8-detail","content":"Gullhaugveien 7 · Sun 13:00","fontFamily":"$font","fontSize":40,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"}]},
        {"type":"icon","id":"format-shot-row-8-icon","library":"lucide","icon":"chevron-right","width":44,"height":44,"fill":"$ink-soft"}]}]}]}]}
```

## Steam

| Piece | Size |
|---|---|
| Header capsule | 920 x 430 |
| Small capsule | 462 x 174 |
| Main capsule | 1232 x 706 |
| Vertical capsule | 748 x 896 |
| Library capsule | 600 x 900 |
| Library hero | 3840 x 1240, artwork only, no logo |
| Library logo | PNG, 1280 wide or 720 high, transparent |
| Screenshots | 1920 x 1080 or larger, 16:9 |
| Page background | 1438 x 810, quiet |
| Event cover, event header | 800 x 450, 1920 x 622 |

The sizes in force since August 2024; the older, smaller ones are refused. Capsules carry the game's logo
and artwork and nothing else. The four store capsules have four shapes: compose each from the same logo and
art, do not crop one.

## Chrome Web Store

| Piece | Size | Note |
|---|---|---|
| Icon | 128 x 128 PNG, the artwork in the middle 96 x 96 | Must hold on light and dark grounds |
| Screenshots | 1280 x 800 (or 640 x 400), 1 to 5, square corners | Shown at 640 x 400 |
| Small tile | 440 x 280 | |
| Marquee | 1400 x 560 | |

Tiles: little or no text, saturated colour to the edge, the brand rather than a screenshot, and it must
still work at half its size.

## Other listings

| Piece | Size | Source |
|---|---|---|
| GitHub repository preview | 1280 x 640 (at least 640 x 320, under 1 MB) | Official |
| Product Hunt gallery, thumbnail | 1270 x 760; 240 x 240 | Third-party, disputed |
| Favicon set | `favicon.ico` 32 x 32; `icon.svg`; `apple-touch-icon.png` 180 x 180; 192 x 192 and 512 x 512 for the manifest; a maskable 512 with the mark inside the middle circle of 409 | Evil Martians' guide, January 2026 |
| Apple Podcasts cover | 3000 x 3000, no transparency, the title readable at 60 across | Official |

A favicon is drawn at 32 first: one shape, two colours, no letters thinner than 3. The larger sizes are
the same mark with more room around it, not more detail. (For the mark itself: the `logos` skill.)
