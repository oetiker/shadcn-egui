# Recipe completeness, app variants and app colours

## Goal

GitHub issue #6: the tab trigger draws its colours, focus ring and line
indicator from values fixed in `tabs.rs`, so an app cannot restyle them
through `Style`. Nine other component files fix colours or sizes the same
way. This spec makes every visual value of every component come from a
recipe, and adds the two things an app needs to define its own look:
colours of its own and variants of its own.

## Why the port has this problem

Upstream, a component's class string holds the style hook (`cn-tabs-trigger`)
followed by the base classes that all styles share. The published registry
replaces the hook with the style's `@apply` classes in place, and at runtime
`cn()` (tailwind-merge) keeps the later class of two that conflict, so a base
class wins over a style class in the same state. An app owns the copied
component file and may edit the base classes.

The port translated only the style classes into recipes and wrote the base
classes into the component code, marked with `// base: "…"`. Code in a crate
from crates.io is not code the app owns, so these values are out of reach.
The `recipe.rs` doc comment states the old rule: "An empty Prop means the
style does not set that property; the component then uses its base value."

## Decisions

- Every visual value (colour, size, spacing, geometry, opacity) comes from a
  recipe. Only structure stays in code: layout order, hit testing, keyboard
  handling, popup placement.
- Stylegen translates the full class string of each part, built the same way
  the published registry builds it. There is no separate base layer.
- Apps get colours of their own (`palette.add`, `Role::custom`) and variants
  of their own for every component that has variants.
- No per-instance or per-region style override (scope). A change for the
  whole app goes through `theme.style`; a different kind of component is a
  variant. Recipe completeness keeps a scope possible later.
- Snapshots cover all components in all seven exported styles, light and
  dark, before any change, so the refactor is proven pixel-identical.
- The next release is 0.5.0: `Recipe`, `Style`, `Role` and the variant enums
  change in ways that break struct literals and exhaustive matches.

## Values that are fixed in code today

Kind 1 values block an app and move into recipes. Kind 2 values are
fallbacks used only when the recipe has no value; they stay.

### Kind 1

| File | Fixed value | Recipe after the change |
|---|---|---|
| `tabs.rs` | trigger text: `Foreground`, `MutedForeground` (dark), `Foreground/0.6` | `tabs_trigger.text`, rules for hover, dark, active |
| `tabs.rs` | active fill: `Background`, `Input/0.3` (dark), none (line) | `tabs_trigger.bg` |
| `tabs.rs` | active border `Input` (dark), `Ring` (focus) | `tabs_trigger.border_color` |
| `tabs.rs` | focus ring `Ring/0.5`, 3 px | `tabs_trigger.ring_color`, `ring_width` |
| `tabs.rs` | focus outline `Ring`, 1 px | `tabs_trigger.outline_color`, `outline_width` (new) |
| `tabs.rs` | line indicator `Foreground`, 2 px, 5 px below, shown when active | `tabs_trigger_after`: `bg`, `height`, `inset_bottom`, `opacity` |
| `tabs.rs` | trigger height minus 1 px, disabled opacity 0.5 | `tabs_trigger.height`, `opacity` |
| `tabs.rs` | list track `Muted`, line variant gap 4 | `tabs_list_variant_default.bg`, `tabs_list_variant_line.gap` |
| `tooltip.rs:40` | fill `Foreground`, text `Background`; arrow fill | `tooltip_content.bg`, `text`; `tooltip_arrow.bg` |
| `tooltip.rs` | max width 320 | `tooltip_content.max_width` |
| `separator.rs:16` | line `Border`, thickness 1 px | `separator.bg`, `height`/`width` (new recipe) |
| `input.rs:104` | text `Foreground` | `input.text` |
| `input.rs:119` | placeholder `MutedForeground` | `input.text` with the `PLACEHOLDER` rule |
| `label.rs:51` | text `Foreground` | `label.text` |
| `button.rs`, `input.rs`, `select.rs`, `switch.rs`, `slider.rs` | disabled opacity 0.5 | `opacity` with the `DISABLED` rule |
| `checkbox.rs`, `switch.rs` | hit area 12 px by 8 px around the box | `checkbox_after`, `switch_after`: `inset_*` |
| `popover.rs` | width 288 | `popover_content.width` |
| `dialog.rs` | footer gap 8 | `dialog_footer.gap` |

The list is complete for the code at `b7c769d`; the stricter token test
(section "Tests") is the check that nothing was missed.

### Kind 2

`button.rs:217`, `badge.rs:129`, `select.rs:310`, `select.rs:450`,
`label.rs:81`: text falls back to `Foreground` or `PopoverForeground`.
`card.rs:228`, `dialog.rs:283`: border falls back to `Border`. These copy
CSS inheritance and Tailwind's preflight (`border-color: var(--border)`).
Each gets a `// behaviour-ok: inherits …` reason.

## Stylegen

For each style and each part, stylegen:

1. reads the part's class string from the pinned svelte source
   (`docs/src/lib/registry/ui/<dir>/*.svelte`), including the variant
   tables of `tv({ base, variants })`;
2. replaces `cn-<marker>` with the style's `@apply` classes, in place;
3. translates the result with the existing pipeline; tailwind-merge keeps
   the later class per field and variant set.

Each generated `<style>.rs` then holds complete recipes. A part without a
`cn-*` marker (separator) gets a recipe named after its `data-slot`, filled
from the base classes only; `markers.txt` lists these parts.

New translations:

- `outline-<n>` and `outline-<colour>` map to the new fields
  `outline_width` and `outline_color`. `outline-none` and `outline-hidden`
  keep their current meaning.
