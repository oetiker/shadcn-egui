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
| label + control form row | `shadcn_egui::layout::form_row(ui, label_w, "Label", \|ui\| ...)` |
| app shell (sidebar/topbar/main) | `SidePanel` / `TopBottomPanel` / `CentralPanel` |
| responsive breakpoints | branch on `ui.available_width()` each frame |
| CSS Grid tracks | `StripBuilder` rows × cells (no named lines) |

`StripBuilder` comes from `egui_extras`; an app that uses it directly adds
`egui_extras = { version = "0.34", default-features = false }`.

```rust
use shadcn_egui::components::button::{Button, ButtonVariant};
use shadcn_egui::layout;

fn footer(ui: &mut egui::Ui) {
    layout::space_between(
        ui,
        |ui| {
            ui.label("3 items selected");
        },
        |ui| {
            layout::row(ui, 8.0, |ui| {
                ui.add(Button::new("Cancel").variant(ButtonVariant::Outline));
                ui.add(Button::new("Delete").variant(ButtonVariant::Destructive));
            });
        },
    );
}
```

Gotcha: `StripBuilder.horizontal` fills all available vertical space. Wrap it
in `ui.allocate_ui(vec2(w, row_h), ...)` to cap a row's height (`form_row`
does this). When no helper fits, port one with `shadcn-egui:port-component`
instead of inlining `available_width()` arithmetic.
