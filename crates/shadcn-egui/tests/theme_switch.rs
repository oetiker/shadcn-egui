mod common;

use common::Loop;
use shadcn_egui::components::button::Button;
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

/// The fill of the first non-transparent rect painted over `at`'s centre.
fn fill_at(out: &egui::FullOutput, at: egui::Rect) -> egui::Color32 {
    common::rect_shapes(out)
        .iter()
        .find(|r| {
            r.rect.contains(at.center())
                && r.fill != egui::Color32::TRANSPARENT
                && r.rect.width() < 300.0
        })
        .map(|r| r.fill)
        .expect("a fill over the widget")
}

/// `ctx.set_theme(..)` in the middle of a pass that no input caused (a
/// timer, an app's own logic): no frame mixes the two modes. Under
/// `run_ui` the frame egui returns draws the button before the call and the
/// one after it in the new mode; an app's own loop gets the pass marked for
/// discard and a repaint, and its next pass is in the new mode.
#[test]
fn a_mode_switch_in_the_middle_of_a_pass_shows_no_mixed_frame() {
    for app_loop in Loop::BOTH {
        let ctx = egui::Context::default();
        Theme::vega().apply(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        let switch = std::cell::Cell::new(false);
        let rects = std::cell::Cell::new([egui::Rect::NOTHING; 2]);
        let frame = || {
            common::pass(app_loop, &ctx, vec![], |ui| {
                let before = ui.add(Button::new("Before")).rect;
                if switch.get() {
                    ui.ctx().set_theme(egui::Theme::Light);
                }
                let after = ui.add(Button::new("After")).rect;
                rects.set([before, after]);
            })
        };
        let _ = frame();
        let _ = frame();
        switch.set(true);
        let out = frame();
        let light = Theme::vega().light.primary;
        let [before, after] = rects.get();
        match app_loop {
            Loop::RunUi => {
                assert_eq!(fill_at(&out, before), light, "{app_loop:?}: before");
                assert_eq!(fill_at(&out, after), light, "{app_loop:?}: after");
            }
            Loop::Own => {
                assert!(
                    out.platform_output.requested_discard(),
                    "{app_loop:?}: the mixed pass asks to be discarded"
                );
                assert!(
                    ctx.has_requested_repaint(),
                    "{app_loop:?}: a repaint follows"
                );
                let out = frame();
                assert_eq!(
                    fill_at(&out, before),
                    light,
                    "{app_loop:?}: next pass, before"
                );
                assert_eq!(
                    fill_at(&out, after),
                    light,
                    "{app_loop:?}: next pass, after"
                );
            }
        }
    }
}

/// `ctx.set_theme(..)` after the last crate widget of a pass: under
/// `run_ui` the frame egui returns draws that widget in the new mode, as
/// the browser restyles the whole page at once, and a repaint is asked
/// for, so no frame stays in the old mode until the next input.
#[test]
fn a_mode_switch_after_the_last_crate_widget_restyles_the_frame() {
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    let switch = std::cell::Cell::new(false);
    let rect = std::cell::Cell::new(egui::Rect::NOTHING);
    let frame = || {
        common::pass(Loop::RunUi, &ctx, vec![], |ui| {
            rect.set(ui.add(Button::new("Only")).rect);
            if switch.get() {
                ui.ctx().set_theme(egui::Theme::Light);
            }
        })
    };
    let _ = frame();
    let _ = frame();
    switch.set(true);
    let out = frame();
    assert_eq!(fill_at(&out, rect.get()), Theme::vega().light.primary);
    assert!(ctx.has_requested_repaint(), "a repaint follows the switch");
}

/// A plain egui `Area` opened after a mid-pass `ctx.set_theme(..)` and the
/// panel drawn before it end in the same mode: the frame `run_ui` returns
/// has both in the new mode.
#[test]
fn a_mode_switch_before_a_plain_area_shows_no_mixed_frame() {
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    let switch = std::cell::Cell::new(false);
    let rects = std::cell::Cell::new([egui::Rect::NOTHING; 2]);
    let frame = || {
        common::pass(Loop::RunUi, &ctx, vec![], |ui| {
            let panel = ui.button("Panel").rect;
            if switch.get() {
                ui.ctx().set_theme(egui::Theme::Light);
            }
            let area = egui::Area::new(egui::Id::new("area"))
                .fixed_pos(egui::pos2(300.0, 200.0))
                .fade_in(false)
                .show(ui.ctx(), |ui| ui.button("Area").rect)
                .inner;
            rects.set([panel, area]);
        })
    };
    let _ = frame();
    let _ = frame();
    switch.set(true);
    let out = frame();
    let light = ctx
        .style_of(egui::Theme::Light)
        .visuals
        .widgets
        .inactive
        .weak_bg_fill;
    assert_ne!(
        light,
        ctx.style_of(egui::Theme::Dark)
            .visuals
            .widgets
            .inactive
            .weak_bg_fill
    );
    let [panel, area] = rects.get();
    assert_eq!(fill_at(&out, panel), light, "panel");
    assert_eq!(fill_at(&out, area), light, "area");
}

/// Vega with Oxanium as its body and heading font.
fn oxanium() -> Theme {
    let mut t = Theme::vega();
    t.fonts = shadcn_egui::ThemeFonts::inherit(&shadcn_egui_font_oxanium::FONT);
    t
}

/// The width of one crate label in `theme`'s fonts, settled.
fn label_width(theme: &Theme) -> f32 {
    use shadcn_egui::components::label::label;
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    let mut width = 0.0;
    for _ in 0..2 {
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            width = label(ui, "Accept terms").rect.width();
        });
    }
    width
}

