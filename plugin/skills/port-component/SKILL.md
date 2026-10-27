---
name: port-component
description: Use when a shadcn component or layout helper the UI needs is missing from the shadcn-egui crate. Ports it to the crate's standard, into the app or into the shadcn-egui repository.
---

# Port a shadcn component

A port is judgement work: reading upstream source, deciding which classes
matter, translating them into egui. The rules below make two ports of the
same component come out the same, whether the port lands in an app or in the
crate.

## Choose the target

- **Upstream** (into the shadcn-egui repository): when working in a
  shadcn-egui checkout, or when the user wants the component in the crate.
  Read `references/upstream.md`.
- **In the app**: otherwise. The port uses only the crate's public API and
  builds the recipes of the new component inside the app. Read
  `references/in-app.md`.

Tell the user which target you chose and why before writing code.

`../build-ui/references/component-map.md` lists what the crate already has.

## The standard

Style values (sizes, colours, radii, shadows, ring widths, alphas) come from
the active style's recipes; `references/styles.md` explains the model. A
recipe holds the part's whole class string: the component's own classes
(base classes) and the style's classes behind its `cn-*` marker. The
drawing code holds no style value of its own.

1. **Source.** Read the component's base `.svelte` files of shadcn-svelte at
   the commit in `tools/stylegen/PIN`, and the bits-ui primitive it wraps.
   Never port from memory, a screenshot, or another style's resolved output.
   - In a shadcn-egui checkout, `PIN` is `tools/stylegen/PIN`. In an app, read
     it from
     `https://raw.githubusercontent.com/oetiker/shadcn-egui/v<version>/tools/stylegen/PIN`,
     with `<version>` the crate version the app depends on.
   - Component source:
     `https://github.com/huntabyte/shadcn-svelte/tree/<PIN>/docs/src/lib/registry/ui/<name>/`
   - Style CSS:
     `https://raw.githubusercontent.com/huntabyte/shadcn-svelte/<PIN>/docs/src/lib/registry/styles/style-<style>.css`
   - The primitive, in the bits-ui version shadcn-svelte locks at `PIN`:
     find `bits-ui@<x.y.z>:` in
     `https://raw.githubusercontent.com/huntabyte/shadcn-svelte/<PIN>/pnpm-lock.yaml`
     (`docs/package.json` holds only a range), then read
     `https://github.com/huntabyte/bits-ui/tree/bits-ui@<x.y.z>/packages/bits-ui/src/lib/bits/<name>/`.
     `https://bits-ui.com/docs/components/<name>` documents the latest
     release, which may differ.
2. **Inventory.** List every `cn-*` marker the source uses and every state it
   styles on (`data-checked`, `aria-invalid`, `data-[icon=…]`,
   `data-orientation`, …). The style CSS also defines `cn-*-aria` markers
   (for example `cn-radio-group-item-aria`); they belong to the React Aria
   variant, the svelte source never uses them, and the port ignores them.
3. **Mapping.** Map each marker to a recipe: a field of `Style` upstream
   (`cn-button-size-sm` is `s.button_size_sm`), an app-local `Recipe` in the
   app. Map each state or context condition to a `State` flag
   (`references/styles.md` has the table). Resolve a property over the
   stacked recipes in the order upstream joins their classes, with `pick`.
   Build the state with `theme.state(ctx, ..)`, which adds `DARK` and the
   window's `SM` and `MD` breakpoint flags, or with
   `interaction_state` (below) for a widget that reacts to the pointer and
   the keyboard.
4. **Base classes go into the recipes.** A class the component source sets
   itself (not through a `cn-*` marker) belongs to the part's recipe, next
   to the style's classes. Upstream, `stylegen` generates both from the
   svelte source: add the part to `tools/stylegen/markers.txt`, as
   `slot:<data-slot>` for a part without a marker
   (`references/upstream.md`). In an app, the hand-written recipe holds the
   base classes too (`references/in-app.md`).
