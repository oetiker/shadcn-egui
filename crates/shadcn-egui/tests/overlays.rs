mod common;

use egui::vec2;
use egui::Key;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::components::dialog::{self, Dialog};
use shadcn_egui::components::popover::{popover_header, Popover};
use shadcn_egui::components::select::{Select, SelectSize};
use shadcn_egui::components::tooltip::tooltip;
use shadcn_egui::Theme;
use shadcn_egui::{MenuColor, Palette, Preset, StyleName};

/// A dialog harness: `open` starts true; the body has a button that counts clicks.
fn dialog_harness(open: bool) -> Harness<'static, (bool, u32)> {
    Harness::builder()
        .with_size(vec2(640.0, 360.0))
        .build_ui_state(
            |ui, (open, clicks): &mut (bool, u32)| {
                Theme::vega().apply(ui.ctx());
                ui.label("page beneath");
                Dialog::new("confirm", "Delete session?")
                    .description("The session and its files are removed.")
                    .show(ui.ctx(), open, |ui| {
                        dialog::footer(ui, |ui| {
                            ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                            if ui
                                .add(Button::new("Delete").variant(ButtonVariant::Destructive))
                                .clicked()
                            {
                                *clicks += 1;
                            }
                        });
                    });
            },
            (open, 0),
        )
}

#[test]
fn dialog_body_is_interactive_and_stays_open() {
    let mut h = dialog_harness(true);
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    assert_eq!(h.state().1, 1);
    assert!(h.state().0, "a click inside the card must not close it");
}

#[test]
fn dialog_closes_on_cross() {
    let mut h = dialog_harness(true);
    h.run();
    h.get_by_label("Close").click();
    h.run();
    assert!(!h.state().0);
}

#[test]
fn dialog_closes_on_escape() {
    let mut h = dialog_harness(true);
    h.run();
    h.key_press(Key::Escape);
    h.run();
    assert!(!h.state().0);
}

#[test]
fn dialog_closes_on_backdrop_click() {
    let mut h = dialog_harness(true);
    h.run();
    h.get_by_label("page beneath").click();
    h.run();
    assert!(!h.state().0, "a backdrop click closes the dialog");
}

#[test]
fn closed_dialog_draws_nothing() {
    let mut h = dialog_harness(false);
    h.run();
    assert!(h.query_by_label("Delete session?").is_none());
}

/// Inner width of a dialog under `theme` in a window of `screen` width.
fn dialog_inner_width(theme: &Theme, screen: f32) -> f32 {
    let inner = std::cell::Cell::new(0.0_f32);
    let mut h = Harness::builder()
        .with_size(vec2(screen, 400.0))
        .build_ui(|ui| {
            theme.apply(ui.ctx());
            Dialog::new("w", "Title")
                .show(ui.ctx(), &mut true, |ui| inner.set(ui.available_width()));
        });
    h.run();
    h.run();
    inner.get()
}

#[test]
fn dialog_box_is_max_w_sm_or_the_window_minus_32() {
    for (name, theme, _) in common::themes() {
        let pad = theme
            .style
            .dialog_content
            .pad_left
            .base
            .expect("fixed padding");
        let max = theme.style.dialog_content.max_width.rules[0].value;
        assert_eq!(
            dialog_inner_width(&theme, 1024.0) + 2.0 * pad,
            max,
            "{name}: sm:max-w-*"
        );
        assert_eq!(
            dialog_inner_width(&theme, 300.0) + 2.0 * pad,
            300.0 - 32.0,
            "{name}: window minus 32"
        );
    }
    let nova = common::themes()
        .into_iter()
        .find(|(n, _, _)| *n == "nova_dark")
        .expect("nova")
        .1;
    assert_eq!(
        dialog_inner_width(&nova, 1024.0) + 32.0,
        384.0,
        "nova is max-w-sm"
    );
}

#[test]
fn dialog_close_button_can_be_hidden() {
    let mut h = Harness::builder()
        .with_size(vec2(640.0, 360.0))
        .build_ui(|ui| {
            Theme::vega().apply(ui.ctx());
            Dialog::new("nc", "Title")
                .show_close_button(false)
                .show(ui.ctx(), &mut true, |_| {});
        });
    h.run();
    assert!(h.query_by_label("Close").is_none());
}

/// A part paints at its own `disabled:` opacity, not egui's fade
/// (conformance 2.2): nova's footer strip has no `disabled:` rule, so in a
/// disabled area of the dialog it keeps its full colour, as in a disabled
/// `<fieldset>`.
#[test]
fn a_disabled_dialog_footer_keeps_its_strip_colour() {
    use shadcn_egui::style::{pick, State};
    let mode = egui::Theme::Light;
    let theme = Theme::new(
        shadcn_egui::Style::nova(),
        Palette::light(),
        Palette::dark(),
        10.0,
    );
    let strip = pick(&[&*theme.style.dialog_footer], |r| &r.bg, State::NONE)
        .map(|p| common::resolved(&theme, mode).paint(p))
        .expect("nova's footer has a strip");
    let fills = |disabled: bool| {
        let ctx = egui::Context::default();
        theme.apply(&ctx);
        ctx.set_theme(mode);
        // No fade-in of the modal's area.
        ctx.global_style_mut(|s| s.animation_time = 0.0);
        let mut out = None;
        for _ in 0..2 {
            out = Some(common::no_textures(ctx.run_ui(
                egui::RawInput::default(),
                |ui| {
                    Dialog::new("d", "Title").show(ui.ctx(), &mut true, |ui| {
                        ui.add_enabled_ui(!disabled, |ui| dialog::footer(ui, |_| {}));
                    });
                },
            )));
        }
        common::rect_shapes(&out.expect("ran"))
            .into_iter()
            .map(|r| r.fill)
            .collect::<Vec<_>>()
    };
    assert!(fills(false).contains(&strip), "enabled strip");
    assert!(
        fills(true).contains(&strip),
        "disabled strip keeps {strip:?}"
    );
}

