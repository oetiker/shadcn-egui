---
name: build-ui
description: Use when building or restyling a Rust egui GUI from a shadcn or web design (screenshot, component code, preset code or description) with the shadcn-egui crate. Also use when an egui app renders without a GPU (CPU rasterizer, softbuffer, custom egui-winit loop) and is slow, pins a CPU core, redraws late, or misroutes pointer input.
---

# Build an egui UI with shadcn-egui

The `shadcn-egui` crate provides shadcn's styles, palettes, presets and
components as plain egui widgets. The hard part of egui is layout (single
pass, no flexbox), so this workflow maps the layout first and the looks second.

## When to use

A shadcn (or generally web or Tailwind) design needs to become an egui UI: a
screenshot to recreate, shadcn component code to port, a preset code from
ui.shadcn.com, or a described screen that must look professional.

The components only touch `egui::Context` and `egui::Ui`, so they work with
any egui integration: eframe, `egui-winit` with a CPU rasterizer (softbuffer),
glow or wgpu.

- A component or layout helper the design needs is not in the crate: port it
  with `shadcn-egui:port-component`. Do not paint it ad hoc.
- The app drives egui itself (`egui-winit`, `begin_pass`/`end_pass`) or
  rasterizes on the CPU: read `references/cpu-rendering.md` before writing
  the frame loop. It covers repaint scheduling, tessellation settings,
  rasterizer fast paths, damage tracking and the input-routing predicates
  that lie in a custom loop.

## Add the crate

Use the crate version that matches this skill: the `version` in
`${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json`. Enable only the `font-*`
and `icons-*` features the design uses (`references/presets.md` lists them).
The crate does not re-export egui, so the app depends on the egui version the
crate uses, `egui = "0.34"` for 0.4:

```bash
cargo add shadcn-egui@<version> --features font-geist,icons-hugeicons
cargo add egui@0.34
```

A path or git dependency takes the features in `Cargo.toml`:
`shadcn-egui = { path = "…", features = ["font-geist", "icons-hugeicons"] }`.

Do not add `eframe` on the crate's account. Add it only if the app already
uses it as its backend.

## Set the look

Set the look once, right after the egui context exists (in eframe: in the
`run_native` app creator, on `cc.egui_ctx`). Pick the route by what the design
gives:

- A preset code from ui.shadcn.com (the create page):

  ```rust
  use shadcn_egui::{Preset, PresetCodeError};

  fn setup(ctx: &egui::Context) -> Result<(), PresetCodeError> {
      Preset::from_code("b2D0wqNxT")?.apply(ctx);
      ctx.set_theme(egui::Theme::Dark);
      Ok(())
  }
  ```

  This code decodes to style Luma, base colour neutral, theme blue, chart
  emerald, font Geist, Hugeicons, the default radius and the menu colour
  `inverted-translucent`. Enable the features the preset names
  (`font-geist`, `icons-hugeicons`). The translucent menu colour draws as
  `inverted` and logs a warning.

- A `components.json`: start from `Preset::default()` and set `.style(..)`,
  `.base_color(..)` and `.icons(..)` from its `style`, `tailwind.baseColor`
  and `iconLibrary`:

  ```rust
  use shadcn_egui::{BaseColor, IconLibrary, Preset, RadiusChoice, StyleName};

  /// components.json: "style": "nova", "tailwind.baseColor": "neutral",
  /// "iconLibrary": "lucide"; no --radius in the CSS.
  fn setup(ctx: &egui::Context) {
      Preset::default()
          .style(StyleName::Nova)
          .base_color(BaseColor::Neutral)
          .icons(IconLibrary::Lucide)
          .radius(RadiusChoice::Default)
          .apply(ctx);
      ctx.set_theme(egui::Theme::Light);
  }
  ```

  Take the radius from `--radius` in the project's CSS when it is given:
  `0` is `RadiusChoice::None`, `0.45rem` is `Small`, `0.625rem` is `Medium`,
  `0.875rem` is `Large`, and any other value is `RadiusChoice::Px(px)` with
  1rem = 16px. Without a `--radius`, keep `RadiusChoice::Default`.

- Nothing given:

  ```rust
  fn setup(ctx: &egui::Context) {
      shadcn_egui::Theme::vega().apply(ctx);
  }
  ```

