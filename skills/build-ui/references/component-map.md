# shadcn component to egui widget

Every widget lives in `shadcn_egui::components::<module>`. Builders go
through `ui.add(..)` and return an `egui::Response`. The markers are the
`cn-*` classes of the style CSS whose recipes the widget reads.

| shadcn | shadcn-egui |
|---|---|
| `<Button variant size>` | `shadcn_egui::components::button::Button::new(text)` with `.variant(ButtonVariant::…)` (`Default`, `Outline`, `Secondary`, `Ghost`, `Destructive`, `Link`), `.size(ButtonSize::…)` (`Xs`, `Sm`, `Default`, `Lg`, `Icon`, `IconXs`, `IconSm`, `IconLg`), `.enabled(..)`, `.expanded(..)` (aria-expanded), `.invalid(..)` (aria-invalid), `.has_popup(..)` (aria-haspopup). Markers: `cn-button`, `cn-button-variant-*`, `cn-button-size-*` |
| `<Button><Mic />Mute</Button>` | `Button::new("Mute").icon(&MIC)` (`data-icon="inline-start"`), `.icon_end(&MIC)` (`inline-end`). Icon only: `.size(ButtonSize::Icon)`; the text becomes the accessible name |
| `<Badge variant>` | `shadcn_egui::components::badge::Badge::new(text)` with `.variant(BadgeVariant::…)` (`Default`, `Secondary`, `Destructive`, `Outline`, `Ghost`, `Link`), `.icon(..)`, `.icon_end(..)`; or `badge(ui, text, variant)`. Markers: `cn-badge`, `cn-badge-variant-*` |
| `<Input>` | `shadcn_egui::components::input::Input::new(&mut text)` with `.hint(..)`, `.password(..)`, `.max_width(..)`, `.id_source(..)`, `.enabled(..)`, `.invalid(..)` (aria-invalid). Marker: `cn-input` |
| `<Label>` | `shadcn_egui::components::label::label(ui, text)`, or `Label::new(text).enabled(..)` (`peer-disabled:opacity-50`). Marker: `cn-label` |
| description text | `shadcn_egui::components::label::description(ui, text)`; reads `cn-card-description` (no marker of its own) |
| `<Label for="x">` + control | Name the control after the label: `let l = label(ui, "Email"); ui.add(Input::new(&mut s)).labelled_by(l.id);` (egui's `Response::labelled_by`). Switch, Checkbox and Select have no accessible name of their own (a Select reports its shown text as the value), so do this for each of them |
| `<Card size>` + Header/Title/Description/Action/Content/Footer | `shadcn_egui::components::card::Card::new().size(CardSize::Sm).footer(\|ui\| ..).show(ui, \|ui\| ..)`, or `shadcn_egui::layout::card(ui, \|ui\| ..)`. Inside: `card_header(ui, title, description)`, `card_header_with_action(ui, title, description, \|ui\| ..)`, `card_content(ui, \|ui\| ..)`, `card_title(ui, text)`, `card_description(ui, text)`. `card_content` adds only the style's `cn-card-content` gap, which only vega sets (nova, maia, lyra, mira, luma and rhea leave it out): wrap its children in `layout::vstack(ui, GAP, ..)` with the design's gap. The card fills the available width. Markers: `cn-card`, `cn-card-header`, `cn-card-title`, `cn-card-description`, `cn-card-action`, `cn-card-content`, `cn-card-footer` |
| `<Tabs>` + List/Trigger | `shadcn_egui::components::tabs::tab_bar(ui, &mut active, &labels)`, or `Tabs::new(&mut active, &labels)` with `.variant(TabsVariant::Line)`, `.enabled(..)`; the content sits `Tabs::content_gap(ui)` below. Horizontal only (`TabsOrientation::Horizontal`). Markers: `cn-tabs`, `cn-tabs-list`, `cn-tabs-list-variant-default`, `cn-tabs-list-variant-line`, `cn-tabs-trigger`, `cn-tabs-content` |
| `<Switch size>` | `shadcn_egui::components::switch::toggle(ui, &mut on)`, or `Switch::new(&mut on)` with `.size(SwitchSize::Sm)`, `.enabled(..)`, `.invalid(..)`. Markers: `cn-switch`, `cn-switch-thumb` |
| `<Checkbox>` | `shadcn_egui::components::checkbox::checkbox(ui, &mut checked)`, or `Checkbox::new(&mut checked)` with `.enabled(..)`, `.invalid(..)`. No indeterminate state. Markers: `cn-checkbox`, `cn-checkbox-indicator` |
| `<Slider orientation>` | `shadcn_egui::components::slider::Slider::new(&mut v, 0.0..=100.0)` with `.step(5.0)`, `.orientation(SliderOrientation::Vertical)`, `.enabled(..)`, `.label(..)`. One thumb. Markers: `cn-slider`, `cn-slider-track`, `cn-slider-range`, `cn-slider-thumb` |
| `<Select>` + Trigger/Content/Item | `shadcn_egui::components::select::Select::new("id", &mut index, &options)` with `.size(SelectSize::Sm)`, `.placeholder(..)`, `.enabled(..)`, `.invalid(..)`, `.width(..)`; or `select(ui, "id", &mut index, &options)`, which returns whether the choice changed. Markers: `cn-select-trigger`, `cn-select-trigger-icon`, `cn-select-content`, `cn-select-group` (its padding frames the rows), `cn-select-item`, `cn-select-item-indicator`. Not offered (options-list API): `cn-select-label`, `cn-select-separator`, per-option disabled; `cn-select-scroll-*` are resolved but unused (an egui `ScrollArea` scrolls) |
| `<Separator>` | `shadcn_egui::components::separator::separator(ui)`, `separator_vertical(ui)`. No marker: base classes only |
| `<Dialog>` + Header/Title/Description/Footer | `shadcn_egui::components::dialog::Dialog::new("id", "Title")` with `.description(..)`, `.width(..)`, `.backdrop(..)`, `.show_close_button(..)`, then `.show(ctx, &mut open, \|ui\| { ..; footer(ui, \|ui\| { primary; secondary }) })`. Closes on Esc, a backdrop click or the close button; the footer runs right to left, and in nova its muted strip reaches the panel edges. The width is the style's `sm:max-w-*`, capped to the window minus 32. The overlay blur is not drawn. Markers: `cn-dialog-overlay`, `cn-dialog-content`, `cn-dialog-close`, `cn-dialog-header`, `cn-dialog-footer`, `cn-dialog-title`, `cn-dialog-description` |
| `<Popover>` + Trigger/Content | `let t = ui.add(Button::new("Open")); shadcn_egui::components::popover::Popover::new(&t).width(..).show(\|ui\| ..)`. A click on the trigger toggles it; Esc or a click outside closes it. `popover_header(ui, title, description)` inside. Markers: `cn-popover-content`, `cn-popover-header`, `cn-popover-title`, `cn-popover-description` |
| `<Tooltip>` | `shadcn_egui::components::tooltip::tooltip(ui.add(..), "text")`. Markers: `cn-tooltip-content`, `cn-tooltip-arrow` |
| Lucide `<Mic />` | `shadcn_egui::components::icon::IconView::new(&shadcn_egui::icons::lucide::MIC)` with `.size(..)`, `.color(..)`; `paint_icon(painter, rect, icon, color, stroke)` for custom widgets. See `icons.md` |

Not yet ported: RadioGroup, Tags or Chips, Menubar, Table, DropdownMenu,
Accordion, gradients. Port a missing one with `shadcn-egui:port-component`
before using it; do not paint it ad hoc in the screen.

```rust
use shadcn_egui::components::badge::{Badge, BadgeVariant};
use shadcn_egui::components::switch::Switch;

fn row(ui: &mut egui::Ui, on: &mut bool) {
    ui.add(Badge::new("Beta").variant(BadgeVariant::Secondary));
    ui.add(Switch::new(on));
}
```
