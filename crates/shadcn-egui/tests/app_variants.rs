//! App variants reach the screen: for each of the 14 components, an app
//! variant that recolours one part paints that part in the app colour.

mod common;

use egui::Color32;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use shadcn_egui::components::badge::{Badge, BadgeVariant};
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::components::card::{Card, CardVariant};
use shadcn_egui::components::checkbox::{Checkbox, CheckboxVariant};
use shadcn_egui::components::dialog::{Dialog, DialogVariant};
use shadcn_egui::components::input::{Input, InputVariant};
use shadcn_egui::components::label::{Label, LabelVariant};
use shadcn_egui::components::popover::{Popover, PopoverVariant};
use shadcn_egui::components::select::{Select, SelectVariant};
use shadcn_egui::components::separator::{Separator, SeparatorVariant};
use shadcn_egui::components::slider::{Slider, SliderVariant};
use shadcn_egui::components::switch::{Switch, SwitchVariant};
use shadcn_egui::components::tabs::{Tabs, TabsVariant};
use shadcn_egui::components::tooltip::{Tooltip, TooltipVariant};
use shadcn_egui::style::{AppVariant, Paint, Role, State};
use shadcn_egui::Theme;

const BRAND: Role = Role::custom("brand");
/// A colour no palette role has, the same in both modes.
const BRAND_RGB: Color32 = Color32::from_rgb(0x12, 0xab, 0x34);
const P: Paint = Paint::Role(BRAND, 1.0);

const BUTTON: AppVariant<ButtonVariant> = ButtonVariant::app("brand");
const BADGE: AppVariant<BadgeVariant> = BadgeVariant::app("brand");
const TABS: AppVariant<TabsVariant> = TabsVariant::app("brand");
const SELECT: AppVariant<SelectVariant> = SelectVariant::app("brand");
const CARD: AppVariant<CardVariant> = CardVariant::app("brand");
const DIALOG: AppVariant<DialogVariant> = DialogVariant::app("brand");
const POPOVER: AppVariant<PopoverVariant> = PopoverVariant::app("brand");
const TOOLTIP: AppVariant<TooltipVariant> = TooltipVariant::app("brand");
const CHECKBOX: AppVariant<CheckboxVariant> = CheckboxVariant::app("brand");
const INPUT: AppVariant<InputVariant> = InputVariant::app("brand");
const LABEL: AppVariant<LabelVariant> = LabelVariant::app("brand");
const SEPARATOR: AppVariant<SeparatorVariant> = SeparatorVariant::app("brand");
const SLIDER: AppVariant<SliderVariant> = SliderVariant::app("brand");
const SWITCH: AppVariant<SwitchVariant> = SwitchVariant::app("brand");

