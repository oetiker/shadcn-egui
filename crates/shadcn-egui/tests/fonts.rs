//! Fonts: the theme's font install, weights and families.

mod common;

use egui::{vec2, Color32, FontData, FontDefinitions, FontFamily, RichText};
use egui_kittest::Harness;
use shadcn_egui::style::Weight;
use shadcn_egui::theme::{
    family_for, heading_family_for, FAMILY_BOLD, FAMILY_HEADING, FAMILY_HEADING_BOLD,
    FAMILY_HEADING_MEDIUM, FAMILY_HEADING_SEMIBOLD, FAMILY_MEDIUM, FAMILY_SEMIBOLD,
};
use shadcn_egui::{FontKind, Theme};
use std::sync::Arc;

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

const OXANIUM: &shadcn_egui::FontFiles = &shadcn_egui_font_oxanium::FONT;

/// Ink (sum of 255 - green) of a word drawn black on white in `family`.
fn ink(theme: &Theme, family: fn(&egui::Context) -> FontFamily) -> u64 {
    let mut applied = false;
    let theme = theme.clone();
    let mut h = Harness::builder()
        .with_size(vec2(560.0, 90.0))
        .build_ui(move |ui| {
            if !applied {
                theme.apply(ui.ctx());
                applied = true;
            }
            ui.painter()
                .rect_filled(ui.ctx().viewport_rect(), 0.0, Color32::WHITE);
            ui.label(
                RichText::new("Hamburgefonstiv")
                    .family(family(ui.ctx()))
                    .size(40.0)
                    .color(Color32::BLACK),
            );
        });
    for _ in 0..3 {
        h.step();
    }
    let img = h.render().expect("wgpu render");
    img.pixels().map(|p| u64::from(255 - p.0[1])).sum()
}

#[test]
fn each_weight_has_more_ink_than_the_one_below() {
    let t = Theme::vega_light();
    let inks = [
        ink(&t, |c| family_for(c, Weight::Normal)),
        ink(&t, |c| family_for(c, Weight::Medium)),
        ink(&t, |c| family_for(c, Weight::Semibold)),
        ink(&t, |c| family_for(c, Weight::Bold)),
    ];
    assert!(
        inks.windows(2).all(|w| w[1] > w[0]),
        "ink must grow with weight: {inks:?}"
    );
}

fn definitions_after_apply(theme: &Theme) -> FontDefinitions {
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    ctx.fonts(|f| f.definitions().clone())
}

#[test]
fn every_named_family_falls_back_to_egui_fonts() {
    let defs = definitions_after_apply(&Theme::vega_dark());
    let egui_proportional = FontDefinitions::default().families[&FontFamily::Proportional].clone();
    for name in [
        FAMILY_MEDIUM,
        FAMILY_SEMIBOLD,
        FAMILY_BOLD,
        FAMILY_HEADING,
        FAMILY_HEADING_MEDIUM,
        FAMILY_HEADING_SEMIBOLD,
        FAMILY_HEADING_BOLD,
    ] {
        let list = &defs.families[&FontFamily::Name(name.into())];
        assert_eq!(
            &list[1..],
            &egui_proportional[..],
            "{name}: egui's fonts follow the theme font"
        );
    }
    assert_eq!(
        defs.families[&FontFamily::Proportional][0],
        "shadcn-body-400"
    );
    assert_eq!(
        &defs.families[&FontFamily::Proportional][1..],
        &egui_proportional[..]
    );
}

#[test]
fn inherit_maps_heading_families_to_the_body_font() {
    let defs = definitions_after_apply(&Theme::vega_dark());
    assert_eq!(
        defs.families[&FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into())][0],
        "shadcn-body-600"
    );
    assert!(
        !defs.font_data.contains_key("shadcn-heading-600"),
        "no second copy for an inherited heading"
    );
}

#[test]
fn a_heading_font_gets_its_own_faces() {
    let defs = definitions_after_apply(
        &Theme::vega_dark().fonts(&shadcn_egui_font_inter::FONT, Some(OXANIUM)),
    );
    assert_eq!(
        defs.families[&FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into())][0],
        "shadcn-heading-600"
    );
    assert_eq!(
        defs.font_data["shadcn-heading-600"].font.len(),
        OXANIUM.face(600).data.len()
    );
}