5. **No style values in code.** The component picks every colour, size,
   radius, shadow, alpha and palette role from its recipe stack, or from
   `Metrics` where upstream sets nothing (the thickness of a link
   underline). The only literals left carry
   `// behaviour-ok: <reason>` on the same line: behaviour constants of the
   primitive (a prop default such as `sideOffset`, a keyboard step) and
   inherited fallbacks (CSS's initial `opacity: 1` where no recipe sets
   one). A colour literal or a literal font size needs a
   `// token-ok: <reason>` comment on the line above. A value the
   component leaves to its caller and only a docs demo sets (for example
   the `gap-3` between a radio item and its label in
   `docs/src/lib/registry/examples/radio-group-demo.svelte`) is a parameter
   the caller passes, not a literal in the component.
6. **egui sizing.** The widget sets its own size before placement (single
   pass). Use the `shadcn_egui::layout` helpers, no `available_width()`
   arithmetic: `layout::capped_width(ui, requested, content_min)` caps a
   width to the room and raises it to the content minimum,
   `layout::overlay_width` does the same against the window, and
   `layout::allocate_overflowing` lets a widget wider than its room
   overflow without widening its parent. The widget allocates its rect
   with `layout::allocate_item` (for `allocate_exact_size`) or
   `layout::allocate_item_space` (for `allocate_space`), never egui's own:
   in a `layout::wrap_row` the crate then moves it to the next line and
   gives that line its own height. A content-sized widget (no width or
   `w-fit`) allocates with `layout::allocate_item_space` and senses its
   rect with `ui.interact`: in a justified layout (`ui.add_sized`) it
   then takes the layout's width, as egui's `Button` does. A breakpoint reads the window through
   `layout::window_at_least` (`sm:`, `md:`; `theme.state` sets `SM` and
   `MD`) or its container through `layout::container_at_least`. Resolve
   geometry without interaction states: the size does not change on hover.
7. **Context.** The widget API takes whatever context the recipe conditions
   need: icon side (`ICON_START`, `ICON_END`), size variant (`SIZE_SM`),
   orientation, enclosing slot (`IN_BUTTON_GROUP`).
8. **Behaviour.** Port the primitive's behaviour (below), not only its look.
9. **Interaction states.** Follow the rules below.
10. **Snapshots.** Render the component in dark and light for each style in
    use. Open every PNG the component appears in and compare it with the same
    component on the shadcn-svelte docs page for that style. Without a
    browser, compare the PNG with the values in the style CSS (sizes, radii,
    colours per state), ask the user to compare it with the docs page, and
    say that you did not. Never claim a comparison you did not make.
11. **Dependencies.** No new runtime dependency without a stated reason and
    the user's consent (below).

```rust
use shadcn_egui::components::shared::{
    interaction_state, part_painter, pressable, ActivationKeys, RecipeBox,
};
use shadcn_egui::style::{pick, State};
use shadcn_egui::Theme;

/// A small square button: geometry, press, keys, state and box the way
/// `Button` does them.
fn small_button(ui: &mut egui::Ui) -> egui::Response {
    let theme = Theme::current(ui.ctx());
    let s = &theme.style;
    let layers = [&*s.button, &*s.button_size_sm];
    // Geometry never depends on interaction.
    let geometry = theme.state(ui.ctx(), State::NONE);
    let side = pick(&layers, |r| &r.height, geometry).unwrap_or(0.0);
    let (rect, mut resp) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
    pressable(&mut resp, ActivationKeys::Button);
    let st = interaction_state(&theme, &resp, State::NONE);
    RecipeBox::resolve(ui.ctx(), &theme, &layers, st).paint(&part_painter(ui, &[&layers[..]], st), rect);
    resp
}
```

