//! Field: gaps, orientation, invalid and disabled flow, the label's press
//! and name, the error's content rules, the choice card, and the
//! shadcn-svelte demos as snapshots.

mod common;

use egui::accesskit::{Invalid, Live, Role};
use egui::{vec2, Color32, Rect, Ui};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::components::checkbox::Checkbox;
use shadcn_egui::components::field::{
    field_content, field_description, field_label, field_title, Field, FieldError, FieldGroup,
    FieldGroupVariant, FieldLabel, FieldLegend, FieldLegendVariant, FieldSeparator, FieldSet,
    FieldSetVariant, FieldVariant,
};
use shadcn_egui::components::input::Input;
use shadcn_egui::components::select::Select;
use shadcn_egui::components::slider::Slider;
use shadcn_egui::components::switch::Switch;
use shadcn_egui::components::textarea::Textarea;
use shadcn_egui::style::{AppVariant, Role as Palette, Style, Weight};
use shadcn_egui::{Palette as Pal, Theme};

fn vega() -> Theme {
    Theme::new(Style::vega(), Pal::light(), Pal::dark(), 10.0)
}

fn nova() -> Theme {
    Theme::new(Style::nova(), Pal::light(), Pal::dark(), 10.0)
}

/// Run frames until the layout settles; what the last one saw and painted.
fn settle<T>(
    theme: &Theme,
    width: f32,
    mut add: impl FnMut(&mut Ui) -> T,
) -> (T, egui::FullOutput) {
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|s| s.animation_time = 0.0);
    let input = || egui::RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(width, 800.0))),
        ..Default::default()
    };
    let mut last = None;
    for _ in 0..4 {
        let mut t = None;
        let out = ctx.run_ui(input(), |ui| t = Some(add(ui)));
        last = Some((t.expect("ran"), out));
    }
    last.expect("a frame ran")
}

/// The colour of the text shape that shows `text` (a painter's opacity
/// is in its alpha) and the shape's own opacity.
fn paint_of(out: &egui::FullOutput, text: &str) -> (Color32, f32) {
    let t = common::text_shapes(out)
        .into_iter()
        .find(|t| t.galley.text() == text)
        .unwrap_or_else(|| panic!("no text {text:?}"));
    let v = t
        .galley
        .rows
        .iter()
        .find_map(|r| r.visuals.mesh.vertices.first())
        .expect("glyphs");
    let c = match t.override_text_color {
        Some(c) => c,
        None if v.color == Color32::PLACEHOLDER => t.fallback_color,
        None => v.color,
    };
    (c, t.opacity_factor)
}

/// The opacity `text` paints at, to a hundredth.
fn faded(out: &egui::FullOutput, text: &str) -> f32 {
    let (c, o) = paint_of(out, text);
    (f32::from(c.a()) / 255.0 * o * 100.0).round() / 100.0
}

fn role(theme: &Theme, r: Palette) -> Color32 {
    let p = theme.palette_of(egui::Theme::Light);
    let t = common::resolved(theme, egui::Theme::Light);
    let _ = p;
    t.role(r)
}

const NORMAL: AppVariant<FieldVariant> = FieldVariant::app("font-normal");
const TIGHT: AppVariant<FieldGroupVariant> = FieldGroupVariant::app("checkbox-group");
const TIGHT_SET: AppVariant<FieldSetVariant> = FieldSetVariant::app("checkbox-group");

/// The demos' classes as app variants: `class="font-normal"` labels, the
/// `data-slot="checkbox-group"` group and the set around it (`gap-3`).
fn demo(theme: Theme) -> Theme {
    theme
        .variant(NORMAL, FieldVariant::Default, |r, _| {
            r.field_label.weight.set(Weight::Normal)
        })
        .variant(TIGHT, FieldGroupVariant::Default, |r, _| {
            r.field_group.gap.set(12.0)
        })
        .variant(TIGHT_SET, FieldSetVariant::Default, |r, _| {
            r.field_set.gap.set(12.0)
        })
}

// ---------------------------------------------------------------------------
// Layout

/// A vertical field stacks its children the field's gap apart: `gap-3` in
/// vega, `gap-2` in nova.
#[test]
fn vertical_children_are_the_fields_gap_apart() {
    for (t, gap) in [(vega(), 12.0), (nova(), 8.0)] {
        let ((label, input, desc), _) = settle(&t, 400.0, |ui| {
            let mut s = String::new();
            let mut r = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING);
            Field::new().show(ui, |ui| {
                r.0 = field_label(ui, "Username").rect;
                r.1 = ui.add(Input::new(&mut s)).rect;
                r.2 = field_description(ui, "Choose one.").rect;
            });
            r
        });
        assert_eq!(input.top() - label.bottom(), gap);
        assert_eq!(desc.top() - input.bottom(), gap);
        // `*:w-full`: the input takes the field's width, not the 280 cap.
        assert_eq!(input.width(), 400.0);
    }
}

