//! Fonts: the theme's font install, weights and families.

mod common;

use shadcn_egui::{FontFiles, FontSource};

fn bytes(f: &FontFiles) -> &'static [u8] {
    match &f.source {
        FontSource::Variable { data, .. } => data,
        FontSource::Static { regular, .. } => regular,
    }
}

#[test]
fn bundled_font_crates_ship_truetype_and_a_licence() {
    for f in [
        &shadcn_egui_font_inter::FONT,
        &shadcn_egui_font_oxanium::FONT,
    ] {
        assert_eq!(&bytes(f)[..4], &[0, 1, 0, 0], "{}: TrueType header", f.name);
        assert!(
            f.license.contains("SIL Open Font License"),
            "{}: OFL text",
            f.name
        );
    }
}
