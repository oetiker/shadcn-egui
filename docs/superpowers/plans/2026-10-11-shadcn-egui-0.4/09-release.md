# Phase 8 — Release: metadata, publish loop, docs, the user's steps

> Part of [00-index.md](00-index.md). Read its Global Constraints first.

**Spec:** D10, D11, D15, D16, §8 (migration, CHANGES), §9 (release
sequence, package metadata, publish loop, rate limit).
**Cites:** `docs/superpowers/notes/0.4/p0d-crates-io.md` (rate limits,
crate names, Phase 0 Task 4), `tools/fontgen/FONTS.tsv` and
`tools/icongen/SETS.tsv` (licences, P0c), the repo-infra skill's release
flow (`repo-infra:repo-infra`, reference `release-flow.md`).

**Outcome:** every crate carries `description`, `repository`, the SPDX
licence its contents need and an `include` list that keeps tests and
snapshots out, checked by `scripts/check-packages.sh`;
`cargo publish --dry-run --workspace` passes; `scripts/publish-crates.sh`
publishes in dependency order and skips versions crates.io already has;
README, RELEASING.md, CLAUDE.md and CHANGES.md describe 0.4.0. Tasks 8.1,
8.5 and 8.6 are the user's: nothing in them runs without the user's
explicit go-ahead.

**What the repo-infra standard changes in the index's sketch** (read
before Tasks 8.4–8.6):

- Under repo-infra, nobody pushes a tag. A release is: dispatch the
  **Create release PR** workflow (it rolls `## [Unreleased]` in CHANGES.md
  into `## [0.4.0] - <date>`, writes the version files, runs CI and opens a
  pull request), then merge that pull request; the publish workflow tags
  `v0.4.0` and runs the publish jobs. "Tag v0.4.0" in the index means this.
- repo-infra's own crates.io piece (`ri-publish-crates-io`) authenticates
  with crates.io Trusted Publishing and runs `cargo publish --workspace`
  after excluding versions crates.io has. Trusted Publishing is configured
  per existing crate, so it cannot create the ~33 new crates of 0.4.0. The
  0.4.0 publish job is therefore a repository-owned job that runs
  `scripts/publish-crates.sh` with the `CARGO_REGISTRY_TOKEN` secret (as the
  index designs it). Task 8.5 has the user confirm this on crates.io.

**Phase review focus:**

1. A publish run stopped half-way (HTTP 429, a network error) and re-run:
   it must skip what is out and continue, never fail on "already exists" —
   `scripts/publish-crates.sh` checks each version first (Task 8.3, DRY_RUN
   check against a crate that is already on crates.io).
2. A crate whose package contains `tests/snapshots` or lacks its licence
   files: `scripts/check-packages.sh` (Task 8.2).
3. A workspace member the publish order does not know (a new crate family):
   the script must refuse before publishing anything (Task 8.3, Step 4).
4. Crates at different versions in one workspace: refused (Task 8.3).
5. The README's preset code going stale when the decoder changes:
   `tests/readme.rs` (Task 8.4).

---

### Task 8.1: STOP (user) — pin the marketplace plugin to v0.3.2

**Files:** none in this repository.

**Interfaces:**
- Produces: the `egui-shadcn` entry of `oposs/claude-plugins` resolves the
  plugin at tag `v0.3.2`, so pushing `main` no longer ships the plugin
  (D11). `main` must not be pushed before this is done.

- [ ] **Step 1: Read the current entry (read-only)**

```bash
gh api repos/oposs/claude-plugins/contents/.claude-plugin/marketplace.json -q .content | base64 -d \
  | jq '.plugins[] | select(.name == "egui-shadcn")'
```

Expected: an entry whose `source` names `oetiker/egui-shadcn` with no
`ref`.

- [ ] **Step 2: STOP — ask the user**

Tell the user:

> Before anything of 0.4 reaches `main`, the `egui-shadcn` entry in
> `oposs/claude-plugins` (`.claude-plugin/marketplace.json`) must pin the
> plugin to `v0.3.2`, e.g. `"source": { "source": "github", "repo":
> "oetiker/egui-shadcn", "ref": "v0.3.2" }`. The marketplace tracks
> `main` HEAD today and would otherwise ship a skill whose `registry/` is
> gone. This is a change in another repository: will you make it (or
> shall I open a pull request there)?

Do not edit, push to or open a pull request in `oposs/claude-plugins`
without the user's explicit yes. Wait for the user to confirm the pin has
landed.

- [ ] **Step 3: Verify**

Re-run Step 1. Expected: the entry carries `"ref": "v0.3.2"`. Record the
confirmation in the phase's commit message of Task 8.2 (`Marketplace
pinned to v0.3.2 (oposs/claude-plugins).`).

---

### Task 8.2: Package metadata, include lists, licence files, dry-run gate

**Files:**
- Create: `scripts/check-packages.sh`
- Create: `LICENSE` copies: `crates/shadcn-egui/LICENSE`, `crates/shadcn-egui-core/LICENSE`, `crates/shadcn-egui-icons-*/LICENSE`
- Modify: `crates/shadcn-egui/Cargo.toml`, `crates/shadcn-egui-core/Cargo.toml`, `crates/shadcn-egui-icons-*/Cargo.toml`
- Modify: `tools/fontgen/src/lib.rs` (`crate_cargo_toml`, `generate`), `tools/fontgen/tests/render.rs`
- Modify (generated): `crates/shadcn-egui-font-*/Cargo.toml`, `crates/shadcn-egui-font-*/LICENSE`

**Interfaces:**
- Consumes: `cargo metadata`, `cargo package --list`; FONTS.tsv column 4
  and SETS.tsv column 5 (licences); `scripts/check-package-size.sh` (Phase 6).
- Produces (NEW INTERFACE): `scripts/check-packages.sh` — exits 0 and
  prints `check-packages: ok` when every workspace crate has a description,
  the expected SPDX licence (`MIT` for `shadcn-egui` and
  `shadcn-egui-core`; `MIT AND <licence>` for font and icon crates, plain
  `MIT` when the data licence is MIT), a `LICENSE` identical to the root
  one, its data licence file, and no `tests/`, `examples/` or `benches/`
  in its package. Every crate directory holds a copy of the root `LICENSE`
  (NEW INTERFACE: fontgen writes it for font crates).

- [ ] **Step 1: Write the check (the failing test)**

`scripts/check-packages.sh`:

```bash
#!/usr/bin/env bash
# What each workspace crate would upload to crates.io: a description, the
# SPDX licence its contents need, the licence texts, and no tests or
# snapshots. Run from anywhere inside the repository.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
fail=0
bad() { echo "check-packages: $*" >&2; fail=1; }
data_licence() { awk -F'\t' -v k="$2" -v c="$3" 'NR > 1 && $1 == k { print $c }' "$1"; }
spdx() { if [ "$1" = MIT ]; then echo MIT; else echo "MIT AND $1"; fi; }

