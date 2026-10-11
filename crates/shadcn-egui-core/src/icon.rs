//! Vector icon data. Painting lives in `shadcn_egui::components::icon`.

use crate::blob::{self, BlobRange, BlobSegs, FillMesh};

/// One path command in icon units (the SVG's user space, origin top-left).
/// A `M` starts a new subpath; `Z` closes the current one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    M(f32, f32),
    L(f32, f32),
    Q(f32, f32, f32, f32),
    C(f32, f32, f32, f32, f32, f32),
    Z,
}

/// A generated icon: its name (for screen readers), the side of its square
/// view box, and where its path data lives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Icon {
    pub name: &'static str,
    pub size: f32,
    pub shape: IconShape,
}

/// Where an icon's path data lives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IconShape {
    /// Path commands in a Rust slice (icongen `--out`); strokes only.
    Segs(&'static [Seg]),
    /// A record in an icon crate's blob (icongen `--crate-dir`): strokes and
    /// an optional fill, in the 24-unit box.
    Blob(BlobRange),
}

impl Icon {
    /// The stroke commands, in icon units. A non-empty sequence starts with `Seg::M`.
    pub fn segs(&self) -> SegIter {
        match self.shape {
            IconShape::Segs(s) => SegIter::Slice(s.iter()),
            IconShape::Blob(r) => SegIter::Words(BlobSegs::new(r)),
        }
    }

    /// The pre-triangulated fill; `None` for stroke-only icons.
    pub fn fill_mesh(&self) -> Option<FillMesh> {
        match self.shape {
            IconShape::Segs(_) => None,
            IconShape::Blob(r) => blob::fill_mesh(r),
        }
    }
}

/// The stroke commands of an [`Icon`], from either storage.
#[derive(Clone, Debug)]
pub enum SegIter {
    Slice(core::slice::Iter<'static, Seg>),
    Words(BlobSegs),
}

impl Iterator for SegIter {
    type Item = Seg;

    fn next(&mut self) -> Option<Seg> {
        match self {
            SegIter::Slice(it) => it.next().copied(),
            SegIter::Words(w) => w.next(),
        }
    }
}

/// The icons the components draw, one per role. Each icon crate exports its
/// set as `SLOTS`; `shadcn_egui::Theme::icons` holds the active one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconSlots {
    /// Checkbox tick and the selected Select item.
    pub check: Icon,
    /// Select trigger (each library uses its own icon here).
    pub chevron_down: Icon,
    /// Dialog close button.
    pub close: Icon,
}
