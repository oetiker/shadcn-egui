//! Scroll Area: bits-ui's hover visibility, the bars' geometry, thumb and
//! track drags, touch-only content drag, keyboard scrolling with Chrome's
//! Tab-stop rule, focus scrolled into view, and snapshots.

mod common;

use egui::{pos2, vec2, Event, Key, Modifiers, Pos2, Rect, Ui, Vec2};
use shadcn_egui::components::scroll_area::ScrollArea;
use shadcn_egui::style::{Paint, Role};
use shadcn_egui::Theme;

/// Where every test places its area: 200 × 200 at (100, 50).
const AREA: Rect = Rect {
    min: pos2(100.0, 50.0),
    max: pos2(300.0, 250.0),
};

/// A context under `theme` in light mode that runs passes at a set time.
struct Sim {
    ctx: egui::Context,
    time: f64,
}

impl Sim {
    fn new(theme: &Theme, mode: egui::Theme) -> Self {
        let ctx = egui::Context::default();
        theme.apply(&ctx);
        ctx.set_theme(mode);
        ctx.all_styles_mut(|s| s.animation_time = 0.0);
        Self { ctx, time: 0.0 }
    }

    fn vega() -> Self {
        Self::new(&Theme::vega(), egui::Theme::Light)
    }

    /// One pass at the current time with `events`; the time then moves on
    /// by a 60 Hz frame.
    fn pass<R>(
        &mut self,
        events: Vec<Event>,
        mut add: impl FnMut(&mut Ui) -> R,
    ) -> (R, egui::FullOutput) {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, common::PASS_WINDOW)),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let mut r = None;
        let mut out = self.ctx.run_ui(input, |ui| r = Some(add(ui)));
        out.textures_delta.clear();
        self.time += 1.0 / 60.0;
        (r.unwrap(), out)
    }

    /// The colour `paint` resolves to under the context's theme.
    fn paint(&self, paint: Paint) -> egui::Color32 {
        Theme::current(&self.ctx).paint(paint)
    }
}

/// `area` at [`AREA`] with `content` in it, 200 px high at most and not
/// shrunk; returns the offset the area left in its state.
fn place(ui: &mut Ui, area: ScrollArea, content: impl FnOnce(&mut Ui)) -> Vec2 {
    ui.scope_builder(egui::UiBuilder::new().max_rect(AREA), |ui| {
        area.max_height(AREA.height())
            .auto_shrink(false)
            .show(ui, content)
            .state
            .clamped_offset()
    })
    .inner
}

/// Content of `size`, painting nothing.
fn block(size: Vec2) -> impl FnOnce(&mut Ui) {
    move |ui| {
        ui.allocate_space(size);
    }
}

/// The vertical area over 1000 px of content.
fn tall(ui: &mut Ui) -> Vec2 {
    place(ui, ScrollArea::vertical(), block(vec2(200.0, 1000.0)))
}

/// Every rect a frame painted in `color` (thumbs are `border`).
fn rects_of(out: &egui::FullOutput, color: egui::Color32) -> Vec<egui::epaint::RectShape> {
    common::rect_shapes(out)
        .into_iter()
        .filter(|r| r.fill == color)
        .collect()
}

fn thumbs(sim: &Sim, out: &egui::FullOutput) -> Vec<Rect> {
    rects_of(out, sim.paint(Paint::Role(Role::Border, 1.0)))
        .iter()
        .map(|r| r.rect)
        .collect()
}

fn moved(p: Pos2) -> Vec<Event> {
    vec![Event::PointerMoved(p)]
}

const INSIDE: Pos2 = pos2(150.0, 150.0);
const OUTSIDE: Pos2 = pos2(500.0, 350.0);

// Track geometry under vega for `tall`: the bar is the right 10 px, its
// border-l 1 px and padding 1 px leave the track x 292..299, y 51..249.
const TRACK_TOP: f32 = 51.0;
const TRACK_LEN: f32 = 198.0;
/// 198 × 200 / 1000.
const THUMB_LEN: f32 = 39.6;
const MAX_SCROLL: f32 = 800.0;

fn offset_for_thumb_move(dy: f32) -> f32 {
    dy / (TRACK_LEN - THUMB_LEN) * MAX_SCROLL
}

#[test]
fn no_bar_without_hover() {
    let mut sim = Sim::vega();
    sim.pass(moved(OUTSIDE), tall);
    let (_, out) = sim.pass(vec![], tall);
    assert!(thumbs(&sim, &out).is_empty(), "{:?}", thumbs(&sim, &out));
}

#[test]
fn hover_shows_the_vertical_bar_only_when_content_overflows() {
    let mut sim = Sim::vega();
    let (_, out) = sim.pass(moved(INSIDE), tall);
    assert_eq!(
        thumbs(&sim, &out).len(),
        1,
        "an overflowing area shows its thumb"
    );

    let mut sim = Sim::vega();
    let short = |ui: &mut Ui| place(ui, ScrollArea::vertical(), block(vec2(200.0, 150.0)));
    let (_, out) = sim.pass(moved(INSIDE), short);
    assert!(
        thumbs(&sim, &out).is_empty(),
        "short content shows no thumb"
    );
}

/// bits compares whole pixels: content 0.4 px taller than the viewport
/// shows no bar, a whole pixel more does.
#[test]
fn sub_pixel_overflow_shows_no_bar() {
    for (height, bars) in [(200.4, 0), (201.0, 1)] {
        let mut sim = Sim::vega();
        let area = |ui: &mut Ui| place(ui, ScrollArea::vertical(), block(vec2(200.0, height)));
        let (_, out) = sim.pass(moved(INSIDE), area);
        assert_eq!(thumbs(&sim, &out).len(), bars, "content {height} px high");
    }
}

