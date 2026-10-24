//! Text placement against Chrome (conformance 2.4): for every font the
//! build has, at 1x, 1.5x and 2x, the text the crate draws must sit on the
//! device pixel rows where Chrome puts it:
//! - the baseline of "HHHH" in the default vega light Button, Badge and
//!   Input;
//! - the Input's selection highlight (all text selected) and its caret
//!   (after the text): their rows and the caret's width;
//! - each row's baseline in a block wrapped into three rows on 19.5 px
//!   lines (`text-xs/relaxed`), whose line tops are fractional.
//!
//! The reference rows are in `text_placement/chrome-baselines.tsv`
//! (device px from the element's border-box top), measured from headless
//! Chrome renders. Regenerate them after a font or style change with
//! `node scripts/chrome-text-baselines.mjs <chrome-binary>`.
//!
//! Everything is read from the frame's drawn shapes, as the tessellator
//! draws them (the galley origin rounded to a device pixel): the baseline
//! is the bottom edge of the ink of the row's glyph quads (an "H" has no
//! descender), from the font atlas by the Chrome script's ink rule, the
//! selection the box of the vertices in the selection
//! colour, the caret the filled rect in the field's text colour. No GPU
//! needed.

use egui::epaint::TextShape;
use egui::{pos2, Color32, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape};
use shadcn_egui::components::badge::Badge;
use shadcn_egui::components::button::{Button, ButtonSize};
use shadcn_egui::components::input::Input;
use shadcn_egui::components::text::TextLook;
use shadcn_egui::style::{Recipe, State, Style, TextSize};
use shadcn_egui::{fonts, FontFiles, FontName, Palette, Theme};

const CHROME: &str = include_str!("text_placement/chrome-baselines.tsv");
const SCALES: [f32; 3] = [1.0, 1.5, 2.0];
/// The TSV's measured columns, after font, scale, item and line_top.
const FIELDS: [&str; 6] = [
    "baseline",
    "sel_top",
    "sel_bottom",
    "caret_top",
    "caret_bottom",
    "caret_w",
];

/// Chrome's rows for `font` at `scale`: item and its measured columns
/// (`None` for "-").
fn chrome(font: &str, scale: f32) -> Vec<(String, [Option<i32>; 6])> {
    CHROME
        .lines()
        .skip(1)
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            (c[0] == font && c[1].parse::<f32>().ok() == Some(scale)).then(|| {
                let v = std::array::from_fn(|i| c[4 + i].parse().ok());
                (c[2].to_owned(), v)
            })
        })
        .collect()
}

fn walk(shape: &Shape, texts: &mut Vec<TextShape>, rects: &mut Vec<(Rect, Color32)>) {
    match shape {
        Shape::Text(t) => texts.push(t.clone()),
        Shape::Rect(r) => rects.push((r.rect, r.fill)),
        Shape::Vec(v) => v.iter().for_each(|s| walk(s, texts, rects)),
        _ => {}
    }
}

/// Device px of `y` points below the galley origin of `t`, which the
/// tessellator rounds to the pixel.
fn dev(t: &TextShape, y: f32, ppp: f32) -> f32 {
    (t.pos.y * ppp).round() + y * ppp
}

/// The bottom edge of the ink in each row of `t`, in device px, by the rule
/// the Chrome script uses: the last pixel row whose summed coverage over
/// the row's glyph quads reaches half the median of the rows with ink.
/// The coverage comes from the font atlas `atlas` at each quad's texels
/// (egui rasterizes glyphs at device px: one texel per pixel row).
fn row_baselines(t: &TextShape, ppp: f32, atlas: &egui::ColorImage) -> Vec<f32> {
    t.galley
        .rows
        .iter()
        .filter(|r| !r.row.glyphs.is_empty())
        .map(|r| {
            let mut rows = std::collections::BTreeMap::<i64, f32>::new();
            for quad in r.row.visuals.mesh.vertices.chunks(4) {
                let (top, bottom) = (
                    quad[0].pos.y.min(quad[3].pos.y),
                    quad[0].pos.y.max(quad[3].pos.y),
                );
                // Galley uvs are in texels; the tessellator normalizes them.
                let (u0, u1) = (
                    quad[0].uv.x.min(quad[3].uv.x),
                    quad[0].uv.x.max(quad[3].uv.x),
                );
                let v0 = quad[0].uv.y.min(quad[3].uv.y);
                let y0 = dev(t, r.pos.y + top, ppp).round() as i64;
                let n = ((bottom - top) * ppp).round() as i64;
                for k in 0..n {
                    let texel = (v0.round() as i64 + k) as usize;
                    let sum: f32 = (u0.round() as usize..u1.round() as usize)
                        .map(|x| f32::from(atlas.pixels[texel * atlas.size[0] + x].a()))
                        .sum();
                    *rows.entry(y0 + k).or_default() += sum;
                }
            }
            let mut ink: Vec<f32> = rows.values().copied().filter(|&v| v > 0.0).collect();
            ink.sort_by(f32::total_cmp);
            let threshold = 0.5 * ink[ink.len() / 2];
            let last = rows
                .iter()
                .filter(|(_, &v)| v >= threshold)
                .map(|(&y, _)| y)
                .max();
            (last.expect("ink") + 1) as f32
        })
        .collect()
}

