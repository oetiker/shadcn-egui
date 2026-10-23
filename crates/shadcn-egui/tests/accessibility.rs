//! Accessible roles egui's `WidgetType` lacks (conformance 8,
//! implementation notes 16).

use egui::accesskit::Role;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::components::shared::widget_info_with_role;

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
