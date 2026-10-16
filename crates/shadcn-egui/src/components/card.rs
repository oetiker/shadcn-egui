//! shadcn Card, ported from shadcn-svelte `ui/card/*.svelte` at
//! `tools/stylegen/PIN`. Padding,
//! gap, radius, ring, shadow, the footer strip and the text of the title and
//! description come from the style's `cn-card*` recipes.
//!
//! The card is a column of header, content and footer, the `cn-card` `gap`
//! apart. egui has no flex column, so [`Card::show`] lays the sections out
//! by hand: the card paints no padding on the sides of its sections, each
//! section insets itself, and the footer strip can therefore reach the
//! card's edges. Inside [`card_content`], only vega spaces the children
//! (`cn-card-content` `gap`); see its docs.
//!
//! Header, content and footer all inset their children by the same side
//! padding in every style (`px-(--card-spacing)`), so [`Card::show`] insets
//! the body once and [`card_header`] adds nothing of its own on the sides.
//!
//! `cn-card-header`'s `rounded-*` (vega `rounded-t-xl`, lyra `rounded-none`)
//! is not read: the header paints no fill or border in any style, so its
//! corners have nothing to shape.

use crate::components::shared::{faded, opacity, ring};
use crate::style::{pick, Recipe, Role, State, TextSize, Weight};
use crate::theme::heading_family_for;
use crate::Theme;
use egui::{
    Align, CornerRadius, Id, Layout, Rect, Response, RichText, Sense, Shape, Stroke, Ui, UiBuilder,
    WidgetInfo, WidgetType,
};

/// `data-size` of a card: `default` or `sm`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardSize {
    #[default]
    Default,
    Sm,
}

impl CardSize {
    fn state(self) -> State {
        match self {
            CardSize::Default => State::SIZE_DEFAULT,
            CardSize::Sm => State::SIZE_SM,
        }
    }
}

/// The size of the card being laid out; `card_title` reads it
/// (`group-data-[size=sm]/card`).
fn size_key() -> Id {
    Id::new("shadcn_egui::card::size")
}

fn current_size(ui: &Ui) -> CardSize {
    ui.ctx()
        .data(|d| d.get_temp::<CardSize>(size_key()))
        .unwrap_or_default()
}

fn len(layers: &[&Recipe], f: fn(&Recipe) -> &crate::style::Prop<f32>, st: State) -> f32 {
    pick(layers, f, st).unwrap_or(0.0)
}

type FooterFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A shadcn card: `Card::new().size(..).footer(..).show(ui, |ui| ..)`.
pub struct Card<'a> {
    size: CardSize,
    footer: Option<FooterFn<'a>>,
}

