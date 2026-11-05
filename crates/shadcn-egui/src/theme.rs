//! shadcn v4 (OKLCH) design tokens and the `Theme` (style x palette x radius), materialized for egui.

use crate::color::{oklch_to_srgb as c, oklch_to_srgb_a as ca, Tone};
use crate::style::app::{AppMap, Resolved, ResolvedPair};
use crate::style::{
    pick, AppEnum, AppVariant, Edit, Paint, Radius, Role, Rung, ShadowLevel, SizeEnum, State,
    Style, TextSize, VariantEnum, Weight,
};
use egui::Color32;
use shadcn_egui_core::IconSlots;

/// The semantic shadcn color tokens. Surfaces come in `x` / `x_foreground` pairs.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// `chart-1` … `chart-5`.
    pub chart: [Color32; 5],
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
            chart: [
                c(0.87, 0.0, 0.0),
                c(0.556, 0.0, 0.0),
                c(0.439, 0.0, 0.0),
                c(0.371, 0.0, 0.0),
                c(0.269, 0.0, 0.0),
            ],
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
            ring: c(0.556, 0.0, 0.0), // solid mid-gray (border/input above are translucent white)
            chart: [
                c(0.87, 0.0, 0.0),
                c(0.556, 0.0, 0.0),
                c(0.439, 0.0, 0.0),
                c(0.371, 0.0, 0.0),
                c(0.269, 0.0, 0.0),
            ],
        }
    }
}

impl Palette {
    /// The palette the shadcn site builds for a base colour, a theme and a
    /// chart colour (upstream `buildRegistryTheme`): the `base` colour's
    /// vars, the `theme` on top (a colour theme, or a base name, which
    /// overlays that whole base), then `chart-1..5` from the `chart` theme.
    /// Ids are upstream's (`"neutral"`, `"blue"`); `None` when one is
    /// unknown. Menu accent is separate: [`Palette::with_bold_menu_accent`].
    pub fn from_upstream(base: &str, theme: &str, chart: &str, dark: bool) -> Option<Palette> {
        crate::palette::merge(base, theme, chart, dark)
    }

    /// shadcn `menuAccent: "bold"`: menus highlight with the primary pair.
    pub fn with_bold_menu_accent(mut self) -> Self {
        self.accent = self.primary;
        self.accent_foreground = self.primary_foreground;
        self
    }
}

/// A shadcn theme: a style, the light and the dark palette, the radius,
/// fonts and icons. [`Theme::apply`] installs both modes; the crate follows
/// egui's light/dark choice (`ctx.set_theme(ThemePreference::Light)`, or the
/// system's), and [`Theme::current`] resolves in the context's mode.
/// Cloning is cheap: the style sits behind an `Arc`.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// The shadcn style (vega, nova, …): every component part's recipe.
    pub style: Arc<Style>,
    /// The light mode's colours.
    pub light: Palette,
    /// The dark mode's colours.
    pub dark: Palette,
    /// shadcn `--radius`; the radius scale multiplies from it.
    pub radius: f32,
    pub metrics: Metrics,
    /// Body and heading font; [`Theme::apply`] installs them.
    pub fonts: ThemeFonts,
    /// The icons components draw (default: built-in Lucide).
    pub icons: IconSlots,
    /// Menu content draws in dark mode in both modes (upstream
    /// `menuColor: "inverted"`; today the Select list).
    pub menu_inverted: bool,
    /// The mode colours and `dark:` rules resolve in.
    mode: egui::Theme,
    /// App variants, app sizes and app colours, shared by every clone.
    pub(crate) apps: Arc<AppMap>,
}

/// Size tokens, in logical px. Defaults are shadcn v4's Tailwind classes; the
/// class each one stands for is noted. Components read sizes from here, never
/// from literals, so one change restyles every screen. Control heights,
/// paddings and gaps belong to the [`Style`]. Geometry that belongs to
/// a single widget's drawing (a checkmark path, a knob inset) stays local.
///
/// Override them on a theme value, with struct update syntax for the
/// `Metrics` (a `Theme { .. }` literal does not compile: some fields are
/// private):
///
/// ```
/// use shadcn_egui::{Metrics, Theme};
/// let mut theme = Theme::vega();
/// theme.metrics = Metrics { icon: 20.0, ..Metrics::default() };
/// assert_eq!(theme.metrics.icon, 20.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// `text-xs`: badges, captions (`TextStyle::Small`).
    pub text_xs: f32,
    /// `text-sm`: body text, labels, controls (`TextStyle::Body`/`Button`).
    pub text_sm: f32,
    /// `text-base`: card titles.
    pub text_base: f32,
    /// `text-lg`: dialog titles.
    pub text_lg: f32,
    /// Tailwind's line height that comes with `text-xs`.
    pub line_xs: f32,
    /// Line height of `text-sm`.
    pub line_sm: f32,
    /// Line height of `text-base`.
    pub line_base: f32,
    /// Line height of `text-lg`.
    pub line_lg: f32,
    /// `TextStyle::Monospace`.
    pub text_mono: f32,
    /// `TextStyle::Heading`.
    pub text_heading: f32,
    /// Default maximum width of text fields and selects.
    pub field_max_width: f32,
    /// `size-4`: icons and checkbox boxes.
    pub icon: f32,
    /// Icon stroke width in icon units (Lucide's 24-unit grid draws with 2).
    pub icon_stroke: f32,
    /// `border`: hairline width.
    pub border: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            text_xs: 12.0,
            text_sm: 14.0,
            text_base: 16.0,
            text_lg: 18.0,
            line_xs: 16.0,
            line_sm: 20.0,
            line_base: 24.0,
            line_lg: 28.0,
            text_mono: 13.0,
            text_heading: 20.0,
            field_max_width: 280.0,
            icon: 16.0,
            icon_stroke: 2.0,
            border: 1.0,
        }
    }
}

impl Theme {
    /// A theme from a style, the light and the dark palette and the radius
    /// (`--radius` in px). It resolves in dark mode, egui's fallback, until
    /// [`Theme::current`] gives it the context's mode.
    pub fn new(style: Style, light: Palette, dark: Palette, radius: f32) -> Self {
        // `Arc::new` copies the large `Style` through two stack frames in a
        // debug build; a box moves it to the heap once.
        let style = Arc::from(Box::new(style));
        Self {
            style,
            light,
            dark,
            radius,
            metrics: Metrics::default(),
            fonts: ThemeFonts::inherit(&shadcn_egui_font_inter::FONT),
            icons: crate::icons::lucide::SLOTS,
            menu_inverted: false,
            mode: egui::Theme::Dark,
            apps: Arc::default(),
        }
    }
    /// The vega style with the neutral palettes and a 10 px radius.
    pub fn vega() -> Self {
        Self::new(Style::vega(), Palette::light(), Palette::dark(), 10.0)
    }

    /// This theme resolving colours and `dark:` rules in `mode`: how
    /// [`Theme::current`] gives it the context's mode, and how the crate's
    /// unit tests pick one without a context. Apps switch modes with
    /// `ctx.set_theme(..)`; [`Theme::apply`] installs both modes whatever
    /// this says.
    pub(crate) fn in_mode(mut self, mode: egui::Theme) -> Self {
        self.mode = mode;
        self
    }

    /// The mode this theme resolves in.
    pub fn mode(&self) -> egui::Theme {
        self.mode
    }

    /// Whether this theme resolves in dark mode.
    pub fn is_dark(&self) -> bool {
        self.mode == egui::Theme::Dark
    }

    /// The palette of the mode this theme resolves in.
    pub fn palette(&self) -> &Palette {
        self.palette_of(self.mode)
    }

    /// The palette of `mode`.
    pub fn palette_of(&self, mode: egui::Theme) -> &Palette {
        match mode {
            egui::Theme::Dark => &self.dark,
            egui::Theme::Light => &self.light,
        }
    }

    /// Body font and heading font (`None`: headings use the body font,
    /// upstream `fontHeading: "inherit"`).
    pub fn fonts(mut self, body: &'static FontFiles, heading: Option<&'static FontFiles>) -> Self {
        self.fonts = ThemeFonts {
            body,
            heading: heading.unwrap_or(body),
        };
        self
    }

    /// The icon set components draw from (an icon crate's `SLOTS`).
    pub fn icons(mut self, slots: IconSlots) -> Self {
        self.icons = slots;
        self
    }

    /// Menu content in dark mode in both modes (upstream
    /// `menuColor: "inverted"`).
    pub fn menu_inverted(mut self, inverted: bool) -> Self {
        self.menu_inverted = inverted;
        self
    }

    /// The theme menu content draws with: this one, in dark mode when the
    /// menu is inverted.
    pub fn menu_theme(&self) -> Theme {
        let mut t = self.clone();
        if self.menu_inverted {
            t.mode = egui::Theme::Dark;
        }
        t
    }

    /// Define the app variant `app` from the built-in variant `base`: the
    /// edit gets copies of `base`'s recipes and the style the theme builds
    /// for, and changes what it names; the rest stays `base`'s, in every
    /// state, mode and style. Both modes share it; a later definition of
    /// the same name replaces it.
    ///
    /// ```
    /// use shadcn_egui::components::button::{Button, ButtonVariant};
    /// use shadcn_egui::style::{AppVariant, Paint, Role};
    /// use shadcn_egui::Theme;
    /// # fn f(ui: &mut egui::Ui) {
    /// const DANGER_GHOST: AppVariant<ButtonVariant> = ButtonVariant::app("danger-ghost");
    /// const DANGER_OUTLINE: AppVariant<ButtonVariant> = ButtonVariant::app("danger-outline");
    /// let theme = Theme::vega()
    ///     .variant(DANGER_GHOST, ButtonVariant::Ghost, |r, _style| {
    ///         r.button.text.set(Paint::Role(Role::Destructive, 1.0)); // Ghost's hover colours stay
    ///     })
    ///     // Red where Outline sets a text colour: on hover and when expanded.
    ///     .variant(DANGER_OUTLINE, ButtonVariant::Outline, |r, _| r.button.recolor(Role::Foreground, Role::Destructive));
    /// ui.add(Button::new("Delete").variant(DANGER_GHOST));
    /// # }
    /// ```
    ///
    /// A size is no variant:
    ///
    /// ```compile_fail,E0277
    /// use shadcn_egui::components::button::ButtonVariant;
    /// const WIDE: shadcn_egui::style::AppVariant<ButtonVariant> = ButtonVariant::app("wide");
    /// let _ = shadcn_egui::Theme::vega().size(WIDE, ButtonVariant::Default, |_, _| {});
    /// ```
    pub fn variant<V: VariantEnum>(mut self, app: AppVariant<V>, base: V, edit: Edit<V>) -> Self {
        Arc::make_mut(&mut self.apps).insert(app, base, edit);
        self
    }

