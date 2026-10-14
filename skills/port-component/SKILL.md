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
   Build the state with `theme.state(..)`, which adds `DARK`, or with
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
   arithmetic. Resolve geometry without interaction states: the size does not
   change on hover.
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
use shadcn_egui::components::shared::interaction_state;
use shadcn_egui::style::{pick, State};
use shadcn_egui::Theme;

/// The height of a small button, resolved the way `Button` does it.
fn button_height(ui: &egui::Ui, resp: &egui::Response) -> f32 {
    let theme = Theme::current(ui.ctx());
    let s = &theme.style;
    let st = interaction_state(&theme, resp, true, State::default());
    pick(&[&*s.button, &*s.button_size_sm], |r| &r.height, st).unwrap_or(0.0)
}
```

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
  `shadcn_egui::components::shared::interaction_state(&theme, &resp, enabled, context)`,
  turns a `Response` into a `State`: the context flags, `HOVER`, `ACTIVE`,
  keyboard focus as `FOCUS_VISIBLE`, and `DARK`. Do not re-derive it per
  component, or components will disagree.
- A disabled widget never gets `HOVER`, `ACTIVE` or `FOCUS_VISIBLE`. It gets
  `DISABLED` only.
- A disabled part fades by its recipe's `opacity` (`disabled:opacity-50`):
  `shared::opacity(&[part_stack, parent_stack], state)` multiplies the
  values down the part tree as CSS does, and
  `shared::style_painter(ui, opacity)` returns the painter faded by it. In a
  disabled `Ui` egui has faded the painter already, and `style_painter`
  leaves it as it is.
- Geometry is resolved without interaction states.
- Every custom-painted widget reports itself with `resp.widget_info(..)`.

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
