//! Checkbox: the shadcn-svelte demos that wrap the checkbox in a Field, as
//! snapshots, and the plain Label beside it in checkbox-demo. The bare
//! checkbox's behaviour and its own snapshot are in `components.rs`.

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

/// Where the plain Labels of a test sit.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Place {
    /// After the checkbox in a field (checkbox-demo's first row).
    After,
    /// Before the checkbox in a field.
    Before,
    /// "First" before the checkbox, "Second" after it, in one field.
    Two,
    /// After the checkbox in a row outside any field.
    Outside,
}

/// The checkbox's value and the first label's rect, after the first frame.
fn plain_label_harness(place: Place) -> Harness<'static, (bool, Rect)> {
    // A control is unnamed on purpose: the test checks the names itself.
    Harness::builder()
        .with_accessibility_check(false)
        .build_ui_state(
            move |ui, (on, label): &mut (bool, Rect)| {
                Theme::vega().apply(ui.ctx());
                let mut row = |ui: &mut Ui| match place {
                    Place::After | Place::Outside => {
                        ui.add(Checkbox::new(on));
                        *label = ui.add(Label::new("First")).rect;
                    }
                    Place::Before => {
                        *label = ui.add(Label::new("First")).rect;
                        ui.add(Checkbox::new(on));
                    }
                    Place::Two => {
                        *label = ui.add(Label::new("First")).rect;
                        ui.add(Checkbox::new(on));
                        ui.add(Label::new("Second"));
                    }
                };
                if place == Place::Outside {
                    ui.horizontal(row);
                } else {
                    Field::new().horizontal().show(ui, |ui| row(ui));
                }
            },
            (false, Rect::NOTHING),
        )
}

/// The labels that name the checkbox, as their text.
fn checkbox_names(h: &Harness<'_, (bool, Rect)>) -> Vec<String> {
    let names: Vec<_> = h
        .get_by_role(Role::CheckBox)
        .accesskit_node()
        .labelled_by()
        .map(|n| n.id())
        .collect();
    ["First", "Second"]
        .into_iter()
        .filter(|text| {
            h.query_by_role_and_label(Role::Label, text)
                .is_some_and(|l| names.contains(&l.accesskit_node().id()))
        })
        .map(str::to_owned)
        .collect()
}

/// checkbox-demo's first row: a plain Label in a field (upstream's
/// `<Label for>`) names the checkbox, and a press on it toggles the
/// checkbox, as a field label does. Before the checkbox it names it from
/// the first frame on; of two labels only the first names it; outside a
/// field a Label stays plain text that takes no press.
#[test]
fn a_plain_label_in_a_field_names_and_toggles_the_checkbox() {
    for place in [Place::After, Place::Before, Place::Two, Place::Outside] {
        let mut h = plain_label_harness(place);
        if place == Place::Before {
            assert_eq!(checkbox_names(&h), ["First"], "{place:?}: first frame");
        }
        h.run();
        let named = checkbox_names(&h);
        if place == Place::Outside {
            assert!(named.is_empty(), "{place:?}: {named:?}");
        } else {
            assert_eq!(named, ["First"], "{place:?}");
        }
        let at = h.state().1.center();
        h.input_mut().events.extend(common::click(at));
        h.run();
        assert_eq!(h.state().0, place != Place::Outside, "{place:?}: a press");
    }
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
/// description, in a `w-72` group. The group's `mx-auto` centres it on
/// the docs page; here it starts at the page's edge.
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
/// checkbox, both from a disabled `Ui`, in a `w-56` group. The group's
/// `mx-auto` centres it on the docs page; here it starts at the page's
/// edge.
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
/// `aria-invalid` from it. In a `w-56` group, whose `mx-auto` centres it
/// on the docs page; here it starts at the page's edge.
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
