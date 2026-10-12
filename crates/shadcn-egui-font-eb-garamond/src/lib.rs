//! EB Garamond for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// EB Garamond, variable weight 400–800.
pub static FONT: FontFiles = FontFiles {
    name: "eb-garamond",
    kind: FontKind::Serif,
    source: FontSource::Variable {
        data: include_bytes!("../fonts/eb-garamond.ttf"),
        wght: 400.0..=800.0,
    },
    y_offset_factor: 0.0,
    license: include_str!("../LICENSE-FONT.txt"),
};
