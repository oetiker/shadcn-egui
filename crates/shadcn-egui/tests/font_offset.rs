//! Measures `FontFiles::y_offset_factor`: the factor that puts the ink
//! centre of capital letters on the vertical centre of a default button.
//! Run on demand, the value goes into `tools/fontgen/FONTS.tsv`:
//! `FONT=inter cargo test -p shadcn-egui --all-features --test font_offset -- --ignored --nocapture`

use egui::vec2;
use egui_kittest::Harness;
use shadcn_egui::components::button::Button;
use shadcn_egui::{fonts, FontFiles, FontName, Theme};

fn files(name: &str) -> &'static FontFiles {
    let id = FontName::from_id(name).unwrap_or_else(|| panic!("unknown font {name}"));
    fonts::files(id).unwrap_or_else(|| {
        panic!("font {name}: no crate, or its feature is off (run with --all-features)")
    })
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

/// Text snaps to whole pixels, so the offset is a step function of the
/// factor: flat runs (plateaus) about 0.07 wide, about 0.6 px apart. The
/// sweep walks every 0.01 in (-0.5, 0.5), takes the plateau whose offset is
/// closest to zero and returns its midpoint, so a small change in the
/// renderer does not push the value over a step edge. A best plateau at
/// either end of the range, or a residual of 0.5 px or more, fails.
#[test]
#[ignore = "measurement, run on demand with FONT=<id>"]
fn measure_y_offset_factor() {
    let name = std::env::var("FONT").expect("set FONT=<font id>");
    let font = files(&name);
    let steps: Vec<(i32, f32)> = (-49..=49)
        .map(|k| (k, offset(font, k as f32 / 100.0)))
        .collect();
    // Runs of equal offset: (first step, last step, offset).
    let mut runs: Vec<(i32, i32, f32)> = Vec::new();
    for &(k, off) in &steps {
        match runs.last_mut() {
            Some(run) if (run.2 - off).abs() < 0.05 => run.1 = k,
            _ => runs.push((k, k, off)),
        }
    }
    let &(first, last, _) = runs
        .iter()
        .min_by(|a, b| a.2.abs().total_cmp(&b.2.abs()))
        .expect("at least one step");
    assert!(
        first > -49 && last < 49,
        "{name}: best plateau {first}..={last} (hundredths) touches the sweep bound"
    );
    let factor = ((first + last) as f32 / 2.0).round() / 100.0;
    let residual = offset(font, factor);
    println!("{name}\t{factor}\t(residual {residual:.2} px)");
    assert!(residual.abs() < 0.5, "{name}: residual {residual:.2} px");
}
