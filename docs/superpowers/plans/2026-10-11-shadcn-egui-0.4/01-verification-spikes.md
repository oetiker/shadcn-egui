# Phase 0 — Verification spikes

> Part of [00-index.md](00-index.md). Read its Global Constraints first.

**Spec:** D12, D13, D14, §6.1 (licences), §6.3 (Remixicon, Hugeicons), §9 (rate limit).

Each spike answers one question the later phases build on and commits a notes
file under `docs/superpowers/notes/0.4/`. A spike whose answer contradicts the
spec **stops the plan**: write the finding into the notes file, commit it, and
ask the user; do not adapt later phases on your own.

This phase runs in the current single-crate layout (repository root is the
crate). Work in `$WT` (see Setup).

---

### Task 0.1 (spike a): React vs svelte `preset.ts`, and fixture codes

**Files:**
- Create: `docs/superpowers/notes/0.4/p0a-preset-codes.md`
- Create: `tools/stylegen/upstream/preset-codes.txt`

**Interfaces:**
- Produces: `preset-codes.txt`, one code per line (`#` comments allowed), each
  followed by ` # <origin>`; Phase 4 Task 2 feeds it to the fixture script.
  The notes file states (1) whether every list in `PRESET_FIELDS_V2` has the
  same order in both repos, (2) whether either site decodes `a` codes with
  `V1_CHART_COLOR_MAP`, (3) the commit of each checkout.

- [ ] **Step 1: Clone shadcn/ui (React) without blobs**

```bash
git clone --filter=blob:none https://github.com/shadcn-ui/ui.git "$SCRATCH/shadcn-ui"
git -C "$SCRATCH/shadcn-ui" log -1 --format='%H %cs'
git -C "$SCRATCH/shadcn-ui" ls-files | grep -E '(^|/)preset\.ts$'
```

Expected: one or two paths such as `packages/shadcn/src/preset/preset.ts`.
Call the path `$REACT_PRESET`.

- [ ] **Step 2: Compare the value lists and the field table**

```bash
S="$UP/packages/cli/src/preset/preset.ts"
R="$SCRATCH/shadcn-ui/$REACT_PRESET"
for f in "$S" "$R"; do
  sed -n '/^export const PRESET_STYLES/,/^export const DEFAULT_PRESET_CONFIG/p' "$f" > "$SCRATCH/$(basename $(dirname $(dirname $(dirname "$f"))))-lists.ts"
done
diff "$SCRATCH"/cli-lists.ts "$SCRATCH"/shadcn-lists.ts || true
grep -n 'iconLibraries\s*=' -A2 "$UP/packages/cli/src/icons/libraries.ts"
git -C "$SCRATCH/shadcn-ui" ls-files | grep -E 'icons/libraries\.ts$'
```

Then open the React `libraries.ts` and list its top-level keys in order.
Expected outcome: every array (`PRESET_STYLES`, `PRESET_BASE_COLORS` keys,
`PRESET_THEMES` keys, `PRESET_FONTS`, `PRESET_FONT_HEADINGS`, `PRESET_RADII`
keys, `PRESET_MENU_ACCENTS`, `PRESET_MENU_COLORS`, icon library keys) and
both `PRESET_FIELDS_V1`/`V2` tables (key, bits) are identical in content and
order. Differences in comments or formatting do not count.

- [ ] **Step 3: Find every use of `V1_CHART_COLOR_MAP`**

```bash
git -C "$UP" grep -n V1_CHART_COLOR_MAP -- packages docs/src
git -C "$SCRATCH/shadcn-ui" grep -n V1_CHART_COLOR_MAP -- apps/v4 packages/shadcn
```

Read each hit. Expected outcome: the map is exported but neither decoder
applies it, so an `a` code decodes `chartColor` to its default (index 0,
`neutral`). Record which file would apply it if one does.

- [ ] **Step 4: Collect codes the sites produce**

```bash
git -C "$SCRATCH/shadcn-ui" grep -nhoE '(preset[=: "'\'']+|--preset )[ab][0-9A-Za-z]{1,10}' -- apps/v4 packages/shadcn | sort -u
git -C "$UP" grep -nhoE '(preset[=: "'\'']+|--preset )[ab][0-9A-Za-z]{1,10}' -- docs packages | sort -u
```