A preset that names a font or icon set whose feature is off falls back to
Inter or Lucide and says so through the `log` crate. Install a logger in the
app (for example `env_logger`) to see these warnings. An app with fonts of its
own uses `Theme::apply_with_fonts` (see `references/presets.md`). Apply again
only when the preset changes; widgets read the tokens with
`Theme::current(ctx)`.

A theme holds a light and a dark palette, and the mode lives in the egui
context: `ctx.set_theme(egui::Theme::Light)` switches crate and egui widgets
without another `apply`, and `egui::ThemePreference::System` follows the
system. Never build a theme per mode.

## Restyle a component part

Every colour, size and spacing a component draws comes from its recipe in
`theme.style` (one `Recipe` per part, named after the shadcn part:
`tabs_trigger`, `button_variant_outline`, `separator`). To change one,
push an important rule, the way a Tailwind `!` class does. It beats every
rule of the style that is not important, in light and dark, whatever states
that rule names. A rule the style itself marks important (a `!` class
upstream) still wins over yours when it also names a state, such as
`hover:bg-muted!`; one without a state loses to yours, because yours comes
later.

```rust
use std::sync::Arc;
use shadcn_egui::style::{Paint, Role, Rule, State, When};
use shadcn_egui::Preset;

fn setup(ctx: &egui::Context) {
    let mut theme = Preset::default().theme();
    let style = Arc::make_mut(&mut theme.style);
    style.tabs_trigger.bg.rules.push(Rule::important(
        When::all(State::SELECTED),
        Paint::Role(Role::Primary, 1.0),
    ));
    style.tabs_trigger.text.rules.push(Rule::important(
        When::all(State::SELECTED),
        Paint::Role(Role::PrimaryForeground, 1.0),
    ));
    theme.apply(ctx);
}
```

Set `prop.base` only to replace an unconditional value; a state the style
styles (hover, dark, active) keeps its own rule unless an important rule
covers it. `When::all(flags)` matches whenever every flag is set, whatever
else is; `When::new(all, none)` also excludes flags. The `State` flags and
the Tailwind variants they stand for are listed in the
`shadcn-egui:port-component` skill (`references/styles.md`). The recipes sit
in `Box`es: a field reaches through (`style.separator.height`), and a whole
replacement writes `*style.separator = Recipe { .. }`.

## App variants

A design whose component has a look upstream lacks (a destructive ghost
button, a roomy card) gets an app variant: a name the theme defines once from
a built-in variant, used like a built-in one. Name it through a constant, so
a misspelling does not compile:

```rust
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::style::{AppVariant, Paint, Role};
use shadcn_egui::Theme;

const DANGER_GHOST: AppVariant<ButtonVariant> = ButtonVariant::app("danger-ghost");
const DANGER_OUTLINE: AppVariant<ButtonVariant> = ButtonVariant::app("danger-outline");

fn setup(ctx: &egui::Context) {
    Theme::vega()
        .variant(DANGER_GHOST, ButtonVariant::Ghost, |r, _style| {
            r.button.text.set(Paint::Role(Role::Destructive, 1.0)); // Ghost's hover colours stay
        })
        .variant(DANGER_OUTLINE, ButtonVariant::Outline, |r, _| {
            // Red where Outline sets a text colour: on hover and when expanded.
            r.button.recolor(Role::Foreground, Role::Destructive)
        })
        .apply(ctx);
}

fn delete(ui: &mut egui::Ui) -> egui::Response {
    ui.add(Button::new("Delete").variant(DANGER_GHOST))
}
```

- The edit is a `fn` that captures nothing. It gets copies of the base's
  recipes, one field per part of the component (`ButtonRecipes { button }`,
  `CardRecipes { card, card_header, card_title, .. }`), and the `StyleName`
  the theme builds for, so a look may differ per style. What it leaves alone
  stays the base's in every state, style and mode.
- `prop.set(v)` replaces the unconditional value and keeps the base's state
  rules (hover, dark, focus); `prop.on(State::HOVER, v)` replaces the rules
  of that one state; `recipe.recolor(from, to)` swaps a colour role in every
  value and rule of the part, opacity kept.
