//! Accessible roles egui's `WidgetType` lacks (conformance 8,
//! implementation notes 16), and states egui's `WidgetInfo` lacks
//! (`aria-invalid`).

use egui::accesskit::{Invalid, Role};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::components::button::Button;
use shadcn_egui::components::checkbox::Checkbox;
use shadcn_egui::components::input::Input;
use shadcn_egui::components::select::Select;
use shadcn_egui::components::shared::widget_info_with_role;
use shadcn_egui::components::switch::Switch;
use shadcn_egui::components::textarea::Textarea;
use shadcn_egui::Theme;

/// The role replaces the one `widget_info` sets from the `WidgetType`, on
/// a quiet pass and on a pass that reports an event (a click), while the
/// name stays the widget info's.
#[test]
fn widget_info_with_role_overrides_the_widget_type_role() {
    for (role, name) in [
        (Role::Alert, "Saved"),
        (Role::AlertDialog, "Delete file?"),
        (Role::Navigation, "pagination"),
        (Role::Group, "Text alignment"),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            let resp = ui.allocate_response(egui::vec2(120.0, 30.0), egui::Sense::click());
            widget_info_with_role(&resp, role, || {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name)
            });
        });
        harness.run();
        assert_eq!(
            harness
                .get_by_role(role)
                .accesskit_node()
                .label()
                .as_deref(),
            Some(name)
        );
        assert!(harness.query_by_role(Role::Button).is_none(), "{role:?}");
        harness.get_by_role(role).click();
        harness.run();
        assert_eq!(
            harness
                .get_by_role(role)
                .accesskit_node()
                .label()
                .as_deref(),
            Some(name)
        );
    }
}

/// One widget of [`aria_invalid_reaches_the_accessibility_node`], drawn
/// with `.invalid(..)`.
type Invalidable = fn(&mut egui::Ui, bool);

/// Upstream's `aria-invalid` is an attribute the browser reports to
/// assistive technology: every widget with an `.invalid(..)` setter says
/// so on its node, and only while it is set.
#[test]
fn aria_invalid_reaches_the_accessibility_node() {
    let widgets: [(&str, Role, Invalidable); 6] = [
        ("button", Role::Button, |ui, yes| {
            ui.add(Button::new("Save").invalid(yes));
        }),
        ("checkbox", Role::CheckBox, |ui, yes| {
            ui.add(Checkbox::new(&mut false).invalid(yes));
        }),
        ("switch", Role::CheckBox, |ui, yes| {
            ui.add(Switch::new(&mut false).invalid(yes));
        }),
        ("select", Role::ComboBox, |ui, yes| {
            ui.add(Select::new("fruit", &mut 0, &["Apple", "Pear"]).invalid(yes));
        }),
        ("input", Role::TextInput, |ui, yes| {
            ui.add(Input::new(&mut String::new()).invalid(yes));
        }),
        ("textarea", Role::MultilineTextInput, |ui, yes| {
            ui.add(Textarea::new(&mut String::new()).invalid(yes));
        }),
    ];
    for (name, role, widget) in widgets {
        for invalid in [false, true] {
            let mut harness = Harness::new_ui(move |ui| {
                Theme::vega().apply(ui.ctx());
                widget(ui, invalid);
            });
            harness.run();
            let node = harness.get_by_role(role);
            assert_eq!(
                node.accesskit_node().data().invalid(),
                invalid.then_some(Invalid::True),
                "{name}, invalid {invalid}"
            );
        }
    }
}

/// A link badge reports the link role (an `<a>`), a plain badge none of
/// it.
#[test]
fn a_link_badge_reports_the_link_role() {
    use shadcn_egui::components::badge::Badge;
    for link in [true, false] {
        let mut harness = Harness::new_ui(move |ui| {
            Theme::vega().apply(ui.ctx());
            ui.add(Badge::new("Docs").link(link));
        });
        harness.run();
        assert_eq!(harness.query_by_role(Role::Link).is_some(), link, "{link}");
    }
}

