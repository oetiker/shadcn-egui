//! Pointer, focus and keys as on upstream's element (conformance 2.6, 4.1):
//! a press focuses without a ring, a keyboard focus shows one, and each
//! widget clicks on its own activation keys; a disabled control shows the
//! cursor upstream gives it.

mod common;

use common::Loop;
use egui::{Key, Rect};
use shadcn_egui::components::button::Button;
use shadcn_egui::components::checkbox::Checkbox;
use shadcn_egui::components::input::Input;
use shadcn_egui::components::label::Label;
use shadcn_egui::components::select::Select;
use shadcn_egui::components::shared::{interaction_state, pressable, ActivationKeys};
use shadcn_egui::components::slider::Slider;
use shadcn_egui::components::switch::Switch;
use shadcn_egui::components::tabs::Tabs;
use shadcn_egui::components::textarea::Textarea;
use shadcn_egui::style::State;
use shadcn_egui::Theme;
use std::cell::Cell;

/// Two pressable targets side by side; returns each one's state per pass.
fn two_targets(ctx: &egui::Context, events: Vec<egui::Event>) -> [State; 2] {
    let mut st = [State::NONE; 2];
    common::pass(Loop::RunUi, ctx, events, |ui| {
        let theme = common::resolved(&Theme::vega(), egui::Theme::Light);
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

/// The browser takes `:focus-visible` when focus moves: a pointer press or
/// drag that moves no focus leaves the ring of a keyboard focus, and a key
/// pressed on a widget a click focused shows the ring, a Ctrl shortcut not.
#[test]
fn the_ring_follows_focus_changes_not_every_press() {
    let ctx = egui::Context::default();
    two_targets(&ctx, vec![]);
    two_targets(&ctx, common::key(Key::Tab));
    let [a, _] = two_targets(&ctx, vec![]);
    assert!(a.contains(State::FOCUS_VISIBLE), "Tab: {a:?}");
    // A drag on empty ground: no click, so egui keeps the focus.
    let (from, to) = (egui::pos2(300.0, 200.0), egui::pos2(360.0, 200.0));
    two_targets(&ctx, common::pointer_down(from));
    two_targets(&ctx, vec![egui::Event::PointerMoved(to)]);
    let [a, _] = two_targets(&ctx, common::pointer_up(to));
    assert!(
        a.contains(State::FOCUS | State::FOCUS_VISIBLE),
        "drag: {a:?}"
    );

    let b = egui::pos2(140.0, 15.0);
    two_targets(&ctx, common::pointer_down(b));
    let [_, bs] = two_targets(&ctx, common::pointer_up(b));
    assert!(bs.contains(State::FOCUS) && !bs.contains(State::FOCUS_VISIBLE));
    let [_, bs] = two_targets(&ctx, common::key_with(egui::Modifiers::COMMAND, Key::S));
    assert!(!bs.contains(State::FOCUS_VISIBLE), "a shortcut: {bs:?}");
    let [_, bs] = two_targets(&ctx, common::key(Key::ArrowLeft));
    assert!(bs.contains(State::FOCUS_VISIBLE), "a key: {bs:?}");
}

/// The modality follows input in passes where no crate widget asks for
/// it: [`Theme::apply`]'s pass hooks take a key pressed in such a pass.
#[test]
fn the_modality_sees_passes_without_crate_widgets() {
    let ctx = egui::Context::default();
    let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(80.0, 30.0));
    let run = |events, ask: bool| {
        let mut st = State::NONE;
        common::pass(Loop::RunUi, &ctx, events, |ui| {
            Theme::vega().apply(ui.ctx());
            ui.ctx().set_theme(egui::Theme::Light);
            let mut resp = ui.interact(rect, egui::Id::new("t"), egui::Sense::click());
            if ask {
                pressable(&mut resp, ActivationKeys::Button);
                st = interaction_state(&Theme::current(ui.ctx()), &resp, State::NONE);
            }
        });
        st
    };
    run(vec![], true);
    run(common::pointer_down(rect.center()), true);
    let st = run(common::pointer_up(rect.center()), true);
    assert!(st.contains(State::FOCUS) && !st.contains(State::FOCUS_VISIBLE));
    run(common::key(Key::ArrowLeft), false);
    let st = run(vec![], true);
    assert!(st.contains(State::FOCUS_VISIBLE), "{st:?}");
}

/// A `<button>`: Enter clicks on press; Space shows `ACTIVE` while held and
/// clicks on release; a Space released after focus left clicks nothing.
#[test]
fn button_clicks_on_enter_press_and_space_release() {
    let ctx = egui::Context::default();
    let (rect, clicks, active) = (Cell::new(Rect::NOTHING), Cell::new(0), Cell::new(false));
    let run = |events| {
        common::pass(Loop::RunUi, &ctx, events, |ui| {
            Theme::vega().apply(ui.ctx());
            ui.ctx().set_theme(egui::Theme::Light);
            let resp = ui.add(Button::new("Save"));
            rect.set(resp.rect);
            clicks.set(clicks.get() + usize::from(resp.clicked()));
            active.set(
                interaction_state(&Theme::current(ui.ctx()), &resp, State::NONE)
                    .contains(State::ACTIVE),
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
            Theme::vega().apply(ui.ctx());
            ui.ctx().set_theme(egui::Theme::Light);
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
            Theme::vega().apply(ui.ctx());
            ui.ctx().set_theme(egui::Theme::Light);
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
            Theme::vega().apply(ui.ctx());
            ui.ctx().set_theme(egui::Theme::Light);
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
            Theme::vega().apply(ui.ctx());
            ui.ctx().set_theme(egui::Theme::Light);
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
            Theme::vega().apply(ui.ctx());
            ui.ctx().set_theme(egui::Theme::Light);
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

/// The click target of a checkbox and a switch is larger than the
/// visible control: a click beside the control, inside the target, still
/// toggles it.
#[test]
fn a_click_beside_a_check_control_inside_its_target_toggles() {
    for switch in [false, true] {
        let ctx = egui::Context::default();
        let (rect, on) = (Cell::new(Rect::NOTHING), Cell::new(false));
        let run = |events| {
            common::pass(Loop::RunUi, &ctx, events, |ui| {
                Theme::vega().apply(ui.ctx());
                ui.ctx().set_theme(egui::Theme::Light);
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    let mut c = on.get();
                    rect.set(if switch {
                        ui.add(Switch::new(&mut c)).rect
                    } else {
                        ui.add(Checkbox::new(&mut c)).rect
                    });
                    on.set(c);
                });
            });
        };
        run(vec![]);
        let beside = rect.get().left_center() - egui::vec2(4.0, 0.0);
        run(common::click(beside));
        assert!(on.get(), "switch {switch}: a click beside toggles");
        let above = rect.get().center_top() - egui::vec2(0.0, 4.0);
        run(common::click(above));
        assert!(!on.get(), "switch {switch}: a click above toggles");
    }
}

/// One control of [`a_disabled_control_shows_the_not_allowed_cursor`]:
/// drawn enabled or with its own `enabled(false)`.
type Control = fn(&mut egui::Ui, bool) -> egui::Response;

/// Upstream's `disabled:cursor-not-allowed` (checkbox, switch, select
/// trigger, textarea; the label as `peer-disabled:`): the pointer over a
/// disabled control, its hit area beside it included, shows the
/// not-allowed cursor, whether its own flag or a disabled `Ui` disables
/// it. The input's rule comes with `disabled:pointer-events-none`, so the
/// browser shows the cursor of what lies under it: the default one here.
/// An enabled control never shows it.
#[test]
fn a_disabled_control_shows_the_not_allowed_cursor() {
    let controls: [(&str, Control, Option<f32>, bool); 6] = [
        (
            "checkbox",
            |ui, on| ui.add(Checkbox::new(&mut false).enabled(on)),
            Some(4.0),
            true,
        ),
        (
            "switch",
            |ui, on| ui.add(Switch::new(&mut false).enabled(on)),
            Some(4.0),
            true,
        ),
        (
            "select",
            |ui, on| ui.add(Select::new("fruit", &mut 0, &["Apple", "Pear"]).enabled(on)),
            None,
            true,
        ),
        (
            "label",
            |ui, on| ui.add(Label::new("Email").enabled(on)),
            None,
            true,
        ),
        (
            "textarea",
            |ui, on| ui.add_enabled(on, Textarea::new(&mut String::new())),
            None,
            true,
        ),
        (
            "input",
            |ui, on| ui.add(Input::new(&mut String::new()).enabled(on)),
            None,
            false,
        ),
    ];
    for (name, control, beside, shows) in controls {
        for (how, enabled, ui_enabled) in [
            ("enabled", true, true),
            ("own flag", false, true),
            ("disabled Ui", true, false),
        ] {
            let ctx = egui::Context::default();
            let rect = Cell::new(Rect::NOTHING);
            let run = |events| {
                common::pass(Loop::RunUi, &ctx, events, |ui| {
                    Theme::vega().apply(ui.ctx());
                    ui.ctx().set_theme(egui::Theme::Light);
                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.add_enabled_ui(ui_enabled, |ui| rect.set(control(ui, enabled).rect));
                    });
                })
            };
            run(vec![]);
            let mut points = vec![rect.get().center()];
            points.extend(beside.map(|d| rect.get().left_center() - egui::vec2(d, 0.0)));
            for at in points {
                let cursor = run(vec![egui::Event::PointerMoved(at)])
                    .platform_output
                    .cursor_icon;
                let disabled = !(enabled && ui_enabled);
                if disabled && shows {
                    assert_eq!(
                        cursor,
                        egui::CursorIcon::NotAllowed,
                        "{name}, {how}, at {at:?}"
                    );
                } else {
                    assert_ne!(
                        cursor,
                        egui::CursorIcon::NotAllowed,
                        "{name}, {how}, at {at:?}"
                    );
                }
                if disabled && !shows {
                    assert_eq!(cursor, egui::CursorIcon::Default, "{name}, {how}");
                }
            }
        }
    }
}

/// The text `ui_fn` paints that contains `needle`: its bounds in a pass.
fn text_bounds(out: &egui::FullOutput, needle: &str) -> Rect {
    common::text_shapes(out)
        .into_iter()
        .find(|t| t.galley.text().contains(needle))
        .map(|t| t.visual_bounding_rect())
        .unwrap_or_else(|| panic!("no text containing {needle:?}"))
}

/// The text a pass asks the platform to copy, if any.
fn copied_text(out: &egui::FullOutput) -> Option<String> {
    out.platform_output.commands.iter().find_map(|c| match c {
        egui::OutputCommand::CopyText(s) => Some(s.clone()),
        _ => None,
    })
}

/// Press on the text containing `needle` near its start, drag a little
/// way into it and release, then move along the text with no button held
/// and press Ctrl+C. Returns what that copies.
fn select_then_hover(
    app_loop: Loop,
    needle: &str,
    mut ui_fn: impl FnMut(&mut egui::Ui),
) -> Option<String> {
    let ctx = egui::Context::default();
    let mut out = common::pass(app_loop, &ctx, vec![], &mut ui_fn);
    for _ in 0..2 {
        out = common::pass(app_loop, &ctx, vec![], &mut ui_fn);
    }
    let r = text_bounds(&out, needle);
    let (y, start) = (r.center().y, r.left() + 1.0);
    let end = start + r.width() * 0.4;
    let _ = common::pass(
        app_loop,
        &ctx,
        common::pointer_down(egui::pos2(start, y)),
        &mut ui_fn,
    );
    let _ = common::pass(
        app_loop,
        &ctx,
        vec![egui::Event::PointerMoved(egui::pos2(end, y))],
        &mut ui_fn,
    );
    let _ = common::pass(
        app_loop,
        &ctx,
        common::pointer_up(egui::pos2(end, y)),
        &mut ui_fn,
    );
    let _ = common::pass(
        app_loop,
        &ctx,
        vec![egui::Event::PointerMoved(egui::pos2(r.right() - 1.0, y))],
        &mut ui_fn,
    );
    copied_text(&common::pass(
        app_loop,
        &ctx,
        vec![egui::Event::Copy],
        &mut ui_fn,
    ))
}

/// cpu-rendering.md section 5 tells a custom loop to run frames through
/// `Context::run_ui`: only there does egui end a label selection on the
/// release and hand Ctrl+C's text to the platform. Through its own
/// `begin_pass`/`end_pass` the selection follows the pointer after the
/// release and nothing is copied.
#[test]
fn dialog_text_selection_ends_on_release_only_under_run_ui() {
    use shadcn_egui::components::dialog::Dialog;
    let title = "Delete this session now";
    let dialog = |ui: &mut egui::Ui| {
        Theme::vega().apply(ui.ctx());
        Dialog::new("d", title).show(ui.ctx(), &mut true, |_| ());
    };
    let copied = select_then_hover(Loop::RunUi, title, dialog).expect("run_ui copies");
    assert!(
        !copied.is_empty() && title.starts_with(&copied) && copied.len() < title.len() - 4,
        "the selection stops where the button came up: {copied:?}"
    );
    assert_eq!(select_then_hover(Loop::Own, title, dialog), None);
}