/// `nth-last-2:-mt-1`: a description before the last child sits closer
/// to what precedes it; as the last child it keeps the gap (`last:mt-0`).
#[test]
fn a_description_last_but_one_moves_up() {
    let ((label, desc), _) = settle(&vega(), 400.0, |ui| {
        let mut s = String::new();
        let mut r = (Rect::NOTHING, Rect::NOTHING);
        Field::new().show(ui, |ui| {
            r.0 = field_label(ui, "Password").rect;
            r.1 = field_description(ui, "At least 8 characters.").rect;
            ui.add(Input::new(&mut s));
        });
        r
    });
    assert_eq!(desc.top() - label.bottom(), 12.0 - 4.0);
}

/// A select and a button take a vertical field's width (`*:w-full`).
#[test]
fn a_vertical_field_stretches_content_sized_children() {
    let ((select, button), _) = settle(&vega(), 400.0, |ui| {
        let mut i = 0;
        let mut r = (Rect::NOTHING, Rect::NOTHING);
        Field::new().label("Department").show(ui, |ui| {
            r.0 = ui
                .add(Select::new("d", &mut i, &["Engineering", "Design"]))
                .rect;
            r.1 = ui.add(Button::new("Go")).rect;
        });
        r
    });
    assert_eq!(select.width(), 400.0);
    assert_eq!(button.width(), 400.0);
}

/// Groups: `gap-7` between fields, `gap-4` in a group directly in a group.
#[test]
fn group_gaps_and_nesting() {
    let (gaps, _) = settle(&vega(), 400.0, |ui| {
        let mut g = (0.0, 0.0);
        FieldGroup::new().show(ui, |ui| {
            let a = ui.label("a").rect;
            let b = ui.label("b").rect;
            g.0 = b.top() - a.bottom();
            FieldGroup::new().show(ui, |ui| {
                let a = ui.label("c").rect;
                let b = ui.label("d").rect;
                g.1 = b.top() - a.bottom();
            });
        });
        g
    });
    assert_eq!(gaps, (28.0, 16.0));
}

/// A set's legend is no flex item: its `mb-3` follows it, not the set's
/// `gap-6`; a description right after a legend moves up by `-mt-1.5`, and
/// the set's gap stands between the description and what follows.
#[test]
fn legend_margin_replaces_the_sets_gap() {
    let ((legend, desc, next), _) = settle(&vega(), 400.0, |ui| {
        let mut r = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING);
        FieldSet::new().show(ui, |ui| {
            r.0 = ui.add(FieldLegend::new("Payment")).rect;
            r.1 = field_description(ui, "Secure.").rect;
            r.2 = ui.label("next").rect;
        });
        r
    });
    assert_eq!(desc.top() - legend.bottom(), 12.0 - 6.0);
    assert_eq!(next.top() - desc.bottom(), 24.0);
    // The label variant gets no `-mt-1.5`, only `nth-last-2:-mt-1`.
    let ((legend, desc), _) = settle(&vega(), 400.0, |ui| {
        let mut r = (Rect::NOTHING, Rect::NOTHING);
        FieldSet::new().show(ui, |ui| {
            r.0 = ui
                .add(FieldLegend::new("Show").variant(FieldLegendVariant::Label))
                .rect;
            r.1 = field_description(ui, "Pick.").rect;
            ui.label("next");
        });
        r
    });
    assert_eq!(desc.top() - legend.bottom(), 12.0 - 4.0);
}

/// Horizontal: the label takes the room the switch leaves
/// (`flex-auto`), so the switch ends at the field's end; a checkbox
/// before its label sits at the start.
#[test]
fn horizontal_label_grows_and_the_control_ends_the_row() {
    let ((label, switch), _) = settle(&vega(), 400.0, |ui| {
        let mut on = false;
        let mut r = (Rect::NOTHING, Rect::NOTHING);
        Field::new().horizontal().show(ui, |ui| {
            r.0 = field_label(ui, "Multi-factor authentication").rect;
            r.1 = ui.add(Switch::new(&mut on)).rect;
        });
        r
    });
    assert_eq!(switch.right(), 400.0);
    assert_eq!(switch.left() - label.right(), 12.0);
    let ((check, label), _) = settle(&vega(), 400.0, |ui| {
        let mut on = false;
        let mut r = (Rect::NOTHING, Rect::NOTHING);
        Field::new().horizontal().show(ui, |ui| {
            r.0 = ui.add(Checkbox::new(&mut on)).rect;
            r.1 = field_label(ui, "Hard disks").rect;
        });
        r
    });
    assert_eq!(check.left(), 0.0);
    assert_eq!(label.left() - check.right(), 12.0);
    // Centred on each other (`items-center`).
    assert!((check.center().y - label.center().y).abs() <= 0.5);
}

