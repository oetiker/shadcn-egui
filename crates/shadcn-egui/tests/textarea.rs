//! Textarea: behaviour (typing, Enter, Tab, disabled), growth with the
//! content, width, accessibility, and the snapshots of upstream's demos.

mod common;

use egui::accesskit::Role as AccessRole;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use shadcn_egui::components::textarea::{textarea, Textarea, TextareaVariant};
use shadcn_egui::style::{AppVariant, Paint, Role};
use shadcn_egui::Theme;

/// A harness that draws `ui_fn` over a `String` under vega.
fn harness(
    text: &str,
    ui_fn: impl FnMut(&mut egui::Ui, &mut String) + 'static,
) -> Harness<'static, String> {
    let mut ui_fn = ui_fn;
    let mut h = Harness::new_ui_state(
        move |ui, text: &mut String| {
            Theme::vega().apply(ui.ctx());
            ui_fn(ui, text);
        },
        text.to_owned(),
    );
    h.run();
    h
}

#[test]
fn typing_writes_the_binding_and_reports_changed() {
    let changed = std::rc::Rc::new(std::cell::Cell::new(false));
    let seen = changed.clone();
    let mut h = harness("", move |ui, text| {
        if textarea(ui, text).changed() {
            seen.set(true);
        }
    });
    h.get_by_role(AccessRole::MultilineTextInput).focus();
    h.run();
    h.get_by_role(AccessRole::MultilineTextInput)
        .type_text("hello");
    h.run();
    assert_eq!(h.state(), "hello");
    assert!(changed.get(), "changed() fires on typing");
}

#[test]
fn enter_starts_a_new_line() {
    let mut h = harness("first", |ui, text| {
        ui.add(Textarea::new(text));
    });
    h.get_by_role(AccessRole::MultilineTextInput).focus();
    h.run();
    h.key_press(egui::Key::End);
    h.key_press(egui::Key::Enter);
    h.run();
    h.get_by_role(AccessRole::MultilineTextInput)
        .type_text("second");
    h.run();
    assert_eq!(h.state(), "first\nsecond");
}

/// Upstream's `<textarea>`: Tab moves the focus on and inserts nothing
/// (D-113, CP-06-12).
#[test]
fn tab_moves_the_focus_and_inserts_nothing() {
    let mut h = harness("", |ui, text| {
        ui.add(Textarea::new(text).id_salt("first"));
        let mut other = String::new();
        ui.add(Textarea::new(&mut other).id_salt("second"));
    });
    h.get_all_by_role(AccessRole::MultilineTextInput)
        .next()
        .unwrap()
        .focus();
    h.run();
    h.key_press(egui::Key::Tab);
    h.run();
    assert_eq!(h.state(), "", "no tab character");
    let mut fields = h.get_all_by_role(AccessRole::MultilineTextInput);
    let (first, second) = (fields.next().unwrap(), fields.next().unwrap());
    assert!(!first.is_focused(), "the first field lost the focus");
    assert!(second.is_focused(), "the second field has it");
}

/// `disabled` comes from the `Ui`: a disabled field takes no typing.
#[test]
fn a_disabled_ui_ignores_typing() {
    let mut h = harness("", |ui, text| {
        ui.add_enabled(false, Textarea::new(text));
    });
    h.get_by_role(AccessRole::MultilineTextInput).focus();
    h.run();
    h.get_by_role(AccessRole::MultilineTextInput)
        .type_text("nope");
    h.run();
    assert_eq!(h.state(), "");
}

/// `show` returns egui's `TextEditOutput`: the response and, while the
/// field has the focus, the cursor.
#[test]
fn show_returns_the_text_edit_output() {
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    let mut text = String::from("abc");
    let mut cursor = None;
    for _ in 0..3 {
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            let out = Textarea::new(&mut text).id_salt("out").show(ui);
            out.response.response.request_focus();
            cursor = out.cursor_range;
            assert_eq!(out.galley.text(), "abc");
        });
    }
    assert!(cursor.is_some(), "a focused field reports its cursor");
}

/// `min-h-16`: an empty or short field is 64 px tall in every style.
#[test]
fn an_empty_field_takes_the_minimum_height() {
    for (name, theme, mode) in common::snapshot_themes() {
        let mut text = String::new();
        let h = common::measure(&theme, mode, |ui| ui.add(Textarea::new(&mut text))).height();
        assert_eq!(h, 64.0, "{name}");
        let mut short = String::from("one line");
        let h = common::measure(&theme, mode, |ui| ui.add(Textarea::new(&mut short))).height();
        assert_eq!(h, 64.0, "{name}: one line");
    }
}

