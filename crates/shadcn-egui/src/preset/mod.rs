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

use crate::components::icon::LUCIDE_SLOTS;
use crate::style::Style;
use crate::theme::{MenuColors, Palette, Theme};

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
/// [`Preset::theme()`] turns it into a [`crate::Theme`]. Light or dark is not
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
    /// Sets the `theme` field; [`theme(mode)`](Preset::theme()) resolves the
    /// preset.
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

impl Preset {
    /// The [`Theme`] this preset describes. Pure apart from one
    /// `log::warn!` line per value this build cannot show (style Sera, a
    /// font or icon library whose feature is off, the `gray` colours, a
    /// translucent menu colour); each falls back to a usable default, and
    /// the lines repeat on every call.
    pub fn theme(&self, mode: Mode) -> Theme {
        let dark = mode == Mode::Dark;
        let style = if self.style == StyleName::Sera {
            log::warn!("shadcn-egui: style \"sera\" is not available in egui; using \"nova\"");
            StyleName::Nova
        } else {
            self.style
        };
        let ids = self.palette_ids();
        let mut theme = with_style(style_fn(style), self.palette(ids, dark), self.radius_px());
        let inter = crate::fonts::files(FontName::Inter).expect("Inter is always bundled");
        let body = crate::fonts::files(self.font).unwrap_or_else(|| {
            log::warn!("{}", crate::fonts::fallback_warning(self.font, false));
            inter
        });
        let heading = match self.font_heading {
            FontHeading::Inherit => None,
            FontHeading::Font(f) => Some(crate::fonts::files(f).unwrap_or_else(|| {
                log::warn!("{}", crate::fonts::fallback_warning(f, true));
                inter
            })),
        };
        let slots = crate::icons::slots(self.icons).unwrap_or_else(|| {
            log::warn!("{}", crate::icons::fallback_warning(self.icons));
            LUCIDE_SLOTS
        });
        let inverted = matches!(
            self.menu_color,
            MenuColor::Inverted | MenuColor::InvertedTranslucent
        );
        if self.translucent_menu() {
            // egui has no backdrop blur, and alpha without it was rejected:
            // a translucent menu draws as its opaque variant.
            let opaque = if inverted {
                MenuColor::Inverted
            } else {
                MenuColor::Default
            };
            log::warn!(
                "shadcn-egui: menu colour \"{}\" needs a backdrop blur egui lacks; using \"{}\"",
                self.menu_color.id(),
                opaque.id()
            );
        }
        let menu = MenuColors {
            palette: inverted.then(|| self.palette(ids, true)),
            dark: inverted.then_some(true),
        };
        theme = theme.fonts(body, heading).icons(slots).menu(menu);
        theme.dark = dark;
        theme
    }

    /// `self.theme(mode).apply(ctx)`.
    pub fn apply(&self, ctx: &egui::Context, mode: Mode) {
        self.theme(mode).apply(ctx);
    }

    /// The base colour with a palette: `gray`, which ui.shadcn.com codes can
    /// name but its `themes.ts` lacks (index §J), resolves to neutral.
    fn base_id(&self) -> &'static str {
        let base = self.base_color.id();
        if crate::palette::base_radius_px(base).is_some() {
            base
        } else {
            "neutral"
        }
    }

    /// The base colour, theme and chart ids with a palette: a code value
    /// ui.shadcn.com has no palette for (`gray`) falls back with a warning,
    /// the base to neutral, theme and chart to the resolved base.
    fn palette_ids(&self) -> (&'static str, &'static str, &'static str) {
        let base = self.base_id();
        if base != self.base_color.id() {
            let id = self.base_color.id();
            log::warn!("shadcn-egui: base colour \"{id}\" has no palette on ui.shadcn.com; using \"{base}\"");
        }
        let pick = |what: &str, id: &'static str| {
            if crate::palette::is_theme(id) {
                id
            } else {
                log::warn!("shadcn-egui: {what} \"{id}\" has no palette on ui.shadcn.com; using \"{base}\"");
                base
            }
        };
        (
            base,
            pick("theme", self.theme.id()),
            pick("chart colour", self.chart_color.id()),
        )
    }

    fn translucent_menu(&self) -> bool {
        matches!(
            self.menu_color,
            MenuColor::DefaultTranslucent | MenuColor::InvertedTranslucent
        )
    }

    /// Base colour × theme × chart colour (`ids` from `palette_ids`), then
    /// the menu accent.
    fn palette(&self, (base, theme, chart): (&str, &str, &str), dark: bool) -> Palette {
        let p = Palette::from_upstream(base, theme, chart, dark)
            .expect("palette_ids returns ids with palettes");
        // ui.shadcn.com forces Subtle for translucent menus
        // (lib/search-params.ts:197-205); silent, and to_code keeps Bold.
        if self.menu_accent == MenuAccent::Bold && !self.translucent_menu() {
            p.with_bold_menu_accent()
        } else {
            p
        }
    }

    /// Lyra and Sera (as requested, before Sera falls back) draw square
    /// corners, and Rhea turns `Large` into the base colour's radius, as the
    /// shadcn site does (`design-system-provider.svelte`).
    fn radius_px(&self) -> f32 {
        if matches!(self.style, StyleName::Lyra | StyleName::Sera) {
            return 0.0;
        }
        let radius = if self.style == StyleName::Rhea && self.radius == RadiusChoice::Large {
            RadiusChoice::Default
        } else {
            self.radius
        };
        match radius {
            RadiusChoice::Default => {
                crate::palette::base_radius_px(self.base_id()).expect("base_id has a palette")
            }
            RadiusChoice::Px(v) => v,
            r => r.rem().expect("coded radii have a rem value") * 16.0,
        }
    }
}

fn style_fn(s: StyleName) -> fn() -> Style {
    match s {
        StyleName::Nova | StyleName::Sera => Style::nova,
        StyleName::Vega => Style::vega,
        StyleName::Maia => Style::maia,
        StyleName::Lyra => Style::lyra,
        StyleName::Mira => Style::mira,
        StyleName::Luma => Style::luma,
        StyleName::Rhea => Style::rhea,
    }
}

/// Builds the `Style` inside its own stack frame, so `Preset::theme` never
/// holds one (a `Style` is ~100 KB in a debug build; see
/// `preset_theme_builds_on_a_small_stack`).
#[inline(never)]
fn with_style(make: fn() -> Style, palette: Palette, radius: f32) -> Theme {
    Theme::new(make(), palette, radius)
}