/// With a FieldContent the row is `items-start` and the checkbox moves
/// down by `mt-px`.
#[test]
fn field_content_aligns_the_row_to_the_top() {
    let ((check, content), _) = settle(&vega(), 400.0, |ui| {
        let mut on = false;
        let mut r = (Rect::NOTHING, Rect::NOTHING);
        Field::new().horizontal().show(ui, |ui| {
            r.0 = ui.add(Checkbox::new(&mut on)).rect;
            r.1 = field_content(ui, |ui| {
                field_label(ui, "Sync");
                field_description(ui, "Your folders are synced with iCloud Drive.");
            })
            .response
            .rect;
        });
        r
    });
    assert_eq!(check.top(), content.top() + 1.0);
    // `flex-1`: the content takes the rest of the row.
    assert_eq!(content.right(), 400.0);
}

/// Responsive: vertical in a group narrower than 28rem, a row from there.
#[test]
fn responsive_reads_the_groups_width() {
    for (width, row) in [(400.0, false), (600.0, true)] {
        let ((label, input), _) = settle(&vega(), width, |ui| {
            let mut s = String::new();
            let mut r = (Rect::NOTHING, Rect::NOTHING);
            FieldGroup::new().show(ui, |ui| {
                Field::new().responsive(true).show(ui, |ui| {
                    r.0 = field_label(ui, "Name").rect;
                    r.1 = ui.add(Input::new(&mut s)).rect;
                });
            });
            r
        });
        assert_eq!(input.top() > label.bottom(), !row, "width {width}");
    }
}

// ---------------------------------------------------------------------------
// Invalid, colours, disabled

fn invalid_node(h: &Harness<'_>, role: Role) -> Option<Invalid> {
    h.get_by_role(role).accesskit_node().data().invalid()
}

/// An invalid field (or one with an error) makes its crate controls
/// invalid; a control's own `.invalid(false)` wins.
#[test]
fn invalid_flows_into_the_controls() {
    type Make = fn(&mut Ui);
    let cases: [(Role, Make, Make); 4] = [
        (
            Role::TextInput,
            |ui| {
                ui.add(Input::new(&mut String::new()));
            },
            |ui| {
                ui.add(Input::new(&mut String::new()).invalid(false));
            },
        ),
        (
            Role::CheckBox,
            |ui| {
                ui.add(Checkbox::new(&mut false));
            },
            |ui| {
                ui.add(Checkbox::new(&mut false).invalid(false));
            },
        ),
        (
            Role::ComboBox,
            |ui| {
                ui.add(Select::new("s", &mut 0, &["A"]));
            },
            |ui| {
                ui.add(Select::new("s", &mut 0, &["A"]).invalid(false));
            },
        ),
        (
            Role::MultilineTextInput,
            |ui| {
                ui.add(Textarea::new(&mut String::new()));
            },
            |ui| {
                ui.add(Textarea::new(&mut String::new()).invalid(false));
            },
        ),
    ];
    for (role, plain, opted_out) in cases {
        for (how, field) in [
            ("invalid", Field::new as fn() -> Field),
            ("error", || Field::new().error("Required.")),
        ] {
            let invalid = how == "invalid";
            let mut h = Harness::new_ui(move |ui| {
                Theme::vega().apply(ui.ctx());
                let f = field();
                let f = if invalid { f.invalid(true) } else { f };
                f.show(ui, plain);
            });
            h.run();
            assert_eq!(
                invalid_node(&h, role),
                Some(Invalid::True),
                "{role:?} {how}"
            );
        }
        let mut h = Harness::new_ui(move |ui| {
            Theme::vega().apply(ui.ctx());
            Field::new().invalid(true).show(ui, opted_out);
        });
        h.run();
        assert_eq!(invalid_node(&h, role), None, "{role:?} opted out");
        let mut h = Harness::new_ui(move |ui| {
            Theme::vega().apply(ui.ctx());
            Field::new().show(ui, plain);
        });
        h.run();
        assert_eq!(invalid_node(&h, role), None, "{role:?} valid");
    }
}

