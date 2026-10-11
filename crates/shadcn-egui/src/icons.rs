//! The icon sets this build bundles, by upstream library id. Lucide is
//! always bundled; every other set needs its `icons-<id>` feature.

use crate::preset::IconLibrary;
use shadcn_egui_core::IconSlots;

/// The slot icons of `lib`, or `None` when its feature is off.
pub fn slots(lib: IconLibrary) -> Option<IconSlots> {
    match lib {
        IconLibrary::Lucide => Some(crate::components::icon::LUCIDE_SLOTS),
        _ => None,
    }
}