    /// Define the app size `app` from the built-in size `base`, as
    /// [`Theme::variant`] does for a variant.
    pub fn size<Z: SizeEnum>(mut self, app: AppVariant<Z>, base: Z, edit: Edit<Z>) -> Self {
        Arc::make_mut(&mut self.apps).insert(app, base, edit);
        self
    }

    /// Define the app colour `role` ([`Role::custom`]) as `light` in light
    /// mode and `dark` in dark mode.
    pub fn color(mut self, role: Role, light: Color32, dark: Color32) -> Self {
        match role {
            Role::Custom(name) => Arc::make_mut(&mut self.apps).set_color(name.name(), light, dark),
            _ => debug_assert!(
                false,
                "Theme::color({role:?}): a palette role, not Role::custom"
            ),
        }
        self
    }

    /// What a component draws a variant or size with: the recipes and state
    /// bits of the built-in case, or of the app case's edited copies.
    pub(crate) fn resolve<X: AppEnum>(&self, x: X) -> Resolved<X> {
        self.apps.resolve(&self.style, x)
    }

    /// The recipes and state bits of a variant and a size that edit the
    /// same parts ([`crate::style::app::AppMap::resolve_pair`]).
    pub(crate) fn resolve_pair<V: AppEnum, Z: AppEnum<Recipes = V::Recipes>>(
        &self,
        v: V,
        z: Z,
    ) -> ResolvedPair<V, Z> {
        self.apps.resolve_pair(&self.style, v, z)
    }

    /// The component state `s` plus this theme's [`State::DARK`], and the
    /// `sm:` flag (the window is at least 640 points wide) and `md:` flag
    /// (768 points), as [`crate::layout::window_at_least`] tests them.
    pub fn state(&self, ctx: &egui::Context, s: State) -> State {
        s.with(State::DARK, self.is_dark())
            .or(crate::layout::breakpoints(ctx))
    }

    /// A radius rung in px: shadcn's scale multiplies `--radius`.
    pub fn rung(&self, r: Rung) -> f32 {
        let k = match r {
            Rung::None => 0.0,
            Rung::Sm => 0.6,
            Rung::Md => 0.8,
            Rung::Lg => 1.0,
            Rung::Xl => 1.4,
            Rung::Xl2 => 1.8,
            Rung::Xl3 => 2.2,
            Rung::Xl4 => 2.6,
            Rung::Full => return f32::INFINITY,
        };
        self.radius * k
    }
    /// Corner radius in px, clamped to `max` (half the part's short side)
    /// and to egui's `u8` corner radius.
    pub fn radius_px(&self, r: Radius, max: f32) -> f32 {
        let v = match r {
            Radius::Rung(r) => self.rung(r),
            Radius::Px(v) => v,
            Radius::Min(r, v) => self.rung(r).min(v),
        };
        v.min(max).clamp(0.0, 255.0)
    }
    pub fn radius_sm(&self) -> f32 {
        self.rung(Rung::Sm)
    }
    pub fn radius_md(&self) -> f32 {
        self.rung(Rung::Md)
    }
    pub fn radius_lg(&self) -> f32 {
        self.rung(Rung::Lg)
    }
    pub fn radius_xl(&self) -> f32 {
        self.rung(Rung::Xl)
    }

    /// The palette colour of a style role.
    pub fn role(&self, r: Role) -> Color32 {
        let p = self.palette();
        match r {
            Role::Background => p.background,
            Role::Foreground => p.foreground,
            Role::Card => p.card,
            Role::CardForeground => p.card_foreground,
            Role::Popover => p.popover,
            Role::PopoverForeground => p.popover_foreground,
            Role::Primary => p.primary,
            Role::PrimaryForeground => p.primary_foreground,
            Role::Secondary => p.secondary,
            Role::SecondaryForeground => p.secondary_foreground,
            Role::Muted => p.muted,
            Role::MutedForeground => p.muted_foreground,
            Role::Accent => p.accent,
            Role::AccentForeground => p.accent_foreground,
            Role::Destructive => p.destructive,
            Role::DestructiveForeground => p.destructive_foreground,
            Role::Border => p.border,
            Role::Input => p.input,
            Role::Ring => p.ring,
            Role::White => Color32::WHITE,
            Role::Black => Color32::BLACK,
            Role::Transparent => Color32::TRANSPARENT,
            Role::Custom(name) => {
                let c = self.apps.color(name.name(), self.is_dark());
                debug_assert!(
                    c.is_some(),
                    "app colour {name:?} is not defined in the theme"
                );
                c.unwrap_or(p.foreground)
            }
        }
    }
    /// A style colour as egui paints it. Alpha scales the premultiplied colour,
    /// which matches CSS `/alpha` compositing in sRGB.
    pub fn paint(&self, p: Paint) -> Color32 {
        self.tone(p).color
    }
    /// [`Theme::paint`] with the unclipped colour it stands for, which a
    /// fade or mix takes: the role's token at the paint's alpha.
    pub(crate) fn tone(&self, p: Paint) -> Tone {
        match p {
            Paint::Role(r, a) if a >= 1.0 => Tone::of(self.role(r)),
            Paint::Role(r, a) => Tone::of(self.role(r)).faded(a),
            Paint::Mix(a, b, t) => {
                Tone::mix_oklch(Tone::of(self.role(a)), Tone::of(self.role(b)), t)
            }
        }
    }
    /// A text size in px, from [`Metrics`].
    pub fn text_px(&self, s: TextSize) -> f32 {
        let m = &self.metrics;
        match s {
            TextSize::Xs => m.text_xs,
            TextSize::Sm => m.text_sm,
            TextSize::Base => m.text_base,
            TextSize::Lg => m.text_lg,
            TextSize::Px(v) => v,
        }
    }
    /// The line height Tailwind pairs with a text size (used when a recipe
    /// has no `leading-*`). An arbitrary pixel size keeps the base ratio.
    pub fn line_px(&self, s: TextSize) -> f32 {
        let m = &self.metrics;
        match s {
            TextSize::Xs => m.line_xs,
            TextSize::Sm => m.line_sm,
            TextSize::Base => m.line_base,
            TextSize::Lg => m.line_lg,
            TextSize::Px(v) => v * m.line_base / m.text_base,
        }
    }
    /// Tailwind v4's `shadow-*` scale with every layer, first layer on top
    /// as CSS paints it. [`RecipeBox`](crate::components::shared::RecipeBox)
    /// draws them to the browser's pixels.
    pub fn shadow(&self, s: ShadowLevel) -> &'static [ShadowLayer] {
        const fn l(y: f32, blur: f32, spread: f32, alpha: f32) -> ShadowLayer {
            ShadowLayer::new(egui::vec2(0.0, y), blur, spread, Color32::BLACK, alpha)
        }
        // Tailwind v4 theme.css `--shadow-*`.
        const XS2: &[ShadowLayer] = &[l(1.0, 0.0, 0.0, 0.05)];
        const XS: &[ShadowLayer] = &[l(1.0, 2.0, 0.0, 0.05)];
        const SM: &[ShadowLayer] = &[l(1.0, 3.0, 0.0, 0.1), l(1.0, 2.0, -1.0, 0.1)];
        const MD: &[ShadowLayer] = &[l(4.0, 6.0, -1.0, 0.1), l(2.0, 4.0, -2.0, 0.1)];
        const LG: &[ShadowLayer] = &[l(10.0, 15.0, -3.0, 0.1), l(4.0, 6.0, -4.0, 0.1)];
        const XL: &[ShadowLayer] = &[l(20.0, 25.0, -5.0, 0.1), l(8.0, 10.0, -6.0, 0.1)];
        const XL2: &[ShadowLayer] = &[l(25.0, 50.0, -12.0, 0.25)];
        match s {
            ShadowLevel::None => &[],
            ShadowLevel::Xs2 => XS2,
            ShadowLevel::Xs => XS,
            ShadowLevel::Sm => SM,
            ShadowLevel::Md => MD,
            ShadowLevel::Lg => LG,
            ShadowLevel::Xl => XL,
            ShadowLevel::Xl2 => XL2,
        }
    }
}

/// One layer of a CSS `box-shadow` (outer): offset, blur radius and spread
/// in px, colour and alpha. The blur is CSS's: a Gaussian with a standard
/// deviation of half the blur radius. The alpha is kept apart from the
/// colour so `0.1` stays exact (egui's colours have 8-bit alpha).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ShadowLayer {
    offset: egui::Vec2,
    blur: f32,
    spread: f32,
    color: Color32,
    alpha: f32,
}

impl ShadowLayer {
    /// A layer as CSS writes it: `<x> <y> <blur> <spread> <color> / <alpha>`.
    /// `color`'s own alpha is ignored.
    pub const fn new(
        offset: egui::Vec2,
        blur: f32,
        spread: f32,
        color: Color32,
        alpha: f32,
    ) -> Self {
        Self {
            offset,
            blur,
            spread,
            color,
            alpha,
        }
    }
    /// The x and y offset in px.
    #[inline]
    pub fn offset(&self) -> egui::Vec2 {
        self.offset
    }
    /// The blur radius in px (CSS's; the Gaussian's deviation is half).
    #[inline]
    pub fn blur(&self) -> f32 {
        self.blur
    }
    /// The spread in px; negative shrinks the shadow.
    #[inline]
    pub fn spread(&self) -> f32 {
        self.spread
    }
    /// The colour; its alpha is not used.
    #[inline]
    pub fn color(&self) -> Color32 {
        self.color
    }
    /// The alpha, 0 to 1.
    #[inline]
    pub fn alpha(&self) -> f32 {
        self.alpha
    }
}

// ---- Font + apply + accessor ----

/// Item spacing of egui-native widgets when the style's tabs recipe sets no gap (`gap-2`).
const DEFAULT_ITEM_GAP: f32 = 8.0;

use egui::{FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle};
use std::sync::Arc;

fn theme_id() -> egui::Id {
    static ID: std::sync::LazyLock<egui::Id> =
        std::sync::LazyLock::new(|| egui::Id::unique("shadcn_egui::active_theme"));
    *ID
}

use egui::epaint::text::VariationCoords;
use shadcn_egui_core::{FontFiles, FontKind, WEIGHTS};

/// A theme's fonts. `heading == body` is upstream `fontHeading: "inherit"`.
/// Two values are equal when they name the same fonts.
#[derive(Clone, Copy)]
pub struct ThemeFonts {
    pub body: &'static FontFiles,
    pub heading: &'static FontFiles,
}

impl ThemeFonts {
    /// Headings in the body font.
    pub fn inherit(body: &'static FontFiles) -> Self {
        Self {
            body,
            heading: body,
        }
    }
}

impl PartialEq for ThemeFonts {
    fn eq(&self, other: &Self) -> bool {
        self.body.name == other.body.name && self.heading.name == other.heading.name
    }
}

