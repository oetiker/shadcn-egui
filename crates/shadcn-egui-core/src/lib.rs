//! Data types shared by `shadcn-egui` and its font and icon crates.
//!
//! Users never name this crate: `shadcn-egui` re-exports everything in it.

mod font;
mod icon;

pub use font::{Face, FontFiles, FontKind, FontSource, WEIGHTS};
pub use icon::{Icon, IconSlots, Seg};
