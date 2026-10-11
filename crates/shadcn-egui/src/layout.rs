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
    // The content height is the tallest widget registered while `add`
    // runs. The child ui's `min_rect` cannot tell: in a centred row every
    // child's frame spans the full height. Not the union of the widgets
    // either: a child that egui aligns to the top (a nested `vstack`) next
    // to a widget whose rect overhangs its allocation would grow the union
    // on every pass.
    let layer = ui.layer_id();
    let before = ui
        .ctx()
        .viewport(|v| v.this_pass.widgets.get_layer(layer).count());
    let size = egui::vec2(ui.available_size_before_wrap().x, height);
    let mut own = id;
    let inner = ui
        .allocate_ui_with_layout(size, layout, |ui| {
            own = ui.unique_id();
            ui.spacing_mut().item_spacing.x = gap;
            add(ui)
        })
        .inner;
    // The child ui registers itself too, as high as the row: skip it.
    let content = ui.ctx().viewport(|v| {
        v.this_pass
            .widgets
            .get_layer(layer)
            .skip(before)
            .filter(|w| w.id != own)
            .map(|w| w.rect.height())
            .fold(0.0, f32::max)
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
