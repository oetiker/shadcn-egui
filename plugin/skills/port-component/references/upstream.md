# Porting into shadcn-egui

The component goes into `crates/shadcn-egui/src/components/<name>.rs` of a
shadcn-egui checkout and follows the standard in `SKILL.md`. This file lists
the repository tooling the port uses and what "done" means there.

## Checkouts

The generators read upstream source from two checkouts, kept in a scratch
directory outside the repository:

- shadcn-svelte (`https://github.com/huntabyte/shadcn-svelte`) at the commit
  in `tools/stylegen/PIN`.
- shadcn-ui (`https://github.com/shadcn-ui/ui`) at the commit in
  `tools/stylegen/PIN_REACT`; the palettes, presets and fonts come from there.

```bash
git clone https://github.com/huntabyte/shadcn-svelte <scratch>/shadcn-svelte
git -C <scratch>/shadcn-svelte checkout "$(cat tools/stylegen/PIN)"
git clone https://github.com/shadcn-ui/ui <scratch>/shadcn-ui
git -C <scratch>/shadcn-ui checkout "$(cat tools/stylegen/PIN_REACT)"
```

## Markers

1. Put the component's parts on its line in `tools/stylegen/markers.txt`:
   `<file>.rs <upstream ui dir>: <parts>`. A part is a `cn-*` marker, or
   `slot:<data-slot>` for a part without one: the one class string without
   a marker in `ui/<dir>/<data-slot>.svelte` (`separator.rs separator:
   slot:separator`). The generator reads each part's whole class string
   from the svelte source, base classes included, and puts the style's
   `@apply` classes in place of the marker. Three tables in the generator
   cover the cases the source alone does not decide:
   - `extract::BRANCHES` (`tools/stylegen/src/extract.rs`): a part whose
     marker appears in several `{#if}` branches names the occurrence the
     crate draws, with the reason (`cn-input`: the `{:else}` branch, as
     `input.rs` draws a text field, never `type="file"`). Without an entry
     the generator stops with an error.
   - `resolve::EXCEPTIONS` (`tools/stylegen/src/resolve.rs`): a variant
     that never matches on the element bits-ui renders is dropped, with the
     reason (`disabled:` on the slider thumb, a `<span>`).
   - `resolve::CORRECTIONS`, next to it: a design correction the
     maintainer approved edits a part's class string before translation.
     The spec of the change names it and `tools/stylegen/reports/<style>.md`
     shows it under the part.
2. Run the generator from the repository root:

   ```bash
   cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- generate <scratch>/shadcn-svelte <scratch>/shadcn-ui .
   ```

   It adds one `Style` field per new marker to
   `crates/shadcn-egui/src/style/generated.rs`, fills it in every exported
   style (`crates/shadcn-egui/src/style/<name>.rs`) and rewrites the reports
   in `tools/stylegen/reports/`. Generated files are never edited by hand;
   fix the generator and regenerate.
3. Read the component's markers in `tools/stylegen/reports/<style>.md`. An
   ignored class carries its reason and is fine. An unsupported class needs
   generator support before the port is done.

## Tokens

`crates/shadcn-egui/tests/tokens.rs` scans every file in
`crates/shadcn-egui/src/components/` and `crates/shadcn-egui/src/layout.rs`,
so a new component file is checked without registration. The scan stops at
the file's trailing `#[cfg(test)] mod tests` module. Every visual value
comes from the style (a recipe or `Metrics`), so a line is reported when it
holds:

- a colour literal (`Color32::from_*`, `Color32::WHITE`, …) or a literal
  font size (`FontId::proportional(14.0)`, `.size(14.0)`) without a
  `// token-ok: <reason>` comment on the line above;
- a palette role (`Role::Primary`), a `Shadow { .. }`, or an alpha
  multiplier with a literal (`gamma_multiply(0.5)`, `from_black_alpha(..)`);
- a float literal other than `0.0`, or `1.0` and `2.0` as anything but a
  factor: `* 2.0` and `/ 1.0` pass, `- 1.0` and `Stroke::new(1.0, ..)` do
  not;
