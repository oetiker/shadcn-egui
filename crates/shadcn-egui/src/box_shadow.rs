//! CSS outer `box-shadow` as an egui mesh, to the browser's pixels
//! (conformance 2.5).
//!
//! A layer is the border box moved by its offset and grown by its spread,
//! blurred by a Gaussian with a deviation of half the blur radius (CSS
//! Backgrounds 3). egui's own `Shadow` is a linear ramp as wide as the blur
//! radius, with no negative spread and one layer, so the crate builds the
//! mesh itself: rings of vertices along the shape's distance contours, each
//! vertex coloured with the blurred shape's exact coverage, and the whole
//! cut to the outside of the border box. The mesh is cached by shape, so
//! an idle box costs no allocation and a moving or fading one no rebuild.

use crate::color::mix_oklab;
use crate::components::shared::{clip_outside, rounded_polygon, Sorter};
use crate::theme::ShadowLayer;
use egui::{Color32, CornerRadius, Mesh, Pos2, Rect};
use std::f32::consts::{FRAC_PI_2, PI, SQRT_2};
use std::sync::Arc;

/// Arc segments per corner of a ring.
const CORNER_SEGMENTS: usize = 8;
/// The rings reach this many deviations into and out of the shape, where
/// the coverage is within 0.2 % of full and of none.
const REACH: f32 = 3.0;
/// Rings per deviation. Linear colour between rings a quarter deviation
/// apart stays within 0.2 % of the Gaussian edge, well under one level of
/// 255 for Tailwind's alphas.
const RINGS_PER_SIGMA: f32 = 4.0;
/// Points on each straight side next to each corner, a quarter deviation
/// apart: along a side the coverage changes within `REACH` deviations of
/// a corner and is flat between.
const SIDE_POINTS: usize = (REACH * RINGS_PER_SIGMA) as usize;
/// Samples across a corner band in the coverage integral.
const BAND_SAMPLES: usize = 16;

/// Most layers a theme shadow has: Tailwind's `--shadow-*` have two.
const MAX_LAYERS: usize = 2;

/// The layer CSS pads the shorter of two shadow lists with when it mixes
/// them: `0 0 0 0 transparent`.
const NO_LAYER: ShadowLayer =
    ShadowLayer::new(egui::Vec2::ZERO, 0.0, 0.0, Color32::TRANSPARENT, 0.0);

/// A `box-shadow` list held by value, so a transition can mix two of them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Layers {
    list: [ShadowLayer; MAX_LAYERS],
    len: usize,
}

impl Layers {
    /// No shadow: `box-shadow: none`.
    pub(crate) const NONE: Layers = Layers {
        list: [NO_LAYER; MAX_LAYERS],
        len: 0,
    };

    /// The theme's `layers`. Panics past [`MAX_LAYERS`], which no theme
    /// shadow reaches.
    pub(crate) fn new(layers: &[ShadowLayer]) -> Layers {
        assert!(
            layers.len() <= MAX_LAYERS,
            "a theme shadow has at most {MAX_LAYERS} layers"
        );
        let mut list = [NO_LAYER; MAX_LAYERS];
        list[..layers.len()].copy_from_slice(layers);
        Layers {
            list,
            len: layers.len(),
        }
    }

    pub(crate) fn as_slice(&self) -> &[ShadowLayer] {
        &self.list[..self.len]
    }

    /// CSS's mix of two shadow lists at `t`: layer by layer, the shorter
    /// list padded with `0 0 0 0 transparent`; lengths move linearly, the
    /// colour in OKLab with premultiplied alpha ([`mix_oklab`]).
    pub(crate) fn mix(a: Layers, b: Layers, t: f32) -> Layers {
        let len = a.len.max(b.len);
        let mut list = [NO_LAYER; MAX_LAYERS];
        for (i, l) in list.iter_mut().enumerate().take(len) {
            *l = mix_layer(a.list[i], b.list[i], t);
        }
        Layers { list, len }
    }
}

fn mix_layer(a: ShadowLayer, b: ShadowLayer, t: f32) -> ShadowLayer {
    let lerp = |p: f32, q: f32| p + (q - p) * t;
    let alpha = lerp(a.alpha(), b.alpha());
    // Premultiplied: each side's colour weighs by its alpha, so a padding
    // layer lends only its alpha.
    let w = if alpha > 0.0 {
        t * b.alpha() / alpha
    } else {
        t
    };
    let opaque = |c: Color32| Color32::from_rgb(c.r(), c.g(), c.b());
    ShadowLayer::new(
        a.offset() + (b.offset() - a.offset()) * t,
        lerp(a.blur(), b.blur()),
        lerp(a.spread(), b.spread()),
        mix_oklab(opaque(a.color()), opaque(b.color()), w),
        alpha,
    )
}

