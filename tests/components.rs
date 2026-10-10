mod common;

use egui_shadcn::components::shared::mix_toward;

#[test]
fn mix_toward_blends() {
    let a = egui::Color32::from_rgb(200, 0, 0);
    let b = egui::Color32::from_rgb(0, 0, 0);
    let m = mix_toward(a, b, 0.5);
    assert!(m.r() > 90 && m.r() < 110, "got {}", m.r());
}

#[test]
fn button_click_sets_flag() {
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use egui_shadcn::components::button::Button;
    use egui_shadcn::Theme;
    use std::cell::Cell;
    let clicked = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        Theme::vega_dark().apply(ui.ctx());
        if ui.add(Button::new("Save")).clicked() {
            clicked.set(true);
        }
    });
    h.get_by_label("Save").click();
    h.run();
    assert!(clicked.get(), "button click did not register");
}

#[test]
fn button_height_follows_style() {
    use egui_shadcn::components::button::Button;
    for (name, theme) in common::themes() {
        let want = if name.starts_with("nova") { 32.0 } else { 36.0 };
        let h = common::measure(&theme, |ui| ui.add(Button::new("Save"))).height();
        assert_eq!(h, want, "{name}");
    }
}

/// A project deviates from the style when it builds the theme, not in the
/// vendored component.
#[test]
fn button_override_from_theme_setup() {
    use egui_shadcn::components::button::Button;
    use egui_shadcn::style::Style;
    use egui_shadcn::{Palette, Theme};
    let mut s = Style::nova();
    s.button_size_default.height.base = Some(40.0);
    let theme = Theme::new(s, Palette::dark(), 10.0);
    let h = common::measure(&theme, |ui| ui.add(Button::new("Save"))).height();
    assert_eq!(h, 40.0);
}

#[test]
fn button_snapshot() {
    use egui_shadcn::components::button::{Button, ButtonSize, ButtonVariant};
    use egui_shadcn::components::icon::CLOSE;
    use egui_shadcn::layout;
    common::snapshot_all("button", egui::vec2(720.0, 200.0), |ui| {
        layout::vstack(ui, 12.0, |ui| {
            layout::row(ui, 8.0, |ui| {
                ui.add(Button::new("Default"));
                ui.add(Button::new("Secondary").variant(ButtonVariant::Secondary));
                ui.add(Button::new("Outline").variant(ButtonVariant::Outline));
                ui.add(Button::new("Ghost").variant(ButtonVariant::Ghost));
                ui.add(Button::new("Delete").variant(ButtonVariant::Destructive));
                ui.add(Button::new("Link").variant(ButtonVariant::Link));
            });
            layout::row(ui, 8.0, |ui| {
                ui.add(Button::new("Xs").size(ButtonSize::Xs));
                ui.add(Button::new("Small").size(ButtonSize::Sm));
                ui.add(Button::new("Default"));
                ui.add(Button::new("Large").size(ButtonSize::Lg));
                ui.add(Button::new("Close").icon(&CLOSE).size(ButtonSize::Icon).variant(ButtonVariant::Outline));
                ui.add(Button::new("With icon").icon(&CLOSE));
            });
            layout::row(ui, 8.0, |ui| {
                ui.add(Button::new("Disabled").enabled(false));
                ui.add(Button::new("Invalid").variant(ButtonVariant::Outline).invalid(true));
                ui.add(Button::new("Open").variant(ButtonVariant::Outline).expanded(true));
            });
        });
    });
}

/// The shared Response-to-State mapping: a disabled widget gets `DISABLED`
/// and loses every pointer and focus bit, also one its caller passed in.
#[test]
fn interaction_state_of_disabled_widget_has_no_pointer_or_focus() {
    use egui_shadcn::components::shared::interaction_state;
    use egui_shadcn::style::State;
    use egui_shadcn::Theme;
    let theme = Theme::vega_dark();
    let target = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(80.0, 30.0));
    let ctx = egui::Context::default();
    let (mut live, mut disabled) = (State::NONE, State::NONE);
    for events in [vec![], pointer_down(target.center()), vec![]] {
        let _ = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
            let resp = ui.interact(target, egui::Id::new("target"), egui::Sense::click());
            resp.request_focus();
            live = interaction_state(&theme, &resp, true, State::INVALID);
            disabled = interaction_state(&theme, &resp, false, State::INVALID | State::FOCUS);
        });
    }
    // Control: the same response, enabled, is hovered, pressed and focused.
    let pointer = State::HOVER | State::ACTIVE | State::FOCUS_VISIBLE;
    assert_eq!(live, pointer | State::INVALID | State::DARK);
    assert_eq!(disabled, State::INVALID | State::DISABLED | State::DARK);
}

