//! Tailwind variant → recipe condition and rank.
//!
//! A variant either sets condition flags ([`V::Cond`]), always matches in
//! egui ([`V::Always`]), retargets the class to a child element
//! ([`V::Pseudo`]), is ignored for a stated reason, or is unsupported. Every
//! translated or ignored variant is listed by its exact text; anything else
//! is unsupported, never guessed.
//!
//! The rank orders rules as the browser's cascade does: `!important` first,
//! then selector specificity, then source order.
//!
//! Source order follows tailwindcss 4.3.3. `compile.ts:64-67` gives each
//! rule a bitmask with one bit per variant it uses, and sorts rules by that
//! mask. The bit of a variant is its index in the list of all variants in
//! use, sorted by `Variants.compare` (`design-system.ts:190-215`,
//! `variants.ts:212-271`); variants that compare unequal get distinct bits,
//! so `data-checked` and `data-disabled` differ. [`tw_cmp`] ports that
//! comparison, and a variant's bit is its index in [`TRANSLATED`] sorted by
//! it. Only the relative order of the bits matters, so indices over this
//! subset compare exactly like Tailwind's.
//!
//! Specificity is the sum of what each variant adds ([`Spec`], listed in
//! [`TRANSLATED`]). Every utility class adds the same (0,1,0) for itself, so
//! that part is left out: a class without variants has specificity zero.
//!
//! A rule's rank packs the three into one `u128`, compared as an integer:
//! [`R_IMPORTANT`] (bit 127) for an `!important` class, the class count of
//! the specificity in bits [`R_CLASSES`]`..127`, its element count in bits
//! [`R_ELEMENTS`]`..R_CLASSES`, and the OR of `1u128 << bit` over the
//! variants below [`R_ELEMENTS`], so variants have at most [`R_ELEMENTS`]
//! distinct bits.

use std::cmp::Ordering;
use std::sync::OnceLock;

use crate::class::Class;
use crate::model::flag;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pseudo {
    After,
    Svg,
}

