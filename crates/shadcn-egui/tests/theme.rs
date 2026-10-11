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
    assert!(Theme::vega_dark().state(State::HOVER).contains(State::DARK));
    assert!(!Theme::vega_light()
        .state(State::HOVER)
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
    assert_eq!(t.shadow(ShadowLevel::None), egui::epaint::Shadow::NONE);
    // (level, y offset, blur, black alpha) from the Tailwind v4 shadow scale.
    for (level, y, blur, a) in [
        (ShadowLevel::Xs2, 1, 0, 13),
        (ShadowLevel::Xs, 1, 2, 13),
        (ShadowLevel::Sm, 1, 3, 26),
        (ShadowLevel::Md, 4, 6, 26),
        (ShadowLevel::Lg, 10, 15, 26),
        (ShadowLevel::Xl, 20, 25, 26),
        (ShadowLevel::Xl2, 25, 50, 64),
    ] {
        let s = t.shadow(level);
        assert_eq!(s.offset, [0, y], "{level:?}");
        assert_eq!(s.blur, blur, "{level:?}");
        assert_eq!(s.spread, 0, "{level:?}");
        assert_eq!(s.color, egui::Color32::from_black_alpha(a), "{level:?}");
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
        t.style.button_size_default.height.get(t.state(State::NONE)),
        Some(40.0)
    );
}

#[test]
fn consumer_set_dark_is_honoured() {
    let mut t = Theme::new(Style::vega(), Palette::dark(), 10.0);
    t.dark = false;
    assert!(!t.state(State::HOVER).contains(State::DARK));
    let mut t = Theme::new(Style::vega(), Palette::light(), 10.0);
    t.dark = true;
    assert!(t.state(State::HOVER).contains(State::DARK));
}

use shadcn_egui::components::icon::{CHECK, CHEVRON_DOWN, CLOSE, LUCIDE_SLOTS};
use shadcn_egui::{FontName, IconLibrary, IconSlots, MenuColors};

#[test]
fn a_new_theme_draws_lucide_and_its_own_menu_colours() {
    let t = Theme::vega_light();
    assert_eq!(
        t.icons,
        IconSlots {
            check: CHECK,
            chevron_down: CHEVRON_DOWN,
            close: CLOSE
        }
    );
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
    let crosses = IconSlots {
        check: CLOSE,
        chevron_down: CLOSE,
        close: CLOSE,
    };
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
