mod common;

use egui::vec2;
use egui::Key;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use egui_shadcn::components::button::{Button, ButtonVariant};
use egui_shadcn::components::dialog::{self, Dialog};
use egui_shadcn::components::popover::Popover;
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

fn popover_harness() -> Harness<'static> {
    Harness::builder().with_size(vec2(420.0, 240.0)).build_ui(|ui| {
        Theme::dark().apply(ui.ctx());
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            let trigger = ui.add(Button::new("Dimensions").variant(ButtonVariant::Outline));
            Popover::new(&trigger).show(|ui| {
                ui.label("Set the dimensions for the layer.");
            });
            ui.add_space(120.0);
            ui.label("elsewhere");
        });
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
fn popover_snapshot() {
    let mut h = popover_harness();
    h.run();
    h.get_by_label("Dimensions").click();
    h.run();
    h.snapshot("popover");
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
