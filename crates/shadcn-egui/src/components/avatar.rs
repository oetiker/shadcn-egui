//! shadcn Avatar, ported from shadcn-svelte `ui/avatar/` (`avatar.svelte`,
//! `avatar-image.svelte`, `avatar-fallback.svelte`, `avatar-badge.svelte`,
//! `avatar-group.svelte`, `avatar-group-count.svelte`) at
//! `tools/stylegen/PIN`, which wrap the bits-ui avatar primitive. Every
//! value comes from the style's `cn-avatar*` recipes.
//!
//! As bits-ui does, the fallback shows while the image loads or after it
//! failed, and the image replaces it once it has loaded (bits' `delayMs`
//! is 0 in shadcn and not offered). The image loads through the egui image
//! loaders the app installed.
//!
//! The avatar's `after:` border blends with what lies under it
//! (`mix-blend-darken`, `dark:mix-blend-lighten`). Over the fallback that
//! is one colour; over an image it differs per pixel, so the avatar
//! composites image, rounded clip and border from the image's pixels into
//! one texture at the avatar's device size (conformance 2.8).

use crate::color::Tone;
use crate::components::atom_row::AtomRow;
use crate::components::context;
use crate::components::icon::{paint_icon, Icon};
use crate::components::registry::{self, Group};
use crate::components::shared::{line_width, part_painter, transitioned_text, RecipeBox};
use crate::components::text::TextLook;
use crate::style::app::{default_variant, state_size};
use crate::style::{pick, pick_len, Blend, Paint, Prop, Recipe, Role, State};
use crate::{layout, Theme};
use egui::load::{Bytes, ImagePoll, SizeHint};
use egui::{
    pos2, Color32, ColorImage, Context, Id, InnerResponse, IntoAtoms, Pos2, Rect, Response, Sense,
    TextureHandle, TextureOptions, Ui, UiBuilder, Vec2, Widget, WidgetInfo, WidgetType,
};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

default_variant!(
    /// The avatar's variant: upstream has none, so only app variants
    /// ([`AvatarVariant::app`]) differ from `Default`. An
    /// [`AvatarGroup`]'s variant reaches the avatars and the count in it.
    AvatarVariant,
    /// The avatar's recipes an app variant's edit changes.
    AvatarRecipes {
        /// `data-slot="avatar"`: the root.
        avatar,
        /// The root's `after:` border, blended with what lies under it.
        avatar_after,
        /// `data-slot="avatar-image"`.
        avatar_image,
        /// `data-slot="avatar-fallback"`.
        avatar_fallback,
        /// `data-slot="avatar-badge"`: the dot in the bottom right corner.
        avatar_badge,
        /// `data-slot="avatar-group"`.
        avatar_group,
        /// `data-slot="avatar-group-count"`.
        avatar_group_count,
    }
);

/// `size` of an avatar: `sm`, `default` or `lg`, or an app size
/// ([`AvatarSize::app`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum AvatarSize {
    Sm,
    #[default]
    Default,
    Lg,
    /// An app size: [`AvatarSize::app`].
    #[doc(hidden)]
    App(crate::style::AppVariant<AvatarSize>),
}

state_size!(AvatarSize, AvatarVariant, lg);

/// An avatar image that carries pixels: a uri an installed egui loader
/// reads, or encoded bytes with their uri, as `egui::include_image!` gives
/// them. It converts from `&'static str`, `String` and `(uri, bytes)`.
///
/// A texture (`ImageSource::Texture`) is not accepted: the avatar's border
/// blends with the image's pixels (`mix-blend-darken`), which a texture on
/// the GPU does not hand back. An app with a texture passes the encoded
/// bytes, or a uri its loader reads, instead.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct AvatarImageSource {
    uri: Cow<'static, str>,
    bytes: Option<Bytes>,
}

impl From<&'static str> for AvatarImageSource {
    fn from(uri: &'static str) -> Self {
        Self {
            uri: uri.into(),
            bytes: None,
        }
    }
}

impl From<String> for AvatarImageSource {
    fn from(uri: String) -> Self {
        Self {
            uri: uri.into(),
            bytes: None,
        }
    }
}

