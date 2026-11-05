//! stylegen generate|drift <shadcn-svelte checkout> <shadcn-ui checkout> <repository root>

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use stylegen::markers;
use stylegen::pin::{check_pin, parse_pin};

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
    match stylegen::generate::generate(tool, root, pin, pin_react) {
        Ok(()) => ExitCode::SUCCESS,
        Err(errors) => {
            for e in errors {
                eprintln!("{e}");
            }
            ExitCode::FAILURE
        }
    }
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
