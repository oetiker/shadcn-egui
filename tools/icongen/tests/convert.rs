use icongen::{const_name, convert, render_module, Seg};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}.svg",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture")
}

fn subpaths(segs: &[Seg]) -> usize {
    segs.iter().filter(|s| matches!(s, Seg::M(..))).count()
}

#[test]
fn lucide_mic_converts() {
    let icon = convert("mic", &fixture("mic")).expect("converts");
    assert_eq!(icon.size, 24.0);
    // <path M12 19v3>, <path …arc…>, <rect rx=3>: three subpaths.
    assert_eq!(subpaths(&icon.segs), 3, "{:?}", icon.segs);
    assert_eq!(icon.segs[0], Seg::M(12.0, 19.0));
    assert_eq!(icon.segs[1], Seg::L(12.0, 22.0));
    // The rounded rect and the arc come out as curves.
    assert!(icon.segs.iter().any(|s| matches!(s, Seg::C(..))));
}

#[test]
fn near_zero_length_dot_survives() {
    // circle-alert draws its dot as <line x1=12 x2=12.01>; it must not be
    // dropped, or the icon loses its dot.
    let icon = convert("circle-alert", &fixture("circle-alert")).expect("converts");
    assert_eq!(subpaths(&icon.segs), 3, "{:?}", icon.segs);
    assert!(
        icon.segs
            .windows(2)
            .any(|w| w == [Seg::M(12.0, 16.0), Seg::L(12.01, 16.0)]),
        "{:?}",
        icon.segs
    );
}

#[test]
fn all_fixtures_convert() {
    for n in ["mic", "circle-alert", "settings", "x", "power", "monitor"] {
        let icon = convert(n, &fixture(n)).unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(icon.segs[0], Seg::M(..)), "{n} must start with M");
        for s in &icon.segs {
            let coords: Vec<f32> = match *s {
                Seg::M(x, y) | Seg::L(x, y) => vec![x, y],
                Seg::Q(a, b, x, y) => vec![a, b, x, y],
                Seg::C(a, b, c, d, x, y) => vec![a, b, c, d, x, y],
                Seg::Z => vec![],
            };
            assert!(
                coords.iter().all(|v| (-1.0..=25.0).contains(v)),
                "{n}: {s:?} outside the view box"
            );
        }
    }
}

#[test]
fn transforms_and_viewbox_are_applied() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 12 12"
        fill="none" stroke="black"><g transform="translate(1 2)"><path d="M0 0L2 0"/></g></svg>"#;
    let icon = convert("t", svg).expect("converts");
    assert_eq!(icon.size, 24.0);
    assert_eq!(icon.segs, vec![Seg::M(2.0, 4.0), Seg::L(6.0, 4.0)]);
}

#[test]
fn moveto_is_restored_after_close() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" fill="none" stroke="black">
        <path d="M1 1L5 1L5 5Z L9 9"/></svg>"#;
    let icon = convert("z", svg).expect("converts");
    let z = icon.segs.iter().position(|s| *s == Seg::Z).expect("has Z");
    assert_eq!(icon.segs[z + 1], Seg::M(1.0, 1.0), "{:?}", icon.segs);
}

#[test]
fn stroke_only_check_rejects_fills() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect width="4" height="4" fill="black"/></svg>"#;
    let icon = convert("filled", svg).expect("fills convert now");
    assert!(icon.fill.is_some());
    let err = icongen::ensure_stroke_only(&icon).expect_err("--out writes strokes only");
    assert!(err.contains("filled"), "{err}");
}

#[test]
fn non_square_is_rejected() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="12" fill="none" stroke="black"><path d="M0 0L1 1"/></svg>"#;
    assert!(convert("wide", svg).is_err());
}

#[test]
fn names_and_module_render() {
    assert_eq!(const_name("circle-alert"), "CIRCLE_ALERT");
    assert_eq!(const_name("3d-box"), "I_3D_BOX");
    let icon = convert("x", &fixture("x")).expect("converts");
    let src = render_module(
        &[icon],
        "crate::shadcn::components::icon",
        "Lucide v1.54.0 (ISC)",
    );
    assert!(
        src.contains("use crate::shadcn::components::icon::{Icon, Seg::*};"),
        "{src}"
    );
    assert!(src.contains("pub const X: Icon = Icon {"), "{src}");
    assert!(src.contains("        M(18.0, 6.0),"), "{src}");
    assert!(src.contains("// Lucide v1.54.0 (ISC)"));
    assert!(src.contains("pub const ALL: &[&Icon] = &[&X];"));
}
