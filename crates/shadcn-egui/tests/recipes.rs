//! Every visual value comes from the style: an app that changes a recipe
//! through `theme.style` sees the change.

mod common;

use egui::vec2;
use shadcn_egui::components::tabs::{Tabs, TabsVariant};
use shadcn_egui::style::{Paint, Role, Rule, State, When};
use shadcn_egui::Theme;

/// Issue #6: a filled `primary` active tab, `primary-foreground` text, a
/// solid 2 px ring and no outline, all through `theme.style`.
fn issue6(theme: Theme) -> Theme {
    common::restyled(theme, |s| {
        let tr = &mut s.tabs_trigger;
        tr.bg.rules.push(Rule::important(
            When::all(State::SELECTED),
            Paint::Role(Role::Primary, 1.0),
        ));
        tr.text.rules.push(Rule::important(
            When::all(State::SELECTED),
            Paint::Role(Role::PrimaryForeground, 1.0),
        ));
        tr.ring_color.rules.push(Rule::important(
            When::all(State::FOCUS_VISIBLE),
            Paint::Role(Role::Ring, 1.0),
        ));
        tr.ring_width
            .rules
            .push(Rule::important(When::all(State::FOCUS_VISIBLE), 2.0));
        tr.outline_width
            .rules
            .push(Rule::important(When::ALWAYS, 0.0));
    })
}

fn focused_tabs(ui: &mut egui::Ui) -> egui::Response {
    let resp = ui.add(Tabs::new(&mut 0, &["Account", "Password"]));
    ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", 0))));
    resp
}

#[test]
fn issue6_tab_trigger_restyled_through_theme_style() {
    for theme in [Theme::vega_light(), Theme::vega_dark()] {
        let theme = issue6(theme);
        let (_, out) = common::measure_output(&theme, focused_tabs);
        let shapes = common::rect_shapes(&out);
        let p = &theme.palette;
        assert!(
            shapes.iter().any(|r| r.fill == p.primary),
            "active fill: {shapes:?}"
        );
        assert!(
            shapes.iter().any(|r| r.stroke.width == 2.0
                && r.stroke.color == p.ring
                && r.stroke_kind == egui::StrokeKind::Outside),
            "2px ring: {shapes:?}"
        );
        assert!(
            !shapes.iter().any(|r| r.stroke.width == 1.0
                && r.stroke.color == p.ring
                && r.stroke_kind == egui::StrokeKind::Outside),
            "no outline: {shapes:?}"
        );
        assert_eq!(common::text_colour(&out), p.primary_foreground);
    }
    common::snapshot_with_themes(
        "tabs_issue6",
        vec2(320.0, 90.0),
        vec![
            ("vega_light", issue6(Theme::vega_light())),
            ("vega_dark", issue6(Theme::vega_dark())),
        ],
        |ui| {
            focused_tabs(ui);
        },
        |_| {},
    );
}

/// An app's important border rule wins on the active trigger. (Upstream
/// luma's `border-transparent!` was important too; correction C1 removes
/// it, so `important_app_rule_beats_generated_important` covers that case.)
#[test]
fn important_app_border_wins_on_active_trigger() {
    let luma = Theme::new(
        shadcn_egui::Style::luma(),
        shadcn_egui::Palette::light(),
        10.0,
    );
    let theme = common::restyled(luma, |s| {
        s.tabs_trigger.border_color.rules.push(Rule::important(
            When::all(State::SELECTED),
            Paint::Role(Role::Primary, 1.0),
        ));
    });
    let (_, out) = common::measure_output(&theme, |ui| ui.add(Tabs::new(&mut 0, &["A", "B"])));
    assert!(
        common::rect_shapes(&out)
            .iter()
            .any(|r| r.stroke.color == theme.palette.primary
                && r.stroke_kind == egui::StrokeKind::Inside),
        "primary border on the active trigger"
    );
}

