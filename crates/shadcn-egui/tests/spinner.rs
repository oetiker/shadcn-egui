mod common;

use egui::accesskit::{Live, Role};
use egui::{Color32, TextureId, Vec2};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::color::oklch_to_srgb;
use shadcn_egui::components::badge::{Badge, BadgeVariant};
use shadcn_egui::components::button::{Button, ButtonSize, ButtonVariant};
use shadcn_egui::components::icon::{Glyph, IconView};
use shadcn_egui::components::spinner::{spinner, Spinner};
use shadcn_egui::{layout, Theme};

/// The textured meshes a frame painted: the icons.
fn icons(out: &egui::FullOutput) -> Vec<egui::Mesh> {
    common::mesh_shapes(out)
        .into_iter()
        .filter(|m| m.texture_id != TextureId::default())
        .collect()
}

/// One pass of `add` at `time` on `ctx`, with vega applied; the icons and
/// whether the pass asked for the next frame at once.
fn timed(
    ctx: &egui::Context,
    time: f64,
    mut add: impl FnMut(&mut egui::Ui),
) -> (Vec<egui::Mesh>, bool) {
    let out = ctx.run_ui(
        egui::RawInput {
            time: Some(time),
            ..Default::default()
        },
        |ui| {
            Theme::vega().apply(ui.ctx());
            add(ui);
        },
    );
    let repaint =
        out.viewport_output[&egui::ViewportId::ROOT].repaint_delay == std::time::Duration::ZERO;
    (icons(&out), repaint)
}

/// The angle of a painted icon quad: of its first edge against the x axis.
fn angle(mesh: &egui::Mesh) -> f32 {
    let d = mesh.vertices[1].pos - mesh.vertices[0].pos;
    d.y.atan2(d.x)
}

/// The icon textures the first frame of `add` at `time` uploads on a
/// fresh context (vega applied).
fn uploaded(time: f64, add: impl Fn(&mut egui::Ui)) -> Vec<egui::ColorImage> {
    let ctx = egui::Context::default();
    let out = ctx.run_ui(
        egui::RawInput {
            time: Some(time),
            ..Default::default()
        },
        |ui| {
            Theme::vega().apply(ui.ctx());
            add(ui);
        },
    );
    out.textures_delta
        .set
        .iter()
        .filter(|(id, _)| *id != TextureId::default())
        .map(|(_, d)| {
            let egui::ImageData::Color(img) = &d.image;
            (**img).clone()
        })
        .collect()
}

/// Asserts that the icon `add` paints at `time` (a quarter turn past a
/// whole one) is the icon `add` paints at 0, turned a quarter clockwise:
/// the turned texture's middle is the unturned one turned, within a level,
/// and its margin is empty.
#[track_caller]
fn assert_quarter_turn(add: impl Fn(&mut egui::Ui) + Copy, what: &str) {
    let start = uploaded(0.0, add);
    let quarter = uploaded(2.25, add);
    let (a, b) = (&start[0], quarter.last().expect("a turned texture"));
    let [w, h] = a.size;
    assert_eq!(w, h, "{what}");
    let m = (b.size[0] - w) / 2;
    for y in 0..b.size[1] {
        for x in 0..b.size[0] {
            let got = b.pixels[y * b.size[0] + x].a();
            let want = if (m..m + w).contains(&x) && (m..m + w).contains(&y) {
                // Clockwise: the pixel at (x, y) comes from (y, w - 1 - x).
                let (tx, ty) = (x - m, y - m);
                a.pixels[(w - 1 - tx) * w + ty].a()
            } else {
                0
            };
            assert!(
                got.abs_diff(want) <= 1,
                "{what}: ({x}, {y}) {got} against {want}"
            );
        }
    }
}

#[test]
fn a_spinner_is_the_styles_size() {
    for (name, theme, mode) in common::themes() {
        let rect = common::measure(&theme, mode, spinner);
        assert_eq!(rect.size(), Vec2::splat(16.0), "{name}");
    }
}

