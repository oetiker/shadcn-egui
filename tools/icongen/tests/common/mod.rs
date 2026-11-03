//! The measures of the precision tests. A shipped fill is compared with the
//! exact area coverage of the upstream geometry (`exact`, which shares no
//! code with icongen's fill module or the crate):
//!
//! - raster: the crate's raster (this module includes
//!   `crates/shadcn-egui/src/components/icon_raster/fill.rs`, the file the
//!   crate paints fills with) of the shipped blob, on every pixel. This is
//!   what a user sees, seams between abutting shapes included; the tests
//!   check it.
//! - geometry: `exact` on the shipped triangle mesh, so only flattening,
//!   rounding, welding and tessellation differ. Reported, to tell a
//!   geometry error from a raster error.
//! - rounding: the crate's raster against `exact` on the same mesh, the
//!   raster's own error (f32 rounding). Reported.
#![allow(dead_code)]

pub mod exact;

use icongen::{fill, fill_paths, Fill, FillPath, Precision};

#[path = "../../../../crates/shadcn-egui/src/components/icon_raster/fill.rs"]
mod raster;

/// The levels of 255 a fill may differ from the exact one.
pub const LEVELS: u8 = 4;
pub const SIDES: [usize; 10] = [16, 20, 24, 32, 48, 64, 96, 128, 192, 256];
/// The upstream geometry: flattened finely, not rounded.
pub const REFERENCE: Precision = Precision {
    tolerance: 1e-4,
    quantise: false,
};

/// Worst differences of one fill at one size, in levels of 255.
#[derive(Clone, Copy, Debug, Default)]
pub struct Measure {
    pub raster: u8,
    pub geometry: u8,
    pub rounding: u8,
}

impl Measure {
    /// The worse of `self` and `m`, measure by measure.
    pub fn max(self, m: Measure) -> Measure {
        Measure {
            raster: self.raster.max(m.raster),
            geometry: self.geometry.max(m.geometry),
            rounding: self.rounding.max(m.rounding),
        }
    }
}

/// The shipped fills of an icon at `prec`, each with the upstream geometry
/// of its layer.
pub fn layers(
    name: &str,
    svg: &str,
    prec: Precision,
) -> Result<Vec<(Fill, Vec<FillPath>)>, String> {
    let shipped = fill_paths(name, svg, prec)?;
    let upstream = fill_paths(name, svg, REFERENCE)?;
    if shipped.len() != upstream.len() {
        return Err(format!("{name}: layer counts differ"));
    }
    let mut out = Vec::new();
    for (s, u) in shipped.into_iter().zip(upstream) {
        if let Some(f) = fill::build(name, &s, prec)? {
            out.push((f, u));
        }
    }
    Ok(out)
}

/// Coverage (0..=255) of `fill` as the crate paints it on a `side`² grid;
/// the 24-unit box fills it.
pub fn coverage(fill: &Fill, side: usize) -> Vec<u8> {
    let scale = side as f32 / 24.0;
    let mut cov = vec![0.0; side * side];
    raster::fill_coverage(
        &mut cov,
        side,
        side,
        &fill.vertices,
        &fill.indices,
        |[x, y]| [x * scale, y * scale],
    );
    cov.iter()
        .map(|c| (c * f32::from(u8::MAX)).round() as u8)
        .collect()
}

fn worst(a: &[u8], b: &[u8]) -> u8 {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.abs_diff(*y))
        .max()
        .unwrap_or(0)
}

/// All three measures of one shipped fill against its upstream geometry.
pub fn measure(shipped: &Fill, upstream: &[FillPath]) -> [Measure; SIDES.len()] {
    let truth = exact::sides(upstream);
    let mesh = exact::mesh_sides(&shipped.vertices, &shipped.indices);
    SIDES.map(|s| {
        let e = exact::coverage(&truth, s);
        let m = exact::coverage(&mesh, s);
        let painted = coverage(shipped, s);
        Measure {
            raster: worst(&painted, &e),
            geometry: worst(&m, &e),
            rounding: worst(&painted, &m),
        }
    })
}
