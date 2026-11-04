//! Every stroked layer of every bundled icon set against an independent
//! reference: the union of simple shapes that SVG's stroke rules give
//! each sub-path, and the fill's triangles, on the same flattened points,
//! sampled on many sub-rows per pixel with the x extent of each sub-row
//! exact. The shapes are a rectangle per piece; per joint, on its outer
//! side only, the sector between the two pieces' offsets (round), the
//! triangle across them (bevel, or a miter past its limit) or that
//! triangle out to the miter's tip (miter); per open end the half disc
//! beyond it (round cap), a half-width box (square cap) or nothing (butt
//! cap). So only the stroke raster differs, and a lost join, cap or band
//! shows as a pixel far off. The fill alone is checked by icongen's
//! `fill_scan`.
//!
//! Joins and caps are the outer half only, as SVG 2 defines them and
//! browsers draw them: a whole disc at a round joint reaches past a butt
//! end next to it (a curve ending in a butt cap, where the flattened
//! pieces are short), which no renderer paints. The miter tip is taken on
//! the bisector of the outer normals, which stays exact where the two
//! pieces run almost straight on; intersecting the two offset lines does
//! not there. Both were checked against Skia's stroker (tiny-skia 0.11)
//! on the same flattened points, at 32 x 32 samples per pixel.
//!
//! Every icon is checked at 16 and 24 px; an icon with a reversal (a
//! sub-path turning back by 179° or more, where the round join is hardest
//! to keep) also at 64 px, since the limit holds at every size.
//!
//! The reference's own error is the sub-row sampling: an edge along a row
//! is off by up to half a sub-row, 255 / (2 x SUB) = 1 level, and a pixel
//! crossed by two such edges twice that.

use super::stroke::{Cap, Join, Style};
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
/// SVG's default `stroke-miterlimit`, the only one icons carry.
const MITER_LIMIT: f64 = 4.0;

type P = [f64; 2];

/// A piece of a stroke's area.
enum Shape {
    /// Centre and radius.
    Disc(P, f64),
    /// The part of a disc (centre, radius) swept from the direction `u`
    /// to the direction `v` the positive way (from x towards y), by at
    /// most half a turn.
    Sector(P, f64, P, P),
    /// A convex polygon.
    Poly(Vec<P>),
}

impl Shape {
    /// The y extent.
    fn rows(&self) -> (f64, f64) {
        match self {
            Shape::Disc(c, r) | Shape::Sector(c, r, ..) => (c[1] - r, c[1] + r),
            Shape::Poly(v) => v.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| {
                (a.0.min(p[1]), a.1.max(p[1]))
            }),
        }
    }

    /// The x extent on the line `y`.
    fn span(&self, y: f64) -> Option<(f64, f64)> {
        match self {
            Shape::Disc(c, r) => {
                let dy = y - c[1];
                (dy.abs() <= *r).then(|| {
                    let dx = (r * r - dy * dy).sqrt();
                    (c[0] - dx, c[0] + dx)
                })
            }
            Shape::Sector(c, r, u, v) => {
                let (mut lo, mut hi) = Shape::Disc(*c, *r).span(y)?;
                // Relative to the centre, q = (x - c, y - c): the sector is
                // cross(u, q) >= 0 and cross(q, v) >= 0, each a bound on x
                // (a x + b >= 0) on this line.
                let qy = y - c[1];
                for (a, b) in [(-u[1], u[0] * qy), (v[1], -v[0] * qy)] {
                    if a == 0.0 {
                        if b < 0.0 {
                            return None;
                        }
                    } else if a > 0.0 {
                        lo = lo.max(c[0] - b / a);
                    } else {
                        hi = hi.min(c[0] - b / a);
                    }
                }
                (lo < hi).then_some((lo, hi))
            }
            Shape::Poly(v) => {
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                for k in 0..v.len() {
                    let (a, b) = (v[k], v[(k + 1) % v.len()]);
                    if (a[1] <= y && y < b[1]) || (b[1] <= y && y < a[1]) {
                        let x = a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
                        lo = lo.min(x);
                        hi = hi.max(x);
                    }
                }
                (lo < hi).then_some((lo, hi))
            }
        }
    }
}

fn at(p: P, d: P, s: f64) -> P {
    [p[0] + d[0] * s, p[1] + d[1] * s]
}

