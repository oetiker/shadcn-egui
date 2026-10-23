//! Shared painting helpers for shadcn-style components.

use crate::box_shadow::clamp_radii;
use crate::style::{pick, pick_ranked, Paint, Prop, Radius, Recipe, Shift, State, TextSize};
use crate::{ShadowLayer, Theme};
use egui::{
    Color32, Context, CornerRadius, Id, Key, Painter, PointerButton, Rect, Response, Shape, Stroke,
    Ui,
};

/// CSS's initial `opacity`.
const INITIAL_OPACITY: f32 = 1.0; // behaviour-ok: CSS initial value of opacity

/// shadcn's `bits-ui` `sideOffset` default of `SelectContent` and
/// `PopoverContent`: a component prop, not a class.
pub const DEFAULT_SIDE_OFFSET: f32 = 4.0; // behaviour-ok: upstream prop default, not a class

/// A part's `opacity` under `s`, multiplied down the part tree as CSS does:
/// `chain` is the part's recipe stack, then its parent's, and so on.
pub fn opacity(chain: &[&[&Recipe]], s: State) -> f32 {
    chain
        .iter()
        .map(|stack| pick(stack, |r| &r.opacity, s).unwrap_or(INITIAL_OPACITY))
        .product()
}

/// The `Ui`'s opacity without egui's disabled fade. `Ui::disable` multiplies
/// the painter's opacity by `visuals.disabled_alpha`; a nested disable fades
/// again, which the `Ui` does not report, so one fade only is taken out.
fn opacity_without_fade(ui: &Ui) -> f32 {
    let fade = ui.visuals().disabled_alpha();
    if ui.is_enabled() || fade <= 0.0 {
        ui.opacity()
    } else {
        (ui.opacity() / fade).min(1.0) // behaviour-ok: opacity never exceeds 1
    }
}

/// The opacity a crate part paints at in `ui`: the `Ui`'s opacity with one
/// egui disabled fade taken out, times the part's [`opacity`] (`chain` as
/// there) under `s`, with `DISABLED` added in a disabled `Ui`. So a part
/// fades by its style's `disabled:` rule, not by egui's fade, and a part
/// with no such rule stays opaque in a disabled `Ui`, as in a disabled
/// `<fieldset>` (conformance 2.2).
///
/// A nested disable fades twice: `ui.add_enabled_ui(false, ..)` inside a
/// `Ui` that is already disabled multiplies egui's fade in again, which the
/// `Ui` does not report, so a part there paints at that one extra fade.
/// Disable an area once.
pub fn part_opacity(ui: &Ui, chain: &[&[&Recipe]], s: State) -> f32 {
    let s = s.with(
        State::DISABLED,
        s.contains(State::DISABLED) || !ui.is_enabled(),
    );
    opacity_without_fade(ui) * opacity(chain, s)
}

/// The `Ui`'s painter at the part's [`part_opacity`]. egui fades shape by
/// shape: where a part lies on another (a switch thumb on its track) the
/// lower part shows through, which CSS opacity on the whole group would not
/// show.
pub fn part_painter(ui: &Ui, chain: &[&[&Recipe]], s: State) -> Painter {
    let mut painter = ui.painter().clone();
    painter.set_opacity(part_opacity(ui, chain, s));
    painter
}

/// Run `add` with `ui` at `opacity`, then restore the `Ui`'s own opacity.
/// A [`Ui::disable`] inside `add` (a disabled card action) keeps its fade:
/// the `Ui` stays disabled, so what it paints afterwards stays faded too.
fn with_opacity<R>(ui: &mut Ui, opacity: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let (before, was_enabled) = (ui.opacity(), ui.is_enabled());
    ui.set_opacity(opacity);
    let inner = add(ui);
    ui.set_opacity(before);
    // Restoring `before` undid the fade egui's `disable` multiplied in.
    if was_enabled && !ui.is_enabled() {
        ui.multiply_opacity(ui.visuals().disabled_alpha());
    }
    inner
}

/// Run `add`, which draws a crate part through egui (a title through
/// `ui.label`), at the part's [`part_opacity`]. Unlike a scope, this adds no
/// layout of its own.
pub fn part_ui<R>(
    ui: &mut Ui,
    chain: &[&[&Recipe]],
    s: State,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let opacity = part_opacity(ui, chain, s);
    with_opacity(ui, opacity, add)
}

/// Run `add`, the caller's content of a container part, at the part's
/// opacity. egui's disabled fade stays, so plain egui widgets in it show the
/// theme's `disabled_alpha`, and crate parts in it take it out again with
/// [`part_opacity`]. Unlike a scope, this adds no layout of its own.
///
/// A nested disable fades twice, here as in [`part_opacity`]:
/// `ui.add_enabled_ui(false, ..)` inside a disabled container fades its
/// widgets by the theme's `disabled_alpha` once more.
pub fn content_ui<R>(
    ui: &mut Ui,
    chain: &[&[&Recipe]],
    s: State,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let opacity = ui.opacity()
        * opacity(
            chain,
            s.with(
                State::DISABLED,
                s.contains(State::DISABLED) || !ui.is_enabled(),
            ),
        );
    with_opacity(ui, opacity, add)
}

