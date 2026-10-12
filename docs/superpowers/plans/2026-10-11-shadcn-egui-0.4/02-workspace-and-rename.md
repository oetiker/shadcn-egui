# Phase 1: Workspace and rename to `shadcn-egui`

> Part of [00-index.md](00-index.md). Read its Global Constraints first.

**Spec:** D1, D2, D3, D4, §3 (layout, core crate), §8 (imports).

**Outcome:** a Cargo workspace with `crates/shadcn-egui` (the moved crate,
renamed) and `crates/shadcn-egui-core` (`Icon`, `Seg`); the vendored
`skills/egui-shadcn/registry/` is gone and the byte-identical rule ends.
**No behaviour change**: every existing test and snapshot stays green
without `UPDATE_SNAPSHOTS`.

---

### Task 1.1: Move the crate into `crates/shadcn-egui` and rename it

**Files:**
- Move: `src/`, `tests/`, `examples/`, `assets/`, `Cargo.toml` → `crates/shadcn-egui/`
- Create: `Cargo.toml` (workspace root)
- Modify: `crates/shadcn-egui/Cargo.toml`
- Modify (sed): every `.rs` under `crates/` and `tools/` that names `egui_shadcn`

**Interfaces:**
- Produces: workspace root `Cargo.toml` with `[workspace.package]` (version
  `0.4.0`, edition, rust-version, license, repository) and
  `[workspace.dependencies]` (`egui`, `egui_extras`), used by every crate
  added later. Library path `shadcn_egui::…` replaces `egui_shadcn::…`.

- [ ] **Step 1: Move the crate**

```bash
mkdir -p crates/shadcn-egui
git mv src tests examples assets Cargo.toml crates/shadcn-egui/
```

- [ ] **Step 2: Write the workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["crates/*"]
# tools/stylegen, tools/icongen (and tools/fontgen from Phase 7) are their
# own workspaces and never dependencies of any crate here.
exclude = ["tools"]

[workspace.package]
version = "0.4.0"
edition = "2021"
rust-version = "1.92"
license = "MIT"
repository = "https://github.com/oetiker/shadcn-egui"

[workspace.dependencies]
egui = "0.34"
egui_extras = { version = "0.34", default-features = false }
```

- [ ] **Step 3: Rewrite `crates/shadcn-egui/Cargo.toml`**

```toml
[package]
name = "shadcn-egui"
description = "The shadcn/ui design system for egui: styles, palettes, presets and components"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

# Keep the runtime dependency surface minimal: these propagate to every
# consumer. The library only needs egui and egui_extras' StripBuilder/Size
# (no loaders, no features). Heavier crates (eframe, egui_kittest, wgpu) are
# dev-only.
[dependencies]
egui.workspace = true
egui_extras.workspace = true

[dev-dependencies]
eframe = "0.34"
egui_kittest = { version = "0.34", features = ["wgpu", "snapshot"] }

[[example]]
name = "settings"
```

- [ ] **Step 4: Rename the library path and egui ids**

```bash
grep -rlI 'egui_shadcn' crates tools | xargs sed -i 's/egui_shadcn/shadcn_egui/g'
sed -i 's/"egui-shadcn settings"/"shadcn-egui settings"/' crates/shadcn-egui/examples/settings.rs
grep -rn 'egui_shadcn\|egui-shadcn' crates tools/stylegen/src tools/icongen/src || echo "no old names left"
```

Expected: the last command prints only `tools/icongen/src/lib.rs` (its
generated-file header, changed in Task 1.2) or "no old names left". The
egui `Id` strings (`"shadcn_egui::active_theme"`, …) change too; ids are not
persisted, so nothing visible changes.

- [ ] **Step 5: Point stylegen at the new crate path**

`tools/stylegen/src/main.rs`: the crate lives below the repository root now:

```rust
//! stylegen generate <shadcn-svelte checkout> <repository root>
//! stylegen drift    <shadcn-svelte checkout> <repository root>

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use stylegen::{css, emit, markers, report, resolve};

