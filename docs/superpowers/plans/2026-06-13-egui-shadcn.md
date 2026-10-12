# egui-shadcn Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a vendored egui component module (shadcn-v4 design tokens + flex-substitute layout helpers + themed components) plus a skill that teaches Claude to rebuild shadcn designs in egui, proven by a settings-form-with-tabs reference rendered headlessly.

**Architecture:** A real, compiling Rust crate `egui_shadcn` is the source-of-truth "registry." Tokens are derived from OKLCH literals via a tested converter and pushed into `egui::Style`/`Visuals`/`Spacing`. Layout helpers wrap `egui_extras::StripBuilder` to emulate flexbox distribution. Components are custom-painted widgets whose variants mirror shadcn's cva axes. A `skills/egui-shadcn/` directory holds `SKILL.md` + reference tables that instruct Claude to copy the registry files into target projects and map designs onto them. Verification is `egui_kittest` snapshot rendering (proven working headless via wgpu/mesa).

**Tech Stack:** Rust 1.95, `egui` 0.34, `eframe` 0.34, `egui_extras` 0.34, `egui_kittest` 0.34 (features `wgpu`, `snapshot`), Oxanium font (OFL).

**Environment constraints:** Compiler/tests must use **no more than 4 cores** — every cargo command in this plan passes `--jobs 4` and sets `CARGO_BUILD_JOBS=4`. Cargo target dir is `$CARGO_TARGET_DIR` (already configured on this machine). Run all commands from the repo root `<checkout>`.

---

## File Structure

```
egui-skill/
├── Cargo.toml                          # [package] egui_shadcn (single crate, not workspace)
├── assets/
│   ├── Oxanium-Regular.ttf
│   ├── Oxanium-Medium.ttf
│   └── Oxanium-SemiBold.ttf
├── src/
│   ├── lib.rs                          # re-exports; theme() accessor
│   ├── color.rs                        # oklch_to_srgb converter (tested)
│   ├── theme.rs                        # Palette, Theme, apply(), fonts
│   ├── layout.rs                       # row/vstack/justify/form_row/card/grid
│   └── components/
│       ├── mod.rs                      # pub use of each component
│       ├── shared.rs                   # focus ring + hover-mix helpers
│       ├── button.rs
│       ├── label.rs
│       ├── input.rs
│       ├── checkbox.rs
│       ├── switch.rs
│       ├── select.rs
│       ├── tabs.rs
│       ├── card.rs                     # CardHeader/Title/Description helpers
│       ├── separator.rs
│       └── badge.rs
├── examples/
│   └── settings.rs                     # the reference screen (runnable via eframe)
├── tests/
│   ├── color.rs                        # unit tests for the converter
│   ├── components.rs                   # kittest interaction tests
│   ├── settings_snapshot.rs           # the acceptance eval
│   └── snapshots/                      # committed reference PNGs
├── skills/
│   └── egui-shadcn/
│       ├── SKILL.md
│       └── references/
│           ├── token-map.md
│           ├── layout-map.md
│           ├── component-map.md
│           └── gotchas.md
└── docs/superpowers/{specs,plans}/
```

Each `src/components/*.rs` owns exactly one component. `theme.rs` owns tokens+style application. `layout.rs` owns the flex substitute. `color.rs` is pure math, separately testable.

---

## Phase 0 — Scaffold

### Task 0: Cargo project + deps + first green test

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `tests/color.rs`

- [ ] **Step 1: Write `Cargo.toml`**

```toml
[package]
name = "egui_shadcn"
version = "0.1.0"
edition = "2021"

[dependencies]
egui = "0.34"
egui_extras = { version = "0.34", features = ["all_loaders"] }

[dev-dependencies]
eframe = "0.34"
egui_kittest = { version = "0.34", features = ["wgpu", "snapshot"] }

[[example]]
name = "settings"
```

- [ ] **Step 2: Write a placeholder `src/lib.rs`**

```rust
pub mod color;
```

- [ ] **Step 3: Write the failing test `tests/color.rs`**

```rust
use egui_shadcn::color::oklch_to_srgb;

#[test]
fn white_is_white() {
    let c = oklch_to_srgb(1.0, 0.0, 0.0);
    assert_eq!((c.r(), c.g(), c.b()), (255, 255, 255));
}
```

- [ ] **Step 4: Run it — expect a COMPILE failure (module `color` missing)**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test color 2>&1 | tail -20`
Expected: build error — `file not found for module \`color\`` / `oklch_to_srgb` unresolved.

- [ ] **Step 5: Commit the scaffold (red state intentionally not committed yet — proceed to Task 1).**

No commit here; Task 1 makes it green and commits together.

---

## Phase 1 — Color + Theme

### Task 1: OKLCH → sRGB converter

**Files:**
- Create: `src/color.rs`
- Test: `tests/color.rs` (extend)

- [ ] **Step 1: Extend `tests/color.rs` with the achromatic anchors (hand-verified)**

```rust
use egui_shadcn::color::{oklch_to_srgb, oklch_to_srgb_a};

#[test]
fn white_is_white() {
    let c = oklch_to_srgb(1.0, 0.0, 0.0);
    assert_eq!((c.r(), c.g(), c.b()), (255, 255, 255));
}

#[test]
fn achromatic_grays_match_zinc() {
    // (lightness, expected gray) verified by hand against shadcn v4 zinc ramp.
    let cases = [
        (0.985, 250u8),
        (0.922, 229),
        (0.708, 161),
        (0.556, 115),
        (0.269, 38),
        (0.205, 23),
        (0.145, 10),
    ];
    for (l, expected) in cases {
        let c = oklch_to_srgb(l, 0.0, 0.0);
        let got = c.r();
        assert!(
            (got as i16 - expected as i16).abs() <= 1,
            "L={l}: expected ~{expected}, got {got}"
        );
        assert_eq!(c.r(), c.g(), "L={l} not gray");
        assert_eq!(c.g(), c.b(), "L={l} not gray");
    }
}

#[test]
fn destructive_red_is_reddish() {
    let c = oklch_to_srgb(0.577, 0.245, 27.325);
    assert!(c.r() > 200, "expected strong red, got r={}", c.r());
    assert!(c.g() < 90 && c.b() < 90, "expected low g/b, got {:?}", (c.g(), c.b()));
}

#[test]
fn alpha_is_passed_through() {
    let c = oklch_to_srgb_a(1.0, 0.0, 0.0, 0.10);
    assert_eq!(c.a(), 26); // round(0.10 * 255)
}
```

- [ ] **Step 2: Run — expect FAIL (unresolved symbols)**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test color 2>&1 | tail -20`
Expected: compile error, `oklch_to_srgb`/`oklch_to_srgb_a` not found.

- [ ] **Step 3: Implement `src/color.rs`**

```rust
//! OKLCH → sRGB conversion (shadcn v4 stores tokens as OKLCH).
//! Converts oklch(L C H) to an egui Color32. Out-of-gamut channels are clamped.

use egui::Color32;

/// `l` in 0..=1, `c` chroma (~0..0.4), `h` hue in degrees.
pub fn oklch_to_srgb(l: f32, c: f32, h: f32) -> Color32 {
    oklch_to_srgb_a(l, c, h, 1.0)
}

