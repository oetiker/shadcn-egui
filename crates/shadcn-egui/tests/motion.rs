//! The transition helper (rules 7, D-88): upstream's timing, reduced motion,
//! and repaints only while a value moves.

use shadcn_egui::components::motion::transition;
use shadcn_egui::style::{Easing, Prop, Recipe, State, Style, Transition};

const PROPERTY: Transition = Transition::OTHER;

/// One pass at `time` seconds: the helper's value for `target` under
/// `layers`, and whether the pass ends with a repaint request (egui adds
/// one trailing repaint after the pass that last asked).
fn pass(ctx: &egui::Context, time: f64, target: f32, layers: &[&Recipe]) -> (f32, bool) {
    let mut value = f32::NAN;
    let out = ctx.run_ui(
        egui::RawInput {
            time: Some(time),
            ..Default::default()
        },
        |ui| {
            value = transition(
                ui.ctx(),
                egui::Id::new("motion-test"),
                target,
                layers,
                PROPERTY,
                State::NONE,
            );
        },
    );
    let repaint =
        out.viewport_output[&egui::ViewportId::ROOT].repaint_delay == std::time::Duration::ZERO;
    (value, repaint)
}

/// A context past egui's own start-up repaints.
fn fresh() -> egui::Context {
    let ctx = egui::Context::default();
    for time in [0.0, 0.1, 0.2] {
        let _ = ctx.run_ui(
            egui::RawInput {
                time: Some(time),
                ..Default::default()
            },
            |_| {},
        );
    }
    ctx
}

fn timed(duration: Option<f32>, easing: Option<Easing>) -> Recipe {
    Recipe {
        transition: Prop::fixed(Transition::ALL),
        duration: Prop {
            base: duration,
            ..Prop::default()
        },
        easing: Prop {
            base: easing,
            ..Prop::default()
        },
        ..Recipe::default()
    }
}

/// CSS's `cubic-bezier(x1, y1, x2, y2)` at `t`, solved independently of
/// the crate with Newton steps in f64.
fn reference_bezier(x1: f64, y1: f64, x2: f64, y2: f64, t: f64) -> f64 {
    let b = |a: f64, c: f64, s: f64| {
        3.0 * (1.0 - s).powi(2) * s * a + 3.0 * (1.0 - s) * s * s * c + s.powi(3)
    };
    let db = |a: f64, c: f64, s: f64| {
        3.0 * (1.0 - s).powi(2) * a + 6.0 * (1.0 - s) * s * (c - a) + 3.0 * s * s * (1.0 - c)
    };
    let mut s = t;
    for _ in 0..50 {
        s -= (b(x1, x2, s) - t) / db(x1, x2, s);
    }
    b(y1, y2, s)
}

#[test]
fn value_reaches_its_target_after_the_duration() {
    let ctx = fresh();
    let r = timed(Some(0.2), None);
    assert_eq!(pass(&ctx, 1.0, 0.0, &[&r]).0, 0.0);
    let (start, moving) = pass(&ctx, 2.0, 100.0, &[&r]);
    assert_eq!(start, 0.0, "the change pass shows the start value");
    assert!(moving);
    let (half, moving) = pass(&ctx, 2.1, 100.0, &[&r]);
    assert!(half > 0.0 && half < 100.0, "{half}");
    assert!(moving);
    assert_eq!(pass(&ctx, 2.2, 100.0, &[&r]).0, 100.0);
    // egui repaints once more after the last request; then no more.
    assert_eq!(pass(&ctx, 2.3, 100.0, &[&r]), (100.0, false));
}

