//! The README's example code and feature list stay true.

use shadcn_egui::Preset;

const README: &str = include_str!("../../../README.md");
const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn readme_preset_code_decodes_and_is_canonical() {
    let key = "Preset::from_code(\"";
    let start = README.find(key).expect("README shows Preset::from_code") + key.len();
    let code = &README[start..start + README[start..].find('"').unwrap()];
    let preset = Preset::from_code(code).unwrap_or_else(|e| panic!("README code {code}: {e}"));
    assert_eq!(
        preset.to_code().as_deref(),
        Some(code),
        "README shows a canonical code"
    );
}

#[test]
fn readme_names_every_icon_feature_and_the_all_features() {
    let icon_features = MANIFEST
        .lines()
        .filter_map(|l| l.split_once(" = [").map(|(k, _)| k.trim()))
        .filter(|k| k.starts_with("icons-"));
    for f in icon_features.chain(["fonts-all"]) {
        assert!(
            README.contains(&format!("`{f}`")),
            "README does not mention `{f}`"
        );
    }
}
