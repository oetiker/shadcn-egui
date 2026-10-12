//! fontgen: turns `tools/fontgen/FONTS.tsv` and font files downloaded from
//! google/fonts (at the commit in `tools/fontgen/GF_PIN`) into the
//! `shadcn-egui-font-*` crates, the registry
//! `crates/shadcn-egui/src/fonts/registry.rs` and the font features.
//!
//! Every file it writes is generated: change FONTS.tsv or this tool and
//! regenerate, never edit the output.

use std::fmt::Write as _;
use std::path::Path;

/// Font licences that may be republished in a crate. Any other font gets no
/// crate and falls back to Inter.
pub const ALLOWED_LICENSES: [&str; 2] = ["OFL-1.1", "Apache-2.0"];

const HEADER: &str = "name\ttitle\tkind\tlicense\tfiles\tlicense_file\ty_offset_factor";
const WEIGHTS: [u16; 4] = [400, 500, 600, 700];
const BEGIN: &str = "# BEGIN fontgen\n";
const END: &str = "# END fontgen\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Sans,
    Serif,
    Mono,
}

impl Kind {
    fn parse(s: &str) -> Result<Kind, String> {
        match s {
            "sans" => Ok(Kind::Sans),
            "serif" => Ok(Kind::Serif),
            "mono" => Ok(Kind::Mono),
            other => Err(format!("unknown kind {other:?}")),
        }
    }

    /// The upstream `type` value.
    pub fn id(self) -> &'static str {
        match self {
            Kind::Sans => "sans",
            Kind::Serif => "serif",
            Kind::Mono => "mono",
        }
    }

    fn rust(self) -> &'static str {
        match self {
            Kind::Sans => "Sans",
            Kind::Serif => "Serif",
            Kind::Mono => "Mono",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Files {
    /// google/fonts path of the variable TTF.
    Variable(String),
    /// (CSS weight, google/fonts path) of each static TTF; 400 is always present.
    Static(Vec<(u16, String)>),
}

/// One FONTS.tsv row.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub name: String,
    pub title: String,
    pub kind: Kind,
    pub license: String,
    pub files: Files,
    pub license_file: String,
    /// `None` while the row says `-` (not measured yet).
    pub y_offset_factor: Option<f32>,
}

impl Row {
    pub fn shipped(&self) -> bool {
        ALLOWED_LICENSES.contains(&self.license.as_str())
    }

    pub fn crate_name(&self) -> String {
        format!("shadcn-egui-font-{}", self.name)
    }

    pub fn lib_name(&self) -> String {
        self.crate_name().replace('-', "_")
    }

    pub fn variant(&self) -> String {
        variant(&self.name)
    }

    /// (file inside the crate, google/fonts path) for each font file.
    pub fn targets(&self) -> Vec<(String, String)> {
        match &self.files {
            Files::Variable(p) => vec![(format!("fonts/{}.ttf", self.name), p.clone())],
            Files::Static(ws) => ws
                .iter()
                .map(|(w, p)| (format!("fonts/{}-{w}.ttf", self.name), p.clone()))
                .collect(),
        }
    }
}

/// The `FontName` variant of an upstream id: `source-sans-3` → `SourceSans3`.
pub fn variant(id: &str) -> String {
    id.split('-')
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

pub fn parse_fonts_tsv(text: &str) -> Result<Vec<Row>, String> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().ok_or("FONTS.tsv is empty")?;
    if header != HEADER {
        return Err(format!(
            "FONTS.tsv header is {header:?}, expected {HEADER:?}"
        ));
    }
    lines
        .enumerate()
        .map(|(i, line)| parse_row(line).map_err(|e| format!("FONTS.tsv line {}: {e}", i + 2)))
        .collect()
}

