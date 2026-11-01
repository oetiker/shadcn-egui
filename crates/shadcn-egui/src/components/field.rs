//! shadcn Field, ported from shadcn-svelte `ui/field/*.svelte` at
//! `tools/stylegen/PIN`: [`Field`] (a label, a control, a description and
//! errors), [`FieldContent`] (a column of label and description beside a
//! control), [`FieldSet`] with its [`FieldLegend`], [`FieldGroup`] (a
//! column of fields), [`FieldSeparator`], and the text parts
//! [`FieldLabel`], [`FieldTitle`], [`FieldDescription`] and
//! [`FieldError`]. Gaps, margins, the text of every part and the choice
//! card's box come from the style's `cn-field*` recipes; a field label
//! stacks `cn-label` under `cn-field-label`, as upstream's FieldLabel is a
//! Label.
//!
//! A field passes what it knows to the crate controls inside (rules, axiom
//! 5): `invalid` to [`Input`](crate::components::input::Input),
//! [`Textarea`](crate::components::textarea::Textarea),
//! [`Checkbox`](crate::components::checkbox::Checkbox),
//! [`Switch`](crate::components::switch::Switch),
//! [`Select`](crate::components::select::Select) and
//! [`Button`](crate::components::button::Button) (each control's own
//! `.invalid(..)` wins), its label as the accessible name of the first
//! of them, and a press on its label, which toggles a checkbox or switch,
//! focuses a text field and opens a select, as upstream's `<label for>`
//! does. Its text colour reaches every part and control that sets none of
//! its own: in an invalid field the label, the title and a control's
//! typed or chosen value turn destructive, placeholders stay muted.
//!
//! What a container must know about its children before it draws (whether
//! a description is its last or last but one child, whether the field
//! holds a horizontal slider or a [`FieldContent`], the width of what
//! follows its label in a row, the checked and focus state of the control
//! in a choice card) it takes from the previous pass; where that changes,
//! the pass is discarded and redone (conformance 3.2).
//!
//! Upstream's `[&>a]` rules on the description style links in caller
//! content, which egui lays out and draws itself (D-100).

use crate::components::context;
use crate::components::shared::{
    disabled_cursor, line_width, never_disabled, part_painter, part_ui, widget_info_never_disabled,
    widget_info_with_role, RecipeBox,
};
use crate::components::text::{content_scope, TextLook};
use crate::layout;
use crate::style::app::{default_variant, Sealed};
use crate::style::{pick, pick_len, AppEnum, AppVariant, Recipe, State, Style, VariantEnum};
use crate::{Orientation, Theme};
use egui::accesskit::{Live, Role};
use egui::{
    Align, Id, InnerResponse, Layout, Pos2, Rect, Response, Sense, Shape, TextWrapMode, Ui,
    UiBuilder, Vec2, Widget, WidgetInfo, WidgetText, WidgetType,
};

/// Tailwind's `@md` container size (28rem): a responsive field is
/// horizontal in a [`FieldGroup`] at least this wide.
const CONTAINER_MD: f32 = 448.0; // behaviour-ok: Tailwind --container-md, the @md/field-group query

/// The error list's `ml-4` and `gap-1`: classes `field-error.svelte` sets
/// on its `<ul>`, an element without a slot, so no recipe carries them.
const ERROR_LIST_INDENT: f32 = 16.0; // behaviour-ok: field-error.svelte's ul ml-4, outside any recipe
const ERROR_LIST_GAP: f32 = 4.0; // behaviour-ok: field-error.svelte's ul gap-1, outside any recipe

default_variant!(
    /// The field's variant: upstream has none, so only app variants
    /// ([`FieldVariant::app`]) differ from `Default`. The parts inside the
    /// field take it.
    FieldVariant,
    /// The field's recipes an app variant's edit changes: the field and
    /// the parts that sit in it.
    FieldRecipes {
        /// `data-slot="field"`.
        field,
        /// `data-slot="field-content"`.
        field_content,
        /// `data-slot="field-label"` (over the `label` recipe).
        field_label,
        /// `data-slot="field-title"`.
        field_title,
        /// `data-slot="field-description"`.
        field_description,
        /// `data-slot="field-error"`.
        field_error,
    }
);

default_variant!(
    /// The field set's variant: upstream has none, so only app variants
    /// ([`FieldSetVariant::app`]) differ from `Default`. A label or
    /// description directly in the set takes it; the legend has its own
    /// ([`FieldLegendVariant`]).
    FieldSetVariant,
    /// The field set's recipes an app variant's edit changes: the set and
    /// the parts that sit directly in it.
    FieldSetRecipes {
        /// `data-slot="field-set"`.
        field_set,
        /// `data-slot="field-label"` directly in the set.
        field_label,
        /// `data-slot="field-description"` directly in the set.
        field_description,
    }
);

default_variant!(
    /// The field group's variant: upstream has none, so only app variants
    /// ([`FieldGroupVariant::app`]) differ from `Default` (a demo's
    /// tighter `gap-3` group). Its separators and choice cards take it.
    FieldGroupVariant,
    /// The field group's recipes an app variant's edit changes: the group
    /// and the parts that sit directly in it.
    FieldGroupRecipes {
        /// `data-slot="field-group"`.
        field_group,
        /// `data-slot="field-label"` directly in the group (a choice card).
        field_label,
        /// `data-slot="field-separator"`.
        field_separator,
        /// `data-slot="field-separator-content"`.
        field_separator_content,
    }
);

/// shadcn `variant` of a [`FieldLegend`], or an app variant
/// ([`FieldLegendVariant::app`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldLegendVariant {
    /// A set's heading (`text-base` in vega).
    #[default]
    Legend,
    /// A legend at a label's size (`text-sm` in vega).
    Label,
    /// An app variant: [`FieldLegendVariant::app`].
    #[doc(hidden)]
    App(AppVariant<FieldLegendVariant>),
}

impl FieldLegendVariant {
    /// An app variant, defined in the theme ([`Theme::variant`]).
    pub const fn app(name: &'static str) -> AppVariant<FieldLegendVariant> {
        AppVariant::new(name)
    }
}

impl From<AppVariant<FieldLegendVariant>> for FieldLegendVariant {
    fn from(app: AppVariant<FieldLegendVariant>) -> Self {
        FieldLegendVariant::App(app)
    }
}

/// The legend's recipes an app variant's edit changes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FieldLegendRecipes {
    /// `data-slot="field-legend"`.
    pub field_legend: Recipe,
}

impl Sealed for FieldLegendVariant {}
impl AppEnum for FieldLegendVariant {
    type Recipes = FieldLegendRecipes;
    fn app_name(self) -> Option<&'static str> {
        match self {
            FieldLegendVariant::App(a) => Some(a.name()),
            _ => None,
        }
    }
    fn parts(r: &mut FieldLegendRecipes) -> Vec<&mut Recipe> {
        vec![&mut r.field_legend]
    }
    fn recipes(self, style: &Style) -> FieldLegendRecipes {
        FieldLegendRecipes {
            field_legend: (*style.field_legend).clone(),
        }
    }
    fn bits(self) -> State {
        match self {
            FieldLegendVariant::Label => State::VARIANT_LABEL,
            _ => State::VARIANT_LEGEND,
        }
    }
}
impl VariantEnum for FieldLegendVariant {}

/// Whose variant a part draws in: the container it sits in.
#[derive(Clone, Copy, Debug)]
enum Owner {
    Field(FieldVariant),
    Set(FieldSetVariant),
    Group(FieldGroupVariant),
    None,
}

/// A part of the family, for [`part_recipe`].
#[derive(Clone, Copy, Debug)]
enum Part {
    Label,
    Title,
    Description,
    Error,
    Content,
    Separator,
    SeparatorContent,
}