/// The height of one line of a part's text: its `leading-*` times the font
/// size or, without one, Tailwind's line height of the text size (`text-*`
/// sets both). This is the content height an `auto` height starts from.
pub fn line_box(theme: &Theme, layers: &[&Recipe], s: State, size: TextSize) -> f32 {
    pick(layers, |r| &r.line_height, s)
        .map_or_else(|| theme.line_px(size), |k| k * theme.text_px(size))
}

/// A part's box resolved from its recipe stack under one [`State`]: fill,
/// border, shadow, focus or invalid ring (`ring-*`) and outline. Every crate
/// widget and panel paints its box through it.
///
/// Resolve geometry (padding, size) with the state without interaction
/// flags, and the box with the [`interaction_state`] of the paint, so the
/// colours follow hover and focus while the size does not.
///
/// ```
/// use shadcn_egui::components::shared::RecipeBox;
/// use shadcn_egui::style::State;
/// use shadcn_egui::Theme;
///
/// fn paint_card(ui: &egui::Ui, rect: egui::Rect) {
///     let theme = Theme::current(ui.ctx());
///     let card = [&*theme.style.card];
///     let state = theme.state(ui.ctx(), State::NONE);
///     RecipeBox::resolve(&theme, &card, state).paint(ui.painter(), rect);
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RecipeBox {
    /// Corner radii in px before CSS's clamping: nw, ne, sw, se.
    radii: [f32; 4],
    fill: Option<Color32>,
    border_color: Color32,
    /// Border widths in px: top, right, bottom, left.
    borders: [f32; 4],
    clip_padding: bool,
    /// `box-shadow` layers, first on top, and their fade.
    shadow: &'static [ShadowLayer],
    shadow_opacity: f32,
    ring: Option<Stroke>,
    outline: Option<Stroke>,
}

/// `rounded-full` (`calc(infinity * 1px)`) as a finite length, so CSS's
/// corner clamping can scale it.
const FULL_RADIUS: f32 = 1.0e9; // behaviour-ok: stands in for CSS's infinite radius

impl RecipeBox {
    /// The box of the part whose recipes are `layers` (in upstream's class
    /// order, as for [`pick`]) under `state`.
    ///
    /// A side's border width (`border-t-*`, `border-l-*`) and a corner's
    /// radius (`rounded-t-*`, `rounded-b-*`, `rounded-l-*`, `rounded-r-*`)
    /// win against `border-*` and `rounded-*` by rank, as in CSS; at equal
    /// rank the later layer wins (tailwind-merge drops the earlier class),
    /// and within one layer the side wins (Tailwind sorts the shorthand
    /// first). A border without a colour of its own takes shadcn's base
    /// `border-border`.
    pub fn resolve(theme: &Theme, layers: &[&Recipe], state: State) -> Self {
        // Candidates in Tailwind's property order: a later one wins a tie.
        fn side<T: Copy>(cands: &[Option<((u128, usize), T)>]) -> Option<T> {
            let mut best: Option<((u128, usize), T)> = None;
            for c in cands.iter().flatten() {
                if best.is_none_or(|(k, _)| c.0 >= k) {
                    best = Some(*c);
                }
            }
            best.map(|(_, v)| v)
        }
        let paint = |f: fn(&Recipe) -> &Prop<Paint>| pick(layers, f, state).map(|p| theme.paint(p));
        let width = |f: fn(&Recipe) -> &Prop<f32>| pick_ranked(layers, f, state);
        let all = width(|r| &r.border_width);
        let top = width(|r| &r.border_top_width);
        let left = width(|r| &r.border_left_width);
        let borders = [
            side(&[all, top]).unwrap_or(0.0),
            side(&[all]).unwrap_or(0.0),
            side(&[all]).unwrap_or(0.0),
            side(&[all, left]).unwrap_or(0.0),
        ];
        let radius = |f: fn(&Recipe) -> &Prop<Radius>| {
            pick_ranked(layers, f, state).map(|(k, r)| {
                let px = match r {
                    Radius::Rung(crate::style::Rung::Full) => FULL_RADIUS,
                    r => theme.radius_px(r, FULL_RADIUS),
                };
                (k, px)
            })
        };
        let (base, t, r, b, l) = (
            radius(|r| &r.radius),
            radius(|r| &r.radius_top),
            radius(|r| &r.radius_right),
            radius(|r| &r.radius_bottom),
            radius(|r| &r.radius_left),
        );
        // Tailwind's order: rounded, rounded-t, -r, -b, -l.
        let corner = |a, b| side(&[base, a, b]).unwrap_or(0.0);
        let ring_w = pick(layers, |r| &r.ring_width, state).unwrap_or(0.0);
        let outline_w = pick(layers, |r| &r.outline_width, state).unwrap_or(0.0);
        let stroke = |w: f32, c: Option<Color32>| {
            c.filter(|c| w > 0.0 && c.a() > 0)
                .map(|c| Stroke::new(w, c))
        };
        // shadcn's base layer `* { @apply border-border }`.
        let base_border = theme.role(crate::style::Role::Border); // behaviour-ok: CSS base layer colour of every border
        Self {
            radii: [corner(t, l), corner(t, r), corner(b, l), corner(r, b)],
            fill: paint(|r| &r.bg).filter(|c| c.a() > 0),
            border_color: paint(|r| &r.border_color).unwrap_or(base_border),
            borders,
            clip_padding: pick(layers, |r| &r.bg_clip_padding, state) == Some(true),
            shadow: pick(layers, |r| &r.shadow, state).map_or(&[], |s| theme.shadow(s)),
            shadow_opacity: INITIAL_OPACITY,
            ring: stroke(ring_w, paint(|r| &r.ring_color)),
            outline: stroke(outline_w, paint(|r| &r.outline_color)),
        }
    }