impl<U: Into<Cow<'static, str>>, B: Into<Bytes>> From<(U, B)> for AvatarImageSource {
    fn from((uri, bytes): (U, B)) -> Self {
        Self {
            uri: uri.into(),
            bytes: Some(bytes.into()),
        }
    }
}

impl AvatarImageSource {
    /// The decoded pixels, once the loader has them.
    fn load(&self, ctx: &Context, hint: SizeHint) -> Option<Arc<ColorImage>> {
        if self.uri.is_empty() {
            return None; // bits: an empty `src` is an error
        }
        if let Some(bytes) = &self.bytes {
            ctx.include_bytes(self.uri.clone(), bytes.clone());
        }
        match ctx.try_load_image(&self.uri, hint) {
            Ok(ImagePoll::Ready { image }) => Some(image),
            _ => None,
        }
    }
}

/// The dot in an avatar's bottom right corner (`<Avatar.Badge>`), shown
/// with [`Avatar::badge`]. Its colour is the style's (`bg-primary`); a
/// demo's colour (`bg-green-600`) is an app variant of the avatar.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[must_use]
pub struct AvatarBadge {
    icon: Option<Icon>,
}

impl AvatarBadge {
    /// A plain dot.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// An icon in the dot, hidden in a small avatar (`[&>svg]:hidden`).
    #[inline]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// The context an avatar group gives its avatars and its count.
#[derive(Clone, Copy, Debug, PartialEq)]
struct AvatarGroupContext {
    variant: AvatarVariant,
    id: Id,
    /// The sizes the group's avatars had in the previous pass.
    has: State,
}

/// An avatar: `ui.add(Avatar::new("CN").image("https://…/shadcn.png"))`.
///
/// It shows `fallback` until the image has loaded, and when the image
/// fails or none is set. The image needs an egui image loader for its uri
/// (`egui_extras::install_image_loaders`).
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use shadcn_egui::components::avatar::{Avatar, AvatarBadge, AvatarSize};
/// ui.add(
///     Avatar::new("CN")
///         .image("https://github.com/shadcn.png")
///         .alt_text("@shadcn")
///         .size(AvatarSize::Lg)
///         .badge(AvatarBadge::new()),
/// );
/// # });
/// ```
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Avatar<'a> {
    atoms: egui::Atoms<'a>,
    /// The icon of an icon-only fallback, with its accessible name.
    icon: Option<(Icon, String)>,
    image: Option<AvatarImageSource>,
    alt: Option<String>,
    size: AvatarSize,
    variant: Option<AvatarVariant>,
    badge: Option<AvatarBadge>,
}

impl<'a> Avatar<'a> {
    /// An avatar whose fallback shows `fallback`: initials, or text and
    /// icons in one row.
    #[inline]
    pub fn new(fallback: impl IntoAtoms<'a>) -> Self {
        Self {
            atoms: fallback.into_atoms(),
            icon: None,
            image: None,
            alt: None,
            size: AvatarSize::Default,
            variant: None,
            badge: None,
        }
    }

    /// An avatar whose fallback shows `icon` alone; `name` is its
    /// accessible name.
    #[inline]
    pub fn icon_only(icon: Icon, name: impl Into<String>) -> Self {
        let mut a = Self::new(egui::Atoms::default());
        a.icon = Some((icon, name.into()));
        a
    }

    /// The image (`<Avatar.Image src>`), shown in place of the fallback
    /// once it has loaded. See [`AvatarImageSource`] for what it takes.
    #[inline]
    pub fn image(mut self, source: impl Into<AvatarImageSource>) -> Self {
        self.image = Some(source.into());
        self
    }