/// The shadow `layers` of the border box `rect` (already on the pixel grid
/// where egui rounds rects) with CSS corner radii `radii` (nw, ne, sw, se),
/// faded by `opacity` and cut to the outside of the box egui fills with
/// `cut`. First layer on top.
///
/// The mesh is built once per shape: its size, radii, cut, layers, scale
/// and where `rect` starts within a device pixel. A box that moves by
/// whole device pixels or fades reuses it, moved and faded per vertex, as
/// does one that comes back after frames without it.
pub(crate) fn shadow_mesh(
    ctx: &egui::Context,
    rect: Rect,
    radii: [f32; 4],
    cut: CornerRadius,
    layers: Layers,
    opacity: f32,
) -> Arc<Mesh> {
    let ppp = ctx.pixels_per_point();
    let (shape, local) = ShapeKey::new(rect, radii, cut, layers, ppp);
    let shift = rect.min - local.min;
    let placed = Placed {
        shape,
        shift: [shift.x, shift.y].map(f32::to_bits),
        opacity: opacity.to_bits(),
    };
    let cache = |f: &mut dyn FnMut(&mut ShadowCache)| {
        ctx.memory_mut(|m| f(m.caches.cache::<ShadowCache>()));
    };
    let (mut hit, mut built) = (None, None);
    cache(&mut |c| {
        hit = c.placed(placed);
        built = c.shape(shape);
    });
    if let Some(mesh) = hit {
        return mesh;
    }
    let built = built.unwrap_or_else(|| {
        let mesh = Arc::new(build(local, radii, cut, layers, 1.0, ppp));
        cache(&mut |c| {
            c.shapes.insert(shape, (mesh.clone(), c.pass));
            c.builds += 1;
        });
        mesh
    });
    let mut mesh = (*built).clone();
    mesh.translate(shift);
    // A fresh build rounds a vertex's faded alpha once, this a second
    // time: within one level per layer.
    if opacity != 1.0 {
        for v in &mut mesh.vertices {
            v.color = v.color.gamma_multiply(opacity);
        }
    }
    let mesh = Arc::new(mesh);
    cache(&mut |c| {
        c.current.insert(placed, mesh.clone());
    });
    mesh
}

/// Steps per device pixel that a shape's size and sub-pixel phase are
/// keyed in: finer than any visible change, coarse enough that float
/// noise in a moved rect does not miss the cache.
const STEPS_PER_PIXEL: f32 = 256.0;

/// Idle shapes kept beyond the ones the last two passes used, for a box
/// that comes back after frames without it.
const MAX_IDLE: usize = 8;

/// Passes an idle shape is kept at most.
const MAX_AGE: u64 = 60;

/// Everything a built shadow mesh depends on, except where it is and how
/// faded.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ShapeKey {
    /// The rect's size and the sub-pixel phase of its `min`, in steps of
    /// [`STEPS_PER_PIXEL`].
    size: [i64; 2],
    phase: [i64; 2],
    radii: [u32; 4],
    cut: CornerRadius,
    layers: [([u32; 5], Color32); MAX_LAYERS],
    len: usize,
    pixels_per_point: u32,
}

impl ShapeKey {
    /// The key of `rect`'s shadow, and the rect to build it at: `rect`'s
    /// size at its sub-pixel phase, next to the origin.
    fn new(
        rect: Rect,
        radii: [f32; 4],
        cut: CornerRadius,
        layers: Layers,
        ppp: f32,
    ) -> (ShapeKey, Rect) {
        let scale = ppp * STEPS_PER_PIXEL;
        let steps = |v: f32| (v * scale).round() as i64;
        let per_pixel = STEPS_PER_PIXEL as i64;
        let phase = [rect.min.x, rect.min.y].map(|v| steps(v).rem_euclid(per_pixel));
        let size = [steps(rect.width()), steps(rect.height())];
        let point = |v: i64| v as f32 / scale;
        let local = Rect::from_min_size(
            egui::pos2(point(phase[0]), point(phase[1])),
            egui::vec2(point(size[0]), point(size[1])),
        );
        let mut list = [([0; 5], Color32::TRANSPARENT); MAX_LAYERS];
        for (k, l) in list.iter_mut().zip(layers.as_slice()) {
            let v = [l.offset().x, l.offset().y, l.blur(), l.spread(), l.alpha()];
            *k = (v.map(f32::to_bits), l.color());
        }
        let key = ShapeKey {
            size,
            phase,
            radii: radii.map(f32::to_bits),
            cut,
            layers: list,
            len: layers.len,
            pixels_per_point: ppp.to_bits(),
        };
        (key, local)
    }
}

