//! shadcn Separator, ported from shadcn-svelte `ui/separator/separator.svelte`
//! at `tools/stylegen/PIN`. It has no style marker; its `data-slot="separator"`
//! classes are the `separator` recipe: a `bg-border` line, `h-px` / `w-px`
//! thick, or `metrics.border` thick where the recipe sets no thickness.

use crate::style::{pick, Prop, Recipe, State};
use crate::Theme;
use egui::{Sense, Ui, Vec2};

fn line(
    ui: &mut Ui,
    orientation: State,
    thickness: fn(&Recipe) -> &Prop<f32>,
    size: impl FnOnce(f32, &Ui) -> Vec2,
) {
    let t = Theme::current(ui.ctx());
    let l = [&*t.style.separator];
    let st = t.state(orientation);
    let px = pick(&l, thickness, st).unwrap_or(t.metrics.border);
    let (rect, _) = ui.allocate_exact_size(size(px, ui), Sense::hover());
    if ui.is_rect_visible(rect) {
        if let Some(c) = pick(&l, |r| &r.bg, st).map(|p| t.paint(p)) {
            ui.painter().rect_filled(rect, 0.0, c);
        }
    }
}

/// A horizontal rule across the available width.
pub fn separator(ui: &mut Ui) {
    // base: "data-horizontal:w-full"
    line(
        ui,
        State::HORIZONTAL,
        |r| &r.height,
        |h, ui| Vec2::new(ui.available_width(), h),
    );
}

/// A vertical rule as tall as the row it sits in.
pub fn separator_vertical(ui: &mut Ui) {
    // base: "data-vertical:self-stretch"
    line(
        ui,
        State::VERTICAL,
        |r| &r.width,
        |w, ui| Vec2::new(w, ui.available_height()),
    );
}
