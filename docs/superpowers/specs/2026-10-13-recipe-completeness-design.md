# Recipe completeness

## Goal

GitHub issue #6: the tab trigger draws its colours, focus ring and line
indicator from values fixed in `tabs.rs`, so an app cannot restyle them
through `Style`. Ten other component files fix colours or sizes the same
way. This spec makes every visual value of every component come from the
style data (a recipe or the theme metrics), so an app can change any of them
through `theme.style`. That closes #6: its use case (a filled `primary`
active tab, a solid 2 px focus ring) becomes a few recipe edits with the
`Rule::important` helper (see "Overriding a value").

App colours and app variants, which let an app define its own kinds of
components, are a follow-up with its own spec (see "Follow-up").

## Why the port has this problem

Upstream, a component's class string holds the style hook (`cn-tabs-trigger`)
followed by the base classes that all styles share. The published registry
replaces the hook with the style's `@apply` classes in place and does no
merge at build time. At runtime `cn()` (tailwind-merge) keeps the later of
two classes that conflict under the same modifiers, so a base class wins
over a style class in the same state. An app owns the copied component file
and may edit the base classes.

The port translated only the style classes into recipes and wrote the base
classes into the component code, marked with `// base: "…"`. Code in a crate
from crates.io is not code the app owns, so these values are out of reach.
The `recipe.rs` doc comment states the old rule: "An empty Prop means the
style does not set that property; the component then uses its base value."

## Decisions

- Every visual value (colour, size, spacing, geometry, opacity) comes from a
  recipe or from the theme metrics. Only structure stays in code: layout
  order, hit testing, keyboard handling, popup placement.
- Stylegen translates the full class string of each part, built the way the
  published registry builds it. There is no separate base layer.
- Where upstream and today's code disagree, upstream wins. Each such case is
  listed below, shown to the maintainer as a before/after snapshot and
  re-baselined only with their approval.
- Snapshots cover all components in all seven exported styles, light and
  dark, before any change.