/// `data-[invalid=true]:text-destructive` is inherited: the label and the
/// typed value turn destructive, the description and the placeholder
/// keep their muted colour, the error is destructive.
#[test]
fn invalid_text_colour_reaches_label_and_value_not_placeholder() {
    let t = vega();
    let (_, out) = settle(&t, 400.0, |ui| {
        let mut typed = String::from("max");
        let mut empty = String::new();
        Field::new()
            .invalid(true)
            .label("Username")
            .description("Pick one.")
            .error("Taken.")
            .show(ui, |ui| {
                ui.add(Input::new(&mut typed));
                ui.add(Input::new(&mut empty).hint("Placeholder"));
            });
    });
    let destructive = role(&t, Palette::Destructive);
    let muted = role(&t, Palette::MutedForeground);
    assert_eq!(paint_of(&out, "Username").0, destructive);
    assert_eq!(paint_of(&out, "max").0, destructive);
    assert_eq!(paint_of(&out, "Taken.").0, destructive);
    assert_eq!(paint_of(&out, "Pick one.").0, muted);
    assert_eq!(paint_of(&out, "Placeholder").0, muted);
    // A valid field leaves them in the foreground colour.
    let (_, out) = settle(&t, 400.0, |ui| {
        let mut typed = String::from("max");
        Field::new()
            .label("Username")
            .show(ui, |ui| ui.add(Input::new(&mut typed)));
    });
    let fg = role(&t, Palette::Foreground);
    assert_eq!(paint_of(&out, "Username").0, fg);
    assert_eq!(paint_of(&out, "max").0, fg);
}

/// A disabled `Ui` fades the label and the title (`group-data-[disabled=true]/field:opacity-50`);
/// the description and the error stay opaque.
#[test]
fn disabled_fades_label_and_title_only() {
    let (_, out) = settle(&vega(), 400.0, |ui| {
        ui.add_enabled_ui(false, |ui| {
            Field::new()
                .label("Name")
                .title("Title")
                .description("Helper.")
                .error("Wrong.")
                .show(ui, |ui| ui.add(Input::new(&mut String::new())));
        });
    });
    assert_eq!(faded(&out, "Name"), 0.5);
    assert_eq!(faded(&out, "Title"), 0.5);
    assert_eq!(faded(&out, "Helper."), 1.0);
    assert_eq!(faded(&out, "Wrong."), 1.0);
}

/// `peer-disabled:opacity-50`: a label after a disabled checkbox fades, a
/// label before it does not.
#[test]
fn a_label_after_a_disabled_peer_fades() {
    let (_, out) = settle(&vega(), 400.0, |ui| {
        Field::new().horizontal().show(ui, |ui| {
            field_label(ui, "Before");
            ui.add_enabled(false, Checkbox::new(&mut true));
            field_label(ui, "After");
        });
    });
    assert_eq!(faded(&out, "Before"), 1.0);
    assert_eq!(faded(&out, "After"), 0.5);
}

// ---------------------------------------------------------------------------
// Errors

/// upstream's `hasContent`: zero entries or one entry without a message
/// draw nothing and take no room; one message is text; two entries show
/// their messages as a list, an empty list included.
#[test]
fn error_content_rules() {
    let h = |e: fn() -> FieldError| {
        settle(&vega(), 400.0, |ui| {
            let a = ui.label("a").rect;
            let r = ui.add(e()).rect;
            let b = ui.label("b").rect;
            (r, b.top() - a.bottom())
        })
        .0
    };
    let spacing = h(|| FieldError::from_messages(Vec::<&str>::new())).1;
    assert_eq!(h(|| FieldError::from_messages([""])).1, spacing);
    let (one, _) = h(|| FieldError::from_messages(["Required."]));
    assert_eq!(one.height(), 20.0);
    // Two entries with one message: a one-item list, indented by `ml-4`.
    let (r, _) = settle(&vega(), 400.0, |ui| {
        ui.add(FieldError::from_messages(["Too short.", ""])).rect
    });
    let (_, out) = settle(&vega(), 400.0, |ui| {
        ui.add(FieldError::from_messages(["Too short.", ""]));
    });
    let text = common::text_shapes(&out)
        .into_iter()
        .find(|t| t.galley.text() == "Too short.")
        .expect("item");
    assert_eq!(text.pos.x, r.left() + 16.0);
    assert!(common::text_shapes(&out)
        .iter()
        .any(|t| t.galley.text().starts_with('\u{2022}')));
    // Two messages, `gap-1` apart.
    let (r, _) = settle(&vega(), 400.0, |ui| {
        ui.add(FieldError::from_messages(["One.", "Two."])).rect
    });
    assert_eq!(r.height(), 20.0 + 4.0 + 20.0);
    // Two empty entries: an empty alert that still takes the gaps.
    let (_, gap) = h(|| FieldError::from_messages(["", ""]));
    assert!(gap > spacing);
}

