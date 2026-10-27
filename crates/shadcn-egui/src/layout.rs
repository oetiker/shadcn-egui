//! Flexbox substitutes for egui's single-pass layout.
//! `gap` maps to item_spacing; space-between right-aligns via a nested layout.

use crate::components::context;
use crate::style::TextSize;
use crate::Theme;
use egui::Ui;
use egui_extras::{Size, StripBuilder};

/// A row height or stack width change below this is float noise, not a
/// reason to redo a pass.
const ROW_HEIGHT_EPSILON: f32 = 0.01; // behaviour-ok: float tolerance, not a design token

/// Whether a pass of `ui` may replace what a layout helper stored from
/// an earlier pass (`stored`). A sizing pass (the first frame of an
/// `egui::Area` or a `Grid`, or one the app forces) lays out in a room and
/// an alignment of its own, invisibly: it fills a slot nothing filled
/// yet, so that the visible pass it discards into starts from it, but it
/// never replaces what a visible pass measured, nor redoes a pass that is
/// not shown.
fn may_store(ui: &Ui, stored: bool) -> bool {
    !(stored && ui.is_sizing_pass())
}

/// Horizontal stack with an explicit gap (flex-direction: row; gap: N;
/// align-items: center).
///
/// egui centres each child in the row height so far, so a child taller than
/// `interact_size.y` would leave the children before it above centre. The
/// row therefore starts from the content height of the last pass, and asks
/// egui to redo a pass whose content height changed.
///
/// A widget that fills the available height (a `ScrollArea`) takes the
/// row's current height as its own, so a row that holds one does not
/// shrink after a taller child goes; a `ScrollArea` grows the row to its
/// full content height. Content that registers no
/// widget (shapes painted directly, `allocate_space`) is not measured.
///
/// A child wider than the room left pushes the children after it past its
/// whole width, and the row takes no more than its room in the parent, so
/// the overflow does not widen the parent: the browser's flex row. A plain
/// `ui.horizontal` cannot do that (see [`allocate_overflowing`]).
///
/// A child that is as high as the row's other children (`self-stretch`,
/// as `separator_vertical`) takes its height from [`stretch_height`].
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
    // The content height: the tallest item of the row, measured from the
    // widgets registered while `add` runs (see `row_lines`), so it does not
    // depend on the row's own height.
    let size = egui::vec2(ui.available_size_before_wrap().x, height);
    let (inner, lines) = overflow_row(ui, size, layout, |_, ui| {
        let items = RowItems::begin(ui);
        ui.spacing_mut().item_spacing.x = gap;
        let inner = shrink_stacks(ui, |ui| {
            let inner = add(ui);
            (inner, ui.min_rect().width())
        });
        (inner, items.finish(ui, false))
    });
    let content = lines.iter().map(|l| l.height).fold(0.0, f32::max);
    let measured = content.max(floor);
    if may_store(ui, stored.is_some())
        && stored.is_none_or(|h| (h - measured).abs() > ROW_HEIGHT_EPSILON)
    {
        ui.data_mut(|d| d.insert_temp(id, measured));
        if (height - measured).abs() > ROW_HEIGHT_EPSILON {
            ui.ctx()
                .request_discard("layout::row content height changed");
        }
    }
    inner
}

fn stretch_id(row: egui::Id) -> egui::Id {
    row.with("shadcn_egui::layout::stretch")
}

fn stretch_pass_id(row: egui::Id) -> egui::Id {
    row.with("shadcn_egui::layout::stretch_pass")
}

/// What a crate row gives its stretch items in a pass and learns about
/// them: the height the previous pass found for each [`stretch_height`]
/// call, in call order; the place on the layer of the next entry after
/// each call, which marks the item that asked; and the `Ui`s whose
/// children are the row's items: `roots` by the `id` their children name
/// as parent, `skip` the unique ids of the `Ui`s between a root and the
/// row, which are no items themselves.
#[derive(Clone, Debug, Default)]
struct Stretch {
    heights: Vec<f32>,
    marks: Vec<usize>,
    roots: Vec<egui::Id>,
    skip: Vec<egui::Id>,
}

/// One line of a crate row's items in a pass: the height of its tallest
/// item and of its first.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Line {
    height: f32,
    first: f32,
}

/// A crate row's pass over its items ([`row`], [`wrap_row`],
/// [`space_between`], a card footer), for [`stretch_height`]: begun on the
/// row's `Ui` before its items, finished after them.
pub(crate) struct RowItems {
    /// The row `Ui`'s unique id, which [`flex_row_of`] finds.
    row: egui::Id,
    layer: egui::LayerId,
    before: usize,
    stored: Option<Vec<f32>>,
}

