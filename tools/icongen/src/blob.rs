//! The icon blob record, read by `shadcn_egui_core`'s decoder. All values
//! are little-endian 16-bit words.
//!
//! - Stroke section: `len` (words that follow), then opcodes, each followed by
//!   its coordinates: 0=M(2) 1=L(2) 2=Q(4) 3=C(6) 4=Z(0). Coordinates are in
//!   the 24-unit box, stored as `round(v * 1024)` (i16, so −32..32 units).
//! - Fill section (always present; all zero = no fill): `nv, ni, nc`;
//!   `nv × (x, y)`; `ni × index` (u16); `nc ×` contours `{ len, len × (x, y, mx, my) }`,
//!   miters stored as `round(m * 8192)`, |m| ≤ 2.

use crate::{Converted, Fill, Seg};

pub const COORD_SCALE: f32 = 1024.0;
pub const MITER_SCALE: f32 = 8192.0;

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
/// the 24-unit box; the fill is already there.
pub fn encode(icon: &Converted) -> Result<Vec<u8>, String> {
    let name = icon.name.as_str();
    let scale = 24.0 / icon.size;
    let mut stroke: Vec<u16> = Vec::new();
    for seg in &icon.segs {
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
    let mut w = Vec::with_capacity(stroke.len() + 4);
    w.push(count(name, "stroke words", stroke.len())?);
    w.extend(stroke);
    match &icon.fill {
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
    Ok(w.iter().flat_map(|v| v.to_le_bytes()).collect())
}

/// Bytes of a record's stroke section, including its length word.
pub fn stroke_bytes(record: &[u8]) -> usize {
    2 + 2 * usize::from(u16::from_le_bytes([record[0], record[1]]))
}

/// Reference decoder for tests: one record back into strokes and fill.
pub fn decode(bytes: &[u8]) -> (Vec<Seg>, Option<Fill>) {
    let mut words = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]));
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
