//! Shared test helpers. Each integration test file that needs them declares
//! `mod common;`.
#![allow(dead_code)]

pub mod logcap;
mod strict;

pub use strict::*;

use egui_kittest::{Harness, SnapshotResults};
use shadcn_egui::style::Style;
use shadcn_egui::{Palette, Theme};

/// `out` without its texture uploads. egui 0.37 debug-asserts that a
/// dropped `FullOutput` has no unapplied texture delta; a test that only
/// reads shapes or AccessKit output has no renderer to apply it to.
pub fn no_textures(mut out: egui::FullOutput) -> egui::FullOutput {
    out.textures_delta.clear();
    out
}

/// A pass's output that a helper hands back with its texture uploads (a
/// test may read them), and that discards them when the test drops it,
/// for the reason given at [`no_textures`].
pub struct Output(pub egui::FullOutput);

impl std::ops::Deref for Output {
    type Target = egui::FullOutput;
    fn deref(&self) -> &egui::FullOutput {
        &self.0
    }
}

impl std::ops::DerefMut for Output {
    fn deref_mut(&mut self) -> &mut egui::FullOutput {
        &mut self.0
    }
}

impl Output {
    /// The output itself, without its texture uploads.
    pub fn into_inner(mut self) -> egui::FullOutput {
        no_textures(std::mem::take(&mut self.0))
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        self.0.textures_delta.clear();
    }
}

