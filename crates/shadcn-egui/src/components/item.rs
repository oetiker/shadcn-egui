//! shadcn Item, ported from shadcn-svelte `ui/item/*.svelte` at
//! `tools/stylegen/PIN`. Padding, gap, radius, border, fill, the media's
//! size and the text of the title and description come from the style's
//! `cn-item*` recipes, stacked as upstream joins their classes (`cn-item`,
//! then `cn-item-variant-*`, then `cn-item-size-*`).
//!
//! Upstream lays the item out as one wrapping flex line: media, content
//! (`flex-1`), actions, with the header and the footer (`basis-full`) on
//! rows of their own above and below. [`Item::show`] builds that line by
//! hand: media and content come from setters and are drawn before the
//! closure, whose free content is the actions element (one row,
//! `items-center gap-2`) at the line's end. The item learns the header's
//! height, the media's and the actions' size and the line's height from
//! the previous pass; where one changes, the pass is discarded and redone
//! (conformance 3.2).

use crate::components::context;
use crate::components::icon::{paint_icon_rotated, Glyph};
use crate::components::registry::{self, Group};
use crate::components::shared::{
    interaction_state, line_width, part_painter, part_ui, pressable, shift, ActivationKeys,
    RecipeBox,
};
use crate::components::text::{content_scope, TextLook};
use crate::layout;
use crate::style::app::{ResolvedPair, Sealed};
use crate::style::{
    pick, pick_len, AppEnum, AppVariant, Recipe, SizeEnum, State, Style, VariantEnum,
};
use crate::Theme;
use egui::{
    Align, CursorIcon, Id, ImageSource, InnerResponse, Layout, Pos2, Rect, Response, Sense, Shape,
    TextWrapMode, Ui, UiBuilder, Vec2, Widget, WidgetInfo, WidgetText, WidgetType,
};
use std::hash::Hash;

/// shadcn `variant` of an item, or an app variant ([`ItemVariant::app`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ItemVariant {
    /// No fill, a transparent border.
    #[default]
    Default,
    /// A border in the border colour.
    Outline,
    /// A muted fill.
    Muted,
    /// An app variant: [`ItemVariant::app`].
    #[doc(hidden)]
    App(AppVariant<ItemVariant>),
}

impl ItemVariant {
    /// An app variant, defined in the theme ([`crate::Theme::variant`]).
    pub const fn app(name: &'static str) -> AppVariant<ItemVariant> {
        AppVariant::new(name)
    }

    /// The variant layer (`cn-item-variant-*`) of a built-in variant.
    fn layer(self, s: &Style) -> &Recipe {
        match self {
            ItemVariant::Default | ItemVariant::App(_) => &s.item_variant_default,
            ItemVariant::Outline => &s.item_variant_outline,
            ItemVariant::Muted => &s.item_variant_muted,
        }
    }
}

impl From<AppVariant<ItemVariant>> for ItemVariant {
    fn from(app: AppVariant<ItemVariant>) -> Self {
        ItemVariant::App(app)
    }
}

/// shadcn `size` of an item: `xs`, `sm` or `default`, or an app size
/// ([`ItemSize::app`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ItemSize {
    Xs,
    Sm,
    #[default]
    Default,
    /// An app size: [`ItemSize::app`].
    #[doc(hidden)]
    App(AppVariant<ItemSize>),
}

impl ItemSize {
    /// An app size, defined in the theme ([`crate::Theme::size`]).
    pub const fn app(name: &'static str) -> AppVariant<ItemSize> {
        AppVariant::new(name)
    }

    /// The size layer (`cn-item-size-*`) of a built-in size.
    fn layer(self, s: &Style) -> &Recipe {
        match self {
            ItemSize::Xs => &s.item_size_xs,
            ItemSize::Sm => &s.item_size_sm,
            ItemSize::Default | ItemSize::App(_) => &s.item_size_default,
        }
    }
}

impl From<AppVariant<ItemSize>> for ItemSize {
    fn from(app: AppVariant<ItemSize>) -> Self {
        ItemSize::App(app)
    }
}

/// The item's recipes an app variant's or app size's edit changes: copies
/// of the base's, in the base's place in the stack. Variant and size share
/// them: `item` is the edited enum's own layer (`cn-item-variant-*` or
/// `cn-item-size-*`); the other parts reach the size through its state
/// bits (`group-data-[size=sm]/item`).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ItemRecipes {
    /// `data-slot="item"`: the variant's or the size's layer, above `cn-item`.
    pub item: Recipe,
    /// `data-slot="item-media"`.
    pub item_media: Recipe,
    /// `data-slot="item-content"`.
    pub item_content: Recipe,
    /// `data-slot="item-title"`.
    pub item_title: Recipe,
    /// `data-slot="item-description"`.
    pub item_description: Recipe,
    /// `data-slot="item-actions"`.
    pub item_actions: Recipe,
    /// `data-slot="item-header"`.
    pub item_header: Recipe,
    /// `data-slot="item-footer"`.
    pub item_footer: Recipe,
    /// `data-slot="item-group"`.
    pub item_group: Recipe,
    /// `data-slot="item-separator"`.
    pub item_separator: Recipe,
}

impl ItemRecipes {
    /// `item` over the style's own parts.
    fn with_layer(item: &Recipe, s: &Style) -> Self {
        ItemRecipes {
            item: item.clone(),
            item_media: (*s.item_media).clone(),
            item_content: (*s.item_content).clone(),
            item_title: (*s.item_title).clone(),
            item_description: (*s.item_description).clone(),
            item_actions: (*s.item_actions).clone(),
            item_header: (*s.item_header).clone(),
            item_footer: (*s.item_footer).clone(),
            item_group: (*s.item_group).clone(),
            item_separator: (*s.item_separator).clone(),
        }
    }

    /// Every part, for an edit of all of them.
    #[cfg(test)]
    pub(crate) fn each_mut(&mut self, f: impl FnMut(&mut Recipe)) {
        self.parts_mut().into_iter().for_each(f);
    }

