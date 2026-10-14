use stylegen::css::parse_style_css;
use stylegen::extract::{class_strings, normalise, part_strings, strip_cli_markers};
use stylegen::markers::parse_markers;
use stylegen::resolve::expand;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// Both sides spelled alike: CLI markers removed (the published JSON keeps
/// `cn-menu-target`, `cn-menu-translucent` and `cn-font-heading` as text),
/// `calc()` operators without underscores (`calc(100%_-_1px)`).
fn canonical(s: &str) -> String {
    normalise(&strip_cli_markers(s).replace("_-_", "-").replace("_+_", "+"))
}

/// Each part's concatenated class string (the style's classes in place of
/// the marker, then the base classes, before any merge) must be a class
/// string of the published component for that style. A part that may
/// differ is listed in published-allow.txt as `<style> <part>  # why`.
#[test]
fn part_strings_match_the_published_components() {
    // A missing allow list is an error, not an empty list: the test must
    // not pass on a checkout that lost the file.
    let allow = std::fs::read_to_string(format!("{ROOT}/tests/fixtures/published-allow.txt"))
        .expect("tests/fixtures/published-allow.txt");
    let allowed = |s: &str, p: &str| {
        allow.lines().any(|l| {
            l.split('#')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .eq([s, p])
        })
    };
    let lines = parse_markers(
        &std::fs::read_to_string(format!("{ROOT}/markers.txt")).expect("markers.txt"),
    );
    let parts = part_strings(
        &std::path::Path::new(ROOT).join("tests/fixtures/ui"),
        &lines,
    )
    .expect("parts");
    let mut missing = Vec::new();
    let mut compared = 0;
    for style in ["vega", "nova"] {
        let css = parse_style_css(
            &std::fs::read_to_string(format!("{ROOT}/tests/fixtures/style-{style}.css")).unwrap(),
        );
        for line in &lines {
            let path = format!("{ROOT}/tests/fixtures/published/{style}/{}.json", line.dir);
            let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
            let v: serde_json::Value = serde_json::from_str(&json).expect("json");
            let mut published: Vec<String> = Vec::new();
            for f in v["files"].as_array().expect("files") {
                let strings = class_strings(f["content"].as_str().unwrap_or(""))
                    .unwrap_or_else(|e| panic!("{path}: {e}"));
                published.extend(strings.iter().map(|s| canonical(s)));
            }
            for m in &line.markers {
                let (_, classes) = parts.iter().find(|(n, _)| n == m).expect("part");
                let ours = canonical(&expand(classes, m, &css).join(" "));
                compared += 1;
                if !published.contains(&ours) && !allowed(style, m) {
                    missing.push(format!("{style} {m}:\n  ours: {ours}"));
                }
            }
        }
    }
    // 72 parts in markers.txt, each for vega and nova.
    assert_eq!(compared, 144, "compared {compared} parts");
    assert!(
        missing.is_empty(),
        "not in published output:\n{}",
        missing.join("\n")
    );
}
