//! shadcn Alert, ported from shadcn-svelte `ui/alert/*.svelte` at
//! `tools/stylegen/PIN`. Padding, gaps, radius, border, colours, the icon's
//! size and offset and the text of the title and description come from the
//! style's `cn-alert*` recipes, stacked as upstream joins their classes
//! (`cn-alert`, then `cn-alert-variant-*`).
//!
//! Upstream lays the alert out as a grid (`has-[>svg]:grid-cols-[auto_1fr]`):
//! with an icon, the icon spans the first two rows of the first column
//! (`*:[svg]:row-span-2`) and the title and description stack in the
//! second, `gap-0.5` apart. [`Alert::show`] builds that grid by hand: the
//! icon column, then one content column that holds the title, the
//! description and the caller's content. Where the content column holds a
//! single title or description, the icon's second row stays empty and its
//! gap stays, as the browser lays out an implicit empty row.
//!
//! The action ([`AlertAction`]) is absolutely placed in the browser
//! (`absolute top-2.5 right-3`): it takes no room in the grid, and the
//! alert reserves room for it with `has-data-[slot=alert-action]:pr-18`.
//! The alert learns that an action exists from the previous pass, and the
//! action takes its width from the previous pass; where either changes,
//! the pass is discarded and redone (conformance 3.2).
//!
//! The `[&_a]` and `[&_p]` rules on the title and description style links
//! and paragraphs in caller content, which egui lays out and draws itself
//! (D-100).

use crate::components::context;
use crate::components::icon::{paint_icon, Icon};
use crate::components::registry::{self, Group, Item};
use crate::components::shared::{
    content_ui, line_width, part_painter, part_ui, shift, widget_info_with_role, RecipeBox,
};
use crate::components::text::{content_scope, TextLook};
use crate::layout;
use crate::style::app::{Resolved, Sealed};
use crate::style::{pick, AppEnum, AppVariant, Prop, Recipe, State, Style, VariantEnum};
use crate::Theme;
use egui::{
    Align, InnerResponse, Layout, Rect, Response, Sense, Shape, TextWrapMode, Ui, UiBuilder,
    Widget, WidgetInfo, WidgetText, WidgetType,
};

/// shadcn `variant` of an alert, or an app variant ([`AlertVariant::app`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum AlertVariant {
    /// `bg-card text-card-foreground`.
    #[default]
    Default,
    /// Destructive text; the description at `text-destructive/90`.
    Destructive,
    /// An app variant: [`AlertVariant::app`].
    #[doc(hidden)]
    App(AppVariant<AlertVariant>),
}

impl AlertVariant {
    /// An app variant, defined in the theme ([`Theme::variant`]).
    pub const fn app(name: &'static str) -> AppVariant<AlertVariant> {
        AppVariant::new(name)
    }

    /// The variant layer (`cn-alert-variant-*`) of a built-in variant.
    fn layer(self, s: &Style) -> &Recipe {
        match self {
            AlertVariant::Default | AlertVariant::App(_) => &s.alert_variant_default,
            AlertVariant::Destructive => &s.alert_variant_destructive,
        }
    }
}

impl From<AppVariant<AlertVariant>> for AlertVariant {
    fn from(app: AppVariant<AlertVariant>) -> Self {
        AlertVariant::App(app)
    }
}

/// The alert's recipes an app variant's edit changes: copies of the
/// base's, in the base's place in the stack.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct AlertRecipes {
    /// `data-slot="alert"`: the variant's layer, above `cn-alert`.
    pub alert: Recipe,
    /// `data-slot="alert-title"`.
    pub alert_title: Recipe,
    /// `data-slot="alert-description"`.
    pub alert_description: Recipe,
    /// `data-slot="alert-action"`.
    pub alert_action: Recipe,
}

