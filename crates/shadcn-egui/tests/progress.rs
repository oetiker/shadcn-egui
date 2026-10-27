//! Progress: geometry, the indicator's offset, motion, accessibility and
//! snapshots.

mod common;

use egui::accesskit::Role;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::components::progress::{progress, Progress};
use shadcn_egui::components::slider::Slider;
use shadcn_egui::style::Style;
use shadcn_egui::{Palette, Theme};

const MODE: egui::Theme = egui::Theme::Light;

/// `add` in a top-down `Ui` `width` wide.
fn in_width(
    width: f32,
    mut add: impl FnMut(&mut egui::Ui) -> egui::Response,
) -> impl FnMut(&mut egui::Ui) -> egui::Response {
    move |ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(width, 100.0),
            egui::Layout::top_down(egui::Align::Min),
            &mut add,
        )
        .inner
    }
}

/// Every path shape a frame painted with `fill`: a cut indicator is a
/// convex polygon.
fn filled_paths(out: &egui::FullOutput, fill: egui::Color32) -> Vec<Vec<egui::Pos2>> {
    fn walk(s: &egui::Shape, fill: egui::Color32, out: &mut Vec<Vec<egui::Pos2>>) {
        match s {
            egui::Shape::Path(p) if p.fill == fill => out.push(p.points.clone()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, fill, out)),
            _ => {}
        }
    }
    let mut paths = Vec::new();
    out.shapes
        .iter()
        .for_each(|c| walk(&c.shape, fill, &mut paths));
    paths
}

/// The horizontal and vertical extent of what a frame painted in `color`:
/// the rects of that fill and the meshes that hold vertices of that colour.
/// egui feathers a mesh's edge from half a pixel inside (opaque) to half a
/// pixel outside (transparent), so an edge sits halfway between the opaque
/// vertices' extent and the whole mesh's; where a clip cut the fringe away
/// the two agree. A filled path counts by its points.
fn painted_extent(out: &egui::FullOutput, color: egui::Color32) -> Option<egui::Rect> {
    let mut found = egui::Rect::NOTHING;
    for points in filled_paths(out, color) {
        found = found.union(egui::Rect::from_points(&points));
    }
    for r in common::rect_shapes(out) {
        if r.fill == color && r.rect.width() > 0.0 {
            found = found.union(r.rect);
        }
    }
    for m in common::mesh_shapes(out) {
        let bounds = |pred: &dyn Fn(&egui::epaint::Vertex) -> bool| {
            egui::Rect::from_points(
                &m.vertices
                    .iter()
                    .filter(|v| pred(v))
                    .map(|v| v.pos)
                    .collect::<Vec<_>>(),
            )
        };
        let opaque = bounds(&|v| v.color == color);
        if !opaque.is_finite() {
            continue;
        }
        let all = bounds(&|_| true);
        found = found.union(egui::Rect::from_min_max(
            opaque.min.lerp(all.min, 0.5),
            opaque.max.lerp(all.max, 0.5),
        ));
    }
    (found.width() > 0.0).then_some(found)
}

/// The left and right edge of what a frame painted in `color`.
fn painted_span(out: &egui::FullOutput, color: egui::Color32) -> Option<(f32, f32)> {
    painted_extent(out, color).map(|r| (r.left(), r.right()))
}

/// The progress rect and the span of its indicator (vega, light, settled).
fn indicator(p: Progress, width: f32) -> (egui::Rect, Option<(f32, f32)>) {
    let theme = Theme::vega();
    let primary = theme.palette_of(MODE).primary;
    // measure_output runs two passes; the widget is `Copy`.
    let (rect, out) = common::measure_output(&theme, MODE, in_width(width, |ui| ui.add(p)));
    (rect, painted_span(&out, primary))
}

/// Within a quarter pixel: where the indicator ends on the track's edge,
/// the track's clip cuts its outer anti-aliasing fringe away.
fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= 0.25
}

