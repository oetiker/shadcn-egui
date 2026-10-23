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
    HeightInset,
    PadTop,
    PadRight,
    PadBottom,
    PadLeft,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    Gap,
    GapX,
    SpaceX,
    InsetTop,
    InsetRight,
    InsetBottom,
    InsetLeft,
    IconSize,
    IconShiftY,
    IconHidden,
    TextSize,
    LineHeight,
    Tracking,
    Weight,
    Uppercase,
    Underline,
    TextWrap,
    TextAlign,
    Radius,
    RadiusTop,
    RadiusBottom,
    RadiusLeft,
    RadiusRight,
    Bg,
    BgClipPadding,
    Text,
    BorderWidth,
    BorderTopWidth,
    BorderLeftWidth,
    BorderColor,
    RingWidth,
    RingColor,
    OutlineWidth,
    OutlineColor,
    Shadow,
    Opacity,
    Blend,
    ShiftX,
    ShiftY,
    HitInsetX,
    HitInsetY,
    BackdropBlur,
    Hidden,
    AspectRatio,
    ObjectCover,
    FieldSizingContent,
    Resizable,
    Transition,
    Duration,
    AnimateIn,
    AnimateOut,
    EnterOpacity,
    EnterScale,
    EnterX,
    EnterY,
    ExitOpacity,
    ExitScale,
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
            HeightInset => "height_inset",
            PadTop => "pad_top",
            PadRight => "pad_right",
            PadBottom => "pad_bottom",
            PadLeft => "pad_left",
            MarginTop => "margin_top",
            MarginRight => "margin_right",
            MarginBottom => "margin_bottom",
            MarginLeft => "margin_left",
            Gap => "gap",
            GapX => "gap_x",
            SpaceX => "space_x",
            InsetTop => "inset_top",
            InsetRight => "inset_right",
            InsetBottom => "inset_bottom",
            InsetLeft => "inset_left",
            IconSize => "icon_size",
            IconShiftY => "icon_shift_y",
            IconHidden => "icon_hidden",
            TextSize => "text_size",
            LineHeight => "line_height",
            Tracking => "tracking",
            Weight => "weight",
            Uppercase => "uppercase",
            Underline => "underline",
            TextWrap => "text_wrap",
            TextAlign => "text_align",
            Radius => "radius",
            RadiusTop => "radius_top",
            RadiusBottom => "radius_bottom",
            RadiusLeft => "radius_left",
            RadiusRight => "radius_right",
            Bg => "bg",
            BgClipPadding => "bg_clip_padding",
            Text => "text",
            BorderWidth => "border_width",
            BorderTopWidth => "border_top_width",
            BorderLeftWidth => "border_left_width",
            BorderColor => "border_color",
            RingWidth => "ring_width",
            RingColor => "ring_color",
            OutlineWidth => "outline_width",
            OutlineColor => "outline_color",
            Shadow => "shadow",
            Opacity => "opacity",
            Blend => "blend",
            ShiftX => "shift_x",
            ShiftY => "shift_y",
            HitInsetX => "hit_inset_x",
            HitInsetY => "hit_inset_y",
            BackdropBlur => "backdrop_blur",
            Hidden => "hidden",
            AspectRatio => "aspect_ratio",
            ObjectCover => "object_cover",
            FieldSizingContent => "field_sizing_content",
            Resizable => "resizable",
            Transition => "transition",
            Duration => "duration",
            AnimateIn => "animate_in",
            AnimateOut => "animate_out",
            EnterOpacity => "enter_opacity",
            EnterScale => "enter_scale",
            EnterX => "enter_x",
            EnterY => "enter_y",
            ExitOpacity => "exit_opacity",
            ExitScale => "exit_scale",
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
    /// `calc(frac × 100% + px)` of the part's own size.
    Calc(f32, f32),
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
    /// `TextWrap::Balance`, `TextWrap::Pretty`.
    TextWrap(&'static str),
    /// `TextAlign::Left`, `TextAlign::Center`, `TextAlign::Right`.
    TextAlign(&'static str),
    /// `Blend::Darken`, `Blend::Lighten`.
    Blend(&'static str),
    /// The `Transition` const names it ORs together.
    Transition(&'static [&'static str]),
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
            Value::Shift(ShiftV::Calc(f, v)) => format!("Shift::Calc({f:?}, {v:?})"),
            Value::TextWrap(w) => format!("TextWrap::{w}"),
            Value::TextAlign(a) => format!("TextAlign::{a}"),
            Value::Blend(b) => format!("Blend::{b}"),
            Value::Transition(names) => match names.split_first() {
                None => "Transition::NONE".into(),
                Some((first, rest)) => {
                    rest.iter().fold(format!("Transition::{first}"), |acc, n| {
                        format!("{acc}.or(Transition::{n})")
                    })
                }
            },
        }
    }
}

/// `State` const names and bits; must match `src/style/recipe.rs`.
pub const FLAGS: &[(&str, u128)] = &[
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
    ("SIZE_LG", 1 << 26),
    ("FIRST", 1 << 27),
    ("LAST", 1 << 28),
    ("NOT_FIRST", 1 << 29),
    ("NOT_LAST", 1 << 30),
    ("SM", 1 << 31),
    ("MD", 1 << 32),
    ("SIDE_TOP", 1 << 33),
    ("SIDE_RIGHT", 1 << 34),
    ("SIDE_BOTTOM", 1 << 35),
    ("SIDE_LEFT", 1 << 36),
    ("HAS_ICON", 1 << 37),
    ("HAS_ACTION", 1 << 38),
    ("HAS_MEDIA", 1 << 39),
    ("HAS_KBD", 1 << 40),
    ("HAS_BUTTON_GROUP", 1 << 41),
    ("HAS_OUTLINE", 1 << 42),
    ("IN_TOOLTIP_CONTENT", 1 << 43),
    ("IN_AVATAR_GROUP", 1 << 44),
    ("VARIANT_DESTRUCTIVE", 1 << 45),
    ("HAS_SIZE_LG", 1 << 46),
    ("HAS_SIZE_SM", 1 << 47),
    ("CLOSED", 1 << 48),
    ("DELAYED_OPEN", 1 << 49),
    ("IN_GROUP_HORIZONTAL", 1 << 50),
    ("IN_GROUP_VERTICAL", 1 << 51),
];

pub fn flag(name: &str) -> u128 {
    FLAGS
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, b)| *b)
        .unwrap_or_else(|| panic!("unknown flag {name}"))
}

/// `State::A.or(State::B)` for a bit set, `State::NONE` for 0.
pub fn state_expr(bits: u128) -> String {
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