#[test]
fn midpoint_follows_the_easing_curve() {
    let curves = [
        (None, (0.4, 0.0, 0.2, 1.0)), // Tailwind's default timing function
        (Some(Easing::InOut), (0.4, 0.0, 0.2, 1.0)),
        (Some(Easing::In), (0.4, 0.0, 1.0, 1.0)),
        (Some(Easing::Out), (0.0, 0.0, 0.2, 1.0)),
        (Some(Easing::Linear), (0.0, 0.0, 1.0, 1.0)),
    ];
    for (easing, (x1, y1, x2, y2)) in curves {
        for t in [0.25, 0.5, 0.75] {
            let ctx = fresh();
            let r = timed(None, easing);
            pass(&ctx, 1.0, 0.0, &[&r]);
            pass(&ctx, 2.0, 1.0, &[&r]);
            // Tailwind's default duration: 150 ms.
            let (v, _) = pass(&ctx, 2.0 + 0.15 * t, 1.0, &[&r]);
            let want = reference_bezier(x1, y1, x2, y2, t) as f32;
            assert!((v - want).abs() < 1e-3, "{easing:?} at {t}: {v} != {want}");
        }
    }
}

#[test]
fn reduced_motion_jumps_without_a_repaint() {
    let ctx = fresh();
    ctx.global_style_mut(|s| s.animation_time = 0.0);
    let r = timed(Some(0.2), None);
    pass(&ctx, 1.0, 0.0, &[&r]);
    assert_eq!(pass(&ctx, 2.0, 100.0, &[&r]), (100.0, false));
}

#[test]
fn a_property_the_transition_does_not_name_jumps() {
    let ctx = fresh();
    let r = Recipe {
        transition: Prop::fixed(Transition::COLORS),
        ..Recipe::default()
    };
    pass(&ctx, 1.0, 0.0, &[&r]);
    assert_eq!(pass(&ctx, 2.0, 100.0, &[&r]), (100.0, false));
    // No transition at all jumps too.
    pass(&ctx, 3.0, 50.0, &[]);
    assert_eq!(pass(&ctx, 4.0, 50.0, &[]), (50.0, false));
}

#[test]
fn a_settled_value_asks_for_no_repaint() {
    let ctx = fresh();
    let r = timed(None, None);
    assert_eq!(pass(&ctx, 1.0, 40.0, &[&r]), (40.0, false));
    assert_eq!(pass(&ctx, 2.0, 40.0, &[&r]), (40.0, false));
}

#[test]
fn a_reversed_transition_takes_only_the_way_covered() {
    // CSS Transitions 1: going back to the start value mid-way shortens the
    // new transition by the eased progress of the running one.
    let ctx = fresh();
    let r = timed(Some(0.2), Some(Easing::Linear));
    pass(&ctx, 1.0, 0.0, &[&r]);
    pass(&ctx, 2.0, 100.0, &[&r]);
    let (at, _) = pass(&ctx, 2.05, 0.0, &[&r]);
    assert!((at - 25.0).abs() < 1e-3, "{at}");
    // A quarter of the way covered: the way back takes 50 ms, not 200.
    assert!(pass(&ctx, 2.07, 0.0, &[&r]).1);
    assert_eq!(pass(&ctx, 2.11, 0.0, &[&r]).0, 0.0);
    assert_eq!(pass(&ctx, 2.2, 0.0, &[&r]), (0.0, false));
}

/// One pass at `time` that does not show the widget.
fn skip(ctx: &egui::Context, time: f64) {
    let _ = ctx.run_ui(
        egui::RawInput {
            time: Some(time),
            ..Default::default()
        },
        |_| {},
    );
}

#[test]
fn a_change_while_not_shown_runs_on_wall_time() {
    // CSS runs a transition whether the element paints or not: a widget
    // skipped for longer than the duration shows its new target at once.
    let ctx = fresh();
    let r = timed(Some(0.2), Some(Easing::Linear));
    pass(&ctx, 1.0, 0.0, &[&r]);
    for time in [1.1, 1.2, 1.3] {
        skip(&ctx, time);
    }
    assert_eq!(pass(&ctx, 1.4, 100.0, &[&r]), (100.0, false));
    // Skipped for half the duration, it is half way.
    skip(&ctx, 1.45);
    let (half, moving) = pass(&ctx, 1.5, 0.0, &[&r]);
    assert!((half - 50.0).abs() < 1e-3, "{half}");
    assert!(moving);
}

