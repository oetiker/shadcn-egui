//! shadcn Item, ported from shadcn-svelte `ui/item/*.svelte` at
//! `tools/stylegen/PIN`.

use crate::style::app::Sealed;
use crate::style::{AppEnum, AppVariant, Recipe, SizeEnum, State, Style, VariantEnum};

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

#[cfg(test)]
mod tests {
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