    /// The same box with every colour faded by `opacity`: for a box an
    /// egui container paints outside the `Ui` that carries the part's fade
    /// (a popup's frame, a modal's panel).
    #[inline]
    pub fn faded(mut self, opacity: f32) -> Self {
        self.fill = self.fill.map(|c| c.gamma_multiply(opacity));
        self.border_color = self.border_color.gamma_multiply(opacity);
        self.shadow_opacity *= opacity;
        for s in [&mut self.ring, &mut self.outline].into_iter().flatten() {
            s.color = s.color.gamma_multiply(opacity);
        }
        self
    }

    /// The corner radii on `rect`, clamped as CSS clamps them: where two
    /// radii on one side add up to more than its length, all four scale
    /// down by the same factor (so `rounded-full` is half the short side).
    pub fn radius_on(&self, rect: Rect) -> CornerRadius {
        let [nw, ne, sw, se] = clamp_radii(self.radii, rect);
        let c = |v: f32| v.clamp(0.0, 255.0).round() as u8; // behaviour-ok: egui's u8 corner radius
        CornerRadius {
            nw: c(nw),
            ne: c(ne),
            sw: c(sw),
            se: c(se),
        }
    }

    /// The box on `rect` inside `outer` on `outer_rect` with `overflow:
    /// hidden`: a corner that sits on the outer box's corner takes the
    /// rounder of the two radii. Exact while the inner box's other corners
    /// stay clear of the outer box's rounding.
    pub(crate) fn clipped_by(mut self, outer: &RecipeBox, outer_rect: Rect, rect: Rect) -> Self {
        let outer_radii = clamp_radii(outer.radii, outer_rect);
        let near = |a: f32, b: f32| (a - b).abs() < f32::EPSILON * a.abs().max(b.abs()).max(1.0); // behaviour-ok: float equality tolerance
        let corners = [
            (rect.left_top(), outer_rect.left_top()),
            (rect.right_top(), outer_rect.right_top()),
            (rect.left_bottom(), outer_rect.left_bottom()),
            (rect.right_bottom(), outer_rect.right_bottom()),
        ];
        for (i, (p, q)) in corners.into_iter().enumerate() {
            if near(p.x, q.x) && near(p.y, q.y) {
                self.radii[i] = self.radii[i].max(outer_radii[i]);
            }
        }
        self
    }

    /// The fill colour, if the recipe sets one that is not transparent.
    #[inline]
    pub fn fill(&self) -> Option<Color32> {
        self.fill
    }

    /// The border widths: top, right, bottom, left.
    #[inline]
    pub fn border_widths(&self) -> [f32; 4] {
        self.borders
    }

    /// The border as one stroke, if all four sides are as wide and not 0.
    fn uniform_border(&self) -> Option<Stroke> {
        let [t, r, b, l] = self.borders;
        (t > 0.0 && t == r && t == b && t == l).then(|| Stroke::new(t, self.border_color))
    }

    /// The box as shapes in paint order on `rect`: shadow (outside the
    /// border box only), ring, fill (inside the border under
    /// `bg-clip-padding`), border inside `rect` (`box-sizing: border-box`),
    /// outline.
    pub fn shapes(&self, ctx: &egui::Context, rect: Rect) -> Vec<Shape> {
        let radius = self.radius_on(rect);
        let mut out = Vec::new();
        out.extend(self.shadow_shape(ctx, rect, radius));
        out.extend(outside_stroke(self.ring, rect, radius));
        let [t, r, b, l] = self.borders;
        if let Some(fill) = self.fill {
            if self.clip_padding && t + r + b + l > 0.0 {
                let inner = padding_box(rect, self.borders);
                out.push(Shape::rect_filled(
                    inner,
                    inner_radius(radius, self.borders),
                    fill,
                ));
            } else {
                out.push(Shape::rect_filled(rect, radius, fill));
            }
        }
        if let Some(stroke) = self.uniform_border() {
            out.push(Shape::rect_stroke(
                rect,
                radius,
                stroke,
                egui::StrokeKind::Inside,
            ));
        } else if t + r + b + l > 0.0 && self.border_color.a() > 0 {
            // Sides of unequal width: the border box minus the padding box.
            let mesh = tessellate(
                ctx,
                &egui::epaint::RectShape::filled(rect, radius, self.border_color),
            );
            let hole = rounded_polygon(
                padding_box(rect, self.borders),
                inner_radius(radius, self.borders),
            );
            out.push(Shape::mesh(clip_outside(&mesh, &hole)));
        }
        out.extend(outside_stroke(self.outline, rect, radius));
        out
    }

