//! Every filled icon of every shipped set, as `generate-crates.sh` reads
//! them from its download folder (env `ICONGEN_DOWNLOADS`, the script's
//! scratch dir): it converts at the blob's and at the reference precision,
//! and the crate's raster of its blob stays within 4 levels of the exact
//! coverage of the upstream outline at every size (`common` explains the
//! measures). With `ICONGEN_RASTER_REPORT` set it also prints the worst
//! levels of every measure per icon.
//!
//! Without `ICONGEN_DOWNLOADS` the test passes with a note (CI has no
//! downloads); with it, a missing or empty set fails.

mod common;

use common::{layers, measure, Measure, LEVELS, REFERENCE, SIDES};
use icongen::{convert_with, Precision};
use std::path::{Path, PathBuf};

const THREADS: usize = 4;

/// The SVG files of every shipped set below `dl`, per SETS.tsv.
fn shipped(dl: &Path) -> Result<Vec<PathBuf>, String> {
    let sets = include_str!("../SETS.tsv");
    let mut files = Vec::new();
    for row in sets.lines().skip(1) {
        let f: Vec<&str> = row.split('\t').collect();
        let [lib, package, version, glob, _, ship] = f[..] else {
            return Err(format!("SETS.tsv: bad row {row:?}"));
        };
        if ship != "yes" {
            continue;
        }
        let found = if lib == "hugeicons" {
            glob_files(&dl.join(format!("hugeicons-svg-{version}")), &["*.svg"])
        } else {
            let dir = dl.join(format!("npm/{}-{version}", package.replace('/', "_")));
            glob_files(&dir, &glob.split('/').collect::<Vec<_>>())
        };
        if found.is_empty() {
            return Err(format!("{lib} {version}: no SVGs under {}", dl.display()));
        }
        files.extend(found);
    }
    Ok(files)
}

/// The files matching `parts` below `dir`; a part is a name, `*` or `*.svg`.
fn glob_files(dir: &Path, parts: &[&str]) -> Vec<PathBuf> {
    let Some((first, rest)) = parts.split_first() else {
        return vec![dir.to_path_buf()];
    };
    let Some(pattern) = first.strip_prefix('*') else {
        return glob_files(&dir.join(first), rest);
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(pattern))
        .collect();
    names.sort();
    names
        .into_iter()
        .flat_map(|p| glob_files(&p, rest))
        .filter(|p| !rest.is_empty() || p.is_file())
        .collect()
}

/// What is wrong with one icon, if anything, and its line for the report
/// (`RASTER <file> <raster> <geometry> <rounding>`, worst levels per size).
fn check(path: &Path) -> (Option<String>, Option<String>) {
    let name = path.display().to_string();
    let svg = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return (Some(format!("{name}: {e}")), None),
    };
    // Conversion must succeed at the reference precision too.
    if let Err(e) = convert_with(&name, &svg, REFERENCE) {
        return (Some(format!("reference: {e}")), None);
    }
    let fills = match layers(&name, &svg, Precision::BLOB) {
        Ok(f) if f.is_empty() => return (None, None),
        Ok(f) => f,
        Err(e) => return (Some(format!("blob: {e}")), None),
    };
    let mut worst = [Measure::default(); SIDES.len()];
    for (shipped, upstream) in &fills {
        for (w, m) in worst.iter_mut().zip(measure(shipped, upstream)) {
            *w = w.max(m);
        }
    }
    let raster = worst.map(|m| m.raster);
    let geometry = worst.map(|m| m.geometry);
    let report = format!(
        "RASTER {name} {raster:?} {geometry:?} {:?}",
        worst.map(|m| m.rounding)
    );
    let fail = raster.iter().any(|&l| l > LEVELS);
    (
        fail.then(|| format!("{name}: raster levels {raster:?}, geometry {geometry:?}")),
        Some(report),
    )
}

#[test]
fn every_shipped_fill_is_sound_and_within_four_levels() {
    let Some(dl) = std::env::var_os("ICONGEN_DOWNLOADS") else {
        eprintln!("skipped: ICONGEN_DOWNLOADS not set");
        return;
    };
    let files = shipped(Path::new(&dl)).unwrap_or_else(|e| panic!("{e}"));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let (mut fails, mut rasters): (Vec<String>, Vec<String>) = std::thread::scope(|s| {
        let workers: Vec<_> = (0..THREADS)
            .map(|_| {
                s.spawn(|| {
                    let mut fails = Vec::new();
                    let mut rasters = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(f) = files.get(i) else {
                            break (fails, rasters);
                        };
                        let (fail, raster) = check(f);
                        fails.extend(fail);
                        rasters.extend(raster);
                    }
                })
            })
            .collect();
        let mut all = (Vec::new(), Vec::new());
        for w in workers {
            let (f, r) = w.join().expect("worker");
            all.0.extend(f);
            all.1.extend(r);
        }
        all
    });
    fails.sort();
    rasters.sort();
    // Every measure of every icon, for the maintainer's report.
    if std::env::var_os("ICONGEN_RASTER_REPORT").is_some() {
        for r in &rasters {
            eprintln!("{r}");
        }
    }
    eprintln!("scanned {} SVGs, {} failed", files.len(), fails.len());
    for f in &fails {
        eprintln!("FAIL {f}");
    }
    assert!(
        fails.is_empty(),
        "{} of {} icons failed",
        fails.len(),
        files.len()
    );
}