/// An open dialog over a page, with a two-action footer.
fn dialog_page(ui: &mut egui::Ui) {
    ui.label("page beneath");
    Dialog::new("confirm", "Delete session?")
        .description("The session and its files are removed.")
        .show(ui.ctx(), &mut true, |ui| {
            ui.label("This cannot be undone.");
            dialog::footer(ui, |ui| {
                ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                ui.add(Button::new("Delete").variant(ButtonVariant::Destructive));
            });
        });
}

#[test]
fn dialog_snapshot() {
    common::snapshot_all("dialog", vec2(800.0, 500.0), dialog_page);
}

/// In a phone-wide window the footer stacks its actions, the primary on
/// top, each as wide as the footer (`flex-col-reverse` below `sm`).
#[test]
fn dialog_narrow_snapshot() {
    common::snapshot_with_themes(
        "dialog_narrow",
        vec2(common::PHONE, 500.0),
        common::themes(),
        dialog_page,
        |_| {},
    );
}

fn popover_trigger(ui: &mut egui::Ui) {
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        let trigger = ui.add(Button::new("Dimensions").variant(ButtonVariant::Outline));
        Popover::new(&trigger).show(|ui| {
            popover_header(ui, "Layer size", "Set the dimensions for the layer.");
            ui.label("Width 100%");
        });
        ui.add_space(120.0);
        ui.label("elsewhere");
    });
}

fn popover_harness() -> Harness<'static> {
    Harness::builder()
        .with_size(vec2(420.0, 240.0))
        .build_ui(|ui| {
            Theme::vega().apply(ui.ctx());
            popover_trigger(ui);
        })
}

#[test]
fn popover_toggles_and_closes_outside() {
    let mut h = popover_harness();
    h.run();
    assert!(h
        .query_by_label("Set the dimensions for the layer.")
        .is_none());
    h.get_by_label("Dimensions").click();
    h.run();
    h.get_by_label("Set the dimensions for the layer.");
    h.get_by_label("elsewhere").click();
    h.run();
    assert!(
        h.query_by_label("Set the dimensions for the layer.")
            .is_none(),
        "click outside closes"
    );
}

#[test]
fn popover_closes_on_escape_and_toggles_on_trigger() {
    let mut h = popover_harness();
    h.run();
    h.get_by_label("Dimensions").click();
    h.run();
    h.get_by_label("Set the dimensions for the layer.");
    h.key_press(Key::Escape);
    h.run();
    assert!(
        h.query_by_label("Set the dimensions for the layer.")
            .is_none(),
        "Esc closes"
    );
    h.get_by_label("Dimensions").click();
    h.run();
    h.get_by_label("Set the dimensions for the layer.");
    h.get_by_label("Dimensions").click();
    h.run();
    assert!(
        h.query_by_label("Set the dimensions for the layer.")
            .is_none(),
        "the trigger toggles"
    );
}

#[test]
fn popover_box_is_w_72_and_content_gets_the_padding() {
    for (name, theme, mode) in common::themes() {
        let inner = std::cell::Cell::new(0.0_f32);
        let mut h = Harness::builder()
            .with_size(vec2(420.0, 240.0))
            .build_ui(|ui| {
                theme.apply(ui.ctx());
                ui.ctx().set_theme(mode);
                let trigger = ui.add(Button::new("Dimensions"));
                Popover::new(&trigger).show(|ui| inner.set(ui.available_width()));
            });
        h.run();
        h.get_by_label("Dimensions").click();
        h.run();
        let pad = theme
            .style
            .popover_content
            .pad_left
            .base
            .expect("fixed padding");
        assert_eq!(
            inner.get() + 2.0 * pad,
            288.0,
            "{name}: w-72 includes the padding"
        );
    }
}

#[test]
fn popover_snapshot() {
    common::snapshot_all_with("popover", vec2(420.0, 240.0), popover_trigger, |h| {
        h.get_by_label("Dimensions").click();
        h.run();
        h.get_by_label("elsewhere").hover();
        h.run();
    });
}

fn tooltip_trigger(ui: &mut egui::Ui) {
    ui.style_mut().interaction.tooltip_delay = 0.0;
    ui.add_space(32.0);
    ui.horizontal(|ui| {
        ui.add_space(32.0);
        tooltip(
            ui.add(Button::new("Hover me").variant(ButtonVariant::Outline)),
            "Add to library",
        );
    });
}

#[test]
fn tooltip_shows_on_hover() {
    let mut h = Harness::builder()
        .with_size(vec2(240.0, 120.0))
        .build_ui(|ui| {
            Theme::vega().apply(ui.ctx());
            tooltip_trigger(ui);
        });
    h.run();
    assert!(h.query_by_label("Add to library").is_none());
    h.get_by_label("Hover me").hover();
    h.run();
    h.get_by_label("Add to library");
}

/// shadcn's `disabled:pointer-events-none`: the pointer on a disabled
/// trigger does not reach it, so neither the crate's tooltip nor egui's
/// `on_hover_text` opens there. egui's `on_disabled_hover_text` is the
/// hint for a disabled widget.
#[test]
fn a_disabled_trigger_shows_no_tooltip() {
    let mut h = Harness::builder()
        .with_size(vec2(360.0, 160.0))
        .build_ui(|ui| {
            Theme::vega().apply(ui.ctx());
            ui.style_mut().interaction.tooltip_delay = 0.0;
            ui.add_space(32.0);
            ui.horizontal(|ui| {
                ui.add_space(32.0);
                tooltip(ui.add_enabled(false, Button::new("Crate")), "Crate tip");
                ui.add_enabled(false, Button::new("Egui"))
                    .on_hover_text("Egui tip");
                ui.add_enabled(false, Button::new("Hint"))
                    .on_disabled_hover_text("Hint tip");
            });
        });
    h.run();
    for (trigger, tip, shows) in [
        ("Crate", "Crate tip", false),
        ("Egui", "Egui tip", false),
        ("Hint", "Hint tip", true),
    ] {
        h.get_by_label(trigger).hover();
        h.run();
        assert_eq!(h.query_by_label(tip).is_some(), shows, "{trigger}");
    }
}

