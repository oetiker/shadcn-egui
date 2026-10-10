mod common;

use egui::vec2;
use egui::Key;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use egui_shadcn::components::button::{Button, ButtonVariant};
use egui_shadcn::components::dialog::{self, Dialog};
use egui_shadcn::components::popover::{popover_header, Popover};
use egui_shadcn::components::select::{Select, SelectSize};
use egui_shadcn::components::tooltip::tooltip;
use egui_shadcn::Theme;

/// A dialog harness: `open` starts true; the body has a button that counts clicks.
fn dialog_harness(open: bool) -> Harness<'static, (bool, u32)> {
    Harness::builder().with_size(vec2(640.0, 360.0)).build_ui_state(
        |ui, (open, clicks): &mut (bool, u32)| {
            Theme::dark().apply(ui.ctx());
            ui.label("page beneath");
            Dialog::new("confirm", "Delete session?")
                .description("The session and its files are removed.")
                .show(ui.ctx(), open, |ui| {
                    dialog::footer(ui, |ui| {
                        if ui.add(Button::new("Delete").variant(ButtonVariant::Destructive)).clicked() {
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

#[test]
fn dialog_snapshot() {
    let mut h = dialog_harness(true);
    h.run();
    h.snapshot("dialog");
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
    Harness::builder().with_size(vec2(420.0, 240.0)).build_ui(|ui| {
        Theme::dark().apply(ui.ctx());
        popover_trigger(ui);
    })
}

#[test]
fn popover_toggles_and_closes_outside() {
    let mut h = popover_harness();
    h.run();
    assert!(h.query_by_label("Set the dimensions for the layer.").is_none());
    h.get_by_label("Dimensions").click();
    h.run();
    h.get_by_label("Set the dimensions for the layer.");
    h.get_by_label("elsewhere").click();
    h.run();
    assert!(h.query_by_label("Set the dimensions for the layer.").is_none(), "click outside closes");
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
    assert!(h.query_by_label("Set the dimensions for the layer.").is_none(), "Esc closes");
    h.get_by_label("Dimensions").click();
    h.run();
    h.get_by_label("Set the dimensions for the layer.");
    h.get_by_label("Dimensions").click();
    h.run();
    assert!(h.query_by_label("Set the dimensions for the layer.").is_none(), "the trigger toggles");
}

#[test]
fn popover_box_is_w_72_and_content_gets_the_padding() {
    for (name, theme) in common::themes() {
        let inner = std::cell::Cell::new(0.0_f32);
        let mut h = Harness::builder().with_size(vec2(420.0, 240.0)).build_ui(|ui| {
            theme.apply(ui.ctx());
            let trigger = ui.add(Button::new("Dimensions"));
            Popover::new(&trigger).show(|ui| inner.set(ui.available_width()));
        });
        h.run();
        h.get_by_label("Dimensions").click();
        h.run();
        let pad = theme.style.popover_content.pad_left.base.expect("fixed padding");
        assert_eq!(inner.get() + 2.0 * pad, 288.0, "{name}: w-72 includes the padding");
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
        tooltip(ui.add(Button::new("Hover me").variant(ButtonVariant::Outline)), "Add to library");
    });
}

#[test]
fn tooltip_shows_on_hover() {
    let mut h = Harness::builder().with_size(vec2(240.0, 120.0)).build_ui(|ui| {
        Theme::dark().apply(ui.ctx());
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
        let mut h = Harness::builder().with_size(vec2(240.0, 120.0)).build_ui(|ui| {
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

#[test]
fn tooltip_snapshot() {
    common::snapshot_all_with("tooltip", vec2(240.0, 140.0), tooltip_trigger, |h| {
        h.get_by_label("Hover me").hover();
        h.run();
    });
}

const FRUITS: [&str; 4] = ["Apple", "Banana", "Blueberry", "Cherry"];

fn select_harness(start: usize) -> Harness<'static, usize> {
    Harness::builder().with_size(vec2(320.0, 260.0)).build_ui_state(
        |ui, value: &mut usize| {
            Theme::dark().apply(ui.ctx());
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
    assert!(h.query_by_label("Cherry").is_none(), "closed list shows no options");
    open_select(&mut h);
    h.get_by_label("Cherry").click();
    h.run();
    assert_eq!(*h.state(), 3);
    assert!(h.query_by_label("Banana").is_none(), "choosing closes the list");
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
    assert!(h.query_by_label("Cherry").is_none(), "Enter closes the list");
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
    assert!(h.query_by_label("Cherry").is_none(), "a click outside closes");
}

#[test]
fn select_typing_jumps_to_the_next_option_with_that_letter() {
    let mut h = select_harness(0);
    h.run();
    open_select(&mut h);
    h.get_by_role(egui::accesskit::Role::ComboBox).type_text("b");
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(*h.state(), 1, "first option after the highlight starting with b");
}

#[test]
fn select_trigger_reports_the_current_text() {
    let mut h = select_harness(2);
    h.run();
    assert_eq!(h.get_by_role(egui::accesskit::Role::ComboBox).accesskit_node().label().as_deref(), Some("Blueberry"));
    let mut h = select_harness(9);
    h.run();
    assert_eq!(h.get_by_role(egui::accesskit::Role::ComboBox).accesskit_node().label().as_deref(), Some("Pick a fruit"));
}

#[test]
fn select_disabled_does_not_open() {
    let mut h = Harness::builder().with_size(vec2(320.0, 260.0)).build_ui_state(
        |ui, value: &mut usize| {
            Theme::dark().apply(ui.ctx());
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
        let (default_h, sm_h) = if name.starts_with("nova") { (32.0, 28.0) } else { (36.0, 32.0) };
        let mut v = 0usize;
        let mut h = |size| common::measure(&theme, |ui| ui.add(Select::new("s", &mut v, &FRUITS).size(size))).height();
        assert_eq!(h(SelectSize::Default), default_h, "{name}");
        assert_eq!(h(SelectSize::Sm), sm_h, "{name}");
    }
}

fn select_triggers(ui: &mut egui::Ui) {
    let (mut a, mut b, mut c, mut d, mut e) = (9usize, 1usize, 1usize, 0usize, 2usize);
    ui.spacing_mut().item_spacing.y = 12.0;
    ui.add(Select::new("a", &mut a, &FRUITS).placeholder("Pick a fruit").width(180.0));
    ui.add(Select::new("b", &mut b, &FRUITS).width(180.0));
    ui.add(Select::new("c", &mut c, &FRUITS).invalid(true).width(180.0));
    ui.add(Select::new("d", &mut d, &FRUITS).enabled(false).width(180.0));
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
        let mut h = Harness::builder().with_size(vec2(320.0, 260.0)).build_ui_state(
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