impl std::fmt::Debug for ThemeFonts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThemeFonts")
            .field("body", &self.body.name)
            .field("heading", &self.heading.name)
            .finish()
    }
}

/// Named font families. Body regular is `FontFamily::Proportional`.
pub const FAMILY_MEDIUM: &str = "shadcn-body-medium";
pub const FAMILY_SEMIBOLD: &str = "shadcn-body-semibold";
pub const FAMILY_BOLD: &str = "shadcn-body-bold";
pub const FAMILY_HEADING: &str = "shadcn-heading-regular";
pub const FAMILY_HEADING_MEDIUM: &str = "shadcn-heading-medium";
pub const FAMILY_HEADING_SEMIBOLD: &str = "shadcn-heading-semibold";
pub const FAMILY_HEADING_BOLD: &str = "shadcn-heading-bold";

/// Families per [`WEIGHTS`] entry; `None` is `Proportional`.
const BODY_FAMILIES: [Option<&str>; 4] = [
    None,
    Some(FAMILY_MEDIUM),
    Some(FAMILY_SEMIBOLD),
    Some(FAMILY_BOLD),
];
const HEADING_FAMILIES: [&str; 4] = [
    FAMILY_HEADING,
    FAMILY_HEADING_MEDIUM,
    FAMILY_HEADING_SEMIBOLD,
    FAMILY_HEADING_BOLD,
];

fn weight_index(w: Weight) -> usize {
    match w {
        Weight::Normal => 0,
        Weight::Medium => 1,
        Weight::Semibold => 2,
        Weight::Bold => 3,
    }
}

/// Return the named font family if it's registered (via `Theme::apply`), else
/// fall back to Proportional. egui defers `set_fonts` to the next frame, so on
/// the very first frame after `apply` a named family may not be in the atlas yet.
pub fn family(ctx: &egui::Context, name: &str) -> egui::FontFamily {
    let fam = egui::FontFamily::Name(name.into());
    if ctx.fonts(|f| f.definitions().families.contains_key(&fam)) {
        fam
    } else {
        egui::FontFamily::Proportional
    }
}

/// The body font family for a weight.
pub fn family_for(ctx: &egui::Context, w: Weight) -> egui::FontFamily {
    match BODY_FAMILIES[weight_index(w)] {
        None => egui::FontFamily::Proportional,
        Some(name) => family(ctx, name),
    }
}

/// The heading font family for a weight (upstream `cn-font-heading`).
pub fn heading_family_for(ctx: &egui::Context, w: Weight) -> egui::FontFamily {
    family(ctx, HEADING_FAMILIES[weight_index(w)])
}

/// The font data key of one theme face: `shadcn-{body,heading}-{weight}`.
fn face_key(role: &str, weight: u16) -> String {
    format!("shadcn-{role}-{weight}")
}

/// One face of `files` at weight `w`, with its `wght` coordinate and
/// baseline correction.
fn face_data(files: &FontFiles, w: u16) -> Arc<FontData> {
    let face = files.face(w);
    let mut tweak = egui::FontTweak {
        y_offset_factor: files.y_offset_factor,
        ..Default::default()
    };
    if let Some(v) = face.wght {
        tweak.coords = VariationCoords::new([(b"wght", v)]);
    }
    Arc::new(FontData::from_static(face.data).tweak(tweak))
}

/// `base` plus the theme's fonts: one registration per weight with the
/// `wght` coordinate, each family followed by Inter at its weight (a face
/// `shadcn-fallback-{weight}`, unless the family's font is Inter) and then
/// `base`'s Proportional list, so a glyph the font lacks takes Inter's, as
/// Chrome takes another font's (P4), and glyphs Inter lacks too (emoji,
/// CJK) still render. A mono body's face in Monospace is followed by
/// Inter's regular face the same way. A theme's
/// faces and families already in `base` (the context's current
/// definitions) are replaced, so a reinstall neither stacks them nor keeps
/// an earlier theme's. Only the exact keys and families a theme installs
/// count as the theme's: an app's own family that lists a theme face keeps
/// it while the new install still defines it, and an app font named
/// `shadcn-…` is left alone. A mono body puts its regular face first in
/// Monospace, once; `mono_installed` says the last install on this context
/// did so, which makes a leading body face in `base`'s Monospace the
/// theme's, to be removed. Any other is the app's.
fn font_definitions(
    fonts: ThemeFonts,
    mut base: FontDefinitions,
    mono_installed: bool,
) -> FontDefinitions {
    let inherit = fonts.heading == fonts.body;
    let theme_keys: Vec<String> = ["body", "heading", "fallback"]
        .into_iter()
        .flat_map(|role| WEIGHTS.map(|w| face_key(role, w)))
        .collect();
    let is_theme_key = |key: &str| theme_keys.iter().any(|k| k == key);
    let inter = &shadcn_egui_font_inter::FONT;
    let needs_fallback = |files: &FontFiles| files.name != inter.name;
    let fallback_used = needs_fallback(fonts.body) || needs_fallback(fonts.heading);
    // An inherited heading registers no faces of its own, and only a font
    // other than Inter registers Inter's faces as its fallback.
    let defined = |key: &str| {
        if key.starts_with("shadcn-fallback-") {
            fallback_used
        } else {
            !inherit || key.starts_with("shadcn-body-")
        }
    };
    base.font_data.retain(|key, _| !is_theme_key(key));
    let is_theme_family =
        |name: &str| BODY_FAMILIES.contains(&Some(name)) || HEADING_FAMILIES.contains(&name);
    base.families
        .retain(|family, _| !matches!(family, FontFamily::Name(name) if is_theme_family(name)));
    let mono_key = face_key("body", WEIGHTS[0]);
    let mono_fallback = face_key("fallback", WEIGHTS[0]);
    if let Some(mono) = base.families.get_mut(&FontFamily::Monospace) {
        if mono_installed && mono.first() == Some(&mono_key) {
            mono.remove(0);
            if mono.first() == Some(&mono_fallback) {
                mono.remove(0);
            }
        }
    }
    for list in base.families.values_mut() {
        list.retain(|key| !is_theme_key(key) || defined(key));
    }
    let mut fallback = base
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    fallback.retain(|key| !is_theme_key(key));
    for (role, files) in [("body", fonts.body), ("heading", fonts.heading)] {
        let data_role = if role == "heading" && inherit {
            "body"
        } else {
            role
        };
        for (i, w) in WEIGHTS.into_iter().enumerate() {
            let key = face_key(data_role, w);
            if data_role == role {
                base.font_data.insert(key.clone(), face_data(files, w));
            }
            let family = match (role, i) {
                ("body", 0) => FontFamily::Proportional,
                ("body", _) => {
                    FontFamily::Name(BODY_FAMILIES[i].expect("named body family").into())
                }
                _ => FontFamily::Name(HEADING_FAMILIES[i].into()),
            };
            let mut list = vec![key];
            if needs_fallback(files) {
                let inter_key = face_key("fallback", w);
                base.font_data
                    .entry(inter_key.clone())
                    .or_insert_with(|| face_data(inter, w));
                list.push(inter_key);
            }
            list.extend(fallback.iter().cloned());
            base.families.insert(family, list);
        }
    }
    if fonts.body.kind == FontKind::Mono {
        let mono = base.families.entry(FontFamily::Monospace).or_default();
        mono.retain(|key| *key != mono_key && *key != mono_fallback);
        let mut lead = vec![mono_key];
        if needs_fallback(fonts.body) {
            lead.push(mono_fallback);
        }
        mono.splice(0..0, lead);
    }
    base
}

/// The fonts last installed on a context by [`Theme::apply`].
#[derive(Clone, PartialEq)]
struct InstalledFonts {
    body: &'static str,
    heading: &'static str,
    /// The body is mono: the install put its face first in Monospace.
    mono_body: bool,
    /// The fonts themselves, for an install that gives way (`gives_way`)
    /// but brings the app's own definitions.
    fonts: ThemeFonts,
}

impl InstalledFonts {
    fn of(fonts: ThemeFonts) -> Self {
        Self {
            body: fonts.body.name,
            heading: fonts.heading.name,
            mono_body: fonts.body.kind == FontKind::Mono,
            fonts,
        }
    }
}

/// Whether an install of `key` gives way to the `installed` fonts: the
/// current viewport applied `key` in this or its previous pass, and some
/// open viewport applied `installed` in its current or previous pass. Two
/// viewports that apply themes with other fonts in every pass (a window in
/// another font than the root), or one viewport that applies two such
/// themes, would else install them in turn in every pass, and each install
/// rebuilds the font atlas, redoes the pass and repaints the other
/// viewports. The fonts of the last real change stay instead. Once no
/// viewport applies `installed` any more (it closed, or stopped), the
/// install goes ahead.
fn gives_way(ctx: &egui::Context, key: &InstalledFonts, installed: &InstalledFonts) -> bool {
    // Outside the closures: `viewport_id` and the pass numbers lock the
    // context. In place: `get_temp` would clone every viewport's list.
    let id = applied_themes_id();
    let ids: Vec<egui::ViewportId> = ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<AppliedThemes>(id)
            .keys()
            .copied()
            .collect()
    });
    let current = ctx.viewport_id();
    let closed = crate::components::shared::closed_viewports(ctx, ids.iter().copied());
    let passes: Vec<(egui::ViewportId, u64)> = ids
        .into_iter()
        .filter(|v| !closed.contains(v))
        .map(|v| (v, ctx.cumulative_pass_nr_for(v)))
        .collect();
    let fonts_of = |t: &Theme| InstalledFonts::of(t.fonts);
    ctx.data_mut(|d| {
        let applied = d.get_temp_mut_or_default::<AppliedThemes>(id);
        let recent = |v: egui::ViewportId, pass: u64, fonts: &InstalledFonts| {
            applied
                .get(&v)
                .is_some_and(|a| a.recent(pass).any(|t| fonts_of(t) == *fonts))
        };
        let mine = passes
            .iter()
            .any(|&(v, pass)| v == current && recent(v, pass, key));
        mine && passes.iter().any(|&(v, pass)| recent(v, pass, installed))
    })
}

/// Whether the context's current fonts bind `family`. Only valid once the
/// context has fonts (`Context::fonts` panics before its first pass).
fn is_bound(ctx: &egui::Context, family: &str) -> bool {
    let fam = FontFamily::Name(family.into());
    ctx.fonts(|f| f.definitions().families.contains_key(&fam))
}