/// The crate stylegen writes into, relative to the repository root.
const CRATE: &str = "crates/shadcn-egui";
```

In `main`, change the usage text to
`"usage: stylegen generate|drift <shadcn-svelte> <repo-root>"`. In
`generate`, change `let out_dir = root.join("src/style");` to
`let out_dir = root.join(CRATE).join("src/style");`. In `main`, change the
`"drift"` arm to `"drift" => drift(&up, &root.join(CRATE), &lines),`.

In `tools/stylegen/tests/generated.rs` and `tools/stylegen/tests/translate.rs`
replace `"../../src/style/` with `"../../crates/shadcn-egui/src/style/`:

```bash
sed -i 's#"\.\./\.\./src/style/#"../../crates/shadcn-egui/src/style/#g' tools/stylegen/tests/generated.rs tools/stylegen/tests/translate.rs
sed -i 's#read(&format!("\.\./\.\./src/style/#read(\&format!("../../crates/shadcn-egui/src/style/#' tools/stylegen/tests/generated.rs
grep -n 'src/style' tools/stylegen/tests/*.rs
```

Expected: every hit now reads `../../crates/shadcn-egui/src/style/…`.

- [ ] **Step 6: Run the gates**

```bash
cargo test --workspace --jobs 4 -- --test-threads 4
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift "$UP" .
```

Expected: all tests pass with the committed snapshots (kittest resolves
`tests/snapshots` relative to the package, which moved with its tests);
`tests/registry.rs` still passes because it only compares directories it
finds relative to `CARGO_MANIFEST_DIR`… it will **fail** (the registry is
now two levels up). That failure is expected and fixed in Task 1.3; confirm
it is the only failing test. Drift prints `drift: clean`.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: move the crate into a workspace as shadcn-egui

The crate moves to crates/shadcn-egui and is renamed shadcn-egui
(lib shadcn_egui); egui_shadcn is taken on crates.io. Workspace package
metadata sets version 0.4.0. stylegen writes into the new path. The
registry byte-identity test fails until the registry is removed in the
next commit.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 1.2: `shadcn-egui-core` with `Icon` and `Seg`

**Files:**
- Create: `crates/shadcn-egui-core/Cargo.toml`
- Create: `crates/shadcn-egui-core/src/lib.rs`
- Create: `crates/shadcn-egui-core/src/icon.rs`
- Modify: `Cargo.toml` (root: `[workspace.dependencies]`)
- Modify: `crates/shadcn-egui/Cargo.toml`, `crates/shadcn-egui/src/lib.rs`, `crates/shadcn-egui/src/components/icon.rs`
- Modify: `tools/icongen/src/lib.rs` (header and doc comments)
- Regenerate: `crates/shadcn-egui/tests/icons/lucide.rs`

**Interfaces:**
- Produces: `shadcn_egui_core::{Icon, Seg}` with the exact fields of today's
  `components::icon::{Icon, Seg}` (`Icon { name: &'static str, size: f32, segs: &'static [Seg] }`).
  `shadcn_egui` re-exports them at its root (`pub use shadcn_egui_core::*;`)
  and from `components::icon`, so `shadcn_egui::components::icon::Icon` keeps
  working.

- [ ] **Step 1: Write a failing test that names the core types through both paths**

Append to `crates/shadcn-egui/tests/icon.rs`:

```rust
#[test]
fn core_types_are_reexported() {
    let a: shadcn_egui::Icon = lucide::X;
    let b: shadcn_egui_core::Icon = a;
    let c: shadcn_egui::components::icon::Icon = b;
    assert_eq!(c.name, "x");
    assert!(matches!(c.segs[0], shadcn_egui::Seg::M(..)));
}
```

Run: `cargo test -p shadcn-egui --jobs 4 --test icon core_types_are_reexported -- --test-threads 4`
Expected: FAIL to compile (`shadcn_egui_core` unresolved, `shadcn_egui::Icon` not found).

- [ ] **Step 2: Create the core crate**

`crates/shadcn-egui-core/Cargo.toml`:

```toml
[package]
name = "shadcn-egui-core"
description = "Data types shared by shadcn-egui and its font and icon crates"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

# A leaf crate: font and icon crates depend on it, shadcn-egui re-exports it.
# It must stay free of dependencies (Cargo forbids cycles, also through
# optional ones).
[dependencies]
```

`crates/shadcn-egui-core/src/lib.rs`:

```rust
//! Data types shared by `shadcn-egui` and its font and icon crates.
//!
//! Users never name this crate: `shadcn-egui` re-exports everything in it.

mod icon;

pub use icon::{Icon, Seg};
```

`crates/shadcn-egui-core/src/icon.rs`: move the two type definitions out of
`crates/shadcn-egui/src/components/icon.rs` unchanged:

```rust
//! Vector icon data. Painting lives in `shadcn_egui::components::icon`.

/// One path command in icon units (the SVG's user space, origin top-left).
/// A `M` starts a new subpath; `Z` closes the current one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    M(f32, f32),
    L(f32, f32),
    Q(f32, f32, f32, f32),
    C(f32, f32, f32, f32, f32, f32),
    Z,
}

/// A generated icon: its name (for screen readers), the side of its square
/// view box, and its path commands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Icon {
    pub name: &'static str,
    pub size: f32,
    pub segs: &'static [Seg],
}
```

Root `Cargo.toml`, add to `[workspace.dependencies]`:

```toml
shadcn-egui-core = { path = "crates/shadcn-egui-core", version = "0.4.0" }
```

`crates/shadcn-egui/Cargo.toml`, add to `[dependencies]`:

```toml
shadcn-egui-core.workspace = true
```

- [ ] **Step 3: Re-export and remove the old definitions**

`crates/shadcn-egui/src/lib.rs`:

```rust
pub mod color;
pub mod components;
pub mod layout;
pub mod reference;
pub mod style;
pub mod theme;

pub use shadcn_egui_core::*;
pub use style::Style;
pub use theme::{theme, Metrics, Palette, Theme};
```

In `crates/shadcn-egui/src/components/icon.rs`, delete the `Seg` enum and
the `Icon` struct (with their doc comments) and change the imports to:

```rust
use crate::Theme;
pub use shadcn_egui_core::{Icon, Seg};
use egui::epaint::{CubicBezierShape, PathShape, QuadraticBezierShape};
use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
```

The `CLOSE`, `CHECK`, `CHEVRON_DOWN` consts stay where they are (Phase 6
moves them).

- [ ] **Step 4: Update icongen's header and docs, regenerate the test fixture**

In `tools/icongen/src/lib.rs`: the doc line 2 becomes
``//! `shadcn_egui::components::icon`.``, the `Seg` doc becomes
``/// Mirror of `shadcn_egui_core::Seg`.``, and the header line in
`render_module` becomes:

```rust
    out.push_str("// Generated by icongen (shadcn-egui/tools/icongen). Do not edit: re-run it.\n");
```

In `tools/icongen/src/main.rs` the default module path stays
`crate::components::icon`. Then:

```bash
cargo test --manifest-path tools/icongen/Cargo.toml --jobs 4 -- --test-threads 4
cargo run --manifest-path tools/icongen/Cargo.toml --jobs 4 -- \
  --out crates/shadcn-egui/tests/icons/lucide.rs \
  --module-path shadcn_egui::components::icon \
  --provenance "Lucide v1.54.0 (ISC), test fixtures from tools/icongen/tests/fixtures" \
  tools/icongen/tests/fixtures/{mic,circle-alert,settings,x,power,monitor}.svg
git diff --stat crates/shadcn-egui/tests/icons/lucide.rs
```

Expected: icongen tests pass; the fixture diff touches only the header line
(the `use` line already reads `shadcn_egui::components::icon` after Task 1.1).

- [ ] **Step 5: Run the test and the gates**

```bash
cargo test --workspace --jobs 4 -- --test-threads 4
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
```

Expected: `core_types_are_reexported` passes; only `registry_src_is_byte_identical` fails (Task 1.3).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: shadcn-egui-core holds the Icon and Seg types

Font and icon crates must not depend on shadcn-egui, so the shared data
types move into a dependency-free leaf crate that shadcn-egui re-exports.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 1.3: Delete the vendored registry; update CLAUDE.md

**Files:**
- Delete: `skills/egui-shadcn/registry/` (whole tree, tracked in git)
- Delete: `crates/shadcn-egui/tests/registry.rs`
- Modify: `CLAUDE.md`

**Interfaces:**
- Produces: the rule "`src/` and `registry/src/` stay byte-identical" no
  longer exists anywhere (CLAUDE.md, tests). D16: the skill text still
  mentions `registry/`; spec 4 rewrites it, do not edit `skills/` here.

- [ ] **Step 1: Remove the registry and its test**

```bash
git rm -r -q skills/egui-shadcn/registry crates/shadcn-egui/tests/registry.rs
```

(`git rm` of tracked files on the feature branch; recoverable from history.)

- [ ] **Step 2: Rewrite `CLAUDE.md`**

```markdown
# CLAUDE.md

`shadcn-egui` is a Cargo workspace that ports the shadcn-v4 (OKLCH) design
system and its styles (vega, nova, …, generated from the upstream CSS by
`tools/stylegen`) to egui as a library published on crates.io. The repository
also hosts the `egui-shadcn` Claude Code plugin (`skills/egui-shadcn/`,
`.claude-plugin/`), frozen at 0.3.2 until the skill is rewritten for the
crate.

## Layout

- `crates/shadcn-egui/` — theme, styles, palettes, presets, components,
  tests, example. The library users depend on.
- `crates/shadcn-egui-core/` — dependency-free data types (`Icon`, `Seg`,
  later `IconSlots`, `FontFiles`) shared with the font and icon crates.
- `tools/stylegen`, `tools/icongen` — generators, each its own workspace;
  never dependencies of a crate.

## Start here — read the source docs, don't restate them

- **Using and porting components, dependency discipline:**
  [`skills/egui-shadcn/SKILL.md`](skills/egui-shadcn/SKILL.md) (still
  describes the 0.3 vendoring flow; the porting standards apply unchanged
  to `crates/shadcn-egui/src/`).
- **Mapping tables:** `skills/egui-shadcn/references/`.
- **Cutting a release:** [`RELEASING.md`](RELEASING.md).
- **What changed per version:** [`CHANGES.md`](CHANGES.md).

## Project invariants

- Generated files are never edited by hand; fix the generator and
  regenerate: `crates/shadcn-egui/src/style/{generated,<style>}.rs`
  (stylegen) and `crates/shadcn-egui/tests/icons/lucide.rs` (icongen).
- Keep runtime `[dependencies]` minimal — read the dependency-discipline
  section of `SKILL.md` before adding any.
- Any component change needs its kittest snapshot PNG regenerated, reviewed
  and committed.
- The repository is public: no private project names or paths.

## Build & verify

- `cargo build|test|clippy --workspace --all-targets --jobs 4` — keep
  warning-clean (`-- -D warnings` for clippy); `cargo test` also
  `-- --test-threads 4`.
- `tools/stylegen` and `tools/icongen` are separate workspaces: test and
  lint them with `--manifest-path tools/<tool>/Cargo.toml`. stylegen runs
  as `cargo run --manifest-path tools/stylegen/Cargo.toml -- generate|drift <shadcn-svelte checkout> .`
  from the repository root.
- Snapshot tests render headlessly (egui_kittest + wgpu, mesa software
  adapter). Regenerate baselines with `UPDATE_SNAPSHOTS=1 cargo test`, then
  look at every changed PNG.
```

- [ ] **Step 3: Gates**

```bash
cargo test --workspace --jobs 4 -- --test-threads 4
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4
cargo clippy --manifest-path tools/stylegen/Cargo.toml --all-targets --jobs 4 -- -D warnings
cargo test --manifest-path tools/icongen/Cargo.toml --jobs 4 -- --test-threads 4
cargo clippy --manifest-path tools/icongen/Cargo.toml --all-targets --jobs 4 -- -D warnings
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift "$UP" .
git status --short crates/shadcn-egui/tests/snapshots   # must print nothing
```

Expected: everything green, `drift: clean`, no snapshot file changed. Test
count = Phase-0 count − 1 (`registry_src_is_byte_identical`) + 1
(`core_types_are_reexported`); verify the arithmetic from the output.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor: drop the vendored registry copy

The crate is a normal dependency from 0.4 on (D1), so the copy under
skills/egui-shadcn/registry and its byte-identity test go away. CLAUDE.md
describes the workspace. The skill text keeps pointing at the old flow
until it is rewritten (D16).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