    /// Paint [`RecipeBox::shapes`] with `painter`.
    pub fn paint(&self, painter: &Painter, rect: Rect) {
        painter.extend(self.shapes(painter.ctx(), rect));
    }

    /// An egui `Frame` that paints nothing, its inner margin `padding`
    /// (top, right, bottom, left) plus the border widths, for a panel whose
    /// size egui learns only while it runs (a popup, a modal). Its first
    /// call in the panel's closure is [`RecipeBox::reserve`]; after `show`,
    /// [`RecipeBox::paint_reserved`] paints the box under the content.
    ///
    /// ```
    /// use shadcn_egui::components::shared::RecipeBox;
    /// use shadcn_egui::style::State;
    /// use shadcn_egui::Theme;
    ///
    /// fn panel(ctx: &egui::Context) {
    ///     let theme = Theme::current(ctx);
    ///     let popover = [&*theme.style.popover_content];
    ///     let state = theme.state(ctx, State::NONE);
    ///     let panel = RecipeBox::resolve(&theme, &popover, state);
    ///     let shown = egui::Area::new(egui::Id::new("panel")).show(ctx, |ui| {
    ///         panel
    ///             .frame([16.0; 4])
    ///             .show(ui, |ui| {
    ///                 let slot = RecipeBox::reserve(ui);
    ///                 ui.label("Content");
    ///                 slot
    ///             })
    ///             .inner
    ///     });
    ///     panel.paint_reserved(ctx, shown.response.layer_id, shown.inner, shown.response.rect);
    /// }
    /// ```
    pub fn frame(&self, padding: [f32; 4]) -> egui::Frame {
        let [t, r, b, l] = std::array::from_fn(|i| (padding[i] + self.borders[i]).round() as i8);
        egui::Frame::NONE.inner_margin(egui::Margin {
            left: l,
            right: r,
            top: t,
            bottom: b,
        })
    }

    /// Hold the box's place in `ui`'s layer, under everything the panel's
    /// content paints after it.
    pub fn reserve(ui: &Ui) -> egui::layers::ShapeIdx {
        ui.painter().add(Shape::Noop)
    }

    /// Paint the box on `rect` into the place [`RecipeBox::reserve`] held
    /// in `layer`: shadow, ring, fill, border and outline under the content,
    /// as CSS paints a box under its children.
    pub fn paint_reserved(
        &self,
        ctx: &egui::Context,
        layer: egui::LayerId,
        slot: egui::layers::ShapeIdx,
        rect: Rect,
    ) {
        ctx.layer_painter(layer)
            .set(slot, Shape::Vec(self.shapes(ctx, rect)));
    }

    /// Every shadow layer as the browser blurs it, cut to the outside of
    /// the border box: an outer `box-shadow` never shows through the box
    /// (conformance 2.5).
    fn shadow_shape(&self, ctx: &egui::Context, rect: Rect, radius: CornerRadius) -> Option<Shape> {
        (!self.shadow.is_empty() && self.shadow_opacity > 0.0).then(|| {
            // The fill's rect as egui's tessellator places it.
            let rect = on_pixels(ctx, rect);
            Shape::Mesh(crate::box_shadow::shadow_mesh(
                ctx,
                rect,
                clamp_radii(self.radii, rect),
                radius,
                self.shadow,
                self.shadow_opacity,
            ))
        })
    }
}

/// `rect` on the physical pixel grid where egui rounds rects before it
/// fills them, so a shadow's cut meets the fill's edge.
fn on_pixels(ctx: &egui::Context, rect: Rect) -> Rect {
    use egui::emath::GuiRounding as _;
    if ctx.tessellation_options(|o| o.round_rects_to_pixels) {
        rect.round_to_pixels(ctx.pixels_per_point())
    } else {
        rect
    }
}

/// A `ring-*` or `outline-*` just outside the border box.
fn outside_stroke(stroke: Option<Stroke>, rect: Rect, radius: CornerRadius) -> Option<Shape> {
    stroke.map(|s| Shape::rect_stroke(rect, radius, s, egui::StrokeKind::Outside))
}

/// The padding box: the border box minus each side's border.
fn padding_box(rect: Rect, [t, r, b, l]: [f32; 4]) -> Rect {
    Rect::from_min_max(rect.min + egui::vec2(l, t), rect.max - egui::vec2(r, b))
}

