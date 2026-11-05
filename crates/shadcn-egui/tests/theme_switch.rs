mod common;

use common::Loop;
use shadcn_egui::components::button::Button;
use shadcn_egui::reference::{settings_ui, SettingsState};
use shadcn_egui::style::Style;
use shadcn_egui::{Palette, Theme};

fn render(ctx: &egui::Context, state: &mut SettingsState) {
    ctx.run_ui(egui::RawInput::default(), |ui| settings_ui(ui, state))
        .drop_without_applying_deltas();
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
            ctx.run_ui(egui::RawInput::default(), |ui| {
                height = ui.button("Save").rect.height()
            })
            .drop_without_applying_deltas();
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
            let area = egui::Area::new(egui::Id::unique("area"))
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
        ctx.run_ui(egui::RawInput::default(), |ui| {
            width = label(ui, "Accept terms").rect.width();
        })
        .drop_without_applying_deltas();
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

/// The first `Theme::apply` on a context, after the last crate widget of
/// a `run_ui` pass (a button that turns the theme on): egui took the list
/// of pass hooks before the pass, so the theme's hooks do not run in it.
/// The frame `run_ui` returns still draws the widget in the theme's fonts,
/// and a repaint follows.
#[test]
fn a_first_apply_after_the_last_crate_widget_shows_the_new_fonts() {
    use shadcn_egui::components::label::label;
    let vega = label_width(&Theme::vega());
    let ctx = egui::Context::default();
    let switch = std::cell::Cell::new(false);
    let width = std::cell::Cell::new(0.0);
    let frame = || {
        common::no_textures(ctx.run_ui(egui::RawInput::default(), |ui| {
            width.set(label(ui, "Accept terms").rect.width());
            if switch.get() {
                Theme::vega().apply(ui.ctx());
            }
        }))
    };
    let _ = frame();
    let _ = frame();
    assert_ne!(width.get(), vega, "egui's default fonts measure apart");
    switch.set(true);
    let out = frame();
    assert_eq!(width.get(), vega, "the frame is in the theme's fonts");
    assert_eq!(
        out.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
        std::time::Duration::ZERO,
        "a repaint follows"
    );
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

/// `Theme::apply_with_fonts` of the same fonts in every pass, after a
/// crate widget or before any: egui keeps its fonts, so no pass is
/// discarded and a still frame asks for no repaint.
#[test]
fn the_same_fonts_applied_every_pass_settle() {
    for first in [false, true] {
        let ctx = egui::Context::default();
        Theme::vega().apply(&ctx);
        let apply = |ctx: &egui::Context| {
            Theme::vega().apply_with_fonts(ctx, egui::FontDefinitions::default());
        };
        for frame in 0..6 {
            let mut passes = 0;
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                passes += 1;
                if first {
                    apply(ui.ctx());
                }
                ui.add(Button::new("A"));
                if !first {
                    apply(ui.ctx());
                }
                ui.add(Button::new("B"));
            });
            out.textures_delta.clear();
            assert_eq!(passes, 1, "first {first}, frame {frame}: passes");
            // egui 0.37 sends the window its theme in the first frame,
            // which asks for a second one.
            if frame > 1 {
                let delay = out.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
                assert_eq!(
                    delay,
                    std::time::Duration::MAX,
                    "first {first}, frame {frame}"
                );
            }
        }
    }
}

/// Native viewports for this thread's contexts, run as eframe runs an
/// immediate one: a `run_ui` pass of its own, inside its parent's pass.
fn native_viewports(ctx: &egui::Context) {
    ctx.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(|ctx, mut viewport| {
        let id = viewport.ids.this;
        let input = egui::RawInput {
            viewport_id: id,
            viewports: [(id, egui::ViewportInfo::default())].into_iter().collect(),
            ..Default::default()
        };
        ctx.run_ui(input, |ui| (viewport.viewport_ui_cb)(ui))
            .drop_without_applying_deltas();
    });
}

fn child_viewport() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("shadcn_egui_test_child")
}

/// The delay before the root viewport's next frame.
fn root_repaint_delay(out: &egui::FullOutput) -> std::time::Duration {
    out.viewport_output[&egui::ViewportId::ROOT].repaint_delay
}

