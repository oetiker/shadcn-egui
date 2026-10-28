//! shadcn Input, ported from shadcn-svelte `ui/input/input.svelte` at
//! `tools/stylegen/PIN`. Every value comes from the style's `cn-input`
//! recipe. A frameless egui `TextEdit` sits inside a box this widget
//! paints.

use crate::components::text::Leading;
use crate::components::text_entry::{within, TextEntry};
use crate::style::app::default_variant;
use crate::style::{pick, State};
use crate::{layout, Theme};
use egui::{TextBuffer, TextEdit, TextWrapMode, Ui, Vec2, Widget};

default_variant!(
    /// The input's variant: upstream has none, so only app variants
    /// ([`InputVariant::app`]) differ from `Default`.
    InputVariant,
    /// The input's recipes an app variant's edit changes.
    InputRecipes {
        /// `data-slot="input"`: the field.
        input,
    }
);

pub struct Input<'a> {
    text: &'a mut String,
    hint: String,
    password: bool,
    max_width: Option<f32>,
    id_salt: Option<egui::Id>,
    enabled: bool,
    invalid: bool,
    variant: InputVariant,
}

impl<'a> Input<'a> {
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            hint: String::new(),
            password: false,
            max_width: None,
            id_salt: None,
            enabled: true,
            invalid: false,
            variant: InputVariant::Default,
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
        self.max_width = Some(w);
        self
    }
    /// A stable id, independent of call order, from any hashable salt (a
    /// `&str`, or a `format!("row{i}")` per row). Needed only where a caller
    /// (a test, or `ctx.memory_mut(|m| m.request_focus(..))`) must address
    /// this field from outside the widget tree that builds it; without one,
    /// `TextEdit` falls back to an id keyed on widget position.
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(egui::Id::new(salt));
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
    /// An app variant ([`InputVariant::app`]).
    pub fn variant(mut self, v: impl Into<InputVariant>) -> Self {
        self.variant = v.into();
        self
    }
}

impl<'a> Widget for Input<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::current(ui.ctx());
        let v = t.resolve(self.variant);
        let l = [v.part(&t.style, |_, s| &s.input, |r| &r.input)];
        let context = v.bits().with(State::INVALID, self.invalid);
        // The field's text sits in its line box as in a browser's
        // text-entry field: the leading's half above rounds up.
        let mut frame = TextEntry::resolve(
            ui,
            &t,
            &l,
            context,
            self.enabled,
            Leading::Field,
            "shadcn_egui::input",
        );
        let look = frame.look.clone();
        // No height: CSS's initial `auto`, one line box in the padding box.
        let height = pick(&l, |r| &r.height, frame.geometry)
            .unwrap_or_else(|| look.line_height() + frame.inset[0] + frame.inset[2]);
        // Blink centres the editor line in the content box even when it is
        // taller; egui would clamp it to the content top. Equal cuts from
        // both margins keep the centre and give it the room.
        let content = height - frame.inset[0] - frame.inset[2];
        if let Some(editor) = look.editor_height(ui.ctx()) {
            let cut = ((editor - content) / 2.0).ceil().max(0.0);
            let cut = cut.min(frame.inset[0]).min(frame.inset[2]);
            frame.inset[0] -= cut;
            frame.inset[2] -= cut;
        }

        let password = self.password;
        let mut layouter = |ui: &Ui, text: &dyn TextBuffer, _wrap_width: f32| {
            let text = if password {
                let n = text.as_str().chars().count();
                std::iter::repeat_n(egui::epaint::text::PASSWORD_REPLACEMENT_CHAR, n).collect()
            } else {
                text.as_str().to_owned()
            };
            look.layout(ui, text, TextWrapMode::Extend, f32::INFINITY)
                .galley()
                .clone()
        };
        let max_width = self.max_width.unwrap_or(t.metrics.field_max_width);
        let width = layout::capped_width(ui, max_width, frame.content_min);
        let mut edit = TextEdit::singleline(self.text);
        if !self.hint.is_empty() {
            // The placeholder is laid out in the field's look, cut to the
            // content box as egui cuts its hint text.
            let content = frame.content_width(width);
            let hint = frame.look.layout_colored(
                ui,
                self.hint.as_str(),
                TextWrapMode::Truncate,
                content,
                frame.placeholder,
            );
            edit = edit.hint_text(within(hint, content));
        }
        let edit = edit
            .layouter(&mut layouter)
            .password(password)
            .vertical_align(egui::Align::Center);
        frame
            .show(ui, Vec2::new(width, height), edit, self.id_salt)
            .response
            .response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{AppVariant, Paint, Role};

    /// An app variant's edit gets `InputRecipes` and changes only what it
    /// names.
    #[test]
    fn edit_compiles_and_changes_its_part() {
        const DANGER: AppVariant<InputVariant> = InputVariant::app("danger");
        let t = Theme::vega().variant(
            DANGER,
            InputVariant::Default,
            |r: &mut InputRecipes, _style| r.input.text.set(Paint::Role(Role::Destructive, 1.0)),
        );
        let v = t.resolve(InputVariant::from(DANGER));
        let l = v.part(&t.style, |_, s| &s.input, |r| &r.input);
        assert_eq!(
            l.text.get(State::NONE),
            Some(Paint::Role(Role::Destructive, 1.0))
        );
        assert_eq!(l.bg, t.style.input.bg);
    }
}
