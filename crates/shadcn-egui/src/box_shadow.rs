//! CSS outer `box-shadow` as a nine-slice texture, to the browser's pixels
//! (conformance 2.5).
//!
//! A layer is the border box moved by its offset and grown by its spread,
//! blurred by a Gaussian with a deviation of half the blur radius (CSS
//! Backgrounds 3). egui's own `Shadow` is a linear ramp as wide as the blur
//! radius, with no negative spread and one layer, so the crate computes the
//! blurred shapes itself: one texel per device pixel, all layers composited
//! as CSS paints them, times the share of the pixel outside the box egui
//! fills, so the shadow never shows through the box. Along a box's straight
//! sides the shadow does not change, so the texture holds the corners and
//! one texel of each side, stretched over the rest (nine-slice): a box of
//! any size shares it with every box of the same radii and shadow, a moved
//! box only moves its quads, and a fade is the quads' vertex colour.

use crate::color::mix_oklab;
use crate::theme::ShadowLayer;
use egui::{Color32, CornerRadius, Mesh, Pos2, Rect, Shape, TextureHandle, TextureOptions};
use std::f64::consts::{FRAC_PI_2, PI, SQRT_2};

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

/// The shadow `layers` of the border box `rect` with CSS corner radii
/// `radii` (nw, ne, sw, se), faded by `opacity` and cut to the outside of
/// the box egui fills with `cut`. First layer on top.
///
/// The box is placed on the device pixel grid, as egui places the fill it
/// rounds there; the texture is built once per shadow and reused by every
/// box with the same radii, cut, layers and scale, at any place, any
/// opacity and any size its sides can stretch to.
pub(crate) fn shadow_shape(
    ctx: &egui::Context,
    rect: Rect,
    radii: [f32; 4],
    cut: CornerRadius,
    layers: Layers,
    opacity: f32,
) -> Shape {
    let ppp = ctx.pixels_per_point();
    if !drawable(rect, ppp) {
        return Shape::Noop;
    }
    let max_side = ctx.input(|i| i.max_texture_side).max(1);
    let device = Device::new(rect.size(), radii, cut, layers, ppp);
    let Some(plan) = Plan::new(&device, max_side) else {
        return Shape::Noop;
    };
    let key = Key::new(&plan, radii, cut, layers, ppp);
    let cache = |f: &mut dyn FnMut(&mut ShadowCache)| {
        ctx.memory_mut(|m| f(m.caches.cache::<ShadowCache>()));
    };
    let mut ids = None;
    cache(&mut |c| ids = c.get(&key));
    let ids = ids.unwrap_or_else(|| {
        let images = plan.images(&device);
        let bytes = images.iter().map(|i| i.pixels.len() * 4).sum();
        let mut spare = None;
        cache(&mut |c| spare = c.spare(images.len()));
        let tiles: Vec<TextureHandle> = match spare {
            Some(mut tiles) => {
                for (tile, image) in tiles.iter_mut().zip(images) {
                    tile.set(image, OPTIONS);
                }
                tiles
            }
            None => images
                .into_iter()
                .map(|image| ctx.load_texture("shadcn-egui box-shadow", image, OPTIONS))
                .collect(),
        };
        let ids: Vec<egui::TextureId> = tiles.iter().map(TextureHandle::id).collect();
        let mut tiles = Some(tiles);
        cache(&mut |c| c.insert(key, tiles.take().unwrap_or_default(), bytes));
        ids
    });
    let origin = [rect.min.x, rect.min.y].map(|v| f64::from((v * ppp).round()));
    plan.shape(&ids, origin, ppp, opacity)
}

/// Largest box side in device pixels that gets a shadow. Beyond any
/// screen; below it, device pixels stay exact integers in `f64` and `i64`.
const MAX_DEVICE_SIDE: f32 = 1e12;

/// Whether the box `rect` at `ppp` gets a shadow: a box with a
/// non-finite or absurd place or size (an item filling unbounded room)
/// gets none.
fn drawable(rect: Rect, ppp: f32) -> bool {
    let device = [rect.min.x, rect.min.y, rect.max.x, rect.max.y].map(|v| v * ppp);
    ppp.is_finite()
        && device
            .iter()
            .all(|v| v.is_finite() && v.abs() <= MAX_DEVICE_SIDE)
}

/// Texels map 1:1 to device pixels, so the nearest texel is the exact one;
/// a stretched side samples its one texel's centre.
const OPTIONS: TextureOptions = TextureOptions::NEAREST;

/// Idle shadows kept beyond the ones the last two passes used, for a box
/// that comes back after frames without it.
const MAX_IDLE: usize = 8;

/// Passes an idle shadow is kept at most.
const MAX_AGE: u64 = 60;

/// Texture bytes kept at most: idle shadows go, oldest first, past it.
/// The shadows of a page at 3x take a few MiB; a `shadow-2xl` box alone
/// 7 MiB.
const MAX_BYTES: usize = 16 << 20;

/// Everything a shadow texture depends on: the box's size only along an
/// axis its sides cannot stretch.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    size: [Option<i64>; 2],
    radii: [u32; 4],
    cut: CornerRadius,
    layers: [([u32; 5], Color32); MAX_LAYERS],
    len: usize,
    pixels_per_point: u32,
    max_side: usize,
}

impl Key {
    fn new(plan: &Plan, radii: [f32; 4], cut: CornerRadius, layers: Layers, ppp: f32) -> Key {
        let mut list = [([0; 5], Color32::TRANSPARENT); MAX_LAYERS];
        for (k, l) in list.iter_mut().zip(layers.as_slice()) {
            let v = [l.offset().x, l.offset().y, l.blur(), l.spread(), l.alpha()];
            *k = (v.map(f32::to_bits), l.color());
        }
        Key {
            size: [0, 1].map(|a| (!plan.axes[a].stretch).then_some(plan.size[a])),
            radii: radii.map(f32::to_bits),
            cut,
            layers: list,
            len: layers.len,
            pixels_per_point: ppp.to_bits(),
            max_side: plan.max_side,
        }
    }
}

/// One shadow's texture tiles and when they were built and last used.
#[derive(Clone)]
struct Entry {
    tiles: Vec<TextureHandle>,
    bytes: usize,
    built: u64,
    used: u64,
}

/// Shadow textures. A shadow stays while the last two passes use it, and
/// up to [`MAX_IDLE`] idle ones for [`MAX_AGE`] passes within
/// [`MAX_BYTES`]; a texture goes only after two passes without it, so no
/// mesh of this pass names a freed one. One a single pass used and none
/// since (a step of a shadow transition) lends its textures to the next
/// new shadow, so a transition does not fill the cache. egui ends each
/// pass of the cache (`CacheTrait::update`), so it empties when an app
/// stops drawing shadows.
#[derive(Clone, Default)]
struct ShadowCache {
    /// Passes ended so far: the number of this one.
    pass: u64,
    entries: std::collections::HashMap<Key, Entry>,
    /// Shadows built, for the tests.
    builds: u64,
}

