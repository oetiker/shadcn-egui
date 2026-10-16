//! icongen: SVG icons -> shadcn-egui icon data.
//!
//! icongen --out src/icons.rs [--module-path crate::components::icon]
//!         [--provenance "Lucide v1.54.0 (ISC)"] path/to/mic.svg ...
//!     Rust `Seg` slices, opaque strokes only (filled or translucent shapes are an error).
//! icongen --crate-dir crates/shadcn-egui-icons-x --slots check=..,chevron_down=..,close=..
//!         [--provenance TEXT] [--measure] path/to/*.svg
//!     A whole icon crate: DIR/src/lib.rs and DIR/src/icons.bin.
//! icongen --measure path/to/*.svg
//!     Print icon count, blob bytes and the largest icons; write nothing.
//!
//! Each icon is named after its file stem.

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: icongen --out FILE [--module-path PATH] [--provenance TEXT] SVG...\n       icongen --crate-dir DIR --slots check=I,chevron_down=I,close=I [--provenance TEXT] [--measure] SVG...\n       icongen --measure SVG...";

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
    let (mut out, mut crate_dir, mut slots, mut measure) = (None, None, None, false);
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
            "--crate-dir" => crate_dir = Some(value("--crate-dir")?),
            "--slots" => slots = Some(icongen::parse_slots(&value("--slots")?)?),
            "--measure" => measure = true,
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
    if out.is_some() && crate_dir.is_some() {
        return Err(format!("--out and --crate-dir exclude each other\n{USAGE}"));
    }
    if out.is_none() && crate_dir.is_none() && !measure {
        return Err(format!(
            "one of --out, --crate-dir, --measure is required\n{USAGE}"
        ));
    }
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
    if measure {
        print!("{}", icongen::measure(&icons)?);
    }
    if let Some(out) = out {
        for icon in &icons {
            icongen::ensure_stroke_only(icon)?;
        }
        std::fs::write(
            &out,
            icongen::render_module(&icons, &module_path, &provenance),
        )
        .map_err(|e| format!("{out}: {e}"))?;
        eprintln!("icongen: wrote {} icons to {out}", icons.len());
    }
    if let Some(dir) = crate_dir {
        let slots = slots.ok_or_else(|| format!("--crate-dir needs --slots\n{USAGE}"))?;
        let (src, blob) = icongen::render_crate(&icons, &slots, &provenance)?;
        let src_dir = Path::new(&dir).join("src");
        std::fs::create_dir_all(&src_dir).map_err(|e| format!("{}: {e}", src_dir.display()))?;
        std::fs::write(src_dir.join("lib.rs"), src)
            .map_err(|e| format!("{dir}/src/lib.rs: {e}"))?;
        std::fs::write(src_dir.join("icons.bin"), &blob)
            .map_err(|e| format!("{dir}/src/icons.bin: {e}"))?;
        eprintln!(
            "icongen: wrote {} icons ({} blob bytes) to {dir}",
            icons.len(),
            blob.len()
        );
    }
    Ok(())
}