#[test]
fn tooltip_is_inverted_with_an_arrow_toward_the_trigger() {
    for (name, theme, mode) in common::themes() {
        let mut h = Harness::builder()
            .with_size(vec2(240.0, 120.0))
            .build_ui(|ui| {
                theme.apply(ui.ctx());
                ui.ctx().set_theme(mode);
                tooltip_trigger(ui);
            });
        h.run();
        let trigger = h.get_by_label("Hover me").rect();
        h.get_by_label("Hover me").hover();
        h.run();
        let text = h.get_by_label("Add to library").rect();
        let img = h.render().expect("render");
        let px = |p: egui::Pos2| {
            let c = img.get_pixel(p.x as u32, p.y as u32);
            egui::Color32::from_rgb(c[0], c[1], c[2])
        };
        // bg-foreground behind the text; the arrow reaches into the trigger below.
        let fg = theme.palette_of(mode).foreground;
        let corner = egui::pos2(text.left() - 4.0, text.center().y);
        assert_eq!(px(corner), fg, "{name}: fill is foreground");
        let tip = egui::pos2(text.center().x, trigger.top() - 2.0);
        assert_eq!(px(tip), fg, "{name}: arrow reaches the trigger");
    }
}

/// The tooltip paints its box through the recipe box: a border an app
/// gives it in `theme.style` shows (review F4).
#[test]
fn tooltip_box_takes_the_styles_border() {
    use shadcn_egui::style::{Paint, Prop, Role};
    let mode = egui::Theme::Light;
    let theme = common::restyled(Theme::vega(), |s| {
        s.tooltip_content.border_width = Prop::fixed(4.0);
        s.tooltip_content.border_color = Prop::fixed(Paint::Role(Role::Destructive, 1.0));
    });
    let red = theme.palette_of(mode).destructive;
    let mut h = Harness::builder()
        .with_size(vec2(240.0, 120.0))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            ui.ctx().set_theme(mode);
            tooltip_trigger(ui);
        });
    h.run();
    h.get_by_label("Hover me").hover();
    h.run();
    let text = h.get_by_label("Add to library").rect();
    let img = h.render().expect("render");
    let y = text.center().y as u32;
    let found = (0..text.left() as u32).rev().any(|x| {
        let c = img.get_pixel(x, y);
        egui::Color32::from_rgb(c[0], c[1], c[2]) == red
    });
    assert!(found, "no destructive border left of the tooltip text");
}

#[test]
fn tooltip_snapshot() {
    common::snapshot_all_with("tooltip", vec2(240.0, 140.0), tooltip_trigger, |h| {
        h.get_by_label("Hover me").hover();
        h.run();
    });
}

const FRUITS: [&str; 4] = ["Apple", "Banana", "Blueberry", "Cherry"];

fn select_harness(start: usize) -> Harness<'static, usize> {
    // A bare control has no accessible name (as upstream's demo); names are
    // not what this test checks.
    Harness::builder()
        .with_accessibility_check(false)
        .with_size(vec2(320.0, 260.0))
        .build_ui_state(
            |ui, value: &mut usize| {
                Theme::vega().apply(ui.ctx());
                ui.add(Select::new("fruit", value, &FRUITS).placeholder("Pick a fruit"));
                ui.label("elsewhere");
            },
            start,
        )
}

fn open_select(h: &mut Harness<'_, usize>) {
    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run();
}

#[test]
fn select_opens_and_a_click_chooses() {
    let mut h = select_harness(0);
    h.run();
    assert!(
        h.query_by_label("Cherry").is_none(),
        "closed list shows no options"
    );
    open_select(&mut h);
    h.get_by_label("Cherry").click();
    h.run();
    assert_eq!(*h.state(), 3);
    assert!(
        h.query_by_label("Banana").is_none(),
        "choosing closes the list"
    );
}

/// bits' list is a `listbox` of `option`s; the chosen one is
/// `aria-selected`, none is toggled.
#[test]
fn select_options_are_listbox_options() {
    use egui::accesskit::Role;
    use egui_kittest::kittest::NodeT;
    let mut h = select_harness(2);
    h.run();
    open_select(&mut h);
    for (i, fruit) in FRUITS.iter().enumerate() {
        let option = h.get_by_role_and_label(Role::ListBoxOption, fruit);
        let node = option.accesskit_node();
        assert_eq!(node.is_selected(), Some(i == 2), "{fruit}");
        assert_eq!(node.toggled(), None, "{fruit}");
        // Up to the window: one listbox and no dialog (egui's popup role).
        let mut roles = Vec::new();
        let mut up = option.parent();
        while let Some(p) = up {
            roles.push(p.accesskit_node().role());
            up = p.parent();
        }
        let lists = roles.iter().filter(|&&r| r == Role::ListBox).count();
        assert_eq!(lists, 1, "{fruit}: {roles:?}");
        assert!(!roles.contains(&Role::Dialog), "{fruit}: {roles:?}");
    }
}

#[test]
fn select_keyboard_down_enter_chooses_the_next_option() {
    let mut h = select_harness(0);
    h.run();
    open_select(&mut h);
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(*h.state(), 0, "moving the highlight does not choose");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(*h.state(), 1);
    assert!(
        h.query_by_label("Cherry").is_none(),
        "Enter closes the list"
    );
}

#[test]
fn select_escape_and_outside_click_keep_the_value() {
    let mut h = select_harness(0);
    h.run();
    open_select(&mut h);
    h.key_press(Key::ArrowDown);
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(*h.state(), 0);
    assert!(h.query_by_label("Cherry").is_none(), "Esc closes");
    open_select(&mut h);
    h.get_by_label("elsewhere").click();
    h.run();
    assert_eq!(*h.state(), 0);
    assert!(
        h.query_by_label("Cherry").is_none(),
        "a click outside closes"
    );
}

/// An open list whose trigger lost focus to another widget leaves the keys
/// to that widget: typing and Enter do not choose an option.
#[test]
fn open_select_ignores_keys_once_focus_moves_elsewhere() {
    // A bare control has no accessible name (as upstream's demo); names are
    // not what this test checks.
    let mut h = Harness::builder()
        .with_accessibility_check(false)
        .with_size(vec2(320.0, 260.0))
        .build_ui_state(
            |ui, value: &mut usize| {
                Theme::vega().apply(ui.ctx());
                ui.add(Select::new("fruit", value, &FRUITS));
                ui.add(shadcn_egui::components::button::Button::new("Other"));
            },
            0,
        );
    h.run();
    open_select(&mut h);
    h.get_by_label("Other").focus();
    h.run();
    // Typing moves the highlight of a list that has the keys; arrows are
    // left out, as egui moves focus with them.
    h.event(egui::Event::Text("c".into()));
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(*h.state(), 0);
}

