//! `stylegen generate` after the fixture refresh: build every output from
//! the fixture copies, then write them.

use crate::{build, preset, themes};
use std::path::Path;

/// The crate stylegen writes into, relative to the repository root.
pub const CRATE: &str = "crates/shadcn-egui";

/// Builds the style recipes, palettes and preset enums from the fixtures in
/// `tool` and writes them under `root`'s crate, and the style reports into
/// `tool/reports`. On any error it writes the reports only, so they name
/// the unsupported classes, and returns every error: a half-written crate
/// may not compile.
pub fn generate(tool: &Path, root: &Path, pin: &str, pin_react: &str) -> Result<(), Vec<String>> {
    let krate = root.join(CRATE).join("src");
    let built = build::styles(tool, pin).map_err(|e| vec![e])?;
    let reports = tool.join("reports");
    std::fs::create_dir_all(&reports).expect("reports dir");
    for (style, md) in &built.reports {
        write(&reports.join(format!("{style}.md")), md);
    }
    let mut errors: Vec<String> = built
        .failed
        .iter()
        .map(|(style, n)| {
            format!("{style} is in EXPORT but has {n} unsupported classes; see reports/{style}.md")
        })
        .collect();
    let mut files = vec![(krate.join("style/generated.rs"), built.generated)];
    for (style, src) in built.exported {
        files.push((krate.join(format!("style/{style}.rs")), src));
    }
    let palettes = themes::parse_themes(
        &read(&tool.join("tests/fixtures/themes.ts")),
        &read(&tool.join("tests/fixtures/base-colors.ts")),
    )
    .and_then(|t| themes::emit_palettes(pin_react, &t));
    match palettes {
        Ok(src) => files.push((krate.join("palette/generated.rs"), src)),
        Err(e) => errors.push(format!("palettes: {e}")),
    }
    let enums = preset::parse_preset(
        &read(&tool.join("tests/fixtures/preset.ts")),
        &read(&tool.join("tests/fixtures/config.ts")),
    )
    .map(|l| preset::emit_preset(pin_react, &l));
    match enums {
        Ok(src) => files.push((krate.join("preset/generated.rs"), src)),
        Err(e) => errors.push(format!("preset: {e}")),
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    for (path, src) in &files {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("crate dir");
        write(path, src);
    }
    Ok(())
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn write(path: &Path, src: &str) {
    std::fs::write(path, src).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}
