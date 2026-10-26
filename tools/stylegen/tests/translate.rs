use std::cmp::Ordering;

use stylegen::class::split;
use stylegen::css::parse_style_css;
use stylegen::markers::parse_markers;
use stylegen::model::{flag, Field, Value, FLAGS};
use stylegen::resolve::{translate, Vars};
use stylegen::utility::{utility, U};
use stylegen::variant::{
    class_variants, rank_of, tw_cmp, variant, Pseudo, Skip, Variants, IGNORED, R_CLASSES,
    R_IMPORTANT, R_PSEUDO_ELEMENT, TRANSLATED, V,
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
    assert_eq!(
        set("px-2.5"),
        [
            (Field::PadLeft, Value::Px(10.0)),
            (Field::PadRight, Value::Px(10.0))
        ]
    );
    assert_eq!(
        set("size-3"),
        [
            (Field::Height, Value::Px(12.0)),
            (Field::Width, Value::Px(12.0))
        ]
    );
    assert_eq!(set("h-[18.4px]"), [(Field::Height, Value::Px(18.4))]);
    assert_eq!(set("h-px"), [(Field::Height, Value::Px(1.0))]);
    assert_eq!(
        set("-mx-4"),
        [
            (Field::MarginLeft, Value::Px(-16.0)),
            (Field::MarginRight, Value::Px(-16.0))
        ]
    );
    assert_eq!(set("p-[3px]").len(), 4);
    assert_eq!(
        set("py-[calc(--spacing(1.25))]"),
        [
            (Field::PadTop, Value::Px(5.0)),
            (Field::PadBottom, Value::Px(5.0))
        ]
    );
    assert_eq!(set("max-w-sm"), [(Field::MaxWidth, Value::Px(384.0))]);
    assert_eq!(
        set("max-w-[calc(100%-2rem)]"),
        [(Field::MaxWidthInset, Value::Px(32.0))]
    );
}

#[test]
fn colours() {
    assert_eq!(
        set("bg-primary/80"),
        [(
            Field::Bg,
            Value::Paint {
                role: "Primary",
                alpha: 0.8
            }
        )]
    );
    assert_eq!(
        set("text-muted-foreground"),
        [(
            Field::Text,
            Value::Paint {
                role: "MutedForeground",
                alpha: 1.0
            }
        )]
    );
    assert_eq!(
        set("bg-black/10"),
        [(
            Field::Bg,
            Value::Paint {
                role: "Black",
                alpha: 0.1
            }
        )]
    );
    assert_eq!(
        set("border-transparent"),
        [(
            Field::BorderColor,
            Value::Paint {
                role: "Transparent",
                alpha: 1.0
            }
        )]
    );
    assert_eq!(
        set("ring-foreground/10"),
        [(
            Field::RingColor,
            Value::Paint {
                role: "Foreground",
                alpha: 0.1
            }
        )]
    );
    assert_eq!(
        set("bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]"),
        [(Field::Bg, Value::Mix("Secondary", "Foreground", 0.05))]
    );
    // An opacity modifier scales the role's alpha; it is never a mix.
    assert_eq!(
        Value::Paint {
            role: "Input",
            alpha: 0.3
        }
        .rust(),
        "Paint::Role(Role::Input, 0.3)"
    );
}

#[test]
fn widths_vs_colours() {
    assert_eq!(set("border"), [(Field::BorderWidth, Value::Px(1.0))]);
    assert_eq!(set("border-2"), [(Field::BorderWidth, Value::Px(2.0))]);
    assert_eq!(set("border-t"), [(Field::BorderTopWidth, Value::Px(1.0))]);
    assert_eq!(set("ring-3"), [(Field::RingWidth, Value::Px(3.0))]);
    assert_eq!(set("ring-1"), [(Field::RingWidth, Value::Px(1.0))]);
    assert_eq!(set("text-sm"), [(Field::TextSize, Value::Text("Sm"))]);
    assert_eq!(
        set("text-[0.8rem]"),
        [(Field::TextSize, Value::TextPx(12.8))]
    );
    assert_eq!(
        set("text-xs/relaxed"),
        [
            (Field::TextSize, Value::Text("Xs")),
            (Field::LineHeight, Value::Factor(1.625))
        ]
    );
}

