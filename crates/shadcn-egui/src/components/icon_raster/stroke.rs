//! The stroke part of the icon rasteriser: each pixel gets the exact area
//! of it that the union of a layer's strokes (and its fill, when it has
//! both) covers. Plain `[f32; 2]` points and no dependencies besides the
//! sibling `fill` module's row walk, so icongen's tests can include both
//! files to measure a stroke exactly as the crate paints it.
//!
//! Each sub-path becomes one closed outline: the offsets on one side
//! forward, the end cap, the offsets on the other side backward, the start
//! cap; at each joint the outer side gets the join (round arc, miter point
//! or bevel chord) and the inner side either the point where the two inner
//! offsets meet or, where the pieces are too short for that, a pivot
//! through the joint point. As a winding number that outline is the sum of
//! one quad per piece, one join shape per joint and the caps, all turned
//! the same way, so it is at least 1 exactly on the stroke's area and 0 off
//! it, for any turn, short pieces and tight curves included. The pixel
//! area of the non-zero region is then found row by row: in each row the
//! edges are cut where they end or cross, and in each band between those
//! cuts the edges keep their order, so the boundary pieces (where the
//! winding goes from 0 to non-zero and back) are found by one walk.

use super::fill::add_edge;

type P = [f32; 2];

/// How the open ends of a sub-path are drawn (SVG `stroke-linecap`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "icons carry no styles yet: every stroke is round")
)]
pub(super) enum Cap {
    Round,
    Butt,
    Square,
}

/// How two pieces of a sub-path meet (SVG `stroke-linejoin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "icons carry no styles yet: every stroke is round")
)]
pub(super) enum Join {
    Round,
    Miter,
    Bevel,
}

/// The style of one stroke. `miter_limit` is SVG's `stroke-miterlimit`:
/// a miter longer than this many stroke widths falls back to a bevel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Style {
    pub cap: Cap,
    pub join: Join,
    pub miter_limit: f32,
}

impl Style {
    /// Round caps and round joins.
    pub const ROUND: Style = Style {
        cap: Cap::Round,
        join: Join::Round,
        miter_limit: 4.0,
    };
}

/// An edge of the outline, top end first; `dir` is +1 for an edge that
/// runs down, -1 for one that runs up.
#[derive(Clone, Copy)]
struct Edge {
    top: P,
    bottom: P,
    dir: f32,
}

impl Edge {
    fn x_at(&self, y: f32) -> f32 {
        let [x0, y0] = self.top;
        let [x1, y1] = self.bottom;
        x0 + (x1 - x0) * ((y - y0) / (y1 - y0))
    }
}

/// The closed outlines of a layer's strokes and fill, as edges.
#[derive(Default)]
pub(super) struct Outline {
    edges: Vec<Edge>,
}

/// A sub-path piece: its start, its end, its direction, its length.
#[derive(Clone, Copy)]
struct Piece {
    a: P,
    b: P,
    d: P,
    len: f32,
}

fn add(a: P, b: P) -> P {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}

fn mul(a: P, s: f32) -> P {
    [a[0] * s, a[1] * s]
}

fn cross(a: P, b: P) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}

fn dot(a: P, b: P) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}

/// The outline side: the direction `d` turned so that a piece's forward
/// side, run forward, and its other side, run backward, enclose it with
/// the same turn as the fill's triangles.
fn side(d: P) -> P {
    [d[1], -d[0]]
}

impl Outline {
    /// Adds the edge `a`–`b`; a level or non-finite edge adds nothing.
    fn edge(&mut self, a: P, b: P) {
        if a[1] == b[1] || !a.iter().chain(&b).all(|v| v.is_finite()) {
            return;
        }
        self.edges.push(if a[1] < b[1] {
            Edge {
                top: a,
                bottom: b,
                dir: 1.0,
            }
        } else {
            Edge {
                top: b,
                bottom: a,
                dir: -1.0,
            }
        });
    }

    /// Adds the edges from `from`, round `centre` at radius `r`, by `sweep`
    /// radians (positive: from x towards y), ending exactly at `to`. The
    /// chords run no more than `tol` inside the arc and, with the vertices
    /// moved out by half that, cross it: the area they lose and gain is
    /// balanced. At most `max` chords.
    #[allow(clippy::too_many_arguments)]
    fn arc(&mut self, centre: P, from: P, to: P, r: f32, sweep: f32, (tol, max): (f32, f32)) {
        let start = (from[1] - centre[1]).atan2(from[0] - centre[0]);
        // The chord of angle `s` lies r (1 - cos(s/2)) inside the arc.
        let step = 2.0 * (1.0 - (tol / r).min(1.0)).acos();
        let n = (sweep.abs() / step).ceil().clamp(1.0, max);
        let n = if n.is_nan() { 1 } else { n as usize };
        let s = sweep / n as f32;
        let rv = r * 2.0 / (1.0 + (s / 2.0).cos());
        let mut prev = from;
        for i in 1..n {
            let t = start + s * i as f32;
            let q = [centre[0] + rv * t.cos(), centre[1] + rv * t.sin()];
            self.edge(prev, q);
            prev = q;
        }
        self.edge(prev, to);
    }