/// A built shape moved by `shift` and faded by `opacity`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Placed {
    shape: ShapeKey,
    shift: [u32; 2],
    opacity: u32,
}

/// Shadow meshes, thousands of vertices with a coverage integral each.
/// The built shapes stay while the last two passes use them, and up to
/// [`MAX_IDLE`] idle ones for [`MAX_AGE`] passes. The placed meshes stay
/// for this pass and the last, as egui's `FrameCache` keeps them, so an
/// idle box costs no allocation and an animated one cannot grow the
/// cache. egui ends each pass of the cache (`CacheTrait::update`), so it
/// empties when an app stops drawing shadows.
#[derive(Clone, Default)]
struct ShadowCache {
    /// Passes ended so far: the number of this one.
    pass: u64,
    /// Each shape with the last pass that used it.
    shapes: std::collections::HashMap<ShapeKey, (Arc<Mesh>, u64)>,
    current: std::collections::HashMap<Placed, Arc<Mesh>>,
    previous: std::collections::HashMap<Placed, Arc<Mesh>>,
    /// Shapes built, for the tests.
    builds: u64,
}

impl egui::cache::CacheTrait for ShadowCache {
    /// The end of a pass: this pass's placed meshes become the previous
    /// ones, and idle shapes past the bounds go.
    fn update(&mut self) {
        self.pass += 1;
        let pass = self.pass;
        self.previous = std::mem::take(&mut self.current);
        let idle = |used: u64| used + 1 < pass;
        let mut ages: Vec<u64> = self
            .shapes
            .values()
            .map(|(_, used)| *used)
            .filter(|used| idle(*used))
            .collect();
        ages.sort_unstable_by(|a, b| b.cmp(a));
        // The oldest pass an idle shape may come from; ties at the
        // bound may keep a few more than `MAX_IDLE`.
        let oldest = ages
            .get(MAX_IDLE.saturating_sub(1))
            .map_or(0, |used| *used)
            .max(pass.saturating_sub(MAX_AGE));
        self.shapes
            .retain(|_, (_, used)| !idle(*used) || *used >= oldest);
    }

    fn len(&self) -> usize {
        self.shapes.len() + self.current.len() + self.previous.len()
    }
}

impl ShadowCache {
    /// The placed mesh `key` from this pass or the last.
    fn placed(&mut self, key: Placed) -> Option<Arc<Mesh>> {
        let hit = self
            .current
            .get(&key)
            .cloned()
            .or_else(|| self.previous.remove(&key));
        if let Some(mesh) = &hit {
            self.current.insert(key, mesh.clone());
        }
        hit
    }

    /// The built shape `key`, marked as used in this pass.
    fn shape(&mut self, key: ShapeKey) -> Option<Arc<Mesh>> {
        let pass = self.pass;
        self.shapes.get_mut(&key).map(|(mesh, used)| {
            *used = pass;
            mesh.clone()
        })
    }
}

/// The shadow mesh of `layers` around `rect`, built from scratch.
fn build(
    rect: Rect,
    radii: [f32; 4],
    cut: CornerRadius,
    layers: Layers,
    opacity: f32,
    pixels_per_point: f32,
) -> Mesh {
    let cut = rounded_polygon(rect, cut);
    let hidden = Sorter::new(&cut);
    let mut mesh = Mesh::default();
    for layer in layers.as_slice().iter().rev() {
        add_layer(
            &mut mesh,
            rect,
            radii,
            layer,
            opacity,
            pixels_per_point,
            Some(&hidden),
        );
    }
    clip_outside(&mesh, &cut)
}

#[cfg(test)]
fn shadow_builds(ctx: &egui::Context) -> u64 {
    ctx.memory_mut(|m| m.caches.cache::<ShadowCache>().builds)
}

/// Meshes the cache holds: shapes and placed ones.
#[cfg(test)]
fn shadow_cached(ctx: &egui::Context) -> usize {
    use egui::cache::CacheTrait as _;
    ctx.memory_mut(|m| m.caches.cache::<ShadowCache>().len())
}

