//! Design corrections (spec, "Design corrections"): `CORRECTIONS` edits a
//! part's class string before translation, every entry matches its target,
//! and the corrected values win where the spec says they do.

use stylegen::css::parse_style_css;
use stylegen::extract::part_strings;
use stylegen::markers::parse_markers;
use stylegen::model::{flag, Field, Value};
use stylegen::resolve::{correct, expand, parts, Correction, Decl, Edit, Part, CORRECTIONS};
use stylegen::sources::STYLES;
use stylegen::variant::R_IMPORTANT;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn css(style: &str) -> Vec<(String, Vec<String>)> {
    let p = format!("{ROOT}/tests/fixtures/style-{style}.css");
    parse_style_css(&std::fs::read_to_string(p).expect("fixture"))
}

fn components() -> Vec<(String, Vec<String>)> {
    let lines = parse_markers(&std::fs::read_to_string(format!("{ROOT}/markers.txt")).unwrap());
    part_strings(
        &std::path::Path::new(ROOT).join("tests/fixtures/ui"),
        &lines,
    )
    .unwrap()
}

fn trigger(style: &str) -> Part {
    parts(style, &css(style), &components())
        .unwrap()
        .into_iter()
        .find(|p| p.name == "cn-tabs-trigger")
        .expect("cn-tabs-trigger")
}

/// The value the crate's `pick` resolves for `field` under `state`: the
/// matching decl with the highest rank, the later one on a tie.
fn resolve(decls: &[Decl], field: Field, state: u32) -> Option<Value> {
    decls
        .iter()
        .filter(|d| d.field == field && d.all & !state == 0 && d.none & state == 0)
        .max_by_key(|d| (d.rank, d.idx))
        .map(|d| d.value)
}

fn paint(role: &'static str) -> Option<Value> {
    Some(Value::Paint { role, alpha: 1.0 })
}

#[test]
fn every_correction_matches_its_target() {
    let comps = components();
    for c in CORRECTIONS {
        if let Some(styles) = c.styles {
            for s in styles {
                assert!(STYLES.contains(s), "{}: unknown style {s}", c.id);
            }
        }
        let mut hits = 0;
        for style in STYLES {
            let css = css(style);
            for (name, classes) in &comps {
                let (_, applied) = correct(
                    std::slice::from_ref(c),
                    style,
                    name,
                    expand(classes, name, &css),
                )
                .unwrap_or_else(|e| panic!("{e}"));
                hits += applied.len();
            }
        }
        assert!(hits > 0, "{} ({}) applies nowhere", c.id, c.part);
    }
}

#[test]
fn an_edit_that_does_not_match_fails() {
    let css = css("vega");
    let comps = components();
    let classes = &comps
        .iter()
        .find(|(n, _)| n == "cn-tabs-trigger")
        .unwrap()
        .1;
    let full = expand(classes, "cn-tabs-trigger", &css);
    let missing = Correction {
        id: "T1",
        styles: Some(&["vega"]),
        part: "cn-tabs-trigger",
        edit: Edit::Replace("border-transparent!", "border-transparent"),
        reason: "test",
    };
    assert!(correct(&[missing], "vega", "cn-tabs-trigger", full.clone()).is_err());
    let present = Correction {
        edit: Edit::Append(&["focus-visible:border-ring"]),
        ..missing
    };
    assert!(correct(&[present], "vega", "cn-tabs-trigger", full.clone()).is_err());
    // Another style or part is left alone.
    let (same, applied) = correct(&[missing], "luma", "cn-tabs-trigger", full.clone()).unwrap();
    assert_eq!((same, applied.len()), (full, 0));
}

/// C1: luma and rhea lose `border-transparent!`, so the base classes'
/// focus and dark active borders win again.
#[test]
fn c1_luma_and_rhea_trigger_border_is_not_important() {
    let (h, def, sel, fv, dark) = (
        flag("HORIZONTAL"),
        flag("VARIANT_DEFAULT"),
        flag("SELECTED"),
        flag("FOCUS_VISIBLE"),
        flag("DARK"),
    );
    for style in ["luma", "rhea"] {
        let p = trigger(style);
        assert!(
            !p.marker
                .decls
                .iter()
                .any(|d| d.field == Field::BorderColor && d.rank >> R_IMPORTANT != 0),
            "{style}: important border rule left"
        );
        let d = &p.marker.decls;
        assert_eq!(
            resolve(d, Field::BorderColor, h | def | fv),
            paint("Ring"),
            "{style}"
        );
        assert_eq!(
            resolve(d, Field::BorderColor, h | def | sel | dark),
            paint("Input"),
            "{style}"
        );
        assert_eq!(
            resolve(d, Field::BorderColor, h | def),
            paint("Transparent"),
            "{style}"
        );
        assert!(
            p.corrections.iter().any(|c| c.contains("(C1)")),
            "{style}: {:?}",
            p.corrections
        );
    }
}

