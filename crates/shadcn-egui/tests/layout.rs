mod common;

use egui_kittest::Harness;
use shadcn_egui::{layout, Theme};

/// `vstack` puts its gap between rows, `row` between items.
#[test]
fn row_and_vstack_compose() {
    let mode = egui::Theme::Dark;
    let theme = Theme::vega();
    let (mut top, mut a, mut b) = (
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
    );
    let mut row_top = 0.0;
    common::measure(&theme, mode, |ui| {
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
    let mode = egui::Theme::Dark;
    let theme = Theme::vega();
    let (mut left, mut right, mut row) = (
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
        egui::Rect::NOTHING,
    );
    common::measure(&theme, mode, |ui| {
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
    let mode = egui::Theme::Dark;
    let theme = Theme::vega();
    let mut control = egui::Rect::NOTHING;
    let mut gap = 0.0;
    let row = common::measure(&theme, mode, |ui| {
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

/// `form_row` draws its label as the crate's `Label`: the same recipe text
/// (font, line box, colour), not egui's `ui.label`.
#[test]
fn form_row_label_is_the_crate_label() {
    use shadcn_egui::components::label::Label;
    let mode = egui::Theme::Dark;
    let theme = Theme::vega();
    let alone = common::measure_output(&theme, mode, |ui| ui.add(Label::new("Name"))).1;
    let in_row = common::measure_output(&theme, mode, |ui| {
        ui.scope(|ui| layout::form_row(ui, 100.0, "Name", |_| {}))
            .response
    })
    .1;
    let (a, b) = (
        &common::text_shapes(&alone)[0],
        &common::text_shapes(&in_row)[0],
    );
    assert_eq!(a.galley.rows.len(), b.galley.rows.len());
    for (ra, rb) in a.galley.rows.iter().zip(&b.galley.rows) {
        assert_eq!(ra.size, rb.size);
        assert_eq!(ra.visuals.mesh.vertices, rb.visuals.mesh.vertices);
    }
    assert_eq!(common::text_colour(&alone), common::text_colour(&in_row));
}

/// `layout::card` is the default-size card.
#[test]
fn layout_card_matches_the_default_card() {
    use shadcn_egui::components::card::Card;
    let mode = egui::Theme::Dark;
    let theme = Theme::vega();
    let a = common::measure(&theme, mode, |ui| {
        ui.scope(|ui| {
            layout::card(ui, |ui| {
                ui.label("x");
            })
        })
        .response
    });
    let b = common::measure(&theme, mode, |ui| {
        ui.scope(|ui| {
            Card::new().show(ui, |ui| {
                ui.label("x");
            })
        })
        .response
    });
    assert_eq!(a, b);
}

/// In unbounded room the card takes its content minimum, its side padding
/// (conformance 3.1), never an infinite width.
#[test]
fn a_card_in_unbounded_room_takes_its_padding() {
    use shadcn_egui::components::card::Card;
    let mode = egui::Theme::Dark;
    let theme = Theme::vega();
    let rect = common::measure(&theme, mode, |ui| {
        ui.allocate_ui(egui::vec2(f32::INFINITY, 400.0), |ui| {
            ui.scope(|ui| Card::new().show(ui, |_| ())).response
        })
        .inner
    });
    // vega's card content is `px-6`.
    assert_eq!(rect.width(), 48.0, "{rect:?}");
}

#[test]
fn wrap_row_wraps_when_out_of_width() {
    use std::cell::RefCell;
    let rects = RefCell::new(Vec::new());
    let mut h = Harness::builder()
        .with_size(egui::vec2(200.0, 200.0))
        .build_ui(|ui| {
            Theme::vega().apply(ui.ctx());
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
    assert_eq!(
        r[2].top() - r[0].bottom(),
        8.0,
        "lines as high as their items, `gap` apart"
    );
}

/// `vstack` items of a `wrap_row` wrap as a whole, as flex items: each
/// keeps its unwrapped width, and one that does not fit the rest of the
/// line starts the next line. Under both app loops: the stack redoes a pass
/// to learn its width.
#[test]
fn vstack_items_wrap_in_a_wrap_row() {
    for app_loop in common::Loop::BOTH {
        let ctx = egui::Context::default();
        let mut rects = Vec::new();
        for _ in 0..3 {
            rects.clear();
            let _ = common::pass(app_loop, &ctx, vec![], |ui| {
                rects.clear();
                ui.set_max_width(300.0);
                layout::wrap_row(ui, 8.0, |ui| {
                    for _ in 0..4 {
                        let r = layout::vstack(ui, 4.0, |ui| ui.label("a long unbroken name").rect);
                        rects.push(r);
                    }
                });
            });
        }
        let one_line = rects[0].height();
        assert!(
            rects.iter().all(|r| r.height() == one_line),
            "{app_loop:?}: no label wraps: {rects:?}"
        );
        assert!(
            rects.iter().all(|r| r.right() <= 300.0),
            "{app_loop:?}: none overflows: {rects:?}"
        );
        let lines = rects.iter().filter(|r| r.top() > rects[0].top()).count();
        assert!(lines > 0, "{app_loop:?}: some items wrap: {rects:?}");
    }
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
        for (name, theme, mode) in common::themes() {
            let ctx = egui::Context::default();
            theme.apply(&ctx);
            ctx.set_theme(mode);
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
    for (name, theme, mode) in common::themes() {
        let ctx = egui::Context::default();
        theme.apply(&ctx);
        ctx.set_theme(mode);
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
    Theme::vega().apply(&ctx);
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

/// A stack relearns its unwrapped width when its image is swapped for one
/// of another intrinsic size that draws at the same size (both shrink to
/// the stack's width, same aspect): the row ends as one that held the new
/// image from the start.
#[test]
fn a_stack_relearns_its_width_when_its_image_keeps_its_drawn_size() {
    fn frame(ctx: &egui::Context, image: &egui::TextureHandle) -> [egui::Rect; 3] {
        let mut out = [egui::Rect::NOTHING; 3];
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.scope(|ui| {
                ui.set_max_width(200.0);
                layout::row(ui, 8.0, |ui| {
                    out[0] = layout::vstack(ui, 4.0, |ui| {
                        let max = ui.available_width();
                        ui.add(
                            egui::Image::from_texture(image)
                                .fit_to_original_size(1.0)
                                .max_width(max),
                        )
                        .rect
                    });
                    out[1] = layout::vstack(ui, 4.0, |ui| {
                        ui.label("Receive emails about new products.").rect
                    });
                    out[2] = ui
                        .allocate_exact_size(egui::vec2(60.0, 20.0), egui::Sense::hover())
                        .0;
                });
            });
        });
        out
    }
    let texture = |ctx: &egui::Context, name: &str, w: usize| {
        let pixels = egui::ColorImage::new([w, w / 3], vec![egui::Color32::GRAY; w * (w / 3)]);
        ctx.load_texture(name, pixels, Default::default())
    };
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    let (small, large) = (texture(&ctx, "small", 120), texture(&ctx, "large", 240));
    let mut before = [egui::Rect::NOTHING; 3];
    for _ in 0..4 {
        before = frame(&ctx, &small);
    }
    let switched = frame(&ctx, &large);
    assert_eq!(
        before[0].size(),
        switched[0].size(),
        "the case needs an equal drawn size"
    );
    let mut after = switched;
    for _ in 0..4 {
        after = frame(&ctx, &large);
    }
    let fresh_ctx = egui::Context::default();
    Theme::vega().apply(&fresh_ctx);
    let fresh_large = texture(&fresh_ctx, "large", 240);
    let mut fresh = [egui::Rect::NOTHING; 3];
    for _ in 0..5 {
        fresh = frame(&fresh_ctx, &fresh_large);
    }
    assert_ne!(before, fresh, "the case needs another split");
    assert_eq!(after, fresh, "the row relearns the image's width");
}

/// Content for the sizing-pass tests: it pushes the rects to compare. The
/// flag says whether its probe widget measures smaller in a sizing pass.
type Content<'a> = &'a dyn Fn(&mut egui::Ui, bool, &mut Vec<egui::Rect>);

/// Where [`frame_in`] lays out the content.
#[derive(Clone, Copy, Debug)]
enum Host {
    /// The screen's `Ui`.
    Screen,
    /// A raw `egui::Area`, forced into a sizing pass when `true`.
    Area(bool),
}

/// One frame of `content` in `host`: how many passes it took, and the
/// rects `content` pushed in its last pass, relative to the content's
/// top-left corner.
fn frame_in(
    ctx: &egui::Context,
    host: Host,
    sensitive: bool,
    content: Content,
) -> (u64, Vec<egui::Rect>) {
    let before = ctx.cumulative_pass_nr();
    let mut rects = Vec::new();
    let mut run = |ui: &mut egui::Ui| {
        rects.clear();
        content(ui, sensitive, &mut rects);
        let origin = ui.max_rect().min.to_vec2();
        rects.iter_mut().for_each(|r| *r = r.translate(-origin));
    };
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| match host {
        Host::Screen => {
            ui.scope(&mut run);
        }
        Host::Area(force) => {
            egui::Area::new(egui::Id::new("popover"))
                .sizing_pass(force)
                .show(ui.ctx(), &mut run);
        }
    });
    (ctx.cumulative_pass_nr() - before, rects)
}

/// `content` in an `egui::Area`, checked against egui's sizing pass.
///
/// The Area's first frame starts with a sizing pass. Where the content
/// measures the same in it, the frame draws what the content draws on the
/// screen and the next frame takes one pass. Where the content measures
/// smaller there, the next frame may redo its pass (the sizing pass knew
/// no better), and the frame after takes one.
///
/// A sizing pass the app forces on the settled Area keeps nothing the
/// next frame has to undo: both take one pass, and that frame draws what
/// the settled one drew.
fn settles_in_an_area(name: &str, content: Content) {
    let fresh = || {
        let ctx = egui::Context::default();
        Theme::vega().apply(&ctx);
        ctx
    };
    let screen = fresh();
    let mut outside = Vec::new();
    for _ in 0..3 {
        outside = frame_in(&screen, Host::Screen, false, content).1;
    }
    let ctx = fresh();
    let (_, first) = frame_in(&ctx, Host::Area(false), false, content);
    let (passes, second) = frame_in(&ctx, Host::Area(false), false, content);
    assert_eq!(
        first, outside,
        "{name}: the sizing frame draws as the screen"
    );
    assert_eq!(passes, 1, "{name}: the frame after the sizing frame");
    assert_eq!(
        second, outside,
        "{name}: the next frame draws as the screen"
    );

    let ctx = fresh();
    let _ = frame_in(&ctx, Host::Area(false), true, content);
    let (_, second) = frame_in(&ctx, Host::Area(false), true, content);
    let (passes, third) = frame_in(&ctx, Host::Area(false), true, content);
    assert_eq!(second, outside, "{name}: a smaller sizing pass, next frame");
    assert_eq!(passes, 1, "{name}: a smaller sizing pass, a still frame");
    assert_eq!(
        third, outside,
        "{name}: a smaller sizing pass, a still frame"
    );

    let (passes, _) = frame_in(&ctx, Host::Area(true), true, content);
    assert_eq!(passes, 1, "{name}: a forced sizing pass");
    let (passes, after) = frame_in(&ctx, Host::Area(false), true, content);
    assert_eq!(passes, 1, "{name}: the frame after a forced sizing pass");
    assert_eq!(
        after, outside,
        "{name}: a forced sizing pass changes nothing"
    );
}

/// A widget that measures smaller in a sizing pass when `sensitive`, as
/// egui's `Separator` (no length) and `Sides` do.
fn probe(ui: &mut egui::Ui, sensitive: bool, size: egui::Vec2) -> egui::Rect {
    let size = if sensitive && ui.is_sizing_pass() {
        size / 2.0
    } else {
        size
    };
    ui.allocate_exact_size(size, egui::Sense::hover()).0
}

/// The row's height (`request_discard` "layout::row content height
/// changed") in an `egui::Area`.
#[test]
fn row_height_settles_in_an_area() {
    settles_in_an_area("row", &|ui, sensitive, out| {
        layout::row(ui, 8.0, |ui| {
            out.push(ui.button("a").rect);
            out.push(probe(ui, sensitive, egui::vec2(20.0, 60.0)));
            out.push(ui.label("b").rect);
        });
    });
}

/// A `vstack`'s width in a `wrap_row` (`request_discard` "layout::vstack
/// width changed in a wrap_row") in an `egui::Area`.
#[test]
fn wrap_row_stack_settles_in_an_area() {
    settles_in_an_area("wrap_row", &|ui, sensitive, out| {
        ui.set_max_width(200.0);
        layout::wrap_row(ui, 8.0, |ui| {
            out.push(ui.label("one").rect);
            let stack = layout::vstack(ui, 4.0, |ui| {
                ui.label("top");
                out.push(probe(ui, sensitive, egui::vec2(120.0, 10.0)));
                ui.min_rect()
            });
            out.push(stack);
            out.push(ui.label("two").rect);
        });
    });
}

/// The widths a row plans for its `vstack` items (`request_discard`
/// "layout: a vstack's width in a row changed") in an `egui::Area`, where
/// the stacks fit and where they shrink.
#[test]
fn row_stack_plan_settles_in_an_area() {
    for room in [400.0, 160.0] {
        settles_in_an_area(&format!("row plan in {room}"), &|ui, sensitive, out| {
            ui.set_max_width(room);
            layout::row(ui, 8.0, |ui| {
                let stack = layout::vstack(ui, 4.0, |ui| {
                    ui.label("Receive emails about new products.");
                    out.push(probe(ui, sensitive, egui::vec2(120.0, 10.0)));
                    ui.min_rect()
                });
                out.push(stack);
                out.push(ui.button("Go").rect);
            });
        });
    }
}

/// A shrunk `vstack` whose content measures another size at its planned
/// width (`request_discard` "layout: a vstack in a row measures its
/// content") in an `egui::Area`.
#[test]
fn row_stack_measure_settles_in_an_area() {
    settles_in_an_area("row measure", &|ui, sensitive, out| {
        ui.set_max_width(160.0);
        layout::row(ui, 8.0, |ui| {
            let stack = layout::vstack(ui, 4.0, |ui| {
                ui.label("Receive emails about new products.");
                out.push(probe(ui, sensitive, egui::vec2(60.0, 10.0)));
                ui.min_rect()
            });
            out.push(stack);
            out.push(ui.button("Go").rect);
        });
    });
}

/// A row of a text label, a box of `tall` points and a stretch item, in
/// `host`: the passes of the frame, what `stretch_height` gave, and the
/// rects of the label, the box and the stretch item.
fn stretch_frame(
    ctx: &egui::Context,
    host: Host,
    tall: f32,
) -> (u64, Option<f32>, Vec<egui::Rect>) {
    let given = std::cell::Cell::new(None);
    let (passes, rects) = frame_in(ctx, host, false, &|ui, _, out| {
        layout::row(ui, 8.0, |ui| {
            out.push(ui.label("Ctrl").rect);
            if tall > 0.0 {
                out.push(
                    ui.allocate_exact_size(egui::vec2(10.0, tall), egui::Sense::hover())
                        .0,
                );
            }
            let height = layout::stretch_height(ui);
            given.set(height);
            let size = egui::vec2(1.0, height.unwrap_or(0.0));
            out.push(ui.allocate_exact_size(size, egui::Sense::hover()).0);
        });
    });
    (passes, given.get(), rects)
}

/// `stretch_height` in a row is the height of the row's tallest other
/// item, below the row's `interact_size` floor too, follows it down when
/// it shrinks, and settles: a still frame redoes no pass. In an
/// `egui::Area` it gives the same.
#[test]
fn stretch_height_is_the_tallest_other_item() {
    for host in [Host::Screen, Host::Area(false)] {
        let ctx = egui::Context::default();
        Theme::vega().apply(&ctx);
        let floor = ctx.global_style().spacing.interact_size.y;
        let mut label = 0.0;
        for tall in [60.0, 0.0] {
            for _ in 0..2 {
                let _ = stretch_frame(&ctx, host, tall);
            }
            let (passes, height, rects) = stretch_frame(&ctx, host, tall);
            label = rects[0].height();
            let expected = if tall > 0.0 { tall } else { label };
            assert_eq!(height, Some(expected), "tall {tall}");
            let stretch = rects.last().copied().unwrap();
            assert_eq!(stretch.height(), expected, "tall {tall}");
            let tallest = rects[usize::from(tall > 0.0)];
            assert_eq!(
                stretch.y_range(),
                tallest.y_range(),
                "spans the tallest: {rects:?}"
            );
            assert_eq!(passes, 1, "tall {tall}: a still frame");
        }
        assert!(label < floor, "the case needs text below the floor");
    }
}

/// In a column or outside any row, `stretch_height` gives no height.
#[test]
fn stretch_height_is_none_outside_a_row() {
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    let mut seen = Vec::new();
    for _ in 0..2 {
        seen.clear();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            seen.push(layout::stretch_height(ui));
            layout::row(ui, 8.0, |ui| {
                ui.label("a");
                layout::vstack(ui, 4.0, |ui| seen.push(layout::stretch_height(ui)));
            });
        });
    }
    assert_eq!(seen, vec![None; 2]);
}

/// A horizontal separator in a `vstack` in a `row`, the crate's and
/// egui's own, neither panics. The crate's has no content width
/// (conformance 3.1), so the stack stays as wide as its label; egui's
/// fills the room it gets, so the stack takes the room the row's other
/// items leave. Both settle: a still frame takes one pass.
#[test]
fn a_separator_in_a_stack_in_a_row_settles() {
    for egui_own in [false, true] {
        let ctx = egui::Context::default();
        Theme::vega().apply(&ctx);
        let mut rects = [egui::Rect::NOTHING; 5];
        let mut passes = 0;
        for _ in 0..3 {
            let before = ctx.cumulative_pass_nr();
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(300.0);
                rects[0] = ui.max_rect();
                layout::row(ui, 8.0, |ui| {
                    rects[3] = layout::vstack(ui, 4.0, |ui| {
                        rects[1] = ui.label("Account").rect;
                        rects[2] = if egui_own {
                            ui.separator().rect
                        } else {
                            ui.add(shadcn_egui::components::separator::Separator::new())
                                .rect
                        };
                        ui.min_rect()
                    });
                    rects[4] = ui.button("Go").rect;
                });
            });
            passes = ctx.cumulative_pass_nr() - before;
        }
        let [room, label, rule, stack, go] = rects;
        assert_eq!(passes, 1, "egui {egui_own}: a still frame");
        if egui_own {
            assert_eq!(rule.width(), stack.width(), "{rects:?}");
            assert!(stack.width() > label.width(), "{rects:?}");
            assert_eq!(go.right(), room.right(), "{rects:?}");
        } else {
            assert_eq!(rule.width(), 0.0, "{rects:?}");
            assert_eq!(stack.width(), label.width(), "{rects:?}");
            assert_eq!(go.left(), stack.right() + 8.0, "{rects:?}");
        }
    }
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    let mut outside = (0.0, egui::Rect::NOTHING);
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
        outside = (
            ui.available_width(),
            ui.add(shadcn_egui::components::separator::Separator::new())
                .rect,
        );
    });
    assert_eq!(outside.1.width(), outside.0, "outside a row it spans");
}

/// A vertical separator is `self-stretch`: in a `row` it spans the row's
/// tallest other item, below the row's `interact_size` floor too, and a
/// still frame takes one pass; outside a crate row it takes the height it
/// is given.
#[test]
fn a_vertical_separator_stretches_in_a_row() {
    use shadcn_egui::components::separator::separator_vertical;
    let rule = |out: &egui::FullOutput, theme: &Theme| {
        let border = theme.palette_of(egui::Theme::Dark).border;
        common::rect_shapes(out)
            .into_iter()
            .filter(|r| r.fill == border)
            .map(|r| r.rect)
            .collect::<Vec<_>>()
    };
    for tall in [60.0, 0.0] {
        let ctx = egui::Context::default();
        let theme = Theme::vega();
        theme.apply(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        let floor = ctx.global_style().spacing.interact_size.y;
        let (mut label, mut passes) = (egui::Rect::NOTHING, 0);
        let mut out = egui::FullOutput::default();
        for _ in 0..3 {
            let before = ctx.cumulative_pass_nr();
            out = ctx.run_ui(egui::RawInput::default(), |ui| {
                layout::row(ui, 8.0, |ui| {
                    label = ui.label("Docs").rect;
                    separator_vertical(ui);
                    if tall > 0.0 {
                        ui.allocate_exact_size(egui::vec2(10.0, tall), egui::Sense::hover());
                    }
                });
            });
            passes = ctx.cumulative_pass_nr() - before;
        }
        let rules = rule(&out, &theme);
        assert_eq!(rules.len(), 1, "{rules:?}");
        let expected = if tall > 0.0 { tall } else { label.height() };
        assert_eq!(rules[0].height(), expected, "tall {tall}");
        assert!(tall > 0.0 || expected < floor, "text below the floor");
        assert_eq!(passes, 1, "tall {tall}: a still frame");
    }
    let ctx = egui::Context::default();
    let theme = Theme::vega();
    theme.apply(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    let mut out = egui::FullOutput::default();
    for _ in 0..2 {
        out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.horizontal(|ui| {
                ui.set_height(24.0);
                separator_vertical(ui);
            });
        });
    }
    assert_eq!(rule(&out, &theme)[0].height(), 24.0, "outside a crate row");
}

/// A box of `w` by `h` points.
fn boxed(ui: &mut egui::Ui, w: f32, h: f32) -> egui::Rect {
    ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover())
        .0
}

/// A stretch item: a rule as high as `stretch_height` gives, 0 without.
fn stretch_rule(ui: &mut egui::Ui) -> egui::Rect {
    let height = layout::stretch_height(ui).unwrap_or(0.0);
    boxed(ui, 1.0, height)
}

/// `content` after it settled in `host`: the passes of a still frame and
/// the rects it pushed.
fn settled(ctx: &egui::Context, host: Host, content: Content) -> (u64, Vec<egui::Rect>) {
    for _ in 0..2 {
        let _ = frame_in(ctx, host, false, content);
    }
    frame_in(ctx, host, false, content)
}

fn vega() -> egui::Context {
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    ctx
}

/// In a `wrap_row` a stretch item is as high as the tallest other item of
/// its own line (`flex-wrap` with `align-self: stretch`), on the screen
/// and in an `egui::Area`, and a still frame takes one pass.
#[test]
fn stretch_height_is_the_tallest_other_item_of_its_wrap_row_line() {
    for host in [Host::Screen, Host::Area(false)] {
        let (passes, r) = settled(&vega(), host, &|ui, _, out| {
            ui.set_max_width(200.0);
            layout::wrap_row(ui, 8.0, |ui| {
                out.push(boxed(ui, 80.0, 60.0));
                out.push(stretch_rule(ui));
                out.push(boxed(ui, 150.0, 20.0));
                out.push(stretch_rule(ui));
            });
        });
        assert!(r[2].top() > r[0].bottom(), "{host:?}: two lines: {r:?}");
        assert_eq!(r[1].y_range(), r[0].y_range(), "{host:?}: line 1: {r:?}");
        assert_eq!(r[3].y_range(), r[2].y_range(), "{host:?}: line 2: {r:?}");
        assert_eq!(passes, 1, "{host:?}: a still frame");
    }
}

/// `space_between`'s two sides are one row: a stretch item on either side
/// is as high as the tallest other item on both, on the screen and in an
/// `egui::Area`.
#[test]
fn stretch_height_spans_both_sides_of_space_between() {
    for host in [Host::Screen, Host::Area(false)] {
        let (passes, r) = settled(&vega(), host, &|ui, _, out| {
            ui.set_max_width(300.0);
            let out = std::cell::RefCell::new(out);
            layout::space_between(
                ui,
                |ui| {
                    let mut out = out.borrow_mut();
                    out.push(boxed(ui, 10.0, 12.0));
                    out.push(stretch_rule(ui));
                },
                |ui| {
                    let mut out = out.borrow_mut();
                    out.push(boxed(ui, 10.0, 40.0));
                    out.push(stretch_rule(ui));
                },
            );
        });
        assert_eq!(r[1].height(), 40.0, "{host:?}: left side: {r:?}");
        assert_eq!(
            r[3].y_range(),
            r[2].y_range(),
            "{host:?}: right side: {r:?}"
        );
        assert_eq!(passes, 1, "{host:?}: a still frame");
    }
}

/// A card footer is a row: a stretch item in it is as high as the
/// footer's tallest other item, on the screen and in an `egui::Area`.
#[test]
fn stretch_height_works_in_a_card_footer() {
    use shadcn_egui::components::card::Card;
    for host in [Host::Screen, Host::Area(false)] {
        let (passes, r) = settled(&vega(), host, &|ui, _, out| {
            ui.set_max_width(300.0);
            let out = std::cell::RefCell::new(out);
            Card::new()
                .footer(|ui| {
                    let mut out = out.borrow_mut();
                    out.push(boxed(ui, 10.0, 40.0));
                    out.push(stretch_rule(ui));
                    out.push(boxed(ui, 10.0, 12.0));
                })
                .show(ui, |ui| {
                    ui.label("body");
                });
        });
        assert_eq!(r[1].y_range(), r[0].y_range(), "{host:?}: {r:?}");
        assert_eq!(passes, 1, "{host:?}: a still frame");
    }
}

/// A `wrap_row` centres each item on its line (`align-items: center`): an
/// item placed before a taller one is centred too, and the line shrinks
/// when the tall item goes. On the screen and in an `egui::Area`, and a
/// still frame takes one pass.
#[test]
fn wrap_row_centres_items_on_their_line() {
    for host in [Host::Screen, Host::Area(false)] {
        let ctx = vega();
        for tall in [38.0, 0.0] {
            let (passes, r) = settled(&ctx, host, &|ui, _, out| {
                layout::wrap_row(ui, 8.0, |ui| {
                    out.push(boxed(ui, 10.0, 17.0));
                    if tall > 0.0 {
                        out.push(boxed(ui, 10.0, tall));
                    }
                    out.push(boxed(ui, 10.0, 12.0));
                });
            });
            let centre = r[0].center().y;
            assert!(
                r.iter().all(|i| i.center().y == centre),
                "{host:?} tall {tall}: {r:?}"
            );
            let line = r.iter().fold(egui::Rect::NOTHING, |a, b| a.union(*b));
            assert_eq!(line.height(), tall.max(17.0), "{host:?} tall {tall}");
            assert_eq!(passes, 1, "{host:?} tall {tall}: a still frame");
        }
    }
}

/// The height of a `wrap_row`'s first line (`request_discard`
/// "layout::wrap_row line height changed") in an `egui::Area`.
#[test]
fn wrap_row_line_settles_in_an_area() {
    settles_in_an_area("wrap_row line", &|ui, sensitive, out| {
        ui.set_max_width(200.0);
        layout::wrap_row(ui, 8.0, |ui| {
            out.push(boxed(ui, 10.0, 17.0));
            out.push(probe(ui, sensitive, egui::vec2(20.0, 60.0)));
            out.push(boxed(ui, 10.0, 12.0));
        });
    });
}

/// A label in a `wrap_row` is a flex item: after a taller `vstack` it
/// reports its own height, centred on the line, as before the stack; a
/// label that does not fit the rest of the line starts the next line
/// whole. A stack alone on a line too narrow for it still wraps its text.
#[test]
fn a_label_in_a_wrap_row_is_a_flex_item() {
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    let mut rects = [egui::Rect::NOTHING; 5];
    for _ in 0..3 {
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.set_max_width(200.0);
            layout::wrap_row(ui, 8.0, |ui| {
                rects[0] = ui.label("one").rect;
                rects[1] = layout::vstack(ui, 4.0, |ui| {
                    ui.label("top");
                    ui.label("bottom");
                    ui.min_rect()
                });
                rects[2] = ui.label("two").rect;
                rects[3] = ui.label("a label too long for the rest").rect;
                rects[4] = layout::vstack(ui, 4.0, |ui| {
                    ui.label("a stack with more text than one line holds");
                    ui.min_rect()
                });
            });
        });
    }
    let [one, stack, two, long, narrow] = rects;
    assert_eq!(two.height(), one.height(), "{rects:?}");
    assert_eq!(two.center().y, stack.center().y, "{rects:?}");
    assert_eq!(long.left(), one.left(), "starts a line: {rects:?}");
    assert!(long.top() >= stack.bottom(), "{rects:?}");
    assert!(narrow.width() <= 200.0, "wraps: {rects:?}");
    assert!(narrow.height() > one.height(), "wraps: {rects:?}");
}