    fn parts_mut(&mut self) -> Vec<&mut Recipe> {
        vec![
            &mut self.item,
            &mut self.item_media,
            &mut self.item_content,
            &mut self.item_title,
            &mut self.item_description,
            &mut self.item_actions,
            &mut self.item_header,
            &mut self.item_footer,
            &mut self.item_group,
            &mut self.item_separator,
        ]
    }
}

impl Sealed for ItemVariant {}
impl AppEnum for ItemVariant {
    type Recipes = ItemRecipes;
    fn app_name(self) -> Option<&'static str> {
        match self {
            ItemVariant::App(a) => Some(a.name()),
            _ => None,
        }
    }
    fn parts(r: &mut ItemRecipes) -> Vec<&mut Recipe> {
        r.parts_mut()
    }
    fn recipes(self, style: &Style) -> ItemRecipes {
        ItemRecipes::with_layer(self.layer(style), style)
    }
}
impl VariantEnum for ItemVariant {}

impl Sealed for ItemSize {}
impl AppEnum for ItemSize {
    type Recipes = ItemRecipes;
    fn app_name(self) -> Option<&'static str> {
        match self {
            ItemSize::App(a) => Some(a.name()),
            _ => None,
        }
    }
    fn parts(r: &mut ItemRecipes) -> Vec<&mut Recipe> {
        r.parts_mut()
    }
    fn recipes(self, style: &Style) -> ItemRecipes {
        let mut r = ItemRecipes::with_layer(self.layer(style), style);
        self.fold(&mut r);
        r
    }
    fn fold(self, recipes: &mut ItemRecipes) {
        let bits = self.bits();
        for r in recipes.parts_mut() {
            r.fold_state(bits);
        }
    }
    fn bits(self) -> State {
        match self {
            ItemSize::Xs => State::SIZE_XS,
            ItemSize::Sm => State::SIZE_SM,
            _ => State::SIZE_DEFAULT,
        }
    }
}
impl SizeEnum for ItemSize {}

/// What an [`ItemGroup`] gives the items inside: its variant and its id,
/// which their sizes report to (`has-data-[size=*]`).
#[derive(Clone, Copy, Debug)]
struct GroupContext {
    variant: ItemVariant,
    id: Id,
}

/// What an item gives its parts: its variant, size and state bits, its id
/// (they report to it), and where the header and the footer rows go.
#[derive(Clone, Copy, Debug)]
struct ItemContext {
    variant: ItemVariant,
    size: ItemSize,
    id: Id,
    state: State,
    /// The left edge and width of the item's content box.
    left: f32,
    width: f32,
    /// The top of the header row and of the footer row.
    header_top: f32,
    footer_top: f32,
    /// Where the actions' free content starts: a header after it is out of
    /// upstream's order.
    actions_start: Pos2,
    /// The part is in the content column, whose min-content width decides
    /// whether the actions wrap.
    in_content: bool,
    /// The part is directly in the actions, which are `flex-none`: text
    /// there takes its one-line width.
    in_actions: bool,
}

/// What the item measured in a pass, for the next one to place by.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Memory {
    header: f32,
    footer: f32,
    media: Vec2,
    content: f32,
    actions: Vec2,
    line: f32,
    /// The content column's width, and its min-content width: its longest
    /// word.
    content_w: f32,
    min_content: f32,
    /// The width the item's box takes where its room is unbounded.
    natural: f32,
    /// Where free content stood when the footer was placed.
    footer_at: Option<Pos2>,
    /// The gap that spreads the header's and the footer's widgets over
    /// the row (`justify-between`).
    header_gap: Option<f32>,
    footer_gap: Option<f32>,
}

fn memory_key(item: Id) -> Id {
    item.with("shadcn_egui::item::memory")
}

fn next_key(item: Id) -> Id {
    item.with("shadcn_egui::item::next")
}

fn item_context(ui: &Ui) -> Option<ItemContext> {
    context::find::<ItemContext>(ui)
}

/// The recipes and state the parts of the item around `ui` draw with.
fn part_setup(ui: &Ui, t: &Theme) -> (ResolvedPair<ItemVariant, ItemSize>, State) {
    let cx = item_context(ui);
    debug_assert!(cx.is_some(), "an item part is called inside an Item");
    let (v, z) = cx.map_or((ItemVariant::Default, ItemSize::Default), |c| {
        (c.variant, c.size)
    });
    let pair = t.resolve_pair(v, z);
    let bits = cx.map_or_else(|| pair.bits(), |c| c.state);
    (pair, t.state(ui.ctx(), bits))
}

/// The part of an image of `image` size that covers a box of `frame` size
/// (`object-cover`), centred, in texture coordinates.
fn cover_uv(image: Vec2, frame: Vec2) -> Rect {
    let full = Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)); // behaviour-ok: texture coordinates run from 0 to 1
    if image.min_elem() <= 0.0 || frame.min_elem() <= 0.0 {
        return full;
    }
    let scale = (frame.x / image.x).max(frame.y / image.y);
    let part = frame / (image * scale);
    Rect::from_center_size(full.center(), part)
}

type MediaFn<'a> = Box<dyn FnOnce(&mut Ui) + 'a>;

