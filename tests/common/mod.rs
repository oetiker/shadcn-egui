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
pub fn snapshot_all(name: &str, size: egui::Vec2, mut ui_fn: impl FnMut(&mut egui::Ui)) {
    // One result set for all harnesses, so a failing style does not abort the rest.
    let mut results = SnapshotResults::new();
    for (suffix, theme) in themes() {
        let bg = theme.palette.background;
        let mut h = Harness::builder().with_size(size).build_ui(|ui| {
            theme.apply(ui.ctx());
            egui::Frame::new().fill(bg).inner_margin(16).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                ui_fn(ui);
            });
        });
        h.run();
        h.snapshot(format!("{name}_{suffix}"));
        results.extend(h.take_snapshot_results());
    }
}