/// Every line of a `wrap_row` is as high as its own tallest item and
/// centres its items, `gap` below the line before: a line after a taller
/// one, started by a crate widget, is not as high as that one; an item
/// placed before a taller one is centred; and the line, an egui `Label` on
/// it, shrinks when its tall item goes, redoing the changed frame's pass
/// once. On the screen and in an `egui::Area`, and a still frame takes one
/// pass.
#[test]
fn wrap_row_lines_have_their_own_height() {
    use shadcn_egui::components::badge::Badge;
    for host in [Host::Screen, Host::Area(false)] {
        let ctx = vega();
        let content = |tall: f32| {
            move |ui: &mut egui::Ui, _: bool, out: &mut Vec<egui::Rect>| {
                ui.set_max_width(200.0);
                layout::wrap_row(ui, 8.0, |ui| {
                    out.push(boxed(ui, 150.0, 50.0));
                    out.push(ui.add(Badge::new("Badge")).rect);
                    if tall > 0.0 {
                        out.push(boxed(ui, 10.0, tall));
                    }
                    out.push(ui.label("label").rect);
                    out.push(boxed(ui, 10.0, 6.0));
                });
            }
        };
        for (i, tall) in [40.0, 0.0].into_iter().enumerate() {
            let content = content(tall);
            if i > 0 {
                let (passes, _) = frame_in(&ctx, host, false, &content);
                assert_eq!(passes, 2, "{host:?} tall {tall}: the change, once");
            }
            let (passes, r) = settled(&ctx, host, &content);
            let line = &r[1..];
            let centre = line[0].center().y;
            assert!(
                line.iter().all(|i| i.center().y == centre),
                "{host:?} tall {tall}: centred: {r:?}"
            );
            let union = line.iter().fold(egui::Rect::NOTHING, |a, b| a.union(*b));
            let own = line.iter().map(|i| i.height()).fold(0.0, f32::max);
            assert_eq!(union.height(), own, "{host:?} tall {tall}: {r:?}");
            assert!(own < 50.0, "{host:?} tall {tall}: {r:?}");
            assert_eq!(
                union.top() - r[0].bottom(),
                8.0,
                "{host:?} tall {tall}: gap"
            );
            assert_eq!(union.left(), r[0].left(), "{host:?} tall {tall}: line 2");
            assert_eq!(passes, 1, "{host:?} tall {tall}: a still frame");
        }
    }
}

