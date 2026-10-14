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

#[test]
fn input_placeholder_and_fade_come_from_the_style() {
    use shadcn_egui::components::input::Input;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.input.text.rules.push(Rule::important(
            When::all(State::PLACEHOLDER),
            Paint::Role(Role::Primary, 1.0),
        ));
        s.input
            .opacity
            .rules
            .push(Rule::important(When::all(State::DISABLED), 0.25));
    });
    let (_, on) = common::measure_output(&theme, |ui| {
        ui.add(Input::new(&mut String::new()).hint("Email"))
    });
    assert_eq!(common::text_colour(&on), theme.palette.primary);
    let (_, off) = common::measure_output(&theme, |ui| {
        ui.add(Input::new(&mut String::new()).hint("Email").enabled(false))
    });
    assert_eq!(
        common::text_colour(&off),
        theme.palette.primary.gamma_multiply(0.25)
    );
    // The box fades with the text: vega's border has no disabled rule.
    let border = |out: &egui::FullOutput| {
        common::rect_shapes(out)
            .into_iter()
            .find(|r| r.stroke.width > 0.0)
            .expect("input box")
            .stroke
            .color
    };
    assert_eq!(border(&off), border(&on).gamma_multiply(0.25));
}

#[test]
fn select_fade_and_offset() {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::select::Select;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.select_trigger
            .opacity
            .rules
            .push(Rule::important(When::all(State::DISABLED), 0.25));
    });
    let (_, on) = common::measure_output(&theme, |ui| ui.add(Select::new("s", &mut 0, &["Apple"])));
    let (_, off) = common::measure_output(&theme, |ui| {
        ui.add(Select::new("s", &mut 0, &["Apple"]).enabled(false))
    });
    assert_eq!(
        common::text_colour(&off),
        common::text_colour(&on).gamma_multiply(0.25)
    );
    // The box fades with the text: vega's border has no disabled rule.
    let border = |out: &egui::FullOutput| {
        common::rect_shapes(out)
            .into_iter()
            .find(|r| r.stroke.width > 0.0)
            .expect("select trigger")
            .stroke
            .color
    };
    assert_eq!(border(&off), border(&on).gamma_multiply(0.25));

    let item_top = |offset: f32| {
        let mut h = egui_kittest::Harness::builder()
            .with_size(vec2(320.0, 260.0))
            .build_ui(move |ui| {
                Theme::vega_light().apply(ui.ctx());
                let mut none = usize::MAX;
                ui.add(
                    Select::new("fruit", &mut none, &["Apple", "Banana"])
                        .placeholder("Pick")
                        .side_offset(offset),
                );
            });
        h.run();
        h.get_by_role(egui::accesskit::Role::ComboBox).click();
        h.run();
        h.get_by_label("Banana").rect().top()
    };
    assert_eq!(item_top(20.0) - item_top(4.0), 16.0);
}

/// Press and release the primary button at `pos` over three frames; true
/// if `add` flipped the value.
fn click_flips(
    theme: &Theme,
    pos: egui::Pos2,
    mut add: impl FnMut(&mut egui::Ui, &mut bool),
) -> bool {
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    let mut value = false;
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    for events in [
        vec![],
        vec![egui::Event::PointerMoved(pos), press(true)],
        vec![press(false)],
    ] {
        let _ = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| add(ui, &mut value),
        );
    }
    value
}

#[test]
fn hit_area_comes_from_the_style() {
    use shadcn_egui::components::checkbox::Checkbox;
    use shadcn_egui::components::switch::Switch;
    type Add = fn(&mut egui::Ui, &mut bool) -> egui::Response;
    // The space above keeps the probe above the control on the screen.
    let widgets: [(&str, Add); 2] = [
        ("checkbox", |ui, v| {
            ui.add_space(40.0);
            ui.add(Checkbox::new(v))
        }),
        ("switch", |ui, v| {
            ui.add_space(40.0);
            ui.add(Switch::new(v))
        }),
    ];
    // Each inset gets its own value, so reading the other one fails.
    let insets = |x: f32, y: f32| {
        common::restyled(Theme::vega_light(), |s| {
            for r in [&mut s.checkbox, &mut s.switch] {
                r.hit_inset_x.rules.push(Rule::important(When::ALWAYS, x));
                r.hit_inset_y.rules.push(Rule::important(When::ALWAYS, y));
            }
        })
    };
    for (name, add) in widgets {
        let theme = Theme::vega_light();
        let rect = common::measure(&theme, |ui| add(ui, &mut false));
        // 7 px out: inside the default 12 by 8 px reach, outside egui's
        // 5 px pointer tolerance.
        let beside = egui::pos2(rect.right() + 7.0, rect.center().y);
        let above = egui::pos2(rect.center().x, rect.top() - 7.0);
        let flips = |theme: &Theme, pos| click_flips(theme, pos, |ui, v| _ = add(ui, v));
        assert!(flips(&theme, beside), "{name}: default reach beside");
        assert!(flips(&theme, above), "{name}: default reach above");
        let narrow = insets(0.0, 20.0);
        assert!(!flips(&narrow, beside), "{name}: no reach beside");
        assert!(flips(&narrow, above), "{name}: reach above");
        let flat = insets(20.0, 0.0);
        assert!(flips(&flat, beside), "{name}: reach beside");
        assert!(!flips(&flat, above), "{name}: no reach above");
    }
}

