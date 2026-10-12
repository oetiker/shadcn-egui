use shadcn_egui::{
    BaseColor, FontHeading, FontName, IconLibrary, MenuAccent, MenuColor, MenuColors, Mode,
    Palette, Preset, RadiusChoice, StyleName, Theme, ThemeColor,
};

#[test]
fn enums_follow_upstream_list_order() {
    assert_eq!(
        StyleName::ALL.iter().map(|s| s.id()).collect::<Vec<_>>(),
        ["nova", "vega", "maia", "lyra", "mira", "luma", "sera", "rhea"]
    );
    assert_eq!(
        BaseColor::ALL.iter().map(|b| b.id()).collect::<Vec<_>>(),
        ["neutral", "stone", "zinc", "gray", "mauve", "olive", "mist", "taupe"]
    );
    assert_eq!(ThemeColor::ALL.len(), 25);
    assert_eq!(
        (ThemeColor::ALL[3], ThemeColor::ALL[4], ThemeColor::ALL[24]),
        (ThemeColor::Gray, ThemeColor::Amber, ThemeColor::Taupe)
    );
    assert_eq!(FontName::ALL.len(), 26);
    assert_eq!(
        (FontName::ALL[0], FontName::ALL[17], FontName::ALL[25]),
        (
            FontName::Inter,
            FontName::Oxanium,
            FontName::InstrumentSerif
        )
    );
    assert_eq!(FontName::SourceSans3.id(), "source-sans-3");
    assert_eq!(
        IconLibrary::ALL.iter().map(|l| l.id()).collect::<Vec<_>>(),
        ["lucide", "hugeicons", "tabler", "phosphor", "remixicon"]
    );
    assert_eq!(
        MenuColor::ALL.iter().map(|m| m.id()).collect::<Vec<_>>(),
        [
            "default",
            "inverted",
            "default-translucent",
            "inverted-translucent"
        ]
    );
    assert_eq!(
        MenuAccent::ALL.iter().map(|m| m.id()).collect::<Vec<_>>(),
        ["subtle", "bold"]
    );
    assert_eq!(
        RadiusChoice::ALL
            .iter()
            .map(|r| r.rem())
            .collect::<Vec<_>>(),
        [None, Some(0.0), Some(0.45), Some(0.625), Some(0.875)]
    );
}

#[test]
fn ids_round_trip_and_unknown_ids_are_none() {
    for &v in StyleName::ALL {
        assert_eq!(StyleName::from_id(v.id()), Some(v));
    }
    for &v in BaseColor::ALL {
        assert_eq!(BaseColor::from_id(v.id()), Some(v));
    }
    for &v in ThemeColor::ALL {
        assert_eq!(ThemeColor::from_id(v.id()), Some(v));
    }
    for &v in FontName::ALL {
        assert_eq!(FontName::from_id(v.id()), Some(v));
    }
    for &v in IconLibrary::ALL {
        assert_eq!(IconLibrary::from_id(v.id()), Some(v));
    }
    for &v in MenuColor::ALL {
        assert_eq!(MenuColor::from_id(v.id()), Some(v));
    }
    for &v in MenuAccent::ALL {
        assert_eq!(MenuAccent::from_id(v.id()), Some(v));
    }
    for &v in RadiusChoice::ALL {
        assert_eq!(RadiusChoice::from_id(v.id().expect("coded")), Some(v));
    }
    assert_eq!(
        (RadiusChoice::Px(3.0).id(), RadiusChoice::Px(3.0).rem()),
        (None, None)
    );
    assert_eq!(
        StyleName::from_id("Vega"),
        None,
        "ids are case sensitive, as upstream"
    );
    assert_eq!(FontName::from_id(""), None);
    assert_eq!(RadiusChoice::from_id("px"), None);
}

#[test]
fn default_is_the_sites_default_config_and_code_b0() {
    let p = Preset::default();
    assert_eq!(
        p,
        Preset {
            style: StyleName::Nova,
            base_color: BaseColor::Neutral,
            theme: ThemeColor::Neutral,
            chart_color: ThemeColor::Neutral,
            font: FontName::Inter,
            font_heading: FontHeading::Inherit,
            icons: IconLibrary::Lucide,
            radius: RadiusChoice::Default,
            menu_color: MenuColor::Default,
            menu_accent: MenuAccent::Subtle,
        }
    );
    // ui.shadcn.com DEFAULT_CONFIG is all indices 0 (the svelte site's was Vega).
    assert_eq!(Preset::from_code("b0"), Ok(p));
    assert_eq!(p.to_code().as_deref(), Some("b0"));
    assert_eq!(Preset::from_code(&p.to_code().expect("coded")), Ok(p));
}

