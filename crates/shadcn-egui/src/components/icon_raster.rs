//! The CPU rasteriser behind [`paint_icon`](super::icon::paint_icon): an
//! icon becomes one coverage texture at its physical pixel size, cached in
//! the context, so a translucent colour or a faded painter applies once to
//! the whole icon and overlapping pieces never blend twice. An icon's
//! translucent layers (Hugeicons' faded parts) are composited into that one
//! texture at their own alpha.
//!
//! A turned icon (the spinner) is rasterised at its angle with the turn
//! applied to its geometry, so it keeps the edges of the unturned icon; a
//! bitmap turned on the GPU would be resampled. Its texture is reused for
//! the next angle on the next pass.
//!
//! Coverage is a distance field: strokes by the distance of each pixel
//! centre to the flattened stroke segments, which is exact for SVG round
//! caps and round joins (every shipped set uses both); fills by the distance
//! to the fill boundary, signed by the pre-triangulated inside.

use crate::FillMesh;
use egui::{
    pos2, vec2, Color32, ColorImage, Context, Id, Pos2, TextureHandle, TextureOptions, Vec2,
    ViewportId,
};
use shadcn_egui_core::{Icon, IconLayer, IconShape, Seg};
use std::collections::HashMap;

/// Curve flattening tolerance in physical pixels. A stroke's edge ramps
/// over one pixel, so a piece this far off the curve moves a pixel's
/// coverage by up to 0.015 x 255 = 3.8 levels: within the 4 levels an
/// opaque edge may differ from the exact one.
const TOLERANCE_PX: f32 = 0.015; // behaviour-ok: curve flattening tolerance, not a design token
/// At most this many line pieces per curve, so a curve with absurd control
/// points cannot allocate without bound.
const MAX_CURVE_PIECES: f32 = 256.0; // behaviour-ok: allocation bound, not a design token
/// Coverage ramps from 0 to 1 over one pixel centred on an edge.
const HALF_PX: f32 = 0.5; // behaviour-ok: anti-aliasing ramp, not a design token
/// A pixel centre this far outside a fill triangle's edge still counts as
/// inside, so rounding cannot open a seam along a shared interior edge.
const EDGE_SLACK_PX: f32 = 1e-3; // behaviour-ok: rounding slack, not a design token
/// The largest texture side; a bigger icon is drawn from a texture of at
/// most this size, scaled up.
pub(crate) const MAX_SIDE: usize = 1024;
/// The cache holds at most this many textures ...
const MAX_ENTRIES: usize = 1024;
/// ... and at most this many pixels (64 MiB of RGBA) in all.
const MAX_PIXELS: usize = 16 << 20;

/// Turned icons ([`turned_texture`]) keep at most this many textures ...
const MAX_TURNED: usize = 64;
/// ... and at most this many pixels (4 MiB of RGBA) in all.
const MAX_TURNED_PIXELS: usize = 1 << 20;
/// A turned texture its viewport has not drawn for this many passes is freed.
const KEEP_PASSES: u64 = 60;

/// What makes two icon textures the same. `Icon` compares blob records by
/// their bytes; the cache keys by address, which at worst caches an icon
/// twice when the linker duplicated its blob.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    name: &'static str,
    size: u32,
    shape: (usize, usize, u32, u32),
    w: usize,
    h: usize,
    stroke: u32,
    linear: bool,
}

impl Key {
    fn new(icon: &Icon, w: usize, h: usize, stroke_width: f32, linear: bool) -> Self {
        let shape = match icon.shape {
            IconShape::Segs(s) => (s.as_ptr() as usize, s.len(), 0, 0),
            IconShape::Blob(r) => (r.blob.as_ptr() as usize, r.blob.len(), r.start, r.len),
        };
        Self {
            name: icon.name,
            size: icon.size.to_bits(),
            shape,
            w,
            h,
            stroke: stroke_width.to_bits(),
            linear,
        }
    }
}

