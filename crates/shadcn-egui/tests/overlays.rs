mod common;

use egui::vec2;
use egui::Key;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::components::dialog::{self, Dialog};
use shadcn_egui::components::popover::{popover_header, Popover};
use shadcn_egui::components::select::{Select, SelectSize};
use shadcn_egui::components::tooltip::tooltip;
use shadcn_egui::Theme;
use shadcn_egui::{MenuColor, Mode, Palette, Preset, StyleName};

/// A dialog harness: `open` starts true; the body has a button that counts clicks.
fn dialog_harness(open: bool) -> Harness<'static, (bool, u32)> {
    Harness::builder()
        .with_size(vec2(640.0, 360.0))
        .build_ui_state(
            |ui, (open, clicks): &mut (bool, u32)| {
                Theme::vega_dark().apply(ui.ctx());
                ui.label("page beneath");
                Dialog::new("confirm", "Delete session?")
                    .description("The session and its files are removed.")
                    .show(ui.ctx(), open, |ui| {
                        dialog::footer(ui, |ui| {
                            if ui
                                .add(Button::new("Delete").variant(ButtonVariant::Destructive))
                                .clicked()
                            {
                                *clicks += 1;
                            }
                            ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                        });
                    });
            },
            (open, 0),
        )
}

#[test]
fn dialog_body_is_interactive_and_stays_open() {
    let mut h = dialog_harness(true);
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    assert_eq!(h.state().1, 1);
    assert!(h.state().0, "a click inside the card must not close it");
}

#[test]
fn dialog_closes_on_cross() {
    let mut h = dialog_harness(true);
    h.run();
    h.get_by_label("Close").click();
    h.run();
    assert!(!h.state().0);
}

#[test]
fn dialog_closes_on_escape() {
    let mut h = dialog_harness(true);
    h.run();
    h.key_press(Key::Escape);
    h.run();
    assert!(!h.state().0);
}

#[test]
fn dialog_closes_on_backdrop_click() {
    let mut h = dialog_harness(true);
    h.run();
    h.get_by_label("page beneath").click();
    h.run();
    assert!(!h.state().0, "a backdrop click closes the dialog");
}

#[test]
fn closed_dialog_draws_nothing() {
    let mut h = dialog_harness(false);
    h.run();
    assert!(h.query_by_label("Delete session?").is_none());
}

/// Inner width of a dialog under `theme` in a window of `screen` width.
fn dialog_inner_width(theme: &Theme, screen: f32) -> f32 {
    let inner = std::cell::Cell::new(0.0_f32);
    let mut h = Harness::builder()
        .with_size(vec2(screen, 400.0))
        .build_ui(|ui| {
            theme.apply(ui.ctx());
            Dialog::new("w", "Title")
                .show(ui.ctx(), &mut true, |ui| inner.set(ui.available_width()));
        });
    h.run();
    h.run();
    inner.get()
}

#[test]
fn dialog_box_is_max_w_sm_or_the_window_minus_32() {
    for (name, theme) in common::themes() {
        let pad = theme
            .style
            .dialog_content
            .pad_left
            .base
            .expect("fixed padding");
        let max = theme.style.dialog_content.max_width.rules[0].value;
        assert_eq!(
            dialog_inner_width(&theme, 1024.0) + 2.0 * pad,
            max,
            "{name}: sm:max-w-*"
        );
        assert_eq!(
            dialog_inner_width(&theme, 300.0) + 2.0 * pad,
            300.0 - 32.0,
            "{name}: window minus 32"
        );
    }
    let nova = common::themes()
        .into_iter()
        .find(|(n, _)| *n == "nova_dark")
        .expect("nova")
        .1;
    assert_eq!(
        dialog_inner_width(&nova, 1024.0) + 32.0,
        384.0,
        "nova is max-w-sm"
    );
}

#[test]
fn dialog_close_button_can_be_hidden() {
    let mut h = Harness::builder()
        .with_size(vec2(640.0, 360.0))
        .build_ui(|ui| {
            Theme::vega_dark().apply(ui.ctx());
            Dialog::new("nc", "Title")
                .show_close_button(false)
                .show(ui.ctx(), &mut true, |_| {});
        });
    h.run();
    assert!(h.query_by_label("Close").is_none());
}