    /// The image's accessible name (`alt`), as egui's `Image::alt_text`.
    #[doc(alias = "alt")]
    #[inline]
    pub fn alt_text(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    /// The size (`size`), or an app size ([`AvatarSize::app`]).
    #[inline]
    pub fn size(mut self, size: impl Into<AvatarSize>) -> Self {
        self.size = size.into();
        self
    }

    /// An app variant ([`AvatarVariant::app`]). Without one, an avatar in
    /// an [`AvatarGroup`] takes the group's variant.
    #[inline]
    pub fn variant(mut self, v: impl Into<AvatarVariant>) -> Self {
        self.variant = Some(v.into());
        self
    }

    /// A badge in the bottom right corner (`<Avatar.Badge>`).
    #[inline]
    pub fn badge(mut self, badge: AvatarBadge) -> Self {
        self.badge = Some(badge);
        self
    }
}

impl Widget for Avatar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let group = context::find::<AvatarGroupContext>(ui);
        let variant = self
            .variant
            .or(group.map(|g| g.variant))
            .unwrap_or_default();
        let v = t.resolve_pair(variant, self.size);
        let root = [v.part(&t.style, |s| &s.avatar, |r| &r.avatar)];
        let after = [v.part(&t.style, |s| &s.avatar_after, |r| &r.avatar_after)];
        let image = [v.part(&t.style, |s| &s.avatar_image, |r| &r.avatar_image)];
        let fallback = [v.part(&t.style, |s| &s.avatar_fallback, |r| &r.avatar_fallback)];
        let badge = [v.part(&t.style, |s| &s.avatar_badge, |r| &r.avatar_badge)];
        let bits = v.bits().with(State::IN_AVATAR_GROUP, group.is_some());
        // An avatar has no interaction states: geometry and paint are one.
        let st = t.state(ui.ctx(), bits);
        if let Some(g) = group {
            let has = State::NONE
                .with(State::HAS_SIZE_LG, bits.contains(State::SIZE_LG))
                .with(State::HAS_SIZE_SM, bits.contains(State::SIZE_SM));
            registry::report(ui.ctx(), g.id, has);
        }

        let look = TextLook::resolve(ui, &fallback, st);
        let icon_px = pick(&fallback, |r| &r.icon_size, st).unwrap_or(t.metrics.icon);
        let (row, fallback_name) = match self.icon {
            Some((icon, name)) => (AtomRow::new(ui, self.atoms).icons(Some(icon), None), name),
            None => {
                let name = self
                    .atoms
                    .text()
                    .map(|s| s.into_owned())
                    .unwrap_or_default();
                (AtomRow::new(ui, self.atoms), name)
            }
        };
        let row = row.icon_size(icon_px).lay_out(ui, &look);
        // No size: CSS's initial `auto`, the fallback's size.
        let size = Vec2::new(
            pick(&root, |r| &r.width, st).unwrap_or(row.size().x),
            pick(&root, |r| &r.height, st).unwrap_or(row.size().y),
        );
        let ppp = ui.ctx().pixels_per_point();
        let device = [(size.x * ppp).round(), (size.y * ppp).round()];
        let pixels = self.image.as_ref().and_then(|src| {
            src.load(
                ui.ctx(),
                SizeHint::Size {
                    width: device[0] as u32,
                    height: device[1] as u32,
                    maintain_aspect_ratio: true,
                },
            )
        });

        let (id, rect) = layout::allocate_item_space(ui, size);
        let resp = ui.interact(rect, id, Sense::hover());
        let enabled = ui.is_enabled();
        let shown_image = pixels.is_some();
        let alt = self.alt.unwrap_or_default();
        resp.widget_info(|| {
            if shown_image {
                WidgetInfo::labeled(WidgetType::Image, enabled, &alt)
            } else {
                WidgetInfo::labeled(WidgetType::Label, enabled, &fallback_name)
            }
        });
        if !ui.is_rect_visible(rect) {
            return resp;
        }

