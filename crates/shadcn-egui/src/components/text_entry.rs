//! The frame every text-entry field shares (Input, Textarea): a frameless
//! egui `TextEdit` inside a box painted from the part's recipe. The frame
//! resolves the box insets, the part's fade, the text look and its typed
//! and placeholder colours, then shows the widget's `TextEdit` in the
//! allocated rect with Chrome's caret and paints the box under it in the
//! field's state. The widget decides only what differs: the box height,
//! how its text lays out, and its hint.

use crate::components::shared::{
    inset_frame, line_width, part_opacity, part_painter, text_entry_state, RecipeBox,
};
use crate::components::text::{Leading, TextLook};
use crate::style::{pick, Prop, Recipe, Role, State};
use crate::{layout, Theme};
use egui::text_edit::TextEditOutput;
use egui::{Color32, FontId, Sense, Shape, TextEdit, Ui, Vec2};

/// A text-entry field's resolved frame. Build it with
/// [`TextEntry::resolve`], size the box from its fields, then
/// [`TextEntry::show`] the widget's `TextEdit` in it.
pub(crate) struct TextEntry<'a> {
    theme: &'a Theme,
    /// The part's recipes, in upstream's class order.
    pub(crate) layers: &'a [&'a Recipe],
    /// The variant's bits and `aria-invalid`.
    context: State,
    /// The state geometry resolves in: never focus, so it is known before
    /// the field is allocated.
    pub(crate) geometry: State,
    pub(crate) enabled: bool,
    /// Top, right, bottom, left: padding plus border.
    pub(crate) inset: [f32; 4],
    pub(crate) look: TextLook,
    pub(crate) font: FontId,
    fade: f32,
    /// The colour of the hint text (`placeholder:`).
    pub(crate) placeholder: Color32,
    /// The colour of the typed text.
    pub(crate) typed: Color32,
    /// The narrowest box: the padding box around one glyph.
    pub(crate) content_min: f32,
}

impl<'a> TextEntry<'a> {
    /// The frame of a field drawn from `layers` in `theme`, in `context`
    /// (the variant's bits, `aria-invalid`), disabled where `ui` is or
    /// `enabled` is false, its text placed in its line box as `leading`
    /// says. `name` keys the part's fade transition.
    pub(crate) fn resolve(
        ui: &Ui,
        theme: &'a Theme,
        layers: &'a [&'a Recipe],
        context: State,
        enabled: bool,
        leading: Leading,
        name: &str,
    ) -> Self {
        let enabled = enabled && ui.is_enabled();
        let geometry = theme.state(ui.ctx(), context);
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(layers, f, geometry).unwrap_or(0.0);
        let border_w = line_width(get(|r| &r.border_width), ui.ctx().pixels_per_point());
        let inset = [
            get(|r| &r.pad_top),
            get(|r| &r.pad_right),
            get(|r| &r.pad_bottom),
            get(|r| &r.pad_left),
        ]
        .map(|pad| pad + border_w);
        let look = TextLook::resolve(ui, layers, geometry).leading(leading);
        let font = look.font_id(ui.ctx());
        // The `TextEdit` is the field's own text: it paints at the part's opacity.
        let fade = part_opacity(
            ui,
            ui.next_auto_id().with(name),
            &[layers],
            geometry.with(State::DISABLED, !enabled),
        );
        // Upstream's text-entry parts all carry
        // `placeholder:text-muted-foreground`; a recipe without a
        // placeholder colour keeps it.
        let muted = theme.role(Role::MutedForeground); // behaviour-ok: upstream's placeholder colour
        let placeholder = pick(layers, |r| &r.text, geometry.or(State::PLACEHOLDER))
            .map_or(muted, |p| theme.paint(p));
        let inherited = theme.role(Role::Foreground); // behaviour-ok: inherits Foreground; reads the part first
        let typed = pick(layers, |r| &r.text, geometry).map_or(inherited, |p| theme.paint(p));
        let content_min = inset[3] + inset[1] + ui.ctx().fonts_mut(|f| f.glyph_width(&font, 'M'));
        Self {
            theme,
            layers,
            context,
            geometry,
            enabled,
            inset,
            look,
            font,
            fade,
            placeholder,
            typed,
            content_min,
        }
    }

    /// The recipe's `f` in the geometry state, 0 where it sets none.
    pub(crate) fn get(&self, f: fn(&Recipe) -> &Prop<f32>) -> f32 {
        pick(self.layers, f, self.geometry).unwrap_or(0.0)
    }

    /// The width between the side insets of a box `width` wide.
    pub(crate) fn content_width(&self, width: f32) -> f32 {
        width - self.inset[3] - self.inset[1]
    }

    /// Allocate the box at `size`, show `edit` in it (`id_salt` gives it
    /// a stable id), and paint the box under it, at the allocated rect, in
    /// the field's state.
    /// Raised to its content minimum, the field overflows its room without
    /// widening the parent (conformance 3.6).
    pub(crate) fn show(
        self,
        ui: &mut Ui,
        size: Vec2,
        edit: TextEdit<'_>,
        id_salt: Option<egui::Id>,
    ) -> TextEditOutput {
        let enabled = self.enabled;
        // The box goes under the text: reserve its place in paint order, fill
        // it once the focus state is known.
        let slot = ui.painter().add(Shape::Noop);
        let (rect, out) = ui
            .scope(|ui| {
                ui.set_opacity(self.fade);
                // As `add_sized`, with the id made in the `Ui` that shows
                // the field: `TextEdit::id_salt` would hash the stored `Id`
                // again.
                let (rect, _) = layout::allocate_overflowing(ui, size, Sense::hover());
                let edit = edit
                    .interactive(enabled)
                    .frame(inset_frame(self.inset))
                    .font(self.font.clone())
                    .text_color(self.typed);
                let mut field = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect)
                        .layout(egui::Layout::centered_and_justified(ui.layout().main_dir())),
                );
                let edit = match id_salt {
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
                    paint_caret(
                        &field,
                        &out,
                        &blink,
                        self.look.caret_span(field.ctx()),
                        self.typed,
                    );
                }
                (rect, out)
            })
            .inner;

        let resp = &out.response.response;
        let st = text_entry_state(
            self.theme,
            resp,
            self.context.with(State::DISABLED, !enabled),
        );
        let l = self.layers;
        let shapes = RecipeBox::resolve(ui.ctx(), self.theme, l, st)
            .transitioned(ui.ctx(), resp.id, l, st)
            .shapes(ui.ctx(), rect);
        part_painter(ui, resp.id, &[l], st).set(slot, Shape::Vec(shapes));
        out
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
    out: &TextEditOutput,
    cursor: &egui::style::TextCursorStyle,
    span: Option<std::ops::Range<f32>>,
    color: Color32,
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