/// One component: the theme defines its app variant, `ui` draws it, and
/// `prepare` opens it where it shows only on a pointer action.
struct Case {
    name: &'static str,
    theme: fn(Theme) -> Theme,
    ui: fn(&mut egui::Ui),
    prepare: fn(&mut Harness<'_>),
}

fn cases() -> Vec<Case> {
    vec![
        Case {
            name: "button",
            theme: |t| t.variant(BUTTON, ButtonVariant::Default, |r, _| r.button.bg.set(P)),
            ui: |ui| {
                ui.add(Button::new("Go").variant(BUTTON));
            },
            prepare: |_| {},
        },
        Case {
            name: "badge",
            theme: |t| t.variant(BADGE, BadgeVariant::Default, |r, _| r.badge.bg.set(P)),
            ui: |ui| {
                ui.add(Badge::new("New").variant(BADGE));
            },
            prepare: |_| {},
        },
        Case {
            name: "tabs",
            theme: |t| t.variant(TABS, TabsVariant::Default, |r, _| r.tabs_list.bg.set(P)),
            ui: |ui| {
                let mut active = 0;
                ui.add(Tabs::new(&mut active, &["One", "Two"]).variant(TABS));
            },
            prepare: |_| {},
        },
        Case {
            name: "select",
            theme: |t| {
                t.variant(SELECT, SelectVariant::Default, |r, _| {
                    r.select_trigger.bg.set(P)
                })
            },
            ui: |ui| {
                let mut selected = 0;
                ui.add(Select::new("s", &mut selected, &["Apple", "Pear"]).variant(SELECT));
            },
            prepare: |_| {},
        },
        Case {
            name: "card",
            theme: |t| t.variant(CARD, CardVariant::Default, |r, _| r.card.bg.set(P)),
            ui: |ui| {
                Card::new().variant(CARD).show(ui, |ui| ui.label("body"));
            },
            prepare: |_| {},
        },
        Case {
            name: "dialog",
            theme: |t| {
                t.variant(DIALOG, DialogVariant::Default, |r, _| {
                    r.dialog_content.bg.set(P)
                })
            },
            ui: |ui| {
                Dialog::new("d", "Title")
                    .variant(DIALOG)
                    .show(ui.ctx(), &mut true, |ui| ui.label("body"));
            },
            prepare: |_| {},
        },
        Case {
            name: "popover",
            theme: |t| {
                t.variant(POPOVER, PopoverVariant::Default, |r, _| {
                    r.popover_content.bg.set(P)
                })
            },
            ui: |ui| {
                let trigger = ui.add(Button::new("Open"));
                Popover::new(&trigger)
                    .variant(POPOVER)
                    .show(|ui| ui.label("body"));
            },
            prepare: |h| {
                h.get_by_label("Open").click();
                h.run();
            },
        },
        Case {
            name: "tooltip",
            theme: |t| {
                t.variant(TOOLTIP, TooltipVariant::Default, |r, _| {
                    r.tooltip_content.bg.set(P)
                })
            },
            ui: |ui| {
                ui.style_mut().interaction.tooltip_delay = 0.0;
                let trigger = ui.add(Button::new("Hover"));
                Tooltip::new(&trigger)
                    .variant(TOOLTIP)
                    .show(ui, |ui| ui.label("tip"));
            },
            prepare: |h| {
                h.get_by_label("Hover").hover();
                h.run();
            },
        },
        Case {
            name: "checkbox",
            theme: |t| {
                t.variant(CHECKBOX, CheckboxVariant::Default, |r, _| {
                    r.checkbox.bg.set(P)
                })
            },
            ui: |ui| {
                ui.add(Checkbox::new(&mut false).variant(CHECKBOX));
            },
            prepare: |_| {},
        },
        Case {
            name: "input",
            theme: |t| t.variant(INPUT, InputVariant::Default, |r, _| r.input.bg.set(P)),
            ui: |ui| {
                ui.add(Input::new(&mut String::from("text")).variant(INPUT));
            },
            prepare: |_| {},
        },
        Case {
            name: "label",
            theme: |t| t.variant(LABEL, LabelVariant::Default, |r, _| r.label.text.set(P)),
            ui: |ui| {
                ui.add(Label::new("Name").variant(LABEL));
            },
            prepare: |_| {},
        },
        Case {
            name: "separator",
            theme: |t| {
                t.variant(SEPARATOR, SeparatorVariant::Default, |r, _| {
                    r.separator.bg.set(P)
                })
            },
            ui: |ui| {
                ui.add(Separator::new().variant(SEPARATOR));
            },
            prepare: |_| {},
        },
        Case {
            name: "slider",
            theme: |t| {
                t.variant(SLIDER, SliderVariant::Default, |r, _| {
                    r.slider_range.bg.set(P)
                })
            },
            ui: |ui| {
                ui.add(Slider::new(&mut 50.0, 0.0..=100.0).variant(SLIDER));
            },
            prepare: |_| {},
        },
        Case {
            name: "switch",
            theme: |t| {
                t.variant(SWITCH, SwitchVariant::Default, |r, _| {
                    r.switch_thumb.bg.set(P)
                })
            },
            ui: |ui| {
                ui.add(Switch::new(&mut false).variant(SWITCH));
            },
            prepare: |_| {},
        },
    ]
}

/// Every colour a frame painted: fills, strokes, mesh vertices and text.
fn colours(out: &egui::FullOutput) -> Vec<Color32> {
    fn walk(s: &egui::Shape, out: &mut Vec<Color32>) {
        match s {
            egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            egui::Shape::Rect(r) => out.extend([r.fill, r.stroke.color]),
            egui::Shape::Circle(c) => out.extend([c.fill, c.stroke.color]),
            egui::Shape::LineSegment { stroke, .. } => out.push(stroke.color),
            egui::Shape::Path(p) => {
                out.push(p.fill);
                if let egui::epaint::ColorMode::Solid(c) = p.stroke.color {
                    out.push(c);
                }
            }
            egui::Shape::Mesh(m) => out.extend(m.vertices.iter().map(|v| v.color)),
            egui::Shape::Text(t) => {
                out.extend(t.override_text_color);
                out.push(t.fallback_color);
                for row in &t.galley.rows {
                    out.extend(row.visuals.mesh.vertices.iter().map(|v| v.color));
                }
            }
            _ => {}
        }
    }
    let mut colours = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut colours));
    colours
}

fn harness(theme: Theme, ui_fn: impl FnMut(&mut egui::Ui) + 'static) -> Harness<'static> {
    common::page_harness(egui::vec2(480.0, 360.0), theme, egui::Theme::Light, ui_fn)
}

/// Each component's app variant recolours its part on the screen. Without
/// the variant the app colour shows nowhere, so the colour can come only
/// from the variant.
#[test]
fn an_app_variant_reaches_the_painted_colour_of_every_component() {
    let cases = cases();
    assert_eq!(cases.len(), 14);
    let mut missing = Vec::new();
    for case in cases {
        let theme = (case.theme)(Theme::vega().color(BRAND, BRAND_RGB, BRAND_RGB));
        let mut h = harness(theme, case.ui);
        (case.prepare)(&mut h);
        if !colours(h.output()).contains(&BRAND_RGB) {
            missing.push(case.name);
        }
    }
    assert!(missing.is_empty(), "no app colour painted: {missing:?}");
}

/// An app badge variant from a base without hover rules takes its own
/// `on(State::HOVER, ..)`: the colour shows on hover and not at rest.
#[test]
fn an_app_badge_variant_takes_its_hover_rule() {
    let theme = Theme::vega().color(BRAND, BRAND_RGB, BRAND_RGB).variant(
        BADGE,
        BadgeVariant::Default,
        |r, _| r.badge.bg.on(State::HOVER, P),
    );
    let mut h = harness(theme, |ui| {
        ui.add(Badge::new("New").variant(BADGE));
    });
    assert!(!colours(h.output()).contains(&BRAND_RGB), "at rest");
    h.get_by_label("New").hover();
    h.run();
    assert!(colours(h.output()).contains(&BRAND_RGB), "on hover");
}
