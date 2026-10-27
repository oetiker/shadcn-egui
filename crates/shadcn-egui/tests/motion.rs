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
