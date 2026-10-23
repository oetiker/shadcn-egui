//! The shared test harness in `tests/common`: one test per helper.

mod common;

use common::Loop;
use egui::{pos2, Key};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use shadcn_egui::components::button::Button;
use shadcn_egui::components::input::Input;
use shadcn_egui::Theme;
use std::cell::Cell;

/// Both widths of a responsive fixture: the input's `md:text-sm` gives
/// 16 px text on the phone and 14 px on the desktop.
#[test]
fn snapshot_widths_renders_phone_and_desktop() {
    let mut text = String::from("you@example.com");
    common::snapshot_widths(
        "harness_widths",
        80.0,
        vec![("vega_light", Theme::vega_light())],
        |ui| {
            ui.add(Input::new(&mut text));
        },
    );
}

/// A press and its release in one pass clicks a button, under both app
/// loops; the press alone in one pass and the release in the next does too.
#[test]
fn click_in_one_pass_under_both_loops() {
    for app_loop in Loop::BOTH {
        let ctx = egui::Context::default();
        let (rect, clicks) = (Cell::new(egui::Rect::NOTHING), Cell::new(0));
        let run = |events| {
            common::pass(app_loop, &ctx, events, |ui| {
                Theme::vega_light().apply(ui.ctx());
                let resp = ui.add(Button::new("Save"));
                rect.set(resp.rect);
                clicks.set(clicks.get() + usize::from(resp.clicked()));
            });
        };
        run(vec![]);
        run(common::click(rect.get().center()));
        run(vec![]);
        run(common::pointer_down(rect.get().center()));
        run(common::pointer_up(rect.get().center()));
        run(vec![]);
        assert_eq!(clicks.get(), 2, "{app_loop:?}");
    }
}

/// A tap (press and release in one pass) is pressed and not held; a key
/// that went down stays held until it comes up.
#[test]
fn key_tap_and_hold() {
    let ctx = egui::Context::default();
    let state = |events| {
        let mut seen = (false, false);
        common::own_pass(&ctx, events, |ui| {
            seen = ui.input(|i| (i.key_pressed(Key::A), i.key_down(Key::A)));
        });
        seen
    };
    assert_eq!(state(common::key(Key::A)), (true, false), "tap");
    assert_eq!(state(common::key_down(Key::A)), (true, true), "press");
    assert_eq!(state(vec![]), (false, true), "held");
    assert_eq!(state(common::key_up(Key::A)), (false, false), "released");
    let shift_tab = common::key_with(egui::Modifiers::SHIFT, Key::Tab);
    common::own_pass(&ctx, shift_tab, |ui| {
        let shifted = |pressed: bool| {
            ui.input(|i| {
                i.events.iter().any(|e| {
                    matches!(e, egui::Event::Key { key: Key::Tab, pressed: p, modifiers, .. }
                        if *p == pressed && modifiers.shift)
                })
            })
        };
        assert!(
            shifted(true) && shifted(false),
            "shift held on press and release"
        );
    });
}

/// A pointer path into a button hovers it; its last point is where the
/// pointer stays.
#[test]
fn pointer_path_hovers() {
    let ctx = egui::Context::default();
    let (rect, hovered) = (Cell::new(egui::Rect::NOTHING), Cell::new(false));
    let run = |events| {
        common::own_pass(&ctx, events, |ui| {
            let resp = ui.add(Button::new("Save"));
            rect.set(resp.rect);
            hovered.set(resp.hovered());
        });
    };
    run(vec![]);
    let target = rect.get().center();
    let outside = pos2(rect.get().right() + 200.0, rect.get().bottom() + 200.0);
    let path = common::pointer_path(outside, target, 8);
    assert_eq!(path.len(), 8);
    run(path);
    run(vec![]);
    assert!(hovered.get());
    assert_eq!(ctx.input(|i| i.pointer.hover_pos()), Some(target));
}

/// Tab moves focus to the first button; the focus query names it.
#[test]
fn focused_label_follows_tab() {
    let mut h = Harness::new_ui(|ui| {
        Theme::vega_light().apply(ui.ctx());
        ui.add(Button::new("One"));
        ui.add(Button::new("Two"));
    });
    h.run();
    assert_eq!(common::focused_label(&h), None);
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(common::focused_label(&h).as_deref(), Some("One"));
    h.get_by_label("Two");
}

/// A node's accesskit bounds are its widget's rect.
#[test]
fn bounds_by_label_finds_the_widget() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let mut rect = egui::Rect::NOTHING;
    let mut out = None;
    for _ in 0..2 {
        out = Some(common::pass(Loop::RunUi, &ctx, vec![], |ui| {
            rect = ui.add(Button::new("Save")).rect;
        }));
    }
    let out = out.expect("a pass");
    assert_eq!(common::bounds_by_label(&out, "Save"), Some(rect));
    assert_eq!(common::bounds_by_label(&out, "Cancel"), None);
}
