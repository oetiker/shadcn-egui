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
use shadcn_egui::components::label::Label;
use shadcn_egui::components::select::Select;
use shadcn_egui::components::slider::Slider;
use shadcn_egui::components::switch::Switch;
use shadcn_egui::components::textarea::Textarea;
use shadcn_egui::style::{AppVariant, Role as Palette, Style, TextSize, Weight};
use shadcn_egui::theme::family_for;
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
        let mut out = ctx.run_ui(input(), |ui| t = Some(add(ui)));
        out.textures_delta.clear();
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

/// `*:w-full` reaches the field's own children only: a label in a row
/// inside a vertical field (input-badge's label beside its badge) keeps
/// its own width, and the row's end stays in the field.
#[test]
fn a_label_in_a_row_in_a_vertical_field_keeps_its_own_width() {
    use shadcn_egui::components::badge::{badge, BadgeVariant};
    let ((field, label, end), _) = settle(&vega(), 400.0, |ui| {
        let mut r = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING);
        r.0 = Field::new()
            .show(ui, |ui| {
                shadcn_egui::layout::space_between(
                    ui,
                    |ui| r.1 = field_label(ui, "Webhook URL").rect,
                    |ui| r.2 = badge(ui, "Beta", BadgeVariant::Secondary).rect,
                );
            })
            .response
            .rect;
        r
    });
    assert!(label.width() < 200.0, "the label's own width: {label:?}");
    assert_eq!(end.right(), field.right(), "the badge ends the row");
}

/// A text field first in a horizontal field (`w-full min-w-0`) shrinks
/// for what follows it (input-inline), as a label does: the button ends
/// the row, the field's gap after the text field.
#[test]
fn a_text_field_first_in_a_horizontal_field_leaves_room_for_what_follows() {
    for textarea in [false, true] {
        let ((field, button), _) = settle(&vega(), 400.0, |ui| {
            let mut text = String::new();
            let mut r = (Rect::NOTHING, Rect::NOTHING);
            Field::new().horizontal().show(ui, |ui| {
                r.0 = if textarea {
                    ui.add(Textarea::new(&mut text)).rect
                } else {
                    ui.add(Input::new(&mut text)).rect
                };
                r.1 = ui.add(Button::new("Search")).rect;
            });
            r
        });
        assert_eq!(field.left(), 0.0, "textarea {textarea}");
        assert_eq!(
            button.right(),
            400.0,
            "textarea {textarea}: the button ends the row"
        );
        assert_eq!(button.left() - field.right(), 12.0, "textarea {textarea}");
    }
}

/// Two text fields in a horizontal field have the same basis (`w-full`,
/// 100 %) and shrink alike for the button (`shrink-0`): equal widths,
/// the button ends the row.
#[test]
fn text_fields_in_a_horizontal_field_share_the_room() {
    let ((a, b, button), _) = settle(&vega(), 400.0, |ui| {
        let (mut x, mut y) = (String::new(), String::new());
        let mut r = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING);
        Field::new().horizontal().show(ui, |ui| {
            r.0 = ui.add(Input::new(&mut x)).rect;
            r.1 = ui.add(Input::new(&mut y)).rect;
            r.2 = ui.add(Button::new("Go")).rect;
        });
        r
    });
    assert_eq!(a.left(), 0.0);
    assert_eq!(a.width(), b.width(), "{a:?} {b:?}");
    assert_eq!(b.left() - a.right(), 12.0);
    assert_eq!(button.left() - b.right(), 12.0);
    assert_eq!(button.right(), 400.0, "the button ends the row");
}

/// A label first in a horizontal field (`flex-auto`, its content width
/// as basis, no narrower than its longest word) before text fields
/// (`w-full min-w-0`, the field's width as basis): the text fields shrink
/// far more, the label keeps its width and the text fields share the
/// rest alike.
#[test]
fn a_label_before_text_fields_keeps_its_width() {
    let natural = settle(&vega(), 400.0, |ui| field_label(ui, "Name").rect.width()).0;
    for n in [1, 2] {
        let ((label, inputs), _) = settle(&vega(), 400.0, |ui| {
            let mut texts = vec![String::new(); n];
            let mut r = (Rect::NOTHING, Vec::new());
            Field::new().horizontal().show(ui, |ui| {
                r.0 = field_label(ui, "Name").rect;
                for t in &mut texts {
                    r.1.push(ui.add(Input::new(t)).rect);
                }
            });
            r
        });
        assert_eq!(label.width(), natural, "{n} inputs: {label:?}");
        assert_eq!(inputs[0].left() - label.right(), 12.0, "{n} inputs");
        let last = inputs[n - 1];
        assert_eq!(last.right(), 400.0, "{n} inputs: {inputs:?}");
        assert!(
            inputs.iter().all(|i| i.width() == inputs[0].width()),
            "{inputs:?}"
        );
    }
}

