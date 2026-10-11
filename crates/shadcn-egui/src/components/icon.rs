//! Vector icons painted with egui shapes, no image loader. Icon data is
//! generated ahead of time by `tools/icongen` (strokes as path commands,
//! fills as pre-triangulated meshes); this module only paints it.
//!
//! Lucide icons are 24x24 outlines drawn with a 2-unit round-capped stroke.
//! Round caps matter: some icons draw dots as zero-length lines, which vanish
//! without them, so every open subpath gets cap discs at both ends.

use crate::{FillMesh, Theme};
use egui::epaint::{CubicBezierShape, PathShape, QuadraticBezierShape};
use egui::{Color32, Mesh, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
pub use shadcn_egui_core::{Icon, IconShape, IconSlots, Seg};

/// The dialog close cross (Lucide `x`, ISC), built in because the registry's
/// own components need it and projects generate their icon sets separately.
pub const CLOSE: Icon = Icon {
    name: "close",
    size: 24.0,
    shape: IconShape::Segs(&[
        Seg::M(18.0, 6.0),
        Seg::L(6.0, 18.0),
        Seg::M(6.0, 6.0),
        Seg::L(18.0, 18.0),
    ]),
};

/// The checkbox tick (Lucide `check`, ISC), built in for the same reason as
/// [`CLOSE`].
pub const CHECK: Icon = Icon {
    name: "check",
    size: 24.0,
    shape: IconShape::Segs(&[Seg::M(20.0, 6.0), Seg::L(9.0, 17.0), Seg::L(4.0, 12.0)]),
};

/// The select chevron (Lucide `chevron-down`, ISC), built in for the same
/// reason as [`CLOSE`].
pub const CHEVRON_DOWN: Icon = Icon {
    name: "chevron-down",
    size: 24.0,
    shape: IconShape::Segs(&[Seg::M(6.0, 9.0), Seg::L(12.0, 15.0), Seg::L(18.0, 9.0)]),
};

/// The built-in Lucide slots every new [`Theme`] starts with.
pub const LUCIDE_SLOTS: IconSlots = IconSlots {
    check: CHECK,
    chevron_down: CHEVRON_DOWN,
    close: CLOSE,
};

/// Curve flattening tolerance in screen points.
const TOLERANCE: f32 = 0.1; // behaviour-ok: curve flattening tolerance, not a design token

/// Paint `icon` scaled into the square `rect`: its fill first (if any), then
/// its strokes. `stroke_width` is in icon units (Lucide's own is 2.0) and
/// scales with the icon.
///
/// Precondition: `icon.segs()` starts with `Seg::M` (icongen guarantees it);
/// a curve with no start point is skipped.
pub fn paint_icon(painter: &Painter, rect: Rect, icon: &Icon, color: Color32, stroke_width: f32) {
    let scale = rect.width() / icon.size;
    let map = |x: f32, y: f32| rect.min + Vec2::new(x, y) * scale;
    let stroke = Stroke::new(stroke_width * scale, color);
    let cap_r = stroke.width / 2.0;

    let mut shapes = Vec::new();
    if let Some(fill) = icon.fill_mesh() {
        shapes.push(fill_shape(
            &fill,
            map,
            color,
            1.0 / painter.pixels_per_point(),
        ));
    }
    let mut points: Vec<Pos2> = Vec::new();
    let flush = |points: &mut Vec<Pos2>, closed: bool, shapes: &mut Vec<Shape>| {
        if points.is_empty() {
            return;
        }
        let pts = std::mem::take(points);
        if closed {
            shapes.push(Shape::Path(PathShape::closed_line(pts, stroke)));
        } else {
            let (first, last) = (pts[0], pts[pts.len() - 1]);
            if pts.len() > 1 {
                shapes.push(Shape::Path(PathShape::line(pts, stroke)));
            }
            shapes.push(Shape::circle_filled(first, cap_r, color));
            shapes.push(Shape::circle_filled(last, cap_r, color));
        }
    };

    for seg in icon.segs() {
        match seg {
            Seg::M(x, y) => {
                flush(&mut points, false, &mut shapes);
                points.push(map(x, y));
            }
            Seg::L(x, y) => points.push(map(x, y)),
            Seg::Q(x1, y1, x, y) => {
                let Some(&from) = points.last() else {
                    continue;
                };
                let q = QuadraticBezierShape::from_points_stroke(
                    [from, map(x1, y1), map(x, y)],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::NONE,
                );
                points.extend(q.flatten(Some(TOLERANCE)).into_iter().skip(1));
            }
            Seg::C(x1, y1, x2, y2, x, y) => {
                let Some(&from) = points.last() else {
                    continue;
                };
                let c = CubicBezierShape::from_points_stroke(
                    [from, map(x1, y1), map(x2, y2), map(x, y)],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::NONE,
                );
                points.extend(c.flatten(Some(TOLERANCE)).into_iter().skip(1));
            }
            Seg::Z => flush(&mut points, true, &mut shapes),
        }
    }
    flush(&mut points, false, &mut shapes);
    painter.extend(shapes);
}

/// The fill as one mesh: its triangles in `color`, plus per contour a strip
/// from the edge (in `color`) to `fringe` points outward along each vertex's
/// miter (transparent). Like epaint's own feathering but outward only, so a
/// translucent colour shows no darker seam: the strip stays off the fill, as
/// icongen zeroes the miter where loops touch. Separate fill paths that
/// overlap each other (two Remixicon icons) still blend twice there.
/// `fringe` is one physical pixel on an untransformed layer.
fn fill_shape(
    fill: &FillMesh,
    map: impl Fn(f32, f32) -> Pos2,
    color: Color32,
    fringe: f32,
) -> Shape {
    let mut mesh = Mesh::default();
    for &[x, y] in &fill.vertices {
        mesh.colored_vertex(map(x, y), color);
    }
    mesh.indices
        .extend(fill.indices.iter().map(|&i| u32::from(i)));
    for contour in &fill.contours {
        if contour.is_empty() {
            continue;
        }
        let base = mesh.vertices.len() as u32;
        let n = contour.len() as u32;
        for &([x, y], [mx, my]) in contour {
            let p = map(x, y);
            mesh.colored_vertex(p, color);
            mesh.colored_vertex(p + Vec2::new(mx, my) * fringe, Color32::TRANSPARENT);
        }
        let mut i0 = n - 1;
        for i1 in 0..n {
            let (in0, out0, in1, out1) = (
                base + 2 * i0,
                base + 2 * i0 + 1,
                base + 2 * i1,
                base + 2 * i1 + 1,
            );
            mesh.add_triangle(in0, in1, out1);
            mesh.add_triangle(in0, out1, out0);
            i0 = i1;
        }
    }
    Shape::mesh(mesh)
}

/// An icon as a widget: `ui.add(IconView::new(&icons::MIC))`. Sized
/// `Metrics::icon`, colored `foreground`, stroked `Metrics::icon_stroke`.
/// It has no accessible name; give it one with `.labelled_by(label.id)`.
pub struct IconView<'a> {
    icon: &'a Icon,
    size: Option<f32>,
    color: Option<Color32>,
}

impl<'a> IconView<'a> {
    pub fn new(icon: &'a Icon) -> Self {
        Self {
            icon,
            size: None,
            color: None,
        }
    }
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }
    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }
}

impl Widget for IconView<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let side = self.size.unwrap_or(t.metrics.icon);
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
        // No label of its own (the icon's name is an id, not text a reader
        // should hear): name it with `labelled_by(label.id)`, which an own
        // label would override.
        let enabled = ui.is_enabled();
        resp.widget_info(|| egui::WidgetInfo {
            enabled,
            ..egui::WidgetInfo::new(egui::WidgetType::Image)
        });
        if ui.is_rect_visible(rect) {
            let color = self.color.unwrap_or(t.palette.foreground);
            paint_icon(ui.painter(), rect, self.icon, color, t.metrics.icon_stroke);
        }
        resp
    }
}
