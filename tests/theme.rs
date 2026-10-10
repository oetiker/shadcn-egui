use egui_shadcn::theme::Theme;

#[test]
fn apply_sets_panel_fill_and_stores_theme() {
    use egui_shadcn::theme::{theme, Theme};
    let ctx = egui::Context::default();
    Theme::dark().apply(&ctx);
    // run one frame so style takes effect
    let _ = ctx.run_ui(Default::default(), |_ui| {});
    let got = theme(&ctx);
    assert!(got.dark);
    assert_eq!(ctx.global_style().visuals.panel_fill, Theme::dark().palette.background);
}

#[test]
fn radius_scale_is_derived() {
    let t = Theme::dark();
    assert_eq!(t.radius, 10.0);
    assert_eq!(t.radius_sm(), 6.0);
    assert_eq!(t.radius_md(), 8.0);
    assert_eq!(t.radius_lg(), 10.0);
    assert_eq!(t.radius_xl(), 14.0);
}

#[test]
fn light_and_dark_differ() {
    let l = Theme::light();
    let d = Theme::dark();
    assert!(!l.dark && d.dark);
    assert_ne!(l.palette.background, d.palette.background);
}

#[test]
fn dark_border_is_low_alpha_white() {
    let d = Theme::dark();
    // dark --border is oklch(1 0 0 / 10%) -> white at ~26 alpha
    assert_eq!(d.palette.border.a(), 26);
}

use egui_shadcn::style::{Paint, Radius, Role, Rung, ShadowLevel, State, Style, TextSize};
use egui_shadcn::Palette;

#[test]
fn new_detects_dark_from_palette() {
    assert!(Theme::new(Style::vega(), Palette::dark(), 10.0).dark);
    assert!(!Theme::new(Style::vega(), Palette::light(), 10.0).dark);
    assert_eq!(Theme::vega_dark().style.name, "vega");
}

#[test]
fn state_adds_dark() {
    assert!(Theme::vega_dark().state(State::HOVER).contains(State::DARK));
    assert!(!Theme::vega_light().state(State::HOVER).contains(State::DARK));
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
    assert_eq!(t.paint(Paint::Role(Role::Transparent, 1.0)), egui::Color32::TRANSPARENT);
    let mix = t.paint(Paint::Mix(Role::Secondary, Role::Foreground, 0.05));
    assert_ne!(mix, t.palette.secondary);
}

#[test]
fn text_and_shadow() {
    let t = Theme::vega_dark();
    assert_eq!(t.text_px(TextSize::Sm), 14.0);
    assert_eq!(t.text_px(TextSize::Px(12.8)), 12.8);
    assert_eq!(t.shadow(ShadowLevel::None), egui::epaint::Shadow::NONE);
    assert_eq!(t.shadow(ShadowLevel::Md).offset, [0, 4]);
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
    assert_eq!(t.style.button_size_default.height.get(t.state(State::NONE)), Some(40.0));
}