#[test]
fn switch_fade_comes_from_the_style() {
    use shadcn_egui::components::switch::Switch;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.switch
            .opacity
            .rules
            .push(Rule::important(When::all(State::DISABLED), 0.25));
    });
    let fills = |enabled: bool| {
        let (_, out) =
            common::measure_output(&theme, |ui| ui.add(Switch::new(&mut true).enabled(enabled)));
        let rects = common::rect_shapes(&out);
        (rects[0].fill, rects.last().expect("thumb").fill)
    };
    let ((track_on, thumb_on), (track_off, thumb_off)) = (fills(true), fills(false));
    assert_eq!(track_off, track_on.gamma_multiply(0.25), "track");
    // The thumb fades with its track: opacity multiplies down the parts.
    assert_eq!(thumb_off, thumb_on.gamma_multiply(0.25), "thumb");
}

#[test]
fn checkbox_fade_comes_from_the_style() {
    use shadcn_egui::components::checkbox::Checkbox;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.checkbox
            .opacity
            .rules
            .push(Rule::important(When::all(State::DISABLED), 0.25));
    });
    let fill = |enabled: bool| {
        let (_, out) = common::measure_output(&theme, |ui| {
            ui.add(Checkbox::new(&mut true).enabled(enabled))
        });
        common::rect_shapes(&out)
            .into_iter()
            .find(|r| r.fill != egui::Color32::TRANSPARENT)
            .expect("checked box")
            .fill
    };
    assert_eq!(fill(false), fill(true).gamma_multiply(0.25));
}

#[test]
fn slider_fade_multiplies_down_the_parts() {
    use shadcn_egui::components::slider::Slider;
    // Each part its own factor, so a part that reads another's opacity
    // (or skips its parent's) lands on the wrong product.
    let (root, track, range, thumb) = (0.25, 0.5, 0.75, 0.625);
    let theme = common::restyled(Theme::vega_light(), |s| {
        for (recipe, v) in [
            (&mut s.slider, root),
            (&mut s.slider_track, track),
            (&mut s.slider_range, range),
            (&mut s.slider_thumb, thumb),
        ] {
            recipe
                .opacity
                .rules
                .push(Rule::important(When::all(State::DISABLED), v));
        }
    });
    let rects = |enabled: bool| {
        let (_, out) = common::measure_output(&theme, |ui| {
            ui.add(Slider::new(&mut 50.0, 0.0..=100.0).enabled(enabled))
        });
        common::rect_shapes(&out)
    };
    let (on, off) = (rects(true), rects(false));
    assert_eq!(on.len(), off.len());
    assert!(on.len() >= 3, "track, range and thumb: {}", on.len());
    // Root > track > range; root > thumb.
    let factors = [track * root, range * track * root];
    for (i, (a, b)) in on.iter().zip(&off).enumerate() {
        let (part, f) = match i {
            0 => ("track", factors[0]),
            1 => ("range", factors[1]),
            _ => ("thumb", thumb * root),
        };
        assert_eq!(b.fill, a.fill.gamma_multiply(f), "{part} fill");
        assert_eq!(
            b.stroke.color,
            a.stroke.color.gamma_multiply(f),
            "{part} stroke"
        );
    }
}

fn popover_probe(theme: Theme, offset: Option<f32>) -> (egui::Rect, f32) {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::button::Button;
    use shadcn_egui::components::popover::Popover;
    use std::{cell::Cell, rc::Rc};
    let width = Rc::new(Cell::new(0.0));
    let w = width.clone();
    let mut h = egui_kittest::Harness::builder()
        .with_size(vec2(520.0, 320.0))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            let resp = ui.add(Button::new("Open"));
            let mut p = Popover::new(&resp);
            if let Some(o) = offset {
                p = p.side_offset(o);
            }
            p.show(|ui| {
                w.set(ui.available_width());
                ui.label("Inside");
            });
        });
    h.run();
    h.get_by_label("Open").click();
    h.run();
    (h.get_by_label("Inside").rect(), width.get())
}

