---
name: egui-shadcn
description: Use when building or restyling a Rust egui GUI from a shadcn/web design (screenshot, component code, or description). Vendors a tested shadcn-v4 component module into the project and maps the design onto it, layout-first. Backend-agnostic — works with eframe, egui-winit, or any egui integration. Also use when an egui app renders without a GPU (CPU rasterizer, softbuffer, custom egui-winit loop) and is slow, pins a CPU core, redraws late, or misroutes pointer input.
---

# egui-shadcn

Rebuild shadcn-style designs as polished egui GUIs without endless iteration.
The hard part of egui is not styling — it is **layout** (single-pass, no flexbox).
This skill carries a tested component module and a layout-first workflow.

The components are **plain egui widgets**: they only touch `egui::Context`/`Ui`
and depend on nothing beyond `egui` + `egui_extras` (StripBuilder). They drop into
any backend — eframe, `egui-winit` + a custom rasterizer (softbuffer), glow, wgpu.
There is **no eframe or wgpu coupling** in what you ship.

## When to use
A shadcn (or generally web/Tailwind) design needs to become an egui UI: a
screenshot to recreate, shadcn component code to port, or a described screen that
must look professional.

This skill covers two distinct activities — keep them separate:
- **Part A — Using the components** to build a screen (the common case).
- **Part B — Porting a new component** into the library (when a needed widget
  isn't in the registry yet).

Driving egui yourself (`egui-winit`, `begin_pass`/`end_pass`) or rasterizing on
the CPU (no GPU, softbuffer)? Read `references/cpu-rendering.md` before writing
the frame loop: repaint scheduling, tessellation settings, rasterizer fast
paths, damage tracking, and the input-routing predicates that lie in a custom
loop.

---

# Part A — Using the components

1. **Vendor-in the registry.** Copy `registry/src/` and `registry/assets/` from
   this skill into the target project (as a module or sub-crate). Idempotent —
   skip files that already exist and match.
2. **Add the (minimal) deps.** Only two are required, and both are intentionally
   small because they propagate to your whole project:
   - `egui = "0.34"`
   - `egui_extras = { version = "0.34", default-features = false }`
     (StripBuilder/Size only — no image/svg/http loaders)

   **Do not add `eframe` on the registry's account.** Add eframe only if your app
   already uses it as its backend. The components work with whatever egui
   integration you already have (eframe, egui-winit + softbuffer, etc.).
3. **Apply the theme once**, right after the egui context is created (in
   eframe: the `run_native` app creator, on `cc.egui_ctx`):
   `egui_shadcn::Theme::vega_dark().apply(ctx)` (or `light`). Apply again only when
   the theme itself changes. Read tokens via `Theme::current(ctx)`.
4. **Decompose the design top-down:** app shell → panels
   (`SidePanel`/`TopBottomPanel`/`CentralPanel`); each region's **layout intent**
   (stack? row? grid? space-between?); components + their cva variants; token
   deviations (usually none).
5. **Map** using the reference tables:
   - `references/layout-map.md` — flexbox/Grid intent → egui helper (**read this
     first; layout is where iteration is lost**).
   - `references/component-map.md` — shadcn component/variant → module widget.
   - `references/token-map.md` — shadcn token → egui field.
   - `references/icons.md` — Lucide (or any outline SVG) icons without an
     image loader: generate only the icons the project uses.
6. **Build** from the vendored components. Drop to raw egui only when no helper
   fits — and then prefer adding a helper (see Part B) over inlining.

## Verify visually
The web-like feedback loop is: render to a PNG and **look at it** against the
design, then iterate on the numbers. The reference loop is an `egui_kittest`
snapshot test (features `["wgpu","snapshot"]`); generate with
`UPDATE_SNAPSHOTS=1 cargo test`.

This harness is **optional and dev-only** — it is how *this* crate verifies
itself, and the best loop if you can run it. It pulls `egui_kittest` + `wgpu` as
**`[dev-dependencies]`**, which never reach a shipped binary. If your project has
its own render/screenshot path (e.g. a softbuffer rasterizer), use that to capture
the PNG instead — the point is "look at a real render," not this specific harness.

---

# Part B — Porting a new component into the library

When you port a component or layout helper that isn't in the registry yet, do it
to the library's standard so the next project gets it for free. A port is done
only when **every** item below holds. A port is judgement work; these rules make
two ports of the same component come out the same.

Style values (sizes, colours, radii, shadows, ring widths, alphas) come from the
active style's recipes — see `references/styles.md` for the model. Values that no
recipe holds are *base classes*, and you quote them.

1. **Source.** Read the component's base `.svelte` files in
   `docs/src/lib/registry/ui/<name>/` of shadcn-svelte, checked out at the commit
   in `tools/stylegen/PIN`, and the bits-ui primitive it wraps. Never port from
   memory, a screenshot, or another style's resolved output.
2. **Inventory.** List every `cn-*` marker the source uses and every state it
   styles on (`data-checked`, `aria-invalid`, `data-[icon=…]`,
   `data-orientation`, …). Put the markers on the component's line in
   `tools/stylegen/markers.txt`. A marker that is new there needs a
   `stylegen generate` run, which adds the field to `Style`
   (`src/style/generated.rs`, generated — never edit by hand).
3. **Mapping.** Map each marker to its `Style` field (`cn-button-size-sm` →
   `s.button_size_sm`) and each state or context condition to a `State` flag
   (`src/style/recipe.rs`). Resolve a property over the stacked recipes in the
   order upstream joins their classes:
   `pick(&[&s.button, &s.button_variant_outline, &s.button_size_sm], |r| &r.height, st)`.
   Build the state with `theme.state(State::HOVER | …)`, which adds `DARK`.
4. **Quote the base classes.** A class the component source sets itself (not
   through a `cn-*` marker) is quoted verbatim on the line above the code that
   uses it: `// base: "<substring of the upstream class string>"`. When a base
   class overrides a style property, say so in that comment and do not read the
   recipe field. `stylegen drift` checks that every quote is still a substring of
   the upstream source.
5. **No style values in code.** `tests/tokens.rs` scans every file in its
   `PORTED` list, up to its `#[cfg(test)]` module. Colour and font-size literals are allowed only with a
   `// token-ok: <reason>` comment on the line above. Float literals other than
   `0.0`, `1.0` and halving/doubling, `Shadow { .. }` and alpha multipliers are
   allowed only with a `// base: "…"` quote on the same line or the line above.
   Add the new file to `PORTED`.
6. **egui sizing.** The widget sets its own size before placement (single pass).
   Use the `layout` helpers, no `available_width()` arithmetic. Resolve geometry
   *without* interaction states: size must not change on hover.
7. **Context.** The widget API takes whatever context the recipe conditions
   need: icon side (`ICON_START`/`ICON_END`), size variant (`SIZE_SM`),
   orientation, enclosing slot (`IN_BUTTON_GROUP`).
8. **Behaviour, not just looks** — see below.
9. **Interaction states** — see below.
10. **Snapshots.** Render with `common::snapshot_all(name, size, ui_fn)` from
    `tests/common/mod.rs`: one PNG per style × {dark, light}
    (`<name>_vega_dark.png`, …). **Open every PNG the component appears in** —
    also the ones of other tests (the gallery, forms) that now show a changed
    look — and compare it with the same component on the shadcn-svelte docs page
    for that style. Re-baseline the older single-theme PNGs the port affects, so
    the suite stays green (`UPDATE_SNAPSHOTS=1 cargo test`); delete the PNGs that
    no test writes any more.
11. **Docs.** A row in `references/component-map.md` with the markers. A line in
    `CHANGES.md` if the look changes (what the user sees, three sentences at
    most).
12. **Done** means: `stylegen drift` prints `drift: clean`; the component's
    markers have no *unsupported* class in `tools/stylegen/reports/<style>.md`
    (ignored classes are fine; each says why); snapshots reviewed; behaviour list
    checked; `cargo clippy --all-targets` and the tests clean; and `src/` copied
    into `skills/egui-shadcn/registry/src/` so both trees stay **byte-identical**.

## Behaviour, not just looks
A shadcn component is a styled bits-ui primitive. The primitive carries the
behaviour. Read its source (`node_modules/bits-ui`, or
https://bits-ui.com/docs/components/<name>), write down what it does, and check
each item in egui — with a test, or with a stated reason why it does not apply.
Cover keyboard handling, focus order, pointer capture while dragging, value
stepping and clamping, disabled handling, degenerate input, and the accessibility
role through `widget_info`.

Example, the list for Slider:
1. A click on the track jumps the value to that point, then snaps.
2. Dragging the thumb keeps the pointer captured, also outside the widget.
3. Arrow keys change the value by one step (continuous: 1% of the range);
   PageUp/PageDown by 10 steps; Home/End go to min/max.
4. The value is clamped to the range and snapped to the step.
5. Disabled: no pointer or keyboard change and no hover or focus styles.
6. `widget_info` reports a slider with its value and label.
7. `min == max`, `step <= 0`, a NaN value, a value outside the range: the widget
   draws, does not panic and never produces NaN.

## Interaction states
- One shared helper, `shared::interaction_state(theme, &resp, enabled, context)`
  (in `src/components/shared.rs`), turns a `Response` into a `State`: context
  flags, `HOVER`, `ACTIVE`, keyboard focus as `FOCUS_VISIBLE`, and `DARK`. Do
  not re-derive it per component, or components will disagree.
- **A disabled widget never gets `HOVER`, `ACTIVE` or `FOCUS_VISIBLE`.** It gets
  `DISABLED` only.
- Geometry is resolved without interaction states.
- Every custom-painted widget reports itself with `resp.widget_info(..)`.

Then **offer to open a PR** to `github.com/oetiker/egui-shadcn` (a nudge, not an
automatic pipeline — it needs the user's go-ahead and push/fork access).

## Dependency discipline (important)
Everything in `[dependencies]` propagates to **every consumer and every vendoring
project**. Keep that surface minimal.

- **Default: add no new runtime dependency.** A ported component should be
  expressible with `egui` + `egui_extras` (StripBuilder) + custom painting, like
  every existing one.
- **A new `[dependencies]` entry needs a strong, stated reason** — a capability
  that genuinely cannot be painted or computed in-crate, worth the build cost and
  version-coupling imposed on all consumers. Prefer a tiny focused crate over a
  large one; prefer an *optional, feature-gated* dep over an always-on one. Heavy
  crates (`eframe`, `image`/`resvg` loaders, `wgpu`) must **not** become runtime
  deps — they pull in backends/decoders most consumers don't want.
- **Tooling stays in `[dev-dependencies]`** (`eframe` for the example,
  `egui_kittest` + `wgpu` for snapshots). Those never reach consumers.
- When in doubt, **ask the user before adding a dependency**, and document the
  reason next to it in `Cargo.toml`.

---

## Hard rules (see `references/gotchas.md`)
- egui is single-pass: a widget sets its own size before placement; there is no
  `flex-grow` distribution across siblings. Use `egui_extras::StripBuilder`
  (`Size::relative/exact/remainder`) via the `layout` helpers.
- Don't fight it with `available_width()` arithmetic when a helper exists.
- Hover, active and focus follow the style's recipe; never invent a state colour.
- Apply the theme once, not every frame; read tokens via `Theme::current(ctx)`.
- **No style values in component code.** Values come from the style's recipes
  (`Theme::current(ctx).style`) and the palette; a value the style lacks is a
  base class quoted under `// base:`, or a gap to report. Geometry private to one
  widget's drawing (a checkmark path) may stay local.
- **Project-specific deviations go into the theme construction**
  (`let mut s = Style::nova(); s.<part>.<field> = …;`), never into vendored files.
- **Every custom-painted widget reports itself** with `resp.widget_info(..)`
  (role, label, enabled, selected), or screen readers see nothing.

## Acceptance bar
The reference `egui_shadcn::reference::settings_ui` (a settings screen whose tabs
double as a component gallery) is the quality yardstick: match its restraint —
subtle 1px borders, one radius, muted palette, 14px text, consistent focus rings.
Run `cargo run --example settings` to see it live.

Before calling a screen done, scan the project's UI files for literals that
bypass the theme. Carry a positive control, because a pattern that matches
nothing looks exactly like a clean codebase:

```bash
printf 'Color32::from_rgb(1,2,3)\n' | grep -cE 'Color32::(from_|WHITE|BLACK)|from_(black|white)_alpha|FontId::[a-z_]+\([0-9]|\.size\([0-9]'   # must print 1
grep -nE 'Color32::(from_|WHITE|BLACK)|from_(black|white)_alpha|FontId::[a-z_]+\([0-9]|\.size\([0-9]' src/ui/*.rs   # your UI files
```

Each hit is either a missing token or carries a `// token-ok: <reason>` comment
on the line above. The registry runs the same check as `tests/tokens.rs`.