impl RowItems {
    pub(crate) fn begin(ui: &Ui) -> Self {
        let (row, layer) = (ui.unique_id(), ui.layer_id());
        let before = ui
            .ctx()
            .viewport(|v| v.this_pass.widgets.get_layer(layer).count());
        let stored = ui.data(|d| d.get_temp::<Vec<f32>>(stretch_id(row)));
        let pass = Stretch {
            heights: stored.clone().unwrap_or_default(),
            roots: vec![ui.id()],
            ..Stretch::default()
        };
        ui.data_mut(|d| d.insert_temp(stretch_pass_id(row), pass));
        Self {
            row,
            layer,
            before,
            stored,
        }
    }

    /// Ends the pass: keeps each stretch item's height for the next pass
    /// (the tallest other item of its line) and redoes the pass where one
    /// changed. Gives the row's lines: one, or where the row `wraps` one
    /// per line its items were placed on.
    pub(crate) fn finish(self, ui: &Ui, wraps: bool) -> Vec<Line> {
        let pass = ui
            .data_mut(|d| d.remove_temp::<Stretch>(stretch_pass_id(self.row)))
            .unwrap_or_default();
        let (lines, heights) = ui.ctx().viewport(|v| {
            let layer = v.this_pass.widgets.get_layer(self.layer);
            let added: Vec<(usize, egui::WidgetRect)> = layer
                .enumerate()
                .skip(self.before)
                .map(|(i, w)| (i, *w))
                .collect();
            let end = self.before + added.len();
            row_lines(&added, end, &pass, wraps)
        });
        let changed = self.stored.as_ref().is_none_or(|old| {
            old.len() != heights.len()
                || old
                    .iter()
                    .zip(&heights)
                    .any(|(a, b)| (a - b).abs() > ROW_HEIGHT_EPSILON)
        });
        if changed && may_store(ui, self.stored.is_some()) {
            ui.data_mut(|d| d.insert_temp(stretch_id(self.row), heights));
            if !pass.marks.is_empty() {
                ui.ctx()
                    .request_discard("layout: a row's stretch height changed");
            }
        }
        lines
    }
}

/// The lines of a crate row's items from the entries `added` on its layer
/// in a pass (each with its place on the layer, `end` the place after the
/// last), and the height for each of the pass's stretch marks: the
/// tallest other item of the mark's line.
fn row_lines(
    added: &[(usize, egui::WidgetRect)],
    end: usize,
    pass: &Stretch,
    wraps: bool,
) -> (Vec<Line>, Vec<f32>) {
    // An item is a child of a root: a leaf widget, or a Ui (a scope,
    // `add_enabled_ui`, a nested `vstack`) that counts as the union of the
    // leaf widgets inside it, not as its own rect. A Ui's own rect cannot
    // serve: in a centred row egui makes it as high as the row, so a row
    // holding one would never shrink. Neither can the union of all
    // widgets: a nested `vstack` sits at the top while a widget beside it
    // is centred, and the union grew on every pass.
    //
    // Rebuild the tree from the creation order: a Ui registers before what
    // it holds, and an entry names its Ui's `id` as parent, not the
    // `unique_id` the Ui registered under. So an entry whose next entry
    // names a parent not on the stack opens a Ui.
    let mut stack = vec![pass.roots[0]];
    // Each item's first place on the layer and its rect.
    let mut items: Vec<(usize, egui::Rect)> = Vec::new();
    for (i, (at, w)) in added.iter().enumerate() {
        while stack.len() > 1 && stack.last() != Some(&w.parent_id) {
            stack.pop();
        }
        let skipped = pass.skip.contains(&w.id);
        if !skipped && stack.last().is_some_and(|p| pass.roots.contains(p)) {
            items.push((*at, egui::Rect::NOTHING));
        }
        match added.get(i + 1).map(|(_, n)| n.parent_id) {
            Some(p) if !stack.contains(&p) => stack.push(p),
            _ if !skipped => {
                if let Some((_, c)) = items.last_mut() {
                    *c = c.union(w.rect);
                }
            }
            _ => {}
        }
    }
    // An item starts a line when it is placed below the line so far.
    let mut line_of = vec![0; items.len()];
    let mut lines: Vec<Line> = Vec::new();
    let mut bottom = f32::NEG_INFINITY;
    for (k, (_, r)) in items.iter().enumerate() {
        if !r.is_finite() {
            line_of[k] = lines.len().saturating_sub(1);
            continue;
        }
        if lines.is_empty() || (wraps && r.top() >= bottom) {
            lines.push(Line {
                height: 0.0,
                first: r.height(),
            });
            bottom = f32::NEG_INFINITY;
        }
        bottom = bottom.max(r.bottom());
        line_of[k] = lines.len() - 1;
        let line = lines.last_mut().expect("a line was pushed");
        line.height = line.height.max(r.height());
    }
    // A stretch item is the item that holds the first entry registered
    // after its mark.
    let item_at = |m: usize| {
        (0..items.len()).find(|&k| {
            let next = items.get(k + 1).map_or(end, |c| c.0);
            (items[k].0..next).contains(&m)
        })
    };
    let marked: Vec<Option<usize>> = pass.marks.iter().map(|&m| item_at(m)).collect();
    let stretches = |k: usize| marked.contains(&Some(k));
    let tallest_other = |line: Option<usize>| {
        (0..items.len())
            .filter(|&k| !stretches(k) && line.is_none_or(|l| line_of[k] == l))
            .map(|k| items[k].1.height())
            .fold(0.0, f32::max)
    };
    let heights = marked
        .iter()
        .map(|k| tallest_other(k.map(|k| line_of[k])))
        .collect();
    (lines, heights)
}

