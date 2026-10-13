//! Shared test helpers. Each integration test file that needs them declares
//! `mod common;`.
#![allow(dead_code)]

pub mod logcap;

use egui_kittest::{Harness, SnapshotResults};
use shadcn_egui::style::Style;
use shadcn_egui::{Palette, Theme};

pub fn themes() -> Vec<(&'static str, Theme)> {
    vec![
        (
            "vega_dark",
            Theme::new(Style::vega(), Palette::dark(), 10.0),
        ),
        (
            "vega_light",
            Theme::new(Style::vega(), Palette::light(), 10.0),
        ),
        (
            "nova_dark",
            Theme::new(Style::nova(), Palette::dark(), 10.0),
        ),
        (
            "nova_light",
            Theme::new(Style::nova(), Palette::light(), 10.0),
        ),
    ]
}

/// The seven exported styles, dark and light: what every component snapshot
/// covers. Measurement tests use [`themes`] (vega and nova) instead.
pub fn snapshot_themes() -> Vec<(&'static str, Theme)> {
    type Make = fn() -> Style;
    const STYLES: [(&str, &str, Make); 7] = [
        ("vega_dark", "vega_light", Style::vega),
        ("nova_dark", "nova_light", Style::nova),
        ("maia_dark", "maia_light", Style::maia),
        ("lyra_dark", "lyra_light", Style::lyra),
        ("mira_dark", "mira_light", Style::mira),
        ("luma_dark", "luma_light", Style::luma),
        ("rhea_dark", "rhea_light", Style::rhea),
    ];
    STYLES
        .iter()
        .flat_map(|(dark, light, make)| {
            [
                (*dark, Theme::new(make(), Palette::dark(), 10.0)),
                (*light, Theme::new(make(), Palette::light(), 10.0)),
            ]
        })
        .collect()
}

/// One snapshot per exported style x {dark, light}: `{name}_{style}_{mode}.png`.
pub fn snapshot_all(name: &str, size: egui::Vec2, ui_fn: impl FnMut(&mut egui::Ui)) {
    snapshot_all_with(name, size, ui_fn, |_| {});
}

/// Like [`snapshot_all`], with `prepare` run on each harness after the first
/// frame (hover a trigger, press a key) and before the snapshot.
pub fn snapshot_all_with(
    name: &str,
    size: egui::Vec2,
    ui_fn: impl FnMut(&mut egui::Ui),
    prepare: impl FnMut(&mut Harness<'_>),
) {
    snapshot_with_themes(name, size, snapshot_themes(), ui_fn, prepare);
}

/// `ui_fn` on a page of `theme`'s background colour, after one run.
pub fn page_harness<'a>(
    size: egui::Vec2,
    theme: Theme,
    mut ui_fn: impl FnMut(&mut egui::Ui) + 'a,
) -> Harness<'a> {
    let bg = theme.palette.background;
    let mut h = Harness::builder().with_size(size).build_ui(move |ui| {
        theme.apply(ui.ctx());
        // kittest wraps the ui in an 8px outer margin; paint the page
        // colour under all of it, so no band of another colour shows.
        let page = ui.ctx().viewport_rect();
        ui.ctx()
            .layer_painter(egui::LayerId::background())
            .rect_filled(page, 0.0, bg);
        egui::Frame::new().fill(bg).inner_margin(16).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui_fn(ui);
        });
    });
    h.run();
    h
}

/// One snapshot per `(suffix, theme)`: `{name}_{suffix}.png`.
pub fn snapshot_with_themes(
    name: &str,
    size: egui::Vec2,
    themes: Vec<(&'static str, Theme)>,
    mut ui_fn: impl FnMut(&mut egui::Ui),
    mut prepare: impl FnMut(&mut Harness<'_>),
) {
    // One result set for all harnesses, so a failing style does not abort the rest.
    let mut results = SnapshotResults::new();
    for (suffix, theme) in themes {
        let mut h = page_harness(size, theme, &mut ui_fn);
        prepare(&mut h);
        h.snapshot(format!("{name}_{suffix}"));
        results.extend(h.take_snapshot_results());
    }
}

/// The rect of the widget `add` places, under `theme`, and the output of
/// the frame that painted it. Runs two frames: the fonts `Theme::apply`
/// installs reach the atlas only on the second, and focus requested in the
/// first frame shows in the second.
pub fn measure_output(
    theme: &Theme,
    mut add: impl FnMut(&mut egui::Ui) -> egui::Response,
) -> (egui::Rect, egui::FullOutput) {
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