/// Every rect shape a frame painted, in paint order.
fn rect_shapes(out: &egui::FullOutput) -> Vec<egui::epaint::RectShape> {
    fn walk(s: &egui::Shape, out: &mut Vec<egui::epaint::RectShape>) {
        match s {
            egui::Shape::Rect(r) => out.push(r.clone()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut rects = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut rects));
    rects
}

/// shadcn `bg-clip-padding` with `border border-transparent`: a solid
/// button's fill stops inside its 1px transparent border.
#[test]
fn button_fill_stops_inside_transparent_border() {
    use egui_shadcn::components::button::Button;
    use egui_shadcn::Theme;
    let theme = Theme::vega_light();
    let (rect, out) = common::measure_output(&theme, |ui| ui.add(Button::new("Save")));
    let fill = theme.palette.primary;
    let fills: Vec<_> = rect_shapes(&out).into_iter().filter(|r| r.fill == fill).collect();
    assert_eq!(fills.len(), 1, "{fills:?}");
    assert_eq!(fills[0].rect, rect.shrink(1.0));
}

/// shadcn `focus-visible:border-ring focus-visible:ring-3 ring-ring/50`: a
/// focused button gets a ring-coloured border and a 3px ring at half alpha.
#[test]
fn focused_button_shows_ring_and_ring_border() {
    use egui_shadcn::components::button::Button;
    use egui_shadcn::Theme;
    let theme = Theme::vega_light();
    let (rect, out) = common::measure_output(&theme, |ui| {
        let resp = ui.add(Button::new("Save"));
        resp.request_focus();
        resp
    });
    let ring = theme.palette.ring;
    let shapes = rect_shapes(&out);
    let ring_shape = shapes.iter().find(|r| r.stroke.width == 3.0);
    let ring_shape = ring_shape.unwrap_or_else(|| panic!("no 3px ring in {shapes:?}"));
    assert_eq!(ring_shape.stroke.color, ring.gamma_multiply(0.5));
    assert_eq!(ring_shape.stroke_kind, egui::StrokeKind::Outside);
    assert_eq!(ring_shape.rect, rect);
    assert!(shapes.iter().any(|r| r.stroke.width == 1.0 && r.stroke.color == ring), "no ring-coloured border in {shapes:?}");
}

/// Move the pointer to `pos` and press the primary button there.
fn pointer_down(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE },
    ]
}

#[test]
fn input_accepts_text() {
    use egui_kittest::Harness;
    use egui_shadcn::components::input::Input;
    use egui_shadcn::Theme;
    let mut text = String::new();
    let mut h = Harness::new_ui(|ui| {
        Theme::dark().apply(ui.ctx());
        ui.add(Input::new(&mut text).hint("Email"));
    });
    h.run();
    assert!(h.ctx.viewport_rect().width() > 0.0);
}

#[test]
fn label_input_snapshot() {
    use egui_kittest::Harness;
    use egui_shadcn::components::input::Input;
    use egui_shadcn::components::label::{description, label};
    use egui_shadcn::Theme;
    let mut h = Harness::builder()
        .with_size(egui::vec2(360.0, 140.0))
        .build_ui(|ui| {
            Theme::dark().apply(ui.ctx());
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    label(ui, "Email");
                    let mut s = String::from("you@example.com");
                    ui.add(Input::new(&mut s));
                    description(ui, "We'll never share your email.");
                });
            });
        });
    h.run();
    h.snapshot("label_input");
}

#[test]
fn switch_toggles() {
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use egui_shadcn::components::switch::toggle;
    use egui_shadcn::Theme;
    use std::cell::Cell;
    let on = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        Theme::dark().apply(ui.ctx());
        let mut v = on.get();
        if toggle(ui, &mut v).clicked() {
            on.set(v);
        }
    });
    h.run();
    h.get_by_role(egui::accesskit::Role::CheckBox).click();
    h.run();
    assert!(on.get(), "switch did not toggle on");
}

