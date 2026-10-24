//! shadcn Input, ported from shadcn-svelte `ui/input/input.svelte` at
//! `tools/stylegen/PIN`. Every value comes from the style's `cn-input`
//! recipe. A frameless egui `TextEdit` sits inside a box this widget
//! paints.

use crate::components::shared::{part_opacity, part_painter, text_entry_state, RecipeBox};
use crate::components::text::{Leading, TextLook};
use crate::style::{pick, Prop, Recipe, Role, State};
use crate::{layout, Theme};
use egui::{Frame, Margin, Sense, Shape, TextBuffer, TextEdit, TextWrapMode, Ui, Vec2, Widget};

pub struct Input<'a> {
    text: &'a mut String,
    hint: String,
    password: bool,
    max_width: Option<f32>,
    id_salt: Option<egui::Id>,
    enabled: bool,
    invalid: bool,
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
}

impl<'a> Widget for Input<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::current(ui.ctx());
        let l = [&*t.style.input];
        let enabled = self.enabled && ui.is_enabled();
        let context = State::NONE.with(State::INVALID, self.invalid);
        // Geometry never depends on focus: resolve it before allocating.
        let g = t.state(ui.ctx(), context);
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(&l, f, g).unwrap_or(0.0);
        let border_w = get(|r| &r.border_width);
        let inset = |pad: f32| (pad + border_w).round() as i8;
        let margin = Margin {
            left: inset(get(|r| &r.pad_left)),
            right: inset(get(|r| &r.pad_right)),
            top: inset(get(|r| &r.pad_top)),
            bottom: inset(get(|r| &r.pad_bottom)),
        };
        // The field's text sits in its line box as in a browser's
        // text-entry field: the leading's half above rounds up.
        let look = TextLook::resolve(ui, &l, g).leading(Leading::Field);
        let font = look.font_id(ui.ctx());
        // No height: CSS's initial `auto`, one line box in the padding box.
        let height = pick(&l, |r| &r.height, g).unwrap_or_else(|| {
            look.line_height() + f32::from(margin.top) + f32::from(margin.bottom)
        });

        // The `TextEdit` is the field's own text: it paints at the part's opacity.
        let fade = part_opacity(ui, &[&l[..]], g.with(State::DISABLED, !enabled));
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
        let content_min = f32::from(margin.left)
            + f32::from(margin.right)
            + ui.ctx().fonts_mut(|f| f.glyph_width(&font, 'M'));
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
                    let width = rect.width() - f32::from(margin.left) - f32::from(margin.right);
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
                    .frame(Frame::NONE.inner_margin(margin))
                    .font(font.clone())
                    .vertical_align(egui::Align::Center)
                    .text_color(typed);
                let mut field = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(egui::Layout::centered_and_justified(ui.layout().main_dir())),
                );
                match self.id_salt {
                    Some(salt) => field.add(edit.id(field.make_persistent_id(salt))),
                    None => field.add(edit),
                }
            })
            .inner;

        let st = text_entry_state(&t, &resp, context.with(State::DISABLED, !enabled));
        let shapes = RecipeBox::resolve(&t, &l, st).shapes(ui.ctx(), resp.rect);
        part_painter(ui, &[&l[..]], st).set(slot, Shape::Vec(shapes));
        resp
    }
}