/// C2: in a line list a focused trigger draws no ring, outline or focus
/// border. Like hover, focus only shows on an inactive trigger: its label
/// turns `Foreground` and its underline shows in `Ring`, 2 px. C2 adds no
/// focus marker to the active trigger: it keeps upstream's 2 px
/// `Foreground` underline, light or dark. The default variant keeps
/// upstream's focus ring.
#[test]
fn c2_adds_focus_to_inactive_line_triggers_only() {
    let (h, v, line, def, sel, fv, dark) = (
        flag("HORIZONTAL"),
        flag("VERTICAL"),
        flag("VARIANT_LINE"),
        flag("VARIANT_DEFAULT"),
        flag("SELECTED"),
        flag("FOCUS_VISIBLE"),
        flag("DARK"),
    );
    for style in STYLES {
        let p = trigger(style);
        let (d, a) = (&p.marker.decls, &p.marker.after);
        for extra in [0, sel, dark, sel | dark] {
            let st = h | line | fv | extra;
            let px0 = Some(Value::Px(0.0));
            assert_eq!(resolve(d, Field::RingWidth, st), px0, "{style} {st:#x}");
            assert_eq!(resolve(d, Field::OutlineWidth, st), px0, "{style} {st:#x}");
            assert_eq!(
                resolve(d, Field::BorderColor, st),
                paint("Transparent"),
                "{style} {st:#x}"
            );
            assert_eq!(
                resolve(a, Field::Opacity, st),
                Some(Value::Factor(1.0)),
                "{style} {st:#x}"
            );
            let active = extra & sel != 0;
            let colour = if active { "Foreground" } else { "Ring" };
            assert_eq!(resolve(a, Field::Bg, st), paint(colour), "{style} {st:#x}");
            assert_eq!(
                resolve(a, Field::Height, st),
                Some(Value::Px(2.0)),
                "{style} {st:#x}"
            );
            let vst = st & !h | v;
            assert_eq!(
                resolve(a, Field::Width, vst),
                Some(Value::Px(2.0)),
                "{style} {vst:#x}"
            );
            assert_eq!(
                resolve(d, Field::Text, st),
                paint("Foreground"),
                "{style} {st:#x}: label"
            );
        }
        // An unfocused inactive label stays dimmed, in dark mode too.
        assert_eq!(
            resolve(d, Field::Text, h | line),
            Some(Value::Paint {
                role: "Foreground",
                alpha: 0.6
            }),
            "{style}"
        );
        assert_eq!(
            resolve(d, Field::Text, h | line | dark),
            paint("MutedForeground"),
            "{style}"
        );
        // An unfocused active line trigger keeps the 2 px underline.
        assert_eq!(
            resolve(a, Field::Height, h | line | sel),
            Some(Value::Px(2.0)),
            "{style}"
        );
        // Unfocused line triggers and focused default triggers as upstream.
        assert_eq!(
            resolve(a, Field::Opacity, h | line),
            Some(Value::Factor(0.0)),
            "{style}"
        );
        assert_eq!(
            resolve(a, Field::Bg, h | line | sel),
            paint("Foreground"),
            "{style}"
        );
        assert_eq!(
            resolve(a, Field::Opacity, h | def | fv),
            Some(Value::Factor(0.0)),
            "{style}"
        );
        assert_eq!(
            resolve(d, Field::RingWidth, h | def | fv),
            Some(Value::Px(3.0)),
            "{style}"
        );
        assert_eq!(
            resolve(d, Field::OutlineWidth, h | def | fv),
            Some(Value::Px(1.0)),
            "{style}"
        );
        assert!(
            p.corrections.iter().any(|c| c.contains("(C2)")),
            "{style}: {:?}",
            p.corrections
        );
    }
}

#[test]
fn the_report_lists_each_correction_under_its_part() {
    let style = "luma";
    let ps = parts(style, &css(style), &components()).unwrap();
    let r = stylegen::report::report(style, "pin", &ps);
    let section = r
        .split("## ")
        .find(|s| s.starts_with("`cn-tabs-trigger`"))
        .expect("cn-tabs-trigger section");
    assert!(
        section.contains("corrected (C1) `border-transparent!` to `border-transparent`"),
        "{section}"
    );
    assert!(section.contains("corrected (C2) added"), "{section}");
}
