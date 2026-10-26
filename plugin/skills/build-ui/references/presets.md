# Presets

A `Preset` holds the choices of the shadcn create page on ui.shadcn.com:
style, base colour, theme colour, chart colour, body and heading font, icon
library, radius, menu colour and menu accent. Light or dark is not part of a
preset; the app passes a `Mode` when it applies one.

## Preset codes

The create page shows a code such as `b2D0wqNxT` for every preset.
`Preset::from_code` decodes it and `to_code` writes it:

```rust
use shadcn_egui::{Preset, PresetCodeError};

fn setup(ctx: &egui::Context) -> Result<(), PresetCodeError> {
    let preset = Preset::from_code("b2D0wqNxT")?;
    preset.apply(ctx);
    ctx.set_theme(egui::Theme::Dark);
    Ok(())
}
```

- `from_code` returns `PresetCodeError` for an empty string, an unknown
  version letter (codes start with `a` or `b`), a character outside base62,
  or a number above 2^53. An index past the end of a list decodes to the
  list's first value, as on the site.
- `to_code` returns the canonical `b` code the site would write, or `None`
  for a `RadiusChoice::Px` radius, which has no code.
- Only codes from ui.shadcn.com are supported. Codes from shadcn-svelte.com
  number their lists differently and decode to another preset.

## Building a preset

`Preset::default()` is the site's default (code `b0`): style Nova, base
colour neutral, font Inter, icons Lucide, `RadiusChoice::Default`. Each field
has a builder method:

- `.style(StyleName::…)`: `Nova`, `Vega`, `Maia`, `Lyra`, `Mira`, `Luma`,
  `Sera`, `Rhea`.
- `.base_color(BaseColor::…)`: `Neutral`, `Stone`, `Zinc`, `Gray`, `Mauve`,
  `Olive`, `Mist`, `Taupe`.
- `.theme_color(ThemeColor::…)` and `.chart_color(ThemeColor::…)`: the base
  colours plus `Amber`, `Blue`, `Cyan`, `Emerald`, `Fuchsia`, `Green`,
  `Indigo`, `Lime`, `Orange`, `Pink`, `Purple`, `Red`, `Rose`, `Sky`, `Teal`,
  `Violet`, `Yellow`.
- `.font(FontName::…)` and `.font_heading(FontHeading::Inherit)` or
  `.font_heading(FontHeading::Font(FontName::…))`.
- `.icons(IconLibrary::…)`: `Lucide`, `Hugeicons`, `Tabler`, `Phosphor`,
  `Remixicon`.
- `.radius(RadiusChoice::…)`: `Default` (the base colour's own radius),
  `None`, `Small` (0.45rem), `Medium` (0.625rem), `Large` (0.875rem), or
  `Px(f32)` for any other radius in logical px.
- `.menu_color(MenuColor::…)`: `Default`, `Inverted`, `DefaultTranslucent`,
  `InvertedTranslucent`; `.menu_accent(MenuAccent::…)`: `Subtle`, `Bold`.

```rust
use shadcn_egui::{BaseColor, Preset, RadiusChoice, StyleName, ThemeColor};

fn setup(ctx: &egui::Context) {
    Preset::default()
        .style(StyleName::Vega)
        .base_color(BaseColor::Zinc)
        .theme_color(ThemeColor::Blue)
        .radius(RadiusChoice::Px(6.0))
        .apply(ctx);
    ctx.set_theme(egui::Theme::Light);
}
```

`preset.theme(mode)` returns the `Theme` without applying it, for an app that
adjusts the theme before `theme.apply(ctx)`.

## Fonts and icons

Inter and the Lucide icons are always included. Every other font and icon set
is a Cargo feature named after its shadcn id:

- `font-*`: one font each, for example `font-geist`, `font-jetbrains-mono`,
  `font-playfair-display`, `font-ibm-plex-sans`.
- `fonts-all`: every font.
- `icons-tabler`, `icons-phosphor`, `icons-remixicon`, `icons-hugeicons`: one
  icon set each.
- `icons-all`: every icon set.

Enable only the features the preset uses. Each one adds a crate with the font
files or the icon data to the build.

## Fallbacks

`Preset::theme` and `Preset::apply` never fail. A value this build cannot
show falls back to a usable one and logs one `log::warn!` line, on every call:

- A font whose feature is off: Inter.
- An icon library whose feature is off: Lucide.
- Style `sera`: `nova`. Sera has no egui version yet.
- Base colour `gray`, which the site's codes can name but which has no
  palette: neutral.
- A translucent menu colour (`DefaultTranslucent`, `InvertedTranslucent`):
  the opaque one. egui has no backdrop blur.

The warnings go through the `log` crate; the app needs a logger (for example
`env_logger`) to show them.

## Applying

`preset.apply(ctx, mode)` is `preset.theme(mode).apply(ctx)`. Call it once
after the context exists, and again when the mode or the preset changes. The
fonts are reinstalled only when they change, so switching between light and
dark is cheap.

An app that installs fonts of its own uses `Theme::apply_with_fonts`. It
starts from the app's `FontDefinitions` instead of egui's defaults and adds
the theme's fonts on top. A plain `ctx.set_fonts` after `apply` drops the
theme's font families, and headings fall back to the default font.

```rust
use shadcn_egui::Preset;

fn setup(ctx: &egui::Context, my_fonts: egui::FontDefinitions) {
    let theme = Preset::default().theme();
    theme.apply_with_fonts(ctx, my_fonts);
}
```