/// Install the theme's fonts. With `base: None` (plain `apply`) nothing
/// happens when the same fonts are installed already, so a light/dark toggle
/// neither rebuilds the atlas nor drops fonts an app added through
/// `apply_with_fonts`. When the app has replaced the fonts since
/// (`ctx.set_fonts`), the theme's families are installed again on top of
/// the app's current fonts, which keeps the app's own families. An
/// install that gives way to the installed fonts (`gives_way`) installs
/// nothing; with `base` it installs the fonts that stay on top of `base`,
/// so the app's own definitions still go in.
fn install_fonts(ctx: &egui::Context, fonts: ThemeFonts, base: Option<FontDefinitions>) {
    let id = egui::Id::unique("shadcn_egui::installed_fonts");
    let mut key = InstalledFonts::of(fonts);
    let mut fonts = fonts;
    let last = ctx.data(|d| d.get_temp::<InstalledFonts>(id));
    if let Some(last) = last
        .as_ref()
        .filter(|last| **last != key && gives_way(ctx, &key, last))
    {
        if base.is_none() {
            return;
        }
        fonts = last.fonts;
        key = last.clone();
    }
    let base = match base {
        Some(base) => base,
        None if last.as_ref() != Some(&key) => FontDefinitions::default(),
        // Before the first pass the fonts are still the ones installed last.
        None if ctx.cumulative_pass_nr() == 0 => return,
        None => {
            let all = BODY_FAMILIES.iter().flatten().chain(&HEADING_FAMILIES);
            if all.into_iter().all(|f| is_bound(ctx, f)) {
                return;
            }
            ctx.fonts(|f| f.definitions().clone())
        }
    };
    let mono_installed = last.is_some_and(|f| f.mono_body);
    let defs = Arc::new(font_definitions(fonts, base, mono_installed));
    let pass = ctx.cumulative_pass_nr();
    // Fonts that change nothing count as no install, or an app that
    // applies the same theme in every pass gets every pass discarded. After
    // the first pass this is egui's own test in `set_fonts`, which then does
    // nothing; before and in the first pass egui may have no fonts yet, so
    // the comparison is with the last install.
    let unchanged = if pass > 0 {
        ctx.fonts(|f| f.definitions() == &*defs)
    } else {
        ctx.data(|d| d.get_temp::<Arc<FontDefinitions>>(last_defs_id()))
            .is_some_and(|last| *last == *defs)
    };
    ctx.data_mut(|d| d.insert_temp(id, key));
    if unchanged {
        if pass == 0 {
            ctx.set_fonts((*defs).clone());
        }
        return;
    }
    ctx.set_fonts((*defs).clone());
    ctx.data_mut(|d| d.insert_temp(last_defs_id(), defs));
    // A pass that has started sees the count change (`check_mode_switch`)
    // and is redone. Where no crate call has seen this pass yet, a call
    // between two passes cannot be told from the first call of a pass:
    // repaint, so a pass that drew in the old fonts is not the last one.
    // egui repaints after the first pass anyway. The fonts are the
    // context's, not the viewport's: every other viewport that drew in the
    // old fonts repaints too.
    let record = pass_mode_id(ctx.viewport_id());
    let seen = ctx.data(|d| {
        d.get_temp::<PassStart>(record)
            .is_some_and(|start| start.pass == pass)
    });
    if pass > 0 && !seen {
        ctx.request_repaint();
    }
    repaint_other_viewports(ctx);
    ctx.data_mut(|d| {
        let n = d.get_temp::<u64>(font_installs_id()).unwrap_or(0);
        d.insert_temp(font_installs_id(), n + 1);
    });
}

/// `TextStyle::Heading` waits in `Proportional` until the heading family is
/// bound: `set_fonts` takes effect only at the start of the next pass, and
/// egui panics on a family the current fonts lack. Holds the stand-in
/// `FontId` of the dark and the light style, which an app may have given
/// different headings, so an app's own later change to a heading is left
/// alone.
#[derive(Clone)]
struct PendingHeading {
    dark: Option<FontId>,
    light: Option<FontId>,
}

impl PendingHeading {
    fn both(id: FontId) -> Self {
        Self {
            dark: Some(id.clone()),
            light: Some(id),
        }
    }

    fn of(&self, theme: egui::Theme) -> Option<&FontId> {
        match theme {
            egui::Theme::Dark => self.dark.as_ref(),
            egui::Theme::Light => self.light.as_ref(),
        }
    }
}

fn pending_heading_id() -> egui::Id {
    egui::Id::unique("shadcn_egui::pending_heading")
}

/// Set once the heading family was seen bound. Every install defines it,
/// but an app's own `ctx.set_fonts` drops it: [`unbind_lost_heading`]
/// clears the flag again.
fn heading_bound_id() -> egui::Id {
    egui::Id::unique("shadcn_egui::heading_bound")
}

fn heading_bound(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(heading_bound_id()).unwrap_or(false))
}

/// The family `TextStyle::Heading` may name now: the heading family once it
/// is known to be bound, else `Proportional`, upgraded by [`bind_heading`]
/// at the start of a later pass ([`ThemeHooks`]).
fn heading_family_now(ctx: &egui::Context) -> FontFamily {
    if heading_bound(ctx) {
        if is_bound(ctx, FAMILY_HEADING_SEMIBOLD) {
            return FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into());
        }
        // The app replaced the fonts; this apply installs the theme's again.
        ctx.data_mut(|d| d.remove::<bool>(heading_bound_id()));
    }
    FontFamily::Proportional
}

/// Once per pass of each viewport: a heading whose family the app's own
/// fonts dropped ([`unbind_lost_heading`]) falls back, a waiting one binds
/// ([`bind_heading`]). Returns the new heading `FontId` when it changed.
/// Each viewport counts its own passes and binds the fonts at the start
/// of each: an immediate viewport's pass, run inside its parent's, may
/// have the number its parent already checked, and fonts its parent's
/// pass does not have yet.
fn check_heading(ctx: &egui::Context) -> Option<FontId> {
    // Outside the closures: `viewport_id` locks the context.
    let id = heading_checked_id(ctx.viewport_id());
    let pass = ctx.cumulative_pass_nr();
    if ctx.data(|d| d.get_temp::<u64>(id)) == Some(pass) {
        return None;
    }
    ctx.data_mut(|d| d.insert_temp(id, pass));
    let fallback = unbind_lost_heading(ctx);
    bind_heading(ctx).or(fallback)
}

/// The pass of the viewport whose heading [`check_heading`] checked last.
fn heading_checked_id(viewport: egui::ViewportId) -> egui::Id {
    egui::Id::unique("shadcn_egui::heading_checked").with(viewport)
}

/// `TextStyle::Heading` back to `Proportional` when the context's fonts no
/// longer bind the heading family: the app called `ctx.set_fonts` after
/// [`Theme::apply`], and egui panics on a family its fonts lack. Each
/// style's heading waits as a [`PendingHeading`] until the family is bound
/// again. Returns the active style's stand-in.
fn unbind_lost_heading(ctx: &egui::Context) -> Option<FontId> {
    if !heading_bound(ctx) || is_bound(ctx, FAMILY_HEADING_SEMIBOLD) {
        return None;
    }
    let warned = egui::Id::unique("shadcn_egui::lost_heading_warned");
    ctx.data_mut(|d| {
        d.remove::<bool>(heading_bound_id());
        if !d.get_temp::<bool>(warned).unwrap_or(false) {
            d.insert_temp(warned, true);
            log::warn!(
                "shadcn-egui: the fonts no longer bind the theme's families \
                 (ctx.set_fonts after Theme::apply?); headings fall back to \
                 Proportional. Use Theme::apply_with_fonts for fonts of your own."
            );
        }
    });
    let name = FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into());
    let fall_back = |theme| {
        let mut stand_in = None;
        ctx.style_mut_of(theme, |s| {
            if let Some(h) = s.text_styles.get_mut(&TextStyle::Heading) {
                if h.family == name {
                    h.family = FontFamily::Proportional;
                    stand_in = Some(h.clone());
                }
            }
        });
        stand_in
    };
    let pending = PendingHeading {
        dark: fall_back(egui::Theme::Dark),
        light: fall_back(egui::Theme::Light),
    };
    if pending.dark.is_none() && pending.light.is_none() {
        return None;
    }
    let active = pending.of(ctx.theme()).cloned();
    ctx.data_mut(|d| d.insert_temp(pending_heading_id(), pending));
    active
}

/// Give `TextStyle::Heading` the heading family once the fonts bind it, in
/// each style that still holds its stand-in. Returns the active style's new
/// heading `FontId` when it changed.
fn bind_heading(ctx: &egui::Context) -> Option<FontId> {
    let pending: PendingHeading = ctx.data(|d| d.get_temp(pending_heading_id()))?;
    let name = FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into());
    if !ctx.fonts(|f| f.definitions().families.contains_key(&name)) {
        return None;
    }
    ctx.data_mut(|d| {
        d.remove::<PendingHeading>(pending_heading_id());
        d.insert_temp(heading_bound_id(), true);
    });
    let mut active = None;
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        let Some(stand_in) = pending.of(theme) else {
            continue;
        };
        let id = FontId::new(stand_in.size, name.clone());
        let mut changed = false;
        ctx.style_mut_of(theme, |s| {
            if s.text_styles.get(&TextStyle::Heading) == Some(stand_in) {
                s.text_styles.insert(TextStyle::Heading, id.clone());
                changed = true;
            }
        });
        if changed && theme == ctx.theme() {
            active = Some(id);
        }
    }
    active
}

impl Theme {
    /// Read the active theme stored by [`Theme::apply`], resolving in the
    /// context's mode (`ctx.theme()`). Falls back to [`Theme::vega`],
    /// shared so the fallback does not build a [`Style`] per call.
    ///
    /// Also binds a heading that still waits for its fonts, and lets one
    /// whose family the app's own fonts dropped fall back, for apps whose
    /// loop does not run egui's pass hooks (`Context::begin_pass`). Needs
    /// the context's fonts, which exist once its first pass has ended. In
    /// such a loop, a heading in a `Ui` built before the first shadcn
    /// component of the pass can still panic after an app's own
    /// `ctx.set_fonts`; use [`Theme::apply_with_fonts`].
    ///
    /// A `ctx.set_theme(..)` in the middle of a pass asks egui to discard
    /// that pass (`Context::run_ui` runs it again) and to repaint, so no
    /// frame shows widgets of both modes. A pass hook checks this at the
    /// end of the pass too; an app's own `begin_pass`/`end_pass` loop runs
    /// no hook, so there only a crate widget after the call catches it.
    /// A switch to a mode the viewport's previous pass also ran in (a
    /// window kept in another mode than the root, a pass that draws in
    /// both modes) is the app's pattern: it discards nothing.
    pub fn current(ctx: &egui::Context) -> Theme {
        static FALLBACK: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(Theme::vega);
        // Before the first pass ends, fonts may not exist yet; the hook binds then.
        if ctx.cumulative_pass_nr() > 0 {
            let _ = check_heading(ctx);
        }
        let mode = check_mode_switch(ctx, Check::Within);
        let theme = ctx.data(|d| d.get_temp::<Theme>(theme_id()));
        theme.unwrap_or_else(|| FALLBACK.clone()).in_mode(mode)
    }

