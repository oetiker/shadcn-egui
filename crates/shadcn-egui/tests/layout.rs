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

/// In unbounded room the card takes its content's width, for an empty
/// card its side padding (conformance 3.1), never an infinite width.
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
/// (conformance 3.1), so the stack stays as wide as its label and the
/// rule spans it (`w-full`); egui's fills the room it gets, so the stack
/// takes the room the row's other items leave. Both settle: a still frame
/// takes one pass.
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
            assert_eq!(rule.x_range(), stack.x_range(), "{rects:?}");
            assert!((stack.width() - label.width()).abs() < 0.5, "{rects:?}");
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

/// An app's own widget that allocates through `layout::allocate_item` or
/// `layout::allocate_item_space` places itself in a `wrap_row` as a crate
/// widget does: the line it starts after a taller line is as high as its
/// own tallest item, `gap` below the line before. On the screen and in an
/// `egui::Area`, and a still frame takes one pass.
#[test]
fn an_app_widget_starts_a_wrap_row_line_at_its_own_height() {
    /// An app widget: a box of `w` by `h` points that the crate wraps.
    fn app_box(ui: &mut egui::Ui, w: f32, h: f32) -> egui::Rect {
        layout::allocate_item(ui, egui::vec2(w, h), egui::Sense::click()).0
    }
    /// The same, allocated as `ui.allocate_space` does.
    fn app_space(ui: &mut egui::Ui, w: f32, h: f32) -> egui::Rect {
        layout::allocate_item_space(ui, egui::vec2(w, h)).1
    }
    type Place = fn(&mut egui::Ui, f32, f32) -> egui::Rect;
    for (name, start) in [("item", app_box as Place), ("space", app_space)] {
        for host in [Host::Screen, Host::Area(false)] {
            let (passes, r) = settled(&vega(), host, &|ui, _, out| {
                ui.set_max_width(200.0);
                layout::wrap_row(ui, 8.0, |ui| {
                    out.push(boxed(ui, 150.0, 50.0));
                    out.push(start(ui, 60.0, 10.0));
                    out.push(boxed(ui, 10.0, 6.0));
                });
            });
            assert_eq!(r[1].left(), r[0].left(), "{name} {host:?}: line 2: {r:?}");
            assert_eq!(r[1].top() - r[0].bottom(), 8.0, "{name} {host:?}: {r:?}");
            assert_eq!(r[1].height(), 10.0, "{name} {host:?}: {r:?}");
            assert_eq!(r[1].center().y, r[2].center().y, "{name} {host:?}: {r:?}");
            assert_eq!(passes, 1, "{name} {host:?}: a still frame");
        }
    }
}

/// A crate `Label` longer than a whole `wrap_row` line is a flex item
/// that shrinks to the line (`min-width: auto` is its longest word): it
/// starts a line, takes the line's width and wraps its text inside, and
/// the next crate item starts the line after it. A short label keeps its one
/// line. On the screen and in an `egui::Area`, and a still frame takes one
/// pass.
#[test]
fn a_crate_label_longer_than_a_wrap_row_line_wraps_to_the_line() {
    use shadcn_egui::components::label::Label;
    for host in [Host::Screen, Host::Area(false)] {
        let (passes, r) = settled(&vega(), host, &|ui, _, out| {
            ui.set_max_width(200.0);
            layout::wrap_row(ui, 8.0, |ui| {
                out.push(ui.add(Label::new("short")).rect);
                out.push(
                    ui.add(Label::new(
                        "a crate label with more text than one whole line holds",
                    ))
                    .rect,
                );
                let size = egui::vec2(10.0, 6.0);
                out.push(layout::allocate_item(ui, size, egui::Sense::hover()).0);
            });
        });
        let [short, long, next] = r[..] else {
            panic!("{r:?}")
        };
        assert_eq!(long.left(), short.left(), "{host:?}: starts a line: {r:?}");
        assert_eq!(long.top() - short.bottom(), 8.0, "{host:?}: gap: {r:?}");
        assert_eq!(long.width(), 200.0, "{host:?}: the line's width: {r:?}");
        assert!(
            long.height() > 2.0 * short.height() - 0.5,
            "{host:?}: wraps: {r:?}"
        );
        assert_eq!(next.left(), short.left(), "{host:?}: next line: {r:?}");
        assert_eq!(next.top() - long.bottom(), 8.0, "{host:?}: {r:?}");
        assert_eq!(passes, 1, "{host:?}: a still frame");
    }
}

