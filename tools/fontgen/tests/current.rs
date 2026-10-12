//! The committed outputs match what fontgen renders from FONTS.tsv. Only
//! needs the font files that are already committed, never a download.

use std::fs;
use std::path::{Path, PathBuf};

use fontgen::*;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(p: impl AsRef<Path>) -> String {
    fs::read_to_string(p.as_ref()).unwrap_or_else(|e| panic!("{}: {e}", p.as_ref().display()))
}

fn rows() -> Vec<Row> {
    parse_fonts_tsv(&read(repo().join("tools/fontgen/FONTS.tsv"))).expect("FONTS.tsv parses")
}

fn wght(row: &Row, dir: &Path) -> Option<(f32, f32)> {
    match row.files {
        Files::Variable(_) => {
            let ttf =
                fs::read(dir.join(format!("fonts/{}.ttf", row.name))).expect("committed font file");
            Some(wght_range(&ttf).unwrap_or_else(|e| panic!("{}: {e}", row.name)))
        }
        Files::Static(_) => None,
    }
}

#[test]
fn fonts_tsv_matches_upstream_font_definitions() {
    let ts = read(repo().join("tools/stylegen/tests/fixtures/font-definitions.ts"));
    check_definitions(&rows(), &ts).unwrap();
}

#[test]
fn committed_font_crates_are_current() {
    let mut checked = Vec::new();
    for row in rows().iter().filter(|r| r.shipped()) {
        let dir = repo().join("crates").join(row.crate_name());
        // Phase 7 Task 2 removes this skip: from then on every shipped font has its crate.
        if !dir.exists() && row.name != "inter" && row.name != "oxanium" {
            continue;
        }
        assert_eq!(
            read(dir.join("Cargo.toml")),
            crate_cargo_toml(row),
            "{}: Cargo.toml",
            row.name
        );
        assert_eq!(
            read(dir.join("src/lib.rs")),
            crate_lib_rs(row, wght(row, &dir)).unwrap(),
            "{}: src/lib.rs",
            row.name
        );
        checked.push(row.name.clone());
    }
    assert!(
        checked.contains(&"inter".to_string()) && checked.contains(&"oxanium".to_string()),
        "{checked:?}"
    );
}
