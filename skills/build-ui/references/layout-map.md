# Web layout intent → egui helper

egui is single-pass; rebuild layout *intent*, don't translate CSS.

| Web intent | egui approach |
|---|---|
| `display:flex; flex-direction:row; gap:N` | `layout::row(ui, N, |ui| ...)` |
| `flex-direction:column; gap:N` | `layout::vstack(ui, N, |ui| ...)` |
| `justify-content:space-between` | `layout::space_between(ui, |ui|{left}, |ui|{right})` |
| `flex:1` / `flex-grow` across siblings | `egui_extras::StripBuilder` with `Size::remainder()` per growing cell |
| fixed + fluid columns | `StripBuilder` `Size::exact(w)` + `Size::remainder()` |
| proportional columns (`fr`) | `StripBuilder` `Size::relative(frac)` |
| a `.card` block | `layout::card(ui, |ui| ...)` |
| label + control form row | `layout::form_row(ui, label_w, "Label", |ui| ...)` |
| app shell (sidebar/topbar/main) | `SidePanel` / `TopBottomPanel` / `CentralPanel` |
| responsive breakpoints | branch on `ui.available_width()` each frame |
| CSS Grid tracks | `StripBuilder` rows × cells (no named lines) |

Gotcha: `StripBuilder.horizontal` fills all available vertical space — wrap it in
`ui.allocate_ui(vec2(w, row_h), ...)` to cap a row's height (see `form_row`).
When no helper fits, add one to `layout.rs` instead of inlining `available_width()`
math.
