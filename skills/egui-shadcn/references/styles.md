# shadcn styles

egui_shadcn ports shadcn-svelte's *styles* (vega, nova, …) as data. A component
draws itself from the active style's recipes; it does not hard-code a look.

## The style model

- **Theme = Style × Palette × radius** (plus `Metrics`, the text-size scale).
  `Theme { style: Arc<Style>, palette, radius, metrics, dark }`. `Theme` is
  `Clone` (the style sits behind an `Arc`), not `Copy`.
- **A style is a class map.** upstream `style-<name>.css` binds Tailwind classes
  to marker classes (`cn-button`, `cn-button-size-sm`, …). `Style` has one
  `Recipe` field per marker (`s.button`, `s.button_size_sm`). A recipe holds one
  `Prop` per property (`height`, `pad_left`, `bg`, `ring_width`, …).
- **A `Prop` is a base value plus rules.** `base: Option<T>` is the
  unconditional value; each `Rule { when, rank, value }` applies when its
  `When` matches the `State` and carries the Tailwind variant order as `rank`.
  An empty `Prop` means the style does not set the property.
- **Markers.** At runtime shadcn injects the marker's classes into the element's
  class list. Five markers have no CSS in any style, so their recipes are empty
  by design: `cn-card-action`, `cn-select-viewport`,
  `cn-select-item-indicator-icon`, `cn-tabs-list-variant-default`,
  `cn-tabs-list-variant-line`.
- **Base vs style classes.** A component's own classes (those written in its
  `.svelte` file, not behind a `cn-*` marker) are *base classes*. The port
  quotes them in `// base: "…"` comments. Style classes live in the recipes.
- **tailwind-merge order.** Upstream joins the classes of stacked markers
  (`cn-button`, then `cn-button-variant-*`, then `cn-button-size-*`) and a later
  class replaces an earlier one that sets the same property. `pick` reproduces it.
- **The `pick` rule.**
  `pick(&[&layer1, &layer2, …], |r| &r.field, state)` collects, from every layer,
  the base value (rank 0) and every rule that matches `state`. The candidate
  with the highest `(rank, layer, rule index)` wins; so a conditional rule in an
  early layer beats a later base, and between equal ranks the later layer wins.
  A rule's rank is the OR of one bit per Tailwind variant it uses, in the order
  of Tailwind v4.3.3 `Variants.compare` (arbitrary variants after named ones);
  an `!important` class has bit 127. This matches the CSS order shadcn gets.
  A rule derived from a CSS variable (e.g. `--card-spacing`) takes the rank of
  the variable's definition, an approximation.
- **Deviating in a project.** Adjust the style when you build the theme, never
  in vendored files:
  ```rust
  let mut s = Style::vega();
  s.button_size_default.height.base = Some(40.0);
  let theme = Theme::new(s, Palette::dark(), 10.0);
  ```

## Recipe fields and conditions

`Recipe` fields (`src/style/recipe.rs`) and the Tailwind utilities that set them:

| Field | Tailwind utilities |
|---|---|
| `height`, `width` | `h-*`, `size-*`, `w-*` |
| `min_height`, `min_width`, `max_width` | `min-h-*`, `min-w-*`, `max-w-*` |
| `max_width_inset` | `max-w-[calc(100%-Npx)]` |
| `pad_top/right/bottom/left` | `p-*`, `px-*`, `py-*`, `pt-*`, `pr-*`, `pb-*`, `pl-*` |
| `margin_*` | `m-*`, `mx-*`, `my-*`, `-mt-*`, … |
| `gap` | `gap-*`, `gap-x-*`, `gap-y-*` |
| `inset_top/right/bottom/left` | `top-*`, `right-*`, `bottom-*`, `left-*` (absolutely placed parts) |
| `icon_size` | `[&_svg:not([class*='size-'])]:size-*`, `[&>svg]:size-*` |
| `text_size` | `text-xs` … `text-lg`, `text-[Npx]` |
| `line_height` | `leading-*` |
| `tracking` | `tracking-*` |
| `weight` | `font-normal`, `font-medium`, `font-semibold`, `font-bold` |
| `uppercase`, `underline` | `uppercase`, `underline` |
| `radius`, `radius_top`, `radius_bottom` | `rounded-*`, `rounded-t-*`, `rounded-b-*` |
| `bg`, `text` | `bg-*`, `text-*` (with `/alpha` and `color-mix`) |
| `border_width`, `border_top_width` | `border`, `border-t`, `border-N` |
| `border_color` | `border-*` colour |
| `ring_width`, `ring_color` | `ring-N`, `ring-*` |
| `shadow` | `shadow-*` |
| `opacity` | `opacity-*`, `disabled:opacity-50` |
| `shift_x`, `shift_y` | `translate-x-*`, `translate-y-*` (`active:translate-y-px` is the press nudge) |
| `hit_inset_x`, `hit_inset_y` | `after:-inset-x-*`, `after:-inset-y-*` (a larger hit area) |
| `backdrop_blur` | `backdrop-blur-*` |

