//! shadcn presets: every choice of the shadcn "create" page (style, colours,
//! fonts, icons, radius, menu) and the preset codes the site shows.
//! The enums are generated from shadcn/ui `preset.ts` (`generated.rs`, by
//! `tools/stylegen`); their order is the index inside a preset code on
//! ui.shadcn.com. Codes from shadcn-svelte.com use other lists and are not
//! supported.

mod code;
// Ruling C5: generated, kept out of rustfmt.
#[rustfmt::skip]
mod generated;

pub use code::PresetCodeError;
pub use generated::{
    BaseColor, FontName, IconLibrary, MenuAccent, MenuColor, RadiusChoice, StyleName, ThemeColor,
};

/// The heading font: the body font (upstream `"inherit"`) or another font.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontHeading {
    Inherit,
    Font(FontName),
}

/// A shadcn preset: the choices of the shadcn "create" page. Plain data;
/// [`Preset::theme`] turns it into a [`crate::Theme`]. Light or dark is not
/// part of a preset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    pub style: StyleName,
    pub base_color: BaseColor,
    pub theme: ThemeColor,
    pub chart_color: ThemeColor,
    pub font: FontName,
    pub font_heading: FontHeading,
    pub icons: IconLibrary,
    pub radius: RadiusChoice,
    pub menu_color: MenuColor,
    pub menu_accent: MenuAccent,
}
