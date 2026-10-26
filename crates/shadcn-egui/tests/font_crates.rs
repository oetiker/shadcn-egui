//! One Button snapshot per font crate (`font_<id>.png`, vega light): each
//! font installs, its medium label is heavier than the body text, and the
//! text sits on the baseline the text path gives it (`y_offset_factor` 0).
#![cfg(feature = "fonts-all")]

mod common;

use shadcn_egui::components::button::Button;
use shadcn_egui::{fonts, FontName, Preset, StyleName, Theme};

#[test]
fn every_font_crate_renders_a_button() {
    let themes: Vec<(&'static str, Theme)> = FontName::ALL
        .iter()
        .copied()
        .filter(|n| fonts::files(*n).is_some())
        .map(|n| {
            (
                n.id(),
                Preset::default().style(StyleName::Vega).font(n).theme(),
            )
        })
        .collect();
    assert!(themes.len() > 2, "fonts-all enables the font crates");
    common::snapshot_with_themes(
        "font",
        egui::vec2(360.0, 96.0),
        themes,
        |ui| {
            ui.horizontal(|ui| {
                ui.add(Button::new("Save changes"));
                ui.label("Body text Hg 0123");
            });
        },
        |_| {},
    );
}
