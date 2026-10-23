//! One marker's classes → field declarations with their conditions and
//! ranks, CSS spacing variables resolved.

use crate::class::split;
use crate::model::{Field, Value};
use crate::utility::{utility, U};
use crate::variant::{class_variants, rank, ChildSel, Skip, IGNORED};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decl {
    pub field: Field,
    pub value: Value,
    pub all: u128,
    pub none: u128,
    /// Cascade rank: important, specificity, Tailwind variant order (see
    /// `variant.rs`).
    pub rank: u128,
    /// Position of the class in the marker's list.
    pub idx: usize,
}

#[derive(Debug, Default)]
pub struct Marker {
    pub decls: Vec<Decl>,
    /// Decls of the part's `after:` pseudo-element.
    pub after: Vec<Decl>,
    /// Decls of caller content without a crate part (`*:` with no crate
    /// child to take it): the part's `_child` recipe.
    pub child: Vec<Decl>,
    /// Decls a child target moves to another part, before routing.
    pub moved: Vec<Moved>,
    pub ignored: Vec<(String, &'static str)>,
    pub unsupported: Vec<(String, String)>,
    /// One line per class moved to or from this part (`parts` routes them).
    pub routed: Vec<String>,
}

/// A decl of a child-target class, before [`route`] gives it its part.
#[derive(Clone, Debug, PartialEq)]
pub struct Moved {
    pub raw: String,
    pub child: ChildSel,
    /// The decl lands on the child's `after:` pseudo-element.
    pub after: bool,
    pub decl: Decl,
}

#[derive(Clone, Debug)]
pub struct VarDef {
    pub all: u128,
    pub none: u128,
    /// The defining class's variants and `!important` flag, for the rank
    /// of the rules that use the variable.
    pub variants: Vec<String>,
    pub important: bool,
    pub px: f32,
}

pub type Vars = HashMap<String, Vec<VarDef>>;

/// CSS variables set anywhere in the style (`[--card-spacing:--spacing(4)]`,
/// `data-[size=sm]:[--card-spacing:--spacing(3)]`).
pub fn collect_vars(css: &[(String, Vec<String>)]) -> Vars {
    let mut vars = Vars::new();
    for (_, classes) in css {
        for raw in classes {
            let c = split(raw);
            let Ok(v) = class_variants(&c) else { continue };
            if let U::VarDef(name, px) = utility(&c.utility, v.pseudo) {
                vars.entry(name).or_default().push(VarDef {
                    all: v.all,
                    none: v.none,
                    variants: c.variants.clone(),
                    important: c.important,
                    px,
                });
            }
        }
    }
    vars
}

pub fn translate(classes: &[String], vars: &Vars) -> Marker {
    let mut m = Marker::default();
    type Show = (String, &'static str, u128, u128, u128, usize);
    let mut shows: Vec<Show> = Vec::new();
    for (idx, raw) in classes.iter().enumerate() {
        let c = split(raw);
        let v = match class_variants(&c) {
            Ok(v) => v,
            Err(Skip::Ignore(why)) => {
                m.ignored.push((raw.clone(), why));
                continue;
            }
            Err(Skip::Unsupported(why)) => {
                m.unsupported.push((raw.clone(), why));
                continue;
            }
        };
        let set = |list: Vec<(Field, Value)>, after: bool, m: &mut Marker| {
            for (field, value) in list {
                let decl = Decl {
                    field,
                    value,
                    all: v.all,
                    none: v.none,
                    rank: v.rank,
                    idx,
                };
                match (&v.child, after) {
                    (Some(child), _) => m.moved.push(Moved {
                        raw: raw.clone(),
                        child: child.clone(),
                        after,
                        decl,
                    }),
                    (None, false) => m.decls.push(decl),
                    (None, true) => m.after.push(decl),
                }
            }
        };
        match utility(&c.utility, v.pseudo) {
            U::Set(list) => set(list, false, &mut m),
            U::After(list) => set(list, true, &mut m),
            _ if v.child.is_some() => match utility(&c.utility, v.pseudo) {
                U::Ignore(why) => m.ignored.push((raw.clone(), why)),
                U::Show(why) => m.ignored.push((raw.clone(), why)),
                _ => m.unsupported.push((
                    raw.clone(),
                    "CSS variable or inherited value on a child target".into(),
                )),
            },
            // Approximation of CSS: a rule derived from a variable is ranked
            // as one class with the variants of both the using and the
            // defining class, not by the specificity of the element the
            // variable is set on.
            U::UseVar(fields, name) => match vars.get(&name) {
                Some(defs) => {
                    for d in defs {
                        for f in &fields {
                            m.decls.push(Decl {
                                field: *f,
                                value: Value::Px(d.px),
                                all: v.all | d.all,
                                none: v.none | d.none,
                                rank: rank(
                                    c.variants.iter().chain(&d.variants).map(String::as_str),
                                    c.important || d.important,
                                ),
                                idx,
                            });
                        }
                    }
                }
                None => m
                    .unsupported
                    .push((raw.clone(), format!("unknown CSS variable {name}"))),
            },
            U::VarDef(..) => {}
            U::Inherit(fields, why) => {
                m.decls.retain(|d| {
                    !(fields.contains(&d.field)
                        && d.all == v.all
                        && d.none == v.none
                        && d.rank == v.rank)
                });
                m.ignored.push((raw.clone(), why));
            }
            U::Show(why) => shows.push((raw.clone(), why, v.all, v.none, v.rank, idx)),
            U::Ignore(why) => m.ignored.push((raw.clone(), why)),
            U::Unsupported(why) => m.unsupported.push((raw.clone(), why.to_owned())),
        }
    }
    // A display class shows a part that a `hidden` class hides (`hidden
    // sm:flex`); on a part that is never hidden it is structure.
    let hides = m.decls.iter().any(|d| d.field == Field::Hidden);
    for (raw, why, all, none, rank, idx) in shows {
        if hides {
            m.decls.push(Decl {
                field: Field::Hidden,
                value: Value::Bool(false),
                all,
                none,
                rank,
                idx,
            });
        } else {
            m.ignored.push((raw, why));
        }
    }
    m.decls = merge(std::mem::take(&mut m.decls));
    m.after = merge(std::mem::take(&mut m.after));
    m
}

/// A base class dropped by name for one part: `(part, variant, why)`.
pub type Exception = (&'static str, &'static str, &'static str);

/// Classes an exception dropped, each with its reason.
pub type Dropped = Vec<(String, &'static str)>;

/// Base classes dropped by name for one part. The component source does
/// not say which element a bits-ui primitive renders.
pub const EXCEPTIONS: &[Exception] = &[(
    "cn-slider-thumb",
    "disabled",
    "bits-ui renders the slider thumb as a <span>, where `disabled:` never matches",
)];

/// `classes` (one part, expanded) without the classes `exceptions` drop
/// for `part`, and the dropped ones with their reasons. An exception for
/// `part` that matches no class is an error: a pin bump must not leave a
/// stale entry behind silently, as with [`correct`].
pub fn except(
    exceptions: &[Exception],
    part: &str,
    classes: Vec<String>,
) -> Result<(Vec<String>, Dropped), String> {
    let mut dropped = Vec::new();
    let kept = classes
        .into_iter()
        .filter(|c| {
            let hit = exceptions
                .iter()
                .find(|(p, v, _)| *p == part && split(c).variants.iter().any(|x| x == v));
            match hit {
                Some((_, _, why)) => {
                    dropped.push((c.clone(), *why));
                    false
                }
                None => true,
            }
        })
        .collect();
    for (p, v, _) in exceptions.iter().filter(|(p, _, _)| *p == part) {
        if !dropped
            .iter()
            .any(|(c, _)| split(c).variants.iter().any(|x| x == v))
        {
            return Err(format!(
                "exception ({p}): no `{v}:` class in the class string"
            ));
        }
    }
    Ok((kept, dropped))
}

/// An exception whose part is not in `parts` (the part left `markers.txt`)
/// is an error too.
pub fn exception_parts(exceptions: &[Exception], parts: &[&str]) -> Result<(), String> {
    match exceptions.iter().find(|(p, _, _)| !parts.contains(p)) {
        Some((p, _, _)) => Err(format!("exception ({p}): no such part")),
        None => Ok(()),
    }
}

/// A design correction's edit of a part's class string.
#[derive(Clone, Copy, Debug)]
pub enum Edit {
    /// Replace the class `from` with `to`; `from` must be in the string.
    Replace(&'static str, &'static str),
    /// Add classes at the end; none of them may be in the string already.
    Append(&'static [&'static str]),
}

/// A design correction (spec, "Design corrections"): where upstream's
/// string loses information the user needs, the crate edits it, with the
/// maintainer's approval.
#[derive(Clone, Copy, Debug)]
pub struct Correction {
    /// The spec's name for it: `C1`.
    pub id: &'static str,
    /// The styles it applies to; `None` for every style.
    pub styles: Option<&'static [&'static str]>,
    /// As in `markers.txt`.
    pub part: &'static str,
    pub edit: Edit,
    pub reason: &'static str,
}

/// Applied to a part's full class string after marker expansion and before
/// translation, in list order.
pub const CORRECTIONS: &[Correction] = &[
    Correction {
        id: "C1",
        styles: Some(&["luma", "rhea"]),
        part: "cn-tabs-trigger",
        edit: Edit::Replace("border-transparent!", "border-transparent"),
        reason: "the important border hides the focus border and leaves the dark active tab marked only by a faint fill; the base classes' `Ring` and `Input` borders win again",
    },
    Correction {
        id: "C2",
        styles: None,
        part: "cn-tabs-trigger",
        edit: Edit::Append(&[
            "group-data-[variant=line]/tabs-list:focus-visible:ring-0",
            "group-data-[variant=line]/tabs-list:focus-visible:outline-0",
            "group-data-[variant=line]/tabs-list:focus-visible:border-transparent",
            "group-data-[variant=line]/tabs-list:focus-visible:after:opacity-100",
            "group-data-[variant=line]/tabs-list:focus-visible:after:bg-ring",
            "group-data-[variant=line]/tabs-list:focus-visible:data-active:after:bg-foreground",
            "group-data-[variant=line]/tabs-list:focus-visible:text-foreground",
            "dark:group-data-[variant=line]/tabs-list:focus-visible:text-foreground",
        ]),
        reason: "a line list draws no pills, so focus shows on an inactive trigger as on hover, not as a pill-shaped ring, outline and border: its label turns `Foreground` and its underline shows in `Ring`; C2 adds no focus marker to the active trigger, its underline stays `Foreground` and 2px",
    },
];

/// `classes` (one part of one style, expanded) with every correction for
/// them applied, and one report line per applied correction. An edit that
/// does not match the string is an error: a pin bump must not drop a
/// correction silently.
pub fn correct(
    corrections: &[Correction],
    style: &str,
    part: &str,
    mut classes: Vec<String>,
) -> Result<(Vec<String>, Vec<String>), String> {
    let mut applied = Vec::new();
    for c in corrections {
        if c.part != part || c.styles.is_some_and(|s| !s.contains(&style)) {
            continue;
        }
        match c.edit {
            Edit::Replace(from, to) => {
                let i = classes.iter().position(|x| x == from).ok_or_else(|| {
                    format!(
                        "{} ({style} {part}): `{from}` is not in the class string",
                        c.id
                    )
                })?;
                classes[i] = to.to_owned();
                applied.push(format!(
                    "corrected ({}) `{from}` to `{to}`: {}",
                    c.id, c.reason
                ));
            }
            Edit::Append(add) => {
                if let Some(x) = add.iter().find(|a| classes.iter().any(|x| x == *a)) {
                    return Err(format!(
                        "{} ({style} {part}): `{x}` is in the class string already",
                        c.id
                    ));
                }
                classes.extend(add.iter().map(|a| (*a).to_owned()));
                let list: Vec<String> = add.iter().map(|a| format!("`{a}`")).collect();
                applied.push(format!(
                    "corrected ({}) added {}: {}",
                    c.id,
                    list.join(" "),
                    c.reason
                ));
            }
        }
    }
    Ok((classes, applied))
}

/// One part of one style, translated.
#[derive(Debug)]
pub struct Part {
    /// As in `markers.txt`: `cn-tabs-trigger`, `slot:separator`.
    pub name: String,
    pub marker: Marker,
    /// One line per design correction applied to the part.
    pub corrections: Vec<String>,
}

/// A part's classes for one style: its marker replaced in place by the
/// style's classes for it (nothing where the style does not define it).
pub fn expand(component: &[String], part: &str, css: &[(String, Vec<String>)]) -> Vec<String> {
    let style: &[String] = css
        .iter()
        .find(|(n, _)| n == part)
        .map(|(_, c)| c.as_slice())
        .unwrap_or_default();
    component
        .iter()
        .flat_map(|c| {
            if c == part {
                style.to_vec()
            } else {
                vec![c.clone()]
            }
        })
        .collect()
}

/// Translate every part for one style, in `components` order, with the
/// style's [`CORRECTIONS`] applied.
pub fn parts(
    style: &str,
    css: &[(String, Vec<String>)],
    components: &[(String, Vec<String>)],
) -> Result<Vec<Part>, String> {
    let vars = collect_vars(css);
    let names: Vec<&str> = components.iter().map(|(n, _)| n.as_str()).collect();
    exception_parts(EXCEPTIONS, &names)?;
    components
        .iter()
        .map(|(name, classes)| {
            let (full, corrections) =
                correct(CORRECTIONS, style, name, expand(classes, name, css))?;
            let (kept, dropped) =
                except(EXCEPTIONS, name, full).map_err(|e| format!("{style}: {e}"))?;
            let mut marker = translate(&kept, &vars);
            marker.ignored.extend(dropped);
            Ok(Part {
                name: name.clone(),
                marker,
                corrections,
            })
        })
        .collect::<Result<Vec<Part>, String>>()
        .map(|mut ps| {
            route(&mut ps);
            ps
        })
}

/// The context a crate parent hands its children (rules axiom 5, "Context
/// flows down"): the flags a child of `parent` carries while it sits in
/// it, and, for `*:` rules that name no child slot, the crate parts that
/// can be its children. A parent missing here hands no context: a child
/// target on it is unsupported.
pub struct Context {
    pub parent: &'static str,
    pub flags: &'static [&'static str],
    /// The crate parts a `*:` rule of `parent` reaches. Empty: the rule
    /// styles caller content, which takes the part's `_child` recipe.
    pub children: &'static [&'static str],
}

