//! shadcn base colours and colour themes, generated from shadcn/ui
//! `themes.ts` (`generated.rs`, by `tools/stylegen`), and the merge upstream
//! `buildRegistryTheme` applies to them.

#[rustfmt::skip]
#[allow(clippy::type_complexity)]
mod generated;

use crate::color::oklch_to_srgb_a;
use crate::theme::Palette;
use egui::Color32;

/// An upstream `oklch(l c h / alpha)`; alpha 1.0 is opaque.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Oklch(pub f32, pub f32, pub f32, pub f32);

impl Oklch {
    fn color(self) -> Color32 {
        oklch_to_srgb_a(self.0, self.1, self.2, self.3)
    }
}

/// The upstream CSS vars a [`Palette`] holds. `sidebar-*` and `radius` are
/// not palette colours and are not generated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Var {
    Background,
    Foreground,
    Card,
    CardForeground,
    Popover,
    PopoverForeground,
    Primary,
    PrimaryForeground,
    Secondary,
    SecondaryForeground,
    Muted,
    MutedForeground,
    Accent,
    AccentForeground,
    Destructive,
    Border,
    Input,
    Ring,
    Chart1,
    Chart2,
    Chart3,
    Chart4,
    Chart5,
}

type Vars = &'static [(Var, Oklch)];

/// A base colour: (radius in rem, light vars, dark vars).
fn base(id: &str) -> Option<(f32, Vars, Vars)> {
    generated::BASES
        .iter()
        .find(|b| b.0 == id)
        .map(|b| (b.1, b.2, b.3))
}

/// A theme's vars: a base name overlays that whole base (upstream `THEMES`
/// holds the bases too), else a colour overlay.
fn theme_vars(id: &str, dark: bool) -> Option<Vars> {
    let pick = |light: Vars, d: Vars| if dark { d } else { light };
    base(id).map(|(_, l, d)| pick(l, d)).or_else(|| {
        generated::OVERLAYS
            .iter()
            .find(|o| o.0 == id)
            .map(|o| pick(o.1, o.2))
    })
}

/// Every field transparent except `destructive_foreground`, which upstream
/// dropped and the components still read.
fn blank() -> Palette {
    let t = Color32::TRANSPARENT;
    Palette {
        background: t,
        foreground: t,
        card: t,
        card_foreground: t,
        popover: t,
        popover_foreground: t,
        primary: t,
        primary_foreground: t,
        secondary: t,
        secondary_foreground: t,
        muted: t,
        muted_foreground: t,
        accent: t,
        accent_foreground: t,
        destructive: t,
        destructive_foreground: Oklch(0.985, 0.0, 0.0, 1.0).color(),
        border: t,
        input: t,
        ring: t,
        chart: [t; 5],
    }
}

fn set(p: &mut Palette, v: Var, c: Color32) {
    match v {
        Var::Background => p.background = c,
        Var::Foreground => p.foreground = c,
        Var::Card => p.card = c,
        Var::CardForeground => p.card_foreground = c,
        Var::Popover => p.popover = c,
        Var::PopoverForeground => p.popover_foreground = c,
        Var::Primary => p.primary = c,
        Var::PrimaryForeground => p.primary_foreground = c,
        Var::Secondary => p.secondary = c,
        Var::SecondaryForeground => p.secondary_foreground = c,
        Var::Muted => p.muted = c,
        Var::MutedForeground => p.muted_foreground = c,
        Var::Accent => p.accent = c,
        Var::AccentForeground => p.accent_foreground = c,
        Var::Destructive => p.destructive = c,
        Var::Border => p.border = c,
        Var::Input => p.input = c,
        Var::Ring => p.ring = c,
        Var::Chart1 => p.chart[0] = c,
        Var::Chart2 => p.chart[1] = c,
        Var::Chart3 => p.chart[2] = c,
        Var::Chart4 => p.chart[3] = c,
        Var::Chart5 => p.chart[4] = c,
    }
}

/// Upstream `buildRegistryTheme` (config.ts): base vars, then the theme on
/// top, then `chart-1..5` from the chart theme. `None` for an unknown id.
pub(crate) fn merge(base_id: &str, theme: &str, chart: &str, dark: bool) -> Option<Palette> {
    let (_, light_vars, dark_vars) = base(base_id)?;
    let theme = theme_vars(theme, dark)?;
    let chart = theme_vars(chart, dark)?;
    let mut p = blank();
    for &(v, c) in if dark { dark_vars } else { light_vars } {
        set(&mut p, v, c.color());
    }
    for &(v, c) in theme {
        set(&mut p, v, c.color());
    }
    for &(v, c) in chart {
        if matches!(
            v,
            Var::Chart1 | Var::Chart2 | Var::Chart3 | Var::Chart4 | Var::Chart5
        ) {
            set(&mut p, v, c.color());
        }
    }
    Some(p)
}