        let painter = part_painter(ui, id, &[&root[..]], st);
        RecipeBox::resolve(ui.ctx(), &t, &root, st).paint(&painter, rect);
        let border = AfterBorder::resolve(ui.ctx(), &t, &after, st, rect);
        match (pixels, &self.image) {
            (Some(pixels), Some(src)) => {
                let image_box = RecipeBox::resolve(ui.ctx(), &t, &image, st);
                let job = Composite {
                    pixels,
                    uri: src.uri.clone(),
                    size: [device[0] as usize, device[1] as usize],
                    image_radius: radii_px(image_box.radius_on(rect), ppp),
                    border: border.on_device(ppp),
                };
                // The texture is the avatar's size in device pixels, drawn
                // 1:1 on the pixel grid.
                let drawn = Rect::from_min_size(
                    pos2(
                        (rect.min.x * ppp).round() / ppp,
                        (rect.min.y * ppp).round() / ppp,
                    ),
                    Vec2::from(device) / ppp,
                );
                let tex = composite_texture(ui.ctx(), job);
                let uv = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)); // behaviour-ok: the whole texture (UV unit square)
                                                                         // token-ok: an untinted texture
                painter.image(tex.id(), drawn, uv, Color32::WHITE);
            }
            _ => {
                let fallback_box = RecipeBox::resolve(ui.ctx(), &t, &fallback, st);
                fallback_box.paint(&painter, rect);
                let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground where the fallback sets no text colour
                let fg =
                    pick(&fallback, |r| &r.text, st).map_or(Tone::of(inherited), |p| t.tone(p));
                let fg = transitioned_text(ui.ctx(), id, fg, &fallback, st);
                row.paint(ui, &painter, rect, &look, fg);
                border.paint_over(&painter, fallback_box.fill());
            }
        }

        if let Some(b) = self.badge {
            let w = pick_len(&badge, |r| &r.width, st);
            let h = pick_len(&badge, |r| &r.height, st);
            let corner = rect.max
                - Vec2::new(
                    pick_len(&badge, |r| &r.inset_right, st),
                    pick_len(&badge, |r| &r.inset_bottom, st),
                );
            let dot = Rect::from_min_max(corner - Vec2::new(w, h), corner);
            RecipeBox::resolve(ui.ctx(), &t, &badge, st).paint(&painter, dot);
            let hidden = pick(&badge, |r| &r.icon_hidden, st) == Some(true);
            if let (Some(icon), false) = (b.icon, hidden) {
                let side = pick(&badge, |r| &r.icon_size, st).unwrap_or(t.metrics.icon);
                let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground where the badge sets no text colour
                let fg = pick(&badge, |r| &r.text, st).map_or(inherited, |p| t.tone(p).color);
                let at = Rect::from_center_size(dot.center(), Vec2::splat(side));
                paint_icon(&painter, at, &icon, fg, t.metrics.icon_stroke);
            }
        }
        resp
    }
}

/// The root's `after:` border: where it lies, its colour and its blend.
#[derive(Clone, Copy, Debug, PartialEq)]
struct AfterBorder {
    rect: Rect,
    radius: [f32; 4],
    width: f32,
    color: Color32,
    blend: Option<Blend>,
}

impl AfterBorder {
    fn resolve(ctx: &Context, t: &Theme, l: &[&Recipe], st: State, root: Rect) -> Self {
        let inset = |f: fn(&Recipe) -> &Prop<f32>| pick_len(l, f, st);
        let rect = Rect::from_min_max(
            root.min + Vec2::new(inset(|r| &r.inset_left), inset(|r| &r.inset_top)),
            root.max - Vec2::new(inset(|r| &r.inset_right), inset(|r| &r.inset_bottom)),
        );
        let boxed = RecipeBox::resolve(ctx, t, l, st);
        // shadcn's base layer `* { @apply border-border }`.
        let base = Paint::Role(Role::Border, 1.0); // behaviour-ok: CSS base layer colour of every border
        Self {
            rect,
            radius: radii_px(boxed.radius_on(rect), 1.0), // behaviour-ok: radii stay in points here
            width: line_width(pick_len(l, |r| &r.border_width, st), ctx.pixels_per_point()),
            color: t
                .tone(pick(l, |r| &r.border_color, st).unwrap_or(base))
                .color,
            blend: pick(l, |r| &r.blend, st),
        }
    }

    /// The border in device pixels, for the composite.
    fn on_device(self, ppp: f32) -> Self {
        Self {
            rect: Rect::from_min_size(Pos2::ZERO, self.rect.size() * ppp),
            radius: self.radius.map(|r| r * ppp),
            width: self.width * ppp,
            ..self
        }
    }

    /// Paint the border over a flat `backdrop` (the fallback's fill): one
    /// colour, blended as the browser blends it.
    fn paint_over(self, painter: &egui::Painter, backdrop: Option<Color32>) {
        if self.width <= 0.0 || self.color.a() == 0 {
            return;
        }
        let color = match backdrop {
            Some(b) => {
                let [r, g, bl, a] = blend_px(self.blend, premul(b), self.color, ONE);
                // The blended colour over the backdrop, as one stroke.
                // token-ok: the style's colours, blended as the browser does
                Color32::from_rgba_premultiplied(to_u8(r), to_u8(g), to_u8(bl), to_u8(a))
            }
            None => self.color,
        };
        let c = |v: f32| v.round().clamp(0.0, U8) as u8;
        let radius = egui::CornerRadius {
            nw: c(self.radius[0]),
            ne: c(self.radius[1]),
            sw: c(self.radius[2]),
            se: c(self.radius[3]),
        };
        painter.rect_stroke(
            self.rect,
            radius,
            egui::Stroke::new(self.width, color),
            egui::StrokeKind::Inside,
        );
    }
}