/// CSS's corner clamping: where two radii on one side add up to more than
/// its length, all four scale down by the same factor.
pub(crate) fn clamp_radii([nw, ne, sw, se]: [f32; 4], rect: Rect) -> [f32; 4] {
    let (w, h) = (rect.width().max(0.0), rect.height().max(0.0));
    let f = [(w, nw + ne), (w, sw + se), (h, nw + sw), (h, ne + se)]
        .into_iter()
        .filter(|(_, sum)| *sum > 0.0)
        .map(|(len, sum)| len / sum)
        .fold(1.0_f32, f32::min);
    [nw, ne, sw, se].map(|r| (r * f).max(0.0))
}

/// A shadow corner's radius: the box's radius plus the spread, where a
/// radius under a positive spread grows less (CSS Backgrounds 3, "spread
/// distance"), floored at 0.
fn spread_radius(r: f32, spread: f32) -> f32 {
    if spread > 0.0 && r < spread {
        r + spread * (1.0 + (r / spread - 1.0).powi(3))
    } else {
        (r + spread).max(0.0)
    }
}

/// `rect` grown by `d` on every side; a side that would cross the other
/// stops at the centre.
fn grow(rect: Rect, d: f32) -> Rect {
    let c = rect.center();
    let half = (rect.size() / 2.0 + egui::Vec2::splat(d)).max(egui::Vec2::ZERO);
    Rect::from_center_size(c, half * 2.0)
}

fn add_layer(
    mesh: &mut Mesh,
    rect: Rect,
    radii: [f32; 4],
    layer: &ShadowLayer,
    opacity: f32,
    pixels_per_point: f32,
    hidden: Option<&Sorter>,
) {
    let spread = layer.spread();
    let shape = grow(rect.translate(layer.offset()), spread);
    let shape_radii = clamp_radii(radii.map(|r| spread_radius(r, spread)), shape);
    let sigma = layer.blur() / 2.0;
    let [r, g, b, _] = layer.color().to_srgba_unmultiplied();
    let alpha = layer.alpha() * opacity;
    // Ring offsets from the shape's outline and their coverage. A blur of
    // 0 is a hard edge, feathered over one physical pixel as egui does.
    let rings: Vec<(f32, Option<f32>)> = if sigma > 0.0 {
        let n = (2.0 * REACH * RINGS_PER_SIGMA) as usize;
        (0..=n)
            .map(|k| (sigma * (k as f32 / RINGS_PER_SIGMA - REACH), None))
            .collect()
    } else {
        let h = 0.5 / pixels_per_point;
        vec![(-h, Some(1.0)), (h, Some(0.0))]
    };
    let per_ring = 4 * (CORNER_SEGMENTS + 1 + 2 * SIDE_POINTS);
    let ring_at = |d: f32| {
        let ring = grow(shape, d);
        (
            ring,
            clamp_radii(shape_radii.map(|r| (r + d).max(0.0)), ring),
        )
    };
    let step = sigma / RINGS_PER_SIGMA;
    // The inner rings that lie wholly inside the cut: every triangle
    // between two of them, and the fan, would be cut away whole, so they
    // are not built. The last of them stays, the inner side of the first
    // strip kept, which the cut splits as before.
    let inside = hidden.map_or(0, |hidden| {
        rings
            .iter()
            .take_while(|&&(d, _)| {
                let (ring, ring_radii) = ring_at(d);
                let mut all = true;
                contour(ring, ring_radii, step, |p| all &= hidden.inside(p));
                all
            })
            .count()
    });
    let rings = &rings[inside.saturating_sub(1)..];
    let first = mesh.vertices.len() as u32;
    for &(d, fixed) in rings {
        let (ring, ring_radii) = ring_at(d);
        contour(ring, ring_radii, step, |p| {
            let cov = fixed.unwrap_or_else(|| coverage(p, shape, shape_radii, sigma));
            let a = (alpha * cov).clamp(0.0, 1.0) * 255.0;
            mesh.colored_vertex(p, Color32::from_rgba_unmultiplied(r, g, b, a.round() as u8));
        });
    }
    // The innermost ring as a fan, then a strip between each pair of rings.
    let n = per_ring as u32;
    if inside == 0 {
        for i in 1..n - 1 {
            mesh.add_triangle(first, first + i, first + i + 1);
        }
    }
    for r in 0..rings.len() as u32 - 1 {
        let (a, b) = (first + r * n, first + (r + 1) * n);
        for i in 0..n {
            let j = (i + 1) % n;
            mesh.add_triangle(a + i, b + i, b + j);
            mesh.add_triangle(a + i, b + j, a + j);
        }
    }
}