/// Children of a button group: the crate parts upstream's demos place in
/// one, textarea (a field like input) and the group's separator. A nested
/// group draws no fill, border or ring, so the position rules
/// (`[&>*:not(:first-child)]:rounded-l-none`) change no pixel of it. The
/// group's orientation reaches its children as `IN_GROUP_HORIZONTAL` or
/// `IN_GROUP_VERTICAL`, apart from a child's own `data-orientation`
/// (`HORIZONTAL`, `VERTICAL`): a separator is perpendicular to its group,
/// and a separator that ends the group takes the group's last-child corner.
const BUTTON_GROUP_CHILDREN: &[&str] = &[
    "cn-button",
    "cn-input",
    "cn-textarea",
    "cn-select-trigger",
    "cn-button-group-separator",
    "cn-button-group-text",
];

pub const CONTEXTS: &[Context] = &[
    Context {
        parent: "cn-tooltip-content",
        flags: &["IN_TOOLTIP_CONTENT"],
        children: &[],
    },
    Context {
        parent: "cn-avatar-group",
        flags: &["IN_AVATAR_GROUP"],
        children: &[],
    },
    Context {
        parent: "cn-alert-variant-destructive",
        flags: &["VARIANT_DESTRUCTIVE"],
        children: &[],
    },
    Context {
        parent: "cn-button-group",
        flags: &["IN_BUTTON_GROUP"],
        children: BUTTON_GROUP_CHILDREN,
    },
    Context {
        parent: "cn-button-group-orientation-horizontal",
        flags: &["IN_BUTTON_GROUP", "IN_GROUP_HORIZONTAL"],
        children: BUTTON_GROUP_CHILDREN,
    },
    Context {
        parent: "cn-button-group-orientation-vertical",
        flags: &["IN_BUTTON_GROUP", "IN_GROUP_VERTICAL"],
        children: BUTTON_GROUP_CHILDREN,
    },
];