/// Same as [`oklch_to_srgb`] with an explicit alpha in 0..=1.
pub fn oklch_to_srgb_a(l: f32, c: f32, h: f32, alpha: f32) -> Color32 {
    let h_rad = h.to_radians();
    let a = c * h_rad.cos();
    let b = c * h_rad.sin();

    // OKLab -> LMS' -> LMS
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_35 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_55 * b;
    let (lc, mc, sc) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);

    // LMS -> linear sRGB
    let r = 4.076_741_66 * lc - 3.307_711_59 * mc + 0.230_969_93 * sc;
    let g = -1.268_438_00 * lc + 2.609_757_40 * mc - 0.341_319_40 * sc;
    let bl = -0.004_196_09 * lc - 0.703_418_61 * mc + 1.707_614_70 * sc;

    let to_u8 = |lin: f32| -> u8 {
        let lin = lin.clamp(0.0, 1.0);
        let srgb = if lin <= 0.003_130_8 {
            12.92 * lin
        } else {
            1.055 * lin.powf(1.0 / 2.4) - 0.055
        };
        (srgb * 255.0).round().clamp(0.0, 255.0) as u8
    };

    Color32::from_rgba_unmultiplied(
        to_u8(r),
        to_u8(g),
        to_u8(bl),
        (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}
```

- [ ] **Step 4: Run — expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test color 2>&1 | tail -20`
Expected: `test result: ok. 4 passed`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/lib.rs src/color.rs tests/color.rs
git commit -m "feat: OKLCH->sRGB converter with verified zinc anchors

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: Palette + Theme + radius scale

**Files:**
- Create: `src/theme.rs`
- Modify: `src/lib.rs`
- Test: `tests/color.rs` is separate; add `tests/theme.rs`

- [ ] **Step 1: Create `tests/theme.rs` (failing)**

```rust
use egui_shadcn::theme::Theme;

#[test]
fn radius_scale_is_derived() {
    let t = Theme::dark();
    assert_eq!(t.radius, 10.0);
    assert_eq!(t.radius_sm(), 6.0);
    assert_eq!(t.radius_md(), 8.0);
    assert_eq!(t.radius_lg(), 10.0);
    assert_eq!(t.radius_xl(), 14.0);
}

#[test]
fn light_and_dark_differ() {
    let l = Theme::light();
    let d = Theme::dark();
    assert!(l.dark == false && d.dark == true);
    assert_ne!(l.palette.background, d.palette.background);
}

#[test]
fn dark_border_is_low_alpha_white() {
    let d = Theme::dark();
    // dark --border is oklch(1 0 0 / 10%) -> white at ~26 alpha
    assert_eq!(d.palette.border.a(), 26);
}
```

- [ ] **Step 2: Run — expect FAIL (module missing)**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test theme 2>&1 | tail -20`
Expected: unresolved `egui_shadcn::theme`.

- [ ] **Step 3: Implement `src/theme.rs` (palette derived from OKLCH literals)**

```rust
//! shadcn v4 (new-york / OKLCH) design tokens, materialized for egui.

use crate::color::{oklch_to_srgb as c, oklch_to_srgb_a as ca};
use egui::Color32;

/// The semantic shadcn color tokens. Surfaces come in `x` / `x_foreground` pairs.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub background: Color32,
    pub foreground: Color32,
    pub card: Color32,
    pub card_foreground: Color32,
    pub popover: Color32,
    pub popover_foreground: Color32,
    pub primary: Color32,
    pub primary_foreground: Color32,
    pub secondary: Color32,
    pub secondary_foreground: Color32,
    pub muted: Color32,
    pub muted_foreground: Color32,
    pub accent: Color32,
    pub accent_foreground: Color32,
    pub destructive: Color32,
    pub destructive_foreground: Color32,
    pub border: Color32,
    pub input: Color32,
    pub ring: Color32,
}

impl Palette {
    pub fn light() -> Self {
        Self {
            background: c(1.0, 0.0, 0.0),
            foreground: c(0.145, 0.0, 0.0),
            card: c(1.0, 0.0, 0.0),
            card_foreground: c(0.145, 0.0, 0.0),
            popover: c(1.0, 0.0, 0.0),
            popover_foreground: c(0.145, 0.0, 0.0),
            primary: c(0.205, 0.0, 0.0),
            primary_foreground: c(0.985, 0.0, 0.0),
            secondary: c(0.97, 0.0, 0.0),
            secondary_foreground: c(0.205, 0.0, 0.0),
            muted: c(0.97, 0.0, 0.0),
            muted_foreground: c(0.556, 0.0, 0.0),
            accent: c(0.97, 0.0, 0.0),
            accent_foreground: c(0.205, 0.0, 0.0),
            destructive: c(0.577, 0.245, 27.325),
            destructive_foreground: c(0.985, 0.0, 0.0),
            border: c(0.922, 0.0, 0.0),
            input: c(0.922, 0.0, 0.0),
            ring: c(0.708, 0.0, 0.0),
        }
    }

    pub fn dark() -> Self {
        Self {
            background: c(0.145, 0.0, 0.0),
            foreground: c(0.985, 0.0, 0.0),
            card: c(0.205, 0.0, 0.0),
            card_foreground: c(0.985, 0.0, 0.0),
            popover: c(0.205, 0.0, 0.0),
            popover_foreground: c(0.985, 0.0, 0.0),
            primary: c(0.922, 0.0, 0.0),
            primary_foreground: c(0.205, 0.0, 0.0),
            secondary: c(0.269, 0.0, 0.0),
            secondary_foreground: c(0.985, 0.0, 0.0),
            muted: c(0.269, 0.0, 0.0),
            muted_foreground: c(0.708, 0.0, 0.0),
            accent: c(0.269, 0.0, 0.0),
            accent_foreground: c(0.985, 0.0, 0.0),
            destructive: c(0.704, 0.191, 22.216),
            destructive_foreground: c(0.985, 0.0, 0.0),
            border: ca(1.0, 0.0, 0.0, 0.10),
            input: ca(1.0, 0.0, 0.0, 0.15),
            ring: c(0.556, 0.0, 0.0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub palette: Palette,
    pub radius: f32,
    pub dark: bool,
}

impl Theme {
    pub fn light() -> Self {
        Self { palette: Palette::light(), radius: 10.0, dark: false }
    }
    pub fn dark() -> Self {
        Self { palette: Palette::dark(), radius: 10.0, dark: true }
    }

    pub fn radius_sm(&self) -> f32 { self.radius - 4.0 }
    pub fn radius_md(&self) -> f32 { self.radius - 2.0 }
    pub fn radius_lg(&self) -> f32 { self.radius }
    pub fn radius_xl(&self) -> f32 { self.radius + 4.0 }
}
```

- [ ] **Step 4: Update `src/lib.rs`**

```rust
pub mod color;
pub mod theme;

pub use theme::{Palette, Theme};
```

- [ ] **Step 5: Run — expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test theme 2>&1 | tail -20`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 6: Commit**

```bash
git add src/theme.rs src/lib.rs tests/theme.rs
git commit -m "feat: shadcn v4 Palette + Theme with derived radius scale

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Fonts + `Theme::apply` + `theme()` accessor

This wires the tokens into `egui::Style`/`Visuals`/`Spacing`, loads Oxanium, and stores the active `Theme` in egui's data store so components can read it.

**Files:**
- Create: `assets/Oxanium-Regular.ttf`, `assets/Oxanium-Medium.ttf`, `assets/Oxanium-SemiBold.ttf`
- Modify: `src/theme.rs`
- Modify: `src/lib.rs`
- Test: `tests/theme.rs` (extend)

- [ ] **Step 1: Download the Oxanium font files (OFL) into `assets/`**

```bash
mkdir -p assets
base="https://github.com/google/fonts/raw/main/ofl/oxanium/static"
curl -sfL "$base/Oxanium-Regular.ttf"  -o assets/Oxanium-Regular.ttf
curl -sfL "$base/Oxanium-Medium.ttf"   -o assets/Oxanium-Medium.ttf
curl -sfL "$base/Oxanium-SemiBold.ttf" -o assets/Oxanium-SemiBold.ttf
file assets/Oxanium-*.ttf
```
Expected: three `TrueType Font data` files. If the path 404s, find the correct raw URLs under `github.com/google/fonts/tree/main/ofl/oxanium` (variable font fallback: download `Oxanium[wght].ttf` and register it three times — egui will use it at one weight; note this limitation in a comment).

- [ ] **Step 2: Extend `tests/theme.rs` with an apply round-trip test**

```rust
#[test]
fn apply_sets_panel_fill_and_stores_theme() {
    use egui_shadcn::theme::{theme, Theme};
    let ctx = egui::Context::default();
    Theme::dark().apply(&ctx);
    // run one frame so style takes effect
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |_ui| {});
    });
    let got = theme(&ctx);
    assert_eq!(got.dark, true);
    assert_eq!(ctx.style().visuals.panel_fill, Theme::dark().palette.background);
}
```

- [ ] **Step 3: Run — expect FAIL**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test theme 2>&1 | tail -20`
Expected: `apply`/`theme` unresolved.

- [ ] **Step 4: Add font loading, `apply`, and `theme()` to `src/theme.rs`**

