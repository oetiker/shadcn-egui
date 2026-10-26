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
//! A pseudo-element (`placeholder:`, `&::placeholder`) is an element of its
//! own. The part's other rules reach it only by inheritance, which loses to
//! any rule on the pseudo-element itself, so its count ranks above the
//! class count, not below it as in a selector's specificity. The crate folds
//! the placeholder into a flag on the part's own field; this keeps the
//! browser's result. One gap remains: an `!important` rule on the part still
//! beats a normal `placeholder:` rule, which the browser would not do.
//!
//! The two svg targets compete on one field. At equal specificity Tailwind
//! orders them by their own variant bits, which sit above every named
//! variant, and `[&>svg]` sorts after `[&_svg:not([class*='size-'])]`: the
//! `[&>svg]` rule wins whatever the other variants. The rank counts classes
//! in half steps to say so without a bit for the targets: a class is two
//! steps, and `[&_svg:not([class*='size-'])]`, one class more specific than
//! `[&>svg]`, adds one step less ([`Spec::earlier`]). So a `[&>svg]` class
//! without other variants keeps rank 0.
//!
//! A rule's rank packs these into one `u128`, compared as an integer:
//! [`R_IMPORTANT`] (bit 127) for an `!important` class,
//! [`R_PSEUDO_ELEMENT`] (bit 126) for a pseudo-element, the class steps
//! (`2 * classes + elements - earlier`: an element selector such as
//! `has-[>svg]` adds a half step, [`Spec::elements`]) in bits
//! [`R_CLASSES`]`..126`, and the OR of
//! `1u128 << bit` over the variants below [`R_CLASSES`], so variants have at
//! most [`R_CLASSES`] distinct bits. Targets add no variant bit.

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
    Cond { all: u128, none: u128, rank: u32 },
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
    /// Pseudo-elements.
    pub pseudo_elements: u32,
    /// Element (type) selectors (`has-[>svg]` is `&:has(> svg)`, (0,0,1)).
    /// At most one: it ranks half a class step, above the same class count
    /// without it and below one class more.
    pub elements: u32,
    /// Classes of a target that Tailwind sorts before its rival target of
    /// equal specificity: each loses that tie (half a class step in the
    /// rank).
    pub earlier: u32,
}

impl std::ops::Add for Spec {
    type Output = Spec;
    fn add(self, o: Spec) -> Spec {
        Spec {
            classes: self.classes + o.classes,
            pseudo_elements: self.pseudo_elements + o.pseudo_elements,
            elements: self.elements + o.elements,
            earlier: self.earlier + o.earlier,
        }
    }
}

/// One class, attribute or pseudo-class: (0,1,0).
pub const CLASS: Spec = Spec {
    classes: 1,
    pseudo_elements: 0,
    elements: 0,
    earlier: 0,
};
/// One pseudo-element: (0,0,1).
pub const PSEUDO_ELEMENT: Spec = Spec {
    classes: 0,
    pseudo_elements: 1,
    elements: 0,
    earlier: 0,
};
/// No specificity: media and supports queries, `:where(…)`.
pub const ZERO: Spec = Spec {
    classes: 0,
    pseudo_elements: 0,
    elements: 0,
    earlier: 0,
};
/// `[&_svg:not([class*='size-'])]` over `[&>svg]`: one class, sorted
/// before it.
pub const SVG_NOT: Spec = Spec {
    classes: 1,
    pseudo_elements: 0,
    elements: 0,
    earlier: 1,
};
/// One element selector: (0,0,1).
pub const ELEMENT: Spec = Spec {
    classes: 0,
    pseudo_elements: 0,
    elements: 1,
    earlier: 0,
};

