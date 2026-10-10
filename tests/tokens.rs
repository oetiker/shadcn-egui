//! Token discipline: components take every color and text size from the
//! theme, so restyling means editing `theme.rs` alone.

/// Color constructors and literal font sizes. A line may opt out with a
/// `// token-ok: <reason>` comment on the line above.
fn violations(source: &str) -> Vec<String> {
    let patterns = [
        "Color32::from_",
        "Color32::WHITE",
        "Color32::BLACK",
        "Color32::GRAY",
        "Color32::RED",
        "from_black_alpha",
        "from_white_alpha",
    ];
    let mut out = Vec::new();
    let mut prev = "";
    for (n, line) in source.lines().enumerate() {
        let literal_size = ["FontId::new(", "FontId::proportional(", "FontId::monospace(", ".size("]
            .iter()
            .any(|p| line.split(p).skip(1).any(|rest| rest.starts_with(|c: char| c.is_ascii_digit())));
        let literal_color = patterns.iter().any(|p| line.contains(p));
        if (literal_size || literal_color) && !prev.contains("token-ok:") {
            out.push(format!("{}: {}", n + 1, line.trim()));
        }
        prev = line;
    }
    out
}

#[test]
fn scanner_catches_literals() {
    // Positive control: the scanner must flag each kind it claims to.
    assert_eq!(violations("let c = Color32::from_rgb(1, 2, 3);").len(), 1);
    assert_eq!(violations("let c = Color32::WHITE;").len(), 1);
    assert_eq!(violations("let f = FontId::proportional(12.0);").len(), 1);
    assert_eq!(violations("RichText::new(t).size(14.0)").len(), 1);
    assert_eq!(violations("// token-ok: test\nlet c = Color32::WHITE;").len(), 0);
    assert_eq!(violations("let f = FontId::proportional(t.metrics.text_xs);").len(), 0);
}

#[test]
fn components_use_theme_tokens() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut files: Vec<_> = std::fs::read_dir(format!("{dir}/components"))
        .expect("components dir")
        .map(|e| e.expect("dir entry").path())
        .collect();
    files.push(format!("{dir}/layout.rs").into());
    assert!(files.len() > 5, "scanned too few files: {files:?}");
    let mut found = Vec::new();
    for f in files {
        let src = std::fs::read_to_string(&f).expect("readable source");
        for v in violations(&src) {
            found.push(format!("{}:{v}", f.display()));
        }
    }
    assert!(found.is_empty(), "literal colors/sizes outside theme.rs:\n{}", found.join("\n"));
}

/// Strict scan for ported components (spec section 7): besides colours and
/// font sizes, literal geometry, shadow definitions and alpha multipliers. A
/// line may carry them only with a `// base: "<upstream classes>"` quote on
/// the same line or the line above (stylegen's drift check verifies the
/// quote). Halving/doubling (`/ 2.0`, `* 2.0`) and 0.0/1.0 are arithmetic,
/// not values.
fn strict_violations(source: &str) -> Vec<String> {
    let mut out = violations(source);
    let mut prev = "";
    for (n, line) in source.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        let quoted = line.contains("// base: \"") || prev.contains("// base: \"");
        let shadow = code.contains("Shadow {");
        let alpha = ["gamma_multiply(", "linear_multiply(", "from_black_alpha(", "from_white_alpha("]
            .iter()
            .any(|p| code.split(p).skip(1).any(|r| r.starts_with(|c: char| c.is_ascii_digit())));
        let geometry = float_literals(code).any(|(lit, before)| {
            !matches!(lit, "0.0" | "1.0") && !((lit == "2.0") && (before.ends_with("/ ") || before.ends_with("* ")))
        });
        if (shadow || alpha || geometry) && !quoted && !line.trim_start().starts_with("//") {
            out.push(format!("{}: {}", n + 1, line.trim()));
        }
        prev = line;
    }
    out
}

