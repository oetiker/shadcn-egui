mod common;

use egui_kittest::Harness;
use egui_shadcn::{layout, Theme};

#[test]
fn row_and_vstack_compose() {
    let mut h = Harness::new_ui(|ui| {
        Theme::vega_dark().apply(ui.ctx());
        layout::vstack(ui, 12.0, |ui| {
            ui.label("top");
            layout::row(ui, 8.0, |ui| {
                ui.label("a");
                ui.label("b");
            });
        });
    });
    h.run();
    assert!(h.ctx.viewport_rect().width() > 0.0);
}

#[test]
fn space_between_composes() {
    let mut h = Harness::new_ui(|ui| {
        Theme::vega_dark().apply(ui.ctx());
        layout::space_between(
            ui,
            |ui| {
                ui.label("left");
            },
            |ui| {
                ui.label("right");
            },
        );
    });
    h.run();
    assert!(h.ctx.viewport_rect().width() > 0.0);
}

/// `form_row`: the control starts after the label column, and the row is at
/// least a text-sm line high.
#[test]
fn form_row_puts_the_control_after_the_label_column() {
    let theme = Theme::vega_dark();
    let mut control = egui::Rect::NOTHING;
    let mut gap = 0.0;
    let row = common::measure(&theme, |ui| {
        ui.scope(|ui| {
            gap = ui.spacing().item_spacing.x;
            ui.set_max_width(380.0);
            layout::form_row(ui, 100.0, "Name", |ui| control = ui.label("hi").rect);
        })
        .response
    });
    assert_eq!(control.left() - row.left(), 100.0 + gap);
    assert!(row.height() >= theme.line_px(egui_shadcn::style::TextSize::Sm));
}

/// `layout::card` is the default-size card.
#[test]
fn layout_card_matches_the_default_card() {
    use egui_shadcn::components::card::Card;
    let theme = Theme::vega_dark();
    let a = common::measure(&theme, |ui| ui.scope(|ui| layout::card(ui, |ui| { ui.label("x"); })).response);
    let b = common::measure(&theme, |ui| ui.scope(|ui| Card::new().show(ui, |ui| { ui.label("x"); })).response);
    assert_eq!(a, b);
}

#[test]
fn wrap_row_wraps_when_out_of_width() {
    use std::cell::RefCell;
    let rects = RefCell::new(Vec::new());
    let mut h = Harness::builder()
        .with_size(egui::vec2(200.0, 200.0))
        .build_ui(|ui| {
            Theme::vega_dark().apply(ui.ctx());
            rects.borrow_mut().clear();
            layout::wrap_row(ui, 8.0, |ui| {
                for _ in 0..4 {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(80.0, 20.0), egui::Sense::hover());
                    rects.borrow_mut().push(r);
                }
            });
        });
    h.run();
    let r = rects.borrow();
    assert_eq!(r[0].top(), r[1].top(), "first two items share a line");
    assert!(r[2].top() >= r[0].bottom() + 8.0, "third item wraps with the vertical gap: {:?}", *r);
    assert_eq!(r[1].left() - r[0].right(), 8.0, "horizontal gap");
}