meta=$(cargo metadata --no-deps --format-version 1)
while IFS=$'\t' read -r name license has_description; do
  case "$name" in
    shadcn-egui|shadcn-egui-core) want=MIT ;;
    shadcn-egui-font-*) want=$(spdx "$(data_licence tools/fontgen/FONTS.tsv "${name#shadcn-egui-font-}" 4)") ;;
    shadcn-egui-icons-*) want=$(spdx "$(data_licence tools/icongen/SETS.tsv "${name#shadcn-egui-icons-}" 5)") ;;
    *) bad "$name: not a known crate family (core, font, icons, shadcn-egui)"; continue ;;
  esac
  [ "$license" = "$want" ] || bad "$name: license is '$license', expected '$want'"
  [ "$has_description" = yes ] || bad "$name: no description"
  cmp -s LICENSE "crates/$name/LICENSE" || bad "$name: crates/$name/LICENSE is missing or differs from LICENSE"
  if ! files=$(cargo package --list --allow-dirty -p "$name" 2>/dev/null); then
    bad "$name: cargo package --list failed"
    continue
  fi
  for f in Cargo.toml src/lib.rs LICENSE; do
    grep -qx "$f" <<< "$files" || bad "$name: package lacks $f"
  done
  if grep -qE '^(tests|examples|benches)/' <<< "$files"; then
    bad "$name: package contains tests, examples or benches"
  fi
  case "$name" in
    shadcn-egui)
      grep -qx README.md <<< "$files" || bad "$name: package lacks README.md" ;;
    shadcn-egui-font-*)
      grep -qx LICENSE-FONT.txt <<< "$files" || bad "$name: package lacks LICENSE-FONT.txt"
      grep -qE '^fonts/.+\.ttf$' <<< "$files" || bad "$name: package lacks fonts/*.ttf" ;;
    shadcn-egui-icons-*)
      grep -qx LICENSE-ICONS.txt <<< "$files" || bad "$name: package lacks LICENSE-ICONS.txt"
      grep -qx src/icons.bin <<< "$files" || bad "$name: package lacks src/icons.bin" ;;
  esac