#[test]
fn bar_hides_600ms_after_pointer_leaves() {
    let mut sim = Sim::vega();
    sim.pass(moved(INSIDE), tall);
    let left = sim.time;
    sim.pass(moved(OUTSIDE), tall);
    sim.time = left + 0.5;
    let (_, out) = sim.pass(vec![], tall);
    assert_eq!(thumbs(&sim, &out).len(), 1, "still shown at 0.5 s");
    sim.time = left + 0.7;
    let (_, out) = sim.pass(vec![], tall);
    assert!(thumbs(&sim, &out).is_empty(), "hidden at 0.7 s");
}

/// The delay before the next pass the root viewport asked for.
fn repaint_delay(out: &egui::FullOutput) -> std::time::Duration {
    out.viewport_output[&egui::ViewportId::ROOT].repaint_delay
}

/// An app that is idle repaints only when asked: a shown bar asks for a
/// pass at its hide time, the passes it asks for then hide it, and a
/// hidden bar asks for none. egui 0.37 wakes the app at the time asked
/// for (0.34 woke it one predicted frame before).
#[test]
fn a_shown_bar_asks_for_a_pass_at_its_hide_time() {
    let mut sim = Sim::vega();
    let entered = sim.time;
    sim.pass(moved(INSIDE), tall);
    sim.pass(moved(OUTSIDE), tall);
    // The pass after an event asks for the next one at once.
    sim.pass(vec![], tall);
    let quiet = sim.time;
    let (_, out) = sim.pass(vec![], tall);
    assert_eq!(thumbs(&sim, &out).len(), 1, "shown after leaving");
    let delay = repaint_delay(&out).as_secs_f64();
    let hide = entered + 0.6;
    assert!(
        (quiet + delay - hide).abs() < 1e-4,
        "asked for {delay} s at {quiet} s"
    );
    // Run the pass the app asks for, as an integration does.
    sim.time = quiet + delay;
    let (_, out) = sim.pass(vec![], tall);
    assert!(thumbs(&sim, &out).is_empty(), "hidden at the hide time");
    assert_eq!(
        repaint_delay(&out),
        std::time::Duration::MAX,
        "hidden: no pass asked"
    );
}

#[test]
fn thumb_is_border_colour_at_recipe_geometry() {
    for (name, theme, mode) in common::snapshot_themes() {
        let mut sim = Sim::new(&theme, mode);
        let (_, out) = sim.pass(moved(INSIDE), tall);
        let border = sim.paint(Paint::Role(Role::Border, 1.0));
        let found = rects_of(&out, border);
        assert_eq!(found.len(), 1, "{name}: one thumb");
        let thumb = &found[0];
        let r = thumb.rect;
        assert_eq!(
            r.width(),
            7.0,
            "{name}: 10 px bar less border-l and padding"
        );
        assert_eq!(r.right(), AREA.right() - 1.0, "{name}: right padding");
        assert_eq!(
            r.top(),
            AREA.top() + 1.0,
            "{name}: at the top, less padding"
        );
        assert!(
            (r.height() - THUMB_LEN).abs() < 1e-3,
            "{name}: {}",
            r.height()
        );
        let radius = thumb.corner_radius.nw;
        if name.starts_with("lyra") {
            assert_eq!(radius, 0, "{name}: square thumb");
        } else {
            assert!(radius >= 3, "{name}: round thumb, radius {radius}");
        }
    }
}

#[test]
fn thumb_follows_the_offset_and_has_a_minimum_length() {
    let mut sim = Sim::vega();
    sim.pass(moved(INSIDE), tall);
    let id = egui::Id::unique("long");
    let long = |ui: &mut Ui| {
        place(
            ui,
            ScrollArea::vertical().id_salt(id),
            block(vec2(200.0, 100_000.0)),
        )
    };
    let (_, out) = sim.pass(vec![], long);
    let t = thumbs(&sim, &out);
    assert_eq!(t[0].height(), 18.0, "bits' smallest thumb");

    let mut sim = Sim::vega();
    sim.pass(moved(INSIDE), tall);
    // A wheel step down: the thumb moves by its share of the track.
    sim.pass(vec![wheel(-100.0)], tall);
    let (offset, out) = sim.pass(vec![], tall);
    assert!(offset.y > 0.0);
    let top = thumbs(&sim, &out)[0].top();
    let want = TRACK_TOP + offset.y / MAX_SCROLL * (TRACK_LEN - THUMB_LEN);
    assert!((top - want).abs() < 1e-3, "thumb top {top}, want {want}");
}

#[test]
fn content_gets_the_full_width() {
    let mut sim = Sim::vega();
    let mut width = 0.0;
    sim.pass(moved(INSIDE), |ui| {
        // Even under egui's solid bars, which take room from the content.
        ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
        place(ui, ScrollArea::vertical(), |ui| {
            width = ui.available_width();
            ui.allocate_space(vec2(10.0, 1000.0));
        })
    });
    assert_eq!(width, AREA.width(), "the bar takes no room");
}

/// Content wider than a vertical area is cut at the area's width, as
/// bits' `overflow-x: hidden` viewport cuts it: the bar stays at the
/// area's edge and the area takes no more room than it has.
#[test]
fn wider_content_is_cut_at_the_area_edge() {
    let mut sim = Sim::vega();
    let wide = |ui: &mut Ui| {
        ui.scope_builder(egui::UiBuilder::new().max_rect(AREA), |ui| {
            ScrollArea::vertical()
                .max_height(AREA.height())
                .auto_shrink(false)
                .show(ui, block(vec2(300.0, 1000.0)));
        })
        .response
        .rect
    };
    let (_, out) = sim.pass(moved(INSIDE), tall);
    let narrow_thumbs = thumbs(&sim, &out);
    let (used, out) = sim.pass(moved(INSIDE), wide);
    assert_eq!(used, AREA, "the room the area took");
    assert_eq!(
        thumbs(&sim, &out),
        narrow_thumbs,
        "the bar at the area's edge"
    );
}

