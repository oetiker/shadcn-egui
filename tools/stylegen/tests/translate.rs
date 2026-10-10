use std::cmp::Ordering;

use stylegen::class::split;
use stylegen::css::parse_style_css;
use stylegen::markers::parse_markers;
use stylegen::model::{flag, Field, Value, FLAGS};
use stylegen::utility::{utility, U};
use stylegen::variant::{
    class_variants, rank_of, tw_cmp, variant, Pseudo, Skip, Variants, V, IGNORED, R_IMPORTANT, TRANSLATED,
};

fn set(u: &str) -> Vec<(Field, Value)> {
    match utility(u, None) {
        U::Set(v) => v,
        other => panic!("{u}: {other:?}"),
    }
}

fn read(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{path}", env!("CARGO_MANIFEST_DIR"))).expect(path)
}

#[test]
fn spacing_scale() {
    assert_eq!(set("h-8"), [(Field::Height, Value::Px(32.0))]);
    assert_eq!(set("px-2.5"), [(Field::PadLeft, Value::Px(10.0)), (Field::PadRight, Value::Px(10.0))]);
    assert_eq!(set("size-3"), [(Field::Height, Value::Px(12.0)), (Field::Width, Value::Px(12.0))]);
    assert_eq!(set("h-[18.4px]"), [(Field::Height, Value::Px(18.4))]);
    assert_eq!(set("h-px"), [(Field::Height, Value::Px(1.0))]);
    assert_eq!(set("-mx-4"), [(Field::MarginLeft, Value::Px(-16.0)), (Field::MarginRight, Value::Px(-16.0))]);
    assert_eq!(set("p-[3px]").len(), 4);
    assert_eq!(set("py-[calc(--spacing(1.25))]"), [(Field::PadTop, Value::Px(5.0)), (Field::PadBottom, Value::Px(5.0))]);
    assert_eq!(set("max-w-sm"), [(Field::MaxWidth, Value::Px(384.0))]);
    assert_eq!(set("max-w-[calc(100%-2rem)]"), [(Field::MaxWidthInset, Value::Px(32.0))]);
}

#[test]
fn colours() {
    assert_eq!(set("bg-primary/80"), [(Field::Bg, Value::Paint { role: "Primary", alpha: 0.8 })]);
    assert_eq!(set("text-muted-foreground"), [(Field::Text, Value::Paint { role: "MutedForeground", alpha: 1.0 })]);
    assert_eq!(set("bg-black/10"), [(Field::Bg, Value::Paint { role: "Black", alpha: 0.1 })]);
    assert_eq!(set("border-transparent"), [(Field::BorderColor, Value::Paint { role: "Transparent", alpha: 1.0 })]);
    assert_eq!(set("ring-foreground/10"), [(Field::RingColor, Value::Paint { role: "Foreground", alpha: 0.1 })]);
    assert_eq!(
        set("bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]"),
        [(Field::Bg, Value::Mix("Secondary", "Foreground", 0.05))]
    );
    // An opacity modifier scales the role's alpha; it is never a mix.
    assert_eq!(Value::Paint { role: "Input", alpha: 0.3 }.rust(), "Paint::Role(Role::Input, 0.3)");
}

#[test]
fn widths_vs_colours() {
    assert_eq!(set("border"), [(Field::BorderWidth, Value::Px(1.0))]);
    assert_eq!(set("border-2"), [(Field::BorderWidth, Value::Px(2.0))]);
    assert_eq!(set("border-t"), [(Field::BorderTopWidth, Value::Px(1.0))]);
    assert_eq!(set("ring-3"), [(Field::RingWidth, Value::Px(3.0))]);
    assert_eq!(set("ring-1"), [(Field::RingWidth, Value::Px(1.0))]);
    assert_eq!(set("text-sm"), [(Field::TextSize, Value::Text("Sm"))]);
    assert_eq!(set("text-[0.8rem]"), [(Field::TextSize, Value::TextPx(12.8))]);
    assert_eq!(set("text-xs/relaxed"), [(Field::TextSize, Value::Text("Xs")), (Field::LineHeight, Value::Factor(1.625))]);
}

