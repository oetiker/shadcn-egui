//! One marker's classes → field declarations with their conditions and
//! ranks, CSS spacing variables resolved.

use crate::class::split;
use crate::model::{Field, Value};
use crate::utility::{utility, U};
use crate::variant::{class_variants, Skip};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decl {
    pub field: Field,
    pub value: Value,
    pub all: u32,
    pub none: u32,
    /// Tailwind variant-order bitmask (see `variant.rs`).
    pub rank: u128,
    /// Position of the class in the marker's list.
    pub idx: usize,
}

#[derive(Debug, Default)]
pub struct Marker {
    pub decls: Vec<Decl>,
    /// Decls of the part's `after:` pseudo-element.
    pub after: Vec<Decl>,
    pub ignored: Vec<(String, &'static str)>,
    pub unsupported: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug)]
pub struct VarDef {
    pub all: u32,
    pub none: u32,
    pub rank: u128,
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
                    rank: v.rank,
                    px,
                });
            }
        }
    }
    vars
}

pub fn translate(classes: &[String], vars: &Vars) -> Marker {
    let mut m = Marker::default();
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
        match utility(&c.utility, v.pseudo) {
            U::Set(list) => {
                for (field, value) in list {
                    m.decls.push(Decl {
                        field,
                        value,
                        all: v.all,
                        none: v.none,
                        rank: v.rank,
                        idx,
                    });
                }
            }
            U::After(list) => {
                for (field, value) in list {
                    m.after.push(Decl {
                        field,
                        value,
                        all: v.all,
                        none: v.none,
                        rank: v.rank,
                        idx,
                    });
                }
            }
            // Approximation of CSS: a rule derived from a variable inherits
            // the rank of the variable's definition (`rank: v.rank | d.rank`),
            // not the specificity of the element the variable is set on.
            U::UseVar(fields, name) => match vars.get(&name) {
                Some(defs) => {
                    for d in defs {
                        for f in &fields {
                            m.decls.push(Decl {
                                field: *f,
                                value: Value::Px(d.px),
                                all: v.all | d.all,
                                none: v.none | d.none,
                                rank: v.rank | d.rank,
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
            U::Ignore(why) => m.ignored.push((raw.clone(), why)),
            U::Unsupported(why) => m.unsupported.push((raw.clone(), why.to_owned())),
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
        .collect()
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
