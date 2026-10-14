//! shadcn Badge, ported from shadcn-svelte `ui/badge/badge.svelte` at
//! `tools/stylegen/PIN`. Every value comes from the style's badge
//! recipes: `cn-badge` and `cn-badge-variant-*`, stacked in that order.

use crate::components::icon::{paint_icon, Icon};
use crate::components::shared::{interaction_state, paint_box, ring};
use crate::style::{pick, Paint, Prop, Recipe, Role, State, Style, TextSize, Weight};
use crate::theme::family_for;
use crate::Theme;
use egui::{Response, Sense, Stroke, Ui, Vec2, Widget};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeVariant {
    Default,
    Secondary,
    Destructive,
    Outline,
    Ghost,
    Link,
}

pub struct Badge {
    text: String,
    variant: BadgeVariant,
    icon: Option<&'static Icon>,
    icon_end: Option<&'static Icon>,
}

impl Badge {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            variant: BadgeVariant::Default,
            icon: None,
            icon_end: None,
        }
    }
    pub fn variant(mut self, v: BadgeVariant) -> Self {
        self.variant = v;
        self
    }
    /// An icon before the text (shadcn `<Icon data-icon="inline-start" />`).
    pub fn icon(mut self, icon: &'static Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// An icon after the text (shadcn `<Icon data-icon="inline-end" />`).
    pub fn icon_end(mut self, icon: &'static Icon) -> Self {
        self.icon_end = Some(icon);
        self
    }

    fn layers<'s>(&self, s: &'s Style) -> [&'s Recipe; 2] {
        let v = match self.variant {
            BadgeVariant::Default => &s.badge_variant_default,
            BadgeVariant::Secondary => &s.badge_variant_secondary,
            BadgeVariant::Destructive => &s.badge_variant_destructive,
            BadgeVariant::Outline => &s.badge_variant_outline,
            BadgeVariant::Ghost => &s.badge_variant_ghost,
            BadgeVariant::Link => &s.badge_variant_link,
        };
        [&*s.badge, v]
    }
}

/// A badge with the default variant order of the shadcn docs.
pub fn badge(ui: &mut Ui, text: &str, variant: BadgeVariant) -> Response {
    ui.add(Badge::new(text).variant(variant))
}

impl Widget for Badge {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let l = self.layers(&t.style);
        let enabled = ui.is_enabled();
        // The `[a]:hover` rules are ignored (a badge is not a link in egui),
        // so only Ghost and Link react to the pointer. A badge is never focused.
        let hoverable = matches!(self.variant, BadgeVariant::Ghost | BadgeVariant::Link);
        let context = State::NONE
            .with(State::ICON_START, self.icon.is_some())
            .with(State::ICON_END, self.icon_end.is_some());
        // Geometry never depends on hover: resolve it before allocating.
        let g = t.state(context);
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(&l, f, g).unwrap_or(0.0);
        let height = get(|r| &r.height);
        let (pad_l, pad_r, gap, border_w) = (
            get(|r| &r.pad_left),
            get(|r| &r.pad_right),
            get(|r| &r.gap),
            get(|r| &r.border_width),
        );
        let icon_px = pick(&l, |r| &r.icon_size, g).unwrap_or(t.metrics.icon);
        let text_px = t.text_px(pick(&l, |r| &r.text_size, g).unwrap_or(TextSize::Xs));
        let weight = pick(&l, |r| &r.weight, g).unwrap_or(Weight::Normal);
        let font = egui::FontId::new(text_px, family_for(ui.ctx(), weight));
        // The text colour depends on the state, known only after allocating:
        // lay out in egui's placeholder colour, paint in the final one.
        let painter = ui.painter().clone();
        let layout = |underline: Stroke| {
            let mut job = egui::text::LayoutJob::simple_singleline(
                self.text.clone(),
                font.clone(),
                egui::Color32::PLACEHOLDER,
            );
            job.sections[0].format.underline = underline;
            painter.layout_job(job)
        };
        let galley = layout(Stroke::NONE);
        let icons = u8::from(self.icon.is_some()) + u8::from(self.icon_end.is_some());
        let content_w = galley.size().x + f32::from(icons) * (icon_px + gap);
        let width = content_w + pad_l + pad_r + border_w * 2.0;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, enabled, &self.text)
        });
        if !ui.is_rect_visible(rect) {
            return resp;
        }

        let st = interaction_state(&t, &resp, enabled, context);
        let st = if hoverable {
            st
        } else {
            State(st.0 & !State::HOVER.0)
        };
        let paint = |f: fn(&Recipe) -> &Prop<Paint>| pick(&l, f, st).map(|p| t.paint(p));
        let bg = paint(|r| &r.bg);
        let fg = paint(|r| &r.text).unwrap_or_else(|| t.role(Role::Foreground));
        let border = paint(|r| &r.border_color);
        let ring_color = paint(|r| &r.ring_color);
        let ring_w = pick(&l, |r| &r.ring_width, st).unwrap_or(0.0);
        let radius =
            pick(&l, |r| &r.radius, g).map_or(0.0, |r| t.radius_px(r, rect.height() / 2.0));

        let clip_padding = pick(&l, |r| &r.bg_clip_padding, st) == Some(true);
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
        if let Some(icon) = self.icon {
            draw_icon(icon, x);
            x += icon_px + gap;
        }
        let galley = if pick(&l, |r| &r.underline, st) == Some(true) {
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
            draw_icon(icon, x + text_w + gap);
        }
        resp
    }
}
