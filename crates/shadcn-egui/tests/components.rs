mod common;

use shadcn_egui::components::badge::{badge, BadgeVariant};
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::components::card::{
    card_content, card_header, card_header_with_action, card_title, Card, CardSize,
};
use shadcn_egui::style::Style;
use shadcn_egui::{Palette, Theme};

#[test]
fn button_click_sets_flag() {
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use shadcn_egui::components::button::Button;
    use shadcn_egui::Theme;
    use std::cell::Cell;
    let clicked = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        Theme::vega().apply(ui.ctx());
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
    use shadcn_egui::components::button::Button;
    for (name, theme, mode) in common::themes() {
        let want = if name.starts_with("nova") { 32.0 } else { 36.0 };
        let h = common::measure(&theme, mode, |ui| ui.add(Button::new("Save"))).height();
        assert_eq!(h, want, "{name}");
    }
}

/// A project deviates from the style when it builds the theme, not in the
/// vendored component.
#[test]
fn button_override_from_theme_setup() {
    use shadcn_egui::components::button::Button;
    use shadcn_egui::style::Style;
    use shadcn_egui::{Palette, Theme};
    let mut s = Style::nova();
    s.button_size_default.height.base = Some(40.0);
    let mode = egui::Theme::Dark;
    let theme = Theme::new(s, Palette::light(), Palette::dark(), 10.0);
    let h = common::measure(&theme, mode, |ui| ui.add(Button::new("Save"))).height();
    assert_eq!(h, 40.0);
}

#[test]
fn button_snapshot() {
    use shadcn_egui::components::button::{Button, ButtonSize, ButtonVariant};
    use shadcn_egui::icons::lucide::X as CLOSE;
    use shadcn_egui::layout;
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
                ui.add(
                    Button::new("Close")
                        .icon(&CLOSE)
                        .size(ButtonSize::Icon)
                        .variant(ButtonVariant::Outline),
                );
                ui.add(Button::new("With icon").icon(&CLOSE));
            });
            layout::row(ui, 8.0, |ui| {
                ui.add(Button::new("Disabled").enabled(false));
                ui.add(
                    Button::new("Invalid")
                        .variant(ButtonVariant::Outline)
                        .invalid(true),
                );
                ui.add(
                    Button::new("Open")
                        .variant(ButtonVariant::Outline)
                        .expanded(true),
                );
            });
        });
    });
}

/// App variants and an app size of Button and an app variant of Badge, in
/// vega light and dark: the app colour follows the mode. "Remove" is
/// hovered: Outline sets its text colour only on hover.
#[test]
fn app_variant_snapshot() {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::badge::Badge;
    use shadcn_egui::components::button::{ButtonSize, ButtonVariant};
    use shadcn_egui::layout;
    use shadcn_egui::style::{AppVariant, Paint, Role};
    const BRAND: Role = Role::custom("brand");
    const DANGER_GHOST: AppVariant<ButtonVariant> = ButtonVariant::app("danger-ghost");
    const DANGER_OUTLINE: AppVariant<ButtonVariant> = ButtonVariant::app("danger-outline");
    const ROOMY: AppVariant<ButtonSize> = ButtonSize::app("roomy");
    const BRAND_BADGE: AppVariant<BadgeVariant> = BadgeVariant::app("brand");
    let theme = |mode| {
        let t = Theme::new(Style::vega(), Palette::light(), Palette::dark(), 10.0)
            .color(
                BRAND,
                egui::Color32::from_rgb(0x25, 0x63, 0xeb),
                egui::Color32::from_rgb(0x93, 0xc5, 0xfd),
            )
            .variant(DANGER_GHOST, ButtonVariant::Ghost, |r, _style| {
                r.button.text.set(Paint::Role(Role::Destructive, 1.0))
            })
            .variant(DANGER_OUTLINE, ButtonVariant::Outline, |r, _| {
                r.button.recolor(Role::Foreground, Role::Destructive)
            })
            .size(ROOMY, ButtonSize::Default, |r, _| {
                r.button.height.set(44.0);
                r.button.pad_left.set(24.0);
                r.button.pad_right.set(24.0);
            })
            .variant(BRAND_BADGE, BadgeVariant::Default, |r, _| {
                r.badge.recolor(Role::Primary, BRAND)
            });
        (t, mode)
    };
    let themes = vec![
        ("vega_dark", theme(egui::Theme::Dark)),
        ("vega_light", theme(egui::Theme::Light)),
    ]
    .into_iter()
    .map(|(n, (t, m))| (n, t, m))
    .collect();
    common::snapshot_with_themes(
        "app_variant",
        egui::vec2(560.0, 120.0),
        themes,
        |ui| {
            layout::vstack(ui, 12.0, |ui| {
                layout::row(ui, 8.0, |ui| {
                    ui.add(Button::new("Delete").variant(DANGER_GHOST));
                    ui.add(Button::new("Remove").variant(DANGER_OUTLINE));
                    ui.add(Button::new("Roomy").size(ROOMY));
                    ui.add(Button::new("Both").variant(DANGER_OUTLINE).size(ROOMY));
                });
                layout::row(ui, 8.0, |ui| {
                    ui.add(Badge::new("Brand").variant(BRAND_BADGE));
                    badge(ui, "Default", BadgeVariant::Default);
                });
            });
        },
        |h| {
            h.get_by_label("Remove").hover();
            h.run();
        },
    );
}

/// The shared Response-to-State mapping: a disabled widget gets `DISABLED`
/// and loses every pointer and focus bit, also one its caller passed in.
#[test]
fn interaction_state_of_disabled_widget_has_no_pointer_or_focus() {
    use shadcn_egui::components::shared::interaction_state;
    use shadcn_egui::style::State;
    use shadcn_egui::Theme;
    let theme = Theme::vega();
    let target = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(80.0, 30.0));
    let ctx = egui::Context::default();
    let (mut live, mut disabled) = (State::NONE, State::NONE);
    for events in [vec![], common::pointer_down(target.center()), vec![]] {
        let _ = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                let resp = ui.interact(target, egui::Id::new("target"), egui::Sense::click());
                // The press focuses it.
                if ui.input(|i| i.pointer.any_pressed()) {
                    resp.request_focus();
                }
                live = interaction_state(&theme, &resp, State::INVALID);
                disabled = interaction_state(
                    &theme,
                    &resp,
                    State::INVALID | State::FOCUS | State::DISABLED,
                );
            },
        );
    }
    // Control: the same response, enabled, is hovered, pressed and focused,
    // without a ring: a pointer press moved focus.
    let pointer = State::HOVER | State::ACTIVE | State::FOCUS;
    // egui's default window is wider than `md`.
    let theme_flags = State::DARK | State::SM | State::MD;
    assert_eq!(live, pointer | State::INVALID | theme_flags);
    assert_eq!(disabled, State::INVALID | State::DISABLED | theme_flags);
}

/// shadcn `bg-clip-padding` with `border border-transparent`: a solid
/// button's fill stops inside its 1px transparent border.
#[test]
fn button_fill_stops_inside_transparent_border() {
    use shadcn_egui::components::button::Button;
    use shadcn_egui::Theme;
    let mode = egui::Theme::Light;
    let theme = Theme::vega();
    let (rect, out) = common::measure_output(&theme, mode, |ui| ui.add(Button::new("Save")));
    let fill = theme.palette_of(mode).primary;
    let fills: Vec<_> = common::rect_shapes(&out)
        .into_iter()
        .filter(|r| r.fill == fill)
        .collect();
    assert_eq!(fills.len(), 1, "{fills:?}");
    assert_eq!(fills[0].rect, rect.shrink(1.0));
}

/// shadcn `focus-visible:border-ring focus-visible:ring-3 ring-ring/50`: a
/// focused button gets a ring-coloured border and a 3px ring at half alpha.
#[test]
fn focused_button_shows_ring_and_ring_border() {
    use shadcn_egui::components::button::Button;
    use shadcn_egui::Theme;
    let mode = egui::Theme::Light;
    let theme = Theme::vega();
    let (rect, out) = common::measure_output(&theme, mode, |ui| {
        let resp = ui.add(Button::new("Save"));
        resp.request_focus();
        resp
    });
    let ring = theme.palette_of(mode).ring;
    let shapes = common::rect_shapes(&out);
    let ring_shape = shapes.iter().find(|r| r.stroke.width == 3.0);
    let ring_shape = ring_shape.unwrap_or_else(|| panic!("no 3px ring in {shapes:?}"));
    assert_eq!(ring_shape.stroke.color, ring.gamma_multiply(0.5));
    assert_eq!(ring_shape.stroke_kind, egui::StrokeKind::Outside);
    assert_eq!(ring_shape.rect, rect);
    assert!(
        shapes
            .iter()
            .any(|r| r.stroke.width == 1.0 && r.stroke.color == ring),
        "no ring-coloured border in {shapes:?}"
    );
}

#[test]
fn input_accepts_text() {
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use shadcn_egui::components::input::Input;
    use shadcn_egui::Theme;
    let mut text = String::new();
    let mut h = Harness::new_ui_state(
        |ui, text: &mut String| {
            Theme::vega().apply(ui.ctx());
            ui.add(Input::new(text).hint("Email"));
        },
        std::mem::take(&mut text),
    );
    h.run();
    let node = h.get_by_role(egui::accesskit::Role::TextInput);
    node.focus();
    h.run();
    h.get_by_role(egui::accesskit::Role::TextInput)
        .type_text("a@b.c");
    h.run();
    assert_eq!(h.state(), "a@b.c");
}

#[test]
fn disabled_input_ignores_typing() {
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use shadcn_egui::components::input::Input;
    use shadcn_egui::Theme;
    let mut h = Harness::new_ui_state(
        |ui, text: &mut String| {
            Theme::vega().apply(ui.ctx());
            ui.add(Input::new(text).enabled(false));
        },
        String::new(),
    );
    h.run();
    let node = h.get_by_role(egui::accesskit::Role::TextInput);
    node.focus();
    h.run();
    h.get_by_role(egui::accesskit::Role::TextInput)
        .type_text("nope");
    h.run();
    assert_eq!(h.state(), "");
}

/// A field raised to its content minimum overflows a column narrower than
/// that minimum without widening the column (conformance 3.6), and still
/// takes typed text.
#[test]
fn input_overflows_a_narrow_column_without_widening_it() {
    use shadcn_egui::components::input::Input;
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    ctx.set_theme(egui::Theme::Light);
    let mut text = String::new();
    let mut widths = (0.0, 0.0);
    for events in [vec![], vec![], vec![egui::Event::Text("ab".into())]] {
        let _ = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                ui.allocate_ui(egui::vec2(10.0, 100.0), |ui| {
                    ui.set_max_width(10.0);
                    let field = ui.add(Input::new(&mut text).id_salt("narrow"));
                    field.request_focus();
                    widths = (field.rect.width(), ui.min_rect().width());
                });
            },
        );
    }
    assert!(widths.0 > 10.0, "the field keeps its minimum: {widths:?}");
    assert_eq!(widths.1, 10.0, "the column keeps its width");
    assert_eq!(text, "ab");
}

