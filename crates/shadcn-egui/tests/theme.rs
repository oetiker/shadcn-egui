use shadcn_egui::theme::Theme;

#[test]
fn apply_sets_panel_fill_and_stores_theme() {
    use shadcn_egui::theme::{theme, Theme};
    let ctx = egui::Context::default();
    Theme::vega_dark().apply(&ctx);
    // run one frame so style takes effect
    let _ = ctx.run_ui(Default::default(), |_ui| {});
    let got = theme(&ctx);
    assert!(got.dark);
    assert_eq!(
        ctx.global_style().visuals.panel_fill,
        Theme::vega_dark().palette.background
    );
}

#[test]
fn radius_scale_is_derived() {
    let t = Theme::vega_dark();
    assert_eq!(t.radius, 10.0);
    assert_eq!(t.radius_sm(), 6.0);
    assert_eq!(t.radius_md(), 8.0);
    assert_eq!(t.radius_lg(), 10.0);
    assert_eq!(t.radius_xl(), 14.0);
}

#[test]
fn light_and_dark_differ() {
    let l = Theme::vega_light();
    let d = Theme::vega_dark();
    assert!(!l.dark && d.dark);
    assert_ne!(l.palette.background, d.palette.background);
}

#[test]
fn dark_border_is_low_alpha_white() {
    let d = Theme::vega_dark();
    // dark --border is oklch(1 0 0 / 10%) -> white at ~26 alpha
    assert_eq!(d.palette.border.a(), 26);
}

use shadcn_egui::style::{Paint, Radius, Role, Rung, ShadowLevel, State, Style, TextSize};
use shadcn_egui::Palette;

#[test]
fn new_detects_dark_from_palette() {
    assert!(Theme::new(Style::vega(), Palette::dark(), 10.0).dark);
    assert!(!Theme::new(Style::vega(), Palette::light(), 10.0).dark);
    assert_eq!(Theme::vega_dark().style.name, "vega");
}

#[test]
fn state_adds_dark() {
    assert!(Theme::vega_dark()
        .state(&egui::Context::default(), State::HOVER)
        .contains(State::DARK));
    assert!(!Theme::vega_light()
        .state(&egui::Context::default(), State::HOVER)
        .contains(State::DARK));
}

#[test]
fn radius_scale_multiplies() {
    let t = Theme::vega_dark();
    assert_eq!(t.rung(Rung::Sm), 6.0);
    assert_eq!(t.rung(Rung::Md), 8.0);
    assert_eq!(t.rung(Rung::Xl), 14.0);
    assert_eq!(t.rung(Rung::Xl4), 26.0);
    assert_eq!(t.radius_px(Radius::Min(Rung::Md, 6.0), 100.0), 6.0);
    assert_eq!(t.radius_px(Radius::Rung(Rung::Full), 10.0), 10.0);
}

#[test]
fn radius_clamps() {
    let mut t = Theme::vega_dark();
    t.radius = 0.0;
    assert_eq!(t.radius_px(Radius::Rung(Rung::Xl4), 50.0), 0.0);
    t.radius = 400.0;
    let r = t.radius_px(Radius::Rung(Rung::Xl4), 1000.0);
    assert!(r <= 255.0, "{r}");
    assert_eq!(t.radius_px(Radius::Rung(Rung::Xl4), 9.0), 9.0);
}

#[test]
fn paint_alpha_and_mix() {
    let t = Theme::vega_light();
    let p = t.paint(Paint::Role(Role::Primary, 1.0));
    assert_eq!(p, t.palette.primary);
    let half = t.paint(Paint::Role(Role::Primary, 0.5));
    assert!((half.a() as i32 - 128).abs() <= 1, "{half:?}");
    assert_eq!(half, t.palette.primary.gamma_multiply(0.5));
    assert_eq!(
        t.paint(Paint::Role(Role::Transparent, 1.0)),
        egui::Color32::TRANSPARENT
    );
    let mix = t.paint(Paint::Mix(Role::Secondary, Role::Foreground, 0.05));
    assert_ne!(mix, t.palette.secondary);
}