- The work lands as two pull requests: A (snapshot coverage) and B
  (stylegen, components, tests, skills; closes #6).
- The next release is 0.5.0: `Recipe` and `Style` gain public fields, which
  breaks struct literals.

## Known upstream disagreements

Found so far; the plan adds every further snapshot difference here.

- Luma and rhea, tabs: `.cn-tabs-trigger` applies `border-transparent!`
  (`!important`), which beats the base classes `focus-visible:border-ring`
  and `dark:data-active:border-input`. Today `tabs.rs:268-284` draws a
  `Ring` border on focus and an `Input` border on the dark active tab in
  these styles. After the change they draw no border there.

## Values that are fixed in code today

Kind 1 values block an app and move into recipes. Kind 2 values are
fallbacks used only when the recipe has no value; they stay, each with a
`// behaviour-ok: inherits …` reason.

### Kind 1

| File | Fixed value | Data after the change |
|---|---|---|
| `tabs.rs` | trigger text: `Foreground`, `MutedForeground` (dark), `Foreground/0.6` | `tabs_trigger.text`, rules for hover, dark, active |
| `tabs.rs` | active fill: `Background`, `Input/0.3` (dark), none (line) | `tabs_trigger.bg` |
| `tabs.rs` | active border `Input` (dark), `Ring` (focus) | `tabs_trigger.border_color` |
| `tabs.rs` | focus ring `Ring/0.5`, 3 px | `tabs_trigger.ring_color`, `ring_width` |
| `tabs.rs` | focus outline `Ring`, 1 px | `tabs_trigger.outline_color`, `outline_width` (new) |
| `tabs.rs:100` | trigger height: list content height minus 1 px | `tabs_trigger.height_inset` (new) |
| `tabs.rs` | line indicator `Foreground`, 2 px, 5 px below, shown when active | `tabs_trigger_after`: `bg`, `height`, `inset_bottom`, `opacity` |
| `tabs.rs` | disabled opacity 0.5 | `tabs_trigger.opacity` with the `DISABLED` rule |
| `tabs.rs:229` | list track `Muted` | `tabs_list_variant_default.bg` |
| `tabs.rs` | line variant gap 4 | `tabs_list_variant_line.gap` |
| `tooltip.rs:40` | fill `Foreground`, text `Background` | `tooltip_content.bg`, `text` |
| `tooltip.rs` | arrow fill `Foreground` | `tooltip_arrow.bg` |
| `tooltip.rs` | max width 320 | `tooltip_content.max_width` |
| `separator.rs:16` | line colour `Border` | `separator.bg` (new base-only recipe) |
| `separator.rs:12` | thickness `metrics.border` | `separator.height` / `width`, falling back to `metrics.border` |
| `button.rs`, `input.rs`, `select.rs`, `switch.rs`, `slider.rs` | disabled opacity 0.5 | `opacity` with the `DISABLED` rule |
| `button.rs:263`, `badge.rs:163` | link underline 1 px | `t.metrics.border` (upstream sets no thickness) |
| `checkbox.rs:59`, `switch.rs:77` | hit area 12 px by 8 px around the control | `checkbox.hit_inset_x`/`_y`, `switch.hit_inset_x`/`_y` |
| `popover.rs` | width 288 | `popover_content.width` |
| `dialog.rs` | footer gap 8 | `dialog_footer.gap` |

The separator reads its thickness from the recipe (`data-horizontal:h-px`,
`data-vertical:w-px`) and falls back to `metrics.border`. An app that
changed `metrics.border` to restyle separators now changes the recipe
instead; a CHANGES entry says so. The link underline keeps reading
`t.metrics.border`: upstream sets no thickness for it.

`sideOffset = 4` in popover (`popover.rs:24`) and select (`select.rs:387`)
is a component prop upstream, not a class. It becomes a builder method
`side_offset(f32)` with default 4.

### Kind 2

These have no upstream class, so the recipe stays empty and the component
uses the inherited value:

- `button.rs:217`, `badge.rs:129`, `select.rs:310`, `select.rs:450`,
  `label.rs:51`, `input.rs:104`: text inherits `Foreground` or
  `PopoverForeground`. `label.svelte` and `cn-label` set no text colour;
  `cn-input` and the input base set only `file:text-foreground`.
- `label.rs:81` (`description()`): reads `card_description`, falls back to
  `MutedForeground`.
- `card.rs:228`, `dialog.rs:283`: border falls back to `Border`, from
  shadcn's base layer (`* { @apply border-border outline-ring/50 }`).

The component still reads the recipe first in every Kind 2 case, so a value
an app sets wins.

`input.rs:119` (placeholder `MutedForeground`) is Kind 1: upstream sets
`placeholder:text-muted-foreground`, which becomes `input.text` under the
`PLACEHOLDER` rule.

## Stylegen

### Sources

The svelte component files of every part in `markers.txt`
(`docs/src/lib/registry/ui/<dir>/*.svelte`) are copied into
`tools/stylegen/tests/fixtures/ui/<dir>/` by `generate`, as the style CSS
is today, and `drift` reports a copy that differs from the checkout.

The extractor reads a part's class string from:

- `cn("…", "…", …)` with one or more string arguments, joined in order;
- `tv({ base, variants })`: the base string for the part, each variant
  string for its variant marker;
- a static `class="…"` attribute;
- a `class=` passed to a nested component (dialog close `<Button>`).

Where a file has the same part in several `{#if}` branches (input's
`type === "file"` branch), the plan names the branch the crate draws; the
extractor takes that one and the test asserts the choice.

`disabled:` on the slider thumb is ignored by a named exception: bits-ui
renders the thumb as a `<span>`, where `disabled:` never matches. The
source does not say which element a bits-ui primitive renders, so further
cases are added to the exception list by name, each with its reason.

### Building the string

For each style and each part, stylegen:

1. takes the part's extracted class string;
2. removes the markers only the shadcn CLI rewrites (the registry keeps
   some as text and drops the classes of `*-logical`; the crate models
   none of them):
   `cn-font-heading`, `cn-menu-*`, `*-logical`, `cn-rtl-flip`,
   `cn-logical-sides` (the list `drift::upstream_markers` already uses,
   moved to one shared place);
3. replaces each marker listed in `markers.txt` with the style's `@apply`
   classes, in place;
4. translates the result with the existing pipeline; tailwind-merge keeps
   the later class per field and modifier set, and `!important` keeps its
   rank bit.

Each generated `<style>.rs` then holds complete recipes. A part without a
`cn-*` marker gets a recipe named after its `data-slot`. In `markers.txt`
such a part is written `slot:<data-slot>`, for example
`separator.rs separator: slot:separator`. `drift` skips `slot:` entries
when it compares markers with upstream.

### New fields and translations

- `outline-<n>` and `outline-<colour>` map to the new fields
  `outline_width` and `outline_color`. `outline-none` and `outline-hidden`
  set `outline_width` to 0.
- `h-[calc(100%-Npx)]` maps to the new field `height_inset`: the part is
  the parent's content height minus N.
- `after:` classes keep their current mapping for hit areas
  (`after:-inset-*` to `hit_inset_x`/`hit_inset_y`, used by the slider
  thumb, now also checkbox and switch). Every other `after:` class goes to
  a second recipe of the part, `<marker>_after`, using the existing fields
  (`bg`, `width`, `height`, `inset_*`, `opacity`). The tab line indicator
  is the only such part today.
- `placeholder:` maps to the `PLACEHOLDER` state.
- `data-highlighted:` (select item) maps to the `FOCUS` state.
- `inset-0`, `inset-x-*`, `inset-y-*` (also under `after:`) map to the
  `inset_*` fields.
- `fill-<colour>` maps to `bg` (the tooltip arrow is painted as a shape).
- `ring-0` sets `ring_width` to 0. `text-current` leaves `text` empty
  (inherit).
- `w-fit`, `min-w-0`: `w-fit` is the components' default sizing and is
  ignored with that reason; `min-w-0` maps to `min_width` 0.

Opacity multiplies down the part tree, as in CSS: a component multiplies a
part's `opacity` with its parent part's.

### Ignored classes

Each is listed in `utility.rs` or `variant.rs` with a reason, by group:

- Structure: `flex`, `grid`, `inline-flex`, `block`, `items-*`,
  `justify-*`, `justify-self-*`, `self-*`, `place-content-*`, `relative`,
  `absolute`, `fixed`, `col-*`, `row-*`, `grid-cols-*`, `grid-rows-*`,
  `auto-rows-*`, `flex-col`, `flex-col-reverse`, `flex-row`, `flex-1`,
  `grow`, `shrink-0`, `@container/*`.
- Browser behaviour: `transition-*`, `pointer-events-none`, `select-none`,
  `touch-none`, `cursor-*`, `scroll-my-*`, `isolate`, `z-*`,
  `whitespace-nowrap`, `line-clamp-*`, `overflow-*`, `peer`, `group/*`.
- Sizing that is the component's default: `h-fit`.
- Selectors the crate does not model: `[&_svg]:…`, `[&>svg]:…` other than
  `size-*`, `rtl:…`,
  `has-data-[slot=card-action]:…`, `has-data-[slot=card-description]:…`.
  The plan checks each against today's drawing; one that changes a pixel
  gets a field or a state instead.
- Placement computed by egui: the bits-ui anchor variables
  (`h-(--bits-select-anchor-height)`, `origin-(--bits-…)`, …) and the
  dialog centring (`top-1/2 left-1/2 -translate-*-1/2`).

The tooltip arrow offsets (`data-[side=*]:translate-*`) stay ignored: the
arrow sits at the edge egui places the tooltip on, which is placement. The
reason text of that ignore in `variant.rs` changes from "slide-in direction
of an open animation" to say so.

`reports/<style>.md` lists the ignored classes per part. An exported style
with an unsupported class still fails generation.

### Tests

- `tests/published.rs` compares the concatenated class string of each part
  (style classes in place of the marker, then the base classes, before any
  merge) with the published vega and nova JSON. Both sides are normalised
  first: CLI markers removed (the published JSON keeps `cn-menu-target`,
  `cn-menu-translucent` and `cn-font-heading` as text) and `calc()` spacing
  unified (`calc(100%_-_1px)` equals `calc(100%-1px)`).
- Golden tests for `outline_*`, `height_inset`, a `<marker>_after` part, a
  `slot:` part, CLI-marker removal and the extractor's string forms.
- The `base_quotes` check in `drift` is removed with the `// base:` quotes.

## Overriding a value

A rule's rank is a Tailwind variant bitmask that an app cannot work out by
hand, and the rule an app adds must beat rules such as
`dark:data-active:bg-input/30`. The crate exports the important rank bit
as `Rule::IMPORTANT` and a helper `Rule::important(when, value)`, which
builds a rule with that bit, as a Tailwind `!` class does. An important
app rule beats every non-important generated rule under the same
condition. Against upstream's own `!` classes (luma and rhea
`border-transparent!`) the normal variant order decides, and on equal rank
the app rule wins because it comes later in the rule list.

