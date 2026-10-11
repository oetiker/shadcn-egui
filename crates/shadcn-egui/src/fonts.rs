//! The font crates this build bundles, by upstream font id. Inter is always
//! bundled; every other font needs its `font-<id>` feature.

use crate::preset::FontName;
use shadcn_egui_core::FontFiles;

/// The files of `name`, or `None` when its feature is off.
pub fn files(name: FontName) -> Option<&'static FontFiles> {
    match name {
        FontName::Inter => Some(&shadcn_egui_font_inter::FONT),
        #[cfg(feature = "font-oxanium")]
        FontName::Oxanium => Some(&shadcn_egui_font_oxanium::FONT),
        _ => None,
    }
}

/// Fonts that get no crate because their licence is neither OFL-1.1 nor
/// Apache-2.0 (rows of `tools/fontgen/FONTS.tsv` whose `license` column says
/// so, P0c). No feature can bring them in.
const UNLICENSED: &[FontName] = &[];

/// The warning `Preset::theme` logs when [`files`] has nothing for `name`
/// (`heading`: the preset's heading font).
pub(crate) fn fallback_warning(name: FontName, heading: bool) -> String {
    let id = name.id();
    let what = if heading { "heading font" } else { "font" };
    if UNLICENSED.contains(&name) {
        format!("shadcn-egui: {what} \"{id}\" is not available (licence); using \"inter\"")
    } else {
        format!("shadcn-egui: {what} \"{id}\" needs the \"font-{id}\" feature; using \"inter\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `UNLICENSED` lists exactly the FONTS.tsv rows fontgen skips.
    #[test]
    fn unlicensed_matches_fonts_tsv() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/fontgen/FONTS.tsv");
        let tsv = std::fs::read_to_string(path).expect("FONTS.tsv");
        let mut expected: Vec<FontName> = tsv
            .lines()
            .skip(1)
            .filter_map(|l| {
                let cols: Vec<&str> = l.split('\t').collect();
                (!matches!(cols[3], "OFL-1.1" | "Apache-2.0"))
                    .then(|| FontName::from_id(cols[0]).expect(cols[0]))
            })
            .collect();
        expected.sort_by_key(|f| f.id());
        let mut listed = UNLICENSED.to_vec();
        listed.sort_by_key(|f| f.id());
        assert_eq!(listed, expected);
    }
}