    /// Push this theme into the egui context: fonts, type scale, spacing,
    /// and the colors of both modes (egui's dark and light style). The
    /// crate's widgets and plain egui widgets then follow
    /// `ctx.set_theme(..)` and the system's mode.
    ///
    /// Call once, right after the context is created, and again only when the
    /// theme changes. Not every frame: that would silently undo any other
    /// code's style writes. Fonts are rebuilt from egui's defaults plus the
    /// theme's fonts, and only when they differ from the last install on
    /// this context. **An app that installs fonts of its own must use
    /// [`Theme::apply_with_fonts`]**. A `ctx.set_fonts` of its own after
    /// `apply` drops the theme's families: headings then fall back to
    /// `Proportional` (with a log warning) until a later `apply` of the same
    /// fonts, in a later pass, installs the theme's families on top of the
    /// app's fonts again. In a loop that does not run egui's pass hooks, a
    /// heading in a `Ui` built before the first shadcn component of that pass
    /// can still panic. Calling `apply` and `set_fonts` every frame rebuilds
    /// the font atlas on every other frame. Fonts belong to the context, not
    /// to a viewport: where windows apply themes with different fonts in
    /// every pass, or one pass applies several, all draw in the fonts of the
    /// last change. So does the ink of the text: while windows draw in the
    /// light and the dark mode, all draw their text in the main window's
    /// mode's ink, as egui keeps one font atlas for all windows.
    ///
    /// egui swaps in new fonts only at the next pass. Called in the middle
    /// of a pass, `apply` of other fonts asks egui to discard that pass and
    /// to repaint, as a mid-pass `ctx.set_theme(..)` does; layout that the
    /// crate takes from the previous pass then needs one more pass, so the
    /// first frame of such a change can show it unsettled. Called before
    /// the first pass (in eframe's app creator, on `cc.egui_ctx`), the
    /// first frame is in the theme's fonts. An app's own `ctx.set_fonts`
    /// in a pass is not seen: egui asks for no repaint, so the screen keeps
    /// the old fonts until the next input. Install the fonts through
    /// [`Theme::apply_with_fonts`] instead, or call `ctx.request_repaint()`
    /// after `set_fonts`.
    pub fn apply(&self, ctx: &egui::Context) {
        self.apply_inner(ctx, None);
    }

    /// Like [`Theme::apply`], starting from the app's own font definitions
    /// instead of egui's defaults. Always builds the fonts again from
    /// `base`; egui takes them only when they differ from its own, so the
    /// same call in every pass costs no extra pass.
    pub fn apply_with_fonts(&self, ctx: &egui::Context, base: FontDefinitions) {
        self.apply_inner(ctx, Some(base));
    }

    fn apply_inner(&self, ctx: &egui::Context, base: Option<FontDefinitions>) {
        install_fonts(ctx, self.fonts, base);
        crate::components::icon_atom::install(ctx);
        let m = &self.metrics;
        let hairline = crate::components::shared::line_width(m.border, ctx.pixels_per_point());
        let heading = FontId::new(m.text_heading, heading_family_now(ctx));
        if heading.family == FontFamily::Proportional {
            ctx.data_mut(|d| {
                d.insert_temp(pending_heading_id(), PendingHeading::both(heading.clone()))
            });
        }

        let ink = shared_ink(ctx);
        for mode in [egui::Theme::Dark, egui::Theme::Light] {
            let dark = mode == egui::Theme::Dark;
            let p = self.palette_of(mode);
            ctx.style_mut_of(mode, |s| {
                self.style_egui(s, p, dark, hairline, &heading);
                s.visuals.text_options.color_transfer_function = coverage(ink.unwrap_or(mode));
            });
        }

        // Outside the closures: `viewport_id` locks the context.
        let viewport = ctx.viewport_id();
        let pass = ctx.cumulative_pass_nr();
        forget_closed_viewports(ctx, viewport, pass);
        let (changed, repeat) = ctx.data_mut(|d| {
            let changed = d.get_temp::<Theme>(theme_id()).as_ref() != Some(self);
            let last = d.get_temp::<u64>(generation_id()).unwrap_or(0);
            let made = d.get_temp::<u64>(generations_made_id()).unwrap_or(0);
            // In place: `get_temp` would clone every viewport's list.
            let applied = d.get_temp_mut_or_default::<AppliedThemes>(applied_themes_id());
            let mine = applied.entry(viewport).or_default();
            mine.turn_to(pass);
            // The app's pattern: this viewport applied the theme in this
            // pass or in its previous one already.
            let repeat = mine.generation_of(self).is_some();
            // A theme applied again gets the generation it had, as long as
            // some viewport applied it in its last two passes: an app that
            // applies several themes in every pass (styles side by side, a
            // variant on top of the page's theme, a window in another
            // palette) ends each pass with the generation it started with,
            // however many themes it applies, and `check_mode_switch`
            // discards none of them.
            let generation = if changed {
                applied.values().find_map(|a| a.generation_of(self))
            } else {
                Some(last)
            };
            let new = generation.is_none();
            let generation = generation.unwrap_or(made + 1);
            let mine = applied.entry(viewport).or_default();
            if !mine.now.iter().any(|(t, _)| t == self) {
                mine.now.push((self.clone(), generation));
            }
            if new {
                d.insert_temp(generations_made_id(), generation);
            }
            d.insert_temp(generation_id(), generation);
            d.insert_temp(theme_id(), self.clone());
            d.insert_temp(hairline_id(), hairline);
            d.insert_temp(hairline_id().with(viewport), hairline);
            (changed, repeat)
        });
        // The styles are the context's: other viewports drew in the old
        // ones. A theme this viewport applies in every pass is no change
        // for them: two viewports that each apply their own theme in every
        // pass would else repaint each other in every frame.
        if changed && !repeat {
            repaint_other_viewports(ctx);
        }
        // egui takes the list of pass hooks before a `run_ui` pass, so hooks
        // first registered in a pass do not run at its end: check here, so
        // a font install after the pass's last crate widget redoes the pass
        // as the end check would, and repaint for what that check would
        // catch later in the pass (a `ctx.set_theme(..)`).
        if ctx.plugin_opt::<ThemeHooks>().is_none() {
            ctx.plugin_or_default::<ThemeHooks>();
            if ctx.cumulative_pass_nr() > 0 {
                check_mode_switch(ctx, Check::End);
                ctx.request_repaint();
            }
        }
    }

    /// One mode's egui style: type scale, spacing and colours.
    fn style_egui(
        &self,
        s: &mut egui::Style,
        p: &Palette,
        dark: bool,
        hairline: f32,
        heading: &FontId,
    ) {
        let m = &self.metrics;
        use FontFamily::Proportional;
        s.text_styles = [
            (TextStyle::Small, FontId::new(m.text_xs, Proportional)),
            (TextStyle::Body, FontId::new(m.text_sm, Proportional)),
            (TextStyle::Button, FontId::new(m.text_sm, Proportional)),
            (
                TextStyle::Monospace,
                FontId::new(m.text_mono, FontFamily::Monospace),
            ),
            (TextStyle::Heading, heading.clone()),
        ]
        .into();

        // egui-native widgets follow the style's default button: its
        // height and horizontal padding, and the tabs gap between items.
        // A style without a value gets egui's default, also when an
        // earlier theme had set one.
        let egui_default = egui::style::Spacing::default();
        let button = [&*self.style.button_size_default];
        // Set once, not per pass: no window breakpoint applies.
        let none = State::NONE.with(State::DARK, dark);
        let gap = pick(&[&*self.style.tabs], |r| &r.gap, none).unwrap_or(DEFAULT_ITEM_GAP);
        let pad_x = pick(&button, |r| &r.pad_left, none).unwrap_or(egui_default.button_padding.x);
        let height = pick(&button, |r| &r.height, none);
        // egui's button is its text row plus twice `button_padding.y`, and
        // at least `interact_size.y`: keep the padding within the height.
        let pad_y = height.map_or(egui_default.button_padding.y, |h| {
            ((h - self.line_px(TextSize::Sm)) / 2.0).max(0.0)
        });
        s.spacing.item_spacing = egui::vec2(gap, gap);
        s.spacing.button_padding = egui::vec2(pad_x, pad_y);
        s.spacing.window_margin = egui::Margin::same(0);
        s.spacing.interact_size.y = height.unwrap_or(egui_default.interact_size.y);

        // Plain egui widgets fade as the style's controls do
        // (`disabled:opacity-50` on the button).
        let control = State::NONE.with(State::DARK, dark).or(State::DISABLED);
        let disabled_alpha = pick(&[&*self.style.button], |r| &r.opacity, control);
        let v = &mut s.visuals;
        v.disabled_alpha = disabled_alpha.unwrap_or(v.disabled_alpha);
        v.dark_mode = dark;
        v.panel_fill = p.background;
        v.window_fill = p.card;
        v.window_stroke = Stroke::new(hairline, p.border);
        v.extreme_bg_color = p.input;
        v.faint_bg_color = p.muted;
        v.override_text_color = Some(p.foreground);
        v.hyperlink_color = p.primary;
        v.selection.bg_fill = p.primary.gamma_multiply(0.35);
        v.selection.stroke = Stroke::new(hairline, p.ring);
        // Chrome's caret: 1 css px of the text colour (`caret-color: auto`).
        v.text_cursor.stroke = Stroke::new(1.0_f32, p.foreground);
        v.widgets.noninteractive.bg_fill = p.background;
        v.widgets.noninteractive.bg_stroke = Stroke::new(hairline, p.border);
        v.widgets.noninteractive.fg_stroke = Stroke::new(hairline, p.foreground);
        for w in [
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            w.bg_fill = p.secondary;
            w.weak_bg_fill = p.secondary;
            w.bg_stroke = Stroke::new(hairline, p.border);
            w.fg_stroke = Stroke::new(hairline, p.foreground);
            w.corner_radius = egui::CornerRadius::same(self.radius_md() as u8);
        }
        // shadcn uses the `accent` token for hover/active feedback so
        // native widgets (e.g. ComboBox dropdown items) don't look inert.
        for w in [&mut v.widgets.hovered, &mut v.widgets.active] {
            w.bg_fill = p.accent;
            w.weak_bg_fill = p.accent;
        }
    }
}

fn hairline_id() -> egui::Id {
    egui::Id::unique("shadcn_egui::hairline")
}

/// The ink of Chrome's grayscale anti-aliasing: light text on dark takes
/// `coverage^0.81`, dark text on light the plain coverage
/// (tests/text_coverage.rs). egui's dark default (`2c - c²`) draws 3 to
/// 25 % more.
fn coverage(mode: egui::Theme) -> egui::epaint::FontColorTransferFunction {
    match mode {
        egui::Theme::Dark => egui::epaint::FontColorTransferFunction::Gamma(0.81),
        egui::Theme::Light => egui::epaint::FontColorTransferFunction::Off,
    }
}