/// The same app rule wins where the code used to force a border: on the
/// focused trigger (`focus-visible:border-ring`) and on the dark active
/// trigger (`dark:data-active:border-input`).
#[test]
fn important_app_border_wins_on_focused_and_dark_active_trigger() {
    for palette in [shadcn_egui::Palette::light(), shadcn_egui::Palette::dark()] {
        let luma = Theme::new(shadcn_egui::Style::luma(), palette, 10.0);
        let theme = common::restyled(luma, |s| {
            s.tabs_trigger.border_color.rules.push(Rule::important(
                When::ALWAYS,
                Paint::Role(Role::Primary, 1.0),
            ));
        });
        let (_, out) = common::measure_output(&theme, focused_tabs);
        let borders: Vec<_> = common::rect_shapes(&out)
            .into_iter()
            .filter(|r| r.stroke_kind == egui::StrokeKind::Inside && r.stroke.width > 0.0)
            .collect();
        assert!(
            borders
                .iter()
                .all(|r| r.stroke.color == theme.palette.primary),
            "every trigger border is the app's primary: {borders:?}"
        );
        assert_eq!(borders.len(), 2, "{borders:?}");
    }
}

/// Luma's `[&>svg]:size-3!` is generated as an important rule; an app's
/// important rule comes later and wins.
#[test]
fn important_app_rule_beats_generated_important() {
    use shadcn_egui::components::badge::Badge;
    use shadcn_egui::icons::lucide::X;
    let luma = Theme::new(
        shadcn_egui::Style::luma(),
        shadcn_egui::Palette::light(),
        10.0,
    );
    let badge = |ui: &mut egui::Ui| ui.add(Badge::new("Badge").icon(&X));
    let before = common::measure(&luma, badge);
    let theme = common::restyled(luma, |s| {
        s.badge
            .icon_size
            .rules
            .push(Rule::important(When::ALWAYS, 20.0));
    });
    let after = common::measure(&theme, badge);
    assert_eq!(after.width() - before.width(), 20.0 - 12.0);
}

/// C2 is a plain correction, not an important rule: an app's important
/// ring on a focused line trigger still wins.
#[test]
fn important_app_ring_beats_line_focus_correction() {
    let theme = common::restyled(Theme::vega_light(), |s| {
        let tr = &mut s.tabs_trigger;
        tr.ring_width
            .rules
            .push(Rule::important(When::all(State::FOCUS_VISIBLE), 2.0));
        tr.ring_color.rules.push(Rule::important(
            When::all(State::FOCUS_VISIBLE),
            Paint::Role(Role::Primary, 1.0),
        ));
    });
    let (_, out) = common::measure_output(&theme, |ui| {
        let resp = ui.add(Tabs::new(&mut 0, &["A", "B"]).variant(TabsVariant::Line));
        ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", 1))));
        resp
    });
    let shapes = common::rect_shapes(&out);
    assert!(
        shapes
            .iter()
            .any(|r| r.stroke_kind == egui::StrokeKind::Outside
                && r.stroke.width == 2.0
                && r.stroke.color == theme.palette.primary),
        "app ring: {shapes:?}"
    );
}

#[test]
fn tabs_track_and_line_bar_come_from_the_style() {
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.tabs_list_variant_default.bg.base = Some(Paint::Role(Role::Primary, 1.0));
        s.tabs_trigger_after.bg.base = Some(Paint::Role(Role::Destructive, 1.0));
        s.tabs_trigger_after
            .height
            .rules
            .push(Rule::important(When::ALWAYS, 4.0));
    });
    let (rect, out) = common::measure_output(&theme, |ui| ui.add(Tabs::new(&mut 0, &["A", "B"])));
    let shapes = common::rect_shapes(&out);
    assert!(
        shapes
            .iter()
            .any(|r| r.fill == theme.palette.primary && r.rect == rect),
        "track: {shapes:?}"
    );
    let (_, out) = common::measure_output(&theme, |ui| {
        ui.add(Tabs::new(&mut 0, &["A", "B"]).variant(TabsVariant::Line))
    });
    let bar = common::rect_shapes(&out)
        .into_iter()
        .find(|r| r.fill == theme.palette.destructive)
        .expect("line bar");
    assert_eq!(bar.rect.height(), 4.0);
}