    /// Adds the outline of a stroke of half width `half` along `pts` (in
    /// pixels), closed back to its first point when `closed`. A sub-path
    /// whose points all coincide is a dot: a disc for round caps, a square
    /// for square caps, nothing for butt caps. `flatten` is the arcs'
    /// tolerance in pixels and their most chords.
    pub(super) fn stroke(
        &mut self,
        pts: &[P],
        closed: bool,
        half: f32,
        style: Style,
        flatten: (f32, f32),
    ) {
        if !(half > 0.0 && half.is_finite()) {
            return;
        }
        let mut pieces: Vec<Piece> = Vec::with_capacity(pts.len());
        let mut push = |a: P, b: P| {
            let v = sub(b, a);
            let len = v[0].hypot(v[1]);
            if len > 0.0 && len.is_finite() {
                pieces.push(Piece {
                    a,
                    b,
                    d: mul(v, 1.0 / len),
                    len,
                });
            }
        };
        for w in pts.windows(2) {
            push(w[0], w[1]);
        }
        if closed {
            if let (Some(&first), Some(&last)) = (pts.first(), pts.last()) {
                push(last, first);
            }
        }
        let Some(&p0) = pts.first() else { return };
        if pieces.is_empty() {
            self.dot(p0, half, style.cap, flatten);
            return;
        }
        // A closed sub-path of pieces that end where the next starts.
        let closed = closed && pieces.len() > 1;
        let n = pieces.len();
        // The inner offsets of each piece, moved to where they meet the
        // neighbour's: [forward side start, end, other side start, end].
        let mut fwd: Vec<[P; 2]> = pieces
            .iter()
            .map(|p| {
                let s = mul(side(p.d), half);
                [add(p.a, s), add(p.b, s)]
            })
            .collect();
        let mut back: Vec<[P; 2]> = pieces
            .iter()
            .map(|p| {
                let s = mul(side(p.d), half);
                [sub(p.a, s), sub(p.b, s)]
            })
            .collect();
        let mut trim = vec![0.0f32; n];
        // Joints: (piece before, piece after). Each is either trimmed
        // (inner offsets meet at a point) or pivots through the joint.
        let joints = if closed { n } else { n - 1 };
        let mut pivot = vec![false; joints];
        for (k, piv) in pivot.iter_mut().enumerate() {
            let (i, j) = (k, (k + 1) % n);
            let (pi, pj) = (pieces[i], pieces[j]);
            let sin = cross(pi.d, pj.d);
            let cos = dot(pi.d, pj.d);
            // How far along each piece the inner offsets meet.
            let along = half * sin.abs() / (1.0 + cos);
            let fits = cos > -0.99
                && along.is_finite()
                && trim[i] + along <= pi.len
                && trim[j] + along <= pj.len
                && half * sin.abs() <= pi.len.min(pj.len);
            if !fits || sin == 0.0 {
                *piv = sin != 0.0 || cos < 0.0;
                continue;
            }
            trim[i] += along;
            trim[j] += along;
            let meet = sub(
                pj.a,
                mul(
                    add(side(pi.d), side(pj.d)),
                    half / (1.0 + cos) * sin.signum(),
                ),
            );
            if sin > 0.0 {
                // Turning towards the other side: it is the inner side.
                back[i][1] = meet;
                back[j][0] = meet;
            } else {
                fwd[i][1] = meet;
                fwd[j][0] = meet;
            }
        }
        for k in 0..n {
            self.edge(fwd[k][0], fwd[k][1]);
            self.edge(back[k][1], back[k][0]);
        }
        for (k, &piv) in pivot.iter().enumerate() {
            let (i, j) = (k, (k + 1) % n);
            let (pi, pj) = (pieces[i], pieces[j]);
            let jp = pj.a;
            let sin = cross(pi.d, pj.d);
            let (si, sj) = (side(pi.d), side(pj.d));
            // Outer side: the forward side when turning towards the other.
            if sin >= 0.0 {
                self.join(jp, fwd[i][1], fwd[j][0], si, sj, half, style, flatten);
                if piv {
                    self.edge(back[j][0], jp);
                    self.edge(jp, back[i][1]);
                }
            } else {
                self.join(
                    jp,
                    back[j][0],
                    back[i][1],
                    mul(sj, -1.0),
                    mul(si, -1.0),
                    half,
                    style,
                    flatten,
                );
                if piv {
                    self.edge(fwd[i][1], jp);
                    self.edge(jp, fwd[j][0]);
                }
            }
        }
        if !closed {
            let (first, last) = (pieces[0], pieces[n - 1]);
            self.cap(
                last.b,
                last.d,
                fwd[n - 1][1],
                back[n - 1][1],
                half,
                style.cap,
                flatten,
            );
            let rev = mul(first.d, -1.0);
            self.cap(
                first.a, rev, back[0][0], fwd[0][0], half, style.cap, flatten,
            );
        }
    }

    /// The outer side of a joint at `jp`, from `from` (offset `u` x half)
    /// to `to` (offset `v` x half), turning from `u` towards `v` the
    /// positive way (from x towards y), by up to half a turn.
    #[allow(clippy::too_many_arguments)]
    fn join(
        &mut self,
        jp: P,
        from: P,
        to: P,
        u: P,
        v: P,
        half: f32,
        style: Style,
        flatten: (f32, f32),
    ) {
        let cos = dot(u, v);
        match style.join {
            Join::Round => {
                // The turn is never negative here. At a reversal (u = -v)
                // the cross product is +0 or -0 by rounding alone, and
                // -0 would send the arc back over the stroke instead of
                // round its far side.
                let sweep = cross(u, v).abs().atan2(cos);
                self.arc(jp, from, to, half, sweep, flatten);
            }
            // The miter is 1 / cos(turn / 2) half widths long.
            Join::Miter if (1.0 + cos) * style.miter_limit * style.miter_limit >= 2.0 => {
                let tip = add(jp, mul(add(u, v), half / (1.0 + cos)));
                self.edge(from, tip);
                self.edge(tip, to);
            }
            Join::Miter | Join::Bevel => self.edge(from, to),
        }
    }