/// `Theme::apply` of other fonts in the root's pass, and then a window
/// (an immediate viewport) that opens and draws a crate widget: the
/// window counts its passes apart from the root, and its check does not
/// hide the font change from the root's own check. The frame `run_ui`
/// returns draws the root's widget in the new fonts.
#[test]
fn a_font_change_before_a_new_window_shows_the_new_fonts() {
    use shadcn_egui::components::label::label;
    let new = label_width(&oxanium());
    let ctx = egui::Context::default();
    native_viewports(&ctx);
    Theme::vega().apply(&ctx);
    let switch = std::cell::Cell::new(false);
    let width = std::cell::Cell::new(0.0);
    let frame = || {
        common::no_textures(ctx.run_ui(egui::RawInput::default(), |ui| {
            width.set(label(ui, "Accept terms").rect.width());
            if switch.get() {
                oxanium().apply(ui.ctx());
                let builder = egui::ViewportBuilder::default();
                ui.ctx()
                    .show_viewport_immediate(child_viewport(), builder, |ui, _| {
                        drop(label(ui, "Settings"));
                    });
            }
        }))
    };
    let _ = frame();
    let _ = frame();
    switch.set(true);
    let _ = frame();
    assert_eq!(width.get(), new);
}

/// Vega with its light and dark palettes swapped: other colours, the
/// same fonts.
fn swapped() -> Theme {
    Theme::new(Style::vega(), Palette::dark(), Palette::light(), 10.0)
}

/// What a window applies in [`a_theme_change_in_a_window_repaints_the_root`].
#[derive(Clone, Copy, Debug, PartialEq)]
enum Change {
    /// The first `Theme::apply` on the context.
    First,
    /// A theme with other fonts.
    Fonts,
    /// A theme with other colours and the same fonts.
    Colours,
}

/// Fonts or styles changed in a window (an immediate viewport) belong to
/// the whole context: the root viewport, which drew in the old ones,
/// repaints.
#[test]
fn a_theme_change_in_a_window_repaints_the_root() {
    use std::time::Duration;
    let changes = [Change::First, Change::Fonts, Change::Colours];
    let mut delays = Vec::new();
    for change in changes {
        let ctx = egui::Context::default();
        native_viewports(&ctx);
        if change != Change::First {
            Theme::vega().apply(&ctx);
        }
        let switch = std::cell::Cell::new(false);
        let frame = || {
            common::no_textures(ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.label("Accept terms");
                let builder = egui::ViewportBuilder::default();
                ui.ctx()
                    .show_viewport_immediate(child_viewport(), builder, |ui, _| {
                        ui.label("Settings");
                        if switch.get() {
                            match change {
                                Change::First => Theme::vega().apply(ui.ctx()),
                                Change::Fonts => oxanium().apply(ui.ctx()),
                                Change::Colours => swapped().apply(ui.ctx()),
                            }
                        }
                    });
            }))
        };
        for _ in 0..3 {
            let _ = frame();
        }
        let still = frame();
        assert_eq!(
            root_repaint_delay(&still),
            Duration::MAX,
            "{change:?}: a still frame"
        );
        switch.set(true);
        let out = frame();
        switch.set(false);
        delays.push((change, root_repaint_delay(&out)));
    }
    let repaint = changes.map(|change| (change, Duration::ZERO));
    assert_eq!(delays, repaint, "the root repaints");
}

