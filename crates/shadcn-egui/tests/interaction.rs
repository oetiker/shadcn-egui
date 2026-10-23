//! Pointer, focus and keys as on upstream's element (conformance 2.6, 4.1):
//! a press focuses without a ring, a keyboard focus shows one, and each
//! widget clicks on its own activation keys.

mod common;

use common::Loop;
use egui::{Key, Rect};
use shadcn_egui::components::button::Button;
use shadcn_egui::components::checkbox::Checkbox;
use shadcn_egui::components::select::Select;
use shadcn_egui::components::shared::{interaction_state, pressable, ActivationKeys};
use shadcn_egui::components::slider::Slider;
use shadcn_egui::components::switch::Switch;
use shadcn_egui::components::tabs::Tabs;
use shadcn_egui::style::State;
use shadcn_egui::Theme;
use std::cell::Cell;

/// Two pressable targets side by side; returns each one's state per pass.
fn two_targets(ctx: &egui::Context, events: Vec<egui::Event>) -> [State; 2] {
    let mut st = [State::NONE; 2];
    common::pass(Loop::RunUi, ctx, events, |ui| {
        let theme = Theme::vega_light();
        for (i, x) in [0.0, 100.0].into_iter().enumerate() {
            let rect = Rect::from_min_size(egui::pos2(x, 0.0), egui::vec2(80.0, 30.0));
            let mut resp = ui.interact(rect, egui::Id::new(("target", i)), egui::Sense::click());
            pressable(&mut resp, ActivationKeys::Button);
            st[i] = interaction_state(&theme, &resp, State::NONE);
        }
    });
    st
}

/// A pointer press focuses the target without a ring; Tab then moves focus
/// with one, and a press takes the ring away again.
#[test]
fn press_focuses_without_ring_and_keyboard_focus_shows_it() {
    let ctx = egui::Context::default();
    two_targets(&ctx, vec![]);
    let [a, _] = two_targets(&ctx, common::pointer_down(egui::pos2(40.0, 15.0)));
    assert!(a.contains(State::FOCUS), "{a:?}");
    assert!(!a.contains(State::FOCUS_VISIBLE), "{a:?}");
    two_targets(&ctx, common::pointer_up(egui::pos2(40.0, 15.0)));
    let [a, _] = two_targets(&ctx, vec![]);
    assert!(a.contains(State::FOCUS) && !a.contains(State::FOCUS_VISIBLE));

    two_targets(&ctx, common::key(Key::Tab));
    let [a, b] = two_targets(&ctx, vec![]);
    assert!(!a.contains(State::FOCUS), "{a:?}");
    assert!(b.contains(State::FOCUS | State::FOCUS_VISIBLE), "{b:?}");

    two_targets(&ctx, common::pointer_down(egui::pos2(40.0, 15.0)));
    let [a, b] = two_targets(&ctx, common::pointer_up(egui::pos2(40.0, 15.0)));
    assert!(a.contains(State::FOCUS) && !a.contains(State::FOCUS_VISIBLE));
    assert!(!b.contains(State::FOCUS));
}

/// A `<button>`: Enter clicks on press; Space shows `ACTIVE` while held and
/// clicks on release; a Space released after focus left clicks nothing.
#[test]
fn button_clicks_on_enter_press_and_space_release() {
    let ctx = egui::Context::default();
    let (rect, clicks, active) = (Cell::new(Rect::NOTHING), Cell::new(0), Cell::new(false));
    let run = |events| {
        common::pass(Loop::RunUi, &ctx, events, |ui| {
            Theme::vega_light().apply(ui.ctx());
            let resp = ui.add(Button::new("Save"));
            rect.set(resp.rect);
            clicks.set(clicks.get() + usize::from(resp.clicked()));
            active.set(
                interaction_state(&Theme::vega_light(), &resp, State::NONE).contains(State::ACTIVE),
            );
            ui.add(Button::new("Other"));
        });
    };
    run(vec![]);
    run(common::click(rect.get().center()));
    assert_eq!(clicks.get(), 1, "the press focuses, the release clicks");
    run(common::key(Key::Enter));
    assert_eq!(clicks.get(), 2, "Enter");
    run(common::key_down(Key::Space));
    assert_eq!(clicks.get(), 2, "Space down is no click");
    run(vec![]);
    assert!(active.get(), "a held Space presses the button");
    run(common::key_up(Key::Space));
    assert_eq!(clicks.get(), 3, "Space up clicks");
    assert!(!active.get());
    run(common::key(Key::Space));
    assert_eq!(clicks.get(), 4, "a Space tap clicks");

    run(common::key_down(Key::Space));
    run(common::key(Key::Tab));
    run(common::key_up(Key::Space));
    assert_eq!(clicks.get(), 4, "focus left before Space came up");
}