- `after:` classes go to a second recipe of the part, `<marker>_after`,
  using the existing fields (`bg`, `width`, `height`, `inset_*`,
  `opacity`). The tab line indicator and the checkbox and switch hit areas
  use it.

Classes that stay out are listed in `utility.rs` with a reason, by group:

- Structure: `flex`, `grid`, `inline-flex`, `items-*`, `justify-*`,
  `relative`, `absolute`, `fixed`, `col-*`, `row-*`, `self-*`, `shrink-0`,
  `flex-1`, `grow`, `place-content-center`.
- Browser behaviour: `transition-*`, `pointer-events-none`, `select-none`,
  `touch-none`, `cursor-*`, `scroll-my-*`, `isolate`, `z-*`,
  `whitespace-nowrap`, `line-clamp-*`, `overflow-*`, `peer`, `group/*`.
- Placement computed by egui: the bits-ui anchor variables
  (`h-(--bits-select-anchor-height)`, `origin-(--bits-…)`, …) and the dialog
  centring (`top-1/2 left-1/2 -translate-*-1/2`).

The tooltip arrow offsets (`translate-y-[calc(-50%+2px)]` and similar) are
decided in the plan: a `translate_*` field if they are geometry, an ignore
with a reason if they are placement.

`reports/<style>.md` lists the ignored classes per part. An exported style
with an unsupported class still fails generation.

`tests/published.rs` compares the whole merged class string of each part
with the published vega and nova JSON, not only the style classes. The
`base_quotes` check in `drift` is removed with the `// base:` quotes.

## Components

Each kind 1 value becomes a `pick(..)` from the part's recipe stack, for
example `[tabs_trigger, tabs_trigger_<variant>]`. The `// base:` comments are
removed. The doc comment in `recipe.rs` changes to: an empty `Prop` means
upstream sets nothing; the component then uses CSS's inherited or initial
value.

## App colours

- `Role::custom(name: &'static str)` names an app colour. `Role` stays
  `Copy`.
- `theme.palette.add(name, light, dark)` registers it with a value per mode.
- A `Paint` with a custom role that the palette does not have draws as
  `Foreground` and logs once, for example
  `shadcn-egui: colour "brand" is not in the palette; using "foreground"`.

## App variants

- `ButtonVariant`, `BadgeVariant` and `TabsVariant` each get a custom case,
  made with `ButtonVariant::custom("brand")`, usually kept as a `const`.
- `Style` gets a map per such component from the custom variant to its
  `Recipe`, for example `style.button_variants`.
- A custom variant's recipe takes the place of the built-in variant's
  recipe in the stack, `[button, <variant>, <size>]`, so size, radius and
  focus ring still come from the style. An app can start from a built-in
  variant: `style.button_variant_outline.clone()`.
- A custom variant that the map does not have draws as the default variant
  and logs once.

```rust
const BRAND: ButtonVariant = ButtonVariant::custom("brand");
theme.palette.add("brand", light, dark);
theme.palette.add("brand-foreground", light_fg, dark_fg);
let mut r = Recipe::default();
r.bg = Prop::fixed(Paint::Role(Role::custom("brand"), 1.0));
r.text = Prop::fixed(Paint::Role(Role::custom("brand-foreground"), 1.0));
theme.style.button_variants.insert(BRAND, r);
// in the UI
Button::new("Save").variant(BRAND).show(ui);
```

The names in this example are the intended API; the plan may adjust
spelling, not shape.

## Performance

`Theme::current` clones the whole theme on every call, and complete recipes
make that clone larger. `examples/bench_frame.rs` builds the gallery screen
in a headless `egui::Context` for a few thousand frames in each of the seven
styles and prints the median time and the allocations per frame, with a
counting allocator in the example and no new dependency. It runs on one
thread. Measurements are taken before any change, after the stylegen step
and after the variant step. If the cost grows noticeably, `Theme` is stored
as `Arc<Theme>` so `Theme::current` copies a pointer, and the measurement is
repeated. The benchmark is a manual tool, not a CI gate.

## Tests

- Snapshots: `snapshot_all` covers vega, nova, maia, lyra, mira, luma and
  rhea, light and dark, for every component.
- Every existing snapshot stays identical through the stylegen and
  component steps. A difference means the hand-written value and upstream
  disagree; it is shown to the maintainer before any re-baseline.
- Issue #6: a custom tabs-list variant plus a changed `tabs_trigger` recipe
  change the active fill, the text colour, the ring and the outline.
- A custom button variant with an app colour, light and dark.
- A missing app colour and a missing app variant each fall back and warn
  once.
- `tests/tokens.rs`: a `Role::…` or a float literal in a component needs
  `// behaviour-ok: <reason>`; `// base:` no longer exempts a value.
- Stylegen: `published.rs` on the whole string; golden tests for the
  outline fields, an `after:` part and a base-only part.

## Order of work

1. Snapshot coverage of all seven styles from the current code, reviewed by
   the maintainer. Its own pull request.
2. Frame benchmark, first measurement.
3. Stylegen: full class strings, outline fields, `after` parts, base-only
   parts, ignore list, `published.rs`. Regenerate; snapshots unchanged.
   Second measurement.
4. Components, one at a time; snapshots unchanged after each. Stricter
   `tokens.rs`; `base_quotes` removed from `drift`.
5. App colours and app variants with their tests. Third measurement.
6. Skills: build-ui shows `theme.style` changes, app colours and app
   variants; port-component states that base classes are generated and that
   a component holds no literal colour or size. CHANGES entry ending with
   `(#6)`.

## Out of scope

- A per-instance or per-region style override.
- Tabs keyboard focus (every trigger is a tab stop; upstream has one tab
  stop and arrow keys). A separate pull request after this work.
- Snapshots of sera (not an exported style).