`RecipeBox` puts border and outline widths on whole device pixels, as
Blink does. Where a border width becomes geometry in the port (padding
plus border, a size), take it through
`shared::line_width(px, ui.ctx().pixels_per_point())`, and inset content
with `shared::inset_frame` (or `RecipeBox::frame`), which keeps a fraction
of a point that egui's `Margin` would round away.

## App variants

A port into the crate gives every new component `XVariant`, `XRecipes` and
`.variant(impl Into<XVariant>)`, so an app can define its own looks of it
(`build-ui` SKILL.md, "App variants"). The mechanism is sealed: a port in an
app has none. `components/separator.rs` is the smallest example,
`components/card.rs` the one with a size and free-function parts.

- No upstream variant: `default_variant!(XVariant, XRecipes { .. })` writes
  the `Default` case, the hidden app case, `XVariant::app` and the recipes
  struct. An enum with built-in variants (`ButtonVariant`, `TabsVariant`)
  carries the app case by hand, as `components/button.rs` does.
- `XRecipes` has one `Recipe` field per part the component draws, named
  after its `data-slot` in snake case (`card_title`); a part without a
  `data-slot` after its `cn-*` recipe, a pseudo-element after its slot plus
  the element (`tabs_trigger_after`).
- The widget resolves its variant with `t.resolve(self.variant)`, takes each
  part from the result (`v.part(..)`) and starts its state from `v.bits()`.
