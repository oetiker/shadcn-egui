//! Flex layouts and button sizes against Chrome at device scale 1, 1.5 and
//! 2. The reference rows (`fixtures/chrome-layout.tsv`, written by
//! `scripts/chrome-layout.mjs`) hold, per case, scale and item, the box
//! Chrome lays out in CSS px from the case's top left; the crate builds
//! the same cases with its layout helpers and its `Button`.

use egui::{vec2, Rect, Sense, Ui};
use egui_extras::{Size, StripBuilder};
use egui_kittest::Harness;
use shadcn_egui::components::button::{Button, ButtonSize};
use shadcn_egui::components::label::description;
use shadcn_egui::layout::{allocate_overflowing, row, space_between, vstack};
use shadcn_egui::style::Style;
use shadcn_egui::{Palette, Theme};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

const CHROME: &str = include_str!("fixtures/chrome-layout.tsv");
/// Keep in step with the texts in `scripts/chrome-layout.mjs`.
const ASK: &str = "Choose what you want to hear about.";
const MAIL: &str = "Receive emails about new products.";
const WORD: &str = "Accessibility";
/// Chrome keeps boxes on LayoutUnits (1/64 px); text widths agree to
/// that, so a box may differ by one LayoutUnit per edge it sums.
const TOLERANCE: f32 = 2.0 / 64.0;

type Items = Rc<RefCell<BTreeMap<String, Rect>>>;

/// A 60x20 box, as an egui widget: it keeps its size in a row.
fn boxed(ui: &mut Ui, items: &Items) {
    let rect = ui.allocate_exact_size(vec2(60.0, 20.0), Sense::hover()).0;
    items.borrow_mut().insert("box".into(), rect);
}

/// A vstack with one paragraph, recorded as `stack` and `text`.
fn para(ui: &mut Ui, items: &Items, n: &str, text: &str) {
    let r = ui.scope(|ui| {
        vstack(ui, 4.0, |ui| {
            let t = description(ui, text).rect;
            items.borrow_mut().insert(format!("text{n}"), t);
        })
    });
    items
        .borrow_mut()
        .insert(format!("stack{n}"), r.response.rect);
}

fn build(case: &str, ui: &mut Ui, items: &Items) {
    match case {
        "row-text" => {
            row(ui, 8.0, |ui| {
                para(ui, items, "", ASK);
                boxed(ui, items);
            });
        }
        "between-text" => {
            ui.spacing_mut().item_spacing.x = 8.0;
            space_between(ui, |ui| para(ui, items, "", ASK), |ui| boxed(ui, items));
        }
        "row-two" | "row-min" => {
            let first = if case == "row-two" { ASK } else { WORD };
            row(ui, 8.0, |ui| {
                para(ui, items, "-1", first);
                para(ui, items, "-2", MAIL);
                boxed(ui, items);
            });
        }
        "max-content" => {
            vstack(ui, 4.0, |ui| {
                for (n, text) in [("-1", ASK), ("-2", MAIL), ("-3", WORD)] {
                    let t = description(ui, text).rect;
                    items.borrow_mut().insert(format!("text{n}"), t);
                }
            });
        }
        "cell-text" => {
            row(ui, 8.0, |ui| {
                StripBuilder::new(ui)
                    .size(Size::exact(100.0))
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            items.borrow_mut().insert("cell".into(), ui.max_rect());
                            para(ui, items, "", ASK);
                        });
                    });
                boxed(ui, items);
            });
        }
        "cell-overflow" => {
            row(ui, 8.0, |ui| {
                StripBuilder::new(ui)
                    .size(Size::exact(100.0))
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            items.borrow_mut().insert("cell".into(), ui.max_rect());
                            let wide = allocate_overflowing(ui, vec2(150.0, 20.0), Sense::hover());
                            items.borrow_mut().insert("wide".into(), wide.0);
                        });
                    });
                boxed(ui, items);
            });
        }
        "strip-text" => {
            ui.allocate_ui(vec2(ui.available_width(), 60.0), |ui| {
                // The grid has no gap (build-ui layout map: `gap-N` is the
                // strip's item spacing, none is 0).
                ui.spacing_mut().item_spacing.x = 0.0;
                StripBuilder::new(ui)
                    .size(Size::exact(100.0))
                    .size(Size::remainder())
                    .horizontal(|mut strip| {
                        strip.cell(|ui| para(ui, items, "-1", ASK));
                        strip.cell(|ui| para(ui, items, "-2", MAIL));
                    });
            });
        }
        _ => panic!("no crate case {case}"),
    }
}