/// `field-sizing-content`: past its minimum the field grows one line box
/// per line, its padding and border around them.
#[test]
fn the_field_grows_with_its_lines() {
    for (name, theme, mode) in common::themes() {
        let height = |lines: usize| {
            let mut text = vec!["line"; lines].join("\n");
            common::measure(&theme, mode, |ui| ui.add(Textarea::new(&mut text))).height()
        };
        let (five, six, seven) = (height(5), height(6), height(7));
        let line = six - five;
        assert!(line > 0.0, "{name}: grows");
        assert_eq!(seven - six, line, "{name}: one line box per line");
        // vega and nova: `py-2`, a 1 px border.
        assert_eq!(five, 5.0 * line + 18.0, "{name}: padding and border");
    }
}

/// Wrapped lines count as lines: a long text in a narrow field grows it.
#[test]
fn wrapped_text_grows_the_field() {
    let theme = Theme::vega();
    let mut text = "word ".repeat(40);
    let h = common::measure(&theme, egui::Theme::Light, |ui| {
        ui.add(Textarea::new(&mut text).desired_width(200.0))
    })
    .height();
    assert!(h > 100.0, "wraps and grows: {h}");
}

/// A word wider than the field breaks inside it, as Chrome's `<textarea>`
/// does (`overflow-wrap: break-word`, the UA default): the field keeps its
/// allocated width and grows in height; the box and the text stay inside
/// the allocation. A word that fits on a line by itself is moved down
/// whole, not broken.
#[test]
fn a_long_word_breaks_inside_the_field() {
    let theme = Theme::vega();
    let mut text = format!("see https://{}", "x".repeat(80));
    let mut start = egui::Pos2::ZERO;
    let mut galley = None;
    let (rect, out) = common::measure_output(&theme, egui::Theme::Light, |ui| {
        start = ui.cursor().min;
        let out = Textarea::new(&mut text).desired_width(200.0).show(ui);
        galley = Some(out.galley.clone());
        out.response.response
    });
    assert_eq!(rect.left(), start.x, "{rect:?}");
    assert_eq!(rect.width(), 200.0, "{rect:?}");
    let galley = galley.unwrap();
    assert!(galley.rows.len() > 3, "{} rows", galley.rows.len());
    assert!(galley.size().x <= 200.0, "galley {:?}", galley.size());
    // `see` stays alone on the first row: the URL fits no line beside it.
    assert_eq!(galley.rows[0].text().trim_end(), "see");
    assert!(galley.rows[1].text().starts_with("https"));
    for r in common::rect_shapes(&out) {
        assert!(
            r.rect.left() >= rect.left() && r.rect.right() <= rect.right(),
            "box {:?} outside {rect:?}",
            r.rect
        );
    }
    let tall = common::measure(&theme, egui::Theme::Light, |ui| {
        ui.add(Textarea::new(&mut text).desired_width(200.0))
    });
    let mut one_row = String::from("x");
    let one = common::measure(&theme, egui::Theme::Light, |ui| {
        ui.add(Textarea::new(&mut one_row).desired_width(200.0))
    });
    // The field holds every row: vega's `py-2` and border add 18.
    assert_eq!(tall.height(), galley.size().y + 18.0, "{tall:?}");
    assert!(tall.height() > one.height(), "{tall:?} {one:?}");
}

/// At its content minimum (the padding box around one `M`) the field
/// breaks a word after each char and keeps its box at the allocated rect.
#[test]
fn a_word_breaks_at_the_content_minimum() {
    let theme = Theme::vega();
    let mut text = String::from("MMMM");
    let mut start = egui::Pos2::ZERO;
    let (rect, out) = common::measure_output(&theme, egui::Theme::Light, |ui| {
        start = ui.cursor().min;
        ui.add(Textarea::new(&mut text).desired_width(0.0))
    });
    assert_eq!(rect.left(), start.x, "{rect:?}");
    for r in common::rect_shapes(&out) {
        assert!(
            r.rect.left() >= rect.left() && r.rect.right() <= rect.right(),
            "box {:?} outside {rect:?}",
            r.rect
        );
    }
}

/// While empty, the field grows to hold its wrapped hint text, as Chrome's
/// `field-sizing: content` sizes an empty field by its placeholder.
#[test]
fn a_long_hint_grows_the_empty_field() {
    let theme = Theme::vega();
    let hint = "a long hint ".repeat(30);
    let mut text = String::new();
    let h = common::measure(&theme, egui::Theme::Light, |ui| {
        ui.add(
            Textarea::new(&mut text)
                .hint_text(hint.as_str())
                .desired_width(200.0),
        )
    })
    .height();
    assert!(h > 100.0, "grows with the hint: {h}");
}

