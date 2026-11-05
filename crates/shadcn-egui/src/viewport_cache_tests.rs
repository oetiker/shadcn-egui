//! The crate's caches with two viewports: egui counts passes and frames
//! per viewport, and each viewport keeps what it drew in its last pass,
//! however the passes of the two alternate.

use crate::components::shared::{self, RecipeBox};
use crate::components::text::{self, TextLook};
use crate::style::{Paint, Recipe, Role, State};
use crate::Theme;
use egui::{Rect, TextWrapMode, Ui, ViewportId};

thread_local! {
    /// The scale of the immediate windows the test renderer runs.
    static WINDOW_SCALE: std::cell::Cell<Option<f32>> = const { std::cell::Cell::new(None) };
    /// Font atlases egui has sent whole on this thread.
    static ATLAS_BUILDS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Count the font atlas of a pass's output when egui sends it whole: it
/// built a new one.
fn count_atlas(out: &egui::FullOutput) {
    let whole = out
        .textures_delta
        .set
        .iter()
        .filter(|(id, deltas)| {
            **id == egui::TextureId::default() && deltas.iter().any(|d| d.pos.is_none())
        })
        .count() as u64;
    ATLAS_BUILDS.with(|b| b.set(b.get() + whole));
}

fn atlas_builds() -> u64 {
    ATLAS_BUILDS.with(std::cell::Cell::get)
}

/// A context whose immediate viewports are real windows, each pass run
/// with `run_ui` at [`WINDOW_SCALE`].
fn context() -> egui::Context {
    let ctx = egui::Context::default();
    ctx.set_embed_viewports(false);
    egui::Context::set_immediate_viewport_renderer(|ctx, mut viewport| {
        let id = viewport.ids.this;
        let info = egui::ViewportInfo {
            native_pixels_per_point: WINDOW_SCALE.with(std::cell::Cell::get),
            ..Default::default()
        };
        let input = egui::RawInput {
            viewport_id: id,
            viewports: [(id, info)].into_iter().collect(),
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| (viewport.viewport_ui_cb)(ui));
        count_atlas(&out);
        out.drop_without_applying_deltas();
    });
    ctx
}

fn window(n: u32) -> ViewportId {
    ViewportId::from_hash_of(("cache window", n))
}

/// `frames` frames of the root drawing `root` and window 0 drawing
/// `win`, after three frames of the root alone, so the window's pass
/// and frame numbers are not the root's. Returns `count` after the
/// first two-window frame and after the last.
fn alternate<T: PartialEq + std::fmt::Debug>(
    root: &mut dyn FnMut(&mut Ui, u64),
    win: &mut dyn FnMut(&mut Ui, u64),
    frames: u64,
    count: impl Fn(&egui::Context) -> T,
) -> (T, T) {
    let ctx = context();
    for n in 0..3 {
        ctx.run_ui(egui::RawInput::default(), |ui| {
            Theme::vega().apply(ui.ctx());
            root(ui, n);
        })
        .drop_without_applying_deltas();
    }
    let mut first = None;
    for n in 3..3 + frames {
        ctx.run_ui(egui::RawInput::default(), |ui| {
            Theme::vega().apply(ui.ctx());
            root(ui, n);
            ui.ctx().show_viewport_immediate(
                window(0),
                egui::ViewportBuilder::default(),
                |ui, _| {
                    Theme::vega().apply(ui.ctx());
                    win(ui, n);
                },
            );
        })
        .drop_without_applying_deltas();
        if first.is_none() {
            first = Some(count(&ctx));
        }
    }
    (first.expect("a two-window frame"), count(&ctx))
}

fn label(ui: &mut Ui, text: &str) {
    let look = TextLook::resolve(ui, &[&Recipe::default()], State::NONE);
    let _ = look.label(ui, text, TextWrapMode::Wrap);
}

/// The same text in one window is laid out and placed once more in the
/// second frame (the theme's fonts, installed in the first, make a new
/// atlas), then not again.
#[test]
fn text_in_one_window_is_laid_out_once() {
    let ctx = context();
    let mut seen = Vec::new();
    for _ in 0..5 {
        ctx.run_ui(egui::RawInput::default(), |ui| {
            Theme::vega().apply(ui.ctx());
            label(ui, "root text");
        })
        .drop_without_applying_deltas();
        seen.push(text::text_builds(&ctx));
    }
    assert_eq!(seen, [(1, 1), (2, 2), (2, 2), (2, 2), (2, 2)]);
}

/// Still text in a root and a window is laid out and placed in the
/// first frame only, though the window's frame numbers are not the
/// root's.
#[test]
fn still_text_in_two_windows_is_laid_out_once() {
    let (first, last) = alternate(
        &mut |ui, _| label(ui, "root text"),
        &mut |ui, _| label(ui, "window text"),
        5,
        text::text_builds,
    );
    assert_eq!(first, last, "(laid out, placed) after frame 1 and 5");
}

/// Still text in a root at 100 % and a window at 150 % (two monitors) is
/// laid out and placed in the first frame only: egui keys its galleys by
/// scale, and the window's pass keeps the root's atlas probe too.
#[test]
fn still_text_in_a_window_at_another_scale_is_laid_out_once() {
    WINDOW_SCALE.with(|s| s.set(Some(1.5)));
    let (first, last) = alternate(
        &mut |ui, _| label(ui, "root text"),
        &mut |ui, _| label(ui, "window text"),
        5,
        text::text_builds,
    );
    WINDOW_SCALE.with(|s| s.set(None));
    assert_eq!(first, last, "(laid out, placed) after frame 1 and 5");
}

/// Text that changes in every frame in a root and a window keeps the
/// measures of its face: they are made once.
#[test]
fn changing_text_in_two_windows_keeps_its_measures() {
    let (first, last) = alternate(
        &mut |ui, n| label(ui, &format!("root {n}")),
        &mut |ui, n| label(ui, &format!("window {n}")),
        5,
        text::measure_builds,
    );
    assert_eq!(first, last, "measures made after frame 1 and 5");
}

/// A window that shows only egui's own text keeps the root's text in
/// the cache.
#[test]
fn a_window_without_crate_text_keeps_the_root_text() {
    let (first, last) = alternate(
        &mut |ui, _| label(ui, "root text"),
        &mut |ui, _| {
            ui.label("plain egui text");
        },
        5,
        text::text_builds,
    );
    assert_eq!(first, last, "(laid out, placed) after frame 1 and 5");
}

/// The ink egui's atlas has in the current pass.
fn ink(ui: &Ui) -> egui::epaint::FontColorTransferFunction {
    ui.ctx().fonts(|f| f.options().color_transfer_function)
}

/// `frames` frames of a root in the dark mode, and from the fourth on a
/// window in the light mode while `open` holds for its frame number.
/// Returns, after each frame, (laid out, placed) text, the atlases built,
/// and the inks of the root's and the window's pass.
type ModeFrame = (
    (u64, u64),
    u64,
    egui::epaint::FontColorTransferFunction,
    Option<egui::epaint::FontColorTransferFunction>,
);

fn dark_root_light_window(
    frames: u64,
    open: impl Fn(u64) -> bool,
) -> (egui::Context, Vec<ModeFrame>) {
    let ctx = context();
    let mut seen = Vec::new();
    for n in 0..frames {
        let mut inks = None;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.ctx().set_theme(egui::Theme::Dark);
            Theme::vega().apply(ui.ctx());
            let root = ink(ui);
            label(ui, "root text");
            let mut win = None;
            if n >= 3 && open(n) {
                ui.ctx().show_viewport_immediate(
                    window(0),
                    egui::ViewportBuilder::default(),
                    |ui, _| {
                        ui.ctx().set_theme(egui::Theme::Light);
                        Theme::vega().apply(ui.ctx());
                        win = Some(ink(ui));
                        label(ui, "window text");
                    },
                );
            }
            inks = Some((root, win));
        });
        out.textures_delta.clear();
        count_atlas(&out);
        let (root, win) = inks.expect("the root's pass ran");
        seen.push((text::text_builds(&ctx), atlas_builds(), root, win));
    }
    (ctx, seen)
}