impl Default for Card<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Card<'a> {
    pub fn new() -> Self {
        Self {
            size: CardSize::Default,
            footer: None,
        }
    }
    /// `data-size`: `Default` or the tighter `Sm`.
    pub fn size(mut self, size: CardSize) -> Self {
        self.size = size;
        self
    }
    /// A `card-footer` row: its children sit in a row, centred on the
    /// cross axis. In a style with a footer strip (nova) the strip reaches
    /// the card's edges.
    pub fn footer(mut self, footer: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.footer = Some(Box::new(footer));
        self
    }

    /// Show the card across the available width. `add` fills the body:
    /// [`card_header`] and [`card_content`] sections, `gap` apart.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
        let t = Theme::current(ui.ctx());
        let has_footer = self.footer.is_some();
        let st = t.state(self.size.state().with(State::HAS_FOOTER, has_footer));
        let card = [&*t.style.card];
        let content = [&*t.style.card_content];
        let foot = [&*t.style.card_footer];

        let (pad_top, pad_bottom, gap) = (
            len(&card, |r| &r.pad_top, st),
            len(&card, |r| &r.pad_bottom, st),
            len(&card, |r| &r.gap, st),
        );
        let (side_l, side_r) = (
            len(&content, |r| &r.pad_left, st),
            len(&content, |r| &r.pad_right, st),
        );
        let radius = pick(&card, |r| &r.radius, st).map_or(0.0, |r| t.radius_px(r, f32::INFINITY));
        let fill = pick(&card, |r| &r.bg, st).map(|p| t.paint(p));
        let fg = pick(&card, |r| &r.text, st).map(|p| t.paint(p));
        let text_px = t.text_px(pick(&card, |r| &r.text_size, st).unwrap_or(TextSize::Sm));
        let ring_width = len(&card, |r| &r.ring_width, st);
        let ring_color = pick(&card, |r| &r.ring_color, st).map(|p| t.paint(p));
        let shadow = pick(&card, |r| &r.shadow, st).map(|s| t.shadow(s));
        // Header, content and footer sit in the card.
        let alpha = opacity(&[&card[..]], st);
        let mut painter = ui.painter().clone();
        painter.multiply_opacity(alpha);

        let outer_w = ui.available_width();
        let origin = ui.cursor().min;
        let back = ui.painter().add(Shape::Noop);

        // Header and content.
        let body_rect = Rect::from_min_max(
            egui::pos2(origin.x + side_l, origin.y + pad_top),
            egui::pos2(
                origin.x + outer_w - side_r,
                origin.y + pad_top + ui.available_height().max(0.0),
            ),
        );
        let mut body = ui.new_child(
            UiBuilder::new()
                .max_rect(body_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        body.spacing_mut().item_spacing.y = gap;
        body.multiply_opacity(alpha);
        if let Some(fg) = fg {
            body.visuals_mut().override_text_color = Some(fg);
        }
        for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
            if let Some(font) = body.style_mut().text_styles.get_mut(&style) {
                font.size = text_px;
            }
        }
        let previous = body.ctx().data(|d| d.get_temp::<CardSize>(size_key()));
        body.ctx()
            .data_mut(|d| d.insert_temp(size_key(), self.size));
        let result = add(&mut body);
        body.ctx().data_mut(|d| match previous {
            Some(p) => {
                d.insert_temp(size_key(), p);
            }
            None => d.remove::<CardSize>(size_key()),
        });
        let mut bottom = body.min_rect().bottom().max(body_rect.top());

        // Footer.
        let mut footer_shapes = None;
        if let Some(footer) = self.footer {
            let (f_top, f_right, f_bottom, f_left) = (
                len(&foot, |r| &r.pad_top, st),
                len(&foot, |r| &r.pad_right, st),
                len(&foot, |r| &r.pad_bottom, st),
                len(&foot, |r| &r.pad_left, st),
            );
            let strip_top = bottom + gap;
            let strip_bg = ui.painter().add(Shape::Noop);
            let inner = Rect::from_min_max(
                egui::pos2(origin.x + f_left, strip_top + f_top),
                egui::pos2(
                    origin.x + outer_w - f_right,
                    strip_top + f_top + ui.spacing().interact_size.y,
                ),
            );
            // A row as high as a control, like `ui.horizontal`.
            let mut row = ui.new_child(
                UiBuilder::new()
                    .max_rect(inner)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            row.spacing_mut().item_spacing.x = len(&foot, |r| &r.gap, st);
            row.multiply_opacity(opacity(&[&foot[..], &card[..]], st));
            if let Some(fg) = fg {
                row.visuals_mut().override_text_color = Some(fg);
            }
            footer(&mut row);
            let strip_bottom = row.min_rect().bottom().max(inner.top()) + f_bottom;
            footer_shapes = Some((strip_bg, strip_top, strip_bottom));
            bottom = strip_bottom;
        }
        let card_bottom = bottom + pad_bottom;
        let outer = Rect::from_min_max(origin, egui::pos2(origin.x + outer_w, card_bottom));

        // Paint behind the sections: shadow, fill, footer strip, then the ring.
        let corner = CornerRadius::same(radius.round() as u8);
        let mut shapes = Vec::new();
        if let Some(s) = shadow {
            shapes.push(Shape::from(s.as_shape(outer, corner)));
        }
        if let Some(fill) = fill {
            shapes.push(Shape::rect_filled(outer, corner, fill));
        }
        painter.set(back, Shape::Vec(shapes));
        if let Some((idx, top, bottom)) = footer_shapes {
            let strip = Rect::from_min_max(
                egui::pos2(outer.left(), top),
                egui::pos2(outer.right(), bottom),
            );
            let r_bottom = pick(&foot, |r| &r.radius_bottom, st)
                .map_or(0.0, |r| t.radius_px(r, f32::INFINITY));
            let corner = CornerRadius {
                nw: 0,
                ne: 0,
                sw: r_bottom.round() as u8,
                se: r_bottom.round() as u8,
            };
            let mut strip_shapes = Vec::new();
            if let Some(bg) = pick(&foot, |r| &r.bg, st).map(|p| t.paint(p)) {
                strip_shapes.push(Shape::rect_filled(strip, corner, bg));
            }
            let line = len(&foot, |r| &r.border_top_width, st);
            if line > 0.0 {
                // `border-t` takes the default border colour.
                let base_border = t.role(Role::Border); // behaviour-ok: shadcn base layer `* { @apply border-border }`
                let color =
                    pick(&foot, |r| &r.border_color, st).map_or(base_border, |p| t.paint(p));
                strip_shapes.push(Shape::line_segment(
                    [strip.left_top(), strip.right_top()],
                    Stroke::new(line, color),
                ));
            }
            let mut strip_painter = painter.clone();
            strip_painter.multiply_opacity(opacity(&[&foot[..]], st));
            strip_painter.set(idx, Shape::Vec(strip_shapes));
        }
        // `ring-1` lies outside the box.
        if let Some(color) = ring_color {
            ring(&painter, outer, radius, ring_width, color);
        }

        let response = ui.allocate_rect(outer, Sense::hover());
        response.widget_info(|| WidgetInfo::new(WidgetType::Panel));
        result
    }
}

