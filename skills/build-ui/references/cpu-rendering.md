# Fast egui without a GPU (CPU rasterizer, custom integration)

Applies when you drive egui yourself (`egui-winit` + `begin_pass`/`end_pass`,
not eframe) and rasterize its meshes on the CPU, e.g. into a `softbuffer`
surface on a box with no usable GPU. eframe + wgpu/glow handles most of this
for you; a hand-rolled loop does not.

**Measure before optimizing.** Time the raster, the composite copy and the
present separately. In practice the raster dominates (~100 ms on a trivial
1280x800 screen) while copy and present are sub-millisecond, and the second
killer is *how often* you raster, not how long one raster takes.

## 1. Frame loop: redraw exactly when egui asks

| Symptom | Cause | Fix |
|---|---|---|
| New screen, cursor blink or animation appears only after the next mouse move | loop runs `ControlFlow::Wait` and ignores egui's own repaint requests | read `full_output.viewport_output[&ViewportId::ROOT].repaint_delay`: zero → `request_redraw()` now; finite → `ControlFlow::WaitUntil(now + delay)`; `Duration::MAX` → wait for the next OS event |
| 100% CPU while idle or typing | loop free-runs at 60 fps, or presents once per external event (network frame, timer) | event-driven present: skip raster **and** present when nothing changed (see 3) |
| Toggles/fades/tab indicator look like slow staged steps | every animation frame is a full CPU raster | `ctx.style_mut(\|s\| s.animation_time = 0.0)` — `animate_bool` then returns its target at once; the registry's `switch.rs` keeps working |

The deferred first-frame font-atlas build (`set_fonts` lands next frame) also
arrives as a repaint request, so ignoring `repaint_delay` leaves the first
screen with fallback fonts until the user moves the mouse.

## 2. Tessellation settings for a non-AA rasterizer

- `ctx.options_mut(|o| o.tessellation_options.feathering = false)`.
  Feathering adds ~1 px alpha "wings" that assume coverage-based blending. An
  integer rasterizer turns them into hard partial-alpha fringes (dimmed
  rounded-corner strokes, banding in stacked translucent focus rings). Off,
  every solid fill is a clean 4-vertex / 6-index quad, which enables the
  row-fill fast path below. Text is unaffected (pre-antialiased atlas texture).
- Tessellate with the **same** `pixels_per_point` egui laid out with. A
  mismatch gives misshapen, mis-sized text and a flood of epaint
  `pixels_per_point` warnings.

## 3. The rasterizer itself

- **Solid-fill fast path: key it on `egui::epaint::WHITE_UV`, never a
  literal.** epaint 0.34 tags solid fills with `WHITE_UV = (0.0, 0.0)`; older
  code and blog posts test `(0.5, 0.5)`. With the wrong constant the fast path
  silently never fires and every fill, including the full-window background,
  takes the ~30x slower textured path (barycentrics + UV interp + sample +
  blend). Compare with an epsilon (1e-3); a mesh is solid when all its
  vertices are.
- **Row-fill opaque axis-aligned rects.** A mesh with 4 vertices, 6 indices,
  all `WHITE_UV`, one color with alpha 255 and axis-aligned positions is a
  rectangle: fill rows with `slice::fill`, skip the per-pixel inside test.
  This catches panel/`Frame` backgrounds and button faces, i.e. most pixels.
- **Hoist the texture lookup** out of the per-pixel loop: resolve
  `mesh.texture_id` once per primitive (measured 195 → 84 ms per frame on a
  Retina-sized surface).
- Apply `textures_delta.set` before and `textures_delta.free` after the
  frame, outside the raster call, so a clipped or skipped raster never drops
  an atlas update.

## 4. Raster less: damage tracking

- **Fingerprint the clipped primitives** (hash clip rects, texture ids,
  indices, vertex pos/uv/color bits). Unchanged fingerprint, empty
  `textures_delta` and no external damage → skip raster and present
  entirely. `egui-winit` requests a repaint on every `CursorMoved`, so
  without this a mouse move re-rasters the whole window.
- **Clip the raster to damage.** Damage = changed external content ∪ egui's
  area this frame ∪ egui's area last frame (so vanished widgets get erased).
  Per damage rect: restore the background under it, then raster egui clipped
  to it. If your rect coalescing does not return disjoint rects, do restore →
  raster per rect in sequence, otherwise overlaps blend translucent egui
  pixels twice.
- **Copy only what changed to the surface.** softbuffer's `buffer.age()` says
  how many presents ago this back buffer was shown; copy the union of the
  damage since then (full copy when age is 0 or after a resize). Plain
  `present()` is enough: the whole buffer must be valid anyway, and the
  damage list of `present_with_damage` is only a compositor hint.
- **Modal over live content:** do not alpha-blend a full-window dimmed
  backdrop every frame. Dim the current background **once** into the
  background layer when the modal opens, pause updates of the content behind
  it, and let only the dialog re-raster; on close, mark everything dirty.

## 5. Integration traps (custom loop)

- **`ctx.is_pointer_over_egui()` is always `true`** under a `CentralPanel`
  when you drive egui with `begin_pass`/`end_pass`: the modern code path needs
  state only `Context::run_ui` writes, and the legacy fallback sees the central
  panel claim the whole unused rect. Likewise `wants_pointer_input()` over a
  `CentralPanel`, and `wants_keyboard_input()` is true for *any* focused
  widget (a button reached by Tab). For "is the pointer over floating UI":

  ```rust,ignore
  ctx.pointer_interact_pos()
      .and_then(|p| ctx.layer_id_at(p))
      .is_some_and(|l| l.order != egui::Order::Background)
  ```

  For keyboard routing use `ctx.text_edit_focused()` plus an explicit modal
  flag. A purely decorative overlay `Area` (a badge, a grid) should be
  `.interactable(false)` so it does not claim the pointer; a modal backdrop
  is the opposite case and must keep blocking clicks. Unit-test the routing predicate itself:
  `egui::Context::default()` runs headless; feed it a `RawInput`.
- **`egui-winit` default features pull `smithay-clipboard`**, which segfaults
  on Wayland (its thread shares winit's `wl_display`; unmaintained, egui#3805).
  Use `egui-winit = { version = "0.34", default-features = false, features =
  ["links", "wayland", "x11"] }` and, if you need a clipboard, a crate with its
  own Wayland connection (`wl-clipboard-rs`). Check with `cargo tree -i
  smithay-clipboard`.
- **Colors on a non-sRGB GPU surface.** softbuffer writes raw sRGB bytes. If
  the same app also has a wgpu path rendering to a `*Unorm` (non-sRGB) surface,
  emit raw sRGB from custom shaders too; linearizing first makes overlays
  render darker than on the CPU path.
