//! App variants and app sizes: looks an app defines once in its theme, from
//! a built-in variant or size, and names at the call
//! ([`crate::Theme::variant`], [`crate::Theme::size`]).

use super::{Recipe, State, Style};
use crate::preset::StyleName;
use egui::Color32;
use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

/// The name of an app variant or app size of `X`, made by `X::app(name)`
/// (`ButtonVariant::app("danger-ghost")`). Name it through a constant, so a
/// misspelling does not compile; two constants with one name are one app
/// variant. A setter takes it as `impl Into<X>`.
pub struct AppVariant<X> {
    name: &'static str,
    _enum: PhantomData<fn() -> X>,
}

impl<X> AppVariant<X> {
    pub(crate) const fn new(name: &'static str) -> Self {
        AppVariant {
            name,
            _enum: PhantomData,
        }
    }

    /// The name the theme knows this app variant by.
    pub const fn name(self) -> &'static str {
        self.name
    }
}

// Derives would bound `X`; an app variant is a name whatever `X` is.
impl<X> Clone for AppVariant<X> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<X> Copy for AppVariant<X> {}
impl<X> PartialEq for AppVariant<X> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl<X> Eq for AppVariant<X> {}
impl<X> std::hash::Hash for AppVariant<X> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
impl<X> std::fmt::Debug for AppVariant<X> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AppVariant").field(&self.name).finish()
    }
}

mod sealed {
    pub trait Sealed {}
}
pub(crate) use sealed::Sealed;

/// A component's variant or size enum: the crate's own enums only.
pub trait AppEnum: Sealed + Copy + Eq + Default + std::fmt::Debug + Send + Sync + 'static {
    /// What an edit gets: one [`Recipe`] per upstream part the enum
    /// reaches, named after its `data-slot` (`ButtonRecipes { button }`).
    type Recipes: Clone + Send + Sync + 'static;
    /// The app case's name; `None` for a built-in case.
    #[doc(hidden)]
    fn app_name(self) -> Option<&'static str>;
    /// Copies of the recipes a built-in case draws with.
    #[doc(hidden)]
    fn recipes(self, style: &Style) -> Self::Recipes;
    /// The state bits a built-in case adds (tabs `VARIANT_LINE`).
    #[doc(hidden)]
    fn bits(self) -> State {
        State::NONE
    }
}

/// A variant enum: what [`crate::Theme::variant`] takes.
pub trait VariantEnum: AppEnum {}

/// A size enum: what [`crate::Theme::size`] takes.
pub trait SizeEnum: AppEnum {}

/// An edit: gets copies of the base's recipes and the style the theme
/// builds for, captures nothing.
pub type Edit<X> = fn(&mut <X as AppEnum>::Recipes, StyleName);

