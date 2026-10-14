//! What `stylegen generate` writes for the style recipes, as strings, so
//! the tests compare the committed files with exactly what generate writes.

use crate::css::parse_style_css;
use crate::markers::parse_markers;
use crate::resolve::{parts, Part};
use crate::sources::STYLES;
use crate::{emit, extract, report};
use std::path::Path;

pub struct Styles {
    /// `crates/shadcn-egui/src/style/generated.rs`.
    pub generated: String,
    /// `(style, src/style/<style>.rs)`, every style in `EXPORT` without
    /// unsupported classes, in [`STYLES`] order.
    pub exported: Vec<(String, String)>,
    /// `(style, reports/<style>.md)` for every upstream style.
    pub reports: Vec<(String, String)>,
    /// Styles in `EXPORT` with unsupported classes, with their count.
    pub failed: Vec<(String, usize)>,
}

pub fn styles(tool: &Path, pin: &str) -> Result<Styles, String> {
    let read = |p: &str| std::fs::read_to_string(tool.join(p)).map_err(|e| format!("{p}: {e}"));
    let lines = parse_markers(&read("markers.txt")?);
    let export = read("EXPORT")?;
    let components = extract::part_strings(&tool.join("tests/fixtures/ui"), &lines)?;
    let mut per_style: Vec<(&str, Vec<Part>)> = Vec::new();
    for style in STYLES {
        let css = parse_style_css(&read(&format!("tests/fixtures/style-{style}.css"))?);
        per_style.push((style, parts(style, &css, &components)?));
    }
    let all: Vec<&[Part]> = per_style.iter().map(|(_, p)| p.as_slice()).collect();
    let fields = emit::fields(&all);
    let mut out = Styles {
        generated: emit::emit_struct(pin, &fields),
        exported: Vec::new(),
        reports: Vec::new(),
        failed: Vec::new(),
    };
    for (style, ps) in &per_style {
        out.reports
            .push((style.to_string(), report::report(style, pin, ps)));
        if export.split_whitespace().any(|e| e == *style) {
            let unsupported: usize = ps.iter().map(|p| p.marker.unsupported.len()).sum();
            if unsupported == 0 {
                out.exported
                    .push((style.to_string(), emit::emit_style(style, pin, &fields, ps)));
            } else {
                out.failed.push((style.to_string(), unsupported));
            }
        }
    }
    Ok(out)
}
