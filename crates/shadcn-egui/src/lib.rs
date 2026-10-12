//! The [shadcn/ui](https://ui.shadcn.com/) design system for egui: the
//! shadcn v4 styles, colour themes, fonts, icon sets and presets, and
//! components drawn as plain egui widgets. Works with any egui backend.
//!
//! Apply a [`Theme`] (or a [`Preset`] from a shadcn preset code) once when
//! the context is created, then use the [`components`] like egui widgets:
//!
//! ```
//! use shadcn_egui::components::button::Button;
//! use shadcn_egui::Theme;
//!
//! fn setup(ctx: &egui::Context) {
//!     Theme::vega_dark().apply(ctx);
//! }
//!
//! fn ui(ui: &mut egui::Ui) {
//!     if ui.add(Button::new("Save changes")).clicked() {
//!         // ...
//!     }
//! }
//! # let ctx = egui::Context::default();
//! # setup(&ctx);
//! # let _ = ctx.run_ui(Default::default(), |u| ui(u));
//! ```
//!
//! Fonts other than Inter and icon sets other than Lucide are Cargo
//! features; the README lists them. The `gallery` example in the git
//! repository (not in the crates.io package) shows every component under
//! any preset: `cargo run --example gallery --features fonts-all,icons-all`.

pub mod color;
pub mod components;
pub mod fonts;
pub mod icons;
pub mod layout;
mod palette;
pub mod preset;
pub mod reference;
pub mod style;
pub mod theme;

pub use preset::{
    BaseColor, FontHeading, FontName, IconLibrary, MenuAccent, MenuColor, Mode, Preset,
    PresetCodeError, RadiusChoice, StyleName, ThemeColor,
};
pub use shadcn_egui_core::*;
pub use style::Style;
pub use theme::{theme, MenuColors, Metrics, Palette, Theme, ThemeFonts};
