//! shadcn Separator, ported from shadcn-svelte `ui/separator/separator.svelte`
//! at `tools/stylegen/PIN`. It has no style marker: the base classes alone
//! describe it, a `bg-border` line one pixel thick.

use crate::style::Role;
use crate::Theme;
use egui::{Sense, Ui, Vec2};

fn line(ui: &mut Ui, size: impl FnOnce(f32, &Ui) -> Vec2) {
    let t = Theme::current(ui.ctx());
    // base: "data-horizontal:h-px" (and `data-vertical:w-px` for the vertical line)
    let thickness = t.metrics.border;
    let (rect, _) = ui.allocate_exact_size(size(thickness, ui), Sense::hover());
    if ui.is_rect_visible(rect) {
        // base: "bg-border"
        ui.painter().rect_filled(rect, 0.0, t.role(Role::Border));
    }
}

/// A horizontal rule across the available width.
pub fn separator(ui: &mut Ui) {
    // base: "data-horizontal:w-full"
    line(ui, |h, ui| Vec2::new(ui.available_width(), h));
}

/// A vertical rule as tall as the row it sits in.
pub fn separator_vertical(ui: &mut Ui) {
    // base: "data-vertical:self-stretch"
    line(ui, |w, ui| Vec2::new(w, ui.available_height()));
}