done < <(jq -r '.packages[] | [.name, (.license // "-"), (if (.description // "") == "" then "no" else "yes" end)] | @tsv' <<< "$meta")
if [ "$fail" = 0 ]; then echo "check-packages: ok"; fi
exit "$fail"
```

```bash
chmod +x scripts/check-packages.sh
```

- [ ] **Step 2: Run it to see it fail**

Run: `scripts/check-packages.sh`
Expected: exit 1, with at least: `crates/<name>/LICENSE is missing` for
every crate, `shadcn-egui: package contains tests, examples or benches`,
and `package lacks LICENSE` for every crate.

- [ ] **Step 3: `shadcn-egui` and core metadata**

`crates/shadcn-egui/Cargo.toml`, in `[package]` after
`repository.workspace = true`:

```toml
readme = "../../README.md"
keywords = ["egui", "shadcn", "gui", "theme", "design-system"]
categories = ["gui"]
include = ["src/**", "LICENSE*", "README*"]
```

and directly after the `[package]` table:

```toml
[package.metadata.docs.rs]
all-features = true
```

`crates/shadcn-egui-core/Cargo.toml`, `[package]` (keep the existing
lines; set or add these):

```toml
description = "Data types shared by shadcn-egui and its font and icon crates"
license.workspace = true
include = ["src/**", "LICENSE*", "README*"]
```

```bash
cp LICENSE crates/shadcn-egui/LICENSE
cp LICENSE crates/shadcn-egui-core/LICENSE
```

- [ ] **Step 4: Icon crates**

For every `crates/shadcn-egui-icons-<lib>/Cargo.toml` (hand-written in
Phase 6), make `[package]` hold these lines, `<Title>` being `Lucide`,
`Tabler`, `Phosphor`, `Remix Icon` or `Hugeicons`:

```toml
description = "<Title> icons for shadcn-egui"
license = "<SPDX>"
include = ["src/**", "LICENSE*", "README*"]
```

`<SPDX>`: Lucide `MIT AND ISC`, Tabler `MIT`, Phosphor `MIT`, Remixicon
`MIT AND Apache-2.0`, Hugeicons `MIT` when SETS.tsv says `MIT` (otherwise
`MIT AND <SETS.tsv licence>`). `src/**` covers `src/icons.bin`.

```bash
for d in crates/shadcn-egui-icons-*; do cp LICENSE "$d/LICENSE"; done
```

- [ ] **Step 5: Font crates through fontgen**

The font crates are generated, so the change goes into fontgen. Add to
`apache_and_static_crates_render` in `tools/fontgen/tests/render.rs`:

```rust
    assert!(
        crate_cargo_toml(&rows[1]).contains(
            "repository.workspace = true\ninclude = [\"src/**\", \"fonts/**\", \"LICENSE*\", \"README*\"]\n"
        ),
        "{}",
        crate_cargo_toml(&rows[1])
    );
```

Run: `cargo test --manifest-path tools/fontgen/Cargo.toml --jobs 4 --test render -- --test-threads 4`
Expected: FAIL in `apache_and_static_crates_render`.

In `crate_cargo_toml` in `tools/fontgen/src/lib.rs`, the template line
`repository.workspace = true` becomes:

```
repository.workspace = true
include = ["src/**", "fonts/**", "LICENSE*", "README*"]
```

In `generate`, after the line that writes `LICENSE-FONT.txt`, add:

```rust
        let code_licence = std::fs::read(repo.join("LICENSE")).map_err(|e| format!("LICENSE: {e}"))?;
        write(&dir.join("LICENSE"), &code_licence)?;
```

Regenerate (the downloads from Phase 7 are still under `$SCRATCH`;
`fetch.sh` re-fetches only what is missing):

```bash
export GFDL="$SCRATCH/gfonts-$(cut -c1-12 tools/fontgen/GF_PIN)"
tools/fontgen/fetch.sh "$GFDL"
cargo run --manifest-path tools/fontgen/Cargo.toml --jobs 4 -- "$GFDL" .
cargo test --manifest-path tools/fontgen/Cargo.toml --jobs 4 -- --test-threads 4
git status --short crates | grep -v -E 'shadcn-egui-font-[^/]+/(Cargo.toml|LICENSE)$' || echo "only Cargo.toml and LICENSE of font crates changed"
```

Expected: fontgen 17 passed; the last line prints `only Cargo.toml and
LICENSE of font crates changed` (apart from the files of Steps 3–4).

- [ ] **Step 6: Run the checks**

```bash
scripts/check-packages.sh
scripts/check-package-size.sh
cargo publish --dry-run --workspace --jobs 4
```

Run the dry run with a 600000 ms timeout; if it does not finish in that
time, run it in the background and poll it until it exits before ending
the turn. Expected: `check-packages: ok`; the size gate passes; the dry run
packages and verifies every crate in dependency order and ends with
`warning: aborting upload due to dry run`. Warnings that cargo ignores the
`settings` example and the test targets of `shadcn-egui` because they are
not in the package are expected (the include list keeps them out on
purpose).

- [ ] **Step 7: Full gates and commit**

Run every gate from the index (both feature sets, clippy, the three tools,
drift, `scripts/check-package-size.sh`) plus `scripts/check-packages.sh`.

```bash
git add -A
git commit -m "build: crates.io metadata, include lists and licence files

Every crate gets a description, the SPDX licence of its contents and an
include list that keeps tests and snapshots out; each crate ships the MIT
LICENSE and, for font and icon crates, the data licence.
scripts/check-packages.sh checks all of it from cargo package --list.
Marketplace pinned to v0.3.2 (oposs/claude-plugins).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8.3: `scripts/publish-crates.sh`

**Files:**
- Create: `scripts/publish-crates.sh`

**Interfaces:**
- Consumes: `cargo metadata --no-deps`; the crates.io API
  `https://crates.io/api/v1/crates/<name>/<version>` (200 = published,
  404 = not yet); `CARGO_REGISTRY_TOKEN` (only for a real publish); the
  P0d notes (rate limits, for the header comment).
- Produces (NEW INTERFACE beyond the index: `PLAN_ONLY=1`):
  - `scripts/publish-crates.sh` — publishes and ends with `all published`;
    exits 2 on HTTP 429 (re-run later), 1 on any other error.
  - `DRY_RUN=1` — asks crates.io and prints `skip <name> <v>` /
    `publish <name> <v> (dry run)`; publishes nothing.
  - `PLAN_ONLY=1` — prints `<name> <v>` per crate in publish order, no
    network.
  - `JOBS` (default 4) — passed to `cargo publish --jobs`.

- [ ] **Step 1: Run the missing script (the failing test)**

Run: `PLAN_ONLY=1 scripts/publish-crates.sh`
Expected: `No such file or directory`.

- [ ] **Step 2: Write the script**

Read the four numbers (burst and refill for new crates, burst and refill
for new versions), the date read and the expected new-crate count from
`docs/superpowers/notes/0.4/p0d-crates-io.md`, and put them into the
`{P0D_…}` fields of the header below when you write the file.

`scripts/publish-crates.sh`:

```bash
#!/usr/bin/env bash
# Publishes every crate of the workspace to crates.io in dependency order:
# shadcn-egui-core, then the font and icon crates, then shadcn-egui. Each
# version crates.io already has is skipped, so a run that stopped part-way
# is finished by running it again until it prints "all published".
#
# crates.io rate limits (read {P0D_DATE}, see
# docs/superpowers/notes/0.4/p0d-crates-io.md):
#   new crates:          burst {P0D_NEW_BURST}, then one per {P0D_NEW_REFILL}
#   new versions:        burst {P0D_UPDATE_BURST}, then one per {P0D_UPDATE_REFILL}
# The first release creates {P0D_NEW_CRATES} new crates, so it stops with
# HTTP 429 after the burst unless crates.io raised the account's limit.
#
# Usage, from anywhere in the repository:
#   CARGO_REGISTRY_TOKEN=… scripts/publish-crates.sh   publish
#   DRY_RUN=1 scripts/publish-crates.sh                ask crates.io, publish nothing
#   PLAN_ONLY=1 scripts/publish-crates.sh              print the order, no network
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
ua='shadcn-egui-release (https://github.com/oetiker/shadcn-egui)'

meta=$(cargo metadata --no-deps --format-version 1)
mapfile -t members < <(jq -r '.packages[] | select(.publish != []) | .name' <<< "$meta" | sort)
mapfile -t versions < <(jq -r '.packages[] | select(.publish != []) | .version' <<< "$meta" | sort -u)
if [ "${#versions[@]}" != 1 ]; then
  echo "publish-crates: the crates disagree on the version: ${versions[*]}" >&2
  exit 1
fi
version=${versions[0]}

order=(shadcn-egui-core)
for n in "${members[@]}"; do
  case "$n" in shadcn-egui-font-*|shadcn-egui-icons-*) order+=("$n") ;; esac
done
order+=(shadcn-egui)
if [ "$(printf '%s\n' "${order[@]}" | sort)" != "$(printf '%s\n' "${members[@]}")" ]; then
  echo "publish-crates: the publish order does not list every workspace crate exactly once:" >&2
  diff <(printf '%s\n' "${order[@]}" | sort) <(printf '%s\n' "${members[@]}") >&2 || true
  exit 1
fi

if [ "${PLAN_ONLY:-}" = 1 ]; then
  for n in "${order[@]}"; do echo "$n $version"; done
  exit 0
fi

mkdir -p target
log=target/publish-crates.log
for n in "${order[@]}"; do
  status=$(curl -sS -o /dev/null -w '%{http_code}' -A "$ua" "https://crates.io/api/v1/crates/$n/$version")
  case "$status" in
    200) echo "skip $n $version (already on crates.io)"; continue ;;
    404) ;;
    429) echo "publish-crates: crates.io rate limit (HTTP 429) while checking $n; re-run later" >&2; exit 2 ;;
    *) echo "publish-crates: crates.io answered HTTP $status for $n $version" >&2; exit 1 ;;
  esac
  if [ "${DRY_RUN:-}" = 1 ]; then
    echo "publish $n $version (dry run)"
    continue
  fi
  echo "publish $n $version"
  if ! cargo publish -p "$n" --locked --jobs "${JOBS:-4}" 2>&1 | tee "$log"; then
    if grep -q '429' "$log"; then
      echo "publish-crates: crates.io rate limit for new crates; re-run this script later, it skips what is already published" >&2
      exit 2
    fi
    exit 1
  fi
done
if [ "${DRY_RUN:-}" = 1 ]; then echo "dry run: nothing published"; else echo "all published"; fi
```

```bash
chmod +x scripts/publish-crates.sh
grep -n '{P0D_' scripts/publish-crates.sh || echo "all P0d values filled in"
bash -n scripts/publish-crates.sh && echo "syntax ok"
command -v shellcheck >/dev/null && shellcheck scripts/publish-crates.sh scripts/check-packages.sh
```

Expected: `all P0d values filled in`, `syntax ok`, and no shellcheck
findings (if shellcheck is installed).

- [ ] **Step 3: Check the order offline**

```bash
PLAN_ONLY=1 scripts/publish-crates.sh | tee "$SCRATCH/plan.txt"
head -1 "$SCRATCH/plan.txt"; tail -1 "$SCRATCH/plan.txt"
wc -l < "$SCRATCH/plan.txt"; cargo metadata --no-deps --format-version 1 | jq '.packages | length'
```

Expected: first line `shadcn-egui-core 0.4.0`, last line
`shadcn-egui 0.4.0`, every `shadcn-egui-font-*` and `shadcn-egui-icons-*`
in between, and the two counts equal.

- [ ] **Step 4: Check the refusals**

An unknown crate family must stop the script before it publishes:

```bash
mkdir -p crates/shadcn-egui-zz/src
printf '[package]\nname = "shadcn-egui-zz"\nversion.workspace = true\nedition.workspace = true\n' > crates/shadcn-egui-zz/Cargo.toml
: > crates/shadcn-egui-zz/src/lib.rs
PLAN_ONLY=1 scripts/publish-crates.sh; echo "exit $?"
git clean -n crates/shadcn-egui-zz
git status --short Cargo.lock
```

Expected: `publish-crates: the publish order does not list every workspace
crate exactly once:` with `> shadcn-egui-zz`, then `exit 1`;
`git status --short Cargo.lock` prints nothing (`cargo metadata --no-deps`
does not resolve, so the lock file is untouched). The
`git clean -n` lists the throwaway crate; ask the user before removing it
(`rm -r crates/shadcn-egui-zz`), then confirm `git status --short` shows
nothing from it.

- [ ] **Step 5: Ask crates.io without publishing**

Run: `DRY_RUN=1 scripts/publish-crates.sh`
Expected (before the first release): one `publish <name> 0.4.0 (dry run)`
line per crate, in the Step 3 order, then `dry run: nothing published` —
the names are free (P0d) and nothing is uploaded. If any line says `skip`, a name was
taken since P0d: **STOP** and tell the user.

The skip path, checked against a crate that is on crates.io (egui at the
workspace's egui version):

```bash
v=$(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "egui") | .version')
curl -sS -o /dev/null -w '%{http_code}\n' -A 'shadcn-egui-release (https://github.com/oetiker/shadcn-egui)' "https://crates.io/api/v1/crates/egui/$v"
```

Expected: `200` — the same request the script makes answers 200 for a
published version, which the script turns into `skip`.

- [ ] **Step 6: Commit**

```bash
git add scripts/publish-crates.sh
git commit -m "build: publish loop for the workspace crates

scripts/publish-crates.sh publishes core, then the font and icon crates,
then shadcn-egui, skipping every version crates.io already has. A run
stopped by the new-crate rate limit (HTTP 429) is finished by re-running
it. DRY_RUN=1 and PLAN_ONLY=1 show what it would do.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8.4: README, RELEASING.md, CLAUDE.md, CHANGES.md

**Files:**
- Modify: `README.md` (rewrite)
- Modify: `RELEASING.md` (rewrite)
- Modify: `CLAUDE.md`
- Modify: `CHANGES.md`
- Create: `crates/shadcn-egui/tests/readme.rs`

**Interfaces:**
- Consumes: `Preset::from_code`, `Preset::to_code`, `Mode`,
  `components::button::Button`; `crates/shadcn-egui/tests/fixtures/preset_codes.txt`
  (Phase 4: `code style base theme chart|- icons font heading radius accent menu canonical(0|1)`);
  `tools/icongen/SETS.tsv` (`ship`); the features in
  `crates/shadcn-egui/Cargo.toml`.
- Produces: `tests/readme.rs` keeps the README's preset code and feature
  list true.

- [ ] **Step 1: Write the failing README test**

`crates/shadcn-egui/tests/readme.rs`:

```rust
//! The README's example code and feature list stay true.

use shadcn_egui::Preset;

const README: &str = include_str!("../../../README.md");
const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn readme_preset_code_decodes_and_is_canonical() {
    let key = "Preset::from_code(\"";
    let start = README.find(key).expect("README shows Preset::from_code") + key.len();
    let code = &README[start..start + README[start..].find('"').unwrap()];
    let preset = Preset::from_code(code).unwrap_or_else(|e| panic!("README code {code}: {e}"));
    assert_eq!(preset.to_code().as_deref(), Some(code), "README shows a canonical code");
}

#[test]
fn readme_names_every_icon_feature_and_the_all_features() {
    let icon_features = MANIFEST
        .lines()
        .filter_map(|l| l.split_once(" = [").map(|(k, _)| k.trim()))
        .filter(|k| k.starts_with("icons-"));
    for f in icon_features.chain(["fonts-all"]) {
        assert!(README.contains(&format!("`{f}`")), "README does not mention `{f}`");
    }
}
```

Run: `cargo test -p shadcn-egui --jobs 4 --test readme -- --test-threads 4`
Expected: FAIL — the current README has no `Preset::from_code`.

- [ ] **Step 2: Rewrite `README.md`**

Pick a real preset code: the first canonical `b` code in the Phase 4
fixture (a code the shadcn site wrote).

```bash
CODE=$(awk '$2 != "null" && $NF == 1 && $1 ~ /^b/ && $1 != "b0" { print $1; exit }' crates/shadcn-egui/tests/fixtures/preset_codes.txt)
echo "$CODE"
```

Write `README.md` (replace `@PRESET_CODE@` by `$CODE` with
`sed -i "s/@PRESET_CODE@/$CODE/" README.md` after writing; keep the two
screenshot files under `docs/` as they are):

````markdown
# shadcn-egui

The [shadcn/ui](https://ui.shadcn.com/) design system for
[egui](https://github.com/emilk/egui): the shadcn v4 styles (vega, nova,
maia, lyra, mira, luma, rhea), its colour themes, fonts, icon sets and
presets, and components drawn as plain egui widgets. It works with any egui
backend (eframe, egui-winit, glow, wgpu).

![shadcn-egui settings screen](docs/settings-screen.png)

## Install

```bash
cargo add shadcn-egui
```

## Use a shadcn preset

Build a look on the shadcn create page (ui.shadcn.com or
shadcn-svelte.com), copy its preset code and apply it:

```rust
use shadcn_egui::{Mode, Preset};

fn setup(ctx: &egui::Context) -> Result<(), shadcn_egui::PresetCodeError> {
    Preset::from_code("@PRESET_CODE@")?.apply(ctx, Mode::Dark);
    Ok(())
}
```

`apply` again switches mode or preset; the fonts are reinstalled only when
they change. An app that installs fonts of its own uses
`Theme::apply_with_fonts` instead (see the `Theme` docs). Without a preset,
`Theme::vega_dark().apply(ctx)` gives the default look.

## Components

Button, Badge, Input, Label, Card, Tabs, Switch, Checkbox, Select, Slider,
Separator, Tooltip, Popover and Dialog:

```rust
use shadcn_egui::components::button::Button;

fn ui(ui: &mut egui::Ui) {
    if ui.add(Button::new("Save changes")).clicked() {
        // …
    }
}
```

`cargo run --example settings` shows all of them with a style, mode and
radius switcher.

![shadcn-egui component gallery](docs/components-screen.png)

## Features

Inter and the Lucide icons are always included. Everything else is a Cargo
feature named after the shadcn id:

| Feature | Adds |
|---|---|
| `font-<id>` | one font of the create page, e.g. `font-geist`, `font-jetbrains-mono`, `font-playfair-display` |
| `fonts-all` | every font |
| `icons-tabler` | Tabler icons |
| `icons-phosphor` | Phosphor icons |
| `icons-remixicon` | Remix Icon |
| `icons-hugeicons` | Hugeicons |
| `icons-all` | every icon set |

A preset that names a font or icon set whose feature is off renders with
Inter or Lucide and logs a warning through the `log` crate. The `sera`
style has no egui version yet and falls back to `nova`.

## Licences

The code is MIT (`LICENSE`). Each font crate also ships its font's licence
(SIL Open Font License 1.1 or Apache License 2.0); the icon crates ship
theirs: Lucide ISC, Tabler and Phosphor MIT, Remix Icon Apache-2.0 (the
last release under that licence; later Remix Icon releases are not
used), Hugeicons MIT. Each crate's `license` field names both.

## The Claude Code plugin

This repository also hosts the `egui-shadcn` Claude Code plugin, which
copied the 0.3 component code into projects. It stays at 0.3.2 until its
skill is rewritten for this crate.

## Development

See [CLAUDE.md](CLAUDE.md) for the layout, generators and checks, and
[RELEASING.md](RELEASING.md) for releases.
````

If SETS.tsv says `ship = no` for hugeicons, delete the `icons-hugeicons`
row and replace `Hugeicons MIT.` by `Hugeicons is not included: its
licence does not allow republishing the icons.` If any font was left out
for its licence (FONTS.tsv), add after the features table: `<Title> is
not available: its licence does not allow republishing.`

Run: `cargo test -p shadcn-egui --jobs 4 --test readme -- --test-threads 4`
Expected: 2 passed.

- [ ] **Step 3: Rewrite `RELEASING.md`**

Fill the `{P0D_…}` values from `docs/superpowers/notes/0.4/p0d-crates-io.md`
as in Task 8.3.

````markdown
# Releasing shadcn-egui

A release publishes every crate of the workspace to crates.io at one
version: `shadcn-egui-core`, the font crates `shadcn-egui-font-*`, the icon
crates `shadcn-egui-icons-*` and `shadcn-egui`. All crates share the
version in the root `Cargo.toml`.

## Cutting a release

1. Check that `main` is green.
2. In the repository's Actions tab, run **Create release PR** and choose
   bugfix, feature or major. Before 1.0, a breaking change is a feature
   release. The workflow rolls `## [Unreleased]` in `CHANGES.md` into the
   new version, writes the version into `Cargo.toml` (`[workspace.package]`
   and every `version = "…"` in `[workspace.dependencies]`) and
   `Cargo.lock`, runs CI and opens a pull request.
3. Merge that pull request. The publish workflow tags `vX.Y.Z`, publishes
   the crates and then makes the GitHub release public.

Do not push tags by hand, and do not add release headers to `CHANGES.md`:
the workflows do both.

## Publishing to crates.io

The publish job runs `scripts/publish-crates.sh` with the repository
secret `CARGO_REGISTRY_TOKEN` (a crates.io token with the scopes
`publish-new` and `publish-update`). The script publishes
`shadcn-egui-core`, then the font and icon crates, then `shadcn-egui`, and
skips every crate whose version is already on crates.io.

crates.io limits new crates per account: a burst of {P0D_NEW_BURST}, then
one per {P0D_NEW_REFILL}; new versions of existing crates: a burst of
{P0D_UPDATE_BURST}, then one per {P0D_UPDATE_REFILL} (read {P0D_DATE}). A
release that creates many crates, like 0.4.0 or one that adds a font or
icon set, stops with HTTP 429. Wait, then use **Re-run failed jobs** on the
publish run until its log ends with `all published`. Before such a release
you can ask crates.io (help@crates.io) to raise the account's new-crate
limit for the workspace.

To see what a publish would do:

```bash
PLAN_ONLY=1 scripts/publish-crates.sh   # the order, offline
DRY_RUN=1 scripts/publish-crates.sh     # skip/publish per crate, asks crates.io
```

## Checks before a release

The CI runs them; to run them locally:

```bash
scripts/check-packages.sh
scripts/check-package-size.sh
cargo publish --dry-run --workspace --jobs 4
```

## Adding a font or an icon set

A new font comes from `tools/fontgen` (row in `tools/fontgen/FONTS.tsv`,
then regenerate), a new icon set from `tools/icongen`. Its crate is new on
crates.io, so the next release creates a crate: see the rate limit above.

## The Claude Code plugin

`.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json` stay at
version `0.3.2` and are not part of the crate release. The marketplace
`oposs/claude-plugins` pins the plugin to the tag `v0.3.2` until the skill
is rewritten for the crate; do not bump the plugin versions until then.
````

```bash
grep -n '{P0D_' RELEASING.md || echo "all P0d values filled in"
```

- [ ] **Step 4: Rewrite `CLAUDE.md`**

````markdown
# CLAUDE.md

`shadcn-egui` is a Cargo workspace that ports the shadcn-v4 (OKLCH) design
system, its styles, palettes, fonts, icon sets and presets to egui as
crates published on crates.io. The repository also hosts the
`egui-shadcn` Claude Code plugin (`skills/egui-shadcn/`, `.claude-plugin/`),
frozen at 0.3.2 until the skill is rewritten for the crate.

## Layout

- `crates/shadcn-egui/` — theme, styles, palettes, presets, components,
  tests, example. The crate users depend on.
- `crates/shadcn-egui-core/` — dependency-free data types (`Icon`,
  `IconSlots`, `FontFiles`) shared with the font and icon crates.
- `crates/shadcn-egui-font-<id>/` — one crate per upstream font (generated
  by `tools/fontgen`); `inter` is always a dependency, the others are
  features `font-<id>`.
- `crates/shadcn-egui-icons-<lib>/` — one crate per icon set (generated by
  `tools/icongen`); `lucide` is always a dependency, the others are
  features `icons-<lib>`.
- `tools/stylegen`, `tools/icongen`, `tools/fontgen` — generators, each its
  own workspace; never dependencies of a crate.
- `scripts/` — `check-package-size.sh`, `check-packages.sh`,
  `publish-crates.sh`.

## Start here — read the source docs, don't restate them

- **Using and porting components, dependency discipline:**
  [`skills/egui-shadcn/SKILL.md`](skills/egui-shadcn/SKILL.md) (still
  describes the 0.3 vendoring flow; the porting standards apply to
  `crates/shadcn-egui/src/`).
- **Mapping tables:** `skills/egui-shadcn/references/`.
- **Cutting a release:** [`RELEASING.md`](RELEASING.md).
- **What changed per version:** [`CHANGES.md`](CHANGES.md).

## Project invariants

- Generated files are never edited by hand; fix the generator and
  regenerate:
  - stylegen: `crates/shadcn-egui/src/style/{generated,vega,nova,maia,lyra,mira,luma,rhea}.rs`,
    `crates/shadcn-egui/src/palette/generated.rs`,
    `crates/shadcn-egui/src/preset/generated.rs`
  - fontgen: `crates/shadcn-egui-font-*/` (all files),
    `crates/shadcn-egui/src/fonts/registry.rs`, the `# BEGIN fontgen` /
    `# END fontgen` blocks in `Cargo.toml` and `crates/shadcn-egui/Cargo.toml`
  - icongen: `crates/shadcn-egui-icons-*/src/{lib.rs,icons.bin}`,
    `crates/shadcn-egui/tests/icons/lucide.rs`