struct Frame {
    texts: Vec<TextShape>,
    rects: Vec<(Rect, Color32)>,
    items: [Rect; 4],
}

/// What the crate draws for `files` at `ppp`: per TSV item, the measured
/// columns in device px below the item's top.
fn drawn(files: &'static FontFiles, ppp: f32) -> Vec<(String, [Option<f32>; 6])> {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(ppp);
    Theme::vega_light().fonts(files, None).apply(&ctx);
    let mut block = Recipe::default();
    block.text_size.base = Some(TextSize::Px(12.0));
    block.line_height.base = Some(1.625);
    let mut value = String::from("HHHH");
    let mut run = |events: Vec<Event>| {
        let mut items = [Rect::NOTHING; 4];
        let input = RawInput {
            events,
            ..Default::default()
        };
        let out = ctx.run_ui(input, |ui| {
            for (i, item) in items.iter_mut().enumerate() {
                let area =
                    egui::Area::new(egui::Id::new(i)).fixed_pos(pos2(8.0, 8.0 + 80.0 * i as f32));
                *item = area
                    .show(ui.ctx(), |ui| match i {
                        0 => ui.add(Button::new("HHHH")),
                        1 => ui.add(Badge::new("HHHH")),
                        2 => ui.add(Input::new(&mut value).max_width(200.0)),
                        _ => {
                            // One word per row, as `width: min-content`.
                            let look = TextLook::resolve(ui, &[&block], State::NONE);
                            let wrap = egui::TextWrapMode::Wrap;
                            let word = look.layout(ui, "HHHH", wrap, f32::INFINITY).size().x;
                            let text = look.layout(ui, "HHHH HHHH HHHH", wrap, word * 1.5);
                            let (rect, resp) =
                                ui.allocate_exact_size(text.size(), egui::Sense::hover());
                            text.paint(ui.painter(), rect.min, look.resolved_color(ui));
                            resp
                        }
                    })
                    .inner
                    .rect;
            }
        });
        let mut f = Frame {
            texts: Vec::new(),
            rects: Vec::new(),
            items,
        };
        for s in &out.shapes {
            walk(&s.shape, &mut f.texts, &mut f.rects);
        }
        f
    };
    let mut plain = run(Vec::new());
    for _ in 0..2 {
        plain = run(Vec::new());
    }
    let text_in = |f: &Frame, i: usize| -> TextShape {
        f.texts
            .iter()
            .find(|t| {
                f.items[i].contains(t.pos + egui::vec2(1.0, 1.0)) && !t.galley.text().is_empty()
            })
            .unwrap_or_else(|| panic!("{}: no text in item {i}", files.name))
            .clone()
    };
    let top = |f: &Frame, i: usize| (f.items[i].top() * ppp).round();
    let atlas = ctx.fonts(|f| f.image());
    let mut out = Vec::new();
    for (i, item) in ["button", "badge", "input"].into_iter().enumerate() {
        let b = row_baselines(&text_in(&plain, i), ppp, &atlas)[0] - top(&plain, i);
        let mut cols = [None; 6];
        cols[0] = Some(b);
        out.push((item.to_owned(), cols));
    }
    let rows = row_baselines(&text_in(&plain, 3), ppp, &atlas);
    assert_eq!(rows.len(), 3, "{}: block rows", files.name);
    for (k, b) in rows.into_iter().enumerate() {
        let mut cols = [None; 6];
        cols[0] = Some(b - top(&plain, 3));
        out.push((format!("block-{}", k + 1), cols));
    }

    // Focus the input with a click, select all, then move the caret to
    // the end.
    let at: Pos2 = plain.items[2].center();
    let press = |pressed| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let key = |key, modifiers| Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };
    run(vec![Event::PointerMoved(at), press(true)]);
    run(vec![press(false)]);
    let selected = run(vec![key(Key::A, Modifiers::COMMAND)]);
    let caret = run(vec![key(Key::End, Modifiers::NONE)]);

    let input_top = top(&selected, 2);
    let fill = ctx.global_style().visuals.selection.bg_fill;
    let t = text_in(&selected, 2);
    let row = &t.galley.rows[0];
    let ys: Vec<f32> = row
        .visuals
        .mesh
        .vertices
        .iter()
        // The field's opacity may round the colour by one step.
        .filter(|v| {
            let (a, b) = (v.color.to_array(), fill.to_array());
            a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 1)
        })
        .map(|v| dev(&t, row.pos.y + v.pos.y, ppp))
        .collect();
    assert!(!ys.is_empty(), "{}: no selection drawn", files.name);
    let sel = &mut out[2].1;
    sel[1] = Some(ys.iter().copied().fold(f32::MAX, f32::min) - input_top);
    sel[2] = Some(ys.iter().copied().fold(f32::MIN, f32::max) - input_top);
    let (rect, _) = caret
        .rects
        .iter()
        .find(|(r, _)| caret.items[2].contains(r.center()) && r.width() * ppp < 2.5)
        .unwrap_or_else(|| {
            panic!(
                "{}: no caret drawn {:?} in {:?}",
                files.name, caret.rects, caret.items[2]
            )
        });
    sel[3] = Some(rect.top() * ppp - input_top);
    sel[4] = Some(rect.bottom() * ppp - input_top);
    sel[5] = Some(rect.width() * ppp);
    out
}