Write `tools/stylegen/upstream/preset-codes.txt`:

```text
# Preset codes for the decoder fixtures (Phase 4, Task 2).
# One code per line, then " # <origin>". Codes found in the upstream repos:
<each code found above> # shadcn-ui <path>:<line>   (or: # shadcn-svelte <path>:<line>)
# Boundary and hand-made codes (decode only; the fixture marks round-trip):
a0 # all indices 0, version a
b0 # all indices 0, version b
bZZZZZZZZ # out-of-range indices decode to index 0
aZZZZZZZ # out-of-range indices, version a
```

The fixture script adds encodes of the sites' default presets and 64 seeded
random configs itself (Phase 4, Task 2), so a repo without literal codes is
not a failure.

- [ ] **Step 5: Write the notes file and decide**

`docs/superpowers/notes/0.4/p0a-preset-codes.md`:

```markdown
# P0a: preset codes, React vs svelte

- shadcn-svelte: 493481fa (PIN). shadcn/ui: <hash> (<date>), preset.ts at <path>.
- Lists and PRESET_FIELDS_V1/V2: identical | differ: <list each difference>.
- Icon library key order: lucide, tabler, hugeicons, phosphor, remixicon (svelte) | <react order>.
- V1_CHART_COLOR_MAP: exported, applied by <none | file:line>.
- Decision for `a` codes: chartColor = index 0 (neutral) | chartColor = V1_CHART_COLOR_MAP[baseColor].
- Codes collected: <n> (see tools/stylegen/upstream/preset-codes.txt).
```

Decision rule:
- Lists identical and map unused → Phase 4 as written.
- Map applied by a decoder on either site → Phase 4 Task 2 takes the
  "V1 chart map" branch written there.
- Any list or bit width differs → **STOP**: D13 says the difference goes back
  into the spec. Commit the notes and ask the user.

- [ ] **Step 6: Commit**

```bash
git add docs/superpowers/notes/0.4/p0a-preset-codes.md tools/stylegen/upstream/preset-codes.txt
git commit -m "docs: P0a preset code spike (React vs svelte preset.ts)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 0.2 (spike b): variable-font ink proof

**Files:**
- Create: `tests/variable_font_spike.rs`
- Create: `docs/superpowers/notes/0.4/p0b-variable-font.md`

**Interfaces:**
- Produces: the ink ratios `ink(700)/ink(400)` and `ink(500)/ink(400)` for
  Oxanium's variable font via `FontTweak::coords`. Phase 2 relies on
  `egui::epaint::text::VariationCoords::new([(b"wght", w)])` working.

- [ ] **Step 1: Write the proof test**

`tests/variable_font_spike.rs`:

```rust
//! Spike (Phase 0): does `FontTweak::coords` change the weight egui renders?
//! One word at wght 700 must have clearly more ink than at 400.

use egui::epaint::text::VariationCoords;
use egui::{vec2, Color32, FontData, FontDefinitions, FontFamily, FontTweak, RichText};
use egui_kittest::Harness;
use std::sync::Arc;

const OXANIUM: &[u8] = include_bytes!("../assets/Oxanium-Variable.ttf");

fn defs(wght: f32) -> FontDefinitions {
    let mut d = FontDefinitions::default();
    let tweak = FontTweak { coords: VariationCoords::new([(b"wght", wght)]), ..Default::default() };
    d.font_data.insert("probe".into(), Arc::new(FontData::from_static(OXANIUM).tweak(tweak)));
    d.families.insert(FontFamily::Name("probe".into()), vec!["probe".into()]);
    d
}

/// Sum of darkness (255 - green) over the rendered image.
fn ink(wght: f32) -> u64 {
    let fam = FontFamily::Name("probe".into());
    let mut installed = false;
    let mut h = Harness::builder().with_size(vec2(560.0, 90.0)).build_ui(move |ui| {
        if !installed {
            ui.ctx().set_fonts(defs(wght));
            installed = true;
        }
        ui.painter().rect_filled(ui.ctx().viewport_rect(), 0.0, Color32::WHITE);
        if ui.ctx().fonts(|f| f.definitions().families.contains_key(&fam)) {
            ui.label(RichText::new("Hamburgefonstiv").family(fam.clone()).size(40.0).color(Color32::BLACK));
        }
    });
    h.run();
    let img = h.render().expect("wgpu render");
    img.pixels().map(|p| u64::from(255 - p.0[1])).sum()
}

