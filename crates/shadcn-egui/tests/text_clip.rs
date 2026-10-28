//! Where a text-entry field clips its text, its hint and its caret, as
//! Chrome does: an `<input>` at its content box, a `<textarea>` at its
//! padding box. A full input scrolls so a caret at the end of its text
//! stays inside the content box.

use egui::accesskit::Role as AccessRole;
use egui::{Rect, Shape};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use shadcn_egui::components::input::Input;
use shadcn_egui::components::textarea::Textarea;
use shadcn_egui::Theme;

/// Vega's fields draw a 1px border (`border`).
const BORDER: f32 = 1.0;

/// A shape's clip rect and its rect.
type Clipped = (Rect, Rect);

/// Every text shape (its galley's place) and every rect shape of a frame,
/// with its clip rect.
fn clipped(out: &egui::FullOutput) -> (Vec<Clipped>, Vec<Clipped>) {
    fn walk(clip: Rect, s: &Shape, texts: &mut Vec<Clipped>, rects: &mut Vec<Clipped>) {
        match s {
            Shape::Text(t) => texts.push((clip, Rect::from_min_size(t.pos, t.galley.size()))),
            Shape::Rect(r) => rects.push((clip, r.rect)),
            Shape::Vec(v) => v.iter().for_each(|s| walk(clip, s, texts, rects)),
            _ => {}
        }
    }
    let (mut texts, mut rects) = (Vec::new(), Vec::new());
    for c in &out.shapes {
        walk(c.clip_rect, &c.shape, &mut texts, &mut rects);
    }
    (texts, rects)
}

/// A harness under vega that draws one field, at its content minimum, at
/// `ppp`. `field` returns the field's response; the harness state is its
/// border box.
fn harness(
    ppp: f32,
    mut field: impl FnMut(&mut egui::Ui) -> egui::Response + 'static,
) -> Harness<'static, Rect> {
    let mut h = Harness::builder()
        .with_size(egui::vec2(200.0, 200.0))
        .with_pixels_per_point(ppp)
        .build_ui_state(
            move |ui, rect: &mut Rect| {
                Theme::vega().apply(ui.ctx());
                ui.ctx().all_styles_mut(|s| s.animation_time = 0.0);
                *rect = field(ui).rect;
            },
            Rect::NOTHING,
        );
    h.run();
    h
}

/// The field's content box: its border box less the side insets, read
/// from where its unscrolled text starts (vega's fields have equal side
/// insets).
fn content_box(rect: Rect, text_left: f32) -> Rect {
    let inset = text_left - rect.left();
    Rect::from_x_y_ranges(rect.left() + inset..=rect.right() - inset, rect.y_range())
}

fn assert_clip_x(what: &str, clip: Rect, want: Rect) {
    assert!(
        (clip.left() - want.left()).abs() < 0.01 && (clip.right() - want.right()).abs() < 0.01,
        "{what}: clip {clip:?}, want x {:?}",
        want.x_range()
    );
}

/// An input's hint and typed text are clipped at the content box: a `W`
/// that overhangs the field's one-`M` content minimum stops at the
/// content box, not in the padding.
#[test]
fn an_input_clips_its_text_and_hint_at_the_content_box() {
    for ppp in [1.0, 2.0] {
        let h = harness(ppp, |ui| {
            let mut empty = String::new();
            ui.add(Input::new(&mut empty).hint("WWW").max_width(0.0))
        });
        let rect = *h.state();
        let (texts, _) = clipped(h.output());
        let hint = texts
            .iter()
            .find(|(_, b)| b.width() > 0.0)
            .expect("hint drawn");
        let content = content_box(rect, hint.1.left());
        for (clip, _) in &texts {
            assert_clip_x(&format!("{ppp}x hint"), *clip, content);
        }

        let h = harness(ppp, |ui| {
            let mut typed = String::from("WWWWWW");
            ui.add(Input::new(&mut typed).max_width(0.0))
        });
        let (texts, _) = clipped(h.output());
        assert!(!texts.is_empty(), "{ppp}x: typed text drawn");
        for (clip, _) in &texts {
            assert_clip_x(&format!("{ppp}x typed"), *clip, content);
        }
    }
}