/// Makes the children of `ui` items of the crate row `ui` sits in, through
/// `Ui`s of the crate's own (`space_between`'s right side, a card footer's
/// text scope), so that [`stretch_height`] counts them one by one.
pub(crate) fn hold_row_items(ui: &Ui) {
    let mut skip = Vec::new();
    for s in ui.stack().iter() {
        if context::on_frame::<RowContext>(s).is_some() {
            let root = ui.id();
            ui.data_mut(|d| {
                if let Some(mut pass) = d.get_temp::<Stretch>(stretch_pass_id(s.id)) {
                    pass.roots.push(root);
                    pass.skip.extend(skip);
                    d.insert_temp(stretch_pass_id(s.id), pass);
                }
            });
            return;
        }
        skip.push(s.id);
    }
}

/// The height of a `self-stretch` flex item: in a crate row ([`row`],
/// [`space_between`], a card footer), the height of the row's tallest
/// other child in the previous pass, without the row's `interact_size`
/// floor (a vertical rule beside small text is as high as the text); in a
/// [`wrap_row`], the tallest other child of the item's own line. The
/// child that calls this before it draws counts as a stretch item, and
/// stretch items do not count as other children; where the row's other
/// children change height, the row redoes the pass.
///
/// `None` where the item gives no height: in a column (a [`vstack`], a
/// vertical `Ui`), in a box of its own on the way to the row (a frame, a
/// scroll area, a `StripBuilder` or table cell), outside a crate row, and
/// in the row's first pass. The item then takes its own minimum.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// shadcn_egui::layout::row(ui, 8.0, |ui| {
///     ui.label("Ctrl");
///     let height = shadcn_egui::layout::stretch_height(ui).unwrap_or(0.0);
///     ui.allocate_exact_size(egui::vec2(1.0, height), egui::Sense::hover());
///     ui.label("K");
/// });
/// # });
/// ```
#[doc(alias = "self-stretch")]
#[must_use]
pub fn stretch_height(ui: &Ui) -> Option<f32> {
    let row = flex_row_of(ui)?;
    let layer = ui.layer_id();
    let at = ui
        .ctx()
        .viewport(|v| v.this_pass.widgets.get_layer(layer).count());
    ui.data_mut(|d| {
        let mut pass = d.get_temp::<Stretch>(stretch_pass_id(row))?;
        let height = pass.heights.get(pass.marks.len()).copied();
        pass.marks.push(at);
        d.insert_temp(stretch_pass_id(row), pass);
        height
    })
}

