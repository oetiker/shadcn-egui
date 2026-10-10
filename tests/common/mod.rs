//! Shared test helpers. Each integration test file that needs them declares
//! `mod common;`.
#![allow(dead_code)]

use egui_kittest::{Harness, SnapshotResults};
use egui_shadcn::style::Style;
use egui_shadcn::{Palette, Theme};

pub fn themes() -> Vec<(&'static str, Theme)> {
    vec![
        ("vega_dark", Theme::new(Style::vega(), Palette::dark(), 10.0)),
        ("vega_light", Theme::new(Style::vega(), Palette::light(), 10.0)),
        ("nova_dark", Theme::new(Style::nova(), Palette::dark(), 10.0)),
        ("nova_light", Theme::new(Style::nova(), Palette::light(), 10.0)),
    ]
}

/// One snapshot per style x {dark, light}: `{name}_{vega_dark|...}.png`.
pub fn snapshot_all(name: &str, size: egui::Vec2, ui_fn: impl FnMut(&mut egui::Ui)) {
    snapshot_all_with(name, size, ui_fn, |_| {});
}

/// Like [`snapshot_all`], with `prepare` run on each harness after the first
/// frame (hover a trigger, press a key) and before the snapshot.
pub fn snapshot_all_with(
    name: &str,
    size: egui::Vec2,
    mut ui_fn: impl FnMut(&mut egui::Ui),
    mut prepare: impl FnMut(&mut Harness<'_>),
) {
    // One result set for all harnesses, so a failing style does not abort the rest.
    let mut results = SnapshotResults::new();
    for (suffix, theme) in themes() {
        let bg = theme.palette.background;
        let mut h = Harness::builder().with_size(size).build_ui(|ui| {
            theme.apply(ui.ctx());
            // kittest wraps the ui in an 8px outer margin; paint the page
            // colour under all of it, so no band of another colour shows.
            let page = ui.ctx().viewport_rect();
            ui.ctx().layer_painter(egui::LayerId::background()).rect_filled(page, 0.0, bg);
            egui::Frame::new().fill(bg).inner_margin(16).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                ui_fn(ui);
            });
        });
        h.run();
        prepare(&mut h);
        h.snapshot(format!("{name}_{suffix}"));
        results.extend(h.take_snapshot_results());
    }
}

/// The rect of the widget `add` places, under `theme`, and the output of
/// the frame that painted it. Runs two frames: the fonts `Theme::apply`
/// installs reach the atlas only on the second, and focus requested in the
/// first frame shows in the second.
pub fn measure_output(theme: &Theme, mut add: impl FnMut(&mut egui::Ui) -> egui::Response) -> (egui::Rect, egui::FullOutput) {
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    let mut rect = egui::Rect::NOTHING;
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| rect = add(ui).rect);
    let out = ctx.run_ui(egui::RawInput::default(), |ui| rect = add(ui).rect);
    (rect, out)
}

/// The rect of the widget `add` places, under `theme` (see [`measure_output`]).
pub fn measure(theme: &Theme, add: impl FnMut(&mut egui::Ui) -> egui::Response) -> egui::Rect {
    measure_output(theme, add).0
}
