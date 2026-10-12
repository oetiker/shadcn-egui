use shadcn_egui::{
    BaseColor, FontHeading, FontName, IconLibrary, MenuAccent, MenuColor, Preset, PresetCodeError,
    RadiusChoice, StyleName, ThemeColor,
};

const FIXTURE: &str = include_str!("fixtures/preset_codes.txt");

fn lines() -> impl Iterator<Item = Vec<&'static str>> {
    FIXTURE
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split(' ').collect())
}

/// Column 5 holds the chart colour the React site uses, also for `a` codes
/// (`V1_CHART_COLOR_MAP[theme] ?? theme`, applied by the fixture script).
fn expected(w: &[&str]) -> Preset {
    Preset {
        style: StyleName::from_id(w[1]).expect("style"),
        base_color: BaseColor::from_id(w[2]).expect("base"),
        theme: ThemeColor::from_id(w[3]).expect("theme"),
        chart_color: ThemeColor::from_id(w[4]).expect("chart"),
        icons: IconLibrary::from_id(w[5]).expect("icons"),
        font: FontName::from_id(w[6]).expect("font"),
        font_heading: if w[7] == "inherit" {
            FontHeading::Inherit
        } else {
            FontHeading::Font(FontName::from_id(w[7]).expect("heading"))
        },
        radius: RadiusChoice::from_id(w[8]).expect("radius"),
        menu_accent: MenuAccent::from_id(w[9]).expect("accent"),
        menu_color: MenuColor::from_id(w[10]).expect("menu"),
    }
}

#[test]
fn every_fixture_code_decodes_like_upstream() {
    let mut n = 0;
    for w in lines() {
        n += 1;
        match w[1] {
            "null" => assert!(
                Preset::from_code(w[0]).is_err(),
                "{}: upstream returns null",
                w[0]
            ),
            "toolong" => assert_eq!(
                Preset::from_code(w[0]),
                Err(PresetCodeError::TooLong),
                "{}",
                w[0]
            ),
            _ => assert_eq!(Preset::from_code(w[0]), Ok(expected(&w)), "{}", w[0]),
        }
    }
    assert!(n >= 70, "only {n} fixture codes");
}

#[test]
fn canonical_codes_round_trip() {
    let mut n = 0;
    for w in lines().filter(|w| w.len() == 12 && w[11] == "1") {
        let p = Preset::from_code(w[0]).expect("decodes");
        assert_eq!(p.to_code().as_deref(), Some(w[0]));
        n += 1;
    }
    assert!(n >= 64, "only {n} canonical codes");
}

/// Review Focus 1: pasted junk is an error, never a panic or a wrong preset.
#[test]
fn malformed_codes_are_errors_not_panics() {
    for s in [
        "", " b0", "b0\n", "b0 ", "bé", "é", "😀", "b😀", "b", "a", "B0", "b0.", "b-1",
    ] {
        assert!(Preset::from_code(s).is_err(), "{s:?} must be an error");
    }
    assert_eq!(
        Preset::from_code(&format!("b{}", "Z".repeat(999))),
        Err(PresetCodeError::TooLong)
    );
    for a in 0u8..128 {
        assert!(Preset::from_code(&char::from(a).to_string()).is_err());
        for b in 0u8..128 {
            let s: String = [char::from(a), char::from(b)].iter().collect();
            let valid = matches!(a, b'a' | b'b') && b.is_ascii_alphanumeric();
            assert_eq!(Preset::from_code(&s).is_ok(), valid, "{s:?}");
        }
    }
}