#[test]
fn every_style_times_the_progress_indicator() {
    // `transition-all` on the indicator, no duration or easing: Tailwind's
    // 150 ms along its default curve.
    for style in [
        Style::vega(),
        Style::nova(),
        Style::maia(),
        Style::lyra(),
        Style::mira(),
        Style::luma(),
        Style::rhea(),
    ] {
        let ind = &*style.progress_indicator;
        let ctx = fresh();
        pass(&ctx, 1.0, 0.0, &[ind]);
        pass(&ctx, 2.0, 1.0, &[ind]);
        let (v, moving) = pass(&ctx, 2.075, 1.0, &[ind]);
        let want = reference_bezier(0.4, 0.0, 0.2, 1.0, 0.5) as f32;
        assert!((v - want).abs() < 1e-3, "{}: {v} != {want}", style.name);
        assert!(moving);
        assert_eq!(pass(&ctx, 2.15, 1.0, &[ind]).0, 1.0, "{}", style.name);
        assert_eq!(pass(&ctx, 2.3, 1.0, &[ind]), (1.0, false), "{}", style.name);
    }
}

/// One pass of a vega switch showing `on` at `time`, `reduced` setting
/// `animation_time` to 0: the left edge of the last filled rect (the
/// thumb), and whether the pass ends with a repaint request.
fn switch_pass(ctx: &egui::Context, time: f64, on: bool, reduced: bool) -> (f32, bool) {
    let out = ctx.run_ui(
        egui::RawInput {
            time: Some(time),
            ..Default::default()
        },
        |ui| {
            shadcn_egui::Theme::vega().apply(ui.ctx());
            if reduced {
                ui.ctx().all_styles_mut(|s| s.animation_time = 0.0);
            }
            let mut on = on;
            ui.add(shadcn_egui::components::switch::Switch::new(&mut on));
        },
    );
    let left = out
        .shapes
        .iter()
        .rev()
        .find_map(|c| match &c.shape {
            egui::Shape::Rect(r) if r.fill.a() > 0 => Some(r.rect.left()),
            _ => None,
        })
        .expect("the thumb");
    let repaint =
        out.viewport_output[&egui::ViewportId::ROOT].repaint_delay == std::time::Duration::ZERO;
    (left, repaint)
}

#[test]
fn switch_thumb_moves_with_upstreams_timing() {
    // The thumb's `transition-transform`: 150 ms on the default curve.
    let ctx = fresh();
    let (off, _) = switch_pass(&ctx, 1.0, false, false);
    switch_pass(&ctx, 1.5, false, false);
    let (start, moving) = switch_pass(&ctx, 2.0, true, false);
    assert_eq!(start, off);
    assert!(moving);
    let (mid, moving) = switch_pass(&ctx, 2.075, true, false);
    let (on, _) = switch_pass(&ctx, 2.2, true, false);
    assert!(on > off + 1.0);
    let want = off + (on - off) * reference_bezier(0.4, 0.0, 0.2, 1.0, 0.5) as f32;
    assert!((mid - want).abs() < 1e-3, "{mid} != {want}");
    assert!(moving);
    // Settled: the pass after arrival asks for nothing.
    assert_eq!(switch_pass(&ctx, 2.4, true, false), (on, false));
}

#[test]
fn switch_thumb_jumps_under_reduced_motion() {
    let ctx = fresh();
    let (off, _) = switch_pass(&ctx, 1.0, false, true);
    switch_pass(&ctx, 1.5, false, true);
    let (on, moving) = switch_pass(&ctx, 2.0, true, true);
    assert!(on > off + 1.0);
    assert!(!moving);
}