impl Sealed for AlertVariant {}
impl AppEnum for AlertVariant {
    type Recipes = AlertRecipes;
    fn app_name(self) -> Option<&'static str> {
        match self {
            AlertVariant::App(a) => Some(a.name()),
            _ => None,
        }
    }
    fn recipes(self, style: &Style) -> AlertRecipes {
        AlertRecipes {
            alert: self.layer(style).clone(),
            alert_title: (*style.alert_title).clone(),
            alert_description: (*style.alert_description).clone(),
            alert_action: (*style.alert_action).clone(),
        }
    }
    fn bits(self) -> State {
        match self {
            AlertVariant::Destructive => State::VARIANT_DESTRUCTIVE,
            _ => State::NONE,
        }
    }
}
impl VariantEnum for AlertVariant {}

/// What an alert gives its parts: its variant, its state bits and where
/// its action goes.
#[derive(Clone, Copy, Debug)]
struct AlertContext {
    variant: AlertVariant,
    /// The alert's id: its parts report to it and key their state under it.
    id: egui::Id,
    /// The alert's state bits (variant, breakpoints).
    state: State,
    /// The top right corner of the action's place: the padding box's top
    /// right, `top-2.5 right-3` inside it.
    action_corner: egui::Pos2,
}

/// The context of the alert around `ui`, if any.
fn alert_context(ui: &Ui) -> Option<AlertContext> {
    context::find::<AlertContext>(ui)
}

/// The resolved variant of the alert around `ui` (the default outside one)
/// and the state its parts draw in.
fn part_setup(ui: &Ui, t: &Theme) -> (Resolved<AlertVariant>, State) {
    let cx = alert_context(ui);
    let v = t.resolve(cx.map_or(AlertVariant::Default, |c| c.variant));
    let bits = cx.map_or_else(|| v.bits(), |c| c.state);
    (v, t.state(ui.ctx(), bits))
}

fn len(layers: &[&Recipe], f: fn(&Recipe) -> &Prop<f32>, st: State) -> f32 {
    pick(layers, f, st).unwrap_or(0.0)
}

/// A shadcn alert (`role="alert"`): a bordered box with an optional icon, a
/// title and a description.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use shadcn_egui::components::alert::{Alert, AlertVariant};
/// use shadcn_egui::icons::lucide;
///
/// ui.add(
///     Alert::new()
///         .variant(AlertVariant::Destructive)
///         .icon(lucide::CIRCLE_ALERT)
///         .title("Payment failed")
///         .description("Your payment could not be processed."),
/// );
/// # });
/// ```
///
/// With content of its own, the alert takes a closure that holds its
/// parts ([`alert_title`], [`alert_description`], [`alert_action`]) and
/// any other widgets, in upstream's order: title, description, action.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use shadcn_egui::components::alert::{alert_action, Alert};
/// use shadcn_egui::components::button::{Button, ButtonSize};
///
/// let enable = Alert::new()
///     .title("Dark mode is now available")
///     .description("Enable it under your profile settings to get started.")
///     .show(ui, |ui| {
///         alert_action(ui, |ui| ui.add(Button::new("Enable").size(ButtonSize::Xs)).clicked())
///             .inner
///     })
///     .inner;
/// # });
/// ```
///
/// The alert fills the width of its `Ui` (upstream's `w-full`); a
/// narrower alert (the docs' `max-w-md`) sits in a narrower `Ui`. Where the
/// width is unbounded, it takes its content's width.
///
/// It keeps whether it has an action from one pass to the next under an
/// auto id; two alerts whose parts change between passes (an action that
/// comes and goes in a loop) need [`Alert::id_salt`].
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Alert {
    variant: AlertVariant,
    icon: Option<Icon>,
    title: Option<WidgetText>,
    description: Option<WidgetText>,
    id_salt: Option<egui::Id>,
}

impl Default for Alert {
    fn default() -> Self {
        Self::new()
    }
}

impl Alert {
    /// An alert without content.
    #[inline]
    pub fn new() -> Self {
        Self {
            variant: AlertVariant::Default,
            icon: None,
            title: None,
            description: None,
            id_salt: None,
        }
    }

    /// shadcn `variant`: `Default` or `Destructive`, or an app variant
    /// ([`AlertVariant::app`]). The alert's parts inside take it.
    #[inline]
    pub fn variant(mut self, variant: impl Into<AlertVariant>) -> Self {
        self.variant = variant.into();
        self
    }