/// shadcn-svelte's own `@custom-variant`s (`shadcn-svelte/tailwind.css`,
/// which the docs site imports): `data-checked` is
/// `&:where([data-state="checked"]), &:where([data-checked]..)`, so it and
/// every compound around it (`not-data-checked`, `peer-data-disabled`,
/// `group-data-horizontal/tabs`) add no specificity. A focused checked
/// checkbox therefore takes `focus-visible:border-ring` over
/// `data-checked:border-primary`, as in the browser.
const WHERE: Spec = ZERO;

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
    ("data-disabled", Is("DISABLED"), WHERE),
    ("data-[disabled=true]", Is("DISABLED"), CLASS),
    ("group-data-[disabled=true]", Is("DISABLED"), CLASS),
    ("group-has-disabled/field", Is("DISABLED"), CLASS),
    ("peer-disabled", Is("DISABLED"), CLASS),
    ("peer-data-disabled", Is("DISABLED"), WHERE),
    ("aria-invalid", Is("INVALID"), CLASS),
    ("data-invalid", Is("INVALID"), CLASS),
    ("data-checked", Is("CHECKED"), WHERE),
    ("data-[state=checked]", Is("CHECKED"), CLASS),
    ("aria-checked", Is("CHECKED"), CLASS),
    ("data-unchecked", Is("UNCHECKED"), WHERE),
    ("data-[state=unchecked]", Is("UNCHECKED"), CLASS),
    ("not-data-checked", IsNot("CHECKED"), WHERE),
    ("aria-expanded", Is("EXPANDED"), CLASS),
    ("not-aria-[haspopup]", IsNot("HAS_POPUP"), CLASS),
    ("data-active", Is("SELECTED"), WHERE),
    ("data-[active=true]", Is("SELECTED"), CLASS),
    ("data-selected", Is("SELECTED"), CLASS),
    ("not-data-selected", IsNot("SELECTED"), CLASS),
    ("data-placeholder", Is("PLACEHOLDER"), CLASS),
    // `&::placeholder`: a pseudo-element.
    ("placeholder", Is("PLACEHOLDER"), PSEUDO_ELEMENT),
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
    ("data-vertical", Is("VERTICAL"), WHERE),
    ("data-[orientation=vertical]", Is("VERTICAL"), CLASS),
    ("group-data-vertical/tabs", Is("VERTICAL"), WHERE),
    ("data-horizontal", Is("HORIZONTAL"), WHERE),
    ("data-[orientation=horizontal]", Is("HORIZONTAL"), CLASS),
    ("group-data-horizontal/tabs", Is("HORIZONTAL"), WHERE),
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
    // `data-open` is a real open state (D-91); bits sets `data-closed` on
    // the same parts when closed. Neither is set on tooltip content.
    ("data-open", Is("EXPANDED"), WHERE),
    ("data-closed", Is("CLOSED"), WHERE),
    ("data-[state=delayed-open]", Is("DELAYED_OPEN"), CLASS),
    // The side a panel took (D-93): slide-in direction, arrow offset.
    ("data-[side=top]", Is("SIDE_TOP"), CLASS),
    ("data-[side=right]", Is("SIDE_RIGHT"), CLASS),
    ("data-[side=bottom]", Is("SIDE_BOTTOM"), CLASS),
    ("data-[side=left]", Is("SIDE_LEFT"), CLASS),
    ("data-[size=lg]", Is("SIZE_LG"), CLASS),
    ("group-data-[size=sm]/avatar", Is("SIZE_SM"), CLASS),
    (
        "group-data-[size=default]/avatar",
        Is("SIZE_DEFAULT"),
        CLASS,
    ),
    ("group-data-[size=lg]/avatar", Is("SIZE_LG"), CLASS),
    (
        "group-data-[size=sm]/alert-dialog-content",
        Is("SIZE_SM"),
        CLASS,
    ),
    (
        "group-data-[size=default]/alert-dialog-content",
        Is("SIZE_DEFAULT"),
        CLASS,
    ),
    (
        "group-has-data-[size=lg]/avatar-group",
        Is("HAS_SIZE_LG"),
        CLASS,
    ),
    (
        "group-has-data-[size=sm]/avatar-group",
        Is("HAS_SIZE_SM"),
        CLASS,
    ),
    // `&:has(> svg)`: an element, (0,0,1).
    ("has-[>svg]", Is("HAS_ICON"), ELEMENT),
    ("group-has-[>svg]/alert", Is("HAS_ICON"), ELEMENT),
    ("has-data-[slot=alert-action]", Is("HAS_ACTION"), CLASS),
    ("has-data-[slot=alert-dialog-media]", Is("HAS_MEDIA"), CLASS),
    (
        "group-has-data-[slot=alert-dialog-media]/alert-dialog-content",
        Is("HAS_MEDIA"),
        CLASS,
    ),
    ("has-data-[slot=kbd]", Is("HAS_KBD"), CLASS),
    (
        "has-[>[data-slot=button-group]]",
        Is("HAS_BUTTON_GROUP"),
        CLASS,
    ),
    ("has-[>[data-variant=outline]]", Is("HAS_OUTLINE"), CLASS),
    // `:where(*[data-slot=tooltip-content]) &`.
    (
        "in-data-[slot=tooltip-content]",
        Is("IN_TOOLTIP_CONTENT"),
        ZERO,
    ),
    // The crate has no destructive select item.
    ("not-data-[variant=destructive]", Always, CLASS),
    // Media queries read the window width (D-69 (a), `layout::breakpoints`).
    // Media and supports queries add no specificity.
    ("sm", Is("SM"), ZERO),
    ("md", Is("MD"), ZERO),
    // egui paints its own backdrop blur.
    ("supports-backdrop-filter", Always, ZERO),
    // A target's rules compete only with rules of the same target, so its
    // specificity counts over the least specific selector of that target:
    // `&::after` (0,0,1), `& > svg` (0,0,1), `& svg:not([class*='size-'])`
    // (0,1,1). Their order at equal specificity is in `SVG_NOT`.
    ("after", Target(Pseudo::After), ZERO),
    (
        "[&_svg:not([class*='size-'])]",
        Target(Pseudo::Svg),
        SVG_NOT,
    ),
    ("[&>svg]", Target(Pseudo::Svg), ZERO),
];