#[test]
fn input_height_follows_style() {
    use shadcn_egui::components::input::Input;
    for (name, theme, mode) in common::themes() {
        let want = if name.starts_with("nova") { 32.0 } else { 36.0 };
        let mut text = String::new();
        let h = common::measure(&theme, mode, |ui| ui.add(Input::new(&mut text))).height();
        assert_eq!(h, want, "{name}");
    }
}

#[test]
fn input_snapshot() {
    use shadcn_egui::components::input::Input;
    use shadcn_egui::components::label::label;
    // `text-base md:text-sm`: the phone and the desktop text size.
    common::snapshot_widths("input", 390.0, common::snapshot_themes(), |ui| {
        let mut empty = String::new();
        let mut filled = String::from("you@example.com");
        let mut invalid = String::from("not an email");
        let mut disabled = String::from("read only");
        let mut focused = String::from("typing here");
        label(ui, "Empty, with hint");
        ui.add(Input::new(&mut empty).hint("Email").id_salt("empty"));
        ui.add_space(8.0);
        label(ui, "Filled");
        ui.add(Input::new(&mut filled).id_salt("filled"));
        ui.add_space(8.0);
        label(ui, "Invalid");
        ui.add(Input::new(&mut invalid).invalid(true).id_salt("invalid"));
        ui.add_space(8.0);
        label(ui, "Disabled");
        ui.add(Input::new(&mut disabled).enabled(false).id_salt("disabled"));
        ui.add_space(8.0);
        label(ui, "Focused");
        let resp = ui.add(Input::new(&mut focused).id_salt("focused"));
        resp.request_focus();
    });
}

/// Runs `Tabs` with state in a cell; the harness draws it under the dark vega theme.
fn tabs_harness(
    labels: &'static [&'static str],
    start: usize,
) -> (
    egui_kittest::Harness<'static>,
    std::rc::Rc<std::cell::Cell<usize>>,
) {
    use shadcn_egui::components::tabs::Tabs;
    let active = std::rc::Rc::new(std::cell::Cell::new(start));
    let seen = active.clone();
    let h = egui_kittest::Harness::new_ui(move |ui| {
        shadcn_egui::Theme::vega().apply(ui.ctx());
        let mut sel = seen.get();
        ui.add(Tabs::new(&mut sel, labels));
        seen.set(sel);
    });
    (h, active)
}

#[test]
fn tabs_click_selects() {
    use egui_kittest::kittest::Queryable;
    let (mut h, active) = tabs_harness(&["Account", "Notifications", "Display"], 0);
    h.run();
    h.get_by_label("Notifications").click();
    h.run();
    assert_eq!(active.get(), 1);
}

#[test]
fn tabs_arrow_keys_move_selection() {
    use egui_kittest::kittest::Queryable;
    let (mut h, active) = tabs_harness(&["Account", "Notifications", "Display"], 0);
    h.run();
    h.get_by_label("Account").focus();
    h.run();
    h.key_press(egui::Key::ArrowRight);
    h.run();
    assert_eq!(active.get(), 1, "ArrowRight selects the next tab");
    h.key_press(egui::Key::End);
    h.run();
    assert_eq!(active.get(), 2, "End selects the last tab");
    h.key_press(egui::Key::ArrowRight);
    h.run();
    assert_eq!(active.get(), 0, "ArrowRight wraps to the first tab");
    h.key_press(egui::Key::ArrowLeft);
    h.run();
    assert_eq!(active.get(), 2, "ArrowLeft wraps to the last tab");
    h.key_press(egui::Key::Home);
    h.run();
    assert_eq!(active.get(), 0, "Home selects the first tab");
}

/// The label of the one focused node, if any.
fn focused_label(h: &egui_kittest::Harness<'_>) -> Option<String> {
    use egui_kittest::kittest::{NodeT, Queryable};
    h.query_all(egui_kittest::kittest::by())
        .find(|n| n.accesskit_node().is_focused())
        .and_then(|n| n.accesskit_node().label())
}

#[test]
fn tabs_are_one_tab_stop() {
    use shadcn_egui::components::tabs::Tabs;
    let mut h = egui_kittest::Harness::new_ui(|ui| {
        shadcn_egui::Theme::vega().apply(ui.ctx());
        ui.add(egui::Button::new("Before"));
        let mut sel = 1;
        ui.add(Tabs::new(
            &mut sel,
            &["Account", "Notifications", "Display"],
        ));
        ui.add(egui::Button::new("After"));
    });
    h.run();
    let mut order = Vec::new();
    for _ in 0..3 {
        h.key_press(egui::Key::Tab);
        h.run();
        order.push(focused_label(&h));
    }
    let name = |s: &str| Some(s.to_owned());
    assert_eq!(
        order,
        [name("Before"), name("Notifications"), name("After")],
        "Tab stops on the selected trigger only"
    );
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    h.run();
    assert_eq!(focused_label(&h), name("Notifications"), "Shift+Tab");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    h.run();
    assert_eq!(
        focused_label(&h),
        name("Before"),
        "Shift+Tab leaves the list"
    );
}

#[test]
fn tabs_keep_focus_when_the_app_moves_the_selection() {
    use egui_kittest::kittest::Queryable;
    let (mut h, active) = tabs_harness(&["Account", "Notifications"], 0);
    h.run();
    h.get_by_label("Account").focus();
    h.run();
    active.set(1);
    h.run();
    h.run();
    assert_eq!(focused_label(&h), Some("Account".to_owned()));
}

#[test]
fn tabs_report_the_selected_trigger() {
    use egui_kittest::kittest::{NodeT, Queryable};
    let (mut h, _) = tabs_harness(&["Account", "Notifications"], 1);
    h.run();
    // egui reports `selected` as the node's toggled state.
    use egui::accesskit::Toggled;
    assert_eq!(
        h.get_by_label("Notifications").accesskit_node().toggled(),
        Some(Toggled::True)
    );
    assert_eq!(
        h.get_by_label("Account").accesskit_node().toggled(),
        Some(Toggled::False)
    );
}

#[test]
fn tabs_are_as_wide_as_their_labels() {
    use shadcn_egui::components::tabs::{Tabs, TabsVariant};
    for (name, theme, mode) in common::themes() {
        let short = common::measure(&theme, mode, |ui| ui.add(Tabs::new(&mut 0, &["A", "B"])));
        let long = common::measure(&theme, mode, |ui| {
            ui.add(Tabs::new(&mut 0, &["Account", "Notifications"]))
        });
        assert!(
            long.width() > short.width() + 40.0,
            "{name}: {} vs {}",
            long.width(),
            short.width()
        );
        // Line triggers are 4px apart (gap-1); the default ones touch.
        let line = common::measure(&theme, mode, |ui| {
            ui.add(Tabs::new(&mut 0, &["A", "B"]).variant(TabsVariant::Line))
        });
        assert_eq!(line.width() - short.width(), 4.0, "{name}");
    }
    let vega = &common::themes()[0].1;
    let nova = &common::themes()[2].1;
    let dark = egui::Theme::Dark;
    assert_eq!(
        common::measure(vega, dark, |ui| ui.add(Tabs::new(&mut 0, &["A"]))).height(),
        36.0
    );
    assert_eq!(
        common::measure(nova, dark, |ui| ui.add(Tabs::new(&mut 0, &["A"]))).height(),
        32.0
    );
}

/// The list rect, the width the `Ui` had available for it, and the rect of
/// each trigger, of a tab list `add` places on a page `page_w` wide.
fn tabs_layout(
    theme: &shadcn_egui::Theme,
    mode: egui::Theme,
    page_w: f32,
    labels: &[&str],
    add: impl Fn(&mut egui::Ui) -> egui::Response + 'static,
) -> (egui::Rect, f32, Vec<egui::Rect>) {
    use egui_kittest::kittest::Queryable;
    use std::cell::Cell;
    use std::rc::Rc;
    let out = Rc::new(Cell::new((egui::Rect::NOTHING, 0.0)));
    let sink = out.clone();
    let h = common::page_harness(egui::vec2(page_w, 120.0), theme.clone(), mode, move |ui| {
        let avail = ui.available_width();
        sink.set((add(ui).rect, avail));
    });
    let (list, avail) = out.get();
    let triggers = labels.iter().map(|l| h.get_by_label(l).rect()).collect();
    (list, avail, triggers)
}

#[test]
fn tabs_width_infinity_fills_the_row_and_shares_it_equally() {
    use shadcn_egui::components::tabs::{Tabs, TabsVariant};
    let labels = ["Display", "Audio", "Camera", "Folders"];
    for (name, theme, mode) in common::themes() {
        for variant in [TabsVariant::Default, TabsVariant::Line] {
            let (list, avail, triggers) = tabs_layout(&theme, mode, 600.0, &labels, move |ui| {
                ui.add(
                    Tabs::new(&mut 0, &labels)
                        .variant(variant)
                        .width(f32::INFINITY),
                )
            });
            assert_eq!(list.width(), avail, "{name} {variant:?}");
            // flex-1: every label fits its share, so all triggers are equal.
            for t in &triggers {
                assert!(
                    (t.width() - triggers[0].width()).abs() < 0.01,
                    "{name} {variant:?}: {triggers:?}"
                );
            }
            // The triggers span the list between its padding.
            let gaps = triggers[1].left() - triggers[0].right();
            let used = triggers.iter().map(|t| t.width()).sum::<f32>() + gaps * 3.0;
            let pad = (triggers[0].left() - list.left()) + (list.right() - triggers[3].right());
            assert!(pad >= 0.0, "{name} {variant:?}");
            assert!(
                (used + pad - list.width()).abs() < 0.01,
                "{name} {variant:?}"
            );
        }
    }
}

#[test]
fn tabs_width_keeps_a_long_label_at_its_own_width() {
    use shadcn_egui::components::tabs::Tabs;
    let labels = ["A", "B", "A rather long tab label"];
    for (name, theme, mode) in common::themes() {
        let (_, _, hug) = tabs_layout(&theme, mode, 600.0, &labels, move |ui| {
            ui.add(Tabs::new(&mut 0, &labels))
        });
        let (list, avail, wide) = tabs_layout(&theme, mode, 360.0, &labels, move |ui| {
            ui.add(Tabs::new(&mut 0, &labels).width(f32::INFINITY))
        });
        assert_eq!(list.width(), avail, "{name}");
        // The long label is wider than a third of the list: flex-1 never
        // shrinks a trigger below its content, the short ones share the rest.
        assert!(hug[2].width() > avail / 3.0, "{name}");
        assert!((wide[2].width() - hug[2].width()).abs() < 0.01, "{name}");
        assert!((wide[0].width() - wide[1].width()).abs() < 0.01, "{name}");
        assert!(wide[0].width() > hug[0].width(), "{name}");
    }
}