impl egui::cache::CacheTrait for ShadowCache {
    /// The end of a pass: idle shadows past the bounds go.
    fn update(&mut self) {
        self.pass += 1;
        let pass = self.pass;
        let idle = |used: u64| used + 1 < pass;
        let mut ages: Vec<u64> = self
            .entries
            .values()
            .map(|e| e.used)
            .filter(|used| idle(*used))
            .collect();
        ages.sort_unstable_by(|a, b| b.cmp(a));
        // The oldest pass an idle shadow may come from; ties at the bound
        // may keep a few more than `MAX_IDLE`.
        let oldest = ages
            .get(MAX_IDLE.saturating_sub(1))
            .map_or(0, |used| *used)
            .max(pass.saturating_sub(MAX_AGE));
        self.entries
            .retain(|_, e| !idle(e.used) || e.used >= oldest);
        let mut bytes: usize = self.entries.values().map(|e| e.bytes).sum();
        while bytes > MAX_BYTES {
            let Some(key) = self
                .entries
                .iter()
                .filter(|(_, e)| idle(e.used))
                .min_by_key(|(_, e)| e.used)
                .map(|(k, _)| *k)
            else {
                break;
            };
            bytes -= self.entries.remove(&key).map_or(0, |e| e.bytes);
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

impl ShadowCache {
    /// The textures of shadow `key`, marked as used in this pass.
    fn get(&mut self, key: &Key) -> Option<Vec<egui::TextureId>> {
        let pass = self.pass;
        self.entries.get_mut(key).map(|e| {
            e.used = pass;
            e.tiles.iter().map(TextureHandle::id).collect()
        })
    }

    /// The `n` textures of a shadow one pass built and used, and none
    /// since, taken out to be refilled.
    fn spare(&mut self, n: usize) -> Option<Vec<TextureHandle>> {
        let pass = self.pass;
        let key = *self
            .entries
            .iter()
            .find(|(_, e)| e.built == e.used && e.used + 2 <= pass && e.tiles.len() == n)?
            .0;
        self.entries.remove(&key).map(|e| e.tiles)
    }

    fn insert(&mut self, key: Key, tiles: Vec<TextureHandle>, bytes: usize) {
        let pass = self.pass;
        self.entries.insert(
            key,
            Entry {
                tiles,
                bytes,
                built: pass,
                used: pass,
            },
        );
        self.builds += 1;
    }
}

#[cfg(test)]
fn shadow_builds(ctx: &egui::Context) -> u64 {
    ctx.memory_mut(|m| m.caches.cache::<ShadowCache>().builds)
}

/// Textures the cache holds.
#[cfg(test)]
fn shadow_textures(ctx: &egui::Context) -> usize {
    ctx.memory_mut(|m| {
        let c = m.caches.cache::<ShadowCache>();
        c.entries.values().map(|e| e.tiles.len()).sum()
    })
}

/// A rounded rect in device pixels: x0, y0, x1, y1 and the radii nw, ne,
/// sw, se.
#[derive(Clone, Copy, Debug)]
struct Round {
    rect: [f64; 4],
    radii: [f64; 4],
}

impl Round {
    /// Signed distance from (x, y) to the outline, negative inside.
    fn sd(&self, x: f64, y: f64) -> f64 {
        let [x0, y0, x1, y1] = self.rect;
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let [nw, ne, sw, se] = self.radii;
        let r = match (x >= cx, y >= cy) {
            (false, false) => nw,
            (true, false) => ne,
            (false, true) => sw,
            (true, true) => se,
        };
        let qx = (x - cx).abs() - (x1 - x0) / 2.0 + r;
        let qy = (y - cy).abs() - (y1 - y0) / 2.0 + r;
        qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
    }

    /// The share of a pixel centred on (x, y) inside, feathered over one
    /// device pixel as egui feathers a fill.
    fn feathered(&self, x: f64, y: f64) -> f64 {
        (0.5 - self.sd(x, y)).clamp(0.0, 1.0)
    }

    /// The largest radius at the low (`far` false) or high end of `axis`.
    fn corner(&self, axis: usize, far: bool) -> f64 {
        let [nw, ne, sw, se] = self.radii;
        match (axis, far) {
            (0, false) => nw.max(sw),
            (0, true) => ne.max(se),
            (_, false) => nw.max(ne),
            (_, true) => sw.max(se),
        }
    }
}

/// One layer in device pixels: its shape, deviation (0 for a hard edge),
/// alpha and colour, and where its shape starts after the box's start and
/// ends after the box's end along each axis.
#[derive(Clone, Copy, Debug)]
struct DeviceLayer {
    shape: Round,
    sigma: f64,
    alpha: f64,
    rgb: [f64; 3],
    edges: [[f64; 2]; 2],
}

impl DeviceLayer {
    /// How far past its shape the layer reaches.
    fn reach(&self) -> f64 {
        if self.sigma > 0.0 {
            4.5 * self.sigma
        } else {
            1.0
        }
    }
}

/// A shadow in device pixels, the box from (0, 0) to its size.
#[derive(Clone, Debug)]
struct Device {
    size: [f64; 2],
    /// The box as egui fills it, which the shadow is cut to the outside of.
    cut: Round,
    /// The box with its CSS radii, which the layers grow from.
    border: Round,
    layers: Vec<DeviceLayer>,
}

impl Device {
    fn new(size: egui::Vec2, radii: [f32; 4], cut: CornerRadius, layers: Layers, ppp: f32) -> Self {
        let (w, h) = (
            (size.x * ppp).round().max(0.0),
            (size.y * ppp).round().max(0.0),
        );
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(w, h));
        let radii = clamp_radii(radii.map(|r| r * ppp), rect);
        let half = w.min(h) / 2.0;
        let cut = [cut.nw, cut.ne, cut.sw, cut.se].map(|r| (f32::from(r) * ppp).min(half));
        let round = |r: Rect, radii: [f32; 4]| Round {
            rect: [r.min.x, r.min.y, r.max.x, r.max.y].map(f64::from),
            radii: radii.map(f64::from),
        };
        let layers = layers
            .as_slice()
            .iter()
            .map(|l| {
                let spread = l.spread() * ppp;
                let offset = l.offset() * ppp;
                // Along each axis: the shape's start after the box's and its
                // end after the box's, both independent of the box's size
                // unless a negative spread shrinks the shape to its centre.
                let edges = [(w, offset.x), (h, offset.y)].map(|(len, off)| {
                    let (len, off, s) = (f64::from(len), f64::from(off), f64::from(spread));
                    if len / 2.0 + s > 0.0 {
                        [off - s, off + s]
                    } else {
                        [len / 2.0 + off, off - len / 2.0]
                    }
                });
                let shape = Rect::from_min_max(
                    egui::pos2(edges[0][0] as f32, edges[1][0] as f32),
                    egui::pos2(w + edges[0][1] as f32, h + edges[1][1] as f32),
                );
                let shape_radii = clamp_radii(radii.map(|r| spread_radius(r, spread)), shape);
                let [r, g, b, _] = l.color().to_srgba_unmultiplied();
                let mut shape = round(shape, shape_radii);
                // The far edges exactly, not through the f32 rect.
                shape.rect[2] = f64::from(w) + edges[0][1];
                shape.rect[3] = f64::from(h) + edges[1][1];
                shape.rect[0] = edges[0][0];
                shape.rect[1] = edges[1][0];
                DeviceLayer {
                    shape,
                    sigma: f64::from(l.blur() * ppp) / 2.0,
                    alpha: f64::from(l.alpha()).clamp(0.0, 1.0),
                    rgb: [r, g, b].map(|c| f64::from(c) / 255.0),
                    edges,
                }
            })
            .collect();
        Device {
            size: [f64::from(w), f64::from(h)],
            cut: round(rect, cut),
            border: round(rect, radii),
            layers,
        }
    }

    /// The composited shadow, premultiplied (r, g, b, a) in 0..=1, at the
    /// pixel centred on (x, y), given each layer's coverage there.
    fn compose(&self, x: f64, y: f64, coverage: impl Fn(usize) -> f64) -> [f64; 4] {
        let keep = 1.0 - self.cut.feathered(x, y);
        let mut v = [0.0; 4];
        if keep <= 0.0 {
            return v;
        }
        for (k, l) in self.layers.iter().enumerate().rev() {
            let a = l.alpha * coverage(k).clamp(0.0, 1.0);
            for (v, rgb) in v.iter_mut().zip(l.rgb) {
                *v = a * rgb + *v * (1.0 - a);
            }
            v[3] = a + v[3] * (1.0 - a);
        }
        v.map(|c| c * keep)
    }
}

/// The texture's layout along one axis, in device pixels from the box's
/// start: the shadow spans `lo..hi`; with `stretch`, `lo..inner_lo` and
/// `inner_hi..hi` are its corners, a texel each, and `inner_lo..inner_hi`
/// one texel stretched, as the shadow does not change there.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Axis {
    lo: i64,
    inner_lo: i64,
    inner_hi: i64,
    hi: i64,
    stretch: bool,
}

impl Axis {
    /// The layout along `axis` (0 is x): the corner slices hold every
    /// corner of the box and of each layer's shape, plus the layer's
    /// reach and a pixel.
    fn new(device: &Device, axis: usize) -> Axis {
        let len = device.size[axis];
        let mut lo: f64 = 0.0;
        let mut hi: f64 = 0.0;
        let corner = |far| {
            device
                .cut
                .corner(axis, far)
                .max(device.border.corner(axis, far))
        };
        let mut inner_lo = corner(false) + 1.0;
        let mut inner_hi = -corner(true) - 1.0;
        for l in &device.layers {
            let [start, end] = l.edges[axis];
            let reach = l.reach() + 1.0;
            lo = lo.min(start - reach);
            hi = hi.max(end + reach);
            inner_lo = inner_lo.max(start + l.shape.corner(axis, false) + reach);
            inner_hi = inner_hi.min(end - l.shape.corner(axis, true) - reach);
        }
        // `hi` and `inner_hi` count from the box's end, so they round the
        // same at every size.
        let len = len as i64;
        let (lo, hi) = (lo.floor() as i64, len + hi.ceil() as i64);
        let (inner_lo, inner_hi) = (inner_lo.ceil() as i64, len + inner_hi.floor() as i64);
        Axis {
            lo,
            inner_lo,
            inner_hi,
            hi,
            stretch: inner_hi > inner_lo + 1,
        }
    }

