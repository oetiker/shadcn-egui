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
