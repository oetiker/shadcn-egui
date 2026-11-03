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
//! Coverage is exact area: each pixel gets the share of it that a layer
//! covers. A layer's strokes become closed outlines (round caps and round
//! joins, as every shipped set draws them), and the pixel area of the union
//! of those outlines and the fill's triangles is found row by row, so
//! strokes that meet or overlap count once. A fill alone takes the triangles
//! directly, which never overlap.

use egui::{
    pos2, vec2, Color32, ColorImage, Context, Id, Pos2, TextureHandle, TextureOptions, Vec2,
    ViewportId,
};
use shadcn_egui_core::{Icon, IconLayer, IconShape, Seg};
use std::collections::HashMap;

mod fill;
mod stroke;

/// Curve flattening tolerance in physical pixels. A piece this far off the
/// curve moves the stroke's edge by as much, a pixel's coverage by up to
/// 0.01 x 255 = 2.6 levels: within the 4 levels an opaque edge may differ
/// from the exact one, rounding to whole levels included.
const TOLERANCE_PX: f32 = 0.01; // behaviour-ok: curve flattening tolerance, not a design token
/// At most this many line pieces per curve, so a curve with absurd control
/// points cannot allocate without bound.
const MAX_CURVE_PIECES: f32 = 256.0; // behaviour-ok: allocation bound, not a design token
/// Coverage ramps from 0 to 1 over one pixel centred on an edge.
const HALF_PX: f32 = 0.5; // behaviour-ok: anti-aliasing ramp, not a design token
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

/// Coverage (0..=1) of one layer: the exact area of the union of its
/// strokes and its fill, so overlaps count once. A fill alone takes the
/// fill raster, whose triangles never overlap.
fn layer_coverage(layer: &IconLayer, w: usize, h: usize, place: Place, stroke_px: f32) -> Vec<f32> {
    let mut cov: Vec<f32> = vec![0.0; w * h];
    let map = |[x, y]: [f32; 2]| place.map(x, y).into();
    let paths = if stroke_px > 0.0 && stroke_px.is_finite() {
        stroke_paths(layer.segs.clone(), place)
    } else {
        Vec::new()
    };
    if paths.is_empty() {
        if let Some(fill) = &layer.fill {
            fill::fill_coverage(&mut cov, w, h, &fill.vertices, &fill.indices, map);
        }
        return cov;
    }
    let mut outline = stroke::Outline::default();
    for (pts, closed) in &paths {
        outline.stroke(
            pts,
            *closed,
            stroke_px / 2.0,
            stroke::Style::ROUND,
            place.flatten,
        );
    }
    if let Some(fill) = &layer.fill {
        outline.triangles(&fill.vertices, &fill.indices, map);
    }
    outline.coverage(&mut cov, w, h);
    cov
}

fn pixel_centre(x: usize, y: usize) -> Pos2 {
    pos2(x as f32 + HALF_PX, y as f32 + HALF_PX)
}

/// A layer's strokes as sub-paths of points in pixels, each with whether
/// `Z` closed it. `M` starts a sub-path; a lone `M` is a dot; a curve
/// without a start point is skipped, a line without one starts a sub-path.
fn stroke_paths(segs: impl Iterator<Item = Seg>, place: Place) -> Vec<(Vec<[f32; 2]>, bool)> {
    let map = |x: f32, y: f32| <[f32; 2]>::from(place.map(x, y));
    let mut out: Paths = Vec::new();
    // Whether the last sub-path takes more pieces, and where a piece
    // after `Z` starts.
    let (mut open, mut after_z) = (false, None);
    type Paths = Vec<(Vec<[f32; 2]>, bool)>;
    // Starts a sub-path for a piece that has none open.
    fn current(out: &mut Paths, open: &mut bool, after_z: &mut Option<[f32; 2]>) {
        if !*open {
            out.push((after_z.take().into_iter().collect(), false));
            *open = true;
        }
    }
    for seg in segs {
        match seg {
            Seg::M(x, y) => {
                out.push((vec![map(x, y)], false));
                (open, after_z) = (true, None);
            }
            Seg::L(x, y) => {
                current(&mut out, &mut open, &mut after_z);
                if let Some((pts, _)) = out.last_mut() {
                    pts.push(map(x, y));
                }
            }
            Seg::Q(x1, y1, x, y) => {
                current(&mut out, &mut open, &mut after_z);
                let Some((pts, _)) = out.last_mut() else {
                    continue;
                };
                let Some(&c) = pts.last() else { continue };
                curve(pts, &[c, map(x1, y1), map(x, y)], place.flatten);
            }
            Seg::C(x1, y1, x2, y2, x, y) => {
                current(&mut out, &mut open, &mut after_z);
                let Some((pts, _)) = out.last_mut() else {
                    continue;
                };
                let Some(&c) = pts.last() else { continue };
                curve(
                    pts,
                    &[c, map(x1, y1), map(x2, y2), map(x, y)],
                    place.flatten,
                );
            }
            Seg::Z => {
                if let Some((pts, closed)) = out.last_mut().filter(|_| open) {
                    if let Some(&start) = pts.first() {
                        *closed = true;
                        (open, after_z) = (false, Some(start));
                    }
                }
            }
        }
    }
    out.retain(|(pts, _)| !pts.is_empty());
    out
}