/// The first `Theme::apply` in the root's pass, after a crate widget and
/// before a window (an immediate viewport) shown in every frame: the
/// window's pass starts in the theme's fonts, and the window binds its
/// waiting heading itself, though the root has already checked the
/// heading in a pass of the same number. No run of the window draws its
/// headings in the stand-in family.
#[test]
fn a_first_apply_before_a_window_binds_the_window_heading() {
    use shadcn_egui::components::label::label;
    let ctx = egui::Context::default();
    native_viewports(&ctx);
    let switch = std::cell::Cell::new(false);
    let families = std::cell::RefCell::new(Vec::new());
    let frame = || {
        common::no_textures(ctx.run_ui(egui::RawInput::default(), |ui| {
            drop(label(ui, "Accept terms"));
            if switch.get() {
                Theme::vega().apply(ui.ctx());
            }
            let builder = egui::ViewportBuilder::default();
            ui.ctx()
                .show_viewport_immediate(child_viewport(), builder, |ui, _| {
                    let heading = &ui.style().text_styles[&egui::TextStyle::Heading];
                    families.borrow_mut().push(heading.family.clone());
                })
        }))
    };
    let _ = frame();
    let _ = frame();
    switch.set(true);
    families.borrow_mut().clear();
    let _ = frame();
    let switch_runs = families.take();
    let _ = frame();
    let _ = frame();
    let settled = families.borrow().last().cloned().expect("a window run");
    assert_ne!(settled, egui::FontFamily::Proportional, "the heading binds");
    assert!(!switch_runs.is_empty(), "the window ran");
    for (run, family) in switch_runs.iter().enumerate() {
        assert_eq!(family, &settled, "window run {run} of the switch frame");
    }
}

/// `Theme::apply` of a theme with other colours and the same fonts in the
/// middle of a pass: no frame mixes the two palettes. Under `run_ui` the
/// frame egui returns draws the button before the call and the one after
/// it in the new colours; an app's own loop gets the pass marked for
/// discard and a repaint, and its next pass is in the new colours.
#[test]
fn a_colour_change_in_the_middle_of_a_pass_shows_no_mixed_frame() {
    let new = swapped().dark.primary;
    assert_ne!(new, Theme::vega().dark.primary, "the palettes differ");
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
                    swapped().apply(ui.ctx());
                }
                let after = ui.add(Button::new("After")).rect;
                rects.set([before, after]);
            })
        };
        let _ = frame();
        let _ = frame();
        switch.set(true);
        let out = frame();
        let [before, after] = rects.get();
        match app_loop {
            Loop::RunUi => {
                assert_eq!(fill_at(&out, before), new, "{app_loop:?}: before");
                assert_eq!(fill_at(&out, after), new, "{app_loop:?}: after");
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
                assert_eq!(fill_at(&out, before), new, "{app_loop:?}: next, before");
                assert_eq!(fill_at(&out, after), new, "{app_loop:?}: next, after");
            }
        }
    }
}

/// `Theme::apply` of one theme in every pass, after a crate widget or
/// before any, also one with other colours than the theme applied first:
/// once applied, the theme is unchanged, so no later pass is discarded
/// and a still frame asks for no repaint.
#[test]
fn the_same_theme_applied_every_pass_settles() {
    for (name, theme) in [("vega", Theme::vega()), ("swapped", swapped())] {
        for first in [false, true] {
            for app_loop in Loop::BOTH {
                let what = format!("{name}, first {first}, {app_loop:?}");
                let ctx = egui::Context::default();
                Theme::vega().apply(&ctx);
                let passes = std::cell::Cell::new(0);
                let frame = || {
                    passes.set(0);
                    common::pass(app_loop, &ctx, vec![], |ui| {
                        passes.set(passes.get() + 1);
                        if first {
                            theme.apply(ui.ctx());
                        }
                        ui.add(Button::new("A"));
                        if !first {
                            theme.apply(ui.ctx());
                        }
                        ui.add(Button::new("B"));
                    })
                };
                for _ in 0..2 {
                    let _ = frame();
                }
                for n in 0..4 {
                    let out = frame();
                    assert_eq!(passes.get(), 1, "{what}, frame {n}: passes");
                    assert!(
                        !out.platform_output.requested_discard(),
                        "{what}, frame {n}: no discard"
                    );
                    let delay = out.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
                    assert_eq!(delay, std::time::Duration::MAX, "{what}, frame {n}");
                }
            }
        }
    }
}

