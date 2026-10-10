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
                vars.entry(name).or_default().push(VarDef { all: v.all, none: v.none, rank: v.rank, px });
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
                    m.decls.push(Decl { field, value, all: v.all, none: v.none, rank: v.rank, idx });
                }
            }
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
                None => m.unsupported.push((raw.clone(), format!("unknown CSS variable {name}"))),
            },
            U::VarDef(..) => {}
            U::Ignore(why) => m.ignored.push((raw.clone(), why)),
            U::Unsupported(why) => m.unsupported.push((raw.clone(), why.to_owned())),
        }
    }
    m.decls = merge(std::mem::take(&mut m.decls));
    m
}

/// Translate every shipped marker of one parsed style, in `shipped` order.
/// A marker the style does not define translates to nothing.
pub fn markers(css: &[(String, Vec<String>)], shipped: &[String]) -> Vec<(String, Marker)> {
    let vars = collect_vars(css);
    shipped
        .iter()
        .map(|m| {
            let classes = css.iter().find(|(n, _)| n == m).map(|(_, c)| c.as_slice()).unwrap_or_default();
            (m.clone(), translate(classes, &vars))
        })
        .collect()
}

/// tailwind-merge inside one marker: for one field under one exact variant
/// set, only the last class survives.
fn merge(decls: Vec<Decl>) -> Vec<Decl> {
    let mut out: Vec<Decl> = Vec::new();
    for d in decls {
        out.retain(|o| !(o.field == d.field && o.all == d.all && o.none == d.none && o.rank == d.rank));
        out.push(d);
    }
    out
}