    /// The cap at the end `e` of a piece running in direction `d`, from its
    /// offset `from` round to `to`.
    #[allow(clippy::too_many_arguments)]
    fn cap(&mut self, e: P, d: P, from: P, to: P, half: f32, cap: Cap, flatten: (f32, f32)) {
        match cap {
            Cap::Round => self.arc(e, from, to, half, std::f32::consts::PI, flatten),
            Cap::Butt => self.edge(from, to),
            Cap::Square => {
                let out = mul(d, half);
                let (a, b) = (add(from, out), add(to, out));
                self.edge(from, a);
                self.edge(a, b);
                self.edge(b, to);
            }
        }
    }

    /// A zero-length sub-path at `p`: SVG draws its caps, facing along x.
    fn dot(&mut self, p: P, half: f32, cap: Cap, flatten: (f32, f32)) {
        let pi = std::f32::consts::PI;
        let (top, bottom) = ([p[0], p[1] - half], [p[0], p[1] + half]);
        match cap {
            Cap::Butt => {}
            Cap::Round => {
                self.arc(p, top, bottom, half, pi, flatten);
                self.arc(p, bottom, top, half, pi, flatten);
            }
            Cap::Square => {
                let c = [
                    [p[0] - half, p[1] - half],
                    [p[0] + half, p[1] - half],
                    [p[0] + half, p[1] + half],
                    [p[0] - half, p[1] + half],
                ];
                for k in 0..4 {
                    self.edge(c[k], c[(k + 1) % 4]);
                }
            }
        }
    }

    /// Adds the fill's triangles, each turned the same way as the strokes.
    /// Edges two triangles share cancel and are left out. An index past
    /// `vertices` or a non-finite point drops its triangle.
    pub(super) fn triangles(&mut self, vertices: &[P], indices: &[u16], map: impl Fn(P) -> P) {
        let pts: Vec<P> = vertices.iter().map(|&p| map(p)).collect();
        let mut keys: Vec<(u16, u16)> = Vec::with_capacity(indices.len());
        for t in indices.as_chunks::<3>().0 {
            let Some(tri) = t
                .iter()
                .map(|&i| pts.get(usize::from(i)).copied())
                .collect::<Option<Vec<P>>>()
            else {
                continue;
            };
            if !tri.iter().flatten().all(|v| v.is_finite()) {
                continue;
            }
            let turn = cross(sub(tri[1], tri[0]), sub(tri[2], tri[0]));
            let [a, b, c] = *t;
            let t = if turn >= 0.0 { [a, b, c] } else { [a, c, b] };
            for k in 0..3 {
                keys.push((t[k], t[(k + 1) % 3]));
            }
        }
        // Edges both ways cancel: what is left is the fill's boundary.
        let mut net: Vec<(u16, u16, i32)> = keys
            .iter()
            .map(|&(a, b)| (a.min(b), a.max(b), if a < b { 1 } else { -1 }))
            .collect();
        net.sort_unstable();
        for group in net.chunk_by(|x, y| (x.0, x.1) == (y.0, y.1)) {
            let (lo, hi) = (group[0].0, group[0].1);
            let sum: i32 = group.iter().map(|g| g.2).sum();
            let (a, b) = if sum > 0 { (lo, hi) } else { (hi, lo) };
            for _ in 0..sum.unsigned_abs() {
                self.edge(pts[usize::from(a)], pts[usize::from(b)]);
            }
        }
    }

    /// Raises `cov` (0..=1, `w`×`h`) to the exact pixel area of the
    /// outline's non-zero region.
    pub(super) fn coverage(&self, cov: &mut [f32], w: usize, h: usize) {
        let mut edges = self.edges.clone();
        edges.sort_unstable_by(|a, b| a.top[1].total_cmp(&b.top[1]));
        let mut next = 0;
        let mut active: Vec<Edge> = Vec::new();
        let mut row = Row::default();
        let mut line = vec![0.0f32; w];
        let mut run = vec![0.0f32; w + 1];
        let first_row = edges.first().map_or(0.0, |e| e.top[1].floor().max(0.0));
        let first_row = (first_row as usize).min(h);
        for y in first_row..h {
            let (r0, r1) = (y as f32, y as f32 + 1.0);
            while next < edges.len() && edges[next].top[1] < r1 {
                active.push(edges[next]);
                next += 1;
            }
            active.retain(|e| e.bottom[1] > r0);
            if active.is_empty() {
                if next == edges.len() {
                    break;
                }
                continue;
            }
            // The edges cut to the row, in row-local y.
            row.segs.clear();
            let mut lo = f32::INFINITY;
            for e in &active {
                let ya = e.top[1].max(r0);
                let yb = e.bottom[1].min(r1);
                if yb <= ya {
                    continue;
                }
                // An end inside the row keeps its point exactly, so the
                // piece that runs on from it finds it.
                let x_at = |y: f32, end: P| if y == end[1] { end[0] } else { e.x_at(y) };
                let s = Seg {
                    ya: ya - r0,
                    yb: yb - r0,
                    xa: x_at(ya, e.top),
                    xb: x_at(yb, e.bottom),
                    dir: e.dir,
                    slope: 0.0,
                };
                let s = Seg {
                    slope: (s.xb - s.xa) / (s.yb - s.ya),
                    ..s
                };
                lo = lo.min(s.lo());
                row.segs.push(s);
            }
            // Only the pixels from the leftmost edge on can change.
            let first = (lo.floor().max(0.0) as usize).min(w);
            let span = &mut line[first..];
            let run = &mut run[first..];
            span.fill(0.0);
            run.fill(0.0);
            row.area(first as f32, span, run);
            let mut sum = 0.0;
            for (x, (cell, r)) in span.iter().zip(run.iter()).enumerate() {
                sum += r;
                let c = &mut cov[y * w + first + x];
                *c = c.max((cell + sum).clamp(0.0, 1.0));
            }
        }
    }
}