/// The recipe of `part` in the variant of `owner`: the owner's app copy
/// where its recipes hold the part, else the style's.
fn part_recipe(t: &Theme, owner: Owner, part: Part) -> Recipe {
    macro_rules! of {
        ($v:expr, $f:ident) => {{
            let r = t.resolve($v);
            r.part(&t.style, |_, s| &s.$f, |r| &r.$f).clone()
        }};
    }
    match (owner, part) {
        (Owner::Field(v), Part::Label) => of!(v, field_label),
        (Owner::Field(v), Part::Title) => of!(v, field_title),
        (Owner::Field(v), Part::Description) => of!(v, field_description),
        (Owner::Field(v), Part::Error) => of!(v, field_error),
        (Owner::Field(v), Part::Content) => of!(v, field_content),
        (Owner::Set(v), Part::Label) => of!(v, field_label),
        (Owner::Set(v), Part::Description) => of!(v, field_description),
        (Owner::Group(v), Part::Label) => of!(v, field_label),
        (Owner::Group(v), Part::Separator) => of!(v, field_separator),
        (Owner::Group(v), Part::SeparatorContent) => of!(v, field_separator_content),
        (_, part) => {
            let s = &t.style;
            (**match part {
                Part::Label => &s.field_label,
                Part::Title => &s.field_title,
                Part::Description => &s.field_description,
                Part::Error => &s.field_error,
                Part::Content => &s.field_content,
                Part::Separator => &s.field_separator,
                Part::SeparatorContent => &s.field_separator_content,
            })
            .clone()
        }
    }
}

/// What kind of container a [`Parent`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Field,
    Content,
    Set,
    Group,
    Card,
}

/// The nearest container of the family around a part: one context type for
/// all of them, so the nearest one wins.
#[derive(Clone, Copy, Debug)]
struct Parent {
    kind: Kind,
    owner: Owner,
    /// The container's id: its children's record lives under it.
    id: Id,
    /// The state bits its parts draw in (invalid, disabled, the
    /// previous pass's `has-*` flags).
    state: State,
    /// Whether its children sit in a row (a horizontal field).
    row: bool,
}

/// What a field gives the crate controls inside it.
#[derive(Clone, Copy, Debug)]
struct FieldCx {
    id: Id,
    /// The label's id, fixed before the label draws.
    label: Id,
    /// The key a press on the field's label (or on the choice card around
    /// it) is stored under, for its control.
    press: Id,
    invalid: bool,
    /// Vertical in bounded room: every child takes the field's width
    /// (`*:w-full`).
    fills: bool,
    /// The bits a control inside takes from the field.
    control: State,
    /// The field's gap.
    gap: f32,
}

/// What a field group gives the responsive fields inside it.
#[derive(Clone, Copy, Debug)]
struct GroupCx {
    /// The group is at least `@md` (28rem) wide.
    wide: bool,
}

/// What a choice card gives the field and control inside it.
#[derive(Clone, Copy, Debug)]
struct CardCx {
    id: Id,
    /// The card's `has-*` flags of the previous pass.
    state: State,
}

/// A container's record of its children in one pass.
#[derive(Clone, Copy, Debug, Default)]
struct Children {
    /// The cursor after the last part.
    mark: Pos2,
    /// Children so far: parts and runs of free content.
    index: usize,
    /// The last child is a legend of the `Legend` variant.
    after_legend: bool,
    /// A peer control before this point is disabled (`peer-disabled:`).
    peer_disabled: bool,
    /// The field's label is drawn.
    label_drawn: bool,
    /// A control took the field's label.
    label_taken: bool,
    /// The field holds a crate control other than a button.
    form_control: bool,
    /// The right edge of the part that grows in a row.
    grower_right: Option<f32>,
    /// The `has-*` flags its children reported.
    presence: State,
    /// The legend's response id.
    legend: Option<Id>,
    /// The widths the controls after the growing part ask for, with the
    /// row's gaps before them.
    wanted: f32,
}

/// What a container learned in the previous pass.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Learned {
    count: usize,
    /// The width a row's growing part leaves to what follows it.
    trailing: f32,
    labelled: bool,
    /// [`Children::form_control`].
    form_control: bool,
    presence: State,
}

fn children_key(id: Id) -> Id {
    id.with("shadcn_egui::field::children")
}

fn learned_key(id: Id) -> Id {
    id.with("shadcn_egui::field::learned")
}

fn learned(ui: &Ui, id: Id) -> Learned {
    ui.data(|d| d.get_temp::<Learned>(learned_key(id)))
        .unwrap_or_default()
}

fn begin_children(ui: &Ui, id: Id) {
    let c = Children {
        mark: ui.cursor().min,
        ..Children::default()
    };
    ui.data_mut(|d| d.insert_temp(children_key(id), c));
}

fn with_children<R>(ui: &Ui, id: Id, f: impl FnOnce(&mut Children) -> R) -> R {
    ui.data_mut(|d| f(d.get_temp_mut_or_default::<Children>(children_key(id))))
}

/// The cursor moved past the mark: free content since the last part.
fn moved(ui: &Ui, mark: Pos2) -> bool {
    let at = ui.cursor().min;
    // behaviour-ok: below half a point the cursor did not move
    at.y > mark.y + 0.5 || at.x > mark.x + 0.5 // behaviour-ok: as above
}

/// A part's place among its container's children.
#[derive(Clone, Copy, Debug, Default)]
struct Place {
    index: usize,
    /// The previous pass's child count.
    count: usize,
    after_legend: bool,
    peer_disabled: bool,
}

impl Place {
    /// `last:` and `nth-last-2:` from the previous pass's count.
    fn state(self) -> State {
        let last = self.count > 0 && self.index + 1 == self.count;
        let nth_last_2 = self.index + 2 == self.count;
        State::NONE
            .with(State::LAST, last)
            .with(State::NTH_LAST_2, nth_last_2)
            .with(State::AFTER_LEGEND, self.after_legend)
    }
}

/// Count a run of free content that ends here, and return the place of
/// the part that comes next.
fn enter(ui: &Ui, p: &Parent) -> Place {
    let count = learned(ui, p.id).count;
    with_children(ui, p.id, |c| {
        if moved(ui, c.mark) {
            c.index += 1;
            c.after_legend = false;
        }
        Place {
            index: c.index,
            count,
            after_legend: c.after_legend,
            peer_disabled: c.peer_disabled,
        }
    })
}

/// The part is placed: count it and move the mark to the cursor.
fn leave(ui: &Ui, p: &Parent, legend: bool) {
    let at = ui.cursor().min;
    with_children(ui, p.id, |c| {
        c.index += 1;
        c.mark = at;
        c.after_legend = legend;
    });
}

/// End the record of `id`'s children: keep what it learned, with
/// `extra` filled in, and redo the pass where that changed.
fn end_children(ui: &Ui, id: Id, trailing: f32) -> Children {
    let c = ui
        .data_mut(|d| d.remove_temp::<Children>(children_key(id)))
        .unwrap_or_default();
    let count = c.index + usize::from(moved(ui, c.mark));
    let now = Learned {
        count,
        trailing,
        labelled: c.label_drawn,
        form_control: c.form_control,
        presence: c.presence,
    };
    let before = ui.data(|d| d.get_temp::<Learned>(learned_key(id)));
    if before != Some(now) && !ui.is_sizing_pass() {
        ui.data_mut(|d| d.insert_temp(learned_key(id), now));
        let differs = before.is_none_or(|b| {
            b.count != now.count
                || (b.trailing - now.trailing).abs() > 0.01 // behaviour-ok: float tolerance
                || b.labelled != now.labelled
                || b.form_control != now.form_control
                || b.presence != now.presence
        });
        if differs {
            ui.ctx()
                .request_discard("field: what a container knows of its children changed");
        }
    }
    c
}

fn parent(ui: &Ui) -> Option<Parent> {
    context::find::<Parent>(ui)
}

/// The part's owner and state: those of the container it sits in.
fn owner_state(ui: &Ui, t: &Theme) -> (Owner, Option<Parent>, State) {
    let p = parent(ui);
    let owner = p.map_or(Owner::None, |p| p.owner);
    let bits = p.map_or(State::NONE, |p| p.state);
    (owner, p, t.state(ui.ctx(), bits))
}

/// Place a part as one child of its container: its place first, then
/// `add`, then the mark after it.
fn as_child<R>(ui: &mut Ui, legend: bool, add: impl FnOnce(&mut Ui, Place) -> R) -> R {
    match parent(ui) {
        None => add(ui, Place::default()),
        Some(p) => {
            let place = enter(ui, &p);
            let inner = add(ui, place);
            leave(ui, &p, legend);
            inner
        }
    }
}

/// The width of the part that grows in a horizontal field
/// (`*:data-[slot=field-label]:flex-auto`, `flex-1` on a FieldContent):
/// the room left less what follows it in the previous pass, at least
/// `natural` where the room is unbounded.
fn grower_width(ui: &Ui, p: &Parent, natural: f32) -> Option<f32> {
    if !p.row || p.kind != Kind::Field {
        return None;
    }
    let first = with_children(ui, p.id, |c| c.grower_right.is_none());
    if !first {
        return None;
    }
    let room = ui.available_width();
    if !room.is_finite() {
        return Some(natural);
    }
    let trailing = learned(ui, p.id).trailing;
    Some((room - trailing).max(0.0))
}