/// One pass of a vega default button at `time`, the pointer moving to
/// `pointer` if given, in the central panel or in an `Area`: the button's rect, its
/// fill (transparent when unpainted), and whether the pass ends with a repaint request.
fn button_pass(
    ctx: &egui::Context,
    time: f64,
    pointer: Option<egui::Pos2>,
    reduced: bool,
    in_area: bool,
) -> (egui::Rect, egui::Color32, bool) {
    let mut rect = egui::Rect::NOTHING;
    let out = ctx.run_ui(
        egui::RawInput {
            time: Some(time),
            events: pointer.map(egui::Event::PointerMoved).into_iter().collect(),
            ..Default::default()
        },
        |ui| {
            shadcn_egui::Theme::vega().apply(ui.ctx());
            if reduced {
                ui.ctx().all_styles_mut(|s| s.animation_time = 0.0);
            }
            let add = |ui: &mut egui::Ui| {
                ui.add(shadcn_egui::components::button::Button::new("Save"))
                    .rect
            };
            rect = if in_area {
                egui::Area::new(egui::Id::new("area"))
                    .fixed_pos(egui::pos2(40.0, 40.0))
                    .fade_in(false)
                    .show(ui.ctx(), add)
                    .inner
            } else {
                add(ui)
            };
        },
    );
    let fill = out
        .shapes
        .iter()
        .find_map(|c| match &c.shape {
            // The fill sits inside the border (`bg-clip-padding`).
            egui::Shape::Rect(r) if rect.contains_rect(r.rect) && r.fill.a() > 0 => Some(r.fill),
            _ => None,
        })
        // None on an `Area`'s sizing pass.
        .unwrap_or(egui::Color32::TRANSPARENT);
    let repaint =
        out.viewport_output[&egui::ViewportId::ROOT].repaint_delay == std::time::Duration::ZERO;
    (rect, fill, repaint)
}

fn near(a: egui::Color32, b: egui::Color32) -> bool {
    a.to_array()
        .iter()
        .zip(b.to_array())
        .all(|(x, y)| x.abs_diff(y) <= 1)
}

#[test]
fn button_background_fades_on_hover() {
    // `transition-all`, no duration or easing: 150 ms on the default
    // curve, the colours mixed in OKLab. Screen and Area alike.
    for in_area in [false, true] {
        let ctx = fresh();
        // An `Area` paints nothing on its first, sizing pass.
        button_pass(&ctx, 0.5, None, false, in_area);
        let away = Some(egui::pos2(-50.0, -50.0));
        let (rect, idle, _) = button_pass(&ctx, 1.0, away, false, in_area);
        button_pass(&ctx, 1.5, None, false, in_area);
        let (_, start, moving) = button_pass(&ctx, 2.0, Some(rect.center()), false, in_area);
        assert_eq!(start, idle);
        assert!(moving);
        let (_, mid, _) = button_pass(&ctx, 2.075, None, false, in_area);
        let (_, hover, _) = button_pass(&ctx, 2.2, None, false, in_area);
        assert_ne!(hover, idle);
        let eased = reference_bezier(0.4, 0.0, 0.2, 1.0, 0.5) as f32;
        let want = shadcn_egui::color::mix_oklab(idle, hover, eased);
        assert!(near(mid, want), "area {in_area}: {mid:?} != {want:?}");
        // Settled: the pass after arrival asks for nothing.
        let (_, settled, moving) = button_pass(&ctx, 2.4, None, false, in_area);
        assert_eq!((settled, moving), (hover, false));
    }
}

#[test]
fn button_background_jumps_under_reduced_motion() {
    let ctx = fresh();
    // The theme's fonts reach the atlas on the second pass.
    button_pass(&ctx, 0.5, Some(egui::pos2(-50.0, -50.0)), true, false);
    let (rect, idle, _) = button_pass(&ctx, 1.0, None, true, false);
    let (_, hover, _) = button_pass(&ctx, 2.0, Some(rect.center()), true, false);
    assert_ne!(hover, idle);
    // The pointer's move repaints, and egui adds one trailing repaint;
    // nothing moves after that.
    button_pass(&ctx, 2.01, None, true, false);
    assert_eq!(
        button_pass(&ctx, 2.02, None, true, false),
        (rect, hover, false)
    );
}