#[test]
fn switch_checkbox_snapshot() {
    use egui_kittest::Harness;
    use egui_shadcn::components::checkbox::checkbox;
    use egui_shadcn::components::switch::toggle;
    use egui_shadcn::{layout, Theme};
    let mut h = Harness::builder()
        .with_size(egui::vec2(220.0, 120.0))
        .build_ui(|ui| {
            Theme::dark().apply(ui.ctx());
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    layout::row(ui, 8.0, |ui| {
                        let mut on = true;
                        toggle(ui, &mut on);
                        let mut off = false;
                        toggle(ui, &mut off);
                    });
                    ui.add_space(12.0);
                    layout::row(ui, 8.0, |ui| {
                        let mut c1 = true;
                        checkbox(ui, &mut c1);
                        let mut c2 = false;
                        checkbox(ui, &mut c2);
                    });
                });
            });
        });
    h.run();
    h.snapshot("switch_checkbox");
}

#[test]
fn tabs_select() {
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use egui_shadcn::components::tabs::tab_bar;
    use egui_shadcn::Theme;
    use std::cell::Cell;
    let active = Cell::new(0usize);
    let mut h = Harness::new_ui(|ui| {
        Theme::dark().apply(ui.ctx());
        let mut sel = active.get();
        tab_bar(ui, &mut sel, &["Account", "Notifications", "Display"]);
        active.set(sel);
    });
    h.run();
    h.get_by_label("Notifications").click();
    h.run();
    assert_eq!(active.get(), 1);
}

#[test]
fn tabs_snapshot() {
    use egui_kittest::Harness;
    use egui_shadcn::components::tabs::tab_bar;
    use egui_shadcn::Theme;
    let mut h = Harness::builder()
        .with_size(egui::vec2(420.0, 70.0))
        .build_ui(|ui| {
            Theme::dark().apply(ui.ctx());
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                let mut sel = 0usize;
                tab_bar(ui, &mut sel, &["Account", "Notifications"]);
            });
        });
    h.run();
    h.snapshot("tabs");
}

#[test]
fn badge_is_h5_in_both_styles() {
    use egui_shadcn::components::badge::{badge, BadgeVariant};
    for (name, theme) in common::themes() {
        let h = common::measure(&theme, |ui| badge(ui, "New", BadgeVariant::Default)).height();
        assert_eq!(h, 20.0, "{name}");
    }
}

#[test]
fn badge_snapshot() {
    use egui_shadcn::components::badge::{badge, BadgeVariant::*};
    common::snapshot_all("badge", egui::vec2(480.0, 64.0), |ui| {
        egui_shadcn::layout::row(ui, 8.0, |ui| {
            for (t, v) in [("Default", Default), ("Secondary", Secondary), ("Destructive", Destructive), ("Outline", Outline), ("Ghost", Ghost), ("Link", Link)] {
                badge(ui, t, v);
            }
        });
    });
}

/// The colour of the first text a frame painted.
fn text_colour(out: &egui::FullOutput) -> egui::Color32 {
    fn walk(s: &egui::Shape) -> Option<egui::Color32> {
        match s {
            egui::Shape::Text(t) => Some(t.galley.job.sections[0].format.color),
            egui::Shape::Vec(v) => v.iter().find_map(walk),
            _ => None,
        }
    }
    out.shapes.iter().find_map(|c| walk(&c.shape)).expect("a text shape")
}

/// shadcn `peer-disabled:opacity-50`: a label of a disabled control paints
/// its text at half alpha, and `text-sm leading-none` makes it 14px high.
#[test]
fn disabled_label_paints_half_alpha_text() {
    use egui_shadcn::components::label::Label;
    use egui_shadcn::style::{pick, State};
    for (name, theme) in common::themes() {
        let (rect, on) = common::measure_output(&theme, |ui| ui.add(Label::new("Email")));
        let (_, off) = common::measure_output(&theme, |ui| ui.add(Label::new("Email").enabled(false)));
        let fg = theme.palette.foreground;
        assert_eq!(text_colour(&on), fg, "{name}");
        assert_eq!(text_colour(&off), fg.gamma_multiply(0.5), "{name}");
        assert_eq!(rect.height(), 14.0, "{name}");
        assert_eq!(pick(&[&theme.style.label], |r| &r.opacity, State::DISABLED), Some(0.5), "{name}");
    }
}

