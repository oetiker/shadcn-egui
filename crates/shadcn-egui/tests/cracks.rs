//! No crack in a cut mesh: where the crate cuts egui's meshes (a faded
//! box with sides of unequal width, a part cut to its rounded parent, a
//! track cut under a faded thumb), the pieces meet at T-junctions. A
//! pixel centre that falls into a crack between them shows the page.
//! Each case is rendered at 1x and 2x, at a few sizes and fractional
//! positions, and every pixel well inside a part must have the part's
//! colour.

use egui::{pos2, vec2, Color32, Pos2, Rect};
use egui_kittest::Harness;
use shadcn_egui::components::progress::{Progress, ProgressRecipes, ProgressVariant};
use shadcn_egui::components::shared::RecipeBox;
use shadcn_egui::components::slider::Slider;
use shadcn_egui::style::{AppVariant, Paint, Radius, Recipe, Role, Rung, State, Style};
use shadcn_egui::{Palette, Theme};

/// How far inside every outline and cut a pixel centre must lie, in
/// device pixels, to count as inside: past egui's feathering band.
const INSIDE: f32 = 1.5;

/// The fractional offsets the parts are placed at, in points.
const OFFSETS: [f32; 4] = [0.0, 0.25, 0.5, 0.7];

/// Signed distance from `p` to a rect with corner radius `r` (negative
/// inside).
fn sdf(p: Pos2, rect: Rect, r: f32) -> f32 {
    let r = r.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let c = rect.center();
    let q = vec2(
        (p.x - c.x).abs() - (rect.width() / 2.0 - r),
        (p.y - c.y).abs() - (rect.height() / 2.0 - r),
    );
    vec2(q.x.max(0.0), q.y.max(0.0)).length() + q.x.max(q.y).min(0.0) - r
}

/// `paint` at `ppp` on a `page` background, `size` points, with a state
/// that starts as `state`.
fn render<S: Clone + 'static>(
    ppp: f32,
    size: egui::Vec2,
    page: Color32,
    state: S,
    mut paint: impl FnMut(&mut egui::Ui, &mut S) + 'static,
) -> (image::RgbaImage, S) {
    let mut h = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(ppp)
        .with_theme(egui::Theme::Dark)
        .build_ui_state(
            move |ui, s: &mut S| {
                ui.painter().rect_filled(ui.ctx().content_rect(), 0.0, page);
                paint(ui, s);
            },
            state,
        );
    h.run();
    let img = h.render().expect("wgpu render");
    (img, h.state().clone())
}

/// The pixels whose centres (in points) `inside` accepts and whose red
/// channel is more than 2 off `want`; at least 20 must be inside.
fn holes(
    img: &image::RgbaImage,
    ppp: f32,
    inside: impl Fn(Pos2) -> bool,
    want: impl Fn(Pos2) -> u8,
) -> Vec<(u32, u32, u8, u8)> {
    let (mut out, mut checked) = (Vec::new(), 0);
    for (x, y, px) in img.enumerate_pixels() {
        let p = pos2((x as f32 + 0.5) / ppp, (y as f32 + 0.5) / ppp);
        if inside(p) {
            checked += 1;
            if px.0[0].abs_diff(want(p)) > 2 {
                out.push((x, y, px.0[0], want(p)));
            }
        }
    }
    assert!(checked >= 20, "only {checked} pixels inside");
    out
}

/// A faded box with sides of unequal width is cut from egui's rect mesh
/// (`RoundedClip::two_tone`), not built on shared vertices; fill and
/// border share one colour, so the whole box reads one value.
#[test]
fn a_faded_box_with_unequal_sides_has_no_crack() {
    let page = Color32::from_gray(10);
    for ppp in [1.0, 2.0] {
        for (size, radius, left) in [
            (vec2(16.0, 16.0), Radius::Px(4.0), 3.0),
            (vec2(44.0, 20.0), Radius::Rung(Rung::Full), 2.0),
            (vec2(28.0, 16.0), Radius::Rung(Rung::Full), 3.0),
            (vec2(60.0, 33.0), Radius::Px(6.0), 2.0),
        ] {
            for dx in OFFSETS {
                let rect = Rect::from_min_size(pos2(10.0 + dx, 10.0 + dx * 0.6), size);
                let (img, r) = render(ppp, vec2(90.0, 60.0), page, 0.0, move |ui, r: &mut f32| {
                    let theme = Theme::current(ui.ctx());
                    let mut recipe = Recipe::default();
                    recipe.border_width.base = Some(1.0);
                    recipe.border_left_width.base = Some(left);
                    recipe.border_color.base = Some(Paint::Role(Role::Primary, 1.0));
                    recipe.bg.base = Some(Paint::Role(Role::Primary, 1.0));
                    recipe.radius.base = Some(radius);
                    let b = RecipeBox::resolve(ui.ctx(), &theme, &[&recipe], State::NONE);
                    *r = b.radius_on(rect).nw.into();
                    let mut painter = ui.painter().clone();
                    painter.set_opacity(0.5);
                    b.paint(&painter, rect);
                });
                let want = img.get_pixel(
                    (rect.center().x * ppp) as u32,
                    (rect.center().y * ppp) as u32,
                )[0];
                let found = holes(&img, ppp, |p| sdf(p, rect, r) <= -INSIDE / ppp, |_| want);
                assert!(
                    found.is_empty(),
                    "{ppp}x {size:?} at +{dx}: {} holes {:?}",
                    found.len(),
                    &found[..found.len().min(8)]
                );
            }
        }
    }
}

