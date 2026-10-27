//! shadcn Textarea, ported from shadcn-svelte `ui/textarea/textarea.svelte`
//! at `tools/stylegen/PIN`. Every value comes from the style's
//! `cn-textarea` recipe. A frameless egui `TextEdit` sits inside a box this
//! widget paints.
//!
//! The field fills the `Ui`'s width up to `Metrics::field_max_width`
//! (`.desired_width` changes the cap) and grows with its text from the
//! recipe's minimum height (`min-h-16`), as upstream's
//! `field-sizing-content` does; an empty field grows with its hint text.
//! The user cannot resize it. Text wraps at the field's width; Enter starts
//! a new line and Tab moves the focus on, as in the browser. A disabled
//! `Ui` (`ui.add_enabled`) disables the field.
//!
//! ```
//! use shadcn_egui::components::textarea::{textarea, Textarea};
//!
//! fn message(ui: &mut egui::Ui, text: &mut String) {
//!     ui.add(Textarea::new(text).hint_text("Type your message here."));
//!     if textarea(ui, text).changed() {
//!         // the text changed
//!     }
//! }
//! ```

use crate::components::icon_atom::retint;
use crate::components::input::paint_caret;
use crate::components::shared::{
    inset_frame, line_width, part_opacity, part_painter, text_entry_state, RecipeBox,
};
use crate::components::text::TextLook;
use crate::style::app::default_variant;
use crate::style::{pick, Prop, Recipe, Role, State};
use crate::{layout, Theme};
use egui::text_edit::TextEditOutput;
use egui::{
    AtomKind, Atoms, IntoAtoms, Response, Sense, Shape, TextBuffer, TextEdit, TextWrapMode, Ui,
    Vec2, Widget, WidgetText,
};

default_variant!(
    /// The textarea's variant: upstream has none, so only app variants
    /// ([`TextareaVariant::app`]) differ from `Default`.
    TextareaVariant,
    /// The textarea's recipes an app variant's edit changes.
    TextareaRecipes {
        /// `data-slot="textarea"`: the field.
        textarea,
    }
);

/// A multi-line text field bound to `text`, as egui's
/// `TextEdit::multiline`. Place it with `ui.add`, or with
/// [`Textarea::show`] for the `TextEditOutput` (cursor, galley).
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Textarea<'t> {
    text: &'t mut dyn TextBuffer,
    hint_text: Atoms<'static>,
    desired_width: Option<f32>,
    id_salt: Option<egui::Id>,
    invalid: bool,
    variant: TextareaVariant,
}

impl<'t> Textarea<'t> {
    /// A field that edits `text`: a `&mut String`, or a `&mut &str` that
    /// shows its text and takes no typing.
    #[inline]
    pub fn new(text: &'t mut dyn TextBuffer) -> Self {
        Self {
            text,
            hint_text: Atoms::default(),
            desired_width: None,
            id_salt: None,
            invalid: false,
            variant: TextareaVariant::Default,
        }
    }

    /// Shown while the field is empty, in the style's placeholder colour,
    /// wrapped at the field's width; the empty field grows to hold it.
    /// Text atoms keep the attributes the caller set on them.
    #[doc(alias = "placeholder")]
    #[inline]
    pub fn hint_text(mut self, hint_text: impl IntoAtoms<'static>) -> Self {
        self.hint_text = hint_text.into_atoms();
        self
    }

