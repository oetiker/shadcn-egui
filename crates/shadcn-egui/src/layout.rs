//! Flexbox substitutes for egui's single-pass layout.
//! `gap` maps to item_spacing; space-between right-aligns via a nested layout.

use crate::style::TextSize;
use crate::Theme;
use egui::Ui;
use egui_extras::{Size, StripBuilder};

/// A row height or stack width change below this is float noise, not a
/// reason to redo a pass.
const ROW_HEIGHT_EPSILON: f32 = 0.01; // behaviour-ok: float tolerance, not a design token

/// Horizontal stack with an explicit gap (flex-direction: row; gap: N;
/// align-items: center).
///
/// egui centres each child in the row height so far, so a child taller than
/// `interact_size.y` would leave the children before it above centre. The
/// row therefore starts from the content height of the last pass, and asks
/// egui to redo a pass whose content height changed.
///
/// A widget that fills the available height (`separator_vertical`, a
/// `ScrollArea`) takes the row's current height as its own, so a row that
/// holds one does not shrink after a taller child goes; a `ScrollArea`
/// grows the row to its full content height. Content that registers no
/// widget (shapes painted directly, `allocate_space`) is not measured.
///
/// A child wider than the room left pushes the children after it past its
/// whole width, and the row takes no more than its room in the parent, so
/// the overflow does not widen the parent: the browser's flex row. A plain
/// `ui.horizontal` cannot do that (see [`allocate_overflowing`]).
pub fn row<R>(ui: &mut Ui, gap: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let id = ui.next_auto_id().with("shadcn_egui::layout::row");
    let floor = ui.spacing().interact_size.y;
    let stored = ui.data(|d| d.get_temp::<f32>(id));
    let height = stored.unwrap_or(floor);
    let layout = if ui.layout().prefer_right_to_left() {
        egui::Layout::right_to_left(egui::Align::Center)
    } else {
        egui::Layout::left_to_right(egui::Align::Center)
    };
    // The content height: the tallest child of the row, where a child that
    // is a Ui (a scope, `add_enabled_ui`, a nested `vstack`) counts as the
    // union of the leaf widgets inside it, not as its own rect. Measured from the widgets
    // registered while `add` runs, so it does not depend on the row's own
    // height. A Ui's own rect cannot serve: in a centred row egui makes it
    // as high as the row, so a row holding one would never shrink. Neither
    // can the union of all widgets: a nested `vstack` sits at the top while
    // a widget beside it is centred, and the union grew on every pass.
    let layer = ui.layer_id();
    let before = ui
        .ctx()
        .viewport(|v| v.this_pass.widgets.get_layer(layer).count());
    let size = egui::vec2(ui.available_size_before_wrap().x, height);
    // The entries the row's two Uis register, and the `id` the row's
    // children name as parent.
    let (mut own, mut row_ui) = ([id; 2], id);
    let inner = overflow_row(ui, size, layout, |room, ui| {
        (own, row_ui) = ([room.unique_id(), ui.unique_id()], ui.id());
        ui.spacing_mut().item_spacing.x = gap;
        add(ui)
    });
    let content = ui.ctx().viewport(|v| {
        let added: Vec<egui::WidgetRect> = v
            .this_pass
            .widgets
            .get_layer(layer)
            .skip(before)
            .filter(|w| !own.contains(&w.id))
            .copied()
            .collect();
        // Rebuild the tree from the creation order: a Ui registers before
        // what it holds, and an entry names its Ui's `id` as parent, not
        // the `unique_id` the Ui registered under. So an entry whose next
        // entry names a parent not on the stack opens a Ui.
        let mut stack = vec![row_ui];
        let mut children: Vec<egui::Rect> = Vec::new();
        for (i, w) in added.iter().enumerate() {
            while stack.len() > 1 && stack.last() != Some(&w.parent_id) {
                stack.pop();
            }
            if stack.len() == 1 {
                children.push(egui::Rect::NOTHING);
            }
            match added.get(i + 1).map(|n| n.parent_id) {
                Some(p) if !stack.contains(&p) => stack.push(p),
                _ => {
                    if let Some(c) = children.last_mut() {
                        *c = c.union(w.rect);
                    }
                }
            }
        }
        children.iter().map(|r| r.height()).fold(0.0, f32::max)
    });
    let measured = content.max(floor);
    if stored.is_none_or(|h| (h - measured).abs() > ROW_HEIGHT_EPSILON) {
        ui.data_mut(|d| d.insert_temp(id, measured));
        if (height - measured).abs() > ROW_HEIGHT_EPSILON {
            ui.ctx()
                .request_discard("layout::row content height changed");
        }
    }
    inner
}