enum Media<'a> {
    Icon(Glyph),
    Image(ImageSource<'a>),
    Ui(MediaFn<'a>),
}

/// A shadcn item: a row of media, a title and description, and actions.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use shadcn_egui::components::button::{Button, ButtonSize};
/// use shadcn_egui::components::item::{Item, ItemVariant};
/// use shadcn_egui::icons::lucide;
///
/// Item::new()
///     .variant(ItemVariant::Outline)
///     .icon(lucide::INBOX)
///     .title("Inbox")
///     .description("3 unread messages")
///     .show(ui, |ui| ui.add(Button::new("Open").size(ButtonSize::Sm)));
/// let row = ui.add(Item::new().title("Settings").sense(egui::Sense::click()));
/// if row.clicked() { /* open the settings */ }
/// # });
/// ```
///
/// Media is one of [`Item::icon`], [`Item::image`] and [`Item::media`];
/// the closure of [`Item::show`] is the actions element, one row at the
/// line's end. [`item_header`] and [`item_footer`] in the closure take
/// rows of their own above and below, the header before any action, the
/// footer last (a `debug_assert!` checks both).
///
/// The title wraps as upstream's does at the pin (its `line-clamp-1` is
/// cancelled by its `flex`); a word too long for the column is clipped.
/// The description clamps at the style's line count.
///
/// The actions take a line of their own below once the media, the
/// content's longest word and the actions do not fit the width (upstream's
/// `flex-wrap`).
///
/// The item fills the width of its `Ui` (upstream's `w-full`); in
/// unbounded room it is as wide as its content. It keeps
/// what it measured from one pass to the next under an auto id; items
/// whose parts change between passes need [`Item::id_salt`].
#[must_use = "You should put this widget in a ui with `ui.add(widget);` or call .show()"]
pub struct Item<'a> {
    variant: Option<ItemVariant>,
    size: ItemSize,
    title: Option<WidgetText>,
    description: Option<WidgetText>,
    media: Option<Media<'a>>,
    alt: Option<String>,
    sense: Sense,
    link: bool,
    selected: Option<bool>,
    id_salt: Option<Id>,
}

