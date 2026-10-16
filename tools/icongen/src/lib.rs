//! Convert SVG icons (stroked or filled) into icon data for shadcn-egui.
//!
//! The input is an icon set such as Lucide (stroked outlines) or Phosphor
//! (filled outlines). usvg resolves shapes, arcs and transforms into
//! move/line/quad/cubic segments. Stroked paths become `Seg` values; filled
//! paths are flattened and triangulated (see `fill`). An element with an
//! opacity below 1 goes into a translucent layer of its own. `--out` writes
//! Rust `Seg` slices (opaque strokes only); `--crate-dir` writes a whole icon
//! crate as a binary blob (see `blob`). Images, text and non-square view boxes are
//! errors, not silent omissions.

use usvg::tiny_skia_path::{PathSegment, Point};

pub mod blob;
pub mod fill;

pub use fill::{Fill, FillPath, FillRule};

/// Mirror of `shadcn_egui_core::Seg`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    M(f32, f32),
    L(f32, f32),
    Q(f32, f32, f32, f32),
    C(f32, f32, f32, f32, f32, f32),
    Z,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Converted {
    /// The icon's name as given (usually the SVG file stem), e.g. `circle-alert`.
    pub name: String,
    /// Side of the square view box; `segs` are in these units.
    pub size: f32,
    /// Stroked subpaths.
    pub segs: Vec<Seg>,
    /// Filled paths, tessellated, in the 24-unit box and quantised.
    pub fill: Option<Fill>,
    /// Translucent layers, drawn over `segs` and `fill` in document order.
    pub layers: Vec<Layer>,
}

/// Strokes and fill painted together at one opacity: an SVG element with
/// `opacity`, `stroke-opacity` or `fill-opacity` below 1.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    /// The product of the element's and its ancestors' opacities, in 0..1.
    pub alpha: f32,
    pub segs: Vec<Seg>,
    pub fill: Option<Fill>,
}

/// One layer's strokes and fill paths, before tessellation.
struct Gather {
    alpha: f32,
    segs: Vec<Seg>,
    fills: Vec<FillPath>,
}

impl Gather {
    fn new(alpha: f32) -> Self {
        Gather {
            alpha,
            segs: Vec::new(),
            fills: Vec::new(),
        }
    }
}

/// Convert one SVG document. `name` becomes the icon's accessible name and,
/// upper-cased, its constant name.
pub fn convert(name: &str, svg: &str) -> Result<Converted, String> {
    let tree =
        usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| format!("{name}: {e}"))?;
    let size = tree.size();
    if size.width() != size.height() {
        return Err(format!(
            "{name}: view box is {}x{}, icons must be square",
            size.width(),
            size.height()
        ));
    }
    // Layer 0 holds everything opaque; every translucent element opens its own.
    let mut gathered = vec![Gather::new(1.0)];
    walk(
        name,
        tree.root(),
        24.0 / size.width(),
        1.0,
        0,
        &mut gathered,
    )?;
    let mut gathered = gathered.into_iter();
    let base = gathered.next().expect("layer 0");
    let fill = fill::build(name, &base.fills)?;
    let mut layers = Vec::new();
    for g in gathered {
        let fill = fill::build(name, &g.fills)?;
        if !g.segs.is_empty() || fill.is_some() {
            layers.push(Layer {
                alpha: g.alpha,
                segs: g.segs,
                fill,
            });
        }
    }
    if base.segs.is_empty() && fill.is_none() && layers.is_empty() {
        return Err(format!("{name}: no stroked or filled paths"));
    }
    Ok(Converted {
        name: name.to_owned(),
        size: size.width(),
        segs: base.segs,
        fill,
        layers,
    })
}

/// The layer to paint into at `alpha` (the opacity of `layers[current]`)
/// times `own`: `current` when the product is full at the blob's alpha
/// precision, `None` when it is zero there (invisible), else a new layer.
fn layer_for(layers: &mut Vec<Gather>, current: usize, alpha: f32, own: f32) -> Option<usize> {
    let product = alpha * own;
    let q = (product * blob::ALPHA_SCALE).round();
    if q < 1.0 {
        return None;
    }
    if own >= 1.0 || q >= blob::ALPHA_SCALE {
        return Some(current);
    }
    layers.push(Gather::new(product));
    Some(layers.len() - 1)
}

