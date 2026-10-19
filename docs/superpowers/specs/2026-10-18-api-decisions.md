# API decisions: port all shadcn components

Stage 2 of the plan `docs/superpowers/plans/2026-10-18-port-all-components.md`.
Companion of the rules draft `docs/superpowers/specs/2026-10-18-api-rules.md`.
Every choice point (`CP-…`) of the survey
`docs/superpowers/specs/2026-10-18-component-survey.md` is covered here,
or on a `Settled because` line of the rules.

How to read an entry: **Case** is the question, **Options** the reasonable
answers, **Affects** the components, **Recommendation** the controller's
proposal and its reason, **Covers** the survey CPs the entry settles. The
line `Decision:` is the maintainer's. "Sketch" means the pick a survey
report assumed. Where an entry bundles several CPs, each item carries its
own options and recommendation; a decision such as "accept all, except …"
settles the bundle.

How to answer: the crate-wide entries (D-1 to D-102, D-127 onward) are
decided now, because the 0.7.0 cleanup applies them. For the per-item
lines of the family bundles (D-109 to D-126) and for the sub-choices of
the very-large verdicts (D-104 to D-108), "defer to the family PR" is a
valid answer (maintainer, 2026-10-18): those choices are then decided when
the family's port starts, against the upstream pin of that time. The
verdicts and dependency choices of D-103 to D-108 (port, defer, which
dependency) are decided now; each of D-104 to D-108 says which of its
parts is which.

Decided (maintainer, 2026-10-19): every `Decision:` line is filled. The
guiding principles behind the decisions are at the top of the rules
document. Where a decision differs from the recommendation, the
Recommendation text stays as written (it records the proposal), and the
Decision line says what changed; entries whose recommendation leaned on
the old answer carry the knock-on in their own Decision line.

Order: the crate-wide anchors first (D-1 to D-13; most other entries
follow from them), then the remaining crate-wide conventions by topic,
then generator work, then verdicts and new dependencies, then one bundle
per component family with the choices that touch only that family.

Existing-crate bugs are not listed: they all go into the stage 3 cleanup
(0.7.0), as the maintainer decided on 2026-10-18.

## Anchors

### D-1 · Disabled: `.enabled(bool)` on widgets, or the `Ui`

- **Case:** Button, Checkbox, Switch, Select, Slider, Input, Label and Tabs
  carry `.enabled(bool)`; egui leaf widgets have none and are disabled
  through the `Ui` (`ui.add_enabled`, `add_enabled_ui`, `ui.disable()`). The
  crate widgets already honour `ui.is_enabled()`. Second part (X2): the name
  of the per-item flag on data items that have no `Ui` of their own
  (`ToggleItem`, `SelectItem`, `Tab`, `Choice`); command items are closure
  items (D-55) and take `add_enabled_ui` (D-80).
- **Options:** leaves: (a) keep `.enabled(bool)` everywhere; (b) drop it,
  `ui.add_enabled` as in egui; (c) both. Per item: (i) `enabled(bool)`;
  (ii) `disabled(bool)` (upstream's word); (iii) none, `add_enabled_ui`
  around closure items only.
- **Affects:** every interactive leaf (14 existing, ~30 new), Label's
  "peer disabled" look, Calendar, Marker/Bubble/Attachment, item types.
- **Recommendation:** (b) with a `#[deprecated]` `.enabled` for one release
  that forwards to the same state; per item (i) `enabled(bool)`. Reason: the
  rule after this entry is "`Ui` state for anything drawn into a `Ui`;
  `.enabled(bool)` only where no `Ui` exists to disable (data items, D-80)
  or where upstream has a container prop (D-79)", egui's idiom (§Enabled
  state); `ui.add_enabled(false, Label::new(..))` gives Label's peer look
  without a second meaning of `enabled`. egui itself names the item-level
  flag `enabled` (`CollapsingHeader::enabled`, `Area::enabled`), so the
  crate keeps one word. Tabs shows the split: the widget loses `.enabled`,
  its items keep it.
- **Covers:** CP-01-1, CP-02-5, CP-05-3, CP-06-1, CP-07-1, CP-07-31, CP-08-12, CP-09-18, CP-12-7, CP-15-17, CP-18-37, CP-08-37

Decision: (b) with the `#[deprecated]` `.enabled` for one release, per item
(i) — as recommended.

### D-2 · Icon parameter form

- **Case:** `Button::icon(&Icon)` and `Badge::icon(&Icon)` copy from a
  reference, `IconView::new(&'a Icon)` borrows; `Icon` is `Copy`. Sketches
  use all three forms.
- **Options:** (a) `Icon` by value; (b) `&Icon` copied (today);
  (c) `&'a Icon` borrowed.
- **Affects:** every icon setter: Button, Badge, IconView, Item, Empty,
  Alert, Avatar, input-group button, Select/Command items, menu items,
  breadcrumb separator, sidebar, attachment, marker.
- **Recommendation:** (a). Reason: `Icon` is `Copy` and small, by value
  removes lifetimes from builders, and egui passes `Copy` options by value
  (`Color32`, `Vec2`). `IconView::new` changes too.
- **Covers:** CP-01-3, CP-02-8, CP-02-28, CP-03-34, CP-04-31, CP-04-32, CP-06-36, CP-08-36, CP-09-35, CP-10-26, CP-13-33, CP-14-45, CP-18-33

Decision: (a) — as recommended.

### D-3 · Boolean setter form

- **Case:** sketches mix `.vertical()` / `.icon_only()` / `.circle()`
  (no argument) with `.invalid(bool)` / `.expanded(bool)` / `.link(bool)`;
  default-true flags (`numbers`, `viewport`, `close_on_select`) need a form
  too (see D-16).
- **Options:** (a) no-argument setters for named modes, `bool` for states
  (sketch); (b) `flag(bool)` everywhere; (c) no-argument everywhere;
  (d) `flag(bool)` for every flag, and a no-argument setter only as a
  shortcut for one value of an enum setter (`.vertical()` =
  `.orientation(Orientation::Vertical)`); (e) both forms for every flag
  (CP-01-31's "both": a no-argument setter plus a `bool` one, which Rust
  can only spell with two names such as `.vertical()` and
  `.set_vertical(bool)`), rejected below.
- **Affects:** every builder.
- **Recommendation:** (d). Reason: one predictable rule. egui itself mixes
  the forms: some no-argument setters are mode shortcuts
  (`Slider::vertical`, `Separator::vertical`, `ScrollArea::vertical`,
  `Label::wrap`), others are plain flags or presets (`Button::small`,
  `Slider::integer`, `TextEdit::code_editor`, `UiBuilder::disabled`,
  `sizing_pass`), and its flags with a value take a `bool`
  (`Button::selected`, `TextEdit::password`). So (a) is as egui-like as (d);
  (d) wins on consistency, not on egui fidelity. A caller who computes a
  flag passes it straight; the "mode or state" line of (a) is a judgement
  each port would draw differently; (e) doubles every setter name.
- **Covers:** CP-01-31, CP-02-29, CP-03-35, CP-04-33, CP-06-38, CP-07-30, CP-09-36, CP-11-36, CP-12-31, CP-13-34, CP-13-35, CP-14-44, CP-15-37, CP-16-39, CP-16-42, CP-16-43, CP-17-15, CP-18-34, CP-19-37

Decision: (d) — as recommended.

### D-4 · Name of the binding setter

- **Case:** state that lives in egui memory but can be bound to the
  caller's variable (overlay open flag D-8, sidebar open, resizable layout,
  calendar month, D-32) needs one setter name.
- **Options:** (a) `.open_bool(&mut bool)` (egui `Popup::open_bool`);
  (b) `.open(&mut bool)` (egui `Window::open`); (c) the state's own name
  per kind: `.open(&mut bool)`, `.layout(&mut Vec<f32>)`,
  `.view_month(&mut Date)`; (d) one generic `.bind(&mut T)`.
- **Affects:** popover, hover-card, menus, select, sidebar, resizable,
  calendar.
- **Recommendation:** (c). Reason: it reads as what it binds, matches
  `Window::open`, and the crate has no forced `open(bool)` that would clash
  (egui only says `open_bool` because `Popup::open(bool)` was taken).
  `.bind` hides the type. Module docs must say that the crate's `.open(&mut
  bool)` binds, while egui's `Popup::open(bool)` and
  `CollapsingHeader::open(Option<bool>)` force the state for one frame; the
  types differ, so a mix-up does not compile. The setter is for optional
  bindings; a required binding is a constructor argument, as in egui
  (D-31 (c1)).
- **Covers:** CP-14-43

Decision: (c) — as recommended.

### D-5 · Icon slots: new slots, and per shape or per role

- **Case:** `IconSlots` holds `check`, `chevron_down`, `close`. Checkbox
  indeterminate, input-otp separator, menus, accordion, breadcrumb,
  pagination, calendar, carousel, sidebar, sonner, message-scroller need
  more icons that differ per icon library. Sub-question (CP-12-33): name a
  slot after its shape (`chevron_left`) or its role
  (`accordion_closed`, `toast_error`).
- **Options:** (a) new slots through icongen, `IconSlots`
  `#[non_exhaustive]`; (b) paint simple shapes (minus line, chevrons) in
  code; (c) lucide only for the new icons. Naming: (i) per shape; (ii) per
  role.
- **Affects:** the components above, `shadcn-egui-core`, icongen, all icon
  crates.
- **Recommendation:** (a) with (i): slots `minus`, `chevron_up`,
  `chevron_left`, `chevron_right`, `more_horizontal`, `loader`,
  `panel_left` (sidebar), `arrow_down`, `circle_check`, `info`,
  `triangle_alert`, `octagon_x`, plus the plain down chevron of D-94; the
  radio dot stays painted (upstream draws a filled circle, CP-05-18) and
  the questionnaire check reuses `check`. Cost of (i): libraries do not
  always use one icon per shape. In hugeicons, calendar's prev/next use
  `ArrowLeftIcon`/`ArrowRightIcon`, while pagination, carousel and the
  breadcrumb separator use `ArrowLeft01Icon`/`ArrowRight01Icon`; one
  `chevron_left` slot (filled with `ArrowLeft01Icon`, the majority) makes
  the hugeicons calendar differ from upstream. Option (iii): shape slots,
  plus a role slot where a library differs (`calendar_prev`,
  `calendar_next`). Recommendation stays (i), accepting the one hugeicons
  calendar difference and naming it in the calendar's module docs.
  Reason: icon libraries name shapes, and most components that share a
  shape share the icon; role slots for every component would multiply the
  slot list. Painting would break the per-library look the crate promises.
  The library-specific names are icongen's table, not API.
- **Covers:** CP-05-6, CP-06-37, CP-12-21, CP-12-33, CP-13-31, CP-14-47, CP-15-36, CP-16-41, CP-16-44, CP-19-39, CP-19-42, CP-05-18

Decision: (a) with (iii) — differs from the recommendation (maintainer, P4:
upstream's icon in every library). The shape slots as listed, plus a
role-named slot wherever an icon library uses another icon for a role than
for the shared shape (`calendar_prev`/`calendar_next`, filled with
hugeicons' `ArrowLeftIcon`/`ArrowRightIcon`, and each further case icongen's
table finds), so every library shows upstream's icon in every component; the
hugeicons calendar difference and its module-docs note fall away. The plain
down chevron is D-94's new slot.

### D-6 · Orientation type

- **Case:** `SliderOrientation` (shadowing `egui::SliderOrientation`) and
  `TabsOrientation` express one idea; button-group, toggle-group,
  separator, field, resizable, carousel and attachment add more.
- **Options:** (a) per-widget enums with `.vertical()`; (b) one crate
  `Orientation { Horizontal, Vertical }`; (c) reuse
  `egui::SliderOrientation`.
- **Affects:** the nine components named, the two existing enums
  (deprecated type aliases for one release).
- **Recommendation:** (b), with `.orientation(Orientation)` plus the
  `.vertical()` shortcut (D-3); Field's responsive mode is a separate flag
  (D-69), not a third variant. Reason: the anti-goal "APIs that differ
  between components for the same idea"; egui has no shared type, and its
  slider-named one would read wrong on a tab list.
- **Covers:** CP-05-37, CP-01-9, CP-01-30, CP-04-20, CP-07-29, CP-12-5, CP-14-36, CP-16-40, CP-18-5

Decision: (b) — as recommended.

### D-7 · Side and align of anchored overlays

- **Case:** bits-ui has `side` + `align` (+ `sideOffset`, `alignOffset`);
  egui has `RectAlign`, whose twelve constants map one to one.
- **Options:** (a) `.align(RectAlign)` (egui `Popup::align`); (b) separate
  crate `Side` and `Align` enums; (c) both.
- **Affects:** popover, hover-card, tooltip, menus, sidebar's collapsed
  tooltip, any anchored overlay that exposes placement (D-36).
- **Recommendation:** (a), plus `.gap(f32)` for the side offset as egui
  names it. Reason: the egui idiom (§Anchoring) maps exactly; a second
  vocabulary adds nothing.
- **Covers:** CP-10-1, CP-09-5, CP-14-23

Decision: (a) with `.gap(f32)` — as recommended.

### D-8 · Open state of anchored overlays

- **Case:** Popover keeps its open flag in egui memory only; Dialog takes
  the caller's `&mut bool`. Sub-questions: (CP-T2-1) do select,
  native-select, menubar and navigation-menu join the rule; (CP-T2-6) do
  hover overlays (hover-card, tooltip) take the binding.
- **Options:** (a) memory only; (b) memory, with an optional binding setter
  (name per D-4); (c) the caller's `&mut bool` always.
- **Affects:** popover, hover-card, tooltip, dropdown-menu, context-menu,
  menubar, select, native-select, navigation-menu.
- **Recommendation:** (b) for popover, hover-card, dropdown-menu,
  context-menu, select and native-select; memory only for tooltip (its open
  state is egui's per-frame decision under D-130 (a), or the crate's hover
  timer under D-130 (b); no caller binding either way) and for the "which
  item is open" state of menubar and navigation-menu (one value per bar,
  D-32 row "which item"). Reason: egui's form 2 plus `Popup::open_bool`
  (§Open state); it keeps the one-liner and lets an app open a popover from
  code, which upstream's `bind:open` allows. Where the open flag is stored
  (egui's popup memory keeps one open popup per viewport, so nesting breaks)
  is the separate entry D-128; this entry's (b) holds under either answer.
- **Covers:** CP-10-2, CP-09-4, CP-08-7, CP-13-2, CP-T2-1, CP-T2-6

Decision: (b) — as recommended. Tooltip's open state is the crate's hover
timer (D-130 (b)).

### D-9 · What an overlay's `show` returns

- **Case:** Popover and Dialog return `Option<R>`; egui `Popup::show`
  returns `Option<InnerResponse<R>>`, `menu_button` returns
  `InnerResponse<Option<R>>`. CP-10-25 asks whether menus follow the
  popover shape.
- **Options:** (a) `Option<R>` (today); (b) `Option<InnerResponse<R>>`
  everywhere; (c) as egui splits it: a builder attached to an existing
  trigger `Response` returns `Option<InnerResponse<R>>`, a call that draws
  its own trigger returns `InnerResponse<Option<R>>` with the trigger's
  `Response`.
- **Affects:** popover, hover-card, tooltip builder, menus, menubar menus,
  navigation-menu items, dialog family, CommandDialog.
- **Recommendation:** (c). Reason: it is egui's own split (`Popup::show` vs
  `menu_button`), gives the panel `Response` the caller needs for hover text
  and rects, and the trigger `Response` is needed exactly when the call drew
  the trigger. Modals use `Option<InnerResponse<R>>`. The modal return
  depends on D-31: under its (a) to (c) modals return
  `Option<InnerResponse<R>>`; under its (d) they return an output with
  `should_close()`.
- **Covers:** CP-10-3, CP-09-3, CP-11-2, CP-13-32, CP-10-25, CP-T5-3

Decision: (c) — as recommended. Modals return `Option<InnerResponse<R>>`
(D-31 (c1)).

### D-10 · Return type of value shortcuts

- **Case:** `checkbox` and `toggle` return `Response`; `select` and
  `tab_bar` return `bool`. Pagination, calendar and range-calendar
  shortcuts were sketched with `bool`; breadcrumb with `Option<usize>`.
- **Options:** (a) `Response` with `changed()`; (b) `bool`; sub-question:
  breadcrumb (i) `Option<usize>` as an exception, (ii) `Response`,
  (iii) no breadcrumb shortcut.
- **Affects:** select, tabs (`tab_bar` renamed `tabs` with a deprecated
  alias), pagination, calendar, range-calendar, breadcrumb (table gets no
  shortcut, D-90).
- **Recommendation:** (a); breadcrumb (iii): its builder is short, and an
  exception would be the only shortcut that does not return `Response`.
  Reason: every egui `Ui` shortcut returns `Response` (§Response methods);
  `changed()` carries the `bool`.
- **Covers:** CP-08-13, CP-05-5, CP-12-10, CP-15-21, CP-15-33, CP-13-23, CP-13-18

Decision: (a), breadcrumb (iii) — as recommended.

### D-11 · `State` flag budget

- **Case:** `State(pub u32)` has 9 free bits; the new components ask for
  about 30 (side flags, `OPEN`, `HAS_MEDIA`, calendar day flags, sidebar
  modes, chat flags). The tuple field is public, so widening breaks code
  that writes `State(..)`.
- **Options:** (a) widen to `u64` (breaking, once, in 0.7.0); (b) reuse
  flags across meanings; (c) resolve variants in code by stacking recipes;
  (d) a wider bit set (`u128` or a small bit array) behind a private field,
  so the width can grow again without a break.
- **Affects:** the style module, stylegen, every family from F3 on.
- **Recommendation:** (d) now: a `u128` behind a private field with
  constructors, so no flag count needs tracking and a later change is not a
  break (the pushback review's proposal; (a) with the same private field is
  the smaller step). Reason: (b) makes one flag mean two things in the
  generator (see D-92); (c) moves style decisions into code, which the
  porting standard forbids ("no style values in code"). With (d) (or (a)),
  each per-component flag CP below becomes "new flag as sketched", except
  where D-91 (`data-open`) and D-92 (`SELECTED`, `PRESSED`) say otherwise.
  Headroom: 23 bits are used today (bits 0-12 and 16-25). The flags the
  recommended entries add are about 36 (sidebar 8, calendar 8, questionnaire
  and chat 5, `SIDE_*` 4 (D-93), group position 4 (D-134), navigation 2,
  scroller 2, `HAS_MEDIA`, `PRESSED`, `LINK`), about 59 of 128 under (d)
  (59 of 64 under (a), where a count past 64 would move to (d) without a
  break because the field is private).
- **Covers:** CP-18-30, CP-11-38, CP-12-34, CP-13-36, CP-14-48, CP-15-38, CP-19-40, CP-19-41, CP-19-36

Decision: (d) — as recommended. The headcount leaves out the scroll-area
recipe states (scrollbar orientation, hover) that D-62 (a) adds; the 128-bit
budget has room, and F12 counts them.

### D-12 · Root id of a stateful component

- **Case:** sketches split between a required `id_salt` in the constructor
  (radio-group, select, native-select, command, modals, navigation-menu,
  sidebar, resizable, table, carousel, chart, message-scroller) and an
  auto id with an optional `.id_salt` (tabs, accordion, inputs, menubar,
  popover, calendars, attachment, questionnaire).
- **Options:** (a) required wherever the widget keeps state;
  (b) auto id plus optional `.id_salt` everywhere; (c) required where the
  egui counterpart requires one (`ComboBox` → select, native-select;
  `Modal`/`Area`/`Panel` → modals, sidebar, toaster), auto plus optional
  `.id_salt` otherwise.
- **Affects:** every stateful component; shortcuts take an id argument
  exactly when the builder requires one (CP-T10-7).
- **Recommendation:** (c). Reason: egui 0.34's in-flow builders make the
  salt optional (`TextEdit`, `ScrollArea`, `CollapsingHeader`), floating
  containers require an `Id` (`Area::new`, `Modal::new`, `Panel::left`), and
  `ComboBox` and `Window` require an id source (a salt, or the label or
  title, which doubles as one); following egui makes the rule predictable
  for egui users. Radio-group, command, table, carousel, resizable,
  message-scroller and navigation-menu move to the optional form. Chart
  keeps a required id because `egui_plot::Plot::new(id)` requires one
  (D-104). Popover, hover-card, tooltip and menus need no root id: their
  popup id derives from the trigger (D-65).
- **Covers:** CP-T6-1, CP-05-12, CP-08-10, CP-07-32, CP-09-30, CP-12-11, CP-15-12, CP-18-11, CP-T10-7

Decision: (c) — as recommended.

### D-13 · What an in-flow container's `show` returns

- **Case:** `Card::show`, `layout::row` and others return `R`; egui
  layout helpers and `Frame::show` return `InnerResponse<R>`.
- **Options:** (a) `InnerResponse<R>`; (b) `R` (today); (c) an own output
  struct only where extra data exists, `InnerResponse<R>` otherwise.
- **Affects:** card, alert, item, empty, attachment, field, button-group,
  radio-group, message, bubble, every `show` and every container part fn.
- **Recommendation:** (c), which is (a) for every container listed here;
  output structs are D-60. Under D-47 (d6), footers are part fns inside the
  closure, so `show` returns the `InnerResponse` directly. Only if D-47
  takes (d4) does a footer-capable container return an intermediate value
  from `show`, and then every call ends with `.footer(ui, ..)` or
  `.end(ui)`, which returns the `InnerResponse`; under D-47 (d3) a begin/end
  handle does the same at `.end(ui)`. Reason: egui's container idiom
  (§Containers); the dropped `Response` is what callers need for hover text
  and rects.
- **Covers:** CP-T5-2, CP-03-1, CP-04-2, CP-18-10

Decision: (c) — as recommended.

## Conventions

### D-14 · `#[must_use]`, `#[inline]`, `Default` derives

- **Case:** no crate type carries `#[must_use]`; some sketches put
  `#[inline]` on setters, most do not; 15 sketched enums lack `Default`.
- **Options:** must_use: (a) every builder, data items and sub-part
  builders included, with egui's two messages (`ui.add` / `.show()`), and a
  message naming the method where it is not `show`; (b) only types that do
  nothing unless consumed. inline: (i) every setter; (ii) none; (iii) leaf
  widgets only. Enums: (x) every public enum derives `Clone, Copy, Debug,
  Default, PartialEq, Eq` (plus `Hash` where used as a key) with a
  `#[default]` that matches upstream's default; (y) only where a builder
  needs it.
- **Affects:** every public type.
- **Recommendation:** (a), (i), and (x) narrowed: every variant and size
  enum of one component derives `Clone, Copy, Debug, Default, PartialEq, Eq`
  with upstream's default as `#[default]`; data-carrying enums (`Glyph`,
  `ToastEvent`) derive what their fields allow; a shared enum
  (`Orientation`, `Side`) derives no `Default`, because its default is per
  component (sheet Right, drawer Bottom, sidebar Left). Whether public enums
  are `#[non_exhaustive]` is D-131. Reason: egui does all three on its
  builders (§Widget builders); one crate rule is cheaper to review than a
  per-type judgement.
- **Covers:** CP-T11-1, CP-T11-2

Decision: (a), (i), (x) narrowed — as recommended.

### D-15 · Text and content parameter types

- **Case:** text parameters are `impl Into<String>`, `&str`,
  `impl Into<WidgetText>` and `impl IntoAtoms` across today's crate and the
  sketches. Sub-questions: secondary text that is also data (item labels a
  filter or typeahead reads, `alt`, error text, CP-T11-4); the empty-field
  hint name (CP-T11-5); whether a caller's `RichText` weight or colour
  beats the recipe (CP-07-4, CP-07-16).