#[test]
fn text_and_shadow() {
    let t = Theme::vega_dark();
    assert_eq!(t.text_px(TextSize::Sm), 14.0);
    assert_eq!(t.text_px(TextSize::Px(12.8)), 12.8);
    assert!(t.shadow(ShadowLevel::None).is_empty());
    // Tailwind v4 theme.css: (level, [(y offset, blur, spread, alpha)]).
    for (level, want) in [
        (ShadowLevel::Xs2, &[(1.0, 0.0, 0.0, 0.05)][..]),
        (ShadowLevel::Xs, &[(1.0, 2.0, 0.0, 0.05)]),
        (
            ShadowLevel::Sm,
            &[(1.0, 3.0, 0.0, 0.1), (1.0, 2.0, -1.0, 0.1)],
        ),
        (
            ShadowLevel::Md,
            &[(4.0, 6.0, -1.0, 0.1), (2.0, 4.0, -2.0, 0.1)],
        ),
        (
            ShadowLevel::Lg,
            &[(10.0, 15.0, -3.0, 0.1), (4.0, 6.0, -4.0, 0.1)],
        ),
        (
            ShadowLevel::Xl,
            &[(20.0, 25.0, -5.0, 0.1), (8.0, 10.0, -6.0, 0.1)],
        ),
        (ShadowLevel::Xl2, &[(25.0, 50.0, -12.0, 0.25)]),
    ] {
        let got: Vec<_> = t
            .shadow(level)
            .iter()
            .map(|l| {
                assert_eq!(l.offset().x, 0.0, "{level:?}");
                assert_eq!(l.color(), egui::Color32::BLACK, "{level:?}");
                (l.offset().y, l.blur(), l.spread(), l.alpha())
            })
            .collect();
        assert_eq!(got, want, "{level:?}");
    }
}

#[test]
fn bold_menu_accent_swaps_accent_for_primary() {
    let p = Palette::light().with_bold_menu_accent();
    assert_eq!(p.accent, Palette::light().primary);
    assert_eq!(p.accent_foreground, Palette::light().primary_foreground);
}

#[test]
fn current_without_apply_is_vega_dark_and_cheap() {
    let ctx = egui::Context::default();
    let a = Theme::current(&ctx);
    let b = Theme::current(&ctx);
    assert!(a.dark);
    assert_eq!(a.style.name, "vega");
    // The fallback shares one Style instead of building one per call.
    assert!(std::sync::Arc::ptr_eq(&a.style, &b.style));
}

#[test]
fn consumer_override_takes_effect() {
    let mut s = Style::nova();
    s.button_size_default.height.base = Some(40.0);
    let t = Theme::new(s, Palette::dark(), 10.0);
    assert_eq!(
        t.style
            .button_size_default
            .height
            .get(t.state(&egui::Context::default(), State::NONE)),
        Some(40.0)
    );
}

#[test]
fn consumer_set_dark_is_honoured() {
    let mut t = Theme::new(Style::vega(), Palette::dark(), 10.0);
    t.dark = false;
    assert!(!t
        .state(&egui::Context::default(), State::HOVER)
        .contains(State::DARK));
    let mut t = Theme::new(Style::vega(), Palette::light(), 10.0);
    t.dark = true;
    assert!(t
        .state(&egui::Context::default(), State::HOVER)
        .contains(State::DARK));
}

use shadcn_egui::icons::lucide::{
    CHECK, CHEVRON_DOWN, CHEVRON_LEFT, CHEVRON_RIGHT, CHEVRON_UP, MORE_HORIZONTAL,
    SLOTS as LUCIDE_SLOTS, X as CLOSE,
};
use shadcn_egui::{FontName, IconLibrary, IconSlots, MenuColors};

