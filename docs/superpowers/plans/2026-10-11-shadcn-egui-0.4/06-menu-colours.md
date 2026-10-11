# Phase 5 — Menu colours: inverted and translucent Select lists

> Part of [00-index.md](00-index.md). Read its Global Constraints first.

**Spec:** D7, §5.2, §7 (inverted menu snapshot).
**Index:** C "Menu rendering (Phase 5)".

**Outcome:** the open Select list draws with `t.menu_theme()`: an inverted
menu is dark (palette **and** `dark: true`) also in light mode; a translucent
menu fills with `popover` at 70 % alpha and highlights rows with
`foreground` at 10 %. The trigger is unchanged. Existing snapshots stay
byte-identical; two new snapshot pairs are reviewed by the user.

---

### Task 5.1: Select draws its list with `menu_theme()` and translucency

**Files:**
- Modify: `crates/shadcn-egui/src/components/select.rs`
- Modify: `crates/shadcn-egui/tests/overlays.rs`
- Create: `crates/shadcn-egui/tests/snapshots/select_open_{inverted,translucent}_{vega_light,vega_dark}.png`

**Interfaces:**
- Consumes: `Theme::menu_theme`, `Theme.menu`, `MenuColors::{TRANSLUCENT_FILL, TRANSLUCENT_HIGHLIGHT}` (Phase 4 Task 4); `Preset`, `Mode`, `MenuColor` (Phase 4); `common::snapshot_with_themes` (Phase 2 Task 4).
- Produces: private `select::list_fill(t: &Theme, recipe: Option<Color32>) -> Option<Color32>` and `select::row_fill(t: &Theme, highlighted: bool, recipe: Option<Color32>) -> Option<Color32>` (module-internal; unit-tested in `select.rs`).

- [ ] **Step 1: Write the failing unit test**

At the bottom of `crates/shadcn-egui/src/components/select.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::MenuColors;

    #[test]
    fn translucent_menus_fill_with_alpha_only() {
        let t = Theme::vega_light().menu(MenuColors { translucent: true, ..MenuColors::default() });
        let recipe = Some(Color32::RED);
        let fill = list_fill(&t, recipe).expect("a fill");
        assert_eq!(fill, t.palette.popover.gamma_multiply(MenuColors::TRANSLUCENT_FILL));
        assert!((170..=190).contains(&fill.a()), "about 70 % of an opaque popover: {}", fill.a());
        assert_eq!(row_fill(&t, true, recipe), Some(t.palette.foreground.gamma_multiply(MenuColors::TRANSLUCENT_HIGHLIGHT)));
        assert_eq!(row_fill(&t, false, recipe), recipe, "only the highlighted row changes");
        let opaque = Theme::vega_light();
        assert_eq!(list_fill(&opaque, recipe), recipe);
        assert_eq!(row_fill(&opaque, true, recipe), recipe);
    }
}
```

Run: `cargo test -p shadcn-egui --jobs 4 --lib select -- --test-threads 4`
Expected: FAIL to compile (`list_fill`, `row_fill` not found).

- [ ] **Step 2: Write the failing pixel test**

Append to `crates/shadcn-egui/tests/overlays.rs` (add the imports to the
top of the file):

```rust
use shadcn_egui::{MenuColor, Mode, Palette, Preset};

/// Pixels darker than (60, 60, 60) once the list of a light-mode Select is open.
fn dark_pixels_of_open_list(theme: Theme) -> usize {
    let bg = theme.palette.background;
    let mut h = Harness::builder().with_size(vec2(260.0, 260.0)).build_ui(move |ui| {
        theme.apply(ui.ctx());
        ui.ctx().layer_painter(egui::LayerId::background()).rect_filled(ui.ctx().viewport_rect(), 0.0, bg);
        let mut v = 1usize;
        ui.add(Select::new("fruit", &mut v, &FRUITS).width(180.0));
    });
    h.run();
    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run();
    let img = h.render().expect("wgpu render");
    img.pixels().filter(|p| p.0[0] < 60 && p.0[1] < 60 && p.0[2] < 60).count()
}

#[test]
fn an_inverted_menu_is_dark_in_light_mode() {
    let inverted = Preset::default().menu_color(MenuColor::Inverted).theme(Mode::Light);
    let mt = inverted.menu_theme();
    assert!(mt.dark && mt.palette == Palette::dark(), "the menu resolves as dark mode");
    let plain = dark_pixels_of_open_list(Preset::default().theme(Mode::Light));
    let dark = dark_pixels_of_open_list(inverted);
    // The open list is ~180 x 140 px: filled dark it adds well over 10 000
    // dark pixels; a light list only has its text.
    assert!(dark > 3 * plain + 5000, "inverted list must be dark: {dark} dark px, default list {plain}");
}
```