- **Options:** (a) egui's split: titles, labels and descriptions
  `impl Into<WidgetText>`; the body of a clickable widget `impl IntoAtoms`;
  text the component also reads as data `impl Into<String>`; (b)
  `impl Into<String>` everywhere plus typed `.icon` slots (today's Button);
  (c) `impl IntoAtoms` everywhere. Hint: (i) `hint_text` for text fields,
  `placeholder` for pick lists; (ii) `hint_text` everywhere; (iii)
  `placeholder` everywhere. RichText: (x) the caller's explicit attributes
  win over the recipe; (y) the recipe wins.
- **Affects:** every component with text.
- **Recommendation:** (a), (i), (x). Clickable bodies keep the typed
  `.icon`/`.icon_end` slots as well, because the recipes read
  `ICON_START`/`ICON_END`; whether a crate `Icon` can also be an atom is
  D-58. Reason: the rule visible in egui's source (§Text); `hint_text` is
  egui's TextEdit name, `placeholder` is what a pick list shows; an
  explicit `RichText` attribute is the caller's intent, as in egui labels.
- **Covers:** CP-T10-3, CP-T11-4, CP-T11-5, CP-01-2, CP-02-1, CP-03-9, CP-04-3, CP-06-2, CP-07-4, CP-08-14, CP-08-28, CP-09-7, CP-11-4, CP-12-3, CP-12-18, CP-13-13, CP-13-20, CP-14-22, CP-16-6, CP-18-24, CP-19-2, CP-19-33, CP-07-16, CP-17-10

Decision: (a), (i), (x) — as recommended.

### D-16 · Flags whose default is `true`, and the close-button name

- **Case:** `numbers`, `prev_next`, `viewport`, `close_on_select`,
  `show_close_button`, `dismissible` default to true. Dialog and Sheet say
  `show_close_button`, Toaster `close_button`.
- **Options:** (a) positive name plus `bool`, called with `false`
  (`.viewport(false)`); (b) negative no-argument names (`.hide_numbers()`,
  `.non_modal()`); (c) per flag. Name: (i) `show_close_button` everywhere;
  (ii) `close_button` everywhere.
- **Affects:** pagination, navigation-menu, menus, dialog family, drawer,
  sonner.
- **Recommendation:** (a) and (i). Reason: follows D-3 (d); positive names
  avoid double negatives; `show_close_button` is upstream's prop name.
- **Covers:** CP-T11-10

Decision: (a), (i) — as recommended.

### D-17 · Positional `bool` parameters

- **Case:** sketches use `tab_enabled(bool, v, atoms)`,
  `row_with(selected, expanded, ..)`, `MenuButton::action(.., show_on_hover)`,
  `menu_skeleton(ui, show_icon)`, `FieldCard::new(selected)`,
  `item(.., anchor, ..)`; today `shared::box_shapes(.., clip_padding)`.
- **Options:** (a) never: chained setters or a small enum; (b) allowed
  where the value is per call and data-driven; (c) as sketched.
- **Affects:** tabs, table, sidebar, field, message-scroller,
  `components::shared`.
- **Recommendation:** (a). Reason: egui expresses options as setters
  (§Widget builders); `row_with(true, false, ..)` cannot be read at the
  call site.
- **Covers:** CP-T11-9, CP-T9-8

Decision: (a) — as recommended.

### D-18 · Names that clash

- **Case:** `toggle` is the Switch shortcut and the natural Toggle
  shortcut; crate types share egui's names (`Button`, `Slider`, `Tooltip`,
  `ScrollArea`); a shortcut may take a variant positionally
  (`badge(ui, text, variant)` today); the sidebar collapse enum was sketched
  as `Collapsible` with a `None` variant; the toast module name.
- **Options:** shortcuts: (a) switch keeps `toggle`, Toggle gets
  `toggle_button`; (b) switch becomes `switch` (deprecated `toggle` alias
  one release), Toggle gets `toggle_button`; (c) switch becomes `switch`,
  Toggle takes over `toggle` after the alias release; (d) switch becomes
  `switch`, Toggle's shortcut is `toggle_value(ui, &mut on, text)`, the name
  and signature of egui's `ui.toggle_value`, which has Toggle's meaning (a
  selectable button). egui names: (i) keep, callers use paths; (ii) keep,
  but never for a helper type with another meaning; (iii) prefix clashing
  types. Variant in a shortcut: (x) content only, variants via the builder;
  (y) positional variant. Sidebar enum: `SidebarCollapse { Offcanvas, Icon,
  Off }`. Module: `components::sonner` with `Toaster`/`Toast`/`toast()`, or
  `components::toast`.
- **Affects:** switch, toggle, radio, spinner, badge, bubble, sidebar,
  sonner, every type named like egui's.
- **Recommendation:** (d), (ii), (x), `SidebarCollapse` with `Off`,
  `components::sonner`. Reason: `switch` is the upstream name and not a Rust
  keyword; reusing `toggle` for another widget after one release would
  change a name's meaning silently, and `toggle_value` is egui's own name
  for exactly this widget (the D-28 argument); egui users expect
  `ui.button(text)` shapes; the module name matches upstream's folder.
- **Covers:** CP-T11-7, CP-T11-12, CP-T10-4, CP-02-6, CP-01-15, CP-05-22, CP-02-22, CP-14-46, CP-16-38

Decision: (d), (ii), (x), `SidebarCollapse` with `Off`, `components::sonner`
— as recommended.

### D-19 · Caller colour overrides and the colour parameter type

- **Case:** demos override colours (`badge-colors`, `alert-colors`,
  avatar badge `bg-green-600`); the spinner's default colour is
  `palette.foreground` today where CSS uses `currentColor`; setters take
  `Color32` or `impl Into<Color32>`.
- **Options:** overrides: (a) `.fill`/`.text_color` setters where a shipped
  demo overrides colours (badge, avatar badge), none elsewhere; (b) on
  every coloured widget; (c) none, recipe replacement only. Default icon
  and spinner colour: (i) the enclosing `Ui`'s text colour; (ii)
  `palette.foreground`. Type: (x) `impl Into<Color32>`; (y) `Color32`.
- **Affects:** badge, alert, avatar, spinner, `IconView`.
- **Recommendation:** (a), (i), (x). Reason: rule 5 of the porting
  standard makes demo-only values caller parameters; `currentColor` is the
  upstream semantics; egui's `Spinner::color` and `Image::tint` take
  `impl Into<Color32>`.
- **Covers:** CP-T11-11, CP-02-7, CP-04-6, CP-04-25, CP-02-20

Decision: (a), (i), (x) — as recommended.

### D-20 · Names of part functions and part types

- **Case:** `card_title`, `menu_label`, `button_group_separator` (prefixed)
  beside `dialog::footer`, `field::description`, `sidebar::input` (module
  qualified, which repeats as `separator::separator`).
- **Options:** (a) component prefix, glob-safe; (b) short names reached by
  module path, glob imports discouraged; (c) methods on a handle or the
  component.
- **Affects:** every multi-part component.
- **Recommendation:** (a). Reason: the crate's prelude and examples use
  glob imports; prefixed names never collide (`field::separator` vs
  `separator`), and today's `card_title` already does it.
- **Covers:** CP-T4-9, CP-T10-9, CP-T11-8

Decision: (a) — as recommended.

## Selection

### D-21 · Value type of a choice

- **Case:** Select and Tabs take `&mut usize` plus `&[&str]`; egui's
  `selectable_value` and `ComboBox` write any `&mut T`. Sub-question: the
  trait bound (CP-T1-2).
- **Options:** (a) generic `T: PartialEq` (+ `Clone` where the widget
  stores or copies a value); (b) index `usize`; (c) per component.
  Bound: (i) `PartialEq + Clone`; (ii) `PartialEq` only, the value moved in
  per item.
- **Affects:** select, native-select, tabs, toggle-group, radio-group,
  accordion, menu radio items, sidebar, questionnaire choices.
- **Recommendation:** (a) with (ii) where items are supplied per call
  (`radio_value` form) and (i) where the widget holds a list; questionnaire
  keeps an index (its answers are data the component validates). Reason:
  egui's selection idiom (§Selection); the caller keeps no index mapping.
  Domain types (slider `f32`, calendar `Date`, pagination page) stay
  domain types.
- **Covers:** CP-T1-1, CP-T1-2, CP-01-20, CP-05-9, CP-08-1, CP-12-1, CP-19-22

Decision: (a) with (ii) for per-call items and (i) for held lists,
questionnaire by index — as recommended.

### D-22 · "Nothing selected" and a value that matches no item

- **Case:** single-select widgets express "nothing selected" as
  `&mut Option<T>` in the signature (toggle-group, accordion) or by the
  caller choosing `T = Option<X>` (radio-group, select, tabs); a value that
  matches no item is handled differently or not stated.
- **Options:** (a) the caller's choice of `T` (`Option<X>` when it wants
  an empty state); (b) `&mut Option<T>` in every signature; (c) per
  component as upstream allows clearing. No match: (i) nothing marked,
  placeholder shown; (ii) fall back to the first item; (iii) debug assert.
- **Affects:** toggle-group, radio-group, select, native-select, tabs,
  accordion.
- **Recommendation:** (a) and (i), with one exception stated as a rule: a
  widget that can clear its value by itself takes `&mut Option<T>`, because
  clearing needs a value to write (D-23): toggle-group single, accordion
  single (both clear by a second click upstream) and calendar single (`&mut
  Option<Date>`, deselect on by default, calendar is not generic). Every
  other single-select widget leaves the choice of `T` to the caller. Reason:
  egui's `selectable_value` works on any `T`; (i) matches what upstream
  renders; tabs keep the first tab as Tab stop (porting standard).
- **Covers:** CP-T1-3, CP-T1-12, CP-01-21, CP-05-10

Decision: (a), (i), with the stated exception for widgets that clear by
themselves — as recommended.

### D-23 · Deselect by a second click

- **Case:** upstream toggle-group clears, bits Select has
  `allowDeselect` (default off), bits Calendar clears by default.
- **Options:** (a) follow each upstream default, with one setter name
  `.allow_deselect(bool)` where it is off by default and the same name
  where it is on; (b) never deselect; (c) per component as sketched
  (`prevent_deselect`, `allow_deselect`, none).
- **Affects:** toggle-group, select, calendar.
- **Recommendation:** (a): one setter, `.allow_deselect(bool)`, default as
  upstream (toggle-group true, calendar true, select false), offered only
  where the bound type can express "empty": toggle-group and calendar bind
  `&mut Option<T>` (D-22); on select the setter lives in an `impl<T>
  Select<'_, Option<T>>` block, so `Select<Option<X>>` gets it and writes
  `None`, and a plain `Select<X>` cannot misuse it. Alternative: drop it
  from select (no shadcn demo uses it). Reason: one name for one behaviour;
  the defaults follow the primitive (porting standard, behaviour); the type
  rules out a deselect with nothing to write.
- **Covers:** CP-T1-4, CP-08-5, CP-15-4

Decision: (a) — as recommended.

### D-24 · Container type of multiple selection

- **Case:** multiple mode needs a collection: `&mut Vec<T>`,
  `&mut HashSet<T>` or `&mut [bool]`.
- **Options:** (a) `&mut Vec<T>` (insertion order, upstream array);
  (b) `&mut HashSet<T>` (needs `Hash + Eq`); (c) `&mut [bool]` by index.
- **Affects:** toggle-group, accordion, chart legend, menu
  CheckboxGroup, questionnaire answers.
- **Recommendation:** (a) everywhere; the menu `CheckboxGroup` is not
  ported (callers keep one `bool` per checkbox item); the questionnaire
  keeps its own state struct (D-125). Reason: no extra bounds, the
  upstream semantics, one type to learn.
- **Covers:** CP-T1-5, CP-01-22, CP-12-15, CP-17-8, CP-09-11, CP-19-23

Decision: (a) — as recommended.

### D-25 · Spelling of single, multiple and range modes; constructor names

- **Case:** `single`/`multiple` constructors (toggle-group, accordion),
  `new`/`multiple`/`range` (calendar, slider), a `.multiple(bool)` setter
  (questionnaire), or separate types. Also alternative content as a
  constructor (`Kbd::icon`) or `new` + setter (CP-T11-6).
- **Options:** (a) `new` for the common mode, a named constructor per other
  mode; (b) a named constructor for every mode, no `new`; (c) one `new` plus
  a mode setter; (d) separate types (for calendar: `Calendar` and
  `RangeCalendar`, which rejects range-only setters at compile time).
  Alternative content: (i) named constructor; (ii) `new` + setter.
- **Affects:** toggle-group, accordion, calendar, range-calendar, slider,
  select, progress, pagination, kbd, input-group button.
- **Recommendation:** (a) and (i), a deviation from the CP-01-24 and
  CP-12-16 sketches (`single`/`multiple`): `ToggleGroup::new` and
  `Accordion::new` are single, `::multiple` the other. Mode constructors fix
  the binding type (`&mut T` vs `&mut Vec<T>`), which a setter cannot
  change; range-only setters on the shared calendar type are ignored outside
  range mode (CP-15-31), the cost of not taking (d). Reason: egui builders
  are `new` plus named alternatives (`ComboBox::from_id_salt`,
  `ScrollArea::vertical`); `from_*` names an alternative input form
  (`Progress::from_value`, `Pagination::from_count`), as egui's
  `ComboBox::from_label`.
- **Covers:** CP-T1-6, CP-T11-6, CP-01-24, CP-12-16, CP-15-2, CP-15-31

Decision: (d) for calendar — differs from the recommendation (maintainer:
own type): `Calendar` and a separate `RangeCalendar`, so a range-only setter
on a single calendar does not compile, as upstream ships range-calendar as
its own component; `Calendar::multiple` stays a mode constructor. (a) and
(i) for every other component, as recommended
(`ToggleGroup::new`/`::multiple`, `Accordion::new`/`::multiple`,
`Kbd::icon`, `from_*` for alternative input forms).

### D-26 · How a selection group gets its items

- **Case:** item data in the constructor (toggle-group, select), chained
  `.tab(value, atoms)` (tabs), an item handle in a closure (accordion), one
  `radio_value`-style call per item (radio-group, native-select, menu
  radio, sidebar), count plus index closure (carousel).
- **Options:** (a) group binding plus item data (a slice of item structs);
  (b) group binding plus chained item calls; (c) group binding plus a
  closure whose items are immediate-mode calls (`radio_value` form, items
  found through a `Ui` tag); (d) per component as sketched.
- **Affects:** toggle-group, radio-group, select, native-select, tabs,
  accordion, menus, sidebar, table rows.
- **Recommendation:** The criterion is whether the widget needs its items
  while it is closed or before it paints them: (a) item data for select and
  native-select (the closed trigger shows the selected item's text and runs
  typeahead while no list is drawn), toggle-group (roving focus, positions)
  and the tabs list; (c) immediate-mode calls for radio-group, menu radio
  items and sidebar, each passing the binding per item like `radio_value`,
  so no value travels through `Ui` tags; accordion takes the handle form
  (`Accordion::new(&mut open).show(ui, |acc| acc.item(..))`, report 12's
  sketch), because its group holds the binding and items would otherwise
  need `V: Clone + Send + Sync + 'static` to read it from a `UiStack` tag.
  Each data form has a `&[(T, text)]` shortcut. Native-select's `opt_group`
  heading then becomes group data (D-51). Reason: egui's `radio_value` run
  is the idiom for free layout; egui's `ComboBox::show_ui` closure needs a
  caller-kept `.selected_text(..)`, a second copy of the label, which item
  data avoids. Table row flags are setters on a row builder (D-17).
- **Covers:** CP-T1-7, CP-08-2, CP-07-24, CP-09-13, CP-14-8, CP-01-23, CP-05-8, CP-12-2, CP-12-17, CP-16-5

Decision: the stated criterion: (a) for select, native-select, toggle-group
and the tabs list; (c) for radio-group, menu radio items and sidebar; the
handle form for accordion — as recommended.

### D-27 · Value form of list shortcuts

- **Case:** the select shortcut keeps `&mut usize` + `&[&str]`, while tabs
  and native-select shortcuts take `&mut V` + `&[(V, &str)]`.
- **Options:** (a) index + `&[&str]` (`ComboBox::show_index`); (b) generic
  `&mut V` + `&[(V, text)]`; (c) the builder's form.
- **Affects:** select, tabs, native-select, toggle-group shortcuts.
- **Recommendation:** (b). Reason: the same value model as the builder
  (D-21); `show_index` exists in egui but is the exception. The
  `toggle_group` shortcut takes `&mut Option<V>`, as the single toggle-group
  binds (D-22).
- **Covers:** CP-T1-8, CP-T10-6

Decision: (b) — as recommended.

### D-28 · Do choice controls carry their own label

- **Case:** egui's `ui.checkbox(&mut b, text)` and
  `ui.radio_value(&mut v, x, text)` draw a label; the crate's Checkbox and
  radio item have none (upstream composes a `Label`), so the crate's
  `radio_value(ui, &mut v, x)` differs from egui's signature. Slider names
  itself with `.label(String)` while Checkbox and Switch use
  `labelled_by`. A click on a `Label` does nothing for its control.
- **Options:** (a) no own label, `Label` + `labelled_by` (upstream); the
  shortcut is renamed so it does not shadow egui's
  signature (`radio_group_item` or `radio`, the names CP-05-19 offers);
  (b) an optional label atom like egui; (c) keep `radio_value`
  with the other signature. Label click: (i) `Label` senses click and the
  caller forwards it; (ii) `Label::for_control(id)` that toggles focus;
  (iii) nothing.
- **Affects:** checkbox, switch, radio-group, slider, label.
- **Recommendation:** (b): `Checkbox::new(&mut b).label(atoms)` and
  `radio_value(ui, &mut v, x, text)` as in egui, drawn with the crate's
  `Label` recipe and linked with `labelled_by`, plus label-less use as
  today; slider's `.label` becomes `labelled_by` only; label click (i).
  Reason: matching egui's shortcut signatures avoids a trap for egui users,
  and an attached label makes the click target right without a forwarding
  mechanism.
- **Covers:** CP-T1-9, CP-T10-5, CP-05-19, CP-05-2, CP-05-20, CP-07-2, CP-05-35

Decision: (b), label click (i) — as recommended.

### D-29 · Index base

- **Case:** pagination counts pages from 1 (bits-ui, matches the label);
  every other index in the crate is 0-based.
- **Options:** (a) 1-based where the number is shown to the user;
  (b) always 0-based.
- **Affects:** pagination.
- **Recommendation:** (a). Reason: the page number is a domain value the
  user sees, not an index; converting at every call site invites
  off-by-one errors.
- **Covers:** CP-T1-10, CP-13-21

Decision: (a) — as recommended.

### D-30 · Name of the "shown as current" flag

- **Case:** when the caller owns the selection and only marks an item:
  `active` (navigation link, sidebar menu button, upstream `data-active`),
  `selected` (table row, field card, legend; egui `Button::selected`),
  `checked` (command item).
- **Options:** (a) `selected(bool)` everywhere (egui); (b) `active(bool)`
  everywhere; (c) the upstream name per component.
- **Affects:** navigation-menu, sidebar, table, field card, chart legend,
  command.
- **Recommendation:** (a), except `checked(bool)` on the command item, where
  a checkmark is drawn and the caller owns the state; the menu checkbox item
  binds `&mut bool` (D-116, CP-09-10). Reason: egui's word for "shown as
  chosen"; `checked` names a different visual.
- **Covers:** CP-T1-11, CP-T9-9

Decision: (a), `checked(bool)` on the command item — as recommended.

## Open state

### D-31 · Open state of modals

- **Case:** Dialog takes `&mut bool` in `show`; D-8 gives anchored
  overlays memory plus an optional binding. egui `Modal` leaves the open
  state to the caller.
- **Options:** (a) keep the caller's `&mut bool` argument to `show` (today's
  crate form); (b) follow D-8 (memory, optional binding, `Dialog::open(ctx,
  id)`); (c) a required binding setter, like `Window::open(&mut bool)`; (d)
  egui's `Modal` form: the caller shows the modal only while open, `show`
  returns an output with `should_close()`, and the caller clears its flag.
- **Affects:** dialog, alert-dialog, sheet, drawer, CommandDialog, the
  sidebar's mobile sheet.
- **Spelling of (c):** (c1) a constructor argument, `Dialog::new(id, &mut
  open)`; (c2) a `.open(&mut open)` setter that must be called before
  `show`. Only (c1) makes the binding required at compile time: (c2) fails
  at run time when the setter is missing, unless the builder uses
  type-state, which egui never does. (c1) follows egui's own rule: a
  required binding is a constructor argument (`Checkbox::new(&mut b)`,
  `Slider::new(&mut v, ..)`, `TextEdit::singleline(&mut s)`), an optional
  one is a setter (`Window::open`). A further variant (c3): an optional
  `.open(&mut bool)` on modals as on `Window`; without it the caller shows
  the modal only while open and reads `should_close()`, which merges (c)
  with (d) (one spelling, two return shapes).
- **Recommendation:** (c) with (c1), as both pushback reviews propose.
  Reason: the binding is required, and egui puts a required binding into
  the constructor and an optional one into a setter; under that rule,
  popovers keep the optional `.open(&mut bool)` of D-4 and modals take
  `&mut bool` in `new`. Both name the state the same way, but they are two
  spellings by design, not one; the rules state the egui rule beside D-4.
  (a), today's crate form with `&mut bool` as a `show` argument, keeps the
  one-call shape but puts the binding into `show`, while egui's bindings sit
  in the constructor or a setter; (d) is the egui idiom and the
  real alternative and changes D-9's modal return to an output with
  `should_close()`. A modal is opened by app logic, not by a trigger the
  crate can watch, so (b) does not fit.
- **Covers:** CP-11-35, CP-11-1, CP-08-24

Decision: (c) with (c1) — as recommended.

### D-32 · Who owns non-overlay state

- **Case:** besides overlays, components hold disclosure state (accordion,
  collapsible, sidebar groups), panel state (sidebar open, resizable
  layout), view state (calendar month, carousel index, scroller mode), a
  queue (sonner), search text (command), and "which item is open"
  (menubar, navigation-menu, accordion). Sketches mix caller-owned,
  memory, and memory plus binding.
- **Options:** (a) caller-owned only; (b) memory plus optional binding
  (D-4 name) for everything; (c) by kind: selection and disclosure the
  caller owns (`&mut`), panel and view state memory plus optional binding,
  transient interaction state (scroll mode, search text, toast queue,
  which menu is open) memory only.
- **Affects:** accordion, collapsible, sidebar, resizable, calendar,
  carousel, message-scroller, questionnaire, sonner, command, chart legend,
  menubar, navigation-menu.
- **Recommendation:** (c). Rows: accordion `&mut Option<V>`/`&mut Vec<V>`;
  collapsible `&mut bool` (sidebar sub-menus use the Collapsible port);
  carousel `&mut usize`; questionnaire `&mut QuestionnaireState`; chart
  legend `&mut Vec<key>` required; sidebar open, resizable layout, calendar
  month memory plus binding; sidebar sub-menus (see the trade-off below)
  memory plus binding; message-scroller, sonner queue, command search memory
  only; "which item is open" of navigation-menu an index in memory. Reason:
  caller-owned selection and disclosure match upstream's `bind:value` and
  today's Tabs and Select (report 12); this departs from egui, whose
  `CollapsingHeader` and `CollapsingState` keep disclosure in memory. Panel
  and view state follow egui's memory form with a binding (D-8); transient
  state has no caller use. One rule per kind is predictable; the sidebar
  sub-menus are the stated exception (trade-off below).
- **Trade-off (CP-14-11, CP-12-28):** with a caller-owned `&mut bool`
  per collapsible, sidebar sub-menus built from data
  (`demo-sidebar-menu-collapsible`, `items[].items`) force the caller to
  store one `bool` per sub-menu. Sub-options: (i) accept that (caller
  keeps a `Vec<bool>` or map); (ii) give Collapsible its memory option
  back (memory by id plus an optional binding, the CP-10-2 form that
  CP-12-28 rejected); (iii) the sidebar's sub-menu keeps its own open
  state in memory (CP-14-11 (a)) with an optional binding, and the plain
  Collapsible stays caller-owned. Recommendation: (iii), since a nav tree
  is panel state in this entry's terms.
- **Covers:** CP-T2-2, CP-T1-13, CP-T2-8, CP-T2-7, CP-12-14, CP-12-28, CP-14-1, CP-14-11, CP-14-33, CP-15-6, CP-16-13, CP-19-10, CP-19-24, CP-16-25, CP-08-23