fn grown(ui: &Ui, p: &Parent, right: f32) {
    with_children(ui, p.id, |c| c.grower_right = Some(right));
}

/// Report `flags` to the record of the container `id`.
fn report(ui: &Ui, id: Id, flags: State) {
    with_children(ui, id, |c| c.presence = c.presence | flags);
}

/// Store a press on a field's label for its control, and paint again so
/// that a control drawn before the label takes it.
fn press(ui: &Ui, key: Id) {
    ui.data_mut(|d| d.insert_temp(key, true));
    ui.ctx().request_repaint();
}

/// A label or a card: clickable, not focusable, as a `<label>`.
fn label_sense() -> Sense {
    Sense::click() - Sense::FOCUSABLE
}

// ---------------------------------------------------------------------------
// What a crate control takes from the field around it.

/// What a crate control inside a [`Field`] takes from it (rules, axiom 5).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ControlSlot {
    field: Option<FieldCx>,
    card: Option<CardCx>,
    parent: Option<Parent>,
    /// This control took the field's label.
    label: Option<Id>,
    /// The label (or the card) was pressed for this control.
    pub(crate) pressed: bool,
}

impl ControlSlot {
    /// The slot of a control drawn in `ui`: the first crate control of a
    /// field takes its label, and with it a press on the label.
    pub(crate) fn of(ui: &Ui) -> Self {
        Self::take(ui, false)
    }

    /// The slot of a button: as [`ControlSlot::of`], but a button leaves
    /// the label to another crate control of the field (an input after a
    /// button, as `<label for=input>`), known from the previous pass.
    pub(crate) fn of_button(ui: &Ui) -> Self {
        Self::take(ui, true)
    }

    fn take(ui: &Ui, button: bool) -> Self {
        let field = context::find::<FieldCx>(ui);
        let card = context::find::<CardCx>(ui);
        let mut slot = Self {
            field,
            card,
            parent: parent(ui),
            label: None,
            pressed: false,
        };
        if let Some(f) = field {
            let learned = learned(ui, f.id);
            let take = with_children(ui, f.id, |c| {
                c.form_control |= !button;
                let take = !c.label_taken && !(button && learned.form_control);
                c.label_taken |= take;
                (take, learned.labelled || c.label_drawn)
            });
            if take.0 {
                slot.label = take.1.then_some(f.label);
                slot.pressed = ui
                    .data_mut(|d| d.remove_temp::<bool>(f.press))
                    .unwrap_or(false);
            }
        }
        slot
    }

    /// A text field after the growing part of a horizontal field asks for
    /// `width` (its default width): the part leaves it that much room from
    /// the next pass on, as the browser shrinks a `flex-1` FieldContent
    /// before a `w-full` input.
    pub(crate) fn want(&self, ui: &Ui, width: f32) {
        if let Some(f) = self.field {
            if width.is_finite() {
                with_children(ui, f.id, |c| {
                    if c.grower_right.is_some() {
                        c.wanted += f.gap + width;
                    }
                });
            }
        }
    }

    /// `aria-invalid`: the control's own setting, else the field's.
    pub(crate) fn invalid(&self, own: Option<bool>) -> bool {
        own.unwrap_or_else(|| self.field.is_some_and(|f| f.invalid))
    }

    /// The control fills the field's width (`*:w-full` of a vertical
    /// field).
    pub(crate) fn fills(&self) -> bool {
        self.field.is_some_and(|f| f.fills)
    }

    /// The bits a control's recipe reads from the field and the card
    /// around it: `HAS_FIELD_CONTENT` (a horizontal field with a
    /// FieldContent), `HAS_FOCUS_VISIBLE` (the card shows keyboard focus).
    pub(crate) fn state(&self) -> State {
        self.field.map_or(State::NONE, |f| f.control)
            | self.card.map_or(State::NONE, |c| {
                c.state.without(c.state.without(State::HAS_FOCUS_VISIBLE))
            })
    }

    /// After the control drew as `resp` in state `st`: name it by the
    /// field's label, report its checked, focus and disabled state to a
    /// choice card, and, for a `peer` (checkbox, switch), its disabled
    /// state to the label after it.
    pub(crate) fn done(&self, ui: &Ui, resp: &Response, st: State, enabled: bool, peer: bool) {
        if let Some(label) = self.label {
            let _ = resp.clone().labelled_by(label);
        }
        if let Some(card) = self.card {
            let flags = State::NONE
                .with(State::HAS_CHECKED, st.contains(State::CHECKED))
                .with(State::HAS_FOCUS_VISIBLE, st.contains(State::FOCUS_VISIBLE))
                .with(State::HAS_DISABLED, !enabled);
            report(ui, card.id, flags);
        }
        if peer && !enabled {
            if let Some(p) = self.parent {
                with_children(ui, p.id, |c| c.peer_disabled = true);
            }
        }
    }
}

/// A crate [`Label`](crate::components::label::Label) placed directly in
/// a field (upstream's demos write `<Label for=..>` there): sense its
/// `rect` as the field's label, so it names the field's first crate
/// control when it is the field's first label, and a press on it acts on
/// that control. `None` outside a field.
pub(crate) fn label_in_field(ui: &Ui, rect: Rect) -> Option<Response> {
    let p = parent(ui).filter(|p| matches!(p.kind, Kind::Field | Kind::Content))?;
    let f = context::find::<FieldCx>(ui)?;
    let first = !with_children(ui, f.id, |c| std::mem::replace(&mut c.label_drawn, true));
    let id = if first {
        f.label
    } else {
        ui.next_auto_id().with((p.id, "shadcn_egui::label"))
    };
    let resp = ui.interact(rect, id, label_sense());
    if resp.clicked() {
        press(ui, f.press);
    }
    Some(resp)
}

/// A widget with a horizontal `data-orientation` (a slider, a separator)
/// tells the field around it (`group-has-data-horizontal/field:`).
pub(crate) fn report_horizontal(ui: &Ui) {
    if let Some(f) = context::find::<FieldCx>(ui) {
        report(ui, f.id, State::HAS_HORIZONTAL);
    }
}

// ---------------------------------------------------------------------------
// Field

/// A shadcn field (`role="group"`): a label, a control, a description and
/// errors, stacked or in a row.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use shadcn_egui::components::field::Field;
/// use shadcn_egui::components::input::Input;
///
/// let mut name = String::new();
/// Field::new()
///     .label("Username")
///     .description("Choose a unique username for your account.")
///     .show(ui, |ui| ui.add(Input::new(&mut name).hint("Max Leiter")));
/// # });
/// ```
///
/// The setters draw the parts in upstream's order: the label (or title),
/// then the closure's content, then the description and the errors; in a
/// horizontal field with a label and a description they sit in an implied
/// [`FieldContent`] before the closure. Other orders call the parts in the
/// closure, in their source order: a checkbox before its label, a
/// description before the control.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use shadcn_egui::components::checkbox::Checkbox;
/// use shadcn_egui::components::field::{field_label, Field};
///
/// let mut agree = false;
/// Field::new().horizontal().show(ui, |ui| {
///     ui.add(Checkbox::new(&mut agree));
///     field_label(ui, "I agree to the terms and conditions");
/// });
/// # });
/// ```
///
/// Each widget of the closure is one child of the field, the field's gap
/// from the next: upstream's demos put the controls there. A vertical
/// field gives each child its width (`*:w-full`): a select or a button
/// fills it, and a text field fills it where it sets no width of its own.
/// In a horizontal field the label (or the [`FieldContent`]) takes the
/// room the other children leave. The field fills its `Ui`'s width; where
/// that is unbounded, it takes its content's.
///
/// `.invalid(true)` (upstream's `data-invalid="true"`) or an error turns
/// the field's text destructive and makes its crate controls invalid. A
/// disabled `Ui` around the field disables its controls and fades its
/// label and title; the description and errors stay at full opacity. A
/// field keeps what it learns about its children from one pass to the
/// next under an auto id; fields whose parts change between passes in a
/// loop need [`Field::id_salt`].
#[must_use = "Call .show(ui, |ui| ..)"]
pub struct Field {
    label: Option<WidgetText>,
    title: Option<WidgetText>,
    description: Option<WidgetText>,
    errors: Vec<WidgetText>,
    invalid: bool,
    orientation: Orientation,
    orientation_set: bool,
    responsive: bool,
    variant: FieldVariant,
    id_salt: Option<Id>,
}

