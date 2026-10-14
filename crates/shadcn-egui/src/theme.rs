//! shadcn v4 (OKLCH) design tokens and the `Theme` (style x palette x radius), materialized for egui.

use crate::color::{oklch_to_srgb as c, oklch_to_srgb_a as ca};
use crate::style::{pick, Paint, Radius, Role, Rung, ShadowLevel, State, Style, TextSize, Weight};
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

/// How menu content is coloured (upstream `menuColor`): today the Select
/// list. `None` keeps the theme's own palette or dark flag.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MenuColors {
    /// The menu's palette; an inverted menu passes the dark palette.
    pub palette: Option<Palette>,
    /// The menu's dark flag; `Some(true)` for an inverted menu, so `dark:`
    /// rules resolve as in dark mode.
    pub dark: Option<bool>,
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
    /// Body and heading font; [`Theme::apply`] installs them.
    pub fonts: ThemeFonts,
    /// The icons components draw (default: built-in Lucide).
    pub icons: IconSlots,
    /// Menu content colours (default: the theme's own).
    pub menu: MenuColors,
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
            fonts: ThemeFonts::inherit(&shadcn_egui_font_inter::FONT),
            icons: crate::icons::lucide::SLOTS,
            menu: MenuColors::default(),
        }
    }
    pub fn vega_dark() -> Self {
        Self::new(Style::vega(), Palette::dark(), 10.0)
    }
    pub fn vega_light() -> Self {
        Self::new(Style::vega(), Palette::light(), 10.0)
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

    /// Menu content colours (upstream `menuColor`). An inverted menu needs
    /// the dark palette: `MenuColors { palette: Some(dark), dark: Some(true), .. }`.
    pub fn menu(mut self, menu: MenuColors) -> Self {
        self.menu = menu;
        self
    }

    /// The theme menu content draws with: this one, with palette and dark
    /// flag replaced by the menu's where set.
    pub fn menu_theme(&self) -> Theme {
        let mut t = self.clone();
        if let Some(p) = self.menu.palette {
            t.palette = p;
        }
        if let Some(d) = self.menu.dark {
            t.dark = d;
        }
        t
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

/// `base` plus the theme's fonts: one registration per weight with the
/// `wght` coordinate, each family followed by `base`'s Proportional list so
/// glyphs the font lacks (emoji, CJK, symbols) still render. A theme's
/// fonts already in `base` (the context's current definitions) are dropped
/// first, so a reinstall neither stacks them nor keeps an earlier theme's.
fn font_definitions(fonts: ThemeFonts, mut base: FontDefinitions) -> FontDefinitions {
    let ours = |name: &str| name.starts_with("shadcn-");
    base.font_data.retain(|key, _| !ours(key));
    base.families
        .retain(|family, _| !matches!(family, FontFamily::Name(name) if ours(name)));
    for list in base.families.values_mut() {
        list.retain(|key| !ours(key));
    }
    let fallback = base
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let inherit = fonts.heading == fonts.body;
    for (role, files) in [("body", fonts.body), ("heading", fonts.heading)] {
        let data_role = if role == "heading" && inherit {
            "body"
        } else {
            role
        };
        for (i, w) in WEIGHTS.into_iter().enumerate() {
            let key = format!("shadcn-{data_role}-{w}");
            if data_role == role {
                let face = files.face(w);
                let mut tweak = egui::FontTweak {
                    y_offset_factor: files.y_offset_factor,
                    ..Default::default()
                };
                if let Some(v) = face.wght {
                    tweak.coords = VariationCoords::new([(b"wght", v)]);
                }
                base.font_data.insert(
                    key.clone(),
                    Arc::new(FontData::from_static(face.data).tweak(tweak)),
                );
            }
            let family = match (role, i) {
                ("body", 0) => FontFamily::Proportional,
                ("body", _) => {
                    FontFamily::Name(BODY_FAMILIES[i].expect("named body family").into())
                }
                _ => FontFamily::Name(HEADING_FAMILIES[i].into()),
            };
            let mut list = vec![key];
            list.extend(fallback.iter().cloned());
            base.families.insert(family, list);
        }
    }
    if fonts.body.kind == FontKind::Mono {
        base.families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "shadcn-body-400".into());
    }
    base
}

/// The fonts last installed on a context by [`Theme::apply`].
#[derive(Clone, PartialEq)]
struct InstalledFonts {
    body: &'static str,
    heading: &'static str,
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
/// the app's current fonts, which keeps the app's own families.
fn install_fonts(ctx: &egui::Context, fonts: ThemeFonts, base: Option<FontDefinitions>) {
    let id = egui::Id::new("shadcn_egui::installed_fonts");
    let key = InstalledFonts {
        body: fonts.body.name,
        heading: fonts.heading.name,
    };
    let base = match base {
        Some(base) => base,
        None if ctx.data(|d| d.get_temp::<InstalledFonts>(id)).as_ref() != Some(&key) => {
            FontDefinitions::default()
        }
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
    ctx.set_fonts(font_definitions(fonts, base));
    ctx.data_mut(|d| d.insert_temp(id, key));
}

/// `TextStyle::Heading` waits in `Proportional` until the heading family is
/// bound: `set_fonts` takes effect only at the start of the next pass, and
/// egui panics on a family the current fonts lack. Holds the stand-in
/// `FontId`, so an app's own later change to the heading is left alone.
#[derive(Clone)]
struct PendingHeading(FontId);

fn pending_heading_id() -> egui::Id {
    egui::Id::new("shadcn_egui::pending_heading")
}

/// Set once the heading family was seen bound. Every install defines it,
/// but an app's own `ctx.set_fonts` drops it: [`unbind_lost_heading`]
/// clears the flag again.
fn heading_bound_id() -> egui::Id {
    egui::Id::new("shadcn_egui::heading_bound")
}

fn heading_bound(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(heading_bound_id()).unwrap_or(false))
}

/// The family `TextStyle::Heading` may name now: the heading family once it
/// is known to be bound, else `Proportional`, upgraded by [`bind_heading`].
fn heading_family_now(ctx: &egui::Context) -> FontFamily {
    if heading_bound(ctx) {
        if is_bound(ctx, FAMILY_HEADING_SEMIBOLD) {
            return FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into());
        }
        // The app replaced the fonts; this apply installs the theme's again.
        ctx.data_mut(|d| d.remove::<bool>(heading_bound_id()));
    }
    let hook_id = egui::Id::new("shadcn_egui::heading_hook");
    if !ctx.data(|d| d.get_temp::<bool>(hook_id).unwrap_or(false)) {
        ctx.data_mut(|d| d.insert_temp(hook_id, true));
        ctx.on_begin_pass(
            "shadcn_egui::bind_heading",
            Arc::new(|ui: &mut egui::Ui| {
                if let Some(id) = check_heading(ui.ctx()) {
                    // The root ui was built before this hook ran.
                    ui.style_mut().text_styles.insert(TextStyle::Heading, id);
                }
            }),
        );
    }
    FontFamily::Proportional
}

/// Once per pass: a heading whose family the app's own fonts dropped
/// ([`unbind_lost_heading`]) falls back, a waiting one binds
/// ([`bind_heading`]). Returns the new heading `FontId` when it changed.
fn check_heading(ctx: &egui::Context) -> Option<FontId> {
    let id = egui::Id::new("shadcn_egui::heading_checked");
    let pass = ctx.cumulative_pass_nr();
    if ctx.data(|d| d.get_temp::<u64>(id)) == Some(pass) {
        return None;
    }
    ctx.data_mut(|d| d.insert_temp(id, pass));
    let fallback = unbind_lost_heading(ctx);
    bind_heading(ctx).or(fallback)
}

/// `TextStyle::Heading` back to `Proportional` when the context's fonts no
/// longer bind the heading family: the app called `ctx.set_fonts` after
/// [`Theme::apply`], and egui panics on a family its fonts lack. The heading
/// waits as a [`PendingHeading`] until a later apply installs the family.
fn unbind_lost_heading(ctx: &egui::Context) -> Option<FontId> {
    if !heading_bound(ctx) || is_bound(ctx, FAMILY_HEADING_SEMIBOLD) {
        return None;
    }
    let warned = egui::Id::new("shadcn_egui::lost_heading_warned");
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
    let mut stand_in = None;
    ctx.all_styles_mut(|s| {
        if let Some(h) = s.text_styles.get_mut(&TextStyle::Heading) {
            if h.family == name {
                h.family = FontFamily::Proportional;
                stand_in = Some(h.clone());
            }
        }
    });
    let stand_in = stand_in?;
    ctx.data_mut(|d| d.insert_temp(pending_heading_id(), PendingHeading(stand_in.clone())));
    Some(stand_in)
}

/// Give `TextStyle::Heading` the heading family once the fonts bind it.
/// Returns the new heading `FontId` when it changed.
fn bind_heading(ctx: &egui::Context) -> Option<FontId> {
    let PendingHeading(stand_in) = ctx.data(|d| d.get_temp(pending_heading_id()))?;
    let name = FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into());
    if !ctx.fonts(|f| f.definitions().families.contains_key(&name)) {
        return None;
    }
    ctx.data_mut(|d| {
        d.remove::<PendingHeading>(pending_heading_id());
        d.insert_temp(heading_bound_id(), true);
    });
    let id = FontId::new(stand_in.size, name);
    let mut changed = false;
    ctx.all_styles_mut(|s| {
        if s.text_styles.get(&TextStyle::Heading) == Some(&stand_in) {
            s.text_styles.insert(TextStyle::Heading, id.clone());
            changed = true;
        }
    });
    changed.then_some(id)
}