#[test]
fn tabs_width_is_at_least_the_content_and_at_most_the_row() {
    use shadcn_egui::components::tabs::Tabs;
    let labels = ["Account", "Notifications"];
    for (name, theme, mode) in common::themes() {
        let (hug, _, _) = tabs_layout(&theme, mode, 600.0, &labels, move |ui| {
            ui.add(Tabs::new(&mut 0, &labels))
        });
        let (fixed, _, _) = tabs_layout(&theme, mode, 600.0, &labels, move |ui| {
            ui.add(Tabs::new(&mut 0, &labels).width(400.0))
        });
        assert_eq!(fixed.width(), 400.0, "{name}");
        for w in [10.0, 0.0, -5.0, f32::NAN] {
            let (small, _, _) = tabs_layout(&theme, mode, 600.0, &labels, move |ui| {
                ui.add(Tabs::new(&mut 0, &labels).width(w))
            });
            assert_eq!(small.width(), hug.width(), "{name} {w}");
        }
        let (capped, avail, _) = tabs_layout(&theme, mode, 300.0, &["A", "B"], move |ui| {
            ui.add(Tabs::new(&mut 0, &["A", "B"]).width(5000.0))
        });
        assert_eq!(capped.width(), avail, "{name}");
    }
}

/// A tab list with an optional `width`, as the one widget of `ui`.
fn add_tabs(ui: &mut egui::Ui, labels: &[&str], w: Option<f32>) -> egui::Response {
    use shadcn_egui::components::tabs::Tabs;
    let mut active = 0;
    let tabs = Tabs::new(&mut active, labels);
    ui.add(match w {
        Some(w) => tabs.width(w),
        None => tabs,
    })
}

#[test]
fn tabs_width_infinity_in_a_scroll_area_fills_the_visible_width() {
    let labels = ["Account", "Notifications"];
    for (name, theme, mode) in common::themes() {
        let (list, avail, _) = tabs_layout(&theme, mode, 600.0, &labels, move |ui| {
            egui::ScrollArea::horizontal()
                .show(ui, |ui| add_tabs(ui, &labels, Some(f32::INFINITY)))
                .inner
        });
        assert_eq!(list.width(), avail, "{name}");
    }
}

#[test]
fn tabs_width_in_an_unbounded_ui_hugs_or_keeps_a_finite_width() {
    let labels = ["Account", "Notifications"];
    let unbounded = |w: Option<f32>| {
        move |ui: &mut egui::Ui| {
            let r = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(f32::INFINITY, 60.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
                assert!(!ui.available_width().is_finite());
                add_tabs(ui, &labels, w)
            })
            .inner
        }
    };
    for (name, theme, mode) in common::themes() {
        let (hug, _, _) = tabs_layout(&theme, mode, 600.0, &labels, unbounded(None));
        let (inf, _, _) = tabs_layout(&theme, mode, 600.0, &labels, unbounded(Some(f32::INFINITY)));
        assert_eq!(inf.width(), hug.width(), "{name}");
        let (fixed, _, _) = tabs_layout(&theme, mode, 600.0, &labels, unbounded(Some(400.0)));
        assert_eq!(fixed.width(), 400.0, "{name}");
    }
}

#[test]
fn tabs_full_width_snapshot() {
    use shadcn_egui::components::tabs::{Tabs, TabsVariant};
    let labels = ["Display", "Audio", "Camera", "Folders"];
    common::snapshot_all("tabs_full_width", egui::vec2(420.0, 150.0), |ui| {
        // Keyboard focus ring on a widened, inactive trigger.
        let resp = ui.add(Tabs::new(&mut 1, &labels).width(f32::INFINITY));
        ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", 2))));
        ui.add_space(24.0);
        ui.add(
            Tabs::new(&mut 1, &labels)
                .variant(TabsVariant::Line)
                .width(f32::INFINITY),
        );
    });
}

#[test]
fn tabs_snapshot() {
    use shadcn_egui::components::tabs::{Tabs, TabsVariant};
    common::snapshot_all("tabs", egui::vec2(380.0, 290.0), |ui| {
        ui.add(Tabs::new(&mut 0, &["Account", "Notifications"]));
        ui.add_space(24.0);
        ui.add(
            Tabs::new(&mut 1, &["Account", "Notifications", "Display"]).variant(TabsVariant::Line),
        );
        ui.add_space(24.0);
        ui.add(Tabs::new(&mut 0, &["Disabled", "Tabs"]).enabled(false));
        ui.add_space(24.0);
        // Keyboard focus ring on the second trigger of a default list.
        let mut sel = 1;
        let resp = ui.add(Tabs::new(&mut sel, &["Account", "Notifications"]));
        ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", 1))));
    });
}

/// Every trigger state of one tabs variant: in an enabled list the active,
/// an inactive and a hovered trigger; a disabled list (its active and its
/// inactive trigger); and a focused active trigger.
fn tabs_matrix(variant: shadcn_egui::components::tabs::TabsVariant) -> impl FnMut(&mut egui::Ui) {
    use shadcn_egui::components::tabs::Tabs;
    move |ui| {
        ui.add(Tabs::new(&mut 0, &["Active", "Inactive", "Hovered"]).variant(variant));
        ui.add_space(24.0);
        ui.add(
            Tabs::new(&mut 0, &["Disabled", "Off"])
                .variant(variant)
                .enabled(false),
        );
        ui.add_space(24.0);
        let mut sel = 0;
        let resp = ui.add(Tabs::new(&mut sel, &["Focused", "Next"]).variant(variant));
        ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", 0))));
    }
}

#[test]
fn tabs_matrix_default_snapshot() {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::tabs::TabsVariant;
    common::snapshot_all_with(
        "tabs_matrix_default",
        egui::vec2(360.0, 220.0),
        tabs_matrix(TabsVariant::Default),
        |h| {
            h.get_by_label("Hovered").hover();
            h.run();
        },
    );
}

#[test]
fn tabs_matrix_line_snapshot() {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::tabs::TabsVariant;
    common::snapshot_all_with(
        "tabs_matrix_line",
        egui::vec2(360.0, 220.0),
        tabs_matrix(TabsVariant::Line),
        |h| {
            h.get_by_label("Hovered").hover();
            h.run();
        },
    );
}

/// A focused inactive trigger, next to the active one. egui focuses one
/// widget per frame, so this cannot share a frame with the matrix's
/// focused active trigger.
fn tabs_focus_inactive(
    variant: shadcn_egui::components::tabs::TabsVariant,
) -> impl FnMut(&mut egui::Ui) {
    use shadcn_egui::components::tabs::Tabs;
    move |ui| {
        let resp = ui.add(Tabs::new(&mut 0, &["Selected", "Focused"]).variant(variant));
        ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", 1))));
    }
}

#[test]
fn tabs_focus_inactive_default_snapshot() {
    use shadcn_egui::components::tabs::TabsVariant;
    common::snapshot_all(
        "tabs_focus_inactive_default",
        egui::vec2(360.0, 90.0),
        tabs_focus_inactive(TabsVariant::Default),
    );
}

#[test]
fn tabs_focus_inactive_line_snapshot() {
    use shadcn_egui::components::tabs::TabsVariant;
    common::snapshot_all(
        "tabs_focus_inactive_line",
        egui::vec2(360.0, 90.0),
        tabs_focus_inactive(TabsVariant::Line),
    );
}

/// One tab-trigger scenario of the review image: the labels, the selected
/// trigger, the focused trigger, the hovered label, and whether the list is
/// enabled.
struct TabsRow {
    labels: &'static [&'static str],
    selected: usize,
    focus: Option<usize>,
    hover: Option<&'static str>,
    enabled: bool,
}

/// Every trigger state of one tabs variant in one image per theme. A tab
/// list takes keyboard focus and the pointer one at a time, so each row
/// renders in its own harness (all the same page, the row at its own
/// height) and the row bands are stacked into one picture.
fn tabs_states(variant: shadcn_egui::components::tabs::TabsVariant, name: &str) {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::tabs::Tabs;

    const ROW_H: usize = 56;
    // Rows start below the page margin; bands are cut a little above the row.
    const BAND_TOP: usize = 10;
    let row = |labels, selected, focus, hover, enabled| TabsRow {
        labels,
        selected,
        focus,
        hover,
        enabled,
    };
    let rows = [
        row(
            &["Inactive", "Hover", "Active"],
            2,
            None,
            Some("Hover"),
            true,
        ),
        row(
            &["Inactive", "Active + hover"],
            1,
            None,
            Some("Active + hover"),
            true,
        ),
        row(&["Inactive", "Focus", "Active"], 2, Some(1), None, true),
        row(
            &["Inactive", "Focus + hover", "Active"],
            2,
            Some(1),
            Some("Focus + hover"),
            true,
        ),
        row(&["Inactive", "Active + focus"], 1, Some(1), None, true),
        row(
            &["Inactive", "Active + focus + hover"],
            1,
            Some(1),
            Some("Active + focus + hover"),
            true,
        ),
        row(&["Disabled", "Disabled active"], 1, None, None, false),
    ];
    let page_h = BAND_TOP + rows.len() * ROW_H + 14;
    let size = egui::vec2(520.0, page_h as f32);

    let mut results = egui_kittest::SnapshotResults::new();
    for (suffix, theme, mode) in common::snapshot_themes() {
        let mut stacked = None;
        for (i, r) in rows.iter().enumerate() {
            let mut h = common::page_harness(size, theme.clone(), mode, |ui| {
                ui.add_space((i * ROW_H) as f32);
                let mut sel = r.selected;
                let resp = ui.add(
                    Tabs::new(&mut sel, r.labels)
                        .variant(variant)
                        .enabled(r.enabled),
                );
                if let Some(f) = r.focus {
                    ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", f))));
                }
            });
            if let Some(label) = r.hover {
                h.get_by_label(label).hover();
                h.run();
            }
            let img = h.render().expect("render the tabs row");
            assert_eq!(img.height() as usize, page_h, "one pixel per point");
            match stacked.as_mut() {
                None => stacked = Some(img),
                Some(out) => {
                    let out: &mut [u8] = &mut *out;
                    let stride = img.width() as usize * 4;
                    let (from, to) = (
                        (BAND_TOP + i * ROW_H) * stride,
                        (BAND_TOP + (i + 1) * ROW_H) * stride,
                    );
                    out[from..to].copy_from_slice(&img.as_raw()[from..to]);
                }
            }
        }
        let img = stacked.expect("at least one row");
        results.add(common::try_image_snapshot(&img, format!("{name}_{suffix}")));
    }
}

#[test]
fn tabs_states_default_snapshot() {
    tabs_states(
        shadcn_egui::components::tabs::TabsVariant::Default,
        "tabs_states_default",
    );
}

#[test]
fn tabs_states_line_snapshot() {
    tabs_states(
        shadcn_egui::components::tabs::TabsVariant::Line,
        "tabs_states_line",
    );
}

#[test]
fn badge_is_h5_in_both_styles() {
    use shadcn_egui::components::badge::{badge, BadgeVariant};
    for (name, theme, mode) in common::themes() {
        let h =
            common::measure(&theme, mode, |ui| badge(ui, "New", BadgeVariant::Default)).height();
        assert_eq!(h, 20.0, "{name}");
    }
}

#[test]
fn badge_snapshot() {
    use shadcn_egui::components::badge::{badge, BadgeVariant::*};
    common::snapshot_all("badge", egui::vec2(480.0, 64.0), |ui| {
        shadcn_egui::layout::row(ui, 8.0, |ui| {
            for (t, v) in [
                ("Default", Default),
                ("Secondary", Secondary),
                ("Destructive", Destructive),
                ("Outline", Outline),
                ("Ghost", Ghost),
                ("Link", Link),
            ] {
                badge(ui, t, v);
            }
        });
    });
}