    fn texels(&self) -> i64 {
        if self.stretch {
            (self.inner_lo - self.lo) + 1 + (self.hi - self.inner_hi)
        } else {
            self.hi - self.lo
        }
    }

    /// The device pixel texel `t` holds.
    fn source(&self, t: i64) -> i64 {
        let a = self.inner_lo - self.lo;
        if !self.stretch || t < a {
            self.lo + t
        } else if t == a {
            self.inner_lo
        } else {
            self.inner_hi + (t - a - 1)
        }
    }

    /// The runs of pixels a quad covers: (first pixel, end, first texel,
    /// stretched).
    fn runs(&self) -> Vec<(i64, i64, i64, bool)> {
        if self.stretch {
            let a = self.inner_lo - self.lo;
            vec![
                (self.lo, self.inner_lo, 0, false),
                (self.inner_lo, self.inner_hi, a, true),
                (self.inner_hi, self.hi, a + 1, false),
            ]
        } else {
            vec![(self.lo, self.hi, 0, false)]
        }
    }
}

/// A shadow's texture layout and tiles: no tile is wider or taller than
/// the renderer's largest texture.
#[derive(Clone, Debug)]
struct Plan {
    size: [i64; 2],
    axes: [Axis; 2],
    max_side: usize,
}

impl Plan {
    /// `None` for a shadow with nothing to paint.
    fn new(device: &Device, max_side: usize) -> Option<Plan> {
        let axes = [Axis::new(device, 0), Axis::new(device, 1)];
        let visible = device.layers.iter().any(|l| l.alpha > 0.0);
        (visible && axes.iter().all(|a| a.texels() > 0)).then(|| Plan {
            size: device.size.map(|v| v as i64),
            axes,
            max_side,
        })
    }

    /// Tiles along `axis`: their first texel and texel count.
    fn tiles(&self, axis: usize) -> Vec<(i64, i64)> {
        let (n, side) = (self.axes[axis].texels(), self.max_side as i64);
        (0..(n as u64).div_ceil(side as u64) as i64)
            .map(|k| (k * side, side.min(n - k * side)))
            .collect()
    }

    /// The tiles' images, row by row.
    fn images(&self, device: &Device) -> Vec<egui::ColorImage> {
        let [tw, th] = self.axes.map(|a| a.texels() as usize);
        let texels = texels(device, &self.axes);
        let mut out = Vec::new();
        for (y0, h) in self.tiles(1) {
            for (x0, w) in self.tiles(0) {
                let (x0, y0, w, h) = (x0 as usize, y0 as usize, w as usize, h as usize);
                let mut pixels = Vec::with_capacity(w * h);
                for y in y0..y0 + h {
                    pixels.extend_from_slice(&texels[y * tw + x0..y * tw + x0 + w]);
                }
                debug_assert!(y0 + h <= th);
                out.push(egui::ColorImage::new([w, h], pixels));
            }
        }
        out
    }

    /// The quads that show the tiles `ids` for the box whose start is at
    /// device pixel `origin`, faded by `opacity`: one mesh per tile. The
    /// middle of a box that stretches both ways is inside it and gets none.
    fn shape(&self, ids: &[egui::TextureId], origin: [f64; 2], ppp: f32, opacity: f32) -> Shape {
        let tiles = [self.tiles(0), self.tiles(1)];
        // Runs cut where a tile ends: (first pixel, end, tile, texel in
        // the tile, stretched).
        let split = |axis: usize| {
            let mut out = Vec::new();
            for (p0, p1, t0, stretched) in self.axes[axis].runs() {
                if p1 <= p0 {
                    continue;
                }
                for (k, &(start, len)) in tiles[axis].iter().enumerate() {
                    let end = start + len;
                    if stretched {
                        if (start..end).contains(&t0) {
                            out.push((p0, p1, k, t0 - start, true));
                        }
                        continue;
                    }
                    let (a, b) = (t0.max(start), (t0 + p1 - p0).min(end));
                    if a < b {
                        out.push((p0 + a - t0, p0 + b - t0, k, a - start, false));
                    }
                }
            }
            out
        };
        let (xs, ys) = (split(0), split(1));
        let color = Color32::WHITE.gamma_multiply(opacity);
        let mut meshes: Vec<Mesh> = ids.iter().map(|id| Mesh::with_texture(*id)).collect();
        let to_points = |v: f64| (v / f64::from(ppp)) as f32;
        for &(y0, y1, ty, ry, sy) in &ys {
            for &(x0, x1, tx, rx, sx) in &xs {
                if sx && sy {
                    continue;
                }
                let (tile_w, tile_h) = (tiles[0][tx].1 as f32, tiles[1][ty].1 as f32);
                let uv = |r: i64, p0: i64, p1: i64, s: bool, side: f32| {
                    if s {
                        let c = (r as f32 + 0.5) / side;
                        (c, c)
                    } else {
                        (r as f32 / side, (r + p1 - p0) as f32 / side)
                    }
                };
                let (u0, u1) = uv(rx, x0, x1, sx, tile_w);
                let (v0, v1) = uv(ry, y0, y1, sy, tile_h);
                let rect = Rect::from_min_max(
                    egui::pos2(
                        to_points(origin[0] + x0 as f64),
                        to_points(origin[1] + y0 as f64),
                    ),
                    egui::pos2(
                        to_points(origin[0] + x1 as f64),
                        to_points(origin[1] + y1 as f64),
                    ),
                );
                let uv = Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u1, v1));
                meshes[ty * tiles[0].len() + tx].add_rect_with_uv(rect, uv, color);
            }
        }
        let mut shapes: Vec<Shape> = meshes
            .into_iter()
            .filter(|m| !m.is_empty())
            .map(Shape::mesh)
            .collect();
        match shapes.len() {
            0 => Shape::Noop,
            1 => shapes.remove(0),
            _ => Shape::Vec(shapes),
        }
    }
}

