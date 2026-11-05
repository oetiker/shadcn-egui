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

pub use fill::{Fill, FillPath, FillRule, Precision};

/// Mirror of `shadcn_egui_core::Seg`, plus the blob's style command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    M(f32, f32),
    L(f32, f32),
    Q(f32, f32, f32, f32),
    C(f32, f32, f32, f32, f32, f32),
    Z,
    /// The caps and joins of the sub-paths that follow, up to the next
    /// `Style` or the end of the layer. A layer starts round and round, so
    /// a set drawn that way never holds one.
    Style(StrokeStyle),
}

/// SVG `stroke-linecap`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cap {
    Round,
    Butt,
    Square,
}

/// SVG `stroke-linejoin`; a miter takes SVG's default limit of 4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Join {
    Round,
    Miter,
    Bevel,
}

/// How a stroked sub-path ends and turns. The stroke width is not part of
/// it: every stroke is drawn at the caller's width, as shadcn's icon
/// components set it on every element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StrokeStyle {
    pub cap: Cap,
    pub join: Join,
}

impl StrokeStyle {
    /// Round caps and round joins: where every layer starts.
    pub const ROUND: StrokeStyle = StrokeStyle {
        cap: Cap::Round,
        join: Join::Round,
    };

    /// The blob's style word: cap in bits 0-1 (0 round, 1 butt, 2 square),
    /// join in bits 2-3 (0 round, 1 miter, 2 bevel).
    pub fn word(self) -> u16 {
        let cap = match self.cap {
            Cap::Round => 0,
            Cap::Butt => 1,
            Cap::Square => 2,
        };
        let join = match self.join {
            Join::Round => 0,
            Join::Miter => 1,
            Join::Bevel => 2,
        };
        cap | join << 2
    }

    /// The style of a style word; `None` for a word icongen does not write.
    pub fn from_word(w: u16) -> Option<StrokeStyle> {
        let cap = match w & 3 {
            0 => Cap::Round,
            1 => Cap::Butt,
            2 => Cap::Square,
            _ => return None,
        };
        let join = match w >> 2 {
            0 => Join::Round,
            1 => Join::Miter,
            2 => Join::Bevel,
            _ => return None,
        };
        Some(StrokeStyle { cap, join })
    }

    /// The style of a usvg stroke. A miter limit other than SVG's default
    /// 4 and SVG 2's `miter-clip` and `arcs` joins are errors: the blob
    /// cannot hold them.
    fn of(name: &str, s: &usvg::Stroke) -> Result<StrokeStyle, String> {
        let cap = match s.linecap() {
            usvg::LineCap::Round => Cap::Round,
            usvg::LineCap::Butt => Cap::Butt,
            usvg::LineCap::Square => Cap::Square,
        };
        let join = match s.linejoin() {
            usvg::LineJoin::Round => Join::Round,
            usvg::LineJoin::Bevel => Join::Bevel,
            usvg::LineJoin::Miter => {
                if s.miterlimit().get() != 4.0 {
                    return Err(format!(
                        "{name}: stroke-miterlimit {} not supported, only the default 4",
                        s.miterlimit().get()
                    ));
                }
                Join::Miter
            }
            j => return Err(format!("{name}: stroke-linejoin {j:?} not supported")),
        };
        Ok(StrokeStyle { cap, join })
    }
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
    convert_with(name, svg, Precision::BLOB)
}

/// [`convert`] with the fills flattened at `prec` instead of the blob's
/// precision.
pub fn convert_with(name: &str, svg: &str, prec: Precision) -> Result<Converted, String> {
    let (size, gathered) = gather(name, svg, prec)?;
    let mut gathered = gathered.into_iter();
    let base = gathered.next().expect("layer 0");
    let fill = fill::build(name, &base.fills, prec)?;
    let mut layers = Vec::new();
    for g in gathered {
        let fill = fill::build(name, &g.fills, prec)?;
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
        size,
        segs: base.segs,
        fill,
        layers,
    })
}

/// The filled paths of one SVG document flattened at `prec`, one entry per
/// layer in paint order, before [`fill::build`] tessellates them: layer 0
/// holds everything opaque. The tests measure fills against these.
pub fn fill_paths(name: &str, svg: &str, prec: Precision) -> Result<Vec<Vec<FillPath>>, String> {
    Ok(gather(name, svg, prec)?
        .1
        .into_iter()
        .map(|g| g.fills)
        .collect())
}

