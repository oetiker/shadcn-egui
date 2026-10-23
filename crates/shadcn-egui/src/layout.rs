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
    // The row's child Ui: the entry it registers, and the `id` its
    // children name as parent.
    let (mut own, mut row_ui) = (id, id);
    let inner = ui
        .allocate_ui_with_layout(size, layout, |ui| {
            (own, row_ui) = (ui.unique_id(), ui.id());
            ui.spacing_mut().item_spacing.x = gap;
            add(ui)
        })
        .inner;
    let content = ui.ctx().viewport(|v| {
        let added: Vec<egui::WidgetRect> = v
            .this_pass
            .widgets
            .get_layer(layer)
            .skip(before)
            .filter(|w| w.id != own)
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
    ui.allocate_ui_with_layout(size, layout.with_main_wrap(true), |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
        add(ui)
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

/// Tailwind's `sm` breakpoint (40rem).
const BREAKPOINT_SM: f32 = 640.0; // behaviour-ok: Tailwind default --breakpoint-sm
/// Tailwind's `md` breakpoint (48rem).
const BREAKPOINT_MD: f32 = 768.0; // behaviour-ok: Tailwind default --breakpoint-md

/// The breakpoint helper for media queries (`sm:`, `md:`, `@media (width
/// >= N)`): whether the window is at least `px` points wide. It reads what
/// a media query reads, the whole viewport (`Context::viewport_rect`),
/// safe-area insets included. Upstream's breakpoints are `min-width`
/// queries, so the boundary counts as wide.
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
    ui.horizontal(|ui| {
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
                        ui.label(label);
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
