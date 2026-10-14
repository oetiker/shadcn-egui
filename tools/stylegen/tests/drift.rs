use stylegen::drift::{check, upstream_markers};
use stylegen::markers::parse_markers;

#[test]
fn marker_scan_skips_cli_markers() {
    let m = upstream_markers("class=\"cn-card-title cn-font-heading x\" cn-select-content-logical cn-menu-target cn-select-item");
    assert_eq!(
        m.into_iter().collect::<Vec<_>>(),
        ["cn-card-title", "cn-select-item"]
    );
}

#[test]
fn reports_marker_changes() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("stylegen-drift-{}", std::process::id()));
    let up = dir.join("up");
    std::fs::create_dir_all(up.join("docs/src/lib/registry/ui/button")).unwrap();
    std::fs::write(
        up.join("docs/src/lib/registry/ui/button/button.svelte"),
        "\"cn-button cn-button-size-new inline-flex disabled:opacity-50\"",
    )
    .unwrap();
    let lines = parse_markers("button.rs button: cn-button cn-button-size-old\n");
    let problems = check(&up, &lines);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(problems.len(), 2, "{problems:#?}");
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

#[test]
fn reports_ui_copies_that_differ_missing_or_stale() {
    use stylegen::drift::check_ui;
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("stylegen-ui-{}", std::process::id()));
    let up = dir.join("up");
    let tool = dir.join("tool");
    let ui_up = up.join("docs/src/lib/registry/ui/tabs");
    let ui_ours = tool.join("tests/fixtures/ui/tabs");
    std::fs::create_dir_all(&ui_up).unwrap();
    std::fs::create_dir_all(&ui_ours).unwrap();
    for f in ["tabs.svelte", "tabs-list.svelte"] {
        std::fs::write(ui_up.join(f), f).unwrap();
        std::fs::write(ui_ours.join(f), f).unwrap();
    }
    let lines = parse_markers("tabs.rs tabs: cn-tabs\n");
    assert!(
        check_ui(&up, &tool, &lines).is_empty(),
        "identical copies are clean"
    );

    std::fs::write(ui_up.join("tabs-list.svelte"), "changed").unwrap();
    std::fs::write(ui_up.join("tabs-trigger.svelte"), "new").unwrap();
    std::fs::write(ui_ours.join("tabs-old.svelte"), "gone upstream").unwrap();
    let problems = check_ui(&up, &tool, &lines);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(problems.len(), 3, "{problems:#?}");
    assert!(problems
        .iter()
        .any(|p| p.contains("tabs-list.svelte differs")));
    assert!(problems
        .iter()
        .any(|p| p.contains("tabs-trigger.svelte is missing")));
    assert!(problems
        .iter()
        .any(|p| p.contains("tabs-old.svelte is no longer upstream")));
}

#[test]
fn cli_markers_are_named_in_one_place() {
    use stylegen::markers::is_cli_marker;
    for m in [
        "cn-font-heading",
        "cn-menu-target",
        "cn-select-content-logical",
        "cn-rtl-flip",
        "cn-logical-sides",
    ] {
        assert!(is_cli_marker(m), "{m}");
    }
    assert!(!is_cli_marker("cn-select-content"));
}

#[test]
fn ui_dirs_that_cannot_be_listed_are_problems() {
    use stylegen::drift::check_ui;
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("stylegen-ui-list-{}", std::process::id()));
    let (up, tool) = (dir.join("up"), dir.join("tool"));
    let lines = parse_markers("tabs.rs tabs: cn-tabs\nbutton.rs button: cn-button\n");
    for d in ["tabs", "button"] {
        let ui_up = up.join("docs/src/lib/registry/ui").join(d);
        std::fs::create_dir_all(&ui_up).unwrap();
        std::fs::write(ui_up.join(format!("{d}.svelte")), d).unwrap();
    }
    // The fixtures lack the button dir and keep a stale tabs file.
    let ui_ours = tool.join("tests/fixtures/ui/tabs");
    std::fs::create_dir_all(&ui_ours).unwrap();
    std::fs::write(ui_ours.join("tabs.svelte"), "tabs").unwrap();
    std::fs::write(ui_ours.join("tabs-old.svelte"), "gone upstream").unwrap();
    let fixtures = check_ui(&up, &tool, &lines);
    // A whole component dir missing upstream.
    std::fs::remove_dir_all(up.join("docs/src/lib/registry/ui/button")).unwrap();
    let upstream = check_ui(&up, &tool, &lines);
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(
        fixtures
            .iter()
            .any(|p| p.contains("cannot list tests/fixtures/ui")),
        "{fixtures:#?}"
    );
    assert_eq!(upstream.len(), 1, "{upstream:#?}");
    assert!(
        upstream[0].contains("cannot list the upstream sources"),
        "{upstream:#?}"
    );
}

/// M5: `generate` (through `refresh`) fails on a fixture `.svelte` that left
/// upstream instead of translating the stale copy. It does not delete it.
#[test]
fn refresh_fails_on_a_stale_fixture() {
    use stylegen::sources::{refresh, sources, Upstream};
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("stylegen-refresh-stale-{}", std::process::id()));
    let (up, up_react, tool) = (dir.join("up"), dir.join("react"), dir.join("tool"));
    for (from, _, path) in sources() {
        let root = if from == Upstream::React {
            &up_react
        } else {
            &up
        };
        let p = root.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "x").unwrap();
    }
    std::fs::create_dir_all(tool.join("tests/fixtures/ui/tabs")).unwrap();
    let ui_up = up.join("docs/src/lib/registry/ui/tabs");
    std::fs::create_dir_all(&ui_up).unwrap();
    std::fs::write(ui_up.join("tabs.svelte"), "tabs").unwrap();
    let lines = parse_markers("tabs.rs tabs: cn-tabs\n");
    let fresh = refresh(&up, &up_react, &tool, &lines);
    let stale_file = tool.join("tests/fixtures/ui/tabs/tabs-old.svelte");
    std::fs::write(&stale_file, "gone upstream").unwrap();
    let stale = refresh(&up, &up_react, &tool, &lines);
    let kept = stale_file.exists();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(fresh.is_ok(), "{fresh:?}");
    let err = stale.expect_err("stale fixture").to_string();
    assert!(err.contains("tabs/tabs-old.svelte"), "{err}");
    assert!(kept, "refresh leaves the file for the maintainer to remove");
}
