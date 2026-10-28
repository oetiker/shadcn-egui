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

/// Chrome does not let a box's background show in the anti-aliased outer
/// edge of an opaque border (Blink's background bleed avoidance): at the
/// rounded corners of a dark box with a red border on white, every edge
/// pixel is a mix of the red and the white only. Measured in headless
/// Chrome at device scale 2: no edge pixel more than 2 levels darker
/// than that mix; the crate painted the fill under the edge, up to 55.
#[test]
fn an_opaque_border_hides_the_fill_in_its_outer_edge() {
    use shadcn_egui::style::Radius;
    let theme = Theme::vega();
    let palette = theme.palette_of(egui::Theme::Light);
    let red = palette.destructive;
    let mut h = Harness::builder()
        .with_size(vec2(160.0, 80.0))
        .with_pixels_per_point(2.0)
        .with_theme(egui::Theme::Light)
        .build_ui(|ui| {
            let theme = Theme::current(ui.ctx());
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, Color32::WHITE);
            for (i, radius) in [6.0, 10.0].into_iter().enumerate() {
                let mut r = Recipe::default();
                r.border_width.base = Some(1.0);
                r.border_color.base = Some(Paint::Role(Role::Destructive, 1.0));
                r.bg.base = Some(Paint::Role(Role::Foreground, 1.0));
                r.radius.base = Some(Radius::Px(radius));
                let rect = Rect::from_min_size(pos2(LEFT + i as f32 * 70.0, TOP), vec2(60.0, 40.0));
                RecipeBox::resolve(ui.ctx(), &theme, &[&r], State::NONE).paint(ui.painter(), rect);
            }
        });
    h.step();
    let img = h.render().expect("wgpu render");
    let (mut worst, mut edge) = (0.0_f32, 0);
    for p in img.pixels() {
        let [r, g, b, _] = p.0.map(f32::from);
        // An outer edge pixel: some white in it (green above the red's and
        // the fill's), not all of it.
        if g <= 12.0 || g >= 250.0 {
            continue;
        }
        edge += 1;
        // The red's share, from green and from blue; the red channel of
        // that mix of red and white.
        let a = (255.0 - g) / (255.0 - f32::from(red.g()));
        let a_b = (255.0 - b) / (255.0 - f32::from(red.b()));
        let want = 255.0 - a.max(a_b) * (255.0 - f32::from(red.r()));
        worst = worst.max(want - r);
    }
    assert!(edge > 20, "{edge} edge pixels");
    assert!(
        worst <= 4.0,
        "the fill darkens an edge pixel by {worst} levels"
    );
}

/// A faded box paints as CSS group opacity: the border does not composite
/// over the fill. `infra/q8`'s Chrome page, 16px boxes at `opacity: .5` on
/// the dark background, measured at device scale 1: a primary box with a
/// primary border reads 120 in the border and the fill alike; a box with
/// `bg-input/30` and a `border-input` border reads 32 in the border and 15
/// in the fill. The crate painted the first border at 147 (border over the
/// faded fill).
#[test]
fn a_faded_box_paints_its_border_and_fill_as_one_group() {
    let mut h = Harness::builder()
        .with_size(vec2(80.0, 40.0))
        .with_pixels_per_point(1.0)
        .with_theme(egui::Theme::Dark)
        .build_ui(|ui| {
            let theme = Theme::current(ui.ctx());
            ui.painter().rect_filled(
                ui.ctx().content_rect(),
                0.0,
                theme.palette_of(egui::Theme::Dark).background,
            );
            let mut painter = ui.painter().clone();
            painter.set_opacity(0.5);
            for (i, (bg, border)) in [
                (
                    Paint::Role(Role::Primary, 1.0),
                    Paint::Role(Role::Primary, 1.0),
                ),
                (Paint::Role(Role::Input, 0.3), Paint::Role(Role::Input, 1.0)),
            ]
            .into_iter()
            .enumerate()
            {
                let mut r = Recipe::default();
                r.border_width.base = Some(1.0);
                r.border_color.base = Some(border);
                r.bg.base = Some(bg);
                let rect =
                    Rect::from_min_size(pos2(LEFT + i as f32 * 30.0, 10.0), vec2(16.0, 16.0));
                RecipeBox::resolve(ui.ctx(), &theme, &[&r], State::NONE).paint(&painter, rect);
            }
        });
    h.step();
    let img = h.render().expect("wgpu render");
    let px = |x: f32, y: f32| i32::from(img.get_pixel(x as u32, y as u32).0[0]);
    for (i, (border, fill)) in [(120, 120), (32, 15)].into_iter().enumerate() {
        let left = LEFT + i as f32 * 30.0;
        let got = (px(left, 18.0), px(left + 8.0, 18.0), px(left + 8.0, 10.0));
        assert!(
            (got.0 - border).abs() <= 1 && (got.2 - border).abs() <= 1,
            "box {i}: border {got:?}, Chrome {border}"
        );
        assert!(
            (got.1 - fill).abs() <= 1,
            "box {i}: fill {got:?}, Chrome {fill}"
        );
    }
}