const ONE: f32 = 1.0; // behaviour-ok: full coverage, full alpha
const HALF: f32 = 0.5; // behaviour-ok: a pixel's centre, half a pixel's coverage ramp
const U8: f32 = 255.0; // behaviour-ok: the u8 channel range
/// Composites kept at most; the oldest goes first.
const MAX_COMPOSITES: usize = 64; // behaviour-ok: cache bound, not a look

fn radii_px(r: egui::CornerRadius, scale: f32) -> [f32; 4] {
    [r.nw, r.ne, r.sw, r.se].map(|v| f32::from(v) * scale)
}

fn to_u8(v: f32) -> u8 {
    (v * U8).round().clamp(0.0, U8) as u8
}

/// A premultiplied colour as `0..=1` floats.
fn premul(c: Color32) -> [f32; 4] {
    c.to_array().map(|v| f32::from(v) / U8)
}

/// The border colour `src` (premultiplied) at `coverage` over the
/// premultiplied `backdrop`, under `blend`, as CSS composites a blended
/// element: `Cs' = (1 - αb)·Cs + αb·B(Cb, Cs)`, then source-over.
fn blend_px(blend: Option<Blend>, backdrop: [f32; 4], src: Color32, coverage: f32) -> [f32; 4] {
    let [sr, sg, sb, sa] = src.to_srgba_unmultiplied().map(|v| f32::from(v) / U8);
    let ab = backdrop[3];
    let cb = |i: usize| if ab > 0.0 { backdrop[i] / ab } else { 0.0 };
    let mix = |s: f32, b: f32| match blend {
        Some(Blend::Darken) => s.min(b),
        Some(Blend::Lighten) => s.max(b),
        None => s,
    };
    let a = sa * coverage;
    let ch = |i: usize, s: f32| {
        let blended = (ONE - ab) * s + ab * mix(s, cb(i));
        a * blended + (ONE - a) * backdrop[i]
    };
    [ch(0, sr), ch(1, sg), ch(2, sb), a + (ONE - a) * ab]
}

/// What one composite texture is made of.
struct Composite {
    pixels: Arc<ColorImage>,
    uri: Cow<'static, str>,
    size: [usize; 2],
    image_radius: [f32; 4],
    border: AfterBorder,
}

impl Composite {
    fn key(&self) -> Id {
        let b = &self.border;
        Id::new((
            &self.uri,
            Arc::as_ptr(&self.pixels) as usize,
            self.size,
            self.image_radius.map(f32::to_bits),
            b.radius.map(f32::to_bits),
            b.width.to_bits(),
            b.color,
            b.blend.map(|b| b as u8),
        ))
    }

    /// The image cropped to cover the box (`object-cover`), cut to its
    /// rounded box, the border blended over it.
    fn render(&self) -> ColorImage {
        let [w, h] = self.size;
        let src = &*self.pixels;
        let [iw, ih] = src.size.map(|v| v as f32);
        let scale = (w as f32 / iw).max(h as f32 / ih);
        let half = Vec2::new(w as f32, h as f32) * HALF;
        let inner = AfterBorder {
            rect: self.border.rect.shrink(self.border.width),
            radius: self.border.radius.map(|r| (r - self.border.width).max(0.0)),
            ..self.border
        };
        let image_box = Rect::from_min_size(Pos2::ZERO, half * 2.0);
        let mut out = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let p = pos2(x as f32 + HALF, y as f32 + HALF);
                let u = (p.x - half.x) / scale + iw * HALF;
                let v = (p.y - half.y) / scale + ih * HALF;
                let px = sample(src, u, v, ONE / scale);
                let cover = coverage(p, image_box, self.image_radius);
                let backdrop = px.map(|c| c * cover);
                let ring = coverage(p, self.border.rect, self.border.radius)
                    - coverage(p, inner.rect, inner.radius);
                let [r, g, b, a] = if ring > 0.0 && self.border.width > 0.0 {
                    blend_px(self.border.blend, backdrop, self.border.color, ring)
                } else {
                    backdrop
                };
                // token-ok: image pixels and the style's border colour, composited
                out.push(Color32::from_rgba_premultiplied(
                    to_u8(r),
                    to_u8(g),
                    to_u8(b),
                    to_u8(a),
                ));
            }
        }
        ColorImage::new([w, h], out)
    }
}

