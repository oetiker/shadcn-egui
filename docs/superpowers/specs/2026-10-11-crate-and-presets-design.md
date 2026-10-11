# shadcn-egui 0.4.0 — a published crate with shadcn presets

Date: 2026-10-11
Status: design approved in conversation (2026-10-11); revised after two
pushback reviews (2026-10-11); amended 2026-10-11 after spike P0a: preset
codes follow ui.shadcn.com (React) only (D13)

## 1. Goal

Up to 0.3.2, egui-shadcn reached projects as code that a Claude Code plugin
vendored into them. The unreleased 0.4 work made the look a parameter
(Theme = Style × Palette × radius, styles generated from upstream CSS).

This spec turns the code into a library on crates.io and completes the
parameter set so that it matches a shadcn **preset**: every choice the shadcn
"create" page offers (style, base colour, theme, chart colour, font, heading
font, icon library, radius, menu colour, menu accent) has an egui equivalent,
and a preset code copied from the shadcn site configures an egui app the same
way.

Success:

- `cargo add shadcn-egui` and `Preset::from_code("b…")?.apply(ctx, Mode::Dark)`
  render the 14 existing components in that preset's style, colours, font,
  icons and radius.
- Every preset code ui.shadcn.com produces decodes (D13). Choices the egui side
  cannot honour (style `sera`, a font or icon library whose feature is off)
  fall back to a documented default and log a warning (§4.1).
- Medium and Semibold text render at their real weight.
- A release is one tag; CI publishes all crates of the workspace.

This is spec 1+2 of four. Later specs: (3) port the remaining ~47 components,
(4) rewrite the plugin skill to teach Claude to use the crate.

## 2. Decisions (settled, do not reopen)

| # | Decision | Notes |
|---|----------|-------|
| D1 | Vendor mode is dropped. The crate is a normal dependency. | `skills/egui-shadcn/registry/` is deleted; the byte-identical rule ends. |
| D2 | Crate name `shadcn-egui`; GitHub repo renamed `oetiker/egui-shadcn` → `oetiker/shadcn-egui`. | crates.io `egui-shadcn`/`egui_shadcn` is taken by FerrisMind/shadcn-rs. We do not adopt that crate (no generated styles, no a11y, no tests); its MIT code may serve as a behaviour reference for spec 3. |
| D3 | One repo, one Cargo workspace, lockstep versions, one CI publishes all. Each release tracks the latest egui; MSRV follows egui's MSRV. | |
| D4 | First crates.io release is **0.4.0**. The version line continues from 0.3.2; CHANGES.md stays one history. 0.4 is never released as vendored code. | |
| D5 | `Preset` is plain data and resolves to a `Theme`. Components keep reading `Theme`. Light/dark (`Mode`) is not part of the preset. | |
| D6 | Colour sources: generated palettes (base colour × theme), chart colours, hand-built `Palette`. No CSS paste import. | |
| D7 | Translucent menu colours draw opaque (their default/inverted variant) with a warning; egui has no backdrop blur and alpha without blur was rejected (user, 2026-10-11). | Documented in `references/gotchas.md`. |
| D8 | Inter is the bundled default font (upstream `DEFAULT_CONFIG`). `fontHeading` is supported. Oxanium moves to its own font crate. | |
| D9 | Font and icon crates connect through **Cargo features** on the main crate (approach A). | Rejected: runtime trait registration (B), link-time registry via `inventory`/`linkme` (C, adds a runtime dependency). |
| D10 | Publishing follows the repo-infra standard, plus a CI publish step on a release tag. | Set up with `repo-infra:apply`. |
| D11 | Until spec 4 ships, the marketplace plugin is pinned to `v0.3.2`. | Spec 4 removes the pin. |
| D12 | Variable fonts are used as they are, via egui's `FontTweak::coords`. No static instancing. | §6.1. |
| D13 | Preset codes follow **ui.shadcn.com (React, repo shadcn-ui/ui) only**. Codes from shadcn-svelte.com are not supported. (User decision 2026-10-11, after spike P0a.) | P0a found the two `preset.ts` differ: React has `gray` at index 3 of the base colours and themes, and orders icon libraries lucide, hugeicons, tabler, phosphor, remixicon; svelte has no `gray` and swaps hugeicons/tabler. So the same code names different presets on the two sites. `BaseColor` and `ThemeColor` gain `Gray` at React's index. `a` codes take `chart_color = V1_CHART_COLOR_MAP[theme] ?? theme` (React site and CLI rule). Upstream for preset lists, palettes and fonts is a second pinned checkout of shadcn-ui/ui (`tools/stylegen/PIN_REACT`); style CSS stays on the shadcn-svelte pin (identical files at both pins). |
| D14 | Remixicon ships pinned to its **last Apache-2.0 release** and is never updated. | Remix Icon License v1.0 (Jan 2026) forbids an icon-pack crate. §6.3. |
| D15 | 0.4.0 ships the full scope (all fonts, all icon sets); no split. | The publish job handles the crates.io new-crate rate limit. §9. |
| D17 | Card and Dialog titles use the heading font, as upstream (`cn-font-heading`). | §6.1. Spec 3 does the same for AlertDialog, Sheet, Drawer, Empty. |
| D18 | Icon data is stored as a compact binary blob per crate, after measuring the generated size. | §6.2. |
| D19 | `theme` and `chart_color` merge onto any base colour, as upstream `buildRegistryTheme`; no fallback. | The picker limit (`getThemesForBaseColor`) is site UI only. |
| D20 | Styles Lyra and Sera force radius 0, and style Rhea turns radius Large into the base colour's radius, as the shadcn site does. | §4. |
| D16 | The in-repo marketplace (`.claude-plugin/marketplace.json`, `"source": "."`) stays as it is until spec 4. Accepted: its users get a skill that points at the deleted `registry/` until then. | `plugin.json` / `marketplace.json` keep `0.3.2` until spec 4; the crate versions move on alone. |

