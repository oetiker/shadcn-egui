//! The Claude Code plugin under plugin/ and .claude-plugin/ stays true to
//! the crate: versions, doctest wiring, component map, feature names.

const PLUGIN: &str = include_str!("../../../plugin/.claude-plugin/plugin.json");
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
            "plugin/.claude-plugin/plugin.json",
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
    // The marketplace installs the plugin/ directory of this repository.
    assert!(MARKETPLACE.contains("\"source\": \"git-subdir\""));
    assert!(MARKETPLACE.contains("\"url\": \"https://github.com/oetiker/shadcn-egui.git\""));
    assert!(MARKETPLACE.contains("\"path\": \"plugin\""));
    for text in [PLUGIN, MARKETPLACE] {
        assert!(!text.contains("egui-shadcn"), "old name left in a manifest");
    }
}

const LIB: &str = include_str!("../src/lib.rs");

/// `<skill>/SKILL.md` and `<skill>/references/*.md` for every directory
/// under plugin/skills/ that holds a SKILL.md.
fn skill_files() -> Vec<String> {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugin/skills");
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
        let item = format!("include_str!(\"../../../plugin/skills/{f}\")");
        assert!(LIB.contains(&item), "lib.rs skill_docs lacks {f}");
    }
}

#[test]
fn every_fence_has_a_language() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugin/skills/");
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

const COMPONENT_MAP: &str =
    include_str!("../../../plugin/skills/build-ui/references/component-map.md");
const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn component_map_has_every_component() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/components");
    for e in std::fs::read_dir(dir).unwrap() {
        let f = e.unwrap().file_name().into_string().unwrap();
        let Some(module) = f.strip_suffix(".rs") else {
            continue;
        };
        if ["mod", "shared", "icon_raster", "atom_row", "context"].contains(&module) {
            continue;
        }
        let path = format!("shadcn_egui::components::{module}::");
        assert!(
            COMPONENT_MAP.contains(&path),
            "component-map.md has no row for {module}"
        );
    }
}

/// `"RadioGroup"` → `"radio_group"`.
fn snake_case(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// The component names of component-map.md's "Not yet ported:" sentence:
/// its paragraph up to the first full stop that ends a sentence (followed
/// by white space or the paragraph's end), so a dot inside a name does not
/// cut the list short.
fn not_yet_ported() -> Vec<String> {
    let start = COMPONENT_MAP
        .find("Not yet ported: ")
        .expect("component-map.md has a \"Not yet ported\" list")
        + "Not yet ported: ".len();
    let rest = &COMPONENT_MAP[start..];
    let paragraph = &rest[..rest.find("\n\n").unwrap_or(rest.len())];
    let paragraph = paragraph.trim_end();
    let end = paragraph
        .match_indices('.')
        .map(|(i, _)| i)
        .find(|&i| {
            paragraph[i + 1..]
                .chars()
                .next()
                .is_none_or(char::is_whitespace)
        })
        .expect("the list ends with a full stop");
    paragraph[..end]
        .split([',', '\n'])
        .flat_map(|item| item.split(" or "))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

#[test]
fn not_yet_ported_lists_no_ported_component() {
    // Entries that are not components and have no module of their own.
    const NOT_COMPONENTS: [&str; 1] = ["gradients"];
    let names = not_yet_ported();
    assert!(names.contains(&"RadioGroup".to_owned()), "parsed {names:?}");
    // Lower this when a listed component gets ported; a parse that loses
    // entries fails here.
    assert!(names.len() >= 8, "parsed only {names:?}");
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/components");
    for name in names {
        if NOT_COMPONENTS.contains(&name.as_str()) {
            continue;
        }
        let module = snake_case(&name);
        assert!(
            !std::path::Path::new(&format!("{dir}/{module}.rs")).exists(),
            "component-map.md lists {name} as not yet ported, but components/{module}.rs exists"
        );
        assert!(
            !COMPONENT_MAP.contains(&format!("shadcn_egui::components::{module}::")),
            "component-map.md lists {name} as not yet ported and maps it"
        );
    }
}

#[test]
fn snake_case_follows_module_names() {
    assert_eq!(snake_case("RadioGroup"), "radio_group");
    assert_eq!(snake_case("DropdownMenu"), "dropdown_menu");
    assert_eq!(snake_case("Tags"), "tags");
}

/// The skills say every exported style has snapshot tests; keep it so.
#[test]
fn every_exported_style_has_light_and_dark_snapshots() {
    use shadcn_egui::StyleName;
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/snapshots");
    let files: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    // Sera is not exported: a preset naming it falls back to nova.
    for style in StyleName::ALL.iter().filter(|s| **s != StyleName::Sera) {
        let id = style.id();
        for mode in ["light", "dark"] {
            let suffix = format!("_{id}_{mode}.png");
            assert!(
                files.iter().any(|f| f.ends_with(&suffix)),
                "no *{suffix} snapshot in tests/snapshots"
            );
        }
    }
}

#[test]
fn skills_name_only_existing_features() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugin/skills/");
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

/// A version written into skill text is not bumped by Create release PR;
/// the skills read it from plugin.json instead.
#[test]
fn skills_pin_no_crate_version() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../plugin/skills/");
    for f in skill_files() {
        let text = std::fs::read_to_string(format!("{root}{f}")).unwrap();
        for (i, _) in text.match_indices("shadcn-egui@") {
            let next = text[i + "shadcn-egui@".len()..].chars().next();
            assert!(
                !next.is_some_and(|c| c.is_ascii_digit()),
                "{f} pins a crate version: {}",
                &text[i..text.len().min(i + 20)]
            );
        }
    }
}