/// Text entry fills up to `Metrics::field_max_width`; `.desired_width`
/// changes the cap, `f32::INFINITY` fills.
#[test]
fn width_caps_at_the_field_maximum_unless_desired() {
    let theme = Theme::vega();
    let max = theme.metrics.field_max_width;
    let mut text = String::new();
    let w = common::measure(&theme, egui::Theme::Light, |ui| {
        ui.add(Textarea::new(&mut text))
    })
    .width();
    assert_eq!(w, max);
    let mut avail = 0.0;
    let w = common::measure(&theme, egui::Theme::Light, |ui| {
        avail = ui.available_width();
        ui.add(Textarea::new(&mut text).desired_width(f32::INFINITY))
    })
    .width();
    assert!(w > max, "fills: {w}");
    assert_eq!(w, avail);
}

/// A field raised to its content minimum overflows a column narrower than
/// that minimum without widening it (conformance 3.6).
#[test]
fn the_field_overflows_a_narrow_column_without_widening_it() {
    let ctx = egui::Context::default();
    Theme::vega().apply(&ctx);
    let mut text = String::new();
    let mut widths = (0.0, 0.0);
    for _ in 0..2 {
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.allocate_ui(egui::vec2(10.0, 100.0), |ui| {
                ui.set_max_width(10.0);
                let field = ui.add(Textarea::new(&mut text));
                widths = (field.rect.width(), ui.min_rect().width());
            });
        });
    }
    assert!(widths.0 > 10.0, "the field keeps its minimum: {widths:?}");
    assert_eq!(widths.1, 10.0, "the column keeps its width");
}

/// The box's 1 px border colour, the last one painted.
fn border(shapes: &[egui::epaint::RectShape]) -> Option<egui::Color32> {
    shapes
        .iter()
        .rev()
        .find(|r| r.stroke.width == 1.0)
        .map(|r| r.stroke.color)
}

/// `border-input focus-visible:border-ring focus-visible:ring-3
/// ring-ring/50`: the border is input-coloured at rest; focus turns it
/// ring-coloured and adds a 3 px ring at half alpha. A text field matches
/// `:focus-visible` after a pointer press too.
#[test]
fn a_focused_field_shows_the_ring() {
    for (name, theme, mode) in common::themes() {
        let p = theme.palette_of(mode);
        let mut text = String::new();
        let (rect, idle) = common::measure_output(&theme, mode, |ui| {
            ui.add(Textarea::new(&mut text).id_salt("field"))
        });
        let idle = common::rect_shapes(&idle);
        assert_eq!(border(&idle), Some(p.input), "{name} idle: {idle:?}");
        assert!(
            idle.iter().all(|r| r.stroke.width != 3.0),
            "{name}: ring while idle"
        );
        let (_, focused) = common::measure_output(&theme, mode, |ui| {
            let resp = ui.add(Textarea::new(&mut text).id_salt("field"));
            resp.request_focus();
            resp
        });
        let focused = common::rect_shapes(&focused);
        assert_eq!(
            border(&focused),
            Some(p.ring),
            "{name} focused: {focused:?}"
        );
        let ring = focused
            .iter()
            .find(|r| r.stroke.width == 3.0)
            .unwrap_or_else(|| panic!("{name}: no ring in {focused:?}"));
        assert_eq!(ring.stroke.color, p.ring.gamma_multiply(0.5), "{name}");
        assert_eq!(ring.rect, rect, "{name}");
    }

    // A pointer press focuses the field and shows the ring.
    let theme = Theme::vega();
    let ring = theme.palette_of(egui::Theme::Light).ring;
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    ctx.set_theme(egui::Theme::Light);
    ctx.all_styles_mut(|s| s.animation_time = 0.0);
    let mut text = String::new();
    let mut frame = |events: Vec<egui::Event>| {
        let input = egui::RawInput {
            events,
            ..Default::default()
        };
        let mut rect = egui::Rect::NOTHING;
        let out = ctx.run_ui(input, |ui| rect = ui.add(Textarea::new(&mut text)).rect);
        (rect, out)
    };
    let (rect, _) = frame(Vec::new());
    let _ = frame(common::click(rect.center()));
    let (_, out) = frame(Vec::new());
    let shapes = common::rect_shapes(&out);
    assert_eq!(border(&shapes), Some(ring), "pointer focus: {shapes:?}");
    assert!(shapes.iter().any(|r| r.stroke.width == 3.0), "{shapes:?}");
}