const LONG_LABEL: &str = "a crate label with more text than one whole line holds";

/// A crate Label wraps where the browser wraps it (conformance 3.4): in a
/// column narrower than its text at the column's width, and a short label
/// keeps its one line.
#[test]
fn a_crate_label_wraps_in_a_narrow_column() {
    use shadcn_egui::components::label::Label;
    for host in [Host::Screen, Host::Area(false)] {
        let (passes, r) = settled(&vega(), host, &|ui, _, out| {
            ui.set_max_width(120.0);
            ui.vertical(|ui| {
                out.push(ui.add(Label::new("short")).rect);
                out.push(ui.add(Label::new(LONG_LABEL)).rect);
            });
        });
        let [short, long] = r[..] else {
            panic!("{r:?}")
        };
        assert!(long.width() <= 120.0, "{host:?}: wraps at 120: {r:?}");
        assert!(
            long.height() > 2.0 * short.height() - 0.5,
            "{host:?}: wraps: {r:?}"
        );
        assert!(short.width() < 120.0, "{host:?}: content-sized: {r:?}");
        assert_eq!(passes, 1, "{host:?}: a still frame");
    }
}

/// In a crate `row` a crate Label is a flex item (`flex-shrink: 1`): it
/// keeps its one line while the row has room, and where it has not it
/// shrinks to the room the row's other items leave and its text wraps, as
/// a `vstack` of text there does. The item after it follows at the gap,
/// inside the row. On the screen and in an `egui::Area`, and a still frame
/// takes one pass.
#[test]
fn a_crate_label_in_a_row_shrinks_to_the_room_left() {
    use shadcn_egui::components::label::Label;
    for host in [Host::Screen, Host::Area(false)] {
        for room in [600.0, 200.0] {
            let (passes, r) = settled(&vega(), host, &|ui, _, out| {
                ui.set_max_width(room);
                out.push(ui.max_rect());
                layout::row(ui, 8.0, |ui| {
                    out.push(ui.button("Go").rect);
                    out.push(ui.add(Label::new(LONG_LABEL)).rect);
                    out.push(boxed(ui, 10.0, 10.0));
                });
            });
            let [area, go, label, after] = r[..] else {
                panic!("{r:?}")
            };
            assert_eq!(label.left(), go.right() + 8.0, "{host:?} {room}: {r:?}");
            assert_eq!(after.left(), label.right() + 8.0, "{host:?} {room}: {r:?}");
            assert!(after.right() <= area.right(), "{host:?} {room}: {r:?}");
            if room > 300.0 {
                assert!(label.height() < 20.0, "{host:?}: one line: {r:?}");
            } else {
                assert!(label.height() > 20.0, "{host:?}: wraps: {r:?}");
                assert_eq!(after.right(), area.right(), "{host:?}: {r:?}");
            }
            assert_eq!(passes, 1, "{host:?} {room}: a still frame");
        }
    }
}

/// In a plain egui horizontal layout a crate Label follows the `Ui`'s
/// text wrap mode, as egui's `Label` does: it extends past the room by
/// default and wraps at the room left where the `Ui` wraps text.
#[test]
fn a_crate_label_in_a_plain_horizontal_follows_its_wrap_mode() {
    use shadcn_egui::components::label::Label;
    for wrap in [false, true] {
        let (_, r) = settled(&vega(), Host::Screen, &|ui, _, out| {
            ui.set_max_width(200.0);
            ui.horizontal(|ui| {
                if wrap {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                }
                out.push(ui.button("Go").rect);
                out.push(ui.add(Label::new(LONG_LABEL)).rect);
            });
        });
        let [go, label] = r[..] else { panic!("{r:?}") };
        if wrap {
            assert!(label.right() <= 200.0, "wraps at the room: {r:?}");
            assert!(label.height() > 20.0, "wraps: {r:?}");
        } else {
            assert!(label.right() > 200.0, "extends: {r:?}");
            assert!(label.height() < 20.0, "one line: {r:?}");
        }
        assert_eq!(label.left(), go.right() + ui_gap(), "{r:?}");
    }
}