/// The mode whose ink both styles carry while open windows run in two
/// modes: the root window's ([`ink_of_windows`]). `None`: each mode its
/// own ink.
fn shared_ink_id() -> egui::Id {
    egui::Id::unique("shadcn_egui::shared_ink")
}

fn shared_ink(ctx: &egui::Context) -> Option<egui::Theme> {
    ctx.data(|d| d.get_temp::<Option<egui::Theme>>(shared_ink_id()))
        .flatten()
}

/// The ink the windows draw their text in: `None` while all of them draw
/// their widgets in one mode, else the root window's mode.
///
/// egui keeps one font atlas per context, and the ink is baked into its
/// glyphs: each viewport's pass that starts with other text options than
/// the last one rebuilds the atlas and drops every laid-out text. A window
/// in another mode than the root would so rebuild it twice per frame, and
/// both windows would draw in the other one's ink. With one ink for both
/// modes the atlas stays, the root window draws in its own mode's ink and
/// only the other windows draw in the root's: a light window beside a dark
/// root draws its text bolder (`coverage^0.81` instead of the coverage), a
/// dark window beside a light root fainter. When the windows draw in one
/// mode again, each mode gets its own ink back (one atlas rebuild). A root
/// window without crate widgets, or a loop that runs no pass hook, keeps
/// each mode's own ink.
fn ink_of_windows(applied: &AppliedThemes) -> Option<egui::Theme> {
    let root = applied.get(&egui::ViewportId::ROOT)?.drawn_in?;
    applied
        .values()
        .any(|a| a.drawn_in.is_some_and(|m| m != root))
        .then_some(root)
}

/// Write the ink of [`ink_of_windows`] into both styles when it changed,
/// and repaint the windows that drew in the old one.
fn set_shared_ink(ctx: &egui::Context, ink: Option<egui::Theme>) {
    let changed = ctx.data_mut(|d| {
        let id = shared_ink_id();
        let changed = d.get_temp::<Option<egui::Theme>>(id).flatten() != ink;
        if changed {
            d.insert_temp(id, ink);
        }
        changed
    });
    if !changed {
        return;
    }
    for mode in [egui::Theme::Dark, egui::Theme::Light] {
        ctx.style_mut_of(mode, |s| {
            s.visuals.text_options.color_transfer_function = coverage(ink.unwrap_or(mode));
        });
    }
    ctx.request_repaint();
    repaint_other_viewports(ctx);
}

/// What a pass started with, as first seen by [`check_mode_switch`].
#[derive(Clone, Copy, PartialEq)]
struct PassStart {
    pass: u64,
    mode: egui::Theme,
    /// [`font_installs`].
    fonts: u64,
    /// [`generation_id`]: the applied theme.
    generation: u64,
    /// Recorded by the begin hook: the end hook checks this pass.
    hooked: bool,
}

/// Where [`check_mode_switch`] runs.
#[derive(Clone, Copy, PartialEq)]
enum Check {
    /// The begin hook of a `run_ui` pass.
    Begin,
    /// A crate call in the pass ([`Theme::current`]).
    Within,
    /// The end hook, or where it does not run yet ([`Theme::apply`]).
    End,
}

/// The [`PassStart`] of the viewport's pass. One per viewport: each
/// counts its own passes, and an immediate viewport runs its passes inside
/// its parent's.
fn pass_mode_id(viewport: egui::ViewportId) -> egui::Id {
    egui::Id::unique("shadcn_egui::pass_mode").with(viewport)
}

/// Repaint every viewport but the current one that has drawn a frame:
/// fonts and styles belong to the context, so a change made in one
/// viewport leaves the others' last frame stale until their next input.
/// egui lists the open viewports in the raw input (eframe fills it); a
/// viewport that has drawn no frame has nothing stale.
fn repaint_other_viewports(ctx: &egui::Context) {
    let current = ctx.viewport_id();
    let mut ids: Vec<egui::ViewportId> = ctx.input(|i| i.raw.viewports.keys().copied().collect());
    ids.extend([egui::ViewportId::ROOT, ctx.parent_viewport_id()]);
    for id in ids {
        if id != current && ctx.cumulative_pass_nr_for(id) > 0 {
            ctx.request_repaint_of(id);
        }
    }
}

/// The font definitions [`install_fonts`] installed last.
fn last_defs_id() -> egui::Id {
    egui::Id::unique("shadcn_egui::last_font_definitions")
}

/// How many times [`install_fonts`] has installed fonts that differ from
/// the ones egui has.
fn font_installs_id() -> egui::Id {
    egui::Id::unique("shadcn_egui::font_installs")
}

fn font_installs(ctx: &egui::Context) -> u64 {
    ctx.data(|d| d.get_temp::<u64>(font_installs_id()).unwrap_or(0))
}

/// The context's mode, checked against the mode its pass started in. A
/// `ctx.set_theme(..)` since then leaves the widgets drawn before it in the
/// other mode: discard the pass, as the browser restyles the whole page at
/// once, and repaint for a loop that cannot redo it. A font install since
/// the pass started does the same: egui swaps in new fonts only at the
/// next pass, so the whole pass is in the old fonts. So does an apply of
/// another theme with the same fonts: the widgets drawn before it keep
/// the old colours. In a pass the begin hook started, only the end hook
/// checks the theme: a pass that ends with the theme it started with is
/// kept, so several themes applied in every pass (styles side by side, a
/// variant on top of the page's theme) cost no extra pass. An app's own
/// loop runs no hook: there a crate call checks. Either way a theme the
/// viewport applied in its previous pass too is the app's pattern, not a
/// switch, and is not discarded: a window that applies another theme than
/// the root in every pass starts each pass with the root's. So is a mode
/// the viewport's previous pass ran in too: the mode belongs to the
/// context, and a window that keeps another mode than the root, or a pass
/// that switches to dark and back, starts each pass in the mode it left
/// last. The end check also sets the ink of all windows
/// ([`ink_of_windows`]). Runs at the
/// start and end of every pass ([`ThemeHooks`]) and in every
/// [`Theme::current`], so an app's own loop, which runs no pass hook, is
/// caught by the next crate widget.
fn check_mode_switch(ctx: &egui::Context, check: Check) -> egui::Theme {
    let mode = ctx.theme();
    let pass = ctx.cumulative_pass_nr();
    let now = PassStart {
        pass,
        mode,
        fonts: font_installs(ctx),
        generation: theme_generation(ctx),
        hooked: check == Check::Begin,
    };
    // Outside the closures: `viewport_id` locks the context.
    let viewport = ctx.viewport_id();
    let record = pass_mode_id(viewport);
    forget_closed_viewports(ctx, viewport, pass);
    let (mode_before, ink) = ctx.data_mut(|d| {
        let applied = d.get_temp_mut_or_default::<AppliedThemes>(applied_themes_id());
        let mine = applied.entry(viewport).or_default();
        mine.turn_to(pass);
        let before = mine.modes_before & mode_bit(mode) != 0;
        mine.modes |= mode_bit(mode);
        if check == Check::Within {
            mine.drawn_in = Some(mode);
        }
        let ink = (check == Check::End).then(|| ink_of_windows(applied));
        (before, ink)
    });
    if let Some(ink) = ink {
        set_shared_ink(ctx, ink);
    }
    match ctx.data(|d| d.get_temp::<PassStart>(record)) {
        Some(start) if start.pass == pass => {
            if start.mode != now.mode && !mode_before {
                ctx.request_discard("shadcn-egui: light/dark mode changed in the pass");
                ctx.request_repaint();
            }
            if start.fonts != now.fonts {
                ctx.request_discard("shadcn-egui: the theme's fonts changed in the pass");
                ctx.request_repaint();
            } else if start.generation != now.generation && (check == Check::End || !start.hooked) {
                // In place: `get_temp` would clone every viewport's list.
                let repeat = ctx.data_mut(|d| {
                    d.get_temp_mut_or_default::<AppliedThemes>(applied_themes_id())
                        .get(&viewport)
                        .is_some_and(|a| a.before(pass, now.generation))
                });
                if !repeat {
                    ctx.request_discard("shadcn-egui: the theme changed in the pass");
                    ctx.request_repaint();
                }
            }
        }
        _ => {
            ctx.data_mut(|d| d.insert_temp(record, now));
        }
    }
    mode
}

/// The generation of the applied theme: one number per theme, so a theme
/// applied again has the number it had (see [`AppliedThemes`]).
fn generation_id() -> egui::Id {
    egui::Id::unique("shadcn_egui::theme_generation")
}

/// The themes each viewport applied in its last two passes, with their
/// generations, and the modes they ran in: [`Theme::apply`] gives a theme
/// found here the number it had, and a theme the viewport applied, or a
/// mode it ran in, in its previous pass too is the app's pattern
/// ([`check_mode_switch`]). Holds no more themes than the app applies in
/// two passes, and keeps every one an app applies in every pass, however
/// many. A viewport egui has closed is dropped
/// ([`forget_closed_viewports`]).
fn applied_themes_id() -> egui::Id {
    egui::Id::unique("shadcn_egui::applied_themes")
}

type AppliedThemes = std::collections::HashMap<egui::ViewportId, Applied>;

/// The themes one viewport applied, and the modes it ran in: in its pass
/// `pass`, and in the pass before it.
#[derive(Clone, Default)]
struct Applied {
    pass: u64,
    now: Vec<(Theme, u64)>,
    before: Vec<(Theme, u64)>,
    /// [`mode_bit`]s.
    modes: u8,
    modes_before: u8,
    /// The mode of the viewport's last crate widget ([`ink_of_windows`]).
    /// Not the mode its pass ends in: an immediate window's mode stays
    /// for the rest of its parent's pass.
    drawn_in: Option<egui::Theme>,
}

fn mode_bit(mode: egui::Theme) -> u8 {
    match mode {
        egui::Theme::Dark => 1,
        egui::Theme::Light => 2,
    }
}

/// Once per pass of the current viewport, before its first record: drop
/// the records of the viewports egui has closed (it forgets a closed
/// viewport, whose pass count then reads 0), so an app that opens and
/// closes windows of new ids keeps no record of each.
fn forget_closed_viewports(ctx: &egui::Context, current: egui::ViewportId, pass: u64) {
    let id = applied_themes_id();
    let ids: Vec<egui::ViewportId> = ctx.data_mut(|d| {
        let applied = d.get_temp_mut_or_default::<AppliedThemes>(id);
        if applied.get(&current).is_some_and(|a| a.pass == pass) {
            return Vec::new();
        }
        applied.keys().copied().collect()
    });
    // Outside the closures: the pass numbers lock the context.
    let closed = crate::components::shared::closed_viewports(ctx, ids);
    if closed.is_empty() {
        return;
    }
    ctx.data_mut(|d| {
        for &v in &closed {
            d.get_temp_mut_or_default::<AppliedThemes>(id).remove(&v);
            d.remove::<PassStart>(pass_mode_id(v));
            d.remove::<u64>(heading_checked_id(v));
            d.remove::<f32>(hairline_id().with(v));
            d.remove::<egui::Id>(crate::components::shared::space_id(v));
        }
    });
}