/// Two themes with other colours applied in every pass, each before some
/// widgets (styles side by side, an app variant on top of the page's
/// theme): every pass ends with the theme it started with, so after the
/// first frame no pass is discarded and a still frame asks for no
/// repaint.
#[test]
fn two_themes_applied_in_every_pass_settle() {
    for app_loop in Loop::BOTH {
        let ctx = egui::Context::default();
        let passes = std::cell::Cell::new(0);
        let frame = || {
            passes.set(0);
            common::pass(app_loop, &ctx, vec![], |ui| {
                passes.set(passes.get() + 1);
                // A crate call before the first apply sees the pass start.
                let _ = Theme::current(ui.ctx());
                Theme::vega().apply(ui.ctx());
                ui.add(Button::new("A"));
                // The second theme built from the one in effect, as an app
                // adds its variants to the page's theme.
                let _ = Theme::current(ui.ctx());
                swapped().apply(ui.ctx());
                ui.add(Button::new("B"));
            })
        };
        // The first frames see each theme change once; a repaint asked
        // for in a pass shows in the output of the next one too.
        for _ in 0..3 {
            let _ = frame();
        }
        for n in 0..4 {
            let out = frame();
            assert_eq!(passes.get(), 1, "{app_loop:?}, frame {n}: passes");
            assert!(
                !out.platform_output.requested_discard(),
                "{app_loop:?}, frame {n}: no discard"
            );
            let delay = out.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
            assert_eq!(delay, std::time::Duration::MAX, "{app_loop:?}, frame {n}");
        }
    }
}

/// Five themes applied in every pass, each before a widget (the styles or
/// presets of a picker side by side): more themes than any short list of
/// recent ones holds, and still every pass ends with the theme it started
/// with, so after the first frames no pass is discarded and a still frame
/// asks for no repaint.
#[test]
fn five_themes_applied_in_every_pass_settle() {
    for app_loop in Loop::BOTH {
        let ctx = egui::Context::default();
        let passes = std::cell::Cell::new(0);
        let frame = || {
            passes.set(0);
            common::pass(app_loop, &ctx, vec![], |ui| {
                passes.set(passes.get() + 1);
                for radius in [6.0, 7.0, 8.0, 9.0, 10.0] {
                    Theme::new(Style::vega(), Palette::dark(), Palette::light(), radius)
                        .apply(ui.ctx());
                    ui.add(Button::new("A"));
                    let _ = Theme::current(ui.ctx());
                }
            })
        };
        for _ in 0..3 {
            let _ = frame();
        }
        for n in 0..4 {
            let out = frame();
            assert_eq!(passes.get(), 1, "{app_loop:?}, frame {n}: passes");
            assert!(
                !out.platform_output.requested_discard(),
                "{app_loop:?}, frame {n}: no discard"
            );
            let delay = out.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
            assert_eq!(delay, std::time::Duration::MAX, "{app_loop:?}, frame {n}");
        }
    }
}

/// The root applies one theme and a window another in every pass (a
/// settings window that previews another palette): neither apply is a
/// change for the other viewport, so after the first frames neither
/// repaints the other. For a window run as eframe runs an immediate
/// viewport, inside the root's pass, and for one run as a deferred
/// viewport, in a pass of its own after the root's. A theme the root
/// applies once (a switch) still repaints the deferred window.
#[test]
fn a_window_with_its_own_theme_settles() {
    use std::time::Duration;
    let child = child_viewport();
    for deferred in [false, true] {
        let ctx = egui::Context::default();
        native_viewports(&ctx);
        let switch = std::cell::Cell::new(false);
        let window = |ui: &mut egui::Ui| {
            swapped().apply(ui.ctx());
            ui.add(Button::new("B"));
        };
        let viewports: egui::ViewportIdMap<egui::ViewportInfo> = [
            (egui::ViewportId::ROOT, egui::ViewportInfo::default()),
            (child, egui::ViewportInfo::default()),
        ]
        .into_iter()
        .collect();
        let frame = || {
            let input = egui::RawInput {
                viewports: viewports.clone(),
                ..Default::default()
            };
            let mut root = ctx.run_ui(input, |ui| {
                if switch.get() {
                    Theme::new(Style::vega(), Palette::light(), Palette::dark(), 4.0)
                        .apply(ui.ctx());
                } else {
                    Theme::vega().apply(ui.ctx());
                }
                ui.add(Button::new("A"));
                let builder = egui::ViewportBuilder::default();
                if deferred {
                    // The test runs the window's pass itself, below.
                    ui.ctx().show_viewport_deferred(child, builder, |_, _| {});
                } else {
                    ui.ctx()
                        .show_viewport_immediate(child, builder, |ui, _| window(ui));
                }
            });
            root.textures_delta.clear();
            let window_repaint = ctx.has_requested_repaint_for(&child);
            if deferred {
                let input = egui::RawInput {
                    viewport_id: child,
                    viewports: viewports.clone(),
                    ..Default::default()
                };
                ctx.run_ui(input, window).drop_without_applying_deltas();
            }
            let root_repaint = ctx.has_requested_repaint_for(&egui::ViewportId::ROOT);
            (root_repaint_delay(&root), window_repaint, root_repaint)
        };
        for _ in 0..3 {
            let _ = frame();
        }
        for n in 0..4 {
            let (delay, window_repaint, root_repaint) = frame();
            let what = format!("deferred {deferred}, frame {n}");
            assert_eq!(delay, Duration::MAX, "{what}: root delay");
            assert!(!root_repaint, "{what}: the window repaints the root");
            if deferred {
                assert!(!window_repaint, "{what}: the root repaints the window");
            }
        }
        if deferred {
            switch.set(true);
            let (_, window_repaint, _) = frame();
            assert!(window_repaint, "a switch in the root repaints the window");
        }
    }
}

