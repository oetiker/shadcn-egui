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

use crate::components::shared::{content_ui, part_painter, part_ui, RecipeBox};
use crate::components::text::{content_scope, TextLook};
use crate::style::{pick, Recipe, State};
use crate::{layout, Theme};
use egui::{
    Align, Id, Layout, Rect, Response, Sense, Shape, TextWrapMode, Ui, UiBuilder, WidgetInfo,
    WidgetType,
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
        let st = t.state(
            ui.ctx(),
            self.size
                .state()
                .with(State::HAS_FOOTER, has_footer)
                .with(State::DISABLED, !ui.is_enabled()),
        );
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
        // The card's `text-*` and text colour reach its content.
        let look = TextLook::resolve(ui, &card, st);
        // Header, content and footer sit in the card.
        let painter = part_painter(ui, &[&card[..]], st);

        // `w-full` in a column: the room left, or the content minimum where
        // that is unbounded (conformance 3.1).
        let outer_w = layout::capped_width(ui, f32::INFINITY, side_l + side_r);
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
        let previous = body.ctx().data(|d| d.get_temp::<CardSize>(size_key()));
        body.ctx()
            .data_mut(|d| d.insert_temp(size_key(), self.size));
        let result =
            content_scope(&mut body, &look, |ui| content_ui(ui, &[&card[..]], st, add)).inner;
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
            let mut row = ui.new_child(crate::layout::row_builder(
                inner,
                Layout::left_to_right(Align::Center),
            ));
            row.spacing_mut().item_spacing.x = len(&foot, |r| &r.gap, st);
            content_scope(&mut row, &look, |ui| {
                content_ui(ui, &[&foot[..], &card[..]], st, footer)
            });
            let strip_bottom = row.min_rect().bottom().max(inner.top()) + f_bottom;
            footer_shapes = Some((strip_bg, strip_top, strip_bottom));
            bottom = strip_bottom;
        }
        let card_bottom = bottom + pad_bottom;
        let outer = Rect::from_min_max(origin, egui::pos2(origin.x + outer_w, card_bottom));

        // Paint behind the sections: the card's box, then the footer strip.
        painter.set(
            back,
            Shape::Vec(RecipeBox::resolve(&t, &card, st).shapes(ui.ctx(), outer)),
        );
        if let Some((idx, top, bottom)) = footer_shapes {
            let strip = Rect::from_min_max(
                egui::pos2(outer.left(), top),
                egui::pos2(outer.right(), bottom),
            );
            part_painter(ui, &[&foot[..], &card[..]], st).set(
                idx,
                Shape::Vec(RecipeBox::resolve(&t, &foot, st).shapes(ui.ctx(), strip)),
            );
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
    let st = card_state(ui);
    let l = [&*t.style.card_content];
    let gap = len(&l, |r| &r.gap, st);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = gap;
        content_ui(ui, &[&l[..]], st, add)
    })
    .inner
}

/// `card-header` with its title and description.
pub fn card_header(ui: &mut Ui, title: &str, description: &str) {
    let t = Theme::current(ui.ctx());
    content_ui(ui, &[&[&*t.style.card_header][..]], card_state(ui), |ui| {
        header_column(ui, title, description)
    });
}

/// The state of a card part: the current card's size, disabled with the `Ui`.
fn card_state(ui: &Ui) -> State {
    let t = Theme::current(ui.ctx());
    t.state(
        ui.ctx(),
        current_size(ui)
            .state()
            .with(State::DISABLED, !ui.is_enabled()),
    )
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
    let t = Theme::current(ui.ctx());
    let st = card_state(ui);
    content_ui(ui, &[&[&*t.style.card_header][..]], st, |ui| {
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            content_ui(ui, &[&[&*t.style.card_action][..]], st, action);
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
        t.state(ui.ctx(), current_size(ui).state()),
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
    let st = t.state(ui.ctx(), current_size(ui).state());
    let l = [&*t.style.card_title];
    // upstream: card-title carries `cn-font-heading`
    let look = TextLook::resolve(ui, &l, st).heading(true);
    part_ui(ui, &[&l[..]], st, |ui| {
        look.label(ui, text, TextWrapMode::Wrap)
    })
}

/// `card-description`: muted text at the style's size.
pub fn card_description(ui: &mut Ui, text: &str) -> Response {
    let t = Theme::current(ui.ctx());
    let st = t.state(ui.ctx(), current_size(ui).state());
    let l = [&*t.style.card_description];
    let look = TextLook::resolve(ui, &l, st);
    part_ui(ui, &[&l[..]], st, |ui| {
        look.label(ui, text, TextWrapMode::Wrap)
    })
}