impl Default for Item<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Item<'a> {
    /// An item without content.
    #[inline]
    pub fn new() -> Self {
        Self {
            variant: None,
            size: ItemSize::Default,
            title: None,
            description: None,
            media: None,
            alt: None,
            sense: Sense::hover(),
            link: false,
            selected: None,
            id_salt: None,
        }
    }

    /// The title ([`ItemTitle`]), first in the content column.
    #[inline]
    pub fn title(mut self, title: impl Into<WidgetText>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The description ([`ItemDescription`]), below the title.
    #[inline]
    pub fn description(mut self, description: impl Into<WidgetText>) -> Self {
        self.description = Some(description.into());
        self
    }

    fn set_media(&mut self, media: Media<'a>) {
        debug_assert!(self.media.is_none(), "an item takes one media setter");
        self.media = Some(media);
    }

    /// Icon media (`Item.Media variant="icon"`), at the style's icon size
    /// in the item's text colour, or [`Glyph::Spinner`] (`<Spinner />` in
    /// the media) at the spinner's own size.
    #[inline]
    pub fn icon(mut self, icon: impl Into<Glyph>) -> Self {
        self.set_media(Media::Icon(icon.into()));
        self
    }

    /// Image media (`Item.Media variant="image"`): sized by the item's
    /// size, rounded. Name it with [`Item::alt_text`].
    #[inline]
    pub fn image(mut self, src: impl Into<ImageSource<'a>>) -> Self {
        self.set_media(Media::Image(src.into()));
        self
    }

    /// The image's accessible name (`alt`), as egui's `Image::alt_text`.
    #[inline]
    pub fn alt_text(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self
    }

    /// Media of the caller's own (`Item.Media`, the default variant): an
    /// avatar, an avatar stack.
    #[inline]
    pub fn media(mut self, add: impl FnOnce(&mut Ui) + 'a) -> Self {
        self.set_media(Media::Ui(Box::new(add)));
        self
    }

    /// shadcn `variant`, or an app variant ([`ItemVariant::app`]). Inside
    /// an [`ItemGroup`] the group's variant is the default.
    #[inline]
    pub fn variant(mut self, variant: impl Into<ItemVariant>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    /// shadcn `size`, or an app size ([`ItemSize::app`]).
    #[inline]
    pub fn size(mut self, size: impl Into<ItemSize>) -> Self {
        self.size = size.into();
        self
    }

    /// What the item senses, as egui's `Sense`. `Sense::click()` makes it a
    /// button: clicks, Tab focus with the focus ring, Enter and Space.
    /// `Sense::CLICK` alone clicks without taking Tab focus. With
    /// [`Item::link`], the senses add up.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// The item is a link (upstream's `child` snippet with an `<a>`): the
    /// style's link hover look, click and focus, Enter only, the pointing
    /// hand. Its `clicked()` is where the app opens the target.
    #[inline]
    pub fn link(mut self, link: bool) -> Self {
        self.link = link;
        self
    }

    /// The item is the caller's chosen one (`aria-selected`): reported to
    /// assistive technology and set as `State::SELECTED` for an app
    /// variant's look; the styles draw no look of their own.
    #[inline]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// The id under which the item keeps what it measured in the previous
    /// pass, scoped to the `Ui` (as `TextEdit::id_salt`).
    #[inline]
    pub fn id_salt(mut self, salt: impl Hash) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    fn sense_all(&self) -> Sense {
        if self.link {
            self.sense | Sense::click()
        } else {
            self.sense
        }
    }

    /// Show the item with `actions` as its actions element: one row at the
    /// line's end, `gap-2` apart, vertically centred. [`item_header`] and
    /// [`item_footer`] go in it too.
    pub fn show<R>(self, ui: &mut Ui, actions: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::current(ui.ctx());
        let id = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => ui.next_auto_id(),
        };
        let in_group = context::find::<GroupContext>(ui);
        let variant = self
            .variant
            .or(in_group.map(|g| g.variant))
            .unwrap_or_default();
        let sense = self.sense_all();
        let sensed = sense.senses_click() || sense.is_focusable();
        let rv = t.resolve(variant);
        let rz = t.resolve(self.size);
        let pair = t.resolve_pair(variant, self.size);
        let root = [
            &*t.style.item,
            rv.part(&t.style, ItemVariant::layer, |r| &r.item),
            rz.part(&t.style, ItemSize::layer, |r| &r.item),
        ];
        let group = Group::begin(ui, id);
        let size_bits = pair.bits();
        if let Some(g) = in_group {
            let has = if size_bits.contains(State::SIZE_SM) {
                State::HAS_SIZE_SM
            } else if size_bits.contains(State::SIZE_XS) {
                State::HAS_SIZE_XS
            } else {
                State::NONE
            };
            if has != State::NONE {
                registry::report(ui.ctx(), g.id, has);
            }
        }
        let has_description =
            self.description.is_some() || group.presence(State::HAS_DESCRIPTION) != State::NONE;
        let bits = size_bits
            .with(State::LINK, self.link)
            .with(State::HAS_DESCRIPTION, has_description)
            .with(State::SELECTED, self.selected == Some(true));
        let rest = t.state(ui.ctx(), bits);
        let mem = ui
            .data(|d| d.get_temp::<Memory>(memory_key(id)))
            .unwrap_or_default();

        let ppp = ui.ctx().pixels_per_point();
        let border = line_width(pick_len(&root, |r| &r.border_width, rest), ppp);
        let (pad_t, pad_r, pad_b, pad_l) = (
            pick_len(&root, |r| &r.pad_top, rest),
            pick_len(&root, |r| &r.pad_right, rest),
            pick_len(&root, |r| &r.pad_bottom, rest),
            pick_len(&root, |r| &r.pad_left, rest),
        );
        let gap = pick_len(&root, |r| &r.gap, rest);
        let look = TextLook::resolve(ui, &root, rest);
        let side = border + pad_l;
        let end = pad_r + border;
        // `w-full`: the item fills its room; in unbounded room it takes
        // its content's width, measured in the previous pass.
        let bounded = ui.available_width().is_finite();
        let outer_w = if bounded {
            layout::capped_width(ui, f32::INFINITY, side + end)
        } else {
            side + mem.natural + end
        };
        let (origin, mut frame) = layout::begin_block(ui, outer_w);
        let fid = frame.unique_id();
        // The item's own hit area goes in before its content, so the
        // actions' widgets lie above it; the size is the previous pass's
        // until the box is known. Its id is not the frame's: egui takes
        // the focus from a `Ui`'s id each time it builds that `Ui`, which
        // does not sense focus.
        let hit = fid.with("shadcn_egui::item::hit");
        // At `origin`: the previous pass's place is stale once the item
        // moves, and egui reports one id at two rects in a pass as a
        // clash. A size change redoes the pass anyway.
        let prev = ui.ctx().read_response(hit).map_or(Rect::NOTHING, |r| {
            Rect::from_min_size(origin, r.rect.size())
        });
        let _ = ui.interact(prev, hit, sense);
        let back = RecipeBox::reserve(ui);

        let left = origin.x + side;
        let right = origin.x + outer_w - end;
        let top = origin.y + border + pad_t;
        let line_top = if mem.header > 0.0 {
            top + mem.header + gap
        } else {
            top
        };
        let line = mem.line;
        let mut next = Memory::default();

        let media_l = [
            pair.part(&t.style, |s| &s.item_media, |r| &r.item_media),
            match self.media {
                Some(Media::Icon(_)) => &*t.style.item_media_variant_icon,
                Some(Media::Image(_)) => &*t.style.item_media_variant_image,
                _ => &*t.style.item_media_variant_default,
            },
        ];
        let centred = |h: f32| line_top + ((line - h) / 2.0).max(0.0);
        let media_top = |h: f32| {
            if has_description {
                shift(
                    Rect::from_min_size(egui::pos2(left, line_top), Vec2::ZERO),
                    None,
                    pick(&media_l, |r| &r.shift_y, rest),
                )
                .top()
            } else {
                centred(h)
            }
        };
        let painter = part_painter(ui, fid, &[&root[..]], rest);
        let alt = self.alt.clone();
        let spinner = matches!(self.media, Some(Media::Icon(Glyph::Spinner)));
        match self.media {
            Some(Media::Icon(glyph)) => {
                // The spinner's `size-4` is a `size-*` class: the media's
                // `[&_svg:not([class*='size-'])]` size does not reach it.
                let (icon, s, angle) = match glyph {
                    Glyph::Icon(icon) => {
                        let s = pick(&media_l, |r| &r.icon_size, rest).unwrap_or(t.metrics.icon);
                        (icon, s, 0.0)
                    }
                    Glyph::Spinner => {
                        let angle = crate::components::spinner::spin(ui, &[&*t.style.spinner]);
                        (
                            t.icons.loader,
                            crate::components::spinner::own_size(&t),
                            angle,
                        )
                    }
                };
                let at = Rect::from_min_size(egui::pos2(left, media_top(s)), Vec2::splat(s));
                paint_icon_rotated(
                    &painter,
                    at,
                    &icon,
                    look.resolved_color(ui),
                    t.metrics.icon_stroke,
                    angle,
                );
                next.media = at.size();
            }
            Some(Media::Image(src)) => {
                let w = pick_len(&media_l, |r| &r.width, rest);
                let h = pick_len(&media_l, |r| &r.height, rest);
                let at = Rect::from_min_size(egui::pos2(left, media_top(h)), egui::vec2(w, h));
                let radius = RecipeBox::resolve(ui.ctx(), &t, &media_l, rest).radius_on(at);
                let mut image = egui::Image::new(src)
                    .corner_radius(radius)
                    .fit_to_exact_size(at.size())
                    .maintain_aspect_ratio(false);
                // `object-cover`: the image fills the box, its overflow
                // cut evenly on both sides, once its size is known.
                if let Some(size) = image
                    .load_for_size(ui.ctx(), at.size())
                    .ok()
                    .and_then(|p| p.size())
                {
                    image = image.uv(cover_uv(size, at.size()));
                }
                if let Some(alt) = alt {
                    image = image.alt_text(alt);
                }
                let _ = frame.put(at, image);
                next.media = at.size();
            }
            Some(Media::Ui(add)) => {
                // As high as the media measured last pass: egui centres a
                // row's widgets in the height it has.
                let at = Rect::from_min_size(
                    egui::pos2(left, media_top(mem.media.y)),
                    egui::vec2(right - left, mem.media.y),
                );
                let mut m = frame.new_child(
                    UiBuilder::new()
                        .max_rect(at)
                        .layout(Layout::left_to_right(Align::Center)),
                );
                m.spacing_mut().item_spacing.x = pick_len(&media_l, |r| &r.gap, rest);
                content_scope(&mut m, &look, add);
                next.media = m.min_rect().size();
            }
            None => {}
        }
        let content_left = if next.media.x > 0.0 {
            left + next.media.x + gap
        } else {
            left
        };
        // `flex-wrap`: the content is `flex-1` with `min-width: auto`, so
        // the actions take a line of their own once media, the content's
        // longest word, the actions and the gaps exceed the width.
        let media_w = if next.media.x > 0.0 {
            next.media.x + gap
        } else {
            0.0
        };
        let wrapped = bounded
            && mem.actions.x > 0.0
            && media_w + mem.min_content + gap + mem.actions.x > right - left;
        let content_right = if !bounded {
            f32::INFINITY
        } else if mem.actions.x > 0.0 && !wrapped {
            right - mem.actions.x - gap
        } else {
            right
        };
        let actions_line = if wrapped { mem.actions.y + gap } else { 0.0 };
        let actions_start = if wrapped {
            egui::pos2(left, line_top + line + gap)
        } else if bounded {
            egui::pos2(right - mem.actions.x, centred(mem.actions.y))
        } else {
            egui::pos2(left + media_w + mem.content_w + gap, centred(mem.actions.y))
        };
        let cx = ItemContext {
            variant,
            size: self.size,
            id,
            state: bits,
            left,
            width: right - left,
            header_top: top,
            footer_top: line_top + line + actions_line + gap,
            actions_start,
            in_content: false,
            in_actions: false,
        };
        ui.data_mut(|d| d.insert_temp(next_key(id), next));

        // The content column: `flex-1`, `overflow-hidden` on the title.
        let content_l = [pair.part(&t.style, |s| &s.item_content, |r| &r.item_content)];
        let column = Rect::from_min_max(
            egui::pos2(content_left, centred(mem.content)),
            egui::pos2(content_right.max(content_left), f32::INFINITY),
        );
        let mut content = frame.new_child(context::push(
            group.push(
                UiBuilder::new()
                    .max_rect(column)
                    .layout(Layout::top_down(Align::Min)),
            ),
            ItemContext {
                in_content: true,
                ..cx
            },
        ));
        content.set_clip_rect(content.clip_rect().intersect(Rect::from_x_y_ranges(
            column.x_range(),
            egui::Rangef::EVERYTHING,
        )));
        content.spacing_mut().item_spacing.y = pick_len(&content_l, |r| &r.gap, rest);
        let (title_text, desc_text) = (
            self.title.as_ref().map(|t| t.text().to_owned()),
            self.description.as_ref().map(|t| t.text().to_owned()),
        );
        content_scope(&mut content, &look, |ui| {
            if let Some(title) = self.title {
                ui.add(ItemTitle::new(title));
            }
            if let Some(description) = self.description {
                ui.add(ItemDescription::new(description));
            }
        });
        let content_h = if title_text.is_some() || desc_text.is_some() {
            content.min_rect().height()
        } else {
            0.0
        };

        // The actions element: the closure's free content.
        let actions_l = [pair.part(&t.style, |s| &s.item_actions, |r| &r.item_actions)];
        let mut act = frame.new_child(context::push(
            group.push(
                UiBuilder::new()
                    .max_rect(Rect::from_min_size(
                        cx.actions_start,
                        egui::vec2(
                            if bounded {
                                right - cx.actions_start.x
                            } else {
                                f32::INFINITY
                            },
                            mem.actions.y,
                        ),
                    ))
                    .layout(Layout::left_to_right(Align::Center)),
            ),
            ItemContext {
                in_actions: true,
                ..cx
            },
        ));
        act.spacing_mut().item_spacing.x = pick_len(&actions_l, |r| &r.gap, rest);
        // The cursor of the scope the closure fills, where `item_footer`
        // read it: `act`'s own moves past the scope's allocation.
        let (inner, end) = content_scope(&mut act, &look, |ui| {
            let inner = actions(ui);
            (inner, ui.cursor().min)
        })
        .inner;
        let mut next = ui
            .data_mut(|d| d.remove_temp::<Memory>(next_key(id)))
            .unwrap_or_default();
        if let Some(at) = next.footer_at {
            debug_assert!(
                end.x <= at.x + 0.5, // behaviour-ok: below half a point the cursor did not move
                "free content in an item comes before item_footer"
            );
        }
        next.footer_at = None;
        let used = act.min_rect();
        next.actions = if used.width() > 0.0 {
            used.size()
        } else {
            Vec2::ZERO
        };
        next.content = content_h;
        next.content_w = if content_h > 0.0 {
            content.min_rect().width()
        } else {
            0.0
        };
        next.line = next.media.y.max(content_h);
        if !wrapped {
            next.line = next.line.max(next.actions.y);
        }
        next.natural = media_w
            + next.content_w
            + if next.actions.x > 0.0 {
                gap + next.actions.x
            } else {
                0.0
            };
        let _ = group.end(ui.ctx());

        let mut h = next.line;
        if wrapped {
            h += gap + next.actions.y;
        }
        if next.header > 0.0 {
            h += next.header + gap;
        }
        if next.footer > 0.0 {
            h += next.footer + gap;
        }
        let outer = Rect::from_min_size(
            origin,
            egui::vec2(outer_w, border + pad_t + h + pad_b + border),
        );
        if next != mem {
            ui.data_mut(|d| d.insert_temp(memory_key(id), next));
            ui.ctx()
                .request_discard("item::show measured parts changed");
        }

        drop(frame);
        let _ = ui.allocate_rect(outer, Sense::hover());
        let mut response = ui.interact(outer, hit, sense);
        let st = if sensed {
            let kind = if self.link {
                ActivationKeys::Enter
            } else {
                ActivationKeys::Button
            };
            pressable(&mut response, kind);
            if self.link && response.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            interaction_state(&t, &response, bits)
        } else {
            rest
        };
        let boxed =
            RecipeBox::resolve(ui.ctx(), &t, &root, st).transitioned(ui.ctx(), fid, &root, st);
        painter.set(
            back,
            Shape::Vec(boxed.shapes_at(ui.ctx(), outer, painter.opacity())),
        );

        let kind = if self.link {
            WidgetType::Link
        } else if sense.senses_click() {
            WidgetType::Button
        } else {
            WidgetType::Other
        };
        let enabled = response.enabled();
        let info = || WidgetInfo::labeled(kind, enabled, title_text.as_deref().unwrap_or(""));
        // A plain item is a `<div>`, with no disabled state; a link or
        // button item is a control.
        if kind == WidgetType::Other {
            crate::components::shared::widget_info_never_disabled(&response, info);
        } else {
            response.widget_info(info);
        }
        if spinner {
            crate::components::spinner::report_hosted(ui, &response, 0);
        }
        // `aria-selected` is AccessKit's selected state; egui's
        // `WidgetInfo::selected` is a toggle button's pressed state.
        let selected = self.selected;
        ui.ctx().accesskit_node_builder(hit, |node| {
            if let Some(desc) = desc_text {
                node.set_description(desc);
            }
            if let Some(selected) = selected {
                node.set_selected(selected);
            }
        });
        InnerResponse::new(inner, response)
    }
}