/// Review Focus 5: the bar of a disabled line list fades with its trigger.
#[test]
fn disabled_line_bar_is_faded_with_its_trigger() {
    let theme = Theme::vega_light();
    let (_, out) = common::measure_output(&theme, |ui| {
        ui.add(
            Tabs::new(&mut 0, &["A", "B"])
                .variant(TabsVariant::Line)
                .enabled(false),
        )
    });
    let fg = theme.palette.foreground;
    let bars: Vec<_> = common::rect_shapes(&out)
        .into_iter()
        .filter(|r| r.rect.height() == 2.0)
        .collect();
    assert_eq!(bars.len(), 1, "{bars:?}");
    assert_eq!(bars[0].fill, fg.gamma_multiply(0.5));
}

/// C1: luma and rhea keep the base classes' borders: `Input` on the dark
/// active trigger, `Ring` on a focused one.
#[test]
fn luma_and_rhea_keep_the_dark_active_and_focus_border() {
    type Make = fn() -> shadcn_egui::Style;
    let styles: [(&str, Make); 2] = [
        ("luma", shadcn_egui::Style::luma),
        ("rhea", shadcn_egui::Style::rhea),
    ];
    for (name, make) in styles {
        let dark = Theme::new(make(), shadcn_egui::Palette::dark(), 10.0);
        let (_, out) = common::measure_output(&dark, |ui| ui.add(Tabs::new(&mut 0, &["A", "B"])));
        let inside = |out: &egui::FullOutput, c: egui::Color32| {
            common::rect_shapes(out).iter().any(|r| {
                r.stroke.width > 0.0
                    && r.stroke.color == c
                    && r.stroke_kind == egui::StrokeKind::Inside
            })
        };
        assert!(
            inside(&out, dark.palette.input),
            "{name}: dark active Input border"
        );
        let light = Theme::new(make(), shadcn_egui::Palette::light(), 10.0);
        let (_, out) = common::measure_output(&light, focused_tabs);
        assert!(
            inside(&out, light.palette.ring),
            "{name}: focus Ring border"
        );
    }
}

/// The colour of every text a frame painted, in paint order.
fn text_colours(out: &egui::FullOutput) -> Vec<egui::Color32> {
    fn walk(s: &egui::Shape, acc: &mut Vec<egui::Color32>) {
        match s {
            egui::Shape::Text(t) => {
                if let Some(v) = t
                    .galley
                    .rows
                    .iter()
                    .find_map(|r| r.visuals.mesh.vertices.first())
                {
                    let c = match t.override_text_color {
                        Some(c) => c,
                        None if v.color == egui::Color32::PLACEHOLDER => t.fallback_color,
                        None => v.color,
                    };
                    acc.push(if t.opacity_factor < 1.0 {
                        c.gamma_multiply(t.opacity_factor)
                    } else {
                        c
                    });
                }
            }
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, acc)),
            _ => {}
        }
    }
    let mut acc = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut acc));
    acc
}