#[test]
fn size_sets_the_side() {
    let theme = Theme::vega();
    for side in [12.0, 24.0, 32.0] {
        let (rect, out) = common::measure_output(&theme, egui::Theme::Light, |ui| {
            ui.add(Spinner::new().size(side))
        });
        assert_eq!(rect.size(), Vec2::splat(side));
        assert_eq!(icons(&out)[0].calc_bounds().size(), Vec2::splat(side));
    }
}

/// The spinner draws the theme's `loader` slot: the same coverage as an
/// `IconView` of that icon.
#[test]
fn a_spinner_draws_the_themes_loader_icon() {
    // One pass on a fresh context: the pass that rasterises the texture.
    let texture = |add: &dyn Fn(&mut egui::Ui)| {
        let ctx = egui::Context::default();
        Theme::vega().apply(&ctx);
        // A still spinner: the loader's own texture, unturned.
        ctx.all_styles_mut(|s| s.animation_time = 0.0);
        let out = ctx.run_ui(egui::RawInput::default(), add);
        let images: Vec<_> = out
            .textures_delta
            .set
            .iter()
            .filter(|(id, _)| *id != TextureId::default())
            .map(|(_, d)| d.image.clone())
            .collect();
        assert_eq!(images.len(), 1, "one icon texture");
        images[0].clone()
    };
    let loader = Theme::vega().icons.loader;
    let spun = texture(&|ui| {
        spinner(ui);
    });
    let drawn = texture(&|ui| {
        ui.add(IconView::new(&loader));
    });
    let pixels = |d: &egui::ImageData| match d {
        egui::ImageData::Color(c) => c.pixels.clone(),
    };
    assert_eq!(pixels(&spun), pixels(&drawn));
}

/// `role="status"` with `aria-label="Loading"`, and the implicit
/// `aria-live="polite"` of a status.
#[test]
fn a_spinner_reports_a_status_named_loading() {
    let mut h = Harness::new_ui(|ui| {
        spinner(ui);
    });
    h.run();
    let node = h.get_by_role(Role::Status);
    let node = node.accesskit_node();
    assert_eq!(node.label().as_deref(), Some("Loading"));
    assert_eq!(node.data().live(), Some(Live::Polite));
}

#[test]
fn alt_text_renames_it() {
    let mut h = Harness::new_ui(|ui| {
        ui.add(Spinner::new().alt_text("Saving"));
    });
    h.run();
    h.get_by_role_and_label(Role::Status, "Saving");
}

/// `currentColor`: the `Ui`'s text colour, unless the caller gives one.
#[test]
fn a_spinner_takes_the_uis_text_colour() {
    let colour = |set: Option<Color32>| {
        let (_, out) = common::measure_output(&Theme::vega(), egui::Theme::Light, |ui| {
            ui.visuals_mut().override_text_color = Some(Color32::RED);
            match set {
                Some(c) => ui.add(Spinner::new().color(c)),
                None => spinner(ui),
            }
        });
        common::mesh_colour(&icons(&out)[0])
    };
    assert_eq!(colour(None), Color32::RED);
    assert_eq!(colour(Some(Color32::BLUE)), Color32::BLUE);
}

/// `animate-spin`: one turn a second, linear, from the start of the
/// app's clock; each visible frame asks for the next.
#[test]
fn the_angle_follows_the_clock() {
    let ctx = egui::Context::default();
    for time in [0.0, 2.25] {
        let (_, moving) = timed(&ctx, time, |ui| {
            spinner(ui);
        });
        assert!(moving, "{time}");
    }
    assert_quarter_turn(
        |ui| {
            spinner(ui);
        },
        "spinner",
    );
}

#[test]
fn a_hidden_spinner_asks_for_no_repaint() {
    let ctx = egui::Context::default();
    let hidden = |ui: &mut egui::Ui| {
        ui.set_clip_rect(egui::Rect::from_min_size(
            egui::pos2(-100.0, -100.0),
            Vec2::splat(1.0),
        ));
        spinner(ui);
    };
    // The first pass applies the theme, which asks for a pass of its own.
    timed(&ctx, 0.0, hidden);
    let (painted, moving) = timed(&ctx, 0.5, hidden);
    assert!(painted.is_empty());
    assert!(!moving);
}