/// The outline of a rounded rect from the nw corner clockwise:
/// `CORNER_SEGMENTS + 1` points per corner and `SIDE_POINTS` on each side
/// next to each corner, `step` apart (closer on a short side), so rings of
/// any size pair up point by point.
fn contour(rect: Rect, [nw, ne, sw, se]: [f32; 4], step: f32, mut point: impl FnMut(Pos2)) {
    let corners = [
        (egui::pos2(rect.min.x + nw, rect.min.y + nw), nw, PI),
        (egui::pos2(rect.max.x - ne, rect.min.y + ne), ne, 1.5 * PI),
        (egui::pos2(rect.max.x - se, rect.max.y - se), se, 0.0),
        (egui::pos2(rect.min.x + sw, rect.max.y - sw), sw, FRAC_PI_2),
    ];
    let arc = |(c, r, start): (Pos2, f32, f32), s: usize| {
        let a = start + FRAC_PI_2 * s as f32 / CORNER_SEGMENTS as f32;
        c + r * egui::vec2(a.cos(), a.sin())
    };
    for k in 0..4 {
        for s in 0..=CORNER_SEGMENTS {
            point(arc(corners[k], s));
        }
        let (a, b) = (
            arc(corners[k], CORNER_SEGMENTS),
            arc(corners[(k + 1) % 4], 0),
        );
        let len = (b - a).length();
        let dir = if len > 0.0 {
            (b - a) / len
        } else {
            egui::Vec2::ZERO
        };
        let at = |j: usize| (j as f32 * step).min(len / 2.0);
        for j in 1..=SIDE_POINTS {
            point(a + dir * at(j));
        }
        for j in (1..=SIDE_POINTS).rev() {
            point(b - dir * at(j));
        }
    }
}

/// The share of a Gaussian of deviation `sigma` around `p` that falls in
/// the rounded rect: exact along x (the error function), integrated along
/// y, numerically only across the corner bands.
fn coverage(p: Pos2, rect: Rect, [nw, ne, sw, se]: [f32; 4], sigma: f32) -> f32 {
    let cdf = |t: f32| 0.5 * (1.0 + erf(t / (sigma * SQRT_2)));
    // The shape's span at height y, as a share of the Gaussian along x.
    let row = |y: f32| {
        let inset = |top: f32, bottom: f32| {
            let (r, dy) = if y < rect.min.y + top {
                (top, rect.min.y + top - y)
            } else if y > rect.max.y - bottom {
                (bottom, y - (rect.max.y - bottom))
            } else {
                return 0.0;
            };
            r - (r * r - dy * dy).max(0.0).sqrt()
        };
        let (x0, x1) = (rect.min.x + inset(nw, sw), rect.max.x - inset(ne, se));
        (cdf(x1 - p.x) - cdf(x0 - p.x)).max(0.0)
    };
    let top_band = (rect.min.y + nw.max(ne)).min(rect.max.y);
    let bottom_band = (rect.max.y - sw.max(se)).max(top_band);
    // Straight sides: the span is constant, the y integral exact.
    let mut sum = row(rect.center().y.clamp(top_band, bottom_band))
        * (cdf(bottom_band - p.y) - cdf(top_band - p.y)).max(0.0);
    let norm = 1.0 / (sigma * (2.0 * PI).sqrt());
    let window = 4.0 * sigma;
    for (a, b) in [(rect.min.y, top_band), (bottom_band, rect.max.y)] {
        let (a, b) = (a.max(p.y - window), b.min(p.y + window));
        if b <= a {
            continue;
        }
        let dy = (b - a) / BAND_SAMPLES as f32;
        for i in 0..BAND_SAMPLES {
            let y = a + (i as f32 + 0.5) * dy;
            let t = (y - p.y) / sigma;
            sum += row(y) * norm * (-0.5 * t * t).exp() * dy;
        }
    }
    sum.clamp(0.0, 1.0)
}

