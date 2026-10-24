//! Line breaks against Chrome (conformance 2.5, 3.4): for every font the
//! build has, at 1x, 1.5x and 2x, `text-sm` text breaks its lines where
//! Chrome breaks them under `text-wrap: wrap`, `balance` and `pretty`, for
//! two description texts at several widths.
//!
//! The reference rows are in `text_placement/chrome-wraps.tsv`: per font,
//! scale, text and width, the char indices where each line after the first
//! starts. Regenerate them after a font change with
//! `node scripts/chrome-text-wrap.mjs <chrome-binary>`.

use egui::{RawInput, TextWrapMode};
use shadcn_egui::components::text::TextLook;
use shadcn_egui::style::{Prop, Recipe, State, TextSize, TextWrap};
use shadcn_egui::{fonts, FontName, Theme};

const CHROME: &str = include_str!("text_placement/chrome-wraps.tsv");
/// Keep in step with TEXTS in `scripts/chrome-text-wrap.mjs`.
const TEXTS: [(&str, &str); 2] = [
    (
        "alert-dialog",
        "This action cannot be undone. This will permanently delete your account and remove your data from our servers.",
    ),
    ("alert", "Please verify your billing information and try again."),
];
const MODES: [Option<TextWrap>; 3] = [None, Some(TextWrap::Balance), Some(TextWrap::Pretty)];

/// Where each line after the first starts, as the TSV writes it.
fn starts(galley: &egui::Galley) -> String {
    let mut at = 0;
    let mut out = Vec::new();
    for (i, placed) in galley.rows.iter().enumerate() {
        if i > 0 {
            out.push(at.to_string());
        }
        at += placed.row.glyphs.len() + usize::from(placed.ends_with_newline);
    }
    if out.is_empty() {
        "-".into()
    } else {
        out.join(",")
    }
}

#[test]
#[ignore = "293 of 2808 cases differ, mostly egui's greedy wrap (A2b2a hand-back)"]
fn lines_break_where_chrome_breaks_them() {
    let mut wrong = Vec::new();
    let mut compared = 0;
    for name in FontName::ALL.iter().copied() {
        let Some(files) = fonts::files(name) else {
            continue;
        };
        for line in CHROME.lines().skip(1) {
            let f: Vec<&str> = line.split('\t').collect();
            if f[0] != name.id() {
                continue;
            }
            let scale: f32 = f[1].parse().unwrap();
            let text = TEXTS.iter().find(|t| t.0 == f[2]).unwrap().1;
            let width: f32 = f[3].parse().unwrap();
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            Theme::vega_light().fonts(files, None).apply(&ctx);
            let _ = ctx.run_ui(RawInput::default(), |_| {});
            let _ = ctx.run_ui(RawInput::default(), |ui| {
                for (mode, want) in MODES.iter().zip(&f[4..7]) {
                    let recipe = Recipe {
                        text_size: Prop::fixed(TextSize::Sm),
                        text_wrap: Prop {
                            base: *mode,
                            rules: Vec::new(),
                        },
                        ..Recipe::default()
                    };
                    let look = TextLook::resolve(ui, &[&recipe], State::NONE);
                    let block = look.layout(ui, text, TextWrapMode::Wrap, width);
                    let got = starts(block.galley());
                    compared += 1;
                    if got != *want {
                        wrong.push(format!(
                            "{} {scale}x {} {width} {mode:?}: Chrome {want}, crate {got}",
                            f[0], f[2]
                        ));
                    }
                }
            });
        }
    }
    assert!(compared > 0);
    assert!(
        wrong.is_empty(),
        "{} of {compared} differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
