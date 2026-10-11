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

use egui::{pos2, vec2, Color32, Id, LayerId, Order, Rect, Shape};
use egui_kittest::Harness;
use shadcn_egui::components::icon::paint_icon;

/// Paint one icon at pixels_per_point 1 and return the flattened shapes.
fn painted(icon: &Icon, rect: Rect) -> Vec<Shape> {
    let ctx = egui::Context::default();
    let out = ctx.run_ui(egui::RawInput::default(), |ui| {
        paint_icon(
            &ui.painter_at(Rect::EVERYTHING),
            rect,
            icon,
            Color32::RED,
            2.0,
        );
    });
    fn walk(s: &Shape, out: &mut Vec<Shape>) {
        match s {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            s => out.push(s.clone()),
        }
    }
    let mut shapes = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut shapes));
    shapes
}

#[test]
fn fill_is_one_mesh_with_an_outward_fringe() {
    // Scale 2: the outer corner (4, 4) of square-hole lands at (8, 8).
    let shapes = painted(
        &fixture::SQUARE_HOLE,
        Rect::from_min_size(pos2(0.0, 0.0), vec2(48.0, 48.0)),
    );
    let meshes: Vec<_> = shapes
        .iter()
        .filter_map(|s| {
            if let Shape::Mesh(m) = s {
                Some(m.clone())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(meshes.len(), 1, "one mesh per icon");
    let mesh = &meshes[0];
    let fill = fixture::SQUARE_HOLE.fill_mesh().expect("filled");
    let nv = fill.vertices.len();
    let ring: usize = fill.contours.iter().map(Vec::len).sum();
    assert_eq!(mesh.vertices.len(), nv + 2 * ring);
    assert_eq!(
        mesh.indices.len(),
        fill.indices.len() + 6 * ring,
        "two fringe triangles per edge"
    );
    assert!(mesh.vertices[..nv].iter().all(|v| v.color == Color32::RED));
    let k = mesh.vertices[nv..]
        .chunks(2)
        .position(|p| p[0].pos == pos2(8.0, 8.0))
        .expect("the corner is a contour vertex");
    let (inner, outer) = (mesh.vertices[nv + 2 * k], mesh.vertices[nv + 2 * k + 1]);
    assert_eq!(inner.color, Color32::RED);
    assert_eq!(outer.color, Color32::TRANSPARENT);
    assert!(
        (outer.pos - pos2(7.0, 7.0)).length() < 1e-3,
        "one pixel out along the miter: {:?}",
        outer.pos
    );
}

#[test]
fn fill_is_painted_before_strokes() {
    let shapes = painted(
        &fixture::MIXED,
        Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0)),
    );
    let mesh = shapes
        .iter()
        .position(|s| matches!(s, Shape::Mesh(_)))
        .expect("the fill");
    let path = shapes
        .iter()
        .position(|s| matches!(s, Shape::Path(_)))
        .expect("the stroke");
    assert!(mesh < path, "fill first, then strokes: {shapes:?}");
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
        painted(&icon, Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0)));
    }
}

/// Review Focus 4: a translucent filled icon shows no darker seam where the
/// fill meets the fringe (nor where the even-odd star's arms touch) and no
/// lighter gap inside, also at a fractional scale. `mixed` is painted with
/// stroke width 0, its dot alone: translucent strokes darken where their cap
/// discs overlap the line, which is the stroke painter's, not the fill's.
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
