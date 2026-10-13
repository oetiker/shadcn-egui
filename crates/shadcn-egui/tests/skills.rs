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

const LIB: &str = include_str!("../src/lib.rs");

/// `<skill>/SKILL.md` and `<skill>/references/*.md` for every directory
/// under skills/ that holds a SKILL.md.
fn skill_files() -> Vec<String> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../skills");
    let mut out = Vec::new();
    for skill in std::fs::read_dir(root).unwrap() {
        let dir = skill.unwrap().path();
        if !dir.join("SKILL.md").is_file() {
            continue;
        }
        let name = dir.file_name().unwrap().to_str().unwrap().to_owned();
        out.push(format!("{name}/SKILL.md"));
        if let Ok(refs) = std::fs::read_dir(dir.join("references")) {
            for r in refs {
                let f = r.unwrap().file_name().into_string().unwrap();
                if f.ends_with(".md") {
                    out.push(format!("{name}/references/{f}"));
                }
            }
        }
    }
    out.sort();
    out
}

#[test]
fn every_skill_file_is_a_doctest() {
    for f in skill_files() {
        let item = format!("include_str!(\"../../../skills/{f}\")");
        assert!(LIB.contains(&item), "lib.rs skill_docs lacks {f}");
    }
}

#[test]
fn every_fence_has_a_language() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../skills/");
    for f in skill_files() {
        let text = std::fs::read_to_string(format!("{root}{f}")).unwrap();
        let mut open = false;
        for (n, line) in text.lines().enumerate() {
            if let Some(rest) = line.trim_start().strip_prefix("```") {
                if !open {
                    assert!(
                        !rest.trim().is_empty(),
                        "{f}:{}: fence without a language",
                        n + 1
                    );
                }
                open = !open;
            }
        }
    }
}

const COMPONENT_MAP: &str = include_str!("../../../skills/build-ui/references/component-map.md");
const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn component_map_has_every_component() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/components");
    for e in std::fs::read_dir(dir).unwrap() {
        let f = e.unwrap().file_name().into_string().unwrap();
        let Some(module) = f.strip_suffix(".rs") else {
            continue;
        };
        if ["mod", "shared", "icon_raster"].contains(&module) {
            continue;
        }
        let path = format!("shadcn_egui::components::{module}::");
        assert!(
            COMPONENT_MAP.contains(&path),
            "component-map.md has no row for {module}"
        );
    }
}

#[test]
fn skills_name_only_existing_features() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../skills/");
    let features: Vec<&str> = MANIFEST
        .lines()
        .filter_map(|l| l.split_once(" = [").map(|(k, _)| k.trim()))
        .collect();
    // Tailwind's font utilities, which styles.md maps to recipe fields.
    let tailwind = [
        "font-thin",
        "font-extralight",
        "font-light",
        "font-normal",
        "font-medium",
        "font-semibold",
        "font-bold",
        "font-extrabold",
        "font-black",
        "font-sans",
        "font-serif",
        "font-mono",
    ];
    for f in skill_files() {
        let text = std::fs::read_to_string(format!("{root}{f}")).unwrap();
        for word in text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-')) {
            if tailwind.contains(&word) {
                continue;
            }
            let named = ["font-", "fonts-", "icons-"]
                .iter()
                .any(|p| word.starts_with(p));
            if named && word.len() > 6 && !word.ends_with('-') {
                assert!(
                    features.contains(&word),
                    "{f} names feature `{word}`, which Cargo.toml lacks"
                );
            }
        }
    }
}
