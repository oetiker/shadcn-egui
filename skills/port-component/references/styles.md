# shadcn styles

shadcn-egui ports shadcn-svelte's *styles* (vega, nova, …) as data. A component
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
  A rule derived from a CSS variable (for example `--card-spacing`) takes the rank of
  the variable's definition, an approximation.
- **Deviating in a project.** Adjust the style when you build the theme, never
  in copies of crate code:
  ```rust
  use shadcn_egui::{Palette, Style, Theme};

  fn tall_buttons() -> Theme {
      let mut s = Style::vega();
      s.button_size_default.height.base = Some(40.0);
      Theme::new(s, Palette::dark(), 10.0)
  }
  ```

## Recipe fields and conditions

`Recipe` fields (`crates/shadcn-egui/src/style/recipe.rs`) and the Tailwind utilities that set them:

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
| `bg_clip_padding` | `bg-clip-padding`: the fill stops inside the border (`shared::paint_box`) |
| `border_width`, `border_top_width` | `border`, `border-t`, `border-N` |
| `border_color` | `border-*` colour |
| `ring_width`, `ring_color` | `ring-N`, `ring-*` |
| `shadow` | `shadow-*` |
| `opacity` | `opacity-*`, `disabled:opacity-50` |
| `shift_x`, `shift_y` | `translate-x-*`, `translate-y-*` (`active:translate-y-px` is the press nudge) |
| `hit_inset_x`, `hit_inset_y` | `after:-inset-x-*`, `after:-inset-y-*` (a larger hit area) |
| `backdrop_blur` | `backdrop-blur-*` |

`disabled:opacity-50` fades each shape of the part (fill, text, border,
ring, shadow) to half alpha, the way egui fades a disabled `Ui`, because egui
cannot composite a group of shapes at one opacity. Text on a solid fill
therefore shows a little less contrast than in the browser, and where one
part lies on another (a switch or slider thumb on its track) the lower part
shows through. The fade comes from `shared::disabled_painter` and applies
once: a widget inside a disabled `Ui` is not faded again by its own flag.

`State` flags (`crates/shadcn-egui/src/style/recipe.rs`) and the variants they stand for.
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

Text sizes and line heights are not part of a style. `Metrics` holds
`text_xs/sm/base/lg` and the line heights Tailwind attaches to them
(`line_xs/line_sm/line_base/line_lg`); `Theme::line_px(TextSize)` returns the
line height for a recipe's `text_size`.

## Breakpoints

`sm:` and `md:` count as **matched**: an egui desktop window is wider than both.
Larger breakpoints (`lg:` and up) are unsupported, so a marker that uses one
cannot be exported.

## The generator

`tools/stylegen` reads the upstream style CSS and writes Rust. It is its own
workspace; run it from the repository root with a shadcn-svelte checkout at
the commit in `tools/stylegen/PIN` and a shadcn-ui checkout at the commit in
`tools/stylegen/PIN_REACT`:

```bash
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- generate <shadcn-svelte> <shadcn-ui> .
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift    <shadcn-svelte> <shadcn-ui> .
```

- `generate` writes `crates/shadcn-egui/src/style/generated.rs` (the `Style` struct, one field per
  marker in `tools/stylegen/markers.txt`), a
  `crates/shadcn-egui/src/style/<name>.rs` for each style
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

## Exported styles

Exported (constructors on `Style`): vega, nova, maia, lyra, mira, luma, rhea.
The snapshot tests render vega and nova only; the other five have a unit test
that they construct (on a 256 KB stack) and resolve their button height, and
have not been checked visually, so they may differ from the web in detail.

Not exported: sera. Its report lists unsupported classes in shipped markers:
single-side bottom borders on inputs (`border-b-*`, underline-style fields),
`peer-data-[slot=…]` variants on the label, and a `translate-x-[calc(…)]`
value on the switch thumb. Support for them has to be added to the generator
first (see `tools/stylegen/reports/sera.md`).

## Choosing a style

Read `style` in the web project's `components.json`. An app sets it through a
preset (the `build-ui` skill). A theme built directly takes the style and the
web project's `--radius` in px:

```rust
use shadcn_egui::{Palette, Style, Theme};

fn setup(ctx: &egui::Context) {
    Theme::new(Style::nova(), Palette::dark(), 10.0).apply(ctx);
}
```

`Style::vega()` is the default. Take the radius in px from `--radius` in the web
project's CSS (`0.625rem` → `10.0`). Apply the theme once; apply again only when
it changes.

## Bumping the pin

1. Put the new shadcn-svelte commit in `tools/stylegen/PIN`.
2. Check out upstream at that commit.
3. Run `stylegen generate`, then `stylegen drift`.
4. Re-port every component that `drift` lists (a quote no longer upstream, a
   marker that appeared or disappeared), following the standard in
   `SKILL.md` and `references/upstream.md`.
5. Re-baseline the snapshots (`UPDATE_SNAPSHOTS=1 cargo test`) and look at the
   PNGs that changed.
6. Commit the generated files, the reports and the PNGs together.
