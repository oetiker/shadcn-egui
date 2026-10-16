use icongen::blob::{decode, encode, ALPHA_SCALE};
use icongen::{convert, ensure_stroke_only, Seg};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}.svg",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture")
}

#[test]
fn a_translucent_path_becomes_its_own_layer() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="none" stroke="black">
        <path d="M1 1L5 1"/><path opacity="0.4" d="M1 3L5 3"/></svg>"#;
    let icon = convert("t", svg).expect("converts");
    assert_eq!(icon.segs, vec![Seg::M(1.0, 1.0), Seg::L(5.0, 1.0)]);
    assert_eq!(icon.layers.len(), 1, "{:?}", icon.layers);
    let layer = &icon.layers[0];
    assert!((layer.alpha - 0.4).abs() < 1e-6, "alpha {}", layer.alpha);
    assert_eq!(layer.segs, vec![Seg::M(1.0, 3.0), Seg::L(5.0, 3.0)]);
    assert_eq!(layer.fill, None);
}

#[test]
fn opacity_multiplies_down_the_tree_and_into_paint_opacity() {
    let icon = convert("faded", &fixture("faded")).expect("converts");
    assert_eq!(icon.segs, vec![Seg::M(4.0, 6.0), Seg::L(20.0, 6.0)]);
    assert_eq!(icon.fill, None, "the only fill is translucent");
    let alphas: Vec<f32> = icon.layers.iter().map(|l| l.alpha).collect();
    assert_eq!(alphas.len(), 4, "{alphas:?}");
    // <path opacity=0.4>, a second one crossing it, <g opacity=0.5><path
    // stroke-opacity=0.5>, <rect fill-opacity=0.25>, in document order.
    for (got, want) in alphas.iter().zip([0.4, 0.4, 0.25, 0.25]) {
        assert!((got - want).abs() < 1e-6, "{alphas:?}");
    }
    assert_eq!(
        icon.layers[0].segs,
        vec![Seg::M(4.0, 12.0), Seg::L(20.0, 12.0)]
    );
    assert_eq!(
        icon.layers[1].segs,
        vec![Seg::M(8.0, 4.0), Seg::L(8.0, 14.0)]
    );
    assert_eq!(
        icon.layers[2].segs,
        vec![Seg::M(4.0, 18.0), Seg::L(20.0, 18.0)]
    );
    assert!(icon.layers[3].segs.is_empty());
    assert!(icon.layers[3].fill.is_some(), "the rect's fill");
}

#[test]
fn an_alpha_that_rounds_to_full_is_opaque_and_one_that_rounds_to_zero_is_dropped() {
    // 0.999999 * 65535 rounds to 65535; 0.001 * 0.004 * 65535 rounds to 0.
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="none" stroke="black">
        <path opacity="0.999999" d="M1 1L5 1"/>
        <g opacity="0.001"><path opacity="0.004" d="M1 3L5 3"/></g>
        <path stroke-opacity="0.000001" d="M1 5L5 5"/>
        <path opacity="0.5" d="M1 7L5 7"/></svg>"#;
    let icon = convert("t", svg).expect("converts");
    assert_eq!(icon.segs, vec![Seg::M(1.0, 1.0), Seg::L(5.0, 1.0)]);
    assert_eq!(icon.layers.len(), 1, "{:?}", icon.layers);
    assert_eq!(
        icon.layers[0].segs,
        vec![Seg::M(1.0, 7.0), Seg::L(5.0, 7.0)]
    );
    encode(&icon).expect("encodes");
}

#[test]
fn an_alpha_outside_the_open_range_is_an_error() {
    let mut icon = convert("t", &fixture("faded")).expect("converts");
    for a in [1.5, 1.0, 0.0, -0.2] {
        icon.layers[0].alpha = a;
        let err = encode(&icon).expect_err("not a translucent alpha");
        assert!(err.contains("strictly between 0 and 1"), "{a}: {err}");
    }
}

#[test]
fn opaque_icons_have_no_layers_and_invisible_paths_are_dropped() {
    let icon = convert("mixed", &fixture("mixed")).expect("converts");
    assert!(icon.layers.is_empty());
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="none" stroke="black">
        <path d="M1 1L5 1"/><path stroke-opacity="0" d="M1 3L5 3"/></svg>"#;
    let icon = convert("t", svg).expect("converts");
    assert!(icon.layers.is_empty(), "{:?}", icon.layers);
}

#[test]
fn the_record_stores_each_layer_with_its_alpha() {
    let icon = convert("faded", &fixture("faded")).expect("converts");
    let rec = encode(&icon).expect("encodes");
    let words: Vec<u16> = rec
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| u16::from_le_bytes(c))
        .collect();
    // Base layer: M L (6 words) and an empty fill section; then the first
    // translucent layer starts with its alpha word.
    assert_eq!(&words[..1], &[6]);
    assert_eq!(&words[7..10], &[0, 0, 0]);
    assert_eq!(words[10], (0.4 * ALPHA_SCALE).round() as u16);
    assert_eq!(words[10], 26214);
    let (segs, fill, layers) = decode(&rec);
    assert_eq!((segs, fill), (icon.segs.clone(), None));
    assert_eq!(layers.len(), 4);
    for (got, want) in layers.iter().zip(&icon.layers) {
        assert!((got.alpha - want.alpha).abs() <= 0.5 / ALPHA_SCALE);
        assert_eq!(got.segs, want.segs);
        // Miters are quantised in the record; fill.rs checks their round trip.
        let shape =
            |f: &Option<icongen::Fill>| f.as_ref().map(|f| (f.vertices.clone(), f.indices.clone()));
        assert_eq!(shape(&got.fill), shape(&want.fill));
    }
}

#[test]
fn opaque_records_are_unchanged_by_layers() {
    // An opaque icon's record ends after its fill section, as before layers.
    let icon = convert("x", &fixture("x")).expect("converts");
    let rec = encode(&icon).expect("encodes");
    let stroke = 2 + 2 * usize::from(u16::from_le_bytes([rec[0], rec[1]]));
    assert_eq!(rec.len(), stroke + 6);
}

#[test]
fn slices_cannot_hold_translucent_layers() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="none" stroke="black">
        <path opacity="0.4" d="M1 3L5 3"/></svg>"#;
    let icon = convert("faint", svg).expect("converts");
    let err = ensure_stroke_only(&icon).expect_err("--out has no alpha");
    assert!(
        err.contains("faint") && err.contains("translucent"),
        "{err}"
    );
}
