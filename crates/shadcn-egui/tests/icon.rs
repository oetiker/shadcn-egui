#[path = "icons/lucide.rs"]
#[rustfmt::skip]
mod lucide;
mod common;

use egui::epaint::{CircleShape, PathShape};
use egui::{pos2, vec2, Color32, Rect, Shape};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use shadcn_egui::components::button::{Button, ButtonSize, ButtonVariant};
use shadcn_egui::components::icon::{paint_icon, Icon, IconView};
use shadcn_egui::{layout, Theme};

/// Paint one icon into `rect` and return the flattened shapes.
fn painted(icon: &Icon, rect: Rect) -> Vec<Shape> {
    let ctx = egui::Context::default();
    let out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let painter = ui.painter_at(Rect::EVERYTHING);
        paint_icon(&painter, rect, icon, Color32::RED, 2.0);
    });
    fn walk(s: &Shape, out: &mut Vec<Shape>) {
        match s {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            s => out.push(s.clone()),
        }
    }
    let mut shapes = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut shapes));
    shapes
}

fn circles(shapes: &[Shape]) -> Vec<CircleShape> {
    shapes
        .iter()
        .filter_map(|s| {
            if let Shape::Circle(c) = s {
                Some(*c)
            } else {
                None
            }
        })
        .collect()
}

fn paths(shapes: &[Shape]) -> Vec<PathShape> {
    shapes
        .iter()
        .filter_map(|s| {
            if let Shape::Path(p) = s {
                Some(p.clone())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn round_caps_draw_the_dot() {
    // circle-alert's dot is a 0.01-unit line; only its cap discs make it visible.
    let shapes = painted(
        &lucide::CIRCLE_ALERT,
        Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0)),
    );
    let dot = circles(&shapes)
        .into_iter()
        .find(|c| (c.center - pos2(12.0, 16.0)).length() < 0.02);
    let dot = dot.expect("cap disc at the dot");
    assert_eq!(dot.radius, 1.0, "cap radius is half the stroke");
    assert_eq!(dot.fill, Color32::RED);
}

#[test]
fn icon_scales_into_rect() {
    // x.svg: M18 6 L6 18 and M6 6 L18 18, painted at 2x offset by (100, 50).
    let shapes = painted(
        &lucide::X,
        Rect::from_min_size(pos2(100.0, 50.0), vec2(48.0, 48.0)),
    );
    let p = paths(&shapes);
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].points, vec![pos2(136.0, 62.0), pos2(112.0, 86.0)]);
    assert!(!p[0].closed);
    assert_eq!(p[0].stroke.width, 4.0, "stroke scales with the icon");
}

#[test]
fn closed_subpaths_stay_closed() {
    let shapes = painted(
        &lucide::SETTINGS,
        Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0)),
    );
    let p = paths(&shapes);
    assert!(p.iter().any(|p| p.closed), "settings has closed subpaths");
    // A closed subpath gets no cap discs: only open ones do.
    let open = p.iter().filter(|p| !p.closed).count();
    assert_eq!(circles(&shapes).len(), open * 2);
}

#[test]
fn icon_widgets_are_labelled() {
    use std::cell::Cell;
    let clicked = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        Theme::vega_dark().apply(ui.ctx());
        ui.add(IconView::new(&lucide::MIC));
        if ui
            .add(
                Button::new("Power off")
                    .icon(&lucide::POWER)
                    .size(ButtonSize::Icon),
            )
            .clicked()
        {
            clicked.set(true);
        }
    });
    h.get_by_label("mic");
    h.get_by_label("Power off").click();
    h.run();
    assert!(
        clicked.get(),
        "icon-only button is reachable by its text label"
    );
}

#[test]
fn icons_snapshot() {
    common::snapshot_all("icons", vec2(420.0, 120.0), |ui| {
        layout::vstack(ui, 16.0, |ui| {
            layout::row(ui, 16.0, |ui| {
                for icon in lucide::ALL {
                    ui.add(IconView::new(icon));
                }
                for icon in lucide::ALL.iter().take(3) {
                    ui.add(IconView::new(icon).size(24.0));
                }
            });
            layout::row(ui, 8.0, |ui| {
                ui.add(Button::new("Microphone").icon(&lucide::MIC));
                ui.add(
                    Button::new("Display")
                        .icon(&lucide::MONITOR)
                        .variant(ButtonVariant::Outline),
                );
                ui.add(
                    Button::new("Close")
                        .icon(&lucide::X)
                        .size(ButtonSize::Icon)
                        .variant(ButtonVariant::Ghost),
                );
                ui.add(
                    Button::new("Power")
                        .icon(&lucide::POWER)
                        .size(ButtonSize::Icon)
                        .variant(ButtonVariant::Destructive),
                );
            });
        });
    });
}