#[test]
fn text_sits_on_chromes_pixels() {
    let mut checked = Vec::new();
    let mut wrong = Vec::new();
    let mut compared = 0;
    for name in FontName::ALL.iter().copied() {
        let Some(files) = fonts::files(name) else {
            continue;
        };
        for scale in SCALES {
            let want = chrome(name.id(), scale);
            let mut got = drawn(files, scale);
            got.sort_by(|a, b| a.0.cmp(&b.0));
            assert_eq!(want.len(), got.len(), "{} {scale}x: items", name.id());
            for ((item, want), (got_item, got)) in want.iter().zip(&got) {
                assert_eq!(item, got_item);
                for (field, (w, g)) in FIELDS.iter().zip(want.iter().zip(got)) {
                    let (Some(w), Some(g)) = (w, g) else {
                        assert_eq!(
                            w.is_some(),
                            g.is_some(),
                            "{} {scale}x {item} {field}",
                            name.id()
                        );
                        continue;
                    };
                    compared += 1;
                    if (g - g.round()).abs() > 1e-3 || g.round() as i32 != *w {
                        wrong.push(format!(
                            "{} {scale}x {item} {field}: drawn {g}, Chrome {w}",
                            name.id()
                        ));
                    }
                }
            }
        }
        checked.push(name.id());
    }
    assert!(checked.contains(&"inter"), "{checked:?}");
    if cfg!(feature = "fonts-all") {
        let fonts = CHROME.lines().skip(1).count() / (SCALES.len() * 6);
        assert_eq!(checked.len(), fonts, "{checked:?}");
    }
    assert!(compared >= checked.len() * SCALES.len() * 11, "{compared}");
    assert!(
        wrong.is_empty(),
        "text off Chrome's pixels:\n{}",
        wrong.join("\n")
    );
}

/// A scale, then Chrome's rows for the button, input, block and line, and
/// the input's selection top and bottom, caret top, bottom and width.
type NovaRows = (f32, [f32; 1], [f32; 1], [f32; 3], [f32; 1], [f32; 5]);

/// Chrome 156's rows (device px below each item's top, Inter) for three
/// cases the vega table does not have, measured with
/// `--force-device-scale-factor` at 1x, 1.5x and 2x:
/// - a nova small button (`text-sm` then `text-[0.8rem]`: 12.8 px text on
///   a 12.8 x 20/14 px line, centred in 28 px less its border);
/// - a nova input at phone width (16 px text, 24 px line, in a 22 px
///   content box): baseline, selection rows and caret rows and width;
/// - a block of three 19.5 px rows (`text-xs/relaxed`) below one such
///   row, so its origin is fractional, and a `text-sm` line below one.
const NOVA_FRAC: [NovaRows; 3] = [
    (
        1.0,
        [18.0],
        [22.0],
        [34.0, 53.0, 73.0],
        [35.0],
        [6.0, 26.0, 6.0, 26.0, 1.0],
    ),
    (
        1.5,
        [27.0],
        [33.0],
        [50.0, 80.0, 109.0],
        [51.0],
        [10.0, 39.0, 10.0, 39.0, 2.0],
    ),
    (
        2.0,
        [37.0],
        [44.0],
        [67.0, 106.0, 145.0],
        [69.0],
        [13.0, 52.0, 13.0, 52.0, 2.0],
    ),
];