#[test]
fn a_colour_the_transition_does_not_name_jumps() {
    use shadcn_egui::components::shared::RecipeBox;
    use shadcn_egui::style::{Paint, Role};
    // `transition-[color,box-shadow]` (vega's input): the background is
    // not named and jumps; under `transition-colors` it fades.
    for (transition, fades) in [
        (Transition::COLOR.or(Transition::SHADOW), false),
        (Transition::COLORS, true),
    ] {
        let part = |role| Recipe {
            bg: Prop::fixed(Paint::Role(role, 1.0)),
            transition: Prop::fixed(transition),
            ..Recipe::default()
        };
        let (from, to) = (part(Role::Muted), part(Role::Primary));
        let theme = shadcn_egui::Theme::vega();
        let ctx = fresh();
        let fill = |time: f64, r: &Recipe| {
            let mut fill = None;
            let _ = ctx.run_ui(
                egui::RawInput {
                    time: Some(time),
                    ..Default::default()
                },
                |ui| {
                    fill = RecipeBox::resolve(ui.ctx(), &theme, &[r], State::NONE)
                        .transitioned(ui.ctx(), egui::Id::new("box"), &[r], State::NONE)
                        .fill();
                },
            );
            fill.expect("a fill")
        };
        let idle = fill(1.0, &from);
        let start = fill(2.0, &to);
        let mid = fill(2.075, &to);
        let target = fill(3.0, &to);
        assert_ne!(target, idle);
        if fades {
            assert_eq!(start, idle);
            assert!(mid != idle && mid != target, "{mid:?}");
        } else {
            assert_eq!((start, mid), (target, target));
            assert_eq!(mid, target);
        }
    }
}

/// A recipe whose `prop` changes under `FOCUS_VISIBLE`, timed by `t`.
fn focus_recipe(t: Transition) -> Recipe {
    use shadcn_egui::style::{Paint, Role, Rule, ShadowLevel, When};
    let when = When::new(State::FOCUS_VISIBLE, State::NONE);
    let mut r = Recipe {
        transition: Prop::fixed(t),
        ..Recipe::default()
    };
    r.outline_width.rules.push(Rule {
        when,
        rank: 1,
        value: 2.0,
    });
    r.outline_color.rules.push(Rule {
        when,
        rank: 1,
        value: Paint::Role(Role::Ring, 1.0),
    });
    r.shadow.rules.push(Rule {
        when,
        rank: 1,
        value: ShadowLevel::Md,
    });
    r
}

/// The shapes of `r`'s box on one pass at `time` under `s`, through
/// `RecipeBox::transitioned`.
fn box_pass(ctx: &egui::Context, time: f64, r: &Recipe, s: State) -> Vec<egui::Shape> {
    use shadcn_egui::components::shared::RecipeBox;
    let theme = shadcn_egui::Theme::vega();
    let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(40.0, 20.0));
    let mut shapes = Vec::new();
    let _ = ctx.run_ui(
        egui::RawInput {
            time: Some(time),
            ..Default::default()
        },
        |ui| {
            shapes = RecipeBox::resolve(ui.ctx(), &theme, &[r], s)
                .transitioned(ui.ctx(), egui::Id::new("box"), &[r], s)
                .shapes(ui.ctx(), rect);
        },
    );
    shapes
}

/// The outline's width among `shapes`: the only stroked rect.
fn outline_width(shapes: &[egui::Shape]) -> f32 {
    shapes
        .iter()
        .find_map(|s| match s {
            egui::Shape::Rect(r) if r.stroke.width > 0.0 => Some(r.stroke.width),
            _ => None,
        })
        .unwrap_or(0.0)
}