/// `aria-invalid:border-destructive aria-invalid:ring-3`: dark themes use
/// the `/50` border and `/40` ring.
#[test]
fn an_invalid_field_shows_the_destructive_border_and_ring() {
    for (name, theme, mode) in common::themes() {
        let mut text = String::new();
        let (_, out) = common::measure_output(&theme, mode, |ui| {
            ui.add(Textarea::new(&mut text).invalid(true))
        });
        let shapes = common::rect_shapes(&out);
        let d = theme.palette_of(mode).destructive;
        let (border, ring) = if name.ends_with("dark") {
            (d.gamma_multiply(0.5), d.gamma_multiply(0.4))
        } else {
            (d, d.gamma_multiply(0.2))
        };
        assert!(
            shapes
                .iter()
                .any(|r| r.stroke.width == 1.0 && r.stroke.color == border),
            "{name}: border in {shapes:?}"
        );
        assert!(
            shapes
                .iter()
                .any(|r| r.stroke.width == 3.0 && r.stroke.color == ring),
            "{name}: ring in {shapes:?}"
        );
    }
}

/// `disabled:opacity-50`, and in nova and lyra `disabled:bg-input/50`: a
/// disabled `Ui` paints the text at the part's disabled opacity and the
/// box in its disabled fill at the same opacity.
#[test]
fn a_disabled_ui_fades_the_field() {
    use shadcn_egui::style::{pick, State};
    for (name, theme, mode) in common::themes() {
        let layers = [&*theme.style.textarea];
        let dark = mode == egui::Theme::Dark;
        let disabled = State::DISABLED.with(State::DARK, dark);
        let opacity = pick(&layers, |r| &r.opacity, disabled).expect("disabled opacity");
        let mut text = String::from("abc");
        let (_, on) = common::measure_output(&theme, mode, |ui| ui.add(Textarea::new(&mut text)));
        let (rect, off) = common::measure_output(&theme, mode, |ui| {
            ui.add_enabled(false, Textarea::new(&mut text))
        });
        assert_eq!(
            common::text_colour(&off),
            common::text_colour(&on).gamma_multiply(opacity),
            "{name}"
        );
        let r = common::resolved(&theme, mode);
        let fill = pick(&layers, |r| &r.bg, disabled).map_or(egui::Color32::TRANSPARENT, |p| {
            r.paint(p).gamma_multiply(opacity)
        });
        let painted = common::rect_shapes(&off)
            .into_iter()
            .find(|s| s.rect == rect && s.fill != egui::Color32::TRANSPARENT)
            .map_or(egui::Color32::TRANSPARENT, |s| s.fill);
        assert_eq!(painted, fill, "{name}");
    }
    // nova and lyra fill the disabled box; vega does not.
    use shadcn_egui::style::Style;
    let bg = |t: &Style, s: State| pick(&[&*t.textarea], |r| &r.bg, s);
    let (nova, vega) = (Style::nova(), Style::vega());
    assert_ne!(bg(&nova, State::DISABLED), bg(&nova, State::NONE));
    assert_eq!(bg(&vega, State::DISABLED), bg(&vega, State::NONE));
}

/// shadcn textarea `text-base md:text-sm`: the field text is 16 px in a
/// window narrower than 768 px, 14 px from 768 px up.
#[test]
fn the_text_follows_the_md_breakpoint() {
    let theme = Theme::vega();
    let text_px = |width: f32| {
        let ctx = egui::Context::default();
        theme.apply(&ctx);
        ctx.set_theme(egui::Theme::Light);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 400.0),
            )),
            ..Default::default()
        };
        let mut text = String::from("Hello");
        let mut out = None;
        for _ in 0..2 {
            out = Some(ctx.run_ui(input.clone(), |ui| {
                ui.add(Textarea::new(&mut text));
            }));
        }
        common::text_shapes(&out.unwrap())
            .iter()
            .map(|t| t.galley.job.sections[0].format.font_id.size)
            .next()
            .expect("the field's text")
    };
    assert_eq!(text_px(767.0), theme.metrics.text_base);
    assert_eq!(text_px(768.0), theme.metrics.text_sm);
}