/// Horizontal stack that wraps onto further lines when it runs out of width
/// (flex-direction: row; flex-wrap: wrap; gap: N; align-items: center).
/// The gap applies both between items on a line and between wrapped lines.
/// A [`vstack`] item wraps as a whole, as a flex item does.
///
/// The first line starts from its height in the last pass, so an item
/// placed before a taller one is centred on the line, as in a [`row`].
/// egui starts each further line as high as the line before it, so there
/// an item before a taller one sits higher than the centre, and a line
/// after a taller one is as high as that one.
pub fn wrap_row<R>(ui: &mut Ui, gap: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    // As `horizontal_wrapped`, but each line is as high as its tallest item:
    // egui's own starts every line `interact_size.y` high, which puts more
    // than `gap` between lines of smaller items (badges).
    let id = ui.next_auto_id().with("shadcn_egui::layout::wrap_row");
    let stored = ui.data(|d| d.get_temp::<f32>(id));
    let height = stored.unwrap_or(0.0);
    let layout = if ui.layout().prefer_right_to_left() {
        egui::Layout::right_to_left(egui::Align::Center)
    } else {
        egui::Layout::left_to_right(egui::Align::Center)
    };
    let size = egui::vec2(ui.available_size_before_wrap().x, height);
    let (inner, lines) = overflow_row(ui, size, layout.with_main_wrap(true), |_, ui| {
        let items = RowItems::begin(ui);
        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
        let inner = add(ui);
        (inner, items.finish(ui, true))
    });
    let first = lines.first().map_or(Line::default(), |l| *l);
    if may_store(ui, stored.is_some())
        && stored.is_none_or(|h| (h - first.height).abs() > ROW_HEIGHT_EPSILON)
    {
        ui.data_mut(|d| d.insert_temp(id, first.height));
        // egui grows the line to each item as it comes, so a line that
        // started lower than it ends places its items as one that started
        // at its height only when the first item is the tallest.
        let low = height < first.height - ROW_HEIGHT_EPSILON;
        if height > first.height + ROW_HEIGHT_EPSILON
            || (low && first.first < first.height - ROW_HEIGHT_EPSILON)
        {
            ui.ctx()
                .request_discard("layout::wrap_row line height changed");
        }
    }
    inner
}

/// The context on the `Ui` of a crate row ([`row`], [`wrap_row`],
/// [`space_between`], a card footer), where [`allocate_overflowing`]
/// allocates a widget's whole width.
#[derive(Clone, Copy)]
struct RowContext;

/// The `UiBuilder` of a crate row's `Ui`: `max_rect` and `layout`, tagged
/// as a row. The caller allocates only the part of the row's `min_rect`
/// that fits its room, so an overflowing child pushes its siblings without
/// widening the parent.
pub(crate) fn row_builder(max_rect: egui::Rect, layout: egui::Layout) -> egui::UiBuilder {
    egui::UiBuilder::new()
        .max_rect(max_rect)
        .layout(layout)
        .ui_stack_info(context::info(egui::UiStackInfo::default(), RowContext))
}

/// The context on the `Ui` of a [`vstack`]: a flex column, which grows to
/// the width of its widest child inside a crate row.
#[derive(Clone, Copy)]
struct VstackContext;

/// Whether `ui` is a crate row, or sits inside one through horizontal
/// `Ui`s (a scope, an enabled scope) and [`vstack`]s: a flex item of the
/// row, which the browser grows to its content's minimum width. A
/// `StripBuilder` or table cell on the way keeps its width, as a sized box
/// does in a flex row; a frame (a card) grows with its content, as a flex
/// item does.
fn in_row(ui: &Ui) -> bool {
    for s in ui.stack().iter() {
        if context::on_frame::<RowContext>(s).is_some() {
            return true;
        }
        if s.kind() == Some(egui::UiKind::TableCell)
            || (!s.layout_direction.is_horizontal()
                && !context::on_frame::<VstackContext>(s).is_some())
        {
            return false;
        }
    }
    false
}

