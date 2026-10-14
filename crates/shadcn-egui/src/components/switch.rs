//! shadcn Switch, ported from shadcn-svelte `ui/switch/switch.svelte` at
//! `tools/stylegen/PIN`. Base classes are quoted under `// base:`; all other
//! values come from the style's `cn-switch` (track) and `cn-switch-thumb`
//! recipes. The thumb sits inside the track's 1px transparent border and
//! slides between its two positions with `animate_bool`.

use crate::components::shared::{
    box_shapes, disabled_painter, interaction_state, ring_shape, shift,
};
use crate::style::{pick, Paint, Prop, Recipe, State};
use crate::Theme;
use egui::{lerp, vec2, Rect, Sense, Shape, Stroke, Ui, Widget};

/// shadcn `data-size` of the switch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwitchSize {
    #[default]
    Default,
    Sm,
}

/// A switch. `toggle(ui, &mut on)` is shorthand for
/// `ui.add(Switch::new(&mut on))`.
///
/// The returned response's rect is the visible track. Its click target is
/// larger (see the `base:` quote in [`Widget::ui`]), and the accessibility
/// node (`widget_info`) carries that larger rect. egui has no switch role,
/// so the node reports as a checkbox.
pub struct Switch<'a> {
    on: &'a mut bool,
    size: SwitchSize,
    enabled: bool,
    invalid: bool,
}

impl<'a> Switch<'a> {
    pub fn new(on: &'a mut bool) -> Self {
        Self {
            on,
            size: SwitchSize::Default,
            enabled: true,
            invalid: false,
        }
    }
    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }
    /// `false` draws the switch disabled: no hover, no focus, no toggling.
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

impl Widget for Switch<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::current(ui.ctx());
        let track = [&*t.style.switch];
        let thumb = [&*t.style.switch_thumb];
        let enabled = self.enabled && ui.is_enabled();
        let size_bit = match self.size {
            SwitchSize::Default => State::SIZE_DEFAULT,
            SwitchSize::Sm => State::SIZE_SM,
        };
        // Geometry depends on neither the value nor interaction.
        let g = t.state(size_bit.with(State::INVALID, self.invalid));
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(&track, f, g).unwrap_or(0.0);

        let (rect, base) =
            ui.allocate_exact_size(vec2(get(|r| &r.width), get(|r| &r.height)), Sense::hover());
        // base: "after:absolute after:-inset-x-3 after:-inset-y-2"
        let hit = rect.expand2(vec2(12.0, 8.0));
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut resp = ui.interact(hit, base.id.with("hit"), sense);
        if enabled && resp.clicked() {
            *self.on = !*self.on;
            resp.mark_changed();
        }
        let on = *self.on;
        let context = |checked: bool| {
            size_bit
                .or(if checked {
                    State::CHECKED
                } else {
                    State::UNCHECKED
                })
                .with(State::INVALID, self.invalid)
        };
        let st = interaction_state(&t, &resp, enabled, context(on));
        let how_on = ui.ctx().animate_bool(base.id, on);

        if ui.is_rect_visible(rect) {
            // base: "data-disabled:opacity-50"
            let painter = disabled_painter(ui, self.enabled, 0.5);
            let paint =
                |l: &[&Recipe], f: fn(&Recipe) -> &Prop<Paint>| pick(l, f, st).map(|p| t.paint(p));
            let radius = pick(&track, |r| &r.radius, st)
                .map_or(0.0, |r| t.radius_px(r, rect.height() / 2.0));
            let border_w = pick(&track, |r| &r.border_width, st).unwrap_or(0.0);
            let border = paint(&track, |r| &r.border_color).map(|b| Stroke::new(border_w, b));
            let clip_padding = pick(&track, |r| &r.bg_clip_padding, st) == Some(true);
            let mut shapes = Vec::new();
            if let Some(s) = pick(&track, |r| &r.shadow, st) {
                shapes.push(Shape::from(t.shadow(s).as_shape(rect, radius)));
            }
            shapes.extend(box_shapes(
                rect,
                radius,
                paint(&track, |r| &r.bg),
                border,
                clip_padding,
            ));
            if let Some(c) = paint(&track, |r| &r.ring_color) {
                shapes.extend(ring_shape(
                    rect,
                    radius,
                    pick(&track, |r| &r.ring_width, st).unwrap_or(0.0),
                    c,
                ));
            }

            // The thumb lies inside the border, vertically centred; its
            // translate differs per value, so the animation blends both.
            let side = |f: fn(&Recipe) -> &Prop<f32>| pick(&thumb, f, st).unwrap_or(0.0);
            let size = vec2(side(|r| &r.width), side(|r| &r.height));
            let home =
                Rect::from_min_size(rect.left_center() + vec2(border_w, -size.y / 2.0), size);
            let at = |checked: bool| {
                let s = t.state(context(checked));
                shift(
                    home,
                    pick(&thumb, |r| &r.shift_x, s),
                    pick(&thumb, |r| &r.shift_y, s),
                )
            };
            let (off_rect, on_rect) = (at(false), at(true));
            let thumb_rect = Rect::from_min_size(
                egui::pos2(
                    lerp(off_rect.left()..=on_rect.left(), how_on),
                    off_rect.top(),
                ),
                size,
            );
            let thumb_radius =
                pick(&thumb, |r| &r.radius, st).map_or(0.0, |r| t.radius_px(r, size.y / 2.0));
            if let Some(s) = pick(&thumb, |r| &r.shadow, st) {
                shapes.push(Shape::from(t.shadow(s).as_shape(thumb_rect, thumb_radius)));
            }
            shapes.push(Shape::rect_filled(
                thumb_rect,
                thumb_radius,
                paint(&thumb, |r| &r.bg).unwrap_or_default(),
            ));
            painter.extend(shapes);
        }
        // No label of its own: name it with `labelled_by(label.id)`, which
        // an own label, even an empty one, would override.
        resp.widget_info(|| egui::WidgetInfo {
            enabled,
            selected: Some(on),
            ..egui::WidgetInfo::new(egui::WidgetType::Checkbox)
        });
        resp.rect = rect;
        resp
    }
}

/// Shorthand for `ui.add(Switch::new(on))`.
pub fn toggle(ui: &mut Ui, on: &mut bool) -> egui::Response {
    ui.add(Switch::new(on))
}