/// `case` laid out in a column `width` wide at `scale`: item boxes from
/// the column's top left.
fn crate_case(scale: f32, width: f32, build: impl Fn(&mut Ui, &Items) + 'static) -> Items {
    let items: Items = Rc::default();
    let out = items.clone();
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_pixels_per_point(scale)
        .build_ui(move |ui| {
            out.borrow_mut().clear();
            let max = Rect::from_min_size(ui.cursor().min, vec2(width, 280.0));
            let mut column = ui.new_child(egui::UiBuilder::new().max_rect(max));
            build(&mut column, &out);
            let mut o = out.borrow_mut();
            for r in o.values_mut() {
                *r = r.translate(-max.min.to_vec2());
            }
        });
    Theme::vega().apply(&h.ctx);
    h.ctx.set_theme(egui::Theme::Light);
    for _ in 0..8 {
        h.step();
    }
    items
}

/// Chrome's rows: (case, scale) -> item -> (x, y, w, h).
fn chrome() -> BTreeMap<(String, String), BTreeMap<String, [f32; 4]>> {
    let mut out: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
    for line in CHROME.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        let v = |i: usize| f[i].parse::<f32>().expect("number");
        out.entry((f[0].to_string(), f[1].to_string()))
            .or_default()
            .insert(f[2].to_string(), [v(3), v(4), v(5), v(6)]);
    }
    out
}

fn width_of(case: &str) -> f32 {
    match case {
        "row-two" | "strip-text" => 300.0,
        "row-min" => 180.0,
        "max-content" => 400.0,
        _ => 220.0,
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOLERANCE
}

/// A vstack in a crate row shrinks as Chrome's flex item and its text
/// wraps where Chrome's does; a vstack in a `StripBuilder` cell keeps to
/// the cell.
#[test]
fn flex_columns_shrink_as_chromes() {
    let mut bad = Vec::new();
    let mut compared = 0;
    for ((case, scale), want) in chrome() {
        if case.starts_with("button-") {
            continue;
        }
        let s: f32 = scale.parse().expect("scale");
        let c = case.clone();
        let got = crate_case(s, width_of(&case), move |ui, items| build(&c, ui, items));
        let got = got.borrow();
        for (item, [x, _y, w, h]) in want {
            let g = got.get(&item).unwrap_or_else(|| panic!("{case} {item}"));
            // A text's own box is its wrapped lines, not the column's width.
            let (gx, gw) = (g.left(), g.width());
            // A stack's own height is the row's in egui (a centred child),
            // and a strip cell's the strip's, which fills the row (at least
            // `interact_size.y`); a text's height counts the lines.
            let ok = if case == "max-content" {
                close(gx, x) && close(gw, w) && close(g.height(), h)
            } else if item.starts_with("text") {
                close(gx, x) && close(g.height(), h)
            } else if item.starts_with("stack") || item == "cell" {
                close(gx, x) && close(gw, w)
            } else {
                close(gx, x) && close(gw, w) && close(g.height(), h)
            };
            compared += 1;
            if !ok {
                bad.push(format!(
                    "{case} {scale}x {item}: crate x {gx} w {gw} h {}, chrome x {x} w {w} h {h}",
                    g.height()
                ));
            }
        }
    }
    assert!(compared > 0);
    assert!(bad.is_empty(), "{} differ:\n{}", bad.len(), bad.join("\n"));
}

/// `first` in the first stack of `row-two` for a few frames, then `ASK`:
/// the row ends as Chrome's `row-two`. Returns the text's size before the
/// change and in the frame of the change.
fn relearn_case(first: &'static str) -> (egui::Vec2, egui::Vec2) {
    let want = &chrome()[&("row-two".to_string(), "1".to_string())];
    let items: Items = Rc::default();
    let long = Rc::new(std::cell::Cell::new(false));
    let (out, flag) = (items.clone(), long.clone());
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .build_ui(move |ui| {
            out.borrow_mut().clear();
            let max = Rect::from_min_size(ui.cursor().min, vec2(300.0, 280.0));
            let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(max));
            row(&mut ui, 8.0, |ui| {
                para(ui, &out, "-1", if flag.get() { ASK } else { first });
                para(ui, &out, "-2", MAIL);
                boxed(ui, &out);
            });
            let mut o = out.borrow_mut();
            for r in o.values_mut() {
                *r = r.translate(-max.min.to_vec2());
            }
        });
    Theme::vega().apply(&h.ctx);
    h.ctx.set_theme(egui::Theme::Light);
    for _ in 0..4 {
        h.step();
    }
    let before = items.borrow()["text-1"].size();
    long.set(true);
    h.step();
    let switched = items.borrow()["text-1"].size();
    for _ in 0..4 {
        h.step();
    }
    for item in ["stack-1", "stack-2"] {
        let (g, [x, _, w, _]) = (items.borrow()[item], want[item]);
        assert!(
            close(g.left(), x) && close(g.width(), w),
            "{first:?} -> {item}: crate x {} w {}, chrome x {x} w {w} \
             (text {before:?}, then {switched:?} at the old width)",
            g.left(),
            g.width()
        );
    }
    (before, switched)
}

