//! shadcn Badge, ported from shadcn-svelte `ui/badge/badge.svelte` at
//! `tools/stylegen/PIN`. Every value comes from the style's badge
//! recipes: `cn-badge` and `cn-badge-variant-*`, stacked in that order.

use crate::components::atom_row::AtomRow;
use crate::components::icon::Icon;
use crate::components::shared::{interaction_state, line_width, part_painter, RecipeBox};
use crate::components::text::TextLook;
use crate::style::{pick, Paint, Prop, Recipe, Role, State, Style};
use crate::Theme;
use egui::{Response, Sense, Ui, Vec2, Widget};

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
    icon: Option<Icon>,
    icon_end: Option<Icon>,
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
    pub fn icon(mut self, icon: &Icon) -> Self {
        self.icon = Some(*icon);
        self
    }
    /// An icon after the text (shadcn `<Icon data-icon="inline-end" />`).
    pub fn icon_end(mut self, icon: &Icon) -> Self {
        self.icon_end = Some(*icon);
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
        let g = t.state(ui.ctx(), context);
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(&l, f, g).unwrap_or(0.0);
        let (pad_l, pad_r, gap, border_w) = (
            get(|r| &r.pad_left),
            get(|r| &r.pad_right),
            get(|r| &r.gap),
            line_width(get(|r| &r.border_width), ui.ctx().pixels_per_point()),
        );
        let icon_px = pick(&l, |r| &r.icon_size, g).unwrap_or(t.metrics.icon);
        // The text colour depends on the state, known only after allocating:
        // lay out in egui's placeholder colour, paint in the final one.
        let look = TextLook::resolve(ui, &l, g);
        let row = AtomRow::new(ui, self.text.as_str())
            .icons(self.icon, self.icon_end, icon_px)
            .gap(gap)
            .inset(
                pad_l + border_w,
                pad_r + border_w,
                get(|r| &r.pad_top) + border_w,
                get(|r| &r.pad_bottom) + border_w,
            )
            .lay_out(ui, &look);
        // No height: CSS's initial `auto`, the content's height.
        let height = pick(&l, |r| &r.height, g).unwrap_or(row.size().y);
        let width = row.size().x;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        resp.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Label, enabled, &self.text)
        });
        if !ui.is_rect_visible(rect) {
            return resp;
        }

        let st = interaction_state(&t, &resp, context);
        let st = if hoverable {
            st
        } else {
            st.without(State::HOVER)
        };
        let painter = part_painter(ui, &[&l[..]], st);
        let paint = |f: fn(&Recipe) -> &Prop<Paint>| pick(&l, f, st).map(|p| t.paint(p));
        let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground; upstream sets no text colour
        let fg = paint(|r| &r.text).unwrap_or(inherited);
        RecipeBox::resolve(ui.ctx(), &t, &l, st).paint(&painter, rect);

        // A state's `underline` (the link variant on hover) lays out again.
        row.paint(ui, &painter, rect, &TextLook::resolve(ui, &l, st), fg);
        resp
    }
}
