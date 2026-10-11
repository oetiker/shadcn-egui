//! Filled paths: flatten to polylines, quantise, tessellate with lyon and
//! compute outward miters for the painter's one-pixel fringe.

use crate::blob::quantise;
use lyon_tessellation::math::point;
use lyon_tessellation::path::iterator::PathIterator;
use lyon_tessellation::path::{Path, PathEvent};
use lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};
use usvg::tiny_skia_path::{PathSegment, Point};

/// Curve flattening tolerance in the 24-unit icon box.
pub const FLATTEN_TOLERANCE: f32 = 0.02;

/// Distance of the inside/outside probe from an edge's midpoint, in icon units.
const PROBE: f32 = 1e-3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

/// One filled SVG path as closed polylines in the 24-unit box, quantised.
#[derive(Clone, Debug, PartialEq)]
pub struct FillPath {
    pub rule: FillRule,
    pub contours: Vec<Vec<[f32; 2]>>,
}

/// The fill of one icon, as the blob stores it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fill {
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
    /// Per contour: (point, outward miter). A zero miter marks a vertex
    /// whose edges have fill on both sides; the painter's fringe vanishes there.
    pub contours: Vec<Vec<([f32; 2], [f32; 2])>>,
}

/// Flatten one usvg path (transform applied, then scaled into the 24-unit
/// box) into closed, quantised polylines. Consecutive duplicate points and a
/// closing point equal to the first are dropped; contours with fewer than
/// three points are dropped.
pub fn flatten(
    path: &usvg::tiny_skia_path::Path,
    ts: usvg::Transform,
    scale: f32,
) -> Vec<Vec<[f32; 2]>> {
    let map = |mut p: Point| {
        ts.map_point(&mut p);
        point(p.x * scale, p.y * scale)
    };
    let mut b = Path::builder();
    let mut open = false;
    let mut start = point(0.0, 0.0);
    for seg in path.segments() {
        // SVG continues from the subpath's start after Z.
        if !open && !matches!(seg, PathSegment::MoveTo(_) | PathSegment::Close) {
            b.begin(start);
            open = true;
        }
        match seg {
            PathSegment::MoveTo(p) => {
                if open {
                    b.end(true);
                }
                start = map(p);
                b.begin(start);
                open = true;
            }
            PathSegment::LineTo(p) => {
                b.line_to(map(p));
            }
            PathSegment::QuadTo(c, p) => {
                b.quadratic_bezier_to(map(c), map(p));
            }
            PathSegment::CubicTo(c1, c2, p) => {
                b.cubic_bezier_to(map(c1), map(c2), map(p));
            }
            PathSegment::Close => {
                if open {
                    b.end(true);
                    open = false;
                }
            }
        }
    }
    if open {
        b.end(true);
    }

    let mut contours = Vec::new();
    let mut cur: Vec<[f32; 2]> = Vec::new();
    for evt in b.build().iter().flattened(FLATTEN_TOLERANCE) {
        match evt {
            PathEvent::Begin { at } => {
                cur.clear();
                push(&mut cur, at.x, at.y);
            }
            PathEvent::Line { to, .. } => push(&mut cur, to.x, to.y),
            PathEvent::End { .. } => {
                if cur.len() > 1 && cur.first() == cur.last() {
                    cur.pop();
                }
                if cur.len() >= 3 {
                    contours.push(std::mem::take(&mut cur));
                }
                cur.clear();
            }
            PathEvent::Quadratic { .. } | PathEvent::Cubic { .. } => {
                unreachable!("flattened paths hold only lines")
            }
        }
    }
    contours
}

fn push(c: &mut Vec<[f32; 2]>, x: f32, y: f32) {
    let q = [quantise(x), quantise(y)];
    if c.last() != Some(&q) {
        c.push(q);
    }
}