/// Parse `svg` and collect its strokes and fill paths per layer.
fn gather(name: &str, svg: &str, prec: Precision) -> Result<(f32, Vec<Gather>), String> {
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
        prec,
        &mut gathered,
    )?;
    Ok((size.width(), gathered))
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
    prec: Precision,
    layers: &mut Vec<Gather>,
) -> Result<(), String> {
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => {
                if let Some(l) = layer_for(layers, layer, alpha, g.opacity().get()) {
                    let a = layers[l].alpha;
                    walk(name, g, scale, a, l, prec, layers)?;
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
                    let contours = fill::flatten(p.data(), p.abs_transform(), scale, prec);
                    let l = layer_for(layers, layer, alpha, f.opacity().get());
                    if let (Some(l), false) = (l, contours.is_empty()) {
                        layers[l].fills.push(FillPath { rule, contours });
                    }
                }
                if let Some(s) = p.stroke() {
                    if let Some(l) = layer_for(layers, layer, alpha, s.opacity().get()) {
                        let style = StrokeStyle::of(name, s)?;
                        path_segs(p.data(), p.abs_transform(), style, &mut layers[l].segs);
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
    if icon.segs.iter().any(|s| matches!(s, Seg::Style(_))) {
        return Err(format!(
            "{}: stroke caps or joins that are not round; --out writes round strokes only, use --crate-dir",
            icon.name
        ));
    }
    Ok(())
}

/// The style in effect at the end of `segs`.
fn style_at_end(segs: &[Seg]) -> StrokeStyle {
    segs.iter()
        .rev()
        .find_map(|s| match s {
            Seg::Style(st) => Some(*st),
            _ => None,
        })
        .unwrap_or(StrokeStyle::ROUND)
}

/// Appends the path's segments to `out`, after a `Style` when `style` is
/// not the one in effect there.
fn path_segs(
    path: &usvg::tiny_skia_path::Path,
    ts: usvg::Transform,
    style: StrokeStyle,
    out: &mut Vec<Seg>,
) {
    if path.segments().next().is_some() && style_at_end(out) != style {
        out.push(Seg::Style(style));
    }
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
                // `ensure_stroke_only` rejects these; a `Seg` slice is round.
                Seg::Style(_) => continue,
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

/// The `IconSlots` fields of `shadcn-egui-core`, in declaration order.
pub const SLOT_NAMES: &[&str] = &[
    "check",
    "select_trigger",
    "chevron_down_plain",
    "chevron_up",
    "chevron_left",
    "chevron_right",
    "more_horizontal",
    "close",
    "loader",
];

/// The icon each slot of [`SLOT_NAMES`] draws, by icon name, in that order.
#[derive(Clone, Debug, PartialEq)]
pub struct Slots(pub Vec<String>);

/// Parse `slot=<icon>,...`: every slot of [`SLOT_NAMES`] once, in any order.
pub fn parse_slots(s: &str) -> Result<Slots, String> {
    let mut icons: Vec<Option<String>> = vec![None; SLOT_NAMES.len()];
    for part in s.split(',') {
        let (k, v) = part
            .split_once('=')
            .ok_or_else(|| format!("--slots: {part:?} is not slot=icon"))?;
        let i = SLOT_NAMES
            .iter()
            .position(|n| *n == k)
            .ok_or_else(|| format!("--slots: unknown slot {k:?}"))?;
        if icons[i].replace(v.to_owned()).is_some() {
            return Err(format!("--slots: slot {k:?} given twice"));
        }
    }
    icons
        .into_iter()
        .zip(SLOT_NAMES)
        .map(|(icon, name)| icon.ok_or_else(|| format!("--slots: no icon for slot {name:?}")))
        .collect::<Result<_, _>>()
        .map(Slots)
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
    let slot_consts = SLOT_NAMES
        .iter()
        .zip(&slots.0)
        .map(|(slot, name)| find(slot, name).map(|c| (*slot, c)))
        .collect::<Result<Vec<_>, _>>()?;

    let mut src = String::new();
    src.push_str("// Generated by icongen (shadcn-egui/tools/icongen). Do not edit: re-run it.\n");
    for line in provenance.lines() {
        src.push_str(&format!("//! {line}\n"));
    }
    src.push_str(
        "//!\n//! Every icon is a `pub const`; [`ALL`] lists them and [`SLOTS`] holds the\n",
    );
    src.push_str("//! icons shadcn-egui's components draw (checkbox tick, select trigger, close button, ...).\n\n");
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
    // `IconSlots` is `#[non_exhaustive]`: start from `all` and set each slot.
    src.push_str("pub const SLOTS: IconSlots = {\n");
    src.push_str(&format!(
        "    let mut s = IconSlots::all({});\n",
        slot_consts[0].1
    ));
    for (slot, c) in &slot_consts {
        src.push_str(&format!("    s.{slot} = {c};\n"));
    }
    src.push_str("    s\n};\n");
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
