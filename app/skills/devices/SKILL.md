---
name: devices
description: Show an app's screen inside the phone or tablet it is for - an iPhone with its real status bar, Dynamic Island, home indicator and side buttons, an Android phone or an iPad - built from a ready template of Pencraft layers. Use whenever the user asks for a mock, mockup, prototype or screen of a phone or tablet app, or names an iPhone, iPad or Android phone, even when the request does not say device or frame. Do not use for an app icon, a logo, one component on its own, or when the user asks for the screen alone.
license: Apache-2.0 with Commons Clause (see LICENSE in the Pencraft repository)
metadata:
  product: pencraft
  version: "1"
---

# Screens on devices in Pencraft

A mock of a phone app is judged by people who hold that phone every day. A screen standing bare on the canvas
looks half done, and a status bar drawn from memory (line icons, the time pushed against the island) gives the
mock away at a glance. So the device is not drawn, it is inserted: the template below is measured from the
iPhone's own status bar and checked in Pencraft. Insert it as it is and change only what this file says.

That exactness is firm, and it is for the device alone, because a phone has one true shape: insert the
template every time a phone screen is asked for, unchanged. The app on its screen is the design and is
free: this skill says nothing about how it should look. Only the user's own words overrule the device:
if they want the screen without one, a different one, or a drawn frame of their own, do that.

## 1. Insert the device first
Insert the template with `edit_layers` as one top-level layer, before you design the app. Change its `x` and
`y` to place it on the canvas; never resize or move the body, the screen, the status bar or what is in it.

- The app goes into the frame "App" (393 × 852, the whole screen). It is under the status bar, the Dynamic
  Island and the home indicator, so its content runs to the screen's edges. Keep its own controls between 54
  from the top and 34 from the bottom; a picture or a colour behind them may fill the whole screen.
- The time: set `fontFamily` to the first of these that is installed: "SF Pro Text", "SF Pro", "Inter",
  "Segoe UI Variable Text", "Segoe UI", "Helvetica Neue", "Arial". Keep size 17, weight 600 and the rest.
- The ink: the status bar and the home indicator are black (`#000000`) over a light top or bottom of the app,
  white over a dark one or over a photograph. For white, change every `#000000` in "Status bar" and "Home
  indicator" to `#ffffff`, the battery "Case" stroke to `#ffffff59` and its "Cap" to `#ffffff66`. The Dynamic
  Island stays black.
- The screen's `fill` is the app's ground (`#ffffff` in the template; a dark app sets its own dark colour).
- The finish: the template is a dark titanium phone (bezel `#0c0c0d`, rim and buttons `#48484a`). For another
  finish change the rim (`stroke` of "Body") and the buttons only: silver `#d4d4d8`, deep blue
  `#2c3a52`, orange `#d9692f`. The bezel stays near black.
- Rename the group to the model the user named. The template is the iPhone 16 (393 × 852 points), which is
  also the iPhone 14 Pro, 15 and 15 Pro; use it as it is when the user names no model. For any other iPhone,
  change it as section 2 says.

