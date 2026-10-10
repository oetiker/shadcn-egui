# egui_shadcn registry (vendored source)

These files are the canonical copy of the `egui_shadcn` module, kept in sync with
the repo `src/` and `assets/`. The skill workflow copies them into a target
project. Do not edit here directly — edit the repo `src/` and re-copy.

Add to the target's Cargo.toml:

```toml
egui = "0.34"
eframe = "0.34"
egui_extras = { version = "0.34", features = ["all_loaders"] }
```

Call `egui_shadcn::Theme::vega_dark().apply(ctx)` once, right after the egui context
is created, then build with the
`layout` + `components` helpers.
