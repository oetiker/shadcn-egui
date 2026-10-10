# egui_shadcn registry (vendored source)

These files are the canonical copy of the `egui_shadcn` module, kept in sync with
the repo `src/` and `assets/`. The skill workflow copies them into a target
project. Do not edit here directly — edit the repo `src/` and re-copy.

Add to the target's Cargo.toml:

```toml
egui = "0.34"
egui_extras = { version = "0.34", default-features = false }
```

Nothing else is needed: no `eframe`, no image loaders. Apply a theme once,
right after the egui context is created, then build with the `layout` +
`components` helpers:

```rust
use egui_shadcn::{Palette, Style, Theme};

Theme::new(Style::vega(), Palette::dark(), 10.0).apply(&ctx);
```
