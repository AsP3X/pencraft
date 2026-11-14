# Sonner, Vaul, cmdk, NumberFlow and the emilkowalski/skills repository (state as of 14 Nov 2026)

Method note: every CSS/TS value below was read from the raw source files on GitHub (default branches) on 2026-11-14 unless marked "(unverified)". Short source labels used throughout:

- SONNER-CSS = https://github.com/emilkowalski/sonner/blob/main/src/styles.css
- SONNER-TSX = https://github.com/emilkowalski/sonner/blob/main/src/index.tsx
- SONNER-ASSETS = https://github.com/emilkowalski/sonner/blob/main/src/assets.tsx
- VAUL-CSS = https://github.com/emilkowalski/vaul/blob/main/src/style.css
- VAUL-CONST = https://github.com/emilkowalski/vaul/blob/main/src/constants.ts
- VAUL-SCALE = https://github.com/emilkowalski/vaul/blob/main/src/use-scale-background.ts
- VAUL-TSX = https://github.com/emilkowalski/vaul/blob/main/src/index.tsx
- CMDK-THEMES = https://github.com/pacocoursey/cmdk/tree/main/website/styles/cmdk (vercel.scss, linear.scss, raycast.scss, framer.scss)
- CMDK-GLOBALS = https://github.com/pacocoursey/cmdk/blob/main/website/styles/globals.scss
- NF-LITE = https://github.com/barvian/number-flow/blob/main/packages/number-flow/src/lite.ts
- SKILLS = https://github.com/emilkowalski/skills (files under /skills/<name>/)

---

## Sonner: exact toast anatomy, types, colours, stacking, motion, and why it feels good

### Takeaway
A Sonner toast is a 356px-wide, 16px-padded, 8px-radius white card with a 1px light-grey border and one soft shadow (0 4px 12px at 10% black), 13px text, a 16px icon slot and 24px-tall mini buttons; up to 3 are shown, stacked 14px apart and each 5% smaller than the one in front. Its motion is deliberately slower and softer than normal UI (400ms `ease`), which Emil says is a cohesion choice, and it is the documented exception to his under-300ms rule.

### Cited Findings