```rust
use egui::{FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle};
use std::sync::Arc;

const THEME_ID: egui::Id = egui::Id::new("egui_shadcn::active_theme");

/// Named font families for shadcn-style weight emphasis.
pub const FAMILY_MEDIUM: &str = "oxanium-medium";
pub const FAMILY_SEMIBOLD: &str = "oxanium-semibold";

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "oxanium".into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/Oxanium-Regular.ttf"))),
    );
    fonts.font_data.insert(
        FAMILY_MEDIUM.into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/Oxanium-Medium.ttf"))),
    );
    fonts.font_data.insert(
        FAMILY_SEMIBOLD.into(),
        Arc::new(FontData::from_static(include_bytes!("../assets/Oxanium-SemiBold.ttf"))),
    );
    // Regular becomes the default proportional face.
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "oxanium".into());
    // Named families for emphasis.
    fonts
        .families
        .insert(FontFamily::Name(FAMILY_MEDIUM.into()), vec![FAMILY_MEDIUM.into()]);
    fonts
        .families
        .insert(FontFamily::Name(FAMILY_SEMIBOLD.into()), vec![FAMILY_SEMIBOLD.into()]);
    ctx.set_fonts(fonts);
}

impl Theme {
    /// Read the active theme stored by [`Theme::apply`]. Falls back to dark.
    pub fn current(ctx: &egui::Context) -> Theme {
        ctx.data(|d| d.get_temp::<Theme>(THEME_ID)).unwrap_or_else(Theme::dark)
    }

    /// Push this theme into the egui context: fonts, type scale, spacing, colors.
    pub fn apply(&self, ctx: &egui::Context) {
        install_fonts(ctx);
        let p = &self.palette;

        ctx.style_mut(|s| {
            use FontFamily::Proportional;
            // Type scale (shadcn: 14px body / medium emphasis).
            s.text_styles = [
                (TextStyle::Small, FontId::new(12.0, Proportional)),
                (TextStyle::Body, FontId::new(14.0, Proportional)),
                (TextStyle::Button, FontId::new(14.0, Proportional)),
                (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
                (TextStyle::Heading, FontId::new(20.0, FontFamily::Name(FAMILY_SEMIBOLD.into()))),
            ]
            .into();

            // Spacing on the 4px grid.
            s.spacing.item_spacing = egui::vec2(8.0, 8.0);
            s.spacing.button_padding = egui::vec2(16.0, 8.0);
            s.spacing.window_margin = egui::Margin::same(0);
            s.spacing.interact_size.y = 36.0;

            let v = &mut s.visuals;
            v.dark_mode = self.dark;
            v.panel_fill = p.background;
            v.window_fill = p.card;
            v.window_stroke = Stroke::new(1.0, p.border);
            v.extreme_bg_color = p.input;
            v.faint_bg_color = p.muted;
            v.override_text_color = Some(p.foreground);
            v.hyperlink_color = p.primary;
            v.selection.bg_fill = p.primary.gamma_multiply(0.35);
            v.selection.stroke = Stroke::new(1.0, p.ring);
            v.widgets.noninteractive.bg_fill = p.background;
            v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
            v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.foreground);
            for w in [
                &mut v.widgets.inactive,
                &mut v.widgets.hovered,
                &mut v.widgets.active,
                &mut v.widgets.open,
            ] {
                w.bg_fill = p.secondary;
                w.weak_bg_fill = p.secondary;
                w.bg_stroke = Stroke::new(1.0, p.border);
                w.fg_stroke = Stroke::new(1.0, p.foreground);
                w.corner_radius = egui::CornerRadius::same(self.radius_md() as u8);
            }
        });

        ctx.data_mut(|d| d.insert_temp(THEME_ID, *self));
    }
}

/// Convenience accessor for components.
pub fn theme(ctx: &egui::Context) -> Theme {
    Theme::current(ctx)
}
```

- [ ] **Step 5: Export `theme` from `src/lib.rs`**

```rust
pub mod color;
pub mod theme;

pub use theme::{theme, Palette, Theme};
```

- [ ] **Step 6: Run — expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test theme 2>&1 | tail -20`
Expected: all theme tests pass.

- [ ] **Step 7: Commit**

```bash
git add assets src/theme.rs src/lib.rs tests/theme.rs
git commit -m "feat: Oxanium fonts + Theme::apply + theme() accessor

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Phase 2 — Layout (the flex substitute)

### Task 4: `row`, `vstack`, `justify`

**Files:**
- Create: `src/layout.rs`
- Modify: `src/lib.rs`
- Test: `tests/layout.rs`

- [ ] **Step 1: Create `tests/layout.rs` (failing, snapshot-style smoke)**

```rust
use egui_kittest::Harness;
use egui_shadcn::{layout, Theme};

#[test]
fn row_and_vstack_compose() {
    let mut h = Harness::new(|ctx| {
        Theme::dark().apply(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            layout::vstack(ui, 12.0, |ui| {
                ui.label("top");
                layout::row(ui, 8.0, |ui| {
                    ui.label("a");
                    ui.label("b");
                });
            });
        });
    });
    h.run();
    // Smoke: it built a frame without panicking.
    assert!(h.ctx.used_rect().width() > 0.0);
}
```

- [ ] **Step 2: Run — expect FAIL (unresolved `layout`)**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test layout 2>&1 | tail -20`
Expected: `layout` unresolved.

- [ ] **Step 3: Implement `src/layout.rs` (row/vstack/justify)**

```rust
//! Flexbox substitutes for egui's single-pass layout.
//! `gap` maps to item_spacing; grow/space-between use egui_extras::StripBuilder.

use egui::Ui;
use egui_extras::{Size, StripBuilder};

/// Horizontal stack with an explicit gap (flex-direction: row; gap: N).
pub fn row<R>(ui: &mut Ui, gap: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        add(ui)
    })
    .inner
}

/// Vertical stack with an explicit gap (flex-direction: column; gap: N).
pub fn vstack<R>(ui: &mut Ui, gap: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = gap;
        add(ui)
    })
    .inner
}

/// A row whose `left` content hugs the start and `right` content hugs the end
/// (justify-content: space-between). Implemented with a remainder strip.
pub fn space_between(
    ui: &mut Ui,
    left: impl FnOnce(&mut Ui),
    right: impl FnOnce(&mut Ui),
) {
    let height = ui.spacing().interact_size.y;
    StripBuilder::new(ui)
        .size(Size::remainder())
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                ui.horizontal(|ui| left(ui));
            });
            strip.cell(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    right(ui)
                });
            });
        });
    let _ = height;
}
```

- [ ] **Step 4: Export from `src/lib.rs`**

```rust
pub mod color;
pub mod layout;
pub mod theme;

pub use theme::{theme, Palette, Theme};
```

- [ ] **Step 5: Run — expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test layout 2>&1 | tail -20`
Expected: pass. If `used_rect` API differs, replace the assertion with `assert!(h.ctx.screen_rect().width() > 0.0);`.

- [ ] **Step 6: Commit**

```bash
git add src/layout.rs src/lib.rs tests/layout.rs
git commit -m "feat: layout helpers row/vstack/space_between

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: `card` + `form_row`

**Files:**
- Modify: `src/layout.rs`
- Test: `tests/layout.rs` (extend)

- [ ] **Step 1: Extend `tests/layout.rs`**

```rust
#[test]
fn card_and_form_row_render() {
    let mut h = Harness::new(|ctx| {
        Theme::dark().apply(ctx);
        let t = Theme::dark();
        egui::CentralPanel::default().show(ctx, |ui| {
            layout::card(ui, &t, |ui| {
                layout::form_row(ui, 120.0, "Name", |ui| {
                    let mut s = String::from("hi");
                    ui.text_edit_singleline(&mut s);
                });
            });
        });
    });
    h.run();
    h.snapshot("layout_card");
}
```

- [ ] **Step 2: Run — expect FAIL (card/form_row missing + missing snapshot)**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test layout 2>&1 | tail -20`
Expected: unresolved `card`/`form_row`.

- [ ] **Step 3: Add `card` + `form_row` to `src/layout.rs`**

```rust
use crate::Theme;
use egui::{CornerRadius, Frame, Margin, Stroke};

/// A shadcn Card surface: card fill, 1px border, rounded-xl, p-6, subtle shadow.
pub fn card<R>(ui: &mut Ui, theme: &Theme, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(theme.palette.card)
        .stroke(Stroke::new(1.0, theme.palette.border))
        .corner_radius(CornerRadius::same(theme.radius_xl() as u8))
        .inner_margin(Margin::same(24))
        .shadow(egui::epaint::Shadow {
            offset: [0, 1],
            blur: 3,
            spread: 0,
            color: egui::Color32::from_black_alpha(20),
        })
        .show(ui, |ui| add(ui))
        .inner
}

/// A labeled control row: fixed-width label column + control filling the rest.
/// Emulates a form grid where labels share a column width.
pub fn form_row(ui: &mut Ui, label_width: f32, label: &str, control: impl FnOnce(&mut Ui)) {
    let row_h = ui.spacing().interact_size.y;
    StripBuilder::new(ui)
        .size(Size::exact(label_width))
        .size(Size::remainder())
        .horizontal(|mut strip| {
            strip.cell(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add_space(0.0);
                    ui.label(label);
                });
            });
            strip.cell(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    control(ui)
                });
            });
        });
    let _ = row_h;
}
```

