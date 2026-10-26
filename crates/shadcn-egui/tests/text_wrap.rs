//! Line breaks against Chrome (conformance 2.5, 3.4): for every font the
//! build has, at every scale the reference has, `text-sm` text breaks its
//! lines where Chrome breaks them under `text-wrap: wrap`, `balance` and
//! `pretty`, for description texts at several widths, and the galley's
//! text stays the caller's.
//!
//! The reference rows are in `text_placement/chrome-wraps.tsv`: per font,
//! scale, text and width, the char indices where each line after the first
//! starts. Regenerate them after a font change with
//! `node scripts/chrome-text-wrap.mjs <chrome-binary>`. A `|` in a text
//! starts or ends a coloured span there and is no char of the text.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, RawInput, TextWrapMode};
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
    (
        "hyphen",
        "A well-known step-by-step guide for a long-term follow-up review",
    ),
    (
        "dash",
        "Wait\u{2014}what? Pages ten\u{2013}twenty are missing\u{2014}again\u{2014}sadly, said the clerk.",
    ),
    (
        "zwsp",
        "averyveryverylong\u{200B}wordwith\u{200B}zerowidth\u{200B}spaces in it",
    ),
    (
        "shy",
        "Rhein\u{AD}wasser\u{AD}boots\u{AD}kapit\u{E4}ns\u{AD}m\u{FC}tzen\u{AD}halter und Crew",
    ),
    (
        "digits",
        "A well-known state-of-the-art e-mail re-sign 2-3 x-1 pre-1990 test",
    ),
    (
        "slash",
        "Read and/or write to path/to/some/doc or example.com/a/b now.",
    ),
    (
        "punct",
        "Hello (world), [this] is {a} test; it\u{2019}s okay! Really? Yes: 1,000.50 & 50% less.",
    ),
    (
        "spans",
        "T|oday AV|A W|aves Y|ou T|ry P|ay V|AT Y|ear AW|AY T|yped W|A V|ow L|T",
    ),
    (
        "cjk",
        "\u{4E2D}\u{6587}\u{FF0C}\u{65E5}\u{672C}\u{8A9E}\u{300C}\u{30C6}\u{30B9}\u{30C8}\u{300D}\u{3067}\u{3059}\u{3002}\u{30AD}\u{30E3}\u{30E9}\u{30FC}\u{30E1} ab\u{4E2D}cd",
    ),
];
/// Rows where the crate breaks other than Chrome by an accepted difference
/// (conformance 2.4): Chrome draws contextual alternates (`calt`), the
/// crate draws one glyph per character. Manrope's `calt` turns the hyphen
/// in "2-3" into `hyphen.case`, which has no kerning against "3"; the
/// crate kerns the plain hyphen 0.56 px closer and balance then prefers
/// "2-|3". The test asserts these rows still differ, so a build that
/// shapes `calt` shows up here. The other texts hit no `liga`, `calt` or
/// `ccmp` substitution in any shipped font; Inter's `calt` swaps the same
/// hyphen without moving a break.
const ACCEPTED: &[&str] = &[
    "manrope 1x digits 127.4 Some(Balance)",
    "manrope 1.25x digits 127.4 Some(Balance)",
    "manrope 1.5x digits 127.4 Some(Balance)",
    "manrope 1.75x digits 127.4 Some(Balance)",
    "manrope 2x digits 127.4 Some(Balance)",
];
const MODES: [Option<TextWrap>; 3] = [None, Some(TextWrap::Balance), Some(TextWrap::Pretty)];

/// `text` in the look's font, a section per `|`-separated part, every
/// second one coloured, as the script's spans.
fn spans(ui: &egui::Ui, look: &TextLook, text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (i, part) in text.split('|').enumerate() {
        let color = if i % 2 == 1 {
            Color32::RED
        } else {
            Color32::PLACEHOLDER
        };
        job.append(part, 0.0, TextFormat::simple(look.font_id(ui.ctx()), color));
    }
    job
}

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
    // Per font the build has: its TSV rows, and the results compared.
    let mut fonts_compared: Vec<(&str, usize, usize)> = Vec::new();
    let rows: Vec<Vec<&str>> = CHROME
        .lines()
        .skip(1)
        .map(|l| l.split('\t').collect())
        .collect();
    for name in FontName::ALL.iter().copied() {
        let Some(files) = fonts::files(name) else {
            continue;
        };
        let font_rows = rows.iter().filter(|f| f[0] == name.id()).count();
        let before = compared;
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
            Theme::vega().fonts(files, None).apply(&ctx);
            ctx.set_theme(egui::Theme::Light);
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
                        let block = if text.contains('|') {
                            let job = spans(ui, &look, text);
                            look.layout(ui, job, TextWrapMode::Wrap, width)
                        } else {
                            look.layout(ui, text, TextWrapMode::Wrap, width)
                        };
                        let got = starts(block.galley());
                        compared += 1;
                        // A break adds no char: the galley's text, its
                        // indices and a copy are the caller's.
                        if block.galley().text() != text.replace('|', "") {
                            wrong.push(format!(
                                "{} {scale}x {} {width} {mode:?}: galley text {:?}",
                                f[0],
                                f[2],
                                block.galley().text()
                            ));
                        }
                        let key = format!("{} {scale}x {} {width} {mode:?}", f[0], f[2]);
                        let accepted = ACCEPTED.contains(&key.as_str());
                        if got != *want && !accepted {
                            wrong.push(format!("{key}: Chrome {want}, crate {got}"));
                        } else if got == *want && accepted {
                            wrong.push(format!("{key}: matches Chrome, leave ACCEPTED"));
                        }
                    }
                }
            });
        }
        fonts_compared.push((name.id(), font_rows, compared - before));
    }
    // Every row of every font the build has is compared in all modes.
    for (id, font_rows, n) in &fonts_compared {
        assert!(*font_rows > 0, "{id}: no Chrome rows");
        assert_eq!(*n, font_rows * MODES.len(), "{id}: results compared");
    }
    let mut tsv_fonts: Vec<&str> = rows.iter().map(|f| f[0]).collect();
    tsv_fonts.sort_unstable();
    tsv_fonts.dedup();
    if cfg!(feature = "fonts-all") {
        assert_eq!(fonts_compared.len(), tsv_fonts.len(), "fonts compared");
    } else {
        assert!(!fonts_compared.is_empty());
    }
    assert!(
        wrong.is_empty(),
        "{} of {compared} differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