/// Collect `group`'s paths into `layers[layer]`, or into new layers where
/// an opacity below 1 starts one. `alpha` is the opacity of `layers[layer]`.
fn walk(
    name: &str,
    group: &usvg::Group,
    scale: f32,
    alpha: f32,
    layer: usize,
    layers: &mut Vec<Gather>,
) -> Result<(), String> {
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => {
                if let Some(l) = layer_for(layers, layer, alpha, g.opacity().get()) {
                    let a = layers[l].alpha;
                    walk(name, g, scale, a, l, layers)?;
                }
            }
            usvg::Node::Path(p) => {
                if !p.is_visible() {
                    continue;
                }
                if let Some(f) = p.fill() {
                    let rule = match f.rule() {
                        usvg::FillRule::NonZero => FillRule::NonZero,
                        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
                    };
                    let contours = fill::flatten(p.data(), p.abs_transform(), scale);
                    let l = layer_for(layers, layer, alpha, f.opacity().get());
                    if let (Some(l), false) = (l, contours.is_empty()) {
                        layers[l].fills.push(FillPath { rule, contours });
                    }
                }
                if let Some(s) = p.stroke() {
                    if let Some(l) = layer_for(layers, layer, alpha, s.opacity().get()) {
                        path_segs(p.data(), p.abs_transform(), &mut layers[l].segs);
                    }
                }
            }
            usvg::Node::Image(_) => return Err(format!("{name}: embedded image not supported")),
            usvg::Node::Text(_) => return Err(format!("{name}: text not supported")),
        }
    }
    Ok(())
}

/// `--out` writes `Seg` slices, which cannot hold a fill or an opacity.
pub fn ensure_stroke_only(icon: &Converted) -> Result<(), String> {
    if icon.fill.is_some() {
        return Err(format!(
            "{}: filled shape; --out writes strokes only, use --crate-dir",
            icon.name
        ));
    }
    if !icon.layers.is_empty() {
        return Err(format!(
            "{}: translucent layer; --out writes opaque strokes only, use --crate-dir",
            icon.name
        ));
    }
    Ok(())
}

fn path_segs(path: &usvg::tiny_skia_path::Path, ts: usvg::Transform, out: &mut Vec<Seg>) {
    let map = |mut p: Point| {
        ts.map_point(&mut p);
        (round(p.x), round(p.y))
    };
    let mut start = None;
    let mut closed = false;
    for seg in path.segments() {
        // SVG continues from the subpath's start after Z; the painter needs an
        // explicit M there.
        if closed && !matches!(seg, PathSegment::MoveTo(_)) {
            let (x, y) = start.expect("Z follows a subpath start");
            out.push(Seg::M(x, y));
        }
        closed = false;
        match seg {
            PathSegment::MoveTo(p) => {
                let (x, y) = map(p);
                start = Some((x, y));
                out.push(Seg::M(x, y));
            }
            PathSegment::LineTo(p) => {
                let (x, y) = map(p);
                out.push(Seg::L(x, y));
            }
            PathSegment::QuadTo(c, p) => {
                let ((x1, y1), (x, y)) = (map(c), map(p));
                out.push(Seg::Q(x1, y1, x, y));
            }
            PathSegment::CubicTo(c1, c2, p) => {
                let ((x1, y1), (x2, y2), (x, y)) = (map(c1), map(c2), map(p));
                out.push(Seg::C(x1, y1, x2, y2, x, y));
            }
            PathSegment::Close => {
                out.push(Seg::Z);
                closed = true;
            }
        }
    }
}

/// Three decimals is far below a pixel at any icon size.
fn round(v: f32) -> f32 {
    (v * 1000.0).round() / 1000.0
}

/// `circle-alert` -> `CIRCLE_ALERT`; a leading digit gets an `I_` prefix.
pub fn const_name(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    if s.starts_with(|c: char| c.is_ascii_digit()) {
        s.insert_str(0, "I_");
    }
    s
}