const BORDER: AppVariant<ProgressVariant> = ProgressVariant::app("crack-border");
const ROUND: AppVariant<ProgressVariant> = ProgressVariant::app("crack-round");

/// An indicator that is no plain fill (a border, rounded ends) is cut to
/// the track's rounded box mesh by mesh (`RoundedClip::apply`): inside
/// both outlines (and inside its border) it reads its fill.
#[test]
fn a_part_cut_to_its_rounded_parent_has_no_crack() {
    let theme = Theme::new(Style::maia(), Palette::light(), Palette::dark(), 10.0)
        .variant(
            BORDER,
            ProgressVariant::Default,
            |r: &mut ProgressRecipes, _| {
                r.progress_indicator.border_width.set(1.0);
                r.progress_indicator
                    .border_color
                    .set(Paint::Role(Role::Destructive, 1.0));
            },
        )
        .variant(
            ROUND,
            ProgressVariant::Default,
            |r: &mut ProgressRecipes, _| {
                r.progress_indicator.radius.set(Radius::Rung(Rung::Full));
            },
        );
    let page = Color32::from_gray(10);
    for ppp in [1.0, 2.0] {
        for (variant, inset) in [(BORDER, 1.0), (ROUND, 0.0)] {
            for (width, fraction) in [(200.0, 0.237), (157.0, 0.5), (90.3, 0.731), (200.0, 0.9)] {
                for dx in OFFSETS {
                    let t = theme.clone();
                    let (img, track) = render(
                        ppp,
                        vec2(240.0, 40.0),
                        page,
                        Rect::NOTHING,
                        move |ui, track: &mut Rect| {
                            t.apply(ui.ctx());
                            ui.ctx().all_styles_mut(|s| s.animation_time = 0.0);
                            ui.add_space(8.0 + dx);
                            ui.horizontal(|ui| {
                                ui.add_space(8.0 + dx);
                                *track = ui
                                    .add(
                                        Progress::new(fraction)
                                            .desired_width(width)
                                            .variant(variant),
                                    )
                                    .rect;
                            });
                        },
                    );
                    let r = track.height() / 2.0;
                    let ind = Rect::from_min_size(
                        track.min,
                        vec2(track.width() * fraction, track.height()),
                    );
                    let ind_r = if variant == ROUND { r } else { 0.0 };
                    let want = img.get_pixel(
                        ((ind.left() + r + 1.0) * ppp) as u32,
                        (ind.center().y * ppp) as u32,
                    )[0];
                    let d = INSIDE / ppp;
                    let found = holes(
                        &img,
                        ppp,
                        |p| {
                            sdf(p, track, r) <= -d
                                && sdf(p, ind.shrink(inset), (ind_r - inset).max(0.0)) <= -d
                        },
                        |_| want,
                    );
                    assert!(
                        found.is_empty(),
                        "{ppp}x {variant:?} {width}x{fraction} at +{dx}: {} holes {:?}",
                        found.len(),
                        &found[..found.len().min(8)]
                    );
                }
            }
        }
    }
}

/// A disabled slider's track and range are cut under its thumb
/// (`RoundedClip::remove`): away from the thumb they read their own
/// colours, and the thumb reads its fill.
#[test]
fn a_track_cut_under_a_faded_thumb_has_no_crack() {
    let page = Color32::from_gray(10);
    for ppp in [1.0, 2.0] {
        for value in [13.7, 50.0, 66.6, 91.3] {
            for dx in OFFSETS {
                let (img, rect) = render(
                    ppp,
                    vec2(200.0, 40.0),
                    page,
                    Rect::NOTHING,
                    move |ui, rect: &mut Rect| {
                        Theme::vega().apply(ui.ctx());
                        ui.ctx().all_styles_mut(|s| s.animation_time = 0.0);
                        ui.add_space(8.0 + dx);
                        ui.horizontal(|ui| {
                            ui.add_space(8.0 + dx);
                            ui.set_width(160.0);
                            let mut v = value;
                            *rect = ui.add(Slider::new(&mut v, 0.0..=100.0).enabled(false)).rect;
                        });
                    },
                );
                // vega: a 6px track, a 16px thumb.
                let track = Rect::from_center_size(rect.center(), vec2(rect.width(), 6.0));
                let y = (rect.center().y * ppp) as u32;
                let row: Vec<u8> = (0..img.width()).map(|x| img.get_pixel(x, y)[0]).collect();
                let thumb_px: Vec<u32> = (0..img.width())
                    .filter(|&x| row[x as usize].abs_diff(133) <= 2)
                    .collect();
                let cx = (thumb_px[0] + thumb_px[thumb_px.len() - 1] + 1) as f32 / 2.0 / ppp;
                let thumb = Rect::from_center_size(pos2(cx, rect.center().y), vec2(16.0, 16.0));
                let d = INSIDE / ppp;
                let (range, rest) = (
                    row[((track.left() + 3.5) * ppp) as usize],
                    row[((track.right() - 3.5) * ppp) as usize],
                );
                let found = holes(
                    &img,
                    ppp,
                    |p| {
                        let t = sdf(p, thumb, 8.0);
                        // The thumb's 1px border is its own colour.
                        t <= -1.0 - d || (t >= d && sdf(p, track, 3.0) <= -d)
                    },
                    |p| {
                        if sdf(p, thumb, 8.0) < 0.0 {
                            133
                        } else if p.x < cx {
                            range
                        } else {
                            rest
                        }
                    },
                );
                assert!(
                    found.is_empty(),
                    "{ppp}x value {value} at +{dx}: {} holes {:?}",
                    found.len(),
                    &found[..found.len().min(8)]
                );
            }
        }
    }
}