## 3. Workspace layout

```
shadcn-egui/
├─ Cargo.toml                     [workspace]; shared version, egui = "0.34"
├─ crates/
│  ├─ shadcn-egui/                theme, style, palette, preset, components,
│  │                              examples/, tests/ (moved here)
│  ├─ shadcn-egui-core/           leaf crate: Icon, Seg, IconSlots, FontFiles
│  ├─ shadcn-egui-font-inter/     normal (non-optional) dependency
│  ├─ shadcn-egui-font-<name>/    one per upstream font, optional
│  ├─ shadcn-egui-icons-lucide/   normal (non-optional) dependency
│  └─ shadcn-egui-icons-<lib>/    tabler, phosphor, remixicon†, hugeicons*
├─ tools/stylegen, tools/icongen  own workspaces, never dependencies
└─ skills/egui-shadcn/            SKILL.md + references (registry/ deleted)
```

\* hugeicons only if its licence allows republishing (§6.3).
† remixicon pinned to its last Apache-2.0 release (D14).

- `shadcn-egui-core` exists because font and icon crates must not depend on
  `shadcn-egui` (Cargo forbids dependency cycles, also through optional
  dependencies). It holds data types only and depends on nothing but egui.
  `shadcn-egui` re-exports everything in it; users never name it.
- Features of `shadcn-egui`: `font-<name>` for each optional font,
  `icons-<lib>` for each optional icon library, `fonts-all`, `icons-all`.
  Feature names use the upstream ids (`font-jetbrains-mono`, `icons-tabler`).
- Publish order: `core` → font and icon crates → `shadcn-egui`.

## 4. `Preset`

Upstream reference (shadcn-ui/ui, pin `c2a67849`): `packages/registry/src/preset/preset.ts`
(lists, codes), `apps/v4/registry/config.ts` (`RADII`, `DEFAULT_CONFIG`).