/// No link: the end of a chain.
const NONE: usize = usize::MAX;

/// Two pieces closer than this (relative to x) at a band's end touch
/// there: well above f32 rounding of the outline's points, well below
/// any gap that shows. The case that needs it is a square cap whose
/// corner lies on another piece's side (`a_reversal_keeps_its_join`);
/// no bundled icon renders differently with an exact-zero test.
const TOUCH: f32 = 2e-5;

/// One pixel row's edges and the scratch to walk them, kept between rows
/// so a row allocates nothing.
#[derive(Default)]
struct Row {
    segs: Vec<Seg>,
    /// The piece that carries each piece on below its bottom end, or NONE.
    next: Vec<usize>,
    /// Whether a piece carries another on: it is no chain's top.
    linked: Vec<bool>,
    /// Each chain's bottom end, at its top piece.
    end: Vec<f32>,
    by_lo: Vec<usize>,
    cuts: Vec<f32>,
    /// The chains spanning the band: x at its middle, top piece, the
    /// first piece that reaches below the band's top.
    order: Vec<(f32, usize, usize)>,
}

impl Row {
    /// Adds to `line` and `run` (local to the pixel at `x0`) the area of
    /// the non-zero region over the row.
    ///
    /// Pieces that run on from one another (the bottom end of one is the
    /// top end of the next, same direction) form a chain; between chains
    /// the order changes only where a chain ends or two cross. The row is
    /// cut into bands there, and in each band the chains, in their order,
    /// are walked from the left.
    fn area(&mut self, x0: f32, line: &mut [f32], run: &mut [f32]) {
        let segs = &mut self.segs;
        segs.sort_unstable_by(|a, b| a.ya.total_cmp(&b.ya).then(a.xa.total_cmp(&b.xa)));
        let n = segs.len();
        let (next, linked, end) = (&mut self.next, &mut self.linked, &mut self.end);
        next.clear();
        next.resize(n, NONE);
        linked.clear();
        linked.resize(n, false);
        for i in 0..n {
            let s = &segs[i];
            if s.yb >= 1.0 {
                continue;
            }
            let key = (s.yb, s.xb);
            let mut j = segs.partition_point(|t| (t.ya, t.xa) < key);
            while j < n && (segs[j].ya, segs[j].xa) == key {
                if !linked[j] && segs[j].dir == s.dir {
                    next[i] = j;
                    linked[j] = true;
                    break;
                }
                j += 1;
            }
        }
        end.clear();
        end.resize(n, 0.0);
        let cuts = &mut self.cuts;
        cuts.clear();
        cuts.extend([0.0, 1.0]);
        for i in 0..n {
            if linked[i] {
                continue;
            }
            let mut k = i;
            while next[k] != NONE {
                k = next[k];
            }
            end[i] = segs[k].yb;
            cuts.extend([segs[i].ya, segs[k].yb]);
        }
        // Where two pieces cross or touch, their chains may swap.
        let by_lo = &mut self.by_lo;
        by_lo.clear();
        by_lo.extend(0..n);
        by_lo.sort_unstable_by(|&a, &b| segs[a].lo().total_cmp(&segs[b].lo()));
        for (k, &i) in by_lo.iter().enumerate() {
            let s = &segs[i];
            let (sl, sh) = (s.lo(), s.hi());
            for &j in &by_lo[k + 1..] {
                let t = &segs[j];
                // Sorted by their left end: none after this one reaches.
                if t.lo() > sh {
                    break;
                }
                if t.hi() < sl {
                    continue;
                }
                let (lo, hi) = (s.ya.max(t.ya), s.yb.min(t.yb));
                if hi <= lo {
                    continue;
                }
                let (x0, x1) = (s.x_at(lo), s.x_at(hi));
                let f0 = x0 - t.x_at(lo);
                let f1 = x1 - t.x_at(hi);
                // A touch, up to rounding: a piece that starts or ends on
                // another (a cap's corner on the stroke's side, say) may
                // leave it there, so the band is cut. A cut too many
                // costs a band, never area.
                if f0.abs() <= TOUCH * (1.0 + x0.abs()) {
                    cuts.push(lo);
                }
                if f1.abs() <= TOUCH * (1.0 + x1.abs()) {
                    cuts.push(hi);
                }
                if (f0 < 0.0 && f1 > 0.0) || (f0 > 0.0 && f1 < 0.0) {
                    let c = lo + (hi - lo) * (f0 / (f0 - f1));
                    if c.is_finite() {
                        cuts.push(c);
                    }
                }
            }
        }
        cuts.sort_unstable_by(f32::total_cmp);
        cuts.dedup();
        // A band takes on the chains that start at its top and drops those
        // that end there; between bands the order changes only where two
        // chains cross, so a pass of insertion sort puts it right.
        let order = &mut self.order;
        order.clear();
        let mut top = 0;
        for c in cuts.windows(2) {
            let (s0, s1) = (c[0], c[1]);
            if s1 <= s0 {
                continue;
            }
            order.retain(|o| end[o.1] > s0);
            while top < n && segs[top].ya <= s0 {
                if !linked[top] && end[top] > s0 {
                    order.push((0.0, top, top));
                }
                top += 1;
            }
            let mid = 0.5 * (s0 + s1);
            for o in order.iter_mut() {
                while segs[o.2].yb <= s0 && next[o.2] != NONE {
                    o.2 = next[o.2];
                }
                let mut k = o.2;
                while segs[k].yb < mid && next[k] != NONE {
                    k = next[k];
                }
                o.0 = segs[k].x_at(mid);
            }
            for k in 1..order.len() {
                let mut j = k;
                while j > 0 && order[j - 1].0 > order[j].0 {
                    order.swap(j - 1, j);
                    j -= 1;
                }
            }
            let mut w = 0.0;
            for &(_, head, first) in order.iter() {
                let before = w;
                w += segs[head].dir;
                let sign = match (before == 0.0, w == 0.0) {
                    (true, false) => -1.0,
                    (false, true) => 1.0,
                    _ => continue,
                };
                let mut k = first;
                while k != NONE && segs[k].ya < s1 {
                    let s = &segs[k];
                    let (a, b) = (s.ya.max(s0), s.yb.min(s1));
                    if b > a {
                        add_edge(line, run, sign * (b - a), s.x_at(a) - x0, s.x_at(b) - x0);
                    }
                    k = next[k];
                }
            }
            // A lost edge leaves the region open to the right: it runs on
            // to the row's end.
            if w != 0.0 {
                add_edge(line, run, s1 - s0, f32::INFINITY, f32::INFINITY);
            }
        }
    }
}

