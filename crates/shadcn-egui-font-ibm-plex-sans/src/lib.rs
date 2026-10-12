//! IBM Plex Sans for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// IBM Plex Sans, variable weight 100–700.
pub static FONT: FontFiles = FontFiles {
    name: "ibm-plex-sans",
    kind: FontKind::Sans,
    source: FontSource::Variable {
        data: include_bytes!("../fonts/ibm-plex-sans.ttf"),
        wght: 100.0..=700.0,
    },
    y_offset_factor: 0.0,
    license: include_str!("../LICENSE-FONT.txt"),
};
