//! Precision by visibility: a filled icon as the blob stores it (curves
//! flattened at `FLATTEN_TOLERANCE`, points rounded to 1/`COORD_SCALE` unit)
//! and painted by the crate's raster stays within 4 levels of the exact
//! coverage of its upstream outline on an opaque icon, up to 256 px
//! (`common` explains the measures). `fill_scan.rs` checks every shipped
//! icon.

mod common;

use common::{layers, measure, Measure, LEVELS, REFERENCE, SIDES};
use icongen::{convert, convert_with, Precision};

/// Filled icons with true curves: Phosphor's arcs (large and small radii),
/// Remix's circle cubics, Hugeicons' rounded corners. `wifi-slash` is the
/// worst of all shipped fills: it missed the budget at a tolerance of
/// 0.0012 units. `earthquake-line` is the icon lyon tessellates wrong
/// unless its axes are swapped (see `fill::tessellate`).
const ICONS: [&str; 9] = [
    "phosphor-circle",
    "phosphor-wifi-slash",
    "phosphor-record",
    "phosphor-spinner",
    "remix-checkbox-blank-circle-fill",
    "remix-record-circle-fill",
    "hugeicons-clipboard-check",
    "remix-earthquake-line",
    "remix-blender-fill",
];

fn svg(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}.svg", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The worst measures of a fixture at `prec`, per size.
fn measures(name: &str, prec: Precision) -> [Measure; SIDES.len()] {
    let mut out = [Measure::default(); SIDES.len()];
    for (shipped, upstream) in layers(name, &svg(name), prec).unwrap_or_else(|e| panic!("{e}")) {
        for (o, m) in out.iter_mut().zip(measure(&shipped, &upstream)) {
            *o = o.max(m);
        }
    }
    out
}

#[test]
fn a_blob_fill_is_within_four_levels_of_the_true_outline() {
    let mut fails = Vec::new();
    for name in ICONS {
        for (side, m) in SIDES.into_iter().zip(measures(name, Precision::BLOB)) {
            eprintln!("FILL {name} {side}: {m:?}");
            if m.raster > LEVELS {
                fails.push(format!("{name} {side} px: {m:?}"));
            }
        }
    }
    assert!(fails.is_empty(), "{fails:#?}");
}

/// Every fixture converts at the reference precision too: unrounded
/// points leave `blender-fill`'s inner disc, drawn once each way, as two
/// polylines a rounding error apart that cross many times.
#[test]
fn every_fixture_converts_at_the_reference_precision() {
    for name in ICONS {
        if let Err(e) = convert_with(name, &svg(name), REFERENCE) {
            panic!("{e}");
        }
    }
}

/// The blob's precision is what `convert` uses.
#[test]
fn convert_flattens_at_the_blob_precision() {
    let svg = svg("phosphor-record");
    assert_eq!(
        convert("record", &svg),
        convert_with("record", &svg, Precision::BLOB)
    );
}

/// How the tolerance was chosen: the worst geometry levels at 256 px per
/// tolerance, rounded as the blob rounds, and at any size.
/// `cargo test --test fill_precision -- --ignored --nocapture`
#[test]
#[ignore = "a measurement, not a check"]
fn tolerance_scan() {
    eprintln!("SCAN tolerance | {} | any size", ICONS.join(" | "));
    for tolerance in [
        0.02, 0.01, 0.005, 0.002, 0.0015, 0.0014, 0.0013, 0.0012, 0.001, 0.0008, 0.0005,
    ] {
        let prec = Precision {
            tolerance,
            quantise: true,
        };
        let mut all = 0;
        let at_256: Vec<String> = ICONS
            .iter()
            .map(|n| {
                let m = measures(n, prec).map(|m| m.geometry);
                all = m.into_iter().fold(all, u8::max);
                m[SIDES.len() - 1].to_string()
            })
            .collect();
        eprintln!("SCAN {tolerance} | {} | {all}", at_256.join(" | "));
    }
}