#[test]
fn radius_and_misc() {
    use stylegen::model::{RadiusV, ShiftV};
    assert_eq!(set("rounded-lg"), [(Field::Radius, Value::Radius(RadiusV::Rung("Lg")))]);
    assert_eq!(set("rounded-4xl"), [(Field::Radius, Value::Radius(RadiusV::Rung("Xl4")))]);
    assert_eq!(set("rounded-[4px]"), [(Field::Radius, Value::Radius(RadiusV::Px(4.0)))]);
    assert_eq!(set("rounded-[min(var(--radius-md),10px)]"), [(Field::Radius, Value::Radius(RadiusV::Min("Md", 10.0)))]);
    assert_eq!(set("rounded-b-xl"), [(Field::RadiusBottom, Value::Radius(RadiusV::Rung("Xl")))]);
    assert_eq!(set("shadow-md"), [(Field::Shadow, Value::Shadow("Md"))]);
    assert_eq!(set("font-medium"), [(Field::Weight, Value::Weight("Medium"))]);
    assert_eq!(set("bg-clip-padding"), [(Field::BgClipPadding, Value::Bool(true))]);
    assert_eq!(set("translate-y-px"), [(Field::ShiftY, Value::Shift(ShiftV::Px(1.0)))]);
    assert_eq!(set("translate-x-[calc(100%-2px)]"), [(Field::ShiftX, Value::Shift(ShiftV::SelfMinus(2.0)))]);
}

#[test]
fn pseudo_targets() {
    assert!(matches!(utility("size-4", Some(Pseudo::Svg)), U::Set(v) if v == [(Field::IconSize, Value::Px(16.0))]));
    assert!(matches!(utility("-inset-2", Some(Pseudo::After)), U::Set(v) if v.len() == 2));
    assert!(matches!(utility("-inset-x-3", Some(Pseudo::After)), U::Set(v) if v == [(Field::HitInsetX, Value::Px(12.0))]));
}

#[test]
fn vars() {
    assert!(matches!(utility("[--card-spacing:--spacing(4)]", None), U::VarDef(n, v) if n == "--card-spacing" && v == 16.0));
    assert!(matches!(utility("px-(--card-spacing)", None), U::UseVar(f, n) if f == [Field::PadLeft, Field::PadRight] && n == "--card-spacing"));
    assert!(matches!(utility("-mx-(--card-spacing)", None), U::Unsupported(_)));
}

#[test]
fn ignored_and_unsupported() {
    assert!(matches!(utility("flex", None), U::Ignore(_)));
    assert!(matches!(utility("duration-100", None), U::Ignore(_)));
    assert!(matches!(utility("border-b-input", None), U::Unsupported(_)));
    assert!(matches!(utility("bogus-utility", None), U::Unsupported(_)));
}

fn rank(v: &str) -> u32 {
    rank_of(v).unwrap_or_else(|| panic!("`{v}` has no rank"))
}

#[test]
fn variants() {
    assert!(matches!(variant("dark"), V::Cond { all: 1, none: 0, rank: r } if r == rank("dark")));
    assert!(matches!(variant("hover"), V::Cond { rank: r, .. } if r == rank("hover")));
    assert!(matches!(variant("has-data-[icon=inline-start]"), V::Cond { all, rank: r, .. } if all == 1 << 16 && r == rank("has-data-[icon=inline-start]")));
    assert!(matches!(variant("not-aria-[haspopup]"), V::Cond { all: 0, none, .. } if none == 1 << 10));
    assert!(matches!(variant("not-dark"), V::Cond { all: 0, none: 1, .. }));
    assert!(matches!(variant("group-data-[size=sm]/switch"), V::Cond { all, .. } if all == 1 << 19));
    assert!(matches!(variant("md"), V::Always { rank: r } if r == rank("md")));
    assert!(matches!(variant("[&_svg:not([class*='size-'])]"), V::Pseudo(Pseudo::Svg, _)));
    assert!(matches!(variant("after"), V::Pseudo(Pseudo::After, _)));
    assert!(matches!(variant("data-open"), V::Ignore(_)));
    assert!(matches!(variant("[a]"), V::Ignore(_)));
    assert!(matches!(variant("lg"), V::Unsupported));
    // Only exact, listed text translates: an unknown group name is not guessed.
    assert!(matches!(variant("group-data-[size=sm]/other"), V::Unsupported));
}

