//! The gallery example's screen: every component under the default preset.

mod common;

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotOptions, SnapshotResults};
use shadcn_egui::reference::{gallery_ui, GalleryState};
use shadcn_egui::{Preset, Theme};

/// Tall enough that nothing needs scrolling.
const FULL: egui::Vec2 = egui::vec2(1280.0, 1940.0);

/// The gallery in a harness, its page colour also under kittest's 8px outer
/// margin, so no band of another colour shows.
fn page(ui: &mut egui::Ui, state: &mut GalleryState) {
    // Painted first, under the panels, in the colour of the theme the last
    // pass applied (`gallery_ui` applies its preset on the first pass).
    let bg = Theme::current(ui.ctx()).palette().background;
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
    for (suffix, mode) in [("light", egui::Theme::Light), ("dark", egui::Theme::Dark)] {
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
    let mut h = harness(GalleryState::default(), egui::vec2(1280.0, 2133.0));
    h.state_mut().code = "b2D0wqNxT".into();
    h.get_by_role_and_label(Role::Button, "Apply").click();
    h.run();
    // Leave the Apply button unhovered.
    h.hover_at(egui::pos2(1270.0, 2123.0));
    h.run();
    common::snapshot(&mut h, "gallery_code_b2D0wqNxT_light");
}

/// The passes of the gallery's first three frames, its preset and mode
/// applied before the first pass (the `gallery` example's app creator) or
/// only by `gallery_ui` in its first pass.
fn first_frames(before: bool) -> Vec<usize> {
    let ctx = egui::Context::default();
    let mut state = GalleryState::new(Preset::default(), egui::Theme::Light);
    if before {
        state.preset.apply(&ctx);
        ctx.set_theme(state.mode);
    }
    (0..3)
        .map(|_| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, FULL)),
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| gallery_ui(ui, &mut state));
            out.platform_output.num_completed_passes
        })
        .collect()
}

/// Applied before the first pass, the gallery's first frame is in the
/// preset's fonts and settles in that frame: the second frame takes one
/// pass. Applied only in the first pass, the fonts arrive a pass late and
/// the second frame still redoes a pass.
#[test]
fn the_gallery_applied_before_the_first_pass_settles_in_its_first_frame() {
    assert_eq!(first_frames(true)[1..], [1, 1]);
    assert_eq!(first_frames(false)[1..], [2, 1]);
}
