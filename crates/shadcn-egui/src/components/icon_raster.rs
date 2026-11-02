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
use shadcn_egui_core::{Icon, IconShape, Seg};
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
/// A turned texture not drawn for this many passes, of any viewport, is
/// freed; so are the textures of a viewport that is gone.
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
    let place = Place::scaled(scale);
    let pixels = placed_coverage(icon, w, h, place, stroke_width * scale, mask);
    let image = ColorImage::new([w, h], pixels);
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
    /// The passes ended so far, of all viewports.
    clock: u64,
    /// Whether the end-of-pass hook that frees old textures is installed.
    hooked: bool,
}

#[derive(Clone)]
struct Turned {
    key: Key,
    viewport: ViewportId,
    angle: u32,
    /// The viewport's pass that last drew it.
    pass: u64,
    /// The [`TurnedCache::clock`] when it was last drawn.
    used: u64,
    tex: TextureHandle,
}

impl TurnedCache {
    /// Frees the slots `free` picks, oldest first, while `more` says more
    /// room is needed.
    fn free(&mut self, free: impl Fn(&Turned) -> bool, more: impl Fn(&Self) -> bool) {
        while more(self) {
            let oldest = (self.slots.iter().enumerate())
                .filter(|(_, s)| free(s))
                .min_by_key(|(_, s)| s.used)
                .map(|(i, _)| i);
            let Some(i) = oldest else { break };
            let [w, h] = self.slots.swap_remove(i).tex.size();
            self.pixels -= w * h;
        }
    }

    /// At the end of every pass: frees the textures no pass has drawn for
    /// [`KEEP_PASSES`] passes, also those of a viewport that is gone or
    /// shows no turned icon any more.
    fn end_pass(&mut self) {
        self.clock += 1;
        let clock = self.clock;
        let old = |s: &Turned| s.used + KEEP_PASSES < clock;
        self.free(old, |c| c.slots.iter().any(old));
    }
}