/// The darkest alpha of the shadow mesh among `shapes`.
fn shadow_alpha(shapes: &[egui::Shape]) -> u8 {
    shapes
        .iter()
        .filter_map(|s| match s {
            egui::Shape::Mesh(m) => m.vertices.iter().map(|v| v.color.a()).max(),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn outline_width_moves_under_transition_all_only() {
    let ctx = fresh();
    let r = focus_recipe(Transition::ALL);
    box_pass(&ctx, 1.0, &r, State::NONE);
    assert_eq!(
        outline_width(&box_pass(&ctx, 2.0, &r, State::FOCUS_VISIBLE)),
        0.0
    );
    let half = outline_width(&box_pass(&ctx, 2.075, &r, State::FOCUS_VISIBLE));
    let want = 2.0 * reference_bezier(0.4, 0.0, 0.2, 1.0, 0.5) as f32;
    assert!((half - want).abs() < 1e-3, "{half} != {want}");
    assert_eq!(
        outline_width(&box_pass(&ctx, 2.2, &r, State::FOCUS_VISIBLE)),
        2.0
    );
    // `transition-colors` does not name `outline-width`: it jumps.
    let ctx = fresh();
    let r = focus_recipe(Transition::COLORS);
    box_pass(&ctx, 1.0, &r, State::NONE);
    assert_eq!(
        outline_width(&box_pass(&ctx, 2.0, &r, State::FOCUS_VISIBLE)),
        2.0
    );
}

#[test]
fn shadow_elevation_fades_where_box_shadow_is_named() {
    let ctx = fresh();
    let r = focus_recipe(Transition::COLOR.or(Transition::SHADOW));
    box_pass(&ctx, 1.0, &r, State::NONE);
    let full = {
        let ctx = fresh();
        shadow_alpha(&box_pass(&ctx, 1.0, &r, State::FOCUS_VISIBLE))
    };
    assert!(full > 0);
    assert_eq!(
        shadow_alpha(&box_pass(&ctx, 2.0, &r, State::FOCUS_VISIBLE)),
        0
    );
    let half = shadow_alpha(&box_pass(&ctx, 2.075, &r, State::FOCUS_VISIBLE));
    assert!(half > 0 && half < full, "{half} of {full}");
    assert_eq!(
        shadow_alpha(&box_pass(&ctx, 2.2, &r, State::FOCUS_VISIBLE)),
        full
    );
    // `transition-colors` does not name `box-shadow`: it jumps.
    let ctx = fresh();
    let r = focus_recipe(Transition::COLORS);
    box_pass(&ctx, 1.0, &r, State::NONE);
    assert_eq!(
        shadow_alpha(&box_pass(&ctx, 2.0, &r, State::FOCUS_VISIBLE)),
        full
    );
}

#[test]
fn disabled_opacity_fades_where_opacity_is_named() {
    use shadcn_egui::components::shared::part_opacity;
    use shadcn_egui::style::{Rule, When};
    let fade = |t: Transition| {
        let mut r = Recipe {
            transition: Prop::fixed(t),
            ..Recipe::default()
        };
        r.opacity.rules.push(Rule {
            when: When::new(State::DISABLED, State::NONE),
            rank: 1,
            value: 0.5,
        });
        r
    };
    let at = |ctx: &egui::Context, time: f64, r: &Recipe, s: State| {
        let mut o = f32::NAN;
        let _ = ctx.run_ui(
            egui::RawInput {
                time: Some(time),
                ..Default::default()
            },
            |ui| o = part_opacity(ui, egui::Id::new("part"), &[&[r]], s),
        );
        o
    };
    let ctx = fresh();
    let r = fade(Transition::ALL);
    assert_eq!(at(&ctx, 1.0, &r, State::NONE), 1.0);
    assert_eq!(at(&ctx, 2.0, &r, State::DISABLED), 1.0);
    // Tailwind's 150 ms along its default curve, at 75 ms.
    let half = at(&ctx, 2.075, &r, State::DISABLED);
    let want = 1.0 - 0.5 * reference_bezier(0.4, 0.0, 0.2, 1.0, 0.5) as f32;
    assert!((half - want).abs() < 1e-3, "{half} != {want}");
    assert_eq!(at(&ctx, 2.2, &r, State::DISABLED), 0.5);
    // `transition-colors` does not name `opacity`: it jumps.
    let ctx = fresh();
    let r = fade(Transition::COLORS);
    at(&ctx, 1.0, &r, State::NONE);
    assert_eq!(at(&ctx, 2.0, &r, State::DISABLED), 0.5);
}

/// One pass of a vega switch showing `on` at `time`, `shift` px from the
/// left: the thumb's left edge minus the track's, and whether the pass
/// ends with a repaint request.
fn moved_switch_pass(ctx: &egui::Context, time: f64, on: bool, shift: f32) -> (f32, bool) {
    let out = ctx.run_ui(
        egui::RawInput {
            time: Some(time),
            ..Default::default()
        },
        |ui| {
            shadcn_egui::Theme::vega().apply(ui.ctx());
            ui.horizontal(|ui| {
                ui.add_space(shift);
                let mut on = on;
                ui.add(shadcn_egui::components::switch::Switch::new(&mut on));
            });
        },
    );
    let lefts: Vec<f32> = out
        .shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Rect(r) if r.fill.a() > 0 => Some(r.rect.left()),
            _ => None,
        })
        .collect();
    let repaint =
        out.viewport_output[&egui::ViewportId::ROOT].repaint_delay == std::time::Duration::ZERO;
    (lefts[lefts.len() - 1] - lefts[0], repaint)
}

#[test]
fn switch_thumb_stays_in_its_track_when_the_switch_moves() {
    // The thumb's `translate` is relative to the switch: moving the
    // switch moves the thumb with it, nothing transitions.
    let ctx = fresh();
    moved_switch_pass(&ctx, 1.0, true, 0.0);
    let (home, _) = moved_switch_pass(&ctx, 1.5, true, 0.0);
    assert!(home > 1.0, "{home}");
    for time in [2.0, 2.02, 2.04] {
        assert_eq!(moved_switch_pass(&ctx, time, true, 40.0), (home, false));
    }
}

#[test]
fn a_nan_target_settles() {
    let ctx = fresh();
    let r = timed(None, None);
    pass(&ctx, 1.0, 0.0, &[&r]);
    let (v, moving) = pass(&ctx, 2.0, f32::NAN, &[&r]);
    assert!(v.is_nan() && !moving);
    let (v, moving) = pass(&ctx, 3.0, f32::NAN, &[&r]);
    assert!(v.is_nan() && !moving);
    // From a NaN there is no way to move along: the next value jumps.
    assert_eq!(pass(&ctx, 4.0, 5.0, &[&r]), (5.0, false));
}

#[test]
fn a_property_moves_where_the_transition_names_any_part_of_it() {
    use shadcn_egui::components::motion::transition_color;
    // `transition-[color,box-shadow]` names `color`: a colour asked for
    // under `Transition::COLORS` fades.
    let r = Recipe {
        transition: Prop::fixed(Transition::COLOR.or(Transition::SHADOW)),
        ..Recipe::default()
    };
    let ctx = fresh();
    let at = |time: f64, c: egui::Color32| {
        let mut out = c;
        let _ = ctx.run_ui(
            egui::RawInput {
                time: Some(time),
                ..Default::default()
            },
            |ui| {
                out = transition_color(
                    ui.ctx(),
                    egui::Id::new("c"),
                    c,
                    &[&r],
                    Transition::COLORS,
                    State::NONE,
                )
            },
        );
        out
    };
    at(1.0, egui::Color32::BLACK);
    assert_eq!(at(2.0, egui::Color32::WHITE), egui::Color32::BLACK);
    let mid = at(2.075, egui::Color32::WHITE);
    assert!(mid != egui::Color32::BLACK && mid != egui::Color32::WHITE);
}
