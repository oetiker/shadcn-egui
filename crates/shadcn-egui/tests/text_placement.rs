//! Text placement against Chrome (conformance 2.4): for every font the
//! build has, at 1x and 2x, the baseline the crate draws in the default
//! vega light Button, Badge and Input with "HHHH" must sit on the device
//! pixel row where Chrome puts it.
//!
//! The reference rows are in `text_placement/chrome-baselines.tsv`
//! (device px from the element's border-box top to its baseline), measured
//! from headless Chrome renders. Regenerate them after a font or style
//! change with `node scripts/chrome-text-baselines.mjs <chrome-binary>`.
//!
//! The drawn baseline comes from the frame's text shapes, as the
//! tessellator draws them: the galley position rounded to a device pixel,
//! plus the row's offset and the glyph's baseline, plus the font's
//! `y_offset_factor`, which egui adds to every glyph quad. No GPU needed.

use egui::{pos2, Rect, Shape};
use shadcn_egui::components::badge::Badge;
use shadcn_egui::components::button::Button;
use shadcn_egui::components::input::Input;
use shadcn_egui::{fonts, FontFiles, FontName, Theme};

const CHROME: &str = include_str!("text_placement/chrome-baselines.tsv");
const ITEMS: [&str; 3] = ["button", "badge", "input"];

/// Chrome's baseline for `font`, `scale`, `item`.
fn chrome(font: &str, scale: u32, item: &str) -> Option<i32> {
    CHROME.lines().skip(1).find_map(|l| {
        let c: Vec<&str> = l.split('\t').collect();
        (c[0] == font && c[1] == scale.to_string() && c[2] == item).then(|| c[4].parse().ok())?
    })
}

fn texts(shape: &Shape, out: &mut Vec<egui::epaint::TextShape>) {
    match shape {
        Shape::Text(t) => out.push(t.clone()),
        Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

/// The drawn baseline of the first row in each item, in device px below
/// the item's top.
fn drawn(files: &'static FontFiles, ppp: f32) -> [i32; 3] {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(ppp);
    Theme::vega_light().fonts(files, None).apply(&ctx);
    let mut value = String::from("HHHH");
    let mut rects = [Rect::NOTHING; 3];
    let mut shapes = Vec::new();
    for _ in 0..3 {
        let out = ctx.run_ui(Default::default(), |ui| {
            for (i, item) in ITEMS.iter().enumerate() {
                let area = egui::Area::new(egui::Id::new(item))
                    .fixed_pos(pos2(8.0, 8.0 + 60.0 * i as f32));
                rects[i] = area
                    .show(ui.ctx(), |ui| match *item {
                        "button" => ui.add(Button::new("HHHH")),
                        "badge" => ui.add(Badge::new("HHHH")),
                        _ => ui.add(Input::new(&mut value).max_width(200.0)),
                    })
                    .inner
                    .rect;
            }
        });
        shapes = out.shapes;
    }
    let mut all = Vec::new();
    for s in &shapes {
        texts(&s.shape, &mut all);
    }
    rects.map(|rect| {
        let t = all
            .iter()
            .find(|t| rect.contains(t.pos) && !t.galley.rows.is_empty())
            .unwrap_or_else(|| panic!("{}: no text in {rect:?}", files.name));
        let row = &t.galley.rows[0];
        let size = t.galley.job.sections[0].format.font_id.size;
        let baseline = (t.pos.y * ppp).round()
            + (row.pos.y + row.row.glyphs[0].pos.y + files.y_offset_factor * size) * ppp
            - rect.top() * ppp;
        assert_eq!(
            baseline,
            baseline.round(),
            "{}: off the pixel grid",
            files.name
        );
        baseline as i32
    })
}

#[test]
fn baselines_sit_on_chromes_pixel() {
    let mut checked = Vec::new();
    let mut wrong = Vec::new();
    for name in FontName::ALL.iter().copied() {
        let Some(files) = fonts::files(name) else {
            continue;
        };
        for scale in [1u32, 2] {
            let got = drawn(files, scale as f32);
            for (item, got) in ITEMS.iter().zip(got) {
                let want = chrome(name.id(), scale, item)
                    .unwrap_or_else(|| panic!("no Chrome row for {} {scale}x {item}", name.id()));
                if got != want {
                    wrong.push(format!(
                        "{} {scale}x {item}: drawn {got}, Chrome {want}",
                        name.id()
                    ));
                }
            }
        }
        checked.push(name.id());
    }
    assert!(checked.contains(&"inter"), "{checked:?}");
    if cfg!(feature = "fonts-all") {
        assert_eq!(
            checked.len(),
            CHROME.lines().skip(1).count() / 6,
            "{checked:?}"
        );
    }
    assert!(
        wrong.is_empty(),
        "baselines off Chrome's pixel:\n{}",
        wrong.join("\n")
    );
}
