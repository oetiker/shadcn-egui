use icongen::blob::{decode, encode, COORD_SCALE, MITER_SCALE};
use icongen::{convert, measure, parse_slots, render_crate, Fill, Seg};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}.svg",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture")
}

fn fill_of(name: &str) -> Fill {
    convert(name, &fixture(name))
        .expect("converts")
        .fill
        .expect("has a fill")
}

fn tri(f: &Fill, t: &[u16]) -> [[f32; 2]; 3] {
    [t[0], t[1], t[2]].map(|i| f.vertices[i as usize])
}

fn area(f: &Fill) -> f32 {
    f.indices
        .chunks(3)
        .map(|t| {
            let [a, b, c] = tri(f, t);
            ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
        })
        .sum()
}

fn covers(f: &Fill, p: [f32; 2]) -> bool {
    f.indices.chunks(3).any(|t| {
        let [a, b, c] = tri(f, t);
        let s = |u: [f32; 2], v: [f32; 2]| {
            (v[0] - u[0]) * (p[1] - u[1]) - (p[0] - u[0]) * (v[1] - u[1])
        };
        let (d1, d2, d3) = (s(a, b), s(b, c), s(c, a));
        !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
    })
}

fn miter_at(f: &Fill, p: [f32; 2]) -> [f32; 2] {
    f.contours
        .iter()
        .flatten()
        .find(|(q, _)| *q == p)
        .map(|(_, m)| *m)
        .unwrap_or_else(|| panic!("no contour point at {p:?}"))
}

fn close(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3
}

#[test]
fn square_with_hole_meshes_its_area() {
    let f = fill_of("square-hole");
    assert!((area(&f) - 220.0).abs() < 0.01, "area {}", area(&f));
    assert!(covers(&f, [6.0, 6.0]));
    assert!(!covers(&f, [12.0, 12.0]), "the hole stays empty");
    assert_eq!(f.contours.len(), 2);
}

#[test]
fn fill_rule_decides_the_star_centre() {
    let even = fill_of("star-evenodd");
    let nonzero = fill_of("star-nonzero");
    assert!(
        !covers(&even, [12.0, 12.0]),
        "even-odd leaves the centre empty"
    );
    assert!(covers(&nonzero, [12.0, 12.0]), "non-zero fills the centre");
    assert!(
        covers(&even, [12.0, 5.0]) && covers(&nonzero, [12.0, 5.0]),
        "tips are filled under both"
    );
}

#[test]
fn reversed_contour_is_a_hole_under_nonzero() {
    let f = fill_of("ring");
    assert!(covers(&f, [12.0, 4.0]));
    assert!(!covers(&f, [12.0, 12.0]));
}

#[test]
fn miters_point_away_from_the_fill() {
    let f = fill_of("square-hole");
    assert!(
        close(miter_at(&f, [4.0, 4.0]), [-1.0, -1.0]),
        "outer corner points out"
    );
    assert!(
        close(miter_at(&f, [9.0, 9.0]), [1.0, 1.0]),
        "hole corner points into the hole"
    );
    let f = fill_of("ring");
    assert!(
        close(miter_at(&f, [8.0, 8.0]), [1.0, 1.0]),
        "non-zero hole corner points into the hole"
    );
}

#[test]
fn edges_with_fill_on_both_sides_get_no_fringe() {
    let f = fill_of("overlap");
    assert!(
        f.contours.iter().flatten().all(|(p, _)| *p != [8.0, 8.0]),
        "both edges at (8, 8) lie inside the first square: {:?}",
        f.contours
    );
    assert!(close(miter_at(&f, [22.0, 22.0]), [1.0, 1.0]));
    assert!(close(miter_at(&f, [2.0, 2.0]), [-1.0, -1.0]));
}

