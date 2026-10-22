# Implementation notes for shadcn-egui components

Companion of `2026-10-18-api-rules.md` and `2026-10-18-conformance.md`. A
note here is one way to build a required behaviour in egui; the behaviour
in the conformance document is the requirement. Notes are grouped by the
shared infrastructure piece they belong to. Most of them record where egui
does something else than the browser, so that every port works around it
in the same place.

## 1 · Shared code

- One crate path per concern serves every component (rules, concept 7). A
  component that needs a variant extends the path instead of copying it.
  The paths: the text path, the interaction state of a `Response`, a
  part's opacity, the recipe box (fill, border, shadow, focus ring), the
  press of a pass, the overlay stack, the overlay panel frame and its
  placement, the hover timer, the item registry and the generic data item,
  a group's id, the control-plus-label row, the width and breakpoint
  helpers, the roving-focus helper, the list keyboard model, the crate
  `ScrollArea`, the app-variant lookup (18) and the memory-state type
  helper (19).
- A part that one component draws for another lives in the drawing
  component's module: a control's own label is the `Label` module's.
- A type several components share (`Orientation`, `Side`) lives in its own
  module under `components/` and is re-exported as
  `shadcn_egui::components::<Name>`.
- An idle widget costs no heap allocation per frame beyond egui's own:
  accessible names are built inside the `widget_info` closure, styles are
  not cloned per frame, text measured for layout is cached by id, and
  per-widget temporary data leaves egui's memory when it returns to its
  default.

## 2 · Text path

- One text path places all crate text (conformance 2.4) and resolves its
  attributes (conformance 2.3) for every component.
- A data item stores `Atoms<'a>` from `into_atoms()`; `&str` has no
  `Into<Atoms>`.

## 3 · Part opacity

- `Ui::disable` multiplies the painter's opacity by
  `visuals.disabled_alpha`, and `multiply_opacity` cannot raise it again.
  So a crate part sets its painter's opacity (`Painter::set_opacity`) to
  the `Ui`'s opacity with one egui disabled fade taken out, times its own
  value. One helper does this for every part.
- egui fades again on every nested disable (`add_enabled_ui(false)` inside
  a disabled `Ui` gives `disabled_alpha` squared), which the `Ui` does not
  report, so the helper removes one fade only.
- `Theme::apply` sets `disabled_alpha` to the style's control opacity.

## 4 · Recipe box

- egui's `Shadow` is a filled blurred rect. The recipe box clips it to the
  outside of the border box.

## 5 · Per-pass mechanisms

- egui runs `on_begin_pass` hooks and plugins only from `run_ui`. A
  mechanism that registers a hook also runs from the first crate call of
  the pass (conformance 6).
- Previous-pass knowledge relies on `request_discard`.

## 6 · Overlay stack and dismissal

- egui's popup memory keeps one open popup and closes a parent on a click
  in a nested popup's layer. So crate overlays keep their own flags (shown
  with `PopupCloseBehavior::IgnoreClicks`).
- The parent of an overlay is the open overlay whose layer holds the
  trigger (`Response::layer_id`); the trigger's `Response` carries no
  `UiStack`.
- "Top" for a press is the stack as the pass began. egui clears
  `press_origin` on release, so the press position comes from this pass's
  `PointerButton { pressed: true }` event.
- The modal shell does not use `ModalResponse::should_close()` as a whole,
  which closes on any backdrop click.
- The stack's open, close and query operations are public, so an overlay
  built outside the crate nests with crate overlays.
- The popup id of an overlay on a caller's trigger derives from the
  trigger's `Response`.

## 7 · Escape

- egui's `Popup` and `Modal` check Escape inside their own `show`. So the
  crate consumes the key at the start of the pass and marks the top entry
  of the previous pass's stack as closing: in a pass-start hook where egui
  runs one, else in the first crate overlay call of the pass.

## 8 · Focus scope and focus return

- egui ignores a focus request below the previous pass's top modal. So a
  modal moves focus in on the first pass in which it is the top modal, and
  its return request waits for the first pass in which it is no longer
  shown.
- A bare Escape clears egui's focus at pass start, so the return target is
  recorded at open.
- A modal's trigger is the widget egui saw the opening interaction on
  (`interact_widgets`, `Memory::focused`).
- An egui `Popup` senses clicks, which makes its panel focusable; the
  crate keeps the panel out of the Tab order.
- A Tab loop in a non-modal overlay consumes Tab at pass start and focuses
  the next focusable widget of the overlay's layer from the previous pass.
  `Memory::set_modal_layer` is not used: it would also block presses below
  it.
- egui paints a focused widget with its `active` (pressed) look whatever
  the modality. This is why a plain egui widget gets focus back only after
  a keyboard open or close (conformance 5.4).
- egui focuses no clicked widget and fakes a click for Space and Enter on
  any focused clickable widget. The crate gives a pressed widget focus and
  handles activation keys itself (conformance 4.1).

