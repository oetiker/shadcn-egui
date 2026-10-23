//! Shared test helpers. Each integration test file that needs them declares
//! `mod common;`.
#![allow(dead_code)]

pub mod logcap;

use egui_kittest::{Harness, SnapshotResults};
use shadcn_egui::style::Style;
use shadcn_egui::{Palette, Theme};

pub fn themes() -> Vec<(&'static str, Theme)> {
    vec![
        (
            "vega_dark",
            Theme::new(Style::vega(), Palette::dark(), 10.0),
        ),
        (
            "vega_light",
            Theme::new(Style::vega(), Palette::light(), 10.0),
        ),
        (
            "nova_dark",
            Theme::new(Style::nova(), Palette::dark(), 10.0),
        ),
        (
            "nova_light",
            Theme::new(Style::nova(), Palette::light(), 10.0),
        ),
    ]
}

/// The seven exported styles, dark and light: what every component snapshot
/// covers. Measurement tests use [`themes`] (vega and nova) instead.
pub fn snapshot_themes() -> Vec<(&'static str, Theme)> {
    type Make = fn() -> Style;
    const STYLES: [(&str, &str, Make); 7] = [
        ("vega_dark", "vega_light", Style::vega),
        ("nova_dark", "nova_light", Style::nova),
        ("maia_dark", "maia_light", Style::maia),
        ("lyra_dark", "lyra_light", Style::lyra),
        ("mira_dark", "mira_light", Style::mira),
        ("luma_dark", "luma_light", Style::luma),
        ("rhea_dark", "rhea_light", Style::rhea),
    ];
    STYLES
        .iter()
        .flat_map(|(dark, light, make)| {
            [
                (*dark, Theme::new(make(), Palette::dark(), 10.0)),
                (*light, Theme::new(make(), Palette::light(), 10.0)),
            ]
        })
        .collect()
}

/// One snapshot per exported style x {dark, light}: `{name}_{style}_{mode}.png`.
pub fn snapshot_all(name: &str, size: egui::Vec2, ui_fn: impl FnMut(&mut egui::Ui)) {
    snapshot_all_with(name, size, ui_fn, |_| {});
}

/// Like [`snapshot_all`], with `prepare` run on each harness after the first
/// frame (hover a trigger, press a key) and before the snapshot.
pub fn snapshot_all_with(
    name: &str,
    size: egui::Vec2,
    ui_fn: impl FnMut(&mut egui::Ui),
    prepare: impl FnMut(&mut Harness<'_>),
) {
    snapshot_with_themes(name, size, snapshot_themes(), ui_fn, prepare);
}

/// `ui_fn` on a page of `theme`'s background colour, after one run.
pub fn page_harness<'a>(
    size: egui::Vec2,
    theme: Theme,
    mut ui_fn: impl FnMut(&mut egui::Ui) + 'a,
) -> Harness<'a> {
    let bg = theme.palette.background;
    let mut h = Harness::builder().with_size(size).build_ui(move |ui| {
        theme.apply(ui.ctx());
        // kittest wraps the ui in an 8px outer margin; paint the page
        // colour under all of it, so no band of another colour shows.
        let page = ui.ctx().viewport_rect();
        ui.ctx()
            .layer_painter(egui::LayerId::background())
            .rect_filled(page, 0.0, bg);
        egui::Frame::new().fill(bg).inner_margin(16).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui_fn(ui);
        });
    });
    h.run();
    h
}