- All crates share one version; font and icon crates depend only on
  `shadcn-egui-core`.
- Keep runtime `[dependencies]` minimal — read the dependency-discipline
  section of `SKILL.md` before adding any.
- Any component change needs its kittest snapshot PNG regenerated, reviewed
  and committed.
- `.claude-plugin/*.json` stay at 0.3.2.
- The repository is public: no private project names or paths.

## Build & verify

- Cap parallelism: every cargo command gets `--jobs 4`, every `cargo test`
  also `-- --test-threads 4`.
- Gates (all must pass, warning-clean):

  ```bash
  cargo test --workspace --jobs 4 -- --test-threads 4
  cargo test --workspace --all-features --jobs 4 -- --test-threads 4
  cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
  cargo clippy --workspace --all-targets --all-features --jobs 4 -- -D warnings
  for t in stylegen icongen fontgen; do
    cargo test --manifest-path tools/$t/Cargo.toml --jobs 4 -- --test-threads 4
    cargo clippy --manifest-path tools/$t/Cargo.toml --all-targets --jobs 4 -- -D warnings
  done
  cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift <shadcn-svelte checkout> .
  scripts/check-packages.sh
  scripts/check-package-size.sh
  cargo publish --dry-run --workspace --jobs 4
  ```

- Generators run from the repository root: `stylegen generate|drift
  <shadcn-svelte checkout> .`; `tools/fontgen/fetch.sh <downloads-dir>`
  then `fontgen <downloads-dir> .`; icongen as described in
  `skills/egui-shadcn/references/icons.md`. Downloads go to a scratch
  directory, never into the repository.