/// Vega with an Inter body and an Oxanium heading: fonts that differ from
/// both `Theme::vega()` and [`oxanium`], with the body of vega.
fn inter_body_oxanium_heading() -> Theme {
    let mut t = Theme::vega();
    t.fonts = shadcn_egui::ThemeFonts {
        body: &shadcn_egui_font_inter::FONT,
        heading: &shadcn_egui_font_oxanium::FONT,
    };
    t
}

/// The root applies vega and a window applies a theme with other fonts in
/// every pass. Fonts belong to the context, so both draw in the fonts of
/// the last real change (the window opening); neither apply is a change
/// for the other viewport: after the first frames no pass is redone for
/// new fonts, and neither viewport repaints the other. Once the window is
/// closed, the root's fonts install again, and a window that opens again
/// installs its own. Fonts the root applies once (a switch) still install:
/// the root draws in them, and the deferred window is asked to repaint.
/// For an immediate and a deferred window, as in
/// [`a_window_with_its_own_theme_settles`].
#[test]
fn a_window_with_its_own_fonts_settles() {
    use shadcn_egui::components::label::label;
    use std::time::Duration;
    let (vega, new) = (label_width(&Theme::vega()), label_width(&oxanium()));
    assert_ne!(vega, new, "the two fonts measure apart");
    let child = child_viewport();
    for deferred in [false, true] {
        let ctx = egui::Context::default();
        native_viewports(&ctx);
        let (open, switch) = (std::cell::Cell::new(true), std::cell::Cell::new(false));
        let width = std::cell::Cell::new(0.0);
        let window = |ui: &mut egui::Ui| {
            oxanium().apply(ui.ctx());
            drop(label(ui, "Settings"));
        };
        let viewports = || -> egui::ViewportIdMap<egui::ViewportInfo> {
            let mut ids = vec![egui::ViewportId::ROOT];
            if open.get() {
                ids.push(child);
            }
            ids.into_iter()
                .map(|id| (id, egui::ViewportInfo::default()))
                .collect()
        };
        let frame = || {
            let input = egui::RawInput {
                viewports: viewports(),
                ..Default::default()
            };
            let mut passes = 0;
            let mut root = ctx.run_ui(input, |ui| {
                passes += 1;
                if switch.get() {
                    inter_body_oxanium_heading().apply(ui.ctx());
                } else {
                    Theme::vega().apply(ui.ctx());
                }
                width.set(label(ui, "Accept terms").rect.width());
                let builder = egui::ViewportBuilder::default();
                match (open.get(), deferred) {
                    (false, _) => {}
                    // The test runs the window's pass itself, below.
                    (true, true) => ui.ctx().show_viewport_deferred(child, builder, |_, _| {}),
                    (true, false) => {
                        ui.ctx()
                            .show_viewport_immediate(child, builder, |ui, _| window(ui));
                    }
                }
            });
            root.textures_delta.clear();
            let window_repaint = ctx.has_requested_repaint_for(&child);
            if deferred && open.get() {
                let input = egui::RawInput {
                    viewport_id: child,
                    viewports: viewports(),
                    ..Default::default()
                };
                ctx.run_ui(input, window).drop_without_applying_deltas();
            }
            let root_repaint = ctx.has_requested_repaint_for(&egui::ViewportId::ROOT);
            (
                passes,
                root_repaint_delay(&root),
                window_repaint,
                root_repaint,
            )
        };
        let settled = |phase: &str, fonts: f32| {
            for n in 0..4 {
                let (passes, delay, window_repaint, root_repaint) = frame();
                let what = format!("deferred {deferred}, {phase}, frame {n}");
                assert_eq!(passes, 1, "{what}: root passes");
                assert_eq!(delay, Duration::MAX, "{what}: root delay");
                assert!(!root_repaint, "{what}: the window repaints the root");
                if deferred && open.get() {
                    assert!(!window_repaint, "{what}: the root repaints the window");
                }
                assert_eq!(width.get(), fonts, "{what}: the root's fonts");
            }
        };
        for _ in 0..3 {
            let _ = frame();
        }
        settled("window open", new);
        open.set(false);
        for _ in 0..2 {
            let _ = frame();
        }
        settled("window closed", vega);
        open.set(true);
        for _ in 0..3 {
            let _ = frame();
        }
        settled("window open again", new);
        switch.set(true);
        let (_, _, window_repaint, _) = frame();
        if deferred {
            assert!(
                window_repaint,
                "a font switch in the root repaints the window"
            );
        }
        let _ = frame();
        settled("root switched", vega);
    }
}

