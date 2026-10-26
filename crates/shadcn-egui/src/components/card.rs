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

use crate::components::context;
use crate::components::shared::{content_ui, part_painter, part_ui, RecipeBox};
use crate::components::text::{content_scope, TextLook};
use crate::style::app::{default_variant, state_size, ResolvedPair};
use crate::style::{pick, Recipe, State};
use crate::{layout, Theme};
use egui::{
    Align, Layout, Rect, Response, Sense, Shape, TextWrapMode, Ui, UiBuilder, WidgetInfo,
    WidgetType,
};

/// `data-size` of a card: `default` or `sm`, or an app size
/// ([`CardSize::app`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum CardSize {
    #[default]
    Default,
    Sm,
    /// An app size: [`CardSize::app`].
    #[doc(hidden)]
    App(crate::style::AppVariant<CardSize>),
}

state_size!(CardSize, CardVariant);

default_variant!(
    /// The card's variant: upstream has none, so only app variants
    /// ([`CardVariant::app`]) differ from `Default`.
    CardVariant,
    /// The card's recipes an app variant's or app size's edit changes.
    CardRecipes {
        /// `data-slot="card"`: the surface.
        card,
        /// `data-slot="card-header"`.
        card_header,
        /// `data-slot="card-title"`.
        card_title,
        /// `data-slot="card-description"`.
        card_description,
        /// `data-slot="card-action"`.
        card_action,
        /// `data-slot="card-content"`.
        card_content,
        /// `data-slot="card-footer"`.
        card_footer,
    }
);

/// The context a card gives its parts: its variant and its size
/// (`group-data-[size=sm]/card`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct CardContext {
    variant: CardVariant,
    size: CardSize,
}

/// The context of the card around `ui`; outside a card, the default.
fn card_context(ui: &Ui) -> CardContext {
    context::find::<CardContext>(ui).unwrap_or_default()
}

/// The recipes of the card around `ui`.
fn card_recipes(ui: &Ui, t: &Theme) -> ResolvedPair<CardVariant, CardSize> {
    let cx = card_context(ui);
    t.resolve_pair(cx.variant, cx.size)
}

fn len(layers: &[&Recipe], f: fn(&Recipe) -> &crate::style::Prop<f32>, st: State) -> f32 {
    pick(layers, f, st).unwrap_or(0.0)
}

type FooterFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