/// Horizontal stack that wraps onto further lines when it runs out of width
/// (flex-direction: row; flex-wrap: wrap; gap: N). The gap applies both between
/// items on a line and between wrapped lines. A [`vstack`] item wraps as a
/// whole, as a flex item does.
pub fn wrap_row<R>(ui: &mut Ui, gap: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    // As `horizontal_wrapped`, but each line is as high as its tallest item:
    // egui's own starts every line `interact_size.y` high, which puts more
    // than `gap` between lines of smaller items (badges).
    let layout = if ui.layout().prefer_right_to_left() {
        egui::Layout::right_to_left(egui::Align::Center)
    } else {
        egui::Layout::left_to_right(egui::Align::Center)
    };
    let size = egui::vec2(ui.available_size_before_wrap().x, 0.0);
    overflow_row(ui, size, layout.with_main_wrap(true), |_, ui| {
        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
        add(ui)
    })
}

/// The tag on the `Ui` of a crate row ([`row`], [`wrap_row`],
/// [`space_between`], a card footer), where [`allocate_overflowing`]
/// allocates a widget's whole width.
const ROW_TAG: &str = "shadcn_egui::layout::row";

/// The `UiBuilder` of a crate row's `Ui`: `max_rect` and `layout`, tagged
/// as a row. The caller allocates only the part of the row's `min_rect`
/// that fits its room, so an overflowing child pushes its siblings without
/// widening the parent.
pub(crate) fn row_builder(max_rect: egui::Rect, layout: egui::Layout) -> egui::UiBuilder {
    egui::UiBuilder::new()
        .max_rect(max_rect)
        .layout(layout)
        .ui_stack_info(egui::UiStackInfo::default().with_tag(ROW_TAG))
}

/// Whether `ui` is a crate row, or a horizontal `Ui` (a scope, an enabled
/// scope) inside one.
fn in_row(ui: &Ui) -> bool {
    ui.stack()
        .iter()
        .take_while(|s| s.layout_direction.is_horizontal())
        .any(|s| s.tags().contains(ROW_TAG))
}

/// A crate row of `size` in `layout`, placed as `allocate_ui_with_layout`
/// places it. `add` gets the `Ui` that holds the room and the row's own
/// `Ui`, built inside it with [`row_builder`]. The row's children may run
/// past the room; the room takes the row's `min_rect` cut to its own
/// width, and the parent allocates that.
fn overflow_row<R>(
    ui: &mut Ui,
    size: egui::Vec2,
    layout: egui::Layout,
    add: impl FnOnce(&Ui, &mut Ui) -> R,
) -> R {
    ui.allocate_ui_with_layout(size, layout, |room| {
        let max = room.max_rect();
        let mut row = room.new_child(row_builder(max, layout));
        let inner = add(room, &mut row);
        // Dropping the row registers its `min_rect`.
        let used = row.min_rect();
        drop(row);
        let x = |v: f32| v.clamp(max.left(), max.right());
        room.advance_cursor_after_rect(egui::Rect::from_x_y_ranges(
            x(used.left())..=x(used.right()),
            used.y_range(),
        ));
        inner
    })
    .inner
}

/// Vertical stack with an explicit gap (flex-direction: column; gap: N).
///
/// As an item of a [`wrap_row`] it behaves as a flex item: its text takes
/// its widest, unwrapped width, and the stack starts a new line when that
/// width does not fit the rest of the line. egui wraps only an item whose
/// size it knows before the item draws, so the stack takes its width from
/// the previous pass and redoes a pass when it changes. Alone on a line too
/// narrow for it, its text wraps (flex-shrink).
pub fn vstack<R>(ui: &mut Ui, gap: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    if !ui.layout().main_wrap() {
        return ui
            .vertical(|ui| {
                ui.spacing_mut().item_spacing.y = gap;
                add(ui)
            })
            .inner;
    }
    let id = ui.next_auto_id().with("shadcn_egui::layout::vstack");
    let stored = ui.data(|d| d.get_temp::<f32>(id));
    let line_start = ui.cursor().min.x <= ui.max_rect().min.x;
    if !line_start && stored.is_some_and(|w| w > ui.available_size_before_wrap().x) {
        ui.end_row();
    }
    let fits = stored.is_none_or(|w| w <= ui.available_size_before_wrap().x);
    let response = ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = gap;
        if fits {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        }
        add(ui)
    });
    // Only an unwrapped pass measures the stack's own width.
    let width = response.response.rect.width();
    if fits && stored.is_none_or(|w| (w - width).abs() > ROW_HEIGHT_EPSILON) {
        ui.data_mut(|d| d.insert_temp(id, width));
        ui.ctx()
            .request_discard("layout::vstack width changed in a wrap_row");
    }
    response.inner
}