/// The heights of a `wrap_row`'s further lines (`request_discard`
/// "layout::wrap_row line height changed") in an `egui::Area`.
#[test]
fn wrap_row_lines_settle_in_an_area() {
    use shadcn_egui::components::badge::Badge;
    settles_in_an_area("wrap_row lines", &|ui, sensitive, out| {
        ui.set_max_width(200.0);
        layout::wrap_row(ui, 8.0, |ui| {
            out.push(boxed(ui, 150.0, 50.0));
            out.push(ui.add(Badge::new("Badge")).rect);
            out.push(probe(ui, sensitive, egui::vec2(20.0, 40.0)));
            out.push(ui.label("label").rect);
        });
    });
}

/// Pins a known limit of `wrap_row`: a line that a plain egui widget
/// starts is started by egui, which makes it as high as a taller line
/// before it and centres its items in that height. Egui gives the crate
/// no hook between two egui widgets; when it does, this test flips to
/// the line's own height (`wrap_row_lines_have_their_own_height`).
#[test]
fn a_wrap_row_line_an_egui_widget_starts_keeps_the_height_before() {
    for host in [Host::Screen, Host::Area(false)] {
        let (passes, r) = settled(&vega(), host, &|ui, _, out| {
            ui.set_max_width(200.0);
            layout::wrap_row(ui, 8.0, |ui| {
                out.push(boxed(ui, 150.0, 50.0));
                out.push(boxed(ui, 60.0, 10.0));
                out.push(boxed(ui, 10.0, 6.0));
            });
        });
        assert_eq!(r[1].left(), r[0].left(), "{host:?}: line 2: {r:?}");
        assert_eq!(r[1].center().y, r[2].center().y, "{host:?}: {r:?}");
        let band_top = r[0].bottom() + 8.0;
        assert_eq!(
            r[1].center().y - band_top,
            25.0,
            "{host:?}: centred in a 50 px line: {r:?}"
        );
        assert_eq!(passes, 1, "{host:?}: a still frame");
    }
}