/// A part paints at its own `disabled:` opacity, not egui's fade
/// (conformance 2.2): nova's footer strip has no `disabled:` rule, so in a
/// disabled area of the dialog it keeps its full colour, as in a disabled
/// `<fieldset>`.
#[test]
fn a_disabled_dialog_footer_keeps_its_strip_colour() {
    use shadcn_egui::style::{pick, State};
    let theme = Theme::new(shadcn_egui::Style::nova(), Palette::light(), 10.0);
    let strip = pick(&[&*theme.style.dialog_footer], |r| &r.bg, State::NONE)
        .map(|p| theme.paint(p))
        .expect("nova's footer has a strip");
    let fills = |disabled: bool| {
        let ctx = egui::Context::default();
        theme.apply(&ctx);
        // No fade-in of the modal's area.
        ctx.global_style_mut(|s| s.animation_time = 0.0);
        let mut out = None;
        for _ in 0..2 {
            out = Some(ctx.run_ui(egui::RawInput::default(), |ui| {
                Dialog::new("d", "Title").show(ui.ctx(), &mut true, |ui| {
                    ui.add_enabled_ui(!disabled, |ui| dialog::footer(ui, |_| {}));
                });
            }));
        }
        common::rect_shapes(&out.expect("ran"))
            .into_iter()
            .map(|r| r.fill)
            .collect::<Vec<_>>()
    };
    assert!(fills(false).contains(&strip), "enabled strip");
    assert!(
        fills(true).contains(&strip),
        "disabled strip keeps {strip:?}"
    );
}

#[test]
fn dialog_snapshot() {
    common::snapshot_all("dialog", vec2(800.0, 500.0), |ui| {
        ui.label("page beneath");
        Dialog::new("confirm", "Delete session?")
            .description("The session and its files are removed.")
            .show(ui.ctx(), &mut true, |ui| {
                ui.label("This cannot be undone.");
                dialog::footer(ui, |ui| {
                    ui.add(Button::new("Delete").variant(ButtonVariant::Destructive));
                    ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                });
            });
    });
}

fn popover_trigger(ui: &mut egui::Ui) {
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        let trigger = ui.add(Button::new("Dimensions").variant(ButtonVariant::Outline));
        Popover::new(&trigger).show(|ui| {
            popover_header(ui, "Layer size", "Set the dimensions for the layer.");
            ui.label("Width 100%");
        });
        ui.add_space(120.0);
        ui.label("elsewhere");
    });
}

fn popover_harness() -> Harness<'static> {
    Harness::builder()
        .with_size(vec2(420.0, 240.0))
        .build_ui(|ui| {
            Theme::vega_dark().apply(ui.ctx());
            popover_trigger(ui);
        })
}

#[test]
fn popover_toggles_and_closes_outside() {
    let mut h = popover_harness();
    h.run();
    assert!(h
        .query_by_label("Set the dimensions for the layer.")
        .is_none());
    h.get_by_label("Dimensions").click();
    h.run();
    h.get_by_label("Set the dimensions for the layer.");
    h.get_by_label("elsewhere").click();
    h.run();
    assert!(
        h.query_by_label("Set the dimensions for the layer.")
            .is_none(),
        "click outside closes"
    );
}

#[test]
fn popover_closes_on_escape_and_toggles_on_trigger() {
    let mut h = popover_harness();
    h.run();
    h.get_by_label("Dimensions").click();
    h.run();
    h.get_by_label("Set the dimensions for the layer.");
    h.key_press(Key::Escape);
    h.run();
    assert!(
        h.query_by_label("Set the dimensions for the layer.")
            .is_none(),
        "Esc closes"
    );
    h.get_by_label("Dimensions").click();
    h.run();
    h.get_by_label("Set the dimensions for the layer.");
    h.get_by_label("Dimensions").click();
    h.run();
    assert!(
        h.query_by_label("Set the dimensions for the layer.")
            .is_none(),
        "the trigger toggles"
    );
}