trait Entry: Send + Sync {
    fn key(&self) -> (TypeId, &'static str);
    fn build(&self, style: &Style, name: StyleName) -> Arc<dyn Any + Send + Sync>;
    fn same(&self, other: &dyn Entry) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn describe(&self) -> String;
}

struct Typed<X: AppEnum> {
    name: &'static str,
    base: X,
    edit: Edit<X>,
}

impl<X: AppEnum> Entry for Typed<X> {
    fn key(&self) -> (TypeId, &'static str) {
        (TypeId::of::<X>(), self.name)
    }
    fn build(&self, style: &Style, name: StyleName) -> Arc<dyn Any + Send + Sync> {
        let mut r = self.base.recipes(style);
        (self.edit)(&mut r, name);
        Arc::new(r)
    }
    fn same(&self, other: &dyn Entry) -> bool {
        other.as_any().downcast_ref::<Typed<X>>().is_some_and(|o| {
            o.name == self.name && o.base == self.base && std::ptr::fn_addr_eq(o.edit, self.edit)
        })
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn describe(&self) -> String {
        format!("{:?} from {:?}", self.name, self.base)
    }
}

/// The edited recipes of every entry, for one style.
struct Built {
    /// Held so a later `Arc::make_mut` of the theme's style copies it and
    /// the pointer comparison sees the change.
    style: Arc<Style>,
    recipes: Vec<Arc<dyn Any + Send + Sync>>,
}

/// A theme's app variants and app colours. Shared behind an `Arc`; the
/// edited recipes are built on the first lookup for a style and kept until
/// the style changes.
#[derive(Default)]
pub(crate) struct AppMap {
    entries: Vec<Arc<dyn Entry>>,
    /// An app colour's light and dark value.
    colors: Vec<(&'static str, Color32, Color32)>,
    built: Mutex<Option<Built>>,
}

impl Clone for AppMap {
    fn clone(&self) -> Self {
        // A clone is made to change the entries: it builds again.
        AppMap {
            entries: self.entries.clone(),
            colors: self.colors.clone(),
            built: Mutex::new(None),
        }
    }
}

impl PartialEq for AppMap {
    /// Equal entries and colours; the built recipes follow from them.
    fn eq(&self, other: &Self) -> bool {
        self.colors == other.colors
            && self.entries.len() == other.entries.len()
            && self
                .entries
                .iter()
                .zip(&other.entries)
                .all(|(a, b)| a.same(&**b))
    }
}

impl std::fmt::Debug for AppMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppMap")
            .field(
                "entries",
                &self
                    .entries
                    .iter()
                    .map(|e| e.describe())
                    .collect::<Vec<_>>(),
            )
            .field("colors", &self.colors)
            .finish()
    }
}

impl AppMap {
    pub(crate) fn insert<X: AppEnum>(&mut self, app: AppVariant<X>, base: X, edit: Edit<X>) {
        debug_assert!(
            base.app_name().is_none(),
            "app variant {:?}: its base {base:?} must be a built-in case",
            app.name
        );
        let base = if base.app_name().is_some() {
            X::default()
        } else {
            base
        };
        let key = (TypeId::of::<X>(), app.name);
        self.entries.retain(|e| e.key() != key);
        self.entries.push(Arc::new(Typed {
            name: app.name,
            base,
            edit,
        }));
        *self.built.get_mut().unwrap_or_else(|e| e.into_inner()) = None;
    }

    pub(crate) fn set_color(&mut self, name: &'static str, light: Color32, dark: Color32) {
        self.colors.retain(|(n, ..)| *n != name);
        self.colors.push((name, light, dark));
    }

    pub(crate) fn color(&self, name: &str, dark: bool) -> Option<Color32> {
        self.colors
            .iter()
            .find(|(n, ..)| *n == name)
            .map(|&(_, l, d)| if dark { d } else { l })
    }

    /// The recipes and state bits `x` draws with in `style`: a built-in
    /// case's own, or an app case's edited copies with its base's bits. An
    /// app case the theme does not define fails a `debug_assert!` and
    /// resolves to `X::default()`.
    pub(crate) fn resolve<X: AppEnum>(&self, style: &Arc<Style>, x: X) -> Resolved<X> {
        let Some(name) = x.app_name() else {
            return Resolved { base: x, app: None };
        };
        let key = (TypeId::of::<X>(), name);
        let Some(i) = self.entries.iter().position(|e| e.key() == key) else {
            debug_assert!(
                false,
                "app variant {name:?} of {} is not defined in the theme",
                std::any::type_name::<X>()
            );
            return Resolved {
                base: X::default(),
                app: None,
            };
        };
        let base = self.entries[i]
            .as_any()
            .downcast_ref::<Typed<X>>()
            .map(|t| t.base)
            .unwrap_or_default();
        let mut built = self.built.lock().unwrap_or_else(|e| e.into_inner());
        if built.as_ref().is_none_or(|b| !Arc::ptr_eq(&b.style, style)) {
            *built = Some(self.build(style));
        }
        let recipes = built
            .as_ref()
            .and_then(|b| b.recipes[i].clone().downcast::<X::Recipes>().ok());
        Resolved { base, app: recipes }
    }

