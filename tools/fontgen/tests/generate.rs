//! `generate` validates everything before it writes anything.

use std::fs;
use std::path::Path;

use fontgen::generate;

fn put(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn missing_licence_fails_and_writes_nothing() {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join("generate-missing-licence");
    let _ = fs::remove_dir_all(&work);
    let repo = work.join("repo");
    let downloads = work.join("downloads");
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    put(
        &repo.join("tools/fontgen/FONTS.tsv"),
        "name\ttitle\tkind\tlicense\tfiles\tlicense_file\ty_offset_factor\n\
         inter\tInter\tsans\tOFL-1.1\tofl/inter/Inter[opsz,wght].ttf\tofl/inter/OFL.txt\t-0.04\n",
    );
    put(&repo.join("tools/fontgen/GF_PIN"), "abc1234\n");
    put(&repo.join("LICENSE"), "MIT\n");
    put(
        &repo.join("tools/stylegen/tests/fixtures/font-definitions.ts"),
        "\t\tname: \"inter\",\n\t\ttitle: \"Inter\",\n\t\ttype: \"sans\",\n",
    );
    put(&repo.join("Cargo.toml"), "[workspace.package]\nversion = \"0.4.0\"\n[workspace.dependencies]\n# BEGIN fontgen\n# END fontgen\n");
    put(
        &repo.join("crates/shadcn-egui/Cargo.toml"),
        "# BEGIN fontgen\n# END fontgen\n",
    );
    let font = downloads.join("ofl/inter/Inter[opsz,wght].ttf");
    fs::create_dir_all(font.parent().unwrap()).unwrap();
    fs::copy(
        manifest.join("../../crates/shadcn-egui-font-inter/fonts/inter.ttf"),
        &font,
    )
    .unwrap();
    // no ofl/inter/OFL.txt in `downloads`

    let before = fs::read_to_string(repo.join("Cargo.toml")).unwrap();
    let e = generate(&downloads, &repo).unwrap_err();
    assert!(e.contains("OFL.txt"), "{e}");
    assert!(
        !repo.join("crates/shadcn-egui-font-inter").exists(),
        "no crate dir written"
    );
    assert!(
        !repo
            .join("crates/shadcn-egui/src/fonts/registry.rs")
            .exists(),
        "no registry written"
    );
    assert_eq!(fs::read_to_string(repo.join("Cargo.toml")).unwrap(), before);
}

/// A complete fixture repository (and downloads) that generates cleanly.
fn fixture(name: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let work = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&work);
    let repo = work.join("repo");
    let downloads = work.join("downloads");
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    put(
        &repo.join("tools/fontgen/FONTS.tsv"),
        "name\ttitle\tkind\tlicense\tfiles\tlicense_file\ty_offset_factor\n\
         inter\tInter\tsans\tOFL-1.1\tofl/inter/Inter[opsz,wght].ttf\tofl/inter/OFL.txt\t-0.04\n",
    );
    put(&repo.join("tools/fontgen/GF_PIN"), "abc1234\n");
    put(&repo.join("LICENSE"), "MIT\n");
    put(
        &repo.join("tools/stylegen/tests/fixtures/font-definitions.ts"),
        "\t\tname: \"inter\",\n\t\ttitle: \"Inter\",\n\t\ttype: \"sans\",\n",
    );
    put(&repo.join("Cargo.toml"), "[workspace.package]\nversion = \"0.4.0\"\n[workspace.dependencies]\n# BEGIN fontgen\n# END fontgen\n");
    put(
        &repo.join("crates/shadcn-egui/Cargo.toml"),
        "# BEGIN fontgen\n# END fontgen\n",
    );
    let font = downloads.join("ofl/inter/Inter[opsz,wght].ttf");
    fs::create_dir_all(font.parent().unwrap()).unwrap();
    fs::copy(
        manifest.join("../../crates/shadcn-egui-font-inter/fonts/inter.ttf"),
        &font,
    )
    .unwrap();
    put(&downloads.join("ofl/inter/OFL.txt"), "OFL\n");
    (repo, downloads)
}

#[test]
fn a_clean_fixture_generates() {
    let (repo, downloads) = fixture("generate-clean");
    generate(&downloads, &repo).unwrap();
    assert!(repo
        .join("crates/shadcn-egui-font-inter/Cargo.toml")
        .exists());
}

#[test]
fn stale_crates_and_font_files_fail_and_are_listed_not_deleted() {
    let (repo, downloads) = fixture("generate-stale");
    put(&repo.join("crates/shadcn-egui-font-gone/Cargo.toml"), "x\n");
    put(
        &repo.join("crates/shadcn-egui-font-gone/fonts/old.ttf"),
        "x\n",
    );
    put(
        &repo.join("crates/shadcn-egui-font-inter/fonts/x.ttf"),
        "x\n",
    );
    let e = generate(&downloads, &repo).unwrap_err();
    assert!(e.contains("shadcn-egui-font-gone"), "{e}");
    assert!(e.contains("fonts/x.ttf"), "{e}");
    assert!(
        !e.contains("fonts/inter.ttf"),
        "an output is not stale: {e}"
    );
    assert!(repo
        .join("crates/shadcn-egui-font-gone/fonts/old.ttf")
        .exists());
    assert!(repo
        .join("crates/shadcn-egui-font-inter/fonts/x.ttf")
        .exists());
    assert!(
        !repo
            .join("crates/shadcn-egui/src/fonts/registry.rs")
            .exists(),
        "nothing is written when stale files exist"
    );
}

#[test]
fn bad_gf_pins_fail() {
    // generate() fails before it writes, so one fixture serves every pin.
    let (repo, downloads) = fixture("generate-bad-pin");
    for bad in ["", "  \n", "4", "xyz1234"] {
        put(&repo.join("tools/fontgen/GF_PIN"), bad);
        let e = generate(&downloads, &repo).unwrap_err();
        assert!(e.contains("GF_PIN"), "{bad:?}: {e}");
    }
}