impl Widget for Item<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui, |_| ()).response
    }
}

/// A title or description in the item's content column.
fn text_part(
    ui: &mut Ui,
    text: WidgetText,
    builtin: fn(&Style) -> &Recipe,
    app: fn(&ItemRecipes) -> &Recipe,
) -> Response {
    let t = Theme::current(ui.ctx());
    let (pair, st) = part_setup(ui, &t);
    let l = [pair.part(&t.style, builtin, app)];
    let id = ui.next_auto_id().with("shadcn_egui::item::part");
    let look = TextLook::resolve(ui, &l, st);
    if let Some(cx) = item_context(ui).filter(|c| c.in_content) {
        // The longest word: what the column cannot shrink below.
        let longest = text
            .text()
            .split_whitespace()
            .map(|w| {
                look.layout(ui, w, TextWrapMode::Extend, f32::INFINITY)
                    .size()
                    .x
            })
            .fold(0.0, f32::max);
        ui.data_mut(|d| {
            let n = d.get_temp_mut_or_default::<Memory>(next_key(cx.id));
            n.min_content = n.min_content.max(longest);
        });
    }
    // The actions are `flex-none`, laid out at the width of the last
    // pass: text wrapped there would keep the first pass's width.
    let wrap = if item_context(ui).is_some_and(|c| c.in_actions) {
        TextWrapMode::Extend
    } else {
        TextWrapMode::Wrap
    };
    part_ui(ui, id, &[&l[..]], st, |ui| look.label(ui, text, wrap))
}