#[test]
fn horizontal_draws_only_the_x_bar() {
    let mut sim = Sim::vega();
    let wide = |ui: &mut Ui| place(ui, ScrollArea::horizontal(), block(vec2(1000.0, 200.0)));
    let (_, out) = sim.pass(moved(INSIDE), wide);
    let t = thumbs(&sim, &out);
    assert_eq!(t.len(), 1, "{t:?}");
    assert_eq!(t[0].height(), 7.0);
    assert_eq!(t[0].bottom(), AREA.bottom() - 1.0);
}

/// An area that shrinks to its content's height puts its bar at the
/// bottom of that height, not at the bottom of the room it had.
#[test]
fn the_bar_sits_on_a_shrunk_area() {
    let mut sim = Sim::vega();
    let wide = |ui: &mut Ui| {
        ui.scope_builder(egui::UiBuilder::new().max_rect(AREA), |ui| {
            ScrollArea::horizontal().show(ui, block(vec2(1000.0, 100.0)))
        });
    };
    let (_, out) = sim.pass(moved(pos2(150.0, 100.0)), wide);
    let t = thumbs(&sim, &out);
    assert_eq!(t.len(), 1, "{t:?}");
    assert_eq!(t[0].bottom(), AREA.top() + 100.0 - 1.0);
}

#[test]
fn both_shortens_the_bars_by_the_corner() {
    let mut sim = Sim::vega();
    let both = |ui: &mut Ui| place(ui, ScrollArea::both(), block(vec2(1000.0, 1000.0)));
    sim.pass(moved(INSIDE), both);
    // Scroll to the end on both axes: each thumb then ends at its track's end.
    let end = vec![Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(-5000.0, -5000.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }];
    sim.pass(end, both);
    let (_, out) = sim.pass(vec![], both);
    let t = thumbs(&sim, &out);
    assert_eq!(t.len(), 2, "{t:?}");
    let vertical = t.iter().find(|r| r.width() == 7.0).unwrap();
    let horizontal = t.iter().find(|r| r.height() == 7.0).unwrap();
    assert_eq!(
        vertical.bottom(),
        AREA.bottom() - 10.0 - 1.0,
        "stops at the corner"
    );
    assert_eq!(
        horizontal.right(),
        AREA.right() - 10.0 - 1.0,
        "stops at the corner"
    );
}

fn wheel(dy: f32) -> Event {
    Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, dy),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn wheel_over_the_bar_scrolls() {
    let mut sim = Sim::vega();
    let on_bar = pos2(295.0, 100.0);
    sim.pass(moved(on_bar), tall);
    sim.pass(vec![wheel(-100.0)], tall);
    let (offset, _) = sim.pass(vec![], tall);
    assert!(offset.y > 0.0, "{offset:?}");
}

/// Hover, press at `from`, drag to `to` in steps, release; returns the
/// offset after each of: the press, the last move, the release, one more
/// pass.
///
/// A bar drag sets the offset after egui's area ran, which egui 0.37
/// takes only in the next pass: each offset is read in a pass without
/// events after the one that moved it.
fn drag(sim: &mut Sim, from: Pos2, to: Pos2, mut area: impl FnMut(&mut Ui) -> Vec2) -> [Vec2; 4] {
    sim.pass(moved(from), &mut area);
    sim.pass(common::pointer_down(from), &mut area);
    let (pressed, _) = sim.pass(vec![], &mut area);
    let mut last = pressed;
    for e in common::pointer_path(from, to, 5) {
        sim.pass(vec![e], &mut area);
        last = sim.pass(vec![], &mut area).0;
    }
    sim.pass(common::pointer_up(to), &mut area);
    let (released, _) = sim.pass(vec![], &mut area);
    let (after, _) = sim.pass(vec![], &mut area);
    [pressed, last, released, after]
}

#[test]
fn dragging_the_thumb_scrolls() {
    let mut sim = Sim::vega();
    let grab = pos2(295.0, TRACK_TOP + 10.0);
    let [pressed, last, released, after] = drag(&mut sim, grab, grab + vec2(0.0, 50.0), tall);
    assert_eq!(pressed.y, 0.0, "a press on the thumb does not move it");
    let want = offset_for_thumb_move(50.0);
    for (when, got) in [("dragged", last), ("released", released), ("after", after)] {
        assert!((got.y - want).abs() < 0.01, "{when}: {} want {want}", got.y);
    }
}

/// A thumb drag that ends outside the area keeps the bar until 600 ms
/// after the release, as bits counts the drag as hovering.
#[test]
fn a_drag_released_outside_hides_600ms_later() {
    let mut sim = Sim::vega();
    let grab = pos2(295.0, TRACK_TOP + 10.0);
    drag(&mut sim, grab, pos2(500.0, TRACK_TOP + 60.0), tall);
    // `drag` ends with a pass after the release.
    let released = sim.time - 2.0 / 60.0;
    sim.time = released + 0.5;
    let (_, out) = sim.pass(vec![], tall);
    assert_eq!(thumbs(&sim, &out).len(), 1, "still shown at 0.5 s");
    sim.time = released + 0.7;
    let (_, out) = sim.pass(vec![], tall);
    assert!(thumbs(&sim, &out).is_empty(), "hidden at 0.7 s");
}

