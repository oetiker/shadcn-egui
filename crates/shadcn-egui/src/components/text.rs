//! The text path: the one way crate-drawn text resolves its look and sits
//! in its line box (conformance 2.3, 2.4).
//!
//! Each attribute of a piece of crate text resolves in one order: what the
//! caller set on it explicitly (a `RichText` size, family, colour, line
//! height, underline), else the part's recipe, else what the enclosing
//! `Ui` gives: for colour its text colour (CSS's `currentColor`), for size
//! its `TextStyle::Body` size, for the line height the factor a crate
//! container set on its content (see [`content_scope`]). egui's `strong`
//! and `weak` are no explicit colours: such text takes the recipe colour.
//!
//! Text sits in its line box as Blink places it: the font's ascent A and
//! descent D, read from the font's own tables, each rounded to whole
//! device pixels, the baseline `floor((line height - (A + D)) / 2) + A`
//! below the line top, the font's line gap unused. A text-entry field
//! places it as Blink places its editor (see [`Leading`]). Every row of
//! wrapped text follows the same rule, and the laid-out text is as tall as
//! its line boxes.
//!
//! ```
//! use shadcn_egui::components::text::TextLook;
//! use shadcn_egui::style::State;
//! use shadcn_egui::Theme;
//!
//! fn title(ui: &mut egui::Ui, text: &str) -> egui::Response {
//!     let theme = Theme::current(ui.ctx());
//!     let layers = [&*theme.style.card_title];
//!     let look = TextLook::resolve(ui, &layers, theme.state(ui.ctx(), State::NONE));
//!     let block = look.layout(ui, text, egui::TextWrapMode::Wrap, ui.available_width());
//!     let (rect, resp) = ui.allocate_exact_size(block.size(), egui::Sense::hover());
//!     block.paint(ui.painter(), rect.min, look.color(ui));
//!     resp
//! }
//! ```

use crate::style::{pick, Recipe, State, TextSize, Weight};
use crate::theme::{family_for, heading_family_for};
use crate::Theme;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Align, Color32, FontFamily, FontId, FontSelection, Galley, Painter, Pos2, Response, Stroke,
    TextWrapMode, Ui, Vec2, WidgetText,
};
use std::sync::Arc;

/// The UiStack tag a crate container puts on its content `Ui` to pass the
/// line-height factor (`leading-*`) to crate text inside, which egui
/// widgets cannot take.
const LINE_FACTOR_TAG: &str = "shadcn_egui::text::line_factor";

/// How a line box's leading (its height minus the font's rounded ascent
/// and descent) splits above and below the text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Leading {
    /// Inline text, as Blink's `CalculateLeadingSpace`: the half above is
    /// rounded down to a device pixel.
    #[default]
    Inline,
    /// The inner editor of a text `<input>`: Blink lays it out as one line
    /// of `line-height: normal` (ascent, descent and the font's line gap,
    /// each rounded to device pixels) centred in the content box, an odd
    /// pixel of slack below the editor's top half, the baseline half the
    /// line gap (rounded down) plus the ascent below that top. Placed in a
    /// line box centred in the same content box (the content box minus
    /// the line box an even number of device pixels), the half above is
    /// `ceil((line height - (A + D + gap)) / 2) + floor(gap / 2)`; without a
    /// line gap that is the inline half rounded up.
    Field,
}

/// The resolved text attributes of one part: its recipe stack over what
/// the enclosing `Ui` gives. Build it with [`TextLook::resolve`], lay text
/// out with [`TextLook::layout`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextLook {
    size: f32,
    line_height: f32,
    weight: Weight,
    heading: bool,
    color: Option<Color32>,
    tracking: f32,
    uppercase: bool,
    underline: bool,
    underline_width: f32,
    leading: Leading,
}