/// The width a widget takes (conformance 3.1; the one place a widget's
/// width arithmetic lives, rule 6 of the porting standard): `requested`
/// (the caller's width setter, else the widget's default), capped to the
/// width `ui` has left, then raised to `content_min`, what the widget
/// cannot shrink below (its padding and fixed parts). `f32::INFINITY`
/// requests the full width; where that is unbounded too (a horizontal
/// `ScrollArea`), the widget takes `content_min`. A NaN request is no
/// request: the widget takes `content_min`, never NaN.
#[must_use]
pub fn capped_width(ui: &Ui, requested: f32, content_min: f32) -> f32 {
    fit_width(requested, ui.available_width(), content_min)
}

/// [`capped_width`] for a floating overlay (dialog, popover): `requested`
/// capped to the window's width less `inset` (upstream's
/// `max-w-[calc(100%-Npx)]`, 0 without one), then raised to
/// `content_min`.
#[must_use]
pub fn overlay_width(ctx: &egui::Context, requested: f32, inset: f32, content_min: f32) -> f32 {
    fit_width(requested, ctx.content_rect().width() - inset, content_min)
}

/// Allocates `size` as the browser places a box wider than its room: the
/// returned rect (and the response's) is `size`, but the parent `Ui` takes
/// at most the width it has left, so a widget raised to its content
/// minimum ([`capped_width`]) overflows without widening its parent and
/// the widgets that fill it. It overflows as a flex item does: past the
/// end in a left-aligned column or a left-to-right row, past the start in
/// a right-aligned column or a right-to-left row, and evenly on both
/// sides in a centred column.
///
/// In a crate row ([`row`], [`wrap_row`], [`space_between`], a card
/// footer), or a horizontal scope inside one, the widget takes its whole
/// width and the next widget starts after it, as in the browser; the row
/// keeps the overflow from its own parent. In a plain `ui.horizontal` the
/// next widget starts after the room the parent gave, so it covers the
/// overflowing part: egui can move a row's cursor only by growing the row,
/// and that would widen the row's parent.
pub fn allocate_overflowing(
    ui: &mut Ui,
    size: egui::Vec2,
    sense: egui::Sense,
) -> (egui::Rect, egui::Response) {
    let room = ui.available_width().max(0.0);
    if size.x <= room || in_row(ui) {
        return ui.allocate_exact_size(size, sense);
    }
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(room, size.y), sense);
    let layout = ui.layout();
    let stretched = layout.is_vertical() && layout.cross_justify();
    let left = match layout.horizontal_placement() {
        _ if stretched => rect.left(),
        egui::Align::Min => rect.left(),
        egui::Align::Center => rect.center().x - size.x / 2.0,
        egui::Align::Max => rect.right() - size.x,
    };
    // The response reports the widget's own rect; it senses only the part
    // inside the parent.
    response.rect = egui::Rect::from_min_size(egui::pos2(left, rect.top()), size);
    (response.rect, response)
}

/// The clamp order of [`capped_width`]: cap first, then the content
/// minimum, so a widget wider than its room overflows instead of
/// truncating.
fn fit_width(requested: f32, available: f32, content_min: f32) -> f32 {
    // `f32::max` and `f32::min` return the other operand for NaN.
    let content_min = content_min.max(0.0);
    if requested.is_nan() {
        return content_min;
    }
    let w = requested.min(available);
    if w.is_finite() {
        w.max(content_min)
    } else {
        content_min
    }
}

/// Tailwind's `sm` breakpoint (40rem).
const BREAKPOINT_SM: f32 = 640.0; // behaviour-ok: Tailwind default --breakpoint-sm
/// Tailwind's `md` breakpoint (48rem).
const BREAKPOINT_MD: f32 = 768.0; // behaviour-ok: Tailwind default --breakpoint-md

