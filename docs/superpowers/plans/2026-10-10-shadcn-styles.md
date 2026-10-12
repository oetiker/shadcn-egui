# egui-shadcn 0.4.0 implementation plan: shadcn styles as a parameter

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `Theme = Style × Palette × radius`, with the styles generated
from upstream shadcn `style-<s>.css` by a new tool, and every component
re-ported to the shadcn-svelte base at the pin (plus a new Slider), so that
`Theme::new(Style::nova(), …)` renders like shadcn-svelte's nova.

**Architecture:** Hand-written recipe types (`src/style/recipe.rs`) describe
what a style can say about one `cn-*` part: lengths, colours, radii, shadows,
behaviour flags, each as a `Prop<T>` with a base value plus conditional rules
(state and context flags, Tailwind variant rank). `tools/stylegen` (its own
cargo workspace) parses `style-<s>.css`, translates each class through a
table, and emits `src/style/style.rs` (the `Style` struct, one `Recipe` per
marker) and `src/style/<s>.rs` (the values), plus a report per style. Each
component reads its stacked recipes (`cn-button`, then variant, then size)
through `pick()`; base classes that every style shares live in the component
code under a `// base: "…"` quote that stylegen's drift check verifies
against upstream.

**Tech Stack:** Rust 2021, egui 0.34, egui_extras 0.34, egui_kittest 0.34
(wgpu snapshots, dev only), eframe 0.34 (example, dev only). stylegen: std
only, `serde_json` as a dev-dependency for the fixture test.

**Spec:** `docs/superpowers/specs/2026-10-09-shadcn-styles-design.md`
(research: `docs/superpowers/research/2026-10-09-shadcn-styles.md`). Read
both before Task 1.

## Global Constraints

- `src/` and `skills/egui-shadcn/registry/src/` stay byte-identical. Every
  task that touches `src/` ends with `cp -r src/. skills/egui-shadcn/registry/src/`
  (and a `git rm` in the registry for files it deletes from `src/`).
- Runtime `[dependencies]` stay exactly `egui = "0.34"` and
  `egui_extras = { version = "0.34", default-features = false }`. No new
  runtime dependency.
- Upstream pin: shadcn-svelte `493481fa` (2026-10-05). Its style CSS files are
  byte copies of shadcn/ui `apps/v4/registry/styles/` (shadcn/ui `2d3f1cd4`).
  Tailwind at that pin: `tailwindcss@4.3.3`.
- new-york is dropped. The default style is **vega**. 0.4.0 exports **vega**
  and **nova**; the other six are generated (reports committed) and exported
  only when their report has no *unsupported* class for any shipped marker.
- `Style` and every `Recipe` are plain structs with public fields. Consumer
  deviations go into theme construction, never into vendored files.
- egui is single-pass: a widget sets its own size before placement; use the
  `layout` helpers, no `available_width()` arithmetic.
- Every custom-painted widget reports `widget_info`.
- Build parallelism: never more than 4 cores. Use `--jobs 4` on every cargo
  command and `-- --test-threads 4` on `cargo test`.
- `cargo clippy --all-targets --jobs 4 -- -D warnings` clean for the crate and
  for `--manifest-path tools/stylegen/Cargo.toml`.
- Pushing `main` publishes the plugin. Never push or merge without asking the
  user.
- English for all code, comments and docs. CHANGES.md entries follow the
  user's rules: reader is a user, lead with what they see, ≤3 sentences, no
  release header.

## Two decisions this plan makes that refine the spec

The user must confirm both at plan review.

1. **Breakpoints (spec §4.4 says "take the non-prefixed value").** egui
   clients are desktop windows, and the web sibling the spec wants to match
   renders at desktop width, where `sm:` (≥640px) and `md:` (≥768px) apply.
   Taking the non-prefixed value would give 16px input text
   (`text-base md:text-sm`) and a viewport-relative dialog width
   (`max-w-[calc(100%-2rem)] sm:max-w-sm`). This plan treats `sm:` and `md:`
   as always matched (rank kept, so they still beat the plain class).
   Larger breakpoints do not occur in the shipped markers; if one appears it
   is reported as unsupported.
2. **The report separates "ignored" from "unsupported" (spec §5 step 4,
   §6).** Classes egui has no counterpart for by design (animations,
   transitions, flex/grid layout keywords, cursor, z-index, `[a]:` link
   badges, kbd/field/img selectors for components the crate does not ship)
   are listed under *Ignored, with reason* from an explicit allow-list in
   the translator. Everything else that does not translate is *Unsupported*.
   The export gate counts only *Unsupported*. Nothing is guessed: a class not
   on either list is Unsupported.

## Review Focus

1. **Slider with a degenerate range** (`min == max`, `step == 0`, value
   outside the range, NaN): a person expects a drawn, inert slider, never a
   panic, NaN position or endless loop. Pinned by tests in Task 16.
2. **Disabled widgets under the pointer:** a disabled button, switch,
   checkbox, slider or input must not show hover, active (press nudge) or
   focus styles, and must not change value. Pinned in Tasks 10, 14, 15, 16.
3. **Unusual theme radius:** `radius: 0.0` gives square corners everywhere;
   a large radius (for example 40.0) with `rounded-4xl` must clamp to half the
   short side and never overflow egui's `u8` corner radius. Pinned in Task 7.
4. **`Theme::current` before `Theme::apply`:** a widget painted before any
   theme is applied falls back to vega dark without building a new `Style`
   on every call. Pinned in Task 7.
5. **Consumer overrides:** `s.button_size_default.height.base = Some(40.0)`
   changes the rendered height, and `Theme::new` with a custom brand palette
   detects dark/light from its background (a consumer may also set
   `theme.dark` explicitly). Pinned in Tasks 7 and 10.

---

## Setup (before Task 1)

Work in a worktree on a new branch, not on `spec/shadcn-styles`:

```bash
cd <checkout>
git worktree add -b feat/shadcn-styles <scratch>/worktrees/egui-shadcn-styles spec/shadcn-styles
cd <scratch>/worktrees/egui-shadcn-styles
```

Upstream checkout at the pin (read-only input for stylegen and for porting):

```bash
UP=<scratch>/shadcn-svelte-493481fa
git clone --filter=blob:none https://github.com/huntabyte/shadcn-svelte.git "$UP"
git -C "$UP" checkout 493481fa
```

If the clone is impossible, the research clone at
`<scratch>/egui-shadcn-styles-research/dl/ss` is at the same
commit (`git -C … log -1` shows `493481f`). Below, `$UP` is that directory.

All paths below are relative to the worktree root.

---

## File structure

| Path | Responsibility | Task |
|---|---|---|
| `src/color.rs` | add sRGB→OKLCH and OKLCH mix | 1 |
| `src/style/recipe.rs` | hand-written recipe types, `State`, `When`, `Prop`, `pick` | 2 |
| `src/style/mod.rs` | hand-written: module wiring, `Style::vega()/nova()`, `Default` | 2, 5 |
| `src/style/style.rs` | **generated**: `pub struct Style` (one `Recipe` per marker) | 5 |
| `src/style/vega.rs`, `src/style/nova.rs` | **generated**: style values | 5 |
| `tools/stylegen/` | generator workspace: `PIN`, `EXPORT`, `markers.txt`, `src/{css,class,model,variant,utility,resolve,emit,report,drift,lib,main}.rs`, `tests/`, `reports/` | 3–6 |
| `src/theme.rs` | `Theme { style, palette, radius, metrics, dark }`, resolution helpers, multiplying radius scale, shadow table | 7, 24 |
| `src/components/shared.rs` | `ring()` and `paint_box()` painting helpers on resolved values | 7 |
| `tests/common/mod.rs` | `themes()`, `snapshot_all()` | 8 |
| `tests/registry.rs` | `src/` == registry copy | 8 |
| `tests/tokens.rs` | extended literal scan with `// base:` quotes | 8, 24 |
| `src/components/*.rs` | one re-port per task | 10–22 |
| `src/components/slider.rs` | new | 16 |
| `src/layout.rs` | `card()` reads card recipes | 21 |
| `src/reference.rs`, `examples/settings.rs` | take a style argument | 23 |
| `skills/egui-shadcn/SKILL.md`, `references/*.md`, `registry/README.md` | rules and docs | 9, 25 |
| `CHANGES.md` | 0.4.0 entry (no release header) | 25 |

---

### Task 1: OKLCH conversion and mix in `color.rs`

shadcn's `hover:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]`
needs a mix of two palette colours in OKLCH. `color.rs` has OKLCH→sRGB only.

**Files:**
- Modify: `src/color.rs`
- Test: `tests/color.rs`

**Interfaces:**
- Produces: `pub fn srgb_to_oklch(c: Color32) -> (f32, f32, f32)` (L 0..1, C, H degrees);
  `pub fn mix_oklch(a: Color32, b: Color32, t: f32) -> Color32` (t = share of `b`, alpha mixed linearly).

- [ ] **Step 1: Read `src/color.rs`** to reuse its sRGB transfer and OKLab matrices (keep one copy of each constant).

- [ ] **Step 2: Write the failing tests** (append to `tests/color.rs`):

```rust
use egui_shadcn::color::{mix_oklch, oklch_to_srgb, srgb_to_oklch};

#[test]
fn oklch_round_trip() {
    for (l, c, h) in [(0.205, 0.0, 0.0), (0.577, 0.245, 27.325), (0.985, 0.0, 0.0), (0.704, 0.191, 22.216)] {
        let rgb = oklch_to_srgb(l, c, h);
        let (l2, c2, h2) = srgb_to_oklch(rgb);
        assert!((l - l2).abs() < 0.01, "L {l} -> {l2}");
        assert!((c - c2).abs() < 0.01, "C {c} -> {c2}");
        if c > 0.02 {
            assert!((h - h2).abs() < 1.5, "H {h} -> {h2}");
        }
    }
}

#[test]
fn mix_endpoints_and_middle() {
    let a = oklch_to_srgb(0.97, 0.0, 0.0);
    let b = oklch_to_srgb(0.145, 0.0, 0.0);
    assert_eq!(mix_oklch(a, b, 0.0), a);
    assert_eq!(mix_oklch(a, b, 1.0), b);
    let (l, _, _) = srgb_to_oklch(mix_oklch(a, b, 0.05));
    // 95% of 0.97 + 5% of 0.145
    assert!((l - (0.97 * 0.95 + 0.145 * 0.05)).abs() < 0.01, "got {l}");
}

#[test]
fn mix_keeps_alpha_linear() {
    let a = egui::Color32::from_rgba_unmultiplied(255, 0, 0, 255);
    let b = egui::Color32::from_rgba_unmultiplied(0, 0, 255, 0);
    assert_eq!(mix_oklch(a, b, 0.5).a(), 128);
}
```

- [ ] **Step 3: Run, expect a compile failure** (`srgb_to_oklch` missing):
  `cargo test --jobs 4 --test color -- --test-threads 4`

- [ ] **Step 4: Implement.** In `src/color.rs`:

```rust
/// sRGB (unmultiplied, alpha ignored) to OKLCH: L in 0..1, C, H in degrees.
pub fn srgb_to_oklch(c: Color32) -> (f32, f32, f32) {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    let lin = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    let (r, g, b) = (lin(r), lin(g), lin(b));
    let l = 0.412_221_47 * r + 0.536_332_55 * g + 0.051_445_99 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;
    let (l, m, s) = (l.cbrt(), m.cbrt(), s.cbrt());
    let ok_l = 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s;
    let ok_a = 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s;
    let ok_b = 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s;
    let chroma = (ok_a * ok_a + ok_b * ok_b).sqrt();
    let hue = ok_b.atan2(ok_a).to_degrees().rem_euclid(360.0);
    (ok_l, chroma, hue)
}

/// CSS `color-mix(in oklch, a, b t)`: `t` is the share of `b`. Hue takes the
/// shorter arc; an achromatic side adopts the other side's hue.
pub fn mix_oklch(a: Color32, b: Color32, t: f32) -> Color32 {
    if t <= 0.0 { return a; }
    if t >= 1.0 { return b; }
    let (l1, c1, mut h1) = srgb_to_oklch(a);
    let (l2, c2, mut h2) = srgb_to_oklch(b);
    if c1 < 1e-4 { h1 = h2; }
    if c2 < 1e-4 { h2 = h1; }
    let mut dh = h2 - h1;
    if dh > 180.0 { dh -= 360.0; } else if dh < -180.0 { dh += 360.0; }
    let alpha = a.a() as f32 / 255.0 * (1.0 - t) + b.a() as f32 / 255.0 * t;
    oklch_to_srgb_a(l1 + (l2 - l1) * t, c1 + (c2 - c1) * t, h1 + dh * t, alpha)
}
```

  If `color.rs` already defines these matrix constants for the forward
  direction, keep its style (it is the source for the numbers' precision).
  The OKLab inverse matrices above are Björn Ottosson's published values.

- [ ] **Step 5: Run the tests, expect PASS.** Then
  `cargo clippy --all-targets --jobs 4 -- -D warnings`.

- [ ] **Step 6: Sync registry and commit.**

```bash
cp -r src/. skills/egui-shadcn/registry/src/
git add src/color.rs skills/egui-shadcn/registry/src/color.rs tests/color.rs
git commit -m "feat(color): sRGB to OKLCH and OKLCH colour mix

shadcn's secondary-button hover is color-mix(in oklch, secondary,
foreground 5%); the style recipes need this to resolve it."
```

---

### Task 2: Recipe types

**Files:**
- Create: `src/style/recipe.rs`, `src/style/mod.rs`
- Modify: `src/lib.rs` (add `pub mod style;`)
- Test: unit tests inside `src/style/recipe.rs`

**Interfaces:**
- Produces (all `pub`, re-exported from `crate::style`):
  - `State(pub u32)` with consts `NONE, DARK, HOVER, FOCUS, FOCUS_VISIBLE, ACTIVE, DISABLED, INVALID, CHECKED, UNCHECKED, EXPANDED, HAS_POPUP, SELECTED, PLACEHOLDER, ICON_START, ICON_END, IN_BUTTON_GROUP, SIZE_SM, SIZE_DEFAULT, VERTICAL, HORIZONTAL, VARIANT_LINE, VARIANT_DEFAULT, HAS_FOOTER`; `const fn or(self, State) -> State`; `const fn with(self, State, bool) -> State`; `const fn contains(self, State) -> bool`; `BitOr`.
  - `When { pub all: State, pub none: State }`, `When::ALWAYS`, `const fn When::new(all, none)`, `fn matches(self, State) -> bool`.
  - `Rule<T> { pub when: When, pub rank: u32, pub value: T }`.
  - `Prop<T> { pub base: Option<T>, pub rules: Vec<Rule<T>> }`, `Prop::fixed(v)`, `Prop::get(&self, State) -> Option<T>`.
  - `pick<T: Copy>(layers: &[&Recipe], field: impl Fn(&Recipe) -> &Prop<T>, s: State) -> Option<T>`.
  - Value types: `Role`, `Paint`, `Rung`, `Radius`, `TextSize`, `Weight`, `ShadowLevel`, `Shift`.
  - `Recipe` with the fields listed in Step 3 (all `Prop<…>`, `Default`).

- [ ] **Step 1: Write the failing tests** at the bottom of `src/style/recipe.rs`
  (create the file with only the test module first):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn rule<T>(all: State, rank: u32, value: T) -> Rule<T> {
        Rule { when: When::new(all, State::NONE), rank, value }
    }

    #[test]
    fn base_applies_without_rules() {
        let p = Prop::fixed(32.0);
        assert_eq!(p.get(State::NONE), Some(32.0));
        assert_eq!(Prop::<f32>::default().get(State::HOVER), None);
    }

    #[test]
    fn matching_rule_beats_base() {
        let p = Prop { base: Some(10.0), rules: vec![rule(State::ICON_START, 1 << 14, 8.0)] };
        assert_eq!(p.get(State::NONE), Some(10.0));
        assert_eq!(p.get(State::ICON_START), Some(8.0));
    }

    #[test]
    fn higher_rank_wins_over_later_rule() {
        // dark:bg-input/30 (rank DARK) and hover:bg-muted (rank HOVER): dark
        // is registered later in Tailwind, so it wins when both match.
        let p = Prop {
            base: None,
            rules: vec![rule(State::DARK, 1 << 20, 1), rule(State::HOVER, 1 << 8, 2)],
        };
        assert_eq!(p.get(State::DARK | State::HOVER), Some(1));
        // dark:hover: has both bits, so it beats dark: alone.
        let mut p2 = p.clone();
        p2.rules.push(rule(State::DARK | State::HOVER, (1 << 20) | (1 << 8), 3));
        assert_eq!(p2.get(State::DARK | State::HOVER), Some(3));
    }

    #[test]
    fn negated_condition() {
        let p = Prop {
            base: None,
            rules: vec![Rule { when: When::new(State::ACTIVE, State::HAS_POPUP), rank: 1, value: true }],
        };
        assert_eq!(p.get(State::ACTIVE), Some(true));
        assert_eq!(p.get(State::ACTIVE | State::HAS_POPUP), None);
    }

    #[test]
    fn later_layer_wins_ties() {
        let button = Recipe { height: Prop::fixed(1.0), ..Recipe::default() };
        let size = Recipe { height: Prop::fixed(32.0), ..Recipe::default() };
        assert_eq!(pick(&[&button, &size], |r| &r.height, State::NONE), Some(32.0));
        // A conditional rule in an earlier layer still beats a later base.
        let button = Recipe {
            height: Prop { base: None, rules: vec![rule(State::SIZE_SM, 1 << 16, 20.0)] },
            ..Recipe::default()
        };
        assert_eq!(pick(&[&button, &size], |r| &r.height, State::SIZE_SM), Some(20.0));
    }

    #[test]
    fn consumer_can_override_base() {
        let mut r = Recipe { height: Prop::fixed(32.0), ..Recipe::default() };
        r.height.base = Some(40.0);
        assert_eq!(r.height.get(State::HOVER), Some(40.0));
    }
}
```

- [ ] **Step 2: Wire the module and run, expect compile failure.**
  `src/style/mod.rs` for now:

```rust
//! shadcn styles: recipe types (hand-written) and generated style values.
//! See `references/styles.md` in the skill.

pub mod recipe;
pub use recipe::*;
```

  `src/lib.rs`: add `pub mod style;` after `pub mod reference;`.
  Run: `cargo test --jobs 4 --lib style:: -- --test-threads 4` → FAIL (types missing).

- [ ] **Step 3: Implement `src/style/recipe.rs`** above the test module:

```rust
//! What one shadcn style says about one component part (a `cn-*` marker).
//! The types are hand-written; the values are generated by `tools/stylegen`
//! from upstream `style-<s>.css`. Every field is a [`Prop`]: an optional base
//! value plus rules that apply under state and context conditions, the way a
//! Tailwind class applies under its variants. An empty `Prop` means the style
//! does not set that property; the component then uses its base value.

use std::ops::BitOr;

/// State and context flags. A component builds one per paint; the theme adds
/// [`State::DARK`]. Each flag stands for the Tailwind variants listed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct State(pub u32);

impl State {
    pub const NONE: State = State(0);
    /// `dark:`
    pub const DARK: State = State(1 << 0);
    /// `hover:`
    pub const HOVER: State = State(1 << 1);
    /// `focus:` (the highlighted item of a list)
    pub const FOCUS: State = State(1 << 2);
    /// `focus-visible:` (keyboard focus)
    pub const FOCUS_VISIBLE: State = State(1 << 3);
    /// `active:` (pressed)
    pub const ACTIVE: State = State(1 << 4);
    /// `disabled:`, `data-disabled:`, `group-data-[disabled=true]:`, `peer-disabled:`
    pub const DISABLED: State = State(1 << 5);
    /// `aria-invalid:`, `data-invalid:`
    pub const INVALID: State = State(1 << 6);
    /// `data-checked:`, `aria-checked:`, `data-[state=checked]:`
    pub const CHECKED: State = State(1 << 7);
    /// `data-unchecked:`, `data-[state=unchecked]:`
    pub const UNCHECKED: State = State(1 << 8);
    /// `aria-expanded:`
    pub const EXPANDED: State = State(1 << 9);
    /// `aria-[haspopup]:` (used negated: `not-aria-[haspopup]:`)
    pub const HAS_POPUP: State = State(1 << 10);
    /// `data-active:`, `data-[active=true]:`, `data-selected:`
    pub const SELECTED: State = State(1 << 11);
    /// `data-placeholder:`
    pub const PLACEHOLDER: State = State(1 << 12);
    /// `has-data-[icon=inline-start]:`
    pub const ICON_START: State = State(1 << 16);
    /// `has-data-[icon=inline-end]:`
    pub const ICON_END: State = State(1 << 17);
    /// `in-data-[slot=button-group]:`
    pub const IN_BUTTON_GROUP: State = State(1 << 18);
    /// `data-[size=sm]:`, `group-data-[size=sm]/…:`
    pub const SIZE_SM: State = State(1 << 19);
    /// `data-[size=default]:`, `group-data-[size=default]/…:`
    pub const SIZE_DEFAULT: State = State(1 << 20);
    /// `data-vertical:`, `data-[orientation=vertical]:`, `group-data-vertical/…:`
    pub const VERTICAL: State = State(1 << 21);
    /// `data-horizontal:`, `data-[orientation=horizontal]:`, `group-data-horizontal/…:`
    pub const HORIZONTAL: State = State(1 << 22);
    /// `data-[variant=line]:`, `group-data-[variant=line]/…:`
    pub const VARIANT_LINE: State = State(1 << 23);
    /// `group-data-[variant=default]/…:`
    pub const VARIANT_DEFAULT: State = State(1 << 24);
    /// `has-data-[slot=card-footer]:`
    pub const HAS_FOOTER: State = State(1 << 25);

    pub const fn or(self, other: State) -> State {
        State(self.0 | other.0)
    }
    pub const fn with(self, flag: State, on: bool) -> State {
        if on { self.or(flag) } else { self }
    }
    pub const fn contains(self, other: State) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for State {
    type Output = State;
    fn bitor(self, rhs: State) -> State {
        self.or(rhs)
    }
}

/// The condition of a rule: every flag in `all` set, no flag in `none` set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct When {
    pub all: State,
    pub none: State,
}

impl When {
    pub const ALWAYS: When = When { all: State::NONE, none: State::NONE };
    pub const fn new(all: State, none: State) -> When {
        When { all, none }
    }
    pub fn matches(self, s: State) -> bool {
        s.contains(self.all) && s.0 & self.none.0 == 0
    }
}

/// A value that applies when `when` matches. `rank` is the Tailwind v4
/// variant-order bitmask of the class's variants: when several rules match,
/// the highest rank wins, as in Tailwind's CSS output order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rule<T> {
    pub when: When,
    pub rank: u32,
    pub value: T,
}

/// One property of a recipe. Set `base` to change the unconditional value;
/// push a [`Rule`] to add a conditional one.
#[derive(Clone, Debug, PartialEq)]
pub struct Prop<T> {
    pub base: Option<T>,
    pub rules: Vec<Rule<T>>,
}

