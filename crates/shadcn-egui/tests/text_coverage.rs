//! Text ink against Chrome (P4, conformance 2.5): for every measured font
//! the build has, at 1x and 2x, in light and dark mode, one line of text
//! through `TextLook` puts as much ink on the screen as Chrome's grayscale
//! anti-aliasing does, within [`TOLERANCE`].
//!
//! The reference is `text_placement/chrome-ink.tsv`, from
//! `node scripts/chrome-text-coverage.mjs <chrome-binary>`: per font,
//! scale, mode, size and weight, the sum over the device pixels of
//! |pixel - background| / |text colour - background|, on the mean of the
//! three channels. This test renders the same cases headlessly (wgpu, as
//! the snapshot tests) and sums the same way.

use egui::{pos2, vec2, Color32, Rect};
use egui_kittest::Harness;
use shadcn_egui::components::text::TextLook;
use shadcn_egui::style::{Recipe, State, TextSize, Weight};
use shadcn_egui::{fonts, FontFiles, FontName, Theme};

const CHROME: &str = include_str!("text_placement/chrome-ink.tsv");
/// Keep in step with `scripts/chrome-text-coverage.mjs`.
const TEXT: &str = "The quick brown fox jumps over 0123 Typography";
const SIZES: [f32; 4] = [12.0, 14.0, 16.0, 20.0];
const WEIGHTS: [(u32, Weight); 3] = [
    (400, Weight::Normal),
    (500, Weight::Medium),
    (600, Weight::Semibold),
];
const SCALES: [f32; 2] = [1.0, 2.0];
const LEFT: f32 = 16.0;
const TOP: f32 = 16.0;
const STEP: f32 = 44.0;
const WIDTH: f32 = 760.0;

/// Largest share by which a case's ink may differ from Chrome's. No one
/// curve brings every case closer: what is left is the font and size
/// (hinting, rasterizer), up to 4.7 % in light and 4.3 % in dark mode.
const TOLERANCE: f32 = 0.05;
/// Largest share by which the mean over a mode's cases may differ: the
/// curve's own error. With Inter alone the light mean is -1.4 %; egui's
/// dark default is +11.5 %, a gamma of 0.76 +1.7 %, plain coverage -5.4 %.
const MEAN_TOLERANCE: f32 = 0.02;

/// Chrome's ink per (size, weight) for `font` at `scale` in `mode`.
fn chrome(font: &str, scale: f32, mode: &str) -> Vec<(f32, u32, f32)> {
    CHROME
        .lines()
        .skip(1)
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .filter(|c| c[0] == font && c[1].parse::<f32>().ok() == Some(scale) && c[2] == mode)
        .map(|c| {
            (
                c[3].parse().unwrap(),
                c[4].parse().unwrap(),
                c[5].parse().unwrap(),
            )
        })
        .collect()
}

