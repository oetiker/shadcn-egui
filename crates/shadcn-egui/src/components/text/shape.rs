//! Glyph advances as Chrome sets them (conformance 3.4). Chrome shapes text
//! with HarfBuzz: each glyph advances by its width at the face's variation
//! location (the `wght` the theme sets, `opsz` at the font size, as
//! `font-optical-sizing: auto` does), plus the pair kerning of the face's
//! GPOS `kern` feature. egui 0.34 takes advances from the font's default
//! instance and kerns from the legacy `kern` table only, which no font the
//! crate ships has. [`advances`] measures a job as Chrome does; the text
//! path moves egui's glyphs there and breaks lines on these widths.

use egui::text::LayoutJob;
use egui::{FontData, FontFamily};
use skrifa::instance::{Location, Size};
use skrifa::raw::tables::gpos::{PairPos, PositionLookup, PositionSubtables, ValueRecord};
use skrifa::raw::tables::layout::DeviceOrVariationIndex;
use skrifa::raw::tables::variations::{DeltaSetIndex, FloatItemDeltaTarget, ItemVariationStore};
use skrifa::raw::types::{F2Dot14, FWord};
use skrifa::raw::FontData as RawData;
use skrifa::raw::{FontRef, TableProvider};
use skrifa::{GlyphId, MetadataProvider, Tag};
use std::sync::Arc;

/// Per char of a job's text: Chrome's advance in points (letter spacing
/// included), the kerning between the char before and it (0 at a
/// paragraph start and next to a char the face lacks), and egui's own
/// advance. A `\n` has 0 for all of them. `fixed` and `fixed_kern` hold
/// the advance (without letter spacing) and the kerning as Chrome holds
/// them on the device: HarfBuzz positions, 1/65536 device px; `spacing`
/// is the letter spacing in device px. A default ignorable char (a soft
/// hyphen, a zero-width space, ...) has 0 for all of them, and the
/// kerning reaches across it, as HarfBuzz hides it. `hyphen` holds, for a
/// soft hyphen, the hyphen Blink draws where it ends a line (U+2010 where
/// the face has it, else `-`, as `HyphenString` picks) and its advance in
/// HarfBuzz positions; `None` for every other char.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Advances {
    pub chrome: Vec<f32>,
    pub kern: Vec<f32>,
    pub egui: Vec<f32>,
    pub fixed: Vec<i32>,
    pub fixed_kern: Vec<i32>,
    pub spacing: Vec<f32>,
    pub hyphen: Vec<Option<(char, i32)>>,
}

/// Unicode's `Default_Ignorable_Code_Point`: chars a shaper draws as
/// nothing.
pub(super) fn ignorable(c: char) -> bool {
    matches!(
        c,
        '\u{AD}'
            | '\u{34F}'
            | '\u{61C}'
            | '\u{115F}'..='\u{1160}'
            | '\u{17B4}'..='\u{17B5}'
            | '\u{180B}'..='\u{180F}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{3164}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FEFF}'
            | '\u{FFA0}'
            | '\u{FFF0}'..='\u{FFF8}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0000}'..='\u{E0FFF}'
    )
}

/// One device px in HarfBuzz positions (16.16 fixed point).
pub(super) const FIXED_ONE: f32 = 65536.0; // behaviour-ok: hb_position_t is 16.16
/// One px in FreeType's 26.6 sizes.
const FT_PX: f32 = 64.0; // behaviour-ok: FreeType's 26.6 fixed point
/// HarfBuzz positions per Blink `LayoutUnit` (1/64 px).
const FIXED_PER_LAYOUT_UNIT: i64 = 1024; // behaviour-ok: 65536 / 64
/// Blink `LayoutUnit`s per px.
const LAYOUT_UNITS: f32 = 64.0; // behaviour-ok: LayoutUnit::kFixedPointDenominator

/// `v` device px in HarfBuzz positions, rounded.
pub(super) fn to_fixed(v: f32) -> i64 {
    (f64::from(v) * f64::from(FIXED_ONE)).round() as i64
}

/// A position `x` in HarfBuzz units rounded up to a `LayoutUnit`
/// (`ToCeil<LayoutUnit>`), in device px.
pub(super) fn ceil_to_layout_unit(x: i64) -> f32 {
    (x + FIXED_PER_LAYOUT_UNIT - 1).div_euclid(FIXED_PER_LAYOUT_UNIT) as f32 / LAYOUT_UNITS
}