impl<T> Default for Prop<T> {
    fn default() -> Self {
        Prop { base: None, rules: Vec::new() }
    }
}

impl<T: Copy> Prop<T> {
    pub fn fixed(value: T) -> Self {
        Prop { base: Some(value), rules: Vec::new() }
    }
    pub fn get(&self, s: State) -> Option<T> {
        pick_props([self], s)
    }
}

/// Resolve one property across stacked recipes, in the order upstream joins
/// their classes (e.g. `cn-button`, then `cn-button-variant-*`, then
/// `cn-button-size-*`). The matching candidate with the highest rank wins;
/// ties go to the later layer, then the later rule. A base value has rank 0.
pub fn pick<T: Copy>(layers: &[&Recipe], field: impl Fn(&Recipe) -> &Prop<T>, s: State) -> Option<T> {
    pick_props(layers.iter().map(|r| field(r)), s)
}

fn pick_props<'a, T: Copy + 'a>(props: impl IntoIterator<Item = &'a Prop<T>>, s: State) -> Option<T> {
    let mut best: Option<((u32, usize, usize), T)> = None;
    let mut consider = |key: (u32, usize, usize), v: T| {
        if best.as_ref().is_none_or(|(k, _)| key >= *k) {
            best = Some((key, v));
        }
    };
    for (li, p) in props.into_iter().enumerate() {
        if let Some(v) = p.base {
            consider((0, li, 0), v);
        }
        for (ri, r) in p.rules.iter().enumerate() {
            if r.when.matches(s) {
                consider((r.rank, li, ri + 1), r.value);
            }
        }
    }
    best.map(|(_, v)| v)
}

/// A palette colour role (shadcn CSS variable), or a fixed colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Background, Foreground, Card, CardForeground, Popover, PopoverForeground,
    Primary, PrimaryForeground, Secondary, SecondaryForeground, Muted, MutedForeground,
    Accent, AccentForeground, Destructive, DestructiveForeground, Border, Input, Ring,
    White, Black, Transparent,
}

/// A colour as a style states it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    /// `bg-primary/80` → `Role(Primary, 0.8)`.
    Role(Role, f32),
    /// `color-mix(in oklch, a, b t)`: `t` is the share of `b`.
    Mix(Role, Role, f32),
}

/// A step of shadcn's radius scale, multiplied from the theme radius
/// (`sm` ×0.6, `md` ×0.8, `lg` ×1, `xl` ×1.4, `2xl` ×1.8, `3xl` ×2.2, `4xl` ×2.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rung { None, Sm, Md, Lg, Xl, Xl2, Xl3, Xl4, Full }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Radius {
    Rung(Rung),
    Px(f32),
    /// `rounded-[min(var(--radius-md),10px)]`
    Min(Rung, f32),
}

/// `text-xs` … `text-lg`, resolved through `Metrics`; or an arbitrary size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextSize { Xs, Sm, Base, Lg, Px(f32) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight { Normal, Medium, Semibold, Bold }

/// Tailwind v4 `shadow-*` levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowLevel { None, Xs2, Xs, Sm, Md, Lg, Xl, Xl2 }

/// A translation: fixed pixels, or `calc(100% - Npx)` of the part's own size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shift { Px(f32), SelfMinus(f32) }

/// Everything a style can say about one part. Lengths are logical px.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recipe {
    pub height: Prop<f32>,
    pub width: Prop<f32>,
    pub min_height: Prop<f32>,
    pub min_width: Prop<f32>,
    pub max_width: Prop<f32>,
    /// `max-w-[calc(100%-Npx)]`: keep N px clear of the parent's width.
    pub max_width_inset: Prop<f32>,
    pub pad_top: Prop<f32>,
    pub pad_right: Prop<f32>,
    pub pad_bottom: Prop<f32>,
    pub pad_left: Prop<f32>,
    pub margin_top: Prop<f32>,
    pub margin_right: Prop<f32>,
    pub margin_bottom: Prop<f32>,
    pub margin_left: Prop<f32>,
    pub gap: Prop<f32>,
    /// `top-N` etc. of an absolutely placed part (dialog close button).
    pub inset_top: Prop<f32>,
    pub inset_right: Prop<f32>,
    pub inset_bottom: Prop<f32>,
    pub inset_left: Prop<f32>,
    /// `[&_svg:not([class*='size-'])]:size-N` and `[&>svg]:size-N`.
    pub icon_size: Prop<f32>,
    pub text_size: Prop<TextSize>,
    /// `leading-*` as a multiple of the font size.
    pub line_height: Prop<f32>,
    /// `tracking-*` in em.
    pub tracking: Prop<f32>,
    pub weight: Prop<Weight>,
    pub uppercase: Prop<bool>,
    pub underline: Prop<bool>,
    pub radius: Prop<Radius>,
    pub radius_top: Prop<Radius>,
    pub radius_bottom: Prop<Radius>,
    pub bg: Prop<Paint>,
    pub text: Prop<Paint>,
    pub border_width: Prop<f32>,
    pub border_top_width: Prop<f32>,
    pub border_color: Prop<Paint>,
    pub ring_width: Prop<f32>,
    pub ring_color: Prop<Paint>,
    pub shadow: Prop<ShadowLevel>,
    pub opacity: Prop<f32>,
    pub shift_x: Prop<Shift>,
    /// `active:translate-y-px` is the press nudge.
    pub shift_y: Prop<Shift>,
    /// `after:-inset-x-N`: hit area grows N px left and right.
    pub hit_inset_x: Prop<f32>,
    pub hit_inset_y: Prop<f32>,
    /// `backdrop-blur-*` in px.
    pub backdrop_blur: Prop<f32>,
}
```

  `Option::is_none_or` needs Rust 1.82; check `rustc --version`. If older,
  write `best.as_ref().map_or(true, |(k, _)| key >= *k)`.

- [ ] **Step 4: Run the tests, expect PASS.** Then clippy.

- [ ] **Step 5: Sync registry and commit.**

```bash
cp -r src/. skills/egui-shadcn/registry/src/
git add src/style src/lib.rs skills/egui-shadcn/registry/src
git commit -m "feat(style): recipe types for generated shadcn styles

A Recipe holds what one style sets for one cn-* part; each property
is a base value plus rules under state/context flags with Tailwind's
variant rank, and pick() resolves stacked recipes the way the CSS
cascade resolves the joined class lists."
```

---

### Task 3: stylegen skeleton, CSS parser, class splitter

**Files:**
- Create: `tools/stylegen/Cargo.toml`, `tools/stylegen/PIN`, `tools/stylegen/EXPORT`,
  `tools/stylegen/markers.txt`, `tools/stylegen/src/lib.rs`, `tools/stylegen/src/css.rs`,
  `tools/stylegen/src/class.rs`, `tools/stylegen/tests/fixtures/style-vega.css`,
  `tools/stylegen/tests/fixtures/style-nova.css`, `tools/stylegen/tests/fixtures/SHADCN-SVELTE-LICENSE`,
  `tools/stylegen/tests/parse.rs`

**Interfaces:**
- Produces: `stylegen::css::parse_style_css(&str) -> Vec<(String, Vec<String>)>` (marker → classes, first-appearance order);
  `stylegen::class::{Class, split}` with `Class { variants: Vec<String>, utility: String, important: bool }`;
  `stylegen::markers::{MarkerLine, parse_markers}` with `MarkerLine { file: String, dir: String, markers: Vec<String> }`.

- [ ] **Step 1: Create the workspace files.**

`tools/stylegen/Cargo.toml`:

```toml
[package]
name = "stylegen"
version = "0.1.0"
edition = "2021"
description = "Generate egui_shadcn style recipes from shadcn style CSS"
license = "MIT"
publish = false

# Its own workspace: this tool never becomes a dependency of egui_shadcn or of
# any project that vendors the registry.
[workspace]

[dependencies]

[dev-dependencies]
serde_json = "1"
```

`tools/stylegen/PIN`: one line, `493481fa`.
`tools/stylegen/EXPORT`: one line, `vega nova`.

`tools/stylegen/markers.txt` (the spec §11 marker list, derived from the base
sources at the pin; `cn-font-heading`, `cn-menu-*` and `*-logical` markers are
CLI transforms, not style rules, and are left out):

```
# <crate file> <upstream ui dir>: <cn markers the style CSS defines for it>
# A file with no markers only has its `// base:` quotes checked.
button.rs button: cn-button cn-button-variant-default cn-button-variant-outline cn-button-variant-secondary cn-button-variant-ghost cn-button-variant-destructive cn-button-variant-link cn-button-size-xs cn-button-size-sm cn-button-size-default cn-button-size-lg cn-button-size-icon-xs cn-button-size-icon-sm cn-button-size-icon cn-button-size-icon-lg
badge.rs badge: cn-badge cn-badge-variant-default cn-badge-variant-secondary cn-badge-variant-destructive cn-badge-variant-outline cn-badge-variant-ghost cn-badge-variant-link
card.rs card: cn-card cn-card-header cn-card-title cn-card-description cn-card-action cn-card-content cn-card-footer
layout.rs card:
checkbox.rs checkbox: cn-checkbox cn-checkbox-indicator
dialog.rs dialog: cn-dialog-overlay cn-dialog-content cn-dialog-close cn-dialog-header cn-dialog-footer cn-dialog-title cn-dialog-description
input.rs input: cn-input
label.rs label: cn-label
popover.rs popover: cn-popover-content cn-popover-header cn-popover-title cn-popover-description
select.rs select: cn-select-trigger cn-select-trigger-icon cn-select-content cn-select-viewport cn-select-group cn-select-label cn-select-item cn-select-item-indicator cn-select-item-indicator-icon cn-select-item-text cn-select-separator cn-select-scroll-up-button cn-select-scroll-down-button
separator.rs separator:
slider.rs slider: cn-slider cn-slider-track cn-slider-range cn-slider-thumb
switch.rs switch: cn-switch cn-switch-thumb
tabs.rs tabs: cn-tabs cn-tabs-list cn-tabs-list-variant-default cn-tabs-list-variant-line cn-tabs-trigger cn-tabs-content
tooltip.rs tooltip: cn-tooltip-content cn-tooltip-arrow
```

  Verify each list against upstream before committing:
  `grep -rhoE "cn-[a-z0-9-]+" $UP/docs/src/lib/registry/ui/<dir>/ | sort -u`.

Fixtures: copy `$UP/docs/src/lib/registry/styles/style-vega.css` and
`style-nova.css` into `tools/stylegen/tests/fixtures/`, and
`$UP/LICENSE.md` as `SHADCN-SVELTE-LICENSE` (MIT, like icongen's
`LUCIDE-LICENSE`).

- [ ] **Step 2: Read upstream's parser** so ours matches it:
  `grep -n "parseStyleCss" -A40 $UP/docs/scripts/build-registry.ts`. Note how
  it picks the subject marker (last `cn-*` ident in the selector), whether
  nested selectors or comments need special care, and how repeated rules
  concatenate. Adjust Step 4 if upstream differs from the description here.

- [ ] **Step 3: Write the failing tests** in `tools/stylegen/tests/parse.rs`:

```rust
use stylegen::class::split;
use stylegen::css::parse_style_css;
use stylegen::markers::parse_markers;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).expect("fixture")
}

#[test]
fn parses_nova_markers() {
    let m = parse_style_css(&fixture("style-nova.css"));
    let get = |k: &str| m.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()).expect(k);
    assert_eq!(get("cn-button-size-default"), ["h-8", "gap-1.5", "px-2.5", "has-data-[icon=inline-end]:pr-2", "has-data-[icon=inline-start]:pl-2"]);
    assert_eq!(get("cn-slider-range"), ["bg-primary"]);
    // ~420 @apply rules; most markers appear once.
    assert!(m.len() > 300, "{}", m.len());
}

#[test]
fn comments_are_not_markers() {
    let css = ".style-x {\n  /* MARK: cn-fake */\n  .cn-a { @apply h-1; }\n  .cn-b .cn-a-part { @apply w-2; }\n  .cn-a { @apply p-1; }\n}";
    let m = parse_style_css(css);
    assert_eq!(m, vec![("cn-a".into(), vec!["h-1".into(), "p-1".into()]), ("cn-a-part".into(), vec!["w-2".into()])]);
}

#[test]
fn split_respects_brackets() {
    let c = split("[&_svg:not([class*='size-'])]:size-4");
    assert_eq!(c.variants, ["[&_svg:not([class*='size-'])]"]);
    assert_eq!(c.utility, "size-4");
    let c = split("dark:hover:bg-input/50");
    assert_eq!(c.variants, ["dark", "hover"]);
    assert_eq!(c.utility, "bg-input/50");
    let c = split("[&>svg]:size-3!");
    assert!(c.important);
    assert_eq!(c.utility, "size-3");
    let c = split("hover:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]");
    assert_eq!(c.variants, ["hover"]);
}

#[test]
fn markers_file_parses() {
    let lines = parse_markers("# c\nbutton.rs button: cn-button cn-button-size-sm\nlayout.rs card:\n");
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].file, "button.rs");
    assert_eq!(lines[0].dir, "button");
    assert_eq!(lines[0].markers, ["cn-button", "cn-button-size-sm"]);
    assert!(lines[1].markers.is_empty());
}
```

- [ ] **Step 4: Run, expect compile failure.**
  `cargo test --jobs 4 --manifest-path tools/stylegen/Cargo.toml -- --test-threads 4`

- [ ] **Step 5: Implement.**

`tools/stylegen/src/lib.rs`:

```rust
//! stylegen: shadcn `style-<s>.css` → egui_shadcn style recipes.
pub mod class;
pub mod css;
pub mod markers;
```

`tools/stylegen/src/css.rs`:

```rust
/// Parse a shadcn `style-<s>.css` into `marker → classes`, as upstream's
/// `parseStyleCss` does: the subject of a rule is the last `cn-*` identifier
/// of its selector; several rules for one marker concatenate in source order.
pub fn parse_style_css(src: &str) -> Vec<(String, Vec<String>)> {
    let src = strip_comments(src);
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut rest = src.as_str();
    while let Some(at) = rest.find("@apply") {
        let head = &rest[..at];
        let open = head.rfind('{').expect("@apply inside a rule");
        let sel_start = head[..open].rfind(['{', '}', ';']).map_or(0, |i| i + 1);
        let selector = &head[sel_start..open];
        let end = at + rest[at..].find(';').expect("@apply ends with ;");
        let classes = rest[at + "@apply".len()..end].split_whitespace().map(str::to_owned);
        if let Some(marker) = last_marker(selector) {
            match out.iter_mut().find(|(m, _)| *m == marker) {
                Some((_, v)) => v.extend(classes),
                None => out.push((marker, classes.collect())),
            }
        }
        rest = &rest[end + 1..];
    }
    out
}

fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        rest = rest[i..].find("*/").map_or("", |j| &rest[i + j + 2..]);
    }
    out.push_str(rest);
    out
}

fn last_marker(selector: &str) -> Option<String> {
    let mut found = None;
    let mut rest = selector;
    while let Some(i) = rest.find("cn-") {
        let tail = &rest[i..];
        let len = tail.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-')).unwrap_or(tail.len());
        found = Some(tail[..len].to_owned());
        rest = &tail[len..];
    }
    found
}
```

`tools/stylegen/src/class.rs`:

```rust
/// One Tailwind class split into variants and utility.
#[derive(Debug, PartialEq)]
pub struct Class {
    pub variants: Vec<String>,
    pub utility: String,
    pub important: bool,
}

/// Split at top-level colons; `[]` and `()` protect colons inside arbitrary
/// values and selectors.
pub fn split(class: &str) -> Class {
    let mut variants = Vec::new();
    let (mut depth, mut start) = (0i32, 0usize);
    for (i, ch) in class.char_indices() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth -= 1,
            ':' if depth == 0 => {
                variants.push(class[start..i].to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }
    let raw = &class[start..];
    let (utility, important) = match (raw.strip_suffix('!'), raw.strip_prefix('!')) {
        (Some(u), _) | (_, Some(u)) => (u.to_owned(), true),
        _ => (raw.to_owned(), false),
    };
    Class { variants, utility, important }
}
```

`tools/stylegen/src/markers.rs` (add `pub mod markers;` to lib.rs):

```rust
/// One line of `markers.txt`.
#[derive(Debug, PartialEq)]
pub struct MarkerLine {
    pub file: String,
    pub dir: String,
    pub markers: Vec<String>,
}

pub fn parse_markers(src: &str) -> Vec<MarkerLine> {
    src.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (head, markers) = l.split_once(':').expect("`<file> <dir>: <markers>`");
            let mut h = head.split_whitespace();
            let file = h.next().expect("file").to_owned();
            let dir = h.next().expect("dir").to_owned();
            MarkerLine { file, dir, markers: markers.split_whitespace().map(str::to_owned).collect() }
        })
        .collect()
}
```

- [ ] **Step 6: Run tests, expect PASS; clippy**
  `cargo clippy --all-targets --jobs 4 --manifest-path tools/stylegen/Cargo.toml -- -D warnings`.

- [ ] **Step 7: Commit.**

```bash
git add tools/stylegen
git commit -m "feat(stylegen): workspace, style CSS parser, class splitter

