//! Alert: layout, colours, action placement, accessibility, and the
//! shadcn-svelte demos as snapshots.

mod common;

use egui::{vec2, Color32, Rect, Ui};
use shadcn_egui::color::oklch_to_srgb;
use shadcn_egui::components::alert::{alert_action, Alert, AlertVariant};
use shadcn_egui::components::button::{Button, ButtonSize};
use shadcn_egui::icons::lucide;
use shadcn_egui::style::{AppVariant, Paint, Role, Style};
use shadcn_egui::{Palette, Theme};

fn vega() -> Theme {
    Theme::new(Style::vega(), Palette::light(), Palette::dark(), 10.0)
}

/// Run frames until the layout settles and return what the last one saw.
fn settle<T>(theme: &Theme, width: f32, mut add: impl FnMut(&mut Ui) -> T) -> T {
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    ctx.all_styles_mut(|s| s.animation_time = 0.0);
    let input = || egui::RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(width, 800.0))),
        ..Default::default()
    };
    let mut last = None;
    for _ in 0..3 {
        let _ = ctx.run_ui(input(), |ui| last = Some(add(ui)));
    }
    last.expect("a frame ran")
}

/// vega: `border px-4 py-3`, a `text-sm` (20 px) line per row, rows
/// `gap-0.5` apart.
#[test]
fn height_is_border_padding_and_rows() {
    let t = vega();
    let h = |a: fn() -> Alert| settle(&t, 900.0, |ui| ui.add(a()).rect.height());
    assert_eq!(
        h(|| Alert::new().title("Saved").description("All good.")),
        68.0
    );
    assert_eq!(h(|| Alert::new().title("Saved")), 46.0);
    assert_eq!(h(|| Alert::new().description("All good.")), 46.0);
    // The icon spans two rows: a lone title keeps the empty row's gap.
    assert_eq!(h(|| Alert::new().icon(lucide::INFO).title("Saved")), 48.0);
    assert_eq!(
        h(|| Alert::new()
            .icon(lucide::INFO)
            .title("Saved")
            .description("All good.")),
        68.0
    );
    // An icon alone is as high as itself.
    assert_eq!(
        h(|| Alert::new().icon(lucide::INFO)),
        1.0 + 12.0 + 16.0 + 12.0 + 1.0
    );
}

/// `w-full`: the alert fills its `Ui`; the text starts after the border,
/// the padding and, with an icon, the icon and `gap-x-2.5`.
#[test]
fn fills_the_width_and_insets_the_text() {
    let t = vega();
    let (alert, text) = settle(&t, 900.0, |ui| {
        ui.set_max_width(448.0);
        let r = Alert::new().icon(lucide::INFO).show(ui, |ui| {
            shadcn_egui::components::alert::alert_title(ui, "Saved").rect
        });
        (r.response.rect, r.inner)
    });
    assert_eq!(alert.width(), 448.0);
    assert_eq!(text.left() - alert.left(), 1.0 + 16.0 + 16.0 + 10.0);
    assert_eq!(text.top() - alert.top(), 1.0 + 12.0);
}

/// `w-full` in a `layout::wrap_row`: a flex item as wide as the line goes
/// onto a line of its own (porting rule 6), never past the row's end.
#[test]
fn wraps_to_its_own_line_in_a_wrap_row() {
    let t = vega();
    let (button, alert) = settle(&t, 900.0, |ui| {
        ui.set_max_width(448.0);
        shadcn_egui::layout::wrap_row(ui, 8.0, |ui| {
            let b = ui.add(Button::new("Before")).rect;
            let a = ui.add(Alert::new().title("Alert in a row")).rect;
            (b, a)
        })
    });
    assert_eq!(alert.left(), button.left(), "{button:?} {alert:?}");
    assert!(alert.top() >= button.bottom() + 8.0, "{button:?} {alert:?}");
    assert_eq!(alert.width(), 448.0);
}

/// In unbounded room the alert is as wide as its content plus its padding,
/// never infinite.
#[test]
fn unbounded_room_takes_the_content_width() {
    let t = vega();
    let (alert, title) = settle(&t, 900.0, |ui| {
        ui.allocate_ui(vec2(f32::INFINITY, 200.0), |ui| {
            let r = Alert::new().show(ui, |ui| {
                shadcn_egui::components::alert::alert_title(ui, "Saved").rect
            });
            (r.response.rect, r.inner)
        })
        .inner
    });
    assert!(alert.width().is_finite());
    assert_eq!(alert.width(), title.width() + 2.0 * (1.0 + 16.0));
}