- Snapshot tests render headlessly (egui_kittest + wgpu, mesa software
  adapter). Regenerate baselines with `UPDATE_SNAPSHOTS=1 cargo test`, then
  look at every changed PNG.
````

If Phase 6 documented the icongen commands elsewhere than
`references/icons.md`, point the icongen line at that file.

- [ ] **Step 5: CHANGES.md — one user-facing 0.4.0 entry**

1. The intro sentence becomes: `All notable changes to the `shadcn-egui`
   crates (and, up to 0.3.2, the egui-shadcn plugin) are recorded here.`
   followed by the existing Keep a Changelog / Semantic Versioning sentence.
2. The compare links at the bottom point at the renamed repository:

   ```bash
   sed -i -E '/^\[[0-9.]+\]: /s#github.com/oetiker/egui-shadcn#github.com/oetiker/shadcn-egui#' CHANGES.md
   ```

3. At the top of `## [Unreleased]` → `### Changed`, add:

   ```markdown
   - The crate is published on crates.io as `shadcn-egui`: add it with
     `cargo add shadcn-egui` and change `egui_shadcn::` to `shadcn_egui::` in
     your imports. Copying the component module into a project is no longer
     supported; the egui-shadcn Claude Code plugin stays at 0.3.2 and keeps
     doing that until it is rewritten for the crate.
   ```

