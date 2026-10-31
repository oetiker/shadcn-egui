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
    let (start, moving) = timed(&ctx, 0.0, |ui| {
        spinner(ui);
    });
    assert!(moving);
    let (quarter, moving) = timed(&ctx, 2.25, |ui| {
        spinner(ui);
    });
    assert!(moving);
    let turned = angle(&quarter[0]) - angle(&start[0]);
    assert!(
        (turned - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
        "{turned}"
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
        let (start, moving) = timed(&ctx, 0.0, host);
        assert!(moving, "{i}");
        let (quarter, moving) = timed(&ctx, 2.25, host);
        assert!(moving, "{i}");
        let turned = angle(&quarter[0]) - angle(&start[0]);
        assert!(
            (turned - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
            "{i}: {turned}"
        );
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
