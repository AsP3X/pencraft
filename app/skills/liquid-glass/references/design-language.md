# Why glass is used the way it is

Background for cases the main skill file does not list. The rules follow Apple's published guidance for its
2025 design language (Human Interface Guidelines: Materials, Color; "Adopting Liquid Glass"; WWDC25 sessions
219, 356, 323). Where Apple publishes behaviour but no numbers, Pencraft's numbers are its own.

## Two layers

- The **content layer** is the app's content: lists, text, media, backgrounds.
- The **functional layer** floats above it: tab bars, toolbars, sidebars, sheets, popovers, menus, floating
  buttons. Liquid glass belongs here only.
- A control inside content with a moving part (a slider's knob, a toggle's knob) is plain at rest and becomes
  glass only while it is being moved. Show that as a second frame.
- Structure inside content uses the standard blur materials (frosted), in four thicknesses. Thicker gives
  fine text more contrast; thinner keeps more of the context. Choose by purpose, not by how it looks today.

## What makes it read as glass

- **Lensing.** Content is bent at the edges of the shape. That, not a border, shows where the element ends
  and that it sits above the content. A glass element with nothing behind it has nothing to bend.
- **Light.** A highlight follows the rim from one light direction, with a weaker one opposite. One light per
  screen; highlights from several directions read as plastic.
- **Shadow.** Larger glass casts a deeper shadow; shadows matter most over text and busy content.
- **No colour of its own.** Glass takes its colour from what is behind it. A tint behaves like coloured
  glass and is reserved for a primary action or a status.

## What a live system adapts, and how to freeze it

| Live behaviour | In a design |
|---|---|
| Small elements flip between light and dark glass with the content under them | Leave them on `auto`; set the foreground to the opposite; read `check_glass` |
| Large elements adapt but do not flip, and are more opaque | Pin `appearance` on sidebars, sheets, menus |
| Thickness, lensing and shadow grow with size | Automatic by `depth`; set `depth` when a small part belongs to a larger bar |
| Glass materializes by ramping its lensing, it does not fade | Frames with `advanced.refraction` 0, 0.5, 1 |
| Shapes merge and split like droplets while morphing | A glass group with `glassGroup` larger than the gap, as a transition frame |
| Tab bars shrink while scrolling down | A second frame with the compact bar |
| Glass in an unfocused window recedes | Lower `advanced.highlight` and `shadow` in the inactive window's frame |

## Shape

Shapes nest around shared centres ("concentric"): hardware corners, window corners, bars, buttons. A corner
that looks pinched or flared has the wrong radius: it should be the outer radius less the gap. Capsules are
the norm on touch screens; on dense desktop layouts small and medium controls keep rounded rectangles and
capsules mark the standout actions. An element near a phone's screen edge is a capsule with extra margin; on
desktop it is concentric with the window.

## Bars

Bars lost their backgrounds: hierarchy comes from grouping. Items that belong together share one glass
shape; a text button and an icon button in one shape read as a single button, so they are kept apart. The
primary action is separate and tinted. The tab bar floats, carries only navigation (no actions for one
screen), and search is its own tab at the trailing end. An accessory (for example playback controls) can sit
above the tab bar as its own capsule in the same glass group.

## Sheets, menus, alerts

A half-height sheet is inset from the edges so content shows around it; at full height it becomes more
opaque (a thick frosted or `tintOpacity` 0.8 pane). Menus and action sheets grow out of the control that
opened them: place them touching or just below that control, never at a far edge. Common menu items carry
icons. Alerts are bolder and left-aligned.

## Scroll edge effect

Where pinned controls overlap scrolling content, the content fades (and on a device, blurs) toward that edge
instead of meeting a divider. Soft (a gradual fade) is the default and suits glass buttons; hard (a more
uniform, more opaque band) suits dense desktop layouts and pinned headers. One per edge, never stacked,
never as decoration. Pencraft has no blur that varies along its length, so the soft style is a gradient fade.

## Colour and type

System colours are tokens, not fixed values; define variables with light and dark values. Foregrounds on
glass are monochrome. Type is bolder and left-aligned in key moments (alerts, onboarding); section headers
use title case, not capitals.

## What Pencraft does not do

No motion, no touch glow, no device-motion highlights, no corner smoothing (continuous corners), no blur
that changes along its length, and liquid glass only on frames, rectangles and whole ellipses (any other
shape is drawn frosted). SVG export writes the tint and the rim light, not the backdrop. Say so when a user
asks for one of these, and show states as separate frames.
