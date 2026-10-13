//! The upstream files stylegen reads. `generate` keeps a byte copy of each in
//! `tests/fixtures/`; tests read the copies, `drift` compares them with the
//! checkout.

use crate::markers::MarkerLine;
use std::collections::BTreeSet;
use std::path::Path;

/// Every upstream style; each gets a report, the ones in `EXPORT` also Rust.
pub const STYLES: &[&str] = &[
    "vega", "nova", "maia", "lyra", "mira", "luma", "sera", "rhea",
];

/// The two upstream checkouts: shadcn-svelte (`PIN`, style CSS) and
/// shadcn/ui (`PIN_REACT`, preset lists, palettes, fonts).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upstream {
    Svelte,
    React,
}

/// Where the component sources live in the shadcn-svelte checkout.
pub const UI: &str = "docs/src/lib/registry/ui";

/// `(dir, file)` of every `*.svelte` in `root/<dir>` for each component
/// directory `markers.txt` names, sorted.
pub fn svelte_files(root: &Path, lines: &[MarkerLine]) -> std::io::Result<Vec<(String, String)>> {
    let dirs: BTreeSet<&str> = lines.iter().map(|l| l.dir.as_str()).collect();
    let mut out = Vec::new();
    for dir in dirs {
        let mut names: Vec<String> = std::fs::read_dir(root.join(dir))?
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".svelte"))
            .collect();
        names.sort();
        out.extend(names.into_iter().map(|n| (dir.to_owned(), n)));
    }
    Ok(out)
}

/// `(checkout, fixture file name, path inside that checkout)`.
pub fn sources() -> Vec<(Upstream, String, String)> {
    let mut v: Vec<(Upstream, String, String)> = [
        ("themes.ts", "apps/v4/registry/themes.ts"),
        ("base-colors.ts", "apps/v4/registry/base-colors.ts"),
        ("config.ts", "apps/v4/registry/config.ts"),
        ("preset.ts", "packages/registry/src/preset/preset.ts"),
        ("font-definitions.ts", "apps/v4/lib/font-definitions.ts"),
    ]
    .iter()
    .map(|(f, u)| (Upstream::React, f.to_string(), u.to_string()))
    .collect();
    v.extend(STYLES.iter().map(|s| {
        (
            Upstream::Svelte,
            format!("style-{s}.css"),
            format!("docs/src/lib/registry/styles/style-{s}.css"),
        )
    }));
    v
}

/// Copy every source from its checkout (`up`: svelte, `up_react`: React)
/// into `<tool>/tests/fixtures/`.
pub fn refresh(
    up: &Path,
    up_react: &Path,
    tool: &Path,
    lines: &[MarkerLine],
) -> std::io::Result<()> {
    for (from, fixture, upstream) in sources() {
        let root = if from == Upstream::React {
            up_react
        } else {
            up
        };
        std::fs::copy(
            root.join(&upstream),
            tool.join("tests/fixtures").join(&fixture),
        )?;
    }
    let ui = up.join(UI);
    for (dir, file) in svelte_files(&ui, lines)? {
        let to = tool.join("tests/fixtures/ui").join(&dir);
        std::fs::create_dir_all(&to)?;
        std::fs::copy(ui.join(&dir).join(&file), to.join(&file))?;
    }
    Ok(())
}
