//! Checkbox: the shadcn-svelte demos that wrap the checkbox in a Field, as
//! snapshots, and the plain Label beside it in checkbox-demo. The bare checkbox's behaviour and its own snapshot are in
//! `components.rs`.

mod common;

use egui::accesskit::Role;
use egui::{vec2, Rect, Ui};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::components::checkbox::Checkbox;
use shadcn_egui::components::field::{
    field_content, field_description, field_label, field_title, Field, FieldGroup,
    FieldGroupVariant, FieldLabel, FieldLegend, FieldLegendVariant, FieldSet, FieldVariant,
};
use shadcn_egui::components::label::Label;
use shadcn_egui::style::{AppVariant, Weight};
use shadcn_egui::Theme;

const NORMAL: AppVariant<FieldVariant> = FieldVariant::app("font-normal");
const GAP_3: AppVariant<FieldGroupVariant> = FieldGroupVariant::app("gap-3");

/// The demos' classes as app variants (axiom 8): `class="font-normal"` on
/// a field label, `class="gap-3"` on a field group.
fn snapshot(name: &str, size: egui::Vec2, ui_fn: impl FnMut(&mut Ui)) {
    let themes = common::snapshot_themes()
        .into_iter()
        .map(|(n, t, m)| {
            let t = t
                .variant(NORMAL, FieldVariant::Default, |r, _| {
                    r.field_label.weight.set(Weight::Normal)
                })
                .variant(GAP_3, FieldGroupVariant::Default, |r, _| {
                    r.field_group.gap.set(12.0)
                });
            (n, t, m)
        })
        .collect();
    common::snapshot_with_themes(name, size, themes, ui_fn, |_| {});
}

/// The demo's width class (`w-56`, `max-w-sm`): the room the content gets.
fn width(ui: &mut Ui, w: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui(vec2(w, ui.available_height()), |ui| {
        ui.set_width(w);
        add(ui);
    });
}

/// checkbox-demo's first row: a plain Label after the checkbox in a
/// field (upstream's `<Label for>`) names the checkbox, and a press on it
/// toggles the checkbox, as a field label does.
#[test]
fn a_plain_label_in_a_field_names_and_toggles_the_checkbox() {
    let mut h = Harness::new_ui_state(
        |ui, (on, label): &mut (bool, Rect)| {
            Theme::vega().apply(ui.ctx());
            Field::new().horizontal().show(ui, |ui| {
                ui.add(Checkbox::new(on));
                *label = ui.add(Label::new("Accept terms and conditions")).rect;
            });
        },
        (false, Rect::NOTHING),
    );
    h.run();
    h.run();
    let label = h
        .get_by_role_and_label(Role::Label, "Accept terms and conditions")
        .accesskit_node()
        .id();
    let named: Vec<_> = h
        .get_by_role(Role::CheckBox)
        .accesskit_node()
        .labelled_by()
        .map(|n| n.id())
        .collect();
    assert_eq!(named, vec![label], "the label names the checkbox");
    let at = h.state().1.center();
    h.input_mut().events.extend(common::click(at));
    h.run();
    assert!(h.state().0, "a press on the label toggles the checkbox");
}

/// checkbox-basic: a checkbox and its label in a `w-56` group. The
/// group's `mx-auto` centres it on the docs page; here it starts at the
/// page's edge.
#[test]
fn checkbox_basic_snapshot() {
    snapshot("checkbox_basic", vec2(260.0, 70.0), |ui| {
        width(ui, 224.0, |ui| {
            FieldGroup::new().show(ui, |ui| {
                Field::new().horizontal().show(ui, |ui| {
                    ui.add(Checkbox::new(&mut false));
                    field_label(ui, "Accept terms and conditions");
                });
            });
        });
    });
}