impl Default for Field {
    fn default() -> Self {
        Self::new()
    }
}

impl Field {
    /// A vertical field without parts of its own.
    #[inline]
    pub fn new() -> Self {
        Self {
            label: None,
            title: None,
            description: None,
            errors: Vec::new(),
            invalid: false,
            orientation: Orientation::Vertical,
            orientation_set: false,
            responsive: false,
            variant: FieldVariant::Default,
            id_salt: None,
        }
    }

    /// The label ([`FieldLabel`]), drawn first. Its press acts on the
    /// field's first crate control, which it names.
    #[inline]
    pub fn label(mut self, text: impl Into<WidgetText>) -> Self {
        self.label = Some(text.into());
        self
    }

    /// The title ([`FieldTitle`]): a label for a control that takes none
    /// (a slider), drawn first, after a label.
    #[inline]
    pub fn title(mut self, text: impl Into<WidgetText>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// The description ([`FieldDescription`]), drawn after the closure's
    /// content.
    #[inline]
    pub fn description(mut self, text: impl Into<WidgetText>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// shadcn `data-invalid`: the field's text turns destructive, and its
    /// crate controls are invalid unless they say otherwise.
    #[inline]
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Adds an error message: one entry of upstream's `errors`, drawn
    /// last ([`FieldError::from_messages`]). Each call adds one; an error
    /// makes the field invalid, as `.invalid(true)`. One message shows as
    /// text, several as a list.
    #[inline]
    pub fn error(mut self, message: impl Into<WidgetText>) -> Self {
        self.errors.push(message.into());
        self
    }

    /// shadcn `orientation`, `Vertical` by default: the parts stacked,
    /// or in a row (`Horizontal`).
    #[inline]
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self.orientation_set = true;
        self
    }

    /// `.orientation(Orientation::Horizontal)`.
    #[inline]
    pub fn horizontal(self) -> Self {
        self.orientation(Orientation::Horizontal)
    }

    /// shadcn `orientation="responsive"`: vertical, and horizontal where
    /// the [`FieldGroup`] around it is at least 28rem (448 points) wide; a
    /// field outside a group stays vertical. With `.orientation(..)` it
    /// fails a `debug_assert!`.
    #[doc(alias = "orientation")]
    #[inline]
    pub fn responsive(mut self, responsive: bool) -> Self {
        self.responsive = responsive;
        self
    }

    /// An app variant ([`FieldVariant::app`]); the parts inside take it.
    #[inline]
    pub fn variant(mut self, variant: impl Into<FieldVariant>) -> Self {
        self.variant = variant.into();
        self
    }

    /// The id under which the field keeps what it learned about its
    /// children in the previous pass, scoped to the `Ui` (as
    /// `TextEdit::id_salt`).
    #[inline]
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    /// Show the field with `add` after its label or title, before its
    /// description and errors.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        debug_assert!(
            !(self.responsive && self.orientation_set),
            "Field: .responsive(true) picks the orientation; drop .orientation(..)"
        );
        let t = Theme::current(ui.ctx());
        let id = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => ui.next_auto_id(),
        };
        let around = parent(ui);
        let card = around
            .filter(|p| p.kind == Kind::Card)
            .and_then(|_| context::find::<CardCx>(ui));
        let orientation = if self.responsive {
            let wide = context::find::<GroupCx>(ui).is_some_and(|g| g.wide);
            if wide {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            }
        } else {
            self.orientation
        };
        let row = orientation == Orientation::Horizontal;
        let v = t.resolve(self.variant);
        let learned = learned(ui, id);
        let invalid = self.invalid || !self.errors.is_empty();
        let enabled = ui.is_enabled();
        let has_content = row && learned.presence.contains(State::HAS_FIELD_CONTENT);
        let bits = v
            .bits()
            .with(State::INVALID, invalid)
            .with(State::DISABLED, !enabled)
            .with(State::IN_FIELD_LABEL, card.is_some())
            .with(
                State::HAS_HORIZONTAL,
                learned.presence.contains(State::HAS_HORIZONTAL),
            )
            .with(State::HAS_FIELD_CONTENT, has_content);
        let st = t.state(ui.ctx(), bits);
        let layer = match orientation {
            Orientation::Horizontal => &*t.style.field_orientation_horizontal,
            Orientation::Vertical => &*t.style.field_orientation_vertical,
        };
        let layer = if self.responsive {
            &*t.style.field_orientation_responsive
        } else {
            layer
        };
        let root = [v.part(&t.style, |_, s| &s.field, |r| &r.field), layer];
        let gap = pick_len(&root, |r| &r.gap, st);
        let (pad_t, pad_r, pad_b, pad_l) = (
            pick_len(&root, |r| &r.pad_top, st),
            pick_len(&root, |r| &r.pad_right, st),
            pick_len(&root, |r| &r.pad_bottom, st),
            pick_len(&root, |r| &r.pad_left, st),
        );
        let look = TextLook::resolve(ui, &root, st);

        let bounded = ui.available_width().is_finite();
        let outer_w = layout::capped_width(ui, f32::INFINITY, pad_l + pad_r);
        let (origin, mut frame) = layout::begin_block(ui, outer_w);
        let column = Rect::from_min_max(
            egui::pos2(origin.x + pad_l, origin.y + pad_t),
            egui::pos2(
                if bounded {
                    origin.x + outer_w - pad_r
                } else {
                    f32::INFINITY
                },
                f32::INFINITY,
            ),
        );
        let press_key = card.map_or_else(|| id.with("press"), |c| c.id.with("press"));
        let cx = FieldCx {
            id,
            label: card.map_or_else(|| id.with("label"), |c| c.id),
            press: press_key,
            invalid,
            fills: !row && bounded,
            control: State::NONE.with(State::HAS_FIELD_CONTENT, has_content),
            gap,
        };
        let me = Parent {
            kind: Kind::Field,
            owner: Owner::Field(self.variant),
            id,
            state: bits,
            row,
        };
        let stale_press = ui.data(|d| d.get_temp::<bool>(press_key).is_some());
        let layout = if !row && bounded {
            Layout::top_down_justified(Align::Min)
        } else {
            Layout::top_down(Align::Min)
        };
        let mut content = frame.new_child(context::push(
            context::push(UiBuilder::new().max_rect(column).layout(layout), me),
            cx,
        ));
        let label = self.label;
        let title = self.title;
        let description = self.description;
        let errors = self.errors;
        let (inner, trailing) = content_scope(&mut content, &look, |ui| {
            ui.spacing_mut().item_spacing.y = gap;
            begin_children(ui, id);
            if row {
                layout::row_aligned(ui, gap, has_content, |ui| {
                    let implied = (label.is_some() || title.is_some()) && description.is_some();
                    let mut description = description;
                    if implied {
                        let d = description.take();
                        FieldContent::new().show(ui, |ui| {
                            heading(ui, label, title);
                            if let Some(d) = d {
                                ui.add(FieldDescription::new(d));
                            }
                        });
                    } else {
                        heading(ui, label, title);
                    }
                    let inner = add(ui);
                    if let Some(d) = description {
                        ui.add(FieldDescription::new(d));
                    }
                    if !errors.is_empty() {
                        ui.add(FieldError::from_messages(errors));
                    }
                    let right = ui.min_rect().right();
                    let grower = with_children(ui, id, |c| c.grower_right);
                    let wanted = with_children(ui, id, |c| c.wanted);
                    let trailing = grower.map_or(0.0, |g| (right - g).max(wanted));
                    (inner, trailing)
                })
            } else {
                heading(ui, label, title);
                let inner = add(ui);
                if let Some(d) = description {
                    ui.add(FieldDescription::new(d));
                }
                if !errors.is_empty() {
                    ui.add(FieldError::from_messages(errors));
                }
                (inner, 0.0)
            }
        })
        .inner;
        let _ = end_children(&content, id, trailing);
        if stale_press {
            // A press from the previous pass that no control took.
            ui.data_mut(|d| d.remove_temp::<bool>(press_key));
        }

        let used = content.min_rect();
        let height = if used.is_positive() || used.height() > 0.0 {
            (used.bottom() - column.top()).max(0.0)
        } else {
            0.0
        };
        let width = if bounded {
            outer_w
        } else {
            (used.right() - origin.x).max(pad_l) + pad_r
        };
        let outer = Rect::from_min_size(origin, egui::vec2(width, pad_t + height + pad_b));
        let response = layout::end_item(ui, frame, outer, Sense::hover());
        // A `<div role="group">` with no `aria-disabled`.
        widget_info_with_role(&response, Role::Group, || {
            WidgetInfo::new(WidgetType::Other)
        });
        never_disabled(&response.ctx, response.id);
        InnerResponse::new(inner, response)
    }
}

