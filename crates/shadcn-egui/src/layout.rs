//! Flexbox substitutes for egui's single-pass layout.
//! `gap` maps to item_spacing; space-between right-aligns via a nested layout.

use crate::style::TextSize;
use crate::Theme;
use egui::Ui;
use egui_extras::{Size, StripBuilder};

/// A row height change below this is float noise, not a reason to redo a pass.
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
/// items on a line and between wrapped lines.
pub fn wrap_row<R>(ui: &mut Ui, gap: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(gap, gap);
        add(ui)
    })
    .inner
}

/// Vertical stack with an explicit gap (flex-direction: column; gap: N).
pub fn vstack<R>(ui: &mut Ui, gap: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = gap;
        add(ui)
    })
    .inner
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