#[test]
fn popover_box_is_w_72_and_content_gets_the_padding() {
    for (name, theme) in common::themes() {
        let inner = std::cell::Cell::new(0.0_f32);
        let mut h = Harness::builder()
            .with_size(vec2(420.0, 240.0))
            .build_ui(|ui| {
                theme.apply(ui.ctx());
                let trigger = ui.add(Button::new("Dimensions"));
                Popover::new(&trigger).show(|ui| inner.set(ui.available_width()));
            });
        h.run();
        h.get_by_label("Dimensions").click();
        h.run();
        let pad = theme
            .style
            .popover_content
            .pad_left
            .base
            .expect("fixed padding");
        assert_eq!(
            inner.get() + 2.0 * pad,
            288.0,
            "{name}: w-72 includes the padding"
        );
    }
}

#[test]
fn popover_snapshot() {
    common::snapshot_all_with("popover", vec2(420.0, 240.0), popover_trigger, |h| {
        h.get_by_label("Dimensions").click();
        h.run();
        h.get_by_label("elsewhere").hover();
        h.run();
    });
}

fn tooltip_trigger(ui: &mut egui::Ui) {
    ui.style_mut().interaction.tooltip_delay = 0.0;
    ui.add_space(32.0);
    ui.horizontal(|ui| {
        ui.add_space(32.0);
        tooltip(
            ui.add(Button::new("Hover me").variant(ButtonVariant::Outline)),
            "Add to library",
        );
    });
}

#[test]
fn tooltip_shows_on_hover() {
    let mut h = Harness::builder()
        .with_size(vec2(240.0, 120.0))
        .build_ui(|ui| {
            Theme::vega_dark().apply(ui.ctx());
            tooltip_trigger(ui);
        });
    h.run();
    assert!(h.query_by_label("Add to library").is_none());
    h.get_by_label("Hover me").hover();
    h.run();
    h.get_by_label("Add to library");
}

#[test]
fn tooltip_is_inverted_with_an_arrow_toward_the_trigger() {
    for (name, theme) in common::themes() {
        let mut h = Harness::builder()
            .with_size(vec2(240.0, 120.0))
            .build_ui(|ui| {
                theme.apply(ui.ctx());
                tooltip_trigger(ui);
            });
        h.run();
        let trigger = h.get_by_label("Hover me").rect();
        h.get_by_label("Hover me").hover();
        h.run();
        let text = h.get_by_label("Add to library").rect();
        let img = h.render().expect("render");
        let px = |p: egui::Pos2| {
            let c = img.get_pixel(p.x as u32, p.y as u32);
            egui::Color32::from_rgb(c[0], c[1], c[2])
        };
        // bg-foreground behind the text; the arrow reaches into the trigger below.
        let fg = theme.palette.foreground;
        let corner = egui::pos2(text.left() - 4.0, text.center().y);
        assert_eq!(px(corner), fg, "{name}: fill is foreground");
        let tip = egui::pos2(text.center().x, trigger.top() - 2.0);
        assert_eq!(px(tip), fg, "{name}: arrow reaches the trigger");
    }
}

/// The tooltip paints its box through the recipe box: a border an app
/// gives it in `theme.style` shows (review F4).
#[test]
fn tooltip_box_takes_the_styles_border() {
    use shadcn_egui::style::{Paint, Prop, Role};
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.tooltip_content.border_width = Prop::fixed(4.0);
        s.tooltip_content.border_color = Prop::fixed(Paint::Role(Role::Destructive, 1.0));
    });
    let red = theme.palette.destructive;
    let mut h = Harness::builder()
        .with_size(vec2(240.0, 120.0))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            tooltip_trigger(ui);
        });
    h.run();
    h.get_by_label("Hover me").hover();
    h.run();
    let text = h.get_by_label("Add to library").rect();
    let img = h.render().expect("render");
    let y = text.center().y as u32;
    let found = (0..text.left() as u32).rev().any(|x| {
        let c = img.get_pixel(x, y);
        egui::Color32::from_rgb(c[0], c[1], c[2]) == red
    });
    assert!(found, "no destructive border left of the tooltip text");
}

#[test]
fn tooltip_snapshot() {
    common::snapshot_all_with("tooltip", vec2(240.0, 140.0), tooltip_trigger, |h| {
        h.get_by_label("Hover me").hover();
        h.run();
    });
}

const FRUITS: [&str; 4] = ["Apple", "Banana", "Blueberry", "Cherry"];

