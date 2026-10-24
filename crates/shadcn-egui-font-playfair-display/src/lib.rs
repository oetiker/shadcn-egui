//! Playfair Display for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// Playfair Display, variable weight 400–900.
pub static FONT: FontFiles = FontFiles {
    name: "playfair-display",
    kind: FontKind::Serif,
    source: FontSource::Variable {
        data: include_bytes!("../fonts/playfair-display.ttf"),
        wght: 400.0..=900.0,
    },
    y_offset_factor: 0.0,
    license: include_str!("../LICENSE-FONT.txt"),
};