/// A text field with its own width after the growing text field is not
/// shrunk to the room left: the field measures what follows the grower
/// in its first pass, and the first frame ends the row at the field's
/// edge.
#[test]
fn a_sized_text_field_after_the_growing_one_fits_in_the_first_frame() {
    let ctx = egui::Context::default();
    vega().apply(&ctx);
    ctx.set_theme(egui::Theme::Light);
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 800.0))),
        ..Default::default()
    };
    let mut r = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING);
    ctx.run_ui(input, |ui| {
        let (mut x, mut y) = (String::new(), String::new());
        Field::new().horizontal().show(ui, |ui| {
            r.0 = ui.add(Input::new(&mut x)).rect;
            r.1 = ui.add(Input::new(&mut y).max_width(100.0)).rect;
            r.2 = ui.add(Button::new("Go")).rect;
        });
    })
    .drop_without_applying_deltas();
    let (a, b, button) = r;
    assert_eq!(b.width(), 100.0, "{b:?}");
    assert_eq!(b.left() - a.right(), 12.0);
    assert_eq!(button.right(), 400.0, "{a:?} {b:?} {button:?}");
}

/// A control in a row inside a vertical field is not the field's own
/// child, so `*:w-full` does not reach it: an input beside a button keeps
/// its default width and the button stays in the field.
#[test]
fn a_control_in_a_row_in_a_vertical_field_keeps_its_own_width() {
    let ((field, input, button), _) = settle(&vega(), 400.0, |ui| {
        let mut text = String::new();
        let mut r = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING);
        r.0 = Field::new()
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    r.1 = ui.add(Input::new(&mut text)).rect;
                    r.2 = ui.add(Button::new("Go")).rect;
                });
            })
            .response
            .rect;
        r
    });
    assert!(input.width() < field.width(), "{input:?} in {field:?}");
    assert!(button.right() <= field.right(), "{button:?} in {field:?}");
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
            // A bare control has no accessible name (as upstream's demo); names are
            // not what this test checks.
            let mut h = Harness::builder()
                .with_accessibility_check(false)
                .build_ui(move |ui| {
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
        // A bare control has no accessible name (as upstream's demo); names are
        // not what this test checks.
        let mut h = Harness::builder()
            .with_accessibility_check(false)
            .build_ui(move |ui| {
                Theme::vega().apply(ui.ctx());
                Field::new().invalid(true).show(ui, opted_out);
            });
        h.run();
        assert_eq!(invalid_node(&h, role), None, "{role:?} opted out");
        // A bare control has no accessible name (as upstream's demo); names are
        // not what this test checks.
        let mut h = Harness::builder()
            .with_accessibility_check(false)
            .build_ui(move |ui| {
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

/// A horizontal field whose label grows (`flex-auto`) and has crate
/// labels after it (flex items that shrink) settles: the leaves keep
/// their unwrapped width where the row has room for their base sizes, and
/// the growing label takes the rest, so the row ends at the field's right
/// edge. The grower once counted at the room it grew into, so the leaves
/// shrank in the pass after it filled the row and the label grew into
/// the room that freed in the pass after that: the layout flipped between
/// two states on every pass and asked for a discard on every frame.
#[test]
fn a_horizontal_field_with_labels_after_its_label_settles() {
    let ctx = egui::Context::default();
    vega().apply(&ctx);
    ctx.set_theme(egui::Theme::Light);
    let mut last = Vec::new();
    let mut discards = Vec::new();
    for _ in 0..6 {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(600.0, 800.0))),
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| {
            Field::new().horizontal().show(ui, |ui| {
                last = vec![
                    field_label(ui, "Before").rect,
                    ui.add(Label::new("Plain before")).rect,
                    ui.add_enabled(false, Checkbox::new(&mut true)).rect,
                    field_label(ui, "After").rect,
                    ui.add(Label::new("Plain after")).rect,
                ];
            });
        });
        discards.push(out.platform_output.request_discard_reasons.len());
        let _ = common::no_textures(out);
    }
    assert_eq!(discards[3..], [0, 0, 0], "{discards:?} {last:?}");
    let [before, plain_before, _, _, plain_after] = last[..] else {
        panic!("{last:?}")
    };
    // One line each: as tall as the line box, not two.
    assert_eq!(plain_before.height(), 14.0, "{last:?}");
    assert_eq!(plain_after.height(), 14.0, "{last:?}");
    assert!((plain_after.right() - 600.0).abs() < 0.1, "{last:?}");
    assert!(before.width() > 300.0, "{last:?}");
}

/// `peer-disabled:opacity-50`: a label after a disabled checkbox fades, a
/// label before it does not. A plain `Label` there is `cn-label` too
/// (checkbox-demo's `<Label for>`): it fades and shows the not-allowed
/// cursor (`peer-disabled:cursor-not-allowed`) as a field label does. A
/// `<label>` has no disabled state (D-1), so neither label reports one to
/// assistive technology; the checkbox does.
#[test]
fn a_label_after_a_disabled_peer_fades() {
    fn row(ui: &mut Ui) {
        Field::new().horizontal().show(ui, |ui| {
            field_label(ui, "Before");
            ui.add(Label::new("Plain before"));
            ui.add_enabled(false, Checkbox::new(&mut true));
            field_label(ui, "After");
            ui.add(Label::new("Plain after"));
        });
    }
    let (_, out) = settle(&vega(), 600.0, row);
    assert_eq!(faded(&out, "Before"), 1.0);
    assert_eq!(faded(&out, "Plain before"), 1.0);
    assert_eq!(faded(&out, "After"), 0.5);
    assert_eq!(faded(&out, "Plain after"), 0.5);

    let mut h = Harness::builder()
        .with_size(vec2(600.0, 200.0))
        .build_ui(|ui| {
            vega().apply(ui.ctx());
            row(ui);
        });
    h.run();
    assert!(h.get_by_role(Role::CheckBox).accesskit_node().is_disabled());
    for text in ["Before", "Plain before", "After", "Plain after"] {
        let label = h.get_by_role_and_label(Role::Label, text);
        assert!(!label.accesskit_node().is_disabled(), "{text}");
    }
    for (text, not_allowed) in [("Plain before", false), ("Plain after", true)] {
        let at = h.get_by_role_and_label(Role::Label, text).rect().center();
        h.input_mut().events.push(egui::Event::PointerMoved(at));
        h.run();
        let cursor = h.output().platform_output.cursor_icon;
        assert_eq!(
            cursor == egui::CursorIcon::NotAllowed,
            not_allowed,
            "{text}: {cursor:?}"
        );
    }
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
        // A control is unnamed on purpose: the test checks the names itself.
        let mut h = Harness::builder()
            .with_accessibility_check(false)
            .build_ui(move |ui| {
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

/// A press on the label of a field whose only control is a Button clicks
/// it, as `<label for=button>` does, and the label names it.
#[test]
fn a_label_press_clicks_a_button() {
    let mut h = Harness::new_ui_state(
        |ui, (clicks, label): &mut (usize, Rect)| {
            Theme::vega().apply(ui.ctx());
            Field::new().show(ui, |ui| {
                *label = field_label(ui, "Avatar").rect;
                if ui.add(Button::new("Upload")).clicked() {
                    *clicks += 1;
                }
            });
        },
        (0, Rect::NOTHING),
    );
    h.run();
    let at = h.state().1.center();
    h.input_mut().events.extend(common::click(at));
    h.run();
    assert_eq!(h.state().0, 1);
    let label = h
        .get_by_role_and_label(Role::Label, "Avatar")
        .accesskit_node()
        .id();
    let button = h.get_by_role(Role::Button).accesskit_node();
    assert_eq!(
        button.labelled_by().map(|n| n.id()).collect::<Vec<_>>(),
        [label]
    );
}

/// A Button before a text field leaves the field's label to the text
/// field: the label names the text field and its press focuses it; the
/// button keeps its own name and is not clicked.
#[test]
fn a_button_before_a_text_field_leaves_it_the_label() {
    let mut h = Harness::new_ui_state(
        |ui, (s, clicks, label): &mut (String, usize, Rect)| {
            Theme::vega().apply(ui.ctx());
            Field::new().show(ui, |ui| {
                *label = field_label(ui, "Name").rect;
                if ui.add(Button::new("Clear")).clicked() {
                    *clicks += 1;
                }
                ui.add(Input::new(s).id_salt("name"));
            });
        },
        (String::new(), 0, Rect::NOTHING),
    );
    h.run();
    let at = h.state().2.center();
    h.input_mut().events.extend(common::click(at));
    h.run();
    assert_eq!(h.state().1, 0, "the button was clicked");
    assert!(h.get_by_role(Role::TextInput).is_focused());
    let label = h
        .get_by_role_and_label(Role::Label, "Name")
        .accesskit_node()
        .id();
    let input = h.get_by_role(Role::TextInput).accesskit_node();
    assert_eq!(
        input.labelled_by().map(|n| n.id()).collect::<Vec<_>>(),
        [label]
    );
    let button = h.get_by_role(Role::Button).accesskit_node();
    assert_eq!(button.labelled_by().count(), 0);
}

/// A choice card names its control by the text inside it, as a `<label>`
/// around the field does in the browser, also when the field has only a
/// title and a description and no label of its own.
#[test]
fn a_choice_card_names_its_control_by_its_text() {
    let mut h = Harness::new_ui(|ui| {
        Theme::vega().apply(ui.ctx());
        FieldLabel::choice_card().show(ui, |ui| {
            Field::new()
                .horizontal()
                .title("Share")
                .description("Across devices.")
                .show(ui, |ui| ui.add(Switch::new(&mut false)));
        });
    });
    h.run();
    let switch = h.get_by_role(Role::CheckBox).accesskit_node();
    assert_eq!(switch.label().as_deref(), Some("Share Across devices."));
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

/// A keyboard-focused control in a choice card keeps its own ring next to
/// the card's. Upstream's `group-has-[:focus-visible]/field-label:ring-0`
/// ties `focus-visible:ring-3` on specificity and Tailwind emits it first,
/// so in Chrome the control's `ring-3` wins; only a control without its own
/// focus (the switch here) drops its ring.
#[test]
fn a_focused_control_in_a_choice_card_keeps_its_ring() {
    let theme = vega();
    let ((card, checkbox, switch), out) = settle(&theme, 400.0, |ui| {
        let mut r = (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING);
        r.0 = FieldLabel::choice_card()
            .show(ui, |ui| {
                Field::new().horizontal().show(ui, |ui| {
                    let c = ui.add(Checkbox::new(&mut false));
                    if !c.has_focus() {
                        c.request_focus();
                    }
                    r.1 = c.rect;
                    field_label(ui, "Touch ID");
                    r.2 = ui.add(Switch::new(&mut false)).rect;
                });
            })
            .response
            .rect;
        r
    });
    let ring = role(&theme, Palette::Ring).gamma_multiply(0.5);
    let rings: Vec<Rect> = common::rect_shapes(&out)
        .into_iter()
        .filter(|r| r.stroke.width == 3.0 && r.stroke.color == ring)
        .map(|r| r.rect)
        .collect();
    assert!(rings.contains(&card), "no card ring in {rings:?}");
    assert!(
        rings.iter().any(|r| checkbox.contains_rect(*r)),
        "no checkbox ring in {rings:?}, checkbox {checkbox:?}"
    );
    assert!(
        !rings.iter().any(|r| switch.contains_rect(*r)),
        "a switch ring in {rings:?}, switch {switch:?}"
    );
    assert_eq!(rings.len(), 2, "{rings:?}");
}

/// The control a choice card holds in the card tests.
#[derive(Clone, Copy, Debug)]
enum CardControl {
    Switch,
    Input,
}

/// The rects of a choice card, its control and its title.
#[derive(Clone, Copy, Debug)]
struct CardRects {
    card: Rect,
    control: Rect,
    title: Rect,
}

/// A choice card with a title and `control`, run on one context. The
/// second frame gets the events `act` makes from the first frame's rects,
/// and three frames without input follow (the pointer stays where the
/// events left it). The rects, whether the switch is on or the text field
/// focused, and the last frame's output.
fn card_after(
    control: CardControl,
    act: impl Fn(CardRects) -> Vec<egui::Event>,
) -> (CardRects, bool, egui::FullOutput) {
    let ctx = egui::Context::default();
    vega().apply(&ctx);
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|s| s.animation_time = 0.0);
    let (mut on, mut text) = (false, String::new());
    let mut frame = |events: Vec<egui::Event>| {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 300.0))),
            events,
            ..Default::default()
        };
        let mut r = CardRects {
            card: Rect::NOTHING,
            control: Rect::NOTHING,
            title: Rect::NOTHING,
        };
        let mut active = false;
        let mut out = ctx.run_ui(input, |ui| {
            r.card = FieldLabel::choice_card()
                .show(ui, |ui| {
                    Field::new().horizontal().show(ui, |ui| {
                        field_content(ui, |ui| r.title = field_title(ui, "Share").rect);
                        let c = match control {
                            CardControl::Switch => ui.add(Switch::new(&mut on)),
                            CardControl::Input => ui.add(Input::new(&mut text).max_width(120.0)),
                        };
                        active = on || c.has_focus();
                        r.control = c.rect;
                    });
                })
                .response
                .rect;
        });
        out.textures_delta.clear();
        (r, active, out)
    };
    let mut last = frame(Vec::new());
    for _ in 0..2 {
        last = frame(Vec::new());
    }
    last = frame(act(last.0));
    for _ in 0..3 {
        last = frame(Vec::new());
    }
    last
}

