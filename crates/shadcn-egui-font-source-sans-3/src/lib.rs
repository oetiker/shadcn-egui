//! Source Sans 3 for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// Source Sans 3, variable weight 200–900.
pub static FONT: FontFiles = FontFiles {
    name: "source-sans-3",
    kind: FontKind::Sans,
    source: FontSource::Variable {
        data: include_bytes!("../fonts/source-sans-3.ttf"),
        wght: 200.0..=900.0,
    },
    y_offset_factor: 0.0,
    license: include_str!("../LICENSE-FONT.txt"),
};