impl Theme {
    /// Read the active theme stored by [`Theme::apply`]. Falls back to vega
    /// dark, shared so the fallback does not build a [`Style`] per call.
    ///
    /// Also binds a heading that still waits for its fonts, and lets one
    /// whose family the app's own fonts dropped fall back, for apps whose
    /// loop does not run egui's pass hooks (`Context::begin_pass`). Needs
    /// the context's fonts, which exist once its first pass has ended. In
    /// such a loop, a heading in a `Ui` built before the first shadcn
    /// component of the pass can still panic after an app's own
    /// `ctx.set_fonts`; use [`Theme::apply_with_fonts`].
    pub fn current(ctx: &egui::Context) -> Theme {
        static FALLBACK: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(Theme::vega_dark);
        // Before the first pass ends, fonts may not exist yet; the hook binds then.
        if ctx.cumulative_pass_nr() > 0 {
            let _ = check_heading(ctx);
        }
        ctx.data(|d| d.get_temp::<Theme>(theme_id()))
            .unwrap_or_else(|| FALLBACK.clone())
    }

    /// Push this theme into the egui context: fonts, type scale, spacing, colors.
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
    /// the font atlas on every other frame.
    pub fn apply(&self, ctx: &egui::Context) {
        self.apply_inner(ctx, None);
    }

    /// Like [`Theme::apply`], starting from the app's own font definitions
    /// instead of egui's defaults. Always reinstalls the fonts.
    pub fn apply_with_fonts(&self, ctx: &egui::Context, base: FontDefinitions) {
        self.apply_inner(ctx, Some(base));
    }

    fn apply_inner(&self, ctx: &egui::Context, base: Option<FontDefinitions>) {
        install_fonts(ctx, self.fonts, base);
        let p = &self.palette;
        let m = &self.metrics;
        let hairline = m.border;
        let heading = FontId::new(m.text_heading, heading_family_now(ctx));
        if heading.family == FontFamily::Proportional {
            ctx.data_mut(|d| d.insert_temp(pending_heading_id(), PendingHeading(heading.clone())));
        }

        ctx.all_styles_mut(|s| {
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
            let none = self.state(State::NONE);
            let gap = pick(&[&*self.style.tabs], |r| &r.gap, none).unwrap_or(DEFAULT_ITEM_GAP);
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