/// The 3 px focus rings a frame paints.
fn rings(out: &egui::FullOutput) -> Vec<Rect> {
    let ring = role(&vega(), Palette::Ring).gamma_multiply(0.5);
    common::rect_shapes(out)
        .into_iter()
        .filter(|r| r.stroke.width == 3.0 && r.stroke.color == ring)
        .map(|r| r.rect)
        .collect()
}

/// A pointer press on a choice card's text or on its switch toggles the
/// switch and shows no ring: pointer focus is not `:focus-visible`.
#[test]
fn a_pointer_press_in_a_choice_card_shows_no_ring() {
    for on_switch in [false, true] {
        let (_, on, out) = card_after(CardControl::Switch, |r| {
            common::click(if on_switch {
                r.control.center()
            } else {
                r.title.center()
            })
        });
        assert!(on, "on switch {on_switch}");
        assert_eq!(rings(&out), [], "on switch {on_switch}");
    }
}

/// A pointer press on a text field in a choice card focuses it and rings
/// both: a text field matches `:focus-visible` also on pointer focus.
#[test]
fn a_pointer_press_on_a_text_field_in_a_choice_card_rings_both() {
    let (r, focused, out) = card_after(CardControl::Input, |r| common::click(r.control.center()));
    assert!(focused);
    let rings = rings(&out);
    assert!(rings.contains(&r.card), "no card ring in {rings:?}");
    assert!(
        rings.iter().any(|x| r.control.contains_rect(*x)),
        "no input ring in {rings:?}, input {:?}",
        r.control
    );
    assert_eq!(rings.len(), 2, "{rings:?}");
}