#[test]
fn wght_700_has_more_ink_than_400() {
    let (regular, medium, bold) = (ink(400.0), ink(500.0), ink(700.0));
    println!("ink 400={regular} 500={medium} 700={bold} ratio700={:.3} ratio500={:.3}", bold as f64 / regular as f64, medium as f64 / regular as f64);
    assert!(regular > 0, "nothing rendered");
    assert!(bold as f64 > regular as f64 * 1.15, "wght 700 is not bolder: {bold} vs {regular}");
    assert!(medium > regular, "wght 500 is not bolder than 400");
}
```

- [ ] **Step 2: Run it**

Run: `cargo test --jobs 4 --test variable_font_spike -- --nocapture --test-threads 4`
Expected: PASS, and a line `ink 400=… 500=… 700=… ratio700=1.3… ratio500=1.0…`.
If it fails because the word is equally dark at all weights: **STOP**. D12
(no static instancing) rests on this; record the numbers and ask the user.

- [ ] **Step 3: Notes and commit**

`docs/superpowers/notes/0.4/p0b-variable-font.md` records the printed line,
the egui/epaint version (`0.34.3`), and the conclusion ("`FontTweak::coords`
sets wght per registered font; D12 holds").

```bash
git add tests/variable_font_spike.rs docs/superpowers/notes/0.4/p0b-variable-font.md
git commit -m "test: P0b spike proves FontTweak::coords renders real weights

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 0.3 (spike c): licences of the 26 fonts and the icon sets; Remixicon pin

**Files:**
- Create: `tools/fontgen/GF_PIN`
- Create: `tools/fontgen/FONTS.tsv`
- Create: `tools/icongen/SETS.tsv`
- Create: `docs/superpowers/notes/0.4/p0c-licences.md`

**Interfaces:**
- Produces `tools/fontgen/GF_PIN`: one line, the google/fonts commit hash.
- Produces `tools/fontgen/FONTS.tsv` (tab-separated, header line first,
  rows in `PRESET_FONTS` order), read by Phase 2 Task 2 and by `fontgen`
  (Phase 7):

  | column | meaning |
  |---|---|
  | `name` | upstream id (`inter`) |
  | `title` | display name (`Inter`) |
  | `kind` | `sans` \| `serif` \| `mono` (font-definitions.ts `type`) |
  | `license` | `OFL-1.1` \| `Apache-2.0` \| other SPDX \| `unknown` |
  | `files` | google/fonts path of the variable TTF, or `400=<path>;700=<path>` for static-only fonts |
  | `license_file` | google/fonts path of the licence text |
  | `y_offset_factor` | `-` until measured (Phase 2 Task 5, Phase 7 Task 2); `0.18` for oxanium |

- Produces `tools/icongen/SETS.tsv` (tab-separated, header first), read by
  Phase 6:

  | column | meaning |
  |---|---|
  | `lib` | `lucide` \| `tabler` \| `phosphor` \| `remixicon` \| `hugeicons` |
  | `package` | npm package (`lucide-static`, `@tabler/icons`, `@phosphor-icons/core`, `remixicon`, `@hugeicons/core-free-icons`) |
  | `version` | exact version to ship |
  | `svg_glob` | path glob inside the package (`package/icons/*.svg`, `package/icons/outline/*.svg`, `package/assets/regular/*.svg`, `package/icons/*/*.svg`, `-` for hugeicons: JS data) |
  | `license` | SPDX as read from the licence file in the tarball |
  | `ship` | `yes` \| `no` (no = licence forbids republishing) |

- [ ] **Step 1: google/fonts tree without blobs**

```bash
git clone --filter=blob:none --no-checkout --depth 1 https://github.com/google/fonts.git "$SCRATCH/gfonts"
git -C "$SCRATCH/gfonts" rev-parse HEAD > tools/fontgen/GF_PIN
git -C "$SCRATCH/gfonts" ls-tree -r --name-only HEAD > "$SCRATCH/gfonts-files.txt"
```