fn select_harness(start: usize) -> Harness<'static, usize> {
    Harness::builder()
        .with_size(vec2(320.0, 260.0))
        .build_ui_state(
            |ui, value: &mut usize| {
                Theme::vega_dark().apply(ui.ctx());
                ui.add(Select::new("fruit", value, &FRUITS).placeholder("Pick a fruit"));
                ui.label("elsewhere");
            },
            start,
        )
}

fn open_select(h: &mut Harness<'_, usize>) {
    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run();
}

#[test]
fn select_opens_and_a_click_chooses() {
    let mut h = select_harness(0);
    h.run();
    assert!(
        h.query_by_label("Cherry").is_none(),
        "closed list shows no options"
    );
    open_select(&mut h);
    h.get_by_label("Cherry").click();
    h.run();
    assert_eq!(*h.state(), 3);
    assert!(
        h.query_by_label("Banana").is_none(),
        "choosing closes the list"
    );
}

#[test]
fn select_keyboard_down_enter_chooses_the_next_option() {
    let mut h = select_harness(0);
    h.run();
    open_select(&mut h);
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(*h.state(), 0, "moving the highlight does not choose");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(*h.state(), 1);
    assert!(
        h.query_by_label("Cherry").is_none(),
        "Enter closes the list"
    );
}

#[test]
fn select_escape_and_outside_click_keep_the_value() {
    let mut h = select_harness(0);
    h.run();
    open_select(&mut h);
    h.key_press(Key::ArrowDown);
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(*h.state(), 0);
    assert!(h.query_by_label("Cherry").is_none(), "Esc closes");
    open_select(&mut h);
    h.get_by_label("elsewhere").click();
    h.run();
    assert_eq!(*h.state(), 0);
    assert!(
        h.query_by_label("Cherry").is_none(),
        "a click outside closes"
    );
}

/// An open list whose trigger lost focus to another widget leaves the keys
/// to that widget: typing and Enter do not choose an option.
#[test]
fn open_select_ignores_keys_once_focus_moves_elsewhere() {
    let mut h = Harness::builder()
        .with_size(vec2(320.0, 260.0))
        .build_ui_state(
            |ui, value: &mut usize| {
                Theme::vega_dark().apply(ui.ctx());
                ui.add(Select::new("fruit", value, &FRUITS));
                ui.add(shadcn_egui::components::button::Button::new("Other"));
            },
            0,
        );
    h.run();
    open_select(&mut h);
    h.get_by_label("Other").focus();
    h.run();
    // Typing moves the highlight of a list that has the keys; arrows are
    // left out, as egui moves focus with them.
    h.event(egui::Event::Text("c".into()));
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(*h.state(), 0);
}

#[test]
fn select_typing_jumps_to_the_next_option_with_that_letter() {
    let mut h = select_harness(0);
    h.run();
    open_select(&mut h);
    h.get_by_role(egui::accesskit::Role::ComboBox)
        .type_text("b");
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(
        *h.state(),
        1,
        "first option after the highlight starting with b"
    );
}

/// The trigger's accessibility value is the shown text; it has no name of
/// its own (a form names it with `labelled_by`).
#[test]
fn select_trigger_reports_the_current_text() {
    let mut h = select_harness(2);
    h.run();
    let node = h
        .get_by_role(egui::accesskit::Role::ComboBox)
        .accesskit_node();
    assert_eq!(node.value().as_deref(), Some("Blueberry"));
    assert_eq!(node.label(), None);
    let mut h = select_harness(9);
    h.run();
    assert_eq!(
        h.get_by_role(egui::accesskit::Role::ComboBox)
            .accesskit_node()
            .value()
            .as_deref(),
        Some("Pick a fruit")
    );
}

#[test]
fn select_disabled_does_not_open() {
    let mut h = Harness::builder()
        .with_size(vec2(320.0, 260.0))
        .build_ui_state(
            |ui, value: &mut usize| {
                Theme::vega_dark().apply(ui.ctx());
                ui.add(Select::new("fruit", value, &FRUITS).enabled(false));
            },
            0usize,
        );
    h.run();
    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run();
    assert!(h.query_by_label("Cherry").is_none());
}

