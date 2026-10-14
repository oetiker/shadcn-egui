//! Tailwind utility → recipe fields. A utility either sets fields, uses or
//! defines a CSS spacing variable, is ignored for a stated reason, or is
//! unsupported. Nothing is guessed: a utility with no rule here is
//! unsupported.

use crate::model::{Field, RadiusV, ShiftV, Value};
use crate::variant::Pseudo;

#[derive(Debug, PartialEq)]
pub enum U {
    Set(Vec<(Field, Value)>),
    /// Fields of the part's `after:` pseudo-element (its `_after` recipe).
    After(Vec<(Field, Value)>),
    UseVar(Vec<Field>, String),
    VarDef(String, f32),
    Ignore(&'static str),
    Unsupported(&'static str),
}

const LAYOUT: &str =
    "size or alignment relative to the parent; egui layout is done by the component";
const ANIM: &str = "animation or transition; out of scope (spec §4.4)";
const STRUCTURE: &str = "structure: egui layout is done by the component";
const BROWSER: &str = "browser behaviour the component handles itself";
/// Also the reason of the `[&_svg]` variant in `variant.rs`.
pub const SVG_CHILD: &str =
    "svg child behaviour (pointer events, shrink); the crate paints icons itself";
const DEFAULT_SIZE: &str = "the component's default sizing";
const ANCHOR: &str = "bits-ui anchor variable: placement computed by egui";
const CENTRE: &str = "dialog centring: egui centres the dialog window";

const IGNORE_EXACT: &[(&str, &str)] = &[
    ("flex", STRUCTURE),
    ("inline-flex", STRUCTURE),
    ("grid", STRUCTURE),
    ("flex-col", STRUCTURE),
    ("flex-1", STRUCTURE),
    ("shrink-0", STRUCTURE),
    ("relative", STRUCTURE),
    ("absolute", STRUCTURE),
    ("isolate", BROWSER),
    ("w-full", LAYOUT),
    ("h-full", LAYOUT),
    ("w-auto", LAYOUT),
    ("block", STRUCTURE),
    ("text-left", LAYOUT),
    ("text-center", LAYOUT),
    ("select-none", BROWSER),
    ("pointer-events-none", BROWSER),
    ("animate-in", ANIM),
    ("animate-out", ANIM),
    (
        "rotate-45",
        "the tooltip arrow is painted as a rotated square by the component",
    ),
    (
        "translate-y-[calc(-50%-2px)]",
        "tooltip arrow placement is component geometry",
    ),
    ("text-balance", "egui has no balanced wrapping"),
    ("fixed", STRUCTURE),
    ("flex-col-reverse", STRUCTURE),
    ("flex-row", STRUCTURE),
    ("grow", STRUCTURE),
    ("peer", BROWSER),
    ("touch-none", BROWSER),
    ("whitespace-nowrap", BROWSER),
    ("w-fit", DEFAULT_SIZE),
    ("h-fit", DEFAULT_SIZE),
    ("top-1/2", CENTRE),
    ("left-1/2", CENTRE),
    ("-translate-x-1/2", CENTRE),
    ("-translate-y-1/2", CENTRE),
    (
        "text-current",
        "inherits the parent's text colour: the recipe leaves `text` empty",
    ),
];
const IGNORE_PREFIX: &[(&str, &str)] = &[
    ("transition", ANIM),
    ("duration-", ANIM),
    ("fade-", ANIM),
    ("zoom-", ANIM),
    ("slide-in-from-", ANIM),
    ("z-", BROWSER),
    ("scroll-my-", BROWSER),
    (
        "underline-offset-",
        "egui draws text underlines at a fixed offset",
    ),
    ("items-", STRUCTURE),
    ("justify-", STRUCTURE),
    ("self-", STRUCTURE),
    ("place-content-", STRUCTURE),
    ("col-", STRUCTURE),
    ("row-", STRUCTURE),
    ("grid-cols-", STRUCTURE),
    ("grid-rows-", STRUCTURE),
    ("auto-rows-", STRUCTURE),
    ("@container/", STRUCTURE),
    ("group/", BROWSER),
    ("cursor-", BROWSER),
    ("overflow-", BROWSER),
    ("line-clamp-", BROWSER),
    ("origin-", ANCHOR),
];

/// Tailwind spacing: `N` → N×4px, `px` → 1, `[Npx]`, `[Nrem]` (16px rem),
/// `--spacing(N)`, `[calc(--spacing(N))]`.
pub fn spacing(s: &str) -> Option<f32> {
    if s == "px" {
        return Some(1.0);
    }
    if s == "0" {
        return Some(0.0);
    }
    if let Some(inner) = s.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        if let Some(n) = inner
            .strip_prefix("calc(--spacing(")
            .and_then(|r| r.strip_suffix("))"))
        {
            return n.parse::<f32>().ok().map(|n| n * 4.0);
        }
        return length(inner);
    }
    if let Some(n) = s
        .strip_prefix("--spacing(")
        .and_then(|r| r.strip_suffix(')'))
    {
        return n.parse::<f32>().ok().map(|n| n * 4.0);
    }
    s.parse::<f32>().ok().map(|n| n * 4.0)
}

fn length(s: &str) -> Option<f32> {
    if let Some(v) = s.strip_suffix("px") {
        return v.parse().ok();
    }
    if let Some(v) = s.strip_suffix("rem") {
        return v.parse::<f32>().ok().map(|v| v * 16.0);
    }
    None
}

/// `primary/80` → ("Primary", 0.8). Roles are the shadcn CSS variables. The
/// opacity modifier scales the role colour's alpha; it is never a mix with
/// `transparent`.
pub fn role(s: &str) -> Option<(&'static str, f32)> {
    let (name, alpha) = match s.split_once('/') {
        Some((n, a)) => (n, a.parse::<f32>().ok()? / 100.0),
        None => (s, 1.0),
    };
    let r = match name {
        "background" => "Background",
        "foreground" => "Foreground",
        "card" => "Card",
        "card-foreground" => "CardForeground",
        "popover" => "Popover",
        "popover-foreground" => "PopoverForeground",
        "primary" => "Primary",
        "primary-foreground" => "PrimaryForeground",
        "secondary" => "Secondary",
        "secondary-foreground" => "SecondaryForeground",
        "muted" => "Muted",
        "muted-foreground" => "MutedForeground",
        "accent" => "Accent",
        "accent-foreground" => "AccentForeground",
        "destructive" => "Destructive",
        "destructive-foreground" => "DestructiveForeground",
        "border" => "Border",
        "input" => "Input",
        "ring" => "Ring",
        "white" => "White",
        "black" => "Black",
        "transparent" => "Transparent",
        _ => return None,
    };
    Some((r, alpha))
}

fn px(fields: &[Field], v: f32) -> U {
    U::Set(fields.iter().map(|f| (*f, Value::Px(v))).collect())
}

/// Spacing utility with a value that may be a CSS variable `(--name)`.
fn spaced(fields: &[Field], value: &str, sign: f32) -> U {
    if let Some(name) = value.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        if sign < 0.0 {
            return U::Unsupported("negated spacing variable");
        }
        return U::UseVar(fields.to_vec(), name.to_owned());
    }
    match spacing(value) {
        Some(v) => px(fields, v * sign),
        None => U::Unsupported("spacing value"),
    }
}

