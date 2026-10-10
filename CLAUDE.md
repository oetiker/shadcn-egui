# CLAUDE.md

`egui_shadcn` is a Rust crate **and** a Claude Code plugin: it ports the shadcn-v4
(OKLCH) design system and its styles (vega, nova, …, generated from the upstream
CSS by `tools/stylegen`) to egui as a vendored, copy-paste component module. The canonical crate under `src/` is the source of truth;
`skills/egui-shadcn/registry/` is the copy the plugin vendors into target projects.

## Start here — read the source docs, don't restate them

- **What the skill does, how to *use* the components, and the standards for
  *porting* new ones (including dependency discipline):**
  [`skills/egui-shadcn/SKILL.md`](skills/egui-shadcn/SKILL.md)
- **Mapping tables** (shadcn token → egui field, web layout intent → egui helper,
  component → widget, and egui gotchas): `skills/egui-shadcn/references/`
- **Cutting a release** (version bumps, tags, the two repos involved):
  [`RELEASING.md`](RELEASING.md)
- **What changed per version:** [`CHANGES.md`](CHANGES.md)

## Project invariants

- `src/` and `skills/egui-shadcn/registry/src/` must stay **byte-identical** —
  change both in lockstep (edit `src/`, then copy into the registry).
- Keep runtime `[dependencies]` minimal — read the dependency-discipline section
  of `SKILL.md` before adding any.
- Any component change needs its kittest snapshot PNG regenerated and committed.

## Build & verify

- `cargo build` / `cargo test` / `cargo clippy --all-targets` — keep
  warning-clean. `tools/icongen` is its own workspace: test and lint it with
  `--manifest-path tools/icongen/Cargo.toml`. After changing it, regenerate
  `tests/icons/lucide.rs` (command in `references/icons.md`). On shared machines, cap parallelism (e.g. `cargo test --jobs 4`).
- Snapshot tests render headlessly (egui_kittest + wgpu, mesa software adapter —
  no display needed). Regenerate baselines with `UPDATE_SNAPSHOTS=1 cargo test`.
  The visual review loop (look at the PNG, iterate) is described in `SKILL.md`
  under "Verify visually".