/// A root in the dark mode and a window in the light one keep egui's
/// font atlas and their text: the window draws in the root's ink, else
/// each pass would start with the other mode's and rebuild the atlas.
#[test]
fn a_window_in_another_mode_keeps_the_atlas() {
    let (_, seen) = dark_root_light_window(8, |_| true);
    let counts = |f: &ModeFrame| (f.0, f.1);
    assert_eq!(
        counts(&seen[4]),
        counts(&seen[7]),
        "((laid out, placed), atlases) per frame: {:?}",
        seen.iter().map(counts).collect::<Vec<_>>()
    );
}

/// The root keeps its own mode's ink while a window runs in the other
/// mode; the window draws in the root's. Once the window has closed, each
/// mode has its own ink again.
#[test]
fn the_root_keeps_its_ink_beside_a_window_in_another_mode() {
    use egui::epaint::FontColorTransferFunction::{Gamma, Off};
    let (ctx, seen) = dark_root_light_window(10, |n| n < 7);
    // The first pass starts before the first apply, in egui's ink.
    let roots: Vec<_> = seen.iter().skip(1).map(|f| f.2).collect();
    assert_eq!(roots, [Gamma(0.81); 9]);
    let windows: Vec<_> = seen.iter().filter_map(|f| f.3).collect();
    assert_eq!(windows, [Gamma(0.81); 4]);
    let light = ctx
        .style_of(egui::Theme::Light)
        .visuals
        .text_options
        .color_transfer_function;
    assert_eq!(light, Off);
}

