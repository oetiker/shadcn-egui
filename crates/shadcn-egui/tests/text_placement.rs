//! Text placement against Chrome (conformance 2.4): for every font the
//! build has, at 1x, 1.5x and 2x, the text the crate draws must sit on the
//! device pixel rows where Chrome puts it:
//! - the baseline of "HHHH" in the default vega light Button, Badge and
//!   Input;
//! - the Input's selection highlight (all text selected) and its caret
//!   (after the text): their rows and the caret's width;
//! - each row's baseline in a block wrapped into three rows on 19.5 px
//!   lines (`text-xs/relaxed`), whose line tops are fractional.
//! - the same for the nova light small Button (`text-[0.8rem]` over
//!   `text-sm`) and Input at phone width, a block of three 19.5 px rows
//!   below one such row (a fractional origin) and a `text-sm` line below
//!   one.
//! - the x of every char of a kerned text in the vega and nova Input (the
//!   caret and selection stops) and where its glyph is drawn
//!   (`text_placement/chrome-input-glyphs.tsv`, from the same script).
//! - the underline of a line of text (`underline`, with
//!   `underline-offset-4` and without), its top and thickness, at 12, 14
//!   and 16 px (`text_placement/chrome-underlines.tsv`, from
//!   `scripts/chrome-underlines.mjs`).
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
use shadcn_egui::style::{Recipe, State, Style, TextSize, Weight};
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

/// Drawn rows: item and its measured columns in device px (`None`: not
/// measured).
type Rows = Vec<(String, [Option<f32>; 6])>;

struct Frame {
    texts: Vec<TextShape>,
    rects: Vec<(Rect, Color32)>,
    items: [Rect; 4],
}

impl Frame {
    /// The last non-empty text drawn in item `i`.
    fn text_in(&self, i: usize, font: &str) -> &TextShape {
        self.texts
            .iter()
            .rfind(|t| {
                self.items[i].contains(t.pos + egui::vec2(1.0, 1.0)) && !t.galley.text().is_empty()
            })
            .unwrap_or_else(|| panic!("{font}: no text in item {i}"))
    }

    /// Device px of item `i`'s top.
    fn top(&self, i: usize, ppp: f32) -> f32 {
        (self.items[i].top() * ppp).round()
    }
}

