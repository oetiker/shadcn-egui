# shadcn styles

shadcn-egui ports shadcn-svelte's *styles* (vega, nova, …) as data. A component
draws itself from the active style's recipes; it does not hard-code a look.

## The style model

- **Theme = Style × Palette × radius** (plus `Metrics`, the text-size scale).
  `Theme { style: Arc<Style>, palette, radius, metrics, dark }`. `Theme` is
  `Clone` (the style sits behind an `Arc`), not `Copy`.
- **A style is a class map.** upstream `style-<name>.css` binds Tailwind classes
  to marker classes (`cn-button`, `cn-button-size-sm`, …). `Style` has one
  `Recipe` field per part (`s.button`, `s.button_size_sm`): a `cn-*` marker,
  a `data-slot` without a marker (`s.separator`), or a part's `after:`
  pseudo-element (`s.tabs_trigger_after`, the line tab underline). A recipe holds one
  `Prop` per property (`height`, `pad_left`, `bg`, `ring_width`, …). The
  fields are `Box<Recipe>`, which keeps `Style` small on the stack: field
  access reaches through the box, a slice of recipes in a `let` takes
  `[&*s.button, &*s.button_size_sm]`, and a whole replacement writes
  `*s.button = Recipe { .. }`.
- **A `Prop` is a base value plus rules.** `base: Option<T>` is the
  unconditional value; each `Rule { when, rank, value }` applies when its
  `When` matches the `State` and carries the Tailwind variant order as `rank`.
  An empty `Prop` means upstream sets nothing: neither the component's own
  classes nor the style's. The component then uses the inherited or
  initial value. `Rule::important(when, value)` builds an app's override
  with the `!important` rank; `When::all(flags)` matches whenever every
  flag is set.
- **Markers.** At runtime shadcn injects the marker's classes into the element's
  class list. Some markers have no class the crate translates, so their
  recipes are empty by design: `cn-card-action`, `cn-select-viewport` and
  `cn-select-item-indicator-icon` in every style, and `cn-select-group` in
  lyra.
- **Base and style classes.** A component's own classes (those written in its
  `.svelte` file, not behind a `cn-*` marker) are *base classes*. The
  generated recipe holds both: the part's whole class string is the
  component's string with the style's `@apply` classes in place of the
  marker, so a base class that comes later in the string wins under the
  same condition, as in the published component.
- **tailwind-merge order.** Upstream joins the classes of stacked markers
  (`cn-button`, then `cn-button-variant-*`, then `cn-button-size-*`) and a later
  class replaces an earlier one that sets the same property. `pick` reproduces it.
- **The `pick` rule.**
  `pick(&[&layer1, &layer2, …], |r| &r.field, state)` collects, from every layer,
  the base value (rank 0) and every rule that matches `state`. The candidate
  with the highest `(rank, layer, rule index)` wins; so a conditional rule in an
  early layer beats a later base, and between equal ranks the later layer wins.
  A rule's rank orders it as the browser's cascade does. An `!important` class
  has bit 127 (`Rule::IMPORTANT`), and so has an app's `Rule::important`, which
  wins a tie by coming later. Bit 126 marks a `placeholder:` class: the
  placeholder is an element of its own, and its rules beat what it inherits
  from the input. Then the specificity the variants add to the class: the
  count of classes, attributes and pseudo-classes, `1 << 122` each (`dark:`,
  `hover:`, `data-[size=sm]:`; `sm:`, `md:`, `supports-*:`, `in-*:`, `after:`
  and `[&>svg]:` add none, and so do shadcn-svelte's own `data-open:`,
  `data-closed:`, `data-checked:`, `data-unchecked:`, `data-disabled:`,
  `data-active:`, `data-horizontal:` and `data-vertical:`, which are
  `:where()`; an element selector such as `has-[>svg]:` adds a half step), and bit 121 for `[&_svg:not([class*='size-'])]:`,
  a half step: one class more specific than `[&>svg]:`, but Tailwind sorts it
  first, so at equal specificity the `[&>svg]:` rule wins. The lowest bits
  hold one bit per Tailwind variant the class uses (targets have none), in
  the order of Tailwind v4.3.3 `Variants.compare` (arbitrary variants after
  named ones), which decides between rules of equal rank above. So
  `data-[size=sm]:h-7` beats `md:h-9`, as in the browser. A value that always
  applies but has a rank above 0, such as an `icon_size` from
  `[&_svg:not([class*='size-'])]:size-4`, is a `When::ALWAYS` rule, not a
  base.
  A rule derived from a CSS variable (for example `--card-spacing`) is ranked as
  one class with the variants of both the using and the defining class, an
  approximation.