fn num(v: f32) -> String {
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') {
        s
    } else {
        format!("{s}.0")
    }
}

/// Render a Rust module defining one `pub const` per icon plus `ALL`.
/// `module_path` is where `Icon`, `IconShape` and `Seg` live in the target crate, e.g.
/// `crate::components::icon` or `crate::shadcn::components::icon`.
pub fn render_module(icons: &[Converted], module_path: &str, provenance: &str) -> String {
    let mut out = String::new();
    out.push_str("// Generated by icongen (shadcn-egui/tools/icongen). Do not edit: re-run it.\n");
    for line in provenance.lines() {
        out.push_str(&format!("// {line}\n"));
    }
    out.push_str(&format!(
        "\nuse {module_path}::{{Icon, IconShape, Seg::*}};\n"
    ));
    for icon in icons {
        out.push_str(&format!(
            "\npub const {}: Icon = Icon {{\n    name: {:?},\n    size: {},\n    shape: IconShape::Segs(&[\n",
            const_name(&icon.name),
            icon.name,
            num(icon.size)
        ));
        for seg in &icon.segs {
            let line = match *seg {
                Seg::M(x, y) => format!("M({}, {})", num(x), num(y)),
                Seg::L(x, y) => format!("L({}, {})", num(x), num(y)),
                Seg::Q(a, b, x, y) => format!("Q({}, {}, {}, {})", num(a), num(b), num(x), num(y)),
                Seg::C(a, b, c, d, x, y) => {
                    format!(
                        "C({}, {}, {}, {}, {}, {})",
                        num(a),
                        num(b),
                        num(c),
                        num(d),
                        num(x),
                        num(y)
                    )
                }
                Seg::Z => "Z".to_owned(),
            };
            out.push_str(&format!("        {line},\n"));
        }
        out.push_str("    ]),\n};\n");
    }
    out.push_str("\n/// Every icon in this module, in input order.\npub const ALL: &[&Icon] = &[");
    out.push_str(
        &icons
            .iter()
            .map(|i| format!("&{}", const_name(&i.name)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    out.push_str("];\n");
    out
}

/// The three icons shadcn-egui's components draw, by icon name.
#[derive(Clone, Debug, PartialEq)]
pub struct Slots {
    pub check: String,
    pub chevron_down: String,
    pub close: String,
}

/// Parse `check=<icon>,chevron_down=<icon>,close=<icon>`.
pub fn parse_slots(s: &str) -> Result<Slots, String> {
    let (mut check, mut chevron_down, mut close) = (None, None, None);
    for part in s.split(',') {
        let (k, v) = part
            .split_once('=')
            .ok_or_else(|| format!("--slots: {part:?} is not slot=icon"))?;
        let slot = match k {
            "check" => &mut check,
            "chevron_down" => &mut chevron_down,
            "close" => &mut close,
            _ => return Err(format!("--slots: unknown slot {k:?}")),
        };
        *slot = Some(v.to_owned());
    }
    match (check, chevron_down, close) {
        (Some(check), Some(chevron_down), Some(close)) => Ok(Slots {
            check,
            chevron_down,
            close,
        }),
        _ => Err("--slots needs check=<icon>,chevron_down=<icon>,close=<icon>".to_owned()),
    }
}

/// Render an icon crate: `src/lib.rs` text and the `src/icons.bin` bytes.
/// Icons are sorted by name, so the output does not depend on argument order.
pub fn render_crate(
    icons: &[Converted],
    slots: &Slots,
    provenance: &str,
) -> Result<(String, Vec<u8>), String> {
    let mut sorted: Vec<&Converted> = icons.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    let find = |slot: &str, name: &str| {
        sorted
            .iter()
            .find(|i| i.name == name)
            .map(|i| const_name(&i.name))
            .ok_or_else(|| format!("--slots: {slot}={name}: no such icon"))
    };
    let check = find("check", &slots.check)?;
    let chevron_down = find("chevron_down", &slots.chevron_down)?;
    let close = find("close", &slots.close)?;

    let mut src = String::new();
    src.push_str("// Generated by icongen (shadcn-egui/tools/icongen). Do not edit: re-run it.\n");
    for line in provenance.lines() {
        src.push_str(&format!("//! {line}\n"));
    }
    src.push_str(
        "//!\n//! Every icon is a `pub const`; [`ALL`] lists them and [`SLOTS`] holds the\n",
    );
    src.push_str("//! icons shadcn-egui's components draw (checkbox tick, select trigger, close button).\n\n");
    src.push_str("use shadcn_egui_core::{BlobRange, Icon, IconShape, IconSlots};\n\n");
    let (mut consts, mut blob) = (String::new(), Vec::new());
    for icon in &sorted {
        let rec = blob::encode(icon)?;
        consts.push_str(&format!(
            "pub const {}: Icon = icon({:?}, {}, {});\n",
            const_name(&icon.name),
            icon.name,
            blob.len(),
            rec.len()
        ));
        blob.extend_from_slice(&rec);
    }
    if u32::try_from(blob.len()).is_err() {
        return Err(format!(
            "blob is {} bytes, more than u32 offsets reach",
            blob.len()
        ));
    }
    // A static array, not a `const` reference: a const has no fixed address,
    // so every crate using the icons would get its own copy of the blob.
    src.push_str(&format!(
        "static ICONS_BIN: [u8; {}] = *include_bytes!(\"icons.bin\");\n\n",
        blob.len()
    ));
    src.push_str("const fn icon(name: &'static str, start: u32, len: u32) -> Icon {\n");
    src.push_str("    Icon { name, size: 24.0, shape: IconShape::Blob(BlobRange { blob: &ICONS_BIN, start, len }) }\n}\n\n");
    src.push_str(&consts);
    src.push_str("\n/// Every icon in this crate, sorted by name.\npub const ALL: &[&Icon] = &[");
    src.push_str(
        &sorted
            .iter()
            .map(|i| format!("&{}", const_name(&i.name)))
            .collect::<Vec<_>>()
            .join(", "),
    );
    src.push_str("];\n\n/// The icons shadcn-egui's components draw.\n");
    src.push_str(&format!(
        "pub const SLOTS: IconSlots = IconSlots {{ check: {check}, chevron_down: {chevron_down}, close: {close} }};\n"
    ));
    Ok((rustfmt(&src)?, blob))
}

/// Format generated source the way `cargo fmt --check` expects it: a crate
/// root cannot opt out with `#![rustfmt::skip]` on stable.
fn rustfmt(src: &str) -> Result<String, String> {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2021"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("rustfmt: {e}"))?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let input = src.to_owned();
    // Write from a thread so a large crate cannot deadlock on full pipes.
    let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
    let mut out = String::new();
    child
        .stdout
        .take()
        .expect("piped stdout")
        .read_to_string(&mut out)
        .map_err(|e| format!("rustfmt: {e}"))?;
    let output = child
        .wait_with_output()
        .map_err(|e| format!("rustfmt: {e}"))?;
    writer
        .join()
        .expect("rustfmt writer thread")
        .map_err(|e| format!("rustfmt: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "rustfmt failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(out)
}

/// Icon count, blob size split into stroke and fill, and the ten largest icons.
pub fn measure(icons: &[Converted]) -> Result<String, String> {
    let (mut stroke, mut fill, mut sizes) = (0, 0, Vec::new());
    for icon in icons {
        let rec = blob::encode(icon)?;
        let s = blob::stroke_bytes(&rec);
        stroke += s;
        fill += rec.len() - s;
        sizes.push((rec.len(), icon.name.as_str()));
    }
    sizes.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    let largest: Vec<String> = sizes
        .iter()
        .take(10)
        .map(|(n, name)| format!("{name} {n}"))
        .collect();
    Ok(format!(
        "icons: {}\nblob bytes: {} (stroke {stroke}, fill {fill})\nlargest: {}\n",
        icons.len(),
        stroke + fill,
        largest.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn rustfmt_failure_is_an_error() {
        let err = super::rustfmt("fn {").expect_err("not Rust");
        assert!(err.starts_with("rustfmt failed"), "{err}");
    }
}
