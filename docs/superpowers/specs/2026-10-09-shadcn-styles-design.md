# egui-shadcn 0.4.0: shadcn styles as a parameter

Date: 2026-10-09
Status: design, revised after pushback review (2026-10-09), awaiting approval

## 1. Goal

egui-shadcn exists to port shadcn components to egui faithfully, not to
invent look-alikes. Today it ports one frozen variant (new-york v4) and bakes
every visual decision into each component's paint code. shadcn itself is
parametrised: a project's `components.json` picks a `style`, a `baseColor` and
theme, a radius, an icon library and menu variants, and the registry hands out
each component in that combination.

0.4.0 makes egui-shadcn parametrised the same way, so that an egui client can
render with exactly the style its web sibling was set up with, and so that a
missing component is ported from upstream source instead of re-imagined.

Success:

- `Theme::new(Style::nova(), Palette::…, radius)` renders every component the
  way shadcn-svelte's nova renders it, part for part, at the pinned upstream
  commit; the same for `Style::vega()`.
- Updating to a newer shadcn is: bump the pin, re-run the generator, review the
  diff of the generated recipes and the snapshot PNGs.
- A consumer whose web client uses nova can drop local "nova-ish" edits to
  its vendored copy.

## 2. How shadcn styles work (research summary)

Evidence and per-component tables: the research notes in §12.

- A component's source exists **once** (shadcn-svelte
  `docs/src/lib/registry/ui/<name>/`). Its class strings contain the classes
  every style shares plus **marker classes** naming a part, for example
  `cn-slider-thumb`, `cn-button-size-default`.
- Each style is one flat CSS file, `style-<s>.css`, of roughly 420 rules
  `.cn-<part> { @apply <tailwind classes>; }`. Styles: vega, nova, maia, lyra,
  mira, luma, sera, rhea. shadcn-svelte copies these byte for byte from
  shadcn/ui `apps/v4/registry/styles/`.
- At registry build time each marker is replaced by its style's classes,
  placed *before* the base classes; tailwind-merge resolves conflicts, so a
  base class wins over a style class for the same property.
- new-york v4 is no longer a style. Upstream keeps it as a hand-written legacy
  registry without markers. vega keeps new-york's sizes on the new base.
- All other `components.json` keys change tokens or assets only: `baseColor`
  and `theme` the palette, `radius` the `--radius` value with a multiplying
  scale (sm ×0.6, md ×0.8, lg ×1, xl ×1.4, 2xl ×1.8, 3xl ×2.2, 4xl ×2.6),
  `menuAccent=bold` accent := primary, `menuColor` a scoped palette swap and
  translucency on menu popups, `iconLibrary` and `font`/`fontHeading` assets.
- About half of the new-york → nova differences come from the **new base**
  shared by all current styles (tinted destructive, muted instead of accent
  hovers, `ring-1 ring-foreground/10` instead of `border` on surfaces,
  `bg-popover` dialog, light blurred overlay, press nudge, enlarged hit areas
  on checkbox/switch/slider, dialog close as a ghost icon button). egui-shadcn
  ports the old base, so its components must be re-ported regardless of
  styles.

## 3. Decisions taken

| Decision | Choice |
|---|---|
| How styles get into the crate | Generated from upstream `style-<s>.css` by a tool, not hand-copied, not interpreted at runtime |
| How base components get into the crate | Ported by the AI following the skill's porting rules (§8); conversion is judgement work, not mechanical |
| Consumer deviations from a style | Plain public `Style` struct, adjusted in the consumer's own theme setup; vendored files stay untouched |
| new-york | Dropped. Default style becomes **vega** |
| Upstream source | One pinned shadcn-svelte commit for both base components and style CSS |
| Runtime dependencies | None added |

## 4. Architecture

```
Theme = Style × Palette × Radius
          │        │         └─ --radius + the multiplying scale
          │        └─ baseColor/theme (unchanged concept, + menuAccent)
          └─ generated: one Recipe per cn-part
components/*.rs  ── read Theme::current(ctx).style.<part>
                 ── contain base-only geometry, never style values
```

