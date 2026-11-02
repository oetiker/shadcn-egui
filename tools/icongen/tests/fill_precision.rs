//! Precision by visibility: a filled icon as the blob stores it (curves
//! flattened at `FLATTEN_TOLERANCE`, points rounded to 1/`COORD_SCALE` unit)
//! stays within 4 levels of its true outline on an opaque icon, up to
//! 256 px.
//!
//! The reference is the same icon flattened at 1e-4 units and not rounded:
//! the rounding moves the outline the user sees as much as the flattening
//! does, so the reference must not share it. Both go through one rasteriser,
//! a copy of `fill_coverage` in `crates/shadcn-egui/src/components/icon_raster.rs`
//! (inside by the triangles, coverage ramping over one pixel by the distance
//! to the fringe contours), so only the geometry differs.

use icongen::{convert, convert_with, Fill, Precision};

/// Filled icons with true curves: Phosphor's arcs (large and small radii),
/// Remix's circle cubics, Hugeicons' rounded corners.
const ICONS: [&str; 6] = [
    "phosphor-circle",
    "phosphor-record",
    "phosphor-spinner",
    "remix-checkbox-blank-circle-fill",
    "remix-record-circle-fill",
    "hugeicons-clipboard-check",
];
const SIDES: [usize; 10] = [16, 20, 24, 32, 48, 64, 96, 128, 192, 256];
/// The levels an opaque edge may differ from the exact one.
const LEVELS: u8 = 4;
const REFERENCE: Precision = Precision {
    tolerance: 1e-4,
    quantise: false,
};

fn fill(name: &str, prec: Precision) -> Fill {
    let path = format!("{}/tests/fixtures/{name}.svg", env!("CARGO_MANIFEST_DIR"));
    let svg = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    convert_with(name, &svg, prec)
        .unwrap_or_else(|e| panic!("{e}"))
        .fill
        .unwrap_or_else(|| panic!("{name} has no fill"))
}

const HALF_PX: f32 = 0.5;
const EDGE_SLACK_PX: f32 = 1e-3;

/// Coverage (0..=255) of `fill` on a `side`² grid; the 24-unit box fills it.
fn coverage(fill: &Fill, side: usize) -> Vec<u8> {
    let scale = side as f32 / 24.0;
    let map = |[x, y]: [f32; 2]| [x * scale, y * scale];
    let (w, h) = (side, side);
    let mut inside = vec![false; w * h];
    for &t in fill.indices.as_chunks::<3>().0 {
        let tri = t.map(|i| map(fill.vertices[usize::from(i)]));
        mark_triangle(&mut inside, w, h, tri);
    }
    let mut dist = vec![f32::INFINITY; w * h];
    for contour in &fill.contours {
        let pts: Vec<[f32; 2]> = contour.iter().map(|&(p, _)| map(p)).collect();
        let mut prev = *pts.last().expect("non-empty contour");
        for &p in &pts {
            min_distance(&mut dist, w, h, prev, p, HALF_PX);
            prev = p;
        }
    }
    dist.iter()
        .zip(&inside)
        .map(|(d, &inn)| {
            let f = if inn { HALF_PX + d } else { HALF_PX - d };
            (f.clamp(0.0, 1.0) * 255.0).round() as u8
        })
        .collect()
}

fn mark_triangle(inside: &mut [bool], w: usize, h: usize, [a, b, c]: [[f32; 2]; 3]) {
    let sub = |p: [f32; 2], q: [f32; 2]| [p[0] - q[0], p[1] - q[1]];
    let (ab, ac) = (sub(b, a), sub(c, a));
    let area2 = ab[0] * ac[1] - ac[0] * ab[1];
    if area2.abs() <= f32::EPSILON {
        return;
    }
    let sign = area2.signum();
    let edges = [(a, b), (b, c), (c, a)].map(|(p, q)| {
        let e = sub(q, p);
        (p, e, e[0].hypot(e[1]))
    });
    let xs = span(a[0].min(b[0]).min(c[0]), a[0].max(b[0]).max(c[0]), 0.0, w);
    let ys = span(a[1].min(b[1]).min(c[1]), a[1].max(b[1]).max(c[1]), 0.0, h);
    for y in ys {
        for x in xs.clone() {
            let p = centre(x, y);
            let hit = edges.iter().all(|&(o, e, len)| {
                let d = sub(p, o);
                (e[0] * d[1] - e[1] * d[0]) * sign >= -EDGE_SLACK_PX * len
            });
            if hit {
                inside[y * w + x] = true;
            }
        }
    }
}