/// shadcn `peer-disabled:opacity-50`: a label of a disabled control paints
/// its text at half alpha, and `text-sm leading-none` makes it 14px high.
#[test]
fn disabled_label_paints_half_alpha_text() {
    use shadcn_egui::components::label::Label;
    use shadcn_egui::style::{pick, State};
    for (name, theme, mode) in common::themes() {
        let (rect, on) = common::measure_output(&theme, mode, |ui| ui.add(Label::new("Email")));
        let (_, off) = common::measure_output(&theme, mode, |ui| {
            ui.add(Label::new("Email").enabled(false))
        });
        let fg = theme.palette_of(mode).foreground;
        assert_eq!(common::text_colour(&on), fg, "{name}");
        assert_eq!(common::text_colour(&off), fg.gamma_multiply(0.5), "{name}");
        assert_eq!(rect.height(), 14.0, "{name}");
        assert_eq!(
            pick(&[&*theme.style.label], |r| &r.opacity, State::DISABLED),
            Some(0.5),
            "{name}"
        );
    }
}

/// The separator is a 1px `bg-border` line: full width when horizontal,
/// full row height when vertical.
#[test]
fn separators_are_one_pixel_border_lines() {
    use shadcn_egui::components::separator::{separator, separator_vertical};
    let mode = egui::Theme::Light;
    let theme = shadcn_egui::Theme::vega();
    let (_, out) = common::measure_output(&theme, mode, |ui| {
        ui.allocate_response(egui::vec2(200.0, 0.0), egui::Sense::hover());
        separator(ui);
        ui.allocate_response(egui::vec2(0.0, 0.0), egui::Sense::hover())
    });
    let lines: Vec<_> = common::rect_shapes(&out)
        .into_iter()
        .filter(|r| r.fill == theme.palette_of(mode).border)
        .collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(lines[0].rect.height(), 1.0);
    assert!(lines[0].rect.width() >= 200.0);
    let (_, out) = common::measure_output(&theme, mode, |ui| {
        ui.horizontal(|ui| {
            ui.set_height(24.0);
            separator_vertical(ui);
        })
        .response
    });
    let v: Vec<_> = common::rect_shapes(&out)
        .into_iter()
        .filter(|r| r.fill == theme.palette_of(mode).border)
        .collect();
    assert_eq!(v.len(), 1, "{v:?}");
    assert_eq!(v[0].rect.width(), 1.0);
    assert_eq!(v[0].rect.height(), 24.0);
}

#[test]
fn label_separator_snapshot() {
    use shadcn_egui::components::label::{description, Label};
    use shadcn_egui::components::separator::{separator, separator_vertical};
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

/// Vertical separators in a `layout::row` of items of different heights
/// (a label, a button, a badge, a two-line stack): each spans the tallest
/// other item (`self-stretch`).
#[test]
fn separator_vertical_row_snapshot() {
    use shadcn_egui::components::badge::{badge, BadgeVariant};
    use shadcn_egui::components::button::Button;
    use shadcn_egui::components::label::{description, Label};
    use shadcn_egui::components::separator::separator_vertical;
    use shadcn_egui::layout;
    let themes = common::themes().into_iter().take(2).collect();
    common::snapshot_with_themes(
        "separator_vertical_row",
        egui::vec2(420.0, 100.0),
        themes,
        |ui| {
            layout::row(ui, 12.0, |ui| {
                ui.add(Label::new("Docs"));
                separator_vertical(ui);
                ui.add(Button::new("Open"));
                separator_vertical(ui);
                badge(ui, "New", BadgeVariant::Secondary);
                separator_vertical(ui);
                layout::vstack(ui, 2.0, |ui| {
                    ui.add(Label::new("Account"));
                    description(ui, "Signed in");
                });
            });
        },
        |_| {},
    );
}

/// The `select` and `badge` shorthands; their sizes are asserted with the
/// widgets they wrap.
#[test]
fn select_and_badge_shorthands_do_not_panic() {
    use egui_kittest::Harness;
    use shadcn_egui::components::badge::{badge, BadgeVariant};
    use shadcn_egui::components::select::select;
    use shadcn_egui::Theme;
    let mut h = Harness::new_ui(|ui| {
        Theme::vega().apply(ui.ctx());
        let mut choice = 0usize;
        select(ui, "lang", &mut choice, &["English", "German"]);
        badge(ui, "New", BadgeVariant::Default);
    });
    h.run();
}

/// A card with a header, content and a footer button, 320 px wide.
fn demo_card(ui: &mut egui::Ui, size: CardSize, footer: bool) {
    ui.set_max_width(320.0);
    let mut card = Card::new().size(size);
    if footer {
        card = card.footer(|ui| {
            ui.add(Button::new("Save"));
        });
    }
    card.show(ui, |ui| {
        card_header(
            ui,
            "Notifications",
            "Choose what you want to be notified about.",
        );
        card_content(ui, |ui| {
            ui.label("You have 3 unread messages.");
            ui.horizontal(|ui| {
                badge(ui, "Default", BadgeVariant::Default);
                badge(ui, "Outline", BadgeVariant::Outline);
            });
        });
    });
}

#[test]
fn card_default_snapshot() {
    common::snapshot_all("card_default", egui::vec2(360.0, 200.0), |ui| {
        demo_card(ui, CardSize::Default, false)
    });
}

#[test]
fn card_footer_snapshot() {
    common::snapshot_all("card_footer", egui::vec2(360.0, 300.0), |ui| {
        demo_card(ui, CardSize::Default, true)
    });
}

#[test]
fn card_sm_snapshot() {
    common::snapshot_all("card_sm", egui::vec2(360.0, 300.0), |ui| {
        demo_card(ui, CardSize::Sm, true)
    });
}

#[test]
fn card_action_snapshot() {
    common::snapshot_all("card_action", egui::vec2(360.0, 140.0), |ui| {
        ui.set_max_width(320.0);
        Card::new().show(ui, |ui| {
            card_header_with_action(
                ui,
                "Notifications",
                "Choose what you want to be notified about.",
                |ui| {
                    ui.add(Button::new("Mark all").variant(ButtonVariant::Outline));
                },
            );
        });
    });
}

/// The footer strip of a style that paints one reaches the card's bottom
/// edge: the card adds no padding below it.
#[test]
fn nova_footer_strip_reaches_card_bottom() {
    let mode = egui::Theme::Dark;
    let theme = Theme::new(Style::nova(), Palette::light(), Palette::dark(), 10.0);
    let (card, out) = common::measure_output(&theme, mode, |ui| {
        ui.scope(|ui| demo_card(ui, CardSize::Default, true))
            .response
    });
    let strip = theme.paint(shadcn_egui::style::Paint::Role(
        shadcn_egui::style::Role::Muted,
        0.5,
    ));
    let strips: Vec<_> = common::rect_shapes(&out)
        .into_iter()
        .filter(|r| r.fill == strip)
        .collect();
    assert_eq!(strips.len(), 1, "one footer strip");
    assert_eq!(strips[0].rect.bottom(), card.bottom());
    assert_eq!(
        (strips[0].rect.left(), strips[0].rect.right()),
        (card.left(), card.right())
    );
}

/// vega has no footer strip: the card's own bottom padding stays below the footer.
#[test]
fn vega_footer_keeps_card_padding_below() {
    let mode = egui::Theme::Dark;
    let theme = Theme::new(Style::vega(), Palette::light(), Palette::dark(), 10.0);
    let mut button = egui::Rect::NOTHING;
    let card = common::measure(&theme, mode, |ui| {
        ui.scope(|ui| {
            ui.set_max_width(320.0);
            Card::new()
                .footer(|ui| button = ui.add(Button::new("Save")).rect)
                .show(ui, |ui| card_header(ui, "T", "D"));
        })
        .response
    });
    assert_eq!(card.bottom() - button.bottom(), 24.0);
}

/// `data-[size=sm]` shrinks the side padding of the content: 12 in nova,
/// 16 in vega; the default card has 16 and 24.
#[test]
fn card_side_padding_follows_size() {
    for (style, default, sm) in [
        (Style::nova as fn() -> Style, 16.0, 12.0),
        (Style::vega, 24.0, 16.0),
    ] {
        let mode = egui::Theme::Dark;
        let theme = Theme::new(style(), Palette::light(), Palette::dark(), 10.0);
        for (size, want) in [(CardSize::Default, default), (CardSize::Sm, sm)] {
            let mut label = egui::Rect::NOTHING;
            let card = common::measure(&theme, mode, |ui| {
                ui.scope(|ui| {
                    ui.set_max_width(320.0);
                    Card::new()
                        .size(size)
                        .show(ui, |ui| label = ui.label("x").rect);
                })
                .response
            });
            assert_eq!(label.left() - card.left(), want, "{size:?}");
        }
    }
}

/// A card fills the available width, like a block element.
#[test]
fn card_fills_available_width() {
    let theme = Theme::vega();
    let card = common::measure(&theme, egui::Theme::Dark, |ui| {
        ui.scope(|ui| {
            ui.set_max_width(320.0);
            Card::new().show(ui, |ui| {
                ui.label("x");
            });
        })
        .response
    });
    assert_eq!(card.width(), 320.0);
}

/// The title row has the style's line height (`leading-6` in vega,
/// `leading-snug` in nova) and shrinks with `data-[size=sm]`.
#[test]
fn card_title_row_height_follows_style_and_size() {
    for (style, default, sm) in [
        (Style::vega as fn() -> Style, 24.0, 21.0),
        (Style::nova, 22.0, 19.25),
    ] {
        let mode = egui::Theme::Dark;
        let theme = Theme::new(style(), Palette::light(), Palette::dark(), 10.0);
        for (size, want) in [(CardSize::Default, default), (CardSize::Sm, sm)] {
            let mut title = egui::Rect::NOTHING;
            common::measure(&theme, mode, |ui| {
                ui.scope(|ui| {
                    Card::new()
                        .size(size)
                        .show(ui, |ui| title = card_title(ui, "Title").rect);
                })
                .response
            });
            assert!(
                (title.height() - want).abs() <= 0.5,
                "{size:?}: {}",
                title.height()
            );
        }
    }
}

/// The action sits at the right edge of the header, the text to its left.
#[test]
fn card_action_is_right_aligned() {
    let theme = Theme::vega();
    let mut action = egui::Rect::NOTHING;
    let mut title = egui::Rect::NOTHING;
    let card = common::measure(&theme, egui::Theme::Dark, |ui| {
        ui.scope(|ui| {
            ui.set_max_width(320.0);
            Card::new().show(ui, |ui| {
                card_header_with_action(ui, "Title", "Description", |ui| {
                    action = ui.add(Button::new("Go")).rect
                });
                title = ui.min_rect();
            });
        })
        .response
    });
    assert_eq!(card.right() - action.right(), 24.0);
    assert!(action.top() <= title.top() + 0.5 + 40.0 && action.left() > card.left() + 24.0);
}

/// Header and content inset their children by the same amount in every
/// style and size, which is why the card insets its body once.
#[test]
fn card_header_and_content_share_side_padding() {
    use shadcn_egui::style::State;
    for style in [Style::vega as fn() -> Style, Style::nova] {
        let style = style();
        for st in [State::SIZE_DEFAULT, State::SIZE_SM] {
            let h = &style.card_header;
            for other in [&style.card_content, &style.card_footer] {
                assert_eq!(h.pad_left.get(st), other.pad_left.get(st));
                assert_eq!(h.pad_right.get(st), other.pad_right.get(st));
            }
        }
    }
}

/// `card_content` spaces its children by the style's `cn-card-content`
/// gap, which only vega sets (12, see its docs); the others leave
/// children touching.
#[test]
fn only_vega_sets_a_card_content_gap() {
    use shadcn_egui::style::State;
    use shadcn_egui::{Preset, StyleName};
    // Sera is not exported: a preset naming it falls back to nova.
    for name in StyleName::ALL.iter().filter(|s| **s != StyleName::Sera) {
        let theme = Preset::default().style(*name).theme();
        let (name, style) = (name.id(), &theme.style);
        assert_eq!(style.name, name, "the preset builds its own style");
        for st in [State::SIZE_DEFAULT, State::SIZE_SM] {
            let gap = style.card_content.gap.get(st);
            let want = (name == "vega").then_some(12.0);
            assert_eq!(gap, want, "{name} card_content gap at {st:?}");
        }
    }
}

/// shadcn `disabled:pointer-events-none`: no click, and a pressed pointer
/// over the button changes nothing it paints (no hover fill, no press nudge).
#[test]
fn disabled_button_ignores_clicks() {
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use shadcn_egui::components::button::Button;
    use shadcn_egui::Theme;
    use std::cell::Cell;
    let clicked = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        Theme::vega().apply(ui.ctx());
        if ui.add(Button::new("Save").enabled(false)).clicked() {
            clicked.set(true);
        }
    });
    h.get_by_label("Save").click();
    h.run();
    assert!(!clicked.get(), "disabled button registered a click");

    /// The button's rect shapes at rest and with the pointer held down on it.
    fn at_rest_and_pressed(
        enabled: bool,
    ) -> (Vec<egui::epaint::RectShape>, Vec<egui::epaint::RectShape>) {
        fn frame(
            ctx: &egui::Context,
            enabled: bool,
            events: Vec<egui::Event>,
        ) -> (egui::Rect, Vec<egui::epaint::RectShape>) {
            let mut rect = egui::Rect::NOTHING;
            let out = ctx.run_ui(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ui| {
                    Theme::vega().apply(ui.ctx());
                    rect = ui.add(Button::new("Save").enabled(enabled)).rect;
                },
            );
            (rect, common::rect_shapes(&out))
        }
        let ctx = egui::Context::default();
        frame(&ctx, enabled, vec![]);
        let (rect, rest) = frame(&ctx, enabled, vec![]);
        frame(&ctx, enabled, common::pointer_down(rect.center()));
        (rest, frame(&ctx, enabled, vec![]).1)
    }
    // Control: an enabled button does change under a pressed pointer.
    let (rest, pressed) = at_rest_and_pressed(true);
    assert_ne!(rest, pressed, "enabled button shows no hover or press");
    let (rest, pressed) = at_rest_and_pressed(false);
    assert_eq!(
        rest, pressed,
        "disabled button shows hover or press styling"
    );
}

