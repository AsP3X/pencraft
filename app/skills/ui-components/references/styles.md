# Named looks

The components of this skill are one neutral build. When the user names a library or one of its styles, keep
the components and change the numbers below. Every value was read from the library's own source in November
2026; values marked (unverified) were not confirmed there. Libraries change: when a number here and the
user's own reference disagree, the user's reference wins.

## shadcn/ui

shadcn/ui has eight styles since 2026. "Classic" in this skill is its Vega style; "Compact" is Nova, which
new shadcn projects get by default (with Base UI, the neutral colour, Lucide icons and Inter). When the user
says only "shadcn", build Classic, say that Compact is the current default, and offer it.

| | Vega (Classic) | Nova (Compact) | Maia | Lyra | Mira | Luma | Sera | Rhea |
|---|---|---|---|---|---|---|---|---|
| Button heights (xs, sm, default, lg) | 24, 32, 36, 40 | 24, 28, 32, 36 | 24, 32, 36, 40 | 24, 28, 32, 36 | 20, 24, 28, 32 | 24, 32, 36, 40 | 28, 36, 40, 44 | 24, 28, 32, 36 |
| Button padding beside the label | 10 | 10 | 12 | 10 | 8 | 12 | 24 | 12 |
| Button radius | 8 | 10 | pill | 0 | 8 | pill | 0 | 18 |
| Button label | 14/500 | 14/500 | 14/500 | 12/500 | 12/500 | 14/500 | 12/600, capitals, spaced 10% | 14/500 |
| Input height, padding | 36, 10 | 32, 10 | 36, 12 | 32, 10 | 28, 8 | 36, 12 | 40, 0 | 32, 10 |
| Input skin | border, clear | border, clear | border, tinted, pill | border, square | border, tinted | no border, tinted, radius 22 | a line under it only | no border, tinted, radius 18 |
| Card radius, padding | 14, 24 | 14, 16 | 18, 24 | 0, 16 | 10, 16 | 26, 24 | 0, 32 | 24, 20 |
| Card edge | hairline 10%, faint shadow | hairline 10% | hairline 10% | hairline 10% | hairline 10% | hairline 5%, floating shadow | hairline 5%, faint shadow | hairline 5%, faint shadow |
| Dialog padding, radius, width | 24, 14, 448 | 16, 14, 384 | 24, 26, 448 | 16, 0, 384 | 16, 14, 384 | 24, 26, 448 | 24, 0, 448 | 24, 24, 448 |
| Menu row (padding beside, above; radius) | 8, 6; 6 | 6, 4; 8 | 12, 8; 14 | 8, 8; 0 | 8, 4; 8 (12 text) | 12, 8; 18 | 12, 8; 0 (12 text, capitals) | 8, 6; 14 |
| Tab list height | 36 | 32 | 36, pill | 32 | 32 | 36, pill | 40 | 32 |
| Usual face and icons | Inter, Lucide | Geist, Lucide | Figtree, Hugeicons | JetBrains Mono | Inter, Hugeicons | Inter, Lucide | Noto Sans with Playfair Display titles | Inter, Lucide |

Pencraft bundles only Lucide: use it for every style and say so when another icon set was named.

shadcn's exact neutral values, for when exact fidelity is asked for (this skill's tokens differ in three
places, named in `references/tokens.md`). The hex values are conversions of its OKLCH values:

| Role | Light | Dark |
|---|---|---|
| background, card, popover | `#ffffff` | `#0a0a0a`; card and popover `#171717` |
| foreground | `#0a0a0a` | `#fafafa` |
| primary, on it | `#171717`, `#fafafa` | `#e5e5e5`, `#171717` |
| secondary, muted, accent | `#f5f5f5` | `#262626` |
| muted-foreground | `#737373` | `#a1a1a1` |
| destructive | `#e7000b` | `#ff6467` |
| border, input | `#e5e5e5` | white at 10%; input white at 15% |
| ring | `#a1a1a1` | `#737373` |

With `#737373`, 12 px labels reach 4.7:1 on white, and Pencraft's review will say so: that remark can stand
when the user asked for shadcn's exact colours.

Other base colours (stone, zinc, mauve, olive, mist, taupe) keep these lightnesses and tint every grey very
slightly toward one hue. Do the same: tint all greys together, never one.

What the classic look changed, against the shadcn of 2025 that most pictures on the web still show. Build
the current one unless the user's reference shows the older:

| Part | Until 2025 ("new-york") | Now |
|---|---|---|
| Padding beside a button's label | 16 | 10 |
| Destructive button | solid red, white label | a 10% tint with red label |
| Edge of cards, menus, dialogs | a solid `border` line | a hairline: the text colour at 10% |
| Scrim behind a dialog | black at 50% | black at 10% with a slight blur |
| Primary button under the pointer | 90% | 80% |
| Outline and ghost button under the pointer | `accent` | `muted` |
| Checked mark in a menu row | leading edge | trailing edge |
| Radius steps | base minus 2, 4 | base times 0.6, 0.8, 1, 1.4 |
| Chart colours of the neutral theme | several hues | greys |
| Component basis | Radix | Base UI by default (Radix still supported) |