#[test]
fn monospace_takes_the_body_font_only_when_it_is_mono() {
    let defs = definitions_after_apply(&Theme::vega_dark());
    assert_eq!(
        defs.families[&FontFamily::Monospace][0],
        "Hack",
        "a sans body leaves egui's mono font"
    );
    let mono: &'static shadcn_egui::FontFiles = Box::leak(Box::new(shadcn_egui::FontFiles {
        name: "fake-mono",
        kind: FontKind::Mono,
        ..shadcn_egui_font_inter::FONT.clone()
    }));
    let defs = definitions_after_apply(&Theme::vega_dark().fonts(mono, None));
    assert_eq!(defs.families[&FontFamily::Monospace][0], "shadcn-body-400");
}

#[test]
fn heading_text_style_uses_the_heading_semibold_family() {
    let ctx = egui::Context::default();
    Theme::vega_dark().apply(&ctx);
    let id = &ctx.global_style().text_styles[&egui::TextStyle::Heading];
    assert_eq!(id.family, FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into()));
}

/// Marks the installed definitions so a rebuild is visible: a rebuild drops the marker.
fn mark(ctx: &egui::Context) {
    let mut defs = ctx.fonts(|f| f.definitions().clone());
    defs.font_data.insert(
        "marker".into(),
        Arc::new(FontData::from_static(OXANIUM.face(400).data)),
    );
    ctx.set_fonts(defs);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
}

fn has_marker(ctx: &egui::Context) -> bool {
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    ctx.fonts(|f| f.definitions().font_data.contains_key("marker"))
}

#[test]
fn a_mode_toggle_does_not_reinstall_fonts() {
    let ctx = egui::Context::default();
    Theme::vega_light().apply(&ctx);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    mark(&ctx);
    Theme::vega_dark().apply(&ctx);
    assert!(has_marker(&ctx), "same fonts: the atlas is not rebuilt");
    Theme::vega_dark()
        .fonts(&shadcn_egui_font_inter::FONT, Some(OXANIUM))
        .apply(&ctx);
    assert!(!has_marker(&ctx), "new heading font: fonts are reinstalled");
}

#[test]
fn apply_with_fonts_keeps_the_apps_fonts() {
    let ctx = egui::Context::default();
    let mut base = FontDefinitions::default();
    base.font_data.insert(
        "app-font".into(),
        Arc::new(FontData::from_static(OXANIUM.face(400).data)),
    );
    base.families
        .insert(FontFamily::Name("app".into()), vec!["app-font".into()]);
    Theme::vega_light().apply_with_fonts(&ctx, base);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    let defs = ctx.fonts(|f| f.definitions().clone());
    assert!(defs.families.contains_key(&FontFamily::Name("app".into())));
    assert!(defs
        .families
        .contains_key(&FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into())));
}

/// Review Focus 2: an app that manages its fonts toggles light/dark with `apply`.
#[test]
fn app_fonts_survive_a_mode_toggle() {
    let ctx = egui::Context::default();
    let mut base = FontDefinitions::default();
    base.font_data.insert(
        "app-font".into(),
        Arc::new(FontData::from_static(OXANIUM.face(400).data)),
    );
    Theme::vega_light().apply_with_fonts(&ctx, base);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    Theme::vega_dark().apply(&ctx);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    assert!(ctx.fonts(|f| f.definitions().font_data.contains_key("app-font")));
}

/// Review Focus 5: switching fonts on a live context, with a heading drawn every frame.
#[test]
fn switching_fonts_on_a_live_context_does_not_panic() {
    use shadcn_egui::components::card::{card_header, Card};
    let frame = std::cell::Cell::new(0);
    let mut h = Harness::builder()
        .with_size(vec2(360.0, 160.0))
        .build_ui(|ui| {
            let n = frame.get();
            if n == 0 {
                Theme::vega_dark().apply(ui.ctx());
            }
            if n == 2 {
                Theme::vega_dark()
                    .fonts(&shadcn_egui_font_inter::FONT, Some(OXANIUM))
                    .apply(ui.ctx());
            }
            frame.set(n + 1);
            Card::new().show(ui, |ui| card_header(ui, "Title", "Description"));
            let _ = heading_family_for(ui.ctx(), Weight::Semibold);
            ui.heading("egui heading");
        });
    for _ in 0..6 {
        h.step();
    }
    let defs = h.ctx.fonts(|f| f.definitions().clone());
    assert_eq!(
        defs.families[&FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into())][0],
        "shadcn-heading-600"
    );
}