### 4.1 Base components

Every component in `src/components/` is re-ported from the shadcn-svelte base
source at the pin. The port keeps upstream's structure and names its drawable
parts by their marker (`slider_track`, `slider_range`, `slider_thumb`,
`button_variant_outline`, `button_size_default`, …). Classes present in the
base source (shared by all styles) are ported into the component directly,
with a `// base:` comment quoting the upstream class string verbatim. Every
value that comes from a marker is read from the style. Where a base class
overrides a style property (tailwind-merge: base classes come after the
style's and win), the component ignores that recipe field and says so in the
`// base:` comment. Porting is done by the AI under the rules in §8.

### 4.2 Style and recipes

`Style` is a generated struct with one field per marker the crate uses. Each
field is a typed recipe; the recipe types are hand-written (`src/style/
recipe.rs`), the values generated (`src/style/<s>.rs`). A recipe holds only
what the translator can express:

- sizes: height, width, size, padding per side, gap, icon size, text size,
  line height, tracking, font weight, uppercase;
- radius as a rung reference (`none`, `sm` … `4xl`, `full`, `min(rung, px)`),
  resolved against the theme radius at paint time;
- colours per part and state as a role reference plus alpha
  (`Primary`, `Destructive@0.10`, `Foreground@0.10`, `White`), each with an
  optional dark-mode value from `dark:` classes;
- outline kind: none, `Border(role)`, `Ring(width, role)`; focus ring width
  and role; shadow level (none, xs, sm, md, lg, xl);
- states: default, `hover`, `active`, `focus-visible`, `disabled`,
  `aria-invalid`, checked/unchecked (`data-checked`/`data-unchecked`),
  `aria-expanded`;
- context conditions, translated like states because they live in the style
  CSS and differ per style: `has-data-[icon=inline-start|inline-end]:`
  (icon present), `in-data-[slot=…]:` (inside a button group, card, …),
  `group-data-[size=…]/…:` (parent size). The component passes its context
  (icon side, enclosing slot, parent size); the recipe answers;
- behaviour flags: press nudge (`active:translate-y-px`), hit-area inflation
  (`after:-inset-*`), backdrop blur.

A recipe holds exactly what the style CSS says for that marker; an empty field
means the style does not set it, and the component uses its ported base value.

`Style` and every recipe are plain structs with public fields. A consumer that
must deviate from a style in one place (for example a flat focus ring for a renderer
without anti-aliasing) adjusts the value where it builds its theme:
`let mut s = Style::nova(); s.<part>.<field> = …;`. Vendored files are never
edited for this; re-vendoring stays free.

### 4.3 Theme

```rust
pub struct Theme { pub style: Style, pub palette: Palette, pub radius: f32, pub dark: bool }
Theme::new(style, palette, radius) // dark follows palette
Theme::vega_dark(), Theme::vega_light()   // replace Theme::dark()/light()
```

`Metrics` loses every value that now lives in a recipe (control heights,
paddings, ring width, …); what remains is genuinely global (base text sizes,
icon default, hairline). The radius ladder switches from additive (-4/-2/0/+4)
to upstream's multiplying scale. `menuAccent` becomes a palette constructor
option (`Palette::with_bold_menu_accent()`). `Theme::apply` and
`Theme::current` keep their contract; `current` keeps its dark fallback.

### 4.4 What stays in component code

The research found shadcn behaviour that is not a per-part value. These stay
as component code paths, each commented with the upstream classes it mirrors:

- knowing the context (icon side, enclosing slot, parent size) and passing it
  to the recipe; the values themselves come from the style (§4.2);
- `color-mix(in oklch, …)` hovers: an OKLCH mix helper in `theme.rs`;
- structural parts: card and dialog footer strips that reach the panel edge,
  dialog close rendered as a ghost icon `Button`;
- shape variants that some styles need (pill switch thumb, underline-only
  input, plain-text badge) are **not** implemented in 0.4.0; see §6.

Out of scope: breakpoints (`sm:`, `md:`), where the component takes the
non-prefixed value and documents it; animations and transitions;
`menuColor` (scoped palette swap), which comes later.

## 5. The generator: `tools/stylegen`

A separate cargo workspace like `tools/icongen`, never a dependency of the
crate.

Input: a shadcn-svelte checkout at the pinned commit (path argument; the pin
is recorded in `tools/stylegen/PIN`). It reads only
`docs/src/lib/registry/styles/style-<s>.css`; it never parses Svelte. The
base components are the AI's job (§4.1, §8).

Steps:

1. Parse `style-<s>.css` into `marker → [classes]` (upstream's own
   `parseStyleCss` is the reference).
2. Within one marker's class list, resolve conflicts per property with
   "later wins", as tailwind-merge does.
3. Translate each class through a table of supported Tailwind utilities into
   recipe fields (Tailwind's default spacing scale, `px`, arbitrary
   `[…px]`/`[…rem]` values, role colours with `/alpha`, variant prefixes for
   states, context conditions and `dark:`).
4. Emit `src/style/<s>.rs` (rustfmt-clean) and
   `tools/stylegen/reports/<s>.md`: every class it did not translate, per
   marker, with the reason. Nothing untranslated is guessed.
5. Drift check: every `// base:` quote in `src/components/` must still occur
   verbatim in the pinned upstream source. stylegen lists each component whose
   quoted base changed, or whose markers appeared or disappeared upstream;
   each listed component is a re-port job for the AI under §8.

Bumping upstream: update `PIN`, re-run, re-port the listed components, commit
generated files, reports and re-baselined snapshots together; the diff is the
review.

## 6. Scope of 0.4.0

In:

- `tools/stylegen`, recipe types, generated `vega` and `nova`.
- All 13 existing components re-ported to the new base, plus **Slider**
  (new). Layout helpers keep working; the card helper in `layout.rs` reads
  the card recipes.
- The other six styles are generated and their reports committed, but a style
  is **exported** only when its report lists no untranslated class for any
  component the crate ships. Styles needing shape variants (luma, rhea, sera,
  maia, likely) stay unexported until those variants exist.
- Reference settings screen and example take a style argument.

Not in: `menuColor`, fonts per style (the crate keeps Oxanium; a heading-font
slot is noted for later), Svelte or React output, runtime Tailwind.

## 7. Tests

- Snapshots: every component × {vega, nova} × {dark, light}. Old new-york
  baselines are removed.
- `tests/tokens.rs` extended: besides colour constructors and font-size
  literals it rejects literal geometry, shadow definitions and alpha
  multipliers in `src/components/` unless marked `// base:` with the upstream
  class quoted. Positive control kept.
- New test: `src/` and `skills/egui-shadcn/registry/src/` byte-identical.
- stylegen's translator unit tests (class → recipe field), and its drift
  check (§5 step 5) clean at the pin.
- Every custom-painted widget still reports `widget_info`.

## 8. Skill and documentation

The skill's purpose is to give the AI strict rules so that every port comes
out right and fits the egui design system. Part B of `SKILL.md` is rewritten
as this per-component checklist; a port is done only when every item holds.

1. **Source.** Fetch the component's base `.svelte` source and its bits-ui
   primitive at the pin (`tools/stylegen/PIN`). Never port from memory, a
   screenshot, or another style's resolved output.
2. **Inventory.** List every `cn-*` marker the source uses and every data
   attribute or state it styles on (`data-checked`, `aria-invalid`,
   `data-[icon=…]`, `data-orientation`, …). Add any marker stylegen does not
   yet emit, re-run stylegen.
3. **Mapping.** Map each marker to a recipe field, each state and context
   condition to the recipe's state/context input, and quote each base class
   string in a `// base:` comment. A base class that overrides a style
   property is noted in that comment and its recipe field is not read.
4. **No style values in code.** Sizes, colours, radii, shadows, ring widths,
   alphas come from the recipe or, for base classes, are quoted under
   `// base:`. The extended token test enforces this.
5. **Behaviour, not just looks.** Port the primitive's behaviour: keyboard
   handling, focus order, pointer capture while dragging, value stepping,
   disabled handling, and the accessibility role via `widget_info`. List
   what bits-ui does and check each item in egui.
6. **egui sizing.** The widget sets its own size before placement (single
   pass); use the layout helpers, no `available_width()` arithmetic.
7. **Context.** The widget's API takes whatever context the recipe's context
   conditions need (icon side, enclosing slot, size variant).
8. **Snapshots.** One per style × {dark, light}, reviewed side by side with
   the same component on the shadcn-svelte docs page for that style.
9. **Docs.** A row in `references/component-map.md` with the markers.
10. **Done** means: stylegen's report and drift check are clean for the
    component, snapshots reviewed, the behaviour list checked, clippy and
    tests clean, registry copy identical.

Plus:

- A rule for consumers: project-specific deviations go into the theme
  construction (§4.2), never into vendored files.
- New reference page `references/styles.md`: the style model, the recipe
  fields and context conditions, the generator and its drift check, choosing
  the style that matches a web project's `components.json`, bumping the pin.
- `references/component-map.md` gains the marker names per component.
- Fixes found during research: `registry/README.md` no longer asks for eframe
  and image loaders; the "hover is opacity only, never a new hue" rule is
  replaced by "hover follows the style's recipe".
- `CHANGES.md` 0.4.0 entry spells out the breaking changes: new-york gone,
  `Theme::dark()/light()` replaced, most `Metrics` fields (introduced in
  0.3.0) moved into recipes, radius scale changed, vega as default.

## 9. Consumer migration

Not part of this release; recorded so the design serves it. A project with a
shadcn-svelte web client and an egui client:

- picks `Theme::new(Style::<same as components.json>(), Palette::…, radius)`,
  adjusting individual recipe values there where it must deviate;
- re-vendors the registry copy; local edits that only approximated its web
  style (fills, hover colours, switch colours) are dropped, brand palette
  constructors and renderer-specific tweaks stay as its own edits;
- updates its web components to the same shadcn-svelte pin, since installed
  components may be an older snapshot of the style;
- checks its CSS radius scale: projects set up with an older CLI define only
  the additive `sm`/`md`/`lg` steps, so `rounded-xl`…`4xl` fall back to
  Tailwind's fixed sizes there; the current multiplying scale is what
  egui-shadcn reproduces.

## 10. Risks

- **Translator coverage.** The class table may need many utilities. Mitigated
  by the per-style report: unsupported classes are visible, and a style ships
  only when clean for the shipped components.
- **Port quality varies by session.** Base components are ported by the AI.
  Mitigated by the §8 checklist, the token test, the drift check and
  per-style snapshots reviewed against the web rendering.
- **Visual change for 0.3 users.** vega is close to new-york but not equal
  (new base). Called out in `CHANGES.md`; there is no compatibility style.
- **Size of the change.** All components are touched; the parts cannot ship
  separately (no style without generator and re-ported components). The
  implementation plan lands stylegen, the recipe types and the new Part B
  first, then ports components one at a time against their snapshots.

## 11. Open points for the plan

- Exact list of markers per component (from the base sources at the pin).
- Whether `Palette` constructors for shadcn's base colours (zinc, neutral, …)
  are generated too, from `registry/colors/<name>.json`. Not required for
  0.4.0; recommended as a follow-up since the generator infrastructure exists.

## 12. References

- Research notes: `docs/superpowers/research/2026-10-09-shadcn-styles.md`
  (style mechanism, URLs, per-component new-york v4 vs nova tables, what the
  other styles change, what is not a per-part value).
- Upstream: `huntabyte/shadcn-svelte` @ `493481fa` (2026-10-05),
  `shadcn-ui/ui` `apps/v4/registry/styles/` @ `2d3f1cd4` (2026-10-09).
