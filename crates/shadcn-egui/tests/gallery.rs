//! The gallery example's screen: every component under the default preset.

mod common;

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotOptions, SnapshotResults};
use shadcn_egui::reference::{gallery_ui, GalleryState};
use shadcn_egui::{Mode, Preset, Theme};

/// Tall enough that nothing needs scrolling.
const FULL: egui::Vec2 = egui::vec2(1280.0, 1420.0);

/// The gallery in a harness, its page colour also under kittest's 8px outer
/// margin, so no band of another colour shows.
fn page(ui: &mut egui::Ui, state: &mut GalleryState) {
    // Painted first, under the panels, in the colour of the theme the last
    // pass applied (`gallery_ui` applies its preset on the first pass).
    let bg = Theme::current(ui.ctx()).palette.background;
    let page = ui.ctx().viewport_rect();
    ui.ctx()
        .layer_painter(egui::LayerId::background())
        .rect_filled(page, 0.0, bg);
    gallery_ui(ui, state);
}

fn harness(state: GalleryState, size: egui::Vec2) -> Harness<'static, GalleryState> {
    let mut h = Harness::builder()
        .with_size(size)
        .build_ui_state(page, state);
    h.run();
    h
}

/// The whole gallery, light and dark; and `docs/gallery-screen.png`, the README's view of the light one.
#[test]
fn gallery_default_preset() {
    let mut results = SnapshotResults::new();
    for (suffix, mode) in [("light", Mode::Light), ("dark", Mode::Dark)] {
        let mut h = harness(GalleryState::new(Preset::default(), mode), FULL);
        results.add(common::try_snapshot(&mut h, format!("gallery_{suffix}")));
    }
    let docs = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs");
    let mut h = harness(GalleryState::default(), egui::vec2(1280.0, 900.0));
    let options = SnapshotOptions::new().output_path(docs);
    results.add(common::try_snapshot_options(
        &mut h,
        "gallery-screen",
        &options,
    ));
}

fn stateful(state: GalleryState) -> Harness<'static, GalleryState> {
    harness(state, FULL)
}

#[test]
fn a_valid_code_sets_the_pickers() {
    let mut h = stateful(GalleryState::default());
    h.state_mut().code = "b2D0wqNxT".into();
    h.get_by_role_and_label(Role::Button, "Apply").click();
    h.run();
    let want = Preset::from_code("b2D0wqNxT").expect("README code decodes");
    assert_eq!(h.state().preset, want);
    assert!(h.query_by_label_contains("Not a preset code").is_none());
}

#[test]
fn an_invalid_code_shows_an_error_and_changes_nothing() {
    let mut h = stateful(GalleryState::default());
    h.state_mut().code = "x-1".into();
    h.get_by_role_and_label(Role::Button, "Apply").click();
    h.run();
    assert_eq!(h.state().preset, Preset::default());
    h.get_by_label_contains("Not a preset code: unknown preset code version 'x'");
}

#[test]
fn a_picker_change_rewrites_the_code() {
    let mut h = stateful(GalleryState::default());
    h.get_by_role_and_label(Role::ComboBox, "Style").click();
    h.run();
    h.get_by_role_and_label(Role::Button, "vega").click();
    h.run();
    let want = Preset::default().style(shadcn_egui::StyleName::Vega);
    assert_eq!(h.state().preset, want);
    assert_eq!(Some(h.state().code.clone()), want.to_code());
}

#[test]
fn the_dialog_closes_on_its_own_buttons() {
    let mut h = stateful(GalleryState::default());
    for close in ["Save changes", "Cancel"] {
        h.get_by_role_and_label(Role::Button, "Open dialog").click();
        h.run();
        h.get_by_label("Edit profile");
        h.get_by_role_and_label(Role::Button, close).click();
        h.run();
        assert!(h.query_by_label("Edit profile").is_none(), "{close}");
    }
}

/// The README's preset code, applied through the code field, restyles the
/// whole gallery: style, colours, fonts and the icon library.
#[cfg(all(feature = "fonts-all", feature = "icons-all"))]
#[test]
fn gallery_after_applying_a_code() {
    // Luma's larger controls need more height.
    let mut h = harness(GalleryState::default(), egui::vec2(1280.0, 1540.0));
    h.state_mut().code = "b2D0wqNxT".into();
    h.get_by_role_and_label(Role::Button, "Apply").click();
    h.run();
    // Leave the Apply button unhovered.
    h.hover_at(egui::pos2(1270.0, 1530.0));
    h.run();
    common::snapshot(&mut h, "gallery_code_b2D0wqNxT_light");
}