/// Tessellate every filled path of one icon and compute the contour miters.
/// `None` when the icon has no fill.
pub fn build(name: &str, paths: &[FillPath]) -> Result<Option<Fill>, String> {
    let mut fill = Fill::default();
    for fp in paths {
        let mut b = Path::builder();
        for c in &fp.contours {
            b.begin(point(c[0][0], c[0][1]));
            for p in &c[1..] {
                b.line_to(point(p[0], p[1]));
            }
            b.end(true);
        }
        let rule = match fp.rule {
            FillRule::NonZero => lyon_tessellation::FillRule::NonZero,
            FillRule::EvenOdd => lyon_tessellation::FillRule::EvenOdd,
        };
        let mut buf: VertexBuffers<[f32; 2], u16> = VertexBuffers::new();
        FillTessellator::new()
            .tessellate_path(
                &b.build(),
                &FillOptions::tolerance(FLATTEN_TOLERANCE).with_fill_rule(rule),
                &mut BuffersBuilder::new(&mut buf, |v: FillVertex| v.position().to_array()),
            )
            .map_err(|e| format!("{name}: tessellation failed: {e:?}"))?;
        let base = fill.vertices.len();
        if base + buf.vertices.len() > usize::from(u16::MAX) {
            return Err(format!("{name}: fill mesh has more than 65535 vertices"));
        }
        // Boundary vertices are already quantised; intersection vertices lyon
        // adds are quantised here so the blob stores exactly these values.
        fill.vertices.extend(
            buf.vertices
                .iter()
                .map(|v| [quantise(v[0]), quantise(v[1])]),
        );
        fill.indices
            .extend(buf.indices.iter().map(|&i| i + base as u16));
        for c in &fp.contours {
            fill.contours.push(miters(c, fp, paths));
        }
    }
    Ok((!fill.vertices.is_empty()).then_some(fill))
}

/// Outward miter per vertex of the closed contour `c` of `own`.
fn miters(c: &[[f32; 2]], own: &FillPath, all: &[FillPath]) -> Vec<([f32; 2], [f32; 2])> {
    let n = c.len();
    // Outward unit normal of edge i (c[i] -> c[i+1]); zero when both sides are filled.
    let normals: Vec<[f32; 2]> = (0..n)
        .map(|i| {
            let (a, b) = (c[i], c[(i + 1) % n]);
            let d = normalize([b[0] - a[0], b[1] - a[1]]);
            let left = [d[1], -d[0]];
            let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            let probe = |v: [f32; 2]| [mid[0] + v[0] * PROBE, mid[1] + v[1] * PROBE];
            let out = if inside(own, probe(left)) {
                [-left[0], -left[1]]
            } else {
                left
            };
            if all.iter().any(|fp| inside(fp, probe(out))) {
                [0.0, 0.0]
            } else {
                out
            }
        })
        .collect();
    (0..n)
        .map(|i| (c[i], miter(normals[(i + n - 1) % n], normals[i])))
        .collect()
}

/// The miter at a vertex between edges with outward normals `n1` (incoming)
/// and `n2` (outgoing): `normalize(n1 + n2) / max(dot(m, n1), 0.5)`, so its
/// length is at most 2. A zero normal (edge with fill on both sides) yields
/// the other edge's normal; two zero normals yield zero.
pub fn miter(n1: [f32; 2], n2: [f32; 2]) -> [f32; 2] {
    let zero = [0.0, 0.0];
    match (n1 == zero, n2 == zero) {
        (true, true) => zero,
        (true, false) => n2,
        (false, true) => n1,
        (false, false) => {
            let s = [n1[0] + n2[0], n1[1] + n2[1]];
            let len = (s[0] * s[0] + s[1] * s[1]).sqrt();
            if len < 1e-6 {
                return n1;
            }
            let m = [s[0] / len, s[1] / len];
            let k = 1.0 / (m[0] * n1[0] + m[1] * n1[1]).max(0.5);
            [m[0] * k, m[1] * k]
        }
    }
}

fn normalize(v: [f32; 2]) -> [f32; 2] {
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    [v[0] / len, v[1] / len]
}

/// Whether `p` lies inside the fill of `fp` under its fill rule.
pub fn inside(fp: &FillPath, p: [f32; 2]) -> bool {
    let w = winding(&fp.contours, p);
    match fp.rule {
        FillRule::NonZero => w != 0,
        FillRule::EvenOdd => w % 2 != 0,
    }
}

fn winding(contours: &[Vec<[f32; 2]>], p: [f32; 2]) -> i32 {
    let mut w = 0;
    for c in contours {
        for i in 0..c.len() {
            let (a, b) = (c[i], c[(i + 1) % c.len()]);
            let cross = (b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1]);
            if a[1] <= p[1] {
                if b[1] > p[1] && cross > 0.0 {
                    w += 1;
                }
            } else if b[1] <= p[1] && cross < 0.0 {
                w -= 1;
            }
        }
    }
    w
}
