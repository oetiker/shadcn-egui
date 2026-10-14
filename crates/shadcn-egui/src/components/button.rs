//! shadcn Button, ported from shadcn-svelte `ui/button/button.svelte` at
//! `tools/stylegen/PIN`. Every value comes from the style's button
//! recipes: `cn-button`, `cn-button-variant-*` and `cn-button-size-*`,
//! stacked in that order.

use crate::components::icon::{paint_icon, Icon};
use crate::components::shared::{
    interaction_state, line_box, opacity, paint_box, ring, shift, style_painter,
};
use crate::style::{pick, Paint, Prop, Recipe, Role, State, Style, TextSize, Weight};
use crate::theme::family_for;
use crate::Theme;
use egui::{Color32, Response, Sense, Stroke, Ui, Vec2, Widget};

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
        let g = t.state(context);
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(l, f, g).unwrap_or(0.0);
        let (pad_l, pad_r, gap, border_w) = (
            get(|r| &r.pad_left),
            get(|r| &r.pad_right),
            get(|r| &r.gap),
            get(|r| &r.border_width),
        );
        let icon_px = pick(l, |r| &r.icon_size, g).unwrap_or(t.metrics.icon);
        let text_size = pick(l, |r| &r.text_size, g).unwrap_or(TextSize::Sm);
        let text_px = t.text_px(text_size);
        let weight = pick(l, |r| &r.weight, g).unwrap_or(Weight::Normal);
        let font = egui::FontId::new(text_px, family_for(ui.ctx(), weight));
        let fonts = ui.painter().clone();
        // The text colour depends on the state, known only after allocating:
        // lay out in egui's placeholder colour, paint in the final one.
        let layout = |underline: Stroke| {
            let mut job = egui::text::LayoutJob::simple_singleline(
                self.text.clone(),
                font.clone(),
                Color32::PLACEHOLDER,
            );
            job.sections[0].format.underline = underline;
            fonts.layout_job(job)
        };
        let galley = layout(Stroke::NONE);
        let icon_only = self.icon_only();
        let has_icon = icon_only.is_some() || self.icon.is_some() || self.icon_end.is_some();
        // No height: CSS's initial `auto`, the content's height.
        let content_h = if icon_only.is_some() {
            icon_px
        } else {
            let line = line_box(&t, l, g, text_size).max(galley.size().y);
            if has_icon {
                line.max(icon_px)
            } else {
                line
            }
        };
        let height = pick(l, |r| &r.height, g).unwrap_or_else(|| {
            content_h + get(|r| &r.pad_top) + get(|r| &r.pad_bottom) + border_w * 2.0
        });
        let content_w = if icon_only.is_some() {
            icon_px
        } else {
            let icons = u8::from(self.icon.is_some()) + u8::from(self.icon_end.is_some());
            galley.size().x + f32::from(icons) * (icon_px + gap)
        };
        let width = if icon_only.is_some() {
            pick(l, |r| &r.width, g).unwrap_or(height)
        } else {
            content_w + pad_l + pad_r + border_w * 2.0
        };
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), sense);
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &self.text)
        });
        if !ui.is_rect_visible(rect) {
            return resp;
        }

        let st = interaction_state(&t, &resp, enabled, context);
        let painter = style_painter(ui, opacity(&[l], st));
        let paint = |f: fn(&Recipe) -> &Prop<Paint>| pick(l, f, st).map(|p| t.paint(p));
        let bg = paint(|r| &r.bg);
        let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground; upstream sets no text colour
        let fg = paint(|r| &r.text).unwrap_or(inherited);
        let border = paint(|r| &r.border_color);
        let ring_color = paint(|r| &r.ring_color);
        let ring_w = pick(l, |r| &r.ring_width, st).unwrap_or(0.0);
        let rect = shift(
            rect,
            pick(l, |r| &r.shift_x, st),
            pick(l, |r| &r.shift_y, st),
        );
        let radius = pick(l, |r| &r.radius, g).map_or(0.0, |r| t.radius_px(r, rect.height() / 2.0));

        if let Some(s) = pick(l, |r| &r.shadow, st) {
            painter.add(t.shadow(s).as_shape(rect, radius));
        }
        let clip_padding = pick(l, |r| &r.bg_clip_padding, st) == Some(true);
        paint_box(
            &painter,
            rect,
            radius,
            bg,
            border.map(|b| Stroke::new(border_w, b)),
            clip_padding,
        );
        if let Some(c) = ring_color {
            ring(&painter, rect, radius, ring_w, c);
        }

        // Content is centred; unequal icon-side padding shifts it.
        let mut x = rect.center().x - content_w / 2.0 + (pad_l - pad_r) / 2.0;
        let draw_icon = |icon: &Icon, x: f32| {
            let r = egui::Rect::from_min_size(
                egui::pos2(x, rect.center().y - icon_px / 2.0),
                Vec2::splat(icon_px),
            );
            paint_icon(&painter, r, icon, fg, t.metrics.icon_stroke);
        };
        if let Some(icon) = icon_only {
            draw_icon(&icon, x);
            return resp;
        }
        if let Some(icon) = self.icon {
            draw_icon(&icon, x);
            x += icon_px + gap;
        }
        let galley = if pick(l, |r| &r.underline, st) == Some(true) {
            layout(Stroke::new(t.metrics.border, fg))
        } else {
            galley
        };
        let text_w = galley.size().x;
        painter.galley(
            egui::pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            fg,
        );
        if let Some(icon) = self.icon_end {
            draw_icon(&icon, x + text_w + gap);
        }
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