/// Two themes with different fonts applied in every pass of one viewport
/// (the styles of a picker side by side): the context keeps the fonts of
/// the last real change, so after the first frames no pass is redone and a
/// still frame asks for no repaint. Once the pass applies only one of the
/// themes, its fonts install.
#[test]
fn two_fonts_applied_in_every_pass_settle() {
    use shadcn_egui::components::label::label;
    let (vega, new) = (label_width(&Theme::vega()), label_width(&oxanium()));
    for app_loop in Loop::BOTH {
        let ctx = egui::Context::default();
        let both = std::cell::Cell::new(true);
        let passes = std::cell::Cell::new(0);
        let width = std::cell::Cell::new(0.0);
        let frame = || {
            passes.set(0);
            common::pass(app_loop, &ctx, vec![], |ui| {
                passes.set(passes.get() + 1);
                Theme::vega().apply(ui.ctx());
                width.set(label(ui, "Accept terms").rect.width());
                if both.get() {
                    oxanium().apply(ui.ctx());
                    drop(label(ui, "Settings"));
                }
            })
        };
        let settled = |phase: &str, fonts: f32| {
            for n in 0..4 {
                let out = frame();
                let what = format!("{app_loop:?}, {phase}, frame {n}");
                assert_eq!(passes.get(), 1, "{what}: passes");
                assert!(
                    !out.platform_output.requested_discard(),
                    "{what}: no discard"
                );
                let delay = out.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
                assert_eq!(delay, std::time::Duration::MAX, "{what}: delay");
                assert_eq!(width.get(), fonts, "{what}: the fonts");
            }
        };
        for _ in 0..3 {
            let _ = frame();
        }
        settled("both themes", new);
        both.set(false);
        for _ in 0..3 {
            let _ = frame();
        }
        settled("vega only", vega);
    }
}