#[test]
fn radius_and_misc() {
    use stylegen::model::{RadiusV, ShiftV};
    assert_eq!(
        set("rounded-lg"),
        [(Field::Radius, Value::Radius(RadiusV::Rung("Lg")))]
    );
    assert_eq!(
        set("rounded-4xl"),
        [(Field::Radius, Value::Radius(RadiusV::Rung("Xl4")))]
    );
    assert_eq!(
        set("rounded-[4px]"),
        [(Field::Radius, Value::Radius(RadiusV::Px(4.0)))]
    );
    assert_eq!(
        set("rounded-[min(var(--radius-md),10px)]"),
        [(Field::Radius, Value::Radius(RadiusV::Min("Md", 10.0)))]
    );
    assert_eq!(
        set("rounded-b-xl"),
        [(Field::RadiusBottom, Value::Radius(RadiusV::Rung("Xl")))]
    );
    assert_eq!(set("shadow-md"), [(Field::Shadow, Value::Shadow("Md"))]);
    assert_eq!(
        set("font-medium"),
        [(Field::Weight, Value::Weight("Medium"))]
    );
    assert_eq!(
        set("bg-clip-padding"),
        [(Field::BgClipPadding, Value::Bool(true))]
    );
    assert_eq!(
        set("translate-y-px"),
        [(Field::ShiftY, Value::Shift(ShiftV::Px(1.0)))]
    );
    assert_eq!(
        set("translate-x-[calc(100%-2px)]"),
        [(Field::ShiftX, Value::Shift(ShiftV::Calc(1.0, -2.0)))]
    );
    // Text alignment is a look (rules axiom 8), not layout.
    assert_eq!(
        set("text-center"),
        [(Field::TextAlign, Value::TextAlign("Center"))]
    );
    assert_eq!(
        set("text-left"),
        [(Field::TextAlign, Value::TextAlign("Left"))]
    );
}

#[test]
fn pseudo_targets() {
    assert!(
        matches!(utility("size-4", Some(Pseudo::Svg)), U::Set(v) if v == [(Field::IconSize, Value::Px(16.0))])
    );
    assert!(matches!(utility("-inset-2", Some(Pseudo::After)), U::Set(v) if v.len() == 2));
    assert!(
        matches!(utility("-inset-x-3", Some(Pseudo::After)), U::Set(v) if v == [(Field::HitInsetX, Value::Px(12.0))])
    );
}

#[test]
fn vars() {
    assert!(
        matches!(utility("[--card-spacing:--spacing(4)]", None), U::VarDef(n, v) if n == "--card-spacing" && v == 16.0)
    );
    assert!(
        matches!(utility("px-(--card-spacing)", None), U::UseVar(f, n) if f == [Field::PadLeft, Field::PadRight] && n == "--card-spacing")
    );
    assert!(matches!(
        utility("-mx-(--card-spacing)", None),
        U::Unsupported(_)
    ));
}

#[test]
fn ignored_and_unsupported() {
    assert!(matches!(utility("flex", None), U::Show(_)));
    assert!(matches!(utility("isolate", None), U::Ignore(_)));
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
    assert!(
        matches!(variant("has-data-[icon=inline-start]"), V::Cond { all, rank: r, .. } if all == 1 << 16 && r == rank("has-data-[icon=inline-start]"))
    );
    assert!(
        matches!(variant("not-aria-[haspopup]"), V::Cond { all: 0, none, .. } if none == 1 << 10)
    );
    assert!(matches!(
        variant("not-dark"),
        V::Cond {
            all: 0,
            none: 1,
            ..
        }
    ));
    assert!(
        matches!(variant("group-data-[size=sm]/switch"), V::Cond { all, .. } if all == 1 << 19)
    );
    assert!(
        matches!(variant("md"), V::Cond { all, rank: r, .. } if all == flag("MD") && r == rank("md"))
    );
    assert!(matches!(
        variant("[&_svg:not([class*='size-'])]"),
        V::Pseudo(Pseudo::Svg, _)
    ));
    assert!(matches!(variant("after"), V::Pseudo(Pseudo::After, _)));
    assert!(matches!(variant("data-open"), V::Cond { all, .. } if all == flag("EXPANDED")));
    assert!(matches!(variant("[a]"), V::Ignore(_)));
    assert!(matches!(variant("lg"), V::Unsupported));
    // Only exact, listed text translates: an unknown group name is not guessed.
    assert!(matches!(
        variant("group-data-[size=sm]/other"),
        V::Unsupported
    ));
}

/// Facts from tailwindcss 4.3.3 `variants.ts`: registration order of the
/// roots, and `Variants.compare` inside one root.
#[test]
fn tailwind_variant_order() {
    let chain = [
        "*",
        "**",
        "not-dark",
        "group-hover",
        "peer-hover",
        "file",
        "after",
        "first",
        "last",
        "focus-within",
        "hover",
        "focus",
        "focus-visible",
        "active",
        "disabled",
        "in-data-x",
        "has-data-x",
        "aria-x",
        "data-x",
        "supports-x",
        "sm",
        "md",
        "lg",
        "dark",
        "[&>svg]",
    ];
    for w in chain.windows(2) {
        assert_eq!(
            tw_cmp(w[0], w[1]),
            Some(Ordering::Less),
            "{} < {}",
            w[0],
            w[1]
        );
    }
    let less = |a: &str, z: &str| assert_eq!(tw_cmp(a, z), Some(Ordering::Less), "{a} < {z}");
    // Inside a root: named values by text, then arbitrary values by text.
    less("aria-checked", "aria-invalid");
    less("data-selected", "data-[active=true]");
    // shadcn-svelte's `@custom-variant data-vertical` comes after every
    // built-in variant, `dark` included.
    less("dark", "data-vertical");
    less("data-checked", "data-vertical");
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
    assert!(rank("data-selected") < rank("dark"));
    assert!(rank("dark") < rank("data-checked"));
}