4. Read all of `## [Unreleased]` once, top to bottom, and fix it in place:
   - Order: Changed starts with the rename, then presets (Phase 4), Inter
     and real font weights (Phase 2), icons (Phase 6), menu colours
     (Phase 5); Added starts with presets, fonts (Phase 7), icon sets.
   - Each entry leads with what a user sees or must change, has at most
     three sentences, and names no generator, test or internal module
     (`stylegen`, `fontgen`, `icongen`, `generated.rs`, kittest). An entry
     that needs more is two entries; two entries with one effect become one.
   - Every user-visible change below has its entry. Tick each off against
     the text; where one is missing, write it in the style of the entries
     around it (the phase in brackets wrote it, so a miss means that phase's
     entry was dropped or merged away):
     - [ ] crate renamed: `cargo add shadcn-egui`, imports `egui_shadcn::` →
       `shadcn_egui::`; module copying ends, plugin stays 0.3.2 (item 3 above);
     - [ ] the default font is Inter, so all text looks different from 0.3;
       Oxanium needs the `font-oxanium` feature (Phase 2);
     - [ ] medium/semibold text renders at its real weight (Phase 2);
     - [ ] apps with their own fonts call `Theme::apply_with_fonts` (Phase 2);
     - [ ] `Palette` gained `chart`: a `Palette { .. }` literal no longer
       compiles (Phase 3);
     - [ ] `Theme` gained `fonts`, `icons` and `menu`: a `Theme { .. }`
       literal no longer compiles; use `Theme::new(..)` and the builders
       (Phases 2 and 4 — merge them into one entry if both are there);
     - [ ] Checkbox, Select and Dialog draw their icons from `Theme::icons`
       (the theme's slots), so a preset's icon library changes them (Phases 4
       and 6);
     - [ ] `Button::icon` and `Button::icon_end` take any `&Icon`, not only a
       `&'static` const (Phase 4);
     - [ ] `components::icon::CHECK`, `CHEVRON_DOWN` and `CLOSE` are gone; use
       `shadcn_egui::icons::lucide::{CHECK, CHEVRON_DOWN, X}`; an `Icon`
       holds `shape` instead of `segs` (Phase 6);
     - [ ] presets: `Preset::from_code`, `apply`, `to_code`, fallbacks with a
       warning line, Lyra/Sera square corners (Phase 4);
     - [ ] menu colours on the Select list (Phase 5);
     - [ ] font features and `fonts-all`, fonts left out for their licence
       (Phase 7); icon features, `icons-all`, Hugeicons, Remix Icon pin
       (Phase 6).
   - No release header and no version number: the release workflow writes
     `## [0.4.0] - <date>`.

- [ ] **Step 6: Gates and commit**

Run every gate in the new CLAUDE.md list.

```bash
git add -A
git commit -m "docs: README, RELEASING and CLAUDE.md for the published crates

README shows cargo add, a preset, the features and the licences; a test
keeps its preset code and feature list true. RELEASING describes the
release pull request, the publish loop and the crates.io rate limit.
CHANGES reads as one 0.4.0 entry for users.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8.5: STOP (user) — repository rename, CI, token

**Files:** none until the user decides (repo-infra:apply makes its own
changes through its own pull request).

**Interfaces:**
- Consumes: Tasks 8.2–8.4.
- Produces: the GitHub repository `oetiker/shadcn-egui` with `origin`
  pointing at it; CI, changelog gate and the release workflows from
  repo-infra; the `CARGO_REGISTRY_TOKEN` secret.

Each step below is the user's or needs the user's explicit yes first.

- [ ] **Step 1: STOP — rename the GitHub repository**

Ask the user to rename `oetiker/egui-shadcn` to `oetiker/shadcn-egui`
(D2), in the GitHub settings or, with their go-ahead, with
`gh repo rename shadcn-egui -R oetiker/egui-shadcn`. Then update `origin`
in the main checkout, keeping its URL scheme:

```bash
git -C "$REPO" remote set-url origin "$(git -C "$REPO" remote get-url origin | sed 's#egui-shadcn#shadcn-egui#')"
git -C "$REPO" remote get-url origin
git -C "$REPO" ls-remote origin HEAD
```

Expected: the URL names `shadcn-egui`, and `ls-remote` answers. GitHub
redirects the old name, so the marketplace pin of Task 8.1 keeps working;
`.claude-plugin/*.json` keep their old URLs (D16).

- [ ] **Step 2: STOP — repo-infra:apply**

Ask the user whether to run `repo-infra:apply` now (it works on a branch
and through a pull request). Running it on top of `feat/shadcn-egui-0.4`
lets the first CI run see the workspace; if it insists on a branch from
`main`, do Task 8.6 Steps 1–2 first. Give it these facts:

- CI must run the gates of CLAUDE.md: both feature sets, clippy in both,
  the three tools, `scripts/check-packages.sh`,
  `scripts/check-package-size.sh` and `cargo publish --dry-run
  --workspace`. Snapshot tests need a Vulkan software driver (mesa
  lavapipe). `stylegen drift` needs the shadcn-svelte checkout at
  `tools/stylegen/PIN`.
- `ri-ci-rust` lints with `cargo fmt --check`. The workspace is not
  rustfmt-formatted today (`cargo fmt --check` reported about 950 diffs at
  commit 20ba879). Ask the user: format the workspace in one commit before
  CI arrives, or leave `fmt` out of the lint step.
- Version files: `Cargo.toml` (`[workspace.package] version` and every
  `version = "…"` of a `shadcn-egui*` entry in `[workspace.dependencies]`)
  and one `Cargo.lock` entry per workspace crate. **Not**
  `.claude-plugin/plugin.json` or `.claude-plugin/marketplace.json` (D16).
- Publish: the release workflow's crates.io job is a repository-owned job
  that checks out the release head and runs `scripts/publish-crates.sh`
  with `CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}`;
  `finalize` needs it. Confirm on crates.io's Trusted Publishing
  documentation that a trusted publisher cannot create a new crate; if it
  can, tell the user, because `ri-publish-crates-io` would then work for
  0.4.0 too.
- If repo-infra writes its own `RELEASING.md`, carry over the sections
  "Publishing to crates.io", "Checks before a release", "Adding a font or
  an icon set" and "The Claude Code plugin" from Task 8.4.

- [ ] **Step 3: STOP — crates.io token secret**

Ask the user to create a crates.io API token with the scopes
`publish-new` and `publish-update`, limited to the crate pattern
`shadcn-egui*`, and to store it as the repository secret
`CARGO_REGISTRY_TOKEN` (GitHub settings, or
`gh secret set CARGO_REGISTRY_TOKEN -R oetiker/shadcn-egui`, which reads
the token from the terminal). Never handle the token yourself.

- [ ] **Step 4: STOP — optional: raised new-crate limit**

Tell the user how many new crates 0.4.0 creates
(`PLAN_ONLY=1 scripts/publish-crates.sh | wc -l`) and the new-crate limit
from the P0d notes, and ask whether they want to ask crates.io
(help@crates.io) for a raised limit before the release. Without it, the
publish job stops with HTTP 429 after the burst and needs re-runs
(Task 8.6 Step 4). The user sends that request.

---

### Task 8.6: STOP (user) — merge, release 0.4.0, watch the publish

**Files:** none.

**Interfaces:**
- Consumes: Task 8.1 (pin confirmed), Task 8.5 (rename, CI, secret).
- Produces: `v0.4.0` tag, a public GitHub release and every crate at
  0.4.0 on crates.io.

- [ ] **Step 1: Confirm the pin**

Re-run Task 8.1 Step 1. Expected: `"ref": "v0.3.2"`. **If the pin is not
there, stop: `main` must not be pushed.**

- [ ] **Step 2: STOP — push and merge the feature branch**

Ask the user for the go-ahead to push `feat/shadcn-egui-0.4` and open a
pull request into `main`; the user merges it once CI is green. Nothing is
pushed without that yes.

- [ ] **Step 3: STOP — the release**

Ask the user to run **Create release PR** with `feature`. Check the pull
request it opens (read-only):

```bash
gh pr list -R oetiker/shadcn-egui --head release/v0.4.0
gh pr diff -R oetiker/shadcn-egui release/v0.4.0 --name-only
```

Expected: the version is 0.4.0; the diff touches `CHANGES.md` (the
`## [0.4.0] - <date>` header), `Cargo.toml` / `Cargo.lock` at most for the
version, and nothing under `.claude-plugin/`. The user merges it; the
publish workflow tags `v0.4.0`.

- [ ] **Step 4: Watch the publish**

```bash
gh run list -R oetiker/shadcn-egui --workflow release-publish.yml --limit 1
gh run watch -R oetiker/shadcn-egui <run id>
```

If the crates.io job stops with `publish-crates: crates.io rate limit …`
(exit 2), tell the user when the limit refills (P0d numbers) and ask them
to use **Re-run failed jobs** on that run then; repeat until its log ends
with `all published`. The GitHub release stays a draft until then.

- [ ] **Step 5: Verify**

```bash
UA='shadcn-egui-release-check (https://github.com/oetiker/shadcn-egui)'
PLAN_ONLY=1 scripts/publish-crates.sh | while read -r n v; do
  printf '%s %s\n' "$n" "$(curl -s -o /dev/null -w '%{http_code}' -A "$UA" "https://crates.io/api/v1/crates/$n/$v")"
  sleep 1
done
gh release view v0.4.0 -R oetiker/shadcn-egui --json isDraft -q .isDraft
```

Expected: `200` for every crate; `false` (the release is public). Then a
smoke test from crates.io in a scratch project:

```bash
cargo new --lib "$SCRATCH/smoke-0.4" && cd "$SCRATCH/smoke-0.4"
cargo add shadcn-egui@0.4.0 --features fonts-all,icons-all
cargo build --jobs 4
```

Expected: it builds. Report the result to the user.