/// `item-title`: the item's text at the title's weight. Inside an
/// [`Item`]; the item's title setter draws one.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct ItemTitle {
    text: WidgetText,
}

impl ItemTitle {
    #[inline]
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for ItemTitle {
    fn ui(self, ui: &mut Ui) -> Response {
        text_part(ui, self.text, |s| &s.item_title, |r| &r.item_title)
    }
}

/// `item-description`: muted text, clamped at the style's line count.
/// Inside an [`Item`]: in its actions closure it is a second content
/// column (`item-image-demo`'s duration). The item moves its media to the
/// top while it has one.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct ItemDescription {
    text: WidgetText,
}

impl ItemDescription {
    #[inline]
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for ItemDescription {
    fn ui(self, ui: &mut Ui) -> Response {
        if let Some(cx) = item_context(ui) {
            registry::report(ui.ctx(), cx.id, State::HAS_DESCRIPTION);
        }
        text_part(
            ui,
            self.text,
            |s| &s.item_description,
            |r| &r.item_description,
        )
    }
}

/// A header or footer row: `add` left to right across the item's width,
/// its widgets spread to both ends (`justify-between`).
fn row_part<R>(ui: &mut Ui, footer: bool, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    let t = Theme::current(ui.ctx());
    let (pair, st) = part_setup(ui, &t);
    let l = [if footer {
        pair.part(&t.style, |s| &s.item_footer, |r| &r.item_footer)
    } else {
        pair.part(&t.style, |s| &s.item_header, |r| &r.item_header)
    }];
    let Some(cx) = item_context(ui) else {
        return ui.horizontal(add);
    };
    let key = next_key(cx.id);
    let next = ui.data(|d| d.get_temp::<Memory>(key)).unwrap_or_default();
    // Upstream's order: the header before the actions, the footer last.
    debug_assert!(
        next.footer_at.is_none(),
        "item_footer comes last in an item"
    );
    debug_assert!(
        footer || ui.cursor().min.x <= cx.actions_start.x + 0.5, // behaviour-ok: below half a point the cursor did not move
        "item_header comes before the item's actions"
    );
    let top = if footer { cx.footer_top } else { cx.header_top };
    // As high as the row measured last pass: egui centres a row's widgets
    // in the height it has.
    let mem = ui
        .data(|d| d.get_temp::<Memory>(memory_key(cx.id)))
        .unwrap_or_default();
    let height = if footer { mem.footer } else { mem.header };
    let mut row = ui.new_child(context::push(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(
                egui::pos2(cx.left, top),
                egui::vec2(cx.width, height),
            ))
            .layout(Layout::left_to_right(Align::Center)),
        ItemContext {
            in_actions: false,
            ..cx
        },
    ));
    // `justify-between`: the widgets spread over the row with the gap
    // their widths left last pass, never closer than `gap`.
    let gap = pick_len(&l, |r| &r.gap, st);
    let spread = if footer {
        mem.footer_gap
    } else {
        mem.header_gap
    };
    row.spacing_mut().item_spacing.x = spread.unwrap_or(gap);
    let items = crate::layout::RowItems::begin(&row);
    let inner = add(&mut row);
    let between = items
        .finish(&row, false)
        .first()
        .map_or(gap, |line| line.between_gap(cx.width, gap));
    let rect = row.min_rect();
    let at = ui.cursor().min;
    ui.data_mut(|d| {
        let n = d.get_temp_mut_or_default::<Memory>(key);
        if footer {
            n.footer = rect.height();
            n.footer_at = Some(at);
            n.footer_gap = Some(between);
        } else {
            n.header = rect.height();
            n.header_gap = Some(between);
        }
    });
    let response = ui.interact(rect, row.unique_id(), Sense::hover());
    InnerResponse::new(inner, response)
}