- [ ] **Step 2: Locate each font and its licence**

For each name in `PRESET_FONTS` (`grep -A30 'PRESET_FONTS = \[' "$UP_REACT/packages/registry/src/preset/preset.ts"`; index §J),
find its directory: the id without dashes, under `ofl/`, `apache/` or `ufl/`
(`jetbrains-mono` → `ofl/jetbrainsmono/`, `source-sans-3` → `ofl/sourcesans3/`):

```bash
for n in inter noto-sans nunito-sans figtree roboto raleway dm-sans public-sans outfit jetbrains-mono geist geist-mono lora merriweather playfair-display noto-serif roboto-slab oxanium manrope space-grotesk montserrat ibm-plex-sans source-sans-3 instrument-sans eb-garamond instrument-serif; do
  d=$(echo "$n" | tr -d -)
  echo "== $n"; grep -E "^(ofl|apache|ufl)/$d/" "$SCRATCH/gfonts-files.txt" | grep -vE '/(article|upstream|static)/' | grep -E '\.ttf$|LICENSE|OFL' | grep -v Italic
done
```

The licence follows from the top directory (`ofl/` = OFL-1.1, `apache/` =
Apache-2.0, `ufl/` = Ubuntu Font Licence, not allowed). Confirm three of them
by reading the licence blob, e.g.
`git -C "$SCRATCH/gfonts" show HEAD:ofl/inter/OFL.txt | head -5`.
Variable files carry `[` in the name (`Inter[opsz,wght].ttf`); a font with
only static files (expected: Instrument Serif, `InstrumentSerif-Regular.ttf`)
gets a `400=…` entry. Italic files are not shipped.

- [ ] **Step 3: Write `tools/fontgen/FONTS.tsv`**

```text
name	title	kind	license	files	license_file	y_offset_factor
inter	Inter	sans	OFL-1.1	ofl/inter/Inter[opsz,wght].ttf	ofl/inter/OFL.txt	-
…one row per font, PRESET_FONTS order…
oxanium	Oxanium	sans	OFL-1.1	ofl/oxanium/Oxanium[wght].ttf	ofl/oxanium/OFL.txt	0.18
instrument-serif	Instrument Serif	serif	OFL-1.1	400=ofl/instrumentserif/InstrumentSerif-Regular.ttf	ofl/instrumentserif/OFL.txt	-
```

`title` and `kind` come from `$UP_REACT/apps/v4/lib/font-definitions.ts` (same data as the svelte copy; index §J).

- [ ] **Step 4: Icon set licences, read from each tarball**

```bash
pkg() { # pkg <npm name> <version>  -> extracts into $SCRATCH/npm/<name>-<version>/
  local dir="$SCRATCH/npm/$(echo "$1" | tr / _)-$2"; mkdir -p "$dir"
  local tgz; tgz=$(curl -sf "https://registry.npmjs.org/$1/$2" | jq -r .dist.tarball)
  curl -sfL "$tgz" | tar -xz -C "$dir"; echo "$dir"
}
latest() { curl -sf "https://registry.npmjs.org/$1/latest" | jq -r .version; }
for p in lucide-static @tabler/icons @phosphor-icons/core @hugeicons/core-free-icons; do
  v=$(latest "$p"); d=$(pkg "$p" "$v"); echo "== $p $v"; ls "$d/package"; head -20 "$d"/package/LICENSE* 2>/dev/null
done
```

Hugeicons: read the licence file shipped in the package, not the `license`
field of `package.json`. Ship only if it allows redistributing the icon data
in another package (MIT/ISC/Apache-2.0 or equivalent). Also record the data
format: list `ls "$d/package/dist"` and print the first 40 lines of the file
that defines `Tick02Icon` (`grep -rl "Tick02Icon" "$d/package/dist" | head -3`).
Phase 6 expects each icon as an array of `[tag, attrs]` pairs.

- [ ] **Step 5: Find the last Apache-2.0 Remixicon release**

```bash
curl -sf https://registry.npmjs.org/remixicon | jq -r '.time | to_entries[] | select(.key|test("^[0-9]")) | "\(.value) \(.key)"' | sort | tail -15
```