/// A faded box paints fill and border as one mesh, cut along its border's
/// inner edge and the feathering band beside it. Those cuts lie on pixel
/// centres for a box on whole pixels; every pixel inside the box must
/// still be painted (a switch track showed the page through one pixel).
#[test]
fn a_faded_box_has_no_hole_along_its_cuts() {
    use shadcn_egui::style::{Radius, Rung};
    // (rect, border width): luma's switch track has 2px.
    let rects = [
        (
            Rect::from_min_size(pos2(72.0, 190.0), vec2(28.0, 16.0)),
            2.0,
        ),
        (
            Rect::from_min_size(pos2(72.0, 160.0), vec2(28.0, 16.0)),
            1.0,
        ),
        (Rect::from_min_size(pos2(20.0, 20.0), vec2(44.0, 20.0)), 1.0),
        (Rect::from_min_size(pos2(20.0, 60.0), vec2(16.0, 16.0)), 1.0),
    ];
    let mut h = Harness::builder()
        .with_size(vec2(120.0, 220.0))
        .with_pixels_per_point(1.0)
        .with_theme(egui::Theme::Dark)
        .build_ui(move |ui| {
            let theme = Theme::current(ui.ctx());
            ui.painter().rect_filled(
                ui.ctx().content_rect(),
                0.0,
                theme.palette_of(egui::Theme::Dark).background,
            );
            let mut painter = ui.painter().clone();
            painter.set_opacity(0.5);
            for (i, (rect, width)) in rects.into_iter().enumerate() {
                let mut r = Recipe::default();
                r.border_width.base = Some(width);
                r.border_color.base = Some(Paint::Role(Role::Primary, 1.0));
                r.bg.base = Some(Paint::Role(Role::Primary, 1.0));
                r.radius.base = Some(if i == 3 {
                    Radius::Px(4.0)
                } else {
                    Radius::Rung(Rung::Full)
                });
                RecipeBox::resolve(ui.ctx(), &theme, &[&r], State::NONE).paint(&painter, rect);
            }
        });
    h.step();
    let img = h.render().expect("wgpu render");
    let mut holes = Vec::new();
    for (rect, _) in rects {
        // Pixels a whole pixel inside the outline: the pill's ends are
        // half circles, the small box's corners 4px arcs.
        let r = if rect.width() > 20.0 {
            rect.height() / 2.0
        } else {
            4.0
        };
        let inner = rect.shrink(r);
        for y in rect.top() as u32..rect.bottom() as u32 {
            for x in rect.left() as u32..rect.right() as u32 {
                let c = pos2(x as f32 + 0.5, y as f32 + 0.5);
                let near = pos2(
                    c.x.clamp(inner.left(), inner.right()),
                    c.y.clamp(inner.top(), inner.bottom()),
                );
                if (c - near).length() > r - 1.0 {
                    continue;
                }
                let v = img.get_pixel(x, y).0[0];
                if v.abs_diff(120) > 2 {
                    holes.push((x, y, v));
                }
            }
        }
    }
    assert!(holes.is_empty(), "pixels off the faded colour: {holes:?}");
}
