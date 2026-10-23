//! Snapshot checks: egui_kittest's comparison, then a strict one.
//!
//! egui_kittest 0.34 compares with dify's anti-aliasing detection always on
//! (snapshot.rs:484). A pixel dify takes for an anti-aliased edge never
//! counts, so a 1 px border may change colour and the test still passes.
//! After kittest's check passes, [`strict_check`] compares the render with
//! the committed PNG again, every pixel, each channel within [`TOLERANCE`]
//! levels. On a failure it writes `<name>.new.png` and `<name>.diff.png`
//! where kittest writes them. In update mode (`UPDATE_SNAPSHOTS` set) it
//! does nothing: kittest does not rewrite a PNG that differs only on edges,
//! so a change that touches a snapshot deletes its PNG before the update.
//!
//! Every snapshot test goes through these functions; `clippy.toml` forbids
//! kittest's own snapshot calls everywhere else.
#![allow(clippy::disallowed_methods)]

use std::path::Path;

use egui_kittest::{Harness, SnapshotError, SnapshotOptions, SnapshotResult, SnapshotResults};

/// Largest per-channel difference (0-255) a pixel may show before the
/// strict check counts it. The mesa software renderer that wrote the
/// baselines reproduces them exactly; 2 levels leave room for the mesa
/// build of another machine (CI).
pub const TOLERANCE: u8 = 2;

/// kittest's check of the harness's render under its default options, then
/// the strict check. Panics at the end of the call on a failure.
#[track_caller]
pub fn snapshot<S>(h: &mut Harness<'_, S>, name: impl Into<String>) {
    let mut results = SnapshotResults::new();
    results.add(try_snapshot(h, name));
}

/// Like [`snapshot`], with `options` in place of the harness's.
#[track_caller]
pub fn snapshot_options<S>(
    h: &mut Harness<'_, S>,
    name: impl Into<String>,
    options: &SnapshotOptions,
) {
    let mut results = SnapshotResults::new();
    results.add(try_snapshot_options(h, name, options));
}

/// Like [`snapshot`], returning the result instead of panicking.
pub fn try_snapshot<S>(h: &mut Harness<'_, S>, name: impl Into<String>) -> SnapshotResult {
    let options = h.options().clone();
    try_snapshot_options(h, name, &options)
}

/// Like [`snapshot_options`], returning the result instead of panicking.
pub fn try_snapshot_options<S>(
    h: &mut Harness<'_, S>,
    name: impl Into<String>,
    options: &SnapshotOptions,
) -> SnapshotResult {
    let image = h
        .render()
        .map_err(|err| SnapshotError::RenderError { err })?;
    try_image_snapshot_options(&image, name, options)
}

/// kittest's check of `image` under the default options, then the strict
/// check.
pub fn try_image_snapshot(image: &image::RgbaImage, name: impl Into<String>) -> SnapshotResult {
    try_image_snapshot_options(image, name, &SnapshotOptions::default())
}

/// Like [`try_image_snapshot`], panicking at the end of the call on a failure.
#[track_caller]
pub fn image_snapshot_options(
    image: &image::RgbaImage,
    name: impl Into<String>,
    options: &SnapshotOptions,
) {
    let mut results = SnapshotResults::new();
    results.add(try_image_snapshot_options(image, name, options));
}

/// kittest's check of `image` under `options`, then the strict check.
pub fn try_image_snapshot_options(
    image: &image::RgbaImage,
    name: impl Into<String>,
    options: &SnapshotOptions,
) -> SnapshotResult {
    let name = name.into();
    egui_kittest::try_image_snapshot_options(image, name.clone(), options)?;
    if update_mode() {
        return Ok(());
    }
    strict_check(image, &name, &options.output_path)
}

/// kittest's own reading of `UPDATE_SNAPSHOTS` (snapshot.rs `Mode::from_env`).
fn update_mode() -> bool {
    matches!(
        std::env::var("UPDATE_SNAPSHOTS").as_deref(),
        Ok("true" | "1" | "yes" | "on" | "force")
    )
}

/// Compare `image` with `{dir}/{name}.png`: every pixel, each channel
/// within [`TOLERANCE`]. kittest's check ran first, so the file exists and
/// has the same size.
fn strict_check(image: &image::RgbaImage, name: &str, dir: &Path) -> SnapshotResult {
    let path = dir.join(format!("{name}.png"));
    let committed = image::open(&path)
        .map_err(|err| SnapshotError::OpenSnapshot {
            path: path.clone(),
            err,
        })?
        .to_rgba8();
    let mut diff = image::RgbaImage::new(image.width(), image.height());
    let mut failed = 0;
    for ((new, old), out) in image
        .pixels()
        .zip(committed.pixels())
        .zip(diff.pixels_mut())
    {
        let over = new
            .0
            .iter()
            .zip(old.0)
            .any(|(a, b)| a.abs_diff(b) > TOLERANCE);
        *out = if over {
            failed += 1;
            image::Rgba([255, 0, 0, 255])
        } else {
            // The render, faded towards white, under the failing pixels.
            let fade = |c: u8| 255 - (255 - c) / 4;
            image::Rgba([fade(new[0]), fade(new[1]), fade(new[2]), 255])
        };
    }
    if failed == 0 {
        return Ok(());
    }
    let write = |img: &image::RgbaImage, path: std::path::PathBuf| {
        img.save(&path)
            .map_err(|err| SnapshotError::WriteSnapshot { path, err })
    };
    write(image, dir.join(format!("{name}.new.png")))?;
    let diff_path = dir.join(format!("{name}.diff.png"));
    write(&diff, diff_path.clone())?;
    Err(SnapshotError::Diff {
        name: format!(
            "{name}' strict check (anti-aliased edges counted, tolerance {TOLERANCE}): \
             delete the PNG before `UPDATE_SNAPSHOTS=1`; '{name}"
        ),
        diff: failed,
        diff_path,
    })
}