- A size upstream applies through state bits (`data-size`, read as
  `data-[size=sm]:` in the parts' own recipes) gets
  `state_size!(XSize, XVariant)`, which folds the base size's state rules
  so an app size's `set` replaces them; the widget resolves with
  `t.resolve_pair(variant, size)`. A size with recipes of its own
  (`cn-button-size-*`) is a layer, as `ButtonSize`.
- Parts that are free functions (`card_title`, `dialog::footer`) read the
  parent's variant from the context tag (`components/context.rs`): the
  parent pushes an `XContext` on the `Ui` it hands its content
  (`context::push`), the part reads it with `context::find`. Never a new
  `ctx.data` channel.
- A unit test `edit_compiles_and_changes_its_part` defines one app variant
  with `Theme::variant`, resolves it and checks that the edited part
  changed and the rest kept the base's values.

## Behaviour, not just looks

A shadcn component is a styled bits-ui primitive, and the primitive carries
the behaviour. Read its source, write down what it does, and check each item
in egui: with a test, or with a stated reason why it does not apply. Cover
keyboard handling, focus order, pointer capture while dragging, value
stepping and clamping, disabled handling, degenerate input, and the
accessibility role through `widget_info`.

Example, the list for Slider:

1. A click on the track jumps the value to that point, then snaps.
2. Dragging the thumb keeps the pointer captured, also outside the widget.
3. Arrow keys change the value by one step (continuous: 1% of the range);
   PageUp and PageDown by 10 steps; Home and End go to min and max.
4. The value is clamped to the range and snapped to the step.
5. Disabled: no pointer or keyboard change and no hover or focus styles.
6. `widget_info` reports a slider with its value and label.
7. `min == max`, `step <= 0`, a NaN value, a value outside the range: the
   widget draws, does not panic and never produces NaN.

Group widgets (RadioGroup, ToggleGroup, Tabs) use roving focus: the group
is one Tab stop (the selected item, else the first), and the arrow keys move
the focus and, in RadioGroup, the selection to the next or previous item,
wrapping at the ends. bits-ui RadioGroup takes both axes (Left and Up go
back, Right and Down forward) whatever its orientation.

## Interaction states

- One shared helper,
  `shadcn_egui::components::shared::interaction_state(&theme, &resp, context)`,
  turns a `Response` into a `State`: the context flags, `HOVER`, `ACTIVE`,
  `FOCUS` while focused, `FOCUS_VISIBLE` where the browser's
  `:focus-visible` would match (focus moved by keyboard, tracked
  crate-wide), and `DARK`. A widget that takes typed text uses
  `text_entry_state`, which shows the ring after a pointer focus too. Do
  not re-derive either per component, or components will disagree.
- Call `shared::pressable(&mut resp, keys)` right after the interaction of
  anything clickable: a pointer press then focuses it without a ring, as
  the browser does, and `clicked()` fires on the element's own keys
  (`ActivationKeys::Button` for a `<button>`: Enter on press, Space on
  release; `EnterAndSpace`, `Space` or `Enter` for a bits primitive or a
  link; `None` for a slider thumb).
- Disabled state comes from the `Ui`; a widget with its own `enabled` flag
  adds `DISABLED` to the context. A disabled widget never gets `HOVER`,
  `ACTIVE`, `FOCUS` or `FOCUS_VISIBLE`. It gets `DISABLED` only.
- A part fades by its recipe's `opacity` (`disabled:opacity-50`):
  `shared::opacity(&[part_stack, parent_stack], state)` multiplies the
  values down the part tree as CSS does. `shared::part_painter(ui, chain,
  state)` returns the painter at the part's opacity: the `Ui`'s opacity
  with egui's disabled fade taken out once, times the part's own value, with
  `DISABLED` added in a disabled `Ui` (`shared::part_opacity` is the bare
  value). So a part fades by its style's `disabled:` rule, not by egui's
  fade. A crate part drawn through egui (a title through `TextLook::label`) runs
  in `shared::part_ui(ui, chain, state, |ui| ..)`; a container hands the
  caller's content `shared::content_ui(ui, chain, state, |ui| ..)`, which
  keeps egui's fade for plain egui widgets. A box an egui container paints
  outside that `Ui` (a popup frame) takes `RecipeBox::faded(opacity)`.
- An empty size `Prop` is CSS's initial value: `width` and `height` fall
  back to the content size (`TextLook::line_height` for a line of text), never
  to 0, so an app that replaces a recipe whole keeps a usable widget.
- Geometry is resolved without interaction states.
- A value that changes where upstream transitions it (`transition-all` on
  the progress indicator) is painted at
  `shadcn_egui::components::motion::transition(ctx, id, target, &stack,
  Transition::OTHER, state)`: it moves over the recipe's `duration` along
  its `easing` (Tailwind's 150 ms and `ease-in-out` curve where the part
  sets none) when the picked `transition` names the property, jumps
  otherwise and under reduced motion (`style.animation_time == 0`), and
  asks for a repaint only while it moves. Name the property by what it is:
  `Transition::COLORS` for a colour, `TRANSFORM` for a translate,
  `OPACITY`, `OTHER` for a size.
- Every custom-painted widget reports itself with `resp.widget_info(..)`;
  a role egui's `WidgetType` lacks (`alert`, `alertdialog`, `navigation`,
  `group`) goes through `shared::widget_info_with_role(&resp, role, ..)`.

## Dependency discipline

Everything in `[dependencies]` propagates to every consumer. Keep that
surface minimal.

- Default: add no new runtime dependency. A ported component should be
  expressible with `egui`, the crate and custom painting, like every existing
  one.
- A new `[dependencies]` entry needs a strong, stated reason: a capability
  that cannot be painted or computed in the crate, worth the build cost and
  the version coupling imposed on all consumers. Prefer a small focused crate
  over a large one, and an optional, feature-gated dependency over an
  always-on one. Heavy crates (`eframe`, `image` or `resvg` loaders, `wgpu`)
  never become runtime dependencies.
- Tooling stays in `[dev-dependencies]` (`eframe` for examples,
  `egui_kittest` and `wgpu` for snapshots).
- When in doubt, ask the user before adding a dependency, and write the
  reason next to it in `Cargo.toml`.

## References

- `references/styles.md`: the style model, recipe fields, `State` flags,
  the `pick` rule and the generator.
- `references/in-app.md`: porting into an app.
- `references/upstream.md`: porting into the shadcn-egui repository.
- `../build-ui/references/component-map.md`: what the crate already has.
