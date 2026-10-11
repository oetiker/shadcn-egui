#[allow(dead_code)]
#[path = "icons/fixture/src/lib.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "icons/lucide.rs"]
#[rustfmt::skip]
mod lucide;

use shadcn_egui::{Icon, Seg};

fn coords(s: Seg) -> Vec<f32> {
    match s {
        Seg::M(x, y) | Seg::L(x, y) => vec![x, y],
        Seg::Q(a, b, x, y) => vec![a, b, x, y],
        Seg::C(a, b, c, d, x, y) => vec![a, b, c, d, x, y],
        Seg::Z => vec![],
    }
}

fn area(f: &shadcn_egui::FillMesh) -> f32 {
    f.indices
        .chunks(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| f.vertices[i as usize]);
            ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
        })
        .sum()
}

#[test]
fn blob_strokes_match_the_slice_form() {
    let blob: Vec<Seg> = fixture::X.segs().collect();
    let slice: Vec<Seg> = lucide::X.segs().collect();
    assert_eq!(blob.len(), slice.len());
    for (b, s) in blob.iter().zip(&slice) {
        for (x, y) in coords(*b).iter().zip(coords(*s)) {
            assert!((x - y).abs() <= 0.5 / 1024.0, "{b:?} vs {s:?}");
        }
    }
    assert_eq!(fixture::X.fill_mesh(), None, "a stroke icon has no fill");
}

#[test]
fn blob_fills_decode_with_their_area_and_contours() {
    let f = fixture::SQUARE_HOLE.fill_mesh().expect("filled");
    assert!((area(&f) - 220.0).abs() < 0.01, "area {}", area(&f));
    assert_eq!(f.contours.len(), 2);
    assert!(f.indices.iter().all(|&i| usize::from(i) < f.vertices.len()));
    let mixed = fixture::MIXED;
    assert!(
        mixed.segs().count() > 0 && mixed.fill_mesh().is_some(),
        "strokes and fill in one record"
    );
    let xs: Vec<f32> = fixture::BIG_BOX
        .fill_mesh()
        .expect("filled")
        .vertices
        .iter()
        .flat_map(|v| *v)
        .collect();
    assert!(
        xs.iter().all(|v| (3.0..=21.0).contains(v)),
        "256 view box normalised to 24 units"
    );
}

#[test]
fn generated_consts_point_into_one_blob() {
    assert_eq!(fixture::ALL.len(), 6);
    assert_eq!(fixture::SLOTS.check, fixture::MIXED);
    assert_eq!(fixture::SLOTS.close.name, "x");
    let icon: &Icon = fixture::ALL[0];
    assert_eq!(icon.size, 24.0);
}

use egui::{pos2, vec2, Color32, ColorImage, Id, LayerId, Order, Rect, Shape, TextureId};
use egui_kittest::Harness;
use shadcn_egui::components::icon::paint_icon;
use std::sync::Arc;

/// What one frame painted: each image (texture, rect, tint) and each
/// texture uploaded for it.
struct Painted {
    images: Vec<(TextureId, Rect, Color32)>,
    uploads: Vec<(TextureId, Arc<ColorImage>)>,
}

/// Run one frame of `ctx` at `ppp`, painting with `paint`.
fn frame(ctx: &egui::Context, ppp: f32, mut paint: impl FnMut(&egui::Painter)) -> Painted {
    ctx.set_pixels_per_point(ppp);
    let out = ctx.run_ui(egui::RawInput::default(), |ui| {
        paint(&ui.painter_at(Rect::EVERYTHING));
    });
    fn walk(s: &Shape, out: &mut Vec<(TextureId, Rect, Color32)>) {
        match s {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            Shape::Mesh(m) if m.texture_id != TextureId::default() => {
                let tint = m.vertices.first().map_or(Color32::TRANSPARENT, |v| v.color);
                out.push((m.texture_id, m.calc_bounds(), tint));
            }
            _ => {}
        }
    }
    let mut images = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut images));
    let uploads = out
        .textures_delta
        .set
        .iter()
        .filter(|(id, _)| *id != TextureId::default())
        .map(|(id, d)| {
            let egui::ImageData::Color(img) = &d.image;
            (*id, img.clone())
        })
        .collect();
    Painted { images, uploads }
}