#[test]
fn each_setter_changes_its_field() {
    let p = Preset::default()
        .style(StyleName::Vega)
        .base_color(BaseColor::Stone)
        .theme_color(ThemeColor::Blue)
        .chart_color(ThemeColor::Rose)
        .font(FontName::Geist)
        .font_heading(FontHeading::Font(FontName::Lora))
        .icons(IconLibrary::Tabler)
        .radius(RadiusChoice::Large)
        .menu_color(MenuColor::Inverted)
        .menu_accent(MenuAccent::Bold);
    assert_eq!(
        p,
        Preset {
            style: StyleName::Vega,
            base_color: BaseColor::Stone,
            theme: ThemeColor::Blue,
            chart_color: ThemeColor::Rose,
            font: FontName::Geist,
            font_heading: FontHeading::Font(FontName::Lora),
            icons: IconLibrary::Tabler,
            radius: RadiusChoice::Large,
            menu_color: MenuColor::Inverted,
            menu_accent: MenuAccent::Bold,
        }
    );
    assert_eq!(Preset::from_code(&p.to_code().expect("coded")), Ok(p));
    assert_eq!(
        Preset::default().radius(RadiusChoice::Px(6.0)).to_code(),
        None
    );
    assert_ne!(Mode::Light, Mode::Dark);
}

#[test]
fn lyra_forces_radius_zero() {
    assert_eq!(
        Preset::default()
            .style(StyleName::Lyra)
            .radius(RadiusChoice::Large)
            .theme(Mode::Light)
            .radius,
        0.0
    );
    assert_eq!(
        Preset::default()
            .style(StyleName::Lyra)
            .radius(RadiusChoice::Px(7.0))
            .theme(Mode::Light)
            .radius,
        0.0
    );
}

#[test]
fn radius_choices_resolve_to_px() {
    let r = |c| Preset::default().radius(c).theme(Mode::Dark).radius;
    assert_eq!(r(RadiusChoice::Default), 10.0, "the base colour's 0.625rem");
    assert_eq!(r(RadiusChoice::None), 0.0);
    assert_eq!(r(RadiusChoice::Small), 0.45 * 16.0);
    assert_eq!(r(RadiusChoice::Large), 0.875 * 16.0);
    assert_eq!(r(RadiusChoice::Px(7.5)), 7.5);
}

#[test]
fn default_preset_is_nova_with_the_hand_written_palettes() {
    let dark = Preset::default().theme(Mode::Dark);
    assert_eq!(
        (dark.style.name, dark.palette, dark.dark),
        ("nova", Palette::dark(), true)
    );
    let light = Preset::default().theme(Mode::Light);
    assert_eq!((light.palette, light.dark), (Palette::light(), false));
    assert_eq!(light.fonts.body.name, "inter");
    assert_eq!(light.menu, MenuColors::default());
}

#[test]
fn colours_merge_like_upstream() {
    let t = Preset::default()
        .base_color(BaseColor::Stone)
        .theme_color(ThemeColor::Blue)
        .chart_color(ThemeColor::Rose)
        .theme(Mode::Light);
    assert_eq!(
        Some(t.palette),
        Palette::from_upstream("stone", "blue", "rose", false)
    );
}

#[test]
fn bold_accent_highlights_with_primary() {
    let t = Preset::default()
        .theme_color(ThemeColor::Blue)
        .menu_accent(MenuAccent::Bold)
        .theme(Mode::Light);
    assert_eq!(
        (t.palette.accent, t.palette.accent_foreground),
        (t.palette.primary, t.palette.primary_foreground)
    );
}

/// ui.shadcn.com turns Bold into Subtle for translucent menus, in the main
/// palette and in the inverted menu palette; silent (the only warning is
/// the menu colour's own), and the code keeps Bold.
#[test]
fn translucent_menus_force_the_subtle_accent() {
    let bold = Preset::default()
        .theme_color(ThemeColor::Blue)
        .menu_accent(MenuAccent::Bold);
    let subtle = Preset::default().theme_color(ThemeColor::Blue);
    for menu in [
        MenuColor::DefaultTranslucent,
        MenuColor::InvertedTranslucent,
    ] {
        let t = bold.menu_color(menu).theme(Mode::Light);
        let s = subtle.menu_color(menu).theme(Mode::Light);
        assert_eq!((t.palette, t.menu), (s.palette, s.menu), "{menu:?}");
        assert_ne!(t.palette.accent, t.palette.primary, "{menu:?}");
        let p = bold.menu_color(menu);
        assert_eq!(
            Preset::from_code(&p.to_code().expect("coded")),
            Ok(p),
            "to_code keeps Bold"
        );
    }
    let solid = bold.menu_color(MenuColor::Inverted).theme(Mode::Light);
    assert_eq!(
        solid.palette.accent, solid.palette.primary,
        "solid menus keep the bold accent"
    );
}