#[test]
fn select_typing_jumps_to_the_next_option_with_that_letter() {
    let mut h = select_harness(0);
    h.run();
    open_select(&mut h);
    h.get_by_role(egui::accesskit::Role::ComboBox)
        .type_text("b");
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(
        *h.state(),
        1,
        "first option after the highlight starting with b"
    );
}

/// The trigger's accessibility value is the shown text; it has no name of
/// its own (a form names it with `labelled_by`).
#[test]
fn select_trigger_reports_the_current_text() {
    let mut h = select_harness(2);
    h.run();
    let node = h
        .get_by_role(egui::accesskit::Role::ComboBox)
        .accesskit_node();
    assert_eq!(node.value().as_deref(), Some("Blueberry"));
    assert_eq!(node.label(), None);
    let mut h = select_harness(9);
    h.run();
    assert_eq!(
        h.get_by_role(egui::accesskit::Role::ComboBox)
            .accesskit_node()
            .value()
            .as_deref(),
        Some("Pick a fruit")
    );
}

#[test]
fn select_disabled_does_not_open() {
    // A bare control has no accessible name (as upstream's demo); names are
    // not what this test checks.
    let mut h = Harness::builder()
        .with_accessibility_check(false)
        .with_size(vec2(320.0, 260.0))
        .build_ui_state(
            |ui, value: &mut usize| {
                Theme::vega().apply(ui.ctx());
                ui.add_enabled(false, Select::new("fruit", value, &FRUITS));
            },
            0usize,
        );
    h.run();
    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run();
    assert!(h.query_by_label("Cherry").is_none());
}

#[test]
fn select_trigger_height_follows_style_and_size() {
    for (name, theme, mode) in common::themes() {
        let (default_h, sm_h) = if name.starts_with("nova") {
            (32.0, 28.0)
        } else {
            (36.0, 32.0)
        };
        let mut v = 0usize;
        let mut h = |size| {
            common::measure(&theme, mode, |ui| {
                ui.add(Select::new("s", &mut v, &FRUITS).size(size))
            })
            .height()
        };
        assert_eq!(h(SelectSize::Default), default_h, "{name}");
        assert_eq!(h(SelectSize::Sm), sm_h, "{name}");
    }
}

fn select_triggers(ui: &mut egui::Ui) {
    let (mut a, mut b, mut c, mut d, mut e) = (9usize, 1usize, 1usize, 0usize, 2usize);
    ui.spacing_mut().item_spacing.y = 12.0;
    ui.add(
        Select::new("a", &mut a, &FRUITS)
            .placeholder("Pick a fruit")
            .width(180.0),
    );
    ui.add(Select::new("b", &mut b, &FRUITS).width(180.0));
    ui.add(Select::new("c", &mut c, &FRUITS).invalid(true).width(180.0));
    ui.add_enabled(false, Select::new("d", &mut d, &FRUITS).width(180.0));
    ui.add(Select::new("e", &mut e, &FRUITS).size(SelectSize::Sm));
}

#[test]
fn select_trigger_snapshot() {
    common::snapshot_all("select_trigger", vec2(240.0, 260.0), select_triggers);
}

#[test]
fn select_open_snapshot() {
    common::snapshot_all_with(
        "select_open",
        vec2(260.0, 260.0),
        |ui| {
            let mut v = 1usize;
            ui.add(Select::new("fruit", &mut v, &FRUITS).width(180.0));
            ui.add_space(200.0);
            ui.label("elsewhere");
        },
        |h| {
            h.get_by_role(egui::accesskit::Role::ComboBox).click();
            h.run();
            h.get_by_label("elsewhere").hover();
            h.run();
        },
    );
}

#[test]
fn select_item_rows_have_the_text_sm_line_plus_padding() {
    for (name, theme, mode) in common::themes() {
        // A bare control has no accessible name (as upstream's demo); names are
        // not what this test checks.
        let mut h = Harness::builder()
            .with_accessibility_check(false)
            .with_size(vec2(320.0, 260.0))
            .build_ui_state(
                |ui, value: &mut usize| {
                    theme.apply(ui.ctx());
                    ui.ctx().set_theme(mode);
                    ui.add(Select::new("fruit", value, &FRUITS));
                },
                0usize,
            );
        h.run();
        open_select(&mut h);
        // 20 px line plus py-1.5 (vega) or py-1 (nova).
        let want = if name.starts_with("nova") { 28.0 } else { 32.0 };
        assert_eq!(h.get_by_label("Cherry").rect().height(), want, "{name}");
    }
}

/// An option wider than the window caps the list to the window (conformance
/// 3.1); its text wraps in its row, as the browser wraps it.
#[test]
fn select_list_is_capped_to_the_window() {
    const LONG: [&str; 2] = [
        "Short",
        "An option whose text is far wider than this narrow window",
    ];
    // A bare control has no accessible name (as upstream's demo); names are
    // not what this test checks.
    let mut h = Harness::builder()
        .with_accessibility_check(false)
        .with_size(vec2(200.0, 300.0))
        .build_ui_state(
            |ui, value: &mut usize| {
                Theme::vega().apply(ui.ctx());
                ui.ctx().set_theme(egui::Theme::Light);
                ui.add(Select::new("long", value, &LONG).width(150.0));
            },
            0usize,
        );
    h.run();
    open_select(&mut h);
    let short = h.get_by_label(LONG[0]).rect();
    let long = h.get_by_label(LONG[1]).rect();
    assert!(long.right() <= 200.0, "row ends at {}", long.right());
    assert_eq!(long.width(), short.width(), "rows share the list width");
    assert!(
        long.height() > short.height(),
        "the long text wraps: {} vs {}",
        long.height(),
        short.height()
    );
}

