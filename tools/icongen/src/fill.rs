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
    /// The closed loops of the filled area's boundary: (point, outward
    /// miter). Edges with fill on both sides or on neither side are not part
    /// of any loop, so the painter's fringe lies only outside the fill.
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

/// Tessellate every filled path of one icon and trace the fringe contours.
/// `None` when the icon has no fill (or only zero-area fill).
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
    }
    if fill.indices.is_empty() {
        return Ok(None);
    }
    fill.contours = boundary(name, paths)?;
    Ok(Some(fill))
}

/// The fringe contours of an icon: the boundary of the union of all its
/// filled paths, as closed loops with an outward miter per vertex.
///
/// Every input edge is split where any other edge crosses or touches it, so
/// the side that holds fill cannot change along a piece. A piece is boundary
/// only when exactly one side is filled; pieces with fill on both sides or on
/// neither side (zero-area subpaths) get no fringe. The boundary pieces are
/// oriented with the outside on their left and chained head to tail.
///
/// Which side holds fill is decided exactly, by counting the winding number
/// along a ray from the piece's midpoint to either side, so no probe distance
/// can reach past a nearby edge. Splitting, classifying and chaining run in
/// f64 on the exact crossing points; only the finished loops are quantised,
/// so the arrangement stays consistent even where crossings lie closer
/// together than one quantum.
fn boundary(name: &str, paths: &[FillPath]) -> Result<Vec<Contour>, String> {
    let edges: Vec<[P; 2]> = paths
        .iter()
        .flat_map(|fp| &fp.contours)
        .flat_map(|c| {
            (0..c.len()).map(move |i| [c[i], c[(i + 1) % c.len()]].map(|p| p.map(f64::from)))
        })
        .collect();
    let splits = split_points(&edges);
    // Whether fill lies right beside `p` (on a piece along `u`) in direction `dir`.
    let vertices: Vec<P> = paths
        .iter()
        .flat_map(|fp| fp.contours.iter().flatten())
        .map(|v| v.map(f64::from))
        .collect();
    let filled = |p: P, u: P, side: P| {
        let dir = ray_dir(&vertices, p, side);
        paths
            .iter()
            .any(|fp| covered(fp, winding(&fp.contours, p, dir, Some(u))))
    };

    let mut pieces: Vec<[P; 2]> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (&[a, b], pts) in edges.iter().zip(&splits) {
        let d = sub(b, a);
        let mut pts = pts.clone();
        pts.push(a);
        pts.push(b);
        let t = |p: &P| dot(sub(*p, a), d);
        pts.sort_by(|p, q| t(p).total_cmp(&t(q)));
        pts.dedup();
        for w in pts.windows(2) {
            let (p, q) = (w[0], w[1]);
            let d = sub(q, p);
            let mid = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0];
            let len = d[0].hypot(d[1]);
            let u = [d[0] / len, d[1] / len];
            let (left, right) = (filled(mid, u, [u[1], -u[0]]), filled(mid, u, [-u[1], u[0]]));
            let piece = match (left, right) {
                (false, true) => [p, q],
                (true, false) => [q, p],
                _ => continue,
            };
            // Coincident edges of stacked paths would draw the fringe twice.
            if seen.insert(piece.map(key)) {
                pieces.push(piece);
            }
        }
    }

    let mut from: std::collections::HashMap<_, Vec<usize>> = std::collections::HashMap::new();
    for (i, piece) in pieces.iter().enumerate() {
        from.entry(key(piece[0])).or_default().push(i);
    }
    let mut used = vec![false; pieces.len()];
    let mut loops = Vec::new();
    for first in 0..pieces.len() {
        if used[first] {
            continue;
        }
        let mut points = Vec::new();
        let mut cur = first;
        loop {
            used[cur] = true;
            points.push(pieces[cur][0]);
            let [p, end] = pieces[cur];
            // Where loops touch (a pinch vertex), stay with the fill wedge on
            // the right of the incoming piece: take the outgoing piece that
            // comes first sweeping clockwise from the way back.
            let back = sub(p, end);
            let next = from
                .get(&key(end))
                .into_iter()
                .flatten()
                .copied()
                .filter(|&i| i == first || !used[i])
                .min_by(|&i, &j| {
                    let turn = |k: usize| clockwise(back, sub(pieces[k][1], end));
                    turn(i).total_cmp(&turn(j))
                })
                .ok_or_else(|| format!("{name}: fill boundary does not close at {end:?}"))?;
            if next == first {
                break;
            }
            cur = next;
        }
        if let Some(l) = quantise_loop(&points) {
            loops.push(l);
        }
    }
    Ok(loops)
}