`State` flags (`src/style/recipe.rs`) and the variants they stand for.
`Theme::state(s)` adds `DARK`.

| Flag | Variants |
|---|---|
| `DARK` | `dark:` |
| `HOVER` | `hover:` |
| `FOCUS` | `focus:` (the highlighted item of a list) |
| `FOCUS_VISIBLE` | `focus-visible:` (keyboard focus) |
| `ACTIVE` | `active:` (pressed) |
| `DISABLED` | `disabled:`, `data-disabled:`, `group-data-[disabled=true]:`, `peer-disabled:` |
| `INVALID` | `aria-invalid:`, `data-invalid:` |
| `CHECKED` | `data-checked:`, `aria-checked:`, `data-[state=checked]:` |
| `UNCHECKED` | `data-unchecked:`, `data-[state=unchecked]:` |
| `EXPANDED` | `aria-expanded:` |
| `HAS_POPUP` | `aria-[haspopup]:` (used negated: `not-aria-[haspopup]:`) |
| `SELECTED` | `data-active:`, `data-[active=true]:`, `data-selected:` |
| `PLACEHOLDER` | `data-placeholder:` |
| `ICON_START` / `ICON_END` | `has-data-[icon=inline-start]:` / `has-data-[icon=inline-end]:` |
| `IN_BUTTON_GROUP` | `in-data-[slot=button-group]:` |
| `SIZE_SM` / `SIZE_DEFAULT` | `data-[size=sm]:` / `data-[size=default]:`, `group-data-[size=…]/…:` |
| `VERTICAL` / `HORIZONTAL` | `data-vertical:`, `data-[orientation=vertical]:`, `group-data-vertical/…:` and the horizontal forms |
| `VARIANT_LINE` / `VARIANT_DEFAULT` | `data-[variant=line]:` / `group-data-[variant=default]/…:` |
| `HAS_FOOTER` | `has-data-[slot=card-footer]:` |

A rule's `When` is `all` (flags that must be set) plus `none` (flags that must
not be set; this is how a negated variant works). A component sets only the
flags that are true for its current paint.

## Breakpoints

`sm:` and `md:` count as **matched**: an egui desktop window is wider than both.
Larger breakpoints (`lg:` and up) are unsupported, so a marker that uses one
cannot be exported.

## The generator

`tools/stylegen` reads the upstream style CSS and writes Rust. It is its own
workspace; run it from the crate root with a shadcn-svelte checkout at the
commit in `tools/stylegen/PIN`:

```bash
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- generate <shadcn-svelte> .
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift    <shadcn-svelte> .
```

- `generate` writes `src/style/generated.rs` (the `Style` struct, one field per
  marker in `tools/stylegen/markers.txt`), a `src/style/<name>.rs` for each style
  in `tools/stylegen/EXPORT`, and `tools/stylegen/reports/<style>.md` for **all**
  eight upstream styles. Generated files carry a "do not edit" header. Commit
  them with the reports.
- `drift` checks the crate against upstream: every `// base: "…"` quote must
  still be a substring of the component source, and the `cn-*` markers in a
  component's upstream source must equal the markers in `markers.txt`. It prints
  `drift: clean` when nothing differs.
- **Reports** list, per marker, every class the generator did not turn into a
  rule. *Ignored* classes carry a reason (animation, link-styled anchors,
  `bg-clip-padding`, …) and are fine. *Unsupported* classes are ones the generator
  cannot translate; it never guesses.
- **EXPORT** names the styles that ship as Rust. `generate` refuses to export a
  style whose report has an unsupported class in any shipped marker; the gate
  counts unsupported classes only. The other styles are generated as reports but
  not exported.
- A few values look odd and are right. vega's `card_content` gap of 12 comes from
  upstream's `.cn-card-content:has(> [data-slot=questionnaire-choices])` rule,
  which upstream's own CSS parser flattens; it matches the published output.

## Choosing a style

Read `style` in the web project's `components.json`. Build the theme with it and
with the web project's `--radius`:

```rust
Theme::new(Style::nova(), Palette::dark(), 10.0).apply(&ctx);
```

`Style::vega()` is the default. Take the radius in px from `--radius` in the web
project's CSS (`0.625rem` → `10.0`). Apply the theme once; apply again only when
it changes.

## Bumping the pin

1. Put the new shadcn-svelte commit in `tools/stylegen/PIN`.
2. Check out upstream at that commit.
3. Run `stylegen generate`, then `stylegen drift`.
4. Re-port every component that `drift` lists (a quote no longer upstream, a
   marker that appeared or disappeared), following Part B of `SKILL.md`.
5. Re-baseline the snapshots (`UPDATE_SNAPSHOTS=1 cargo test`) and look at the
   PNGs that changed.
6. Commit the generated files, the reports and the PNGs together.
