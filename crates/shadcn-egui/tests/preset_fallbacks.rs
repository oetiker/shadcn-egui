//! One test per row of spec §4.1: the fallback result and its exact warning.
mod common;

use common::logcap::warnings;
use shadcn_egui::components::icon::LUCIDE_SLOTS;
use shadcn_egui::{FontHeading, FontName, IconLibrary, Mode, Preset, StyleName};

/// The font and icon library these tests ask for, unless a feature bundles
/// them (then the fallback cannot happen and the test has nothing to check).
fn geist_missing() -> bool {
    let missing = shadcn_egui::fonts::files(FontName::Geist).is_none();
    if !missing {
        eprintln!("skipped: font-geist is enabled");
    }
    missing
}

fn tabler_missing() -> bool {
    let missing = shadcn_egui::icons::slots(IconLibrary::Tabler).is_none();
    if !missing {
        eprintln!("skipped: icons-tabler is enabled");
    }
    missing
}

const SERA: &str = "shadcn-egui: style \"sera\" is not available in egui; using \"nova\"";
const GEIST: &str = "shadcn-egui: font \"geist\" needs the \"font-geist\" feature; using \"inter\"";

#[test]
fn sera_falls_back_to_nova_with_square_corners() {
    let (t, lines) = warnings(|| Preset::default().style(StyleName::Sera).theme(Mode::Dark));
    assert_eq!((t.style.name, t.radius), ("nova", 0.0));
    assert_eq!(lines, [SERA]);
}

#[test]
fn a_font_without_its_feature_falls_back_to_inter() {
    if !geist_missing() {
        return;
    }
    let (t, lines) = warnings(|| Preset::default().font(FontName::Geist).theme(Mode::Light));
    assert_eq!(t.fonts.body.name, "inter");
    assert_eq!(lines, [GEIST]);
}

#[test]
fn a_heading_font_without_its_feature_falls_back_to_inter() {
    if !geist_missing() {
        return;
    }
    let (t, lines) = warnings(|| {
        Preset::default()
            .font_heading(FontHeading::Font(FontName::Geist))
            .theme(Mode::Light)
    });
    assert_eq!(t.fonts.heading.name, "inter");
    assert_eq!(
        lines,
        ["shadcn-egui: heading font \"geist\" needs the \"font-geist\" feature; using \"inter\""]
    );
}

#[test]
fn an_icon_library_without_its_feature_falls_back_to_lucide() {
    if !tabler_missing() {
        return;
    }
    let (t, lines) = warnings(|| {
        Preset::default()
            .icons(IconLibrary::Tabler)
            .theme(Mode::Light)
    });
    assert_eq!(t.icons, LUCIDE_SLOTS);
    assert_eq!(
        lines,
        ["shadcn-egui: icon library \"tabler\" needs the \"icons-tabler\" feature; using \"lucide\""]
    );
}

#[test]
fn two_unsupported_values_log_two_lines_and_every_call_repeats_them() {
    if !geist_missing() {
        return;
    }
    let p = Preset::default()
        .style(StyleName::Sera)
        .font(FontName::Geist);
    let (_, first) = warnings(|| p.theme(Mode::Light));
    assert_eq!(first, [SERA, GEIST]);
    let (_, toggled) = warnings(|| {
        let _ = p.theme(Mode::Light);
        p.theme(Mode::Dark)
    });
    // theme() builds the palette twice when the menu is inverted; here it is not.
    assert_eq!(
        toggled,
        [SERA, GEIST, SERA, GEIST],
        "each theme() call warns again"
    );
}

/// `gray` is a code value on ui.shadcn.com without a palette there (index §J).
#[test]
fn gray_falls_back_to_neutral_and_the_base() {
    use shadcn_egui::{BaseColor, Palette, ThemeColor};
    let (t, lines) = warnings(|| {
        Preset::default()
            .base_color(BaseColor::Gray)
            .theme(Mode::Light)
    });
    assert_eq!(t.palette, Palette::light());
    assert_eq!(t.radius, 10.0);
    assert_eq!(
        lines,
        ["shadcn-egui: base colour \"gray\" has no palette on ui.shadcn.com; using \"neutral\""]
    );
    let (t, lines) = warnings(|| {
        Preset::default()
            .base_color(BaseColor::Stone)
            .theme_color(ThemeColor::Gray)
            .chart_color(ThemeColor::Gray)
            .theme(Mode::Dark)
    });
    assert_eq!(
        Some(t.palette),
        Palette::from_upstream("stone", "stone", "stone", true)
    );
    assert_eq!(
        lines,
        [
            "shadcn-egui: theme \"gray\" has no palette on ui.shadcn.com; using \"stone\"",
            "shadcn-egui: chart colour \"gray\" has no palette on ui.shadcn.com; using \"stone\"",
        ]
    );
}

#[test]
fn a_supported_preset_logs_nothing() {
    let (_, lines) = warnings(|| Preset::default().theme(Mode::Dark));
    assert!(lines.is_empty(), "{lines:?}");
}