```rust
let style = Arc::make_mut(&mut theme.style);
style.tabs_trigger.bg.rules.push(Rule::important(
    When::all(State::SELECTED),
    Paint::Role(Role::Primary, 1.0),
));
```

The plan may adjust spelling (`When::all`), not shape. The build-ui skill
shows this idiom.

## Components

Each Kind 1 value becomes a `pick(..)` from the part's recipe stack, for
example `[tabs_trigger]` with the variant state bits. The `// base:`
comments are removed. The doc comment in `recipe.rs` changes to: an empty
`Prop` means upstream sets nothing; the component then uses the inherited
or initial value.

## Performance

`Theme.style` is an `Arc<Style>`, so larger recipes do not make
`Theme::current` copy more. The cost to watch is `pick(..)` over more rules.
`examples/bench_frame.rs` builds the gallery screen in a headless
`egui::Context` for a few thousand frames in each of the seven styles and
prints the median time and the allocations per frame, with a counting
allocator in the example and no new dependency. It runs on one thread and
is measured before step 3 and after step 4. The benchmark is a manual tool,
not a CI gate.

## Tests

- Snapshots: `snapshot_all` covers vega, nova, maia, lyra, mira, luma and
  rhea, light and dark, for every component. It gets its own seven-style
  theme list; `common::themes()` and the measurement tests that use it
  (for example `button_height_follows_style`, which expects 32 for nova and
  36 for the rest) stay on vega and nova. For tabs the state matrix is:
  default and line variant; inactive, hovered, active, disabled; focused
  active; each in light and dark.