/// The ink of each case, in the order of `SIZES` x `WEIGHTS`, as the
/// Chrome script sums it.
fn drawn(files: &'static FontFiles, scale: f32, mode: egui::Theme) -> Vec<f32> {
    let theme = Theme::vega().fonts(files, None);
    let bg = theme.palette_of(mode).background;
    let n = SIZES.len() * WEIGHTS.len();
    let height = TOP + n as f32 * STEP + 20.0;
    let mut h = Harness::builder()
        .with_size(vec2(WIDTH, height))
        .with_pixels_per_point(scale)
        .with_theme(mode)
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            let page = ui.ctx().viewport_rect();
            let painter = ui.ctx().layer_painter(egui::LayerId::background());
            painter.rect_filled(page, 0.0, bg);
            let mut fg = Color32::PLACEHOLDER;
            for (i, (size, (_, weight))) in SIZES
                .iter()
                .flat_map(|s| WEIGHTS.iter().map(move |w| (*s, *w)))
                .enumerate()
            {
                let mut recipe = Recipe::default();
                recipe.text_size.base = Some(TextSize::Px(size));
                recipe.line_height.base = Some(1.5);
                recipe.weight.base = Some(weight);
                egui::Area::new(egui::Id::new(("ink", i)))
                    .fixed_pos(pos2(LEFT, TOP + i as f32 * STEP))
                    .show(ui.ctx(), |ui| {
                        let look = TextLook::resolve(ui, &[&recipe], State::NONE);
                        let text = look.layout(ui, TEXT, egui::TextWrapMode::Extend, f32::INFINITY);
                        let (rect, _) = ui.allocate_exact_size(text.size(), egui::Sense::hover());
                        fg = look.resolved_color(ui);
                        text.paint(ui.painter(), rect.min, fg);
                    });
            }
            // A swatch of the text colour, as the Chrome page has.
            let swatch = Rect::from_min_size(pos2(LEFT, TOP + n as f32 * STEP), vec2(20.0, 20.0));
            painter.rect_filled(swatch, 0.0, fg);
        });
    h.run();
    let img = h.render().expect("render");
    let at = |x: u32, y: u32| {
        let p = img.get_pixel(x, y).0;
        (p[0] as f32 + p[1] as f32 + p[2] as f32) / 3.0
    };
    let s = |v: f32| (v * scale).round() as u32;
    let back = at(1, 1);
    let fore = at(s(LEFT + 10.0), s(TOP + n as f32 * STEP + 10.0));
    (0..n)
        .map(|i| {
            let (y0, y1) = (
                s(TOP + i as f32 * STEP - 6.0),
                s(TOP + (i + 1) as f32 * STEP - 6.0),
            );
            let mut ink = 0.0;
            for y in y0..y1 {
                for x in 0..img.width() {
                    ink += (at(x, y) - back).abs();
                }
            }
            ink / (fore - back).abs()
        })
        .collect()
}

/// Per case: (tag, drawn / Chrome - 1).
fn residuals() -> Vec<(String, f32)> {
    let mut out = Vec::new();
    let fonts: Vec<&str> = {
        let mut f: Vec<&str> = CHROME
            .lines()
            .skip(1)
            .filter_map(|l| l.split('\t').next())
            .collect();
        f.dedup();
        f
    };
    for id in fonts {
        let Some(files) = FontName::ALL
            .iter()
            .find(|n| n.id() == id)
            .and_then(|n| fonts::files(*n))
        else {
            continue;
        };
        for scale in SCALES {
            for (mode, name) in [(egui::Theme::Light, "light"), (egui::Theme::Dark, "dark")] {
                let want = chrome(id, scale, name);
                let got = drawn(files, scale, mode);
                assert_eq!(want.len(), got.len(), "{id} {scale}x {name}: cases");
                for ((size, weight, w), g) in want.iter().zip(got) {
                    out.push((
                        format!("{id} {scale}x {name} {size}px/{weight}"),
                        g / w - 1.0,
                    ));
                }
            }
        }
    }
    out
}

#[test]
fn text_has_chromes_ink() {
    let res = residuals();
    assert!(res.iter().any(|(t, _)| t.starts_with("inter ")), "no inter");
    let mut wrong: Vec<String> = res
        .iter()
        .filter(|(_, r)| r.abs() > TOLERANCE)
        .map(|(t, r)| format!("{t}: {:+.1} %", r * 100.0))
        .collect();
    for mode in ["light", "dark"] {
        let r: Vec<f32> = res
            .iter()
            .filter(|(t, _)| t.split(' ').nth(2) == Some(mode))
            .map(|(_, r)| *r)
            .collect();
        let mean = r.iter().sum::<f32>() / r.len() as f32;
        if mean.abs() > MEAN_TOLERANCE {
            wrong.push(format!("{mode}: mean {:+.1} %", mean * 100.0));
        }
    }
    assert!(
        wrong.is_empty(),
        "text ink off Chrome's (per case {} %, mean {} %):\n{}",
        TOLERANCE * 100.0,
        MEAN_TOLERANCE * 100.0,
        wrong.join("\n")
    );
}
