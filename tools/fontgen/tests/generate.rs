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
    put(&repo.join("tools/fontgen/GF_PIN"), "abc\n");
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
