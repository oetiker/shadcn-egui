# Conformance: what "behaves and looks as upstream" means

Companion of `2026-10-18-api-rules.md` (concept 1). The rules say that
behaviour, looks and defaults are upstream's; this document says
precisely what that means where the browser, bits-ui or CSS do something
an egui port could easily get wrong. It is grouped by concern, not by
component. Every clause holds in every way an app drives egui. How the
crate builds each item in egui is in `2026-10-18-implementation-notes.md`.

## 1 · Reference

- Upstream is shadcn-svelte at `tools/stylegen/PIN`, the bits-ui primitive
  it locks, and their dependencies (vaul, svelte-sonner, embla), as the
  browser runs and renders them.
- Defaults are upstream's, also where egui's counterpart has another
  one: a separator is horizontal, a slider steps by 1, a menu trigger
  fires on pointer down. egui decides no default and no behaviour, and an
  egui layout never decides a component's default (a separator in a
  horizontal row is still horizontal unless the caller says
  `.vertical()`).
- A type's docs name each difference an egui user would not expect.
  A `debug_assert!` catches the case that makes a widget silently useless
  (a slider step larger than its range, which leaves one position).

## 2 · Looks

### 2.1 Visual values

- The style is the single source of every visual value a crate widget
  paints: colour, opacity, size, spacing, line height, font. Values come
  from the active style's recipes, or from `Metrics` where upstream sets
  none (porting standard rule 5). They hold for every style, a restyled
  one included, not only the shipped ones.
- An app variant or app size (rules, axiom 8) draws every value its edit
  does not set as its base does, in every state, style and mode, and a
  value it sets in every one of them.