- [ ] **Step 4: Run — first run WRITES the snapshot and FAILS with "Missing snapshot"**

Run: `CARGO_BUILD_JOBS=4 UPDATE_SNAPSHOTS=1 cargo test --jobs 4 --test layout 2>&1 | tail -20`
Expected: snapshot `tests/snapshots/layout_card.png` created; rerun without the env var passes.

- [ ] **Step 5: Inspect the render**

Read `tests/snapshots/layout_card.png` as an image. Confirm: a dark rounded card with a visible 1px border, a left-aligned "Name" label, and a text field to its right. If the border/radius/padding look wrong, adjust the values in `card`/`form_row` and re-run with `UPDATE_SNAPSHOTS=1`.

- [ ] **Step 6: Run normally — expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test layout 2>&1 | tail -20`
Expected: pass.

- [ ] **Step 7: Commit (include the snapshot PNG)**

```bash
git add src/layout.rs tests/layout.rs tests/snapshots/layout_card.png
git commit -m "feat: card + form_row layout helpers

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Phase 3 — Components

### Task 6: shared helpers (hover mix + focus ring)

**Files:**
- Create: `src/components/mod.rs`
- Create: `src/components/shared.rs`
- Modify: `src/lib.rs`
- Test: `tests/components.rs`

- [ ] **Step 1: Create `tests/components.rs` (failing)**

```rust
use egui_shadcn::components::shared::mix_toward;

#[test]
fn mix_toward_blends() {
    let a = egui::Color32::from_rgb(200, 0, 0);
    let b = egui::Color32::from_rgb(0, 0, 0);
    let m = mix_toward(a, b, 0.5);
    assert!(m.r() > 90 && m.r() < 110, "got {}", m.r());
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: unresolved `components::shared`.

- [ ] **Step 3: Implement `src/components/shared.rs`**

```rust
//! Shared painting helpers for shadcn-style components.

use egui::{Color32, Rect, Response, Stroke, Ui};
use crate::Theme;

/// Linearly blend `a` toward `b` by `t` in 0..=1 (sRGB space, good enough for hover).
pub fn mix_toward(a: Color32, b: Color32, t: f32) -> Color32 {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()), a.a())
}

/// shadcn hover = base color at 90% opacity over the background. We approximate
/// by mixing the fill 10% toward the page background.
pub fn hover_fill(fill: Color32, bg: Color32) -> Color32 {
    mix_toward(fill, bg, 0.10)
}

/// Paint the shadcn focus ring: a 3px ring at 50% of the ring color, just outside
/// the widget rect, when the response has keyboard focus.
pub fn focus_ring(ui: &Ui, resp: &Response, theme: &Theme, corner: f32) {
    if resp.has_focus() {
        let ring = theme.palette.ring.gamma_multiply(0.5);
        let rect = resp.rect.expand(2.0);
        ui.painter().rect_stroke(
            rect,
            corner,
            Stroke::new(3.0, ring),
            egui::StrokeKind::Outside,
        );
    }
    let _ = Rect::NOTHING;
}
```

- [ ] **Step 4: Implement `src/components/mod.rs`**

```rust
pub mod shared;
```

- [ ] **Step 5: Export from `src/lib.rs`**

```rust
pub mod color;
pub mod components;
pub mod layout;
pub mod theme;

pub use theme::{theme, Palette, Theme};
```

- [ ] **Step 6: Run — expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add src/components tests/components.rs src/lib.rs
git commit -m "feat: shared component helpers (mix/hover/focus ring)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 7: Button

**Files:**
- Create: `src/components/button.rs`
- Modify: `src/components/mod.rs`
- Test: `tests/components.rs` (extend)

- [ ] **Step 1: Extend `tests/components.rs` with a click test**

```rust
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use egui_shadcn::components::button::{Button, ButtonVariant};
use egui_shadcn::Theme;

#[test]
fn button_click_fires() {
    let mut clicked = false;
    let mut h = Harness::new(|ctx| {
        Theme::dark().apply(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.add(Button::new("Save").variant(ButtonVariant::Default)).clicked() {
                // captured below via state; see harness pattern
            }
        });
    });
    // Drive: find by label, click, run.
    h.get_by_label("Save").click();
    h.run();
    let _ = &mut clicked;
}
```

Note: capturing the click across the closure boundary is awkward; the assertion that matters is that a node labeled "Save" exists and is clickable without panic. If the team prefers an explicit assertion, store click state in a `std::cell::Cell` captured by reference:

```rust
#[test]
fn button_click_sets_flag() {
    use std::cell::Cell;
    let clicked = Cell::new(false);
    let mut h = Harness::new(|ctx| {
        Theme::dark().apply(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.add(Button::new("Save")).clicked() {
                clicked.set(true);
            }
        });
    });
    h.get_by_label("Save").click();
    h.run();
    assert!(clicked.get(), "button click did not register");
}
```

- [ ] **Step 2: Run — expect FAIL (Button missing)**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: unresolved `components::button`.

- [ ] **Step 3: Implement `src/components/button.rs`**

```rust
//! shadcn Button: variants (default/destructive/outline/secondary/ghost/link)
//! x sizes (sm/default/lg/icon). Custom-painted for full per-state control.

use crate::components::shared::{focus_ring, hover_fill};
use crate::Theme;
use egui::{
    Align2, Color32, CornerRadius, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget,
    WidgetText,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Default,
    Destructive,
    Outline,
    Secondary,
    Ghost,
    Link,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    Sm,
    Default,
    Lg,
    Icon,
}

pub struct Button {
    text: WidgetText,
    variant: ButtonVariant,
    size: ButtonSize,
}

impl Button {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self { text: text.into(), variant: ButtonVariant::Default, size: ButtonSize::Default }
    }
    pub fn variant(mut self, v: ButtonVariant) -> Self {
        self.variant = v;
        self
    }
    pub fn size(mut self, s: ButtonSize) -> Self {
        self.size = s;
        self
    }
}

struct Colors {
    fill: Color32,
    text: Color32,
    border: Option<Color32>,
}