/// A faded box with sides of unequal width, as the button group's
/// `NOT_FIRST` rule makes it: no left border.
fn one_side_off(ctx: &egui::Context) -> RecipeBox {
    let mut r = Recipe::default();
    r.border_width.base = Some(1.0);
    r.border_left_width.base = Some(0.0);
    r.bg.base = Some(Paint::Role(Role::Background, 1.0));
    RecipeBox::resolve(
        ctx,
        &Theme::vega().in_mode(egui::Theme::Light),
        &[&r],
        State::NONE,
    )
}

fn faded_box(ui: &mut Ui, width: f32) {
    let b = one_side_off(ui.ctx());
    let rect = Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(width, 20.0));
    let _ = b.shapes_at(ui.ctx(), rect, 0.5);
}

/// A still faded box with unequal sides in a root and another in a
/// window are cut in the first frame only.
#[test]
fn still_faded_boxes_in_two_windows_are_cut_once() {
    let (first, last) = alternate(
        &mut |ui, _| faded_box(ui, 40.0),
        &mut |ui, _| faded_box(ui, 60.0),
        5,
        shared::cut_builds,
    );
    assert_eq!(first, last, "meshes cut after frame 1 and 5");
}

/// Viewports with a state in each cache: cut meshes, laid-out text,
/// placed text, measures.
fn cache_viewports(ctx: &egui::Context) -> [usize; 4] {
    let (laid, placed) = text::text_viewports(ctx);
    [
        shared::cut_viewports(ctx),
        laid,
        placed,
        text::measure_viewports(ctx),
    ]
}

/// Fifty windows, each open for one frame with text and a faded box:
/// the caches drop the state of each closed window and hold no more
/// than the root and the windows of the last two frames.
#[test]
fn closed_windows_leave_no_cache() {
    let ctx = context();
    let mut most = [0; 4];
    for n in 0..50 {
        ctx.run_ui(egui::RawInput::default(), |ui| {
            Theme::vega().apply(ui.ctx());
            label(ui, "root text");
            faded_box(ui, 40.0);
            ui.ctx().show_viewport_immediate(
                window(n),
                egui::ViewportBuilder::default(),
                |ui, _| {
                    Theme::vega().apply(ui.ctx());
                    label(ui, "window text");
                    faded_box(ui, 60.0);
                },
            );
        })
        .drop_without_applying_deltas();
        let now = cache_viewports(&ctx);
        for (m, c) in most.iter_mut().zip(now) {
            *m = (*m).max(c);
        }
    }
    assert!(
        most.iter().all(|&m| m <= 3),
        "most states per cache: {most:?}"
    );
}

/// A transition of the same widget id in a root and a window: each
/// viewport shows its own target from its first pass, though the two
/// count their passes apart.
#[test]
fn a_transition_in_two_windows_keeps_each_windows_value() {
    use crate::components::motion::transition;
    use crate::style::{Prop, Transition};
    let moving = Recipe {
        transition: Prop::fixed(Transition::ALL),
        ..Recipe::default()
    };
    let (root, win) = (std::cell::Cell::new(0.0), std::cell::Cell::new(0.0));
    let value = |ui: &Ui, target: f32| {
        let id = egui::Id::unique("shared transition");
        let s = State::NONE;
        transition(ui.ctx(), id, target, &[&moving], Transition::OTHER, s)
    };
    alternate(
        &mut |ui, _| root.set(value(ui, 10.0)),
        &mut |ui, _| win.set(value(ui, 20.0)),
        5,
        |_| (),
    );
    assert_eq!((root.get(), win.get()), (10.0, 20.0));
}

/// An overlay open in a root and a window under the same id plays its
/// enter animation once in each, though the two count their passes apart.
#[test]
fn an_overlay_in_two_windows_finishes_its_enter_animation() {
    use crate::components::motion::Enter;
    let theme = Theme::vega();
    let dialog = [&*theme.style.dialog_content];
    let (root, win) = (std::cell::Cell::new(true), std::cell::Cell::new(true));
    let running = |ui: &Ui| {
        let id = egui::Id::unique("shared overlay");
        let enter = Enter::at(ui.ctx(), id, &dialog, State::NONE);
        Enter::mark(ui, id);
        enter.running()
    };
    // Each pass advances a viewport's clock by egui's default 1/60 s: the
    // 100 ms animation has run after six of them.
    alternate(
        &mut |ui, _| root.set(running(ui)),
        &mut |ui, _| win.set(running(ui)),
        30,
        |_| (),
    );
    assert_eq!((root.get(), win.get()), (false, false));
}