/// An app applies two themes with other fonts in every pass, and once
/// applies the first through `apply_with_fonts` with definitions of its
/// own: the install gives way to the fonts of the last change, but the
/// app's own family goes in.
#[test]
fn own_definitions_go_in_while_two_fonts_alternate() {
    use shadcn_egui::components::label::label;
    let own_family = egui::FontFamily::Name("app-own".into());
    let own_defs = || {
        let mut defs = egui::FontDefinitions::default();
        let face = defs
            .font_data
            .keys()
            .next()
            .expect("a default face")
            .clone();
        defs.families.insert(own_family.clone(), vec![face]);
        defs
    };
    for app_loop in Loop::BOTH {
        let ctx = egui::Context::default();
        let own = std::cell::Cell::new(false);
        let frame = || {
            common::pass(app_loop, &ctx, vec![], |ui| {
                if own.get() {
                    Theme::vega().apply_with_fonts(ui.ctx(), own_defs());
                } else {
                    Theme::vega().apply(ui.ctx());
                }
                drop(label(ui, "Accept terms"));
                oxanium().apply(ui.ctx());
                drop(label(ui, "Settings"));
            })
        };
        for _ in 0..4 {
            let _ = frame();
        }
        own.set(true);
        let _ = frame();
        own.set(false);
        for _ in 0..2 {
            let _ = frame();
        }
        let has_own = ctx.fonts(|f| f.definitions().families.contains_key(&own_family));
        assert!(has_own, "{app_loop:?}: the app's own family");
    }
}

/// One pass of `app_loop` with `input`: `run_ui`, or an app's own
/// `begin_pass`/`end_pass` loop with a root `Ui` built by hand.
fn run_with(
    app_loop: Loop,
    ctx: &egui::Context,
    input: egui::RawInput,
    mut ui_fn: impl FnMut(&mut egui::Ui),
) -> common::Output {
    common::Output(match app_loop {
        Loop::RunUi => ctx.run_ui(input, ui_fn),
        Loop::Own => {
            ctx.begin_pass(input);
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, common::PASS_WINDOW);
            let builder = egui::UiBuilder::new().max_rect(rect);
            ui_fn(&mut egui::Ui::new(
                ctx.clone(),
                egui::Id::unique("own"),
                builder,
            ));
            ctx.end_pass()
        }
    })
}

/// [`native_viewports`], with the immediate window's pass run by
/// `app_loop`.
fn native_viewports_in(ctx: &egui::Context, app_loop: Loop) {
    ctx.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(move |ctx, mut viewport| {
        let id = viewport.ids.this;
        let input = egui::RawInput {
            viewport_id: id,
            viewports: [(id, egui::ViewportInfo::default())].into_iter().collect(),
            ..Default::default()
        };
        let _ = run_with(app_loop, ctx, input, |ui| (viewport.viewport_ui_cb)(ui));
    });
}

