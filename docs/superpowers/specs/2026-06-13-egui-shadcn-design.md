# egui-shadcn: design spec

**Date:** 2026-06-13
**Status:** Approved (brainstorming → spec)

## Problem

Building polished GUIs with Rust `egui`/`eframe` requires a lot of iterative
back-and-forth. Unlike the web ecosystem (where shadcn/ui and similar provide a
deep corpus of positive examples), there is no good body of reference egui code
that produces modern, professional-looking interfaces on the first try. The goal
is to **construct beautiful, composite GUIs** (forms, panels, dialogs with many
aligned elements), beyond styling a single button here and there.

## Key finding (grounds the whole design)

shadcn/ui has three layers, and they translate to egui very differently:

| Layer | Translates to egui? |
|---|---|
| **Design tokens**: color pairs (`X`/`X-foreground`), `--radius`, the 4px spacing grid, 14px/medium type scale, subtle shadows, 3px focus rings | **Cleanly.** Map ~1:1 onto `egui::Visuals` / `Spacing` / `Frame` / `text_styles`. |
| **Component anatomy**: Button variants, Card, Input, Tabs, Switch, etc. | **Cleanly, as helper widgets.** Each cva variant → a Rust enum → paint params. |
| **Layout**: flexbox `flex-grow`/`gap`/`justify-content`, CSS Grid, intrinsic sizing, responsive reflow | **Does NOT translate.** egui is single-pass immediate mode with no parent↔child size negotiation. Must be re-derived imperatively. |

The scarce knowledge Claude lacks is therefore **layout recipes**, not per-widget
styling. The good news: `egui_extras::StripBuilder` provides
`Size::relative(frac)` / `Size::exact` / `Size::remainder`, which is the closest
thing to flexbox `fr` / `flex-grow` distribution and is the foundation for a
usable layout layer.

A *literal* "shadcn code → egui code" transform is the wrong target: the
Tailwind/layout classes are exactly the part that doesn't carry over. The
**design language** carries; layout must be rebuilt. Screenshot and component
code are complementary inputs: code tells you *which component / variant / token*;
a screenshot tells you *layout and intent* (which you rebuild imperatively
regardless).

## Reference target / eval

A **settings form with tabs**, rendered in egui to genuine shadcn-grade quality.
The skill "works" when Claude can produce this screen without many iterations. It
serves as the end-to-end worked example and the acceptance test.

## Approach (chosen)

