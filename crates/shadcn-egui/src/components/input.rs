//! shadcn Input, ported from shadcn-svelte `ui/input/input.svelte` at
//! `tools/stylegen/PIN`. Every value comes from the style's `cn-input`
//! recipe. A frameless egui `TextEdit` sits inside a box this widget
//! paints.

use crate::components::shared::{
    inset_frame, line_width, part_opacity, part_painter, text_entry_state, RecipeBox,
};
use crate::components::text::{Leading, TextLook};
use crate::style::app::default_variant;
use crate::style::{pick, Prop, Recipe, Role, State};
use crate::{layout, Theme};
use egui::{Sense, Shape, TextBuffer, TextEdit, TextWrapMode, Ui, Vec2, Widget};

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
        let enabled = self.enabled && ui.is_enabled();
        let context = v.bits().with(State::INVALID, self.invalid);
        // Geometry never depends on focus: resolve it before allocating.
        let g = t.state(ui.ctx(), context);
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(&l, f, g).unwrap_or(0.0);
        let border_w = line_width(get(|r| &r.border_width), ui.ctx().pixels_per_point());
        // Top, right, bottom, left: padding plus border.
        let mut inset = [
            get(|r| &r.pad_top),
            get(|r| &r.pad_right),
            get(|r| &r.pad_bottom),
            get(|r| &r.pad_left),
        ]
        .map(|pad| pad + border_w);
        // The field's text sits in its line box as in a browser's
        // text-entry field: the leading's half above rounds up.
        let look = TextLook::resolve(ui, &l, g).leading(Leading::Field);
        let font = look.font_id(ui.ctx());
        // No height: CSS's initial `auto`, one line box in the padding box.
        let height =
            pick(&l, |r| &r.height, g).unwrap_or_else(|| look.line_height() + inset[0] + inset[2]);
        // Blink centres the editor line in the content box even when it is
        // taller; egui would clamp it to the content top. Equal cuts from
        // both margins keep the centre and give it the room.
        let content = height - inset[0] - inset[2];
        if let Some(editor) = look.editor_height(ui.ctx()) {
            let cut = ((editor - content) / 2.0).ceil().max(0.0);
            let cut = cut.min(inset[0]).min(inset[2]);
            inset[0] -= cut;
            inset[2] -= cut;
        }

        // The `TextEdit` is the field's own text: it paints at the part's opacity.
        let fade = part_opacity(
            ui,
            ui.next_auto_id().with("shadcn_egui::input"),
            &[&l[..]],
            g.with(State::DISABLED, !enabled),
        );
        let placeholder = pick(&l, |r| &r.text, g.or(State::PLACEHOLDER)).map(|p| t.paint(p));
        let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground; reads input first
        let typed = pick(&l, |r| &r.text, g).map_or(inherited, |p| t.paint(p));
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
        // The content minimum: the padding box around one glyph.
        let content_min = inset[3] + inset[1] + ui.ctx().fonts_mut(|f| f.glyph_width(&font, 'M'));
        // The box goes under the text: reserve its place in paint order, fill
        // it once the focus state is known.
        let slot = ui.painter().add(Shape::Noop);
        let resp = ui
            .scope(|ui| {
                ui.set_opacity(fade);
                // As `add_sized`, with the id made in the `Ui` that shows
                // the field: `TextEdit::id_salt` would hash the stored `Id`
                // again. Raised to its content minimum, the field overflows
                // its room without widening the parent (conformance 3.6).
                let size = Vec2::new(layout::capped_width(ui, max_width, content_min), height);
                let (rect, _) = layout::allocate_overflowing(ui, size, Sense::hover());
                let mut edit = TextEdit::singleline(self.text);
                if !self.hint.is_empty() {
                    // The placeholder is laid out in the field's look, cut
                    // to the content box as egui cuts its hint text.
                    let color = placeholder.unwrap_or_else(|| ui.visuals().weak_text_color());
                    let width = rect.width() - inset[3] - inset[1];
                    let hint = look.layout_colored(
                        ui,
                        self.hint.as_str(),
                        TextWrapMode::Truncate,
                        width,
                        color,
                    );
                    edit = edit.hint_text(hint);
                }
                let edit = edit
                    .layouter(&mut layouter)
                    .password(password)
                    .interactive(enabled)
                    .frame(inset_frame(inset))
                    .font(font.clone())
                    .vertical_align(egui::Align::Center)
                    .text_color(typed);
                let mut field = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(egui::Layout::centered_and_justified(ui.layout().main_dir())),
                );
                let edit = match self.id_salt {
                    Some(salt) => edit.id(field.make_persistent_id(salt)),
                    None => edit,
                };
                // egui's caret spans the row and 1.5 pt beyond it; the
                // field paints Chrome's instead.
                let blink = field.visuals().text_cursor.clone();
                field.visuals_mut().text_cursor.stroke = egui::Stroke::NONE;
                field.visuals_mut().text_cursor.blink = false;
                let out = edit.show(&mut field);
                if enabled {
                    paint_caret(&field, &out, &blink, look.caret_span(field.ctx()), typed);
                }
                out.response.response
            })
            .inner;

        let st = text_entry_state(&t, &resp, context.with(State::DISABLED, !enabled));
        let shapes = RecipeBox::resolve(ui.ctx(), &t, &l, st)
            .transitioned(ui.ctx(), resp.id, &l, st)
            .shapes(ui.ctx(), resp.rect);
        part_painter(ui, resp.id, &[&l[..]], st).set(slot, Shape::Vec(shapes));
        resp
    }
}