```rust
pub struct Preset {
    pub style: StyleName,          // Nova Vega Maia Lyra Mira Luma Sera Rhea
    pub base_color: BaseColor,     // Neutral Stone Zinc Gray Mauve Olive Mist Taupe
    pub theme: ThemeColor,         // 25 values (incl. Gray)
    pub chart_color: ThemeColor,
    pub font: FontName,            // 26 values
    pub font_heading: FontHeading, // Inherit | Font(FontName)
    pub icons: IconLibrary,        // Lucide Hugeicons Tabler Phosphor Remixicon
    pub radius: RadiusChoice,      // Default None Small Medium Large | Px(f32)
    pub menu_color: MenuColor,     // Default Inverted DefaultTranslucent InvertedTranslucent
    pub menu_accent: MenuAccent,   // Subtle Bold
}

pub enum Mode { Light, Dark }

impl Preset {
    pub fn from_code(code: &str) -> Result<Self, PresetCodeError>;
    pub fn to_code(&self) -> Option<String>;   // None when radius is Px(_)
    pub fn theme(&self, mode: Mode) -> Theme;  // pure
    pub fn apply(&self, ctx: &egui::Context, mode: Mode); // = self.theme(mode).apply(ctx)
    // builder: one by-value setter per field, e.g. .style(StyleName::Nova)
}
impl Default for Preset; // upstream DEFAULT_CONFIG (React apps/v4/registry/config.ts:173)
```

- **Enum order is the upstream list order.** The enums are generated by
  stylegen from `preset.ts`, so `from_code`/`to_code` are a direct port of
  `decodePreset`/`encodePreset`: version letter `a` (40 bits) or `b` (51 bits,
  adds `chartColor` and `fontHeading`), base62 payload, fields packed in
  `PRESET_FIELDS_V2` order.
- Decoding follows upstream exactly: an index past the end of a list decodes
  to index 0; `a` codes get `fontHeading = Inherit` and, as the React site
  and CLI do after `decodePreset` (`preset-query.ts`, `commands/preset.ts`),
  `chartColor = V1_CHART_COLOR_MAP[theme] ?? theme` (keyed by the theme;
  the map has the seven base-colour themes).
  `PresetCodeError` covers what upstream returns `null` for: empty or
  1-character code, unknown version letter, non-base62 character. It also
  rejects a payload above 2^53 (`TooLong`): upstream decodes in f64 and
  silently loses precision there; the sites never produce such codes, so we
  decode in checked `u64` and refuse instead of guessing.