/// Facts from tailwindcss 4.3.3 `variants.ts`: registration order of the
/// roots, and `Variants.compare` inside one root.
#[test]
fn tailwind_variant_order() {
    let chain = [
        "*", "**", "not-dark", "group-hover", "peer-hover", "file", "after", "first", "last",
        "focus-within", "hover", "focus", "focus-visible", "active", "disabled", "in-data-x",
        "has-data-x", "aria-x", "data-x", "supports-x", "sm", "md", "lg", "dark", "[&>svg]",
    ];
    for w in chain.windows(2) {
        assert_eq!(tw_cmp(w[0], w[1]), Some(Ordering::Less), "{} < {}", w[0], w[1]);
    }
    let less = |a: &str, z: &str| assert_eq!(tw_cmp(a, z), Some(Ordering::Less), "{a} < {z}");
    // Inside a root: named values by text, then arbitrary values by text.
    less("aria-checked", "aria-invalid");
    less("data-vertical", "data-[active=true]");
    less("data-[size=default]", "data-[size=sm]");
    // Compound: inner variant first, then no modifier before a modifier,
    // then modifiers by text.
    less("group-has-disabled/field", "group-data-horizontal/tabs");
    less("group-data-[size=sm]", "group-data-[size=sm]/card");
    less("group-data-[size=sm]/card", "group-data-[size=sm]/switch");
    // Arbitrary variants come after all named ones, ordered by the decoded
    // selector (`_` is a space, so `[&_svg…]` sorts before `[&>svg]`).
    less("[&_svg:not([class*='size-'])]", "[&>svg]");
    assert_eq!(tw_cmp("hover", "hover"), Some(Ordering::Equal));
    assert_eq!(tw_cmp("bogus", "hover"), None);
    // Ranks are dense indices in that order.
    assert!(rank("aria-checked") < rank("aria-invalid"));
    assert!(rank("data-checked") < rank("dark"));
}

/// The rank table is sorted by the port of `Variants.compare`: for every
/// pair, rank order equals Tailwind order, and every rank stays below the
/// important bit.
#[test]
fn ranks_follow_tailwind_order() {
    for (a, _) in TRANSLATED {
        assert!(rank(a) < R_IMPORTANT);
        for (z, _) in TRANSLATED {
            assert_eq!(Some(rank(a).cmp(&rank(z))), tw_cmp(a, z), "`{a}` vs `{z}`");
        }
    }
}

#[test]
fn tables_are_consistent() {
    for (name, _) in TRANSLATED {
        assert!(IGNORED.iter().all(|(n, _)| n != name), "`{name}` is both translated and ignored");
        assert!(TRANSLATED.iter().filter(|(n, _)| n == name).count() == 1, "`{name}` listed twice");
        assert!(rank_of(name).is_some());
    }
    for (name, why) in IGNORED {
        assert!(!why.is_empty(), "`{name}` has no reason");
        assert!(IGNORED.iter().filter(|(n, _)| n == name).count() == 1, "`{name}` listed twice");
    }
}

