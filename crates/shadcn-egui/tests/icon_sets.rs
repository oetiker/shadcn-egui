mod common;

use shadcn_egui::style::Style;
use shadcn_egui::{icons, Icon, IconLibrary, Palette, Seg, Theme};

/// Every icon of a set decodes to something drawable with in-range indices.
fn every_icon_decodes(all: &[&Icon]) {
    assert!(!all.is_empty());
    for icon in all {
        assert_eq!(icon.size, 24.0, "{}", icon.name);
        let fill = icon.fill_mesh();
        assert!(
            icon.segs().count() > 0 || fill.is_some(),
            "{} draws nothing",
            icon.name
        );
        if let Some(f) = fill {
            assert_eq!(f.indices.len() % 3, 0, "{}", icon.name);
            assert!(
                f.indices.iter().all(|&i| usize::from(i) < f.vertices.len()),
                "{}: index out of range",
                icon.name
            );
        }
    }
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
    let segs = |i: &Icon| i.segs().collect::<Vec<_>>();
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
