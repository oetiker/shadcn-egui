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

/// Light or dark: chosen by the app, not part of a preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Light,
    Dark,
}

/// ui.shadcn.com's `DEFAULT_CONFIG`: Nova, neutral, Inter, Lucide; the same
/// as code `b0` (all indices 0).
impl Default for Preset {
    fn default() -> Self {
        Preset {
            style: StyleName::Nova,
            base_color: BaseColor::Neutral,
            theme: ThemeColor::Neutral,
            chart_color: ThemeColor::Neutral,
            font: FontName::Inter,
            font_heading: FontHeading::Inherit,
            icons: IconLibrary::Lucide,
            radius: RadiusChoice::Default,
            menu_color: MenuColor::Default,
            menu_accent: MenuAccent::Subtle,
        }
    }
}

impl Preset {
    pub fn style(mut self, v: StyleName) -> Self {
        self.style = v;
        self
    }
    pub fn base_color(mut self, v: BaseColor) -> Self {
        self.base_color = v;
        self
    }
    /// Sets the `theme` field; `theme(mode)` resolves the preset.
    pub fn theme_color(mut self, v: ThemeColor) -> Self {
        self.theme = v;
        self
    }
    pub fn chart_color(mut self, v: ThemeColor) -> Self {
        self.chart_color = v;
        self
    }
    pub fn font(mut self, v: FontName) -> Self {
        self.font = v;
        self
    }
    pub fn font_heading(mut self, v: FontHeading) -> Self {
        self.font_heading = v;
        self
    }
    pub fn icons(mut self, v: IconLibrary) -> Self {
        self.icons = v;
        self
    }
    pub fn radius(mut self, v: RadiusChoice) -> Self {
        self.radius = v;
        self
    }
    pub fn menu_color(mut self, v: MenuColor) -> Self {
        self.menu_color = v;
        self
    }
    pub fn menu_accent(mut self, v: MenuAccent) -> Self {
        self.menu_accent = v;
        self
    }
}