/// An edge cut to one pixel row, `ya < yb` local to the row.
struct Seg {
    ya: f32,
    yb: f32,
    xa: f32,
    xb: f32,
    dir: f32,
    /// x per y.
    slope: f32,
}

impl Seg {
    fn lo(&self) -> f32 {
        self.xa.min(self.xb)
    }

    fn hi(&self) -> f32 {
        self.xa.max(self.xb)
    }

    fn x_at(&self, y: f32) -> f32 {
        if y <= self.ya {
            return self.xa;
        }
        if y >= self.yb {
            return self.xb;
        }
        self.xa + self.slope * (y - self.ya)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FINE: (f32, f32) = (1e-4, 4096.0);
    const PI: f32 = std::f32::consts::PI;

    fn style(cap: Cap, join: Join) -> Style {
        Style {
            cap,
            join,
            miter_limit: 4.0,
        }
    }

    /// The coverage of strokes of half width `half` along `paths` on a
    /// `side`² grid.
    fn cov(side: usize, half: f32, style: Style, paths: &[(&[P], bool)]) -> Vec<f32> {
        let mut o = Outline::default();
        for &(pts, closed) in paths {
            o.stroke(pts, closed, half, style, FINE);
        }
        let mut c = vec![0.0; side * side];
        o.coverage(&mut c, side, side);
        c
    }

    fn total(c: &[f32]) -> f32 {
        c.iter().sum()
    }

    fn near(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn a_line_covers_its_rectangle_and_caps() {
        let line: &[P] = &[[4.0, 12.0], [20.0, 12.0]];
        let round = cov(24, 1.0, Style::ROUND, &[(line, false)]);
        assert!(
            near(total(&round), 16.0 * 2.0 + PI, 1e-3),
            "{}",
            total(&round)
        );
        for x in 4..20 {
            assert!(near(round[11 * 24 + x], 1.0, 1e-5) && near(round[12 * 24 + x], 1.0, 1e-5));
            assert_eq!(round[10 * 24 + x], 0.0);
        }
        // Pixel (3, 11): the quarter disc of radius 1 at (4, 12).
        assert!(
            near(round[11 * 24 + 3], PI / 4.0, 1e-3),
            "{}",
            round[11 * 24 + 3]
        );
        let butt = cov(24, 1.0, style(Cap::Butt, Join::Round), &[(line, false)]);
        assert!(near(total(&butt), 32.0, 1e-3));
        assert_eq!(butt[11 * 24 + 3], 0.0);
        let square = cov(24, 1.0, style(Cap::Square, Join::Round), &[(line, false)]);
        assert!(near(total(&square), 36.0, 1e-3));
        assert!(near(square[11 * 24 + 3], 1.0, 1e-5));
        // A slanted line: the same area at any angle.
        for k in 0..12 {
            let (s, c) = (k as f32 * 0.29).sin_cos();
            let p = |t: f32| [16.0 + t * c, 16.0 + t * s];
            let l = [p(-9.0), p(9.0)];
            let a = total(&cov(32, 1.3, style(Cap::Butt, Join::Round), &[(&l, false)]));
            assert!(near(a, 18.0 * 2.6, 2e-3), "{k}: {a}");
        }
    }

    /// Two strokes closer than a pixel are counted once where they
    /// overlap: full coverage, never more.
    #[test]
    fn overlapping_strokes_count_once() {
        let a: &[P] = &[[2.0, 10.0], [22.0, 10.0]];
        let b: &[P] = &[[2.0, 10.5], [22.0, 10.5]];
        let c = cov(
            24,
            1.0,
            style(Cap::Butt, Join::Round),
            &[(a, false), (b, false)],
        );
        for x in 2..22 {
            let col = [c[8 * 24 + x], c[9 * 24 + x], c[10 * 24 + x], c[11 * 24 + x]];
            assert!(
                near(col[0], 0.0, 1e-6) && near(col[1], 1.0, 1e-5),
                "{x}: {col:?}"
            );
            assert!(
                near(col[2], 1.0, 1e-5) && near(col[3], 0.5, 1e-5),
                "{x}: {col:?}"
            );
        }
        // An X: two bars minus the square they share.
        let d1: &[P] = &[[4.0, 16.0], [28.0, 16.0]];
        let d2: &[P] = &[[16.0, 4.0], [16.0, 28.0]];
        let c = cov(
            32,
            1.0,
            style(Cap::Butt, Join::Round),
            &[(d1, false), (d2, false)],
        );
        assert!(near(total(&c), 2.0 * 48.0 - 4.0, 1e-3), "{}", total(&c));
        assert!(c.iter().all(|v| *v <= 1.0));
    }

    /// The pixel in the inside corner of a right angle is covered by both
    /// strokes' area together: 0.75, not one stroke's half.
    #[test]
    fn an_inside_corner_pixel_is_three_quarters() {
        let corner: &[P] = &[[0.0, 5.5], [5.5, 5.5], [5.5, 0.0]];
        for join in [Join::Round, Join::Miter, Join::Bevel] {
            let c = cov(12, 1.0, style(Cap::Butt, join), &[(corner, false)]);
            assert!(
                near(c[4 * 12 + 4], 0.75, 1e-5),
                "{join:?}: {}",
                c[4 * 12 + 4]
            );
        }
        // Pixel (6, 6) spans 6..7; the outer corner square 5.5..6.5 meets
        // a quarter of it. A miter covers all of that quarter, a round join
        // the part within 1 of (5.5, 5.5), a bevel (x + y <= 12) none.
        let at = |join| cov(12, 1.0, style(Cap::Butt, join), &[(corner, false)])[6 * 12 + 6];
        let (m, r, b) = (at(Join::Miter), at(Join::Round), at(Join::Bevel));
        // ∫ from 0.5 to √0.75 of (√(1 - u²) - 0.5) du
        let quarter = (0.75f32.sqrt() * 0.5 + PI / 3.0 - 0.5 * 0.75f32.sqrt() - PI / 6.0) / 2.0
            - 0.5 * (0.75f32.sqrt() - 0.5);
        assert!(near(m, 0.25, 1e-5), "miter {m}");
        assert!(near(r, quarter, 1e-3), "round {r} vs {quarter}");
        assert!(near(b, 0.0, 1e-6), "bevel {b}");
    }

    /// A miter past the limit is a bevel; the turn and the pieces' lengths
    /// do not change the area beyond the join's own.
    #[test]
    fn joins_add_their_own_area() {
        // A 90° turn with a 4 unit leg each way at half width 1.
        let corner: &[P] = &[[4.0, 12.0], [12.0, 12.0], [12.0, 4.0]];
        let a = |join| total(&cov(24, 1.0, style(Cap::Butt, join), &[(corner, false)]));
        // Two 8×2 bars share a 1×1 inside square... minus overlap.
        let base = 8.0 * 2.0 * 2.0 - 1.0;
        assert!(near(a(Join::Bevel), base + 0.5, 1e-3), "{}", a(Join::Bevel));
        assert!(near(a(Join::Miter), base + 1.0, 1e-3), "{}", a(Join::Miter));
        assert!(
            near(a(Join::Round), base + PI / 4.0, 1e-3),
            "{}",
            a(Join::Round)
        );
        // A sharp turn: the miter (1/sin(10°) ≈ 5.8 widths) is cut off.
        let sharp: &[P] = &[[2.0, 12.0], [20.0, 12.0], [2.0, 15.17]];
        let m = total(&cov(
            24,
            1.0,
            style(Cap::Butt, Join::Miter),
            &[(sharp, false)],
        ));
        let b = total(&cov(
            24,
            1.0,
            style(Cap::Butt, Join::Bevel),
            &[(sharp, false)],
        ));
        assert!(near(m, b, 1e-4), "miter {m} bevel {b}");
    }

    /// A closed square outline: the ring between two squares.
    #[test]
    fn a_closed_path_has_no_caps() {
        let sq: &[P] = &[[4.0, 4.0], [20.0, 4.0], [20.0, 20.0], [4.0, 20.0]];
        let c = cov(24, 1.0, style(Cap::Round, Join::Miter), &[(sq, true)]);
        assert!(
            near(total(&c), 18.0 * 18.0 - 14.0 * 14.0, 1e-2),
            "{}",
            total(&c)
        );
        assert!(near(c[3 * 24 + 3], 1.0, 1e-5) && c[5 * 24 + 5] == 0.0);
    }

    /// Short pieces and tight curves: a circle of many pieces whose
    /// stroke is wider than its radius fills the disc it reaches.
    #[test]
    fn a_tight_curve_fills_its_disc() {
        let r = 1.0;
        let pts: Vec<P> = (0..64)
            .map(|i| {
                let t = i as f32 / 64.0 * 2.0 * PI;
                [12.0 + r * t.cos(), 12.0 + r * t.sin()]
            })
            .collect();
        let c = cov(24, 3.0, Style::ROUND, &[(&pts, true)]);
        // The outer radius is 4 (less the 64-gon's sag), no hole.
        assert!(near(total(&c), PI * 16.0, 0.05), "{}", total(&c));
        assert!(near(c[12 * 24 + 12], 1.0, 1e-5));
    }

    #[test]
    fn a_dot_is_its_cap() {
        let p: &[P] = &[[12.0, 12.0]];
        assert!(near(
            total(&cov(24, 2.0, Style::ROUND, &[(p, false)])),
            4.0 * PI,
            1e-3
        ));
        let sq = cov(24, 2.0, style(Cap::Square, Join::Round), &[(p, false)]);
        assert!(near(total(&sq), 16.0, 1e-4));
        assert_eq!(
            total(&cov(24, 2.0, style(Cap::Butt, Join::Round), &[(p, false)])),
            0.0
        );
    }

    /// A stroke over a fill's edge: the union, no double cover, no gap.
    #[test]
    fn a_stroke_over_a_fill_is_their_union() {
        let mut o = Outline::default();
        // The fill: the square 4..12, as two triangles of either turn.
        let v = [[4.0, 4.0], [12.0, 4.0], [12.0, 12.0], [4.0, 12.0]];
        o.triangles(&v, &[0, 1, 2, 0, 3, 2], |p| p);
        // A stroke along its right edge, 2 wide.
        o.stroke(
            &[[12.0, 2.0], [12.0, 14.0]],
            false,
            1.0,
            style(Cap::Butt, Join::Round),
            FINE,
        );
        let mut c = vec![0.0; 16 * 16];
        o.coverage(&mut c, 16, 16);
        // 8×8 + 2×12 - 1×8 shared.
        assert!(near(total(&c), 64.0 + 24.0 - 8.0, 1e-3), "{}", total(&c));
        assert!(c.iter().all(|x| *x <= 1.0 + 1e-6));
        assert!(near(c[8 * 16 + 11], 1.0, 1e-6) && near(c[8 * 16 + 7], 1.0, 1e-6));
    }

    /// ∫ √(r² - t²) dt from `a` to `b` (both within -r..=r): a slice of a
    /// half disc of radius `r` between the lines t = a and t = b.
    fn half_disc_slice(r: f32, a: f32, b: f32) -> f32 {
        let f = |t: f32| {
            0.5 * (t * (r * r - t * t).max(0.0).sqrt() + r * r * (t / r).clamp(-1.0, 1.0).asin())
        };
        f(b) - f(a)
    }

    /// A convex polygon of the reference, in pixels.
    type Poly = Vec<[f64; 2]>;

    /// The area of the convex polygon `poly` inside pixel (`x`, `y`):
    /// the polygon clipped by the pixel's four sides, then the shoelace.
    fn pixel_area(poly: &[[f64; 2]], x: f64, y: f64) -> f64 {
        let mut p: Vec<[f64; 2]> = poly.to_vec();
        // Keep `sign * (q[axis] - edge) >= 0` for each side of the pixel.
        let sides = [
            (0, x, 1.0),
            (0, x + 1.0, -1.0),
            (1, y, 1.0),
            (1, y + 1.0, -1.0),
        ];
        for (axis, edge, sign) in sides {
            let side = |q: [f64; 2]| sign * (q[axis] - edge);
            let mut out = Vec::with_capacity(p.len() + 2);
            for k in 0..p.len() {
                let (a, b) = (p[k], p[(k + 1) % p.len()]);
                let (fa, fb) = (side(a), side(b));
                if fa >= 0.0 {
                    out.push(a);
                }
                if (fa >= 0.0) != (fb >= 0.0) {
                    let t = fa / (fa - fb);
                    out.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
                }
            }
            p = out;
            if p.is_empty() {
                return 0.0;
            }
        }
        let n = p.len();
        (0..n)
            .map(|k| p[k][0] * p[(k + 1) % n][1] - p[(k + 1) % n][0] * p[k][1])
            .sum::<f64>()
            .abs()
            / 2.0
    }

    /// A sub-path that turns straight back, A→B→C with C between A and B:
    /// the stroke of A–B alone plus, past B, the join (a half disc for a
    /// round join, nothing for a bevel, and a miter falls back to a bevel
    /// since it is infinitely long); C's cap lies inside A–B's stroke.
    /// Every pixel against the exact area of those pieces (convex
    /// polygons, a half disc as 512 chords, under 2e-5 px² short), so a
    /// join of the right area in the wrong place fails too. Every
    /// direction and both orders, since a reversal's turn is ±0 and
    /// either sign must work.
    #[test]
    fn a_reversal_keeps_its_join() {
        let (h, l, back) = (1.25f32, 9.0f32, 3.5f32);
        let c = [16.0f32, 16.0];
        let mut worst = 0.0f64;
        for k in 0..16 {
            let (s, co) = (k as f32 * PI / 8.0).sin_cos();
            let at = |t: f32| [c[0] + t * co, c[1] + t * s];
            // A at -l/2, B at +l/2, C back towards A.
            let path = [at(-l / 2.0), at(l / 2.0), at(l / 2.0 - back)];
            // The reference in f64, at `t` along A→B and `n` across it.
            let (hd, ld) = (f64::from(h), f64::from(l));
            let (sd, cd) = (f64::from(s), f64::from(co));
            let pt = |t: f64, n: f64| [16.0 + t * cd - n * sd, 16.0 + t * sd + n * cd];
            let quad = |t0: f64, t1: f64| vec![pt(t0, -hd), pt(t1, -hd), pt(t1, hd), pt(t0, hd)];
            // The half disc at `t` on the side `dir` (1 past B, -1 before A).
            let half_disc = |t: f64, dir: f64| {
                (0..=512)
                    .map(|i| {
                        let a = (i as f64 / 512.0 - 0.5) * std::f64::consts::PI;
                        pt(t + dir * hd * a.cos(), dir * hd * a.sin())
                    })
                    .collect::<Vec<_>>()
            };
            let rect = quad(-ld / 2.0, ld / 2.0);
            let round_cap = half_disc(-ld / 2.0, -1.0);
            let square_cap = quad(-ld / 2.0 - hd, -ld / 2.0);
            let round_join = half_disc(ld / 2.0, 1.0);
            let cases: [(Cap, Join, &[&Poly]); 7] = [
                (Cap::Round, Join::Round, &[&rect, &round_cap, &round_join]),
                (Cap::Butt, Join::Round, &[&rect, &round_join]),
                (Cap::Square, Join::Round, &[&rect, &square_cap, &round_join]),
                (Cap::Butt, Join::Bevel, &[&rect]),
                (Cap::Butt, Join::Miter, &[&rect]),
                (Cap::Round, Join::Bevel, &[&rect, &round_cap]),
                (Cap::Square, Join::Miter, &[&rect, &square_cap]),
            ];
            for (cap, join, pieces) in cases {
                let got = cov(32, h, style(cap, join), &[(&path, false)]);
                for (i, g) in got.iter().enumerate() {
                    let (x, y) = ((i % 32) as f64, (i / 32) as f64);
                    let want: f64 = pieces.iter().map(|p| pixel_area(p, x, y)).sum();
                    let e = (f64::from(*g) - want).abs();
                    worst = worst.max(e);
                    assert!(
                        e <= 2e-4,
                        "{k} {cap:?} {join:?} pixel ({x},{y}): {g} vs {want}"
                    );
                }
            }
            // Nearly a reversal (179.9°), turning either way: about the
            // same area, the join's half disc included.
            for turn in [1.0f32, -1.0] {
                let e = turn * 0.1f32.to_radians();
                let (es, ec) = e.sin_cos();
                let b = at(l / 2.0);
                let dir = [-(co * ec - s * es), -(s * ec + co * es)];
                let near_path = [
                    at(-l / 2.0),
                    b,
                    [b[0] + back * dir[0], b[1] + back * dir[1]],
                ];
                let got = total(&cov(32, h, Style::ROUND, &[(&near_path, false)]));
                let want = 2.0 * h * l + PI * h * h;
                assert!(near(got, want, 0.02), "{k} {turn}: {got} vs {want}");
            }
        }
        eprintln!("a reversal: worst pixel {worst:.2e} off");
    }

    /// The two Tabler icons where the defect showed, at 16 px (stroke 2
    /// units = 4/3 px): `currency-rupee` (`M 18 5 h -11 h 3`) and
    /// `number-23-small` (`h -1.5 h 1.5` at y = 12), their reversals'
    /// pixels against the analytic area of the strokes there.
    #[test]
    fn tabler_reversals_at_16_px() {
        let s = 16.0f32 / 24.0;
        let h = s;
        let px = |x: f32, y: f32| [x * s, y * s];
        let rupee = [px(18.0, 5.0), px(7.0, 5.0), px(10.0, 5.0)];
        let c = cov(16, h, Style::ROUND, &[(&rupee, false)]);
        // Pixel (4, 3): the joint is at (14/3, 10/3); the bar covers
        // x 14/3..5 of it, the join's half disc the rest, y 3..4 of it.
        let want = (5.0 - 14.0 / 3.0) + half_disc_slice(h, 3.0 - 10.0 / 3.0, 4.0 - 10.0 / 3.0);
        let got = c[3 * 16 + 4];
        assert!(near(got, want, 1e-3), "rupee (4,3): {got} vs {want}");
        // The joint at (10, 8): pixels (9, 7) and (9, 8) hold a quarter
        // disc each.
        let n23 = [px(16.5, 12.0), px(15.0, 12.0), px(16.5, 12.0)];
        let c = cov(16, h, Style::ROUND, &[(&n23, false)]);
        for (x, y) in [(9, 7), (9, 8)] {
            let got = c[y * 16 + x];
            assert!(
                near(got, PI * h * h / 4.0, 1e-3),
                "23-small ({x},{y}): {got}"
            );
        }
    }

    #[test]
    fn garbage_does_not_panic() {
        let bad: [&[P]; 4] = [
            &[[f32::NAN, 0.0], [3.0, 3.0]],
            &[[0.0, 0.0], [f32::INFINITY, -1e30]],
            &[[1e30, 1e30], [-1e30, 2.0], [3.0, 1e-30]],
            &[[5.0, 5.0], [5.0, 5.0], [5.0, 5.0]],
        ];
        for pts in bad {
            for half in [0.0, 1.0, 1e30, f32::NAN] {
                let c = cov(8, half, Style::ROUND, &[(pts, true), (pts, false)]);
                assert!(c.iter().all(|v| (0.0..=1.0).contains(v)));
            }
        }
    }
}