/// `model::FLAGS` and `Field::rust_name` must name exactly what
/// `src/style/recipe.rs` defines.
#[test]
fn model_matches_recipe_rs() {
    let src = read("../../src/style/recipe.rs");
    let consts: Vec<(String, u32)> = src
        .lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix("pub const ")?;
            let (name, value) = rest.split_once(": State = State(")?;
            let bit = value.strip_prefix("1 << ")?.strip_suffix(");")?.parse::<u32>().ok()?;
            Some((name.to_owned(), 1 << bit))
        })
        .collect();
    let flags: Vec<(String, u32)> = FLAGS.iter().map(|(n, b)| (n.to_string(), *b)).collect();
    assert_eq!(flags, consts);

    let fields: Vec<String> = src
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub ")?.split_once(": Prop<").map(|(n, _)| n.to_owned()))
        .collect();
    use Field::*;
    let all = [
        Height, Width, MinHeight, MinWidth, MaxWidth, MaxWidthInset, PadTop, PadRight, PadBottom, PadLeft,
        MarginTop, MarginRight, MarginBottom, MarginLeft, Gap, InsetTop, InsetRight, InsetBottom, InsetLeft,
        IconSize, TextSize, LineHeight, Tracking, Weight, Uppercase, Underline, Radius, RadiusTop, RadiusBottom,
        Bg, BgClipPadding, Text, BorderWidth, BorderTopWidth, BorderColor, RingWidth, RingColor, Shadow, Opacity, ShiftX, ShiftY,
        HitInsetX, HitInsetY, BackdropBlur,
    ];
    let names: Vec<String> = all.iter().map(|f| f.rust_name().to_owned()).collect();
    assert_eq!(names, fields);
}

fn variants_of(raw: &str) -> Result<Variants, String> {
    class_variants(&split(raw)).map_err(|e| match e {
        Skip::Ignore(_) => "ignored".to_owned(),
        Skip::Unsupported(why) => format!("{why} unsupported"),
    })
}

/// `marker → classes` of one style.
type Css = Vec<(String, Vec<String>)>;