/// The label and title setters of a field, in that order.
fn heading(ui: &mut Ui, label: Option<WidgetText>, title: Option<WidgetText>) {
    if let Some(label) = label {
        ui.add(FieldLabel::new(label));
    }
    if let Some(title) = title {
        ui.add(FieldTitle::new(title));
    }
}

// ---------------------------------------------------------------------------
// FieldContent

/// `field-content`: a column of a label (or title) and a description,
/// beside a control in a horizontal [`Field`], which it takes the room
/// of (`flex-1`). It takes the field's variant.
#[must_use = "Call .show(ui, |ui| ..)"]
#[derive(Clone, Copy, Debug, Default)]
pub struct FieldContent {}

impl FieldContent {
    #[inline]
    pub fn new() -> Self {
        Self {}
    }

    /// Show `add` in the column, its children the content's gap apart.
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::current(ui.ctx());
        let (owner, p, st) = owner_state(ui, &t);
        let l = [part_recipe(&t, owner, Part::Content)];
        let l = [&l[0]];
        let gap = pick_len(&l, |r| &r.gap, st);
        let look = TextLook::resolve(ui, &l, st);
        let id = ui.next_auto_id().with("shadcn_egui::field::content");
        if let Some(p) = p.filter(|p| p.kind == Kind::Field) {
            report(ui, p.id, State::HAS_FIELD_CONTENT);
        }
        let me = Parent {
            kind: Kind::Content,
            owner,
            id,
            state: p.map_or(State::NONE, |p| p.state),
            row: false,
        };
        let draw = |ui: &mut Ui, width: Option<f32>| {
            let layout = Layout::top_down(Align::Min);
            let room = match width {
                Some(w) => egui::vec2(w, 0.0),
                None => egui::vec2(ui.available_width(), 0.0),
            };
            ui.allocate_ui_with_layout(room, layout, |ui| {
                let builder = context::push(UiBuilder::new(), me);
                ui.scope_builder(builder, |ui| {
                    if let Some(w) = width.filter(|w| w.is_finite()) {
                        ui.set_width(w);
                    }
                    content_scope(ui, &look, |ui| {
                        ui.spacing_mut().item_spacing.y = gap;
                        begin_children(ui, id);
                        let inner = add(ui);
                        let _ = end_children(ui, id, 0.0);
                        inner
                    })
                    .inner
                })
                .inner
            })
        };
        as_child(ui, false, |ui, _| {
            let grow = p.and_then(|p| grower_width(ui, &p, 0.0).map(|w| (p, w)));
            match grow {
                Some((p, w)) => {
                    let r = draw(ui, Some(w).filter(|w| *w > 0.0));
                    grown(ui, &p, r.response.rect.right());
                    r
                }
                None => draw(ui, None),
            }
        })
    }
}

/// `FieldContent::new().show(ui, add)`.
pub fn field_content<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    FieldContent::new().show(ui, add)
}

// ---------------------------------------------------------------------------
// Text parts

/// A text part drawn through the text path at its place in its container.
fn text_part(ui: &mut Ui, text: WidgetText, part: Part, extra: State) -> Response {
    let t = Theme::current(ui.ctx());
    let (owner, p, _) = owner_state(ui, &t);
    let l = [part_recipe(&t, owner, part)];
    let l = [&l[0]];
    let base = p.map_or(State::NONE, |p| p.state).or(extra);
    as_child(ui, false, |ui, place| {
        let st = t.state(ui.ctx(), base.or(place.state()));
        let mt = pick(&l, |r| &r.margin_top, st).unwrap_or(0.0);
        if mt != 0.0 {
            ui.add_space(mt);
        }
        let look = TextLook::resolve(ui, &l, st);
        let id = ui.next_auto_id().with("shadcn_egui::field::text");
        part_ui(ui, id, &[&l[..]], st, |ui| {
            look.label(ui, text, TextWrapMode::Wrap)
        })
    })
}

/// `field-title`: the label of a control without a label of its own (a
/// slider), at the label's size and weight. It takes the field's text
/// colour and fades in a disabled field.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct FieldTitle {
    text: WidgetText,
}

impl FieldTitle {
    #[inline]
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for FieldTitle {
    fn ui(self, ui: &mut Ui) -> Response {
        text_part(ui, self.text, Part::Title, State::NONE)
    }
}

/// `field-description`: muted helper text. Its place among its
/// container's children sets its margin: closer to what comes before it
/// as the last but one child (`nth-last-2:-mt-1`), and right below a
/// legend (`[[data-variant=legend]+&]:-mt-1.5`). In a field that holds a
/// horizontal slider or separator it wraps balanced
/// (`group-has-data-horizontal/field:text-balance`).
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct FieldDescription {
    text: WidgetText,
}

impl FieldDescription {
    #[inline]
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self { text: text.into() }
    }
}

impl Widget for FieldDescription {
    fn ui(self, ui: &mut Ui) -> Response {
        text_part(ui, self.text, Part::Description, State::NONE)
    }
}

/// `field-label`: a field's label (a Label with the field label's
/// classes). It takes the field's text colour, and fades in a disabled
/// field and after a disabled checkbox or switch (`peer-disabled:`). A
/// press on it acts on the field's first crate control: it toggles a
/// checkbox or switch, focuses a text field, opens a select and clicks a
/// button. A button leaves the label to a crate control of another kind
/// in the same field. In a horizontal field it takes the room the other
/// children leave.
///
/// [`FieldLabel::choice_card`] wraps a whole field instead.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct FieldLabel {
    text: WidgetText,
}

impl FieldLabel {
    #[inline]
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self { text: text.into() }
    }

    /// upstream's choice card: a label that wraps a [`Field`], drawn as a
    /// bordered card that a press anywhere on acts on the field's control
    /// (`FieldLabel::choice_card().show(ui, |ui| Field::new()...)`).
    #[inline]
    pub fn choice_card() -> FieldChoiceCard {
        FieldChoiceCard { id_salt: None }
    }
}

impl Widget for FieldLabel {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let (owner, p, _) = owner_state(ui, &t);
        let fl = part_recipe(&t, owner, Part::Label);
        let l = [&*t.style.label, &fl];
        let field = context::find::<FieldCx>(ui)
            .filter(|_| p.is_some_and(|p| matches!(p.kind, Kind::Field | Kind::Content)));
        let base = p.map_or(State::NONE, |p| p.state);
        let text = self.text;
        as_child(ui, false, |ui, place| {
            let disabled = !ui.is_enabled() || place.peer_disabled;
            let st = t.state(
                ui.ctx(),
                base.with(State::DISABLED, disabled || base.contains(State::DISABLED)),
            );
            let look = TextLook::resolve(ui, &l, st);
            let natural = look
                .layout(ui, text.clone(), TextWrapMode::Extend, f32::INFINITY)
                .size()
                .x;
            let grow = p.and_then(|p| grower_width(ui, &p, natural).map(|w| (p, w)));
            let room = ui.available_width();
            let width = match grow {
                Some((_, w)) => w,
                None if field.is_some_and(|f| f.fills) => room,
                None => natural.min(room),
            };
            let wrap = if room.is_finite() { width } else { natural };
            let block = look.layout(ui, text.clone(), TextWrapMode::Wrap, wrap);
            let size = egui::vec2(width.max(block.size().x).max(0.0), block.size().y);
            let (rect, _) = layout::allocate_item(ui, size, Sense::hover());
            if let Some((p, _)) = grow {
                grown(ui, &p, rect.right());
            }
            // The field's first label takes the id fixed up front, which
            // its control names.
            let id = match field {
                Some(f)
                    if !with_children(ui, f.id, |c| {
                        std::mem::replace(&mut c.label_drawn, true)
                    }) =>
                {
                    f.label
                }
                _ => ui.next_auto_id().with("shadcn_egui::field::label"),
            };
            let resp = ui.interact(rect, id, label_sense());
            if resp.clicked() {
                if let Some(f) = field {
                    press(ui, f.press);
                }
            }
            let name = text.text().to_owned();
            // A `<label>` has no disabled state; it only looks disabled.
            widget_info_never_disabled(&resp, || {
                WidgetInfo::labeled(WidgetType::Label, true, &name)
            });
            // `peer-disabled:cursor-not-allowed`.
            disabled_cursor(&resp, disabled);
            if ui.is_rect_visible(rect) {
                let painter = part_painter(ui, resp.id, &[&l[..]], st);
                block.paint(&painter, rect.min, look.resolved_color(ui));
            }
            resp
        })
    }
}

