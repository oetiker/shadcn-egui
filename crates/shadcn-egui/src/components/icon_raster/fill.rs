//! The fill part of the icon rasteriser: each pixel gets the exact area of
//! it that the fill's triangles cover. Plain `[f32; 2]` points and no
//! dependencies, so icongen's tests include this file to measure a fill
//! exactly as the crate paints it.
//!
//! The tessellator's triangles do not overlap and meet edge to edge, so
//! their areas add up to the area of the fill: abutting shapes leave no
//! seam, and a thin part or a slanted edge covers each pixel by what lies
//! in it.

type P = [f32; 2];

/// Raises `cov` (0..=1, `w`×`h`) to the fill's coverage where that is
/// higher. `map` takes icon units to pixels; an index past `vertices` maps
/// to NaN and adds nothing.
pub(super) fn fill_coverage(
    cov: &mut [f32],
    w: usize,
    h: usize,
    vertices: &[P],
    indices: &[u16],
    map: impl Fn(P) -> P,
) {
    let mut area = vec![0.0; w * h];
    let mut run = Vec::new();
    for &t in indices.as_chunks::<3>().0 {
        let tri = t.map(|i| {
            vertices
                .get(usize::from(i))
                .copied()
                .map_or([f32::NAN, f32::NAN], &map)
        });
        add_triangle(&mut area, &mut run, w, h, tri);
    }
    for (c, a) in cov.iter_mut().zip(&area) {
        *c = c.max(a.clamp(0.0, 1.0));
    }
}

/// A convex polygon of at most 8 points: a triangle cut by up to three
/// lines.
#[derive(Clone, Copy)]
struct Poly {
    p: [P; 8],
    n: usize,
}

impl Poly {
    fn points(&self) -> &[P] {
        &self.p[..self.n]
    }

    /// The part where `d(p) >= 0`, `d` affine. A point where an edge
    /// crosses `d = 0` goes through `on`, which pins the cut coordinate.
    fn cut(&self, d: impl Fn(P) -> f32, on: impl Fn(P) -> P) -> Poly {
        let mut out = Poly {
            p: [[0.0; 2]; 8],
            n: 0,
        };
        let pts = self.points();
        for (i, &q) in pts.iter().enumerate() {
            let p = pts[(i + pts.len() - 1) % pts.len()];
            let (dp, dq) = (d(p), d(q));
            if (dp >= 0.0) != (dq >= 0.0) && out.n < out.p.len() {
                let t = dp / (dp - dq);
                out.p[out.n] = on([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]);
                out.n += 1;
            }
            if dq >= 0.0 && out.n < out.p.len() {
                out.p[out.n] = q;
                out.n += 1;
            }
        }
        out
    }
}

/// Adds the area each pixel shares with the triangle to `area`.
///
/// The triangle is cut into one-pixel rows. In a row, the polygon's area
/// inside a pixel is the sum over its edges of `dy` times the edge's mean
/// overlap with the pixel's column span (`clamp(x - left, 0, 1)` averaged
/// over the edge): the right chain adds, the left chain, running the other
/// way, subtracts. An edge adds its whole `dy` to every pixel left of it,
/// so those go into a running sum; only the pixels an edge passes through
/// get a computed share. One walk of the row's edges per row, no clipping
/// per pixel.
fn add_triangle(area: &mut [f32], run: &mut Vec<f32>, w: usize, h: usize, tri: [P; 3]) {
    if !tri.iter().flatten().all(|v| v.is_finite()) {
        return;
    }
    let [a, b, c] = tri;
    let turn = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let tri = if turn >= 0.0 { [a, b, c] } else { [a, c, b] };
    let ys = tri.map(|p| p[1]);
    let rows = span(ys[0].min(ys[1]).min(ys[2]), ys[0].max(ys[1]).max(ys[2]), h);
    for y in rows {
        // Points local to the row (y in 0..=1, x from its first pixel)
        // keep the products small, so f32 rounds the areas far below a
        // level of 255.
        let y0 = y as f32;
        let mut row = Poly {
            p: [[0.0; 2]; 8],
            n: 3,
        };
        for (r, p) in row.p.iter_mut().zip(tri) {
            *r = [p[0], p[1] - y0];
        }
        let mut row = row
            .cut(|p| p[1], |p| [p[0], 0.0])
            .cut(|p| 1.0 - p[1], |p| [p[0], 1.0]);
        if row.n < 3 {
            continue;
        }
        let (lo, hi) = row.points().iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(l, h), p| {
            (l.min(p[0]), h.max(p[0]))
        });
        let cols = span(lo, hi, w);
        let x0 = cols.start as f32;
        for p in &mut row.p[..row.n] {
            p[0] -= x0;
        }
        let line = &mut area[y * w + cols.start..y * w + cols.end];
        let k = line.len();
        run.clear();
        run.resize(k + 1, 0.0);
        let pts = row.points();
        for (i, &q) in pts.iter().enumerate() {
            let p = pts[(i + pts.len() - 1) % pts.len()];
            let dy = q[1] - p[1];
            if dy == 0.0 {
                continue;
            }
            let (el, eh) = (p[0].min(q[0]), p[0].max(q[0]));
            // Pixels left of the edge's first pixel: all of dy.
            let first = (el.floor().max(0.0) as usize).min(k);
            run[0] += dy;
            run[first] -= dy;
            let end = (eh.ceil().max(0.0) as usize).min(k);
            for (x, cell) in line.iter_mut().enumerate().take(end).skip(first) {
                *cell += dy * mean_overlap(el, eh, x as f32);
            }
        }
        let mut sum = 0.0;
        for (cell, r) in line.iter_mut().zip(run.iter()) {
            sum += r;
            *cell += sum;
        }
    }
}

/// The mean over `lo..=hi` of `clamp(x - left, 0, 1)`: how far an edge
/// spanning `lo..=hi` reaches, on average, into the pixel column starting
/// at `left`. Each part is a length times a value, so a nearly vertical
/// edge loses no precision to cancellation.
fn mean_overlap(lo: f32, hi: f32, left: f32) -> f32 {
    let len = hi - lo;
    if len <= 0.0 {
        return (lo - left).clamp(0.0, 1.0);
    }
    let right = left + 1.0;
    let (w0, w1) = (lo.max(left), hi.min(right));
    let inside = if w1 > w0 {
        (w1 - w0) * ((w0 + w1) * 0.5 - left)
    } else {
        0.0
    };
    let beyond = (hi - lo.max(right)).max(0.0);
    (inside + beyond) / len
}

/// The pixel indices whose cells meet `lo..=hi`, clipped to `0..n`.
/// Non-finite bounds give an empty or clipped range.
fn span(lo: f32, hi: f32, n: usize) -> std::ops::Range<usize> {
    let first = lo.floor().max(0.0);
    let end = hi.ceil();
    // `as` saturates: NaN becomes 0, huge values become `usize::MAX`.
    (first as usize).min(n)..(end.max(0.0) as usize).min(n)
}
