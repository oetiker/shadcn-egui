//! JetBrains Mono for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// JetBrains Mono, variable weight 100–800.
pub static FONT: FontFiles = FontFiles {
    name: "jetbrains-mono",
    kind: FontKind::Mono,
    source: FontSource::Variable {
        data: include_bytes!("../fonts/jetbrains-mono.ttf"),
        wght: 100.0..=800.0,
    },
    y_offset_factor: 0.0,
    license: include_str!("../LICENSE-FONT.txt"),
};
