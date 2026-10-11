//! The icon sets this build bundles, by upstream library id. Lucide is
//! always bundled; every other set needs its `icons-<id>` feature.

/// Lucide, the default icon set (always available).
pub use shadcn_egui_icons_lucide as lucide;

use crate::preset::IconLibrary;
use shadcn_egui_core::IconSlots;

/// The slot icons of `lib`, or `None` when its feature is off.
pub fn slots(lib: IconLibrary) -> Option<IconSlots> {
    match lib {
        IconLibrary::Lucide => Some(lucide::SLOTS),
        _ => None,
    }
}

/// The warning `Preset::theme` logs when [`slots`] has nothing for `lib`.
pub(crate) fn fallback_warning(lib: IconLibrary) -> String {
    let id = lib.id();
    format!("shadcn-egui: icon library \"{id}\" needs the \"icons-{id}\" feature; using \"lucide\"")
}