const LEADING_IMAGE: &str = "card with a leading image is not offered";
/// D-100 (a): rules for links and paragraphs inside caller content.
const CALLER_CONTENT: &str =
    "styles links or paragraphs in caller content, which egui lays out and draws itself (D-100)";
pub const INPUT_GROUP: &str = "inside or around an InputGroup; the crate does not ship InputGroup";

/// Every variant stylegen ignores, by exact text, with the reason.
pub const IGNORED: &[(&str, &str)] = &[
    ("[a]", CALLER_CONTENT),
    ("[&_a]", CALLER_CONTENT),
    ("[&_p:not(:last-child)]", CALLER_CONTENT),
    ("file", "file-input button; egui has no file input"),
    ("[.border-b]", "card header with an explicit border class is not offered"),
    ("[.border-t]", "card footer with an explicit border class is not offered"),
    ("has-[>img:first-child]", LEADING_IMAGE),
    ("[img:first-child]", LEADING_IMAGE),
    ("[img:last-child]", "card with a trailing image is not offered"),
    ("group-has-[:focus-visible]/field-label", "inside a focused FieldLabel; the crate does not ship Field"),
    ("[span]", "select item text wrapper; the crate draws the item text itself"),
    ("data-[slot=input-group]", INPUT_GROUP),
    ("in-data-[slot=input-group]", INPUT_GROUP),
    ("[&>[data-slot=input-group]:has(:focus-visible)]", INPUT_GROUP),
    (
        "[&>[data-slot=select-trigger]:not([class*='w-'])]",
        "a select trigger in a button group keeps `w-fit`, its default width",
    ),
    (
        "has-[select[aria-hidden=true]:last-child]",
        "a hidden native `<select>` after a select trigger; the crate renders none, so the trigger is the group's last child (`LAST`)",
    ),
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
    // shadcn-svelte's `@custom-variant`s in `shadcn-svelte/tailwind.css`
    // order. A custom variant with a new name is registered after every
    // built-in one (`Variants.set` takes `lastOrder + 1`), so these sort
    // after `dark` and take precedence over the `data` root's functional
    // form.
    ("data-open", CUSTOM),
    ("data-closed", CUSTOM + 1),
    ("data-checked", CUSTOM + 2),
    ("data-unchecked", CUSTOM + 3),
    ("data-disabled", CUSTOM + 4),
    ("data-active", CUSTOM + 5),
    ("data-horizontal", CUSTOM + 6),
    ("data-vertical", CUSTOM + 7),
];

/// The first order after the built-in variants.
const CUSTOM: u32 = 2000;

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

/// The rank bit of a pseudo-element (`placeholder:`), below
/// [`R_IMPORTANT`].
pub const R_PSEUDO_ELEMENT: u32 = 126;

/// Lowest bit of the class steps (5 bits, below [`R_PSEUDO_ELEMENT`]): a
/// class adds `1 << (R_CLASSES + 1)`. The variant bits sit below it.
pub const R_CLASSES: u32 = 121;

