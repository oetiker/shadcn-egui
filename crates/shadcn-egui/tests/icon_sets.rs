mod common;

use egui::vec2;
use shadcn_egui::components::button::{Button, ButtonSize, ButtonVariant};
use shadcn_egui::components::checkbox::Checkbox;
use shadcn_egui::components::icon::IconView;
use shadcn_egui::components::select::Select;
use shadcn_egui::style::Style;
use shadcn_egui::{icons, layout, Icon, IconLayer, IconLibrary, Mode, Palette, Preset, Seg, Theme};

/// Every icon of a set decodes to something drawable with in-range indices,
/// an opaque layer first and translucent ones after it.
fn every_icon_decodes(all: &[&Icon]) {
    assert!(!all.is_empty());
    for icon in all {
        assert_eq!(icon.size, 24.0, "{}", icon.name);
        let layers: Vec<IconLayer> = icon.layers().collect();
        assert_eq!(layers.first().map(|l| l.alpha), Some(1.0), "{}", icon.name);
        assert!(
            layers[1..].iter().all(|l| l.alpha > 0.0 && l.alpha < 1.0),
            "{}: layer alpha",
            icon.name
        );
        assert!(
            layers
                .iter()
                .any(|l| l.segs.clone().count() > 0 || l.fill.is_some()),
            "{} draws nothing",
            icon.name
        );
        for f in layers.iter().filter_map(|l| l.fill.as_ref()) {
            assert_eq!(f.indices.len() % 3, 0, "{}", icon.name);
            assert!(
                f.indices.iter().all(|&i| usize::from(i) < f.vertices.len()),
                "{}: index out of range",
                icon.name
            );
        }
    }
}

/// The opaque layer's fill.
#[cfg(any(feature = "icons-phosphor", feature = "icons-remixicon"))]
fn fill(icon: &Icon) -> Option<shadcn_egui::FillMesh> {
    icon.layers().next().and_then(|l| l.fill)
}

#[test]
fn lucide_is_the_default_icon_set() {
    assert_eq!(
        Theme::new(Style::vega(), Palette::light(), 10.0).icons,
        icons::lucide::SLOTS
    );
    assert_eq!(
        icons::slots(IconLibrary::Lucide),
        Some(icons::lucide::SLOTS)
    );
    let s = icons::lucide::SLOTS;
    assert_eq!(
        (s.check.name, s.chevron_down.name, s.close.name),
        ("check", "chevron-down", "x")
    );
}

#[test]
fn lucide_slots_keep_the_built_in_paths() {
    let segs = |i: &Icon| i.layers().flat_map(|l| l.segs).collect::<Vec<_>>();
    assert_eq!(
        segs(&icons::lucide::CHECK),
        vec![Seg::M(20.0, 6.0), Seg::L(9.0, 17.0), Seg::L(4.0, 12.0)]
    );
    assert_eq!(
        segs(&icons::lucide::CHEVRON_DOWN),
        vec![Seg::M(6.0, 9.0), Seg::L(12.0, 15.0), Seg::L(18.0, 9.0)]
    );
    assert_eq!(
        segs(&icons::lucide::X),
        vec![
            Seg::M(18.0, 6.0),
            Seg::L(6.0, 18.0),
            Seg::M(6.0, 6.0),
            Seg::L(18.0, 18.0)
        ]
    );
}

#[test]
fn every_lucide_icon_decodes() {
    every_icon_decodes(icons::lucide::ALL);
    assert!(
        icons::lucide::ALL.len() > 1000,
        "the full set, not a sample"
    );
}

/// The slot icons of `lib` as the components draw them, vega light.
fn slot_snapshot(lib: IconLibrary, name: &str) {
    let theme = Preset::default()
        .style(shadcn_egui::StyleName::Vega)
        .icons(lib)
        .theme(Mode::Light);
    let slots = theme.icons;
    common::snapshot_with_themes(
        name,
        vec2(360.0, 120.0),
        vec![("vega_light", theme)],
        move |ui| {
            layout::vstack(ui, 12.0, |ui| {
                layout::row(ui, 16.0, |ui| {
                    for icon in [&slots.check, &slots.chevron_down, &slots.close] {
                        ui.add(IconView::new(icon));
                        ui.add(IconView::new(icon).size(24.0));
                    }
                });
                layout::row(ui, 12.0, |ui| {
                    ui.add(Checkbox::new(&mut true));
                    ui.add(Select::new("slots", &mut 0, &["Option"]));
                    ui.add(
                        Button::new("Close")
                            .icon(&slots.close)
                            .size(ButtonSize::Icon)
                            .variant(ButtonVariant::Ghost),
                    );
                });
            });
        },
        |_| {},
    );
}