#[test]
fn fills_the_width_at_the_styles_height() {
    let theme = Theme::vega();
    let rect = common::measure(&theme, MODE, in_width(300.0, |ui| progress(ui, 0.5)));
    assert_eq!(rect.width(), 300.0);
    assert_eq!(
        rect.height(),
        *theme.style.progress.height.base.as_ref().unwrap()
    );
    let nova = Theme::new(Style::nova(), Palette::light(), Palette::dark(), 10.0);
    let rect = common::measure(&nova, MODE, in_width(300.0, |ui| progress(ui, 0.5)));
    assert_eq!(rect.height(), 4.0);
}

#[test]
fn desired_width_is_capped_by_the_room() {
    let theme = Theme::vega();
    let narrow = Progress::new(0.5).desired_width(120.0);
    let rect = common::measure(&theme, MODE, in_width(300.0, |ui| ui.add(narrow)));
    assert_eq!(rect.width(), 120.0);
    let wide = Progress::new(0.5).desired_width(500.0);
    let rect = common::measure(&theme, MODE, in_width(300.0, |ui| ui.add(wide)));
    assert_eq!(rect.width(), 300.0);
}

#[test]
fn the_indicator_covers_the_fraction_from_the_start() {
    let (rect, span) = indicator(Progress::new(0.25), 200.0);
    let (lo, hi) = span.expect("an indicator");
    assert!(near(lo, rect.left()), "{lo} vs {rect:?}");
    assert!(near(hi, rect.left() + 50.0), "{hi} vs {rect:?}");
}

#[test]
fn from_value_divides_by_max() {
    let (rect, span) = indicator(Progress::from_value(56.0, 100.0), 200.0);
    let (lo, hi) = span.expect("an indicator");
    assert!(near(lo, rect.left()));
    assert!(near(hi, rect.left() + 112.0), "{hi}");
    let (_, same) = indicator(Progress::new(0.56), 200.0);
    assert_eq!(span, same);
}

#[test]
fn zero_and_negative_values_show_no_indicator() {
    assert_eq!(indicator(Progress::new(0.0), 200.0).1, None);
    assert_eq!(indicator(Progress::from_value(-20.0, 100.0), 200.0).1, None);
}

/// `translateX(-{100 - 100 * value / max}%)` past `max` moves the
/// indicator right of the track start: the track shows it from there to
/// its end, as the browser does.
#[test]
fn a_value_past_max_shifts_the_indicator_as_upstream() {
    let (rect, span) = indicator(Progress::from_value(150.0, 100.0), 200.0);
    let (lo, hi) = span.expect("an indicator");
    assert!(near(lo, rect.left() + 100.0), "{lo} vs {rect:?}");
    assert!(near(hi, rect.right()), "{hi} vs {rect:?}");
    assert_eq!(indicator(Progress::new(2.0), 200.0).1, None);
}

/// A non-finite offset makes the browser drop the `transform`: the
/// indicator fills the track, never NaN geometry.
#[test]
fn a_non_finite_offset_fills_the_track() {
    for p in [
        Progress::from_value(5.0, 0.0),
        Progress::from_value(0.0, 0.0),
        Progress::new(f32::NAN),
        Progress::new(f32::INFINITY),
    ] {
        let (rect, span) = indicator(p, 200.0);
        let (lo, hi) = span.expect("an indicator");
        assert!(
            near(lo, rect.left()) && near(hi, rect.right()),
            "{p:?}: {lo}..{hi}"
        );
    }
}

/// At 0.2 % the indicator is narrower than the track's round cap: the
/// track's `overflow-x-hidden` leaves the circular segment of the cap, as
/// the browser does, not a rect that pokes out of it.
#[test]
fn a_sliver_shows_a_segment_of_the_round_cap() {
    for style in [Style::vega(), Style::maia()] {
        let name = style.name;
        let theme = Theme::new(style, Palette::light(), Palette::dark(), 10.0);
        let primary = theme.palette_of(MODE).primary;
        let (rect, out) =
            common::measure_output(&theme, MODE, in_width(300.0, |ui| progress(ui, 0.002)));
        let r = rect.height() / 2.0;
        let centre = egui::pos2(rect.left() + r, rect.center().y);
        let paths = filled_paths(&out, primary);
        assert_eq!(paths.len(), 1, "{name}");
        for p in &paths[0] {
            assert!(
                (*p - centre).length() <= r + 1e-3,
                "{name}: {p:?} outside the cap of {rect:?}"
            );
        }
        // 0.6 px into the cap: a chord well short of the track's height.
        let seg = painted_extent(&out, primary).expect("an indicator");
        assert!(
            seg.height() < rect.height() * 0.9,
            "{name}: {seg:?} in {rect:?}"
        );
    }
}

