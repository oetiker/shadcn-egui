//! Roboto Slab for shadcn-egui. The font is licensed under the Apache License
//! 2.0 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// Roboto Slab, variable weight 100–900.
pub static FONT: FontFiles = FontFiles {
    name: "roboto-slab",
    kind: FontKind::Serif,
    source: FontSource::Variable {
        data: include_bytes!("../fonts/roboto-slab.ttf"),
        wght: 100.0..=900.0,
    },
    y_offset_factor: 0.0,
    license: include_str!("../LICENSE-FONT.txt"),
};