/// Every fringe vertex and every fringe edge points out of the painted fill:
/// a little along a miter is uncovered; a little to the outer side of each
/// edge is uncovered and a little to its inner side is covered. Vertices
/// where loops touch (the even-odd star's tips meet at points) are left out
/// of the vertex check: their miter points between two fill wedges.
fn assert_fringe_points_out(name: &str) {
    let f = fill_of(name);
    assert!(!f.contours.is_empty(), "{name}: no fringe");
    let points: Vec<[f32; 2]> = f.contours.iter().flatten().map(|(p, _)| *p).collect();
    let pinch = |p: [f32; 2]| points.iter().filter(|q| **q == p).count() > 1;
    for c in &f.contours {
        for (i, &(p, m)) in c.iter().enumerate() {
            let len = m[0].hypot(m[1]);
            assert!((0.99..=2.01).contains(&len), "{name}: miter {m:?} at {p:?}");
            let along = [p[0] + m[0] * 0.05, p[1] + m[1] * 0.05];
            assert!(
                pinch(p) || !covers(&f, along),
                "{name}: miter {m:?} at {p:?} points into the fill"
            );
            let q = c[(i + 1) % c.len()].0;
            let d = [q[0] - p[0], q[1] - p[1]];
            let len = d[0].hypot(d[1]);
            let n = [d[1] / len, -d[0] / len];
            let mid = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0];
            assert!(
                !covers(&f, [mid[0] + n[0] * 0.01, mid[1] + n[1] * 0.01]),
                "{name}: edge {p:?}-{q:?} has fill outside"
            );
            assert!(
                covers(&f, [mid[0] - n[0] * 0.01, mid[1] - n[1] * 0.01]),
                "{name}: edge {p:?}-{q:?} has no fill inside"
            );
        }
    }
}

fn perimeter(f: &Fill) -> f32 {
    f.contours
        .iter()
        .map(|c| {
            (0..c.len())
                .map(|i| {
                    let (p, q) = (c[i].0, c[(i + 1) % c.len()].0);
                    (q[0] - p[0]).hypot(q[1] - p[1])
                })
                .sum::<f32>()
        })
        .sum()
}

#[test]
fn fringe_follows_the_true_fill_boundary() {
    for name in [
        "star-nonzero",
        "star-evenodd",
        "overlap",
        "square-hole",
        "ring",
        "mixed",
        "big-box",
    ] {
        assert_fringe_points_out(name);
    }
    // Overlap: the union's outline, including the pieces x=20..22 at y=8 and
    // y=20..22 at x=8 that lie on edges whose midpoints are inside the other square.
    let f = fill_of("overlap");
    assert_eq!(f.contours.len(), 1, "{:?}", f.contours);
    assert_eq!(f.contours[0].len(), 8, "{:?}", f.contours);
    assert!(
        (perimeter(&f) - 80.0).abs() < 1e-3,
        "perimeter {}",
        perimeter(&f)
    );
    assert!(
        close(miter_at(&f, [20.0, 8.0]), [1.0, -1.0]),
        "concave corner"
    );
    assert!(
        close(miter_at(&f, [8.0, 20.0]), [-1.0, 1.0]),
        "concave corner"
    );
    // Non-zero star: one outline of five tips and five notches.
    let f = fill_of("star-nonzero");
    assert_eq!(f.contours.len(), 1, "{:?}", f.contours);
    assert_eq!(f.contours[0].len(), 10, "{:?}", f.contours);
    // Even-odd star: the empty pentagon leaves five triangles that touch at
    // their corners.
    let f = fill_of("star-evenodd");
    assert_eq!(f.contours.len(), 5, "{:?}", f.contours);
    assert!(f.contours.iter().all(|c| c.len() == 3), "{:?}", f.contours);
    assert!(
        miter_at(&f, [12.0, 2.0])[1] < -0.9,
        "the top tip points up: {:?}",
        miter_at(&f, [12.0, 2.0])
    );
}

