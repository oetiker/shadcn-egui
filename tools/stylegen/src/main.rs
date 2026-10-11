//! stylegen generate <shadcn-svelte checkout> <repository root>
//! stylegen drift    <shadcn-svelte checkout> <repository root>

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use stylegen::{css, emit, markers, report, resolve};

/// The crate stylegen writes into, relative to the repository root.
const CRATE: &str = "crates/shadcn-egui";

/// Every upstream style; each gets a report, the ones in `EXPORT` also Rust.
const STYLES: &[&str] = &[
    "vega", "nova", "maia", "lyra", "mira", "luma", "sera", "rhea",
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (Some(cmd), Some(up), Some(root)) = (args.get(1), args.get(2), args.get(3)) else {
        eprintln!("usage: stylegen generate|drift <shadcn-svelte> <repo-root>");
        return ExitCode::from(2);
    };
    let (up, root) = (PathBuf::from(up), PathBuf::from(root));
    let tool = root.join("tools/stylegen");
    let pin = std::fs::read_to_string(tool.join("PIN"))
        .expect("PIN")
        .trim()
        .to_owned();
    check_pin(&up, &pin);
    let lines = markers::parse_markers(
        &std::fs::read_to_string(tool.join("markers.txt")).expect("markers.txt"),
    );
    match cmd.as_str() {
        "generate" => generate(&up, &root, &tool, &pin, &lines),
        "drift" => drift(&up, &root.join(CRATE), &lines),
        _ => {
            eprintln!("unknown command `{cmd}`");
            ExitCode::from(2)
        }
    }
}

fn check_pin(up: &Path, pin: &str) {
    let out = Command::new("git")
        .arg("-C")
        .arg(up)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git");
    let head = String::from_utf8_lossy(&out.stdout);
    assert!(
        head.starts_with(pin),
        "upstream checkout is at {head}, PIN says {pin}"
    );
}

fn generate(
    up: &Path,
    root: &Path,
    tool: &Path,
    pin: &str,
    lines: &[markers::MarkerLine],
) -> ExitCode {
    let shipped: Vec<String> = lines.iter().flat_map(|l| l.markers.clone()).collect();
    let export: Vec<String> = std::fs::read_to_string(tool.join("EXPORT"))
        .expect("EXPORT")
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    let out_dir = root.join(CRATE).join("src/style");
    let reports = tool.join("reports");
    std::fs::create_dir_all(&reports).expect("reports dir");
    write(
        &out_dir.join("generated.rs"),
        &emit::emit_struct(pin, &shipped),
    );
    // An export style that fails the gate must not stop the other reports.
    let mut failed = false;
    for style in STYLES {
        let path = up.join(format!("docs/src/lib/registry/styles/style-{style}.css"));
        let parsed = css::parse_style_css(&std::fs::read_to_string(&path).expect("style css"));
        let translated = resolve::markers(&parsed, &shipped);
        write(
            &reports.join(format!("{style}.md")),
            &report::report(style, pin, &translated),
        );
        if export.iter().any(|e| e == style) {
            let unsupported: usize = translated.iter().map(|(_, m)| m.unsupported.len()).sum();
            if unsupported != 0 {
                eprintln!("{style} is in EXPORT but has {unsupported} unsupported classes; see reports/{style}.md");
                failed = true;
                continue;
            }
            write(
                &out_dir.join(format!("{style}.rs")),
                &emit::emit_style(style, pin, &translated),
            );
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn write(path: &Path, src: &str) {
    std::fs::write(path, src).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn drift(up: &Path, root: &Path, lines: &[markers::MarkerLine]) -> ExitCode {
    let problems = stylegen::drift::check(up, root, lines);
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