impl Applied {
    /// Move on to the viewport's pass `pass`: the current list becomes the
    /// previous one when `pass` follows it, and is dropped otherwise.
    fn turn_to(&mut self, pass: u64) {
        if self.pass == pass {
            return;
        }
        let now = std::mem::take(&mut self.now);
        let modes = std::mem::take(&mut self.modes);
        (self.before, self.modes_before) = if self.pass + 1 == pass {
            (now, modes)
        } else {
            (Vec::new(), 0)
        };
        self.pass = pass;
    }

    /// The generation `theme` had when applied in either pass.
    fn generation_of(&self, theme: &Theme) -> Option<u64> {
        let mut all = self.now.iter().chain(&self.before);
        all.find(|(t, _)| t == theme).map(|(_, g)| *g)
    }

    /// The themes the viewport applied in its pass `pass` and the one
    /// before it.
    fn recent(&self, pass: u64) -> impl Iterator<Item = &Theme> {
        let now = (self.pass == pass).then_some(&self.now);
        let before = if self.pass == pass {
            Some(&self.before)
        } else if self.pass + 1 == pass {
            Some(&self.now)
        } else {
            None
        };
        now.into_iter().chain(before).flatten().map(|(t, _)| t)
    }

    /// Whether the viewport applied the theme of `generation` in the pass
    /// before `pass`.
    fn before(&self, pass: u64, generation: u64) -> bool {
        let previous = if self.pass == pass {
            &self.before
        } else if self.pass + 1 == pass {
            &self.now
        } else {
            return false;
        };
        previous.iter().any(|(_, g)| *g == generation)
    }
}

/// The highest generation given out so far.
fn generations_made_id() -> egui::Id {
    egui::Id::unique("shadcn_egui::theme_generations_made")
}

fn theme_generation(ctx: &egui::Context) -> u64 {
    ctx.data(|d| d.get_temp::<u64>(generation_id()).unwrap_or(0))
}

/// What the colours of the context's pass resolve from: the applied
/// theme (its generation, which changes with each [`Theme::apply`] of a
/// different theme) and the mode. A transition that sees it change jumps.
pub(crate) fn theme_stamp(ctx: &egui::Context) -> u64 {
    theme_generation(ctx) * 2 + u64::from(ctx.theme() == egui::Theme::Dark)
}

/// Every stroke [`Theme::apply`] gives the theme's hairline (`border`).
fn set_hairline(v: &mut egui::Visuals, width: f32) {
    v.window_stroke.width = width;
    v.selection.stroke.width = width;
    v.widgets.noninteractive.bg_stroke.width = width;
    v.widgets.noninteractive.fg_stroke.width = width;
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.bg_stroke.width = width;
        w.fg_stroke.width = width;
    }
}

/// The theme's pass hooks, registered once per context by
/// [`Theme::apply`]. A plugin, not a flag in egui's memory: a memory reset
/// (egui's "Reset memory" button) dropped such a flag and the next apply
/// installed a second set of hooks.
///
/// egui runs plugin pass hooks only in `Context::run_ui` and
/// `Context::run_pass`; under `begin_pass`/`end_pass` no hook runs. There the next
/// crate widget catches a mode or font switch through
/// [`Theme::current`], and the hairline follows a scale change only at
/// the next [`Theme::apply`].
#[derive(Default)]
struct ThemeHooks;

impl egui::plugin::Plugin for ThemeHooks {
    fn debug_name(&self) -> &'static str {
        "shadcn_egui::theme"
    }

    /// Keep the hairline of plain egui widgets on whole device pixels, as
    /// the crate's own borders: `Theme::apply` runs once, often before the
    /// window's scale is known, and the scale can change later (zoom,
    /// another monitor). Each pass checks the scale and rewrites the stroke
    /// widths only when the snapped width changes, so style writes of the
    /// app's own stay. Before that, bind a waiting heading, track the
    /// focus modality and keep the text cache's atlas probes in egui's
    /// galley cache; after it, check for a mode switch.
    fn on_begin_pass(&mut self, ui: &mut egui::Ui) {
        #[cfg(test)]
        hook_tests::count();
        if let Some(id) = check_heading(ui.ctx()) {
            // The root ui was built before this hook ran.
            ui.style_mut().text_styles.insert(TextStyle::Heading, id);
        }
        #[cfg(test)]
        hook_tests::count();
        crate::components::shared::focus_modality_keyboard(ui.ctx());
        crate::components::text::keep_probe(ui.ctx());
        #[cfg(test)]
        hook_tests::count();
        if let Some(width) = snap_hairline(ui.ctx()) {
            // The root ui was built before this hook ran.
            set_hairline(&mut ui.style_mut().visuals, width);
        }
        #[cfg(test)]
        hook_tests::count();
        check_mode_switch(ui.ctx(), Check::Begin);
    }

    /// Track the focus modality, then check for a mode switch after the
    /// pass's last crate widget, or before a plain egui `Area`, so the
    /// frame is not mixed or stale.
    fn on_end_pass(&mut self, ui: &mut egui::Ui) {
        #[cfg(test)]
        hook_tests::count();
        crate::components::shared::focus_modality_keyboard(ui.ctx());
        #[cfg(test)]
        hook_tests::count();
        check_mode_switch(ui.ctx(), Check::End);
    }
}

/// The hairline for the current viewport's scale, written to both styles
/// when it differs from the one this viewport last had (a viewport new to
/// the theme starts with the one last written). Returns the new width
/// then. The styles are the context's: with windows at different scales
/// they hold the width of the window whose scale changed last, rather
/// than one rewritten in every pass, which would undo the app's own
/// stroke widths.
fn snap_hairline(ctx: &egui::Context) -> Option<f32> {
    let border = ctx
        .data(|d| d.get_temp::<Theme>(theme_id()))?
        .metrics
        .border;
    let width = crate::components::shared::line_width(border, ctx.pixels_per_point());
    // Outside the closures: `viewport_id` locks the context.
    let mine = hairline_id().with(ctx.viewport_id());
    let (own, last) = ctx.data(|d| (d.get_temp::<f32>(mine), d.get_temp::<f32>(hairline_id())));
    if own.or(last) == Some(width) {
        if own.is_none() {
            ctx.data_mut(|d| d.insert_temp(mine, width));
        }
        return None;
    }
    ctx.data_mut(|d| {
        d.insert_temp(mine, width);
        d.insert_temp(hairline_id(), width);
    });
    ctx.all_styles_mut(|s| set_hairline(&mut s.visuals, width));
    Some(width)
}

/// Convenience accessor for components.
pub fn theme(ctx: &egui::Context) -> Theme {
    Theme::current(ctx)
}

#[cfg(test)]
mod hook_tests {
    use super::Theme;
    use std::cell::Cell;

    thread_local! {
        /// The theme's pass hooks run so far on this thread.
        static RUNS: Cell<u32> = const { Cell::new(0) };
    }

    /// Counts one run of a theme pass hook.
    pub(super) fn count() {
        RUNS.with(|r| r.set(r.get() + 1));
    }

    /// The theme's pass hooks that run in one empty `run_ui` pass.
    fn runs_per_pass(ctx: &egui::Context) -> u32 {
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .drop_without_applying_deltas();
        let before = RUNS.with(Cell::get);
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .drop_without_applying_deltas();
        RUNS.with(Cell::get) - before
    }

    /// A reset of egui's memory (its "Reset memory" button) and a new
    /// `Theme::apply` install no second set of pass hooks.
    #[test]
    fn a_memory_reset_installs_no_second_set_of_hooks() {
        let ctx = egui::Context::default();
        let mut runs = Vec::new();
        for _ in 0..3 {
            Theme::vega().apply(&ctx);
            runs.push(runs_per_pass(&ctx));
            ctx.memory_mut(|m| *m = Default::default());
        }
        // At the start of a pass the heading binding, the focus modality,
        // the hairline and the mode check; at its end the focus modality
        // and the mode check.
        assert_eq!(runs, [6; 3]);
    }
}

#[cfg(test)]
mod viewport_tests {
    use super::Theme;
    use super::{
        applied_themes_id, hairline_id, heading_checked_id, pass_mode_id, AppliedThemes, PassStart,
    };
    use std::cell::{Cell, RefCell};

    fn records(ctx: &egui::Context) -> usize {
        ctx.data_mut(|d| {
            d.get_temp_mut_or_default::<AppliedThemes>(applied_themes_id())
                .len()
        })
    }

    thread_local! {
        /// The scale of the immediate windows the test renderer runs.
        static WINDOW_SCALE: Cell<Option<f32>> = const { Cell::new(None) };
        /// The input events of the next window pass.
        static WINDOW_EVENTS: RefCell<Vec<egui::Event>> = const { RefCell::new(Vec::new()) };
    }

    /// A context whose immediate viewports are real windows, each pass
    /// run with `run_ui` (so the theme's hooks run) at [`WINDOW_SCALE`]
    /// and with the events in [`WINDOW_EVENTS`].
    fn windowed_context() -> egui::Context {
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(false);
        egui::Context::set_immediate_viewport_renderer(|ctx, mut viewport| {
            let id = viewport.ids.this;
            let info = egui::ViewportInfo {
                native_pixels_per_point: WINDOW_SCALE.with(Cell::get),
                ..Default::default()
            };
            let input = egui::RawInput {
                viewport_id: id,
                viewports: [(id, info)].into_iter().collect(),
                events: WINDOW_EVENTS.with(|e| std::mem::take(&mut *e.borrow_mut())),
                ..Default::default()
            };
            ctx.run_ui(input, |ui| (viewport.viewport_ui_cb)(ui))
                .drop_without_applying_deltas();
        });
        ctx
    }