/// bits' checkbox toggles on Space, never on Enter.
#[test]
fn checkbox_toggles_on_space_only() {
    let ctx = egui::Context::default();
    let (rect, checked) = (Cell::new(Rect::NOTHING), Cell::new(false));
    let run = |events| {
        common::pass(Loop::RunUi, &ctx, events, |ui| {
            Theme::vega_light().apply(ui.ctx());
            let mut c = checked.get();
            rect.set(ui.add(Checkbox::new(&mut c)).rect);
            checked.set(c);
        });
    };
    run(vec![]);
    run(common::click(rect.get().center()));
    assert!(checked.get(), "a click toggles and focuses");
    run(common::key(Key::Enter));
    assert!(checked.get(), "Enter does nothing");
    run(common::key(Key::Space));
    assert!(!checked.get(), "Space toggles");
}

/// A slider's thumb takes focus on press; Space and Enter are no keys of
/// it, so they leave the value where it is.
#[test]
fn slider_focuses_on_press_and_ignores_space() {
    let ctx = egui::Context::default();
    let (rect, value, focused) = (Cell::new(Rect::NOTHING), Cell::new(50.0), Cell::new(false));
    let run = |events| {
        common::pass(Loop::RunUi, &ctx, events, |ui| {
            Theme::vega_light().apply(ui.ctx());
            ui.set_width(200.0);
            let mut v = value.get();
            let resp = ui.add(Slider::new(&mut v, 0.0..=100.0));
            rect.set(resp.rect);
            focused.set(resp.has_focus());
            value.set(v);
        });
    };
    run(vec![]);
    let quarter = rect.get().lerp_inside(egui::vec2(0.25, 0.5));
    run(common::click(quarter));
    assert!(focused.get(), "the press focuses the slider");
    let pressed = value.get();
    run(vec![egui::Event::PointerMoved(rect.get().right_center())]);
    run(common::key(Key::Space));
    run(common::key(Key::Enter));
    assert_eq!(value.get(), pressed, "Space and Enter move nothing");
}

/// bits' tabs activate automatically: the press focuses a trigger and so
/// selects it, before the release.
#[test]
fn tab_trigger_selects_on_press() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    let active = Cell::new(0);
    let run = |events| {
        common::pass(Loop::RunUi, &ctx, events, |ui| {
            Theme::vega_light().apply(ui.ctx());
            let mut a = active.get();
            ui.add(Tabs::new(&mut a, &["Account", "Password"]));
            active.set(a);
        })
    };
    let out = run(vec![]);
    let password = common::bounds_by_label(&out, "Password").expect("trigger");
    run(common::pointer_down(password.center()));
    assert_eq!(active.get(), 1, "selected on press");
    assert!(ctx.memory(|m| m.focused().is_some()), "and focused");
}

/// bits' switch toggles on Enter and Space, both on keydown, once per key
/// repeat; the key coming up does nothing.
#[test]
fn switch_toggles_on_enter_and_space_presses_with_repeats() {
    let ctx = egui::Context::default();
    let (rect, on) = (Cell::new(Rect::NOTHING), Cell::new(false));
    let run = |events| {
        common::pass(Loop::RunUi, &ctx, events, |ui| {
            Theme::vega_light().apply(ui.ctx());
            let mut v = on.get();
            rect.set(ui.add(Switch::new(&mut v)).rect);
            on.set(v);
        });
    };
    run(vec![]);
    run(common::click(rect.get().center()));
    assert!(on.get(), "a click toggles and focuses");
    run(common::key(Key::Enter));
    assert!(!on.get(), "Enter");
    run(common::key(Key::Space));
    assert!(on.get(), "Space");
    run(common::key_down(Key::Enter));
    assert!(!on.get(), "on keydown");
    run(vec![egui::Event::Key {
        key: Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: true,
        modifiers: egui::Modifiers::NONE,
    }]);
    assert!(on.get(), "a repeat toggles again");
    run(common::key_up(Key::Enter));
    assert!(on.get(), "keyup does nothing");
}

/// bits' select trigger opens on Enter, Space and ArrowDown, on keydown.
#[test]
fn closed_select_opens_on_enter_space_and_arrow_down() {
    const FRUITS: [&str; 3] = ["Apple", "Banana", "Cherry"];
    let ctx = egui::Context::default();
    let (open, value) = (Cell::new(false), Cell::new(0usize));
    let run = |events| {
        common::pass(Loop::RunUi, &ctx, events, |ui| {
            Theme::vega_light().apply(ui.ctx());
            let mut v = value.get();
            let resp = ui.add(Select::new("fruit", &mut v, &FRUITS));
            open.set(egui::Popup::is_id_open(
                ui.ctx(),
                egui::Popup::default_response_id(&resp),
            ));
            value.set(v);
        });
    };
    run(vec![]);
    run(common::key(Key::Tab));
    assert!(!open.get());
    for k in [Key::Enter, Key::Space, Key::ArrowDown] {
        run(common::key_down(k));
        assert!(open.get(), "{k:?} opens");
        run(common::key_up(k));
        // Enter chooses the highlighted option and closes the list.
        run(common::key(Key::Enter));
        assert!(!open.get(), "after {k:?}, Enter closes");
        assert_eq!(value.get(), 0, "the highlight stayed on the value");
    }
}