/// A ray direction from `p` into the half-plane of the unit normal `side`
/// whose line passes no vertex. The perpendicular is tried first, then directions
/// turned away from it: a ray through a vertex, or along an edge, is
/// decided by the sign of a rounding error.
fn ray_dir(vertices: &[P], p: P, side: P) -> P {
    const TURNS: [f64; 7] = [0.0, 0.37, -0.37, 0.71, -0.71, 1.07, -1.07];
    for a in TURNS {
        let (s, c) = a.sin_cos();
        let d = [side[0] * c - side[1] * s, side[0] * s + side[1] * c];
        // The whole line counts, not only the ray ahead: the crossing rule
        // tests an edge against the line first.
        let clear = vertices.iter().all(|v| cross(d, sub(*v, p)).abs() > WELD);
        if clear {
            return d;
        }
    }
    side
}

/// A point in f64, for the boundary arithmetic.
type P = [f64; 2];

/// One fringe loop: (point, outward miter) per vertex.
type Contour = Vec<([f32; 2], [f32; 2])>;

/// Quantise a closed loop. Runs of points that land on the same grid point
/// become one vertex whose miter joins the run's incoming and outgoing edges,
/// so pieces shorter than a quantum do not turn the miter. `None` when fewer
/// than three vertices remain.
fn quantise_loop(points: &[P]) -> Option<Contour> {
    let n = points.len();
    let q = |p: P| [quantise(p[0] as f32), quantise(p[1] as f32)];
    let normal = |i: usize| {
        let (p, r) = (points[i], points[(i + 1) % n]);
        let d = sub(r, p);
        let len = d[0].hypot(d[1]);
        [(d[1] / len) as f32, (-d[0] / len) as f32]
    };
    // Start at a point that begins a run, so no run wraps around the end.
    let start = (0..n).find(|&i| q(points[i]) != q(points[(i + n - 1) % n]))?;
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        let first = (start + i) % n;
        let mut last = first;
        while i + 1 < n && q(points[(start + i + 1) % n]) == q(points[first]) {
            i += 1;
            last = (start + i) % n;
        }
        out.push((
            q(points[first]),
            miter(normal((first + n - 1) % n), normal(last)),
        ));
        i += 1;
    }
    (out.len() >= 3).then_some(out)
}

/// Per edge, the points where other edges cross it or end on it.
///
/// Every point goes through one [`Weld`] table, so a crossing found on two
/// collinear edges (stacked paths sharing an edge line, a spike running back
/// along itself) is the same value on both, and so is a crossing that lands
/// on a vertex. Without that the two copies differ in the last bit and the
/// boundary pieces do not meet.
fn split_points(edges: &[[P; 2]]) -> Vec<Vec<P>> {
    let mut weld = Weld::default();
    // Input vertices first: they are the representatives crossings snap to.
    for e in edges {
        weld.snap(e[0]);
        weld.snap(e[1]);
    }
    let mut splits = vec![Vec::new(); edges.len()];
    let lo = |e: &[P; 2]| e[0][0].min(e[1][0]);
    let hi = |e: &[P; 2]| e[0][0].max(e[1][0]);
    // Sweep along x so only edges whose x ranges overlap are compared.
    let mut order: Vec<usize> = (0..edges.len()).collect();
    order.sort_by(|&i, &j| lo(&edges[i]).total_cmp(&lo(&edges[j])));
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            if lo(&edges[j]) > hi(&edges[i]) {
                break;
            }
            let ([a, b], [c, d]) = (edges[i], edges[j]);
            if a[1].max(b[1]) < c[1].min(d[1]) || c[1].max(d[1]) < a[1].min(b[1]) {
                continue;
            }
            if let Some(p) = crossing(a, b, c, d) {
                let p = weld.snap(p);
                splits[i].push(p);
                splits[j].push(p);
            }
            for p in [c, d] {
                if on_segment(p, a, b) {
                    splits[i].push(p);
                }
            }
            for p in [a, b] {
                if on_segment(p, c, d) {
                    splits[j].push(p);
                }
            }
        }
    }
    splits
}

/// Points closer than this are one point. Far above f64 rounding of a
/// crossing (about 1e-14 at icon scale), far below a quantum (1/1024).
const WELD: f64 = 1e-9;

/// A table of distinct points; `snap` returns the first point seen within
/// [`WELD`] of its argument.
#[derive(Default)]
struct Weld {
    cells: std::collections::HashMap<[i64; 2], Vec<P>>,
}