/// The breakpoint helper for media queries (`sm:`, `md:`,
/// `@media (width >= N)`): whether the window is at least `px` points
/// wide. It reads what a media query reads, the whole viewport
/// (`Context::viewport_rect`), safe-area insets included. Upstream's
/// breakpoints are `min-width` queries, so the boundary counts as wide.
#[must_use]
pub fn window_at_least(ctx: &egui::Context, px: f32) -> bool {
    ctx.viewport_rect().width() >= px
}

/// The breakpoint helper for container queries (`@md/field-group`,
/// `@container (width >= N)`): whether the width `ui` has left is at
/// least `px` points; the boundary counts as wide. With
/// [`window_at_least`], the one written-down exception to single-pass
/// sizing: a widget reads its room here, never with arithmetic of its own.
#[must_use]
pub fn container_at_least(ui: &Ui, px: f32) -> bool {
    ui.available_width() >= px
}

/// The media-query flags of the window: [`State::SM`] at 640 px and up,
/// [`State::MD`] at 768 px and up. [`Theme::state`] adds them to every
/// state it resolves.
///
/// [`State::SM`]: crate::style::State::SM
/// [`State::MD`]: crate::style::State::MD
#[must_use]
pub(crate) fn breakpoints(ctx: &egui::Context) -> crate::style::State {
    use crate::style::State;
    State::NONE
        .with(State::SM, window_at_least(ctx, BREAKPOINT_SM))
        .with(State::MD, window_at_least(ctx, BREAKPOINT_MD))
}

/// A row whose `left` content hugs the start and `right` content hugs the end
/// (justify-content: space-between).
pub fn space_between(ui: &mut Ui, left: impl FnOnce(&mut Ui), right: impl FnOnce(&mut Ui)) {
    // `ui.horizontal`'s size and layout, as a crate row.
    let size = egui::vec2(
        ui.available_size_before_wrap().x,
        ui.spacing().interact_size.y,
    );
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    overflow_row(ui, size, layout, |_, ui| {
        left(ui);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            right(ui);
        });
    });
}

/// A shadcn Card at the default size, across the available width. See
/// [`crate::components::card::Card`] for sizes and a footer.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    crate::components::card::Card::new().show(ui, add)
}

