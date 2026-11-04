//! The gallery example's screen: every component under the default preset.

mod common;

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, SnapshotOptions, SnapshotResults};
use shadcn_egui::reference::{gallery_ui, GalleryState};
use shadcn_egui::{Preset, Theme};

/// Tall enough that nothing needs scrolling.
const FULL: egui::Vec2 = egui::vec2(1280.0, 3304.0);

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
    // Reduced motion: every transition jumps to its end state and the
    // spinner holds still, so the frame `run` settles on is the one shown.
    h.ctx.all_styles_mut(|s| s.animation_time = 0.0);
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

/// Switched from light to dark, the gallery draws as when started dark:
/// its text draws from the font atlas egui built for dark mode.
#[test]
fn gallery_switched_to_dark_draws_as_started_dark() {
    let mut h = harness(GalleryState::default(), FULL);
    h.state_mut().mode = egui::Theme::Dark;
    h.run();
    common::snapshot(&mut h, "gallery_dark");
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
    h.get_by_role_and_label(Role::ListBoxOption, "vega").click();
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
    let mut h = harness(GalleryState::default(), egui::vec2(1280.0, 3651.0));
    h.state_mut().code = "b2D0wqNxT".into();
    h.get_by_role_and_label(Role::Button, "Apply").click();
    h.run();
    // Leave the Apply button unhovered.
    h.hover_at(egui::pos2(1270.0, 3641.0));
    h.run();
    common::snapshot(&mut h, "gallery_code_b2D0wqNxT_light");
}

/// The passes of the gallery's first three frames and the bounds of the
/// widget named `label` in each, its preset and mode applied before the
/// first pass (the `gallery` example's app creator) or only by
/// `gallery_ui` in its first pass.
fn first_frames(before: bool, label: &str) -> Vec<(usize, Option<egui::Rect>)> {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
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
            let mut out = ctx.run_ui(input, |ui| gallery_ui(ui, &mut state));
            out.textures_delta.clear();
            (
                out.platform_output.num_completed_passes,
                common::bounds_by_label(&out, label),
            )
        })
        .collect()
}

/// Applied before the first pass, the gallery's first frame is in the
/// preset's fonts: the Apply button in frame 0 has the bounds it settles
/// on in frame 2. Applied only in the first pass, the fonts arrive a pass
/// late, in frame 0's second pass: frame 0 takes two passes either way
/// (its first is discarded), so under egui 0.37, which takes the new
/// fonts in the next pass, it ends settled too (under 0.34 it ended in egui's default
/// font). Either way the later frames take one pass each: the form rows
/// are Fields, which keep no row height from the previous pass.
#[test]
fn the_gallery_applied_before_the_first_pass_is_in_its_fonts_in_its_first_frame() {
    let early = first_frames(true, "Apply");
    assert!(early[0].1.is_some(), "the Apply button is in frame 0");
    assert_eq!(early[0].1, early[2].1, "applied early, frame 0 is settled");
    let late = first_frames(false, "Apply");
    assert_eq!(late[0].0, 2, "frame 0 takes two passes: {late:?}");
    assert_eq!(late[0].1, late[2].1, "applied late, frame 0 is settled");
    for frames in [early, late] {
        let passes: Vec<usize> = frames[1..].iter().map(|f| f.0).collect();
        assert_eq!(passes, [1, 1]);
    }
}