/// A crate row of `size` in `layout`, placed as `allocate_ui_with_layout`
/// places it. `add` gets the `Ui` that holds the room and the row's own
/// `Ui`, built inside it with [`row_builder`]. The row's children may run
/// past the room; the room takes the row's `min_rect` cut to its own
/// width, and the parent allocates that. A row that is itself a flex item
/// of a crate row (in a [`vstack`] in a row) is not cut: the outer row
/// takes the overflow, as the browser grows a flex item to its content.
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
        let grows = in_row(room);
        let x = |v: f32| {
            if grows {
                v
            } else {
                v.clamp(max.left(), max.right())
            }
        };
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
///
/// Inside a crate row ([`row`], [`wrap_row`], [`space_between`], a card
/// footer) it grows to its widest child, as a flex item does: a child
/// raised past the room ([`allocate_overflowing`]) widens the stack and
/// pushes the row's next children, and the row keeps the overflow from
/// its own parent. Outside a crate row the child overflows the stack.
///
/// In a [`row`], [`space_between`] or a card footer the stack shrinks as a
/// flex item (`flex-shrink: 1`, `min-width: auto`): its text keeps its
/// widest, unwrapped width while the row has room for it, and wraps at the
/// width the row leaves after its other children when it has not. Several
/// stacks in one row share the shortfall in proportion to their unwrapped
/// widths, none below its narrowest (its longest word or widest child).
/// The widths come from the previous pass, and a pass that changes them
/// is redone. When the stack's content takes another size or paints
/// another text or image at the same width (its text, image, font or
/// scale changed), the next frame measures it unwrapped again in a pass
/// that is not shown.
///
/// In a `StripBuilder` or table cell (directly, or through scopes that
/// keep the cell's layout) the stack takes the cell's width, as a block in
/// a grid track or a sized flex item fills it, and its text wraps there.
pub fn vstack<R>(ui: &mut Ui, gap: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    // `ui.vertical`, tagged as a flex column.
    let column = egui::UiBuilder::new()
        .layout(egui::Layout::top_down(egui::Align::Min))
        .ui_stack_info(context::info(egui::UiStackInfo::default(), VstackContext));
    let id = ui.next_auto_id().with("shadcn_egui::layout::vstack");
    if !ui.layout().main_wrap() {
        let Some(row) = flex_row_of(ui) else {
            let fill = in_cell(ui);
            return ui
                .scope_builder(column, |ui| {
                    if fill {
                        ui.set_min_width(ui.available_width());
                    }
                    ui.spacing_mut().item_spacing.y = gap;
                    add(ui)
                })
                .inner;
        };
        return flex_item(ui, row, id, column, |ui| {
            ui.spacing_mut().item_spacing.y = gap;
            add(ui)
        });
    }
    let stored = ui.data(|d| d.get_temp::<f32>(id));
    let line_start = ui.cursor().min.x <= ui.max_rect().min.x;
    if !line_start && stored.is_some_and(|w| w > ui.available_size_before_wrap().x) {
        ui.end_row();
    }
    let fits = stored.is_none_or(|w| w <= ui.available_size_before_wrap().x);
    let response = ui.scope_builder(column, |ui| {
        ui.spacing_mut().item_spacing.y = gap;
        if fits {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        }
        add(ui)
    });
    // Only an unwrapped pass measures the stack's own width.
    let width = response.response.rect.width();
    if fits
        && may_store(ui, stored.is_some())
        && stored.is_none_or(|w| (w - width).abs() > ROW_HEIGHT_EPSILON)
    {
        ui.data_mut(|d| d.insert_temp(id, width));
        ui.ctx()
            .request_discard("layout::vstack width changed in a wrap_row");
    }
    response.inner
}

/// The crate row whose flex item a [`vstack`] started in `ui` is: the
/// row's `Ui` unique id, reached through horizontal `Ui`s (a scope, an enabled
/// scope, `space_between`'s right side), not through a [`vstack`], a
/// vertical `Ui` or a box of its own (a `StripBuilder` or table cell, a
/// frame, a scroll area: any `Ui` of a [`egui::UiKind`]), whose children
/// are not the row's items.
fn flex_row_of(ui: &Ui) -> Option<egui::Id> {
    for s in ui.stack().iter() {
        if context::on_frame::<RowContext>(s).is_some() {
            return Some(s.id);
        }
        if s.kind().is_some()
            || !s.layout_direction.is_horizontal()
            || context::on_frame::<VstackContext>(s).is_some()
        {
            return None;
        }
    }
    None
}

/// Whether a [`vstack`] started in `ui` is the content of a `StripBuilder`
/// or table cell: `ui` is the cell, or reaches it through scopes that keep
/// their parent's layout and are no crate row or stack.
fn in_cell(ui: &Ui) -> bool {
    let mut s = ui.stack();
    loop {
        match s.kind() {
            Some(egui::UiKind::TableCell) => return true,
            Some(_) => return false,
            None => {}
        }
        let Some(parent) = s.parent.as_ref() else {
            return false;
        };
        if !s.tags().0.is_empty() || s.layout_direction != parent.layout_direction {
            return false;
        }
        s = parent;
    }
}

/// What a row learns about one of its [`vstack`] items in a pass.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct FlexItem {
    /// The width it took.
    width: f32,
    /// Its unwrapped width (CSS max-content), the flex base size.
    basis: f32,
    /// Its narrowest width (CSS min-content), as far as known: what it
    /// overflowed to at a smaller width, else 0.
    min: f32,
}