/// A labeled control row: fixed-width label column + control filling the rest.
pub fn form_row(ui: &mut Ui, label_width: f32, label: &str, control: impl FnOnce(&mut Ui)) {
    let t = Theme::current(ui.ctx());
    let row_h = ui.spacing().interact_size.y.max(t.line_px(TextSize::Sm));
    ui.allocate_ui(egui::vec2(ui.available_width(), row_h), |ui| {
        StripBuilder::new(ui)
            .size(Size::exact(label_width))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(crate::components::label::Label::new(label));
                    });
                });
                strip.cell(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        control(ui)
                    });
                });
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_caps_then_raises_to_the_content_minimum() {
        assert_eq!(fit_width(300.0, 200.0, 50.0), 200.0);
        assert_eq!(fit_width(100.0, 200.0, 50.0), 100.0);
        // Cap first, then the minimum: an overflow, not a truncation.
        assert_eq!(fit_width(300.0, 20.0, 50.0), 50.0);
        // Fill.
        assert_eq!(fit_width(f32::INFINITY, 200.0, 50.0), 200.0);
        // Nothing finite requested and unbounded room: the minimum.
        assert_eq!(fit_width(f32::INFINITY, f32::INFINITY, 50.0), 50.0);
        assert_eq!(fit_width(120.0, f32::INFINITY, 50.0), 120.0);
    }

    #[test]
    fn width_is_never_nan() {
        // A NaN request does not fill (`NaN.min(x)` is `x`).
        assert_eq!(fit_width(f32::NAN, 200.0, 50.0), 50.0);
        assert_eq!(fit_width(300.0, f32::NAN, 50.0), 300.0);
        assert_eq!(fit_width(f32::INFINITY, 200.0, f32::NAN), 200.0);
        assert_eq!(fit_width(f32::NAN, f32::NAN, f32::NAN), 0.0);
    }

    #[test]
    fn an_overflowing_widget_keeps_its_parents_width() {
        let ctx = egui::Context::default();
        let mut widths = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.allocate_ui(egui::vec2(100.0, 200.0), |ui| {
                ui.set_max_width(100.0);
                let (rect, response) =
                    allocate_overflowing(ui, egui::vec2(150.0, 20.0), egui::Sense::hover());
                widths = Some((rect.width(), response.rect.width(), ui.min_rect().width()));
            });
        });
        assert_eq!(widths, Some((150.0, 150.0, 100.0)));
    }

    /// `rows` in a column 100 wide: the left edge of the widget after a
    /// 150 wide overflowing one, and the column's width afterwards.
    fn after_overflow(rows: impl Fn(&mut Ui, &mut dyn FnMut(&mut Ui))) -> (f32, f32) {
        let ctx = egui::Context::default();
        let mut out = None;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let max = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 200.0));
            let mut column = ui.new_child(egui::UiBuilder::new().max_rect(max));
            let mut next = 0.0;
            rows(&mut column, &mut |ui| {
                allocate_overflowing(ui, egui::vec2(150.0, 20.0), egui::Sense::hover());
                next = ui
                    .allocate_exact_size(egui::vec2(10.0, 20.0), egui::Sense::hover())
                    .0
                    .left();
            });
            out = Some((next, column.min_rect().width()));
        });
        out.expect("ran")
    }

    /// In a crate row an overflowing child pushes its siblings past its
    /// whole width, and the column holding the row keeps its width, as a
    /// flex row in the browser.
    #[test]
    fn a_crate_row_places_siblings_after_the_overflow() {
        assert_eq!(
            after_overflow(|ui, add| row(ui, 4.0, |ui| add(ui))),
            (154.0, 100.0)
        );
        assert_eq!(
            after_overflow(|ui, add| wrap_row(ui, 4.0, |ui| add(ui))),
            // The sibling does not fit the rest of the line: the next one.
            (0.0, 100.0)
        );
        assert_eq!(
            after_overflow(|ui, add| space_between(ui, |ui| add(ui), |_| {})),
            (150.0 + 8.0, 100.0)
        );
        // In a scope inside the row too.
        assert_eq!(
            after_overflow(|ui, add| row(ui, 4.0, |ui| {
                ui.scope(|ui| add(ui));
            })),
            (154.0, 100.0)
        );
    }

    /// In a plain `ui.horizontal` the next widget covers the overflow,
    /// and a widget that runs past the room still widens the parent.
    #[test]
    fn a_plain_horizontal_row_overlaps_the_overflow() {
        assert_eq!(
            after_overflow(|ui, add| {
                ui.horizontal(|ui| add(ui));
            }),
            (108.0, 118.0)
        );
    }

    /// An egui widget wider than a crate row's room does not widen the
    /// row's parent either.
    #[test]
    fn a_crate_row_keeps_an_egui_widget_overflow_from_its_parent() {
        assert_eq!(
            after_overflow(|ui, _| row(ui, 4.0, |ui| {
                ui.allocate_exact_size(egui::vec2(150.0, 20.0), egui::Sense::hover());
            })),
            (0.0, 100.0)
        );
    }

    /// The overflow goes where a flex item's goes: the end of a left
    /// column, both sides of a centred one, the start of a right column
    /// and of a right-to-left row.
    #[test]
    fn an_overflowing_widget_overflows_as_its_layout_aligns() {
        use egui::{Align, Layout};
        let overflow = |layout: Layout| {
            let ctx = egui::Context::default();
            let mut span = None;
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                let max =
                    egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(100.0, 50.0));
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(max).layout(layout));
                let (rect, _) =
                    allocate_overflowing(&mut child, egui::vec2(150.0, 20.0), egui::Sense::hover());
                span = Some((rect.left(), rect.right(), child.min_rect().width()));
            });
            span.expect("ran")
        };
        assert_eq!(
            overflow(Layout::top_down(Align::Min)),
            (100.0, 250.0, 100.0)
        );
        assert_eq!(
            overflow(Layout::top_down(Align::Center)),
            (75.0, 225.0, 100.0)
        );
        assert_eq!(overflow(Layout::top_down(Align::Max)), (50.0, 200.0, 100.0));
        assert_eq!(
            overflow(Layout::top_down_justified(Align::Center)),
            (100.0, 250.0, 100.0)
        );
        assert_eq!(
            overflow(Layout::left_to_right(Align::Center)),
            (100.0, 250.0, 100.0)
        );
        assert_eq!(
            overflow(Layout::right_to_left(Align::Center)),
            (50.0, 200.0, 100.0)
        );
    }
}