Decision: (c), sidebar sub-menus (iii) — as recommended.

### D-33 · Reading and driving memory-owned state; a trigger's `expanded`

- **Case:** app code may need to query or set memory-owned state
  (popover open, sidebar toggle, scroller commands), and a trigger button
  wants `Button::expanded(open)` before the overlay is shown.
- **Options:** (a) associated functions keyed by `Id`
  (`Popover::is_open(ctx, id)`, `Sidebar::toggle(ctx, id)`), the id being
  the one the caller passed or the trigger's; (b) typed state
  `load`/`store` (egui `CollapsingState`); (c) none, use the binding.
  `expanded`: (i) the helper that owns the state sets it on its trigger;
  (ii) the caller sets it from a read accessor; (iii) both.
- **Affects:** popover, sidebar, message-scroller, menus, select,
  collapsible, navigation-menu, table rows.
- **Recommendation:** (a), keyed by the component's own id, and (iii):
  components that draw or wrap their trigger set `expanded` themselves. A
  caller-drawn trigger gets the value from the binding (`.open(&mut bool)`,
  D-8 (b)) read before the trigger is drawn: the default popup id derives
  from the trigger's `Response` (`response.id.with("popup")`,
  `Popup::default_response_id`), which exists only after the trigger is
  painted, so `Popover::is_open(ctx, id)` cannot serve a trigger that is not
  drawn yet. With `.id_salt`, a helper `Popover::id_for(ui, salt)` returns
  the id, so the caller never rebuilds the crate's id rule. The sidebar
  trigger is `sidebar_trigger(ui, id)` toggling memory.
- **Covers:** CP-T9-12, CP-T2-3, CP-T6-4, CP-10-24, CP-14-18

Decision: (a), (iii) — as recommended.

### D-34 · Initial value of memory-owned state

- **Case:** Memory-owned state (sidebar open, resizable layout, calendar
  month, overlays) needs a starting value; egui offers
  `.default_open(bool)`, upstream has `defaultOpen`/`defaultSize` props on
  some components only.
- **Options:** (a) `.default_open(bool)` (or `.default_*`) wherever state
  lives in memory (egui `CollapsingHeader::default_open`); (b) only where
  upstream has a default prop (sidebar `defaultOpen`, resizable
  `defaultSize`); (c) none.
- **Affects:** sidebar, resizable, calendar month, popover and menus.
- **Recommendation:** (b). Reason: anchored overlays always start closed
  upstream; a default setter there adds API with no upstream use.
- **Covers:** CP-T2-4

Decision: (b) — as recommended.

### D-35 · Persisting memory-owned state across restarts

- **Case:** Upstream's sidebar keeps its open state in a cookie for 7 days
  and resizable has `autoSaveId`; egui can persist memory with
  `insert_persisted`.
- **Options:** (a) persisted by default (`insert_persisted`; sidebar
  cookie upstream); (b) opt-in `.persist(bool)`; (c) temp only, the caller
  persists through the binding.
- **Affects:** sidebar, resizable.
- **Recommendation:** (b), default off. Reason: persistence writes into the
  app's egui storage, which the app should choose; upstream's `autoSaveId`
  is opt-in too. Persist only std types (`bool`, `Vec<f32>`):
  `insert_persisted` is stored only when the app enables `egui/persistence`,
  and then every persisted type must implement serde
  (`util/id_type_map.rs:37-53`), so a crate state struct without serde would
  stop compiling in such an app; the docs say `.persist` needs
  `egui/persistence`.
- **Covers:** CP-T2-5, CP-14-2, CP-14-38

Decision: (b) — as recommended.

## Overlay placement, dismissal and focus

### D-36 · Placement setters on list overlays; align offset

- **Case:** popover family exposes `.align(RectAlign)` (D-7); select,
  native-select, menubar, navigation-menu and the calendar caption are
  fixed. bits `alignOffset` (sub-menu −3, menubar −4) has no egui
  counterpart (`Popup::gap` is the main axis only).
- **Options:** (a) fixed, no setter on list pickers; (b) `.align` +
  `.gap` on every anchored overlay; (c) `.gap` only on list pickers.
  Align offset: (i) internal constants with `// behaviour-ok`;
  (ii) `.align_offset(f32)` everywhere.
- **Affects:** select, native-select, menus, navigation-menu.
- **Recommendation:** (b) and (i). Reason: one surface for every anchored
  overlay; upstream select exposes `side`/`align` too; the offsets are
  constants of the shadcn wrappers.
- **Covers:** CP-T3-1, CP-T3-2, CP-08-8

Decision: (b), (i) — as recommended.

### D-37 · Collision fallback

- **Case:** When an anchored overlay does not fit, egui `Popup` tries
  `symmetries()` and then all twelve aligns; bits-ui flips to the opposite
  side only; the chart tooltip upstream stays inside the chart.
- **Options:** (a) egui default (`symmetries()` then all twelve aligns);
  (b) opposite side only (bits `flip()`); (c) per family.
- **Affects:** every anchored overlay; chart tooltip containment.
- **Recommendation:** (b) for every anchored overlay, through
  `.align_alternatives`; chart tooltip egui default. Reason: matches the
  primitive and keeps menus from jumping to a corner.
- **Covers:** CP-T3-3, CP-10-20, CP-17-5

Decision: (b), chart tooltip on egui's default — as recommended.

### D-38 · Escape with several open overlays

- **Case:** an egui `Popup` does not consume Escape, so one press closes
  every open popup; bits closes only the top layer. Every `Popup::show`
  closes on `key_pressed(Escape)` for every close behaviour, `IgnoreClicks`
  included (`containers/popup.rs:613-616`), and `ModalResponse::should_close`
  closes a modal on Escape when no egui popup is open
  (`containers/modal.rs:151-161`), which crate flags under D-128 (ii) do not
  count. So the helper must consume Escape before the first crate overlay of
  the pass is shown; D-128 (ii+) says where it runs.
- **Options:** (a) egui default; (b) the top layer consumes Escape and
  closes alone (one shared helper).
- **Affects:** menus with sub-menus, popover inside dialog, select inside
  popover (nesting itself needs D-128 (ii), or a caller binding under (i));
  alert-dialog closes on Escape like bits.
- **Recommendation:** (b). Reason: nested menus are unusable when Escape
  closes the whole stack; one helper keeps the components consistent.
- **Covers:** CP-T3-4, CP-09-20, CP-11-13

Decision: (b) — as recommended.

### D-39 · Focus on open, on close, and Tab leaving an overlay

- **Case:** Bits-ui traps focus in modals and returns it on close, and
  returns focus for popovers; today's crate does neither, and the sketches
  answer per component (dialog both, alert-dialog Cancel first, drawer none,
  select and menus close on Tab).
- **Options:** (a) per component as sketched; (b) one rule: modals move
  focus in on open (first focusable; alert-dialog its Cancel button) and
  return it to the trigger on close; anchored overlays return focus on
  close only; Tab out of a non-modal overlay closes it; (c) egui default
  (none).
- **Affects:** dialog family, drawer (vaul: no initial focus), popover,
  menus, select, hover-card, navigation-menu.
- **Recommendation:** (b), drawer without initial focus as vaul.
  Reason: keyboard users need the return; one rule instead of five
  per-component answers; alert-dialog focuses Cancel because egui has no
  focusable panel (deviation from bits, stated).
- **Covers:** CP-T3-5, CP-T3-6, CP-08-15, CP-09-21, CP-10-7, CP-11-7, CP-11-16, CP-11-33

Decision: (b), drawer without initial focus — as recommended.

### D-40 · Hover overlays: mechanism, delays, keyboard opening

- **Case:** tooltip uses egui's global `tooltip_delay` (0.5 s, shadcn's
  provider sets 0); hover-card needs its own timer (bits 0.7 s / 0.3 s);
  navigation-menu has hover-intent timings. bits opens tooltip and
  hover-card on keyboard focus.
- **Options:** mechanism for hover-card: (a) `Popup` + own timer in
  memory; (b) egui `Tooltip` with keep-alive. Delays: (i) `.open_delay` +
  `.close_delay` on hover-card and navigation-menu, tooltip on egui's
  global delay; (ii) the same pair on all three; (iii) one `.delay`.
  Tooltip default: (x) leave egui's 0.5 s; (y) the theme sets
  `tooltip_delay = 0` as shadcn. Keyboard focus: port for both, or skip.
  Popover `openOnHover`: skip.
- **Affects:** tooltip, hover-card, navigation-menu, popover.
- **Recommendation:** (a), (ii) with the tooltip pair defaulting to the
  theme's `tooltip_delay`, (x), keyboard opening ported for both, popover
  hover skipped, hover-card stays open while the pointer is down in it.
  Reason: one timing API across hover overlays; changing egui's global
  tooltip timing would change every app tooltip, which the theme should not
  do silently. Whether tooltip keeps egui's tooltip machinery at all (and so
  whether `.open_delay` and keyboard opening exist for it) is D-130; this
  recommendation assumes D-130 (b).
- **Covers:** CP-T3-7, CP-10-16, CP-10-11, CP-13-10, CP-10-15, CP-10-6, CP-10-19, CP-10-12, CP-10-22

Decision: (a), (ii), (x), keyboard opening for both, popover hover skipped,
hover-card open while the pointer is down in it — as recommended, except the
tooltip's default delay, which differs (maintainer, P4): the crate tooltip
opens with upstream's delay, the shadcn provider's `delayDuration = 0`, on
its own timer (D-130 (b)), not with egui's `tooltip_delay`; hover-card keeps
bits' 0.7 s / 0.3 s. egui's global `tooltip_delay` stays untouched, so plain
`on_hover_text` tooltips keep egui's timing.

### D-41 · Dismissal configuration

- **Case:** Only drawer sketched dismissal setters (`dismissible`, `modal`);
  egui `Popup` has `close_behavior(PopupCloseBehavior)`; navigation-menu
  upstream does not close hover items on an outside touch.
- **Options:** (a) only where upstream has a prop (drawer `dismissible`,
  `modal`); (b) forward egui `.close_behavior(PopupCloseBehavior)` on
  anchored overlays; (c) one `.dismissible(bool)` on every overlay and
  modal. Navigation-menu hover items on touch: close on any outside click
  (egui) or copy upstream (no close).
- **Affects:** drawer, dialog family, popover, menus, navigation-menu.
- **Recommendation:** (a), drawer `dismissible(false)` blocks Escape,
  backdrop and drag; navigation-menu closes on any outside click. Reason: no
  shadcn demo needs more.
- **Covers:** CP-T3-8, CP-11-31, CP-13-11

Decision: (a), drawer `dismissible(false)` blocks Escape, backdrop and drag
— as recommended, except navigation-menu, which differs (maintainer, P4):
its hover items stay open on an outside touch, as upstream.

### D-42 · Edge side of panels

- **Case:** Sheet, drawer and sidebar sit on a window edge; sketches use
  `.side(Side)` (sheet, drawer) and `Sidebar::left/right` constructors
  (sidebar, like `egui::Panel::left`).
- **Options:** (a) one crate `Side { Top, Right, Bottom, Left }` with
  `.side(Side)` on sheet, drawer and sidebar; (b) constructors per side
  (`Sheet::left`, like `Panel::left`); (c) per-component enums;
  (d) `RectAlign`. Drawer default: Bottom (vaul) or Right (sheet).
- **Affects:** sheet, drawer, sidebar.
- **Recommendation:** (a) with defaults Right (sheet), Bottom (drawer),
  Left (sidebar). Reason: one type for one idea (D-6 logic); anchored
  overlays keep `RectAlign`, which describes a different thing.
- **Covers:** CP-11-18, CP-11-19, CP-14-13, CP-11-34

Decision: (a) with defaults Right (sheet), Bottom (drawer), Left (sidebar) —
as recommended.

### D-43 · Start/End alignment and other placement types

- **Case:** message and bubble alignment, bubble reactions,
  message-scroller button edge, carousel align, sonner position,
  input-group addon placement, `scroll_to_message` alignment.
