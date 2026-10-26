# Web layout intent to egui helper

egui is single-pass: rebuild the layout intent instead of translating CSS.
The helpers live in `shadcn_egui::layout`.

| Web intent | egui approach |
|---|---|
| `display:flex; flex-direction:row; gap:N` | `shadcn_egui::layout::row(ui, N, \|ui\| ...)` |
| `flex-wrap:wrap` row | `shadcn_egui::layout::wrap_row(ui, N, \|ui\| ...)` |
| `flex-direction:column; gap:N` | `shadcn_egui::layout::vstack(ui, N, \|ui\| ...)` |
| `justify-content:space-between` | `shadcn_egui::layout::space_between(ui, \|ui\| {left}, \|ui\| {right})` |
| `flex:1` / `flex-grow` across siblings | `egui_extras::StripBuilder` with `Size::remainder()` per growing cell |
| fixed + fluid columns | `StripBuilder` `Size::exact(w)` + `Size::remainder()` |
| proportional columns (`fr`) | `StripBuilder` `Size::relative(frac)` |
| a `.card` block | `shadcn_egui::layout::card(ui, \|ui\| ...)`, or `components::card::Card` for sizes and a footer |
| `w-full max-w-*` on a custom-painted widget, `max-w-[calc(100%-Npx)]` on a floating panel | `shadcn_egui::layout::capped_width(ui, max, content_min)`, `layout::overlay_width(ctx, max, inset, content_min)`; allocate with `layout::allocate_overflowing` so a widget wider than its room overflows instead of widening its column |
| label + control form row | `shadcn_egui::layout::form_row(ui, label_w, "Label", \|ui\| ...)` |
| app shell (sidebar/topbar/main) | `SidePanel` / `TopBottomPanel` / `CentralPanel` |
| responsive breakpoints (`sm:`, `md:`, `@container`) | branch on `shadcn_egui::layout::window_at_least(ui.ctx(), px)` (media query) or `layout::container_at_least(ui, px)` (container query); components apply their own |
| CSS Grid tracks | `StripBuilder` rows × cells (no named lines) |

Every gap `N` here is a named constant from the design (Tailwind `gap-N` is
N×4 px); see the hard rules in SKILL.md. `StripBuilder` comes from
`egui_extras`; an app that uses it directly adds
`egui_extras = { version = "0.34", default-features = false }`.

```rust
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::layout;

/// gap-2 between the footer buttons.
const ACTION_GAP: f32 = 8.0;

fn footer(ui: &mut egui::Ui) {
    layout::space_between(
        ui,
        |ui| {
            ui.label("3 items selected");
        },
        |ui| {
            layout::row(ui, ACTION_GAP, |ui| {
                ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                ui.add(Button::new("Delete").variant(ButtonVariant::Destructive));
            });
        },
    );
}
```

- `layout::row` is at least `ui.spacing().interact_size.y` tall (36 px in
  luma), whatever its children need. For a tight list (radio rows,
  checkbox rows), lower `interact_size.y` in a scope around the list:
  `ui.scope(|ui| { ui.spacing_mut().interact_size.y = ROW_H; … })`, with
  `ROW_H` the design's row height.
- In `layout::row`, `wrap_row` and `space_between` a child wider than the
  room left (an input or select raised to its content minimum, a long
  label) pushes the children after it past its whole width, and the row
  does not widen its parent, as a flex row in the browser. A
  `layout::vstack` in such a row grows to its widest child, as a flex
  column does, and pushes the row's next children too. A plain
  `ui.horizontal` cannot: the next child covers the overflow, and a child
  that runs past the room widens the parent. Use the crate rows for rows of
  controls.
- `card_content` spaces its children with the gap of the style's
  `cn-card-content` recipe, and only vega sets one (nova, maia, lyra, mira,
  luma and rhea set none). Put the
  children in a `layout::vstack` with the design's gap.

Gotcha: `StripBuilder.horizontal` fills all available vertical space. Wrap it
in `ui.allocate_ui(vec2(w, row_h), ...)` to cap a row's height (`form_row`
does this). When no helper fits, port one with `shadcn-egui:port-component`
instead of inlining `available_width()` arithmetic.
