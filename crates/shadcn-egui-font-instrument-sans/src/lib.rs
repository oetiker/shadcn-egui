//! Instrument Sans for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// Instrument Sans, variable weight 400–700.
pub static FONT: FontFiles = FontFiles {
    name: "instrument-sans",
    kind: FontKind::Sans,
    source: FontSource::Variable {
        data: include_bytes!("../fonts/instrument-sans.ttf"),
        wght: 400.0..=700.0,
    },
    y_offset_factor: -0.07,
    license: include_str!("../LICENSE-FONT.txt"),
};