/// The card keeps its hover fill (`hover:bg-muted/50`) while the pointer
/// is over its control or its text, and drops it outside.
#[test]
fn a_choice_card_stays_hovered_over_its_control_and_text() {
    let hover = role(&vega(), Palette::Muted).gamma_multiply(0.5);
    let filled = |out: &egui::FullOutput, card: Rect| {
        common::rect_shapes(out)
            .iter()
            .any(|s| card.contains_rect(s.rect) && s.fill == hover)
    };
    type Spot = fn(CardRects) -> egui::Pos2;
    let spots: [(&str, Spot); 3] = [
        ("control", |r| r.control.center()),
        ("title", |r| r.title.center()),
        ("card", |r| r.card.left_top() + vec2(4.0, 4.0)),
    ];
    for (name, at) in spots {
        let (r, _, out) = card_after(CardControl::Switch, |r| {
            vec![egui::Event::PointerMoved(at(r))]
        });
        assert!(filled(&out, r.card), "not hovered over the {name}");
    }
    let (r, _, out) = card_after(CardControl::Switch, |r| {
        vec![egui::Event::PointerMoved(
            r.card.right_bottom() + vec2(20.0, 20.0),
        )]
    });
    assert!(!filled(&out, r.card), "hovered outside");
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

/// field-field-group-demo. The "Manage tasks" link in the Tasks
/// description is left out: the crate has no link inside a run of text
/// (the description's `[&>a]` rules style caller content, D-100).
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

/// field-slider, with one thumb where upstream's slider has two
/// (`type="multiple"`): the crate's Slider has one thumb.
#[test]
fn field_slider_snapshot() {
    snapshot("field_slider", vec2(400.0, 140.0), |ui| {
        max_w(ui, 320.0, |ui| {
            let mut v = 200.0;
            Field::new().show(ui, |ui| {
                field_title(ui, "Price Range");
                // The values are `font-medium`; egui has no `tabular-nums`.
                let size = Theme::current(ui.ctx()).text_px(TextSize::Sm);
                let mut job = egui::text::LayoutJob::default();
                for (text, w) in [
                    ("Set your budget range ($", Weight::Normal),
                    ("200", Weight::Medium),
                    (" - ", Weight::Normal),
                    ("800", Weight::Medium),
                    (").", Weight::Normal),
                ] {
                    let font = egui::FontId::new(size, family_for(ui.ctx(), w));
                    job.append(
                        text,
                        0.0,
                        egui::TextFormat::simple(font, Color32::PLACEHOLDER),
                    );
                }
                field_description(ui, job);
                ui.add_space(8.0);
                // Upstream: `aria-label="Price Range"`.
                ui.add(
                    Slider::new(&mut v, 0.0..=1000.0)
                        .step(10.0)
                        .label("Price Range"),
                );
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

/// `.peer:disabled ~ label` matches siblings only: of two rows in one
/// field, only the labels in the row of the disabled checkbox fade, a
/// plain `Label` and a field label alike.
#[test]
fn a_disabled_peer_fades_only_the_labels_in_its_row() {
    let (_, out) = settle(&vega(), 600.0, |ui| {
        Field::new().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_enabled(false, Checkbox::new(&mut true));
                ui.add(Label::new("Row one"));
                field_label(ui, "Row one field");
            });
            ui.horizontal(|ui| {
                ui.add(Checkbox::new(&mut true));
                ui.add(Label::new("Row two"));
                field_label(ui, "Row two field");
            });
        });
    });
    assert_eq!(faded(&out, "Row one"), 0.5);
    assert_eq!(faded(&out, "Row one field"), 0.5);
    assert_eq!(faded(&out, "Row two"), 1.0, "row two");
    assert_eq!(faded(&out, "Row two field"), 1.0, "row two");
}

/// A disabled peer fades no label in a later field, nor one outside any
/// field.
#[test]
fn a_disabled_peer_fades_no_label_in_a_later_field() {
    let (_, out) = settle(&vega(), 600.0, |ui| {
        Field::new().horizontal().show(ui, |ui| {
            ui.add_enabled(false, Checkbox::new(&mut true));
            ui.add(Label::new("First"));
        });
        Field::new().horizontal().show(ui, |ui| {
            ui.add(Checkbox::new(&mut true));
            ui.add(Label::new("Second"));
        });
        ui.add(Label::new("Outside"));
    });
    assert_eq!(faded(&out, "First"), 0.5);
    assert_eq!(faded(&out, "Second"), 1.0);
    assert_eq!(faded(&out, "Outside"), 1.0);
}

/// A disabled switch is a peer as a checkbox is.
#[test]
fn a_label_after_a_disabled_switch_fades() {
    let (_, out) = settle(&vega(), 600.0, |ui| {
        Field::new().horizontal().show(ui, |ui| {
            ui.add(Label::new("Before"));
            ui.add_enabled(false, Switch::new(&mut true));
            ui.add(Label::new("After"));
        });
    });
    assert_eq!(faded(&out, "Before"), 1.0);
    assert_eq!(faded(&out, "After"), 0.5);
}
