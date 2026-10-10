mod common;

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use egui_shadcn::reference::{settings_ui, SettingsState};
use egui_shadcn::style::Style;
use egui_shadcn::{Palette, Theme};

fn state(tab: usize) -> SettingsState {
    SettingsState {
        tab,
        name: "Ada Lovelace".into(),
        email: "ada@example.com".into(),
        password: "hunter2".into(),
        marketing_emails: true,
        weekly_digest: true,
        ..Default::default()
    }
}

/// Render the settings screen with `tab` pre-selected, once per style and palette.
fn snapshot_tab(tab: usize, name: &str) {
    let mut state = state(tab);
    common::snapshot_all(name, egui::vec2(640.0, 840.0), move |ui| settings_ui(ui, &mut state));
}

#[test]
fn settings_account_tab() {
    snapshot_tab(0, "settings_account");
}

#[test]
fn settings_notifications_tab() {
    snapshot_tab(1, "settings_notifications");
}

#[test]
fn settings_components_tab() {
    snapshot_tab(2, "settings_components");
}

#[test]
fn settings_default_slider_is_50() {
    assert_eq!(SettingsState::default().slider, 50.0);
}

/// Render one tab and hand the harness to `check`.
fn with_tab(tab: usize, check: impl FnOnce(&Harness<'_>)) {
    let theme = Theme::new(Style::vega(), Palette::dark(), 10.0);
    let mut state = state(tab);
    let mut h = Harness::builder().with_size(egui::vec2(640.0, 840.0)).build_ui(move |ui| {
        theme.apply(ui.ctx());
        settings_ui(ui, &mut state);
    });
    h.run();
    check(&h);
}

#[test]
fn every_widget_on_every_tab_is_exposed_to_accessibility() {
    for tab in ["Account", "Notifications", "Components"] {
        with_tab(0, |h| {
            // The tab trigger; the card title may share its text.
            h.get_by_role_and_label(Role::Button, tab);
        });
    }
    with_tab(0, |h| {
        for l in ["Name", "Email", "Password", "Role", "Save", "Cancel"] {
            h.get_by_label(l);
        }
        h.get_by_role_and_label(Role::ComboBox, "Member"); // the role select's trigger
    });
    with_tab(1, |h| {
        for l in ["Marketing emails", "Security emails", "Weekly digest", "Active", "Beta", "Deprecated", "Optional"] {
            h.get_by_label(l);
        }
        // Switches and the checkbox carry no text, so they are found by role.
        assert_eq!(h.query_all_by_role(Role::CheckBox).count(), 3);
    });
    with_tab(2, |h| {
        for l in ["Default", "Secondary", "Destructive", "Outline", "Ghost", "Link", "Small", "Large", "Volume", "Select a region"] {
            assert!(h.query_all_by_label(l).count() >= 1, "{l} missing");
        }
        h.get_by_role(Role::Slider);
    });
}