#[test]
fn the_content_follows_the_thumb_in_the_next_pass() {
    let mut sim = Sim::vega();
    let grab = pos2(295.0, TRACK_TOP + 10.0);
    let mut top = f32::NAN;
    let mut area = |ui: &mut Ui| {
        place(ui, ScrollArea::vertical(), |ui| {
            top = ui.allocate_space(vec2(200.0, 1000.0)).1.top();
        })
    };
    sim.pass(moved(grab), &mut area);
    sim.pass(common::pointer_down(grab), &mut area);
    sim.pass(moved(grab + vec2(0.0, 50.0)), &mut area);
    sim.pass(vec![], &mut area);
    // egui puts the content on whole pixels.
    let want = AREA.top() - offset_for_thumb_move(50.0);
    assert!((top - want).abs() <= 0.5, "content top {top}, want {want}");
}

#[test]
fn track_press_centres_the_thumb_on_the_pointer() {
    let mut sim = Sim::vega();
    let at = pos2(295.0, 200.0);
    let [pressed, last, ..] = drag(&mut sim, at, at + vec2(0.0, 20.0), tall);
    let want = offset_for_thumb_move(200.0 - TRACK_TOP - THUMB_LEN / 2.0);
    assert!(
        (pressed.y - want).abs() < 0.01,
        "press: {} want {want}",
        pressed.y
    );
    let want = offset_for_thumb_move(220.0 - TRACK_TOP - THUMB_LEN / 2.0);
    assert!((last.y - want).abs() < 0.01, "drag: {} want {want}", last.y);
}

/// The stuck-to-end trap: egui snaps a stick-to-bottom area back to the
/// end unless it sees the user's scroll; a thumb drag must let go.
#[test]
fn drag_up_unsticks_stick_to_bottom() {
    let mut sim = Sim::vega();
    let mut area = |ui: &mut Ui| {
        place(
            ui,
            ScrollArea::vertical().stick_to_bottom(true),
            block(vec2(200.0, 1000.0)),
        )
    };
    sim.pass(moved(INSIDE), &mut area);
    let (start, _) = sim.pass(vec![], &mut area);
    assert_eq!(start.y, MAX_SCROLL, "starts stuck to the end");
    let thumb = pos2(295.0, TRACK_TOP + TRACK_LEN - THUMB_LEN / 2.0);
    let [_, last, released, after] = drag(&mut sim, thumb, thumb - vec2(0.0, 50.0), &mut area);
    let want = MAX_SCROLL - offset_for_thumb_move(50.0);
    for (when, got) in [("dragged", last), ("released", released), ("after", after)] {
        assert!((got.y - want).abs() < 0.01, "{when}: {} want {want}", got.y);
    }
}

#[test]
fn stick_to_bottom_follows_new_rows() {
    let mut sim = Sim::vega();
    let mut height = 1000.0;
    for _ in 0..3 {
        let (offset, _) = sim.pass(vec![], |ui| {
            place(
                ui,
                ScrollArea::vertical().stick_to_bottom(true),
                block(vec2(200.0, height)),
            )
        });
        assert_eq!(offset.y, height - AREA.height());
        height += 100.0;
    }
}

#[test]
fn a_bar_drag_selects_no_text() {
    let mut sim = Sim::vega();
    let mut area = |ui: &mut Ui| {
        place(ui, ScrollArea::vertical(), |ui| {
            // Lines that wrap fill the width, under the bar too.
            for i in 0..30 {
                ui.label(format!("line {i} ").repeat(12));
            }
        })
    };
    let grab = pos2(295.0, TRACK_TOP + 5.0);
    drag(&mut sim, grab, grab + vec2(-150.0, 40.0), &mut area);
    let (_, out) = sim.pass(vec![Event::Copy], &mut area);
    let copied = out
        .platform_output
        .commands
        .iter()
        .any(|c| matches!(c, egui::OutputCommand::CopyText(t) if !t.is_empty()));
    assert!(!copied, "the drag began on the bar: no text is selected");
}

/// A mouse drag over the content does not scroll (the browser drags
/// only by touch); after a touch event it does.
#[test]
fn content_drags_only_on_a_touch_screen() {
    let from = pos2(150.0, 200.0);
    let to = pos2(150.0, 120.0);

    let mut sim = Sim::vega();
    // A first pass: egui senses a content drag once it knows the content
    // overflows.
    sim.pass(vec![], tall);
    let [.., after] = drag(&mut sim, from, to, tall);
    assert_eq!(
        after.y, 0.0,
        "a mouse drag over the content scrolls nothing"
    );

    let mut sim = Sim::vega();
    let touch = Event::Touch {
        device_id: egui::TouchDeviceId(1),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::Start,
        pos: from,
        force: None,
    };
    sim.pass(vec![touch], tall);
    let [.., released, _] = drag(&mut sim, from, to, tall);
    assert!(released.y >= 79.0, "a touch drag scrolls: {}", released.y);
}

// Keyboard.

fn tab() -> Vec<Event> {
    common::key(Key::Tab)
}

/// Whether the pass reported a scroll view node to accessibility.
fn has_scroll_view(out: &egui::FullOutput) -> bool {
    out.platform_output
        .accesskit_update
        .as_ref()
        .is_some_and(|u| {
            u.nodes
                .iter()
                .any(|(_, n)| n.role() == egui::accesskit::Role::ScrollView)
        })
}

