//! shadcn/ui `apps/v4/registry/themes.ts` (+ `base-colors.ts`, which names
//! the bases) → `crates/shadcn-egui/src/palette/generated.rs`.
//!
//! A line-based reader: upstream writes one `key: "value",` per line inside
//! each `light: {` / `dark: {` map. Anything else inside such a map is an
//! error, so a format change upstream fails loudly instead of dropping vars.

/// An `oklch(l c h / alpha)` value; `alpha` is 1.0 when absent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    pub l: f32,
    pub c: f32,
    pub h: f32,
    pub alpha: f32,
}

/// One entry of `THEMES`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeEntry {
    pub name: String,
    /// `radius: "0.625rem"` (bases only).
    pub radius_rem: Option<f32>,
    /// Colour vars in source order, sidebar vars included.
    pub light: Vec<(String, Oklch)>,
    pub dark: Vec<(String, Oklch)>,
}

/// The base colours (`BASE_COLORS` of `base-colors.ts`) and the other
/// `THEMES` entries, the colour overlays, each in `THEMES` order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Themes {
    pub bases: Vec<ThemeEntry>,
    pub overlays: Vec<ThemeEntry>,
}

/// Upstream var → crate `palette::Var` variant, in `Var` order. A base
/// palette must define all of them; `sidebar-*` and `radius` are skipped.
pub const VARS: [(&str, &str); 23] = [
    ("background", "Background"),
    ("foreground", "Foreground"),
    ("card", "Card"),
    ("card-foreground", "CardForeground"),
    ("popover", "Popover"),
    ("popover-foreground", "PopoverForeground"),
    ("primary", "Primary"),
    ("primary-foreground", "PrimaryForeground"),
    ("secondary", "Secondary"),
    ("secondary-foreground", "SecondaryForeground"),
    ("muted", "Muted"),
    ("muted-foreground", "MutedForeground"),
    ("accent", "Accent"),
    ("accent-foreground", "AccentForeground"),
    ("destructive", "Destructive"),
    ("border", "Border"),
    ("input", "Input"),
    ("ring", "Ring"),
    ("chart-1", "Chart1"),
    ("chart-2", "Chart2"),
    ("chart-3", "Chart3"),
    ("chart-4", "Chart4"),
    ("chart-5", "Chart5"),
];

fn num(s: &str) -> Result<f32, String> {
    s.trim()
        .parse::<f32>()
        .map_err(|e| format!("bad number `{s}`: {e}"))
}

/// `oklch(L C H)` or `oklch(L C H / A)` with `A` a number or a percentage.
pub fn parse_oklch(s: &str) -> Result<Oklch, String> {
    let inner = s
        .strip_prefix("oklch(")
        .and_then(|r| r.strip_suffix(')'))
        .ok_or_else(|| format!("not oklch(…): {s}"))?;
    let (lch, alpha) = match inner.split_once('/') {
        Some((lch, a)) => {
            let a = a.trim();
            let alpha = match a.strip_suffix('%') {
                Some(p) => num(p)? / 100.0,
                None => num(a)?,
            };
            (lch, alpha)
        }
        None => (inner, 1.0),
    };
    let v: Vec<f32> = lch.split_whitespace().map(num).collect::<Result<_, _>>()?;
    let &[l, c, h] = v.as_slice() else {
        return Err(format!("oklch needs three numbers: {s}"));
    };
    Ok(Oklch { l, c, h, alpha })
}

/// The names in the `[...].includes(` filter of `base-colors.ts`.
fn base_names(base_colors_ts: &str) -> Result<Vec<String>, String> {
    let line = base_colors_ts
        .lines()
        .find(|l| l.contains("].includes("))
        .ok_or("base-colors.ts: no `[...].includes(` line")?;
    let list = line
        .split_once('[')
        .and_then(|(_, r)| r.split_once(']'))
        .map(|(l, _)| l)
        .unwrap_or_default();
    let names: Vec<String> = list
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect();
    if names.is_empty() {
        return Err("base-colors.ts: empty base list".into());
    }
    Ok(names)
}

