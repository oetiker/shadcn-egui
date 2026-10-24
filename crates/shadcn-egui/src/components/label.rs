//! shadcn Label, ported from shadcn-svelte `ui/label/label.svelte` at
//! `tools/stylegen/PIN`. The label has no base classes that matter in egui
//! (`flex items-center select-none` and the pointer/cursor rules); its look
//! comes from the style's `cn-label` recipe. `peer-disabled:opacity-50`
//! arrives as the DISABLED state of that recipe.
//!
//! `description` has no marker of its own in shadcn: it is the muted `text-sm`
//! paragraph that `cn-card-description` and `cn-dialog-description` share, so
//! it reads the `card_description` recipe.

use crate::components::shared::part_painter;
use crate::components::text::TextLook;
use crate::style::{pick, Role, State};
use crate::Theme;
use egui::{Response, Sense, TextWrapMode, Ui, Widget};

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
        let st = t.state(ui.ctx(), State::NONE.with(State::DISABLED, !enabled));
        let look = TextLook::resolve(ui, &l, st);
        let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground; reads label first
        let fg = pick(&l, |r| &r.text, st).map_or(inherited, |p| t.paint(p));
        let painter = part_painter(ui, &[&l[..]], st);

        // The label is as tall as its line box (`leading-none`: the font
        // size).
        let block = look.layout(ui, self.text.as_str(), TextWrapMode::Extend, f32::INFINITY);
        let (rect, resp) = ui.allocate_exact_size(block.size(), Sense::hover());
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, enabled, &self.text)
        });
        if ui.is_rect_visible(rect) {
            block.paint(&painter, rect.min, fg);
        }
        resp
    }
}

/// Muted `text-sm` helper text, wrapping like a paragraph.
pub fn description(ui: &mut Ui, text: &str) -> Response {
    let t = Theme::current(ui.ctx());
    let l = [&*t.style.card_description];
    let st = t.state(ui.ctx(), State::NONE);
    let look = TextLook::resolve(ui, &l, st);
    let muted = t.role(Role::MutedForeground); // behaviour-ok: inherits; reads card_description first
    let look = match look.recipe_color() {
        Some(_) => look,
        None => look.with_color(muted),
    };
    look.label(ui, text, TextWrapMode::Wrap)
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
