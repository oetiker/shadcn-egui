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
/// length (`- 1.0`, `Stroke::new(1.0, ..)`). The same holds for an integer
/// inside an egui geometry constructor ([`GEOMETRY`]), which takes integers
/// (`CornerRadius::same(4)`). A `// base:` quote exempts nothing and is
/// reported itself: base classes live in the recipes. String literals and
/// `/* .. */` comments hold no code; a block comment is stripped only on
/// one line, so the lines inside a multi-line one are scanned as code.
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
        let (code, comment) = split_line(line);
        let code = code.as_str();
        let reason = comment
            .split("// behaviour-ok:")
            .nth(1)
            .is_some_and(|r| !r.trim().is_empty());
        let stale_quote = comment.contains("// base:");
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
        let free = |n: &Number| {
            let factor = n.before.trim_end().ends_with(['*', '/']);
            n.value == 0.0 || (factor && (n.value == 1.0 || n.value == 2.0))
        };
        let geometry = numbers(code).any(|n| n.float && !free(&n));
        let integer_geometry =
            constructor_args(code).any(|args| numbers(args).any(|n| !n.float && !free(&n)));
        let flagged = (role || shadow || alpha || geometry || integer_geometry) && !reason;
        if stale_quote || flagged {
            out.push(format!("{}: {}", n + 1, line.trim()));
        }
    }
    out
}

/// A line split into its code, with string and char literals blanked and
/// one-line `/* .. */` comments removed, and its `//` comment (empty if none).
fn split_line(line: &str) -> (String, &str) {
    let b = line.as_bytes();
    let mut code = String::with_capacity(line.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => return (code, &line[i..]),
            b'/' if b.get(i + 1) == Some(&b'*') => {
                // An unclosed block comment runs to the end of the line.
                i = line[i + 2..].find("*/").map_or(b.len(), |j| i + 2 + j + 2);
                code.push(' ');
            }
            b'"' => {
                let mut j = i + 1;
                while j < b.len() && b[j] != b'"' {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
                code.push_str("\"\"");
                i = j + 1;
            }
            // A char literal (`'"'`, `'\\''`), not a lifetime (`'a`).
            b'\'' if b.get(i + 2) == Some(&b'\'') || b.get(i + 1) == Some(&b'\\') => {
                let close = line[i + 2..].find('\'').map_or(b.len(), |j| i + 2 + j);
                code.push_str("' '");
                i = close + 1;
            }
            _ => {
                let c = line[i..].chars().next().expect("char boundary");
                code.push(c);
                i += c.len_utf8();
            }
        }
    }
    (code, "")
}

/// Every numeric literal outside an identifier (`u8`, `vec2`) and a tuple
/// index (`pad.1`).
struct Number<'a> {
    /// Has a `.` or an `f32`/`f64` suffix (`4.`, `1.5`, `4f32`).
    float: bool,
    value: f64,
    /// The code before the literal.
    before: &'a str,
}

fn numbers(code: &str) -> impl Iterator<Item = Number<'_>> {
    let b = code.as_bytes();
    let digits = |mut i: usize| {
        while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'_') {
            i += 1;
        }
        i
    };
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < b.len() {
            let start = i;
            let ident_before = start > 0
                && (b[start - 1].is_ascii_alphanumeric() || matches!(b[start - 1], b'_' | b'.'));
            if !b[i].is_ascii_digit() || ident_before {
                i += 1;
                continue;
            }
            // `0x10`, `0o17`, `0b100`: an integer in another radix.
            let radix = match (b[i], b.get(i + 1)) {
                (b'0', Some(b'x')) => 16,
                (b'0', Some(b'o')) => 8,
                (b'0', Some(b'b')) => 2,
                _ => 10,
            };
            if radix != 10 {
                i += 2;
                while i < b.len() && (b[i].is_ascii_hexdigit() || b[i] == b'_') {
                    i += 1;
                }
                let text = code[start + 2..i].replace('_', "");
                let value = u64::from_str_radix(&text, radix).map_or(f64::NAN, |v| v as f64);
                return Some(Number {
                    float: false,
                    value,
                    before: &code[..start],
                });
            }
            i = digits(i);
            let mut float = false;
            // `4.` and `4.5`, but not a range (`0..n`) or a method (`1.max(x)`).
            let after_dot = b.get(i + 1).copied();
            if b.get(i) == Some(&b'.')
                && !after_dot.is_some_and(|c| c == b'.' || c.is_ascii_alphabetic() || c == b'_')
            {
                float = true;
                i = digits(i + 1);
            }
            let text = code[start..i].replace('_', "");
            for suffix in ["f32", "f64"] {
                if code[i..].starts_with(suffix) {
                    float = true;
                    i += suffix.len();
                }
            }
            let value = text.trim_end_matches('.').parse().unwrap_or(f64::NAN);
            return Some(Number {
                float,
                value,
                before: &code[..start],
            });
        }
        None
    })
}

