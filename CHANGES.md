# Changelog

All notable changes to the **egui-shadcn** plugin and the `egui_shadcn` crate are
recorded here. The format is based on [Keep a Changelog](https://keepachangelog.com),
and the project aims to follow [Semantic Versioning](https://semver.org).

## [Unreleased]

### Changed
- A theme now carries a shadcn style: `Theme::new(Style::vega(), palette, radius)`
  or `Style::nova()` for the compact look; `Theme::vega_dark()` and
  `Theme::vega_light()` give the default vega style. `Theme` can no longer be
  copied implicitly, so code that reuses a theme value must call `.clone()`.
- The `sm` to `4xl` corner radii now scale with `radius` by a factor, as in
  current shadcn, instead of adding or subtracting fixed pixels. At the
  default radius of 10 px, small, medium, large and extra-large corners look
  the same as before.
- Buttons follow the chosen style: vega buttons are 36 px high, nova buttons
  32 px with rounder corners, both with 10 px side padding instead of 16 px,
  and the Destructive button is a light red tint with red text instead of a
  solid red fill. A pressed button moves down by 1 px. New sizes (`Xs`,
  `IconXs`, `IconSm`, `IconLg`), a trailing icon and the expanded, invalid
  and has-popup states are available.
- Button states follow the style too: a hovered Ghost or Outline button
  takes the style's muted hover fill instead of `accent`, and a hovered Link
  button is underlined. Keyboard focus draws a 3 px ring right at the
  button's edge and turns its border to the ring colour. A disabled button
  is drawn at half opacity, so what lies behind it shows through, instead of
  being blended toward the page colour.
- Badges follow the chosen style: 20 px high with fully rounded ends, and
  the Destructive badge is a light red tint with red text instead of a solid
  red fill. New `Ghost` and `Link` variants and leading or trailing icons are
  available.

## [0.3.2] - 2026-10-09

### Changed
- The skill, the gotchas and the registry README told you to call
  `Theme::apply` every frame. They now say to call it once, right after the
  egui context is created (for eframe, in the `run_native` app creator), and
  again only when the theme changes. Re-applying every frame silently undid
  any other code's style changes, so such bugs never showed.
- The settings example applies the theme in the eframe app creator instead
  of on every frame.

## [0.3.1] - 2026-10-09

### Fixed
- Ghost and Outline buttons kept their normal text colour on hover while the
  background turned to `accent`. With a palette whose `accent` is bright,
  the label became unreadable. Hovered, they now use `accent_foreground`
  for text and icon, as shadcn does.
- `Stroke::new(2.0, …)` in the checkbox drew a `float_literal_f32_fallback`
  warning from newer Rust compilers, which will become an error.

## [0.3.0] - 2026-10-09

### Added
- Dialog, Popover and Tooltip components in the shadcn style. A dialog closes
  on Esc, a click on the dimmed backdrop, or its × button. A popover opens and
  closes when its trigger is clicked and closes when you click elsewhere.
  Their colors, shadows and widths are theme settings (`Palette.overlay`,
  `Metrics.dialog_width`, `Metrics.popover_width`).
- Icons without an image loader. The new `icongen` tool (in `tools/icongen/`)
  turns Lucide or other outline SVG icons into Rust constants, and
  `IconView` / `Button::icon` draw them in the theme's colors and sizes. A
  project includes only the icons it lists and gains no runtime dependency.
  Dots drawn as tiny lines, as in Lucide's `circle-alert`, stay visible.
- `Button::enabled(false)` greys a button out halfway toward the page
  background and stops it from reacting to clicks or taking focus, like
  shadcn's `disabled` buttons. Screen readers announce it as disabled.
- `Input::id_source("…")` gives a text field a fixed id, so code elsewhere can
  focus it or a test can find it, however many widgets come before it.
- Sizes are now part of the theme: `Theme.metrics` holds text sizes, control
  heights, paddings, the default gap, card padding, field width, icon size and
  border/ring widths. Change one value and every component follows.
- `Palette.shadow` sets the card shadow color.
- Badges are announced to screen readers.
- The skill now requires colors and text sizes to come from the theme, and
  gives a check to run over a project's own UI files before calling a screen
  done. New components must also report themselves to screen readers.
- `layout::wrap_row` lays items out in a row that wraps onto the next line when
  it runs out of width, with the same gap between items and between lines.

### Changed
- **Breaking:** code that builds a `Theme` or `Palette` field by field must add
  `metrics` (use `Metrics::default()`) and `shadow`. Projects using
  `Theme::light()`/`dark()` are unaffected.
- The destructive badge's text uses the `destructive_foreground` color instead
  of pure white.
- The skill's advice on animations no longer contradicts itself: a fade timed
  by hand vanishes instead of fading unless it repaints while it runs.
- A focused text field's border now turns the `ring` colour, like shadcn's
  inputs, instead of staying the plain `border` colour under the focus ring.

## [0.2.1] - 2026-10-01

### Added
- **New `references/cpu-rendering.md`** for apps that drive egui themselves and
  rasterize without a GPU (softbuffer): what to change when one keystroke pins a
  CPU core, a new screen shows up only after the mouse moves, or toggles animate
  in visible steps. It also covers `is_pointer_over_egui()` reporting `true`
  everywhere under a `CentralPanel`, and the Wayland connect segfault from
  `egui-winit`'s default clipboard feature. The skill now triggers on these
  symptoms too.

## [0.2.0] - 2026-06-14

### Changed
- **Slimmed the runtime dependency surface to just `egui` + `egui_extras`.**
  `egui_extras` now uses `default-features = false` (it only needs
  `StripBuilder`/`Size`); the previous `all_loaders` feature pulled `image`,
  `resvg`/`usvg`, and `ehttp` into every consumer for no reason. `eframe`,
  `egui_kittest`, and `wgpu` are dev-only — they never reach a shipped binary.
- **Migrated to the non-deprecated egui/eframe 0.34 `&mut Ui` model.**
  `reference::settings_ui` now takes `&mut egui::Ui` (uses
  `CentralPanel::show_inside`); the example uses `eframe::run_ui_native`; the
  snapshot harness uses `Harness::builder().build_ui`. All `#[allow(deprecated)]`
  removed. **Breaking:** `settings_ui(ctx, …)` → `settings_ui(ui, …)`.
- **Reworked `SKILL.md`** into two clearly separated parts — *Using the
  components* vs *Porting a new component* — and added a **dependency-discipline
  standard** (default: add no runtime dep; heavy crates like eframe/image/wgpu
  stay dev-only; ask before adding one). README and plugin manifests reframed as
  backend-agnostic (plain egui widgets; no eframe/wgpu coupling).

### Fixed
- **Vertical text centering** in buttons, inputs, badges, and tabs. Oxanium's
  ascent reserves more empty space above the cap than below the baseline, so
  egui's row-box centering left single-line text ~2.5 px too high. Corrected
  globally with `FontTweak { y_offset_factor: 0.18 }` on all three font weights.

### Added
- **Component gallery.** `reference::settings_ui` is now a three-tab screen
  (Account / Notifications / Components) that exercises every ported component:
  Button (all variants + sizes), Input (incl. password), Select, Label, Switch,
  Checkbox, Badge (all variants), Card, Tabs, Separator.
- Snapshot coverage for all three tabs; refreshed README screenshots and added a
  component-gallery shot.

## [0.1.0] - 2026-06-13

### Added
- Initial release: shadcn-v4 (new-york / OKLCH) design tokens and theme for egui,
  a flexbox-substitute layout layer (`StripBuilder` helpers), and themed
  components (Button, Input, Label, Card, Tabs, Switch, Checkbox, Select,
  Separator, Badge).
- Layout-first `SKILL.md` workflow with reference tables (token / layout /
  component maps) and a headless `egui_kittest` render-verification loop.
- Reference settings-form-with-tabs screen + snapshot eval; Claude Code plugin
  manifest and marketplace listing.

[0.3.2]: https://github.com/oetiker/egui-shadcn/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/oetiker/egui-shadcn/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/oetiker/egui-shadcn/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/oetiker/egui-shadcn/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/oetiker/egui-shadcn/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/oetiker/egui-shadcn/releases/tag/v0.1.0