/// 4-point Gauss-Legendre nodes and weights on 0..1.
const GAUSS_4: [(f64, f64); 4] = [
    (0.069_431_844_202_973_7, 0.173_927_422_568_726_9),
    (0.330_009_478_207_571_9, 0.326_072_577_431_273_1),
    (0.669_990_521_792_428_1, 0.326_072_577_431_273_1),
    (0.930_568_155_797_026_3, 0.173_927_422_568_726_9),
];

/// The texels, row by row, premultiplied. Per blurred layer the Gaussian
/// of the rounded rect is separable along the straight sides: one error
/// function pair per texel column and one per row. Across a corner band
/// the rows' integral runs on fixed nodes, 4-point Gauss-Legendre panels
/// at most a deviation long in the angle t of y = edge + H (1 - cos t),
/// which takes out the square root of the corner's arc at the band's edge;
/// the span in x is integrated once per node and column, and each texel
/// row sums the nodes within 5 deviations, weighted by the Gaussian. A
/// hard layer is its shape feathered over one device pixel.
fn texels(device: &Device, axes: &[Axis; 2]) -> Vec<Color32> {
    let [tw, th] = axes.map(|a| a.texels() as usize);
    let width = device.size[0] as i64;
    // Columns fold onto their mirror where every layer is centred along x
    // and the radii are left/right symmetric (every theme shadow).
    let symmetric = |r: &Round| r.radii[0] == r.radii[1] && r.radii[2] == r.radii[3];
    let fold = device
        .layers
        .iter()
        .all(|l| l.edges[0][0] == -l.edges[0][1] && symmetric(&l.shape))
        && symmetric(&device.cut)
        && symmetric(&device.border);
    let canonical = |i: i64| if fold { i.min(width - 1 - i) } else { i };
    let mut cols: Vec<i64> = (0..tw as i64)
        .map(|t| canonical(axes[0].source(t)))
        .collect();
    cols.sort_unstable();
    cols.dedup();
    let col_of: Vec<usize> = (0..tw as i64)
        .map(|t| cols.partition_point(|&c| c < canonical(axes[0].source(t))))
        .collect();
    let xs: Vec<f64> = cols.iter().map(|&c| c as f64 + 0.5).collect();
    let ys: Vec<f64> = (0..th as i64)
        .map(|t| axes[1].source(t) as f64 + 0.5)
        .collect();
    let coverage: Vec<Vec<f32>> = device
        .layers
        .iter()
        .map(|l| layer_coverage(l, &xs, &ys))
        .collect();
    let nc = xs.len();
    let mut out = vec![Color32::TRANSPARENT; tw * th];
    for (j, &y) in ys.iter().enumerate() {
        for t in 0..tw {
            let x = axes[0].source(t as i64) as f64 + 0.5;
            let c = col_of[t];
            let v = device.compose(x, y, |k| f64::from(coverage[k][j * nc + c]));
            let [r, g, b, a] = v.map(|c| (c * 255.0).round().clamp(0.0, 255.0) as u8);
            out[j * tw + t] = Color32::from_rgba_premultiplied(r, g, b, a);
        }
    }
    out
}

/// One layer's coverage at every (row `ys`, column `xs`) pixel centre.
fn layer_coverage(layer: &DeviceLayer, xs: &[f64], ys: &[f64]) -> Vec<f32> {
    let (nc, shape) = (xs.len(), layer.shape);
    let mut cov = vec![0f32; ys.len() * nc];
    let sigma = layer.sigma;
    if sigma <= 0.0 {
        for (j, &y) in ys.iter().enumerate() {
            for (c, &x) in xs.iter().enumerate() {
                cov[j * nc + c] = shape.feathered(x, y) as f32;
            }
        }
        return cov;
    }
    let [x0, y0, x1, y1] = shape.rect;
    let [nw, ne, sw, se] = shape.radii;
    let cdf = |t: f64| 0.5 * (1.0 + erf(t / (sigma * SQRT_2)));
    // The shape's span at height y.
    let span = |y: f64| {
        let inset = |top: f64, bottom: f64| {
            let (r, dy) = if y < y0 + top {
                (top, y0 + top - y)
            } else if y > y1 - bottom {
                (bottom, y - (y1 - bottom))
            } else {
                return 0.0;
            };
            r - (r * r - dy * dy).max(0.0).sqrt()
        };
        (x0 + inset(nw, sw), x1 - inset(ne, se))
    };
    let along = |(a, b): (f64, f64), x: f64| (cdf(b - x) - cdf(a - x)).max(0.0);
    let top_band = (y0 + nw.max(ne)).min(y1);
    let bottom_band = (y1 - sw.max(se)).max(top_band);
    let middle = span(((y0 + y1) / 2.0).clamp(top_band, bottom_band));
    let middle_x: Vec<f64> = xs.iter().map(|&x| along(middle, x)).collect();
    // The corner bands' nodes (y, weight), ascending.
    let mut nodes: Vec<(f64, f64)> = Vec::new();
    for (edge, h, dir) in [(y0, top_band - y0, 1.0), (y1, y1 - bottom_band, -1.0)] {
        if h <= 0.0 {
            continue;
        }
        let panels = (h * FRAC_PI_2 / sigma).ceil().max(2.0) as usize;
        let dt = FRAC_PI_2 / panels as f64;
        for p in 0..panels {
            for (u, w) in GAUSS_4 {
                let t = (p as f64 + u) * dt;
                nodes.push((edge + dir * h * (1.0 - t.cos()), w * dt * h * t.sin()));
            }
        }
    }
    nodes.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut rows = vec![0f32; nodes.len() * nc];
    for (k, &(y, _)) in nodes.iter().enumerate() {
        let s = span(y);
        for (c, &x) in xs.iter().enumerate() {
            rows[k * nc + c] = along(s, x) as f32;
        }
    }
    let norm = 1.0 / (sigma * (2.0 * PI).sqrt());
    for (j, &y) in ys.iter().enumerate() {
        let out = &mut cov[j * nc..(j + 1) * nc];
        let straight = (cdf(bottom_band - y) - cdf(top_band - y)).max(0.0);
        for (o, m) in out.iter_mut().zip(&middle_x) {
            *o = (m * straight) as f32;
        }
        let a = nodes.partition_point(|n| n.0 < y - 5.0 * sigma);
        let b = nodes.partition_point(|n| n.0 <= y + 5.0 * sigma);
        for (k, &(ny, w)) in nodes.iter().enumerate().take(b).skip(a) {
            let t = (ny - y) / sigma;
            let g = (w * norm * (-0.5 * t * t).exp()) as f32;
            for (o, &r) in out.iter_mut().zip(&rows[k * nc..(k + 1) * nc]) {
                *o += g * r;
            }
        }
    }
    cov
}

