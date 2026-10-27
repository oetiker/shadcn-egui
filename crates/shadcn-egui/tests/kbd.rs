mod common;

use egui::vec2;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::components::kbd::{kbd, Kbd, KbdGroup, KbdRecipes, KbdVariant};
use shadcn_egui::components::label::description;
use shadcn_egui::components::tooltip::Tooltip;
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

/// In a tooltip the chip takes the tooltip's look
/// (`in-data-[slot=tooltip-content]:bg-background/20 text-background`).
#[test]
fn a_chip_in_a_tooltip_takes_the_tooltips_look() {
    let mode = egui::Theme::Light;
    let theme = Theme::vega();
    let pal = theme.palette_of(mode);
    let mut h = Harness::builder()
        .with_size(vec2(240.0, 120.0))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            ui.ctx().set_theme(mode);
            tooltip_with_kbd(ui);
        });
    h.run();
    h.get_by_label("Save").hover();
    h.run();
    h.run();
    let chip = h.get_by_label("S").rect();
    let img = h.render().expect("render");
    let c = img.get_pixel(chip.left() as u32 + 2, chip.center().y as u32);
    // background at 20% over the foreground fill.
    let want = pal.foreground.lerp_to_gamma(pal.background, 0.2);
    for (got, want) in [c[0], c[1], c[2]]
        .into_iter()
        .zip([want.r(), want.g(), want.b()])
    {
        assert!(got.abs_diff(want) <= 3, "chip fill {c:?}, want {want:?}");
    }
}

/// `has-data-[slot=kbd]:pr-1.5`: a tooltip that holds a chip has the
/// smaller right padding.
#[test]
fn a_tooltip_with_a_chip_has_the_smaller_right_padding() {
    let mut h = Harness::builder()
        .with_size(vec2(240.0, 120.0))
        .build_ui(|ui| {
            Theme::vega().apply(ui.ctx());
            tooltip_with_kbd(ui);
        });
    h.run();
    h.get_by_label("Save").hover();
    h.run();
    h.run();
    let chip = h.get_by_label("S").rect();
    let img = h.render().expect("render");
    // The fill ends `pr-1.5` (6 px) right of the chip.
    let y = chip.center().y as u32;
    let fill = img.get_pixel(chip.right() as u32 + 3, y);
    let page = img.get_pixel(chip.right() as u32 + 9, y);
    assert_ne!(fill, page, "tooltip fill right of the chip");
    let past = img.get_pixel(chip.right() as u32 + 12, y);
    assert_eq!(page, past, "no tooltip fill 9 px right of the chip");
}

fn tooltip_with_kbd(ui: &mut egui::Ui) {
    ui.style_mut().interaction.tooltip_delay = 0.0;
    ui.add_space(56.0);
    ui.horizontal(|ui| {
        ui.add_space(56.0);
        let save = ui.add(Button::new("Save").variant(ButtonVariant::Outline));
        Tooltip::new(&save).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Save Changes");
                kbd(ui, "S");
            });
        });
    });
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

/// shadcn-svelte `kbd-tooltip-demo`'s first tooltip, open.
#[test]
fn kbd_tooltip_snapshot() {
    common::snapshot_all_with("kbd_tooltip", vec2(240.0, 120.0), tooltip_with_kbd, |h| {
        h.get_by_label("Save").hover();
        h.run();
        h.run();
    });
}