/// The action sits at `top-2.5 right-3` inside the border, and the alert
/// reserves `pr-18` for it from the next pass on.
#[test]
fn action_sits_top_right_and_reserves_room() {
    let t = vega();
    let long = "Enable it under your profile settings to get started, \
                and tell your friends about it while you are there.";
    let (alert, button, desc) = settle(&t, 900.0, |ui| {
        ui.set_max_width(448.0);
        let r = Alert::new().title("Dark mode").show(ui, |ui| {
            let d = shadcn_egui::components::alert::alert_description(ui, long).rect;
            let b = alert_action(ui, |ui| ui.add(Button::new("Enable").size(ButtonSize::Xs)))
                .inner
                .rect;
            (b, d)
        });
        (r.response.rect, r.inner.0, r.inner.1)
    });
    assert_eq!(button.right(), alert.right() - 1.0 - 12.0);
    assert_eq!(button.top(), alert.top() + 1.0 + 10.0);
    assert!(
        desc.right() <= alert.right() - 1.0 - 72.0 + 0.5,
        "{desc:?} {alert:?}"
    );
    // The action takes no room: the alert is as high as without it.
    let plain = settle(&t, 900.0, |ui| {
        ui.set_max_width(448.0);
        Alert::new()
            .title("Dark mode")
            .show(ui, |ui| {
                ui.set_max_width(448.0 - 2.0 - 16.0 - 72.0);
                shadcn_egui::components::alert::alert_description(ui, long);
            })
            .response
            .rect
    });
    assert_eq!(alert.height(), plain.height());
}

/// The variant's colours: title in the alert's text colour, description
/// muted, or destructive at 90 % in the destructive variant.
#[test]
fn variant_colours_reach_title_and_description() {
    for (name, theme, mode) in common::themes() {
        let p = theme.palette_of(mode);
        let title = |v: AlertVariant| {
            let (_, out) = common::measure_output(&theme, mode, |ui| {
                ui.add(Alert::new().variant(v).title("Saved"))
            });
            common::text_colour(&out)
        };
        let desc = |v: AlertVariant| {
            let (_, out) = common::measure_output(&theme, mode, |ui| {
                ui.add(Alert::new().variant(v).description("All good."))
            });
            common::text_colour(&out)
        };
        assert_eq!(title(AlertVariant::Default), p.card_foreground, "{name}");
        assert_eq!(title(AlertVariant::Destructive), p.destructive, "{name}");
        assert_eq!(desc(AlertVariant::Default), p.muted_foreground, "{name}");
        assert_eq!(
            desc(AlertVariant::Destructive),
            p.destructive.gamma_multiply(0.9),
            "{name}"
        );
    }
}

/// Plain egui text in the closure takes the alert's text colour.
#[test]
fn caller_content_takes_the_alert_text_colour() {
    for (name, theme, mode) in common::themes() {
        let p = theme.palette_of(mode);
        let (_, out) = common::measure_output(&theme, mode, |ui| {
            Alert::new()
                .variant(AlertVariant::Destructive)
                .show(ui, |ui| ui.label("Plain"))
                .response
        });
        assert_eq!(common::text_colour(&out), p.destructive, "{name}");
    }
}

/// `role="alert"`.
#[test]
fn reports_the_alert_role() {
    use egui_kittest::kittest::Queryable;
    let mut h = egui_kittest::Harness::new_ui(|ui| {
        ui.add(Alert::new().title("Saved").description("All good."));
    });
    h.run();
    let _ = h.get_by_role(egui::accesskit::Role::Alert);
    let _ = h.get_by_label("Saved");
}

/// The alert's node holds its title, description and the closure's
/// content, as upstream's `div role="alert"` holds its children: a screen
/// reader announces the live region's subtree.
#[test]
fn the_alert_node_holds_its_content() {
    use egui_kittest::kittest::{NodeT, Queryable};
    let mut h = egui_kittest::Harness::new_ui(|ui| {
        Alert::new()
            .title("Saved")
            .description("All good.")
            .show(ui, |ui| ui.label("More"));
    });
    h.run();
    let alert = h
        .get_by_role(egui::accesskit::Role::Alert)
        .accesskit_node()
        .id();
    for label in ["Saved", "All good.", "More"] {
        let node = h.get_by_label(label);
        let mut up = node.parent();
        while let Some(p) = up {
            if p.accesskit_node().id() == alert {
                break;
            }
            up = p.parent();
        }
        assert!(up.is_some(), "{label} is not inside the alert node");
    }
}

/// Degenerate input: an empty alert and a zero-width room draw without
/// NaN or a panic.
#[test]
fn degenerate_input_draws() {
    let t = vega();
    let empty = settle(&t, 900.0, |ui| ui.add(Alert::new()).rect);
    assert_eq!(empty.height(), 1.0 + 12.0 + 12.0 + 1.0);
    let narrow = settle(&t, 900.0, |ui| {
        ui.set_max_width(0.0);
        ui.add(Alert::new().icon(lucide::INFO).title("Saved")).rect
    });
    assert!(narrow.is_finite());
}

const AMBER_BG: Role = Role::custom("amber-bg");
const AMBER_BORDER: Role = Role::custom("amber-border");
const AMBER_TEXT: Role = Role::custom("amber-text");
const AMBER: AppVariant<AlertVariant> = AlertVariant::app("amber");

