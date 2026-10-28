//! One test per row of spec §4.1: the fallback result and its exact warning.
mod common;

use common::logcap::warnings;
use shadcn_egui::icons::lucide::SLOTS as LUCIDE_SLOTS;
use shadcn_egui::{FontHeading, FontName, IconLibrary, Preset, StyleName};

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
    let (t, lines) = warnings(|| Preset::default().style(StyleName::Sera).theme());
    assert_eq!((t.style.name, t.radius), ("nova", 0.0));
    assert_eq!(lines, [SERA]);
}

#[test]
fn a_font_without_its_feature_falls_back_to_inter() {
    if !geist_missing() {
        return;
    }
    let (t, lines) = warnings(|| Preset::default().font(FontName::Geist).theme());
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
            .theme()
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
    let (t, lines) = warnings(|| Preset::default().icons(IconLibrary::Tabler).theme());
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
    let (_, first) = warnings(|| p.theme());
    assert_eq!(first, [SERA, GEIST]);
    let (_, toggled) = warnings(|| {
        let _ = p.theme();
        p.theme()
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
    let (t, lines) = warnings(|| Preset::default().base_color(BaseColor::Gray).theme());
    assert_eq!(t.light, Palette::light());
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
            .theme()
    });
    assert_eq!(
        Some(t.dark),
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

/// egui has no backdrop blur, and alpha without it was rejected (user,
/// 2026-10-11): a translucent menu colour draws as its opaque variant.
#[test]
fn translucent_menu_colours_draw_opaque() {
    use shadcn_egui::MenuColor;
    for (translucent, opaque) in [
        (MenuColor::DefaultTranslucent, MenuColor::Default),
        (MenuColor::InvertedTranslucent, MenuColor::Inverted),
    ] {
        let p = Preset::default().menu_color(translucent);
        let (t, lines) = warnings(|| p.theme());
        let o = Preset::default().menu_color(opaque).theme();
        assert_eq!(
            (t.light, t.dark, t.menu_inverted),
            (o.light, o.dark, o.menu_inverted),
            "{translucent:?}"
        );
        assert_eq!(
            lines,
            [format!(
                "shadcn-egui: menu colour \"{}\" needs a backdrop blur egui lacks; using \"{}\"",
                translucent.id(),
                opaque.id()
            )]
        );
        assert_eq!(Preset::from_code(&p.to_code().expect("coded")), Ok(p));
    }
}

#[test]
fn a_supported_preset_logs_nothing() {
    let (_, lines) = warnings(|| Preset::default().theme());
    assert!(lines.is_empty(), "{lines:?}");
}

/// `Preset::apply` logs each fallback warning once per context: applying
/// the same preset again (an app that re-applies its settings, the
/// gallery after its app creator) logs nothing new, while another context
/// gets the lines again. `Preset::theme` has no context and logs on every
/// call.
#[test]
fn apply_logs_each_fallback_warning_once_per_context() {
    let sera = Preset::default().style(StyleName::Sera);
    let ctx = egui::Context::default();
    let ((), first) = warnings(|| sera.apply(&ctx));
    assert_eq!(first, [SERA]);
    let ((), again) = warnings(|| {
        sera.apply(&ctx);
        sera.apply(&ctx);
    });
    assert!(again.is_empty(), "{again:?}");
    let ((), other) = warnings(|| sera.apply(&egui::Context::default()));
    assert_eq!(other, [SERA]);
    let (_, theme) = warnings(|| sera.theme());
    assert_eq!(theme, [SERA]);
}