impl TextLook {
    /// The text look of the part whose recipes are `layers` (in upstream's
    /// class order) under `state`. What the recipes leave out comes from
    /// `ui`: the size from its `TextStyle::Body`, the line height from the
    /// factor of an enclosing [`content_scope`] (else CSS's 1.5 of the
    /// base size), the colour from its text colour at paint time.
    pub fn resolve(ui: &Ui, layers: &[&Recipe], state: State) -> Self {
        let theme = Theme::current(ui.ctx());
        let recipe_size = pick(layers, |r| &r.text_size, state);
        let size = recipe_size.map_or_else(|| body_size(ui), |s| theme.text_px(s));
        let line_height = match (pick(layers, |r| &r.line_height, state), recipe_size) {
            (Some(k), _) => k * size,
            // Tailwind's `text-*` sets a line height too.
            (None, Some(s)) => theme.line_px(s),
            (None, None) => inherited_line_factor(ui)
                .map_or_else(|| theme.line_px(TextSize::Px(size)), |k| k * size),
        };
        Self {
            size,
            line_height,
            weight: pick(layers, |r| &r.weight, state).unwrap_or(Weight::Normal),
            heading: false,
            color: pick(layers, |r| &r.text, state).map(|p| theme.paint(p)),
            tracking: pick(layers, |r| &r.tracking, state).unwrap_or(0.0),
            uppercase: pick(layers, |r| &r.uppercase, state) == Some(true),
            underline: pick(layers, |r| &r.underline, state) == Some(true),
            underline_width: theme.metrics.border,
            leading: Leading::Inline,
        }
    }

    /// `cn-font-heading`: the theme's heading font instead of the body
    /// font.
    #[inline]
    pub fn heading(mut self, heading: bool) -> Self {
        self.heading = heading;
        self
    }

    /// How the leading splits around the text; [`Leading::Field`] for a
    /// text-entry field.
    #[inline]
    pub fn leading(mut self, leading: Leading) -> Self {
        self.leading = leading;
        self
    }

    /// The colour of text without an explicit colour, in place of the
    /// recipe's (a part whose colour comes from another part's state).
    #[inline]
    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    /// Whether the text is underlined (`underline`), as a part whose
    /// underline follows its state lays it out again when it changes.
    #[inline]
    pub fn underlined(&self) -> bool {
        self.underline
    }

    /// The font size in px.
    #[inline]
    pub fn size(&self) -> f32 {
        self.size
    }

    /// The line box height in px.
    #[inline]
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// The font of text without an explicit size or family.
    pub fn font_id(&self, ctx: &egui::Context) -> FontId {
        let family = if self.heading {
            heading_family_for(ctx, self.weight)
        } else {
            family_for(ctx, self.weight)
        };
        FontId::new(self.size, family)
    }

    /// The colour of text without an explicit colour: the recipe's, else
    /// `ui`'s text colour.
    pub fn color(&self, ui: &Ui) -> Color32 {
        self.color.unwrap_or_else(|| ui.visuals().text_color())
    }

    /// The recipe's colour alone, if it sets one.
    #[inline]
    pub fn recipe_color(&self) -> Option<Color32> {
        self.color
    }

    /// `text` as a layout job: the caller's explicit attributes kept, the
    /// recipe's filled in where the caller set none. Text without an
    /// explicit colour (`strong` and `weak` text included) is
    /// `Color32::PLACEHOLDER`, which the painter replaces with the colour
    /// passed at paint time. Sections without an explicit line height get
    /// the look's.
    fn job(&self, ui: &Ui, text: WidgetText) -> LayoutJob {
        let ctx = ui.ctx();
        let font = self.font_id(ctx);
        let mut job = match text {
            WidgetText::Text(s) => {
                LayoutJob::simple_format(s, egui::TextFormat::simple(font, Color32::PLACEHOLDER))
            }
            WidgetText::RichText(rich) => {
                let style = placeholder_style(ctx);
                WidgetText::RichText(rich)
                    .into_layout_job(&style, FontSelection::FontId(font), Align::BOTTOM)
                    .as_ref()
                    .clone()
            }
            WidgetText::LayoutJob(job) => job.as_ref().clone(),
            WidgetText::Galley(galley) => galley.job.as_ref().clone(),
        };
        if self.uppercase {
            uppercase(&mut job);
        }
        for section in &mut job.sections {
            let f = &mut section.format;
            if f.line_height.is_none() {
                f.line_height = Some(self.line_height);
            }
            if f.extra_letter_spacing == 0.0 {
                f.extra_letter_spacing = self.tracking * f.font_id.size;
            }
            if self.underline && f.underline == Stroke::NONE {
                // The underline takes the colour of its own run.
                f.underline = Stroke::new(self.underline_width, f.color);
            }
        }
        job
    }