#[derive(Debug, PartialEq)]
pub enum V {
    Cond { all: u32, none: u32, rank: u32 },
    Always { rank: u32 },
    Pseudo(Pseudo, u32),
    Ignore(&'static str),
    Unsupported,
}

/// What a translated variant means for the recipe.
#[derive(Clone, Copy, Debug)]
pub enum Meaning {
    /// Matches when the flag is set.
    Is(&'static str),
    /// Matches when the flag is not set.
    IsNot(&'static str),
    /// Always true for the parts the crate draws.
    Always,
    /// The class styles a child element, not the part.
    Target(Pseudo),
}

use Meaning::*;

/// The specificity a variant adds to its class's selector, as Tailwind
/// writes it: `dark` is `&:is(.dark *)`, `data-[size=sm]` is
/// `&[data-size=sm]`, `group-hover` is `&:is(:where(.group):hover *)`, each
/// one class, attribute or pseudo-class. The ID count is always zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Spec {
    /// Classes, attributes and pseudo-classes.
    pub classes: u32,
    /// Type selectors and pseudo-elements.
    pub elements: u32,
}

impl std::ops::Add for Spec {
    type Output = Spec;
    fn add(self, o: Spec) -> Spec {
        Spec {
            classes: self.classes + o.classes,
            elements: self.elements + o.elements,
        }
    }
}

/// One class, attribute or pseudo-class: (0,1,0).
pub const CLASS: Spec = Spec {
    classes: 1,
    elements: 0,
};
/// One pseudo-element: (0,0,1).
pub const ELEMENT: Spec = Spec {
    classes: 0,
    elements: 1,
};
/// No specificity: media and supports queries, `:where(…)`.
pub const ZERO: Spec = Spec {
    classes: 0,
    elements: 0,
};

/// Every variant stylegen translates, by exact text, with the specificity
/// it adds.
pub const TRANSLATED: &[(&str, Meaning, Spec)] = &[
    ("dark", Is("DARK"), CLASS),
    ("not-dark", IsNot("DARK"), CLASS),
    ("hover", Is("HOVER"), CLASS),
    ("focus", Is("FOCUS"), CLASS),
    ("focus-visible", Is("FOCUS_VISIBLE"), CLASS),
    ("data-highlighted", Is("FOCUS"), CLASS),
    ("active", Is("ACTIVE"), CLASS),
    ("disabled", Is("DISABLED"), CLASS),
    ("data-disabled", Is("DISABLED"), CLASS),
    ("data-[disabled=true]", Is("DISABLED"), CLASS),
    ("group-data-[disabled=true]", Is("DISABLED"), CLASS),
    ("group-has-disabled/field", Is("DISABLED"), CLASS),
    ("peer-disabled", Is("DISABLED"), CLASS),
    ("peer-data-disabled", Is("DISABLED"), CLASS),
    ("aria-invalid", Is("INVALID"), CLASS),
    ("data-invalid", Is("INVALID"), CLASS),
    ("data-checked", Is("CHECKED"), CLASS),
    ("data-[state=checked]", Is("CHECKED"), CLASS),
    ("aria-checked", Is("CHECKED"), CLASS),
    ("data-unchecked", Is("UNCHECKED"), CLASS),
    ("data-[state=unchecked]", Is("UNCHECKED"), CLASS),
    ("not-data-checked", IsNot("CHECKED"), CLASS),
    ("aria-expanded", Is("EXPANDED"), CLASS),
    ("not-aria-[haspopup]", IsNot("HAS_POPUP"), CLASS),
    ("data-active", Is("SELECTED"), CLASS),
    ("data-[active=true]", Is("SELECTED"), CLASS),
    ("data-selected", Is("SELECTED"), CLASS),
    ("not-data-selected", IsNot("SELECTED"), CLASS),
    ("data-placeholder", Is("PLACEHOLDER"), CLASS),
    // `&::placeholder`: a pseudo-element.
    ("placeholder", Is("PLACEHOLDER"), ELEMENT),
    ("has-data-[icon=inline-start]", Is("ICON_START"), CLASS),
    ("has-data-[icon=inline-end]", Is("ICON_END"), CLASS),
    // `:where(*[data-slot=button-group]) &`.
    ("in-data-[slot=button-group]", Is("IN_BUTTON_GROUP"), ZERO),
    ("data-[size=sm]", Is("SIZE_SM"), CLASS),
    ("group-data-[size=sm]/switch", Is("SIZE_SM"), CLASS),
    ("group-data-[size=sm]/card", Is("SIZE_SM"), CLASS),
    ("data-[size=default]", Is("SIZE_DEFAULT"), CLASS),
    (
        "group-data-[size=default]/switch",
        Is("SIZE_DEFAULT"),
        CLASS,
    ),
    ("data-vertical", Is("VERTICAL"), CLASS),
    ("data-[orientation=vertical]", Is("VERTICAL"), CLASS),
    ("group-data-vertical/tabs", Is("VERTICAL"), CLASS),
    ("data-horizontal", Is("HORIZONTAL"), CLASS),
    ("data-[orientation=horizontal]", Is("HORIZONTAL"), CLASS),
    ("group-data-horizontal/tabs", Is("HORIZONTAL"), CLASS),
    ("data-[variant=line]", Is("VARIANT_LINE"), CLASS),
    (
        "group-data-[variant=line]/tabs-list",
        Is("VARIANT_LINE"),
        CLASS,
    ),
    (
        "group-data-[variant=default]/tabs-list",
        Is("VARIANT_DEFAULT"),
        CLASS,
    ),
    ("has-data-[slot=card-footer]", Is("HAS_FOOTER"), CLASS),
    // The crate has no destructive select item.
    ("not-data-[variant=destructive]", Always, CLASS),
    // An egui window counts as wider than the `sm` and `md` breakpoints.
    // Media and supports queries add no specificity.
    ("sm", Always, ZERO),
    ("md", Always, ZERO),
    // egui paints its own backdrop blur.
    ("supports-backdrop-filter", Always, ZERO),
    // A target's rules compete only with rules of the same target, so its
    // specificity counts over the least specific selector of that target:
    // `&::after` (0,0,1), `& > svg` (0,0,1), `& svg:not([class*='size-'])`
    // (0,1,1).
    ("after", Target(Pseudo::After), ZERO),
    ("[&_svg:not([class*='size-'])]", Target(Pseudo::Svg), CLASS),
    ("[&>svg]", Target(Pseudo::Svg), ZERO),
];

const ANIM: &str = "open/close animation state; egui shows and hides at once";
const SIDE: &str = "the side egui placed the popup on: placement and slide-in direction, both done by egui (also the tooltip arrow offset)";
const LEADING_IMAGE: &str = "card with a leading image is not offered";
const KBD: &str = "keyboard-shortcut chip in a tooltip; the crate does not ship Kbd";

/// Every variant stylegen ignores, by exact text, with the reason.
pub const IGNORED: &[(&str, &str)] = &[
    ("data-open", ANIM),
    ("data-closed", ANIM),
    ("data-[state=delayed-open]", ANIM),
    ("data-entering", ANIM),
    ("data-exiting", ANIM),
    ("data-ending-style", ANIM),
    ("data-starting-style", ANIM),
    ("data-[side=top]", SIDE),
    ("data-[side=bottom]", SIDE),
    ("data-[side=left]", SIDE),
    ("data-[side=right]", SIDE),
    // Shipped uses: card images, description links, select item spans, the
    // select value wrapper.
    ("*", "`*:` styles direct child elements the crate does not draw as elements"),
    // Shipped uses: the tooltip Kbd chip, and the highlighted select item's
    // icon and text, which the crate paints in the item's own text colour.
    ("**", "`**:` styles descendant elements: a Kbd chip (not shipped) or select item content painted in the item's text colour"),
    ("[a]", "link-styled badge/description anchors are not ported"),
    ("file", "file-input button; egui has no file input"),
    ("[.border-b]", "card header with an explicit border class is not offered"),
    ("[.border-t]", "card footer with an explicit border class is not offered"),
    ("has-[>img:first-child]", LEADING_IMAGE),
    ("[img:first-child]", LEADING_IMAGE),
    ("[img:last-child]", "card with a trailing image is not offered"),
    ("group-has-[:focus-visible]/field-label", "inside a focused FieldLabel; the crate does not ship Field"),
    ("[span]", "select item text wrapper; the crate draws the item text itself"),
    ("data-[slot=select-value]", "select value wrapper; the crate draws the value itself"),
    ("data-[slot=kbd]", KBD),
    ("has-data-[slot=kbd]", KBD),
    ("rtl", "right-to-left layout; the crate lays out left to right"),
    ("[&_svg]", crate::utility::SVG_CHILD),
    ("has-data-[slot=card-action]", "card header grid template; the crate lays out the header itself"),
    ("has-data-[slot=card-description]", "card header grid template; the crate lays out the header itself"),
];

/// Registration order of the variant roots this tool meets: the line of the
/// registration call in tailwindcss 4.3.3 `packages/tailwindcss/src/variants.ts`.
/// `createVariants` registers in source order, so the line orders them.
/// shadcn's `@custom-variant dark` replaces `dark` in place (`Variants.set`
/// keeps an existing variant's order).
const ROOTS: &[(&str, u32)] = &[
    ("*", 375),
    ("**", 376),
    ("not", 494),
    ("group", 569),
    ("peer", 621),
    ("file", 684),
    ("placeholder", 685),
    ("before", 699),
    ("after", 713),
    ("first", 729),
    ("last", 730),
    ("focus-within", 763),
    ("hover", 764),
    ("focus", 767),
    ("focus-visible", 768),
    ("active", 769),
    ("enabled", 770),
    ("disabled", 771),
    ("in", 775),
    ("has", 810),
    ("aria", 845),
    ("data", 873),
    ("supports", 919),
    // One group (variants.ts:1046-1075), ordered by breakpoint width.
    ("sm", BREAKPOINT_GROUP),
    ("md", BREAKPOINT_GROUP),
    ("lg", BREAKPOINT_GROUP),
    ("xl", BREAKPOINT_GROUP),
    ("2xl", BREAKPOINT_GROUP),
    ("dark", 1199),
];

const BREAKPOINT_GROUP: u32 = 1046;

/// Default `--breakpoint-*` widths in rem, for ordering inside the group.
const BREAKPOINTS: &[(&str, u32)] = &[("sm", 40), ("md", 48), ("lg", 64), ("xl", 80), ("2xl", 96)];

const COMPOUND: &[&str] = &["not", "group", "peer", "in", "has"];
const FUNCTIONAL: &[&str] = &["aria", "data", "supports"];

/// Tailwind's parsed form of a variant (`candidate.ts` `parseVariant`), as
/// far as ordering needs it.
#[derive(Debug)]
enum Tw<'a> {
    Arbitrary(String),
    Static(&'a str),
    Functional {
        root: &'a str,
        value: &'a str,
        arbitrary: bool,
    },
    Compound {
        root: &'a str,
        inner: Box<Tw<'a>>,
        modifier: Option<&'a str>,
    },
}

/// Split at the top-level `/` (`segment(variant, '/')`).
fn split_modifier(v: &str) -> (&str, Option<&str>) {
    let mut depth = 0i32;
    for (i, ch) in v.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            '/' if depth == 0 => return (&v[..i], Some(&v[i + 1..])),
            _ => {}
        }
    }
    (v, None)
}

