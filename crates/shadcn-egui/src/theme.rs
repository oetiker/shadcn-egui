//! shadcn v4 (OKLCH) design tokens and the `Theme` (style x palette x radius), materialized for egui.

use crate::color::{oklch_to_srgb as c, oklch_to_srgb_a as ca};
use crate::style::{pick, Paint, Radius, Role, Rung, ShadowLevel, State, Style, TextSize, Weight};
use egui::Color32;

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
            ring: c(0.556, 0.0, 0.0), // solid mid-gray (border/input above are translucent white)
        }
    }
}

impl Palette {
    /// shadcn `menuAccent: "bold"`: menus highlight with the primary pair.
    pub fn with_bold_menu_accent(mut self) -> Self {
        self.accent = self.primary;
        self.accent_foreground = self.primary_foreground;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// The shadcn style (vega, nova, …): every component part's recipe.
    pub style: Arc<Style>,
    pub palette: Palette,
    /// shadcn `--radius`; the radius scale multiplies from it.
    pub radius: f32,
    pub metrics: Metrics,
    pub dark: bool,
}

/// Size tokens, in logical px. Defaults are shadcn v4's Tailwind classes; the
/// class each one stands for is noted. Components read sizes from here, never
/// from literals, so one change restyles every screen. Control heights,
/// paddings and gaps belong to the [`Style`]. Geometry that belongs to
/// a single widget's drawing (a checkmark path, a knob inset) stays local.
///
/// Override with struct update syntax:
/// `Theme { metrics: Metrics { icon: 20.0, ..Metrics::default() }, ..Theme::vega_dark() }`.
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
    /// `dark` follows the palette's background (OKLCH lightness below 0.5);
    /// set `theme.dark` afterwards to override.
    pub fn new(style: Style, palette: Palette, radius: f32) -> Self {
        let dark = crate::color::srgb_to_oklch(palette.background).0 < 0.5;
        // `Arc::new` copies the large `Style` through two stack frames in a
        // debug build; a box moves it to the heap once.
        let style = Arc::from(Box::new(style));
        Self {
            style,
            palette,
            radius,
            metrics: Metrics::default(),
            dark,
        }
    }
    pub fn vega_dark() -> Self {
        Self::new(Style::vega(), Palette::dark(), 10.0)
    }
    pub fn vega_light() -> Self {
        Self::new(Style::vega(), Palette::light(), 10.0)
    }

    /// The component state `s` plus this theme's [`State::DARK`].
    pub fn state(&self, s: State) -> State {
        s.with(State::DARK, self.dark)
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
        let p = &self.palette;
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
        }
    }
    /// A style colour as egui paints it. Alpha scales the premultiplied colour,
    /// which matches CSS `/alpha` compositing in sRGB.
    pub fn paint(&self, p: Paint) -> Color32 {
        match p {
            Paint::Role(r, a) if a >= 1.0 => self.role(r),
            Paint::Role(r, a) => self.role(r).gamma_multiply(a.max(0.0)),
            Paint::Mix(a, b, t) => crate::color::mix_oklch(self.role(a), self.role(b), t),
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
    /// Tailwind v4 shadows, first layer only (egui draws one); egui's spread
    /// is unsigned, so Tailwind's negative spreads become 0.
    pub fn shadow(&self, s: ShadowLevel) -> egui::epaint::Shadow {
        let (y, blur, a) = match s {
            ShadowLevel::None => return egui::epaint::Shadow::NONE,
            ShadowLevel::Xs2 => (1, 0, 0.05),
            ShadowLevel::Xs => (1, 2, 0.05),
            ShadowLevel::Sm => (1, 3, 0.10),
            ShadowLevel::Md => (4, 6, 0.10),
            ShadowLevel::Lg => (10, 15, 0.10),
            ShadowLevel::Xl => (20, 25, 0.10),
            ShadowLevel::Xl2 => (25, 50, 0.25),
        };
        egui::epaint::Shadow {
            offset: [0, y],
            blur,
            spread: 0,
            color: Color32::from_black_alpha((a * 255.0_f32).round() as u8),
        }
    }
}

// ---- Font + apply + accessor ----

/// Item spacing of egui-native widgets when the style's tabs recipe sets no gap (`gap-2`).
const DEFAULT_ITEM_GAP: f32 = 8.0;

use egui::{FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle};
use std::sync::Arc;

fn theme_id() -> egui::Id {
    static ID: std::sync::LazyLock<egui::Id> =
        std::sync::LazyLock::new(|| egui::Id::new("shadcn_egui::active_theme"));
    *ID
}

/// Named font families for shadcn-style weight emphasis.
pub const FAMILY_MEDIUM: &str = "oxanium-medium";
pub const FAMILY_SEMIBOLD: &str = "oxanium-semibold";

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

/// Font family for a weight; Oxanium ships regular, medium and semibold.
pub fn family_for(ctx: &egui::Context, w: Weight) -> egui::FontFamily {
    match w {
        Weight::Normal => egui::FontFamily::Proportional,
        Weight::Medium => family(ctx, FAMILY_MEDIUM),
        Weight::Semibold | Weight::Bold => family(ctx, FAMILY_SEMIBOLD),
    }
}

// NOTE: The Google Fonts repository only ships Oxanium as a variable font
// (Oxanium[wght].ttf) — there are no separate static weight files any more.
// All three weight slots use the same variable font binary; egui will render
// at one effective weight.  A future improvement would be to use font
// variation settings once egui gains that support.
fn install_fonts(ctx: &egui::Context) {
    // Building the font atlas is expensive, so install the fonts only once per
    // context; re-applying (e.g. a light/dark toggle) only refreshes the style.
    let installed_id = egui::Id::new("shadcn_egui::fonts_installed");
    if ctx
        .data(|d| d.get_temp::<bool>(installed_id))
        .unwrap_or(false)
    {
        return;
    }
    // Oxanium's ascent reserves more empty space above the cap height than below
    // the baseline, so egui's row-box centering (`Align2::CENTER_CENTER` in
    // buttons/badges/tabs, and `TextEdit` vertical centering) leaves single-line
    // text sitting ~0.18em too high. Nudge every glyph down by that fraction so
    // centered text is optically balanced. Measured against the reference button.
    let tweak = egui::FontTweak {
        y_offset_factor: 0.18,
        ..Default::default()
    };
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "oxanium".into(),
        Arc::new(
            FontData::from_static(include_bytes!("../assets/Oxanium-Regular.ttf"))
                .tweak(tweak.clone()),
        ),
    );
    fonts.font_data.insert(
        FAMILY_MEDIUM.into(),
        Arc::new(
            FontData::from_static(include_bytes!("../assets/Oxanium-Medium.ttf"))
                .tweak(tweak.clone()),
        ),
    );
    fonts.font_data.insert(
        FAMILY_SEMIBOLD.into(),
        Arc::new(
            FontData::from_static(include_bytes!("../assets/Oxanium-SemiBold.ttf")).tweak(tweak),
        ),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "oxanium".into());
    fonts.families.insert(
        FontFamily::Name(FAMILY_MEDIUM.into()),
        vec![FAMILY_MEDIUM.into()],
    );
    fonts.families.insert(
        FontFamily::Name(FAMILY_SEMIBOLD.into()),
        vec![FAMILY_SEMIBOLD.into()],
    );
    ctx.set_fonts(fonts);
    ctx.data_mut(|d| d.insert_temp(installed_id, true));
}

