use shadcn_egui::color::{mix_oklch, oklch_to_srgb, oklch_to_srgb_a, srgb_to_oklch};

#[test]
fn white_is_white() {
    let c = oklch_to_srgb(1.0, 0.0, 0.0);
    assert_eq!((c.r(), c.g(), c.b()), (255, 255, 255));
}

#[test]
fn achromatic_grays_match_zinc() {
    // (lightness, expected gray) verified against shadcn v4 zinc ramp.
    let cases = [
        (0.985, 250u8),
        (0.922, 229),
        (0.708, 161),
        (0.556, 115),
        (0.269, 38),
        (0.205, 23),
        (0.145, 10),
    ];
    for (l, expected) in cases {
        let c = oklch_to_srgb(l, 0.0, 0.0);
        let got = c.r();
        assert!(
            (got as i16 - expected as i16).abs() <= 1,
            "L={l}: expected ~{expected}, got {got}"
        );
        assert_eq!(c.r(), c.g(), "L={l} not gray");
        assert_eq!(c.g(), c.b(), "L={l} not gray");
    }
}

#[test]
fn destructive_red_is_reddish() {
    let c = oklch_to_srgb(0.577, 0.245, 27.325);
    assert!(c.r() > 200, "expected strong red, got r={}", c.r());
    assert!(
        c.g() < 90 && c.b() < 90,
        "expected low g/b, got {:?}",
        (c.g(), c.b())
    );
}

#[test]
fn alpha_is_passed_through() {
    let c = oklch_to_srgb_a(1.0, 0.0, 0.0, 0.10);
    assert_eq!(c.a(), 26); // round(0.10 * 255)
}

#[test]
fn oklch_round_trip() {
    for (l, c, h) in [
        (0.205, 0.0, 0.0),
        (0.577, 0.245, 27.325),
        (0.985, 0.0, 0.0),
        (0.704, 0.191, 22.216),
    ] {
        let rgb = oklch_to_srgb(l, c, h);
        let (l2, c2, h2) = srgb_to_oklch(rgb);
        assert!((l - l2).abs() < 0.01, "L {l} -> {l2}");
        assert!((c - c2).abs() < 0.01, "C {c} -> {c2}");
        if c > 0.02 {
            assert!((h - h2).abs() < 1.5, "H {h} -> {h2}");
        }
    }
}

#[test]
fn mix_endpoints_and_middle() {
    let a = oklch_to_srgb(0.97, 0.0, 0.0);
    let b = oklch_to_srgb(0.145, 0.0, 0.0);
    assert_eq!(mix_oklch(a, b, 0.0), a);
    assert_eq!(mix_oklch(a, b, 1.0), b);
    let (l, _, _) = srgb_to_oklch(mix_oklch(a, b, 0.05));
    // 95% of 0.97 + 5% of 0.145
    assert!((l - (0.97 * 0.95 + 0.145 * 0.05)).abs() < 0.01, "got {l}");
}

#[test]
fn mix_keeps_alpha_linear() {
    let a = egui::Color32::from_rgba_unmultiplied(255, 0, 0, 255);
    let b = egui::Color32::from_rgba_unmultiplied(0, 0, 255, 0);
    assert_eq!(mix_oklch(a, b, 0.5).a(), 128);
}
