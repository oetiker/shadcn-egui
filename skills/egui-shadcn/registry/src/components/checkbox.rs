//! shadcn Checkbox, ported from shadcn-svelte `ui/checkbox/checkbox.svelte`
//! at `tools/stylegen/PIN`. Base classes are quoted under `// base:`; all
//! other values come from the style's `cn-checkbox` and
//! `cn-checkbox-indicator` recipes. The tick is Lucide `check`.
//!
//! The indeterminate state of the upstream component is not offered: the
//! value is a plain `bool`.

use crate::components::icon::{paint_icon, CHECK};
use crate::components::shared::{box_shapes, disabled_painter, interaction_state, ring_shape};
use crate::style::{pick, Paint, Prop, Recipe, State};
use crate::Theme;
use egui::{vec2, Rect, Sense, Shape, Stroke, Ui, Vec2, Widget};

/// A checkbox. `checkbox(ui, &mut on)` is shorthand for
/// `ui.add(Checkbox::new(&mut on))`.
///
/// The returned response's rect is the visible box. Its click target is
/// larger (see the `base:` quote in [`Widget::ui`]), and the accessibility
/// node carries that larger rect.
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    enabled: bool,
    invalid: bool,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool) -> Self {
        Self { checked, enabled: true, invalid: false }
    }
    /// `false` draws the box disabled: no hover, no focus, no toggling.
    pub fn enabled(mut self, yes: bool) -> Self {
        self.enabled = yes;
        self
    }
    /// shadcn `aria-invalid`: destructive border and ring.
    pub fn invalid(mut self, yes: bool) -> Self {
        self.invalid = yes;
        self
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::current(ui.ctx());
        let l = [&t.style.checkbox];
        let enabled = self.enabled && ui.is_enabled();
        // Geometry depends on neither the value nor interaction.
        let g = t.state(State::NONE.with(State::INVALID, self.invalid));
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(&l, f, g).unwrap_or(0.0);

        let (rect, base) = ui.allocate_exact_size(vec2(get(|r| &r.width), get(|r| &r.height)), Sense::hover());
        // base: "after:absolute after:-inset-x-3 after:-inset-y-2"
        let hit = rect.expand2(vec2(12.0, 8.0));
        let sense = if enabled { Sense::click() } else { Sense::hover() };
        let mut resp = ui.interact(hit, base.id.with("hit"), sense);
        if enabled && resp.clicked() {
            *self.checked = !*self.checked;
            resp.mark_changed();
        }
        let checked = *self.checked;
        let context = (if checked { State::CHECKED } else { State::UNCHECKED }).with(State::INVALID, self.invalid);
        let st = interaction_state(&t, &resp, enabled, context);

        if ui.is_rect_visible(rect) {
            // The style's `disabled:opacity-50` arrives as the DISABLED state.
            let painter = disabled_painter(ui, self.enabled, pick(&l, |r| &r.opacity, st).unwrap_or(1.0));
            let paint = |f: fn(&Recipe) -> &Prop<Paint>| pick(&l, f, st).map(|p| t.paint(p));
            let radius = pick(&l, |r| &r.radius, st).map_or(0.0, |r| t.radius_px(r, rect.height() / 2.0));
            let mut shapes = Vec::new();
            if let Some(s) = pick(&l, |r| &r.shadow, st) {
                shapes.push(Shape::from(t.shadow(s).as_shape(rect, radius)));
            }
            let border_w = pick(&l, |r| &r.border_width, st).unwrap_or(0.0);
            let border = paint(|r| &r.border_color).map(|b| Stroke::new(border_w, b));
            let clip_padding = pick(&l, |r| &r.bg_clip_padding, st) == Some(true);
            shapes.extend(box_shapes(rect, radius, paint(|r| &r.bg), border, clip_padding));
            if let Some(c) = paint(|r| &r.ring_color) {
                shapes.extend(ring_shape(rect, radius, pick(&l, |r| &r.ring_width, st).unwrap_or(0.0), c));
            }
            painter.extend(shapes);
            if checked {
                let ind = [&t.style.checkbox_indicator];
                let side = pick(&ind, |r| &r.icon_size, st).unwrap_or(0.0);
                if let Some(color) = paint(|r| &r.text) {
                    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(side));
                    paint_icon(&painter, icon_rect, &CHECK, color, t.metrics.icon_stroke);
                }
            }
        }
        resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, checked, ""));
        resp.rect = rect;
        resp
    }
}

/// Shorthand for `ui.add(Checkbox::new(checked))`.
pub fn checkbox(ui: &mut Ui, checked: &mut bool) -> egui::Response {
    ui.add(Checkbox::new(checked))
}
