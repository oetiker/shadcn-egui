# shadcn token → egui mapping

| shadcn token | egui target |
|---|---|
| `--background` | `Visuals.panel_fill`; `Palette.background` |
| `--foreground` | `Visuals.override_text_color`; `Palette.foreground` |
| `--card` / `--card-foreground` | `layout::card` fill / text |
| `--popover` | ComboBox/menu surfaces (`window_fill`) |
| `--primary` / `-foreground` | Button Default fill / text |
| `--secondary` / `-foreground` | Button Secondary; `widgets.*.bg_fill` |
| `--muted` / `-foreground` | Tabs track; description text |
| `--accent` | Ghost hover fill; native widget hover |
| `--destructive` | Button Destructive fill |
| `--border` | 1px strokes everywhere; `widgets.*.bg_stroke` |
| `--input` | TextEdit bg; switch off-track |
| `--ring` | `Palette.ring`; the ring colour and width come from the style's recipe props `ring_color` / `ring_width` |
| `--radius` (10px) | `Theme.radius`; `radius_sm/md/lg/xl` |
| 4px spacing grid | `Spacing.item_spacing`, margins |
| 14px / medium | `text_styles[Body]` + `shadcn_egui::theme::family_for(ctx, Weight::Medium)` |
| `shadow-*` | `Theme::shadow(ShadowLevel)` gives every layer as a `ShadowLayer`; the style's recipes pick the level and `RecipeBox` paints it |
| dialog overlay `bg-black/50` | the style's `dialog_overlay` recipe (`bg`) |
| `text-xs` / `text-sm` / `text-base` / `text-lg` | `Metrics.text_xs` / `text_sm` / `text_base` / `text_lg` |
| control heights, paddings, gaps, card padding, dialog and popover widths | the style's recipes (`button_size_*`, `card`, `dialog_content`, ...), not `Metrics` |
| `text-*` line heights (Tailwind's implicit `leading`) | `Metrics.line_xs` / `line_sm` / `line_base` / `line_lg`; `Theme::line_px(TextSize)` |
| `size-4` (icons, checkbox) | `Metrics.icon` |
| `border` width | the style's `border_width` props; `Metrics.border` where a recipe sets none (the separator fallback) |
| `h-px` / `w-px` separator | `separator.height` / `separator.width` |
| `ring-[3px]` | the style's `ring_width` / `ring_color` props |

The palettes are OKLCH values from upstream, converted with
`shadcn_egui::color::oklch_to_srgb`. A preset (`references/presets.md`) picks
the palette. To deviate from it, change the theme in the app and apply the
result; never edit copies of crate code:

```rust
use shadcn_egui::color::oklch_to_srgb;
use shadcn_egui::Theme;

/// A brand primary on top of whatever preset is active.
fn brand(ctx: &egui::Context) {
    let mut t = Theme::current(ctx);
    let brand = oklch_to_srgb(0.55, 0.2, 260.0);
    t.light.primary = brand;
    t.dark.primary = brand;
    t.metrics.icon = 20.0;
    t.apply(ctx);
}
```

Text and icon sizes live in `Metrics`; control sizes belong to the `Style`
(the `shadcn-egui:port-component` skill explains recipes in its
`references/styles.md`).

**Colours and text sizes come from the theme and nowhere else.** A widget that
writes `Color32::from_rgb(..)` or `FontId::proportional(14.0)` drifts from the
theme the first time the palette changes; that is how hand-built screens end
up with a navy panel inside a green app. Run the literal scan from SKILL.md
(Acceptance bar) over the app's UI files.