/// The padding box's corner radii: the outer radius minus the wider
/// border that meets at the corner.
fn inner_radius(radius: CornerRadius, [t, r, b, l]: [f32; 4]) -> CornerRadius {
    let inset = |v: u8, a: f32, b: f32| (f32::from(v) - a.max(b)).max(0.0).round() as u8;
    CornerRadius {
        nw: inset(radius.nw, t, l),
        ne: inset(radius.ne, t, r),
        sw: inset(radius.sw, b, l),
        se: inset(radius.se, b, r),
    }
}

/// `shape` as the mesh egui's tessellator makes of it in this context.
fn tessellate(ctx: &egui::Context, shape: &egui::epaint::RectShape) -> egui::Mesh {
    let mut tess = egui::epaint::Tessellator::new(
        ctx.pixels_per_point(),
        ctx.tessellation_options(|o| *o),
        [1, 1], // behaviour-ok: no text in a rect, the font atlas size is unused
        Vec::new(),
    );
    let mut mesh = egui::Mesh::default();
    tess.tessellate_rect(shape, &mut mesh);
    mesh
}

/// The convex outline of a rounded rect, as egui's tessellator traces it.
pub(crate) fn rounded_polygon(rect: Rect, radius: CornerRadius) -> Vec<egui::Pos2> {
    let mut points = Vec::new();
    egui::epaint::tessellator::path::rounded_rectangle(&mut points, rect, radius.into());
    points.dedup_by(|a, b| (*a - *b).length_sq() < 1e-6);
    points
}

