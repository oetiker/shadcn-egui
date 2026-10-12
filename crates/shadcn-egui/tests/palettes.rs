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

use egui::Color32;
use shadcn_egui::color::oklch_to_srgb_a;

/// `oklch(L C H)` / `oklch(L C H / A%)`, as upstream writes them.
fn oklch(s: &str) -> Color32 {
    let inner = s
        .strip_prefix("oklch(")
        .and_then(|r| r.strip_suffix(')'))
        .unwrap_or_else(|| panic!("not oklch: {s}"));
    let (lch, alpha) = match inner.split_once('/') {
        Some((lch, a)) => {
            let a = a.trim();
            (
                lch,
                a.strip_suffix('%').map_or_else(
                    || a.parse::<f32>().unwrap(),
                    |p| p.parse::<f32>().unwrap() / 100.0,
                ),
            )
        }
        None => (inner, 1.0),
    };
    let v: Vec<f32> = lch.split_whitespace().map(|n| n.parse().unwrap()).collect();
    oklch_to_srgb_a(v[0], v[1], v[2], alpha)
}

/// Our field for an upstream var; `None` for vars the palette does not hold.
fn field(p: &Palette, var: &str) -> Option<Color32> {
    Some(match var {
        "background" => p.background,
        "foreground" => p.foreground,
        "card" => p.card,
        "card-foreground" => p.card_foreground,
        "popover" => p.popover,
        "popover-foreground" => p.popover_foreground,
        "primary" => p.primary,
        "primary-foreground" => p.primary_foreground,
        "secondary" => p.secondary,
        "secondary-foreground" => p.secondary_foreground,
        "muted" => p.muted,
        "muted-foreground" => p.muted_foreground,
        "accent" => p.accent,
        "accent-foreground" => p.accent_foreground,
        "destructive" => p.destructive,
        "border" => p.border,
        "input" => p.input,
        "ring" => p.ring,
        "chart-1" => p.chart[0],
        "chart-2" => p.chart[1],
        "chart-3" => p.chart[2],
        "chart-4" => p.chart[3],
        "chart-5" => p.chart[4],
        v if v == "radius" || v.starts_with("sidebar") => return None,
        other => panic!("upstream var `{other}` has no Palette field"),
    })
}

#[test]
fn merged_palettes_equal_upstream_build_registry_theme() {
    let text = include_str!("fixtures/registry_themes.txt");
    let mut checked = 0;
    for line in text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let w: Vec<&str> = line.splitn(7, ' ').collect();
        let [base, theme, chart, accent, mode, var, value] = w[..] else {
            panic!("bad line: {line}")
        };
        let mut p = Palette::from_upstream(base, theme, chart, mode == "dark")
            .unwrap_or_else(|| panic!("{line}: unknown id"));
        if accent == "bold" {
            p = p.with_bold_menu_accent();
        }
        if let Some(ours) = field(&p, var) {
            assert_eq!(ours, oklch(value), "{line}");
            checked += 1;
        }
    }
    assert_eq!(checked, 41 * 2 * 23, "41 cases x 2 modes x 23 colour vars");
}

#[test]
fn unknown_ids_give_none() {
    assert_eq!(
        Palette::from_upstream("blue", "neutral", "neutral", false),
        None,
        "blue is not a base colour"
    );
    assert_eq!(
        Palette::from_upstream("neutral", "nope", "neutral", false),
        None
    );
    assert_eq!(
        Palette::from_upstream("neutral", "neutral", "nope", true),
        None
    );
    assert_eq!(Palette::from_upstream("", "", "", true), None);
    // `gray` is a preset code value on ui.shadcn.com without a palette (index §J).
    assert_eq!(
        Palette::from_upstream("gray", "neutral", "neutral", false),
        None
    );
    assert_eq!(
        Palette::from_upstream("neutral", "gray", "neutral", false),
        None
    );
}