#[test]
fn lucide_slot_snapshot() {
    slot_snapshot(IconLibrary::Lucide, "icon_slots_lucide");
}

#[cfg(feature = "icons-tabler")]
#[test]
fn tabler_set() {
    every_icon_decodes(icons::tabler::ALL);
    assert_eq!(
        icons::slots(IconLibrary::Tabler),
        Some(icons::tabler::SLOTS)
    );
    let s = icons::tabler::SLOTS;
    assert_eq!(
        (s.check.name, s.chevron_down.name, s.close.name),
        ("check", "selector", "x")
    );
    slot_snapshot(IconLibrary::Tabler, "icon_slots_tabler");
}

#[cfg(feature = "icons-phosphor")]
#[test]
fn phosphor_set() {
    every_icon_decodes(icons::phosphor::ALL);
    assert_eq!(
        icons::slots(IconLibrary::Phosphor),
        Some(icons::phosphor::SLOTS)
    );
    let s = icons::phosphor::SLOTS;
    assert_eq!(
        (s.check.name, s.chevron_down.name, s.close.name),
        ("check", "caret-down", "x")
    );
    assert!(
        fill(&s.check).is_some(),
        "Phosphor regular is a filled outline"
    );
    slot_snapshot(IconLibrary::Phosphor, "icon_slots_phosphor");
}

#[cfg(feature = "icons-remixicon")]
#[test]
fn remixicon_set() {
    every_icon_decodes(icons::remixicon::ALL);
    assert_eq!(
        icons::slots(IconLibrary::Remixicon),
        Some(icons::remixicon::SLOTS)
    );
    let s = icons::remixicon::SLOTS;
    assert_eq!(
        (s.check.name, s.chevron_down.name, s.close.name),
        ("check-line", "arrow-down-s-line", "close-line")
    );
    assert!(fill(&s.close).is_some(), "Remix Icon is a filled outline");
    slot_snapshot(IconLibrary::Remixicon, "icon_slots_remixicon");
}

#[cfg(feature = "icons-hugeicons")]
#[test]
fn hugeicons_set() {
    every_icon_decodes(icons::hugeicons::ALL);
    assert_eq!(
        icons::slots(IconLibrary::Hugeicons),
        Some(icons::hugeicons::SLOTS)
    );
    let s = icons::hugeicons::SLOTS;
    assert_eq!(
        (s.check.name, s.chevron_down.name, s.close.name),
        ("tick-02", "unfold-more", "cancel-01")
    );
    slot_snapshot(IconLibrary::Hugeicons, "icon_slots_hugeicons");
}

/// The Hugeicons icons with an `opacity="0.4"` layer draw it faded, not at
/// full strength and not left out.
#[cfg(feature = "icons-hugeicons")]
#[test]
fn hugeicons_faded_layers() {
    use icons::hugeicons::{ARROW_BIG_RIGHT_DASH, HAMBURGER_01, RIGHT_TO_LEFT_LIST_BULLET};
    let faded = [
        ARROW_BIG_RIGHT_DASH,
        HAMBURGER_01,
        RIGHT_TO_LEFT_LIST_BULLET,
    ];
    for icon in &faded {
        let alphas: Vec<f32> = icon.layers().map(|l| l.alpha).collect();
        assert!(alphas.len() > 1, "{}: {alphas:?}", icon.name);
        assert!(
            alphas[1..].iter().all(|a| (a - 0.4).abs() < 1e-4),
            "{}: {alphas:?}",
            icon.name
        );
    }
    // hamburger-01: the bun top and two seeds at 0.4.
    assert_eq!(HAMBURGER_01.layers().count(), 4);
    let theme = Preset::default()
        .style(shadcn_egui::StyleName::Vega)
        .icons(IconLibrary::Hugeicons)
        .theme(Mode::Light);
    common::snapshot_with_themes(
        "icon_hugeicons_faded",
        vec2(240.0, 64.0),
        vec![("vega_light", theme)],
        move |ui| {
            layout::row(ui, 16.0, |ui| {
                ui.add(IconView::new(&HAMBURGER_01).size(48.0));
                for icon in &faded {
                    ui.add(IconView::new(icon).size(24.0));
                }
            });
        },
        |_| {},
    );
}
