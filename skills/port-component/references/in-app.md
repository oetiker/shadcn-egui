# Porting into an app

The component lives in the app and uses the crate as a library. It follows
the standard in `SKILL.md`; this file covers what differs from a port into
the crate.

## What the app can use

- `Theme::current(ctx)`: the active theme, with `palette`, `metrics`,
  `radius`, `dark`, `icons` and `style`.
- The `Style` fields of the markers the crate has ported
  (`theme.style.button`, `theme.style.checkbox`, …), for parts a new component
  shares with an existing one.
- `shadcn_egui::style::{pick, Recipe, Prop, Rule, When, State, Paint, Role, Radius, Rung, TextSize, Weight}`.
- `Theme::{state, paint, role, radius_px, text_px, line_px, shadow}` to turn
  recipe values into pixels and colours.
- `shadcn_egui::components::shared::interaction_state` for hover, press and
  focus, and the other helpers in `components::shared` (`paint_box`, `ring`,
  `disabled_painter`).
- `shadcn_egui::components::icon::paint_icon` and the theme's icon slots.

## Recipes for new markers

`Style` holds only the markers of ported components, so a new component's
markers (`cn-radio-group`, `cn-radio-group-item`, …) have no field. The app
builds the recipes itself:

- One `Recipe` per marker for each style the app uses, with the values read
  from that style's `style-<name>.css` in shadcn-svelte at the pinned commit
  (`SKILL.md`, step 1). Never take the values from another style.
- A `// style: "<css class string>"` quote above each recipe, copied verbatim
  from the CSS. The quote stays in the code, so a later port into the crate
  can check the recipe against upstream.
- A rule's `rank` orders it the way Tailwind orders the CSS. Take the bit of
  each variant from a generated recipe in the crate that uses the same
  variant (`crates/shadcn-egui/src/style/nova.rs`); a rule with several
  variants ORs their bits. The bits used below: `focus-visible:` `1 << 19`, `aria-checked:` `1 << 26`, `aria-invalid:`
  `1 << 28`, `data-checked:` `1 << 30`, `dark:` `1 << 50`.
- Classes the crate ignores stay out of the recipe: layout keywords (`flex`,
  `items-center`), transitions and animations, and variants for slots the
  crate does not ship (`group-has-[:focus-visible]/field-label:`).
  `tools/stylegen/reports/<style>.md` in the repository says why for each.
- The component selects the recipe for `Theme::current(ctx).style.name`. A
  style without a recipe falls back to the first one and logs a warning.

`cn-radio-group-item` in nova, at shadcn-svelte `493481fa`:

```rust
use shadcn_egui::style::{Paint, Prop, Radius, Recipe, Role, Rule, Rung, State, When};
use shadcn_egui::Theme;

fn when(all: State) -> When {
    When::new(all, State::NONE)
}

// style: "border-input dark:bg-input/30 data-checked:bg-primary data-checked:text-primary-foreground dark:data-checked:bg-primary data-checked:border-primary aria-invalid:aria-checked:border-primary aria-invalid:border-destructive focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 dark:aria-invalid:border-destructive/50 flex size-4 rounded-full focus-visible:ring-3 aria-invalid:ring-3 group-has-[:focus-visible]/field-label:ring-0 group-has-[:focus-visible]/field-label:not-data-checked:border-input group-has-[:focus-visible]/field-label:data-checked:border-primary"
fn radio_group_item_nova() -> Recipe {
    let (dark, focus, invalid) = (State::DARK, State::FOCUS_VISIBLE, State::INVALID);
    // data-checked: and aria-checked: are one flag with different rank bits.
    let checked = State::CHECKED;
    Recipe {
        width: Prop::fixed(16.0),
        height: Prop::fixed(16.0),
        radius: Prop::fixed(Radius::Rung(Rung::Full)),
        bg: Prop {
            base: None,
            rules: vec![
                Rule { when: when(checked), rank: 1 << 30, value: Paint::Role(Role::Primary, 1.0) },
                Rule { when: when(dark), rank: 1 << 50, value: Paint::Role(Role::Input, 0.3) },
                Rule { when: when(dark | checked), rank: (1 << 30) | (1 << 50), value: Paint::Role(Role::Primary, 1.0) },
            ],
        },
        text: Prop {
            base: None,
            rules: vec![
                Rule { when: when(checked), rank: 1 << 30, value: Paint::Role(Role::PrimaryForeground, 1.0) },
            ],
        },
        border_color: Prop {
            base: Some(Paint::Role(Role::Input, 1.0)),
            rules: vec![
                Rule { when: when(focus), rank: 1 << 19, value: Paint::Role(Role::Ring, 1.0) },
                Rule { when: when(invalid), rank: 1 << 28, value: Paint::Role(Role::Destructive, 1.0) },
                Rule { when: when(invalid | checked), rank: (1 << 26) | (1 << 28), value: Paint::Role(Role::Primary, 1.0) },
                Rule { when: when(checked), rank: 1 << 30, value: Paint::Role(Role::Primary, 1.0) },
                Rule { when: when(dark | invalid), rank: (1 << 28) | (1 << 50), value: Paint::Role(Role::Destructive, 0.5) },
            ],
        },
        ring_width: Prop {
            base: None,
            rules: vec![
                Rule { when: when(focus), rank: 1 << 19, value: 3.0 },
                Rule { when: when(invalid), rank: 1 << 28, value: 3.0 },
            ],
        },
        ring_color: Prop {
            base: None,
            rules: vec![
                Rule { when: when(focus), rank: 1 << 19, value: Paint::Role(Role::Ring, 0.5) },
                Rule { when: when(invalid), rank: 1 << 28, value: Paint::Role(Role::Destructive, 0.2) },
                Rule { when: when(dark | invalid), rank: (1 << 28) | (1 << 50), value: Paint::Role(Role::Destructive, 0.4) },
            ],
        },
        ..Recipe::default()
    }
}

fn radio_group_item(theme: &Theme) -> Recipe {
    match theme.style.name {
        "nova" => radio_group_item_nova(),
        other => {
            log::warn!("radio group item: no recipe for style {other}, using nova");
            radio_group_item_nova()
        }
    }
}
```

The border width is not in the recipe: `border` is a base class of
`radio-group-item.svelte`, so the widget code quotes it
(`// base: "border"`) next to the `1.0` it draws, like `disabled:opacity-50`
and the larger hit area `after:-inset-x-3 after:-inset-y-2`.

## Where the code goes

One file per component, for example `src/ui/components/radio_group.rs`, with
its recipes in the same file. The `// base:` and `// style:` quotes stay in
the code.

## Checks

- The literal scan from the `build-ui` skill (Acceptance bar) over the
  component file, with its positive control. Recipe values under a
  `// style:` quote and values under a `// base:` quote are the only numbers
  allowed.
- A snapshot test per style the app uses, in dark and light, opened and
  compared with the shadcn-svelte docs page.
- The behaviour list from the bits-ui primitive, each item checked by a test
  or a stated reason.

## Moving it upstream

When the component works, offer to open a pull request to
`oetiker/shadcn-egui`, which needs the user's approval and a fork or push
access. Upstream the app-local recipes become generated `Style` fields, and
the port follows `upstream.md`.