#[test]
fn nova_and_fractional_origins_sit_on_chromes_pixels() {
    for (ppp, button, input, block, line, field) in NOVA_FRAC {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(ppp);
        Theme::new(Style::nova(), Palette::light(), 10.0)
            .fonts(fonts::files(FontName::Inter).unwrap(), None)
            .apply(&ctx);
        let mut b = Recipe::default();
        b.text_size.base = Some(TextSize::Px(12.0));
        b.line_height.base = Some(1.625);
        let mut s = Recipe::default();
        s.text_size.base = Some(TextSize::Sm);
        let mut value = String::from("HHHH");
        let mut run = |events: Vec<Event>| {
            let mut items = [Rect::NOTHING; 4];
            let input = RawInput {
                events,
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(390.0, 800.0))),
                ..Default::default()
            };
            let out = ctx.run_ui(input, |ui| {
                for (i, item) in items.iter_mut().enumerate() {
                    let area = egui::Area::new(egui::Id::new(i))
                        .fixed_pos(pos2(8.0, 8.0 + 80.0 * i as f32));
                    *item = area
                        .show(ui.ctx(), |ui| match i {
                            0 => ui.add(Button::new("HHHH").size(ButtonSize::Sm)),
                            1 => ui.add(Input::new(&mut value).max_width(200.0)),
                            _ => {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.vertical(|ui| {
                                    let wrap = egui::TextWrapMode::Wrap;
                                    let lb = TextLook::resolve(ui, &[&b], State::NONE);
                                    let word = lb.layout(ui, "HHHH", wrap, f32::INFINITY).size().x;
                                    let first = lb.layout(ui, "HHHH", wrap, f32::INFINITY);
                                    let (r, _) =
                                        ui.allocate_exact_size(first.size(), egui::Sense::hover());
                                    first.paint(ui.painter(), r.min, lb.resolved_color(ui));
                                    let (look, text) = if i == 2 {
                                        (lb, "HHHH HHHH HHHH")
                                    } else {
                                        (TextLook::resolve(ui, &[&s], State::NONE), "HHHH")
                                    };
                                    let t = look.layout(ui, text, wrap, word * 1.5);
                                    let (r, _) =
                                        ui.allocate_exact_size(t.size(), egui::Sense::hover());
                                    t.paint(ui.painter(), r.min, look.resolved_color(ui));
                                })
                                .response
                            }
                        })
                        .inner
                        .rect;
                }
            });
            let mut f = Frame {
                texts: Vec::new(),
                rects: Vec::new(),
                items,
            };
            for s in &out.shapes {
                walk(&s.shape, &mut f.texts, &mut f.rects);
            }
            f
        };
        let mut plain = run(Vec::new());
        for _ in 0..2 {
            plain = run(Vec::new());
        }
        let atlas = ctx.fonts(|f| f.image());
        for i in 0..4 {
            let top = (plain.items[i].top() * ppp).round();
            let rows: Vec<Vec<f32>> = plain
                .texts
                .iter()
                .filter(|t| {
                    plain.items[i].contains(t.pos + egui::vec2(1.0, 1.0))
                        && !t.galley.text().is_empty()
                })
                .map(|t| {
                    row_baselines(t, ppp, &atlas)
                        .into_iter()
                        .map(|b| b - top)
                        .collect()
                })
                .collect();
            let want: &[f32] = [&button[..], &input, &block, &line][i];
            assert_eq!(
                rows.last().map(Vec::as_slice),
                Some(want),
                "{ppp}x item {i}"
            );
        }
        let at = plain.items[1].center();
        let press = |pressed| Event::PointerButton {
            pos: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let key = |key, modifiers| Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        run(vec![Event::PointerMoved(at), press(true)]);
        run(vec![press(false)]);
        let selected = run(vec![key(Key::A, Modifiers::COMMAND)]);
        let caret = run(vec![key(Key::End, Modifiers::NONE)]);
        let itop = (selected.items[1].top() * ppp).round();
        let fill = ctx.global_style().visuals.selection.bg_fill;
        let t = selected
            .texts
            .iter()
            .find(|t| {
                selected.items[1].contains(t.pos + egui::vec2(1.0, 1.0))
                    && !t.galley.text().is_empty()
            })
            .unwrap();
        let row = &t.galley.rows[0];
        let ys: Vec<f32> = row
            .visuals
            .mesh
            .vertices
            .iter()
            .filter(|v| {
                v.color
                    .to_array()
                    .iter()
                    .zip(fill.to_array())
                    .all(|(x, y)| x.abs_diff(y) <= 1)
            })
            .map(|v| dev(t, row.pos.y + v.pos.y, ppp))
            .collect();
        let (cr, _) = caret
            .rects
            .iter()
            .find(|(r, _)| caret.items[1].contains(r.center()) && r.width() * ppp < 2.5)
            .unwrap();
        let got = [
            ys.iter().copied().fold(f32::MAX, f32::min) - itop,
            ys.iter().copied().fold(f32::MIN, f32::max) - itop,
            cr.top() * ppp - itop,
            cr.bottom() * ppp - itop,
            cr.width() * ppp,
        ];
        assert_eq!(
            got.map(|v| v.round()),
            field,
            "{ppp}x input selection and caret"
        );
        assert!(got.iter().all(|v| (v - v.round()).abs() < 1e-3), "{got:?}");
    }
}