/// What a [`vstack`] keeps of its own content between passes.
#[derive(Clone, Copy, Debug, Default)]
struct Intrinsic {
    /// The width it took unwrapped.
    max: Option<f32>,
    /// The width it overflowed to when made narrower (exact), or lowered
    /// to the width it wrapped to (an upper bound).
    min: Option<f32>,
    /// The last pass at a planned width: that width, the scale, the
    /// content's size and the hash of the text and images it painted
    /// ([`content_hash`]). The same width and scale giving another size or
    /// hash means the content or its font changed, and `max` and `min`
    /// with it, also where the wrapped block kept its size.
    seen: Option<([f32; 4], u64)>,
    /// Set when `seen` changed: the next pass measures the content again.
    stale: bool,
}

/// A hash of what decides the unwrapped width of the content painted on
/// `layer` from shape `first` on: each galley's text and per section its
/// byte range, font and letter spacing (not its colour, which a hover
/// changes), and each image's texture, its size and the part (uv) drawn.
/// A shrunk image of another intrinsic size, or an icon atom (an image
/// per icon) swapped for another, can draw at the same size.
///
/// Only `layer` counts: what the stack shows on another layer (a popup,
/// a tooltip) is out of flow, as an absolutely positioned box is in CSS,
/// and never sizes the stack. Shapes without a texture (an icon a crate
/// widget paints as paths, a frame) draw at a size their widget fixes,
/// so the content's size already tells a change.
fn content_hash(ctx: &egui::Context, layer: egui::LayerId, first: usize) -> u64 {
    use std::hash::{Hash, Hasher};
    fn image(
        id: egui::TextureId,
        uv: impl Iterator<Item = egui::Pos2>,
        h: &mut impl Hasher,
        images: &mut Vec<egui::TextureId>,
    ) {
        // The font atlas (the default texture) grows as glyphs are added;
        // text is hashed by its galley.
        if id != egui::TextureId::default() {
            id.hash(h);
            uv.for_each(|p| [p.x.to_bits(), p.y.to_bits()].hash(h));
            images.push(id);
        }
    }
    fn add(shape: &egui::Shape, h: &mut impl Hasher, images: &mut Vec<egui::TextureId>) {
        match shape {
            egui::Shape::Text(text) => {
                let job = &text.galley.job;
                job.text.hash(h);
                for section in &job.sections {
                    section.byte_range.hash(h);
                    section.leading_space.to_bits().hash(h);
                    section.format.font_id.hash(h);
                    section.format.extra_letter_spacing.to_bits().hash(h);
                }
            }
            egui::Shape::Rect(rect) => {
                if let Some(brush) = &rect.brush {
                    let uv = brush.uv;
                    image(
                        brush.fill_texture_id,
                        [uv.min, uv.max].into_iter(),
                        h,
                        images,
                    );
                }
            }
            egui::Shape::Mesh(mesh) => {
                image(
                    mesh.texture_id,
                    mesh.vertices.iter().map(|v| v.uv),
                    h,
                    images,
                );
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| add(s, h, images)),
            _ => {}
        }
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut images = Vec::new();
    ctx.graphics(|g| {
        if let Some(list) = g.get(layer) {
            list.all_entries()
                .skip(first)
                .for_each(|c| add(&c.shape, &mut h, &mut images));
        }
    });
    // The texture's own size: the same id can be given another image.
    let textures = ctx.tex_manager();
    let textures = textures.read();
    for id in images {
        textures.meta(id).map(|m| m.size).hash(&mut h);
    }
    h.finish()
}

fn flex_items_id(row: egui::Id) -> egui::Id {
    row.with("shadcn_egui::layout::flex_items")
}

fn flex_plan_id(row: egui::Id) -> egui::Id {
    row.with("shadcn_egui::layout::flex_plan")
}

/// Runs `add` in a crate row's `ui` and resolves the widths of its
/// [`vstack`] items for the next pass as the browser shrinks flex items.
/// `add` returns its result and the row's content width: the sum of its
/// items and the gaps between them, without free space.
pub(crate) fn shrink_stacks<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> (R, f32)) -> R {
    // The id a `UiStack` entry carries, which `flex_row_of` finds.
    let row = ui.unique_id();
    ui.data_mut(|d| d.insert_temp(flex_items_id(row), Vec::<FlexItem>::new()));
    let room = ui.max_rect().width();
    let (inner, content) = add(ui);
    let items = ui
        .data_mut(|d| d.remove_temp::<Vec<FlexItem>>(flex_items_id(row)))
        .unwrap_or_default();
    let stacks: f32 = items.iter().map(|i| i.width).sum();
    // A stack that filled the unbounded room (egui's `Separator`) took an
    // infinite width and pushed the rest of the row out of reach; its plan
    // gives it a finite one, and the next pass measures the rest.
    let others = Some(content - stacks).filter(|w| w.is_finite());
    let plan = shrink(room - others.unwrap_or(0.0), &items);
    let stored = ui.data(|d| d.get_temp::<Vec<Option<f32>>>(flex_plan_id(row)));
    let same = stored.as_ref().is_some_and(|old| {
        old.len() == plan.len()
            && old.iter().zip(&plan).all(|(a, b)| match (a, b) {
                (Some(a), Some(b)) => (a - b).abs() <= ROW_HEIGHT_EPSILON,
                (a, b) => a.is_none() && b.is_none(),
            })
    });
    if !same && may_store(ui, stored.is_some()) {
        ui.data_mut(|d| d.insert_temp(flex_plan_id(row), plan));
        if !items.is_empty() {
            ui.ctx()
                .request_discard("layout: a vstack's width in a row changed");
        }
    }
    inner
}

