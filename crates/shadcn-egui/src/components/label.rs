//! shadcn Label, ported from shadcn-svelte `ui/label/label.svelte` at
//! `tools/stylegen/PIN`. The label has no base classes that matter in egui
//! (`flex items-center select-none` and the pointer/cursor rules); its look
//! comes from the style's `cn-label` recipe. `peer-disabled:opacity-50`
//! arrives as the DISABLED state of that recipe.
//!
//! `description` has no marker of its own in shadcn: it is the muted `text-sm`
//! paragraph that `cn-card-description` and `cn-dialog-description` share, so
//! it reads the `card_description` recipe.

use crate::components::shared::disabled_painter;
use crate::style::{pick, Prop, Recipe, Role, State, TextSize, Weight};
use crate::theme::family_for;
use crate::Theme;
use egui::{Response, RichText, Sense, Ui, Vec2, Widget};

pub struct Label {
    text: String,
    enabled: bool,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            enabled: true,
        }
    }
    /// `false` draws the label as the label of a disabled peer control.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

pub fn label(ui: &mut Ui, text: &str) -> Response {
    ui.add(Label::new(text))
}

impl Widget for Label {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let l = [&*t.style.label];
        let enabled = self.enabled && ui.is_enabled();
        let st = t.state(State::NONE.with(State::DISABLED, !enabled));
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(&l, f, st);
        let size = pick(&l, |r| &r.text_size, st).unwrap_or(TextSize::Sm);
        let text_px = t.text_px(size);
        let weight = pick(&l, |r| &r.weight, st).unwrap_or(Weight::Normal);
        let font = egui::FontId::new(text_px, family_for(ui.ctx(), weight));
        let fg = t.role(Role::Foreground);
        let painter = disabled_painter(ui, self.enabled, get(|r| &r.opacity).unwrap_or(1.0));

        let galley = painter.layout_no_wrap(self.text.clone(), font, fg);
        // `leading-none`: the line box is `line_height` times the font size;
        // without one, Tailwind's line height of the text size.
        let height = get(|r| &r.line_height).map_or_else(|| t.line_px(size), |k| k * text_px);
        let (rect, resp) =
            ui.allocate_exact_size(Vec2::new(galley.size().x, height), Sense::hover());
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, enabled, &self.text)
        });
        if ui.is_rect_visible(rect) {
            let pos = egui::pos2(rect.min.x, rect.center().y - galley.size().y / 2.0);
            painter.galley(pos, galley, fg);
        }
        resp
    }
}

/// Muted `text-sm` helper text, wrapping like a paragraph.
pub fn description(ui: &mut Ui, text: &str) -> Response {
    let t = Theme::current(ui.ctx());
    let l = [&*t.style.card_description];
    let st = t.state(State::NONE);
    let size = pick(&l, |r| &r.text_size, st).unwrap_or(TextSize::Sm);
    let px = t.text_px(size);
    // Tailwind's `text-*` sets a line height too; `leading-*` overrides it.
    let line = pick(&l, |r| &r.line_height, st).map_or_else(|| t.line_px(size), |k| k * px);
    let color =
        pick(&l, |r| &r.text, st).map_or_else(|| t.role(Role::MutedForeground), |p| t.paint(p));
    ui.label(
        RichText::new(text)
            .size(px)
            .line_height(Some(line))
            .color(color),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{pick, Style};

    #[test]
    fn disabled_label_recipe_is_half_opacity() {
        for make in [Style::vega as fn() -> Style, Style::nova] {
            let s = make();
            let l = [&*s.label];
            assert_eq!(pick(&l, |r| &r.opacity, State::DISABLED), Some(0.5));
            assert_eq!(pick(&l, |r| &r.opacity, State::NONE), None);
        }
    }
}
