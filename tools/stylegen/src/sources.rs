//! The upstream files stylegen reads. `generate` keeps a byte copy of each in
//! `tests/fixtures/`; tests read the copies, `drift` compares them with the
//! checkout.

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
pub fn refresh(up: &Path, up_react: &Path, tool: &Path) -> std::io::Result<()> {
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
    Ok(())
}
