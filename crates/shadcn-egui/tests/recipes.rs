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

/// An app's important border rule wins on the trigger. (Upstream luma's
/// `border-transparent!` was important too; correction C1 removes it.)
#[test]
fn important_app_rule_beats_upstream_important() {
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

/// C2: a focused trigger in a line list draws no ring, outline or focus
/// border; its underline shows in `Ring`, whether it is active or not.
#[test]
fn focused_line_trigger_shows_its_underline_in_ring() {
    for (name, theme) in common::snapshot_themes() {
        for focused in [0, 1] {
            let (_, out) = common::measure_output(&theme, |ui| {
                let resp = ui.add(Tabs::new(&mut 0, &["A", "B"]).variant(TabsVariant::Line));
                ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", focused))));
                resp
            });
            let shapes = common::rect_shapes(&out);
            let ring = theme.palette.ring;
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
            let bars: Vec<_> = shapes
                .iter()
                .filter(|r| r.stroke.width == 0.0 && r.fill == ring)
                .collect();
            assert_eq!(
                bars.len(),
                1,
                "{name} tab {focused}: one Ring underline: {shapes:?}"
            );
            assert_eq!(bars[0].rect.height(), 2.0, "{name} tab {focused}");
        }
    }
}