/// The separator is a 1px `bg-border` line: full width when horizontal,
/// full row height when vertical.
#[test]
fn separators_are_one_pixel_border_lines() {
    use egui_shadcn::components::separator::{separator, separator_vertical};
    let theme = egui_shadcn::Theme::vega_light();
    let (_, out) = common::measure_output(&theme, |ui| {
        ui.allocate_response(egui::vec2(200.0, 0.0), egui::Sense::hover());
        separator(ui);
        ui.allocate_response(egui::vec2(0.0, 0.0), egui::Sense::hover())
    });
    let lines: Vec<_> = rect_shapes(&out).into_iter().filter(|r| r.fill == theme.palette.border).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].rect.height(), 1.0);
    assert!(lines[0].rect.width() >= 200.0);
    let (_, out) = common::measure_output(&theme, |ui| {
        ui.horizontal(|ui| {
            ui.set_height(24.0);
            separator_vertical(ui);
        })
        .response
    });
    let v: Vec<_> = rect_shapes(&out).into_iter().filter(|r| r.fill == theme.palette.border).collect();
    assert_eq!(v.len(), 1, "{v:?}");
    assert_eq!(v[0].rect.width(), 1.0);
    assert_eq!(v[0].rect.height(), 24.0);
}

#[test]
fn label_separator_snapshot() {
    use egui_shadcn::components::label::{description, Label};
    use egui_shadcn::components::separator::{separator, separator_vertical};
    common::snapshot_all("label_separator", egui::vec2(360.0, 200.0), |ui| {
        ui.add(Label::new("Email"));
        ui.add(Label::new("Email (disabled)").enabled(false));
        description(ui, "We'll never share your email.");
        separator(ui);
        ui.horizontal(|ui| {
            ui.set_height(20.0);
            ui.label("Docs");
            separator_vertical(ui);
            ui.label("Source");
            separator_vertical(ui);
            ui.label("Blog");
        });
    });
}

#[test]
fn select_and_badge_render() {
    use egui_kittest::Harness;
    use egui_shadcn::components::badge::{badge, BadgeVariant};
    use egui_shadcn::components::select::select;
    use egui_shadcn::Theme;
    let mut h = Harness::new_ui(|ui| {
        Theme::dark().apply(ui.ctx());
        let mut choice = 0usize;
        select(ui, "lang", &mut choice, &["English", "German"]);
        badge(ui, "New", BadgeVariant::Default);
    });
    h.run();
    assert!(h.ctx.viewport_rect().width() > 0.0);
}

#[test]
fn card_badge_select_snapshot() {
    use egui_kittest::Harness;
    use egui_shadcn::components::badge::{badge, BadgeVariant};
    use egui_shadcn::components::card::{card_description, card_title};
    use egui_shadcn::components::select::select;
    use egui_shadcn::{layout, Theme};
    let mut h = Harness::builder()
        .with_size(egui::vec2(420.0, 220.0))
        .build_ui(|ui| {
            Theme::dark().apply(ui.ctx());
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    layout::card(ui, |ui| {
                        card_title(ui, "Notifications");
                        card_description(ui, "Choose what you want to be notified about.");
                        ui.add_space(8.0);
                        layout::row(ui, 6.0, |ui| {
                            badge(ui, "Default", BadgeVariant::Default);
                            badge(ui, "Secondary", BadgeVariant::Secondary);
                            badge(ui, "New", BadgeVariant::Destructive);
                            badge(ui, "Outline", BadgeVariant::Outline);
                        });
                        ui.add_space(8.0);
                        let mut choice = 0usize;
                        select(ui, "lang", &mut choice, &["English", "German"]);
                    });
                });
            });
        });
    h.run();
    h.snapshot("card_badge_select");
}