/// The error is an alert, an assertive live region.
#[test]
fn error_is_an_assertive_alert() {
    let mut h = Harness::new_ui(|ui| {
        Theme::vega().apply(ui.ctx());
        ui.add(FieldError::new("Required."));
    });
    h.run();
    let node = h.get_by_role(Role::Alert);
    assert_eq!(node.accesskit_node().data().live(), Some(Live::Assertive));
}

// ---------------------------------------------------------------------------
// Label and accessibility

/// The field is a group; its first control is named by its label, also
/// when the control comes before the label.
#[test]
fn the_label_names_the_first_control() {
    for label_first in [true, false] {
        let mut h = Harness::new_ui(move |ui| {
            Theme::vega().apply(ui.ctx());
            Field::new().horizontal().show(ui, |ui| {
                if label_first {
                    field_label(ui, "Accept");
                }
                ui.add(Checkbox::new(&mut false));
                ui.add(Switch::new(&mut false));
                if !label_first {
                    field_label(ui, "Accept");
                }
            });
        });
        h.run();
        h.run();
        let _ = h.get_by_role(Role::Group);
        let label = h
            .get_by_role_and_label(Role::Label, "Accept")
            .accesskit_node();
        let boxes = h.get_all_by_role(Role::CheckBox).collect::<Vec<_>>();
        let named: Vec<_> = boxes[0]
            .accesskit_node()
            .labelled_by()
            .map(|n| n.id())
            .collect();
        assert_eq!(named, vec![label.id()]);
        assert_eq!(boxes[1].accesskit_node().labelled_by().count(), 0);
    }
}

/// A press on the label toggles a checkbox before or after it, and
/// focuses a text field.
#[test]
fn a_label_press_acts_on_the_control() {
    for label_first in [true, false] {
        let mut on = false;
        let mut label = Rect::NOTHING;
        let mut h = Harness::new_ui_state(
            |ui, (on, label): &mut (bool, Rect)| {
                Theme::vega().apply(ui.ctx());
                Field::new().horizontal().show(ui, |ui| {
                    if label_first {
                        *label = field_label(ui, "Accept").rect;
                    }
                    ui.add(Checkbox::new(on));
                    if !label_first {
                        *label = field_label(ui, "Accept").rect;
                    }
                });
            },
            (on, label),
        );
        h.run();
        label = h.state().1;
        let at = label.center();
        h.input_mut().events.extend(common::click(at));
        h.run();
        on = h.state().0;
        assert!(on, "label first {label_first}");
    }
    let mut h = Harness::new_ui_state(
        |ui, (s, label): &mut (String, Rect)| {
            Theme::vega().apply(ui.ctx());
            Field::new().show(ui, |ui| {
                *label = field_label(ui, "Name").rect;
                ui.add(Input::new(s).id_salt("name"));
            });
        },
        (String::new(), Rect::NOTHING),
    );
    h.run();
    let at = h.state().1.center();
    h.input_mut().events.extend(common::click(at));
    h.run();
    assert!(h.get_by_role(Role::TextInput).is_focused());
}

/// A press on a choice card toggles its switch; the card shows the
/// switch's checked state from the next pass.
#[test]
fn a_choice_card_press_toggles_its_control() {
    let mut h = Harness::new_ui_state(
        |ui, (on, card): &mut (bool, Rect)| {
            Theme::vega().apply(ui.ctx());
            *card = FieldLabel::choice_card()
                .show(ui, |ui| {
                    Field::new()
                        .horizontal()
                        .title("Share")
                        .description("Across devices.")
                        .show(ui, |ui| ui.add(Switch::new(on)));
                })
                .response
                .rect;
        },
        (false, Rect::NOTHING),
    );
    h.run();
    let card = h.state().1;
    assert!(card.width() > 100.0);
    h.input_mut()
        .events
        .extend(common::click(card.left_top() + vec2(20.0, 20.0)));
    h.run();
    assert!(h.state().0);
}

/// Degenerate input: no content, empty text, zero width.
#[test]
fn degenerate_fields_do_not_panic() {
    for width in [0.0, 400.0] {
        let _ = settle(&vega(), width, |ui| {
            Field::new().show(ui, |_| ());
            Field::new().label("").description("").show(ui, |_| ());
            Field::new().horizontal().label("x").show(ui, |_| ());
            FieldSet::new().show(ui, |_| ());
            FieldGroup::new().show(ui, |ui| {
                ui.add(FieldSeparator::new().content(""));
            });
            ui.add(FieldError::from_messages(["", "", ""]));
        });
    }
}