/// A select of thirty options: more than its list shows in a 260 px
/// window.
fn long_select_harness() -> Harness<'static, usize> {
    let options: Vec<String> = (0..30).map(|i| format!("Option {i}")).collect();
    // A bare control has no accessible name (as upstream's demo); names are
    // not what this test checks.
    Harness::builder()
        .with_accessibility_check(false)
        .with_size(vec2(320.0, 260.0))
        .build_ui_state(
            move |ui, value: &mut usize| {
                Theme::vega().apply(ui.ctx());
                ui.ctx().set_theme(egui::Theme::Light);
                let options: Vec<&str> = options.iter().map(String::as_str).collect();
                ui.add(Select::new("many", value, &options).width(180.0));
            },
            0usize,
        )
}

/// bits keeps the highlighted option in view: arrows that move the
/// highlight past the visible rows scroll the list after it.
#[test]
fn select_list_follows_the_highlight() {
    let mut h = long_select_harness();
    h.run();
    open_select(&mut h);
    assert!(
        h.get_by_label("Option 15").rect().top() > 260.0,
        "starts below the window"
    );
    for _ in 0..15 {
        h.key_press(Key::ArrowDown);
        h.run();
    }
    let row = h.get_by_label("Option 15").rect();
    assert!(
        row.top() >= 0.0 && row.bottom() <= 260.0,
        "the highlighted row is in view: {row:?}"
    );
}

/// The list scrolls in the crate's `ScrollArea`: hovered, an overflowing
/// list shows the 7 px `border` thumb of the scroll-area recipes.
#[test]
fn select_list_uses_the_crate_scroll_bar() {
    use shadcn_egui::style::{Paint, Role};
    let mut h = long_select_harness();
    h.run();
    open_select(&mut h);
    h.get_by_label("Option 2").hover();
    h.run();
    let border = Theme::current(&h.ctx).paint(Paint::Role(Role::Border, 1.0));
    let thumbs: Vec<_> = common::rect_shapes(h.output())
        .into_iter()
        .filter(|r| r.fill == border && r.rect.width() == 7.0)
        .collect();
    assert_eq!(thumbs.len(), 1, "{thumbs:?}");
}

/// The trigger is `w-fit`, capped to the room (conformance 3.1): an option
/// wider than the window, or a wider `width`, keeps it inside the window.
#[test]
fn select_trigger_is_capped_to_the_room() {
    const LONG: [&str; 2] = [
        "Short",
        "An option whose text is far wider than this narrow window",
    ];
    for width in [None, Some(400.0)] {
        // A bare control has no accessible name (as upstream's demo); names are
        // not what this test checks.
        let mut h = Harness::builder()
            .with_accessibility_check(false)
            .with_size(vec2(200.0, 300.0))
            .build_ui_state(
                move |ui, value: &mut usize| {
                    Theme::vega().apply(ui.ctx());
                    ui.ctx().set_theme(egui::Theme::Light);
                    let select = Select::new("long", value, &LONG);
                    match width {
                        Some(w) => ui.add(select.width(w)),
                        None => ui.add(select),
                    };
                },
                0usize,
            );
        h.run();
        let trigger = h.get_by_role(egui::accesskit::Role::ComboBox).rect();
        assert!(
            trigger.right() <= 200.0,
            "{width:?}: trigger ends at {}",
            trigger.right()
        );
    }
}

/// Where the room is narrower than the trigger's padding, gap and chevron,
/// the trigger takes that content minimum (conformance 3.1) and overflows
/// without widening its parent (3.6).
#[test]
fn select_trigger_in_no_room_takes_its_content_minimum() {
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    ctx.set_theme(egui::Theme::Light);
    let mut v = 0usize;
    let mut rects = (egui::Rect::NOTHING, egui::Rect::NOTHING);
    for _ in 0..2 {
        ctx.run_ui(egui::RawInput::default(), |ui| {
            let parent = ui.allocate_ui(vec2(0.0, 100.0), |ui| {
                ui.add(Select::new("fruit", &mut v, &FRUITS)).rect
            });
            rects = (parent.inner, parent.response.rect);
        })
        .drop_without_applying_deltas();
    }
    let (trigger, parent) = rects;
    // vega: pl-2.5, pr-2, a 1 px border each side, gap-1.5, a size-4 chevron.
    assert_eq!(trigger.width(), 10.0 + 8.0 + 2.0 + 6.0 + 16.0);
    assert_eq!(parent.width(), 0.0, "the parent keeps its width");
}

/// Pixels darker than (60, 60, 60) once the list of a light-mode Select is open.
fn dark_pixels_of_open_list(theme: Theme) -> usize {
    let mode = egui::Theme::Light;
    let bg = theme.palette_of(mode).background;
    // A bare control has no accessible name (as upstream's demo); names are
    // not what this test checks.
    let mut h = Harness::builder()
        .with_accessibility_check(false)
        .with_size(vec2(260.0, 260.0))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            ui.ctx().set_theme(mode);
            ui.ctx()
                .layer_painter(egui::LayerId::background())
                .rect_filled(ui.ctx().viewport_rect(), 0.0, bg);
            let mut v = 1usize;
            ui.add(Select::new("fruit", &mut v, &FRUITS).width(180.0));
        });
    h.run();
    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run();
    let img = h.render().expect("wgpu render");
    img.pixels()
        .filter(|p| p.0[0] < 60 && p.0[1] < 60 && p.0[2] < 60)
        .count()
}

#[test]
fn an_inverted_menu_is_dark_in_light_mode() {
    let inverted = Preset::default()
        .style(StyleName::Vega)
        .menu_color(MenuColor::Inverted)
        .theme();
    let mt = inverted.menu_theme();
    assert!(
        mt.is_dark() && mt.palette() == &Palette::dark(),
        "the menu resolves as dark mode"
    );
    let plain = dark_pixels_of_open_list(Preset::default().style(StyleName::Vega).theme());
    let dark = dark_pixels_of_open_list(inverted);
    // The open list is ~180 x 140 px: filled dark it adds well over 10 000
    // dark pixels; a light list only has its text.
    assert!(
        dark > 3 * plain + 5000,
        "inverted list must be dark: {dark} dark px, default list {plain}"
    );
}

