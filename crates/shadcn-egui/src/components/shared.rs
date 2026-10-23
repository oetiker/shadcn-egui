//! Shared painting helpers for shadcn-style components.

use crate::style::{pick, pick_when, Recipe, Shift, State, TextSize};
use crate::Theme;
use egui::{Color32, CornerRadius, Painter, Rect, Response, Shape, Stroke, Ui};

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

/// A part's resolved [`opacity`] for painting in `ui`. In a disabled `Ui`
/// (`ui.add_enabled(false, ..)`, `Ui::disable`) egui has faded every shape
/// once already, standing in for the style's `disabled:` fade. So where a
/// rule that needs `DISABLED` wins a part's pick, that part's own opacity
/// is 1.0: the `disabled:` rule replaces the part's base value, as in CSS,
/// and egui's fade replaces the rule. Where none wins, the normal pick
/// applies (a base 0.5 stays 0.5 on top of egui's fade).
pub fn ui_opacity(ui: &Ui, chain: &[&[&Recipe]], s: State) -> f32 {
    if ui.is_enabled() {
        return opacity(chain, s);
    }
    chain
        .iter()
        .map(|stack| match pick_when(stack, |r| &r.opacity, s) {
            Some((when, _)) if when.all.contains(State::DISABLED) => INITIAL_OPACITY,
            Some((_, v)) => v,
            None => INITIAL_OPACITY,
        })
        .product()
}

/// The `Ui`'s painter, faded by the part's [`ui_opacity`] (`chain` as in
/// [`opacity`], for example the style's `disabled:opacity-50`). egui fades
/// shape by shape: where a part lies on another (a switch thumb on its
/// track) the lower part shows through, which CSS opacity on the whole
/// group would not show.
pub fn style_painter(ui: &Ui, chain: &[&[&Recipe]], s: State) -> Painter {
    let mut painter = ui.painter().clone();
    painter.multiply_opacity(ui_opacity(ui, chain, s));
    painter
}

/// Run `add` with everything it paints in `ui` faded by `opacity` (a
/// container part's resolved [`opacity`]), then restore the `Ui`'s own
/// opacity. Unlike a scope, this adds no layout of its own. Parts nested in
/// `add` multiply their own opacity on top, as CSS does. A
/// [`Ui::disable`] inside `add` (a disabled card action) keeps its fade:
/// the `Ui` stays disabled, so what it paints afterwards stays faded too.
pub fn faded<R>(ui: &mut Ui, opacity: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let (before, was_enabled) = (ui.opacity(), ui.is_enabled());
    ui.multiply_opacity(opacity);
    let inner = add(ui);
    ui.set_opacity(before);
    // Restoring `before` undid the fade egui's `disable` multiplied in.
    if was_enabled && !ui.is_enabled() {
        ui.multiply_opacity(ui.visuals().disabled_alpha());
    }
    inner
}

/// The height of one line of a part's text: its `leading-*` times the font
/// size or, without one, Tailwind's line height of the text size (`text-*`
/// sets both). This is the content height an `auto` height starts from.
pub fn line_box(theme: &Theme, layers: &[&Recipe], s: State, size: TextSize) -> f32 {
    pick(layers, |r| &r.line_height, s)
        .map_or_else(|| theme.line_px(size), |k| k * theme.text_px(size))
}

/// Build a CornerRadius from an f32 radius, rounding to the nearest pixel.
pub fn corner(r: f32) -> CornerRadius {
    CornerRadius::same(r.round() as u8)
}

/// shadcn's focus/invalid ring: a `ring-N` box-shadow drawn just outside
/// the part's border box.
pub fn ring(painter: &egui::Painter, rect: Rect, radius: f32, width: f32, color: Color32) {
    painter.extend(ring_shape(rect, radius, width, color));
}

/// The shape of [`ring`], if it has any width and colour.
pub fn ring_shape(rect: Rect, radius: f32, width: f32, color: Color32) -> Option<Shape> {
    (width > 0.0 && color.a() > 0).then(|| {
        Shape::rect_stroke(
            rect,
            radius,
            Stroke::new(width, color),
            egui::StrokeKind::Outside,
        )
    })
}

/// A part's background and border, as shapes in paint order. The border
/// lies inside `rect` (`box-sizing: border-box`). With `clip_padding`
/// (`bg-clip-padding`) the fill stops inside the border, so a transparent or
/// translucent border shows what lies under the part instead of the fill.
pub fn box_shapes(
    rect: Rect,
    radius: f32,
    fill: Option<Color32>,
    border: Option<Stroke>,
    clip_padding: bool,
) -> Vec<Shape> {
    let border = border.filter(|b| b.width > 0.0);
    let mut out = Vec::new();
    if let Some(fill) = fill {
        let inset = if clip_padding {
            border.map_or(0.0, |b| b.width)
        } else {
            0.0
        };
        out.push(Shape::rect_filled(
            rect.shrink(inset),
            (radius - inset).max(0.0),
            fill,
        ));
    }
    if let Some(b) = border {
        out.push(Shape::rect_stroke(
            rect,
            radius,
            b,
            egui::StrokeKind::Inside,
        ));
    }
    out
}

/// Paint [`box_shapes`].
pub fn paint_box(
    painter: &egui::Painter,
    rect: Rect,
    radius: f32,
    fill: Option<Color32>,
    border: Option<Stroke>,
    clip_padding: bool,
) {
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
    let context = if live {
        context
    } else {
        context.without(pointer_and_focus)
    };
    theme.state(
        context
            .with(State::HOVER, live && resp.hovered())
            .with(State::ACTIVE, live && resp.is_pointer_button_down_on())
            .with(State::FOCUS_VISIBLE, live && resp.has_focus())
            .with(State::DISABLED, !live),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_multiplies_down_the_part_tree() {
        let mut half = Recipe::default();
        half.opacity.base = Some(0.5);
        let part: &[&Recipe] = &[&half];
        let parent: &[&Recipe] = &[&half];
        assert_eq!(opacity(&[part, parent], State::default()), 0.25);
        assert_eq!(opacity(&[], State::default()), 1.0);
    }
}