/// The part of `mesh` that lies outside the convex `polygon`. A triangle
/// that straddles an edge is cut along it, its vertex colours and texture
/// coordinates interpolated; one wholly inside is dropped at once.
pub(crate) fn clip_outside(mesh: &egui::Mesh, polygon: &[egui::Pos2]) -> egui::Mesh {
    use egui::epaint::Vertex;
    let n = polygon.len();
    let mut out = egui::Mesh::with_texture(mesh.texture_id);
    if n < 3 {
        out.append_ref(mesh);
        return out;
    }
    let area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (polygon[i], polygon[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let orient = area.signum();
    // > 0 inside edge `i`, < 0 outside it.
    let side = |i: usize, p: egui::Pos2| {
        let (a, b) = (polygon[i], polygon[(i + 1) % n]);
        orient * (b - a).x.mul_add(p.y - a.y, -(b - a).y * (p.x - a.x))
    };
    let lerp = |a: &Vertex, b: &Vertex, t: f32| Vertex {
        pos: a.pos + (b.pos - a.pos) * t,
        uv: a.uv + (b.uv - a.uv) * t,
        color: a.color.lerp_to_gamma(b.color, t),
    };
    let mut emit = |poly: &[Vertex]| {
        if poly.len() < 3 {
            return;
        }
        let base = out.vertices.len() as u32;
        out.vertices.extend_from_slice(poly);
        for k in 1..poly.len() as u32 - 1 {
            out.add_triangle(base, base + k, base + k + 1);
        }
    };
    // Buffers shared by every triangle: each cut adds at most one vertex.
    let (mut inside, mut next, mut cut_off) = (
        Vec::with_capacity(n + 3),
        Vec::with_capacity(n + 3),
        Vec::with_capacity(n + 3),
    );
    for tri in mesh.indices.as_chunks::<3>().0 {
        let tri = tri.map(|i| mesh.vertices[i as usize]);
        let (mut all_inside, mut outside_one) = (true, false);
        for e in 0..n {
            let d = tri.map(|v| side(e, v.pos));
            if d.iter().all(|&d| d <= 0.0) {
                outside_one = true;
                break;
            }
            all_inside &= d.iter().all(|&d| d >= 0.0);
        }
        if outside_one {
            emit(&tri);
            continue;
        }
        if all_inside {
            continue;
        }
        inside.clear();
        inside.extend_from_slice(&tri);
        for e in 0..n {
            next.clear();
            cut_off.clear();
            for k in 0..inside.len() {
                let (a, b) = (&inside[k], &inside[(k + 1) % inside.len()]);
                let (da, db) = (side(e, a.pos), side(e, b.pos));
                if da >= 0.0 {
                    next.push(*a);
                } else {
                    cut_off.push(*a);
                }
                if (da >= 0.0) != (db >= 0.0) {
                    let v = lerp(a, b, da / (da - db));
                    next.push(v);
                    cut_off.push(v);
                }
            }
            emit(&cut_off);
            std::mem::swap(&mut inside, &mut next);
            if inside.len() < 3 {
                break;
            }
        }
    }
    out
}

/// Apply `translate-x/y` to a part's rect. `Calc(f, n)` is `calc(f × 100% +
/// n)` of the part's own size.
pub fn shift(rect: Rect, x: Option<Shift>, y: Option<Shift>) -> Rect {
    let d = |s: Option<Shift>, own: f32| match s {
        Some(Shift::Px(v)) => v,
        Some(Shift::Calc(f, v)) => own * f + v,
        None => 0.0,
    };
    rect.translate(egui::vec2(d(x, rect.width()), d(y, rect.height())))
}

/// Which keys activate a focused widget: those of upstream's element
/// (conformance 4.1). egui fakes a click for Space and Enter on any focused
/// clickable widget; [`pressable`] replaces that with these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationKeys {
    /// A native `<button>`: Enter clicks on press, Space on release, and
    /// the held Space shows `ACTIVE`.
    Button,
    /// Enter and Space both click on press: bits switch and tab trigger.
    EnterAndSpace,
    /// Space clicks on press, Enter does nothing: bits checkbox.
    Space,
    /// Enter clicks on press: an `<a>`, a link look.
    Enter,
    /// No key clicks: a slider thumb, whose keys are its own.
    None,
}

fn modality_id() -> Id {
    Id::new("shadcn_egui::focus_modality")
}

fn space_id() -> Id {
    Id::new("shadcn_egui::space_down_on")
}

/// The crate-wide focus modality: the focused widget as last seen, whether
/// `:focus-visible` holds for it, and what this pass's input was.
#[derive(Clone, Copy, Debug)]
struct Modality {
    focused: Option<Id>,
    keyboard: bool,
    pass: u64,
    /// The last key press (`true`) or pointer button event (`false`) of
    /// `pass`, if any.
    input: Option<bool>,
}

/// Whether `:focus-visible` holds for the focused widget, as the browser
/// tracks it (conformance 2.6), crate-wide. The modality is taken when
/// focus changes: by the pass's last key press or pointer button event,
/// and a focus move with no input in its pass (an overlay giving focus
/// back, a scripted focus) keeps the one before, as the browser keeps it
/// for a script's `focus()`. A key press while focus stays makes the
/// focused widget focus-visible; a pointer press that moves no focus (a
/// drag on a scroll bar, a press on the focused widget) changes nothing.
/// A key pressed with Ctrl, Alt or Command is a shortcut and counts as no
/// key, as in Chrome. It starts as keyboard, as a page that has seen no
/// input shows a ring on a scripted focus.
///
/// [`Theme::apply`] runs it at the start and end of every pass, so a
/// focus change in a pass where no crate widget asks is taken with that
/// pass's input; without the hooks, the first crate call of a pass does
/// the work (conformance 6).
pub(crate) fn focus_modality_keyboard(ctx: &Context) -> bool {
    let pass = ctx.cumulative_pass_nr();
    let focused = ctx.memory(|m| m.focused());
    let mut m = ctx
        .data(|d| d.get_temp::<Modality>(modality_id()))
        .unwrap_or(Modality {
            focused,
            keyboard: true,
            pass: u64::MAX,
            input: None,
        });
    if m.pass != pass {
        // The raw events: a widget may have consumed the key already.
        m.input = ctx.input(|i| {
            i.raw.events.iter().rev().find_map(|e| match e {
                egui::Event::Key {
                    pressed: true,
                    modifiers,
                    ..
                } => (!(modifiers.alt || modifiers.ctrl || modifiers.command || modifiers.mac_cmd))
                    .then_some(true),
                egui::Event::PointerButton { .. } => Some(false),
                _ => None,
            })
        });
        m.pass = pass;
        if m.input == Some(true) {
            m.keyboard = true;
        }
    }
    if focused != m.focused {
        m.focused = focused;
        m.keyboard = m.input.unwrap_or(m.keyboard);
    }
    ctx.data_mut(|d| d.insert_temp(modality_id(), m));
    m.keyboard
}

/// Run [`focus_modality_keyboard`] at the start and end of every pass of
/// `ctx`; installed once.
pub(crate) fn install_focus_modality(ctx: &Context) {
    let id = Id::new("shadcn_egui::focus_modality_hooks");
    if ctx.data(|d| d.get_temp::<bool>(id).unwrap_or(false)) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(id, true));
    let track = |ui: &mut Ui| {
        focus_modality_keyboard(ui.ctx());
    };
    ctx.on_begin_pass("shadcn_egui::focus_modality", std::sync::Arc::new(track));
    ctx.on_end_pass("shadcn_egui::focus_modality", std::sync::Arc::new(track));
}