/// An open Select in a preset's menu colours, light and dark.
fn select_open_in(name: &str, color: MenuColor) {
    let themes = vec![
        // Preset::default() is Nova (ui.shadcn.com DEFAULT_CONFIG); these snapshots show Vega.
        (
            "vega_light",
            Preset::default()
                .style(StyleName::Vega)
                .menu_color(color)
                .theme(),
            egui::Theme::Light,
        ),
        (
            "vega_dark",
            Preset::default()
                .style(StyleName::Vega)
                .menu_color(color)
                .theme(),
            egui::Theme::Dark,
        ),
    ];
    common::snapshot_with_themes(
        name,
        vec2(260.0, 260.0),
        themes,
        |ui| {
            let mut v = 1usize;
            ui.add(Select::new("fruit", &mut v, &FRUITS).width(180.0));
            ui.add_space(160.0);
            ui.label("elsewhere");
        },
        |h| {
            h.get_by_role(egui::accesskit::Role::ComboBox).click();
            h.run();
            h.get_by_label("elsewhere").hover();
            h.run();
        },
    );
}

#[test]
fn select_open_inverted_snapshot() {
    select_open_in("select_open_inverted", MenuColor::Inverted);
}

/// Rects of `n` free 40×10 boxes in a dialog's body and of one box in
/// its footer, as offsets from the title's bottom (the dialog is centred,
/// so its place moves with its height).
fn dialog_free_boxes(n: usize) -> (Vec<egui::Rect>, egui::Rect) {
    let seen = std::cell::RefCell::new((Vec::new(), egui::Rect::NOTHING));
    let boxed = |ui: &mut egui::Ui| {
        ui.allocate_exact_size(vec2(40.0, 10.0), egui::Sense::hover())
            .0
    };
    let mut h = Harness::builder()
        .with_size(vec2(640.0, 400.0))
        .build_ui(|ui| {
            Theme::vega().apply(ui.ctx());
            Dialog::new("free", "Title").show(ui.ctx(), &mut true, |ui| {
                let free: Vec<_> = (0..n).map(|_| boxed(ui)).collect();
                let mut foot = egui::Rect::NOTHING;
                dialog::footer(ui, |ui| foot = boxed(ui));
                *seen.borrow_mut() = (free, foot);
            });
        });
    h.run();
    h.run();
    let up = vec2(0.0, -h.get_by_label("Title").rect().bottom());
    let (free, foot) = seen.borrow().clone();
    (
        free.iter().map(|r| r.translate(up)).collect(),
        foot.translate(up),
    )
}

/// Free content in a dialog stands for a block (rules, Container; G4):
/// its widgets stack with no gap, and the dialog's gap stays only between
/// the run and the header and footer.
#[test]
fn free_dialog_content_is_one_block() {
    let (one, foot_one) = dialog_free_boxes(1);
    let (two, foot_two) = dialog_free_boxes(2);
    assert_eq!(two[0], one[0], "the header's gap before the run");
    assert_eq!(two[1].top(), two[0].bottom(), "no gap inside the run");
    assert_eq!(
        foot_two.top() - two[1].bottom(),
        foot_one.top() - one[0].bottom(),
        "the footer keeps its distance from the run"
    );
}

/// The footer comes last (rules, Part), free content too.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "comes before its footer")]
fn free_content_after_the_dialog_footer_fails_the_order_assert() {
    let mut h = Harness::builder()
        .with_size(vec2(640.0, 400.0))
        .build_ui(|ui| {
            Theme::vega().apply(ui.ctx());
            Dialog::new("late", "Title").show(ui.ctx(), &mut true, |ui| {
                dialog::footer(ui, |ui| {
                    ui.add(Button::new("Ok"));
                });
                ui.label("Late");
            });
        });
    h.run();
}

/// Free content in a popover stands for a block (rules, Container; G4):
/// its widgets stack with no gap, the popover's gap from the header.
#[test]
fn free_popover_content_is_one_block() {
    let seen = std::cell::RefCell::new(Vec::new());
    let mut h = Harness::builder()
        .with_size(vec2(420.0, 240.0))
        .build_ui(|ui| {
            Theme::vega().apply(ui.ctx());
            let trigger = ui.add(Button::new("Dimensions"));
            Popover::new(&trigger).show(|ui| {
                popover_header(ui, "Layer size", "Set the dimensions.");
                let boxes: Vec<_> = (0..2)
                    .map(|_| {
                        ui.allocate_exact_size(vec2(40.0, 10.0), egui::Sense::hover())
                            .0
                    })
                    .collect();
                *seen.borrow_mut() = boxes;
            });
        });
    h.run();
    h.get_by_label("Dimensions").click();
    h.run();
    h.run();
    let boxes = seen.borrow().clone();
    let description = h.get_by_label("Set the dimensions.").rect();
    let gap = Theme::vega().style.popover_content.gap.base.expect("gap");
    assert_eq!(
        boxes[0].top(),
        description.bottom() + gap,
        "{description:?}"
    );
    assert_eq!(boxes[1].top(), boxes[0].bottom(), "no gap inside the run");
}

// ---------------------------------------------------------------------------
// Upstream's demos that hold a Field

const GAP_4: shadcn_egui::style::AppVariant<shadcn_egui::components::field::FieldGroupVariant> =
    shadcn_egui::components::field::FieldGroupVariant::app("gap-4");

/// The snapshot themes with the demos' classes as app variants: the
/// `gap-4` field group of popover-form.
fn field_demo_themes() -> Vec<common::Themed> {
    use shadcn_egui::components::field::FieldGroupVariant;
    common::snapshot_themes()
        .into_iter()
        .map(|(n, t, m)| {
            let t = t.variant(GAP_4, FieldGroupVariant::Default, |r, _| {
                r.field_group.gap.set(16.0)
            });
            (n, t, m)
        })
        .collect()
}

/// select-invalid: an invalid `max-w-48` field with an error; the field
/// makes its select invalid (`aria-invalid` on the trigger).
#[test]
fn select_invalid_snapshot() {
    use shadcn_egui::components::field::Field;
    common::snapshot_all("select_invalid", vec2(240.0, 130.0), |ui| {
        ui.allocate_ui(vec2(192.0, ui.available_height()), |ui| {
            ui.set_width(192.0);
            let mut i = usize::MAX;
            Field::new()
                .invalid(true)
                .label("Fruit")
                .error("Please select a fruit.")
                .show(ui, |ui| {
                    ui.add(
                        Select::new("fruit", &mut i, &["Apple", "Banana", "Blueberry"])
                            .placeholder("Select a fruit"),
                    )
                });
        });
    });
}