// ---------------------------------------------------------------------------
// Snapshots (shadcn-svelte demos)

fn max_w(ui: &mut Ui, w: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui(vec2(w, ui.available_height()), |ui| {
        ui.set_width(w);
        add(ui);
    });
}

fn snapshot(name: &str, size: egui::Vec2, ui_fn: impl FnMut(&mut Ui) + Clone) {
    let themes = common::snapshot_themes()
        .into_iter()
        .map(|(n, t, m)| (n, demo(t), m))
        .collect();
    common::snapshot_with_themes(name, size, themes, ui_fn, |_| {});
}

#[test]
fn field_input_snapshot() {
    snapshot("field_input", vec2(400.0, 260.0), |ui| {
        max_w(ui, 320.0, |ui| {
            FieldSet::new().show(ui, |ui| {
                FieldGroup::new().show(ui, |ui| {
                    Field::new()
                        .label("Username")
                        .description("Choose a unique username for your account.")
                        .show(ui, |ui| {
                            ui.add(Input::new(&mut String::new()).hint("Max Leiter"))
                        });
                    Field::new().show(ui, |ui| {
                        field_label(ui, "Password");
                        field_description(ui, "Must be at least 8 characters long.");
                        ui.add(
                            Input::new(&mut String::new())
                                .password(true)
                                .hint("••••••••"),
                        );
                    });
                });
            });
        });
    });
}

#[test]
fn field_textarea_snapshot() {
    snapshot("field_textarea", vec2(400.0, 220.0), |ui| {
        max_w(ui, 320.0, |ui| {
            FieldSet::new().show(ui, |ui| {
                FieldGroup::new().show(ui, |ui| {
                    Field::new()
                        .label("Feedback")
                        .description("Share your thoughts about our service.")
                        .show(ui, |ui| {
                            ui.add(
                                Textarea::new(&mut String::new())
                                    .hint_text("Your feedback helps us improve..."),
                            )
                        });
                });
            });
        });
    });
}

#[test]
fn field_select_snapshot() {
    snapshot("field_select", vec2(400.0, 140.0), |ui| {
        max_w(ui, 320.0, |ui| {
            let mut i = usize::MAX;
            Field::new()
                .label("Department")
                .description("Select your department or area of work.")
                .show(ui, |ui| {
                    ui.add(
                        Select::new(
                            "department",
                            &mut i,
                            &["Engineering", "Design", "Marketing"],
                        )
                        .placeholder("Choose department"),
                    )
                });
        });
    });
}