/// `alert-colors.svelte`: `border-amber-200 bg-amber-50 text-amber-900`,
/// `dark:border-amber-900 dark:bg-amber-950 dark:text-amber-50`, as an app
/// variant with app colours (Tailwind v4's amber scale).
fn with_amber(theme: Theme) -> Theme {
    let amber_50 = oklch_to_srgb(0.987, 0.022, 95.277);
    let amber_200 = oklch_to_srgb(0.924, 0.12, 95.746);
    let amber_900 = oklch_to_srgb(0.414, 0.112, 45.904);
    let amber_950 = oklch_to_srgb(0.279, 0.077, 45.635);
    theme
        .color(AMBER_BG, amber_50, amber_950)
        .color(AMBER_BORDER, amber_200, amber_900)
        .color(AMBER_TEXT, amber_900, amber_50)
        .variant(AMBER, AlertVariant::Default, |r, _| {
            r.alert.bg.set(Paint::Role(AMBER_BG, 1.0));
            r.alert.border_color.set(Paint::Role(AMBER_BORDER, 1.0));
            r.alert.text.set(Paint::Role(AMBER_TEXT, 1.0));
        })
}

/// The app variant draws its colours and keeps the base's description.
#[test]
fn app_variant_colours_the_box_and_title() {
    for (name, theme, mode) in common::themes() {
        let theme = with_amber(theme);
        let p = theme.palette_of(mode);
        let (_, out) = common::measure_output(&theme, mode, |ui| {
            ui.add(Alert::new().variant(AMBER).title("Expiring"))
        });
        let want: Color32 = if mode == egui::Theme::Dark {
            oklch_to_srgb(0.987, 0.022, 95.277)
        } else {
            oklch_to_srgb(0.414, 0.112, 45.904)
        };
        assert_eq!(common::text_colour(&out), want, "{name}");
        let (_, out) = common::measure_output(&theme, mode, |ui| {
            ui.add(Alert::new().variant(AMBER).description("Renew now."))
        });
        assert_eq!(common::text_colour(&out), p.muted_foreground, "{name}");
    }
}

/// `max-w-md` on the page: a column at most 448 px wide.
fn max_w_md(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.scope(|ui| {
        ui.set_max_width(448.0);
        shadcn_egui::layout::vstack(ui, 16.0, add);
    });
}

/// The shadcn-svelte demos: `alert-demo`, `alert-basic`,
/// `alert-destructive`, `alert-action` and `alert-colors`, each in
/// `max-w-md`.
fn demos(ui: &mut Ui) {
    max_w_md(ui, |ui| {
        ui.add(
            Alert::new()
                .icon(lucide::CIRCLE_CHECK)
                .title("Payment successful")
                .description(
                    "Your payment of $29.99 has been processed. A receipt has been sent to \
                     your email address.",
                ),
        );
        ui.add(
            Alert::new()
                .icon(lucide::INFO)
                .title("New feature available")
                .description(
                    "We've added dark mode support. You can enable it in your account settings.",
                ),
        );
        ui.add(
            Alert::new()
                .icon(lucide::CIRCLE_CHECK)
                .title("Account updated successfully")
                .description(
                    "Your profile information has been saved. Changes will be reflected \
                     immediately.",
                ),
        );
        ui.add(
            Alert::new()
                .variant(AlertVariant::Destructive)
                .icon(lucide::CIRCLE_ALERT)
                .title("Payment failed")
                .description(
                    "Your payment could not be processed. Please check your payment method \
                     and try again.",
                ),
        );
        Alert::new()
            .title("Dark mode is now available")
            .description("Enable it under your profile settings to get started.")
            .show(ui, |ui| {
                alert_action(ui, |ui| {
                    ui.add(Button::new("Enable").size(ButtonSize::Xs));
                });
            });
        ui.add(
            Alert::new()
                .variant(AMBER)
                .icon(lucide::TRIANGLE_ALERT)
                .title("Your subscription will expire in 3 days.")
                .description(
                    "Renew now to avoid service interruption or upgrade to a paid plan to \
                     continue using the service.",
                ),
        );
    });
}

#[test]
fn alert_snapshot() {
    let themes = common::snapshot_themes()
        .into_iter()
        .map(|(n, t, m)| (n, with_amber(t), m))
        .collect();
    common::snapshot_with_themes("alert", vec2(512.0, 640.0), themes, demos, |_| {});
}

/// At `md` and wider the description wraps `text-pretty` instead of
/// `text-balance`.
#[test]
fn alert_md_snapshot() {
    common::snapshot_all("alert_md", vec2(800.0, 130.0), |ui| {
        max_w_md(ui, |ui| {
            ui.add(
                Alert::new()
                    .icon(lucide::CIRCLE_CHECK)
                    .title("Payment successful")
                    .description(
                        "Your payment of $29.99 has been processed. A receipt has been sent \
                         to your email address.",
                    ),
            );
        });
    });
}