- Every component has a `.variant(..)`. One without built-in variants
  (`CheckboxVariant`, `InputVariant`, `CardVariant`, …) has only `Default`,
  which an app variant starts from.
- Button, Card, Switch and Select also take app sizes, defined by
  `Theme::size` from a built-in size; a size passed to `Theme::variant`
  does not compile. A card, switch or select size edits the same parts as its
  variant, and `set` replaces the base size's value:

  ```rust
  use shadcn_egui::components::card::{Card, CardSize};
  use shadcn_egui::style::AppVariant;
  use shadcn_egui::Theme;

  const ROOMY: AppVariant<CardSize> = CardSize::app("roomy");

  fn setup(ctx: &egui::Context) {
      Theme::vega()
          .size(ROOMY, CardSize::Sm, |r, _| r.card.gap.set(20.0)) // gap-5
          .apply(ctx);
  }

  fn panel(ui: &mut egui::Ui) {
      Card::new().size(ROOMY).show(ui, |ui| ui.label("Roomy"));
  }
  ```

- App colours give an app variant a colour no palette role has, as a light
  and dark pair. Name it through a constant too:

  ```rust
  use shadcn_egui::color::oklch_to_srgb;
  use shadcn_egui::components::button::ButtonVariant;
  use shadcn_egui::style::{AppVariant, Role};
  use shadcn_egui::Theme;

  const BRAND: Role = Role::custom("brand");
  const BRAND_BUTTON: AppVariant<ButtonVariant> = ButtonVariant::app("brand");

  fn setup(ctx: &egui::Context) {
      Theme::vega()
          .color(BRAND, oklch_to_srgb(0.55, 0.2, 260.0), oklch_to_srgb(0.7, 0.16, 260.0))
          .variant(BRAND_BUTTON, ButtonVariant::Default, |r, _| {
              r.button.recolor(Role::Primary, BRAND)
          })
          .apply(ctx);
  }
  ```

- Light and dark share every app variant; the mode comes from the context
  (`ctx.set_theme(..)`), as for built-in variants.
- Parts follow their parent's variant: `card_title` in a
  `Card::new().variant(..)`, `dialog::footer` in a dialog, `popover_header`
  in a popover draw the parent's app variant without a setter of their own.
- A name the theme does not define stops a debug build with an assertion
  and draws as `Default` in a release build.

## Decompose and map

1. Decompose the design top-down: app shell, panels
   (`SidePanel`, `TopBottomPanel`, `CentralPanel`), the layout intent of each
   region (stack, row, grid, space-between), components with their variants,
   token deviations (usually none).
2. Map each part with the references:
   - `references/layout-map.md`: flexbox and grid intent to an egui helper.
     Read it first; layout is where iterations are lost.
   - `references/component-map.md`: shadcn component and variant to the
     crate's widget.
   - `references/token-map.md`: shadcn token to the egui field.
   - `references/icons.md`: icon libraries and drawing an icon.
   - `references/presets.md`: preset codes, the preset builder, features and
     fallbacks.

## Build

Build from `shadcn_egui::components` and `shadcn_egui::layout`. Drop to raw
egui only when no helper fits. A missing component goes through
`shadcn-egui:port-component`.

```rust
use shadcn_egui::components::button::Button;

fn toolbar(ui: &mut egui::Ui) {
    if ui.add(Button::new("Save changes")).clicked() {
        // save
    }
}
```

## Verify visually

Render the screen to a PNG and look at it next to the design, then iterate
on the numbers. The reference loop is an `egui_kittest` snapshot test. It is
dev-only: `egui_kittest` and `wgpu` never reach a shipped binary.

```toml
[dev-dependencies]
egui_kittest = { version = "0.34", features = ["wgpu", "snapshot"] }
```

```rust,ignore
use egui_kittest::Harness;

#[test]
fn settings_screen_dark() {
    let mut h = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .with_theme(egui::Theme::Dark)
        .build_ui(|ui| my_app::ui::settings(ui));
    // Once, on the harness's context, like an app right after startup.
    shadcn_egui::Theme::vega().apply(&h.ctx);
    h.run();
    h.snapshot("settings_dark");
}
```

