//! Convert stroked SVG icons into Rust constants for
//! `egui_shadcn::components::icon`.
//!
//! The input is an outline icon set such as Lucide: paths, lines, circles and
//! rects drawn with a stroke and no fill. usvg resolves shapes, arcs and
//! transforms into move/line/quad/cubic segments; this crate writes those out
//! as `Seg` values. Anything it cannot draw as a stroke (fills, images, text,
//! a non-square view box) is an error, not a silent omission.

use usvg::tiny_skia_path::{PathSegment, Point};

/// Mirror of `egui_shadcn::components::icon::Seg`.
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
    /// Side of the square view box.
    pub size: f32,
    pub segs: Vec<Seg>,
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
    let mut segs = Vec::new();
    walk(name, tree.root(), &mut segs)?;
    if segs.is_empty() {
        return Err(format!("{name}: no stroked paths"));
    }
    Ok(Converted {
        name: name.to_owned(),
        size: size.width(),
        segs,
    })
}

fn walk(name: &str, group: &usvg::Group, out: &mut Vec<Seg>) -> Result<(), String> {
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => walk(name, g, out)?,
            usvg::Node::Path(p) => {
                if p.fill().is_some() {
                    return Err(format!(
                        "{name}: filled shape (id {:?}); only stroked outlines are supported",
                        p.id()
                    ));
                }
                if p.stroke().is_none() || !p.is_visible() {
                    continue;
                }
                path_segs(p.data(), p.abs_transform(), out);
            }
            usvg::Node::Image(_) => return Err(format!("{name}: embedded image not supported")),
            usvg::Node::Text(_) => return Err(format!("{name}: text not supported")),
        }
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
/// `module_path` is where `Icon` and `Seg` live in the target crate, e.g.
/// `crate::components::icon` or `crate::shadcn::components::icon`.
pub fn render_module(icons: &[Converted], module_path: &str, provenance: &str) -> String {
    let mut out = String::new();
    out.push_str("// Generated by icongen (egui-shadcn/tools/icongen). Do not edit: re-run it.\n");
    for line in provenance.lines() {
        out.push_str(&format!("// {line}\n"));
    }
    out.push_str(&format!("\nuse {module_path}::{{Icon, Seg::*}};\n"));
    for icon in icons {
        out.push_str(&format!(
            "\npub const {}: Icon = Icon {{\n    name: {:?},\n    size: {},\n    segs: &[\n",
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
        out.push_str("    ],\n};\n");
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