/// The rank table is sorted by the port of `Variants.compare`: for every
/// pair, rank order equals Tailwind order, and every rank stays below the
/// specificity bits.
#[test]
fn ranks_follow_tailwind_order() {
    for (a, _, _) in TRANSLATED {
        assert!(rank(a) < R_CLASSES);
        for (z, _, _) in TRANSLATED {
            assert_eq!(Some(rank(a).cmp(&rank(z))), tw_cmp(a, z), "`{a}` vs `{z}`");
        }
    }
}

#[test]
fn tables_are_consistent() {
    for (name, _, _) in TRANSLATED {
        assert!(
            IGNORED.iter().all(|(n, _)| n != name),
            "`{name}` is both translated and ignored"
        );
        assert!(
            TRANSLATED.iter().filter(|(n, _, _)| n == name).count() == 1,
            "`{name}` listed twice"
        );
        assert!(rank_of(name).is_some());
    }
    for (name, why) in IGNORED {
        assert!(!why.is_empty(), "`{name}` has no reason");
        assert!(
            IGNORED.iter().filter(|(n, _)| n == name).count() == 1,
            "`{name}` listed twice"
        );
    }
}

/// `model::FLAGS` and `Field::rust_name` must name exactly what
/// `src/style/recipe.rs` defines.
#[test]
fn model_matches_recipe_rs() {
    let src = read("../../crates/shadcn-egui/src/style/recipe.rs");
    let consts: Vec<(String, u128)> = src
        .lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix("pub const ")?;
            let (name, value) = rest.split_once(": State = State(")?;
            let bit = value
                .strip_prefix("1 << ")?
                .strip_suffix(");")?
                .parse::<u32>()
                .ok()?;
            Some((name.to_owned(), 1u128 << bit))
        })
        .collect();
    let flags: Vec<(String, u128)> = FLAGS.iter().map(|(n, b)| (n.to_string(), *b)).collect();
    assert_eq!(flags, consts);

    let fields: Vec<String> = src
        .lines()
        .filter_map(|l| {
            l.trim()
                .strip_prefix("pub ")?
                .split_once(": Prop<")
                .map(|(n, _)| n.to_owned())
        })
        .collect();
    use Field::*;
    let all = [
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
        UnderlineOffset,
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
        (
            "vega",
            parse_style_css(&read("tests/fixtures/style-vega.css")),
        ),
        (
            "nova",
            parse_style_css(&read("tests/fixtures/style-nova.css")),
        ),
    ]
}

