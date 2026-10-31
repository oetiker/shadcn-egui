//! Switch: the shadcn-svelte demos that wrap the switch in a Field, as
//! snapshots. The bare switch's behaviour and its own snapshot are in
//! `components.rs`.

mod common;

use egui::{vec2, Rect, Ui};
use shadcn_egui::components::field::{field_label, Field, FieldGroup, FieldLabel};
use shadcn_egui::components::switch::{Switch, SwitchSize};

/// The demo's width class (`max-w-sm`, `max-w-[10rem]`): the room the
/// content gets.
fn width(ui: &mut Ui, w: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui(vec2(w, ui.available_height()), |ui| {
        ui.set_width(w);
        add(ui);
    });
}

/// switch-description: a label and a description beside the switch, in a
/// `max-w-sm` field.
#[test]
fn switch_description_snapshot() {
    common::snapshot_all("switch_description", vec2(420.0, 110.0), |ui| {
        width(ui, 384.0, |ui| {
            Field::new()
                .horizontal()
                .label("Share across devices")
                .description(
                    "Focus is shared across devices, and turns off when you leave the app.",
                )
                .show(ui, |ui| ui.add(Switch::new(&mut false)));
        });
    });
}

/// switch-disabled: `data-disabled` on a `w-fit` field and `disabled` on
/// the switch, both from a disabled `Ui`.
#[test]
fn switch_disabled_snapshot() {
    common::snapshot_all("switch_disabled", vec2(200.0, 60.0), |ui| {
        // `w-fit`: the field in unbounded room takes its content's width.
        let room = Rect::from_min_size(ui.cursor().min, vec2(f32::INFINITY, 40.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(room), |ui| {
            ui.add_enabled_ui(false, |ui| {
                Field::new().horizontal().show(ui, |ui| {
                    ui.add(Switch::new(&mut false));
                    field_label(ui, "Disabled");
                });
            });
        });
    });
}

/// switch-invalid: `data-invalid` on the field; the switch takes
/// `aria-invalid` from it.
#[test]
fn switch_invalid_snapshot() {
    common::snapshot_all("switch_invalid", vec2(420.0, 110.0), |ui| {
        width(ui, 384.0, |ui| {
            Field::new()
                .horizontal()
                .invalid(true)
                .label("Accept terms and conditions")
                .description("You must accept the terms and conditions to continue.")
                .show(ui, |ui| ui.add(Switch::new(&mut false)));
        });
    });
}

/// switch-sizes: the small and the default switch, each with its label,
/// in a `max-w-[10rem]` group.
#[test]
fn switch_sizes_snapshot() {
    common::snapshot_all("switch_sizes", vec2(200.0, 110.0), |ui| {
        width(ui, 160.0, |ui| {
            FieldGroup::new().show(ui, |ui| {
                for (size, name) in [(SwitchSize::Sm, "Small"), (SwitchSize::Default, "Default")] {
                    Field::new().horizontal().show(ui, |ui| {
                        ui.add(Switch::new(&mut false).size(size));
                        field_label(ui, name);
                    });
                }
            });
        });
    });
}

/// switch-choice-card: two fields wrapped in a FieldLabel, which upstream
/// draws as choice cards, in a `max-w-sm` group; the second is checked.
#[test]
fn switch_choice_card_snapshot() {
    common::snapshot_all("switch_choice_card", vec2(420.0, 280.0), |ui| {
        width(ui, 384.0, |ui| {
            FieldGroup::new().show(ui, |ui| {
                for (title, desc, mut on) in [
                    (
                        "Share across devices",
                        "Focus is shared across devices, and turns off when you leave the app.",
                        false,
                    ),
                    (
                        "Enable notifications",
                        "Receive notifications when focus mode is enabled or disabled.",
                        true,
                    ),
                ] {
                    FieldLabel::choice_card().show(ui, |ui| {
                        Field::new()
                            .horizontal()
                            .title(title)
                            .description(desc)
                            .show(ui, |ui| ui.add(Switch::new(&mut on)));
                    });
                }
            });
        });
    });
}