fn colors_for(variant: ButtonVariant, t: &Theme) -> Colors {
    let p = &t.palette;
    match variant {
        ButtonVariant::Default => Colors { fill: p.primary, text: p.primary_foreground, border: None },
        ButtonVariant::Destructive => Colors { fill: p.destructive, text: Color32::WHITE, border: None },
        ButtonVariant::Secondary => Colors { fill: p.secondary, text: p.secondary_foreground, border: None },
        ButtonVariant::Outline => Colors { fill: p.background, text: p.foreground, border: Some(p.border) },
        ButtonVariant::Ghost => Colors { fill: Color32::TRANSPARENT, text: p.foreground, border: None },
        ButtonVariant::Link => Colors { fill: Color32::TRANSPARENT, text: p.primary, border: None },
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let (height, pad_x) = match self.size {
            ButtonSize::Sm => (32.0, 12.0),
            ButtonSize::Default => (36.0, 16.0),
            ButtonSize::Lg => (40.0, 24.0),
            ButtonSize::Icon => (36.0, 0.0),
        };

        let galley = self
            .text
            .into_galley(ui, Some(egui::TextWrapMode::Extend), f32::INFINITY, egui::TextStyle::Button);

        let width = if self.size == ButtonSize::Icon {
            36.0
        } else {
            galley.size().x + pad_x * 2.0
        };
        let desired = Vec2::new(width.max(galley.size().x), height);
        let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());

        let c = colors_for(self.variant, &t);
        let hovered = resp.hovered();
        let mut fill = c.fill;
        if hovered && fill != Color32::TRANSPARENT {
            fill = hover_fill(fill, t.palette.background);
        } else if hovered && matches!(self.variant, ButtonVariant::Ghost) {
            fill = t.palette.accent;
        }

        let corner = CornerRadius::same(t.radius_md() as u8);
        if ui.is_rect_visible(rect) {
            if fill != Color32::TRANSPARENT {
                ui.painter().rect_filled(rect, corner, fill);
            }
            if let Some(b) = c.border {
                ui.painter().rect_stroke(rect, corner, Stroke::new(1.0, b), StrokeKind::Inside);
            }
            let text_pos = rect.center();
            ui.painter().galley(
                egui::pos2(text_pos.x - galley.size().x / 2.0, text_pos.y - galley.size().y / 2.0),
                galley,
                c.text,
            );
            let _ = Align2::CENTER_CENTER;
        }
        focus_ring(ui, &resp, &t, t.radius_md());
        resp
    }
}
```

- [ ] **Step 4: Register in `src/components/mod.rs`**

```rust
pub mod button;
pub mod shared;
```

- [ ] **Step 5: Run — expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: `button_click_sets_flag` passes. If `into_galley` / `galley()` signatures differ in 0.34, fix per compiler guidance (the intent: lay out button text, center it, paint).

- [ ] **Step 6: Add a visual snapshot of all variants**

Append to `tests/components.rs`:

```rust
#[test]
fn button_variants_snapshot() {
    use egui_shadcn::components::button::ButtonSize;
    let mut h = Harness::builder().with_size(egui::vec2(520.0, 120.0)).build(|ctx| {
        Theme::dark().apply(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            egui_shadcn::layout::row(ui, 8.0, |ui| {
                ui.add(Button::new("Default"));
                ui.add(Button::new("Secondary").variant(ButtonVariant::Secondary));
                ui.add(Button::new("Outline").variant(ButtonVariant::Outline));
                ui.add(Button::new("Ghost").variant(ButtonVariant::Ghost));
                ui.add(Button::new("Delete").variant(ButtonVariant::Destructive));
                let _ = ButtonSize::Sm;
            });
        });
    });
    h.run();
    h.snapshot("button_variants");
}
```

Run: `CARGO_BUILD_JOBS=4 UPDATE_SNAPSHOTS=1 cargo test --jobs 4 --test components 2>&1 | tail -20`
Then Read `tests/snapshots/button_variants.png`. Confirm: filled light "Default", gray "Secondary", bordered "Outline", borderless "Ghost", red "Delete" — all with 8px rounded corners and Oxanium text. Adjust colors/padding if off; re-run with `UPDATE_SNAPSHOTS=1`.

- [ ] **Step 7: Run normally + commit**

```bash
CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -5
git add src/components/button.rs src/components/mod.rs tests/components.rs tests/snapshots/button_variants.png
git commit -m "feat: shadcn Button (variants x sizes)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 8: Label + Input

**Files:**
- Create: `src/components/label.rs`, `src/components/input.rs`
- Modify: `src/components/mod.rs`
- Test: `tests/components.rs` (extend)

- [ ] **Step 1: Extend `tests/components.rs`**

```rust
#[test]
fn input_accepts_text() {
    use egui_shadcn::components::input::Input;
    let mut text = String::new();
    let mut h = Harness::new(|ctx| {
        Theme::dark().apply(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(Input::new(&mut text).hint("Email"));
        });
    });
    h.run();
    // smoke: built without panic and the field exists
    assert!(h.ctx.screen_rect().width() > 0.0);
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: unresolved `components::input`.

- [ ] **Step 3: Implement `src/components/label.rs`**

```rust
//! shadcn Label: small medium-weight text using the muted-aware foreground.

use crate::theme::FAMILY_MEDIUM;
use egui::{FontFamily, RichText, Ui};

pub fn label(ui: &mut Ui, text: &str) -> egui::Response {
    ui.label(RichText::new(text).family(FontFamily::Name(FAMILY_MEDIUM.into())).size(14.0))
}

pub fn description(ui: &mut Ui, text: &str) -> egui::Response {
    let muted = crate::Theme::current(ui.ctx()).palette.muted_foreground;
    ui.label(RichText::new(text).size(14.0).color(muted))
}
```

- [ ] **Step 4: Implement `src/components/input.rs`**

```rust
//! shadcn Input: a single-line text field themed with the input border,
//! 36px height, rounded-md, focus ring.

use crate::components::shared::focus_ring;
use crate::Theme;
use egui::{Margin, TextEdit, Ui, Vec2, Widget};

pub struct Input<'a> {
    text: &'a mut String,
    hint: String,
    password: bool,
}

impl<'a> Input<'a> {
    pub fn new(text: &'a mut String) -> Self {
        Self { text, hint: String::new(), password: false }
    }
    pub fn hint(mut self, h: impl Into<String>) -> Self {
        self.hint = h.into();
        self
    }
    pub fn password(mut self, yes: bool) -> Self {
        self.password = yes;
        self
    }
}

impl<'a> Widget for Input<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::current(ui.ctx());
        let corner = egui::CornerRadius::same(t.radius_md() as u8);
        let resp = ui.add_sized(
            Vec2::new(ui.available_width().min(280.0), 36.0),
            TextEdit::singleline(self.text)
                .hint_text(self.hint)
                .password(self.password)
                .margin(Margin::symmetric(12, 8))
                .background_color(t.palette.input)
                .text_color(t.palette.foreground),
        );
        // Border (egui's TextEdit frame is replaced by our own outline).
        ui.painter().rect_stroke(
            resp.rect,
            corner,
            egui::Stroke::new(1.0, t.palette.border),
            egui::StrokeKind::Inside,
        );
        focus_ring(ui, &resp, &t, t.radius_md());
        resp
    }
}
```

- [ ] **Step 5: Register in `src/components/mod.rs`**

```rust
pub mod button;
pub mod input;
pub mod label;
pub mod shared;
```

- [ ] **Step 6: Run — expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: pass. If `TextEdit::margin`/`background_color` signatures differ, adapt (intent: 12/8 padding, input-colored bg).

- [ ] **Step 7: Commit**

```bash
git add src/components/label.rs src/components/input.rs src/components/mod.rs tests/components.rs
git commit -m "feat: Label + Input components

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 9: Switch + Checkbox

**Files:**
- Create: `src/components/switch.rs`, `src/components/checkbox.rs`
- Modify: `src/components/mod.rs`
- Test: `tests/components.rs` (extend)

- [ ] **Step 1: Extend `tests/components.rs`**

```rust
#[test]
fn switch_toggles() {
    use egui_shadcn::components::switch::toggle;
    use std::cell::Cell;
    let on = Cell::new(false);
    let mut h = Harness::new(|ctx| {
        Theme::dark().apply(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut v = on.get();
            if toggle(ui, &mut v).clicked() {
                on.set(v);
            }
        });
    });
    h.run();
    // click at the switch center
    h.get_by_role(egui_kittest::kittest::Role::Checkbox).click();
    h.run();
    assert!(on.get(), "switch did not toggle on");
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: unresolved `components::switch`. If `get_by_role`/`Role` path differs, the subagent substitutes the correct kittest query (fallback: `h.get_by_label`).

- [ ] **Step 3: Implement `src/components/switch.rs`**

```rust
//! shadcn Switch: a pill track + sliding round thumb. ~32x18px.

use crate::Theme;
use egui::{Color32, Sense, Stroke, Ui, Vec2};

pub fn toggle(ui: &mut Ui, on: &mut bool) -> egui::Response {
    let t = Theme::current(ui.ctx());
    let size = Vec2::new(32.0, 18.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let how_on = ui.ctx().animate_bool(resp.id, *on);
    let track = if *on { t.palette.primary } else { t.palette.input };
    let radius = rect.height() / 2.0;
    ui.painter().rect_filled(rect, radius, track);
    let knob_r = rect.height() / 2.0 - 2.0;
    let cx = egui::lerp((rect.left() + knob_r + 2.0)..=(rect.right() - knob_r - 2.0), how_on);
    ui.painter()
        .circle(egui::pos2(cx, rect.center().y), knob_r, t.palette.background, Stroke::NONE);
    let _ = Color32::WHITE;
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *on, ""));
    resp
}
```

- [ ] **Step 4: Implement `src/components/checkbox.rs`**

```rust
//! shadcn Checkbox: 16px square, rounded, filled with primary when checked.

use crate::Theme;
use egui::{Sense, Stroke, StrokeKind, Ui, Vec2};

