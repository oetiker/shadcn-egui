//! shadcn presets: every choice of the shadcn "create" page (style, colours,
//! fonts, icons, radius, menu) and the preset codes the site shows.
//! The enums are generated from shadcn/ui `preset.ts` (`generated.rs`, by
//! `tools/stylegen`); their order is the index inside a preset code on
//! ui.shadcn.com. Codes from shadcn-svelte.com use other lists and are not
//! supported.

// `Field`, `FIELDS_*` and `V1_CHART_COLORS` are read by `code.rs` from the
// next task on, which removes this allow.
// Ruling C5: generated, kept out of rustfmt.
#[rustfmt::skip]
#[allow(dead_code)]
mod generated;

pub use generated::{
    BaseColor, FontName, IconLibrary, MenuAccent, MenuColor, RadiusChoice, StyleName, ThemeColor,
};