/// Make `resp` act on press and keys as upstream's element: a pointer
/// press focuses it, as the browser focuses a button, a tab trigger or a
/// slider thumb on press, without a ring (conformance 2.6, 4.1); and
/// `clicked()` fires for `keys` only. Call it right after the interaction,
/// before reading `clicked()`. A disabled or non-clicking `resp` is left
/// as it is.
pub fn pressable(resp: &mut Response, keys: ActivationKeys) {
    if !resp.enabled() || !resp.sense.senses_click() {
        return;
    }
    let ctx = resp.ctx.clone();
    // A press and its release in one pass leave no press down, only a click.
    let pressed_on = resp.is_pointer_button_down_on() || resp.clicked_by(PointerButton::Primary);
    if pressed_on && ctx.input(|i| i.pointer.any_pressed()) && !resp.has_focus() {
        resp.request_focus();
    }
    let focused = resp.has_focus();
    let (enter, space_down, space_up, access) = ctx.input(|i| {
        (
            i.key_pressed(Key::Enter),
            i.key_pressed(Key::Space),
            i.key_released(Key::Space),
            i.has_accesskit_action_request(resp.id, egui::accesskit::Action::Click),
        )
    });
    let held = ctx.data(|d| d.get_temp::<Id>(space_id())) == Some(resp.id);
    let mut click = false;
    if focused {
        click = match keys {
            ActivationKeys::Button => enter || (space_up && (held || space_down)),
            ActivationKeys::EnterAndSpace => enter || space_down,
            ActivationKeys::Space => space_down,
            ActivationKeys::Enter => enter,
            ActivationKeys::None => false,
        };
    }
    // The held Space of a `<button>`: `ACTIVE` until it comes up or focus
    // leaves, a click only if it comes up on the focused button.
    if keys == ActivationKeys::Button && focused && space_down && !space_up {
        ctx.data_mut(|d| d.insert_temp(space_id(), resp.id));
    } else if held && (space_up || !focused) {
        ctx.data_mut(|d| d.remove::<Id>(space_id()));
    }
    resp.flags
        .set(egui::response::Flags::FAKE_PRIMARY_CLICKED, click || access);
}

/// A widget's [`State`] for one paint: `context` (variant, icon side, …),
/// what pointer and keyboard do to `resp`, and the theme's `DARK`. `FOCUS`
/// is set while `resp` has focus; `FOCUS_VISIBLE` too where the browser's
/// `:focus-visible` would match: focus moved by keyboard, not by a pointer
/// press (crate-wide modality, conformance 2.6). A held Space on a
/// [`pressable`] button is `ACTIVE`. A disabled widget (a disabled `Ui`, or
/// `DISABLED` in `context` for a widget's own `enabled` flag) gets
/// `DISABLED` and no `HOVER`, `ACTIVE`, `FOCUS` or `FOCUS_VISIBLE`, even if
/// `context` has them: shadcn's `disabled:pointer-events-none`. A
/// composite widget passes the response of its focusable inner part (a
/// select's trigger, a slider's thumb), so focus and press are read where
/// they happen. For a widget that takes typed text use [`text_entry_state`].
pub fn interaction_state(theme: &Theme, resp: &Response, context: State) -> State {
    state_of(theme, resp, context, focus_modality_keyboard(&resp.ctx))
}

/// [`interaction_state`] for a widget that takes typed text (input,
/// textarea): the browser's `:focus-visible` matches it whenever it has
/// focus, after a pointer press too.
pub fn text_entry_state(theme: &Theme, resp: &Response, context: State) -> State {
    state_of(theme, resp, context, true)
}

/// Reports `make_info` for `resp` as [`Response::widget_info`] does, then
/// gives the widget an accessible `role` that egui's `WidgetType` lacks
/// (upstream's `role="alert"`, `alertdialog`, `navigation`, `group`).
/// `widget_info` sets a role from the `WidgetType`, so the role is set
/// after it; the name, value and state stay those of `make_info`.
pub fn widget_info_with_role(
    resp: &Response,
    role: egui::accesskit::Role,
    make_info: impl Fn() -> egui::WidgetInfo,
) {
    resp.widget_info(make_info);
    resp.ctx
        .accesskit_node_builder(resp.id, |node| node.set_role(role));
}

