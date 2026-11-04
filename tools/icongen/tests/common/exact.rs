//! Exact area coverage of filled paths, sharing no code with icongen's fill
//! module or with the crate's rasteriser: the reference the precision tests
//! measure the shipped raster against.
//!
//! A sweep cuts the plane into horizontal slabs at every vertex and every
//! crossing of two edges, so inside a slab no edges meet and their order
//! along x is fixed. Counting the winding number of each path from left to
//! right through that order (fill rule as the SVG says, union of the paths)
//! gives the filled runs, each a trapezoid bounded by two edges. The
//! trapezoids do not overlap, so adding their exact pixel areas (the signed
//! area accumulation of font rasterisers, exact for straight sides) yields
//! the exact coverage of every pixel.

use icongen::{FillPath, FillRule};

type P = [f64; 2];

/// A non-horizontal edge, `a` above `b` (smaller y), with the winding
/// direction it had in its contour and the path it belongs to.
struct Edge {
    a: P,
    b: P,
    dir: i32,
    path: usize,
}

impl Edge {
    fn x_at(&self, y: f64) -> f64 {
        self.a[0] + (y - self.a[1]) * (self.b[0] - self.a[0]) / (self.b[1] - self.a[1])
    }
}

/// The filled area of `paths` (in units) as directed trapezoid sides: down
/// on the left of each trapezoid, up on its right.
pub fn sides(paths: &[FillPath]) -> Vec<[P; 2]> {
    let mut edges = Vec::new();
    for (path, fp) in paths.iter().enumerate() {
        for c in &fp.contours {
            for i in 0..c.len() {
                let (p, q) = (c[i].map(f64::from), c[(i + 1) % c.len()].map(f64::from));
                if p[1] == q[1] {
                    continue;
                }
                let (a, b, dir) = if p[1] < q[1] { (p, q, 1) } else { (q, p, -1) };
                edges.push(Edge { a, b, dir, path });
            }
        }
    }
    let mut ys: Vec<f64> = edges.iter().flat_map(|e| [e.a[1], e.b[1]]).collect();
    crossings(&edges, &mut ys);
    ys.sort_by(f64::total_cmp);
    ys.dedup();

    edges.sort_by(|e, f| e.a[1].total_cmp(&f.a[1]));
    let mut next = 0;
    let mut active: Vec<usize> = Vec::new();
    let mut out = Vec::new();
    let mut winding = vec![0; paths.len()];
    for w in ys.windows(2) {
        let (y0, y1) = (w[0], w[1]);
        active.retain(|&i| edges[i].b[1] > y0);
        while next < edges.len() && edges[next].a[1] <= y0 {
            if edges[next].b[1] > y0 {
                active.push(next);
            }
            next += 1;
        }
        let ym = (y0 + y1) / 2.0;
        let mut order: Vec<(f64, usize)> = active.iter().map(|&i| (edges[i].x_at(ym), i)).collect();
        order.sort_by(|p, q| p.0.total_cmp(&q.0));
        winding.fill(0);
        let mut left: Option<usize> = None;
        for &(_, i) in &order {
            let e = &edges[i];
            winding[e.path] += e.dir;
            let filled = paths.iter().zip(&winding).any(|(fp, &w)| match fp.rule {
                FillRule::NonZero => w != 0,
                FillRule::EvenOdd => w % 2 != 0,
            });
            match (left, filled) {
                (None, true) => left = Some(i),
                (Some(l), false) => {
                    let l = &edges[l];
                    out.push([[l.x_at(y0), y0], [l.x_at(y1), y1]]);
                    out.push([[e.x_at(y1), y1], [e.x_at(y0), y0]]);
                    left = None;
                }
                _ => {}
            }
        }
    }
    out
}

/// Adds the y of every point where two edges cross to `ys`.
fn crossings(edges: &[Edge], ys: &mut Vec<f64>) {
    let lo = |e: &Edge| e.a[0].min(e.b[0]);
    let hi = |e: &Edge| e.a[0].max(e.b[0]);
    let mut order: Vec<usize> = (0..edges.len()).collect();
    order.sort_by(|&i, &j| lo(&edges[i]).total_cmp(&lo(&edges[j])));
    for (k, &i) in order.iter().enumerate() {
        let e = &edges[i];
        for &j in &order[k + 1..] {
            let f = &edges[j];
            if lo(f) > hi(e) {
                break;
            }
            if e.b[1] <= f.a[1] || f.b[1] <= e.a[1] {
                continue;
            }
            let r = [e.b[0] - e.a[0], e.b[1] - e.a[1]];
            let s = [f.b[0] - f.a[0], f.b[1] - f.a[1]];
            let denom = r[0] * s[1] - r[1] * s[0];
            if denom == 0.0 {
                continue;
            }
            let ac = [f.a[0] - e.a[0], f.a[1] - e.a[1]];
            let t = (ac[0] * s[1] - ac[1] * s[0]) / denom;
            let u = (ac[0] * r[1] - ac[1] * r[0]) / denom;
            if t > 0.0 && t < 1.0 && u > 0.0 && u < 1.0 {
                ys.push(e.a[1] + t * r[1]);
            }
        }
    }
}

