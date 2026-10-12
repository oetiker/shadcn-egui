use stylegen::drift::{base_quotes, check, upstream_markers};
use stylegen::markers::parse_markers;

#[test]
fn finds_base_quotes() {
    let src = "let a = 1;\n    // base: \"disabled:opacity-50\"\n// base: \"inline-flex shrink-0\" — note\nlet b = 2; // base: \"h-px\"\n";
    assert_eq!(
        base_quotes(src),
        [
            (2, "disabled:opacity-50".to_string()),
            (3, "inline-flex shrink-0".into()),
            (4, "h-px".into())
        ]
    );
}

#[test]
fn marker_scan_skips_cli_markers() {
    let m = upstream_markers("class=\"cn-card-title cn-font-heading x\" cn-select-content-logical cn-menu-target cn-select-item");
    assert_eq!(
        m.into_iter().collect::<Vec<_>>(),
        ["cn-card-title", "cn-select-item"]
    );
}

#[test]
fn reports_stale_quote_and_marker_change() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("stylegen-drift-{}", std::process::id()));
    let up = dir.join("up");
    let krate = dir.join("crate");
    std::fs::create_dir_all(up.join("docs/src/lib/registry/ui/button")).unwrap();
    std::fs::create_dir_all(krate.join("src/components")).unwrap();
    std::fs::write(
        up.join("docs/src/lib/registry/ui/button/button.svelte"),
        "\"cn-button cn-button-size-new inline-flex disabled:opacity-50\"",
    )
    .unwrap();
    std::fs::write(
        krate.join("src/components/button.rs"),
        "// base: \"disabled:opacity-50\"\n// base: \"gone-class\"\n",
    )
    .unwrap();
    let lines = parse_markers("button.rs button: cn-button cn-button-size-old\n");
    let problems = check(&up, &krate, &lines);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(problems.len(), 3, "{problems:#?}");
    assert!(problems.iter().any(|p| p.contains("gone-class")));
    assert!(problems
        .iter()
        .any(|p| p.contains("cn-button-size-new") && p.contains("appeared")));
    assert!(problems
        .iter()
        .any(|p| p.contains("cn-button-size-old") && p.contains("disappeared")));
}

#[test]
fn reports_fixture_copies_that_differ_from_upstream() {
    use stylegen::drift::check_sources;
    use stylegen::sources::{sources, Upstream};
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("stylegen-sources-{}", std::process::id()));
    let (up, up_react, tool) = (dir.join("up"), dir.join("up-react"), dir.join("tool"));
    for (from, fixture, upstream) in sources() {
        let src = if from == Upstream::React {
            up_react.join(&upstream)
        } else {
            up.join(&upstream)
        };
        std::fs::create_dir_all(src.parent().unwrap()).unwrap();
        std::fs::write(&src, format!("upstream {fixture}")).unwrap();
        std::fs::create_dir_all(tool.join("tests/fixtures")).unwrap();
        std::fs::write(
            tool.join("tests/fixtures").join(&fixture),
            format!("upstream {fixture}"),
        )
        .unwrap();
    }
    assert!(
        check_sources(&up, &up_react, &tool).is_empty(),
        "identical copies are clean"
    );

    std::fs::write(
        up_react.join("apps/v4/registry/themes.ts"),
        "changed upstream",
    )
    .unwrap();
    std::fs::remove_file(tool.join("tests/fixtures/preset.ts")).unwrap();
    std::fs::write(
        up.join("docs/src/lib/registry/styles/style-vega.css"),
        "changed upstream",
    )
    .unwrap();
    let problems = check_sources(&up, &up_react, &tool);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(problems.len(), 3, "{problems:#?}");
    assert!(problems
        .iter()
        .any(|p| p.contains("themes.ts") && p.ends_with("re-run stylegen generate")));
    assert!(problems
        .iter()
        .any(|p| p.contains("preset.ts") && p.ends_with("re-run stylegen generate")));
    assert!(problems
        .iter()
        .any(|p| p.contains("style-vega.css") && p.ends_with("re-run stylegen generate")));
}