```json
{"type":"group","name":"iPhone 16","x":0,"y":0,"children":[
  {"type":"rectangle","name":"Action button","x":0,"y":120,"width":4,"height":30,"cornerRadius":[2,0,0,2],"fill":"#48484a"},
  {"type":"rectangle","name":"Volume up","x":0,"y":175,"width":4,"height":60,"cornerRadius":[2,0,0,2],"fill":"#48484a"},
  {"type":"rectangle","name":"Volume down","x":0,"y":245,"width":4,"height":60,"cornerRadius":[2,0,0,2],"fill":"#48484a"},
  {"type":"rectangle","name":"Side button","x":419,"y":200,"width":4,"height":96,"cornerRadius":[0,2,2,0],"fill":"#48484a"},
  {"type":"rectangle","name":"Camera Control","x":419,"y":543,"width":4,"height":44,"cornerRadius":[0,2,2,0],"fill":"#48484a"},
  {"type":"frame","name":"Body","x":3,"y":0,"width":417,"height":876,"layout":"none","cornerRadius":67,"fill":"#0c0c0d","stroke":"#48484a","strokeWidth":2,"strokeAlignment":"inner","effect":{"type":"shadow","color":"#00000040","offset":{"x":0,"y":24},"blur":48},"children":[
    {"type":"frame","name":"Screen","x":12,"y":12,"width":393,"height":852,"layout":"none","cornerRadius":55,"clip":true,"fill":"#ffffff","children":[
      {"type":"frame","name":"App","x":0,"y":0,"width":393,"height":852,"layout":"none","children":[]},
      {"type":"frame","name":"Status bar","x":0,"y":0,"width":393,"height":54,"layout":"none","children":[
        {"type":"text","name":"Time","content":"9:41","x":33,"y":18,"width":66,"textGrowth":"fixed-width","textAlign":"center","fontFamily":"SF Pro Text","fontSize":17,"fontWeight":"600","letterSpacing":-0.4,"lineHeight":1.3,"fill":"#000000"},
        {"type":"frame","name":"Levels","x":290,"y":23,"layout":"horizontal","gap":6,"alignItems":"center","children":[
          {"type":"path","name":"Cellular","width":17,"height":11,"viewBox":[0,0,17,11],"fill":"#000000","geometry":"M0 8A1 1 0 0 1 1 7H2A1 1 0 0 1 3 8V10A1 1 0 0 1 2 11H1A1 1 0 0 1 0 10Z M4.7 6A1 1 0 0 1 5.7 5H6.7A1 1 0 0 1 7.7 6V10A1 1 0 0 1 6.7 11H5.7A1 1 0 0 1 4.7 10Z M9.4 3.5A1 1 0 0 1 10.4 2.5H11.4A1 1 0 0 1 12.4 3.5V10A1 1 0 0 1 11.4 11H10.4A1 1 0 0 1 9.4 10Z M14 1A1 1 0 0 1 15 0H16A1 1 0 0 1 17 1V10A1 1 0 0 1 16 11H15A1 1 0 0 1 14 10Z"},
          {"type":"path","name":"Wi-Fi","width":16,"height":11.3,"viewBox":[0,0,16,11.3],"fill":"#000000","geometry":"M0.01 3.31A11.3 11.3 0 0 1 15.99 3.31L14.29 5.01A8.9 8.9 0 0 0 1.71 5.01Z M2.84 6.14A7.3 7.3 0 0 1 13.16 6.14L11.47 7.84A4.9 4.9 0 0 0 4.53 7.84Z M5.67 8.97A3.3 3.3 0 0 1 10.33 8.97L8 11.3Z"},
          {"type":"frame","name":"Battery","width":27.5,"height":12,"layout":"none","children":[
            {"type":"rectangle","name":"Case","x":0,"y":0,"width":25,"height":12,"cornerRadius":3.8,"stroke":"#00000059","strokeWidth":1,"strokeAlignment":"inner"},
            {"type":"rectangle","name":"Charge","x":2,"y":2,"width":21,"height":8,"cornerRadius":2,"fill":"#000000"},
            {"type":"rectangle","name":"Cap","x":26,"y":4,"width":1.5,"height":4,"cornerRadius":[0,1,1,0],"fill":"#00000066"}
          ]}
        ]}
      ]},
      {"type":"rectangle","name":"Dynamic Island","x":133.5,"y":11,"width":126,"height":37,"cornerRadius":18.5,"fill":"#000000"},
      {"type":"rectangle","name":"Home indicator","x":129.5,"y":839,"width":134,"height":5,"cornerRadius":2.5,"fill":"#000000"}
    ]}
  ]}
]}
```

## 2. Every iPhone since the XR
Start from the template and change only the numbers below; everything else (the glyphs, the fonts, the inks,
the finish) stays as it is. `W` × `H` is the screen in points, `R` its corners, `M` the width of the island or
the notch, `S` the height of the status bar (the app's controls stay below it).

| Model | W × H | R | Top | M | S | Left side | Camera Control |
|---|---|---|---|---|---|---|---|
| iPhone XR, 11 | 414 × 896 | 41.5 | notch | 223 | 48 | switch | no |
| iPhone XS, 11 Pro | 375 × 812 | 39 | notch | 209 | 44 | switch | no |
| iPhone XS Max, 11 Pro Max | 414 × 896 | 39 | notch | 209 | 44 | switch | no |
| iPhone 12 mini | 375 × 812 | 44 | notch | 226 | 50 | switch | no |
| iPhone 13 mini | 375 × 812 | 44 | notch | 181 | 50 | switch | no |
| iPhone 12, 12 Pro | 390 × 844 | 47.33 | notch | 210 | 47 | switch | no |
| iPhone 12 Pro Max | 428 × 926 | 53.33 | notch | 210 | 47 | switch | no |
| iPhone 13, 13 Pro, 14 | 390 × 844 | 47.33 | notch | 162 | 47 | switch | no |
| iPhone 13 Pro Max, 14 Plus | 428 × 926 | 53.33 | notch | 162 | 47 | switch | no |
| iPhone 16e (2025), 17e (2026) | 390 × 844 | 47.33 | notch | 162 | 47 | Action button | no |
| iPhone 14 Pro, 15 | 393 × 852 | 55 | island | 126 | 54 | switch | no |
| iPhone 15 Pro | 393 × 852 | 55 | island | 126 | 54 | Action button | no |
| iPhone 16 (the template) | 393 × 852 | 55 | island | 126 | 54 | Action button | yes |
| iPhone 14 Pro Max, 15 Plus | 430 × 932 | 55 | island | 126 | 54 | switch | no |
| iPhone 15 Pro Max | 430 × 932 | 55 | island | 126 | 54 | Action button | no |
| iPhone 16 Plus | 430 × 932 | 55 | island | 126 | 54 | Action button | yes |
| iPhone 16 Pro, 17, 17 Pro, 18 Pro | 402 × 874 | 62 | island | 126 | 62 | Action button | yes |
| iPhone Air | 420 × 912 | 62 | island | 126 | 62 | Action button | yes |
| iPhone 16 Pro Max, 17 Pro Max, 18 Pro Max | 440 × 956 | 62 | island | 126 | 62 | Action button | yes |
| iPhone SE (2nd and 3rd generation) | 375 × 667 | 0 | home button | – | 20 | switch | no |

The iPhone 17 (402 × 874) is the current standard model; the newest are the iPhone 18 Pro and 18 Pro Max (September
2026), whose island Apple made smaller without giving its size: keep 126 × 37 unless the user gives one. The
foldable iPhone Duo has no measured size here yet; tell the user so rather than guessing one.

With the row's numbers, set (halves are fine, Pencraft keeps them):
- "Body": `width` W + 24, `height` H + 24, `cornerRadius` R + 12. "Screen": W × H at 12, 12, `cornerRadius` R.
  "App": W × H. "Status bar": W × S. "Side button": `x` W + 26.
- "Dynamic Island" (island rows): `x` (W − 126) / 2; `y` 11, or 14 on the 402, 420 and 440 wide models. Its
  middle is `m` = `y` + 18.5.
- A notch (notch rows) takes the island's place: a rectangle "Notch", M × 32 (30 on the iPhone XS, XS Max, 11
  Pro and 11 Pro Max), `x` (W − M) / 2, `y` 0, `cornerRadius` [0,0,20,20], `fill` `#000000`, always black. The
  status bar's middle is `m` = S / 2.
