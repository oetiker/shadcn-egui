#[path = "icons/lucide.rs"]
#[rustfmt::skip]
mod lucide;
mod common;

use egui::{pos2, vec2, Color32, ColorImage, Rect, Shape, TextureId};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use shadcn_egui::components::button::{Button, ButtonSize, ButtonVariant};
use shadcn_egui::components::icon::{paint_icon, Icon, IconView};
use shadcn_egui::{layout, Theme};
use std::sync::Arc;

/// Paint one icon into `rect` at ppp 1 and return the drawn rect and the
/// icon's coverage texture, with a lookup of its alpha at a point in the
/// texture's pixel space.
fn raster(icon: &Icon, rect: Rect) -> (Rect, Arc<ColorImage>) {
    let ctx = egui::Context::default();
    let out = ctx.run_ui(egui::RawInput::default(), |ui| {
        let painter = ui.painter_at(Rect::EVERYTHING);
        paint_icon(&painter, rect, icon, Color32::RED, 2.0);
    });
    fn walk(s: &Shape, out: &mut Vec<Rect>) {
        match s {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            Shape::Mesh(m) if m.texture_id != TextureId::default() => out.push(m.calc_bounds()),
            _ => {}
        }
    }
    let mut rects = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut rects));
    let imgs: Vec<_> = out
        .textures_delta
        .set
        .iter()
        .filter(|(id, _)| *id != TextureId::default())
        .map(|(_, d)| {
            let egui::ImageData::Color(img) = &d.image;
            img.clone()
        })
        .collect();
    assert_eq!((rects.len(), imgs.len()), (1, 1), "one textured image");
    (rects[0], imgs[0].clone())
}

fn alpha(img: &ColorImage, x: usize, y: usize) -> u8 {
    img.pixels[y * img.size[0] + x].a()
}

#[test]
fn round_caps_draw_the_dot() {
    // circle-alert's dot is a 0.01-unit line at (12, 16); only its round
    // caps make it visible: a disc of radius 1.
    let (_, img) = raster(
        &lucide::CIRCLE_ALERT,
        Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0)),
    );
    for (x, y) in [(11, 15), (12, 15), (11, 16), (12, 16)] {
        assert!(alpha(&img, x, y) > 180, "dot at ({x}, {y})");
    }
    // Two pixels left of the dot lie between it and the circle.
    assert_eq!(alpha(&img, 9, 16), 0, "the dot has radius 1");
}

#[test]
fn icon_scales_into_rect() {
    // x.svg: M18 6 L6 18 and M6 6 L18 18, painted at 2x offset by (100, 50).
    let (drawn, img) = raster(
        &lucide::X,
        Rect::from_min_size(pos2(100.0, 50.0), vec2(48.0, 48.0)),
    );
    assert_eq!(
        drawn,
        Rect::from_min_size(pos2(100.0, 50.0), vec2(48.0, 48.0))
    );
    assert_eq!(img.size, [48, 48]);
    // (12, 12) is the crossing, at 2x pixel (24, 24); the stroke is 4 px
    // wide, so a pixel 1.4 px off the line is covered, 2.8 px off is not.
    assert_eq!(alpha(&img, 24, 24), 255);
    assert_eq!(alpha(&img, 25, 23), 255, "1.4 px off the line");
    assert_eq!(alpha(&img, 28, 24), 0, "stroke scales with the icon");
}

#[test]
fn closed_subpaths_stay_closed() {
    use shadcn_egui::{IconShape, Seg};
    // An open and a closed L: only the closed one draws its third edge,
    // from (20, 20) back to (12, 4), passing (16, 12).
    static OPEN: [Seg; 3] = [Seg::M(12.0, 4.0), Seg::L(4.0, 20.0), Seg::L(20.0, 20.0)];
    static CLOSED: [Seg; 4] = [
        Seg::M(12.0, 4.0),
        Seg::L(4.0, 20.0),
        Seg::L(20.0, 20.0),
        Seg::Z,
    ];
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0));
    let at = |segs: &'static [Seg]| {
        let icon = Icon {
            name: "triangle",
            size: 24.0,
            shape: IconShape::Segs(segs),
        };
        alpha(&raster(&icon, rect).1, 15, 11)
    };
    assert_eq!(at(&OPEN), 0, "an open path has no closing edge");
    assert!(at(&CLOSED) > 200, "Z draws the closing edge");
}

#[test]
fn icon_widgets_are_labelled() {
    use std::cell::Cell;
    let clicked = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        Theme::vega_dark().apply(ui.ctx());
        // No name of its own: a visible label names it.
        let label = ui.label("Microphone");
        ui.add(IconView::new(&lucide::MIC)).labelled_by(label.id);
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
    h.get_by_role_and_label(egui::accesskit::Role::Image, "Microphone");
    assert!(
        h.query_by_label("mic").is_none(),
        "the icon's internal name is no label"
    );
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

#[test]
fn core_types_are_reexported() {
    let a: shadcn_egui::Icon = lucide::X;
    let b: shadcn_egui_core::Icon = a;
    let c: shadcn_egui::components::icon::Icon = b;
    assert_eq!(c.name, "x");
    let mut layer: shadcn_egui::IconLayer = c.layers().next().expect("a layer");
    assert!(matches!(layer.segs.next(), Some(shadcn_egui::Seg::M(..))));
}