/// Paint one icon into `rect` (stroke width 2) on a fresh context and
/// return the drawn rect and its coverage texture.
fn raster(icon: &Icon, rect: Rect, ppp: f32) -> (Rect, Arc<ColorImage>) {
    let ctx = egui::Context::default();
    let p = frame(&ctx, ppp, |p| paint_icon(p, rect, icon, Color32::RED, 2.0));
    assert_eq!(p.images.len(), 1, "one image per icon");
    assert_eq!(p.uploads.len(), 1, "one texture per icon");
    assert_eq!(p.images[0].0, p.uploads[0].0);
    (p.images[0].1, p.uploads[0].1.clone())
}

#[test]
fn fill_edges_are_anti_aliased_by_coverage() {
    // 20 px for 24 units: the outer edge x = 4 lands at 3.33 px, the hole's
    // edge x = 9 at 7.5 px. Row 10 (y 12.6 units) crosses both.
    let (drawn, img) = raster(
        &fixture::SQUARE_HOLE,
        Rect::from_min_size(pos2(0.0, 0.0), vec2(20.0, 20.0)),
        1.0,
    );
    assert_eq!(drawn, Rect::from_min_size(pos2(0.0, 0.0), vec2(20.0, 20.0)));
    assert_eq!(img.size, [20, 20]);
    let a = |x: usize| i32::from(img.pixels[10 * 20 + x].a());
    assert_eq!(a(2), 0, "outside");
    assert!((a(3) - 170).abs() <= 2, "two thirds covered: {}", a(3));
    assert_eq!(a(4), 255, "inside");
    assert!((a(7) - 128).abs() <= 2, "half covered: {}", a(7));
    assert_eq!(a(10), 0, "the hole");
    assert_eq!(
        img.pixels[10 * 20 + 4],
        Color32::from_white_alpha(255),
        "white coverage"
    );
}

#[test]
fn icon_snaps_to_physical_pixels() {
    // At ppp 1.25, (10.1, 20.3) + 24 maps to pixels (12.6, 25.4) + 30.
    let (drawn, img) = raster(
        &fixture::MIXED,
        Rect::from_min_size(pos2(10.1, 20.3), vec2(24.0, 24.0)),
        1.25,
    );
    assert_eq!(img.size, [30, 30]);
    let px = drawn.min.to_vec2() * 1.25;
    assert!((px - vec2(13.0, 25.0)).length() < 1e-4, "{px:?}");
    assert!((drawn.size() * 1.25 - vec2(30.0, 30.0)).length() < 1e-4);
}

/// Review Focus: two crossing strokes at 50 % alpha are as light at the
/// crossing as along one line; the overlap counts once.
#[test]
fn crossing_strokes_blend_once() {
    use shadcn_egui::IconShape;
    static X: [Seg; 4] = [
        Seg::M(4.0, 4.0),
        Seg::L(20.0, 20.0),
        Seg::M(20.0, 4.0),
        Seg::L(4.0, 20.0),
    ];
    let icon = Icon {
        name: "cross",
        size: 24.0,
        shape: IconShape::Segs(&X),
    };
    let mut h = Harness::builder()
        .with_size(vec2(40.0, 40.0))
        .build_ui(|ui| {
            let page = ui.ctx().viewport_rect();
            ui.ctx()
                .layer_painter(LayerId::background())
                .rect_filled(page, 0.0, Color32::WHITE);
            let p = ui
                .ctx()
                .layer_painter(LayerId::new(Order::Foreground, Id::new("icon")));
            let rect = Rect::from_min_size(pos2(8.0, 8.0), vec2(24.0, 24.0));
            paint_icon(&p, rect, &icon, Color32::RED.gamma_multiply(0.5), 2.0);
        });
    h.run();
    let img = h.render().expect("render");
    let g = |x: u32, y: u32| i32::from(img.get_pixel(x, y).0[1]);
    // Pixel (20, 20) is the crossing, (16, 16) lies on one line only.
    let (crossing, line) = (g(20, 20), g(16, 16));
    assert!((120..=136).contains(&line), "line green {line}");
    assert!(
        (crossing - line).abs() <= 2,
        "crossing green {crossing} vs line {line}"
    );
}

