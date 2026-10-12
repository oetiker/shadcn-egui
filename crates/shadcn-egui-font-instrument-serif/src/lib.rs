//! Instrument Serif for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// Instrument Serif, static weights 400.
pub static FONT: FontFiles = FontFiles {
    name: "instrument-serif",
    kind: FontKind::Serif,
    source: FontSource::Static {
        regular: include_bytes!("../fonts/instrument-serif-400.ttf"),
        medium: None,
        semibold: None,
        bold: None,
    },
    y_offset_factor: 0.07,
    license: include_str!("../LICENSE-FONT.txt"),
};
