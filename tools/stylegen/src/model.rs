//! The recipe model as stylegen sees it: one [`Field`] per `Recipe` field and
//! the [`Value`]s a translated class can give it. Mirrors
//! `src/style/recipe.rs`; a field or flag added there is added here too.

/// One recipe field; `rust_name` is the field name in `src/style/recipe.rs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Field {
    Height,
    Width,
    MinHeight,
    MinWidth,
    MaxWidth,
    MaxWidthInset,
    PadTop,
    PadRight,
    PadBottom,
    PadLeft,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    Gap,
    InsetTop,
    InsetRight,
    InsetBottom,
    InsetLeft,
    IconSize,
    TextSize,
    LineHeight,
    Tracking,
    Weight,
    Uppercase,
    Underline,
    Radius,
    RadiusTop,
    RadiusBottom,
    Bg,
    BgClipPadding,
    Text,
    BorderWidth,
    BorderTopWidth,
    BorderColor,
    RingWidth,
    RingColor,
    Shadow,
    Opacity,
    ShiftX,
    ShiftY,
    HitInsetX,
    HitInsetY,
    BackdropBlur,
}

impl Field {
    pub fn rust_name(self) -> &'static str {
        use Field::*;
        match self {
            Height => "height",
            Width => "width",
            MinHeight => "min_height",
            MinWidth => "min_width",
            MaxWidth => "max_width",
            MaxWidthInset => "max_width_inset",
            PadTop => "pad_top",
            PadRight => "pad_right",
            PadBottom => "pad_bottom",
            PadLeft => "pad_left",
            MarginTop => "margin_top",
            MarginRight => "margin_right",
            MarginBottom => "margin_bottom",
            MarginLeft => "margin_left",
            Gap => "gap",
            InsetTop => "inset_top",
            InsetRight => "inset_right",
            InsetBottom => "inset_bottom",
            InsetLeft => "inset_left",
            IconSize => "icon_size",
            TextSize => "text_size",
            LineHeight => "line_height",
            Tracking => "tracking",
            Weight => "weight",
            Uppercase => "uppercase",
            Underline => "underline",
            Radius => "radius",
            RadiusTop => "radius_top",
            RadiusBottom => "radius_bottom",
            Bg => "bg",
            BgClipPadding => "bg_clip_padding",
            Text => "text",
            BorderWidth => "border_width",
            BorderTopWidth => "border_top_width",
            BorderColor => "border_color",
            RingWidth => "ring_width",
            RingColor => "ring_color",
            Shadow => "shadow",
            Opacity => "opacity",
            ShiftX => "shift_x",
            ShiftY => "shift_y",
            HitInsetX => "hit_inset_x",
            HitInsetY => "hit_inset_y",
            BackdropBlur => "backdrop_blur",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RadiusV {
    Rung(&'static str),
    Px(f32),
    Min(&'static str, f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShiftV {
    Px(f32),
    SelfMinus(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Px(f32),
    Factor(f32),
    Bool(bool),
    Text(&'static str),
    TextPx(f32),
    /// A role colour with its alpha scaled by `alpha` (`bg-primary/80`).
    Paint {
        role: &'static str,
        alpha: f32,
    },
    Mix(&'static str, &'static str, f32),
    Radius(RadiusV),
    Weight(&'static str),
    Shadow(&'static str),
    Shift(ShiftV),
}

impl Value {
    /// A Rust expression, valid with `use crate::style::*;` in scope.
    pub fn rust(&self) -> String {
        match *self {
            Value::Px(v) | Value::Factor(v) => format!("{v:?}"),
            Value::Bool(b) => b.to_string(),
            Value::Text(t) => format!("TextSize::{t}"),
            Value::TextPx(v) => format!("TextSize::Px({v:?})"),
            Value::Paint { role, alpha } => format!("Paint::Role(Role::{role}, {alpha:?})"),
            Value::Mix(a, b, t) => format!("Paint::Mix(Role::{a}, Role::{b}, {t:?})"),
            Value::Radius(RadiusV::Rung(r)) => format!("Radius::Rung(Rung::{r})"),
            Value::Radius(RadiusV::Px(v)) => format!("Radius::Px({v:?})"),
            Value::Radius(RadiusV::Min(r, v)) => format!("Radius::Min(Rung::{r}, {v:?})"),
            Value::Weight(w) => format!("Weight::{w}"),
            Value::Shadow(s) => format!("ShadowLevel::{s}"),
            Value::Shift(ShiftV::Px(v)) => format!("Shift::Px({v:?})"),
            Value::Shift(ShiftV::SelfMinus(v)) => format!("Shift::SelfMinus({v:?})"),
        }
    }
}

/// `State` const names and bits; must match `src/style/recipe.rs`.
pub const FLAGS: &[(&str, u32)] = &[
    ("DARK", 1 << 0),
    ("HOVER", 1 << 1),
    ("FOCUS", 1 << 2),
    ("FOCUS_VISIBLE", 1 << 3),
    ("ACTIVE", 1 << 4),
    ("DISABLED", 1 << 5),
    ("INVALID", 1 << 6),
    ("CHECKED", 1 << 7),
    ("UNCHECKED", 1 << 8),
    ("EXPANDED", 1 << 9),
    ("HAS_POPUP", 1 << 10),
    ("SELECTED", 1 << 11),
    ("PLACEHOLDER", 1 << 12),
    ("ICON_START", 1 << 16),
    ("ICON_END", 1 << 17),
    ("IN_BUTTON_GROUP", 1 << 18),
    ("SIZE_SM", 1 << 19),
    ("SIZE_DEFAULT", 1 << 20),
    ("VERTICAL", 1 << 21),
    ("HORIZONTAL", 1 << 22),
    ("VARIANT_LINE", 1 << 23),
    ("VARIANT_DEFAULT", 1 << 24),
    ("HAS_FOOTER", 1 << 25),
];

pub fn flag(name: &str) -> u32 {
    FLAGS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| *b)
        .expect("known flag")
}

/// `State::A.or(State::B)` for a bit set, `State::NONE` for 0.
pub fn state_expr(bits: u32) -> String {
    let names: Vec<String> = FLAGS
        .iter()
        .filter(|(_, b)| bits & b != 0)
        .map(|(n, _)| format!("State::{n}"))
        .collect();
    match names.split_first() {
        None => "State::NONE".into(),
        Some((first, rest)) => rest
            .iter()
            .fold(first.clone(), |acc, n| format!("{acc}.or({n})")),
    }
}