/// `field-error`: the field's error messages in destructive text, as an
/// alert (`role="alert"`, an assertive live region). [`FieldError::new`]
/// shows its text; [`FieldError::from_messages`] takes upstream's
/// `errors` list: none, or one empty message, shows nothing and takes no
/// room; one message shows as text; two or more show their non-empty
/// messages as a bulleted list (`list-disc`), an empty list included.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct FieldError {
    text: Option<WidgetText>,
    messages: Vec<WidgetText>,
}

impl FieldError {
    /// An error with `text` as its content (upstream's children).
    #[inline]
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: Some(text.into()),
            messages: Vec::new(),
        }
    }

    /// An error from a list of messages (upstream's `errors`); an empty
    /// text is an entry without a message.
    #[inline]
    pub fn from_messages<T: Into<WidgetText>>(messages: impl IntoIterator<Item = T>) -> Self {
        Self {
            text: None,
            messages: messages.into_iter().map(Into::into).collect(),
        }
    }
}

impl Widget for FieldError {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        // upstream's `hasContent`: children, else by the number of
        // entries before it drops those without a message.
        let single = match (&self.text, self.messages.len()) {
            (Some(text), _) => Some(text.clone()),
            (None, 1) => Some(self.messages[0].clone()).filter(|m| !m.is_empty()),
            _ => None,
        };
        let list = self.text.is_none() && self.messages.len() >= 2;
        if single.is_none() && !list {
            let at = Rect::from_min_size(ui.cursor().min, Vec2::ZERO);
            return ui.interact(at, ui.next_auto_id(), Sense::hover());
        }
        let (owner, p, _) = owner_state(ui, &t);
        let l = [part_recipe(&t, owner, Part::Error)];
        let l = [&l[0]];
        let base = p.map_or(State::NONE, |p| p.state);
        let id = ui.next_auto_id().with("shadcn_egui::field::error");
        let messages = self.messages;
        let response = as_child(ui, false, |ui, _| {
            let st = t.state(ui.ctx(), base);
            let look = TextLook::resolve(ui, &l, st);
            if let Some(text) = single {
                return part_ui(ui, id, &[&l[..]], st, |ui| {
                    look.label(ui, text, TextWrapMode::Wrap)
                });
            }
            // `ml-4 flex list-disc flex-col gap-1`: the markers sit in the
            // margin, before each item.
            let (indent, item_gap) = (ERROR_LIST_INDENT, ERROR_LIST_GAP);
            let start = ui.cursor().min;
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = item_gap;
                let col = Rect::from_min_max(
                    egui::pos2(start.x + indent, start.y),
                    egui::pos2(ui.max_rect().right(), f32::INFINITY),
                );
                let mut list = ui.new_child(
                    UiBuilder::new()
                        .max_rect(col)
                        .layout(Layout::top_down(Align::Min)),
                );
                part_ui(&mut list, id, &[&l[..]], st, |ui| {
                    for m in messages.into_iter().filter(|m| !m.is_empty()) {
                        let r = look.label(ui, m, TextWrapMode::Wrap);
                        let marker =
                            look.layout(ui, "\u{2022} ", TextWrapMode::Extend, f32::INFINITY);
                        let at = egui::pos2(r.rect.left() - marker.size().x, r.rect.top());
                        marker.paint(ui.painter(), at, look.resolved_color(ui));
                    }
                });
                let used = list.min_rect();
                let h = if used.is_finite() {
                    (used.bottom() - start.y).max(0.0)
                } else {
                    0.0
                };
                let w = ui.available_width().max(0.0);
                ui.allocate_exact_size(egui::vec2(w, h), Sense::hover()).1
            })
            .inner
        });
        widget_info_with_role(&response, Role::Alert, || {
            WidgetInfo::new(WidgetType::Other)
        });
        never_disabled(&response.ctx, response.id);
        response
            .ctx
            .accesskit_node_builder(response.id, |node| node.set_live(Live::Assertive));
        response
    }
}

/// `ui.add(FieldLabel::new(text))`.
pub fn field_label(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(FieldLabel::new(text))
}

/// `ui.add(FieldTitle::new(text))`.
pub fn field_title(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(FieldTitle::new(text))
}

/// `ui.add(FieldDescription::new(text))`.
pub fn field_description(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(FieldDescription::new(text))
}

/// `ui.add(FieldError::new(text))`.
pub fn field_error(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(FieldError::new(text))
}

// ---------------------------------------------------------------------------
// FieldLegend

/// `field-legend`: a [`FieldSet`]'s heading, its first child. The set's
/// gap does not follow it: its margin below does (`mb-3` in vega), as the
/// browser lays out a `<legend>` outside the fieldset's flex content.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct FieldLegend {
    text: WidgetText,
    variant: FieldLegendVariant,
}

impl FieldLegend {
    #[inline]
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            variant: FieldLegendVariant::Legend,
        }
    }

    /// shadcn `variant`: `Legend` (default) or `Label`, or an app variant
    /// ([`FieldLegendVariant::app`]).
    #[inline]
    pub fn variant(mut self, variant: impl Into<FieldLegendVariant>) -> Self {
        self.variant = variant.into();
        self
    }
}

impl Widget for FieldLegend {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let v = t.resolve(self.variant);
        let l = [v.part(&t.style, |_, s| &s.field_legend, |r| &r.field_legend)];
        let p = parent(ui);
        let in_set = p.filter(|p| p.kind == Kind::Set);
        debug_assert!(
            in_set.is_none_or(|p| with_children(ui, p.id, |c| c.index == 0 && !moved(ui, c.mark))),
            "a FieldLegend is the first child of its FieldSet"
        );
        let base = p.map_or(State::NONE, |p| p.state).without(State::DISABLED);
        let st = t.state(ui.ctx(), base.or(v.bits()));
        let legend = matches!(self.variant, FieldLegendVariant::Legend)
            || (matches!(self.variant, FieldLegendVariant::App(_))
                && v.bits().contains(State::VARIANT_LEGEND));
        let mb = pick_len(&l, |r| &r.margin_bottom, st);
        let look = TextLook::resolve(ui, &l, st);
        let id = ui.next_auto_id().with("shadcn_egui::field::legend");
        let text = self.text;
        let resp = as_child(ui, legend, |ui, _| {
            let spacing = ui.spacing().item_spacing.y;
            let r = part_ui(ui, id, &[&l[..]], st, |ui| {
                look.label(ui, text, TextWrapMode::Wrap)
            });
            // The margin below the legend replaces the set's gap.
            ui.add_space(mb - spacing);
            r
        });
        if let Some(p) = in_set {
            with_children(ui, p.id, |c| c.legend = Some(resp.id));
        }
        resp.ctx
            .accesskit_node_builder(resp.id, |node| node.set_role(Role::Legend));
        resp
    }
}

/// `ui.add(FieldLegend::new(text))`.
pub fn field_legend(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(FieldLegend::new(text))
}

// ---------------------------------------------------------------------------
// FieldSet

/// `field-set`: a `<fieldset>` of fields, an accessible group named by its
/// [`FieldLegend`]. Its legend and description setters draw first, then
/// the closure, its children the set's gap apart.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use shadcn_egui::components::field::{Field, FieldGroup, FieldSet};
/// use shadcn_egui::components::input::Input;
///
/// let mut street = String::new();
/// FieldSet::new()
///     .legend("Address Information")
///     .description("We need your address to deliver your order.")
///     .show(ui, |ui| {
///         FieldGroup::new().show(ui, |ui| {
///             Field::new()
///                 .label("Street Address")
///                 .show(ui, |ui| ui.add(Input::new(&mut street)));
///         });
///     });
/// # });
/// ```
#[must_use = "Call .show(ui, |ui| ..)"]
pub struct FieldSet {
    legend: Option<WidgetText>,
    description: Option<WidgetText>,
    variant: FieldSetVariant,
}