/// How much of the pixel centred on `p` the rounded box covers.
fn coverage(p: Pos2, rect: Rect, radius: [f32; 4]) -> f32 {
    let c = rect.center();
    let d = p - c;
    let r = match (d.x < 0.0, d.y < 0.0) {
        (true, true) => radius[0],
        (false, true) => radius[1],
        (true, false) => radius[2],
        (false, false) => radius[3],
    }
    .min(rect.width() * HALF)
    .min(rect.height() * HALF);
    let q = d.abs() - rect.size() * HALF + Vec2::splat(r);
    let outside = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - r;
    (HALF - outside).clamp(0.0, ONE)
}

/// The premultiplied colour of `img` around source point (`u`, `v`):
/// bilinear when the image is drawn larger, the average over the
/// `footprint` (in source pixels) when it is drawn smaller.
fn sample(img: &ColorImage, u: f32, v: f32, footprint: f32) -> [f32; 4] {
    let [w, h] = img.size;
    let at = |x: isize, y: isize| {
        let x = x.clamp(0, w as isize - 1) as usize;
        let y = y.clamp(0, h as isize - 1) as usize;
        premul(img.pixels[y * w + x])
    };
    let mut acc = [0.0; 4];
    if footprint <= ONE {
        let (x, y) = (u - HALF, v - HALF);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let (x0, y0) = (x0 as isize, y0 as isize);
        for (dx, dy, wgt) in [
            (0, 0, (ONE - fx) * (ONE - fy)),
            (1, 0, fx * (ONE - fy)),
            (0, 1, (ONE - fx) * fy),
            (1, 1, fx * fy),
        ] {
            let c = at(x0 + dx, y0 + dy);
            for i in 0..4 {
                acc[i] += c[i] * wgt;
            }
        }
        return acc;
    }
    let r = footprint * HALF;
    let (u0, u1, v0, v1) = (u - r, u + r, v - r, v + r);
    let mut total = 0.0;
    for y in v0.floor() as isize..v1.ceil() as isize {
        let wy = (v1.min(y as f32 + ONE) - v0.max(y as f32)).max(0.0);
        for x in u0.floor() as isize..u1.ceil() as isize {
            let wx = (u1.min(x as f32 + ONE) - u0.max(x as f32)).max(0.0);
            let c = at(x, y);
            for i in 0..4 {
                acc[i] += c[i] * wx * wy;
            }
            total += wx * wy;
        }
    }
    if total > 0.0 {
        acc.map(|c| c / total)
    } else {
        acc
    }
}

/// Composite textures by key, each with the tick of its last use.
#[derive(Clone, Default)]
struct CompositeCache {
    map: HashMap<Id, (TextureHandle, u64)>,
    tick: u64,
}

/// The texture of `job`: from the cache, or composited and uploaded once.
fn composite_texture(ctx: &Context, job: Composite) -> TextureHandle {
    let cache_id = Id::new("shadcn_egui::avatar::composite");
    let key = job.key();
    let hit = ctx.data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<CompositeCache>(cache_id);
        cache.tick += 1;
        let tick = cache.tick;
        cache.map.get_mut(&key).map(|(tex, used)| {
            *used = tick;
            tex.clone()
        })
    });
    if let Some(tex) = hit {
        return tex;
    }
    let tex = ctx.load_texture(
        format!("avatar:{}", job.uri),
        job.render(),
        TextureOptions::NEAREST,
    );
    ctx.data_mut(|d| {
        let cache = d.get_temp_mut_or_default::<CompositeCache>(cache_id);
        while cache.map.len() >= MAX_COMPOSITES {
            let oldest = cache.map.iter().min_by_key(|(_, (_, used))| *used);
            let Some((&old, _)) = oldest else { break };
            cache.map.remove(&old);
        }
        let tick = cache.tick;
        cache.map.insert(key, (tex.clone(), tick));
    });
    tex
}