impl Weld {
    /// Cells are 1000 × WELD wide, so a match lies in the 3 × 3 block around a point's cell.
    const CELL: f64 = WELD * 1000.0;

    fn snap(&mut self, p: P) -> P {
        let c = p.map(|v| (v / Self::CELL).floor() as i64);
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(found) = self.cells.get(&[c[0] + dx, c[1] + dy]).and_then(|ps| {
                    ps.iter()
                        .find(|q| (q[0] - p[0]).hypot(q[1] - p[1]) < WELD)
                        .copied()
                }) {
                    return found;
                }
            }
        }
        self.cells.entry(c).or_default().push(p);
        p
    }
}

/// The crossing point of segments `ab` and `cd`, if they are not parallel and meet.
fn crossing(a: P, b: P, c: P, d: P) -> Option<P> {
    let (r, s) = (sub(b, a), sub(d, c));
    let denom = cross(r, s);
    if denom.abs() <= 1e-12 * (r[0].hypot(r[1]) * s[0].hypot(s[1])) {
        return None;
    }
    let ac = sub(c, a);
    let (t, u) = (cross(ac, s) / denom, cross(ac, r) / denom);
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u))
        .then(|| [a[0] + t * r[0], a[1] + t * r[1]])
}

/// Whether `p` lies on segment `ab`, strictly between its ends.
fn on_segment(p: P, a: P, b: P) -> bool {
    let (r, ap) = (sub(b, a), sub(p, a));
    let len2 = dot(r, r);
    let t = dot(ap, r) / len2;
    t > 0.0 && t < 1.0 && cross(r, ap).abs() / len2.sqrt() < WELD
}

/// The angle from `u` to `w` sweeping clockwise in math orientation
/// (`cross < 0`), in `(0, 2π]`.
fn clockwise(u: P, w: P) -> f64 {
    let a = (-cross(u, w)).atan2(dot(u, w));
    if a <= 0.0 {
        a + std::f64::consts::TAU
    } else {
        a
    }
}

fn sub(u: P, v: P) -> P {
    [u[0] - v[0], u[1] - v[1]]
}

fn dot(u: P, v: P) -> f64 {
    u[0] * v[0] + u[1] * v[1]
}

fn cross(u: P, v: P) -> f64 {
    u[0] * v[1] - u[1] * v[0]
}

/// Hash key of a point; `+ 0.0` folds -0.0 into 0.0.
fn key(p: P) -> [u64; 2] {
    p.map(|v| (v + 0.0).to_bits())
}

/// The miter at a vertex between edges with outward normals `n1` (incoming)
/// and `n2` (outgoing): `normalize(n1 + n2) / max(dot(m, n1), 0.5)`, so its
/// length is at most 2. A zero normal yields the other edge's normal; two
/// zero normals yield zero.
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

/// Whether `p` lies inside the fill of `fp` under its fill rule.
pub fn inside(fp: &FillPath, p: [f32; 2]) -> bool {
    covered(
        fp,
        winding(&fp.contours, p.map(f64::from), [1.0, 0.0], None),
    )
}

fn covered(fp: &FillPath, w: i32) -> bool {
    match fp.rule {
        FillRule::NonZero => w != 0,
        FillRule::EvenOdd => w % 2 != 0,
    }
}

/// Winding number of the contours around `p`, counted along the ray from
/// `p` in direction `dir`. With `along = Some(u)` (a unit vector), edges on
/// the line through `p` along `u` are left out: for `p` on such an edge and
/// `dir` across it, the result is the winding number infinitesimally beside
/// the edge on the `dir` side, decided without any probe distance.
fn winding(contours: &[Vec<[f32; 2]>], p: P, dir: P, along: Option<P>) -> i32 {
    // Ten times WELD: a piece whose ends were welded lies at most WELD off its edge's line.
    const ALONG: f64 = 10.0 * WELD;
    let mut w = 0;
    for c in contours {
        for i in 0..c.len() {
            let (a, b) = (c[i].map(f64::from), c[(i + 1) % c.len()].map(f64::from));
            if let Some(u) = along {
                if cross(u, sub(a, p)).abs() < ALONG && cross(u, sub(b, p)).abs() < ALONG {
                    continue;
                }
            }
            let (ha, hb) = (cross(dir, sub(a, p)), cross(dir, sub(b, p)));
            let side = cross(sub(b, a), sub(p, a));
            if ha <= 0.0 {
                if hb > 0.0 && side > 0.0 {
                    w += 1;
                }
            } else if hb <= 0.0 && side < 0.0 {
                w -= 1;
            }
        }
    }
    w
}
