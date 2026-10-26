//! Border widths and the content inset against Chrome at device scale 1,
//! 1.5 and 2. The reference rows (`fixtures/chrome-borders.tsv`, written by
//! `scripts/chrome-borders.mjs`) hold, per scale and CSS border width, the
//! device pixels Chrome draws for each side of a 30px box and where its
//! content starts behind 6px of padding. The crate paints the same boxes
//! through `RecipeBox` and places the content with `RecipeBox::frame`.

use egui::{pos2, vec2, Color32, Rect};
use egui_kittest::Harness;
use shadcn_egui::components::shared::RecipeBox;
use shadcn_egui::style::{Paint, Recipe, Role, State};
use shadcn_egui::Theme;

const CHROME: &str = include_str!("fixtures/chrome-borders.tsv");
/// Keep in step with WIDTHS, SIZE, PAD, STEP, LEFT and TOP in
/// `scripts/chrome-borders.mjs`.
const WIDTHS: [f32; 5] = [0.5, 1.0, 1.5, 2.0, 3.0];
const SIZE: f32 = 30.0;
const PAD: f32 = 6.0;
const STEP: f32 = 40.0;
const LEFT: f32 = 20.0;
const TOP: f32 = 20.0;

/// Runs of `n` pixels where `pred` holds, as (first, length).
fn runs(n: u32, pred: impl Fn(u32) -> bool) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for i in 0..n {
        if !pred(i) {
            continue;
        }
        match out.last_mut() {
            Some((a, l)) if *a + *l == i => *l += 1,
            _ => out.push((i, 1)),
        }
    }
    out
}

/// Per border width: left, right, top, bottom border and the content's x
/// and y, in device px from the box's outer edge.
fn crate_rows(scale: f32) -> Vec<[u32; 6]> {
    let mut h = Harness::builder()
        .with_size(vec2(240.0, 80.0))
        .with_pixels_per_point(scale)
        .with_theme(egui::Theme::Light)
        .build_ui(|ui| {
            let theme = Theme::current(ui.ctx());
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, Color32::WHITE);
            for (i, w) in WIDTHS.into_iter().enumerate() {
                let mut r = Recipe::default();
                r.border_width.base = Some(w);
                r.border_color.base = Some(Paint::Role(Role::Foreground, 1.0));
                r.bg.base = Some(Paint::Role(Role::Background, 1.0));
                let rect = Rect::from_min_size(pos2(LEFT + i as f32 * STEP, TOP), vec2(SIZE, SIZE));
                let b = RecipeBox::resolve(ui.ctx(), &theme, &[&r], State::NONE);
                b.paint(ui.painter(), rect);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                b.frame([PAD; 4]).show(&mut child, |ui| {
                    let at = ui.max_rect().min;
                    ui.painter().rect_filled(
                        Rect::from_min_size(at, vec2(4.0, 4.0)),
                        0.0,
                        Color32::RED,
                    );
                });
            }
        });
    h.step();
    let img = h.render().expect("wgpu render");
    let px = |x: u32, y: u32| img.get_pixel(x, y).0;
    let black = |x, y| {
        let [r, g, b, _] = px(x, y);
        u32::from(r) + u32::from(g) + u32::from(b) < 3 * 128
    };
    let red = |x, y| {
        let [r, g, b, _] = px(x, y);
        r > 200 && g < 80 && b < 80
    };
    let dev = |v: f32| (v * scale).round() as u32;
    (0..WIDTHS.len())
        .map(|i| {
            let (x0, y0, n) = (dev(LEFT + i as f32 * STEP), dev(TOP), dev(SIZE));
            let (mid_x, mid_y) = (x0 + n / 2, y0 + n / 2);
            let across = runs(n, |k| black(x0 + k, mid_y));
            let down = runs(n, |k| black(mid_x, y0 + k));
            assert_eq!(across.len(), 2, "{scale}x {}: {across:?}", WIDTHS[i]);
            assert_eq!(down.len(), 2, "{scale}x {}: {down:?}", WIDTHS[i]);
            let (l, r, t, b) = (across[0], across[1], down[0], down[1]);
            assert_eq!(
                (l.0, t.0, r.0 + r.1, b.0 + b.1),
                (0, 0, n, n),
                "{scale}x {}: off the edge",
                WIDTHS[i]
            );
            let probe = ((PAD + 2.0) * scale).floor() as u32;
            let cx = runs(n, |k| red(x0 + k, y0 + probe + t.1));
            let cy = runs(n, |k| red(x0 + probe + l.1, y0 + k));
            [l.1, r.1, t.1, b.1, cx[0].0, cy[0].0]
        })
        .collect()
}

#[test]
fn borders_and_content_sit_on_chromes_device_pixels() {
    let mut wrong = Vec::new();
    let mut compared = 0;
    for scale in [1.0, 1.5, 2.0] {
        let got = crate_rows(scale);
        for line in CHROME.lines().skip(1) {
            let f: Vec<&str> = line.split('\t').collect();
            if f[0].parse::<f32>().unwrap() != scale {
                continue;
            }
            let w: f32 = f[1].parse().unwrap();
            let i = WIDTHS
                .iter()
                .position(|v| *v == w)
                .expect("width in WIDTHS");
            let want: Vec<u32> = f[2..8].iter().map(|v| v.parse().unwrap()).collect();
            compared += 1;
            if got[i][..] != want[..] {
                wrong.push(format!(
                    "{scale}x {w}px (left, right, top, bottom, content x, y): crate {:?}, Chrome {want:?}",
                    got[i]
                ));
            }
        }
    }
    assert_eq!(compared, 15);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