/// Icon textures by key, each with the tick of its last use. When full, the
/// least recently used texture goes.
#[derive(Clone, Default)]
struct IconCache {
    map: HashMap<Key, (TextureHandle, u64)>,
    tick: u64,
    pixels: usize,
}

/// The texture of `icon` at `w`×`h` physical pixels: from the cache, or
/// rasterised and uploaded once. `linear` samples it smoothed, for a
/// texture drawn turned or scaled; otherwise it is smoothed only when
/// clamped to [`MAX_SIDE`].
pub(crate) fn texture(
    ctx: &Context,
    icon: &Icon,
    w: usize,
    h: usize,
    stroke_width: f32,
    linear: bool,
) -> TextureHandle {
    let id = Id::new("shadcn_egui::icon_raster");
    let key = Key::new(icon, w, h, stroke_width, linear);
    let hit = ctx.data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<IconCache>(id);
        cache.tick += 1;
        let tick = cache.tick;
        cache.map.get_mut(&key).map(|(tex, used)| {
            *used = tick;
            tex.clone()
        })
    });
    if let Some(tex) = hit {
        return tex;
    }
    // Rasterise and upload outside `data_mut`: `load_texture` locks the
    // context too.
    let scale = w as f32 / icon.size;
    let alpha = coverage(icon, w, h, scale, stroke_width * scale);
    let image = ColorImage::new(
        [w, h],
        // token-ok: a coverage mask, tinted by the caller's colour when drawn
        alpha.into_iter().map(Color32::from_white_alpha).collect(),
    );
    // 1:1 pixels stay exact; a clamped texture is scaled up and smoothed.
    let options = if linear || w.max(h) >= MAX_SIDE {
        TextureOptions::LINEAR
    } else {
        TextureOptions::NEAREST
    };
    let tex = ctx.load_texture(format!("icon:{}", icon.name), image, options);
    ctx.data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<IconCache>(id);
        while !cache.map.is_empty()
            && (cache.map.len() >= MAX_ENTRIES || cache.pixels + w * h > MAX_PIXELS)
        {
            let oldest = cache.map.iter().min_by_key(|(_, (_, used))| *used);
            let Some((&old, _)) = oldest else { break };
            if let Some((t, _)) = cache.map.remove(&old) {
                let [ow, oh] = t.size();
                cache.pixels -= ow * oh;
            }
        }
        cache.pixels += w * h;
        let tick = cache.tick;
        if let Some((old, _)) = cache.map.insert(key, (tex.clone(), tick)) {
            let [ow, oh] = old.size();
            cache.pixels -= ow * oh;
        }
    });
    tex
}

/// Turned icon textures, one per icon, size, stroke and viewport that
/// shows it, more where one pass draws the icon at several angles. A
/// texture is rasterised again in place for the next angle, never while
/// a mesh of the same pass still shows its old angle.
#[derive(Clone, Default)]
struct TurnedCache {
    slots: Vec<Turned>,
    pixels: usize,
}

#[derive(Clone)]
struct Turned {
    key: Key,
    viewport: ViewportId,
    angle: u32,
    /// The viewport's pass that last drew it.
    pass: u64,
    tex: TextureHandle,
}

/// The margin, in pixels on each side, around a `w`×`h` box that holds
/// the box turned to any angle about its centre, its edge ramp included.
pub(crate) fn turned_margin(w: usize, h: usize) -> [usize; 2] {
    let d = (w as f32).hypot(h as f32);
    let m = |s: usize| ((d - s as f32) / 2.0).ceil().max(0.0) as usize + 1;
    [m(w), m(h)]
}