#[test]
fn faded_and_opaque_icons_share_one_texture() {
    let ctx = egui::Context::default();
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0));
    let p = frame(&ctx, 1.0, |p| {
        paint_icon(p, rect, &fixture::MIXED, Color32::RED, 2.0);
        paint_icon(
            p,
            rect,
            &fixture::MIXED,
            Color32::RED.gamma_multiply(0.5),
            2.0,
        );
        let mut faded = p.clone();
        faded.multiply_opacity(0.5);
        paint_icon(&faded, rect, &fixture::MIXED, Color32::RED, 2.0);
    });
    assert_eq!(p.uploads.len(), 1, "one texture for all three");
    assert_eq!(p.images.len(), 3);
    assert!(p.images.iter().all(|(id, _, _)| *id == p.uploads[0].0));
    assert_eq!(p.images[0].2, Color32::RED);
    assert_eq!(p.images[1].2, Color32::RED.gamma_multiply(0.5));
    assert_eq!(p.images[2].2, Color32::RED.gamma_multiply(0.5));
}

#[test]
fn textures_are_cached_and_bounded() {
    let ctx = egui::Context::default();
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0));
    let once = |p: &egui::Painter| paint_icon(p, rect, &fixture::MIXED, Color32::RED, 2.0);
    assert_eq!(frame(&ctx, 1.0, once).uploads.len(), 1);
    let again = frame(&ctx, 1.0, once);
    assert_eq!(again.uploads.len(), 0, "cached: no upload in frame 2");
    assert_eq!(again.images.len(), 1);
    // 1200 distinct stroke widths, 100 per frame: never more than 1024
    // icon textures alive (plus the font atlas).
    let small = Rect::from_min_size(pos2(0.0, 0.0), vec2(4.0, 4.0));
    for f in 0..12 {
        frame(&ctx, 1.0, |p| {
            for i in 0..100 {
                let w = 1.0 + (f * 100 + i) as f32 / 1000.0;
                paint_icon(p, small, &fixture::X, Color32::RED, w);
            }
        });
        let alive = ctx.tex_manager().read().num_allocated();
        assert!(alive <= 1024 + 1, "frame {f}: {alive} textures");
    }
    assert!(
        ctx.tex_manager().read().num_allocated() >= 1000,
        "the cache is used"
    );
}

/// Rects of no, negative, absurd and non-finite size paint something
/// bounded or nothing, never a panic or a giant texture.
#[test]
fn odd_rects_do_not_panic() {
    let ctx = egui::Context::default();
    for rect in [
        Rect::from_min_size(pos2(5.0, 5.0), vec2(0.0, 0.0)),
        Rect::from_min_size(pos2(5.0, 5.0), vec2(-3.0, 8.0)),
        Rect::from_min_size(pos2(0.0, 0.0), vec2(1e7, 1e7)),
        Rect::from_min_size(pos2(f32::NAN, 0.0), vec2(24.0, 24.0)),
        Rect::from_min_size(pos2(0.0, 0.0), vec2(f32::INFINITY, 24.0)),
        Rect::EVERYTHING,
    ] {
        let p = frame(&ctx, 1.0, |p| {
            paint_icon(p, rect, &fixture::MIXED, Color32::RED, 2.0);
        });
        for (_, img) in &p.uploads {
            assert!(
                img.size[0] <= 1024 && img.size[1] <= 1024,
                "{rect:?}: {:?}",
                img.size
            );
        }
    }
}

/// A curve without a preceding `M` (a public `IconShape::Segs` or a damaged
/// record) is skipped, not a panic in the middle of a frame.
#[test]
fn a_curve_without_a_start_point_does_not_panic() {
    use shadcn_egui::IconShape;
    for segs in [
        &[Seg::Q(1.0, 1.0, 2.0, 2.0), Seg::L(4.0, 4.0)][..],
        &[Seg::C(1.0, 1.0, 2.0, 2.0, 3.0, 3.0), Seg::Z][..],
    ] {
        let icon = Icon {
            name: "bad",
            size: 24.0,
            shape: IconShape::Segs(segs),
        };
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0));
        frame(&ctx, 1.0, |p| paint_icon(p, rect, &icon, Color32::RED, 2.0));
    }
}