- **Options:** (a) egui types where they fit (`egui::Align` for
  start/center/end, `Align2` for sonner's corner like `Area::anchor`);
  (b) one crate `{Start, End}`; (c) per-widget enums. Names: logical
  (`Start`/`End`, `InlineStart`) or physical (`Left`/`Right`) in an LTR-only
  crate. Bubble alignment from an enclosing Message: pushed through the
  `Ui` (setter overrides) or always explicit.
- **Affects:** message, bubble, message-scroller, carousel, sonner,
  input-group.
- **Recommendation:** (a): `egui::Align` for chat alignment, carousel and
  `scroll_to_message` (`Option<Align>`, `None` = nearest, like
  `scroll_to_me`), `Align2` for sonner; input-group keeps its own
  `AddonAlign` (four values, two axes); Message pushes its alignment to
  Bubble through a `Ui` tag. Reason: reuses egui vocabulary where egui has
  one. Note: `egui::Align` (`Min`/`Center`/`Max`, constants `LEFT`/`RIGHT`)
  and `Align2` are physical names; whether the crate accepts physical names
  or keeps a logical `{Start, End}` for a later RTL, and what the `align`
  setters are called, is D-132.
- **Covers:** CP-T7-4, CP-T7-5, CP-T3-9, CP-18-18, CP-18-15, CP-19-8, CP-06-25, CP-19-15, CP-18-16, CP-19-6

Decision: (a) — as recommended.

### D-44 · Painting outside the allocated rect

- **Case:** carousel prev/next buttons sit 48 px outside the slides, bubble
  reactions overhang the bubble.
- **Options:** (a) reserve the space inside the allocation; (b) paint
  outside as CSS does (overlaps neighbours).
- **Affects:** carousel, bubble.
- **Recommendation:** (a). Reason: egui lays widgets out by their
  allocation; painting outside overlaps the next widget with no layout
  guard.
- **Covers:** CP-16-16, CP-18-23

Decision: (a) — as recommended.

### D-45 · List keyboard model and menu machinery

- **Case:** select has its own list keyboard code; menus could use egui's
  menu machinery (`Popup::menu`, `SubMenu`, `MenuState`) or share select's
  code. Sub-questions: typeahead on a closed select trigger, 1 s typeahead
  buffer, press-drag-release selection, native-select arrow keys on a
  closed trigger.
- **Options:** (a) one shared list primitive (highlight in memory,
  focus-lock filter, typeahead, skip disabled, loop, Home/End,
  PageUp/PageDown) used by select, native-select, menus and command; menu
  and sub-menu open state is the crate's own flag of D-128 (ii), egui's menu
  machinery (`Popup::menu`, `MenuState`, `SubMenu`) is not used, because it
  rests on the single-open popup memory; (b) egui's menu machinery and focus
  movement for menus (then menus are exempt from D-128 (ii) and stay
  single-open), select keeps its own; (c) own code per component.
- **Affects:** select, native-select, dropdown-menu, context-menu,
  menubar, command.
- **Recommendation:** (a), consistent with D-128 (ii); if D-128 takes (i),
  (a) may use egui's `MenuState`/`SubMenu` for sub-menus. Typeahead as bits
  (1 s buffer, selects on a closed trigger); click to open and click to
  choose as egui `ComboBox` (no press-drag-release); native-select opens on
  Up/Down like Select. Reason: one keyboard model across pickers is the
  consistency goal; egui's focus movement lacks typeahead and Home/End.
- **Covers:** CP-09-19, CP-08-16, CP-08-17, CP-08-18, CP-07-28, CP-09-1

Decision: (a), consistent with D-128 (ii); typeahead as bits and
native-select on Up/Down — as recommended, except pointer handling, which
differs (maintainer, P4): select and menu triggers open on pointer down and
support press-drag-release selection as bits, not click to open and click to
choose as egui's `ComboBox` (D-116 CP-09-22 likewise).

## Sub-parts and compound components

### D-46 · Header parts of a multi-part component

- **Case:** title, description, label, media and action reach the
  component as builder setters (dialog family, empty, attachment, message),
  as free part fns in the body (card today, popover), or both. The card
  and alert header action follows D-48, not this entry.
- **Options:** (a) builder setters only; (b) builder setters for the
  common case plus prefixed free part fns (`card_title`) for custom
  layouts; (c) free fns only.
- **Affects:** card, item, empty, alert, field, popover, attachment,
  message, sidebar, progress label, dialog family.
- **Recommendation:** (b): the root needs the header data before it lays out
  (`has-*` recipes, gaps); the free fns let a caller build an unusual
  header. Description optional everywhere; Field gets `.title()` besides
  `.label()`; FieldContent is implied in horizontal mode; progress keeps no
  label part (svelte has none); message's empty avatar slot is
  `.avatar_space(bool)` (D-3). Reason: egui builders carry required display
  data as setters (§Widget builders); one shape for every multi-part
  component. The card and alert action follows D-48: under its (a') it is
  a part fn in the `show` closure, so the root learns of it (and of its
  `has-*` recipes) one pass late, with `request_discard` when it changes.
- **Covers:** CP-T4-1, CP-03-2, CP-03-10, CP-03-11, CP-03-23, CP-04-1, CP-07-5, CP-07-15, CP-07-19, CP-10-4, CP-18-1, CP-19-1, CP-14-5, CP-04-12, CP-19-4

Decision: (b) — as recommended.

### D-47 · What the `show` closure holds; footers and body framing

- **Case:** the closure is the main content (card, empty, message, tabs,
  dialog, field) or the slot upstream leaves free (item and attachment
  actions, alert-dialog footer, alert description). Footers are builder
  closures (card) or free fns (dialog). Body padding and scrolling of
  sheets, the card footer direction, the tabs content text size.
- **Options:** (a) the closure is always the main content; secondary slots
  (footer, actions, reactions) are builder closures stored in the builder
  (today's `Card::footer`); (b) the closure is the free slot upstream leaves
  open; (c) per component; (d) the main content closure, with secondary
  slots as a second step or a second `show` argument, egui's form
  (`CollapsingState::show_header(ui, ..).body(..)`, `Sides::show(ui, left,
  right)`). Footer direction: fixed as upstream, caller nests a layout; or a
  `footer_layout(Layout)` setter.
- **Costs:** (a) stores `Box<dyn FnOnce(&mut Ui) + 'a>` (`card.rs:63`) that
  lives while the `show` closure runs, so a body that edits `self.name` and
  a footer that reads it do not compile together (two borrows of `self`);
  the footer closure also returns nothing. Passing both closures to one call
  (`show(ui, body, footer)`, `show_with_footer`, egui's `Sides::show`) has
  the same problem: a closure captures its variables when it is created, so
  both borrows overlap for the whole call. Only forms that create the second
  closure after the first is consumed avoid it: a chained step, a begin/end
  handle, or closures that receive the shared state as an argument. A
  fourth form needs no second closure at all: the secondary slot is a part
  fn called inside the one `show` closure, as `dialog::footer(ui, ..)` is
  today (`dialog.rs:247`).
- **Limits of a chained step (second pushback review):** a modal on
  `egui::Modal` cannot take one. `Modal::show` takes the content as a
  closure and returns no handle (`containers/modal.rs:77-121`), and
  `Area::begin`, the only way to keep an area's `Ui` open after a call, is
  `pub(crate)` (`containers/area.rs:418`), so after `show` returns no
  `.footer(ui, ..)` can draw inside the panel. On an in-flow card the card's
  ring and fill must enclose the footer, so the card cannot finish before it
  knows whether a footer follows: every call, footer or not, must end with
  `.footer(ui, ..)` or `.end(ui)` (the step has no `Ui` of its own, and
  `.footer(ui, ..)` needs it again, so it cannot keep one). A forgotten
  `.end(ui)` leaves the card unpainted and unallocated, and only
  `#[must_use]` warns (`Frame::Prepared` has no `Drop`, and a `Drop` would
  have no `Ui`). Every `Card::show` call site changes. The cited model differs:
  `CollapsingState::show_header` allocates the header before it returns, and
  its body step is optional (`collapsing_header.rs:290`). Side slots (item
  and attachment actions) sit beside the main content, so a body drawn
  first takes the width before they exist; their main content is setter
  data (D-46), so their closure holds no state a second closure would share.
- **Affects:** card, item, empty, alert, alert-dialog, dialog, sheet,
  message, bubble, attachment, tabs.
- **Recommendation:** (d) in one of these forms, a real sub-choice: (d1) and
  (d2), both closures in one call (`show(ui, body, footer)` or
  `show_with_footer`), withdrawn: they keep the borrow trap (see Costs);
  (d3) a begin/end handle (`Card::new().begin(ui)` with `.body(..)`,
  `.footer(..)`, `.end(ui)`, as egui's `Frame::begin`/`end`): works for
  in-flow containers, not for modals (see Limits), amends D-13 for these
  containers, least fluent; (d4) a chained step: `show(ui, body)` returns an
  intermediate value whose `.footer(ui, f)` or `.end(ui)` draws the rest and
  gives back `InnerResponse<(R, F)>`: not buildable for the dialog family,
  and every footer-less call gains `.end(ui)` (see Limits); (d4') `show(ui,
  body)` complete on its own (`InnerResponse<R>`), plus an optional
  `.footer(ui, f)` that extends the already allocated card over its bottom
  padding and re-sets its shape slots, modals keeping the part fn: works,
  but fragile (the parent cursor and item spacing have already moved), and
  "secondary slot" then has two forms; (d5) both closures in one call, but
  each receives the shared state as an argument (`show_with_footer(ui, &mut
  state, |ui, s| .., |ui, s| ..)`), run in turn: no borrow overlap, a less
  familiar shape; (d6) a secondary slot below the body is a prefixed part fn
  called last inside the one `show` closure (`card_footer(ui, ..)`,
  `dialog_footer(ui, ..)`, `sheet_footer(ui, ..)`), upstream's structure
  (`Card.Footer` is a child of `Card.Root`) and the dialog's form today.
  Recommendation (d6), as the second pushback review proposes (it replaces
  the earlier pick (d4)). The part fn finds its container through the `Ui`
  tag (D-57) and records under the container's id in `ctx.data` that a
  footer was drawn; the root reads that after the closure and sets
  `HAS_FOOTER`, which today changes only the card's bottom padding
  (`style/nova.rs:557-559`, `style/lyra.rs:536-538`, `card.rs:102`) and is
  used after the body. The footer part paints its strip (nova) to the
  container's edges, inside the container's allocation (D-44). The sheet's
  footer part places itself at the bottom of the full-height panel. One
  closure means no borrow trap, no intermediate value and no `.end(ui)`;
  `show` returns `InnerResponse<R>` as D-13 says, and D-46 (b), D-20 and
  D-61 stay simple. Cost: the footer part must be the closure's last call,
  which the module docs say. Side slots (item and attachment actions) take
  the closure as the free slot ((b)), because their main content is setter
  data. Bubble reactions overhang the bubble's edge and the bubble reserves
  that space in its allocation (D-44), so they are setter data
  (`.reactions(..)`) the bubble lays out itself; how a click on a reaction
  comes back is left to F16. `Card::footer` keeps a `#[deprecated]` forward
  for one release. Footer direction fixed as upstream (caller nests a
  layout inside), caller pads and scrolls the sheet body as upstream demos
  do, tabs content text size applied by `show`. This changes `Card::footer`
  and the input-group's stored addon closures (D-113 CP-06-24, which
  follows this entry). Reason: one meaning of "the closure", no borrow trap,
  and the one form that works for in-flow containers and modals alike.
- **Covers:** CP-T4-2, CP-T7-7, CP-03-3, CP-03-15, CP-11-5, CP-11-10, CP-19-3, CP-11-23, CP-04-4, CP-03-16, CP-03-24, CP-18-17, CP-11-21, CP-11-22, CP-12-13

Decision: (d6) — as recommended. The sheet body that the caller scrolls uses
the crate `ScrollArea` in demos and docs (D-62 (a)).

### D-48 · Meaning of `action`, and how a secondary part's `Response` comes back

- **Case:** `action` means a closure (card, alert), an icon button (sidebar
  group and menu button), text (toast), or nothing (item and attachment put
  actions in `show`); the sidebar sketch returns the action's `Response` in
  an output struct.
- **Options:** `action`: (a) always a closure slot stored in the builder
  (`Card::new().action(|ui| ..)`); it has the borrow trap of D-47 (a): an
  action that touches state the body also edits does not compile (Compound
  rule 2); (a') a prefixed part fn inside the `show` closure
  (`card_action(ui, ..)`, `alert_action(ui, ..)`), D-47 (d6)'s form for the
  header slot: the root draws the header from setter data before the
  closure and keeps the action slot free; the part, called first, paints
  into that slot through the `Ui` tag (D-57); the root learns that an
  action exists (the card header's `has-data-[slot=card-action]` columns,
  the alert's `has-data-[slot=alert-action]:pr-18`)
  from the previous pass, with `request_discard` when it changes, as D-134;
  (b) an `Icon` (sidebar group and menu button) / text (toast) / closure
  (card, alert) per component; (c) no `action` setter, actions live in the
  `show` closure or helpers. Responses: (i) a field on an output struct
  (`MenuButtonOutput { response, action }`); (ii) the caller's own widgets
  in a closure, read directly.
- **Affects:** card, alert, alert-dialog, attachment, sidebar group and
  menu button, sonner.
- **Recommendation:** (a') with (ii) for card and alert, a real choice
  against (a): (a') keeps one closure and no borrow trap, as D-47 (d6), at
  the cost of a one-pass-late layout when an action appears or goes; (a)
  is simpler to lay out but brings back the stored closure that D-47
  rejects. Where upstream's action is a fixed icon
  button (sidebar), `action_icon(Icon)` plus an output field (i) on
  `SidebarMenuButtonOutput` (prefixed, D-20; egui has its own `MenuButton`).
  Alert-dialog keeps `alert_dialog_cancel`/`alert_dialog_action` helpers in
  its `alert_dialog_footer(ui, ..)` part. Reason: one word, one meaning, one
  closure per container; closures return widget responses the egui way.
- **Covers:** CP-T4-3, CP-T5-6, CP-18-2, CP-14-7, CP-14-9, CP-11-11

Decision: (a') with (ii) for card and alert; sidebar `action_icon` with
output field (i) — as recommended.

### D-49 · Media and image slots

- **Case:** Item and empty take `.icon`/`.image`/`.media(closure)`,
  attachment `.icon/.image/.spinner`, alert-dialog an icon only, avatar
  `new(fallback).image(src)`; image types differ (`egui::Image` vs `impl
  Into<ImageSource>`).
- **Options:** (a) `.icon(Icon)` + `.image(impl Into<ImageSource>)` +
  `.media(closure)` on every component with a media slot; (b) an enum
  `Media::{Icon, Image, Ui}`; (c) per component (avatar one widget with
  `.image`, attachment `.icon/.image/.spinner`, alert-dialog icon only).
  Avatar badge: `AvatarBadge` struct or setters on Avatar. Missing image
  loaders: egui's placeholder or an icon fallback.
- **Affects:** item, empty, avatar, attachment, alert-dialog.
- **Recommendation:** (a), image as `impl Into<ImageSource>` everywhere;
  avatar one widget with a fallback; avatar badge as an `AvatarBadge`
  struct; egui's loading/error placeholder. Reason: one slot vocabulary;
  `ImageSource` is what `egui::Image::new` takes.
- **Covers:** CP-T4-4, CP-03-14, CP-03-28, CP-04-22, CP-04-24, CP-18-3, CP-18-13, CP-11-14, CP-04-21

Decision: (a) — as recommended.

### D-50 · Layout-only groups

- **Case:** Groups that only lay out their children (kbd, bubble, message,
  attachment, button-group text and separator, item group) are sketched as
  free fns or as builders.
- **Options:** (a) a free fn `x_group(ui, |ui| ..)` when the group has no
  options, a builder `XGroup::new().show` when it has; (b) always a
  builder; (c) always a free fn.
- **Affects:** kbd, bubble, message, attachment, button-group (text,
  separator), item group.
- **Recommendation:** (a): `kbd_group`, `bubble_group`, `message_group`,
  `button_group_separator`, `button_group_text` free fns; `ItemGroup` a
  builder with an explicit size. Reason: egui has both forms
  (`ui.horizontal` beside `Frame::show`); options decide.
- **Covers:** CP-T4-6, CP-T5-8, CP-19-7, CP-02-10, CP-18-21, CP-01-10, CP-01-11, CP-03-17

Decision: (a) — as recommended.

### D-51 · Labelled groups of items

- **Case:** Select groups carry their label (`SelectGroup { label, items
  }`), menus and native-select put a heading part before the items.
- **Options:** (a) a container carrying the label (`SelectGroup { label,
  items }`, `cmd.group(heading, ..)`); (b) a heading part followed by items
  (`menu_label`, `opt_group`); (c) per component.
- **Affects:** select, command, menus, native-select, sidebar group.
- **Recommendation:** (a) for data-driven lists (select and native-select,
  whose `opt_group` becomes a group of item data, D-26) and for command, as
  the survey sketched it (`cmd.group(heading, |cmd| ..)`): the group runs
  its items first, then draws the heading above them or drops the whole
  group when none matched the search, as upstream hides empty groups (bits
  `CommandGroup.shouldRender`, `command.svelte.ts:1194-1198`) and hides
  separators while searching (`alwaysRender`). egui cannot release an
  allocation, so a heading slot allocated before the items would leave a
  gap over an empty group; the group instead runs its items in a child `Ui`
  offset by the heading height but not yet allocated in the parent
  (`ui.new_child(UiBuilder::new().max_rect(..))`), and after the closure
  allocates heading plus items when one item matched, nothing when none
  did. The item registry of D-134 (one pass late, plus `request_discard`)
  is the fallback. (b) for menus, a
  deviation from no sketch. Select separators by `.group_separators(bool)`.
  Reason: (b) on command would show headings over empty groups, a visible
  difference in the main demo.
- **Covers:** CP-T4-7, CP-08-3, CP-09-16

Decision: (a) for data lists and command, (b) for menus — as recommended.

### D-52 · What passive part functions return

- **Case:** Separators, labels and headings return `()` (button-group
  separator, menu label and separator, today's `separator`) or `Response`
  (egui `Ui::separator`, `card_title`, `item_separator`).
- **Options:** (a) `Response` (egui `Ui::separator`); (b) `()`;
  (c) `Option<Response>` when the part may paint nothing.
- **Affects:** separator, menu label and separator, button-group
  separator, sidebar separator, `card_title`, field parts.
- **Recommendation:** (a), (c) only for `field_error` with no message.
  Reason: egui's pattern; `()` drops what the caller may hover.
- **Covers:** CP-T4-8, CP-T5-11, CP-T10-2, CP-04-16

Decision: (a), (c) for `field_error` — as recommended.

### D-53 · How overlays and helpers take their trigger

- **Case:** Overlay builders take `&Response`; `tooltip(resp, text)` takes
  it by value; context-menu could be an extension trait
  (`resp.context_menu`, egui's spelling); the menu shortcut trigger was
  sketched as a crate `Button` or text.
- **Options:** (a) builders take `&Response` (`Popover::new(&resp)`), free
  helpers that decorate a trigger take it by value and return it
  (`tooltip(resp, text) -> Response`, like `on_hover_text`); (b) an
  extension trait (`resp.tooltip(text)`, `resp.toggles(&mut open)`); (c)
  free fns only. Menu shortcut trigger: `impl IntoAtoms` (egui
  `menu_button`) or a crate `Button`.
- **Affects:** popover, hover-card, tooltip, context-menu, collapsible,
  chart tooltip, dropdown and menubar shortcuts.
- **Recommendation:** (a); `ContextMenu::new(&resp)` plus
  `context_menu(&resp, ..)`; `collapsible_trigger(resp, &mut open) ->
  Response`; menu shortcuts take `impl IntoAtoms` and draw a crate button.
  Reason: no extension trait to import; by-value chaining is egui's
  `on_hover_*` form.
- **Covers:** CP-T4-10, CP-T10-10, CP-T10-8, CP-T5-5, CP-09-23, CP-10-8, CP-10-14, CP-12-29

Decision: (a) — as recommended.

### D-54 · Content form: text, closure, or both; leaf, container, or both

- **Case:** Tooltip, bubble and alert are sketched with a text form and a
  closure form, others with only one; separator is free fns today, a struct
  in the sketch.
- **Options:** (a) a component with free content offers both
  `ui.add(X::new(text))` and `X::new(..).show(ui, |ui| ..)`; (b) closure
  only; (c) text only; (d) per component.
- **Affects:** tooltip, bubble, alert, item, field, separator, tabs,
  attachment, empty.
- **Recommendation:** (a) where upstream demos use both forms (tooltip
  text and Kbd content, bubble, alert, item, field, tabs list alone or with
  content), separator as a `Separator` struct with `impl Widget` plus the
  two shortcuts. Reason: egui's leaf/two-step pair (§Widget builders).
- **Covers:** CP-T4-11, CP-T5-7, CP-10-9, CP-18-14, CP-03-12, CP-07-6, CP-04-15, CP-12-4

Decision: (a) — as recommended.

### D-55 · How non-selection collections get their items

- **Case:** Collections that are not a selection (command items, navigation
  items, breadcrumb, resizable panes, carousel slides, message-scroller
  items, avatar group, input-otp groups) are sketched with five different
  supply forms.
- **Options:** (a) immediate-mode closure, items return `Response`
  (menus); (b) a handle in the closure (`|nav| nav.item(..)`,
  `|bc| bc.link(..)`); (c) a count plus index closure (`show_rows` style);
  (d) data list.
- **Affects:** command, navigation-menu, breadcrumb, resizable, carousel,
  message-scroller, avatar group, input-otp groups.
- **Recommendation:** (b) where the parent must place or separate items
  (breadcrumb separators, navigation-menu viewport, message-scroller
  anchors), (c) for carousel and resizable panes, (a) for command, (d) for
  avatar group and input-otp `.groups(&[usize])`, resizable handles one per
  gap. Reason: each form is the one egui uses for that layout problem.
- **Covers:** CP-08-19, CP-14-32, CP-16-14, CP-19-12, CP-13-1, CP-13-16, CP-04-26, CP-06-14, CP-14-35

Decision: (b), (c), (a), (d) per layout problem as listed — as recommended.

### D-56 · A crate widget restyled by its parent

- **Case:** A button inside a button-group or input-group, input-group text,
  the sidebar's input and separator, and menu items need their parent's
  recipe; sketches use either a `Ui` tag on the plain widget or a dedicated
  type (`InputGroupButton`, `sidebar::input`).
- **Options:** (a) the plain widget reads a `Ui` tag and stacks the
  parent's recipe; (b) a dedicated type per parent (`InputGroupButton`,
  `sidebar_input`); (c) per component.
- **Affects:** button in button-group or input-group, input-group text,
  sidebar input and separator, menu items.
- **Recommendation:** (a): one `Button`, one `Input`, one `separator`;
  input-group text is a plain label styled by the addon `Ui`. Reason: fewer
  types, and egui's `UiStack` exists for exactly this.
- **Covers:** CP-T4-5, CP-06-27, CP-14-21, CP-06-26

Decision: (a) — as recommended.

### D-57 · How a part learns context from its parent

- **Case:** Parts need their parent's context (kbd inside a tooltip, card
  size in free part fns, item inside a menu, radio item in a group, sidebar
  collapse mode); today the card uses a global `ctx.data` key.
- **Options:** (a) the parent pushes a `UiStackInfo` tag, the part reads
  `ui.stack()`; (b) an explicit setter on the part; (c) the parent writes
  `Ui` data; (d) a typed handle passed to the closure.
- **Affects:** kbd in tooltip/input-group, card size, item in a menu,
  radio item in a group, menu in a menubar, sidebar parts, calendar
  background, separator default orientation.
- **Recommendation:** (a), with an explicit setter overriding where a
  caller may disagree (calendar `.background(bool)`). Separator infers its
  orientation from the `Ui` layout like egui's. Reason: egui's mechanism;
  replaces today's global `ctx.data` key for the card size (drift).
- **Covers:** CP-T9-11, CP-T7-2, CP-02-13, CP-03-7, CP-03-18, CP-05-11, CP-09-27, CP-14-6, CP-15-20

Decision: (a) — as recommended.

### D-58 · A crate `Icon` inside `IntoAtoms` content

- **Case:** Tab triggers, navigation links, alert-dialog buttons and
  native-select options take `IntoAtoms` content, but a crate `Icon` is not
  an egui atom.
- **Options:** (a) crate icons become atoms, in the form D-129 picks (a
  plain `impl From<Icon> for egui::Atom` cannot be written: the orphan rule
  forbids it in `shadcn-egui`, which owns neither type, and
  `shadcn-egui-core` must stay dependency-free); (b) `Atom::custom`
  placeholders painted by each component; (c) icons only through typed
  `.icon` slots.
- **Affects:** tab triggers, navigation links, alert-dialog buttons,
  native-select options, kbd chips, buttons in egui hosts.
- **Recommendation:** (a) with the form of D-129, typed slots kept on every
  crate widget and data item that shows icons (`ToggleItem`, `Tab`, select
  items, alert-dialog buttons), because only the typed slot gets the
  recipe's state colour. Kbd keeps `Kbd::new(text)` and `Kbd::icon(Icon)`,
  plus `paint_at` for hosts. Reason: lets crate icons go anywhere egui takes
  atoms.
- **Covers:** CP-T4-13, CP-02-12, CP-02-14

Decision: (a) in the form of D-129 — as recommended.

### D-59 · Icon slot content: `Icon`, `Glyph`, or a spinner setter

- **Case:** Upstream's loading demos put a spinner where a button, badge,
  attachment or marker icon goes; sketches use `Icon` only, a `Glyph` enum,
  or a separate `.spinner()` setter.
- **Options:** (a) `Icon` only; (b) `impl Into<Glyph>` with
  `Glyph::{Icon, Spinner}`; (c) separate `.icon` and `.spinner()` setters.
  Spinner drawing: rotate the icon set's `loader` slot, or egui's arc.
- **Affects:** button, badge, attachment, marker, spinner.
- **Recommendation:** (b), spinner drawn by rotating the `loader` slot
  icon (needs a rotation-capable `paint_icon`). Reason: one setter per
  slot; upstream's loading demos put a spinner where the icon goes.
- **Covers:** CP-01-32, CP-02-2, CP-18-26, CP-18-35, CP-02-18

Decision: (b), spinner from the rotated `loader` slot — as recommended.

## Containers and outputs

### D-60 · Output structs and their suffix

- **Case:** Leaves with more to return than a `Response` (input cursor,
  input-otp completion, questionnaire submit, resizable layout, scroller
  visibility) need an output type; sketches name them `*Output` or
  `*Response`.
- **Options:** (a) `*Output` for every own output type (egui
  `TextEditOutput`, `ScrollAreaOutput`); (b) `*Response`; (c) `*Output`
  for leaves, `*Response` for containers. Which leaves get a richer
  output: Input/Textarea `show() -> TextEditOutput`, InputOtp
  `completed`, Questionnaire `submitted`, Resizable layout and `changed`,
  MessageScroller visibility (always computed); carousel `Response` with
  `changed()`; accordion item `CollapsingResponse<R>`.
- **Recommendation:** (a), with the outputs listed. Reason: egui's newer
  types end in `Output`; `CollapsingResponse` is reused as is.
- **Affects:** input, textarea, input-otp, questionnaire, resizable,
  message-scroller, carousel, accordion.
- **Covers:** CP-T5-9, CP-T5-1, CP-14-40, CP-06-16, CP-19-28, CP-19-17, CP-06-3, CP-12-19

Decision: (a) — as recommended.

### D-61 · Container method name

- **Case:** egui names the container method `show`, `show_inside` (`Panel`),
  `show_ui` (`ComboBox`) or `ui` (`MenuBar`); the sidebar, native-select and
  menubar sketches copied those names.
- **Options:** (a) always `show`; (b) egui's name where the egui
  model differs (`show_inside` sidebar, `show_ui` native-select, `ui`
  menubar).
- **Affects:** sidebar, native-select, menubar.
- **Recommendation:** (a): the verb is `show`; secondary slots are part
  fns inside its closure (D-47 (d6)), not new verbs (under D-47 (d4) they
  would be methods on its result); a crate type that mirrors an egui type
  keeps that type's `show_*` variants (the crate `ScrollArea` of D-62 (a):
  `ScrollArea::show_rows`). Reason: crate users learn one verb; the egui
  type underneath is an implementation detail.
- **Covers:** CP-T5-4, CP-09-28

Decision: (a) — as recommended. The mirrored-type exception is live: the
crate `ScrollArea` (D-62 (a)) keeps egui's `show_rows`.

### D-62 · Scroll areas: own type, theme only, or helper

- **Case:** the theme sets no `ScrollStyle` (a cleanup bug); shadcn's
  scroll-area has a transparent track, `bg-border` thumb, hover-only
  visibility. Composites (select list, command, message-scroller, sheet
  body, attachment group) need a scroll area.
- **Options:** (a) a `shadcn_egui::ScrollArea` mirroring egui's builder,
  painting its own bars; (b) no new type: the theme styles every
  `egui::ScrollArea`; (c) a `scroll_area(ui, ..)` helper. Composites use
  the crate's type, raw `egui::ScrollArea`, or leave it to the caller.
  Axis form: egui constructors. Visibility modes: bits' four types, hover
  only, or egui's `ScrollBarVisibility`.
- **Affects:** scroll-area, select, command, message-scroller, sheet,
  attachment.
- **Costs (corrected after the second pushback review):** egui's
  `ScrollStyle` has `floating`, `foreground_color` and dormant, active and
  interact opacities for the track and the handle (`style.rs`, struct
  `ScrollStyle`); the painter multiplies the track colour
  (`extreme_bg_color`) and the handle colour by them
  (`containers/scroll_area.rs:1434-1483`). So under (b) floating bars with
  track opacities 0 give a transparent track, and a dormant handle opacity
  of 0 gives bars that show only while the pointer is over the area,
  shadcn's default `hover` type; the style can also be set for one area
  through a scoped `ui.style_mut()`. What (b) cannot reach exactly is the
  thumb colour: the handle takes `bg_fill` (the theme sets `p.secondary`,
  `theme.rs:909-916`) or, with `foreground_color`, `fg_stroke.color`
  (`p.foreground`) of the widget visuals, and buttons and text edits share
  both. `bg-border` is approximated by the foreground at a low handle
  opacity (in shadcn's dark palettes `border` is white at 10 % alpha,
  `palette/generated.rs:48`, close to the near-white foreground) or by
  `secondary`, its neighbour grey. bits' `scroll` type (shown
  while scrolling) has no egui counterpart; `always` and `auto` map to
  `ScrollBarVisibility`. CP-14-30 (move select's list) is moot under (b):
  every egui `ScrollArea`, select's included, changes at once. Under (a)
  the crate hides egui's bars and paints its own from the output state,
  with the exact thumb colour and all four bits types, and CP-14-30 is (i)
  move select's list in the same PR or (ii) later. (a)'s cost, not named
  before: the crate then also implements thumb dragging, track clicks, the
  hover widening and the offset writes through `scroll_area::State`, a
  second scroll bar implementation that follows egui at every upgrade.
- **Recommendation:** (b), as the second pushback review proposes (it
  replaces the earlier pick (a)): the theme sets a full `ScrollStyle`
  (floating, track opacities 0, dormant handle opacity 0, handle in the
  foreground colour at a low opacity), which D-135 (a) allows as a look;
  the thumb colour is the nearest shared colour, which the module docs
  state. No new type, no D-61 exception, no own bar code; composites use
  `egui::ScrollArea`; bits' other visibility types become a scoped-style
  helper if a demo needs one. Reason: the exact `bg-border` thumb is the
  only thing (a) adds, and it costs a second scroll bar implementation.
  If the maintainer takes (a) instead: hover-only visibility first and
  bits' four types later, select's list moved in the same PR (CP-14-30
  (i)), the type keeps egui's name and builder (D-18 (ii) allows a
  same-named type with the same meaning), and it mirrors only the subset
  the composites and demos use (constructors per axis, `max_height`,
  `max_width`, `auto_shrink`, `stick_to_bottom`, `id_salt`, `show`,
  `show_rows`; the `show_*` variants are D-61's exception for mirrored
  egui types), because mirroring all of egui's builder (about 30 setters)
  would mean following it at every egui upgrade.
- **Covers:** CP-14-26, CP-14-27, CP-14-30, CP-T4-12, CP-T5-10, CP-T7-8, CP-08-9, CP-18-9, CP-14-28

Decision: (a) — differs from the recommendation (maintainer, P9: colours
match upstream exactly; a `ScrollStyle` thumb reaches only the nearest
shared colour). A crate `ScrollArea` hides egui's bars and paints its own
(transparent track, exact `bg-border` thumb), with the cost named above
accepted: thumb drag, track click, hover widening and offset writes through
`scroll_area::State`, followed at every egui upgrade. Under (a), as the
recommendation describes: hover-only visibility first, bits' other types
later; CP-14-30 (i), select's list moves to the crate type in the
scroll-area PR; the type keeps egui's name (D-18 (ii)) and mirrors only the
subset (constructors per axis, `max_height`, `max_width`, `auto_shrink`,
`stick_to_bottom`, `id_salt`, `show`, `show_rows`). Composites use the crate
type: select and command lists (D-118), message-scroller follow-bottom
(D-108), the sheet body in demos and docs (D-47), attachment group (D-124),
table horizontal overflow (D-122). Stage order: F12 ports the scroll area
after F9 (sheet) and F10 (select, command), which use `egui::ScrollArea`
until the F12 PR migrates them; the plan may instead move the scroll-area
port before F9. The theme sets no `ScrollStyle` for plain egui scroll areas
(D-135).

### D-63 · The `layout` module, `layout::card`, `form_row` and `egui_extras`

- **Case:** `layout::row` keys stored height by call order; `layout::card`
  duplicates `Card::new().show`; `form_row` is the only user of the
  runtime dependency `egui_extras` and is superseded by Field; aspect-ratio
  could live in `layout`.
- **Options:** row: keep, add an id, or drop the stored state.
  `layout::card`: keep, remove (deprecated one release), move. `form_row`:
  keep, rebuild on Field, or deprecate and drop `egui_extras`.
  aspect-ratio: `AspectRatio` builder in `components`, a `layout` fn, or
  not ported.
- **Recommendation:** row gets `.id_salt` per D-12 (auto id default);
  `layout::card` deprecated; `form_row` rebuilt on Field when Field lands,
  deprecated then, and `egui_extras` dropped with it; `AspectRatio` builder
  in `components`. Reason: one public path per thing, and one runtime
  dependency fewer.
- **Affects:** `layout`, card, field, aspect-ratio.
- **Covers:** CP-T6-8, CP-03-8, CP-03-29

Decision: as recommended.

## Identity

### D-64 · Id parameter type, scope and constructor name

- **Case:** The id parameter is `id_salt: impl Hash` in most sketches, `impl
  Into<Id>` for the sidebar; modals hash the salt globally, select and
  inputs scope it to the parent `Ui`; native-select uses `from_id_salt`.
- **Options:** type: (a) `id_salt: impl Hash` everywhere, `Id` only in
  static helpers; (b) `impl Into<Id>` where egui does (`Panel::left`).
  Scope: (i) global `Id::new(salt)` for floating and app-level components
  (modals, sidebar, toaster), `ui.make_persistent_id` for in-flow ones;
  (ii) always parent-scoped. Constructor: (x) `new(id_salt, ..)` /
  `.id_salt(..)`; (y) `from_id_salt` like `ComboBox`.
- **Affects:** every component with an id.
- **Recommendation:** (a), (i), (x). Reason: `id_salt` is egui 0.34's name;
  (i) is what egui does (`Modal::new(Id)` global, `ScrollArea` scoped).
- **Covers:** CP-T6-2, CP-T6-3, CP-T6-6, CP-11-8

Decision: (a), (i), (x) — as recommended.

### D-65 · Popup id of a trigger-plus-list widget

- **Case:** Popups on a caller's trigger derive their id from it in egui;
  select, native-select and command derive it from a required salt like
  `ComboBox`.
- **Options:** (a) derived from the trigger `Response` (egui default)
  plus an optional `.id_salt`; (b) from the required salt (select,
  `ComboBox`); (c) per component.
- **Affects:** popover, hover-card, menus, select, native-select, command.
- **Recommendation:** (a) for builders on a caller's trigger, (b) where
  D-12 requires a salt. Reason: follows D-12.
- **Covers:** CP-T6-5, CP-10-17

Decision: (a) on a caller's trigger, (b) where D-12 requires a salt — as
recommended.

### D-66 · Item ids inside data-driven containers

- **Case:** Items inside data-driven containers get ids by index (tabs,
  accordion, toggle-group, slider thumbs) or by value or caller key
  (command, message-scroller, sonner).
- **Options:** (a) index (`id.with(i)`); (b) a value hash or caller key
  (`impl Hash`); (c) auto ids.
- **Affects:** toggle-group, tabs, accordion, navigation-menu, resizable,
  slider thumbs, command, message-scroller, sonner.
- **Recommendation:** (a) where items do not reorder, (b) where they do
  (message-scroller items `message: impl Hash`, command items by value,
  sonner `ToastId`). Reason: an index id loses focus and state on reorder.
- **Covers:** CP-T6-7, CP-19-11

Decision: (a) where items do not reorder, (b) where they do — as
recommended.

## Orientation and direction

### D-67 · Orientation setter surface

- **Case:** The orientation setter is `.orientation(E)` plus `.vertical()`
  in most sketches, a shortcut only (toggle-group), one method per value
  (separator) or constructors per value (resizable, scroll-area).
- **Options:** (a) `.orientation(Orientation)` + `.vertical()` on every
  widget with an orientation, with constructors per axis only where the
  crate mirrors an egui type that uses them (if D-62 takes (a), the crate's
  `ScrollArea` keeps egui's `vertical()`/`horizontal()`/`both()`); (b) per widget as
  sketched.
- **Affects:** button-group, toggle-group, separator, slider, tabs,
  field, resizable, carousel, attachment.
- **Recommendation:** (a); resizable gets `ResizablePanes::new()` plus
  `.vertical()`. Reason: D-6 type, D-3 shortcut form.
- **Covers:** CP-T7-1

Decision: (a), resizable `ResizablePanes::new()` plus `.vertical()` — as
recommended. The per-axis constructor exception is live: the crate
`ScrollArea` (D-62 (a)) keeps egui's `vertical()`/`horizontal()`/`both()`.

### D-68 · Roving focus: one helper, keys per orientation

- **Case:** toggle-group, radio-group, tabs, accordion, menubar,
  navigation-menu, pagination, questionnaire need group keyboard focus;
  today it is inline in `tabs.rs`. Upstream differs: radio takes both
  axes, slider ignores Left/Right when vertical, carousel uses Left/Right
  in both, accordion makes every trigger a Tab stop, pagination has no
  roving focus, toggle-group's Tab stop is the first item until one is
  on.
- **Options:** (a) one shared helper with the axis and Tab-stop rule as
  parameters, each component configured as its primitive does;
  (b) per-component copies; (c) one crate rule for all (one Tab stop,
  arrows follow the axis, always wrap).
- **Recommendation:** (a), each component as its primitive (radio both
  axes, slider vertical ignores Left/Right, accordion every trigger a Tab
  stop, pagination plain Tab order, tabs automatic activation and wrap,
  radio arrow moves focus only when no item matches). Reason: shared code,
  upstream behaviour (porting standard "Behaviour").
- **Affects:** the components listed.
- **Covers:** CP-T7-3, CP-01-29, CP-05-21, CP-05-29, CP-16-17, CP-01-27, CP-12-26, CP-13-30, CP-05-17, CP-12-9, CP-05-16, CP-12-8

Decision: (a) — as recommended.

### D-69 · Responsive breakpoints

- **Case:** field `@md` responsive orientation, calendar months `md:`,
  sonner below 600 px, sidebar below 768 px (sheet), pagination text below
  `sm`. The porting standard forbids `available_width()` arithmetic.
- **Options:** (a) evaluate against the widget's own available width
  (an exception to the standard, written down); (b) treat as always true
  (desktop); (c) a caller flag (`.responsive()`, `.mobile(bool)`).
- **Affects:** field, calendar, sonner, sidebar, pagination.
- **Recommendation:** (b) by default with (c) where a demo depends on the
  narrow branch (field `.responsive(bool)` evaluating the width as in (a),
  sonner narrow layout from the window width). Reason: egui apps are mostly
  desktop; an explicit opt-in keeps the single-pass sizing rule. The width
  read goes through the breakpoint helper of D-133.
- **Covers:** CP-T7-6, CP-07-14, CP-15-13, CP-14-12, CP-16-37, CP-13-27

Decision: (a) — differs from the recommendation (maintainer, P4: responsive
layout by default, as upstream). A breakpoint reads what upstream's query
reads: the media queries (calendar `md:`, sonner below 600 px, sidebar below
768 px, pagination text below `sm`) read the window width, the container
query (field `@md/field-group`) the field group's width, both through
D-133's helper; this is the written-down exception to the single-pass sizing
rule. Field's responsive orientation stays the opt-in `.responsive(bool)`
(D-6), because upstream's is opt-in too (`orientation="responsive"`).

### D-70 · Modes upstream has but shadcn neither styles nor demos

- **Case:** Bits-ui offers modes no shadcn class styles and no demo uses:
  radio orientation, navigation-menu vertical, pagination vertical, command
  grid columns, marker stacked layout.
- **Options:** (a) leave out (radio orientation, navigation-menu vertical,
  pagination vertical, command grid, marker stacked); (b) offer them.
- **Recommendation:** (a). Reason: no recipe to test against, no demo to
  compare; the shape keeps room to add them.
- **Affects:** radio-group, navigation-menu, pagination, command, marker.
- **Covers:** CP-05-13, CP-13-6, CP-13-25, CP-08-30, CP-18-28

Decision: (a) — as recommended.

## Sizing

### D-71 · Width setter: one name, one meaning

- **Case:** `.width` means fixed (select, popover; slider sketched one, it has
  none today), `.max_width` a
  cap over fill (input, card), button sketched `.min_width`, egui uses
  `desired_width`; skeleton sketched `f32::INFINITY` for full width. Two
  shipped setters fit none of these: `Tabs::width` (0.6.0, `tabs.rs:76-86`,
  `:156-163`) is capped to the available width, never narrower than the
  triggers, and `f32::INFINITY` means fill; `Dialog::width`
  (`dialog.rs:46-52`) is exact, capped to keep clear of the window edges.
- **Options:** (a) `.width(f32)` = exact, `.max_width(f32)` = cap over fill,
  `.min_width(f32)` = floor, each where it makes sense, default fill or
  content per D-72, `f32::INFINITY` = fill (TextEdit idiom); tabs `.width`
  would then become `.min_width` plus fill (a rename with a deprecated
  alias), dialog keeps `.width` as exact with a documented window clamp; (b)
  as (a), but `.width(f32)` defined as "requested width, clamped to what
  fits" (available width, window, content minimum), which keeps dialog and
  tabs as shipped and makes select and popover clamp (see Facts); (c) as
  (a), with tabs as a
  stated exception; (d) only `.width`; (e) egui's `desired_width`.
- **Affects:** every sized widget.
- **Facts (second pushback review, re-review r3):** of the shipped `.width`
  setters, only dialog and tabs clamp. `Select::width` is exact
  (`allocate_space(vec2(width, ..))` with no cap, `select.rs:162,173`), and
  `Popover::width` is exact (`ui.set_width(w)`, `popover.rs:87-98`). Slider
  has no width setter; it fills (`slider.rs:139`). So (b) changes select and
  popover, which start to clamp to the available width. Clamp order under
  (b): the requested width, capped to the available width (or the window,
  for floating overlays), then raised to the content minimum, which then
  overflows, as tabs do today (`tabs.rs:156-163`); the opposite order would
  truncate. Cost of (b): on a widget that fills by default (input,
  textarea, card, select under D-72), `.width(x)` is `min(x, available)`
  raised to the content minimum, and `.max_width(x)` is `min(available,
  x)`; the two differ only by the content minimum, so offering both on one
  widget gives two names for nearly one meaning, against this entry's
  title. Sub-question for (b): (s1) a widget offers `.width` when it is
  content-sized by default and `.max_width` when it fills by default,
  never both; (s2) both setters where the sketch had them, the overlap
  documented.
- **Recommendation:** (b) with (s1) and the clamp order above: `.max_width`
  (stored as `Option<f32>`) on the widgets that fill by default: input,
  textarea, card, alert, aspect-ratio, progress, slider (new), select
  (which fills to `Metrics::field_max_width` under D-72; today's
  `Select::width` becomes a `#[deprecated]` forward to `.max_width` for one
  release); `.width` on the content-sized ones: attachment, menus, popover,
  dialog and tabs; button `min_width`; field none (caller layout); menubar
  content width. Reason: dialog and tabs already clamp, select and popover
  start to clamp (a small behaviour change, stated in CHANGES), no rename
  one release after tabs shipped its width, and with (s1) each widget has
  one width word whose meaning follows from its default. (a) with the tabs
  rename stays the alternative that keeps `.width` exact. The clamp itself
  goes through the helper of D-133 (today `tabs.rs` does
  `w.min(ui.available_width())` inline).
- **Covers:** CP-T8-1, CP-T8-3, CP-01-5, CP-03-4, CP-03-30, CP-04-5, CP-04-11, CP-05-36, CP-06-4, CP-07-11, CP-08-11, CP-10-18, CP-18-6, CP-09-31, CP-09-6, CP-02-24

Decision: (b) with (s1) and the stated clamp order — as recommended. Under
D-72's decision select and native-select are content-sized, so under (s1)
they take `.width` (requested, clamped), not `.max_width`, and
`Select::width` keeps its name with the clamp meaning. Maintainer: renames
are fine (no installed base yet), so the "no rename one release after tabs
shipped" part of the reason carries no weight; the decision rests on (s1)'s
one width word per widget.

### D-72 · Default width of text-entry and choice controls

- **Case:** Input and input-group cap at `Metrics::field_max_width`,
  native-select, field and command fill, select takes its widest option,
  textarea caps only when set.
- **Options:** (a) cap at `Metrics::field_max_width` (input,
  input-group today); (b) fill; (c) per kind as sketched (select widest
  option, native-select fill).
- **Affects:** input, textarea, input-group, select, native-select,
  command, field.
- **Recommendation:** (a) for text entry and both selects (one form row
  looks aligned), command fills; input-group owns the width of its
  control. Reason: consistent rows.
- **Covers:** CP-T8-2, CP-06-33

Decision: (a) for text entry (input, textarea, input-group), command fills,
input-group owns its control's width — as recommended; differs for the
selects (maintainer, P4): the select trigger and native-select are
content-sized as upstream (`w-fit` on `select-trigger.svelte` and on the
native-select wrapper at the pin), not capped at `Metrics::field_max_width`.

### D-73 · Size enums: per component or shared; names; `Default`

- **Case:** Size variants are per-component enums today (`ButtonSize`,
  `SwitchSize`); sketches add more, some without `Default`, sidebar's
  sub-button uses `Md`.
- **Options:** (a) per-component enums (`SwitchSize`, `SelectSize`),
  middle size `Default`, every enum derives `Default`; (b) one shared
  `Size`. Native-select reuses `SelectSize`; input-group button offers
  `Sm` in every style.
- **Recommendation:** (a), native-select re-exports `SelectSize`, `Sm`
  offered (falls back to the button default size). Reason: the variants
  differ per component upstream; `Default` is upstream's name.
- **Affects:** every component with sizes.
- **Covers:** CP-T8-4, CP-T11-3, CP-05-24, CP-11-15, CP-07-23, CP-06-35

Decision: (a), native-select re-exports `SelectSize`, `Sm` offered — as
recommended.

### D-74 · Meaning of `size`

- **Case:** `size` means a variant enum on most widgets but pixels on the
  spinner (`Spinner::size(f32)` in egui), sheet and drawer, and a `Vec2` on
  skeleton.
- **Options:** (a) `size` is always a variant enum; pixel sizes get
  other names (`diameter`, `slot_size(Vec2)`, `extent`); (b) pixels where
  there are no variants (`Spinner::size(f32)` as egui); (c) `Vec2` where
  both axes matter.
- **Affects:** spinner, skeleton, sheet, drawer, input-otp.
- **Recommendation:** (b) and (c): `Spinner::size(f32)`,
  `Skeleton::new(impl Into<Vec2>)`, sheet and drawer `size(f32)` along
  their side, input-otp `slot_size(Vec2)`. Reason: egui's `Spinner::size`
  precedent; no component has both variants and a pixel size.
- **Covers:** CP-T8-5, CP-02-16, CP-02-23, CP-11-20, CP-06-22

Decision: (b) and (c) — as recommended.

### D-75 · How a group shares size with its children

- **Case:** Item group gap depends on its child size, the avatar group count
  follows the avatars, a vertical button-group stretches children to the
  widest.
- **Options:** (a) explicit size on the group (item group, toggle-group,
  input-group); (b) inferred from children (avatar group, button-group
  stretching) from the previous pass; (c) per component.
- **Recommendation:** (c) as sketched: explicit where the size is a
  variant, inferred where it is a measured width (vertical button-group
  stretches to the widest child; avatar group count follows the largest
  avatar). Reason: the two cases differ in kind.
- **Affects:** item group, avatar group, button-group, toggle-group.
- **Covers:** CP-T8-6, CP-04-27, CP-01-12

Decision: (c) — as recommended.

### D-76 · Height model

- **Case:** Resizable, vertical carousel, message-scroller and chart need a
  height from somewhere; empty can fill or take its content height; textarea
  can grow or keep fixed rows.
- **Options:** for widgets that need a height (resizable, carousel
  vertical, message-scroller, chart): (a) the parent's available height,
  optional `.height`; (b) required height. Empty: content height or
  fill. Textarea: grow with content from `min-h-16` (`rows` raises the
  minimum) or fixed rows; user resize handle or none.
- **Recommendation:** (a) with a floor: a recipe or `Metrics` minimum height
  per component (egui_plot has `min_size`) plus `.height(f32)`, and module
  docs that say what happens in a `ScrollArea` (content gets the visible
  size, `containers/scroll_area.rs:757-771`) and in a horizontal row, as
  `Tabs::width`'s doc does; empty content height; textarea grows, no resize
  handle. Reason: without a floor a chart below other content in a scrolling
  page gets near zero height; matches upstream `field-sizing-content` for
  textarea.
- **Affects:** resizable, carousel, message-scroller, chart, empty, textarea.
- **Covers:** CP-T8-7, CP-03-26, CP-14-41, CP-06-10, CP-06-11

Decision: (a) with a floor, empty content height, textarea grows — as
recommended. The module docs name the crate `ScrollArea` (D-62 (a)).

### D-77 · Fractions of the parent

- **Case:** Bubble is capped at 80 % of its parent (`max-w-[80%]`),
  resizable sizes are percentages upstream, carousel sizes slides per view.
- **Options:** bubble `max-w-[80%]`: a recipe field for a fraction plus a
  layout helper, `.max_width` from the caller, or a coded 80 %.
  Resizable sizes: percent 0–100 (paneforge), fraction 0–1, pixels.
- **Recommendation:** a recipe fraction field (generator work) for
  bubble; resizable in percent as upstream. Reason: values stay in
  recipes; percent matches upstream docs and demos.
- **Affects:** bubble, resizable, carousel `per_view`.
- **Covers:** CP-T8-8, CP-18-20, CP-14-34

Decision: as recommended.

### D-78 · When a recipe dimension gets a public setter

- **Case:** Some recipe dimensions are overridden in demos (calendar cell
  size, input-otp slot size, sidebar widths, toggle-group spacing), others
  only in one block demo (card spacing, separator length).
- **Options:** (a) only values a shipped demo changes (calendar
  `cell_size`, input-otp `slot_size`, sidebar widths, toggle-group
  `spacing` in points); (b) every recipe dimension; (c) never, restyle
  via the theme.
- **Recommendation:** (a): card spacing, separator length, scroll-area
  timings and sidebar drag-resize stay off. Reason: rule 5 of the porting
  standard (a demo-only value is a caller parameter).
- **Affects:** card, calendar, sidebar, scroll-area, separator,
  toggle-group.
- **Covers:** CP-T8-9, CP-03-5, CP-15-15, CP-14-14, CP-14-31, CP-04-18, CP-01-25, CP-14-15

Decision: (a) — as recommended.

## State flags and behaviour

### D-79 · `.enabled(bool)` on containers and overlays

- **Case:** D-1 removes `.enabled` from leaves; egui keeps `.enabled` on
  some containers (`Area`, `Window`, `CollapsingHeader`), and Field,
  FieldSet, AccordionItem, MenubarMenu and HoverCard were sketched with it.
- **Options:** (a) yes for containers whose upstream has a `disabled` prop
  that disables the container or its trigger: Field and FieldSet (`<fieldset
  disabled>`), AccordionItem (bits `disabled`), MenubarMenu (bits
  `bits/menubar/types.ts:53-57`, "Whether the trigger for the menubar item
  is disabled"), HoverCard (bits LinkPreview root
  `bits/link-preview/types.ts:52-56`, "the preview will be disabled and will
  not open", at bits-ui 2.19.2); (b) never, `add_enabled_ui` around; (c) as
  leaves (D-1).
- **Recommendation:** (a), with the upstream-prop test as the rule a later
  port applies. Reason: egui shows that containers may carry `.enabled`
  (`Area`, `Window`, `CollapsingHeader`); of the five, only AccordionItem
  has such an egui counterpart (`CollapsingHeader`), so the test is
  upstream's prop, not an egui counterpart (MenubarMenu maps to
  `menu_button`, HoverCard to `Popup`, neither has `.enabled`).
- **Affects:** field, accordion, menubar, hover-card.
- **Covers:** CP-T9-1

Decision: (a) — as recommended.

### D-80 · Shape of per-item disabled

- **Case:** Per-item disabled is sketched as an item setter (`ToggleItem`,
  `SelectItem`, `AccordionItem`), as a parent method `tab_enabled(bool, v,
  atoms)` (tabs), or as `add_enabled_ui` around a closure item (radio,
  navigation-menu).
- **Options:** (a) a setter on the item builder for data items
  (`ToggleItem::enabled`, `SelectItem::enabled`, native-select items,
  `Tab`), `add_enabled_ui` around closure items (radio, menu and command
  items, navigation item); (b) `*_enabled(bool, ..)` on the parent (tabs
  sketch); (c) one form for all.
- **Recommendation:** (a); tabs and accordion items become item builders
  with `.enabled(bool)` (D-1 name, D-17 no positional bool); command items
  are closure items and take `add_enabled_ui`. Reason: data items have no
  `Ui` to disable; closure items do.
- **Affects:** tabs, accordion, toggle-group, select, command, radio,
  native-select, navigation-menu, questionnaire choices.
- **Covers:** CP-T9-2, CP-12-6, CP-12-22, CP-12-32, CP-13-8, CP-05-15, CP-07-27, CP-08-32, CP-19-38

Decision: (a) — as recommended.

### D-81 · Overriding built-in Previous/Next buttons

- **Case:** Pagination and carousel disable Previous/Next at the ends
  automatically; the questionnaire navigation-state demo disables Next from
  outside.
- **Options:** (a) automatic only (pagination, carousel disable at the
  ends); (b) `.next_enabled(bool)`-style setters; (c) `add_enabled_ui`
  around the whole component.
- **Recommendation:** (b) for questionnaire, whose demo needs it; (a)
  elsewhere. Reason: only one demo needs the override.
- **Affects:** pagination, carousel, questionnaire.
- **Covers:** CP-T9-15, CP-19-31

Decision: (b) for questionnaire, (a) elsewhere — as recommended.

### D-82 · Overlay on a disabled trigger

- **Case:** Egui's `on_hover_text` hides tooltips on disabled widgets unless
  `on_disabled_hover_text` is used; upstream's tooltip-disabled demo shows
  one; popover and menus follow their trigger.
- **Options:** (a) hint overlays (tooltip) open for disabled triggers,
  action overlays (popover, menus, collapsible) do not; (b) all follow the
  trigger; (c) per component.
- **Recommendation:** (a). Reason: upstream's tooltip-disabled demo; an
  action overlay on a disabled control would be a bug.
- **Affects:** tooltip, popover, menus, collapsible, hover-card.
- **Covers:** CP-T9-3, CP-10-10

Decision: (a) — as recommended.

### D-83 · `invalid`: presence and flow

- **Case:** Badge's base classes style `aria-invalid`, calendar's do not;
  radio-group sets invalid on the group and items, input-otp on the whole
  widget, Field on its label only in the sketch.
- **Options:** presence: (a) `.invalid(bool)` wherever a recipe styles
  `aria-invalid` (badge yes, calendar no); (b) on every control.
  Flow: (i) group sets, parts inherit and may override (radio-group);
  (ii) group pushes `INVALID` into its `Ui` for crate controls (field);
  (iii) caller sets each part.
- **Recommendation:** (a); input-otp whole widget; radio-group (i); Field
  (ii), so `Field::invalid` reaches the control as upstream's label and
  control both turn red. Reason: flags follow recipes (porting standard
  rule 3); one inheritance mechanism (D-57).
- **Affects:** badge, calendar, radio-group, input-otp, field,
  input-group, toggle-group.
- **Covers:** CP-T9-4, CP-05-14, CP-06-18, CP-07-8, CP-02-4, CP-15-19

Decision: (a); input-otp whole widget, radio-group (i), Field (ii) — as
recommended.

### D-84 · Validation API and `required`

- **Case:** Field takes `.invalid(bool)` plus errors, Questionnaire
  `error(Option<text>)`; formsnap carries `aria-required`, the questionnaire
  has a behavioural `required`.
- **Options:** (a) `.invalid(bool)` + `.error(text)` on Field and
  Questionnaire, an error implies invalid; (b) `error(Option<text>)` only;
  `required`: AccessKit-only setter, behavioural in questionnaire only, or
  not offered outside questionnaire.
- **Recommendation:** (a); `required` behavioural in questionnaire,
  AccessKit-only `required(bool)` on Field. Reason: one API for one idea;
  formsnap carries `aria-required`.
- **Affects:** field, form, questionnaire.
- **Covers:** CP-T9-5, CP-07-9, CP-19-27, CP-T9-6, CP-07-21

Decision: (a); `required` behavioural in questionnaire, AccessKit-only on
Field — as recommended.

### D-85 · `read_only`

- **Case:** Calendar has `readonly` in shadcn's demo; bits-ui radio-group
  and checkbox have it too, unused by shadcn.
- **Options:** (a) `read_only(bool)` where upstream has `readonly`
  (calendar; radio-group, checkbox in bits); (b) calendar only; (c) none.
- **Recommendation:** (b) now, the name reserved for the others. Reason:
  only calendar's shadcn demo uses it.
- **Affects:** calendar, radio-group, checkbox.
- **Covers:** CP-T9-7, CP-15-18

Decision: (b) — as recommended.

### D-86 · Clickable display widgets and `link`

- **Case:** Display widgets become clickable with `.sense(Sense)` (item,
  attachment, bubble, marker) or `.link(bool)` (badge, marker); a link badge
  today underlines but cannot be clicked (cleanup bug).
- **Options:** (a) `.sense(Sense)` like `egui::Label::sense` for item,
  attachment, bubble, marker, avatar; (b) `.link(bool)` = click + focus +
  `WidgetType::Link` (badge, marker) plus `.sense` for plain clicks;
  (c) separate clickable types. Button `href` mode: skip (caller does
  `ctx.open_url`) or `.link(url)`.
- **Recommendation:** (a) and (b): `.sense` for clicks, `.link(bool)` for
  the link look and role; the `[a]` hover rules become a `LINK` flag;
  button `href` skipped. Reason: egui's `Label::sense`; the link look is a
  recipe state upstream.
- **Affects:** badge, item, attachment, bubble, marker, avatar, button.
- **Covers:** CP-T9-10, CP-02-3, CP-03-13, CP-18-19, CP-18-25, CP-18-4, CP-04-28, CP-02-9, CP-01-6

Decision: (a) and (b), button `href` skipped — as recommended.

### D-87 · Mutually exclusive states: enum or bools

- **Case:** Checkbox indeterminate is a `bool` display flag in the sketch,
  attachment state an enum, menu destructive an enum variant.
- **Options:** (a) enum for three or more states, `bool` for two;
  (b) always enums; (c) as sketched.
- **Recommendation:** (a): checkbox `.indeterminate(bool)` display flag
  (click from indeterminate writes `true`), `AttachmentState` enum, menu
  item `.variant(MenuItemVariant::Destructive)`. Reason: matches egui's
  checkbox and the crate's variant enums.
- **Affects:** checkbox, attachment, menus.
- **Covers:** CP-T9-13, CP-05-1, CP-18-7, CP-09-9

Decision: (a) — as recommended.

### D-88 · Animation scope

- **Case:** the crate ignores animation classes (spec §4.4, stylegen
  ANIM rule). Upstream animates spinner, skeleton pulse, progress value,
  caret blink, accordion and collapsible height, sheet slide, sidebar
  width, navigation-menu viewport, carousel slides, sonner stack, shimmer
  text, scroller.
- **Options:** (a) none anywhere; (b) only continuous indicators whose
  meaning is motion (spinner, skeleton pulse, shimmer), with constants
  `// behaviour-ok`; (c) animate where upstream does; (d) per component
  `.animate(bool)`, or one global switch from `style.animation_time`.
- **Recommendation:** (b), plus carousel slide and sonner enter/exit
  (their state change is unreadable without motion), all honouring
  `style.animation_time == 0` as "reduced motion". Caret blink is egui's.
  Reason: keeps the "no animation" rule except where the animation is the
  information.
- **Affects:** spinner, skeleton, progress, input-otp, accordion,
  collapsible, sheet, sidebar, navigation-menu, carousel, sonner,
  attachment, marker, message-scroller.
- **Covers:** CP-T9-14, CP-T3-10, CP-02-21, CP-02-17, CP-02-27, CP-04-13, CP-06-20, CP-14-16, CP-12-20, CP-12-30, CP-11-24, CP-13-4, CP-16-21, CP-16-32, CP-16-35, CP-18-8, CP-19-16, CP-18-27

Decision: (c) — differs from the recommendation (maintainer, P4: upstream
motion). Animate where upstream does (spinner, skeleton pulse, progress
value, accordion and collapsible height, sheet slide, sidebar width,
navigation-menu viewport, carousel slides, sonner stack, shimmer text,
scroller), with upstream's durations and easings, all honouring
`style.animation_time == 0` as reduced motion; caret blink stays egui's.

### D-89 · Accessibility roles egui's `WidgetType` lacks

- **Case:** Egui's `WidgetType` has no switch, tab, menu, dialog, log or
  table; AccessKit (a non-optional egui dependency) has those roles through
  `accesskit_node_builder`.
- **Options:** (a) `WidgetType` where one fits, plus AccessKit roles
  through `accesskit_node_builder` where it does not (switch, tab,
  tablist, log, table, live regions, expanded); (b) `WidgetType` only.
  Per component: toggle `Button` + selected; toggle-group upstream roles;
  spinner `.label` default "Loading"; skeleton `Other`; separator
  decorative; command highlight as `selected`; accordion `expanded`;
  carousel roledescription; field error and marker `status` live regions;
  item `Button`/`Other`.
- **Recommendation:** (a) with the per-component answers listed. Reason:
  AccessKit is a non-optional egui dependency; the porting standard
  requires the role.
- **Affects:** the components listed.
- **Covers:** CP-01-19, CP-01-28, CP-02-19, CP-02-26, CP-04-19, CP-05-23, CP-08-34, CP-12-12, CP-12-25, CP-19-20, CP-16-10, CP-16-23, CP-07-17, CP-03-22, CP-18-29

Decision: (a) with the per-component answers — as recommended.

## Shortcut functions

### D-90 · Which components get a shortcut, and what closure shortcuts return

- **Case:** Today checkbox, switch, label, select and tabs have shortcuts;
  sketches add some for new leaves and some for menus and tooltip, and leave
  others out.
- **Options:** (a) every leaf whose common use is one call (button, badge,
  kbd, spinner, skeleton, progress, separator, label, checkbox, switch,
  slider, input, textarea, marker, alert, radio), plus value shortcuts for
  list choices (`select`, `native_select`, `tabs`, `toggle_group`, D-27) and
  for `pagination` (page, total) and `calendar`/`range_calendar` (one
  binding); none for containers, popover-style overlays and data-heavy
  widgets (item, avatar, field, attachment, table, popover, hover-card,
  command); named exceptions: the menu shortcuts `dropdown_menu`,
  `menubar_menu`, `context_menu` (egui has `menu_button` and
  `Response::context_menu`) and the trigger helper `tooltip(resp, text)`
  (egui has `on_hover_text`); (b) only where egui has a `Ui` or `Response`
  counterpart; (c) per component. Closure shortcuts return what the egui
  counterpart returns (D-9: `menu_button` form).
- **Recommendation:** (a) with the named exceptions (they are (b)'s
  counterparts); table gets none (the builder is short). Reason: one rule
  a port can apply without asking.
- **Affects:** every component.
- **Covers:** CP-T10-11, CP-T10-1, CP-01-4, CP-04-8, CP-04-14, CP-05-38, CP-06-5, CP-16-11, CP-18-32, CP-03-19, CP-04-30, CP-07-13, CP-08-35, CP-10-21, CP-18-12, CP-17-11

Decision: (a) with the named exceptions — as recommended.

## Generator and recipes

### D-91 · `data-open`: a real state, as `OPEN` or `EXPANDED`

- **Case:** stylegen drops `data-open` as animation, but it carries an
  open look. Where it sits: accordion on the *item*
  (`data-open:bg-muted/50`, maia/mira/luma/rhea), while `aria-expanded` is
  on the accordion trigger (it switches the chevrons); menus on the
  sub-trigger (`data-open:bg-accent`), which also carries `aria-expanded`;
  navigation-menu on the trigger (`data-open:bg-muted/50`, plus
  `group-data-open/navigation-menu-trigger` for the icon rotation);
  sidebar menu button `data-open:hover:*` (a collapsible sub-menu that is
  open). On content parts `data-open` only drives animation (stays
  ignored). The sketches ask for `OPEN` (CP-12-34, CP-14-48) or reuse
  `EXPANDED` (CP-13-36).
- **Options:** (a) new `OPEN` flag; (b) reuse `EXPANDED` (`aria-expanded`
  is set on the same triggers); (c) resolve in code.
- **Recommendation:** (b), a deviation from the CP-12-34 and CP-14-48
  sketches. Reason: both attributes mean the same thing, "the disclosure
  this part belongs to is open"; upstream writes it as `aria-expanded` on
  triggers and `data-open` on items and sub-triggers, but the crate sets
  flags per part, so the accordion item gets `EXPANDED` from its own open
  state just as its trigger does. This is consistent with D-92 (one
  meaning per flag): it is one meaning, not a reuse. No part found reads
  both attributes with different meanings (report 09 checked the menus).
  If the maintainer prefers the DOM split, (a) costs one bit (D-11).
- **Affects:** stylegen, accordion, menus, navigation-menu, sidebar.
- **Covers:** CP-T11-14

Decision: (b) — as recommended.

### D-92 · Meaning of `SELECTED`; `PRESSED`

- **Case:** `SELECTED` holds `data-selected`/`aria-selected` today; toggle's
  `aria-pressed`, the sidebar's `data-active` and the message-scroller
  button's `data-active` (edge reached) were sketched either as new flags or
  as `SELECTED`.
- **Options:** (a) `SELECTED` keeps one meaning (`data-selected`,
  `aria-selected`); new `PRESSED` (`aria-pressed`, toggle `data-[state=on]`)
  and a new flag for the scroller button's `data-active`; (b) map
  `aria-pressed` and `data-active` onto `SELECTED` to save bits.
- **Recommendation:** (a), and the sidebar's `peer-data-active` /
  `data-active` (the current item, D-30 `selected`) maps to `SELECTED`
  as CP-14-48 sketches, because it has the same meaning; the scroller's
  `data-active` (an edge is reached) gets its own flag. Reason: with D-11
  (d) (or (a)) bits are no longer short; one meaning per flag keeps the generator's
  mapping unambiguous.
- **Affects:** stylegen, toggle, toggle-group, message-scroller.
- **Covers:** CP-T11-15, CP-01-17, CP-19-18

Decision: (a) — as recommended.

### D-93 · `data-[side=*]`: side flags

- **Case:** stylegen ignores `data-[side=*]` as a popup side, so sheet
  borders, `w-3/4` and `max-w` drop silently; drawer uses
  `data-[vaul-drawer-direction=*]`, sidebar `data-[side=right]`.
- **Options:** (a) four shared `SIDE_TOP/RIGHT/BOTTOM/LEFT` flags read by
  sheet, drawer and sidebar; (b) only the sides a recipe reads; (c) one
  recipe per side, picked in code.
- **Recommendation:** (a). Reason: one mapping in stylegen for three
  components; popup sides stay ignored where they mean placement.
- **Affects:** stylegen, sheet, drawer, sidebar.
- **Covers:** CP-T11-16, CP-11-37

Decision: (a) — as recommended.

### D-94 · The plain down chevron slot

- **Case:** `IconSlots::chevron_down` holds Select's selector icon for
  tabler (`IconSelector`) and hugeicons (`UnfoldMoreIcon`), so accordion,
  navigation-menu and calendar cannot use it as a chevron.
- **Options:** (a) new `chevron_down_plain`; (b) rename the Select slot
  `select_trigger` and make `chevron_down` the true chevron (breaking,
  deprecated alias impossible for a changed meaning); (c) one
  `chevron_down`, accept Select's icon.
- **Recommendation:** (b) in the 0.7.0 cleanup. Reason: slots are named
  per shape (D-5); a slot called `chevron_down` that is not one misleads
  every future port.
- **Affects:** `shadcn-egui-core`, icongen, select, accordion,
  navigation-menu, calendar.
- **Covers:** CP-T11-13

Decision: (b) with a new name for the true chevron too — differs from the
recommendation (maintainer: old code must fail to compile; renames are fine
before 1.0). The Select slot becomes `select_trigger`, the plain down
chevron takes a new slot name (`chevron_down_plain`, option (a)'s name,
unless the 0.7.0 cleanup picks a better one), and `chevron_down` is removed,
so every old use is a compile error rather than another icon.

### D-95 · Child targets and pseudo elements (`*:`, `**:`, `before:`, `after:`)

- **Case:** `*:` and `**:` sit in stylegen's IGNORED list and drop bubble
  variant colours, the accordion trigger icon, command headings, menu
  focus text and input-group Kbd chips with no diagnostic; `before:` and
  `after:` carry the drawer inset card, the avatar inner border and the
  marker dot.
- **Options:** (a) retarget child-slot rules to the child part's recipe
  and add `_before`/`_after` recipes in stylegen; (b) hand-written
  recipes per case; (c) leave out. Navigation content links: suppress the
  focus ring as upstream via an in-content flag, or keep the ring.
- **Recommendation:** (a), the avatar border painted without blending
  (egui has no blend modes), navigation content links without the ring.
  Reason: the porting standard keeps style values in recipes; these are
  recurring forms, not one-offs.
- **Affects:** stylegen, bubble, accordion, command, menus, input-group,
  drawer, avatar, marker, navigation-menu.
- **Covers:** CP-18-31, CP-04-29, CP-11-32, CP-13-12

Decision: (a), navigation content links without the ring — as recommended,
except the avatar inner border, which differs (maintainer, P9): it shows
upstream's blended colour (`mix-blend-darken`, `dark:mix-blend-lighten`),
not a plain unblended border; how egui reaches that colour is the avatar
port's work (F2, D-110).

### D-96 · Recipe source when svelte and the style CSS disagree

- **Case:** command group headings (svelte inline classes vs the style's
  `**:[[cmdk-group-heading]]` rules), chart tooltip (`cn-chart-tooltip`
  vs svelte's inline vega classes), chart plot overrides `[&_.lc-*]`,
  sonner's look in svelte-sonner CSS.
- **Options:** (a) what the pinned svelte source renders; (b) the style
  CSS marker; (c) per case. Sonner and chart plot values: hand-written
  constants with `// behaviour-ok`/`// token-ok`, a hand-written recipe
  set in stylegen, or `Metrics`.
- **Recommendation:** (a) for command (what the pin renders), (b) for the
  chart tooltip (the marker exists in every style), sonner and plot values
  as documented constants, the sonner description in `muted-foreground`
  rather than svelte-sonner's literal. Reason: follow the pin; a literal
  that ignores the palette breaks themes.
- **Affects:** command, chart, sonner.
- **Covers:** CP-08-26, CP-17-7, CP-16-29, CP-16-30, CP-17-12

Decision: (a) for command; sonner and plot values as documented constants —
as recommended; differs on colour (maintainer, P9): the sonner description
takes svelte-sonner's literal colour exactly, not `muted-foreground`; the
chart tooltip takes the style CSS marker (b) only where it gives the colours
the svelte docs page renders, otherwise the svelte source (a), checked in
the chart PR (F20).

### D-97 · Upstream rules the browser never applies, or that look broken

- **Case:** `data-[active=true]` where bits sets `data-active=""`
  (input-otp, navigation-menu in six styles); input-group's disabled fade
  nobody sets; menubar checkbox items with no disabled fade; no focus
  style on a closed menubar trigger; rhea's invalid `hsl(oklch)` outline;
  calendar selected-hover text near-black on near-black; calendar and
  range-calendar spacing and unavailable-day look differ.
- **Options:** (a) reproduce the browser (the rule stays dead); (b) port
  the evident intent; (c) per case.
- **Recommendation:** (b) where the intent is unambiguous and another
  style or React shows it (input-otp active ring, navigation active
  background, input-group fade, rhea ring, unify calendar on Calendar's
  classes), (a) where no intent is visible (menubar fade and focus,
  calendar hover text, flagged in the module docs). Reason: users compare
  with the docs page; an invisible rule that egui would show is a visible
  difference.
- **Affects:** input-otp, navigation-menu, input-group, menubar, sidebar,
  calendar, range-calendar.
- **Covers:** CP-13-9, CP-06-17, CP-06-30, CP-09-32, CP-09-33, CP-14-24, CP-15-27, CP-15-34, CP-15-35, CP-15-16

Decision: (a) everywhere — differs from the recommendation (maintainer, P4
and P9: render only the rings and backgrounds upstream renders). The
input-otp active ring, navigation active background, input-group fade and
rhea ring stay dead as in the browser; calendar and range-calendar each keep
the spacing and unavailable-day look their pinned classes render (no
unification); menubar fade and focus and the calendar hover text as
recommended. Each dead or odd rule is flagged in the module docs.

### D-98 · Relative colours (`oklch(from …)`)

- **Case:** Bubble's tinted variant colour is a relative OKLCH colour
  derived from a palette role (`oklch(from var(--…) …)` in the style CSS);
  stylegen has no paint kind for it.
- **Options:** (a) a new relative-OKLCH `Paint` kind in stylegen;
  (b) a precomputed role per palette.
- **Recommendation:** (a). Reason: keeps palettes generic.
- **Affects:** stylegen, bubble.
- **Covers:** CP-18-22

Decision: (a) — as recommended.

### D-99 · Ternary class strings (carousel orientation)

- **Case:** Carousel picks its slide geometry with orientation ternaries
  inside `cn(...)`; stylegen's extractor rejects them, and also stopped on
  the `class={cn(` forms of chart, pagination and sidebar.
- **Options:** (a) slide geometry as behaviour constants, stylegen reads
  the two markers; (b) teach the extractor ternaries plus logical-side,
  rotate and basis rules. The `class={cn(` forms that stopped the
  extractor (carousel, chart, pagination, sidebar) need extractor support
  either way.
- **Recommendation:** (b). Reason: the same ternaries recur in sidebar
  and pagination.
- **Affects:** stylegen, carousel, pagination, sidebar, chart.
- **Covers:** CP-16-22

Decision: (b) — as recommended.

### D-100 · Generator scope for content-styling classes

- **Case:** `font-sans`; link and paragraph rules inside content
  (`[&_a]`, `[&_p]`, `[&>a]`); a caller `[.border-t]` divider;
  `[&:has([role=checkbox])]:pr-0` in table cells.
- **Options:** (a) ignore with a fitting reason (`font-sans`, links,
  paragraphs, divider) and a `Column::checkbox()` kind for the table rule;
  (b) translate them.
- **Recommendation:** (a). Reason: they style caller content egui
  renders itself; the IGNORED reasons must say so (today's reasons do not
  fit, a cleanup item).
- **Affects:** stylegen, kbd, alert, accordion, item, input-group, table.
- **Covers:** CP-02-15, CP-04-7, CP-12-23, CP-03-21, CP-06-32, CP-16-9

Decision: (a) — as recommended.

### D-101 · Sidebar palette roles

- **Case:** The palette import skips the eight `sidebar-*` roles
  (`themes.rs:217`), so the sidebar recipes have no colours.
- **Options:** (a) add the eight `sidebar-*` roles to `Palette` and the
  palette import (`themes.rs:217` skips them); (b) map them onto existing
  roles.
- **Recommendation:** (a). Reason: every upstream theme sets them
  differently from the base roles.
- **Affects:** `Palette`, stylegen, sidebar.
- **Covers:** CP-14-25

Decision: (a) — as recommended.

### D-102 · The `sera` style

- **Case:** stylegen reads `sera` but does not export it (618 unsupported
  pairs, uppercase and tracking fields unread). No CP; recurring across
  reports.
- **Options:** (a) keep it out of this project; (b) export it once the
  generator gaps above are closed.
- **Recommendation:** (a), revisit after F20. Reason: it is a style, not a
  component, and its gap count would hold up every family.
- **Affects:** stylegen, badge (`uppercase`, `tracking`).
- **Covers:** none (survey "Generator work").

Decision: (a) — as recommended.

## Verdicts and dependencies

### D-103 · `form`: not applicable in egui

- **Case:** shadcn's form wraps formsnap and sveltekit-superforms
  (SvelteKit stores, validation, submission).
- **Options:** (a) not applicable: fold the aria wiring (described-by,
  invalid from errors, required) into Field, ship a build-ui map row
  pointing to Field; (b) a thin `Form` container collecting field errors
  and gating a submit button; (c) a validation crate dependency.
- **Recommendation:** (a). Reason: egui has no form submission model;
  Field covers what is visible.
- **Affects:** form, field.
- **Covers:** CP-07-20

Decision: (a) — as recommended.

### D-104 · `chart`: new optional dependency `egui_plot`

- **Case:** chart is a theming wrapper around layerchart; in egui the
  plotting library would be `egui_plot`. The release that matches the
  crate's egui 0.34 is `egui_plot` 0.35.0 (requires `egui ^0.34.0`,
  rust-version 1.92; from the crates.io index). The latest, 0.37.0,
  requires egui 0.36, so the chart feature moves with each egui upgrade.
  Without it, chart is very large.
- **Options:** (a) optional `egui_plot` behind a `chart` feature, the crate
  ships `ChartConfig`, tooltip, legend and palette; (b) paint bar, line,
  area, pie in the crate; (c) ship tooltip, legend and palette only.
  Scope: bar first / bar+line+area / all kinds. Series colour: `Color32`,
  `{light, dark}` pair, or palette index (sketch: all three). Data model:
  egui_plot items plus `ChartConfig`. `nestLabel` per `chart-tooltip.svelte`.
  Legend click toggles visibility; no keyboard access to points; no hover
  dimming; bar orientation left to egui_plot.
- **Recommendation:** (a), bar first, colours as the sketch, the rest as
  listed; chart stays in F20 (last). Reason: dependency discipline allows
  an optional, feature-gated crate for a capability the crate cannot
  reasonably paint; it needs the maintainer's consent.
- **Scope:** decide now: the dependency (`egui_plot` 0.35.0 as an optional
  `chart` feature, or (b)/(c)) and the verdict (F20); deferrable to the
  chart PR: chart kinds, colour type, data model, `nestLabel`, legend,
  keyboard, hover dimming, orientation.
- **Affects:** chart, `Cargo.toml` features.
- **Covers:** CP-17-1, CP-17-2, CP-17-3, CP-17-4, CP-17-6, CP-17-9, CP-17-14, CP-T7-9, CP-17-13

Decision: (a) — maintainer consents: `egui_plot` 0.35.0 as an optional
`chart` feature; verdict F20, as recommended. Sub-choices (kinds, colour
type, data model, `nestLabel`, legend, keyboard, hover dimming, orientation)
deferred to the chart PR (recommendation is the default proposal).

### D-105 · Date type for calendar and range-calendar (`jiff` or own `Date`)

- **Case:** calendar's verdict is `port` only with a crate-owned date;
  otherwise it needs `jiff`. "Today" needs the local date; `Weekday` is
  used for `week_start`.
- **Options:** (a) crate-owned `Date { year, month, day }` and `Weekday`
  with own arithmetic, optional `jiff` feature for `From`/`Into` and the
  local "today"; (b) `jiff::civil::Date` (what `egui_extras`' date picker
  uses) as a required dependency; (c) `chrono` or `time`. Today without
  `jiff`: (t1) UTC from `SystemTime`, as first written: `SystemTime::now()`
  panics on `wasm32-unknown-unknown`, the web gallery's target, and UTC is
  the wrong day near midnight elsewhere; (t2) no implicit today:
  `.today(Date)` is the only source, nothing is marked when unset; with the
  `jiff` feature, today comes from `jiff` (`jiff/js` on wasm); (t3)
  `SystemTime` gated off wasm; (t4) a `web-time` dependency.
- **Gallery "today" under (t2), a sub-choice:** the wasm gate builds the
  gallery with `--features fonts-all,icons-all`, without `jiff`, and `jiff`
  is not in `Cargo.lock` today. (g1) `jiff` as a dev-dependency of the
  gallery (with `jiff/js` on `wasm32`), converted to the crate's `Date` in
  the example: a new crate in the build, which the dependency discipline
  of the porting standard asks the maintainer to consent to; (g2) a fixed
  demo date, marked as such (some shadcn docs demos use one); (g3) JS glue
  in the web loader page that passes the local date, which the example
  does not have today.
- **Recommendation:** (a) with (t2), and (g1) if the maintainer consents
  to the dev-dependency, else (g2). Reason: no required dependency,
  calendar arithmetic on civil dates is small, the optional feature serves
  apps that already use `jiff`, and no code path panics in the browser.
  Calendar stays very large by effort (F18).
- **Scope:** decide now: (a)/(b)/(c), the dependency choice, the today
  source (t1)-(t4), which shapes the API, and the gallery's today source
  (g1)-(g3), whose (g1) adds a dev-dependency; the calendar's other choices are
  D-126 and may be deferred to F18.
- **Affects:** calendar, range-calendar.
- **Covers:** CP-15-1, CP-15-26, CP-T7-10

Decision: (a) with (t2) — as recommended; the maintainer consents to the
optional `jiff` feature and to (g1), `jiff` as a dev-dependency of the
gallery (with `jiff/js` on `wasm32`). The calendar's other choices are
D-126.

### D-106 · `navigation-menu`: very large

- **Case:** hover intent, shared viewport, focus proxy, ~15 generator
  gaps. Verdict "very large": last or a later project.
- **Options:** (a) port in F20; (b) defer to a later project.
  Sub-choices: (i) `viewport = false` too, via `.viewport(bool)`; (ii)
  skip the Indicator (the shadcn root never renders it); (iii) viewport
  sized to content + 1rem as the pin; (iv) emulate the focus proxy (Tab
  from an open trigger into its content).
- **Recommendation:** (a) with (i) to (iv) as listed. Reason: the menu
  primitive (F8) and hover timer (D-40) carry most of the work by then.
- **Scope:** decide now: (a) port in F20 or (b) defer to a later project;
  deferrable to the family PR: sub-choices (i) to (iv).
- **Affects:** navigation-menu.
- **Covers:** CP-13-3, CP-13-5, CP-13-7, CP-13-14

Decision: (a), port in F20 — as recommended; sub-choices (i) to (iv)
deferred to the family PR (recommendation is the default proposal). Under
D-41's decision, hover items stay open on an outside touch, as upstream.

### D-107 · `sidebar`: very large

- **Case:** eight palette roles (D-101), ~25 variants, ~15 parts; needs
  sheet, collapsible, skeleton, tooltip side.
- **Options:** (a) port in F19; (b) defer. Sub-choices: root a `Sidebar`
  builder over `egui::Panel` (vs free layout fns or a provider/root
  split); Cmd/Ctrl+B by `.shortcut(Option<KeyboardShortcut>)`, default on;
  sub-menus as `sidebar_menu_sub` container plus `SidebarMenuSubButton`;
  rail as `.rail(bool)`; `sidebar_inset(ui, ..)` reading the variant from
  context; skeleton width deterministic from the id.
- **Recommendation:** (a) with the sub-choices as listed (mobile mode per
  D-69). Reason: apps ask for a sidebar early; the dependencies land in
  F1 to F11.
- **Scope:** decide now: (a) port in F19 or (b) defer; deferrable to the
  family PR: root shape, shortcut, sub-menus, rail, inset, skeleton width
  (the sub-menu state trade-off of D-32 is crate-wide and decided now).
- **Affects:** sidebar.
- **Covers:** CP-14-3, CP-14-4, CP-14-10, CP-14-17, CP-14-19, CP-14-20

Decision: (a), port in F19 — as recommended; sub-choices deferred to the
family PR (recommendation is the default proposal). Under D-69's decision
the mobile mode (a sheet below 768 px window width) is on by default, as
upstream.

### D-108 · `message-scroller`: very large; the follow-bottom subset

- **Case:** the full primitive (anchoring, tail spacer, prepend, opening
  position, visibility) is very large; follow-bottom, a scroll-to-end
  button and `scroll_to_message` on `ScrollArea::stick_to_bottom` is
  medium.
- **Options:** (a) ship the subset in F16, the full primitive in F20;
  (b) full primitive only, in F20; (c) subset only. Shape: one
  `MessageScroller` builder with an item handle; scroll button as a
  builder option; prepend correction next frame plus `request_discard`.
- **Recommendation:** (a), shape as listed. Reason: chat apps need the
  subset with the chat parts.
- **Scope:** decide now: (a)/(b)/(c), which sets the stage-4 order;
  deferrable to the family PR: the shape items.
- **Affects:** message-scroller.
- **Covers:** CP-19-13, CP-19-9, CP-19-14, CP-19-19

Decision: (a), the subset in F16 and the full primitive in F20 — as
recommended; the shape deferred to the family PR (recommendation is the
default proposal). The follow-bottom subset sits on the crate
`ScrollArea::stick_to_bottom` (D-62 (a)).

## Family bundles

Each item: CP, the question, the options, the recommendation. Unless
said otherwise the recommendation is the sketch pick, which follows the
pinned primitive.

### D-109 · F1 display leaves (kbd, skeleton, progress, separator, aspect-ratio)

- **Case / Options / Recommendation per item:**
  - `CP-02-11` kbd shortcut helper: none / one chip per modifier via
    `ModifierNames` / one chip with `ctx.format_shortcut` · per modifier.
  - `CP-02-25` skeleton shape: `.circle()` / `.radius(..)` / egui
    `.corner_radius` · `.corner_radius(impl Into<CornerRadius>)` plus
    `.circle(bool)` per D-3.
  - `CP-04-9` progress value: fraction (egui `ProgressBar`) / `value`+`max`
    · `new(fraction)` plus `from_value(value, max)` (D-25).
  - `CP-04-10` indeterminate progress: `Option<f32>` drawn empty / animated
    bar / not offered · not offered.
  - `CP-04-17` separator default orientation: infer from the `Ui` / always
    horizontal · infer (egui `Separator`).
  - `CP-03-31` aspect-ratio degenerate ratio: height 0 / fall back to 1.0 ·
    height 0, as upstream.
  - `CP-03-32` clip content: clip / overflow · overflow, as upstream.
  - `CP-03-33` background and rounding: caller paints / `.frame(Frame)` ·
    caller paints.
- **Affects:** kbd, skeleton, progress, separator, aspect-ratio.
- **Covers:** CP-02-11, CP-02-25, CP-04-9, CP-04-10, CP-04-17, CP-03-31, CP-03-32, CP-03-33

Decision: deferred to the family PR (recommendation is the default
proposal).

### D-110 · F2 content blocks (card, item, empty, avatar)

- **Case / Options / Recommendation per item:**
  - `CP-03-6` card image and edge-to-edge content: not offered / `.image` +
    bleed part · not offered.
  - `CP-03-20` item line clamp: title 1 row, description 2 / wrap freely ·
    clamp as upstream.
  - `CP-03-25` empty outline: not offered / `.outline(bool)` with a 1 px
    dashed border whose width has no recipe source · `.outline(bool)`, width
    from `Metrics` (needs a new metric).
  - `CP-03-27` empty background: not offered / `.fill` · not offered
    (demo-only).
  - `CP-04-23` avatar loading-status control: expose / egui loader only ·
    egui loader only; pending and error show the fallback.
- **Affects:** card, item, empty, avatar.
- **Covers:** CP-03-6, CP-03-20, CP-03-25, CP-03-27, CP-04-23

Decision: deferred to the family PR (recommendation is the default
proposal). The avatar's inner border follows D-95's decision (upstream's
blended colour).

### D-111 · F3 buttons and groups

- **Case:** the group-position protocol (now D-134) and per-corner radius
  have no CP of their own; per-corner radius is listed here.
- **Options / Recommendation per item:**
  - `CP-01-8` groupable widgets: Button only / Button, Input, Select
    trigger, group text, nested group · all listed.
  - `CP-01-13` focus ring above siblings: reserved later shape slot /
    accept overlap · reserved slot.
  - `CP-01-16` icon-only toggle: `.icon_only(bool)` / an `Icon` size
    variant / empty text · `.icon_only(bool)` (D-3).
  - `CP-01-18` `Toggle::icon` sets `ICON_START`: always / never / separate
    setter · never (toggle demos keep their padding).
  - `CP-01-26` per-item variant/size override in toggle-group: drop / keep
    · drop.
  - (no CP) per-corner radius and single-side borders in the shared
    painters (`CornerRadius` instead of one `f32`) · yes, in the cleanup.
- **Affects:** button, button-group, toggle, toggle-group, input-group.
- **Covers:** CP-01-8, CP-01-13, CP-01-16, CP-01-18, CP-01-26

Decision: deferred to the family PR (recommendation is the default
proposal).

### D-112 · F4 choice controls (checkbox, slider)

- **Case:** Choices that touch only checkbox and slider behaviour and the
  slider's value model.
- **Options / Recommendation per item:**
  - `CP-05-4` Enter toggles a checkbox: yes (egui fake click) / Space only
    (bits) · Space only.
  - `CP-05-7` checked and indeterminate both true: indeterminate wins /
    tick wins · indeterminate wins.
  - `CP-05-25` several thumbs: `Slider::multiple` / `RangeSlider` / none ·
    `multiple` on the same type, later (as the sketch and the verdict);
    the cleanup shapes the API so it can be added without a break.
  - `CP-05-26` slider value type: `f32` / `emath::Numeric` · `f32` now.
  - `CP-05-27` default step: 0 continuous (egui) / 1 (bits) · 0, documented
    as a crate choice.
  - `CP-05-28` PageUp/PageDown: keep ten steps (WAI-ARIA) / drop (bits has
    none) · keep, no longer credited to bits.
  - `CP-05-30` Meta+Arrow to the ends: add / omit · add.
  - `CP-05-31` pointer mapping: over the thumb travel / over the root ·
    thumb travel.
  - `CP-05-32` End when the step does not divide the range: `max` / last
    grid value · last grid value.
  - `CP-05-33` crossing thumbs: swap and move focus / stop · swap.
  - `CP-05-34` array steps: `.steps(&[f32])` / not offered · not offered.
- **Affects:** checkbox, slider.
- **Covers:** CP-05-4, CP-05-7, CP-05-25, CP-05-26, CP-05-27, CP-05-28, CP-05-30, CP-05-31, CP-05-32, CP-05-33, CP-05-34

Decision: deferred to the family PR (recommendation is the default
proposal), except CP-05-27, decided now: default step 1, as bits
(maintainer, P4).

### D-113 · F5 text entry (input, textarea, input-otp, input-group)

- **Case:** Choices that touch only input, textarea, input-otp and input-group.
- **Options / Recommendation per item:**
  - `CP-06-6` TextEdit options passed through: none / `char_limit` / also
    `return_key`, `horizontal_align`, `prefix`/`suffix` · `char_limit`.
  - `CP-06-7` HTML input types: `password(bool)` only / a `kind` enum ·
    `password(bool)` only.
  - `CP-06-9` textarea: own `Textarea` / `Input::multiline` · own type.
  - `CP-06-12` Tab in a textarea: moves focus / inserts `\t` / option ·
    moves focus.
  - `CP-06-13` input-otp value: `&mut String` / `Vec<Option<char>>` /
    `[char; N]` · `&mut String`.
  - `CP-06-15` pattern: regex (new `regex` dependency) / `fn(char) -> bool`
    / enum · `fn(char) -> bool`, no dependency.
  - `CP-06-19` input model: own event handling / hidden `TextEdit` (keeps
    IME) · own handling.
  - `CP-06-21` paste with a bad char: reject whole / keep allowed / filter
    hook · reject whole (bits).
  - `CP-06-23` focus on a full value: last cell selected / caret at end ·
    last cell (bits).
  - `CP-06-24` input-group structure: builder with stored addon closures /
    `show` with a handle / TextEdit prefix and suffix · follows D-47: the
    `show` closure holds the addons and the control as calls in display
    order (prefixed part fns, or a handle passed to the closure), not
    closures stored in the builder.
  - `CP-06-28` allowed controls: Input and Textarea / any widget · Input
    and Textarea.
  - `CP-06-29` input-group return: control's `Response` / `InnerResponse`
    / frame `Response` · control's `Response`.
  - `CP-06-31` addon click focus: any control / Input only · any control.
  - `CP-06-34` `in-data-[slot=combobox-content]`: now / with the Combobox
    port · with the Combobox port.
- **Affects:** input, textarea, input-otp, input-group.
- **Covers:** CP-06-6, CP-06-7, CP-06-9, CP-06-12, CP-06-13, CP-06-15, CP-06-19, CP-06-21, CP-06-23, CP-06-24, CP-06-28, CP-06-29, CP-06-31, CP-06-34

Decision: deferred to the family PR (recommendation is the default
proposal).

### D-114 · F6 forms (field, native-select)

- **Case:** Choices that touch only field and native-select.
- **Options / Recommendation per item:**
  - `CP-07-7` label/control order in a horizontal field: `.control_first(bool)`
    / `LabelSide` enum / closure order · `.control_first(bool)`.
  - `CP-07-10` `Field::enabled(false)` disables the control: yes
    (`add_enabled_ui`) / fades the label only · yes.
  - `CP-07-12` choice card: `FieldCard` with `.selected(bool)` /
    `Field::card` / not offered · `FieldCard`; focus-visible of the inner
    radio reaches the card through the `Ui` (D-57).
  - `CP-07-18` checkbox-group gap and nested gap: `.tight(bool)` plus
    auto-detected nesting / `.tight` and `.nested` · the first: `.tight(bool)`
    stays as a setter, nesting is detected through the `Ui` (D-57).
  - `CP-07-22` native-select: own `NativeSelect` / `Select::native()` / not
    offered · own type.
  - `CP-07-25` native-select list look: egui popup / Select's recipes /
    plain popup in Popover colours · plain popup in Popover colours.
  - `CP-07-26` native-select placeholder: none (upstream) / `.placeholder`
    · none.
- **Affects:** field, native-select.
- **Covers:** CP-07-7, CP-07-10, CP-07-12, CP-07-18, CP-07-22, CP-07-25, CP-07-26

Decision: deferred to the family PR (recommendation is the default
proposal).

### D-115 · F7 anchored overlays (popover, tooltip)

- **Case:** the overlay frame helper (from `popover.rs`) has no CP; it is
  shared by popover, hover-card, menus, select. Recommendation: extract it.
- **Options / Recommendation per item:**
  - `CP-10-5` PopoverClose: `ui.close()` only / helper · `ui.close()`.
  - `CP-10-13` tooltip arrow gap: tip touches the trigger / bits
    `sideOffset` + arrow box height · bits formula, checked against the
    docs page.
  - `CP-10-23` arrow on left and right sides: top/bottom only / all four ·
    all four.
- **Affects:** popover, tooltip, hover-card, menus.
- **Covers:** CP-10-5, CP-10-13, CP-10-23

Decision: deferred to the family PR (recommendation is the default
proposal). Tooltip timing follows D-40 and D-130.

### D-116 · F8 menus (dropdown-menu, context-menu, menubar)

- **Case:** Choices that touch only the three menus and their shared item widgets.
- **Options / Recommendation per item:**
  - `CP-09-2` item widgets: one shared set reading the menu's `Ui` tag /
    per-menu types · shared (D-56).
  - `CP-09-8` item shortcut text: `.shortcut(text)` / a part fn · setter.
  - `CP-09-10` checkbox item value: `&mut bool` / `bool` + `clicked()` ·
    `&mut bool`.
  - `CP-09-12` indeterminate checkbox item: display flag / second binding
    / not supported · display flag.
  - `CP-09-14` close on select: shadcn's defaults (item closes,
    checkbox/radio stay) / bits (all close) / egui (every click) · shadcn's,
    with `.close_on_select(bool)`.
  - `CP-09-15` item selection report: `clicked()` / `on_select` closure ·
    `clicked()`.
  - `CP-09-17` sub-menu: `MenuSub::new(text).show` / themed nested
    `menu_button` · `MenuSub`.
  - `CP-09-22` trigger fires on pointer down (bits) / click (egui) · click.
  - `CP-09-24` context-menu placement: `RIGHT_START`, gap 2 (bits) / egui
    default · bits.
  - `CP-09-25` long-press on touch: `Response::long_touched()` / skip ·
    `long_touched()`.
  - `CP-09-26` indeterminate on context-menu checkbox items: not offered
    (upstream file ignores it) / offered (shared widget) · offered.
  - `CP-09-29` menubar hover-switch and Left/Right: port / egui `MenuBar`
    (none) · port.
  - `CP-09-34` indicator side: from the recipe insets / per component ·
    from the recipes.
- **Affects:** dropdown-menu, context-menu, menubar.
- **Covers:** CP-09-2, CP-09-8, CP-09-10, CP-09-12, CP-09-14, CP-09-15, CP-09-17, CP-09-22, CP-09-24, CP-09-25, CP-09-26, CP-09-29, CP-09-34

Decision: deferred to the family PR (recommendation is the default
proposal), except CP-09-22, decided now: the trigger fires on pointer down,
as bits (maintainer, P4; D-45).

### D-117 · F9 modals (dialog, alert-dialog, sheet, drawer)

- **Case:** the shared modal shell on `egui::Modal` (overlay, panel,
  header, close, footer) has no CP. Recommendation: extract it from
  `dialog.rs`. The shell joins the overlay stack of D-128 (ii+): `Modal`
  closes on Escape only while egui's popup memory is empty
  (`containers/modal.rs:151-161`), which crate overlays do not fill, and on
  any backdrop click, whatever is open above it (`:157-159`). So the shell
  does not use `ModalResponse::should_close()` as a whole (today
  `dialog.rs:154` does): it closes on `ui.close()` from its content, on the
  crate Escape hook's mark, and on a backdrop click only when it is the top
  entry of the stack (D-128 (o1); under (o2), when no overlay above it
  holds the click). Footers are part fns inside the content
  closure (D-47 (d6)), because `Modal::show` returns no handle. Drawer's verdict is "port" with two open provisos: it holds
  only if snap points (CP-11-29) and background scaling (CP-11-30) are
  left out as recommended; taking either raises drawer's effort beyond
  large.
- **Options / Recommendation per item:**
  - `CP-11-6` footer `showCloseButton`: omit / `dialog_footer_with_close`
    / flag · omit (caller adds a Button with `ui.close()`).
  - `CP-11-9` corner close button tab order: last / first · last.
  - `CP-11-12` alert-dialog Action closes: no (bits) / yes · no.
  - `CP-11-17` alert-dialog: own type / `Dialog::alert()` · own type on the
    shell.
  - `CP-11-25` sheet: own type / `Dialog::side` · own type.
  - `CP-11-26` drawer: own type / `Sheet::draggable` · own type.
  - `CP-11-27` drag to dismiss: port (threshold 0.25, 0.4 px/ms) / omit ·
    port.
  - `CP-11-28` non-modal drawer: `.modal(bool)` / omit · `.modal(bool)`.
  - `CP-11-29` snap points: omit / `.snap_points` · omit.
  - `CP-11-30` nested drawers, background scaling: not applicable / draw
    parents displaced · not applicable.
- **Affects:** dialog, alert-dialog, sheet, drawer, CommandDialog.
- **Covers:** CP-11-6, CP-11-9, CP-11-12, CP-11-17, CP-11-25, CP-11-26, CP-11-27, CP-11-28, CP-11-29, CP-11-30

Decision: deferred to the family PR (recommendation is the default
proposal). The sheet slide and the drawer's motion animate as upstream (D-88
(c)); the sheet body scrolls in the crate `ScrollArea` (D-47, D-62 (a)).

### D-118 · F10 pickers (select, command)

- **Case:** Choices that touch only select and command.
- **Options / Recommendation per item:**
  - `CP-08-4` select multiple: leave out / `Select::multiple` / own type ·
    leave out, shape keeps room (D-25).
  - `CP-08-6` select return: `Response` + `changed()` /
    `InnerResponse<Option<T>>` · `Response`.
  - `CP-08-20` command result order: source order / re-sort by score ·
    source order.
  - `CP-08-21` command filter: port `computeCommandScore` with `.filter(fn)`
    and `.should_filter(bool)` / substring / caller only · port.
  - `CP-08-22` command activation: per-item `clicked()` / `Option<value>`
    from `show` · per-item `clicked()`.
  - `CP-08-25` CommandDialog placement: top-third option on Dialog / own
    `Modal` · option on the modal shell.
  - `CP-08-27` command checked indicator: `.checked(bool)` / caller icon ·
    `.checked(bool)` (D-30).
  - `CP-08-29` command container: `show` closure / data slice · `show`.
  - `CP-08-31` command keyboard extras: full bits set with
    `.vim_bindings(bool)` / arrows, Home/End, Enter only · full set.
  - `CP-08-33` command loading row: omit / `cmd.loading(..)` · omit.
- **Affects:** select, command.
- **Covers:** CP-08-4, CP-08-6, CP-08-20, CP-08-21, CP-08-22, CP-08-25, CP-08-27, CP-08-29, CP-08-31, CP-08-33

Decision: deferred to the family PR (recommendation is the default
proposal). Select and command lists run in the crate `ScrollArea` (D-62
(a)); select's list moves there in the F12 PR (CP-14-30 (i)). Triggers open
on pointer down with press-drag-release (D-45).

### D-119 · F11 tabs and disclosure (accordion, collapsible)

- **Case:** Choices that touch only accordion and collapsible.
- **Options / Recommendation per item:**
  - `CP-12-24` accordion `not-last:border-b`: separator above every item
    but the first / record and draw after · separator above all but the
    first.
  - `CP-12-27` collapsible: document the `if` pattern only / helper fn
    `collapsible_trigger` / container `Collapsible::new(&mut open).show` ·
    helper fn.
- **Affects:** accordion, collapsible.
- **Covers:** CP-12-24, CP-12-27

Decision: deferred to the family PR (recommendation is the default
proposal). Accordion and collapsible animate their height as upstream (D-88
(c)).

### D-120 · F12 scroll and panes (scroll-area, resizable)

- **Case:** Choices that touch only scroll-area and resizable.
- **Options / Recommendation per item:**
  - `CP-14-29` keyboard scrolling and focus ring of a scroll viewport: add
    / skip · skip.
  - `CP-14-37` collapsible panes: port with min/max / min/max only · port.
  - `CP-14-39` F6 handle cycling: port / drop · drop.
  - `CP-14-42` directional drag cursor at a limit: port / two-way always ·
    port.
- **Affects:** scroll-area, resizable.
- **Covers:** CP-14-29, CP-14-37, CP-14-39, CP-14-42

Decision: deferred to the family PR (recommendation is the default
proposal). Under D-62 (a) F12 ports a real widget (the crate `ScrollArea`
with its own bars, thumb drag, track click, hover widening) and moves
select's list onto it; CP-14-29's skip still holds, and the PR checks that
egui's keyboard scrolling keeps working under the hidden bars.

### D-121 · F13 paging (breadcrumb, pagination)

- **Case:** Choices that touch only breadcrumb and pagination.
- **Options / Recommendation per item:**
  - `CP-13-15` breadcrumb separators: automatic plus `.separator(icon)` /
    explicit calls · automatic.
  - `CP-13-17` per-item truncation: `.max_item_width` / caller · caller.
  - `CP-13-19` ellipsis: static marker / built-in collapse into a dropdown
    · static marker.
  - `CP-13-22` pagination input: `total_pages` / `count` + `per_page` /
    both · both (`new`, `from_count`).
  - `CP-13-24` composition: one widget / exposed parts / both via public
    `page_items` · one widget plus `page_items`.
  - `CP-13-26` centring: centre with `.centered(bool)` / never · never.
  - `CP-13-28` degenerate input: clamp the caller's `page` into
    `1..=max(total_pages, 1)` and mark changed / clamp only for drawing,
    leave the value / leave it like upstream · clamp and mark changed, so
    the write follows T1 (a widget that writes the caller's value marks
    `changed()`); the report's caveat (a silent write) no longer applies.
  - `CP-13-29` implementation: compose crate `Button` / paint with button
    recipes · compose `Button`.
- **Affects:** breadcrumb, pagination.
- **Covers:** CP-13-15, CP-13-17, CP-13-19, CP-13-22, CP-13-24, CP-13-26, CP-13-28, CP-13-29

Decision: deferred to the family PR (recommendation is the default
proposal). Pagination's text below `sm` follows the window width by default
(D-69).

### D-122 · F14 data display (table, carousel)

- **Case:** the drag/fling/snap scroller primitive has no CP;
  recommendation: build it for carousel, shared with drawer drag where it
  fits.
- **Options / Recommendation per item:**
  - `CP-16-1` table parts: one builder with `header/row/footer` and
    `text/cell` / struct per part / wrap `egui::Grid` · one builder.
  - `CP-16-2` layout engine: own (widths from the previous frame) /
    `egui::Grid` / `egui_extras::TableBuilder` (keeps a runtime dependency)
    · own.
  - `CP-16-3` column sizing: declared (`Column::auto/exact/remainder`) /
    inferred · declared.
  - `CP-16-4` horizontal overflow: table wraps itself / caller /
    `.scroll(bool)` · table wraps itself.
  - `CP-16-7` colspan: `cell_span(n, ..)` / not offered · `cell_span`.
  - `CP-16-8` caption: bottom only / `.caption_top(bool)` too · bottom only.
  - `CP-16-12` striping, clickable rows: neither / `.striped(bool)` ·
    neither.
  - `CP-16-15` carousel prev/next: built in by `.buttons(bool)` / caller /
    both · built in.
  - `CP-16-18` drag: drag with snap / buttons and keys only / drag-free
    option · drag with snap.
  - `CP-16-19` embla options: `align`, `loop`, `per_view`, `spacing` /
    more / none · the four.
  - `CP-16-20` autoplay: not offered / `.autoplay(Duration)` · not offered.
- **Affects:** table, carousel.
- **Covers:** CP-16-1, CP-16-2, CP-16-3, CP-16-4, CP-16-7, CP-16-8, CP-16-12, CP-16-15, CP-16-18, CP-16-19, CP-16-20

Decision: deferred to the family PR (recommendation is the default
proposal). CP-16-4's self-wrapping table scrolls in the crate `ScrollArea`
(D-62 (a)).

### D-123 · F15 toasts (sonner)

- **Case:** Choices that touch only sonner.
- **Options / Recommendation per item:**
  - `CP-16-24` action callbacks: events from `Toaster::show` / query fn /
    stored closures · events (`Vec<ToastEvent>`).
  - `CP-16-26` multiple toasters: one per context / keyed · one.
  - `CP-16-27` custom component toasts: not offered / `'static` closure ·
    not offered.
  - `CP-16-28` promise toasts: loading toast updated by id / poll handle ·
    by id.
  - `CP-16-31` swipe to dismiss: port / close only · port.
  - `CP-16-33` Alt+T hotkey and Escape: port / leave out · port.
  - `CP-16-34` `rich_colors`: not offered / offered · not offered.
  - `CP-16-36` warning/error icon mapping (remixicon swapped between Svelte
    and React): React / Svelte at the pin · Svelte at the pin (follow the
    pin), flagged in the icongen table.
- **Affects:** sonner.
- **Covers:** CP-16-24, CP-16-26, CP-16-27, CP-16-28, CP-16-31, CP-16-33, CP-16-34, CP-16-36

Decision: deferred to the family PR (recommendation is the default
proposal). The description colour is D-96's decision (svelte-sonner's
literal); the narrow layout below 600 px is on by default (D-69).

### D-124 · F16 chat parts (bubble, message, attachment, marker)

- **Case:** the scroll-fade mask and shimmer painter have no CP of their
  own (shimmer is D-88); recommendation: no scroll-fade in the first port.
- **Options / Recommendation:**
  - `CP-18-36`, `CP-19-5` how Message learns a ghost Bubble is inside:
    caller sets `Message::ghost(bool)` / Bubble writes a `Ui` flag read one
    frame late / ignore · caller sets it.
- **Affects:** bubble, message.
- **Covers:** CP-18-36, CP-19-5

Decision: deferred to the family PR (recommendation is the default
proposal). The attachment group scrolls in the crate `ScrollArea` (D-62
(a)).

### D-125 · F17 questionnaire

- **Case:** Choices that touch only the questionnaire.
- **Options / Recommendation per item:**
  - `CP-19-21` overall shape: data model (`&[Question]` + caller state) /
    closure composition · data model.
  - `CP-19-25` action row: built in / caller-placed / both · built in.
  - `CP-19-26` custom layouts: not offered / free part fns / closure hook ·
    `questionnaire_progress(ui, &state)` only.
  - `CP-19-29` freeform input types: text only / `.password` / type enum ·
    text only.
  - `CP-19-30` status callback: read from state / output field · read from
    state.
  - `CP-19-32` reset: `reset(defaults)` / caller rebuilds · caller rebuilds.
  - `CP-19-34` built-in strings: English defaults with setters / required ·
    English defaults with setters.
  - `CP-19-35` arrow and shortcut keys: only while focus is inside /
    whenever visible · focus inside.
- **Affects:** questionnaire.
- **Covers:** CP-19-21, CP-19-25, CP-19-26, CP-19-29, CP-19-30, CP-19-32, CP-19-34, CP-19-35

Decision: deferred to the family PR (recommendation is the default
proposal).

### D-126 · F18 calendar and range-calendar details

- **Case:** Choices that touch only calendar and range-calendar.
- **Options / Recommendation per item:**
  - `CP-15-3` return: `Response` / `Option<Date>` / `InnerResponse` ·
    `Response`.
  - `CP-15-5` unavailable dates: not selectable / selectable and invalid ·
    not selectable.
  - `CP-15-7` view follows an external value: no (Calendar) / yes
    (RangeCalendar) · per mode as upstream.
  - `CP-15-8` date picker: inline only / also `DatePicker` · inline only.
  - `CP-15-9` caption dropdowns: crate `Select` with a caption trigger look
    / egui `ComboBox` / label only · crate `Select`.
  - `CP-15-10` custom day content: `day_note` / `day_ui` closure / none ·
    `day_note`.
  - `CP-15-11` prev/next button variant: expose / fixed ghost · fixed.
  - `CP-15-14` week start: Sunday / Monday / locale · Sunday with
    `.week_start(Weekday)`.
  - `CP-15-22` month and weekday names: English plus a text struct /
    format callbacks / `icu` dependency · English plus struct.
  - `CP-15-23` focus model: one focusable widget with a focused date / a
    widget per day · one widget.
  - `CP-15-24` keys: upstream set / plus PageUp/PageDown/Home/End · upstream.
  - `CP-15-25` ±7 step over disabled days: count cells (upstream quirk) /
    step by date · step by date.
  - `CP-15-28` outside-month days: selectable / disabled · selectable, with
    `.disable_outside_days(bool)`.
  - `CP-15-29` navigation options: all / `months` + `min`/`max` · all.
  - `CP-15-30` range value: `&mut DateRange` with optional ends /
    `&mut Option<(Date, Date)>` with pending start in memory · `DateRange`.
  - `CP-15-32` hover preview of the pending range: none / accent · none.
- **Affects:** calendar, range-calendar.
- **Covers:** CP-15-3, CP-15-5, CP-15-7, CP-15-8, CP-15-9, CP-15-10, CP-15-11, CP-15-14, CP-15-22, CP-15-23, CP-15-24, CP-15-25, CP-15-28, CP-15-29, CP-15-30, CP-15-32

Decision: deferred to the family PR (recommendation is the default
proposal). Range-calendar is its own type (D-25 (d)); each calendar keeps
the look its pinned classes render (D-97 (a)); the months layout follows the
window width (D-69).

## Added in review round 1

### D-127 · Label wrapping: a setter beside the new default

- **Case:** today's `Label` never wraps (`layout_no_wrap`); the
  maintainer put this among the cleanup bugs, so the default becomes
  wrapping at the available width, as `egui::Label` and upstream do. Open
  is whether Label also gets a setter to choose another mode.
- **Options:** (a) wrap only, no setter; (b) `.wrap_mode(egui::TextWrapMode)`
  like `egui::Label` (Extend, Wrap, Truncate); (c) `.wrap(bool)`.
- **Affects:** label, and every part that draws through Label (field
  label, card title if it reuses it).
- **Recommendation:** (b). Reason: egui's own setter and type; it also gives
  truncation for callers who want it, without a second setter (breadcrumb
  leaves per-item truncation to the caller, D-121 CP-13-17, and the caller
  can then use this setter), and a `bool` cannot express truncation.
- **Covers:** CP-07-3

Decision: (b) — as recommended.


## Added in the adversarial review (Task 7)

### D-128 · Where the open flag of anchored overlays lives

- **Case:** egui's popup memory holds at most one open popup per
  viewport: `Popup::open_id` "opens the given popup and closes all
  others" (`containers/popup.rs:669`), `Memory` keeps
  `popups: ViewportIdMap<OpenPopup>` (`memory/mod.rs:118`). Today's
  popover and select use it (`popover.rs:10-12` says nesting works only
  through `open_bool`). D-8 (b) puts popover, menus, select and
  native-select into memory, and D-38 assumes nested overlays (select in a
  popover, sub-menus).
- **Options:** (i) egui popup memory: one open popup at a time, nesting only
  through a caller binding; `Popup::is_id_open` reads it; (ii) a crate-owned
  open flag per popup id in `ctx.data`, shown through `Popup::open_bool`:
  popups are independent and nest; egui's own menus (`menu_button`) and
  `ComboBox` stay single-open beside them, and the crate's query fns read
  its own flag. Under both, a click inside a nested overlay closes its
  parent: egui's `Popup::show` closes on `clicked_elsewhere()`
  (`containers/popup.rs:597-603`), which is true when the pointer's layer is
  not the popup's own, and a nested list is its own `Area` layer; egui
  avoids this only inside its menu machinery
  (`MenuState::is_deepest_open_sub_menu`, `popup.rs:609-611`), and
  `Context::set_sublayer` supports one level only (`context.rs:3121-3129`).
  So (ii) needs (ii+): every crate overlay uses
  `PopupCloseBehavior::IgnoreClicks` plus a crate-owned overlay stack
  (parent id per open overlay, pushed through a `Ui` tag, D-57); closing an
  overlay closes its descendants; sub-menus and D-38's Escape helper use the
  same stack. An egui `ComboBox` or `menu_button` inside a crate overlay
  remains a known limit.
- **Escape under (ii+) (second pushback review):** `Popup::show` closes on
  `key_pressed(Escape)` whatever the close behaviour
  (`containers/popup.rs:613-616`), and a parent popup's `show` runs that
  check after its content. `key_pressed` counts the pass's key events, and
  `consume_key` removes them (`input_state/mod.rs:727-769`). So the stack
  lives in `ctx.data` from the previous pass, and a crate hook at the start
  of each pass consumes Escape and marks the top overlay of that stack as
  closing. egui has such a hook: `Context::on_begin_pass` or
  `Context::add_plugin` with `Plugin::on_begin_pass` (`context.rs:1947`,
  `:1968`, `plugin.rs:27`); `Theme::apply` registers it once, as it already
  registers an `on_begin_pass` hook for the heading font
  (`theme.rs:693-704`). The hook consumes
  Escape only when the previous pass's stack is not empty, so an app's own
  Escape handling keeps working while no crate overlay is open.
  If the helper ran inside the top overlay, a sibling overlay drawn earlier
  in the pass would already have closed. Modals: `ModalResponse::should_close`
  closes on Escape when `!any_popup_open` (`containers/modal.rs:151-161`),
  read from egui's popup memory (`Popup::is_any_open`,
  `containers/popup.rs:664-666`), which crate flags shown through
  `open_bool` never fill; a select open in a dialog would let the dialog
  close on the same Escape. Under the hook, Escape is consumed before
  `Modal` sees it, so the modal shell (D-117) joins the stack and closes
  itself when it is the top entry. The same holds for clicks:
  `ModalResponse::should_close` is true on `backdrop_response.clicked()`
  whatever popups are open (`containers/modal.rs:157-159`), and today's
  shell uses it as is (`dialog.rs:154`), so a backdrop click with a crate
  select open in the dialog would close both. The shell therefore does not
  call `should_close()` as a whole: it closes on `ui.close()` from its
  content, on the hook's Escape mark, and on a backdrop click only when it
  is the top entry of the stack (under (o1); under (o2), when no overlay
  above it holds the click).
- **Outside click under (ii+), a sub-question:** (o1) as bits' dismissible
  layer: only the top overlay reacts to a click outside it
  (`isResponsibleLayer`, bits `use-dismissable-layer.svelte.ts:280-293`),
  whatever the click hits, so one click closes one level; (o2) every
  overlay that is neither the clicked overlay nor one of its ancestors
  closes, so a click in a grandparent closes both levels above it and a
  click outside every overlay closes the whole chain (the second pushback
  review's proposal). The two agree whenever only one open level lies above
  the clicked one.
- **Affects:** popover, hover-card, dropdown-menu, context-menu, menubar,
  select, native-select, command popover, navigation-menu, the modal shell;
  D-33, D-38, D-117.
- **Recommendation:** (ii) with the overlay stack (ii+), the Escape hook at
  the start of the pass, and (o1). Reason: nested overlays are common in
  shadcn demos (combobox = popover + command, select inside a popover, a
  menu in a dialog); under (i) opening the inner one closes the outer one,
  and without the stack a click in the inner one closes the outer one. (o1)
  is the primitive's behaviour; (o2) saves a click when the user leaves a
  nested chain, at the cost of a difference from upstream. Cost: the stack
  is new shared machinery for F7, F8, F9 and F10, more effort than a flag
  store; it lands with F7.
- **Covers:** none (review finding A1).

Decision: (ii) with the overlay stack (ii+), the Escape hook at the start of
the pass, and (o1) — as recommended.

### D-129 · How a crate `Icon` becomes an egui atom

- **Case:** D-58 (a) wants crate icons inside `IntoAtoms` content. `Icon`
  lives in `shadcn-egui-core`, `Atom` and `IntoAtoms` in egui, so the orphan
  rule forbids `impl From<Icon> for Atom` in `shadcn-egui`; core has no
  dependencies by invariant; and a rasterised image needs a texture, which a
  plain `From` cannot make without a `Context`. Two more limits: egui paints
  an image atom with its own tint, while only text atoms get the widget's
  `fallback_text_color` (`atomics/atom_layout.rs:479-482`), so an icon atom
  created by `From` cannot follow hover, an active tab, a toggle's on state
  or a destructive variant (upstream icons use `currentColor`); and
  `Icon::name` is the screen-reader name
  (`shadcn-egui-core/src/icon.rs:16-23`), shared across libraries
  (`chevron-down`), so a URI built from it cannot find the path data again.
- **Options:** (a') a local wrapper `IconAtom(Icon)` with `impl
  From<IconAtom> for egui::Atom<'_>` (allowed: the local type is the trait's
  parameter), backed by an `ImageSource::Uri` that a crate image loader
  registered by `Theme::install` rasterises, as first written; (a'1) typed
  `.icon` slots, painted with the recipe's state colour, on every crate
  widget and data item that shows icons (`ToggleItem`, `Tab`, select items,
  alert-dialog buttons, D-58), and (a') only for egui hosts and free atoms,
  tinted at creation with a colour the caller passes or the default text
  colour; crate widgets that take `IntoAtoms` re-tint image atoms of the
  `shadcn-icon` scheme with their state colour. (a'1) needs a URI from
  which the loader finds the path data again, and an `Icon` knows neither
  its library nor an index: it has `name`, `size` and `shape` only
  (`shadcn-egui-core/src/icon.rs:19-23`), a `Blob` shape holds the blob
  slice, `start` and `len` (`blob.rs:18-22`) over an icon crate's private
  `static ICONS_BIN`, and a `Segs` icon (icongen `--out`, the form the
  port-component skill gives an app) belongs to no library. URI forms:
  (u1) a process-wide registry: `From<IconAtom>` stores the `Icon` (it is
  `Copy` with `'static` data) in a `std::sync::Mutex<HashMap<Key, Icon>>`,
  keyed by blob address and range as the raster cache already keys
  (`components/icon_raster.rs:40-53`), and writes `shadcn-icon://<n>`; the
  loader looks `<n>` up and rasterises; std only, works for `Segs` icons,
  core unchanged; (u2) `library` and `index` fields on `Icon`: a breaking
  change of the core type (struct literals in every icon crate and in apps,
  icongen regenerates all crates), with a sentinel library for `Segs`
  icons; (u3) a URI that encodes the blob address and range directly,
  which the loader would turn back into a slice with `unsafe` code; (b) `Atom::custom`
  placeholders painted by the component (right colour, works only inside
  crate widgets, D-58 (b)); (a'') `icon.atom()` through an extension trait
  (conflicts with D-53's "no extension trait"); (a''') egui as a dependency
  of `shadcn-egui-core` (breaks an invariant).
- **Affects:** tabs, navigation links, alert-dialog buttons, native-select
  options, kbd, any egui host of crate icons; D-15, D-28.
- **Recommendation:** (a'1), as the pushback review proposes, with (u1), as
  the second pushback review proposes. Reason: state colours stay right
  wherever crate widgets draw icons, and egui hosts still get crate icons;
  (u1) lets the loader find the path data without a core change or
  `unsafe` code, and covers icons of every origin. The registry only grows
  (one entry per distinct icon, a few hundred bytes each), which the module
  docs say. The rasteriser draws the translucent icon layers (0.6.0, icon
  opacity) at their alpha.
- **Covers:** none (review finding A2).

Decision: (a'1) with (u1) — as recommended.

### D-130 · Tooltip mechanism: egui's tooltip or an own popup

- **Case:** D-40 gives tooltip `.open_delay`/`.close_delay` and keyboard
  opening, but egui's `Tooltip` (`Response::on_hover_ui`) has one global
  `tooltip_delay` and shows on hover only. Today's crate tooltip uses
  egui's machinery.
- **Options:** (a) keep egui's `Tooltip`: global delay, no keyboard
  opening, no per-tooltip delay; `tooltip(resp, text)` stays the mirror
  of `on_hover_text`; (b) an own `Popup` plus the hover timer of D-40:
  per-tooltip delays and keyboard opening, egui's `tooltip_delay` ignored
  by crate tooltips, one more custom overlay to maintain.
- **Affects:** tooltip, sidebar collapsed tooltips, D-40, D-90's
  `tooltip` helper.
- **Recommendation:** (b). Reason: keyboard users get the tooltip bits shows
  on focus, and hover-card and tooltip share one timer. `tooltip` keeps the
  `on_hover_text` shape (same arguments, returns the `Response`), though not
  its machinery. Under (b) the tooltip popup is built with
  `PopupKind::Tooltip`: egui hides its own tooltips while a layer has open
  popups (`containers/tooltip.rs:222-227`), and `Popup::show` registers
  every popup except the `Tooltip` kind (`popup.rs:558-566`).
- **Covers:** none (review finding A7).

Decision: (b) — as recommended. With D-40's decision the crate tooltip opens
with upstream's delay (0, the shadcn provider's `delayDuration`); egui's
`tooltip_delay` is neither read nor changed by crate tooltips.

### D-131 · `#[non_exhaustive]` on public enums

- **Case:** shadcn adds variants and sizes over time; adding a variant to
  an exhaustive public enum breaks callers who `match` on it.
  `TabsOrientation` is `#[non_exhaustive]` (`tabs.rs:30`), `ButtonVariant`
  and `ButtonSize` are not.
- **Options:** (a) every public enum `#[non_exhaustive]`; (b) variant and
  size enums only (upstream grows them), not closed sets (`Orientation`,
  `Side`); (c) none.
- **Affects:** every public enum.
- **Recommendation:** (b). Reason: variants and sizes follow upstream,
  which adds them; a closed set gains nothing from the attribute and loses
  exhaustive matching.
- **Covers:** none (review finding A31).

Decision: (b) — as recommended.

### D-132 · Logical or physical placement names; the `align` setter names

- **Case:** D-43 picks `egui::Align`/`Align2`, whose names are physical
  (`Min`/`Max`, `LEFT`/`RIGHT`, `LEFT_TOP`), and the rules promised
  logical names for a later RTL. `.align(..)` would then take three
  types: `RectAlign` (overlays, D-7), `egui::Align` (bubble, message,
  carousel) and `AddonAlign` (input-group).
- **Options:** names: (a) accept physical names (the crate is LTR only) and
  drop the RTL promise; (b) a crate `{Start, End}` enum for chat and
  carousel alignment, logical as upstream. Setter: (i) `.align(..)`
  everywhere, the type disambiguates; (ii) name the setter for what it
  aligns (`.alignment(egui::Align)` for messages and bubbles,
  `.snap_align(..)` for carousel, `.addon(align, ..)` for input-group),
  `.align` only for overlays and `.side(Side)` only for edge panels (D-42).
- **Affects:** message, bubble, carousel, input-group, message-scroller,
  sonner.
- **Recommendation:** (a) and (ii). Reason: egui types where they fit
  (D-43), and RTL is not planned; one setter name with several types would
  repeat what D-71 avoids for widths, so `side` stays with `Side` and chat
  alignment takes `alignment`.
- **Covers:** none (review findings A8, A29).

Decision: (a) and (ii) — as recommended.

### D-133 · How a filling widget is capped without `available_width()` arithmetic

- **Case:** T8 has `.max_width` cap a filling widget and text entry
  default to `Metrics::field_max_width`; that is
  `available_width().min(cap)`, which rule 6 of the porting standard
  forbids and the survey lists as a bug (`input.rs:138`). egui's own
  `TextEdit::desired_width(f32::INFINITY)` reads the available width
  inside.
- **Options:** (a) rule 6 allows clamping the available width to a cap
  (as egui does), and the survey item becomes a non-bug; (b) a `layout`
  helper (`layout::capped_width(ui, cap)`) does it, rule 6 points to it,
  and `input.rs:138` is fixed by calling it; (c) widgets never fill; the
  caller sizes them.
- **Affects:** input, textarea, input-group, select, native-select, card,
  alert, slider, progress, `plugin/skills/port-component/SKILL.md` rule 6.
- **Recommendation:** (b), extended: the `layout` module gets the cap helper
  (`layout::capped_width(ui, cap)`) and a breakpoint helper
  (`layout::wider_than(ui, px)`, for field `.responsive(bool)` and sonner's
  narrow layout, D-69), and rule 6 points to both. Reason: one audited place
  for the arithmetic keeps rule 6's intent (no ad-hoc width maths in
  widgets) and gives the cleanup a replacement for `input.rs:138` and
  `tabs.rs`'s inline cap.
- **Covers:** none (review finding A10).

Decision: (b), extended — as recommended. Under D-69's decision the
breakpoint helper serves calendar, sonner, sidebar and pagination by
default, so beside `layout::wider_than(ui, px)` (container width) it offers
a window-width form for upstream's media queries.

### D-134 · Group-position protocol (first, last, stretch)

- **Case:** button-group, toggle-group, input-group and input-otp need
  each child to know its position (first, middle, last) and the group's
  orientation, and a vertical button-group stretches its children to the
  widest; `State::IN_BUTTON_GROUP` exists but nothing sets it. In
  immediate mode the group does not know its children before they paint.
- **Options / Recommendation per item:**
  - `CP-01-7` how grouped children learn their position: `Ui` tag plus a
    child count stored from the previous pass (by group id), with
    `request_discard` when the count or the widest width changes /
    typed `ButtonGroupUi` that counts / data-only item list / repaint
    corners after the closure · `Ui` tag plus previous-pass count
    (D-57); this is a stated exception to the single-pass sizing rule for
    groups.
  - `CP-01-14` group corners and borders: generator child-target recipe
    with `FIRST`/`LAST`/`NOT_FIRST`/`NOT_LAST` and single-side radius and
    border fields / hand-coded in the component · generator (four flags,
    counted in D-11).
- **Affects:** button-group, toggle-group, input-group, input-otp, button,
  input, select trigger.
- **Scope question:** widen this entry into one crate-wide mechanism, the
  pushback review's proposal: a group collects its items' ids, labels and
  enabled flags during the closure, stores them by group id, handles keys
  after the closure on this pass's list, and reads the previous pass's list
  where an item must decide before the rest are drawn (`request_discard` on
  change); it also sets the event filter that stops egui's arrow-key focus
  movement. Users: radio-group's Tab stop when no item matches (D-68),
  End/loop/PageDown in menus and command (D-45), command group hiding
  (D-51), group positions above. Options: (w1) widen (recommended: three
  families would otherwise build it three times); (w2) group positions only,
  each family builds its own list.
- **Covers:** CP-01-7, CP-01-14

Decision: CP-01-7 and CP-01-14 as listed, (w1) — as recommended.

### D-135 · What the theme may change in plain egui widgets

- **Case:** D-40 keeps egui's global `tooltip_delay` because the theme
  should not silently change every app tooltip; D-62 has the cleanup set a
  `ScrollStyle`, which changes every app `egui::ScrollArea`. The two
  reasons point opposite ways.
- **Options:** (a) the theme changes looks of plain egui widgets (colours,
  bar style, spacing) but never behaviour (timing, focus, keys); (b) the
  theme changes nothing beyond what it sets today; (c) per setting.
- **Affects:** `Theme::install`, tooltips, scroll areas, any egui style
  field the theme sets.
- **Recommendation:** (a): a scroll bar's look is a look, a tooltip delay
  is behaviour. Reason: one principle that both D-40 and D-62 then follow.
- **Covers:** none (review finding A27).

Decision: (a), narrowed by P9 — differs from the recommendation (maintainer:
no nearest-colour styling of plain egui widgets). The theme changes a look
of a plain egui widget only where it reaches upstream's exact colour, and
never behaviour; it does not style plain `egui::ScrollArea`, whose thumb
could take only the nearest shared colour, so plain egui scroll areas keep
egui's bars and shadcn's bars come from the crate `ScrollArea` (D-62 (a)).
The "theme sets no `ScrollStyle`" item in D-62's case is then no cleanup
bug.

## Rejected review findings

No Task 7 review finding was rejected: A1–A43 and the re-review findings
B1–B6 and C1–C2 were all applied. The second pushback review's ten issues
and re-review round 3's open items and new findings were all applied too;
where the second pushback review proposed an answer to a real choice, the
choice stays open as an option (D-128 outside click: its (o2) is kept
beside the recommended (o1)).