pub fn checkbox(ui: &mut Ui, checked: &mut bool) -> egui::Response {
    let t = Theme::current(ui.ctx());
    let size = Vec2::splat(16.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }
    let corner = egui::CornerRadius::same(4);
    if *checked {
        ui.painter().rect_filled(rect, corner, t.palette.primary);
        // checkmark
        let c = rect.shrink(3.5);
        let stroke = Stroke::new(2.0, t.palette.primary_foreground);
        ui.painter().line_segment(
            [egui::pos2(c.left(), c.center().y), egui::pos2(c.center().x - 1.0, c.bottom())],
            stroke,
        );
        ui.painter().line_segment(
            [egui::pos2(c.center().x - 1.0, c.bottom()), egui::pos2(c.right(), c.top())],
            stroke,
        );
    } else {
        ui.painter().rect_stroke(rect, corner, Stroke::new(1.0, t.palette.border), StrokeKind::Inside);
    }
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *checked, ""));
    resp
}
```

- [ ] **Step 5: Register in `src/components/mod.rs`**

```rust
pub mod button;
pub mod checkbox;
pub mod input;
pub mod label;
pub mod shared;
pub mod switch;
```

- [ ] **Step 6: Run — expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add src/components/switch.rs src/components/checkbox.rs src/components/mod.rs tests/components.rs
git commit -m "feat: Switch + Checkbox components

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 10: Tabs

**Files:**
- Create: `src/components/tabs.rs`
- Modify: `src/components/mod.rs`
- Test: `tests/components.rs` (extend)

- [ ] **Step 1: Extend `tests/components.rs`**

```rust
#[test]
fn tabs_select() {
    use egui_shadcn::components::tabs::tab_bar;
    use std::cell::Cell;
    let active = Cell::new(0usize);
    let mut h = Harness::new(|ctx| {
        Theme::dark().apply(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut sel = active.get();
            tab_bar(ui, &mut sel, &["Account", "Notifications", "Display"]);
            active.set(sel);
        });
    });
    h.run();
    h.get_by_label("Notifications").click();
    h.run();
    assert_eq!(active.get(), 1);
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: unresolved `components::tabs`.

- [ ] **Step 3: Implement `src/components/tabs.rs`**

```rust
//! shadcn Tabs: a muted track with the active trigger as a raised pill.

use crate::Theme;
use egui::{Align2, CornerRadius, FontId, Sense, Ui, Vec2};

/// Renders a tab bar. `active` is the selected index; returns true if it changed.
pub fn tab_bar(ui: &mut Ui, active: &mut usize, labels: &[&str]) -> bool {
    let t = Theme::current(ui.ctx());
    let changed = std::cell::Cell::new(false);
    let pad = 3.0;
    let trigger_h = 30.0;
    let bar_h = trigger_h + pad * 2.0;

    let total_w = ui.available_width().min(520.0);
    let (rect, _resp) = ui.allocate_exact_size(Vec2::new(total_w, bar_h), Sense::hover());
    // track
    ui.painter().rect_filled(rect, CornerRadius::same(t.radius_lg() as u8), t.palette.muted);

    let inner = rect.shrink(pad);
    let n = labels.len().max(1) as f32;
    let tw = inner.width() / n;
    for (i, lbl) in labels.iter().enumerate() {
        let tr = egui::Rect::from_min_size(
            egui::pos2(inner.left() + tw * i as f32, inner.top()),
            Vec2::new(tw, trigger_h),
        );
        let resp = ui.interact(tr, ui.id().with(("tab", i)), Sense::click());
        if resp.clicked() && *active != i {
            *active = i;
            changed.set(true);
        }
        let is_active = *active == i;
        if is_active {
            ui.painter().rect_filled(tr, CornerRadius::same(t.radius_md() as u8), t.palette.background);
        }
        let color = if is_active { t.palette.foreground } else { t.palette.muted_foreground };
        ui.painter().text(
            tr.center(),
            Align2::CENTER_CENTER,
            lbl,
            FontId::proportional(14.0),
            color,
        );
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, lbl));
    }
    changed.get()
}
```

- [ ] **Step 4: Register in `src/components/mod.rs`**

```rust
pub mod button;
pub mod checkbox;
pub mod input;
pub mod label;
pub mod shared;
pub mod switch;
pub mod tabs;
```

- [ ] **Step 5: Run — expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: `tabs_select` passes. (kittest clicks the rect carrying the "Notifications" widget-info label.)

- [ ] **Step 6: Commit**

```bash
git add src/components/tabs.rs src/components/mod.rs tests/components.rs
git commit -m "feat: Tabs component (raised-pill active state)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 11: Card section helpers + Separator + Badge + Select

**Files:**
- Create: `src/components/card.rs`, `src/components/separator.rs`, `src/components/badge.rs`, `src/components/select.rs`
- Modify: `src/components/mod.rs`
- Test: `tests/components.rs` (extend)

- [ ] **Step 1: Extend `tests/components.rs`**

```rust
#[test]
fn select_and_badge_render() {
    use egui_shadcn::components::badge::{badge, BadgeVariant};
    use egui_shadcn::components::select::select;
    let mut h = Harness::new(|ctx| {
        Theme::dark().apply(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut choice = 0usize;
            select(ui, "lang", &mut choice, &["English", "German"]);
            badge(ui, "New", BadgeVariant::Default);
        });
    });
    h.run();
    assert!(h.ctx.screen_rect().width() > 0.0);
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: unresolved modules.

- [ ] **Step 3: Implement `src/components/separator.rs`**

```rust
//! shadcn Separator: a 1px border-colored divider.

use crate::Theme;
use egui::Ui;

pub fn separator(ui: &mut Ui) {
    let t = Theme::current(ui.ctx());
    let prev = ui.visuals().widgets.noninteractive.bg_stroke;
    ui.visuals_mut().widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, t.palette.border);
    ui.separator();
    ui.visuals_mut().widgets.noninteractive.bg_stroke = prev;
}
```

- [ ] **Step 4: Implement `src/components/badge.rs`**

```rust
//! shadcn Badge: small pill, text-xs, rounded-full.

use crate::Theme;
use egui::{Align2, CornerRadius, FontId, Sense, Ui, Vec2};

#[derive(Clone, Copy)]
pub enum BadgeVariant {
    Default,
    Secondary,
    Destructive,
    Outline,
}

pub fn badge(ui: &mut Ui, text: &str, variant: BadgeVariant) -> egui::Response {
    let t = Theme::current(ui.ctx());
    let font = FontId::proportional(12.0);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE);
    let size = Vec2::new(galley.size().x + 16.0, 20.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let p = &t.palette;
    let (fill, fg, border) = match variant {
        BadgeVariant::Default => (p.primary, p.primary_foreground, None),
        BadgeVariant::Secondary => (p.secondary, p.secondary_foreground, None),
        BadgeVariant::Destructive => (p.destructive, egui::Color32::WHITE, None),
        BadgeVariant::Outline => (egui::Color32::TRANSPARENT, p.foreground, Some(p.border)),
    };
    let radius = CornerRadius::same((rect.height() / 2.0) as u8);
    if fill != egui::Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, radius, fill);
    }
    if let Some(b) = border {
        ui.painter().rect_stroke(rect, radius, egui::Stroke::new(1.0, b), egui::StrokeKind::Inside);
    }
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, text, font, fg);
    resp
}
```

- [ ] **Step 5: Implement `src/components/select.rs` (themed ComboBox)**

```rust
//! shadcn Select: a styled egui ComboBox matching Input's border/height.

use crate::Theme;
use egui::Ui;

/// Renders a select. `selected` is the chosen index. Returns true if it changed.
pub fn select(ui: &mut Ui, id: &str, selected: &mut usize, options: &[&str]) -> bool {
    let t = Theme::current(ui.ctx());
    let mut changed = false;
    let current = options.get(*selected).copied().unwrap_or("");
    egui::ComboBox::from_id_salt(id)
        .selected_text(current)
        .width(200.0)
        .show_ui(ui, |ui| {
            for (i, opt) in options.iter().enumerate() {
                if ui.selectable_label(*selected == i, *opt).clicked() {
                    *selected = i;
                    changed = true;
                }
            }
        });
    let _ = t;
    changed
}
```

- [ ] **Step 6: Implement `src/components/card.rs` (section helpers reusing `layout::card`)**

```rust
//! shadcn Card section helpers: title + description headers inside layout::card.

use crate::components::label::description;
use crate::theme::FAMILY_SEMIBOLD;
use egui::{FontFamily, RichText, Ui};

pub fn card_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).family(FontFamily::Name(FAMILY_SEMIBOLD.into())).size(16.0));
}

pub fn card_description(ui: &mut Ui, text: &str) {
    description(ui, text);
}
```

- [ ] **Step 7: Register all in `src/components/mod.rs`**

```rust
pub mod badge;
pub mod button;
pub mod card;
pub mod checkbox;
pub mod input;
pub mod label;
pub mod select;
pub mod separator;
pub mod shared;
pub mod switch;
pub mod tabs;
```