impl Default for FieldSet {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldSet {
    #[inline]
    pub fn new() -> Self {
        Self {
            legend: None,
            description: None,
            variant: FieldSetVariant::Default,
        }
    }

    /// The legend ([`FieldLegend`] of the `Legend` variant), drawn first.
    /// A legend of another variant is a [`FieldLegend`] the closure draws
    /// first.
    #[inline]
    pub fn legend(mut self, text: impl Into<WidgetText>) -> Self {
        self.legend = Some(text.into());
        self
    }

    /// The description ([`FieldDescription`]), drawn after the legend.
    #[inline]
    pub fn description(mut self, text: impl Into<WidgetText>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// An app variant ([`FieldSetVariant::app`]).
    #[inline]
    pub fn variant(mut self, variant: impl Into<FieldSetVariant>) -> Self {
        self.variant = variant.into();
        self
    }

    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::current(ui.ctx());
        let id = ui.next_auto_id();
        let v = t.resolve(self.variant);
        let l = [v.part(&t.style, |_, s| &s.field_set, |r| &r.field_set)];
        let bits = v.bits().with(State::DISABLED, !ui.is_enabled());
        let st = t.state(ui.ctx(), bits);
        let gap = pick_len(&l, |r| &r.gap, st);
        let me = Parent {
            kind: Kind::Set,
            owner: Owner::Set(self.variant),
            id,
            state: bits,
            row: false,
        };
        let (legend, description) = (self.legend, self.description);
        let (inner, response, children) = column_container(ui, me, gap, |ui| {
            if let Some(legend) = legend {
                ui.add(FieldLegend::new(legend));
            }
            if let Some(d) = description {
                ui.add(FieldDescription::new(d));
            }
            add(ui)
        });
        widget_info_with_role(&response, Role::Group, || {
            WidgetInfo::new(WidgetType::Other)
        });
        if let Some(legend) = children.legend {
            let _ = response.clone().labelled_by(legend);
        }
        InnerResponse::new(inner, response)
    }
}

/// A column container of the family (a set, a group): its children
/// `gap` apart, its box an accessible node around them.
fn column_container<R>(
    ui: &mut Ui,
    me: Parent,
    gap: f32,
    add: impl FnOnce(&mut Ui) -> R,
) -> (R, Response, Children) {
    let bounded = ui.available_width().is_finite();
    let outer_w = layout::capped_width(ui, f32::INFINITY, 0.0);
    let (origin, mut frame) = layout::begin_block(ui, outer_w);
    let column = Rect::from_min_max(
        origin,
        egui::pos2(
            if bounded {
                origin.x + outer_w
            } else {
                f32::INFINITY
            },
            f32::INFINITY,
        ),
    );
    let mut content = frame.new_child(context::push(
        UiBuilder::new()
            .max_rect(column)
            .layout(Layout::top_down(Align::Min)),
        me,
    ));
    content.spacing_mut().item_spacing.y = gap;
    begin_children(&content, me.id);
    let inner = add(&mut content);
    let children = end_children(&content, me.id, 0.0);
    let used = content.min_rect();
    let height = if used.is_finite() {
        (used.bottom() - origin.y).max(0.0)
    } else {
        0.0
    };
    let width = if bounded {
        outer_w
    } else {
        (used.right() - origin.x).max(0.0)
    };
    let outer = Rect::from_min_size(origin, egui::vec2(width, height));
    let response = layout::end_item(ui, frame, outer, Sense::hover());
    (inner, response, children)
}

// ---------------------------------------------------------------------------
// FieldGroup

/// `field-group`: a column of fields, sets and separators, the group's gap
/// apart (`gap-7` in vega, `gap-4` for a group directly in a group). A
/// responsive [`Field`] inside reads the group's width.
#[must_use = "Call .show(ui, |ui| ..)"]
#[derive(Clone, Copy, Debug, Default)]
pub struct FieldGroup {
    variant: FieldGroupVariant,
}

impl FieldGroup {
    #[inline]
    pub fn new() -> Self {
        Self {
            variant: FieldGroupVariant::Default,
        }
    }

    /// An app variant ([`FieldGroupVariant::app`]).
    #[inline]
    pub fn variant(mut self, variant: impl Into<FieldGroupVariant>) -> Self {
        self.variant = variant.into();
        self
    }

    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::current(ui.ctx());
        let id = ui.next_auto_id();
        let v = t.resolve(self.variant);
        let l = [v.part(&t.style, |_, s| &s.field_group, |r| &r.field_group)];
        let nested = parent(ui).is_some_and(|p| p.kind == Kind::Group);
        let bits = v
            .bits()
            .with(State::IN_FIELD_GROUP, nested)
            .with(State::DISABLED, !ui.is_enabled());
        let st = t.state(ui.ctx(), bits);
        let gap = pick_len(&l, |r| &r.gap, st);
        let me = Parent {
            kind: Kind::Group,
            owner: Owner::Group(self.variant),
            id,
            state: bits,
            row: false,
        };
        // `@container/field-group`: the group's own width.
        let wide = layout::container_at_least(ui, CONTAINER_MD);
        let (inner, response) = as_child(ui, false, |ui, _| {
            ui.scope_builder(context::push(UiBuilder::new(), GroupCx { wide }), |ui| {
                let (inner, response, _) = column_container(ui, me, gap, add);
                (inner, response)
            })
            .inner
        });
        InnerResponse::new(inner, response)
    }
}

// ---------------------------------------------------------------------------
// FieldSeparator

/// `field-separator`: a horizontal rule between the children of a
/// [`FieldGroup`], with optional text on it ([`FieldSeparator::content`]),
/// on the page's background.
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct FieldSeparator {
    content: Option<WidgetText>,
}

impl Default for FieldSeparator {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldSeparator {
    #[inline]
    pub fn new() -> Self {
        Self { content: None }
    }

    /// Text on the rule (`field-separator-content`), centred, in muted
    /// `text-sm`.
    #[inline]
    pub fn content(mut self, text: impl Into<WidgetText>) -> Self {
        self.content = Some(text.into());
        self
    }
}

impl Widget for FieldSeparator {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let (owner, p, st) = owner_state(ui, &t);
        let st = st.without(State::DISABLED);
        let l = [part_recipe(&t, owner, Part::Separator)];
        let l = [&l[0]];
        let cl = [part_recipe(&t, owner, Part::SeparatorContent)];
        let cl = [&cl[0], l[0]];
        let line = [&*t.style.separator];
        let _ = p;
        as_child(ui, false, |ui, _| {
            let mt = pick_len(&l, |r| &r.margin_top, st);
            let mb = pick_len(&l, |r| &r.margin_bottom, st);
            let look = TextLook::resolve(ui, &cl, st);
            let h = pick(&l, |r| &r.height, st).unwrap_or_else(|| look.line_height());
            ui.add_space(mt);
            let w = layout::capped_width(ui, f32::INFINITY, 0.0);
            let (rect, resp) = layout::allocate_item(ui, egui::vec2(w, h), Sense::hover());
            if ui.is_rect_visible(rect) {
                let lst = st.or(State::HORIZONTAL);
                let thick = pick(&line, |r| &r.height, lst).unwrap_or(t.metrics.border);
                let thick = line_width(thick, ui.ctx().pixels_per_point()).max(thick.min(0.0));
                if let Some(c) = pick(&line, |r| &r.bg, lst).map(|c| t.paint(c)) {
                    // `absolute inset-0 top-1/2`: the rule's top at the
                    // middle of the box.
                    let y = rect.center().y;
                    let r = Rect::from_min_max(
                        egui::pos2(rect.left(), y),
                        egui::pos2(rect.right(), y + thick),
                    );
                    part_painter(ui, resp.id, &[&line[..]], lst).rect_filled(r, 0.0, c);
                }
                if let Some(text) = self.content {
                    let (pl, pr) = (
                        pick_len(&cl, |r| &r.pad_left, st),
                        pick_len(&cl, |r| &r.pad_right, st),
                    );
                    let block = look.layout(ui, text, TextWrapMode::Wrap, (w - pl - pr).max(0.0));
                    let size = egui::vec2(block.size().x + pl + pr, block.size().y.max(h));
                    let boxr = Rect::from_center_size(rect.center(), size);
                    let painter = part_painter(ui, resp.id.with("content"), &[&cl[..]], st);
                    RecipeBox::resolve(ui.ctx(), &t, &cl, st).paint(&painter, boxr);
                    let at = egui::pos2(boxr.left() + pl, boxr.center().y - block.size().y / 2.0);
                    block.paint(&painter, at, look.resolved_color(ui));
                }
            }
            ui.add_space(mb);
            resp
        })
    }
}

