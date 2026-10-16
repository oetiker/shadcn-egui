//! The icon blob record, read by `shadcn_egui_core`'s decoder. All values
//! are little-endian 16-bit words. A record is one or more layers; the first
//! is opaque, every further one starts with its alpha word.
//!
//! - Alpha (not in the first layer): `round(a * 65535)`, 0 < a < 1.
//! - Stroke section: `len` (words that follow), then opcodes, each followed by
//!   its coordinates: 0=M(2) 1=L(2) 2=Q(4) 3=C(6) 4=Z(0). Coordinates are in
//!   the 24-unit box, stored as `round(v * 1024)` (i16, so −32..32 units).
//! - Fill section (always present; all zero = no fill): `nv, ni, nc`;
//!   `nv × (x, y)`; `ni × index` (u16); `nc ×` contours `{ len, len × (x, y, mx, my) }`,
//!   miters stored as `round(m * 8192)`, |m| ≤ 2.
//!
//! An icon without translucent parts is one layer, so its record ends after
//! the first fill section.

use crate::{Converted, Fill, Layer, Seg};

pub const COORD_SCALE: f32 = 1024.0;
pub const MITER_SCALE: f32 = 8192.0;
pub const ALPHA_SCALE: f32 = 65535.0;

/// `v` rounded to the stored precision; the decoder returns exactly this value.
pub fn quantise(v: f32) -> f32 {
    (v * COORD_SCALE).round() / COORD_SCALE
}

fn coord(name: &str, v: f32) -> Result<u16, String> {
    let q = (v * COORD_SCALE).round();
    if !(f32::from(i16::MIN)..=f32::from(i16::MAX)).contains(&q) {
        return Err(format!("{name}: coordinate {v} outside -32..32 icon units"));
    }
    Ok(q as i16 as u16)
}

fn miter(m: f32) -> u16 {
    (m.clamp(-2.0, 2.0) * MITER_SCALE).round() as i16 as u16
}

fn count(name: &str, what: &str, n: usize) -> Result<u16, String> {
    u16::try_from(n).map_err(|_| format!("{name}: {n} {what}, at most 65535"))
}

/// Encode one icon. Stroke coordinates are scaled from the SVG's units into
/// the 24-unit box; the fills are already there.
pub fn encode(icon: &Converted) -> Result<Vec<u8>, String> {
    let name = icon.name.as_str();
    let scale = 24.0 / icon.size;
    let mut w = Vec::new();
    encode_layer(name, scale, &icon.segs, icon.fill.as_ref(), &mut w)?;
    for layer in &icon.layers {
        w.push(alpha(name, layer.alpha)?);
        encode_layer(name, scale, &layer.segs, layer.fill.as_ref(), &mut w)?;
    }
    Ok(w.iter().flat_map(|v| v.to_le_bytes()).collect())
}

fn alpha(name: &str, a: f32) -> Result<u16, String> {
    let q = (a * ALPHA_SCALE).round();
    // icongen merges a full alpha into the opaque layer and drops a zero one.
    if !(1.0..ALPHA_SCALE).contains(&q) {
        return Err(format!(
            "{name}: translucent layer alpha {a} must lie strictly between 0 and 1 (in steps of 1/65535)"
        ));
    }
    Ok(q as u16)
}

