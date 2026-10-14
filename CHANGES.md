# Changelog

All notable changes to the `shadcn-egui` crates and the Claude Code plugin
are recorded here. The format is based on
[Keep a Changelog](https://keepachangelog.com), and the project aims to follow [Semantic Versioning](https://semver.org).

## [Unreleased]

### New
- Every colour, size and spacing of every component can be changed through
  `theme.style`, for example a filled `primary` active tab with a solid 2 px
  focus ring. `Rule::important(When::all(State::SELECTED), value)` adds a
  rule ranked `Rule::IMPORTANT`, as a Tailwind `!` class is: it beats every
  rule of the style that is not important. A rule the style itself marks
  important still wins over it where that rule also names a state, such as
  hover. (#6)
- `Popover::side_offset` and `Select::side_offset` set the gap between the
  trigger and the popup; the default stays 4
  (`components::shared::DEFAULT_SIDE_OFFSET`). (#6)
- An `opacity` set in `theme.style` on any part now fades that part and
  everything inside it, multiplied as in the browser: a badge, card, dialog
  (its backdrop too), popover, tooltip, separator, tab list, select list,
  checkbox tick, select chevron or select tick. Before, only the outer part
  of a button, input, checkbox, switch, slider, label, select trigger or tab
  took its `opacity`. A custom component fades a container part's content
  with `components::shared::faded`. (#6)

### Changed
- In a line tab list (`TabsVariant::Line`), a focused tab that is not active
  shows its label in the foreground colour and a 2 px underline in the ring
  colour, in every style, and no longer draws a rounded ring around itself.
  The active tab looks the same focused or not. (#6)
- In the luma and rhea styles, the dialog's close button sits on a round
  disc in the secondary colour, as on the shadcn site. (#6)
- The separator's thickness comes from the style's `separator` recipe. An app
  that changed `metrics.border` to make separators thicker now changes
  `separator.height` (horizontal) or `separator.width` (vertical) instead. (#6)
- Replacing a whole recipe on a `Style` now needs a `*`:
  `*style.button = Recipe { .. }`. A `let` array of recipes for `pick`
  needs it too: `[&*style.button, &*style.button_size_sm]`. Setting one
  value, as in `style.button.height.base = Some(36.0)`, is unchanged.
- `Recipe` has the new fields `height_inset`, `outline_width` and
  `outline_color`, and `Style` the new recipes `separator` and
  `tabs_trigger_after` (the line tab underline). Code that builds either
  with a struct literal needs `..Default::default()`. (#6)
- `components::shared::disabled_painter` is gone. A custom widget fades a
  disabled part with `style_painter(ui, opacity(&[stack], state))`, which
  takes the fade from the part's recipe (`disabled:opacity-50`). (#6)

### Fixed
- A popover whose style recipe an app replaced whole
  (`*style.popover_content = Recipe { .. }`) showed each word on its own
  line; it is now as wide as its content. Buttons, badges, inputs, select
  triggers, tab lists, checkboxes and switches in the same case are no longer
  0 px high, and checkbox ticks, select chevrons and the check mark of the
  selected option no longer shrink to a dot. (#6)
- In the mira style, the open select list showed a tiny dot next to the
  selected option; it now shows a check mark. (#6)
- An app that held or copied several styles in one function could crash
  with `thread '<unknown>' has overflowed its stack`, mostly on worker
  threads and in WebAssembly, where the stack is 1 to 2 MiB. Each `Style`
  now takes a few hundred bytes of stack instead of about 110 KB.
- Building an app with Rust 1.99 no longer prints
  `warning: the following packages contain code that will be rejected by a future version of Rust: shadcn-egui v0.4.0`.

## 0.4.0 - 2026-10-13
### New
- A gallery example shows every component in each variant and size, under
  a preset you paste as a code or pick part by part (style, colours, fonts,
  icon library, radius, menu), in light or dark mode. Run it with
  `cargo run --example gallery --features fonts-all,icons-all`; an app shows
  the same screen with `reference::gallery_ui`.
- `Preset::from_code("b…")` reads a preset code copied from ui.shadcn.com,
  and `preset.apply(ctx, Mode::Dark)` sets up the app with its style, base
  colour, theme, chart colour, fonts, icon library, radius and menu colours.
  `Preset::default()` is the site's default (Nova, neutral, Inter, Lucide);
  `to_code()` gives the code back. Codes from shadcn-svelte.com are not
  supported: that site numbers colours and icon libraries differently.
- When a preset asks for something this build cannot show, the app uses a
  stand-in and logs a warning such as
  `shadcn-egui: font "geist" needs the "font-geist" feature; using "inter"`.
  The Sera style becomes Nova, the gray base colour, theme or chart colour
  (ui.shadcn.com has no colours for gray) becomes neutral or the base colour,
  and a font or icon library whose Cargo feature is off becomes Inter or
  Lucide. The translucent menu colours draw as the default or inverted menu
  colour, because egui cannot blur what lies behind a menu.
- With the Lyra or Sera style every corner is square, whatever radius the
  preset names, as on the shadcn site.
- A translucent menu colour always highlights menu items with the subtle
  accent, also when the preset asks for the bold one, as on ui.shadcn.com.
- An open Select list draws in the preset's menu colours: with the
  `inverted` menu colour it is dark also on a light page.
- Every font of the shadcn create page is available, each behind its own
  feature named after the font (`font-geist`, `font-jetbrains-mono`,
  `font-playfair-display`, …), and a preset that names the font uses it.
  The `fonts-all` feature turns them all on. A preset that names a font
  whose feature is off renders in Inter and logs a warning.
- The font crates carry the font's own licence next to MIT: `MIT AND OFL-1.1`
  for most fonts, `MIT AND Apache-2.0` for the Apache-licensed ones.
- The checkbox tick, select arrow and dialog close button follow the preset's
  icon library: Lucide by default, and Tabler, Phosphor, Remix Icon and
  Hugeicons with the features `icons-tabler`, `icons-phosphor`,
  `icons-remixicon` and `icons-hugeicons` (`icons-all` for all). Every icon of
  each set is available as a constant, for example
  `shadcn_egui::icons::tabler::HOME`.
- Filled icons (Phosphor, Remix Icon) are drawn with smooth edges, also when
  faded for a disabled widget or a translucent menu.
- `Palette::from_upstream(base, theme, chart, dark)` gives the palette the
  shadcn site builds for a base colour, a colour theme and a chart colour,
  for example `Palette::from_upstream("stone", "blue", "rose", true)`.
- `Palette` has the five chart colours (`chart-1` … `chart-5`) as `chart`.
- The `settings` example has a toolbar with a style select (vega, nova, maia,
  lyra, mira, luma, rhea), a Dark/Light tab pair and a radius slider from 0 to
  20 px, so the look can be changed while the window is open.
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

### Changed
- The Claude Code plugin is now called `shadcn-egui` and builds on the
  crate instead of copying code into the project; its skill `build-ui`
  builds a UI and `port-component` ports a missing component. It installs
  from the release tag, so it updates only with a crate release. Remove an
  `egui-shadcn` install with `/plugin uninstall egui-shadcn@egui-shadcn-dev`
  and `/plugin marketplace remove egui-shadcn-dev`, and install the new
  plugin with `/plugin marketplace add oetiker/shadcn-egui` and
  `/plugin install shadcn-egui@shadcn-egui`.
- The crate is published on crates.io as `shadcn-egui`: add it with
  `cargo add shadcn-egui` and change `egui_shadcn::` to `shadcn_egui::` in
  your imports. Copying the component module into a project is no longer
  supported.
- The default font is Inter, as on the shadcn site, so all text looks
  different from 0.3. Oxanium remains available through the `font-oxanium`
  feature.
- Medium and semibold text (button labels, badges, tabs, card and dialog
  titles) now renders at its real weight; before, every weight looked like
  regular text.
- Text in Inter and Oxanium sits up to one pixel higher in some sizes, for
  example in badges and tooltips.
- Card and dialog titles use the theme's heading font, which is the body
  font unless `Theme::fonts(body, Some(heading))` sets another one.
- An app that installs fonts of its own calls
  `Theme::apply_with_fonts(ctx, its_font_definitions)` instead of
  `Theme::apply`; this keeps the app's fonts next to the theme's.
- `Theme::apply` sets egui's light and its dark style alike: text sizes,
  spacing and colours that an app set with `ctx.set_style_of` or
  `ctx.style_mut_of` before the call are overwritten in both modes.
- `Theme` has the new public fields `style`, `fonts`, `icons` and `menu`, so
  a `Theme { .. }` struct literal written for 0.3 no longer compiles; build
  themes with `Theme::new(..)` and set the rest with `Theme::fonts(..)`,
  `Theme::icons(..)` and `Theme::menu(..)`. Checkbox, Select and Dialog draw
  the icons of the theme, so a preset's icon library changes them.
- A `Palette { .. }` struct literal written for 0.3 no longer compiles,
  because `Palette` has the new field `chart`; start from `Palette::light()`,
  `Palette::dark()` or `Palette::from_upstream(..)` and change single fields.
- `Button::icon` and `Button::icon_end` accept any icon reference, not only
  constants.
- Icon data you generated with 0.3 no longer compiles with 0.4. Use the
  icons in `shadcn_egui::icons` (Lucide, and the other sets behind their
  features) or regenerate your own.
- Remix Icon stays at version 4.8.0, its last release under the Apache-2.0
  licence; later releases forbid redistribution as an icon pack.
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
- `card_title` and `card_description` return the label's `Response`. Where
  such a call ends a block that must return nothing, add a `;`.
- The new `ButtonSize` and `BadgeVariant` variants (see New) break a
  `match` that lists every variant of these enums; add the new arms or a `_`
  arm.
- The `sm` to `4xl` corner radii scale with `radius` by a factor
  (`rounded-xl` is 1.4 x radius), as in current shadcn. At the default radius
  of 10 px, small, medium, large and extra-large corners look as before;
  larger radii now round more.
- `Popover::width(..)` and `Dialog::width(..)` set the width of the whole box
  including its padding, so existing callers get narrower content.
- Native egui widgets such as `ui.button(..)` take their height, padding and
  spacing from the style's default button and tabs gap: a native button is
  36 px high in vega and 32 px in nova, like the crate's own buttons.
  Applying a style that sets no button height brings back egui's default
  height instead of keeping the previous theme's.
- Disabled controls are drawn at half opacity both with `.enabled(false)` and
  inside a disabled area (`ui.add_enabled(false, ..)`, `Ui::disable`). A
  disabled slider or switch shows its track through the thumb.
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
  `.invalid(true)` and `.enabled(false)` work as for inputs, and the track or
  thumb casts the shadow the style sets (the track in vega).
- Labels are 14 px text with no extra line height, and a label shown with
  `Label::new(..).enabled(false)` is drawn at half opacity. Separators are
  exactly 1 px thick and add no spacing of their own.
- Helper text from `label::description`, popover headers and tooltips use the
  line height of the web version (20 px for 14 px text, 16 px for 12 px), so
  they take a little more room. Popover titles and descriptions take their
  own size and line height from the style, which makes the title larger in
  maia, lyra, mira, luma and rhea.
- Tabs: the tab list is as wide as its labels instead of filling the row, and
  the active tab shows a raised pill with a soft shadow. Left, Right, Home and
  End move the selection, and `.enabled(false)` draws the tabs at half opacity.
  A mouse click selects a tab without drawing the keyboard focus ring.
- Tooltips are inverted as in current shadcn: a foreground-coloured bubble
  with background-coloured text instead of the primary colour, with a small
  rounded arrow pointing at the trigger. Text wraps at 320 px.
- Popovers draw a thin outline ring instead of a border and a softer shadow,
  with padding and corners from the style. The default width is 288 px.
- Selects: the trigger has a chevron and is as wide as its widest option
  instead of the full field width, and the list has a thin outline ring and a
  check mark on the chosen row. The list takes Up, Down, Home, End, Enter, Esc
  and type-ahead keys until keyboard focus moves to another control.
- Screen readers get no name for a switch, checkbox or select on its own:
  name each one after its label with `.labelled_by(label.id)`, as the
  `settings` example does. A select reports its shown text as its value.
- Cards fill the available width, with padding, gaps and corners from the
  style.
- Dialogs have a thin outline ring and no shadow, a 384 px (nova) or 448 px
  (vega) wide panel, a close button that looks like a ghost icon button, and a
  lighter backdrop without blur. In nova the footer is a tinted strip reaching
  the panel edges.
- The `settings` example takes the style and palette on the command line, for
  example `cargo run --example settings -- nova light` (default `vega dark`).
  `SettingsState` has new `slider` and `region` fields.
- A `match` on a preset enum (`StyleName`, `BaseColor`, `ThemeColor`,
  `FontName`, `IconLibrary`, `MenuColor`, `MenuAccent`, `RadiusChoice`) or
  on `Role`, `TabsOrientation` or `PresetCodeError` needs a `_ =>` arm, and
  each preset enum's `ALL` is a slice (`&'static [Self]`), not an array. New
  shadcn styles, colours, fonts and icon sets can then arrive in a minor
  release.

### Removed
- `Theme::dark()` and `Theme::light()`: use `Theme::vega_dark()` /
  `Theme::vega_light()` or `Theme::new(..)`. `Theme::shadow_md()` and
  `shadow_lg()` are gone too; use `Theme::shadow(ShadowLevel)`.
- `Metrics` fields that a style now sets (control heights, paddings, gap, card
  padding, dialog and popover width, ring width), and `Palette::shadow` and
  `Palette::overlay`. Set such values on the `Style` in your theme setup
  instead.
- The new-york look; there is no compatibility style.
- `components::shared::mix_toward`, `hover_fill` and `focus_ring`: the
  components take hover and focus colours from the style.
- `components::icon::{CHECK, CHEVRON_DOWN, CLOSE, LUCIDE_SLOTS}`: use
  `icons::lucide::{CHECK, CHEVRON_DOWN, X, SLOTS}`, which also gives you every
  other Lucide icon (2141 in all).

### Fixed
- Every `Theme::apply_with_fonts` that passed the context's current fonts
  added the body font to the font list once more, so the list grew with each
  call. After such a call switched from a monospace body font to a sans one,
  code and other monospace text still showed in the old body font. Both
  are fixed.
- On a desktop with a light system theme, an app that called `Theme::apply`
  showed egui's own light look instead of the shadcn theme, and the theme
  also vanished when the desktop switched between light and dark. The theme
  now holds in both cases.
- Faded and disabled icons are evenly transparent: line ends, crossings and
  filled shapes no longer show darker spots.
- An app that called `Theme::apply` while drawing a frame crashed with
  "FontFamily::Name(\"shadcn-heading-semibold\") is not bound to any fonts"
  as soon as a window or popup showed a heading in that frame. Such a heading
  now shows in the body font for that one frame.
- An app that replaced the fonts with `ctx.set_fonts` after `Theme::apply`
  crashed with "FontFamily::Name(\"shadcn-heading-semibold\") is not bound to
  any fonts" on the next heading or the next `Theme::apply`. Headings now fall
  back to the regular text font with a log warning, and a later
  `Theme::apply` adds the theme's fonts back and keeps the app's own.
- Screen readers announced a standalone icon by its internal name ("mic",
  "power") and ignored a label attached with `labelled_by`. An `IconView` now
  has no name of its own; name it with `.labelled_by(label.id)`.
- In a `layout::row`, items placed before a taller item, such as a label
  before a large button, sat about 2 px above the row's centre. They are now
  centred. A row shrinks back when its tallest item goes, except a row that
  holds a vertical separator or a scroll area, which keeps its tallest height.

## 0.3.2 - 2026-10-09

### Changed
- The skill, the gotchas and the registry README told you to call
  `Theme::apply` every frame. They now say to call it once, right after the
  egui context is created (for eframe, in the `run_native` app creator), and
  again only when the theme changes. Re-applying every frame silently undid
  any other code's style changes, so such bugs never showed.
- The settings example applies the theme in the eframe app creator instead
  of on every frame.

## 0.3.1 - 2026-10-09

### Fixed
- Ghost and Outline buttons kept their normal text colour on hover while the
  background turned to `accent`. With a palette whose `accent` is bright,
  the label became unreadable. Hovered, they now use `accent_foreground`
  for text and icon, as shadcn does.
- `Stroke::new(2.0, …)` in the checkbox drew a `float_literal_f32_fallback`
  warning from newer Rust compilers, which will become an error.

## 0.3.0 - 2026-10-09

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

## 0.2.1 - 2026-10-01

### Added
- **New `references/cpu-rendering.md`** for apps that drive egui themselves and
  rasterize without a GPU (softbuffer): what to change when one keystroke pins a
  CPU core, a new screen shows up only after the mouse moves, or toggles animate
  in visible steps. It also covers `is_pointer_over_egui()` reporting `true`
  everywhere under a `CentralPanel`, and the Wayland connect segfault from
  `egui-winit`'s default clipboard feature. The skill now triggers on these
  symptoms too.

## 0.2.0 - 2026-06-14

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

## 0.1.0 - 2026-06-13

### Added
- Initial release: shadcn-v4 (new-york / OKLCH) design tokens and theme for egui,
  a flexbox-substitute layout layer (`StripBuilder` helpers), and themed
  components (Button, Input, Label, Card, Tabs, Switch, Checkbox, Select,
  Separator, Badge).
- Layout-first `SKILL.md` workflow with reference tables (token / layout /
  component maps) and a headless `egui_kittest` render-verification loop.
- Reference settings-form-with-tabs screen + snapshot eval; Claude Code plugin
  manifest and marketplace listing.