/// How [`a_disabled_ui_disables_every_leaf`] draws a leaf.
#[derive(Clone, Copy, Debug, PartialEq)]
enum How {
    /// Not disabled: the control that shows the leaf takes the input.
    Enabled,
    /// `ui.add_enabled(false, leaf)`.
    AddEnabled,
    /// `ui.disable()` before `ui.add(leaf)`.
    UiDisable,
}

/// What the leaves of [`a_disabled_ui_disables_every_leaf`] change.
#[derive(Default)]
struct Leaves {
    clicked: bool,
    checked: bool,
    on: bool,
    index: usize,
    open: bool,
    value: f32,
    tab: usize,
    text: String,
}

/// One leaf of [`a_disabled_ui_disables_every_leaf`]: its name, the label
/// or role of the node a click goes to, how it is drawn, and whether the
/// click (and the typed text after it) changed anything.
type Leaf = (
    &'static str,
    Result<&'static str, Role>,
    fn(&mut egui::Ui, &mut Leaves, How),
    Option<fn(&Leaves) -> bool>,
);

fn put(ui: &mut egui::Ui, how: How, leaf: impl egui::Widget) -> egui::Response {
    match how {
        How::AddEnabled => ui.add_enabled(false, leaf),
        How::Enabled | How::UiDisable => ui.add(leaf),
    }
}

/// D-1: a leaf has no `enabled` flag of its own; the `Ui` disables it, by
/// `add_enabled` or `Ui::disable`, as an egui leaf. Every control in a
/// disabled `Ui` reports the disabled state to assistive technology and
/// ignores a click and typed text; the same control in an enabled `Ui`
/// takes them. A label is no control: upstream's `<label>` has no disabled
/// state, so it reports none ([`a_disabled_ui_disables_no_part_without_a_disabled_state`]).
#[test]
fn a_disabled_ui_disables_every_leaf() {
    use shadcn_egui::components::label::Label;
    use shadcn_egui::components::slider::Slider;
    use shadcn_egui::components::tabs::Tabs;
    let leaves: [Leaf; 8] = [
        (
            "button",
            Ok("Save"),
            |ui, s, how| s.clicked |= put(ui, how, Button::new("Save")).clicked(),
            Some(|s| s.clicked),
        ),
        (
            "checkbox",
            Err(Role::CheckBox),
            |ui, s, how| {
                put(ui, how, Checkbox::new(&mut s.checked));
            },
            Some(|s| s.checked),
        ),
        (
            "switch",
            Err(Role::CheckBox),
            |ui, s, how| {
                put(ui, how, Switch::new(&mut s.on));
            },
            Some(|s| s.on),
        ),
        (
            "select",
            Err(Role::ComboBox),
            |ui, s, how| {
                put(
                    ui,
                    how,
                    Select::new("fruit", &mut s.index, &["Apple", "Pear"]),
                );
                s.open |= egui::Popup::is_any_open(ui.ctx());
            },
            Some(|s| s.open),
        ),
        (
            "input",
            Err(Role::TextInput),
            |ui, s, how| {
                put(ui, how, Input::new(&mut s.text));
            },
            Some(|s| s.text == "x"),
        ),
        (
            "slider",
            Err(Role::Slider),
            |ui, s, how| {
                put(ui, how, Slider::new(&mut s.value, 0.0..=100.0));
            },
            Some(|s| s.value != 0.0),
        ),
        (
            "tabs",
            Ok("B"),
            |ui, s, how| {
                put(ui, how, Tabs::new(&mut s.tab, &["A", "B"]));
            },
            Some(|s| s.tab == 1),
        ),
        // A label takes no input and reports no disabled state; it only
        // looks disabled.
        (
            "label",
            Ok("Email"),
            |ui, _, how| {
                put(ui, how, Label::new("Email"));
            },
            None,
        ),
    ];
    for (name, node, add, changed) in leaves {
        for how in [How::Enabled, How::AddEnabled, How::UiDisable] {
            let mut harness = Harness::new_ui_state(
                move |ui, s: &mut Leaves| {
                    Theme::vega().apply(ui.ctx());
                    if how == How::UiDisable {
                        ui.disable();
                    }
                    add(ui, s, how);
                },
                Leaves::default(),
            );
            harness.set_size(egui::vec2(300.0, 200.0));
            harness.run();
            let target = || match node {
                Ok(label) => harness.get_by_label(label),
                Err(role) => harness.get_by_role(role),
            };
            // Only a leaf that takes input is a control.
            let disabled = how != How::Enabled && changed.is_some();
            assert_eq!(
                target().accesskit_node().is_disabled(),
                disabled,
                "{name}, {how:?}: AccessKit disabled state"
            );
            let Some(changed) = changed else { continue };
            target().click();
            harness.run();
            harness.event(egui::Event::Text("x".to_owned()));
            harness.run();
            assert_eq!(
                changed(harness.state()),
                !disabled,
                "{name}, {how:?}: a click changed the leaf"
            );
        }
    }
}