fn encode_layer(
    name: &str,
    scale: f32,
    segs: &[Seg],
    fill: Option<&Fill>,
    w: &mut Vec<u16>,
) -> Result<(), String> {
    let mut stroke: Vec<u16> = Vec::new();
    for seg in segs {
        let (op, coords) = match *seg {
            Seg::M(x, y) => (0, vec![x, y]),
            Seg::L(x, y) => (1, vec![x, y]),
            Seg::Q(a, b, x, y) => (2, vec![a, b, x, y]),
            Seg::C(a, b, c, d, x, y) => (3, vec![a, b, c, d, x, y]),
            Seg::Z => (4, vec![]),
        };
        stroke.push(op);
        for v in coords {
            stroke.push(coord(name, v * scale)?);
        }
    }
    w.push(count(name, "stroke words", stroke.len())?);
    w.extend(stroke);
    match fill {
        None => w.extend([0, 0, 0]),
        Some(f) => {
            w.push(count(name, "fill vertices", f.vertices.len())?);
            w.push(count(name, "fill indices", f.indices.len())?);
            w.push(count(name, "contours", f.contours.len())?);
            for v in &f.vertices {
                w.push(coord(name, v[0])?);
                w.push(coord(name, v[1])?);
            }
            w.extend(&f.indices);
            for c in &f.contours {
                w.push(count(name, "contour points", c.len())?);
                for (p, m) in c {
                    w.push(coord(name, p[0])?);
                    w.push(coord(name, p[1])?);
                    w.push(miter(m[0]));
                    w.push(miter(m[1]));
                }
            }
        }
    }
    Ok(())
}

fn words(bytes: &[u8]) -> Vec<u16> {
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| u16::from_le_bytes(c))
        .collect()
}

/// Bytes of a record's stroke sections and alpha words, all layers.
pub fn stroke_bytes(record: &[u8]) -> usize {
    let w = words(record);
    let (mut pos, mut stroke) = (0, 0);
    while pos < w.len() {
        let start = pos;
        if pos > 0 {
            pos += 1; // alpha
        }
        pos += 1 + usize::from(w[pos]);
        stroke += pos - start;
        let (nv, ni, nc) = (
            usize::from(w[pos]),
            usize::from(w[pos + 1]),
            usize::from(w[pos + 2]),
        );
        pos += 3 + 2 * nv + ni;
        for _ in 0..nc {
            pos += 1 + 4 * usize::from(w[pos]);
        }
    }
    2 * stroke
}

/// Reference decoder for tests: one record back into its opaque strokes and
/// fill and its translucent layers.
pub fn decode(bytes: &[u8]) -> (Vec<Seg>, Option<Fill>, Vec<Layer>) {
    let words = words(bytes);
    let mut words = words.into_iter();
    let (segs, fill) = decode_layer(&mut words);
    let mut layers = Vec::new();
    while let Some(a) = words.next() {
        let (segs, fill) = decode_layer(&mut words);
        layers.push(Layer {
            alpha: f32::from(a) / ALPHA_SCALE,
            segs,
            fill,
        });
    }
    (segs, fill, layers)
}

fn decode_layer(words: &mut impl Iterator<Item = u16>) -> (Vec<Seg>, Option<Fill>) {
    let mut next = || words.next().expect("truncated record");
    let c = |w: u16| f32::from(w as i16) / COORD_SCALE;
    let m = |w: u16| f32::from(w as i16) / MITER_SCALE;
    let stroke_len = usize::from(next());
    let (mut segs, mut used) = (Vec::new(), 0);
    while used < stroke_len {
        let op = next();
        let n = [2, 2, 4, 6, 0][usize::from(op)];
        let v: Vec<f32> = (0..n).map(|_| c(next())).collect();
        used += 1 + n;
        segs.push(match op {
            0 => Seg::M(v[0], v[1]),
            1 => Seg::L(v[0], v[1]),
            2 => Seg::Q(v[0], v[1], v[2], v[3]),
            3 => Seg::C(v[0], v[1], v[2], v[3], v[4], v[5]),
            _ => Seg::Z,
        });
    }
    let (nv, ni, nc) = (
        usize::from(next()),
        usize::from(next()),
        usize::from(next()),
    );
    if nv == 0 && nc == 0 {
        return (segs, None);
    }
    let vertices = (0..nv).map(|_| [c(next()), c(next())]).collect();
    let indices = (0..ni).map(|_| next()).collect();
    let contours = (0..nc)
        .map(|_| {
            let len = usize::from(next());
            (0..len)
                .map(|_| ([c(next()), c(next())], [m(next()), m(next())]))
                .collect()
        })
        .collect();
    (
        segs,
        Some(Fill {
            vertices,
            indices,
            contours,
        }),
    )
}
