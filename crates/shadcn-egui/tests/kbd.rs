mod common;

use egui::vec2;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use shadcn_egui::components::kbd::{kbd, Kbd, KbdGroup, KbdRecipes, KbdVariant};
use shadcn_egui::components::label::description;
use shadcn_egui::style::AppVariant;
use shadcn_egui::style::Style;
use shadcn_egui::{layout, Palette, Theme};

/// The rects of the chips `add` places, by their text.
fn rects(
    theme: &Theme,
    add: impl FnMut(&mut egui::Ui) + 'static,
    names: &[&str],
) -> Vec<egui::Rect> {
    let theme = theme.clone();
    let mut add = add;
    let mut h = Harness::builder()
        .with_size(vec2(320.0, 120.0))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            add(ui);
        });
    h.run();
    names.iter().map(|n| h.get_by_label(n).rect()).collect()
}

/// `h-5 min-w-5`: a one-letter chip is a square of the style's height;
/// luma's `h-5.5 min-w-5.5` is larger.
#[test]
fn a_one_letter_chip_is_square_at_the_styles_height() {
    let theme = |s: fn() -> Style| Theme::new(s(), Palette::light(), Palette::dark(), 10.0);
    for (theme, side) in [
        (theme(Style::vega), 20.0),
        (theme(Style::lyra), 20.0),
        (theme(Style::luma), 22.0),
    ] {
        let r = rects(&theme, |ui| _ = kbd(ui, "K"), &["K"])[0];
        assert_eq!(r.size(), vec2(side, side), "{side}");
    }
}

/// A longer text widens the chip by its text plus `px-1` on each side.
#[test]
fn a_wide_chip_grows_with_its_text() {
    let theme = Theme::vega();
    let [short, long] = rects(
        &theme,
        |ui| {
            kbd(ui, "K");
            kbd(ui, "Ctrl + Shift");
        },
        &["K", "Ctrl + Shift"],
    )[..] else {
        panic!("two chips")
    };
    assert_eq!(long.height(), short.height());
    assert!(long.width() > 60.0, "{long:?}");
}

/// An empty chip keeps its minimum size and does not panic.
#[test]
fn an_empty_chip_keeps_its_minimum_size() {
    let r = common::measure(&Theme::vega(), egui::Theme::Light, |ui| kbd(ui, ""));
    assert_eq!(r.size(), vec2(20.0, 20.0));
}

/// An icon-only chip shows its icon and reports its name.
#[test]
fn an_icon_only_chip_reports_its_name() {
    let theme = Theme::vega();
    let icon = theme.icons.check;
    let r = rects(
        &theme,
        move |ui| _ = ui.add(Kbd::icon_only(icon, "Enter")),
        &["Enter"],
    )[0];
    assert_eq!(r.size(), vec2(20.0, 20.0));
}

/// `cn-kbd-group`'s `gap-1` between chips, which share one centre line.
#[test]
fn a_group_lays_its_chips_out_in_one_row() {
    let theme = Theme::vega();
    let [ctrl, b] = rects(
        &theme,
        |ui| {
            KbdGroup::new().show(ui, |ui| {
                ui.add(Kbd::new("Ctrl"));
                ui.add(Kbd::new("B"));
            });
        },
        &["Ctrl", "B"],
    )[..] else {
        panic!("two chips")
    };
    assert_eq!(b.left() - ctrl.right(), 4.0);
    assert_eq!(b.center().y, ctrl.center().y);
}

/// A group's app variant reaches its chips; a chip's own variant wins.
#[test]
fn a_groups_variant_reaches_its_chips() {
    const WIDE: AppVariant<KbdVariant> = KbdVariant::app("wide");
    let theme = Theme::vega().variant(WIDE, KbdVariant::Default, |r: &mut KbdRecipes, _| {
        r.kbd.min_width.set(40.0);
    });
    let [inner, own] = rects(
        &theme,
        |ui| {
            KbdGroup::new().variant(WIDE).show(ui, |ui| {
                ui.add(Kbd::new("A"));
                ui.add(Kbd::new("B").variant(KbdVariant::Default));
            });
        },
        &["A", "B"],
    )[..] else {
        panic!("two chips")
    };
    assert_eq!(inner.width(), 40.0);
    assert_eq!(own.width(), 20.0);
}

/// shadcn-svelte `kbd-demo` and `kbd-group-demo`, and an icon-only chip.
fn kbd_demo(ui: &mut egui::Ui) {
    let icons = Theme::current(ui.ctx()).icons;
    layout::vstack(ui, 16.0, |ui| {
        KbdGroup::new().show(ui, |ui| {
            for k in ["⌘", "⇧", "⌥", "⌃"] {
                kbd(ui, k);
            }
        });
        KbdGroup::new().show(ui, |ui| {
            kbd(ui, "Ctrl");
            ui.label("+");
            kbd(ui, "B");
        });
        ui.horizontal_wrapped(|ui| {
            description(ui, "Use");
            KbdGroup::new().show(ui, |ui| {
                kbd(ui, "Ctrl + B");
                kbd(ui, "Ctrl + K");
            });
            description(ui, "to open the command palette");
        });
        ui.add(Kbd::icon_only(icons.check, "Enter"));
    });
}

#[test]
fn kbd_snapshot() {
    common::snapshot_all("kbd", vec2(420.0, 180.0), kbd_demo);
}