Run: `cargo test -p shadcn-egui --jobs 4 --test overlays an_inverted_menu -- --test-threads 4`
Expected: FAIL on the pixel assertion (the list still draws light).

- [ ] **Step 3: Implement**

In `crates/shadcn-egui/src/components/select.rs`, import
`use crate::theme::{family_for, MenuColors};` (replacing the
`use crate::theme::family_for;` line). At the start of the `if open {` block:

```rust
        if open {
            // Menu content draws with the menu's colours (upstream
            // `menuColor`): an inverted menu is dark, `dark:` rules included,
            // also in light mode. The trigger above keeps the page theme.
            let menu = t.menu_theme();
            let t = &menu;
```

The frame fill becomes:

```rust
            if let Some(fill) = list_fill(t, pick(&l, |r| &r.bg, gs).map(|p| t.paint(p))) {
                frame = frame.fill(fill);
            }
```

and the row fill:

```rust
                                let fill = row_fill(t, highlighted, pick(&item, |r| &r.bg, s).map(|p| t.paint(p)));
```

Every other `t.` inside the block (`state`, `paint`, `role`, `radius_px`,
`shadow`, `text_px`, `line_px`, `metrics`) now reads the menu theme through
the shadowed `t`; the item text and tick colour follow. If the compiler
reports a `&&Theme` where a function takes `&Theme`, pass `t` instead of `&t`.

Add above `/// Renders a select.`:

```rust
/// The list's fill: the recipe's, or for a translucent menu the popover
/// colour at [`MenuColors::TRANSLUCENT_FILL`] (upstream `TRANSLUCENT_CLASSES`
/// in the CLI's `transform-menu.ts`; alpha only, egui has no backdrop blur).
fn list_fill(t: &Theme, recipe: Option<Color32>) -> Option<Color32> {
    if t.menu.translucent {
        Some(t.palette.popover.gamma_multiply(MenuColors::TRANSLUCENT_FILL))
    } else {
        recipe
    }
}

/// A row's fill: the recipe's, or for the highlighted row of a translucent
/// menu the foreground at [`MenuColors::TRANSLUCENT_HIGHLIGHT`].
fn row_fill(t: &Theme, highlighted: bool, recipe: Option<Color32>) -> Option<Color32> {
    if t.menu.translucent && highlighted {
        Some(t.palette.foreground.gamma_multiply(MenuColors::TRANSLUCENT_HIGHLIGHT))
    } else {
        recipe
    }
}
```

No `// base:` quotes for these values: they come from the CLI's
`transform-menu.ts`, not from `ui/select`, and drift would flag them.

- [ ] **Step 4: Run the tests; existing snapshots unchanged**

```bash
cargo test -p shadcn-egui --jobs 4 --lib select -- --test-threads 4
cargo test -p shadcn-egui --jobs 4 --test overlays --test tokens -- --test-threads 4
cargo test --workspace --jobs 4 -- --test-threads 4
git status --short crates/shadcn-egui/tests/snapshots   # must print nothing
```

Expected: all pass; no PNG changed (with default `MenuColors`,
`menu_theme()` is the theme itself).

- [ ] **Step 5: Add the snapshot tests**

Append to `crates/shadcn-egui/tests/overlays.rs`:

```rust
/// An open Select in a preset's menu colours, light and dark. With `behind`,
/// lines of text run under the list so translucency shows.
fn select_open_in(name: &str, color: MenuColor, behind: bool) {
    let themes = vec![
        ("vega_light", Preset::default().menu_color(color).theme(Mode::Light)),
        ("vega_dark", Preset::default().menu_color(color).theme(Mode::Dark)),
    ];
    common::snapshot_with_themes(
        name,
        vec2(260.0, 260.0),
        themes,
        |ui| {
            let mut v = 1usize;
            ui.add(Select::new("fruit", &mut v, &FRUITS).width(180.0));
            if behind {
                for i in 1..=8 {
                    ui.label(format!("Text behind the menu, line {i}"));
                }
            } else {
                ui.add_space(160.0);
            }
            ui.label("elsewhere");
        },
        |h| {
            h.get_by_role(egui::accesskit::Role::ComboBox).click();
            h.run();
            h.get_by_label("elsewhere").hover();
            h.run();
        },
    );
}

#[test]
fn select_open_inverted_snapshot() {
    select_open_in("select_open_inverted", MenuColor::Inverted, false);
}

#[test]
fn select_open_translucent_snapshot() {
    select_open_in("select_open_translucent", MenuColor::DefaultTranslucent, true);
}
```

```bash
UPDATE_SNAPSHOTS=1 cargo test -p shadcn-egui --jobs 4 --test overlays select_open_ -- --test-threads 4
cargo test -p shadcn-egui --jobs 4 --test overlays -- --test-threads 4
git status --short crates/shadcn-egui/tests/snapshots
```

Expected: exactly four new PNGs
(`select_open_inverted_vega_{light,dark}.png`,
`select_open_translucent_vega_{light,dark}.png`); the `select_open_*`
baselines unchanged. Open each new PNG: the inverted list is dark on the
light page and looks like the dark-mode list; the translucent list shows
the text lines through it without blur, with the trigger still drawn in the
page theme. If "elsewhere" lies under the list, move it with more space;
the hover must not land on the list.

- [ ] **Step 6: Clippy and commit**

```bash
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
git add -A
git commit -m "feat: Select draws its list in the menu colours

An inverted menu draws dark, dark-mode rules included, also in light mode;
a translucent menu fills with the popover colour at 70 % and highlights
with the foreground at 10 %. New snapshots await review.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5.2: User review, docs, changelog

**Files:**
- Modify: `skills/egui-shadcn/references/gotchas.md`
- Modify: `CHANGES.md`

- [ ] **Step 1: STOP — user reviews the PNGs**

Show the user the four new PNGs (`git show --stat HEAD -- crates/shadcn-egui/tests/snapshots`)
and ask them to review and approve them. Do not continue until they
approve; apply their requested changes first (regenerate, look again, ask
again).

- [ ] **Step 2: Gotchas (D7)**

`skills/egui-shadcn/references/gotchas.md`, after the bullet that starts
`- **egui has no backdrop blur.**`:

```markdown
- **Translucent menus are alpha only.** With the menu colour
  `default-translucent` or `inverted-translucent` the Select list is the
  popover colour at 70 % alpha and the highlighted row the foreground at
  10 %. Upstream also blurs what lies behind the menu (`backdrop-blur-2xl`);
  egui cannot, so text behind the list shows through sharply.
```

- [ ] **Step 3: Changelog**

`CHANGES.md`, `## [Unreleased]` → `### Added`:

```markdown
- A preset's menu colour applies to the open Select list: `inverted` draws
  it dark also in light mode, and the translucent variants let the page show
  through it, without the blur the shadcn site adds.
```

- [ ] **Step 4: Phase gates and commit**

Run every gate from the index (both feature sets, clippy, tools, drift).

```bash
git add -A
git commit -m "docs: translucent menus are alpha only; changelog for menu colours

The new select_open_inverted and select_open_translucent snapshots were
reviewed and approved by the user.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