/// The triangles of a tessellated mesh as directed sides, each triangle
/// turned the way [`sides`] turns trapezoids: the mesh's exact coverage.
pub fn mesh_sides(vertices: &[[f32; 2]], indices: &[u16]) -> Vec<[P; 2]> {
    let mut out = Vec::new();
    for t in indices.as_chunks::<3>().0 {
        let [a, b, c] = t.map(|i| vertices[usize::from(i)].map(f64::from));
        let turn = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        let tri = if turn < 0.0 { [a, b, c] } else { [a, c, b] };
        for k in 0..3 {
            out.push([tri[k], tri[(k + 1) % 3]]);
        }
    }
    out
}

/// Exact coverage (0..=255) of the trapezoid `sides` on a `side`² grid; the
/// 24-unit box fills it.
pub fn coverage(sides: &[[P; 2]], side: usize) -> Vec<u8> {
    let scale = side as f64 / 24.0;
    let stride = side + 2;
    let mut acc = vec![0.0; stride * side];
    for &[p, q] in sides {
        let (p, q) = (p.map(|v| v * scale), q.map(|v| v * scale));
        for [p, q] in clip_x(p, q, side as f64) {
            line(&mut acc, side, stride, p, q);
        }
    }
    let mut out = vec![0; side * side];
    for y in 0..side {
        let mut sum = 0.0;
        for x in 0..side {
            sum += acc[y * stride + x];
            out[y * side + x] = (sum.abs().min(1.0) * 255.0).round() as u8;
        }
    }
    out
}

/// `p`–`q` cut where it crosses x = 0 or x = `w`, each piece with its x
/// clamped into `0..=w`: area left of the grid lands on its edge, area right
/// of it beyond the last pixel.
fn clip_x(p: P, q: P, w: f64) -> Vec<[P; 2]> {
    let mut ts = vec![0.0, 1.0];
    for x in [0.0, w] {
        let t = (x - p[0]) / (q[0] - p[0]);
        if t > 0.0 && t < 1.0 {
            ts.push(t);
        }
    }
    ts.sort_by(f64::total_cmp);
    let at = |t: f64| {
        [
            (p[0] + t * (q[0] - p[0])).clamp(0.0, w),
            p[1] + t * (q[1] - p[1]),
        ]
    };
    ts.windows(2).map(|t| [at(t[0]), at(t[1])]).collect()
}

/// Accumulates the signed area of one straight side (font-rs's
/// `draw_line`): after a running sum along each row, a cell holds the area
/// of its pixel lying right of the down-going and left of the up-going
/// sides. `x` lies in `0..=w`.
fn line(acc: &mut [f64], h: usize, stride: usize, p: P, q: P) {
    if p[1] == q[1] {
        return;
    }
    let (dir, p, q) = if p[1] < q[1] {
        (1.0, p, q)
    } else {
        (-1.0, q, p)
    };
    let (top, bottom) = (p[1].max(0.0), q[1].min(h as f64));
    if top >= bottom {
        return;
    }
    let w = (stride - 2) as f64;
    let dxdy = (q[0] - p[0]) / (q[1] - p[1]);
    let mut x = p[0] + (top - p[1]) * dxdy;
    for y in (top.floor() as usize)..(bottom.ceil() as usize).min(h) {
        let dy = ((y + 1) as f64).min(bottom) - (y as f64).max(top);
        let xnext = (x + dxdy * dy).clamp(0.0, w);
        let d = dy * dir;
        let row = y * stride;
        let (x0, x1) = if x < xnext { (x, xnext) } else { (xnext, x) };
        let x0floor = x0.floor();
        let x0i = x0floor as usize;
        let x1ceil = x1.ceil();
        let x1i = x1ceil as usize;
        if x1i <= x0i + 1 {
            let xmf = 0.5 * (x + xnext) - x0floor;
            acc[row + x0i] += d - d * xmf;
            acc[row + x0i + 1] += d * xmf;
        } else {
            let s = (x1 - x0).recip();
            let x0f = x0 - x0floor;
            let a0 = 0.5 * s * (1.0 - x0f) * (1.0 - x0f);
            let x1f = x1 - x1ceil + 1.0;
            let am = 0.5 * s * x1f * x1f;
            acc[row + x0i] += d * a0;
            if x1i == x0i + 2 {
                acc[row + x0i + 1] += d * (1.0 - a0 - am);
            } else {
                let a1 = s * (1.5 - x0f);
                acc[row + x0i + 1] += d * (a1 - a0);
                for xi in x0i + 2..x1i - 1 {
                    acc[row + xi] += d * s;
                }
                let a2 = a1 + (x1i - x0i - 3) as f64 * s;
                acc[row + x1i - 1] += d * (1.0 - a2 - am);
            }
            acc[row + x1i] += d * am;
        }
        x = xnext;
    }
}
