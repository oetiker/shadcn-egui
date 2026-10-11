# shadcn-egui 0.4.0 — a published crate with shadcn presets

Date: 2026-10-11
Status: design approved in conversation (2026-10-11), awaiting spec review

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
- Every preset code the shadcn site produces decodes. Choices the egui side
  cannot honour (style `sera`, a font or icon library whose feature is off)
  fall back to a documented default and log one warning.
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
| D7 | Translucent menu colours use alpha only; egui has no backdrop blur. | Documented in `references/gotchas.md`. |
| D8 | Inter is the bundled default font (upstream `DEFAULT_CONFIG`). `fontHeading` is supported. Oxanium moves to its own font crate. | |
| D9 | Font and icon crates connect through **Cargo features** on the main crate (approach A). | Rejected: runtime trait registration (B), link-time registry via `inventory`/`linkme` (C, adds a runtime dependency). |
| D10 | Publishing follows the repo-infra standard, plus a CI publish step on a release tag. | Set up with `repo-infra:apply`. |
| D11 | Until spec 4 ships, the marketplace plugin is pinned to `v0.3.2`. | Spec 4 removes the pin. |
| D12 | Variable fonts are used as they are, via egui's `FontTweak::coords`. No static instancing. | §6.1. |

## 3. Workspace layout

```
shadcn-egui/
├─ Cargo.toml                     [workspace]; shared version, egui = "0.34"
├─ crates/
│  ├─ shadcn-egui/                theme, style, palette, preset, components,
│  │                              examples/, tests/ (moved here)
│  ├─ shadcn-egui-core/           leaf crate: Icon, Seg, IconSlots, FontFamily
│  ├─ shadcn-egui-font-inter/     normal (non-optional) dependency
│  ├─ shadcn-egui-font-<name>/    one per upstream font, optional
│  ├─ shadcn-egui-icons-lucide/   normal (non-optional) dependency
│  └─ shadcn-egui-icons-<lib>/    tabler, phosphor, remixicon, hugeicons*
├─ tools/stylegen, tools/icongen  own workspaces, never dependencies
└─ skills/egui-shadcn/            SKILL.md + references (registry/ deleted)
```

\* hugeicons only if its licence allows republishing (§6.3).

- `shadcn-egui-core` exists because font and icon crates must not depend on
  `shadcn-egui` (Cargo forbids dependency cycles, also through optional
  dependencies). It holds data types only and depends on nothing but egui.
  `shadcn-egui` re-exports everything in it; users never name it.
- Features of `shadcn-egui`: `font-<name>` for each optional font,
  `icons-<lib>` for each optional icon library, `fonts-all`, `icons-all`.
  Feature names use the upstream ids (`font-jetbrains-mono`, `icons-tabler`).
- Publish order: `core` → font and icon crates → `shadcn-egui`.

## 4. `Preset`

Upstream reference (pin 493481f): `packages/cli/src/preset/preset.ts`.

```rust
pub struct Preset {
    pub style: StyleName,          // Nova Vega Maia Lyra Mira Luma Sera Rhea
    pub base_color: BaseColor,     // Neutral Stone Zinc Mauve Olive Mist Taupe
    pub theme: ThemeColor,         // 24 values
    pub chart_color: ThemeColor,
    pub font: FontName,            // 26 values
    pub font_heading: FontHeading, // Inherit | Font(FontName)
    pub icons: IconLibrary,        // Lucide Tabler Hugeicons Phosphor Remixicon
    pub radius: RadiusChoice,      // Default None Small Medium Large | Px(f32)
    pub menu_color: MenuColor,     // Default Inverted DefaultTranslucent InvertedTranslucent
    pub menu_accent: MenuAccent,   // Subtle Bold
}

pub enum Mode { Light, Dark }

impl Preset {
    pub fn from_code(code: &str) -> Result<Self, PresetCodeError>;
    pub fn to_code(&self) -> Option<String>;   // None when radius is Px(_)
    pub fn theme(&self, mode: Mode) -> Theme;  // pure
    pub fn apply(&self, ctx: &egui::Context, mode: Mode); // theme() + fonts + icons
    // builder: one by-value setter per field, e.g. .style(StyleName::Nova)
}
impl Default for Preset; // index 0 of every upstream list
```

