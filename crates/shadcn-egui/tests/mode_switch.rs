//! Text after a change of the font atlas. egui builds a new atlas when
//! the text options change (a light/dark switch changes how glyph
//! coverage turns into alpha) and when the atlas is almost full; text the
//! crate laid out before must not keep the glyph positions of the old
//! atlas. A font change through the theme builds a new atlas too.

use egui::vec2;
use egui_kittest::Harness;
use shadcn_egui::components::button::Button;
use shadcn_egui::style::Style;
use shadcn_egui::{Palette, Theme};
use std::cell::Cell;
use std::rc::Rc;

const OXANIUM: &shadcn_egui::FontFiles = &shadcn_egui_font_oxanium::FONT;

fn vega() -> Theme {
    Theme::new(Style::vega(), Palette::light(), Palette::dark(), 10.0)
}

/// Text drawn by the crate in `start` mode, switched to `end` mode with
/// `ctx.set_theme` (as the gallery's switch does) when they differ.
fn rendered(start: egui::Theme, end: egui::Theme) -> image::RgbaImage {
    let theme = vega();
    let mut h = Harness::builder()
        .with_size(vec2(360.0, 80.0))
        .with_theme(start)
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            let bg = theme.palette_of(ui.ctx().theme()).background;
            ui.painter().rect_filled(ui.ctx().viewport_rect(), 0.0, bg);
            ui.add(Button::new("Hamburgefonstiv quick brown fox"));
        });
    for _ in 0..3 {
        h.step();
    }
    if start != end {
        h.ctx.set_theme(end);
        for _ in 0..3 {
            h.step();
        }
    }
    h.render().expect("wgpu render")
}

fn assert_same(a: &image::RgbaImage, b: &image::RgbaImage, what: &str) {
    assert_eq!(a.dimensions(), b.dimensions());
    let differ = a.pixels().zip(b.pixels()).filter(|(p, q)| p != q).count();
    assert_eq!(
        differ, 0,
        "{what}: {differ} pixels differ from a fresh render"
    );
}

/// After a switch to dark the text draws as in an app started dark.
#[test]
fn light_to_dark_draws_text_as_a_dark_start() {
    let switched = rendered(egui::Theme::Light, egui::Theme::Dark);
    let fresh = rendered(egui::Theme::Dark, egui::Theme::Dark);
    assert_same(&switched, &fresh, "light to dark");
}

/// After a switch to light the text draws as in an app started light.
#[test]
fn dark_to_light_draws_text_as_a_light_start() {
    let switched = rendered(egui::Theme::Dark, egui::Theme::Light);
    let fresh = rendered(egui::Theme::Light, egui::Theme::Light);
    assert_same(&switched, &fresh, "dark to light");
}

/// Text drawn by the crate after its theme took another body font and
/// then its own again, or (`round_trip` false) in its own font only.
fn rendered_after_font_change(round_trip: bool) -> image::RgbaImage {
    let other = Rc::new(Cell::new(round_trip));
    let set = other.clone();
    let mut h = Harness::builder()
        .with_size(vec2(360.0, 80.0))
        .build_ui(move |ui| {
            let theme = if other.get() {
                vega().fonts(OXANIUM, None)
            } else {
                vega()
            };
            theme.apply(ui.ctx());
            let bg = theme.palette_of(ui.ctx().theme()).background;
            ui.painter().rect_filled(ui.ctx().viewport_rect(), 0.0, bg);
            ui.add(Button::new("Hamburgefonstiv quick brown fox"));
        });
    for _ in 0..3 {
        h.step();
    }
    set.set(false);
    for _ in 0..3 {
        h.step();
    }
    h.render().expect("wgpu render")
}

/// Back in its own font after another, the text draws as if the font
/// never changed.
#[test]
fn a_font_round_trip_draws_text_as_before() {
    let changed = rendered_after_font_change(true);
    let fresh = rendered_after_font_change(false);
    assert_same(&changed, &fresh, "font round trip");
}
