# Icons

shadcn-egui draws icons from vector data compiled into the binary, with no
image loader and no SVG library at runtime.

## Icon libraries

The five icon libraries of the shadcn create page are one crate each, re-exported
under `shadcn_egui::icons`:

| `IconLibrary` | Module | Feature |
|---|---|---|
| `Lucide` | `shadcn_egui::icons::lucide` | none, always present |
| `Hugeicons` | `shadcn_egui::icons::hugeicons` | `icons-hugeicons` |
| `Tabler` | `shadcn_egui::icons::tabler` | `icons-tabler` |
| `Phosphor` | `shadcn_egui::icons::phosphor` | `icons-phosphor` |
| `Remixicon` | `shadcn_egui::icons::remixicon` | `icons-remixicon` |

`icons-all` enables every library. Each module holds every icon of its
library as a `pub const` named after the upstream file name in upper snake
case (`circle-alert` is `CIRCLE_ALERT`), plus `ALL` and `SLOTS`. Remix Icon
stays at 4.8.0, its last Apache-2.0 release.

The library comes from the design: a preset code or `components.json`'s
`iconLibrary` picks it, and `Preset::icons(IconLibrary::…)` sets it (see
`presets.md`). Use the icons of the same library in the app's own screens, so
they match the icons the components draw.

## Drawing an icon

`IconView` is a widget. It is `Metrics.icon` px square, drawn in the
foreground colour, unless `.size(..)` or `.color(..)` say otherwise:

```rust
use shadcn_egui::components::icon::IconView;

fn mic(ui: &mut egui::Ui) {
    ui.add(IconView::new(&shadcn_egui::icons::lucide::MIC));
}
```

```rust
use shadcn_egui::components::badge::Badge;
use shadcn_egui::components::button::{Button, ButtonSize};
use shadcn_egui::components::icon::IconView;
use shadcn_egui::icons::lucide;
use shadcn_egui::Theme;

fn icons(ui: &mut egui::Ui) {
    let t = Theme::current(ui.ctx());
    ui.add(IconView::new(&lucide::MIC).size(24.0).color(t.palette().destructive));
    ui.add(Button::new("Mute").icon(&lucide::MIC));
    ui.add(Button::new("Close").icon(&lucide::X).size(ButtonSize::Icon));
    ui.add(Badge::new("Live").icon_end(&lucide::RADIO));
}
```

- Buttons and badges take an icon through `.icon(..)` (before the text) and
  `.icon_end(..)` (after it). An icon-only button uses `ButtonSize::Icon`
  (or `IconXs`, `IconSm`, `IconLg`); its text becomes the accessible name.
- An `IconView` has no accessible name; give it one with
  `.labelled_by(label.id)` when it carries meaning.
- A custom-painted widget draws an icon with
  `shadcn_egui::components::icon::paint_icon(painter, rect, &icon, colour, t.metrics.icon_stroke)`.
- The stroke width is `Metrics.icon_stroke` in icon units (Lucide draws 2 on
  a 24 grid) and scales with the icon.
- Icons are rasterised on the CPU into a cached texture per icon and pixel
  size, so a translucent colour applies once and very large icons cost a
  texture each. This works the same under a CPU rasterizer.

## Icon slots

Components draw a few icons of their own: the checkbox tick and the selected
Select item (`check`), the Select trigger icon (`select_trigger`), the plain
chevrons (`chevron_down_plain`, `chevron_up`, `chevron_left`,
`chevron_right`), three dots (`more_horizontal`) and the Dialog close button
(`close`). These come from `Theme::icons`, an `IconSlots` value. It gains
slots over time, so build one from `IconSlots::all(icon)` and set the slots
to change. A preset fills the slots from its icon library's `SLOTS`,
so switching the library changes these icons too. Without a preset, a theme
uses Lucide's slots; `theme.icons(shadcn_egui::icons::lucide::SLOTS)` sets
them explicitly, and `shadcn_egui::icons::slots(IconLibrary::…)` returns a
library's slots, or `None` when its feature is off.