- **Enum order is the upstream list order.** The enums are generated by
  stylegen from `preset.ts`, so `from_code`/`to_code` are a direct port of
  `decodePreset`/`encodePreset`: version letter `a` (40 bits) or `b` (51 bits,
  adds `chartColor` and `fontHeading`), base62 payload, fields packed in
  `PRESET_FIELDS_V2` order.
- Decoding follows upstream exactly: an index past the end of a list decodes
  to index 0; `a` codes get `fontHeading = Inherit` and `chartColor` = index 0.
  `PresetCodeError` covers what upstream returns `null` for: empty or
  1-character code, unknown version letter, non-base62 character.
- `to_code` always writes version `b`.
- `RadiusChoice::Default` means "keep the base colour's radius" (upstream
  `config.ts:241`), not 0.5rem. `None/Small/Medium/Large` = 0 / 0.45 / 0.625 /
  0.875 rem. `Px(f32)` is our extension; it has no code.
- `Theme::new(style, palette, radius)`, `Palette` and `Theme::apply` stay
  public. `Preset` is a layer on top.
- Runtime switching: call `apply` again (as today).

### 4.1 Fallbacks

Every fallback logs exactly one `log::warn!` line per `apply` call and still
produces a usable theme. `log` becomes a runtime dependency (tiny, and the
usual way egui crates warn).

| Preset value | Fallback |
|--------------|----------|
| `StyleName::Sera` | Nova |
| font or heading font whose feature is off | Inter |
| icon library whose feature is off | Lucide |
| `theme` not allowed for `base_color` (not the base itself and not one of the 17 colour themes) | the base colour's own palette |

## 5. Colours

Upstream reference: `docs/src/lib/registry/themes.ts`,
`config.ts:buildRegistryTheme`, `packages/cli/src/utils/transformers/transform-menu.ts`.

### 5.1 Palettes

- stylegen reads `themes.ts` and generates `palette/generated.rs`: the 7 base
  palettes and the 17 colour-theme overlays, each for light and dark, as OKLCH
  constants. The product (7 × 18) is not generated.
- `Preset::theme()` merges at runtime, in upstream order:
  1. base colour vars;
  2. theme overlay on top (it sets `primary(-foreground)`,
     `secondary(-foreground)`, `chart-1..5`, `sidebar-primary(-foreground)`);
  3. `chart-1..5` from the `chart_color` theme;
  4. `menu_accent: Bold` → `accent = primary` (existing
     `Palette::with_bold_menu_accent`);
  5. radius per §4.
- `Palette` gains `chart: [Color32; 5]`. Sidebar vars are not added until a
  Sidebar component exists. `destructive_foreground` stays (upstream dropped
  it; our components read it).
- `Palette::light()` / `Palette::dark()` stay and equal the generated neutral
  palette (tested).
- stylegen `drift` covers `themes.ts`, `preset.ts` and
  `font-definitions.ts` in addition to the style CSS.

### 5.2 Menu colours

Upstream applies `menuColor` only to menu content: Select, DropdownMenu,
ContextMenu, Menubar (and their sub-menus). Of the 14 existing components
only **Select** is affected.

- `Inverted` / `InvertedTranslucent`: menu content uses the **dark** palette
  of the same preset, also in light mode.
- `*Translucent`: menu fill = `popover` at 70% alpha; highlighted item =
  `foreground` at 10%; separator = `foreground` at 5%; destructive item text =
  `accent_foreground` (from upstream `TRANSLUCENT_CLASSES`). No blur.
- `Theme` holds a resolved `menu: MenuColors` (palette + translucent flag), so
  components never look at the preset.

## 6. Fonts and icons