/// egui's default `item_spacing.x` under the vega theme.
fn ui_gap() -> f32 {
    vega().global_style().spacing.item_spacing.x
}

/// A `vstack` that fits its `row` is laid out at its measured width, so
/// its `w-full` children span the stack's width, as in CSS: a horizontal
/// crate separator spans it, and an Input in a stack wider than the
/// Input's maximum width keeps that maximum. On the screen and in an `egui::Area`, and a
/// still frame takes one pass.
#[test]
fn w_full_children_span_a_fitting_stack_in_a_row() {
    use shadcn_egui::components::input::Input;
    use shadcn_egui::components::separator::Separator;
    for host in [Host::Screen, Host::Area(false)] {
        let (passes, r) = settled(&vega(), host, &|ui, _, out| {
            ui.set_max_width(500.0);
            layout::row(ui, 8.0, |ui| {
                let stack = layout::vstack(ui, 4.0, |ui| {
                    out.push(
                        ui.label("A label that is wider than the field's own maximum width")
                            .rect,
                    );
                    out.push(ui.add(Separator::new()).rect);
                    let mut text = String::new();
                    out.push(ui.add(Input::new(&mut text)).rect);
                    ui.min_rect()
                });
                out.push(stack);
                out.push(ui.button("Go").rect);
            });
        });
        let [label, rule, input, stack, go] = r[..] else {
            panic!("{r:?}")
        };
        assert_eq!(stack.width(), label.width(), "{host:?}: {r:?}");
        assert_eq!(rule.x_range(), stack.x_range(), "{host:?}: {r:?}");
        // Text entry fills up to its maximum width (conformance 3.1).
        let field = Theme::vega().metrics.field_max_width;
        assert!(stack.width() > field, "{host:?}: {r:?}");
        assert_eq!(input.left(), stack.left(), "{host:?}: {r:?}");
        assert_eq!(input.width(), field, "{host:?}: {r:?}");
        assert_eq!(go.left(), stack.right() + 8.0, "{host:?}: {r:?}");
        assert_eq!(passes, 1, "{host:?}: a still frame");
    }
}

/// When the text of a fitting `vstack` in a `row` changes, the frame of
/// the change redoes its pass once and never shows the text wrapped at the
/// old width; the next frame lays the stack out at its new width, its
/// separator spanning it, in one pass. On the screen and in an
/// `egui::Area`.
#[test]
fn a_fitting_stack_whose_text_changes_shows_no_wrapped_frame() {
    use shadcn_egui::components::separator::Separator;
    let content = |text: &'static str| {
        move |ui: &mut egui::Ui, _: bool, out: &mut Vec<egui::Rect>| {
            ui.set_max_width(400.0);
            layout::row(ui, 8.0, |ui| {
                let stack = layout::vstack(ui, 4.0, |ui| {
                    out.push(ui.label(text).rect);
                    out.push(ui.add(Separator::new()).rect);
                    ui.min_rect()
                });
                out.push(stack);
            });
        }
    };
    for host in [Host::Screen, Host::Area(false)] {
        let ctx = vega();
        let (_, before) = settled(&ctx, host, &content("Account"));
        let (passes, change) = frame_in(&ctx, host, false, &content("Account settings"));
        assert_eq!(passes, 2, "{host:?}: one discard: {change:?}");
        assert_eq!(
            change[0].height(),
            before[0].height(),
            "{host:?}: not wrapped: {change:?}"
        );
        let (passes, after) = frame_in(&ctx, host, false, &content("Account settings"));
        assert_eq!(passes, 1, "{host:?}: the frame after");
        assert_eq!(after[0].height(), before[0].height(), "{host:?}");
        assert_eq!(
            after[1].x_range(),
            after[2].x_range(),
            "{host:?}: {after:?}"
        );
        assert!(after[2].width() > before[2].width(), "{host:?}: {after:?}");
    }
}