/// Reduced motion (`animation_time == 0`, D-88 (c)): the spinner stands
/// still at its start angle and asks for nothing.
#[test]
fn reduced_motion_stops_it() {
    let ctx = egui::Context::default();
    ctx.all_styles_mut(|s| s.animation_time = 0.0);
    // The first pass applies the theme, which asks for a pass of its own.
    timed(&ctx, 0.0, |ui| {
        spinner(ui);
    });
    for time in [0.3, 0.6] {
        let (painted, moving) = timed(&ctx, time, |ui| {
            spinner(ui);
        });
        assert_eq!(angle(&painted[0]), 0.0);
        assert!(!moving);
    }
}

/// `spinner-button-demo`: in a button the spinner keeps its own `size-4`
/// (the button sizes only icons without one) and draws in the button's
/// text colour; an icon size draws it alone.
#[test]
fn a_spinner_in_a_button_keeps_its_size_and_takes_the_buttons_colour() {
    for (name, theme, mode) in common::themes() {
        for size in [ButtonSize::Sm, ButtonSize::Default, ButtonSize::Icon] {
            let (_, out) = common::measure_output(&theme, mode, |ui| {
                ui.add(
                    Button::new("Loading")
                        .size(size)
                        .variant(ButtonVariant::Outline)
                        .icon(Glyph::Spinner),
                )
            });
            let painted = icons(&out);
            assert_eq!(painted.len(), 1, "{name}");
            assert_eq!(
                painted[0].calc_bounds().size(),
                Vec2::splat(16.0),
                "{name} {size:?}"
            );
            if size != ButtonSize::Icon {
                assert_eq!(
                    common::mesh_colour(&painted[0]),
                    common::text_colour(&out),
                    "{name}"
                );
            }
        }
    }
}

/// `spinner-badge-demo`, `badge-spinner`: the badge's `[&>svg]:size-3!`
/// reaches the spinner, before and after the text.
#[test]
fn a_spinner_in_a_badge_takes_the_badges_icon_size() {
    for (name, theme, mode) in common::themes() {
        for end in [false, true] {
            let (_, out) = common::measure_output(&theme, mode, |ui| {
                let b = Badge::new("Syncing").variant(BadgeVariant::Secondary);
                ui.add(if end {
                    b.icon_end(Glyph::Spinner)
                } else {
                    b.icon(Glyph::Spinner)
                })
            });
            let painted = icons(&out);
            assert_eq!(painted.len(), 1, "{name}");
            let want = theme.style.badge.sized_icon_size.get(Default::default());
            assert_eq!(
                Some(painted[0].calc_bounds().width()),
                want,
                "{name} end {end}"
            );
            assert_eq!(
                common::mesh_colour(&painted[0]),
                common::text_colour(&out),
                "{name}"
            );
        }
    }
}

/// Upstream's `<svg role="status" aria-label="Loading">` stays a node
/// inside the button or badge: the host reports a status child named
/// "Loading", and keeps its own name.
#[test]
fn a_hosted_spinner_is_a_status_inside_its_host() {
    let mut h = Harness::new_ui(|ui| {
        ui.add(
            Button::new("Processing")
                .size(ButtonSize::Sm)
                .icon(Glyph::Spinner),
        );
        ui.add(Badge::new("Syncing").icon_end(Glyph::Spinner));
    });
    h.run();
    for host in [
        h.get_by_role_and_label(Role::Button, "Processing"),
        h.get_by_label("Syncing"),
    ] {
        let status = host.get_by_role(Role::Status);
        assert_eq!(status.accesskit_node().label().as_deref(), Some("Loading"));
        assert_eq!(
            status.accesskit_node().parent().map(|p| p.id()),
            Some(host.accesskit_node().id())
        );
    }
}

/// A disabled button reports itself disabled, but not the spinner's
/// status inside it: ARIA's `role="status"` takes no `aria-disabled`, and
/// CORE-AAM passes a disabled state down to focusable descendants only;
/// upstream's `<svg role="status">` is neither a form control nor
/// focusable.
#[test]
fn a_hosted_spinner_in_a_disabled_host_is_not_disabled() {
    let mut h = Harness::new_ui(|ui| {
        ui.add_enabled(false, Button::new("Processing").icon(Glyph::Spinner));
    });
    h.run();
    let host = h.get_by_role_and_label(Role::Button, "Processing");
    assert!(host.accesskit_node().is_disabled(), "the button");
    let status = host.get_by_role(Role::Status);
    assert!(!status.accesskit_node().is_disabled(), "its status");
}