### 6.1 Fonts

egui 0.34 renders text with `skrifa` and supports variable fonts:
`FontTweak::coords` (`epaint-0.34.3/src/text/fonts.rs:194`) sets variation
coordinates per registered font. Our current `install_fonts` loads the
Oxanium variable font three times without `coords`, so every weight renders
as Regular. This is the Medium/Semibold bug; it is our bug, not egui's.

- A font crate exports one `shadcn_egui_core::FontFamily`:
  `name`, `Variable { data, wght: RangeInclusive<f32> }` or
  `Static { regular, medium, semibold, bold }` (only for fonts with no
  variable version), `y_offset_factor: f32`, and `LICENSE` text.
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
- `Monospace` stays egui's default unless the body font is a mono font.
- `apply` reinstalls fonts only when `(font, font_heading)` changed since the
  last install on that context; a light/dark switch does not rebuild the atlas.
- Known egui limit: row height ignores variation coords
  (`fonts.rs:673`, TODO upstream). `wght` does not change ascent/descent, so
  this has no visible effect.
- Licences: the plan lists all 26 upstream fonts with their licence. Only
  OFL-1.1 / Apache-2.0 fonts get a crate; any other is left out and falls
  back to Inter.

### 6.2 Icon slots

- `IconSlots` (in core) names every icon a component draws: today `check`,
  `chevron_down`, `x` (dialog close); spec 3 extends it. `Theme` holds an
  `IconSlots`; components read `t.icons.check` instead of the const `CHECK`.
- Each icon crate exports `SLOTS: IconSlots` and every icon of the set as a
  `pub const`, generated by icongen.

### 6.3 Filled icons

`components/icon.rs` paints strokes only, and egui fills only convex
polygons. Lucide, Tabler and Hugeicons are stroke sets; Phosphor (regular)
and Remixicon are filled outlines with holes.

- `Icon` gets a second form: a pre-triangulated mesh (vertices + indices in
  the 24-unit icon box). icongen triangulates filled paths at generation
  time with `lyon` (tool-only dependency). The runtime gains no dependency.
- Hugeicons: the plan checks its licence first. If it does not allow
  republishing the icons, the crate and feature do not exist and
  `IconLibrary::Hugeicons` falls back to Lucide.

## 7. Testing

- **Preset codes:** fixture list of codes from the shadcn site (`a` and `b`)
  with their upstream `decodePreset` output; `from_code` must match;
  `to_code(from_code(c)) == c` for `b` codes; each `PresetCodeError` case.
- **Palettes:** merged palettes for a set of combinations equal upstream
  `buildRegistryTheme` output (fixtures produced from upstream once, committed).
- **Fonts:** proof test first: one word at `wght` 700 has more ink than at
  400. Per font crate: a Button snapshot (checks `y_offset_factor`).
- **Icons:** one snapshot per icon crate showing its slot icons; one for a
  filled icon.
- **Fallbacks:** one test per row of §4.1, checking the result and the single
  warning.
- **Existing snapshots** change (Inter replaces Oxanium; weights now differ).
  The user reviews and approves the new PNGs, as in 0.4.
- Gates: `cargo test`, `cargo clippy --all-targets` warning-clean for the
  workspace and for both tools; `stylegen drift` clean; `--jobs 4`.

## 8. Migration

- Imports: `egui_shadcn::…` → `shadcn_egui::…`.
- Breaking: default font is Inter; `Palette` has `chart`; components take
  icons from `Theme::icons` (the `CHECK`/`CHEVRON_DOWN` consts move to the
  icon crates).
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

## 10. Out of scope

- The remaining ~47 components (spec 3).
- Skill rewrite and removal of the plugin pin (spec 4).
- Sidebar colour vars; the Sera style.
- Known unrelated bugs (offer separately): `layout::row` ~1px off-centre for
  mixed heights; Tabs list track not faded on own-flag disable; stale comment
  at `tests/theme_switch.rs:33`.