/// popover-form: a `w-64` popover at the trigger's start with a header
/// and a `gap-4` group of two horizontal fields. The labels' `w-1/2` is a
/// share of the row, which no recipe length holds: in the browser's flex
/// row a `w-1/2` label and a `w-full` input shrink by their bases to a
/// third and two thirds of the row less the gap, so the input takes that
/// width and the label the rest.
#[test]
fn popover_form_snapshot() {
    use shadcn_egui::components::field::{Field, FieldGroup};
    use shadcn_egui::components::input::Input;
    use shadcn_egui::style::{pick, State};
    common::snapshot_with_themes(
        "popover_form",
        vec2(320.0, 260.0),
        field_demo_themes(),
        |ui| {
            let trigger = ui.add(Button::new("Open Popover").variant(ButtonVariant::Outline));
            Popover::new(&trigger).width(256.0).show(|ui| {
                popover_header(ui, "Dimensions", "Set the dimensions for the layer.");
                let t = Theme::current(ui.ctx());
                let row = [&*t.style.field, &*t.style.field_orientation_horizontal];
                let gap = pick(&row, |r| &r.gap, State::NONE).unwrap_or(0.0);
                let input = (ui.available_width() - gap) * 2.0 / 3.0;
                FieldGroup::new().variant(GAP_4).show(ui, |ui| {
                    for (name, value) in [("Width", "100%"), ("Height", "25px")] {
                        Field::new().horizontal().label(name).show(ui, |ui| {
                            ui.add(
                                Input::new(&mut String::from(value))
                                    .max_width(input)
                                    .id_salt(name),
                            )
                        });
                    }
                });
            });
        },
        |h| {
            h.get_by_label("Open Popover").click();
            h.run();
            // Move the pointer off the trigger, as the click leaves it there.
            h.event(egui::Event::PointerGone);
            h.run();
        },
    );
}

/// dialog-demo: a `sm:max-w-sm` dialog over its trigger, a group of two
/// fields, each with a plain Label, and the footer's actions.
#[test]
fn dialog_demo_snapshot() {
    use shadcn_egui::components::field::{Field, FieldGroup};
    use shadcn_egui::components::input::Input;
    use shadcn_egui::components::label::label;
    common::snapshot_all("dialog_demo", vec2(800.0, 500.0), |ui| {
        ui.add(Button::new("Open Dialog").variant(ButtonVariant::Outline));
        Dialog::new("dialog-demo", "Edit profile")
            .description("Make changes to your profile here. Click save when you're done.")
            .width(384.0)
            .show(ui.ctx(), &mut true, |ui| {
                FieldGroup::new().show(ui, |ui| {
                    for (name, value) in [("Name", "Pedro Duarte"), ("Username", "@peduarte")] {
                        Field::new().show(ui, |ui| {
                            label(ui, name);
                            ui.add(Input::new(&mut String::from(value)).id_salt(name))
                        });
                    }
                });
                dialog::footer(ui, |ui| {
                    ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                    ui.add(Button::new("Save changes"));
                });
            });
    });
}

/// The overlays the enter animation runs on, with upstream's duration:
/// tw-animate-css `animate-in` over `duration-100`, the tooltip without a
/// duration over tw-animate's 150 ms.
#[derive(Clone, Copy, Debug)]
enum Overlay {
    Dialog,
    Popover,
    Select,
    Tooltip,
}

impl Overlay {
    const ALL: [Overlay; 4] = [
        Overlay::Dialog,
        Overlay::Popover,
        Overlay::Select,
        Overlay::Tooltip,
    ];

    fn duration(self) -> f64 {
        match self {
            Overlay::Tooltip => 0.15,
            _ => 0.1,
        }
    }

    /// The panel's fill in vega light.
    fn fill(self) -> egui::Color32 {
        let theme = Theme::vega();
        let p = theme.palette_of(egui::Theme::Light);
        match self {
            Overlay::Dialog => p.background,
            Overlay::Popover | Overlay::Select => p.popover,
            Overlay::Tooltip => p.foreground,
        }
    }
}

/// What one pass painted of an open overlay: the panel's fill rect and
/// colour (the largest box of the panel's fill, `None` while it is
/// invisible), the dialog's backdrop colour, and whether the pass asks for
/// a repaint.
#[derive(Debug)]
struct Drawn {
    panel: Option<(egui::Rect, egui::Color32)>,
    backdrop: Option<egui::Color32>,
    repaint: bool,
}

