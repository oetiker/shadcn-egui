use stylegen::preset::{emit_preset, parse_preset};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/tests/fixtures/{p}")).expect(p)
}

fn parse() -> stylegen::preset::PresetLists {
    parse_preset(&read("preset.ts"), &read("config.ts")).expect("parse")
}

#[test]
fn lists_follow_upstream_order() {
    let l = parse();
    assert_eq!(
        l.styles,
        ["nova", "vega", "maia", "lyra", "mira", "luma", "sera", "rhea"]
    );
    assert_eq!(
        l.base_colors,
        ["neutral", "stone", "zinc", "gray", "mauve", "olive", "mist", "taupe"]
    );
    assert_eq!(l.themes.len(), 25);
    assert_eq!(
        (
            l.themes[0].as_str(),
            l.themes[3].as_str(),
            l.themes[4].as_str(),
            l.themes[24].as_str()
        ),
        ("neutral", "gray", "amber", "taupe")
    );
    assert_eq!(l.fonts.len(), 26);
    assert_eq!(
        (
            l.fonts[0].as_str(),
            l.fonts[17].as_str(),
            l.fonts[25].as_str()
        ),
        ("inter", "oxanium", "instrument-serif")
    );
    let radii: Vec<(&str, Option<f32>)> = l.radii.iter().map(|(n, r)| (n.as_str(), *r)).collect();
    assert_eq!(
        radii,
        [
            ("default", None),
            ("none", Some(0.0)),
            ("small", Some(0.45)),
            ("medium", Some(0.625)),
            ("large", Some(0.875))
        ]
    );
    assert_eq!(l.menu_accents, ["subtle", "bold"]);
    assert_eq!(
        l.menu_colors,
        [
            "default",
            "inverted",
            "default-translucent",
            "inverted-translucent"
        ]
    );
    assert_eq!(
        l.icon_libraries,
        ["lucide", "hugeicons", "tabler", "phosphor", "remixicon"]
    );
    let v1: Vec<(&str, &str)> = l
        .v1_chart_colors
        .iter()
        .map(|(t, c)| (t.as_str(), c.as_str()))
        .collect();
    assert_eq!(
        v1,
        [
            ("neutral", "blue"),
            ("stone", "lime"),
            ("zinc", "amber"),
            ("mauve", "emerald"),
            ("olive", "violet"),
            ("mist", "rose"),
            ("taupe", "cyan")
        ]
    );
    assert_eq!(l.fields_v1.iter().map(|f| f.1).sum::<u32>(), 40);
    assert_eq!(l.fields_v2.iter().map(|f| f.1).sum::<u32>(), 51);
    assert_eq!(l.fields_v2[0], ("menuColor".to_string(), 3));
    assert_eq!(l.fields_v2[8], ("chartColor".to_string(), 6));
    assert_eq!(l.fields_v2[9], ("fontHeading".to_string(), 5));
}

#[test]
fn a_list_too_long_for_its_bits_is_an_error() {
    let src = read("preset.ts").replace(
        "values: PRESET_MENU_COLORS, bits: 3 }",
        "values: PRESET_MENU_COLORS, bits: 1 }",
    );
    let err = parse_preset(&src, &read("config.ts")).unwrap_err();
    assert!(err.contains("menuColor"), "{err}");
}

#[test]
fn radii_must_match_between_preset_ts_and_config_ts() {
    let config = read("config.ts").replace(
        "{ name: \"small\", label: \"Small\", value: \"0.45rem\" },",
        "",
    );
    assert!(parse_preset(&read("preset.ts"), &config)
        .unwrap_err()
        .contains("RADII"));
}

#[test]
fn an_unknown_values_list_is_an_error() {
    let src = read("preset.ts").replace("values: PRESET_STYLES,", "values: PRESET_SHAPES,");
    assert!(parse_preset(&src, &read("config.ts"))
        .unwrap_err()
        .contains("PRESET_SHAPES"));
}

#[test]
fn emit_is_deterministic() {
    let l = parse();
    let a = emit_preset("c2a67849", &l);
    assert_eq!(a, emit_preset("c2a67849", &l));
    assert!(a.contains("    Gray,"));
    // Each enum's `ALL` lists every value: 25 theme colours, 26 fonts.
    let all_lens: Vec<usize> = a
        .lines()
        .filter(|l| l.contains("pub const ALL: &'static [Self] = &["))
        .map(|l| l.matches("Self::").count())
        .collect();
    assert!(
        all_lens.contains(&25) && all_lens.contains(&26),
        "{all_lens:?}"
    );
    assert!(a.contains("(ThemeColor::Neutral, ThemeColor::Blue),"));
    assert!(a.contains("Self::Default => None,"));
    assert!(a.contains("pub enum FontName {"));
    assert!(a.contains("Self::SourceSans3 => \"source-sans-3\","));
    assert!(a.contains("Px(f32),"));
    assert!(a.contains("pub(crate) const FIELDS_V2: [(Field, u32); 10] = ["));
}
