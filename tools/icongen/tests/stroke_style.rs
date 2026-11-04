//! Per-path `stroke-linecap` / `stroke-linejoin`: what reaches the
//! segments and the blob.

use icongen::blob::{decode, encode};
use icongen::{convert, ensure_stroke_only, Cap, Join, Seg, StrokeStyle};

fn svg(body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="none" stroke="black">{body}</svg>"#
    )
}

fn style(cap: Cap, join: Join) -> Seg {
    Seg::Style(StrokeStyle { cap, join })
}

fn styles(segs: &[Seg]) -> Vec<Seg> {
    segs.iter()
        .filter(|s| matches!(s, Seg::Style(_)))
        .copied()
        .collect()
}

#[test]
fn svg_default_is_butt_and_miter() {
    let icon = convert("d", &svg(r#"<path d="M1 1L5 1"/>"#)).expect("converts");
    assert_eq!(
        icon.segs,
        vec![
            style(Cap::Butt, Join::Miter),
            Seg::M(1.0, 1.0),
            Seg::L(5.0, 1.0)
        ]
    );
}

#[test]
fn round_strokes_hold_no_style() {
    let body = r#"<g stroke-linecap="round" stroke-linejoin="round"><path d="M1 1L5 1"/><path d="M1 3L5 3"/></g>"#;
    let icon = convert("r", &svg(body)).expect("converts");
    assert_eq!(styles(&icon.segs), vec![], "{:?}", icon.segs);
}

#[test]
fn style_is_inherited_from_a_group() {
    let body = r#"<g stroke-linecap="square" stroke-linejoin="bevel"><path d="M1 1L5 1"/></g>"#;
    let icon = convert("g", &svg(body)).expect("converts");
    assert_eq!(icon.segs[0], style(Cap::Square, Join::Bevel));
}

#[test]
fn a_style_is_written_only_where_it_changes() {
    // butt/round twice, then round/round: two styles, the second one
    // switching back to round.
    let body = r#"<path d="M1 1L5 1" stroke-linejoin="round"/>
        <path d="M1 3L5 3" stroke-linejoin="round"/>
        <path d="M1 5L5 5" stroke-linecap="round" stroke-linejoin="round"/>"#;
    let icon = convert("s", &svg(body)).expect("converts");
    assert_eq!(
        icon.segs,
        vec![
            style(Cap::Butt, Join::Round),
            Seg::M(1.0, 1.0),
            Seg::L(5.0, 1.0),
            Seg::M(1.0, 3.0),
            Seg::L(5.0, 3.0),
            Seg::Style(StrokeStyle::ROUND),
            Seg::M(1.0, 5.0),
            Seg::L(5.0, 5.0),
        ]
    );
}

#[test]
fn a_layer_starts_round() {
    // The translucent path is its own layer: its style is written there
    // although the opaque path before it set the same one.
    let body = r#"<path d="M1 1L5 1"/><path d="M1 3L5 3" opacity="0.4"/>"#;
    let icon = convert("l", &svg(body)).expect("converts");
    assert_eq!(styles(&icon.segs), vec![style(Cap::Butt, Join::Miter)]);
    assert_eq!(icon.layers.len(), 1);
    assert_eq!(icon.layers[0].segs[0], style(Cap::Butt, Join::Miter));
}

#[test]
fn styles_survive_the_blob() {
    let body = r#"<path d="M1 1L5 1" stroke-linecap="square"/>
        <path d="M1 3L5 3" stroke-linejoin="bevel" stroke-linecap="round"/>
        <path d="M1 7L5 7" stroke-linejoin="round" opacity="0.5"/>"#;
    let icon = convert("b", &svg(body)).expect("converts");
    let (segs, _, layers) = decode(&encode(&icon).expect("encodes"));
    assert_eq!(segs, icon.segs);
    assert_eq!(
        styles(&segs),
        vec![
            style(Cap::Square, Join::Miter),
            style(Cap::Round, Join::Bevel)
        ]
    );
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].segs, icon.layers[0].segs);
    assert_eq!(styles(&layers[0].segs), vec![style(Cap::Butt, Join::Round)]);
}

#[test]
fn every_style_word_reads_back() {
    for cap in [Cap::Round, Cap::Butt, Cap::Square] {
        for join in [Join::Round, Join::Miter, Join::Bevel] {
            let s = StrokeStyle { cap, join };
            assert_eq!(StrokeStyle::from_word(s.word()), Some(s));
        }
    }
    assert_eq!(StrokeStyle::ROUND.word(), 0);
    assert_eq!(StrokeStyle::from_word(3), None);
    assert_eq!(StrokeStyle::from_word(3 << 2), None);
    assert_eq!(StrokeStyle::from_word(1 << 4), None);
}

#[test]
fn a_miter_limit_other_than_4_is_an_error() {
    let err = convert(
        "m",
        &svg(r#"<path d="M1 1L5 1L1 5" stroke-miterlimit="10"/>"#),
    )
    .expect_err("the blob holds limit 4 only");
    assert!(err.contains("m: stroke-miterlimit 10"), "{err}");
    // With a join that is not a miter the limit does not matter.
    convert(
        "b",
        &svg(r#"<path d="M1 1L5 1L1 5" stroke-miterlimit="10" stroke-linejoin="bevel"/>"#),
    )
    .expect("converts");
}

#[test]
fn a_miter_clip_join_is_an_error() {
    let err = convert(
        "c",
        &svg(r#"<path d="M1 1L5 1L1 5" stroke-linejoin="miter-clip"/>"#),
    )
    .expect_err("the blob holds no miter-clip");
    assert!(err.contains("c: stroke-linejoin MiterClip"), "{err}");
}

#[test]
fn out_rejects_styles() {
    let icon = convert("o", &svg(r#"<path d="M1 1L5 1"/>"#)).expect("converts");
    let err = ensure_stroke_only(&icon).expect_err("--out writes round strokes only");
    assert!(err.contains("o: stroke caps or joins"), "{err}");
}

#[test]
fn hugeicons_fixture_keeps_its_styles() {
    // clipboard-check: a round/round tick, a butt/round clip and a filled
    // board that the root's stroke outlines with SVG's butt/miter default.
    let svg = std::fs::read_to_string(format!(
        "{}/tests/fixtures/hugeicons-clipboard-check.svg",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture");
    let icon = convert("clipboard-check", &svg).expect("converts");
    assert_eq!(
        styles(&icon.segs),
        vec![style(Cap::Butt, Join::Round), style(Cap::Butt, Join::Miter)]
    );
    assert!(!matches!(icon.segs[0], Seg::Style(_)), "the tick is round");
}