/// An area that overflows and holds nothing focusable is a Tab stop, a
/// scroll view to accessibility; short content or a focusable widget
/// inside takes the stop away.
#[test]
fn tab_stop_only_when_overflowing_with_nothing_focusable() {
    type Area = fn(&mut Ui) -> Vec2;
    let cases: [(&str, Area, bool); 3] = [
        ("tall", tall, true),
        (
            "short",
            |ui| place(ui, ScrollArea::vertical(), block(vec2(200.0, 150.0))),
            false,
        ),
        (
            "button inside",
            |ui| {
                place(ui, ScrollArea::vertical(), |ui| {
                    ui.allocate_space(vec2(200.0, 900.0));
                    let _ = ui.button("inside");
                })
            },
            false,
        ),
    ];
    for (name, area, stop) in cases {
        let mut sim = Sim::vega();
        sim.ctx.enable_accesskit();
        sim.pass(vec![], area);
        sim.pass(tab(), area);
        let (_, out) = sim.pass(vec![], area);
        let focused = sim.ctx.memory(|m| m.focused());
        if name == "button inside" {
            // The Tab goes to the button, never to the area.
            sim.pass(tab(), area);
            sim.pass(vec![], area);
            assert_eq!(
                sim.ctx.memory(|m| m.focused()),
                focused,
                "{name}: one Tab stop only"
            );
        } else {
            assert_eq!(focused.is_some(), stop, "{name}: Tab focus");
        }
        assert_eq!(has_scroll_view(&out), stop, "{name}: scroll view node");
    }
}

/// A focused area paints the viewport recipe's focus ring; unfocused, none.
#[test]
fn focused_area_shows_the_ring() {
    let mut sim = Sim::vega();
    let ring = sim.paint(Paint::Role(Role::Ring, 0.5));
    let has_ring = |out: &egui::FullOutput| {
        common::rect_shapes(out)
            .iter()
            .any(|r| r.stroke.color == ring && r.stroke.width > 0.0)
            || common::mesh_shapes(out)
                .iter()
                .any(|m| m.vertices.iter().any(|v| v.color == ring))
    };
    let (_, out) = sim.pass(vec![], tall);
    assert!(!has_ring(&out), "no ring before focus");
    sim.pass(tab(), tall);
    let (_, out) = sim.pass(vec![], tall);
    assert!(has_ring(&out), "a ring once Tab focused the area");
}

/// [`tall`] with a button below it, where focus could go.
fn tall_and_button(ui: &mut Ui) -> Vec2 {
    let offset = tall(ui);
    let below = Rect::from_min_size(pos2(100.0, 300.0), vec2(100.0, 30.0));
    ui.scope_builder(egui::UiBuilder::new().max_rect(below), |ui| {
        let _ = ui.button("below");
    });
    offset
}

/// The keys Chrome scrolls a focused scroller with.
#[test]
fn keys_scroll_a_focused_area() {
    let tall = tall_and_button;
    let mut sim = Sim::vega();
    sim.pass(vec![], tall);
    sim.pass(tab(), tall);
    sim.pass(vec![], tall);
    let page = AREA.height() * 0.875;
    let steps: [(&str, Vec<Event>, f32); 7] = [
        ("Down", common::key(Key::ArrowDown), 40.0),
        ("Down", common::key(Key::ArrowDown), 80.0),
        ("Up", common::key(Key::ArrowUp), 40.0),
        ("PageDown", common::key(Key::PageDown), 40.0 + page),
        (
            "Shift+Space",
            common::key_with(Modifiers::SHIFT, Key::Space),
            40.0,
        ),
        ("Space", common::key(Key::Space), 40.0 + page),
        ("End", common::key(Key::End), MAX_SCROLL),
    ];
    for (name, events, want) in steps {
        sim.pass(events, tall);
        let (offset, _) = sim.pass(vec![], tall);
        assert!(
            (offset.y - want).abs() < 0.01,
            "{name}: {} want {want}",
            offset.y
        );
    }
    sim.pass(common::key(Key::PageUp), tall);
    let (offset, _) = sim.pass(vec![], tall);
    assert!(
        (offset.y - (MAX_SCROLL - page)).abs() < 0.01,
        "PageUp: {}",
        offset.y
    );
    sim.pass(common::key(Key::Home), tall);
    let (offset, _) = sim.pass(vec![], tall);
    assert_eq!(offset.y, 0.0, "Home");
    // The arrows stay with the area: focus does not move to the button.
    let focused = sim.ctx.memory(|m| m.focused());
    assert!(focused.is_some());
    sim.pass(common::key(Key::ArrowDown), tall);
    sim.pass(vec![], tall);
    assert_eq!(sim.ctx.memory(|m| m.focused()), focused);
}

/// A browser scroller inside `<fieldset disabled>` stays a Tab stop and
/// scrolls by keys, wheel and thumb; the controls in it stay disabled, so
/// the area is the Tab stop even with a button inside.
#[test]
fn a_disabled_ui_scrolls_by_keys_wheel_and_thumb() {
    fn disabled(ui: &mut Ui) -> Vec2 {
        ui.disable();
        place(ui, ScrollArea::vertical(), |ui| {
            ui.allocate_space(vec2(200.0, 900.0));
            let button = ui.button("inside");
            assert!(!button.enabled(), "the button inside stays disabled");
            ui.allocate_space(vec2(200.0, 100.0));
        })
    }
    fn disabled_tall(ui: &mut Ui) -> Vec2 {
        ui.disable();
        tall(ui)
    }
    let mut sim = Sim::vega();
    sim.ctx.enable_accesskit();
    sim.pass(vec![], disabled);
    sim.pass(tab(), disabled);
    let (_, out) = sim.pass(vec![], disabled);
    assert!(sim.ctx.memory(|m| m.focused()).is_some(), "Tab focus");
    assert!(has_scroll_view(&out), "scroll view node");
    sim.pass(common::key(Key::ArrowDown), disabled);
    let (offset, _) = sim.pass(vec![], disabled);
    assert_eq!(offset.y, 40.0, "ArrowDown");

    let mut sim = Sim::vega();
    sim.pass(moved(INSIDE), disabled_tall);
    sim.pass(vec![wheel(-100.0)], disabled_tall);
    let (offset, _) = sim.pass(vec![], disabled_tall);
    assert!(offset.y > 0.0, "wheel: {offset:?}");

    let mut sim = Sim::vega();
    let grab = pos2(295.0, TRACK_TOP + 10.0);
    let [.., after] = drag(&mut sim, grab, grab + vec2(0.0, 50.0), disabled_tall);
    let want = offset_for_thumb_move(50.0);
    assert!(
        (after.y - want).abs() < 0.01,
        "thumb: {} want {want}",
        after.y
    );
}