/// The root switches to dark and a window to light in every pass (a
/// window that keeps its own mode). The mode belongs to the context, so
/// each viewport's pass starts in the mode the other one left; a mode the
/// viewport switched to in its previous pass too is the app's pattern, not
/// a switch: after the first frames no pass is discarded and neither
/// viewport repaints the other. Once the window is closed, a real switch
/// in the root (light after its first widget) still discards that pass.
/// For both loops, and an immediate and a deferred window.
#[test]
fn a_window_in_another_mode_settles() {
    use std::time::Duration;
    let child = child_viewport();
    for app_loop in Loop::BOTH {
        for deferred in [false, true] {
            let ctx = egui::Context::default();
            Theme::vega().apply(&ctx);
            native_viewports_in(&ctx, app_loop);
            let (open, switch) = (std::cell::Cell::new(true), std::cell::Cell::new(false));
            let window = |ui: &mut egui::Ui| {
                // A crate call before the switch sees the pass start, as
                // the begin hook of `run_ui` does.
                let _ = Theme::current(ui.ctx());
                ui.ctx().set_theme(egui::Theme::Light);
                ui.add(Button::new("B"));
            };
            let viewports = || -> egui::ViewportIdMap<egui::ViewportInfo> {
                let mut ids = vec![egui::ViewportId::ROOT];
                if open.get() {
                    ids.push(child);
                }
                ids.into_iter()
                    .map(|id| (id, egui::ViewportInfo::default()))
                    .collect()
            };
            let frame = || {
                let input = egui::RawInput {
                    viewports: viewports(),
                    ..Default::default()
                };
                let mut passes = 0;
                let root = run_with(app_loop, &ctx, input, |ui| {
                    passes += 1;
                    let _ = Theme::current(ui.ctx());
                    ui.ctx().set_theme(egui::Theme::Dark);
                    ui.add(Button::new("A"));
                    if switch.get() {
                        ui.ctx().set_theme(egui::Theme::Light);
                    }
                    ui.add(Button::new("C"));
                    let builder = egui::ViewportBuilder::default();
                    match (open.get(), deferred) {
                        (false, _) => {}
                        // The test runs the window's pass itself, below.
                        (true, true) => ui.ctx().show_viewport_deferred(child, builder, |_, _| {}),
                        (true, false) => {
                            ui.ctx()
                                .show_viewport_immediate(child, builder, |ui, _| window(ui));
                        }
                    }
                });
                let window_repaint = ctx.has_requested_repaint_for(&child);
                let mut window_discard = false;
                if deferred && open.get() {
                    let input = egui::RawInput {
                        viewport_id: child,
                        viewports: viewports(),
                        ..Default::default()
                    };
                    let out = run_with(app_loop, &ctx, input, window);
                    window_discard = out.platform_output.requested_discard();
                }
                let root_repaint = ctx.has_requested_repaint_for(&egui::ViewportId::ROOT);
                (passes, root, window_repaint, root_repaint, window_discard)
            };
            let settled = |phase: &str| {
                for n in 0..4 {
                    let (passes, root, window_repaint, root_repaint, window_discard) = frame();
                    let what = format!("{app_loop:?}, deferred {deferred}, {phase}, frame {n}");
                    assert_eq!(passes, 1, "{what}: root passes");
                    assert!(
                        !root.platform_output.requested_discard(),
                        "{what}: root discard"
                    );
                    assert!(!window_discard, "{what}: window discard");
                    assert_eq!(
                        root_repaint_delay(&root),
                        Duration::MAX,
                        "{what}: root delay"
                    );
                    assert!(!root_repaint, "{what}: the window repaints the root");
                    if deferred && open.get() {
                        assert!(!window_repaint, "{what}: the root repaints the window");
                    }
                }
            };
            for _ in 0..3 {
                let _ = frame();
            }
            settled("window open");
            open.set(false);
            for _ in 0..3 {
                let _ = frame();
            }
            settled("window closed");
            switch.set(true);
            let (passes, root, ..) = frame();
            let what = format!("{app_loop:?}, deferred {deferred}, root switched");
            match app_loop {
                Loop::RunUi => assert_eq!(passes, 2, "{what}: the pass is redone"),
                Loop::Own => {
                    assert!(
                        root.platform_output.requested_discard(),
                        "{what}: the mixed pass asks to be discarded"
                    );
                    assert_eq!(root_repaint_delay(&root), Duration::ZERO, "{what}: repaint");
                }
            }
        }
    }
}

/// One viewport that draws in dark and then in light in every pass (a
/// preview of each mode side by side): each pass starts in the mode the
/// last one left, so after the first frames no pass is discarded and a
/// still frame asks for no repaint.
#[test]
fn two_modes_in_every_pass_settle() {
    for app_loop in Loop::BOTH {
        let ctx = egui::Context::default();
        Theme::vega().apply(&ctx);
        let passes = std::cell::Cell::new(0);
        let frame = || {
            passes.set(0);
            common::pass(app_loop, &ctx, vec![], |ui| {
                passes.set(passes.get() + 1);
                let _ = Theme::current(ui.ctx());
                ui.ctx().set_theme(egui::Theme::Dark);
                ui.add(Button::new("A"));
                ui.ctx().set_theme(egui::Theme::Light);
                ui.add(Button::new("B"));
            })
        };
        for _ in 0..3 {
            let _ = frame();
        }
        for n in 0..4 {
            let out = frame();
            assert_eq!(passes.get(), 1, "{app_loop:?}, frame {n}: passes");
            assert!(
                !out.platform_output.requested_discard(),
                "{app_loop:?}, frame {n}: no discard"
            );
            let delay = out.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
            assert_eq!(delay, std::time::Duration::MAX, "{app_loop:?}, frame {n}");
        }
    }
}