/// One snapshot per `(suffix, theme)`: `{name}_{suffix}.png`.
pub fn snapshot_with_themes(
    name: &str,
    size: egui::Vec2,
    themes: Vec<(&'static str, Theme)>,
    mut ui_fn: impl FnMut(&mut egui::Ui),
    mut prepare: impl FnMut(&mut Harness<'_>),
) {
    // One result set for all harnesses, so a failing style does not abort the rest.
    let mut results = SnapshotResults::new();
    for (suffix, theme) in themes {
        let mut h = page_harness(size, theme, &mut ui_fn);
        prepare(&mut h);
        h.snapshot(format!("{name}_{suffix}"));
        results.extend(h.take_snapshot_results());
    }
}

/// The rect of the widget `add` places, under `theme`, and the output of
/// the frame that painted it. Runs two frames: the fonts `Theme::apply`
/// installs reach the atlas only on the second, and focus requested in the
/// first frame shows in the second.
pub fn measure_output(
    theme: &Theme,
    mut add: impl FnMut(&mut egui::Ui) -> egui::Response,
) -> (egui::Rect, egui::FullOutput) {
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    let mut rect = egui::Rect::NOTHING;
    let _ = ctx.run_ui(egui::RawInput::default(), |ui| rect = add(ui).rect);
    let out = ctx.run_ui(egui::RawInput::default(), |ui| rect = add(ui).rect);
    (rect, out)
}

/// The rect of the widget `add` places, under `theme` (see [`measure_output`]).
pub fn measure(theme: &Theme, add: impl FnMut(&mut egui::Ui) -> egui::Response) -> egui::Rect {
    measure_output(theme, add).0
}

/// Every rect shape a frame painted, in paint order.
pub fn rect_shapes(out: &egui::FullOutput) -> Vec<egui::epaint::RectShape> {
    fn walk(s: &egui::Shape, out: &mut Vec<egui::epaint::RectShape>) {
        match s {
            egui::Shape::Rect(r) => out.push(r.clone()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut rects = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut rects));
    rects
}

/// Every text shape a frame painted, in paint order.
pub fn text_shapes(out: &egui::FullOutput) -> Vec<egui::epaint::TextShape> {
    fn walk(s: &egui::Shape, out: &mut Vec<egui::epaint::TextShape>) {
        match s {
            egui::Shape::Text(t) => out.push(t.clone()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut texts = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut texts));
    texts
}

/// The colour the first text a frame painted reaches the screen in,
/// resolved as epaint's tessellator does: the shape's override colour, or
/// its fallback colour for glyphs laid out in `Color32::PLACEHOLDER`, then
/// faded by the shape's opacity (a painter's opacity fades the glyphs, not
/// the layout job).
pub fn text_colour(out: &egui::FullOutput) -> egui::Color32 {
    fn walk(s: &egui::Shape) -> Option<egui::Color32> {
        match s {
            egui::Shape::Text(t) => t
                .galley
                .rows
                .iter()
                .find_map(|r| r.visuals.mesh.vertices.first())
                .map(|v| {
                    let colour = match t.override_text_color {
                        Some(c) => c,
                        None if v.color == egui::Color32::PLACEHOLDER => t.fallback_color,
                        None => v.color,
                    };
                    if t.opacity_factor < 1.0 {
                        colour.gamma_multiply(t.opacity_factor)
                    } else {
                        colour
                    }
                }),
            egui::Shape::Vec(v) => v.iter().find_map(walk),
            _ => None,
        }
    }
    out.shapes
        .iter()
        .find_map(|c| walk(&c.shape))
        .expect("a text shape")
}

/// `theme` with `f` applied to its style: what an app does in its setup.
pub fn restyled(mut theme: Theme, f: impl FnOnce(&mut Style)) -> Theme {
    f(std::sync::Arc::make_mut(&mut theme.style));
    theme
}

/// The phone width of a two-width fixture (conformance 3.5).
pub const PHONE: f32 = 390.0;
/// The desktop width of a two-width fixture.
pub const DESKTOP: f32 = 1280.0;

/// One snapshot per `(suffix, theme)` at the phone and at the desktop
/// width, `height` high: `{name}_390_{suffix}.png` and
/// `{name}_1280_{suffix}.png`. For components and screens with
/// responsive classes (`sm:`, `md:`), which must work at both.
pub fn snapshot_widths(
    name: &str,
    height: f32,
    themes: Vec<(&'static str, Theme)>,
    mut ui_fn: impl FnMut(&mut egui::Ui),
) {
    // One result set for all harnesses, as in `snapshot_with_themes`.
    let mut results = SnapshotResults::new();
    for width in [PHONE, DESKTOP] {
        for (suffix, theme) in &themes {
            let mut h = page_harness(egui::vec2(width, height), theme.clone(), &mut ui_fn);
            h.snapshot(format!("{name}_{width}_{suffix}"));
            results.extend(h.take_snapshot_results());
        }
    }
}

/// The window of [`pass`] and [`own_pass`].
pub const PASS_WINDOW: egui::Vec2 = egui::vec2(600.0, 400.0);

/// The two ways an app drives egui (conformance 6): `Context::run_ui`,
/// which runs egui's pass hooks, and its own `begin_pass`/`end_pass` loop,
/// which runs none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Loop {
    RunUi,
    Own,
}

impl Loop {
    pub const BOTH: [Loop; 2] = [Loop::RunUi, Loop::Own];
}

/// One frame of `app_loop` on `ctx`: `events` are the frame's input, and
/// `ui_fn` draws into a root `Ui` filling a [`PASS_WINDOW`] window. Under
/// `run_ui`, a pass that calls `request_discard` is followed by another
/// pass in the same frame, which runs `ui_fn` again. Neither loop applies
/// a theme: a test that needs one applies it in `ui_fn`.
pub fn pass(
    app_loop: Loop,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    ui_fn: impl FnMut(&mut egui::Ui),
) -> egui::FullOutput {
    match app_loop {
        Loop::RunUi => ctx.run_ui(window_input(events), ui_fn),
        Loop::Own => own_pass(ctx, events, ui_fn),
    }
}

/// One pass of an app's own `begin_pass`/`end_pass` loop: egui runs no
/// plugin, so no pass hook either, and `ui_fn` draws into a root `Ui`
/// built by hand, as such an app does.
pub fn own_pass(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    mut ui_fn: impl FnMut(&mut egui::Ui),
) -> egui::FullOutput {
    ctx.begin_pass(window_input(events));
    let window = egui::Rect::from_min_size(egui::Pos2::ZERO, PASS_WINDOW);
    let builder = egui::UiBuilder::new().max_rect(window);
    ui_fn(&mut egui::Ui::new(
        ctx.clone(),
        egui::Id::new("own_pass"),
        builder,
    ));
    ctx.end_pass()
}

fn window_input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, PASS_WINDOW)),
        events,
        ..Default::default()
    }
}