    /// The icon before the title and description (upstream's `<svg>` child),
    /// in the alert's text colour.
    #[inline]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The title ([`AlertTitle`]), drawn before the closure's content.
    #[inline]
    pub fn title(mut self, title: impl Into<WidgetText>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The description ([`AlertDescription`]), drawn after the title and
    /// before the closure's content.
    #[inline]
    pub fn description(mut self, description: impl Into<WidgetText>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// The id under which the alert keeps what it learned about its parts
    /// in the previous pass, scoped to the `Ui` (as `TextEdit::id_salt`).
    #[inline]
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(egui::Id::new(salt));
        self
    }

    /// Show the alert with `add` below its title and description, in the
    /// content column. `add` holds the alert's parts and other widgets; it
    /// takes the alert's text size and colour.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::current(ui.ctx());
        let id = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => ui.next_auto_id(),
        };
        let variant = t.resolve(self.variant);
        let root = [
            &*t.style.alert,
            variant.part(&t.style, AlertVariant::layer, |r| &r.alert),
        ];
        let group = Group::begin(ui, id);
        let bits = variant.bits();
        let st = t.state(
            ui.ctx(),
            bits.with(State::HAS_ICON, self.icon.is_some())
                .or(group.presence(State::HAS_ACTION)),
        );

        let ppp = ui.ctx().pixels_per_point();
        let border = line_width(len(&root, |r| &r.border_width, st), ppp);
        let (pad_t, pad_r, pad_b, pad_l) = (
            len(&root, |r| &r.pad_top, st),
            len(&root, |r| &r.pad_right, st),
            len(&root, |r| &r.pad_bottom, st),
            len(&root, |r| &r.pad_left, st),
        );
        let gap = len(&root, |r| &r.gap, st);
        let icon_size = self
            .icon
            .map(|_| pick(&root, |r| &r.icon_size, st).unwrap_or(t.metrics.icon));
        let icon_col = icon_size.map_or(0.0, |s| s + len(&root, |r| &r.gap_x, st));
        let side = border + pad_l + icon_col;
        let end = pad_r + border;
        let look = TextLook::resolve(ui, &root, st);

        // `w-full`: the alert fills its room; in unbounded room it takes
        // its content's width.
        let bounded = ui.available_width().is_finite();
        let outer_w = layout::capped_width(ui, f32::INFINITY, side + end);
        let (origin, mut frame) = layout::begin_item(ui, outer_w);
        let width_key = id.with("shadcn_egui::alert::width");
        let known_w = if bounded {
            Some(outer_w)
        } else {
            ui.data(|d| d.get_temp::<f32>(width_key))
        };
        let action = [variant.part(&t.style, |_, s| &s.alert_action, |r| &r.alert_action)];
        let parts = AlertContext {
            variant: self.variant,
            id,
            state: bits,
            action_corner: egui::pos2(
                known_w.map_or(f32::INFINITY, |w| origin.x + w - border)
                    - len(&action, |r| &r.inset_right, st),
                origin.y + border + len(&action, |r| &r.inset_top, st),
            ),
        };

        let back = RecipeBox::reserve(ui);
        let top = origin.y + border + pad_t;
        let column = Rect::from_min_max(
            egui::pos2(origin.x + side, top),
            egui::pos2(
                if bounded {
                    origin.x + outer_w - end
                } else {
                    f32::INFINITY
                },
                f32::INFINITY,
            ),
        );
        let mut content = frame.new_child(context::push(
            group.push(
                UiBuilder::new()
                    .max_rect(column)
                    .layout(Layout::top_down(Align::Min)),
            ),
            parts,
        ));
        content.spacing_mut().item_spacing.y = gap;
        let inner = content_scope(&mut content, &look, |ui| {
            content_ui(ui, id.with("content"), &[&root[..]], st, |ui| {
                if let Some(title) = self.title {
                    ui.add(AlertTitle::new(title));
                }
                if let Some(description) = self.description {
                    ui.add(AlertDescription::new(description));
                }
                add(ui)
            })
        })
        .inner;
        let rows = group.end(ui.ctx()).items.len();