#[test]
fn zero_area_subpaths_get_no_fringe() {
    let svg = |d: &str| {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><path d="{d}"/></svg>"#
        )
    };
    let err = convert("line", &svg("M12 12L16 16L20 20Z")).expect_err("nothing is painted");
    assert!(err.contains("no stroked or filled paths"), "{err}");
    let f = convert("box", &svg("M2 2H6V6H2Z M12 12L16 16L20 20Z"))
        .expect("converts")
        .fill
        .expect("the box is filled");
    assert_eq!(
        f.contours.len(),
        1,
        "only the box has a fringe: {:?}",
        f.contours
    );
    assert!(
        f.contours[0]
            .iter()
            .all(|(p, _)| p[0] <= 6.0 && p[1] <= 6.0),
        "{:?}",
        f.contours
    );
}

#[test]
fn view_box_is_normalised_to_24_units() {
    let f = fill_of("big-box");
    let xs: Vec<f32> = f.vertices.iter().flat_map(|v| *v).collect();
    let (min, max) = xs
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    assert_eq!((min, max), (3.0, 21.0));
}

#[test]
fn strokes_and_fills_mix_in_one_icon() {
    let icon = convert("mixed", &fixture("mixed")).expect("converts");
    assert_eq!(icon.segs, vec![Seg::M(4.0, 12.0), Seg::L(20.0, 12.0)]);
    let f = icon.fill.expect("the dot is a fill");
    // Flattening at 0.02 units loses a little area against the true disc.
    assert!(
        (area(&f) - std::f32::consts::PI * 4.0).abs() < 0.25,
        "disc area {}",
        area(&f)
    );
}

#[test]
fn records_round_trip_through_quantisation() {
    for name in ["mixed", "ring", "big-box", "square-hole"] {
        let icon = convert(name, &fixture(name)).expect("converts");
        let (segs, fill) = decode(&encode(&icon).expect("encodes"));
        let scale = 24.0 / icon.size;
        assert_eq!(segs.len(), icon.segs.len(), "{name}");
        for (got, want) in segs.iter().zip(&icon.segs) {
            let (g, w) = (coords(*got), coords(*want));
            assert_eq!(g.len(), w.len(), "{name}");
            for (a, b) in g.iter().zip(w) {
                assert!(
                    (a - b * scale).abs() <= 0.5 / COORD_SCALE,
                    "{name}: {got:?} vs {want:?}"
                );
            }
        }
        let (got, want) = (fill.expect("fill survives"), icon.fill.expect("fill"));
        assert_eq!(
            got.vertices, want.vertices,
            "{name}: vertices are already quantised"
        );
        assert_eq!(got.indices, want.indices, "{name}");
        for (gc, wc) in got.contours.iter().zip(&want.contours) {
            for ((gp, gm), (wp, wm)) in gc.iter().zip(wc) {
                assert_eq!(gp, wp, "{name}");
                assert!(
                    (gm[0] - wm[0]).abs() <= 0.5 / MITER_SCALE
                        && (gm[1] - wm[1]).abs() <= 0.5 / MITER_SCALE,
                    "{name}"
                );
            }
        }
    }
}

fn coords(s: Seg) -> Vec<f32> {
    match s {
        Seg::M(x, y) | Seg::L(x, y) => vec![x, y],
        Seg::Q(a, b, x, y) => vec![a, b, x, y],
        Seg::C(a, b, c, d, x, y) => vec![a, b, c, d, x, y],
        Seg::Z => vec![],
    }
}

#[test]
fn stroke_only_icons_have_an_all_zero_fill_section() {
    let icon = convert("x", &fixture("x")).expect("converts");
    let rec = encode(&icon).expect("encodes");
    assert_eq!(&rec[rec.len() - 6..], &[0, 0, 0, 0, 0, 0]);
    assert_eq!(decode(&rec).1, None);
}

#[test]
fn coordinates_outside_the_range_are_errors() {
    let icon = convert("far", &fixture("far")).expect("converts");
    let err = encode(&icon).expect_err("40 units does not fit an i16 at 1/1024");
    assert!(err.contains("far") && err.contains("outside"), "{err}");
}