fn state_of(theme: &Theme, resp: &Response, context: State, keyboard: bool) -> State {
    let live = resp.enabled() && !context.contains(State::DISABLED);
    let pointer_and_focus = State::HOVER | State::ACTIVE | State::FOCUS | State::FOCUS_VISIBLE;
    let context = if live {
        context
    } else {
        context.without(pointer_and_focus)
    };
    let focused = live && resp.has_focus();
    let space_held = resp.ctx.data(|d| d.get_temp::<Id>(space_id())) == Some(resp.id);
    theme.state(
        &resp.ctx,
        context
            .with(State::HOVER, live && resp.hovered())
            .with(
                State::ACTIVE,
                live && (resp.is_pointer_button_down_on() || focused && space_held),
            )
            .with(State::FOCUS, context.contains(State::FOCUS) || focused)
            .with(State::FOCUS_VISIBLE, focused && keyboard)
            .with(State::DISABLED, !live),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Rung;

    #[test]
    fn opacity_multiplies_down_the_part_tree() {
        let mut half = Recipe::default();
        half.opacity.base = Some(0.5);
        let part: &[&Recipe] = &[&half];
        let parent: &[&Recipe] = &[&half];
        assert_eq!(opacity(&[part, parent], State::default()), 0.25);
        assert_eq!(opacity(&[], State::default()), 1.0);
    }

    fn area(mesh: &egui::Mesh) -> f32 {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let [a, b, c] = [0, 1, 2].map(|k| mesh.vertices[t[k] as usize].pos);
                ((b - a).x * (c - a).y - (b - a).y * (c - a).x).abs() / 2.0
            })
            .sum()
    }

    #[test]
    fn clip_outside_removes_the_polygon() {
        let mut quad = egui::Mesh::default();
        quad.add_colored_rect(
            Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(10.0, 10.0)),
            Color32::BLACK,
        );
        let hole = rounded_polygon(
            Rect::from_min_size(egui::pos2(3.0, 3.0), egui::vec2(4.0, 4.0)),
            CornerRadius::ZERO,
        );
        assert!((area(&clip_outside(&quad, &hole)) - 84.0).abs() < 1e-3);
        // A polygon that covers the mesh leaves nothing; one beside it all.
        let all = rounded_polygon(
            Rect::from_min_size(egui::pos2(-1.0, -1.0), egui::vec2(12.0, 12.0)),
            CornerRadius::ZERO,
        );
        assert_eq!(area(&clip_outside(&quad, &all)), 0.0);
        let beside = rounded_polygon(
            Rect::from_min_size(egui::pos2(20.0, 0.0), egui::vec2(4.0, 4.0)),
            CornerRadius::ZERO,
        );
        assert!((area(&clip_outside(&quad, &beside)) - 100.0).abs() < 1e-3);
    }

    #[test]
    fn a_clipped_box_takes_the_outer_rounding_where_corners_meet() {
        let theme = Theme::vega_light();
        let mut full = Recipe::default();
        full.radius.base = Some(Radius::Rung(Rung::Full));
        let outer = RecipeBox::resolve(&theme, &[&full], State::NONE);
        let inner = RecipeBox::resolve(&theme, &[&Recipe::default()], State::NONE);
        let track = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 6.0));
        let range = Rect::from_min_max(track.min, egui::pos2(40.0, 6.0));
        let r = inner.clipped_by(&outer, track, range).radius_on(range);
        assert_eq!((r.nw, r.sw, r.ne, r.se), (3, 3, 0, 0));
    }

    fn rounded(base: Option<Rung>, left: Option<Rung>, left_rank: u128) -> RecipeBox {
        let mut r = Recipe::default();
        r.radius.base = base.map(Radius::Rung);
        if let Some(l) = left {
            r.radius_left.rules.push(crate::style::Rule {
                when: crate::style::When::ALWAYS,
                rank: left_rank,
                value: Radius::Rung(l),
            });
        }
        let theme = Theme::vega_light();
        RecipeBox::resolve(&theme, &[&r], State::NONE)
    }

    #[test]
    fn corner_radii_resolve_per_side_and_clamp_as_css() {
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(40.0, 20.0));
        // `rounded-full` is half the short side.
        let full = rounded(Some(Rung::Full), None, 0).radius_on(rect);
        assert_eq!(full, CornerRadius::same(10));
        // `rounded-l-none` beside `rounded-md` squares the left corners only.
        let md = Theme::vega_light().rung(Rung::Md).round() as u8;
        let left_none = rounded(Some(Rung::Md), Some(Rung::None), 0).radius_on(rect);
        assert_eq!(
            left_none,
            CornerRadius {
                nw: 0,
                ne: md,
                sw: 0,
                se: md
            }
        );
        // A side ranked below the shorthand loses to it.
        let mut base = Recipe::default();
        base.radius.rules.push(crate::style::Rule {
            when: crate::style::When::ALWAYS,
            rank: 2,
            value: Radius::Rung(Rung::Md),
        });
        base.radius_left.rules.push(crate::style::Rule {
            when: crate::style::When::ALWAYS,
            rank: 1,
            value: Radius::Rung(Rung::None),
        });
        let b = RecipeBox::resolve(&Theme::vega_light(), &[&base], State::NONE);
        assert_eq!(b.radius_on(rect), CornerRadius::same(md));
    }

    #[test]
    fn unequal_borders_paint_the_border_box_minus_the_padding_box() {
        let mut r = Recipe::default();
        r.border_width.base = Some(1.0);
        r.border_left_width.base = Some(0.0);
        let b = RecipeBox::resolve(&Theme::vega_light(), &[&r], State::NONE);
        assert_eq!(b.border_widths(), [1.0, 1.0, 1.0, 0.0]);
        let ctx = egui::Context::default();
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(20.0, 10.0));
        let shapes = b.shapes(&ctx, rect);
        let meshes: Vec<_> = shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Mesh(m) => Some(m),
                _ => None,
            })
            .collect();
        assert_eq!(meshes.len(), 1);
        let padding_box = Rect::from_min_max(egui::pos2(0.0, 1.0), egui::pos2(19.0, 9.0));
        for t in meshes[0].indices.as_chunks::<3>().0 {
            let c = t
                .iter()
                .map(|&i| meshes[0].vertices[i as usize].pos.to_vec2())
                .fold(egui::Vec2::ZERO, |a, b| a + b)
                / 3.0;
            assert!(!padding_box.shrink(0.01).contains(c.to_pos2()), "{c:?}");
        }
        // Top 20 + bottom 20 + right 8, plus egui's feathering band.
        let border = area(meshes[0]);
        assert!((48.0..48.0 + 60.0).contains(&border), "{border}");
    }
}
