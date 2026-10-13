const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{path}")).expect(path)
}

fn built() -> stylegen::build::Styles {
    let pin = read("PIN").trim().to_owned();
    stylegen::build::styles(std::path::Path::new(ROOT), &pin).expect("build")
}

/// The fixtures are byte copies of the pinned upstream files, so the
/// committed crate sources must be exactly what the generator gives now,
/// and giving it twice must not change a byte.
#[test]
fn committed_sources_are_current_and_output_is_deterministic() {
    let (first, second) = (built(), built());
    assert!(
        first.failed.is_empty(),
        "unsupported classes: {:?}",
        first.failed
    );
    assert_eq!(
        first.generated, second.generated,
        "output differs between runs"
    );
    assert_eq!(
        first.generated,
        read("../../crates/shadcn-egui/src/style/generated.rs"),
        "re-run stylegen generate"
    );
    let export = read("EXPORT");
    let names: Vec<&str> = first.exported.iter().map(|(s, _)| s.as_str()).collect();
    assert_eq!(names, export.split_whitespace().collect::<Vec<_>>());
    for ((style, src), (_, again)) in first.exported.iter().zip(&second.exported) {
        assert_eq!(src, again, "{style}: output differs between runs");
        assert_eq!(
            src,
            &read(&format!("../../crates/shadcn-egui/src/style/{style}.rs")),
            "{style}: re-run stylegen generate"
        );
    }
}

/// Stale reports hide new unsupported classes, so all eight must be current.
#[test]
fn committed_reports_are_current() {
    let b = built();
    assert_eq!(b.reports.len(), 8);
    for (style, md) in &b.reports {
        assert_eq!(
            md,
            &read(&format!("reports/{style}.md")),
            "{style}: re-run stylegen generate"
        );
    }
}

/// `palette/generated.rs` is exactly what the generator gives for the fixture copy.
#[test]
fn committed_palettes_are_current() {
    let pin = read("PIN_REACT").trim().to_owned();
    let themes = stylegen::themes::parse_themes(
        &read("tests/fixtures/themes.ts"),
        &read("tests/fixtures/base-colors.ts"),
    )
    .expect("themes.ts");
    let fresh = stylegen::themes::emit_palettes(&pin, &themes).expect("emit");
    assert_eq!(
        fresh,
        read("../../crates/shadcn-egui/src/palette/generated.rs"),
        "re-run stylegen generate"
    );
}

#[test]
fn committed_preset_enums_are_current() {
    let pin = read("PIN_REACT").trim().to_owned();
    let lists = stylegen::preset::parse_preset(
        &read("tests/fixtures/preset.ts"),
        &read("tests/fixtures/config.ts"),
    )
    .expect("parse");
    assert_eq!(
        stylegen::preset::emit_preset(&pin, &lists),
        read("../../crates/shadcn-egui/src/preset/generated.rs"),
        "re-run stylegen generate"
    );
}
