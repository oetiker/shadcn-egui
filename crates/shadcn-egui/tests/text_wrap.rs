//! Line breaks against Chrome (conformance 2.5, 3.4): for every font the
//! build has, at every scale the reference has, `text-sm` text breaks its
//! lines where Chrome breaks them under `text-wrap: wrap`, `balance` and
//! `pretty`, for description texts at several widths.
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
const TEXTS: &[(&str, &str)] = &[
    (
        "alert-dialog",
        "This action cannot be undone. This will permanently delete your account and remove your data from our servers.",
    ),
    ("alert", "Please verify your billing information and try again."),
    (
        "form",
        "Use the form below to tell us what went wrong with your order so we can sort it out",
    ),
    (
        "code",
        "We have sent you a code by email. Please enter it below to continue with your sign up.",
    ),
    (
        "save",
        "Make changes to your account here. Click save when you are done.",
    ),
    ("avatar", "Select View avatar"),
    ("email", "Open messages email"),
    ("reset", "Reset billing email"),
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
fn lines_break_where_chrome_breaks_them() {
    let mut wrong = Vec::new();
    let mut compared = 0;
    let rows: Vec<Vec<&str>> = CHROME
        .lines()
        .skip(1)
        .map(|l| l.split('\t').collect())
        .collect();
    for name in FontName::ALL.iter().copied() {
        let Some(files) = fonts::files(name) else {
            continue;
        };
        let mut scales: Vec<&str> = rows
            .iter()
            .filter(|f| f[0] == name.id())
            .map(|f| f[1])
            .collect();
        scales.dedup();
        for scale_id in scales {
            let scale: f32 = scale_id.parse().unwrap();
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            Theme::vega_light().fonts(files, None).apply(&ctx);
            let _ = ctx.run_ui(RawInput::default(), |_| {});
            let _ = ctx.run_ui(RawInput::default(), |ui| {
                for f in rows
                    .iter()
                    .filter(|f| f[0] == name.id() && f[1] == scale_id)
                {
                    let text = TEXTS.iter().find(|t| t.0 == f[2]).unwrap().1;
                    let width: f32 = f[3].parse().unwrap();
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