        let used = content.min_rect();
        let mut content_h = (used.bottom() - top).max(0.0);
        if let Some(size) = icon_size {
            // The icon spans two rows: a single row keeps the empty
            // second row's gap.
            if rows == 1 {
                content_h += gap;
            }
            content_h = content_h.max(size);
        }
        let outer_w = if bounded {
            outer_w
        } else {
            let w = (used.right() - origin.x).max(side) + end;
            if known_w.is_none_or(|k| (k - w).abs() > f32::EPSILON) {
                ui.data_mut(|d| d.insert_temp(width_key, w));
                ui.ctx()
                    .request_discard("alert::show unbounded width changed");
            }
            w
        };
        let outer = Rect::from_min_size(
            origin,
            egui::vec2(outer_w, border + pad_t + content_h + pad_b + border),
        );

        let painter = part_painter(ui, id, &[&root[..]], st);
        painter.set(
            back,
            Shape::Vec(RecipeBox::resolve(ui.ctx(), &t, &root, st).shapes(ui.ctx(), outer)),
        );
        if let (Some(icon), Some(size)) = (self.icon, icon_size) {
            // `*:[svg]:text-current`: the icon in the alert's text colour.
            let at = Rect::from_min_size(
                egui::pos2(origin.x + border + pad_l, top),
                egui::vec2(size, size),
            );
            let at = shift(at, None, pick(&root, |r| &r.icon_shift_y, st));
            paint_icon(
                &painter,
                at,
                &icon,
                look.resolved_color(ui),
                t.metrics.icon_stroke,
            );
        }

        let response = layout::end_item(ui, frame, outer, Sense::hover());
        widget_info_with_role(&response, egui::accesskit::Role::Alert, || {
            WidgetInfo::new(WidgetType::Other)
        });
        InnerResponse::new(inner, response)
    }
}

impl Widget for Alert {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui, |_| ()).response
    }
}

/// `alert-title`: the alert's text at the title's weight. Inside an
/// [`Alert`], in its content column.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct AlertTitle {
    text: WidgetText,
}

impl AlertTitle {
    #[inline]
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for AlertTitle {
    fn ui(self, ui: &mut Ui) -> Response {
        text_part(ui, self.text, |_, s| &s.alert_title, |r| &r.alert_title)
    }
}

/// `alert-description`: muted text (destructive in the destructive
/// variant), wrapped balanced (`text-balance`, `md:text-pretty`). Inside
/// an [`Alert`], in its content column.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct AlertDescription {
    text: WidgetText,
}

impl AlertDescription {
    #[inline]
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for AlertDescription {
    fn ui(self, ui: &mut Ui) -> Response {
        text_part(
            ui,
            self.text,
            |_, s| &s.alert_description,
            |r| &r.alert_description,
        )
    }
}

/// A title or description: one row of the alert's content column.
fn text_part(
    ui: &mut Ui,
    text: WidgetText,
    builtin: fn(AlertVariant, &Style) -> &Recipe,
    app: fn(&AlertRecipes) -> &Recipe,
) -> Response {
    let t = Theme::current(ui.ctx());
    // Upstream's source order: title and description, then the action,
    // whose widgets come last in Tab order.
    debug_assert!(
        alert_context(ui)
            .is_none_or(|cx| registry::reported(ui.ctx(), cx.id, State::HAS_ACTION) == State::NONE),
        "alert_action comes after the title and description"
    );
    let (v, st) = part_setup(ui, &t);
    let l = [v.part(&t.style, builtin, app)];
    let look = TextLook::resolve(ui, &l, st);
    let id = ui.next_auto_id().with("shadcn_egui::alert::part");
    let response = part_ui(ui, id, &[&l[..]], st, |ui| {
        look.label(ui, text, TextWrapMode::Wrap)
    });
    let _ = registry::join(
        ui,
        Item {
            id: response.id,
            size: response.rect.size(),
            enabled: response.enabled(),
        },
    );
    response
}

/// `alert-action`: widgets placed in the alert's top right corner
/// (`absolute top-2.5 right-3`), left to right in the order the code adds
/// them. It takes no room in the alert's content; the alert reserves room
/// on its right for it (`has-data-[slot=alert-action]:pr-18`). Call it in
/// the [`Alert::show`] closure after the title and description.
///
/// It takes its width from the previous pass, and the alert learns from
/// the previous pass that it has an action; where either changes, the pass
/// is discarded and redone. Outside an alert it lays out in place.
#[must_use = "Call .show(ui, |ui| ..)"]
#[derive(Clone, Copy, Debug, Default)]
pub struct AlertAction {}

impl AlertAction {
    #[inline]
    pub fn new() -> Self {
        Self {}
    }