/// `item-header`: a row of its own above the media and content, its
/// widgets left to right and spread to both ends (`justify-between`): the
/// first at the start, the last at the end. Call it in the [`Item::show`] closure before the
/// actions.
#[must_use = "Call .show(ui, |ui| ..)"]
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemHeader {}

impl ItemHeader {
    #[inline]
    pub fn new() -> Self {
        Self {}
    }

    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        row_part(ui, false, add)
    }
}

/// `item-footer`: a row of its own below the media and content, its
/// widgets spread as the header's. Call it
/// last in the [`Item::show`] closure.
#[must_use = "Call .show(ui, |ui| ..)"]
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemFooter {}

impl ItemFooter {
    #[inline]
    pub fn new() -> Self {
        Self {}
    }

    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        row_part(ui, true, add)
    }
}

/// `ui.add(ItemTitle::new(text))`.
pub fn item_title(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(ItemTitle::new(text))
}

/// `ui.add(ItemDescription::new(text))`.
pub fn item_description(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(ItemDescription::new(text))
}

/// `ItemHeader::new().show(ui, add)`.
pub fn item_header<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ItemHeader::new().show(ui, add)
}

/// `ItemFooter::new().show(ui, add)`.
pub fn item_footer<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ItemFooter::new().show(ui, add)
}

/// `item-group` (`role="list"`): a column of items, its gap following the
/// items' size (`has-data-[size=sm|xs]`), learned from the previous pass.
/// Its variant reaches the items that name none.
///
/// Only the items report their size. Upstream's `has-data-[size=*]` also
/// matches any sized descendant: an avatar, a switch, a select, a card or
/// a toggle group with `size="sm"` inside a default item makes the browser
/// group's gap smaller; here it does not.
#[must_use = "Call .show(ui, |ui| ..)"]
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemGroup {
    variant: ItemVariant,
    id_salt: Option<Id>,
}

impl ItemGroup {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// The variant of the items inside that name none.
    #[inline]
    pub fn variant(mut self, variant: impl Into<ItemVariant>) -> Self {
        self.variant = variant.into();
        self
    }

    /// The id under which the group keeps its items' sizes from the
    /// previous pass, scoped to the `Ui`.
    #[inline]
    pub fn id_salt(mut self, salt: impl Hash) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::current(ui.ctx());
        let id = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => ui.next_auto_id(),
        };
        let group = Group::begin(ui, id);
        let v = t.resolve(self.variant);
        let l = [v.part(&t.style, |_, s| &s.item_group, |r| &r.item_group)];
        let st = t.state(
            ui.ctx(),
            v.bits()
                .or(group.presence(State::HAS_SIZE_SM | State::HAS_SIZE_XS)),
        );
        let gap = pick_len(&l, |r| &r.gap, st);
        let (_, mut frame) = layout::begin_item(ui);
        let mut list = frame.new_child(context::push(
            group.push(UiBuilder::new().layout(Layout::top_down(Align::Min))),
            GroupContext {
                variant: self.variant,
                id,
            },
        ));
        list.spacing_mut().item_spacing.y = gap;
        let inner = add(&mut list);
        let rect = list.min_rect();
        // The `List` role goes on the `Ui` the items are in, so they are
        // its children in the AccessKit tree, not grandchildren.
        let list_id = list.unique_id();
        drop(list);
        let _ = group.end(ui.ctx());
        let response = layout::end_item(ui, frame, rect, Sense::hover());
        ui.ctx().accesskit_node_builder(list_id, |node| {
            node.set_role(egui::accesskit::Role::List);
        });
        InnerResponse::new(inner, response)
    }
}

/// `item-separator`: a horizontal rule between the items of an
/// [`ItemGroup`], with the style's margin above and below; decorative.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemSeparator {}

impl ItemSeparator {
    #[inline]
    pub fn new() -> Self {
        Self {}
    }
}

impl Widget for ItemSeparator {
    fn ui(self, ui: &mut Ui) -> Response {
        let g = context::find::<GroupContext>(ui);
        debug_assert!(g.is_some(), "item_separator is called inside an ItemGroup");
        let t = Theme::current(ui.ctx());
        let v = t.resolve(g.map_or(ItemVariant::Default, |g| g.variant));
        let l = [v.part(&t.style, |_, s| &s.item_separator, |r| &r.item_separator)];
        let st = t.state(ui.ctx(), v.bits());
        ui.add_space(pick_len(&l, |r| &r.margin_top, st));
        let response = ui.add(crate::components::separator::Separator::new());
        ui.add_space(pick_len(&l, |r| &r.margin_bottom, st));
        response
    }
}

/// `ui.add(ItemSeparator::new())`.
pub fn item_separator(ui: &mut Ui) -> Response {
    ui.add(ItemSeparator::new())
}