This skill's scrim is black at 20% (60% in the dark theme) and has no blur: in a still picture 10% reads as
no scrim at all, and a blurred page reads as a fault. Use `#0000001a` when the user wants shadcn's value
exactly.

Vega also puts the faintest shadow on inputs, select triggers, checkboxes, switches and outline buttons. This
skill leaves it off (as Nova does): an outline with a shadow on every control reads heavy in a still, and
Pencraft's review remarks on it.

## Radix Themes

Sizes are numbered. Size 2 is the desktop default.

| | Size 1 | Size 2 | Size 3 | Size 4 |
|---|---|---|---|---|
| Button height | 24 | 32 | 40 | 48 |
| Button padding beside, gap | 8, 4 | 12, 8 | 16, 12 | 24, 12 |
| Button text | 12 | 14 | 16 | 18 |
| Button radius | 3 | 4 | 6 | 8 |
| Text field and select height, text | 24, 12 | 32, 14 | 40, 16 | none |
| Menu row height, text | 24, 12 | 32, 14 | | |
| Menu padding, radius, row radius | 4, 6, 3 | 8, 8, 4 | | |
| Checkbox and radio | 14 | 16 | 20 | |
| Switch track, thumb | 28 by 16, 14 | 35 by 20, 18 | 42 by 24, 22 | |
| Tab height, text | 32, 12 | 40, 14 | | |
| Card padding, radius | 12, 8 | 16, 8 | 24, 12 | 32, 12 |
| Dialog and popover padding, radius | 12, 8 | 16, 8 | 24, 12 | 32, 12 |
| Table cell padding, least row height | 8, 36 | 12, 44 | 12 by 16, 48 | |

- Space: 4, 8, 12, 16, 24, 32, 40, 48, 64. Type: 12, 14, 16, 18, 20, 24, 28, 35, 60. Radius: 3, 4, 6, 8, 12,
  16 at the medium setting (none: 0; small: three quarters; large: one and a half; full: pills).
- Its tabs are the line variant: a 2 high indicator in the accent colour under the selected tab.
- Variants of a control are built from one 12-step colour scale: solid (step 9 as the fill, white or dark
  text), soft (step 3 at part opacity as the fill, step 11 text), surface (a translucent surface, a step 7
  line, step 11 text), outline (a step 8 line, step 11 text), ghost (text only; step 3 under the pointer).
  In this skill's tokens: solid is primary, soft is the destructive-tint manner with any colour, surface and
  outline are the outline button, ghost is ghost.
- Focus is a 2 wide outline in the accent colour, 2 outside solid buttons, checkboxes, radios and switches,
  and on the border of fields. Use the focus ring with `"spread":2` and the accent at full strength.
- Popups: 4 from the trigger (popover and hover card 8), start edges aligned, 10 from the screen's edge.
- Default theme: indigo accent, translucent panels, medium radius (unverified). Dialog width 600
  (unverified).

Radix Colors gives every hue twelve steps with fixed jobs, which is the way to build any extra colour here:
1 and 2 backgrounds; 3, 4, 5 a control's fill at rest, under the pointer, pressed; 6, 7, 8 lines (subtle,
a control's border, the border under the pointer and the focus ring); 9 and 10 the solid fill and its hover;
11 text in that colour; 12 text of high contrast. Sky, mint, lime, yellow and amber need dark text on their
solid step; every other hue takes white.

## Base UI

Base UI is unstyled; "like Base UI" means its documentation's look: black and white, 1 wide near-black
borders, square corners, 32 high controls, a hard shadow of 4 right and 4 down without blur in black at 12%
on popups. Build Classic with every radius 0, `border` and `input` as `$foreground`, height 32, and that
shadow (Pencraft's review will remark on block shadows; that remark can stand). What Base UI really gives is
part names and states, which this skill already follows.

## Sonner, Vaul, cmdk

The toast, the drawer and the command palette in `references/overlays.md` follow them.

| | Exact values |
|---|---|
| Sonner toast | 356 wide, padding 16, gap 6, radius 8, a 1 line in `#ededed`, shadow 4 down, blur 12, black 10%; title 13/500, description 13/400; action 24 high, 8 beside, radius 4, 12/500; stack of three, 14 apart, each 5% smaller; 24 from the screen's edge (16 and full width under 600) |
| Vaul drawer | handle 32 by 5 in `#e2e2e4` (shadcn's drawer, built on it, uses 100 by 6); the page behind scaled to (screen width - 26) / screen width, radius 8, 14 lower, on black; overlay black at 40% in its examples |
| cmdk palettes | at most 640 wide, shadow 16 down, blur 70, black 20%; rows 14 text, headings 12. Vercel: radius 12, rows 48, input 17. Linear: radius 8, rows 48 edge to edge, the selected row with a 3 wide bar at its start. Raycast: radius 12, rows 40, a 40 high footer with key hints. Framer: radius 16, two panes, the selected row in solid blue |

A command palette opens and closes without motion; a toast is the one thing that may move slowly. Neither
matters in a still, but say so if the user asks how it should behave.
