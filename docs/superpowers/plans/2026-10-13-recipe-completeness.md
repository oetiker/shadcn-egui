# Recipe Completeness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every visual value of every component comes from the style data (a recipe or the theme metrics), so an app restyles any part through `theme.style`; this closes GitHub issue #6.

**Architecture:** Two pull requests. PR A widens snapshot coverage to all seven exported styles and adds a tabs state matrix, from today's code, so PR B has a baseline. PR B makes stylegen translate the full class string of each part (the svelte component's base classes with the style's `@apply` classes in place of the `cn-*` marker), adds the recipe fields this needs, moves every Kind 1 value of the components into recipes one component at a time with snapshots held identical, adds `Rule::important`, tightens `tests/tokens.rs` and updates the skills and CHANGES.

**Tech Stack:** Rust 2021 (MSRV 1.92), egui 0.34, egui_kittest 0.34 (wgpu, mesa software adapter), the `tools/stylegen` generator (own workspace, `serde_json` dev-dependency only).

**Spec:** [`docs/superpowers/specs/2026-10-13-recipe-completeness-design.md`](../specs/2026-10-13-recipe-completeness-design.md). Read it before any task; this plan argues from it.

## Global Constraints

- Every cargo command gets `--jobs 4`; every `cargo test` also `-- --test-threads 4`.
- Every `cargo test` and every benchmark run goes under a memory cap: `systemd-run --user --scope -p MemoryMax=4G -- <command>`.
- No `/tmp`. Scratch files and upstream checkouts go under `<scratch>`, a scratch directory outside the repository; worktrees go under `<worktrees>`; `<repo>` is the primary checkout.
- Generated files are never edited by hand: `crates/shadcn-egui/src/style/{generated,vega,nova,maia,lyra,mira,luma,rhea}.rs`, `tools/stylegen/reports/*.md`. Fix the generator and regenerate.
- Snapshot rule (every PR B task): all snapshots stay identical except the spec's "Known upstream disagreements" (luma and rhea tabs: no `Ring` border on focus, no `Input` border on the dark active tab). Any other snapshot difference is a STOP: show the maintainer the baseline PNG and the `.new.png` (and `.diff.png`) kittest wrote, wait for a decision, and only after approval add the case to the spec's "Known upstream disagreements" list and re-baseline with `UPDATE_SNAPSHOTS=1`.
- Upstream wins: where upstream and today's code disagree, the generated data is right; the STOP rule above decides whether a visible difference is accepted.
- The skills' rust fences compile as doctests (`skill_docs` in `crates/shadcn-egui/src/lib.rs`); an API change updates the skills in the same PR.
- Benchmark numbers are recorded in this plan's Ledger and in the commit message; the benchmark is not a CI gate.
- Commit messages: subject line, a body that says why, last line `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No em dashes in prose (docs, CHANGES, comments, commit messages).
- The repository is public: no private project names or paths in committed files other than this plan's command lines.
- Do not run anything from inside an upstream checkout; read its files, pass its path as an argument.
- PR B's tasks start from `main` after PR A merged (Task B0).

### Upstream checkouts (PR B, stylegen tasks)

```bash
SVELTE=<scratch>/upstream/shadcn-svelte
REACT=<scratch>/upstream/shadcn-ui
```

Task B0 creates both (see `skills/port-component/references/upstream.md`): shadcn-svelte at the commit in `tools/stylegen/PIN` (`493481fa`), shadcn-ui at `tools/stylegen/PIN_REACT` (`c2a67849`).

### Commands used in many tasks

```bash
# crate tests (snapshots compare against the committed PNGs)
systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4
# stylegen tests
systemd-run --user --scope -p MemoryMax=4G -- cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4
# regenerate (repository root)
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- generate "$SVELTE" "$REACT" .
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift "$SVELTE" "$REACT" .
# snapshot identity: nothing under snapshots/ modified
git status --short crates/shadcn-egui/tests/snapshots
```

A kittest snapshot test that fails writes `<name>.new.png` and `<name>.diff.png` next to the baseline in `crates/shadcn-egui/tests/snapshots/`. Those paths are what the STOP rule shows the maintainer.

## Review Focus

1. A style that sets a value the component's base classes also set under the same condition (for example a `gap-*` in `.cn-dialog-footer` next to the base `gap-2`): the base class comes later in the string, so it wins, as in the published component. Pinned by `base_classes_reach_every_style` (Task B5).
2. An app's `Rule::important` against an upstream `!` class without variants (luma and rhea `border-transparent!` on the tab trigger): equal rank, the later app rule wins. Pinned by `important_rule_beats_every_normal_rule` (Task B2) and `important_app_rule_beats_upstream_important` (Task B7).
3. A widget inside a disabled `Ui` (`ui.add_enabled(false, ..)`): faded once by egui, not again by the recipe's `disabled:opacity-50`. Pinned by `disabled_ui_fades_once` (Task B10).
4. An app that clears the separator's thickness rule (`separator.height = Prop::default()`): the line falls back to `metrics.border`, it does not vanish. Pinned by `separator_line_comes_from_the_style` (Task B9).
5. The line indicator of the active trigger in a disabled line list: faded with the trigger (opacity multiplies down the part tree), not drawn at full strength. Pinned by `disabled_line_bar_is_faded_with_its_trigger` (Task B7).

## Decision: which `{#if}` branch of `input.svelte` the extractor takes

`input.svelte` holds `cn-input` twice: the `{#if type === "file"}` branch (line 28) and the `{:else}` branch (line 41). `crates/shadcn-egui/src/components/input.rs:97` builds `TextEdit::singleline(self.text)` with an optional `.password(..)`: a text field, never a file input. The extractor takes the `{:else}` branch, occurrence 1 (0-based) of `cn-input` in `input/input.svelte`, recorded in `extract::BRANCHES`; `tests/extract.rs` asserts the entry and that the chosen occurrence is the `{:else}` one. Both branches carry the same class string at the pin, so the choice changes no value today; it decides what happens if they diverge after a pin bump.

## Ledger

Frame benchmark (`examples/bench_frame.rs`, release build, 3000 frames after 20 warm-up frames, one thread). Task B1 fills the "before" column, Task B16 the "after" column; both commits quote the table.

| style | median µs before | allocs/frame before | median µs after | allocs/frame after |
|---|---|---|---|---|
| vega | | | | |
| nova | | | | |
| maia | | | | |
| lyra | | | | |
| mira | | | | |
| luma | | | | |
| rhea | | | | |

Accepted upstream disagreements found during PR B (each with the maintainer's approval and the spec commit that lists it):

- (none yet)

## File Structure

PR A:
- Modify `crates/shadcn-egui/tests/common/mod.rs`: `snapshot_themes()` (seven styles, dark and light); `snapshot_all` and `snapshot_all_with` use it; `themes()` stays vega and nova.
- Modify `crates/shadcn-egui/tests/components.rs`: tabs state matrix snapshots.
- Create PNGs in `crates/shadcn-egui/tests/snapshots/`.

PR B, stylegen (`tools/stylegen/`):
- Create `src/extract.rs`: class strings of svelte sources, part association, `BRANCHES`.
- Create `src/build.rs`: everything `generate` writes for the styles, as strings (shared by `main.rs` and the tests).
- Modify `src/markers.rs` (`is_cli_marker`), `src/sources.rs` (svelte fixtures), `src/drift.rs` (fixture check of ui files, `slot:` skip, later `base_quotes` removal), `src/model.rs` (three fields), `src/utility.rs`, `src/variant.rs`, `src/resolve.rs` (`Part`, `parts`, `expand`, `EXCEPTIONS`, `after` decls), `src/emit.rs` (`fields`, slot and after parts), `src/report.rs`, `src/main.rs`, `src/lib.rs`, `markers.txt`.
- Create `tests/extract.rs`; modify `tests/golden.rs`, `tests/generated.rs`, `tests/published.rs`, `tests/drift.rs`, `tests/translate.rs` (if a changed ignore breaks one of its cases), `tests/fixtures/published-allow.txt`.
- Create `tests/fixtures/ui/<dir>/*.svelte` (byte copies, written by `generate`).

PR B, crate (`crates/shadcn-egui/`):
- Modify `src/style/recipe.rs`: `height_inset`, `outline_width`, `outline_color`; `When::all`; `Rule::IMPORTANT`, `Rule::important`; doc comment.
- Modify `src/style/mod.rs`: tests of generated data.
- Modify `src/components/shared.rs`: `opacity`, `style_painter`, `DEFAULT_SIDE_OFFSET`; later remove `disabled_painter`.
- Modify every component file in `src/components/` (one task each, see below).
- Create `examples/bench_frame.rs`; modify `Cargo.toml` (`[[example]]`).
- Create `tests/recipes.rs`: one test per component that a value comes from the style, and the issue #6 test.
- Modify `tests/common/mod.rs` (`rect_shapes`, `text_colour`, `restyled` move here), `tests/components.rs` (use them), `tests/tokens.rs`.
- Modify skills: `skills/build-ui/SKILL.md`, `skills/build-ui/references/component-map.md`, `skills/port-component/SKILL.md`, `skills/port-component/references/{upstream,styles,in-app}.md`; `CHANGES.md`.

---

# Part 1: Pull request A (snapshot coverage)

PR A is built on the current branch `feat/recipe-completeness` in `<worktrees>/shadcn-egui-recipes`, which already holds the spec and this plan. It changes tests only.

### Task A1: Seven-style snapshot theme list

**Files:**
- Modify: `crates/shadcn-egui/tests/common/mod.rs:11-49`
- Create: `crates/shadcn-egui/tests/snapshots/*_{maia,lyra,mira,luma,rhea}_{dark,light}.png`

**Interfaces:**
- Consumes: `Style::{vega,nova,maia,lyra,mira,luma,rhea}`, `Theme::new(Style, Palette, f32)`.
- Produces: `pub fn snapshot_themes() -> Vec<(&'static str, Theme)>` in `tests/common/mod.rs`; `snapshot_all` and `snapshot_all_with` render one PNG per entry, named `{name}_{style}_{mode}.png`. `themes()` keeps returning vega and nova only (the measurement tests, such as `button_height_follows_style`, expect 32 for nova and 36 for the rest).

- [ ] **Step 1: Add `snapshot_themes` and switch the snapshot helpers to it**

In `crates/shadcn-egui/tests/common/mod.rs`, below `themes()`, add:

```rust
/// The seven exported styles, dark and light: what every component snapshot
/// covers. Measurement tests use [`themes`] (vega and nova) instead.
pub fn snapshot_themes() -> Vec<(&'static str, Theme)> {
    type Make = fn() -> Style;
    const STYLES: [(&str, &str, Make); 7] = [
        ("vega_dark", "vega_light", Style::vega),
        ("nova_dark", "nova_light", Style::nova),
        ("maia_dark", "maia_light", Style::maia),
        ("lyra_dark", "lyra_light", Style::lyra),
        ("mira_dark", "mira_light", Style::mira),
        ("luma_dark", "luma_light", Style::luma),
        ("rhea_dark", "rhea_light", Style::rhea),
    ];
    STYLES
        .iter()
        .flat_map(|(dark, light, make)| {
            [
                (*dark, Theme::new(make(), Palette::dark(), 10.0)),
                (*light, Theme::new(make(), Palette::light(), 10.0)),
            ]
        })
        .collect()
}
```

Change the doc comment of `snapshot_all` to `/// One snapshot per exported style x {dark, light}: `{name}_{vega_dark|...}.png`.` and, in `snapshot_all_with`, replace `snapshot_with_themes(name, size, themes(), ui_fn, prepare);` with:

```rust
    snapshot_with_themes(name, size, snapshot_themes(), ui_fn, prepare);
```

The radius stays 10 for every style, as in `themes()`: the snapshots are a regression baseline for the recipes, not a replica of each style's preset.

- [ ] **Step 2: Run the snapshot tests to see the new images missing**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: FAIL in every `*_snapshot` test that uses `snapshot_all` / `snapshot_all_with` (button, input, tabs, badge, label_separator, card_*, checkbox, switch, slider, icons, dialog, popover, tooltip, select_trigger, select_open, settings_*), each naming missing `*_maia_*`, `*_lyra_*`, `*_mira_*`, `*_luma_*`, `*_rhea_*` images. No vega or nova image fails.

- [ ] **Step 3: Write the new baselines**

Run: `systemd-run --user --scope -p MemoryMax=4G -- env UPDATE_SNAPSHOTS=1 cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS.

- [ ] **Step 4: Check that no existing baseline changed**

Run: `git status --short crates/shadcn-egui/tests/snapshots | grep -v '^??'`
Expected: no output (only new, untracked files). If a vega or nova PNG shows as modified, stop: the helper change altered existing output; restore it with `git checkout -- <file>` and find out why before going on.

Run: `git status --short crates/shadcn-egui/tests/snapshots | wc -l`
Expected: 210 (21 snapshot names x 5 new styles x 2 modes). A different count means a snapshot name was missed or added; list them and explain the difference in the commit message.

- [ ] **Step 5: Run the tests again without UPDATE_SNAPSHOTS**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/shadcn-egui/tests/common/mod.rs crates/shadcn-egui/tests/snapshots
git commit -m "tests: snapshot every component in all seven exported styles

The recipe-completeness work moves every component value into the style
data. Snapshots of vega and nova alone would not show a change that only
maia, lyra, mira, luma or rhea draw differently. The measurement tests
keep their vega/nova list, since their expectations are per style.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task A2: Tabs state matrix

**Files:**
- Modify: `crates/shadcn-egui/tests/components.rs` (after `tabs_snapshot`, around line 412)
- Create: `crates/shadcn-egui/tests/snapshots/tabs_matrix_{default,line}_*.png`

**Interfaces:**
- Consumes: `common::snapshot_all_with` (Task A1), `Tabs`, `TabsVariant`.
- Produces: snapshot names `tabs_matrix_default` and `tabs_matrix_line`, which PR B's tabs task (B7) must keep identical except the listed luma and rhea disagreement.

- [ ] **Step 1: Add the matrix tests**

Append to `crates/shadcn-egui/tests/components.rs`, after `tabs_snapshot`:

```rust
/// Every trigger state of one tabs variant: in an enabled list the active,
/// an inactive and a hovered trigger; a disabled list (its active and its
/// inactive trigger); and a focused active trigger.
fn tabs_matrix(variant: shadcn_egui::components::tabs::TabsVariant) -> impl FnMut(&mut egui::Ui) {
    use shadcn_egui::components::tabs::Tabs;
    move |ui| {
        ui.add(Tabs::new(&mut 0, &["Active", "Inactive", "Hovered"]).variant(variant));
        ui.add_space(24.0);
        ui.add(
            Tabs::new(&mut 0, &["Disabled", "Off"])
                .variant(variant)
                .enabled(false),
        );
        ui.add_space(24.0);
        let mut sel = 0;
        let resp = ui.add(Tabs::new(&mut sel, &["Focused", "Next"]).variant(variant));
        ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", 0))));
    }
}

#[test]
fn tabs_matrix_default_snapshot() {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::tabs::TabsVariant;
    common::snapshot_all_with(
        "tabs_matrix_default",
        egui::vec2(360.0, 220.0),
        tabs_matrix(TabsVariant::Default),
        |h| {
            h.get_by_label("Hovered").hover();
            h.run();
        },
    );
}

#[test]
fn tabs_matrix_line_snapshot() {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::tabs::TabsVariant;
    common::snapshot_all_with(
        "tabs_matrix_line",
        egui::vec2(360.0, 220.0),
        tabs_matrix(TabsVariant::Line),
        |h| {
            h.get_by_label("Hovered").hover();
            h.run();
        },
    );
}
```

- [ ] **Step 2: Run them to see the images missing**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --test components tabs_matrix -- --test-threads 4`
Expected: FAIL, missing `tabs_matrix_default_*` and `tabs_matrix_line_*` images.

- [ ] **Step 3: Write the baselines**

Run: `systemd-run --user --scope -p MemoryMax=4G -- env UPDATE_SNAPSHOTS=1 cargo test -p shadcn-egui --jobs 4 --test components tabs_matrix -- --test-threads 4`
Expected: PASS, 28 new PNGs.

- [ ] **Step 4: Look at four of them**

Open `tabs_matrix_default_vega_light.png`, `tabs_matrix_line_vega_dark.png`, `tabs_matrix_default_luma_dark.png`, `tabs_matrix_line_rhea_light.png` with the Read tool. Check: three rows fit inside 360 x 220 without clipping; the "Hovered" trigger shows hover text colour; the disabled row is faded; the focused row shows the focus ring. If a row is clipped, raise the height in both tests and redo Step 3.

- [ ] **Step 5: Run the whole crate suite**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/shadcn-egui/tests/components.rs crates/shadcn-egui/tests/snapshots
git commit -m "tests: snapshot the tabs trigger in every state

The tab trigger takes the most state-dependent values (issue #6). A matrix
of default and line variant, inactive, hovered, active, disabled and
focused active, in light and dark and all seven styles, gives the recipe
move a baseline for each rule the trigger has.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task A3: Gates, maintainer review, pull request A

**Files:** none changed unless a gate fails.

- [ ] **Step 1: Run the gates that PR A touches**

```bash
systemd-run --user --scope -p MemoryMax=4G -- cargo test --workspace --jobs 4 -- --test-threads 4
systemd-run --user --scope -p MemoryMax=4G -- cargo test --workspace --all-features --jobs 4 -- --test-threads 4
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
cargo clippy --workspace --all-targets --all-features --jobs 4 -- -D warnings
cargo fmt --all --check
scripts/check-package-size.sh
```
Expected: all pass, clippy warning-free. `check-package-size.sh` must still pass: the crate packages `src/**` only, so the new PNGs do not count.

- [ ] **Step 2: STOP: maintainer reviews the new PNGs**

Report to the maintainer: the count of new PNGs per snapshot name, the path of the snapshot directory, and the four matrix images opened in Task A2 Step 4. Ask them to look at the new maia, lyra, mira, luma and rhea images and at the tabs matrix. These images become the baseline PR B must hold, so a wrong-looking image now (a style that draws a component badly today) is a bug to report, not to fix in PR A. Wait for their go-ahead before Step 3. List every problem they name as a separate issue to offer after PR A.

- [ ] **Step 3: Open pull request A**

```bash
git push -u origin feat/recipe-completeness
gh pr create --title "Snapshot coverage: all seven styles, tabs state matrix" --body "$(cat <<'EOF'
Tests only. Widens `snapshot_all` from vega and nova to the seven exported
styles and adds a tabs trigger state matrix (default and line; inactive,
hovered, active, disabled; focused active; light and dark), from the
current code. This is the baseline for the recipe-completeness work (#6),
whose spec and plan are in docs/superpowers/.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
gh pr edit --add-label no-changelog
```

The `no-changelog` label is the repository's escape hatch for changes a user does not see (`.github/workflows/changelog.yml`). PR A changes tests and docs only.

- [ ] **Step 4: Wait for the merge**

PR B does not start until PR A is merged into `main`.

---

# Part 2: Pull request B (recipes, stylegen, components, closes #6)

### Task B0: Worktree from main, upstream checkouts

**Files:** none.

- [ ] **Step 1: Create the PR B worktree from the merged main**

```bash
git -C <repo> fetch origin
git -C <repo> worktree add <worktrees>/shadcn-egui-recipe-data -b feat/recipe-data origin/main
cd <worktrees>/shadcn-egui-recipe-data
git log --oneline -3
ls crates/shadcn-egui/tests/snapshots | grep -c tabs_matrix
```
Expected: the log shows PR A's merge; the count is 28. Every later PR B command runs in this worktree.

- [ ] **Step 2: Clone the pinned upstream checkouts**

```bash
mkdir -p <scratch>/upstream
SVELTE=<scratch>/upstream/shadcn-svelte
REACT=<scratch>/upstream/shadcn-ui
git clone --filter=blob:none https://github.com/huntabyte/shadcn-svelte "$SVELTE"
git -C "$SVELTE" checkout "$(cat tools/stylegen/PIN)"
git clone --filter=blob:none https://github.com/shadcn-ui/ui "$REACT"
git -C "$REACT" checkout "$(cat tools/stylegen/PIN_REACT)"
git -C "$SVELTE" rev-parse HEAD
git -C "$REACT" rev-parse HEAD
```
Expected: the hashes start with `493481fa` and `c2a67849`. These checkouts are data: read files from them, never run a script or build inside them.

- [ ] **Step 3: Confirm a clean start**

```bash
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift "$SVELTE" "$REACT" .
systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4
```
Expected: `drift: clean`; all tests pass. A red test here is a bug on main: report it to the maintainer before going on.

### Task B1: Frame benchmark, first measurement

**Files:**
- Create: `crates/shadcn-egui/examples/bench_frame.rs`
- Modify: `crates/shadcn-egui/Cargo.toml:205-210` (`[[example]]` list)
- Modify: `docs/superpowers/plans/2026-10-13-recipe-completeness.md` (Ledger, "before" columns)

**Interfaces:**
- Consumes: `shadcn_egui::reference::{gallery_ui, GalleryState}`, `GalleryState::new(Preset, Mode)`, `Preset::style(StyleName)`.
- Produces: `cargo run --release -p shadcn-egui --example bench_frame --jobs 4 -- [frames]`, printing one line per style: name, median microseconds per frame, heap allocations per frame. Task B16 runs it again.

- [ ] **Step 1: Write the example**

`crates/shadcn-egui/examples/bench_frame.rs`:

```rust
//! Frame cost of the gallery screen in each exported style: the median time
//! and the heap allocations per frame, in a headless `egui::Context` on one
//! thread. A manual tool, not a test:
//! `cargo run --release --example bench_frame --jobs 4 [-- <frames>]`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use shadcn_egui::reference::{gallery_ui, GalleryState};
use shadcn_egui::{Mode, Preset, StyleName};

/// Counts every allocation and reallocation; frees are not counted.
struct Counting;

static ALLOCS: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

const STYLES: [(&str, StyleName); 7] = [
    ("vega", StyleName::Vega),
    ("nova", StyleName::Nova),
    ("maia", StyleName::Maia),
    ("lyra", StyleName::Lyra),
    ("mira", StyleName::Mira),
    ("luma", StyleName::Luma),
    ("rhea", StyleName::Rhea),
];

/// Frames run before measuring: fonts reach the atlas, layouts settle.
const WARMUP: usize = 20;

fn input() -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1420.0),
        )),
        ..Default::default()
    }
}

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(3000);
    println!("| style | median µs | allocs/frame |  ({frames} frames)");
    for (name, style) in STYLES {
        let ctx = egui::Context::default();
        let mut state = GalleryState::new(Preset::default().style(style), Mode::Light);
        for _ in 0..WARMUP {
            let _ = ctx.run_ui(input(), |ui| gallery_ui(ui, &mut state));
        }
        let mut times: Vec<Duration> = Vec::with_capacity(frames);
        let before = ALLOCS.load(Ordering::Relaxed);
        for _ in 0..frames {
            let raw = input();
            let start = Instant::now();
            let _ = ctx.run_ui(raw, |ui| gallery_ui(ui, &mut state));
            times.push(start.elapsed());
        }
        let allocs = ALLOCS.load(Ordering::Relaxed) - before;
        times.sort();
        let median = times[times.len() / 2];
        println!(
            "| {name} | {:.1} | {:.0} |",
            median.as_secs_f64() * 1e6,
            allocs as f64 / frames as f64
        );
    }
}
```

In `crates/shadcn-egui/Cargo.toml`, after the `gallery` example entry, add:

```toml
[[example]]
name = "bench_frame"
```

- [ ] **Step 2: Check it builds warning-free**

Run: `cargo clippy -p shadcn-egui --example bench_frame --jobs 4 -- -D warnings`
Expected: no warnings.

- [ ] **Step 3: Measure**

Run (machine otherwise as quiet as you can see it; note the load in the commit message):
`systemd-run --user --scope -p MemoryMax=4G -- cargo run --release -p shadcn-egui --example bench_frame --jobs 4 -- 3000`
Expected: seven table rows. Run it twice; if a median differs by more than 10 % between the runs, run a third time and record the middle run.

- [ ] **Step 4: Record the numbers in the Ledger**

Fill the "median µs before" and "allocs/frame before" columns of the Ledger table in this plan file.

- [ ] **Step 5: Commit**

```bash
git add crates/shadcn-egui/examples/bench_frame.rs crates/shadcn-egui/Cargo.toml docs/superpowers/plans/2026-10-13-recipe-completeness.md
git commit -m "examples: frame benchmark of the gallery in each style

The recipe work makes every component pick more rules per frame. The
benchmark measures the median frame time and the allocations per frame
before and after, as a manual tool (no new dependency, no CI gate).

Before (3000 frames, release, one thread):
<paste the seven table rows here>

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B2: `Rule::important`, `When::all`, opacity helpers, shared test helpers

**Files:**
- Modify: `crates/shadcn-egui/src/style/recipe.rs` (`When` impl ~line 87-104, `Rule` ~106-116, tests)
- Modify: `crates/shadcn-egui/src/style/mod.rs` (tests)
- Modify: `crates/shadcn-egui/src/components/shared.rs`
- Modify: `crates/shadcn-egui/tests/common/mod.rs`, `crates/shadcn-egui/tests/components.rs` (move `rect_shapes`, `text_colour`)

**Interfaces:**
- Produces (crate, public):
  - `impl When { pub const fn all(all: State) -> When }`
  - `impl<T> Rule<T> { pub const IMPORTANT: u128; pub fn important(when: When, value: T) -> Rule<T> }`. Spelling note: a bare `Rule::IMPORTANT` does not compile (E0282, the generic `T` cannot be inferred); write `Rule::<Paint>::IMPORTANT` (any `T`). `Rule::important(..)` infers `T` from `value`.
  - `shared::opacity(chain: &[&[&Recipe]], s: State) -> f32`: product of each stack's `opacity` under `s`, 1 where a stack sets none.
  - `shared::style_painter(ui: &Ui, opacity: f32) -> Painter`: the `Ui` painter faded by `opacity` when the `Ui` is enabled; unchanged in a disabled `Ui` (egui faded it already).
  - `shared::DEFAULT_SIDE_OFFSET: f32 = 4.0`.
- Produces (tests): `common::rect_shapes(&FullOutput) -> Vec<RectShape>`, `common::text_colour(&FullOutput) -> Color32`, `common::restyled(Theme, impl FnOnce(&mut Style)) -> Theme`.

- [ ] **Step 1: Write the failing recipe tests**

In `crates/shadcn-egui/src/style/recipe.rs`, inside `mod tests`, add:

```rust
    #[test]
    fn important_rule_beats_every_normal_rule() {
        // dark:data-active:bg-input/30 has two high variant bits; an app's
        // important rule under SELECTED alone still wins.
        let mut p = Prop {
            base: Some(0),
            rules: vec![rule(State::DARK.or(State::SELECTED), (1 << 120) | (1 << 100), 1)],
        };
        p.rules
            .push(Rule::important(When::all(State::SELECTED), 2));
        assert_eq!(p.get(State::DARK | State::SELECTED), Some(2));
        assert_eq!(p.get(State::DARK), Some(0));
        // An upstream `!` class without variants (luma `border-transparent!`)
        // has the same rank: the later rule, the app's, wins.
        let mut q = Prop {
            base: None,
            rules: vec![rule(State::NONE, Rule::<i32>::IMPORTANT, 1)],
        };
        q.rules
            .push(Rule::important(When::all(State::SELECTED), 2));
        assert_eq!(q.get(State::SELECTED), Some(2));
        assert_eq!(q.get(State::NONE), Some(1));
    }

    #[test]
    fn when_all_needs_every_flag() {
        let w = When::all(State::SELECTED | State::DARK);
        assert!(w.matches(State::SELECTED | State::DARK | State::HOVER));
        assert!(!w.matches(State::SELECTED));
        assert_eq!(w.none, State::NONE);
    }
```

In `crates/shadcn-egui/src/style/mod.rs`, inside `mod tests`, add:

```rust
    /// stylegen ranks an `!important` class with bit 127; `Rule::IMPORTANT`
    /// must be that bit, or an app's important rule would lose to upstream's.
    #[test]
    fn generated_important_rank_is_rule_important() {
        let luma = Style::luma();
        let r = luma
            .tabs_trigger
            .border_color
            .rules
            .iter()
            .find(|r| r.when == When::ALWAYS && r.value == Paint::Role(Role::Transparent, 1.0))
            .expect("luma cn-tabs-trigger border-transparent!");
        assert_eq!(r.rank, Rule::<Paint>::IMPORTANT);
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --lib --jobs 4 -- --test-threads 4 style::`
Expected: compile error: no function `important` / `all`, no associated const `IMPORTANT`.

- [ ] **Step 3: Implement**

In `recipe.rs`, add to `impl When`:

```rust
    /// Matches when every flag in `all` is set, whatever else is.
    pub const fn all(all: State) -> When {
        When {
            all,
            none: State::NONE,
        }
    }
```

After `pub struct Rule<T>`, add:

```rust
impl<T> Rule<T> {
    /// The rank bit of an `!important` class (`bg-primary!`). A rule with
    /// it beats every rule without it that also matches, whatever their
    /// variants. Spell it `Rule::<Paint>::IMPORTANT` (any `T`).
    pub const IMPORTANT: u128 = 1 << 127;

    /// An app's override, ranked like a Tailwind `!` class: it beats every
    /// generated rule that is not important. Against an upstream `!` class
    /// of the same rank it wins because it comes later in the rule list.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use shadcn_egui::style::{Paint, Role, Rule, State, When};
    /// let mut theme = shadcn_egui::Theme::vega_light();
    /// let style = Arc::make_mut(&mut theme.style);
    /// style.tabs_trigger.bg.rules.push(Rule::important(
    ///     When::all(State::SELECTED),
    ///     Paint::Role(Role::Primary, 1.0),
    /// ));
    /// ```
    pub fn important(when: When, value: T) -> Rule<T> {
        Rule {
            when,
            rank: Self::IMPORTANT,
            value,
        }
    }
}
```

In `crates/shadcn-egui/src/components/shared.rs`, change the `use` lines to `use crate::style::{pick, Recipe, Shift, State};` and add:

```rust
/// CSS's initial `opacity`.
const INITIAL_OPACITY: f32 = 1.0; // behaviour-ok: CSS initial value of opacity

/// shadcn's `bits-ui` `sideOffset` default of `SelectContent` and
/// `PopoverContent`: a component prop, not a class.
pub const DEFAULT_SIDE_OFFSET: f32 = 4.0; // behaviour-ok: upstream prop default, not a class

/// A part's `opacity` under `s`, multiplied down the part tree as CSS does:
/// `chain` is the part's recipe stack, then its parent's, and so on.
pub fn opacity(chain: &[&[&Recipe]], s: State) -> f32 {
    chain
        .iter()
        .map(|stack| pick(stack, |r| &r.opacity, s).unwrap_or(INITIAL_OPACITY))
        .product()
}

/// The `Ui`'s painter, faded by `opacity` (a part's resolved [`opacity`],
/// for example the style's `disabled:opacity-50`). In a disabled `Ui`
/// (`ui.add_enabled(false, ..)`, `Ui::disable`) egui has faded the painter
/// already, and fading again would paint at a quarter, so there it is left
/// as it is. egui fades shape by shape: where a part lies on another (a
/// switch thumb on its track) the lower part shows through, which CSS
/// opacity on the whole group would not show.
pub fn style_painter(ui: &Ui, opacity: f32) -> Painter {
    let mut painter = ui.painter().clone();
    if ui.is_enabled() {
        painter.multiply_opacity(opacity);
    }
    painter
}
```

Keep `disabled_painter` for now; Task B16 removes it once no component calls it.

- [ ] **Step 4: Move the test helpers into `common`**

Cut `rect_shapes` (components.rs ~line 130-143) and `text_colour` (~line 441-461) from `crates/shadcn-egui/tests/components.rs` and paste them, made `pub`, into `crates/shadcn-egui/tests/common/mod.rs`. Add there:

```rust
/// `theme` with `f` applied to its style: what an app does in its setup.
pub fn restyled(mut theme: Theme, f: impl FnOnce(&mut Style)) -> Theme {
    f(std::sync::Arc::make_mut(&mut theme.style));
    theme
}
```

In `components.rs`, replace each call `rect_shapes(` with `common::rect_shapes(` and `text_colour(` with `common::text_colour(`.

- [ ] **Step 5: Run the tests**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS, including the three new tests and the doctest of `Rule::important`. `git status --short crates/shadcn-egui/tests/snapshots` prints nothing.

- [ ] **Step 6: Commit**

```bash
git add crates/shadcn-egui/src crates/shadcn-egui/tests
git commit -m "style: Rule::important and When::all for app overrides

A rule's rank is a Tailwind variant bitmask an app cannot work out by
hand, so an app could not be sure its rule beats one such as
dark:data-active:bg-input/30. Rule::important ranks an app rule like a
Tailwind ! class. Also adds the opacity and style_painter helpers the
component tasks use, and moves two test helpers into tests/common.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B3: Stylegen copies the svelte component sources

**Files:**
- Modify: `tools/stylegen/src/markers.rs` (`is_cli_marker`)
- Modify: `tools/stylegen/src/sources.rs` (`UI`, `svelte_files`, `refresh`)
- Modify: `tools/stylegen/src/drift.rs` (`upstream_markers` uses `is_cli_marker`; `check_ui`)
- Modify: `tools/stylegen/src/main.rs` (`refresh` call, `drift`)
- Modify: `tools/stylegen/tests/drift.rs`
- Create: `tools/stylegen/tests/fixtures/ui/<dir>/*.svelte` (by `generate`)

**Interfaces:**
- Produces:
  - `markers::is_cli_marker(m: &str) -> bool`: true for `cn-font-heading`, `cn-menu-*`, `*-logical`, `cn-rtl-flip`, `cn-logical-sides`.
  - `sources::UI: &str = "docs/src/lib/registry/ui"`.
  - `sources::svelte_files(root: &Path, lines: &[MarkerLine]) -> std::io::Result<Vec<(String, String)>>`: `(dir, file)` of every `*.svelte` under `root/<dir>` for each dir in `markers.txt`, sorted.
  - `sources::refresh(up: &Path, up_react: &Path, tool: &Path, lines: &[MarkerLine]) -> std::io::Result<()>` (new `lines` parameter).
  - `drift::check_ui(up: &Path, tool: &Path, lines: &[MarkerLine]) -> Vec<String>`.

- [ ] **Step 1: Write the failing drift test**

Append to `tools/stylegen/tests/drift.rs`:

```rust
#[test]
fn reports_ui_copies_that_differ_missing_or_stale() {
    use stylegen::drift::check_ui;
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("stylegen-ui-{}", std::process::id()));
    let up = dir.join("up");
    let tool = dir.join("tool");
    let ui_up = up.join("docs/src/lib/registry/ui/tabs");
    let ui_ours = tool.join("tests/fixtures/ui/tabs");
    std::fs::create_dir_all(&ui_up).unwrap();
    std::fs::create_dir_all(&ui_ours).unwrap();
    for f in ["tabs.svelte", "tabs-list.svelte"] {
        std::fs::write(ui_up.join(f), f).unwrap();
        std::fs::write(ui_ours.join(f), f).unwrap();
    }
    let lines = parse_markers("tabs.rs tabs: cn-tabs\n");
    assert!(check_ui(&up, &tool, &lines).is_empty(), "identical copies are clean");

    std::fs::write(ui_up.join("tabs-list.svelte"), "changed").unwrap();
    std::fs::write(ui_up.join("tabs-trigger.svelte"), "new").unwrap();
    std::fs::write(ui_ours.join("tabs-old.svelte"), "gone upstream").unwrap();
    let problems = check_ui(&up, &tool, &lines);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(problems.len(), 3, "{problems:#?}");
    assert!(problems.iter().any(|p| p.contains("tabs-list.svelte differs")));
    assert!(problems.iter().any(|p| p.contains("tabs-trigger.svelte is missing")));
    assert!(problems.iter().any(|p| p.contains("tabs-old.svelte is no longer upstream")));
}

#[test]
fn cli_markers_are_named_in_one_place() {
    use stylegen::markers::is_cli_marker;
    for m in ["cn-font-heading", "cn-menu-target", "cn-select-content-logical", "cn-rtl-flip", "cn-logical-sides"] {
        assert!(is_cli_marker(m), "{m}");
    }
    assert!(!is_cli_marker("cn-select-content"));
}
```

- [ ] **Step 2: Run to see it fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 --test drift -- --test-threads 4`
Expected: compile error, `check_ui` and `is_cli_marker` not found.

- [ ] **Step 3: Implement**

`tools/stylegen/src/markers.rs`, append:

```rust
/// Markers only the shadcn CLI rewrites (fonts, menu targets, RTL and
/// logical-side helpers). The registry keeps some as text and drops the
/// classes of `*-logical`; the crate models none of them.
pub fn is_cli_marker(m: &str) -> bool {
    m == "cn-font-heading"
        || m.starts_with("cn-menu-")
        || m.ends_with("-logical")
        || m == "cn-rtl-flip"
        || m == "cn-logical-sides"
}
```

`tools/stylegen/src/drift.rs`, in `upstream_markers`, replace the `let cli = …;` expression with `let cli = crate::markers::is_cli_marker(m);`.

`tools/stylegen/src/sources.rs`, add (and `use crate::markers::MarkerLine; use std::collections::BTreeSet;`):

```rust
/// Where the component sources live in the shadcn-svelte checkout.
pub const UI: &str = "docs/src/lib/registry/ui";

/// `(dir, file)` of every `*.svelte` in `root/<dir>` for each component
/// directory `markers.txt` names, sorted.
pub fn svelte_files(root: &Path, lines: &[MarkerLine]) -> std::io::Result<Vec<(String, String)>> {
    let dirs: BTreeSet<&str> = lines.iter().map(|l| l.dir.as_str()).collect();
    let mut out = Vec::new();
    for dir in dirs {
        let mut names: Vec<String> = std::fs::read_dir(root.join(dir))?
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".svelte"))
            .collect();
        names.sort();
        out.extend(names.into_iter().map(|n| (dir.to_owned(), n)));
    }
    Ok(out)
}
```

Change `refresh` to take `lines: &[MarkerLine]` and, after the existing loop, copy the component sources:

```rust
    let ui = up.join(UI);
    for (dir, file) in svelte_files(&ui, lines)? {
        let to = tool.join("tests/fixtures/ui").join(&dir);
        std::fs::create_dir_all(&to)?;
        std::fs::copy(ui.join(&dir).join(&file), to.join(&file))?;
    }
    Ok(())
```

`tools/stylegen/src/drift.rs`, append:

```rust
/// Component source copies (`<tool>/tests/fixtures/ui/`) that differ from
/// the checkout, are missing, or are no longer upstream.
pub fn check_ui(up: &Path, tool: &Path, lines: &[MarkerLine]) -> Vec<String> {
    use crate::sources::{svelte_files, UI};
    let ours_root = tool.join("tests/fixtures/ui");
    let theirs = match svelte_files(&up.join(UI), lines) {
        Ok(t) => t,
        Err(e) => return vec![format!("{UI}: cannot list the upstream sources ({e})")],
    };
    let ours = svelte_files(&ours_root, lines).unwrap_or_default();
    let mut problems = Vec::new();
    for (dir, file) in &theirs {
        let a = std::fs::read(ours_root.join(dir).join(file));
        let b = std::fs::read(up.join(UI).join(dir).join(file));
        match (a, b) {
            (Ok(a), Ok(b)) if a == b => {}
            (Err(_), _) => problems.push(format!(
                "tests/fixtures/ui/{dir}/{file} is missing; re-run stylegen generate"
            )),
            _ => problems.push(format!(
                "tests/fixtures/ui/{dir}/{file} differs from {UI}/{dir}/{file}; re-run stylegen generate"
            )),
        }
    }
    for (dir, file) in ours.iter().filter(|f| !theirs.contains(f)) {
        problems.push(format!(
            "tests/fixtures/ui/{dir}/{file} is no longer upstream; remove it"
        ));
    }
    problems
}
```

`tools/stylegen/src/main.rs`: in `generate`, call `stylegen::sources::refresh(up, up_react, tool, lines)`; in `drift`, after `check_sources`, add `problems.extend(stylegen::drift::check_ui(up, tool, lines));`.

- [ ] **Step 4: Run the stylegen tests**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4`
Expected: PASS.

- [ ] **Step 5: Copy the sources and check drift**

```bash
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- generate "$SVELTE" "$REACT" .
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift "$SVELTE" "$REACT" .
git status --short
```
Expected: `drift: clean`; the only new paths are `tools/stylegen/tests/fixtures/ui/**`; no generated `.rs` or report changed (this task does not change translation).

- [ ] **Step 6: Commit**

```bash
git add tools/stylegen
git commit -m "stylegen: keep copies of the svelte component sources

The next steps translate each part's full class string, base classes
included, so the component sources become generator input. Like the style
CSS they are copied by generate and checked by drift, so tests and output
never describe stale sources.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B4: Extractor of part class strings

**Files:**
- Create: `tools/stylegen/src/extract.rs`
- Modify: `tools/stylegen/src/lib.rs` (`pub mod extract;`)
- Create: `tools/stylegen/tests/extract.rs`

**Interfaces:**
- Consumes: `markers::{is_cli_marker, MarkerLine, parse_markers}`.
- Produces:
  - `extract::class_strings(src: &str) -> Vec<String>`: every class string of a svelte source in source order, whitespace collapsed: the string arguments of each `cn(…)` joined with a space, the `base` string and each string in `variants` of a `tv({…})`, each static `class="…"` (also on a nested component such as `<Button class="cn-dialog-close">`). HTML comments and `//` comments are skipped; non-string arguments (`className`, `arrowClasses`) are skipped.
  - `extract::normalise(s: &str) -> String`; `extract::strip_cli_markers(s: &str) -> String`.
  - `extract::BRANCHES: &[(&str, &str, usize, &str)]`: `(file under ui/, marker, occurrence the crate draws, why)`.
  - `extract::part_strings(ui: &Path, lines: &[MarkerLine]) -> Result<Vec<(String, Vec<String>)>, String>`: `(part, classes)` in `markers.txt` order; a `cn-*` part's classes still hold the marker token; a `slot:<name>` part's classes are the only marker-less class string of `ui/<dir>/<name>.svelte`.

- [ ] **Step 1: Write the failing tests**

`tools/stylegen/tests/extract.rs`:

```rust
use stylegen::extract::{class_strings, part_strings, BRANCHES};
use stylegen::markers::parse_markers;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn ui() -> std::path::PathBuf {
    std::path::Path::new(ROOT).join("tests/fixtures/ui")
}

fn part(name: &str) -> String {
    let lines = parse_markers(&std::fs::read_to_string(format!("{ROOT}/markers.txt")).unwrap());
    let parts = part_strings(&ui(), &lines).expect("parts");
    parts
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("{name}"))
        .1
        .join(" ")
}

#[test]
fn reads_every_string_form() {
    let src = r#"
<script lang="ts" module>
	export const v = tv({
		base: "cn-x-base a b",
		variants: { size: { sm: "cn-x-sm c", "icon-xs": "cn-x-icon d" } },
		defaultVariants: { size: "sm" },
	});
</script>
<!-- parity-ignore: data-[x]:hidden | not a class string -->
<div class={cn(
	"cn-x e [&_svg:not([class*='size-'])]:size-4",
	// parity-ignore: a comment with "quotes"
	"f h-[calc(100%-1px)]",
	arrowClasses,
	className
)}></div>
<Button variant="ghost" class="cn-x-close g" />
"#;
    let s = class_strings(src);
    assert!(s.contains(&"cn-x-base a b".to_string()), "{s:?}");
    assert!(s.contains(&"cn-x-sm c".to_string()), "{s:?}");
    assert!(s.contains(&"cn-x-icon d".to_string()), "{s:?}");
    assert!(
        s.contains(&"cn-x e [&_svg:not([class*='size-'])]:size-4 f h-[calc(100%-1px)]".to_string()),
        "{s:?}"
    );
    assert!(s.contains(&"cn-x-close g".to_string()), "{s:?}");
    assert!(!s.iter().any(|x| x.contains("data-[x]:hidden")), "{s:?}");
    assert!(!s.iter().any(|x| x.contains("quotes")), "{s:?}");
}

#[test]
fn tabs_trigger_joins_its_four_cn_arguments() {
    let t = part("cn-tabs-trigger");
    assert!(t.starts_with("cn-tabs-trigger relative inline-flex h-[calc(100%-1px)]"), "{t}");
    assert!(t.ends_with("group-data-[variant=line]/tabs-list:data-active:after:opacity-100"), "{t}");
}

#[test]
fn tv_variant_strings_are_their_own_parts() {
    assert_eq!(part("cn-tabs-list-variant-line"), "cn-tabs-list-variant-line gap-1 bg-transparent");
    assert!(part("cn-button").starts_with("cn-button group/button inline-flex"));
    assert_eq!(part("cn-button-variant-ghost"), "cn-button-variant-ghost");
}

#[test]
fn cli_markers_are_removed() {
    assert_eq!(part("cn-card-title"), "cn-card-title");
    assert!(!part("cn-select-content").contains("cn-menu-"));
    assert!(!part("cn-popover-content").contains("-logical"));
}

#[test]
fn nested_component_class_is_the_part() {
    assert_eq!(part("cn-dialog-close"), "cn-dialog-close");
    assert_eq!(part("cn-select-item-indicator-icon"), "cn-select-item-indicator-icon pointer-events-none");
}

#[test]
fn slot_part_is_the_marker_less_string_of_its_file() {
    let lines = parse_markers("separator.rs separator: slot:separator\n");
    let parts = part_strings(&ui(), &lines).expect("slot part");
    assert_eq!(
        parts[0].1.join(" "),
        "shrink-0 bg-border data-horizontal:h-px data-horizontal:w-full data-vertical:w-px data-vertical:self-stretch"
    );
}

/// The decision in the plan: input.rs draws a text field, so the `{:else}`
/// branch of input.svelte (occurrence 1 of cn-input) is the part.
#[test]
fn input_takes_the_else_branch() {
    assert!(BRANCHES
        .iter()
        .any(|(f, m, k, _)| *f == "input/input.svelte" && *m == "cn-input" && *k == 1));
    let src = std::fs::read_to_string(ui().join("input/input.svelte")).unwrap();
    let hits: Vec<String> = class_strings(&src)
        .into_iter()
        .filter(|s| s.split_whitespace().any(|t| t == "cn-input"))
        .collect();
    assert_eq!(hits.len(), 2, "{hits:?}");
    let else_at = src.find("{:else}").unwrap();
    let second = src.rfind("\"cn-input ").unwrap();
    assert!(second > else_at, "occurrence 1 lies in the {{:else}} branch");
    assert_eq!(part("cn-input"), hits[1]);
}

#[test]
fn branch_choice_picks_the_named_occurrence_and_unnamed_duplicates_fail() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("stylegen-extract-{}", std::process::id()));
    let input = dir.join("input");
    std::fs::create_dir_all(&input).unwrap();
    std::fs::write(
        input.join("input.svelte"),
        "{#if file}<input class=\"cn-input only-file\" />{:else}<input class=\"cn-input only-text\" />{/if}",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("label")).unwrap();
    std::fs::write(
        dir.join("label/label.svelte"),
        "<a class=\"cn-label one\" /><a class=\"cn-label two\" />",
    )
    .unwrap();
    let ok = part_strings(&dir, &parse_markers("input.rs input: cn-input\n")).unwrap();
    assert_eq!(ok[0].1.join(" "), "cn-input only-text");
    let err = part_strings(&dir, &parse_markers("label.rs label: cn-label\n")).unwrap_err();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(err.contains("BRANCHES"), "{err}");
}

#[test]
fn every_shipped_part_has_a_string() {
    let lines = parse_markers(&std::fs::read_to_string(format!("{ROOT}/markers.txt")).unwrap());
    let parts = part_strings(&ui(), &lines).expect("every part found");
    let shipped: usize = lines.iter().map(|l| l.markers.len()).sum();
    assert_eq!(parts.len(), shipped);
}
```

- [ ] **Step 2: Run to see them fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 --test extract -- --test-threads 4`
Expected: compile error, module `extract` not found.

- [ ] **Step 3: Implement `src/extract.rs`**

```rust
//! Class strings of the shadcn-svelte component sources
//! (`docs/src/lib/registry/ui/<dir>/*.svelte`), and the string of each part.

use crate::markers::{is_cli_marker, MarkerLine};
use std::path::Path;

/// A part a component file draws in several `{#if}` branches:
/// `(file under ui/, marker, occurrence the crate draws (0-based), why)`.
pub const BRANCHES: &[(&str, &str, usize, &str)] = &[(
    "input/input.svelte",
    "cn-input",
    1,
    "input.rs draws a text field (egui TextEdit), never type=\"file\": the {:else} branch",
)];

/// Every class string in a component source, in source order: the string
/// arguments of each `cn(…)` joined with a space, the `base` string and each
/// string in `variants` of a `tv({…})`, and each static `class="…"`, also
/// on a nested component. HTML comments and `//` comments are skipped, and
/// so are non-string arguments (`className`, `arrowClasses`).
pub fn class_strings(src: &str) -> Vec<String> {
    let src = strip_html_comments(src);
    let mut found: Vec<(usize, String)> = Vec::new();
    for (at, args) in calls(&src, "cn") {
        found.push((at, top_level_strings(args).join(" ")));
    }
    for (at, args) in calls(&src, "tv") {
        for s in tv_strings(args) {
            found.push((at, s));
        }
    }
    let mut from = 0;
    while let Some(i) = src[from..].find("class=\"") {
        let start = from + i + "class=\"".len();
        let Some(len) = src[start..].find('"') else {
            break;
        };
        found.push((start, src[start..start + len].to_owned()));
        from = start + len + 1;
    }
    // Stable: the strings of one `tv` keep their order.
    found.sort_by_key(|(at, _)| *at);
    found
        .into_iter()
        .map(|(_, s)| normalise(&s))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Whitespace collapsed to single spaces.
pub fn normalise(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `s` without the markers only the shadcn CLI rewrites.
pub fn strip_cli_markers(s: &str) -> String {
    s.split_whitespace()
        .filter(|t| !(t.starts_with("cn-") && is_cli_marker(t)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The class string of every part in `lines`, read from
/// `<ui>/<dir>/*.svelte`: `(part, classes)` in `markers.txt` order. A
/// `cn-*` part's classes still hold its marker. Every problem is named in
/// the error: a part no string carries, a string with two shipped markers,
/// several differing strings for one part, several strings in one file
/// without a [`BRANCHES`] entry.
pub fn part_strings(
    ui: &Path,
    lines: &[MarkerLine],
) -> Result<Vec<(String, Vec<String>)>, String> {
    let shipped: Vec<&str> = lines
        .iter()
        .flat_map(|l| l.markers.iter().map(String::as_str))
        .filter(|m| !m.starts_with("slot:"))
        .collect();
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for line in lines {
        if line.markers.is_empty() {
            continue;
        }
        let files = svelte_sources(&ui.join(&line.dir))
            .map_err(|e| format!("ui/{}: {e}", line.dir))?;
        for part in &line.markers {
            match part_string(&line.dir, &files, part, &shipped) {
                Ok(s) => out.push((
                    part.clone(),
                    s.split_whitespace().map(str::to_owned).collect(),
                )),
                Err(e) => problems.push(e),
            }
        }
    }
    if problems.is_empty() {
        Ok(out)
    } else {
        Err(problems.join("\n"))
    }
}

/// `(file name, class strings without CLI markers)` of every `*.svelte`
/// in `dir`, by name.
fn svelte_sources(dir: &Path) -> std::io::Result<Vec<(String, Vec<String>)>> {
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".svelte"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|n| {
            let src = std::fs::read_to_string(dir.join(&n))?;
            let strings = class_strings(&src)
                .iter()
                .map(|s| strip_cli_markers(s))
                .collect();
            Ok((n, strings))
        })
        .collect()
}

fn part_string(
    dir: &str,
    files: &[(String, Vec<String>)],
    part: &str,
    shipped: &[&str],
) -> Result<String, String> {
    if let Some(slot) = part.strip_prefix("slot:") {
        let file = format!("{slot}.svelte");
        let strings = files
            .iter()
            .find(|(n, _)| *n == file)
            .map(|(_, s)| s)
            .ok_or_else(|| format!("{part}: no ui/{dir}/{file}"))?;
        let plain: Vec<&String> = strings
            .iter()
            .filter(|s| !s.split_whitespace().any(|t| t.starts_with("cn-")))
            .collect();
        return match plain.as_slice() {
            [one] => Ok((*one).clone()),
            _ => Err(format!(
                "{part}: ui/{dir}/{file} has {} class strings without a marker, expected one",
                plain.len()
            )),
        };
    }
    let mut chosen: Vec<String> = Vec::new();
    for (name, strings) in files {
        let hits: Vec<&String> = strings
            .iter()
            .filter(|s| s.split_whitespace().any(|t| t == part))
            .collect();
        let pick: &String = match hits.len() {
            0 => continue,
            1 => hits[0],
            n => {
                let path = format!("{dir}/{name}");
                let Some((_, _, k, _)) = BRANCHES
                    .iter()
                    .find(|(f, m, _, _)| *f == path && *m == part)
                else {
                    return Err(format!(
                        "{part}: ui/{path} has {n} class strings for it; name the one the crate draws in extract::BRANCHES"
                    ));
                };
                hits.get(*k).copied().ok_or_else(|| {
                    format!("{part}: BRANCHES names occurrence {k}, ui/{path} has {n}")
                })?
            }
        };
        let others: Vec<&str> = pick
            .split_whitespace()
            .filter(|t| *t != part && shipped.contains(t))
            .collect();
        if !others.is_empty() {
            return Err(format!(
                "{part}: its string in ui/{dir}/{name} also holds {others:?}"
            ));
        }
        if !chosen.contains(pick) {
            chosen.push(pick.clone());
        }
    }
    match chosen.len() {
        0 => Err(format!("{part}: no class string in ui/{dir}")),
        1 => Ok(chosen.remove(0)),
        _ => Err(format!(
            "{part}: differing class strings in ui/{dir}: {chosen:?}"
        )),
    }
}

fn strip_html_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(i) = rest.find("<!--") {
        out.push_str(&rest[..i]);
        rest = rest[i..].find("-->").map_or("", |j| &rest[i + j + 3..]);
    }
    out.push_str(rest);
    out
}

/// `(position, argument text)` of every call `name(…)` where `name` is not
/// the tail of a longer identifier or a method.
fn calls<'a>(src: &'a str, name: &str) -> Vec<(usize, &'a str)> {
    let pat = format!("{name}(");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = src[from..].find(&pat) {
        let at = from + i;
        from = at + pat.len();
        let joined = src[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '$');
        if joined {
            continue;
        }
        if let Some(end) = closing(src, from - 1) {
            out.push((at, &src[from..end]));
            from = end + 1;
        }
    }
    out
}

/// Index of the bracket closing the one at `open`, skipping strings and
/// `//` comments.
fn closing(src: &str, open: usize) -> Option<usize> {
    let b = src.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' | b'`' => i = skip_string(b, i)?,
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Index of the quote closing the string that starts at `i`.
fn skip_string(b: &[u8], i: usize) -> Option<usize> {
    let q = b[i];
    let mut j = i + 1;
    while j < b.len() {
        if b[j] == b'\\' {
            j += 2;
            continue;
        }
        if b[j] == q {
            return Some(j);
        }
        j += 1;
    }
    None
}

/// String literals at bracket depth 0 of `args`.
fn top_level_strings(args: &str) -> Vec<String> {
    strings(args, false)
}

/// String literals of `args`, at depth 0 only or at any depth.
fn strings(args: &str, any_depth: bool) -> Vec<String> {
    let b = args.as_bytes();
    let (mut out, mut depth, mut i) = (Vec::new(), 0i32, 0usize);
    while i < b.len() {
        match b[i] {
            b'"' | b'\'' | b'`' => {
                let Some(end) = skip_string(b, i) else {
                    break;
                };
                if any_depth || depth == 0 {
                    out.push(args[i + 1..end].to_owned());
                }
                i = end;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    out
}

/// The `base` string and every string in `variants` of a `tv({…})` call.
/// Quoted variant keys (`"icon-xs":`) come along; they hold no marker, so
/// no part takes them.
fn tv_strings(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(i) = args.find("base:") {
        out.extend(top_level_strings(&args[i + "base:".len()..]).into_iter().take(1));
    }
    if let Some(i) = args.find("variants:") {
        if let Some(open) = args[i..].find('{').map(|o| i + o) {
            if let Some(close) = closing(args, open) {
                out.extend(strings(&args[open..=close], true));
            }
        }
    }
    out
}
```

Add `pub mod extract;` to `tools/stylegen/src/lib.rs` (alphabetical, after `emit`).

- [ ] **Step 4: Run the tests**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4`
Expected: PASS (all stylegen tests; nothing uses the extractor yet).

- [ ] **Step 5: Commit**

```bash
git add tools/stylegen
git commit -m "stylegen: extract each part's class string from the svelte sources

The style CSS alone gives only the classes behind a cn-* marker. The base
classes the component sets itself (h-[calc(100%-1px)], disabled:opacity-50,
the tab line indicator) live in the svelte file. The extractor reads cn(),
tv() and class= strings the way the published registry builds them, and
names the one {#if} branch it takes (input: the text field).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B5: Full class strings, new fields, `after` and `slot` parts, regeneration

**Files:**
- Modify: `crates/shadcn-egui/src/style/recipe.rs` (three `Recipe` fields, module doc)
- Modify: `tools/stylegen/src/model.rs` (`Field::{HeightInset, OutlineWidth, OutlineColor}`)
- Modify: `tools/stylegen/src/utility.rs`, `tools/stylegen/src/variant.rs`
- Modify: `tools/stylegen/src/resolve.rs` (`Part`, `parts`, `expand`, `EXCEPTIONS`, `Marker.after`; remove `markers`)
- Modify: `tools/stylegen/src/emit.rs` (`field_name`, `fields`, `emit_struct`, `emit_style`, `recipe`)
- Modify: `tools/stylegen/src/report.rs` (takes `&[Part]`)
- Create: `tools/stylegen/src/build.rs`; modify `src/lib.rs`, `src/main.rs`
- Modify: `tools/stylegen/src/drift.rs` (`slot:` entries skipped), `tools/stylegen/markers.txt` (`slot:separator`)
- Modify: `tools/stylegen/tests/golden.rs`, `tools/stylegen/tests/generated.rs`, `tools/stylegen/tests/translate.rs` (only cases a changed ignore breaks)
- Modify: `crates/shadcn-egui/src/style/mod.rs` (test)
- Regenerate: `crates/shadcn-egui/src/style/*.rs` (generated), `tools/stylegen/reports/*.md`

**Interfaces:**
- Consumes: `extract::part_strings` (B4), `is_cli_marker` (B3).
- Produces (crate): `Recipe::height_inset: Prop<f32>`, `Recipe::outline_width: Prop<f32>`, `Recipe::outline_color: Prop<Paint>`; `Style::separator: Recipe` (`slot:separator`), `Style::tabs_trigger_after: Recipe` (`after:` classes of `cn-tabs-trigger`). Every generated recipe holds the base classes.
- Produces (stylegen):
  - `resolve::Part { pub name: String, pub marker: Marker }`; `Marker` gains `pub after: Vec<Decl>`.
  - `resolve::expand(component: &[String], part: &str, css: &[(String, Vec<String>)]) -> Vec<String>`.
  - `resolve::parts(css: &[(String, Vec<String>)], components: &[(String, Vec<String>)]) -> Vec<Part>`.
  - `resolve::EXCEPTIONS: &[(&str, &str, &str)]`: `(part, variant, why)`.
  - `utility::U::After(Vec<(Field, Value)>)`.
  - `emit::field_name(part: &str) -> String` (`slot:separator` gives `separator`), `emit::fields(styles: &[&[Part]]) -> Vec<(String, String)>`, `emit::emit_struct(pin: &str, fields: &[(String, String)]) -> String`, `emit::emit_style(style: &str, pin: &str, fields: &[(String, String)], parts: &[Part]) -> String`.
  - `report::report(style: &str, pin: &str, parts: &[Part]) -> String`.
  - `build::styles(tool: &Path, pin: &str) -> Result<build::Styles, String>` with `Styles { generated, exported, reports, failed }`.

- [ ] **Step 1: Write the failing golden tests**

Append to `tools/stylegen/tests/golden.rs` (add `use stylegen::extract::part_strings; use stylegen::markers::parse_markers; use stylegen::model::flag; use stylegen::resolve::{parts, Part, Vars};`):

```rust
fn classes(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn part_of(style_name: &str, part: &str) -> Part {
    let root = env!("CARGO_MANIFEST_DIR");
    let lines = parse_markers(&std::fs::read_to_string(format!("{root}/markers.txt")).unwrap());
    let comps = part_strings(&std::path::Path::new(root).join("tests/fixtures/ui"), &lines).unwrap();
    parts(&style(style_name), &comps)
        .into_iter()
        .find(|p| p.name == part)
        .unwrap_or_else(|| panic!("{part}"))
}

fn after_when(p: &Part, field: Field, all: u32) -> Option<Value> {
    p.marker
        .after
        .iter()
        .rfind(|d| d.field == field && d.all == all)
        .map(|d| d.value)
}

#[test]
fn outline_classes_set_the_outline_fields() {
    let m = translate(
        &classes(&["outline-none", "focus-visible:outline-1", "focus-visible:outline-ring"]),
        &Vars::new(),
    );
    assert_eq!(base(&m, Field::OutlineWidth), Some(Value::Px(0.0)));
    let fv = flag("FOCUS_VISIBLE");
    assert_eq!(when(&m, Field::OutlineWidth, fv), Some(Value::Px(1.0)));
    assert_eq!(
        when(&m, Field::OutlineColor, fv),
        Some(Value::Paint { role: "Ring", alpha: 1.0 })
    );
    let hidden = translate(&classes(&["outline-hidden"]), &Vars::new());
    assert_eq!(base(&hidden, Field::OutlineWidth), Some(Value::Px(0.0)));
}

#[test]
fn calc_height_is_a_height_inset() {
    let m = translate(&classes(&["h-[calc(100%-1px)]"]), &Vars::new());
    assert_eq!(base(&m, Field::HeightInset), Some(Value::Px(1.0)));
    assert_eq!(base(&m, Field::Height), None);
}

#[test]
fn tab_line_indicator_is_an_after_part() {
    let p = part_of("vega", "cn-tabs-trigger");
    let (h, line, sel) = (flag("HORIZONTAL"), flag("VARIANT_LINE"), flag("SELECTED"));
    assert_eq!(base(&p.marker, Field::HeightInset), Some(Value::Px(1.0)));
    let after_base = |f: Field| {
        p.marker
            .after
            .iter()
            .rfind(|d| d.field == f && d.all == 0 && d.rank == 0)
            .map(|d| d.value)
    };
    assert_eq!(after_base(Field::Opacity), Some(Value::Factor(0.0)));
    assert_eq!(
        after_base(Field::Bg),
        Some(Value::Paint { role: "Foreground", alpha: 1.0 })
    );
    assert_eq!(after_when(&p, Field::Height, h), Some(Value::Px(2.0)));
    assert_eq!(after_when(&p, Field::InsetBottom, h), Some(Value::Px(-5.0)));
    assert_eq!(after_when(&p, Field::InsetLeft, h), Some(Value::Px(0.0)));
    assert_eq!(after_when(&p, Field::Opacity, line | sel), Some(Value::Factor(1.0)));
    // The trigger's own decls hold none of these.
    assert_eq!(when(&p.marker, Field::Height, h), None);
}

#[test]
fn separator_is_a_slot_part() {
    let p = part_of("vega", "slot:separator");
    assert_eq!(
        base(&p.marker, Field::Bg),
        Some(Value::Paint { role: "Border", alpha: 1.0 })
    );
    assert_eq!(when(&p.marker, Field::Height, flag("HORIZONTAL")), Some(Value::Px(1.0)));
    assert_eq!(when(&p.marker, Field::Width, flag("VERTICAL")), Some(Value::Px(1.0)));
    assert!(p.marker.unsupported.is_empty(), "{:?}", p.marker.unsupported);
}

#[test]
fn slider_thumb_drops_disabled_classes_by_name() {
    let p = part_of("vega", "cn-slider-thumb");
    assert!(
        !p.marker.decls.iter().any(|d| d.field == Field::Opacity),
        "{:?}",
        p.marker.decls
    );
    assert!(p.marker.ignored.iter().any(|(c, _)| c == "disabled:opacity-50"));
}

#[test]
fn checkbox_and_switch_hit_area_come_from_the_base_classes() {
    for part in ["cn-checkbox", "cn-switch"] {
        let p = part_of("vega", part);
        assert_eq!(base(&p.marker, Field::HitInsetX), Some(Value::Px(12.0)), "{part}");
        assert_eq!(base(&p.marker, Field::HitInsetY), Some(Value::Px(8.0)), "{part}");
    }
}

#[test]
fn highlighted_item_and_placeholder_text_have_their_states() {
    let item = part_of("vega", "cn-select-item");
    assert_eq!(
        when(&item.marker, Field::Bg, flag("FOCUS")),
        Some(Value::Paint { role: "Accent", alpha: 1.0 })
    );
    let input = part_of("vega", "cn-input");
    assert_eq!(
        when(&input.marker, Field::Text, flag("PLACEHOLDER")),
        Some(Value::Paint { role: "MutedForeground", alpha: 1.0 })
    );
}

#[test]
fn luma_trigger_border_stays_important() {
    let p = part_of("luma", "cn-tabs-trigger");
    assert!(p
        .marker
        .decls
        .iter()
        .any(|d| d.field == Field::BorderColor && d.all == 0 && d.rank == 1 << 127));
}

#[test]
fn every_exported_style_translates_every_part() {
    for s in ["vega", "nova", "maia", "lyra", "mira", "luma", "rhea"] {
        let root = env!("CARGO_MANIFEST_DIR");
        let lines = parse_markers(&std::fs::read_to_string(format!("{root}/markers.txt")).unwrap());
        let comps = part_strings(&std::path::Path::new(root).join("tests/fixtures/ui"), &lines).unwrap();
        for p in parts(&style(s), &comps) {
            assert!(p.marker.unsupported.is_empty(), "{s} {}: {:?}", p.name, p.marker.unsupported);
        }
    }
}
```

In `crates/shadcn-egui/src/style/mod.rs` tests, add (Review Focus 1):

```rust
    /// Base classes come after the style's classes in each part's string,
    /// so they win under the same condition, as in the published component.
    #[test]
    fn base_classes_reach_every_style() {
        let line_active = State::HORIZONTAL | State::VARIANT_LINE | State::SELECTED;
        for (name, make, _) in MAKERS {
            let s = make();
            assert_eq!(s.dialog_footer.gap.get(State::NONE), Some(8.0), "{name}");
            assert_eq!(s.tooltip_content.max_width.get(State::NONE), Some(320.0), "{name}");
            assert_eq!(s.popover_content.width.get(State::NONE), Some(288.0), "{name}");
            assert_eq!(s.tabs_trigger.height_inset.get(State::NONE), Some(1.0), "{name}");
            assert_eq!(
                s.separator.bg.get(State::NONE),
                Some(Paint::Role(Role::Border, 1.0)),
                "{name}"
            );
            assert_eq!(s.tabs_trigger_after.opacity.get(line_active), Some(1.0), "{name}");
            assert_eq!(s.tabs_trigger_after.opacity.get(State::HORIZONTAL), Some(0.0), "{name}");
            assert_eq!(s.button.opacity.get(State::DISABLED), Some(0.5), "{name}");
        }
    }
```

If one of these assertions fails for a style after Step 6, read that part in `tools/stylegen/reports/<style>.md` and the style CSS: if upstream's merged string gives another value, the assertion is wrong for that style; STOP and show the maintainer before changing it.

- [ ] **Step 2: Run to see them fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 --test golden -- --test-threads 4`
Expected: compile errors (`Field::OutlineWidth`, `parts`, `Part` not found).

- [ ] **Step 3: Recipe fields (crate) and model fields (stylegen)**

`crates/shadcn-egui/src/style/recipe.rs`: replace the module doc's first paragraph with:

```rust
//! What one shadcn style says about one component part. The types are
//! hand-written; the values are generated by `tools/stylegen` from the
//! part's full class string: the svelte component's own classes with the
//! style's `@apply` classes (`style-<s>.css`) in place of its `cn-*` marker.
//! Every field is a [`Prop`]: an optional base value plus rules that apply
//! under state and context conditions, the way a Tailwind class applies
//! under its variants. An empty `Prop` means upstream sets nothing; the
//! component then uses the inherited or initial value.
```

In `pub struct Recipe`, after `max_width_inset`, add:

```rust
    /// `h-[calc(100%-Npx)]`: the parent's content height minus N px.
    pub height_inset: Prop<f32>,
```

after `ring_color`, add:

```rust
    /// `outline-N`: an outline N px wide just outside the border box
    /// (`outline-none`, `outline-hidden`: 0).
    pub outline_width: Prop<f32>,
    pub outline_color: Prop<Paint>,
```

`tools/stylegen/src/model.rs`: add `HeightInset` after `MaxWidthInset` and `OutlineWidth, OutlineColor` after `RingColor` in `enum Field`, and in `rust_name`:

```rust
            HeightInset => "height_inset",
            OutlineWidth => "outline_width",
            OutlineColor => "outline_color",
```

Then the part list: In `tools/stylegen/markers.txt`, replace the header line `# A file with no markers only has its `// base:` quotes checked.` with:

```
# `slot:<data-slot>` names a part without a `cn-*` marker: the one class
# string without a marker in `ui/<dir>/<data-slot>.svelte`.
```

and the line `separator.rs separator:` with `separator.rs separator: slot:separator`.

- [ ] **Step 4: Translations and ignores**

`tools/stylegen/src/utility.rs`:

1. Add `After(Vec<(Field, Value)>)` to `enum U` with the doc `/// Fields of the part's `after:` pseudo-element (its `_after` recipe).`
2. Add reason constants:

```rust
const STRUCTURE: &str = "structure: egui layout is done by the component";
const BROWSER: &str = "browser behaviour the component handles itself";
const DEFAULT_SIZE: &str = "the component's default sizing";
const ANCHOR: &str = "bits-ui anchor variable: placement computed by egui";
const CENTRE: &str = "dialog centring: egui centres the dialog window";
```

3. Remove `("outline-hidden", …)` and `("outline-none", …)` from `IGNORE_EXACT`. Add to `IGNORE_EXACT`:

```rust
    ("fixed", STRUCTURE),
    ("flex-col-reverse", STRUCTURE),
    ("flex-row", STRUCTURE),
    ("grow", STRUCTURE),
    ("peer", BROWSER),
    ("touch-none", BROWSER),
    ("whitespace-nowrap", BROWSER),
    ("w-fit", DEFAULT_SIZE),
    ("h-fit", DEFAULT_SIZE),
    ("top-1/2", CENTRE),
    ("left-1/2", CENTRE),
    ("-translate-x-1/2", CENTRE),
    ("-translate-y-1/2", CENTRE),
    (
        "text-current",
        "inherits the parent's text colour: the recipe leaves `text` empty",
    ),
```

and to `IGNORE_PREFIX` (keep the existing entries; replace the exact `items-center`, `justify-center`, `overflow-hidden`, `cursor-default` entries by these prefixes):

```rust
    ("items-", STRUCTURE),
    ("justify-", STRUCTURE),
    ("self-", STRUCTURE),
    ("place-content-", STRUCTURE),
    ("col-", STRUCTURE),
    ("row-", STRUCTURE),
    ("grid-cols-", STRUCTURE),
    ("grid-rows-", STRUCTURE),
    ("auto-rows-", STRUCTURE),
    ("@container/", STRUCTURE),
    ("group/", BROWSER),
    ("cursor-", BROWSER),
    ("overflow-", BROWSER),
    ("line-clamp-", BROWSER),
    ("origin-", ANCHOR),
```

4. Replace the `Some(Pseudo::Svg)` and `Some(Pseudo::After)` arms of `utility` with:

```rust
        Some(Pseudo::Svg) => {
            if let Some(v) = u.strip_prefix("size-").and_then(spacing) {
                return px(&[IconSize], v);
            }
            return match IGNORE_EXACT.iter().find(|(k, _)| *k == u) {
                Some((_, why)) => U::Ignore(why),
                None => U::Unsupported("svg child utility other than size-*"),
            };
        }
        Some(Pseudo::After) => {
            // `after:-inset-*` grows the hit area (checkbox, switch, slider thumb).
            for (prefix, fields) in [
                ("-inset-x-", &[HitInsetX][..]),
                ("-inset-y-", &[HitInsetY][..]),
                ("-inset-", &[HitInsetX, HitInsetY][..]),
            ] {
                if let Some(v) = u.strip_prefix(prefix) {
                    return spaced(fields, v, 1.0);
                }
            }
            // Anything else draws the pseudo-element: the part's `_after` recipe.
            return match utility(u, None) {
                U::Set(list) => U::After(list),
                other => other,
            };
        }
```

5. After the `IGNORE_PREFIX` check, before the `[--` variable definition, add:

```rust
    if u.contains("(--bits-") {
        return U::Ignore(ANCHOR);
    }
    if let Some(n) = u
        .strip_prefix("h-[calc(100%-")
        .and_then(|r| r.strip_suffix(")]"))
        .and_then(length)
    {
        return px(&[HeightInset], n);
    }
```

6. In `sides`, add before `("top-", &[InsetTop])`:

```rust
        ("inset-x-", &[InsetLeft, InsetRight]),
        ("inset-y-", &[InsetTop, InsetBottom]),
        ("inset-", &[InsetTop, InsetRight, InsetBottom, InsetLeft]),
```

7. In the `match u` of single words, add `"outline" => return px(&[OutlineWidth], 1.0),`. Before the `ring-` block, add:

```rust
    if let Some(v) = u.strip_prefix("outline-") {
        if v == "none" || v == "hidden" {
            return px(&[OutlineWidth], 0.0);
        }
        if let Ok(n) = v.parse::<f32>() {
            return px(&[OutlineWidth], n);
        }
        return role(v).map_or(U::Unsupported("outline-* value"), |(r, a)| {
            U::Set(vec![(OutlineColor, Value::Paint { role: r, alpha: a })])
        });
    }
    if let Some(v) = u.strip_prefix("fill-") {
        // The tooltip arrow is painted as a shape: its fill is its background.
        return role(v).map_or(U::Unsupported("fill-* value"), |(r, a)| {
            U::Set(vec![(Bg, Value::Paint { role: r, alpha: a })])
        });
    }
```

`tools/stylegen/src/variant.rs`:

1. In `TRANSLATED`, add `("data-highlighted", Is("FOCUS")),` after `("focus-visible", …)` and `("placeholder", Is("PLACEHOLDER")),` after `("data-placeholder", …)`. Update the doc of `State::FOCUS` in `recipe.rs` to ``/// `focus:`, `data-highlighted:` (the highlighted item of a list)`` and of `State::PLACEHOLDER` to ``/// `data-placeholder:`, `placeholder:` (placeholder text shows)``.
2. Change `const SIDE` to: `"the side egui placed the popup on: placement and slide-in direction, both done by egui (also the tooltip arrow offset)"`.
3. Add to `IGNORED`:

```rust
    ("rtl", "right-to-left layout; the crate lays out left to right"),
    ("[&_svg]", "svg child behaviour (pointer events, shrink); the crate paints icons itself"),
    ("has-data-[slot=card-action]", "card header grid template; the crate lays out the header itself"),
    ("has-data-[slot=card-description]", "card header grid template; the crate lays out the header itself"),
```

- [ ] **Step 5: Parts, after decls, exceptions, emission, report, build**

`tools/stylegen/src/resolve.rs`: add `pub after: Vec<Decl>` to `Marker`. In `translate`, add an arm after `U::Set`:

```rust
            U::After(list) => {
                for (field, value) in list {
                    m.after.push(Decl {
                        field,
                        value,
                        all: v.all,
                        none: v.none,
                        rank: v.rank,
                        idx,
                    });
                }
            }
```

and after `m.decls = merge(…)`: `m.after = merge(std::mem::take(&mut m.after));`. Replace `pub fn markers(…)` with:

```rust
/// Base classes dropped by name for one part: `(part, variant, why)`. The
/// component source does not say which element a bits-ui primitive renders.
pub const EXCEPTIONS: &[(&str, &str, &str)] = &[(
    "cn-slider-thumb",
    "disabled",
    "bits-ui renders the slider thumb as a <span>, where `disabled:` never matches",
)];

/// One part of one style, translated.
#[derive(Debug)]
pub struct Part {
    /// As in `markers.txt`: `cn-tabs-trigger`, `slot:separator`.
    pub name: String,
    pub marker: Marker,
}

/// A part's classes for one style: its marker replaced in place by the
/// style's classes for it (nothing where the style does not define it).
pub fn expand(component: &[String], part: &str, css: &[(String, Vec<String>)]) -> Vec<String> {
    let style: &[String] = css
        .iter()
        .find(|(n, _)| n == part)
        .map(|(_, c)| c.as_slice())
        .unwrap_or_default();
    component
        .iter()
        .flat_map(|c| {
            if c == part {
                style.to_vec()
            } else {
                vec![c.clone()]
            }
        })
        .collect()
}

/// Translate every part for one style, in `components` order.
pub fn parts(css: &[(String, Vec<String>)], components: &[(String, Vec<String>)]) -> Vec<Part> {
    let vars = collect_vars(css);
    components
        .iter()
        .map(|(name, classes)| {
            let mut dropped = Vec::new();
            let kept: Vec<String> = expand(classes, name, css)
                .into_iter()
                .filter(|c| {
                    let hit = EXCEPTIONS.iter().find(|(p, v, _)| {
                        *p == name.as_str() && split(c).variants.iter().any(|x| x == v)
                    });
                    match hit {
                        Some((_, _, why)) => {
                            dropped.push((c.clone(), *why));
                            false
                        }
                        None => true,
                    }
                })
                .collect();
            let mut marker = translate(&kept, &vars);
            marker.ignored.extend(dropped);
            Part {
                name: name.clone(),
                marker,
            }
        })
        .collect()
}
```

`tools/stylegen/src/emit.rs`:

```rust
/// `cn-button-size-icon-xs` → `button_size_icon_xs`; `slot:separator` → `separator`.
pub fn field_name(part: &str) -> String {
    part.strip_prefix("slot:")
        .unwrap_or_else(|| part.trim_start_matches("cn-"))
        .replace('-', "_")
}

/// The `Style` fields as `(field, doc)`: each part in `markers.txt` order,
/// followed by its `_after` part where any style gives it `after:` classes.
pub fn fields(styles: &[&[Part]]) -> Vec<(String, String)> {
    let first = styles.first().copied().unwrap_or_default();
    let mut out = Vec::new();
    for (i, p) in first.iter().enumerate() {
        let doc = match p.name.strip_prefix("slot:") {
            Some(s) => format!("`data-slot=\"{s}\"` (no style marker)"),
            None => format!("`{}`", p.name),
        };
        out.push((field_name(&p.name), doc));
        if styles.iter().any(|s| !s[i].marker.after.is_empty()) {
            out.push((
                format!("{}_after", field_name(&p.name)),
                format!("`{}` `after:` pseudo-element", p.name),
            ));
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for (f, _) in &out {
        assert!(seen.insert(f.clone()), "two parts give the Style field `{f}`");
    }
    out
}
```

Change `emit_struct(pin, markers: &[String])` to `emit_struct(pin: &str, fields: &[(String, String)])`; its doc line becomes `"/// One recipe per part the crate draws: a `cn-*` marker, a `data-slot`\n/// without one, or an `after:` pseudo-element. Adjust a value in\n"` followed by the existing example line, and the field loop:

```rust
    for (field, doc) in fields {
        s += &format!("    /// {doc}\n    pub {field}: Recipe,\n");
    }
```

Change the header source text in `emit_struct` to `"the parts in tools/stylegen/markers.txt"`. Replace `emit_style` and `recipe` with:

```rust
pub fn emit_style(style: &str, pin: &str, fields: &[(String, String)], parts: &[Part]) -> String {
    let mut s = header(pin, &format!("`style-{style}.css` and the ui/ component sources"));
    s += "use super::recipe::*;\nuse super::Style;\n\n";
    s += &format!("pub(super) fn style() -> Style {{\n    Style {{\n        name: \"{style}\",\n");
    let mut decls: BTreeMap<String, &[Decl]> = BTreeMap::new();
    for p in parts {
        decls.insert(field_name(&p.name), &p.marker.decls);
        decls.insert(format!("{}_after", field_name(&p.name)), &p.marker.after);
    }
    let mut fns = String::new();
    for (field, _) in fields {
        match decls.get(field).and_then(|d| recipe(d)) {
            None => s += &format!("        {field}: Recipe::default(),\n"),
            Some(body) => {
                s += &format!("        {field}: {field}(),\n");
                fns += &format!("\nfn {field}() -> Recipe {{\n    {body}\n}}\n");
            }
        }
    }
    s += "    }\n}\n";
    s + &fns
}

/// The `Recipe { .. }` literal, or `None` when nothing is set.
fn recipe(decls: &[Decl]) -> Option<String> {
    let mut by_field: BTreeMap<Field, Vec<&Decl>> = BTreeMap::new();
    for d in decls {
        by_field.entry(d.field).or_default().push(d);
    }
    // (the rest of the old body, from `if by_field.is_empty()` on, unchanged)
}
```

Keep the rest of the old `recipe` body as it is; change its `use` to `use crate::resolve::{Decl, Part};`.

`tools/stylegen/src/report.rs`: change the signature to `pub fn report(style: &str, pin: &str, parts: &[Part]) -> String` (`use crate::resolve::Part;`), iterate `for p in parts { let (name, m) = (&p.name, &p.marker); … }`, and sum `p.marker.unsupported.len()`.

`tools/stylegen/src/build.rs`:

```rust
//! What `stylegen generate` writes for the style recipes, as strings, so
//! the tests compare the committed files with exactly what generate writes.

use crate::css::parse_style_css;
use crate::markers::parse_markers;
use crate::resolve::{parts, Part};
use crate::sources::STYLES;
use crate::{emit, extract, report};
use std::path::Path;

pub struct Styles {
    /// `crates/shadcn-egui/src/style/generated.rs`.
    pub generated: String,
    /// `(style, src/style/<style>.rs)`, every style in `EXPORT` without
    /// unsupported classes, in [`STYLES`] order.
    pub exported: Vec<(String, String)>,
    /// `(style, reports/<style>.md)` for every upstream style.
    pub reports: Vec<(String, String)>,
    /// Styles in `EXPORT` with unsupported classes, with their count.
    pub failed: Vec<(String, usize)>,
}

pub fn styles(tool: &Path, pin: &str) -> Result<Styles, String> {
    let read = |p: &str| std::fs::read_to_string(tool.join(p)).map_err(|e| format!("{p}: {e}"));
    let lines = parse_markers(&read("markers.txt")?);
    let export = read("EXPORT")?;
    let components = extract::part_strings(&tool.join("tests/fixtures/ui"), &lines)?;
    let mut per_style: Vec<(&str, Vec<Part>)> = Vec::new();
    for style in STYLES {
        let css = parse_style_css(&read(&format!("tests/fixtures/style-{style}.css"))?);
        per_style.push((style, parts(&css, &components)));
    }
    let all: Vec<&[Part]> = per_style.iter().map(|(_, p)| p.as_slice()).collect();
    let fields = emit::fields(&all);
    let mut out = Styles {
        generated: emit::emit_struct(pin, &fields),
        exported: Vec::new(),
        reports: Vec::new(),
        failed: Vec::new(),
    };
    for (style, ps) in &per_style {
        out.reports.push((style.to_string(), report::report(style, pin, ps)));
        if export.split_whitespace().any(|e| e == *style) {
            let unsupported: usize = ps.iter().map(|p| p.marker.unsupported.len()).sum();
            if unsupported == 0 {
                out.exported
                    .push((style.to_string(), emit::emit_style(style, pin, &fields, ps)));
            } else {
                out.failed.push((style.to_string(), unsupported));
            }
        }
    }
    Ok(out)
}
```

Add `pub mod build;` to `src/lib.rs`. In `src/main.rs` `generate`, replace everything between `refresh(…)` and `let palettes = …` with:

```rust
    let out_dir = root.join(CRATE).join("src/style");
    let reports = tool.join("reports");
    std::fs::create_dir_all(&reports).expect("reports dir");
    let built = match stylegen::build::styles(tool, pin) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    write(&out_dir.join("generated.rs"), &built.generated);
    for (style, src) in &built.exported {
        write(&out_dir.join(format!("{style}.rs")), src);
    }
    for (style, md) in &built.reports {
        write(&reports.join(format!("{style}.md")), md);
    }
    // An export style that fails the gate must not stop the other outputs.
    let mut failed = false;
    for (style, n) in &built.failed {
        eprintln!("{style} is in EXPORT but has {n} unsupported classes; see reports/{style}.md");
        failed = true;
    }
```

Remove the now unused imports (`STYLES`, `css`, `emit`, `markers` if unused, `report`, `resolve`) that clippy names.

`tools/stylegen/src/drift.rs` `check`: replace

```rust
        if line.markers.is_empty() {
            continue;
        }
        let up_set = upstream_markers(&upstream);
        let ours: BTreeSet<String> = line.markers.iter().cloned().collect();
```

with

```rust
        // `slot:` parts have no marker to compare.
        let ours: BTreeSet<String> = line
            .markers
            .iter()
            .filter(|m| !m.starts_with("slot:"))
            .cloned()
            .collect();
        if ours.is_empty() {
            continue;
        }
        let up_set = upstream_markers(&upstream);
```

`tools/stylegen/tests/generated.rs`: replace `shipped`, `style_source` and the first two tests with:

```rust
fn built() -> stylegen::build::Styles {
    let pin = read("PIN").trim().to_owned();
    stylegen::build::styles(std::path::Path::new(ROOT), &pin).expect("build")
}

/// The fixtures are byte copies of the pinned upstream files, so the
/// committed crate sources must be exactly what the generator gives now,
/// and giving it twice must not change a byte.
#[test]
fn committed_sources_are_current_and_output_is_deterministic() {
    let (first, second) = (built(), built());
    assert!(first.failed.is_empty(), "unsupported classes: {:?}", first.failed);
    assert_eq!(first.generated, second.generated, "output differs between runs");
    assert_eq!(
        first.generated,
        read("../../crates/shadcn-egui/src/style/generated.rs"),
        "re-run stylegen generate"
    );
    let export = read("EXPORT");
    let names: Vec<&str> = first.exported.iter().map(|(s, _)| s.as_str()).collect();
    assert_eq!(names, export.split_whitespace().collect::<Vec<_>>());
    for ((style, src), (_, again)) in first.exported.iter().zip(&second.exported) {
        assert_eq!(src, again, "{style}: output differs between runs");
        assert_eq!(
            src,
            &read(&format!("../../crates/shadcn-egui/src/style/{style}.rs")),
            "{style}: re-run stylegen generate"
        );
    }
}

/// Stale reports hide new unsupported classes, so all eight must be current.
#[test]
fn committed_reports_are_current() {
    let b = built();
    assert_eq!(b.reports.len(), 8);
    for (style, md) in &b.reports {
        assert_eq!(md, &read(&format!("reports/{style}.md")), "{style}: re-run stylegen generate");
    }
}
```

and remove the now unused `use` lines. In `tests/golden.rs` and `tests/translate.rs`, a test that asserted an old ignore (`outline-none` ignored, `items-center` exact) is updated to the new translation; name each change in the commit message.

- [ ] **Step 6: Run the stylegen tests (fixture-based ones)**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 --test golden --test extract --test translate --test parse -- --test-threads 4`
Expected: PASS. `every_exported_style_translates_every_part` lists any class still unsupported. For each: if the spec's "New fields and translations" or "Ignored classes" covers it, add the rule with its reason group; if not, STOP and show the maintainer the class, the part and the style before adding anything.

- [ ] **Step 7: Regenerate**

```bash
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- generate "$SVELTE" "$REACT" .
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift "$SVELTE" "$REACT" .
systemd-run --user --scope -p MemoryMax=4G -- cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4
```
Expected: generate exits 0; `drift: clean`; stylegen tests PASS except `tests/published.rs` (Task B6 rewrites it; note its failure in the commit message if it fails).

Read `git diff --stat crates/shadcn-egui/src/style tools/stylegen/reports` and spot-check `crates/shadcn-egui/src/style/vega.rs` `fn tabs_trigger()` and `fn tabs_trigger_after()` against `tabs-trigger.svelte`.

- [ ] **Step 8: Crate tests, snapshots unchanged**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS, including `base_classes_reach_every_style`; `git status --short crates/shadcn-egui/tests/snapshots` prints nothing. Components already read some fields the base classes now fill (dialog footer `gap`, checkbox and label `opacity`, the tab trigger's `border_color`), so a snapshot difference is possible here: apply the snapshot rule (STOP, before/after PNGs, maintainer decides).

- [ ] **Step 9: Lint**

```bash
cargo clippy --manifest-path tools/stylegen/Cargo.toml --all-targets --jobs 4 -- -D warnings
cargo fmt --check --manifest-path tools/stylegen/Cargo.toml
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
cargo fmt --all --check
```
Expected: clean.

- [ ] **Step 10: Commit**

```bash
git add tools/stylegen crates/shadcn-egui/src/style
git commit -m "stylegen: translate each part's full class string

Recipes held only the style's classes; the base classes the svelte
component sets itself were written into the component code, out of an
app's reach (#6). Stylegen now builds each part's string the way the
published registry does (style classes in place of the marker, then the
base classes) and translates all of it. New: outline_*, height_inset, an
_after recipe for after: classes (the tab line indicator), slot: parts
without a marker (separator), data-highlighted:, placeholder:, inset-*,
fill-*, and the ignore groups of the spec, each with its reason.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B6: `published.rs` compares whole part strings

**Files:**
- Modify: `tools/stylegen/tests/published.rs`
- Modify: `tools/stylegen/tests/fixtures/published-allow.txt`

**Interfaces:**
- Consumes: `extract::{class_strings, part_strings, strip_cli_markers, normalise}`, `resolve::expand`, `css::parse_style_css`.

- [ ] **Step 1: Rewrite the test**

`tools/stylegen/tests/published.rs`:

```rust
use stylegen::css::parse_style_css;
use stylegen::extract::{class_strings, normalise, part_strings, strip_cli_markers};
use stylegen::markers::parse_markers;
use stylegen::resolve::expand;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// Both sides spelled alike: CLI markers removed (the published JSON keeps
/// `cn-menu-target`, `cn-menu-translucent` and `cn-font-heading` as text),
/// `calc()` operators without underscores (`calc(100%_-_1px)`).
fn canonical(s: &str) -> String {
    normalise(&strip_cli_markers(s).replace("_-_", "-").replace("_+_", "+"))
}

/// Each part's concatenated class string (the style's classes in place of
/// the marker, then the base classes, before any merge) must be a class
/// string of the published component for that style. A part that may
/// differ is listed in published-allow.txt as `<style> <part>  # why`.
#[test]
fn part_strings_match_the_published_components() {
    let allow = std::fs::read_to_string(format!("{ROOT}/tests/fixtures/published-allow.txt"))
        .unwrap_or_default();
    let allowed = |s: &str, p: &str| {
        allow
            .lines()
            .any(|l| l.split('#').next().unwrap_or("").split_whitespace().eq([s, p]))
    };
    let lines = parse_markers(
        &std::fs::read_to_string(format!("{ROOT}/markers.txt")).expect("markers.txt"),
    );
    let parts = part_strings(&std::path::Path::new(ROOT).join("tests/fixtures/ui"), &lines)
        .expect("parts");
    let mut missing = Vec::new();
    let mut compared = 0;
    for style in ["vega", "nova"] {
        let css = parse_style_css(
            &std::fs::read_to_string(format!("{ROOT}/tests/fixtures/style-{style}.css")).unwrap(),
        );
        for line in &lines {
            let path = format!("{ROOT}/tests/fixtures/published/{style}/{}.json", line.dir);
            let Ok(json) = std::fs::read_to_string(&path) else {
                continue;
            };
            let v: serde_json::Value = serde_json::from_str(&json).expect("json");
            let published: Vec<String> = v["files"]
                .as_array()
                .expect("files")
                .iter()
                .flat_map(|f| class_strings(f["content"].as_str().unwrap_or("")))
                .map(|s| canonical(&s))
                .collect();
            for m in &line.markers {
                let (_, classes) = parts.iter().find(|(n, _)| n == m).expect("part");
                let ours = canonical(&expand(classes, m, &css).join(" "));
                compared += 1;
                if !published.contains(&ours) && !allowed(style, m) {
                    missing.push(format!("{style} {m}:\n  ours: {ours}"));
                }
            }
        }
    }
    assert!(compared > 50, "compared only {compared} parts");
    assert!(missing.is_empty(), "not in published output:\n{}", missing.join("\n"));
}
```

Replace the content of `tests/fixtures/published-allow.txt` with:

```
# <style> <part>  # why this part's string is not in the published component
# Upstream changed after the pin, or the published build rewrote the
# string in a way canonical() in published.rs does not undo.
```

- [ ] **Step 2: Run it**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 --test published -- --test-threads 4`
Expected: PASS. If a part fails, print the published strings of that component next to ours and find the cause: an extractor bug (fix the extractor and add a case to `tests/extract.rs`), an ordering difference in `parse_style_css` (fix and add a `tests/parse.rs` case), or a published rewrite (add an allow line with the exact reason). Ask the maintainer before adding an allow line.

- [ ] **Step 3: Commit**

```bash
git add tools/stylegen/tests
git commit -m "stylegen: published test compares each part's whole string

The old check looked up each style class as a token and needed an allow
list for classes tailwind-merge dropped. With base classes in the recipe,
the test compares the concatenated string before any merge, so order and
base classes are checked too and the calc() spelling entries are gone.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B7: Tabs from the recipe, issue #6

**Files:**
- Modify: `crates/shadcn-egui/src/components/tabs.rs`
- Create: `crates/shadcn-egui/tests/recipes.rs`
- Create: `crates/shadcn-egui/tests/snapshots/tabs_issue6_vega_{light,dark}.png`
- Possibly re-baseline: `tabs*_luma_*.png`, `tabs*_rhea_*.png` (listed disagreement)

**Interfaces:**
- Consumes: `Rule::important`, `When::all` (B2); `shared::{opacity, style_painter}` (B2); `Style::tabs_trigger_after`, `Recipe::{height_inset, outline_width, outline_color}` (B5); `common::{rect_shapes, text_colour, restyled}` (B2).
- Produces: `tests/recipes.rs` with `mod common;` that later component tasks append to.

- [ ] **Step 1: Write the failing tests**

`crates/shadcn-egui/tests/recipes.rs`:

```rust
//! Every visual value comes from the style: an app that changes a recipe
//! through `theme.style` sees the change.

mod common;

use egui::vec2;
use shadcn_egui::components::tabs::{Tabs, TabsVariant};
use shadcn_egui::style::{Paint, Role, Rule, State, When};
use shadcn_egui::Theme;

/// Issue #6: a filled `primary` active tab, `primary-foreground` text, a
/// solid 2 px ring and no outline, all through `theme.style`.
fn issue6(theme: Theme) -> Theme {
    common::restyled(theme, |s| {
        let tr = &mut s.tabs_trigger;
        tr.bg.rules.push(Rule::important(
            When::all(State::SELECTED),
            Paint::Role(Role::Primary, 1.0),
        ));
        tr.text.rules.push(Rule::important(
            When::all(State::SELECTED),
            Paint::Role(Role::PrimaryForeground, 1.0),
        ));
        tr.ring_color.rules.push(Rule::important(
            When::all(State::FOCUS_VISIBLE),
            Paint::Role(Role::Ring, 1.0),
        ));
        tr.ring_width
            .rules
            .push(Rule::important(When::all(State::FOCUS_VISIBLE), 2.0));
        tr.outline_width
            .rules
            .push(Rule::important(When::ALWAYS, 0.0));
    })
}

fn focused_tabs(ui: &mut egui::Ui) -> egui::Response {
    let resp = ui.add(Tabs::new(&mut 0, &["Account", "Password"]));
    ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", 0))));
    resp
}

#[test]
fn issue6_tab_trigger_restyled_through_theme_style() {
    for theme in [Theme::vega_light(), Theme::vega_dark()] {
        let theme = issue6(theme);
        let (_, out) = common::measure_output(&theme, focused_tabs);
        let shapes = common::rect_shapes(&out);
        let p = &theme.palette;
        assert!(shapes.iter().any(|r| r.fill == p.primary), "active fill: {shapes:?}");
        assert!(
            shapes.iter().any(|r| r.stroke.width == 2.0
                && r.stroke.color == p.ring
                && r.stroke_kind == egui::StrokeKind::Outside),
            "2px ring: {shapes:?}"
        );
        assert!(
            !shapes.iter().any(|r| r.stroke.width == 1.0
                && r.stroke.color == p.ring
                && r.stroke_kind == egui::StrokeKind::Outside),
            "no outline: {shapes:?}"
        );
        assert_eq!(common::text_colour(&out), p.primary_foreground);
    }
    common::snapshot_with_themes(
        "tabs_issue6",
        vec2(320.0, 90.0),
        vec![
            ("vega_light", issue6(Theme::vega_light())),
            ("vega_dark", issue6(Theme::vega_dark())),
        ],
        |ui| {
            focused_tabs(ui);
        },
        |_| {},
    );
}

/// luma's `border-transparent!` has the important rank too; the app's rule
/// comes later and wins.
#[test]
fn important_app_rule_beats_upstream_important() {
    let luma = Theme::new(
        shadcn_egui::Style::luma(),
        shadcn_egui::Palette::light(),
        10.0,
    );
    let theme = common::restyled(luma, |s| {
        s.tabs_trigger.border_color.rules.push(Rule::important(
            When::all(State::SELECTED),
            Paint::Role(Role::Primary, 1.0),
        ));
    });
    let (_, out) = common::measure_output(&theme, |ui| ui.add(Tabs::new(&mut 0, &["A", "B"])));
    assert!(
        common::rect_shapes(&out)
            .iter()
            .any(|r| r.stroke.color == theme.palette.primary && r.stroke_kind == egui::StrokeKind::Inside),
        "primary border on the active trigger"
    );
}

#[test]
fn tabs_track_and_line_bar_come_from_the_style() {
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.tabs_list_variant_default.bg.base = Some(Paint::Role(Role::Primary, 1.0));
        s.tabs_trigger_after.bg.base = Some(Paint::Role(Role::Destructive, 1.0));
        s.tabs_trigger_after.height.rules.push(Rule::important(When::ALWAYS, 4.0));
    });
    let (rect, out) = common::measure_output(&theme, |ui| ui.add(Tabs::new(&mut 0, &["A", "B"])));
    let shapes = common::rect_shapes(&out);
    assert!(
        shapes.iter().any(|r| r.fill == theme.palette.primary && r.rect == rect),
        "track: {shapes:?}"
    );
    let (_, out) = common::measure_output(&theme, |ui| {
        ui.add(Tabs::new(&mut 0, &["A", "B"]).variant(TabsVariant::Line))
    });
    let bar = common::rect_shapes(&out)
        .into_iter()
        .find(|r| r.fill == theme.palette.destructive)
        .expect("line bar");
    assert_eq!(bar.rect.height(), 4.0);
}

/// Review Focus 5: the bar of a disabled line list fades with its trigger.
#[test]
fn disabled_line_bar_is_faded_with_its_trigger() {
    let theme = Theme::vega_light();
    let (_, out) = common::measure_output(&theme, |ui| {
        ui.add(
            Tabs::new(&mut 0, &["A", "B"])
                .variant(TabsVariant::Line)
                .enabled(false),
        )
    });
    let fg = theme.palette.foreground;
    let bars: Vec<_> = common::rect_shapes(&out)
        .into_iter()
        .filter(|r| r.rect.height() == 2.0)
        .collect();
    assert_eq!(bars.len(), 1, "{bars:?}");
    assert_eq!(bars[0].fill, fg.gamma_multiply(0.5));
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --test recipes -- --test-threads 4`
Expected: `issue6_tab_trigger_restyled_through_theme_style`, `important_app_rule_beats_upstream_important` and `tabs_track_and_line_bar_come_from_the_style` FAIL (fixed colours in `tabs.rs`); `disabled_line_bar_is_faded_with_its_trigger` passes already and must keep passing.

- [ ] **Step 3: Implement**

In `crates/shadcn-egui/src/components/tabs.rs`, replace the module doc's second sentence with: `Every value comes from the style's `cn-tabs-list`, `cn-tabs-list-variant-*`, `cn-tabs-trigger` recipes and the trigger's `after:` recipe (the line indicator).` Change the imports to:

```rust
use crate::components::shared::{box_shapes, interaction_state, opacity, ring_shape, style_painter};
use crate::style::{pick, Paint, Prop, Recipe, Role, State, TextSize, Weight};
```

In `ui`, after `let trigger = [&s.tabs_trigger];` add `let after = [&s.tabs_trigger_after];`. Replace the computation of `trigger_h` and `gap` with:

```rust
        let trigger_h = list_h - pad_t - pad_b - geo(&trigger, |r| &r.height_inset);
        let (tpad_l, tpad_r, border_w) = (
            geo(&trigger, |r| &r.pad_left),
            geo(&trigger, |r| &r.pad_right),
            geo(&trigger, |r| &r.border_width),
        );
        let gap = geo(&list, |r| &r.gap);
```

Remove the `// base: "inline-flex w-fit items-center justify-center"` comment. Replace the `below` and `thickness` lines with:

```rust
        // The line indicator may reach below the list (`after:bottom-[-5px]`).
        let reach = (-geo(&after, |r| &r.inset_bottom)).max(0.0);
```

and replace the paint block, from `if ui.is_rect_visible(list_rect.expand(below)) {` to its closing brace, with:

```rust
        if ui.is_rect_visible(list_rect.expand(reach)) {
            let list_radius = pick(&list, |r| &r.radius, g)
                .map_or(0.0, |r| t.radius_px(r, list_rect.height() / 2.0));
            let track = pick(&list, |r| &r.bg, g).map(|p| t.paint(p));
            // The fade is the triggers' (`disabled:opacity-50`), not the list's.
            ui.painter()
                .extend(box_shapes(list_rect, list_radius, track, None, false));
            let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground where neither trigger nor list sets a colour
            let list_text = pick(&list, |r| &r.text, g).map(|p| t.paint(p));

            for (i, (rect, r)) in trigger_rects.iter().zip(&resps).enumerate() {
                let context = State::HORIZONTAL
                    .or(variant_bit)
                    .with(State::SELECTED, i == active);
                let st = interaction_state(&t, r, enabled, context);
                let painter = style_painter(ui, opacity(&[&trigger[..], &list[..]], st));
                let paint =
                    |l: &[&Recipe], f: fn(&Recipe) -> &Prop<Paint>| pick(l, f, st).map(|p| t.paint(p));
                let px = |l: &[&Recipe], f: fn(&Recipe) -> &Prop<f32>| pick(l, f, st).unwrap_or(0.0);

                let fg = paint(&trigger, |r| &r.text)
                    .or(list_text)
                    .unwrap_or(inherited);
                let radius = pick(&trigger, |r| &r.radius, st)
                    .map_or(0.0, |r| t.radius_px(r, rect.height() / 2.0));
                if let Some(level) = pick(&trigger, |r| &r.shadow, st) {
                    painter.add(t.shadow(level).as_shape(*rect, radius));
                }
                let stroke = paint(&trigger, |r| &r.border_color).map(|b| Stroke::new(border_w, b));
                let mut shapes =
                    box_shapes(*rect, radius, paint(&trigger, |r| &r.bg), stroke, false);
                if let Some(c) = paint(&trigger, |r| &r.ring_color) {
                    shapes.extend(ring_shape(*rect, radius, px(&trigger, |r| &r.ring_width), c));
                }
                if let Some(c) = paint(&trigger, |r| &r.outline_color) {
                    shapes.extend(ring_shape(*rect, radius, px(&trigger, |r| &r.outline_width), c));
                }
                // The `after:` line indicator, placed against the padding box.
                let bar_alpha = opacity(&[&after[..]], st);
                if let (Some(c), true) = (paint(&after, |r| &r.bg), bar_alpha > 0.0) {
                    let inner = rect.shrink(border_w);
                    let bottom = inner.bottom() - px(&after, |r| &r.inset_bottom);
                    let bar = Rect::from_min_max(
                        pos2(
                            inner.left() + px(&after, |r| &r.inset_left),
                            bottom - px(&after, |r| &r.height),
                        ),
                        pos2(inner.right() - px(&after, |r| &r.inset_right), bottom),
                    );
                    shapes.push(Shape::rect_filled(bar, 0.0, c.gamma_multiply(bar_alpha)));
                }
                painter.extend(shapes);
                let galley = galleys[i].clone();
                painter.galley(rect.center() - galley.size() / 2.0, galley, fg);
            }
        }
```

Delete every remaining `// base:` comment in `tabs.rs` and the unused `line_gap` variable. If rustfmt moves the `// behaviour-ok:` comment off the `Role::` line, shorten the comment until it stays.

- [ ] **Step 4: Run the tabs tests and the snapshots**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: the `recipes` tests PASS except the `tabs_issue6` snapshot, which is missing. Every `tabs*` snapshot PASSES except, possibly, `tabs*_luma_*` and `tabs*_rhea_*` (the listed disagreement: no `Ring` border on focus, no `Input` border on the dark active trigger). Any other failing snapshot: STOP (snapshot rule).

- [ ] **Step 5: Show the luma and rhea differences, then re-baseline**

For each failing `tabs*_luma_*` / `tabs*_rhea_*`: open the baseline and the `.new.png` with the Read tool and confirm the only change is the border described in the spec. STOP and show the maintainer the pairs; on approval:

```bash
systemd-run --user --scope -p MemoryMax=4G -- env UPDATE_SNAPSHOTS=1 cargo test -p shadcn-egui --jobs 4 --test components --test recipes tabs -- --test-threads 4
```

Open `tabs_issue6_vega_light.png` and `tabs_issue6_vega_dark.png`: the active "Account" tab is filled primary with primary-foreground text and a solid 2 px ring, no thin outline. Then run the full crate suite again; expected PASS and `git status --short crates/shadcn-egui/tests/snapshots` lists only the approved luma/rhea files and the two new `tabs_issue6` files.

- [ ] **Step 6: Commit**

```bash
git add crates/shadcn-egui/src/components/tabs.rs crates/shadcn-egui/tests
git commit -m "tabs: every trigger value from the style (#6)

The trigger drew its text colours, active fill, borders, focus ring and
outline, line indicator and height inset from values fixed in tabs.rs, so
an app could not restyle them. They now come from tabs_trigger, the
list variants and tabs_trigger_after. Luma and rhea lose the focus and
dark active border, as upstream's border-transparent! says (approved).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B8: Tooltip from the recipe

**Files:**
- Modify: `crates/shadcn-egui/src/components/tooltip.rs`
- Modify: `crates/shadcn-egui/tests/recipes.rs`

**Interfaces:**
- Consumes: `tooltip_content.{bg,text,max_width}`, `tooltip_arrow.bg` (B5).

- [ ] **Step 1: Write the failing test**

Append to `crates/shadcn-egui/tests/recipes.rs`:

```rust
#[test]
fn tooltip_colours_come_from_the_style() {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::button::{Button, ButtonVariant};
    use shadcn_egui::components::tooltip::tooltip;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.tooltip_content.bg.base = Some(Paint::Role(Role::Primary, 1.0));
        s.tooltip_arrow.bg.base = Some(Paint::Role(Role::Primary, 1.0));
    });
    let primary = theme.palette.primary;
    let mut h = egui_kittest::Harness::builder()
        .with_size(vec2(240.0, 120.0))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            ui.style_mut().interaction.tooltip_delay = 0.0;
            ui.add_space(32.0);
            ui.horizontal(|ui| {
                ui.add_space(32.0);
                tooltip(
                    ui.add(Button::new("Hover me").variant(ButtonVariant::Outline)),
                    "Add to library",
                );
            });
        });
    h.run();
    let trigger = h.get_by_label("Hover me").rect();
    h.get_by_label("Hover me").hover();
    h.run();
    let text = h.get_by_label("Add to library").rect();
    let img = h.render().expect("render");
    let px = |p: egui::Pos2| {
        let c = img.get_pixel(p.x as u32, p.y as u32);
        egui::Color32::from_rgb(c[0], c[1], c[2])
    };
    assert_eq!(px(egui::pos2(text.left() - 4.0, text.center().y)), primary, "content");
    assert_eq!(px(egui::pos2(text.center().x, trigger.top() - 2.0)), primary, "arrow");
}
```

- [ ] **Step 2: Run to see it fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --test recipes tooltip -- --test-threads 4`
Expected: FAIL, the content pixel is the foreground colour.

- [ ] **Step 3: Implement**

In `tooltip.rs`: change the module doc's base-class sentence to `Every value comes from the style's `cn-tooltip-content` and `cn-tooltip-arrow` recipes.`; import `use crate::style::{pick, State, TextSize};` (no `Role`). Replace the fixed colours and `max_w`:

```rust
    let fill = pick(&content, |r| &r.bg, g).map(|p| t.paint(p));
    let fg = pick(&content, |r| &r.text, g).map(|p| t.paint(p));
    let arrow_fill = pick(&arrow, |r| &r.bg, g).map(|p| t.paint(p));
    let mut tip = egui::Tooltip::for_enabled(&resp);
    let max_w = pick(&content, |r| &r.max_width, g).unwrap_or(f32::INFINITY);
```

Build the frame without a fixed fill (`let mut frame = Frame::new().corner_radius(..).inner_margin(..); if let Some(f) = fill { frame = frame.fill(f); }`), and the label as:

```rust
        let mut label = RichText::new(text).size(text_px).line_height(Some(line));
        if let Some(fg) = fg {
            label = label.color(fg);
        }
        ui.label(label)
```

Paint the arrow only `if let Some(fill) = arrow_fill`. Remove every `// base:` comment.

- [ ] **Step 4: Run tests and snapshots**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS; `git status --short crates/shadcn-egui/tests/snapshots` prints nothing. Otherwise: snapshot rule.

- [ ] **Step 5: Commit**

```bash
git add crates/shadcn-egui/src/components/tooltip.rs crates/shadcn-egui/tests/recipes.rs
git commit -m "tooltip: fill, text, arrow and max width from the style

They were fixed in tooltip.rs (bg-foreground, text-background, max-w-xs),
so an app could not restyle the tooltip (#6).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B9: Separator from its `slot:` recipe

**Files:**
- Modify: `crates/shadcn-egui/src/components/separator.rs`
- Modify: `crates/shadcn-egui/tests/recipes.rs`

**Interfaces:**
- Consumes: `Style::separator` (B5), `t.metrics.border`.

- [ ] **Step 1: Write the failing test**

Append to `recipes.rs`:

```rust
#[test]
fn separator_line_comes_from_the_style() {
    use shadcn_egui::components::separator::separator;
    use shadcn_egui::style::Prop;
    let line = |theme: &Theme| {
        let (_, out) = common::measure_output(theme, |ui| {
            ui.allocate_response(vec2(200.0, 0.0), egui::Sense::hover());
            separator(ui);
            ui.allocate_response(vec2(0.0, 0.0), egui::Sense::hover())
        });
        common::rect_shapes(&out)
            .into_iter()
            .find(|r| r.fill == theme.palette.primary)
            .map(|r| r.rect.height())
    };
    let thick = common::restyled(Theme::vega_light(), |s| {
        s.separator.bg.base = Some(Paint::Role(Role::Primary, 1.0));
        s.separator
            .height
            .rules
            .push(Rule::important(When::all(State::HORIZONTAL), 3.0));
    });
    assert_eq!(line(&thick), Some(3.0));
    // Review Focus 4: without a thickness rule, `metrics.border`.
    let mut fallback = common::restyled(Theme::vega_light(), |s| {
        s.separator.bg.base = Some(Paint::Role(Role::Primary, 1.0));
        s.separator.height = Prop::default();
    });
    fallback.metrics.border = 2.0;
    assert_eq!(line(&fallback), Some(2.0));
}
```

- [ ] **Step 2: Run to see it fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --test recipes separator -- --test-threads 4`
Expected: FAIL, no primary line (colour fixed to `Border`).

- [ ] **Step 3: Implement**

Replace the body of `separator.rs` below the module doc (change the doc to: `It has no style marker; its `data-slot="separator"` classes are the `separator` recipe: a `bg-border` line, `h-px` / `w-px` thick, or `metrics.border` thick where the recipe sets no thickness.`) with:

```rust
use crate::style::{pick, Prop, Recipe, State};
use crate::Theme;
use egui::{Sense, Ui, Vec2};

fn line(
    ui: &mut Ui,
    orientation: State,
    thickness: fn(&Recipe) -> &Prop<f32>,
    size: impl FnOnce(f32, &Ui) -> Vec2,
) {
    let t = Theme::current(ui.ctx());
    let l = [&t.style.separator];
    let st = t.state(orientation);
    let px = pick(&l, thickness, st).unwrap_or(t.metrics.border);
    let (rect, _) = ui.allocate_exact_size(size(px, ui), Sense::hover());
    if ui.is_rect_visible(rect) {
        if let Some(c) = pick(&l, |r| &r.bg, st).map(|p| t.paint(p)) {
            ui.painter().rect_filled(rect, 0.0, c);
        }
    }
}

/// A horizontal rule across the available width.
pub fn separator(ui: &mut Ui) {
    line(ui, State::HORIZONTAL, |r| &r.height, |h, ui| {
        Vec2::new(ui.available_width(), h)
    });
}

/// A vertical rule as tall as the row it sits in.
pub fn separator_vertical(ui: &mut Ui) {
    line(ui, State::VERTICAL, |r| &r.width, |w, ui| {
        Vec2::new(w, ui.available_height())
    });
}
```

- [ ] **Step 4: Run tests and snapshots**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS (also `separators_are_one_pixel_border_lines`); no snapshot changed. Otherwise: snapshot rule.

- [ ] **Step 5: Commit**

```bash
git add crates/shadcn-egui/src/components/separator.rs crates/shadcn-egui/tests/recipes.rs
git commit -m "separator: colour and thickness from its recipe

The line colour was fixed to Border and its thickness was metrics.border.
Both now come from the separator recipe (data-horizontal:h-px,
data-vertical:w-px, bg-border); metrics.border remains the fallback when
the recipe sets no thickness.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B10: Button and badge

**Files:**
- Modify: `crates/shadcn-egui/src/components/button.rs`, `crates/shadcn-egui/src/components/badge.rs`
- Modify: `crates/shadcn-egui/tests/recipes.rs`

**Interfaces:**
- Consumes: `button.opacity` (base `disabled:opacity-50`), `shared::{opacity, style_painter}`, `t.metrics.border`.

- [ ] **Step 1: Write the failing tests**

Append to `recipes.rs`:

```rust
#[test]
fn button_disabled_opacity_comes_from_the_style() {
    use shadcn_egui::components::button::Button;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.button
            .opacity
            .rules
            .push(Rule::important(When::all(State::DISABLED), 0.25));
    });
    let (_, on) = common::measure_output(&theme, |ui| ui.add(Button::new("Save")));
    let (_, off) = common::measure_output(&theme, |ui| ui.add(Button::new("Save").enabled(false)));
    assert_eq!(
        common::text_colour(&off),
        common::text_colour(&on).gamma_multiply(0.25)
    );
}

/// Review Focus 3: in a disabled `Ui` egui fades once; the recipe's
/// `disabled:opacity-50` does not fade again.
#[test]
fn disabled_ui_fades_once() {
    use shadcn_egui::components::button::Button;
    let theme = Theme::vega_light();
    let (_, own) = common::measure_output(&theme, |ui| ui.add(Button::new("Save").enabled(false)));
    let (_, in_ui) =
        common::measure_output(&theme, |ui| ui.add_enabled(false, Button::new("Save")));
    let (_, on) = common::measure_output(&theme, |ui| ui.add(Button::new("Save")));
    let a = |c: egui::Color32| c.a();
    assert_eq!(a(common::text_colour(&own)), a(common::text_colour(&on).gamma_multiply(0.5)));
    assert!(
        a(common::text_colour(&in_ui)) > a(common::text_colour(&on)) / 4 + 8,
        "faded twice: {:?}",
        common::text_colour(&in_ui)
    );
}
```

- [ ] **Step 2: Run to see them fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --test recipes button disabled_ui -- --test-threads 4`
Expected: `button_disabled_opacity_comes_from_the_style` FAILS (0.5 fixed); `disabled_ui_fades_once` passes and must keep passing.

- [ ] **Step 3: Implement**

`button.rs`: import `opacity, style_painter` instead of `disabled_painter`. Replace

```rust
        // base: "disabled:pointer-events-none disabled:opacity-50"
        let painter = disabled_painter(ui, self.enabled, 0.5);
```

with `let fonts = ui.painter().clone();` and use `fonts.layout_job(job)` inside `layout`. After `let st = interaction_state(...)`, add:

```rust
        let painter = style_painter(ui, opacity(&[&l[..]], st));
```

Replace `layout(Stroke::new(1.0_f32, fg))` with `layout(Stroke::new(t.metrics.border, fg))` (upstream sets no underline thickness). Remove every `// base:` comment and the module doc's `// base:` sentence (`Every value comes from the style's button recipes.`).

`badge.rs`: same underline change (`Stroke::new(t.metrics.border, fg)`), remove `// base:` comments and the doc sentence.

- [ ] **Step 4: Run tests and snapshots**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS; no snapshot changed. Otherwise: snapshot rule.

- [ ] **Step 5: Commit**

```bash
git add crates/shadcn-egui/src/components/button.rs crates/shadcn-egui/src/components/badge.rs crates/shadcn-egui/tests/recipes.rs
git commit -m "button, badge: disabled opacity and underline from the style

The disabled fade was a fixed 0.5 and the link underline a fixed 1 px.
The fade now comes from the button recipe's opacity; the underline uses
metrics.border, since upstream sets no thickness for it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B11: Input

**Files:**
- Modify: `crates/shadcn-egui/src/components/input.rs`
- Modify: `crates/shadcn-egui/tests/recipes.rs`

**Interfaces:**
- Consumes: `input.text` under `PLACEHOLDER`, `input.opacity` under `DISABLED`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn input_placeholder_and_fade_come_from_the_style() {
    use shadcn_egui::components::input::Input;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.input.text.rules.push(Rule::important(
            When::all(State::PLACEHOLDER),
            Paint::Role(Role::Primary, 1.0),
        ));
        s.input
            .opacity
            .rules
            .push(Rule::important(When::all(State::DISABLED), 0.25));
    });
    let (_, on) = common::measure_output(&theme, |ui| {
        ui.add(Input::new(&mut String::new()).hint("Email"))
    });
    assert_eq!(common::text_colour(&on), theme.palette.primary);
    let (_, off) = common::measure_output(&theme, |ui| {
        ui.add(Input::new(&mut String::new()).hint("Email").enabled(false))
    });
    assert_eq!(common::text_colour(&off), theme.palette.primary.gamma_multiply(0.25));
}
```

- [ ] **Step 2: Run to see it fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --test recipes input -- --test-threads 4`
Expected: FAIL, the hint is `MutedForeground`.

- [ ] **Step 3: Implement**

In `input.rs` (imports: `opacity, style_painter` instead of `disabled_painter`):

```rust
        let fade = opacity(&[&l[..]], g.with(State::DISABLED, !self.enabled));
        let placeholder = pick(&l, |r| &r.text, g.or(State::PLACEHOLDER)).map(|p| t.paint(p));
        let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground; upstream sets only file:text-foreground
```

`.text_color(inherited)` on the `TextEdit`. Inside the scope: `if let Some(c) = placeholder { ui.visuals_mut().weak_text_color = Some(c); }` and `if ui.is_enabled() { ui.multiply_opacity(fade); }`. After `let st = …`, paint the box with `style_painter(ui, opacity(&[&l[..]], st)).set(slot, Shape::Vec(shapes));`. Remove the `disabled_opacity` variable and every `// base:` comment; update the module doc.

- [ ] **Step 4: Run tests and snapshots**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS; no snapshot changed. Otherwise: snapshot rule.

- [ ] **Step 5: Commit**

```bash
git add crates/shadcn-egui/src/components/input.rs crates/shadcn-egui/tests/recipes.rs
git commit -m "input: placeholder colour and disabled fade from the style

placeholder:text-muted-foreground and disabled:opacity-50 were fixed in
input.rs. The text colour itself still inherits Foreground: upstream sets
none for a text field.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B12: Select (fade, `side_offset`)

**Files:**
- Modify: `crates/shadcn-egui/src/components/select.rs`
- Modify: `crates/shadcn-egui/tests/recipes.rs`

**Interfaces:**
- Produces: `Select::side_offset(self, px: f32) -> Self` (default `DEFAULT_SIDE_OFFSET`).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn select_fade_and_offset() {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::select::Select;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.select_trigger
            .opacity
            .rules
            .push(Rule::important(When::all(State::DISABLED), 0.25));
    });
    let (_, on) = common::measure_output(&theme, |ui| ui.add(Select::new("s", &mut 0, &["Apple"])));
    let (_, off) = common::measure_output(&theme, |ui| {
        ui.add(Select::new("s", &mut 0, &["Apple"]).enabled(false))
    });
    assert_eq!(common::text_colour(&off), common::text_colour(&on).gamma_multiply(0.25));

    let item_top = |offset: f32| {
        let mut h = egui_kittest::Harness::builder()
            .with_size(vec2(320.0, 260.0))
            .build_ui(move |ui| {
                Theme::vega_light().apply(ui.ctx());
                ui.add(
                    Select::new("fruit", &mut usize::MAX, &["Apple", "Banana"])
                        .placeholder("Pick")
                        .side_offset(offset),
                );
            });
        h.run();
        h.get_by_role(egui::accesskit::Role::ComboBox).click();
        h.run();
        h.get_by_label("Banana").rect().top()
    };
    assert_eq!(item_top(20.0) - item_top(4.0), 16.0);
}
```

- [ ] **Step 2: Run to see it fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --test recipes select -- --test-threads 4`
Expected: compile error, no method `side_offset`.

- [ ] **Step 3: Implement**

Add `side_offset: f32` to `struct Select`, `side_offset: DEFAULT_SIDE_OFFSET` in `new`, and:

```rust
    /// Gap between the trigger and the list (shadcn `sideOffset`, default 4).
    pub fn side_offset(mut self, px: f32) -> Self {
        self.side_offset = px;
        self
    }
```

Replace `let side_offset = 4.0;` (and its `// base:` line) with `let side_offset = self.side_offset;`. Replace the trigger's `disabled_painter(ui, self.enabled, 0.5)` with `style_painter(ui, opacity(&[&trig[..]], st))`. Remove every `// base:` comment and update the module doc.

- [ ] **Step 4: Run tests and snapshots**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS; no snapshot changed. Otherwise: snapshot rule.

- [ ] **Step 5: Commit**

```bash
git add crates/shadcn-egui/src/components/select.rs crates/shadcn-egui/tests/recipes.rs
git commit -m "select: disabled fade from the style, side_offset builder

The trigger's fade was a fixed 0.5. sideOffset is a prop of upstream's
SelectContent, not a class, so it becomes a builder method with
upstream's default of 4.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B13: Switch and checkbox (fade, hit area)

**Files:**
- Modify: `crates/shadcn-egui/src/components/switch.rs`, `crates/shadcn-egui/src/components/checkbox.rs`
- Modify: `crates/shadcn-egui/tests/recipes.rs`

**Interfaces:**
- Consumes: `switch.{hit_inset_x, hit_inset_y, opacity}`, `checkbox.{hit_inset_x, hit_inset_y, opacity}`.

- [ ] **Step 1: Write the failing tests**

```rust
/// Press and release the primary button at `pos` over three frames; true
/// if `add` flipped the value.
fn click_flips(theme: &Theme, pos: egui::Pos2, mut add: impl FnMut(&mut egui::Ui, &mut bool)) -> bool {
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    let mut value = false;
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    for events in [vec![], vec![egui::Event::PointerMoved(pos), press(true)], vec![press(false)]] {
        let _ = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| add(ui, &mut value),
        );
    }
    value
}

#[test]
fn hit_area_comes_from_the_style() {
    use shadcn_egui::components::checkbox::Checkbox;
    use shadcn_egui::components::switch::Switch;
    type Add = fn(&mut egui::Ui, &mut bool) -> egui::Response;
    let widgets: [(&str, Add); 2] = [
        ("checkbox", |ui, v| ui.add(Checkbox::new(v))),
        ("switch", |ui, v| ui.add(Switch::new(v))),
    ];
    for (name, add) in widgets {
        let theme = Theme::vega_light();
        let rect = common::measure(&theme, |ui| add(ui, &mut false));
        let beside = egui::pos2(rect.right() + 6.0, rect.center().y);
        assert!(click_flips(&theme, beside, |ui, v| { add(ui, v); }), "{name}: default reach");
        let tight = common::restyled(Theme::vega_light(), |s| {
            for r in [&mut s.checkbox, &mut s.switch] {
                r.hit_inset_x.rules.push(Rule::important(When::ALWAYS, 0.0));
            }
        });
        assert!(!click_flips(&tight, beside, |ui, v| { add(ui, v); }), "{name}: no reach");
    }
}

#[test]
fn switch_fade_comes_from_the_style() {
    use shadcn_egui::components::switch::Switch;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.switch
            .opacity
            .rules
            .push(Rule::important(When::all(State::DISABLED), 0.25));
    });
    let fill = |enabled: bool| {
        let (_, out) = common::measure_output(&theme, |ui| ui.add(Switch::new(&mut true).enabled(enabled)));
        common::rect_shapes(&out)[0].fill
    };
    assert_eq!(fill(false), fill(true).gamma_multiply(0.25));
}
```

- [ ] **Step 2: Run to see them fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --test recipes hit_area switch_fade -- --test-threads 4`
Expected: FAIL: "no reach" still flips (12 px fixed); the switch fades by 0.5.

- [ ] **Step 3: Implement**

In both files, replace

```rust
        // base: "after:absolute after:-inset-x-3 after:-inset-y-2"
        let hit = rect.expand2(vec2(12.0, 8.0));
```

with

```rust
        let hit = rect.expand2(vec2(get(|r| &r.hit_inset_x), get(|r| &r.hit_inset_y)));
```

(`get` is the file's existing geometry getter over the control's recipe and `g`). In `switch.rs`, replace `disabled_painter(ui, self.enabled, 0.5)` with `style_painter(ui, opacity(&[&track[..]], st))`; if the thumb is painted through the same painter, paint its shapes with `style_painter(ui, opacity(&[&thumb[..], &track[..]], st))` instead. In `checkbox.rs`, replace the `disabled_painter(ui, self.enabled, pick(…).unwrap_or(1.0))` call with `style_painter(ui, opacity(&[&l[..]], st))`. Remove every `// base:` comment, update module docs, fix imports.

- [ ] **Step 4: Run tests and snapshots**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS; no snapshot changed. Otherwise: snapshot rule.

- [ ] **Step 5: Commit**

```bash
git add crates/shadcn-egui/src/components/switch.rs crates/shadcn-egui/src/components/checkbox.rs crates/shadcn-egui/tests/recipes.rs
git commit -m "switch, checkbox: hit area and disabled fade from the style

The 12 by 8 px hit area around both controls and the switch's 0.5 fade
were fixed in code. They now come from after:-inset-x/-y and
disabled:opacity-50 in the recipes.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B14: Slider (opacity down the part tree)

**Files:**
- Modify: `crates/shadcn-egui/src/components/slider.rs`
- Modify: `crates/shadcn-egui/tests/recipes.rs`

**Interfaces:**
- Consumes: `slider.opacity` (root, `data-disabled:opacity-50`), `slider_thumb.opacity` (no disabled rule: `resolve::EXCEPTIONS`).

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn slider_fade_multiplies_down_the_parts() {
    use shadcn_egui::components::slider::Slider;
    let theme = common::restyled(Theme::vega_light(), |s| {
        s.slider
            .opacity
            .rules
            .push(Rule::important(When::all(State::DISABLED), 0.25));
    });
    let fills = |enabled: bool| {
        let (_, out) = common::measure_output(&theme, |ui| {
            ui.add(Slider::new(&mut 50.0, 0.0..=100.0).enabled(enabled))
        });
        common::rect_shapes(&out).into_iter().map(|r| r.fill).collect::<Vec<_>>()
    };
    let (on, off) = (fills(true), fills(false));
    assert_eq!(on.len(), off.len());
    for (a, b) in on.iter().zip(&off) {
        // Every part, the thumb included, at the root's 0.25: the thumb's
        // own `disabled:` never matches upstream (a <span>).
        assert_eq!(*b, a.gamma_multiply(0.25));
    }
}
```

- [ ] **Step 2: Run to see it fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --test recipes slider -- --test-threads 4`
Expected: FAIL (fixed 0.5).

- [ ] **Step 3: Implement**

Replace `disabled_painter(ui, self.enabled, 0.5)` and its `// base:` line with:

```rust
            let painter = style_painter(ui, opacity(&[&root[..]], st));
            let thumb_painter = style_painter(ui, opacity(&[&thumb[..], &root[..]], st));
```

Paint the thumb's shapes (shadow, box, ring) through `thumb_painter` and the track and range through `painter`. Annotate the arithmetic the stricter scan (B16) will flag, each on its own line: `let dir = if vertical { -1.0 } else { 1.0 }; // behaviour-ok: direction of travel` and the `clamp(0.0, 1.0)` lines with `// behaviour-ok: fraction of the range`. Remove every `// base:` comment; update the module doc.

- [ ] **Step 4: Run tests and snapshots**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS; no snapshot changed. Otherwise: snapshot rule.

- [ ] **Step 5: Commit**

```bash
git add crates/shadcn-egui/src/components/slider.rs crates/shadcn-egui/tests/recipes.rs
git commit -m "slider: disabled fade from the style, multiplied down the parts

The fade was a fixed 0.5 for the whole slider. The root's opacity now
comes from its recipe and the thumb multiplies its own, as CSS does; the
thumb's disabled: class never matches upstream and stays dropped.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B15: Popover (width, `side_offset`) and dialog footer gap

**Files:**
- Modify: `crates/shadcn-egui/src/components/popover.rs`, `crates/shadcn-egui/src/components/dialog.rs`
- Modify: `crates/shadcn-egui/tests/recipes.rs`

**Interfaces:**
- Produces: `Popover::side_offset(self, px: f32) -> Self` (default `DEFAULT_SIDE_OFFSET`).
- Consumes: `popover_content.width` (`w-72`), `dialog_footer.gap` (`gap-2`).

- [ ] **Step 1: Write the failing test**

```rust
fn popover_probe(theme: Theme, offset: Option<f32>) -> (egui::Rect, f32) {
    use egui_kittest::kittest::Queryable;
    use shadcn_egui::components::button::Button;
    use shadcn_egui::components::popover::Popover;
    use std::{cell::Cell, rc::Rc};
    let width = Rc::new(Cell::new(0.0));
    let w = width.clone();
    let mut h = egui_kittest::Harness::builder()
        .with_size(vec2(520.0, 320.0))
        .build_ui(move |ui| {
            theme.apply(ui.ctx());
            let resp = ui.add(Button::new("Open"));
            let mut p = Popover::new(&resp);
            if let Some(o) = offset {
                p = p.side_offset(o);
            }
            p.show(|ui| {
                w.set(ui.available_width());
                ui.label("Inside");
            });
        });
    h.run();
    h.get_by_label("Open").click();
    h.run();
    (h.get_by_label("Inside").rect(), width.get())
}

#[test]
fn popover_width_and_offset() {
    let wide = common::restyled(Theme::vega_light(), |s| {
        s.popover_content.width.base = Some(400.0);
    });
    let (near, w288) = popover_probe(Theme::vega_light(), None);
    let (_, w400) = popover_probe(wide, None);
    assert_eq!(w400 - w288, 112.0);
    let (far, _) = popover_probe(Theme::vega_light(), Some(20.0));
    assert_eq!(far.top() - near.top(), 16.0);
}
```

- [ ] **Step 2: Run to see it fail**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --test recipes popover -- --test-threads 4`
Expected: compile error, no method `side_offset`.

- [ ] **Step 3: Implement**

`popover.rs`: delete the `DEFAULT_WIDTH` and `SIDE_OFFSET` consts with their `// base:` lines; add `side_offset: f32` to the struct (`DEFAULT_SIDE_OFFSET` in `new`) and the builder method documented as in Task B12. In `show`:

```rust
        let width = self.width.unwrap_or_else(|| px(|r| &r.width));
        let inner_width = (width - pad_l - pad_r).max(0.0);
```

and `.gap(self.side_offset)`. Update the `width` doc to `default: the style's `w-72``.

`dialog.rs`: delete `const FOOTER_GAP` and its `// base:` line; `let gap = pick(&l, |r| &r.gap, g).unwrap_or(0.0);`. Leave the Kind 2 border fallback (`dialog.rs:283`) for Task B16. Remove the other `// base:` comments in `dialog.rs` and update both module docs.

- [ ] **Step 4: Run tests and snapshots**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`
Expected: PASS; no snapshot changed. Otherwise: snapshot rule.

- [ ] **Step 5: Commit**

```bash
git add crates/shadcn-egui/src/components/popover.rs crates/shadcn-egui/src/components/dialog.rs crates/shadcn-egui/tests/recipes.rs
git commit -m "popover, dialog: width and footer gap from the style

The popover's 288 px default width and the dialog footer's 8 px gap were
fixed in code; they now come from w-72 and gap-2 in the recipes.
sideOffset is a prop of upstream's PopoverContent, so it becomes a builder
method with upstream's default of 4.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B16: Stricter `tokens.rs`, Kind 2 annotations, cleanup, second measurement

**Files:**
- Modify: `crates/shadcn-egui/tests/tokens.rs`
- Modify: `crates/shadcn-egui/src/components/{button,badge,card,dialog,label,select,input,icon,checkbox,slider,shared}.rs`, `crates/shadcn-egui/src/layout.rs` (annotations, `// base:` removal)
- Modify: `tools/stylegen/src/drift.rs`, `tools/stylegen/tests/drift.rs`, `tools/stylegen/markers.txt` (remove `base_quotes`)
- Modify: `docs/superpowers/plans/2026-10-13-recipe-completeness.md` (Ledger "after")

**Interfaces:**
- Removes: `shared::disabled_painter`, `drift::base_quotes`.

- [ ] **Step 1: Write the stricter scanner and its tests**

In `crates/shadcn-egui/tests/tokens.rs`, replace `strict_violations` and its doc with:

```rust
/// Strict scan for components: besides colours and font sizes, literal
/// geometry, shadow definitions, alpha multipliers and palette roles. Every
/// visual value comes from the style (a recipe or the theme metrics); a
/// line may carry one only with a `// behaviour-ok: <reason>` comment on
/// the same line, for behaviour constants and inherited fallbacks. `0.0` is
/// free; `1.0` and `2.0` only as factors (`* 2.0`, `/ 1.0`), not as a
/// length (`- 1.0`, `Stroke::new(1.0, ..)`). A `// base:` quote exempts
/// nothing and is reported itself: base classes live in the recipes.
fn strict_violations(source: &str) -> Vec<String> {
    let mut out = violations(source);
    let mut in_icon_data = false;
    for (n, line) in source.lines().enumerate() {
        // Lucide path data: `pub const NAME: Icon = Icon {` ... `};`.
        if line.starts_with("pub const ") && line.trim_end().ends_with(": Icon = Icon {") {
            in_icon_data = true;
        }
        if in_icon_data {
            in_icon_data = line.trim_end() != "};";
            continue;
        }
        let code = line.split("//").next().unwrap_or("");
        let reason = line
            .split("// behaviour-ok:")
            .nth(1)
            .is_some_and(|r| !r.trim().is_empty());
        let stale_quote = line.contains("// base:");
        let role = code.contains("Role::");
        let shadow = code.contains("Shadow {");
        let alpha = [
            "gamma_multiply(",
            "linear_multiply(",
            "from_black_alpha(",
            "from_white_alpha(",
        ]
        .iter()
        .any(|p| {
            code.split(p)
                .skip(1)
                .any(|r| r.starts_with(|c: char| c.is_ascii_digit()))
        });
        let geometry = float_literals(code).any(|(lit, before)| {
            let factor = before.trim_end().ends_with(['*', '/']);
            match lit {
                "0.0" => false,
                "1.0" | "2.0" => !factor,
                _ => true,
            }
        });
        let flagged = (role || shadow || alpha || geometry) && !reason;
        if stale_quote || (flagged && !line.trim_start().starts_with("//")) {
            out.push(format!("{}: {}", n + 1, line.trim()));
        }
    }
    out
}
```

Replace `strict_scanner_catches_literals` with:

```rust
#[test]
fn strict_scanner_catches_literals() {
    let n = |s: &str| strict_violations(s).len();
    assert_eq!(n("let h = 32.0;"), 1);
    assert_eq!(n("let c = fill.gamma_multiply(0.5);"), 1);
    assert_eq!(n("let s = Shadow { offset: [0, 1], blur: 3, spread: 0, color };"), 1);
    // A base quote exempts nothing and is reported itself.
    assert_eq!(n("// base: \"disabled:opacity-50\"\nlet c = fill.gamma_multiply(0.5);"), 2);
    assert_eq!(n("let c = rect.height() / 2.0;"), 0);
    assert_eq!(n("let k = x * 1.0;"), 0);
    assert_eq!(n("let w = 2.0 + x;"), 1);
    assert_eq!(n("let s = Stroke::new(1.0, c);"), 1);
    assert_eq!(n("let h = x - 1.0;"), 1);
    assert_eq!(n("let z = 0.0;"), 0);
    assert_eq!(n("let o = 1.0_f32; let p = 1.0f64;"), 1);
    assert_eq!(n("let h = 32.0_f32;"), 1);
    assert_eq!(n("let v = vec2(x, 0.0);"), 0);
    assert_eq!(n("let o = v.clamp(0.0, 1.0);"), 1);
    assert_eq!(n("let o = v.clamp(0.0, 1.0); // behaviour-ok: fraction of the range"), 0);
    assert_eq!(n("let fg = t.role(Role::Foreground);"), 1);
    assert_eq!(n("let fg = t.role(Role::Foreground); // behaviour-ok: inherits Foreground"), 0);
    assert_eq!(n("let t = 0.1; // behaviour-ok: flattening tolerance"), 0);
    assert_eq!(n("let t = 0.1; // behaviour-ok:"), 1);
    assert_eq!(n("pub const X: Icon = Icon {\n    a: 3.5,\n};\nlet h = 32.0;"), 1);
}
```

In `ported_components_take_values_from_the_style`, skip the rasterizer:

```rust
    for path in files {
        // A rasterizer, not a component: its numbers are pixel arithmetic.
        if path.ends_with("icon_raster.rs") {
            continue;
        }
```

Update the module doc of `tokens.rs` and the doc of `non_test_code_cuts_only_the_trailing_test_module` only if they mention `// base:`.

- [ ] **Step 2: Run to see the new violations**

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --test tokens -- --test-threads 4`
Expected: `strict_scanner_catches_literals` PASSES; `ported_components_take_values_from_the_style` FAILS and lists every line to fix. Expected entries (fix each, then re-run): the Kind 2 fallbacks `button.rs` (text `Foreground`), `badge.rs` (text `Foreground`), `select.rs` (trigger text `Foreground`, item text `PopoverForeground`), `label.rs` (text `Foreground`, `description()` `MutedForeground`), `card.rs` and `dialog.rs` (border `Border`); `icon.rs` (`.max(1.0)` twice per line, `pos2(1.0, 1.0)`); `label.rs` (`unwrap_or(1.0)`); any `// base:` comment left in `card.rs`, `dialog.rs`, `label.rs`, `layout.rs`. A line not in this list is a value still fixed in code: STOP and report it before deciding whether it is Kind 1 (move it to a recipe in its own commit, snapshot rule applies) or Kind 2.

- [ ] **Step 3: Fix the listed lines**

Kind 2 fallbacks: put the inherited colour on its own line with the reason the spec gives, for example in `button.rs`:

```rust
        let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground; upstream sets no text colour
        let fg = paint(|r| &r.text).unwrap_or(inherited);
```

in `card.rs` and `dialog.rs`:

```rust
            let base_border = t.role(Role::Border); // behaviour-ok: shadcn base layer `* { @apply border-border }`
```

in `label.rs` `description()`: `let muted = t.role(Role::MutedForeground); // behaviour-ok: inherits; reads card_description first`. In `label.rs` `ui`, replace `disabled_painter(ui, self.enabled, get(|r| &r.opacity).unwrap_or(1.0))` with `style_painter(ui, opacity(&[&l[..]], st))`. In `icon.rs`, annotate `// behaviour-ok: at least one device pixel` on the `.max(1.0)` lines and `// behaviour-ok: the whole texture (UV unit square)` on the `uv` line. Delete every remaining `// base:` comment and update module docs that still say "Base classes are quoted under `// base:`". Remove `disabled_painter` from `shared.rs` once `grep -rn disabled_painter crates/shadcn-egui/src` is empty. Run `cargo fmt --all` and re-run the tokens test: a `// behaviour-ok:` that rustfmt moved off its line shows up again.

- [ ] **Step 4: Remove the base-quote drift check**

`tools/stylegen/src/drift.rs`: delete `pub fn base_quotes` and, in `check`, the block that reads the crate file and pushes "base quote no longer upstream" problems (drop the `crate_root` parameter only if nothing else uses it; keep `main.rs` compiling). Update the module doc to: `Drift between the crate and the pinned upstream: `cn-*` markers that appeared or disappeared in the upstream component sources.` In `tools/stylegen/tests/drift.rs`, delete `finds_base_quotes`, and in `reports_stale_quote_and_marker_change` (rename it `reports_marker_changes`) drop the `button.rs` crate file write and expect `problems.len() == 2`.

- [ ] **Step 5: Run every gate**

```bash
systemd-run --user --scope -p MemoryMax=4G -- cargo test --workspace --jobs 4 -- --test-threads 4
systemd-run --user --scope -p MemoryMax=4G -- cargo test --workspace --all-features --jobs 4 -- --test-threads 4
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
cargo clippy --workspace --all-targets --all-features --jobs 4 -- -D warnings
cargo fmt --all --check
for t in stylegen icongen fontgen; do
  systemd-run --user --scope -p MemoryMax=4G -- cargo test --manifest-path tools/$t/Cargo.toml --jobs 4 -- --test-threads 4
  cargo clippy --manifest-path tools/$t/Cargo.toml --all-targets --jobs 4 -- -D warnings
  cargo fmt --check --manifest-path tools/$t/Cargo.toml
done
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift "$SVELTE" "$REACT" .
grep -rn "// base:" crates/shadcn-egui/src || echo "no base quotes"
git status --short crates/shadcn-egui/tests/snapshots
```
Expected: all pass, `drift: clean`, `no base quotes`, no snapshot modified beyond those approved earlier.

- [ ] **Step 6: Second measurement**

Run Task B1 Step 3 again (same command, same frame count, two runs, a third on a > 10 % spread) and fill the "after" columns of the Ledger. If a style's median grew by more than 20 % or its allocations per frame grew at all, report it to the maintainer with both rows before committing; do not optimise without their go-ahead.

- [ ] **Step 7: Commit**

```bash
git add -A crates/shadcn-egui tools/stylegen docs/superpowers/plans/2026-10-13-recipe-completeness.md
git commit -m "tokens: no literal colour or size in a component

With base classes in the recipes a // base: quote no longer marks a
legitimate number, so it exempts nothing; a palette role and a 1.0 used
as a length now need a behaviour-ok reason. The remaining roles are the
inherited fallbacks the spec lists. drift's base-quote check goes with the
quotes.

Frame benchmark, before -> after (3000 frames, release, one thread):
<paste the Ledger rows here>

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task B17: Skills, CHANGES, pull request B

**Files:**
- Modify: `skills/build-ui/SKILL.md` (restyling section), `skills/build-ui/references/component-map.md:22`
- Modify: `skills/port-component/SKILL.md` (steps around lines 25-80), `skills/port-component/references/upstream.md` (Markers, Tokens, Done), `skills/port-component/references/styles.md` (lines 15-30, 149), `skills/port-component/references/in-app.md` (lines 115-145)
- Modify: `CHANGES.md` (`[Unreleased]`)

**Interfaces:**
- Consumes: `Rule::important`, `When::all`, `Preset::theme(Mode) -> Theme`, `Theme::apply`.

- [ ] **Step 1: build-ui: restyle through `theme.style`**

Add a section `## Restyle a component part` to `skills/build-ui/SKILL.md`, after the theme setup section:

````markdown
## Restyle a component part

Every colour, size and spacing a component draws comes from its recipe in
`theme.style` (one `Recipe` per part, named after the shadcn part:
`tabs_trigger`, `button_variant_outline`, `separator`). To change one,
push an important rule: it beats every rule the style generated, in light
and dark, the way a Tailwind `!` class does.

```rust
use std::sync::Arc;
use shadcn_egui::style::{Paint, Role, Rule, State, When};
use shadcn_egui::{Mode, Preset};

fn setup(ctx: &egui::Context) {
    let mut theme = Preset::default().theme(Mode::Light);
    let style = Arc::make_mut(&mut theme.style);
    style.tabs_trigger.bg.rules.push(Rule::important(
        When::all(State::SELECTED),
        Paint::Role(Role::Primary, 1.0),
    ));
    style.tabs_trigger.text.rules.push(Rule::important(
        When::all(State::SELECTED),
        Paint::Role(Role::PrimaryForeground, 1.0),
    ));
    theme.apply(ctx);
}
```

Set `prop.base` only to replace an unconditional value; a state the style
styles (hover, dark, active) keeps its own rule unless an important rule
covers it.
````

In `component-map.md:22`, change "No marker: base classes only" to "No marker: the `separator` recipe (`data-slot`)".

- [ ] **Step 2: port-component and its references**

Rewrite the parts that describe base classes:
- `skills/port-component/SKILL.md`: replace step 4 ("Quote the base classes") with a step saying that stylegen generates base classes into the recipes from the svelte source (add the part to `markers.txt`, `slot:<data-slot>` for a part without a marker), and that a component holds no literal colour or size: it picks every value from its recipe stack; the only literals left carry `// behaviour-ok: <reason>` (behaviour constants, inherited fallbacks). Change line 30's sentence accordingly.
- `references/upstream.md`: the Markers step (slot entries, `extract::BRANCHES` for a part in several `{#if}` branches, `resolve::EXCEPTIONS` for a bits-ui element rule), the Tokens section (the rules of Task B16), the Done list (drift checks markers and the ui/ copies, no base quotes).
- `references/styles.md`: lines 15-30 and 149, base classes are part of the generated recipe; an empty `Prop` means upstream sets nothing.
- `references/in-app.md`: an app-local port writes its recipe by hand, so its recipe holds the base classes too; the `// base:` and `// style:` quotes stay as provenance comments in the app's recipe file, not as exemptions in drawing code.

Run: `systemd-run --user --scope -p MemoryMax=4G -- cargo test -p shadcn-egui --jobs 4 --doc -- --test-threads 4` and `... --test skills`
Expected: PASS (the new fence compiles).

- [ ] **Step 3: CHANGES**

Under `## [Unreleased]` in `CHANGES.md`:

```markdown
### New
- Every colour, size and spacing of every component can be changed through
  `theme.style`. `Rule::important(When::all(State::SELECTED), value)` adds a
  rule that beats the style's own, as a Tailwind `!` class does: for example a
  filled `primary` active tab with a solid 2 px focus ring. (#6)
- `Popover::side_offset` and `Select::side_offset` set the gap between the
  trigger and the popup; the default stays 4. (#6)

### Changed
- In the luma and rhea styles, a focused tab and the active tab in dark mode
  no longer draw a border, as upstream's `border-transparent!` says. (#6)
- The separator's thickness comes from the style's `separator` recipe. An app
  that changed `metrics.border` to make separators thicker now changes
  `separator.height` (horizontal) or `separator.width` (vertical) instead. (#6)
- `Recipe` and `Style` have new fields (`height_inset`, `outline_width`,
  `outline_color`, `separator`, `tabs_trigger_after`); code that builds them
  with a struct literal needs `..Default::default()`. (#6)
```

Add a CHANGES line for every approved snapshot difference recorded in the Ledger, in the reader's terms (what looks different, in which style).

- [ ] **Step 4: All gates once more**

Run every command of Task B16 Step 5, plus:

```bash
scripts/check-version-files.sh
scripts/check-packages.sh
scripts/check-package-size.sh
cargo publish --dry-run --workspace --jobs 4
```
Expected: all pass.

- [ ] **Step 5: Commit and open pull request B**

```bash
git add skills CHANGES.md
git commit -m "skills, CHANGES: restyling through theme.style (#6)

build-ui shows the Rule::important idiom; port-component says base classes
are generated and a component holds no literal colour or size. CHANGES
names the luma/rhea tab border and the separator thickness change.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push -u origin feat/recipe-data
gh pr create --title "Every component value from the style data (closes #6)" --body "$(cat <<'EOF'
Implements docs/superpowers/specs/2026-10-13-recipe-completeness-design.md
(PR B of the plan in docs/superpowers/plans/).

- stylegen translates each part's full class string (base classes included)
  from the pinned svelte component sources
- every Kind 1 value of the components comes from a recipe; Rule::important
  lets an app override any of them
- snapshots unchanged except the approved upstream disagreements listed in
  the spec
- frame benchmark before/after in the plan's Ledger

Closes #6.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```