## 9 · Hover timer and tooltip

- The crate tooltip is an own popup on its own timer, not egui's tooltip
  machinery.
- One "press of this pass" helper serves the overlay stack and the hover
  timer.
- A bound hover overlay writes the timer's open and close into the
  binding, and a binding the app changed opens or closes it at once; the
  timer then runs from that state (conformance 5.5).

## 10 · Placement

- egui's `RectAlign::find_best_align` takes an alternative only when the
  whole rect fits, and `Area::constrain` then clamps onto the trigger. So
  the crate picks the side itself (conformance 5.6) and passes one align.

## 11 · Roving focus

- egui moves focus spatially on bare arrows and on Tab at pass start. A
  group or widget that owns arrow keys (a roving group, a vertical
  slider's Left/Right) sets `Memory::set_focus_lock_filter` with both
  arrow axes.

## 12 · Item registry and group context

- A parent pushes a `UiStackInfo` tag; the child reads `ui.stack()`
  (rules, concept 5).
- The item registry keeps the children's count, size and ids from the
  previous pass by id, gives each child its position flags through the
  parent context, and calls `request_discard` when they change.

## 13 · Layout helpers

- A widget may read the available width to fill or to wrap. Any arithmetic
  on it goes through one `layout` helper, which rule 6 of the porting
  standard points to; a breakpoint read (container width or window width)
  goes through a second one, the breakpoint helper.

## 14 · State flags

- `State` becomes a `u128` behind a private field.

## 15 · Icons

- `IconAtom(icon)` builds an image atom whose URI names an entry of a
  process-wide icon registry. The entry records whether the caller gave a
  colour, so a crate widget re-tints only an atom without one.

## 16 · Accessibility

- Roles egui's `WidgetType` lacks are set through `accesskit_node_builder`
  after `widget_info`, which sets a role from `WidgetType`.

## 17 · egui idiom catalogue

The sections of the egui 0.34.3 idiom catalogue that show the egui form
each kind of the rules mirrors. By kind: leaf §Widget builders, §Text,
§Enabled state, §Response methods (`mark_changed`); container §Containers
(`show`, `InnerResponse`, `UiBuilder` stack info, `ScrollAreaOutput`,
`TextEditOutput`), §Widget builders (two-step widgets); anchored overlay
§Popups (`Popup`, `menu_button`, `Popup::open_bool`, `Popup::is_id_open`),
§Anchoring (`RectAlign`), §Open state, §Focus and keyboard, §Response
methods (`on_hover_text(self)`); modal §Popups (`Modal`); data and
immediate items §Selection (`selectable_value`, `radio_value` runs,
`ComboBox::show_ui`), §Atom, §Popups (`menu_button` items); ids §IDs;
sizing §Widget builders (`add_sized`), §TextEdit (`desired_width`,
`f32::INFINITY`); orientation §Direction and layout, §Focus and keyboard
(`EventFilter`); state §widget_info and accessibility.

## 18 · App variants

- The theme keeps the app variants in one map behind an `Arc`, beside the
  app colours, keyed by component and name: the base variant and the
  edit. Cloning a `Theme` stays cheap and `Theme::current` copies no
  recipe.
- The edited recipes are built once per style and kept with the map, not
  per pass; a style change rebuilds them.
- The app case of a variant enum holds the `&'static str` name, so the
  enum stays `Copy`, `Eq` and `const`-constructible. Two constants with
  one name are one app variant.
- A component resolves its variant through one helper that returns the
  recipe stack and the state bits: the built-in variant's recipes, or the
  app variant's edited copies with the base's bits. A name the theme does
  not define fails a `debug_assert!` and resolves to the `Default` variant:
  only the theme's entry knows the base.
- An app variant's recipes take the base's place in the recipe stack
  (base, variant, size). A part whose look comes from the base's state
  bits, not from a variant recipe (the tab trigger), gets its edited copy
  of the part recipe.
- A field the edit sets replaces the base's rules for that field; an edit
  that keeps them adds a rule (`Rule::important`).
- The edit receives the `StyleName` the theme builds for. A `Style` carries
  only its `name`, so the theme maps it to the `StyleName`; a restyled
  style keeps the name of the style it came from.

## 19 · Memory-state types

- A state type keeps its `Id` and the value, as `CollapsingState`, and
  stores it with `insert_temp`, or `insert_persisted` under `.persist`
  (D-35: std types only).
- One crate helper serves every state type. It keeps, beside the value,
  the binding's value as it last saw it. At the start of a call with no
  value in memory, memory takes `.default_<state>` if set, else the
  binding's value, else upstream's default, and the binding takes
  memory's value. Later calls: a binding that differs from the last seen
  value sets memory; else a memory value that differs from the binding
  (a `store`) sets the binding. After a user change, memory and binding
  take the new value. `store` requests the repaint.
- The helper calls `Context::check_for_id_clash` for the state id.