/// The widths of flex items that share `room`, by CSS's "resolve flexible
/// lengths" for a shrinking line (`flex-shrink: 1`, no grow): `None` for
/// all when their bases fit, else each base less its share of the
/// shortfall in proportion to its base, an item held at its min-content
/// leaving the rest to the others.
///
/// An item whose base is infinite (it holds a widget that fills whatever
/// room it gets, as egui's `Separator`) takes the room the others leave,
/// as that widget fills a plain egui row, at least its min-content.
fn shrink(room: f32, items: &[FlexItem]) -> Vec<Option<f32>> {
    let fills = items.iter().filter(|i| i.basis.is_infinite()).count();
    if fills > 0 {
        let (fill, rest): (Vec<_>, Vec<_>) = items.iter().partition(|i| i.basis.is_infinite());
        let rest: Vec<FlexItem> = rest.into_iter().copied().collect();
        let held: f32 = fill.iter().map(|i| i.min).sum();
        let mut plan = shrink(room - held, &rest).into_iter();
        let used: f32 = rest
            .iter()
            .zip(plan.clone())
            .map(|(i, w)| w.unwrap_or(i.basis))
            .sum();
        let share = ((room - used) / fills as f32).max(0.0);
        return items
            .iter()
            .map(|i| {
                if i.basis.is_infinite() {
                    Some(share.max(i.min))
                } else {
                    plan.next().flatten()
                }
            })
            .collect();
    }
    let bases: f32 = items.iter().map(|i| i.basis).sum();
    if bases <= room + ROW_HEIGHT_EPSILON {
        return vec![None; items.len()];
    }
    let mut frozen = vec![false; items.len()];
    let mut width: Vec<f32> = items.iter().map(|i| i.basis).collect();
    loop {
        let held: f32 = (0..items.len())
            .filter(|&i| frozen[i])
            .map(|i| width[i])
            .sum();
        let open: f32 = (0..items.len())
            .filter(|&i| !frozen[i])
            .map(|i| items[i].basis)
            .sum();
        if open <= 0.0 {
            break;
        }
        let free = room - held;
        let mut clamped = false;
        for (i, item) in items.iter().enumerate() {
            if frozen[i] {
                continue;
            }
            width[i] = item.basis * free / open;
            if width[i] < item.min {
                width[i] = item.min;
                frozen[i] = true;
                clamped = true;
            }
        }
        if !clamped {
            break;
        }
    }
    width.into_iter().map(Some).collect()
}