#[test]
fn popover_width_and_offset() {
    let wide = common::restyled(Theme::vega_light(), |s| {
        s.popover_content.width.base = Some(400.0);
    });
    let (near, w288) = popover_probe(Theme::vega_light(), None);
    let (_, w400) = popover_probe(wide, None);
    assert_eq!(w400 - w288, 112.0);
    let (far, _) = popover_probe(Theme::vega_light(), Some(20.0));
    assert_eq!(far.top() - near.top(), 16.0);
}

/// Space between the two buttons of a dialog footer under `theme`.
fn footer_gap(theme: Theme) -> f32 {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::button::Button;
    use shadcn_egui::components::dialog::{self, Dialog};
    let mut h = egui_kittest::Harness::builder()
        .with_size(vec2(640.0, 360.0))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            Dialog::new("gap", "Title").show(ui.ctx(), &mut true, |ui| {
                dialog::footer(ui, |ui| {
                    ui.add(Button::new("Right"));
                    ui.add(Button::new("Left"));
                });
            });
        });
    h.run();
    h.run();
    h.get_by_label("Right").rect().left() - h.get_by_label("Left").rect().right()
}

#[test]
fn dialog_footer_gap_comes_from_the_style() {
    let wide = common::restyled(Theme::vega_light(), |s| {
        s.dialog_footer.gap.base = Some(20.0);
    });
    assert_eq!(footer_gap(wide), 20.0);
    // No code fallback: a cleared gap is no gap.
    let none = common::restyled(Theme::vega_light(), |s| {
        s.dialog_footer.gap = shadcn_egui::style::Prop::default();
    });
    assert_eq!(footer_gap(none), 0.0);
}

/// `cn-dialog-close` sits on the close button's own class list, after the
/// button's recipes: an app value set on it reaches the button's paint.
#[test]
fn dialog_close_recipe_paints_the_close_button() {
    use shadcn_egui::components::dialog::Dialog;
    // Both layers set a base fill: the later `dialog_close` must win.
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.button_variant_ghost.bg.base = Some(Paint::Role(Role::Primary, 1.0));
        s.dialog_close.bg.base = Some(Paint::Role(Role::Destructive, 1.0));
    });
    let red = theme.role(Role::Destructive);
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    let frame = |ui: &mut egui::Ui| {
        Dialog::new("close", "Title").show(ui.ctx(), &mut true, |ui| ui.label("Body"));
    };
    // The modal fades in: paint until the fade is over.
    let mut out = egui::FullOutput::default();
    for time in [0.0, 1.0, 2.0] {
        let input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        out = ctx.run_ui(input, frame);
    }
    let filled: Vec<_> = common::rect_shapes(&out)
        .into_iter()
        .filter(|r| r.fill == red)
        .collect();
    assert_eq!(filled.len(), 1, "one close button in the app's colour");
    assert!(
        filled[0].rect.width() < 40.0,
        "the close button, not the panel"
    );
}

/// Kind 2: the label inherits `Foreground`, but reads `label.text` first.
#[test]
fn label_text_colour_comes_from_the_style() {
    use shadcn_egui::components::label::Label;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.label.text.base = Some(Paint::Role(Role::Primary, 1.0));
    });
    let (_, out) = common::measure_output(&theme, |ui| ui.add(Label::new("Email")));
    assert_eq!(common::text_colour(&out), theme.palette.primary);
}

/// Kind 2: typed text inherits `Foreground`, but reads `input.text` first.
#[test]
fn input_text_colour_comes_from_the_style() {
    use shadcn_egui::components::input::Input;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.input.text.base = Some(Paint::Role(Role::Primary, 1.0));
    });
    let (_, out) = common::measure_output(&theme, |ui| ui.add(Input::new(&mut "typed".to_owned())));
    assert_eq!(common::text_colour(&out), theme.palette.primary);
}

/// The tick inherits the box's text colour, but reads
/// `checkbox_indicator.text` first.
#[test]
fn checkbox_tick_colour_comes_from_the_indicator() {
    use shadcn_egui::components::checkbox::Checkbox;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.checkbox_indicator.text.base = Some(Paint::Role(Role::Destructive, 1.0));
    });
    let (_, out) = common::measure_output(&theme, |ui| ui.add(Checkbox::new(&mut true)));
    let tick = out
        .shapes
        .iter()
        .find_map(|c| match &c.shape {
            egui::Shape::Mesh(m) if m.texture_id != egui::TextureId::default() => {
                m.vertices.first().map(|v| v.color)
            }
            _ => None,
        })
        .expect("a tick");
    assert_eq!(tick, theme.palette.destructive);
}
