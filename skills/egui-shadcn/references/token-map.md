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
| `--ring` | focus ring (3px @ 50%) |
| `--radius` (10px) | `Theme.radius`; `radius_sm/md/lg/xl` |
| 4px spacing grid | `Spacing.item_spacing`, margins |
| 14px / medium | `text_styles[Body]` + `theme::family(ctx, FAMILY_MEDIUM)` |
| `shadow-*` | `Theme::shadow(ShadowLevel)`; the style's recipes pick the level |
| dialog overlay `bg-black/50` | the style's `dialog_overlay` recipe (`bg`) |
| `text-xs` / `text-sm` / `text-base` / `text-lg` | `Metrics.text_xs` / `text_sm` / `text_base` / `text_lg` |
| control heights, paddings, gaps, card padding, dialog and popover widths | the style's recipes (`button_size_*`, `card`, `dialog_content`, ...), not `Metrics` |
| `size-4` (icons, checkbox) | `Metrics.icon` |
| `border` / `ring-[3px]` widths | `Metrics.border` / the style's `ring` props |

Tokens are OKLCH literals converted by `color::oklch_to_srgb` into the `Palette`.
Customize by editing `Palette::light()/dark()` in the vendored `theme.rs`, or
add your own preset there (a brand palette is one more `Palette` constructor).
Text and icon sizes: `Theme { metrics: Metrics { icon: 20.0, ..Metrics::default() }, ..Theme::vega_dark() }`. Control sizes belong to the `Style`.

**Colors and text sizes live in `theme.rs` and nowhere else.** A widget that
writes `Color32::from_rgb(..)` or `FontId::proportional(14.0)` drifts from the
theme the first time the palette changes; that is how hand-built screens end up
with a navy panel inside a green app. `tests/tokens.rs` enforces this for the
registry; vendoring projects should run the same scan over their own UI files
(see SKILL.md, Acceptance bar).