/// `d` turned by 90°.
fn normal(d: P) -> P {
    [-d[1], d[0]]
}

fn cross(a: P, b: P) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

/// The shapes whose union is the stroke of half width `h` along `pts`,
/// closed when `closed`, drawn with `style`.
fn stroke_shapes(pts: &[P], closed: bool, h: f64, style: Style, out: &mut Vec<Shape>) {
    let mut pts: Vec<P> = pts.to_vec();
    if closed && pts.len() > 1 {
        pts.push(pts[0]);
    }
    // Pieces of zero length have no direction and draw nothing.
    let mut pieces: Vec<(P, P, P)> = Vec::new();
    for w in pts.windows(2) {
        let v = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
        let l = v[0].hypot(v[1]);
        if l > 0.0 {
            pieces.push((w[0], w[1], [v[0] / l, v[1] / l]));
        }
    }
    let Some(&first) = pieces.first() else {
        // A dot: SVG draws its caps, facing along x.
        let p = pts[0];
        match style.cap {
            Cap::Round => out.push(Shape::Disc(p, h)),
            Cap::Square => out.push(Shape::Poly(vec![
                [p[0] - h, p[1] - h],
                [p[0] + h, p[1] - h],
                [p[0] + h, p[1] + h],
                [p[0] - h, p[1] + h],
            ])),
            Cap::Butt => {}
        }
        return;
    };
    for &(a, b, d) in &pieces {
        let n = normal(d);
        out.push(Shape::Poly(vec![
            at(a, n, h),
            at(b, n, h),
            at(b, n, -h),
            at(a, n, -h),
        ]));
    }
    let closed = closed && pieces.len() > 1;
    let joints = if closed {
        pieces.len()
    } else {
        pieces.len() - 1
    };
    for k in 0..joints {
        let (_, j, di) = pieces[k];
        let (_, _, dj) = pieces[(k + 1) % pieces.len()];
        let (ni, nj) = (normal(di), normal(dj));
        let cos = di[0] * dj[0] + di[1] * dj[1];
        // The outer side is the one the path turns away from.
        let turn = cross(di, dj);
        if turn == 0.0 && cos >= 0.0 {
            // Straight on: the two rectangles meet edge to edge.
            continue;
        }
        if style.join == Join::Round {
            // The arc between the outer offsets; turning straight back,
            // the half disc beyond the joint.
            let (u, v) = if turn > 0.0 {
                ([-ni[0], -ni[1]], [-nj[0], -nj[1]])
            } else if turn < 0.0 {
                (nj, ni)
            } else {
                ([-ni[0], -ni[1]], ni)
            };
            out.push(Shape::Sector(j, h, u, v));
            continue;
        }
        let s = -h * turn.signum();
        let (a, b) = (at(j, ni, s), at(j, nj, s));
        // The miter is 1 / sin(angle / 2) stroke widths long, the angle
        // being the one between the two pieces at the joint.
        let angle = (-cos).clamp(-1.0, 1.0).acos();
        if style.join == Join::Miter && 1.0 / (angle / 2.0).sin() <= MITER_LIMIT {
            // Where the outer offset lines meet: on the bisector of the
            // outer normals, h / cos(turn / 2) out; |ni + nj| is
            // 2 cos(turn / 2) and 1 + cos is 2 cos²(turn / 2).
            let tip = at(j, [ni[0] + nj[0], ni[1] + nj[1]], s / (1.0 + cos));
            out.push(Shape::Poly(vec![j, a, tip, b]));
        } else {
            out.push(Shape::Poly(vec![j, a, b]));
        }
    }
    if !closed {
        let last = pieces[pieces.len() - 1];
        for (e, d) in [(first.0, [-first.2[0], -first.2[1]]), (last.1, last.2)] {
            match style.cap {
                // The half disc beyond the end.
                Cap::Round => {
                    let n = normal(d);
                    out.push(Shape::Sector(e, h, [-n[0], -n[1]], n));
                }
                Cap::Square => {
                    let n = normal(d);
                    let f = at(e, d, h);
                    out.push(Shape::Poly(vec![
                        at(e, n, h),
                        at(f, n, h),
                        at(f, n, -h),
                        at(e, n, -h),
                    ]));
                }
                Cap::Butt => {}
            }
        }
    }
}

