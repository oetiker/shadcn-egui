//! shadcn Input, ported from shadcn-svelte `ui/input/input.svelte` at
//! `tools/stylegen/PIN`. Every value comes from the style's `cn-input`
//! recipe. A frameless egui `TextEdit` sits inside a box this widget
//! paints.

use crate::components::shared::{
    box_shapes, interaction_state, line_box, opacity, ring_shape, style_painter,
};
use crate::style::{pick, Paint, Prop, Recipe, Role, State, TextSize, Weight};
use crate::theme::family_for;
use crate::Theme;
use egui::{Frame, Margin, Shape, Stroke, TextEdit, Ui, Vec2, Widget};

pub struct Input<'a> {
    text: &'a mut String,
    hint: String,
    password: bool,
    max_width: f32,
    id_source: Option<&'static str>,
    enabled: bool,
    invalid: bool,
}

impl<'a> Input<'a> {
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            hint: String::new(),
            password: false,
            max_width: f32::NAN,
            id_source: None,
            enabled: true,
            invalid: false,
        }
    }
    pub fn hint(mut self, h: impl Into<String>) -> Self {
        self.hint = h.into();
        self
    }
    pub fn password(mut self, yes: bool) -> Self {
        self.password = yes;
        self
    }
    /// Override the default maximum width (`Metrics::field_max_width`). Pass
    /// `f32::INFINITY` for full-width.
    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = w;
        self
    }
    /// A stable id, independent of call order. Needed only where a caller (a
    /// test, or `ctx.memory_mut(|m| m.request_focus(..))`) must address this
    /// field from outside the widget tree that builds it; without one,
    /// `TextEdit` falls back to an id keyed on widget position.
    pub fn id_source(mut self, s: &'static str) -> Self {
        self.id_source = Some(s);
        self
    }
    /// `false` draws the field disabled: no focus, no typing.
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

impl<'a> Widget for Input<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::current(ui.ctx());
        let l = [&*t.style.input];
        let enabled = self.enabled && ui.is_enabled();
        let context = State::NONE.with(State::INVALID, self.invalid);
        // Geometry never depends on focus: resolve it before allocating.
        let g = t.state(context);
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(&l, f, g).unwrap_or(0.0);
        let border_w = get(|r| &r.border_width);
        let inset = |pad: f32| (pad + border_w).round() as i8;
        let margin = Margin {
            left: inset(get(|r| &r.pad_left)),
            right: inset(get(|r| &r.pad_right)),
            top: inset(get(|r| &r.pad_top)),
            bottom: inset(get(|r| &r.pad_bottom)),
        };
        let text_size = pick(&l, |r| &r.text_size, g).unwrap_or(TextSize::Sm);
        let text_px = t.text_px(text_size);
        let font = egui::FontId::new(
            text_px,
            family_for(
                ui.ctx(),
                pick(&l, |r| &r.weight, g).unwrap_or(Weight::Normal),
            ),
        );
        // No height: CSS's initial `auto`, one line of text in the padding box.
        let height = pick(&l, |r| &r.height, g).unwrap_or_else(|| {
            line_box(&t, &l, g, text_size).max(ui.ctx().fonts_mut(|f| f.row_height(&font)))
                + f32::from(margin.top)
                + f32::from(margin.bottom)
        });

        let fade = opacity(&[&l[..]], g.with(State::DISABLED, !self.enabled));
        let placeholder = pick(&l, |r| &r.text, g.or(State::PLACEHOLDER)).map(|p| t.paint(p));
        let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground; reads input first
        let typed = pick(&l, |r| &r.text, g).map_or(inherited, |p| t.paint(p));
        let mut edit = TextEdit::singleline(self.text)
            .hint_text(self.hint)
            .password(self.password)
            .interactive(enabled)
            .frame(Frame::NONE.inner_margin(margin))
            .font(font)
            .vertical_align(egui::Align::Center)
            .text_color(typed);
        if let Some(id_source) = self.id_source {
            edit = edit.id_salt(id_source);
        }
        let max_width = if self.max_width.is_nan() {
            t.metrics.field_max_width
        } else {
            self.max_width
        };
        // The box goes under the text: reserve its place in paint order, fill
        // it once the focus state is known.
        let slot = ui.painter().add(Shape::Noop);
        let resp = ui
            .scope(|ui| {
                if let Some(c) = placeholder {
                    ui.visuals_mut().weak_text_color = Some(c);
                }
                // A disabled `Ui` has faded already; see `style_painter`.
                if ui.is_enabled() {
                    ui.multiply_opacity(fade);
                }
                ui.add_sized(Vec2::new(ui.available_width().min(max_width), height), edit)
            })
            .inner;

        let st = interaction_state(&t, &resp, enabled, context);
        let paint = |f: fn(&Recipe) -> &Prop<Paint>| pick(&l, f, st).map(|p| t.paint(p));
        let radius =
            pick(&l, |r| &r.radius, g).map_or(0.0, |r| t.radius_px(r, resp.rect.height() / 2.0));
        let mut shapes = Vec::new();
        if let Some(s) = pick(&l, |r| &r.shadow, st) {
            shapes.push(Shape::from(t.shadow(s).as_shape(resp.rect, radius)));
        }
        let clip_padding = pick(&l, |r| &r.bg_clip_padding, st) == Some(true);
        let border = paint(|r| &r.border_color).map(|b| Stroke::new(border_w, b));
        shapes.extend(box_shapes(
            resp.rect,
            radius,
            paint(|r| &r.bg),
            border,
            clip_padding,
        ));
        if let Some(c) = paint(|r| &r.ring_color) {
            shapes.extend(ring_shape(
                resp.rect,
                radius,
                pick(&l, |r| &r.ring_width, st).unwrap_or(0.0),
                c,
            ));
        }
        style_painter(ui, opacity(&[&l[..]], st)).set(slot, Shape::Vec(shapes));
        resp
    }
}
