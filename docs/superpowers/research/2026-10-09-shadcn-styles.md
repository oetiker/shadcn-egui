# shadcn "styles": how they work, and what nova changes vs new-york-v4

Evidence base: shallow clones and registry JSON fetched on 2026-10-09:
- `huntabyte/shadcn-svelte` @ `493481fa` (2026-10-05)
- `shadcn-ui/ui` @ `2d3f1cd4` (2026-10-09), `apps/v4/registry`
- `https://shadcn-svelte.com/registry/styles/{nova,vega}/<comp>.json` and the legacy `https://ui.shadcn.com/r/styles/new-york-v4/<comp>.json`

## 1. The mechanism (answers Q1 + Q4)

**A style is a class map applied to one shared set of component sources at registry build time.** It is neither a separate hand-written set of components nor a runtime CSS layer.

1. Component sources exist once ("base"): `ss/docs/src/lib/registry/ui/<comp>/*.svelte` (React: `ui/apps/v4/registry/bases/{radix,base}/ui/*`). Their class strings carry **style-invariant structural classes plus marker classes** `cn-<component>-<part>[-variant-x|-size-y]`, e.g. slider thumb:
   `"cn-slider-thumb block shrink-0 select-none disabled:pointer-events-none disabled:opacity-50"`;
   button cva: `base: "cn-button group/button inline-flex shrink-0 items-center justify-center whitespace-nowrap transition-all outline-none select-none disabled:pointer-events-none disabled:opacity-50 …"`, `variants.size.default: "cn-button-size-default"`, `variants.variant.outline: "cn-button-variant-outline"`.
2. Each style is one flat CSS file of `@apply` rules keyed by those markers: `ui/apps/v4/registry/styles/style-{vega,nova,maia,lyra,mira,luma,sera,rhea}.css` (~1750 lines, 422 `@apply` in nova), e.g.
   ```css
   .style-nova {
     .cn-slider-thumb { @apply border-ring ring-ring/50 relative size-3 rounded-full border bg-white transition-[color,box-shadow] after:absolute after:-inset-2 hover:ring-3 focus-visible:ring-3 focus-visible:outline-hidden active:ring-3; }
   ```
   shadcn-svelte copies these **byte-for-byte** from upstream (`ss/docs/src/lib/registry/styles/README.md`: "DO NOT EDIT … `pnpm pull:styles`"; `ss/docs/scripts/pull-styles.ts` fetches `https://raw.githubusercontent.com/shadcn-ui/ui/refs/heads/main/apps/v4/registry/styles/style-${style}.css`). I verified `cmp` identity for nova, vega, luma.
3. Build step `ss/docs/scripts/build-registry.ts` -> `rewriteRegistryForStyle()` parses the CSS into `Record<cnClass, applyString>` (`parseStyleCss`, subject = *last* `cn-*` ident in the selector, multiple rules concatenated) and `injectStyleClasses()` (`ss/packages/parity/src/inject-style-classes.ts`) replaces each `cn-x` marker with its `@apply` list, inserted **in front of** the base classes; `tailwind-merge` then resolves conflicts by position ("the order of each `@apply` list is semantic", README). Result: e.g. nova slider root = `"data-vertical:min-h-40 relative flex w-full …"`.
4. A few markers are *not* inlined at build time but left for the CLI (`SKIP_CN_CLASSES`): `cn-menu-target`, `cn-menu-translucent` (menuColor), `cn-font-heading` (font), `cn-logical-sides`/`cn-rtl-flip`/`*-logical` (RTL), `cn-toast`, `cn-input-otp`.

So the most faithful thing to mirror is exactly this two-layer split: **base component (structure, invariant classes) + per-style map `cn-part -> class list`**.

Upstream React confirms it is the same model (`ui/apps/v4/registry/README.md`, "The style model"): generated combinations `{base}-{style}` where base ∈ {`radix`, `base` (Base UI)} (+ `aria-nova` listed in `_legacy-styles.ts`), crossed with the 8 style CSS files. **`new-york-v4` is a "legacy source registry … authored directly and committed. It is not generated from a base/style combination."** It has no `cn-*` markers.

### Styles that exist (shadcn-svelte `PRESET_STYLES`, `ss/packages/cli/src/preset/preset.ts:17`)

