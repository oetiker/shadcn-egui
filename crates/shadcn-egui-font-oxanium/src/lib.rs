//! Oxanium for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// Oxanium, variable weight 200–800.
pub static FONT: FontFiles = FontFiles {
    name: "oxanium",
    kind: FontKind::Sans,
    source: FontSource::Variable {
        data: include_bytes!("../fonts/oxanium.ttf"),
        wght: 200.0..=800.0,
    },
    y_offset_factor: 0.14,
    license: include_str!("../LICENSE-FONT.txt"),
};
