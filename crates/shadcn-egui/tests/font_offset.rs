//! Measures `FontFiles::y_offset_factor`: the factor that puts the ink
//! centre of capital letters on the vertical centre of a default button,
//! the way Oxanium's 0.18 was found by eye. Run on demand:
//! `FONT=inter cargo test -p shadcn-egui --test font_offset -- --ignored --nocapture`

use egui::vec2;
use egui_kittest::Harness;
use shadcn_egui::components::button::Button;
use shadcn_egui::{FontFiles, Theme};

fn files(name: &str) -> &'static FontFiles {
    match name {
        "inter" => &shadcn_egui_font_inter::FONT,
        "oxanium" => &shadcn_egui_font_oxanium::FONT,
        other => panic!("unknown font {other}"),
    }
}

/// Ink centre of "HHHH" minus the button centre, in px, at `factor`.
fn offset(font: &FontFiles, factor: f32) -> f32 {
    let probe: &'static FontFiles = Box::leak(Box::new(FontFiles {
        y_offset_factor: factor,
        ..font.clone()
    }));
    let theme = Theme::vega_light().fonts(probe, None);
    let rect = std::cell::Cell::new(egui::Rect::NOTHING);
    let mut applied = false;
    let mut h = Harness::builder()
        .with_size(vec2(200.0, 80.0))
        .build_ui(|ui| {
            if !applied {
                theme.apply(ui.ctx());
                applied = true;
            }
            rect.set(ui.add(Button::new("HHHH")).rect);
        });
    for _ in 0..3 {
        h.step();
    }
    let img = h.render().expect("wgpu render");
    let r = rect.get();
    // Light text on the dark primary fill: ink is brightness above the fill.
    let fill = f64::from(img.get_pixel(r.left() as u32 + 2, r.center().y as u32).0[1]);
    let (mut sum, mut weight) = (0.0f64, 0.0f64);
    for y in r.top().ceil() as u32..r.bottom().floor() as u32 {
        for x in r.left() as u32 + 4..r.right() as u32 - 4 {
            let w = (f64::from(img.get_pixel(x, y).0[1]) - fill).max(0.0);
            sum += w * (f64::from(y) + 0.5);
            weight += w;
        }
    }
    assert!(weight > 0.0, "no text ink found");
    (sum / weight) as f32 - r.center().y
}

#[test]
#[ignore = "measurement, run on demand with FONT=<id>"]
fn measure_y_offset_factor() {
    let name = std::env::var("FONT").expect("set FONT=<font id>");
    let font = files(&name);
    let (mut lo, mut hi) = (-0.5f32, 0.5f32);
    for _ in 0..12 {
        let mid = (lo + hi) / 2.0;
        if offset(font, mid) > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let factor = ((lo + hi) / 2.0 * 100.0).round() / 100.0;
    println!(
        "{name}\t{factor}\t(residual {:.2} px)",
        offset(font, factor)
    );
}