/// Pack a rule's rank: see the module docs.
pub fn encode(mask: u128, spec: Spec, important: bool) -> u128 {
    // Fail loudly rather than carry into the next field.
    assert!(
        mask >> R_CLASSES == 0,
        "variant bits overflow the rank mask"
    );
    assert!(spec.earlier <= spec.classes, "{spec:?}");
    assert!(
        spec.elements <= 1 && (spec.elements == 0 || spec.earlier == 0),
        "element count {spec:?} does not fit its half step"
    );
    let steps = 2 * spec.classes + spec.elements - spec.earlier;
    assert!(
        steps < 1 << (R_PSEUDO_ELEMENT - R_CLASSES),
        "specificity class count {} overflows its rank bits",
        spec.classes
    );
    assert!(
        spec.pseudo_elements <= 1,
        "pseudo-element count {} overflows its rank bit",
        spec.pseudo_elements
    );
    mask | u128::from(steps) << R_CLASSES
        | u128::from(spec.pseudo_elements) << R_PSEUDO_ELEMENT
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
            out.last().is_none_or(|(_, r)| *r < R_CLASSES),
            "more than {R_CLASSES} variant ranks"
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
    pub all: u128,
    pub none: u128,
    /// The class's [`rank`].
    pub rank: u128,
    /// A pseudo target selects the field. It adds its specificity to the
    /// rank, but no variant bit.
    pub pseudo: Option<Pseudo>,
    /// A child target moves the class to a child (D-95 (a)): `all` and
    /// `none` are then the child's condition.
    pub child: Option<ChildSel>,
}

/// What a child-target variant selects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChildSel {
    /// Any child the part lays out: the part's `_child` recipe, which a
    /// child stacks over its own while it sits in the part.
    Any,
    /// The child part with this `data-slot`: that part's own recipe, under
    /// the context flags `resolve::CONTEXTS` names.
    Slot(String),
}

/// Two classes, attributes or pseudo-classes: (0,2,0).
const TWO_CLASSES: Spec = Spec {
    classes: 2,
    pseudo_elements: 0,
    elements: 0,
    earlier: 0,
};
/// A class and an element: (0,1,1).
const CLASS_ELEMENT: Spec = Spec {
    classes: 1,
    pseudo_elements: 0,
    elements: 1,
    earlier: 0,
};

/// Every child-target variant, by exact text: the child it selects (`None`:
/// any child), the flags the selector asks of the child, and the
/// specificity the selector adds beyond the class itself. `*:` is
/// `:is(& > *)` and `**:` is `:is(& *)`, which add nothing; a
/// `data-[slot=S]` right after either names the child part and adds its
/// attribute (one class). A child target adds no variant bit.
pub const CHILD_TARGETS: &[(&str, Option<&str>, &[&str], Spec)] = &[
    ("*", None, &[], ZERO),
    ("**", None, &[], ZERO),
    ("[&>*:not(:first-child)]", None, &["NOT_FIRST"], CLASS),
    ("[&>*:not(:last-child)]", None, &["NOT_LAST"], CLASS),
    // The last child with a data-slot: every crate child counts.
    (
        "[&>[data-slot]:not(:has(~[data-slot]))]",
        None,
        &["LAST"],
        TWO_CLASSES,
    ),
    ("[&>input]", Some("input"), &[], ELEMENT),
    (
        "[&>input:focus-visible]",
        Some("input"),
        &["FOCUS_VISIBLE"],
        CLASS_ELEMENT,
    ),
    (
        "[&>[data-slot=select-trigger]:focus-visible]",
        Some("select-trigger"),
        &["FOCUS_VISIBLE"],
        TWO_CLASSES,
    ),
];

/// Why a `**:` class without a slot is ignored.
pub const DESCENDANTS: &str = "`**:` without a child slot styles content the part draws in its own text colour (select item) or caller content";

/// Flags a parent may hand to its children as context: a condition before a
/// child target must be one of them, or the class is unsupported (the
/// parent's hover is not the child's).
fn context_flag(f: &str) -> bool {
    matches!(f, "DARK" | "SM" | "MD")
        || f.starts_with("HAS_")
        || f.starts_with("SIZE_")
        || f.starts_with("VARIANT_")
}