#[test]
fn a_new_theme_draws_lucide_and_its_own_menu_colours() {
    let t = Theme::vega_light();
    let mut lucide = IconSlots::all(CLOSE);
    lucide.check = CHECK;
    lucide.select_trigger = CHEVRON_DOWN;
    lucide.chevron_down_plain = CHEVRON_DOWN;
    lucide.chevron_up = CHEVRON_UP;
    lucide.chevron_left = CHEVRON_LEFT;
    lucide.chevron_right = CHEVRON_RIGHT;
    lucide.more_horizontal = MORE_HORIZONTAL;
    assert_eq!(t.icons, lucide);
    assert_eq!(t.icons, LUCIDE_SLOTS);
    assert_eq!(t.menu, MenuColors::default());
    assert_eq!(
        t.menu_theme(),
        t,
        "no menu colours: the menu draws with the theme itself"
    );
}

#[test]
fn icon_and_menu_builders() {
    let crosses = IconSlots::all(CLOSE);
    assert_eq!(Theme::vega_light().icons(crosses).icons, crosses);
    let menu = MenuColors {
        palette: Some(Palette::dark()),
        dark: Some(true),
    };
    let t = Theme::vega_light().menu(menu);
    assert_eq!(t.menu, menu);
    let mt = t.menu_theme();
    assert_eq!((mt.palette, mt.dark), (Palette::dark(), true));
    assert!(
        std::sync::Arc::ptr_eq(&mt.style, &t.style),
        "the menu theme shares the style"
    );
    assert_eq!(
        (t.palette, t.dark),
        (Palette::light(), false),
        "the theme itself is unchanged"
    );
}

#[test]
fn registries_hold_what_this_build_bundles() {
    assert_eq!(
        shadcn_egui::fonts::files(FontName::Inter).map(|f| f.name),
        Some("inter")
    );
    assert_eq!(
        shadcn_egui::icons::slots(IconLibrary::Lucide),
        Some(LUCIDE_SLOTS)
    );
    #[cfg(feature = "font-oxanium")]
    assert_eq!(
        shadcn_egui::fonts::files(FontName::Oxanium).map(|f| f.name),
        Some("oxanium")
    );
    #[cfg(not(feature = "font-oxanium"))]
    assert_eq!(shadcn_egui::fonts::files(FontName::Oxanium), None);
}

/// New `Theme` fields must stay small: `Theme::vega_dark()` already needs
/// ~200 KB of a 256 KB test stack (see `every_exported_style_builds_on_a_small_stack`).
#[test]
fn theme_stays_small() {
    assert!(
        std::mem::size_of::<Theme>() <= 1024,
        "Theme is {} bytes",
        std::mem::size_of::<Theme>()
    );
}

/// Assert that both of egui's styles carry `t`, whichever one is active.
fn assert_both_styles(ctx: &egui::Context, t: &Theme) {
    for which in [egui::Theme::Dark, egui::Theme::Light] {
        let s = ctx.style_of(which);
        assert_eq!(s.visuals.panel_fill, t.palette.background, "{which:?}");
        assert_eq!(
            s.text_styles[&egui::TextStyle::Body].size,
            t.metrics.text_sm,
            "{which:?}"
        );
        assert_eq!(
            s.spacing.item_spacing,
            ctx.global_style().spacing.item_spacing,
            "{which:?}"
        );
    }
}

#[test]
fn apply_reaches_both_styles_on_a_light_context() {
    let ctx = egui::Context::default();
    ctx.set_theme(egui::Theme::Light);
    let t = Theme::vega_dark();
    t.apply(&ctx);
    let _ = ctx.run_ui(Default::default(), |_ui| {});
    assert_eq!(ctx.global_style().visuals.panel_fill, t.palette.background);
    assert_both_styles(&ctx, &t);
}

#[test]
fn apply_survives_a_system_theme_switch() {
    let ctx = egui::Context::default();
    let t = Theme::vega_light();
    t.apply(&ctx);
    for sys in [egui::Theme::Light, egui::Theme::Dark] {
        let input = egui::RawInput {
            system_theme: Some(sys),
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |_ui| {});
        assert_eq!(
            ctx.global_style().visuals.panel_fill,
            t.palette.background,
            "{sys:?}"
        );
        assert_both_styles(&ctx, &t);
    }
}
