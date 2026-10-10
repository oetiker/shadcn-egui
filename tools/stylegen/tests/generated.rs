use stylegen::css::parse_style_css;
use stylegen::emit::{emit_struct, emit_style};
use stylegen::markers::parse_markers;
use stylegen::report::report;
use stylegen::resolve::markers;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/{path}")).expect(path)
}

fn shipped() -> Vec<String> {
    parse_markers(&read("markers.txt")).into_iter().flat_map(|l| l.markers).collect()
}

fn style_source(style: &str, pin: &str) -> String {
    let css = parse_style_css(&read(&format!("tests/fixtures/style-{style}.css")));
    emit_style(style, pin, &markers(&css, &shipped()))
}

/// The fixture CSS is a byte copy of the pinned upstream file, so the
/// committed crate sources must be exactly what the generator gives now,
/// and giving it twice must not change a byte.
#[test]
fn committed_sources_are_current_and_output_is_deterministic() {
    let pin = read("PIN").trim().to_owned();
    assert_eq!(emit_struct(&pin, &shipped()), read("../../src/style/generated.rs"), "re-run stylegen generate");
    for style in ["vega", "nova"] {
        let first = style_source(style, &pin);
        assert_eq!(first, style_source(style, &pin), "{style}: output differs between runs");
        assert_eq!(first, read(&format!("../../src/style/{style}.rs")), "{style}: re-run stylegen generate");
    }
}

/// Stale reports hide new unsupported classes, so all eight must be current.
#[test]
fn committed_reports_are_current() {
    let pin = read("PIN").trim().to_owned();
    for style in ["vega", "nova", "maia", "lyra", "mira", "luma", "sera", "rhea"] {
        let css = parse_style_css(&read(&format!("tests/fixtures/style-{style}.css")));
        let fresh = report(style, &pin, &markers(&css, &shipped()));
        assert_eq!(fresh, read(&format!("reports/{style}.md")), "{style}: re-run stylegen generate");
    }
}