/// shadcn `border-input focus-visible:border-ring focus-visible:ring-3
/// ring-ring/50`: the border is input-coloured at rest; focus turns it
/// ring-coloured and adds a 3px ring at half alpha.
#[test]
fn focused_input_border_turns_ring_colored() {
    use shadcn_egui::components::input::Input;

    for (name, theme, mode) in common::themes() {
        let p = theme.palette_of(mode);
        let shapes = |focus: bool| {
            let mut text = String::new();
            let (rect, out) = common::measure_output(&theme, mode, |ui| {
                let resp = ui.add(Input::new(&mut text).id_salt("field"));
                if focus {
                    resp.request_focus();
                }
                resp
            });
            (rect, common::rect_shapes(&out))
        };
        let (rect, idle) = shapes(false);
        let border = |s: &[egui::epaint::RectShape]| {
            s.iter()
                .rev()
                .find(|r| r.stroke.width == 1.0)
                .map(|r| r.stroke.color)
        };
        assert_eq!(border(&idle), Some(p.input), "{name} idle: {idle:?}");
        assert!(
            idle.iter().all(|r| r.stroke.width != 3.0),
            "{name}: ring while idle"
        );
        let (_, focused) = shapes(true);
        assert_eq!(
            border(&focused),
            Some(p.ring),
            "{name} focused: {focused:?}"
        );
        let ring = focused
            .iter()
            .find(|r| r.stroke.width == 3.0)
            .unwrap_or_else(|| panic!("{name}: no ring in {focused:?}"));
        assert_eq!(ring.stroke.color, p.ring.gamma_multiply(0.5), "{name}");
        assert_eq!(ring.rect, rect, "{name}");
    }
}

/// `aria-invalid:border-destructive aria-invalid:ring-3`: dark themes use
/// the `/50` border and `/40` ring.
#[test]
fn invalid_input_border_is_destructive() {
    use shadcn_egui::components::input::Input;
    for (name, theme, mode) in common::themes() {
        let mut text = String::new();
        let (_, out) = common::measure_output(&theme, mode, |ui| {
            ui.add(Input::new(&mut text).invalid(true))
        });
        let shapes = common::rect_shapes(&out);
        let d = theme.palette_of(mode).destructive;
        let (border, ring) = if name.ends_with("dark") {
            (d.gamma_multiply(0.5), d.gamma_multiply(0.4))
        } else {
            (d, d.gamma_multiply(0.2))
        };
        assert!(
            shapes
                .iter()
                .any(|r| r.stroke.width == 1.0 && r.stroke.color == border),
            "{name}: border in {shapes:?}"
        );
        assert!(
            shapes
                .iter()
                .any(|r| r.stroke.width == 3.0 && r.stroke.color == ring),
            "{name}: ring in {shapes:?}"
        );
    }
}

/// A word wider than the field stays on one line, as in Chrome's
/// `<input>`, which scrolls its text sideways and never wraps: the field
/// keeps its allocated width, and its text is clipped to the field.
#[test]
fn a_long_word_stays_on_one_line_in_the_input() {
    use shadcn_egui::components::input::Input;
    let theme = Theme::vega();
    let mut text = format!("see https://{}", "x".repeat(80));
    let mut start = egui::Pos2::ZERO;
    let (rect, out) = common::measure_output(&theme, egui::Theme::Light, |ui| {
        start = ui.cursor().min;
        ui.add(Input::new(&mut text).max_width(200.0))
    });
    assert_eq!(rect.left(), start.x, "{rect:?}");
    assert_eq!(rect.width(), 200.0, "{rect:?}");
    let shapes = common::text_shapes(&out);
    let galley = &shapes.first().expect("the field's text").galley;
    assert_eq!(galley.rows.len(), 1);
    assert!(galley.size().x > 200.0, "{:?}", galley.size());
    for r in common::rect_shapes(&out) {
        assert!(
            r.rect.left() >= rect.left() && r.rect.right() <= rect.right(),
            "box {:?} outside {rect:?}",
            r.rect
        );
    }
}

/// A glyph wider than the content minimum's `M` keeps the input's
/// response at its allocated box (typed text and hint text).
#[test]
fn a_glyph_wider_than_the_content_minimum_keeps_the_input_response_in_the_box() {
    use shadcn_egui::components::input::Input;
    let theme = Theme::vega();
    for (typed, hint) in [("W", ""), ("WWW", ""), ("", "W")] {
        let mut text = String::from(typed);
        let mut start = egui::Pos2::ZERO;
        let mut narrow = 0.0;
        let (rect, out) = common::measure_output(&theme, egui::Theme::Light, |ui| {
            start = ui.cursor().min;
            narrow = common::measure(&theme, egui::Theme::Light, |ui| {
                let mut m = String::from("M");
                ui.add(Input::new(&mut m).max_width(0.0))
            })
            .width();
            ui.add(Input::new(&mut text).hint(hint).max_width(0.0))
        });
        assert_eq!(rect.left(), start.x, "{typed:?}/{hint:?}: {rect:?}");
        assert_eq!(rect.width(), narrow, "{typed:?}/{hint:?}: {rect:?}");
        for r in common::rect_shapes(&out) {
            assert!(
                r.rect.left() >= rect.left() && r.rect.right() <= rect.right(),
                "{typed:?}/{hint:?}: box {:?} outside {rect:?}",
                r.rect
            );
        }
    }
}

/// A recipe without a placeholder colour shows the hint in upstream's
/// `placeholder:text-muted-foreground`, never in an egui colour
/// (conformance 2.1).
#[test]
fn input_hint_without_a_recipe_colour_is_muted() {
    use shadcn_egui::components::input::{Input, InputVariant};
    use shadcn_egui::style::{AppVariant, Prop, Role};
    const PLAIN: AppVariant<InputVariant> = InputVariant::app("plain");
    let theme = Theme::vega().variant(PLAIN, InputVariant::Default, |r, _| {
        r.input.text = Prop::default();
    });
    let mut text = String::new();
    let (_, out) = common::measure_output(&theme, egui::Theme::Light, |ui| {
        ui.add(Input::new(&mut text).hint("Type here").variant(PLAIN))
    });
    let want = common::resolved(&theme, egui::Theme::Light).role(Role::MutedForeground);
    assert_eq!(common::text_colour(&out), want);
}

/// `Input::id_salt` gives the field an id that does not depend on how many
/// widgets precede it, so callers can address it from outside.
#[test]
fn input_id_salt_is_stable() {
    use shadcn_egui::components::input::Input;
    fn id_after(ctx: &egui::Context, preceding: usize, salt: Option<&'static str>) -> egui::Id {
        let mut text = String::new();
        let mut id = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            for i in 0..preceding {
                ui.label(format!("filler {i}"));
            }
            let input = Input::new(&mut text);
            let input = match salt {
                Some(s) => input.id_salt(s),
                None => input,
            };
            id = Some(ui.add(input).id);
        });
        id.expect("input was added")
    }
    let ctx = egui::Context::default();
    // Control: without a salt the id follows widget position.
    assert_ne!(id_after(&ctx, 1, None), id_after(&ctx, 3, None));
    assert_eq!(
        id_after(&ctx, 1, Some("login-user")),
        id_after(&ctx, 3, Some("login-user"))
    );
}

/// Salts built at run time (one field per row): each row's field gets its
/// own id, the same in every pass.
#[test]
fn input_id_salt_takes_runtime_salts() {
    use shadcn_egui::components::input::Input;
    let ctx = egui::Context::default();
    let ids = |filler: usize| {
        let mut texts = vec![String::new(); 2];
        let mut ids = Vec::new();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            for i in 0..filler {
                ui.label(format!("filler {i}"));
            }
            for (i, text) in texts.iter_mut().enumerate() {
                ids.push(ui.add(Input::new(text).id_salt(format!("row{i}"))).id);
            }
        });
        ids
    };
    let first = ids(0);
    assert_ne!(first[0], first[1], "rows share an id");
    assert_eq!(ids(2), first, "the ids follow the salt, not the position");
}

