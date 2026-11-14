# shadcn/ui design-system and component spec reference (state of the repo on 2026-11-13/14)

Method note: almost every number below was read directly from the `shadcn-ui/ui` GitHub repo `main` branch (latest commit seen: 2026-11-13) rather than from summaries. Source shorthands used in citations:

- [vega.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-vega.css) and siblings `style-{nova,maia,lyra,mira,luma,sera,rhea}.css` in [registry/styles](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/styles) = per-style visual classes (`.cn-*` rules made of Tailwind `@apply`).
- [bases/base/ui](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/ui) = Base UI component sources (structural classes that are the same in every style).
- [themes.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/themes.ts) = colour tokens. [globals.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/app/globals.css) = docs-site CSS. [config.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/config.ts) = presets/defaults.
- [new-york-v4/ui](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/new-york-v4/ui) = the legacy (2025) "new-york" Radix components.

Px conversions assume Tailwind v4 defaults: spacing unit 4px (`h-9` = 36px, `px-2.5` = 10px), `text-xs` 12/16, `text-sm` 14/20, `text-base` 16/24, `text-lg` 18/28, `text-xl` 20/28, `text-2xl` 24/32, `text-3xl` 30/36. These conversions are mine (standard Tailwind scale, [Tailwind docs](https://tailwindcss.com/docs/font-size)), not re-fetched this session.

## 1. Theming tokens (CSS variables, base colours, styles, radius)

### Takeaway
The token set is unchanged in shape (background/foreground pairs, border/input/ring, chart-1..5, sidebar-*), in OKLCH, but by late 2026 the *style* layer was completely rebuilt: there are eight styles (Vega, Nova, Maia, Lyra, Mira, Luma, Sera, Rhea), the default is **Base UI + Nova + neutral + Lucide + Inter**, base colours are now neutral/stone/zinc/mauve/olive/mist/taupe (slate and gray are gone from the picker), and the radius scale is multiplicative (`--radius` x 0.6/0.8/1/1.4/1.8/2.2/2.6).

### Cited Findings

**Neutral theme, light** (all from [themes.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/themes.ts)). Hex values in brackets are my approximate sRGB conversions (they match Tailwind's neutral ramp), not from the source.

| Token | Light | Dark |
|---|---|---|
| background | oklch(1 0 0) [#ffffff] | oklch(0.145 0 0) [#0a0a0a] |
| foreground | oklch(0.145 0 0) [#0a0a0a] | oklch(0.985 0 0) [#fafafa] |
| card | oklch(1 0 0) | oklch(0.205 0 0) [#171717] |
| card-foreground | oklch(0.145 0 0) | oklch(0.985 0 0) |
| popover | oklch(1 0 0) | oklch(0.205 0 0) |
| popover-foreground | oklch(0.145 0 0) | oklch(0.985 0 0) |
| primary | oklch(0.205 0 0) [#171717] | oklch(0.922 0 0) [#e5e5e5] |
| primary-foreground | oklch(0.985 0 0) [#fafafa] | oklch(0.205 0 0) |
| secondary | oklch(0.97 0 0) [#f5f5f5] | oklch(0.269 0 0) [#262626] |
| secondary-foreground | oklch(0.205 0 0) | oklch(0.985 0 0) |
| muted | oklch(0.97 0 0) | oklch(0.269 0 0) |
| muted-foreground | oklch(0.556 0 0) [#737373] | oklch(0.708 0 0) [#a1a1a1] |
| accent | oklch(0.97 0 0) | oklch(0.269 0 0) |
| accent-foreground | oklch(0.205 0 0) | oklch(0.985 0 0) |
| destructive | oklch(0.577 0.245 27.325) [~#e7000b, red-600] | oklch(0.704 0.191 22.216) [~#ff6467, red-400] |
| border | oklch(0.922 0 0) [#e5e5e5] | oklch(1 0 0 / 10%) (white at 10%) |
| input | oklch(0.922 0 0) | oklch(1 0 0 / 15%) (white at 15%) |
| ring | oklch(0.708 0 0) [#a1a1a1] | oklch(0.556 0 0) [#737373] |
| chart-1..5 | 0.87 / 0.556 / 0.439 / 0.371 / 0.269 (all `0 0` chroma: a grey ramp) | same five values |
| radius | 0.625rem (10px) | - |
| sidebar | oklch(0.985 0 0) [#fafafa] | oklch(0.205 0 0) |
| sidebar-foreground | oklch(0.145 0 0) | oklch(0.985 0 0) |
| sidebar-primary | oklch(0.205 0 0) | oklch(0.488 0.243 264.376) [~#1447e6, blue-700] |
| sidebar-primary-foreground | oklch(0.985 0 0) | oklch(0.985 0 0) |
| sidebar-accent | oklch(0.97 0 0) | oklch(0.269 0 0) |
| sidebar-accent-foreground | oklch(0.205 0 0) | oklch(0.985 0 0) |
| sidebar-border | oklch(0.922 0 0) | oklch(1 0 0 / 10%) |
| sidebar-ring | oklch(0.708 0 0) | oklch(0.556 0 0) |

- The neutral chart palette is now monochrome greys in both modes (it was multi-hue orange/teal/blue in 2025). The docs site itself overrides charts to `--color-blue-300/500/600/700/800` and uses pure black `oklch(0% 0 0)` foreground/primary plus extra tokens `surface`, `code`, `code-highlight`, `code-number`, `selection`; those extras are site-only, not part of the distributed theme. - [themes.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/themes.ts); [globals.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/app/globals.css)
- `destructive-foreground` is not in the distributed theme; destructive is used as a text/tint colour (see Button). - [themes.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/themes.ts)
- Token semantics per the docs: `card` = elevated surfaces; `popover` = floating surfaces; `primary` = high-emphasis actions; `secondary` = lower-emphasis filled actions; `muted` = subtle surfaces, descriptions, placeholders, helper text; `accent` = hover/focus/active surfaces (ghost buttons, menu highlight); `border` = borders and separators; `input` = form-control borders and input surface treatment; `ring` = focus rings; `sidebar-primary` = active items, icon tiles, sidebar CTAs; `sidebar-accent` = sidebar hover/selected. Convention: the surface token has no suffix and pairs with `-foreground` for text/icons on it. - [theming.mdx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/(root)/theming.mdx)
- Global base layer: `* { border-color: var(--border); outline-color: ring at 50% }`, body `bg-background text-foreground`. - [globals.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/app/globals.css)

**Base colours.** The picker list is `neutral, stone, zinc, mauve, olive, mist, taupe`. - [base-colors.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/base-colors.ts). They all share the same lightness skeleton and differ only by a very small chroma and a hue; white backgrounds, the destructive red, and dark-mode `border`/`input` (white 10%/15%) are identical across all of them. Key values (light: foreground / primary / muted=secondary=accent / muted-foreground / border=input / ring; dark background = light foreground, dark card = light primary, dark primary = light border) - [themes.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/themes.ts):

| Base | foreground | primary | muted | muted-fg | border | ring | dark muted |
|---|---|---|---|---|---|---|---|
| neutral | 0.145 0 0 | 0.205 0 0 | 0.97 0 0 | 0.556 0 0 | 0.922 0 0 | 0.708 0 0 | 0.269 0 0 |
| stone | 0.147 0.004 49.25 | 0.216 0.006 56.043 | 0.97 0.001 106.424 | 0.553 0.013 58.071 | 0.923 0.003 48.717 | 0.709 0.01 56.259 | 0.268 0.007 34.298 |
| zinc | 0.141 0.005 285.823 | 0.21 0.006 285.885 | 0.967 0.001 286.375 | 0.552 0.016 285.938 | 0.92 0.004 286.32 | 0.705 0.015 286.067 | 0.274 0.006 286.033 |
| mauve | 0.145 0.008 326 | 0.212 0.019 322.12 | 0.96 0.003 325.6 | 0.542 0.034 322.5 | 0.922 0.005 325.62 | 0.711 0.019 323.02 | 0.263 0.024 320.12 |
| olive | 0.153 0.006 107.1 | 0.228 0.013 107.4 | 0.966 0.005 106.5 | 0.58 0.031 107.3 | 0.93 0.007 106.5 | 0.737 0.021 106.9 | 0.286 0.016 107.4 |
| mist | 0.148 0.004 228.8 | 0.218 0.008 223.9 | 0.963 0.002 197.1 | 0.56 0.021 213.5 | 0.925 0.005 214.3 | 0.723 0.014 214.4 | 0.275 0.011 216.9 |
| taupe | 0.147 0.004 49.3 | 0.214 0.009 43.1 | 0.96 0.002 17.2 | 0.547 0.021 43.1 | 0.922 0.005 34.3 | 0.714 0.014 41.2 | 0.268 0.011 36.5 |

- Accent "themes" layered on a base colour: amber, blue, cyan, emerald, fuchsia, green, indigo, lime, orange, pink, purple, red, rose, sky, teal, violet, yellow. - [themes.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/themes.ts)

**Radius scale.** `--radius-sm: calc(var(--radius) * 0.6)`, `--radius-md: * 0.8`, `--radius-lg: var(--radius)`, `--radius-xl: * 1.4`, `--radius-2xl: * 1.8`, `--radius-3xl: * 2.2`, `--radius-4xl: * 2.6`. With the default `--radius: 0.625rem` (10px) that is sm 6px, md 8px, lg 10px, xl 14px, 2xl 18px, 3xl 22px, 4xl 26px. - [theming.mdx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/(root)/theming.mdx); [globals.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/app/globals.css)
- shadcn/create radius presets: Default (style's own), None `0`, Small `0.45rem`, Medium `0.625rem`, Large `0.875rem`. - [config.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/config.ts)

**Styles (eight, all available for Base UI, Radix and React Aria).** Descriptions from [styles.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles.tsx) and changelog; preset pairings from [config.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/config.ts):

| Style | Tagline | Default preset pairing | Control height (default button/input) | Signature geometry |
|---|---|---|---|---|
| Vega | "Clean, neutral, and familiar" / "the classic shadcn/ui look" | Lucide, Inter | 36px | rounded-md (8px) controls, rounded-xl (14px) cards, shadow-xs on controls |
| Nova | "Reduced padding and margins" | Lucide, Geist | 32px | rounded-lg (10px) controls, 16px card padding, no shadow on controls, muted footers |
| Maia | "Rounded, with generous spacing" | Hugeicons, Figtree | 36px | pill (rounded-4xl) controls, rounded-2xl cards, filled inputs `bg-input/30` |
| Lyra | "Boxy and sharp. For mono fonts" | icon lib `phosphor` (title text says Tabler; source conflict), JetBrains Mono | 32px | rounded-none everywhere, 12px text |
| Mira | "Made for compact interfaces" | Hugeicons, Inter | 28px | rounded-md, 12px text, 10px badges |
| Luma (Mar 2026) | "Fluid, luminous, and soft"; "Inspired by macOS Tahoe, minus the glass" | Lucide, Inter | 36px | pill controls, borderless filled inputs `bg-input/50`, rounded-4xl cards with shadow-md |
| Sera (Apr 2026) | "Editorial and typographic"; underline controls, uppercase headings | Lucide, Noto Sans body + Playfair Display headings, taupe base | 40px | square corners, underline-only inputs, uppercase tracked labels |
| Rhea (May 2026) | "A more compact Luma" | Lucide, Inter | 32px | rounded-2xl controls, card radius min(4xl, 24px), 20px card padding |

- Default config for new projects: `base: "base"` (Base UI), `style: "nova"`, `baseColor/theme/chartColor: "neutral"`, `iconLibrary: "lucide"`, `font: "inter"`, `fontHeading: "inherit"`, `menuAccent: "subtle"`, `menuColor: "default"`, `radius: "default"`, `template: "next"`; `components.json` in the docs shows `"style": "base-nova"`. - [config.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/config.ts); [theming.mdx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/(root)/theming.mdx)
- Why a new style rather than a spacing knob: "`--spacing` is a multiplier. Changing it would change what familiar utilities mean across your app" (Rhea announcement). - [2026-05-rhea changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-05-rhea.mdx)
- The legacy `new-york` style still exists in the repo as `new-york-v4` (Radix, unified `radix-ui` package since Feb 2026); the old `default` style is no longer present. - [new-york-v4/ui](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/new-york-v4/ui); [2026-02-radix-ui changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-02-radix-ui.mdx)

### Inferences
- A design tool can model "shadcn" as three orthogonal axes: colour theme (variables), style (geometry/spacing/typography), and icon/font choice. Components should bind fills/strokes to the semantic variables and never to literal greys.
- Dark mode is not an inversion: surfaces step up in lightness (background 0.145, card/popover 0.205, muted/secondary 0.269) and borders become translucent white (10%/15%), so borders must be drawn as alpha strokes, not solid greys.
- If Pencraft ships one preset, "Nova on neutral" is the 2026 default; "Vega" is what most people recognise as shadcn.

### Gaps
- Exact hex values: the source is OKLCH only; bracketed hex values are my conversions and should be recomputed by the renderer.
- I did not read the per-accent theme values (blue, rose, etc.) or the `menuAccent: "bold"` / `menuColor` transformations in detail.
- The pre-2026 radius formula (`calc(var(--radius) - 4px)` etc.) is from memory, not re-verified; the multiplicative scale above is what is in the repo now.

## 2. Typography scale and conventions

### Takeaway
shadcn uses a very narrow type range: 14px/medium for almost everything interactive, 12px for meta/labels in menus, 16px/medium for card titles, muted-foreground for all secondary text. Hierarchy comes from weight (400 vs 500) and colour (foreground vs muted-foreground), not size.

### Cited Findings (Vega unless noted; all from [vega.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-vega.css))
- Body/control text: `text-sm` (14/20) on buttons, inputs (inputs are `text-base` 16px below the `md` breakpoint, `md:text-sm`), selects, menu items, table, tabs, cards (`.cn-card` sets `text-sm`), dialogs, popovers.
- Card title: `text-base leading-normal font-medium` (16/24, 500); `text-sm` when card `size=sm`. Card description: `text-muted-foreground text-sm`.
- Dialog title: `leading-none font-medium` at inherited 14px in Vega (Nova/Maia/Luma/Rhea add `text-base`; Lyra/Mira `text-sm`; Sera `text-lg font-semibold uppercase tracking-wider`). Alert-dialog title: `text-lg font-medium` (18px). Sheet/drawer title: `text-foreground font-medium`. Descriptions: `text-muted-foreground text-sm`, links inside underlined with `underline-offset-3` and hover `text-foreground`.
- Label: `text-sm leading-none font-medium`, gap 8px to inline content, 50% opacity when its control is disabled. Field description/error: `text-sm` muted / `text-destructive`. Field legend: `font-medium`, `text-base` (legend variant) or `text-sm` (label variant), margin-bottom 12px.
- Menu group labels (dropdown/context/select/combobox/command headings): `text-xs` (12px) `text-muted-foreground`, padding 8px x 6px, `font-medium` for dropdown/context/command. Keyboard shortcuts in menus: `text-xs tracking-widest` (0.1em) muted, pushed right with `ml-auto`.
- Badge: `text-xs font-medium`. Kbd: `text-xs font-medium font-sans`. Tooltip: `text-xs`. Sidebar group label: `text-xs font-medium` at 70% sidebar-foreground. Table head: `font-medium text-foreground` (not muted, not uppercase) 14px. Table caption: muted `text-sm`, margin-top 16px.
- Empty title: `text-lg font-medium tracking-tight`; Empty description `text-sm/relaxed` muted. Item title: `text-sm leading-snug font-medium`; item description: muted `text-sm leading-normal`, clamped to 2 lines.
- Calendar weekday headers: `text-[0.8rem]` (12.8px) `font-normal text-muted-foreground`. - [calendar.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/calendar.tsx)
- Titles get a `cn-font-heading` class which maps to `font-heading`; `fontHeading` defaults to `inherit`, so headings use the body font unless a preset sets one (Sera: Playfair Display). - [card.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/card.tsx); [globals.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/app/globals.css); [config.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/config.ts)
- Block-level type: dashboard KPI value `text-2xl font-semibold tabular-nums` (30px in wide cards), site header title `text-base font-medium`, login-04 heading `text-2xl font-bold` with `text-balance text-muted-foreground` subtitle, login-03 card title `text-xl`. - [dashboard-01](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/blocks/dashboard-01); [login-04](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/blocks/login-04)
- Style-specific type: Lyra and Mira drop everything to `text-xs` (Mira `text-xs/relaxed`, 12px with 1.625 line-height; Mira badge and xs button `text-[0.625rem]` = 10px). Sera: buttons `text-xs font-semibold uppercase tracking-widest`, labels `text-xs font-semibold uppercase tracking-wide`, table heads `text-xs uppercase tracking-wider text-muted-foreground`, card titles `text-lg font-semibold tracking-wider uppercase`. - [style-mira.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-mira.css); [style-sera.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-sera.css)
- July 2026 added shadcn/typeset: one CSS file that styles raw HTML/markdown (headings, lists, tables, code) with three controls: size, leading, flow. - [2026-07-typeset changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-07-typeset.mdx)

### Inferences
- Rule of thumb for drawing: weights used are 400 and 500 only (600 appears just for KPI numbers, sidebar brand, Sera); never bold body text; never larger than 16px inside a component.
- Negative tracking is not used on component text (only `tracking-tight` on Empty title); wide tracking is reserved for keyboard shortcuts.

### Gaps
- Typeset's actual scale values were not read. Default font files/metrics (Inter vs Geist) not inspected beyond preset names.

## 3. Per-component anatomy and measurements

### Takeaway
Every component is a set of `data-slot` parts with fixed heights (36px default in Vega, 32px in Nova), 4px-grid paddings, 16px icons, 1px borders and a 3px focus ring. Tables below give Vega (classic) values with Nova deltas; the other six styles' deltas are listed at the end.

### Cited Findings
All Vega values: [vega.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-vega.css). Nova: [style-nova.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-nova.css). Structure: [bases/base/ui](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/ui).

**Button** ([button.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/button.tsx))
- Base: inline-flex centred, no wrap, `rounded-md` (8px), 1px transparent border with `bg-clip-padding`, `text-sm font-medium`, icons default 16px (`size-4`), `disabled:opacity-50`, pressed state nudges 1px down (`active:translate-y-px`).
- Variants (6): default `bg-primary text-primary-foreground`, hover `bg-primary/80`. outline `border-border bg-background shadow-xs`, hover `bg-muted text-foreground`; dark `bg-input/30 border-input`, hover `bg-input/50`. secondary `bg-secondary text-secondary-foreground`, hover = secondary mixed with 5% foreground. ghost transparent, hover `bg-muted text-foreground` (dark `bg-muted/50`). destructive is a *tint*, not solid red: `bg-destructive/10 text-destructive`, hover `bg-destructive/20` (dark `/20` then `/30`). link `text-primary`, underline on hover, `underline-offset-4`.
- Sizes (8), Vega: xs h 24px, px 8, gap 4, text 12px, icon 12px, radius min(radius-md, 8px). sm h 32, px 10, gap 4, radius min(radius-md, 10px). default h 36, px 10, gap 6. lg h 40, px 10, gap 6. icon 36x36, icon-xs 24, icon-sm 32, icon-lg 40. When an icon is marked `data-icon="inline-start|inline-end"` the padding on that side shrinks: 8px for default/lg, 6px for xs/sm.
- Nova sizes: xs 24, sm 28 (text 12.8px, icon 14px), default 32, lg 36, icon 32; radius `rounded-lg` (10px); outline has no shadow.
- Focus: `focus-visible:border-ring` + `ring-3` (3px) `ring-ring/50`, no offset. Invalid: `border-destructive` + 3px ring `destructive/20` (dark `/40`, border `destructive/50`).
- Legacy new-york-v4 for contrast: default `h-9 px-4 py-2` (16px padding, 12px when it has an icon), gap 8, lg `px-6`, solid `bg-destructive text-white`, hover `bg-primary/90`, outline/ghost hover use `accent`. - [new-york-v4 button](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/new-york-v4/ui/button.tsx)

**Input / Textarea / Native select**
- Input: h 36, `rounded-md`, 1px `border-input`, `bg-transparent` (dark `bg-input/30`), px 10, py 4, `shadow-xs`, placeholder `text-muted-foreground`, focus = border-ring + 3px ring/50, disabled 50% opacity. Nova: h 32, `rounded-lg`, no shadow, disabled fill `bg-input/50`.
- Textarea: same skin, px 10, py 8, `min-h-16` (64px), auto-grows (`field-sizing-content`).
- Native select: h 36 (sm 32), `pl-2.5 pr-8`, chevron 16px muted at right 10px.

**Select** ([select.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/select.tsx))
- Trigger: `w-fit`, space-between, h 36 (sm 32), `pl-2.5 pr-2`, gap 6, `rounded-md`, border-input, `shadow-xs`, placeholder muted, trailing ChevronDown 16px `text-muted-foreground`.
- Content: `bg-popover`, `rounded-md`, `shadow-md`, 1px `ring-foreground/10` (ring instead of border), min-width 144px, width = trigger width, side offset 4px. Group padding 4px. Item: `rounded-sm`, `py-1.5 pl-2 pr-8`, gap 8, text 14, highlight `bg-accent text-accent-foreground`; check indicator 16px absolutely at right 8px. Label 12px muted `px-2 py-1.5`. Separator 1px `bg-border`, `-mx-1 my-1`.

**Checkbox / Radio / Switch / Slider**
- Checkbox: 16x16, `rounded-[4px]`, 1px border-input, `shadow-xs`, dark `bg-input/30`; checked `bg-primary border-primary text-primary-foreground` with 14px Check icon; hit area extended 12px horizontally / 8px vertically. Maia radius 6px, Luma/Rhea 5px borderless filled `bg-input/90`, Sera 18px square.
- Radio: 16x16 circle, same border; checked = filled `bg-primary border-primary` with an 8px `bg-primary-foreground` dot (a filled disc with a light dot, not a ring with dark dot). Group: grid gap 12px.
- Switch: track 32 x 18.4px (sm 24 x 14), `rounded-full`, 1px transparent border, `shadow-xs`; unchecked `bg-input` (dark `bg-input/80`), checked `bg-primary`; thumb 16px (sm 12px) `bg-background`, travels `calc(100% - 2px)`. Luma: 44 x 20 with 2px border; Mira 28 x 16.6; Sera square 33 x 18.
- Slider: track 6px high `bg-muted rounded-full`, range `bg-primary`, thumb 16px circle `bg-white` with 1px `border-primary` and `shadow-sm`, hover/focus 4px ring/50. Nova: 12px thumb, `border-ring`. Luma: pill thumb 24 x 16 with shadow-md. Vertical min-height 160px.

**Badge**: h 20, px 8, py 2, gap 4, `rounded-4xl` (pill), 1px transparent border, 12px/500, icons 12px. Variants: default (primary), secondary, outline (`border-border text-foreground`), destructive (tint `bg-destructive/10 text-destructive`), ghost, link.

**Card** ([card.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/card.tsx))
- Root: flex column, `bg-card`, `rounded-xl` (14px), `ring-1 ring-foreground/10` (no border), `shadow-xs`, `text-sm`, vertical padding and gap both `--card-spacing` = 24px (16px for `size="sm"`); `overflow-hidden`; a leading image removes top padding.
- Header: grid, `px` = card spacing, gap 4px between title and description; optional `CardAction` in a second column spanning both rows, top-right. Content: `px` = spacing. Footer: `px` = spacing, flex row; adds top padding only when it has `.border-t`.
- Nova: spacing 16px (sm 12px), no shadow, footer is a tinted band `bg-muted/50 border-t` with full padding and the card's bottom padding removed. Maia: `rounded-2xl`. Luma: `rounded-4xl`, `shadow-md`, ring 5%. Sera: square, spacing 32px. Rhea: radius min(4xl, 24px), spacing 20px.
- Legacy new-york-v4: `rounded-xl border shadow-sm py-6 gap-6`, header gap 8.

**Dialog / Alert dialog** ([dialog.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/dialog.tsx))
- Overlay: `bg-black/10` plus `backdrop-blur-xs` (much lighter than the 2025 `bg-black/50`). Maia/Mira use `bg-black/80`, Luma/Rhea `/30`, Sera `/20`.
- Content: centred, `bg-popover`, `rounded-xl`, `p-6`, `gap-6`, `ring-1 ring-foreground/10`, no drop shadow in Vega, max-width `calc(100% - 2rem)` then `sm:max-w-md` (448px). Close button absolute top 16 / right 16 with X icon. Header: column, gap 8. Footer: column-reverse on mobile, row right-aligned on `sm`, gap 8.
- Nova: `p-4 gap-4`, `sm:max-w-sm` (384px), title `text-base`, footer is a full-bleed muted band (`bg-muted/50 -mx-4 -mb-4 border-t p-4 rounded-b-xl`).
- Alert dialog: same shell, max-w 320px (`max-w-xs`) growing to 512px (`sm:max-w-lg`) at default size; header centred on mobile / left on desktop, gap 6; optional media tile 64px `bg-muted rounded-md` with 32px icon; title `text-lg font-medium`.

**Sheet**: `bg-popover`, `shadow-lg`, 1px border on the inner edge, column gap 16; left/right width 75% capped at `sm:max-w-sm` (384px); header `p-4 gap-1.5`, footer `p-4 gap-2` pinned to bottom (`mt-auto`); close at top 16 / right 16; overlay `bg-black/10` + blur. Default side right.

**Drawer**: `bg-popover`, `rounded-t-xl` + top border for bottom drawers, handle 100 x 6px `bg-muted rounded-full` with 16px top margin; header `p-4` gap 2px (6px on md), footer `p-4 gap-2`.

**Popover / Hover card**: popover `w-72` (288px), `p-4`, `gap-4`, `rounded-md`, `shadow-md`, `ring-1 ring-foreground/10`, offset 4px, header gap 4 with `font-medium` title and muted description. Hover card: `w-64` (256px), `p-4`, `rounded-lg`. Nova popover: `p-2.5 gap-2.5 rounded-lg`.

**Dropdown menu / Context menu / Menubar**
- Content: `bg-popover`, `rounded-md` (8px), `p-1`, `shadow-md`, `ring-1 ring-foreground/10`, min-width 128px (context menu/menubar 144px), offset 4px, default align start.
- Item: `rounded-sm` (6px), `px-2 py-1.5` (row height 32px), gap 8, 14px text, 16px icons; highlight `bg-accent text-accent-foreground`; `inset` = 32px left padding; disabled 50% opacity; destructive items `text-destructive`, highlight `bg-destructive/10` (dark `/20`).
- Checkbox/radio items: `pl-2 pr-8` with the indicator at right 8px (this moved from the left in the legacy style, where it was `pl-8 pr-2`). Label 12px/500 muted. Separator 1px, `-mx-1 my-1`. Shortcut 12px muted `tracking-widest`. Sub-content `shadow-lg`, min-width 96px.
- Nova item: `rounded-md px-1.5 py-1` gap 6 (row 28px), content `rounded-lg`. Maia/Luma items `px-3 py-2 rounded-xl/2xl`, content min-width 192px.
- Menubar: h 36, `p-1`, gap 4, `rounded-md border shadow-xs`; trigger `px-2 py-1 rounded-sm text-sm font-medium`.
- Optional `cn-menu-translucent`: `bg-popover/70` with `backdrop-blur-2xl backdrop-saturate-150`.

**Command**: `bg-popover rounded-xl p-1`; input sits in an input-group h 32 `rounded-lg bg-input/30` with a 16px search icon at 50% opacity; list max-height 288px; group heading 12px/500 muted `px-2 py-1.5`; item `px-2 py-1.5 gap-2 rounded-sm`, selected `bg-muted text-foreground`; empty state `py-6 text-center text-sm`; shortcut 12px `tracking-widest`.

**Combobox**: content like select (max-h 288, min-w 144); chips field min-h 36 `px-2.5 py-1.5 gap-1.5`; chip h 22px `bg-muted rounded-sm px-1.5 text-xs font-medium`.

**Tooltip** ([tooltip.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/tooltip.tsx)): inverted colours `bg-foreground text-background`, `rounded-md`, `px-3 py-1.5`, 12px text, max-width 320px, gap 6, arrow = 10px square rotated 45deg with 2px radius, offset 4px, default side top, delay 0.

**Tabs** ([tabs.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/tabs.tsx)): list h 36, `bg-muted`, `rounded-lg`, padding 3px, `w-fit`; trigger fills height minus 1px, `rounded-md`, `px-2 py-1`, gap 6, 14px/500, inactive `text-foreground/60` (dark `text-muted-foreground`), active `bg-background text-foreground shadow-sm` (dark `bg-input/30 border-input`). `variant="line"`: transparent list, gap 4, active marked by a 2px `bg-foreground` underline 5px below the trigger. Tabs root gap 8. Nova: list h 32, trigger `px-1.5 py-0.5`.

**Accordion**: item `border-b` except last; trigger `py-4`, 14px/500, underline on hover, trailing ChevronDown 16px muted (swaps to ChevronUp when open rather than rotating); content 14px with `pb-4`. Nova trigger `py-2.5`.

**Table**: `w-full text-sm`; header row `border-b`; head cell h 40, `px-2`, left, `font-medium text-foreground`; cell `p-2`; rows `border-b`, hover `bg-muted/50`, selected `bg-muted`, last row no border; footer `bg-muted/50 border-t font-medium`. Maia/Luma/Sera: head h 48, `px-3`, cells `p-3`.

**Alert**: grid, `rounded-lg border`, `px-4 py-3`, 14px, `bg-card`; with a leading icon it becomes two columns (icon 16px, 10px column gap, icon nudged down 2px); title `font-medium`; description muted `text-sm`; row gap 2px; destructive variant = `text-destructive` on `bg-card` (no red fill); optional action slot absolute top 10 / right 12. Nova `px-2.5 py-2`.

**Avatar** ([avatar.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/avatar.tsx)): 32px default, 24px sm, 40px lg, `rounded-full`, with a 1px `border-border` overlay blended darken/lighten; fallback `bg-muted text-muted-foreground` 14px (12px for sm); status badge bottom-right `bg-primary` with 2px `ring-background`; group overlaps by -8px with 2px background ring per avatar; overflow count tile matches avatar size.

**Breadcrumb**: list 14px muted, gap 6px (10px from `sm`), wraps; links hover `text-foreground`; current page `text-foreground font-normal`; separator ChevronRight 14px; ellipsis 20px box with 16px icon.

**Pagination**: centred row gap 4; links are ghost buttons (outline when active) at `size="icon"` (36px); previous/next show text from `sm` with 8px padding on the chevron side; ellipsis 36px box with 16px icon.

**Progress**: track h 6px `bg-muted rounded-full`, indicator `bg-primary`; optional label 14px/500 and right-aligned muted tabular value. Nova 4px, Maia/Luma 12px, Rhea 8px, Sera 2px.

**Skeleton**: `bg-muted rounded-md` with pulse animation. **Separator**: 1px `bg-border`. **Spinner**: Lucide `Loader2` at 16px, spinning. **Kbd**: h 20, min-w 20, `px-1`, `rounded-sm`, `bg-muted text-muted-foreground`, 12px/500 sans, group gap 4; inside tooltips it becomes `bg-background/20 text-background`. **Scroll area**: 10px scrollbar gutter, pill thumb. **Resizable handle** grip: 4 x 24px `bg-border`.

**Toggle / Toggle group**: toggle h 36 (sm 32, lg 40), min-width = height, `px-2.5`, gap 4, `rounded-md`, 14px/500, transparent, hover `bg-muted`, pressed `bg-muted`; outline variant adds `border-input shadow-xs`. Group: default spacing 2 (8px gap between separate pills); `spacing=0` joins them (inner corners square, shared outline, `px-2`).

**Button group**: children joined with squared inner corners and collapsed inner borders; nested groups separated by 8px; text addon `bg-muted border rounded-md px-2.5 text-sm font-medium shadow-xs`; separator `bg-input`.

**Input group**: wrapper h 36 `rounded-md border-input shadow-xs` (dark `bg-input/30`) owns the border and focus ring; inner input is borderless/transparent; addons muted 14px/500 with 16px icons, 8px outer padding (inline) or `px-2.5 pt-2/pb-2` (block-start/end, turning the group into a column with auto height); embedded buttons default ghost `xs` (h 24, radius = radius - 5px).

**Field / Form** ([field.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/field.tsx)): `FieldSet` gap 24 (12 when it directly holds a checkbox/radio group); `FieldGroup` gap 28 (Nova 20), nested groups 16; `Field` gap 12 between label, control and description (Nova/Lyra/Mira 8); `FieldContent` gap 4; orientations vertical / horizontal / responsive (row from the `@md` container width); invalid field turns text destructive; choice-card pattern: a `FieldLabel` wrapping a `Field` gets `rounded-md border`, inner padding 12, hover `bg-muted/50`, checked `bg-primary/5 border-primary/30`; `FieldSeparator` h 20 with centred muted text on `bg-background` ("Or continue with"). `Form` (react-hook-form wrapper) remains only in new-york-v4; Field is the 2026 abstraction.

**Calendar**: padding 12, cell 32px (`--cell-size`), cell radius = radius-md, day button ghost with `font-normal`; selected `bg-primary text-primary-foreground`; range middle `bg-muted` square; today `bg-muted`; caption 14px; nav chevrons 16px; dropdown caption h 32. Nova: padding 8, cell 28. Mira: cell 24. Date picker = outline button + popover containing the calendar (calendar background goes transparent inside popover/card).

**Sidebar** ([sidebar.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/sidebar.tsx)): width 16rem (256px), mobile sheet 18rem (288px), icon-collapsed 3rem (48px); `bg-sidebar`, 1px side border; variants `sidebar` / `floating` (8px outer padding, `rounded-lg`, 1px ring, `shadow-sm`) / `inset` (content panel gets 8px margin, `rounded-xl`, `shadow-sm`); collapsible `offcanvas` / `icon` / `none`. Header/footer `p-2 gap-2`; content gap 8; group `p-2`; group label h 32 `px-2` 12px/500 at 70% opacity; menu gap 4; menu button h 32, `p-2`, gap 8, `rounded-md`, 14px, 16px icon, hover/active `bg-sidebar-accent`, active also `font-medium`; sizes sm h 28 / 12px, lg h 48; collapsed icon mode 32x32; sub-menu indented `mx-3.5 px-2.5` with a left border, sub-buttons h 28; badge h 20 min-w 20 12px; action button 20px at top 6 / right 4; separator `mx-2`; rail 16px hit area with 2px hover line; trigger icon PanelLeft; toggle shortcut key "b".

**Sonner / Toast**: Sonner is themed via `--normal-bg: var(--popover)`, `--normal-text: var(--popover-foreground)`, `--normal-border: var(--border)`, `--border-radius: var(--radius)`; status icons 16px: CircleCheck, Info, TriangleAlert, OctagonX, Loader2 (spinning). The new Base UI Toast (July 2026): viewport bottom-right, 16px inset, max-width 384px; toast `border bg-popover shadow-lg`, Vega radius `rounded-2xl`, stacked with 12px peek and 10% scale step; title `text-sm font-medium`, description `text-sm text-muted-foreground`, content gap 4. - [sonner.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/sonner.tsx); [toast.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/toast.tsx)

**Empty**: centred column, `p-12`, gap 16, `rounded-lg`, dashed border style available; header max-width 384px gap 8; icon media tile 40px `bg-muted rounded-lg` with 24px icon, 8px bottom margin; title 18px/500 tight; description muted; content gap 16. Nova `p-6 rounded-xl`.

**Item**: row, `rounded-md`, 1px border (transparent by default; `outline` variant `border-border`; `muted` variant `bg-muted/50`); sizes default `px-4 py-3.5 gap-3.5`, sm `px-3 py-2.5 gap-2.5`, xs `px-2.5 py-2 gap-2`; media: icon 16px or image 40px (32 sm, 24 xs) `rounded-sm`; content column gap 4; actions gap 8; group gap 16.

**Navigation menu**: trigger `px-4 py-2 rounded-md text-sm font-medium`, hover `bg-muted`, 12px chevron rotating 180deg; popup `bg-popover rounded-lg shadow ring-1 ring-foreground/10`; links `p-2 rounded-md gap-1.5`.

**Components added 2025-2026**
- Oct 2025: Spinner, Kbd, Button Group, Input Group, Field, Item, Empty. - [2025-10 changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2025-10-new-components.mdx)
- June 2026 chat set: MessageScroller, Message, Bubble, Attachment, Marker. Bubble: max-width 80%, content `rounded-xl px-3 py-2 text-sm leading-relaxed`, variants default (primary) / secondary / muted / tinted / outline / ghost / destructive; reactions pill overlaps the bubble edge with a 3px `ring-card`. Message: gap 8, avatar min 32px, header/footer 12px/500 muted `px-3`; scroller content gap 32. Attachment: `rounded-xl`, media tile 40px `bg-muted rounded-lg`, title `font-medium`, description 12px. Marker: 14px muted row with 16px icon, separator variant draws 1px rules either side. - [2026-06 changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-06-chat-components.mdx); [vega.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-vega.css)
- July 2026: Toast (Base UI), React Aria base. Aug 2026: Questionnaire (multi-step questions; choice rows `px-4 py-3.5 rounded-md border shadow-xs gap-3`, checked `border-primary/40 bg-muted`, 20px mono shortcut key chip). Oct 2026: Trace (agent steps: 14px lines with 16px icon, gap 8, expandable detail indented behind a left border) and Code Block (`rounded-lg border-input`, `px-4 py-4`, 14px, header/footer muted, line variants highlighted/added/removed), plus "AI Elements" (21 patterns built from shadcn components, replacing AI SDK Elements). - [2026-08 questionnaire](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-08-questionnaire.mdx); [2026-10 changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-10-ai-elements.mdx); [changelog index](https://ui.shadcn.com/docs/changelog)
- Other components present in the registry: aspect-ratio, carousel, chart, collapsible, combobox, direction, input-otp (slots 36px, joined, active slot ring), native-select, resizable. - [bases/base/ui](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/ui)

**Style deltas at a glance** (from the eight style files in [registry/styles](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/styles)):

| | Vega | Nova | Maia | Lyra | Mira | Luma | Sera | Rhea |
|---|---|---|---|---|---|---|---|---|
| Button h xs/sm/default/lg | 24/32/36/40 | 24/28/32/36 | 24/32/36/40 | 24/28/32/36 | 20/24/28/32 | 24/32/36/40 | 28/36/40/44 | 24/28/32/36 |
| Button px (default) | 10 | 10 | 12 | 10 | 8 | 12 | 24 | 12 |
| Button radius | md 8 | lg 10 | 4xl pill | 0 | md 8 | 4xl pill | 0 | 2xl 18 |
| Button text | 14/500 | 14/500 | 14/500 | 12/500 | 12/500 | 14/500 | 12/600 upper, 0.1em | 14/500 |
| Input h / px | 36 / 10 | 32 / 10 | 36 / 12 | 32 / 10 | 28 / 8 | 36 / 12 | 40 / 0 | 32 / 10 |
| Input skin | border, transparent, shadow-xs | border, transparent | border + `bg-input/30`, pill | border, square | border + `bg-input/20` | no border, `bg-input/50`, 3xl | bottom border only | no border, `bg-input/50`, 2xl |
| Card radius / pad | xl / 24 | xl / 16 | 2xl / 24 | 0 / 16 | lg / 16 | 4xl / 24 | 0 / 32 | min(4xl,24) / 20 |
| Card elevation | ring 10% + shadow-xs | ring 10% | ring 10% | ring 10% | ring 10% | ring 5% + shadow-md | ring 5% + shadow-sm | ring 5% + shadow-sm |
| Dialog pad / radius / max-w | 24 / xl / 448 | 16 / xl / 384 | 24 / 4xl / 448 | 16 / 0 / 384 | 16 / xl / 384 | 24 / 4xl / 448 (shadow-xl) | 24 / 0 / 448 | 24 / min(4xl,24) / 448 |
| Menu item | px8 py6 sm | px6 py4 md | px12 py8 xl | px8 py8 sq | px8 py4 md, 12px | px12 py8 2xl, 500 | px12 py8 sq, 12px upper | px8 py6 xl |
| Tabs list h | 36 | 32 | 36 pill | 32 | 32 | 36 pill, p 4 | 40 | 32 |

### Inferences
- The 2026 "classic" look differs from the widely-copied 2025 new-york look in ways a model should know: horizontal button padding dropped from 16px to 10px; destructive is a 10% tint not a solid red; cards/menus/dialogs use a 1px 10%-foreground ring instead of a solid `border`; overlays are a 10% black scrim with slight blur; hover on outline/ghost uses `muted`, not `accent`; primary hover is 80% not 90%.
- Row heights derive from padding, so a menu item is 32px (Vega) / 28px (Nova); sidebar rows are 32px; table rows about 37px (20px line + 16px padding + 1px border).
- Every popover-like surface shares one recipe: `bg-popover` + radius md/lg + `shadow-md` + 1px ring at 10% foreground + 4px offset + 4px inner padding.

### Gaps
- Radix and React Aria variants were not diffed against Base UI; the style CSS is shared so visuals should match, but default props (e.g. tooltip delay) may differ.
- Chart, Carousel, Resizable, Input OTP, Collapsible, Aspect Ratio, Navigation Menu details only partially read. Combobox, Drawer and Command internals read from style CSS only.
- Animation timings were deliberately skipped (static design target); overlays use 100ms fade+zoom-95.
- The Lyra preset description says "Tabler" while its `iconLibrary` field is `phosphor`; unresolved conflict in [config.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/config.ts).

## 4. Shadow, border and ring conventions

### Takeaway
Elevation is minimal and systematic: controls get `shadow-xs` (Vega) or none (Nova), floating surfaces get `shadow-md` plus a hairline ring, and state is communicated with a 3px translucent ring and border-colour change, never with offset rings or glows.

### Cited Findings (all [vega.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-vega.css) unless noted)
- Focus ring: `focus-visible:border-ring` + `focus-visible:ring-3` (3px) in `ring-ring/50`; no ring offset. In neutral light that is a #a1a1a1 border and a 3px halo of #a1a1a1 at 50%. Sidebar buttons use a 2px `sidebar-ring`. Slider thumb uses a 4px ring.
- Invalid: `aria-invalid:border-destructive` + `ring-3 ring-destructive/20`; dark: border `destructive/50`, ring `destructive/40`.
- Disabled: `opacity-50`, pointer events off; labels of disabled controls also 50%.
- Dark-mode control surface: `dark:bg-input/30` on inputs, textarea, select trigger, checkbox, radio, outline button, input group (white 15% x 30% = about 4.5% white over the background); hover `dark:bg-input/50`. Switch unchecked track `dark:bg-input/80`.
- Borders: 1px everywhere. `border-input` for form controls, `border-border` for layout (table rows, separators, accordion, outline button in light). Surfaces (card, dialog, popover, menus, select content, hover card, navigation popup) use `ring-1 ring-foreground/10` instead of a border; Maia/Luma/Rhea lower it to 5% in light.
- Shadows by role: `shadow-xs` = inputs, select trigger, checkbox, switch, outline button/toggle, menubar, card (Vega), button-group text; `shadow-sm` = active tab, slider thumb, floating/inset sidebar; `shadow-md` = popover, dropdown, select, combobox, hover card; `shadow-lg` = sub-menus, sheet, toast; `shadow-xl` = chart tooltip; `shadow-2xl` = Maia menus/popovers. Dialogs have no shadow in Vega/Nova (the scrim separates them); Luma/Rhea add `shadow-xl`.
- Tailwind v4 default shadow values for conversion (standard Tailwind, not re-fetched): xs `0 1px 2px 0 rgb(0 0 0 / 0.05)`; sm `0 1px 3px 0 /0.1, 0 1px 2px -1px /0.1`; md `0 4px 6px -1px /0.1, 0 2px 4px -2px /0.1`; lg `0 10px 15px -3px /0.1, 0 4px 6px -4px /0.1`; xl `0 20px 25px -5px /0.1, 0 8px 10px -6px /0.1`; 2xl `0 25px 50px -12px /0.25`. - [Tailwind box-shadow docs](https://tailwindcss.com/docs/box-shadow)
- Translucency recipes: hover of solid fills via opacity (`bg-primary/80`), tints via `/10` and `/20`, row hover `bg-muted/50`, footer bands `bg-muted/50`, selected choice card `bg-primary/5 border-primary/30`.
- Scrims: `bg-black/10` + `backdrop-blur-xs` (Vega/Nova/Lyra); 80% in Maia/Mira; 30% Luma/Rhea; 20% Sera.
- Optional `cursor: pointer` on buttons is a project setup flag (`--pointer`), off by default. - [2026-04-pointer-cursor changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-04-pointer-cursor.mdx)

### Inferences
- For a static renderer: focus state = recolour the 1px stroke to `ring` and add a 3px outside stroke of `ring` at 50% alpha; invalid = same with destructive at 20%.
- Because surfaces use an alpha ring rather than a border, the hairline adapts to any surface colour; drawing it as an inside/outside 1px stroke of foreground at 10% reproduces it.

### Gaps
- Whether the 1px ring is rendered inside or outside the box matters at pixel level: CSS `ring-1` is an outside box-shadow, so the surface is its stated size and the ring adds 1px around it.

## 5. Blocks and page-level composition

### Takeaway
Blocks are assembled from the same primitives on a 4px grid with a small set of repeated gaps (8, 16, 24px) and container widths (320/384/448px forms, 256px sidebar, 48-64px headers).

### Cited Findings
- Block inventory (Base UI and Radix): dashboard-01, login-01..05, signup-01..05, sidebar-01..16. - [bases/base/blocks](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/blocks); [2026-02-blocks changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-02-blocks.mdx)
- dashboard-01: sidebar `variant="inset"`, `collapsible="offcanvas"`, `--sidebar-width: 288px` (spacing x 72), `--header-height: 48px` (spacing x 12). Header: h 48, bottom border, inner `px-4` (24px from `lg`), gap 4 (8 from `lg`), sidebar trigger with -4px left margin, vertical separator h 16 with 8px side margins, title `text-base font-medium`. Main: column gap 8, then sections `gap-4 py-4` (24px from `md`), section side padding 16 (24 from `lg`). KPI cards: grid 1 column, 2 from container `@xl`, 4 from `@5xl`, gap 16; each card has a subtle gradient `from-primary/5 to-card` and `shadow-xs`; value `text-2xl font-semibold tabular-nums` (30px when the card is wider than 250px); trend shown as an outline Badge in the CardAction slot; footer column gap 6, 14px, first line `font-medium` with a 16px icon, second line muted. Sidebar brand row: 20px icon, `text-base font-semibold`. User row: lg menu button (48px), 32px `rounded-lg` avatar, name `text-sm font-medium` truncated, email 12px muted, trailing 16px icon; its dropdown min-width 224px. - [dashboard-01](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/blocks/dashboard-01)
- sidebar-07/08 shell: header h 64 (48 when the sidebar is icon-collapsed), `px-4`, gap 8, trigger + vertical separator (h 16, 8px right margin) + breadcrumb; content `p-4 pt-0 gap-4`; placeholder grid 3 columns gap 16 of `aspect-video rounded-xl bg-muted/50` tiles above a full-height `rounded-xl bg-muted/50` panel. - [sidebar-07](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/sidebar-07/page.tsx)
- login-01: page `min-h-svh` centred, padding 24 (40 from `md`), container `max-w-sm` (384px); one Card with header (title + description), then `FieldGroup` of Fields: Email; Password with a right-aligned "Forgot your password?" link (`text-sm`, underline on hover, offset 4); a Field holding a primary "Login" button and an outline "Login with Google" button stacked full width; centred sign-up line as a field description. - [login-01](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/blocks/login-01)
- login-02: two-column grid from `lg`; left column `p-6 md:p-10` gap 16 with a brand row (24px `rounded-md bg-primary` tile holding a 16px icon, `font-medium` name) and the form centred at `max-w-xs` (320px); right column `bg-muted` cover image (darkened to 20% brightness and greyscale in dark mode). - [login-02](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/blocks/login-02/page.tsx)
- login-03: `bg-muted` page, column gap 24, brand row above a `max-w-sm` card with centred `text-xl` title, social buttons, "Or continue with" field separator, then fields. login-04: `bg-muted` page, card `max-w-4xl` (896px) with `p-0`, split 2 columns from `md`: form side `p-6 md:p-8` with centred `text-2xl font-bold` heading and muted subtitle, a 3-column grid (gap 16) of outline social buttons, and an image side. - [login-03](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/blocks/login-03); [login-04](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/blocks/login-04)
- Composition guidance from the official agent skill: "Settings page = Tabs + Card + form controls. Dashboard = Sidebar + Card + Chart + Table." - [SKILL.md](https://github.com/shadcn-ui/ui/blob/main/skills/shadcn/SKILL.md)

### Inferences
- Spacing rhythm to encode: 4 (icon-text tight), 6 (button icon gap), 8 (label-to-inline, header gaps, footer buttons), 12 (label-control-help inside a field, radio list), 16 (page gutters, card grid gap, popover padding), 24 (card padding, dialog padding, form sections, page padding), 28 (between fields in Vega), 40 (desktop page padding on auth screens).
- Responsive widening is by step (16 to 24px gutters at `lg`; 24 to 40px page padding at `md`), and card grids use container queries, so a fixed-size artboard should pick the desktop values.

### Gaps
- Chart and data-table sections of dashboard-01, signup blocks, sidebar-01..06 and 09..16, and login-05 were not read.

## 6. What makes shadcn output look "designed"

### Takeaway
Restraint: one neutral ramp, one accent role (primary, which is near-black/near-white), two weights, a 4px grid, hairline borders, tiny shadows, 16px stroke icons, and strict reuse of composed parts instead of ad-hoc boxes.

### Cited Findings
- Icons: default library Lucide; every component sizes un-sized icons to 16px (`[&_svg:not([class*='size-'])]:size-4`), 12px in xs buttons/badges/kbd, 14px in Nova sm buttons, breadcrumb separators and checkbox ticks, 24px in Empty media, 32px in alert-dialog media. Alternative libraries selectable per preset: Tabler, Hugeicons, Phosphor, Remix (each component maps one icon name per library). - [vega.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-vega.css); [spinner.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/spinner.tsx)
- Icons used structurally: Check (checkbox, select, menu items), ChevronDown/ChevronUp (select, accordion), ChevronRight (submenu, breadcrumb, pagination next), ChevronLeft, MoreHorizontal (ellipsis), X (dialog/sheet close), PanelLeft (sidebar trigger), Loader2 (spinner), CircleCheck/Info/TriangleAlert/OctagonX (toasts). - [bases/base/ui](https://github.com/shadcn-ui/ui/tree/main/apps/v4/registry/bases/base/ui)
- Official always-enforced rules given to agents: use semantic colours, "never raw values like `bg-blue-500`"; "`className` for layout, not styling. Never override component colors or typography"; no `space-x/y`, use flex + `gap-*`; `size-*` when width equals height; no manual `dark:` colour overrides; forms use `FieldGroup` + `Field`; option sets of 2-7 choices use `ToggleGroup`; "Use full Card composition", don't dump everything in `CardContent`; callouts use `Alert`; empty states use `Empty`; use `Separator`, `Skeleton`, `Badge` rather than custom markup; icons in buttons carry `data-icon` and get no sizing classes; items always sit inside their group; dialogs/sheets always have a title; avatars always have a fallback; status colours come from Badge variants or semantic tokens, not raw Tailwind colours. - [SKILL.md](https://github.com/shadcn-ui/ui/blob/main/skills/shadcn/SKILL.md); [rules/styling.md](https://github.com/shadcn-ui/ui/blob/main/skills/shadcn/rules/styling.md)
- Customisation order of preference: built-in variants, then semantic tokens, then new CSS variables. - [rules/styling.md](https://github.com/shadcn-ui/ui/blob/main/skills/shadcn/rules/styling.md)
- shadcn's own diagnosis of sameness: "somewhere along the way, all apps started looking the same. I guess the defaults were a little too good", which motivated styles that change geometry and spacing, "beyond theming". - [2025-12-shadcn-create changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2025-12-shadcn-create.mdx)
- Colour restraint in numbers: in neutral, secondary = muted = accent (one grey, 0.97 light / 0.269 dark), border = input, and only `destructive` (and dark `sidebar-primary`) has chroma. - [themes.ts](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/themes.ts)
- Text refinement utilities applied by default: `text-balance` on descriptions/empty states, `tabular-nums` on numeric values, `truncate` on names, `line-clamp-1/2` on item titles/descriptions. - [vega.css](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/styles/style-vega.css); [item.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/item.tsx)

### Inferences
- Lucide's default 2px stroke on a 24px grid, rendered at 16px, gives about a 1.33px visual stroke; shadcn does not override stroke width (I saw no `stroke-` overrides in the component sources), so a drawing tool should keep Lucide defaults and scale.
- Concentric radii: inner elements are one or two steps smaller than their container (menu item sm 6px inside menu md 8px with 4px padding; tab trigger md inside list lg with 3px padding; card xl 14px holding md 8px controls).
- Emphasis budget: at most one solid primary button per view; secondary actions are outline or ghost; destructive is a tint. Colour appears through content (charts, avatars, images), not chrome.

### Gaps
- Lucide stroke width is not stated in shadcn sources; the 2px default is Lucide's own convention and was not re-verified this session.
- No official written "design principles" page was found; the rules above are the agent-skill rules plus what the source encodes.

## 7. Radix, Base UI, React Aria; registry, CLI, MCP and AI consumption

### Takeaway
In 2026 shadcn/ui is a three-base system (Base UI default since July 2026, Radix fully supported, React Aria added July 2026) with one shared visual layer, distributed through a registry/CLI that is explicitly designed for coding agents (skills, presets, MCP, `docs` command, composition trees).

### Cited Findings
- "Starting today, Base UI is the default component library in shadcn/ui" (2026-07-02); reasons given include Base UI being stable "at 1.6.0 with 6M+ weekly downloads"; every component was rebuilt for Base UI with "the same abstraction"; Radix remains supported. - [2026-07-base-ui-default changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-07-base-ui-default.mdx)
- React Aria became "a first-class component base" on 2026-07-17 (`--base aria`), across all eight styles. - [2026-07-react-aria changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-07-react-aria.mdx)
- Timeline: Dec 2025 `npx shadcn create` (pick library, icons, base colour, theme, fonts; 5 styles; "rewrites the component code to match your setup"); Jan 2026 Base UI docs, RTL, logical inline-start/end styles; Feb 2026 unified `radix-ui` package and blocks for both bases; Mar 2026 CLI v4 and Luma; Apr 2026 Sera, `shadcn apply`/presets, composition docs, pointer cursor; May 2026 Rhea, `shadcn eject`, registry include/validate; Jun 2026 chat components, GitHub registries; Jul 2026 Base UI default, typeset, `@shadcn/helpers`, React Aria, Toast, dynamic search; Aug 2026 Questionnaire, private GitHub registries; Sep 2026 standalone `cn` package (`shadcn migrate cn`); Oct 2026 Trace, Code Block, AI Elements. - [changelog](https://ui.shadcn.com/docs/changelog); [changelog directory](https://github.com/shadcn-ui/ui/tree/main/apps/v4/content/docs/changelog)
- Architecture of the visual layer: base components carry semantic class hooks (`cn-button`, `cn-button-variant-outline`, `cn-button-size-sm`) plus `data-slot` attributes; each style is one CSS file mapping those hooks to Tailwind utilities; the CLI compiles the chosen style into the component source. State variants are normalised across libraries (`data-open`, `data-checked`, `data-active`, `data-horizontal` custom variants). - [button.tsx](https://github.com/shadcn-ui/ui/blob/main/apps/v4/registry/bases/base/ui/button.tsx); [packages/shadcn/src/tailwind.css](https://github.com/shadcn-ui/ui/blob/main/packages/shadcn/src/tailwind.css)
- Presets: "A preset packs your entire design system config into a short code. Colors, theme, icon library, fonts, radius"; usable as `--preset <code>` and intended to be dropped into prompts "across Claude, Codex, v0, Replit". - [2026-03-cli-v4 changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-03-cli-v4.mdx)
- shadcn/skills (CLI v4): gives agents project context via `shadcn info --json` (framework, Tailwind version, aliases, base library, icon library, installed components), CLI reference (`init`, `add`, `search`, `view`, `docs`, `diff`, `info`, `build`), theming and registry authoring knowledge; installed with `pnpm dlx skills add shadcn/ui`. - [skills docs](https://ui.shadcn.com/docs/skills); [SKILL.md](https://github.com/shadcn-ui/ui/blob/main/skills/shadcn/SKILL.md)
- Composition sections in docs exist because "LLMs and coding agents compose elements more reliably when they can see the full structure". - [2026-04-component-composition changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2026-04-component-composition.mdx)
- MCP server: started with `npx shadcn@latest mcp` (setup helper `pnpm dlx shadcn@latest mcp init --client claude`); lets assistants browse, search and install items from the shadcn registry and any namespaced/private registry declared in `components.json` under `registries` (`"@acme": "https://registry.acme.com/{name}.json"`, optional auth headers); supported clients documented: Claude Code, Cursor, VS Code, Codex. - [MCP docs](https://ui.shadcn.com/docs/mcp)
- CLI 3.0 (Aug 2025) introduced namespaced registries (`@registry/name`), private registries, search/view commands and the MCP server for all registries; a registry index/directory followed in Sep-Oct 2025. - [2025-08-cli-3-mcp changelog](https://github.com/shadcn-ui/ui/blob/main/apps/v4/content/docs/changelog/2025-08-cli-3-mcp.mdx)
- The skill description also covers "building a design system or applying a DESIGN.md or brand spec to shadcn/ui". - [SKILL.md](https://github.com/shadcn-ui/ui/blob/main/skills/shadcn/SKILL.md)

### Inferences
- The slot + semantic-class + style-file design is directly portable to a canvas tool: define each component once as a tree of named slots, and express a "style" as a table of per-slot properties (height, padding, gap, radius, fill token, stroke token, shadow, text style). The eight style files are effectively eight such tables for about 440 slots.
- A "preset" (base colour + accent theme + style + radius + font + icon library) is the minimal state an AI assistant needs to draw consistently; Pencraft could expose the same six knobs.
- Behavioural library choice (Radix vs Base UI vs React Aria) is irrelevant for static drawing; only the style and theme matter.

### Gaps
- MCP tool names are not listed on the docs page; `skills/shadcn/mcp.md` in the repo likely has them but was not read.
- `design-system.md` and `design-system-page.md` in the skill folder (guidance for turning a brand spec into a shadcn theme) were not read and are probably relevant to Pencraft.
- Usage/adoption statistics for each style or base were not found.