/// The error function, Numerical Recipes' `erfcc` (fractional error below
/// 1.2e-7).
fn erf(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let poly = -z * z - 1.265_512_23
        + t * (1.000_023_68
            + t * (0.374_091_96
                + t * (0.096_784_18
                    + t * (-0.186_288_06
                        + t * (0.278_868_07
                            + t * (-1.135_203_98
                                + t * (1.488_515_87 + t * (-0.822_152_23 + t * 0.170_872_77))))))));
    let r = t * poly.exp();
    if x >= 0.0 {
        1.0 - r
    } else {
        r - 1.0
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erf_matches_known_values() {
        for (x, want) in [
            (0.0, 0.0),
            (0.5, 0.520_499_877_8),
            (1.0, 0.842_700_792_9),
            (2.0, 0.995_322_265_0),
        ] {
            assert!((erf(x) - want).abs() < 2e-7, "erf({x})");
            assert!((erf(-x) + want).abs() < 2e-7, "erf(-{x})");
        }
    }

    #[test]
    fn spread_follows_css() {
        assert_eq!(spread_radius(16.0, -12.0), 4.0);
        assert_eq!(spread_radius(4.0, -12.0), 0.0);
        assert_eq!(spread_radius(0.0, 4.0), 0.0);
        assert_eq!(spread_radius(8.0, 4.0), 12.0);
    }

    /// Tailwind's `shadow-sm`: two layers, one with a negative spread.
    fn shadow_sm() -> Layers {
        let black = Color32::BLACK;
        Layers::new(&[
            ShadowLayer::new(egui::vec2(0.0, 1.0), 3.0, 0.0, black, 0.1),
            ShadowLayer::new(egui::vec2(0.0, 1.0), 2.0, -1.0, black, 0.1),
        ])
    }

    /// The `layers` shadow of a `size` box at `at`, faded by `opacity`,
    /// drawn in one frame of `ctx` (none where `at` is `None`).
    fn frame_with(
        ctx: &egui::Context,
        at: Option<Pos2>,
        size: egui::Vec2,
        layers: Layers,
        opacity: f32,
    ) -> Shape {
        let mut shape = Shape::Noop;
        let _ = ctx.run_ui(egui::RawInput::default(), |_| {
            if let Some(at) = at {
                let rect = Rect::from_min_size(at, size);
                shape = shadow_shape(ctx, rect, [6.0; 4], CornerRadius::same(6), layers, opacity);
            }
        });
        shape
    }

    fn frame(ctx: &egui::Context, at: Option<Pos2>, opacity: f32) -> Shape {
        frame_with(ctx, at, egui::vec2(80.0, 30.0), shadow_sm(), opacity)
    }

    fn triangles(shape: &Shape) -> usize {
        match shape {
            Shape::Mesh(m) => m.indices.len() / 3,
            Shape::Vec(v) => v.iter().map(triangles).sum(),
            _ => 0,
        }
    }

    /// The first frame of a shadowed box builds one small texture and
    /// paints a few quads, where 0.7 built ~13k triangles in ~130 ms; a
    /// box that moves, grows, fades or comes back after a frame without it
    /// builds nothing (#25).
    #[test]
    fn a_moved_resized_faded_or_returning_shadow_is_not_rebuilt() {
        let ctx = egui::Context::default();
        let start = shadow_builds(&ctx);
        let t = std::time::Instant::now();
        let first = frame(&ctx, Some(egui::pos2(10.0, 10.0)), 1.0);
        let first_ms = t.elapsed().as_secs_f64() * 1e3;
        assert_eq!(shadow_builds(&ctx) - start, 1, "first frame");
        assert!(triangles(&first) <= 16, "{} triangles", triangles(&first));
        assert!(first_ms < 50.0, "first frame {first_ms:.1} ms");
        for x in 11..15 {
            let at = egui::pos2(x as f32 + 0.3, 2.0 * x as f32);
            let _ = frame(&ctx, Some(at), 1.0);
        }
        assert_eq!(shadow_builds(&ctx) - start, 1, "moved");
        for w in [90.0, 200.0, 41.0] {
            let _ = frame_with(
                &ctx,
                Some(egui::pos2(3.0, 4.0)),
                egui::vec2(w, 60.0),
                shadow_sm(),
                1.0,
            );
        }
        assert_eq!(shadow_builds(&ctx) - start, 1, "resized");
        for o in [0.9, 0.5, 0.1] {
            let _ = frame(&ctx, Some(egui::pos2(14.0, 28.0)), o);
        }
        assert_eq!(shadow_builds(&ctx) - start, 1, "faded");
        let _ = frame(&ctx, None, 1.0);
        let _ = frame(&ctx, Some(egui::pos2(14.0, 28.0)), 1.0);
        assert_eq!(shadow_builds(&ctx) - start, 1, "absent for a frame");
    }

    /// A shadow whose layers change every frame (a transition) refills the
    /// textures of its last steps instead of keeping one per step, and a
    /// box too small to stretch, resized every frame, keeps the cache
    /// bounded.
    #[test]
    fn the_shadow_cache_stays_bounded_while_a_shadow_animates() {
        let ctx = egui::Context::default();
        for f in 0..40 {
            let blur = 2.0 + f as f32 * 0.25;
            let layers = Layers::new(&[ShadowLayer::new(
                egui::vec2(0.0, 2.0),
                blur,
                0.0,
                Color32::BLACK,
                0.2,
            )]);
            let _ = frame_with(
                &ctx,
                Some(egui::pos2(10.0, 10.0)),
                egui::vec2(80.0, 30.0),
                layers,
                1.0,
            );
        }
        assert_eq!(shadow_builds(&ctx), 40);
        let textures = shadow_textures(&ctx);
        assert!(textures <= 3, "transition textures {textures}");
        for f in 0..40 {
            let size = egui::vec2(8.0 + f as f32, 10.0);
            let _ = frame_with(&ctx, Some(egui::pos2(10.0, 10.0)), size, shadow_sm(), 1.0);
        }
        let textures = shadow_textures(&ctx);
        assert!(textures <= MAX_IDLE + 4, "resize textures {textures}");
    }

    /// An app that stops drawing shadows lets go of the textures: they go
    /// once idle for [`MAX_AGE`] passes, without another shadow drawn.
    #[test]
    fn the_shadow_cache_empties_when_no_shadow_is_drawn() {
        let ctx = egui::Context::default();
        for w in [10.0, 11.0, 12.0] {
            for _ in 0..2 {
                let _ = frame_with(
                    &ctx,
                    Some(egui::pos2(10.0, 10.0)),
                    egui::vec2(w, w),
                    shadow_sm(),
                    1.0,
                );
            }
        }
        assert_eq!(shadow_textures(&ctx), 3);
        for _ in 0..2 {
            let _ = frame(&ctx, None, 1.0);
        }
        assert_eq!(
            shadow_textures(&ctx),
            3,
            "a box away for a frame comes back"
        );
        for _ in 0..MAX_AGE {
            let _ = frame(&ctx, None, 1.0);
        }
        assert_eq!(shadow_textures(&ctx), 0);
    }

    /// The cache drops its textures while egui ends a pass, under egui's
    /// context lock. Dropping a texture locks the texture manager, a lock of
    /// its own that egui takes after the context's, never before; so the
    /// pass ends, and the textures are freed in the output of that pass.
    #[test]
    fn textures_dropped_at_the_end_of_a_pass_are_freed_without_a_deadlock() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let ctx = egui::Context::default();
            let _ = frame(&ctx, None, 1.0);
            let count = || ctx.tex_manager().read().num_allocated();
            let before = count();
            let _ = frame(&ctx, Some(egui::pos2(10.0, 10.0)), 1.0);
            let built = count();
            let mut freed = Vec::new();
            for _ in 0..MAX_AGE + 2 {
                let out = ctx.run_ui(egui::RawInput::default(), |_| {});
                freed.extend(out.textures_delta.free);
            }
            let _ = tx.send((before, built, count(), freed.len()));
        });
        let (before, built, after, freed) = rx
            .recv_timeout(std::time::Duration::from_secs(60))
            .expect("the passes that drop the shadow's texture end");
        assert_eq!(built, before + 1, "one shadow texture");
        assert_eq!(after, before, "the texture is gone");
        assert_eq!(freed, 1, "the backend is told to free it");
    }

    /// Past [`MAX_BYTES`] idle shadows go, oldest first; ones the last two
    /// passes used stay.
    #[test]
    fn the_shadow_cache_keeps_to_its_byte_budget() {
        use egui::cache::CacheTrait as _;
        let ctx = egui::Context::default();
        let tile = || {
            let image = egui::ColorImage::new([1, 1], vec![Color32::TRANSPARENT]);
            vec![ctx.load_texture("t", image, OPTIONS)]
        };
        let device = Device::new(
            egui::vec2(20.0, 20.0),
            [4.0; 4],
            CornerRadius::same(4),
            shadow_sm(),
            1.0,
        );
        let plan = Plan::new(&device, 2048).expect("a shadow");
        let mut cache = ShadowCache::default();
        let mib = 1 << 20;
        for k in 0..5 {
            let ppp = 1.0 + k as f32;
            let key = Key::new(&plan, [k as f32; 4], CornerRadius::ZERO, shadow_sm(), ppp);
            cache.insert(key, tile(), 6 * mib);
            cache.update();
        }
        // The last pass used the newest; the idle ones go, oldest first,
        // until 12 MiB remain.
        assert_eq!(cache.len(), 2);
        let bytes: usize = cache.entries.values().map(|e| e.bytes).sum();
        assert!(bytes <= MAX_BYTES, "{bytes}");
    }

    /// Premultiplied alpha the shapes show at device pixel (i, j) of a box
    /// whose start is at device pixel `origin`, sampled as a GPU samples
    /// NEAREST textures: the texel under the pixel centre times the vertex
    /// colour. `images` holds each mesh's texture by `TextureId::User`.
    fn sample(shape: &Shape, images: &[egui::ColorImage], ppp: f32, p: [f64; 2]) -> f64 {
        let meshes: Vec<&Mesh> = match shape {
            Shape::Mesh(m) => vec![&**m],
            Shape::Vec(v) => v
                .iter()
                .filter_map(|s| match s {
                    Shape::Mesh(m) => Some(&**m),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        };
        let q = [p[0] / f64::from(ppp), p[1] / f64::from(ppp)];
        let mut found = 0.0;
        let mut hits = 0;
        for m in meshes {
            let egui::TextureId::User(k) = m.texture_id else {
                panic!("a shadow texture")
            };
            let image = &images[k as usize];
            for t in m.indices.chunks(3) {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| m.vertices[i as usize]);
                let pos = |v: &egui::epaint::Vertex| [f64::from(v.pos.x), f64::from(v.pos.y)];
                let (pa, pb, pc) = (pos(&a), pos(&b), pos(&c));
                let cross = |u: [f64; 2], v: [f64; 2], w: [f64; 2]| {
                    (v[0] - u[0]) * (w[1] - u[1]) - (v[1] - u[1]) * (w[0] - u[0])
                };
                let area = cross(pa, pb, pc);
                if area.abs() < 1e-12 {
                    continue;
                }
                let (wa, wb, wc) = (
                    cross(pb, pc, q) / area,
                    cross(pc, pa, q) / area,
                    cross(pa, pb, q) / area,
                );
                if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                    continue;
                }
                let uv = |k: usize| {
                    let f =
                        |v: &egui::epaint::Vertex| f64::from(if k == 0 { v.uv.x } else { v.uv.y });
                    wa * f(&a) + wb * f(&b) + wc * f(&c)
                };
                let [w, h] = image.size;
                let x = ((uv(0) * w as f64).floor() as usize).min(w - 1);
                let y = ((uv(1) * h as f64).floor() as usize).min(h - 1);
                let texel = f64::from(image.pixels[y * w + x].a());
                found = texel * f64::from(a.color.a()) / 255.0;
                hits += 1;
                break;
            }
        }
        assert!(hits <= 1, "pixel {p:?} under {hits} quads");
        found
    }

    /// The reference: the shadow's exact value, independent of the build.
    /// A layer's Gaussian over its rounded rect: the error function along
    /// x (Abramowitz and Stegun 7.1.26, not the build's), integrated along
    /// y by the midpoint rule within 6 deviations, in u with y = a + (b -
    /// a) u^2 towards the shape's edge, where the corner's span has a
    /// square root.
    mod reference {
        use std::f64::consts::{PI, SQRT_2};

        fn erf(x: f64) -> f64 {
            let t = 1.0 / (1.0 + 0.327_591_1 * x.abs());
            let p = t
                * (0.254_829_592
                    + t * (-0.284_496_736
                        + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
            (1.0 - p * (-x * x).exp()).copysign(x)
        }

        /// x0, y0, x1, y1 and nw, ne, sw, se.
        pub(super) type Shape = ([f64; 4], [f64; 4]);

        /// CSS Backgrounds 3, "corner overlap": all radii scale by the
        /// smallest side length over the sum of its two radii, if below 1.
        pub(super) fn clamp(radii: [f64; 4], w: f64, h: f64) -> [f64; 4] {
            let [nw, ne, sw, se] = radii;
            let mut f: f64 = 1.0;
            for (len, sum) in [(w, nw + ne), (w, sw + se), (h, nw + sw), (h, ne + se)] {
                if sum > 0.0 {
                    f = f.min(len.max(0.0) / sum);
                }
            }
            radii.map(|r| (r * f).max(0.0))
        }

        /// CSS Backgrounds 3, "spread distance": a shadow corner's radius
        /// is the radius plus the spread, except that under a positive
        /// spread a radius r < spread grows by spread (1 + (r / spread -
        /// 1)^3); never below 0.
        pub(super) fn spread(r: f64, spread: f64) -> f64 {
            if spread > 0.0 && r < spread {
                let q = r / spread - 1.0;
                r + spread * (1.0 + q * q * q)
            } else {
                (r + spread).max(0.0)
            }
        }

        pub(super) fn feather(x: f64, y: f64, (r, [nw, ne, sw, se]): Shape) -> f64 {
            // Distance to the outline, by the corner circle nearest.
            let corner = |cx: f64, cy: f64, rad: f64, inx: bool, iny: bool| (cx, cy, rad, inx, iny);
            let cs = [
                corner(r[0] + nw, r[1] + nw, nw, x < r[0] + nw, y < r[1] + nw),
                corner(r[2] - ne, r[1] + ne, ne, x > r[2] - ne, y < r[1] + ne),
                corner(r[0] + sw, r[3] - sw, sw, x < r[0] + sw, y > r[3] - sw),
                corner(r[2] - se, r[3] - se, se, x > r[2] - se, y > r[3] - se),
            ];
            let d = cs
                .iter()
                .find(|c| c.3 && c.4 && c.2 > 0.0)
                .map(|&(cx, cy, rad, _, _)| ((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - rad)
                .unwrap_or_else(|| {
                    let dx = (r[0] - x).max(x - r[2]);
                    let dy = (r[1] - y).max(y - r[3]);
                    if dx > 0.0 && dy > 0.0 {
                        dx.hypot(dy)
                    } else {
                        dx.max(dy)
                    }
                });
            (0.5 - d).clamp(0.0, 1.0)
        }

        pub(super) fn blurred(x: f64, y: f64, (r, [nw, ne, sw, se]): Shape, sigma: f64) -> f64 {
            let cdf = |t: f64| 0.5 * (1.0 + erf(t / (sigma * SQRT_2)));
            let row = |v: f64| {
                let inset = |top: f64, bottom: f64| {
                    let (rad, d) = if v < r[1] + top {
                        (top, r[1] + top - v)
                    } else if v > r[3] - bottom {
                        (bottom, v - (r[3] - bottom))
                    } else {
                        return 0.0;
                    };
                    rad - (rad * rad - d * d).max(0.0).sqrt()
                };
                (cdf(r[2] - inset(ne, se) - x) - cdf(r[0] + inset(nw, sw) - x)).max(0.0)
            };
            let top = (r[1] + nw.max(ne)).min(r[3]);
            let bottom = (r[3] - sw.max(se)).max(top);
            let mid = row((r[1] + r[3]) / 2.0);
            let mut sum = mid * (cdf(bottom - y) - cdf(top - y)).max(0.0);
            let norm = 1.0 / (sigma * (2.0 * PI).sqrt());
            let n = 200;
            for (a, b, from_top) in [(r[1], top, true), (bottom, r[3], false)] {
                let (lo, hi) = (a.max(y - 6.0 * sigma), b.min(y + 6.0 * sigma));
                if hi <= lo {
                    continue;
                }
                for i in 0..n {
                    let u = (i as f64 + 0.5) / n as f64;
                    let (v, dv) = if from_top {
                        (lo + (hi - lo) * u * u, (hi - lo) * 2.0 * u / n as f64)
                    } else {
                        (hi - (hi - lo) * u * u, (hi - lo) * 2.0 * u / n as f64)
                    };
                    let t = (v - y) / sigma;
                    sum += row(v) * norm * (-0.5 * t * t).exp() * dv;
                }
            }
            sum.clamp(0.0, 1.0)
        }
    }

    /// (name, width, height, radius, layers as (x, y, blur, spread,
    /// alpha), opaque): Tailwind's theme shadows and a hard, translucent
    /// shadow grown by a positive spread (judged after alpha), and custom
    /// opaque shadows with offsets past the corners, round ends and hard
    /// edges (judged at alpha 1).
    #[allow(clippy::type_complexity)]
    fn cases() -> Vec<(&'static str, f32, f32, f32, Vec<[f32; 5]>, bool)> {
        let xs = vec![[0.0, 1.0, 2.0, 0.0, 0.05]];
        let sm = vec![[0.0, 1.0, 3.0, 0.0, 0.1], [0.0, 1.0, 2.0, -1.0, 0.1]];
        let md = vec![[0.0, 4.0, 6.0, -1.0, 0.1], [0.0, 2.0, 4.0, -2.0, 0.1]];
        let lg = vec![[0.0, 10.0, 15.0, -3.0, 0.1], [0.0, 4.0, 6.0, -4.0, 0.1]];
        let xl = vec![[0.0, 20.0, 25.0, -5.0, 0.1], [0.0, 8.0, 10.0, -6.0, 0.1]];
        let x2 = vec![[0.0, 25.0, 50.0, -12.0, 0.25]];
        let spread = vec![[0.0, 0.0, 0.0, 3.0, 0.5]];
        let one = |x, y, blur| vec![[x, y, blur, 0.0, 1.0]];
        vec![
            ("xs button", 80.0, 36.0, 6.0, xs.clone(), false),
            ("xs checkbox", 16.0, 16.0, 4.0, xs, false),
            ("sm card", 350.0, 200.0, 12.0, sm, false),
            ("md popover", 288.0, 200.0, 6.0, md, false),
            ("lg dialog", 512.0, 300.0, 10.0, lg, false),
            ("xl", 400.0, 300.0, 12.0, xl, false),
            ("2xl", 400.0, 300.0, 12.0, x2, false),
            ("hard spread 3 a0.5", 80.0, 36.0, 6.0, spread, false),
            ("r8 (0,6) b1", 100.0, 60.0, 8.0, one(0.0, 6.0, 1.0), true),
            ("r8 (4,4) b1", 100.0, 60.0, 8.0, one(4.0, 4.0, 1.0), true),
            (
                "r255 (20,20) b2",
                80.0,
                30.0,
                255.0,
                one(20.0, 20.0, 2.0),
                true,
            ),
            (
                "r30 (-4,4) b0.5",
                100.0,
                60.0,
                30.0,
                one(-4.0, 4.0, 0.5),
                true,
            ),
            (
                "r30 (20,20) b2",
                100.0,
                60.0,
                30.0,
                one(20.0, 20.0, 2.0),
                true,
            ),
            (
                "r255 (100,0) b4",
                400.0,
                300.0,
                255.0,
                one(100.0, 0.0, 4.0),
                true,
            ),
            (
                "r8 (0,6) b0.1833",
                100.0,
                60.0,
                8.0,
                one(0.0, 6.0, 0.1833),
                true,
            ),
            ("hard r8 (4,4)", 100.0, 60.0, 8.0, one(4.0, 4.0, 0.0), true),
            (
                "hard r255 (100,100)",
                400.0,
                300.0,
                255.0,
                one(100.0, 100.0, 0.0),
                true,
            ),
        ]
    }

    /// The worst difference in levels of 255 between what the quads and
    /// textures show and the reference, at opacity 1 and 0.5, over the
    /// shadow's whole extent (every pixel, or a regular sample of at most
    /// `budget` pixels for big blurred shadows).
    fn worst(
        w: f32,
        h: f32,
        r: f32,
        layers: &[[f32; 5]],
        ppp: f32,
        max_side: usize,
        budget: usize,
    ) -> [f64; 2] {
        let list: Vec<ShadowLayer> = layers
            .iter()
            .map(|&[x, y, blur, spread, alpha]| {
                ShadowLayer::new(egui::vec2(x, y), blur, spread, Color32::BLACK, alpha)
            })
            .collect();
        let layers = Layers::new(&list);
        let size = egui::vec2(w, h);
        // The CSS radii: the build clamps them on the device pixels the box
        // is filled on, as the call site does.
        let cut = CornerRadius::same(r.min(255.0) as u8);
        let device = Device::new(size, [r; 4], cut, layers, ppp);
        let plan = Plan::new(&device, max_side).expect("a shadow");
        let images = plan.images(&device);
        let ids: Vec<egui::TextureId> = (0..images.len() as u64)
            .map(egui::TextureId::User)
            .collect();
        // The box starts at device pixel (40, 40).
        let origin = [40.0, 40.0];
        let shapes = [1.0, 0.5].map(|o| plan.shape(&ids, origin, ppp, o));
        // The reference geometry, from the CSS definitions.
        let (bw, bh) = (f64::from((w * ppp).round()), f64::from((h * ppp).round()));
        let device_radii = reference::clamp([f64::from(r) * f64::from(ppp); 4], bw, bh);
        let cut_r = (f64::from(cut.nw) * f64::from(ppp)).min(bw.min(bh) / 2.0);
        let boxed: reference::Shape = ([0.0, 0.0, bw, bh], [cut_r; 4]);
        let shadows: Vec<(reference::Shape, f64, f64)> = list
            .iter()
            .map(|l| {
                let s = l.spread() * ppp;
                let (ox, oy) = (f64::from(l.offset().x * ppp), f64::from(l.offset().y * ppp));
                let hx = (bw / 2.0 + f64::from(s)).max(0.0);
                let hy = (bh / 2.0 + f64::from(s)).max(0.0);
                let (cx, cy) = (bw / 2.0 + ox, bh / 2.0 + oy);
                let rect = [cx - hx, cy - hy, cx + hx, cy + hy];
                let rad = reference::clamp(
                    device_radii.map(|v| reference::spread(v, f64::from(s))),
                    2.0 * hx,
                    2.0 * hy,
                );
                (
                    (rect, rad),
                    f64::from(l.blur() * ppp) / 2.0,
                    f64::from(l.alpha()),
                )
            })
            .collect();
        let value = |x: f64, y: f64| {
            let keep = 1.0 - reference::feather(x, y, boxed);
            let mut v = 0.0;
            for &(shape, sigma, alpha) in shadows.iter().rev() {
                let cov = if sigma > 0.0 {
                    reference::blurred(x, y, shape, sigma)
                } else {
                    reference::feather(x, y, shape)
                };
                let a = alpha * cov;
                v = a + v * (1.0 - a);
            }
            v * keep * 255.0
        };
        let [ax, ay] = plan.axes;
        let n = ((ax.hi - ax.lo) * (ay.hi - ay.lo)) as usize;
        let stride = ((n as f64 / budget as f64).sqrt().ceil() as usize).max(1);
        let mut worst = [0.0_f64; 2];
        for j in (ay.lo..ay.hi).step_by(stride) {
            for i in (ax.lo..ax.hi).step_by(stride) {
                let (x, y) = (i as f64 + 0.5, j as f64 + 0.5);
                let want = value(x, y);
                let p = [origin[0] + x, origin[1] + y];
                for (k, o) in [1.0, 0.5].into_iter().enumerate() {
                    let got = sample(&shapes[k], &images, ppp, p);
                    worst[k] = worst[k].max((got - want * o).abs());
                }
            }
        }
        worst
    }

    /// Every theme shadow, the hard spread shadow and the custom shadows show
    /// within the precision rule against the independent reference: theme
    /// shadows within one level after alpha, faded to half too; custom
    /// opaque ones within 4 (hard edges within one level, so their 50 %
    /// line lies inside the device pixel egui feathers), at 1, 1.25, 1.5,
    /// 2 and 3 pixels per point.
    #[test]
    fn shadows_match_the_exact_gaussian() {
        let mut report = String::new();
        let mut failed = Vec::new();
        for (name, w, h, r, layers, opaque) in cases() {
            let hard = layers.iter().all(|l| l[2] == 0.0);
            for ppp in [1.0, 1.25, 1.5, 2.0, 3.0] {
                let budget = if hard { usize::MAX } else { 3000 };
                let [full, half] = worst(w, h, r, &layers, ppp, 8192, budget);
                let limit = if opaque && !hard { 4.0 } else { 1.0 };
                report += &format!("{name} @{ppp}: {full:.2} / {half:.2}\n");
                if full > limit || half > limit.max(1.0) {
                    failed.push(format!("{name} @{ppp}: {full:.2} / {half:.2} > {limit}"));
                }
            }
        }
        eprintln!("{report}");
        assert!(failed.is_empty(), "{failed:#?}");
    }

    /// A box of infinite or absurd size, or at an infinite place, paints
    /// no shadow and does not panic (an item filling unbounded room).
    #[test]
    fn an_infinite_or_absurd_box_has_no_shadow() {
        let ctx = egui::Context::default();
        let inf = f32::INFINITY;
        let rects = [
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(inf, 30.0)),
            Rect::from_min_max(egui::pos2(-inf, 0.0), egui::pos2(10.0, 30.0)),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(10.0, f32::NAN)),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1e13, 30.0)),
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(f32::MAX, f32::MAX)),
            Rect::EVERYTHING,
        ];
        for rect in rects {
            let mut shape = None;
            let _ = ctx.run_ui(egui::RawInput::default(), |_| {
                shape = Some(shadow_shape(
                    &ctx,
                    rect,
                    [6.0; 4],
                    CornerRadius::same(6),
                    shadow_sm(),
                    1.0,
                ));
            });
            assert!(matches!(shape, Some(Shape::Noop)), "{rect:?}");
        }
        assert_eq!(shadow_builds(&ctx), 0);
        // A box at the edge of the limit still gets one.
        let rect = Rect::from_min_size(egui::pos2(1e6, 0.0), egui::vec2(1e6, 30.0));
        assert!(
            triangles(&frame_with(
                &ctx,
                Some(rect.min),
                rect.size(),
                shadow_sm(),
                1.0
            )) > 0
        );
    }

    /// A texture larger than the renderer allows is cut into tiles that
    /// show the same pixels.
    #[test]
    fn a_big_shadow_is_tiled_to_the_largest_texture() {
        let layers = [[0.0, 20.0, 25.0, -5.0, 0.1], [0.0, 8.0, 10.0, -6.0, 0.1]];
        let device = Device::new(
            egui::vec2(400.0, 300.0),
            [12.0; 4],
            CornerRadius::same(12),
            Layers::new(&layers.map(|[x, y, b, s, a]| {
                ShadowLayer::new(egui::vec2(x, y), b, s, Color32::BLACK, a)
            })),
            1.0,
        );
        let (one, tiled) = (
            Plan::new(&device, 8192).expect("a shadow"),
            Plan::new(&device, 64).expect("a shadow"),
        );
        let (a, b) = (one.images(&device), tiled.images(&device));
        assert_eq!(a.len(), 1);
        assert!(b.len() > 4 && b.iter().all(|i| i.size[0] <= 64 && i.size[1] <= 64));
        let ids = |n: usize| (0..n as u64).map(egui::TextureId::User).collect::<Vec<_>>();
        let (sa, sb) = (
            one.shape(&ids(a.len()), [0.0; 2], 1.0, 1.0),
            tiled.shape(&ids(b.len()), [0.0; 2], 1.0, 1.0),
        );
        let [ax, ay] = one.axes;
        for j in (ay.lo..ay.hi).step_by(3) {
            for i in (ax.lo..ax.hi).step_by(3) {
                let p = [i as f64 + 0.5, j as f64 + 0.5];
                assert_eq!(sample(&sa, &a, 1.0, p), sample(&sb, &b, 1.0, p), "{p:?}");
            }
        }
        // A box within the shadow shows nothing.
        assert_eq!(sample(&sa, &a, 1.0, [200.5, 150.5]), 0.0);
    }
}
