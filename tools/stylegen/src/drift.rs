//! Drift between the crate and the pinned upstream: `cn-*` markers that
//! appeared or disappeared in the upstream component sources.

use crate::markers::MarkerLine;
use std::collections::BTreeSet;
use std::path::Path;

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
        let cli = crate::markers::is_cli_marker(m);
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
pub fn check(up: &Path, lines: &[MarkerLine]) -> Vec<String> {
    let mut problems = Vec::new();
    for line in lines {
        let upstream = read_dir_text(&up.join("docs/src/lib/registry/ui").join(&line.dir));
        // `slot:` parts have no marker to compare.
        let ours: BTreeSet<String> = line
            .markers
            .iter()
            .filter(|m| !m.starts_with("slot:"))
            .cloned()
            .collect();
        if ours.is_empty() {
            continue;
        }
        let up_set = upstream_markers(&upstream);
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

/// Component source copies (`<tool>/tests/fixtures/ui/`) that differ from
/// the checkout, are missing, or are no longer upstream.
pub fn check_ui(up: &Path, tool: &Path, lines: &[MarkerLine]) -> Vec<String> {
    use crate::sources::{svelte_files, UI};
    let ours_root = tool.join("tests/fixtures/ui");
    let theirs = match svelte_files(&up.join(UI), lines) {
        Ok(t) => t,
        Err(e) => return vec![format!("{UI}: cannot list the upstream sources ({e})")],
    };
    let ours = svelte_files(&ours_root, lines).unwrap_or_default();
    let mut problems = Vec::new();
    for (dir, file) in &theirs {
        let a = std::fs::read(ours_root.join(dir).join(file));
        let b = std::fs::read(up.join(UI).join(dir).join(file));
        match (a, b) {
            (Ok(a), Ok(b)) if a == b => {}
            (Err(_), _) => problems.push(format!(
                "tests/fixtures/ui/{dir}/{file} is missing; re-run stylegen generate"
            )),
            _ => problems.push(format!(
                "tests/fixtures/ui/{dir}/{file} differs from {UI}/{dir}/{file}; re-run stylegen generate"
            )),
        }
    }
    for (dir, file) in ours.iter().filter(|f| !theirs.contains(f)) {
        problems.push(format!(
            "tests/fixtures/ui/{dir}/{file} is no longer upstream; remove it"
        ));
    }
    problems
}