#[test]
fn select_trigger_height_follows_style_and_size() {
    for (name, theme) in common::themes() {
        let (default_h, sm_h) = if name.starts_with("nova") {
            (32.0, 28.0)
        } else {
            (36.0, 32.0)
        };
        let mut v = 0usize;
        let mut h = |size| {
            common::measure(&theme, |ui| {
                ui.add(Select::new("s", &mut v, &FRUITS).size(size))
            })
            .height()
        };
        assert_eq!(h(SelectSize::Default), default_h, "{name}");
        assert_eq!(h(SelectSize::Sm), sm_h, "{name}");
    }
}

fn select_triggers(ui: &mut egui::Ui) {
    let (mut a, mut b, mut c, mut d, mut e) = (9usize, 1usize, 1usize, 0usize, 2usize);
    ui.spacing_mut().item_spacing.y = 12.0;
    ui.add(
        Select::new("a", &mut a, &FRUITS)
            .placeholder("Pick a fruit")
            .width(180.0),
    );
    ui.add(Select::new("b", &mut b, &FRUITS).width(180.0));
    ui.add(Select::new("c", &mut c, &FRUITS).invalid(true).width(180.0));
    ui.add(
        Select::new("d", &mut d, &FRUITS)
            .enabled(false)
            .width(180.0),
    );
    ui.add(Select::new("e", &mut e, &FRUITS).size(SelectSize::Sm));
}

#[test]
fn select_trigger_snapshot() {
    common::snapshot_all("select_trigger", vec2(240.0, 260.0), select_triggers);
}

#[test]
fn select_open_snapshot() {
    common::snapshot_all_with(
        "select_open",
        vec2(260.0, 260.0),
        |ui| {
            let mut v = 1usize;
            ui.add(Select::new("fruit", &mut v, &FRUITS).width(180.0));
            ui.add_space(200.0);
            ui.label("elsewhere");
        },
        |h| {
            h.get_by_role(egui::accesskit::Role::ComboBox).click();
            h.run();
            h.get_by_label("elsewhere").hover();
            h.run();
        },
    );
}

#[test]
fn select_item_rows_have_the_text_sm_line_plus_padding() {
    for (name, theme) in common::themes() {
        let mut h = Harness::builder()
            .with_size(vec2(320.0, 260.0))
            .build_ui_state(
                |ui, value: &mut usize| {
                    theme.apply(ui.ctx());
                    ui.add(Select::new("fruit", value, &FRUITS));
                },
                0usize,
            );
        h.run();
        open_select(&mut h);
        // 20 px line plus py-1.5 (vega) or py-1 (nova).
        let want = if name.starts_with("nova") { 28.0 } else { 32.0 };
        assert_eq!(h.get_by_label("Cherry").rect().height(), want, "{name}");
    }
}

/// An option wider than the window caps the list to the window (conformance
/// 3.1); its text wraps in its row, as the browser wraps it.
#[test]
fn select_list_is_capped_to_the_window() {
    const LONG: [&str; 2] = [
        "Short",
        "An option whose text is far wider than this narrow window",
    ];
    let mut h = Harness::builder()
        .with_size(vec2(200.0, 300.0))
        .build_ui_state(
            |ui, value: &mut usize| {
                Theme::vega_light().apply(ui.ctx());
                ui.add(Select::new("long", value, &LONG).width(150.0));
            },
            0usize,
        );
    h.run();
    open_select(&mut h);
    let short = h.get_by_label(LONG[0]).rect();
    let long = h.get_by_label(LONG[1]).rect();
    assert!(long.right() <= 200.0, "row ends at {}", long.right());
    assert_eq!(long.width(), short.width(), "rows share the list width");
    assert!(
        long.height() > short.height(),
        "the long text wraps: {} vs {}",
        long.height(),
        short.height()
    );
}

/// The trigger is `w-fit`, capped to the room (conformance 3.1): an option
/// wider than the window, or a wider `width`, keeps it inside the window.
#[test]
fn select_trigger_is_capped_to_the_room() {
    const LONG: [&str; 2] = [
        "Short",
        "An option whose text is far wider than this narrow window",
    ];
    for width in [None, Some(400.0)] {
        let mut h = Harness::builder()
            .with_size(vec2(200.0, 300.0))
            .build_ui_state(
                move |ui, value: &mut usize| {
                    Theme::vega_light().apply(ui.ctx());
                    let select = Select::new("long", value, &LONG);
                    match width {
                        Some(w) => ui.add(select.width(w)),
                        None => ui.add(select),
                    };
                },
                0usize,
            );
        h.run();
        let trigger = h.get_by_role(egui::accesskit::Role::ComboBox).rect();
        assert!(
            trigger.right() <= 200.0,
            "{width:?}: trigger ends at {}",
            trigger.right()
        );
    }
}