/// A string salt gives the field the id it had in 0.4, where `Input` passed
/// the salt to egui's own `TextEdit::id_salt` inside a scope and
/// `add_sized`.
#[test]
fn input_id_salt_keeps_the_0_4_id() {
    use shadcn_egui::components::input::Input;
    let ctx = egui::Context::default();
    let id_of = |add: &mut dyn FnMut(&mut egui::Ui) -> egui::Id| {
        let mut id = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| id = Some(add(ui)));
        id.expect("field was added")
    };
    let mut text = String::new();
    let reference = id_of(&mut |ui| {
        ui.scope(|ui| {
            ui.add_sized(
                egui::vec2(200.0, 32.0),
                egui::TextEdit::singleline(&mut text).id_salt("login-user"),
            )
        })
        .inner
        .id
    });
    let salted = id_of(&mut |ui| ui.add(Input::new(&mut text).id_salt("login-user")).id);
    assert_eq!(salted, reference, "id_salt");
}

/// A badge takes any icon, also one borrowed from the theme for a moment.
#[test]
fn badge_takes_a_borrowed_icon() {
    use shadcn_egui::components::badge::Badge;
    let ctx = egui::Context::default();
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
        let icons = Theme::current(ui.ctx()).icons;
        ui.add(Badge::new("Done").icon(&icons.check).icon_end(&icons.check));
    });
}

/// Harness state for the checkbox tests: the value and the rect of the
/// visible box (the returned response's rect).
struct CheckboxState {
    checked: bool,
    rect: egui::Rect,
}

fn checkbox_harness(enabled: bool, checked: bool) -> egui_kittest::Harness<'static, CheckboxState> {
    use shadcn_egui::components::checkbox::Checkbox;
    use shadcn_egui::Theme;
    let mut h = egui_kittest::Harness::new_ui_state(
        move |ui, s: &mut CheckboxState| {
            Theme::vega().apply(ui.ctx());
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                s.rect = ui.add(Checkbox::new(&mut s.checked).enabled(enabled)).rect;
            });
        },
        CheckboxState {
            checked,
            rect: egui::Rect::NOTHING,
        },
    );
    h.run();
    h
}

/// Press and release the primary button at `pos`.
fn click_pos(h: &mut egui_kittest::Harness<'_, CheckboxState>, pos: egui::Pos2) {
    h.hover_at(pos);
    h.run();
    h.drag_at(pos);
    h.run();
    h.drop_at(pos);
    h.run();
}

/// `after:-inset-x-3 after:-inset-y-2`: a click 6px left of the 16px box
/// still lands on the checkbox, one 20px away does not. The response's rect is
/// the visible box; the accessibility node carries the larger target.
#[test]
fn checkbox_hit_area_extends_beyond_the_box() {
    use egui_kittest::kittest::Queryable;
    let mut h = checkbox_harness(true, false);
    let boxr = h.state().rect;
    assert_eq!(boxr.size(), egui::vec2(16.0, 16.0));
    let node = h.get_by_role(egui::accesskit::Role::CheckBox).rect();
    assert_eq!(
        node.expand2(egui::vec2(-12.0, -8.0)),
        boxr,
        "node rect is the box plus 12px x 8px"
    );
    click_pos(&mut h, egui::pos2(boxr.left() - 6.0, boxr.center().y));
    assert!(
        h.state().checked,
        "click outside the box but inside the hit area did not toggle"
    );
    click_pos(&mut h, egui::pos2(boxr.center().x, boxr.top() - 6.0));
    assert!(
        !h.state().checked,
        "click above the box did not toggle back"
    );
    click_pos(&mut h, egui::pos2(boxr.left() - 20.0, boxr.center().y));
    assert!(!h.state().checked, "click beyond the hit area toggled");
}

#[test]
fn disabled_checkbox_ignores_click_and_space() {
    use egui_kittest::kittest::Queryable;
    let mut h = checkbox_harness(false, true);
    let boxr = h.state().rect;
    click_pos(&mut h, boxr.center());
    assert!(h.state().checked, "disabled checkbox toggled by click");
    h.get_by_role(egui::accesskit::Role::CheckBox).focus();
    h.run();
    h.key_press(egui::Key::Space);
    h.run();
    assert!(h.state().checked, "disabled checkbox toggled by Space");
}

#[test]
fn checkbox_toggles_on_space_when_focused() {
    use egui_kittest::kittest::Queryable;
    let mut h = checkbox_harness(true, false);
    h.get_by_role(egui::accesskit::Role::CheckBox).focus();
    h.run();
    h.key_press(egui::Key::Space);
    h.run();
    assert!(h.state().checked, "Space did not check");
    h.key_press(egui::Key::Space);
    h.run();
    assert!(!h.state().checked, "Space did not uncheck");
}

#[test]
fn checkbox_reports_toggled_state() {
    use egui_kittest::kittest::{NodeT, Queryable};
    let h = checkbox_harness(true, true);
    let node = h.get_by_role(egui::accesskit::Role::CheckBox);
    assert_eq!(
        node.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
}

#[test]
fn checkbox_box_follows_style() {
    use shadcn_egui::components::checkbox::Checkbox;
    for (name, theme, mode) in common::themes() {
        let mut on = true;
        let r = common::measure(&theme, mode, |ui| ui.add(Checkbox::new(&mut on)));
        assert_eq!(r.size(), egui::vec2(16.0, 16.0), "{name}");
    }
}

#[test]
fn checkbox_snapshot() {
    use shadcn_egui::components::checkbox::Checkbox;
    common::snapshot_all("checkbox", egui::vec2(220.0, 150.0), |ui| {
        let (mut off, mut on, mut invalid, mut disabled) = (false, true, false, true);
        ui.horizontal(|ui| {
            ui.add(Checkbox::new(&mut off));
            ui.add_space(12.0);
            ui.add(Checkbox::new(&mut on));
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add(Checkbox::new(&mut invalid).invalid(true));
            ui.add_space(12.0);
            ui.add(Checkbox::new(&mut disabled).enabled(false));
        });
        ui.add_space(8.0);
        let mut focused = true;
        let resp = ui.add(Checkbox::new(&mut focused));
        resp.request_focus();
    });
}

/// Harness state for the switch tests: the value and the rect of the
/// visible track (the returned response's rect).
struct SwitchState {
    on: bool,
    rect: egui::Rect,
}

fn switch_harness(enabled: bool, on: bool) -> egui_kittest::Harness<'static, SwitchState> {
    use shadcn_egui::components::switch::Switch;
    use shadcn_egui::Theme;
    let mut h = egui_kittest::Harness::new_ui_state(
        move |ui, s: &mut SwitchState| {
            Theme::vega().apply(ui.ctx());
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                s.rect = ui.add(Switch::new(&mut s.on).enabled(enabled)).rect;
            });
        },
        SwitchState {
            on,
            rect: egui::Rect::NOTHING,
        },
    );
    h.run();
    h
}

fn click_switch(h: &mut egui_kittest::Harness<'_, SwitchState>, pos: egui::Pos2) {
    h.hover_at(pos);
    h.run();
    h.drag_at(pos);
    h.run();
    h.drop_at(pos);
    h.run();
}

/// The response's rect is the visible track; the accessibility node carries
/// the click target, which `after:-inset-x-3 after:-inset-y-2` grows by
/// 12px x 8px.
#[test]
fn switch_track_size_and_hit_area() {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::switch::{Switch, SwitchSize};
    for (name, theme, mode) in common::themes() {
        let mut on = false;
        let r = common::measure(&theme, mode, |ui| ui.add(Switch::new(&mut on)));
        assert!(
            (r.size() - egui::vec2(32.0, 18.4)).abs().max_elem() < 0.5,
            "{name} default {:?}",
            r.size()
        );
        let r = common::measure(&theme, mode, |ui| {
            ui.add(Switch::new(&mut on).size(SwitchSize::Sm))
        });
        assert!(
            (r.size() - egui::vec2(24.0, 14.0)).abs().max_elem() < 0.5,
            "{name} sm {:?}",
            r.size()
        );
    }
    let mut h = switch_harness(true, false);
    let track = h.state().rect;
    let node = h.get_by_role(egui::accesskit::Role::CheckBox).rect();
    assert_eq!(
        node.expand2(egui::vec2(-12.0, -8.0)),
        track,
        "node rect is the track plus 12px x 8px"
    );
    click_switch(&mut h, egui::pos2(track.left() - 6.0, track.center().y));
    assert!(h.state().on, "click inside the hit area did not toggle");
    click_switch(&mut h, egui::pos2(track.left() - 20.0, track.center().y));
    assert!(h.state().on, "click beyond the hit area toggled");
}

/// Paint the switch for `on`, let the thumb settle, and return the thumb's
/// bounding box: the only shape of the thumb colour that is a circle.
fn thumb_rect(
    on: bool,
    size: shadcn_egui::components::switch::SwitchSize,
) -> (egui::Rect, egui::Rect) {
    use shadcn_egui::components::switch::Switch;
    let mode = egui::Theme::Light;
    let theme = shadcn_egui::Theme::vega();
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    ctx.set_theme(mode);
    let mut value = on;
    let mut track = egui::Rect::NOTHING;
    let mut out = None;
    for _ in 0..3 {
        out = Some(ctx.run_ui(egui::RawInput::default(), |ui| {
            track = ui.add(Switch::new(&mut value).size(size)).rect;
        }));
    }
    let mut thumb = None;
    for clipped in out.unwrap().shapes {
        if let egui::Shape::Rect(r) = clipped.shape {
            if r.fill == theme.palette_of(mode).background {
                thumb = Some(r.rect);
            }
        }
    }
    (track, thumb.expect("thumb was painted"))
}

/// The thumb lies inside the 1px border; checked it moves by its width
/// minus 2px, which leaves it 1px inside the right edge.
#[test]
fn switch_thumb_sits_inside_the_border_and_moves_when_checked() {
    use shadcn_egui::components::switch::SwitchSize;
    for size in [SwitchSize::Default, SwitchSize::Sm] {
        let (track, off) = thumb_rect(false, size);
        let (_, on) = thumb_rect(true, size);
        assert!(
            (off.left() - (track.left() + 1.0)).abs() < 0.01,
            "{size:?} off thumb left"
        );
        assert!(
            (on.right() - (track.right() - 1.0)).abs() < 0.01,
            "{size:?} on thumb right"
        );
        assert!(
            (on.center().y - track.center().y).abs() < 0.01,
            "{size:?} thumb centred"
        );
        assert!(
            (on.left() - off.left() - (off.width() - 2.0)).abs() < 0.01,
            "{size:?} travel"
        );
    }
    let (_, off) = thumb_rect(false, SwitchSize::Default);
    assert_eq!(off.size(), egui::vec2(16.0, 16.0));
    let (_, off) = thumb_rect(false, SwitchSize::Sm);
    assert_eq!(off.size(), egui::vec2(12.0, 12.0));
}

#[test]
fn disabled_switch_ignores_click_and_space() {
    use egui_kittest::kittest::Queryable;
    let mut h = switch_harness(false, true);
    let track = h.state().rect;
    click_switch(&mut h, track.center());
    assert!(h.state().on, "disabled switch toggled by click");
    h.get_by_role(egui::accesskit::Role::CheckBox).focus();
    h.run();
    h.key_press(egui::Key::Space);
    h.run();
    assert!(h.state().on, "disabled switch toggled by Space");
}

