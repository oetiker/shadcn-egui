//! icongen: SVG icons -> shadcn_egui icon constants.
//!
//! icongen --out src/icons.rs [--module-path crate::components::icon]
//!         [--provenance "Lucide v1.54.0 (ISC)"] path/to/mic.svg path/to/power.svg ...
//!
//! Each icon is named after its file stem.

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: icongen --out FILE [--module-path PATH] [--provenance TEXT] SVG...";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("icongen: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut out = None;
    let mut module_path = "crate::components::icon".to_owned();
    let mut provenance = String::new();
    let mut inputs = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .ok_or_else(|| format!("{flag} needs a value\n{USAGE}"))
        };
        match a.as_str() {
            "--out" => out = Some(value("--out")?),
            "--module-path" => module_path = value("--module-path")?,
            "--provenance" => provenance = value("--provenance")?,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            f if f.starts_with("--") => return Err(format!("unknown flag {f}\n{USAGE}")),
            _ => inputs.push(a),
        }
    }
    let out = out.ok_or_else(|| format!("--out is required\n{USAGE}"))?;
    if inputs.is_empty() {
        return Err(format!("no SVG files given\n{USAGE}"));
    }

    let mut icons = Vec::new();
    let mut seen = std::collections::HashMap::new();
    for input in &inputs {
        let path = Path::new(input);
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("{input}: no file name"))?;
        let svg = std::fs::read_to_string(path).map_err(|e| format!("{input}: {e}"))?;
        let icon = icongen::convert(name, &svg)?;
        if let Some(prev) = seen.insert(icongen::const_name(name), input.clone()) {
            return Err(format!(
                "{input} and {prev} both map to constant {}",
                icongen::const_name(name)
            ));
        }
        icons.push(icon);
    }
    std::fs::write(
        &out,
        icongen::render_module(&icons, &module_path, &provenance),
    )
    .map_err(|e| format!("{out}: {e}"))?;
    eprintln!("icongen: wrote {} icons to {out}", icons.len());
    Ok(())
}