#[test]
fn field_switch_snapshot() {
    snapshot("field_switch", vec2(340.0, 70.0), |ui| {
        let mut on = false;
        // `w-fit`: the field in unbounded room takes its content's width.
        let room = Rect::from_min_size(ui.cursor().min, vec2(f32::INFINITY, 40.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(room), |ui| {
            Field::new()
                .horizontal()
                .label("Multi-factor authentication")
                .show(ui, |ui| ui.add(Switch::new(&mut on)));
        });
    });
}

#[test]
fn field_checkbox_snapshot() {
    snapshot("field_checkbox", vec2(400.0, 400.0), |ui| {
        max_w(ui, 320.0, |ui| {
            FieldGroup::new().show(ui, |ui| {
                FieldSet::new().show(ui, |ui| {
                    ui.add(
                        FieldLegend::new("Show these items on the desktop")
                            .variant(FieldLegendVariant::Label),
                    );
                    field_description(ui, "Select the items you want to show on the desktop.");
                    FieldGroup::new().variant(TIGHT).show(ui, |ui| {
                        for (name, mut on) in [
                            ("Hard disks", false),
                            ("External disks", false),
                            ("CDs, DVDs, and iPods", false),
                            ("Connected servers", false),
                        ] {
                            Field::new().horizontal().variant(NORMAL).show(ui, |ui| {
                                ui.add(Checkbox::new(&mut on));
                                field_label(ui, name);
                            });
                        }
                    });
                });
                ui.add(FieldSeparator::new());
                let mut on = true;
                Field::new().horizontal().show(ui, |ui| {
                    ui.add(Checkbox::new(&mut on));
                    field_content(ui, |ui| {
                        field_label(ui, "Sync Desktop & Documents folders");
                        field_description(
                            ui,
                            "Your Desktop & Documents folders are being synced with iCloud \
                             Drive. You can access them from other devices.",
                        );
                    });
                });
            });
        });
    });
}

#[test]
fn field_fieldset_snapshot() {
    snapshot("field_fieldset", vec2(420.0, 290.0), |ui| {
        max_w(ui, 384.0, |ui| {
            FieldSet::new()
                .legend("Address Information")
                .description("We need your address to deliver your order.")
                .show(ui, |ui| {
                    FieldGroup::new().show(ui, |ui| {
                        Field::new().label("Street Address").show(ui, |ui| {
                            ui.add(Input::new(&mut String::new()).hint("123 Main St"))
                        });
                        ui.spacing_mut().item_spacing.x = 16.0;
                        ui.columns_const(|[a, b]| {
                            Field::new().label("City").show(a, |ui| {
                                ui.add(Input::new(&mut String::new()).hint("New York"))
                            });
                            Field::new()
                                .label("Postal Code")
                                .show(b, |ui| ui.add(Input::new(&mut String::new()).hint("90502")));
                        });
                    });
                });
        });
    });
}

#[test]
fn field_field_group_demo_snapshot() {
    snapshot("field_field_group_demo", vec2(400.0, 380.0), |ui| {
        max_w(ui, 320.0, |ui| {
            FieldGroup::new().show(ui, |ui| {
                FieldSet::new().variant(TIGHT_SET).show(ui, |ui| {
                    field_label(ui, "Responses");
                    field_description(
                        ui,
                        "Get notified when ChatGPT responds to requests that take time, like \
                         research or image generation.",
                    );
                    FieldGroup::new().variant(TIGHT).show(ui, |ui| {
                        Field::new().horizontal().variant(NORMAL).show(ui, |ui| {
                            ui.add_enabled(false, Checkbox::new(&mut true));
                            field_label(ui, "Push notifications");
                        });
                    });
                });
                ui.add(FieldSeparator::new());
                FieldSet::new().variant(TIGHT_SET).show(ui, |ui| {
                    field_label(ui, "Tasks");
                    field_description(ui, "Get notified when tasks you've created have updates.");
                    FieldGroup::new().variant(TIGHT).show(ui, |ui| {
                        for name in ["Push notifications", "Email notifications"] {
                            Field::new().horizontal().variant(NORMAL).show(ui, |ui| {
                                ui.add(Checkbox::new(&mut false));
                                field_label(ui, name);
                            });
                        }
                    });
                });
            });
        });
    });
}

#[test]
fn field_demo_snapshot() {
    snapshot("field_demo", vec2(480.0, 760.0), |ui| {
        max_w(ui, 448.0, |ui| {
            let (mut month, mut year) = (usize::MAX, usize::MAX);
            FieldGroup::new().show(ui, |ui| {
                FieldSet::new()
                    .legend("Payment Method")
                    .description("All transactions are secure and encrypted")
                    .show(ui, |ui| {
                        FieldGroup::new().show(ui, |ui| {
                            Field::new().label("Name on Card").show(ui, |ui| {
                                ui.add(Input::new(&mut String::new()).hint("Evil Rabbit"))
                            });
                            Field::new()
                                .label("Card Number")
                                .description("Enter your 16-digit card number")
                                .show(ui, |ui| {
                                    ui.add(
                                        Input::new(&mut String::new()).hint("1234 5678 9012 3456"),
                                    )
                                });
                            ui.spacing_mut().item_spacing.x = 16.0;
                            ui.columns_const(|[a, b, c]| {
                                Field::new().label("Month").show(a, |ui| {
                                    ui.add(
                                        Select::new("m", &mut month, &["01", "02"])
                                            .placeholder("MM"),
                                    )
                                });
                                Field::new().label("Year").show(b, |ui| {
                                    ui.add(
                                        Select::new("y", &mut year, &["2024", "2025"])
                                            .placeholder("YYYY"),
                                    )
                                });
                                Field::new().label("CVV").show(c, |ui| {
                                    ui.add(Input::new(&mut String::new()).hint("123"))
                                });
                            });
                        });
                    });
                ui.add(FieldSeparator::new());
                FieldSet::new()
                    .legend("Billing Address")
                    .description("The billing address associated with your payment method")
                    .show(ui, |ui| {
                        FieldGroup::new().show(ui, |ui| {
                            Field::new().horizontal().variant(NORMAL).show(ui, |ui| {
                                ui.add(Checkbox::new(&mut true));
                                field_label(ui, "Same as shipping address");
                            });
                        });
                    });
                FieldSet::new().show(ui, |ui| {
                    FieldGroup::new().show(ui, |ui| {
                        Field::new().label("Comments").show(ui, |ui| {
                            ui.add(
                                Textarea::new(&mut String::new())
                                    .hint_text("Add any additional comments"),
                            )
                        });
                    });
                });
                Field::new().horizontal().show(ui, |ui| {
                    ui.add(Button::new("Submit"));
                    ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                });
            });
        });
    });
}

#[test]
fn field_slider_snapshot() {
    snapshot("field_slider", vec2(400.0, 140.0), |ui| {
        max_w(ui, 320.0, |ui| {
            let mut v = 200.0;
            Field::new().show(ui, |ui| {
                field_title(ui, "Price Range");
                field_description(ui, "Set your budget range ($200 - 800).");
                ui.add_space(8.0);
                ui.add(Slider::new(&mut v, 0.0..=1000.0).step(10.0));
            });
        });
    });
}

#[test]
fn field_invalid_snapshot() {
    snapshot("field_invalid", vec2(400.0, 330.0), |ui| {
        max_w(ui, 320.0, |ui| {
            FieldGroup::new().show(ui, |ui| {
                Field::new()
                    .label("Email")
                    .error("Enter a valid email address.")
                    .show(ui, |ui| ui.add(Input::new(&mut String::from("max@"))));
                Field::new()
                    .label("Password")
                    .error("Must be at least 8 characters long.")
                    .error("Must contain a number.")
                    .show(ui, |ui| {
                        ui.add(Input::new(&mut String::new()).hint("••••••••"))
                    });
                Field::new()
                    .invalid(true)
                    .horizontal()
                    .variant(NORMAL)
                    .show(ui, |ui| {
                        ui.add(Checkbox::new(&mut false));
                        field_label(ui, "Invalid checkbox");
                    });
            });
        });
    });
}

#[test]
fn field_disabled_snapshot() {
    snapshot("field_disabled", vec2(400.0, 200.0), |ui| {
        max_w(ui, 320.0, |ui| {
            ui.add_enabled_ui(false, |ui| {
                FieldGroup::new().show(ui, |ui| {
                    Field::new()
                        .label("Disabled input")
                        .description("This field is currently disabled.")
                        .show(ui, |ui| {
                            ui.add(Input::new(&mut String::new()).hint("Disabled"))
                        });
                    Field::new().horizontal().variant(NORMAL).show(ui, |ui| {
                        ui.add(Checkbox::new(&mut false));
                        field_label(ui, "Disabled checkbox");
                    });
                });
            });
        });
    });
}

fn responsive(ui: &mut Ui) {
    FieldSet::new()
        .legend("Profile")
        .description("Fill in your profile information.")
        .show(ui, |ui| {
            FieldGroup::new().show(ui, |ui| {
                Field::new().responsive(true).show(ui, |ui| {
                    field_content(ui, |ui| {
                        field_label(ui, "Name");
                        field_description(ui, "Provide your full name for identification");
                    });
                    ui.add(Input::new(&mut String::new()).hint("Evil Rabbit"));
                });
                Field::new().responsive(true).show(ui, |ui| {
                    ui.add(Button::new("Submit"));
                    ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                });
            });
        });
}

#[test]
fn field_responsive_narrow_snapshot() {
    snapshot("field_responsive_narrow", vec2(360.0, 330.0), |ui| {
        responsive(ui)
    });
}

#[test]
fn field_responsive_wide_snapshot() {
    snapshot("field_responsive_wide", vec2(560.0, 240.0), |ui| {
        max_w(ui, 512.0, responsive)
    });
}

#[test]
fn field_separator_text_snapshot() {
    snapshot("field_separator_text", vec2(400.0, 70.0), |ui| {
        FieldGroup::new().show(ui, |ui| {
            ui.add(FieldSeparator::new().content("Or continue with"));
        });
    });
}

#[test]
fn field_choice_card_snapshot() {
    snapshot("field_choice_card", vec2(420.0, 360.0), |ui| {
        max_w(ui, 384.0, |ui| {
            FieldGroup::new().show(ui, |ui| {
                for (title, desc, mut on) in [
                    (
                        "Share across devices",
                        "Focus is shared across devices, and turns off when you leave the app.",
                        false,
                    ),
                    (
                        "Enable notifications",
                        "Receive notifications when focus mode is enabled or disabled.",
                        true,
                    ),
                ] {
                    FieldLabel::choice_card().show(ui, |ui| {
                        Field::new()
                            .horizontal()
                            .title(title)
                            .description(desc)
                            .show(ui, |ui| ui.add(Switch::new(&mut on)));
                    });
                }
                let mut on = true;
                FieldLabel::choice_card().show(ui, |ui| {
                    Field::new().horizontal().show(ui, |ui| {
                        ui.add(Checkbox::new(&mut on));
                        field_content(ui, |ui| {
                            field_title(ui, "Enable Touch ID");
                            field_description(ui, "Enable Touch ID to quickly unlock your device.");
                        });
                    });
                });
            });
        });
    });
}