`Harness.ctx` is the harness's `egui::Context`; apply the theme (or
`preset.apply(&h.ctx)`) there once, before `h.run()`, and set the mode with
the builder's `.with_theme(..)`. Applying it inside `build_ui` would apply it
every frame, which the hard rules forbid.

Each test holds one harness that takes snapshots. A second harness in the
same test (for example dark and light) fails with "Multiple SnapshotResults
were dropped without being handled": write one test per mode, or merge the
results with `h.take_snapshot_results()` and `SnapshotResults::extend`.

Generate the PNG with `UPDATE_SNAPSHOTS=1 cargo test`; it lands in
`tests/snapshots/`. Delete the PNG before each such run: kittest ignores a
change on anti-aliased pixels (1 px borders, rounded corners), even with
`UPDATE_SNAPSHOTS=force`, so an existing PNG can keep the old edges. Open the
PNG, compare it with the design, change the code, and render again. An update
run leaves `<name>.old.png` (the previous baseline) and a failing comparison
leaves `<name>.new.png` and `<name>.diff.png` beside the baselines.
Never commit them: delete them after looking, and add
`tests/snapshots/*.old.png`, `*.new.png` and `*.diff.png` to
`.gitignore`. To catch such edge changes in the test run too, compare
`h.render()` with the PNG again after `h.snapshot(..)` passes, every pixel,
each channel within 2 levels: shadcn-egui's own tests do that in
`crates/shadcn-egui/tests/common/strict.rs`. An app with its own render or screenshot path (for example a
softbuffer rasterizer) may capture the PNG there instead: the point is to look
at a real render.

## Hard rules

See `references/gotchas.md` for the reasons.

- egui is single-pass: a widget sets its own size before placement; there is
  no `flex-grow` distribution across siblings. Use the `shadcn_egui::layout`
  helpers, or `egui_extras::StripBuilder` (`Size::relative`, `exact`,
  `remainder`) where none fits.
- No `available_width()` arithmetic when a helper exists.
- Hover, active and focus follow the style's recipe; never invent a state
  colour.
- Apply the theme once, not every frame; read tokens with
  `Theme::current(ctx)`.
- No style values in UI code. Colours and sizes come from the theme
  (`Theme::current(ctx)`: palette, metrics, the style's recipes).
- Component internals (paddings, heights, gaps inside a widget) come from
  the recipes. Gaps between app-level blocks (`vstack`, `row`, card
  contents) come from the design: Tailwind `gap-N` is N×4 px (`gap-3` is
  12 px). Write each as a named constant (`const FIELD_GAP: f32 = 12.0;`
  with the class in a comment), not as a bare number at the call.
- Project-specific deviations go into the theme construction
  (`let mut s = Style::nova(); s.<part>.<field> = …;`, an important rule
  as in "Restyle a component part", or an app variant), never into copies
  of crate code.
- Every custom-painted widget reports itself with `resp.widget_info(..)`
  (role, label, enabled, selected), or screen readers see nothing; a role
  egui's `WidgetType` lacks goes through
  `shadcn_egui::components::shared::widget_info_with_role`.

## Acceptance bar

`shadcn_egui::reference::settings_ui` (a settings screen whose tabs double as
a component gallery) is the quality yardstick: match its restraint, with
subtle 1px borders, one radius, a muted palette, 14px text and consistent
focus rings. In a checkout of the shadcn-egui repository,
`cargo run --example settings` shows it live, and
`cargo run --example gallery --features fonts-all,icons-all` shows every
component under any preset.

Before calling a screen done, scan the project's UI files for literals that
bypass the theme. Carry a positive control, because a pattern that matches
nothing looks exactly like a clean codebase:

```bash
printf 'Color32::from_rgb(1,2,3)\n' | grep -cE 'Color32::(from_|WHITE|BLACK)|from_(black|white)_alpha|FontId::[a-z_]+\([0-9]|\.size\([0-9]'   # must print 1
grep -nE 'Color32::(from_|WHITE|BLACK)|from_(black|white)_alpha|FontId::[a-z_]+\([0-9]|\.size\([0-9]' src/ui/*.rs   # your UI files
```

Each hit is either a missing token or carries a `// token-ok: <reason>`
comment on the line above.