/// C2: a focused trigger in a line list draws no ring, outline or focus
/// border. A focused inactive trigger shows its label in `Foreground` and
/// its underline in `Ring`, 2 px; C2 adds no focus marker to the active
/// trigger: it keeps upstream's 2 px `Foreground` underline and no `Ring`.
#[test]
fn focused_line_trigger_shows_its_underline() {
    for (name, theme) in common::snapshot_themes() {
        for focused in [0, 1] {
            let (_, out) = common::measure_output(&theme, |ui| {
                let resp = ui.add(Tabs::new(&mut 0, &["A", "B"]).variant(TabsVariant::Line));
                ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", focused))));
                resp
            });
            let shapes = common::rect_shapes(&out);
            let (ring, fg) = (theme.palette.ring, theme.palette.foreground);
            assert!(
                !shapes
                    .iter()
                    .any(|r| r.stroke_kind == egui::StrokeKind::Outside
                        && r.stroke.width > 0.0
                        && r.stroke.color.a() > 0),
                "{name} tab {focused}: no ring or outline: {shapes:?}"
            );
            assert!(
                !shapes
                    .iter()
                    .any(|r| r.stroke.width > 0.0 && r.stroke.color == ring),
                "{name} tab {focused}: no Ring border: {shapes:?}"
            );
            let bars = |c: egui::Color32| -> Vec<f32> {
                shapes
                    .iter()
                    .filter(|r| r.stroke.width == 0.0 && r.fill == c)
                    .map(|r| r.rect.height())
                    .collect()
            };
            if focused == 0 {
                assert_eq!(bars(ring), Vec::<f32>::new(), "{name}: no Ring underline");
                assert_eq!(bars(fg), vec![2.0], "{name}: 2px Foreground underline");
            } else {
                assert_eq!(bars(ring), vec![2.0], "{name}: 2px Ring underline");
                assert_eq!(
                    bars(fg),
                    vec![2.0],
                    "{name}: active 2px Foreground underline"
                );
                assert_eq!(
                    text_colours(&out)[1],
                    fg,
                    "{name}: focused inactive label in Foreground"
                );
            }
        }
    }
}

/// The tooltip's fill, text colour, arrow fill and max width all come from
/// the style: an app restyles `tooltip_content` and `tooltip_arrow`.
#[test]
fn tooltip_colours_come_from_the_style() {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::button::{Button, ButtonVariant};
    use shadcn_egui::components::tooltip::tooltip;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.tooltip_content.bg.base = Some(Paint::Role(Role::Primary, 1.0));
        s.tooltip_content.text.base = Some(Paint::Role(Role::Destructive, 1.0));
        s.tooltip_content.max_width.base = Some(80.0);
        s.tooltip_arrow.bg.base = Some(Paint::Role(Role::Ring, 1.0));
    });
    let p = &theme.palette;
    let (primary, destructive, ring) = (p.primary, p.destructive, p.ring);
    let mut h = egui_kittest::Harness::builder()
        .with_size(vec2(240.0, 160.0))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            ui.style_mut().interaction.tooltip_delay = 0.0;
            ui.add_space(72.0);
            ui.horizontal(|ui| {
                ui.add_space(32.0);
                tooltip(
                    ui.add(Button::new("Hover me").variant(ButtonVariant::Outline)),
                    "Add to library",
                );
            });
        });
    h.run();
    let trigger = h.get_by_label("Hover me").rect();
    h.get_by_label("Hover me").hover();
    h.run();
    let text = h.get_by_label("Add to library").rect();
    assert!(
        text_colours(h.output()).contains(&destructive),
        "text: {:?}",
        text_colours(h.output())
    );
    // `max-w` 80 less 12 px padding on each side leaves 56 px: the label wraps.
    assert!(text.width() <= 56.0, "max width: {text:?}");
    assert!(text.height() > 24.0, "wraps: {text:?}");
    let img = h.render().expect("render");
    let px = |p: egui::Pos2| {
        let c = img.get_pixel(p.x as u32, p.y as u32);
        egui::Color32::from_rgb(c[0], c[1], c[2])
    };
    assert_eq!(
        px(egui::pos2(text.left() - 4.0, text.center().y)),
        primary,
        "content"
    );
    assert_eq!(
        px(egui::pos2(text.center().x, trigger.top() - 2.0)),
        ring,
        "arrow"
    );
}