    fn build(&self, style: &Arc<Style>) -> Built {
        let name = StyleName::from_id(style.name);
        debug_assert!(
            name.is_some(),
            "style {:?} is no upstream style: a restyled style keeps its source's name",
            style.name
        );
        let name = name.unwrap_or(StyleName::Vega);
        Built {
            style: Arc::clone(style),
            recipes: self.entries.iter().map(|e| e.build(style, name)).collect(),
        }
    }
}

/// The variant enum of a component without upstream variants: `$x` with
/// the one built-in case `Default` and the hidden app case, and `$recipes`
/// with one field per part, each named after its `data-slot` and copied
/// from the style's recipe of that name (rules, App variants).
macro_rules! default_variant {
    (
        $(#[$m:meta])*
        $x:ident,
        $(#[$rm:meta])*
        $recipes:ident { $($(#[$fm:meta])* $field:ident),+ $(,)? }
    ) => {
        $(#[$m])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum $x {
            /// The component as upstream draws it.
            #[default]
            Default,
            /// An app variant: [`Self::app`].
            #[doc(hidden)]
            App($crate::style::AppVariant<$x>),
        }

        impl $x {
            /// An app variant, defined in the theme ([`crate::Theme::variant`]).
            pub const fn app(name: &'static str) -> $crate::style::AppVariant<$x> {
                $crate::style::AppVariant::new(name)
            }
        }

        impl From<$crate::style::AppVariant<$x>> for $x {
            fn from(app: $crate::style::AppVariant<$x>) -> Self {
                $x::App(app)
            }
        }

        $(#[$rm])*
        #[derive(Clone, Debug, PartialEq)]
        #[non_exhaustive]
        pub struct $recipes {
            $($(#[$fm])* pub $field: $crate::style::Recipe,)+
        }

        impl $crate::style::app::Sealed for $x {}
        impl $crate::style::AppEnum for $x {
            type Recipes = $recipes;
            fn app_name(self) -> Option<&'static str> {
                match self {
                    $x::App(a) => Some(a.name()),
                    $x::Default => None,
                }
            }
            fn recipes(self, style: &$crate::style::Style) -> $recipes {
                $recipes {
                    $($field: (*style.$field).clone(),)+
                }
            }
        }
        impl $crate::style::VariantEnum for $x {}
    };
}
pub(crate) use default_variant;

/// What a component draws an enum value with ([`AppMap::resolve`]).
pub(crate) struct Resolved<X: AppEnum> {
    base: X,
    app: Option<Arc<X::Recipes>>,
}

impl<X: AppEnum> Resolved<X> {
    /// The built-in case: the value itself, or the app case's base.
    pub(crate) fn base(&self) -> X {
        self.base
    }

    /// The state bits of the built-in case.
    pub(crate) fn bits(&self) -> State {
        self.base.bits()
    }

    /// One part's recipe: the app case's edited copy (`app`), else the
    /// built-in case's (`builtin`).
    pub(crate) fn part<'a>(
        &'a self,
        style: &'a Style,
        builtin: fn(X, &Style) -> &Recipe,
        app: fn(&X::Recipes) -> &Recipe,
    ) -> &'a Recipe {
        match &self.app {
            Some(r) => app(r),
            None => builtin(self.base, style),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::button::ButtonVariant;
    use crate::style::{pick, Paint, Prop, Role, Rule, When};
    use crate::Theme;

    const DANGER_GHOST: AppVariant<ButtonVariant> = ButtonVariant::app("danger-ghost");
    const DANGER_OUTLINE: AppVariant<ButtonVariant> = ButtonVariant::app("danger-outline");
    const BRAND: Role = Role::custom("brand");

    fn danger_ghost(r: &mut <ButtonVariant as AppEnum>::Recipes, _: StyleName) {
        r.button.text.set(Paint::Role(Role::Destructive, 1.0));
    }

    fn themed() -> Theme {
        Theme::vega()
            .variant(DANGER_GHOST, ButtonVariant::Ghost, danger_ghost)
            .variant(DANGER_OUTLINE, ButtonVariant::Outline, |r, _| {
                r.button.recolor(Role::Foreground, Role::Destructive)
            })
    }

    /// The variant layer `v` draws with, edited or built in.
    fn layer(t: &Theme, v: ButtonVariant) -> Recipe {
        let r = t.resolve(v);
        r.part(&t.style, ButtonVariant::layer, |r| &r.button)
            .clone()
    }

    const P: fn(Role) -> Paint = |r| Paint::Role(r, 1.0);

    #[test]
    fn app_variant_is_a_name() {
        assert_eq!(DANGER_GHOST, ButtonVariant::app("danger-ghost"));
        assert_ne!(DANGER_GHOST, DANGER_OUTLINE);
        assert_eq!(DANGER_GHOST.name(), "danger-ghost");
        assert_eq!(
            ButtonVariant::from(DANGER_GHOST),
            ButtonVariant::from(ButtonVariant::app("danger-ghost"))
        );
        assert_eq!(
            ButtonVariant::from(DANGER_GHOST).app_name(),
            Some("danger-ghost")
        );
        assert_eq!(ButtonVariant::Ghost.app_name(), None);
    }

    #[test]
    fn set_replaces_the_unconditional_value_and_keeps_state_rules() {
        let t = themed();
        let s = layer(&t, DANGER_GHOST.into());
        let ghost = &t.style.button_variant_ghost;
        assert_eq!(s.text.get(State::NONE), Some(P(Role::Destructive)));
        // Ghost's hover and expanded colours stay.
        assert_eq!(s.text.get(State::HOVER), ghost.text.get(State::HOVER));
        assert_eq!(s.text.rules, ghost.text.rules);
        assert_eq!(s.bg, ghost.bg);
    }

    #[test]
    fn set_writes_into_always_rules() {
        let mut p = Prop {
            base: None,
            rules: vec![Rule {
                when: When::ALWAYS,
                rank: 1 << 121,
                value: 16.0,
            }],
        };
        p.set(20.0);
        assert_eq!((p.base, p.get(State::NONE)), (None, Some(20.0)));
        let mut q = Prop::fixed(1.0);
        q.rules.push(Rule {
            when: When::all(State::HOVER),
            rank: 1 << 122,
            value: 2.0,
        });
        q.set(3.0);
        assert_eq!(
            (q.get(State::NONE), q.get(State::HOVER)),
            (Some(3.0), Some(2.0))
        );
    }

    #[test]
    fn on_replaces_the_rules_of_exactly_that_state() {
        let ghost = Style::vega().button_variant_ghost;
        let mut bg = ghost.bg.clone();
        let hover_rank = bg
            .rules
            .iter()
            .find(|r| r.when == When::all(State::HOVER))
            .map(|r| r.rank);
        bg.on(State::HOVER, P(Role::Accent));
        let hovers: Vec<_> = bg
            .rules
            .iter()
            .filter(|r| r.when == When::all(State::HOVER))
            .collect();
        assert_eq!(hovers.len(), 1);
        assert_eq!(Some(hovers[0].rank), hover_rank);
        assert_eq!(bg.get(State::HOVER), Some(P(Role::Accent)));
        // `dark:hover:` is another condition: it stays and wins in dark mode.
        assert_eq!(
            bg.get(State::DARK | State::HOVER),
            Some(Paint::Role(Role::Muted, 0.5))
        );
        // A state the property had no rule for ranks as a generated
        // `focus-visible:` class would.
        bg.on(State::FOCUS_VISIBLE, P(Role::Ring));
        let r = bg.rules.last().expect("pushed");
        assert_eq!(
            (r.when, r.rank),
            (When::all(State::FOCUS_VISIBLE), (1 << 29) | (1 << 122))
        );
        // `dark:sm:active:`: the variant bits combine, the class steps add.
        bg.on(State::DARK | State::SM | State::ACTIVE, P(Role::Ring));
        assert_eq!(
            bg.rules.last().map(|r| r.rank),
            Some((1 << 30) | (1 << 66) | (1 << 68) | (2 << 122))
        );
    }

    /// `on(state, v)` for a state the property has no rule for beats or
    /// loses to a generated rule of another state as the generated rules
    /// order them: ghost's `aria-expanded:` beats its `hover:`.
    #[test]
    fn on_for_a_new_state_orders_as_generated_rules() {
        for style in [Style::vega(), Style::nova(), Style::lyra()] {
            let ghost = &style.button_variant_ghost.bg;
            let only = |state| Prop {
                base: None,
                rules: ghost
                    .rules
                    .iter()
                    .filter(|r| r.when == When::all(state))
                    .cloned()
                    .collect(),
            };
            let both = State::HOVER | State::EXPANDED;
            // The generated order: the expanded rule wins.
            let mut generated = ghost.clone();
            generated
                .rules
                .retain(|r| r.when != When::all(State::DARK | State::HOVER));
            for r in &mut generated.rules {
                r.value = P(if r.when == When::all(State::EXPANDED) {
                    Role::Primary
                } else {
                    Role::Accent
                });
            }
            assert_eq!(
                generated.get(both),
                Some(P(Role::Primary)),
                "{}",
                style.name
            );
            // Hover rule kept, expanded added by `on`: `on` wins.
            let mut a = only(State::HOVER);
            a.on(State::EXPANDED, P(Role::Primary));
            assert_eq!(a.get(both), Some(P(Role::Primary)), "{}", style.name);
            // Expanded rule kept, hover added by `on`: `on` loses.
            let mut b = only(State::EXPANDED);
            b.on(State::HOVER, P(Role::Accent));
            assert_eq!(
                b.get(both),
                Some(ghost.get(State::EXPANDED).unwrap()),
                "{}",
                style.name
            );
            // The added rule's rank is the generated one.
            assert_eq!(
                a.rules.last().map(|r| r.rank),
                only(State::EXPANDED).rules.first().map(|r| r.rank)
            );
            assert_eq!(
                b.rules.last().map(|r| r.rank),
                only(State::HOVER).rules.first().map(|r| r.rank)
            );
        }
    }

    #[test]
    fn recolor_swaps_a_role_everywhere_and_keeps_opacity() {
        let t = themed();
        let s = layer(&t, DANGER_OUTLINE.into());
        let outline = &t.style.button_variant_outline;
        assert_eq!(outline.text.get(State::HOVER), Some(P(Role::Foreground)));
        assert_eq!(s.text.get(State::HOVER), Some(P(Role::Destructive)));
        // Other roles stay.
        assert_eq!(s.bg, outline.bg);
        let mut r = Recipe {
            border_color: Prop::fixed(Paint::Mix(Role::Foreground, Role::Border, 0.3)),
            ..Recipe::default()
        };
        r.ring_color.rules.push(Rule {
            when: When::all(State::FOCUS_VISIBLE),
            rank: 1,
            value: Paint::Role(Role::Foreground, 0.5),
        });
        r.recolor(Role::Foreground, BRAND);
        assert_eq!(
            r.border_color.base,
            Some(Paint::Mix(BRAND, Role::Border, 0.3))
        );
        assert_eq!(r.ring_color.rules[0].value, Paint::Role(BRAND, 0.5));
    }

    #[test]
    fn built_in_resolves_to_itself() {
        let t = themed();
        let r = t.resolve(ButtonVariant::Outline);
        assert!(r.app.is_none());
        assert_eq!(r.base(), ButtonVariant::Outline);
        assert_eq!(r.bits(), State::NONE);
    }

    #[test]
    fn app_variant_takes_the_base_place_in_the_stack() {
        let t = themed();
        let r = t.resolve(ButtonVariant::from(DANGER_GHOST));
        assert_eq!(r.base(), ButtonVariant::Ghost);
        let v = r.part(&t.style, |_, s| &s.button_variant_default, |r| &r.button);
        let stack = [&*t.style.button, v, &*t.style.button_size_default];
        let text = |s| pick(&stack, |r| &r.text, s);
        assert_eq!(text(State::NONE), Some(P(Role::Destructive)));
        assert_eq!(text(State::HOVER), Some(P(Role::Foreground)));
    }

    #[test]
    #[should_panic(expected = "is not defined in the theme")]
    fn undefined_name_fails_a_debug_assert() {
        let _ = Theme::vega().resolve(ButtonVariant::from(ButtonVariant::app("nope")));
    }

    #[test]
    fn edited_recipes_build_once_per_style() {
        let mut t = themed();
        let get = |t: &Theme| {
            t.resolve(ButtonVariant::from(DANGER_GHOST))
                .app
                .expect("defined")
        };
        let a = get(&t);
        assert!(Arc::ptr_eq(&a, &get(&t.clone())));
        // A style change builds from the new style.
        t.style = Arc::new(Style::nova());
        let b = get(&t);
        assert!(!Arc::ptr_eq(&a, &b));
        let mut want = Style::nova().button_variant_ghost.as_ref().clone();
        want.text.set(P(Role::Destructive));
        assert_eq!(b.button, want);
        // Changing the style in place counts too.
        Arc::make_mut(&mut t.style).button_variant_ghost.bg = Prop::fixed(P(Role::Accent));
        assert_eq!(get(&t).button.bg, Prop::fixed(P(Role::Accent)));
    }

    #[test]
    fn edit_gets_the_style_name() {
        const TALL: AppVariant<ButtonVariant> = ButtonVariant::app("tall");
        let mut t = Theme::vega().variant(TALL, ButtonVariant::Default, |r, style| {
            r.button
                .height
                .set(if style == StyleName::Nova { 40.0 } else { 50.0 })
        });
        let h = |t: &Theme| {
            t.resolve(ButtonVariant::from(TALL))
                .app
                .expect("defined")
                .button
                .height
                .get(State::NONE)
        };
        assert_eq!(h(&t), Some(50.0));
        t.style = Arc::new(Style::nova());
        assert_eq!(h(&t), Some(40.0));
    }

    #[test]
    fn app_colours_follow_the_mode_and_variants_do_not() {
        let t = themed().color(BRAND, egui::Color32::RED, egui::Color32::BLUE);
        let light = t.clone().in_mode(egui::Theme::Light);
        let dark = t.clone().in_mode(egui::Theme::Dark);
        assert_eq!(light.role(BRAND), egui::Color32::RED);
        assert_eq!(dark.role(BRAND), egui::Color32::BLUE);
        assert_eq!(
            dark.paint(Paint::Role(BRAND, 0.5)),
            egui::Color32::BLUE.gamma_multiply(0.5)
        );
        let v = |t: &Theme| {
            t.resolve(ButtonVariant::from(DANGER_GHOST))
                .app
                .expect("defined")
        };
        assert!(Arc::ptr_eq(&v(&light), &v(&dark)));
    }

    #[test]
    #[should_panic(expected = "is not defined in the theme")]
    fn undefined_colour_fails_a_debug_assert() {
        let _ = Theme::vega().role(BRAND);
    }

    #[test]
    fn theme_equality_compares_entries_not_the_cache() {
        let (a, b) = (themed(), themed());
        let _ = a.resolve(ButtonVariant::from(DANGER_GHOST));
        assert_eq!(a, b);
        let other = Theme::vega().variant(DANGER_GHOST, ButtonVariant::Ghost, |r, _| {
            r.button.text.set(P(Role::Primary))
        });
        assert_ne!(
            Theme::vega().variant(DANGER_GHOST, ButtonVariant::Ghost, danger_ghost),
            other
        );
        assert_ne!(
            Theme::vega(),
            Theme::vega().color(BRAND, egui::Color32::RED, egui::Color32::RED)
        );
        // A later definition of a name replaces the earlier one.
        let replaced = other.variant(DANGER_GHOST, ButtonVariant::Ghost, danger_ghost);
        assert_eq!(
            replaced,
            Theme::vega().variant(DANGER_GHOST, ButtonVariant::Ghost, danger_ghost)
        );
    }
}
