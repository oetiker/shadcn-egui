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