/// Chrome's scale of one face at one device font size: FreeType's for the
/// advances (Skia's linear advances on Linux), HarfBuzz's for the GPOS
/// values (Blink sets the HarfBuzz scale to the size in 16.16).
#[derive(Clone, Copy, Debug)]
struct FixedScale {
    /// FreeType's `x_scale`: font units to 26.6 px, in 16.16.
    ft: i64,
    /// HarfBuzz's `x_scale` and its `x_mult` (`(x_scale << 16) / upem`).
    hb: i64,
    hb_mult: i64,
    upem: i64,
}

impl FixedScale {
    fn new(size_px: f32, upem: f32) -> Self {
        let upem = upem as i64;
        // `FT_Set_Char_Size` takes the size in 26.6 (Skia truncates);
        // `FT_DivFix` rounds.
        let size_26_6 = (size_px * FT_PX) as i64;
        let hb = (size_px * FIXED_ONE) as i64;
        Self {
            ft: (size_26_6 * 65536 + upem / 2) / upem,
            hb,
            hb_mult: (hb << 16) / upem,
            upem,
        }
    }

    /// An advance of `units` (variations applied) as FreeType's linear
    /// advance: the units rounded, then `FT_MulDiv(units, x_scale, 64)`.
    fn advance(self, units: f32) -> i32 {
        let units = units.round() as i64;
        let v = units * self.ft;
        ((v.abs() + 32) / 64 * v.signum()) as i32
    }

    /// A GPOS x advance of `base` units plus a variation `delta` as
    /// HarfBuzz scales them: `em_scale_x` and `em_scalef_x`.
    fn gpos(self, base: i16, delta: f32) -> i32 {
        let base = (i64::from(base) * self.hb_mult + 32768) >> 16;
        let delta = (delta * self.hb as f32 / self.upem as f32).round() as i64;
        (base + delta) as i32
    }
}

/// The first face of `family` in the context's font definitions.
pub(super) fn family_face(ctx: &egui::Context, family: &FontFamily) -> Option<Arc<FontData>> {
    ctx.fonts(|f| {
        let defs = f.definitions();
        let key = defs.families.get(family)?.first()?;
        defs.font_data.get(key).cloned()
    })
}

/// Whether egui must be told the optical size: `data` has an `opsz` axis
/// and neither the face's tweak nor `coords` set it. Chrome then draws at
/// `opsz` = the font size (`font-optical-sizing: auto`); egui draws the
/// outlines at the tweak's and the section's coordinates only.
pub(super) fn needs_opsz(data: &FontData, coords: &[(Tag, f32)]) -> bool {
    let opsz = Tag::new(b"opsz");
    let set = data.tweak.coords.as_ref().iter().chain(coords);
    if set.into_iter().any(|(t, _)| *t == opsz) {
        return false;
    }
    FontRef::from_index(&data.font, data.index)
        .is_ok_and(|f| f.axes().iter().any(|a| a.tag() == opsz))
}

/// One face, ready to measure: its tables at the location Chrome uses.
struct Face<'a> {
    font: FontRef<'a>,
    location: Location,
    upem: f32,
    kern: Vec<PositionLookup<'a>>,
}

impl<'a> Face<'a> {
    fn new(data: &'a FontData, coords: &[(Tag, f32)], size: f32) -> Option<Self> {
        let font = FontRef::from_index(&data.font, data.index).ok()?;
        let axes = font.axes();
        let opsz = Tag::new(b"opsz");
        // The theme's coordinates (`wght`), then the section's; `opsz`
        // follows the font size unless one of them sets it.
        let mut settings: Vec<(Tag, f32)> = data.tweak.coords.as_ref().to_vec();
        settings.extend_from_slice(coords);
        if !settings.iter().any(|(t, _)| *t == opsz) {
            settings.insert(0, (opsz, size));
        }
        let location = axes.location(settings.iter().copied());
        let upem = f32::from(font.head().ok()?.units_per_em());
        Some(Self {
            kern: kern_lookups(&font).unwrap_or_default(),
            font,
            location,
            upem,
        })
    }

    /// The pair kerning of `a` followed by `b`, in font units: every
    /// `kern` lookup in order, each applying its first subtable that
    /// covers the pair, as HarfBuzz does.
    fn kerning(&self, a: GlyphId, b: GlyphId, deltas: &Deltas, scale: FixedScale) -> i32 {
        let mut total = 0;
        for lookup in &self.kern {
            let Ok(PositionSubtables::Pair(subtables)) = lookup.subtables() else {
                continue;
            };
            for sub in subtables.iter().flatten() {
                if let Some((base, delta)) = pair_value(&sub, a, b, deltas) {
                    total += scale.gpos(base, delta);
                    break;
                }
            }
        }
        total
    }
}

