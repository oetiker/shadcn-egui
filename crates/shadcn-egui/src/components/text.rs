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
//!     block.paint(ui.painter(), rect.min, look.resolved_color(ui));
//!     resp
//! }
//! ```

use crate::style::{pick, Recipe, State, TextSize, Weight};
use crate::theme::{family_for, heading_family_for};
use crate::Theme;
use egui::text::{LayoutJob, TextWrapping};
use egui::FontData;
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
    /// each rounded to device pixels), the baseline half the line gap
    /// (rounded down) plus the ascent below its top. Each row is that
    /// line, not the line box, so a field's caret and selection span it;
    /// the field centres it in its content box (egui's
    /// `Align::Center`), and the painter rounds its top to the device
    /// pixel as Blink snaps the editor.
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
    pub fn color(mut self, color: Color32) -> Self {
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

    /// Where Blink paints a field's caret in its editor line, in points
    /// below the line's top: the font's ascent and descent (each rounded
    /// to device pixels) around the baseline, which sits half the line
    /// gap plus the ascent below that top.
    pub(crate) fn caret_span(&self, ctx: &egui::Context) -> Option<std::ops::Range<f32>> {
        let ppp = ctx.pixels_per_point();
        let em = family_vmetrics(ctx, &self.font_id(ctx).family)?;
        let [a, d, gap] =
            [em.ascent, em.descent, em.line_gap].map(|v| (v * self.size * ppp).round());
        let top = (gap / 2.0).floor();
        Some(top / ppp..(top + a + d) / ppp)
    }

    /// The colour of text without an explicit colour: the recipe's, else
    /// `ui`'s text colour.
    pub fn resolved_color(&self, ui: &Ui) -> Color32 {
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
        fill_color(&mut job, self.resolved_color(ui));
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
/// `None` for a file that is not a font or is cut short. Each read stays
/// inside its table: a short `OS/2` table without the typo fields counts
/// as none. A font whose `hhea` ascender and descender are both 0 takes
/// the typo metrics, else the win metrics, as FreeType does.
fn vmetrics(data: &[u8], index: u32) -> Option<VMetrics> {
    let bytes = |o: usize, n: usize| data.get(o..o.checked_add(n)?);
    let u16_at = |o: usize| bytes(o, 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    let u32_at = |o: usize| bytes(o, 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    let base = if bytes(0, 4)? == b"ttcf" {
        // numFonts at 8, the offset table of each face from 12.
        if index >= u32_at(8)? {
            return None;
        }
        let rec = usize::try_from(index)
            .ok()?
            .checked_mul(4)?
            .checked_add(12)?;
        usize::try_from(u32_at(rec)?).ok()?
    } else {
        0
    };
    let tables = usize::from(u16_at(base.checked_add(4)?)?);
    // A table as its bytes, from its record: tag, checksum, offset, length.
    let table = |tag: &[u8; 4]| {
        (0..tables).find_map(|i| {
            let rec = base.checked_add(12 + 16 * i)?;
            if bytes(rec, 4)? != tag {
                return None;
            }
            let offset = usize::try_from(u32_at(rec.checked_add(8)?)?).ok()?;
            bytes(offset, usize::try_from(u32_at(rec.checked_add(12)?)?).ok()?)
        })
    };
    let be16 = |t: &[u8], o: usize| t.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    let i16_in = |t: &[u8], o: usize| be16(t, o).map(|v| f32::from(v as i16));
    let upem = f32::from(be16(table(b"head")?, 18)?);
    let hhea = table(b"hhea")?;
    let (mut ascent, mut descent, mut gap) =
        (i16_in(hhea, 4)?, -i16_in(hhea, 6)?, i16_in(hhea, 8)?);
    // sTypoAscender, sTypoDescender, sTypoLineGap; a short table lacks them.
    let os2 = table(b"OS/2");
    let typo = os2.and_then(|t| Some((i16_in(t, 68)?, -i16_in(t, 70)?, i16_in(t, 72)?)));
    if ascent == 0.0 && descent == 0.0 {
        let win = os2.and_then(|t| Some((f32::from(be16(t, 74)?), f32::from(be16(t, 76)?))));
        match (typo, win) {
            (Some((a, d, g)), _) if a != 0.0 || d != 0.0 => (ascent, descent, gap) = (a, d, g),
            (_, Some((a, d))) => (ascent, descent, gap) = (a, d, 0.0),
            _ => {}
        }
    }
    // fsSelection bit 7, USE_TYPO_METRICS.
    let use_typo = os2
        .and_then(|t| be16(t, 62))
        .is_some_and(|sel| sel & 0x80 != 0);
    if let (true, Some(t)) = (use_typo, typo) {
        (ascent, descent, gap) = t;
    }
    (upem > 0.0).then_some(VMetrics {
        ascent: ascent / upem,
        descent: descent / upem,
        line_gap: gap.max(0.0) / upem,
    })
}

/// The metrics of the first face of `family`, read from the context's
/// font definitions once per face. The cache keeps the face's data with
/// the metrics: a theme that switches fonts reuses the key names for
/// other faces.
fn family_vmetrics(ctx: &egui::Context, family: &FontFamily) -> Option<VMetrics> {
    let (id, data) = ctx.fonts(|f| {
        let defs = f.definitions();
        let key = defs.families.get(family)?.first()?;
        let id = egui::Id::new("shadcn_egui::text::vmetrics").with(key);
        Some((id, defs.font_data.get(key)?.clone()))
    })?;
    let cached = ctx.data(|d| d.get_temp::<(Arc<FontData>, Option<VMetrics>)>(id));
    if let Some((_, m)) = cached.filter(|(src, _)| Arc::ptr_eq(src, &data)) {
        return m;
    }
    let m = vmetrics(&data.font, data.index);
    ctx.data_mut(|d| d.insert_temp(id, (data, m)));
    m
}

/// A galley placed by [`place_rows`], with what it was placed from.
#[derive(Clone)]
struct Placed {
    source: Arc<Galley>,
    ppp: f32,
    leading: Leading,
    placed: Arc<Galley>,
    frame: u64,
}

/// The galleys [`place_rows`] placed in this frame and the one before, by
/// the address of the galley egui laid out. egui's galley cache hands out
/// the same galley while the text stays the same, so an idle widget's text
/// is placed once, not once per frame.
#[derive(Clone, Default)]
struct PlacedCache {
    frame: u64,
    entries: std::collections::HashMap<usize, Placed>,
}

/// [`place_galley_rows`], cached per galley egui laid out (notes §1: an
/// idle widget allocates nothing beyond egui's own).
pub(crate) fn place_rows(
    ctx: &egui::Context,
    galley: Arc<Galley>,
    leading: Leading,
) -> Arc<Galley> {
    let id = egui::Id::new("shadcn_egui::text::placed");
    let (ppp, frame) = (ctx.pixels_per_point(), ctx.cumulative_frame_nr());
    let key = Arc::as_ptr(&galley) as usize;
    let hit = ctx.data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<PlacedCache>(id);
        if cache.frame != frame {
            // Text not shown in the last frame is gone from egui's cache too.
            cache.entries.retain(|_, e| e.frame + 1 >= frame);
            cache.frame = frame;
        }
        let e = cache.entries.get_mut(&key)?;
        let same = Arc::ptr_eq(&e.source, &galley) && e.ppp == ppp && e.leading == leading;
        same.then(|| {
            e.frame = frame;
            e.placed.clone()
        })
    });
    if let Some(placed) = hit {
        return placed;
    }
    // Placing reads the font tables through `ctx.data`: not inside the
    // `data_mut` above.
    let placed = place_galley_rows(ctx, galley.clone(), leading);
    ctx.data_mut(|d| {
        let entry = Placed {
            source: galley,
            ppp,
            leading,
            placed: placed.clone(),
            frame,
        };
        d.get_temp_mut_or_default::<PlacedCache>(id)
            .entries
            .insert(key, entry);
    });
    placed
}

/// `galley` with each row in its line box as Blink lays it out: the line
/// tops at the unrounded sum of the line heights above, the baseline
/// where Blink paints it, the half leading (split by `leading`) and the
/// ascent of the row's first font added to the line top in device pixels,
/// ascent and descent each rounded to whole device pixels, the sum
/// rounded to the pixel. egui rounds every row's height and top to the
/// pixel and puts the baseline at the font's ascent; here each row keeps
/// its line box as its rect (selection and cursor span it) and its glyphs
/// move inside it. The galley is as tall as its line boxes.
fn place_galley_rows(
    ctx: &egui::Context,
    mut galley: Arc<Galley>,
    leading: Leading,
) -> Arc<Galley> {
    let ppp = ctx.pixels_per_point();
    let g = Arc::make_mut(&mut galley);
    let job = g.job.clone();
    let mut bounds = egui::Rect::NOTHING;
    // The line top of the row, in points from the galley top.
    let mut top = 0.0;
    // The byte where the row starts in the job's text: rows take its
    // characters in order, a row ended by `\n` one more.
    let mut byte = 0;
    for placed in &mut g.rows {
        let start = byte;
        let chars = placed.row.glyphs.len() + usize::from(placed.ends_with_newline);
        for _ in 0..chars {
            byte += job.text[byte..].chars().next().map_or(0, char::len_utf8);
        }
        let section = job
            .sections
            .iter()
            .find(|s| s.byte_range.contains(&start))
            .or(job.sections.last());
        // The row's ascent, descent and line gap in device px, each
        // rounded: from its first font's tables, else egui's metrics.
        let glyph = placed.row.glyphs.first();
        let dev = section.and_then(|s| {
            let size = s.format.font_id.size;
            let em = family_vmetrics(ctx, &s.format.font_id.family).or_else(|| {
                glyph.map(|g| VMetrics {
                    ascent: g.font_face_ascent / size,
                    descent: (g.font_face_height - g.font_face_ascent) / size,
                    line_gap: 0.0,
                })
            })?;
            Some([em.ascent, em.descent, em.line_gap].map(|v| (v * size * ppp).round()))
        });
        // The row's line box: its tallest line height, as egui takes it
        // before rounding; an empty row has its section's. A field's row
        // is its editor line.
        let line = match (leading, dev) {
            (Leading::Field, Some([a, d, gap])) => (a + d + gap) / ppp,
            _ => placed
                .row
                .glyphs
                .iter()
                .map(|g| g.line_height)
                .reduce(f32::max)
                .or_else(|| section.and_then(|s| s.format.line_height))
                .unwrap_or(placed.row.size.y),
        };
        placed.pos.y = top;
        let row = Arc::make_mut(&mut placed.row);
        row.size.y = line;
        if let (Some(glyph), Some([ascent, descent, gap])) = (row.glyphs.first(), dev) {
            let half = match leading {
                Leading::Inline => ((line * ppp - ascent - descent) / 2.0).floor(),
                Leading::Field => (gap / 2.0).floor(),
            };
            let baseline = (top * ppp + half + ascent).round() / ppp - top;
            let shift = egui::vec2(0.0, baseline - glyph.pos.y);
            for glyph in &mut row.glyphs {
                glyph.pos.y += shift.y;
            }
            row.visuals.mesh.translate(shift);
            row.visuals.mesh_bounds = row.visuals.mesh_bounds.translate(shift);
        }
        bounds |= row.visuals.mesh_bounds.translate(placed.pos.to_vec2());
        top += line;
    }
    g.rect.max.y = g.rect.min.y + top;
    g.mesh_bounds = bounds;
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
            assert_eq!(look.resolved_color(ui), Color32::GREEN);
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
                // The field's row is its editor line, with the line gap of
                // egui's default font, the baseline half the gap below.
                let g = (em.line_gap * 10.0 * ppp).round();
                assert_eq!(block.size().y * ppp, a + d + g);
                assert_eq!(baselines(block.galley(), ppp), [(g / 2.0).floor() + a]);
            });
        }
    }

    #[test]
    fn fractional_line_boxes_stay_unrounded() {
        // `text-xs/relaxed`: 12 px text on 19.5 px lines. Blink keeps the
        // line tops at k * 19.5 and rounds only the painted baseline.
        for ppp in [1.0, 1.5, 2.0] {
            with_ui(ppp, |ui| {
                let mut r = Recipe::default();
                r.text_size.base = Some(TextSize::Px(12.0));
                r.line_height.base = Some(1.625);
                let look = TextLook::resolve(ui, &[&r], State::NONE);
                let text = "Hg\nHg\nHg";
                let block = look.layout(ui, text, TextWrapMode::Extend, f32::INFINITY);
                let galley = block.galley();
                assert_eq!(block.size().y, 58.5);
                let tops: Vec<_> = galley.rows.iter().map(|r| r.rect()).collect();
                for (k, rect) in tops.iter().enumerate() {
                    assert_eq!((rect.top(), rect.height()), (k as f32 * 19.5, 19.5));
                }
                let font = look.font_id(ui.ctx());
                let em = family_vmetrics(ui.ctx(), &font.family).expect("font tables");
                let (a, d) = (
                    (em.ascent * 12.0 * ppp).round(),
                    (em.descent * 12.0 * ppp).round(),
                );
                let half = ((19.5 * ppp - a - d) / 2.0).floor();
                let want: Vec<_> = (0..3)
                    .map(|k| (k as f32 * 19.5 * ppp + half + a).round())
                    .collect();
                assert_eq!(baselines(galley, ppp), want);
                // The drawn glyphs move with the baseline: each vertex
                // keeps egui's offset from its glyph's baseline.
                let mut job = look.job(ui, text.into());
                job.wrap =
                    TextWrapping::from_wrap_mode_and_width(TextWrapMode::Extend, f32::INFINITY);
                let egui_galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
                for (placed, egui_row) in galley.rows.iter().zip(&egui_galley.rows) {
                    let (y, y0) = (placed.glyphs[0].pos.y, egui_row.glyphs[0].pos.y);
                    let mesh = &placed.visuals.mesh.vertices;
                    for (v, v0) in mesh.iter().zip(&egui_row.visuals.mesh.vertices) {
                        assert!((v.pos.y - y - (v0.pos.y - y0)).abs() < 1e-4);
                    }
                    // and land on the device pixel grid.
                    let top = (placed.pos.y + mesh[0].pos.y) * ppp;
                    assert!((top - top.round()).abs() < 1e-3, "{top}");
                }
            });
        }
    }

    #[test]
    fn an_unchanged_text_is_placed_once() {
        let ctx = egui::Context::default();
        let mut galleys = Vec::new();
        for _ in 0..3 {
            let _ = ctx.run_ui(Default::default(), |ui| {
                let look = TextLook::resolve(ui, &[], State::NONE);
                let block = look.layout(ui, "Hg", TextWrapMode::Extend, f32::INFINITY);
                galleys.push(block.galley().clone());
            });
        }
        assert!(Arc::ptr_eq(&galleys[0], &galleys[1]));
        assert!(Arc::ptr_eq(&galleys[1], &galleys[2]));
        // Another scale places it again.
        ctx.set_pixels_per_point(2.0);
        let _ = ctx.run_ui(Default::default(), |ui| {
            let look = TextLook::resolve(ui, &[], State::NONE);
            let block = look.layout(ui, "Hg", TextWrapMode::Extend, f32::INFINITY);
            assert!(!Arc::ptr_eq(&galleys[2], block.galley()));
            assert_eq!(block.galley().pixels_per_point, 2.0);
        });
    }

    #[test]
    fn metrics_follow_a_font_switch_under_the_same_key() {
        // A theme that switches fonts keeps its key names: the metrics
        // must come from the face now under the key.
        let ctx = egui::Context::default();
        let defs = |data: egui::FontData| {
            let mut d = egui::FontDefinitions::empty();
            d.font_data.insert("k".into(), Arc::new(data));
            d.families
                .insert(FontFamily::Proportional, vec!["k".into()]);
            d
        };
        let measure = |ctx: &egui::Context| {
            let mut m = None;
            let _ = ctx.run_ui(Default::default(), |ui| {
                m = family_vmetrics(ui.ctx(), &FontFamily::Proportional);
            });
            m.expect("font tables")
        };
        let other = egui::FontDefinitions::default();
        let ubuntu = other.font_data["Ubuntu-Light"].as_ref().clone();
        let inter = shadcn_egui_font_inter::FONT.face(400);
        ctx.set_fonts(defs(ubuntu.clone()));
        let before = measure(&ctx);
        ctx.set_fonts(defs(egui::FontData::from_static(inter.data)));
        let after = measure(&ctx);
        assert_ne!(before, after);
        assert_eq!(after, vmetrics(inter.data, 0).expect("tables"));
        assert_eq!(before, vmetrics(&ubuntu.font, 0).expect("tables"));
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

    /// The `OS/2` fields the metrics read, in a table of `len` bytes.
    struct Os2 {
        len: usize,
        sel: u16,
        typo: (i16, i16, i16),
        win: (u16, u16),
    }

    /// A minimal font file: `head`, `hhea` and, if given, an `OS/2`
    /// table, all values big-endian at their offsets.
    fn font(hhea: (i16, i16, i16), os2: Option<Os2>) -> Vec<u8> {
        let mut head = vec![0u8; 54];
        head[18..20].copy_from_slice(&1000u16.to_be_bytes());
        let mut hh = vec![0u8; 36];
        for (o, v) in [(4, hhea.0), (6, hhea.1), (8, hhea.2)] {
            hh[o..o + 2].copy_from_slice(&v.to_be_bytes());
        }
        let mut tables: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"head", head), (b"hhea", hh)];
        if let Some(Os2 {
            len,
            sel,
            typo,
            win,
        }) = os2
        {
            let mut t = vec![0u8; 78];
            t[62..64].copy_from_slice(&sel.to_be_bytes());
            for (o, v) in [(68, typo.0), (70, typo.1), (72, typo.2)] {
                t[o..o + 2].copy_from_slice(&v.to_be_bytes());
            }
            t[74..76].copy_from_slice(&win.0.to_be_bytes());
            t[76..78].copy_from_slice(&win.1.to_be_bytes());
            t.truncate(len);
            tables.push((b"OS/2", t));
        }
        let mut out = vec![0u8; 12 + 16 * tables.len()];
        out[4..6].copy_from_slice(&(tables.len() as u16).to_be_bytes());
        for (i, (tag, t)) in tables.iter().enumerate() {
            let rec = 12 + 16 * i;
            out[rec..rec + 4].copy_from_slice(*tag);
            let offset = out.len() as u32;
            out[rec + 8..rec + 12].copy_from_slice(&offset.to_be_bytes());
            out[rec + 12..rec + 16].copy_from_slice(&(t.len() as u32).to_be_bytes());
            out.extend_from_slice(t);
        }
        out
    }

    #[test]
    fn metrics_survive_broken_and_unusual_tables() {
        let m = |a: f32, d: f32, g: f32| VMetrics {
            ascent: a / 1000.0,
            descent: d / 1000.0,
            line_gap: g / 1000.0,
        };
        // hhea, typo metrics when USE_TYPO_METRICS is set.
        let typo = Some(Os2 {
            len: 78,
            sel: 0x80,
            typo: (900, -300, 50),
            win: (1100, 400),
        });
        assert_eq!(
            vmetrics(&font((800, -200, 10), None), 0),
            Some(m(800.0, 200.0, 10.0))
        );
        assert_eq!(
            vmetrics(&font((800, -200, 10), typo), 0),
            Some(m(900.0, 300.0, 50.0))
        );
        // A version-0 OS/2 cut before the typo fields: hhea.
        let short = Some(Os2 {
            len: 68,
            sel: 0x80,
            typo: (900, -300, 50),
            win: (1100, 400),
        });
        assert_eq!(
            vmetrics(&font((800, -200, 10), short), 0),
            Some(m(800.0, 200.0, 10.0))
        );
        // hhea all zero: typo metrics, else win metrics (FreeType).
        let no_flag = Some(Os2 {
            len: 78,
            sel: 0,
            typo: (900, -300, 50),
            win: (1100, 400),
        });
        assert_eq!(
            vmetrics(&font((0, 0, 0), no_flag), 0),
            Some(m(900.0, 300.0, 50.0))
        );
        let win_only = Some(Os2 {
            len: 78,
            sel: 0,
            typo: (0, 0, 0),
            win: (1100, 400),
        });
        assert_eq!(
            vmetrics(&font((0, 0, 0), win_only), 0),
            Some(m(1100.0, 400.0, 0.0))
        );
        // A table record pointing past the end, a face index past the
        // collection, offsets near the address limit: no metrics, no panic.
        let mut bad = font((800, -200, 10), None);
        bad[12 + 16 + 8..12 + 16 + 12].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(vmetrics(&bad, 0), None);
        let mut ttc = b"ttcf".to_vec();
        ttc.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 1]);
        ttc.extend_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(vmetrics(&ttc, 1), None);
        assert_eq!(vmetrics(&ttc, 0), None);
        assert_eq!(vmetrics(b"tt", 0), None);
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