/// `ui.add(FieldSeparator::new())`.
pub fn field_separator(ui: &mut Ui) -> Response {
    ui.add(FieldSeparator::new())
}

// ---------------------------------------------------------------------------
// The choice card

/// upstream's choice card ([`FieldLabel::choice_card`]): a [`FieldLabel`]
/// that wraps a [`Field`], drawn as a bordered card. A press anywhere on
/// it acts on the field's control as a press on a label does; it shows
/// the control's checked state (`has-data-checked:`) and keyboard focus
/// (`has-[:focus-visible]:`) from the previous pass, and its hover look
/// while the control is enabled.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use shadcn_egui::components::field::{Field, FieldLabel};
/// use shadcn_egui::components::switch::Switch;
///
/// let mut share = false;
/// FieldLabel::choice_card().show(ui, |ui| {
///     Field::new()
///         .horizontal()
///         .title("Share across devices")
///         .description("Focus is shared across devices.")
///         .show(ui, |ui| ui.add(Switch::new(&mut share)));
/// });
/// # });
/// ```
#[doc(alias = "FieldLabel")]
#[must_use = "Call .show(ui, |ui| ..)"]
pub struct FieldChoiceCard {
    id_salt: Option<Id>,
}

impl FieldChoiceCard {
    /// The id under which the card keeps its control's state from the
    /// previous pass, scoped to the `Ui`.
    #[inline]
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    /// Show the card around `add`, which holds the card's [`Field`].
    pub fn show<R>(self, ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let t = Theme::current(ui.ctx());
        let id = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => ui.next_auto_id(),
        };
        let (owner, p, _) = owner_state(ui, &t);
        let fl = part_recipe(&t, owner, Part::Label);
        let l = [&*t.style.label, &fl];
        let had = learned(ui, id).presence;
        let rect_key = id.with("shadcn_egui::field::card_rect");
        let before = ui.data(|d| d.get_temp::<Rect>(rect_key));
        let enabled = ui.is_enabled();
        let resp = ui.interact(before.unwrap_or(Rect::NOTHING), id, label_sense());
        if resp.clicked() {
            press(ui, id.with("press"));
        }
        let hover = enabled && resp.contains_pointer();
        let bits = p
            .map_or(State::NONE, |p| p.state)
            .without(State::DISABLED)
            .or(State::HAS_FIELD)
            .or(had)
            .with(State::HOVER, hover);
        let st = t.state(ui.ctx(), bits);
        let geometry = t.state(ui.ctx(), State::HAS_FIELD);
        let border = line_width(
            pick_len(&l, |r| &r.border_width, geometry),
            ui.ctx().pixels_per_point(),
        );
        let look = TextLook::resolve(ui, &l, st);
        let me = Parent {
            kind: Kind::Card,
            owner,
            id,
            state: bits,
            row: false,
        };
        let card = CardCx { id, state: had };
        let inner = as_child(ui, false, |ui, _| {
            let bounded = ui.available_width().is_finite();
            let outer_w = layout::capped_width(ui, f32::INFINITY, border * 2.0);
            let (origin, mut frame) = layout::begin_block(ui, outer_w);
            let back = RecipeBox::reserve(&frame);
            let column = Rect::from_min_max(
                origin + egui::vec2(border, border),
                egui::pos2(
                    if bounded {
                        origin.x + outer_w - border
                    } else {
                        f32::INFINITY
                    },
                    f32::INFINITY,
                ),
            );
            let mut content = frame.new_child(context::push(
                context::push(
                    UiBuilder::new()
                        .max_rect(column)
                        .layout(Layout::top_down(Align::Min)),
                    me,
                ),
                card,
            ));
            begin_children(&content, id);
            // A press on the card's text acts on its control, as on a
            // `<label>`: its text takes no press of its own.
            content.style_mut().interaction.selectable_labels = false;
            let inner = content_scope(&mut content, &look, add).inner;
            let _ = end_children(&content, id, 0.0);
            let used = content.min_rect();
            let h = if used.is_finite() {
                (used.bottom() - column.top()).max(0.0)
            } else {
                0.0
            };
            let w = if bounded {
                outer_w
            } else {
                (used.right() - origin.x).max(0.0) + border * 2.0
            };
            let outer = Rect::from_min_size(origin, egui::vec2(w, h + border * 2.0));
            let painter = part_painter(ui, id, &[&l[..]], st);
            painter.set(
                back,
                Shape::Vec(
                    RecipeBox::resolve(ui.ctx(), &t, &l, st)
                        .transitioned(ui.ctx(), id, &l, st)
                        .shapes_at(ui.ctx(), outer, painter.opacity()),
                ),
            );
            drop(frame);
            let _ = ui.allocate_rect(outer, Sense::hover());
            if before != Some(outer) && !ui.is_sizing_pass() {
                ui.data_mut(|d| d.insert_temp(rect_key, outer));
                ui.ctx()
                    .request_discard("field: a choice card's box changed");
            }
            inner
        });
        widget_info_never_disabled(&resp, || WidgetInfo::new(WidgetType::Label));
        InnerResponse::new(inner, resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Paint, Role as PaletteRole, Weight};

    /// An app variant's edit gets each recipes struct and changes only
    /// what it names; the rest keeps the base's values.
    #[test]
    fn edit_compiles_and_changes_its_part() {
        const NORMAL: AppVariant<FieldVariant> = FieldVariant::app("normal");
        const TIGHT: AppVariant<FieldGroupVariant> = FieldGroupVariant::app("tight");
        const TIGHT_SET: AppVariant<FieldSetVariant> = FieldSetVariant::app("tight");
        const LOUD: AppVariant<FieldLegendVariant> = FieldLegendVariant::app("loud");
        let t = Theme::vega()
            .variant(NORMAL, FieldVariant::Default, |r: &mut FieldRecipes, _| {
                r.field_label.weight.set(Weight::Normal)
            })
            .variant(
                TIGHT,
                FieldGroupVariant::Default,
                |r: &mut FieldGroupRecipes, _| r.field_group.gap.set(12.0),
            )
            .variant(
                TIGHT_SET,
                FieldSetVariant::Default,
                |r: &mut FieldSetRecipes, _| r.field_set.gap.set(12.0),
            )
            .variant(
                LOUD,
                FieldLegendVariant::Label,
                |r: &mut FieldLegendRecipes, _| {
                    r.field_legend
                        .text
                        .set(Paint::Role(PaletteRole::Primary, 1.0))
                },
            );
        let label = part_recipe(&t, Owner::Field(NORMAL.into()), Part::Label);
        assert_eq!(label.weight.get(State::NONE), Some(Weight::Normal));
        assert_eq!(
            part_recipe(&t, Owner::Field(NORMAL.into()), Part::Description),
            *t.style.field_description
        );
        let g = t.resolve(FieldGroupVariant::from(TIGHT));
        let g = g.part(&t.style, |_, s| &s.field_group, |r| &r.field_group);
        assert_eq!(g.gap.get(State::NONE), Some(12.0));
        assert_eq!(g.gap.get(State::IN_FIELD_GROUP), Some(16.0));
        let s = t.resolve(FieldSetVariant::from(TIGHT_SET));
        let s = s.part(&t.style, |_, s| &s.field_set, |r| &r.field_set);
        assert_eq!(s.gap.get(State::NONE), Some(12.0));
        let lv = t.resolve(FieldLegendVariant::from(LOUD));
        assert_eq!(lv.bits(), State::VARIANT_LABEL);
        let legend = lv.part(&t.style, |_, s| &s.field_legend, |r| &r.field_legend);
        assert_eq!(
            legend.text.get(State::NONE),
            Some(Paint::Role(PaletteRole::Primary, 1.0))
        );
        assert_eq!(legend.margin_bottom, t.style.field_legend.margin_bottom);
    }

    /// A description's position flags come from the previous pass's count.
    #[test]
    fn place_flags() {
        let p = |index, count| {
            Place {
                index,
                count,
                ..Place::default()
            }
            .state()
        };
        assert!(p(2, 3).contains(State::LAST));
        assert!(p(1, 3).contains(State::NTH_LAST_2));
        assert!(!p(0, 3).contains(State::LAST));
        assert_eq!(p(0, 0), State::NONE);
    }
}