fn parse(v: &str) -> Option<Tw<'_>> {
    if let Some(inner) = v.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        // decodeArbitraryValue turns `_` into a space; a selector without
        // `&` is wrapped in `&:is(…)` unless it is relative.
        let sel = inner.replace('_', " ");
        let relative = sel.starts_with(['>', '+', '~']);
        let sel = if !relative && !sel.starts_with('@') && !sel.contains('&') {
            format!("&:is({sel})")
        } else {
            sel
        };
        return Some(Tw::Arbitrary(sel));
    }
    let (body, modifier) = split_modifier(v);
    let is_static = ROOTS.iter().any(|(r, _)| *r == body)
        && !COMPOUND.contains(&body)
        && !FUNCTIONAL.contains(&body);
    if is_static {
        return modifier.is_none().then_some(Tw::Static(body));
    }
    let (root, value) = body.split_once('-')?;
    if COMPOUND.contains(&root) {
        return Some(Tw::Compound {
            root,
            inner: Box::new(parse(value)?),
            modifier,
        });
    }
    if FUNCTIONAL.contains(&root) && modifier.is_none() {
        return Some(
            match value.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                Some(a) => Tw::Functional {
                    root,
                    value: a,
                    arbitrary: true,
                },
                None => Tw::Functional {
                    root,
                    value,
                    arbitrary: false,
                },
            },
        );
    }
    None
}