/// The size, colour, button and badge demos. `spinner-custom-demo` is not
/// ported: it turns another icon itself and uses no `<Spinner>`.
#[test]
fn spinner_snapshot() {
    common::snapshot_all("spinner", egui::vec2(420.0, 240.0), |ui| {
        ui.ctx().all_styles_mut(|s| s.animation_time = 0.0);
        layout::vstack(ui, 16.0, |ui| {
            // spinner-size-demo: size-3, size-4, size-6, size-8.
            layout::row(ui, 24.0, |ui| {
                for side in [12.0, 16.0, 24.0, 32.0] {
                    ui.add(Spinner::new().size(side));
                }
            });
            // spinner-color-demo: size-6 text-{red,green,blue,yellow,purple}-500.
            layout::row(ui, 24.0, |ui| {
                for (l, c, h) in [
                    (0.637, 0.237, 25.331),
                    (0.723, 0.219, 149.579),
                    (0.623, 0.214, 259.815),
                    (0.795, 0.184, 86.047),
                    (0.627, 0.265, 303.9),
                ] {
                    ui.add(Spinner::new().size(24.0).color(oklch_to_srgb(l, c, h)));
                }
            });
            // spinner-button-demo: disabled, size sm.
            layout::row(ui, 16.0, |ui| {
                ui.add_enabled_ui(false, |ui| {
                    layout::row(ui, 16.0, |ui| {
                        ui.add(
                            Button::new("Loading...")
                                .size(ButtonSize::Sm)
                                .icon(Glyph::Spinner),
                        );
                        ui.add(
                            Button::new("Please wait")
                                .size(ButtonSize::Sm)
                                .variant(ButtonVariant::Outline)
                                .icon(Glyph::Spinner),
                        );
                        ui.add(
                            Button::new("Processing")
                                .size(ButtonSize::Sm)
                                .variant(ButtonVariant::Secondary)
                                .icon(Glyph::Spinner),
                        );
                    });
                });
            });
            // spinner-badge-demo; its `[--radius:1.2rem]` draws the same
            // pill at h-5 as the style's own radius.
            layout::row(ui, 16.0, |ui| {
                ui.add(Badge::new("Syncing").icon(Glyph::Spinner));
                ui.add(
                    Badge::new("Updating")
                        .variant(BadgeVariant::Secondary)
                        .icon(Glyph::Spinner),
                );
                ui.add(
                    Badge::new("Processing")
                        .variant(BadgeVariant::Outline)
                        .icon(Glyph::Spinner),
                );
            });
            // badge-spinner: inline-start and inline-end.
            layout::row(ui, 8.0, |ui| {
                ui.add(
                    Badge::new("Deleting")
                        .variant(BadgeVariant::Destructive)
                        .icon(Glyph::Spinner),
                );
                ui.add(
                    Badge::new("Generating")
                        .variant(BadgeVariant::Secondary)
                        .icon_end(Glyph::Spinner),
                );
            });
        });
    });
}

/// A spinner in a button or badge turns as a bare one does, and each
/// frame that shows it asks for the next.
#[test]
fn a_hosted_spinner_turns() {
    let hosts: [fn(&mut egui::Ui); 2] = [
        |ui| {
            ui.add(Button::new("Saving").icon(Glyph::Spinner));
        },
        |ui| {
            ui.add(Badge::new("Syncing").icon_end(Glyph::Spinner));
        },
    ];
    for (i, host) in hosts.into_iter().enumerate() {
        let ctx = egui::Context::default();
        for time in [0.0, 2.25] {
            let (_, moving) = timed(&ctx, time, host);
            assert!(moving, "{i} {time}");
        }
        assert_quarter_turn(host, &format!("host {i}"));
    }
}