/// `key` goes down (and stays down until [`key_up`]).
pub fn key_down(key: egui::Key) -> Vec<egui::Event> {
    vec![key_event(key, true, egui::Modifiers::NONE)]
}

/// `key` comes up.
pub fn key_up(key: egui::Key) -> Vec<egui::Event> {
    vec![key_event(key, false, egui::Modifiers::NONE)]
}

/// A press and the release of `key` in one pass, as a slow frame delivers
/// a quick tap.
pub fn key(key: egui::Key) -> Vec<egui::Event> {
    key_with(egui::Modifiers::NONE, key)
}

/// [`key`] with `modifiers` held.
pub fn key_with(modifiers: egui::Modifiers, key: egui::Key) -> Vec<egui::Event> {
    vec![
        key_event(key, true, modifiers),
        key_event(key, false, modifiers),
    ]
}

fn key_event(key: egui::Key, pressed: bool, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

/// Move the pointer to `pos` and press the primary button there.
pub fn pointer_down(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![egui::Event::PointerMoved(pos), pointer_button(pos, true)]
}

/// Release the primary button at `pos`.
pub fn pointer_up(pos: egui::Pos2) -> Vec<egui::Event> {
    vec![pointer_button(pos, false)]
}

/// A press and its release at `pos` in one pass, as a slow frame delivers
/// a quick click.
pub fn click(pos: egui::Pos2) -> Vec<egui::Event> {
    let mut events = pointer_down(pos);
    events.extend(pointer_up(pos));
    events
}

fn pointer_button(pos: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

/// The pointer moving from `from` to `to` in `steps` even moves, as a
/// mouse does: one event per step, `to` last. A pass that gets the path
/// sees every point a hover-tracking widget would.
pub fn pointer_path(from: egui::Pos2, to: egui::Pos2, steps: usize) -> Vec<egui::Event> {
    (1..=steps.max(1))
        .map(|i| egui::Event::PointerMoved(from.lerp(to, i as f32 / steps.max(1) as f32)))
        .collect()
}

/// The label of the one focused node of a kittest harness, if any.
pub fn focused_label<S>(h: &Harness<'_, S>) -> Option<String> {
    use egui_kittest::kittest::{NodeT, Queryable};
    h.query_all(egui_kittest::kittest::by())
        .find(|n| n.accesskit_node().is_focused())
        .and_then(|n| n.accesskit_node().label())
}

/// The bounds of the node labelled `label` in a pass's accesskit update
/// (the context needs `enable_accesskit`).
pub fn bounds_by_label(out: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    let update = out.platform_output.accesskit_update.as_ref()?;
    update
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some(label))
        .and_then(|(_, n)| n.bounds())
        .map(|b| {
            egui::Rect::from_min_max(
                egui::pos2(b.x0 as f32, b.y0 as f32),
                egui::pos2(b.x1 as f32, b.y1 as f32),
            )
        })
}