#[test]
fn the_track_is_muted() {
    let theme = Theme::vega();
    let muted = theme.palette_of(MODE).muted;
    let (rect, out) = common::measure_output(&theme, MODE, in_width(200.0, |ui| progress(ui, 0.5)));
    assert_eq!(painted_span(&out, muted), Some((rect.left(), rect.right())));
}

/// One pass at `time` showing `fraction`: the indicator's right edge and
/// whether the pass asks for a repaint.
fn timed_pass(ctx: &egui::Context, time: f64, fraction: f32) -> (f32, bool) {
    let out = ctx.run_ui(
        egui::RawInput {
            time: Some(time),
            ..Default::default()
        },
        |ui| {
            Theme::vega().apply(ui.ctx());
            in_width(200.0, |ui| progress(ui, fraction))(ui);
        },
    );
    let primary = Theme::vega().palette_of(egui::Theme::Light).primary;
    let (_, hi) = painted_span(&out, primary).expect("an indicator");
    let repaint =
        out.viewport_output[&egui::ViewportId::ROOT].repaint_delay == std::time::Duration::ZERO;
    (hi, repaint)
}

/// `transition-all` on the indicator: a new value slides over Tailwind's
/// 150 ms; the first pass shows the value at once.
#[test]
fn a_new_value_slides_over_150_ms() {
    let ctx = egui::Context::default();
    ctx.set_theme(egui::Theme::Light);
    let (start, _) = timed_pass(&ctx, 1.0, 0.13);
    timed_pass(&ctx, 1.5, 0.13);
    let (first, moving) = timed_pass(&ctx, 2.0, 0.66);
    assert!(near(first, start), "{first} vs {start}");
    assert!(moving);
    let (mid, moving) = timed_pass(&ctx, 2.075, 0.66);
    assert!(mid > start + 1.0 && mid < start + 106.0 - 1.0, "{mid}");
    assert!(moving);
    let (end, _) = timed_pass(&ctx, 2.2, 0.66);
    assert!(near(end - start, 106.0), "{start} -> {end}");
    assert!(!timed_pass(&ctx, 2.4, 0.66).1);
}

#[test]
fn reduced_motion_jumps() {
    let ctx = egui::Context::default();
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|s| s.animation_time = 0.0);
    let (start, _) = timed_pass(&ctx, 1.0, 0.13);
    let (end, moving) = timed_pass(&ctx, 2.0, 0.66);
    assert!(near(end - start, 106.0));
    assert!(!moving);
}

/// bits-ui's root: `role="progressbar"`, `aria-valuenow`,
/// `aria-valuemin` 0 and `aria-valuemax`.
#[test]
fn reports_a_progressbar_with_value_and_range() {
    let mut h = Harness::new_ui(|ui| {
        ui.add(Progress::from_value(56.0, 200.0));
    });
    h.run();
    let node = h.get_by_role(Role::ProgressIndicator);
    let node = node.accesskit_node();
    assert_eq!(node.numeric_value(), Some(56.0));
    assert_eq!(node.min_numeric_value(), Some(0.0));
    assert_eq!(node.max_numeric_value(), Some(200.0));
}

#[test]
fn snapshot() {
    let (mut a, b) = (50.0_f32, 0.66_f32);
    common::snapshot_all("progress", egui::vec2(360.0, 220.0), |ui| {
        shadcn_egui::layout::vstack(ui, 16.0, |ui| {
            // progress-demo: `w-[60%]`.
            let w = ui.available_width() * 0.6;
            ui.add(Progress::new(b).desired_width(w));
            // progress-controlled: `max-w-sm flex-col gap-4`.
            shadcn_egui::layout::vstack(ui, 16.0, |ui| {
                ui.add(Progress::from_value(a, 100.0));
                ui.add(Slider::new(&mut a, 0.0..=100.0).step(1.0).label("Value"));
            });
            progress(ui, 0.0);
            progress(ui, 1.0);
            progress(ui, 0.01);
        });
    });
}