/// Flattens a quadratic (3 points) or cubic (4 points) Bézier into uniform
/// pieces, enough that none strays more than `tolerance` pixels from the
/// curve, and at most `max_pieces`: the deviation of `n` pieces is at most
/// `max|B''| / (8 n²)`.
fn curve(out: &mut Vec<[f32; 2]>, p: &[[f32; 2]], (tolerance, max_pieces): (f32, f32)) {
    let p: Vec<Pos2> = p.iter().map(|&q| q.into()).collect();
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
    for i in 1..=n {
        let t = i as f32 / n as f32;
        out.push(bezier(&p, t).into());
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

    /// Where two strokes meet, a pixel gets the area their union covers,
    /// not the coverage of the nearer one: the pixel in the inside corner
    /// of a right angle is three quarters covered (191 of 255), and the
    /// strokes of an X fill the corners between their arms the same way.
    #[test]
    fn strokes_cover_each_pixel_by_their_union() {
        static CORNER: [Seg; 3] = [Seg::M(0.0, 5.5), Seg::L(5.5, 5.5), Seg::L(5.5, 0.0)];
        let cov = coverage(&icon(&CORNER), 12, 12, 1.0, 2.0);
        assert_eq!(cov[4 * 12 + 4], 191, "inside corner");
        static CROSS: [Seg; 4] = [
            Seg::M(2.0, 12.5),
            Seg::L(22.0, 12.5),
            Seg::M(12.5, 2.0),
            Seg::L(12.5, 22.0),
        ];
        let cov = coverage(&icon(&CROSS), 24, 24, 1.0, 2.0);
        for (x, y) in [(11, 11), (13, 11), (11, 13), (13, 13)] {
            assert_eq!(cov[y * 24 + x], 191, "corner pixel ({x}, {y})");
        }
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
        let paths = stroke_paths(icon(&ARC).layers().flat_map(|l| l.segs), Place::scaled(1.0));
        let pts = &paths[0].0;
        assert!(pts.len() > 4, "flattened into {} pieces", pts.len());
        for &a in pts {
            let r = (Pos2::from(a) - pos2(2.0, 2.0)).length();
            assert!((r - 10.0).abs() < 0.11, "radius {r}");
        }
    }

    /// The worst difference, in levels of 255, between `icon` rasterised as
    /// painted and with every curve and arc flattened to 1e-4 px (the curve itself
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
            flatten: (1e-4, 4096.0),
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

    /// The fill's coverage of `tris` (pixel coordinates) on a `side`² grid.
    fn fill_cov(side: usize, tris: &[[[f32; 2]; 3]]) -> Vec<f32> {
        let vertices: Vec<[f32; 2]> = tris.iter().flatten().copied().collect();
        let indices: Vec<u16> = (0..vertices.len() as u16).collect();
        let mut cov = vec![0.0; side * side];
        fill::fill_coverage(&mut cov, side, side, &vertices, &indices, |p| p);
        cov
    }

    /// A fill covers each pixel by the area of it that lies inside.
    #[test]
    fn a_fill_covers_each_pixel_by_its_area() {
        // A square from 2.25 to 5.5, as two triangles of either turn.
        let (a, b, c, d) = ([2.25, 2.25], [5.5, 2.25], [5.5, 5.5], [2.25, 5.5]);
        let cov = fill_cov(8, &[[a, b, c], [a, d, c]]);
        let at = |x: usize, y: usize| cov[y * 8 + x];
        assert!((at(2, 2) - 0.75 * 0.75).abs() < 1e-5);
        assert!((at(3, 3) - 1.0).abs() < 1e-5);
        assert!((at(5, 2) - 0.75 * 0.5).abs() < 1e-5);
        assert!((at(4, 5) - 0.5).abs() < 1e-5);
        assert_eq!(at(6, 6), 0.0);
        // A 45° edge through pixel corners halves the diagonal pixels.
        let cov = fill_cov(8, &[[[0.0, 0.0], [8.0, 8.0], [0.0, 8.0]]]);
        for i in 0..8 {
            assert!(
                (cov[i * 8 + i] - 0.5).abs() < 1e-5,
                "{i}: {}",
                cov[i * 8 + i]
            );
            assert_eq!(cov[i * 8 + (i + 1).min(7)], if i == 7 { 0.5 } else { 0.0 });
        }
        // A sliver 0.1 px thin across a row covers each pixel by 0.1.
        let cov = fill_cov(
            8,
            &[
                [[0.0, 3.2], [8.0, 3.2], [8.0, 3.3]],
                [[0.0, 3.2], [8.0, 3.3], [0.0, 3.3]],
            ],
        );
        assert!((1..7).all(|x| (cov[3 * 8 + x] - 0.1).abs() < 1e-5));
    }

    /// Triangles that meet along a slanted edge add up to full coverage:
    /// no seam, at any angle.
    #[test]
    fn abutting_triangles_leave_no_seam() {
        for k in 0..16 {
            let t = k as f32 * 0.37;
            let (s, c) = t.sin_cos();
            let p = |x: f32, y: f32| [16.0 + x * c - y * s, 16.0 + x * s + y * c];
            let quad = [p(-9.0, -7.3), p(8.1, -7.3), p(8.1, 6.6), p(-9.0, 6.6)];
            let cov = fill_cov(
                32,
                &[[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]],
            );
            for y in 12..20 {
                for x in 12..20 {
                    let v = cov[y * 32 + x];
                    assert!((v - 1.0).abs() < 1e-5, "{t} rad ({x}, {y}): {v}");
                }
            }
            let total: f32 = cov.iter().sum();
            assert!((total - 17.1 * 13.9).abs() < 2e-3, "{t} rad: area {total}");
        }
    }

    /// The previous fill raster: each row cut out of the triangle, each
    /// pixel the row's area left of its right side minus left of its left
    /// side. The per-row edge walk must give the same coverage.
    fn fill_cov_by_clipping(side: usize, tris: &[[[f32; 2]; 3]]) -> Vec<f32> {
        type P = [f32; 2];
        fn cut(pts: &[P], d: impl Fn(P) -> f32, on: impl Fn(P) -> P) -> Vec<P> {
            let mut out = Vec::new();
            for (i, &q) in pts.iter().enumerate() {
                let p = pts[(i + pts.len() - 1) % pts.len()];
                let (dp, dq) = (d(p), d(q));
                if (dp >= 0.0) != (dq >= 0.0) {
                    let t = dp / (dp - dq);
                    out.push(on([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]));
                }
                if dq >= 0.0 {
                    out.push(q);
                }
            }
            out
        }
        fn left_of(pts: &[P], x: f32) -> f32 {
            let part = cut(pts, |p| x - p[0], |p| [x, p[1]]);
            let mut sum = 0.0;
            for (i, &q) in part.iter().enumerate() {
                let p = part[(i + part.len() - 1) % part.len()];
                sum += (p[0] - q[0]) * (p[1] + q[1]);
            }
            sum / 2.0
        }
        let mut cov = vec![0.0f32; side * side];
        for &[a, b, c] in tris {
            let turn = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            let tri = if turn >= 0.0 { [a, b, c] } else { [a, c, b] };
            for y in 0..side {
                let y0 = y as f32;
                let local: Vec<P> = tri.iter().map(|p| [p[0], p[1] - y0]).collect();
                let row = cut(&local, |p| p[1], |p| [p[0], 0.0]);
                let row = cut(&row, |p| 1.0 - p[1], |p| [p[0], 1.0]);
                if row.len() < 3 {
                    continue;
                }
                for x in 0..side {
                    let (l, r) = (x as f32, x as f32 + 1.0);
                    cov[y * side + x] += left_of(&row, r) - left_of(&row, l);
                }
            }
        }
        cov.iter().map(|a| a.clamp(0.0, 1.0)).collect()
    }

    /// The per-row edge walk matches clipping each pixel out of each row,
    /// within f32 rounding far below one level, on random triangles of
    /// every size and slant, slivers included.
    #[test]
    fn the_edge_walk_matches_clipping() {
        let mut seed = 0x2545_f491_u32;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as f32 / u32::MAX as f32
        };
        let mut worst = 0.0f32;
        for n in 0..400 {
            let scale = [1.0, 4.0, 15.0, 30.0][n % 4];
            let mut p = || [1.0 + rnd() * scale, 1.0 + rnd() * scale];
            let (a, b) = (p(), p());
            // Every fifth triangle is a sliver: c close to the line ab.
            let c = if n % 5 == 0 {
                let t = rnd();
                [
                    a[0] + (b[0] - a[0]) * t + 0.01 * rnd(),
                    a[1] + (b[1] - a[1]) * t,
                ]
            } else {
                p()
            };
            let tris = [[a, b, c]];
            let new = fill_cov(32, &tris);
            let old = fill_cov_by_clipping(32, &tris);
            for (x, y) in new.iter().zip(&old) {
                worst = worst.max((x - y).abs());
            }
        }
        assert!(
            worst * 255.0 < 0.05,
            "worst difference {} levels",
            worst * 255.0
        );
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