**Vendored, copy-paste (shadcn's own model).** A canonical egui module lives
inside the skill; the workflow copies the files it needs into the target project,
where Claude adapts them directly. This is idiomatic for both shadcn ("you own the
code") and egui (per-widget styling = editing the widget), and directly attacks
"no positive examples" by making the examples real, tested, in-repo, and editable.

Rejected alternatives:
- **Standalone crate dependency**: most reusable, but fights egui's styling model
  and shadcn's philosophy; when a needed variant isn't in the crate, it's back to
  from-scratch code.
- **Pure knowledge skill (markdown only)**: layout-by-prose is precisely what
  causes the endless iteration.

## Artifact

A skill named **`egui-shadcn`** with two parts:

1. A canonical, compiling egui source module (the "registry").
2. A workflow (`SKILL.md`) that decomposes a shadcn design and rebuilds it in
   egui using that module, layout-first.

Plus the settings-form-with-tabs reference, built entirely from the module.

### The canonical module

A small Rust workspace in this repo: a library crate (source-of-truth
components) + an example binary rendering the reference screen.

- **`theme.rs`**: design tokens. `Theme { light: Palette, dark: Palette, radius,
  spacing, type_scale }`; `Palette` = the ~27 semantic shadcn tokens as
  `Color32`. Precompute the **shadcn v4 / new-york / OKLCH** defaults into sRGB
  literals (no runtime color math). `theme.apply(ctx)` pushes everything into
  `egui::Style`/`Visuals`/`Spacing`/`text_styles` and loads the bundled
  **Oxanium** font. Ships **both light + dark**. Font selection is a single
  swappable knob (Oxanium is a geometric/display face that gives a squared, sci-fi
  flavor distinct from vanilla shadcn; type sizes/weights kept easy to nudge for
  readability at the 14px body size).
- **`layout.rs`**: the flexbox substitute (the high-value part). Ergonomic
  wrappers over `egui_extras::StripBuilder`: `row` / `vstack` with **gap,
  grow-weights, justify (start/center/end/space-between), align**;
  `form_row(label, control)` with a shared label column; `card(...)`; a reflowing
  `grid`.
- **`components/`**: themed widgets, cva variants → Rust enums. v1 set:
  **Button** (Default/Destructive/Outline/Secondary/Ghost/Link × Sm/Default/Lg/
  Icon), **Input**, **Label**, **Tabs** (raised-pill active state), **Card**,
  **Switch**, **Checkbox**, **Select** (styled `ComboBox`), **Separator**,
  **Badge**. Shared helpers: one focus-ring routine + "hover/active = opacity
  modulation," applied uniformly.
- **`examples/settings.rs`**: the reference screen.

**Deferred (YAGNI for v1):** Dialog/Popover, Table, Tooltip, menus, gradients;
add them when a real design needs them.

### The workflow (`SKILL.md`)

When asked to build an egui GUI from a shadcn design (screenshot, component code,
or description):

1. **Vendor-in** the module idempotently (copy needed files; add `egui` /
   `eframe` / `egui_extras` deps).
2. **Decompose the design** top-down: app shell → per-region layout *intent* →
   components + variants → token deviations.
3. **Map**, using three reference tables baked into the skill:
   - app shell → egui panels (`SidePanel` / `TopBottomPanel` / `CentralPanel`)
   - **layout intent → layout helper** (the flexbox-property → egui table, the
     high-value part)
   - cva variant → Rust enum; tokens → theme (customize only what differs)
4. **Build** from the vendored components; only drop to raw egui when no helper
   fits, and when that happens, add a new helper rather than inlining.
5. **Verify visually**: render and screenshot, compare to the design, iterate on
   the layout numbers.

It also carries an **"egui gotchas"** section (single-pass; no `flex-grow` across
siblings; first-frame jitter) so Claude stops fighting the model.

## Verification / eval

The settings screen is the acceptance test. Use **`egui_kittest`** (egui's
official test harness) to render the UI to a PNG headlessly for snapshot
comparison, so the eval runs without a real window.

**Verified (2026-06-13):** `egui_kittest` v0.34.3 with features `["wgpu",
"snapshot"]` renders egui UIs to PNG headlessly on the target machine via
`wgpu` over mesa (no display server needed). A throwaway spike produced a correct
800×600 RGBA render of a heading + button. This also enables the development loop:
the rendered PNG can be inspected directly to iterate on the layout visually:
the missing "web-like" feedback loop.

## Defaults (settled)

- shadcn **v4 / new-york / OKLCH** as canonical target.
- **light + dark** both shipped.
- latest stable **egui (0.34.x)** + `egui_extras`.
- bundled **Oxanium** font (OFL), kept as a swappable knob.

## Build constraints (from environment)

- Compiler and tests must use **no more than 4 cores** in parallel.
- Containers via **podman** if needed (not docker).

## Reference data (captured during research)

### shadcn v4 default tokens (OKLCH)

Light (`:root`): `--background: oklch(1 0 0)`, `--foreground: oklch(0.145 0 0)`,
`--card: oklch(1 0 0)`, `--primary: oklch(0.205 0 0)`,
`--primary-foreground: oklch(0.985 0 0)`, `--secondary/-muted/-accent: oklch(0.97
0 0)`, `--muted-foreground: oklch(0.556 0 0)`,
`--destructive: oklch(0.577 0.245 27.325)`, `--border/--input: oklch(0.922 0 0)`,
`--ring: oklch(0.708 0 0)`, `--radius: 0.625rem` (10px).

Dark (`.dark`): `--background: oklch(0.145 0 0)`, `--foreground: oklch(0.985 0
0)`, `--card/--primary(dark): oklch(0.205 0 0)`/`oklch(0.922 0 0)`,
`--secondary/-muted/-accent: oklch(0.269 0 0)`,
`--muted-foreground: oklch(0.708 0 0)`,
`--destructive: oklch(0.704 0.191 22.216)`, `--border: oklch(1 0 0 / 10%)`,
`--input: oklch(1 0 0 / 15%)` (low-alpha white hairlines), `--ring: oklch(0.556 0
0)`.

Radius scale: `sm = radius − 4px` (6), `md = radius − 2px` (8), `lg = radius`
(10), `xl = radius + 4px` (14).

### Type / spacing scale

4px spacing grid (`N × 4px`). Body text `text-sm` = 14px / line-height 20;
emphasis = `font-medium`. Control height `h-9` = 36px; card padding `p-6` = 24px;
control radius `rounded-md` = 8px.

### Key cva strings (new-york v4)

- **Button base:** `inline-flex items-center justify-center gap-2 rounded-md
  text-sm font-medium ... focus-visible:border-ring focus-visible:ring-[3px]
  focus-visible:ring-ring/50 disabled:opacity-50`. Variants: `default: bg-primary
  text-primary-foreground hover:bg-primary/90`; `outline: border bg-background
  shadow-xs hover:bg-accent`; `secondary: bg-secondary hover:bg-secondary/80`;
  `ghost: hover:bg-accent`; `link: text-primary hover:underline`. Sizes: `default
  h-9 px-4 py-2`, `sm h-8 px-3`, `lg h-10 px-6`, `icon size-9`.
- **Tabs:** list `rounded-lg bg-muted p-[3px] text-muted-foreground`; active
  trigger `bg-background text-foreground shadow-sm` (raised white pill on a muted
  track).
- **Input:** `h-9 rounded-md border border-input bg-transparent px-3 py-1 text-sm
  shadow-xs` + shared focus/invalid treatment.
- **Card:** `rounded-xl border bg-card py-6 shadow-sm`; sections `px-6`; title
  `font-semibold`; description `text-sm text-muted-foreground`.
- **Switch:** track `h-[1.15rem] w-8 rounded-full`; checked `bg-primary`,
  unchecked `bg-input`; thumb 16px round, slides.
- **Checkbox:** `size-4 rounded-[4px] border border-input`; checked `bg-primary
  text-primary-foreground`.

### egui mapping notes

- Tokens → `Visuals` / `WidgetVisuals` per state (inactive/hovered/active);
  radius → `CornerRadius`; spacing → `Spacing` + `Frame` margins; type → mapped
  `text_styles`; elevation → `Shadow`.
- Layout: `StripBuilder` `Size::relative/exact/remainder` ≈ flex distribution;
  `available_width()` + `add_sized` for manual cases; panels for the app shell.
- Hard mismatches: `flex-grow` across siblings, `gap`, `justify-content:
  space-between` over wrapped rows, CSS Grid tracks, intrinsic min/max-content,
  automatic responsive reflow. None is declarative; emulate them imperatively.