fn root_order(root: &str) -> u32 {
    ROOTS
        .iter()
        .find(|(r, _)| *r == root)
        .map(|(_, o)| *o)
        .expect("root in ROOTS")
}

fn root_of<'a>(t: &Tw<'a>) -> &'a str {
    match t {
        Tw::Static(r) => r,
        Tw::Functional { root, .. } | Tw::Compound { root, .. } => root,
        Tw::Arbitrary(_) => {
            unreachable!("arbitrary variants are ordered before roots are compared")
        }
    }
}

/// `Variants.compare` (variants.ts:212-271) for the variants [`parse`] knows.
fn compare(a: &Tw, z: &Tw) -> Ordering {
    match (a, z) {
        (Tw::Arbitrary(x), Tw::Arbitrary(y)) => return x.cmp(y),
        (Tw::Arbitrary(_), _) => return Ordering::Greater,
        (_, Tw::Arbitrary(_)) => return Ordering::Less,
        _ => {}
    }
    let (ra, rz) = (root_of(a), root_of(z));
    let by_root = root_order(ra).cmp(&root_order(rz));
    if by_root != Ordering::Equal {
        return by_root;
    }
    match (a, z) {
        (
            Tw::Compound {
                inner: ia,
                modifier: ma,
                ..
            },
            Tw::Compound {
                inner: iz,
                modifier: mz,
                ..
            },
        ) => {
            // `None < Some`, and two modifiers compare as strings.
            compare(ia, iz).then_with(|| ma.cmp(mz))
        }
        _ if root_order(ra) == BREAKPOINT_GROUP => {
            let width = |r: &str| BREAKPOINTS.iter().find(|(b, _)| *b == r).map(|(_, w)| *w);
            width(ra).cmp(&width(rz))
        }
        _ if ra != rz => ra.cmp(rz),
        (
            Tw::Functional {
                value: va,
                arbitrary: xa,
                ..
            },
            Tw::Functional {
                value: vz,
                arbitrary: xz,
                ..
            },
        ) => {
            // Named values before arbitrary ones, then by value.
            xa.cmp(xz).then_with(|| va.cmp(vz))
        }
        _ => Ordering::Equal,
    }
}

/// Tailwind's order of two variants; `None` if either is not parseable here.
pub fn tw_cmp(a: &str, z: &str) -> Option<Ordering> {
    Some(compare(&parse(a)?, &parse(z)?))
}

/// The rank bit of an `!important` class (`size-3!`). In CSS an important
/// declaration beats every normal one whatever its selector or order, so
/// this bit sits above everything else.
pub const R_IMPORTANT: u32 = 127;

/// Lowest bit of the specificity's class count (4 bits, below
/// [`R_IMPORTANT`]).
pub const R_CLASSES: u32 = 123;

/// Lowest bit of the specificity's element count (2 bits, below
/// [`R_CLASSES`]). The variant bits sit below it.
pub const R_ELEMENTS: u32 = 121;

/// Pack a rule's rank: see the module docs.
pub fn encode(mask: u128, spec: Spec, important: bool) -> u128 {
    // Fail loudly rather than carry into the next field.
    assert!(
        mask >> R_ELEMENTS == 0,
        "variant bits overflow the rank mask"
    );
    assert!(
        spec.classes < 1 << (R_IMPORTANT - R_CLASSES),
        "specificity class count {} overflows its rank bits",
        spec.classes
    );
    assert!(
        spec.elements < 1 << (R_CLASSES - R_ELEMENTS),
        "specificity element count {} overflows its rank bits",
        spec.elements
    );
    mask | u128::from(spec.classes) << R_CLASSES
        | u128::from(spec.elements) << R_ELEMENTS
        | u128::from(important) << R_IMPORTANT
}