pub fn utility(u: &str, pseudo: Option<Pseudo>) -> U {
    use Field::*;
    match pseudo {
        Some(Pseudo::Svg) => {
            if let Some(v) = u.strip_prefix("size-").and_then(spacing) {
                return px(&[IconSize], v);
            }
            let ignored = IGNORE_EXACT.iter().any(|(k, _)| *k == u)
                || IGNORE_PREFIX.iter().any(|(k, _)| u.starts_with(k));
            return if ignored {
                U::Ignore(SVG_CHILD)
            } else {
                U::Unsupported("svg child utility other than size-*")
            };
        }
        Some(Pseudo::After) => {
            // `after:-inset-*` grows the hit area (checkbox, switch, slider thumb).
            for (prefix, fields) in [
                ("-inset-x-", &[HitInsetX][..]),
                ("-inset-y-", &[HitInsetY][..]),
                ("-inset-", &[HitInsetX, HitInsetY][..]),
            ] {
                if let Some(v) = u.strip_prefix(prefix) {
                    return spaced(fields, v, 1.0);
                }
            }
            // Anything else draws the pseudo-element: the part's `_after` recipe.
            // A CSS variable has no place there: `_after` takes fields only.
            return match utility(u, None) {
                U::Set(list) => U::After(list),
                U::UseVar(..) | U::VarDef(..) => {
                    U::Unsupported("CSS variable under after:; the _after recipe takes fields only")
                }
                other @ (U::After(_) | U::Ignore(_) | U::Unsupported(_)) => other,
            };
        }
        None => {}
    }
    if let Some((_, why)) = IGNORE_EXACT.iter().find(|(k, _)| *k == u) {
        return U::Ignore(why);
    }
    if let Some((_, why)) = IGNORE_PREFIX.iter().find(|(k, _)| u.starts_with(k)) {
        return U::Ignore(why);
    }
    if u.contains("(--bits-") {
        return U::Ignore(ANCHOR);
    }
    if let Some(n) = u
        .strip_prefix("h-[calc(100%-")
        .and_then(|r| r.strip_suffix(")]"))
        .and_then(length)
    {
        return px(&[HeightInset], n);
    }
    if let Some(def) = u.strip_prefix("[--").and_then(|r| r.strip_suffix(']')) {
        let (name, value) = def.split_once(':').unwrap_or((def, ""));
        return match spacing(value) {
            Some(v) => U::VarDef(format!("--{name}"), v),
            None => U::Unsupported("CSS variable definition"),
        };
    }
    // Negative values: `-mx-4`, `-translate-y-px`.
    let (sign, body) = match u.strip_prefix('-') {
        Some(b) => (-1.0, b),
        None => (1.0, u),
    };
    let sides: &[(&str, &[Field])] = &[
        ("size-", &[Height, Width]),
        ("h-", &[Height]),
        ("w-", &[Width]),
        ("min-h-", &[MinHeight]),
        ("min-w-", &[MinWidth]),
        ("px-", &[PadLeft, PadRight]),
        ("py-", &[PadTop, PadBottom]),
        ("pt-", &[PadTop]),
        ("pr-", &[PadRight]),
        ("pb-", &[PadBottom]),
        ("pl-", &[PadLeft]),
        ("p-", &[PadTop, PadRight, PadBottom, PadLeft]),
        ("mx-", &[MarginLeft, MarginRight]),
        ("my-", &[MarginTop, MarginBottom]),
        ("mt-", &[MarginTop]),
        ("mr-", &[MarginRight]),
        ("mb-", &[MarginBottom]),
        ("ml-", &[MarginLeft]),
        ("m-", &[MarginTop, MarginRight, MarginBottom, MarginLeft]),
        ("gap-", &[Gap]),
        ("inset-x-", &[InsetLeft, InsetRight]),
        ("inset-y-", &[InsetTop, InsetBottom]),
        ("inset-", &[InsetTop, InsetRight, InsetBottom, InsetLeft]),
        ("top-", &[InsetTop]),
        ("right-", &[InsetRight]),
        ("bottom-", &[InsetBottom]),
        ("left-", &[InsetLeft]),
    ];
    for (prefix, fields) in sides {
        if let Some(v) = body.strip_prefix(prefix) {
            // `size-N!` arrives without `!` (class::split strips it).
            return spaced(fields, v, sign);
        }
    }
    if let Some(v) = u.strip_prefix("max-w-") {
        return match v {
            "xs" => px(&[MaxWidth], 320.0),
            "sm" => px(&[MaxWidth], 384.0),
            "md" => px(&[MaxWidth], 448.0),
            "lg" => px(&[MaxWidth], 512.0),
            "xl" => px(&[MaxWidth], 576.0),
            "2xl" => px(&[MaxWidth], 672.0),
            _ => match v
                .strip_prefix("[calc(100%-")
                .and_then(|r| r.strip_suffix(")]"))
                .and_then(length)
            {
                Some(n) => px(&[MaxWidthInset], n),
                None => U::Unsupported("max-w value"),
            },
        };
    }
    if let Some(v) = u.strip_prefix("text-") {
        let (size, leading) = v.split_once('/').map_or((v, None), |(s, l)| (s, Some(l)));
        let size_val = match size {
            "xs" => Some(Value::Text("Xs")),
            "sm" => Some(Value::Text("Sm")),
            "base" => Some(Value::Text("Base")),
            "lg" => Some(Value::Text("Lg")),
            _ => size
                .strip_prefix('[')
                .and_then(|r| r.strip_suffix(']'))
                .and_then(length)
                .map(Value::TextPx),
        };
        if let Some(sv) = size_val {
            let mut out = vec![(TextSize, sv)];
            if let Some(l) = leading {
                match leading_factor(l) {
                    Some(f) => out.push((LineHeight, Value::Factor(f))),
                    None => return U::Unsupported("text size / leading"),
                }
            }
            return U::Set(out);
        }
        return match role(v) {
            Some((r, a)) => U::Set(vec![(Text, Value::Paint { role: r, alpha: a })]),
            None => U::Unsupported("text-* value"),
        };
    }
    if let Some(v) = u.strip_prefix("leading-") {
        return leading_factor(v).map_or(U::Unsupported("leading"), |f| {
            U::Set(vec![(LineHeight, Value::Factor(f))])
        });
    }
    if let Some(v) = u.strip_prefix("tracking-") {
        let em = match v {
            "tighter" => -0.05,
            "tight" => -0.025,
            "normal" => 0.0,
            "wide" => 0.025,
            "wider" => 0.05,
            "widest" => 0.1,
            _ => return U::Unsupported("tracking"),
        };
        return U::Set(vec![(Tracking, Value::Factor(em))]);
    }
    if let Some(v) = u.strip_prefix("font-") {
        let w = match v {
            "normal" => "Normal",
            "medium" => "Medium",
            "semibold" => "Semibold",
            "bold" => "Bold",
            _ => return U::Unsupported("font-* other than weight"),
        };
        return U::Set(vec![(Weight, Value::Weight(w))]);
    }
    match u {
        "uppercase" => return U::Set(vec![(Uppercase, Value::Bool(true))]),
        "normal-case" => return U::Set(vec![(Uppercase, Value::Bool(false))]),
        "underline" => return U::Set(vec![(Underline, Value::Bool(true))]),
        "no-underline" => return U::Set(vec![(Underline, Value::Bool(false))]),
        "bg-clip-padding" => return U::Set(vec![(BgClipPadding, Value::Bool(true))]),
        "border" => return px(&[BorderWidth], 1.0),
        "border-none" => return px(&[BorderWidth], 0.0),
        "border-t" => return px(&[BorderTopWidth], 1.0),
        "ring" => return px(&[RingWidth], 1.0),
        "outline" => return px(&[OutlineWidth], 1.0),
        _ => {}
    }
    for (prefix, field) in [
        ("rounded-t-", RadiusTop),
        ("rounded-b-", RadiusBottom),
        ("rounded-", Radius),
    ] {
        if let Some(v) = u.strip_prefix(prefix) {
            return radius(v).map_or(U::Unsupported("radius value"), |r| {
                U::Set(vec![(field, Value::Radius(r))])
            });
        }
    }
    if let Some(v) = u.strip_prefix("bg-") {
        if let Some(mix) = v
            .strip_prefix("[color-mix(in_oklch,var(--")
            .and_then(|r| r.strip_suffix("%)]"))
        {
            // `secondary),var(--foreground)_5`
            let parsed = mix.split_once("),var(--").and_then(|(a, rest)| {
                let (b, pct) = rest.split_once(")_")?;
                Some((role(a)?.0, role(b)?.0, pct.parse::<f32>().ok()? / 100.0))
            });
            return parsed.map_or(U::Unsupported("color-mix form"), |(a, b, t)| {
                U::Set(vec![(Bg, Value::Mix(a, b, t))])
            });
        }
        return role(v).map_or(U::Unsupported("bg-* value"), |(r, a)| {
            U::Set(vec![(Bg, Value::Paint { role: r, alpha: a })])
        });
    }
    if let Some(v) = u.strip_prefix("border-") {
        if let Ok(n) = v.parse::<f32>() {
            return px(&[BorderWidth], n);
        }
        if v.starts_with("b-")
            || v.starts_with("l-")
            || v.starts_with("r-")
            || v.starts_with("x-")
            || v.starts_with("y-")
        {
            return U::Unsupported(
                "single-side border other than border-t (underline-style inputs)",
            );
        }
        return role(v).map_or(U::Unsupported("border-* value"), |(r, a)| {
            U::Set(vec![(BorderColor, Value::Paint { role: r, alpha: a })])
        });
    }
    if let Some(v) = u.strip_prefix("outline-") {
        if v == "none" || v == "hidden" {
            return px(&[OutlineWidth], 0.0);
        }
        if let Ok(n) = v.parse::<f32>() {
            return px(&[OutlineWidth], n);
        }
        return role(v).map_or(U::Unsupported("outline-* value"), |(r, a)| {
            U::Set(vec![(OutlineColor, Value::Paint { role: r, alpha: a })])
        });
    }
    if let Some(v) = u.strip_prefix("fill-") {
        // The tooltip arrow is painted as a shape: its fill is its background.
        return role(v).map_or(U::Unsupported("fill-* value"), |(r, a)| {
            U::Set(vec![(Bg, Value::Paint { role: r, alpha: a })])
        });
    }
    if let Some(v) = u.strip_prefix("ring-") {
        if let Ok(n) = v.parse::<f32>() {
            return px(&[RingWidth], n);
        }
        if let Some(n) = v
            .strip_prefix('[')
            .and_then(|r| r.strip_suffix(']'))
            .and_then(length)
        {
            return px(&[RingWidth], n);
        }
        return role(v).map_or(U::Unsupported("ring-* value"), |(r, a)| {
            U::Set(vec![(RingColor, Value::Paint { role: r, alpha: a })])
        });
    }
    if let Some(v) = u.strip_prefix("shadow-") {
        let s = match v {
            "none" => "None",
            "2xs" => "Xs2",
            "xs" => "Xs",
            "sm" => "Sm",
            "md" => "Md",
            "lg" => "Lg",
            "xl" => "Xl",
            "2xl" => "Xl2",
            _ => return U::Unsupported("shadow value"),
        };
        return U::Set(vec![(Shadow, Value::Shadow(s))]);
    }
    if let Some(v) = u.strip_prefix("opacity-") {
        return v.parse::<f32>().map_or(U::Unsupported("opacity"), |n| {
            U::Set(vec![(Opacity, Value::Factor(n / 100.0))])
        });
    }
    for (prefix, field) in [("translate-x-", ShiftX), ("translate-y-", ShiftY)] {
        if let Some(v) = body.strip_prefix(prefix) {
            if let Some(n) = v
                .strip_prefix("[calc(100%-")
                .and_then(|r| r.strip_suffix(")]"))
                .and_then(length)
            {
                return U::Set(vec![(field, Value::Shift(ShiftV::SelfMinus(n)))]);
            }
            return spacing(v).map_or(U::Unsupported("translate value"), |n| {
                U::Set(vec![(field, Value::Shift(ShiftV::Px(n * sign)))])
            });
        }
    }
    if let Some(v) = u.strip_prefix("backdrop-blur-") {
        let n = match v {
            "xs" => 4.0,
            "sm" => 8.0,
            "md" => 12.0,
            "lg" => 16.0,
            "xl" => 24.0,
            _ => return U::Unsupported("blur value"),
        };
        return px(&[BackdropBlur], n);
    }
    U::Unsupported("no translation rule")
}

fn leading_factor(v: &str) -> Option<f32> {
    Some(match v {
        "none" => 1.0,
        "tight" => 1.25,
        "snug" => 1.375,
        "normal" => 1.5,
        "relaxed" => 1.625,
        "loose" => 2.0,
        _ => return None,
    })
}

fn radius(v: &str) -> Option<RadiusV> {
    let rung = |s: &str| -> Option<&'static str> {
        Some(match s {
            "none" => "None",
            "sm" => "Sm",
            "md" => "Md",
            "lg" => "Lg",
            "xl" => "Xl",
            "2xl" => "Xl2",
            "3xl" => "Xl3",
            "4xl" => "Xl4",
            "full" => "Full",
            _ => return None,
        })
    };
    if let Some(r) = rung(v) {
        return Some(RadiusV::Rung(r));
    }
    let inner = v.strip_prefix('[')?.strip_suffix(']')?;
    if let Some(n) = length(inner) {
        return Some(RadiusV::Px(n));
    }
    // min(var(--radius-md),10px)
    let args = inner.strip_prefix("min(var(--radius-")?.strip_suffix(')')?;
    let (r, n) = args.split_once("),")?;
    Some(RadiusV::Min(rung(r)?, length(n)?))
}