/// A framed child: the crate's `Card` (with a footer when `footer`) or an
/// egui `Frame` with an inner margin of 16. Its rect: the last widget it
/// registered, the box it allocated around its content.
fn framed(ui: &mut egui::Ui, card: bool, footer: bool) -> egui::Rect {
    use shadcn_egui::components::card::Card;
    if card {
        let card = Card::new();
        let card = if footer {
            card.footer(|ui| {
                ui.label("Footer");
            })
        } else {
            card
        };
        card.show(ui, |ui| {
            ui.label("Card");
        });
    } else {
        egui::Frame::new().inner_margin(16.0).show(ui, |ui| {
            ui.label("Frame");
        });
    }
    let layer = ui.layer_id();
    ui.ctx()
        .viewport(|v| v.this_pass.widgets.get_layer(layer).last().map(|w| w.rect))
        .expect("the frame registered its box")
}

/// A framed child in a crate row: an egui `Frame` with an inner margin,
/// a crate `Card` with and without a footer. In a `row` and a `wrap_row`
/// its line is as high as its box with the margin, a `Button` beside it
/// is centred on it, and a stretch item beside it spans it. The card,
/// which sets no width upstream, is as wide as its content, so the
/// button follows it on its line. On the screen and in an `egui::Area`,
/// and a still frame takes one pass.
#[test]
fn a_framed_child_is_one_item_of_a_crate_row() {
    use shadcn_egui::components::button::Button;
    let cases = [(false, false), (true, false), (true, true)];
    for wraps in [false, true] {
        for (card, footer) in cases {
            for host in [Host::Screen, Host::Area(false)] {
                let (passes, r) = settled(&vega(), host, &|ui, _, out| {
                    ui.set_max_width(400.0);
                    let top = ui.cursor().top();
                    let items = |ui: &mut egui::Ui| {
                        out.push(framed(ui, card, footer));
                        out.push(ui.add(Button::new("Go")).rect);
                        out.push(stretch_rule(ui));
                    };
                    if wraps {
                        layout::wrap_row(ui, 8.0, items);
                    } else {
                        layout::row(ui, 8.0, items);
                    }
                    let gap = ui.spacing().item_spacing.y;
                    out.push(egui::Rect::from_x_y_ranges(
                        0.0..=0.0,
                        top..=ui.cursor().top() - gap,
                    ));
                });
                let case = format!("wraps {wraps} card {card} footer {footer} {host:?}: {r:?}");
                let [frame, go, rule, band] = r[..] else {
                    panic!("{case}")
                };
                assert_eq!(rule.left(), go.right() + 8.0, "{case}");
                assert_eq!(passes, 1, "{case}: a still frame");
                assert!(frame.width() < 200.0, "content-sized: {case}");
                assert_eq!(go.left(), frame.right() + 8.0, "{case}");
                assert_eq!(band.height(), frame.height(), "line height: {case}");
                assert_eq!(rule.y_range(), frame.y_range(), "stretch: {case}");
                assert!(
                    (go.center().y - frame.center().y).abs() < 0.5,
                    "centred: {case}"
                );
            }
        }
    }
}

/// Two `vstack`s that shrink in a `row`, each with wrapped text of a
/// fixed line height and a `w-full` separator, keep their size when the
/// theme's body font changes under the same family: the row still ends as
/// one that had the new font from the start.
#[test]
fn a_shrunk_stack_relearns_its_width_when_its_font_changes() {
    use shadcn_egui::components::separator::Separator;
    fn frame(ctx: &egui::Context) -> [egui::Rect; 2] {
        let mut out = [egui::Rect::NOTHING; 2];
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.scope(|ui| {
                ui.set_max_width(280.0);
                layout::row(ui, 8.0, |ui| {
                    for (i, text) in [
                        "Receive emails about new products and offers.",
                        "Get notified when someone mentions you in a comment.",
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        out[i] = layout::vstack(ui, 4.0, |ui| {
                            ui.label(egui::RichText::new(text).line_height(Some(20.0)));
                            ui.add(Separator::new());
                            ui.min_rect()
                        });
                    }
                });
            });
        });
        out
    }
    let oxanium = || Theme::vega().fonts(&shadcn_egui_font_oxanium::FONT, None);
    let ctx = vega();
    let mut before = [egui::Rect::NOTHING; 2];
    for _ in 0..4 {
        before = frame(&ctx);
    }
    oxanium().apply(&ctx);
    let switched = frame(&ctx);
    assert_eq!(
        before.map(|r| r.size()),
        switched.map(|r| r.size()),
        "the case needs an equal size"
    );
    let mut after = switched;
    for _ in 0..4 {
        after = frame(&ctx);
    }
    let fresh_ctx = egui::Context::default();
    oxanium().apply(&fresh_ctx);
    let mut fresh = [egui::Rect::NOTHING; 2];
    for _ in 0..5 {
        fresh = frame(&fresh_ctx);
    }
    assert_ne!(before, fresh, "the case needs another split");
    assert_eq!(after, fresh, "the row relearns the font's widths");
}