/// [`TRANSLATED`] in Tailwind variant order, each with its bit index:
/// variants that compare equal share one (`getVariantOrder`).
fn ranked() -> &'static [(&'static str, u32)] {
    static RANKED: OnceLock<Vec<(&'static str, u32)>> = OnceLock::new();
    RANKED.get_or_init(|| {
        let cmp = |a: &str, z: &str| {
            tw_cmp(a, z).unwrap_or_else(|| panic!("unparseable variant `{a}` or `{z}`"))
        };
        let mut names: Vec<&'static str> = TRANSLATED.iter().map(|(n, _, _)| *n).collect();
        names.sort_by(|a, z| cmp(a, z));
        let mut out: Vec<(&'static str, u32)> = Vec::with_capacity(names.len());
        for n in names {
            let rank = match out.last() {
                Some((prev, r)) if cmp(prev, n) == Ordering::Equal => *r,
                Some((_, r)) => r + 1,
                None => 0,
            };
            out.push((n, rank));
        }
        // Fail loudly rather than move a bit into the specificity.
        assert!(
            out.last().is_none_or(|(_, r)| *r < R_ELEMENTS),
            "more than {R_ELEMENTS} variant ranks"
        );
        out
    })
}

/// A translated variant's rank: its bit index in Tailwind's variant order.
pub fn rank_of(v: &str) -> Option<u32> {
    ranked().iter().find(|(n, _)| *n == v).map(|(_, r)| *r)
}

pub fn variant(v: &str) -> V {
    if let Some((_, m, _)) = TRANSLATED.iter().find(|(n, _, _)| *n == v) {
        let rank = rank_of(v).expect("translated variant has a rank");
        return match *m {
            Is(f) => V::Cond {
                all: flag(f),
                none: 0,
                rank,
            },
            IsNot(f) => V::Cond {
                all: 0,
                none: flag(f),
                rank,
            },
            Always => V::Always { rank },
            Target(p) => V::Pseudo(p, rank),
        };
    }
    match IGNORED.iter().find(|(n, _)| *n == v) {
        Some((_, why)) => V::Ignore(why),
        None => V::Unsupported,
    }
}

/// The variants of one class, combined.
#[derive(Debug, PartialEq)]
pub struct Variants {
    pub all: u32,
    pub none: u32,
    /// The class's [`rank`].
    pub rank: u128,
    /// A pseudo target selects the field. It adds its specificity to the
    /// rank, but no variant bit.
    pub pseudo: Option<Pseudo>,
}

#[derive(Debug, PartialEq)]
pub enum Skip {
    Ignore(&'static str),
    Unsupported(String),
}

/// The rank of a class with these translated variants (each counted once)
/// and `!important` flag; see the module docs. Variants that are not
/// translated add nothing.
pub fn rank<'a>(variants: impl IntoIterator<Item = &'a str>, important: bool) -> u128 {
    let mut seen: Vec<&str> = Vec::new();
    let (mut mask, mut spec) = (0u128, ZERO);
    for v in variants {
        if seen.contains(&v) {
            continue;
        }
        seen.push(v);
        let Some((_, m, s)) = TRANSLATED.iter().find(|(n, _, _)| *n == v) else {
            continue;
        };
        spec = spec + *s;
        if !matches!(m, Target(_)) {
            mask |= 1 << rank_of(v).expect("translated variant has a rank");
        }
    }
    encode(mask, spec, important)
}

/// Combine a class's variants. The first variant that is ignored or
/// unsupported decides for the whole class.
pub fn class_variants(c: &Class) -> Result<Variants, Skip> {
    let mut out = Variants {
        all: 0,
        none: 0,
        rank: rank(c.variants.iter().map(String::as_str), c.important),
        pseudo: None,
    };
    for v in &c.variants {
        match variant(v) {
            V::Cond { all, none, .. } => {
                out.all |= all;
                out.none |= none;
            }
            V::Always { .. } => {}
            V::Pseudo(p, _) => out.pseudo = Some(p),
            V::Ignore(why) => return Err(Skip::Ignore(why)),
            V::Unsupported => return Err(Skip::Unsupported(format!("variant `{v}`"))),
        }
    }
    Ok(out)
}