#[test]
fn switch_toggles_on_click_space_and_enter() {
    use egui_kittest::kittest::Queryable;
    let mut h = switch_harness(true, false);
    let track = h.state().rect;
    click_switch(&mut h, track.center());
    assert!(h.state().on, "click did not switch on");
    h.get_by_role(egui::accesskit::Role::CheckBox).focus();
    h.run();
    h.key_press(egui::Key::Space);
    h.run();
    assert!(!h.state().on, "Space did not switch off");
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().on, "Enter did not switch on");
}

#[test]
fn switch_reports_toggled_state() {
    use egui_kittest::kittest::{NodeT, Queryable};
    let h = switch_harness(true, true);
    let node = h.get_by_role(egui::accesskit::Role::CheckBox);
    assert_eq!(
        node.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
}

#[test]
fn switch_snapshot() {
    use shadcn_egui::components::switch::{Switch, SwitchSize};
    common::snapshot_all("switch", egui::vec2(220.0, 290.0), |ui| {
        for size in [SwitchSize::Default, SwitchSize::Sm] {
            for enabled in [true, false] {
                ui.horizontal(|ui| {
                    let (mut off, mut on) = (false, true);
                    ui.add(Switch::new(&mut off).size(size).enabled(enabled));
                    ui.add_space(12.0);
                    ui.add(Switch::new(&mut on).size(size).enabled(enabled));
                });
                ui.add_space(8.0);
            }
        }
        ui.horizontal(|ui| {
            let (mut bad, mut focused) = (true, true);
            ui.add(Switch::new(&mut bad).invalid(true));
            ui.add_space(12.0);
            let resp = ui.add(Switch::new(&mut focused));
            resp.request_focus();
        });
    });
}

/// Run a slider in a 240 x 40 harness. The value lives in a `Cell` so a test
/// can read it between steps; it is written back to `v` at the end. `f` gets
/// the harness, the cell and the slider's rect.
fn run_slider(
    v: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    enabled: bool,
    f: impl FnOnce(&mut egui_kittest::Harness<'_>, &std::cell::Cell<f32>, &std::cell::Cell<egui::Rect>),
) {
    use shadcn_egui::components::slider::Slider;
    let cell = std::cell::Cell::new(*v);
    let rect = std::cell::Cell::new(egui::Rect::NOTHING);
    {
        let mut h = egui_kittest::Harness::builder()
            .with_size(egui::vec2(240.0, 40.0))
            .build_ui(|ui| {
                shadcn_egui::Theme::vega().apply(ui.ctx());
                let mut value = cell.get();
                rect.set(
                    ui.add(
                        Slider::new(&mut value, range.clone())
                            .step(step)
                            .enabled(enabled)
                            .label("Volume"),
                    )
                    .rect,
                );
                cell.set(value);
            });
        h.run();
        f(&mut h, &cell, &rect);
    }
    *v = cell.get();
}

fn slider_press(h: &mut egui_kittest::Harness<'_>, pos: egui::Pos2) {
    h.hover_at(pos);
    h.run();
    h.drag_at(pos);
    h.run();
    h.drop_at(pos);
    h.run();
}

#[test]
fn slider_keyboard_steps() {
    use egui_kittest::kittest::Queryable;
    let mut v = 50.0;
    run_slider(&mut v, 0.0..=100.0, 10.0, true, |h, value, _| {
        h.get_by_label("Volume").focus();
        h.run();
        h.key_press(egui::Key::ArrowRight);
        h.run();
        assert_eq!(value.get(), 60.0, "ArrowRight is one step");
        h.key_press(egui::Key::ArrowDown);
        h.run();
        assert_eq!(value.get(), 50.0, "ArrowDown is one step back");
        h.key_press(egui::Key::PageUp);
        h.run();
        assert_eq!(value.get(), 100.0, "PageUp is ten steps, clamped");
        h.key_press(egui::Key::Home);
        h.run();
        assert_eq!(value.get(), 0.0, "Home");
        h.key_press(egui::Key::End);
        h.run();
        assert_eq!(value.get(), 100.0, "End");
    });
    assert_eq!(v, 100.0);
}

#[test]
fn slider_continuous_keyboard_step_is_one_percent() {
    use egui_kittest::kittest::Queryable;
    let mut v = 0.0;
    run_slider(&mut v, 0.0..=200.0, 0.0, true, |h, value, _| {
        h.get_by_label("Volume").focus();
        h.run();
        h.key_press(egui::Key::ArrowRight);
        h.run();
        assert!((value.get() - 2.0).abs() < 1e-4, "{}", value.get());
    });
}

#[test]
fn slider_click_on_track_snaps_and_drag_follows_outside() {
    let mut v = 0.0;
    run_slider(&mut v, 0.0..=100.0, 25.0, true, |h, value, rect| {
        let r = rect.get();
        // The thumb centre travels over the track minus one thumb width (16 px in vega).
        let at = |f: f32| egui::pos2(r.left() + 8.0 + (r.width() - 16.0) * f, r.center().y);
        slider_press(h, at(0.6));
        assert_eq!(value.get(), 50.0, "click at 60% snaps to 50");
        h.hover_at(at(0.1));
        h.run();
        h.drag_at(at(0.1));
        h.run();
        h.hover_at(egui::pos2(at(1.0).x + 500.0, r.center().y + 300.0));
        h.run();
        assert_eq!(
            value.get(),
            100.0,
            "drag outside the widget clamps to the end and keeps following"
        );
        h.drop_at(at(1.0));
        h.run();
    });
}

#[test]
fn slider_vertical_has_minimum_at_the_bottom() {
    use shadcn_egui::components::slider::{Slider, SliderOrientation};
    let mut h = egui_kittest::Harness::new_ui_state(
        |ui, s: &mut (f32, egui::Rect)| {
            shadcn_egui::Theme::vega().apply(ui.ctx());
            s.1 = ui
                .add(
                    Slider::new(&mut s.0, 0.0..=100.0)
                        .orientation(SliderOrientation::Vertical)
                        .label("V"),
                )
                .rect;
        },
        (50.0, egui::Rect::NOTHING),
    );
    h.run();
    let r = h.state().1;
    assert_eq!(r.height(), 160.0, "vertical min-h-40");
    slider_press_state(&mut h, egui::pos2(r.center().x, r.top() + 8.0));
    assert_eq!(h.state().0, 100.0, "top is the maximum");
    slider_press_state(&mut h, egui::pos2(r.center().x, r.bottom() - 8.0));
    assert_eq!(h.state().0, 0.0, "bottom is the minimum");
}

fn slider_press_state<S>(h: &mut egui_kittest::Harness<'_, S>, pos: egui::Pos2) {
    h.hover_at(pos);
    h.run();
    h.drag_at(pos);
    h.run();
    h.drop_at(pos);
    h.run();
}

#[test]
fn slider_degenerate_inputs_do_not_panic() {
    for (start, range, step) in [
        (5.0, 5.0..=5.0, 1.0),
        (3.0, 0.0..=10.0, 0.0),
        (3.0, 0.0..=10.0, -2.0),
        (f32::NAN, 0.0..=1.0, 0.1),
        (99.0, 0.0..=1.0, 0.1),
    ] {
        let mut v = start;
        run_slider(&mut v, range.clone(), step, true, |h, _, _| h.run_steps(2));
        assert!(v.is_finite(), "{start} {range:?} {step}");
        assert!(range.contains(&v), "{v} not in {range:?}");
    }
    // min == max: pointer and keys do nothing.
    let mut v = 5.0;
    run_slider(&mut v, 5.0..=5.0, 1.0, true, |h, value, rect| {
        slider_press(h, rect.get().center());
        h.key_press(egui::Key::End);
        h.run();
        assert_eq!(value.get(), 5.0);
    });
}

#[test]
fn disabled_slider_ignores_input() {
    use egui_kittest::kittest::Queryable;
    let mut v = 50.0;
    run_slider(&mut v, 0.0..=100.0, 0.0, false, |h, value, rect| {
        slider_press(
            h,
            egui::pos2(rect.get().left() + 2.0, rect.get().center().y),
        );
        h.get_by_label("Volume").focus();
        h.run();
        h.key_press(egui::Key::Home);
        h.run();
        assert_eq!(value.get(), 50.0);
    });
    assert_eq!(v, 50.0);
}

#[test]
fn slider_reports_role_label_and_value_to_accesskit() {
    use egui_kittest::kittest::{NodeT, Queryable};
    let mut v = 30.0;
    run_slider(&mut v, 0.0..=100.0, 0.0, true, |h, _, _| {
        let node = h.get_by_role_and_label(egui::accesskit::Role::Slider, "Volume");
        assert_eq!(node.accesskit_node().numeric_value(), Some(30.0));
    });
}

#[test]
fn slider_snapshot() {
    use shadcn_egui::components::slider::{Slider, SliderOrientation};
    let (mut a, mut b, mut c, mut d) = (30.0, 70.0, 50.0, 100.0);
    common::snapshot_all("slider", egui::vec2(320.0, 200.0), |ui| {
        shadcn_egui::layout::row(ui, 24.0, |ui| {
            shadcn_egui::layout::vstack(ui, 16.0, |ui| {
                ui.set_width(200.0);
                ui.add(Slider::new(&mut a, 0.0..=100.0).label("A"));
                ui.add(Slider::new(&mut b, 0.0..=100.0).enabled(false).label("B"));
                ui.add(Slider::new(&mut d, 0.0..=100.0).label("D"));
            });
            ui.add(
                Slider::new(&mut c, 0.0..=100.0)
                    .orientation(SliderOrientation::Vertical)
                    .label("C"),
            );
        });
    });
}

/// Every rect shape of a frame with its fill and stroke, in paint order.
fn rect_paint(out: &egui::FullOutput) -> Vec<(egui::Rect, egui::Color32, egui::Stroke)> {
    common::rect_shapes(out)
        .into_iter()
        .map(|r| (r.rect, r.fill, r.stroke))
        .collect()
}

/// The colour and opacity of every text a frame painted, in paint order.
fn text_paint(out: &egui::FullOutput) -> Vec<(egui::Color32, egui::Color32, f32)> {
    fn walk(s: &egui::Shape, acc: &mut Vec<(egui::Color32, egui::Color32, f32)>) {
        match s {
            egui::Shape::Text(t) => {
                let first = t
                    .galley
                    .rows
                    .iter()
                    .find_map(|r| r.visuals.mesh.vertices.first())
                    .map_or(egui::Color32::TRANSPARENT, |v| v.color);
                acc.push((first, t.fallback_color, t.opacity_factor));
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, acc)),
            _ => {}
        }
    }
    let mut acc = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut acc));
    acc
}