/// checkbox-demo: a plain Label, a FieldContent with a description, a
/// disabled field (disabled through the `Ui`), and a field wrapped in a
/// FieldLabel, which upstream draws as a choice card.
#[test]
fn checkbox_demo_snapshot() {
    snapshot("checkbox_demo", vec2(420.0, 330.0), |ui| {
        width(ui, 384.0, |ui| {
            FieldGroup::new().show(ui, |ui| {
                Field::new().horizontal().show(ui, |ui| {
                    ui.add(Checkbox::new(&mut false));
                    ui.add(Label::new("Accept terms and conditions"));
                });
                Field::new().horizontal().show(ui, |ui| {
                    ui.add(Checkbox::new(&mut true));
                    field_content(ui, |ui| {
                        field_label(ui, "Accept terms and conditions");
                        field_description(ui, "By clicking this checkbox, you agree to the terms.");
                    });
                });
                ui.add_enabled_ui(false, |ui| {
                    Field::new().horizontal().show(ui, |ui| {
                        ui.add(Checkbox::new(&mut false));
                        field_label(ui, "Enable notifications");
                    });
                });
                FieldLabel::choice_card().show(ui, |ui| {
                    Field::new().horizontal().show(ui, |ui| {
                        ui.add(Checkbox::new(&mut false));
                        field_content(ui, |ui| {
                            field_title(ui, "Enable notifications");
                            field_description(
                                ui,
                                "You can enable or disable notifications at any time.",
                            );
                        });
                    });
                });
            });
        });
    });
}

/// checkbox-description: a checked checkbox beside a label and a
/// description, in a `w-72` group.
#[test]
fn checkbox_description_snapshot() {
    snapshot("checkbox_description", vec2(320.0, 100.0), |ui| {
        width(ui, 288.0, |ui| {
            FieldGroup::new().show(ui, |ui| {
                Field::new().horizontal().show(ui, |ui| {
                    ui.add(Checkbox::new(&mut true));
                    field_content(ui, |ui| {
                        field_label(ui, "Accept terms and conditions");
                        field_description(
                            ui,
                            "By clicking this checkbox, you agree to the terms and conditions.",
                        );
                    });
                });
            });
        });
    });
}

/// checkbox-disabled: `data-disabled` on the field and `disabled` on the
/// checkbox, both from a disabled `Ui`.
#[test]
fn checkbox_disabled_snapshot() {
    snapshot("checkbox_disabled", vec2(260.0, 70.0), |ui| {
        width(ui, 224.0, |ui| {
            ui.add_enabled_ui(false, |ui| {
                FieldGroup::new().show(ui, |ui| {
                    Field::new().horizontal().show(ui, |ui| {
                        ui.add(Checkbox::new(&mut false));
                        field_label(ui, "Enable notifications");
                    });
                });
            });
        });
    });
}

/// checkbox-invalid: `data-invalid` on the field; the checkbox takes
/// `aria-invalid` from it.
#[test]
fn checkbox_invalid_snapshot() {
    snapshot("checkbox_invalid", vec2(260.0, 70.0), |ui| {
        width(ui, 224.0, |ui| {
            FieldGroup::new().show(ui, |ui| {
                Field::new().horizontal().invalid(true).show(ui, |ui| {
                    ui.add(Checkbox::new(&mut false));
                    field_label(ui, "Accept terms and conditions");
                });
            });
        });
    });
}

/// checkbox-group: a set with a label-variant legend and a description
/// over a `gap-3` group of `font-normal` labels, both app variants. The
/// set has no width class; it gets 320 points here.
#[test]
fn checkbox_group_snapshot() {
    snapshot("checkbox_group", vec2(360.0, 260.0), |ui| {
        width(ui, 320.0, |ui| {
            FieldSet::new().show(ui, |ui| {
                ui.add(
                    FieldLegend::new("Show these items on the desktop:")
                        .variant(FieldLegendVariant::Label),
                );
                field_description(ui, "Select the items you want to show on the desktop.");
                FieldGroup::new().variant(GAP_3).show(ui, |ui| {
                    for (name, mut on) in [
                        ("Hard disks", true),
                        ("External disks", true),
                        ("CDs, DVDs, and iPods", false),
                        ("Connected servers", false),
                    ] {
                        Field::new().horizontal().variant(NORMAL).show(ui, |ui| {
                            ui.add(Checkbox::new(&mut on));
                            field_label(ui, name);
                        });
                    }
                });
            });
        });
    });
}