/// A button or badge outside the visible area draws no spinner and asks
/// for no repaint.
#[test]
fn a_hidden_host_asks_for_no_repaint() {
    let ctx = egui::Context::default();
    let hidden = |ui: &mut egui::Ui| {
        ui.set_clip_rect(egui::Rect::from_min_size(
            egui::pos2(-100.0, -100.0),
            Vec2::splat(1.0),
        ));
        ui.add(Button::new("Saving").icon(Glyph::Spinner));
        ui.add(Badge::new("Syncing").icon_end(Glyph::Spinner));
    };
    // The first pass applies the theme, which asks for a pass of its own.
    timed(&ctx, 0.0, hidden);
    let (painted, moving) = timed(&ctx, 0.5, hidden);
    assert!(painted.is_empty());
    assert!(!moving);
}

/// Turning a button's icon into a spinner and back leaves the ids of the
/// widgets after it alone: a focused text field keeps its focus.
#[test]
fn a_hosted_spinner_keeps_the_ids_after_it() {
    let ctx = egui::Context::default();
    let mut text = String::new();
    let mut field = |glyph: Glyph, focus: bool| {
        let mut id = egui::Id::NULL;
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            Theme::vega().apply(ui.ctx());
            ui.add(Button::new("Save").icon(glyph));
            ui.add(Badge::new("Draft").icon_end(glyph));
            let resp = ui.text_edit_singleline(&mut text);
            if focus {
                resp.request_focus();
            }
            id = resp.id;
        });
        id
    };
    let check: Glyph = Theme::vega().icons.check.into();
    let first = field(check, true);
    for glyph in [Glyph::Spinner, check, Glyph::Spinner] {
        assert_eq!(field(glyph, false), first, "{glyph:?}");
        assert_eq!(ctx.memory(|m| m.focused()), Some(first), "{glyph:?}");
    }
}

/// A spinner keeps one texture per size however long it turns: each
/// frame rasterises the next angle into the texture of the frame before,
/// and the texture of a size no longer shown is freed after a while.
#[test]
fn turning_spinners_reuse_their_textures() {
    let ctx = egui::Context::default();
    let live = || ctx.tex_manager().read().allocated().count();
    let mut counts = Vec::new();
    for frame in 0..200 {
        let sides: &[f32] = if frame < 100 {
            &[12.0, 16.0, 24.0]
        } else {
            &[16.0]
        };
        timed(&ctx, f64::from(frame) * 0.017, |ui| {
            for &side in sides {
                ui.add(Spinner::new().size(side));
            }
        });
        if [1, 99, 199].contains(&frame) {
            counts.push(live());
        }
    }
    assert_eq!(counts[0], counts[1], "no growth while turning: {counts:?}");
    assert_eq!(counts[2], counts[1] - 2, "two sizes freed: {counts:?}");
}

/// The spinner demos at a fixed angle (the clock at 0.3 s, 108 degrees),
/// at 1x and 2x: the turned loader keeps the edges of the unturned one.
#[test]
fn spinner_turned_snapshot() {
    let mut results = egui_kittest::SnapshotResults::new();
    for (ppp, suffix) in [(1.0, "1x"), (2.0, "2x")] {
        for (mode, m) in [(egui::Theme::Light, "light"), (egui::Theme::Dark, "dark")] {
            let theme = Theme::vega();
            let bg = theme.palette_of(mode).background;
            let mut h = Harness::builder()
                .with_size(egui::vec2(260.0, 120.0))
                .with_pixels_per_point(ppp)
                .with_theme(mode)
                .build_ui(move |ui| {
                    theme.apply(ui.ctx());
                    ui.ctx().all_styles_mut(|s| s.animation_time = 0.1);
                    let page = ui.ctx().viewport_rect();
                    ui.ctx()
                        .layer_painter(egui::LayerId::background())
                        .rect_filled(page, 0.0, bg);
                    egui::Frame::new().fill(bg).inner_margin(16).show(ui, |ui| {
                        layout::vstack(ui, 16.0, |ui| {
                            layout::row(ui, 24.0, |ui| {
                                for side in [12.0, 16.0, 24.0, 32.0] {
                                    ui.add(Spinner::new().size(side));
                                }
                            });
                            layout::row(ui, 16.0, |ui| {
                                ui.add(
                                    Button::new("Loading...")
                                        .size(ButtonSize::Sm)
                                        .icon(Glyph::Spinner),
                                );
                                ui.add(Badge::new("Syncing").icon(Glyph::Spinner));
                            });
                        });
                    });
                });
            for _ in 0..3 {
                h.input_mut().time = Some(0.3);
                h.step();
            }
            results.add(common::try_snapshot(
                &mut h,
                format!("spinner_turned_vega_{m}_{suffix}"),
            ));
        }
    }
}

