//! Accessible roles egui's `WidgetType` lacks (conformance 8,
//! implementation notes 16), and states egui's `WidgetInfo` lacks
//! (`aria-invalid`).

use egui::accesskit::{Invalid, Role};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::components::button::Button;
use shadcn_egui::components::checkbox::Checkbox;
use shadcn_egui::components::input::Input;
use shadcn_egui::components::select::Select;
use shadcn_egui::components::shared::widget_info_with_role;
use shadcn_egui::components::switch::Switch;
use shadcn_egui::components::textarea::Textarea;
use shadcn_egui::Theme;

/// The role replaces the one `widget_info` sets from the `WidgetType`, on
/// a quiet pass and on a pass that reports an event (a click), while the
/// name stays the widget info's.
#[test]
fn widget_info_with_role_overrides_the_widget_type_role() {
    for (role, name) in [
        (Role::Alert, "Saved"),
        (Role::AlertDialog, "Delete file?"),
        (Role::Navigation, "pagination"),
        (Role::Group, "Text alignment"),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            let resp = ui.allocate_response(egui::vec2(120.0, 30.0), egui::Sense::click());
            widget_info_with_role(&resp, role, || {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
            });
        });
        harness.run();
        assert_eq!(
            harness
                .get_by_role(role)
                .accesskit_node()
                .label()
                .as_deref(),
            Some(name)
        );
        assert!(harness.query_by_role(Role::Button).is_none(), "{role:?}");
        harness.get_by_role(role).click();
        harness.run();
        assert_eq!(
            harness
                .get_by_role(role)
                .accesskit_node()
                .label()
                .as_deref(),
            Some(name)
        );
    }
}

/// One widget of [`aria_invalid_reaches_the_accessibility_node`], drawn
/// with `.invalid(..)`.
type Invalidable = fn(&mut egui::Ui, bool);

/// Upstream's `aria-invalid` is an attribute the browser reports to
/// assistive technology: every widget with an `.invalid(..)` setter says
/// so on its node, and only while it is set.
#[test]
fn aria_invalid_reaches_the_accessibility_node() {
    let widgets: [(&str, Role, Invalidable); 6] = [
        ("button", Role::Button, |ui, yes| {
            ui.add(Button::new("Save").invalid(yes));
        }),
        ("checkbox", Role::CheckBox, |ui, yes| {
            ui.add(Checkbox::new(&mut false).invalid(yes));
        }),
        ("switch", Role::CheckBox, |ui, yes| {
            ui.add(Switch::new(&mut false).invalid(yes));
        }),
        ("select", Role::ComboBox, |ui, yes| {
            ui.add(Select::new("fruit", &mut 0, &["Apple", "Pear"]).invalid(yes));
        }),
        ("input", Role::TextInput, |ui, yes| {
            ui.add(Input::new(&mut String::new()).invalid(yes));
        }),
        ("textarea", Role::MultilineTextInput, |ui, yes| {
            ui.add(Textarea::new(&mut String::new()).invalid(yes));
        }),
    ];
    for (name, role, widget) in widgets {
        for invalid in [false, true] {
            let mut harness = Harness::new_ui(move |ui| {
                Theme::vega().apply(ui.ctx());
                widget(ui, invalid);
            });
            harness.run();
            let node = harness.get_by_role(role);
            assert_eq!(
                node.accesskit_node().data().invalid(),
                invalid.then_some(Invalid::True),
                "{name}, invalid {invalid}"
            );
        }
    }
}