fn min_distance(dist: &mut [f32], w: usize, h: usize, a: [f32; 2], b: [f32; 2], reach: f32) {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    for y in span(a[1].min(b[1]), a[1].max(b[1]), reach, h) {
        for x in span(a[0].min(b[0]), a[0].max(b[0]), reach, w) {
            let p = centre(x, y);
            let ap = [p[0] - a[0], p[1] - a[1]];
            let t = if len2 > 0.0 {
                ((ap[0] * ab[0] + ap[1] * ab[1]) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = (ap[0] - ab[0] * t).hypot(ap[1] - ab[1] * t);
            let slot = &mut dist[y * w + x];
            *slot = slot.min(d);
        }
    }
}

fn span(lo: f32, hi: f32, reach: f32, n: usize) -> std::ops::Range<usize> {
    let first = (lo - reach - HALF_PX).ceil().max(0.0);
    let end = (hi + reach - HALF_PX).floor() + 1.0;
    (first as usize).min(n)..(end.max(0.0) as usize).min(n)
}

fn centre(x: usize, y: usize) -> [f32; 2] {
    [x as f32 + HALF_PX, y as f32 + HALF_PX]
}

/// The worst difference, in levels of 255, between `painted` and `exact`.
fn worst(painted: &Fill, exact: &Fill, side: usize) -> u8 {
    coverage(painted, side)
        .iter()
        .zip(coverage(exact, side))
        .map(|(a, b)| a.abs_diff(b))
        .max()
        .unwrap_or(0)
}

#[test]
fn a_blob_fill_is_within_four_levels_of_the_true_outline() {
    let mut fails = Vec::new();
    for name in ICONS {
        let painted = fill(name, Precision::BLOB);
        let exact = fill(name, REFERENCE);
        for side in SIDES {
            let e = worst(&painted, &exact, side);
            eprintln!("FILL {name} {side}: {e}");
            if e > LEVELS {
                fails.push(format!("{name} {side} px: {e} levels"));
            }
        }
    }
    assert!(fails.is_empty(), "{fails:#?}");
}

/// The blob's precision is what `convert` uses.
#[test]
fn convert_flattens_at_the_blob_precision() {
    let path = format!(
        "{}/tests/fixtures/phosphor-record.svg",
        env!("CARGO_MANIFEST_DIR")
    );
    let svg = std::fs::read_to_string(path).expect("fixture");
    assert_eq!(
        convert("record", &svg),
        convert_with("record", &svg, Precision::BLOB)
    );
}

/// How the tolerance was chosen: the worst levels at 256 px per tolerance,
/// rounded as the blob rounds. `cargo test --test fill_precision -- --ignored --nocapture`
#[test]
#[ignore = "a measurement, not a check"]
fn tolerance_scan() {
    let exact: Vec<Fill> = ICONS.iter().map(|n| fill(n, REFERENCE)).collect();
    eprintln!("SCAN tolerance | {} | any size", ICONS.join(" | "));
    for tolerance in [
        0.02, 0.01, 0.005, 0.002, 0.0015, 0.0014, 0.0013, 0.0012, 0.001, 0.0008, 0.0005,
    ] {
        let prec = Precision {
            tolerance,
            quantise: true,
        };
        let mut all = 0;
        let at_256: Vec<String> = ICONS
            .iter()
            .zip(&exact)
            .map(|(n, e)| {
                let p = fill(n, prec);
                all = SIDES
                    .map(|s| worst(&p, e, s))
                    .into_iter()
                    .fold(all, u8::max);
                worst(&p, e, 256).to_string()
            })
            .collect();
        eprintln!("SCAN {tolerance} | {} | {all}", at_256.join(" | "));
    }
}
