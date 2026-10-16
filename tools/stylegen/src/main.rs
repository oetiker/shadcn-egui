//! stylegen generate|drift <shadcn-svelte checkout> <shadcn-ui checkout> <repository root>

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use stylegen::pin::{check_pin, parse_pin};
use stylegen::{markers, preset, themes};

/// The crate stylegen writes into, relative to the repository root.
const CRATE: &str = "crates/shadcn-egui";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (Some(cmd), Some(up), Some(up_react), Some(root)) =
        (args.get(1), args.get(2), args.get(3), args.get(4))
    else {
        eprintln!("usage: stylegen generate|drift <shadcn-svelte> <shadcn-ui> <repo-root>");
        return ExitCode::from(2);
    };
    let (up, up_react, root) = (
        PathBuf::from(up),
        PathBuf::from(up_react),
        PathBuf::from(root),
    );
    let tool = root.join("tools/stylegen");
    let read_pin = |name: &str| {
        let text = std::fs::read_to_string(tool.join(name)).map_err(|e| format!("{name}: {e}"))?;
        parse_pin(&text).map_err(|e| format!("{name}: {e}"))
    };
    let (pin, pin_react) = match (read_pin("PIN"), read_pin("PIN_REACT")) {
        (Ok(a), Ok(b)) => (a, b),
        (a, b) => {
            for e in [a.err(), b.err()].into_iter().flatten() {
                eprintln!("{e}");
            }
            return ExitCode::FAILURE;
        }
    };
    for (checkout, pin) in [(&up, &pin), (&up_react, &pin_react)] {
        if let Err(e) = check_pin(checkout, pin) {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    }
    let lines = markers::parse_markers(
        &std::fs::read_to_string(tool.join("markers.txt")).expect("markers.txt"),
    );
    match cmd.as_str() {
        "generate" => generate(&up, &up_react, &root, &tool, &pin, &pin_react, &lines),
        "drift" => drift(&up, &up_react, &tool, &lines),
        _ => {
            eprintln!("unknown command `{cmd}`");
            ExitCode::from(2)
        }
    }
}

fn generate(
    up: &Path,
    up_react: &Path,
    root: &Path,
    tool: &Path,
    pin: &str,
    pin_react: &str,
    lines: &[markers::MarkerLine],
) -> ExitCode {
    stylegen::sources::refresh(up, up_react, tool, lines)
        .expect("copy the upstream sources into tests/fixtures");
    let out_dir = root.join(CRATE).join("src/style");
    let reports = tool.join("reports");
    std::fs::create_dir_all(&reports).expect("reports dir");
    let built = match stylegen::build::styles(tool, pin) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    write(&out_dir.join("generated.rs"), &built.generated);
    for (style, src) in &built.exported {
        write(&out_dir.join(format!("{style}.rs")), src);
    }
    for (style, md) in &built.reports {
        write(&reports.join(format!("{style}.md")), md);
    }
    // An export style that fails the gate must not stop the other outputs.
    let mut failed = false;
    for (style, n) in &built.failed {
        eprintln!("{style} is in EXPORT but has {n} unsupported classes; see reports/{style}.md");
        failed = true;
    }
    let palettes = themes::parse_themes(
        &read(&tool.join("tests/fixtures/themes.ts")),
        &read(&tool.join("tests/fixtures/base-colors.ts")),
    )
    .and_then(|t| themes::emit_palettes(pin_react, &t));
    match palettes {
        Ok(src) => {
            let dir = root.join(CRATE).join("src/palette");
            std::fs::create_dir_all(&dir).expect("palette dir");
            write(&dir.join("generated.rs"), &src);
        }
        Err(e) => {
            eprintln!("palettes: {e}");
            failed = true;
        }
    }
    let enums = preset::parse_preset(
        &read(&tool.join("tests/fixtures/preset.ts")),
        &read(&tool.join("tests/fixtures/config.ts")),
    )
    .map(|l| preset::emit_preset(pin_react, &l));
    match enums {
        Ok(src) => {
            let dir = root.join(CRATE).join("src/preset");
            std::fs::create_dir_all(&dir).expect("preset dir");
            write(&dir.join("generated.rs"), &src);
        }
        Err(e) => {
            eprintln!("preset: {e}");
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn write(path: &Path, src: &str) {
    std::fs::write(path, src).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn drift(up: &Path, up_react: &Path, tool: &Path, lines: &[markers::MarkerLine]) -> ExitCode {
    let mut problems = stylegen::drift::check(up, lines);
    problems.extend(stylegen::drift::check_sources(up, up_react, tool));
    problems.extend(stylegen::drift::check_ui(up, tool, lines));
    for p in &problems {
        println!("{p}");
    }
    if problems.is_empty() {
        println!("drift: clean");
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