- an integer literal other than `0` inside an egui geometry constructor
  (`CornerRadius`, `Margin`, `Stroke`, `Shadow`, `vec2`, `pos2`,
  `Vec2::new`, `Pos2::new`), with the same factor rule
  (`CornerRadius::same(4)` is reported);
- a `// base:` comment. Base classes live in the generated recipes, so a
  quote in drawing code is stale and is reported itself.

A role, shadow, alpha or number passes only with a
`// behaviour-ok: <reason>` comment on the same line, for a behaviour
constant of the primitive or an inherited fallback:

```rust
pub const DEFAULT_SIDE_OFFSET: f32 = 4.0; // behaviour-ok: upstream prop default, not a class
```

String literals and one-line `/* .. */` comments are not scanned; the lines
inside a multi-line block comment are. Neither are Lucide icon data blocks
(`pub const NAME: Icon = Icon {` to `};`) nor the icon rasterizer
`icon_raster.rs`, whose numbers are path data and rasterizer geometry.

## Snapshots

Render with `common::snapshot_all(name, size, ui_fn)` from
`crates/shadcn-egui/tests/common/mod.rs`: one PNG per style and mode
(`<name>_vega_dark.png`, `<name>_nova_light.png`, …). Generate them with
`UPDATE_SNAPSHOTS=1 cargo test -p shadcn-egui --jobs 4 -- --test-threads 4`.

Open every PNG the component appears in, also those of other tests (the
gallery, forms) that now look different, and compare each with the
shadcn-svelte docs page for that style. Re-baseline the PNGs the port
changes, and delete PNGs that no test writes any more.

## Done

- `stylegen drift` prints `drift: clean`:

  ```bash
  cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift <scratch>/shadcn-svelte <scratch>/shadcn-ui .
  ```

  It checks that the markers in `markers.txt` equal the markers of the
  upstream source, and that the copies of the component sources in
  `tools/stylegen/tests/fixtures/ui/` and of the style CSS match the
  checkout (`generate` writes them).
- No `// base:` comment in the component; every value comes from its
  recipes (`tokens.rs` passes).
- No unsupported class for the component's markers in
  `tools/stylegen/reports/<style>.md`.
- Snapshots reviewed, the behaviour list checked.
- A row in `plugin/skills/build-ui/references/component-map.md`, and the component
  removed from its "Not yet ported" list.
- A `CHANGES.md` entry under [Unreleased]: what the user can now use.
- Every gate under "Build & verify" in `CLAUDE.md` passes.

Then open a pull request.

## Icons

The icon crates `crates/shadcn-egui-icons-<lib>/` are generated by
`tools/icongen` from the pinned npm packages in `tools/icongen/SETS.tsv`;
never edit their `src/`:

```bash
tools/icongen/scripts/generate-crates.sh <scratch dir> [lib...]
```

Each crate holds every icon as a `pub const` (a range into one `icons.bin`
blob), `ALL`, and `SLOTS` for the checkbox tick, the select trigger and the
close button. Remix Icon stays at its last Apache-2.0 release. A component
that needs a new slot icon adds it to `IconSlots` in `shadcn-egui-core` and
to the `slots()` table in `generate-crates.sh`.

After changing icongen, regenerate the test fixtures:

```bash
cargo run --manifest-path tools/icongen/Cargo.toml --jobs 4 -- \
  --out crates/shadcn-egui/tests/icons/lucide.rs \
  --module-path shadcn_egui::components::icon \
  --provenance "Lucide v1.54.0 (ISC), test fixtures from tools/icongen/tests/fixtures" \
  tools/icongen/tests/fixtures/{mic,circle-alert,settings,x,power,monitor}.svg
cargo run --manifest-path tools/icongen/Cargo.toml --jobs 4 -- \
  --crate-dir crates/shadcn-egui/tests/icons/fixture \
  --slots check=mixed,chevron_down=ring,close=x \
  --provenance "icongen test fixtures (tools/icongen/tests/fixtures); x.svg is Lucide (ISC)" \
  tools/icongen/tests/fixtures/{big-box,mixed,ring,square-hole,star-evenodd,x}.svg
```

`scripts/check-package-size.sh` fails when any crate packages above 8 MB.