fn turned_id() -> Id {
    Id::new("shadcn_egui::icon_raster::turned")
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
/// textures are at their bounds, all drawn in this pass by this viewport
/// or drawn lately by another.
pub(crate) fn turned_texture(
    ctx: &Context,
    icon: &Icon,
    w: usize,
    h: usize,
    stroke_width: f32,
    angle: f32,
) -> Option<TextureHandle> {
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
    let (found, hook) = ctx.data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<TurnedCache>(turned_id());
        let hook = !std::mem::replace(&mut cache.hooked, true);
        let used = cache.clock;
        let mine = |s: &&mut Turned| s.key == key && s.viewport == viewport;
        if let Some(s) = cache
            .slots
            .iter_mut()
            .filter(mine)
            .find(|s| s.angle == bits)
        {
            (s.pass, s.used) = (pass, used);
            return (Found::Same(s.tex.clone()), hook);
        }
        if let Some(s) = cache.slots.iter_mut().filter(mine).find(|s| s.pass != pass) {
            // Taken for this pass now, so no other call reuses it.
            (s.angle, s.pass, s.used) = (bits, pass, used);
            return (Found::Reuse(s.tex.clone()), hook);
        }
        // Room from this viewport's textures that this pass has not drawn.
        let full =
            |c: &TurnedCache| c.slots.len() >= MAX_TURNED || c.pixels + tw * th > MAX_TURNED_PIXELS;
        cache.free(|s| s.viewport == viewport && s.pass != pass, full);
        (if full(cache) { Found::Full } else { Found::New }, hook)
    });
    if hook {
        ctx.on_end_pass(
            "shadcn_egui::turned_icons",
            std::sync::Arc::new(|ui: &mut egui::Ui| {
                ui.ctx().data_mut(|d| {
                    d.get_temp_mut_or_default::<TurnedCache>(turned_id())
                        .end_pass();
                });
            }),
        );
    }
    let image = || {
        let scale = w as f32 / icon.size;
        let place = Place::turned(scale, w, h, angle);
        let pixels = placed_coverage(icon, tw, th, place, stroke_width * scale, mask);
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
                let cache = d.get_temp_mut_or_default::<TurnedCache>(turned_id());
                cache.pixels += tw * th;
                let used = cache.clock;
                cache.slots.push(Turned {
                    key,
                    viewport,
                    angle: bits,
                    pass,
                    used,
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

    /// The columns of row `y` where [`Place::in_box`] is 1 for certain:
    /// their centres lie inside the turned box by more than its ramp, with
    /// [`REGION_SLACK_PX`] to spare for rounding. All of them for an
    /// unturned icon.
    fn inner_cols(&self, y: usize) -> std::ops::Range<usize> {
        let Some(t) = self.turn else {
            return 0..usize::MAX;
        };
        let c = t.centre + t.margin;
        let dy = pixel_centre(0, y).y - c.y;
        let (u, v) = (
            t.half.x - HALF_PX - REGION_SLACK_PX,
            t.half.y - HALF_PX - REGION_SLACK_PX,
        );
        // |dx cos + dy sin| <= u and |dy cos - dx sin| <= v, as four
        // half-planes n * dx + rest >= 0.
        let planes = [
            (t.cos, dy * t.sin + u),
            (-t.cos, -dy * t.sin + u),
            (-t.sin, dy * t.cos + v),
            (t.sin, -dy * t.cos + v),
        ];
        let (mut lo, mut hi) = (f32::NEG_INFINITY, f32::INFINITY);
        for (n, rest) in planes {
            if n > 0.0 {
                lo = lo.max(-rest / n);
            } else if n < 0.0 {
                hi = hi.min(-rest / n);
            } else if rest < 0.0 {
                return 0..0;
            }
        }
        if lo.is_nan() || hi.is_nan() || lo > hi {
            return 0..0;
        }
        span(c.x + lo, c.x + hi, 0.0, usize::MAX)
    }
}

/// Coverage (0..=255) of `icon` on a `w`×`h` pixel grid, row by row.
/// `scale` maps icon units to pixels, `stroke_px` is the stroke width in
/// pixels. Each layer is rasterised on its own, scaled by its alpha and
/// composited over the layers before it. Any input gives an image, never a
/// panic.
pub(crate) fn coverage(icon: &Icon, w: usize, h: usize, scale: f32, stroke_px: f32) -> Vec<u8> {
    placed_coverage(icon, w, h, Place::scaled(scale), stroke_px, |c| c)
}

/// [`coverage`] with the icon's points mapped by `place`, each level made
/// a pixel by `pixel`. Only the pixels the icon can reach are worked on;
/// all others stay the default pixel, which `pixel(0)` must equal.
fn placed_coverage<T: Copy + Default>(
    icon: &Icon,
    w: usize,
    h: usize,
    place: Place,
    stroke_px: f32,
    pixel: impl Fn(u8) -> T,
) -> Vec<T> {
    let stroked = stroke_px > 0.0 && stroke_px.is_finite();
    let half = stroke_px / 2.0;
    let layers: Vec<Placed> = icon
        .layers()
        .map(|layer| Placed {
            alpha: layer.alpha.clamp(0.0, 1.0),
            pieces: if stroked {
                stroke_segments(layer.segs, place)
            } else {
                Vec::new()
            },
            fill: layer.fill,
        })
        .collect();
    let area = Area::reached(&layers, w, h, place, half + HALF_PX);
    let mut total: Option<Vec<f32>> = None;
    for layer in &layers {
        let mut cov = layer_coverage(layer, area, place, stroked.then_some(half));
        match &mut total {
            // Over nothing, a layer is its own coverage times its alpha.
            None => {
                if layer.alpha != 1.0 {
                    cov.iter_mut().for_each(|c| *c *= layer.alpha);
                }
                total = Some(cov);
            }
            Some(total) => {
                for (t, c) in total.iter_mut().zip(cov) {
                    let src = c * layer.alpha;
                    *t = src + *t * (1.0 - src);
                }
            }
        }
    }
    let mut out = vec![T::default(); w * h];
    let (Some(total), false) = (total, area.xs().is_empty()) else {
        return out;
    };
    let width = area.x1 - area.x0;
    for (y, row) in area.ys().zip(total.chunks_exact(width)) {
        let line = &mut out[y * w + area.x0..y * w + area.x1];
        let inner = place.inner_cols(y);
        for ((x, &c), px) in area.xs().zip(row).zip(line) {
            // 0 stays 0; most of the area is empty.
            if c != 0.0 {
                let c = if inner.contains(&x) {
                    c
                } else {
                    c * place.in_box(x, y)
                };
                *px = pixel(level(c));
            }
        }
    }
    out
}

/// A coverage level as a texture pixel: white at that alpha.
fn mask(level: u8) -> Color32 {
    // token-ok: a coverage mask, tinted by the caller's colour when drawn
    Color32::from_white_alpha(level)
}

/// `(c * 255).round() as u8` for a coverage `c` in 0..=1 (NaN gives 0),
/// without the library call `round` is on a plain x86-64 target.
fn level(c: f32) -> u8 {
    let x = c * f32::from(u8::MAX);
    // `as` truncates (floor for x >= 0) and gives 0 for NaN; the fraction
    // is exact below 2^23.
    let floor = x as u32;
    let up = u32::from(x - floor as f32 >= 0.5);
    (floor + up).min(u32::from(u8::MAX)) as u8
}

/// One layer of an icon, mapped: its stroke as line pieces in pixels.
struct Placed {
    alpha: f32,
    pieces: Vec<(Pos2, Pos2)>,
    fill: Option<FillMesh>,
}

/// The pixels of a `w`×`h` grid an icon can cover, a rectangle: every
/// pixel outside it has coverage 0. Buffers hold this rectangle only.
#[derive(Clone, Copy)]
struct Area {
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
}

impl Area {
    /// The pixels within `reach` of a stroke piece, in a fill triangle or
    /// within [`HALF_PX`] of a fill contour of `layers`; the whole grid
    /// when a point is not finite.
    fn reached(layers: &[Placed], w: usize, h: usize, place: Place, reach: f32) -> Self {
        let whole = Self {
            x0: 0,
            x1: w,
            y0: 0,
            y1: h,
        };
        let (mut lo, mut hi) = (
            pos2(f32::INFINITY, f32::INFINITY),
            pos2(f32::NEG_INFINITY, f32::NEG_INFINITY),
        );
        let mut add = |p: Pos2, r: f32| {
            (lo, hi) = (lo.min(p - Vec2::splat(r)), hi.max(p + Vec2::splat(r)));
            p.is_finite()
        };
        for layer in layers {
            for &(a, b) in &layer.pieces {
                if !(add(a, reach) && add(b, reach)) {
                    return whole;
                }
            }
            let Some(fill) = &layer.fill else { continue };
            for &[x, y] in &fill.vertices {
                if !add(place.map(x, y), 0.0) {
                    return whole;
                }
            }
            for &([x, y], _) in fill.contours.iter().flatten() {
                if !add(place.map(x, y), HALF_PX) {
                    return whole;
                }
            }
        }
        let (xs, ys) = (span(lo.x, hi.x, 0.0, w), span(lo.y, hi.y, 0.0, h));
        Self {
            x0: xs.start.min(xs.end),
            x1: xs.end,
            y0: ys.start.min(ys.end),
            y1: ys.end,
        }
    }

    fn len(&self) -> usize {
        (self.x1 - self.x0) * (self.y1 - self.y0)
    }

    fn xs(&self) -> std::ops::Range<usize> {
        self.x0..self.x1
    }

    fn ys(&self) -> std::ops::Range<usize> {
        self.y0..self.y1
    }

    fn index(&self, x: usize, y: usize) -> usize {
        (y - self.y0) * (self.x1 - self.x0) + (x - self.x0)
    }

    /// [`span`] of the columns, clipped to the area.
    fn cols(&self, lo: f32, hi: f32, reach: f32) -> std::ops::Range<usize> {
        let s = span(lo, hi, reach, self.x1);
        s.start.max(self.x0)..s.end
    }

    /// [`span`] of the rows, clipped to the area.
    fn rows(&self, lo: f32, hi: f32, reach: f32) -> std::ops::Range<usize> {
        let s = span(lo, hi, reach, self.y1);
        s.start.max(self.y0)..s.end
    }
}

/// Coverage (0..=1) of one layer on `area`: its strokes `half` a stroke
/// wide on each side (none when `None`) and its fill, overlaps once.
fn layer_coverage(layer: &Placed, area: Area, place: Place, half: Option<f32>) -> Vec<f32> {
    let mut cov = match half {
        Some(half) => {
            // The distances become the coverage in place.
            let mut cov = vec![f32::INFINITY; area.len()];
            distances(&mut cov, area, &layer.pieces, half + HALF_PX);
            for c in &mut cov {
                *c = (HALF_PX + half - c.sqrt()).clamp(0.0, 1.0);
            }
            cov
        }
        None => vec![0.0; area.len()],
    };
    if let Some(fill) = &layer.fill {
        fill_coverage(&mut cov, area, fill, place);
    }
    cov
}

/// Raises `cov` to the fill's coverage where that is higher.
fn fill_coverage(cov: &mut [f32], area: Area, fill: &FillMesh, place: Place) {
    let map = |[x, y]: [f32; 2]| place.map(x, y);
    let mut inside = vec![false; area.len()];
    for &t in fill.indices.as_chunks::<3>().0 {
        let [a, b, c] = t.map(|i| {
            fill.vertices
                .get(usize::from(i))
                .copied()
                .map_or(pos2(f32::NAN, f32::NAN), map)
        });
        mark_triangle(&mut inside, area, [a, b, c]);
    }
    let mut pieces = Vec::new();
    for contour in &fill.contours {
        let pts: Vec<Pos2> = contour.iter().map(|&(p, _)| map(p)).collect();
        let Some(&last) = pts.last() else { continue };
        let mut prev = last;
        for &p in &pts {
            pieces.push((prev, p));
            prev = p;
        }
    }
    let mut dist = vec![f32::INFINITY; area.len()];
    distances(&mut dist, area, &pieces, HALF_PX);
    for ((c, d), &inn) in cov.iter_mut().zip(&dist).zip(&inside) {
        let d = d.sqrt();
        let f = if inn { HALF_PX + d } else { HALF_PX - d };
        *c = c.max(f.clamp(0.0, 1.0));
    }
}

/// Marks the pixels whose centre lies in the triangle (either orientation).
fn mark_triangle(inside: &mut [bool], area: Area, [a, b, c]: [Pos2; 3]) {
    let area2 = (b - a).x * (c - a).y - (c - a).x * (b - a).y;
    if area2.abs() <= f32::EPSILON || !area2.is_finite() {
        return;
    }
    let sign = area2.signum();
    let edges = [(a, b), (b, c), (c, a)].map(|(p, q)| (p, q - p, (q - p).length()));
    let (xs, ys) = (
        area.cols(a.x.min(b.x).min(c.x), a.x.max(b.x).max(c.x), 0.0),
        area.rows(a.y.min(b.y).min(c.y), a.y.max(b.y).max(c.y), 0.0),
    );
    for y in ys {
        for x in xs.clone() {
            let p = pixel_centre(x, y);
            let hit = edges.iter().all(|&(o, e, len)| {
                let cross = e.x * (p - o).y - e.y * (p - o).x;
                cross * sign >= -EDGE_SLACK_PX * len
            });
            if hit {
                inside[area.index(x, y)] = true;
            }
        }
    }
}

/// How far, in pixels, the regions [`distances`] visits reach past their
/// exact bounds. A pixel whose foot on a piece lies this far inside the
/// piece's end is nearer to the foot than to the end by 0.25² = 0.06 px²,
/// far more than
/// rounding moves a squared distance at texture coordinates up to a few
/// thousand pixels (about 0.01 px² at 1500 px and a 43 px reach), so the
/// piece a region skips is never the nearest by a rounding.
const REGION_SLACK_PX: f32 = 0.25; // behaviour-ok: rounding slack, not a design token

/// A half-plane `n · p + k >= 0`.
type HalfPlane = (Vec2, f32);

/// Lowers `dist` to the squared distance from each pixel centre within
/// `reach` of the line pieces `pieces` to the nearest piece.
///
/// The nearest point of a chain of pieces to a pixel is inside one piece,
/// or a point where two pieces meet, or an end. So each piece visits the
/// band of pixels beside it, and its start the wedge outside the bend
/// where it meets the piece before; an unchained start or end visits a
/// half disc. A dot, or a piece with a point that is not finite, visits
/// its whole box. Every visited pixel takes the distance to the whole
/// piece, so a pixel visited twice is harmless.
fn distances(dist: &mut [f32], area: Area, pieces: &[(Pos2, Pos2)], reach: f32) {
    let line = |&(a, b): &(Pos2, Pos2)| a.is_finite() && b.is_finite() && a != b;
    let half_plane = |n: Vec2, at: Pos2, k: f32| -> HalfPlane {
        (n, k - n.dot(at.to_vec2()) + REGION_SLACK_PX * n.length())
    };
    for (i, &(a, b)) in pieces.iter().enumerate() {
        if !line(&(a, b)) {
            let xs = area.cols(a.x.min(b.x), a.x.max(b.x), reach);
            for y in area.rows(a.y.min(b.y), a.y.max(b.y), reach) {
                for x in xs.clone() {
                    lower(dist, area, (a, b), x, y);
                }
            }
            continue;
        }
        let e = b - a;
        let n = e.rot90();
        let width = reach * e.length();
        let (top, bottom) = (a.y.min(b.y) - reach, a.y.max(b.y) + reach);
        // The band: 0 <= t <= 1 along the piece, within `reach` across it.
        let band = [
            half_plane(e, a, 0.0),
            half_plane(-e, b, 0.0),
            half_plane(n, a, width),
            half_plane(-n, a, width),
        ];
        visit(dist, area, [(a, b)], (top, bottom), band, None);
        // The start: t <= 0, and past the end of a piece ending here, whose
        // distance it takes too: where both pieces end, either may be the
        // nearer by a rounding.
        let disc = |c: Pos2| (c.y - reach, c.y + reach);
        let before = i.checked_sub(1).map(|j| pieces[j]);
        match before.filter(|p| line(p) && p.1 == a) {
            Some((pa, _)) => {
                let wedge = [half_plane(-e, a, 0.0), half_plane(a - pa, a, 0.0)];
                let both = [(pa, a), (a, b)];
                visit(dist, area, both, disc(a), wedge, Some((a, reach)));
            }
            None => {
                let start = [half_plane(-e, a, 0.0)];
                visit(dist, area, [(a, b)], disc(a), start, Some((a, reach)));
            }
        }
        // The end, unless the next piece starts here and covers it.
        let after = pieces.get(i + 1).filter(|p| line(p) && p.0 == b);
        if after.is_none() {
            let end = [half_plane(e, b, 0.0)];
            visit(dist, area, [(a, b)], disc(b), end, Some((b, reach)));
        }
    }
}

/// Lowers `dist` at the pixels of rows `top..=bottom` that lie in every
/// half-plane of `planes` and in the disc `disc` (centre, radius), to their
/// squared distance to each of `pieces`.
fn visit<const N: usize, const P: usize>(
    dist: &mut [f32],
    area: Area,
    pieces: [(Pos2, Pos2); N],
    (top, bottom): (f32, f32),
    planes: [HalfPlane; P],
    disc: Option<(Pos2, f32)>,
) {
    // n.x * x + rest >= 0 bounds x by -rest / n.x; the inverse once per
    // call. The slack covers its rounding.
    let planes = planes.map(|(n, k)| (n, k, 1.0 / n.x));
    let pieces = pieces.map(Piece::new);
    let width = area.x1 - area.x0;
    for y in area.rows(top, bottom, REGION_SLACK_PX) {
        let yc = pixel_centre(0, y).y;
        let (mut lo, mut hi) = (f32::NEG_INFINITY, f32::INFINITY);
        if let Some((c, r)) = disc {
            let r = r + REGION_SLACK_PX;
            let dy = yc - c.y;
            let s2 = r * r - dy * dy;
            // A row past the disc (or NaN) has no pixels in it.
            if s2.is_nan() || s2 < 0.0 {
                continue;
            }
            let s = s2.sqrt();
            (lo, hi) = (c.x - s, c.x + s);
        }
        for &(n, k, inv) in &planes {
            let rest = n.y * yc + k;
            if n.x > 0.0 {
                lo = lo.max(-rest * inv);
            } else if n.x < 0.0 {
                hi = hi.min(-rest * inv);
            } else if rest < 0.0 {
                hi = f32::NEG_INFINITY;
            }
        }
        if lo.is_nan() || hi.is_nan() || lo > hi {
            continue;
        }
        let xs = area.cols(lo, hi, REGION_SLACK_PX);
        let row = (y - area.y0) * width;
        let Some(line) = dist.get_mut(row + xs.start - area.x0..row + xs.end - area.x0) else {
            continue;
        };
        for piece in &pieces {
            for (x, slot) in xs.clone().zip(line.iter_mut()) {
                *slot = slot.min(piece.dist2(pixel_centre(x, y)));
            }
        }
    }
}

/// A line piece with what [`Piece::dist2`] needs of it.
#[derive(Clone, Copy)]
struct Piece {
    a: Pos2,
    b: Pos2,
    ab: Vec2,
    len2: f32,
}

impl Piece {
    fn new((a, b): (Pos2, Pos2)) -> Self {
        let ab = b - a;
        Self {
            a,
            b,
            ab,
            len2: ab.length_sq(),
        }
    }

    /// The squared distance from `p` to the piece (a point when its ends
    /// coincide).
    fn dist2(&self, p: Pos2) -> f32 {
        let Self { a, b, ab, len2 } = *self;
        let t = if len2 > 0.0 {
            ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        // An end is the end point itself, bit for bit, so every piece that
        // ends at a point gives the same distance to it.
        let q = if t <= 0.0 {
            a
        } else if t >= 1.0 {
            b
        } else {
            a + ab * t
        };
        (p - q).length_sq()
    }
}

/// Lowers `dist` at pixel `(x, y)` to its squared distance to the segment
/// `a`–`b` (a point when they coincide).
fn lower(dist: &mut [f32], area: Area, piece: (Pos2, Pos2), x: usize, y: usize) {
    let d = Piece::new(piece).dist2(pixel_centre(x, y));
    let slot = &mut dist[area.index(x, y)];
    *slot = slot.min(d);
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
        let painted = placed_coverage(icon, w, w, place, 2.0 * scale, |c| c);
        let exact = placed_coverage(icon, w, w, fine, 2.0 * scale, |c| c);
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

    /// [`distances`] visits only the band beside each piece and the caps
    /// and bends at its ends. It gives, bit for bit, the distances of
    /// visiting the whole box around every piece.
    #[test]
    fn distances_equal_those_of_the_whole_boxes() {
        use crate::icons::lucide;
        let mut cases = 0;
        let all = lucide::ALL.iter().map(|&i| (i, 24));
        let some = lucide::ALL.iter().step_by(40).map(|&i| (i, 96));
        let loader = std::iter::once((&lucide::LOADER_CIRCLE, 512));
        for (icon, side) in all.chain(some).chain(loader) {
            for angle in [0.0, 0.7, 2.9] {
                let scale = side as f32 / icon.size;
                let [m, _] = turned_margin(side, side);
                let (place, w) = if angle == 0.0 {
                    (Place::scaled(scale), side)
                } else {
                    (Place::turned(scale, side, side, angle), side + 2 * m)
                };
                let reach = scale + HALF_PX;
                let area = Area {
                    x0: 0,
                    x1: w,
                    y0: 0,
                    y1: w,
                };
                for layer in icon.layers() {
                    let pieces = stroke_segments(layer.segs, place);
                    let mut fast = vec![f32::INFINITY; w * w];
                    distances(&mut fast, area, &pieces, reach);
                    let mut boxes = vec![f32::INFINITY; w * w];
                    for &(a, b) in &pieces {
                        let xs = area.cols(a.x.min(b.x), a.x.max(b.x), reach);
                        for y in area.rows(a.y.min(b.y), a.y.max(b.y), reach) {
                            for x in xs.clone() {
                                lower(&mut boxes, area, (a, b), x, y);
                            }
                        }
                    }
                    // Beyond `reach` a distance only says "no coverage".
                    let near = |d: f32| if d.sqrt() < reach { d } else { f32::INFINITY };
                    let diff = fast
                        .iter()
                        .zip(&boxes)
                        .position(|(f, b)| near(*f).to_bits() != near(*b).to_bits());
                    if let Some(i) = diff {
                        let (x, y) = (i % w, i / w);
                        let p = pixel_centre(x, y);
                        let mut one = vec![f32::INFINITY; w * w];
                        let near_pieces: Vec<String> = pieces
                            .iter()
                            .enumerate()
                            .filter_map(|(j, &(a, b))| {
                                one[i] = f32::INFINITY;
                                lower(&mut one, area, (a, b), x, y);
                                (one[i].sqrt() < reach)
                                    .then(|| format!("#{j} {a:?}-{b:?}: {}", one[i]))
                            })
                            .collect();
                        panic!(
                            "{} at {side} px, {angle} rad: pixel ({x}, {y}) centre {p:?}: fast {} boxes {}; {} pieces; near: {near_pieces:#?}",
                            icon.name, fast[i], boxes[i], pieces.len()
                        );
                    }
                    cases += 1;
                }
            }
        }
        assert!(cases > 5000, "{cases} cases");
    }

    /// The exact coverage (0..=1) at device pixel `(x, y)` of `pieces`
    /// (device px from `origin`) stroked `stroke` device px wide, turned
    /// clockwise by `angle` about `centre`, all in device px: the crate's
    /// coverage model (a one-pixel ramp across the stroke's edge), applied to
    /// the turned geometry.
    fn turned_coverage(
        pieces: &[(Pos2, Pos2)],
        origin: Pos2,
        centre: Pos2,
        angle: f64,
        stroke: f64,
        x: usize,
        y: usize,
    ) -> f64 {
        let (s, c) = angle.sin_cos();
        let map = |p: Pos2| {
            let (px, py) = (
                f64::from(origin.x) + f64::from(p.x) - f64::from(centre.x),
                f64::from(origin.y) + f64::from(p.y) - f64::from(centre.y),
            );
            (
                f64::from(centre.x) + px * c - py * s,
                f64::from(centre.y) + px * s + py * c,
            )
        };
        let (qx, qy) = (x as f64 + 0.5, y as f64 + 0.5);
        let mut d = f64::INFINITY;
        for &(a, b) in pieces {
            let ((ax, ay), (bx, by)) = (map(a), map(b));
            let (ex, ey) = (bx - ax, by - ay);
            let len2 = ex * ex + ey * ey;
            let t = if len2 > 0.0 {
                (((qx - ax) * ex + (qy - ay) * ey) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            d = d.min(((qx - ax - ex * t).powi(2) + (qy - ay - ey * t).powi(2)).sqrt());
        }
        (0.5 + stroke / 2.0 - d).clamp(0.0, 1.0)
    }

    /// The worst difference, in levels of 255, between a black spinner of
    /// `side` on white, rendered at `ppp` with the clock at `time`, and the
    /// exact raster of the loader icon turned to that angle.
    fn turned_error(ppp: f32, side: f32, time: f64) -> (u32, f64) {
        let theme = crate::Theme::vega();
        let loader = theme.icons.loader;
        let stroke = theme.metrics.icon_stroke;
        let at = std::cell::Cell::new(egui::Rect::NOTHING);
        let mut h = egui_kittest::Harness::builder()
            .with_size(egui::vec2(80.0, 80.0))
            .with_pixels_per_point(ppp)
            .with_theme(egui::Theme::Light)
            .build_ui(|ui| {
                crate::Theme::vega().apply(ui.ctx());
                ui.ctx().all_styles_mut(|s| s.animation_time = 0.1);
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, Color32::WHITE);
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(
                    egui::Rect::from_min_size(pos2(20.3, 21.7), Vec2::splat(side)),
                ));
                at.set(
                    child
                        .add(
                            crate::components::spinner::Spinner::new()
                                .size(side)
                                .color(Color32::BLACK),
                        )
                        .rect,
                );
            });
        for _ in 0..3 {
            h.input_mut().time = Some(time);
            h.step();
        }
        let img = h.render().expect("wgpu render");
        let rect = at.get();
        let (x0, y0) = ((rect.min.x * ppp).round(), (rect.min.y * ppp).round());
        let w = (side * ppp).round();
        let centre = pos2(x0 + w / 2.0, y0 + w / 2.0);
        let angle = (time.fract()) * std::f64::consts::TAU;
        let scale = w / loader.size;
        // The crate's own pieces, so the comparison measures the turn, not
        // the flattening.
        let pieces: Vec<(Pos2, Pos2)> = loader
            .layers()
            .flat_map(|layer| {
                assert!(
                    layer.fill.is_none() && layer.alpha == 1.0,
                    "stroke-only icon"
                );
                stroke_segments(layer.segs, Place::scaled(scale))
            })
            .collect();
        let reach = (w * 0.75 + 2.0) as i64;
        let (mut worst, mut sum, mut n) = (0u32, 0.0, 0.0);
        for y in (centre.y as i64 - reach)..(centre.y as i64 + reach) {
            for x in (centre.x as i64 - reach)..(centre.x as i64 + reach) {
                let (x, y) = (x as usize, y as usize);
                let exact = turned_coverage(
                    &pieces,
                    pos2(x0, y0),
                    centre,
                    angle,
                    f64::from(stroke * scale),
                    x,
                    y,
                );
                let painted = f64::from(255 - img.get_pixel(x as u32, y as u32)[0]);
                let e = (painted - exact * 255.0).abs();
                worst = worst.max(e.round() as u32);
                sum += e;
                n += 1.0;
            }
        }
        (worst, sum / n)
    }

    /// `animate-spin` turns the SVG and the browser rasterises it at each
    /// angle: a turned spinner shows the loader rasterised at its angle, as
    /// sharp as the unturned one, not a turned bitmap.
    #[test]
    fn a_turned_spinner_is_the_icon_rasterised_at_its_angle() {
        let mut fails = Vec::new();
        for ppp in [1.0, 1.25, 2.0] {
            for side in [16.0, 24.0] {
                for time in [0.0, 0.03, 0.125, 0.3, 0.55, 0.81] {
                    let (worst, mean) = turned_error(ppp, side, time);
                    eprintln!("ppp {ppp} side {side} time {time}: worst {worst} mean {mean:.2}");
                    if worst > 1 {
                        fails.push(format!("ppp {ppp} side {side} time {time}: {worst}"));
                    }
                }
            }
        }
        assert!(fails.is_empty(), "{fails:#?}");
    }

    /// Where [`Place::inner_cols`] skips the turned box's ramp, the ramp
    /// is 1, bit for bit, at any size and angle.
    #[test]
    fn inner_columns_lie_wholly_in_the_box() {
        for side in [3usize, 16, 24, 97, 512, 1024] {
            for angle in [
                1e-4f32,
                0.3,
                std::f32::consts::FRAC_PI_4,
                std::f32::consts::FRAC_PI_2,
                2.9,
                std::f32::consts::PI,
                4.0,
                std::f32::consts::TAU - 0.003,
            ] {
                let (w, h) = (side, side + 3);
                let place = Place::turned(1.0, w, h, angle);
                let [mx, my] = turned_margin(w, h);
                let (tw, th) = (w + 2 * mx, h + 2 * my);
                let mut inner = 0;
                for y in 0..th {
                    let cols = place.inner_cols(y);
                    for x in cols.start..cols.end.min(tw) {
                        let b = place.in_box(x, y);
                        assert!(b == 1.0, "{side} px {angle} rad ({x}, {y}): {b}");
                        inner += 1;
                    }
                }
                // All but the ramp and the slack: about (w - 2) x (h - 2).
                assert!(
                    inner * 10 >= (w - 2) * (h - 2) * 8,
                    "{side} px {angle} rad: {inner}"
                );
            }
        }
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