- Through the stylegen and component steps, every snapshot stays identical
  except the listed upstream disagreements.
- Issue #6: a test changes `tabs_trigger` through `theme.style` with
  `Rule::important` (active fill `Primary`, text `PrimaryForeground`, solid
  2 px ring, no outline), in light and dark, and the snapshot shows all
  four.
- `tests/tokens.rs`: a `Role::…` or a float literal in a component needs
  `// behaviour-ok: <reason>`; `// base:` no longer exempts a value, and
  `1.0` is exempt only as a factor (`* 1.0`, `/ 1.0`), not as a length or a
  width (`- 1.0`, `Stroke::new(1.0…`). Arithmetic that is not a design
  value gets `// behaviour-ok:` with its reason: the `clamp(0.0, 1.0)` and
  ±1.0 direction signs in `slider.rs`, `.max(1.0)` in `icon.rs`, the
  `unwrap_or(1.0)` scale factors in `label.rs` and `checkbox.rs`.
  `icon_raster.rs` is a rasterizer, not a component, and is skipped as a
  file.

## Order of work

1. Pull request A: snapshot coverage of all seven styles and the tabs state
   matrix, from the current code, reviewed by the maintainer.
2. Frame benchmark, first measurement.
3. Stylegen: svelte fixtures, extractor, CLI-marker removal, full class
   strings, new fields, `after` parts, `slot:` parts, ignore list,
   `published.rs`. Regenerate; snapshots unchanged, since no component
   reads the new values yet.
4. Components, one at a time; after each, snapshots unchanged except listed
   disagreements. Stricter `tokens.rs`; `base_quotes` removed; `side_offset`
   builder methods. Second measurement.
5. Skills: build-ui shows restyling through `theme.style` with
   `Rule::important`; port-component
   states that base classes are generated and that a component holds no
   literal colour or size. CHANGES entries ending with `(#6)`, one of them
   naming the luma and rhea tab border change and one the separator
   thickness.

Steps 2 to 5 are pull request B.

## Follow-up: app colours and app variants

A separate spec after B. Settled so far:

- App colours live in a `Theme`-level map of light/dark pairs behind an
  `Arc`. `Palette` stays `Copy` and single-mode. `Role::custom(name)` names
  one; a missing name draws as `Foreground` and warns once.
- App variants for button and badge: a custom case of the variant enum and
  a map in `Style` from it to a `Recipe` that takes the built-in variant's
  place in the stack. Tabs are left out, because the trigger takes its
  variant look from state bits, not from a variant recipe.
- The benchmark measures the colour map.

## Out of scope

- A per-instance or per-region style override.
- Tabs keyboard focus (every trigger is a tab stop; upstream has one tab
  stop and arrow keys). A separate pull request after B.
- Snapshots of sera (not an exported style).