/// The caret as Blink paints it in a text field: 1 css px wide, from the
/// font's ascent above the baseline to its descent below (`span`, below the
/// top of the galley's row, the editor line), in the text colour
/// (`caret-color: auto`), snapped to device pixels where the painter puts
/// the galley. Blinks as egui's caret does, from the last edit or cursor
/// move.
fn paint_caret(
    ui: &Ui,
    out: &egui::text_edit::TextEditOutput,
    cursor: &egui::style::TextCursorStyle,
    span: Option<std::ops::Range<f32>>,
    color: egui::Color32,
) {
    let Some(range) = out.cursor_range else {
        return;
    };
    let focused = out.response.response.has_focus() && ui.input(|i| i.focused);
    if !focused {
        return;
    }
    let now = ui.input(|i| i.time);
    let id = out.response.response.id.with("shadcn_egui::input::caret");
    let since = ui.ctx().data_mut(|d| {
        let last = d.get_temp_mut_or_insert_with(id, || (range, now));
        if out.response.response.changed() || last.0 != range {
            *last = (range, now);
        }
        now - last.1
    });
    if cursor.blink {
        let total = cursor.on_duration + cursor.off_duration;
        let phase = (since % f64::from(total)) as f32;
        let (shown, wake) = if phase < cursor.on_duration {
            (true, cursor.on_duration - phase)
        } else {
            (false, total - phase)
        };
        ui.ctx().request_repaint_after_secs(wake);
        if !shown {
            return;
        }
    }
    let ppp = ui.ctx().pixels_per_point();
    let origin = out.galley_pos - egui::vec2(out.galley.rect.left(), 0.0);
    let row = out.galley.pos_from_cursor(range.primary);
    let span = span.unwrap_or(0.0..row.height());
    // The painter rounds the galley origin to the device pixel; the row
    // is whole device pixels tall.
    let top = (origin.y * ppp).round();
    let left = ((origin.x + row.min.x) * ppp).round();
    // Blink's caret: 1 css px wide, at least one device pixel.
    let width = ppp.round().max(1.0); // behaviour-ok: the browser's caret width
    let rect = egui::Rect::from_min_max(
        egui::pos2(left, top + (row.min.y + span.start) * ppp),
        egui::pos2(left + width, top + (row.min.y + span.end) * ppp),
    );
    let rect = egui::Rect::from_min_max(rect.min / ppp, rect.max / ppp);
    // As egui's TextEdit: the clip grows a point so a caret at the edge shows.
    let clip = out.text_clip_rect.expand(1.0); // behaviour-ok: egui's caret clip
    ui.painter_at(clip).rect_filled(rect, 0.0, color);
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
