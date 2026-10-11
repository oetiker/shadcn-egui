//! Vector icon data. Painting lives in `shadcn_egui::components::icon`.

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
/// view box, and its path commands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Icon {
    pub name: &'static str,
    pub size: f32,
    pub segs: &'static [Seg],
}
