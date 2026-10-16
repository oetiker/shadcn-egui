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
  `opacity`, `ui_opacity` and `style_painter` for a part's fade, `faded` for a container
  part's content, `line_box` for the height of a line of text).
- `shadcn_egui::components::icon::paint_icon` and the theme's icon slots.

## Recipes for new markers

`Style` holds only the markers of ported components, so a new component's
markers (`cn-radio-group`, `cn-radio-group-item`, …) have no field. The app
builds the recipes itself:

- One `Recipe` per marker for each style the app uses, with the values read
  from that style's `style-<name>.css` in shadcn-svelte at the pinned commit
  (`SKILL.md`, step 1). Never take the values from another style.
- The recipe holds the part's whole class string, as a generated one does:
  the style's classes from the CSS and the component's own (base) classes
  from its `.svelte` file, the base classes after the style's. A
  `// style: "<css class string>"` quote and a `// base: "<class string>"`
  quote above each recipe, copied verbatim, record where the values come
  from, so a later port into the crate can check the recipe against
  upstream.
- A rule's `rank` orders it the way the browser orders the CSS: by
  specificity, then by Tailwind's variant order. Take the bit of each
  variant from a generated recipe in the crate that uses the same variant
  (`crates/shadcn-egui/src/style/nova.rs` of the crate version the app
  depends on: the bits move when the generator learns a new variant); a
  rule with several variants ORs their bits. Then add the specificity the
  variants add: `n << 123` for n classes, attributes and pseudo-classes
  (one for each variant below), `n << 121` for n pseudo-elements
  (`placeholder:`); media variants (`sm:`, `md:`) and `in-*:` add none.
  The bits used below: `focus-visible:` `1 << 20`, `disabled:` `1 << 22`,
  `aria-checked:` `1 << 27`, `aria-invalid:` `1 << 29`, `data-checked:`
  `1 << 31`, `dark:` `1 << 52`; so `dark:aria-invalid:` is
  `(1 << 29) | (1 << 52) | (2 << 123)`.
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

// style: "border-input dark:bg-input/30 data-checked:bg-primary data-checked:text-primary-foreground dark:data-checked:bg-primary data-checked:border-primary aria-invalid:aria-checked:border-primary aria-invalid:border-destructive focus-visible:border-ring focus-visible:ring-ring/50 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 dark:aria-invalid:border-destructive/50 flex size-4 rounded-full focus-visible:ring-3 aria-invalid:ring-3 group-has-[:focus-visible]/field-label:ring-0 group-has-[:focus-visible]/field-label:not-data-checked:border-input group-has-[:focus-visible]/field-label:data-checked:border-primary"
// base: "group/radio-group-item peer relative aspect-square shrink-0 border outline-none after:absolute after:-inset-x-3 after:-inset-y-2 disabled:cursor-not-allowed disabled:opacity-50"
fn radio_group_item_nova() -> Recipe {
    let (dark, focus, invalid) = (State::DARK, State::FOCUS_VISIBLE, State::INVALID);
    // data-checked: and aria-checked: are one flag with different rank bits.
    let checked = State::CHECKED;
    let when = When::all;
    Recipe {
        width: Prop::fixed(16.0),
        height: Prop::fixed(16.0),
        radius: Prop::fixed(Radius::Rung(Rung::Full)),
        bg: Prop {
            base: None,
            rules: vec![
                Rule { when: when(checked), rank: (1 << 31) | (1 << 123), value: Paint::Role(Role::Primary, 1.0) },
                Rule { when: when(dark), rank: (1 << 52) | (1 << 123), value: Paint::Role(Role::Input, 0.3) },
                Rule { when: when(dark | checked), rank: (1 << 31) | (1 << 52) | (2 << 123), value: Paint::Role(Role::Primary, 1.0) },
            ],
        },
        text: Prop {
            base: None,
            rules: vec![
                Rule { when: when(checked), rank: (1 << 31) | (1 << 123), value: Paint::Role(Role::PrimaryForeground, 1.0) },
            ],
        },
        // base: "border"
        border_width: Prop::fixed(1.0),
        border_color: Prop {
            base: Some(Paint::Role(Role::Input, 1.0)),
            rules: vec![
                Rule { when: when(focus), rank: (1 << 20) | (1 << 123), value: Paint::Role(Role::Ring, 1.0) },
                Rule { when: when(invalid), rank: (1 << 29) | (1 << 123), value: Paint::Role(Role::Destructive, 1.0) },
                Rule { when: when(invalid | checked), rank: (1 << 27) | (1 << 29) | (2 << 123), value: Paint::Role(Role::Primary, 1.0) },
                Rule { when: when(checked), rank: (1 << 31) | (1 << 123), value: Paint::Role(Role::Primary, 1.0) },
                Rule { when: when(dark | invalid), rank: (1 << 29) | (1 << 52) | (2 << 123), value: Paint::Role(Role::Destructive, 0.5) },
            ],
        },
        ring_width: Prop {
            base: None,
            rules: vec![
                Rule { when: when(focus), rank: (1 << 20) | (1 << 123), value: 3.0 },
                Rule { when: when(invalid), rank: (1 << 29) | (1 << 123), value: 3.0 },
            ],
        },
        ring_color: Prop {
            base: None,
            rules: vec![
                Rule { when: when(focus), rank: (1 << 20) | (1 << 123), value: Paint::Role(Role::Ring, 0.5) },
                Rule { when: when(invalid), rank: (1 << 29) | (1 << 123), value: Paint::Role(Role::Destructive, 0.2) },
                Rule { when: when(dark | invalid), rank: (1 << 29) | (1 << 52) | (2 << 123), value: Paint::Role(Role::Destructive, 0.4) },
            ],
        },
        // base: "outline-none"
        outline_width: Prop::fixed(0.0),
        // base: "disabled:opacity-50"
        opacity: Prop {
            base: None,
            rules: vec![Rule { when: when(State::DISABLED), rank: (1 << 22) | (1 << 123), value: 0.5 }],
        },
        // base: "after:-inset-x-3 after:-inset-y-2"
        hit_inset_x: Prop::fixed(12.0),
        hit_inset_y: Prop::fixed(8.0),
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

The base classes are the same in every style, so each style's recipe
carries them; where a style's CSS sets the same property under the same
condition (luma's class string sets `border` too), the base class comes
later in the string and wins. The widget code then resolves every value
with `pick` over the recipe and holds no number of its own.

`log` is not a dependency of the app by default. Add `log = "0.4"` (the
crate already depends on it, so the build gains no new crate), or report
the fallback through the app's own logging.

## Where the code goes

One file per component, for example `src/ui/components/radio_group.rs`, with
its recipes in the same file. The `// style:` and `// base:` quotes stay
above the recipes as the record of where each value comes from; the
drawing code carries no quotes and no style values.

## Checks

- The literal scan from the `build-ui` skill (Acceptance bar) over the
  component file, with its positive control. Numbers appear only in the
  recipe functions, under their `// style:` and `// base:` quotes; a
  behaviour constant in the drawing code carries
  `// behaviour-ok: <reason>`, as in the crate (`upstream.md`, Tokens).
- A snapshot test per style the app uses, in dark and light, opened and
  compared with the shadcn-svelte docs page.
- The behaviour list from the bits-ui primitive, each item checked by a test
  or a stated reason.

## Moving it upstream

When the component works, offer to open a pull request to
`oetiker/shadcn-egui`, which needs the user's approval and a fork or push
access. Upstream the app-local recipes become generated `Style` fields, and
the port follows `upstream.md`.
