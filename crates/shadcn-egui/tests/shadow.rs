//! `box-shadow` pixels against Chrome. The reference rows come from
//! headless Chrome rendering white boxes with Tailwind v4's shadows on a
//! white page at a device pixel ratio of 1; each value is 255 minus the
//! red channel. Both sides sit within about one level of the exact
//! Gaussian: Chrome a little under it, egui through its 8-bit vertex
//! colours (one rounding per layer and one in the output), so a pixel may
//! differ by 2.

use egui::{pos2, vec2, Color32, Rect};
use egui_kittest::Harness;
use shadcn_egui::components::shared::RecipeBox;
use shadcn_egui::style::{Radius, Recipe, ShadowLevel, State};
use shadcn_egui::Theme;

/// (level, border box, radius) as the reference page lays them out.
const BOXES: [(ShadowLevel, [f32; 4], f32); 4] = [
    (ShadowLevel::Md, [100.0, 60.0, 300.0, 180.0], 8.0),
    (ShadowLevel::Xl2, [500.0, 60.0, 800.0, 260.0], 16.0),
    (ShadowLevel::Xs, [100.0, 400.0, 140.0, 420.0], 6.0),
    (ShadowLevel::Xl, [300.0, 400.0, 500.0, 520.0], 0.0),
];

fn render() -> image::RgbaImage {
    let theme = Theme::vega();
    let mut h = Harness::builder()
        .with_size(vec2(900.0, 600.0))
        .with_pixels_per_point(1.0)
        .build_ui(|ui| {
            let p = ui.painter();
            p.rect_filled(ui.ctx().content_rect(), 0.0, Color32::WHITE);
            for (level, [x0, y0, x1, y1], r) in BOXES {
                let mut recipe = Recipe::default();
                recipe.shadow.base = Some(level);
                recipe.radius.base = Some(Radius::Px(r));
                let rect = Rect::from_min_max(pos2(x0, y0), pos2(x1, y1));
                RecipeBox::resolve(ui.ctx(), &theme, &[&recipe], State::NONE).paint(p, rect);
            }
        });
    h.step();
    h.render().expect("wgpu render")
}

#[test]
fn shadows_match_chrome() {
    let img = render();
    let dark = |x: u32, y: u32| 255 - i32::from(img.get_pixel(x, y).0[0]);
    let col = |x: u32, y0: u32, n: u32| (y0..y0 + n).map(|y| dark(x, y)).collect::<Vec<_>>();
    let row = |y: u32, x0: u32, n: u32| (x0..x0 + n).map(|x| dark(x, y)).collect::<Vec<_>>();
    let diag = |x0: u32, y0: u32, n: u32, step: u32| {
        (0..n)
            .map(|k| dark(x0 + k * step, y0 + k * step))
            .collect::<Vec<_>>()
    };
    #[rustfmt::skip]
    let cases: [(&str, Vec<i32>, &[i32]); 9] = [
        ("md below", col(200, 180, 12), &[30, 23, 17, 12, 8, 5, 3, 2, 1, 0, 0, 0]),
        ("md right", row(120, 300, 8), &[11, 6, 3, 2, 1, 0, 0, 0]),
        ("md corner", diag(300, 180, 6, 1), &[4, 2, 0, 0, 0, 0]),
        ("2xl below", col(650, 260, 70), &[44, 43, 42, 41, 40, 39, 38, 37, 36, 35, 34, 33, 32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 19, 18, 17, 16, 15, 14, 13, 13, 12, 11, 10, 10, 9, 9, 8, 7, 7, 6, 6, 5, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0]),
        ("2xl right", row(160, 800, 50), &[19, 19, 18, 17, 16, 15, 14, 13, 13, 12, 11, 10, 10, 9, 9, 8, 7, 7, 6, 6, 5, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        ("2xl corner", diag(800, 260, 25, 2), &[13, 12, 10, 8, 7, 6, 5, 4, 3, 3, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        ("xs below", col(120, 420, 5), &[9, 3, 0, 0, 0]),
        ("xl below", col(400, 520, 50), &[36, 35, 32, 31, 28, 26, 24, 21, 19, 18, 17, 15, 15, 14, 13, 12, 11, 11, 10, 9, 8, 8, 7, 6, 6, 5, 4, 4, 3, 3, 3, 2, 2, 2, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        ("xl right", row(460, 500, 30), &[10, 9, 8, 6, 6, 5, 4, 4, 3, 3, 3, 2, 2, 2, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
    ];
    for (name, got, want) in cases {
        let worst = got.iter().zip(want).map(|(g, w)| (g - w).abs()).max();
        assert!(worst <= Some(2), "{name}: got {got:?}, Chrome {want:?}");
    }
}