    /// Windows of new ids that open and close one after the other leave
    /// no record behind: the theme's records hold the root and the windows
    /// egui has not closed yet, however many windows came and went.
    #[test]
    fn closed_windows_leave_no_record() {
        let ctx = windowed_context();
        let frame = |open: Option<u32>| {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                Theme::vega().apply(ui.ctx());
                let _ = Theme::current(ui.ctx());
                if let Some(n) = open {
                    let builder = egui::ViewportBuilder::default();
                    ui.ctx()
                        .show_viewport_immediate(window(n), builder, |ui, _| {
                            Theme::vega().apply(ui.ctx());
                            let _ = Theme::current(ui.ctx());
                        });
                }
            })
            .drop_without_applying_deltas();
        };
        let mut most = 0;
        for n in 0..50 {
            frame(Some(n));
            most = most.max(records(&ctx));
        }
        // The root, the open window and the one egui closes at the end of
        // the root's pass.
        assert!(most <= 3, "records with windows coming and going: {most}");
        for _ in 0..2 {
            frame(None);
        }
        assert_eq!(records(&ctx), 1, "records once all windows closed");
        let left = (0..50).filter(|&n| {
            ctx.data(|d| {
                d.get_temp::<PassStart>(pass_mode_id(window(n))).is_some()
                    || d.get_temp::<u64>(heading_checked_id(window(n))).is_some()
                    || d.get_temp::<f32>(hairline_id().with(window(n))).is_some()
            })
        });
        assert_eq!(left.count(), 0, "pass records of closed windows");
    }

    fn window(n: u32) -> egui::ViewportId {
        egui::ViewportId::from_hash_of(("shadcn_egui_test_window", n))
    }

    /// A focusable widget, focused when `focus`; the focus modality after it.
    fn target(ui: &mut egui::Ui, name: &str, focus: bool) -> bool {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(10.0, 10.0));
        let resp = ui.interact(rect, egui::Id::unique(name), egui::Sense::click());
        if focus {
            resp.request_focus();
        }
        crate::components::shared::focus_modality_keyboard(ui.ctx())
    }

    /// A pointer press that focuses a widget in one window leaves the
    /// keyboard ring of the widget focused in another: the browser keeps
    /// `:focus-visible` per document.
    #[test]
    fn a_press_in_one_window_keeps_the_ring_in_another() {
        let ctx = windowed_context();
        let key = egui::Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let press = egui::Event::PointerButton {
            pos: egui::pos2(5.0, 5.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        };
        // (root events, root focuses, window events, window focuses)
        let frames = [
            (vec![key], true, vec![], false),
            (vec![], false, vec![press], true),
            (vec![], false, vec![], false),
            (vec![], false, vec![], false),
        ];
        let mut seen = Vec::new();
        for (root_events, root_focus, window_events, window_focus) in frames {
            WINDOW_EVENTS.with(|e| *e.borrow_mut() = window_events);
            let input = egui::RawInput {
                events: root_events,
                ..Default::default()
            };
            let mut root = false;
            let mut win = false;
            ctx.run_ui(input, |ui| {
                Theme::vega().apply(ui.ctx());
                root = target(ui, "root target", root_focus);
                let builder = egui::ViewportBuilder::default();
                ui.ctx()
                    .show_viewport_immediate(window(0), builder, |ui, _| {
                        win = target(ui, "window target", window_focus);
                    });
            })
            .drop_without_applying_deltas();
            seen.push((root, win));
        }
        // Frame 0: the root's widget focused by a key, the window has no
        // focus (keyboard, as a page that saw no input). From frame 1 the
        // window's widget is focused by a press.
        assert_eq!(
            seen,
            [(true, true), (true, false), (true, false), (true, false)],
            "(root, window) keyboard modality per frame"
        );
    }

    /// With windows at two scales (1x and 1.5x monitors) the hairline is
    /// written when a window's snapped width changes, not in every pass:
    /// a stroke width the app set itself stays.
    #[test]
    fn an_app_stroke_stays_with_windows_at_two_scales() {
        let ctx = windowed_context();
        WINDOW_SCALE.with(|s| s.set(Some(1.5)));
        let frame = || {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                let builder = egui::ViewportBuilder::default();
                ui.ctx()
                    .show_viewport_immediate(window(0), builder, |ui, _| {
                        let _ = Theme::current(ui.ctx());
                    });
            })
            .drop_without_applying_deltas();
        };
        Theme::vega().apply(&ctx);
        for _ in 0..3 {
            frame();
        }
        ctx.all_styles_mut(|s| s.visuals.selection.stroke.width = 3.0);
        for _ in 0..3 {
            frame();
        }
        for mode in [egui::Theme::Dark, egui::Theme::Light] {
            let width = ctx.style_of(mode).visuals.selection.stroke.width;
            assert_eq!(width, 3.0, "the app's selection stroke ({mode:?})");
        }
    }

    fn space(pressed: bool) -> egui::Event {
        egui::Event::Key {
            key: egui::Key::Space,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// A focused `<button>` that acts on keys as upstream's; whether it
    /// clicked in this pass.
    fn space_button(ui: &mut egui::Ui, name: &str) -> bool {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(10.0, 10.0));
        let mut resp = ui.interact(rect, egui::Id::unique(name), egui::Sense::click());
        resp.request_focus();
        crate::components::shared::pressable(
            &mut resp,
            crate::components::shared::ActivationKeys::Button,
        );
        resp.clicked()
    }

    /// Space held on a button in each of two windows at the same time:
    /// each button clicks when Space comes up in its own window, as each
    /// browser tab keeps its own held key.
    #[test]
    fn space_held_in_two_windows_clicks_both_buttons() {
        let ctx = windowed_context();
        // (root events, window events): both press Space, then each
        // releases it in a frame of its own.
        let frames = [
            (vec![space(true)], vec![space(true)]),
            (vec![space(false)], vec![]),
            (vec![], vec![space(false)]),
        ];
        let mut seen = Vec::new();
        for (root_events, window_events) in frames {
            WINDOW_EVENTS.with(|e| *e.borrow_mut() = window_events);
            let input = egui::RawInput {
                events: root_events,
                ..Default::default()
            };
            let mut root = false;
            let mut win = false;
            ctx.run_ui(input, |ui| {
                Theme::vega().apply(ui.ctx());
                root = space_button(ui, "root button");
                let builder = egui::ViewportBuilder::default();
                ui.ctx()
                    .show_viewport_immediate(window(0), builder, |ui, _| {
                        win = space_button(ui, "window button");
                    });
            })
            .drop_without_applying_deltas();
            seen.push((root, win));
        }
        assert_eq!(
            seen,
            [(false, false), (true, false), (false, true)],
            "(root, window) clicks per frame"
        );
    }

    /// Windows of new ids that open and close one after the other, each
    /// with Space held on a button, leave no held-Space record.
    #[test]
    fn closed_windows_leave_no_held_space() {
        let ctx = windowed_context();
        let frame = |open: Option<u32>| {
            WINDOW_EVENTS.with(|e| *e.borrow_mut() = vec![space(true)]);
            ctx.run_ui(egui::RawInput::default(), |ui| {
                Theme::vega().apply(ui.ctx());
                if let Some(n) = open {
                    let builder = egui::ViewportBuilder::default();
                    ui.ctx()
                        .show_viewport_immediate(window(n), builder, |ui, _| {
                            let _ = space_button(ui, "window button");
                        });
                }
            })
            .drop_without_applying_deltas();
        };
        let held = |n: u32| {
            let id = crate::components::shared::space_id(window(n));
            ctx.data(|d| d.get_temp::<egui::Id>(id).is_some())
        };
        for n in 0..50 {
            frame(Some(n));
            assert!(held(n), "window {n} holds Space on its button");
        }
        for _ in 0..2 {
            frame(None);
        }
        let left = (0..50).filter(|&n| held(n)).count();
        assert_eq!(left, 0, "held-Space records of closed windows");
    }

    /// A deferred window in its first pass that shows an immediate window
    /// keeps its records: egui counts the deferred window's first pass at
    /// the pass's end, and the immediate window's crate calls must not take
    /// it for closed and drop its theme record and hit areas mid-pass.
    #[test]
    fn a_window_in_its_first_pass_keeps_its_records() {
        let ctx = windowed_context();
        let deferred = egui::ViewportId::from_hash_of("first-pass window");
        let hit = |ui: &mut egui::Ui| {
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(10.0, 10.0));
            let id = egui::Id::unique("hit");
            let _ = crate::components::shared::hit_area(ui, rect, id, egui::Sense::click());
        };
        for _ in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                Theme::vega().apply(ui.ctx());
                let builder = egui::ViewportBuilder::default();
                ui.ctx()
                    .show_viewport_deferred(deferred, builder, |_, _| {});
            })
            .drop_without_applying_deltas();
        }
        let info = egui::ViewportInfo {
            parent: Some(egui::ViewportId::ROOT),
            ..Default::default()
        };
        let input = egui::RawInput {
            viewport_id: deferred,
            viewports: [(deferred, info)].into_iter().collect(),
            ..Default::default()
        };
        let seen = Cell::new(((0, 0), 0));
        ctx.run_ui(input, |ui| {
            assert_eq!(ui.ctx().cumulative_pass_nr(), 0, "the window's first pass");
            Theme::vega().apply(ui.ctx());
            hit(ui);
            let builder = egui::ViewportBuilder::default();
            ui.ctx()
                .show_viewport_immediate(window(0), builder, |ui, _| {
                    Theme::vega().apply(ui.ctx());
                    hit(ui);
                });
            let hit_areas = crate::components::shared::viewport_records(ui.ctx(), &[deferred]);
            let applied = ui.ctx().data_mut(|d| {
                d.get_temp_mut_or_default::<AppliedThemes>(applied_themes_id())
                    .contains_key(&deferred)
            });
            seen.set((hit_areas, usize::from(applied)));
        })
        .drop_without_applying_deltas();
        assert_eq!(
            seen.get(),
            ((1, 1), 1),
            "((hit-area, modality), theme) records of the window after the immediate one"
        );
    }

    /// Windows of new ids that call `hit_area` leave no hit-area record and
    /// no focus-modality record once egui has closed them.
    #[test]
    fn closed_windows_leave_no_hit_areas() {
        let ctx = windowed_context();
        let hit = |ui: &mut egui::Ui| {
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(10.0, 10.0));
            let id = egui::Id::unique("hit");
            let _ = crate::components::shared::hit_area(ui, rect, id, egui::Sense::click());
        };
        let frame = |open: Option<u32>| {
            ctx.run_ui(egui::RawInput::default(), |ui| {
                Theme::vega().apply(ui.ctx());
                hit(ui);
                if let Some(n) = open {
                    let builder = egui::ViewportBuilder::default();
                    ui.ctx()
                        .show_viewport_immediate(window(n), builder, |ui, _| hit(ui));
                }
            })
            .drop_without_applying_deltas();
        };
        let all: Vec<egui::ViewportId> = std::iter::once(egui::ViewportId::ROOT)
            .chain((0..50).map(window))
            .collect();
        let mut most = (0, 0);
        for n in 0..50 {
            frame(Some(n));
            let (hit, modality) = crate::components::shared::viewport_records(&ctx, &all);
            most = (most.0.max(hit), most.1.max(modality));
        }
        assert!(
            most.0 <= 3 && most.1 <= 3,
            "(hit-area, modality) records with windows coming and going: {most:?}"
        );
        for _ in 0..2 {
            frame(None);
        }
        let left = crate::components::shared::viewport_records(&ctx, &all);
        assert_eq!(
            left,
            (1, 1),
            "(hit-area, modality) records once all windows closed"
        );
    }
}
