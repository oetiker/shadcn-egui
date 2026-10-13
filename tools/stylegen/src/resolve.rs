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
            U::Ignore(why) => m.ignored.push((raw.clone(), why)),
            U::Unsupported(why) => m.unsupported.push((raw.clone(), why.to_owned())),
        }
    }
    m.decls = merge(std::mem::take(&mut m.decls));
    m.after = merge(std::mem::take(&mut m.after));
    m
}

/// Base classes dropped by name for one part: `(part, variant, why)`. The
/// component source does not say which element a bits-ui primitive renders.
pub const EXCEPTIONS: &[(&str, &str, &str)] = &[(
    "cn-slider-thumb",
    "disabled",
    "bits-ui renders the slider thumb as a <span>, where `disabled:` never matches",
)];

/// One part of one style, translated.
#[derive(Debug)]
pub struct Part {
    /// As in `markers.txt`: `cn-tabs-trigger`, `slot:separator`.
    pub name: String,
    pub marker: Marker,
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

/// Translate every part for one style, in `components` order.
pub fn parts(css: &[(String, Vec<String>)], components: &[(String, Vec<String>)]) -> Vec<Part> {
    let vars = collect_vars(css);
    components
        .iter()
        .map(|(name, classes)| {
            let mut dropped = Vec::new();
            let kept: Vec<String> = expand(classes, name, css)
                .into_iter()
                .filter(|c| {
                    let hit = EXCEPTIONS.iter().find(|(p, v, _)| {
                        *p == name.as_str() && split(c).variants.iter().any(|x| x == v)
                    });
                    match hit {
                        Some((_, _, why)) => {
                            dropped.push((c.clone(), *why));
                            false
                        }
                        None => true,
                    }
                })
                .collect();
            let mut marker = translate(&kept, &vars);
            marker.ignored.extend(dropped);
            Part {
                name: name.clone(),
                marker,
            }
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