/// `Theme::apply` of a theme with other fonts in the middle of a pass
/// (a preset picker, an app's own logic): egui swaps in the new fonts only
/// at the next pass, so the pass asks to be discarded and a repaint
/// follows, as a mid-pass `ctx.set_theme(..)` does. Under `run_ui` the
/// frame egui returns draws the labels before and after the call in the
/// new fonts; an app's own loop gets the pass marked for discard and a
/// repaint, and its next pass is in the new fonts. Either way no frame
/// keeps the old fonts until the next input.
#[test]
fn a_font_change_in_the_middle_of_a_pass_shows_the_new_fonts() {
    use shadcn_egui::components::label::label;
    let (vega, new) = (label_width(&Theme::vega()), label_width(&oxanium()));
    assert_ne!(vega, new, "the two fonts measure apart");
    for app_loop in Loop::BOTH {
        let ctx = egui::Context::default();
        Theme::vega().apply(&ctx);
        let switch = std::cell::Cell::new(false);
        let widths = std::cell::Cell::new([0.0; 2]);
        let frame = || {
            common::pass(app_loop, &ctx, vec![], |ui| {
                let before = label(ui, "Accept terms").rect.width();
                if switch.get() {
                    oxanium().apply(ui.ctx());
                }
                let after = label(ui, "Accept terms").rect.width();
                widths.set([before, after]);
            })
        };
        let _ = frame();
        let _ = frame();
        assert_eq!(widths.get(), [vega; 2], "{app_loop:?}: settled in vega");
        switch.set(true);
        let out = frame();
        switch.set(false);
        match app_loop {
            Loop::RunUi => {
                assert_eq!(widths.get(), [new; 2], "{app_loop:?}: the frame");
            }
            Loop::Own => {
                assert!(
                    out.platform_output.requested_discard(),
                    "{app_loop:?}: the pass in the old fonts asks to be discarded"
                );
                assert!(
                    ctx.has_requested_repaint(),
                    "{app_loop:?}: a repaint follows"
                );
                let _ = frame();
                assert_eq!(widths.get(), [new; 2], "{app_loop:?}: next pass");
            }
        }
    }
}

/// `Theme::apply` of a theme with other fonts after the last crate widget
/// of a pass: the pass hook asks for a discard, so the frame `run_ui`
/// returns draws the widget in the new fonts.
#[test]
fn a_font_change_after_the_last_crate_widget_shows_the_new_fonts() {
    use shadcn_egui::components::label::label;
    let new = label_width(&oxanium());
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    let switch = std::cell::Cell::new(false);
    let width = std::cell::Cell::new(0.0);
    let frame = || {
        common::pass(Loop::RunUi, &ctx, vec![], |ui| {
            width.set(label(ui, "Accept terms").rect.width());
            if switch.get() {
                oxanium().apply(ui.ctx());
            }
        })
    };
    let _ = frame();
    let _ = frame();
    switch.set(true);
    let _ = frame();
    assert_eq!(width.get(), new);
}

/// An app's own loop whose pass starts with `Theme::apply` of other fonts
/// and draws no crate widget after it: no crate call saw the pass start,
/// so the change cannot be told from one between two passes, and a
/// repaint follows; the next pass is in the new fonts.
#[test]
fn a_font_change_as_the_first_crate_call_of_a_pass_repaints() {
    use shadcn_egui::components::label::label;
    let new = label_width(&oxanium());
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    for _ in 0..2 {
        let _ = common::own_pass(&ctx, vec![], |ui| drop(label(ui, "Accept terms")));
    }
    let _ = common::own_pass(&ctx, vec![], |ui| {
        oxanium().apply(ui.ctx());
        ui.label("Accept terms");
    });
    assert!(ctx.has_requested_repaint(), "a repaint follows");
    let mut width = 0.0;
    let _ = common::own_pass(&ctx, vec![], |ui| {
        width = label(ui, "Accept terms").rect.width();
    });
    assert_eq!(width, new);
}
