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
        let literal_size = [
            "FontId::new(",
            "FontId::proportional(",
            "FontId::monospace(",
            ".size(",
        ]
        .iter()
        .any(|p| {
            line.split(p)
                .skip(1)
                .any(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        });
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
    assert_eq!(
        violations("// token-ok: test\nlet c = Color32::WHITE;").len(),
        0
    );
    assert_eq!(
        violations("let f = FontId::proportional(t.metrics.text_xs);").len(),
        0
    );
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
    assert!(
        found.is_empty(),
        "literal colors/sizes outside theme.rs:\n{}",
        found.join("\n")
    );
}

/// Strict scan for components: besides colours and font sizes, literal
/// geometry, shadow definitions, alpha multipliers and palette roles. Every
/// visual value comes from the style (a recipe or the theme metrics); a
/// line may carry one only with a `// behaviour-ok: <reason>` comment on
/// the same line, for behaviour constants and inherited fallbacks. `0.0` is
/// free; `1.0` and `2.0` only as factors (`* 2.0`, `/ 1.0`), not as a
/// length (`- 1.0`, `Stroke::new(1.0, ..)`). A `// base:` quote exempts
/// nothing and is reported itself: base classes live in the recipes.
fn strict_violations(source: &str) -> Vec<String> {
    let mut out = violations(source);
    let mut in_icon_data = false;
    for (n, line) in source.lines().enumerate() {
        // Lucide path data: `pub const NAME: Icon = Icon {` ... `};`.
        if line.starts_with("pub const ") && line.trim_end().ends_with(": Icon = Icon {") {
            in_icon_data = true;
        }
        if in_icon_data {
            in_icon_data = line.trim_end() != "};";
            continue;
        }
        let code = line.split("//").next().unwrap_or("");
        let reason = line
            .split("// behaviour-ok:")
            .nth(1)
            .is_some_and(|r| !r.trim().is_empty());
        let stale_quote = line.contains("// base:");
        let role = code.contains("Role::");
        let shadow = code.contains("Shadow {");
        let alpha = [
            "gamma_multiply(",
            "linear_multiply(",
            "from_black_alpha(",
            "from_white_alpha(",
        ]
        .iter()
        .any(|p| {
            code.split(p)
                .skip(1)
                .any(|r| r.starts_with(|c: char| c.is_ascii_digit()))
        });
        let geometry = float_literals(code).any(|(lit, before)| {
            let factor = before.trim_end().ends_with(['*', '/']);
            match lit {
                "0.0" => false,
                "1.0" | "2.0" => !factor,
                _ => true,
            }
        });
        let flagged = (role || shadow || alpha || geometry) && !reason;
        if stale_quote || (flagged && !line.trim_start().starts_with("//")) {
            out.push(format!("{}: {}", n + 1, line.trim()));
        }
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
            let ident_before =
                start > 0 && (b[start - 1].is_ascii_alphanumeric() || b[start - 1] == b'_');
            if b[i].is_ascii_digit() && !ident_before {
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.' || b[i] == b'_') {
                    i += 1;
                }
                if code[i..].starts_with("f32") || code[i..].starts_with("f64") {
                    i += 3;
                }
                // The value without its type suffix: `1.0_f32` is `1.0`.
                let lit = code[start..i]
                    .trim_end_matches("f32")
                    .trim_end_matches("f64")
                    .trim_end_matches('_');
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
    let n = |s: &str| strict_violations(s).len();
    assert_eq!(n("let h = 32.0;"), 1);
    assert_eq!(n("let c = fill.gamma_multiply(0.5);"), 1);
    assert_eq!(
        n("let s = Shadow { offset: [0, 1], blur: 3, spread: 0, color };"),
        1
    );
    // A base quote exempts nothing and is reported itself.
    assert_eq!(
        n("// base: \"disabled:opacity-50\"\nlet c = fill.gamma_multiply(0.5);"),
        2
    );
    assert_eq!(n("let c = rect.height() / 2.0;"), 0);
    assert_eq!(n("let k = x * 1.0;"), 0);
    assert_eq!(n("let w = 2.0 + x;"), 1);
    assert_eq!(n("let s = Stroke::new(1.0, c);"), 1);
    assert_eq!(n("let h = x - 1.0;"), 1);
    assert_eq!(n("let z = 0.0;"), 0);
    assert_eq!(n("let o = 1.0_f32; let p = 1.0f64;"), 1);
    assert_eq!(n("let h = 32.0_f32;"), 1);
    assert_eq!(n("let v = vec2(x, 0.0);"), 0);
    assert_eq!(n("let o = v.clamp(0.0, 1.0);"), 1);
    assert_eq!(
        n("let o = v.clamp(0.0, 1.0); // behaviour-ok: fraction of the range"),
        0
    );
    assert_eq!(n("let fg = t.role(Role::Foreground);"), 1);
    assert_eq!(
        n("let fg = t.role(Role::Foreground); // behaviour-ok: inherits Foreground"),
        0
    );
    assert_eq!(n("let t = 0.1; // behaviour-ok: flattening tolerance"), 0);
    assert_eq!(n("let t = 0.1; // behaviour-ok:"), 1);
    assert_eq!(
        n("pub const X: Icon = Icon {\n    a: 3.5,\n};\nlet h = 32.0;"),
        1
    );
}

/// The code before a file's trailing `#[cfg(test)] mod tests` module. Unit
/// tests compare against the style's numbers, so they hold literals by
/// design. Only a module that runs to the end of the file is cut: other
/// test-only items, and code after them, are scanned.
fn non_test_code(source: &str) -> &str {
    const MODULE: &str = "\n#[cfg(test)]\nmod tests {\n";
    let Some(at) = source.rfind(MODULE) else {
        return source;
    };
    let body: Vec<&str> = source[at + MODULE.len()..]
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    // Inside the module every line is indented; only its closing brace is not.
    let to_eof = body.split_last().is_some_and(|(last, rest)| {
        *last == "}" && rest.iter().all(|l| l.starts_with(char::is_whitespace))
    });
    if to_eof {
        &source[..at]
    } else {
        source
    }
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
    assert_eq!(
        strict_violations(non_test_code(&format!("\n{early}"))).len(),
        2
    );
}

#[test]
fn ported_components_take_values_from_the_style() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut files: Vec<_> = std::fs::read_dir(format!("{dir}/components"))
        .expect("components dir")
        .map(|e| e.expect("dir entry").path())
        .collect();
    files.push(format!("{dir}/layout.rs").into());
    assert!(files.len() > 5, "scanned too few files: {files:?}");
    let mut found = Vec::new();
    for path in files {
        // A rasterizer, not a component: its numbers are pixel arithmetic.
        if path.ends_with("icon_raster.rs") {
            continue;
        }
        let path = path.display().to_string();
        let src = std::fs::read_to_string(&path).expect("source file readable");
        for v in strict_violations(non_test_code(&src)) {
            found.push(format!("{path}:{v}"));
        }
    }
    assert!(
        found.is_empty(),
        "style values hard-coded in components:\n{}",
        found.join("\n")
    );
}
