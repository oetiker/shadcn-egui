# egui gotchas (why web habits fail)

- **Single pass, no negotiation.** A widget commits its size before the parent's
  size is known. No `flex-grow` across siblings, no intrinsic min/max-content.
  Use `StripBuilder` for distribution.
- **`gap` is `item_spacing`.** Set it inside the stack closure; it doesn't cascade
  like CSS.
- **`StripBuilder.horizontal` eats all vertical space.** Wrap in `allocate_ui` with
  a fixed row height.
- **First-frame jitter & deferred fonts.** `ctx.set_fonts` takes effect next frame,
  so a named font family may be missing on frame 1. Use `theme::family(ctx, name)`
  which falls back to Proportional until the atlas is ready. For snapshot tests,
  `harness.run()` steps frames before snapshotting.
- **`override_text_color` is baked into galleys.** `WidgetText::into_galley` bakes
  the global text color, so `painter().galley(.., color)` can't override it. To
  force a per-widget color (for example dark text on a light button), build the galley
  with `painter().layout_no_wrap(text, font_id, explicit_color)` (see `button.rs`).
- **`strong()` is not a bold font.** egui's `strong` brightens color. For weight use the
  named families via `theme::family(ctx, FAMILY_MEDIUM|FAMILY_SEMIBOLD)`.
- **Geometry is resolved without interaction states.** Sizes, paddings and
  gaps are picked before hover, press or focus are known, so a style rule that
  changes a size on `hover:` or `active:` does not move the layout. Only colours,
  borders, rings, shadows and the 1px press nudge follow those states.
- **egui has no backdrop blur.** `backdrop-blur-*` on a dialog overlay is not
  drawn; the overlay is only the translucent fill.
- **No ligatures or contextual alternates.** egui draws one glyph per
  character, so a font's `liga`, `calt` and `ccmp` substitutions (Geist's
  `tt`, Manrope's case hyphen in "2-3", JetBrains Mono's `//`) are not
  drawn, and a line holding such a pair can break one word off from
  Chrome. Crate text otherwise breaks where Chrome does: at spaces, before
  and after em dashes, after hyphens and en dashes, at a zero-width
  space, at a soft hyphen (U+00AD, drawn as a hyphen at the line end) and
  between CJK characters.
- **Translucent menus draw opaque.** The menu colours
  `default-translucent` and `inverted-translucent` draw like `default`
  and `inverted`, and log a warning. Upstream blurs what lies behind the
  menu (`backdrop-blur-2xl`); egui cannot, and alpha without the blur
  makes the text behind the list compete with the list.
- **The ring is drawn outside the border box.** A focus ring (`ring-3`) is
  painted around the control and takes no layout space, as in CSS. Leave room
  around a control at the edge of a clipped area, or the ring is cut off.
- **Heading font.** Upstream marks card and dialog titles with
  `cn-font-heading`; the crate draws them with `heading_family_for(ctx, weight)`,
  which is the theme's heading font (`Theme::fonts(body, Some(heading))`) or
  the body font when none is set.
- **CSS transitions.** Crate components move a value where upstream's
  `transition-*` does. For your own widget, paint
  `components::motion::transition(ctx, id, target, &stack, Transition::OTHER, state)`
  with the part's recipe stack (upstream's duration and easing), or
  `ctx.animate_bool(id, on)` and lerp. Both honour `animation_time`; set to
  0 (as `cpu-rendering.md` advises) they jump to the target, which is what
  you want.
- **A fade you time yourself does not honour `animation_time`.** Opacity
  computed from `ctx.input(|i| i.time)` (a hint that fades out after N seconds)
  only moves while frames are drawn. `request_repaint_after(N)` alone draws one
  frame at the end, so the element vanishes instead of fading. Either repaint
  every frame while the fade runs (`ctx.request_repaint()` while `0 < alpha < 1`)
  or drop the fade and show/hide in one step, which is the right call under a
  CPU rasterizer.
- **A nested disable fades twice.** `ui.add_enabled_ui(false, ..)` inside a
  `Ui` that is already disabled (a disabled card's content) multiplies
  egui's fade in again, so its widgets show at 25 % instead of the style's
  50 %; the crate cannot see the second fade. Disable an area once, at its
  outermost `Ui`.
- **One remembered popup.** egui's popup memory holds one open popup id, so
  opening a Popover closes any open menu or combo box (as on the web). For a
  popover inside a popover, keep the inner one's state yourself
  (`egui::Popup::..open_bool(&mut flag)`).
- **Overlays must share an area or use distinct `Order`s.** A backdrop and a
  card in two areas at the same `Order` can swap z-order once another window
  has been shown, leaving the backdrop on top. `Dialog` uses `egui::Modal`,
  which draws both in one area.
- **No multi-stop/radial gradients.** Gradients need a hand-built `Mesh`. A
  `box-shadow` stack goes through `RecipeBox`, which draws every Tailwind
  layer; egui's own `Shadow` is one linear ramp and does not match the browser.
- **Apply the theme once, not every frame.** Call `Theme::apply` right after the
  context is created, and again only when the theme changes (a new preset). It
  installs the dark and the light style, so a light/dark toggle
  (`ctx.set_theme(..)`) or a system theme switch needs no second `apply`, and eframe restores its saved state before the app creator runs
  (styles are not saved). Re-applying every frame buys nothing and hides bugs:
  code that overwrites the style is silently repainted over on the next frame
  instead of showing. Read tokens via `Theme::current(ctx)` inside widgets.
- **Icons are rasterised.** `paint_icon` draws each icon from a cached
  coverage texture at the physical pixel size, so a translucent colour
  applies once; very large icons cost a texture each.