/// The caret has the text's clip, and a full input scrolls its text so a
/// caret at the end stays whole inside the content box, as in Chrome.
#[test]
fn a_caret_at_the_end_of_a_full_input_is_inside_the_content_box() {
    for ppp in [1.0, 2.0] {
        let h = harness(ppp, |ui| {
            let mut empty = String::new();
            ui.add(Input::new(&mut empty).hint("M").max_width(0.0))
        });
        let (texts, _) = clipped(h.output());
        let hint = texts
            .iter()
            .find(|(_, b)| b.width() > 0.0)
            .expect("hint drawn");
        let content = content_box(*h.state(), hint.1.left());

        let mut h = harness(ppp, |ui| {
            let mut typed = String::from("WWWWWW");
            ui.add(Input::new(&mut typed).max_width(0.0))
        });
        h.get_by_role(AccessRole::TextInput).focus();
        h.run();
        h.key_press(egui::Key::End);
        h.run();
        let (texts, rects) = clipped(h.output());
        let caret = rects
            .iter()
            .find(|(_, r)| r.width() * ppp <= 2.5 && r.height() > 4.0 && content.intersects(*r))
            .unwrap_or_else(|| panic!("{ppp}x: no caret drawn in {rects:?}"));
        let (clip, r) = *caret;
        assert_eq!(clip, texts[0].0, "{ppp}x: the caret has the text's clip");
        assert!(
            r.left() >= content.left() && r.right() <= content.right(),
            "{ppp}x: caret {r:?} outside the content box {:?}",
            content.x_range()
        );
        assert!(
            clip.contains_rect(r),
            "{ppp}x: caret {r:?} cut by its clip {clip:?}"
        );
    }
}

/// A textarea's hint and typed text are clipped at the padding box: a
/// glyph wider than the content box paints into the padding, as in
/// Chrome, and stops at the border.
#[test]
fn a_textarea_clips_its_text_and_hint_at_the_padding_box() {
    for ppp in [1.0, 2.0] {
        for (typed, hint) in [("", "WWW"), ("WWW", "")] {
            let h = harness(ppp, move |ui| {
                let mut text = String::from(typed);
                ui.add(Textarea::new(&mut text).hint_text(hint).desired_width(0.0))
            });
            let rect = *h.state();
            let padding = rect.shrink(BORDER);
            let (texts, _) = clipped(h.output());
            assert!(!texts.is_empty(), "{ppp}x {typed:?}/{hint:?}: text drawn");
            for (clip, _) in &texts {
                assert_clip_x(&format!("{ppp}x {typed:?}/{hint:?}"), *clip, padding);
            }
        }
    }
}

/// The glyphs of every text shape a frame painted, as text, with its row
/// count.
fn painted_texts(out: &egui::FullOutput) -> Vec<(String, usize)> {
    fn walk(s: &Shape, texts: &mut Vec<(String, usize)>) {
        match s {
            Shape::Text(t) => {
                let glyphs = t.galley.rows.iter().flat_map(|r| &r.glyphs);
                let text: String = glyphs.map(|g| g.chr).collect();
                if !text.is_empty() {
                    texts.push((text, t.galley.rows.len()));
                }
            }
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, texts)),
            _ => {}
        }
    }
    let mut texts = Vec::new();
    for c in &out.shapes {
        walk(&c.shape, &mut texts);
    }
    texts
}

/// A hint too long for its field is cut, never elided: Chrome's
/// placeholder has `text-overflow: clip`, so an input shows the hint's
/// start on one line (line breaks stripped, as the HTML spec says for an
/// input's placeholder) up to the clip, and a textarea wraps it, a word
/// wider than the field broken, with no ellipsis in either (Chrome page
/// `infra/q9/ph.html`).
#[test]
fn a_hint_too_long_for_its_field_is_clipped_not_elided() {
    for ppp in [1.0, 2.0] {
        for (hint, want) in [
            (
                "Enter your email address please",
                "Enter your email address please",
            ),
            ("line one\nline two", "line oneline two"),
        ] {
            let h = harness(ppp, move |ui| {
                let mut empty = String::new();
                ui.add(Input::new(&mut empty).hint(hint).max_width(120.0))
            });
            assert_eq!(
                painted_texts(h.output()),
                vec![(want.to_owned(), 1)],
                "{ppp}x input hint {hint:?}"
            );
        }
        for hint in [
            "Enter your email address please and some more words",
            "Supercalifragilisticexpialidocious",
        ] {
            let h = harness(ppp, move |ui| {
                let mut empty = String::new();
                ui.add(
                    Textarea::new(&mut empty)
                        .hint_text(hint)
                        .desired_width(120.0),
                )
            });
            let texts = painted_texts(h.output());
            assert_eq!(texts.len(), 1, "{ppp}x textarea hint {hint:?}: {texts:?}");
            assert_eq!(
                texts[0].0.replace(' ', ""),
                hint.replace(' ', ""),
                "{ppp}x textarea hint shows all its text"
            );
            assert!(
                texts[0].1 > 1,
                "{ppp}x textarea hint {hint:?} wraps: {texts:?}"
            );
        }
    }
}
