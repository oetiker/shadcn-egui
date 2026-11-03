//! Every stroked layer of every bundled icon set against an independent
//! reference: the union of round-capped capsules (a stroke with round caps
//! and round joins is exactly that union) and the fill's triangles, on the
//! same flattened points, sampled on many sub-rows per pixel with the x
//! extent of each sub-row exact. So only the stroke raster differs, and a
//! lost join, cap or band shows as a pixel far off. The fill alone is
//! checked by icongen's `fill_scan`.
//!
//! Every icon is checked at 16 and 24 px; an icon with a reversal (a
//! sub-path turning back by 179° or more, where the round join is hardest
//! to keep) also at 64 px, since the limit holds at every size.
//!
//! The reference's own error is the sub-row sampling: an edge along a row
//! is off by up to half a sub-row, 255 / (2 x SUB) = 1 level, and a pixel
//! crossed by two such edges twice that.

use super::{layer_coverage, stroke_paths, Place};
use shadcn_egui_core::Icon;

/// Sub-rows per pixel of the reference.
const SUB: usize = 128;
/// The levels of 255 a pixel may differ from the reference: its sampling
/// (2 levels at worst) and the raster's rounding, with room to spare; a
/// lost piece of area is tens to hundreds of levels.
const LEVELS: f64 = 4.0;
const SIDES: [usize; 2] = [16, 24];
/// The extra size for icons with a reversal.
const REVERSAL_SIDE: usize = 64;

type P = [f64; 2];

/// The x extent of the capsule `p`–`q` of radius `h` on the line `y`.
fn capsule(p: P, q: P, h: f64, y: f64) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for c in [p, q] {
        let dy = y - c[1];
        if dy.abs() <= h {
            let dx = (h * h - dy * dy).sqrt();
            lo = lo.min(c[0] - dx);
            hi = hi.max(c[0] + dx);
        }
    }
    // The rectangle: 0 <= along <= len and |across| <= h, both linear in x.
    let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
    let len = dx.hypot(dy);
    if len > 0.0 {
        let (ux, uy) = (dx / len, dy / len);
        let (mut a, mut b) = (f64::NEG_INFINITY, f64::INFINITY);
        let ry = y - p[1];
        // coef * (x - p.x) + off within min..=max
        for (coef, off, min, max) in [(ux, ry * uy, 0.0, len), (-uy, ry * ux, -h, h)] {
            if coef.abs() < 1e-12 {
                if off < min || off > max {
                    return (lo <= hi).then_some((lo, hi));
                }
            } else {
                let (u, v) = ((min - off) / coef, (max - off) / coef);
                a = a.max(u.min(v) + p[0]);
                b = b.min(u.max(v) + p[0]);
            }
        }
        if a <= b {
            lo = lo.min(a);
            hi = hi.max(b);
        }
    }
    (lo <= hi).then_some((lo, hi))
}

/// The x extent of the triangle `t` on the line `y`.
fn triangle(t: &[P; 3], y: f64) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for k in 0..3 {
        let (a, b) = (t[k], t[(k + 1) % 3]);
        if (a[1] <= y && y < b[1]) || (b[1] <= y && y < a[1]) {
            let x = a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            lo = lo.min(x);
            hi = hi.max(x);
        }
    }
    (lo < hi).then_some((lo, hi))
}