#[test]
fn separator_line_comes_from_the_style() {
    use shadcn_egui::components::separator::{separator, separator_vertical};
    use shadcn_egui::style::Prop;
    let line = |theme: &Theme| {
        let (_, out) = common::measure_output(theme, |ui| {
            ui.allocate_response(vec2(200.0, 0.0), egui::Sense::hover());
            separator(ui);
            ui.allocate_response(vec2(0.0, 0.0), egui::Sense::hover())
        });
        common::rect_shapes(&out)
            .into_iter()
            .find(|r| r.fill == theme.palette.primary)
            .map(|r| r.rect.height())
    };
    let thick = common::restyled(Theme::vega_light(), |s| {
        s.separator.bg.base = Some(Paint::Role(Role::Primary, 1.0));
        s.separator
            .height
            .rules
            .push(Rule::important(When::all(State::HORIZONTAL), 3.0));
    });
    assert_eq!(line(&thick), Some(3.0));
    // Review Focus 4: without a thickness rule, `metrics.border`.
    let mut fallback = common::restyled(Theme::vega_light(), |s| {
        s.separator.bg.base = Some(Paint::Role(Role::Primary, 1.0));
        s.separator.height = Prop::default();
    });
    fallback.metrics.border = 2.0;
    assert_eq!(line(&fallback), Some(2.0));
    // The vertical line reads `width` under `data-vertical`, not `height`.
    let vertical = common::restyled(thick, |s| {
        s.separator
            .width
            .rules
            .push(Rule::important(When::all(State::VERTICAL), 4.0));
    });
    let (_, out) = common::measure_output(&vertical, |ui| {
        ui.horizontal(|ui| {
            ui.allocate_response(vec2(0.0, 40.0), egui::Sense::hover());
            separator_vertical(ui);
        })
        .response
    });
    let bar = common::rect_shapes(&out)
        .into_iter()
        .find(|r| r.fill == vertical.palette.primary)
        .expect("vertical line");
    assert_eq!(bar.rect.width(), 4.0);
}

#[test]
fn button_disabled_opacity_comes_from_the_style() {
    use shadcn_egui::components::button::Button;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.button
            .opacity
            .rules
            .push(Rule::important(When::all(State::DISABLED), 0.25));
    });
    let (_, on) = common::measure_output(&theme, |ui| ui.add(Button::new("Save")));
    let (_, off) = common::measure_output(&theme, |ui| ui.add(Button::new("Save").enabled(false)));
    assert_eq!(
        common::text_colour(&off),
        common::text_colour(&on).gamma_multiply(0.25)
    );
}

/// The underline width of the first text a frame painted.
fn underline_width(out: &egui::FullOutput) -> f32 {
    fn walk(s: &egui::Shape) -> Option<f32> {
        match s {
            egui::Shape::Text(t) => Some(t.galley.job.sections[0].format.underline.width),
            egui::Shape::Vec(v) => v.iter().find_map(walk),
            _ => None,
        }
    }
    out.shapes
        .iter()
        .find_map(|c| walk(&c.shape))
        .expect("a text shape")
}

#[test]
fn link_underline_is_metrics_border_wide() {
    use shadcn_egui::components::badge::{Badge, BadgeVariant};
    use shadcn_egui::components::button::{Button, ButtonVariant};
    // Upstream underlines on hover; an always-on underline needs no pointer.
    let mut theme = common::restyled(Theme::vega_light(), |s| {
        s.button_variant_link.underline.base = Some(true);
        s.badge_variant_link.underline.base = Some(true);
    });
    theme.metrics.border = 3.0;
    let (_, out) = common::measure_output(&theme, |ui| {
        ui.add(Button::new("Save").variant(ButtonVariant::Link))
    });
    assert_eq!(underline_width(&out), 3.0, "button");
    let (_, out) = common::measure_output(&theme, |ui| {
        ui.add(Badge::new("New").variant(BadgeVariant::Link))
    });
    assert_eq!(underline_width(&out), 3.0, "badge");
}