/// Read the `THEMES` array; `base_colors_ts` says which entries are bases.
pub fn parse_themes(src: &str, base_colors_ts: &str) -> Result<Themes, String> {
    let base_ids = base_names(base_colors_ts)?;
    let mut inside = false;
    let mut all: Vec<ThemeEntry> = Vec::new();
    // Some(true) inside a `dark: {` map, Some(false) inside `light: {`.
    let mut mode: Option<bool> = None;
    for (i, line) in src.lines().enumerate() {
        let n = i + 1;
        let t = line.trim();
        if t.starts_with("export const THEMES") && t.ends_with('[') {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if mode.is_none() && (t == "]" || t.starts_with("] as const")) {
            inside = false;
            continue;
        }
        let list = &mut all;
        if mode.is_none() {
            if let Some(name) = t
                .strip_prefix("name: \"")
                .and_then(|r| r.strip_suffix("\","))
            {
                list.push(ThemeEntry {
                    name: name.to_owned(),
                    ..ThemeEntry::default()
                });
                continue;
            }
        }
        match t {
            "light: {" => {
                mode = Some(false);
                continue;
            }
            "dark: {" => {
                mode = Some(true);
                continue;
            }
            "}," | "}" if mode.is_some() => {
                mode = None;
                continue;
            }
            _ => {}
        }
        // title:, type:, cssVars: and closing braces outside a mode map.
        let Some(dark) = mode else { continue };
        let entry = list
            .last_mut()
            .ok_or_else(|| format!("themes.ts:{n}: var before any `name:`"))?;
        let (key, value) = t
            .strip_suffix(',')
            .and_then(|kv| kv.split_once(": "))
            .ok_or_else(|| format!("themes.ts:{n}: expected `key: \"value\",`: {t}"))?;
        let key = key.trim_matches('"');
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .ok_or_else(|| format!("themes.ts:{n}: value is not a string: {t}"))?;
        if key == "radius" {
            let rem = value
                .strip_suffix("rem")
                .ok_or_else(|| format!("themes.ts:{n}: radius is not in rem: {value}"))?;
            entry.radius_rem = Some(num(rem).map_err(|e| format!("themes.ts:{n}: {e}"))?);
            continue;
        }
        let c = parse_oklch(value).map_err(|e| format!("themes.ts:{n}: {e}"))?;
        let vars = if dark {
            &mut entry.dark
        } else {
            &mut entry.light
        };
        vars.push((key.to_owned(), c));
    }
    let mut out = Themes::default();
    for id in &base_ids {
        if !all.iter().any(|e| &e.name == id) {
            return Err(format!(
                "base-colors.ts names `{id}`, which themes.ts lacks"
            ));
        }
    }
    for e in all {
        if base_ids.contains(&e.name) {
            out.bases.push(e);
        } else {
            out.overlays.push(e);
        }
    }
    Ok(out)
}

/// The vars of one mode as generated tuple lines; sidebar vars are skipped.
fn var_lines(theme: &str, mode: &str, vars: &[(String, Oklch)]) -> Result<String, String> {
    let mut s = String::new();
    for (name, c) in vars {
        if name.starts_with("sidebar") {
            continue;
        }
        let v = VARS
            .iter()
            .find(|v| v.0 == name)
            .ok_or_else(|| format!("{theme} {mode}: unknown var `{name}`"))?
            .1;
        s += &format!(
            "        ({v}, O({:?}, {:?}, {:?}, {:?})),\n",
            c.l, c.c, c.h, c.alpha
        );
    }
    Ok(s)
}

/// `crates/shadcn-egui/src/palette/generated.rs`.
pub fn emit_palettes(pin: &str, themes: &Themes) -> Result<String, String> {
    let mut s = crate::emit::header_react(pin, "`apps/v4/registry/themes.ts` and `base-colors.ts`");
    s += "use super::{Oklch as O, Var, Var::*};\n\n";
    s += "/// The base colours (`base-colors.ts`): (id, radius in rem, light vars, dark vars). Every var is present.\n";
    s += "pub(crate) const BASES: &[(&str, f32, &[(Var, O)], &[(Var, O)])] = &[\n";
    for b in &themes.bases {
        let radius = b
            .radius_rem
            .ok_or_else(|| format!("base `{}` has no radius", b.name))?;
        for (mode, vars) in [("light", &b.light), ("dark", &b.dark)] {
            let missing: Vec<&str> = VARS
                .iter()
                .map(|v| v.0)
                .filter(|k| !vars.iter().any(|(n, _)| n == k))
                .collect();
            if !missing.is_empty() {
                return Err(format!(
                    "base `{}` {mode} lacks {}",
                    b.name,
                    missing.join(", ")
                ));
            }
        }
        s += &format!(
            "    (\"{}\", {radius:?}, &[\n{}    ], &[\n{}    ]),\n",
            b.name,
            var_lines(&b.name, "light", &b.light)?,
            var_lines(&b.name, "dark", &b.dark)?
        );
    }
    s += "];\n\n";
    s += "/// The other `THEMES` entries, colour overlays: (id, light vars, dark vars).\n";
    s += "pub(crate) const OVERLAYS: &[(&str, &[(Var, O)], &[(Var, O)])] = &[\n";
    for o in &themes.overlays {
        s += &format!(
            "    (\"{}\", &[\n{}    ], &[\n{}    ]),\n",
            o.name,
            var_lines(&o.name, "light", &o.light)?,
            var_lines(&o.name, "dark", &o.dark)?
        );
    }
    s += "];\n";
    Ok(s)
}
