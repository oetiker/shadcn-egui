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
/// where loops touch (the even-odd star's tips meet at points) have a zero
/// miter: any direction there leads into one fill wedge or the other.
fn assert_fringe_points_out(name: &str) {
    let f = fill_of(name);
    assert!(!f.contours.is_empty(), "{name}: no fringe");
    let points: Vec<[f32; 2]> = f.contours.iter().flatten().map(|(p, _)| *p).collect();
    let pinch = |p: [f32; 2]| points.iter().filter(|q| **q == p).count() > 1;
    for c in &f.contours {
        for (i, &(p, m)) in c.iter().enumerate() {
            if pinch(p) {
                assert_eq!(m, [0.0, 0.0], "{name}: pinch {p:?} has a miter");
            } else {
                let len = m[0].hypot(m[1]);
                assert!((0.99..=2.01).contains(&len), "{name}: miter {m:?} at {p:?}");
                let along = [p[0] + m[0] * 0.05, p[1] + m[1] * 0.05];
                assert!(
                    !covers(&f, along),
                    "{name}: miter {m:?} at {p:?} points into the fill"
                );
            }
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

/// Where two loops of the even-odd star touch, the arms are vertical angles:
/// an outward miter of one arm points into the other, and its fringe would
/// cover that arm's fill a second time. The five inner pinches get none.
#[test]
fn pinch_vertices_get_a_zero_miter() {
    let f = fill_of("star-evenodd");
    let points: Vec<[f32; 2]> = f.contours.iter().flatten().map(|(p, _)| *p).collect();
    let mut pinches: Vec<[f32; 2]> = points
        .iter()
        .copied()
        .filter(|p| points.iter().filter(|q| *q == p).count() > 1)
        .collect();
    pinches.sort_by(|a, b| a.partial_cmp(b).unwrap());
    pinches.dedup();
    assert_eq!(pinches.len(), 5, "{pinches:?}");
    for c in &f.contours {
        for &(p, m) in c {
            let zero = m == [0.0, 0.0];
            assert_eq!(zero, pinches.contains(&p), "miter {m:?} at {p:?}");
        }
    }
    assert_fringe_points_out("star-evenodd");
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
        let (segs, fill, _) = decode(&encode(&icon).expect("encodes"));
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
    let slots = parse_slots(&fixture_slots("mixed", "ring", "x")).expect("parses");
    let (src, blob) = render_crate(&icons, &slots, "Test set 1.0 (MIT)").expect("renders");
    assert!(src.starts_with("// Generated by icongen"), "{src}");
    assert!(src.contains("//! Test set 1.0 (MIT)\n"), "{src}");
    // A static, not a const: one copy of the blob at one address in every
    // crate that uses the icons.
    assert!(
        src.contains(&format!(
            "static ICONS_BIN: [u8; {}] = *include_bytes!(\"icons.bin\");",
            blob.len()
        )),
        "{src}"
    );
    assert!(src.contains("blob: &ICONS_BIN,"), "{src}");
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
    // `IconSlots` is `#[non_exhaustive]`: built from `all`, slot by slot.
    assert!(
        src.contains("pub const SLOTS: IconSlots = {\n    let mut s = IconSlots::all(MIXED);\n    s.check = MIXED;\n    s.select_trigger = RING;\n"),
        "{src}"
    );
    assert!(src.contains("    s.close = X;\n    s\n};\n"), "{src}");
    let total: usize = icons
        .iter()
        .map(|i| encode(i).expect("encodes").len())
        .sum();
    assert_eq!(blob.len(), total);
}

/// `--slots` for every slot: `check`, `close` and the rest as `other`.
fn fixture_slots(check: &str, other: &str, close: &str) -> String {
    icongen::SLOT_NAMES
        .iter()
        .map(|s| match *s {
            "check" => format!("check={check}"),
            "close" => format!("close={close}"),
            s => format!("{s}={other}"),
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn slots_must_name_existing_icons() {
    assert!(
        parse_slots("check=a,select_trigger=b").is_err(),
        "slots are missing"
    );
    let all = fixture_slots("a", "b", "c");
    assert!(parse_slots(&all).is_ok());
    assert!(
        parse_slots(&format!("{all},tick=d")).is_err(),
        "unknown slot"
    );
    assert!(
        parse_slots(&format!("{all},check=d")).is_err(),
        "slot given twice"
    );
    let icons = vec![convert("x", &fixture("x")).expect("converts")];
    let slots = parse_slots(&fixture_slots("x", "x", "nope")).expect("parses");
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

fn svg_of(paths: &[(&str, &str)]) -> String {
    let mut s = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24">"#);
    for (d, rule) in paths {
        s += &format!(r#"<path fill-rule="{rule}" d="{d}"/>"#);
    }
    s + "</svg>"
}

/// Every loop has at least three points and every miter is 1..2 long, or
/// zero where loops touch.
fn assert_loops_sound(name: &str, f: &Fill) {
    let points: Vec<[f32; 2]> = f.contours.iter().flatten().map(|(p, _)| *p).collect();
    let pinch = |p: &[f32; 2]| points.iter().filter(|q| *q == p).count() > 1;
    for c in &f.contours {
        assert!(c.len() >= 3, "{name}: {c:?}");
        for (p, m) in c {
            if pinch(p) {
                assert_eq!(*m, [0.0, 0.0], "{name}: pinch {p:?} has a miter");
            } else {
                let len = m[0].hypot(m[1]);
                assert!((0.99..=2.01).contains(&len), "{name}: miter {m:?} at {p:?}");
            }
        }
    }
}

#[test]
fn crossing_two_collinear_edges_off_the_grid_closes() {
    // Two squares share the line y = 2; the triangle crosses it at an x that
    // no f32 or f64 holds exactly, once per square edge.
    let svg = svg_of(&[
        ("M2 2H12V12H2Z", "nonzero"),
        ("M6 2H16V12H6Z", "nonzero"),
        ("M7.1 0.3L8.2 3.1L10.3 0.2Z", "nonzero"),
    ]);
    let f = convert("stacked", &svg)
        .expect("converts")
        .fill
        .expect("filled");
    assert_loops_sound("stacked", &f);
    assert_eq!(f.contours.len(), 1, "{:?}", f.contours);
    // One path whose edges (16,24)-(0,8) and (0,8)-(4,12) lie on one line.
    let svg = svg_of(&[("M16 24L0 8L4 12L0 4L4 20Z", "nonzero")]);
    let f = convert("folded", &svg)
        .expect("converts")
        .fill
        .expect("filled");
    assert_loops_sound("folded", &f);
}

#[test]
fn rays_through_vertices_and_along_edges_classify_exactly() {
    // From the fuzz run below with other seeds. In the first, the side ray of
    // the edge (20,12)-(16,0) runs through (0,12) and along (0,12)-(12,8).
    // In the second, the line of a side ray holds the edge (24,0)-(6,6)
    // behind the ray's start.
    let icons = [
        vec![(
            "M20 12L16 0L0 8L0 20L20 20L12 8ZM12 8L0 24L16 4L8 20L0 12Z",
            "nonzero",
        )],
        vec![
            ("M0 0L0 12L0 0L6 18L12 0L6 0Z", "evenodd"),
            (
                "M12 6L24 0L6 6L18 12L0 24ZM0 12L0 24L24 6L24 24Z",
                "evenodd",
            ),
        ],
    ];
    for (i, paths) in icons.iter().enumerate() {
        let name = format!("ray{i}");
        let f = convert(&name, &svg_of(paths))
            .expect("converts")
            .fill
            .expect("filled");
        assert_loops_sound(&name, &f);
    }
}

#[test]
fn random_grid_icons_all_close() {
    // Random polygons on coarse grids: shared vertices, collinear overlaps,
    // rays through vertices. Fixed xorshift seed, so failures reproduce.
    let mut seed: u64 = 0x1234_5678;
    let mut rnd = |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    };
    for it in 0..400 {
        let grid = if it % 2 == 0 { 7 } else { 25 };
        let mut paths = Vec::new();
        for _ in 0..1 + rnd(3) {
            let mut d = String::new();
            for _ in 0..1 + rnd(2) {
                for k in 0..3 + rnd(4) {
                    let mut v = || rnd(grid) as f64 * 24.0 / (grid - 1) as f64;
                    let (x, y) = (v(), v());
                    d += &format!("{}{x:.4} {y:.4}", if k == 0 { "M" } else { "L" });
                }
                d += "Z";
            }
            paths.push((d, if rnd(2) == 0 { "nonzero" } else { "evenodd" }));
        }
        let refs: Vec<(&str, &str)> = paths.iter().map(|(d, r)| (d.as_str(), *r)).collect();
        let svg = svg_of(&refs);
        let name = format!("fuzz{it}");
        match convert(&name, &svg) {
            Ok(icon) => {
                if let Some(f) = icon.fill {
                    assert_loops_sound(&name, &f);
                }
            }
            Err(e) => assert!(e.contains("no stroked or filled paths"), "{e}\n{svg}"),
        }
    }
}

#[test]
fn an_icon_named_blob_does_not_clash_with_the_blob_static() {
    let icons = vec![convert("blob", &fixture("x")).expect("converts")];
    let slots = parse_slots(&fixture_slots("blob", "blob", "blob")).expect("parses");
    let (src, _) = render_crate(&icons, &slots, "Test set 1.0 (MIT)").expect("renders");
    assert!(
        src.contains("pub const BLOB: Icon = icon(\"blob\", 0,"),
        "{src}"
    );
    assert!(!src.contains("static BLOB"), "{src}");
}
