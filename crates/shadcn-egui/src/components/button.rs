//! shadcn Button, ported from shadcn-svelte `ui/button/button.svelte` at
//! `tools/stylegen/PIN`. Every value comes from the style's button
//! recipes: `cn-button`, `cn-button-variant-*` and `cn-button-size-*`,
//! stacked in that order.

use crate::components::atom_row::AtomRow;
use crate::components::icon::Icon;
use crate::components::shared::{
    interaction_state, line_width, part_painter, pressable, shift, ActivationKeys, RecipeBox,
};
use crate::components::text::TextLook;
use crate::style::{pick, Paint, Prop, Recipe, Role, State, Style};
use crate::Theme;
use egui::{Response, Sense, Ui, Vec2, Widget};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    Default,
    Outline,
    Secondary,
    Ghost,
    Destructive,
    Link,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonSize {
    Xs,
    Sm,
    Default,
    Lg,
    /// Square, icon only; the text is the accessible name.
    Icon,
    IconXs,
    IconSm,
    IconLg,
}

pub struct Button {
    text: String,
    variant: ButtonVariant,
    size: ButtonSize,
    enabled: bool,
    icon: Option<Icon>,
    icon_end: Option<Icon>,
    expanded: bool,
    invalid: bool,
    has_popup: bool,
    extra: Option<fn(&Style) -> &Recipe>,
}

