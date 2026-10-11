//! Spike (Phase 0): does `FontTweak::coords` change the weight egui renders?
//! One word at wght 700 must have clearly more ink than at 400.

use egui::epaint::text::VariationCoords;
use egui::{vec2, Color32, FontData, FontDefinitions, FontFamily, FontTweak, RichText};
use egui_kittest::Harness;
use std::sync::Arc;

const OXANIUM: &[u8] = include_bytes!("../assets/Oxanium-Variable.ttf");

fn defs(wght: f32) -> FontDefinitions {
    let mut d = FontDefinitions::default();
    let tweak = FontTweak {
        coords: VariationCoords::new([(b"wght", wght)]),
        ..Default::default()
    };
    d.font_data.insert(
        "probe".into(),
        Arc::new(FontData::from_static(OXANIUM).tweak(tweak)),
    );
    d.families
        .insert(FontFamily::Name("probe".into()), vec!["probe".into()]);
    d
}

/// Sum of darkness (255 - green) over the rendered image.
fn ink(wght: f32) -> u64 {
    let fam = FontFamily::Name("probe".into());
    let mut installed = false;
    let mut h = Harness::builder()
        .with_size(vec2(560.0, 90.0))
        .build_ui(move |ui| {
            if !installed {
                ui.ctx().set_fonts(defs(wght));
                installed = true;
            }
            ui.painter()
                .rect_filled(ui.ctx().viewport_rect(), 0.0, Color32::WHITE);
            if ui
                .ctx()
                .fonts(|f| f.definitions().families.contains_key(&fam))
            {
                ui.label(
                    RichText::new("Hamburgefonstiv")
                        .family(fam.clone())
                        .size(40.0)
                        .color(Color32::BLACK),
                );
            }
        });
    h.run();
    let img = h.render().expect("wgpu render");
    img.pixels().map(|p| u64::from(255 - p.0[1])).sum()
}

#[test]
fn wght_700_has_more_ink_than_400() {
    let (regular, medium, bold) = (ink(400.0), ink(500.0), ink(700.0));
    println!(
        "ink 400={regular} 500={medium} 700={bold} ratio700={:.3} ratio500={:.3}",
        bold as f64 / regular as f64,
        medium as f64 / regular as f64
    );
    assert!(regular > 0, "nothing rendered");
    assert!(
        bold as f64 > regular as f64 * 1.15,
        "wght 700 is not bolder: {bold} vs {regular}"
    );
    assert!(medium > regular, "wght 500 is not bolder than 400");
}