    /// Lay `text` out in this look. `wrap` and `width` say where it wraps
    /// (`TextWrapMode::Wrap` at the available width is where the browser
    /// wraps). Paint the result with [`TextBlock::paint`]. A laid-out
    /// galley the caller passes is laid out again in this look, its own
    /// attributes kept, so its rows sit in their line boxes too.
    pub fn layout(
        &self,
        ui: &Ui,
        text: impl Into<WidgetText>,
        wrap: TextWrapMode,
        width: f32,
    ) -> TextBlock {
        let mut job = self.job(ui, text.into());
        job.wrap = TextWrapping::from_wrap_mode_and_width(wrap, width);
        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
        TextBlock {
            galley: place_rows(ui.ctx(), galley, self.leading),
        }
    }

    /// Show `text` in this look as an egui `Label` (selectable as egui's
    /// labels are, with the label's accessibility info): wrapped at the
    /// available width unless `wrap` says otherwise, each row placed as
    /// the browser places it, in the look's colour.
    pub fn label(&self, ui: &mut Ui, text: impl Into<WidgetText>, wrap: TextWrapMode) -> Response {
        let mut job = self.job(ui, text.into());
        fill_color(&mut job, self.color(ui));
        job.wrap = TextWrapping::from_wrap_mode_and_width(wrap, ui.available_width());
        job.halign = ui.layout().horizontal_placement();
        job.justify = ui.layout().horizontal_justify();
        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
        let galley = place_rows(ui.ctx(), galley, self.leading);
        ui.add(egui::Label::new(galley))
    }
}

impl TextLook {
    /// [`TextLook::layout`] with every colour the caller did not set
    /// replaced by `color` in the galley itself, for a galley egui paints
    /// (a `TextEdit`'s hint text).
    pub(crate) fn layout_colored(
        &self,
        ui: &Ui,
        text: impl Into<WidgetText>,
        wrap: TextWrapMode,
        width: f32,
        color: Color32,
    ) -> Arc<Galley> {
        let mut job = self.job(ui, text.into());
        fill_color(&mut job, color);
        job.wrap = TextWrapping::from_wrap_mode_and_width(wrap, width);
        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
        place_rows(ui.ctx(), galley, self.leading)
    }
}

/// Every placeholder colour of `job` (text and underline) set to `color`.
fn fill_color(job: &mut LayoutJob, color: Color32) {
    for section in &mut job.sections {
        if section.format.color == Color32::PLACEHOLDER {
            section.format.color = color;
        }
        if section.format.underline.color == Color32::PLACEHOLDER {
            section.format.underline.color = color;
        }
    }
}

/// Text laid out by [`TextLook::layout`].
#[derive(Clone, Debug)]
pub struct TextBlock {
    galley: Arc<Galley>,
}

impl TextBlock {
    /// The size of the line boxes: the text's width, the line height times
    /// the rows.
    pub fn size(&self) -> Vec2 {
        self.galley.size()
    }

    /// The laid-out galley, its rows placed in their line boxes.
    pub fn galley(&self) -> &Arc<Galley> {
        &self.galley
    }

    /// Paint the text with its first line box's top left at `pos`. `color`
    /// replaces every colour the caller did not set (see
    /// [`TextLook::color`]).
    pub fn paint(&self, painter: &Painter, pos: Pos2, color: Color32) {
        painter.galley(pos, self.galley.clone(), color);
    }
}

/// Run `add` in a child `Ui` whose text a crate container's recipe sets
/// (upstream's text colour, `text-*` and `leading-*` on the container):
/// plain egui widgets inside take the colour and the size, crate text
/// inside takes all three through [`TextLook::resolve`]. What the recipe
/// leaves out stays as `ui` has it.
pub fn content_scope<R>(
    ui: &mut Ui,
    look: &TextLook,
    add: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    let factor = look.line_height / look.size;
    let builder = egui::UiBuilder::new()
        .ui_stack_info(egui::UiStackInfo::default().with_tag_value(LINE_FACTOR_TAG, factor));
    ui.scope_builder(builder, |ui| {
        if let Some(c) = look.color {
            ui.visuals_mut().override_text_color = Some(c);
        }
        for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
            if let Some(font) = ui.style_mut().text_styles.get_mut(&style) {
                font.size = look.size;
            }
        }
        add(ui)
    })
}