/// The rects of a card in `ui` with `add` in its body: the card's box
/// (the last widget it registered) and what `add` returns.
fn card_with(
    ui: &mut egui::Ui,
    add: impl FnOnce(&mut egui::Ui) -> egui::Rect,
) -> (egui::Rect, egui::Rect) {
    use shadcn_egui::components::card::Card;
    let inner = Card::new().show(ui, add);
    let layer = ui.layer_id();
    let card = ui
        .ctx()
        .viewport(|v| v.this_pass.widgets.get_layer(layer).last().map(|w| w.rect))
        .expect("the card registered its box");
    (card, inner)
}

/// A card sets no width upstream, so in a crate row it is a flex item:
/// as wide as its content (its padding, 24 px a side in vega) where the
/// row has room, shrinking with its text wrapped to the room the row's
/// other items leave where it has not. Every style's card is
/// `overflow-hidden`, a scroll container, whose automatic minimum width
/// in a flex row is 0: too narrow for its widest unbreakable content it
/// still takes only the room left, pushes nothing, and clips its content
/// at its box. In a `wrap_row` a card that does not fit beside the button
/// takes a line of its own at the line's width. In a `row` and a
/// `wrap_row`, on the screen and in an `egui::Area`; a still frame takes
/// one pass.
#[test]
fn a_card_is_a_flex_item_of_a_crate_row() {
    use shadcn_egui::components::button::Button;
    const LONG: &str = "Receive emails about new products, features and more.";
    for wraps in [false, true] {
        for host in [Host::Screen, Host::Area(false)] {
            for (room, text, wide) in [
                (400.0, "Short", 0.0),
                (300.0, LONG, 0.0),
                (150.0, "", 200.0),
            ] {
                let (passes, r) = settled(&vega(), host, &|ui, _, out| {
                    ui.set_max_width(room);
                    out.push(ui.max_rect());
                    let items = |ui: &mut egui::Ui| {
                        let mut clip = egui::Rect::NOTHING;
                        let (card, inner) = card_with(ui, |ui| {
                            clip = ui.clip_rect();
                            if wide > 0.0 {
                                boxed(ui, wide, 20.0)
                            } else {
                                ui.label(text).rect
                            }
                        });
                        out.extend([card, inner, clip]);
                        out.push(ui.add(Button::new("Go")).rect);
                    };
                    if wraps {
                        layout::wrap_row(ui, 8.0, items);
                    } else {
                        layout::row(ui, 8.0, items);
                    }
                });
                let case = format!("wraps {wraps} {host:?} room {room}: {r:?}");
                let [space, card, inner, clip, go] = r[..] else {
                    panic!("{case}")
                };
                assert_eq!(passes, 1, "{case}: a still frame");
                assert_eq!(card.left(), space.left(), "{case}");
                if room < 400.0 {
                    // Laid out in the room it got, the card clips there;
                    // laid out unbounded (fitting a `wrap_row` line), its
                    // content cannot overflow it.
                    assert!(clip.left() >= card.left(), "clipped: {case}");
                    assert!(clip.right() <= card.right(), "clipped: {case}");
                }
                if wraps && room < 400.0 {
                    // The card does not fit beside the button: it takes
                    // its own line at the line's width, and the button
                    // starts the next.
                    assert_eq!(card.width(), room, "{case}");
                    assert_eq!(go.left(), card.left(), "{case}");
                    assert_eq!(go.top(), card.bottom() + 8.0, "{case}");
                    continue;
                }
                assert_eq!(go.left(), card.right() + 8.0, "{case}");
                assert_eq!(inner.left(), card.left() + 24.0, "{case}");
                if room == 400.0 {
                    assert_eq!(card.width(), inner.width() + 48.0, "content: {case}");
                } else if room == 300.0 {
                    assert_eq!(go.right(), space.right(), "shrinks: {case}");
                    assert!(inner.height() > 30.0, "wraps: {case}");
                    assert!(inner.right() <= card.right() - 24.0, "{case}");
                } else {
                    assert_eq!(go.right(), space.right(), "shrinks: {case}");
                    assert!(inner.right() > card.right(), "overflows: {case}");
                }
            }
        }
    }
}

