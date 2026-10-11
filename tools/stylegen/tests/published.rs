use stylegen::css::parse_style_css;
use stylegen::markers::parse_markers;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// Every class a style assigns to a shipped marker must appear in the
/// published component for that style, unless tailwind-merge dropped it in
/// favour of a base class, or upstream changed after the pin; each such case
/// is listed in published-allow.txt with its reason.
#[test]
fn style_classes_reach_the_published_components() {
    let allow = std::fs::read_to_string(format!("{ROOT}/tests/fixtures/published-allow.txt"))
        .unwrap_or_default();
    let allowed = |s: &str, m: &str, c: &str| {
        allow.lines().any(|l| {
            l.split('#')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .eq([s, m, c])
        })
    };
    let markers = parse_markers(
        &std::fs::read_to_string(format!("{ROOT}/markers.txt")).expect("markers.txt"),
    );
    let mut missing = Vec::new();
    for style in ["vega", "nova"] {
        let css = parse_style_css(
            &std::fs::read_to_string(format!("{ROOT}/tests/fixtures/style-{style}.css")).unwrap(),
        );
        for line in &markers {
            let path = format!("{ROOT}/tests/fixtures/published/{style}/{}.json", line.dir);
            let Ok(json) = std::fs::read_to_string(&path) else {
                continue;
            };
            let v: serde_json::Value = serde_json::from_str(&json).expect("json");
            let content: String = v["files"]
                .as_array()
                .expect("files")
                .iter()
                .map(|f| f["content"].as_str().unwrap_or("").to_owned())
                .collect::<Vec<_>>()
                .join("\n");
            let tokens: std::collections::HashSet<&str> = content
                .split(|c: char| c.is_whitespace() || c == '"' || c == '`')
                .collect();
            for m in &line.markers {
                let Some((_, classes)) = css.iter().find(|(n, _)| n == m) else {
                    continue;
                };
                for c in classes {
                    if !tokens.contains(c.as_str()) && !allowed(style, m, c) {
                        missing.push(format!("{style} {m} {c}"));
                    }
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "not in published output:\n{}",
        missing.join("\n")
    );
}