/// The reference coverage (0..=1) of the union of `shapes` on a `side`²
/// grid.
fn reference(shapes: &[Shape], side: usize) -> Vec<f64> {
    let mut cov = vec![0.0; side * side];
    let mut iv: Vec<(f64, f64)> = Vec::new();
    let rows: Vec<(f64, f64)> = shapes.iter().map(Shape::rows).collect();
    for row in 0..side {
        let (r0, r1) = (row as f64, row as f64 + 1.0);
        let near: Vec<&Shape> = shapes
            .iter()
            .zip(&rows)
            .filter(|(_, (lo, hi))| *lo < r1 && *hi > r0)
            .map(|(s, _)| s)
            .collect();
        for s in 0..SUB {
            let y = r0 + (s as f64 + 0.5) / SUB as f64;
            iv.clear();
            iv.extend(near.iter().filter_map(|s| s.span(y)));
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
            .any(|(p, c, _)| has_reversal(p, *c))
    })
}

/// Whether a stroked sub-path of `icon` has a cap or join that is not
/// round.
fn styled(icon: &Icon) -> bool {
    let place = Place::scaled(24.0 / icon.size);
    icon.layers().any(|layer| {
        stroke_paths(layer.segs.clone(), place)
            .iter()
            .any(|(.., s)| *s != Style::ROUND)
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
                let mut shapes = Vec::new();
                for (pts, closed, style) in &paths {
                    let pts: Vec<P> = pts.iter().map(|&p| f(p)).collect();
                    stroke_shapes(&pts, *closed, h, *style, &mut shapes);
                }
                if let Some(fill) = &layer.fill {
                    let v = |i: u16| {
                        let [x, y] = fill.vertices[usize::from(i)];
                        f(place.map(x, y).into())
                    };
                    for t in fill.indices.as_chunks::<3>().0 {
                        shapes.push(Shape::Poly(vec![v(t[0]), v(t[1]), v(t[2])]));
                    }
                }
                let want = reference(&shapes, side);
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
/// (and with it the 64 px check) fails, and that `styles` icons have a cap
/// or join that is not round, so styles lost on the way to the raster
/// fail too.
fn check(set: &str, all: &[&Icon], part: usize, parts: usize, (reversals, styles): (usize, usize)) {
    if part == 0 {
        let found = all.iter().filter(|i| reversed(i)).count();
        assert_eq!(found, reversals, "{set}: icons with a reversal");
        let found = all.iter().filter(|i| styled(i)).count();
        assert_eq!(found, styles, "{set}: icons with caps or joins not round");
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
fn every_lucide_stroke_matches_its_shapes() {
    check("lucide", crate::icons::lucide::ALL, 0, 1, (4, 0));
}

#[cfg(feature = "icons-tabler")]
#[test]
fn every_tabler_stroke_matches_its_shapes_1() {
    check("tabler", crate::icons::tabler::ALL, 0, 2, (101, 0));
}

#[cfg(feature = "icons-tabler")]
#[test]
fn every_tabler_stroke_matches_its_shapes_2() {
    check("tabler", crate::icons::tabler::ALL, 1, 2, (101, 0));
}

#[cfg(feature = "icons-hugeicons")]
#[test]
fn every_hugeicons_stroke_matches_its_shapes_1() {
    check("hugeicons", crate::icons::hugeicons::ALL, 0, 3, (69, 2820));
}

#[cfg(feature = "icons-hugeicons")]
#[test]
fn every_hugeicons_stroke_matches_its_shapes_2() {
    check("hugeicons", crate::icons::hugeicons::ALL, 1, 3, (69, 2820));
}

#[cfg(feature = "icons-hugeicons")]
#[test]
fn every_hugeicons_stroke_matches_its_shapes_3() {
    check("hugeicons", crate::icons::hugeicons::ALL, 2, 3, (69, 2820));
}

#[cfg(feature = "icons-phosphor")]
#[test]
fn every_phosphor_stroke_matches_its_shapes() {
    check("phosphor", crate::icons::phosphor::ALL, 0, 1, (0, 0));
}

#[cfg(feature = "icons-remixicon")]
#[test]
fn every_remixicon_stroke_matches_its_shapes() {
    check("remixicon", crate::icons::remixicon::ALL, 0, 1, (0, 0));
}
