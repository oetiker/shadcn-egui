mod common;

use egui::vec2;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::components::icon_atom::IconAtom;
use shadcn_egui::components::kbd::{kbd, Kbd, KbdGroup, KbdRecipes, KbdVariant};
use shadcn_egui::components::label::description;
use shadcn_egui::components::text::TextLook;
use shadcn_egui::style::Style;
use shadcn_egui::style::{AppVariant, State};
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

/// `[&_svg:not([class*='size-'])]:size-3`: an icon atom without a side
/// is 12 px in a chip, `px-1` from each edge: a 20 px square. One with a
/// side keeps it: 16 px make a 24 px wide chip.
#[test]
fn an_icon_atom_takes_the_chips_icon_size_unless_sized() {
    let theme = Theme::vega();
    let check = theme.icons.check;
    let measure = |atom: IconAtom| {
        common::measure(&theme, egui::Theme::Light, move |ui| ui.add(Kbd::new(atom))).size()
    };
    assert_eq!(measure(IconAtom::new(check)), vec2(20.0, 20.0));
    assert_eq!(measure(IconAtom::new(check).size(16.0)), vec2(24.0, 20.0));
}

/// In a plain egui widget an icon atom without a side is `Metrics::icon`
/// (16 px), as one sized 16 px.
#[test]
fn an_icon_atom_in_an_egui_button_keeps_the_default_size() {
    let theme = Theme::vega();
    let check = theme.icons.check;
    let measure = |atom: IconAtom| {
        common::measure(&theme, egui::Theme::Light, move |ui| {
            ui.add(egui::Button::new(atom))
        })
        .size()
    };
    let plain = measure(IconAtom::new(check));
    assert_eq!(plain, measure(IconAtom::new(check).size(16.0)));
    // The button keeps egui's `interact_size` height: only the width shows it.
    assert_eq!(plain.x - measure(IconAtom::new(check).size(12.0)).x, 4.0);
}

/// `pointer-events-none`: the pointer over a chip does not hover the chip
/// but still hovers what lies under it, a clickable rect and a hover-only
/// one; the chip keeps its accessible label node.
#[test]
fn a_chip_lets_hover_through_to_what_is_under_it() {
    #[derive(Default)]
    struct Hover {
        chip: bool,
        click: bool,
        plain: bool,
    }
    let theme = Theme::vega();
    let mut h = Harness::builder()
        .with_size(vec2(200.0, 80.0))
        .build_ui_state(
            move |ui, s: &mut Hover| {
                theme.apply(ui.ctx());
                let under = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), vec2(100.0, 60.0));
                let click = ui.interact(under, egui::Id::new("click"), egui::Sense::click());
                let plain = ui.interact(under, egui::Id::new("plain"), egui::Sense::hover());
                let chip = ui.put(under.shrink(20.0), Kbd::new("K"));
                *s = Hover {
                    chip: chip.hovered(),
                    click: click.hovered(),
                    plain: plain.hovered(),
                };
            },
            Hover::default(),
        );
    h.run();
    let centre = {
        let node = h.get_by_label("K");
        assert_eq!(node.accesskit_node().role(), egui::accesskit::Role::Label);
        node.rect().center()
    };
    h.hover_at(centre);
    h.run();
    let s = h.state();
    assert!(!s.chip, "chip hovered");
    assert!(s.click, "clickable rect under the chip not hovered");
    assert!(s.plain, "hover-only rect under the chip not hovered");
}

/// `inline-flex`: a group is as high as its tallest chip, also in a `Ui`
/// whose `interact_size.y` is larger.
#[test]
fn a_group_is_as_high_as_its_tallest_chip() {
    let r = common::measure(&Theme::vega(), egui::Theme::Light, |ui| {
        ui.spacing_mut().interact_size.y = 60.0;
        ui.scope(|ui| {
            KbdGroup::new().show(ui, |ui| {
                kbd(ui, "Ctrl");
                kbd(ui, "B");
            })
        })
        .response
    });
    assert_eq!(r.height(), 20.0);
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

/// `kbd-demo`'s `Ctrl + B` group: the `+` is a plain span between the
/// chips.
fn ctrl_plus_b(ui: &mut egui::Ui) {
    KbdGroup::new().show(ui, |ui| {
        kbd(ui, "Ctrl");
        TextLook::resolve(ui, &[], State::NONE).label(ui, "+", egui::TextWrapMode::Extend);
        kbd(ui, "B");
    });
}

/// The `+` between two chips is body text in the inherited look: one
/// line box of the body size, centred on the chips' centre line
/// (`items-center`), as in the browser.
#[test]
fn the_plus_between_chips_is_body_text_centred_on_the_chips() {
    for (name, theme, mode) in common::snapshot_themes() {
        let mut h = Harness::builder()
            .with_size(vec2(320.0, 120.0))
            .build_ui_state(
                move |ui, line: &mut f32| {
                    theme.apply(ui.ctx());
                    ui.ctx().set_theme(mode);
                    *line = TextLook::resolve(ui, &[], State::NONE).line_height();
                    ctrl_plus_b(ui);
                },
                0.0,
            );
        h.run();
        let line = *h.state();
        let [ctrl, plus, b] = ["Ctrl", "+", "B"].map(|n| h.get_by_label(n).rect());
        let case = format!("{name}: {ctrl:?} {plus:?} {b:?}");
        assert_eq!(plus.height(), line, "{case}");
        assert!((plus.center().y - ctrl.center().y).abs() < 0.01, "{case}");
        assert!(
            plus.left() > ctrl.right() && b.left() > plus.right(),
            "{case}"
        );
    }
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
        ctrl_plus_b(ui);
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
