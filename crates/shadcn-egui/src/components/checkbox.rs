//! shadcn Checkbox, ported from shadcn-svelte `ui/checkbox/checkbox.svelte`
//! at `tools/stylegen/PIN`. Every value, the hit area and the disabled
//! fade included, comes from the style's `cn-checkbox` and
//! `cn-checkbox-indicator` recipes. The tick is the theme's `check` icon
//! ([`crate::Theme::icons`]).
//!
//! The indeterminate state of the upstream component is not offered: the
//! value is a plain `bool`.

use crate::components::icon::paint_icon;
use crate::components::shared::{box_shapes, interaction_state, ring_shape, style_painter};
use crate::style::{pick, Paint, Prop, Recipe, State};
use crate::Theme;
use egui::{vec2, Rect, Sense, Shape, Stroke, Ui, Vec2, Widget};

/// A checkbox. `checkbox(ui, &mut on)` is shorthand for
/// `ui.add(Checkbox::new(&mut on))`.
///
/// The returned response's rect is the visible box. Its click target is
/// larger (the recipe's `hit_inset_x` and `hit_inset_y`), and the
/// accessibility node carries that larger rect.
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    enabled: bool,
    invalid: bool,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool) -> Self {
        Self {
            checked,
            enabled: true,
            invalid: false,
        }
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
        let l = [&*t.style.checkbox];
        let enabled = self.enabled && ui.is_enabled();
        // Geometry depends on neither the value nor interaction.
        let g = t.state(State::NONE.with(State::INVALID, self.invalid));
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(&l, f, g).unwrap_or(0.0);

        // No size: CSS's initial `auto`, the indicator's tick inside the
        // border. The box keeps that size unticked too, so it does not jump.
        let ind = [&*t.style.checkbox_indicator];
        let side = pick(&ind, |r| &r.icon_size, g).unwrap_or(t.metrics.icon);
        let auto = side + get(|r| &r.border_width) * 2.0;
        let size = vec2(
            pick(&l, |r| &r.width, g).unwrap_or(auto),
            pick(&l, |r| &r.height, g).unwrap_or(auto),
        );
        let (rect, base) = ui.allocate_exact_size(size, Sense::hover());
        let hit = rect.expand2(vec2(get(|r| &r.hit_inset_x), get(|r| &r.hit_inset_y)));
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut resp = ui.interact(hit, base.id.with("hit"), sense);
        if enabled && resp.clicked() {
            *self.checked = !*self.checked;
            resp.mark_changed();
        }
        let checked = *self.checked;
        let context = (if checked {
            State::CHECKED
        } else {
            State::UNCHECKED
        })
        .with(State::INVALID, self.invalid);
        let st = interaction_state(&t, &resp, enabled, context);

        if ui.is_rect_visible(rect) {
            let painter = style_painter(ui, &[&l[..]], st);
            let paint = |f: fn(&Recipe) -> &Prop<Paint>| pick(&l, f, st).map(|p| t.paint(p));
            let radius =
                pick(&l, |r| &r.radius, st).map_or(0.0, |r| t.radius_px(r, rect.height() / 2.0));
            let mut shapes = Vec::new();
            if let Some(s) = pick(&l, |r| &r.shadow, st) {
                shapes.push(Shape::from(t.shadow(s).as_shape(rect, radius)));
            }
            let border_w = pick(&l, |r| &r.border_width, st).unwrap_or(0.0);
            let border = paint(|r| &r.border_color).map(|b| Stroke::new(border_w, b));
            let clip_padding = pick(&l, |r| &r.bg_clip_padding, st) == Some(true);
            shapes.extend(box_shapes(
                rect,
                radius,
                paint(|r| &r.bg),
                border,
                clip_padding,
            ));
            if let Some(c) = paint(|r| &r.ring_color) {
                shapes.extend(ring_shape(
                    rect,
                    radius,
                    pick(&l, |r| &r.ring_width, st).unwrap_or(0.0),
                    c,
                ));
            }
            painter.extend(shapes);
            if checked {
                let side = pick(&ind, |r| &r.icon_size, st).unwrap_or(t.metrics.icon);
                // `text-current`: the tick inherits the box's text colour.
                let tick = pick(&ind, |r| &r.text, st).or_else(|| pick(&l, |r| &r.text, st));
                if let Some(color) = tick.map(|p| t.paint(p)) {
                    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(side));
                    // The indicator sits in the box: both opacities apply.
                    paint_icon(
                        &style_painter(ui, &[&ind[..], &l[..]], st),
                        icon_rect,
                        &t.icons.check,
                        color,
                        t.metrics.icon_stroke,
                    );
                }
            }
        }
        // No label of its own: name it with `labelled_by(label.id)`, which
        // an own label, even an empty one, would override.
        resp.widget_info(|| egui::WidgetInfo {
            enabled,
            selected: Some(checked),
            ..egui::WidgetInfo::new(egui::WidgetType::Checkbox)
        });
        resp.rect = rect;
        resp
    }
}

/// Shorthand for `ui.add(Checkbox::new(checked))`.
pub fn checkbox(ui: &mut Ui, checked: &mut bool) -> egui::Response {
    ui.add(Checkbox::new(checked))
}
