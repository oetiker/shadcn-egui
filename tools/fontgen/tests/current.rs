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

#[test]
fn shipped_fonts_are_measured() {
    let unmeasured: Vec<String> = rows()
        .into_iter()
        .filter(|r| r.shipped() && r.y_offset_factor.is_none())
        .map(|r| r.name)
        .collect();
    assert!(
        unmeasured.is_empty(),
        "measure with tests/font_offset.rs: {unmeasured:?}"
    );
}

#[test]
fn no_stale_font_crates() {
    let shipped: Vec<String> = rows()
        .iter()
        .filter(|r| r.shipped())
        .map(Row::crate_name)
        .collect();
    for entry in fs::read_dir(repo().join("crates")).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if name.starts_with("shadcn-egui-font-") {
            assert!(
                shipped.contains(&name),
                "{name} has no shipped FONTS.tsv row; delete the crate"
            );
        }
    }
}

#[test]
fn registry_and_cargo_blocks_are_current() {
    let rows = rows();
    let pin = read(repo().join("tools/fontgen/GF_PIN")).trim().to_string();
    assert_eq!(
        read(repo().join("crates/shadcn-egui/src/fonts/registry.rs")),
        registry_rs(&rows, &pin).unwrap()
    );
    let main = read(repo().join("crates/shadcn-egui/Cargo.toml"));
    assert_eq!(
        replace_block(&main, &features_block(&rows)).unwrap(),
        main,
        "crates/shadcn-egui/Cargo.toml block"
    );
    let root = read(repo().join("Cargo.toml"));
    let version = workspace_version(&root).unwrap();
    assert_eq!(
        replace_block(&root, &workspace_deps_block(&rows, &version)).unwrap(),
        root,
        "root Cargo.toml block"
    );
}