fn parse_row(line: &str) -> Result<Row, String> {
    let cols: Vec<&str> = line.split('\t').collect();
    let [name, title, kind, license, files, license_file, y] = cols[..] else {
        return Err(format!("expected 7 columns, got {}", cols.len()));
    };
    let files = if files.contains('=') {
        let mut ws = Vec::new();
        for part in files.split(';') {
            let (w, p) = part
                .split_once('=')
                .ok_or_else(|| format!("bad static entry {part:?}"))?;
            let w: u16 = w.parse().map_err(|_| format!("bad weight {w:?}"))?;
            if !WEIGHTS.contains(&w) || ws.iter().any(|(x, _)| *x == w) {
                return Err(format!(
                    "weight {w} is not one of 400/500/600/700 or appears twice"
                ));
            }
            ws.push((w, p.to_string()));
        }
        if !ws.iter().any(|(w, _)| *w == 400) {
            return Err("a static font needs a 400 file".into());
        }
        ws.sort();
        Files::Static(ws)
    } else {
        Files::Variable(files.to_string())
    };
    let y_offset_factor = match y {
        "-" => None,
        v => Some(
            v.parse::<f32>()
                .map_err(|_| format!("bad y_offset_factor {v:?}"))?,
        ),
    };
    Ok(Row {
        name: name.to_string(),
        title: title.to_string(),
        kind: Kind::parse(kind)?,
        license: license.to_string(),
        files,
        license_file: license_file.to_string(),
        y_offset_factor,
    })
}

/// One entry of upstream `FONT_DEFINITIONS`.
#[derive(Clone, Debug, PartialEq)]
pub struct Definition {
    pub name: String,
    pub title: String,
    pub kind: String,
    /// The `weight` list; empty for variable fonts.
    pub weights: Vec<u16>,
}

fn quoted(line: &str, key: &str) -> Option<String> {
    line.strip_prefix(key)?
        .strip_prefix('"')?
        .strip_suffix("\",")
        .map(str::to_string)
}

/// Line-based reader of `font-definitions.ts`: each `name: "…",` starts an
/// entry; `title`, `type` and `weight` lines fill it.
pub fn parse_font_definitions(ts: &str) -> Result<Vec<Definition>, String> {
    let mut defs: Vec<Definition> = Vec::new();
    for line in ts.lines().map(str::trim) {
        if let Some(name) = quoted(line, "name: ") {
            defs.push(Definition {
                name,
                title: String::new(),
                kind: String::new(),
                weights: Vec::new(),
            });
            continue;
        }
        let Some(d) = defs.last_mut() else { continue };
        if let Some(v) = quoted(line, "title: ") {
            d.title = v;
        } else if let Some(v) = quoted(line, "type: ") {
            d.kind = v;
        } else if let Some(rest) = line.strip_prefix("weight: [") {
            d.weights = rest
                .trim_end_matches(',')
                .trim_end_matches(']')
                .split(',')
                .map(|w| {
                    w.trim()
                        .trim_matches('"')
                        .parse::<u16>()
                        .map_err(|_| format!("{}: bad weight {w:?}", d.name))
                })
                .collect::<Result<_, _>>()?;
        }
    }
    if defs.is_empty() {
        return Err("font-definitions.ts: no `name: \"…\",` lines found".into());
    }
    Ok(defs)
}

