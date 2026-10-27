//! OKLCH <-> sRGB conversion and OKLCH color mix (shadcn v4 stores tokens as OKLCH).
//! Converts oklch(L C H) to an egui Color32 and back. Out-of-gamut channels are clamped.

use egui::Color32;

/// `l` in 0..=1, `c` chroma (~0..0.4), `h` hue in degrees.
pub fn oklch_to_srgb(l: f32, c: f32, h: f32) -> Color32 {
    oklch_to_srgb_a(l, c, h, 1.0)
}

/// Same as [`oklch_to_srgb`] with an explicit alpha in 0..=1.
pub fn oklch_to_srgb_a(l: f32, c: f32, h: f32, alpha: f32) -> Color32 {
    let h_rad = h.to_radians();
    oklab_to_srgb_a(l, c * h_rad.cos(), c * h_rad.sin(), alpha)
}

/// oklab(L a b / alpha) to sRGB, out-of-gamut channels clamped.
fn oklab_to_srgb_a(l: f32, oklab_a: f32, oklab_b: f32, alpha: f32) -> Color32 {
    // OKLab -> LMS' -> LMS
    let l_ = l + 0.396_337_78 * oklab_a + 0.215_803_76 * oklab_b;
    let m_ = l - 0.105_561_35 * oklab_a - 0.063_854_17 * oklab_b;
    let s_ = l - 0.089_484_18 * oklab_a - 1.291_485_5 * oklab_b;
    let (lc, mc, sc) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);

    // LMS -> linear sRGB
    let r = 4.076_741_7 * lc - 3.307_711_6 * mc + 0.230_969_93 * sc;
    let g = -1.268_438 * lc + 2.609_757_4 * mc - 0.341_319_4 * sc;
    let b = -0.004_196_09 * lc - 0.703_418_6 * mc + 1.707_614_7 * sc;

    let to_u8 = |lin: f32| -> u8 {
        let lin = lin.clamp(0.0, 1.0);
        let srgb = if lin <= 0.003_130_8 {
            12.92 * lin
        } else {
            1.055 * lin.powf(1.0 / 2.4) - 0.055
        };
        (srgb * 255.0).round().clamp(0.0, 255.0) as u8
    };

    Color32::from_rgba_unmultiplied(
        to_u8(r),
        to_u8(g),
        to_u8(b),
        (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

/// sRGB (unmultiplied, alpha ignored) to OKLCH: L in 0..1, C, H in degrees.
pub fn srgb_to_oklch(c: Color32) -> (f32, f32, f32) {
    let (ok_l, ok_a, ok_b) = srgb_to_oklab(c);
    let chroma = (ok_a * ok_a + ok_b * ok_b).sqrt();
    let hue = ok_b.atan2(ok_a).to_degrees().rem_euclid(360.0);
    (ok_l, chroma, hue)
}

/// The OKLCH a mix starts from: a theme's own token unclipped, as the
/// browser mixes it ([`crate::palette::unclipped`]), with `true`; else
/// `c` itself, with `false`.
fn mix_input(c: Color32) -> ((f32, f32, f32), bool) {
    match crate::palette::unclipped(c) {
        Some([l, c, h]) => ((l, c, h), true),
        None => (srgb_to_oklch(c), false),
    }
}

/// sRGB (unmultiplied, alpha ignored) to OKLab: L, a, b.
fn srgb_to_oklab(c: Color32) -> (f32, f32, f32) {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    let lin = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(r), lin(g), lin(b));
    let l = 0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_99 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;
    let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
    let ok_l = 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s;
    let ok_a = 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s;
    let ok_b = 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s;
    (ok_l, ok_a, ok_b)
}

/// The colour a CSS transition or animation shows between `a` and `b` at
/// eased progress `t` (the share of `b`). CSS Color 4 §12 interpolates two
/// colours in OKLab with premultiplied alpha unless both are legacy sRGB
/// colours, and shadcn's tokens are `oklch()`, so Chrome mixes them in
/// OKLab; a fully transparent side lends only its alpha. A colour that is
/// exactly a theme token's mixes as the token, before its clip to sRGB.
pub fn mix_oklab(a: Color32, b: Color32, t: f32) -> Color32 {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let premultiplied = |c: Color32| {
        let alpha = f32::from(c.a()) / 255.0;
        let ((l, chroma, h), _) = mix_input(c);
        let (a, b) = (chroma * h.to_radians().cos(), chroma * h.to_radians().sin());
        [l * alpha, a * alpha, b * alpha, alpha]
    };
    let (p, q) = (premultiplied(a), premultiplied(b));
    let [l, ok_a, ok_b, alpha] = std::array::from_fn(|i| p[i] + (q[i] - p[i]) * t);
    if alpha <= 0.0 {
        return Color32::TRANSPARENT;
    }
    oklab_to_srgb_a(l / alpha, ok_a / alpha, ok_b / alpha, alpha)
}

/// CSS `color-mix(in oklch, a, b t)`: `t` is the share of `b`. Hue takes the
/// shorter arc; an achromatic side adopts the other side's hue. A colour
/// that is exactly a theme token's mixes as the token's `oklch(l c h)`,
/// before its clip to sRGB and with its own hue even at chroma 0.
pub fn mix_oklch(a: Color32, b: Color32, t: f32) -> Color32 {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let ((l1, c1, mut h1), token1) = mix_input(a);
    let ((l2, c2, mut h2), token2) = mix_input(b);
    // A colour converted from sRGB has no hue at chroma 0 (CSS: powerless,
    // so missing) and takes the other side's; a token's `oklch(l 0 h)`
    // keeps the hue it names.
    if c1 < 1e-4 && !token1 {
        h1 = h2;
    }
    if c2 < 1e-4 && !token2 {
        h2 = h1;
    }
    let mut dh = h2 - h1;
    if dh > 180.0 {
        dh -= 360.0;
    } else if dh < -180.0 {
        dh += 360.0;
    }
    let alpha = a.a() as f32 / 255.0 * (1.0 - t) + b.a() as f32 / 255.0 * t;
    oklch_to_srgb_a(l1 + (l2 - l1) * t, c1 + (c2 - c1) * t, h1 + dh * t, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_oklab_is_css_color_mix_in_oklab() {
        // `color-mix(in oklab, white, black)` is oklab(0.5 0 0), rgb(99 99 99).
        let mid = mix_oklab(Color32::WHITE, Color32::BLACK, 0.5);
        assert_eq!(mid, Color32::from_gray(99));
        // A transparent side lends only its alpha (premultiplied).
        let red = Color32::from_rgb(255, 0, 0);
        let half = mix_oklab(red, Color32::TRANSPARENT, 0.5);
        assert_eq!(half.to_srgba_unmultiplied(), [255, 0, 0, 128]);
    }

    /// Within 1/255 per channel of what Chrome shows.
    fn near(got: Color32, chrome: [u8; 4]) -> bool {
        let got = got.to_srgba_unmultiplied();
        (0..4).all(|i| got[i].abs_diff(chrome[i]) <= 1)
    }

    /// A fade that involves a theme colour outside sRGB mixes the colour
    /// the theme defines, not its clipped sRGB value, as Chrome does.
    /// Chrome 156 at 75 of 150 ms along `ease-in-out`.
    #[test]
    fn a_fade_mixes_an_out_of_gamut_theme_colour_unclipped() {
        let (light, dark) = (crate::Palette::light(), crate::Palette::dark());
        let t = crate::style::Easing::InOut.at(0.5);
        let mid = mix_oklab(light.primary, light.destructive, t);
        assert!(near(mid, [181, 22, 25, 255]), "{mid:?}");
        let mid = mix_oklab(dark.destructive, dark.primary, t);
        assert!(near(mid, [240, 203, 200, 255]), "{mid:?}");
    }

    /// `Paint::Mix` at rest with an out-of-gamut role matches Chrome's
    /// `color-mix(in oklch, ..)`; a role colour the app sets itself mixes
    /// as that exact colour.
    #[test]
    fn a_rest_mix_takes_an_out_of_gamut_theme_colour_unclipped() {
        use crate::style::{Paint, Role};
        let mix = Paint::Mix(Role::Destructive, Role::Background, 0.3);
        let mut theme = crate::Theme::vega().in_mode(egui::Theme::Light);
        let got = theme.paint(mix);
        assert!(near(got, [248, 108, 116, 255]), "{got:?}");
        let rest = Paint::Mix(Role::Secondary, Role::Foreground, 0.05);
        assert!(near(theme.paint(rest), [231, 231, 231, 255]));
        let own = Color32::from_rgb(232, 0, 11);
        theme.light.destructive = own;
        let expect = {
            let (l1, c1, h1) = srgb_to_oklch(own);
            // Background is the token oklch(1 0 0): its hue 0 counts.
            oklch_to_srgb(l1 + (1.0 - l1) * 0.3, c1 * 0.7, h1 * 0.7)
        };
        let got = theme.paint(mix);
        assert!(near(got, expect.to_array()), "{got:?} {expect:?}");
        assert!(!near(got, [248, 108, 116, 255]), "{got:?}");
    }
}