/// The offsets a touch pan from `from` up by 80 px leaves, pass by
/// pass, through the press, the moves, the release and a fling's coast,
/// with `area` drawn in a `Ui` that is disabled or not.
fn touch_pan(disabled: bool, from: Pos2, area: fn(&mut Ui) -> [Vec2; 2]) -> Vec<[Vec2; 2]> {
    let mut sim = Sim::vega();
    let mut add = move |ui: &mut Ui| {
        if disabled {
            ui.disable();
        }
        area(ui)
    };
    let touch = Event::Touch {
        device_id: egui::TouchDeviceId(1),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::Start,
        pos: from,
        force: None,
    };
    let mut offsets = vec![sim.pass(vec![touch], &mut add).0];
    offsets.push(sim.pass(moved(from), &mut add).0);
    offsets.push(sim.pass(common::pointer_down(from), &mut add).0);
    // Fast moves, so the release leaves a fling.
    for e in common::pointer_path(from, from - vec2(0.0, 80.0), 4) {
        offsets.push(sim.pass(vec![e], &mut add).0);
    }
    offsets.push(
        sim.pass(common::pointer_up(from - vec2(0.0, 80.0)), &mut add)
            .0,
    );
    for _ in 0..20 {
        offsets.push(sim.pass(vec![], &mut add).0);
    }
    offsets
}

/// A browser pans a scroller inside `<fieldset disabled>` by touch as it
/// pans an enabled one, fling included; in nested areas the inner one
/// under the finger takes the pan, and a drag beside an area shrunk to
/// its content pans nothing.
#[test]
fn a_disabled_ui_pans_by_touch_as_an_enabled_one() {
    type Area = fn(&mut Ui) -> [Vec2; 2];
    let centre = pos2(150.0, 200.0);
    let scenes: [(&str, Pos2, Area); 3] = [
        ("tall", centre, |ui| [tall(ui), Vec2::ZERO]),
        // Content 100 wide shrinks the area from the 200 px room to
        // x 100..200; the finger at x 250 is beside it.
        ("beside a shrunk area", pos2(250.0, 200.0), |ui| {
            let offset = ui
                .scope_builder(egui::UiBuilder::new().max_rect(AREA), |ui| {
                    ScrollArea::vertical()
                        .max_height(AREA.height())
                        .show(ui, block(vec2(100.0, 1000.0)))
                        .state
                        .clamped_offset()
                })
                .inner;
            [offset, Vec2::ZERO]
        }),
        ("nested", centre, |ui| {
            let mut inner = Vec2::ZERO;
            let outer = place(ui, ScrollArea::vertical(), |ui| {
                inner = ScrollArea::vertical()
                    .id_salt("inner")
                    .max_height(150.0)
                    .auto_shrink(false)
                    .show(ui, block(vec2(180.0, 600.0)))
                    .state
                    .clamped_offset();
                ui.allocate_space(vec2(200.0, 600.0));
            });
            [outer, inner]
        }),
    ];
    for (name, from, area) in scenes {
        let enabled = touch_pan(false, from, area);
        let disabled = touch_pan(true, from, area);
        let sum = |o: &[Vec2; 2]| o[0].y + o[1].y;
        if name == "beside a shrunk area" {
            assert!(
                enabled.iter().all(|o| sum(o) == 0.0),
                "{name}: the enabled area does not pan"
            );
        } else {
            assert!(
                enabled.iter().any(|o| sum(o) > 0.0),
                "{name}: the enabled area pans"
            );
            // Pass 7 is the release; the fling carries the area on.
            assert!(
                sum(enabled.last().unwrap()) > sum(&enabled[7]) + 1.0,
                "{name}: the release leaves a fling"
            );
        }
        if name == "nested" {
            assert!(
                enabled.iter().all(|o| o[0].y == 0.0),
                "{name}: the inner area takes the pan"
            );
        }
        for (pass, (e, d)) in enabled.iter().zip(&disabled).enumerate() {
            for k in 0..2 {
                assert!(
                    (e[k].y - d[k].y).abs() < 0.01,
                    "{name} pass {pass} area {k}: enabled {} disabled {}",
                    e[k].y,
                    d[k].y
                );
            }
        }
    }
}

/// Chrome's scroller stops at its edge: a touch pan past the start, and
/// the fling it leaves, never move the content past it (egui 0.37's
/// rubber band would), in an enabled and in a disabled `Ui`.
#[test]
fn a_touch_pan_past_the_edge_does_not_stretch() {
    for disabled in [false, true] {
        let mut sim = Sim::vega();
        let mut area = move |ui: &mut Ui| {
            if disabled {
                ui.disable();
            }
            ui.scope_builder(egui::UiBuilder::new().max_rect(AREA), |ui| {
                ScrollArea::vertical()
                    .max_height(AREA.height())
                    .auto_shrink(false)
                    .show(ui, block(vec2(200.0, 1000.0)))
                    .state
                    .unclamped_offset()
            })
            .inner
        };
        let from = pos2(150.0, 100.0);
        let touch = Event::Touch {
            device_id: egui::TouchDeviceId(1),
            id: egui::TouchId(0),
            phase: egui::TouchPhase::Start,
            pos: from,
            force: None,
        };
        let mut offsets = vec![sim.pass(vec![touch], &mut area).0];
        offsets.push(sim.pass(moved(from), &mut area).0);
        offsets.push(sim.pass(common::pointer_down(from), &mut area).0);
        // Down: the content would follow the finger past its start.
        for e in common::pointer_path(from, from + vec2(0.0, 80.0), 4) {
            offsets.push(sim.pass(vec![e], &mut area).0);
        }
        offsets.push(
            sim.pass(common::pointer_up(from + vec2(0.0, 80.0)), &mut area)
                .0,
        );
        for _ in 0..10 {
            offsets.push(sim.pass(vec![], &mut area).0);
        }
        assert!(
            offsets.iter().all(|o| o.y == 0.0),
            "disabled {disabled}: {offsets:?}"
        );
    }
}