/// The reference coverage (0..=1) of capsules of radius `h` and
/// triangles on a `side`² grid.
fn reference(caps: &[(P, P)], tris: &[[P; 3]], h: f64, side: usize) -> Vec<f64> {
    let mut cov = vec![0.0; side * side];
    let mut iv: Vec<(f64, f64)> = Vec::new();
    for row in 0..side {
        let (r0, r1) = (row as f64, row as f64 + 1.0);
        let near: Vec<&(P, P)> = caps
            .iter()
            .filter(|(p, q)| p[1].min(q[1]) - h < r1 && p[1].max(q[1]) + h > r0)
            .collect();
        let near_tris: Vec<&[P; 3]> = tris
            .iter()
            .filter(|t| {
                let (lo, hi) = t.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| {
                    (a.0.min(p[1]), a.1.max(p[1]))
                });
                lo < r1 && hi > r0
            })
            .collect();
        for s in 0..SUB {
            let y = r0 + (s as f64 + 0.5) / SUB as f64;
            iv.clear();
            iv.extend(near.iter().filter_map(|(p, q)| capsule(*p, *q, h, y)));
            iv.extend(near_tris.iter().filter_map(|t| triangle(t, y)));
            iv.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut merged: Vec<(f64, f64)> = Vec::new();
            for &(a, b) in &iv {
                match merged.last_mut() {
                    Some(m) if a <= m.1 => m.1 = m.1.max(b),
                    _ => merged.push((a, b)),
                }
            }
            let line = &mut cov[row * side..(row + 1) * side];
            for (a, b) in merged {
                let (a, b) = (a.max(0.0), b.min(side as f64));
                if b <= a {
                    continue;
                }
                for (k, c) in line
                    .iter_mut()
                    .enumerate()
                    .take(b.ceil() as usize)
                    .skip(a.floor() as usize)
                {
                    let o = b.min(k as f64 + 1.0) - a.max(k as f64);
                    *c += o.max(0.0) / SUB as f64;
                }
            }
        }
    }
    cov
}

/// Whether the sub-path `pts` turns back by 179° or more at a joint.
fn has_reversal(pts: &[[f32; 2]], closed: bool) -> bool {
    let mut d: Vec<[f64; 2]> = Vec::new();
    let mut piece = |a: [f32; 2], b: [f32; 2]| {
        let v = [f64::from(b[0] - a[0]), f64::from(b[1] - a[1])];
        let l = v[0].hypot(v[1]);
        if l > 0.0 {
            d.push([v[0] / l, v[1] / l]);
        }
    };
    for w in pts.windows(2) {
        piece(w[0], w[1]);
    }
    if closed && pts.len() > 1 {
        piece(pts[pts.len() - 1], pts[0]);
    }
    let limit = 179f64.to_radians().cos();
    let joints = if closed {
        d.len()
    } else {
        d.len().saturating_sub(1)
    };
    (0..joints).any(|k| {
        let (a, b) = (d[k], d[(k + 1) % d.len()]);
        a[0] * b[0] + a[1] * b[1] <= limit
    })
}

/// Whether a stroked sub-path of `icon` (flattened at 24 px) has a
/// reversal.
fn reversed(icon: &Icon) -> bool {
    let place = Place::scaled(24.0 / icon.size);
    icon.layers().any(|layer| {
        stroke_paths(layer.segs.clone(), place)
            .iter()
            .any(|(p, c)| has_reversal(p, *c))
    })
}

