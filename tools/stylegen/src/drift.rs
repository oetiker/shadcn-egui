//! Drift between the crate and the pinned upstream: `// base: "…"` quotes
//! that upstream no longer contains, and `cn-*` markers that appeared or
//! disappeared in the upstream component sources.

use crate::markers::MarkerLine;
use std::collections::BTreeSet;
use std::path::Path;

/// `// base: "<upstream classes>"` quotes, with 1-based line numbers.
pub fn base_quotes(src: &str) -> Vec<(usize, String)> {
    src.lines()
        .enumerate()
        .filter_map(|(i, l)| {
            let rest = &l[l.find("// base: \"")? + "// base: \"".len()..];
            Some((i + 1, rest[..rest.find('"')?].to_owned()))
        })
        .collect()
}

/// `cn-*` markers in upstream component text, minus the ones only the shadcn
/// CLI rewrites (fonts, menu targets, RTL/logical-side helpers).
pub fn upstream_markers(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = text;
    while let Some(i) = rest.find("cn-") {
        let tail = &rest[i..];
        let len = tail
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(tail.len());
        let m = &tail[..len];
        let cli = m == "cn-font-heading"
            || m.starts_with("cn-menu-")
            || m.ends_with("-logical")
            || m == "cn-rtl-flip"
            || m == "cn-logical-sides";
        if !cli && m.len() > 3 {
            out.insert(m.to_owned());
        }
        rest = &tail[len..];
    }
    out
}

fn read_dir_text(dir: &Path) -> String {
    let mut s = String::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            if let Ok(t) = std::fs::read_to_string(e.path()) {
                s.push_str(&t);
                s.push('\n');
            }
        }
    }
    s
}

/// One human-readable line per problem; empty means clean.
pub fn check(up: &Path, crate_root: &Path, lines: &[MarkerLine]) -> Vec<String> {
    let mut problems = Vec::new();
    for line in lines {
        let upstream = read_dir_text(&up.join("docs/src/lib/registry/ui").join(&line.dir));
        let file = if line.file == "layout.rs" {
            crate_root.join("src/layout.rs")
        } else {
            crate_root.join("src/components").join(&line.file)
        };
        if let Ok(src) = std::fs::read_to_string(&file) {
            for (n, q) in base_quotes(&src) {
                if !upstream.contains(&q) {
                    problems.push(format!(
                        "{}:{n}: base quote no longer upstream in ui/{}: \"{q}\"",
                        line.file, line.dir
                    ));
                }
            }
        }
        if line.markers.is_empty() {
            continue;
        }
        let up_set = upstream_markers(&upstream);
        let ours: BTreeSet<String> = line.markers.iter().cloned().collect();
        for m in up_set.difference(&ours) {
            problems.push(format!(
                "{}: marker {m} appeared upstream; re-port",
                line.file
            ));
        }
        for m in ours.difference(&up_set) {
            problems.push(format!(
                "{}: marker {m} disappeared upstream; re-port",
                line.file
            ));
        }
    }
    problems
}

/// Fixture copies (`<tool>/tests/fixtures/`) that no longer match the
/// upstream checkout: tests and generated code would describe stale sources.
pub fn check_sources(up: &Path, up_react: &Path, tool: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    for (from, fixture, upstream) in crate::sources::sources() {
        let root = if from == crate::sources::Upstream::React {
            up_react
        } else {
            up
        };
        let ours = std::fs::read(tool.join("tests/fixtures").join(&fixture));
        let theirs = std::fs::read(root.join(&upstream));
        match (ours, theirs) {
            (Ok(a), Ok(b)) if a == b => {}
            (_, Err(e)) => problems.push(format!(
                "{upstream}: cannot read the upstream file ({e}); re-run stylegen generate"
            )),
            (Err(_), _) => problems.push(format!(
                "tests/fixtures/{fixture} is missing; re-run stylegen generate"
            )),
            _ => problems.push(format!(
                "tests/fixtures/{fixture} differs from {upstream}; re-run stylegen generate"
            )),
        }
    }
    problems
}