Parses shadcn style-<s>.css into cn-marker -> class lists the way
upstream's parseStyleCss does. Pin 493481fa recorded in PIN; the
marker list per component is markers.txt."
```

---

### Task 4: stylegen translation tables (variants and utilities)

**Files:**
- Create: `tools/stylegen/src/model.rs`, `tools/stylegen/src/variant.rs`, `tools/stylegen/src/utility.rs`
- Test: `tools/stylegen/tests/translate.rs`

**Interfaces:**
- Consumes: `class::split` (Task 3).
- Produces:
  - `model::Field` (one variant per `Recipe` field, `fn rust_name(self) -> &'static str` giving the exact field name from Task 2), `model::Value` (`Px(f32)`, `Factor(f32)`, `Bool(bool)`, `Text(&'static str)` for `TextSize` variants or `Value::TextPx(f32)`, `Paint{role:&'static str, alpha:f32}`, `Mix(&'static str,&'static str,f32)`, `Radius(RadiusV)`, `Weight(&'static str)`, `Shadow(&'static str)`, `Shift(ShiftV)`) with `fn rust(&self) -> String` giving a Rust expression in terms of `crate::style` types.
  - `model::FLAGS: &[(&str, u32)]`: the `State` const names and bits, identical to Task 2.
  - `variant::{variant, V, Pseudo}`: `V::Cond { all: u32, none: u32, rank: u32 }`, `V::Always { rank: u32 }`, `V::Pseudo(Pseudo, u32)`, `V::Ignore(&'static str)`, `V::Unsupported`.
  - `utility::{utility, U}`: `U::Set(Vec<(Field, Value)>)`, `U::UseVar(Vec<Field>, String)`, `U::VarDef(String, f32)`, `U::Ignore(&'static str)`, `U::Unsupported(&'static str)`.

- [ ] **Step 1: Pin the Tailwind variant order.** Fetch
  `https://raw.githubusercontent.com/tailwindlabs/tailwindcss/v4.3.3/packages/tailwindcss/src/variants.ts`
  into the scratchpad, and read how candidates are sorted
  (`packages/tailwindcss/src/compile.ts` / `sort.ts`: variant order bitmask).
  Write down the registration order of the variants this tool meets. The
  ranks in Step 4 assume this order (lowest first): `*`/`**` and arbitrary
  `[…]` (0), `not-*` (1), `group-*` (2), `peer-*` (3), pseudo-elements
  `after`/`file` (4), `first`/`last` (5), `focus-within` (6), `hover` (7),
  `focus` (8), `focus-visible` (9), `active` (10), `disabled` (11), `in-*`
  (12), `has-*` (13), `aria-*` (14), `data-*` (15), `supports-*` (16), `sm`
  (17), `md` (18), `dark` (19). Correct the constants if the file says
  otherwise, and note the source line numbers in a comment.

- [ ] **Step 2: Write the failing tests** `tools/stylegen/tests/translate.rs`:

```rust
use stylegen::model::{Field, Value};
use stylegen::utility::{utility, U};
use stylegen::variant::{variant, Pseudo, V, R_DARK, R_HAS, R_HOVER, R_MD};

fn set(u: &str) -> Vec<(Field, Value)> {
    match utility(u, None) {
        U::Set(v) => v,
        other => panic!("{u}: {other:?}"),
    }
}

#[test]
fn spacing_scale() {
    assert_eq!(set("h-8"), [(Field::Height, Value::Px(32.0))]);
    assert_eq!(set("px-2.5"), [(Field::PadLeft, Value::Px(10.0)), (Field::PadRight, Value::Px(10.0))]);
    assert_eq!(set("size-3"), [(Field::Height, Value::Px(12.0)), (Field::Width, Value::Px(12.0))]);
    assert_eq!(set("h-[18.4px]"), [(Field::Height, Value::Px(18.4))]);
    assert_eq!(set("h-px"), [(Field::Height, Value::Px(1.0))]);
    assert_eq!(set("-mx-4"), [(Field::MarginLeft, Value::Px(-16.0)), (Field::MarginRight, Value::Px(-16.0))]);
    assert_eq!(set("p-[3px]").len(), 4);
    assert_eq!(set("max-w-sm"), [(Field::MaxWidth, Value::Px(384.0))]);
    assert_eq!(set("max-w-[calc(100%-2rem)]"), [(Field::MaxWidthInset, Value::Px(32.0))]);
}

#[test]
fn colours() {
    assert_eq!(set("bg-primary/80"), [(Field::Bg, Value::Paint { role: "Primary", alpha: 0.8 })]);
    assert_eq!(set("text-muted-foreground"), [(Field::Text, Value::Paint { role: "MutedForeground", alpha: 1.0 })]);
    assert_eq!(set("bg-black/10"), [(Field::Bg, Value::Paint { role: "Black", alpha: 0.1 })]);
    assert_eq!(set("border-transparent"), [(Field::BorderColor, Value::Paint { role: "Transparent", alpha: 1.0 })]);
    assert_eq!(set("ring-foreground/10"), [(Field::RingColor, Value::Paint { role: "Foreground", alpha: 0.1 })]);
    assert_eq!(
        set("bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]"),
        [(Field::Bg, Value::Mix("Secondary", "Foreground", 0.05))]
    );
}

#[test]
fn widths_vs_colours() {
    assert_eq!(set("border"), [(Field::BorderWidth, Value::Px(1.0))]);
    assert_eq!(set("border-2"), [(Field::BorderWidth, Value::Px(2.0))]);
    assert_eq!(set("border-t"), [(Field::BorderTopWidth, Value::Px(1.0))]);
    assert_eq!(set("ring-3"), [(Field::RingWidth, Value::Px(3.0))]);
    assert_eq!(set("ring-1"), [(Field::RingWidth, Value::Px(1.0))]);
    assert_eq!(set("text-sm"), [(Field::TextSize, Value::Text("Sm"))]);
    assert_eq!(set("text-[0.8rem]"), [(Field::TextSize, Value::TextPx(12.8))]);
    assert_eq!(set("text-xs/relaxed"), [(Field::TextSize, Value::Text("Xs")), (Field::LineHeight, Value::Factor(1.625))]);
}

#[test]
fn radius_and_misc() {
    use stylegen::model::RadiusV;
    assert_eq!(set("rounded-lg"), [(Field::Radius, Value::Radius(RadiusV::Rung("Lg")))]);
    assert_eq!(set("rounded-4xl"), [(Field::Radius, Value::Radius(RadiusV::Rung("Xl4")))]);
    assert_eq!(set("rounded-[4px]"), [(Field::Radius, Value::Radius(RadiusV::Px(4.0)))]);
    assert_eq!(set("rounded-[min(var(--radius-md),10px)]"), [(Field::Radius, Value::Radius(RadiusV::Min("Md", 10.0)))]);
    assert_eq!(set("rounded-b-xl"), [(Field::RadiusBottom, Value::Radius(RadiusV::Rung("Xl")))]);
    assert_eq!(set("shadow-md"), [(Field::Shadow, Value::Shadow("Md"))]);
    assert_eq!(set("font-medium"), [(Field::Weight, Value::Weight("Medium"))]);
    assert_eq!(set("translate-y-px"), [(Field::ShiftY, Value::Shift(stylegen::model::ShiftV::Px(1.0)))]);
    assert_eq!(set("translate-x-[calc(100%-2px)]"), [(Field::ShiftX, Value::Shift(stylegen::model::ShiftV::SelfMinus(2.0)))]);
}

#[test]
fn pseudo_targets() {
    assert!(matches!(utility("size-4", Some(Pseudo::Svg)), U::Set(v) if v == [(Field::IconSize, Value::Px(16.0))]));
    assert!(matches!(utility("-inset-2", Some(Pseudo::After)), U::Set(v) if v.len() == 2));
    assert!(matches!(utility("-inset-x-3", Some(Pseudo::After)), U::Set(v) if v == [(Field::HitInsetX, Value::Px(12.0))]));
}

#[test]
fn vars() {
    assert!(matches!(utility("[--card-spacing:--spacing(4)]", None), U::VarDef(n, v) if n == "--card-spacing" && v == 16.0));
    assert!(matches!(utility("px-(--card-spacing)", None), U::UseVar(f, n) if f == [Field::PadLeft, Field::PadRight] && n == "--card-spacing"));
}

#[test]
fn ignored_and_unsupported() {
    assert!(matches!(utility("flex", None), U::Ignore(_)));
    assert!(matches!(utility("duration-100", None), U::Ignore(_)));
    assert!(matches!(utility("border-b-input", None), U::Unsupported(_)));
    assert!(matches!(utility("bogus-utility", None), U::Unsupported(_)));
}

#[test]
fn variants() {
    assert!(matches!(variant("dark"), V::Cond { all: 1, none: 0, rank } if rank == R_DARK));
    assert!(matches!(variant("hover"), V::Cond { rank, .. } if rank == R_HOVER));
    assert!(matches!(variant("has-data-[icon=inline-start]"), V::Cond { all, rank, .. } if all == 1 << 16 && rank == R_HAS));
    assert!(matches!(variant("not-aria-[haspopup]"), V::Cond { all: 0, none, .. } if none == 1 << 10));
    assert!(matches!(variant("group-data-[size=sm]/switch"), V::Cond { all, .. } if all == 1 << 19));
    assert!(matches!(variant("md"), V::Always { rank } if rank == R_MD));
    assert!(matches!(variant("[&_svg:not([class*='size-'])]"), V::Pseudo(Pseudo::Svg, _)));
    assert!(matches!(variant("after"), V::Pseudo(Pseudo::After, _)));
    assert!(matches!(variant("data-open"), V::Ignore(_)));
    assert!(matches!(variant("[a]"), V::Ignore(_)));
    assert!(matches!(variant("lg"), V::Unsupported));
}
```

- [ ] **Step 3: Run, expect compile failure.**

- [ ] **Step 4: Implement `model.rs`, `variant.rs`, `utility.rs`.**

`model.rs`: field and value model, mirroring `src/style/recipe.rs` exactly:

```rust
/// One recipe field; `rust_name` is the field name in `src/style/recipe.rs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Field {
    Height, Width, MinHeight, MinWidth, MaxWidth, MaxWidthInset,
    PadTop, PadRight, PadBottom, PadLeft, MarginTop, MarginRight, MarginBottom, MarginLeft,
    Gap, InsetTop, InsetRight, InsetBottom, InsetLeft, IconSize, TextSize, LineHeight,
    Tracking, Weight, Uppercase, Underline, Radius, RadiusTop, RadiusBottom, Bg, Text,
    BorderWidth, BorderTopWidth, BorderColor, RingWidth, RingColor, Shadow, Opacity,
    ShiftX, ShiftY, HitInsetX, HitInsetY, BackdropBlur,
}

impl Field {
    pub fn rust_name(self) -> &'static str {
        use Field::*;
        match self {
            Height => "height", Width => "width", MinHeight => "min_height", MinWidth => "min_width",
            MaxWidth => "max_width", MaxWidthInset => "max_width_inset", PadTop => "pad_top",
            PadRight => "pad_right", PadBottom => "pad_bottom", PadLeft => "pad_left",
            MarginTop => "margin_top", MarginRight => "margin_right", MarginBottom => "margin_bottom",
            MarginLeft => "margin_left", Gap => "gap", InsetTop => "inset_top", InsetRight => "inset_right",
            InsetBottom => "inset_bottom", InsetLeft => "inset_left", IconSize => "icon_size",
            TextSize => "text_size", LineHeight => "line_height", Tracking => "tracking", Weight => "weight",
            Uppercase => "uppercase", Underline => "underline", Radius => "radius", RadiusTop => "radius_top",
            RadiusBottom => "radius_bottom", Bg => "bg", Text => "text", BorderWidth => "border_width",
            BorderTopWidth => "border_top_width", BorderColor => "border_color", RingWidth => "ring_width",
            RingColor => "ring_color", Shadow => "shadow", Opacity => "opacity", ShiftX => "shift_x",
            ShiftY => "shift_y", HitInsetX => "hit_inset_x", HitInsetY => "hit_inset_y",
            BackdropBlur => "backdrop_blur",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RadiusV { Rung(&'static str), Px(f32), Min(&'static str, f32) }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShiftV { Px(f32), SelfMinus(f32) }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Px(f32),
    Factor(f32),
    Bool(bool),
    Text(&'static str),
    TextPx(f32),
    Paint { role: &'static str, alpha: f32 },
    Mix(&'static str, &'static str, f32),
    Radius(RadiusV),
    Weight(&'static str),
    Shadow(&'static str),
    Shift(ShiftV),
}

impl Value {
    /// A Rust expression, valid with `use crate::style::*;` in scope.
    pub fn rust(&self) -> String {
        match *self {
            Value::Px(v) | Value::Factor(v) => format!("{v:?}"),
            Value::Bool(b) => b.to_string(),
            Value::Text(t) => format!("TextSize::{t}"),
            Value::TextPx(v) => format!("TextSize::Px({v:?})"),
            Value::Paint { role, alpha } => format!("Paint::Role(Role::{role}, {alpha:?})"),
            Value::Mix(a, b, t) => format!("Paint::Mix(Role::{a}, Role::{b}, {t:?})"),
            Value::Radius(RadiusV::Rung(r)) => format!("Radius::Rung(Rung::{r})"),
            Value::Radius(RadiusV::Px(v)) => format!("Radius::Px({v:?})"),
            Value::Radius(RadiusV::Min(r, v)) => format!("Radius::Min(Rung::{r}, {v:?})"),
            Value::Weight(w) => format!("Weight::{w}"),
            Value::Shadow(s) => format!("ShadowLevel::{s}"),
            Value::Shift(ShiftV::Px(v)) => format!("Shift::Px({v:?})"),
            Value::Shift(ShiftV::SelfMinus(v)) => format!("Shift::SelfMinus({v:?})"),
        }
    }
}

/// `State` const names and bits; must match `src/style/recipe.rs`.
pub const FLAGS: &[(&str, u32)] = &[
    ("DARK", 1 << 0), ("HOVER", 1 << 1), ("FOCUS", 1 << 2), ("FOCUS_VISIBLE", 1 << 3),
    ("ACTIVE", 1 << 4), ("DISABLED", 1 << 5), ("INVALID", 1 << 6), ("CHECKED", 1 << 7),
    ("UNCHECKED", 1 << 8), ("EXPANDED", 1 << 9), ("HAS_POPUP", 1 << 10), ("SELECTED", 1 << 11),
    ("PLACEHOLDER", 1 << 12), ("ICON_START", 1 << 16), ("ICON_END", 1 << 17),
    ("IN_BUTTON_GROUP", 1 << 18), ("SIZE_SM", 1 << 19), ("SIZE_DEFAULT", 1 << 20),
    ("VERTICAL", 1 << 21), ("HORIZONTAL", 1 << 22), ("VARIANT_LINE", 1 << 23),
    ("VARIANT_DEFAULT", 1 << 24), ("HAS_FOOTER", 1 << 25),
];

pub fn flag(name: &str) -> u32 {
    FLAGS.iter().find(|(n, _)| *n == name).map(|(_, b)| *b).expect("known flag")
}

/// `State::A.or(State::B)` for a bit set, `State::NONE` for 0.
pub fn state_expr(bits: u32) -> String {
    let names: Vec<String> = FLAGS.iter().filter(|(_, b)| bits & b != 0).map(|(n, _)| format!("State::{n}")).collect();
    match names.split_first() {
        None => "State::NONE".into(),
        Some((first, rest)) => rest.iter().fold(first.clone(), |acc, n| format!("{acc}.or({n})")),
    }
}
```

`variant.rs`: every variant the shipped markers use (see the variant census
in this plan's research: `dark`, `hover`, `focus-visible`, `aria-invalid`,
`data-checked`, …). Shape:

```rust
use crate::model::flag;

// Tailwind v4.3.3 variant order (variants.ts; see Step 1). Rank = bit index.
pub const R_ARBITRARY: u32 = 0;
pub const R_NOT: u32 = 1;
pub const R_GROUP: u32 = 2;
pub const R_PEER: u32 = 3;
pub const R_PSEUDO_ELEMENT: u32 = 4;
pub const R_FIRST_LAST: u32 = 5;
pub const R_FOCUS_WITHIN: u32 = 6;
pub const R_HOVER: u32 = 7;
pub const R_FOCUS: u32 = 8;
pub const R_FOCUS_VISIBLE: u32 = 9;
pub const R_ACTIVE: u32 = 10;
pub const R_DISABLED: u32 = 11;
pub const R_IN: u32 = 12;
pub const R_HAS: u32 = 13;
pub const R_ARIA: u32 = 14;
pub const R_DATA: u32 = 15;
pub const R_SUPPORTS: u32 = 16;
pub const R_SM: u32 = 17;
pub const R_MD: u32 = 18;
pub const R_DARK: u32 = 19;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pseudo { After, Svg }

#[derive(Debug, PartialEq)]
pub enum V {
    Cond { all: u32, none: u32, rank: u32 },
    Always { rank: u32 },
    Pseudo(Pseudo, u32),
    Ignore(&'static str),
    Unsupported,
}

const ANIM: &str = "open/close animation state; egui shows and hides at once";
const NOT_SHIPPED: &str = "styles a child or wrapper the crate does not ship";

fn cond(f: &str, rank: u32) -> V {
    V::Cond { all: flag(f), none: 0, rank }
}

pub fn variant(v: &str) -> V {
    match v {
        "dark" => cond("DARK", R_DARK),
        "hover" => cond("HOVER", R_HOVER),
        "focus" => cond("FOCUS", R_FOCUS),
        "focus-visible" => cond("FOCUS_VISIBLE", R_FOCUS_VISIBLE),
        "active" => cond("ACTIVE", R_ACTIVE),
        "disabled" => cond("DISABLED", R_DISABLED),
        "data-disabled" | "data-[disabled=true]" => cond("DISABLED", R_DATA),
        "group-data-[disabled=true]" | "group-has-disabled/field" => cond("DISABLED", R_GROUP),
        "peer-disabled" | "peer-data-disabled" => cond("DISABLED", R_PEER),
        "aria-invalid" => cond("INVALID", R_ARIA),
        "data-invalid" => cond("INVALID", R_DATA),
        "data-checked" | "data-[state=checked]" => cond("CHECKED", R_DATA),
        "aria-checked" => cond("CHECKED", R_ARIA),
        "data-unchecked" | "data-[state=unchecked]" => cond("UNCHECKED", R_DATA),
        "not-data-checked" => V::Cond { all: 0, none: flag("CHECKED"), rank: R_NOT },
        "aria-expanded" => cond("EXPANDED", R_ARIA),
        "not-aria-[haspopup]" => V::Cond { all: 0, none: flag("HAS_POPUP"), rank: R_NOT },
        "data-active" | "data-[active=true]" | "data-selected" => cond("SELECTED", R_DATA),
        "not-data-selected" => V::Cond { all: 0, none: flag("SELECTED"), rank: R_NOT },
        "data-placeholder" => cond("PLACEHOLDER", R_DATA),
        "has-data-[icon=inline-start]" => cond("ICON_START", R_HAS),
        "has-data-[icon=inline-end]" => cond("ICON_END", R_HAS),
        "in-data-[slot=button-group]" => cond("IN_BUTTON_GROUP", R_IN),
        "data-[size=sm]" => cond("SIZE_SM", R_DATA),
        "data-[size=default]" => cond("SIZE_DEFAULT", R_DATA),
        "data-vertical" | "data-[orientation=vertical]" => cond("VERTICAL", R_DATA),
        "data-horizontal" | "data-[orientation=horizontal]" => cond("HORIZONTAL", R_DATA),
        "data-[variant=line]" => cond("VARIANT_LINE", R_DATA),
        "has-data-[slot=card-footer]" => cond("HAS_FOOTER", R_HAS),
        "not-data-[variant=destructive]" => V::Always { rank: R_NOT },
        "sm" => V::Always { rank: R_SM },
        "md" => V::Always { rank: R_MD },
        "supports-backdrop-filter" => V::Always { rank: R_SUPPORTS },
        "after" => V::Pseudo(Pseudo::After, R_PSEUDO_ELEMENT),
        "[&_svg:not([class*='size-'])]" | "[&>svg]" => {
            V::Pseudo(Pseudo::Svg, R_ARBITRARY)
        }
        _ if v.starts_with("group-data-[size=sm]/") => cond("SIZE_SM", R_GROUP),
        _ if v.starts_with("group-data-[size=default]/") => cond("SIZE_DEFAULT", R_GROUP),
        _ if v.starts_with("group-data-vertical/") => cond("VERTICAL", R_GROUP),
        _ if v.starts_with("group-data-horizontal/") => cond("HORIZONTAL", R_GROUP),
        _ if v.starts_with("group-data-[variant=line]/") => cond("VARIANT_LINE", R_GROUP),
        _ if v.starts_with("group-data-[variant=default]/") => cond("VARIANT_DEFAULT", R_GROUP),
        "data-open" | "data-closed" | "data-[state=delayed-open]" | "data-entering" | "data-exiting"
        | "data-ending-style" | "data-starting-style" => V::Ignore(ANIM),
        _ if v.starts_with("data-[side=") || v.starts_with("data-[placement=") => {
            V::Ignore("slide-in direction of an animation")
        }
        "[a]" | "*:[a]" => V::Ignore("link-styled badge/description anchors are not ported"),
        "**" => V::Ignore("descendant repeat of the part's own colour"),
        // `*:` targets direct children; in the shipped markers only children
        // the crate does not draw (img, a, span, select-value, kbd).
        "*" => V::Ignore(NOT_SHIPPED),
        "file" => V::Ignore("file-input button; egui has no file input"),
        "[.border-b]" | "[.border-t]" => V::Ignore("card header/footer with an explicit border class is not offered"),
        _ if v.contains("kbd") || v.contains("field-label") || v.contains("img") || v.contains("combobox")
            || v.contains("input-group") || v.contains("button-group") && v != "in-data-[slot=button-group]"
            || v.starts_with("peer-data-[slot=") || v.contains("select-value") || v.contains("[span]") =>
        {
            V::Ignore(NOT_SHIPPED)
        }
        _ => V::Unsupported,
    }
}
```

  After implementing, run the census command from Step 5 and make every
  variant used by the shipped markers in **vega and nova** either translate or
  carry a precise ignore reason. A variant that is truly needed and has no
  flag yet is a spec gap: add a flag to `State` (Task 2) **and** `FLAGS` in
  the same commit, and tell the controller.

`utility.rs`: shape (complete it until the census in Step 5 shows no
unsupported utility in vega or nova):

```rust
use crate::model::{Field, RadiusV, ShiftV, Value};
use crate::variant::Pseudo;

#[derive(Debug, PartialEq)]
pub enum U {
    Set(Vec<(Field, Value)>),
    UseVar(Vec<Field>, String),
    VarDef(String, f32),
    Ignore(&'static str),
    Unsupported(&'static str),
}

const LAYOUT: &str = "flex/grid/position keyword; egui layout is done by the component";
const ANIM: &str = "animation or transition; out of scope (spec §4.4)";

const IGNORE_EXACT: &[(&str, &str)] = &[
    ("flex", LAYOUT), ("inline-flex", LAYOUT), ("grid", LAYOUT), ("flex-col", LAYOUT),
    ("flex-1", LAYOUT), ("items-center", LAYOUT), ("justify-center", LAYOUT), ("shrink-0", LAYOUT),
    ("relative", LAYOUT), ("absolute", LAYOUT), ("isolate", LAYOUT), ("w-full", LAYOUT),
    ("h-full", LAYOUT), ("w-auto", LAYOUT), ("block", LAYOUT), ("text-left", LAYOUT),
    ("text-center", LAYOUT), ("overflow-hidden", "egui clips children to the frame already"),
    ("select-none", "text selection is off for egui widgets"),
    ("pointer-events-none", "the component decides what senses input"),
    ("cursor-default", "cursor icon is chosen by the component"),
    ("outline-hidden", "focus is drawn with the ring, never a browser outline"),
    ("outline-none", "focus is drawn with the ring, never a browser outline"),
    ("bg-clip-padding", "egui fills inside the border already"),
    ("animate-in", ANIM), ("animate-out", ANIM),
    ("rotate-45", "the tooltip arrow is painted as a rotated square by the component"),
    ("translate-y-[calc(-50%-2px)]", "tooltip arrow placement is component geometry"),
    ("text-balance", "egui has no balanced wrapping"),
];
const IGNORE_PREFIX: &[(&str, &str)] = &[
    ("transition", ANIM), ("duration-", ANIM), ("fade-", ANIM), ("zoom-", ANIM),
    ("slide-in-from-", ANIM), ("z-", LAYOUT), ("scroll-my-", "scroll snapping"),
    ("underline-offset-", "egui draws text underlines at a fixed offset"),
];

/// Tailwind spacing: `N` → N×4px, `px` → 1, `[Npx]`, `[Nrem]` (16px rem),
/// `--spacing(N)`.
pub fn spacing(s: &str) -> Option<f32> {
    if s == "px" { return Some(1.0); }
    if s == "0" { return Some(0.0); }
    if let Some(inner) = s.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        return length(inner);
    }
    if let Some(n) = s.strip_prefix("--spacing(").and_then(|r| r.strip_suffix(')')) {
        return n.parse::<f32>().ok().map(|n| n * 4.0);
    }
    s.parse::<f32>().ok().map(|n| n * 4.0)
}

fn length(s: &str) -> Option<f32> {
    if let Some(v) = s.strip_suffix("px") { return v.parse().ok(); }
    if let Some(v) = s.strip_suffix("rem") { return v.parse::<f32>().ok().map(|v| v * 16.0); }
    None
}

/// `primary/80` → ("Primary", 0.8). Roles are the shadcn CSS variables.
pub fn role(s: &str) -> Option<(&'static str, f32)> {
    let (name, alpha) = match s.split_once('/') {
        Some((n, a)) => (n, a.parse::<f32>().ok()? / 100.0),
        None => (s, 1.0),
    };
    let r = match name {
        "background" => "Background", "foreground" => "Foreground", "card" => "Card",
        "card-foreground" => "CardForeground", "popover" => "Popover",
        "popover-foreground" => "PopoverForeground", "primary" => "Primary",
        "primary-foreground" => "PrimaryForeground", "secondary" => "Secondary",
        "secondary-foreground" => "SecondaryForeground", "muted" => "Muted",
        "muted-foreground" => "MutedForeground", "accent" => "Accent",
        "accent-foreground" => "AccentForeground", "destructive" => "Destructive",
        "destructive-foreground" => "DestructiveForeground", "border" => "Border",
        "input" => "Input", "ring" => "Ring", "white" => "White", "black" => "Black",
        "transparent" => "Transparent",
        _ => return None,
    };
    Some((r, alpha))
}

fn px(fields: &[Field], v: f32) -> U {
    U::Set(fields.iter().map(|f| (*f, Value::Px(v))).collect())
}

/// Spacing utility with a value that may be a CSS variable `(--name)`.
fn spaced(fields: &[Field], value: &str, sign: f32) -> U {
    if let Some(name) = value.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        return U::UseVar(fields.to_vec(), name.to_owned());
    }
    match spacing(value) {
        Some(v) => px(fields, v * sign),
        None => U::Unsupported("spacing value"),
    }
}

pub fn utility(u: &str, pseudo: Option<Pseudo>) -> U {
    use Field::*;
    match pseudo {
        Some(Pseudo::Svg) => {
            return match u.strip_prefix("size-").and_then(spacing) {
                Some(v) => px(&[IconSize], v),
                None => U::Unsupported("svg child utility other than size-*"),
            }
        }
        Some(Pseudo::After) => {
            return match u {
                "absolute" => U::Ignore(LAYOUT),
                _ if u.starts_with("-inset-x-") => spaced(&[HitInsetX], &u[9..], 1.0),
                _ if u.starts_with("-inset-y-") => spaced(&[HitInsetY], &u[9..], 1.0),
                _ if u.starts_with("-inset-") => spaced(&[HitInsetX, HitInsetY], &u[7..], 1.0),
                _ => U::Unsupported("after: utility other than -inset-*"),
            }
        }
        None => {}
    }
    if let Some((_, why)) = IGNORE_EXACT.iter().find(|(k, _)| *k == u) { return U::Ignore(why); }
    if let Some((_, why)) = IGNORE_PREFIX.iter().find(|(k, _)| u.starts_with(k)) { return U::Ignore(why); }
    if let Some(def) = u.strip_prefix("[--").and_then(|r| r.strip_suffix(']')) {
        let (name, value) = def.split_once(':').unwrap_or((def, ""));
        return match spacing(value) {
            Some(v) => U::VarDef(format!("--{name}"), v),
            None => U::Unsupported("CSS variable definition"),
        };
    }
    // Negative margins: `-mx-4`.
    let (sign, body) = match u.strip_prefix('-') { Some(b) => (-1.0, b), None => (1.0, u) };
    let sides: &[(&str, &[Field])] = &[
        ("size-", &[Height, Width]), ("h-", &[Height]), ("w-", &[Width]),
        ("min-h-", &[MinHeight]), ("min-w-", &[MinWidth]),
        ("px-", &[PadLeft, PadRight]), ("py-", &[PadTop, PadBottom]),
        ("pt-", &[PadTop]), ("pr-", &[PadRight]), ("pb-", &[PadBottom]), ("pl-", &[PadLeft]),
        ("p-", &[PadTop, PadRight, PadBottom, PadLeft]),
        ("mx-", &[MarginLeft, MarginRight]), ("my-", &[MarginTop, MarginBottom]),
        ("mt-", &[MarginTop]), ("mr-", &[MarginRight]), ("mb-", &[MarginBottom]), ("ml-", &[MarginLeft]),
        ("m-", &[MarginTop, MarginRight, MarginBottom, MarginLeft]),
        ("gap-", &[Gap]), ("top-", &[InsetTop]), ("right-", &[InsetRight]),
        ("bottom-", &[InsetBottom]), ("left-", &[InsetLeft]),
    ];
    for (prefix, fields) in sides {
        if let Some(v) = body.strip_prefix(prefix) {
            // `size-N!` arrives without `!` (class::split strips it).
            return spaced(fields, v, sign);
        }
    }
    if let Some(v) = u.strip_prefix("max-w-") {
        return match v {
            "xs" => px(&[MaxWidth], 320.0), "sm" => px(&[MaxWidth], 384.0),
            "md" => px(&[MaxWidth], 448.0), "lg" => px(&[MaxWidth], 512.0),
            "xl" => px(&[MaxWidth], 576.0), "2xl" => px(&[MaxWidth], 672.0),
            _ => match v.strip_prefix("[calc(100%-").and_then(|r| r.strip_suffix(")]")).and_then(length) {
                Some(n) => px(&[MaxWidthInset], n),
                None => U::Unsupported("max-w value"),
            },
        };
    }
    if let Some(v) = u.strip_prefix("text-") {
        let (size, leading) = v.split_once('/').map_or((v, None), |(s, l)| (s, Some(l)));
        let size_val = match size {
            "xs" => Some(Value::Text("Xs")), "sm" => Some(Value::Text("Sm")),
            "base" => Some(Value::Text("Base")), "lg" => Some(Value::Text("Lg")),
            _ => size.strip_prefix('[').and_then(|r| r.strip_suffix(']')).and_then(length).map(Value::TextPx),
        };
        if let Some(sv) = size_val {
            let mut out = vec![(TextSize, sv)];
            if let Some(l) = leading {
                match leading_factor(l) {
                    Some(f) => out.push((LineHeight, Value::Factor(f))),
                    None => return U::Unsupported("text size / leading"),
                }
            }
            return U::Set(out);
        }
        return match role(v) {
            Some((r, a)) => U::Set(vec![(Text, Value::Paint { role: r, alpha: a })]),
            None => U::Unsupported("text-* value"),
        };
    }
    if let Some(v) = u.strip_prefix("leading-") {
        return leading_factor(v).map_or(U::Unsupported("leading"), |f| U::Set(vec![(LineHeight, Value::Factor(f))]));
    }
    if let Some(v) = u.strip_prefix("tracking-") {
        let em = match v { "tighter" => -0.05, "tight" => -0.025, "normal" => 0.0, "wide" => 0.025, "wider" => 0.05, "widest" => 0.1, _ => return U::Unsupported("tracking") };
        return U::Set(vec![(Tracking, Value::Factor(em))]);
    }
    if let Some(v) = u.strip_prefix("font-") {
        let w = match v { "normal" => "Normal", "medium" => "Medium", "semibold" => "Semibold", "bold" => "Bold", _ => return U::Unsupported("font-* other than weight") };
        return U::Set(vec![(Weight, Value::Weight(w))]);
    }
    match u {
        "uppercase" => return U::Set(vec![(Uppercase, Value::Bool(true))]),
        "normal-case" => return U::Set(vec![(Uppercase, Value::Bool(false))]),
        "underline" => return U::Set(vec![(Underline, Value::Bool(true))]),
        "no-underline" => return U::Set(vec![(Underline, Value::Bool(false))]),
        "border" => return px(&[BorderWidth], 1.0),
        "border-none" => return px(&[BorderWidth], 0.0),
        "border-t" => return px(&[BorderTopWidth], 1.0),
        "ring" => return px(&[RingWidth], 1.0),
        _ => {}
    }
    for (prefix, field) in [("rounded-t-", RadiusTop), ("rounded-b-", RadiusBottom), ("rounded-", Radius)] {
        if let Some(v) = u.strip_prefix(prefix) {
            return radius(v).map_or(U::Unsupported("radius value"), |r| U::Set(vec![(field, Value::Radius(r))]));
        }
    }
    if let Some(v) = u.strip_prefix("bg-") {
        if let Some(mix) = v.strip_prefix("[color-mix(in_oklch,var(--").and_then(|r| r.strip_suffix("%)]")) {
            // `secondary),var(--foreground)_5`
            let parsed = mix.split_once("),var(--").and_then(|(a, rest)| {
                let (b, pct) = rest.split_once(")_")?;
                Some((role(a)?.0, role(b)?.0, pct.parse::<f32>().ok()? / 100.0))
            });
            return parsed.map_or(U::Unsupported("color-mix form"), |(a, b, t)| U::Set(vec![(Bg, Value::Mix(a, b, t))]));
        }
        return role(v).map_or(U::Unsupported("bg-* value"), |(r, a)| U::Set(vec![(Bg, Value::Paint { role: r, alpha: a })]));
    }
    if let Some(v) = u.strip_prefix("border-") {
        if let Ok(n) = v.parse::<f32>() { return px(&[BorderWidth], n); }
        if v.starts_with("b-") || v.starts_with("l-") || v.starts_with("r-") || v.starts_with("x-") || v.starts_with("y-") {
            return U::Unsupported("single-side border other than border-t (underline-style inputs)");
        }
        return role(v).map_or(U::Unsupported("border-* value"), |(r, a)| U::Set(vec![(BorderColor, Value::Paint { role: r, alpha: a })]));
    }
    if let Some(v) = u.strip_prefix("ring-") {
        if let Ok(n) = v.parse::<f32>() { return px(&[RingWidth], n); }
        if let Some(n) = v.strip_prefix('[').and_then(|r| r.strip_suffix(']')).and_then(length) { return px(&[RingWidth], n); }
        return role(v).map_or(U::Unsupported("ring-* value"), |(r, a)| U::Set(vec![(RingColor, Value::Paint { role: r, alpha: a })]));
    }
    if let Some(v) = u.strip_prefix("shadow-") {
        let s = match v { "none" => "None", "2xs" => "Xs2", "xs" => "Xs", "sm" => "Sm", "md" => "Md", "lg" => "Lg", "xl" => "Xl", "2xl" => "Xl2", _ => return U::Unsupported("shadow value") };
        return U::Set(vec![(Shadow, Value::Shadow(s))]);
    }
    if let Some(v) = u.strip_prefix("opacity-") {
        return v.parse::<f32>().map_or(U::Unsupported("opacity"), |n| U::Set(vec![(Opacity, Value::Factor(n / 100.0))]));
    }
    for (prefix, field) in [("translate-x-", ShiftX), ("translate-y-", ShiftY)] {
        if let Some(v) = body.strip_prefix(prefix) {
            if let Some(n) = v.strip_prefix("[calc(100%-").and_then(|r| r.strip_suffix(")]")).and_then(length) {
                return U::Set(vec![(field, Value::Shift(ShiftV::SelfMinus(n)))]);
            }
            return spacing(v).map_or(U::Unsupported("translate value"), |n| U::Set(vec![(field, Value::Shift(ShiftV::Px(n * sign)))]));
        }
    }
    if let Some(v) = u.strip_prefix("backdrop-blur-") {
        let n = match v { "xs" => 4.0, "sm" => 8.0, "md" => 12.0, "lg" => 16.0, "xl" => 24.0, _ => return U::Unsupported("blur value") };
        return px(&[BackdropBlur], n);
    }
    U::Unsupported("no translation rule")
}

fn leading_factor(v: &str) -> Option<f32> {
    Some(match v { "none" => 1.0, "tight" => 1.25, "snug" => 1.375, "normal" => 1.5, "relaxed" => 1.625, "loose" => 2.0, _ => return None })
}

fn radius(v: &str) -> Option<RadiusV> {
    let rung = |s: &str| -> Option<&'static str> {
        Some(match s { "none" => "None", "sm" => "Sm", "md" => "Md", "lg" => "Lg", "xl" => "Xl", "2xl" => "Xl2", "3xl" => "Xl3", "4xl" => "Xl4", "full" => "Full", _ => return None })
    };
    if let Some(r) = rung(v) { return Some(RadiusV::Rung(r)); }
    let inner = v.strip_prefix('[')?.strip_suffix(']')?;
    if let Some(n) = length(inner) { return Some(RadiusV::Px(n)); }
    // min(var(--radius-md),10px)
    let args = inner.strip_prefix("min(var(--radius-")?.strip_suffix(')')?;
    let (r, n) = args.split_once("),")?;
    Some(RadiusV::Min(rung(r)?, length(n)?))
}
```

  Note `spacing` handles `size-N` with the `!` already stripped by
  `class::split`. Check the `"-inset-…"` slicing indices against the
  prefixes' lengths when you write them.

- [ ] **Step 5: Census.** Add a temporary test (or `cargo run --example`)
  that runs every class of every shipped marker in `style-vega.css` and
  `style-nova.css` through `split` + `variant` + `utility` and prints the
  unsupported ones. Extend the tables until vega and nova show none. Then
  run it over all eight styles (`$UP/docs/src/lib/registry/styles/`) and keep
  the list of what remains unsupported for the other six: it goes into the
  Task 5 reports. Delete the temporary test.

- [ ] **Step 6: Run tests, clippy, commit.**

```bash
git add tools/stylegen
git commit -m "feat(stylegen): Tailwind variant and utility translation tables

Every class of the shipped markers in vega and nova translates to a
recipe field or is ignored with a stated reason; anything else is
reported as unsupported, never guessed. Variant ranks follow
tailwindcss 4.3.3's variant order."
```

---

### Task 5: stylegen resolve, emit, report; generate vega and nova

**Files:**
- Create: `tools/stylegen/src/resolve.rs`, `tools/stylegen/src/emit.rs`, `tools/stylegen/src/report.rs`, `tools/stylegen/src/main.rs`
- Create (generated): `src/style/style.rs`, `src/style/vega.rs`, `src/style/nova.rs`, `tools/stylegen/reports/{vega,nova,maia,lyra,mira,luma,sera,rhea}.md`
- Create: `tools/stylegen/tests/published.rs`, `tools/stylegen/tests/golden.rs`, fixtures `tools/stylegen/tests/fixtures/published/{vega,nova}/*.json`, `tools/stylegen/tests/fixtures/published-allow.txt`
- Modify: `src/style/mod.rs`

**Interfaces:**
- Consumes: Tasks 3–4.
- Produces:
  - `resolve::{collect_vars, translate, Decl, Marker, Vars}`: `translate(classes: &[String], vars: &Vars) -> Marker`, `Marker { decls: Vec<Decl>, ignored: Vec<(String, &'static str)>, unsupported: Vec<(String, String)> }`, `Decl { field, value, all, none, rank, idx }`.
  - `emit::{emit_struct, emit_style}` → Rust source strings.
  - `report::report(style: &str, pin: &str, markers: &[(String, Marker)]) -> String`.
  - CLI: `stylegen generate <shadcn-svelte> <crate-root>`, `stylegen drift <shadcn-svelte> <crate-root>` (drift lands in Task 6).
  - Crate: `crate::style::Style` with `pub name: &'static str` plus one `pub <marker>: Recipe` per marker in `markers.txt` (field name = marker without `cn-`, `-` → `_`, for example `button_size_icon_xs`, `slider_thumb`, `card_footer`); `Style::vega()`, `Style::nova()`, `impl Default for Style` (vega).

- [ ] **Step 1: Write the failing tests.**

`tools/stylegen/tests/golden.rs`: hand-checked values from the research
tables:

```rust
use stylegen::css::parse_style_css;
use stylegen::model::{Field, Value};
use stylegen::resolve::{collect_vars, translate, Marker};

fn style(name: &str) -> Vec<(String, Vec<String>)> {
    let p = format!("{}/tests/fixtures/style-{name}.css", env!("CARGO_MANIFEST_DIR"));
    parse_style_css(&std::fs::read_to_string(p).expect("fixture"))
}

fn marker(css: &[(String, Vec<String>)], m: &str) -> Marker {
    let vars = collect_vars(css);
    let classes = &css.iter().find(|(n, _)| n == m).expect(m).1;
    translate(classes, &vars)
}

/// Value of `field` with no condition.
fn base(m: &Marker, field: Field) -> Option<Value> {
    m.decls.iter().filter(|d| d.field == field && d.all == 0 && d.none == 0 && d.rank == 0).last().map(|d| d.value)
}
fn when(m: &Marker, field: Field, all: u32) -> Option<Value> {
    m.decls.iter().filter(|d| d.field == field && d.all == all).last().map(|d| d.value)
}

#[test]
fn nova_button_default_size() {
    let css = style("nova");
    let m = marker(&css, "cn-button-size-default");
    assert_eq!(base(&m, Field::Height), Some(Value::Px(32.0)));
    assert_eq!(base(&m, Field::PadLeft), Some(Value::Px(10.0)));
    assert_eq!(when(&m, Field::PadLeft, 1 << 16), Some(Value::Px(8.0)));
    assert!(m.unsupported.is_empty(), "{:?}", m.unsupported);
}

#[test]
fn vega_button_default_is_36() {
    let m = marker(&style("vega"), "cn-button-size-default");
    assert_eq!(base(&m, Field::Height), Some(Value::Px(36.0)));
}

#[test]
fn nova_destructive_is_tinted() {
    let m = marker(&style("nova"), "cn-button-variant-destructive");
    assert_eq!(base(&m, Field::Bg), Some(Value::Paint { role: "Destructive", alpha: 0.1 }));
    assert_eq!(when(&m, Field::Bg, 1), Some(Value::Paint { role: "Destructive", alpha: 0.2 }));
}

#[test]
fn nova_slider_thumb() {
    let m = marker(&style("nova"), "cn-slider-thumb");
    assert_eq!(base(&m, Field::Width), Some(Value::Px(12.0)));
    assert_eq!(base(&m, Field::HitInsetX), Some(Value::Px(8.0)));
    assert_eq!(base(&m, Field::Bg), Some(Value::Paint { role: "White", alpha: 1.0 }));
}

#[test]
fn card_spacing_var_resolves_with_size_condition() {
    let m = marker(&style("nova"), "cn-card-content");
    assert_eq!(base(&m, Field::PadLeft), Some(Value::Px(16.0)));
    assert_eq!(when(&m, Field::PadLeft, 1 << 19), Some(Value::Px(12.0)));
}

#[test]
fn later_class_wins_under_same_condition() {
    let css = vec![("cn-x".to_string(), vec!["h-8".to_string(), "h-9".to_string(), "hover:h-1".to_string(), "hover:h-2".to_string()])];
    let m = translate(&css[0].1, &collect_vars(&css));
    assert_eq!(m.decls.iter().filter(|d| d.field == Field::Height).count(), 2);
    assert_eq!(base(&m, Field::Height), Some(Value::Px(36.0)));
    assert_eq!(when(&m, Field::Height, 1 << 1), Some(Value::Px(8.0)));
}
```

`tools/stylegen/tests/published.rs`: the parse check against shadcn-svelte's
published per-style output. Copy the JSON from
`<scratch>/egui-shadcn-styles-research/dl/reg/{vega,nova}/`
into `tests/fixtures/published/{vega,nova}/` (fetched 2026-10-09 from
`https://shadcn-svelte.com/registry/styles/<s>/<c>.json`):

```rust
use stylegen::css::parse_style_css;
use stylegen::markers::parse_markers;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// Every class a style assigns to a shipped marker must appear in the
/// published component for that style, unless tailwind-merge dropped it in
/// favour of a base class, or upstream changed after the pin; each such case
/// is listed in published-allow.txt with its reason.
#[test]
fn style_classes_reach_the_published_components() {
    let allow = std::fs::read_to_string(format!("{ROOT}/tests/fixtures/published-allow.txt")).unwrap_or_default();
    let allowed = |s: &str, m: &str, c: &str| allow.lines().any(|l| l.split('#').next().unwrap_or("").split_whitespace().eq([s, m, c]));
    let markers = parse_markers(&std::fs::read_to_string(format!("{ROOT}/markers.txt")).expect("markers.txt"));
    let mut missing = Vec::new();
    for style in ["vega", "nova"] {
        let css = parse_style_css(&std::fs::read_to_string(format!("{ROOT}/tests/fixtures/style-{style}.css")).unwrap());
        for line in &markers {
            let path = format!("{ROOT}/tests/fixtures/published/{style}/{}.json", line.dir);
            let Ok(json) = std::fs::read_to_string(&path) else { continue };
            let v: serde_json::Value = serde_json::from_str(&json).expect("json");
            let content: String = v["files"].as_array().expect("files").iter()
                .map(|f| f["content"].as_str().unwrap_or("").to_owned()).collect::<Vec<_>>().join("\n");
            let tokens: std::collections::HashSet<&str> = content.split(|c: char| c.is_whitespace() || c == '"' || c == '`').collect();
            for m in &line.markers {
                let Some((_, classes)) = css.iter().find(|(n, _)| n == m) else { continue };
                for c in classes {
                    if !tokens.contains(c.as_str()) && !allowed(style, m, c) {
                        missing.push(format!("{style} {m} {c}"));
                    }
                }
            }
        }
    }
    assert!(missing.is_empty(), "not in published output:\n{}", missing.join("\n"));
}
```

  Run it once the parser exists; for each reported line, look at the
  published JSON to find out why (base class wins in tailwind-merge, or
  published site newer than the pin), and add it to `published-allow.txt`
  as `<style> <marker> <class>  # <reason>`. A line you cannot explain is a
  parser bug: fix the parser instead.

- [ ] **Step 2: Run, expect compile failure.**

- [ ] **Step 3: Implement `resolve.rs`.**

```rust
use crate::class::split;
use crate::model::{Field, Value};
use crate::utility::{utility, U};
use crate::variant::{variant, V};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decl {
    pub field: Field,
    pub value: Value,
    pub all: u32,
    pub none: u32,
    /// Tailwind variant-order bitmask (1 << R_*).
    pub rank: u32,
    /// Position of the class in the marker's list.
    pub idx: usize,
}

#[derive(Debug, Default)]
pub struct Marker {
    pub decls: Vec<Decl>,
    pub ignored: Vec<(String, &'static str)>,
    pub unsupported: Vec<(String, String)>,
}

#[derive(Clone, Copy, Debug)]
pub struct VarDef { pub all: u32, pub none: u32, pub rank: u32, pub px: f32 }
pub type Vars = HashMap<String, Vec<VarDef>>;

struct Variants { all: u32, none: u32, rank: u32, pseudo: Option<crate::variant::Pseudo> }

enum Skip { Ignore(&'static str), Unsupported(String) }

fn variants(vs: &[String]) -> Result<Variants, Skip> {
    let mut out = Variants { all: 0, none: 0, rank: 0, pseudo: None };
    for v in vs {
        match variant(v) {
            V::Cond { all, none, rank } => { out.all |= all; out.none |= none; out.rank |= 1 << rank; }
            V::Always { rank } => out.rank |= 1 << rank,
            // A pseudo target (svg child, ::after) is a different element:
            // it selects the field, it does not compete in the cascade.
            V::Pseudo(p, _) => out.pseudo = Some(p),
            V::Ignore(why) => return Err(Skip::Ignore(why)),
            V::Unsupported => return Err(Skip::Unsupported(format!("variant `{v}`"))),
        }
    }
    Ok(out)
}

/// CSS variables set anywhere in the style (`[--card-spacing:--spacing(4)]`,
/// `data-[size=sm]:[--card-spacing:--spacing(3)]`).
pub fn collect_vars(css: &[(String, Vec<String>)]) -> Vars {
    let mut vars = Vars::new();
    for (_, classes) in css {
        for raw in classes {
            let c = split(raw);
            let Ok(v) = variants(&c.variants) else { continue };
            if let U::VarDef(name, px) = utility(&c.utility, v.pseudo) {
                vars.entry(name).or_default().push(VarDef { all: v.all, none: v.none, rank: v.rank, px });
            }
        }
    }
    vars
}

pub fn translate(classes: &[String], vars: &Vars) -> Marker {
    let mut m = Marker::default();
    for (idx, raw) in classes.iter().enumerate() {
        let c = split(raw);
        let v = match variants(&c.variants) {
            Ok(v) => v,
            Err(Skip::Ignore(why)) => { m.ignored.push((raw.clone(), why)); continue; }
            Err(Skip::Unsupported(why)) => { m.unsupported.push((raw.clone(), why)); continue; }
        };
        match utility(&c.utility, v.pseudo) {
            U::Set(list) => {
                for (field, value) in list {
                    m.decls.push(Decl { field, value, all: v.all, none: v.none, rank: v.rank, idx });
                }
            }
            U::UseVar(fields, name) => match vars.get(&name) {
                Some(defs) => {
                    for d in defs {
                        for f in &fields {
                            m.decls.push(Decl { field: *f, value: Value::Px(d.px), all: v.all | d.all, none: v.none | d.none, rank: v.rank | d.rank, idx });
                        }
                    }
                }
                None => m.unsupported.push((raw.clone(), format!("unknown CSS variable {name}"))),
            },
            U::VarDef(..) => {}
            U::Ignore(why) => m.ignored.push((raw.clone(), why)),
            U::Unsupported(why) => m.unsupported.push((raw.clone(), why.to_owned())),
        }
    }
    m.decls = merge(std::mem::take(&mut m.decls));
    m
}

/// tailwind-merge inside one marker: for one field under one exact variant
/// set, only the last class survives.
fn merge(decls: Vec<Decl>) -> Vec<Decl> {
    let mut out: Vec<Decl> = Vec::new();
    for d in decls {
        out.retain(|o| !(o.field == d.field && o.all == d.all && o.none == d.none && o.rank == d.rank));
        out.push(d);
    }
    out
}
```

- [ ] **Step 4: Implement `emit.rs` and `report.rs`.**

`emit.rs`:

```rust
use crate::model::{state_expr, Field};
use crate::resolve::{Decl, Marker};
use std::collections::BTreeMap;

pub fn field_name(marker: &str) -> String {
    marker.trim_start_matches("cn-").replace('-', "_")
}

const HEADER: &str = "// @generated by tools/stylegen from shadcn-svelte";

pub fn emit_struct(pin: &str, markers: &[String]) -> String {
    let mut s = format!("{HEADER} {pin}. Do not edit; re-run stylegen.\n\nuse super::recipe::Recipe;\n\n");
    s += "/// One recipe per shadcn `cn-*` part the crate draws. Adjust a value in\n/// your theme setup: `let mut s = Style::nova(); s.button_size_default.height.base = Some(36.0);`\n";
    s += "#[derive(Clone, Debug, PartialEq)]\npub struct Style {\n    /// The upstream style name (`vega`, `nova`, …).\n    pub name: &'static str,\n";
    for m in markers {
        s += &format!("    /// `{m}`\n    pub {}: Recipe,\n", field_name(m));
    }
    s += "}\n";
    s
}

pub fn emit_style(style: &str, pin: &str, markers: &[String], per: &BTreeMap<String, Marker>) -> String {
    let mut s = format!("{HEADER} {pin} `style-{style}.css`. Do not edit; re-run stylegen.\n\nuse super::recipe::*;\nuse super::Style;\n\n");
    s += &format!("pub(super) fn style() -> Style {{\n    Style {{\n        name: \"{style}\",\n");
    for m in markers {
        s += &format!("        {}: {},\n", field_name(m), recipe(per.get(m)));
    }
    s += "    }\n}\n";
    s
}

fn recipe(m: Option<&Marker>) -> String {
    let Some(m) = m else { return "Recipe::default()".into() };
    let mut by_field: BTreeMap<Field, Vec<&Decl>> = BTreeMap::new();
    for d in &m.decls {
        by_field.entry(d.field).or_default().push(d);
    }
    if by_field.is_empty() {
        return "Recipe::default()".into();
    }
    let mut s = String::from("Recipe {\n");
    for (field, mut decls) in by_field {
        let base = decls.iter().filter(|d| d.all == 0 && d.none == 0 && d.rank == 0).last().map(|d| d.value.rust());
        decls.retain(|d| !(d.all == 0 && d.none == 0 && d.rank == 0));
        decls.sort_by_key(|d| (d.rank, d.idx));
        let rules: Vec<String> = decls.iter().map(|d| format!(
            "Rule {{ when: When::new({}, {}), rank: {}, value: {} }}",
            state_expr(d.all), state_expr(d.none), d.rank, d.value.rust()
        )).collect();
        let base = base.map_or("None".into(), |b| format!("Some({b})"));
        s += &format!("{}: Prop {{ base: {base}, rules: vec![{}] }},\n", field.rust_name(), rules.join(", "));
    }
    s += "..Recipe::default()\n}";
    s
}
```

`report.rs`:

```rust
use crate::resolve::Marker;

/// Markdown report: per marker, what did not translate and why.
pub fn report(style: &str, pin: &str, markers: &[(String, Marker)]) -> String {
    let unsupported: usize = markers.iter().map(|(_, m)| m.unsupported.len()).sum();
    let mut s = format!("# stylegen report: {style}\n\nUpstream: shadcn-svelte `{pin}`.\n\n");
    s += &format!("Unsupported classes in shipped markers: **{unsupported}**{}\n\n",
        if unsupported == 0 { " (exportable)" } else { " (not exportable)" });
    for (name, m) in markers {
        if m.unsupported.is_empty() && m.ignored.is_empty() { continue; }
        s += &format!("## `{name}`\n\n");
        for (c, why) in &m.unsupported { s += &format!("- **unsupported** `{c}`: {why}\n"); }
        for (c, why) in &m.ignored { s += &format!("- ignored `{c}`: {why}\n"); }
        s += "\n";
    }
    s
}
```

- [ ] **Step 5: Implement `main.rs`.**

```rust
//! stylegen generate <shadcn-svelte checkout> <crate root>
//! stylegen drift    <shadcn-svelte checkout> <crate root>

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use stylegen::{css, emit, markers, report, resolve};

const STYLES: &[&str] = &["vega", "nova", "maia", "lyra", "mira", "luma", "sera", "rhea"];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let (Some(cmd), Some(up), Some(root)) = (args.get(1), args.get(2), args.get(3)) else {
        eprintln!("usage: stylegen generate|drift <shadcn-svelte> <crate-root>");
        return ExitCode::from(2);
    };
    let (up, root) = (PathBuf::from(up), PathBuf::from(root));
    let tool = root.join("tools/stylegen");
    let pin = std::fs::read_to_string(tool.join("PIN")).expect("PIN").trim().to_owned();
    check_pin(&up, &pin);
    let lines = markers::parse_markers(&std::fs::read_to_string(tool.join("markers.txt")).expect("markers.txt"));
    match cmd.as_str() {
        "generate" => generate(&up, &root, &tool, &pin, &lines),
        "drift" => drift(&up, &root, &lines),
        _ => ExitCode::from(2),
    }
}

fn check_pin(up: &Path, pin: &str) {
    let out = Command::new("git").arg("-C").arg(up).args(["rev-parse", "HEAD"]).output().expect("git");
    let head = String::from_utf8_lossy(&out.stdout);
    assert!(head.starts_with(pin), "upstream checkout is at {head}, PIN says {pin}");
}

fn generate(up: &Path, root: &Path, tool: &Path, pin: &str, lines: &[markers::MarkerLine]) -> ExitCode {
    let shipped: Vec<String> = lines.iter().flat_map(|l| l.markers.clone()).collect();
    let export: Vec<String> = std::fs::read_to_string(tool.join("EXPORT")).expect("EXPORT").split_whitespace().map(str::to_owned).collect();
    let out_dir = root.join("src/style");
    write_rs(&out_dir.join("style.rs"), &emit::emit_struct(pin, &shipped));
    for style in STYLES {
        let path = up.join(format!("docs/src/lib/registry/styles/style-{style}.css"));
        let parsed = css::parse_style_css(&std::fs::read_to_string(&path).expect("style css"));
        let vars = resolve::collect_vars(&parsed);
        let mut per = BTreeMap::new();
        let mut ordered = Vec::new();
        for m in &shipped {
            let classes = parsed.iter().find(|(n, _)| n == m).map(|(_, c)| c.clone()).unwrap_or_default();
            ordered.push((m.clone(), resolve::translate(&classes, &vars)));
            per.insert(m.clone(), resolve::translate(&classes, &vars));
        }
        std::fs::write(tool.join(format!("reports/{style}.md")), report::report(style, pin, &ordered)).expect("report");
        if export.iter().any(|e| e == style) {
            let unsupported: usize = ordered.iter().map(|(_, m)| m.unsupported.len()).sum();
            assert_eq!(unsupported, 0, "{style} is in EXPORT but has unsupported classes; see reports/{style}.md");
            write_rs(&out_dir.join(format!("{style}.rs")), &emit::emit_style(style, pin, &shipped, &per));
        }
    }
    ExitCode::SUCCESS
}

fn write_rs(path: &Path, src: &str) {
    std::fs::write(path, src).expect("write");
    let ok = Command::new("rustfmt").args(["--edition", "2021"]).arg(path).status().expect("rustfmt").success();
    assert!(ok, "rustfmt failed on {}", path.display());
}

fn drift(_up: &Path, _root: &Path, _lines: &[markers::MarkerLine]) -> ExitCode {
    eprintln!("drift: implemented in Task 6");
    ExitCode::from(2)
}
```

  Add `pub mod emit; pub mod report; pub mod resolve;` (and `model`,
  `utility`, `variant` from Task 4) to `lib.rs`. Create
  `tools/stylegen/reports/` with a `.gitkeep` only if `generate` does not
  create the directory; better: `std::fs::create_dir_all` before writing.

- [ ] **Step 6: Generate.**

```bash
cargo run --jobs 4 --manifest-path tools/stylegen/Cargo.toml -- generate "$UP" "$PWD"
```

  Then make `src/style/mod.rs`:

```rust
//! shadcn styles: recipe types (hand-written, `recipe.rs`) and generated
//! style values (`style.rs`, `<name>.rs`, by `tools/stylegen`).

pub mod recipe;
mod style;
mod nova;
mod vega;

pub use recipe::*;
pub use style::Style;

impl Style {
    /// shadcn's classic look: new-york sizes on the current base.
    pub fn vega() -> Self { vega::style() }
    /// Reduced padding and margins for compact layouts.
    pub fn nova() -> Self { nova::style() }
}

impl Default for Style {
    fn default() -> Self { Style::vega() }
}
```

  Add a unit test in `mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_styles_carry_known_values() {
        let (v, n) = (Style::vega(), Style::nova());
        assert_eq!(v.name, "vega");
        assert_eq!(v.button_size_default.height.get(State::NONE), Some(36.0));
        assert_eq!(n.button_size_default.height.get(State::NONE), Some(32.0));
        assert_eq!(n.button_size_default.pad_left.get(State::ICON_START), Some(8.0));
        assert_eq!(Style::default(), Style::vega());
    }
}
```

- [ ] **Step 7: Run all tests** (crate and stylegen), clippy on both. Read
  `tools/stylegen/reports/vega.md` and `nova.md`: zero unsupported, every
  ignored reason makes sense for that class. Skim the other six reports.

- [ ] **Step 8: Sync registry, commit.**

```bash
cp -r src/. skills/egui-shadcn/registry/src/
git add tools/stylegen src/style skills/egui-shadcn/registry/src
git commit -m "feat(stylegen): emit Style, generate vega and nova

stylegen generate writes src/style/style.rs (one Recipe per shipped
marker), the exported styles' values, and a report per upstream style.
vega and nova translate without unsupported classes; the fixture test
checks the parse against shadcn-svelte's published per-style output."
```

---

### Task 6: stylegen drift check

**Files:**
- Create: `tools/stylegen/src/drift.rs`
- Modify: `tools/stylegen/src/main.rs` (`drift` command), `tools/stylegen/src/lib.rs`
- Test: `tools/stylegen/tests/drift.rs`

**Interfaces:**
- Produces: `drift::base_quotes(src: &str) -> Vec<(usize, String)>`;
  `drift::upstream_markers(text: &str) -> BTreeSet<String>` (skips `cn-font-heading`, `cn-menu-*`, `*-logical`, `cn-rtl-flip`);
  `drift::check(up: &Path, crate_root: &Path, lines: &[MarkerLine]) -> Vec<String>` (one human-readable line per problem; empty = clean).
- Exit code: `stylegen drift` exits 1 when `check` returns anything.

- [ ] **Step 1: Write the failing tests** `tools/stylegen/tests/drift.rs`:

```rust
use stylegen::drift::{base_quotes, check, upstream_markers};
use stylegen::markers::parse_markers;

#[test]
fn finds_base_quotes() {
    let src = "let a = 1;\n    // base: \"disabled:opacity-50\"\n// base: \"inline-flex shrink-0\" — note\nlet b = 2; // base: \"h-px\"\n";
    assert_eq!(base_quotes(src), [(2, "disabled:opacity-50".to_string()), (3, "inline-flex shrink-0".into()), (4, "h-px".into())]);
}

#[test]
fn marker_scan_skips_cli_markers() {
    let m = upstream_markers("class=\"cn-card-title cn-font-heading x\" cn-select-content-logical cn-menu-target cn-select-item");
    assert_eq!(m.into_iter().collect::<Vec<_>>(), ["cn-card-title", "cn-select-item"]);
}

#[test]
fn reports_stale_quote_and_marker_change() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("stylegen-drift-{}", std::process::id()));
    let up = dir.join("up");
    let krate = dir.join("crate");
    std::fs::create_dir_all(up.join("docs/src/lib/registry/ui/button")).unwrap();
    std::fs::create_dir_all(krate.join("src/components")).unwrap();
    std::fs::write(up.join("docs/src/lib/registry/ui/button/button.svelte"), "\"cn-button cn-button-size-new inline-flex disabled:opacity-50\"").unwrap();
    std::fs::write(krate.join("src/components/button.rs"), "// base: \"disabled:opacity-50\"\n// base: \"gone-class\"\n").unwrap();
    let lines = parse_markers("button.rs button: cn-button cn-button-size-old\n");
    let problems = check(&up, &krate, &lines);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(problems.len(), 3, "{problems:#?}");
    assert!(problems.iter().any(|p| p.contains("gone-class")));
    assert!(problems.iter().any(|p| p.contains("cn-button-size-new") && p.contains("appeared")));
    assert!(problems.iter().any(|p| p.contains("cn-button-size-old") && p.contains("disappeared")));
}
```

  (`CARGO_TARGET_TMPDIR` is Cargo's per-target scratch directory for
  integration tests, under the target dir, never `/tmp`; the test removes
  its directory at the end.)

- [ ] **Step 2: Run, expect compile failure.**

- [ ] **Step 3: Implement `drift.rs`.**

```rust
use crate::markers::MarkerLine;
use std::collections::BTreeSet;
use std::path::Path;

/// `// base: "<upstream classes>"` quotes, with 1-based line numbers.
pub fn base_quotes(src: &str) -> Vec<(usize, String)> {
    src.lines().enumerate().filter_map(|(i, l)| {
        let rest = &l[l.find("// base: \"")? + "// base: \"".len()..];
        Some((i + 1, rest[..rest.find('"')?].to_owned()))
    }).collect()
}

pub fn upstream_markers(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = text;
    while let Some(i) = rest.find("cn-") {
        let tail = &rest[i..];
        let len = tail.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-')).unwrap_or(tail.len());
        let m = &tail[..len];
        let cli = m == "cn-font-heading" || m.starts_with("cn-menu-") || m.ends_with("-logical") || m == "cn-rtl-flip" || m == "cn-logical-sides";
        if !cli && m.len() > 3 { out.insert(m.to_owned()); }
        rest = &tail[len..];
    }
    out
}

fn read_dir_text(dir: &Path) -> String {
    let mut s = String::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            if let Ok(t) = std::fs::read_to_string(e.path()) { s.push_str(&t); s.push('\n'); }
        }
    }
    s
}

pub fn check(up: &Path, crate_root: &Path, lines: &[MarkerLine]) -> Vec<String> {
    let mut problems = Vec::new();
    for line in lines {
        let upstream = read_dir_text(&up.join("docs/src/lib/registry/ui").join(&line.dir));
        let file = if line.file == "layout.rs" { crate_root.join("src/layout.rs") } else { crate_root.join("src/components").join(&line.file) };
        if let Ok(src) = std::fs::read_to_string(&file) {
            for (n, q) in base_quotes(&src) {
                if !upstream.contains(&q) {
                    problems.push(format!("{}:{n}: base quote no longer upstream in ui/{}: \"{q}\"", line.file, line.dir));
                }
            }
        }
        if line.markers.is_empty() { continue; }
        let up_set = upstream_markers(&upstream);
        let ours: BTreeSet<String> = line.markers.iter().cloned().collect();
        for m in up_set.difference(&ours) { problems.push(format!("{}: marker {m} appeared upstream; re-port", line.file)); }
        for m in ours.difference(&up_set) { problems.push(format!("{}: marker {m} disappeared upstream; re-port", line.file)); }
    }
    problems
}
```

  A `// base:` quote in `layout.rs` can only be checked against the line
  that names `layout.rs`; if `layout.rs` gets helpers for other components
  later, give each its own line.

  `main.rs`: replace the `drift` stub with

```rust
fn drift(up: &Path, root: &Path, lines: &[markers::MarkerLine]) -> ExitCode {
    let problems = stylegen::drift::check(up, root, lines);
    for p in &problems { println!("{p}"); }
    if problems.is_empty() { println!("drift: clean"); ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
```

- [ ] **Step 4: Run tests, then the real check** (expected clean: no quotes
  yet, markers.txt matches upstream):
  `cargo run --jobs 4 --manifest-path tools/stylegen/Cargo.toml -- drift "$UP" "$PWD"`

- [ ] **Step 5: Commit.**

```bash
git add tools/stylegen
git commit -m "feat(stylegen): drift check for base quotes and markers

stylegen drift lists every // base: quote that no longer occurs in the
pinned upstream component, and every cn marker that appeared or
disappeared upstream; each is a re-port job."
```

---

### Task 7: Theme carries a Style; resolution helpers

**Files:**
- Modify: `src/theme.rs`, `src/lib.rs`, `src/components/shared.rs`
- Test: `tests/theme.rs`

**Interfaces:**
- Consumes: `crate::style::{Style, State, Paint, Role, Radius, Rung, TextSize, Weight, ShadowLevel, Shift}` (Tasks 2, 5); `color::{srgb_to_oklch, mix_oklch}` (Task 1).
- Produces (later tasks rely on these exact names):
  - `Theme { pub style: Arc<Style>, pub palette: Palette, pub radius: f32, pub metrics: Metrics, pub dark: bool }`; it is `Clone` and no longer `Copy`.
  - `Theme::new(style: Style, palette: Palette, radius: f32) -> Theme` (dark = palette background OKLCH L < 0.5).
  - `Theme::vega_dark()`, `Theme::vega_light()`. `Theme::dark()`/`light()` stay as aliases of these until Task 24.
  - `Theme::state(&self, s: State) -> State` (adds `DARK`).
  - `Theme::paint(&self, p: Paint) -> Color32`.
  - `Theme::rung(&self, r: Rung) -> f32`; `Theme::radius_px(&self, r: Radius, max: f32) -> f32` (clamped to `0..=max` and to 255); `radius_sm/md/lg/xl` now use the multiplying scale.
  - `Theme::text_px(&self, s: TextSize) -> f32`.
  - `Theme::shadow(&self, s: ShadowLevel) -> egui::epaint::Shadow`.
  - `theme::family_for(ctx, Weight) -> FontFamily`.
  - `Palette::with_bold_menu_accent(self) -> Palette`.
  - `shared::ring(painter: &egui::Painter, rect: Rect, radius: f32, width: f32, color: Color32)`;
    `shared::shift(rect: Rect, x: Option<Shift>, y: Option<Shift>) -> Rect`.
  - `pub use style::Style;` from `lib.rs`.

- [ ] **Step 1: Write the failing tests** (append to `tests/theme.rs`):

```rust
use egui_shadcn::style::{Paint, Radius, Role, Rung, ShadowLevel, State, Style, TextSize};
use egui_shadcn::{Palette, Theme};

#[test]
fn new_detects_dark_from_palette() {
    assert!(Theme::new(Style::vega(), Palette::dark(), 10.0).dark);
    assert!(!Theme::new(Style::vega(), Palette::light(), 10.0).dark);
    assert_eq!(Theme::vega_dark().style.name, "vega");
}

#[test]
fn state_adds_dark() {
    assert!(Theme::vega_dark().state(State::HOVER).contains(State::DARK));
    assert!(!Theme::vega_light().state(State::HOVER).contains(State::DARK));
}

#[test]
fn radius_scale_multiplies() {
    let t = Theme::vega_dark();
    assert_eq!(t.rung(Rung::Sm), 6.0);
    assert_eq!(t.rung(Rung::Md), 8.0);
    assert_eq!(t.rung(Rung::Xl), 14.0);
    assert_eq!(t.rung(Rung::Xl4), 26.0);
    assert_eq!(t.radius_px(Radius::Min(Rung::Md, 6.0), 100.0), 6.0);
    assert_eq!(t.radius_px(Radius::Rung(Rung::Full), 10.0), 10.0);
}

#[test]
fn radius_clamps() {
    let mut t = Theme::vega_dark();
    t.radius = 0.0;
    assert_eq!(t.radius_px(Radius::Rung(Rung::Xl4), 50.0), 0.0);
    t.radius = 400.0;
    let r = t.radius_px(Radius::Rung(Rung::Xl4), 1000.0);
    assert!(r <= 255.0, "{r}");
    assert_eq!(t.radius_px(Radius::Rung(Rung::Xl4), 9.0), 9.0);
}

#[test]
fn paint_alpha_and_mix() {
    let t = Theme::vega_light();
    let p = t.paint(Paint::Role(Role::Primary, 1.0));
    assert_eq!(p, t.palette.primary);
    let half = t.paint(Paint::Role(Role::Primary, 0.5));
    assert!((half.a() as i32 - 128).abs() <= 1, "{half:?}");
    assert_eq!(t.paint(Paint::Role(Role::Transparent, 1.0)), egui::Color32::TRANSPARENT);
    let mix = t.paint(Paint::Mix(Role::Secondary, Role::Foreground, 0.05));
    assert_ne!(mix, t.palette.secondary);
}

#[test]
fn text_and_shadow() {
    let t = Theme::vega_dark();
    assert_eq!(t.text_px(TextSize::Sm), 14.0);
    assert_eq!(t.text_px(TextSize::Px(12.8)), 12.8);
    assert_eq!(t.shadow(ShadowLevel::None), egui::epaint::Shadow::NONE);
    assert_eq!(t.shadow(ShadowLevel::Md).offset, [0, 4]);
}

#[test]
fn bold_menu_accent_swaps_accent_for_primary() {
    let p = Palette::light().with_bold_menu_accent();
    assert_eq!(p.accent, Palette::light().primary);
    assert_eq!(p.accent_foreground, Palette::light().primary_foreground);
}

#[test]
fn current_without_apply_is_vega_dark_and_cheap() {
    let ctx = egui::Context::default();
    let a = Theme::current(&ctx);
    let b = Theme::current(&ctx);
    assert!(a.dark);
    assert_eq!(a.style.name, "vega");
    // The fallback shares one Style instead of building one per call.
    assert!(std::sync::Arc::ptr_eq(&a.style, &b.style));
}

#[test]
fn consumer_override_takes_effect() {
    let mut s = Style::nova();
    s.button_size_default.height.base = Some(40.0);
    let t = Theme::new(s, Palette::dark(), 10.0);
    assert_eq!(t.style.button_size_default.height.get(t.state(State::NONE)), Some(40.0));
}
```

- [ ] **Step 2: Run, expect compile failure.**

- [ ] **Step 3: Implement in `src/theme.rs`.** Key parts:

```rust
use crate::style::{Paint, Radius, Role, Rung, ShadowLevel, State, Style, TextSize, Weight};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// The shadcn style (vega, nova, …): every component part's recipe.
    pub style: Arc<Style>,
    pub palette: Palette,
    /// shadcn `--radius`; the scale multiplies from it.
    pub radius: f32,
    pub metrics: Metrics,
    pub dark: bool,
}

impl Theme {
    /// `dark` follows the palette's background (OKLCH lightness below 0.5);
    /// set `theme.dark` afterwards to override.
    pub fn new(style: Style, palette: Palette, radius: f32) -> Self {
        let dark = crate::color::srgb_to_oklch(palette.background).0 < 0.5;
        Self { style: Arc::new(style), palette, radius, metrics: Metrics::default(), dark }
    }
    pub fn vega_dark() -> Self { Self::new(Style::vega(), Palette::dark(), 10.0) }
    pub fn vega_light() -> Self { Self::new(Style::vega(), Palette::light(), 10.0) }
    /// Alias of [`Theme::vega_light`]; removed before 0.4.0 ships (Task 24).
    pub fn light() -> Self { Self::vega_light() }
    /// Alias of [`Theme::vega_dark`]; removed before 0.4.0 ships (Task 24).
    pub fn dark() -> Self { Self::vega_dark() }

    pub fn state(&self, s: State) -> State { s.with(State::DARK, self.dark) }

    pub fn rung(&self, r: Rung) -> f32 {
        let k = match r {
            Rung::None => 0.0, Rung::Sm => 0.6, Rung::Md => 0.8, Rung::Lg => 1.0, Rung::Xl => 1.4,
            Rung::Xl2 => 1.8, Rung::Xl3 => 2.2, Rung::Xl4 => 2.6, Rung::Full => return f32::INFINITY,
        };
        self.radius * k
    }
    /// Corner radius in px, clamped to `max` (half the part's short side)
    /// and to egui's `u8` corner radius.
    pub fn radius_px(&self, r: Radius, max: f32) -> f32 {
        let v = match r {
            Radius::Rung(r) => self.rung(r),
            Radius::Px(v) => v,
            Radius::Min(r, v) => self.rung(r).min(v),
        };
        v.min(max).clamp(0.0, 255.0)
    }
    pub fn radius_sm(&self) -> f32 { self.rung(Rung::Sm) }
    pub fn radius_md(&self) -> f32 { self.rung(Rung::Md) }
    pub fn radius_lg(&self) -> f32 { self.rung(Rung::Lg) }
    pub fn radius_xl(&self) -> f32 { self.rung(Rung::Xl) }

    pub fn role(&self, r: Role) -> Color32 {
        let p = &self.palette;
        match r {
            Role::Background => p.background, Role::Foreground => p.foreground,
            Role::Card => p.card, Role::CardForeground => p.card_foreground,
            Role::Popover => p.popover, Role::PopoverForeground => p.popover_foreground,
            Role::Primary => p.primary, Role::PrimaryForeground => p.primary_foreground,
            Role::Secondary => p.secondary, Role::SecondaryForeground => p.secondary_foreground,
            Role::Muted => p.muted, Role::MutedForeground => p.muted_foreground,
            Role::Accent => p.accent, Role::AccentForeground => p.accent_foreground,
            Role::Destructive => p.destructive, Role::DestructiveForeground => p.destructive_foreground,
            Role::Border => p.border, Role::Input => p.input, Role::Ring => p.ring,
            Role::White => Color32::WHITE, Role::Black => Color32::BLACK,
            Role::Transparent => Color32::TRANSPARENT,
        }
    }
    /// A style colour as egui paints it. Alpha scales the premultiplied colour,
    /// which matches CSS `/alpha` compositing in sRGB.
    pub fn paint(&self, p: Paint) -> Color32 {
        match p {
            Paint::Role(r, a) => if a >= 1.0 { self.role(r) } else { self.role(r).gamma_multiply(a) },
            Paint::Mix(a, b, t) => crate::color::mix_oklch(self.role(a), self.role(b), t),
        }
    }
    pub fn text_px(&self, s: TextSize) -> f32 {
        let m = &self.metrics;
        match s { TextSize::Xs => m.text_xs, TextSize::Sm => m.text_sm, TextSize::Base => m.text_base, TextSize::Lg => m.text_lg, TextSize::Px(v) => v }
    }
    /// Tailwind v4 shadows, first layer only (egui draws one); egui's spread
    /// is unsigned, so Tailwind's negative spreads become 0.
    pub fn shadow(&self, s: ShadowLevel) -> egui::epaint::Shadow {
        let (y, blur, a) = match s {
            ShadowLevel::None => return egui::epaint::Shadow::NONE,
            ShadowLevel::Xs2 => (1, 0, 0.05), ShadowLevel::Xs => (1, 2, 0.05),
            ShadowLevel::Sm => (1, 3, 0.10), ShadowLevel::Md => (4, 6, 0.10),
            ShadowLevel::Lg => (10, 15, 0.10), ShadowLevel::Xl => (20, 25, 0.10),
            ShadowLevel::Xl2 => (25, 50, 0.25),
        };
        egui::epaint::Shadow { offset: [0, y], blur, spread: 0, color: Color32::from_black_alpha((a * 255.0_f32).round() as u8) }
    }
}

impl Palette {
    /// shadcn `menuAccent: "bold"`: menus highlight with the primary pair.
    pub fn with_bold_menu_accent(mut self) -> Self {
        self.accent = self.primary;
        self.accent_foreground = self.primary_foreground;
        self
    }
}

/// Font family for a weight; Oxanium ships regular, medium and semibold.
pub fn family_for(ctx: &egui::Context, w: Weight) -> egui::FontFamily {
    match w {
        Weight::Normal => egui::FontFamily::Proportional,
        Weight::Medium => family(ctx, FAMILY_MEDIUM),
        Weight::Semibold | Weight::Bold => family(ctx, FAMILY_SEMIBOLD),
    }
}
```

  `Theme::current` keeps its contract with a shared fallback:

```rust
pub fn current(ctx: &egui::Context) -> Theme {
    static FALLBACK: std::sync::LazyLock<Theme> = std::sync::LazyLock::new(Theme::vega_dark);
    ctx.data(|d| d.get_temp::<Theme>(theme_id())).unwrap_or_else(|| FALLBACK.clone())
}
```

  In `apply`, `insert_temp(theme_id(), self.clone())`. Remove `shadow_md()`
  / `shadow_lg()` only if nothing uses them; otherwise reimplement them on
  `self.shadow(ShadowLevel::Md|Lg)` and leave removal to Task 24. `Palette`
  gains `Debug` already; `Theme` derives `Debug` now, so `Metrics` keeps
  its `Debug`. Fix any `Copy`-dependent call (`*self`, `let t = theme;`
  reuse) the compiler reports in `src/` and `tests/`.

  `src/lib.rs`: `pub mod style;` exists; add `pub use style::Style;`.

  `src/components/shared.rs` additions:

```rust
use crate::style::Shift;

/// shadcn's focus/invalid ring: a `ring-N` box-shadow drawn just outside
/// the part's border box.
pub fn ring(painter: &egui::Painter, rect: Rect, radius: f32, width: f32, color: Color32) {
    if width > 0.0 && color.a() > 0 {
        painter.rect_stroke(rect, radius, Stroke::new(width, color), egui::StrokeKind::Outside);
    }
}

/// Apply `translate-x/y` to a part's rect. `SelfMinus(n)` is `calc(100% - n)`
/// of the part's own size.
pub fn shift(rect: Rect, x: Option<Shift>, y: Option<Shift>) -> Rect {
    let d = |s: Option<Shift>, own: f32| match s {
        Some(Shift::Px(v)) => v,
        Some(Shift::SelfMinus(v)) => own - v,
        None => 0.0,
    };
    rect.translate(egui::vec2(d(x, rect.width()), d(y, rect.height())))
}
```

  (add `Rect` to the `use egui::{…}` line).

- [ ] **Step 4: Run all crate tests** (old snapshots must still pass: at
  radius 10 the multiplying and the additive scale agree on sm/md/lg/xl).
  `cargo test --jobs 4 -- --test-threads 4`, then clippy.

- [ ] **Step 5: Sync registry, commit.**

```bash
cp -r src/. skills/egui-shadcn/registry/src/
git add src tests skills/egui-shadcn/registry/src
git commit -m "feat(theme): Theme carries a Style; resolution helpers

Theme::new(style, palette, radius) detects dark from the palette;
paint/rung/radius_px/text_px/shadow resolve recipe values. The radius
scale now multiplies (sm x0.6 ... 4xl x2.6) like current shadcn; at
the default 10px radius sm/md/lg/xl are unchanged. Theme is Clone,
no longer Copy."
```

---

### Task 8: Test infrastructure: per-style snapshots, registry identity, extended token scan

**Files:**
- Create: `tests/common/mod.rs`, `tests/registry.rs`
- Modify: `tests/tokens.rs`

**Interfaces:**
- Produces:
  - `common::themes() -> Vec<(&'static str, Theme)>` → `vega_dark`, `vega_light`, `nova_dark`, `nova_light`.
  - `common::snapshot_all(name: &str, size: egui::Vec2, ui_fn: impl FnMut(&mut egui::Ui))` → writes/compares `tests/snapshots/{name}_{theme}.png`, painting a background-filled `CentralPanel` with 16px margin around `ui_fn`.
  - `tests/tokens.rs`: `const PORTED: &[&str]`, the file names that get the strict scan; each port task adds its file.

- [ ] **Step 1: `tests/common/mod.rs`:**

```rust
//! Shared test helpers. Each integration test file that needs them declares
//! `mod common;`.
#![allow(dead_code)]

use egui_kittest::Harness;
use egui_shadcn::style::Style;
use egui_shadcn::{Palette, Theme};

pub fn themes() -> Vec<(&'static str, Theme)> {
    vec![
        ("vega_dark", Theme::new(Style::vega(), Palette::dark(), 10.0)),
        ("vega_light", Theme::new(Style::vega(), Palette::light(), 10.0)),
        ("nova_dark", Theme::new(Style::nova(), Palette::dark(), 10.0)),
        ("nova_light", Theme::new(Style::nova(), Palette::light(), 10.0)),
    ]
}

/// One snapshot per style × {dark, light}: `{name}_{vega_dark|…}.png`.
pub fn snapshot_all(name: &str, size: egui::Vec2, mut ui_fn: impl FnMut(&mut egui::Ui)) {
    for (suffix, theme) in themes() {
        let bg = theme.palette.background;
        let mut h = Harness::builder().with_size(size).build_ui(|ui| {
            theme.apply(ui.ctx());
            egui::Frame::new().fill(bg).inner_margin(16).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                ui_fn(ui);
            });
        });
        h.run();
        h.snapshot(&format!("{name}_{suffix}"));
    }
}
```

  If `Harness::builder().build_ui` needs `'static` or the borrow of `ui_fn`
  across iterations does not compile, restructure to build each harness in
  its own block so the previous one is dropped first; keep the signature.

- [ ] **Step 2: `tests/registry.rs`:**

```rust
//! The registry copy is what projects vendor; it must equal `src/`.

use std::path::{Path, PathBuf};

fn files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).expect("dir") {
            let p = e.expect("entry").path();
            if p.is_dir() { stack.push(p) } else { out.push(p.strip_prefix(root).unwrap().to_path_buf()) }
        }
    }
    out.sort();
    out
}

#[test]
fn registry_src_is_byte_identical() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (src, reg) = (root.join("src"), root.join("skills/egui-shadcn/registry/src"));
    let (a, b) = (files(&src), files(&reg));
    assert_eq!(a, b, "file lists differ");
    assert!(a.len() > 10, "too few files: {a:?}");
    for f in a {
        assert!(std::fs::read(src.join(&f)).unwrap() == std::fs::read(reg.join(&f)).unwrap(), "{} differs", f.display());
    }
}
```

- [ ] **Step 3: Extend `tests/tokens.rs`.** Keep `violations` and its
  tests. Add a strict scanner and positive controls:

```rust
/// Strict scan for ported components (spec §7): besides colours and font
/// sizes, literal geometry, shadow definitions and alpha multipliers. A line
/// may carry them only with a `// base: "<upstream classes>"` quote on the
/// same line or the line above (stylegen's drift check verifies the quote).
/// Halving/doubling (`/ 2.0`, `* 2.0`) and 0.0/1.0 are arithmetic, not values.
fn strict_violations(source: &str) -> Vec<String> {
    let mut out = violations(source);
    let mut prev = "";
    for (n, line) in source.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        let quoted = line.contains("// base: \"") || prev.contains("// base: \"");
        let shadow = code.contains("Shadow {");
        let alpha = ["gamma_multiply(", "linear_multiply(", "from_black_alpha(", "from_white_alpha("]
            .iter().any(|p| code.split(p).skip(1).any(|r| r.starts_with(|c: char| c.is_ascii_digit())));
        let geometry = float_literals(code).any(|(lit, before)| {
            !matches!(lit, "0.0" | "1.0") && !((lit == "2.0") && (before.ends_with("/ ") || before.ends_with("* ")))
        });
        if (shadow || alpha || geometry) && !quoted && !line.trim_start().starts_with("//") {
            out.push(format!("{}: {}", n + 1, line.trim()));
        }
        prev = line;
    }
    out
}

/// Float literals in a line, each with the text before it.
fn float_literals(code: &str) -> impl Iterator<Item = (&str, &str)> {
    let b = code.as_bytes();
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < b.len() {
            let start = i;
            let ident_before = start > 0 && (b[start - 1].is_ascii_alphanumeric() || b[start - 1] == b'_');
            if b[i].is_ascii_digit() && !ident_before {
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.' || b[i] == b'_') { i += 1; }
                let lit = &code[start..i];
                if lit.contains('.') && !lit.ends_with('.') { return Some((lit.trim_end_matches("_f32"), &code[..start])); }
            } else {
                i += 1;
            }
        }
        None
    })
}

#[test]
fn strict_scanner_catches_literals() {
    assert_eq!(strict_violations("let h = 32.0;").len(), 1);
    assert_eq!(strict_violations("let c = fill.gamma_multiply(0.5);").len(), 1);
    assert_eq!(strict_violations("let s = Shadow { offset: [0, 1], blur: 3, spread: 0, color };").len(), 1);
    assert_eq!(strict_violations("// base: \"disabled:opacity-50\"\nlet c = fill.gamma_multiply(0.5);").len(), 0);
    assert_eq!(strict_violations("let c = rect.height() / 2.0;").len(), 0);
    assert_eq!(strict_violations("let w = 2.0 + x;").len(), 1);
    assert_eq!(strict_violations("let z = 0.0; let o = 1.0;").len(), 0);
    assert_eq!(strict_violations("let v = vec2(x, 0.0);").len(), 0);
}

/// Files under `src/components/` (plus `layout.rs`) already re-ported to the
/// shadcn-svelte base. Each port task adds its file; Task 24 replaces this
/// list with "all files".
const PORTED: &[&str] = &[];

#[test]
fn ported_components_take_values_from_the_style() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut found = Vec::new();
    for name in PORTED {
        let path = if *name == "layout.rs" { format!("{dir}/layout.rs") } else { format!("{dir}/components/{name}") };
        let src = std::fs::read_to_string(&path).expect("ported file exists");
        for v in strict_violations(&src) {
            found.push(format!("{path}:{v}"));
        }
    }
    assert!(found.is_empty(), "style values hard-coded in ported components:\n{}", found.join("\n"));
}
```

  Delete `metrics_drive_component_sizes` (its premise, `Metrics::control_md`,
  goes away); Task 10 adds the style-driven equivalent.

- [ ] **Step 4: Run** `cargo test --jobs 4 --test tokens --test registry -- --test-threads 4`
  → PASS. Clippy.

- [ ] **Step 5: Commit.**

```bash
git add tests
git commit -m "test: per-style snapshot helper, registry identity, strict token scan

snapshot_all renders a component in vega and nova, dark and light.
The registry copy is now checked byte for byte. Ported components get
a stricter literal scan: geometry, shadows and alpha need a // base:
quote of the upstream class."
```

---

### Task 9: Rewrite SKILL.md Part B as the porting checklist; start `references/styles.md`

Ports in Tasks 10–22 follow these rules, so they land first (spec §10).

**Files:**
- Modify: `skills/egui-shadcn/SKILL.md` (Part B, Hard rules)
- Create: `skills/egui-shadcn/references/styles.md`

- [ ] **Step 1: Replace "Part B" up to "Dependency discipline"** with the
  spec §8 checklist, adapted to the real names. Required content, in this
  order:
  1. Intro: a port is done only when every item holds; ports are judgement
     work, the rules make them come out the same each time.
  2. The ten checklist items of spec §8, with these concrete names:
     source path `docs/src/lib/registry/ui/<name>/` at `tools/stylegen/PIN`;
     marker list in `tools/stylegen/markers.txt`; recipe fields from
     `src/style/recipe.rs`; resolution via `pick(&[layers…], |r| &r.<field>, st)`;
     state via `Theme::state(State::…)`; base classes quoted as
     `// base: "<verbatim substring of the upstream class string>"` on the
     line above the code that uses them; a base class overriding a style
     property is said in that comment and the recipe field is not read;
     `stylegen drift` and `reports/<style>.md` must be clean for the
     component; snapshot via `common::snapshot_all`; the file goes into
     `PORTED` in `tests/tokens.rs`.
  3. "Behaviour, not just looks": list what the bits-ui primitive does
     (read `node_modules/bits-ui` source or https://bits-ui.com/docs/components/<name>)
     and check each item in egui; give the checklist used in Task 16 for
     Slider as the example.
  4. "Interaction states": never set `HOVER`/`ACTIVE`/`FOCUS_VISIBLE` when the
     widget is disabled; geometry is resolved without interaction states.
- [ ] **Step 2: Hard rules.** Replace "Hover/active = opacity modulation of
  the base color, never a new hue" with "Hover, active and focus follow the
  style's recipe; never invent a state colour". Replace "Colors come from
  `Palette`, sizes from `Metrics` …" with "Values come from the style's
  recipes (`Theme::current(ctx).style`) and the palette; a value the style
  lacks is a base class quoted under `// base:`, or a gap to report." Add:
  "Project-specific deviations go into the theme construction
  (`let mut s = Style::nova(); s.<part>.<field> = …;`), never into vendored
  files."
- [ ] **Step 3: Create `references/styles.md`** with these sections (filled
  now; Task 25 completes the last two): *The style model* (Theme = Style ×
  Palette × radius; style = class map; markers; base vs style classes;
  tailwind-merge order; the `pick` rule), *Recipe fields and conditions*
  (table: each `Recipe` field → the Tailwind utilities that set it; each
  `State` flag → its variants, copied from `recipe.rs` doc comments),
  *Breakpoints* (sm/md treated as matched; why), *The generator*
  (`stylegen generate`, `drift`, reports, EXPORT, ignored vs unsupported),
  *Choosing a style* (read `components.json` `style`; `Theme::new(Style::<that>(),
  Palette::…, radius)`; radius from the web project's `--radius`), *Bumping
  the pin* (TODO-free: update `PIN`, check out upstream at the new commit,
  `generate`, `drift`, re-port listed components, re-baseline snapshots,
  commit generated files, reports and PNGs together).
- [ ] **Step 4: Commit.**

```bash
git add skills/egui-shadcn
git commit -m "docs(skill): porting checklist for shadcn styles; styles reference

Part B is now the per-component checklist a port must satisfy:
source at the pin, marker inventory, recipe mapping, base quotes,
behaviour list, per-style snapshots, drift check clean."
```

---

## Port tasks (10–22): common frame

Every port task follows the Part B checklist from Task 9. Each one:

1. reads the upstream source `$UP/docs/src/lib/registry/ui/<dir>/*.svelte`
   and the bits-ui primitive it wraps (if any);
2. writes the tests listed in the task first and watches them fail;
3. rewrites the component so all values come from `pick(...)` over the
   named recipes, base classes are quoted, the behaviour list holds;
4. regenerates its snapshots:
   `UPDATE_SNAPSHOTS=1 cargo test --jobs 4 --test <file> <test_name> -- --test-threads 4`,
   then **reads every new PNG** (Read tool) and checks it against the
   numbers in the research tables and the published per-style JSON in
   `tools/stylegen/tests/fixtures/published/<style>/<dir>.json`;
5. deletes the component's old new-york snapshot(s) with `git rm`;
6. adds the file to `PORTED` in `tests/tokens.rs`, adds a row to
   `references/component-map.md` (component → markers), runs
   `stylegen drift` (must print `drift: clean`), the full crate test suite
   and clippy;
7. syncs the registry and commits (`feat(<component>): port to shadcn-svelte base`).

Task 10 (Button) shows the full pattern in code. The other port tasks give
the API, the recipe layering, the states and context to pass, the base
classes to look for, the behaviour list, and the tests; their paint code
follows Task 10's structure.

---

### Task 10: Button

**Files:**
- Modify: `src/components/button.rs`, `tests/components.rs`, `tests/tokens.rs` (`PORTED += "button.rs"`)
- Snapshots: create `tests/snapshots/button_{vega,nova}_{dark,light}.png`; `git rm tests/snapshots/button_variants.png`

**Interfaces:**
- Consumes: `Theme::{current, state, paint, radius_px, text_px, shadow}`, `family_for`, `shared::{ring, shift}`, `style::{pick, State, …}`.
- Produces (used by Dialog, Select, reference):
  - `ButtonVariant { Default, Outline, Secondary, Ghost, Destructive, Link }`
  - `ButtonSize { Xs, Sm, Default, Lg, Icon, IconXs, IconSm, IconLg }`
  - `Button::new(text)`, `.variant(v)`, `.size(s)`, `.enabled(bool)`,
    `.icon(&'static Icon)` (inline-start), `.icon_end(&'static Icon)`,
    `.expanded(bool)` (aria-expanded), `.invalid(bool)`, `.has_popup(bool)`.
  - `impl Widget for Button`.

- [ ] **Step 1: Tests first.** Replace the button tests in `tests/components.rs`
  (`button_click_sets_flag` stays, using `Theme::vega_dark()`;
  `button_variants_snapshot` goes) and add:

```rust
mod common;

#[test]
fn button_height_follows_style() {
    use egui_shadcn::components::button::Button;
    for (name, theme) in common::themes() {
        let want = if name.starts_with("nova") { 32.0 } else { 36.0 };
        let ctx = egui::Context::default();
        let mut h = 0.0;
        for _ in 0..2 {
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                theme.apply(ui.ctx());
                h = ui.add(Button::new("Save")).rect.height();
            });
        }
        assert_eq!(h, want, "{name}");
    }
}

#[test]
fn button_override_from_theme_setup() {
    use egui_shadcn::components::button::Button;
    use egui_shadcn::{Palette, Style, Theme};
    let mut s = Style::nova();
    s.button_size_default.height.base = Some(40.0);
    let theme = Theme::new(s, Palette::dark(), 10.0);
    let ctx = egui::Context::default();
    let mut h = 0.0;
    for _ in 0..2 {
        let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
            theme.apply(ui.ctx());
            h = ui.add(Button::new("Save")).rect.height();
        });
    }
    assert_eq!(h, 40.0);
}

#[test]
fn disabled_button_ignores_clicks() {
    use egui_kittest::kittest::Queryable;
    use egui_kittest::Harness;
    use egui_shadcn::components::button::Button;
    use std::cell::Cell;
    let clicked = Cell::new(false);
    let mut h = Harness::new_ui(|ui| {
        egui_shadcn::Theme::vega_dark().apply(ui.ctx());
        if ui.add(Button::new("Save").enabled(false)).clicked() {
            clicked.set(true);
        }
    });
    h.get_by_label("Save").click();
    h.run();
    assert!(!clicked.get());
}

#[test]
fn button_snapshot() {
    use egui_shadcn::components::button::{Button, ButtonSize, ButtonVariant};
    use egui_shadcn::components::icon::CLOSE;
    use egui_shadcn::layout;
    common::snapshot_all("button", egui::vec2(720.0, 200.0), |ui| {
        layout::vstack(ui, 12.0, |ui| {
            layout::row(ui, 8.0, |ui| {
                ui.add(Button::new("Default"));
                ui.add(Button::new("Secondary").variant(ButtonVariant::Secondary));
                ui.add(Button::new("Outline").variant(ButtonVariant::Outline));
                ui.add(Button::new("Ghost").variant(ButtonVariant::Ghost));
                ui.add(Button::new("Delete").variant(ButtonVariant::Destructive));
                ui.add(Button::new("Link").variant(ButtonVariant::Link));
            });
            layout::row(ui, 8.0, |ui| {
                ui.add(Button::new("Xs").size(ButtonSize::Xs));
                ui.add(Button::new("Small").size(ButtonSize::Sm));
                ui.add(Button::new("Default"));
                ui.add(Button::new("Large").size(ButtonSize::Lg));
                ui.add(Button::new("Close").icon(&CLOSE).size(ButtonSize::Icon).variant(ButtonVariant::Outline));
                ui.add(Button::new("With icon").icon(&CLOSE));
            });
            layout::row(ui, 8.0, |ui| {
                ui.add(Button::new("Disabled").enabled(false));
                ui.add(Button::new("Invalid").variant(ButtonVariant::Outline).invalid(true));
                ui.add(Button::new("Open").variant(ButtonVariant::Outline).expanded(true));
            });
        });
    });
}
```

  The in-module tests (`ghost_and_outline_hover_switch_text_with_fill`,
  `solid_hover_keeps_text`) are replaced by:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{Paint, Role, State, Style};

    fn layers(s: &Style, v: ButtonVariant, z: ButtonSize) -> [&Recipe; 3] {
        Button::new("x").variant(v).size(z).layers(s)
    }

    #[test]
    fn nova_icon_start_tightens_left_padding() {
        let s = Style::nova();
        let l = layers(&s, ButtonVariant::Default, ButtonSize::Default);
        assert_eq!(pick(&l, |r| &r.pad_left, State::NONE), Some(10.0));
        assert_eq!(pick(&l, |r| &r.pad_left, State::ICON_START), Some(8.0));
    }

    #[test]
    fn outline_hover_in_dark_uses_input_tint() {
        let s = Style::nova();
        let l = layers(&s, ButtonVariant::Outline, ButtonSize::Default);
        assert_eq!(pick(&l, |r| &r.bg, State::DARK | State::HOVER), Some(Paint::Role(Role::Input, 0.5)));
        assert_eq!(pick(&l, |r| &r.bg, State::HOVER), Some(Paint::Role(Role::Muted, 1.0)));
    }

    #[test]
    fn press_nudge_only_without_popup() {
        let s = Style::vega();
        let l = layers(&s, ButtonVariant::Default, ButtonSize::Default);
        assert!(pick(&l, |r| &r.shift_y, State::ACTIVE).is_some());
        assert!(pick(&l, |r| &r.shift_y, State::ACTIVE | State::HAS_POPUP).is_none());
    }
}
```

- [ ] **Step 2: Run, expect FAIL** (new sizes/methods missing).

- [ ] **Step 3: Port.** Read `$UP/docs/src/lib/registry/ui/button/button.svelte`
  (base string quoted in this plan's research section above). Structure:

```rust
//! shadcn Button, ported from shadcn-svelte `ui/button/button.svelte` at
//! `tools/stylegen/PIN`. Base classes are quoted under `// base:`; all
//! other values come from the style's `cn-button`, `cn-button-variant-*`
//! and `cn-button-size-*` recipes, stacked in that order.

use crate::components::icon::{paint_icon, Icon};
use crate::components::shared::{ring, shift};
use crate::style::{pick, Paint, Recipe, Role, State, Style, TextSize, Weight};
use crate::theme::family_for;
use crate::Theme;
use egui::{Color32, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};

impl Button {
    fn layers<'s>(&self, s: &'s Style) -> [&'s Recipe; 3] {
        let v = match self.variant {
            ButtonVariant::Default => &s.button_variant_default,
            ButtonVariant::Outline => &s.button_variant_outline,
            ButtonVariant::Secondary => &s.button_variant_secondary,
            ButtonVariant::Ghost => &s.button_variant_ghost,
            ButtonVariant::Destructive => &s.button_variant_destructive,
            ButtonVariant::Link => &s.button_variant_link,
        };
        let z = match self.size {
            ButtonSize::Xs => &s.button_size_xs,
            ButtonSize::Sm => &s.button_size_sm,
            ButtonSize::Default => &s.button_size_default,
            ButtonSize::Lg => &s.button_size_lg,
            ButtonSize::Icon => &s.button_size_icon,
            ButtonSize::IconXs => &s.button_size_icon_xs,
            ButtonSize::IconSm => &s.button_size_icon_sm,
            ButtonSize::IconLg => &s.button_size_icon_lg,
        };
        [&s.button, v, z]
    }

    fn icon_only(&self) -> bool {
        matches!(self.size, ButtonSize::Icon | ButtonSize::IconXs | ButtonSize::IconSm | ButtonSize::IconLg)
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let l = self.layers(&t.style);
        let enabled = self.enabled && ui.is_enabled();
        let context = State::NONE
            .with(State::ICON_START, self.icon.is_some())
            .with(State::ICON_END, self.icon_end.is_some())
            .with(State::EXPANDED, self.expanded)
            .with(State::INVALID, self.invalid)
            .with(State::HAS_POPUP, self.has_popup);
        // Geometry never depends on hover/press: resolve it before allocating.
        let g = t.state(context);
        let get = |f: fn(&Recipe) -> &crate::style::Prop<f32>| pick(&l, f, g).unwrap_or(0.0);
        let height = get(|r| &r.height);
        let (pad_l, pad_r, gap, border_w) = (get(|r| &r.pad_left), get(|r| &r.pad_right), get(|r| &r.gap), get(|r| &r.border_width));
        let icon_px = pick(&l, |r| &r.icon_size, g).unwrap_or(t.metrics.icon);
        let text_px = t.text_px(pick(&l, |r| &r.text_size, g).unwrap_or(TextSize::Sm));
        let weight = pick(&l, |r| &r.weight, g).unwrap_or(Weight::Normal);
        let font = egui::FontId::new(text_px, family_for(ui.ctx(), weight));
        // Text colour is laid out per state below; measure with any colour.
        let measure = ui.painter().layout_no_wrap(self.text.clone(), font.clone(), Color32::PLACEHOLDER);
        let icons = self.icon.is_some() as u8 + self.icon_end.is_some() as u8;
        let content_w = if self.icon_only() {
            icon_px
        } else {
            measure.size().x + icons as f32 * (icon_px + gap)
        };
        let width = if self.icon_only() {
            pick(&l, |r| &r.width, g).unwrap_or(height)
        } else {
            content_w + pad_l + pad_r + border_w * 2.0
        };
        let sense = if enabled { Sense::click() } else { Sense::hover() };
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), sense);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &self.text));
        if !ui.is_rect_visible(rect) {
            return resp;
        }
        // base: "disabled:pointer-events-none disabled:opacity-50"
        let st = t.state(context
            .with(State::HOVER, enabled && resp.hovered())
            .with(State::ACTIVE, enabled && resp.is_pointer_button_down_on())
            .with(State::FOCUS_VISIBLE, enabled && resp.has_focus())
            .with(State::DISABLED, !enabled));
        // base: "disabled:pointer-events-none disabled:opacity-50"
        let fade = |c: Color32| if enabled { c } else { c.gamma_multiply(0.5) };
        let paint = |f: fn(&Recipe) -> &crate::style::Prop<Paint>| pick(&l, f, st).map(|p| fade(t.paint(p)));
        let bg = paint(|r| &r.bg);
        let fg = paint(|r| &r.text).unwrap_or_else(|| fade(t.role(Role::Foreground)));
        let border = paint(|r| &r.border_color);
        let ring_c = paint(|r| &r.ring_color);
        let ring_w = pick(&l, |r| &r.ring_width, st).unwrap_or(0.0);
        let rect = shift(rect, pick(&l, |r| &r.shift_x, st), pick(&l, |r| &r.shift_y, st));
        let radius = pick(&l, |r| &r.radius, g).map_or(0.0, |r| t.radius_px(r, rect.height() / 2.0));
        let painter = ui.painter();
        if let Some(s) = pick(&l, |r| &r.shadow, st) {
            painter.add(t.shadow(s).as_shape(rect, radius));
        }
        if let Some(bg) = bg {
            painter.rect_filled(rect, radius, bg);
        }
        if let (Some(b), true) = (border, border_w > 0.0) {
            painter.rect_stroke(rect, radius, Stroke::new(border_w, b), StrokeKind::Inside);
        }
        if let Some(c) = ring_c {
            ring(painter, rect, radius, ring_w, c);
        }
        // … lay out icon(s) and text centred in `rect` (as the 0.3 code does),
        // in colour `fg`; underline the text when
        // `pick(&l, |r| &r.underline, st) == Some(true)` (link variant hover).
        resp
    }
}
```

  Notes for this port:
  - The `x` origin of the content is `rect.center().x - content_w / 2.0`
    shifted by `(pad_l - pad_r) / 2.0` so asymmetric icon-side padding
    shows; icons are painted with `paint_icon(painter, icon_rect, icon, fg, t.metrics.icon_stroke)`.
  - Ring: `focus-visible:ring-3 ring-ring/50` and `aria-invalid:ring-3
    ring-destructive/20` come through `ring_width`/`ring_color`; with no
    state set they resolve to `None`, so no ring.
  - `Color32::PLACEHOLDER` is egui's "use the galley's colour later" value;
    if the token scan objects, pass `fg` after computing it and lay out once.
  - Remove `hover_fill`/`mix_toward` use from this file; they stay in
    `shared.rs` until Task 24.
  - The `fade` line's `0.5` is the base `disabled:opacity-50`; the quote on
    the line above satisfies the strict scan.

- [ ] **Step 4: Run tests, regenerate snapshots, review PNGs** (common frame
  steps 4–6). Check in the nova PNG: default button 32px tall, `rounded-lg`
  (10px), destructive tinted; vega: 36px, `rounded-md` (8px), outline with
  `shadow-xs`.

- [ ] **Step 5: Sync registry, commit** (`feat(button): port to shadcn-svelte base`).

---

### Task 11: Badge

**Files:** `src/components/badge.rs`, `tests/components.rs`, `tests/tokens.rs`;
snapshots `badge_*` (new), `git rm tests/snapshots/card_badge_select.png` only
in Task 21 (it also shows card and select; until then leave it and its test).

**API:** `BadgeVariant { Default, Secondary, Destructive, Outline, Ghost, Link }`;
`badge(ui, text, variant) -> Response` stays; add
`Badge::new(text).variant(v).icon(&Icon).icon_end(&Icon)` implementing
`Widget`; `badge()` becomes `ui.add(Badge::new(text).variant(variant))`.

**Layers:** `[&s.badge, &s.badge_variant_<v>]`. **State:** `ICON_START`,
`ICON_END`, `DARK`; `HOVER` only for `Ghost`/`Link` (the `[a]:hover` rules are
ignored: badges are not links in egui); `FOCUS_VISIBLE` never (badge is not
focusable). Height comes from `height` (`h-5` in vega/nova); radius
`rounded-4xl` clamps to half the height. Icon size from `icon_size`
(`[&>svg]:size-3!`).

**Base classes to quote:** the base `badgeVariants` string in
`ui/badge/badge.svelte` (`inline-flex … shrink-0 … overflow-hidden …`) as
needed for layout decisions; there is no base literal geometry.

**Tests:**

```rust
#[test]
fn badge_is_h5_in_both_styles() {
    use egui_shadcn::components::badge::{badge, BadgeVariant};
    for (name, theme) in common::themes() {
        let ctx = egui::Context::default();
        let mut h = 0.0;
        for _ in 0..2 {
            let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                theme.apply(ui.ctx());
                h = badge(ui, "New", BadgeVariant::Default).rect.height();
            });
        }
        assert_eq!(h, 20.0, "{name}");
    }
}

#[test]
fn badge_snapshot() {
    use egui_shadcn::components::badge::{badge, BadgeVariant::*};
    common::snapshot_all("badge", egui::vec2(480.0, 64.0), |ui| {
        egui_shadcn::layout::row(ui, 8.0, |ui| {
            for (t, v) in [("Default", Default), ("Secondary", Secondary), ("Destructive", Destructive), ("Outline", Outline), ("Ghost", Ghost), ("Link", Link)] {
                badge(ui, t, v);
            }
        });
    });
}
```

**Behaviour list:** not interactive; `widget_info` label with the text.

---

### Task 12: Label and Separator

**Files:** `src/components/label.rs`, `src/components/separator.rs`,
`tests/components.rs`, `tests/tokens.rs` (both files). Snapshots
`label_separator_*`. `label_input.png` stays until Task 13.

**Label API:** `label(ui, text)` and `description(ui, text)` stay; add
`Label::new(text).enabled(bool)` (`peer-disabled:opacity-50` →
`State::DISABLED` → `opacity`). Recipe `[&s.label]`: `text_size`, `weight`,
`line_height`, `gap`, `opacity` under `DISABLED`. `description` has no
shadcn marker of its own: it renders `text-muted-foreground text-sm` (the
`cn-card-description`/`cn-dialog-description` value); read
`s.card_description` for it and say so in the doc comment.

**Separator:** no marker. Base: `"shrink-0 bg-border data-horizontal:h-px data-horizontal:w-full data-vertical:w-px data-vertical:self-stretch"`.
API: `separator(ui)` (horizontal), add `separator_vertical(ui)`. The 1px
comes from `t.metrics.border` with the `// base:` quote of `data-horizontal:h-px`.

**Tests:** snapshot of a label, a disabled label, a description, horizontal
and vertical separators; a unit test that the disabled label's text colour
alpha is half the enabled one (read through `pick(&[&s.label], |r| &r.opacity, State::DISABLED) == Some(0.5)`).

---

### Task 13: Input

**Files:** `src/components/input.rs`, `tests/components.rs`, `tests/tokens.rs`;
snapshots `input_*`; `git rm tests/snapshots/label_input.png` and its test.

**API:** `Input::new(&mut String)`, `.hint()`, `.password()`, `.max_width()`,
`.id_source()` stay; add `.enabled(bool)`, `.invalid(bool)`.

**Layers:** `[&s.input]`. **State:** `DARK`, `FOCUS_VISIBLE` (egui
`TextEdit` has focus), `DISABLED`, `INVALID`, `HOVER` (unused by vega/nova,
harmless). Values: `height` (`h-8` nova / `h-9` vega), `pad_left/right`
(`px-2.5`), `pad_top/bottom` (`py-1`), `radius`, `border_width` +
`border_color` (`border-input`, focus `border-ring`, invalid
`border-destructive`, dark invalid `/50`), `bg` (`bg-transparent`,
`dark:bg-input/30`, `disabled:bg-input/50`), `shadow` (vega `shadow-xs`),
`ring_*`, `text_size` (`text-base md:text-sm` → `Sm` via the md rule).

**Implementation:** keep the 0.3 approach (frameless `TextEdit` inside a
painted box) but size the box from the recipe; `TextEdit::font` gets the
resolved size; the box paints background, border, ring after the edit
reports focus (use `ui.painter().set` on a reserved shape index, or paint
the ring on the response afterwards as 0.3 does with `focus_ring`).

**Tests:** height per style (32 nova, 36 vega); typing works (`input_accepts_text`
updated to `Theme::vega_dark()`); a disabled input does not accept text
(`Harness` + `type_text` on the node, value stays empty); snapshot with
empty-with-hint, filled, invalid, disabled, and focused (`h.get_by_role(…)
.focus()` before `run()` if kittest offers it; otherwise skip focused in the
snapshot and cover ring colour with a `pick` unit test).

---

### Task 14: Checkbox

**Files:** `src/components/checkbox.rs`, `src/components/icon.rs` (add `CHECK`),
`tests/components.rs`, `tests/tokens.rs`; snapshots `checkbox_*`.
`switch_checkbox.png` is removed in Task 15.

**Icon:** generate Lucide `check` with icongen (command in
`skills/egui-shadcn/references/icons.md`) and add `pub const CHECK: Icon`
next to `CLOSE`, with the same licence note.

**API:** `checkbox(ui, &mut bool) -> Response` stays; add
`Checkbox::new(&mut bool).enabled(bool).invalid(bool)` (`Widget`).

**Layers:** box `[&s.checkbox]`, indicator `[&s.checkbox_indicator]`.
**State:** `CHECKED`/`UNCHECKED` from the value, `HOVER`, `FOCUS_VISIBLE`,
`INVALID`, `DISABLED`, `DARK`. Values: `width/height` (`size-4`),
`radius` (`rounded-[4px]`), border, bg (`dark:bg-input/30`,
`data-checked:bg-primary`), text colour of the check (`data-checked:text-primary-foreground`),
indicator `icon_size` (`[&>svg]:size-3.5`), ring.

**Base classes to quote:** `after:absolute after:-inset-x-3 after:-inset-y-2`
is the hit area. Allocate the visual size, then
`ui.interact(rect.expand2(vec2(12.0, 8.0)), id, Sense::click())` with the
quote on the line above; the 12/8 are those base classes.
`group-has-disabled/field:opacity-50`, `disabled:cursor-not-allowed disabled:opacity-50`.

**Behaviour list (bits-ui Checkbox):** click and Space toggle; disabled
ignores both; `widget_info` `selected(Checkbox, enabled, checked, "")`;
indeterminate is not offered (note in doc comment).

**Tests:** click inside the hit area but outside the box toggles (kittest
click at `rect.left() - 6.0`); disabled click does not toggle; Space
toggles when focused; snapshot (unchecked, checked, invalid, disabled
checked).

---

### Task 15: Switch

**Files:** `src/components/switch.rs`, `tests/components.rs`, `tests/tokens.rs`;
snapshots `switch_*`; `git rm tests/snapshots/switch_checkbox.png` and its test.

**API:** `toggle(ui, &mut bool)` stays; add `Switch::new(&mut bool).size(SwitchSize::{Default, Sm}).enabled(bool).invalid(bool)`.

**Layers:** track `[&s.switch]`, thumb `[&s.switch_thumb]`. **State:**
`CHECKED`/`UNCHECKED`, `SIZE_DEFAULT`/`SIZE_SM` (always one of them: track
sizes are `data-[size=…]` rules, thumb sizes `group-data-[size=…]/switch`),
`FOCUS_VISIBLE`, `INVALID`, `DISABLED`, `DARK`. Thumb position:
`shift_x` under `CHECKED` is `SelfMinus(2.0)` → thumb moves by
`thumb_w - 2`; animate between unchecked and checked offsets with
`ctx.animate_bool` as 0.3 does. Track has `border border-transparent`:
the thumb sits inside the 1px border (thumb rect left = track left + border).

**Base classes to quote:** hit area `after:-inset-x-3 after:-inset-y-2`,
`data-disabled:opacity-50`.

**Behaviour (bits-ui Switch):** click/Space/Enter toggle; disabled
inert; `widget_info selected(Checkbox, …)` (egui has no switch role).

**Tests:** nova default track is 32×18.4 (`rect.size()` ≈ (32, 18.4), since egui
rounds allocation to points; assert with 0.5 tolerance), sm is 24×14;
checked thumb right edge is 1px inside the track's right edge (from the
shift); disabled click no toggle; snapshot default/sm × off/on × enabled/disabled.

---

### Task 16: Slider (new)

**Files:**
- Create: `src/components/slider.rs`
- Modify: `src/components/mod.rs` (`pub mod slider;`), `tests/components.rs`, `tests/tokens.rs`
- Snapshots: `slider_*`

**API:**

```rust
pub enum SliderOrientation { Horizontal, Vertical }
pub struct Slider<'a> { /* value: &'a mut f32, range, step, orientation, enabled */ }
impl<'a> Slider<'a> {
    pub fn new(value: &'a mut f32, range: std::ops::RangeInclusive<f32>) -> Self;
    /// Snap to multiples of `step` from the range start; 0 = continuous.
    pub fn step(self, step: f32) -> Self;
    pub fn orientation(self, o: SliderOrientation) -> Self;
    pub fn enabled(self, yes: bool) -> Self;
    /// Accessible name (shadcn sets it through a label).
    pub fn label(self, text: impl Into<String>) -> Self;
}
impl Widget for Slider<'_> { … }
```

  One thumb only (bits-ui supports several; note it in the doc comment).

**Layers:** root `[&s.slider]` (`data-vertical:min-h-40`), track
`[&s.slider_track]`, range `[&s.slider_range]`, thumb `[&s.slider_thumb]`.
**State:** `HORIZONTAL`/`VERTICAL`, `HOVER` (thumb), `ACTIVE` (dragging),
`FOCUS_VISIBLE`, `DISABLED`, `DARK`.

**Geometry:** horizontal width = available width (`w-full` of the base
root; this is the one place `ui.available_width()` is the upstream
intent; quote `"relative flex w-full touch-none items-center"`), height =
max(thumb height, track height). Track thickness from `height` under
`HORIZONTAL` (`data-horizontal:h-1` nova / `h-1.5` vega). Thumb `size-3`
(nova) / `size-4` (vega), `border` + `border-ring` (nova) /
`border-primary` (vega), `bg-white`, ring `ring-3 ring-ring/50` on
hover/focus/active, nova hit area `after:-inset-2` (style class, via
`hit_inset_x/y`), shadow `shadow-sm` (vega).

**Behaviour list (bits-ui Slider; check each in a test or a stated reason):**
1. Click on the track jumps the value to that point (then snaps).
2. Drag the thumb; the pointer stays captured while the button is held,
   even outside the widget (egui `Sense::drag` does this).
3. Arrow Left/Down decrease, Right/Up increase by one step (continuous:
   1% of the range); PageUp/PageDown by 10 steps; Home/End to min/max.
4. Value is clamped to the range and snapped to the step.
5. Disabled: no pointer or keyboard change, no hover/focus styles, `data-disabled:opacity-50` (quote).
6. `widget_info`: `WidgetInfo::slider(enabled, value as f64, label)`.
7. Degenerate input: `min == max`, `step <= 0`, NaN value, value outside
   range → drawn, no panic, no NaN: the thumb sits at the start, the value is
   clamped (NaN becomes `min`), keyboard/pointer do nothing when `min == max`.

**Tests (all in `tests/components.rs`):**

```rust
fn run_slider(v: &mut f32, range: std::ops::RangeInclusive<f32>, step: f32, f: impl FnOnce(&mut egui_kittest::Harness<'_>)) {
    use egui_shadcn::components::slider::Slider;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(240.0, 40.0)).build_ui(|ui| {
        egui_shadcn::Theme::vega_dark().apply(ui.ctx());
        ui.add(Slider::new(v, range.clone()).step(step).label("Volume"));
    });
    h.run();
    f(&mut h);
}

#[test]
fn slider_keyboard_steps() {
    use egui_kittest::kittest::{Key, Queryable};
    let mut v = 50.0;
    run_slider(&mut v, 0.0..=100.0, 10.0, |h| {
        h.get_by_label("Volume").focus();
        h.run();
        h.key_press(Key::ArrowRight);
        h.run();
        h.key_press(Key::End);
        h.run();
    });
    assert_eq!(v, 100.0);
}

#[test]
fn slider_click_on_track_snaps() {
    use egui_kittest::kittest::Queryable;
    let mut v = 0.0;
    run_slider(&mut v, 0.0..=100.0, 25.0, |h| {
        let r = h.get_by_label("Volume").rect();
        h.click_at(egui::pos2(r.left() + r.width() * 0.6, r.center().y));
        h.run();
    });
    assert_eq!(v, 50.0);
}

#[test]
fn slider_degenerate_inputs_do_not_panic() {
    for (start, range, step) in [(5.0, 5.0..=5.0, 1.0), (3.0, 0.0..=10.0, 0.0), (f32::NAN, 0.0..=1.0, 0.1), (99.0, 0.0..=1.0, 0.1)] {
        let mut v = start;
        run_slider(&mut v, range.clone(), step, |h| { h.run(); });
        assert!(v.is_finite(), "{start} {range:?} {step}");
        assert!(range.contains(&v), "{v} not in {range:?}");
    }
}

#[test]
fn disabled_slider_ignores_input() {
    use egui_kittest::kittest::{Key, Queryable};
    use egui_shadcn::components::slider::Slider;
    let mut v = 50.0;
    {
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(240.0, 40.0)).build_ui(|ui| {
            egui_shadcn::Theme::vega_dark().apply(ui.ctx());
            ui.add(Slider::new(&mut v, 0.0..=100.0).enabled(false).label("Volume"));
        });
        h.run();
        let r = h.get_by_label("Volume").rect();
        h.click_at(egui::pos2(r.left() + 2.0, r.center().y));
        h.key_press(Key::Home);
        h.run();
    }
    assert_eq!(v, 50.0);
}

#[test]
fn slider_snapshot() {
    use egui_shadcn::components::slider::{Slider, SliderOrientation};
    let (mut a, mut b, mut c) = (30.0, 70.0, 50.0);
    common::snapshot_all("slider", egui::vec2(320.0, 200.0), |ui| {
        egui_shadcn::layout::row(ui, 24.0, |ui| {
            egui_shadcn::layout::vstack(ui, 16.0, |ui| {
                ui.set_width(200.0);
                ui.add(Slider::new(&mut a, 0.0..=100.0).label("A"));
                ui.add(Slider::new(&mut b, 0.0..=100.0).enabled(false).label("B"));
            });
            ui.add(Slider::new(&mut c, 0.0..=100.0).orientation(SliderOrientation::Vertical).label("C"));
        });
    });
}
```

  Check the exact kittest API names (`click_at`, `key_press`, `focus`,
  `rect`) in egui_kittest 0.34 and adapt the calls, not the assertions.

---

### Task 17: Tabs

**Files:** `src/components/tabs.rs`, `tests/components.rs`, `tests/tokens.rs`;
snapshots `tabs_*`; `git rm tests/snapshots/tabs.png` and its test.

**API:** `tab_bar(ui, &mut usize, labels) -> bool` stays; add
`Tabs::new(&mut usize, labels).variant(TabsVariant::{Default, Line}).orientation(…)`;
offer `Horizontal` only in 0.4.0 if vertical needs content layout the
crate lacks (state it in the doc comment).

**Layers:** list `[&s.tabs_list, &s.tabs_list_variant_<v>]`, trigger
`[&s.tabs_trigger]`, root `[&s.tabs]` (gap between list and content, used by
callers via the returned layout or documented). **State:** list:
`HORIZONTAL`, `VARIANT_LINE`; trigger: `SELECTED` (active), `VARIANT_DEFAULT`
or `VARIANT_LINE` (trigger rules are `group-data-[variant=…]/tabs-list`),
`HORIZONTAL`, `HOVER`, `FOCUS_VISIBLE`, `ICON_START/END`, `DISABLED`, `DARK`.
Height from `tabs_list` `height` under `HORIZONTAL` (`h-8` nova / `h-9`
vega); `p-[3px]`; active trigger `bg-background` (+`dark:` values),
`shadow-sm` under `VARIANT_DEFAULT|SELECTED`. The line variant draws the
`after:` underline: read the base trigger classes in
`ui/tabs/tabs-trigger.svelte` and quote them; the underline is base geometry.

**Width:** the 0.3 bar filled `available_width()`. Upstream list is
`inline-flex w-fit` (base): triggers size to their text. Port that: width =
sum of trigger widths. Quote the base class.

**Behaviour (bits-ui Tabs):** click selects; Left/Right move selection
(automatic activation); Home/End first/last; `widget_info` per trigger
`selected(SelectableLabel… )`: use `WidgetType::SelectableLabel` with
`selected`.

**Tests:** click selects; ArrowRight moves; trigger widths differ with label
length; snapshot default and line variants.

---

### Task 18: Tooltip

**Files:** `src/components/tooltip.rs`, `tests/overlays.rs`, `tests/tokens.rs`;
snapshots `tooltip_*`; `git rm tests/snapshots/tooltip.png` and its test.

**API:** `tooltip(resp, text) -> Response` stays.

**Layers:** content `[&s.tooltip_content]`, arrow `[&s.tooltip_arrow]`.
Colours: the base `TooltipContent` uses `bg-foreground text-background`
(read `ui/tooltip/tooltip-content.svelte`; 0.3 used `primary`, which is a
new-york-era choice; follow the base). Values: `pad_*` (`px-3 py-1.5`),
`text_size` (`text-xs`), `radius` (`rounded-md`), `gap`. Arrow: `size-2.5`
square, `rounded-[2px]`, rotated 45° (base `rotate-45` ignored by stylegen,
painted here: four points of a diamond centred on the content edge;
quote the base arrow classes), placed toward the trigger. Max width
`max-w-xs` (base) → quote.

**Tests:** hover shows the text (kittest `hover` on the trigger, then
`get_by_label("Mute")`); snapshot (trigger + open tooltip) per style.

---

### Task 19: Popover

**Files:** `src/components/popover.rs`, `tests/overlays.rs`, `tests/tokens.rs`;
snapshots `popover_*`; `git rm tests/snapshots/popover.png` and its test.

**API:** `Popover::new(&Response)`, `.width()`, `.show(…)` stay; add
`popover_header(ui, title, description)` for `cn-popover-header/title/description`.

**Layers:** `[&s.popover_content]` (`bg-popover`, `ring-1
ring-foreground/10` (nova/vega both use ring now, not border), `rounded-lg`
nova / `rounded-md` vega, `p-2.5` nova / `p-4` vega, `gap`, `shadow-md`,
`text-sm`). Width default `w-72` is a base class (quote it); the `.width()`
override stays. `sideOffset` 4 is bits-ui's default in the base (quote
from `popover-content.svelte`). The ring is `ring-1` outside the frame: egui
`Frame::stroke` draws inside; paint it with `shared::ring` around the
frame rect (`Frame::show` returns the response rect).

**Behaviour (bits-ui Popover):** click trigger toggles; Esc and click
outside close; focus moves into content (egui: not required; state it).

**Tests:** open by click, close by Esc (existing tests updated); snapshot
with header + body per style.

---

### Task 20: Select

**Files:** `src/components/select.rs`, `src/components/icon.rs` (add
`CHEVRON_DOWN` via icongen), `tests/overlays.rs`, `tests/components.rs`,
`tests/tokens.rs`; snapshots `select_*`.

**API:** `select(ui, id, &mut usize, options) -> bool` stays; add
`Select::new(id, &mut usize, options).size(SelectSize::{Default, Sm}).placeholder(text).enabled(bool).invalid(bool).width(w)`.

**Port:** the 0.3 version wraps `egui::ComboBox`. Upstream is a custom
trigger + popup list. Port both: the trigger is a custom-painted widget
(like Button) with `cn-select-trigger` and `cn-select-trigger-icon`
(`CHEVRON_DOWN`), the list an `egui::Popup` below it with
`cn-select-content` (frame), `cn-select-item` rows (`focus:` → `State::FOCUS`
for the keyboard-highlighted/hovered row, `CHECKED`/`SELECTED` for the
chosen one), and `cn-select-item-indicator` (`CHECK` icon at `right-2`,
`size-4`). Scroll buttons are replaced by an egui `ScrollArea` (state it;
markers `cn-select-scroll-*` resolved but unused; note it in the doc).
`cn-select-group`, `-label`, `-separator` are not offered by the
options-list API in 0.4.0; note it in the doc and in `component-map.md`.

**State:** trigger: `SIZE_DEFAULT`/`SIZE_SM`, `PLACEHOLDER` (nothing chosen),
`HOVER`, `FOCUS_VISIBLE`, `INVALID`, `DISABLED`, `EXPANDED` (popup open), `DARK`.
Items: `FOCUS` (hovered or arrow-key highlighted), `DISABLED`, `DARK`.

**Behaviour (bits-ui Select):** click/Enter/Space opens; Up/Down move the
highlight; Enter selects and closes; Esc closes without change; click
outside closes; typing a letter jumps to the next option starting with it;
`widget_info` `WidgetType::ComboBox` with the current text.

**Tests:** open + click option changes value; keyboard Down+Enter selects
next; Esc keeps the value; trigger height per style (nova 32, vega 36) and
sm (nova 28); snapshot closed trigger (placeholder, chosen, invalid,
disabled) and an open list per style (open it in the harness by clicking the
trigger before `snapshot`).

---

### Task 21: Card and `layout::card`

**Files:** `src/components/card.rs`, `src/layout.rs`, `tests/layout.rs`,
`tests/components.rs`, `tests/tokens.rs` (`card.rs`, `layout.rs`);
snapshots `card_*`; `git rm tests/snapshots/card_badge_select.png`,
`tests/snapshots/layout_card.png` and their tests.

**API:** `layout::card(ui, add)` stays (default size); add
`layout::card_with(ui, CardOptions { size: CardSize::{Default, Sm}, footer: Option<&mut dyn FnMut(&mut Ui)> }, add)`,
or a builder `Card::new().size(..).footer(|ui| …).show(ui, |ui| …)` in
`components/card.rs` with `layout::card` delegating to it. Keep
`card_title`, `card_description`; add `card_header(ui, title, description)`,
`card_action` slot in the header (right-aligned).

**Layers:** `[&s.card]`, `[&s.card_header]`, `[&s.card_title]`,
`[&s.card_description]`, `[&s.card_content]`, `[&s.card_footer]`,
`[&s.card_action]`. **State:** `SIZE_SM`/`SIZE_DEFAULT`, `HAS_FOOTER`,
`DARK`. Values: `py-(--card-spacing)` → `pad_top/bottom` with `pb-0` under
`HAS_FOOTER`; header/content/footer `px-(--card-spacing)`; `gap`;
`ring-1 ring-foreground/10` (no border) and vega `shadow-xs`… check the
generated values; `rounded-xl`; footer `bg-muted/50 border-t
rounded-b-xl` strip that reaches the card edges (nova): paint the footer
background as a rect spanning the card's full width with bottom corners
`radius_bottom`, and the top border line, then lay out the footer content
inside `p-(--card-spacing)`.

**Implementation hint:** build the card as an `egui::Frame` with zero inner
margin and paint/allocate the sections yourself inside, so the footer strip
can reach the edges; the outer ring via `shared::ring`.

**Tests:** card with footer: footer strip bottom equals card bottom (no
padding below it) in nova; `CardSize::Sm` content padding is 12 in nova;
snapshot: default card with header/content, card with footer, sm card, per
style.

---

### Task 22: Dialog

**Files:** `src/components/dialog.rs`, `tests/overlays.rs`, `tests/tokens.rs`;
snapshots `dialog_*`; `git rm tests/snapshots/dialog.png` and its test.

**API:** `Dialog::new(id, title)`, `.description()`, `.width()`, `.show()`
stay; `.backdrop(Color32)` stays as an override; add
`.show_close_button(bool)` (default true, as upstream). `dialog::footer`
keeps its signature but now paints the `cn-dialog-footer` strip.

**Layers:** overlay `[&s.dialog_overlay]` (`bg-black/10` +
`backdrop-blur-xs`: egui cannot blur; apply the colour only and say so),
content `[&s.dialog_content]` (`bg-popover`, `ring-1 ring-foreground/10`,
`rounded-xl`, `p-4` nova / `p-6` vega, `gap`, `text-sm`, `max-w-sm` (via
the `sm:` rule) and `max_width_inset` 32), title `[&s.dialog_title]`,
description `[&s.dialog_description]`, header `[&s.dialog_header]`, footer
`[&s.dialog_footer]` (nova: `bg-muted/50 -mx-4 -mb-4 rounded-b-xl border-t
p-4` → negative margins make it reach the panel edges), close
`[&s.dialog_close]` (`absolute top-2 right-2` nova / `top-4 right-4` vega).

**Close button:** upstream renders `<Button variant="ghost" size="icon-sm">`
with an X icon: use `Button::new("Close").icon(&CLOSE).variant(ButtonVariant::Ghost).size(ButtonSize::IconSm)`
placed with `ui.put` at `inset_top`/`inset_right` from the content rect.
Delete `close_cross`.

**Footer:** the panel's bottom padding must not show below the strip.
Lay out the body, then if `footer` was used, paint the strip from the
panel's left to right edge and down to its bottom edge, using
`margin_left/right/bottom` (negative) as the bleed.

**Tests:** Esc closes (existing); close button (`get_by_label("Close")`)
closes; dialog width in nova is 384 (`max-w-sm`) on a 1024-wide harness and
`screen - 32` on a 300-wide harness; snapshot with title, description, body,
footer with two buttons, per style (open dialog in an 800×500 harness).

---

### Task 23: Reference screen and example take a style

**Files:** `src/reference.rs`, `examples/settings.rs`, `tests/settings_snapshot.rs`;
snapshots `settings_*`.

**API:** `settings_ui(ui, &mut SettingsState)` stays. `SettingsState` gets a
`pub slider: f32` field (default 50.0) and the components tab shows the
new Slider, the Badge variants, and a Select with placeholder. The style is
whatever theme is applied; no new parameter is needed in `settings_ui`.

**Example:** `cargo run --example settings -- nova` picks the style from the
first argument (`vega` default, `nova`), and `light` as second argument
picks the light palette:

```rust
fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let style = match args.get(1).map(String::as_str) {
        Some("nova") => Style::nova(),
        _ => Style::vega(),
    };
    let palette = if args.get(2).map(String::as_str) == Some("light") { Palette::light() } else { Palette::dark() };
    let theme = Theme::new(style, palette, 10.0);
    eframe::run_native("egui-shadcn settings", eframe::NativeOptions::default(), Box::new(move |cc| {
        // Once, when the context is created: the theme does not change while the app runs.
        theme.apply(&cc.egui_ctx);
        Ok(Box::new(SettingsApp { state: SettingsState::default() }))
    }))
}
```

**Snapshots:** `settings_snapshot.rs` renders each tab with
`common::snapshot_all("settings_<tab>", …)`; remove the three old
`settings_*.png`.

**Accessibility test (spec §7, "every custom-painted widget reports
`widget_info`"):** in `settings_snapshot.rs`, render each tab once and
query every component on it by its label with kittest (`get_by_label`),
for example the Save button, the Volume slider, the role select, each switch and
checkbox, each tab trigger. A widget without `widget_info` is not found.

**Commit:** `feat(reference): settings gallery per style; example takes a style`.

---

### Task 24: Remove the migration scaffolding

**Files:** `src/theme.rs`, `src/components/shared.rs`, `src/lib.rs`,
`tests/tokens.rs`, `tests/components.rs`, any file still using the removed
items; registry copy.

- [ ] **Step 1:** Remove `Theme::dark()`/`Theme::light()`; replace remaining
  uses with `vega_dark()`/`vega_light()`.
- [ ] **Step 2:** Trim `Metrics` to the global values: `text_xs`, `text_sm`,
  `text_base`, `text_lg`, `text_mono`, `text_heading`, `icon`, `icon_stroke`,
  `border`, `field_max_width`. Remove `control_sm/md/lg`, `pad_sm/md/lg`,
  `gap`, `card_padding`, `dialog_width`, `popover_width`, `ring`. `apply`
  derives `spacing.item_spacing` / `button_padding` / `interact_size.y` for
  egui-native widgets from `Style` values (`button_size_default` height and
  padding, `8.0` gap → read `tabs.gap` or keep a documented default in
  `theme.rs`).
- [ ] **Step 3:** Remove `Palette::shadow` and `Palette::overlay` (shadows are
  `Theme::shadow`, the overlay colour comes from `dialog_overlay`), and
  `shadow_md()`/`shadow_lg()`.
- [ ] **Step 4:** Remove `shared::{hover_fill, mix_toward, focus_ring}` if
  unused, and the `mix_toward_blends` test.
- [ ] **Step 5:** In `tests/tokens.rs`, replace the `PORTED` list with all
  files in `src/components/` plus `layout.rs` (keep the `assert!(files.len() > 5)`
  guard). `icon.rs` path data and `shared.rs` helpers will hit the geometry
  rule: icon path coordinates are Lucide data, so mark the `CLOSE`/`CHECK`/
  `CHEVRON_DOWN` blocks with a file-level allow (skip lines inside
  `pub const …: Icon = Icon {` … `};`), not with per-line quotes.
- [ ] **Step 6:** Delete every remaining new-york snapshot PNG that no test
  produces (`git status` after a full test run with `UPDATE_SNAPSHOTS` unset
  shows none modified; list `tests/snapshots/` and `git rm` the orphans).
- [ ] **Step 7:** Full gates: crate tests, stylegen tests, both clippys,
  `stylegen drift` clean, registry test. Sync registry (`git rm` deleted
  files there too). Commit
  `refactor: drop new-york scaffolding (Metrics fields, Theme::dark/light, palette shadow/overlay)`.

---

### Task 25: Export gate for the other styles; docs; CHANGES

**Files:** `tools/stylegen/EXPORT`, `tools/stylegen/reports/*.md`,
`skills/egui-shadcn/SKILL.md`, `skills/egui-shadcn/references/{styles,component-map,token-map,gotchas}.md`,
`skills/egui-shadcn/registry/README.md`, `CHANGES.md`, `README.md` if it
names `Theme::dark()`.

- [ ] **Step 1: Export gate.** Re-run `stylegen generate`. For each of maia,
  lyra, mira, luma, sera, rhea: if its report says "(exportable)", add it
  to `EXPORT`, re-generate, add `mod <s>;` and `pub fn <s>() -> Self` to
  `src/style/mod.rs`, add it to `common::themes()`? No: snapshots stay
  vega/nova (spec §7); add a unit test that the style constructs and a
  button resolves a height. Otherwise leave it unexported. Record the
  outcome in `references/styles.md` ("exported: …; not exported, with the
  blocking classes: …").
- [ ] **Step 2: Docs.**
  - `references/component-map.md`: every row lists its markers (done per
    port; verify all 14 are there, Slider included).
  - `references/token-map.md`: rewrite around `Palette` roles and the
    recipe fields; remove `Metrics` fields that no longer exist.
  - `references/gotchas.md`: remove the "hover is opacity only" rule if it
    appears; add "geometry resolved without interaction states", "egui
    has no backdrop blur", "ring is drawn outside the border box".
  - `registry/README.md`: drop the request for `eframe` and image loaders
    (the crate needs only `egui` + `egui_extras`); show
    `Theme::new(Style::vega(), Palette::dark(), 10.0).apply(&ctx)`.
  - `SKILL.md` Part A: every `Theme::dark()` → `Theme::new(Style::…, …)`;
    explain choosing the style from the web project's `components.json`.
  - `references/styles.md`: complete "Choosing a style" and "Bumping the pin".
- [ ] **Step 3: CHANGES.md.** Add at the top, under no version header (the
  CI adds it), sections `### Changed`, `### Added`, `### Removed`. Draft
  (keep each entry ≤3 sentences, lead with what users see):

```markdown
### Changed
- Components now take their sizes, colours, radii and shadows from a shadcn
  style. `Theme::new(Style::vega(), Palette::dark(), 10.0)` gives the classic
  look and `Style::nova()` the compact one, matching a web client set up with
  the same `style` in `components.json`. Buttons, inputs and other controls
  look slightly different from 0.3 even with vega: destructive buttons and
  badges are tinted instead of solid, hovers use `muted`, cards and popovers
  draw a faint ring instead of a border.
- The radius scale multiplies the theme radius (`rounded-xl` = 1.4 × radius)
  as current shadcn does; at the default 10px radius `sm`, `md`, `lg` and `xl`
  are unchanged, larger themes get rounder corners.
- `Theme` is `Clone`, no longer `Copy`.

### Added
- Slider component, with keyboard stepping, track click and disabled state.
- Badge variants Ghost and Link; Button sizes Xs, IconXs, IconSm, IconLg and
  trailing icons; Switch size Sm; Tabs line variant; card footer strip.
- `Palette::with_bold_menu_accent()` for shadcn's `menuAccent: "bold"`.

### Removed
- `Theme::dark()` and `Theme::light()`: use `Theme::vega_dark()` /
  `Theme::vega_light()` or `Theme::new(…)`.
- `Metrics` fields that a style now sets (control heights, paddings, gap,
  card padding, dialog and popover width, ring width), and `Palette::shadow`
  / `Palette::overlay`. Set such values on the `Style` in your theme setup
  instead.
- The new-york look; there is no compatibility style.
```

- [ ] **Step 4: Commit** `docs: shadcn styles in the skill, references and CHANGES`.

---

### Task 26: Visual review with the user (controller task, not a subagent)

The spec (§8 item 8) wants each component reviewed against the web
rendering of the same style.

- [ ] **Step 1:** Generate a side-by-side page in the scratchpad: for each
  component, the four PNGs from `tests/snapshots/` next to a link to
  `https://shadcn-svelte.com/docs/components/<name>` (the docs page has a
  style switcher; vega and nova). Publish it as a private Artifact or open
  it locally, and ask the user to review.
- [ ] **Step 2:** Collect the user's findings; each one becomes a fix in the
  owning component (with a test or a snapshot update), not a tweak to a
  generated file.
- [ ] **Step 3:** Run the full gates once more and hand off with
  superpowers:finishing-a-development-branch. Version bumps, tag and
  publishing follow `RELEASING.md` and need the user's go-ahead.

---

## Self-review notes

- **Spec coverage.** §4.1 base components → Tasks 10–22; §4.2 Style and
  recipes → Tasks 2, 5; consumer adjustment → Tasks 7, 10 (tests); §4.3
  Theme → Task 7, 24; §4.4 code paths (context, OKLCH mix, footer strips,
  dialog close as Button) → Tasks 1, 10, 21, 22; §5 generator steps 1–5 →
  Tasks 3–6; bumping → Task 9/25 docs; §6 scope: vega+nova exported, six
  generated with reports and gated → Tasks 5, 25; reference + example →
  Task 23; §7 tests → Tasks 8, 10–22, 5, 6; §8 skill → Tasks 9, 25; §9
  consumer migration is not in this release (docs only, Task 25); §11 marker
  list → `markers.txt` in Task 3.
- **Deviations to confirm with the user:** breakpoints (sm/md matched) and
  ignored-vs-unsupported in the report; see "Two decisions" at the top.
- **Port tasks 11–22** state API, layering, states, base classes and
  behaviour lists, and full tests for the risky parts (Slider); their paint
  code follows Task 10's pattern rather than being written out here,
  because the spec makes base-component porting judgement work under the
  §8 checklist.