- "Time": `x` (W − M) / 4 − 33, `y` m − 11. "Levels": `x` (3 × W + M) / 4 − 36.25, `y` m − 6. So the time is
  centred in the strip left of the island or notch, the glyphs in the strip right of it.
- "Home indicator": `x` (W − 134) / 2, `y` H − 13.
- The left side: "Action button" where the row says so; where it says switch, the same rectangle named
  "Ring/Silent switch" with `height` 20.
- "Camera Control": on rows with yes, `x` W + 26 and `y` 0.62 × (H + 24); on rows with no, delete it.

The iPhone SE has no island, no notch and no home indicator; its screen sits between wide bezels:
- "Body" 431 × 887, `cornerRadius` 58; "Screen" 375 × 667 at 28, 110, `cornerRadius` 0; "Side button" `x` 433.
- "Status bar" 20 high: the time centred (`x` 154.5, `y` 1, semibold 12 instead of 17); "Cellular" and
  "Wi-Fi" at 6 from the left, the battery at 6 from the right, all at `y` 4.
- On the bezel: an earpiece 52 × 6, `cornerRadius` 3, at 189.5, 52, and a camera 10 × 10 ellipse at 150, 50,
  both `#1f1f22`; under the screen a home button: an ellipse 64 × 64 at 183.5, 800 with the bezel's fill and a
  2 wide stroke in the rim's colour.

## 3. Several screens
Each screen is a device of its own, from the same template: side by side, 96 apart, level at the top. Put
nothing behind them unless the user asks for a board or a background; then the board is one colour the
device's finish stands out on. Number nothing.

## 4. Other devices
There is no template for these yet. Build them the same way: a body frame with the bezel's fill and a rim of 2,
the screen inside it as a clipped frame holding "App", the status bar and the system marks on top.

- An Android phone (Pixel): body 436 × 939, `cornerRadius` 48; screen 412 × 915 at 12, 12, `cornerRadius` 36.
  Status bar 40 high: the time at 16 from the left, regular 14; on the right Wi-Fi, signal and battery glyphs
  as above, 16 from the edge. A round camera hole, black, 12 across, centred, its middle 20 from the top. A
  gesture bar 108 × 4, `cornerRadius` 2, centred, 8 from the bottom.
- An iPad (11 inch): body 860 × 1220, `cornerRadius` 36; screen 820 × 1180 at 20, 20, `cornerRadius` 18. Status
  bar 24 high: the time and the date ("9:41  Tue 15 Nov") at 20 from the left, semibold 13 and regular 13; the
  glyphs of the template at 20 from the right. No island; a home indicator 320 × 5, 8 from the bottom.

## 5. Check
Take the `screenshot` of the device, not of the screen inside it, so you see what people will see. Then zoom to
the status bar at `scale: 2`: the time sits in the middle of the strip left of the island, the glyphs in the
middle of the strip right of it, both level with the island's middle (on a notch, the status bar's), and in an
ink that reads on what is under them.