/// Review Focus 4: a translucent filled icon shows no darker seam where the
/// fill meets the fringe (nor where the even-odd star's arms touch) and no
/// lighter gap inside, also at a fractional scale. `mixed` is painted with
/// stroke width 0, its dot alone.
#[test]
fn filled_icon_has_no_seam_at_fractional_scale() {
    type Inside = fn(f32, f32) -> bool;
    let cases: [(&str, Icon, f32, [f32; 2], Inside); 3] = [
        // Icon (12, 4) lies between the ring's edge (radius 10) and its hole (8..16).
        ("ring", fixture::RING, 2.0, [12.0, 4.0], |x, y| {
            let r = ((x - 12.0).powi(2) + (y - 12.0).powi(2)).sqrt();
            r < 9.0 && !((7.0..=17.0).contains(&x) && (7.0..=17.0).contains(&y))
        }),
        // The top arm, tip (12, 2), base 9.79..14.21 at y = 9.
        (
            "star-evenodd",
            fixture::STAR_EVENODD,
            2.0,
            [12.0, 6.0],
            |x, y| (4.0..=8.0).contains(&y) && (x - 12.0).abs() < (y - 2.0) * 0.316 - 0.8,
        ),
        // The dot, centre (12, 6), radius 2.
        ("mixed", fixture::MIXED, 0.0, [12.0, 6.0], |x, y| {
            ((x - 12.0).powi(2) + (y - 6.0).powi(2)).sqrt() < 1.0
        }),
    ];
    for (name, icon, stroke, [sx, sy], inside) in cases {
        for ppp in [1.0_f32, 1.25] {
            let mut h = Harness::builder()
                .with_size(vec2(40.0, 40.0))
                .with_pixels_per_point(ppp)
                .build_ui(|ui| {
                    let page = ui.ctx().viewport_rect();
                    ui.ctx().layer_painter(LayerId::background()).rect_filled(
                        page,
                        0.0,
                        Color32::WHITE,
                    );
                    let p = ui
                        .ctx()
                        .layer_painter(LayerId::new(Order::Foreground, Id::new("icon")));
                    let rect = Rect::from_min_size(pos2(8.0, 8.0), vec2(24.0, 24.0));
                    paint_icon(&p, rect, &icon, Color32::RED.gamma_multiply(0.5), stroke);
                });
            h.run();
            let img = h.render().expect("render");
            let g = |x: u32, y: u32| i32::from(img.get_pixel(x, y).0[1]);
            let px = |v: f32| ((8.0 + v) * ppp) as u32;
            let interior = g(px(sx), px(sy));
            assert!(
                (120..=136).contains(&interior),
                "{name} ppp {ppp}: interior green {interior}"
            );
            for y in 0..img.height() {
                for x in 0..img.width() {
                    let v = g(x, y);
                    assert!(
                        v >= interior - 3,
                        "{name} ppp {ppp}: ({x}, {y}) green {v} < interior {interior} - 3: seam"
                    );
                    let (ix, iy) = ((x as f32 + 0.5) / ppp - 8.0, (y as f32 + 0.5) / ppp - 8.0);
                    if inside(ix, iy) {
                        assert!(
                            (v - interior).abs() <= 3,
                            "{name} ppp {ppp}: ({x}, {y}) green {v} vs interior {interior}: gap"
                        );
                    }
                }
            }
        }
    }
}

fn zoomed(name: &str, alpha: f32) {
    let mut h = Harness::builder()
        .with_size(vec2(96.0, 32.0))
        .with_pixels_per_point(8.0)
        .build_ui(|ui| {
            let page = ui.ctx().viewport_rect();
            ui.ctx()
                .layer_painter(LayerId::background())
                .rect_filled(page, 0.0, Color32::WHITE);
            let p = ui
                .ctx()
                .layer_painter(LayerId::new(Order::Foreground, Id::new("icons")));
            let color = Color32::from_rgb(9, 9, 11).gamma_multiply(alpha);
            for (i, icon) in [&fixture::RING, &fixture::STAR_EVENODD, &fixture::MIXED]
                .into_iter()
                .enumerate()
            {
                let rect = Rect::from_min_size(pos2(4.0 + 32.0 * i as f32, 4.0), vec2(24.0, 24.0));
                paint_icon(&p, rect, icon, color, 2.0);
            }
        });
    h.run();
    h.snapshot(name);
}

#[test]
fn filled_icon_edges_zoomed_opaque() {
    zoomed("icon_fill_edge_a100", 1.0);
}

#[test]
fn filled_icon_edges_zoomed_half() {
    zoomed("icon_fill_edge_a50", 0.5);
}