/// shadcn `disabled:opacity-50` applies once: a widget disabled by its own
/// flag, by a disabled `Ui`, or by both paints exactly the same shapes.
#[test]
fn disabled_widgets_fade_once() {
    use shadcn_egui::components::checkbox::Checkbox;
    use shadcn_egui::components::input::Input;
    use shadcn_egui::components::label::Label;
    use shadcn_egui::components::select::Select;
    use shadcn_egui::components::slider::Slider;
    use shadcn_egui::components::switch::Switch;
    use shadcn_egui::components::tabs::{Tabs, TabsVariant};
    type Add = fn(&mut egui::Ui, bool) -> egui::Response;
    let widgets: [(&str, Add); 8] = [
        ("button", |ui, on| ui.add(Button::new("Save").enabled(on))),
        ("label", |ui, on| ui.add(Label::new("Email").enabled(on))),
        ("input", |ui, on| {
            ui.add(Input::new(&mut "text".to_owned()).enabled(on))
        }),
        ("switch", |ui, on| {
            ui.add(Switch::new(&mut true).enabled(on))
        }),
        ("checkbox", |ui, on| {
            ui.add(Checkbox::new(&mut true).enabled(on))
        }),
        ("select", |ui, on| {
            ui.add(Select::new("s", &mut 0, &["One", "Two"]).enabled(on))
        }),
        ("slider", |ui, on| {
            ui.add(Slider::new(&mut 30.0, 0.0..=100.0).enabled(on))
        }),
        // The line variant: the default variant's list track does not fade
        // with its triggers, but does with a disabled `Ui`.
        ("tabs", |ui, on| {
            ui.add(
                Tabs::new(&mut 0, &["A", "B"])
                    .variant(TabsVariant::Line)
                    .enabled(on),
            )
        }),
    ];
    let mode = egui::Theme::Light;
    let theme = Theme::vega();
    for (name, add) in widgets {
        let (_, own) = common::measure_output(&theme, mode, |ui| add(ui, false));
        let (_, by_ui) = common::measure_output(&theme, mode, |ui| {
            ui.add_enabled_ui(false, |ui| add(ui, true)).inner
        });
        let (_, both) = common::measure_output(&theme, mode, |ui| {
            ui.add_enabled_ui(false, |ui| add(ui, false)).inner
        });
        assert_eq!(
            rect_paint(&own),
            rect_paint(&by_ui),
            "{name}: own flag vs disabled Ui"
        );
        assert_eq!(
            rect_paint(&both),
            rect_paint(&by_ui),
            "{name}: both vs disabled Ui"
        );
        assert_eq!(
            text_paint(&own),
            text_paint(&by_ui),
            "{name}: text, own flag vs disabled Ui"
        );
        assert_eq!(
            text_paint(&both),
            text_paint(&by_ui),
            "{name}: text, both vs disabled Ui"
        );
    }
    // And the fade is a half: a solid button's fill.
    let (rect, out) = common::measure_output(&theme, mode, |ui| {
        ui.add_enabled(false, Button::new("Save"))
    });
    let fill = common::rect_shapes(&out)
        .into_iter()
        .find(|r| r.rect == rect.shrink(1.0))
        .expect("button fill")
        .fill;
    assert_eq!(fill, theme.palette_of(mode).primary.gamma_multiply(0.5));
}

/// A click selects a tab without the keyboard focus ring; the arrow keys
/// move the focus, and with it the ring.
#[test]
fn tab_click_shows_no_focus_ring() {
    use egui_kittest::kittest::Queryable;
    let three_px = |h: &egui_kittest::Harness<'_>| {
        h.output()
            .shapes
            .iter()
            .any(|c| matches!(&c.shape, egui::Shape::Rect(r) if r.stroke.width == 3.0))
    };
    let (mut h, active) = tabs_harness(&["Account", "Notifications", "Display"], 0);
    h.run();
    let at = h.get_by_label("Notifications").rect().center();
    h.hover_at(at);
    h.run();
    h.drag_at(at);
    h.run();
    h.drop_at(at);
    h.run();
    assert_eq!(active.get(), 1);
    assert!(!three_px(&h), "a mouse click shows the focus ring");
    h.get_by_label("Notifications").focus();
    h.run();
    h.key_press(egui::Key::ArrowRight);
    h.run();
    assert_eq!(active.get(), 2);
    assert!(three_px(&h), "keyboard focus shows the ring");
}

/// `description` is `text-sm`: each wrapped line is 20px, Tailwind's line
/// height for `text-sm`, not egui's font row height.
#[test]
fn description_lines_are_text_sm_line_height() {
    use shadcn_egui::components::label::description;
    for (name, theme, mode) in common::themes() {
        let one = common::measure(&theme, mode, |ui| description(ui, "Short."));
        assert_eq!(one.height(), 20.0, "{name}");
        let wrapped = common::measure(&theme, mode, |ui| {
            ui.set_max_width(120.0);
            description(
                ui,
                "We will never share your email with anyone else, ever, at all.",
            )
        });
        let lines = (wrapped.height() / 20.0).round();
        assert!(lines >= 3.0, "{name}: {wrapped:?}");
        assert_eq!(wrapped.height(), lines * 20.0, "{name}");
    }
}

/// The switch paints the shadow its style sets (`shadow-xs` on vega's track).
#[test]
fn switch_paints_its_style_shadow() {
    use shadcn_egui::components::switch::Switch;
    use shadcn_egui::style::{pick, State};
    let mode = egui::Theme::Light;
    let theme = Theme::vega();
    assert!(pick(&[&*theme.style.switch], |r| &r.shadow, State::NONE).is_some());
    let (_, out) = common::measure_output(&theme, mode, |ui| ui.add(Switch::new(&mut false)));
    // The shadow is egui's blurred rect, cut to a mesh outside the box.
    assert!(
        common::mesh_shapes(&out)
            .iter()
            .any(|m| m.vertices.iter().any(|v| v.color.a() > 0)),
        "no shadow painted"
    );
}

/// An outer `box-shadow` paints only outside the border box (conformance
/// 2.5): vega's light input has a transparent fill and `shadow-xs`, and no
/// part of the shadow lies under it.
#[test]
fn shadow_paints_only_outside_the_border_box() {
    let mode = egui::Theme::Light;
    let theme = Theme::vega();
    let mut text = String::new();
    let (rect, out) = common::measure_output(&theme, mode, |ui| {
        ui.add(shadcn_egui::components::input::Input::new(&mut text))
    });
    let meshes = common::mesh_shapes(&out);
    assert!(!meshes.is_empty(), "no shadow painted");
    // Inside the box: the rect minus its rounded corners, with a hair of slack.
    let r = 12.0;
    let inside = |p: egui::Pos2| {
        let a = rect.shrink2(egui::vec2(r, 0.01));
        let b = rect.shrink2(egui::vec2(0.01, r));
        a.contains(p) || b.contains(p)
    };
    for m in &meshes {
        for tri in m.indices.as_chunks::<3>().0 {
            let c = tri
                .iter()
                .map(|&i| m.vertices[i as usize].pos.to_vec2())
                .fold(egui::Vec2::ZERO, |a, b| a + b)
                / 3.0;
            assert!(
                !inside(c.to_pos2()),
                "shadow triangle at {c:?} under {rect:?}"
            );
        }
    }
}

/// A container's fade ends with its content, but a `Ui::disable` inside
/// the content (for example a disabled card action) keeps egui's disabled
/// fade for what the `Ui` paints afterwards, as the `Ui` stays disabled.
#[test]
fn content_ui_keeps_a_disable_inside_its_content() {
    use shadcn_egui::components::shared::content_ui;
    use shadcn_egui::style::{Recipe, State};
    let mut half = Recipe::default();
    half.opacity.base = Some(0.5);
    let chain: &[&[&Recipe]] = &[&[&half]];
    let ctx = egui::Context::default();
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
        let alpha = ui.visuals().disabled_alpha();
        content_ui(ui, egui::Id::new("part"), chain, State::NONE, |ui| {
            assert_eq!(ui.opacity(), 0.5);
            ui.disable()
        });
        assert!(!ui.is_enabled());
        assert!(
            (ui.opacity() - alpha).abs() < 1e-6,
            "opacity {} after a disable inside content_ui, egui's fade is {alpha}",
            ui.opacity()
        );
        content_ui(ui, egui::Id::new("part"), chain, State::NONE, |_| {});
        assert!(
            (ui.opacity() - alpha).abs() < 1e-6,
            "an already disabled Ui"
        );
    });
}

/// A crate part paints at the `Ui`'s opacity with one egui disabled fade
/// taken out, times its own value (conformance 2.2, implementation notes
/// 3): its `disabled:opacity-25` gives 25 %, not 25 % of egui's fade, and a
/// part without a `disabled:` rule stays opaque in a disabled `Ui`.
#[test]
fn part_opacity_takes_one_egui_fade_out() {
    use shadcn_egui::components::shared::part_opacity;
    use shadcn_egui::style::{Recipe, Rule, State, When};
    let mut quarter = Recipe::default();
    quarter.opacity.rules.push(Rule {
        when: When::new(State::DISABLED, State::NONE),
        rank: 1,
        value: 0.25,
    });
    let plain = Recipe::default();
    let ctx = egui::Context::default();
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
        assert_eq!(
            part_opacity(ui, egui::Id::new("part"), &[&[&quarter]], State::NONE),
            1.0
        );
        ui.add_enabled_ui(false, |ui| {
            assert!(
                (part_opacity(ui, egui::Id::new("part"), &[&[&quarter]], State::NONE) - 0.25).abs()
                    < 1e-6
            );
            assert!(
                (part_opacity(ui, egui::Id::new("part"), &[&[&plain]], State::NONE) - 1.0).abs()
                    < 1e-6
            );
            // A nested disable fades twice in egui; one fade only comes out.
            ui.add_enabled_ui(false, |ui| {
                let fade = ui.visuals().disabled_alpha();
                let got = part_opacity(ui, egui::Id::new("part"), &[&[&plain]], State::NONE);
                assert!((got - fade).abs() < 1e-6, "{got}");
            });
        });
    });
}

/// `Theme::apply` gives plain egui widgets the style's control opacity.
#[test]
fn theme_sets_disabled_alpha_from_the_style() {
    use shadcn_egui::style::{pick, State};
    for (name, theme, mode) in common::themes() {
        let ctx = egui::Context::default();
        theme.apply(&ctx);
        ctx.set_theme(mode);
        let want = pick(&[&*theme.style.button], |r| &r.opacity, State::DISABLED).unwrap();
        assert_eq!(ctx.global_style().visuals.disabled_alpha, want, "{name}");
    }
}

/// shadcn input `text-base md:text-sm`: the field text is 16 px in a window
/// narrower than 768 px, 14 px from 768 px up.
#[test]
fn input_text_follows_md_breakpoint() {
    use shadcn_egui::components::input::Input;
    let mode = egui::Theme::Light;
    let theme = Theme::vega();
    let text_px = |width: f32| {
        let ctx = egui::Context::default();
        theme.apply(&ctx);
        ctx.set_theme(mode);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 400.0),
            )),
            ..Default::default()
        };
        let mut text = String::from("Hello");
        let mut out = None;
        for _ in 0..2 {
            out = Some(ctx.run_ui(input.clone(), |ui| {
                ui.add(Input::new(&mut text));
            }));
        }
        common::text_shapes(&out.unwrap())
            .iter()
            .map(|t| t.galley.job.sections[0].format.font_id.size)
            .next()
            .expect("the field's text")
    };
    assert_eq!(text_px(767.0), theme.metrics.text_base);
    assert_eq!(text_px(768.0), theme.metrics.text_sm);
}