/// A test theme: its name suffix, the theme, and the mode it renders in.
/// The mode travels beside the theme: an app picks it with
/// `ctx.set_theme(..)`, and the helpers below do the same.
pub type Themed = (&'static str, Theme, egui::Theme);

pub fn themes() -> Vec<Themed> {
    vec![
        (
            "vega_dark",
            Theme::new(Style::vega(), Palette::light(), Palette::dark(), 10.0),
            egui::Theme::Dark,
        ),
        (
            "vega_light",
            Theme::new(Style::vega(), Palette::light(), Palette::dark(), 10.0),
            egui::Theme::Light,
        ),
        (
            "nova_dark",
            Theme::new(Style::nova(), Palette::light(), Palette::dark(), 10.0),
            egui::Theme::Dark,
        ),
        (
            "nova_light",
            Theme::new(Style::nova(), Palette::light(), Palette::dark(), 10.0),
            egui::Theme::Light,
        ),
    ]
}

/// The seven exported styles, dark and light: what every component snapshot
/// covers. Measurement tests use [`themes`] (vega and nova) instead.
pub fn snapshot_themes() -> Vec<Themed> {
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
                (
                    *dark,
                    Theme::new(make(), Palette::light(), Palette::dark(), 10.0),
                    egui::Theme::Dark,
                ),
                (
                    *light,
                    Theme::new(make(), Palette::light(), Palette::dark(), 10.0),
                    egui::Theme::Light,
                ),
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

/// `ui_fn` on a page of `theme`'s background colour in `mode`, after one run.
pub fn page_harness<'a>(
    size: egui::Vec2,
    theme: Theme,
    mode: egui::Theme,
    mut ui_fn: impl FnMut(&mut egui::Ui) + 'a,
) -> Harness<'a> {
    let bg = theme.palette_of(mode).background;
    // Snapshots show bare controls, as upstream's style demos do, most
    // without an accessible name; tests/accessibility.rs and the
    // interaction tests check the names.
    let mut h = Harness::builder()
        .with_accessibility_check(false)
        .with_size(size)
        .with_theme(mode)
        .build_ui(move |ui| {
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
    themes: Vec<Themed>,
    mut ui_fn: impl FnMut(&mut egui::Ui),
    mut prepare: impl FnMut(&mut Harness<'_>),
) {
    // One result set for all harnesses, so a failing style does not abort the rest.
    let mut results = SnapshotResults::new();
    for (suffix, theme, mode) in themes {
        let mut h = page_harness(size, theme, mode, &mut ui_fn);
        prepare(&mut h);
        results.add(try_snapshot(&mut h, format!("{name}_{suffix}")));
    }
}

/// The rect of the widget `add` places, under `theme` in `mode`, and the output of
/// the frame that painted it. Runs two frames: the fonts `Theme::apply`
/// installs reach the atlas only on the second, and focus requested in the
/// first frame shows in the second. Transitions jump (`animation_time`
/// 0, as in the snapshots), so the frame shows the settled state.
pub fn measure_output(
    theme: &Theme,
    mode: egui::Theme,
    add: impl FnMut(&mut egui::Ui) -> egui::Response,
) -> (egui::Rect, Output) {
    let (rect, out, _) = measure_frame(theme, mode, add);
    (rect, out)
}

/// [`measure_output`] and the context it ran on. The output's
/// `textures_delta` holds what both frames uploaded, so it has the image
/// of every texture the frame paints with.
pub fn measure_frame(
    theme: &Theme,
    mode: egui::Theme,
    mut add: impl FnMut(&mut egui::Ui) -> egui::Response,
) -> (egui::Rect, Output, egui::Context) {
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    ctx.set_theme(mode);
    ctx.all_styles_mut(|s| s.animation_time = 0.0);
    let mut rect = egui::Rect::NOTHING;
    let first = ctx.run_ui(egui::RawInput::default(), |ui| rect = add(ui).rect);
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| rect = add(ui).rect);
    let mut textures = first.textures_delta;
    textures.append(std::mem::take(&mut out.textures_delta));
    out.textures_delta = textures;
    (rect, Output(out), ctx)
}

/// The textures `ctx` holds for box shadows (the crate names them so).
pub fn shadow_textures(ctx: &egui::Context) -> Vec<egui::TextureId> {
    ctx.tex_manager()
        .read()
        .allocated()
        .filter(|(_, meta)| meta.name == "shadcn-egui box-shadow")
        .map(|(id, _)| *id)
        .collect()
}

/// Every whole texture image `out` uploads, by id.
pub fn texture_images(
    out: &egui::FullOutput,
) -> std::collections::HashMap<egui::TextureId, egui::ColorImage> {
    out.textures_delta
        .set
        .iter()
        .filter_map(|(id, deltas)| {
            // The last whole image of the pass is the texture.
            let d = deltas.iter().rev().find(|d| d.pos.is_none())?;
            let egui::ImageData::Color(image) = &d.image;
            Some((*id, (**image).clone()))
        })
        .collect()
}

/// The premultiplied alpha, 0..=255, that the textured `meshes` paint at
/// `p` (in points), sampled as a GPU samples a `NEAREST` texture: the
/// texel under `p` times the vertex colour, of the first mesh that covers
/// `p`; 0 where none does.
pub fn textured_alpha(
    meshes: &[egui::Mesh],
    images: &std::collections::HashMap<egui::TextureId, egui::ColorImage>,
    p: egui::Pos2,
) -> f32 {
    for m in meshes {
        let Some(image) = images.get(&m.texture_id) else {
            continue;
        };
        for t in m.indices.as_chunks::<3>().0 {
            let [a, b, c] = t.map(|i| m.vertices[i as usize]);
            let cross = |u: egui::Pos2, v: egui::Pos2, w: egui::Pos2| {
                (v - u).x * (w - u).y - (v - u).y * (w - u).x
            };
            let area = cross(a.pos, b.pos, c.pos);
            if area.abs() < 1e-9 {
                continue;
            }
            let w = [
                cross(b.pos, c.pos, p),
                cross(c.pos, a.pos, p),
                cross(a.pos, b.pos, p),
            ]
            .map(|x| x / area);
            if w.iter().any(|x| *x < 0.0) {
                continue;
            }
            let uv = a.uv.to_vec2() * w[0] + b.uv.to_vec2() * w[1] + c.uv.to_vec2() * w[2];
            let [iw, ih] = image.size;
            let x = ((uv.x * iw as f32).floor() as usize).min(iw - 1);
            let y = ((uv.y * ih as f32).floor() as usize).min(ih - 1);
            let texel = f32::from(image.pixels[y * iw + x].a());
            return texel * f32::from(a.color.a()) / 255.0;
        }
    }
    0.0
}

/// The rect of the widget `add` places, under `theme` in `mode` (see
/// [`measure_output`]).
pub fn measure(
    theme: &Theme,
    mode: egui::Theme,
    add: impl FnMut(&mut egui::Ui) -> egui::Response,
) -> egui::Rect {
    measure_output(theme, mode, add).0
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

/// Every mesh shape in `out` (a recipe box's shadow, a border of unequal
/// sides).
pub fn mesh_shapes(out: &egui::FullOutput) -> Vec<egui::Mesh> {
    fn walk(s: &egui::Shape, out: &mut Vec<egui::Mesh>) {
        match s {
            egui::Shape::Mesh(m) => out.push((**m).clone()),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut meshes = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut meshes));
    meshes
}

/// The colour a mesh paints in its middle: the vertex nearest the centre
/// of its bounds (a faded recipe box's fill, where the border is around).
pub fn mesh_colour(mesh: &egui::Mesh) -> egui::Color32 {
    let centre = mesh.calc_bounds().center();
    mesh.vertices
        .iter()
        .min_by(|a, b| {
            let d = |v: &egui::epaint::Vertex| (v.pos - centre).length_sq();
            d(a).total_cmp(&d(b))
        })
        .map_or(egui::Color32::TRANSPARENT, |v| v.color)
}

/// Every filled box a frame painted, in paint order, as (rect, colour): a
/// rect shape's fill, and a mesh's bounds and [`mesh_colour`] (a faded
/// recipe box paints its fill and border as one mesh).
pub fn box_fills(out: &egui::FullOutput) -> Vec<(egui::Rect, egui::Color32)> {
    fn walk(s: &egui::Shape, out: &mut Vec<(egui::Rect, egui::Color32)>) {
        match s {
            egui::Shape::Rect(r) => out.push((r.rect, r.fill)),
            egui::Shape::Mesh(m) => out.push((m.calc_bounds(), mesh_colour(m))),
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            _ => {}
        }
    }
    let mut fills = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut fills));
    fills
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

/// `theme` as the crate's components see it in a context set to `mode`
/// (`Theme::current`): its colours and `dark:` rules resolve in `mode`.
pub fn resolved(theme: &Theme, mode: egui::Theme) -> Theme {
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    ctx.set_theme(mode);
    Theme::current(&ctx)
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
    themes: Vec<Themed>,
    mut ui_fn: impl FnMut(&mut egui::Ui),
) {
    // One result set for all harnesses, as in `snapshot_with_themes`.
    let mut results = SnapshotResults::new();
    for width in [PHONE, DESKTOP] {
        for (suffix, theme, mode) in &themes {
            let mut h = page_harness(egui::vec2(width, height), theme.clone(), *mode, &mut ui_fn);
            results.add(try_snapshot(&mut h, format!("{name}_{width}_{suffix}")));
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
) -> Output {
    match app_loop {
        Loop::RunUi => Output(ctx.run_ui(window_input(events), ui_fn)),
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
) -> Output {
    ctx.begin_pass(window_input(events));
    let window = egui::Rect::from_min_size(egui::Pos2::ZERO, PASS_WINDOW);
    let builder = egui::UiBuilder::new().max_rect(window);
    ui_fn(&mut egui::Ui::new(
        ctx.clone(),
        egui::Id::unique("own_pass"),
        builder,
    ));
    Output(ctx.end_pass())
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

/// The upstream demos' own muted paragraph (`text-sm
/// text-muted-foreground`, no part of any component): caller text in a
/// recipe the test builds, wrapping like a paragraph.
pub fn muted_text(ui: &mut egui::Ui, text: &str) -> egui::Response {
    use shadcn_egui::components::text::TextLook;
    use shadcn_egui::style::{Paint, Prop, Recipe, Role, State, TextSize};
    let muted = Recipe {
        text_size: Prop::fixed(TextSize::Sm),
        text: Prop::fixed(Paint::Role(Role::MutedForeground, 1.0)),
        ..Recipe::default()
    };
    let st = Theme::current(ui.ctx()).state(ui.ctx(), State::NONE);
    TextLook::resolve(ui, &[&muted], st).label(ui, text, egui::TextWrapMode::Wrap)
}
