//! Stroked vector icons (Lucide-style) painted with egui shapes, no image
//! loader. Icon data is generated ahead of time by `tools/icongen` from SVG
//! files; this module only knows how to paint it.
//!
//! Lucide icons are 24x24 outlines drawn with a 2-unit round-capped stroke.
//! Round caps matter: some icons draw dots as zero-length lines, which vanish
//! without them, so every open subpath gets cap discs at both ends.

use crate::Theme;
use egui::epaint::{CubicBezierShape, PathShape, QuadraticBezierShape};
use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};

/// One path command in icon units (the SVG's user space, origin top-left).
/// A `M` starts a new subpath; `Z` closes the current one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    M(f32, f32),
    L(f32, f32),
    Q(f32, f32, f32, f32),
    C(f32, f32, f32, f32, f32, f32),
    Z,
}

/// A generated icon: its name (for screen readers), the side of its square
/// view box, and its path commands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Icon {
    pub name: &'static str,
    pub size: f32,
    pub segs: &'static [Seg],
}

/// The dialog close cross (Lucide `x`, ISC), built in because the registry's
/// own components need it and projects generate their icon sets separately.
pub const CLOSE: Icon = Icon {
    name: "close",
    size: 24.0,
    segs: &[Seg::M(18.0, 6.0), Seg::L(6.0, 18.0), Seg::M(6.0, 6.0), Seg::L(18.0, 18.0)],
};

/// The checkbox tick (Lucide `check`, ISC), built in for the same reason as
/// [`CLOSE`].
pub const CHECK: Icon = Icon {
    name: "check",
    size: 24.0,
    segs: &[Seg::M(20.0, 6.0), Seg::L(9.0, 17.0), Seg::L(4.0, 12.0)],
};

/// The select chevron (Lucide `chevron-down`, ISC), built in for the same
/// reason as [`CLOSE`].
pub const CHEVRON_DOWN: Icon = Icon {
    name: "chevron-down",
    size: 24.0,
    segs: &[Seg::M(6.0, 9.0), Seg::L(12.0, 15.0), Seg::L(18.0, 9.0)],
};

/// Curve flattening tolerance in screen points.
const TOLERANCE: f32 = 0.1;

/// Paint `icon` scaled into the square `rect`. `stroke_width` is in icon units
/// (Lucide's own is 2.0) and scales with the icon.
///
/// Precondition: `icon.segs` starts with `Seg::M` (the generator guarantees it).
pub fn paint_icon(painter: &Painter, rect: Rect, icon: &Icon, color: Color32, stroke_width: f32) {
    let scale = rect.width() / icon.size;
    let map = |x: f32, y: f32| rect.min + Vec2::new(x, y) * scale;
    let stroke = Stroke::new(stroke_width * scale, color);
    let cap_r = stroke.width / 2.0;

    let mut shapes = Vec::new();
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

    for seg in icon.segs {
        match *seg {
            Seg::M(x, y) => {
                flush(&mut points, false, &mut shapes);
                points.push(map(x, y));
            }
            Seg::L(x, y) => points.push(map(x, y)),
            Seg::Q(x1, y1, x, y) => {
                let from = *points.last().expect("icon path starts with M");
                let q = QuadraticBezierShape::from_points_stroke(
                    [from, map(x1, y1), map(x, y)],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::NONE,
                );
                points.extend(q.flatten(Some(TOLERANCE)).into_iter().skip(1));
            }
            Seg::C(x1, y1, x2, y2, x, y) => {
                let from = *points.last().expect("icon path starts with M");
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

/// An icon as a widget: `ui.add(IconView::new(&icons::MIC))`. Sized
/// `Metrics::icon`, colored `foreground`, stroked `Metrics::icon_stroke`.
pub struct IconView<'a> {
    icon: &'a Icon,
    size: Option<f32>,
    color: Option<Color32>,
}

impl<'a> IconView<'a> {
    pub fn new(icon: &'a Icon) -> Self {
        Self { icon, size: None, color: None }
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
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, ui.is_enabled(), self.icon.name));
        if ui.is_rect_visible(rect) {
            let color = self.color.unwrap_or(t.palette.foreground);
            paint_icon(ui.painter(), rect, self.icon, color, t.metrics.icon_stroke);
        }
        resp
    }
}