/// A stack measures its unwrapped width again when its text changes: the
/// first text, one word, is replaced by `row-two`'s, and the row ends as
/// Chrome's `row-two`. (Every case above also starts with egui's own font
/// before the theme's, the same relearning.)
#[test]
fn a_stack_relearns_its_width_when_its_text_changes() {
    relearn_case(WORD);
}

/// The same where the new text wraps to the very size of the old one at
/// the old width (87 x 60: the widest line is kept, a word on the last
/// line changes), so only the painted text tells the stack it changed.
#[test]
fn a_stack_relearns_its_width_when_its_text_keeps_its_wrapped_size() {
    let (before, switched) = relearn_case("Choose what you want to hear it.");
    assert_eq!(before, switched, "the case needs an equal wrapped size");
}

fn button_case(case: &str) -> (Theme, Button) {
    let mut parts = case.strip_prefix("button-").expect("button").splitn(3, '-');
    let (style, content, size) = (
        parts.next().expect("style"),
        parts.next().expect("content"),
        parts.next().expect("size"),
    );
    let style = match style {
        "vega" => Style::vega(),
        "nova" => Style::nova(),
        _ => panic!("{style}"),
    };
    let size = match size {
        "sm" => ButtonSize::Sm,
        "default" => ButtonSize::Default,
        "lg" => ButtonSize::Lg,
        "icon-sm" => ButtonSize::IconSm,
        "icon" => ButtonSize::Icon,
        "icon-lg" => ButtonSize::IconLg,
        _ => panic!("{size}"),
    };
    let icon = &shadcn_egui::icons::lucide::CHEVRON_RIGHT;
    let b = Button::new("Continue").size(size);
    let b = match content {
        "text" => b,
        "start" | "icon" => b.icon(icon),
        "both" => b.icon(icon).icon_end(icon),
        _ => panic!("{content}"),
    };
    (
        Theme::new(style, Palette::light(), Palette::dark(), 10.0),
        b,
    )
}

/// Button boxes (text only, icon and text, icon text icon, icon only) of
/// two styles at each size equal Chrome's.
#[test]
fn button_boxes_equal_chromes() {
    let mut bad = Vec::new();
    let mut compared = 0;
    for ((case, scale), want) in chrome() {
        if !case.starts_with("button-") {
            continue;
        }
        let s: f32 = scale.parse().expect("scale");
        let [_, _, w, h] = want["button"];
        let (theme, _) = button_case(&case);
        let rect = Rc::new(RefCell::new(Rect::NOTHING));
        let out = rect.clone();
        let c = case.clone();
        let mut harness = Harness::builder()
            .with_size(vec2(400.0, 100.0))
            .with_pixels_per_point(s)
            .build_ui(move |ui| {
                let (_, b) = button_case(&c);
                *out.borrow_mut() = ui.add(b).rect;
            });
        theme.apply(&harness.ctx);
        for _ in 0..4 {
            harness.step();
        }
        let g = *rect.borrow();
        compared += 1;
        if !close(g.width(), w) || !close(g.height(), h) {
            bad.push(format!(
                "{case} {scale}x: crate {} x {}, chrome {w} x {h}",
                g.width(),
                g.height()
            ));
        }
    }
    // 2 styles x 12 buttons x 3 scales.
    assert_eq!(compared, 72);
    assert!(bad.is_empty(), "{} differ:\n{}", bad.len(), bad.join("\n"));
}