impl Theme {
    /// Read the active theme stored by [`Theme::apply`]. Falls back to vega
    /// dark, shared so the fallback does not build a [`Style`] per call.
    pub fn current(ctx: &egui::Context) -> Theme {
        static FALLBACK: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(Theme::vega_dark);
        ctx.data(|d| d.get_temp::<Theme>(theme_id()))
            .unwrap_or_else(|| FALLBACK.clone())
    }

    /// Push this theme into the egui context: fonts, type scale, spacing, colors.
    ///
    /// Call once, right after the context is created, and again only when the
    /// theme changes: the font atlas is only built on the first call per
    /// context (later calls just refresh the style and stored theme). Not every
    /// frame: that would silently undo any other code's style writes. egui-native widgets pick up the inactive/hovered/active visuals
    /// set here, while the crate's own custom components manage their own
    /// per-state colors.
    pub fn apply(&self, ctx: &egui::Context) {
        install_fonts(ctx);
        let p = &self.palette;
        let m = &self.metrics;
        let hairline = m.border;

        ctx.global_style_mut(|s| {
            use FontFamily::Proportional;
            s.text_styles = [
                (TextStyle::Small, FontId::new(m.text_xs, Proportional)),
                (TextStyle::Body, FontId::new(m.text_sm, Proportional)),
                (TextStyle::Button, FontId::new(m.text_sm, Proportional)),
                (
                    TextStyle::Monospace,
                    FontId::new(m.text_mono, FontFamily::Monospace),
                ),
                (
                    TextStyle::Heading,
                    FontId::new(m.text_heading, FontFamily::Name(FAMILY_SEMIBOLD.into())),
                ),
            ]
            .into();

            // egui-native widgets follow the style's default button: its
            // height and horizontal padding, and the tabs gap between items.
            // A style without a value gets egui's default, also when an
            // earlier theme had set one.
            let egui_default = egui::style::Spacing::default();
            let button = [&self.style.button_size_default];
            let none = self.state(State::NONE);
            let gap = pick(&[&self.style.tabs], |r| &r.gap, none).unwrap_or(DEFAULT_ITEM_GAP);
            let pad_x =
                pick(&button, |r| &r.pad_left, none).unwrap_or(egui_default.button_padding.x);
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

            let v = &mut s.visuals;
            v.dark_mode = self.dark;
            v.panel_fill = p.background;
            v.window_fill = p.card;
            v.window_stroke = Stroke::new(hairline, p.border);
            v.extreme_bg_color = p.input;
            v.faint_bg_color = p.muted;
            v.override_text_color = Some(p.foreground);
            v.hyperlink_color = p.primary;
            v.selection.bg_fill = p.primary.gamma_multiply(0.35);
            v.selection.stroke = Stroke::new(hairline, p.ring);
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
        });

        ctx.data_mut(|d| d.insert_temp(theme_id(), self.clone()));
    }
}

/// Convenience accessor for components.
pub fn theme(ctx: &egui::Context) -> Theme {
    Theme::current(ctx)
}