#[test]
fn crate_source_has_consts_all_and_slots() {
    let icons: Vec<_> = ["x", "ring", "mixed"]
        .iter()
        .map(|n| convert(n, &fixture(n)).expect("converts"))
        .collect();
    let slots = parse_slots("check=mixed,chevron_down=ring,close=x").expect("parses");
    let (src, blob) = render_crate(&icons, &slots, "Test set 1.0 (MIT)").expect("renders");
    assert!(src.starts_with("// Generated by icongen"), "{src}");
    assert!(src.contains("//! Test set 1.0 (MIT)\n"), "{src}");
    assert!(
        src.contains("const BLOB: &[u8] = include_bytes!(\"icons.bin\");"),
        "{src}"
    );
    // Sorted by name: mixed, ring, x.
    let mixed_len = encode(&icons[2]).expect("encodes").len();
    assert!(
        src.contains(&format!(
            "pub const MIXED: Icon = icon(\"mixed\", 0, {mixed_len});"
        )),
        "{src}"
    );
    assert!(
        src.contains(&format!(
            "pub const RING: Icon = icon(\"ring\", {mixed_len}, "
        )),
        "{src}"
    );
    assert!(
        src.contains("pub const ALL: &[&Icon] = &[&MIXED, &RING, &X];"),
        "{src}"
    );
    // render_crate runs its text through rustfmt, which breaks the struct literal.
    assert!(
        src.contains("pub const SLOTS: IconSlots = IconSlots {\n    check: MIXED,\n    chevron_down: RING,\n    close: X,\n};\n"),
        "{src}"
    );
    let total: usize = icons
        .iter()
        .map(|i| encode(i).expect("encodes").len())
        .sum();
    assert_eq!(blob.len(), total);
}

#[test]
fn slots_must_name_existing_icons() {
    assert!(
        parse_slots("check=a,chevron_down=b").is_err(),
        "close is missing"
    );
    assert!(
        parse_slots("check=a,chevron_down=b,close=c,tick=d").is_err(),
        "unknown slot"
    );
    let icons = vec![convert("x", &fixture("x")).expect("converts")];
    let slots = parse_slots("check=x,chevron_down=x,close=nope").expect("parses");
    let err = render_crate(&icons, &slots, "").expect_err("nope is not an icon");
    assert!(err.contains("close=nope"), "{err}");
}

#[test]
fn measure_reports_count_and_bytes() {
    let icons: Vec<_> = ["x", "ring"]
        .iter()
        .map(|n| convert(n, &fixture(n)).expect("converts"))
        .collect();
    let report = measure(&icons).expect("measures");
    let total: usize = icons
        .iter()
        .map(|i| encode(i).expect("encodes").len())
        .sum();
    assert!(report.starts_with("icons: 2\n"), "{report}");
    assert!(
        report.contains(&format!("blob bytes: {total} ")),
        "{report}"
    );
    assert!(
        report.contains("largest: ring "),
        "the ring outweighs the cross: {report}"
    );
}

#[test]
fn hairline_spike_beside_a_shallow_edge_keeps_the_outline_closed() {
    // Phosphor's mask-happy has this shape: an edge runs down to a spike tip
    // one quantum long, comes back, and leaves along an edge only 0.4 degrees
    // off the vertical. A probe a fixed distance beside the spike lands past
    // that edge and breaks the outline.
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><path d="M10 4L10 12.0009765625L10 12L9.9453125 20L4 20L4 4Z"/></svg>"#;
    let f = convert("spike", svg)
        .expect("converts")
        .fill
        .expect("filled");
    assert_eq!(f.contours.len(), 1, "{:?}", f.contours);
    assert!(
        f.contours[0]
            .iter()
            .all(|(p, _)| *p != [10.0, 12.0 + 1.0 / 1024.0]),
        "the spike tip encloses nothing: {:?}",
        f.contours
    );
}