/// The line-height factor of the nearest enclosing [`content_scope`].
fn inherited_line_factor(ui: &Ui) -> Option<f32> {
    ui.stack()
        .iter()
        .find_map(|frame| frame.tags().get_downcast::<f32>(LINE_FACTOR_TAG).copied())
}

fn body_size(ui: &Ui) -> f32 {
    egui::TextStyle::Body.resolve(ui.style()).size
}

/// The context's style with every colour egui would resolve for text
/// without an explicit colour (the override, `strong`, `weak`, the
/// underline's line colour) set to `Color32::PLACEHOLDER`, and no font
/// override. Built once per context style, not per frame.
fn placeholder_style(ctx: &egui::Context) -> Arc<egui::Style> {
    let id = egui::Id::new("shadcn_egui::text::placeholder_style");
    let source = ctx.global_style();
    let cached = ctx.data(|d| d.get_temp::<(Arc<egui::Style>, Arc<egui::Style>)>(id));
    if let Some((_, style)) = cached.filter(|(src, _)| Arc::ptr_eq(src, &source)) {
        return style;
    }
    let mut style = (*source).clone();
    style.override_font_id = None;
    style.override_text_style = None;
    let v = &mut style.visuals;
    v.override_text_color = None;
    v.weak_text_color = Some(Color32::PLACEHOLDER);
    v.widgets.active.fg_stroke.color = Color32::PLACEHOLDER;
    v.widgets.noninteractive.fg_stroke.color = Color32::PLACEHOLDER;
    let style = Arc::new(style);
    ctx.data_mut(|d| d.insert_temp(id, (source, style.clone())));
    style
}

/// A face's ascent, descent and line gap in em, as Skia picks them: OS/2
/// typo metrics when `USE_TYPO_METRICS` is set, else hhea. A line box with
/// a set line height leaves the line gap out; only a field's editor, a
/// `line-height: normal` line, uses it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct VMetrics {
    ascent: f32,
    descent: f32,
    line_gap: f32,
}

