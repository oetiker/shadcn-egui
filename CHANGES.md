# Changelog

All notable changes to the **egui-shadcn** plugin and the `egui_shadcn` crate are
recorded here. The format is based on [Keep a Changelog](https://keepachangelog.com),
and the project aims to follow [Semantic Versioning](https://semver.org).

## [Unreleased]

### Changed
- Components now take their sizes, colours, radii and shadows from a shadcn
  style. `Theme::new(Style::vega(), Palette::dark(), 10.0)` gives the classic
  look and `Style::nova()` the compact one, matching a web client set up with
  the same `style` in `components.json`; `Style::maia()`, `lyra()`, `mira()`,
  `luma()` and `rhea()` are available too, but are not covered by the snapshot
  tests.
- Controls look slightly different from 0.3 even with vega: destructive
  buttons and badges are light red tints instead of solid red, hovered Ghost
  and Outline buttons use the muted fill, and cards and popovers draw a thin
  outline ring instead of a border. Controls are 36 px high in vega and 32 px
  in nova, with 10 px side padding.
- `Theme` is `Clone` and no longer `Copy`, so code that reuses a theme value
  must call `.clone()`.
- The `sm` to `4xl` corner radii scale with `radius` by a factor
  (`rounded-xl` is 1.4 x radius), as in current shadcn. At the default radius
  of 10 px, small, medium, large and extra-large corners look as before;
  larger radii now round more.
- `Popover::width(..)` and `Dialog::width(..)` set the width of the whole box
  including its padding, so existing callers get narrower content.
- Native egui widgets take their height,
  padding and spacing from the style's default button and tabs gap, so in nova
  they look slightly shorter than before.
- Buttons: a pressed button moves down by 1 px, a hovered Link button is
  underlined, and a disabled button is drawn at half opacity instead of being
  blended toward the page colour. Keyboard focus draws a 3 px ring at the
  button's edge and turns its border to the ring colour.
- Badges are 20 px high with fully rounded ends.
- Text inputs have a 3 px focus ring, and the border is the `input` colour at
  rest and the ring colour on keyboard focus. `.invalid(true)` draws a red
  border and ring, and `.enabled(false)` draws the field at half opacity and
  ignores typing.
- Checkboxes are a 16 px box with a 1 px border, a checkmark icon, a soft
  shadow and a 3 px focus ring, and the click target reaches 12 px to each
  side and 8 px above and below. `.invalid(true)` and `.enabled(false)` work as
  for inputs.
- Switches have a 32 x 18.4 px track, a round thumb inside the 1 px border
  and a 3 px focus ring, with the same enlarged click target as checkboxes.
  `.invalid(true)` and `.enabled(false)` work as for inputs.
- Labels are 14 px text with no extra line height, and a label shown with
  `Label::new(..).enabled(false)` is drawn at half opacity. Separators are
  exactly 1 px thick and add no spacing of their own.
- Tabs: the tab list is as wide as its labels instead of filling the row, and
  the active tab shows a raised pill with a soft shadow. Left, Right, Home and
  End move the selection, and `.enabled(false)` draws the tabs at half opacity.
- Tooltips are inverted as in current shadcn: a foreground-coloured bubble
  with background-coloured text instead of the primary colour, with a small
  rounded arrow pointing at the trigger. Text wraps at 320 px.
- Popovers draw a thin outline ring instead of a border and a softer shadow,
  with padding and corners from the style. The default width is 288 px.
- Selects: the trigger has a chevron and is as wide as its widest option
  instead of the full field width, and the list has a thin outline ring and a
  check mark on the chosen row. The list takes Up, Down, Home, End, Enter, Esc
  and type-ahead keys.
- Cards fill the available width, with padding, gaps and corners from the
  style.
- Dialogs have a thin outline ring and no shadow, a 384 px (nova) or 448 px
  (vega) wide panel, a close button that looks like a ghost icon button, and a
  lighter backdrop without blur. In nova the footer is a tinted strip reaching
  the panel edges.
- The `settings` example takes the style and palette on the command line, for
  example `cargo run --example settings -- nova light` (default `vega dark`).
  `SettingsState` has new `slider` and `region` fields.

### Added
- `Slider` with one thumb, horizontal or vertical. A click or drag sets the
  value, the arrow, Page Up/Down, Home and End keys step it, and
  `.enabled(false)` draws it at half opacity and ignores input.
- Button sizes `Xs`, `IconXs`, `IconSm` and `IconLg`, and leading or trailing
  button icons. Buttons also show the expanded, invalid and has-popup states.
- Badge variants `Ghost` and `Link`, and leading or trailing badge icons.
- `SwitchSize::Sm` (24 x 14 px) and `Tabs::new(..).variant(TabsVariant::Line)`,
  which draws an underline under the active tab.
- `Input::invalid`, `Checkbox::invalid`, `Switch::invalid` and
  `Select::new(..)` options `.width(..)`, `.size(SelectSize::Sm)`,
  `.placeholder(..)`, `.invalid(..)` and `.enabled(..)`; `Dialog` gets
  `show_close_button(false)`.
- `Card::new().size(CardSize::Sm).footer(..).show(ui, ..)` with
  `card_header`, `card_header_with_action` and `card_content`, and
  `popover_header(ui, title, description)`; the card footer is a tinted strip
  reaching the card edges in nova.
- `separator_vertical` draws a vertical line.
- `Palette::with_bold_menu_accent()` for shadcn's `menuAccent: "bold"`.
- `Metrics` has new fields `line_xs`, `line_sm`, `line_base` and `line_lg`
  (Tailwind's line heights for the text sizes), and `Theme::line_px(TextSize)`
  returns the line height for a text size.
- `Theme::vega_dark()` and `Theme::vega_light()` for the default vega style.

### Removed
- `Theme::dark()` and `Theme::light()`: use `Theme::vega_dark()` /
  `Theme::vega_light()` or `Theme::new(..)`. `Theme::shadow_md()` and
  `shadow_lg()` are gone too; use `Theme::shadow(ShadowLevel)`.
- `Metrics` fields that a style now sets (control heights, paddings, gap, card
  padding, dialog and popover width, ring width), and `Palette::shadow` and
  `Palette::overlay`. Set such values on the `Style` in your theme setup
  instead.
- The new-york look; there is no compatibility style.

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