    /// Show `add` in the action's place.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::current(ui.ctx());
        let (v, st) = part_setup(ui, &t);
        let l = [v.part(&t.style, |_, s| &s.alert_action, |r| &r.alert_action)];
        let id = ui.next_auto_id().with("shadcn_egui::alert::action");
        let Some(cx) = alert_context(ui) else {
            return ui.scope(|ui| content_ui(ui, id, &[&l[..]], st, add));
        };
        registry::report(ui.ctx(), cx.id, State::HAS_ACTION);
        let room = ui.max_rect();
        let place = Rect::from_min_max(
            egui::pos2(room.left(), cx.action_corner.y),
            egui::pos2(cx.action_corner.x, f32::INFINITY),
        );
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(place)
                .layout(Layout::left_to_right(Align::Min)),
        );
        layout::end_aligned(
            &mut child,
            id,
            Align::Min,
            "alert::alert_action width changed",
            |ui| content_ui(ui, id.with("content"), &[&l[..]], st, add),
        )
    }
}

/// `ui.add(AlertTitle::new(text))`.
pub fn alert_title(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(AlertTitle::new(text))
}

/// `ui.add(AlertDescription::new(text))`.
pub fn alert_description(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(AlertDescription::new(text))
}

/// `AlertAction::new().show(ui, add)`.
pub fn alert_action<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    AlertAction::new().show(ui, add)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Paint, Role};

    /// An app variant's edit gets `AlertRecipes` and changes only what it
    /// names; the rest keeps the base's values, its state bits included.
    #[test]
    fn edit_compiles_and_changes_its_part() {
        const LOUD: AppVariant<AlertVariant> = AlertVariant::app("loud");
        let t = Theme::vega().variant(
            LOUD,
            AlertVariant::Destructive,
            |r: &mut AlertRecipes, _| r.alert_title.text.set(Paint::Role(Role::Primary, 1.0)),
        );
        let v = t.resolve(AlertVariant::from(LOUD));
        assert_eq!(v.bits(), State::VARIANT_DESTRUCTIVE);
        let title = v.part(&t.style, |_, s| &s.alert_title, |r| &r.alert_title);
        assert_eq!(
            title.text.get(State::NONE),
            Some(Paint::Role(Role::Primary, 1.0))
        );
        let layer = v.part(&t.style, AlertVariant::layer, |r| &r.alert);
        assert_eq!(layer, &*t.style.alert_variant_destructive);
        let desc = v.part(
            &t.style,
            |_, s| &s.alert_description,
            |r| &r.alert_description,
        );
        assert_eq!(desc, &*t.style.alert_description);
    }

    /// The alert's parts take its variant from the context; a nested alert
    /// gives its own, and outside an alert there is none.
    #[test]
    fn parts_read_the_alerts_variant() {
        const LOUD: AppVariant<AlertVariant> = AlertVariant::app("loud");
        let ctx = egui::Context::default();
        Theme::vega()
            .variant(LOUD, AlertVariant::Default, |_, _| {})
            .apply(&ctx);
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert!(alert_context(ui).is_none());
            Alert::new().variant(LOUD).show(ui, |ui| {
                let cx = alert_context(ui).expect("inside an alert");
                assert_eq!(cx.variant, AlertVariant::from(LOUD));
                Alert::new()
                    .variant(AlertVariant::Destructive)
                    .show(ui, |ui| {
                        let cx = alert_context(ui).expect("inside an alert");
                        assert_eq!(cx.variant, AlertVariant::Destructive);
                        assert!(cx.state.contains(State::VARIANT_DESTRUCTIVE));
                    });
            });
        });
    }
}