/// FONTS.tsv and upstream agree on the set of fonts, each title and kind,
/// and the static weights.
pub fn check_definitions(rows: &[Row], ts: &str) -> Result<(), String> {
    let defs = parse_font_definitions(ts)?;
    let mut errors = Vec::new();
    for d in &defs {
        let Some(r) = rows.iter().find(|r| r.name == d.name) else {
            errors.push(format!(
                "{}: in font-definitions.ts but not in FONTS.tsv",
                d.name
            ));
            continue;
        };
        if r.title != d.title {
            errors.push(format!(
                "{}: title {:?} in FONTS.tsv, {:?} upstream",
                r.name, r.title, d.title
            ));
        }
        if r.kind.id() != d.kind {
            errors.push(format!(
                "{}: kind {} in FONTS.tsv, {} upstream",
                r.name,
                r.kind.id(),
                d.kind
            ));
        }
        let weights: Vec<u16> = match &r.files {
            Files::Variable(_) => Vec::new(),
            Files::Static(ws) => ws.iter().map(|(w, _)| *w).collect(),
        };
        if weights != d.weights {
            errors.push(format!(
                "{}: static weights {weights:?} in FONTS.tsv, {:?} upstream",
                r.name, d.weights
            ));
        }
    }
    for r in rows {
        if !defs.iter().any(|d| d.name == r.name) {
            errors.push(format!(
                "{}: in FONTS.tsv but not in font-definitions.ts",
                r.name
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

/// The file starts like a TrueType font (`00 01 00 00`), every table in its
/// directory lies inside the file, and skrifa reads it.
pub fn check_truetype(ttf: &[u8]) -> Result<(), String> {
    if ttf.get(..4) != Some(&[0, 1, 0, 0][..]) {
        return Err("not a TrueType font (header is not 00 01 00 00)".into());
    }
    let be16 = |at: usize| {
        ttf.get(at..at + 2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
    };
    let be32 = |at: usize| {
        ttf.get(at..at + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize)
    };
    let tables = be16(4).ok_or("truncated font: no table count")? as usize;
    for i in 0..tables {
        let rec = 12 + 16 * i;
        let (Some(offset), Some(length)) = (be32(rec + 8), be32(rec + 12)) else {
            return Err(format!(
                "truncated font: table directory ends at record {i}"
            ));
        };
        if offset.checked_add(length).is_none_or(|end| end > ttf.len()) {
            let tag = String::from_utf8_lossy(&ttf[rec..rec + 4]).into_owned();
            return Err(format!(
                "truncated font: table `{tag}` ends at {} but the file has {} bytes",
                offset.saturating_add(length),
                ttf.len()
            ));
        }
    }
    skrifa::FontRef::new(ttf)
        .map(|_| ())
        .map_err(|e| format!("unreadable font: {e}"))
}

/// The `wght` axis range of a variable TrueType font.
pub fn wght_range(ttf: &[u8]) -> Result<(f32, f32), String> {
    use skrifa::{MetadataProvider, Tag};
    check_truetype(ttf)?;
    let font = skrifa::FontRef::new(ttf).map_err(|e| format!("unreadable font: {e}"))?;
    let axis = font
        .axes()
        .iter()
        .find(|a| a.tag() == Tag::new(b"wght"))
        .ok_or("variable font without a wght axis")?;
    Ok((axis.min_value(), axis.max_value()))
}

pub fn crate_cargo_toml(row: &Row) -> String {
    format!(
        r#"[package]
name = "{crate_name}"
description = "{title} for shadcn-egui"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license = "MIT AND {license}"
repository.workspace = true

[dependencies]
shadcn-egui-core.workspace = true
"#,
        crate_name = row.crate_name(),
        title = row.title,
        license = row.license,
    )
}

/// `src/lib.rs` of a font crate, in the form of the Phase 2 inter and
/// oxanium crates. `wght` is the variable font's axis range (None for
/// static fonts).
pub fn crate_lib_rs(row: &Row, wght: Option<(f32, f32)>) -> Result<String, String> {
    let licence = match row.license.as_str() {
        "OFL-1.1" => "The font is licensed under the SIL Open Font\n//! License 1.1",
        "Apache-2.0" => "The font is licensed under the Apache License\n//! 2.0",
        other => return Err(format!("{}: licence {other} gets no crate", row.name)),
    };
    let (doc, source) = match (&row.files, wght) {
        (Files::Variable(_), Some((lo, hi))) => (
            format!("{}, variable weight {lo}–{hi}.", row.title),
            format!(
                "FontSource::Variable {{ data: include_bytes!(\"../fonts/{}.ttf\"), wght: {lo:?}..={hi:?} }}",
                row.name
            ),
        ),
        (Files::Static(ws), None) => {
            let file = |w: u16| {
                ws.iter().find(|(x, _)| *x == w).map(|_| format!("include_bytes!(\"../fonts/{}-{w}.ttf\")", row.name))
            };
            let opt = |w: u16| file(w).map_or_else(|| "None".to_string(), |b| format!("Some({b})"));
            let list: Vec<String> = ws.iter().map(|(w, _)| w.to_string()).collect();
            (
                format!("{}, static weights {}.", row.title, list.join(", ")),
                format!(
                    "FontSource::Static {{ regular: {}, medium: {}, semibold: {}, bold: {} }}",
                    file(400).ok_or("static font without 400")?,
                    opt(500),
                    opt(600),
                    opt(700)
                ),
            )
        }
        _ => {
            return Err(format!(
                "{}: a variable font needs its wght range and a static font must not have one",
                row.name
            ))
        }
    };
    let text = format!(
        r#"//! {title} for shadcn-egui. {licence} (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{{FontFiles, FontKind, FontSource}};

/// {doc}
pub static FONT: FontFiles = FontFiles {{
    name: "{name}",
    kind: FontKind::{kind},
    source: {source},
    y_offset_factor: {y:?},
    license: include_str!("../LICENSE-FONT.txt"),
}};
"#,
        title = row.title,
        name = row.name,
        kind = row.kind.rust(),
        y = row.y_offset_factor.unwrap_or(0.0),
    );
    rustfmt(&text)
}

/// `crates/shadcn-egui/src/fonts/registry.rs`.
pub fn registry_rs(rows: &[Row], gf_pin: &str) -> Result<String, String> {
    let mut arms = String::new();
    for r in rows.iter().filter(|r| r.shipped()) {
        if r.name != "inter" {
            writeln!(arms, "        #[cfg(feature = \"font-{}\")]", r.name).unwrap();
        }
        writeln!(
            arms,
            "        FontName::{} => Some(&{}::FONT),",
            r.variant(),
            r.lib_name()
        )
        .unwrap();
    }
    let unlicensed: Vec<String> = rows
        .iter()
        .filter(|r| !r.shipped())
        .map(|r| format!("FontName::{}", r.variant()))
        .collect();
    let unlicensed = unlicensed.join(", ");
    let text = format!(
        r#"// @generated by tools/fontgen from tools/fontgen/FONTS.tsv (google/fonts {gf_pin}).
// Do not edit: change FONTS.tsv or fontgen and regenerate.

use crate::{{FontFiles, FontName}};

/// The font files for `name`: Inter always, every other font when its
/// `font-<id>` feature is on. `None` means the feature is off, or the font
/// has no crate because its licence does not allow one.
pub fn files(name: FontName) -> Option<&'static FontFiles> {{
    match name {{
{arms}        #[allow(unreachable_patterns)]
        _ => None,
    }}
}}

/// Fonts with no crate: their licence is neither OFL-1.1 nor Apache-2.0.
pub(crate) const UNLICENSED: &[FontName] = &[{unlicensed}];
"#
    );
    rustfmt(&text)
}

fn optional(rows: &[Row]) -> impl Iterator<Item = &Row> {
    rows.iter().filter(|r| r.shipped() && r.name != "inter")
}

/// The block in `crates/shadcn-egui/Cargo.toml`: one optional dependency
/// subtable per font, then the `[features]` header and one feature per font.
/// The hand-written icon features follow the block and so belong to the
/// `[features]` table it opens.
pub fn features_block(rows: &[Row]) -> String {
    let mut s =
        format!("{BEGIN}# Generated by tools/fontgen from tools/fontgen/FONTS.tsv; do not edit.\n");
    for r in optional(rows) {
        write!(
            s,
            "\n[dependencies.{}]\nworkspace = true\noptional = true\n",
            r.crate_name()
        )
        .unwrap();
    }
    s.push_str("\n[features]\n");
    for r in optional(rows) {
        writeln!(s, "font-{} = [\"dep:{}\"]", r.name, r.crate_name()).unwrap();
    }
    s.push_str(END);
    s
}

/// The block inside the root `[workspace.dependencies]`.
pub fn workspace_deps_block(rows: &[Row], version: &str) -> String {
    let mut s = String::from(BEGIN);
    for r in rows.iter().filter(|r| r.shipped()) {
        let c = r.crate_name();
        writeln!(
            s,
            "{c} = {{ path = \"crates/{c}\", version = \"{version}\" }}"
        )
        .unwrap();
    }
    s.push_str(END);
    s
}

/// `version` of `[workspace.package]` in the root Cargo.toml.
pub fn workspace_version(root: &str) -> Result<String, String> {
    root.lines()
        .skip_while(|l| l.trim() != "[workspace.package]")
        .skip(1)
        .take_while(|l| !l.trim_start().starts_with('['))
        .find_map(|l| {
            l.trim()
                .strip_prefix("version = \"")?
                .strip_suffix('"')
                .map(str::to_string)
        })
        .ok_or_else(|| "root Cargo.toml has no [workspace.package] version".into())
}

/// Replaces the one `# BEGIN fontgen` … `# END fontgen` block (markers
/// included) by `block`.
pub fn replace_block(text: &str, block: &str) -> Result<String, String> {
    let start = text.find(BEGIN).ok_or("no `# BEGIN fontgen` marker")?;
    if text[start + BEGIN.len()..].contains(BEGIN) {
        return Err("more than one `# BEGIN fontgen` marker".into());
    }
    let end = text[start..]
        .find(END)
        .ok_or("no `# END fontgen` marker after `# BEGIN fontgen`")?
        + start
        + END.len();
    Ok(format!("{}{block}{}", &text[..start], &text[end..]))
}

/// Formats generated Rust like `cargo fmt` does. Stdin is written from a
/// separate thread so a large input cannot deadlock against rustfmt's
/// output pipes; rustfmt's stderr is part of every error.
pub fn rustfmt(src: &str) -> Result<String, String> {
    use std::io::Write as _;
    use std::process::{Command, Stdio};
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2021"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run rustfmt: {e}"))?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let input = src.to_string();
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let out = child
        .wait_with_output()
        .map_err(|e| format!("rustfmt: {e}"))?;
    let written = writer
        .join()
        .map_err(|_| "rustfmt stdin writer panicked".to_string())?;
    if !out.status.success() {
        return Err(format!(
            "rustfmt failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    written.map_err(|e| {
        format!(
            "writing to rustfmt: {e}; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        )
    })?;
    String::from_utf8(out.stdout).map_err(|e| e.to_string())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn read_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The whole run: every shipped font crate, the registry and both
/// `Cargo.toml` blocks. `downloads` mirrors google/fonts paths
/// (`tools/fontgen/fetch.sh` fills it).
pub fn generate(downloads: &Path, repo: &Path) -> Result<(), String> {
    let rows = parse_fonts_tsv(&read_text(&repo.join("tools/fontgen/FONTS.tsv"))?)?;
    let pin = read_text(&repo.join("tools/fontgen/GF_PIN"))?
        .trim()
        .to_string();
    check_definitions(
        &rows,
        &read_text(&repo.join("tools/stylegen/tests/fixtures/font-definitions.ts"))?,
    )?;
    if !rows.iter().any(|r| r.name == "inter" && r.shipped()) {
        return Err("inter must be shipped: it is the default font".into());
    }
    let root_toml_path = repo.join("Cargo.toml");
    let root_toml = read_text(&root_toml_path)?;
    let version = workspace_version(&root_toml)?;
    // Everything is read, validated and rendered first; files are written
    // only when no step can fail any more.
    let mut outputs: Vec<(std::path::PathBuf, Vec<u8>)> = Vec::new();
    for row in &rows {
        if !row.shipped() {
            println!(
                "skip {}: licence {} (falls back to Inter)",
                row.name, row.license
            );
            continue;
        }
        let dir = repo.join("crates").join(row.crate_name());
        let mut wght = None;
        for (target, gf_path) in row.targets() {
            let src = downloads.join(&gf_path);
            let bytes = std::fs::read(&src).map_err(|e| {
                format!("{}: {e} (run tools/fontgen/fetch.sh first)", src.display())
            })?;
            match row.files {
                Files::Variable(_) => {
                    wght = Some(wght_range(&bytes).map_err(|e| format!("{gf_path}: {e}"))?)
                }
                Files::Static(_) => {
                    check_truetype(&bytes).map_err(|e| format!("{gf_path}: {e}"))?
                }
            }
            outputs.push((dir.join(target), bytes));
        }
        let licence_src = downloads.join(&row.license_file);
        let licence =
            std::fs::read(&licence_src).map_err(|e| format!("{}: {e}", licence_src.display()))?;
        outputs.push((dir.join("LICENSE-FONT.txt"), licence));
        outputs.push((dir.join("Cargo.toml"), crate_cargo_toml(row).into_bytes()));
        outputs.push((
            dir.join("src/lib.rs"),
            crate_lib_rs(row, wght)?.into_bytes(),
        ));
    }
    outputs.push((
        repo.join("crates/shadcn-egui/src/fonts/registry.rs"),
        registry_rs(&rows, &pin)?.into_bytes(),
    ));
    let main_toml_path = repo.join("crates/shadcn-egui/Cargo.toml");
    let main_toml = replace_block(&read_text(&main_toml_path)?, &features_block(&rows))
        .map_err(|e| format!("{}: {e}", main_toml_path.display()))?;
    outputs.push((main_toml_path, main_toml.into_bytes()));
    let root_toml = replace_block(&root_toml, &workspace_deps_block(&rows, &version))
        .map_err(|e| format!("{}: {e}", root_toml_path.display()))?;
    outputs.push((root_toml_path, root_toml.into_bytes()));
    for (path, bytes) in &outputs {
        write(path, bytes)?;
    }
    for row in rows.iter().filter(|r| r.shipped()) {
        println!("crate {}", row.crate_name());
    }
    println!("registry and Cargo.toml blocks written");
    Ok(())
}
