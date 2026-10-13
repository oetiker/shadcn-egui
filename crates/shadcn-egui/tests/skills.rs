//! The Claude Code plugin under skills/ and .claude-plugin/ stays true to
//! the crate: versions, doctest wiring, component map, feature names.

const PLUGIN: &str = include_str!("../../../.claude-plugin/plugin.json");
const MARKETPLACE: &str = include_str!("../../../.claude-plugin/marketplace.json");
const REPO_INFRA: &str = include_str!("../../../.github/repo-infra.json");
const WORKSPACE: &str = include_str!("../../../Cargo.toml");

fn lines_starting<'a>(text: &'a str, prefix: &str) -> Vec<&'a str> {
    text.lines().filter(|l| l.starts_with(prefix)).collect()
}

fn workspace_version() -> &'static str {
    let line = lines_starting(WORKSPACE, "version = \"")[0];
    line.trim_start_matches("version = \"")
        .trim_end_matches('"')
}

#[test]
fn plugin_versions_follow_the_workspace() {
    let v = workspace_version();
    assert_eq!(
        lines_starting(PLUGIN, "  \"version\": \""),
        vec![format!("  \"version\": \"{v}\",").as_str()],
        "plugin.json version"
    );
    assert_eq!(
        lines_starting(MARKETPLACE, "      \"version\": \""),
        vec![format!("      \"version\": \"{v}\",").as_str()],
        "marketplace.json plugin entry version"
    );
    assert_eq!(
        lines_starting(MARKETPLACE, "        \"ref\": \"v"),
        vec![format!("        \"ref\": \"v{v}\"").as_str()],
        "marketplace.json source ref"
    );
}

#[test]
fn release_bumps_the_plugin() {
    for (path, pattern) in [
        (
            ".claude-plugin/plugin.json",
            r#"^  \"version\": \"[^\"]*\""#,
        ),
        (
            ".claude-plugin/marketplace.json",
            r#"^      \"version\": \"[^\"]*\""#,
        ),
        (
            ".claude-plugin/marketplace.json",
            r#"^        \"ref\": \"v[^\"]*\""#,
        ),
    ] {
        let entry = format!("\"path\": \"{path}\",\n      \"pattern\": \"{pattern}\"");
        assert!(
            REPO_INFRA.contains(&entry),
            "repo-infra.json has no version_files entry {path} {pattern}"
        );
    }
}

#[test]
fn plugin_is_named_shadcn_egui() {
    assert!(PLUGIN.contains("\"name\": \"shadcn-egui\""));
    assert!(PLUGIN.contains("\"repository\": \"https://github.com/oetiker/shadcn-egui\""));
    assert!(MARKETPLACE.starts_with("{\n  \"name\": \"shadcn-egui\","));
    assert!(MARKETPLACE.contains("\"repo\": \"oetiker/shadcn-egui\""));
    for text in [PLUGIN, MARKETPLACE] {
        assert!(!text.contains("egui-shadcn"), "old name left in a manifest");
    }
}