- [ ] **Step 8: Run — expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test components 2>&1 | tail -20`
Expected: pass. Fix any 0.34 API drift (`from_id_salt` vs `from_id_source`, `layout_no_wrap` signature) per compiler.

- [ ] **Step 9: Commit**

```bash
git add src/components tests/components.rs
git commit -m "feat: Card sections + Separator + Badge + Select

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Phase 4 — Reference screen

### Task 12: Settings-form-with-tabs example

**Files:**
- Create: `examples/settings.rs`
- Test: covered by the snapshot in Task 13

- [ ] **Step 1: Implement `examples/settings.rs` (a reusable render fn + an eframe `main`)**

```rust
//! The reference screen: a settings form with tabs. The render function is
//! reused by the snapshot test; `main` lets you run it live (`cargo run --example settings`).

use egui_shadcn::components::button::{Button, ButtonVariant};
use egui_shadcn::components::card::{card_description, card_title};
use egui_shadcn::components::input::Input;
use egui_shadcn::components::label::label;
use egui_shadcn::components::separator::separator;
use egui_shadcn::components::switch::toggle;
use egui_shadcn::components::tabs::tab_bar;
use egui_shadcn::{layout, Theme};

#[derive(Default)]
pub struct SettingsState {
    pub tab: usize,
    pub name: String,
    pub email: String,
    pub marketing_emails: bool,
    pub security_emails: bool,
}

pub fn settings_ui(ctx: &egui::Context, state: &mut SettingsState) {
    let t = Theme::dark();
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(t.palette.background).inner_margin(egui::Margin::same(32)))
        .show(ctx, |ui| {
            layout::vstack(ui, 24.0, |ui| {
                card_title(ui, "Settings");
                card_description(ui, "Manage your account settings and preferences.");
                tab_bar(ui, &mut state.tab, &["Account", "Notifications"]);
                match state.tab {
                    0 => layout::card(ui, &t, |ui| {
                        layout::vstack(ui, 16.0, |ui| {
                            card_title(ui, "Account");
                            card_description(ui, "Update your account details.");
                            separator(ui);
                            layout::vstack(ui, 6.0, |ui| {
                                label(ui, "Name");
                                ui.add(Input::new(&mut state.name).hint("Your name"));
                            });
                            layout::vstack(ui, 6.0, |ui| {
                                label(ui, "Email");
                                ui.add(Input::new(&mut state.email).hint("you@example.com"));
                            });
                            layout::space_between(
                                ui,
                                |_ui| {},
                                |ui| {
                                    let _ = ui.add(Button::new("Save").variant(ButtonVariant::Default));
                                },
                            );
                        });
                    }),
                    _ => layout::card(ui, &t, |ui| {
                        layout::vstack(ui, 16.0, |ui| {
                            card_title(ui, "Notifications");
                            card_description(ui, "Choose what you want to be notified about.");
                            separator(ui);
                            layout::space_between(
                                ui,
                                |ui| {
                                    label(ui, "Marketing emails");
                                },
                                |ui| {
                                    let _ = toggle(ui, &mut state.marketing_emails);
                                },
                            );
                            layout::space_between(
                                ui,
                                |ui| {
                                    label(ui, "Security emails");
                                },
                                |ui| {
                                    let _ = toggle(ui, &mut state.security_emails);
                                },
                            );
                        });
                    }),
                };
            });
        });
}

fn main() -> eframe::Result<()> {
    let mut state = SettingsState::default();
    let native = eframe::NativeOptions::default();
    eframe::run_simple_native("egui-shadcn settings", native, move |ctx, _frame| {
        Theme::dark().apply(ctx);
        settings_ui(ctx, &mut state);
    })
}
```

