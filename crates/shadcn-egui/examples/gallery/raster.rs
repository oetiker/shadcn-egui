//! A CPU rasterizer for egui's tessellated meshes, with damage tracking.
//!
//! It draws what egui-wgpu draws into a gamma-space (`*8Unorm`) surface,
//! eframe's default: vertex colors and texture samples multiply in gamma
//! space, textures filter in gamma space, the shader dithers, and blending
//! is premultiplied (`One`, `OneMinusSrcAlpha`). Pixels are sampled at their
//! centers with the top-left fill rule on positions snapped to 1/256 pixel,
//! so feathered (anti-aliased) edges come out as on the GPU and the
//! tessellation stays egui's default.

use std::collections::HashMap;

use egui::epaint::{ClippedPrimitive, ImageData, Primitive, Vertex};
use egui::{Color32, TextureFilter, TextureId, TextureOptions, TextureWrapMode, TexturesDelta};

/// Subpixel steps per pixel; positions snap to this grid as on the GPU.
const SUB: i64 = 256;
/// Positions are clamped to this many pixels so the fixed-point edge
/// functions fit in an `i64`.
const MAX_PX: f32 = (1 << 21) as f32;
/// Above this many damage rectangles, one bounding rectangle is cheaper.
const MAX_DAMAGE_RECTS: usize = 24;

/// A pixel rectangle, `x1`/`y1` exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    pub const EMPTY: Self = Self {
        x0: 0,
        y0: 0,
        x1: 0,
        y1: 0,
    };

    pub fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }

    pub fn intersect(&self, o: &Self) -> Self {
        Self {
            x0: self.x0.max(o.x0),
            y0: self.y0.max(o.y0),
            x1: self.x1.min(o.x1),
            y1: self.y1.min(o.y1),
        }
    }

    pub fn union(&self, o: &Self) -> Self {
        if self.is_empty() {
            return *o;
        }
        if o.is_empty() {
            return *self;
        }
        Self {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }

    fn area(&self) -> i64 {
        if self.is_empty() {
            0
        } else {
            i64::from(self.x1 - self.x0) * i64::from(self.y1 - self.y0)
        }
    }
}

/// The frame the rasterizer draws into, one `0x00RRGGBB` word per pixel as
/// softbuffer takes it.
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width * height],
        }
    }

    pub fn rect(&self) -> IRect {
        IRect {
            x0: 0,
            y0: 0,
            x1: self.width as i32,
            y1: self.height as i32,
        }
    }

    fn fill(&mut self, r: IRect, color: u32) {
        for y in r.y0..r.y1 {
            let row = y as usize * self.width;
            self.pixels[row + r.x0 as usize..row + r.x1 as usize].fill(color);
        }
    }
}

struct Texture {
    size: [usize; 2],
    pixels: Vec<Color32>,
    options: TextureOptions,
}

/// One triangle of last frame's meshes: what it draws (hash) and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TriKey {
    hash: u64,
    bbox: IRect,
}

/// Draws egui frames into a [`Canvas`], re-rasterizing only where the
/// triangles changed since the previous frame.
#[derive(Default)]
pub struct Rasterizer {
    textures: HashMap<TextureId, Texture>,
    prev: Vec<TriKey>,
    prev_size: (usize, usize),
    full_damage: bool,
    keys: Vec<TriKey>,
}

impl Rasterizer {
    /// Applies the new and changed textures of a frame; call before
    /// [`Self::render`]. A replaced texture damages the whole frame.
    pub fn set_textures(&mut self, delta: &TexturesDelta) {
        // egui 0.37 sends a list of deltas per texture, in order.
        for (id, image_deltas) in &delta.set {
            for image_delta in image_deltas {
                let ImageData::Color(image) = &image_delta.image;
                let [w, h] = image.size;
                match image_delta.pos {
                    None => {
                        self.textures.insert(
                            *id,
                            Texture {
                                size: [w, h],
                                pixels: image.pixels.clone(),
                                options: image_delta.options,
                            },
                        );
                        self.full_damage = true;
                    }
                    Some([x, y]) => {
                        let Some(tex) = self.textures.get_mut(id) else {
                            continue;
                        };
                        for row in 0..h {
                            let dst = (y + row) * tex.size[0] + x;
                            tex.pixels[dst..dst + w].copy_from_slice(&image.pixels[row * w..][..w]);
                        }
                        // The font atlas only grows into unused space; other
                        // textures may change under triangles that did not.
                        if *id != TextureId::default() {
                            self.full_damage = true;
                        }
                    }
                }
            }
        }
    }