#[test]
fn inverted_menus_use_the_dark_palette_and_flag() {
    let p = Preset::default()
        .theme_color(ThemeColor::Blue)
        .menu_accent(MenuAccent::Bold);
    let dark_palette = p.theme(Mode::Dark).palette;
    let t = p.menu_color(MenuColor::Inverted).theme(Mode::Light);
    assert_eq!(
        t.menu,
        MenuColors {
            palette: Some(dark_palette),
            dark: Some(true),
        }
    );
    assert!(!t.dark);
}

#[test]
fn a_heading_font_resolves() {
    let t = Preset::default()
        .font_heading(FontHeading::Font(FontName::Inter))
        .theme(Mode::Light);
    assert_eq!(
        (t.fonts.body.name, t.fonts.heading.name),
        ("inter", "inter")
    );
}

/// Review Focus 3: a value that falls back still encodes as requested.
#[test]
fn to_code_keeps_values_that_fall_back() {
    let p = Preset::default()
        .style(StyleName::Sera)
        .font(FontName::Geist)
        .menu_color(MenuColor::InvertedTranslucent);
    // falls back to Nova / Inter / an opaque inverted menu, the preset stays as it is
    let _ = p.theme(Mode::Dark);
    assert_eq!(Preset::from_code(&p.to_code().expect("coded")), Ok(p));
}

/// Every id has a generated palette except `gray`, which ui.shadcn.com
/// codes can name but its `themes.ts` lacks (index §J); `theme()` still
/// resolves every combination.
#[test]
fn every_palette_id_resolves() {
    for &b in BaseColor::ALL {
        for &c in ThemeColor::ALL {
            let has = Palette::from_upstream(b.id(), c.id(), c.id(), false).is_some();
            assert_eq!(
                has,
                b != BaseColor::Gray && c != ThemeColor::Gray,
                "{b:?} {c:?}"
            );
            let _ = Preset::default()
                .base_color(b)
                .theme_color(c)
                .chart_color(c)
                .theme(Mode::Light);
        }
    }
}

#[test]
fn apply_installs_the_theme() {
    let ctx = egui::Context::default();
    Preset::default().apply(&ctx, Mode::Light);
    assert_eq!(Theme::current(&ctx).palette, Palette::light());
}

/// A `Style` is ~100 KB in a debug build; resolving a preset must not hold
/// two of them, so every style resolves on a 256 KB thread.
#[test]
fn preset_theme_builds_on_a_small_stack() {
    let built = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            for &s in StyleName::ALL {
                let _ = Preset::default().style(s).theme(Mode::Dark);
            }
            Preset::default().theme(Mode::Dark).style.name
        })
        .expect("spawn")
        .join();
    assert_eq!(built.ok(), Some("nova"));
}

#[test]
fn rhea_turns_radius_large_into_the_base_radius() {
    let rhea = Preset::default().style(StyleName::Rhea);
    assert_eq!(
        rhea.radius(RadiusChoice::Large).theme(Mode::Light).radius,
        10.0,
        "Large → the base colour's 0.625rem"
    );
    assert_eq!(
        rhea.base_color(BaseColor::Stone)
            .radius(RadiusChoice::Large)
            .theme(Mode::Dark)
            .radius,
        10.0
    );
    assert_eq!(
        rhea.radius(RadiusChoice::Small).theme(Mode::Light).radius,
        0.45 * 16.0,
        "only Large changes"
    );
    assert_eq!(
        rhea.radius(RadiusChoice::None).theme(Mode::Light).radius,
        0.0
    );
    assert_eq!(
        rhea.radius(RadiusChoice::Px(14.0))
            .theme(Mode::Light)
            .radius,
        14.0,
        "Px is not a coded Large"
    );
    assert_eq!(
        Preset::default()
            .style(StyleName::Vega)
            .radius(RadiusChoice::Large)
            .theme(Mode::Light)
            .radius,
        0.875 * 16.0,
        "other styles keep Large"
    );
    let p = rhea.radius(RadiusChoice::Large);
    assert_eq!(
        Preset::from_code(&p.to_code().expect("coded")),
        Ok(p),
        "to_code keeps the requested Large"
    );
}