/// A shadcn card: `Card::new().size(..).footer(..).show(ui, |ui| ..)`.
pub struct Card<'a> {
    size: CardSize,
    variant: CardVariant,
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
            variant: CardVariant::Default,
            footer: None,
        }
    }
    /// `data-size`: `Default` or the tighter `Sm`, or an app size
    /// ([`CardSize::app`]). The card's parts inside take it.
    pub fn size(mut self, size: impl Into<CardSize>) -> Self {
        self.size = size.into();
        self
    }
    /// An app variant ([`CardVariant::app`]). The card's parts inside take
    /// it.
    pub fn variant(mut self, v: impl Into<CardVariant>) -> Self {
        self.variant = v.into();
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
        let parts = CardContext {
            variant: self.variant,
            size: self.size,
        };
        let v = t.resolve_pair(self.variant, self.size);
        let st = t.state(
            ui.ctx(),
            v.bits()
                .with(State::HAS_FOOTER, has_footer)
                .with(State::DISABLED, !ui.is_enabled()),
        );
        let card = [v.part(&t.style, |s| &s.card, |r| &r.card)];
        let content = [v.part(&t.style, |s| &s.card_content, |r| &r.card_content)];
        let foot = [v.part(&t.style, |s| &s.card_footer, |r| &r.card_footer)];

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
        let mut body = ui.new_child(context::push(
            UiBuilder::new()
                .max_rect(body_rect)
                .layout(Layout::top_down(Align::Min)),
            parts,
        ));
        body.spacing_mut().item_spacing.y = gap;
        let result =
            content_scope(&mut body, &look, |ui| content_ui(ui, &[&card[..]], st, add)).inner;
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
            let mut row = ui.new_child(context::push(
                crate::layout::row_builder(inner, Layout::left_to_right(Align::Center)),
                parts,
            ));
            row.spacing_mut().item_spacing.x = len(&foot, |r| &r.gap, st);
            crate::layout::shrink_stacks(&mut row, |ui| {
                content_scope(ui, &look, |ui| {
                    content_ui(ui, &[&foot[..], &card[..]], st, footer)
                });
                ((), ui.min_rect().width())
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
            Shape::Vec(RecipeBox::resolve(ui.ctx(), &t, &card, st).shapes(ui.ctx(), outer)),
        );
        if let Some((idx, top, bottom)) = footer_shapes {
            let strip = Rect::from_min_max(
                egui::pos2(outer.left(), top),
                egui::pos2(outer.right(), bottom),
            );
            part_painter(ui, &[&foot[..], &card[..]], st).set(
                idx,
                Shape::Vec(RecipeBox::resolve(ui.ctx(), &t, &foot, st).shapes(ui.ctx(), strip)),
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
    let v = card_recipes(ui, &t);
    let st = card_state(ui, &v);
    let l = [v.part(&t.style, |s| &s.card_content, |r| &r.card_content)];
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
    let v = card_recipes(ui, &t);
    let header = [v.part(&t.style, |s| &s.card_header, |r| &r.card_header)];
    content_ui(ui, &[&header[..]], card_state(ui, &v), |ui| {
        header_column(ui, title, description)
    });
}

/// The state of a card part: the current card's variant and size bits,
/// disabled with the `Ui`.
fn card_state(ui: &Ui, v: &ResolvedPair<CardVariant, CardSize>) -> State {
    let t = Theme::current(ui.ctx());
    t.state(ui.ctx(), v.bits().with(State::DISABLED, !ui.is_enabled()))
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
    let v = card_recipes(ui, &t);
    let st = card_state(ui, &v);
    let header = [v.part(&t.style, |s| &s.card_header, |r| &r.card_header)];
    let card_action = [v.part(&t.style, |s| &s.card_action, |r| &r.card_action)];
    content_ui(ui, &[&header[..]], st, |ui| {
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            content_ui(ui, &[&card_action[..]], st, action);
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
    let v = card_recipes(ui, &t);
    let gap = len(
        &[v.part(&t.style, |s| &s.card_header, |r| &r.card_header)],
        |r| &r.gap,
        t.state(ui.ctx(), v.bits()),
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
    let v = card_recipes(ui, &t);
    let st = t.state(ui.ctx(), v.bits());
    let l = [v.part(&t.style, |s| &s.card_title, |r| &r.card_title)];
    // upstream: card-title carries `cn-font-heading`
    let look = TextLook::resolve(ui, &l, st).heading(true);
    part_ui(ui, &[&l[..]], st, |ui| {
        look.label(ui, text, TextWrapMode::Wrap)
    })
}

/// `card-description`: muted text at the style's size.
pub fn card_description(ui: &mut Ui, text: &str) -> Response {
    let t = Theme::current(ui.ctx());
    let v = card_recipes(ui, &t);
    let st = t.state(ui.ctx(), v.bits());
    let l = [v.part(&t.style, |s| &s.card_description, |r| &r.card_description)];
    let look = TextLook::resolve(ui, &l, st);
    part_ui(ui, &[&l[..]], st, |ui| {
        look.label(ui, text, TextWrapMode::Wrap)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{AppVariant, Paint, Role};

    /// An app variant's edit gets `CardRecipes` and changes only what it
    /// names; an app size edits the same copy after it.
    #[test]
    fn edit_compiles_and_changes_its_part() {
        const BRAND: AppVariant<CardVariant> = CardVariant::app("brand");
        const ROOMY: AppVariant<CardSize> = CardSize::app("roomy");
        let t = Theme::vega()
            .variant(BRAND, CardVariant::Default, |r: &mut CardRecipes, _| {
                r.card_title.text.set(Paint::Role(Role::Primary, 1.0))
            })
            .size(ROOMY, CardSize::Sm, |r, _| {
                r.card.gap.on(State::SIZE_SM, 40.0)
            });
        let v = t.resolve_pair(CardVariant::from(BRAND), CardSize::from(ROOMY));
        assert_eq!(v.bits(), State::SIZE_SM);
        let title = v.part(&t.style, |s| &s.card_title, |r| &r.card_title);
        assert_eq!(
            title.text.get(State::NONE),
            Some(Paint::Role(Role::Primary, 1.0))
        );
        let card = v.part(&t.style, |s| &s.card, |r| &r.card);
        assert_eq!(pick(&[card], |r| &r.gap, State::SIZE_SM), Some(40.0));
        let footer = v.part(&t.style, |s| &s.card_footer, |r| &r.card_footer);
        assert_eq!(footer, &*t.style.card_footer);
    }

    /// The card's parts take its variant from the context.
    #[test]
    fn parts_read_the_cards_variant() {
        const BRAND: AppVariant<CardVariant> = CardVariant::app("brand");
        let ctx = egui::Context::default();
        Theme::vega()
            .variant(BRAND, CardVariant::Default, |_, _| {})
            .apply(&ctx);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            Card::new().variant(BRAND).show(ui, |ui| {
                assert_eq!(card_context(ui).variant, CardVariant::from(BRAND));
                Card::new().show(ui, |ui| {
                    assert_eq!(card_context(ui).variant, CardVariant::Default)
                });
            });
        });
    }

    /// Each card part reads the size of its nearest card; outside a card
    /// it reads the default.
    #[test]
    fn parts_read_the_nearest_cards_size() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert_eq!(card_context(ui).size, CardSize::Default);
            Card::new()
                .size(CardSize::Sm)
                .footer(|ui| assert_eq!(card_context(ui).size, CardSize::Sm))
                .show(ui, |ui| {
                    assert_eq!(card_context(ui).size, CardSize::Sm);
                    Card::new().show(ui, |ui| {
                        assert_eq!(card_context(ui).size, CardSize::Default);
                        Card::new()
                            .size(CardSize::Sm)
                            .show(ui, |ui| assert_eq!(card_context(ui).size, CardSize::Sm));
                    });
                    assert_eq!(card_context(ui).size, CardSize::Sm);
                });
            assert_eq!(card_context(ui).size, CardSize::Default);
        });
    }
}