/// The texture of `icon` in a `w`×`h` physical pixel box turned clockwise
/// by `angle` radians about the box's centre, rasterised at that angle,
/// with [`turned_margin`] pixels around the box. `None` when the turned
/// textures are at their bounds, all drawn in this pass.
pub(crate) fn turned_texture(
    ctx: &Context,
    icon: &Icon,
    w: usize,
    h: usize,
    stroke_width: f32,
    angle: f32,
) -> Option<TextureHandle> {
    let id = Id::new("shadcn_egui::icon_raster::turned");
    let key = Key::new(icon, w, h, stroke_width, false);
    let [mx, my] = turned_margin(w, h);
    let (tw, th) = (w + 2 * mx, h + 2 * my);
    let viewport = ctx.viewport_id();
    let pass = ctx.cumulative_pass_nr();
    let bits = angle.to_bits();
    enum Found {
        Same(TextureHandle),
        Reuse(TextureHandle),
        New,
        Full,
    }
    let found = ctx.data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<TurnedCache>(id);
        let mut freed = 0;
        cache.slots.retain(|s| {
            let keep = s.viewport != viewport || s.pass + KEEP_PASSES >= pass;
            if !keep {
                let [ow, oh] = s.tex.size();
                freed += ow * oh;
            }
            keep
        });
        cache.pixels -= freed;
        let mine = |s: &&mut Turned| s.key == key && s.viewport == viewport;
        if let Some(s) = cache
            .slots
            .iter_mut()
            .filter(mine)
            .find(|s| s.angle == bits)
        {
            s.pass = pass;
            return Found::Same(s.tex.clone());
        }
        if let Some(s) = cache.slots.iter_mut().filter(mine).find(|s| s.pass != pass) {
            // Taken for this pass now, so no other call reuses it.
            (s.angle, s.pass) = (bits, pass);
            return Found::Reuse(s.tex.clone());
        }
        if cache.slots.len() >= MAX_TURNED || cache.pixels + tw * th > MAX_TURNED_PIXELS {
            Found::Full
        } else {
            Found::New
        }
    });
    let image = || {
        let scale = w as f32 / icon.size;
        let place = Place::turned(scale, w, h, angle);
        let alpha = placed_coverage(icon, tw, th, place, stroke_width * scale);
        // token-ok: a coverage mask, tinted by the caller's colour when drawn
        let pixels = alpha.into_iter().map(Color32::from_white_alpha).collect();
        ColorImage::new([tw, th], pixels)
    };
    // Rasterise and upload outside `data_mut`: textures lock the context too.
    match found {
        Found::Same(tex) => Some(tex),
        Found::Full => None,
        Found::Reuse(mut tex) => {
            tex.set(image(), TextureOptions::NEAREST);
            Some(tex)
        }
        Found::New => {
            let name = format!("icon:{}:turned", icon.name);
            let tex = ctx.load_texture(name, image(), TextureOptions::NEAREST);
            ctx.data_mut(|d| {
                let cache = d.get_temp_mut_or_default::<TurnedCache>(id);
                cache.pixels += tw * th;
                cache.slots.push(Turned {
                    key,
                    viewport,
                    angle: bits,
                    pass,
                    tex: tex.clone(),
                });
            });
            Some(tex)
        }
    }
}

/// How icon units map to the pixels of a texture: scaled and, for a
/// turned icon, turned about the centre of its box.
#[derive(Clone, Copy)]
struct Place {
    scale: f32,
    turn: Option<Turn>,
    /// How far a flattened curve may stray from the curve, in pixels,
    /// and the most pieces a curve is cut into.
    flatten: (f32, f32),
}

/// A clockwise turn by the angle of `sin` and `cos` about `centre` (the
/// scaled box's centre, `half` its half size), moved by `margin`.
#[derive(Clone, Copy)]
struct Turn {
    sin: f32,
    cos: f32,
    centre: Pos2,
    margin: Vec2,
    half: Vec2,
}

impl Place {
    fn scaled(scale: f32) -> Self {
        Self {
            scale,
            turn: None,
            flatten: (TOLERANCE_PX, MAX_CURVE_PIECES),
        }
    }

