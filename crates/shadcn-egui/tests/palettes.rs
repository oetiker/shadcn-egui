use shadcn_egui::color::oklch_to_srgb;
use shadcn_egui::Palette;

#[test]
fn light_and_dark_are_the_generated_neutral_palette() {
    assert_eq!(
        Palette::from_upstream("neutral", "neutral", "neutral", false),
        Some(Palette::light())
    );
    assert_eq!(
        Palette::from_upstream("neutral", "neutral", "neutral", true),
        Some(Palette::dark())
    );
}

#[test]
fn neutral_charts_are_the_grey_ramp_in_both_modes() {
    let ramp = [0.87, 0.556, 0.439, 0.371, 0.269].map(|l| oklch_to_srgb(l, 0.0, 0.0));
    assert_eq!(Palette::light().chart, ramp);
    assert_eq!(Palette::dark().chart, ramp);
}

#[test]
fn destructive_foreground_is_the_near_white_upstream_dropped() {
    let p = Palette::from_upstream("stone", "blue", "rose", true).expect("known ids");
    assert_eq!(p.destructive_foreground, oklch_to_srgb(0.985, 0.0, 0.0));
}