/// The empty field shows its hint in the style's placeholder colour.
#[test]
fn the_hint_takes_the_placeholder_colour() {
    let theme = Theme::vega();
    let mut text = String::new();
    let (_, out) = common::measure_output(&theme, egui::Theme::Light, |ui| {
        ui.add(Textarea::new(&mut text).hint_text("Type here"))
    });
    let want = common::resolved(&theme, egui::Theme::Light).role(Role::MutedForeground);
    assert_eq!(common::text_colour(&out), want);
}

/// A recipe without a placeholder colour shows the hint in upstream's
/// `placeholder:text-muted-foreground`, never in an egui colour
/// (conformance 2.1).
#[test]
fn a_recipe_without_a_placeholder_colour_falls_back_to_muted() {
    const PLAIN: AppVariant<TextareaVariant> = TextareaVariant::app("plain");
    let theme = Theme::vega().variant(PLAIN, TextareaVariant::Default, |r, _| {
        r.textarea.text = shadcn_egui::style::Prop::default();
    });
    let mut text = String::new();
    let (_, out) = common::measure_output(&theme, egui::Theme::Light, |ui| {
        ui.add(
            Textarea::new(&mut text)
                .hint_text("Type here")
                .variant(PLAIN),
        )
    });
    let want = common::resolved(&theme, egui::Theme::Light).role(Role::MutedForeground);
    assert_eq!(common::text_colour(&out), want);
}

/// An app variant's edit reaches the drawn field.
#[test]
fn an_app_variant_recolours_the_field() {
    const BRAND: AppVariant<TextareaVariant> = TextareaVariant::app("brand");
    let theme = Theme::vega().variant(BRAND, TextareaVariant::Default, |r, _| {
        r.textarea.text.set(Paint::Role(Role::Destructive, 1.0))
    });
    let mut text = String::from("typed");
    let (_, out) = common::measure_output(&theme, egui::Theme::Light, |ui| {
        ui.add(Textarea::new(&mut text).variant(BRAND))
    });
    let want = common::resolved(&theme, egui::Theme::Light).role(Role::Destructive);
    assert_eq!(common::text_colour(&out), want);
}

/// A recipe without `field-sizing-content` cannot be drawn (no scrolling
/// field): it fails a `debug_assert!` instead of drawing silently wrong.
#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "grows with its content")]
fn a_recipe_that_does_not_grow_fails_a_debug_assert() {
    const FIXED: AppVariant<TextareaVariant> = TextareaVariant::app("fixed");
    let theme = Theme::vega().variant(FIXED, TextareaVariant::Default, |r, _| {
        r.textarea.field_sizing_content.set(false)
    });
    let mut text = String::new();
    let _ = common::measure(&theme, egui::Theme::Light, |ui| {
        ui.add(Textarea::new(&mut text).variant(FIXED))
    });
}

/// Upstream's textarea-demo and textarea-button, then the disabled
/// (`disabled` from the `Ui`) and invalid (`aria-invalid`) states on the
/// bare field, a filled field with a wrapped and a broken line, and a
/// focused one. textarea-field, -disabled and -invalid wrap the field in
/// Field, which waits for the Field port.
#[test]
fn textarea_snapshot() {
    use shadcn_egui::components::button::Button;
    // `text-base md:text-sm`: the phone and the desktop text size.
    common::snapshot_widths("textarea", 700.0, common::snapshot_themes(), |ui| {
        let mut demo = String::new();
        let mut button = String::new();
        let mut disabled = String::new();
        let mut invalid = String::new();
        let mut filled = String::from(
            "A message long enough to wrap onto a second line in the phone layout, \
             and a line of its own:\nthe last line.",
        );
        let mut focused = String::from("typing here");
        let hint = "Type your message here.";
        ui.add(Textarea::new(&mut demo).hint_text(hint).id_salt("demo"));
        ui.add_space(16.0);
        // textarea-button: `grid w-full gap-2`.
        shadcn_egui::layout::vstack(ui, 8.0, |ui| {
            ui.add(Textarea::new(&mut button).hint_text(hint).id_salt("button"));
            ui.add(Button::new("Send message"));
        });
        ui.add_space(16.0);
        ui.add_enabled(
            false,
            Textarea::new(&mut disabled)
                .hint_text(hint)
                .id_salt("disabled"),
        );
        ui.add_space(16.0);
        ui.add(
            Textarea::new(&mut invalid)
                .hint_text(hint)
                .invalid(true)
                .id_salt("invalid"),
        );
        ui.add_space(16.0);
        ui.add(Textarea::new(&mut filled).id_salt("filled"));
        ui.add_space(16.0);
        let resp = ui.add(Textarea::new(&mut focused).id_salt("focused"));
        resp.request_focus();
    });
}