    /// Drops the textures egui freed; call after [`Self::render`].
    pub fn free_textures(&mut self, delta: &TexturesDelta) {
        for id in &delta.free {
            // A mesh may still name the texture next frame; egui-wgpu then
            // skips it, and its old pixels must go. Frees are rare.
            if self.textures.remove(id).is_some() {
                self.full_damage = true;
            }
        }
    }

    /// Draws `primitives` into `canvas` where they differ from the last
    /// frame and returns the rectangles it redrew (empty when nothing
    /// changed). `clear` is the color under everything.
    pub fn render(
        &mut self,
        canvas: &mut Canvas,
        primitives: &[ClippedPrimitive],
        pixels_per_point: f32,
        clear: u32,
    ) -> Vec<IRect> {
        let frame = canvas.rect();
        let mut keys = std::mem::take(&mut self.keys);
        keys.clear();
        for prim in primitives {
            let Primitive::Mesh(mesh) = &prim.primitive else {
                continue;
            };
            if !self.textures.contains_key(&mesh.texture_id) {
                // Once per mesh and frame, as egui-wgpu warns.
                log::warn!("Missing texture: {:?}", mesh.texture_id);
            }
            let scissor = scissor_rect(prim.clip_rect, pixels_per_point, frame);
            let base = hash_words(
                0x9e37_79b9,
                &[
                    scissor.x0 as u32,
                    scissor.y0 as u32,
                    scissor.x1 as u32,
                    scissor.y1 as u32,
                    texture_id_word(mesh.texture_id) as u32,
                    (texture_id_word(mesh.texture_id) >> 32) as u32,
                ],
            );
            for tri in mesh.indices.as_chunks::<3>().0.iter() {
                let v = [
                    &mesh.vertices[tri[0] as usize],
                    &mesh.vertices[tri[1] as usize],
                    &mesh.vertices[tri[2] as usize],
                ];
                keys.push(TriKey {
                    hash: hash_vertices(base, v),
                    bbox: triangle_bbox(v, pixels_per_point).intersect(&scissor),
                });
            }
        }

        let size = (canvas.width, canvas.height);
        let damage = if self.full_damage || size != self.prev_size {
            vec![frame]
        } else {
            coalesce(diff_damage(&self.prev, &keys), frame)
        };
        self.full_damage = false;
        self.prev_size = size;

        for rect in &damage {
            canvas.fill(*rect, clear);
            let mut k = 0;
            for prim in primitives {
                let Primitive::Mesh(mesh) = &prim.primitive else {
                    continue;
                };
                let n = mesh.indices.len() / 3;
                let tris = &keys[k..k + n];
                k += n;
                let scissor = scissor_rect(prim.clip_rect, pixels_per_point, frame);
                let clip = scissor.intersect(rect);
                if clip.is_empty() {
                    continue;
                }
                // egui-wgpu skips a mesh whose texture it lacks.
                let Some(texture) = self.textures.get(&mesh.texture_id) else {
                    continue;
                };
                for (tri, key) in mesh.indices.as_chunks::<3>().0.iter().zip(tris) {
                    if key.bbox.intersect(&clip).is_empty() {
                        continue;
                    }
                    let v = [
                        &mesh.vertices[tri[0] as usize],
                        &mesh.vertices[tri[1] as usize],
                        &mesh.vertices[tri[2] as usize],
                    ];
                    raster_triangle(canvas, clip, v, pixels_per_point, texture);
                }
            }
        }

        self.keys = std::mem::replace(&mut self.prev, keys);
        damage
    }
}

