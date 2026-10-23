mod common;

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotOptions};
use shadcn_egui::reference::{settings_ui, SettingsState};
use shadcn_egui::style::Style;
use shadcn_egui::{Palette, Theme};

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

/// The fixture height: the tallest tab at the phone width fits.
const HEIGHT: f32 = 1000.0;

/// Render the settings screen with `tab` pre-selected, once per style and
/// palette, at the phone and the desktop width.
fn snapshot_tab(tab: usize, name: &str) {
    let mut state = state(tab);
    common::snapshot_widths(name, HEIGHT, common::snapshot_themes(), move |ui| {
        settings_ui(ui, &mut state)
    });
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

/// `docs/settings-screen.png`, the README's first image: the Account tab as
/// `cargo run --example settings` starts (vega, dark, radius 10), the
/// desktop render beside the phone render, as `settings_account` renders
/// them.
#[test]
fn readme_settings_screen() {
    let theme = Theme::new(Style::vega(), Palette::dark(), 10.0);
    let mut state = state(0);
    let mut render = |width: f32| {
        let mut h = common::page_harness(egui::vec2(width, HEIGHT), theme.clone(), |ui| {
            settings_ui(ui, &mut state)
        });
        h.render().expect("render")
    };
    let (desktop, phone) = (render(common::DESKTOP), render(common::PHONE));
    let gap = 32;
    let page = *desktop.get_pixel(0, 0);
    let mut both = image::RgbaImage::from_pixel(
        desktop.width() + gap + phone.width(),
        desktop.height().max(phone.height()),
        page,
    );
    image::imageops::replace(&mut both, &desktop, 0, 0);
    image::imageops::replace(&mut both, &phone, i64::from(desktop.width() + gap), 0);
    let docs = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs");
    common::image_snapshot_options(
        &both,
        "settings-screen",
        &SnapshotOptions::new().output_path(docs),
    );
}

#[test]
fn settings_default_slider_is_50() {
    assert_eq!(SettingsState::default().slider, 50.0);
}

/// Render one tab and hand the harness to `check`.
fn with_tab(tab: usize, check: impl FnOnce(&Harness<'_>)) {
    let theme = Theme::new(Style::vega(), Palette::dark(), 10.0);
    let mut state = state(tab);
    let mut h = Harness::builder()
        .with_size(egui::vec2(common::DESKTOP, HEIGHT))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            settings_ui(ui, &mut state);
        });
    h.run();
    check(&h);
}

#[test]
fn every_widget_on_every_tab_is_exposed_to_accessibility() {
    with_tab(0, |h| {
        // The tab triggers; a card title may share their text.
        for tab in ["Account", "Notifications", "Components"] {
            h.get_by_role_and_label(Role::Button, tab);
        }
        // Each field's label names its control (`labelled_by`).
        h.get_by_role_and_label(Role::TextInput, "Name");
        h.get_by_role_and_label(Role::TextInput, "Email");
        h.get_by_role_and_label(Role::PasswordInput, "Password");
        h.get_by_role_and_label(Role::ComboBox, "Role");
        for l in ["Save", "Cancel"] {
            h.get_by_role_and_label(Role::Button, l);
        }
    });
    with_tab(1, |h| {
        // Switches and the checkbox carry no text: their row label names them.
        for l in ["Marketing emails", "Security emails", "Weekly digest"] {
            h.get_by_role_and_label(Role::CheckBox, l);
        }
        for l in ["Active", "Beta", "Deprecated", "Optional"] {
            h.get_by_role_and_label(Role::Label, l);
        }
    });
    with_tab(2, |h| {
        // "Default" is a variant and a size button.
        assert_eq!(
            h.query_all_by_role_and_label(Role::Button, "Default")
                .count(),
            2
        );
        for l in [
            "Secondary",
            "Destructive",
            "Outline",
            "Ghost",
            "Link",
            "Small",
            "Large",
        ] {
            h.get_by_role_and_label(Role::Button, l);
        }
        for l in ["Default", "Secondary", "Destructive", "Outline"] {
            h.get_by_role_and_label(Role::Label, l);
        }
        h.get_by_role_and_label(Role::Slider, "Volume");
        h.get_by_role_and_label(Role::ComboBox, "Region");
    });
}