fn shipped() -> Vec<Vec<String>> {
    parse_markers(&read("markers.txt"))
        .into_iter()
        .map(|l| l.markers)
        .collect()
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

/// The cascade the recipe models (`pick`: highest rank wins, then the later
/// layer, then the later rule) must agree with the browser, where
/// `!important` decides first, then selector specificity, then source order. In Tailwind's output
/// most variants add one class, attribute or pseudo-class of specificity;
/// `placeholder` styles a pseudo-element, which beats the part's own rules
/// whatever their classes (it inherits from them), `in-*` (`:where(…) &`)
/// and the media/supports variants add none, and of the svg targets only
/// `[&_svg:not([class*='size-'])]` adds one class over `[&>svg]`. For every
/// pair of rules on one field that can match together, within the marker
/// stack of one shipped component, the rank order must equal the
/// specificity-then-rank order.
#[test]
fn rank_order_agrees_with_specificity() {
    /// (pseudo-elements, classes), compared in that order.
    fn specificity(v: &str) -> (u32, u32) {
        let none = v.starts_with("in-")
            || ["sm", "md", "supports-backdrop-filter", "after", "[&>svg]"].contains(&v);
        match v {
            "placeholder" => (1, 0),
            _ if none => (0, 0),
            _ => (0, 1),
        }
    }
    struct Rule {
        field: Field,
        value: Value,
        all: u128,
        none: u128,
        key: (u128, usize, usize),
        spec: (u32, u32),
        raw: String,
    }
    let exclusive = [
        ("SIZE_SM", "SIZE_DEFAULT"),
        ("VERTICAL", "HORIZONTAL"),
        ("CHECKED", "UNCHECKED"),
        ("VARIANT_LINE", "VARIANT_DEFAULT"),
    ];
    let mut pairs = 0;
    for (style, css) in fixtures() {
        for stack in shipped() {
            let mut rules = Vec::new();
            for (layer, marker) in stack.iter().enumerate() {
                let Some((_, classes)) = css.iter().find(|(m, _)| m == marker) else {
                    continue;
                };
                for (idx, raw) in classes.iter().enumerate() {
                    let Ok(t) = variants_of(raw) else { continue };
                    let c = split(raw);
                    let U::Set(list) = utility(&c.utility, t.pseudo) else {
                        continue;
                    };
                    let spec = c
                        .variants
                        .iter()
                        .map(|v| specificity(v))
                        .fold((0, 0), |(a, b), (x, y)| (a + x, b + y));
                    for (field, value) in list {
                        rules.push(Rule {
                            field,
                            value,
                            all: t.all,
                            none: t.none,
                            key: (t.rank, layer, idx),
                            spec,
                            raw: raw.clone(),
                        });
                    }
                }
            }
            for a in &rules {
                for b in &rules {
                    let both = a.all | b.all;
                    let can_match_together = a.all & b.none == 0
                        && b.all & a.none == 0
                        && !exclusive
                            .iter()
                            .any(|(x, y)| both & flag(x) != 0 && both & flag(y) != 0);
                    if a.field != b.field
                        || a.value == b.value
                        || !can_match_together
                        || a.key <= b.key
                    {
                        continue;
                    }
                    pairs += 1;
                    let important = |r: &Rule| r.key.0 >> R_IMPORTANT;
                    assert!(
                        important(a) >= important(b),
                        "{style}: `{}` outranks important `{}`",
                        a.raw,
                        b.raw
                    );
                    if important(a) == important(b) {
                        assert!(
                            a.spec >= b.spec,
                            "{style}: `{}` outranks `{}` but has lower specificity",
                            a.raw,
                            b.raw
                        );
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
    let both = variants_of("aria-invalid:aria-checked:border-primary")
        .unwrap()
        .rank;
    let one = variants_of("aria-invalid:border-destructive").unwrap().rank;
    let dark = variants_of("dark:aria-invalid:border-destructive/50")
        .unwrap()
        .rank;
    assert!(both > one);
    assert!(dark > both);
}

/// `!important` beats every normal declaration: the class gets the top rank
/// bit above its specificity and variant bits (cn-badge `[&>svg]:size-3!`).
#[test]
fn important_class_gets_the_top_rank_bit() {
    let c = split("[&>svg]:size-3!");
    assert!(c.important);
    let v = class_variants(&c).unwrap();
    assert_eq!(
        v,
        Variants {
            all: 0,
            none: 0,
            rank: 1 << R_IMPORTANT,
            pseudo: Some(Pseudo::Svg),
            child: None,
        }
    );
    assert_eq!(
        utility(&c.utility, v.pseudo),
        U::Set(vec![(Field::IconSize, Value::Px(12.0))])
    );
    // An important plain class outranks the highest-ranked normal one.
    let important = variants_of("h-8!").unwrap().rank;
    let normal = variants_of("dark:aria-invalid:aria-checked:h-9")
        .unwrap()
        .rank;
    assert!(important > normal);
    // Specificity and variant bits are kept below the important bit.
    let both = variants_of("dark:h-8!").unwrap().rank;
    assert_eq!(
        both,
        (1 << R_IMPORTANT) | (2 << R_CLASSES) | (1 << rank("dark"))
    );
    assert!(variants_of("dark:h-8").unwrap().rank < 1 << R_IMPORTANT);
}

#[test]
fn after_variables_are_unsupported() {
    // A variable under `after:` would land in the part's own declarations.
    assert!(matches!(
        utility("w-(--x)", Some(Pseudo::After)),
        U::Unsupported(_)
    ));
    assert!(matches!(
        utility("[--x:4px]", Some(Pseudo::After)),
        U::Unsupported(_)
    ));
}

/// One reason per spec group ("Ignored classes" in the design spec), and an
/// svg child class ignored for the reason its `[&_svg]` sibling has.
#[test]
fn ignore_reasons_follow_the_spec_groups() {
    let why = |u: &str, p: Option<Pseudo>| match utility(u, p) {
        U::Ignore(w) => w,
        other => panic!("{u}: {other:?}"),
    };
    // Display classes show a hidden part, else are structure (`U::Show`).
    for u in ["flex", "grid", "inline-flex", "block"] {
        assert!(
            matches!(utility(u, None), U::Show(w) if w == why("relative", None)),
            "{u}"
        );
    }
    let structure = [
        "items-center",
        "justify-between",
        "justify-self-end",
        "self-start",
        "place-content-center",
        "relative",
        "absolute",
        "fixed",
        "col-start-2",
        "row-span-2",
        "grid-cols-2",
        "grid-rows-2",
        "auto-rows-min",
        "flex-col",
        "flex-col-reverse",
        "flex-row",
        "flex-1",
        "grow",
        "shrink-0",
        "@container/card-header",
    ];
    for u in structure {
        assert_eq!(why(u, None), why("relative", None), "{u}");
    }
    let browser = [
        "pointer-events-none",
        "select-none",
        "touch-none",
        "cursor-default",
        "scroll-my-1",
        "isolate",
        "z-50",
        "whitespace-nowrap",
        "overflow-hidden",
        "peer",
        "group/button",
    ];
    // Motion classes are looks (rules axiom 8): recipe values.
    for u in ["transition-all", "duration-100", "fade-in-0", "zoom-in-95"] {
        assert!(matches!(utility(u, None), U::Set(_)), "{u}");
    }
    for u in browser {
        assert_eq!(why(u, None), why("peer", None), "{u}");
    }
    let svg = IGNORED.iter().find(|(v, _)| *v == "[&_svg]").unwrap().1;
    assert_eq!(why("pointer-events-none", Some(Pseudo::Svg)), svg);
    assert_eq!(why("shrink-0", Some(Pseudo::Svg)), svg);
    // The prefix list counts too.
    assert_eq!(why("cursor-pointer", Some(Pseudo::Svg)), svg);
}

/// `line-clamp-N` sets `display: -webkit-box`; Tailwind 4.3.3 emits the
/// display utilities after it, so a display class on the same target and
/// variants wins and Chrome clamps nothing (the select value). Without one
/// the clamp is real, and the crate does not build it yet.
#[test]
fn a_display_class_cancels_a_line_clamp() {
    use stylegen::resolve::{CLAMP_CANCELLED, CLAMP_EFFECTIVE};
    let run = |classes: &[&str]| {
        let classes: Vec<String> = classes.iter().map(|c| (*c).to_owned()).collect();
        translate(&classes, &Vars::new())
    };
    let select = run(&[
        "*:data-[slot=select-value]:line-clamp-1",
        "*:data-[slot=select-value]:flex",
    ]);
    assert!(select.ignored.contains(&(
        "*:data-[slot=select-value]:line-clamp-1".into(),
        CLAMP_CANCELLED
    )));
    assert!(select.unsupported.is_empty());
    for classes in [
        &["line-clamp-2"][..],
        &["line-clamp-1", "md:flex"],
        &["flex", "*:line-clamp-1"],
    ] {
        let m = run(classes);
        assert!(
            m.unsupported.iter().any(|(_, why)| why == CLAMP_EFFECTIVE),
            "{classes:?}"
        );
    }
    assert!(run(&["flex", "line-clamp-3"]).unsupported.is_empty());
}

/// The winning value of `field` under `state` in one class list, as `pick`
/// resolves it: the matching decl with the highest rank, the later one on a
/// tie.
fn winner(classes: &[&str], field: Field, state: u128) -> Option<Value> {
    let classes: Vec<String> = classes.iter().map(|c| (*c).to_owned()).collect();
    translate(&classes, &Vars::new())
        .decls
        .into_iter()
        .filter(|d| d.field == field && d.all & !state == 0 && d.none & state == 0)
        .max_by_key(|d| (d.rank, d.idx))
        .map(|d| d.value)
}

/// CSS compares specificity before source order. `data-[size=sm]:h-7` is
/// `.x[data-size=sm]` (0,2,0); `md:h-9` is `.x` inside a media query
/// (0,1,0). Tailwind emits the breakpoint later, but the browser picks h-7.
#[test]
fn specificity_beats_variant_order() {
    let md = flag("MD");
    let sm = flag("SIZE_SM") | md;
    let classes = ["data-[size=sm]:h-7", "md:h-9"];
    assert_eq!(winner(&classes, Field::Height, sm), Some(Value::Px(28.0)));
    assert_eq!(winner(&classes, Field::Height, md), Some(Value::Px(36.0)));
    // Class order does not matter either.
    let swapped = ["md:h-9", "data-[size=sm]:h-7"];
    assert_eq!(winner(&swapped, Field::Height, sm), Some(Value::Px(28.0)));
    // `supports-*` adds no specificity either, although Tailwind emits it
    // after `data-*`.
    let supports = ["data-[size=sm]:h-7", "supports-backdrop-filter:h-6"];
    assert_eq!(winner(&supports, Field::Height, sm), Some(Value::Px(28.0)));
    // Equal specificity: the variant order decides, as before (`dark` is
    // registered after `hover`).
    let tie = ["dark:h-6", "hover:h-7"];
    let dh = flag("DARK") | flag("HOVER");
    assert_eq!(winner(&tie, Field::Height, dh), Some(Value::Px(24.0)));
    // Two variants beat one, whatever their order.
    let two = ["hover:focus:h-6", "dark:h-7"];
    let all = dh | flag("FOCUS");
    assert_eq!(winner(&two, Field::Height, all), Some(Value::Px(24.0)));
}

/// `[&_svg:not([class*='size-'])]` (0,2,1) is more specific than `[&>svg]`
/// (0,1,1), so it wins on an svg child without a size class.
#[test]
fn svg_targets_compare_by_specificity() {
    let classes = ["[&_svg:not([class*='size-'])]:size-4", "[&>svg]:size-3"];
    assert_eq!(winner(&classes, Field::IconSize, 0), Some(Value::Px(16.0)));
}

/// `!important` still beats every normal declaration, whatever its
/// specificity; among important ones specificity decides again.
#[test]
fn important_beats_specificity() {
    let sm = flag("SIZE_SM");
    let important = ["h-8!", "dark:data-[size=sm]:aria-invalid:h-9"];
    let state = sm | flag("DARK") | flag("INVALID");
    assert_eq!(
        winner(&important, Field::Height, state),
        Some(Value::Px(32.0))
    );
    let both = ["data-[size=sm]:h-7!", "md:h-9!"];
    assert_eq!(winner(&both, Field::Height, sm), Some(Value::Px(28.0)));
    let svg = ["[&_svg:not([class*='size-'])]:size-4", "[&>svg]:size-3!"];
    assert_eq!(winner(&svg, Field::IconSize, 0), Some(Value::Px(12.0)));
    let high = variants_of("dark:data-[size=sm]:aria-invalid:aria-checked:h-9")
        .unwrap()
        .rank;
    assert!(high < 1 << R_IMPORTANT);
    assert!(variants_of("h-8!").unwrap().rank > high);
}

/// The rank packs important, pseudo-element, class steps and variant order
/// (module docs of `variant.rs`), each variant counted once.
#[test]
fn rank_packs_specificity_above_the_variant_bits() {
    let r = |raw: &str| variants_of(raw).unwrap().rank;
    // (pseudo-element, class steps): a class is two steps.
    let spec = |raw: &str| {
        let r = r(raw);
        (r >> R_PSEUDO_ELEMENT & 1, r >> R_CLASSES & 0x1f)
    };
    assert_eq!(r("h-8"), 0);
    assert_eq!(r("md:h-8"), 1 << rank("md"));
    assert_eq!(
        r("data-[size=sm]:h-8"),
        (2 << R_CLASSES) | (1 << rank("data-[size=sm]"))
    );
    assert_eq!(spec("dark:placeholder:text-muted-foreground"), (1, 2));
    assert_eq!(spec("in-data-[slot=button-group]:rounded-none"), (0, 0));
    assert_eq!(spec("hover:[&_svg:not([class*='size-'])]:size-4"), (0, 3));
    // A target adds no variant bit; `[&>svg]` and `after` add nothing.
    assert_eq!(r("[&_svg:not([class*='size-'])]:size-4"), 1 << R_CLASSES);
    assert_eq!(r("[&>svg]:size-3"), 0);
    assert_eq!(r("after:-inset-2"), 0);
    // A pseudo-element outranks every class, any class every variant bit.
    assert!(r("placeholder:h-8") > r("dark:hover:focus:h-8"));
    assert!(r("hover:h-8") > r("in-data-[slot=button-group]:md:sm:supports-backdrop-filter:h-8"));
}

/// `::placeholder` is an element of its own: the input's `text-*` rules
/// reach it only by inheritance, so any `placeholder:` rule beats them,
/// whatever their specificity. Among placeholder rules the class count
/// decides again.
#[test]
fn placeholder_rule_beats_the_inputs_own_text_rules() {
    let muted = set("text-muted-foreground")[0].1;
    let fg = set("text-foreground")[0].1;
    let p = flag("PLACEHOLDER");
    let dark = flag("DARK");
    let classes = ["dark:text-foreground", "placeholder:text-muted-foreground"];
    assert_eq!(winner(&classes, Field::Text, dark | p), Some(muted));
    assert_eq!(winner(&classes, Field::Text, dark), Some(fg));
    let hover = [
        "placeholder:text-muted-foreground",
        "hover:focus:text-foreground",
    ];
    let all = p | flag("HOVER") | flag("FOCUS");
    assert_eq!(winner(&hover, Field::Text, all), Some(muted));
    let both = [
        "dark:placeholder:text-foreground",
        "placeholder:text-muted-foreground",
    ];
    assert_eq!(winner(&both, Field::Text, dark | p), Some(fg));
}

/// At equal specificity Tailwind orders the svg targets by their own
/// variant bits, which come after every named variant: `[&>svg]` sorts
/// after `[&_svg:not(…)]`. `.x:hover:focus>svg` and
/// `.x:is(.dark *) svg:not([class*='size-'])` are both (0,3,1).
#[test]
fn svg_targets_keep_their_variant_order() {
    let classes = [
        "hover:focus:[&>svg]:size-3",
        "dark:[&_svg:not([class*='size-'])]:size-4",
    ];
    let state = flag("DARK") | flag("HOVER") | flag("FOCUS");
    assert_eq!(
        winner(&classes, Field::IconSize, state),
        Some(Value::Px(12.0))
    );
    // One more class and `[&_svg:not(…)]` wins again.
    let more = [
        "hover:focus:[&>svg]:size-3",
        "dark:focus:[&_svg:not([class*='size-'])]:size-4",
    ];
    assert_eq!(winner(&more, Field::IconSize, state), Some(Value::Px(16.0)));
    // One target: the named variants decide (`dark` after `hover`).
    let same = ["dark:[&>svg]:size-3", "hover:[&>svg]:size-4"];
    assert_eq!(winner(&same, Field::IconSize, state), Some(Value::Px(12.0)));
}

/// `rank_of` over every translated variant, in the order Tailwind 4.3.3
/// `Variants.compare` sorts them with the default theme (the breakpoints
/// sit between `supports-*` and `dark`, shadcn-svelte's custom `data-*`
/// variants after `dark`). No two of them tie. Unlike
/// `ranks_follow_tailwind_order`, which checks the table against the port
/// of `compare`, this pins the result itself: a change to the port that
/// moves a variant fails here.
#[test]
fn ranks_pin_tailwind_order() {
    const ORDER: [&str; 79] = [
        "not-aria-[haspopup]",
        "not-data-selected",
        "not-data-[variant=destructive]",
        "not-dark",
        "not-data-checked",
        "group-has-disabled/field",
        "group-has-data-[size=lg]/avatar-group",
        "group-has-data-[size=sm]/avatar-group",
        "group-has-data-[slot=alert-dialog-media]/alert-dialog-content",
        "group-has-[>svg]/alert",
        "group-data-[disabled=true]",
        "group-data-[size=default]/alert-dialog-content",
        "group-data-[size=default]/avatar",
        "group-data-[size=default]/switch",
        "group-data-[size=lg]/avatar",
        "group-data-[size=sm]/alert-dialog-content",
        "group-data-[size=sm]/avatar",
        "group-data-[size=sm]/card",
        "group-data-[size=sm]/switch",
        "group-data-[variant=default]/tabs-list",
        "group-data-[variant=line]/tabs-list",
        "group-data-horizontal/tabs",
        "group-data-vertical/tabs",
        "peer-disabled",
        "peer-data-disabled",
        "placeholder",
        "after",
        "hover",
        "focus",
        "focus-visible",
        "active",
        "disabled",
        "in-data-[slot=button-group]",
        "in-data-[slot=tooltip-content]",
        "has-data-[icon=inline-end]",
        "has-data-[icon=inline-start]",
        "has-data-[slot=alert-action]",
        "has-data-[slot=alert-dialog-media]",
        "has-data-[slot=card-footer]",
        "has-data-[slot=kbd]",
        "has-[>[data-slot=button-group]]",
        "has-[>[data-variant=outline]]",
        "has-[>svg]",
        "aria-checked",
        "aria-expanded",
        "aria-invalid",
        "data-highlighted",
        "data-invalid",
        "data-placeholder",
        "data-selected",
        "data-[active=true]",
        "data-[disabled=true]",
        "data-[orientation=horizontal]",
        "data-[orientation=vertical]",
        "data-[side=bottom]",
        "data-[side=left]",
        "data-[side=right]",
        "data-[side=top]",
        "data-[size=default]",
        "data-[size=lg]",
        "data-[size=sm]",
        "data-[state=checked]",
        "data-[state=delayed-open]",
        "data-[state=unchecked]",
        "data-[variant=line]",
        "supports-backdrop-filter",
        "sm",
        "md",
        "dark",
        "data-open",
        "data-closed",
        "data-checked",
        "data-unchecked",
        "data-disabled",
        "data-active",
        "data-horizontal",
        "data-vertical",
        "[&_svg:not([class*='size-'])]",
        "[&>svg]",
    ];
    let mut translated: Vec<&str> = TRANSLATED.iter().map(|(n, _, _)| *n).collect();
    let mut listed = ORDER.to_vec();
    translated.sort_unstable();
    listed.sort_unstable();
    assert_eq!(
        translated, listed,
        "ORDER must list every translated variant"
    );
    for w in ORDER.windows(2) {
        assert!(
            rank(w[0]) < rank(w[1]),
            "`{}` (rank {}) must rank below `{}` (rank {})",
            w[0],
            rank(w[0]),
            w[1],
            rank(w[1])
        );
    }
}

/// The classes of `marker` in the fixture `style-<style>.css`.
fn marker_classes(style: &str, marker: &str) -> Vec<String> {
    parse_style_css(&read(&format!("tests/fixtures/style-{style}.css")))
        .into_iter()
        .find(|(m, _)| m == marker)
        .unwrap_or_else(|| panic!("{style} has no {marker}"))
        .1
}

/// A focused, checked checkbox shows the focus border, not the checked
/// border. Tailwind emits `focus-visible:border-ring` before
/// `data-checked:border-primary`, but shadcn-svelte's `data-checked` is
/// `&:where(..)`: (0,1,0) against (0,2,0), so the focus border wins in the
/// browser. The luma switch has the same pair; the vega switch has no
/// checked border.
#[test]
fn focus_visible_border_beats_checked_border() {
    let ring = Some(Value::Paint {
        role: "Ring",
        alpha: 1.0,
    });
    let focus = flag("FOCUS_VISIBLE");
    let both = focus | flag("CHECKED");
    for (style, marker) in [("vega", "cn-checkbox"), ("luma", "cn-switch")] {
        let owned = marker_classes(style, marker);
        let classes: Vec<&str> = owned.iter().map(String::as_str).collect();
        assert_eq!(
            winner(&classes, Field::BorderColor, both),
            ring,
            "{style} {marker} focused and checked"
        );
        assert_eq!(
            winner(&classes, Field::BorderColor, focus),
            ring,
            "{style} {marker} focused"
        );
    }
    let owned = marker_classes("vega", "cn-switch");
    let classes: Vec<&str> = owned.iter().map(String::as_str).collect();
    assert_eq!(winner(&classes, Field::BorderColor, both), ring);
}

/// shadcn-svelte's `data-checked` is `&:where(..)`: no specificity, so
/// `focus-visible:border-ring` (0,2,0) beats `data-checked:border-primary`
/// (0,1,0) on a focused checked checkbox, though Tailwind emits the custom
/// variant later.
#[test]
fn custom_data_variants_add_no_specificity() {
    let both = flag("CHECKED") | flag("FOCUS_VISIBLE");
    let classes = ["focus-visible:border-ring", "data-checked:border-primary"];
    let ring = Value::Paint {
        role: "Ring",
        alpha: 1.0,
    };
    assert_eq!(winner(&classes, Field::BorderColor, both), Some(ring));
    // Alone, the checked rule still applies.
    assert_eq!(
        winner(&classes, Field::BorderColor, flag("CHECKED")),
        Some(Value::Paint {
            role: "Primary",
            alpha: 1.0
        })
    );
    // At equal specificity the custom variant sorts after `dark`.
    let dark = flag("DARK") | flag("CHECKED");
    let classes = ["data-checked:bg-primary", "dark:bg-input"];
    assert_eq!(
        winner(&classes, Field::Bg, dark),
        Some(Value::Paint {
            role: "Input",
            alpha: 1.0
        })
    );
}

/// A value a parent's rule gives a slot without a crate part is a look
/// with nowhere to go: unsupported, unless the crate does not ship the
/// slot's component (input-group). A slot with a part takes the rule.
#[test]
fn route_reports_a_look_on_a_slot_without_a_part() {
    use stylegen::resolve::{route, Part};
    let part = |name: &str, classes: &[&str]| Part {
        name: name.to_owned(),
        marker: translate(
            &classes.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>(),
            &Vars::new(),
        ),
        corrections: Vec::new(),
    };
    let mut ps = vec![
        part(
            "cn-tooltip-content",
            &[
                "*:data-[slot=nosuch]:gap-2",
                "*:data-[slot=input-group]:gap-2",
                "*:data-[slot=kbd]:gap-2",
            ],
        ),
        part("cn-kbd", &[]),
    ];
    route(&mut ps);
    let tooltip = &ps[0].marker;
    let unsupported: Vec<&str> = tooltip
        .unsupported
        .iter()
        .map(|(c, _)| c.as_str())
        .collect();
    assert_eq!(unsupported, ["*:data-[slot=nosuch]:gap-2"]);
    assert!(tooltip
        .ignored
        .iter()
        .any(|(c, _)| c == "*:data-[slot=input-group]:gap-2"));
    assert_eq!(ps[1].marker.routed.len(), 1, "{:?}", ps[1].marker.routed);
}
