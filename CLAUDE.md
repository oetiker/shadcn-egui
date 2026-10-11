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
- `cargo fmt --all --check`, and `cargo fmt --check --manifest-path
  tools/<tool>/Cargo.toml` for each tool.
- `tools/stylegen` and `tools/icongen` are separate workspaces: test and
  lint them with `--manifest-path tools/<tool>/Cargo.toml`. stylegen runs
  as `cargo run --manifest-path tools/stylegen/Cargo.toml -- generate|drift <shadcn-svelte checkout> .`
  from the repository root.
- Snapshot tests render headlessly (egui_kittest + wgpu, mesa software
  adapter). Regenerate baselines with `UPDATE_SNAPSHOTS=1 cargo test`, then
  look at every changed PNG.