**Status**
- Latest release v2.0.8, published 2026-08-09; repo last pushed 2026-08-10; ~13.2k stars; not archived — [GitHub API](https://api.github.com/repos/emilkowalski/sonner)
- Repo description: "An opinionated toast component for React." — [GitHub](https://github.com/emilkowalski/sonner)

**Container (the "Toaster")**
- Width 356px (`TOAST_WIDTH = 356`, exposed as `--width`) — [SONNER-TSX](https://github.com/emilkowalski/sonner/blob/main/src/index.tsx)
- Default position `bottom-right`; six positions: top-left, top-center, top-right, bottom-left, bottom-center, bottom-right — [SONNER-TSX](https://github.com/emilkowalski/sonner/blob/main/src/index.tsx); [ask-sonner API.md](https://github.com/emilkowalski/skills/blob/main/skills/ask-sonner/API.md)
- Offset from viewport edges 24px desktop (`VIEWPORT_OFFSET`), 16px mobile (`MOBILE_VIEWPORT_OFFSET`) — [SONNER-TSX](https://github.com/emilkowalski/sonner/blob/main/src/index.tsx)
- Font stack is the system UI stack (`ui-sans-serif, system-ui, -apple-system, ... Segoe UI, Roboto ...`); z-index 999999999; `--border-radius: 8px` — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Built-in grey scale (light): gray1 hsl(0,0%,99%), gray2 97.3%, gray3 95.1%, gray4 93%, gray5 90.9%, gray6 88.7%, gray7 85.8%, gray8 78%, gray9 56.1%, gray10 52.3%, gray11 43.5%, gray12 9% (all hue 0, saturation 0) — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)

**Toast card (styled default)**
- `padding: 16px`; `border: 1px solid var(--normal-border)`; `border-radius: 8px`; `box-shadow: 0px 4px 12px rgba(0,0,0,0.1)`; `width: 356px`; `font-size: 13px`; `display: flex; align-items: center; gap: 6px` — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Light theme: background `#fff`, border gray4 = hsl(0,0%,93%), text gray12 = hsl(0,0%,9%) — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Dark theme: background `#000`, border hsl(0,0%,20%), text gray1 = hsl(0,0%,99%); hover variants bg hsl(0,0%,12%), border hsl(0,0%,25%) (used by the close button) — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- `invert` flips: in light theme an inverted toast is `#000` bg / hsl(0,0%,20%) border / gray1 text; in dark theme `#fff` bg / gray3 border / gray12 text — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Focus ring: the shadow plus `0 0 0 2px rgba(0,0,0,0.2)` — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Title: weight 500, line-height 1.5, colour inherits — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Description: weight 400, line-height 1.4, colour `#3f3f3f` (light), hsl(0,0%,91%) (dark), `inherit` under rich colours — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Content column (title + description): vertical flex, `gap: 2px`, `flex: 1; min-width: 0` — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Icon slot: 16x16px box, `flex-shrink: 0`, margin-left -3px, margin-right 4px (LTR); the SVG inside gets margin-left -1px — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Status SVGs are authored at `width="20" height="20"` (viewBox 0 0 20 20; one at 0 0 24 24); the close-button X is 12x12 with stroke width 1.5 — [SONNER-ASSETS](https://github.com/emilkowalski/sonner/blob/main/src/assets.tsx)
- Loading spinner: 16px, 12 radial bars, bar colour gray11, each bar 24% wide x 8% tall with 6px radius, opacity fades 1 to 0.15 over 1.2s linear, bars offset by 0.1s — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css); [SONNER-ASSETS](https://github.com/emilkowalski/sonner/blob/main/src/assets.tsx)
- Action button: height 24px, horizontal padding 8px, radius 4px, font 12px weight 500, no border, pushed to the right with `margin-left: auto`; colours are inverted against the toast (background = toast text colour, text = toast background), i.e. near-black button with white text in light theme — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Cancel button: same geometry; text = toast text colour, background `rgba(0,0,0,0.08)` (light) or `rgba(255,255,255,0.3)` (dark) — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Button focus ring: `0 0 0 2px rgba(0,0,0,0.4)` — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Close button (off by default): 20x20px circle (radius 50%), absolutely placed at the toast's top-left corner and shifted `translate(-35%, -35%)` so it overhangs the corner (top-right, `translate(35%,-35%)` in RTL); same background and 1px border as the toast; hover bg gray2, border gray5 — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)

**Types and options**
- Calls: `toast()` (title only), `{ description }`, `toast.success/.error/.info/.warning`, `toast.loading`, `toast.promise(promise, {loading, success, error})`, `{ action: {label, onClick} }`, `{ cancel }`, `toast(<jsx/>)` (custom content, default shell), `toast.custom()` (headless, no styles) — [ask-sonner SKILL.md](https://github.com/emilkowalski/skills/blob/main/skills/ask-sonner/SKILL.md); [Sonner docs](https://sonner.emilkowal.ski/toast)
- Defaults: duration 4000ms (`Infinity` persists), `visibleToasts` 3, `gap` 14, `expand` false, `closeButton` false, `richColors` false, `theme` 'light' (does not follow the OS unless `theme="system"`), `dismissible` true, hotkey Alt+T focuses the toast region — [ask-sonner API.md](https://github.com/emilkowalski/skills/blob/main/skills/ask-sonner/API.md); [SONNER-TSX](https://github.com/emilkowalski/sonner/blob/main/src/index.tsx)
- Without `richColors`, success and error toasts are the neutral card with only an icon; the skill's troubleshooting table calls grey success/error "the default" — [ask-sonner SKILL.md](https://github.com/emilkowalski/skills/blob/main/skills/ask-sonner/SKILL.md)
- Clicking an action closes the toast unless the handler prevents default; cancel is "the secondary variant" — [ask-sonner SKILL.md](https://github.com/emilkowalski/skills/blob/main/skills/ask-sonner/SKILL.md)
- Promise toasts: the incoming status icon enters from `scale(0.8)` + opacity 0 over 300ms `ease`; the spinner leaves to `scale(0.8)` + opacity 0 over 200ms — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)

**Rich colours palette (background / border / text; close button takes the same three)** — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)

| Type | Light bg | Light border | Light text | Dark bg | Dark border | Dark text |
| --- | --- | --- | --- | --- | --- | --- |
| success | hsl(143, 85%, 96%) | hsl(145, 92%, 87%) | hsl(140, 100%, 27%) | hsl(150, 100%, 6%) | hsl(147, 100%, 12%) | hsl(150, 86%, 65%) |
| info | hsl(208, 100%, 97%) | hsl(221, 91%, 93%) | hsl(210, 92%, 45%) | hsl(215, 100%, 6%) | hsl(223, 43%, 17%) | hsl(216, 87%, 65%) |
| warning | hsl(49, 100%, 97%) | hsl(49, 91%, 84%) | hsl(31, 92%, 45%) | hsl(64, 100%, 6%) | hsl(60, 100%, 9%) | hsl(46, 87%, 65%) |
| error | hsl(359, 100%, 97%) | hsl(359, 100%, 94%) | hsl(360, 100%, 45%) | hsl(358, 76%, 10%) | hsl(357, 89%, 16%) | hsl(358, 100%, 81%) |

**Stacking (collapsed) and expanding**
- Every toast is `position: absolute`; toasts behind the front one are translated by `gap x index` (14px, 28px ...) toward the stack direction and scaled `1 - 0.05 x index` (0.95, 0.90) — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css) (`--scale: var(--toasts-before) * 0.05 + 1`, `--lift-amount: lift x gap`); confirmed in [Building a toast component](https://emilkowal.ski/ui/building-a-toast-component)
- For bottom positions the older toasts peek out above the front toast (lift -1); for top positions they peek out below (lift 1) — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Collapsed back toasts take the front toast's height and all their children are `opacity: 0` (so they read as empty card edges) — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Only 3 are visible (`VISIBLE_TOASTS_AMOUNT = 3`); the rest are `opacity: 0; pointer-events: none` — [SONNER-TSX](https://github.com/emilkowalski/sonner/blob/main/src/index.tsx); [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Expanded (hover, or `expand` prop): each toast returns to its own height and is offset by `index x gap + sum of heights of toasts in front`, i.e. a plain list with a 14px gap — [SONNER-TSX](https://github.com/emilkowalski/sonner/blob/main/src/index.tsx)
- An invisible `::after` strip `gap + 1px` tall fills the space between expanded toasts so hover is not lost — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)

**Mobile**
- At `max-width: 600px` the toaster spans the full width and each toast is `calc(100% - 2 x mobile offset)` wide, i.e. full width minus 16px each side; centre positions lose their centring transform — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)

**Motion and gestures**
- Enter: from `translateY(100%)` (bottom) or `-100%` (top) and opacity 0, to 0 / 1 — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Toast transition: `transform 400ms, opacity 400ms, height 400ms, box-shadow 200ms` (no easing named, so CSS default `ease`); toaster `transform 400ms ease` — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Exit of the front toast: back the way it came (`translateY(lift x -100%)`, opacity 0); a collapsed back toast exits with `translateY(40%)`, `transform 500ms, opacity 200ms` — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Unmount delay after removal 200ms (`TIME_BEFORE_UNMOUNT`) — [SONNER-TSX](https://github.com/emilkowalski/sonner/blob/main/src/index.tsx)
- Swipe dismiss when distance >= 45px (`SWIPE_THRESHOLD`) OR velocity > 0.11 px/ms, and only in an allowed direction — [SONNER-TSX](https://github.com/emilkowalski/sonner/blob/main/src/index.tsx)
- Default allowed swipe directions are the two parts of the position name (e.g. bottom-right allows down and right) — [SONNER-TSX](https://github.com/emilkowalski/sonner/blob/main/src/index.tsx)
- Dragging the wrong way is damped, not blocked: multiplier `1 / (1.5 + |delta|/20)` — [SONNER-TSX](https://github.com/emilkowalski/sonner/blob/main/src/index.tsx)
- Swipe-out animation: 200ms `ease-out`, translate a further 100% in the swipe direction, opacity to 0 — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- `prefers-reduced-motion` removes all toast transitions and animations — [SONNER-CSS](https://github.com/emilkowalski/sonner/blob/main/src/styles.css)
- Timers pause on hover and while the browser tab is hidden — [Building a toast component](https://emilkowal.ski/ui/building-a-toast-component)

**Emil's stated reasoning**
- CSS transitions were chosen over keyframes because keyframes cannot be retargeted; rapidly added toasts jumped instead of gliding — [Building a toast component](https://emilkowal.ski/ui/building-a-toast-component)
- The 0.11 velocity value was found by trial and error — [Building a toast component](https://emilkowal.ski/ui/building-a-toast-component)
- Sonner is "slightly slower than typical UI animations" and uses `ease` rather than `ease-out` to feel more elegant; easing, duration, toast design, page design and name are meant to be in harmony — [emil-design-eng SKILL.md](https://github.com/emilkowalski/skills/blob/main/skills/emil-design-eng/SKILL.md); [Great animations](https://emilkowal.ski/ui/great-animations)
- The toast enters and exits from the same direction so swipe-to-dismiss feels intuitive (spatial consistency) — [emil-design-eng SKILL.md](https://github.com/emilkowalski/skills/blob/main/skills/emil-design-eng/SKILL.md); [gestures SKILL.md](https://github.com/emilkowalski/skills/blob/main/skills/gestures/SKILL.md)
- The six "Sonner principles": DX first (one `<Toaster />`, call `toast()` anywhere); good defaults matter more than options; naming creates identity (French "to ring"); handle edge cases invisibly; transitions not keyframes; build a great interactive docs site — [emil-design-eng SKILL.md](https://github.com/emilkowalski/skills/blob/main/skills/emil-design-eng/SKILL.md)
- "Beauty is underutilized in software" is his stated reason good defaults are a differentiator — [emil-design-eng SKILL.md](https://github.com/emilkowalski/skills/blob/main/skills/emil-design-eng/SKILL.md)
- Styling ladder: defaults (+ `richColors`, `invert`), inline style, per-part class names (need `!important`), then headless `toast.custom()`, which the skill calls the recommended end state for a design-system toast — [ask-sonner SKILL.md](https://github.com/emilkowalski/skills/blob/main/skills/ask-sonner/SKILL.md)

### Inferences
- Static spec for a Pencraft "toast" component (light): frame 356 wide, auto height, horizontal auto-layout, padding 16, gap 6, cross-axis centre, fill #FFFFFF, stroke 1px #EDEDED (hsl 0 0% 93%), radius 8, shadow y4 blur12 #000 at 10%. Title 13px/500 line-height 1.5 in #171717 (hsl 0 0% 9%); description 13px/400 line-height 1.4 in #3F3F3F; 2px between them. With title only, the card is about 53.5px tall (16 + 19.5 + 16 + 2 border); with description about 74px (computed from the CSS, not measured).
- A stack mock-up should show at most 3 cards: front at 100%, second 14px further from the screen edge at 95% width, third 28px at 90% width, both back cards empty and the same height as the front card. Scaling is from the card centre (default transform origin), so each back card is inset equally left and right.
- An "expanded" state frame is simply the three full cards in a column with 14px gaps.
- Rich-colour toasts are tinted cards (pale bg, slightly stronger border, saturated text), not solid colour blocks; the same hue triple is reused for icon and text because the colour is set on the card and inherited.
- The action button reads as a tiny inverted pill (24px tall, 4px radius) rather than a text link, which is the main visual differentiator from Material-style snackbars.
- Because animation cannot be drawn, the transferable craft is: one axis for enter and exit, the peeking-edge stack to show "more behind", and a motion personality matched to the visual calm of the card.

### Gaps
- The Sonner docs page fetched did not list separate info/warning sections even though the functions exist in the API table; the docs site itself was not inspected for its page styling.
- Exact rendered size of the 20px-authored status SVG inside the 16px slot was not measured in a browser.
- Whether a header/footer, multi-line action layout or stacked buttons exist as documented variants was not found.

---

## Vaul: drawer anatomy, handle, background scaling, easing, snap points, nested drawers, maintenance

### Takeaway
Vaul is an unstyled, Radix-Dialog-based iOS-style sheet: the only visuals it ships are a 32x5px grey handle and the motion (500ms on `cubic-bezier(0.32, 0.72, 0, 1)`), plus an optional effect that shrinks the page behind to `(viewport width - 26) / viewport width`, rounds it to 8px and pushes it down 14px on a black body. The repository's README declares it unmaintained (last release v1.1.2, Dec 2024), yet Emil's skills still name it the default drawer choice in Nov 2026.

### Cited Findings

**Status**
- README opens with a note that the repo "is unmaintained", described as a hobby project the author lacks time for — [Vaul README](https://github.com/emilkowalski/vaul/blob/main/README.md)
- Latest release v1.1.2 published 2024-12-14; last push 2025-10-03; ~8.7k stars; not archived — [GitHub API](https://api.github.com/repos/emilkowalski/vaul)
- Despite that, pick-ui-library (updated Nov 2026) lists Vaul for "Drawers / bottom sheets", and the gestures skill says to default to Vaul for a bottom sheet — [pick-ui-library](https://github.com/emilkowalski/skills/blob/main/skills/pick-ui-library/SKILL.md); [gestures](https://github.com/emilkowalski/skills/blob/main/skills/gestures/SKILL.md)
- The docs site showed no maintenance notice when fetched — [Vaul docs](https://vaul.emilkowal.ski/getting-started)

**Anatomy**
- Parts: Root, Trigger, Portal, Overlay, Content, Close, Title, Description, Handle (Title and Description are optional accessible labels; Handle is an optional drag handle) — [Vaul API docs](https://vaul.emilkowal.ski/api)
- `NestedRoot` exists in source for drawers opened from inside a drawer — [VAUL-TSX](https://github.com/emilkowalski/vaul/blob/main/src/index.tsx)
- API mirrors Radix Dialog on purpose; built on Radix Dialog for focus and accessibility — [Building a drawer component](https://emilkowal.ski/ui/building-a-drawer-component)
- Root defaults: `modal` true, `direction` 'bottom', `dismissible` true, `handleOnly` false, `repositionInputs` true, `shouldScaleBackground` false, `setBackgroundColorOnScale` true, `closeThreshold` 0.25, `scrollLockTimeout` 100 — [VAUL-TSX](https://github.com/emilkowalski/vaul/blob/main/src/index.tsx); [VAUL-CONST](https://github.com/emilkowalski/vaul/blob/main/src/constants.ts)
- Directions: bottom, top, left, right; each slides in from `100%` of its own size on that axis — [VAUL-CSS](https://github.com/emilkowalski/vaul/blob/main/src/style.css)
- The docs' minimal example styles Overlay as `fixed inset-0 bg-black/40` (40% black) and Content as `bg-gray-100 h-fit fixed bottom-0 left-0 right-0` with an inner `p-4 bg-white` block — [Vaul docs](https://vaul.emilkowal.ski/getting-started)

**Handle (the only shipped visual)**
- 32px wide, 5px tall, `border-radius: 1rem` (fully round), colour `#e2e2e4`, opacity 0.7 (1 on hover/active), centred horizontally — [VAUL-CSS](https://github.com/emilkowalski/vaul/blob/main/src/style.css)
- Its invisible hit area is at least 44x44px (`max(100%, 2.75rem)`) — [VAUL-CSS](https://github.com/emilkowalski/vaul/blob/main/src/style.css)

**Motion**
- `transition: transform 0.5s cubic-bezier(0.32, 0.72, 0, 1)`; same curve and 0.5s for the overlay fade — [VAUL-CSS](https://github.com/emilkowalski/vaul/blob/main/src/style.css); [VAUL-CONST](https://github.com/emilkowalski/vaul/blob/main/src/constants.ts)
- The curve comes from the Ionic Framework and, with 500ms, was chosen to match the iOS sheet — [Building a drawer component](https://emilkowal.ski/ui/building-a-drawer-component)
- The curve is very steep at the start, which lets 500ms still feel snappy with a spring-like settle — [gestures](https://github.com/emilkowalski/skills/blob/main/skills/gestures/SKILL.md)
- A small floating drawer (not attached to the screen edge) should be overridden to about 200ms with `cubic-bezier(0.165, 0.84, 0.44, 1)`; the 500ms timing only suits a full-width edge-attached sheet — [gestures](https://github.com/emilkowalski/skills/blob/main/skills/gestures/SKILL.md)
- A `::after` pseudo-element extends the drawer's own background 200% beyond its outer edge so overshoot never reveals a gap — [VAUL-CSS](https://github.com/emilkowalski/vaul/blob/main/src/style.css)

**Background scaling (`shouldScaleBackground`)**
- Scale of the page wrapper = `(window.innerWidth - 26) / window.innerWidth` (`WINDOW_TOP_OFFSET = 26`) — [VAUL-SCALE](https://github.com/emilkowalski/vaul/blob/main/src/use-scale-background.ts); [VAUL-CONST](https://github.com/emilkowalski/vaul/blob/main/src/constants.ts)
- Wrapper also gets `border-radius: 8px` (`BORDER_RADIUS = 8`), `overflow: hidden`, and `translate3d(0, calc(env(safe-area-inset-top) + 14px), 0)`; transform origin is `top` for vertical drawers and `left` for horizontal — [VAUL-SCALE](https://github.com/emilkowalski/vaul/blob/main/src/use-scale-background.ts)
- `document.body` background is set to `black` while open (unless disabled), so the shrunken page sits on black — [VAUL-SCALE](https://github.com/emilkowalski/vaul/blob/main/src/use-scale-background.ts)
- During a drag the scale and radius follow drag progress (dragging 40% of the way leaves 60% of the radius) — [Building a drawer component](https://emilkowal.ski/ui/building-a-drawer-component)

**Nested drawers**
- When a nested drawer opens, the parent drawer scales to `(window.innerWidth - 16) / window.innerWidth` and moves -16px (`NESTED_DISPLACEMENT = 16`) — [VAUL-TSX](https://github.com/emilkowalski/vaul/blob/main/src/index.tsx); [VAUL-CONST](https://github.com/emilkowalski/vaul/blob/main/src/constants.ts)

**Snap points, drag and dismissal**
- `snapPoints` is an array of fractions of the viewport or fixed pixel values; on release the drawer snaps to the nearest point, and momentum can skip points or close it — [Building a drawer component](https://emilkowal.ski/ui/building-a-drawer-component)
- `fadeFromIndex` defaults to the last snap point: the overlay only fades in from that snap point upward; `snapToSequentialPoint` (default false) stops velocity from skipping points — [VAUL-TSX](https://github.com/emilkowalski/vaul/blob/main/src/index.tsx)
- Constants: `VELOCITY_THRESHOLD = 0.4`, `CLOSE_THRESHOLD = 0.25` (fraction of drawer size dragged before it closes), `SCROLL_LOCK_TIMEOUT = 100` ms — [VAUL-CONST](https://github.com/emilkowalski/vaul/blob/main/src/constants.ts)
- Over-drag past the open position is damped logarithmically: `8 * (ln(v + 1) - 2)` — [Vaul helpers.ts](https://github.com/emilkowalski/vaul/blob/main/src/helpers.ts)
- Drag is only allowed when inner content is scrolled to top, and is blocked for 100ms after reaching the top — [Building a drawer component](https://emilkowal.ski/ui/building-a-drawer-component)
- Drag position is written to `transform` directly; an earlier CSS-variable approach lagged with roughly 20+ list items — [Building a drawer component](https://emilkowal.ski/ui/building-a-drawer-component)
- `data-vaul-no-drag` on interactive children stops a press from starting a drag — [gestures](https://github.com/emilkowalski/skills/blob/main/skills/gestures/SKILL.md)
- `modal={false}` allows interaction with the page behind (no focus trap/overlay behaviour); `handleOnly` restricts dragging to the handle — [VAUL-TSX](https://github.com/emilkowalski/vaul/blob/main/src/index.tsx)
- Emil's stated preference: a drawer rather than a modal on mobile — [Building a drawer component](https://emilkowal.ski/ui/building-a-drawer-component)

### Inferences
- Background scale is width-relative, so it differs by device: 390px-wide phone = 364/390 = 0.933; 430px = 0.940; 768px = 0.966; 1440px = 0.982. The visible result on a phone is a 13px margin each side of the page plus a 14px top offset below the safe area.
- Static "sheet open" frame recipe: black canvas; page frame scaled to about 93.3% (phone), radius 8, top-anchored and offset 14px below the status-bar inset; overlay #000 at 40% over it; sheet anchored to the bottom, full width, top corners rounded; handle 32x5 pill #E2E2E4 at 70% opacity centred near the top.
- Nested sheet frame: parent sheet scaled to (w-16)/w (0.959 at 390px) and raised 16px behind the child sheet.
- Snap points are best shown as separate frames (e.g. peek, half, full) with the overlay absent below the fade index and present at the top one.
- The 44px hit area around a 5px handle is a concrete instance of Emil's "44px minimum hit area via pseudo-element" rule.

### Gaps
- Sheet corner radius, background colour, max width and handle margins are user styling, not shipped by Vaul. The commonly copied example values (`rounded-t-[10px]`, handle `w-12 h-1.5 bg-gray-300`/zinc-300, content `h-[96%] mt-24`) are from memory and were NOT verified in this session; the docs page fetched showed only `bg-gray-100`, `p-4 bg-white` and `bg-black/40`.
- Per-prop descriptions are absent from the API docs page; behaviour descriptions above come from source and the article.
- No successor or official fork of Vaul was identified; whether Base UI's own drawer is the intended replacement was not established.

---

## cmdk: anatomy, the four showcased themes, and filtering/ranking behaviour

### Takeaway
cmdk is an unstyled command-menu primitive; its website shows four fully specified skins (Vercel, Linear, Raycast, Framer), all 640px max width with a `0 16px 70px` 20%-black shadow, Inter, 14px items, 12px group headings and 20px-square keycaps, differing mainly in input size, item height/radius and selection treatment. It filters and re-sorts automatically with a fuzzy score that favours continuous and word-start matches.

### Cited Findings

**Status**
- Latest release v1.1.1 published 2025-03-14; last push 2025-10-29; ~13.1k stars; not archived — [GitHub API](https://api.github.com/repos/pacocoursey/cmdk)
- Described as a "Fast, unstyled command menu React component" that also works as an accessible combobox — [cmdk README](https://github.com/pacocoursey/cmdk/blob/main/README.md)

**Anatomy**
- Parts and their selectors: Command `[cmdk-root]`, Dialog `[cmdk-dialog]` + `[cmdk-overlay]`, Input `[cmdk-input]`, List `[cmdk-list]`, Item `[cmdk-item]` (with `data-selected`, `data-disabled`), Group `[cmdk-group]` with `[cmdk-group-heading]`, Separator `[cmdk-separator]`, Empty `[cmdk-empty]`, Loading `[cmdk-loading]` — [cmdk README](https://github.com/pacocoursey/cmdk/blob/main/README.md)
- Dialog composes Radix UI Dialog and always renders an overlay; inline (non-dialog) and popover (combobox) uses are supported — [cmdk README](https://github.com/pacocoursey/cmdk/blob/main/README.md)
- List exposes `--cmdk-list-height` so the list height can animate as results change — [cmdk README](https://github.com/pacocoursey/cmdk/blob/main/README.md)
- Separator is visible only when the search is empty (or `alwaysRender`) — [cmdk README](https://github.com/pacocoursey/cmdk/blob/main/README.md)

**Shared tokens on the demo site** — [CMDK-GLOBALS](https://github.com/pacocoursey/cmdk/blob/main/website/styles/globals.scss)
- Font: Inter, then system stack. Shadow `--cmdk-shadow: 0 16px 70px rgb(0 0 0 / 20%)`.
- Light greys identical to Sonner's scale (gray1 99% ... gray12 9%). Dark greys: gray1 hsl(0,0%,8.5%), gray2 11%, gray3 13.6%, gray4 15.8%, gray5 17.9%, gray6 20.5%, gray7 24.3%, gray8 31.2%, gray9 43.9%, gray10 49.4%, gray11 62.8%, gray12 93%.
- Alpha greys (light): grayA3 = black at 4.7%, grayA11 = black at 56.5%; (dark) grayA3 = white at 5.6%.
- Blue: blue3 hsl(209,100%,96.5%), blue9 hsl(206,100%,50%), blue11 hsl(211,100%,43.2%).

**Theme specs** — [CMDK-THEMES](https://github.com/pacocoursey/cmdk/tree/main/website/styles/cmdk)

| Property | Vercel | Linear | Raycast | Framer |
| --- | --- | --- | --- | --- |
| Root max width | 640px | 640px | 640px | 640px |
| Root padding | 8px | 0 | 8px 0 | 8px |
| Root radius | 12px | 8px | 12px | 16px |
| Root background (light) | #ffffff | #ffffff | gray1 | #ffffff |
| Root background (dark) | rgba(22,22,22,0.7) | gradient 136.61deg rgb(39,40,43) to rgb(45,46,49) | gray2 | gray2 |
| Root border | 1px gray6 | none | 1px gray6 (dark: none, animated 1px edge shine) | 1px gray6 |
| Input font size | 17px | 18px | 15px (16px under 640px) | 16px |
| Input padding | 8px 8px 16px 8px | 20px | 8px 16px | in a 48px header row |
| Input divider | 1px gray6 bottom, 16px margin below | 1px gray6 bottom | 1px loader line with 12px margins | 1px gray5 under header, 12px below |
| Placeholder colour | gray9 | gray9 | gray9 | gray9 |
| Item height | 48px | 48px | 40px | auto, padding 8px (32px icon tile gives 48px) |
| Item padding | 0 16px | 0 16px | 0 8px | 8px |
| Item radius | 8px | 0 (full-bleed) | 8px | 12px |
| Item font | 14px | 14px | 14px | 14px, weight 500 |
| Item gap (icon to label) | 8px | 12px | 8px | 12px |
| Item text colour | gray11, selected gray12 | gray12 | gray12 | gray12 |
| Selected item | bg grayA3 | bg gray3 + 3px left bar #5f6ad2 | bg gray4 | bg blue9, text white |
| Gap between items | 4px | 4px | 4px | 4px |
| Icon size | 18px | 16px, colour gray10 (gray12 selected) | 18px | 16px white in 32px tile, radius 8 |
| List height | min(330px, content), max 400px | min(300px, content), max 400px | fixed 393px, 40px bottom padding | items area min 308px |
| Group heading | 12px gray11, padding 0 8px, 8px below | 12px gray11, padding 0 8px | 12px gray11, padding 0 8px | 12px gray11, padding 0 8px, 8px below |
| Group spacing | 8px above each group | n/a | 8px above each group | n/a |
| Separator | 1px gray5, margin 4px 0 | n/a | 1px gray5, margin 4px 0 | 1px gray5, margin 4px 0 |
| Empty state | 14px gray11, centred, 48px tall | 14px gray11, 64px tall | 14px gray11, 64px tall | 14px gray11, padding 32px |

- Vercel keycaps: `kbd` 20px high, min-width 20px, padding 4px, radius 4px, 12px uppercase, text gray11 on gray4, 8px gap between keys, right-aligned — [vercel.scss](https://github.com/pacocoursey/cmdk/blob/main/website/styles/cmdk/vercel.scss)
- Vercel breadcrumb badge (page scope chip above input): 20px high, padding 0 8px, 12px weight 500, radius 4px, text grayA11 on grayA3 — [vercel.scss](https://github.com/pacocoursey/cmdk/blob/main/website/styles/cmdk/vercel.scss)
- Vercel demo content: placeholder "What do you need?", groups Projects / Teams / Help, shortcuts such as "S P" and "⇧ P", empty text "No results found." — [vercel.tsx](https://github.com/pacocoursey/cmdk/blob/main/website/components/cmdk/vercel.tsx)
- Linear: context badge above the input ("Issue - FUN-343"): 24px high, padding 0 8px, 12px, radius 4px, gray11 on gray3, margin 16px 16px 0; shortcuts are plain 13px gray11 letters with no keycap box, 8px apart; caret colour #6e5ed2; placeholder "Type a command or search..." — [linear.scss](https://github.com/pacocoursey/cmdk/blob/main/website/styles/cmdk/linear.scss); [linear.tsx](https://github.com/pacocoursey/cmdk/blob/main/website/components/cmdk/linear.tsx)
- Raycast keycaps: 20x20px, radius 4px, padding 0 4px, gray11 on gray3 — [raycast.scss](https://github.com/pacocoursey/cmdk/blob/main/website/styles/cmdk/raycast.scss)
- Raycast item meta (right-aligned type label "Application"/"Command"): 13px gray11 — [raycast.scss](https://github.com/pacocoursey/cmdk/blob/main/website/styles/cmdk/raycast.scss); [raycast.tsx](https://github.com/pacocoursey/cmdk/blob/main/website/components/cmdk/raycast.tsx)
- Raycast footer: 40px high, padding 8px, 1px gray6 top border, background matches root, bottom corners 12px; 20px greyscale logo on the left; right side "Open Application ↵", a 12px-tall 1px divider, then "Actions ⌘ K"; footer triggers are 28px high, 12px weight 500, radius 6px, letter-spacing -0.25px — [raycast.scss](https://github.com/pacocoursey/cmdk/blob/main/website/styles/cmdk/raycast.scss); [raycast.tsx](https://github.com/pacocoursey/cmdk/blob/main/website/components/cmdk/raycast.tsx)
- Raycast app-icon tiles: 20x20px, radius 6px, vertical gradient (#f55354 to #eb4646; #6cb9a3 to #2c6459), 14px white glyph — [raycast.scss](https://github.com/pacocoursey/cmdk/blob/main/website/styles/cmdk/raycast.scss)
- Raycast actions sub-menu (popover): 320px wide, radius 8px, 1px gray6 border, bg gray2, list padding 8px, 40px items at 13px, selected bg gray5, search input at the bottom (padding 12px, 13px, top border); enters from `scale(0.96)` in 0.2s — [raycast.scss](https://github.com/pacocoursey/cmdk/blob/main/website/styles/cmdk/raycast.scss)
- Raycast groups in the demo: "Suggestions" and "Commands"; placeholder "Search for apps and commands..." — [raycast.tsx](https://github.com/pacocoursey/cmdk/blob/main/website/components/cmdk/raycast.tsx)
- Framer: two-pane layout, left list 40% and right live preview 60% separated by a 1px gray6 rule; items have a title plus 12px gray11 subtitle (white when selected, 4px apart) and a 32px rounded icon tile; header row 48px with a 20px gray9 search icon; placeholder "Find components, packages, and interactions..." — [framer.scss](https://github.com/pacocoursey/cmdk/blob/main/website/styles/cmdk/framer.scss); [framer.tsx](https://github.com/pacocoursey/cmdk/blob/main/website/components/cmdk/framer.tsx)
- Item selection has no transition in Vercel/Linear/Framer themes (`transition-property: none`); only the list height animates (100ms ease) — [CMDK-THEMES](https://github.com/pacocoursey/cmdk/tree/main/website/styles/cmdk)

**Filtering and ranking**
- Items are filtered and sorted automatically as the user types; `shouldFilter={false}` turns both off; `filter` can be replaced; `keywords` act as aliases that also affect rank; `forceMount` keeps an item or group visible regardless; `loop` wraps arrow-key navigation — [cmdk README](https://github.com/pacocoursey/cmdk/blob/main/README.md)
- Score constants: continuous match 1; word start after a space 0.9; word start after other separators 0.8; any other character jump 0.17; transposed letters 0.1; each skipped character x0.999; case mismatch x0.9999; distance from start x0.9; incomplete match x0.99 — [command-score.ts](https://github.com/pacocoursey/cmdk/blob/main/cmdk/src/command-score.ts)
- The source comments give the intent: "html" outranks "html5" when "html" is typed, and "HTML" outranks "haml" for "HM" — [command-score.ts](https://github.com/pacocoursey/cmdk/blob/main/cmdk/src/command-score.ts)
- Emil's rule for this component class: the command-palette toggle is a 100+/day keyboard action and gets no animation; "Raycast has no open/close animation" — [emil-design-eng](https://github.com/emilkowalski/skills/blob/main/skills/emil-design-eng/SKILL.md)
- Choreograph skill: never stagger command-palette rows or menu items — [choreograph](https://github.com/emilkowalski/skills/blob/main/skills/choreograph/SKILL.md)

### Inferences
- Common skeleton for a designed palette: 640px card, big shadow (y16 blur70 black 20%), radius 8 to 16; input row 15 to 18px text with gray9 placeholder; hairline; scroll list about 300 to 400px tall; 12px grey group headings inset 8px; 40 to 48px rows with 14px labels, 16 to 18px icons, 4px between rows; right-aligned 20px keycaps (4px radius, 12px text); exactly one selected row.
- A realistic "typed query" state frame should (a) show fewer rows re-ordered so prefix and word-start matches come first, (b) drop groups that have no matches entirely (heading included), (c) hide separators, (d) keep the first row selected, and (e) have an "empty" state frame with a single centred 14px grey line.
- The four themes show four distinct selection idioms worth offering as variants: subtle alpha fill (Vercel), fill plus accent bar (Linear), neutral fill with footer action hints (Raycast), saturated accent fill (Framer).
- The first row always being selected by default follows from keyboard-first use; a mock-up with no selected row looks wrong for this component.

### Gaps
- The demo site was not rendered; values are from SCSS source. Rendered heights of auto-height parts (Framer items, Vercel root) are computed, not measured.
- The demo dialog width, top offset (`--page-top`) and overlay colour for `[cmdk-overlay]` were not extracted.
- The real Vercel, Linear, Raycast and Framer products were not inspected; these are Paco Coursey's recreations and may differ from the products' current UI.
- `[cmdk-loading]` has no theme styling in the four files apart from Raycast's 1px animated loader line.

---

## NumberFlow: what it does, formatting, timing defaults, and implications for static number design

### Takeaway
NumberFlow renders a number via `Intl.NumberFormat` and, on change, rolls each digit vertically like an odometer while symbols and digits that appear or disappear fade and the layout slides (900ms spring-like curve, 450ms fade). For static design the lesson is: format with locale-aware rules, use tabular (equal-width) numerals wherever a value can change or sits in a column, and treat prefix/suffix/currency symbols as separate, smaller-styleable parts.

### Cited Findings

**Status**
- Core package release `number-flow@0.6.2` published 2026-07-18; last push 2026-09-20; ~7.8k stars; not archived — [GitHub API](https://api.github.com/repos/barvian/number-flow)
- Described as "An animated number component for React, Vue, Svelte, and TS/JS" — [GitHub](https://github.com/barvian/number-flow)
- Docs describe it as dependency-free, accessible and customizable; built as a custom element with CSS `::part`s — [NumberFlow docs](https://number-flow.barvian.me/)
- Emil's pick-ui-library names it for "Animating numbers (counters, prices, stats)" and flags animating a number by re-rendering text as a mismatch — [pick-ui-library](https://github.com/emilkowalski/skills/blob/main/skills/pick-ui-library/SKILL.md)

**Props and defaults**
- `value` (number); `format` = `Intl.NumberFormatOptions` (e.g. `{ notation: 'compact' }`, currency, percent); `locales`; `prefix` / `suffix` strings — [NumberFlow docs](https://number-flow.barvian.me/)
- Default `transformTiming`: duration 900ms with a long `linear(...)` easing table (a spring-like decelerating curve reaching about 0.90 by 43% of the time) — [NF-LITE](https://github.com/barvian/number-flow/blob/main/packages/number-flow/src/lite.ts)
- Default `spinTiming`: undefined, falls back to `transformTiming` — [NF-LITE](https://github.com/barvian/number-flow/blob/main/packages/number-flow/src/lite.ts)
- Default `opacityTiming`: 450ms `ease-out` — [NF-LITE](https://github.com/barvian/number-flow/blob/main/packages/number-flow/src/lite.ts)
- Default `trend`: `Math.sign(value - oldValue)`: +1 digits always roll up, -1 always roll down, 0 each digit takes the shorter direction — [NF-LITE](https://github.com/barvian/number-flow/blob/main/packages/number-flow/src/lite.ts); [NumberFlow docs](https://number-flow.barvian.me/)
- `animated` true; `respectMotionPreference` true; `willChange` false; `isolate` false — [NumberFlow docs](https://number-flow.barvian.me/)
- `plugins`: one plugin, `continuous`, which makes digits pass through intermediate numbers; no effect when trend is 0 — [NumberFlow docs](https://number-flow.barvian.me/)
- `digits`: per-position maximum (e.g. countdowns where seconds wrap 59 to 00) — [NumberFlow docs](https://number-flow.barvian.me/)
- `<NumberFlowGroup>` synchronises several NumberFlows whose widths affect each other (docs example: `$124.23` beside `+5.64%`); renders no element — [NumberFlow docs](https://number-flow.barvian.me/)

**Styling**
- Root is `white-space: nowrap`, `direction: ltr`, `line-height: 1` by default; docs examples use 0.85 — [NumberFlow styles.ts](https://github.com/barvian/number-flow/blob/main/packages/number-flow/src/styles.ts); [NumberFlow docs](https://number-flow.barvian.me/)
- Edge fade masks: `--number-flow-mask-height` default 0.25em (also acts as top/bottom padding), `--number-flow-mask-width` default 0.5em — [NumberFlow styles.ts](https://github.com/barvian/number-flow/blob/main/packages/number-flow/src/styles.ts)
- Parts such as `suffix` can be styled separately (docs example sets the suffix to 0.75rem) — [NumberFlow docs](https://number-flow.barvian.me/)
- Docs recommend `font-variant-numeric: tabular-nums` so digits keep equal width and do not shift during transitions — [NumberFlow docs](https://number-flow.barvian.me/)
- Limitations: no scientific/engineering notation, no non-Latin digits or RTL locales; backgrounds and borders on the element do not scale smoothly — [NumberFlow docs](https://number-flow.barvian.me/)
- Docs examples: hero `431.10`; price `$3/mo` (USD with `/mo` suffix); `42%`; countdown; activity counters; coloured trends; input stepper — [NumberFlow docs](https://number-flow.barvian.me/); [site source tree](https://github.com/barvian/number-flow/tree/main/site/src/pages)

**Emil's related rules on numbers**
- "Numbers jitter when they change" is fixed with `font-variant-numeric: tabular-nums` — [ui-cheatsheet.md](https://github.com/emilkowalski/skills/blob/main/ui-cheatsheet.md)
- Use `tabular-nums` in price columns so digits align — [Agents with Taste](https://emilkowal.ski/ui/agents-with-taste)
- Raw numbers (`1284`, `NaN`) should go through `Intl.NumberFormat`; "1 members" through `Intl.PluralRules`; never truncate numbers, amounts or dates — [break-ui](https://github.com/emilkowalski/skills/blob/main/skills/break-ui/SKILL.md)
- Functional data should not move for style: no decorative animation on a graph in a banking app — [emil-design-eng](https://github.com/emilkowalski/skills/blob/main/skills/emil-design-eng/SKILL.md)

### Inferences
- NumberFlow's 900ms is far above Emil's 300ms cap; it fits his "rare, explanatory or delight" tier, so it belongs on occasional value changes (price toggle monthly/yearly, a stat updating, a counter after an action), not on values that change many times a minute.
- Static design rules to encode: (1) any number that can change, tick, or sits in a table column uses tabular figures; proportional figures only for fixed display headlines; (2) right-align numeric columns and keep decimals fixed (e.g. always 2) so the decimal point lines up; (3) group thousands and place currency per locale; (4) compact notation (1.2K, 3.4M) for dense stat tiles, full precision in detail views; (5) show sign and colour for trends ("+5.64%" beside the value); (6) style currency symbols and unit suffixes ("/mo", "%") as lighter or smaller secondary parts; (7) reserve width for the widest plausible value so neighbours do not shift.
- A "before/after" pair of state frames with identical layout and tabular digits is the honest static equivalent of a digit roll.

### Gaps
- Exact values of the 900ms `linear()` curve as a named spring (stiffness/damping) are not stated by the author.
- The docs' own guidance page text on tabular numerals was read only in summary; no explicit list of "use for prices, counters, stats" exists in the docs beyond the examples.
- Whether per-digit width is forced equal by NumberFlow itself without `tabular-nums` was not verified.

---

## The emilkowalski/skills repository: contents, structure, pick-ui-library in full, the other skills, and Emil's published principles

### Takeaway
The repo holds 18 Agent Skills (one folder each, a `SKILL.md` plus zero to two sibling reference files), nearly all about motion and interface polish, written as strict gates, decision tables, "never ship" lists and mandatory output formats with exact numbers and the reason for each. `pick-ui-library` is a short lookup table that names base-ui for primitives (plus Sonner, Vaul, cmdk, NumberFlow and others) and contains no discussion of shadcn/ui or Radix at all.

### Cited Findings

**Repository facts**
- Created 2026-03-16; last push 2026-11-13; ~56.9k stars, ~3.3k forks; description "Skills for Designers and Engineers." — [GitHub API](https://api.github.com/repos/emilkowalski/skills)
- Install: `npx skills@latest add emilkowalski/skills`; listed on skills.sh; each skill has a page at aiforui.dev/skills showing when to use it, an example result and prompts to try — [README](https://github.com/emilkowalski/skills/blob/main/README.md)
- Root files: `README.md`, `LICENSE`, `ui-cheatsheet.md`, `performance-cheatsheet.md`, `.gitattributes` (marks skill markdown as linguist-detectable), `.gitignore`; all skills under `skills/<name>/` — [repo tree](https://github.com/emilkowalski/skills)
- README's rationale: agents "don't have great taste"; examples given are `ease-in` on an enter animation and a solid border where a semi-transparent shadow is wanted; the skills list small mistakes agents make and how to fix them — [README](https://github.com/emilkowalski/skills/blob/main/README.md)
- README says the skills are "a side-effect of domain-expertise" from work at Vercel and Linear — [README](https://github.com/emilkowalski/skills/blob/main/README.md)
- Most commits are co-authored by Claude models; the 2026-11-13 commit fixed wrong APIs and cross-skill contradictions (300ms-cap exceptions, stagger, easing tokens) after a multi-round review — [commit history](https://github.com/emilkowalski/skills/commits/main)
- Emil builds the skill files with Anthropic's skill-creator skill — [Agents with Taste](https://emilkowal.ski/ui/agents-with-taste)

**Every skill (folder: files, approx size, purpose)** — [skills directory](https://github.com/emilkowalski/skills/tree/main/skills)

| Skill | Files | Purpose |
| --- | --- | --- |
| emil-design-eng | SKILL.md (28KB) | Main philosophy skill: animation decisions, component polish, clip-path, gestures, performance, review checklist |
| animate | SKILL.md (12KB), RECIPES.md (8KB) | Build one animation by a 7-step ordered sequence; recipes for button, dropdown, tooltip, modal, drawer, toast, accordion, stagger, hold-to-confirm, tabs, scroll reveal, drag-dismiss |
| animate-expo | SKILL.md (18KB), RECIPES.md (17KB) | Same bar for React Native/Expo (Reanimated, Gesture Handler, haptics) |
| review-animations | SKILL.md (8KB), STANDARDS.md (10KB) | Strict review; ten standards; findings table plus Block/Approve verdict |
| improve-animations | SKILL.md, AUDIT.md, PLAN-TEMPLATE.md | Read-only codebase audit producing prioritised, self-contained plans for other agents |
| find-animation-opportunities | SKILL.md (10KB) | Read-only; proposes where motion would help and must list rejected candidates |
| remove-animations | SKILL.md (12KB) | Subtract-only; removes needless motion and must list what was kept |
| choreograph | SKILL.md (16KB), RECIPES.md (10KB) | Ordering/overlap of several moving elements |
| animation-performance | SKILL.md (14KB) | Dropped-frame diagnosis; symptom table |
| animation-vocabulary | SKILL.md (13KB) | Reverse-lookup glossary of motion terms in 12 categories |
| apple-design | SKILL.md (23KB) | Apple WWDC principles for fluid interfaces, materials, typography, eight design principles |
| gestures | SKILL.md (9KB) | Drag/swipe/sheet rules; track vs release phases |
| pick-ui-library | SKILL.md (4.7KB) | Curated library lookup |
| prototype | SKILL.md (8KB), PICKER.md (8KB) | Build 3 to 5 divergent variants behind a fixed picker |
| mobile-native | SKILL.md (17KB) | 11 mobile-web fixes keyed by symptom |
| break-ui | SKILL.md (15KB), CATALOG.md (8KB) | Stress UI with realistic worst-case data behind a Demo/Worst-case toggle |
| learn-taste | SKILL.md (9KB) | Turn a session's design corrections into rules with reasons in DESIGN.md |
| ask-sonner | SKILL.md (7KB), API.md (5KB) | Sonner setup, call picker, styling ladder, troubleshooting |
| write-swift | SKILL.md (43KB) | Modern Swift guidance (off-topic for UI design) |

**Frontmatter and shared structure**
- Frontmatter has only `name` and `description`; four skills (pick-ui-library, prototype, learn-taste, review-animations) add `disable-model-invocation: true`, making them user-invoked only — [skill files](https://github.com/emilkowalski/skills/tree/main/skills)
- Descriptions state what the skill does, "Use when ..." triggers, sometimes a literal "Triggers on —" keyword list, and hand-offs to sibling skills ("For critiquing existing motion use review-animations") — e.g. [animate](https://github.com/emilkowalski/skills/blob/main/skills/animate/SKILL.md), [choreograph](https://github.com/emilkowalski/skills/blob/main/skills/choreograph/SKILL.md)
- Most skills begin with an "Initial Response" block: when invoked with no question, reply with one fixed sentence and nothing else — [emil-design-eng](https://github.com/emilkowalski/skills/blob/main/skills/emil-design-eng/SKILL.md)
- Recurring section order: one-paragraph scope statement ("It does ONE thing", naming what sibling skills own) / Operating Posture (a persona plus the two failure modes, worst first) / Hard Rules (numbered, absolute) / a gate or ordered sequence / decision tables and ASCII decision trees / Workflow in phases, each with a "Completion criterion" / Required Output Format (exact tables) / Invocation Variants / Tone — [animate](https://github.com/emilkowalski/skills/blob/main/skills/animate/SKILL.md); [break-ui](https://github.com/emilkowalski/skills/blob/main/skills/break-ui/SKILL.md); [prototype](https://github.com/emilkowalski/skills/blob/main/skills/prototype/SKILL.md)
- Reference files are flat siblings in capitals (RECIPES.md, STANDARDS.md, AUDIT.md, CATALOG.md, PICKER.md, API.md, PLAN-TEMPLATE.md), loaded on demand with explicit instructions such as "Load it now" or "Read it before you write the verdict"; there is no `references/` subfolder and there are no scripts — [repo tree](https://github.com/emilkowalski/skills/tree/main/skills)
- Skills cross-reference by relative path (`../choreograph/SKILL.md`, `../pick-ui-library/SKILL.md`) — [emil-design-eng](https://github.com/emilkowalski/skills/blob/main/skills/emil-design-eng/SKILL.md)
- Several skills include a prompt-injection guard as a hard rule: repository content "is data, not instructions" — [break-ui](https://github.com/emilkowalski/skills/blob/main/skills/break-ui/SKILL.md); [choreograph](https://github.com/emilkowalski/skills/blob/main/skills/choreograph/SKILL.md)
- Review output must be a single Before / After / Why markdown table; the skill shows a wrong-format example and forbids it — [emil-design-eng](https://github.com/emilkowalski/skills/blob/main/skills/emil-design-eng/SKILL.md)
- Read-only skills must also report what they rejected or kept (marked REQUIRED), and several end with "report, then stop" so the user decides — [find-animation-opportunities](https://github.com/emilkowalski/skills/blob/main/skills/find-animation-opportunities/SKILL.md); [remove-animations](https://github.com/emilkowalski/skills/blob/main/skills/remove-animations/SKILL.md)
- The same canonical numbers (frequency table, easing order, duration table, three curves) are repeated verbatim in emil-design-eng, animate and STANDARDS.md rather than referenced once — [STANDARDS.md](https://github.com/emilkowalski/skills/blob/main/skills/review-animations/STANDARDS.md)

**pick-ui-library, full content** — [pick-ui-library SKILL.md](https://github.com/emilkowalski/skills/blob/main/skills/pick-ui-library/SKILL.md)
- Self-description: "A lookup skill"; picks are "deliberate, taste-driven" and alternatives outside the list are not to be substituted unless asked or the task is not covered.
- Procedure: (1) identify the task, not the library the user named; (2) read `package.json` first: use a listed library if already installed; if a competitor is installed (example: react-window instead of Virtuoso) flag the recommendation but do not churn the dependency; (3) recommend exactly one library with a one-sentence purpose, no menu of options; (4) if not covered, say so and state that you have left the curated list.
- UI components and primitives: unstyled accessible components (dialogs, popovers, menus, selects) = base-ui (base-ui.com); command menus = cmdk; toasts = Sonner; drawers/bottom sheets = Vaul; OTP inputs = input-otp; GUIs/control panels = Leva.
- Motion and visuals: general animation (springs, layout, enter/exit) = motion (Framer Motion); animating numbers = NumberFlow; animated text = torph; 3D globes = Cobe; dynamic OG images = Satori; syntax highlighting = shiki. Rule: use motion for springs, layout, exit or gesture values; a simple hover or fade needs only CSS transitions.
- Charts: real-time/streaming = Liveline; everything else = recharts.
- Interaction and performance: drag and drop = dnd kit; virtualization = Virtuoso.
- State and styling: state = zustand; conditional class strings = clsx; typed variants for Tailwind = cva; theme switching without flash = next-themes. Rule: clsx for ad-hoc conditionals, cva when a component has real variants (size, intent, state).
- "Common mismatches to catch": hand-built toasts or toasts made with a modal library (use Sonner); a div-based dropdown/dialog with manual focus handling (use base-ui); animating a number by re-rendering text (NumberFlow); rendering 1,000+ rows directly (Virtuoso); per-component state web (zustand); nested className ternaries (clsx or cva).
- shadcn/ui and Radix are not mentioned anywhere in this skill. Across the whole repo they appear once, in improve-animations, as examples of component libraries to detect during recon ("Radix, Base UI, shadcn/ui") — [improve-animations](https://github.com/emilkowalski/skills/blob/main/skills/improve-animations/SKILL.md)
- Related: Emil's articles reference both Radix and Base UI transform-origin variables, and one says shadcn/ui already implements origin-aware popovers — [7 practical animation tips](https://emilkowal.ski/ui/7-practical-animation-tips); [Good vs great animations](https://emilkowal.ski/ui/good-vs-great-animations)

**Core numeric rules (emil-design-eng, animate, STANDARDS.md)** — [emil-design-eng](https://github.com/emilkowalski/skills/blob/main/skills/emil-design-eng/SKILL.md); [STANDARDS.md](https://github.com/emilkowalski/skills/blob/main/skills/review-animations/STANDARDS.md)
- Frequency gate: 100+ times/day (keyboard shortcuts, command palette toggle) no animation ever; tens/day (hover, list navigation) remove or drastically reduce; occasional (modals, drawers, toasts) standard; rare/first-time (onboarding, celebrations) may add delight.
- Valid purposes: spatial consistency, state indication, explanation, feedback, preventing a jarring change. "It looks cool" on a frequent element is not valid.
- Easing order: entering/exiting = ease-out; moving/morphing on screen = ease-in-out; hover/colour = ease; constant motion = linear; default ease-out. Never ease-in on UI.
- Curves: `--ease-out: cubic-bezier(0.23, 1, 0.32, 1)`; `--ease-in-out: cubic-bezier(0.77, 0, 0.175, 1)`; `--ease-drawer: cubic-bezier(0.32, 0.72, 0, 1)`. Built-in CSS easings are called too weak; take others from easing.dev or easings.co.
- Durations: button press 100 to 160ms; tooltips/small popovers 125 to 200ms; dropdowns/selects 150 to 250ms; modals/drawers 200 to 500ms; marketing can be longer. UI stays under 300ms; modals, drawers, sheets and toasts are the documented exceptions (Sonner 400ms ease).
- Exit faster than enter (example enter 250ms, exit 200ms); stagger 30 to 80ms between items, non-uniform (example delays 0, 50, 90, 120ms), with 8px translateY and 300ms.
- Press feedback: `scale(0.97)` on `:active` with `transform 160ms`; acceptable range 0.95 to 0.98.
- Never animate from `scale(0)`: start at 0.9 to 0.97 (examples use 0.95) plus opacity 0.
- Popovers, dropdowns and tooltips scale from their trigger (`transform-origin`); modals are exempt and stay centred.
- Tooltips: delay the first, then adjacent tooltips open instantly with no animation; tooltip transition 125ms, start `scale(0.97)`.
- Blur: add `filter: blur(2px)` to mask a crossfade that will not settle; keep blur under 20px.
- Springs: Apple-style `{duration: 0.5, bounce: 0.2}` or `{mass 1, stiffness 100, damping 10}`; bounce 0.1 to 0.3 and avoided in most UI.
- Hold-to-delete: 2s linear fill on press, 200ms ease-out on release (slow where the user decides, fast where the system responds).
- Hover motion only behind `@media (hover: hover) and (pointer: fine)`; reduced motion means gentler (keep opacity/colour, drop movement), not zero.
- Only `transform` and `opacity` (clip-path and measured accordion height are exceptions); never `transition: all`.
- Philosophy headings: "Taste is trained, not innate", "Unseen details compound", "Beauty is leverage"; review work the next day with fresh eyes and in slow motion.

**ui-cheatsheet.md (static-design relevant rows)** — [ui-cheatsheet.md](https://github.com/emilkowalski/skills/blob/main/ui-cheatsheet.md)
- Old timestamps: switch to an absolute date after a week.
- Numbers that change: `tabular-nums`.
- Large headings: tighter tracking, e.g. `-0.02em`.
- Body text: cap line length at about 65 characters (`max-width: 65ch`).
- Avatars: `flex-shrink: 0` so they never squash.
- Gradient fades: use an eased gradient with extra colour stops to avoid a visible edge.
- Counts: pluralise properly ("1 members" is a bug).
- performance-cheatsheet.md covers stutter, virtualisation, blur under 20px, no `transition: all`, `will-change` only when needed — [performance-cheatsheet.md](https://github.com/emilkowalski/skills/blob/main/performance-cheatsheet.md)

**Other skills, key content**
- review-animations: ten standards (justified motion, frequency-appropriate, responsive easing, sub-300ms, origin and physical correctness, interruptibility, GPU-only properties, accessibility, asymmetric enter/exit, cohesion); a list of flag-on-sight triggers; a nine-step fix preference order starting with "Delete the animation"; "Default to flagging; approval is earned" — [review-animations](https://github.com/emilkowalski/skills/blob/main/skills/review-animations/SKILL.md)
- animate: five hard rules including "No approximated values" (never invent `cubic-bezier(0.4, 0, 0.2, 1)`), extend existing tokens rather than fork them, cheapest tool that works; never offer motion options as a menu; a "Never Ship" table of 13 rows; if the task needs a component (toast, drawer, command menu, dropdown), stop and consult pick-ui-library — [animate](https://github.com/emilkowalski/skills/blob/main/skills/animate/SKILL.md)
- choreograph: cast each element as Lead, Support, Decoration or Static; one entrance per container; uniform stagger forbidden; no stagger on high-frequency UI; relationships are Together, Overlap or Sequence; "If more than one element wants to be the lead, the design has a hierarchy problem" — [choreograph](https://github.com/emilkowalski/skills/blob/main/skills/choreograph/SKILL.md)
- gestures: track phase is 1:1 with the finger (no easing), release phase is physics; dismiss on distance (about 40 to 50% of travel) or velocity; hit targets at least 44x44px via an invisible pseudo-element; elements dismiss in the direction they entered — [gestures](https://github.com/emilkowalski/skills/blob/main/skills/gestures/SKILL.md)
- apple-design, static-design content: translucent chrome (`rgba(255,255,255,0.6)` + `blur(20px) saturate(180%)` + bright 1px top edge); heavier/darker materials for structure, lighter for interactive; never stack a light translucent surface on another; bigger surfaces get stronger blur and deeper shadow; modal = dimming scrim and pushed-back background, non-blocking panel = no scrim; fade/blur masks instead of 1px dividers under sticky headers; tracking is size-specific (display negative, e.g. -0.02em with line-height 1.05; body near 0); hierarchy from weight + size + leading together; spring table (move 1.0/0.4, rotation 0.8/0.4, drawer 0.8/0.3 damping/response); rubber-band constant 0.55; eight principles (Purpose, Agency, Responsibility, Familiarity, Flexibility, Simplicity, Craft, Delight); four feedback kinds (status, completion, warning, error); wayfinding questions; specific labels over generic ones — [apple-design](https://github.com/emilkowalski/skills/blob/main/skills/apple-design/SKILL.md)
- break-ui: worst-case values must be plausible or schema-backed (a long hyphenated Polish surname, a 64-character corporate email, a one-letter-ish name "Jo", 1,284 members); states Demo / Worst case / Empty / One / 1,000 rows; check at real container width, 320px, widest, 200% zoom, dark mode, RTL; failure-signature table (squished avatar, overflow, wrapped badge, orphaned dash, "1 members", proportional figures, clipped diacritics); per-field choice of wrap, end-truncate, middle-truncate, clamp, and never truncate numbers or dates; severities Broken / Ugly / Fragile — [break-ui](https://github.com/emilkowalski/skills/blob/main/skills/break-ui/SKILL.md)
- prototype: default 3 variants (max 5), each on a named axis (layout, density, personality, motion, interaction model), named by direction ("Quiet", "Editorial", "Playful", "Dense") never "Option A/B/C"; realistic content, no lorem ipsum; judged one at a time full size in context, never thumbnails; the picker is fixed chrome copied verbatim; table of when each wins and what it costs; never pre-pick a favourite — [prototype](https://github.com/emilkowalski/skills/blob/main/skills/prototype/SKILL.md)
- learn-taste: collects five correction signals (explicit correction, silent edit, repeated nudge, revert/rejection, confirmed non-obvious choice); rules must trace to the user, never invent a why (mark it inferred and ask), generalise to the component class not the instance or the universe, prefer a lint check over a sentence; writes to DESIGN.md sections Surfaces, Color, Typography, Spacing & layout, Motion, Interaction, Copy with a "Why:" line and a dated comment quoting the user; includes a self-test (would a fresh agent with only this rule get it right?) — [learn-taste](https://github.com/emilkowalski/skills/blob/main/skills/learn-taste/SKILL.md)
- find-animation-opportunities / remove-animations: both run a gate of frequency, purpose, speed, function and must list rejected or kept items — [find-animation-opportunities](https://github.com/emilkowalski/skills/blob/main/skills/find-animation-opportunities/SKILL.md); [remove-animations](https://github.com/emilkowalski/skills/blob/main/skills/remove-animations/SKILL.md)
- improve-animations: recon, parallel audit in 8 categories (purpose and frequency, easing and duration, physicality and origin, interruptibility, performance, accessibility, cohesion and tokens, missed opportunities), then numbered plan files from a template (Problem, Target, Repo conventions, Steps, Boundaries, Verification) — [improve-animations](https://github.com/emilkowalski/skills/tree/main/skills/improve-animations)
- animation-vocabulary: glossary in 12 groups (Entrances & Exits, Sequencing & Timing, Movement & Transforms, Transitions Between States, Scroll, Feedback & Interaction, Easing, Springs, Looping & Ambient, Polish & Effects, Performance, Principles) — [animation-vocabulary](https://github.com/emilkowalski/skills/blob/main/skills/animation-vocabulary/SKILL.md)
- mobile-native: symptom-keyed fixes (stuck hover, tap flash, 100vh bug, input zoom, laggy taps, pull-to-refresh, notch, long-press selection, carousel direction, status bar colour, test on hardware) — [mobile-native](https://github.com/emilkowalski/skills/blob/main/skills/mobile-native/SKILL.md)

**Emil's published principles (articles)**
- "7 practical animation tips": scale buttons to 0.97 on press; do not animate from scale(0) (start about 0.9+, demo 0.93, the deflated-balloon analogy); do not delay subsequent tooltips; ease-out for enter/exit (two 300ms dropdowns, the ease-in one feels slower); origin-aware popovers; keep UI animations under 300ms (180ms select beats 400ms; a faster spinner feels like faster loading); use blur (2px) when nothing else works — [7 practical animation tips](https://emilkowal.ski/ui/7-practical-animation-tips)
- "Great animations": feel natural (springs), fast (ease-out, usually under 300ms), purposeful (never animate keyboard actions; Raycast has none), performant (60fps, transform and opacity), interruptible, accessible (reduced motion: swap movement for fades), and "feel right" (Sonner slightly slower with `ease`; review the next day) — [Great animations](https://emilkowal.ski/ui/great-animations)
- "Good vs great animations": origin-aware; right easing (ease-in-out for things moving on screen, "just like a car"); custom curves because built-ins are weak, `ease` acceptable only for simple hovers; spring-smoothed mouse tracking for decoration only; clip-path for tab colour transitions; explore 3D transforms — [Good vs great animations](https://emilkowal.ski/ui/good-vs-great-animations)
- "Developing taste": taste is a trained instinct, not preference; surround yourself with great work, articulate why something works, and practise with feedback; when everything works, taste differentiates — [Developing taste](https://emilkowal.ski/ui/developing-taste)
- "Agents with Taste": write down why each choice feels right and hand those reasons to the agent as strict rules, one skill per aspect (animation, typography; layout, icons, colour next); use tables, flowcharts and numbers so the agent does not guess. Extra rules given there: durations micro 100 to 150ms, standard UI 150 to 250ms, modals/drawers 200 to 300ms; exits about 20% faster; larger elements and longer distances animate slower; 44px minimum hit area; typography: about 65ch body measure, tabular-nums in price columns, real ellipsis character, looser letter-spacing on uppercase labels, metric-matched fallback font, underline only links, bold for UI emphasis and italics for prose — [Agents with Taste](https://emilkowal.ski/ui/agents-with-taste)

### Inferences
- What transfers to a static-design tool even though Pencraft does not animate: transform-origin (place popovers visibly attached to and aligned with their trigger); spatial consistency (state frames should show an element entering from the edge it lives on); pressed-state frames at 97% scale; "nothing appears from nothing" (intermediate state frames at 95% scale, reduced opacity); semi-transparent shadows instead of solid borders on raised surfaces; tabular numerals; heading tracking -0.02em; 65-character measure; 44px touch targets; realistic worst-case content; one clear lead element per group.
- Authoring pattern Pencraft can copy for its own skills: (1) narrow single-purpose skills with explicit hand-offs to siblings in the description; (2) a persona plus two named failure modes ranked by severity; (3) numbered absolute hard rules; (4) an ordered gate where early steps can legitimately produce "do nothing"; (5) every value in a table, with a ban on approximated values; (6) a "Never / Instead" self-check table; (7) per-phase completion criteria; (8) a mandatory output format with a worked example and a shown wrong format; (9) heavy catalogues and recipes split into sibling files loaded on an explicit cue; (10) a one-line reason attached to each rule so the model can generalise.
- The repo duplicates canonical tables across skills, and the Nov 2026 "fix contradictions" commit shows the cost: duplicated numbers drift. A single shared reference would avoid that, at the price of an extra file load.
- There is a real inconsistency in duration guidance between sources: the skills allow modals/drawers 200 to 500ms, while the "Agents with Taste" article says 200 to 300ms; the skills (updated 2026-11-13) are the newer statement.
- The brief assumed pick-ui-library holds opinions on shadcn vs Base UI vs Radix. It does not; the only signal is that base-ui is the single named primitives library and Radix/shadcn are absent from the list.
- learn-taste is directly reusable as a Pencraft feature idea: harvest a user's design corrections into a per-project DESIGN.md with rule, reason and dated quote.

### Gaps
- animations.dev course material is paywalled and was not accessed; only public articles and the skills were read. Publication dates of the articles were not captured.
- aiforui.dev skill pages (examples and suggested prompts per skill) were not fetched.
- Read in full: README, both cheatsheets, pick-ui-library, emil-design-eng, review-animations (SKILL + STANDARDS), animate (SKILL), ask-sonner (SKILL + API), apple-design, learn-taste, break-ui (SKILL), gestures, prototype (SKILL). Read as headings plus the opening sections only: choreograph, find-animation-opportunities, remove-animations, improve-animations, animation-vocabulary, animation-performance, mobile-native, animate-expo, write-swift, and the RECIPES/CATALOG/PICKER/AUDIT/PLAN-TEMPLATE reference files. Details inside those are not covered here.
- No explicit statement from Emil on why base-ui was chosen over Radix or shadcn/ui was found.
- Star counts and dates are as returned by the GitHub API on 2026-11-14 and were not cross-checked elsewhere.
