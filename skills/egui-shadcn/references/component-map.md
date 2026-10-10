# shadcn component → egui widget

| shadcn | module widget |
|---|---|
| `<Button variant size>` | `components::button::Button::new(t).variant(..).size(..)` via `ui.add(..)` — variants Default/Outline/Secondary/Ghost/Destructive/Link; sizes Xs/Sm/Default/Lg/Icon/IconXs/IconSm/IconLg; `.enabled(..)`, `.expanded(..)` (aria-expanded), `.invalid(..)` (aria-invalid), `.has_popup(..)` (aria-haspopup). Markers: `cn-button`, `cn-button-variant-*`, `cn-button-size-*` |
| `<Input>` | `components::input::Input::new(&mut s).hint(..).password(..).max_width(..)` via `ui.add(..)`; `.enabled(..)`, `.invalid(..)` (aria-invalid). Marker: `cn-input` |
| `<Label>` | `components::label::label(ui, "..")`, or `Label::new("..").enabled(..)` via `ui.add(..)` (`peer-disabled:opacity-50`). Marker: `cn-label` |
| description text | `components::label::description(ui, "..")` — reads `cn-card-description` (no marker of its own) |
| `<Card>` + Header/Title/Description | `layout::card(ui, |ui| ...)` + `components::card::{card_title, card_description}` |
| `<Tabs>` | `components::tabs::tab_bar(ui, &mut active, &[..])` |
| `<Switch size>` | `components::switch::toggle(ui, &mut on)` or `ui.add(Switch::new(&mut on).size(SwitchSize::Sm).enabled(..).invalid(..))`. Markers: `cn-switch`, `cn-switch-thumb` |
| `<Checkbox>` | `components::checkbox::checkbox(ui, &mut checked)` or `ui.add(Checkbox::new(&mut checked).enabled(..).invalid(..))`. Markers: `cn-checkbox`, `cn-checkbox-indicator`. No indeterminate state |
| `<Select>` | `components::select::select(ui, "id", &mut idx, &[..])` |
| `<Separator>` | `components::separator::separator(ui)`, `separator_vertical(ui)`. No marker: base classes only |
| `<Badge variant>` | `components::badge::Badge::new(t).variant(..).icon(..).icon_end(..)` via `ui.add(..)`, or `badge(ui, t, v)` — variants Default/Secondary/Destructive/Outline/Ghost/Link. Markers: `cn-badge`, `cn-badge-variant-*` |
| `<Dialog>` + Header/Title/Description/Footer | `components::dialog::Dialog::new("id", "Title").description(..).show(ctx, &mut open, \|ui\| { ..; dialog::footer(ui, \|ui\| { primary; secondary }) })` — closes on Esc, backdrop, ×; footer runs right to left |
| `<Popover>` + Trigger/Content | `let t = ui.add(Button::..); components::popover::Popover::new(&t).show(\|ui\| ..)` — click toggles, click outside closes |
| `<Tooltip>` | `components::tooltip::tooltip(ui.add(..), "text")` |
| Lucide `<Mic />` | `components::icon::IconView::new(&icons::MIC)` via `ui.add(..)`; `.size(..)`, `.color(..)`; data from `icongen` (see `references/icons.md`) |
| `<Button><Mic />Mute</Button>` | `Button::new("Mute").icon(&icons::MIC)` (`data-icon="inline-start"`), `.icon_end(..)` (`inline-end`); icon-only: `.size(ButtonSize::Icon)`, the text becomes the accessible name |

Not yet ported (add when needed): RadioGroup, Tags/Chips, Menubar, Table,
DropdownMenu, Accordion, gradients. Build them as new
files under `components/` following the custom-paint pattern in `button.rs`
(floating components like Menubar/Dropdown/Dialog can lean on egui's native
`menu`/`Area`/`Window` primitives, then theme them with the tokens). After it
renders correctly, follow SKILL.md Part B and **offer to open a PR upstream** so
the registry grows for everyone.