/// The x advance adjustment of the first glyph for the pair `a`, `b` in
/// one pair subtable, in units and its variation delta, `None` where the
/// subtable does not apply.
/// A face's variation deltas at its location: GDEF's item variation
/// store and the normalized coordinates.
struct Deltas<'a> {
    store: Option<ItemVariationStore<'a>>,
    coords: &'a [F2Dot14],
}

impl Deltas<'_> {
    /// The x advance of `record` (its offsets relative to `data`) in units
    /// and its variation delta, unrounded as HarfBuzz sums it: read-fonts
    /// rounds the delta of `ValueRecord::value` to whole units.
    fn x_advance(&self, record: &ValueRecord, data: RawData) -> (i16, f32) {
        let base = record.x_advance().unwrap_or_default();
        let delta = match (record.x_advance_device(data), &self.store) {
            (Some(Ok(DeviceOrVariationIndex::VariationIndex(v))), Some(store)) => store
                .compute_float_delta(
                    DeltaSetIndex {
                        outer: v.delta_set_outer_index(),
                        inner: v.delta_set_inner_index(),
                    },
                    self.coords,
                )
                .map_or(0.0, |d| FWord::new(0).apply_float_delta(d)),
            _ => 0.0,
        };
        (base, delta)
    }
}

fn pair_value(sub: &PairPos, a: GlyphId, b: GlyphId, deltas: &Deltas) -> Option<(i16, f32)> {
    match sub {
        PairPos::Format1(t) => {
            let index = t.coverage().ok()?.get(a)?;
            let set = t.pair_sets().get(usize::from(index)).ok()?;
            let records = set.pair_value_records();
            let record = records
                .iter()
                .flatten()
                .find(|r| GlyphId::from(r.second_glyph()) == b)?;
            Some(deltas.x_advance(record.value_record1(), set.offset_data()))
        }
        PairPos::Format2(t) => {
            t.coverage().ok()?.get(a)?;
            let c1 = t.class_def1().ok()?.get(a);
            let c2 = t.class_def2().ok()?.get(b);
            // Through the record, not `PairPosFormat2::values`: read-fonts
            // 0.37 resolves the device offsets there one level too deep, so
            // a variable font's kerning keeps its default-instance value.
            let class1 = t.class1_records().get(usize::from(c1)).ok()?;
            let record = class1.class2_records().get(usize::from(c2)).ok()?;
            Some(deltas.x_advance(record.value_record1(), t.offset_data()))
        }
    }
}

/// The lookups of the GPOS `kern` feature for Latin text (the `latn`
/// script, else `DFLT`; its default language), in lookup order.
fn kern_lookups<'a>(font: &FontRef<'a>) -> Option<Vec<PositionLookup<'a>>> {
    let gpos = font.gpos().ok()?;
    let scripts = gpos.script_list().ok()?;
    let script = [Tag::new(b"latn"), Tag::new(b"DFLT")]
        .into_iter()
        .find_map(|tag| {
            scripts
                .script_records()
                .iter()
                .find(|r| r.script_tag() == tag)
        })?
        .script(scripts.offset_data())
        .ok()?;
    let lang = script.default_lang_sys()?.ok()?;
    let features = gpos.feature_list().ok()?;
    let mut indices: Vec<u16> = Vec::new();
    for i in lang.feature_indices() {
        let record = features.feature_records().get(usize::from(i.get()))?;
        if record.feature_tag() == Tag::new(b"kern") {
            let feature = record.feature(features.offset_data()).ok()?;
            indices.extend(feature.lookup_list_indices().iter().map(|l| l.get()));
        }
    }
    indices.sort_unstable();
    indices.dedup();
    let lookups = gpos.lookup_list().ok()?.lookups();
    Some(
        indices
            .into_iter()
            .filter_map(|i| lookups.get(usize::from(i)).ok())
            .collect(),
    )
}