fn fixtures() -> [(&'static str, Css); 2] {
    [
        ("vega", parse_style_css(&read("tests/fixtures/style-vega.css"))),
        ("nova", parse_style_css(&read("tests/fixtures/style-nova.css"))),
    ]
}

fn shipped() -> Vec<Vec<String>> {
    parse_markers(&read("markers.txt")).into_iter().map(|l| l.markers).collect()
}

/// Every class of every shipped marker in vega and nova translates or is
/// ignored with a reason.
#[test]
fn vega_and_nova_have_no_unsupported_class() {
    let shipped: Vec<String> = shipped().concat();
    let mut bad = Vec::new();
    for (style, css) in fixtures() {
        for (marker, classes) in css.iter().filter(|(m, _)| shipped.contains(m)) {
            for raw in classes {
                let why = match variants_of(raw) {
                    Err(why) if why != "ignored" => why,
                    Err(_) => continue,
                    Ok(t) => match utility(&split(raw).utility, t.pseudo) {
                        U::Unsupported(why) => why.to_owned(),
                        _ => continue,
                    },
                };
                bad.push(format!("{style} {marker} `{raw}`: {why}"));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// A rank-0 conditional rule would tie with base values and mis-cascade
/// (`pick` in recipe.rs), so every class with a condition must carry a
/// nonzero rank. Checked over every marker of vega and nova.
#[test]
fn conditional_classes_have_nonzero_rank() {
    let mut checked = 0;
    for (_, css) in fixtures() {
        for raw in css.iter().flat_map(|(_, c)| c) {
            if let Ok(t) = variants_of(raw) {
                if t.all != 0 || t.none != 0 {
                    assert!(t.rank > 0, "`{raw}` is conditional with rank 0");
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 100, "{checked}");
}

/// The cascade the recipe models (`pick`: highest rank bitmask wins, then the
/// later layer, then the later rule) must agree with the browser, where
/// `!important` decides first, then selector specificity, then source order. In Tailwind's output
/// most variants add one class, attribute or pseudo-class of specificity;
/// `in-*` (`:where(…) &`) and the media/supports variants add none, and of
/// the svg targets only `[&_svg:not([class*='size-'])]` adds one. For every
/// pair of rules on one field that can match together, within the marker
/// stack of one shipped component, the rank order must equal the
/// specificity-then-rank order.
#[test]
fn rank_order_agrees_with_specificity() {
    fn specificity(v: &str) -> u32 {
        let none = v.starts_with("in-") || ["sm", "md", "supports-backdrop-filter", "after", "[&>svg]"].contains(&v);
        u32::from(!none)
    }
    struct Rule { field: Field, value: Value, all: u32, none: u32, key: (u128, usize, usize), spec: u32, raw: String }
    let exclusive = [("SIZE_SM", "SIZE_DEFAULT"), ("VERTICAL", "HORIZONTAL"), ("CHECKED", "UNCHECKED"), ("VARIANT_LINE", "VARIANT_DEFAULT")];
    let mut pairs = 0;
    for (style, css) in fixtures() {
        for stack in shipped() {
            let mut rules = Vec::new();
            for (layer, marker) in stack.iter().enumerate() {
                let Some((_, classes)) = css.iter().find(|(m, _)| m == marker) else { continue };
                for (idx, raw) in classes.iter().enumerate() {
                    let Ok(t) = variants_of(raw) else { continue };
                    let c = split(raw);
                    let U::Set(list) = utility(&c.utility, t.pseudo) else { continue };
                    let spec = c.variants.iter().map(|v| specificity(v)).sum();
                    for (field, value) in list {
                        rules.push(Rule { field, value, all: t.all, none: t.none, key: (t.rank, layer, idx), spec, raw: raw.clone() });
                    }
                }
            }
            for a in &rules {
                for b in &rules {
                    let both = a.all | b.all;
                    let can_match_together = a.all & b.none == 0
                        && b.all & a.none == 0
                        && !exclusive.iter().any(|(x, y)| both & flag(x) != 0 && both & flag(y) != 0);
                    if a.field != b.field || a.value == b.value || !can_match_together || a.key <= b.key {
                        continue;
                    }
                    pairs += 1;
                    let important = |r: &Rule| r.key.0 >> R_IMPORTANT;
                    assert!(important(a) >= important(b), "{style}: `{}` outranks important `{}`", a.raw, b.raw);
                    if important(a) == important(b) {
                        assert!(a.spec >= b.spec, "{style}: `{}` outranks `{}` but has lower specificity", a.raw, b.raw);
                    }
                }
            }
        }
    }
    assert!(pairs > 50, "{pairs}");
}

/// cn-checkbox (vega and nova), the case a rank per variant *group* gets
/// wrong: when the box is invalid and checked,
/// `aria-invalid:aria-checked:border-primary` must beat
/// `aria-invalid:border-destructive`; `dark:aria-invalid:` beats both.
#[test]
fn checkbox_invalid_and_checked_keeps_primary_border() {
    let both = variants_of("aria-invalid:aria-checked:border-primary").unwrap().rank;
    let one = variants_of("aria-invalid:border-destructive").unwrap().rank;
    let dark = variants_of("dark:aria-invalid:border-destructive/50").unwrap().rank;
    assert!(both > one);
    assert!(dark > both);
}

/// `!important` beats every normal declaration: the class gets the top rank
/// bit above its variant bits (cn-badge `[&>svg]:size-3!`).
#[test]
fn important_class_gets_the_top_rank_bit() {
    let c = split("[&>svg]:size-3!");
    assert!(c.important);
    let v = class_variants(&c).unwrap();
    assert_eq!(v, Variants { all: 0, none: 0, rank: 1 << R_IMPORTANT, pseudo: Some(Pseudo::Svg) });
    assert_eq!(utility(&c.utility, v.pseudo), U::Set(vec![(Field::IconSize, Value::Px(12.0))]));
    // An important plain class outranks the highest-ranked normal one.
    let important = variants_of("h-8!").unwrap().rank;
    let normal = variants_of("dark:aria-invalid:aria-checked:h-9").unwrap().rank;
    assert!(important > normal);
    // Variant bits are kept below the important bit.
    let both = variants_of("dark:h-8!").unwrap().rank;
    assert_eq!(both, (1 << R_IMPORTANT) | (1 << rank("dark")));
    assert!(variants_of("dark:h-8").unwrap().rank < 1 << R_IMPORTANT);
}