/// A [`vstack`] as an item of the crate row `row`: laid out unwrapped
/// when the row's last plan gives it no width, else at that width, and
/// reported to the row.
fn flex_item<R>(
    ui: &mut Ui,
    row: egui::Id,
    id: egui::Id,
    column: egui::UiBuilder,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let index = ui.data_mut(|d| {
        let items = d.get_temp_mut_or_default::<Vec<FlexItem>>(flex_items_id(row));
        items.push(FlexItem::default());
        items.len() - 1
    });
    let stored = ui.data(|d| d.get_temp::<Intrinsic>(id));
    let store = may_store(ui, stored.is_some());
    let mut known = stored.unwrap_or_default();
    let target = ui
        .data(|d| d.get_temp::<Vec<Option<f32>>>(flex_plan_id(row)))
        .and_then(|plan| plan.get(index).copied().flatten())
        .filter(|_| !known.stale);
    if known.stale && store {
        // A measuring pass: never shown.
        ui.ctx()
            .request_discard("layout: a vstack in a row measures its content");
        known = Intrinsic::default();
    }
    let mut content = egui::Vec2::ZERO;
    let layer = ui.layer_id();
    let first = ui
        .ctx()
        .graphics(|g| g.get(layer).map_or(0, |l| l.next_idx().0));
    let response = ui.scope_builder(column, |ui| {
        // Unbounded, text does not wrap and a filling widget takes its
        // content minimum (`capped_width`): the stack's max-content.
        ui.set_max_width(target.unwrap_or(f32::INFINITY));
        let inner = add(ui);
        content = ui.min_rect().size();
        // A flex item is as wide as the row made it, also where its
        // content wraps narrower.
        if let Some(w) = target {
            ui.set_min_width(w);
        }
        inner
    });
    if let Some(w) = target.filter(|_| store) {
        let seen = (
            [w, ui.ctx().pixels_per_point(), content.x, content.y],
            content_hash(ui.ctx(), layer, first),
        );
        if known
            .seen
            .is_some_and(|s| s.0[..2] == seen.0[..2] && s != seen)
        {
            // Measured next frame, so this one keeps a valid layout.
            known.stale = true;
            ui.ctx().request_repaint();
        }
        known.seen = Some(seen);
    }
    let content = content.x;
    match target {
        None => known.max = Some(content),
        Some(w) if content > w + ROW_HEIGHT_EPSILON => known.min = Some(content),
        Some(_) => {
            if known.min.is_some_and(|m| content < m - ROW_HEIGHT_EPSILON) {
                known.min = Some(content);
            }
        }
    }
    let width = response.response.rect.width();
    ui.data_mut(|d| {
        if store {
            d.insert_temp(id, known);
        }
        if let Some(item) = d
            .get_temp_mut_or_default::<Vec<FlexItem>>(flex_items_id(row))
            .get_mut(index)
        {
            *item = FlexItem {
                width,
                basis: known.max.unwrap_or(width),
                min: known.min.unwrap_or(0.0),
            };
        }
    });
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
/// footer), or a horizontal scope or a [`vstack`] inside one, the widget
/// takes its whole width and the next widget starts after it, as in the
/// browser; the row keeps the overflow from its own parent. A
/// `StripBuilder` or table cell in the row keeps its width: the widget
/// overflows the cell, and the row's next widget stays after the cell. In a plain `ui.horizontal` the
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
        let items = RowItems::begin(ui);
        shrink_stacks(ui, |ui| {
            left(ui);
            let start = ui.min_rect().width();
            let end = ui
                .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    hold_row_items(ui);
                    right(ui);
                })
                .response
                .rect
                .width();
            // The free space between the two sides is not content.
            let gap = if end > 0.0 {
                ui.spacing().item_spacing.x
            } else {
                0.0
            };
            ((), start + gap + end)
        });
        items.finish(ui, false);
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

    /// A `vstack` in a crate row is a flex item: it grows to an
    /// overflowing child, and the row's next child starts after it, also
    /// when the child sits in a row of its own inside the stack. The
    /// column holding the row keeps its width.
    #[test]
    fn a_vstack_in_a_crate_row_grows_to_its_overflowing_child() {
        let in_stack = |inner_row: bool| {
            let ctx = egui::Context::default();
            let mut out = None;
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                let max = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 200.0));
                let mut column = ui.new_child(egui::UiBuilder::new().max_rect(max));
                let mut stack_width = 0.0;
                let next = row(&mut column, 4.0, |ui| {
                    stack_width = vstack(ui, 0.0, |ui| {
                        let overflow = |ui: &mut Ui| {
                            allocate_overflowing(ui, egui::vec2(150.0, 20.0), egui::Sense::hover());
                        };
                        if inner_row {
                            row(ui, 4.0, overflow);
                        } else {
                            overflow(ui);
                        }
                        ui.min_rect().width()
                    });
                    ui.allocate_exact_size(egui::vec2(10.0, 20.0), egui::Sense::hover())
                        .0
                        .left()
                });
                out = Some((stack_width, next, column.min_rect().width()));
            });
            out.expect("ran")
        };
        assert_eq!(in_stack(false), (150.0, 154.0, 100.0));
        assert_eq!(in_stack(true), (150.0, 154.0, 100.0));
    }

    /// Outside a crate row a `vstack` is a block: the child overflows it.
    #[test]
    fn a_vstack_outside_a_row_keeps_its_width() {
        assert_eq!(
            after_overflow(|ui, add| {
                vstack(ui, 0.0, |ui| add(ui));
            }),
            (0.0, 100.0)
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