/// In a column a card spans the available width, as a block does, also
/// inside a `vstack` in a row, where it spans the stack; where the width
/// is unbounded it takes its content's width. A still frame takes one
/// pass.
///
/// The stack is as wide as its label unwrapped (243.0625 in vega). The
/// label's own rect is up to half a point narrower: at a finite width
/// epaint wraps text at that width rounded to whole points, plus 0.5 so
/// it never wraps earlier, and reports the galley no wider than the
/// rounded width (`Fonts::layout_job`, `Galley::round_output_to_gui`).
#[test]
fn a_card_fills_a_column_and_takes_its_content_width_when_unbounded() {
    let (passes, r) = settled(&vega(), Host::Screen, &|ui, _, out| {
        ui.set_max_width(400.0);
        out.push(ui.max_rect());
        out.push(card_with(ui, |ui| ui.label("Column").rect).0);
        layout::row(ui, 8.0, |ui| {
            let stack = layout::vstack(ui, 4.0, |ui| {
                out.push(ui.label("A label wider than the card's content").rect);
                out.push(card_with(ui, |ui| ui.label("In a stack").rect).0);
                ui.min_rect()
            });
            out.push(stack);
        });
        ui.allocate_ui(egui::vec2(f32::INFINITY, 200.0), |ui| {
            out.extend(<[_; 2]>::from(card_with(ui, |ui| {
                ui.label("Unbounded").rect
            })));
        });
    });
    let [space, column, label, in_stack, stack, unbounded, text] = r[..] else {
        panic!("{r:?}")
    };
    assert_eq!(column.x_range(), space.x_range(), "{r:?}");
    assert_eq!(in_stack.x_range(), stack.x_range(), "{r:?}");
    assert_eq!(stack.width(), 243.0625, "{r:?}");
    assert_eq!(label.width(), stack.width().round(), "{r:?}");
    assert_eq!(unbounded.width(), text.width() + 48.0, "{r:?}");
    assert_eq!(passes, 1, "{r:?}");
}

/// A stack that fits its row is laid out at its unwrapped width, where
/// epaint reports a label up to half a point narrower than it measured
/// unwrapped (see `a_card_fills_a_column_and_takes_its_content_width_when_unbounded`);
/// that is no content change: a still frame takes one pass, on the
/// screen and in an `egui::Area`, and on the screen asks for no repaint.
#[test]
fn a_stack_of_a_label_with_a_fractional_width_settles() {
    for host in [Host::Screen, Host::Area(false)] {
        let ctx = vega();
        let (passes, r) = settled(&ctx, host, &|ui, _, out| {
            let row = layout::row(ui, 8.0, |ui| {
                layout::vstack(ui, 4.0, |ui| {
                    out.push(ui.label("A label wider than the card's content").rect);
                });
                ui.min_rect()
            });
            out.push(row);
        });
        let [label, row] = r[..] else { panic!("{r:?}") };
        assert_eq!(passes, 1, "{host:?} {r:?}");
        assert_eq!(row.width(), 243.0625, "{host:?} {r:?}");
        assert_eq!(label.width(), 243.0, "{host:?} {r:?}");
        // An Area's fade-in asks for repaints while the test's clock
        // stands still.
        if matches!(host, Host::Screen) {
            assert!(!ctx.has_requested_repaint());
        }
    }
}