Starting from the newest version and walking back, extract each tarball
(`pkg remixicon <v>`) and read its `License` file (`ls "$d/package"`), until
the licence text is the Apache License 2.0. That version is the pin; record
the first version under "Remix Icon License v1.0" as well.

- [ ] **Step 6: Write `tools/icongen/SETS.tsv`**

```text
lib	package	version	svg_glob	license	ship
lucide	lucide-static	<v>	package/icons/*.svg	ISC	yes
tabler	@tabler/icons	<v>	package/icons/outline/*.svg	MIT	yes
phosphor	@phosphor-icons/core	<v>	package/assets/regular/*.svg	MIT	yes
remixicon	remixicon	<last Apache-2.0 version>	package/icons/*/*.svg	Apache-2.0	yes
hugeicons	@hugeicons/core-free-icons	<v>	-	<SPDX or "proprietary">	yes|no
```

- [ ] **Step 7: Notes, decision, commit**

`docs/superpowers/notes/0.4/p0c-licences.md`: a table of all 26 fonts
(name, licence, file, variable axes from the file name, shipped yes/no), the
icon table above with how each licence was confirmed, the Remixicon pin and
the first non-Apache version, the Hugeicons verdict with the licence text's
key sentence, and the Hugeicons data format.

Decision rule:
- A font that is not OFL-1.1/Apache-2.0 → `license` column holds its SPDX;
  fontgen (Phase 7) skips it and it falls back to Inter. No stop.
- Hugeicons not redistributable → `ship = no`; Phase 6 Task 6 takes the
  "no Hugeicons crate" branch. No stop (the spec foresees it).
- No Apache-2.0 Remixicon release exists → **STOP** and ask the user.

```bash
git add tools/fontgen/GF_PIN tools/fontgen/FONTS.tsv tools/icongen/SETS.tsv docs/superpowers/notes/0.4/p0c-licences.md
git commit -m "docs: P0c licence spike for fonts and icon sets

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 0.4 (spike d): crates.io new-crate rate limit and crate names

**Files:**
- Create: `docs/superpowers/notes/0.4/p0d-crates-io.md`

**Interfaces:**
- Produces: burst size and refill interval for new crates, the limit for new
  versions of existing crates, and the availability of every crate name the
  workspace will publish. Phase 8 cites them in RELEASING.md and the publish
  script's message.

- [ ] **Step 1: Read the current limits**

Read crates.io's documentation and source (generic User-Agent):
`https://crates.io/policies`, and in `https://github.com/rust-lang/crates.io`
the rate-limiter defaults (search the source for `PublishNew` and
`PublishUpdate`, e.g. `src/rate_limiter.rs`). Record burst and refill for
`PublishNew` and `PublishUpdate`, and the date read.

- [ ] **Step 2: Check the names**

```bash
UA='shadcn-egui-release-check (https://github.com/oetiker/shadcn-egui)'
for n in shadcn-egui shadcn-egui-core shadcn-egui-icons-lucide shadcn-egui-icons-tabler shadcn-egui-icons-phosphor shadcn-egui-icons-remixicon shadcn-egui-icons-hugeicons $(awk -F'\t' 'NR>1{print "shadcn-egui-font-"$1}' tools/fontgen/FONTS.tsv); do
  printf '%s %s\n' "$n" "$(curl -s -o /dev/null -w '%{http_code}' -A "$UA" "https://crates.io/api/v1/crates/$n")"
  sleep 1
done
```

Expected: `404` (free) for every name. A `200` for any name → **STOP** and
ask the user (D2 assumed the names are free).

- [ ] **Step 3: Notes and commit**

`p0d-crates-io.md`: the limits with source links, the expected number of new
crates (core + `shadcn-egui` + Lucide + icon crates shipped + font crates
shipped), the resulting minimum publish time, and the name table.

```bash
git add docs/superpowers/notes/0.4/p0d-crates-io.md
git commit -m "docs: P0d crates.io rate limit and crate name spike

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Phase 0 gate

Run the old single-crate gates (`cargo test --jobs 4 -- --test-threads 4`,
`cargo clippy --all-targets --jobs 4 -- -D warnings`). All green, four commits.