- `to_code` always writes version `b`.
- `Preset::default()` is ui.shadcn.com's `DEFAULT_CONFIG`: **Nova**,
  neutral/neutral, chart neutral, Lucide, Inter, heading Inherit, radius
  Default, menu default/subtle. It equals the code `b0` (all indices 0).
  (Amended 2026-10-11: the svelte site's default was Vega; the React site's
  is Nova. `Theme::current`'s fallback stays `vega_dark`.)
- `Gray` (base colour or theme) is a code value on ui.shadcn.com but has no
  palette there: `themes.ts` has no `gray` entry, the site's schema rejects
  it and its randomiser skips it. It decodes and encodes like any value;
  resolving it is a fallback (§4.1).
- A translucent menu colour (`DefaultTranslucent`, `InvertedTranslucent`)
  forces the menu accent to Subtle, as ui.shadcn.com does
  (`lib/search-params.ts:150-169,197-205`, `menu-picker.tsx:89`): the bold
  accent is dropped from the palette and from the inverted menu palette. No
  warning for the accent (site behaviour, not a fallback); `to_code` keeps
  the requested Bold. (User decision 2026-10-11.) The menu colour itself
  draws opaque, with a warning (§4.1, §5.2).
- `RadiusChoice::Default` means "keep the base colour's radius" (upstream
  `config.ts:241`), not 0.5rem. `None/Small/Medium/Large` = 0 / 0.45 / 0.625 /
  0.875 rem (React `RADII`; `default` has no rem value there). `Px(f32)` is
  our extension; it has no code.
- Styles Lyra and Sera force radius 0 whatever `radius` says, as the site
  does (`design-system-provider.svelte:37,44`). For Sera this still holds
  after the fall back to Nova, so the result is as close to the request as
  possible.
- Style Rhea with radius `Large` resolves as `Default` (the base colour's
  radius), silently, as the site does (`design-system-provider.svelte:49`);
  `to_code` keeps `Large`. (User decision 2026-10-11.)
- `Theme::new(style, palette, radius)`, `Palette` and `Theme::apply` stay
  public. `Preset` is a layer on top.
- `Theme` carries everything `apply` needs: resolved palette, menu colours
  (§5.2), fonts (§6.1) and icon slots (§6.2). `Theme::apply` is the **only**
  place that installs fonts and writes egui style; `Preset::apply` just calls
  it.
- Without a preset, `Theme::new(style, palette, radius)` gets Inter, heading
  Inherit, Lucide slots and the default menu colours. Builders change them:
  `.fonts(body, heading)`, `.icons(IconSlots)`, `.menu(MenuColors)`. For a
  hand-built `Palette`, `MenuColors::default()` uses the theme's own palette
  (not inverted); an inverted menu needs the caller to pass
  the dark palette in `MenuColors`.
- Runtime switching: call `apply` again (as today).

### 4.1 Fallbacks

Each fallback that applies logs one `log::warn!` line, so a preset with two
unsupported values logs two lines. The lines come from `Preset::theme()`, so
they repeat on every call (e.g. each light/dark toggle); this is accepted.
Every fallback still produces a usable theme. `log` becomes a runtime dependency (tiny, and the
usual way egui crates warn).

| Preset value | Fallback |
|--------------|----------|
| `StyleName::Sera` | Nova |
| font or heading font whose feature is off | Inter |
| icon library whose feature is off | Lucide |
| `IconLibrary::Hugeicons` when its crate does not exist (§6.3) | Lucide |
| base colour without an upstream palette (`Gray`) | Neutral |
| theme or chart colour without an upstream palette (`Gray`) | the resolved base colour |
| menu colour `DefaultTranslucent` / `InvertedTranslucent` (egui has no backdrop blur) | `Default` / `Inverted`; the subtle accent stays forced |

## 5. Colours

Upstream reference (shadcn-ui/ui): `apps/v4/registry/themes.ts`,
`apps/v4/registry/base-colors.ts`, `apps/v4/registry/config.ts:buildRegistryTheme`;
menu colours: shadcn-svelte `packages/cli/src/utils/transformers/transform-menu.ts`.
The values of the 24 shared palettes are identical in both repositories at
the pins (checked 2026-10-11).

### 5.1 Palettes

- stylegen reads `themes.ts` (bases named by `base-colors.ts`) and generates
  `palette/generated.rs`: the 7 base palettes and the 17 colour-theme
  overlays, each for light and dark, as OKLCH constants. The product
  (7 × 24) is not generated. `gray` has no palette upstream (§4).
- `Preset::theme()` merges at runtime, in upstream order, for any
  combination of base colour, theme and chart colour (D19):
  1. base colour vars;
  2. theme overlay on top (it sets `primary(-foreground)`,
     `secondary(-foreground)`, `chart-1..5`, `sidebar-primary(-foreground)`);
  3. `chart-1..5` from the `chart_color` theme;
  4. `menu_accent: Bold` → `accent = primary` (existing
     `Palette::with_bold_menu_accent`), unless the menu colour is
     translucent (§4; the menu itself draws opaque, §5.2);
  5. radius per §4.
- `Palette` gains `chart: [Color32; 5]`. Sidebar vars are not added until a
  Sidebar component exists. `destructive_foreground` stays (upstream dropped
  it; our components read it).
- `Palette::light()` / `Palette::dark()` stay and equal the generated neutral
  palette (tested).
- stylegen `drift` covers `themes.ts`, `base-colors.ts`, `preset.ts`,
  `config.ts` and `font-definitions.ts` (React pin) in addition to the style
  CSS (svelte pin).

### 5.2 Menu colours

Upstream applies `menuColor` only to menu content: Select, DropdownMenu,
ContextMenu, Menubar (and their sub-menus). Of the 14 existing components
only **Select** is affected.

- `Inverted` / `InvertedTranslucent`: menu content renders as dark, also in
  light mode. Upstream adds the `dark` class to the menu
  (`transform-menu.ts:30`), which switches both the colour vars and the
  `dark:` variants. So the menu popup draws with a derived theme: the dark
  palette **and** `dark: true`, so recipes with `State::DARK` rules
  (e.g. `src/style/nova.rs`) resolve as in dark mode.
- `*Translucent`: draws like the opaque variant (`DefaultTranslucent` as
  `Default`, `InvertedTranslucent` as `Inverted`) and logs a warning (§4.1).
  Upstream's `TRANSLUCENT_CLASSES` pair `bg-popover/70` with
  `backdrop-blur-2xl`; egui has no backdrop blur, and alpha without the blur
  was rejected (user, 2026-10-11, D7).
- `Theme` holds a resolved `menu: MenuColors` (palette, dark flag) and a
  `menu_theme()` helper that returns the derived
  theme; components never look at the preset.

## 6. Fonts and icons

### 6.1 Fonts

egui 0.34 renders text with `skrifa` and supports variable fonts:
`FontTweak::coords` (`epaint-0.34.3/src/text/fonts.rs:194`) sets variation
coordinates per registered font. Our current `install_fonts` loads the
Oxanium variable font three times without `coords`, so every weight renders
as Regular. This is the Medium/Semibold bug; it is our bug, not egui's.

- A font crate exports one `shadcn_egui_core::FontFiles` (not `FontFamily`,
  which would clash with `egui::FontFamily` under glob imports):
  `name`, `Variable { data, wght: RangeInclusive<f32> }` or
  `Static { regular, medium: Option<_>, semibold: Option<_>, bold: Option<_> }`
  (only for fonts with no variable version; a missing weight uses the nearest
  weight that exists, e.g. Instrument Serif ships 400 only),
  `y_offset_factor: f32`, and `LICENSE` text.
- `Theme` holds the resolved `fonts: ThemeFonts` (body and heading
  `&'static FontFiles`); `Theme::apply` installs them (§4).
- `install_fonts` registers the bytes once per weight (400/500/600/700)
  with `FontTweak { coords: wght=…, y_offset_factor }`. A weight outside the
  font's `wght` range is clamped. `FontData::from_static` shares the bytes.
- `y_offset_factor` per font is measured once, the way Oxanium's `0.18` was,
  and stored in the font crate.
- Families: `Proportional` = body Regular; named families
  `shadcn-body-{medium,semibold,bold}` and `shadcn-heading-{regular,medium,semibold,bold}`.
  `family_for(ctx, Weight)` stays; new `heading_family_for(ctx, Weight)`.
  `TextStyle::Heading` uses the heading font at Semibold. `Inherit` maps the
  heading families to the body font.
- Card title and Dialog title use `heading_family_for(<recipe weight>)`
  (D17); today they call `family_for` (`src/components/card.rs`,
  `src/components/dialog.rs`).
- Every named family lists egui's default fonts after its own font, so
  glyphs the font lacks (emoji, CJK, symbols) still render, as they do in
  `Proportional`.
- `Monospace` stays egui's default unless the body font is a mono font.
- App fonts: `Theme::apply` rebuilds the font definitions from egui's
  defaults, so fonts an app installed itself are lost, and an app's own
  later `set_fonts` removes our named families (then `TextStyle::Heading`
  points at a missing family and egui panics; this already happens today).
  Fix: `Theme::apply_with_fonts(ctx, base: FontDefinitions)` starts from the
  app's definitions instead of egui's defaults; `apply` documents that an
  app which manages fonts must use it.
- `Theme::apply` reinstalls fonts only when the theme's fonts changed since
  the last install on that context; a light/dark switch does not rebuild the
  atlas.
- Known egui limit: row height ignores variation coords
  (`fonts.rs:673`, TODO upstream). `wght` does not change ascent/descent, so
  this has no visible effect.
- Licences: the plan lists all 26 upstream fonts with their licence. Only
  OFL-1.1 / Apache-2.0 fonts get a crate; any other is left out and falls
  back to Inter.

### 6.2 Icon slots

- `IconSlots` (in core) names every icon a component draws: today `check`,
  `chevron_down`, `close` (dialog close); spec 3 extends it. `Theme` holds an
  `IconSlots`; components read `t.icons.check` instead of the const `CHECK`.
- Each icon crate exports `SLOTS: IconSlots` and every icon of the set as a
  `pub const` **handle**, generated by icongen.
- Storage (D18): full sets are large (Tabler ~5k, Hugeicons ~4.6k,
  Remixicon ~3k, Phosphor ~1.5k icons) and crates.io refuses a `.crate`
  above 10 MB; rustc is also slow on huge literal arrays. So each icon crate
  stores its data as one binary blob (`include_bytes!`): coordinates
  quantised to `i16` in the 24-unit box, plus an offset table. An `Icon`
  const is a name plus a range into the blob. The first icongen task in the
  plan generates Remixicon and Phosphor and measures the size before the
  format is fixed; the gates run `cargo package` on every icon crate and fail
  above 8 MB.

### 6.3 Filled icons

`components/icon.rs` paints strokes only, and egui fills only convex
polygons. Lucide, Tabler and Hugeicons are stroke sets; Phosphor (regular)
and Remixicon are filled outlines with holes.

- `Icon` gets a second form: a pre-triangulated mesh (vertices + indices in
  the 24-unit icon box). icongen triangulates filled paths at generation
  time with `lyon` (tool-only dependency). The runtime gains no dependency.
- egui anti-aliases only shapes it tessellates itself; a raw `Mesh` has hard
  edges. A stroke over the edge would overlap the fill and draw a darker
  seam whenever the colour has alpha below 1 (disabled fade, translucent
  menus). So icongen stores, for each contour, the outward vertex normals,
  and the painter adds an **outward-only** fringe strip, the way epaint
  feathers its own shapes: from the fill colour at the edge to transparent
  over one physical pixel. Zoomed snapshots of a filled icon edge at full
  and at 50% opacity check that edges are smooth and seam-free. User decision
  (2026-10-11): icons are instead rasterised on the CPU into cached coverage
  textures and drawn tinted, so overlapping strokes, caps and the fill
  boundary count once at any opacity; the fringe data stays in the blob.
- Remixicon: since January 2026 it ships under "Remix Icon License v1.0",
  which forbids distributing the icons as an icon pack or library (§3.1,
  §3.2), although npm metadata still says Apache-2.0. The crate is built from
  the **last Apache-2.0 release** (the plan finds the exact version and
  records it with the licence text in the crate) and is never updated.
- Hugeicons: the plan checks its licence first, reading the licence file in
  the package, not only the npm metadata (`@hugeicons/core-free-icons` says
  MIT). If it does not allow
  republishing the icons, the crate and feature do not exist and
  `IconLibrary::Hugeicons` falls back to Lucide.

## 7. Testing

- **Preset codes:** fixture list of codes (D13: decoded with the React
  lists; codes found on the svelte site are kept as inputs), `a` and `b`,
  with their upstream `decodePreset` output plus the React `a`-code chart
  rule; `from_code` must match;
  `to_code(from_code(c)) == c` for canonical `b` codes (as the sites write
  them; a hand-made code with out-of-range indices does not round-trip);
  each `PresetCodeError` case.
- **Palettes:** merged palettes for a set of combinations equal upstream
  `buildRegistryTheme` output (fixtures produced from upstream once, committed).
- **Fonts:** proof test first: one word at `wght` 700 has more ink than at
  400. Per font crate: a Button snapshot (checks `y_offset_factor`).
- **Icons:** one snapshot per icon crate showing its slot icons; one zoomed
  snapshot of a filled icon edge (§6.3).
- **Inverted menu:** Select with `MenuColor::Inverted` in light mode, snapshot.
- **Fallbacks:** one test per row of §4.1, checking the result and the
  warning line. `log` has one global logger per process and tests run in
  parallel threads, so the tests use a capture logger that records the
  thread id and each test reads only its own lines.
- **Heading font:** Card and Dialog snapshot with a heading font clearly
  different from the body font.
- **Lyra radius:** a Lyra preset with `radius: Large` resolves to radius 0;
  a Rhea preset with `radius: Large` resolves to the base colour's radius.
- **Existing snapshots** change (Inter replaces Oxanium; weights now differ).
  The user reviews and approves the new PNGs, as in 0.4.
- Gates, all with `--jobs 4`: `cargo test --workspace --all-features` (font
  and icon snapshots) **and** `cargo test --workspace` with default features
  (fallback tests); `cargo clippy --workspace --all-targets` warning-clean in
  both feature sets and for both tools; `stylegen drift` clean;
  `cargo publish --dry-run --workspace` (catches missing metadata and
  oversized packages).

## 8. Migration

- Imports: `egui_shadcn::…` → `shadcn_egui::…`.
- Breaking: default font is Inter; `Palette` has `chart`; components take
  icons from `Theme::icons` (the `CHECK`, `CHEVRON_DOWN` and `CLOSE` consts
  move out of `components/icon.rs` into the icon crates).
- CHANGES.md: one 0.4.0 entry for users (rename, presets, Inter, real font
  weights, breaking changes).
- oxulnk moves to 0.4 as a separate, later project.

## 9. Release sequence

1. Pin the marketplace entry in `oposs/claude-plugins` to `v0.3.2`
   (needs the user's go-ahead: other repo).
2. Rename the GitHub repo to `oetiker/shadcn-egui` (user, or `gh` after
   approval); update `origin`.
3. Implement on a branch; merge to `main`.
4. `repo-infra:apply`: CI, changelog gate, publish job. The user stores the
   crates.io token as a GitHub secret.
5. Rewrite `RELEASING.md` for the new flow.
6. Tag `v0.4.0`; CI publishes the workspace.

Package metadata: every crate gets `description`, `repository`,
`license` (SPDX, per crate: the code is MIT as today (`LICENSE`); font crates add
OFL-1.1 or Apache-2.0; icon crates ISC (Lucide), MIT (Tabler, Phosphor,
Hugeicons if allowed) or Apache-2.0 (Remixicon pin)), and an `include` list
so `tests/snapshots` and other large files stay out of the package. The
crates.io token needs the `publish-new` and `publish-update` scopes.

The first release creates about 33 new crates. crates.io limits how fast one
account may create **new** crates (a small burst, then one per several
minutes; the plan looks up the current numbers). So:

- The publish job is a small loop (not `cargo publish --workspace`, which
  stops at a version that already exists): in dependency order, for each
  crate, it checks crates.io for that version, skips it if present, else
  publishes it. A job stopped by HTTP 429 can simply be re-run until
  everything is out.
- Before tagging, ask crates.io for a raised new-crate limit (they grant this
  for workspaces), or accept that the first publish takes several re-runs.
- Later releases only publish new versions of existing crates, which have a
  much higher limit.

## 10. Out of scope

- The remaining ~47 components (spec 3).
- The in-repo marketplace and `plugin.json` (D16, spec 4).
- Skill rewrite and removal of the plugin pin (spec 4).
- Sidebar colour vars; the Sera style.
- Known unrelated bugs (offer separately): `layout::row` ~1px off-centre for
  mixed heights; Tabs list track not faded on own-flag disable; stale comment
  at `tests/theme_switch.rs:33`.