- egui state (enabled, hover, focus, the `Ui`'s text colour) selects
  which recipe rule applies. An egui setting (`disabled_alpha`,
  `override_text_color`, `strong_text_color`, `weak_text_color`,
  `interact_size`) never decides the look of a crate widget or a crate
  layout helper: a crate row is as tall as its tallest item, as a flex
  row.
- A crate container changes no egui setting for caller content that
  upstream does not set.

### 2.2 Disabled opacity

- A disabled part paints at the opacity its style's `disabled:` rules
  give, multiplied along upstream's part tree as CSS multiplies it, not
  along egui's `Ui` nesting: a part fn inside a container's content
  closure paints at container × part.
- A style that sets `disabled:opacity-25` shows 25 %. A part with no
  `disabled:` rule stays opaque in a disabled `Ui`, as in a disabled
  `<fieldset>`.
- A nested disable in egui (`add_enabled_ui(false)` inside a disabled
  `Ui`) fades twice; the docs name this case.
- Plain egui widgets show the theme's `disabled_alpha`, which the theme
  sets to its style's control opacity (`disabled:opacity-50`).
- Images in content fade with their part.

### 2.3 Text attributes and colour

- Every piece of crate-drawn text and every crate icon resolves each of
  its attributes in one order: what the caller set on it explicitly (a
  `RichText` attribute, an icon's caller colour); else the recipe's
  value; else, for colour, the enclosing `Ui`'s text colour, which plays
  CSS's `currentColor`.
- egui's `strong` and `weak` are no explicit colours: on a crate widget
  such text takes the recipe colour.
- A crate container whose upstream sets a text colour or size for its
  content sets it on its content `Ui`, so crate text and plain egui
  widgets inside take it. What egui widgets cannot take (`leading-*`)
  reaches crate text only.
- The theme's global text colour for plain egui widgets never overrides
  a recipe colour.
- An underline takes the colour of its own text run.

### 2.4 Text placement

- Crate text sits in its line box as Blink places it: baseline = line
  top + floor((line height − (A + D)) / 2) + A, with the font's ascent A
  and descent D each rounded to whole device pixels, and the font's line
  gap unused when a line height is set.
- A text-entry field (`<input>`) rounds that half up instead.
- A and D come from the font's own tables (typo metrics where the font
  sets `USE_TYPO_METRICS`, else `hhea`), by one generic path with no
  per-font table.
- A caller's text without a line height takes the recipe's.
- A font may carry a y correction (a `FontFiles` field, from
  `FONTS.tsv`), 0 by default. Only a font whose capitals still differ
  visibly from Chrome's render on the path above gets a value.
- The test that measures text placement compares against a Chrome
  reference render, not against centred capitals.

### 2.5 Boxes

- Boxes paint as CSS paints them. A fixed `h-*` or `w-*` is the border
  box (`box-sizing: border-box`): content larger than it overflows
  evenly, and the box does not grow.
- An outer `box-shadow` paints only outside the border box, so a
  transparent or translucent fill shows what lies under the box, never
  the shadow.
- Where egui has no primitive for a CSS look, the crate builds it from
  egui shapes to the browser's pixels. Where that is not possible, the
  port stops and states the case with its options to the maintainer. It
  never drops the look and never ships an approximation silently.

### 2.6 Focus ring

- A focus ring (`FOCUS_VISIBLE`) shows exactly when the browser's
  `:focus-visible` would: after focus moved by keyboard; not after a
  pointer press, and not after a focus move the crate makes in response
  to one (an overlay opened by click).
- The modality of the last focus change is tracked crate-wide.

### 2.7 Plain egui widgets

- The theme may change the look of plain egui widgets (colours, spacing)
  only where it reaches upstream's exact colour, and never their
  behaviour (timing, focus, keys).
- The theme leaves plain `egui::ScrollArea` bars as egui draws them: a
  `ScrollStyle` thumb reaches only the nearest shared colour.
- The crate `ScrollArea` hides egui's bars and paints shadcn's:
  transparent track, the exact `bg-border` thumb, hover-only visibility
  first, bits' other visibility types later.

### 2.8 Colours and recipes

- Colours match what the pinned docs page renders, exactly.
- Where svelte and the style CSS disagree, the pinned svelte source
  wins. The style CSS marker serves only where it gives the colours the
  svelte docs page renders (chart tooltip).
- A literal colour of an upstream package (svelte-sonner's description)
  is ported as that literal.
- Upstream rules the browser never applies stay dead, as the browser
  renders them, and are flagged in the module docs.
- A blend the browser applies (the avatar's inner border) is reproduced
  as its resulting colour.
- Style values come from recipes. A recurring upstream class form gets a
  generator rule, not hand-written values (porting standard rule 5);
  a fraction class (the bubble's `max-w-[80%]`) becomes a recipe
  fraction field.
- A class that styles caller content egui renders itself is ignored with
  a reason that fits. An ignore never hides a class silently: the
  IGNORED entries for `*:` and `**:` get diagnostics.
- Palette roles a component needs are added to `Palette` and the palette
  import.

### 2.9 State flags

- Upstream attributes map to flags: `data-open` to `EXPANDED`;
  `data-[side=*]` on panels to four `SIDE_*` flags; group children get
  `FIRST`, `LAST`, `NOT_FIRST`, `NOT_LAST`. A flag has one meaning, the
  attribute it stands for: `SELECTED` keeps one, and `ICON_START` means
  "the content holds an icon with `data-icon="inline-start"`".
- A typed icon slot sets `ICON_START` (or `ICON_END`) where that
  component's upstream demos put `data-icon` on their icons (button), and
  not where they do not (tab trigger, toggle).
- A part tells its container that it exists only where a recipe styles
  that fact.

## 3 · Sizing and layout

### 3.1 Width

- The width a widget takes is the requested (or default) width, capped
  to the available width (to the window, for a floating overlay), then
  raised to the content minimum.
- Where nothing finite is requested and the available width is unbounded
  (a horizontal `ScrollArea`), the width is the content minimum.
- The content minimum is what the widget cannot shrink below: its
  padding plus its fixed parts (chevron, gaps, a thumb's length, one
  glyph for text entry), never 0.
- `f32::INFINITY` means fill.
- Whether a widget fills or is content-sized follows from upstream's
  default width: `w-full` fills, `w-fit` or no width is content-sized.
  Text entry fills up to `Metrics::field_max_width`; select and
  native-select are `w-fit`; a button is content-sized. Where a widget has
  a width word (rules, Sizing), this decides which.
- A content-sized widget in a justified layout (`ui.add_sized`) takes the
  layout's width, as egui's `Button`; its content stays placed as the
  recipe places it.
- A decorative fill never widens an auto-sized parent: in egui's sizing
  pass it reports its minimum.

### 3.2 Single pass and group knowledge

- Leaf widgets size themselves before placement (single pass).
- A group that needs its children's count, size or ids (button-group,
  toggle-group, input-group, input-otp; radio-group's Tab stop, menu and
  command keys, command group hiding) gets them from the previous pass,
  keyed by id, and gives each child its position flags (`FIRST`, `LAST`,
  `NOT_FIRST`, `NOT_LAST`) through the parent context.
- A container uses what it learns only after its closure (a footer was
  drawn) after the closure only.
- Responsive breakpoints are the one written-down exception to
  single-pass sizing (3.5).

### 3.3 Height

- Heights resolve as CSS resolves them. `h-full` takes a parent height
  only where the parent has a definite one: a crate container whose
  height is fixed before its content is drawn (a recipe `h-*`, or the
  height resizable takes from the caller's `Ui`).
- An egui parent (`add_sized`, `allocate_ui`) is no definite parent.
  Against any other parent `h-full` is auto, and the recipe or `Metrics`
  minimum applies (a vertical slider's `min-h-40`).
- `self-stretch` in a row is the height of the row's tallest other item,
  from the previous pass; in a column it gives no height.
- egui's available height (the rest of the panel) and `interact_size`
  are never a parent height.

### 3.4 Text wrapping

- Text wraps where the browser would wrap it: at the available width, in
  a row as in a column.
- Label wraps by default; every part drawn through Label (a field label,
  a control's own label) does the same.

### 3.5 Responsive breakpoints

- Breakpoints apply by default, as upstream: a media query (calendar
  `md:`, sonner, sidebar, pagination) reads the window width; a container
  query (field `@md/field-group`) reads the container's width.
- Every component and layout helper carries upstream's responsive
  classes (`sm:`, `md:`, container queries, the stacking of a demo's
  `flex-col sm:flex-row` layout) and applies them by itself. A screen
  built from them works at a phone width and at a desktop width with the
  same code.

### 3.6 Painting bounds

- Components paint inside their allocation.

## 4 · Input

### 4.1 Pointer, focus and keys

- Pointer and keys act as on upstream's element in the browser.
- A press focuses, without a ring (2.6), what the browser focuses on
  press (a button, a tab trigger, a checkbox, bits' slider thumb). So
  keys work after a mouse press, an overlay returns focus to its trigger,
  and focus-bound behaviour follows (automatic tab activation selects on
  press).
- Activation keys are the element's: `<button>` Enter and Space, `<a>`
  (a link look) Enter only, bits checkbox Space only, switch both.
- No key upstream lacks. A key egui does not report on a platform (Meta
  outside macOS) is left out there, and the docs say so.
- A widget with the link look acts on click and focus, with Enter as its
  only key, and shows the pointing-hand cursor.

### 4.2 Roving focus

- Group keyboard focus works as the group's primitive: axes, Tab stop
  and its memory, wrap.
- An arrow key off the group's axes does nothing, and focus stays.
- The Tab stop is the primitive's also in degenerate cases: a tab list
  whose value matches no trigger has none (bits gives every trigger
  `tabindex=-1`).

### 4.3 List keyboard model

- A filtered list hides a group none of whose items match, heading
  included, and hides separators while a search is active (bits
  `CommandGroup`, `alwaysRender`).
- Pickers and menus share one list keyboard model: typeahead (1 s
  buffer), Home/End, loop, skipping disabled items, configured per
  component as its primitive does (PageUp/PageDown move in the select
  list; in a bits menu they act from the panel only, to the first and
  last item).
- The pointer works as bits: a trigger opens on pointer down, and a
  press-drag-release chooses the item under the pointer.

### 4.4 Values and selection

- A widget writes the caller's value only in response to user input. A
  change the app makes through a state type reaches a binding on the
  next call (rules, Memory-owned state).
- A bound value off the step grid or outside the range shows at its
  nearest position and stays as it is until the user changes it (bits).
- A value that matches no item marks nothing and shows the placeholder.
- Deselect by a second click behaves as upstream (select
  `allowDeselect`, calendar `preventDeselect`; toggle-group always
  clears).
- Built-in Previous/Next buttons disable themselves at the ends.

### 4.5 A choice control's own label

- A control's own label is drawn through the Label text path (`RichText`
  attributes kept) and composed with the control as that control's
  upstream demo composes the two, with that demo's gap per control from
  `Metrics`.
- It wraps as Label does, in the width beside the control.
- A press on it acts on the control (`<label for>`).

## 5 · Overlays

### 5.1 The overlay stack

- Crate overlays are independent of each other and nest. They form one
  overlay stack: an overlay's parent is the open overlay that contains
  its trigger, and a modal is a root.
- Closing an overlay closes its descendants.
- Hover overlays (hints: tooltip, hover-card) are on the stack for
  Escape, never for presses, and never return focus.
- Every crate overlay and picker joins the stack, list pickers (select,
  native-select, combobox) included. An overlay built outside the crate
  joins it through the public stack operations.
- A trigger gets its `expanded` look one pass late, because the popup id
  derives from the trigger's `Response`.

### 5.2 Dismissal

- Dismissal is bits' dismissible layer. A press (pointer down) outside
  the top non-hint overlay closes that overlay only, and the same press
  reaches no other overlay in that pass.
- A press on an overlay's own trigger, or on a hint opened from inside
  the overlay, is no outside press, and the trigger keeps toggling on
  click.
- A modal closes on a press on its backdrop only while it is the top
  overlay. A modal closes by reporting `should_close()`; the app clears
  its flag (rules, Modal).
- A press counts also when its release arrives in the same pass.
- Only a primary-button press dismisses (a context-menu trigger's
  secondary press too). A touch dismisses on its release, not on its
  press.
- Navigation-menu hover items stay open on an outside touch.

### 5.3 Escape

- Escape closes the top crate overlay only, an open hint first, before
  any crate overlay, `Modal` or egui popup reacts to that press.
- Escape with a modifier counts too.
- While no crate overlay is open, the crate leaves Escape to the app.

### 5.4 Focus

- Focus is bits' focus scope. An overlay whose bits content traps focus
  (dialog, alert-dialog, sheet, a popover not opened by hover) moves
  focus in on open, to its first focusable widget (alert-dialog its
  Cancel button; drawer none, as vaul), and Tab and Shift+Tab loop inside
  it.
- An overlay that does not trap moves focus on open as its primitive
  does (a menu into its panel, where the highlighted item takes the keys;
  a list picker keeps it on its trigger), and Tab closes it and moves on
  from its trigger.
- Hints move no focus and never return focus.
- However an overlay closes (Escape, an outside press, a close part,
  `ui.close()`, the binding, a modal's flag), focus returns one pass
  later, if it is gone or inside the closed overlays: to the trigger the
  overlay opened from (for a modal, the widget the opening press or key
  landed on), else to the widget that held focus when it opened.
- Crate widgets get focus on press and on return. A plain egui widget
  gets focus back only when the overlay was opened or closed by keyboard,
  so no pointer action leaves a pressed look the browser does not show.
- An overlay's panel is no Tab stop (bits `tabindex=-1`).

### 5.5 Hover overlays

- Hover overlays open on keyboard focus too.
- Delays are upstream's: tooltip 0 (the shadcn provider's
  `delayDuration`), hover-card bits' 0.7 s open and 0.3 s close. egui's
  global `tooltip_delay` is neither read nor changed.
- Hint overlays open for disabled triggers; action overlays do not.
- A bound open flag behaves as bits' `bind:open`: the app opens or closes
  the overlay by writing it, hover and focus then close or keep it as
  usual, and every change the overlay makes shows in the binding. The
  open item of menubar and navigation-menu binds the same way.

### 5.6 Collision

- Collision behaviour is upstream's: bits `shift()` with `limitShift`,
  then floating-ui's `flip()` defaults.
- Along its alignment axis an anchored overlay shifts to stay in the
  window, never past its trigger.
- When it still overflows on any side, it tries in order, for a start or
  end alignment: the other alignment, the opposite side, and the
  opposite side with the other alignment (centred: the opposite side
  only).
- It takes the first that fits, else the one that fits on the side axis
  with the least overflow on the alignment axis, else the one with the
  least total overflow.
- It never covers its trigger. A part that depends on placement (an
  arrow) follows the side it took, and works for every placement the
  overlay offers.
- The chart tooltip stays inside the chart.

## 6 · Per-pass mechanisms

- Every per-pass mechanism (the overlay stack, Escape, focus return,
  hover timers, the heading font) behaves the same whether the app runs
  egui through `Context::run_ui` or through its own
  `begin_pass`/`end_pass` loop, and whether or not it called
  `Theme::apply`.
- A pass-start hook may give a mechanism its place in the pass, never
  its correctness: where no hook runs, the first crate call of the pass
  does the work.
- The one allowed difference: previous-pass knowledge (3.2) relies on a
  discarded pass, so a loop without the discarded pass shows one stale
  first frame, which the docs say.

## 7 · Animation

- Components animate where upstream animates (spinner, skeleton pulse,
  progress value, accordion and collapsible height, sheet slide, sidebar
  width, navigation-menu viewport, carousel slide, toast stack, shimmer,
  scroller), with upstream's timing.
- `style.animation_time == 0` counts as reduced motion.
- Caret blink is egui's.

## 8 · Accessibility

- Every custom-painted widget reports upstream's accessible role and
  state, with names and values in full text, never the truncated galley.
- A part upstream hides from assistive technology (a decorative
  separator) reports none. A wrapper upstream gives a role (accordion's
  `role="heading"`) reports it.