#[cfg(test)]
mod tests {
    /// `object-cover`: a wide image loses its sides, a tall one its top
    /// and bottom, one of the box's shape nothing.
    #[test]
    fn cover_uv_cuts_the_overflow_evenly() {
        use super::cover_uv;
        use egui::{pos2, vec2, Rect};
        let r = |a: (f32, f32), b: (f32, f32)| Rect::from_min_max(pos2(a.0, a.1), pos2(b.0, b.1));
        let full = r((0.0, 0.0), (1.0, 1.0));
        assert_eq!(
            cover_uv(vec2(80.0, 40.0), vec2(40.0, 40.0)),
            r((0.25, 0.0), (0.75, 1.0))
        );
        assert_eq!(
            cover_uv(vec2(40.0, 160.0), vec2(40.0, 40.0)),
            r((0.0, 0.375), (1.0, 0.625))
        );
        assert_eq!(cover_uv(vec2(64.0, 64.0), vec2(32.0, 32.0)), full);
        assert_eq!(cover_uv(vec2(0.0, 0.0), vec2(32.0, 32.0)), full);
    }

    use super::*;
    use crate::style::{pick, Paint, Role, TextSize};
    use crate::Theme;

    /// An app variant's edit gets `ItemRecipes` and changes only what it
    /// names: its `item` is the variant's layer, the parts the style's.
    #[test]
    fn variant_edit_compiles_and_changes_its_part() {
        const BRAND: AppVariant<ItemVariant> = ItemVariant::app("brand");
        let t = Theme::vega().variant(BRAND, ItemVariant::Outline, |r: &mut ItemRecipes, _| {
            r.item_title.text.set(Paint::Role(Role::Primary, 1.0))
        });
        let v = t.resolve(ItemVariant::from(BRAND));
        assert_eq!(
            v.part(&t.style, ItemVariant::layer, |r| &r.item),
            &*t.style.item_variant_outline
        );
        let pair = t.resolve_pair(ItemVariant::from(BRAND), ItemSize::Default);
        let title = pair.part(&t.style, |s| &s.item_title, |r| &r.item_title);
        assert_eq!(
            title.text.get(State::NONE),
            Some(Paint::Role(Role::Primary, 1.0))
        );
        let desc = pair.part(&t.style, |s| &s.item_description, |r| &r.item_description);
        assert_eq!(desc, &*t.style.item_description);
    }

    /// An app size's `item` is its base size's layer; its parts take the
    /// base's state-bit values unconditionally, so a `set` replaces them.
    #[test]
    fn size_edit_compiles_and_replaces_its_base_size_value() {
        const TIGHT: AppVariant<ItemSize> = ItemSize::app("tight");
        let t = Theme::vega().size(TIGHT, ItemSize::Xs, |r: &mut ItemRecipes, _| {
            r.item.gap.set(4.0);
            r.item_description.text_size.set(TextSize::Lg);
        });
        let z = t.resolve(ItemSize::from(TIGHT));
        assert_eq!(z.bits(), State::SIZE_XS);
        let root = z.part(&t.style, ItemSize::layer, |r| &r.item);
        assert_eq!(pick(&[root], |r| &r.gap, State::SIZE_XS), Some(4.0));
        assert_eq!(
            pick(&[root], |r| &r.pad_top, State::NONE),
            pick(&[&*t.style.item_size_xs], |r| &r.pad_top, State::NONE)
        );
        let pair = t.resolve_pair(ItemVariant::Default, ItemSize::from(TIGHT));
        let desc = pair.part(&t.style, |s| &s.item_description, |r| &r.item_description);
        assert_eq!(
            pick(&[desc], |r| &r.text_size, pair.bits()),
            Some(TextSize::Lg)
        );
        // An unedited part is `Xs`'s, its `data-[size=xs]` value unconditional.
        let content = pair.part(&t.style, |s| &s.item_content, |r| &r.item_content);
        assert_eq!(pick(&[content], |r| &r.gap, State::NONE), Some(0.0));
    }

    /// With an app variant and an app size, the root keeps both edits
    /// (each on its own layer) and the parts take both in stack order.
    #[test]
    fn variant_and_size_edits_both_hold() {
        const BRAND: AppVariant<ItemVariant> = ItemVariant::app("brand");
        const TIGHT: AppVariant<ItemSize> = ItemSize::app("tight");
        let t = Theme::vega()
            .variant(BRAND, ItemVariant::Muted, |r: &mut ItemRecipes, _| {
                r.item.bg.set(Paint::Role(Role::Primary, 1.0));
                r.item_title.text.set(Paint::Role(Role::Primary, 1.0));
            })
            .size(TIGHT, ItemSize::Sm, |r: &mut ItemRecipes, _| {
                r.item.gap.set(4.0);
                r.item_content.gap.set(1.0);
            });
        let (v, z) = (ItemVariant::from(BRAND), ItemSize::from(TIGHT));
        let rv = t.resolve(v);
        let rz = t.resolve(z);
        let root = [
            &*t.style.item,
            rv.part(&t.style, ItemVariant::layer, |r| &r.item),
            rz.part(&t.style, ItemSize::layer, |r| &r.item),
        ];
        let pair = t.resolve_pair(v, z);
        let st = pair.bits();
        assert_eq!(st, State::SIZE_SM);
        assert_eq!(
            pick(&root, |r| &r.bg, st),
            Some(Paint::Role(Role::Primary, 1.0))
        );
        assert_eq!(pick(&root, |r| &r.gap, st), Some(4.0));
        let title = pair.part(&t.style, |s| &s.item_title, |r| &r.item_title);
        assert_eq!(title.text.get(st), Some(Paint::Role(Role::Primary, 1.0)));
        let content = pair.part(&t.style, |s| &s.item_content, |r| &r.item_content);
        assert_eq!(pick(&[content], |r| &r.gap, st), Some(1.0));
    }
}