    /// A `w`×`h` pixel box at `scale`, turned clockwise by `angle` about
    /// its centre, in a texture with [`turned_margin`] around it.
    fn turned(scale: f32, w: usize, h: usize, angle: f32) -> Self {
        let [mx, my] = turned_margin(w, h);
        let (sin, cos) = angle.sin_cos();
        let half = vec2(w as f32 / 2.0, h as f32 / 2.0);
        Self {
            turn: Some(Turn {
                sin,
                cos,
                centre: half.to_pos2(),
                margin: vec2(mx as f32, my as f32),
                half,
            }),
            ..Self::scaled(scale)
        }
    }

    fn map(&self, x: f32, y: f32) -> Pos2 {
        let p = pos2(x * self.scale, y * self.scale);
        match self.turn {
            // Unturned points are only scaled, bit for bit as before.
            None => p,
            Some(t) => {
                let d = p - t.centre;
                t.centre + t.margin + vec2(d.x * t.cos - d.y * t.sin, d.x * t.sin + d.y * t.cos)
            }
        }
    }

    /// How much of pixel `(x, y)` lies in the turned box (the SVG's
    /// view box, which clips the icon), with the same one-pixel ramp as
    /// the strokes; 1 everywhere for an unturned icon, whose texture is
    /// the box.
    fn in_box(&self, x: usize, y: usize) -> f32 {
        let Some(t) = self.turn else { return 1.0 };
        let d = pixel_centre(x, y) - (t.centre + t.margin);
        let (u, v) = (d.x * t.cos + d.y * t.sin, d.y * t.cos - d.x * t.sin);
        let ramp = |a: f32, half: f32| (HALF_PX + half - a.abs()).clamp(0.0, 1.0);
        ramp(u, t.half.x) * ramp(v, t.half.y)
    }
}

/// Coverage (0..=255) of `icon` on a `w`×`h` pixel grid, row by row.
/// `scale` maps icon units to pixels, `stroke_px` is the stroke width in
/// pixels. Each layer is rasterised on its own, scaled by its alpha and
/// composited over the layers before it. Any input gives an image, never a
/// panic.
pub(crate) fn coverage(icon: &Icon, w: usize, h: usize, scale: f32, stroke_px: f32) -> Vec<u8> {
    placed_coverage(icon, w, h, Place::scaled(scale), stroke_px)
}

/// [`coverage`] with the icon's points mapped by `place`.
fn placed_coverage(icon: &Icon, w: usize, h: usize, place: Place, stroke_px: f32) -> Vec<u8> {
    let mut total: Vec<f32> = vec![0.0; w * h];
    for layer in icon.layers() {
        let cov = layer_coverage(&layer, w, h, place, stroke_px);
        let alpha = layer.alpha.clamp(0.0, 1.0);
        for (t, c) in total.iter_mut().zip(cov) {
            let src = c * alpha;
            *t = src + *t * (1.0 - src);
        }
    }
    if place.turn.is_some() {
        for (i, t) in total.iter_mut().enumerate() {
            *t *= place.in_box(i % w, i / w);
        }
    }
    total
        .into_iter()
        .map(|c| (c * f32::from(u8::MAX)).round() as u8)
        .collect()
}

/// Coverage (0..=1) of one layer: its strokes and its fill, overlaps once.
fn layer_coverage(layer: &IconLayer, w: usize, h: usize, place: Place, stroke_px: f32) -> Vec<f32> {
    let mut cov: Vec<f32> = vec![0.0; w * h];
    if stroke_px > 0.0 && stroke_px.is_finite() {
        let half = stroke_px / 2.0;
        let mut dist = vec![f32::INFINITY; w * h];
        for (a, b) in stroke_segments(layer.segs.clone(), place) {
            min_distance(&mut dist, w, h, a, b, half + HALF_PX);
        }
        for (c, d) in cov.iter_mut().zip(&dist) {
            *c = (HALF_PX + half - d).clamp(0.0, 1.0);
        }
    }
    if let Some(fill) = &layer.fill {
        fill_coverage(&mut cov, w, h, fill, place);
    }
    cov
}