/// The scissor rectangle egui-wgpu sets for a clip rectangle.
fn scissor_rect(clip: egui::Rect, ppp: f32, frame: IRect) -> IRect {
    let round = |v: f32, max: i32| ((v * ppp).round().max(0.0) as i64).min(i64::from(max)) as i32;
    let x0 = round(clip.min.x, frame.x1);
    let y0 = round(clip.min.y, frame.y1);
    IRect {
        x0,
        y0,
        x1: round(clip.max.x, frame.x1).max(x0),
        y1: round(clip.max.y, frame.y1).max(y0),
    }
}

fn texture_id_word(id: TextureId) -> u64 {
    match id {
        TextureId::Managed(n) => n << 1,
        TextureId::User(n) => (n << 1) | 1,
    }
}

fn hash_words(mut h: u64, words: &[u32]) -> u64 {
    for &w in words {
        h = (h.rotate_left(5) ^ u64::from(w)).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
    h
}

fn hash_vertices(base: u64, v: [&Vertex; 3]) -> u64 {
    let mut words = [0u32; 15];
    for (i, vert) in v.iter().enumerate() {
        words[i * 5..i * 5 + 5].copy_from_slice(&[
            vert.pos.x.to_bits(),
            vert.pos.y.to_bits(),
            vert.uv.x.to_bits(),
            vert.uv.y.to_bits(),
            u32::from_le_bytes(vert.color.to_array()),
        ]);
    }
    hash_words(base, &words)
}

/// Every pixel whose center the triangle may cover, with a pixel to spare.
fn triangle_bbox(v: [&Vertex; 3], ppp: f32) -> IRect {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for vert in v {
        let (x, y) = (vert.pos.x * ppp, vert.pos.y * ppp);
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let lo = |v: f32| (v.floor().clamp(-MAX_PX, MAX_PX) as i32) - 1;
    let hi = |v: f32| (v.ceil().clamp(-MAX_PX, MAX_PX) as i32) + 1;
    if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
        return IRect::EMPTY;
    }
    IRect {
        x0: lo(x0),
        y0: lo(y0),
        x1: hi(x1),
        y1: hi(y1),
    }
}

/// The areas where `new` draws something other than `old`: the boxes of
/// the triangles outside a common subsequence of both. Elsewhere every
/// pixel is covered by the same triangles in the same order, so it does
/// not change.
fn diff_damage(old: &[TriKey], new: &[TriKey]) -> Vec<IRect> {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let old_mid = &old[prefix..old.len() - suffix];
    let new_mid = &new[prefix..new.len() - suffix];
    let mut damage = Vec::new();
    if old_mid.is_empty() || new_mid.is_empty() {
        damage.extend(old_mid.iter().chain(new_mid).map(|k| k.bbox));
        return damage;
    }
    // Greedy in-order matching: each new triangle takes the first unused
    // equal old triangle after the last match.
    let mut by_key: HashMap<TriKey, (Vec<usize>, usize)> = HashMap::new();
    for (i, k) in old_mid.iter().enumerate() {
        by_key.entry(*k).or_default().0.push(i);
    }
    let mut matched = vec![false; old_mid.len()];
    let mut last: Option<usize> = None;
    for k in new_mid {
        let found = by_key.get_mut(k).and_then(|(idx, cursor)| {
            while *cursor < idx.len() && last.is_some_and(|l| idx[*cursor] <= l) {
                *cursor += 1;
            }
            idx.get(*cursor).copied()
        });
        match found {
            Some(i) => {
                matched[i] = true;
                last = Some(i);
            }
            None => damage.push(k.bbox),
        }
    }
    damage.extend(
        old_mid
            .iter()
            .zip(&matched)
            .filter(|(_, m)| !**m)
            .map(|(k, _)| k.bbox),
    );
    damage
}

impl std::hash::Hash for TriKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

/// Merges damage boxes that touch or nearly touch into fewer, larger
/// rectangles, clipped to `frame`.
fn coalesce(rects: Vec<IRect>, frame: IRect) -> Vec<IRect> {
    const SLACK: i32 = 8;
    let mut out: Vec<IRect> = Vec::new();
    for r in rects {
        let mut r = r.intersect(&frame);
        if r.is_empty() {
            continue;
        }
        loop {
            let grown = IRect {
                x0: r.x0 - SLACK,
                y0: r.y0 - SLACK,
                x1: r.x1 + SLACK,
                y1: r.y1 + SLACK,
            };
            let Some(i) = out.iter().position(|o| !o.intersect(&grown).is_empty()) else {
                break;
            };
            r = r.union(&out.swap_remove(i));
        }
        out.push(r);
        if out.len() > MAX_DAMAGE_RECTS {
            let all = out.iter().fold(IRect::EMPTY, |a, b| a.union(b));
            out = vec![all];
        }
    }
    // A few big rectangles that overlap a lot cost more than their union.
    let total: i64 = out.iter().map(IRect::area).sum();
    let all = out.iter().fold(IRect::EMPTY, |a, b| a.union(b));
    if out.len() > 1 && total * 4 >= all.area() * 3 {
        out = vec![all];
    }
    out
}

/// egui-wgpu's dither offset for the pixel at (`x`, `y`), in 0-1 units
/// (interleaved gradient noise, scaled so flat colors stay flat).
fn dither(x: i32, y: i32) -> f32 {
    let fract = |v: f32| v - v.floor();
    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
    let n = fract(52.982_918 * fract(0.067_110_56 * fx + 0.005_837_15 * fy));
    (n - 0.5) * 0.95 / 255.0
}

fn to_unorm(v: f32) -> u32 {
    (v * 255.0).round().clamp(0.0, 255.0) as u32
}

/// Blends premultiplied `src` (0-1, gamma) over the pixel, dithered as
/// egui-wgpu does.
#[inline]
fn blend(dst: &mut u32, src: [f32; 4], x: i32, y: i32) {
    let n = dither(x, y);
    let inv = 1.0 - src[3];
    let d = *dst;
    let ch = |shift: u32, s: f32| {
        let dv = ((d >> shift) & 0xff) as f32 / 255.0;
        to_unorm(s + n + dv * inv) << shift
    };
    *dst = ch(16, src[0]) | ch(8, src[1]) | ch(0, src[2]);
}

fn rgba(c: Color32) -> [f32; 4] {
    let [r, g, b, a] = c.to_array();
    [r, g, b, a].map(|v| f32::from(v) / 255.0)
}

impl Texture {
    /// What the GPU sampler returns at `uv`, 0-1 per channel.
    fn sample(&self, uv: [f32; 2], filter: TextureFilter) -> [f32; 4] {
        let [w, h] = self.size;
        let wrap = |i: i64, n: usize| -> usize {
            let n = n as i64;
            (match self.options.wrap_mode {
                TextureWrapMode::ClampToEdge => i.clamp(0, n - 1),
                TextureWrapMode::Repeat => i.rem_euclid(n),
                TextureWrapMode::MirroredRepeat => {
                    let m = i.rem_euclid(2 * n);
                    if m < n {
                        m
                    } else {
                        2 * n - 1 - m
                    }
                }
            }) as usize
        };
        let texel = |x: i64, y: i64| rgba(self.pixels[wrap(y, h) * w + wrap(x, w)]);
        let (u, v) = (uv[0] * w as f32, uv[1] * h as f32);
        match filter {
            TextureFilter::Nearest => texel(u.floor() as i64, v.floor() as i64),
            TextureFilter::Linear => {
                let (u, v) = (u - 0.5, v - 0.5);
                let (x, y) = (u.floor(), v.floor());
                let (fx, fy) = (u - x, v - y);
                let (x, y) = (x as i64, y as i64);
                let t00 = texel(x, y);
                if fx == 0.0 && fy == 0.0 {
                    return t00;
                }
                let (t10, t01, t11) = (texel(x + 1, y), texel(x, y + 1), texel(x + 1, y + 1));
                std::array::from_fn(|i| {
                    let top = t00[i] + (t10[i] - t00[i]) * fx;
                    let bottom = t01[i] + (t11[i] - t01[i]) * fx;
                    top + (bottom - top) * fy
                })
            }
        }
    }
}

/// One edge function `E(p) = dx (p.y - a.y) - dy (p.x - a.x)` in
/// 1/`SUB` pixel units, set up for row-by-row evaluation.
struct Edge {
    dx: i64,
    dy: i64,
    ax: i64,
    ay: i64,
    /// 0 on a top or left edge (pixels on it are inside), 1 otherwise.
    bias: i64,
}

impl Edge {
    fn new(a: (i64, i64), b: (i64, i64)) -> Self {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let top_left = (dy == 0 && dx > 0) || dy < 0;
        Self {
            dx,
            dy,
            ax: a.0,
            ay: a.1,
            bias: i64::from(!top_left),
        }
    }

    /// E at the center of pixel (0, `y`); E grows by `step()` per pixel in x.
    fn row_start(&self, y: i32) -> i64 {
        let py = i64::from(y) * SUB + SUB / 2;
        self.dx * (py - self.ay) - self.dy * (SUB / 2 - self.ax)
    }

    fn step(&self) -> i64 {
        -self.dy * SUB
    }

    /// Narrows `[lo, hi]` to the pixels of the row inside this edge.
    fn clip_span(&self, c: i64, lo: &mut i64, hi: &mut i64) {
        let s = self.step();
        if s > 0 {
            *lo = (*lo)
                .max((self.bias - c).div_euclid(s) + i64::from((self.bias - c).rem_euclid(s) != 0));
        } else if s < 0 {
            *hi = (*hi).min((c - self.bias).div_euclid(-s));
        } else if c < self.bias {
            *hi = *lo - 1;
        }
    }
}

fn snap(v: f32, ppp: f32) -> i64 {
    ((v * ppp).clamp(-MAX_PX, MAX_PX) * SUB as f32).round() as i64
}

fn raster_triangle(canvas: &mut Canvas, clip: IRect, v: [&Vertex; 3], ppp: f32, texture: &Texture) {
    let mut p = v.map(|v| (snap(v.pos.x, ppp), snap(v.pos.y, ppp)));
    let mut v = v;
    let orient = |p: &[(i64, i64); 3]| {
        (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[1].1 - p[0].1) * (p[2].0 - p[0].0)
    };
    let mut area = orient(&p);
    if area == 0 {
        return;
    }
    if area < 0 {
        p.swap(1, 2);
        v.swap(1, 2);
        area = -area;
    }
    // The weight of vertex i is edge i over the area.
    let edges = [
        Edge::new(p[1], p[2]),
        Edge::new(p[2], p[0]),
        Edge::new(p[0], p[1]),
    ];
    let min_y = p.iter().map(|q| q.1).min().unwrap_or(0).div_euclid(SUB) as i32 - 1;
    let max_y = p.iter().map(|q| q.1).max().unwrap_or(0).div_euclid(SUB) as i32 + 1;
    let min_x = p.iter().map(|q| q.0).min().unwrap_or(0).div_euclid(SUB) - 1;
    let max_x = p.iter().map(|q| q.0).max().unwrap_or(0).div_euclid(SUB) + 1;
    let y0 = clip.y0.max(min_y);
    let y1 = clip.y1.min(max_y + 1);

    let colors = v.map(|v| rgba(v.color));
    let uvs = v.map(|v| [v.uv.x, v.uv.y]);
    let flat_color = v[0].color == v[1].color && v[1].color == v[2].color;
    let flat_uv = uvs[0] == uvs[1] && uvs[1] == uvs[2];
    let filter = texture_filter(texture, &uvs, area);
    let sample = |uv: [f32; 2]| texture.sample(uv, filter);

    // A constant source color: fill opaque spans, blend translucent ones.
    let flat = (flat_color && flat_uv).then(|| {
        let t = sample(uvs[0]);
        let src: [f32; 4] = std::array::from_fn(|i| colors[0][i] * t[i]);
        let opaque = (t == [1.0; 4] && v[0].color.a() == 255).then(|| {
            (u32::from(v[0].color.r()) << 16)
                | (u32::from(v[0].color.g()) << 8)
                | u32::from(v[0].color.b())
        });
        (src, opaque)
    });
    let inv_area = 1.0 / area as f64;

    for y in y0..y1 {
        let c = edges.each_ref().map(|e| e.row_start(y));
        let mut lo = i64::from(clip.x0).max(min_x);
        let mut hi = i64::from(clip.x1 - 1).min(max_x);
        for (e, &c) in edges.iter().zip(&c) {
            e.clip_span(c, &mut lo, &mut hi);
        }
        if lo > hi {
            continue;
        }
        let row = y as usize * canvas.width;
        let span = &mut canvas.pixels[row + lo as usize..=row + hi as usize];
        match flat {
            Some((_, Some(opaque))) => span.fill(opaque),
            Some((src, None)) => {
                for (i, px) in span.iter_mut().enumerate() {
                    blend(px, src, lo as i32 + i as i32, y);
                }
            }
            None => {
                for (i, px) in span.iter_mut().enumerate() {
                    let x = lo + i as i64;
                    let w =
                        [0, 1, 2].map(|k| ((c[k] + edges[k].step() * x) as f64 * inv_area) as f32);
                    let color = if flat_color {
                        colors[0]
                    } else {
                        std::array::from_fn(|ch| {
                            w[0] * colors[0][ch] + w[1] * colors[1][ch] + w[2] * colors[2][ch]
                        })
                    };
                    let t = if flat_uv {
                        sample(uvs[0])
                    } else {
                        sample(std::array::from_fn(|ch| {
                            w[0] * uvs[0][ch] + w[1] * uvs[1][ch] + w[2] * uvs[2][ch]
                        }))
                    };
                    let src = std::array::from_fn(|ch| color[ch] * t[ch]);
                    blend(px, src, x as i32, y);
                }
            }
        }
    }
}

/// The filter the GPU picks for this triangle: magnification when a
/// texel covers at least a pixel, minification otherwise.
fn texture_filter(t: &Texture, uvs: &[[f32; 2]; 3], area: i64) -> TextureFilter {
    let o = t.options;
    if o.magnification == o.minification {
        return o.magnification;
    }
    let tex_area = ((uvs[1][0] - uvs[0][0]) * (uvs[2][1] - uvs[0][1])
        - (uvs[1][1] - uvs[0][1]) * (uvs[2][0] - uvs[0][0]))
        .abs()
        * (t.size[0] * t.size[1]) as f32;
    let px_area = area as f32 / (SUB * SUB) as f32;
    if tex_area <= px_area {
        o.magnification
    } else {
        o.minification
    }
}

#[cfg(test)]
impl Rasterizer {
    /// A rasterizer holding a 1x1 white font atlas, as egui uploads one
    /// before the first frame: meshes with the default texture draw in
    /// their vertex colours.
    fn with_white_atlas() -> Self {
        let mut r = Self::default();
        let mut delta = TexturesDelta::default();
        delta.push(
            TextureId::default(),
            egui::epaint::ImageDelta::full(
                egui::ColorImage::filled([1, 1], Color32::WHITE),
                TextureOptions::LINEAR,
            ),
        );
        r.set_textures(&delta);
        delta.clear();
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::epaint::Mesh;
    use egui::{pos2, Rect};

    fn prim(mesh: Mesh) -> ClippedPrimitive {
        ClippedPrimitive {
            clip_rect: Rect::EVERYTHING,
            primitive: Primitive::Mesh(mesh),
        }
    }

    fn rect_mesh(r: Rect, color: Color32) -> Mesh {
        let mut mesh = Mesh::default();
        mesh.add_colored_rect(r, color);
        mesh
    }

    fn render(r: &mut Rasterizer, c: &mut Canvas, prims: &[ClippedPrimitive]) -> Vec<IRect> {
        r.render(c, prims, 1.0, 0x00ff_ffff)
    }

    #[test]
    fn a_mesh_whose_texture_is_missing_draws_nothing() {
        // egui-wgpu skips the draw of a mesh whose texture it does not
        // have; plain vertex colour would show a solid box instead.
        let mut c = Canvas::new(8, 8);
        let mut r = Rasterizer::with_white_atlas();
        let mut mesh = rect_mesh(
            Rect::from_min_max(pos2(2.0, 2.0), pos2(6.0, 6.0)),
            Color32::RED,
        );
        mesh.texture_id = TextureId::Managed(7);
        render(&mut r, &mut c, &[prim(mesh)]);
        assert!(
            c.pixels.iter().all(|&p| p == 0x00ff_ffff),
            "{:x?}",
            c.pixels
        );
    }

    #[test]
    fn a_freed_texture_still_in_use_leaves_no_stale_pixels() {
        // egui-wgpu draws nothing for a mesh whose texture was freed, so
        // the damage render must clear the pixels it drew before, as a
        // full render of the same frame shows.
        let id = TextureId::Managed(9);
        let mut mesh = Mesh::with_texture(id);
        mesh.add_rect_with_uv(
            Rect::from_min_max(pos2(0.0, 0.0), pos2(4.0, 4.0)),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let mut red = TexturesDelta::default();
        red.push(
            id,
            egui::epaint::ImageDelta::full(
                egui::ColorImage::filled([2, 2], Color32::RED),
                TextureOptions::NEAREST,
            ),
        );
        let mut r = Rasterizer::with_white_atlas();
        let mut c = Canvas::new(4, 4);
        r.set_textures(&red);
        red.clear();
        render(&mut r, &mut c, &[prim(mesh.clone())]);
        assert!(
            c.pixels.iter().all(|&p| p == 0x00ff_0000),
            "{:x?}",
            c.pixels
        );
        let mut freed = TexturesDelta::default();
        freed.free(id);
        r.free_textures(&freed);
        freed.clear();
        render(&mut r, &mut c, &[prim(mesh.clone())]);
        let mut full = Canvas::new(4, 4);
        render(
            &mut Rasterizer::with_white_atlas(),
            &mut full,
            &[prim(mesh)],
        );
        assert_eq!(c.pixels, full.pixels, "stale pixels of a freed texture");
    }

    /// A `NEAREST` texture drawn 1:1 shows each texel at its pixel, as
    /// the GPU does; the box shadows rely on it (#25).
    #[test]
    fn a_nearest_texture_drawn_one_to_one_shows_its_texels() {
        let id = TextureId::Managed(9);
        let texels = [
            Color32::RED,
            Color32::GREEN,
            Color32::BLUE,
            Color32::from_rgb(10, 20, 30),
        ];
        let mut mesh = Mesh::with_texture(id);
        mesh.add_rect_with_uv(
            Rect::from_min_max(pos2(0.0, 0.0), pos2(4.0, 1.0)),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let mut delta = TexturesDelta::default();
        delta.push(
            id,
            egui::epaint::ImageDelta::full(
                egui::ColorImage::new([4, 1], texels.to_vec()),
                TextureOptions::NEAREST,
            ),
        );
        let mut r = Rasterizer::with_white_atlas();
        let mut c = Canvas::new(4, 1);
        r.set_textures(&delta);
        delta.clear();
        render(&mut r, &mut c, &[prim(mesh)]);
        let want: Vec<u32> = texels
            .iter()
            .map(|t| u32::from_be_bytes([0, t.r(), t.g(), t.b()]))
            .collect();
        assert_eq!(c.pixels, want);
    }

    #[test]
    fn an_opaque_rect_covers_exactly_its_pixels() {
        let mut c = Canvas::new(8, 8);
        let mut r = Rasterizer::with_white_atlas();
        let mesh = rect_mesh(
            Rect::from_min_max(pos2(2.0, 3.0), pos2(5.0, 4.0)),
            Color32::RED,
        );
        render(&mut r, &mut c, &[prim(mesh)]);
        for y in 0..8 {
            for x in 0..8 {
                let inside = (2..5).contains(&x) && y == 3;
                let want = if inside { 0x00ff_0000 } else { 0x00ff_ffff };
                assert_eq!(c.pixels[y * 8 + x], want, "({x}, {y})");
            }
        }
    }

    #[test]
    fn a_shared_edge_blends_each_pixel_once() {
        // Two triangles of a translucent quad: a pixel center on the
        // diagonal belongs to one of them only (top-left rule).
        let mut c = Canvas::new(8, 8);
        let mut r = Rasterizer::with_white_atlas();
        let half = Color32::from_black_alpha(128);
        let mesh = rect_mesh(Rect::from_min_max(pos2(1.0, 1.0), pos2(6.0, 6.0)), half);
        render(&mut r, &mut c, &[prim(mesh)]);
        let first = c.pixels[8 + 1];
        for y in 1..6 {
            for x in 1..6 {
                assert_eq!(c.pixels[y * 8 + x], first, "({x}, {y})");
            }
        }
        assert_eq!(c.pixels[0], 0x00ff_ffff);
    }

    #[test]
    fn an_unchanged_frame_redraws_nothing_and_a_change_only_its_box() {
        let mut c = Canvas::new(64, 64);
        let mut r = Rasterizer::with_white_atlas();
        let a = rect_mesh(
            Rect::from_min_max(pos2(0.0, 0.0), pos2(64.0, 64.0)),
            Color32::GRAY,
        );
        let b = rect_mesh(
            Rect::from_min_max(pos2(10.0, 10.0), pos2(20.0, 20.0)),
            Color32::RED,
        );
        let frame = [prim(a.clone()), prim(b)];
        assert_eq!(render(&mut r, &mut c, &frame), vec![c.rect()]);
        assert!(render(&mut r, &mut c, &frame).is_empty());
        let moved = rect_mesh(
            Rect::from_min_max(pos2(12.0, 10.0), pos2(22.0, 20.0)),
            Color32::RED,
        );
        let damage = render(&mut r, &mut c, &[prim(a.clone()), prim(moved.clone())]);
        assert_eq!(damage.len(), 1);
        assert!(
            damage[0].x0 <= 10 && damage[0].x1 >= 22 && damage[0].x1 < 40,
            "{damage:?}"
        );

        // Drawing the damage only gives the same pixels as a full redraw.
        let mut full = Canvas::new(64, 64);
        Rasterizer::with_white_atlas().render(&mut full, &[prim(a), prim(moved)], 1.0, 0x00ff_ffff);
        assert_eq!(c.pixels, full.pixels);
    }

    #[test]
    fn the_dither_never_moves_a_flat_color() {
        for y in 0..64 {
            for x in 0..64 {
                let n = dither(x, y) * 255.0;
                assert!(n.abs() < 0.5, "{n}");
            }
        }
    }

    #[test]
    fn scissors_round_like_egui_wgpu() {
        let frame = IRect {
            x0: 0,
            y0: 0,
            x1: 100,
            y1: 100,
        };
        let s = scissor_rect(
            Rect::from_min_max(pos2(1.4, 1.6), pos2(50.5, 200.0)),
            2.0,
            frame,
        );
        assert_eq!(
            s,
            IRect {
                x0: 3,
                y0: 3,
                x1: 100,
                y1: 100
            }
        );
    }
}

#[cfg(test)]
#[path = "raster_checks.rs"]
mod checks;