/// Where the room is narrower than the trigger's padding, gap and chevron,
/// the trigger takes that content minimum (conformance 3.1) and overflows
/// without widening its parent (3.6).
#[test]
fn select_trigger_in_no_room_takes_its_content_minimum() {
    let ctx = egui::Context::default();
    Theme::vega_light().apply(&ctx);
    let mut v = 0usize;
    let mut rects = (egui::Rect::NOTHING, egui::Rect::NOTHING);
    for _ in 0..2 {
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let parent = ui.allocate_ui(vec2(0.0, 100.0), |ui| {
                ui.add(Select::new("fruit", &mut v, &FRUITS)).rect
            });
            rects = (parent.inner, parent.response.rect);
        });
    }
    let (trigger, parent) = rects;
    // vega: pl-2.5, pr-2, a 1 px border each side, gap-1.5, a size-4 chevron.
    assert_eq!(trigger.width(), 10.0 + 8.0 + 2.0 + 6.0 + 16.0);
    assert_eq!(parent.width(), 0.0, "the parent keeps its width");
}

/// Pixels darker than (60, 60, 60) once the list of a light-mode Select is open.
fn dark_pixels_of_open_list(theme: Theme) -> usize {
    let bg = theme.palette.background;
    let mut h = Harness::builder()
        .with_size(vec2(260.0, 260.0))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            ui.ctx()
                .layer_painter(egui::LayerId::background())
                .rect_filled(ui.ctx().viewport_rect(), 0.0, bg);
            let mut v = 1usize;
            ui.add(Select::new("fruit", &mut v, &FRUITS).width(180.0));
        });
    h.run();
    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run();
    let img = h.render().expect("wgpu render");
    img.pixels()
        .filter(|p| p.0[0] < 60 && p.0[1] < 60 && p.0[2] < 60)
        .count()
}

#[test]
fn an_inverted_menu_is_dark_in_light_mode() {
    let inverted = Preset::default()
        .style(StyleName::Vega)
        .menu_color(MenuColor::Inverted)
        .theme(Mode::Light);
    let mt = inverted.menu_theme();
    assert!(
        mt.dark && mt.palette == Palette::dark(),
        "the menu resolves as dark mode"
    );
    let plain =
        dark_pixels_of_open_list(Preset::default().style(StyleName::Vega).theme(Mode::Light));
    let dark = dark_pixels_of_open_list(inverted);
    // The open list is ~180 x 140 px: filled dark it adds well over 10 000
    // dark pixels; a light list only has its text.
    assert!(
        dark > 3 * plain + 5000,
        "inverted list must be dark: {dark} dark px, default list {plain}"
    );
}

/// An open Select in a preset's menu colours, light and dark.
fn select_open_in(name: &str, color: MenuColor) {
    let themes = vec![
        // Preset::default() is Nova (ui.shadcn.com DEFAULT_CONFIG); these snapshots show Vega.
        (
            "vega_light",
            Preset::default()
                .style(StyleName::Vega)
                .menu_color(color)
                .theme(Mode::Light),
        ),
        (
            "vega_dark",
            Preset::default()
                .style(StyleName::Vega)
                .menu_color(color)
                .theme(Mode::Dark),
        ),
    ];
    common::snapshot_with_themes(
        name,
        vec2(260.0, 260.0),
        themes,
        |ui| {
            let mut v = 1usize;
            ui.add(Select::new("fruit", &mut v, &FRUITS).width(180.0));
            ui.add_space(160.0);
            ui.label("elsewhere");
        },
        |h| {
            h.get_by_role(egui::accesskit::Role::ComboBox).click();
            h.run();
            h.get_by_label("elsewhere").hover();
            h.run();
        },
    );
}

#[test]
fn select_open_inverted_snapshot() {
    select_open_in("select_open_inverted", MenuColor::Inverted);
}