/// Raises `cov` to the fill's coverage where that is higher.
fn fill_coverage(cov: &mut [f32], w: usize, h: usize, fill: &FillMesh, place: Place) {
    let map = |[x, y]: [f32; 2]| place.map(x, y);
    let mut inside = vec![false; w * h];
    for &t in fill.indices.as_chunks::<3>().0 {
        let [a, b, c] = t.map(|i| {
            fill.vertices
                .get(usize::from(i))
                .copied()
                .map_or(pos2(f32::NAN, f32::NAN), map)
        });
        mark_triangle(&mut inside, w, h, [a, b, c]);
    }
    let mut dist = vec![f32::INFINITY; w * h];
    for contour in &fill.contours {
        let pts: Vec<Pos2> = contour.iter().map(|&(p, _)| map(p)).collect();
        let Some(&last) = pts.last() else { continue };
        let mut prev = last;
        for &p in &pts {
            min_distance(&mut dist, w, h, prev, p, HALF_PX);
            prev = p;
        }
    }
    for ((c, d), &inn) in cov.iter_mut().zip(&dist).zip(&inside) {
        let f = if inn { HALF_PX + d } else { HALF_PX - d };
        *c = c.max(f.clamp(0.0, 1.0));
    }
}

/// Marks the pixels whose centre lies in the triangle (either orientation).
fn mark_triangle(inside: &mut [bool], w: usize, h: usize, [a, b, c]: [Pos2; 3]) {
    let area2 = (b - a).x * (c - a).y - (c - a).x * (b - a).y;
    if area2.abs() <= f32::EPSILON || !area2.is_finite() {
        return;
    }
    let sign = area2.signum();
    let edges = [(a, b), (b, c), (c, a)].map(|(p, q)| (p, q - p, (q - p).length()));
    let (xs, ys) = (
        span(a.x.min(b.x).min(c.x), a.x.max(b.x).max(c.x), 0.0, w),
        span(a.y.min(b.y).min(c.y), a.y.max(b.y).max(c.y), 0.0, h),
    );
    for y in ys {
        for x in xs.clone() {
            let p = pixel_centre(x, y);
            let hit = edges.iter().all(|&(o, e, len)| {
                let cross = e.x * (p - o).y - e.y * (p - o).x;
                cross * sign >= -EDGE_SLACK_PX * len
            });
            if hit {
                inside[y * w + x] = true;
            }
        }
    }
}