| style | blurb (`docs/src/lib/registry/styles/index.ts`) | default preset font / icons (`presets.ts`) |
|---|---|---|
| vega | "The classic shadcn/ui look. Clean, neutral, and familiar." | Inter / lucide |
| nova | "Reduced padding and margins for compact layouts." | Geist / lucide |
| maia | "Soft and rounded, with generous spacing." | Figtree / hugeicons |
| lyra | "Boxy and sharp. Pairs well with mono fonts." | JetBrains Mono / phosphor |
| mira | "Compact. Made for dense interfaces." | Inter / hugeicons |
| luma | "Rounded geometry. Soft elevation. Breathable layouts." | Inter / lucide |
| sera | "Editorial and typographic." | Noto Sans + Playfair Display headings / lucide, base color taupe |
| rhea | "Like Luma but compact." | |

`new-york` / `default` no longer exist as styles in shadcn-svelte (fetching `registry/styles/new-york/*.json` -> 404). `DEFAULT_CONFIG.style = "nova"`; note `getRegistryUrl()` falls back to `"vega"` if `style` is absent.

### Other components.json parameters (`ss/packages/cli/src/utils/config/schema.ts`, `docs/src/lib/registry/config.ts:buildRegistryTheme`)

All parameters except `style` are **token/CSS-variable or icon/font changes, not per-component class changes**:

| key | values | effect |
|---|---|---|
| `tailwind.baseColor` | neutral, stone, zinc, mauve, olive, mist, taupe, slate, gray (PRESET_BASE_COLORS) | the full light/dark palette of CSS vars written to the app CSS (`registry/colors/<name>.json`) |
| `theme` (preset only, not stored in components.json) | base colors + accent themes | overrides a subset of vars (primary, ring, charts…) on top of baseColor |
| `chartColor` (preset v2) | theme keys | only `--chart-1..5` |
| `radius` (preset only) | default/none 0/small 0.45/medium 0.625/large 0.875rem (`PRESET_RADII`) | non-default sets `--radius` (default keeps the base colour's `--radius`, 0.625rem); CLI writes a **multiplicative** scale (`update-css-vars.ts:181`): `--radius-sm: calc(var(--radius)*0.6)`, md ×0.8, lg ×1, xl ×1.4, 2xl ×1.8, 3xl ×2.2, 4xl ×2.6 |
| `menuAccent` | subtle, bold | `bold`: `--accent := --primary`, `--accent-foreground := --primary-foreground` (light and dark). Pure token swap. |
| `menuColor` | default, inverted, default-translucent, inverted-translucent | CLI transform `transform-menu.ts`: `inverted` replaces `cn-menu-target` with `dark` (menu popups render with the *dark palette* inside a light app); `translucent` inlines `cn-menu-translucent` = `bg-popover/70 before:backdrop-blur-2xl before:backdrop-saturate-150 **:data-[slot$=-item]:focus:bg-foreground/10 …` Applies to select/dropdown/menubar/context-menu content. |
| `iconLibrary` | lucide, tabler, hugeicons, phosphor, remix… | rewrites icon imports (`transform-icons.ts`) |
| `font`, `fontHeading` | inter, geist, figtree, jetbrains-mono, … / inherit | adds `font-*` registry deps; `cn-font-heading` marker (card title, dialog title) gets the heading font |

## 2. Fetching a component for a given style (Q2)

- shadcn-svelte: `getRegistryUrl()` (`ss/packages/cli/src/utils/registry/index.ts:12`) = `${registry}/styles/${style ?? "vega"}` ->
  `https://shadcn-svelte.com/registry/styles/<style>/<name>.json` and `…/styles/<style>/index.json`. All 14 components × {nova, vega} returned 200.
- `https://shadcn-svelte.com/registry/<name>.json` (no `styles/`) is the **legacy pre-style registry**: old Tailwind syntax (`data-[orientation=vertical]:…`, `{#each thumbs}`), and its class strings equal new-york-v4 (slider thumb `size-4 … border-primary … shadow-sm … hover:ring-4`, track `h-1.5`, button default `h-9 px-4 py-2`, destructive solid `bg-destructive … text-white shadow-xs`, card `rounded-xl border py-6 shadow-sm`). It is effectively svelte-new-york-v4.
- Theme colors: `${registry}/colors/<theme>.json`.
- React: `https://ui.shadcn.com/r/styles/new-york-v4/<name>.json` (200) and `https://ui.shadcn.com/r/styles/{radix|base}-<style>/<name>.json` (radix-nova, base-nova, radix-vega: 200). radix-nova button default size = `"h-8 gap-1.5 px-2.5 has-data-[icon=inline-end]:pr-2 has-data-[icon=inline-start]:pl-2"`, same as svelte nova.
- Raw style maps: `https://raw.githubusercontent.com/shadcn-ui/ui/refs/heads/main/apps/v4/registry/styles/style-<style>.css`.

**Drift note:** installed nova components in an existing project can be an *older* snapshot (e.g. `[a]:hover:bg-primary/80` vs current `hover:bg-primary/80`, and `hover:bg-secondary/80` vs current `hover:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]`). Styles evolve; egui-shadcn pins one upstream commit for its style tables.

**Radius note:** projects initialised with an older CLI define only the additive scale (`--radius-md: calc(var(--radius) - 2px)`, `--radius-sm: calc(var(--radius) - 4px)`) and no `--radius-xl…4xl`; there `rounded-xl`/`rounded-4xl` resolve to Tailwind defaults (0.75rem / 2rem). Current CLI output is the multiplying scale, which egui-shadcn reproduces.

## 3. new-york-v4 vs nova, per component (Q3)

Columns: **ny4** = `ui.shadcn.com/r/styles/new-york-v4`, **nova** = base classes + `style-nova.css`. Vega is listed where it shows whether a change comes from the *new base* (vega = nova ≠ ny4) or from the *style* (vega = ny4 ≠ nova). Class: (a) numeric size, (b) colour role, (c) feature present/absent, (d) structural.

Tailwind px: h-6=24 h-7=28 h-8=32 h-9=36 h-10=40; p-2.5=10 p-4=16 p-6=24; `ring-3` == `ring-[3px]`. Radius (default --radius 0.625rem=10px, new scale): md 8, lg 10, xl 14, 4xl 26.

### Button
| part | ny4 | nova | vega | class |
|---|---|---|---|---|
| radius | `rounded-md` | `rounded-lg` | `rounded-md` | a (style) |
| border | none (outline only `border`) | `border border-transparent bg-clip-padding` always | same as nova | d (base) – keeps size identical across variants |
| press | – | `active:not-aria-[haspopup]:translate-y-px` | same | c (base) |
| size default | `h-9 px-4 py-2 has-[>svg]:px-3`, gap-2 | `h-8 gap-1.5 px-2.5`, icon-side `pl-2`/`pr-2` | `h-9 gap-1.5 px-2.5` | a |
| size sm | `h-8 gap-1.5 px-3` | `h-7 gap-1 px-2.5 text-[0.8rem] rounded-[min(var(--radius-md),12px)] svg size-3.5` | `h-8 … rounded-[min(var(--radius-md),10px)]` | a |
| size lg | `h-10 px-6` | `h-9 px-2.5` | `h-10 px-2.5` | a |
| size xs | `h-6 px-2 text-xs` | `h-6 px-2 text-xs rounded-[min(--radius-md,10px)]` | 8px cap | a |
| icon / icon-sm / icon-lg | 9 / 8 / 10 | 8 / 7 / 9 (+icon-xs 6) | 9 / 8 / 10 | a |
| default hover | `hover:bg-primary/90` | `hover:bg-primary/80` | /80 | a (alpha) |
| destructive | **solid** `bg-destructive text-white hover:/90 dark:bg-destructive/60` | **tinted** `bg-destructive/10 text-destructive hover:bg-destructive/20 dark:bg-destructive/20 dark:hover:/30` | = nova | b (base) |
| outline | `border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground` | `border-border bg-background hover:bg-muted hover:text-foreground`, no shadow, `aria-expanded:bg-muted` | nova + `shadow-xs` | b (accent→muted, base), c (shadow, style) |
| ghost | `hover:bg-accent hover:text-accent-foreground dark:hover:bg-accent/50` | `hover:bg-muted hover:text-foreground dark:hover:bg-muted/50` | = nova | b (base) |
| secondary hover | `bg-secondary/80` | `color-mix(in oklch, secondary, foreground 5%)` | = nova | b (computed colour) |
| focus ring | `ring-[3px] ring-ring/50 border-ring` | `ring-3 ring-ring/50 border-ring` | same | – |

### Badge
| part | ny4 | nova (= vega) | class |
|---|---|---|---|
| shape | `rounded-full`, no fixed height | `h-5 rounded-4xl` | a |
| padding | `px-2 py-0.5` gap-1 | same + icon-side `pl-1.5/pr-1.5`, `[&>svg]:size-3!` | a |
| default/secondary hover | `/90` | `/80` | a |
| destructive | solid `bg-destructive text-white` | tinted `bg-destructive/10 text-destructive dark:bg-destructive/20` | b |
| outline hover | `bg-accent text-accent-foreground` | `bg-muted text-muted-foreground` | b |
| new variants | – | `ghost` (`hover:bg-muted`), `link` | d |

### Card
| part | ny4 | nova | vega | class |
|---|---|---|---|---|
| outline | `border` (border token) + `shadow-sm` | `ring-1 ring-foreground/10`, no shadow, `overflow-hidden` | ring-1 ring-foreground/10 + `shadow-xs` | b+c |
| spacing | `gap-6 py-6`, parts `px-6` | `--card-spacing: 4` (16px), size=sm 3 (12px) | 6 / 4 | a (via a CSS var) |
| text | inherit | `text-sm` | `text-sm` | a |
| title | `leading-none font-semibold` | `text-base leading-snug font-medium` (+`cn-font-heading`) | `leading-normal` | a (weight/line-height) |
| header | `gap-2`, `[.border-b]:pb-6` | `gap-1 rounded-t-xl` | gap-1 | a |
| content | `px-6` | `px-(--card-spacing)` | + `flex flex-col gap-3` | d |
| footer | `flex items-center px-6 [.border-t]:pt-6` | **`bg-muted/50 rounded-b-xl border-t p-(--card-spacing)`** and card `has-data-[slot=card-footer]:pb-0` | plain, like ny4 | d (tinted footer strip that bleeds to card edge) |

### Checkbox
| ny4 | nova | vega | class |
|---|---|---|---|
| `size-4 rounded-[4px] border border-input shadow-xs`, checked `bg-primary border-primary text-primary-foreground`, `dark:bg-input/30` | same tokens/sizes, **no `shadow-xs`**, `transition-colors`; base adds hit area `after:absolute after:-inset-x-3 after:-inset-y-2` | has shadow-xs | c (shadow), d (hit-area) |
Indicator icon `size-3.5` in both.

### Dialog
| part | ny4 | nova | vega | class |
|---|---|---|---|---|
| overlay | `bg-black/50` | `bg-black/10 supports-backdrop-filter:backdrop-blur-xs` | = nova | b + c (blur) |
| panel colour | `bg-background border` | `bg-popover text-popover-foreground ring-1 ring-foreground/10` | = nova | b |
| panel shape | `rounded-lg p-6 gap-4 shadow-lg sm:max-w-lg` | `rounded-xl p-4 gap-4 text-sm sm:max-w-sm`, **no shadow** | `p-6 gap-6 sm:max-w-md` | a, c |
| close | raw button `top-4 right-4 opacity-70 rounded-xs` + ring-offset focus | `<Button variant="ghost" size="icon-sm">` at `top-2 right-2` | top-4 right-4 | d (reuses Button) |
| title | `text-lg leading-none font-semibold` | `text-base leading-none font-medium` | `leading-none font-medium` | a |
| footer | `flex-col-reverse gap-2 sm:flex-row sm:justify-end` | + **`bg-muted/50 -mx-4 -mb-4 rounded-b-xl border-t p-4`** | `gap-2` only | d (bleeding tinted strip via negative margins) |
| header | `gap-2 text-center sm:text-left` | `gap-2` | | – |

### Input
| ny4 | nova | vega | class |
|---|---|---|---|
| `h-9 px-3 py-1 rounded-md border-input bg-transparent shadow-xs`, `dark:bg-input/30`, focus `ring-[3px]` | `h-8 px-2.5 py-1 rounded-lg`, **no shadow**, `disabled:bg-input/50 dark:disabled:bg-input/80`, `file:h-6`, invalid `aria-invalid:border-destructive ring-destructive/20` | `h-9 px-2.5 rounded-md shadow-xs` | a, c, b (disabled fill) |
Both: `text-base md:text-sm` (responsive).

### Label, Separator
Identical in ny4 / nova / vega (`gap-2 text-sm leading-none font-medium`; `bg-border` 1px).

### Popover
| ny4 | nova | vega | class |
|---|---|---|---|
| `w-72 rounded-md border bg-popover p-4 shadow-md` | `w-72 ring-1 ring-foreground/10 rounded-lg p-2.5 gap-2.5 flex flex-col shadow-md text-sm`; header `gap-0.5` | `rounded-md p-4 gap-4`, header `gap-1` | b (border→ring), a |
sideOffset 4 in both.

### Select
| part | ny4 | nova | vega | class |
|---|---|---|---|---|
| trigger | `h-9 (sm h-8) px-3 py-2 gap-2 rounded-md border-input shadow-xs` | `h-8 (sm h-7, sm radius min(md,10px)) pl-2.5 pr-2 py-2 gap-1.5 rounded-lg`, no shadow | h-9/h-8, rounded-md, shadow-xs | a, c |
| content | `rounded-md border shadow-md min-w-[8rem]` | `ring-1 ring-foreground/10 rounded-lg shadow-md min-w-36` + `cn-menu-target`/`cn-menu-translucent` hooks | rounded-md | b, a |
| item | `gap-2 rounded-sm py-1.5 pr-8 pl-2`, focus `bg-accent text-accent-foreground` | `gap-1.5 rounded-md py-1 pr-8 pl-1.5`, same tokens | like ny4 | a |
| label | `px-2 py-1.5 text-xs text-muted-foreground` | `px-1.5 py-1` | like ny4 | a |
| check indicator | `size-3.5` | `size-4` | size-4 | a |

### Switch
| ny4 | nova | vega | class |
|---|---|---|---|
| `h-[1.15rem] w-8` (sm `h-3.5 w-6`), `shadow-xs`, checked `bg-primary`, unchecked `bg-input dark:bg-input/80`, thumb `bg-background size-4` (dark: unchecked `bg-foreground`, checked `bg-primary-foreground`) | identical sizes (`h-[18.4px] w-[32px]`, sm 14×24) and tokens, **no `shadow-xs`**; base adds hit area `after:-inset-x-3 after:-inset-y-2` | has shadow-xs | c, d |

### Tabs
| part | ny4 | nova | vega | class |
|---|---|---|---|---|
| list | `h-9 rounded-lg p-[3px] bg-muted text-muted-foreground` | `h-8`, rest same | h-9 | a |
| trigger | `px-2 py-1 rounded-md gap-1.5`, `text-foreground/60`, active `bg-background shadow-sm` | `px-1.5 py-0.5`, icon-side `pl-1/pr-1`; tokens same | px-2 py-1 | a |
| line variant | `gap-1 bg-transparent`, underline via `after:` | same (base) | | – |

### Tooltip
Identical colours/sizes in all three: `rounded-md bg-foreground text-background px-3 py-1.5 text-xs`, arrow `size-2.5 rotate-45 rounded-[2px] bg-foreground`, sideOffset 0. nova adds `inline-flex items-center gap-1.5`, `max-w-xs`, kbd slot styling.

### Slider
| part | ny4 | nova | vega | class |
|---|---|---|---|---|
| track | `bg-muted rounded-full h-1.5` | `h-1` | h-1.5 | a |
| range | `bg-primary` | same | | – |
| thumb | `size-4 border border-primary bg-white shadow-sm hover/focus:ring-4 ring-ring/50` | `size-3 border border-ring bg-white`, no shadow, `hover/focus/active:ring-3`, **hit area `after:absolute after:-inset-2`** | = ny4 | a, b (border-primary→border-ring), c, d |
| vertical min height | `min-h-44` | `min-h-40` | 40 | a |

### Summary of ny4 -> nova
- Roughly half of the deltas are **base changes shared by every new style** (vega has them too): tinted destructive, accent→muted hovers, `ring-1 ring-foreground/10` instead of `border` on surfaces, `bg-popover` dialog, light blurred overlay, always-present transparent button border, press nudge, Button-as-dialog-close, hit-area `after:` pseudo-elements on checkbox/switch/slider.
- The other half is the **nova style proper**: one step smaller control heights (36→32), tighter paddings (p-6→p-4, card spacing 24→16), `rounded-md`→`rounded-lg`, dropped `shadow-xs` on inputs/checkbox/switch/outline buttons, tinted footer strips on card and dialog, smaller slider.
- **vega ≠ new-york-v4**: vega is "ny4 sizes on the new base".

## 4. What the other styles do (breadth: all 8 styles diffed against vega)

Most style deltas are (a) sizes/radii/ring widths, but the styles also change colour roles and structure:
- **lyra**: `rounded-none` everywhere, `ring-1` focus rings, `text-xs` buttons/inputs, `text-xs/relaxed` bodies.
- **mira**: compact (`h-7` inputs), `ring-2 ring-ring/30`, input fill `bg-input/20`, smaller switch (16.6×28) and thumb.
- **maia / luma / rhea**: pill radii (`rounded-4xl`, `rounded-3xl`, `rounded-2xl`), **filled inputs** `bg-input/50 border-transparent` (luma/rhea) or `bg-input/30` (maia), checkbox `bg-input/90 border-transparent`; luma/rhea switch gets `border-2`, sizes `h-5 w-11`, and a **non-round pill thumb** (`h-4 w-6`, `shadow-sm`); luma slider thumb is a `w-6 h-4` pill with `ring-1 ring-black/10 shadow-md`; luma/rhea cards `ring-foreground/5 shadow-md|sm`, dialogs `shadow-xl`.
- **sera**: typographic: buttons `uppercase tracking-widest text-xs font-semibold rounded-none`; **inputs/select triggers underline-only** (`border-transparent border-b-input px-0 h-10`, focus `border-b-ring` with no ring); badges become plain text (`bg-transparent px-0 text-[0.625rem] uppercase`, default `text-foreground` not `bg-primary`); slider thumb `bg-primary border-none`.

## 5. Mapping to egui-shadcn: what is and is not a per-component parameter set

Expressible as a per-style table of per-component parameters (numbers / token refs / bools):
- heights, paddings, gaps, icon sizes, font size/weight/line-height (a);
- radius as a token reference (`md`/`lg`/`xl`/`4xl`/`none`/`min(md, Npx)`) evaluated against `--radius` and the chosen radius scale (a);
- fill/stroke/text colour role per part and state: token + alpha, e.g. `Destructive@0.10`, `Muted`, `Input@0.5`, `Foreground@0.10` (b);
- outline kind: `Border(token)` vs `Ring1(token@alpha)` vs `UnderlineBottom(token)` vs none; shadow level none/xs/sm/md/xl; focus-ring width 1/2/3 px and alpha 0.3/0.5 (b, c);
- booleans: press-nudge, tinted footer strip, backdrop blur on overlay, uppercase/tracking (c);
- hit-area inflation (px) for checkbox/switch/slider (d, but a single number).

Not cleanly a flat parameter set (needs code paths or a richer model):
1. **Dark-mode-specific overrides inside the style** (`dark:bg-input/30`, `dark:hover:bg-input/50`, `dark:border-input`, `dark:data-unchecked:bg-foreground`): each colour slot needs a light and a dark value, not just a token.
2. **Computed colours**: `hover:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]` (nova/vega secondary hover) - needs an OKLCH mix helper.
3. **Structural parts**: card/dialog footer strip that bleeds to the panel edge via negative margins + removed bottom padding (`-mx-4 -mb-4 … border-t`, `has-data-[slot=card-footer]:pb-0`); dialog close button is a `Button ghost icon-sm`; sera's underline-only inputs; luma's pill switch thumb (w≠h, translate `calc(100%-8px)`); badge "plain text" in sera. These change *what is drawn*, not just values.
4. **Context selectors**: `has-data-[icon=inline-start]:pl-2` (padding depends on whether an icon is present), `in-data-[slot=button-group]:rounded-lg`, `group-data-[size=sm]/card:text-sm`, `group-has-[:focus-visible]/field-label:ring-0`. Expressible only if the egui component knows its context (has-icon flag, in-group flag).
5. **Responsive**: `sm:max-w-sm`, `text-base md:text-sm` - breakpoint-dependent.
6. **menuColor = inverted**: a *scoped theme swap* (popup subtree rendered with the dark palette) and translucent = `bg-popover/70` + backdrop blur. Requires per-subtree theme override, not a component parameter.
7. **menuAccent = bold / baseColor / theme / radius / font**: pure token-layer changes; belong in `Theme`, not in the style table. Fonts include a separate heading font slot (`cn-font-heading` on card and dialog titles).
8. **tailwind-merge ordering**: style classes are prepended and twMerge resolves conflicts; a few base classes get overridden (e.g. nova slider `data-vertical:min-h-40` vs older base). Mirroring the resolved output (the published per-style JSON) avoids having to emulate twMerge.
9. Animations (`duration-100 zoom-in-95`, slide-in) and `transition-*` lists - probably out of scope.

Recommended source of truth for a generator: `style-<name>.css` parsed into the `cn-*` map (same algorithm as `parseStyleCss`), joined with the base component sources from shadcn-svelte (`docs/src/lib/registry/ui`), pinned to one shadcn/ui commit; or simply the resolved per-style JSON at `https://shadcn-svelte.com/registry/styles/<style>/<comp>.json`.