- **Deviating in a project.** Adjust the style when you build the theme, never
  in copies of crate code. Setting `base` changes the unconditional value only
  where no `When::ALWAYS` rule outranks it; the `icon_size` of buttons, select
  triggers, select items, select scroll buttons and tab triggers sits in such
  a rule, so replace the whole property with `Prop::fixed(..)`:
  ```rust
  use shadcn_egui::style::{pick, Prop, State};
  use shadcn_egui::{Palette, Style, Theme};

  fn tall_buttons() -> Theme {
      let mut s = Style::vega();
      s.button_size_default.height.base = Some(40.0);
      s.button.icon_size = Prop::fixed(20.0);
      Theme::new(s, Palette::light(), Palette::dark(), 10.0)
  }

  let theme = tall_buttons();
  let s = &theme.style;
  let stack = [&*s.button, &*s.button_variant_default, &*s.button_size_default];
  assert_eq!(pick(&stack, |r| &r.height, State::NONE), Some(40.0));
  assert_eq!(pick(&stack, |r| &r.icon_size, State::NONE), Some(20.0));
  ```

## Recipe fields and conditions

`Recipe` fields (`crates/shadcn-egui/src/style/recipe.rs`) and the Tailwind utilities that set them:

| Field | Tailwind utilities |
|---|---|
| `height`, `width` | `h-*`, `size-*`, `w-*` |
| `min_height`, `min_width`, `max_width` | `min-h-*`, `min-w-*`, `max-w-*` |
| `max_width_inset` | `max-w-[calc(100%-Npx)]` |
| `height_inset` | `h-[calc(100%-Npx)]` |
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
| `underline_offset` | `underline-offset-*` (px); the text path draws the underline as Chrome does, its thickness a tenth of the font size |
| `radius`, `radius_top`, `radius_right`, `radius_bottom`, `radius_left` | `rounded-*`, `rounded-t-*`, `rounded-r-*`, `rounded-b-*`, `rounded-l-*` |
| `bg`, `text` | `bg-*`, `text-*` (with `/alpha` and `color-mix`) |
| `bg_clip_padding` | `bg-clip-padding`: the fill stops inside the border (`shared::RecipeBox`) |
| `border_width`, `border_top_width`, `border_left_width` | `border`, `border-t`, `border-l`, `border-N` |
| `border_color` | `border-*` colour |
| `ring_width`, `ring_color` | `ring-N`, `ring-*` |
| `outline_width`, `outline_color` | `outline-N`, `outline-*`; `outline-none` and `outline-hidden` are 0 |
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
shows through. The value comes from the recipe's `opacity` prop
(`shared::opacity` multiplies it down the part tree, so the line underline
of a disabled tab fades with the tab), and `shared::part_painter` applies
it once: inside a disabled `Ui`, which egui fades already, the part paints
at the `Ui`'s opacity with egui's fade taken out, times its own value under
`DISABLED`, so a part fades by its style's `disabled:` rule, not by egui's
fade.

`State` flags (`crates/shadcn-egui/src/style/recipe.rs`) and the variants they stand for.
`Theme::state(ctx, s)` adds `DARK` and the window's `SM` and `MD`. A flag
means the upstream attribute it stands for wherever it sits: `SIZE_SM` is
`data-size="sm"` on the part or, through `group-data-[size=sm]/…:`, on its
group.

