mod common;

use egui_kittest::Harness;
use shadcn_egui::{layout, Theme};

/// `vstack` puts its gap between rows, `row` between items.
#[test]
fn row_and_vstack_compose() {
    let theme = Theme::vega_dark();
    let (mut top, mut a, mut b) = (
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
    );
    let mut row_top = 0.0;
    common::measure(&theme, |ui| {
        layout::vstack(ui, 12.0, |ui| {
            top = ui.label("top").rect;
            layout::row(ui, 8.0, |ui| {
                row_top = ui.max_rect().top();
                a = ui.label("a").rect;
                b = ui.label("b").rect;
            });
        });
        ui.label("end")
    });
    assert_eq!(row_top - top.bottom(), 12.0, "vstack gap");
    assert_eq!(b.left() - a.right(), 8.0, "row gap: {a:?} {b:?}");
    assert_eq!(a.left(), top.left());
}

/// `space_between`: the left part hugs the start, the right part the end.
#[test]
fn space_between_composes() {
    let theme = Theme::vega_dark();
    let (mut left, mut right, mut row) = (
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
    );
    common::measure(&theme, |ui| {
        ui.set_width(300.0);
        row = ui.max_rect();
        layout::space_between(
            ui,
            |ui| left = ui.label("left").rect,
            |ui| right = ui.label("right").rect,
        );
        ui.label("end")
    });
    assert_eq!(left.left(), row.left());
    assert_eq!(right.right(), row.right());
    assert!(right.left() > left.right() + 100.0, "{left:?} {right:?}");
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
    assert!(row.height() >= theme.line_px(shadcn_egui::style::TextSize::Sm));
}

/// `layout::card` is the default-size card.
#[test]
fn layout_card_matches_the_default_card() {
    use shadcn_egui::components::card::Card;
    let theme = Theme::vega_dark();
    let a = common::measure(&theme, |ui| {
        ui.scope(|ui| {
            layout::card(ui, |ui| {
                ui.label("x");
            })
        })
        .response
    });
    let b = common::measure(&theme, |ui| {
        ui.scope(|ui| {
            Card::new().show(ui, |ui| {
                ui.label("x");
            })
        })
        .response
    });
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
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(80.0, 20.0), egui::Sense::hover());
                    rects.borrow_mut().push(r);
                }
            });
        });
    h.run();
    let r = rects.borrow();
    assert_eq!(r[0].top(), r[1].top(), "first two items share a line");
    assert!(
        r[2].top() >= r[0].bottom() + 8.0,
        "third item wraps with the vertical gap: {:?}",
        *r
    );
    assert_eq!(r[1].left() - r[0].right(), 8.0, "horizontal gap");
}

/// The vertical centres of a row and its children (a label, then a button
/// `tall`er or shorter than egui's row height), after the row has settled.
fn row_centres(ctx: &egui::Context, tall: bool, wrap: Wrap) -> (f32, f32, f32) {
    use shadcn_egui::components::button::{Button, ButtonSize};
    let (mut row, mut a, mut b) = (
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
    );
    let size = if tall { ButtonSize::Lg } else { ButtonSize::Sm };
    // Each run settles within egui's passes; a third run checks it stays put.
    for _ in 0..3 {
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            row = ui
                .scope(|ui| {
                    layout::row(ui, 8.0, |ui| {
                        a = match wrap {
                            Wrap::None => ui.label("label").rect,
                            Wrap::EnabledUi => {
                                ui.add_enabled_ui(true, |ui| ui.label("label")).inner.rect
                            }
                            Wrap::Scope => ui.scope(|ui| ui.label("label")).inner.rect,
                        };
                        b = ui.add(Button::new("Button").size(size)).rect;
                    });
                })
                .response
                .rect;
        });
    }
    (row.center().y, a.center().y, b.center().y)
}

/// How the short child sits in the row: bare, or inside a child Ui that
/// egui makes as high as the row.
#[derive(Clone, Copy, Debug)]
enum Wrap {
    None,
    EnabledUi,
    Scope,
}

/// `row` centres a short child placed before a child taller than egui's
/// row height, and shrinks again when the tall child goes, also when the
/// short child sits in a child Ui.
#[test]
fn row_centres_children_of_mixed_height() {
    for wrap in [Wrap::None, Wrap::EnabledUi, Wrap::Scope] {
        for (name, theme) in common::themes() {
            let ctx = egui::Context::default();
            theme.apply(&ctx);
            let (row, a, b) = row_centres(&ctx, true, wrap);
            assert_eq!(
                (a, b),
                (row, row),
                "{name} {wrap:?} tall: row {row}, label {a}, button {b}"
            );
            let tall_row = row;
            let (row, a, b) = row_centres(&ctx, false, wrap);
            assert_eq!(
                (a, b),
                (row, row),
                "{name} {wrap:?} short: row {row}, label {a}, button {b}"
            );
            assert!(
                row < tall_row,
                "{name} {wrap:?}: the row shrinks: {row} vs {tall_row}"
            );
            assert_eq!(row, ctx.global_style().spacing.interact_size.y / 2.0);
        }
    }
}

/// A row that mixes a nested `vstack` (egui puts it at the top) with a tall
/// widget settles: after the first frame no pass is redone.
#[test]
fn row_with_a_nested_stack_settles() {
    use shadcn_egui::components::slider::{Slider, SliderOrientation};
    for (name, theme) in common::themes() {
        let ctx = egui::Context::default();
        theme.apply(&ctx);
        let (mut a, mut b) = (30.0, 50.0);
        let mut run = || {
            let mut row = egui::Rect::NOTHING;
            let out = ctx.run_ui(egui::RawInput::default(), |ui| {
                row = ui
                    .scope(|ui| {
                        layout::row(ui, 24.0, |ui| {
                            layout::vstack(ui, 16.0, |ui| {
                                ui.set_width(200.0);
                                ui.add(Slider::new(&mut a, 0.0..=100.0).label("A"));
                                ui.add(Slider::new(&mut a, 0.0..=100.0).label("B"));
                            });
                            ui.add(
                                Slider::new(&mut b, 0.0..=100.0)
                                    .orientation(SliderOrientation::Vertical)
                                    .label("C"),
                            );
                        });
                    })
                    .response
                    .rect;
            });
            (row, out.platform_output.num_completed_passes)
        };
        let _ = run();
        let (first, _) = run();
        let (second, passes) = run();
        assert_eq!(first, second, "{name}: the row keeps its height");
        assert_eq!(passes, 1, "{name}: a settled row redoes no pass");
    }
}

/// A nested `vstack` counts with the height of what it holds: a button
/// beside a three-line stack is centred on the stack.
#[test]
fn row_centres_a_widget_beside_a_nested_stack() {
    use shadcn_egui::components::button::Button;
    let ctx = egui::Context::default();
    Theme::vega_dark().apply(&ctx);
    let (mut stack, mut button) = (egui::Rect::NOTHING, egui::Rect::NOTHING);
    for _ in 0..3 {
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            layout::row(ui, 8.0, |ui| {
                stack = layout::vstack(ui, 8.0, |ui| {
                    ui.label("one");
                    ui.label("two");
                    ui.label("three");
                    ui.min_rect()
                });
                button = ui.add(Button::new("Button")).rect;
            });
        });
    }
    assert!(stack.height() > button.height(), "{stack:?} {button:?}");
    assert_eq!(button.center().y, stack.center().y, "{stack:?} {button:?}");
}