/// More spinner sizes in one pass than turned textures are kept: the
/// spinners past the bound draw the unturned bitmap turned, sampled
/// smoothed (a nearest sample would leave stair steps), never ragged.
#[test]
fn spinners_past_the_turned_texture_bound_are_smoothed() {
    let ctx = egui::Context::default();
    let mut options = std::collections::HashMap::new();
    let mut meshes = Vec::new();
    for time in [0.3, 0.4] {
        let out = ctx.run_ui(
            egui::RawInput {
                time: Some(time),
                ..Default::default()
            },
            |ui| {
                Theme::vega().apply(ui.ctx());
                ui.horizontal_wrapped(|ui| {
                    for side in 10..80 {
                        ui.add(Spinner::new().size(side as f32));
                    }
                });
            },
        );
        for (id, delta) in &out.textures_delta.set {
            options.insert(*id, delta.options);
        }
        meshes = icons(&out);
    }
    let turned: Vec<_> = meshes.iter().filter(|m| angle(m).abs() > 1e-3).collect();
    assert!(!turned.is_empty(), "the bound is reached");
    for m in turned {
        assert_eq!(
            options[&m.texture_id].magnification,
            egui::TextureFilter::Linear,
            "{:?}",
            m.calc_bounds()
        );
    }
}

/// How an app drives egui each frame.
#[derive(Clone, Copy, Debug)]
enum Loop {
    /// `Context::run_ui`, which runs egui's plugin pass hooks.
    RunUi,
    /// The deprecated `Context::run` of custom loops.
    Run,
    /// `Context::begin_pass` and `Context::end_pass` of custom loops.
    Split,
}

/// One pass of `add` in viewport `vp` at `time` on `ctx`, with vega
/// applied.
fn run_in(
    ctx: &egui::Context,
    vp: egui::ViewportId,
    time: f64,
    add: impl FnMut(&mut egui::Ui),
) -> egui::FullOutput {
    run_by(Loop::RunUi, ctx, vp, time, add)
}

/// [`run_in`], driven by `by`.
#[expect(deprecated, reason = "custom loops still call `Context::run`")]
fn run_by(
    by: Loop,
    ctx: &egui::Context,
    vp: egui::ViewportId,
    time: f64,
    mut add: impl FnMut(&mut egui::Ui),
) -> egui::FullOutput {
    let mut input = egui::RawInput {
        time: Some(time),
        viewport_id: vp,
        ..Default::default()
    };
    if vp != egui::ViewportId::ROOT {
        input.viewports.insert(vp, Default::default());
    }
    match by {
        Loop::RunUi => ctx.run_ui(input, |ui| {
            Theme::vega().apply(ui.ctx());
            add(ui);
        }),
        Loop::Run => ctx.run(input, |ctx| central(ctx, &mut add)),
        Loop::Split => {
            ctx.begin_pass(input);
            central(ctx, &mut add);
            ctx.end_pass()
        }
    }
}

/// `add` in a root `Ui` of its own, with vega applied: a custom loop's
/// pass, which runs no plugin pass hooks.
fn central(ctx: &egui::Context, add: &mut dyn FnMut(&mut egui::Ui)) {
    Theme::vega().apply(ctx);
    let mut ui = egui::Ui::new(
        ctx.clone(),
        egui::Id::new("custom_loop"),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.content_rect()),
    );
    add(&mut ui);
}

/// How many turned icons a frame draws as the unturned bitmap turned
/// (the fallback: a turned quad) and how many rasterised at their angle
/// (an upright quad).
fn fallen_and_turned(out: &egui::FullOutput) -> (usize, usize) {
    let all = icons(out);
    let fallen = all.iter().filter(|m| angle(m).abs() > 1e-3).count();
    (fallen, all.len() - fallen)
}