/// shadcn `disabled:pointer-events-none`: no click, and a pressed pointer
/// over the button changes nothing it paints (no hover fill, no press nudge).
#[test]
fn disabled_button_ignores_clicks() {
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use egui_shadcn::components::button::Button;
    use egui_shadcn::Theme;
    use std::cell::Cell;
    let clicked = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        Theme::vega_dark().apply(ui.ctx());
        if ui.add(Button::new("Save").enabled(false)).clicked() {
            clicked.set(true);
        }
    });
    h.get_by_label("Save").click();
    h.run();
    assert!(!clicked.get(), "disabled button registered a click");

    /// The button's rect shapes at rest and with the pointer held down on it.
    fn at_rest_and_pressed(enabled: bool) -> (Vec<egui::epaint::RectShape>, Vec<egui::epaint::RectShape>) {
        fn frame(ctx: &egui::Context, enabled: bool, events: Vec<egui::Event>) -> (egui::Rect, Vec<egui::epaint::RectShape>) {
            let mut rect = egui::Rect::NOTHING;
            let out = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
                Theme::vega_dark().apply(ui.ctx());
                rect = ui.add(Button::new("Save").enabled(enabled)).rect;
            });
            (rect, rect_shapes(&out))
        }
        let ctx = egui::Context::default();
        frame(&ctx, enabled, vec![]);
        let (rect, rest) = frame(&ctx, enabled, vec![]);
        frame(&ctx, enabled, pointer_down(rect.center()));
        (rest, frame(&ctx, enabled, vec![]).1)
    }
    // Control: an enabled button does change under a pressed pointer.
    let (rest, pressed) = at_rest_and_pressed(true);
    assert_ne!(rest, pressed, "enabled button shows no hover or press");
    let (rest, pressed) = at_rest_and_pressed(false);
    assert_eq!(rest, pressed, "disabled button shows hover or press styling");
}

/// shadcn `focus-visible:border-ring`: the input's own 1px border turns
/// ring-colored while focused, under the separate focus ring.
#[test]
fn focused_input_border_turns_ring_colored() {
    use egui_shadcn::components::input::Input;
    use egui_shadcn::Theme;

    fn border_colors(ctx: &egui::Context, text: &mut String, focus: bool) -> Vec<egui::Color32> {
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 200.0))),
            ..Default::default()
        };
        let out = ctx.run_ui(raw, |ui| {
            Theme::dark().apply(ui.ctx());
            let resp = ui.add(Input::new(text).id_source("field"));
            if focus {
                resp.request_focus();
            }
        });
        fn walk(s: &egui::Shape, out: &mut Vec<egui::Color32>) {
            match s {
                egui::Shape::Rect(r) if r.stroke.width == 1.0 => out.push(r.stroke.color),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
                _ => {}
            }
        }
        let mut colors = Vec::new();
        out.shapes.iter().for_each(|c| walk(&c.shape, &mut colors));
        colors
    }

    let ctx = egui::Context::default();
    let mut text = String::new();
    let p = Theme::dark().palette;
    // Our border is painted over egui's own TextEdit frame (which already uses
    // `selection.stroke` on focus), so the visible border is the last 1px stroke.
    let unfocused = border_colors(&ctx, &mut text, false);
    assert_eq!(unfocused.last(), Some(&p.border), "unfocused border: {unfocused:?}");
    border_colors(&ctx, &mut text, true); // focus lands next frame
    let focused = border_colors(&ctx, &mut text, false);
    assert_eq!(focused.last(), Some(&p.ring), "focused border: {focused:?}");
}

/// `Input::id_source` gives the field an id that does not depend on how many
/// widgets precede it, so callers can address it from outside.
#[test]
fn input_id_source_is_stable() {
    use egui_shadcn::components::input::Input;
    fn id_after(ctx: &egui::Context, preceding: usize, salt: Option<&'static str>) -> egui::Id {
        let mut text = String::new();
        let mut id = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            for i in 0..preceding {
                ui.label(format!("filler {i}"));
            }
            let input = Input::new(&mut text);
            let input = match salt {
                Some(s) => input.id_source(s),
                None => input,
            };
            id = Some(ui.add(input).id);
        });
        id.expect("input was added")
    }
    let ctx = egui::Context::default();
    // Control: without a salt the id follows widget position.
    assert_ne!(id_after(&ctx, 1, None), id_after(&ctx, 3, None));
    assert_eq!(id_after(&ctx, 1, Some("login-user")), id_after(&ctx, 3, Some("login-user")));
}