/// A row of overlapping avatars (`<Avatar.Group>`):
/// `AvatarGroup::new().show(ui, |ui| ..)` with [`Avatar`]s and an
/// [`AvatarGroupCount`] inside. The avatars overlap by the style's
/// `-space-x-*` and each gets a ring in the page colour.
///
/// The count takes its size from the largest and smallest avatars of the
/// previous pass; where they change, the pass is discarded and redone. Two
/// groups in one `Ui` whose avatars change between passes need
/// [`AvatarGroup::id_salt`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use shadcn_egui::components::avatar::{avatar_group_count, Avatar, AvatarGroup};
/// AvatarGroup::new().show(ui, |ui| {
///     ui.add(Avatar::new("CN"));
///     ui.add(Avatar::new("LR"));
///     avatar_group_count(ui, "+3");
/// });
/// # });
/// ```
#[must_use = "You should call .show()"]
#[derive(Clone, Copy, Debug, Default)]
pub struct AvatarGroup {
    variant: AvatarVariant,
    id_salt: Option<Id>,
}

impl AvatarGroup {
    /// An empty group.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// An app variant ([`AvatarVariant::app`]); the avatars and the count
    /// inside take it unless they name their own.
    #[inline]
    pub fn variant(mut self, v: impl Into<AvatarVariant>) -> Self {
        self.variant = v.into();
        self
    }

    /// The id under which the group keeps its avatars' sizes from the
    /// previous pass, scoped to the `Ui` (as `TextEdit::id_salt`).
    #[inline]
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    /// Lay out `add`'s avatars in one overlapping row.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::current(ui.ctx());
        let id = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => ui.next_auto_id(),
        };
        let v = t.resolve(self.variant);
        let l = [v.part(&t.style, |_, s| &s.avatar_group, |r| &r.avatar_group)];
        let st = t.state(ui.ctx(), v.bits());
        let group = Group::begin(ui, id);
        let parts = AvatarGroupContext {
            variant: self.variant,
            id,
            has: group.presence(State::HAS_SIZE_LG | State::HAS_SIZE_SM),
        };
        // `flex -space-x-*`: a crate row whose items overlap.
        let space = pick_len(&l, |r| &r.space_x, st);
        let inner = ui.scope_builder(context::push(group.push(UiBuilder::new()), parts), |ui| {
            layout::row(ui, space, add)
        });
        let _ = group.end(ui.ctx());
        inner
    }
}

/// The count at the end of an avatar group (`<Avatar.GroupCount>`):
/// `ui.add(AvatarGroupCount::new("+3"))`, sized after the group's
/// avatars.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct AvatarGroupCount<'a> {
    atoms: egui::Atoms<'a>,
    icon: Option<(Icon, String)>,
}

impl<'a> AvatarGroupCount<'a> {
    /// A count showing `content`: text, or text and icons in one row.
    #[inline]
    pub fn new(content: impl IntoAtoms<'a>) -> Self {
        Self {
            atoms: content.into_atoms(),
            icon: None,
        }
    }

    /// A count showing `icon` alone; `name` is its accessible name.
    #[inline]
    pub fn icon_only(icon: Icon, name: impl Into<String>) -> Self {
        Self {
            atoms: egui::Atoms::default(),
            icon: Some((icon, name.into())),
        }
    }
}

/// `ui.add(AvatarGroupCount::new(content))`.
pub fn avatar_group_count<'a>(ui: &mut Ui, content: impl IntoAtoms<'a>) -> Response {
    ui.add(AvatarGroupCount::new(content))
}