/// The number of accessibility nodes a pass reported.
fn node_count(out: &egui::FullOutput) -> usize {
    out.platform_output
        .accesskit_update
        .as_ref()
        .map_or(0, |u| u.nodes.len())
}

/// A disabled area that does not overflow reports the same nodes as an
/// enabled one: nothing in it takes input.
#[test]
fn a_disabled_area_without_overflow_adds_no_node() {
    let count = |disabled: bool| {
        let mut sim = Sim::vega();
        sim.ctx.enable_accesskit();
        let mut add = move |ui: &mut Ui| {
            if disabled {
                ui.disable();
            }
            place(ui, ScrollArea::vertical(), block(vec2(200.0, 150.0)))
        };
        sim.pass(vec![], &mut add);
        node_count(&sim.pass(vec![], &mut add).1)
    };
    assert_eq!(count(true), count(false));
}

/// The browser scrolls an element that takes focus into view, centred.
#[test]
fn a_focused_widget_is_scrolled_into_view() {
    let mut sim = Sim::vega();
    let mut rects = [Rect::NOTHING; 10];
    let mut ids = [egui::Id::NULL; 10];
    let mut area = |ui: &mut Ui| {
        place(ui, ScrollArea::vertical(), |ui| {
            for i in 0..10 {
                let r = ui.add_sized(vec2(100.0, 50.0), egui::Button::new(format!("b{i}")));
                rects[i] = r.rect;
                ids[i] = r.id;
            }
        })
    };
    sim.pass(vec![], &mut area);
    // Tab to b0..b5; b5 starts below the 200 px viewport.
    for _ in 0..6 {
        sim.pass(tab(), &mut area);
        sim.pass(vec![], &mut area);
    }
    let (offset, _) = sim.pass(vec![], &mut area);
    drop(area);
    assert_eq!(sim.ctx.memory(|m| m.focused()), Some(ids[5]));
    assert!(offset.y > 0.0, "scrolled");
    assert!(AREA.contains_rect(rects[5]), "b5 at {:?}", rects[5]);
    assert!(
        (rects[5].center().y - AREA.center().y).abs() < 1.0,
        "centred: {:?}",
        rects[5]
    );
}

/// A widget that is already in full view when it takes focus stays put.
#[test]
fn a_visible_focused_widget_does_not_scroll() {
    let mut sim = Sim::vega();
    let mut area = |ui: &mut Ui| {
        place(ui, ScrollArea::vertical(), |ui| {
            for i in 0..10 {
                ui.add_sized(vec2(100.0, 50.0), egui::Button::new(format!("b{i}")));
            }
        })
    };
    sim.pass(vec![], &mut area);
    // b0..b2: b2 sits below the centre, in full view.
    for _ in 0..3 {
        sim.pass(tab(), &mut area);
        sim.pass(vec![], &mut area);
    }
    let (offset, _) = sim.pass(vec![], &mut area);
    assert_eq!(offset.y, 0.0);
}

#[test]
fn the_default_id_is_egui_s() {
    let mut sim = Sim::vega();
    let (ids, _) = sim.pass(vec![], |ui| {
        let ours = ScrollArea::vertical().id_salt("list").show(ui, |_| ()).id;
        let ours_default = ScrollArea::vertical().show(ui, |_| ()).id;
        (
            ours,
            ui.make_persistent_id(egui::IdSalt::new("list")),
            ours_default,
            ui.make_persistent_id(egui::IdSalt::new("scroll_area")),
        )
    });
    assert_eq!(ids.0, ids.1);
    assert_eq!(ids.2, ids.3);
}

/// `show_rows` hands the content egui's row range for the offset.
#[test]
fn show_rows_passes_egui_s_range() {
    let mut ranges = Vec::new();
    for ours in [true, false] {
        let mut sim = Sim::vega();
        let mut range = 0..0;
        let mut rows = |ui: &mut Ui| {
            ui.scope_builder(egui::UiBuilder::new().max_rect(AREA), |ui| {
                let add = |ui: &mut Ui, rows: std::ops::Range<usize>| {
                    for _ in rows.clone() {
                        ui.allocate_space(vec2(100.0, 20.0));
                    }
                    range = rows;
                };
                if ours {
                    ScrollArea::vertical()
                        .max_height(200.0)
                        .show_rows(ui, 20.0, 100, add);
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show_rows(ui, 20.0, 100, add);
                }
            });
        };
        sim.pass(moved(INSIDE), &mut rows);
        sim.pass(vec![wheel(-300.0)], &mut rows);
        sim.pass(vec![], &mut rows);
        drop(rows);
        assert!(range.start > 0, "scrolled: {range:?}");
        ranges.push(range);
    }
    assert_eq!(ranges[0], ranges[1]);
}

