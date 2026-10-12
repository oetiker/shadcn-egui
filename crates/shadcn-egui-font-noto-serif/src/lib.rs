//! Noto Serif for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// Noto Serif, variable weight 100–900.
pub static FONT: FontFiles = FontFiles {
    name: "noto-serif",
    kind: FontKind::Serif,
    source: FontSource::Variable {
        data: include_bytes!("../fonts/noto-serif.ttf"),
        wght: 100.0..=900.0,
    },
    y_offset_factor: -0.07,
    license: include_str!("../LICENSE-FONT.txt"),
};