impl Widget for AvatarGroupCount<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let group = context::find::<AvatarGroupContext>(ui);
        let v = t.resolve(group.map(|g| g.variant).unwrap_or_default());
        let l = [v.part(
            &t.style,
            |_, s| &s.avatar_group_count,
            |r| &r.avatar_group_count,
        )];
        let st = t.state(ui.ctx(), v.bits().or(group.map_or(State::NONE, |g| g.has)));
        let look = TextLook::resolve(ui, &l, st);
        let icon_px = pick(&l, |r| &r.icon_size, st).unwrap_or(t.metrics.icon);
        let (row, name) = match self.icon {
            Some((icon, name)) => (AtomRow::new(ui, self.atoms).icons(Some(icon), None), name),
            None => {
                let name = self
                    .atoms
                    .text()
                    .map(|s| s.into_owned())
                    .unwrap_or_default();
                (AtomRow::new(ui, self.atoms), name)
            }
        };
        let row = row.icon_size(icon_px).lay_out(ui, &look);
        let size = Vec2::new(
            pick(&l, |r| &r.width, st).unwrap_or(row.size().x),
            pick(&l, |r| &r.height, st).unwrap_or(row.size().y),
        );
        let (id, rect) = layout::allocate_item_space(ui, size);
        let resp = ui.interact(rect, id, Sense::hover());
        let enabled = ui.is_enabled();
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Label, enabled, &name));
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        let painter = part_painter(ui, id, &[&l[..]], st);
        RecipeBox::resolve(ui.ctx(), &t, &l, st).paint(&painter, rect);
        let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground where the count sets no text colour
        let fg = pick(&l, |r| &r.text, st).map_or(Tone::of(inherited), |p| t.tone(p));
        let fg = transitioned_text(ui.ctx(), id, fg, &l, st);
        row.paint(ui, &painter, rect, &look, fg);
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::AppVariant;

    /// An app variant's edit gets `AvatarRecipes` and changes only what it
    /// names.
    #[test]
    fn edit_compiles_and_changes_its_part() {
        const ONLINE: AppVariant<AvatarVariant> = AvatarVariant::app("online");
        let t = Theme::vega().variant(
            ONLINE,
            AvatarVariant::Default,
            |r: &mut AvatarRecipes, _| r.avatar_badge.recolor(Role::Primary, Role::Destructive),
        );
        let v = t.resolve(AvatarVariant::from(ONLINE));
        let badge = v.part(&t.style, |_, s| &s.avatar_badge, |r| &r.avatar_badge);
        assert_eq!(
            badge.bg.get(State::NONE),
            Some(Paint::Role(Role::Destructive, 1.0))
        );
        let fallback = v.part(&t.style, |_, s| &s.avatar_fallback, |r| &r.avatar_fallback);
        assert_eq!(fallback, &*t.style.avatar_fallback);
    }

    /// An app size from `Lg` keeps the large size's values.
    #[test]
    fn an_app_size_from_lg_is_large() {
        const XL: AppVariant<AvatarSize> = AvatarSize::app("xl");
        let t = Theme::vega().size(XL, AvatarSize::Lg, |r: &mut AvatarRecipes, _| {
            r.avatar.width.set(48.0);
        });
        let v = t.resolve_pair(AvatarVariant::Default, AvatarSize::from(XL));
        let root = v.part(&t.style, |s| &s.avatar, |r| &r.avatar);
        assert_eq!(pick(&[root], |r| &r.width, v.bits()), Some(48.0));
        assert_eq!(pick(&[root], |r| &r.height, v.bits()), Some(40.0));
        assert!(v.bits().contains(State::SIZE_LG));
    }

    /// Darken takes the darker channel, lighten the lighter; a translucent
    /// border mixes the result with the backdrop by its alpha.
    #[test]
    fn the_border_blends_as_css_does() {
        let back = premul(Color32::from_rgb(100, 200, 50));
        let border = Color32::from_rgb(150, 150, 150);
        let px = |b, c| blend_px(b, back, c, ONE).map(to_u8);
        assert_eq!(px(Some(Blend::Darken), border), [100, 150, 50, 255]);
        assert_eq!(px(Some(Blend::Lighten), border), [150, 200, 150, 255]);
        let white10 = Color32::from_rgba_unmultiplied(255, 255, 255, 26);
        let plain = px(None, white10);
        assert_eq!(px(Some(Blend::Lighten), white10), plain);
    }

    /// A pixel inside the box is covered, one outside is not, one on the
    /// edge half.
    #[test]
    fn coverage_follows_the_rounded_box() {
        let r = Rect::from_min_size(Pos2::ZERO, Vec2::splat(32.0));
        let full = [16.0; 4];
        assert_eq!(coverage(pos2(16.0, 16.0), r, full), 1.0);
        assert_eq!(coverage(pos2(0.5, 0.5), r, full), 0.0);
        assert!((coverage(pos2(16.0, 0.0), r, full) - 0.5).abs() < 1e-4);
    }
}