impl Button {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: ButtonVariant::Default,
            size: ButtonSize::Default,
            enabled: true,
            icon: None,
            icon_end: None,
            expanded: false,
            invalid: false,
            has_popup: false,
            extra: None,
        }
    }
    pub fn variant(mut self, v: ButtonVariant) -> Self {
        self.variant = v;
        self
    }
    pub fn size(mut self, s: ButtonSize) -> Self {
        self.size = s;
        self
    }
    /// Disabled buttons do not sense clicks, cannot take focus, show no
    /// hover or press styling, and paint at the style's disabled opacity.
    pub fn enabled(mut self, yes: bool) -> Self {
        self.enabled = yes;
        self
    }
    /// An icon before the text (shadcn `<Icon data-icon="inline-start" />`).
    /// With an icon size (`ButtonSize::Icon`, …) only the icon is drawn and
    /// the text is the button's accessible name.
    pub fn icon(mut self, icon: &Icon) -> Self {
        self.icon = Some(*icon);
        self
    }
    /// An icon after the text (shadcn `<Icon data-icon="inline-end" />`).
    pub fn icon_end(mut self, icon: &Icon) -> Self {
        self.icon_end = Some(*icon);
        self
    }
    /// `aria-expanded`: the menu or popover this button opens is open.
    pub fn expanded(mut self, yes: bool) -> Self {
        self.expanded = yes;
        self
    }
    /// `aria-invalid`: the destructive ring and border.
    pub fn invalid(mut self, yes: bool) -> Self {
        self.invalid = yes;
        self
    }
    /// `aria-haspopup`: the button opens a popup, so it does not nudge down
    /// when pressed.
    pub fn has_popup(mut self, yes: bool) -> Self {
        self.has_popup = yes;
        self
    }

    /// A recipe stacked over the button's own, for a part that puts its
    /// class on a button (`cn-dialog-close`): it wins ties of equal rank,
    /// so its base value still loses to a button rule for a matching state.
    pub(crate) fn extra_layer(mut self, layer: fn(&Style) -> &Recipe) -> Self {
        self.extra = Some(layer);
        self
    }

    /// The recipe stack and how many of its layers are in use.
    fn layers<'s>(&self, s: &'s Style) -> ([&'s Recipe; 4], usize) {
        let v = match self.variant {
            ButtonVariant::Default => &s.button_variant_default,
            ButtonVariant::Outline => &s.button_variant_outline,
            ButtonVariant::Secondary => &s.button_variant_secondary,
            ButtonVariant::Ghost => &s.button_variant_ghost,
            ButtonVariant::Destructive => &s.button_variant_destructive,
            ButtonVariant::Link => &s.button_variant_link,
        };
        let z = match self.size {
            ButtonSize::Xs => &s.button_size_xs,
            ButtonSize::Sm => &s.button_size_sm,
            ButtonSize::Default => &s.button_size_default,
            ButtonSize::Lg => &s.button_size_lg,
            ButtonSize::Icon => &s.button_size_icon,
            ButtonSize::IconXs => &s.button_size_icon_xs,
            ButtonSize::IconSm => &s.button_size_icon_sm,
            ButtonSize::IconLg => &s.button_size_icon_lg,
        };
        match self.extra {
            Some(extra) => ([&*s.button, v, z, extra(s)], 4),
            None => ([&*s.button, v, z, z], 3),
        }
    }

    /// The icon of an icon size (`ButtonSize::Icon`, …): drawn alone, the
    /// text is not. `None` for text sizes. An icon size without an icon is a
    /// caller error (debug builds panic); release builds fall back to a
    /// button as wide as its text.
    fn icon_only(&self) -> Option<Icon> {
        let square = matches!(
            self.size,
            ButtonSize::Icon | ButtonSize::IconXs | ButtonSize::IconSm | ButtonSize::IconLg
        );
        let icon = self.icon.or(self.icon_end);
        debug_assert!(
            !square || icon.is_some(),
            "Button {:?}: an icon size needs .icon(..)",
            self.text
        );
        if square {
            icon
        } else {
            None
        }
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let (layers, n) = self.layers(&t.style);
        let l = &layers[..n];
        let enabled = self.enabled && ui.is_enabled();
        let context = State::NONE
            .with(State::ICON_START, self.icon.is_some())
            .with(State::ICON_END, self.icon_end.is_some())
            .with(State::EXPANDED, self.expanded)
            .with(State::INVALID, self.invalid)
            .with(State::HAS_POPUP, self.has_popup);
        // Geometry never depends on hover or press: resolve it before allocating.
        let g = t.state(ui.ctx(), context);
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(l, f, g).unwrap_or(0.0);
        let (pad_l, pad_r, gap, border_w) = (
            get(|r| &r.pad_left),
            get(|r| &r.pad_right),
            get(|r| &r.gap),
            line_width(get(|r| &r.border_width), ui.ctx().pixels_per_point()),
        );
        let icon_px = pick(l, |r| &r.icon_size, g).unwrap_or(t.metrics.icon);
        // The text colour depends on the state, known only after allocating:
        // lay out in egui's placeholder colour, paint in the final one.
        let look = TextLook::resolve(ui, l, g);
        let icon_only = self.icon_only();
        let row = match icon_only {
            Some(icon) => AtomRow::new(ui, "").icons(Some(icon), None, icon_px),
            None => AtomRow::new(ui, self.text.as_str()).icons(self.icon, self.icon_end, icon_px),
        };
        let (pad_t, pad_b) = (get(|r| &r.pad_top), get(|r| &r.pad_bottom));
        let row = row
            .gap(gap)
            .inset(
                pad_l + border_w,
                pad_r + border_w,
                pad_t + border_w,
                pad_b + border_w,
            )
            .lay_out(ui, &look);
        // No height: CSS's initial `auto`, the content's height.
        let height = pick(l, |r| &r.height, g).unwrap_or(row.size().y);
        let width = if icon_only.is_some() {
            pick(l, |r| &r.width, g).unwrap_or(height)
        } else {
            row.size().x
        };
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(width, height), sense);
        pressable(&mut resp, ActivationKeys::Button);
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &self.text)
        });
        if !ui.is_rect_visible(rect) {
            return resp;
        }

        let st = interaction_state(&t, &resp, context.with(State::DISABLED, !enabled));
        let painter = part_painter(ui, &[l], st);
        let paint = |f: fn(&Recipe) -> &Prop<Paint>| pick(l, f, st).map(|p| t.paint(p));
        let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground; upstream sets no text colour
        let fg = paint(|r| &r.text).unwrap_or(inherited);
        let rect = shift(
            rect,
            pick(l, |r| &r.shift_x, st),
            pick(l, |r| &r.shift_y, st),
        );
        RecipeBox::resolve(ui.ctx(), &t, l, st).paint(&painter, rect);
        // A state's `underline` (the link variant on hover) lays out again.
        row.paint(ui, &painter, rect, &TextLook::resolve(ui, l, st), fg);
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Paint, Role, State, Style};

    fn layers(s: &Style, v: ButtonVariant, z: ButtonSize) -> Vec<&Recipe> {
        let (l, n) = Button::new("x").variant(v).size(z).layers(s);
        l[..n].to_vec()
    }

    #[test]
    fn nova_icon_start_tightens_left_padding() {
        let s = Style::nova();
        let l = layers(&s, ButtonVariant::Default, ButtonSize::Default);
        assert_eq!(pick(&l, |r| &r.pad_left, State::NONE), Some(10.0));
        assert_eq!(pick(&l, |r| &r.pad_left, State::ICON_START), Some(8.0));
    }

    #[test]
    fn outline_hover_in_dark_uses_input_tint() {
        let s = Style::nova();
        let l = layers(&s, ButtonVariant::Outline, ButtonSize::Default);
        assert_eq!(
            pick(&l, |r| &r.bg, State::DARK | State::HOVER),
            Some(Paint::Role(Role::Input, 0.5))
        );
        assert_eq!(
            pick(&l, |r| &r.bg, State::HOVER),
            Some(Paint::Role(Role::Muted, 1.0))
        );
    }

    #[test]
    fn press_nudge_only_without_popup() {
        let s = Style::vega();
        let l = layers(&s, ButtonVariant::Default, ButtonSize::Default);
        assert!(pick(&l, |r| &r.shift_y, State::ACTIVE).is_some());
        assert!(pick(&l, |r| &r.shift_y, State::ACTIVE | State::HAS_POPUP).is_none());
    }
}