/// The error function, Abramowitz and Stegun 7.1.26 (error below 1.5e-7).
fn erf(x: f32) -> f32 {
    let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
    let poly = t
        * (0.254_829_6
            + t * (-0.284_496_74 + t * (1.421_413_8 + t * (-1.453_152 + t * 1.061_405_4))));
    (1.0 - poly * (-x * x).exp()).copysign(x)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::components::shared::split_convex_with;

    #[test]
    fn erf_matches_known_values() {
        for (x, want) in [
            (0.0, 0.0),
            (0.5, 0.520_499_9),
            (1.0, 0.842_700_8),
            (2.0, 0.995_322_3),
        ] {
            assert!((erf(x) - want).abs() < 1e-6, "erf({x})");
            assert!((erf(-x) + want).abs() < 1e-6, "erf(-{x})");
        }
    }

    #[test]
    fn coverage_of_a_wide_box_is_the_gaussian_edge() {
        // Far from the corners the blurred edge is the normal CDF.
        let rect = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1000.0, 100.0));
        let r = [8.0; 4];
        let at = |y: f32| coverage(egui::pos2(500.0, y), rect, r, 10.0);
        assert!((at(100.0) - 0.5).abs() < 1e-3);
        assert!((at(110.0) - 0.158_655).abs() < 1e-3);
        assert!((at(50.0) - 1.0).abs() < 1e-3);
        // A corner gives less than either side.
        let corner = coverage(egui::pos2(1000.0, 100.0), rect, r, 10.0);
        assert!(corner < 0.25 && corner > 0.1, "{corner}");
    }

    /// Tailwind's `shadow-sm`: two layers, one with a negative spread.
    fn shadow_sm() -> Layers {
        let black = Color32::BLACK;
        Layers::new(&[
            ShadowLayer::new(egui::vec2(0.0, 1.0), 3.0, 0.0, black, 0.1),
            ShadowLayer::new(egui::vec2(0.0, 1.0), 2.0, -1.0, black, 0.1),
        ])
    }

    /// The shadow of an 80 x 30 box at `at`, faded by `opacity`, drawn in
    /// one frame of `ctx` (none where `at` is `None`).
    fn frame(ctx: &egui::Context, at: Option<Pos2>, opacity: f32) -> Option<Arc<Mesh>> {
        let mut mesh = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |_| {
            mesh = at.map(|at| {
                let rect = Rect::from_min_size(at, egui::vec2(80.0, 30.0));
                shadow_mesh(
                    ctx,
                    rect,
                    [6.0; 4],
                    CornerRadius::same(6),
                    shadow_sm(),
                    opacity,
                )
            });
        });
        mesh
    }

    /// Moving by whole device pixels, fading and missing a frame reuse the
    /// built mesh; only a new sub-pixel phase builds another (#25).
    #[test]
    fn a_moved_faded_or_returning_shadow_is_not_rebuilt() {
        let ctx = egui::Context::default();
        let builds = |ctx: &egui::Context| shadow_builds(ctx);
        let start = builds(&ctx);
        frame(&ctx, Some(egui::pos2(10.0, 10.0)), 1.0);
        assert_eq!(builds(&ctx) - start, 1, "first frame");
        for x in 11..15 {
            frame(&ctx, Some(egui::pos2(x as f32, 2.0 * x as f32)), 1.0);
        }
        assert_eq!(builds(&ctx) - start, 1, "moved by whole pixels");
        for o in [0.9, 0.5, 0.1] {
            frame(&ctx, Some(egui::pos2(14.0, 28.0)), o);
        }
        assert_eq!(builds(&ctx) - start, 1, "faded");
        frame(&ctx, None, 1.0);
        frame(&ctx, Some(egui::pos2(14.0, 28.0)), 1.0);
        assert_eq!(builds(&ctx) - start, 1, "absent for a frame");
        frame(&ctx, Some(egui::pos2(14.5, 28.0)), 1.0);
        assert_eq!(builds(&ctx) - start, 2, "a new sub-pixel phase");
    }

    /// `mesh` painted on a transparent `w` x `h` canvas at `origin`, one
    /// premultiplied RGBA sample per pixel (off the pixel centre, so no
    /// sample lies on a grid-aligned edge), as a GPU blends it.
    fn raster(mesh: &Mesh, origin: Pos2, w: usize, h: usize) -> Vec<[f32; 4]> {
        let mut px = vec![[0.0_f32; 4]; w * h];
        for t in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
            let area =
                (b.pos - a.pos).x * (c.pos - a.pos).y - (b.pos - a.pos).y * (c.pos - a.pos).x;
            if area.abs() < 1e-12 {
                continue;
            }
            let bounds = Rect::from_points(&[a.pos, b.pos, c.pos]);
            let (x0, y0) = (
                (bounds.min.x - origin.x).floor().max(0.0) as usize,
                (bounds.min.y - origin.y).floor().max(0.0) as usize,
            );
            let (x1, y1) = (
                ((bounds.max.x - origin.x).ceil().max(0.0) as usize).min(w),
                ((bounds.max.y - origin.y).ceil().max(0.0) as usize).min(h),
            );
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = origin + egui::vec2(x as f32 + 0.537, y as f32 + 0.291);
                    let edge =
                        |u: Pos2, v: Pos2| ((v - u).x * (p - u).y - (v - u).y * (p - u).x) / area;
                    let (wa, wb, wc) = (edge(b.pos, c.pos), edge(c.pos, a.pos), edge(a.pos, b.pos));
                    if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                        continue;
                    }
                    let src: [f32; 4] = std::array::from_fn(|k| {
                        wa * f32::from(a.color[k])
                            + wb * f32::from(b.color[k])
                            + wc * f32::from(c.color[k])
                    });
                    let dst = &mut px[y * w + x];
                    let keep = 1.0 - src[3] / 255.0;
                    *dst = std::array::from_fn(|k| src[k] + dst[k] * keep);
                }
            }
        }
        px
    }

    /// The reused mesh, moved and faded, paints what a fresh build at that
    /// place and opacity paints. The cut and the triangles may differ in
    /// float noise; a pixel may differ by one level of rounding per
    /// layer, where the fade rounds a vertex's alpha a second time.
    #[test]
    fn a_moved_or_faded_shadow_matches_a_fresh_build() {
        let ctx = egui::Context::default();
        frame(&ctx, Some(egui::pos2(3.0, 4.0)), 1.0);
        for (at, opacity) in [
            (egui::pos2(103.0, 57.0), 1.0),
            (egui::pos2(-20.0, 999.0), 0.5),
            (egui::pos2(40.0, 40.0), 0.13),
        ] {
            let got = frame(&ctx, Some(at), opacity).expect("drawn");
            assert_eq!(shadow_builds(&ctx), 1, "reused");
            let rect = Rect::from_min_size(at, egui::vec2(80.0, 30.0));
            let want = build(
                rect,
                [6.0; 4],
                CornerRadius::same(6),
                shadow_sm(),
                opacity,
                1.0,
            );
            let origin = at - egui::vec2(10.0, 10.0);
            let (g, w) = (
                raster(&got, origin, 100, 55),
                raster(&want, origin, 100, 55),
            );
            let worst = g
                .iter()
                .zip(&w)
                .flat_map(|(g, w)| (0..4).map(move |k| (g[k] - w[k]).abs()))
                .fold(0.0, f32::max);
            assert!(worst <= 2.0, "{at:?} {opacity}: {worst}");
            let painted = w.iter().filter(|p| p[3] > 0.0).count();
            assert!(painted > 300, "{at:?}: the shadow shows ({painted} px)");
        }
    }

    /// A box that changes size and opacity every frame keeps the cache
    /// bounded.
    #[test]
    fn the_shadow_cache_stays_bounded_while_a_box_resizes() {
        let ctx = egui::Context::default();
        for f in 0..40 {
            let _ = ctx.run_ui(egui::RawInput::default(), |_| {
                let rect =
                    Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(40.0 + f as f32, 30.0));
                let opacity = (f % 10) as f32 / 10.0 + 0.05;
                let _ = shadow_mesh(
                    &ctx,
                    rect,
                    [6.0; 4],
                    CornerRadius::same(6),
                    shadow_sm(),
                    opacity,
                );
            });
        }
        let len = shadow_cached(&ctx);
        assert!(len <= MAX_IDLE + 4, "cached meshes {len}");
    }

    /// An app that stops drawing shadows lets go of the cached ones: they
    /// go once idle for [`MAX_AGE`] passes, without another shadow drawn.
    #[test]
    fn the_shadow_cache_empties_when_no_shadow_is_drawn() {
        let ctx = egui::Context::default();
        for x in 0..3 {
            frame(&ctx, Some(egui::pos2(10.0 + x as f32 * 0.25, 10.0)), 1.0);
        }
        assert!(shadow_cached(&ctx) >= 3);
        for _ in 0..2 {
            frame(&ctx, None, 1.0);
        }
        assert!(
            shadow_cached(&ctx) >= 3,
            "a box away for a frame comes back"
        );
        for _ in 0..MAX_AGE {
            frame(&ctx, None, 1.0);
        }
        assert_eq!(shadow_cached(&ctx), 0);
    }

    /// The same mesh, vertex for vertex and index for index.
    pub(crate) fn assert_same_mesh(a: &egui::Mesh, b: &egui::Mesh, what: &str) {
        assert_eq!(a.indices, b.indices, "{what} indices");
        assert_eq!(a.vertices.len(), b.vertices.len(), "{what} vertices");
        for (u, v) in a.vertices.iter().zip(&b.vertices) {
            assert_eq!(u.pos.x.to_bits(), v.pos.x.to_bits(), "{what} {u:?} {v:?}");
            assert_eq!(u.pos.y.to_bits(), v.pos.y.to_bits(), "{what} {u:?} {v:?}");
            assert_eq!(u.uv, v.uv, "{what}");
            assert_eq!(u.color, v.color, "{what}");
        }
    }

    /// Both halves of a split as the per-edge test alone splits them.
    pub(crate) fn assert_same_split(mesh: &egui::Mesh, polygon: &[egui::Pos2]) {
        let fast = split_convex_with(mesh, polygon, true);
        let reference = split_convex_with(mesh, polygon, false);
        assert_same_mesh(&fast.0, &reference.0, "inside");
        assert_same_mesh(&fast.1, &reference.1, "outside");
    }

    /// Sorting triangles before the per-edge test cuts a shadow exactly as
    /// the per-edge test alone does, and leaving out the rings the cut
    /// hides builds the same shadow: uniform, uneven and cut radii, whole
    /// and fractional places, one and two layers, a hard edge (#25).
    #[test]
    fn the_quick_split_cuts_a_shadow_as_the_edge_test_does() {
        let black = Color32::BLACK;
        let md = Layers::new(&[
            ShadowLayer::new(egui::vec2(0.0, 4.0), 6.0, -1.0, black, 0.1),
            ShadowLayer::new(egui::vec2(0.0, 2.0), 4.0, -2.0, black, 0.1),
        ]);
        let hard = Layers::new(&[ShadowLayer::new(egui::vec2(2.0, 2.0), 0.0, 1.0, black, 0.5)]);
        let cases: [([f32; 4], CornerRadius, Pos2, Layers, f32); 6] = [
            (
                [6.0; 4],
                CornerRadius::same(6),
                egui::pos2(10.0, 10.0),
                shadow_sm(),
                1.0,
            ),
            (
                [6.0; 4],
                CornerRadius::same(6),
                egui::pos2(10.3, 7.7),
                md,
                1.0,
            ),
            (
                [0.0, 12.0, 3.0, 20.0],
                CornerRadius {
                    nw: 0,
                    ne: 12,
                    sw: 3,
                    se: 20,
                },
                egui::pos2(-5.5, 3.25),
                md,
                2.0,
            ),
            (
                [8.0; 4],
                CornerRadius::ZERO,
                egui::pos2(100.0, 50.0),
                shadow_sm(),
                1.5,
            ),
            (
                [0.0; 4],
                CornerRadius::same(10),
                egui::pos2(1.0, 1.0),
                md,
                1.0,
            ),
            (
                [4.0; 4],
                CornerRadius::same(4),
                egui::pos2(0.25, 0.75),
                hard,
                1.0,
            ),
        ];
        for (k, (radii, cut, at, layers, ppp)) in cases.into_iter().enumerate() {
            let rect = Rect::from_min_size(at, egui::vec2(90.0, 29.0));
            let polygon = rounded_polygon(rect, cut);
            let (mut all, mut kept) = (Mesh::default(), Mesh::default());
            let hidden = Sorter::new(&polygon);
            for layer in layers.as_slice().iter().rev() {
                add_layer(&mut all, rect, radii, layer, 1.0, ppp, None);
                add_layer(&mut kept, rect, radii, layer, 1.0, ppp, Some(&hidden));
            }
            assert_same_split(&all, &polygon);
            let reference = split_convex_with(&all, &polygon, false).1;
            assert_same_mesh(
                &build(rect, radii, cut, layers, 1.0, ppp),
                &reference,
                "shadow",
            );
            if k == 0 {
                let (kept, all) = (kept.vertices.len(), all.vertices.len());
                assert!(
                    kept < all,
                    "rings hidden by the cut are built: {kept} of {all}"
                );
            }
        }
    }

    #[test]
    fn spread_follows_css() {
        assert_eq!(spread_radius(16.0, -12.0), 4.0);
        assert_eq!(spread_radius(4.0, -12.0), 0.0);
        assert_eq!(spread_radius(0.0, 4.0), 0.0);
        assert_eq!(spread_radius(8.0, 4.0), 12.0);
    }
}