    /// The width the field asks for, in points, capped to the available
    /// width (default `Metrics::field_max_width`). `f32::INFINITY` fills
    /// the `Ui`'s width.
    #[inline]
    pub fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
    }

    /// A stable id, independent of call order, from any hashable salt, as
    /// `TextEdit::id_salt`. Needed only where a caller must address this
    /// field from outside the widget tree that builds it.
    #[inline]
    pub fn id_salt(mut self, id_salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(egui::Id::new(id_salt));
        self
    }

    /// shadcn `aria-invalid`: destructive border and ring.
    #[inline]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// An app variant ([`TextareaVariant::app`]).
    #[inline]
    pub fn variant(mut self, variant: impl Into<TextareaVariant>) -> Self {
        self.variant = variant.into();
        self
    }

    /// Show the field and return egui's `TextEditOutput` (its `response`,
    /// the laid-out text and the cursor).
    pub fn show(self, ui: &mut Ui) -> TextEditOutput {
        let t = Theme::current(ui.ctx());
        let v = t.resolve(self.variant);
        let l = [v.part(&t.style, |_, s| &s.textarea, |r| &r.textarea)];
        let enabled = ui.is_enabled();
        let context = v.bits().with(State::INVALID, self.invalid);
        // Geometry never depends on focus: resolve it before allocating.
        let g = t.state(ui.ctx(), context);
        debug_assert!(
            pick(&l, |r| &r.field_sizing_content, g).unwrap_or(false),
            "Textarea grows with its content: a recipe without field_sizing_content \
             would need a scrolling field, which the crate does not draw"
        );
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(&l, f, g).unwrap_or(0.0);
        let border_w = line_width(get(|r| &r.border_width), ui.ctx().pixels_per_point());
        // Top, right, bottom, left: padding plus border.
        let inset = [
            get(|r| &r.pad_top),
            get(|r| &r.pad_right),
            get(|r| &r.pad_bottom),
            get(|r| &r.pad_left),
        ]
        .map(|pad| pad + border_w);
        // A `<textarea>`'s rows are line boxes, placed as inline text.
        let look = TextLook::resolve(ui, &l, g);
        let font = look.font_id(ui.ctx());

        let fade = part_opacity(
            ui,
            ui.next_auto_id().with("shadcn_egui::textarea"),
            &[&l[..]],
            g.with(State::DISABLED, !enabled),
        );
        let placeholder = pick(&l, |r| &r.text, g.or(State::PLACEHOLDER)).map(|p| t.paint(p));
        let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground; reads textarea first
        let typed = pick(&l, |r| &r.text, g).map_or(inherited, |p| t.paint(p));

        let max_width = self.desired_width.unwrap_or(t.metrics.field_max_width);
        // The content minimum: the padding box around one glyph.
        let content_min = inset[3] + inset[1] + ui.ctx().fonts_mut(|f| f.glyph_width(&font, 'M'));
        let width = layout::capped_width(ui, max_width, content_min);
        let wrap = width - inset[3] - inset[1];

        // `field-sizing-content`: the box holds its text, or its hint text
        // while empty, inside the recipe's height bounds.
        let text_height = look
            .layout(ui, self.text.as_str(), TextWrapMode::Wrap, wrap)
            .size()
            .y;
        let mut hint = self.hint_text;
        let mut hint_height = 0.0_f32;
        if self.text.as_str().is_empty() {
            let color = placeholder.unwrap_or_else(|| ui.visuals().weak_text_color());
            hint.map_kind(|kind| match kind {
                AtomKind::Text(text) => {
                    let galley = look.layout_colored(ui, text, TextWrapMode::Wrap, wrap, color);
                    hint_height = hint_height.max(galley.size().y);
                    AtomKind::Text(WidgetText::Galley(galley))
                }
                AtomKind::Image(image) => AtomKind::Image(retint(image, color)),
                other => other,
            });
        }
        let content = text_height.max(hint_height) + inset[0] + inset[2];
        let height = pick(&l, |r| &r.height, g)
            .unwrap_or(content)
            .max(get(|r| &r.min_height));

        let mut layouter = |ui: &Ui, text: &dyn TextBuffer, wrap_width: f32| {
            look.layout(ui, text.as_str(), TextWrapMode::Wrap, wrap_width)
                .galley()
                .clone()
        };
        // The box goes under the text: reserve its place in paint order, fill
        // it once the focus state is known.
        let slot = ui.painter().add(Shape::Noop);
        let out = ui
            .scope(|ui| {
                ui.set_opacity(fade);
                // Raised to its content minimum, the field overflows its
                // room without widening the parent (conformance 3.6).
                let (rect, _) =
                    layout::allocate_overflowing(ui, Vec2::new(width, height), Sense::hover());
                let mut edit = TextEdit::multiline(self.text);
                if !hint.is_empty() {
                    edit = edit.hint_text(hint);
                }
                let edit = edit
                    .layouter(&mut layouter)
                    .interactive(enabled)
                    .frame(inset_frame(inset))
                    .font(font.clone())
                    .desired_width(f32::INFINITY)
                    .desired_rows(1)
                    .vertical_align(egui::Align::Min)
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
                out
            })
            .inner;

        let resp = &out.response.response;
        let st = text_entry_state(&t, resp, context.with(State::DISABLED, !enabled));
        let shapes = RecipeBox::resolve(ui.ctx(), &t, &l, st)
            .transitioned(ui.ctx(), resp.id, &l, st)
            .shapes(ui.ctx(), resp.rect);
        part_painter(ui, resp.id, &[&l[..]], st).set(slot, Shape::Vec(shapes));
        out
    }
}

impl Widget for Textarea<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response.response
    }
}

/// A [`Textarea`] bound to `text`: `ui.add(Textarea::new(text))`.
pub fn textarea(ui: &mut Ui, text: &mut dyn TextBuffer) -> Response {
    ui.add(Textarea::new(text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{AppVariant, Paint, Role};

    /// An app variant's edit gets `TextareaRecipes` and changes only what
    /// it names.
    #[test]
    fn edit_compiles_and_changes_its_part() {
        const DANGER: AppVariant<TextareaVariant> = TextareaVariant::app("danger");
        let t = Theme::vega().variant(
            DANGER,
            TextareaVariant::Default,
            |r: &mut TextareaRecipes, _style| {
                r.textarea.text.set(Paint::Role(Role::Destructive, 1.0))
            },
        );
        let v = t.resolve(TextareaVariant::from(DANGER));
        let l = v.part(&t.style, |_, s| &s.textarea, |r| &r.textarea);
        assert_eq!(
            l.text.get(State::NONE),
            Some(Paint::Role(Role::Destructive, 1.0))
        );
        assert_eq!(l.bg, t.style.textarea.bg);
    }
}