/// The [`VMetrics`] of face `index` in the font file `data` (TrueType,
/// OpenType or a collection), from its `head`, `hhea` and `OS/2` tables.
fn vmetrics(data: &[u8], index: u32) -> Option<VMetrics> {
    let u16_at = |o: usize| data.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    let i16_at = |o: usize| u16_at(o).map(|v| f32::from(v as i16));
    let u32_at = |o: usize| {
        data.get(o..o + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    let base = if data.get(0..4)? == b"ttcf" {
        u32_at(12 + 4 * index as usize)? as usize
    } else {
        0
    };
    let tables = usize::from(u16_at(base + 4)?);
    let table = |tag: &[u8; 4]| {
        (0..tables).find_map(|i| {
            let rec = base + 12 + 16 * i;
            (data.get(rec..rec + 4)? == tag)
                .then(|| u32_at(rec + 8).map(|o| o as usize))
                .flatten()
        })
    };
    let upem = f32::from(u16_at(table(b"head")? + 18)?);
    let hhea = table(b"hhea")?;
    let (mut ascent, mut descent, mut gap) =
        (i16_at(hhea + 4)?, -i16_at(hhea + 6)?, i16_at(hhea + 8)?);
    if let Some(os2) = table(b"OS/2") {
        // fsSelection bit 7, USE_TYPO_METRICS.
        if u16_at(os2 + 62).is_some_and(|sel| sel & 0x80 != 0) {
            ascent = i16_at(os2 + 68)?;
            descent = -i16_at(os2 + 70)?;
            gap = i16_at(os2 + 72)?;
        }
    }
    (upem > 0.0).then_some(VMetrics {
        ascent: ascent / upem,
        descent: descent / upem,
        line_gap: gap.max(0.0) / upem,
    })
}

/// The metrics of the first face of `family`, read from the context's
/// font definitions once and cached under the face's key.
fn family_vmetrics(ctx: &egui::Context, family: &FontFamily) -> Option<VMetrics> {
    let (id, data) = ctx.fonts(|f| {
        let defs = f.definitions();
        let key = defs.families.get(family)?.first()?;
        let id = egui::Id::new("shadcn_egui::text::vmetrics").with(key);
        Some((id, defs.font_data.get(key)?.clone()))
    })?;
    if let Some(m) = ctx.data(|d| d.get_temp::<Option<VMetrics>>(id)) {
        return m;
    }
    let m = vmetrics(&data.font, data.index);
    ctx.data_mut(|d| d.insert_temp(id, m));
    m
}

/// `galley` with each row moved so its baseline sits where Blink puts it
/// in the row's line box: ascent and descent of the row's first font
/// rounded to whole device pixels, the leading split by `leading`. egui
/// puts the baseline at the font's ascent rounded once, and its row
/// includes the line gap. The galley keeps its size, the line boxes, so
/// layout, selection and the cursor follow it.
pub(crate) fn place_rows(
    ctx: &egui::Context,
    mut galley: Arc<Galley>,
    leading: Leading,
) -> Arc<Galley> {
    let ppp = ctx.pixels_per_point();
    let g = Arc::make_mut(&mut galley);
    let job = g.job.clone();
    let mut bounds = egui::Rect::NOTHING;
    // The byte where the row starts in the job's text: rows take its
    // characters in order, a row ended by `\n` one more.
    let mut byte = 0;
    for placed in &mut g.rows {
        let start = byte;
        let chars = placed.row.glyphs.len() + usize::from(placed.ends_with_newline);
        for _ in 0..chars {
            byte += job.text[byte..].chars().next().map_or(0, char::len_utf8);
        }
        let Some(glyph) = placed.row.glyphs.first() else {
            continue;
        };
        let Some(section) = job
            .sections
            .iter()
            .find(|s| s.byte_range.contains(&start))
            .or(job.sections.last())
        else {
            continue;
        };
        let format = &section.format;
        let (size, line) = (format.font_id.size, placed.row.size.y);
        let em = family_vmetrics(ctx, &format.font_id.family).unwrap_or(VMetrics {
            ascent: glyph.font_face_ascent / size,
            descent: (glyph.font_face_height - glyph.font_face_ascent) / size,
            line_gap: 0.0,
        });
        let ascent = (em.ascent * size * ppp).round();
        let descent = (em.descent * size * ppp).round();
        let half = match leading {
            Leading::Inline => ((line * ppp - ascent - descent) / 2.0).floor(),
            Leading::Field => {
                let gap = (em.line_gap * size * ppp).round();
                ((line * ppp - ascent - descent - gap) / 2.0).ceil() + (gap / 2.0).floor()
            }
        };
        let shift = (half + ascent - (glyph.pos.y * ppp).round()) / ppp;
        placed.pos.y += shift;
        bounds |= placed
            .row
            .visuals
            .mesh_bounds
            .translate(placed.pos.to_vec2());
    }
    if bounds.is_positive() {
        g.mesh_bounds = bounds;
    }
    galley
}

/// `uppercase`: every section's text in upper case, byte ranges adjusted
/// (a letter may grow, as `ß` to `SS`).
fn uppercase(job: &mut LayoutJob) {
    let mut text = String::with_capacity(job.text.len());
    for section in &mut job.sections {
        let start = text.len();
        text.push_str(&job.text[section.byte_range.clone()].to_uppercase());
        section.byte_range = start..text.len();
    }
    job.text = text;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_ui(ppp: f32, f: impl FnOnce(&mut Ui)) {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(ppp);
        let mut f = Some(f);
        let _ = ctx.run_ui(Default::default(), |ui| {
            if let Some(f) = f.take() {
                f(ui);
            }
        });
    }

    #[test]
    fn recipe_fills_what_the_caller_left_out() {
        with_ui(1.0, |ui| {
            let mut r = Recipe::default();
            r.text_size.base = Some(TextSize::Xs);
            r.text.base = Some(crate::style::Paint::Role(crate::style::Role::Primary, 1.0));
            let look = TextLook::resolve(ui, &[&r], State::NONE);
            let theme = Theme::current(ui.ctx());
            assert_eq!(look.size(), theme.metrics.text_xs);
            assert_eq!(look.line_height(), theme.metrics.line_xs);
            // Plain text: recipe size, placeholder colour, recipe line box.
            let job = look.job(ui, "a".into());
            let f = &job.sections[0].format;
            assert_eq!(f.font_id.size, theme.metrics.text_xs);
            assert_eq!(f.color, Color32::PLACEHOLDER);
            assert_eq!(f.line_height, Some(theme.metrics.line_xs));
            // Explicit attributes win; strong is no explicit colour.
            let rich = egui::RichText::new("b")
                // token-ok: test data, a caller's explicit attribute
                .size(30.0)
                .line_height(Some(40.0))
                .strong();
            let job = look.job(ui, rich.into());
            let f = &job.sections[0].format;
            assert_eq!(f.font_id.size, 30.0);
            assert_eq!(f.line_height, Some(40.0));
            assert_eq!(f.color, Color32::PLACEHOLDER);
            // token-ok: test data, a caller's explicit attribute
            let red = egui::RichText::new("c").color(Color32::RED).weak();
            let job = look.job(ui, red.into());
            // token-ok: test data, a caller's explicit attribute
            assert_eq!(job.sections[0].format.color, Color32::RED);
        });
    }

    #[test]
    fn colour_falls_back_to_the_ui_text_colour() {
        with_ui(1.0, |ui| {
            let look = TextLook::resolve(ui, &[], State::NONE);
            ui.visuals_mut().override_text_color = Some(Color32::GREEN);
            assert_eq!(look.color(ui), Color32::GREEN);
        });
    }

    #[test]
    fn content_scope_passes_line_factor_and_size() {
        with_ui(1.0, |ui| {
            let mut r = Recipe::default();
            r.text_size.base = Some(TextSize::Px(10.0));
            r.line_height.base = Some(2.0);
            let outer = TextLook::resolve(ui, &[&r], State::NONE);
            content_scope(ui, &outer, |ui| {
                let inner = TextLook::resolve(ui, &[], State::NONE);
                assert_eq!(inner.size(), 10.0);
                assert_eq!(inner.line_height(), 20.0);
            });
        });
    }

    /// The baseline of every row, in device pixels below the galley top.
    fn baselines(galley: &Galley, ppp: f32) -> Vec<f32> {
        galley
            .rows
            .iter()
            .map(|r| (r.pos.y + r.row.glyphs[0].pos.y) * ppp)
            .collect()
    }

    #[test]
    fn every_row_sits_on_the_blink_baseline() {
        for ppp in [1.0, 1.5, 2.0] {
            with_ui(ppp, |ui| {
                let mut r = Recipe::default();
                r.text_size.base = Some(TextSize::Px(10.0));
                r.line_height.base = Some(3.0);
                let look = TextLook::resolve(ui, &[&r], State::NONE);
                let block = look.layout(ui, "Hg\nHg", TextWrapMode::Extend, f32::INFINITY);
                assert_eq!(block.size().y, 60.0);
                let font = look.font_id(ui.ctx());
                let em = family_vmetrics(ui.ctx(), &font.family).expect("font tables");
                let (a, d) = (
                    (em.ascent * 10.0 * ppp).round(),
                    (em.descent * 10.0 * ppp).round(),
                );
                let first = ((30.0 * ppp - a - d) / 2.0).floor() + a;
                assert_eq!(baselines(block.galley(), ppp), [first, first + 30.0 * ppp]);
                let field = look.clone().leading(Leading::Field);
                let block = field.layout(ui, "Hg", TextWrapMode::Extend, f32::INFINITY);
                // The field adds the line gap of egui's default font.
                let g = (em.line_gap * 10.0 * ppp).round();
                let up = ((30.0 * ppp - a - d - g) / 2.0).ceil() + (g / 2.0).floor() + a;
                assert_eq!(baselines(block.galley(), ppp), [up]);
            });
        }
    }

    #[test]
    fn metrics_come_from_the_font_tables() {
        // Inter sets USE_TYPO_METRICS: typo ascender 1984, descender -494,
        // 2048 units per em.
        let inter = shadcn_egui_font_inter::FONT.face(400);
        let m = vmetrics(inter.data, 0).expect("tables");
        assert_eq!(m.ascent, 1984.0 / 2048.0);
        assert_eq!(m.descent, 494.0 / 2048.0);
        assert_eq!(m.line_gap, 0.0);
    }

    #[test]
    fn uppercase_keeps_sections_apart() {
        let mut job = LayoutJob::default();
        job.append("straße", 0.0, Default::default());
        job.append("x", 0.0, Default::default());
        uppercase(&mut job);
        assert_eq!(job.text, "STRASSEX");
        assert_eq!(job.sections[0].byte_range, 0..7);
        assert_eq!(job.sections[1].byte_range, 7..8);
    }
}