/// Lowers `dist` to the distance from each pixel centre within `reach` of
/// the segment `a`–`b` (a point when they coincide).
fn min_distance(dist: &mut [f32], w: usize, h: usize, a: Pos2, b: Pos2, reach: f32) {
    let ab = b - a;
    let len2 = ab.length_sq();
    for y in span(a.y.min(b.y), a.y.max(b.y), reach, h) {
        for x in span(a.x.min(b.x), a.x.max(b.x), reach, w) {
            let p = pixel_centre(x, y);
            let t = if len2 > 0.0 {
                ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = (p - (a + ab * t)).length();
            let slot = &mut dist[y * w + x];
            *slot = slot.min(d);
        }
    }
}

/// The pixel indices whose centres lie in `lo - reach ..= hi + reach`,
/// clipped to `0..n`. Non-finite bounds give an empty or clipped range.
fn span(lo: f32, hi: f32, reach: f32, n: usize) -> std::ops::Range<usize> {
    let first = (lo - reach - HALF_PX).ceil().max(0.0);
    let end = (hi + reach - HALF_PX).floor() + 1.0;
    // `as` saturates: NaN becomes 0, huge values become `usize::MAX`.
    (first as usize).min(n)..(end.max(0.0) as usize).min(n)
}

fn pixel_centre(x: usize, y: usize) -> Pos2 {
    pos2(x as f32 + HALF_PX, y as f32 + HALF_PX)
}

/// A layer's strokes as line pieces in pixels. `M` starts a sub-path, `Z`
/// closes it and returns to its start, a lone `M` is a dot (a round cap); a
/// curve without a start point is skipped, a line without one starts a
/// sub-path.
fn stroke_segments(segs: impl Iterator<Item = Seg>, place: Place) -> Vec<(Pos2, Pos2)> {
    let map = |x: f32, y: f32| place.map(x, y);
    let mut out = Vec::new();
    let (mut start, mut cur): (Option<Pos2>, Option<Pos2>) = (None, None);
    let mut drawn = false;
    fn dot(out: &mut Vec<(Pos2, Pos2)>, start: Option<Pos2>, drawn: bool) {
        if let (Some(s), false) = (start, drawn) {
            out.push((s, s));
        }
    }
    for seg in segs {
        match seg {
            Seg::M(x, y) => {
                dot(&mut out, start, drawn);
                let p = map(x, y);
                (start, cur, drawn) = (Some(p), Some(p), false);
            }
            Seg::L(x, y) => {
                let p = map(x, y);
                match cur {
                    Some(c) => {
                        out.push((c, p));
                        drawn = true;
                    }
                    None => (start, drawn) = (Some(p), false),
                }
                cur = Some(p);
            }
            Seg::Q(x1, y1, x, y) => {
                let Some(c) = cur else { continue };
                let p = map(x, y);
                curve(&mut out, &[c, map(x1, y1), p], place.flatten);
                (cur, drawn) = (Some(p), true);
            }
            Seg::C(x1, y1, x2, y2, x, y) => {
                let Some(c) = cur else { continue };
                let p = map(x, y);
                curve(&mut out, &[c, map(x1, y1), map(x2, y2), p], place.flatten);
                (cur, drawn) = (Some(p), true);
            }
            Seg::Z => {
                if let (Some(c), Some(s)) = (cur, start) {
                    out.push((c, s));
                    (cur, drawn) = (Some(s), true);
                }
            }
        }
    }
    dot(&mut out, start, drawn);
    out
}

/// Flattens a quadratic (3 points) or cubic (4 points) Bézier into uniform
/// pieces, enough that none strays more than `tolerance` pixels from the
/// curve, and at most `max_pieces`: the deviation of `n` pieces is at most
/// `max|B''| / (8 n²)`.
fn curve(out: &mut Vec<(Pos2, Pos2)>, p: &[Pos2], (tolerance, max_pieces): (f32, f32)) {
    let second = p
        .windows(3)
        .map(|w| (w[0].to_vec2() - w[1].to_vec2() * 2.0 + w[2].to_vec2()).length())
        .fold(0.0, f32::max);
    let degree = (p.len() - 1) as f32;
    let bound = degree * (degree - 1.0) * second;
    let n = (bound / (8.0 * tolerance)) // behaviour-ok: from the error bound above
        .sqrt()
        .ceil()
        .clamp(1.0, max_pieces);
    let n = if n.is_nan() { 1 } else { n as usize };
    let mut prev = p[0];
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let q = bezier(p, t);
        out.push((prev, q));
        prev = q;
    }
}