/// The egui geometry constructors whose arguments may be integers.
const GEOMETRY: &[&str] = &[
    "CornerRadius",
    "Margin",
    "Stroke",
    "Shadow",
    "vec2",
    "pos2",
    "Vec2::new",
    "Pos2::new",
];

/// The argument text of each [`GEOMETRY`] call or struct literal in `code`
/// (`CornerRadius::same(..)`, `Margin { .. }`, `vec2(..)`), to the bracket
/// closing it or to the end of the line.
fn constructor_args(code: &str) -> impl Iterator<Item = &str> {
    GEOMETRY.iter().flat_map(move |name| {
        code.match_indices(name).filter_map(move |(at, _)| {
            let b = code.as_bytes();
            if at > 0 && (b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_') {
                return None;
            }
            // A type position (`-> Margin {`, `impl Margin {`, `for Margin {`)
            // opens a body, not a struct literal.
            let before = code[..at].trim_end();
            let keyword = |k: &str| {
                before.strip_suffix(k).is_some_and(|head| {
                    !head.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
                })
            };
            if before.ends_with("->") || keyword("impl") || keyword("for") {
                return None;
            }
            let mut rest = &code[at + name.len()..];
            // `CornerRadius::same(` and the like: skip the associated fn.
            if let Some(tail) = rest.strip_prefix("::") {
                let len = tail
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(tail.len());
                rest = &tail[len..];
            }
            let rest = rest.trim_start();
            if !rest.starts_with(['(', '{']) {
                return None;
            }
            let mut depth = 0;
            for (i, c) in rest.char_indices() {
                match c {
                    '(' | '{' | '[' => depth += 1,
                    ')' | '}' | ']' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(&rest[1..i]);
                        }
                    }
                    _ => {}
                }
            }
            Some(&rest[1..])
        })
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
    // A `//` inside a string literal starts no comment.
    assert_eq!(n("ui.hyperlink(\"http://x\"); let h = 32.0;"), 1);
    assert_eq!(n("let s = \"// behaviour-ok: quoted\"; let h = 32.0;"), 1);
    // A float with a trailing dot, or an integer with a float suffix.
    assert_eq!(n("let v = vec2(4., 2.);"), 1);
    assert_eq!(n("let h = 4f32;"), 1);
    assert_eq!(n("for i in 0..n {}"), 0);
    // Integer geometry in the egui constructors that take integers.
    assert_eq!(n("let r = CornerRadius::same(4);"), 1);
    assert_eq!(n("let m = Margin::same(8);"), 1);
    assert_eq!(n("let m = Margin::symmetric(0, 6);"), 1);
    assert_eq!(n("let r = CornerRadius { nw: 2, ..r };"), 1);
    assert_eq!(n("let r = CornerRadius::same(0);"), 0);
    assert_eq!(n("let r = CornerRadius::same(r.round() as u8);"), 0);
    assert_eq!(n("let p = pos2(x + pad.1, y);"), 0);
    assert_eq!(
        n("let m = Margin::same(8); // behaviour-ok: egui default"),
        0
    );
    // A type position is no struct literal: the body after it is not
    // constructor arguments.
    assert_eq!(n("fn m(x: u8) -> Margin { Margin::same(x) }"), 0);
    assert_eq!(n("fn m() -> Margin { let n = 3; Margin::same(n) }"), 0);
    assert_eq!(n("impl Margin { fn f() { g(3) } }"), 0);
    assert_eq!(n("impl From<u8> for Margin { fn from(n: u8) { g(3) } }"), 0);
    assert_eq!(n("fn m() -> Margin { Margin::same(8) }"), 1);
    // Hex, octal and binary integers are numbers too.
    assert_eq!(n("let r = CornerRadius::same(0x10);"), 1);
    assert_eq!(n("let m = Margin::same(0b100);"), 1);
    assert_eq!(n("let r = CornerRadius::same(0x0);"), 0);
    assert_eq!(n("let c = 0xff;"), 0);
    // A `/* .. */` comment on one line holds no code.
    assert_eq!(n("let w = /* 4.0 wide */ width;"), 0);
    assert_eq!(n("let h = 32.0; /* ok */"), 1);
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
