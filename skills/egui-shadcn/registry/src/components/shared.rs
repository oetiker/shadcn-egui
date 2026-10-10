//! Shared painting helpers for shadcn-style components.

use egui::{Color32, CornerRadius, Rect, Response, Shape, Stroke, Ui};
use crate::style::{Shift, State};
use crate::Theme;

/// Build a CornerRadius from an f32 radius, rounding to the nearest pixel.
pub fn corner(r: f32) -> CornerRadius {
    CornerRadius::same(r.round() as u8)
}

/// Linearly blend `a` toward `b` by `t` in 0..=1 (sRGB space, good enough for hover).
pub fn mix_toward(a: Color32, b: Color32, t: f32) -> Color32 {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    // token-ok: computes a blend of two token colors, introduces none.
    Color32::from_rgba_unmultiplied(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()), a.a())
}

/// shadcn hover = base color at 90% opacity over the background. We approximate
/// by mixing the fill 10% toward the page background.
pub fn hover_fill(fill: Color32, bg: Color32) -> Color32 {
    mix_toward(fill, bg, 0.10)
}

/// Paint the shadcn focus ring: a `Metrics::ring`-wide ring at 50% of the ring color, just outside
/// the widget rect, when the response has keyboard focus.
pub fn focus_ring(ui: &Ui, resp: &Response, theme: &Theme, corner: f32) {
    if resp.has_focus() {
        let ring = theme.palette.ring.gamma_multiply(0.5);
        let rect = resp.rect.expand(2.0);
        ui.painter().rect_stroke(
            rect,
            corner,
            Stroke::new(theme.metrics.ring, ring),
            egui::StrokeKind::Outside,
        );
    }
}

/// shadcn's focus/invalid ring: a `ring-N` box-shadow drawn just outside
/// the part's border box.
pub fn ring(painter: &egui::Painter, rect: Rect, radius: f32, width: f32, color: Color32) {
    painter.extend(ring_shape(rect, radius, width, color));
}

/// The shape of [`ring`], if it has any width and colour.
pub fn ring_shape(rect: Rect, radius: f32, width: f32, color: Color32) -> Option<Shape> {
    (width > 0.0 && color.a() > 0).then(|| Shape::rect_stroke(rect, radius, Stroke::new(width, color), egui::StrokeKind::Outside))
}

/// A part's background and border, as shapes in paint order. The border
/// lies inside `rect` (`box-sizing: border-box`). With `clip_padding`
/// (`bg-clip-padding`) the fill stops inside the border, so a transparent or
/// translucent border shows what lies under the part instead of the fill.
pub fn box_shapes(rect: Rect, radius: f32, fill: Option<Color32>, border: Option<Stroke>, clip_padding: bool) -> Vec<Shape> {
    let border = border.filter(|b| b.width > 0.0);
    let mut out = Vec::new();
    if let Some(fill) = fill {
        let inset = if clip_padding { border.map_or(0.0, |b| b.width) } else { 0.0 };
        out.push(Shape::rect_filled(rect.shrink(inset), (radius - inset).max(0.0), fill));
    }
    if let Some(b) = border {
        out.push(Shape::rect_stroke(rect, radius, b, egui::StrokeKind::Inside));
    }
    out
}

/// Paint [`box_shapes`].
pub fn paint_box(painter: &egui::Painter, rect: Rect, radius: f32, fill: Option<Color32>, border: Option<Stroke>, clip_padding: bool) {
    painter.extend(box_shapes(rect, radius, fill, border, clip_padding));
}

/// Apply `translate-x/y` to a part's rect. `SelfMinus(n)` is `calc(100% - n)`
/// of the part's own size.
pub fn shift(rect: Rect, x: Option<Shift>, y: Option<Shift>) -> Rect {
    let d = |s: Option<Shift>, own: f32| match s {
        Some(Shift::Px(v)) => v,
        Some(Shift::SelfMinus(v)) => own - v,
        None => 0.0,
    };
    rect.translate(egui::vec2(d(x, rect.width()), d(y, rect.height())))
}

/// A widget's [`State`] for one paint: `context` (variant, icon side, …),
/// what pointer and keyboard do to `resp`, and the theme's `DARK`. Keyboard
/// focus is `FOCUS_VISIBLE`: egui focuses a widget by Tab or code, never by
/// click. A disabled widget (`enabled` false, or a disabled `Ui`) gets
/// `DISABLED` and no `HOVER`, `ACTIVE`, `FOCUS` or `FOCUS_VISIBLE`, even if
/// `context` has them: shadcn's `disabled:pointer-events-none`. A composite
/// widget passes the response of its focusable inner part (a select's
/// trigger, a slider's thumb), so focus and press are read where they happen.
pub fn interaction_state(theme: &Theme, resp: &Response, enabled: bool, context: State) -> State {
    let live = enabled && resp.enabled();
    let pointer_and_focus = State::HOVER | State::ACTIVE | State::FOCUS | State::FOCUS_VISIBLE;
    let context = if live { context } else { State(context.0 & !pointer_and_focus.0) };
    theme.state(
        context
            .with(State::HOVER, live && resp.hovered())
            .with(State::ACTIVE, live && resp.is_pointer_button_down_on())
            .with(State::FOCUS_VISIBLE, live && resp.has_focus())
            .with(State::DISABLED, !live),
    )
}
