use shadcn_egui::reference::{settings_ui, SettingsState};
use shadcn_egui::style::Style;
use shadcn_egui::{Palette, Theme};

fn render(ctx: &egui::Context, state: &mut SettingsState) {
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| settings_ui(ui, state));
}

/// Applying a second theme on a live context restyles the next frame: the
/// stored theme changes and the native control height follows the new style.
#[test]
fn applying_nova_after_vega_restyles_the_context() {
    let ctx = egui::Context::default();
    let mut state = SettingsState::default();

    Theme::new(Style::vega(), Palette::light(), Palette::dark(), 10.0).apply(&ctx);
    render(&ctx, &mut state);
    assert_eq!(Theme::current(&ctx).style.name, "vega");
    let vega_h = ctx.global_style().spacing.interact_size.y;

    Theme::new(Style::nova(), Palette::light(), Palette::dark(), 10.0).apply(&ctx);
    render(&ctx, &mut state);
    assert_eq!(Theme::current(&ctx).style.name, "nova");
    let nova_h = ctx.global_style().spacing.interact_size.y;

    assert_ne!(vega_h, nova_h, "interact_size.y must follow the style");
}

/// Every exported style defines a default-button height, so switching between
/// any two leaves no value of the previous style behind.
#[test]
fn every_style_sets_interact_height_itself() {
    // Constructors, not values: a Style is large and seven at once overflow a test thread's stack.
    let styles: [fn() -> Style; 7] = [
        Style::vega,
        Style::nova,
        Style::maia,
        Style::lyra,
        Style::mira,
        Style::luma,
        Style::rhea,
    ];
    let ctx = egui::Context::default();
    // Poison the value; each apply must overwrite it.
    for make in styles {
        let style = make();
        ctx.global_style_mut(|s| s.spacing.interact_size.y = 12345.0);
        let name = style.name;
        Theme::new(style, Palette::light(), Palette::dark(), 10.0).apply(&ctx);
        assert_ne!(
            ctx.global_style().spacing.interact_size.y,
            12345.0,
            "{name} left a stale interact_size.y"
        );
    }
}

/// A style without a default-button height leaves egui's default, not the
/// height of the theme applied before it.
#[test]
fn style_without_button_height_resets_interact_height() {
    let ctx = egui::Context::default();
    Theme::new(Style::vega(), Palette::light(), Palette::dark(), 10.0).apply(&ctx);
    assert_eq!(ctx.global_style().spacing.interact_size.y, 36.0);
    let mut bare = Style::nova();
    bare.button_size_default.height = Default::default();
    Theme::new(bare, Palette::light(), Palette::dark(), 10.0).apply(&ctx);
    assert_eq!(
        ctx.global_style().spacing.interact_size.y,
        egui::style::Spacing::default().interact_size.y
    );
}

/// A native egui button is as high as the style's default button.
#[test]
fn native_button_height_is_the_default_button_height() {
    let styles: [fn() -> Style; 7] = [
        Style::vega,
        Style::nova,
        Style::maia,
        Style::lyra,
        Style::mira,
        Style::luma,
        Style::rhea,
    ];
    for make in styles {
        let style = make();
        let name = style.name;
        let ctx = egui::Context::default();
        Theme::new(style, Palette::light(), Palette::dark(), 10.0).apply(&ctx);
        let mut height = 0.0;
        // The second frame lays out with the installed fonts.
        for _ in 0..2 {
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                height = ui.button("Save").rect.height()
            });
        }
        assert_eq!(height, ctx.global_style().spacing.interact_size.y, "{name}");
    }
}