/// `card-content` section. Its children are the style's `cn-card-content`
/// `gap` apart, which only vega sets (12 px); in nova, maia, lyra, mira,
/// luma and rhea they touch. Wrap them in [`crate::layout::vstack`] for the
/// design's spacing.
pub fn card_content<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let t = Theme::current(ui.ctx());
    let st = t.state(current_size(ui).state());
    let l = [&*t.style.card_content];
    let gap = len(&l, |r| &r.gap, st);
    ui.scope(|ui| {
        ui.multiply_opacity(opacity(&[&l[..]], st));
        ui.spacing_mut().item_spacing.y = gap;
        add(ui)
    })
    .inner
}

/// `card-header` with its title and description.
pub fn card_header(ui: &mut Ui, title: &str, description: &str) {
    faded(ui, part_opacity(ui, |s| &s.card_header), |ui| {
        header_column(ui, title, description)
    });
}

/// The resolved `opacity` of one card part, at the current card's size.
fn part_opacity(ui: &Ui, part: fn(&crate::style::Style) -> &Recipe) -> f32 {
    let t = Theme::current(ui.ctx());
    opacity(&[&[part(&t.style)][..]], t.state(current_size(ui).state()))
}

/// `card-header` with a `card-action` to the right of the title and
/// description, aligned to the top.
pub fn card_header_with_action(
    ui: &mut Ui,
    title: &str,
    description: &str,
    action: impl FnOnce(&mut Ui),
) {
    // The action sits in the header, beside the title and description.
    let header = part_opacity(ui, |s| &s.card_header);
    let action_fade = part_opacity(ui, |s| &s.card_action);
    faded(ui, header, |ui| {
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            faded(ui, action_fade, action);
            let width = ui.available_width();
            ui.allocate_ui_with_layout(
                egui::vec2(width, 0.0),
                Layout::top_down(Align::Min),
                |ui| header_column(ui, title, description),
            );
        })
    });
}

fn header_column(ui: &mut Ui, title: &str, description: &str) {
    let t = Theme::current(ui.ctx());
    let gap = len(
        &[&*t.style.card_header],
        |r| &r.gap,
        t.state(current_size(ui).state()),
    );
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = gap;
        card_title(ui, title);
        card_description(ui, description);
    });
}

/// `card-title`: size, weight and line height of the style.
pub fn card_title(ui: &mut Ui, text: &str) -> Response {
    let t = Theme::current(ui.ctx());
    let st = t.state(current_size(ui).state());
    let l = [&*t.style.card_title];
    let size = pick(&l, |r| &r.text_size, st).unwrap_or(TextSize::Base);
    let px = t.text_px(size);
    let line = pick(&l, |r| &r.line_height, st).map_or_else(|| t.line_px(size), |k| k * px);
    let weight = pick(&l, |r| &r.weight, st).unwrap_or(Weight::Normal);
    // upstream: card-title carries `cn-font-heading`
    faded(ui, opacity(&[&l[..]], st), |ui| {
        ui.label(
            RichText::new(text)
                .family(heading_family_for(ui.ctx(), weight))
                .size(px)
                .line_height(Some(line)),
        )
    })
}

/// `card-description`: muted text at the style's size.
pub fn card_description(ui: &mut Ui, text: &str) -> Response {
    let t = Theme::current(ui.ctx());
    let st = t.state(current_size(ui).state());
    let l = [&*t.style.card_description];
    let size = pick(&l, |r| &r.text_size, st).unwrap_or(TextSize::Sm);
    let line =
        pick(&l, |r| &r.line_height, st).map_or_else(|| t.line_px(size), |k| k * t.text_px(size));
    let mut text = RichText::new(text)
        .size(t.text_px(size))
        .line_height(Some(line));
    if let Some(c) = pick(&l, |r| &r.text, st).map(|p| t.paint(p)) {
        text = text.color(c);
    }
    faded(ui, opacity(&[&l[..]], st), |ui| ui.label(text))
}