| Flag | Variants |
|---|---|
| `DARK` | `dark:` |
| `HOVER` | `hover:` |
| `FOCUS` | `focus:`, `data-highlighted:` (the highlighted item of a list) |
| `FOCUS_VISIBLE` | `focus-visible:` (keyboard focus) |
| `ACTIVE` | `active:` (pressed) |
| `DISABLED` | `disabled:`, `data-disabled:`, `group-data-[disabled=true]:`, `peer-disabled:` |
| `INVALID` | `aria-invalid:`, `data-invalid:` |
| `CHECKED` | `data-checked:`, `aria-checked:`, `data-[state=checked]:` |
| `UNCHECKED` | `data-unchecked:`, `data-[state=unchecked]:` |
| `EXPANDED` | `aria-expanded:`, `data-open:` |
| `CLOSED` | `data-closed:` (a closed panel) |
| `DELAYED_OPEN` | `data-[state=delayed-open]:` (a tooltip opened after its delay) |
| `HAS_POPUP` | `aria-[haspopup]:` (used negated: `not-aria-[haspopup]:`) |
| `SELECTED` | `data-active:`, `data-[active=true]:`, `data-selected:` |
| `PLACEHOLDER` | `data-placeholder:`, `placeholder:` (placeholder text shows) |
| `ICON_START` / `ICON_END` | `has-data-[icon=inline-start]:` / `has-data-[icon=inline-end]:` |
| `HAS_ICON` | `has-[>svg]:`, `group-has-[>svg]/…:` (the part shows an icon) |
| `SIZE_SM` / `SIZE_DEFAULT` / `SIZE_LG` | `data-[size=sm]:` / `data-[size=default]:` / `data-[size=lg]:`, and `group-data-[size=…]/…:` |
| `HAS_SIZE_SM` / `HAS_SIZE_LG` | `group-has-data-[size=sm]/…:` / `group-has-data-[size=lg]/…:` (the group holds a small or large child) |
| `VERTICAL` / `HORIZONTAL` | the part's own orientation: `data-vertical:`, `data-[orientation=vertical]:`, `group-data-vertical/…:` and the horizontal forms |
| `VARIANT_LINE` / `VARIANT_DEFAULT` | `data-[variant=line]:` / `group-data-[variant=default]/…:` |
| `VARIANT_DESTRUCTIVE` | a part of the destructive variant (`cn-alert-variant-destructive`'s `*:data-[slot=alert-description]:`) |
| `SIDE_TOP` / `SIDE_RIGHT` / `SIDE_BOTTOM` / `SIDE_LEFT` | `data-[side=top]:` and the other sides (where a panel sits against its trigger) |
| `SM` / `MD` | `sm:` / `md:` (the window is at least 640 / 768 points wide) |
| `HAS_FOOTER` | `has-data-[slot=card-footer]:` |
| `HAS_ACTION` | `has-data-[slot=alert-action]:` |
| `HAS_MEDIA` | `has-data-[slot=alert-dialog-media]:`, `group-has-data-[slot=alert-dialog-media]/…:` |
| `HAS_KBD` | `has-data-[slot=kbd]:` |
| `HAS_BUTTON_GROUP` | `has-[>[data-slot=button-group]]:` (a nested button group) |
| `HAS_OUTLINE` | `has-[>[data-variant=outline]]:` (a child with the outline variant) |
| `IN_BUTTON_GROUP` | `in-data-[slot=button-group]:`, and every child rule of a button group |
| `IN_GROUP_HORIZONTAL` / `IN_GROUP_VERTICAL` | a child of a group whose `data-orientation` is horizontal / vertical (the group's `[&>*]` rules), apart from the child's own orientation |
| `FIRST` / `LAST` / `NOT_FIRST` / `NOT_LAST` | a group child's position: `:first-child`, `:last-child` (the last `[data-slot]` child), `:not(:first-child)`, `:not(:last-child)` |
| `IN_TOOLTIP_CONTENT` | `in-data-[slot=tooltip-content]:`, and a tooltip's `**:data-[slot=kbd]:` |
| `IN_AVATAR_GROUP` | an avatar in an avatar group (the group's `*:data-[slot=avatar]:`) |

A rule's `When` is `all` (flags that must be set) plus `none` (flags that must
not be set; this is how a negated variant works). A component sets only the
flags that are true for its current paint.

Text sizes and line heights are not part of a style. `Metrics` holds
`text_xs/sm/base/lg` and the line heights Tailwind attaches to them
(`line_xs/line_sm/line_base/line_lg`); `Theme::line_px(TextSize)` returns the
line height for a recipe's `text_size`.

## Breakpoints

`sm:` and `md:` are media queries on the window width, as in the browser:
`Theme::state(ctx, s)` sets `SM` when the window is at least 640 points wide
and `MD` at 768, so a component that builds its state there follows the
window by itself. A layout or a screen reads the same test through
`layout::window_at_least(ctx, px)`, and a container query (`@md/…`) through
`layout::container_at_least(ui, px)`; never with width arithmetic of its own.
Larger breakpoints (`lg:` and up) are unsupported, so a marker that uses one
cannot be exported.

## The generator

`tools/stylegen` reads the upstream component sources and style CSS and
writes Rust. It is its own
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
  eight upstream styles. It also copies the component sources and the style
  CSS it read to `tools/stylegen/tests/fixtures/`. Generated files carry a
  "do not edit" header. Commit them with the reports.
- `drift` checks the crate against upstream: the `cn-*` markers in a
  component's upstream source must equal the markers in `markers.txt`, and
  the copies in `tools/stylegen/tests/fixtures/` must equal the checkout. It
  prints `drift: clean` when nothing differs.
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
Every exported style has snapshot tests in light and dark mode
(`crates/shadcn-egui/tests/snapshots/*_<style>_{light,dark}.png`), and a unit
test that it constructs on a 256 KB stack and resolves its button height.

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
    Theme::new(Style::nova(), Palette::light(), Palette::dark(), 10.0).apply(ctx);
}
```

`Style::vega()` is the default. Take the radius in px from `--radius` in the web
project's CSS (`0.625rem` → `10.0`). Apply the theme once; apply again only when
it changes.

## Bumping the pin

1. Put the new shadcn-svelte commit in `tools/stylegen/PIN`.
2. Check out upstream at that commit.
3. Run `stylegen generate`, then `stylegen drift`. `generate` stops when an
   `extract::BRANCHES` entry or a `resolve::CORRECTIONS` edit no longer
   matches the new source; update the table first (`references/upstream.md`,
   Markers).
4. Re-port every component that `drift` lists (a marker that appeared or
   disappeared), following the standard in
   `SKILL.md` and `references/upstream.md`.
5. Re-baseline the snapshots: delete `crates/shadcn-egui/tests/snapshots/*.png`,
   run `UPDATE_SNAPSHOTS=1 cargo test`, and look at the PNGs that changed.
   kittest ignores a change on anti-aliased pixels, so a baseline that is not
   deleted can keep stale pixels; the strict check then fails the next test
   run (`references/upstream.md`, Snapshots).
6. Commit the generated files, the reports and the PNGs together.