/// An overlay opened at time 0 (its first passes size and draw it), then
/// one pass at each of `times`: what the opening passes drew, and what
/// the timed ones drew. `reduced` turns motion off.
fn enter_passes(overlay: Overlay, reduced: bool, times: &[f64]) -> (Vec<Drawn>, Vec<Drawn>) {
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|s| {
        s.interaction.tooltip_delay = 0.0;
        if reduced {
            s.animation_time = 0.0;
        }
    });
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(640.0, 360.0));
    let trigger = std::cell::Cell::new(None::<egui::Rect>);
    let mut selected = 0;
    let mut page = |ui: &mut egui::Ui| {
        ui.add_space(40.0);
        ui.horizontal(|ui| {
            ui.add_space(40.0);
            match overlay {
                Overlay::Dialog => {
                    Dialog::new("enter", "Title").show(ui.ctx(), &mut true, |ui| {
                        ui.label("Body");
                    });
                }
                Overlay::Popover => {
                    let r = ui.add(Button::new("Open").variant(ButtonVariant::Outline));
                    if trigger.replace(Some(r.rect)).is_none() {
                        egui::Popup::open_id(ui.ctx(), egui::Popup::default_response_id(&r));
                    }
                    Popover::new(&r).show(|ui| {
                        ui.label("Body");
                    });
                }
                Overlay::Select => {
                    let r = ui.add(Select::new("enter", &mut selected, &["One", "Two"]));
                    if trigger.replace(Some(r.rect)).is_none() {
                        egui::Popup::open_id(ui.ctx(), egui::Popup::default_response_id(&r));
                    }
                }
                Overlay::Tooltip => {
                    let r = ui.add(Button::new("Hover").variant(ButtonVariant::Outline));
                    trigger.set(Some(r.rect));
                    tooltip(r, "Tip");
                }
            }
        });
    };
    let fill = overlay.fill();
    let same_rgb = |c: egui::Color32, want: egui::Color32| {
        let [r, g, b, a] = c.to_srgba_unmultiplied();
        let w = want.to_srgba_unmultiplied();
        a > 0 && [r, g, b].iter().zip(w).all(|(x, y)| x.abs_diff(y) <= 2)
    };
    let mut run = |time: f64, events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            time: Some(time),
            events,
            ..Default::default()
        };
        let out = common::no_textures(ctx.run_ui(input, &mut page));
        let rects = common::rect_shapes(&out);
        let panel = rects
            .iter()
            .filter(|r| r.rect != screen && same_rgb(r.fill, fill))
            .max_by(|a, b| a.rect.area().total_cmp(&b.rect.area()))
            .map(|r| (r.rect, r.fill));
        let backdrop = rects.iter().find(|r| r.rect == screen).map(|r| r.fill);
        let repaint =
            out.viewport_output[&egui::ViewportId::ROOT].repaint_delay == std::time::Duration::ZERO;
        Drawn {
            panel,
            backdrop,
            repaint,
        }
    };
    // Open it: the pointer moves onto the tooltip's trigger once the
    // first pass placed it.
    let mut opening = vec![run(0.0, Vec::new())];
    let hover = trigger
        .get()
        .map(|r| vec![egui::Event::PointerMoved(r.center())])
        .unwrap_or_default();
    opening.push(run(0.0, hover));
    for _ in 0..3 {
        opening.push(run(0.0, Vec::new()));
    }
    let timed = times.iter().map(|&t| run(t, Vec::new())).collect();
    (opening, timed)
}

/// An overlay opens as upstream's does (tw-animate-css `animate-in
/// fade-in-0 zoom-in-95`, conformance 7): from transparent at 95 % scale
/// to its place along CSS's `ease` over the style's duration, a dialog's
/// backdrop fading in with it; a popover or a select list slides down
/// from 8 px above its place (`slide-in-from-top-2`), a tooltip up from
/// below. egui's own fade of its area (0.2 s since egui 0.37) does not
/// show.
#[test]
fn an_overlay_opens_with_upstreams_enter_animation() {
    use shadcn_egui::style::Easing;
    for overlay in Overlay::ALL {
        let d = overlay.duration();
        let half = d / 2.0;
        let (_, passes) = enter_passes(overlay, false, &[half, d, 1.0, 2.0]);
        let p = Easing::Ease.at(0.5);
        let (r_half, c_half) = passes[0].panel.expect("drawn at half time");
        let (r_end, c_end) = passes[1].panel.expect("drawn at the end");
        let (r_settled, c_settled) = passes[3].panel.expect("settled");
        assert_eq!(
            (r_end, c_end),
            (r_settled, c_settled),
            "{overlay:?} ends in place"
        );
        let opacity = f32::from(c_half.a()) / f32::from(c_end.a());
        assert!(
            (opacity - p).abs() < 0.01,
            "{overlay:?}: opacity {opacity}, want {p}"
        );
        let scale = r_half.width() / r_end.width();
        let want = 0.95 + 0.05 * p;
        assert!(
            (scale - want).abs() < 0.002,
            "{overlay:?}: scale {scale}, want {want}"
        );
        assert!(
            passes[0].repaint,
            "{overlay:?}: a running animation repaints"
        );
        let slide = 8.0 * (1.0 - p);
        match overlay {
            Overlay::Dialog => {
                let moved = r_half.center() - r_end.center();
                assert!(moved.length() < 0.01, "{overlay:?}: zooms about its centre");
                let half = passes[0].backdrop.expect("backdrop at half time");
                let end = passes[1].backdrop.expect("backdrop at the end");
                let faded = f32::from(half.a()) / f32::from(end.a());
                assert!(
                    (faded - p).abs() < 0.05,
                    "backdrop opacity {faded}, want {p}"
                );
                assert!(!passes[3].repaint, "no repaint once it has run");
            }
            Overlay::Popover | Overlay::Select => {
                // The origin lies on the top edge: only the slide moves it.
                let moved = r_half.top() - r_end.top();
                assert!(
                    (moved + slide).abs() < 0.01,
                    "{overlay:?}: top moved {moved}"
                );
            }
            Overlay::Tooltip => {
                assert!(
                    r_half.center().y > r_end.center().y,
                    "starts below its place"
                );
            }
        }
    }
}

/// Under reduced motion (`animation_time == 0`) an overlay shows at its
/// place and full opacity in the first pass that draws it.
#[test]
fn reduced_motion_opens_an_overlay_at_once() {
    for overlay in Overlay::ALL {
        let (_, passes) = enter_passes(overlay, true, &[0.0, 1.0]);
        let first = passes[0]
            .panel
            .unwrap_or_else(|| panic!("{overlay:?} drawn {:?}", overlay.fill()));
        let settled = passes[1].panel.expect("settled");
        assert_eq!(first, settled, "{overlay:?}");
        if let Overlay::Dialog = overlay {
            assert_eq!(passes[0].backdrop, passes[1].backdrop, "backdrop at once");
        }
    }
}

/// egui measures a popup, and a modal shown for the first time, in a
/// sizing pass that draws nothing, at a place egui guesses: the panel's
/// box is not drawn there either, also under reduced motion, where no
/// fade hides it. Every pass draws the panel at its place or not at all.
#[test]
fn an_opening_overlay_draws_its_box_only_in_place() {
    for overlay in Overlay::ALL {
        let (opening, timed) = enter_passes(overlay, true, &[1.0]);
        let settled = timed[0].panel.expect("settled").0;
        for (n, drawn) in opening.iter().enumerate() {
            // A box of the panel's size (the trigger's own box is smaller).
            if let Some((rect, _)) = drawn.panel.filter(|(r, _)| r.size() == settled.size()) {
                assert_eq!(rect, settled, "{overlay:?}: opening pass {n}");
            }
        }
    }
}
