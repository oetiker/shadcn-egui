# shadcn-egui

The [shadcn/ui](https://ui.shadcn.com/) design system for
[egui](https://github.com/emilk/egui): the shadcn v4 styles (vega, nova,
maia, lyra, mira, luma, rhea), their colour themes, fonts, icon sets and
presets, and components drawn as plain egui widgets. It works with any egui
backend (eframe, egui-winit, glow, wgpu).

![shadcn-egui settings screen](https://raw.githubusercontent.com/oetiker/shadcn-egui/main/docs/settings-screen.png)

## Install

```bash
cargo add shadcn-egui
```

## Use a shadcn preset

Build a look on the shadcn create page (ui.shadcn.com), copy its preset code
and apply it. Codes from shadcn-svelte.com number colours differently and
are not supported.

```rust
use shadcn_egui::{Mode, Preset};

fn setup(ctx: &egui::Context) -> Result<(), shadcn_egui::PresetCodeError> {
    Preset::from_code("b2D0wqNxT")?.apply(ctx, Mode::Dark);
    Ok(())
}
```

This preset uses the Geist font and the Hugeicons icons: add
`--features font-geist,icons-hugeicons`, else it falls back to Inter and
Lucide with log warnings.

`apply` again to switch mode or preset; the fonts are reinstalled only when
they change. An app that installs fonts of its own uses
`Theme::apply_with_fonts` instead (see the `Theme` docs). Without a preset,
`Theme::vega_dark().apply(ctx)` gives the default look.

## Components

Button, Badge, Input, Label, Card, Tabs, Switch, Checkbox, Select, Slider,
Separator, Tooltip, Popover and Dialog:

```rust
use shadcn_egui::components::button::Button;

fn ui(ui: &mut egui::Ui) {
    if ui.add(Button::new("Save changes")).clicked() {
        // ...
    }
}
```

`cargo run --example gallery --features fonts-all,icons-all` shows every
component, each variant and size, and the icons of the chosen icon library.
Paste a preset code or pick each preset part (style, colours, fonts, icon
library, radius, menu) and the light or dark mode, and the gallery restyles
live.

![shadcn-egui component gallery](https://raw.githubusercontent.com/oetiker/shadcn-egui/main/docs/gallery-screen.png)

`cargo run --example settings` opens a settings page built from Badge,
Button, Card, Checkbox, Input, Label, Select, Separator, Slider, Switch and
Tabs, with a toolbar that switches style, mode and radius.

## Features

Inter and the Lucide icons are always included. Everything else is a Cargo
feature named after the shadcn id:

| Feature | Adds |
|---|---|
| `font-<id>` | one font of the create page, for example `font-geist`, `font-jetbrains-mono`, `font-playfair-display` |
| `fonts-all` | every font |
| `icons-tabler` | Tabler icons |
| `icons-phosphor` | Phosphor icons |
| `icons-remixicon` | Remix Icon |
| `icons-hugeicons` | Hugeicons |
| `icons-all` | every icon set |

A preset that names a font or icon set whose feature is off renders with
Inter or Lucide and logs a warning through the `log` crate. The `sera`
style has no egui version yet and falls back to `nova`.

## Licences

The code is MIT (`LICENSE`). Each font crate also ships its font's licence
(SIL Open Font License 1.1 or Apache License 2.0). The icon crates ship
theirs: Lucide ISC, Tabler and Phosphor MIT, Remix Icon Apache-2.0 (version
4.8.0, the last release under that licence; later releases are not used),
Hugeicons MIT. Each crate's `license` field names both the code and the
bundled licence.

## The Claude Code plugin

This repository also hosts the `egui-shadcn` Claude Code plugin, which
copied the 0.3 component code into projects. It stays at 0.3.2 until its
skill is rewritten for this crate.

## Development

See [CLAUDE.md](CLAUDE.md) for the layout, generators and checks, and
[RELEASING.md](RELEASING.md) for releases.
