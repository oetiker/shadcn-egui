//! Space Grotesk for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// Space Grotesk, variable weight 300–700.
pub static FONT: FontFiles = FontFiles {
    name: "space-grotesk",
    kind: FontKind::Sans,
    source: FontSource::Variable {
        data: include_bytes!("../fonts/space-grotesk.ttf"),
        wght: 300.0..=700.0,
    },
    y_offset_factor: 0.0,
    license: include_str!("../LICENSE-FONT.txt"),
};