/// `*:[svg]` and `*:[svg:not([class*='size-'])]` name the part's own icon,
/// as `[&>svg]` and `[&_svg:not([class*='size-'])]` do, with the same
/// specificity: the pairs are spelt as those.
pub fn normalise(variants: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(variants.len());
    let mut i = 0;
    while i < variants.len() {
        let pair = variants.get(i + 1).map(String::as_str);
        match (variants[i].as_str(), pair) {
            ("*", Some("[svg]")) => {
                out.push("[&>svg]".into());
                i += 2;
            }
            ("*", Some("[svg:not([class*='size-'])]")) => {
                out.push("[&_svg:not([class*='size-'])]".into());
                i += 2;
            }
            _ => {
                out.push(variants[i].clone());
                i += 1;
            }
        }
    }
    out
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
    rank_with(variants, ZERO, important)
}

/// [`rank`] with `extra` specificity (a child target's selector).
pub fn rank_with<'a>(
    variants: impl IntoIterator<Item = &'a str>,
    extra: Spec,
    important: bool,
) -> u128 {
    let mut seen: Vec<&str> = Vec::new();
    let (mut mask, mut spec) = (0u128, extra);
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

/// The variant that stands for a state flag: the first [`TRANSLATED`]
/// entry that sets it. The list names the part's own attribute or
/// pseudo-class first (`hover`, `aria-expanded`, `data-[size=sm]`), a
/// `group-`/`peer-` form only where the flag has no other.
pub fn flag_variant(flag: &str) -> Option<&'static str> {
    TRANSLATED
        .iter()
        .find(|(_, m, _)| matches!(m, Is(f) if *f == flag))
        .map(|(n, _, _)| *n)
}

/// The rank a generated rule under one state flag has, for every flag
/// with a [`flag_variant`], in [`crate::model::FLAGS`] order: what
/// `Prop::on` composes the rank of a state from. Composing adds the class
/// steps and ORs the variant and pseudo-element bits, as [`rank`] does for
/// the flags' variants together.
pub fn state_ranks() -> Vec<(&'static str, u128)> {
    crate::model::FLAGS
        .iter()
        .filter_map(|(f, _)| flag_variant(f).map(|v| (*f, rank([v], false))))
        .collect()
}

/// Combine a class's variants. The first variant that is ignored or
/// unsupported decides for the whole class. A child target
/// ([`CHILD_TARGETS`]) splits them: the flags before it are the parent's
/// context, those after it and its own the child's state; both make the
/// child's condition.
pub fn class_variants(c: &Class) -> Result<Variants, Skip> {
    let vs = normalise(&c.variants);
    let mut out = Variants {
        all: 0,
        none: 0,
        rank: 0,
        pseudo: None,
        child: None,
    };
    let mut extra = ZERO;
    let mut i = 0;
    while i < vs.len() {
        let v = vs[i].as_str();
        i += 1;
        if let Some((_, slot, flags, spec)) = CHILD_TARGETS.iter().find(|(n, ..)| *n == v) {
            if out.child.is_some() {
                return Err(Skip::Unsupported(format!("second child target `{v}`")));
            }
            let named = vs
                .get(i)
                .filter(|_| matches!(v, "*" | "**"))
                .and_then(|n| n.strip_prefix("data-[slot=")?.strip_suffix(']'));
            if v == "**" && named.is_none() {
                return Err(Skip::Ignore(DESCENDANTS));
            }
            if let Some(f) = crate::model::FLAGS
                .iter()
                .find(|(n, b)| (out.all | out.none) & b != 0 && !context_flag(n))
            {
                return Err(Skip::Unsupported(format!(
                    "parent state `{}` before a child target",
                    f.0
                )));
            }
            extra = extra + *spec;
            for f in *flags {
                out.all |= flag(f);
            }
            out.child = Some(match (slot, named) {
                (Some(s), _) => ChildSel::Slot((*s).to_owned()),
                (None, Some(s)) => {
                    i += 1;
                    extra = extra + CLASS;
                    ChildSel::Slot(s.to_owned())
                }
                (None, None) => ChildSel::Any,
            });
            continue;
        }
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
    out.rank = rank_with(vs.iter().map(String::as_str), extra, c.important);
    Ok(out)
}
