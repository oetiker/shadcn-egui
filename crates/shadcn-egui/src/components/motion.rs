//! Upstream's CSS transitions (rules 7, D-88): a part's value that changes
//! moves to its new value over the recipe's `duration-*`, along its
//! `ease-*` curve, when its `transition-*` names the property; otherwise it
//! jumps. `style.animation_time == 0` counts as reduced motion: every
//! transition jumps at once.

use crate::style::{pick, Easing, Recipe, State, Transition};
use egui::{Context, Id};

/// Tailwind 4's `--default-transition-duration`, in seconds: the duration
/// of a transition whose part sets no `duration-*`.
const DEFAULT_DURATION: f32 = 0.15; // behaviour-ok: Tailwind --default-transition-duration

impl Easing {
    /// The eased progress at `t` in `0..=1`, as CSS computes the timing
    /// function: 0 at the start, 1 at the end.
    pub fn at(self, t: f32) -> f32 {
        let (x1, y1, x2, y2) = match self {
            Easing::Linear => return t.clamp(0.0, 1.0), // behaviour-ok: the unit interval
            Easing::In => (0.4, 0.0, 1.0, 1.0),         // behaviour-ok: Tailwind --ease-in
            Easing::Out => (0.0, 0.0, 0.2, 1.0),        // behaviour-ok: Tailwind --ease-out
            Easing::InOut => (0.4, 0.0, 0.2, 1.0),      // behaviour-ok: Tailwind --ease-in-out
        };
        cubic_bezier(x1, y1, x2, y2, t)
    }
}

/// CSS `cubic-bezier(x1, y1, x2, y2)` at `t`: the curve's y where its x is
/// `t`. x rises monotonically for x1, x2 in `0..=1`, so bisection finds
/// the curve parameter.
fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64, t: f32) -> f32 {
    let at_end = t <= 0.0 || t >= 1.0; // behaviour-ok: the unit interval
    if at_end {
        // The curve's fixed ends, exactly.
        return t.clamp(0.0, 1.0); // behaviour-ok: the unit interval
    }
    // One coordinate of the Bernstein form with ends 0 and 1.
    let at = |a: f64, b: f64, s: f64| {
        let u = 1.0 - s; // behaviour-ok: Bernstein polynomial
        3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s // behaviour-ok: Bernstein polynomial
    };
    let t = f64::from(t);
    let (mut lo, mut hi) = (0.0, 1.0); // behaviour-ok: the unit interval
    for _ in 0..40 {
        // behaviour-ok: bisection steps, below f32 precision
        let mid = (lo + hi) / 2.0;
        if at(x1, x2, mid) < t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    at(y1, y2, (lo + hi) / 2.0) as f32
}

/// One running (or finished) transition, kept per id in the context's
/// temporary data.
#[derive(Clone, Copy, Debug)]
struct Run {
    from: f32,
    to: f32,
    start: f64,
    duration: f32,
    easing: Easing,
    /// CSS's reversing-adjusted start value and reversing shortening
    /// factor: a transition back to where the last one came from takes
    /// only as long as the way already covered.
    reverse_to: f32,
    shortening: f32,
}

impl Run {
    fn settled(value: f32) -> Run {
        Run {
            from: value,
            to: value,
            start: 0.0,
            duration: 0.0,
            easing: Easing::Linear,
            reverse_to: value,
            shortening: 1.0, // behaviour-ok: CSS's initial shortening factor
        }
    }

    /// The value and eased progress at `now`, and whether it still runs.
    fn at(&self, now: f64) -> (f32, f32, bool) {
        let elapsed = (now - self.start) as f32;
        if self.duration <= 0.0 || elapsed >= self.duration {
            return (self.to, 1.0, false); // behaviour-ok: the finished progress
        }
        let eased = self.easing.at(elapsed.max(0.0) / self.duration);
        (self.from + (self.to - self.from) * eased, eased, true)
    }
}

/// The value to paint this pass for a property of a part that changes to
/// `target`, timed by the part's recipe stack `layers` under state `s`:
/// when the picked `transition` names `property`, the value moves from where
/// it is to `target` over the picked `duration` (Tailwind's 150 ms without
/// one) along the picked `easing` ([`Easing::InOut`] without one), as a CSS
/// transition does; else it jumps. Under reduced motion
/// (`style.animation_time == 0`) it jumps too. `id` names the property of
/// this widget; the first pass that sees it shows `target`. Asks for a
/// repaint only while the value moves.
///
/// ```
/// use shadcn_egui::components::motion::transition;
/// use shadcn_egui::style::{Prop, Recipe, State, Transition};
///
/// let indicator = Recipe {
///     transition: Prop::fixed(Transition::ALL),
///     ..Recipe::default()
/// };
/// let ctx = egui::Context::default();
/// let _ = ctx.run_ui(Default::default(), |ui| {
///     let width = transition(
///         ui.ctx(),
///         ui.id().with("width"),
///         120.0,
///         &[&indicator],
///         Transition::OTHER,
///         State::NONE,
///     );
///     assert_eq!(width, 120.0);
/// });
/// ```
pub fn transition(
    ctx: &Context,
    id: Id,
    target: f32,
    layers: &[&Recipe],
    property: Transition,
    s: State,
) -> f32 {
    let animates = pick(layers, |r| &r.transition, s).is_some_and(|t| t.contains(property))
        && ctx.global_style().animation_time > 0.0;
    let now = ctx.input(|i| i.time);
    let run = match ctx.data(|d| d.get_temp::<Run>(id)) {
        Some(run) if animates => {
            let (current, progress, running) = run.at(now);
            if target == run.to {
                run
            } else {
                let duration = pick(layers, |r| &r.duration, s).unwrap_or(DEFAULT_DURATION);
                let easing = pick(layers, |r| &r.easing, s).unwrap_or(Easing::InOut);
                // CSS Transitions 1, "starting of transitions", step 4.
                let (reverse_to, shortening) = if running && target == run.reverse_to {
                    let f = progress * run.shortening + 1.0 - run.shortening; // behaviour-ok: CSS reversing shortening factor
                    (run.to, f.abs().clamp(0.0, 1.0)) // behaviour-ok: the unit interval
                } else {
                    (current, 1.0) // behaviour-ok: CSS's initial shortening factor
                };
                Run {
                    from: current,
                    to: target,
                    start: now,
                    duration: duration * shortening,
                    easing,
                    reverse_to,
                    shortening,
                }
            }
        }
        _ => Run::settled(target),
    };
    let (value, _, running) = run.at(now);
    ctx.data_mut(|d| {
        d.insert_temp(id, if running { run } else { Run::settled(target) });
    });
    if running {
        ctx.request_repaint();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_control_points_give_linear_timing() {
        for t in [0.1, 0.3, 0.5, 0.9] {
            assert!((cubic_bezier(0.25, 0.25, 0.75, 0.75, t) - t).abs() < 1e-5);
        }
    }

    #[test]
    fn every_easing_starts_at_zero_and_ends_at_one() {
        for e in [Easing::Linear, Easing::In, Easing::Out, Easing::InOut] {
            assert_eq!(e.at(0.0), 0.0, "{e:?}");
            assert_eq!(e.at(1.0), 1.0, "{e:?}");
        }
        assert!(Easing::Out.at(0.5) > 0.5);
        assert!(Easing::In.at(0.5) < 0.5);
    }
}