/// `job` measured as Chrome measures it, per char of its text. A section
/// whose family has no face data, and a char its first face lacks, keep
/// egui's advance.
pub(super) fn advances(ctx: &egui::Context, job: &LayoutJob) -> Advances {
    let mut out = Advances::default();
    let ppp = ctx.pixels_per_point();
    for section in &job.sections {
        let format = &section.format;
        let size = format.font_id.size;
        let text = &job.text[section.byte_range.clone()];
        let egui: Vec<f32> = ctx.fonts_mut(|f| {
            text.chars()
                .map(|c| {
                    if c == '\n' {
                        0.0
                    } else {
                        f.glyph_width(&format.font_id, c)
                    }
                })
                .collect()
        });
        let data = family_face(ctx, &format.font_id.family);
        let face = data
            .as_deref()
            .and_then(|d| Face::new(d, format.coords.as_ref(), size));
        let fixed_scale = face.as_ref().map(|f| FixedScale::new(size * ppp, f.upem));
        let metrics = face
            .as_ref()
            .map(|f| f.font.glyph_metrics(Size::unscaled(), &f.location));
        let charmap = face.as_ref().map(|f| f.font.charmap());
        let gdef = face.as_ref().and_then(|f| f.font.gdef().ok());
        let store = gdef.and_then(|g| g.item_var_store()).and_then(Result::ok);
        let coords = face.as_ref().map_or(&[][..], |f| f.location.coords());
        let deltas = Deltas { store, coords };
        let fixed_of = |c: char| {
            let glyph = charmap.as_ref()?.map(c).filter(|g| *g != GlyphId::NOTDEF)?;
            Some(fixed_scale?.advance(metrics.as_ref()?.advance_width(glyph)?))
        };
        // The glyph before, where it is this face's and in this paragraph.
        let mut before: Option<GlyphId> = None;
        for (c, egui_adv) in text.chars().zip(egui) {
            if c == '\n' || ignorable(c) {
                out.chrome.push(0.0);
                out.kern.push(0.0);
                out.egui.push(0.0);
                out.fixed.push(0);
                out.fixed_kern.push(0);
                out.spacing.push(0.0);
                out.hyphen.push((c == '\u{AD}').then(|| {
                    fixed_of('\u{2010}').map_or_else(
                        || {
                            let w = fixed_of('-').unwrap_or_else(|| {
                                let w = ctx.fonts_mut(|f| f.glyph_width(&format.font_id, '-'));
                                (w * ppp * FIXED_ONE).round() as i32
                            });
                            ('-', w)
                        },
                        |w| ('\u{2010}', w),
                    )
                }));
                if c == '\n' {
                    before = None;
                }
                continue;
            }
            let glyph = (c == ' ' || !c.is_whitespace())
                .then(|| charmap.as_ref()?.map(c))
                .flatten()
                .filter(|g| *g != GlyphId::NOTDEF);
            let fixed = glyph
                .and_then(|g| Some(fixed_scale?.advance(metrics.as_ref()?.advance_width(g)?)))
                .unwrap_or_else(|| (egui_adv * ppp * FIXED_ONE).round() as i32);
            let kern = match (face.as_ref(), fixed_scale, before, glyph) {
                (Some(f), Some(s), Some(a), Some(b)) => f.kerning(a, b, &deltas, s),
                _ => 0,
            };
            let points = |v: i32| v as f32 / FIXED_ONE / ppp;
            out.chrome.push(points(fixed) + format.extra_letter_spacing);
            out.kern.push(points(kern));
            out.egui.push(egui_adv);
            out.fixed.push(fixed);
            out.fixed_kern.push(kern);
            out.spacing.push(format.extra_letter_spacing * ppp);
            out.hyphen.push(None);
            before = glyph;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advances_and_kerning_round_as_freetype_and_harfbuzz() {
        // 14 px on 1000 units: FreeType's x_scale is 896 * 65536 / 1000
        // rounded = 58720; 600 units advance 600 * 58720 / 64 = 550500,
        // a hair under the exact 8.4 px (550502.4).
        let s = FixedScale::new(14.0, 1000.0);
        assert_eq!(s.advance(600.0), 550_500);
        // A fractional advance (a variation) rounds to whole units first.
        assert_eq!(s.advance(599.6), 550_500);
        // HarfBuzz: x_mult = (14 * 65536 << 16) / 1000; -50 units scale
        // to -45875.2, rounded half up to -45875.
        assert_eq!(s.gpos(-50, 0.0), -45_875);
        assert_eq!(ceil_to_layout_unit(550_500), 538.0 / 64.0);
        assert_eq!(ceil_to_layout_unit(-1), 0.0);
    }
}