/// `overflow: hidden` clips at the viewport's edge: scrolled content
/// never paints past it, over the root's border (egui 0.34's
/// `clip_rect_margin` let it paint 3 px past).
#[test]
fn content_is_clipped_at_the_viewport() {
    let mut sim = Sim::vega();
    let fill = egui::Color32::from_rgb(1, 2, 3);
    let area = |ui: &mut Ui| {
        place(ui, ScrollArea::vertical(), |ui| {
            let (_, r) = ui.allocate_space(vec2(200.0, 1000.0));
            ui.painter().rect_filled(r, 0.0, fill);
        })
    };
    let (_, out) = sim.pass(vec![], area);
    let clips: Vec<Rect> = out
        .shapes
        .iter()
        .filter(|c| matches!(&c.shape, egui::Shape::Rect(r) if r.fill == fill))
        .map(|c| c.clip_rect)
        .collect();
    assert_eq!(clips, vec![AREA]);
}

/// Upstream paints nothing over a scrolled edge; egui fades it.
#[test]
fn no_fade_over_a_scrolled_edge() {
    let mut sim = Sim::vega();
    sim.pass(moved(INSIDE), tall);
    sim.pass(vec![wheel(-300.0)], tall);
    sim.pass(moved(OUTSIDE), tall);
    sim.time += 1.0;
    let (offset, out) = sim.pass(vec![], tall);
    assert!(offset.y > 0.0 && offset.y < MAX_SCROLL, "{offset:?}");
    assert!(
        common::mesh_shapes(&out).is_empty(),
        "a fade mesh was painted"
    );
}

// Snapshots: the docs page's two demos and an area that scrolls both
// ways, scrolled and hovered so the bars show.

/// The pointer over the area, then a wheel step of `delta`.
fn hover_and_scroll(at: Pos2, delta: Vec2) -> impl FnMut(&mut egui_kittest::Harness<'_>) {
    move |h| {
        h.input_mut().events.push(Event::PointerMoved(at));
        h.run();
        h.input_mut().events.push(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        });
        h.run();
    }
}

/// The root's `rounded-md border`, as the demos' caller draws it.
fn root(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let t = Theme::current(ui.ctx());
    egui::Frame::new()
        .stroke(egui::Stroke::new(
            1.0_f32,
            t.paint(Paint::Role(Role::Border, 1.0)),
        ))
        .corner_radius(t.radius_md())
        .show(ui, add);
}

fn muted(ui: &Ui) -> egui::Color32 {
    Theme::current(ui.ctx()).paint(Paint::Role(Role::MutedForeground, 1.0))
}

/// `scroll-area-demo`: 50 tags in an `h-72 w-48` root, `p-4` inside.
fn tags_demo(ui: &mut Ui) {
    use shadcn_egui::components::separator::separator;
    root(ui, |ui| {
        ui.set_width(190.0);
        ScrollArea::vertical()
            .max_height(286.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Frame::new().inner_margin(16).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(egui::RichText::new("Tags").strong());
                    ui.add_space(16.0);
                    for i in (1..=50).rev() {
                        ui.label(format!("v1.2.0-beta.{i}"));
                        ui.add_space(8.0);
                        separator(ui);
                        ui.add_space(8.0);
                    }
                });
            });
    });
}

/// `scroll-area-horizontal`: three works in a `w-96` root, `p-4` and
/// `space-x-4` inside; `muted` boxes of the images' 300 × 400 stand in
/// for the photos (no image loader).
fn works_demo(ui: &mut Ui) {
    let artists = ["Ornella Binni", "Tom Byrom", "Vladimir Malyavko"];
    root(ui, |ui| {
        ui.set_width(382.0);
        ScrollArea::horizontal()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Frame::new().inner_margin(16).show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 16.0;
                        for artist in artists {
                            ui.vertical(|ui| {
                                let t = Theme::current(ui.ctx());
                                let (_, r) = ui.allocate_space(vec2(300.0, 400.0));
                                ui.painter().rect_filled(
                                    r,
                                    t.radius_md(),
                                    t.paint(Paint::Role(Role::Muted, 1.0)),
                                );
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    ui.label(
                                        egui::RichText::new("Photo by ")
                                            .size(12.0)
                                            .color(muted(ui)),
                                    );
                                    ui.label(egui::RichText::new(artist).size(12.0).strong());
                                });
                            });
                        }
                    });
                });
            });
    });
}

/// An area that scrolls both ways over a grid of cells.
fn grid_demo(ui: &mut Ui) {
    root(ui, |ui| {
        ui.set_width(278.0);
        ScrollArea::both()
            .max_height(198.0)
            .auto_shrink(false)
            .show(ui, |ui| {
                egui::Grid::new("cells")
                    .spacing(vec2(16.0, 8.0))
                    .show(ui, |ui| {
                        for row in 0..20 {
                            for col in 0..8 {
                                ui.label(format!("Cell {row}.{col}"));
                            }
                            ui.end_row();
                        }
                    });
            });
    });
}

/// Two styles, light and dark: vega's round thumb and lyra's square one.
fn two_styles() -> Vec<common::Themed> {
    common::snapshot_themes()
        .into_iter()
        .filter(|(name, ..)| name.starts_with("vega") || name.starts_with("lyra"))
        .collect()
}

#[test]
fn scroll_area_snapshot() {
    common::snapshot_all_with(
        "scroll_area",
        vec2(240.0, 340.0),
        tags_demo,
        hover_and_scroll(pos2(100.0, 150.0), vec2(0.0, -120.0)),
    );
}

#[test]
fn scroll_area_horizontal_snapshot() {
    common::snapshot_with_themes(
        "scroll_area_horizontal",
        vec2(430.0, 540.0),
        two_styles(),
        works_demo,
        hover_and_scroll(pos2(200.0, 200.0), vec2(-200.0, 0.0)),
    );
}

#[test]
fn scroll_area_both_snapshot() {
    common::snapshot_with_themes(
        "scroll_area_both",
        vec2(330.0, 250.0),
        two_styles(),
        grid_demo,
        hover_and_scroll(pos2(150.0, 120.0), vec2(-60.0, -60.0)),
    );
}
