# Platform design languages an AI design tool should reproduce as static mock-ups (state: 20 November 2026)

Reading notes for the report writer

- "Verified" below means the value was read this session from the cited primary source (token source files, Apple HIG page data, Microsoft docs markdown). Values I only know from memory are kept out of "Cited Findings" and listed under "Gaps" as *unverified, from memory*.
- Material 3 numbers were read from the Jetpack Compose Material 3 token files (generated from Google's Material token database, `androidx-main` branch). m3.material.io itself is JavaScript-rendered and was not readable; the token files are the same numbers the spec pages publish, but spec-page-only values (paddings not expressed as tokens) are missing.
- Apple numbers were read from the HIG page data (`developer.apple.com/tutorials/data/design/human-interface-guidelines/<page>.json`), which is what the public HIG pages render.
- Date premise check: the brief says "iOS 26 / macOS 26 era". Apple announced iOS 27 / macOS 27 "Golden Gate" at WWDC on 8 June 2026 with Liquid Glass refinements. I did not confirm the autumn 2026 public release, but the HIG change logs carry 2026 edits (see Apple section). Treat "iOS 26" numbers as the base and the iOS 27 changes as a flagged gap.

## 1. Material 3 and Material 3 Expressive (colour, type, shape, elevation, states, component anatomy, window size classes, Android system bars)

### Takeaway
Every foundation value and most component dimensions for Material 3 Expressive can be drawn from the Compose token files: 15 type roles plus 15 "emphasized" twins, a 10-step corner scale (new 20/32/48 dp steps), six elevation levels, four state-layer opacities, tone-number colour roles, and Expressive component sizes (buttons XS 32 to XL 136 dp, FAB 56/80/96, 64 dp navigation bar, 96 dp rail, 64 dp toolbars). Android gives no official dp heights for status/navigation bars; it only prescribes transparent, edge-to-edge bars.

### Cited Findings

**Colour roles as tone numbers (light / dark).** Palettes are Primary, Secondary, Tertiary, Neutral (N), Neutral Variant (NV), Error; tone 0 = black, 100 = white. — [ColorLightTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/ColorLightTokens.kt), [ColorDarkTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/ColorDarkTokens.kt)

| Role | Light | Dark |
|---|---|---|
| primary / onPrimary | P40 / P100 | P80 / P20 |
| primaryContainer / onPrimaryContainer | P90 / P10 | P30 / P90 |
| secondary / onSecondary | S40 / S100 | S80 / S20 |
| secondaryContainer / onSecondaryContainer | S90 / S10 | S30 / S90 |
| tertiary / onTertiary | T40 / T100 | T80 / T20 |
| tertiaryContainer / onTertiaryContainer | T90 / T10 | T30 / T90 |
| error / onError | E40 / E100 | E80 / E20 |
| errorContainer / onErrorContainer | E90 / E10 | E30 / E90 |
| primaryFixed / primaryFixedDim (same in both themes) | P90 / P80 | P90 / P80 |
| onPrimaryFixed / onPrimaryFixedVariant | P10 / P30 | P10 / P30 |
| (secondary and tertiary "fixed" roles follow the same 90 / 80 / 10 / 30 pattern) | | |
| surface (= background) | N98 | N6 |
| surfaceDim / surfaceBright | N87 / N98 | N6 / N24 |
| surfaceContainerLowest / Low / (default) / High / Highest | N100 / N96 / N94 / N92 / N90 | N4 / N10 / N12 / N17 / N22 |
| onSurface (= onBackground) | N10 | N90 |
| surfaceVariant / onSurfaceVariant | NV90 / NV30 | NV30 / NV80 |
| outline / outlineVariant | NV50 / NV80 | NV60 / NV30 |
| inverseSurface / inverseOnSurface / inversePrimary | N20 / N95 / P80 | N90 / N20 / P40 |
| scrim | N0 | N0 |
| surfaceTint | = primary | = primary |

- Scrim opacity is 32%. — [ScrimTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/ScrimTokens.kt)
- Baseline (purple seed) hex values for the tones above, converted from the RGB in the palette file: P10 #21005D, P20 #381E72, P30 #4F378B, P40 #6750A4, P80 #D0BCFF, P90 #EADDFF; S30 #4A4458, S40 #625B71, S80 #CCC2DC, S90 #E8DEF8; T40 #7D5260, T80 #EFB8C8, T90 #FFD8E4; E10 #410E0B, E20 #601410, E30 #8C1D18, E40 #B3261E, E80 #F2B8B5, E90 #F9DEDC; N4 #0F0D13, N6 #141218, N10 #1D1B20, N12 #211F26, N17 #2B2930, N20 #322F35, N22 #36343B, N24 #3B383E, N87 #DED8E1, N90 #E6E0E9, N92 #ECE6F0, N94 #F3EDF7, N95 #F5EFF7, N96 #F7F2FA, N98 #FEF7FF; NV30 #49454F, NV50 #79747E, NV60 #938F99, NV80 #CAC4D0, NV90 #E7E0EC. — [PaletteTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/PaletteTokens.kt)

**Type scale (sp = dp at default font scale): size / line height / tracking / weight.** — [TypeScaleTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/TypeScaleTokens.kt)

| Role | Size / line | Tracking | Weight | Emphasized weight |
|---|---|---|---|---|
| Display Large | 57 / 64 | -0.25 | 400 | 500 |
| Display Medium | 45 / 52 | 0 | 400 | 500 |
| Display Small | 36 / 44 | 0 | 400 | 500 |
| Headline Large | 32 / 40 | 0 | 400 | 500 |
| Headline Medium | 28 / 36 | 0 | 400 | 500 |
| Headline Small | 24 / 32 | 0 | 400 | 500 |
| Title Large | 22 / 28 | 0 | 400 | 500 |
| Title Medium | 16 / 24 | 0.15 | 500 | 700 |
| Title Small | 14 / 20 | 0.1 | 500 | 700 |
| Body Large | 16 / 24 | 0.5 | 400 | 500 |
| Body Medium | 14 / 20 | 0.25 | 400 | 500 |
| Body Small | 12 / 16 | 0.4 | 400 | 500 |
| Label Large | 14 / 20 | 0.1 | 500 | 700 |
| Label Medium | 12 / 16 | 0.5 | 500 | 700 |
| Label Small | 11 / 16 | 0.5 | 500 | 700 |

- The Expressive "emphasized" styles keep size, line height and tracking and only raise weight (Regular to Medium; Medium to Bold). Display, Headline and Title Large use the "Brand" typeface slot, the rest "Plain"; both default to the platform sans-serif in Compose. — [TypeScaleTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/TypeScaleTokens.kt), [TypefaceTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/TypefaceTokens.kt)

**Shape scale (corner radius, dp).** None 0, Extra Small 4, Small 8, Medium 12, Large 16, Large Increased 20 (new in Expressive), Extra Large 28, Extra Large Increased 32 (new), Extra Extra Large 48 (new), Full = pill/circle. — [ShapeTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/ShapeTokens.kt)

**Elevation (dp).** Level 0 = 0, 1 = 1, 2 = 3, 3 = 6, 4 = 8, 5 = 12. — [ElevationTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/ElevationTokens.kt)

**State layers (overlay of the content colour).** Hover 8%, focus 10%, pressed 10%, dragged 16%. Disabled content in several components is 38% (radio icon, menu label); disabled outlined-button container 10%. — [StateTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/StateTokens.kt), [RadioButtonTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/RadioButtonTokens.kt)

**Component anatomy (all dp; "font" names refer to the type table).** All rows from the token directory: [androidx Material 3 tokens](https://github.com/androidx/androidx/tree/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens) (file named in each row).

| Component | Values | File |
|---|---|---|
| Top app bar, shared | icon 24, avatar 32, leading/trailing space 4, flat (level 0), level 2 when scrolled under, square corners | AppBarTokens |
| Top app bar, small | height 64; title Title Large; subtitle Label Medium | AppBarSmallTokens |
| Top app bar, medium / large (older) | 112, title Headline Small / 152, title Headline Medium | AppBarMediumTokens, AppBarLargeTokens |
| Top app bar, medium flexible (Expressive) | 112 (136 with subtitle); title Headline Medium; subtitle Label Large | AppBarMediumFlexibleTokens |
| Top app bar, large flexible (Expressive) | 120 (152 with subtitle); title Display Small; subtitle Title Medium | AppBarLargeFlexibleTokens |
| Bottom app bar | height 80, level 2 | BottomAppBarTokens |
| Navigation bar (Expressive) | height 64 (older "tall" 80); level 2; label Label Medium; icon 24; active indicator 56 x 32 pill, 4 between indicator and label; horizontal-item variant (wider windows): indicator height 40 with 16 side padding | NavigationBarTokens, NavigationBarVerticalItemTokens, NavigationBarHorizontalItemTokens |
| Navigation rail, collapsed | width 96 (narrow 80); top space 44; item indicator 56 x 32; icon-label gap 4; 4 between items; label Label Medium | NavigationRailCollapsedTokens, NavigationRailVerticalItemTokens |
| Navigation rail, expanded | width 220 to 360; item height 56 pill, 16 leading, icon-label gap 8, label Label Large; modal version has level 2 and 16 corners | NavigationRailExpandedTokens, NavigationRailHorizontalItemTokens |
| Navigation rail, older baseline | item container height 64, indicator padding 16, icon 24 | NavigationRailBaselineItemTokens |
| Navigation drawer | width 360; item indicator 336 x 56 pill; icon 24; label Label Large; headline Title Small; trailing-side corners 16; modal level 1 | NavigationDrawerTokens |
| FAB | baseline 56 x 56, corner 16, icon 24; small 40, corner 12; medium 80, corner 20, icon 28; large 96, corner 28, icon 32; level 3 | FabBaselineTokens, FabSmallTokens, FabMediumTokens, FabLargeTokens |
| Extended FAB | small: height 56, corner 16, icon 24, gap 8, side padding 16; medium: 80, corner 20, icon 28, gap 16, padding 26; large: 96, corner 28, icon 32, gap 20, padding 28; label Label Large (small) | ExtendedFabSmallTokens, ExtendedFabMediumTokens, ExtendedFabLargeTokens |
| FAB menu | close button 56 circle, icon 20; list item height 56 pill, icon 24, gap 8, side padding 24, 4 between items | FabMenuBaselineTokens |
| Buttons (Expressive sizes) | XS: h 32, side padding 16, icon 20, gap 8, square corner 12, pressed corner 8. S: h 40, padding 16, icon 20, gap 8, square 12, pressed 8. M: h 56, padding 24, icon 24, gap 8, square 16, pressed 12. L: h 96, padding 48, icon 32, gap 12, square 28, pressed 16, outline 2. XL: h 136, padding 64, icon 40, gap 16, square 28, pressed 16, outline 3. Round shape = pill for all sizes; outline 1 for XS to M | ButtonXSmallTokens ... ButtonXLargeTokens |
| Button, pre-Expressive baseline | h 40, side padding 24, icon 20, gap 8, pill | BaselineButtonTokens |
| Shape morph | Default shape is round (pill) or square (size-specific corner above); pressed state morphs to the smaller "pressed" corner; a selected toggle button swaps round and square | Button*Tokens, *IconButtonTokens |
| Icon buttons (Expressive sizes) | XS 32 (icon 20), S 40 (icon 24), M 56 (icon 24), L 96 (icon 32), XL 136 (icon 40); widths narrow / default / wide via side padding: XS 4/6/10, S 4/8/14, M 12/16/24, L 16/32/48, XL 32/48/72 | XSmallIconButtonTokens ... XLargeIconButtonTokens |
| Button group, standard | height 40 (small), 12 between buttons | ButtonGroupSmallTokens |
| Button group, connected | height 40, 2 between segments, outer ends pill, inner corners 8 (4 when pressed), selected segment fully pill | ConnectedButtonGroupSmallTokens |
| Split button | gap 2 between the two parts; inner corners 4 (XS to M), 8 (L), 12 (XL); heights 32/40/56/96/136; trailing icon 22/22/26/38/50; leading padding (start/end) 12/10, 16/12, 24/24, 48/48, 64/64; trailing padding 13, 13, 15, 29, 43 | SplitButtonXSmallTokens ... SplitButtonXLargeTokens |
| Segmented button (older) | height 40, pill, outline 1, icon 18, Label Large | OutlinedSegmentedButtonTokens |
| Chips (assist, filter, input, suggestion) | height 32, corner 8, outline 1 (none when selected), icon 18, input-chip avatar 24, Label Large, elevated variant level 1 | AssistChipTokens, FilterChipTokens, InputChipTokens, SuggestionChipTokens |
| Cards | corner 12; elevated level 1; filled level 0; outlined level 0 with 1 outline | ElevatedCardTokens, FilledCardTokens, OutlinedCardTokens |
| Lists | one-line 56, two-line 72, three-line 88; side padding 16; top/bottom 10; 12 between elements; leading icon 24 (Expressive 20), avatar 40, image 56 x 56, video 100 x 56 (large 114 x 64); label Body Large, supporting Body Medium, overline and trailing text Label Small; Expressive segmented lists: 2 gap between items, item corner 4, selected/pressed corner 16, list container corner 16; divider inset 16 | ListTokens |
| Text field, filled | top corners 4; indicator line 1 (2 focused); input and label Body Large; supporting Body Small; icons 24 | FilledTextFieldTokens |
| Text field, outlined | height 56; corner 4; outline 1 (2 focused); same fonts and icons | OutlinedTextFieldTokens |
| Switch | track 52 x 32, outline 2; handle 16 unselected, 24 selected or with icon, 28 pressed; icon 16; state layer 40 | SwitchTokens |
| Checkbox | box 18, corner 2, outline 2 when unselected, state layer 40 | CheckboxTokens |
| Radio button | icon 20, state layer 40 | RadioButtonTokens |
| Slider (new style) | track height 16 pill; handle 4 wide x 44 tall (2 wide when pressed/focused); 6 gap each side of handle; stop indicator dot 4, 6 from track end; value label Label Large, 12 above | SliderTokens |
| Dialog | corner 28, level 3, icon 24; headline Headline Small; body Body Medium; actions Label Large | DialogTokens |
| Bottom sheet | top corners 28, level 1, drag handle 32 x 4 | SheetBottomTokens |
| Snackbar | single line 48, two lines 68, corner 4, level 3; text Body Medium; action Label Large | SnackbarTokens |
| Tabs | height 48 (64 with icon and label); label Title Small; icon 24; primary indicator 3 thick with 3 top corners; secondary divider 1 | PrimaryNavigationTabTokens, SecondaryNavigationTabTokens |
| Search bar / view | bar height 56 pill, avatar 30, level 3, text Body Large; docked view header 56, corner 28; full-screen header 72 | SearchBarTokens, SearchViewTokens |
| Date picker (modal) | 360 x 568, corner 28, level 3; header 120 high, headline Headline Large, supporting Label Large; day cell 40 circle, Body Large; today outline 1; year cell 72 x 36; range header 128 | DatePickerModalTokens |
| Date input (modal) | 328 x 512, header 120 | DateInputModalTokens |
| Time picker | clock dial 256; selector handle 48; centre dot 8; track 2; time field 96 x 80 corner 8, Display Large; period selector vertical 52 x 80, horizontal 216 x 38 | TimePickerTokens |
| Loading indicator (Expressive) | container 48 x 48 circle, active shape 38 | LoadingIndicatorTokens |
| Progress indicators | linear track 4 thick, 4 gap between active and track, stop dot 4, wavy amplitude 3 / wavelength 40, wavy height 10; circular 40 (wavy 48), 4 thick | LinearProgressIndicatorTokens, CircularProgressIndicatorTokens |
| Toolbar, docked (Expressive) | height 64, side padding 16, square corners, item spacing 4 to 32 | DockedToolbarTokens |
| Toolbar, floating (Expressive) | height 64 pill, inner side padding 8, 4 between items, 16 margin from screen edge | FloatingToolbarTokens |
| Badge | small dot 6; large 16 high pill, Label Small | BadgeTokens |
| Menu | corner 4, level 2 (classic); Expressive segmented menu: container corner 16, group corner 8, group padding 4, gap 2, item height 44, icon 20, side padding 16, Body Large | MenuTokens, SegmentedMenuTokens |
| Divider | 1 thick | DividerTokens |
| Tooltips | plain: corner 4, Body Small; rich: corner 12, level 2, Title Small / Body Medium / Label Large | PlainTooltipTokens, RichTooltipTokens |

**Window size classes.** Width: compact < 600 dp ("99.96% of phones in portrait"); medium 600 to 839 (93.73% of tablets in portrait, unfolded foldables); expanded 840 to 1199 (97.22% of tablets in landscape); large 1200 to 1599 (large tablets); extra-large >= 1600 (desktop). Height: compact < 480 (99.78% of phones in landscape); medium 480 to 899; expanded >= 900. Large and extra-large were added for desktop and connected displays. — [Android Developers, Use window size classes](https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes)

**Android system bars.** Edge-to-edge is enforced from Android 15 (target SDK 35): status bar transparent by default; the gesture navigation bar is drawn transparent with a handle that adapts light/dark to content, and Google says never to give it a background; three-button navigation gets a translucent system scrim that an app may remove when a bottom app bar or navigation bar sits behind it. The page gives no dp heights and tells designers to rely on window insets. — [Android Developers, System bars](https://developer.android.com/design/ui/mobile/guides/foundations/system-bars)

### Inferences
- A "material" skill can be almost entirely table-driven from the token files; the Expressive additions a model most needs are: five button sizes with round/square shapes, 64 dp navigation bar, flexible app bars with subtitles, floating 64 dp pill toolbar, split button and button groups with 2 dp gaps, segmented lists with 2 dp gaps and 4/16 corner pairs, the thick 16 dp slider, and emphasized type.
- Because seed-colour derivation is an algorithm (HCT tone mapping), a static skill should ship (a) the tone-number table above and (b) a few precomputed palettes, rather than expect a model to compute HCT tones.
- The fixed text-field height of 56 dp applies to both filled and outlined variants in practice (only the outlined file carries the token).

### Gaps
- Seed-colour derivation rules (which hue/chroma each palette gets from the seed; scheme variants such as Tonal Spot, Vibrant, Expressive) were not read from a primary source this session. *Unverified, from memory:* Tonal Spot uses primary chroma 36 (48 before 2023 revisions), secondary chroma 16, tertiary hue +60 degrees at chroma 24, neutral chroma about 4 to 6, neutral variant about 8; source to check: github.com/material-foundation/material-color-utilities.
- m3.material.io spec pages (paddings, measurement diagrams, canonical layout pane widths) could not be rendered. *Unverified, from memory:* canonical layouts are list-detail, supporting pane and feed; margins 16 dp compact and 24 dp medium and up; pane spacer 24 dp.
- Status and navigation bar heights are not published. *Unverified, from memory:* status bar about 24 dp on devices without a tall cutout (commonly 28 to 52 dp with punch-hole cameras), gesture navigation inset about 24 dp (handle roughly 108 x 4 dp) and three-button bar 48 dp.
- Common Android device sizes in dp were not verified this session. *Unverified, from memory:* 360 x 800 (most common budget/mid class), 412 x 915 (Pixel 8/9 class), 384 x 832 / 411 x 914 variants, Galaxy S 360 x 780, foldable inner about 841 x 701 to 882 x 968, tablets 800 x 1280 and 1280 x 800.
- Whether Google Sans Flex or Roboto Flex is the reference face in the official 2026 Figma kit was not confirmed.

## 2. Apple beyond the glass material (iOS / iPadOS type, layout, components, colours, widgets, Live Activities, icons; macOS window anatomy; watchOS and visionOS)

### Takeaway
Apple's HIG gives exact tables for type, widgets, Live Activities, app icon canvases and a few visionOS/tvOS sizes, but as of late 2026 it publishes no bar heights, list row heights, system colour hex values, or macOS window measurements in readable form, and the device screen-size table is no longer on the Layout page. Those must come from Apple's Figma/Sketch kits or measurement.

### Cited Findings

**iOS / iPadOS Dynamic Type at the default "Large" setting (SF Pro; points).** — [HIG Typography](https://developer.apple.com/design/human-interface-guidelines/typography)

| Style | Weight | Size | Leading | Emphasized weight |
|---|---|---|---|---|
| Large Title | Regular | 34 | 41 | Bold |
| Title 1 | Regular | 28 | 34 | Bold |
| Title 2 | Regular | 22 | 28 | Bold |
| Title 3 | Regular | 20 | 25 | Semibold |
| Headline | Semibold | 17 | 22 | Semibold |
| Body | Regular | 17 | 22 | Semibold |
| Callout | Regular | 16 | 21 | Semibold |
| Subhead | Regular | 15 | 20 | Semibold |
| Footnote | Regular | 13 | 18 | Semibold |
| Caption 1 | Regular | 12 | 16 | Semibold |
| Caption 2 | Regular | 11 | 13 | Semibold |

- Default / minimum text sizes: iOS and iPadOS 17 / 11 pt; macOS 13 / 10; tvOS 29 / 23; visionOS 17 / 12; watchOS 16 / 12. — [HIG Typography](https://developer.apple.com/design/human-interface-guidelines/typography)
- SF Pro tracking by size (1/1000 em): 11 pt +6; 12 pt 0; 13 pt -6; 15 pt -16; 16 pt -20; 17 pt -26; 20 pt -23; 22 pt -12; 28 pt +14; 34 pt +12. — [HIG Typography](https://developer.apple.com/design/human-interface-guidelines/typography)
- macOS built-in styles (weight, size/line height, emphasized): Large Title Regular 26/32 (Bold); Title 1 Regular 22/26 (Bold); Title 2 Regular 17/22 (Bold); Title 3 Regular 15/20 (Semibold); Headline Bold 13/16 (Heavy); Body Regular 13/16 (Semibold); Callout Regular 12/15 (Semibold); Subheadline Regular 11/14 (Semibold); Footnote Regular 10/13 (Semibold); Caption 1 Regular 10/13 (Medium); Caption 2 Medium 10/13 (Semibold). — [HIG Typography](https://developer.apple.com/design/human-interface-guidelines/typography)
- tvOS styles (weight, size/leading): Title 1 Medium 76/96; Title 2 Medium 57/66; Title 3 Medium 48/56; Headline Medium 38/46; Subtitle 1 Regular 38/46; Callout Medium 31/38; Body Medium 29/36; Caption 1 Medium 25/32; Caption 2 Medium 23/30. — [HIG Typography](https://developer.apple.com/design/human-interface-guidelines/typography)
- watchOS (SF Compact) at "Large", the default for 40/41/42 mm: Large Title 36/38.5; Title 1 34/36.5; Title 2 28/30.5; Title 3 19/21.5; Headline (Semibold) 16/18.5; Body 16/18.5; Caption 1 15/17.5; Caption 2 14/16.5; Footnote 1 13/15.5; Footnote 2 12/14.5. At "xLarge", the default for 44/45/49 mm: 40/42.5, 38/40.5, 30/32.5, 20/22.5, 17/19.5, 17/19.5, 16/18.5, 15/17.5, 14/16.5. — [HIG Typography](https://developer.apple.com/design/human-interface-guidelines/typography)

**iPhone screens and safe areas (points).** iPhone 17 and 17 Pro: 402 x 874 @3x (1206 x 2622 px); iPhone Air: 420 x 912 @3x; iPhone 17 Pro Max: 440 x 956 @3x. Status bar 54. Portrait safe area: top 62 / bottom 34 (iPhone Air top 68). Landscape: top 20, bottom 20, left and right 62 (Air: bottom 29, sides 68). Older sizes: 430 x 932 (16 Plus, 15 Pro Max), 393 x 852 (16, 15, 15 Pro, 14 Pro), 390 x 844 (16e, 14, 13, 12), 375 x 812, 375 x 667 @2x. — [Use Your Loaf, iPhone 17 Screen Sizes](https://useyourloaf.com/blog/iphone-17-screen-sizes/) (third-party measurement, not Apple)

**Widgets (points).** — [HIG Widgets](https://developer.apple.com/design/human-interface-guidelines/widgets)

| iPhone screen (portrait) | Small | Medium | Large | Lock Screen circular | rectangular | inline |
|---|---|---|---|---|---|---|
| 430 x 932 | 170 x 170 | 364 x 170 | 364 x 382 | 76 x 76 | 172 x 76 | 257 x 26 |
| 393 x 852 and 390 x 844 | 158 x 158 | 338 x 158 | 338 x 354 | 72 x 72 | 160 x 72 | 234 x 26 |
| 375 x 812 and 360 x 780 | 155 x 155 | 329 x 155 | 329 x 345 | 72 x 72 | 157 x 72 | 225 x 26 |
| 375 x 667 | 148 x 148 | 321 x 148 | 321 x 324 | 68 x 68 | 153 x 68 | 225 x 26 |

- The table has no rows for 402 x 874, 420 x 912 or 440 x 956 screens. — [HIG Widgets](https://developer.apple.com/design/human-interface-guidelines/widgets)
- iPad widgets, on-device size (small / medium / large / extra large): 768 x 1024 and 744 x 1133: 120 x 120, 260 x 120, 260 x 260, 540 x 260; 820 x 1180 and 834 x 1194: 136 x 136, 300 x 136, 300 x 300, 628 x 300; 1024 x 1366: 160 x 160, 356 x 160, 356 x 356, 748 x 356. — [HIG Widgets](https://developer.apple.com/design/human-interface-guidelines/widgets)
- visionOS widgets: small 158 x 158, medium 338 x 158, large 338 x 354, extra large 450 x 338, extra large portrait 338 x 450. watchOS Smart Stack widget: 152 x 69.5 (40 mm), 165 x 72.5 (41 mm), 176 x 72.5 (42 mm), 173 x 76.5 (44 mm), 184 x 80.5 (45 mm), 194 x 80.5 (46 mm), 198 x 81.5 (49 mm Ultra 3, 4). — [HIG Widgets](https://developer.apple.com/design/human-interface-guidelines/widgets)
- Widget families by surface: small widgets also appear in StandBy, CarPlay and "Apple HomeView"; accessory circular / rectangular / inline are the Lock Screen and watch complication families; rendering modes are full-colour, accented and vibrant. The page was updated on 13 October 2026 to add Apple HomeView. — [HIG Widgets](https://developer.apple.com/design/human-interface-guidelines/widgets)

**Live Activities and Dynamic Island (points).** — [HIG Live Activities](https://developer.apple.com/design/human-interface-guidelines/live-activities)

| Screen | Compact (leading, trailing) | Minimal | Expanded | Lock Screen |
|---|---|---|---|---|
| 430 x 932 | 62.33 x 36.67 | 36.67 to 45 wide x 36.67 | 408 x 84 to 160 | 408 x 84 to 160 |
| 393 x 852 | 52.33 x 36.67 | 36.67 to 45 wide x 36.67 | 371 x 84 to 160 | 371 x 84 to 160 |

- "Limited width" presentation is 28 x 28; standard Lock Screen Live Activity margin is 14 pt; iPad Lock Screen Live Activity is 500 x 84 to 160 on 1366 x 1024 and 425 wide on smaller iPads; Live Activities also appear in the Mac menu bar, the watch Smart Stack and the CarPlay Dashboard (changes dated 16 December 2025 and 13 October 2026). — [HIG Live Activities](https://developer.apple.com/design/human-interface-guidelines/live-activities)

**App icons (2025-2026 layered format).** Canvas: iOS, iPadOS and macOS 1024 x 1024 px square, masked to a rounded rectangle, layered, with six appearances (default, dark, clear light, clear dark, tinted light, tinted dark); watchOS 1088 x 1088 px masked to a circle; visionOS 1024 x 1024 px circular, 3D layers; tvOS 800 x 480 px, two to five parallax layers. Icons for iOS, iPadOS, macOS and watchOS are assembled from a background layer plus foreground layers in Icon Composer; the system adds highlights, shadows and glass, so artwork should be flat with hard edges. A single bitmap is still accepted and the system segments it into two layers (format-options guidance added 28 October 2026). — [HIG App icons](https://developer.apple.com/design/human-interface-guidelines/app-icons)

**Colours.** The HIG lists system colours by name only in readable form (red, orange, yellow, green, mint, teal, cyan, blue, indigo, purple, pink, brown; grays 1 to 6) with values shown as swatch images; it says visionOS uses the dark values, and it defines semantic label, secondary/tertiary/quaternary label, placeholder, separator, opaque separator and link colours, plus system and grouped backgrounds in primary/secondary/tertiary levels. System colour values were updated on 9 June 2025 with Liquid Glass. — [HIG Color](https://developer.apple.com/design/human-interface-guidelines/color)

**Other measurements the HIG does state.**
- tvOS: inset primary content 60 pt top and bottom and 80 pt at the sides; tab bar is 68 pt high and sits 46 pt from the top of the screen. — [HIG Layout](https://developer.apple.com/design/human-interface-guidelines/layout), [HIG Tab bars](https://developer.apple.com/design/human-interface-guidelines/tab-bars)
- visionOS: default window 1280 x 720 pt, placed about two metres away; button sizes mini 28, small 32, regular 44, large 52, extra large 64 pt; keep button centres at least 60 pt apart; alert accessory views at most 154 pt high with 16 pt corners. Ornaments float slightly in front of a window edge, should be no wider than the window, and toolbars and tab bars become ornaments automatically. — [HIG Windows](https://developer.apple.com/design/human-interface-guidelines/windows), [HIG Buttons](https://developer.apple.com/design/human-interface-guidelines/buttons), [HIG Alerts](https://developer.apple.com/design/human-interface-guidelines/alerts), [HIG Ornaments](https://developer.apple.com/design/human-interface-guidelines/ornaments)
- Sheets: iPhone sheets rest at system "medium" and "large" detents and should show a grabber when resizable; no point heights are given. — [HIG Sheets](https://developer.apple.com/design/human-interface-guidelines/sheets)
- SF Symbols come in nine weights and three scales (small, medium, large) defined relative to the cap height of the system font. — [HIG SF Symbols](https://developer.apple.com/design/human-interface-guidelines/sf-symbols)

**What changed in 2025-2026.**
- The Layout page's change log records device specifications being added through 9 September 2025 (iPhone 17 family, Apple Watch Series 11, Ultra 3), then "Updated guidance to reflect current best practices" on 9 September 2026; the page data fetched on 20 November 2026 contains no device dimension table. — [HIG Layout](https://developer.apple.com/design/human-interface-guidelines/layout)
- iOS 27, iPadOS 27 and macOS 27 "Golden Gate" were previewed on 8 June 2026 with refinements to Liquid Glass (a user transparency slider, more consistent window corner radius and sidebars on macOS, a uniform toolbar, extra glass layers on icons); apps built for Liquid Glass pick the changes up without recompiling. — [MacRumors](https://macrumors.com/2026/06/10/how-liquid-glass-is-changing-in-ios-27), [Cult of Mac](https://www.cultofmac.com/news/liquid-glass-changes-ios-27-macos-27)
- macOS 26 window corner radius is not uniform: community measurements put toolbar windows at about 26 pt and title-bar-only windows at about 16 pt, against about 10 pt before macOS 26; macOS 27 is reported at about 20 pt. Firefox uses 16 px for its chrome block radius on Tahoe versus 10 px before. — [MacRumors forum](https://forums.macrumors.com/threads/want-the-bigger-window-corner-radius-back-come-inside.2484898/), [Mozilla bug 2053333](https://bugzilla.mozilla.org/show_bug.cgi?id=2053333) (community figures, not Apple)

### Inferences
- An "apple-apps" skill cannot be sourced from the HIG alone. The drawable numbers for bars, lists, sheets, alerts and menus have to be taken from Apple's official design kits (Figma/Sketch) or from simulator measurement; the HIG deliberately stopped publishing them.
- Widget and Live Activity sizes for the current 402 / 420 / 440 pt-wide phones are missing from Apple's tables; scaling from the nearest row (393 for 402; 430 for 440) is the practical fallback and should be labelled approximate.
- Because system colour hex values are not published as text and changed in June 2025, any hex table in a skill should be labelled "measured, iOS 26" and kept in one place for easy correction.
- For icon stand-ins when only Lucide is available: SF Symbols at regular weight are monoline with rounded joins, so Lucide (24 grid, 2 px stroke, round caps) is the closest of the common free sets in character; scale the stroke to about 1.5 to 1.75 px at 24 px to approximate SF Symbols "regular" next to 17 pt text, and use filled shapes for selected tab items, which Lucide lacks.

### Gaps
- iPhone 18 family (September 2026) screen sizes and safe areas: only pre-launch rumours found (same 6.3 and 6.9 inch sizes, possibly a narrower Dynamic Island). Not confirmed.
- Whether iOS 27 / macOS 27 shipped publicly and whether they changed any metric was not confirmed.
- iPad screen sizes and safe areas were not verified. *Unverified, from memory:* 11-inch 834 x 1194 / 834 x 1210, 13-inch 1024 x 1366 / 1032 x 1376, iPad mini 744 x 1133, iPad 820 x 1180; top safe area 24, bottom 20 on Face ID iPads.
- Component metrics, *unverified, from memory (pre-26 UIKit conventions unless noted)*: navigation bar 44 pt (large title area adds 52); tab bar 49 pt plus 34 home-indicator inset (iOS 26 floating glass tab bar is taller and inset, already covered by the existing liquid-glass skill); toolbar 44; search field 36; list row minimum 44, inset-grouped list side margin 16 to 20 with 10 to 26 corner radius depending on OS version (iOS 26 uses larger radii), separator 0.33 pt inset 16 to 20; standard layout margin 16 (20 on Plus/Max); minimum hit target 44 x 44; alert width 270 with 14 radius (iOS 26 alerts are wider, left-aligned, about 300 with larger radius); action sheet row 57; context menu row 44, width about 250; medium detent about half screen height, large detent stops below the status bar; Dynamic Island about 126 x 37 at 11 from the top; home indicator about 134 to 139 x 5.
- System colour hex values, *unverified, from memory of pre-June-2025 values*: blue #007AFF / #0A84FF, red #FF3B30 / #FF453A, green #34C759 / #30D158, orange #FF9500 / #FF9F0A, yellow #FFCC00 / #FFD60A, gray #8E8E93, systemGray6 #F2F2F7 / #1C1C1E, label #000000 / #FFFFFF, secondary label 60% of #3C3C43 / 60% of #EBEBF5, grouped background #F2F2F7 / #000000. Apple changed several of these in June 2025; the new values need to be read from the iOS 26 kit.
- macOS window anatomy, *unverified, from memory*: menu bar 24 pt (taller on notched displays), title bar 28, unified toolbar 52 (compact 38), traffic lights 12 pt diameter with 8 gap, first light about 20 from the left (larger lights under macOS 26), sidebar typically 200 to 280 wide, control heights mini 16 / small 19 / regular 22 / large 28 pre-26 with macOS 26 adding taller controls and an extra-large size, settings windows as sidebar plus grouped form about 715 wide.
- watchOS screen sizes, *unverified, from memory*: 41 mm 176 x 215, 45 mm 198 x 242, 42 mm (Series 10/11) 187 x 223, 46 mm 208 x 248, 49 mm Ultra 205 x 251.
- App Store screenshot sizes were not checked. *Unverified, from memory:* 6.9-inch 1320 x 2868 (or 1290 x 2796), 6.5-inch 1284 x 2778 / 1242 x 2688, iPad 13-inch 2064 x 2752 (or 2048 x 2732).
- The Live Activities page also lists sizes 240 x 78, 240 x 100, 170 x 78 pt and display configurations 1920 x 720, 900 x 1200, 800 x 480; I did not capture which surface each belongs to (likely CarPlay and/or Apple HomeView).

## 3. Microsoft Fluent 2 / Windows 11

### Takeaway
Windows 11 has a compact, well-documented spec: a 9-step Segoe UI Variable type ramp, two corner radii (4 and 8 px), a seven-step elevation scale, a 32 px title bar (48 px when it holds search or a profile picture), three window-width breakpoints, and everything on a 4 px grid. Fluent 2 web tokens add exact shadow recipes, a neutral grey ramp and a 16-step brand ramp.

### Cited Findings
- Windows 11 type ramp (Segoe UI Variable; size/line height in effective pixels): Caption 12/16 Regular ("Small" optical size); Body 14/20 Regular; Body Strong 14/20 Semibold; Body Large 18/24 Regular; Body Large Strong 18/24 Semibold; Subtitle 20/28 Semibold (Display optical size); Title 28/36 Semibold; Title Large 40/52 Semibold; Display 68/92 Semibold. Minimum sizes 12 px Regular and 14 px Semibold; sentence case; weights used are Regular and Semibold. Weight axis: Light 300, Semilight 350, Regular 400, Semibold 600, Bold 700. — [Microsoft Learn, Typography in Windows](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography)
- Corner radius: 8 px for top-level containers (app windows, flyouts, dialogs, menus, teaching tips); 4 px for in-page controls (buttons, checkboxes, combo boxes, text boxes, list backplates), bars (progress, scroll, slider) and tooltips; 0 px where straight edges meet and for snapped or maximized windows. Resource names: ControlCornerRadius 4, OverlayCornerRadius 8. — [Microsoft Learn, Geometry in Windows 11](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/geometry)
- Elevation values (each with a 1 px stroke): window 128, dialog 128, flyout 32, tooltip 16, card 8, control 2, layer 1; control states: rest 2, hover 2, pressed 1. — [Microsoft Learn, Layering and elevation](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/layering)
- Title bar: standard height 32 px; 48 px when it contains a search box or person picture; window icon 16 x 16, 16 px from the left edge and vertically centred (8 px above and below at 32 px height); title text 16 px after the icon; back button 16 x 16 glyph, 16 px from the edge. — [Microsoft Learn, Title bar design](https://learn.microsoft.com/en-us/windows/apps/design/basics/titlebar-design)
- Breakpoints: small up to 640 px, medium 641 to 1007, large 1008 and up; sizes, margins and positions should be multiples of 4 epx. — [Microsoft Learn, Screen sizes and breakpoints](https://learn.microsoft.com/en-us/windows/apps/design/layout/screen-sizes-and-breakpoints-for-responsive-design)
- NavigationView: expanded left pane at 1008 px and wider, icon-only compact pane from 641 to 1007, menu button only at 640 and below; header area fixed 52 px high; content margins 24 px (12 px in minimal mode). — [Microsoft Learn, NavigationView](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/navigationview)
- Command bar: primary command icons 20 x 20 px, overflow-menu icons 16 x 16; at 320 epx width at most four primary commands fit. — [Microsoft Learn, Command bar](https://learn.microsoft.com/en-us/windows/apps/develop/ui/controls/command-bar)
- Mica is an opaque material tinted by the desktop wallpaper, sampled once, used as the base layer of long-lived windows and shown through the title bar; Mica Alt is a stronger-tinted variant for tabbed title bars; both fall back to a solid colour in high contrast, when the window is inactive, or in battery saver. — [Microsoft Learn, Mica](https://learn.microsoft.com/en-us/windows/apps/design/style/mica)
- Acrylic recipe, in layer order: background, Gaussian blur, exclusion blend, colour/tint overlay, noise texture; "background acrylic" shows the desktop and other windows and is for transient surfaces, "in-app acrylic" blurs app content. — [Microsoft Learn, Acrylic](https://learn.microsoft.com/en-us/windows/apps/design/style/acrylic)
- Segoe Fluent Icons: recommended sizes 16, 20, 24, 32, 40, 48, 64; all glyphs share one size and position; not meant for inline text. — [Microsoft Learn, Segoe Fluent Icons font](https://learn.microsoft.com/en-us/windows/apps/design/iconography/segoe-fluent-icons-font)
- Fluent 2 web tokens: font sizes 10, 12, 14, 16, 20, 24, 28, 32, 40, 68 px with line heights 14, 16, 20, 22, 28, 32, 36, 40, 52, 92; weights 400, 500, 600, 700; font stack 'Segoe UI', 'Segoe UI Web (West European)', -apple-system, BlinkMacSystemFont, Roboto, 'Helvetica Neue', sans-serif. — [microsoft/fluentui tokens, fonts.ts](https://github.com/microsoft/fluentui/blob/master/packages/tokens/src/global/fonts.ts)
- Fluent 2 named text styles appear in this order in the token source, matching the sizes above: body1 14/20 Regular (Strong = Semibold, Stronger = Bold), body2 16/22, caption1 12/16, caption2 10/14, subtitle1 20/28 Semibold, subtitle2 16/22 Semibold, title1 32/40 Semibold, title2 28/36 Semibold, title3 24/32 Semibold, largeTitle 40/52 Semibold, display 68/92 Semibold. — [microsoft/fluentui tokens, typographyStyles.ts](https://github.com/microsoft/fluentui/blob/master/packages/tokens/src/global/typographyStyles.ts)
- Fluent 2 corner radius tokens: none 0, small 2, medium 4, large 6, XLarge 8, 2XLarge 12, 3XLarge 16, 4XLarge 24, 5XLarge 32, 6XLarge 40, circular. — [microsoft/fluentui tokens, borderRadius.ts](https://github.com/microsoft/fluentui/blob/master/packages/tokens/src/global/borderRadius.ts)
- Fluent 2 shadow recipes (two layers, ambient then key): shadow2 = 0 0 2px + 0 1px 2px; shadow4 = 0 0 2px + 0 2px 4px; shadow8 = 0 0 2px + 0 4px 8px; shadow16 = 0 0 2px + 0 8px 16px; shadow28 = 0 0 8px + 0 14px 28px; shadow64 = 0 0 8px + 0 32px 64px. — [microsoft/fluentui tokens, shadows.ts](https://github.com/microsoft/fluentui/blob/master/packages/tokens/src/utils/shadows.ts)
- Fluent 2 neutral grey ramp (index = lightness): grey 2 #050505, 4 #0A0A0A, 6 #0F0F0F, 8 #141414, 10 #1A1A1A, 12 #1F1F1F, 14 #242424, 16 #292929, 18 #2E2E2E, 20 #333333, 22 #383838, 24 #3D3D3D, 26 #424242, 30 #4D4D4D, 38 #616161, 44 #707070, 50 #808080, 60 #999999, 70 #B3B3B3, 74 #BDBDBD, 80 #CCCCCC, 82 #D1D1D1, 88 #E0E0E0, 90 #E6E6E6, 92 #EBEBEB, 94 #F0F0F0, 96 #F5F5F5, 98 #FAFAFA. — [microsoft/fluentui tokens, colors.ts](https://github.com/microsoft/fluentui/blob/master/packages/tokens/src/global/colors.ts)
- Fluent 2 default brand ramp (web, blue): 10 #061724, 20 #082338, 30 #0A2E4A, 40 #0C3B5E, 50 #0E4775, 60 #0F548C, 70 #115EA3, 80 #0F6CBD, 90 #2886DE, 100 #479EF5, 110 #62ABF5, 120 #77B7F7, 130 #96C6FA, 140 #B4D6FA, 150 #CFE4FA, 160 #EBF3FC. — [microsoft/fluentui tokens, brandColors.ts](https://github.com/microsoft/fluentui/blob/master/packages/tokens/src/global/brandColors.ts)

### Inferences
- Two dialects need separating in a skill: "Windows 11 native" (WinUI: 4/8 radii, 14 px body, Mica, 32 px title bar) and "Fluent 2 web" (Microsoft 365 look: brand ramp, shadow tokens, 14/20 body1, 4 px medium radius). They share sizes but not colours or materials.
- Mica can be drawn statically as a flat, slightly wallpaper-tinted neutral (close to grey 94 to 96 light, grey 12 to 14 dark); Acrylic as a blurred backdrop with a tint overlay and faint noise.
- Lucide is a weak match for Segoe Fluent Icons (1 px monoline, squarer geometry); thinning Lucide's stroke to about 1 to 1.25 px at 16 to 20 px is the nearest approximation.

### Gaps
- Caption button size is not on the fetched pages. *Unverified, from memory:* 46 x 32 px each (minimize, maximize, close), close hover fill #C42B1C, glyphs 10 px.
- Control heights are not stated on the fetched pages. *Unverified, from memory:* standard WinUI control height 32 px (compact 24), NavigationView compact pane 48 px and open pane 320 px, item height 36 to 40, CommandBar 48 px (40 compact), checkbox 20, toggle switch 40 x 20, list item 40, text padding 11 horizontal, card padding 16, standard gutters 4 / 8 / 12 / 16 / 24.
- Shadow colours per theme were not read. *Unverified, from memory:* ambient black 12% and key black 14% in light theme; 24% and 28% in dark.
- Windows accent default (#0078D4 family, light theme accent text #005FB8, dark #60CDFF) and WinUI fill/stroke colour tokens (for example card fill white 70%, control stroke black 5.78%) were not verified.
- fluent2.microsoft.design component pages were not fetched.

## 4. Other targets: Android widgets, Wear OS, TV, car, browser chrome, One UI

### Takeaway
TV has firm numbers on both sides (Android TV 960 x 540 dp canvas with 5% overscan and a 12-column grid; tvOS 60/80 pt safe area with its own type table). CarPlay and Live Activity car sizes are published by Apple. Android widgets, Wear OS, Android Auto, browser chrome and One UI could not be verified this session.

### Cited Findings
- Android TV: design at 960 x 540 (MDPI, 1 px = 1 dp), 16:9, assets at 1080p; overscan margin 5% = 48 dp left/right and 27 dp top/bottom; grid of 12 columns 52 dp wide with 20 dp gutters and 58 dp side margins; card widths 844 dp (1 across), 412 (2), 268 (3), 196 (4), 124 (5); the page notes most modern TVs no longer overscan. The same page gives both 48/27 and 58/28 dp as the margin, an internal inconsistency. — [Android Developers, TV layouts](https://developer.android.com/design/ui/tv/guides/styles/layouts)
- tvOS: safe area 60 pt top/bottom and 80 pt sides; focused items enlarge, so leave room; tab bar 68 pt high, 46 pt from the top; body text 29 pt, minimum 23 pt. — [HIG Layout](https://developer.apple.com/design/human-interface-guidelines/layout), [HIG Tab bars](https://developer.apple.com/design/human-interface-guidelines/tab-bars), [HIG Typography](https://developer.apple.com/design/human-interface-guidelines/typography)
- CarPlay: common screens 800 x 480 (5:3), 960 x 540 (16:9), 1280 x 720 (16:9), 1920 x 720 (8:3); landscape and portrait, @2x and @3x; app icon 120 x 120 px @2x and 180 x 180 px @3x. — [HIG CarPlay](https://developer.apple.com/design/human-interface-guidelines/carplay)
- Windows also treats TVs as the "small" breakpoint class (320 x 569, 360 x 640, 480 x 854 effective pixels on 20 to 65 inch screens). — [Microsoft Learn, Screen sizes and breakpoints](https://learn.microsoft.com/en-us/windows/apps/design/layout/screen-sizes-and-breakpoints-for-responsive-design)

### Inferences
- A single "tv" section can cover both platforms with one 1920 x 1080 frame: tvOS works in points at 1x on that frame; Android TV numbers are doubled from the 960 x 540 dp canvas.
- Car templates are constrained by the system (fixed templates, few items per screen); mock-ups mainly need the frame sizes above plus large type and big targets.

### Gaps
- Wear OS: the canvas page URL I tried returned 404. *Unverified, from memory:* round screens 192 dp (small) to 225 dp and larger; side margins as percentages of the screen (about 5.2% outer, more for text); Material 3 Expressive for Wear OS (Wear OS 6) with edge-hugging buttons.
- Android widgets: not fetched. *Unverified, from memory:* grid-cell based (typically 4 or 5 columns), common sizes 2x1, 2x2, 4x1, 4x2; system corner radius 16 to 28 dp (system_app_widget_background_radius, max 28 dp), inner radius 8 dp less, padding 16 dp.
- Android Auto / Android for Cars template sizes: not fetched.
- Browser chrome for web mock-ups: no official specification exists. *Unverified, from memory:* Chrome desktop (2023 refresh) tab strip about 41 px plus toolbar about 47 px, about 88 px total, bookmarks bar +33; Safari on macOS compact unified bar about 52 px (separate tab bar about +28 to 36); mobile Safari bottom address bar about 44 to 50 pt above the home indicator.
- Samsung One UI (8 / 8.5): no official metric source found. *Unverified, from memory:* large top "viewing area" header that collapses, content cards with about 26 dp corners on a grey background, bottom-anchored actions, Samsung One / SamsungSans UI font.

## 5. Evidence of demand

### Takeaway
By traffic share Android and Windows are the two largest platforms, then iOS, then macOS; by design-tool behaviour Google's Material 3 kit is reported as the most-used platform kit in Figma Community, with Apple's kits next. Exact Figma duplicate counts could not be read (Figma blocks automated fetches).

### Cited Findings
- All platforms, worldwide, October 2026: Android 44.56%, Windows 29.08%, iOS 17.45%. — [StatCounter, OS market share (all)](https://gs.statcounter.com/os-market-share/all/worldwide/2012) (as reported in search results for the October 2026 edition)
- Mobile, worldwide, October 2026: Android 72.36%, iOS 27.61%. — [StatCounter, mobile OS worldwide](https://gs.statcounter.com/os-market-share/mobile/worldwide); a search summary of earlier 2026 editions of the same pages showed Android 67.5 to 69.1% and iOS 30.8 to 32.5% for June to August 2026 — [StatCounter](https://gs.statcounter.com/os-market-share/mobile-tablet/worldwide/). The two sets do not agree well; the figures may come from different page variants (mobile versus mobile plus tablet) or have been revised.
- Desktop, worldwide, October 2026: Windows 76.65%, "OS X" 9.86% plus "macOS" 5.81% (StatCounter reports the two labels separately), Linux 5.63%, Chrome OS 2.04%. — [StatCounter, desktop OS worldwide](https://gs.statcounter.com/os-market-share/desktop/worldwide)
- Google says designers use its Material Design kit "more than the second-most popular kit by a factor of ten". — [Google Design, When the Design Kit Went Dynamic](https://design.google/library/euphrates-dahout-material-design-figma) (vendor's own claim)
- Third-party roundups: one 2025 list gives the Material 3 Design Kit 1.5 million users; a 2026 list gives an iOS kit 228,000+ users; shadcn/ui kit 329,000+; Untitled UI 280,000 to 320,000+; Ant Design 159,000+. — [Untitled UI roundup](https://www.untitledui.com/blog/figma-ui-kits), [Mockuuups Studio roundup](https://mockuuups.studio/blog/post/best-figma-design-systems/) (affiliate-style blogs, no method stated; which figure belongs to which page was not confirmed)
- Android window-size statistics that bear on template choice: 99.96% of phones are compact width in portrait; 97.22% of tablets are expanded width in landscape. — [Android Developers](https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes)

### Inferences
- Suggested priority by usage and kit demand: (1) Material 3 Expressive / Android phone, (2) Apple non-glass app patterns for iPhone (the existing liquid-glass skill covers only materials and bars), (3) Windows 11 / Fluent, (4) macOS window anatomy and generic desktop app windows plus browser frames, (5) widgets, Lock Screen and Live Activities, (6) tablet/foldable adaptive layouts, (7) watch, TV, car, spatial and One UI as short reference sections.
- Mobile share understates iOS importance for designers: iOS over-indexes in North America, Japan and in paid-app revenue, and Apple kits are among the most duplicated; Material and Apple skills should be treated as equal first priorities.
- Windows' 77% desktop share and the absence of any Windows skill today make "windows-fluent" the largest uncovered desktop gap.

### Gaps
- Figma Community pages returned HTTP 403; no duplicate/user counts for the Material 3 Design Kit, Apple's "iOS and iPadOS 26" kit, or Microsoft's Windows UI kit were verified at source.
- No regional or revenue split was gathered; no Windows-version split (Windows 11 versus 10) was gathered.

## 6. Licensing limits on system fonts and icons

### Takeaway
Roboto, Google Sans Flex and Material Symbols are open and can be bundled; Apple's San Francisco fonts and Microsoft's Segoe fonts cannot be bundled or redistributed by a third-party tool, though Apple permits registered developers to use SF for mock-ups of Apple-platform apps and Microsoft permits images made with Windows fonts. A design tool should ship open substitutes and name the system font only as a font-stack request.

### Cited Findings
- Apple San Francisco licence: the font is "to be used solely for creating mock-ups of user interfaces to be used in software products running on Apple's iOS, OS X or tvOS operating systems"; it may be shown in screenshots and mock-ups of such products; "You may not embed the Apple Font in any software programs or other products"; no redistribution, no derivative works, no use for mock-ups targeting non-Apple operating systems; limited to registered Apple developers. SF Compact has the same terms for watchOS. Fonts offered: SF Pro, SF Compact, SF Mono, New York, and SF Arabic, Armenian, Georgian and Hebrew. — [Apple Developer, Fonts](https://developer.apple.com/fonts/)
- Microsoft: "you may not redistribute the Windows fonts", copy them to other machines or servers, convert or modify them; embedding in apps, games or devices is not allowed, including as bitmap fonts; images of text made with the fonts (screenshots, graphics) are allowed and treated like print output; web pages may name the fonts in a CSS font stack; "Segoe UI can be licensed from Monotype, Segoe UI Variable is not available for licensing or use outside of Microsoft products or on non-Windows platforms". — [Microsoft Learn, Font redistribution FAQ](https://learn.microsoft.com/en-us/typography/fonts/font-faq)
- Microsoft lists Selawik as "an open-source font that's metrically compatible with Segoe UI, intended for apps on other platforms that don't want to bundle Segoe UI". — [Microsoft Learn, Typography in Windows](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography)
- Google Sans Flex was released under the SIL Open Font License 1.1 on 18 November 2025 (Google Sans followed on 10 December 2025); it is free for personal and commercial use, with variable axes for weight, width, optical size/grade, slant and roundness, and is on Google Fonts and GitHub. — [Wikipedia, Product Sans](https://en.wikipedia.org/wiki/Product_Sans), [Android Authority](https://www.androidauthority.com/google-sans-flex-free-3617034/), [OMG! Ubuntu](https://www.omgubuntu.co.uk/2025/11/google-sans-flex-font-ubuntu)
- Fluent's own web font stack falls back from Segoe UI to -apple-system, BlinkMacSystemFont and Roboto, which shows Microsoft's accepted substitution order off Windows. — [microsoft/fluentui tokens, fonts.ts](https://github.com/microsoft/fluentui/blob/master/packages/tokens/src/global/fonts.ts)

### Inferences
- Bundle-safe set for Pencraft: Roboto / Roboto Flex and Google Sans Flex (Material), Inter as the SF Pro stand-in, Selawik (or Segoe UI by name with fallback) for Windows, and Lucide for icons. Request "SF Pro" or "Segoe UI Variable" by family name so the mock-up uses the real font only when the user's machine already has it.
- Drawing look-alike shapes (traffic lights, caption buttons, generic status-bar glyphs) is a design-convention matter, not a font-licence one; copying Apple's or Microsoft's actual glyph outlines into bundled assets is what the licences forbid.

### Gaps
- SF Symbols licence text was not fetched. *Unverified, from memory:* symbols may be used only in apps and mock-ups for Apple platforms, may not be used in app icons, logos or trademarks, and some symbols are restricted to referring to specific Apple technologies; they may not be redistributed.
- Roboto and Material Symbols licences were not fetched. *Unverified, from memory:* Roboto is Apache 2.0 (Roboto Flex OFL 1.1) and Material Symbols / Material Icons are Apache 2.0, so both may be bundled and modified.
- Segoe Fluent Icons licence was not fetched. *Unverified, from memory:* it ships with Windows 11 and falls under the same no-redistribution rule; Microsoft's separate "Fluent UI System Icons" set is MIT-licensed and is the lawful bundled stand-in.
- New York font licence terms were not on the fetched page.