/// A `space_between` in unbounded room (a card footer where the card is
/// laid out without a width limit) has no free space to put between its
/// sides, as a `justify-between` row in max-content sizing: the right
/// side follows the left one a gap apart (none in vega's footer), and the
/// card around it takes that width plus its footer padding (24 px a side
/// in vega). On its own in unbounded room the row is as wide as its two
/// sides and the gap. In a crate row with room the card is that wide too,
/// as the row measures it unbounded, and the row's next item follows it.
#[test]
fn a_space_between_in_unbounded_room_takes_its_content_width() {
    use shadcn_egui::components::card::Card;
    for in_row in [false, true] {
        let (passes, r) = settled(&vega(), Host::Screen, &|ui, _, out| {
            let body = |ui: &mut egui::Ui, out: &mut Vec<egui::Rect>| {
                let (mut l, mut b, mut gap) = (egui::Rect::NOTHING, egui::Rect::NOTHING, 0.0);
                Card::new()
                    .footer(|ui| {
                        gap = ui.spacing().item_spacing.x;
                        layout::space_between(
                            ui,
                            |ui| l = ui.label("Left side").rect,
                            |ui| b = ui.button("Go").rect,
                        )
                    })
                    .show(ui, |ui| ui.label("Body"));
                let layer = ui.layer_id();
                let card = ui
                    .ctx()
                    .viewport(|v| v.this_pass.widgets.get_layer(layer).last().map(|w| w.rect))
                    .expect("the card registered its box");
                let mut gap2 = 0.0;
                let row = ui
                    .horizontal(|ui| {
                        gap2 = ui.spacing().item_spacing.x;
                        layout::space_between(
                            ui,
                            |ui| {
                                ui.label("Left side");
                            },
                            |ui| {
                                let _ = ui.button("Go");
                            },
                        );
                    })
                    .response
                    .rect;
                out.extend([
                    card,
                    l,
                    b,
                    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(gap, gap2)),
                    row,
                ]);
            };
            if in_row {
                ui.set_max_width(400.0);
                layout::row(ui, 8.0, |ui| body(ui, out));
            } else {
                ui.allocate_ui(egui::vec2(f32::INFINITY, 400.0), |ui| body(ui, out));
            }
        });
        let [card, left, go, gap, row] = r[..] else {
            panic!("{r:?}")
        };
        let case = format!("in a row {in_row}: {r:?}");
        assert_eq!(go.left(), left.right() + gap.width(), "{case}");
        assert_eq!(card.right(), go.right() + 24.0, "{case}");
        if in_row {
            assert_eq!(row.left(), card.right() + 8.0, "{case}");
        } else {
            assert_eq!(
                row.width(),
                left.width() + gap.height() + go.width(),
                "{case}"
            );
        }
        assert_eq!(passes, 1, "{case}");
    }
}

/// A raw egui widget that fills to infinity (`add_sized` with an
/// infinite width) in a card or a vstack of a crate row or wrap_row in
/// unbounded room
/// makes that item infinitely wide, as it makes an egui `Frame` around it,
/// and the row settles: one pass a frame, no repaint asked for.
#[test]
fn an_infinitely_filling_item_of_a_row_in_unbounded_room_settles() {
    use shadcn_egui::components::card::Card;
    for (card, wrap) in [(false, false), (true, false), (false, true), (true, true)] {
        let ctx = vega();
        let (passes, r) = settled(&ctx, Host::Screen, &|ui, _, out| {
            ui.allocate_ui(egui::vec2(f32::INFINITY, 400.0), |ui| {
                let row = |ui: &mut egui::Ui, add: &mut dyn FnMut(&mut egui::Ui)| {
                    if wrap {
                        layout::wrap_row(ui, 8.0, |ui| add(ui));
                    } else {
                        layout::row(ui, 8.0, |ui| add(ui));
                    }
                };
                row(ui, &mut |ui| {
                    let fill = |ui: &mut egui::Ui| {
                        ui.add_sized([f32::INFINITY, 20.0], egui::Separator::default())
                    };
                    let item = if card {
                        Card::new().show(ui, |ui| fill(ui));
                        let layer = ui.layer_id();
                        ui.ctx()
                            .viewport(|v| {
                                v.this_pass.widgets.get_layer(layer).last().map(|w| w.rect)
                            })
                            .expect("the card registered its box")
                    } else {
                        layout::vstack(ui, 4.0, |ui| fill(ui));
                        ui.min_rect()
                    };
                    out.push(item);
                })
            });
        });
        let case = format!("card {card}, wrap_row {wrap}: {r:?}");
        assert_eq!(r[0].width(), f32::INFINITY, "{case}");
        assert_eq!(passes, 1, "{case}");
        assert!(!ctx.has_requested_repaint(), "{case}");
    }
}