fn bezier(p: &[Pos2], t: f32) -> Pos2 {
    let mut pts: Vec<Pos2> = p.to_vec();
    while pts.len() > 1 {
        pts = pts.windows(2).map(|w| w[0].lerp(w[1], t)).collect();
    }
    pts[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon(segs: &'static [Seg]) -> Icon {
        Icon {
            name: "test",
            size: 24.0,
            shape: IconShape::Segs(segs),
        }
    }

    #[test]
    fn a_line_has_full_rows_and_round_caps() {
        static LINE: [Seg; 2] = [Seg::M(4.0, 12.0), Seg::L(20.0, 12.0)];
        let cov = coverage(&icon(&LINE), 24, 24, 1.0, 2.0);
        let at = |x: usize, y: usize| cov[y * 24 + x];
        for x in 5..=19 {
            assert!(at(x, 11) >= 250 && at(x, 12) >= 250, "full at x {x}");
            assert_eq!((at(x, 9), at(x, 14)), (0, 0), "clear at x {x}");
        }
        // Pixel (3, 11) has its centre 0.5 left of and 0.5 above the cap
        // centre: a round cap covers it partly, a square cap fully.
        let corner = at(3, 11);
        assert!(corner > 0 && corner < 250, "round cap corner {corner}");
        assert_eq!(at(2, 11), 0, "nothing beyond the cap");
    }

    #[test]
    fn a_lone_move_is_a_dot() {
        static DOT: [Seg; 1] = [Seg::M(12.0, 12.0)];
        let cov = coverage(&icon(&DOT), 24, 24, 1.0, 2.0);
        assert!(cov[11 * 24 + 11] > 0 && cov[12 * 24 + 12] > 0);
        assert_eq!(cov[12 * 24 + 15], 0);
    }

    #[test]
    fn a_curve_stays_within_the_tolerance() {
        // A quarter circle of radius 10 around (2, 2) as a cubic; its
        // midpoint lies 10 from the centre.
        static ARC: [Seg; 2] = [
            Seg::M(12.0, 2.0),
            Seg::C(12.0, 7.523, 7.523, 12.0, 2.0, 12.0),
        ];
        let segs = stroke_segments(icon(&ARC).layers().flat_map(|l| l.segs), Place::scaled(1.0));
        assert!(segs.len() > 4, "flattened into {} pieces", segs.len());
        for (a, _) in segs {
            let r = (a - pos2(2.0, 2.0)).length();
            assert!((r - 10.0).abs() < 0.11, "radius {r}");
        }
    }

    /// The worst difference, in levels of 255, between `icon` rasterised as
    /// painted and with every curve cut into 512 pieces (the curve itself
    /// for all a pixel can show), at `side` pixels and `angle`.
    fn curve_error(icon: &Icon, side: usize, angle: f32) -> u8 {
        let scale = side as f32 / icon.size;
        let (place, w) = if angle == 0.0 {
            (Place::scaled(scale), side)
        } else {
            let [m, _] = turned_margin(side, side);
            (Place::turned(scale, side, side, angle), side + 2 * m)
        };
        let fine = Place {
            flatten: (0.0, 512.0),
            ..place
        };
        let painted = placed_coverage(icon, w, w, place, 2.0 * scale);
        let exact = placed_coverage(icon, w, w, fine, 2.0 * scale);
        painted
            .iter()
            .zip(&exact)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0)
    }

    /// Precision by visibility: a flattened curve stays within 4 levels of
    /// the curve on an opaque icon, at any size and angle.
    #[test]
    fn a_curve_is_within_four_levels_of_the_true_curve() {
        use crate::icons::lucide;
        let mut fails = Vec::new();
        for icon in [
            lucide::CIRCLE,
            lucide::LOADER_CIRCLE,
            lucide::HEART,
            lucide::SEARCH,
        ] {
            for side in [12, 16, 20, 24, 32, 48, 64, 96, 192] {
                for angle in [0.0, 0.3, 1.1, 2.0] {
                    let e = curve_error(&icon, side, angle);
                    eprintln!("CURVE {} {side} {angle}: {e}", icon.name);
                    if e > 4 {
                        fails.push(format!("{} {side} px {angle} rad: {e}", icon.name));
                    }
                }
            }
        }
        assert!(fails.is_empty(), "{fails:#?}");
    }

    #[test]
    fn garbage_does_not_panic() {
        static BAD: [&[Seg]; 4] = [
            &[Seg::Q(1.0, 1.0, 2.0, 2.0), Seg::L(4.0, 4.0)],
            &[Seg::C(1.0, 1.0, 2.0, 2.0, 3.0, 3.0), Seg::Z],
            &[Seg::M(f32::NAN, 0.0), Seg::L(f32::INFINITY, -1e30)],
            &[
                Seg::M(0.0, 0.0),
                Seg::C(1e30, -1e30, f32::NAN, 1e30, 3.0, 3.0),
            ],
        ];
        for segs in BAD {
            for (w, h) in [(0, 0), (1, 1), (24, 24)] {
                assert_eq!(coverage(&icon(segs), w, h, 1.0, 2.0).len(), w * h);
            }
        }
    }
}