/// The turned textures `ctx` holds.
fn turned_textures(ctx: &egui::Context) -> usize {
    let mngr = ctx.tex_manager();
    let mngr = mngr.read();
    mngr.allocated()
        .filter(|(_, m)| m.name.ends_with(":turned"))
        .count()
}

/// Every size of a pass that fills the turned textures.
fn many_sizes(ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        for side in 10..80 {
            ui.add(Spinner::new().size(side as f32));
        }
    });
}

/// A viewport that drew many spinners and is gone leaves no textures
/// behind: after a while the spinner of another viewport is rasterised
/// at its angle again, and when no spinner turns, no turned texture is
/// kept; also in custom loops, which run no plugin pass hooks.
#[test]
fn a_gone_viewports_turned_textures_are_freed() {
    // Per loop: what the root's last spinner frame drew (fallen back,
    // rasterised at its angle) and the turned textures kept when idle.
    let mut got = Vec::new();
    for by in [Loop::RunUi, Loop::Run, Loop::Split] {
        let ctx = egui::Context::default();
        let popup = egui::ViewportId::from_hash_of("popup");
        let out = run_by(by, &ctx, popup, 0.3, many_sizes);
        assert!(
            fallen_and_turned(&out).0 > 0,
            "{by:?}: the bound is reached"
        );
        let mut last = (0, 0);
        for frame in 0..100 {
            let out = run_by(
                by,
                &ctx,
                egui::ViewportId::ROOT,
                0.31 + f64::from(frame) * 0.017,
                |ui| {
                    ui.add(Spinner::new().size(16.0));
                },
            );
            last = fallen_and_turned(&out);
        }
        for frame in 0..100 {
            run_by(
                by,
                &ctx,
                egui::ViewportId::ROOT,
                2.0 + f64::from(frame) * 0.017,
                |_| {},
            );
        }
        got.push((by, last, turned_textures(&ctx)));
    }
    let want = [Loop::RunUi, Loop::Run, Loop::Split].map(|by| (by, (0, 1), 0));
    assert_eq!(format!("{got:?}"), format!("{want:?}"));
}

/// The idle passes until the turned textures are freed after the last
/// spinner pass.
fn passes_to_free(ctx: &egui::Context) -> Option<u32> {
    (1..=200).find(|&i| {
        run_in(
            ctx,
            egui::ViewportId::ROOT,
            1.0 + f64::from(i) * 0.017,
            |_| {},
        );
        turned_textures(ctx) == 0
    })
}

/// A turned texture lives as many idle passes after a reset of egui's
/// memory (its "Reset memory" button) as before.
#[test]
fn a_memory_reset_keeps_how_long_turned_textures_live() {
    let ctx = egui::Context::default();
    let spin = |ctx: &egui::Context, t0: f64| {
        for f in 0..5 {
            run_in(
                ctx,
                egui::ViewportId::ROOT,
                t0 + f64::from(f) * 0.017,
                |ui| {
                    ui.add(Spinner::new().size(16.0));
                },
            );
        }
    };
    let mut lives = Vec::new();
    for _ in 0..3 {
        spin(&ctx, 0.3);
        lives.push(passes_to_free(&ctx));
        ctx.memory_mut(|m| *m = Default::default());
    }
    assert_eq!(lives, [Some(60); 3]);
}

/// A new spinner size takes the room of sizes the last pass drew and
/// this one does not, at once, even when they filled the turned
/// textures.
#[test]
fn a_new_size_takes_the_room_of_sizes_no_longer_drawn() {
    let ctx = egui::Context::default();
    let root = egui::ViewportId::ROOT;
    run_in(&ctx, root, 0.3, many_sizes);
    for frame in 0..10 {
        let out = run_in(&ctx, root, 0.31 + f64::from(frame) * 0.017, |ui| {
            ui.add(Spinner::new().size(100.0));
        });
        assert_eq!(fallen_and_turned(&out), (0, 1), "frame {frame}");
    }
}
