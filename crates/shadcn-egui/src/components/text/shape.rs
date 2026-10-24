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
use skrifa::raw::tables::gpos::{PairPos, PositionLookup, PositionSubtables, ValueContext};
use skrifa::raw::{FontRef, TableProvider};
use skrifa::{GlyphId, MetadataProvider, Tag};
use std::sync::Arc;

/// Per char of a job's text: Chrome's advance in points (letter spacing
/// included), the kerning between the char before and it (0 at a
/// paragraph start and next to a char the face lacks), and egui's own
/// advance. A `\n` has 0 for all three.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Advances {
    pub chrome: Vec<f32>,
    pub kern: Vec<f32>,
    pub egui: Vec<f32>,
}

/// The first face of `family` in the context's font definitions.
pub(super) fn family_face(ctx: &egui::Context, family: &FontFamily) -> Option<Arc<FontData>> {
    ctx.fonts(|f| {
        let defs = f.definitions();
        let key = defs.families.get(family)?.first()?;
        defs.font_data.get(key).cloned()
    })
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
    fn kerning(&self, a: GlyphId, b: GlyphId, context: &ValueContext) -> f32 {
        let mut total = 0.0;
        for lookup in &self.kern {
            let Ok(PositionSubtables::Pair(subtables)) = lookup.subtables() else {
                continue;
            };
            for sub in subtables.iter().flatten() {
                if let Some(v) = pair_value(&sub, a, b, context) {
                    total += v;
                    break;
                }
            }
        }
        total
    }
}

/// The x advance adjustment of the first glyph for the pair `a`, `b` in
/// one pair subtable, `None` where the subtable does not apply.
fn pair_value(sub: &PairPos, a: GlyphId, b: GlyphId, context: &ValueContext) -> Option<f32> {
    match sub {
        PairPos::Format1(t) => {
            let index = t.coverage().ok()?.get(a)?;
            let set = t.pair_sets().get(usize::from(index)).ok()?;
            let records = set.pair_value_records();
            let record = records
                .iter()
                .flatten()
                .find(|r| GlyphId::from(r.second_glyph()) == b)?;
            let v = record
                .value_record1()
                .value(set.offset_data(), context)
                .ok()?;
            Some(f32::from(v.x_advance) + v.x_advance_delta as f32)
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
            let v = record
                .value_record1()
                .value(t.offset_data(), context)
                .ok()?;
            Some(f32::from(v.x_advance) + v.x_advance_delta as f32)
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
        let scale = face.as_ref().map_or(0.0, |f| size / f.upem);
        let metrics = face
            .as_ref()
            .map(|f| f.font.glyph_metrics(Size::unscaled(), &f.location));
        let charmap = face.as_ref().map(|f| f.font.charmap());
        let gdef = face.as_ref().and_then(|f| f.font.gdef().ok());
        let store = gdef.and_then(|g| g.item_var_store()).and_then(Result::ok);
        let coords = face.as_ref().map_or(&[][..], |f| f.location.coords());
        let context = ValueContext::new()
            .with_coords(coords)
            .with_var_store(store);
        // The glyph before, where it is this face's and in this paragraph.
        let mut before: Option<GlyphId> = None;
        for (c, egui_adv) in text.chars().zip(egui) {
            if c == '\n' {
                out.chrome.push(0.0);
                out.kern.push(0.0);
                out.egui.push(0.0);
                before = None;
                continue;
            }
            let glyph = (c == ' ' || !c.is_whitespace())
                .then(|| charmap.as_ref()?.map(c))
                .flatten()
                .filter(|g| *g != GlyphId::NOTDEF);
            let width = glyph.and_then(|g| Some(metrics.as_ref()?.advance_width(g)? * scale));
            let kern = match (face.as_ref(), before, glyph) {
                (Some(f), Some(a), Some(b)) => f.kerning(a, b, &context) * scale,
                _ => 0.0,
            };
            out.chrome
                .push(width.unwrap_or(egui_adv) + format.extra_letter_spacing);
            out.kern.push(kern);
            out.egui.push(egui_adv);
            before = glyph;
        }
    }
    out
}