/// An app widget that reports `role` through `widget_info_with_role`.
fn app_widget_with_role(ui: &mut egui::Ui, role: Role) {
    let resp = ui.allocate_response(egui::vec2(120.0, 30.0), egui::Sense::hover());
    widget_info_with_role(&resp, role, || {
        egui::WidgetInfo::labeled(egui::WidgetType::Other, ui.is_enabled(), "App widget")
    });
}

/// Upstream draws these parts as a `<label>`, `<div>`, `<span>`, `<kbd>`,
/// `<svg>`, `<legend>` or with `role="status"`, `progressbar`, `alert` or
/// `list`: HTML-AAM and ARIA 1.2 give none of them a disabled state, so
/// none reports one in a disabled `Ui` (conformance 8), although egui
/// marks every widget there disabled. The controls among them, and a
/// FieldSet (`<fieldset disabled>`), still report it.
#[test]
fn a_disabled_ui_disables_no_part_without_a_disabled_state() {
    use shadcn_egui::components::alert::{alert_title, Alert};
    use shadcn_egui::components::avatar::{avatar_group_count, Avatar};
    use shadcn_egui::components::badge::Badge;
    use shadcn_egui::components::card::{card_header, Card};
    use shadcn_egui::components::field::{Field, FieldLabel, FieldSet};
    use shadcn_egui::components::icon::IconView;
    use shadcn_egui::components::item::{Item, ItemGroup};
    use shadcn_egui::components::kbd::Kbd;
    use shadcn_egui::components::label::Label;
    use shadcn_egui::components::progress::Progress;
    use shadcn_egui::components::scroll_area::ScrollArea;
    use shadcn_egui::components::spinner::Spinner;
    type Add = fn(&mut egui::Ui);
    // Each case: what it draws, the roles that must be there and report
    // no disabled state, and the roles that must report it.
    let cases: [(&str, Add, &[Role], &[Role]); 21] = [
        (
            "label",
            |ui| drop(ui.add(Label::new("Email"))),
            &[Role::Label],
            &[],
        ),
        (
            "kbd",
            |ui| drop(ui.add(Kbd::new("Ctrl"))),
            &[Role::Label],
            &[],
        ),
        (
            "card",
            |ui| Card::new().show(ui, |ui| card_header(ui, "Title", "Description")),
            &[Role::Pane, Role::Label],
            &[],
        ),
        (
            "badge",
            |ui| drop(ui.add(Badge::new("New"))),
            &[Role::Label],
            &[],
        ),
        (
            "alert",
            |ui| drop(Alert::new().show(ui, |ui| alert_title(ui, "Saved"))),
            &[Role::Alert, Role::Label],
            &[],
        ),
        (
            "progress",
            |ui| drop(ui.add(Progress::new(0.4))),
            &[Role::ProgressIndicator],
            &[],
        ),
        (
            "icon",
            |ui| drop(ui.add(IconView::new(&shadcn_egui::icons::lucide::INBOX))),
            &[Role::Image],
            &[],
        ),
        (
            "spinner",
            |ui| drop(ui.add(Spinner::new())),
            &[Role::Status],
            &[],
        ),
        (
            "avatar",
            |ui| {
                ui.add(Avatar::new("CN"));
                avatar_group_count(ui, "+3");
            },
            &[Role::Label],
            &[],
        ),
        (
            "item",
            |ui| {
                ItemGroup::new().show(ui, |ui| {
                    Item::new().title("Inbox").show(ui, |_| ());
                });
            },
            &[Role::List, Role::Unknown],
            &[],
        ),
        (
            "item image",
            |ui| {
                drop(
                    Item::new()
                        .image("bytes://none.png")
                        .title("Cover")
                        .show(ui, |_| ()),
                )
            },
            &[Role::Image],
            &[],
        ),
        (
            "link item",
            |ui| drop(Item::new().title("Open").link(true).show(ui, |_| ())),
            &[],
            &[Role::Link],
        ),
        (
            "button item",
            |ui| {
                drop(
                    Item::new()
                        .title("Open")
                        .sense(egui::Sense::click())
                        .show(ui, |_| ()),
                )
            },
            &[],
            &[Role::Button],
        ),
        (
            "link badge",
            |ui| drop(ui.add(Badge::new("Docs").link(true))),
            &[],
            &[Role::Link],
        ),
        (
            "scroll area",
            |ui| {
                ScrollArea::vertical().max_height(40.0).show(ui, |ui| {
                    for i in 0..20 {
                        ui.add(Label::new(format!("Row {i}")));
                    }
                });
            },
            &[Role::ScrollView, Role::Label],
            &[],
        ),
        (
            "field",
            |ui| {
                Field::new()
                    .label("Email")
                    .error("Required.")
                    .show(ui, |ui| ui.add(Input::new(&mut String::new())));
            },
            &[Role::Group, Role::Label, Role::Alert],
            &[Role::TextInput],
        ),
        (
            "choice card",
            |ui| {
                FieldLabel::choice_card().show(ui, |ui| {
                    Field::new()
                        .title("Share")
                        .show(ui, |ui| ui.add(Switch::new(&mut false)));
                });
            },
            &[Role::Label],
            &[Role::CheckBox],
        ),
        (
            "fieldset",
            |ui| drop(FieldSet::new().legend("Address").show(ui, |_| ())),
            &[Role::Legend],
            &[Role::Group],
        ),
        (
            "button",
            |ui| drop(ui.add(Button::new("Save"))),
            &[],
            &[Role::Button],
        ),
        // An app widget given a role through `widget_info_with_role`:
        // `alert` has no `aria-disabled` in ARIA 1.2, `group` has it.
        (
            "app alert",
            |ui| app_widget_with_role(ui, Role::Alert),
            &[Role::Alert],
            &[],
        ),
        (
            "app group",
            |ui| app_widget_with_role(ui, Role::Group),
            &[],
            &[Role::Group],
        ),
    ];
    for (name, add, quiet, disabled) in cases {
        let mut harness = Harness::new_ui(move |ui| {
            Theme::vega().apply(ui.ctx());
            ui.disable();
            add(ui);
        });
        harness.run();
        for (roles, want) in [(quiet, false), (disabled, true)] {
            for &role in roles {
                // egui's bare interaction nodes are `Unknown` without a
                // name; an item's node has its title.
                let named = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
                    n.role() == role && (role != Role::Unknown || n.label().is_some())
                };
                let nodes: Vec<_> = harness
                    .query_all(egui_kittest::kittest::By::new().predicate(named))
                    .collect();
                assert!(!nodes.is_empty(), "{name}: no {role:?} node");
                for node in nodes {
                    assert_eq!(
                        node.accesskit_node().is_disabled(),
                        want,
                        "{name}: {role:?} {:?} disabled state",
                        node.accesskit_node().label()
                    );
                }
            }
        }
    }
}