/// Checks every stroked layer of `all`; returns the failures, the number
/// of icons with a reversal and the worst difference in levels.
fn scan(all: &[&Icon]) -> (Vec<String>, usize, f64) {
    let mut fails = Vec::new();
    let mut reversals = 0;
    let mut overall = 0.0f64;
    for icon in all {
        let reversed = reversed(icon);
        let extra = reversed.then_some(REVERSAL_SIDE);
        for side in SIDES.into_iter().chain(extra) {
            let scale = side as f32 / icon.size;
            let place = Place::scaled(scale);
            let stroke_px = 2.0 * scale;
            let h = f64::from(stroke_px) / 2.0;
            let mut worst = (0.0f64, 0usize);
            for layer in icon.layers() {
                let paths = stroke_paths(layer.segs.clone(), place);
                if paths.is_empty() {
                    continue;
                }
                let f = |p: [f32; 2]| [f64::from(p[0]), f64::from(p[1])];
                let mut caps = Vec::new();
                for (pts, closed) in &paths {
                    if pts.len() == 1 {
                        caps.push((f(pts[0]), f(pts[0])));
                    }
                    caps.extend(pts.windows(2).map(|w| (f(w[0]), f(w[1]))));
                    if *closed && pts.len() > 1 {
                        caps.push((f(pts[pts.len() - 1]), f(pts[0])));
                    }
                }
                let mut tris = Vec::new();
                if let Some(fill) = &layer.fill {
                    let v = |i: u16| {
                        let [x, y] = fill.vertices[usize::from(i)];
                        f(place.map(x, y).into())
                    };
                    for t in fill.indices.as_chunks::<3>().0 {
                        tris.push([v(t[0]), v(t[1]), v(t[2])]);
                    }
                }
                let want = reference(&caps, &tris, h, side);
                let got = layer_coverage(&layer, side, side, place, stroke_px);
                for (i, (g, w)) in got.iter().zip(&want).enumerate() {
                    let e = (f64::from(*g) - w).abs() * 255.0;
                    if e > worst.0 {
                        worst = (e, i);
                    }
                }
            }
            overall = overall.max(worst.0);
            if worst.0 > LEVELS {
                let (x, y) = (worst.1 % side, worst.1 / side);
                fails.push(format!(
                    "{} {side} px: {:.1} levels at ({x},{y})",
                    icon.name, worst.0
                ));
            }
        }
        reversals += usize::from(reversed);
    }
    (fails, reversals, overall)
}

/// Checks part `part` of `parts` of `all` (the long sets are split so the
/// test threads share them). Part 0 also checks that `reversals` icons of
/// the whole set have a reversal, so a filter that stops finding them
/// (and with it the 64 px check) fails.
fn check(set: &str, all: &[&Icon], part: usize, parts: usize, reversals: usize) {
    if part == 0 {
        let found = all.iter().filter(|i| reversed(i)).count();
        assert_eq!(found, reversals, "{set}: icons with a reversal");
    }
    let n = all.len().div_ceil(parts);
    let all = &all[(part * n).min(all.len())..((part + 1) * n).min(all.len())];
    let (fails, reversals, worst) = scan(all);
    eprintln!(
        "{set} {}/{parts}: {} icons, {reversals} with a reversal (179° or more), \
         worst {worst:.2} levels, {} failed",
        part + 1,
        all.len(),
        fails.len()
    );
    assert!(fails.is_empty(), "{set}: {fails:#?}");
}

#[test]
fn every_lucide_stroke_matches_its_capsules() {
    check("lucide", crate::icons::lucide::ALL, 0, 1, 4);
}

#[cfg(feature = "icons-tabler")]
#[test]
fn every_tabler_stroke_matches_its_capsules_1() {
    check("tabler", crate::icons::tabler::ALL, 0, 2, 101);
}

#[cfg(feature = "icons-tabler")]
#[test]
fn every_tabler_stroke_matches_its_capsules_2() {
    check("tabler", crate::icons::tabler::ALL, 1, 2, 101);
}

#[cfg(feature = "icons-hugeicons")]
#[test]
fn every_hugeicons_stroke_matches_its_capsules_1() {
    check("hugeicons", crate::icons::hugeicons::ALL, 0, 3, 69);
}

#[cfg(feature = "icons-hugeicons")]
#[test]
fn every_hugeicons_stroke_matches_its_capsules_2() {
    check("hugeicons", crate::icons::hugeicons::ALL, 1, 3, 69);
}

#[cfg(feature = "icons-hugeicons")]
#[test]
fn every_hugeicons_stroke_matches_its_capsules_3() {
    check("hugeicons", crate::icons::hugeicons::ALL, 2, 3, 69);
}

#[cfg(feature = "icons-phosphor")]
#[test]
fn every_phosphor_stroke_matches_its_capsules() {
    check("phosphor", crate::icons::phosphor::ALL, 0, 1, 0);
}

#[cfg(feature = "icons-remixicon")]
#[test]
fn every_remixicon_stroke_matches_its_capsules() {
    check("remixicon", crate::icons::remixicon::ALL, 0, 1, 0);
}