/// Runs a frame of `ctx` with each of the four items in its own area, 80
/// points apart, on `screen` (egui's default when `None`).
fn frame(
    ctx: &egui::Context,
    screen: Option<Rect>,
    events: Vec<Event>,
    mut item: impl FnMut(&mut egui::Ui, usize) -> egui::Response,
) -> Frame {
    let mut items = [Rect::NOTHING; 4];
    let input = RawInput {
        events,
        screen_rect: screen,
        ..Default::default()
    };
    let out = ctx.run_ui(input, |ui| {
        for (i, rect) in items.iter_mut().enumerate() {
            let area =
                egui::Area::new(egui::Id::new(i)).fixed_pos(pos2(8.0, 8.0 + 80.0 * i as f32));
            *rect = area.show(ui.ctx(), |ui| item(ui, i)).inner.rect;
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
}

/// What the crate draws: per TSV item, the measured columns in device px
/// below the item's top. `item` draws the four items; `names` names the
/// TSV items of the first three (`None`: not measured; the third is the
/// input) and `block` the rows of the fourth, a text block.
fn drawn(
    ctx: &egui::Context,
    screen: Option<Rect>,
    font: &str,
    ppp: f32,
    names: [Option<&str>; 3],
    block: &[&str],
    mut item: impl FnMut(&mut egui::Ui, usize) -> egui::Response,
) -> Rows {
    let mut plain = frame(ctx, screen, Vec::new(), &mut item);
    for _ in 0..2 {
        plain = frame(ctx, screen, Vec::new(), &mut item);
    }
    let atlas = ctx.fonts(|f| f.image());
    let mut out = Vec::new();
    for (i, name) in names.into_iter().enumerate() {
        let Some(name) = name else { continue };
        let b = row_baselines(plain.text_in(i, font), ppp, &atlas)[0] - plain.top(i, ppp);
        let mut cols = [None; 6];
        cols[0] = Some(b);
        out.push((name.to_owned(), cols));
    }
    let rows = row_baselines(plain.text_in(3, font), ppp, &atlas);
    assert_eq!(rows.len(), block.len(), "{font}: block rows");
    for (name, b) in block.iter().zip(rows) {
        let mut cols = [None; 6];
        cols[0] = Some(b - plain.top(3, ppp));
        out.push(((*name).to_owned(), cols));
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
    frame(
        ctx,
        screen,
        vec![Event::PointerMoved(at), press(true)],
        &mut item,
    );
    frame(ctx, screen, vec![press(false)], &mut item);
    let selected = frame(
        ctx,
        screen,
        vec![key(Key::A, Modifiers::COMMAND)],
        &mut item,
    );
    let caret = frame(ctx, screen, vec![key(Key::End, Modifiers::NONE)], &mut item);

    let input_top = selected.top(2, ppp);
    let fill = ctx.global_style().visuals.selection.bg_fill;
    let t = selected.text_in(2, font);
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
        .map(|v| dev(t, row.pos.y + v.pos.y, ppp))
        .collect();
    assert!(!ys.is_empty(), "{font}: no selection drawn");
    let sel = &mut out
        .iter_mut()
        .find(|(n, _)| Some(n.as_str()) == names[2])
        .unwrap()
        .1;
    sel[1] = Some(ys.iter().copied().fold(f32::MAX, f32::min) - input_top);
    sel[2] = Some(ys.iter().copied().fold(f32::MIN, f32::max) - input_top);
    let (rect, _) = caret
        .rects
        .iter()
        .find(|(r, _)| caret.items[2].contains(r.center()) && r.width() * ppp < 2.5)
        .unwrap_or_else(|| {
            panic!(
                "{font}: no caret drawn {:?} in {:?}",
                caret.rects, caret.items[2]
            )
        });
    sel[3] = Some(rect.top() * ppp - input_top);
    sel[4] = Some(rect.bottom() * ppp - input_top);
    sel[5] = Some(rect.width() * ppp);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The vega items: Button, Badge and Input, and a block wrapped into three
/// rows on 19.5 px lines.
fn drawn_vega(files: &'static FontFiles, ppp: f32) -> Rows {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(ppp);
    Theme::vega_light().fonts(files, None).apply(&ctx);
    let mut block = Recipe::default();
    block.text_size.base = Some(TextSize::Px(12.0));
    block.line_height.base = Some(1.625);
    let mut value = String::from("HHHH");
    let names = [Some("button"), Some("badge"), Some("input")];
    let rows = ["block-1", "block-2", "block-3"];
    drawn(&ctx, None, files.name, ppp, names, &rows, |ui, i| match i {
        0 => ui.add(Button::new("HHHH")),
        1 => ui.add(Badge::new("HHHH")),
        2 => ui.add(Input::new(&mut value).max_width(200.0)),
        _ => {
            // One word per row, as `width: min-content`.
            let look = TextLook::resolve(ui, &[&block], State::NONE);
            let wrap = egui::TextWrapMode::Wrap;
            let word = look.layout(ui, "HHHH", wrap, f32::INFINITY).size().x;
            let text = look.layout(ui, "HHHH HHHH HHHH", wrap, word * 1.5);
            let (rect, resp) = ui.allocate_exact_size(text.size(), egui::Sense::hover());
            text.paint(ui.painter(), rect.min, look.resolved_color(ui));
            resp
        }
    })
}

/// The nova items: the small Button (`text-sm` then `text-[0.8rem]`), the
/// Input at phone width (390 points), and the frac cases: one 19.5 px row, then a
/// block wrapped into three such rows (`frac-block`), so its origin is
/// fractional. The `text-sm` line below one such row (`frac-line`) is a
/// second run of the same frame.
fn drawn_nova(files: &'static FontFiles, ppp: f32) -> Rows {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(ppp);
    Theme::new(Style::nova(), Palette::light(), 10.0)
        .fonts(files, None)
        .apply(&ctx);
    let mut b = Recipe::default();
    b.text_size.base = Some(TextSize::Px(12.0));
    b.line_height.base = Some(1.625);
    let mut s = Recipe::default();
    s.text_size.base = Some(TextSize::Sm);
    let phone = Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(390.0, 800.0)));
    let mut out = Vec::new();
    for (rows, line) in [
        (&["frac-block-1", "frac-block-2", "frac-block-3"][..], false),
        (&["frac-line"][..], true),
    ] {
        let mut value = String::from("HHHH");
        let names = [Some("nova-button-sm"), None, Some("nova-input")];
        let got = drawn(&ctx, phone, files.name, ppp, names, rows, |ui, i| match i {
            0 => ui.add(Button::new("HHHH").size(ButtonSize::Sm)),
            1 => ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover()),
            2 => ui.add(Input::new(&mut value).max_width(200.0)),
            _ => {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.vertical(|ui| {
                    let wrap = egui::TextWrapMode::Wrap;
                    let lb = TextLook::resolve(ui, &[&b], State::NONE);
                    let word = lb.layout(ui, "HHHH", wrap, f32::INFINITY).size().x;
                    let first = lb.layout(ui, "HHHH", wrap, f32::INFINITY);
                    let (r, _) = ui.allocate_exact_size(first.size(), egui::Sense::hover());
                    first.paint(ui.painter(), r.min, lb.resolved_color(ui));
                    let (look, text) = if line {
                        (TextLook::resolve(ui, &[&s], State::NONE), "HHHH")
                    } else {
                        (lb, "HHHH HHHH HHHH")
                    };
                    let t = look.layout(ui, text, wrap, word * 1.5);
                    let (r, _) = ui.allocate_exact_size(t.size(), egui::Sense::hover());
                    t.paint(ui.painter(), r.min, look.resolved_color(ui));
                })
                .response
            }
        });
        // The second run measures only its line.
        out.extend(
            got.into_iter()
                .filter(|(n, _)| !line || n.starts_with("frac")),
        );
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Compares what `draw` draws with Chrome's rows whose item `pick`
/// accepts, for every font the build has at every scale.
fn sits_on_chromes_pixels(pick: fn(&str) -> bool, draw: fn(&'static FontFiles, f32) -> Rows) {
    let mut checked = Vec::new();
    let mut wrong = Vec::new();
    let mut compared = 0;
    for name in FontName::ALL.iter().copied() {
        let Some(files) = fonts::files(name) else {
            continue;
        };
        for scale in SCALES {
            let mut want = chrome(name.id(), scale);
            want.retain(|(item, _)| pick(item));
            let got = draw(files, scale);
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
        let mut fonts: Vec<&str> = CHROME
            .lines()
            .skip(1)
            .filter_map(|l| l.split('\t').next())
            .collect();
        fonts.dedup();
        assert_eq!(checked.len(), fonts.len(), "{checked:?}");
    }
    assert!(compared >= checked.len() * SCALES.len() * 11, "{compared}");
    assert!(
        wrong.is_empty(),
        "text off Chrome's pixels:\n{}",
        wrong.join("\n")
    );
}

/// The nova and frac rows; the rest are the vega rows.
fn nova(item: &str) -> bool {
    item.starts_with("nova-") || item.starts_with("frac-")
}

#[test]
fn text_sits_on_chromes_pixels() {
    sits_on_chromes_pixels(|item| !nova(item), drawn_vega);
}

#[test]
fn nova_and_fractional_origins_sit_on_chromes_pixels() {
    sits_on_chromes_pixels(nova, drawn_nova);
}

const GLYPHS: &str = include_str!("text_placement/chrome-input-glyphs.tsv");
/// Keep in step with GLYPH_TEXT in `scripts/chrome-text-baselines.mjs`.
const GLYPH_TEXT: &str = "To AVAY, Wave 7.5x Ky";
/// How far a glyph's x may sit from Chrome's, in device px: Chrome keeps
/// positions in 1/64 css px; the kerning of a variable font at a location
/// off its default differs from HarfBuzz's by up to 2 font units (0.07 px
/// at 2x: outfit, space-grotesk), less than egui's quarter-pixel bins.
const GLYPH_SLACK: f32 = 0.1;

/// The input's text as drawn, in device px from the input's left: per
/// char its x (where the caret and the selection stop) and the left edge
/// of its quad in the mesh (`None` for a space), then the right edge of
/// the row as its last x.
struct DrawnGlyphs {
    xs: Vec<f32>,
    quads: Vec<Option<f32>>,
    /// The left edge of the quad egui draws for char `i` with its pen at
    /// `x` device px from the input's left.
    reference: Box<dyn Fn(usize, f32) -> f32>,
}

fn drawn_glyphs(ctx: &egui::Context, screen: Option<Rect>, font: &str, ppp: f32) -> DrawnGlyphs {
    let mut value = String::from(GLYPH_TEXT);
    let mut item = |ui: &mut egui::Ui, i: usize| {
        if i == 0 {
            ui.add(Input::new(&mut value).max_width(320.0))
        } else {
            ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover())
        }
    };
    let mut f = frame(ctx, screen, Vec::new(), &mut item);
    for _ in 0..2 {
        f = frame(ctx, screen, Vec::new(), &mut item);
    }
    let t = f.text_in(0, font).clone();
    assert_eq!(t.galley.text(), GLYPH_TEXT, "{font}");
    assert_eq!(t.galley.rows.len(), 1, "{font}");
    let row = &t.galley.rows[0];
    let left = (t.pos.x * ppp).round() - (f.items[0].left() * ppp).round();
    let origin = left + row.pos.x * ppp;
    let quad = |mesh: &egui::Mesh, first: usize| {
        mesh.vertices[first..first + 4]
            .iter()
            .map(|v| v.pos.x)
            .fold(f32::MAX, f32::min)
    };
    let glyphs = &row.row.glyphs;
    let mut xs: Vec<f32> = glyphs.iter().map(|g| origin + g.pos.x * ppp).collect();
    xs.push(origin + row.row.size.x * ppp);
    let quads = glyphs
        .iter()
        .map(|g| {
            (!g.chr.is_whitespace())
                .then(|| origin + quad(&row.row.visuals.mesh, g.first_vertex as usize) * ppp)
        })
        .collect();
    let format = t.galley.job.sections[0].format.clone();
    let chars: Vec<char> = GLYPH_TEXT.chars().collect();
    let ctx = ctx.clone();
    let reference = Box::new(move |i: usize, x: f32| {
        let mut job = egui::text::LayoutJob::single_section(chars[i].to_string(), format.clone());
        job.sections[0].leading_space = (x - origin) / ppp;
        let g = ctx.fonts_mut(|f| f.layout_job(job));
        origin + quad(&g.rows[0].visuals.mesh, 0) * ppp
    });
    DrawnGlyphs {
        xs,
        quads,
        reference,
    }
}

/// The input's text keeps Chrome's glyph positions (conformance 3.4): each
/// char's x (caret and selection) within `GLYPH_SLACK` of Chrome's, and
/// its glyph drawn where egui draws it with the pen at Chrome's x (egui
/// bins the pen to a quarter device pixel; near a bin's edge either
/// neighbour within the slack counts).
#[test]
fn input_glyphs_sit_on_chromes_x() {
    let mut wrong = Vec::new();
    let mut compared = 0;
    for name in FontName::ALL.iter().copied() {
        let Some(files) = fonts::files(name) else {
            continue;
        };
        for scale in SCALES {
            for item in ["input", "nova-input"] {
                let tag = format!("{} {scale}x {item}", name.id());
                let want: Vec<f32> = GLYPHS
                    .lines()
                    .skip(1)
                    .map(|l| l.split('\t').collect::<Vec<_>>())
                    .find(|c| {
                        c[0] == name.id() && c[1].parse::<f32>().ok() == Some(scale) && c[2] == item
                    })
                    .unwrap_or_else(|| panic!("{tag}: no Chrome row"))[3]
                    .split(',')
                    .map(|x| x.parse().unwrap())
                    .collect();
                let ctx = egui::Context::default();
                ctx.set_pixels_per_point(scale);
                let screen = if item == "input" {
                    Theme::vega_light().fonts(files, None).apply(&ctx);
                    None
                } else {
                    Theme::new(Style::nova(), Palette::light(), 10.0)
                        .fonts(files, None)
                        .apply(&ctx);
                    Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(390.0, 800.0)))
                };
                let got = drawn_glyphs(&ctx, screen, name.id(), scale);
                assert_eq!(want.len(), got.xs.len(), "{tag}");
                compared += 1;
                if want[0] != got.xs[0] {
                    wrong.push(format!(
                        "{tag} origin: crate {}, Chrome {}",
                        got.xs[0], want[0]
                    ));
                }
                // From here on, x from the text's origin.
                let want: Vec<f32> = want.iter().map(|w| got.xs[0] + w - want[0]).collect();
                for (i, (w, g)) in want.iter().zip(&got.xs).enumerate().skip(1) {
                    compared += 1;
                    if (w - g).abs() > GLYPH_SLACK {
                        wrong.push(format!("{tag} char {i} x: crate {g}, Chrome {w}"));
                    }
                }
                for (i, q) in got.quads.iter().enumerate() {
                    let Some(q) = q else { continue };
                    compared += 1;
                    let fits = [0.0, -GLYPH_SLACK, GLYPH_SLACK]
                        .iter()
                        .any(|d| ((got.reference)(i, want[i] + d) - q).abs() < 1e-3);
                    if !fits {
                        let at = (got.reference)(i, want[i]);
                        wrong.push(format!("{tag} char {i} drawn at {q}, at Chrome's x {at}"));
                    }
                }
            }
        }
    }
    assert!(
        compared >= 2 * SCALES.len() * (GLYPH_TEXT.len() + 1),
        "{compared}"
    );
    assert!(
        wrong.is_empty(),
        "{} of {compared} glyphs off Chrome's x:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

const UNDERLINES: &str = include_str!("text_placement/chrome-underlines.tsv");

/// The underlines drawn in `t`, as (top, thickness) in device px below the
/// galley origin: its quads whose four vertices sit on the white texel.
fn underlines(t: &TextShape, ppp: f32) -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    for r in &t.galley.rows {
        for quad in r.row.visuals.mesh.vertices.chunks(4) {
            if quad.iter().all(|v| v.uv == egui::epaint::WHITE_UV) {
                let top = quad.iter().map(|v| v.pos.y).fold(f32::INFINITY, f32::min);
                let bottom = quad
                    .iter()
                    .map(|v| v.pos.y)
                    .fold(f32::NEG_INFINITY, f32::max);
                let origin = (t.pos.y * ppp).round();
                out.push((dev(t, r.pos.y + top, ppp) - origin, (bottom - top) * ppp));
            }
        }
    }
    out
}

#[test]
fn underlines_sit_on_chromes_pixels() {
    let mut wrong = Vec::new();
    let mut compared = 0;
    for name in FontName::ALL.iter().copied() {
        let Some(files) = fonts::files(name) else {
            continue;
        };
        for scale in SCALES {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            Theme::vega_light().fonts(files, None).apply(&ctx);
            for l in UNDERLINES.lines().skip(1) {
                let c: Vec<&str> = l.split('\t').collect();
                if c[0] != name.id() || c[1].parse::<f32>().ok() != Some(scale) {
                    continue;
                }
                let tag = format!("{} {scale}x {}px/{} offset {}", c[0], c[2], c[3], c[4]);
                let (size, line): (f32, f32) = (c[2].parse().unwrap(), c[3].parse().unwrap());
                let mut recipe = Recipe::default();
                recipe.text_size.base = Some(TextSize::Px(size));
                recipe.line_height.base = Some(line / size);
                recipe.weight.base = Some(Weight::Medium);
                recipe.underline.base = Some(true);
                recipe.underline_offset.base = c[4].strip_suffix("px").map(|v| v.parse().unwrap());
                let want = (c[6].parse::<f32>().unwrap(), c[7].split(',').count() as f32);
                // A new area shows from its second frame.
                let run = |ui: &mut egui::Ui| {
                    egui::Area::new(egui::Id::new("u"))
                        .fixed_pos(pos2(8.0, 8.0))
                        .show(ui.ctx(), |ui| {
                            let look = TextLook::resolve(ui, &[&recipe], State::NONE);
                            let text = look.layout(
                                ui,
                                "HHHHHHHH",
                                egui::TextWrapMode::Extend,
                                f32::INFINITY,
                            );
                            let (rect, _) =
                                ui.allocate_exact_size(text.size(), egui::Sense::hover());
                            text.paint(ui.painter(), rect.min, look.resolved_color(ui));
                        });
                };
                let _ = ctx.run_ui(RawInput::default(), run);
                let out = ctx.run_ui(RawInput::default(), run);
                let (mut texts, mut rects) = (Vec::new(), Vec::new());
                for s in &out.shapes {
                    walk(&s.shape, &mut texts, &mut rects);
                }
                let [t] = &texts[..] else {
                    panic!("{tag}: {} texts", texts.len());
                };
                let got = underlines(t, scale);
                compared += 1;
                let on = |g: f32, w: f32| (g - w).abs() < 1e-3;
                if !matches!(got[..], [(top, h)] if on(top, want.0) && on(h, want.1)) {
                    wrong.push(format!("{tag}: drawn {got:?}, Chrome {want:?}"));
                }
            }
        }
    }
    assert!(compared >= SCALES.len() * 6, "{compared}");
    assert!(
        wrong.is_empty(),
        "underlines off Chrome's pixels:\n{}",
        wrong.join("\n")
    );
}