/// Float literals in a line, each with the text before it.
fn float_literals(code: &str) -> impl Iterator<Item = (&str, &str)> {
    let b = code.as_bytes();
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < b.len() {
            let start = i;
            let ident_before = start > 0 && (b[start - 1].is_ascii_alphanumeric() || b[start - 1] == b'_');
            if b[i].is_ascii_digit() && !ident_before {
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.' || b[i] == b'_') {
                    i += 1;
                }
                let lit = &code[start..i];
                if lit.contains('.') && !lit.ends_with('.') {
                    return Some((lit, &code[..start]));
                }
            } else {
                i += 1;
            }
        }
        None
    })
}

#[test]
fn strict_scanner_catches_literals() {
    assert_eq!(strict_violations("let h = 32.0;").len(), 1);
    assert_eq!(strict_violations("let c = fill.gamma_multiply(0.5);").len(), 1);
    assert_eq!(strict_violations("let s = Shadow { offset: [0, 1], blur: 3, spread: 0, color };").len(), 1);
    assert_eq!(strict_violations("// base: \"disabled:opacity-50\"\nlet c = fill.gamma_multiply(0.5);").len(), 0);
    assert_eq!(strict_violations("let c = rect.height() / 2.0;").len(), 0);
    assert_eq!(strict_violations("let w = 2.0 + x;").len(), 1);
    assert_eq!(strict_violations("let z = 0.0; let o = 1.0;").len(), 0);
    assert_eq!(strict_violations("let v = vec2(x, 0.0);").len(), 0);
}

/// The code before a file's trailing `#[cfg(test)] mod tests` module. Unit
/// tests compare against the style's numbers, so they hold literals by
/// design. Only a module that runs to the end of the file is cut: other
/// test-only items, and code after them, are scanned.
fn non_test_code(source: &str) -> &str {
    const MODULE: &str = "\n#[cfg(test)]\nmod tests {\n";
    let Some(at) = source.rfind(MODULE) else { return source };
    let body: Vec<&str> = source[at + MODULE.len()..].lines().filter(|l| !l.trim().is_empty()).collect();
    // Inside the module every line is indented; only its closing brace is not.
    let to_eof = body.split_last().is_some_and(|(last, rest)| *last == "}" && rest.iter().all(|l| l.starts_with(char::is_whitespace)));
    if to_eof { &source[..at] } else { source }
}

#[test]
fn non_test_code_cuts_only_the_trailing_test_module() {
    let tail = "let h = 32.0;\n#[cfg(test)]\nmod tests {\n    const H: f32 = 36.0;\n}\n";
    assert_eq!(strict_violations(non_test_code(tail)).len(), 1);
    assert_eq!(strict_violations(tail).len(), 2);
    // A test-only item mid-file exempts nothing after it.
    let mid = "\n#[cfg(test)]\nfn helper() {}\nlet h = 32.0;\n#[cfg(test)]\nmod tests {\n    const H: f32 = 36.0;\n}\n";
    assert_eq!(strict_violations(non_test_code(mid)).len(), 1);
    // A test module followed by more code is not the trailing module.
    let early = "#[cfg(test)]\nmod tests {\n    const H: f32 = 36.0;\n}\nlet h = 32.0;\n";
    assert_eq!(strict_violations(non_test_code(&format!("\n{early}"))).len(), 2);
}

/// Files under `src/components/` (plus `layout.rs`) already re-ported to the
/// shadcn-svelte base. Each port task adds its file; Task 24 replaces this
/// list with "all files".
const PORTED: &[&str] = &["button.rs", "badge.rs", "label.rs", "separator.rs", "input.rs", "checkbox.rs"];

#[test]
fn ported_components_take_values_from_the_style() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut found = Vec::new();
    for name in PORTED {
        let path = if *name == "layout.rs" { format!("{dir}/layout.rs") } else { format!("{dir}/components/{name}") };
        let src = std::fs::read_to_string(&path).expect("ported file exists");
        for v in strict_violations(non_test_code(&src)) {
            found.push(format!("{path}:{v}"));
        }
    }
    assert!(found.is_empty(), "style values hard-coded in ported components:\n{}", found.join("\n"));
}