- [ ] **Step 2: Build the example (don't run the window in CI)**

Run: `CARGO_BUILD_JOBS=4 cargo build --jobs 4 --example settings 2>&1 | tail -20`
Expected: builds cleanly. Fix any API drift surfaced here.

- [ ] **Step 3: Commit**

```bash
git add examples/settings.rs
git commit -m "feat: settings-form-with-tabs reference example

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Phase 5 — Eval + Skill

### Task 13: Snapshot eval for the settings screen

**Files:**
- Create: `tests/settings_snapshot.rs`
- Modify: `examples/settings.rs` is reused — but examples aren't importable from tests. Move the render fn into the library.

- [ ] **Step 1: Move `SettingsState` + `settings_ui` into the library**

Create `src/reference.rs` with the exact contents of the `SettingsState` struct and `settings_ui` fn from Task 12 (drop the `main`). Add `pub mod reference;` to `src/lib.rs`. Then rewrite `examples/settings.rs` to call it:

```rust
use egui_shadcn::reference::{settings_ui, SettingsState};
use egui_shadcn::Theme;

fn main() -> eframe::Result<()> {
    let mut state = SettingsState::default();
    eframe::run_simple_native("egui-shadcn settings", eframe::NativeOptions::default(), move |ctx, _f| {
        Theme::dark().apply(ctx);
        settings_ui(ctx, &mut state);
    })
}
```

- [ ] **Step 2: Write `tests/settings_snapshot.rs`**

```rust
use egui_kittest::Harness;
use egui_shadcn::reference::{settings_ui, SettingsState};
use egui_shadcn::Theme;

#[test]
fn settings_account_tab() {
    let mut state = SettingsState::default();
    let mut h = Harness::builder().with_size(egui::vec2(640.0, 560.0)).build(move |ctx| {
        Theme::dark().apply(ctx);
        settings_ui(ctx, &mut state);
    });
    h.run();
    h.snapshot("settings_account");
}
```

- [ ] **Step 3: Generate + inspect the snapshot**

Run: `CARGO_BUILD_JOBS=4 UPDATE_SNAPSHOTS=1 cargo test --jobs 4 --test settings_snapshot 2>&1 | tail -20`
Read `tests/snapshots/settings_account.png`. **This is the acceptance check.** Confirm it reads as shadcn-grade: dark background, a "Settings" heading, a muted tab track with "Account" raised, a bordered rounded card containing a title/description, a separator, two labeled inputs, and a light "Save" button bottom-right. Iterate on spacing/colors/sizes across `theme.rs`/`layout.rs`/components until it looks right; re-run with `UPDATE_SNAPSHOTS=1` after each change.

- [ ] **Step 4: Lock the snapshot — run normally, expect PASS**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 --test settings_snapshot 2>&1 | tail -20`
Expected: pass against the committed PNG.

- [ ] **Step 5: Full test sweep**

Run: `CARGO_BUILD_JOBS=4 cargo test --jobs 4 2>&1 | tail -25`
Expected: all tests across `color`, `theme`, `layout`, `components`, `settings_snapshot` pass.

- [ ] **Step 6: Commit**

```bash
git add src/reference.rs src/lib.rs examples/settings.rs tests/settings_snapshot.rs tests/snapshots/settings_account.png
git commit -m "feat: settings screen snapshot eval (acceptance test)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 14: SKILL.md + reference tables

**Files:**
- Create: `skills/egui-shadcn/SKILL.md`
- Create: `skills/egui-shadcn/references/token-map.md`
- Create: `skills/egui-shadcn/references/layout-map.md`
- Create: `skills/egui-shadcn/references/component-map.md`
- Create: `skills/egui-shadcn/references/gotchas.md`

- [ ] **Step 1: Write `skills/egui-shadcn/SKILL.md`**

````markdown
---
name: egui-shadcn
description: Use when building or restyling a Rust egui/eframe GUI from a shadcn/web design (screenshot, component code, or description). Vendors a tested shadcn-v4 component module into the project and maps the design onto it, layout-first.
---

# egui-shadcn

Rebuild shadcn-style designs as polished egui GUIs without endless iteration.
The hard part of egui is not styling — it is **layout** (single-pass, no flexbox).
This skill carries a tested component module and a layout-first workflow.

## When to use
A shadcn (or generally web/Tailwind) design needs to become an egui/eframe UI:
a screenshot to recreate, shadcn component code to port, or a described screen
that must look professional.

## Workflow

1. **Vendor-in the registry.** Copy `registry/` (this skill's source-of-truth
   module: `src/color.rs`, `src/theme.rs`, `src/layout.rs`, `src/components/`,
   and `assets/`) into the target project as a module, and add deps:
   `egui = "0.34"`, `eframe = "0.34"`, `egui_extras = { version = "0.34",
   features = ["all_loaders"] }`. Call `Theme::dark().apply(ctx)` (or `light`)
   once per frame at the top of your update. This step is idempotent — skip files
   that already exist and match.
2. **Decompose the design top-down:**
   - App shell → egui panels (`SidePanel`/`TopBottomPanel`/`CentralPanel`).
   - Each region's **layout intent** (stack? row? grid? space-between?).
   - Components + their cva variants.
   - Token deviations from the defaults (usually none).
3. **Map** using the reference tables:
   - `references/layout-map.md` — flexbox/Grid intent → egui helper (**read this
     first; layout is where iteration is lost**).
   - `references/component-map.md` — shadcn component/variant → module widget.
   - `references/token-map.md` — shadcn token → egui field.
4. **Build** from the vendored components. Only drop to raw egui when no helper
   fits — and when you do, add a new helper to `layout.rs`/`components/` rather
   than inlining, so the registry keeps growing.
5. **Verify visually.** Add an `egui_kittest` snapshot test
   (features `["wgpu","snapshot"]`), render to PNG, and **look at the PNG**
   against the design. Iterate on numbers. This is the web-like feedback loop —
   use it instead of guessing.

## Hard rules (see `references/gotchas.md`)
- egui is single-pass: a widget sets its own size before placement; there is no
  `flex-grow` distribution across siblings. Use `egui_extras::StripBuilder`
  (`Size::relative/exact/remainder`) via the `layout` helpers.
- Don't fight it with `available_width()` arithmetic when a helper exists.
- Hover/active = opacity modulation of the base color, never a new hue.
- Apply the theme every frame; read tokens via `Theme::current(ctx)`.

## Acceptance bar
The reference `settings_ui` (settings form with tabs) is the quality yardstick:
match its restraint — subtle 1px borders, one radius, muted palette, 14px text,
consistent focus rings.
````

- [ ] **Step 2: Write `references/token-map.md`**

```markdown
# shadcn token → egui mapping

| shadcn token | egui target |
|---|---|
| `--background` | `Visuals.panel_fill`; `Palette.background` |
| `--foreground` | `Visuals.override_text_color`; `Palette.foreground` |
| `--card` / `--card-foreground` | `layout::card` fill / text |
| `--popover` | ComboBox/menu surfaces (`window_fill`) |
| `--primary` / `-foreground` | Button Default fill / text |
| `--secondary` / `-foreground` | Button Secondary; `widgets.*.bg_fill` |
| `--muted` / `-foreground` | Tabs track; description text |
| `--accent` | Ghost hover fill |
| `--destructive` | Button Destructive fill |
| `--border` | 1px strokes everywhere; `widgets.*.bg_stroke` |
| `--input` | TextEdit bg; switch off-track |
| `--ring` | focus ring (3px @ 50%) |
| `--radius` (10px) | `Theme.radius`; `radius_sm/md/lg/xl` |
| 4px spacing grid | `Spacing.item_spacing`, margins |
| 14px / medium | `text_styles[Body]` + `FAMILY_MEDIUM` |
| shadow-sm | `Frame::shadow` (offset [0,1], blur 3) |

Tokens are OKLCH literals converted by `color::oklch_to_srgb`. Customize by
editing `Palette::light()/dark()` in the vendored `theme.rs`.
```

- [ ] **Step 3: Write `references/layout-map.md`**

```markdown
# Web layout intent → egui helper

egui is single-pass; rebuild layout *intent*, don't translate CSS.

| Web intent | egui approach |
|---|---|
| `display:flex; flex-direction:row; gap:N` | `layout::row(ui, N, |ui| ...)` |
| `flex-direction:column; gap:N` | `layout::vstack(ui, N, |ui| ...)` |
| `justify-content:space-between` | `layout::space_between(ui, left, right)` |
| `flex:1` / `flex-grow` across siblings | `egui_extras::StripBuilder` with `Size::remainder()` per growing cell |
| fixed + fluid columns | `StripBuilder` `Size::exact(w)` + `Size::remainder()` |
| proportional columns (`fr`) | `StripBuilder` `Size::relative(frac)` |
| a `.card` block | `layout::card(ui, &theme, |ui| ...)` |
| label + control form row | `layout::form_row(ui, label_w, "Label", |ui| ...)` |
| app shell (sidebar/topbar/main) | `SidePanel` / `TopBottomPanel` / `CentralPanel` |
| responsive breakpoints | branch on `ui.available_width()` each frame |
| CSS Grid tracks | `StripBuilder` rows × cells (no named lines) |

When no helper fits, add one to `layout.rs` instead of inlining `available_width()`
math.
```

- [ ] **Step 4: Write `references/component-map.md`**

```markdown
# shadcn component → egui widget

| shadcn | module widget |
|---|---|
| `<Button variant size>` | `components::button::Button::new(t).variant(..).size(..)` — variants Default/Destructive/Outline/Secondary/Ghost/Link; sizes Sm/Default/Lg/Icon |
| `<Input>` | `components::input::Input::new(&mut s).hint(..).password(..)` |
| `<Label>` | `components::label::label(ui, "..")` |
| description text | `components::label::description(ui, "..")` |
| `<Card>` + Header/Title/Description | `layout::card` + `components::card::{card_title, card_description}` |
| `<Tabs>` | `components::tabs::tab_bar(ui, &mut active, &[..])` |
| `<Switch>` | `components::switch::toggle(ui, &mut on)` |
| `<Checkbox>` | `components::checkbox::checkbox(ui, &mut checked)` |
| `<Select>` | `components::select::select(ui, id, &mut idx, &[..])` |
| `<Separator>` | `components::separator::separator(ui)` |
| `<Badge variant>` | `components::badge::badge(ui, "..", BadgeVariant::..)` |

Not yet ported (add when needed): Dialog, Popover, Table, Tooltip, DropdownMenu,
Accordion, gradients. Build them as new files under `components/` following the
custom-paint pattern in `button.rs`.
```

- [ ] **Step 5: Write `references/gotchas.md`**

```markdown
# egui gotchas (why web habits fail)

- **Single pass, no negotiation.** A widget commits its size before the parent's
  size is known. There is no `flex-grow` distribution across siblings, no
  intrinsic `min-content`/`max-content`. Use `StripBuilder` for distribution.
- **`gap` is `item_spacing`.** Set it inside the stack closure; it doesn't cascade
  like CSS.
- **First-frame jitter.** Content-derived sizes (Grid, auto tables) can be wrong
  on frame 1 then correct themselves. For snapshot tests call `harness.run()`
  (which steps frames) before snapshotting.
- **`strong()` ≠ bold font.** egui's `strong` brightens color. For weight, use the
  named families `FAMILY_MEDIUM` / `FAMILY_SEMIBOLD`.
- **No CSS transitions.** Animate manually with `ctx.animate_bool(id, on)` and lerp
  (see `switch.rs`).
- **No multi-stop/radial gradients, no per-widget box-shadow stacks.** One `Shadow`
  per `Frame`. Gradients require a hand-built `Mesh`.
- **Apply theme every frame.** `Theme::apply` sets fonts/style; read tokens via
  `Theme::current(ctx)` inside widgets.
```

- [ ] **Step 6: Mirror the registry into the skill**

The skill must ship the vendorable source. Copy the built module into the skill:

```bash
mkdir -p skills/egui-shadcn/registry
cp -r src skills/egui-shadcn/registry/src
cp -r assets skills/egui-shadcn/registry/assets
ls -R skills/egui-shadcn/registry | head -30
```

Add a short `skills/egui-shadcn/registry/README.md` stating these files are the
canonical copy and are kept in sync with the repo `src/`.

- [ ] **Step 7: Commit**

```bash
git add skills/egui-shadcn
git commit -m "feat: SKILL.md + reference tables + vendored registry

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Done criteria

- `CARGO_BUILD_JOBS=4 cargo test --jobs 4` is green (color/theme/layout/components/settings_snapshot).
- `tests/snapshots/settings_account.png` reads as shadcn-grade (visual check).
- `cargo run --example settings` opens the live screen.
- `skills/egui-shadcn/SKILL.md` + four reference tables + vendored `registry/` exist.

## Notes on egui 0.34 API drift

This plan targets egui 0.34. A few method names may differ slightly at execution
time (e.g. `ComboBox::from_id_salt` vs `from_id_source`, `Margin::same(i8)` vs
`f32`, `rect_stroke` `StrokeKind` arg, kittest `get_by_role`/`Role` path,
`Harness::builder().with_size`). When the compiler disagrees, follow it — the
*intent* in each step is the spec; adjust the call to match the installed 0.34.x
API. The snapshot/render loop is the ground truth for visual correctness.
```