/// Move every child-target decl to the part that draws the child, under
/// the parent's [`Context`] flags: a named slot to that slot's part, `*:`
/// to each crate child, or to the parent's `_child` recipe where it has
/// none. A slot without a crate part is ignored for the reason its
/// `data-[slot=..]` variant has, else unsupported.
pub fn route(parts: &mut [Part]) {
    for i in 0..parts.len() {
        let moved = std::mem::take(&mut parts[i].marker.moved);
        let parent = parts[i].name.clone();
        let ctx = CONTEXTS.iter().find(|c| c.parent == parent);
        for mv in moved {
            // A slot without a crate part: nothing to route to.
            if let ChildSel::Slot(s) = &mv.child {
                let names = [format!("cn-{s}"), format!("slot:{s}")];
                if !parts.iter().any(|p| names.contains(&p.name)) {
                    let v = format!("data-[slot={s}]");
                    match IGNORED.iter().find(|(n, _)| *n == v) {
                        Some((_, why)) => {
                            let line = (mv.raw, *why);
                            if !parts[i].marker.ignored.contains(&line) {
                                parts[i].marker.ignored.push(line);
                            }
                        }
                        None => parts[i]
                            .marker
                            .unsupported
                            .push((mv.raw, format!("child slot `{s}` has no crate part"))),
                    }
                    continue;
                }
            }
            let Some(ctx) = ctx else {
                parts[i].marker.unsupported.push((
                    mv.raw.clone(),
                    format!("child target on `{parent}`, which hands its children no context"),
                ));
                continue;
            };
            let flags = ctx
                .flags
                .iter()
                .fold(0, |acc, f| acc | crate::model::flag(f));
            let decl = Decl {
                all: mv.decl.all | flags,
                ..mv.decl
            };
            let under = crate::model::state_expr(decl.all);
            let targets: Vec<String> = match &mv.child {
                ChildSel::Slot(s) => {
                    let names = [format!("cn-{s}"), format!("slot:{s}")];
                    parts
                        .iter()
                        .filter(|p| names.contains(&p.name))
                        .map(|p| p.name.clone())
                        .collect()
                }
                ChildSel::Any => ctx.children.iter().map(|c| (*c).to_owned()).collect(),
            };
            if targets.is_empty() {
                if mv.after {
                    parts[i]
                        .marker
                        .unsupported
                        .push((mv.raw, "`after:` on caller content".into()));
                    continue;
                }
                parts[i].marker.child.push(decl);
                let line = format!("`{}` to `{parent}` `_child` under {under}", mv.raw);
                push_unique(&mut parts[i].marker.routed, format!("moved {line}"));
                continue;
            }
            for t in &targets {
                let Some(p) = parts.iter_mut().find(|p| &p.name == t) else {
                    panic!("context child `{t}` of `{parent}` is not a part");
                };
                if mv.after {
                    p.marker.after.push(decl);
                } else {
                    p.marker.decls.push(decl);
                }
                push_unique(
                    &mut p.marker.routed,
                    format!("from `{parent}`: `{}` under {under}", mv.raw),
                );
            }
            push_unique(
                &mut parts[i].marker.routed,
                format!("moved `{}` to {} under {under}", mv.raw, list(&targets)),
            );
        }
        let m = &mut parts[i].marker;
        m.child = merge(std::mem::take(&mut m.child));
    }
}

fn push_unique(v: &mut Vec<String>, line: String) {
    if !v.contains(&line) {
        v.push(line);
    }
}

fn list(names: &[String]) -> String {
    names
        .iter()
        .map(|n| format!("`{n}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// tailwind-merge inside one marker: for one field under one exact variant
/// set, only the last class survives.
fn merge(decls: Vec<Decl>) -> Vec<Decl> {
    let mut out: Vec<Decl> = Vec::new();
    for d in decls {
        out.retain(|o| {
            !(o.field == d.field && o.all == d.all && o.none == d.none && o.rank == d.rank)
        });
        out.push(d);
    }
    out
}
