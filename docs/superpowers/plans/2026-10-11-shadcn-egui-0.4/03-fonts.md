# Phase 2 — Fonts: real weights, Inter, heading font

> Part of [00-index.md](00-index.md). Read its Global Constraints first.

**Spec:** D8, D12, D17, §6.1, §7 (fonts proof test, heading font snapshot, existing snapshots change).
**Cites:** `docs/superpowers/notes/0.4/p0b-variable-font.md` (coords work),
`tools/fontgen/FONTS.tsv` + `tools/fontgen/GF_PIN` (Inter source path).

**Outcome:** `FontFiles` in core; crates `shadcn-egui-font-inter` (normal
dependency, the default) and `shadcn-egui-font-oxanium` (optional,
feature `font-oxanium`); `Theme.fonts` with body and heading font;
`Theme::apply` is the only installer, registers each font once per weight
400/500/600/700 with `FontTweak::coords`, keeps egui's fonts as glyph
fallback in every family, reinstalls only when the fonts change, and has an
`apply_with_fonts` twin for apps that manage fonts. Card and Dialog titles
use the heading font.

Tasks 2.3–2.5 change rendering; the snapshot tests are red between the
Task 2.3 commit and Task 2.6, where all baselines are regenerated once and
reviewed by the user. Every non-snapshot test stays green throughout.

---

### Task 2.1: `FontFiles` in core

**Files:**
- Create: `crates/shadcn-egui-core/src/font.rs`
- Modify: `crates/shadcn-egui-core/src/lib.rs`

**Interfaces:**
- Produces (all `pub` in `shadcn_egui_core`, re-exported by `shadcn_egui`):
  - `enum FontKind { Sans, Serif, Mono }`
  - `const WEIGHTS: [u16; 4] = [400, 500, 600, 700]`
  - `enum FontSource { Variable { data: &'static [u8], wght: RangeInclusive<f32> }, Static { regular: &'static [u8], medium: Option<&'static [u8]>, semibold: Option<&'static [u8]>, bold: Option<&'static [u8]> } }` (`Clone`)
  - `struct FontFiles { name: &'static str, kind: FontKind, source: FontSource, y_offset_factor: f32, license: &'static str }` — `Clone`; `Debug` prints only `name`; `PartialEq` compares `name` (upstream ids are unique; comparing megabytes of font data on every `Theme` comparison would be wasteful).
  - `struct Face { data: &'static [u8], wght: Option<f32> }` (`Clone, Copy, Debug, PartialEq`)
  - `FontFiles::face(&self, weight: u16) -> Face` — variable: the font's data with `wght` clamped into its range; static: the nearest existing file, on a tie the lighter one for weights ≤ 500 and the heavier one above (CSS font matching), `wght: None`.

- [ ] **Step 1: Write the failing tests**

`crates/shadcn-egui-core/src/font.rs` (tests at the bottom of the new file;
write the whole file in Step 3, the tests first):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    static VAR: FontFiles = FontFiles {
        name: "var",
        kind: FontKind::Sans,
        source: FontSource::Variable { data: b"var", wght: 200.0..=650.0 },
        y_offset_factor: 0.0,
        license: "",
    };
    static ONLY_REGULAR: FontFiles = FontFiles {
        name: "only-regular",
        kind: FontKind::Serif,
        source: FontSource::Static { regular: b"r", medium: None, semibold: None, bold: None },
        y_offset_factor: 0.0,
        license: "",
    };
    static REGULAR_AND_BOLD: FontFiles = FontFiles {
        name: "regular-bold",
        kind: FontKind::Sans,
        source: FontSource::Static { regular: b"r", medium: None, semibold: None, bold: Some(b"b") },
        y_offset_factor: 0.0,
        license: "",
    };

    #[test]
    fn variable_weights_are_clamped_into_the_axis() {
        assert_eq!(VAR.face(400), Face { data: b"var", wght: Some(400.0) });
        assert_eq!(VAR.face(700).wght, Some(650.0), "above the axis clamps to its max");
        assert_eq!(VAR.face(100).wght, Some(200.0), "below the axis clamps to its min");
    }

    #[test]
    fn static_fonts_use_the_nearest_weight() {
        for w in WEIGHTS {
            assert_eq!(ONLY_REGULAR.face(w), Face { data: b"r", wght: None }, "{w}");
        }
        assert_eq!(REGULAR_AND_BOLD.face(500).data, b"r", "500: tie-free, 400 is nearer");
        assert_eq!(REGULAR_AND_BOLD.face(600).data, b"b", "600: 700 is nearer");
        assert_eq!(REGULAR_AND_BOLD.face(700).data, b"b");
    }

    #[test]
    fn font_files_compare_and_print_by_name() {
        assert_eq!(VAR, VAR.clone());
        assert_ne!(VAR, ONLY_REGULAR);
        assert_eq!(format!("{VAR:?}"), "FontFiles(\"var\")");
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p shadcn-egui-core --jobs 4 -- --test-threads 4`
Expected: FAIL to compile (`FontFiles` not found) — only after Step 3's
module line exists; write `mod font;` into `lib.rs` first so the test module
is compiled.

- [ ] **Step 3: Implement**

`crates/shadcn-egui-core/src/font.rs` (above the tests):

```rust
//! Font data a font crate exports and `shadcn_egui::Theme` installs.

use std::ops::RangeInclusive;

/// The font's upstream `type`; a mono body font also becomes egui's
/// `Monospace`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontKind {
    Sans,
    Serif,
    Mono,
}

/// The CSS weights a theme installs: regular, medium, semibold, bold.
pub const WEIGHTS: [u16; 4] = [400, 500, 600, 700];

/// The font files of one family.
#[derive(Clone)]
pub enum FontSource {
    /// One variable font; `wght` is its weight axis range.
    Variable { data: &'static [u8], wght: RangeInclusive<f32> },
    /// Static files, only for fonts without a variable version. A missing
    /// weight uses the nearest one that exists.
    Static {
        regular: &'static [u8],
        medium: Option<&'static [u8]>,
        semibold: Option<&'static [u8]>,
        bold: Option<&'static [u8]>,
    },
}

/// One font family as a font crate ships it.
#[derive(Clone)]
pub struct FontFiles {
    /// The upstream font id (`"inter"`, `"jetbrains-mono"`).
    pub name: &'static str,
    pub kind: FontKind,
    pub source: FontSource,
    /// egui `FontTweak::y_offset_factor` that centres capitals in a button,
    /// measured per font.
    pub y_offset_factor: f32,
    /// The font's licence text (OFL-1.1 or Apache-2.0).
    pub license: &'static str,
}

/// The bytes and variation coordinate to register for one CSS weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    pub data: &'static [u8],
    /// `Some` for a variable font: the `wght` coordinate.
    pub wght: Option<f32>,
}

impl FontFiles {
    /// The face for a CSS weight (see [`WEIGHTS`]).
    pub fn face(&self, weight: u16) -> Face {
        match &self.source {
            FontSource::Variable { data, wght } => {
                Face { data: *data, wght: Some(f32::from(weight).clamp(*wght.start(), *wght.end())) }
            }
            FontSource::Static { regular, medium, semibold, bold } => {
                let have = [(400u16, Some(*regular)), (500, *medium), (600, *semibold), (700, *bold)];
                // CSS font matching: on a tie, lighter for <= 500, heavier above.
                let tie = |w: u16| if weight <= 500 { w } else { u16::MAX - w };
                let data = have
                    .iter()
                    .filter_map(|&(w, d)| d.map(|d| (w, d)))
                    .min_by_key(|&(w, _)| (w.abs_diff(weight), tie(w)))
                    .map(|(_, d)| d)
                    .unwrap_or(*regular);
                Face { data, wght: None }
            }
        }
    }
}

impl std::fmt::Debug for FontFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FontFiles").field(&self.name).finish()
    }
}

impl PartialEq for FontFiles {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
```

`crates/shadcn-egui-core/src/lib.rs`:

```rust
//! Data types shared by `shadcn-egui` and its font and icon crates.
//!
//! Users never name this crate: `shadcn-egui` re-exports everything in it.

mod font;
mod icon;

pub use font::{Face, FontFiles, FontKind, FontSource, WEIGHTS};
pub use icon::{Icon, Seg};
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p shadcn-egui-core --jobs 4 -- --test-threads 4`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/shadcn-egui-core
git commit -m "feat(core): FontFiles describes a font crate's variable or static files

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2.2: Font crates `shadcn-egui-font-inter` and `shadcn-egui-font-oxanium`

**Files:**
- Create: `crates/shadcn-egui-font-inter/{Cargo.toml,src/lib.rs,fonts/inter.ttf,LICENSE-FONT.txt}`
- Create: `crates/shadcn-egui-font-oxanium/{Cargo.toml,src/lib.rs,fonts/oxanium.ttf,LICENSE-FONT.txt}`
- Move: `crates/shadcn-egui/assets/Oxanium-Variable.ttf` → `crates/shadcn-egui-font-oxanium/fonts/oxanium.ttf`; `crates/shadcn-egui/assets/Oxanium-OFL.txt` → `crates/shadcn-egui-font-oxanium/LICENSE-FONT.txt`
- Delete: `crates/shadcn-egui/assets/Oxanium-{Regular,Medium,SemiBold}.ttf` (tracked; recoverable from history)
- Modify: root `Cargo.toml` (`[workspace.dependencies]`)

**Interfaces:**
- Produces: `shadcn_egui_font_inter::FONT: FontFiles` and
  `shadcn_egui_font_oxanium::FONT: FontFiles` (both `pub static`). Phase 7's
  fontgen must reproduce these two crates byte for byte; keep the layout
  (`fonts/<id>.ttf`, `LICENSE-FONT.txt`, the lib.rs text below).

- [ ] **Step 1: Fetch Inter at the pinned google/fonts commit**

```bash
GF=$(cat tools/fontgen/GF_PIN)
P=$(awk -F'\t' '$1=="inter"{print $5}' tools/fontgen/FONTS.tsv)
L=$(awk -F'\t' '$1=="inter"{print $6}' tools/fontgen/FONTS.tsv)
enc() { printf %s "$1" | sed 's/\[/%5B/g; s/\]/%5D/g'; }
mkdir -p crates/shadcn-egui-font-inter/fonts crates/shadcn-egui-font-inter/src
curl -sfL -o crates/shadcn-egui-font-inter/fonts/inter.ttf "https://raw.githubusercontent.com/google/fonts/$GF/$(enc "$P")"
curl -sfL -o crates/shadcn-egui-font-inter/LICENSE-FONT.txt "https://raw.githubusercontent.com/google/fonts/$GF/$(enc "$L")"
head -c 4 crates/shadcn-egui-font-inter/fonts/inter.ttf | od -An -tx1   # 00 01 00 00 (TrueType)
head -3 crates/shadcn-egui-font-inter/LICENSE-FONT.txt                  # Copyright … / SIL Open Font License
```

- [ ] **Step 2: Move Oxanium, drop its static copies**

```bash
mkdir -p crates/shadcn-egui-font-oxanium/fonts crates/shadcn-egui-font-oxanium/src
git mv crates/shadcn-egui/assets/Oxanium-Variable.ttf crates/shadcn-egui-font-oxanium/fonts/oxanium.ttf
git mv crates/shadcn-egui/assets/Oxanium-OFL.txt crates/shadcn-egui-font-oxanium/LICENSE-FONT.txt
git rm -q crates/shadcn-egui/assets/Oxanium-Regular.ttf crates/shadcn-egui/assets/Oxanium-Medium.ttf crates/shadcn-egui/assets/Oxanium-SemiBold.ttf
git rm -q crates/shadcn-egui/tests/variable_font_spike.rs
```

(The Phase-0 spike test included the moved file; `tests/fonts.rs` replaces
it in Task 2.3. `git rm` of tracked files on the feature branch is
recoverable from history.)

- [ ] **Step 3: Write the two crates**

`crates/shadcn-egui-font-inter/Cargo.toml`:

```toml
[package]
name = "shadcn-egui-font-inter"
description = "Inter for shadcn-egui"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license = "MIT AND OFL-1.1"
repository.workspace = true

[dependencies]
shadcn-egui-core.workspace = true
```

`crates/shadcn-egui-font-inter/src/lib.rs`:

```rust
//! Inter for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// Inter, variable weight 100–900.
pub static FONT: FontFiles = FontFiles {
    name: "inter",
    kind: FontKind::Sans,
    source: FontSource::Variable { data: include_bytes!("../fonts/inter.ttf"), wght: 100.0..=900.0 },
    y_offset_factor: 0.0,
    license: include_str!("../LICENSE-FONT.txt"),
};
```

(`y_offset_factor` 0.0 is provisional; Task 2.5 measures it.)

`crates/shadcn-egui-font-oxanium/Cargo.toml`: as Inter's, with
`name = "shadcn-egui-font-oxanium"` and `description = "Oxanium for shadcn-egui"`.

`crates/shadcn-egui-font-oxanium/src/lib.rs`:

```rust
//! Oxanium for shadcn-egui. The font is licensed under the SIL Open Font
//! License 1.1 (`LICENSE-FONT.txt`); this crate's code is MIT.

use shadcn_egui_core::{FontFiles, FontKind, FontSource};

/// Oxanium, variable weight 200–800.
pub static FONT: FontFiles = FontFiles {
    name: "oxanium",
    kind: FontKind::Sans,
    source: FontSource::Variable { data: include_bytes!("../fonts/oxanium.ttf"), wght: 200.0..=800.0 },
    y_offset_factor: 0.18,
    license: include_str!("../LICENSE-FONT.txt"),
};
```

Root `Cargo.toml`, `[workspace.dependencies]`:

```toml
shadcn-egui-font-inter = { path = "crates/shadcn-egui-font-inter", version = "0.4.0" }
shadcn-egui-font-oxanium = { path = "crates/shadcn-egui-font-oxanium", version = "0.4.0" }
```

- [ ] **Step 4: Prove the declared axis ranges with a test**

`crates/shadcn-egui/tests/fonts.rs` (new file; Task 2.3 adds more tests):

```rust
//! Fonts: the theme's font install, weights and families.

mod common;

use shadcn_egui::{FontFiles, FontSource};

fn bytes(f: &FontFiles) -> &'static [u8] {
    match &f.source {
        FontSource::Variable { data, .. } => data,
        FontSource::Static { regular, .. } => regular,
    }
}

#[test]
fn bundled_font_crates_ship_truetype_and_a_licence() {
    for f in [&shadcn_egui_font_inter::FONT, &shadcn_egui_font_oxanium::FONT] {
        assert_eq!(&bytes(f)[..4], &[0, 1, 0, 0], "{}: TrueType header", f.name);
        assert!(f.license.contains("SIL Open Font License"), "{}: OFL text", f.name);
    }
}
```

Add to `crates/shadcn-egui/Cargo.toml`:

```toml
[dependencies]
egui.workspace = true
egui_extras.workspace = true
shadcn-egui-core.workspace = true
shadcn-egui-font-inter.workspace = true
shadcn-egui-font-oxanium = { workspace = true, optional = true }

[features]
font-oxanium = ["dep:shadcn-egui-font-oxanium"]

[dev-dependencies]
eframe = "0.34"
egui_kittest = { version = "0.34", features = ["wgpu", "snapshot"] }
# Tests compare against a heading font that differs from the body font.
shadcn-egui-font-oxanium.workspace = true
```

`theme.rs` still `include_bytes!`s `../assets/Oxanium-*.ttf`; change those
three `install_fonts` lines to use `shadcn_egui_font_inter::FONT` so the
crate compiles until Task 2.3 rewrites the function:

```rust
    let inter = match &shadcn_egui_font_inter::FONT.source {
        shadcn_egui_core::FontSource::Variable { data, .. } => *data,
        shadcn_egui_core::FontSource::Static { regular, .. } => *regular,
    };
    fonts.font_data.insert("oxanium".into(), Arc::new(FontData::from_static(inter).tweak(tweak.clone())));
    fonts.font_data.insert(FAMILY_MEDIUM.into(), Arc::new(FontData::from_static(inter).tweak(tweak.clone())));
    fonts.font_data.insert(FAMILY_SEMIBOLD.into(), Arc::new(FontData::from_static(inter).tweak(tweak)));
```

- [ ] **Step 5: Run**

Run: `cargo test -p shadcn-egui --jobs 4 --test fonts -- --test-threads 4` → PASS.
Run: `cargo build --workspace --all-features --jobs 4` → builds.
(Snapshot tests are red from here until Task 2.6: the body font is Inter.)

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: Inter and Oxanium font crates

Inter becomes the bundled font (upstream DEFAULT_CONFIG); Oxanium moves
into its own optional crate behind the font-oxanium feature. Snapshot
baselines are regenerated at the end of the font phase.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2.3: Theme fonts, per-weight install, families, `apply_with_fonts`

**Files:**
- Modify: `crates/shadcn-egui/src/theme.rs` (font section, `Theme` struct, `Theme::new`, `apply`)
- Modify: `crates/shadcn-egui/src/lib.rs` (export `ThemeFonts`)
- Modify: `crates/shadcn-egui/tests/fonts.rs`

**Interfaces:**
- Consumes: `FontFiles`, `FontKind`, `WEIGHTS`, `FontFiles::face` (Task 2.1); `shadcn_egui_font_inter::FONT`.
- Produces (in `shadcn_egui::theme`, `ThemeFonts` also at the crate root):
  - `struct ThemeFonts { pub body: &'static FontFiles, pub heading: &'static FontFiles }` (`Clone, Copy, Debug, PartialEq`); `ThemeFonts::inherit(body)` sets `heading = body`.
  - `Theme.fonts: ThemeFonts` (new public field); `Theme::new` sets `ThemeFonts::inherit(&shadcn_egui_font_inter::FONT)`.
  - `Theme::fonts(self, body: &'static FontFiles, heading: Option<&'static FontFiles>) -> Theme` (by-value builder; `None` = inherit).
  - Family names: `FAMILY_MEDIUM = "shadcn-body-medium"`, `FAMILY_SEMIBOLD = "shadcn-body-semibold"`, `FAMILY_BOLD = "shadcn-body-bold"`, `FAMILY_HEADING = "shadcn-heading-regular"`, `FAMILY_HEADING_MEDIUM`, `FAMILY_HEADING_SEMIBOLD`, `FAMILY_HEADING_BOLD` (`"shadcn-heading-{medium,semibold,bold}"`). Body regular is `FontFamily::Proportional`.
  - Font data keys `"shadcn-body-400"` … `"shadcn-body-700"`, `"shadcn-heading-400"` … (heading keys only when heading ≠ body; otherwise the heading families list the body keys).
  - `family_for(ctx, Weight)` (Bold now maps to `FAMILY_BOLD`); new `heading_family_for(ctx, Weight) -> FontFamily`.
  - `Theme::apply(&self, ctx)`; new `Theme::apply_with_fonts(&self, ctx, base: FontDefinitions)`.
  - `TextStyle::Heading` → `FontFamily::Name(FAMILY_HEADING_SEMIBOLD)`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/shadcn-egui/tests/fonts.rs`:

```rust
use egui::{vec2, Color32, FontData, FontDefinitions, FontFamily, RichText};
use egui_kittest::Harness;
use shadcn_egui::style::Weight;
use shadcn_egui::theme::{family_for, heading_family_for, FAMILY_BOLD, FAMILY_HEADING, FAMILY_HEADING_BOLD, FAMILY_HEADING_MEDIUM, FAMILY_HEADING_SEMIBOLD, FAMILY_MEDIUM, FAMILY_SEMIBOLD};
use shadcn_egui::{FontKind, Theme};
use std::sync::Arc;

const OXANIUM: &shadcn_egui::FontFiles = &shadcn_egui_font_oxanium::FONT;

/// Ink (sum of 255 - green) of a word drawn black on white in `family`.
fn ink(theme: &Theme, family: fn(&egui::Context) -> FontFamily) -> u64 {
    let mut applied = false;
    let theme = theme.clone();
    let mut h = Harness::builder().with_size(vec2(560.0, 90.0)).build_ui(move |ui| {
        if !applied {
            theme.apply(ui.ctx());
            applied = true;
        }
        ui.painter().rect_filled(ui.ctx().viewport_rect(), 0.0, Color32::WHITE);
        ui.label(RichText::new("Hamburgefonstiv").family(family(ui.ctx())).size(40.0).color(Color32::BLACK));
    });
    for _ in 0..3 {
        h.step();
    }
    let img = h.render().expect("wgpu render");
    img.pixels().map(|p| u64::from(255 - p.0[1])).sum()
}

#[test]
fn each_weight_has_more_ink_than_the_one_below() {
    let t = Theme::vega_light();
    let inks = [
        ink(&t, |c| family_for(c, Weight::Normal)),
        ink(&t, |c| family_for(c, Weight::Medium)),
        ink(&t, |c| family_for(c, Weight::Semibold)),
        ink(&t, |c| family_for(c, Weight::Bold)),
    ];
    assert!(inks.windows(2).all(|w| w[1] > w[0]), "ink must grow with weight: {inks:?}");
}

fn definitions_after_apply(theme: &Theme) -> FontDefinitions {
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    ctx.fonts(|f| f.definitions().clone())
}

#[test]
fn every_named_family_falls_back_to_egui_fonts() {
    let defs = definitions_after_apply(&Theme::vega_dark());
    let egui_proportional = FontDefinitions::default().families[&FontFamily::Proportional].clone();
    for name in [FAMILY_MEDIUM, FAMILY_SEMIBOLD, FAMILY_BOLD, FAMILY_HEADING, FAMILY_HEADING_MEDIUM, FAMILY_HEADING_SEMIBOLD, FAMILY_HEADING_BOLD] {
        let list = &defs.families[&FontFamily::Name(name.into())];
        assert_eq!(&list[1..], &egui_proportional[..], "{name}: egui's fonts follow the theme font");
    }
    assert_eq!(defs.families[&FontFamily::Proportional][0], "shadcn-body-400");
    assert_eq!(&defs.families[&FontFamily::Proportional][1..], &egui_proportional[..]);
}

#[test]
fn inherit_maps_heading_families_to_the_body_font() {
    let defs = definitions_after_apply(&Theme::vega_dark());
    assert_eq!(defs.families[&FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into())][0], "shadcn-body-600");
    assert!(!defs.font_data.contains_key("shadcn-heading-600"), "no second copy for an inherited heading");
}

#[test]
fn a_heading_font_gets_its_own_faces() {
    let defs = definitions_after_apply(&Theme::vega_dark().fonts(&shadcn_egui_font_inter::FONT, Some(OXANIUM)));
    assert_eq!(defs.families[&FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into())][0], "shadcn-heading-600");
    assert_eq!(defs.font_data["shadcn-heading-600"].font.len(), OXANIUM.face(600).data.len());
}

#[test]
fn monospace_takes_the_body_font_only_when_it_is_mono() {
    let defs = definitions_after_apply(&Theme::vega_dark());
    assert_eq!(defs.families[&FontFamily::Monospace][0], "Hack", "a sans body leaves egui's mono font");
    let mono: &'static shadcn_egui::FontFiles = Box::leak(Box::new(shadcn_egui::FontFiles { name: "fake-mono", kind: FontKind::Mono, ..shadcn_egui_font_inter::FONT.clone() }));
    let defs = definitions_after_apply(&Theme::vega_dark().fonts(mono, None));
    assert_eq!(defs.families[&FontFamily::Monospace][0], "shadcn-body-400");
}

#[test]
fn heading_text_style_uses_the_heading_semibold_family() {
    let ctx = egui::Context::default();
    Theme::vega_dark().apply(&ctx);
    let id = &ctx.global_style().text_styles[&egui::TextStyle::Heading];
    assert_eq!(id.family, FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into()));
}

/// Marks the installed definitions so a rebuild is visible: a rebuild drops the marker.
fn mark(ctx: &egui::Context) {
    let mut defs = ctx.fonts(|f| f.definitions().clone());
    defs.font_data.insert("marker".into(), Arc::new(FontData::from_static(OXANIUM.face(400).data)));
    ctx.set_fonts(defs);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
}

fn has_marker(ctx: &egui::Context) -> bool {
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    ctx.fonts(|f| f.definitions().font_data.contains_key("marker"))
}

#[test]
fn a_mode_toggle_does_not_reinstall_fonts() {
    let ctx = egui::Context::default();
    Theme::vega_light().apply(&ctx);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    mark(&ctx);
    Theme::vega_dark().apply(&ctx);
    assert!(has_marker(&ctx), "same fonts: the atlas is not rebuilt");
    Theme::vega_dark().fonts(&shadcn_egui_font_inter::FONT, Some(OXANIUM)).apply(&ctx);
    assert!(!has_marker(&ctx), "new heading font: fonts are reinstalled");
}

#[test]
fn apply_with_fonts_keeps_the_apps_fonts() {
    let ctx = egui::Context::default();
    let mut base = FontDefinitions::default();
    base.font_data.insert("app-font".into(), Arc::new(FontData::from_static(OXANIUM.face(400).data)));
    base.families.insert(FontFamily::Name("app".into()), vec!["app-font".into()]);
    Theme::vega_light().apply_with_fonts(&ctx, base);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    let defs = ctx.fonts(|f| f.definitions().clone());
    assert!(defs.families.contains_key(&FontFamily::Name("app".into())));
    assert!(defs.families.contains_key(&FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into())));
}

/// Review Focus 2: an app that manages its fonts toggles light/dark with `apply`.
#[test]
fn app_fonts_survive_a_mode_toggle() {
    let ctx = egui::Context::default();
    let mut base = FontDefinitions::default();
    base.font_data.insert("app-font".into(), Arc::new(FontData::from_static(OXANIUM.face(400).data)));
    Theme::vega_light().apply_with_fonts(&ctx, base);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    Theme::vega_dark().apply(&ctx);
    let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
    assert!(ctx.fonts(|f| f.definitions().font_data.contains_key("app-font")));
}

/// Review Focus 5: switching fonts on a live context, with a heading drawn every frame.
#[test]
fn switching_fonts_on_a_live_context_does_not_panic() {
    use shadcn_egui::components::card::{card_header, Card};
    let frame = std::cell::Cell::new(0);
    let mut h = Harness::builder().with_size(vec2(360.0, 160.0)).build_ui(|ui| {
        let n = frame.get();
        if n == 0 {
            Theme::vega_dark().apply(ui.ctx());
        }
        if n == 2 {
            Theme::vega_dark().fonts(&shadcn_egui_font_inter::FONT, Some(OXANIUM)).apply(ui.ctx());
        }
        frame.set(n + 1);
        Card::new().show(ui, |ui| card_header(ui, "Title", "Description"));
        let _ = heading_family_for(ui.ctx(), Weight::Semibold);
        ui.heading("egui heading");
    });
    for _ in 0..6 {
        h.step();
    }
    let defs = h.ctx.fonts(|f| f.definitions().clone());
    assert_eq!(defs.families[&FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into())][0], "shadcn-heading-600");
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p shadcn-egui --jobs 4 --test fonts -- --test-threads 4`
Expected: FAIL to compile (`heading_family_for`, `FAMILY_BOLD`, `Theme::fonts`, `apply_with_fonts` missing).

- [ ] **Step 3: Implement — `Theme` field and builder**

In `crates/shadcn-egui/src/theme.rs`, the `Theme` struct gains (after `dark`):

```rust
    /// Body and heading font; [`Theme::apply`] installs them.
    pub fonts: ThemeFonts,
```

`Theme::new` ends with:

```rust
        Self { style, palette, radius, metrics: Metrics::default(), dark, fonts: ThemeFonts::inherit(&shadcn_egui_font_inter::FONT) }
```

and a builder after `vega_light`:

```rust
    /// Body font and heading font (`None`: headings use the body font,
    /// upstream `fontHeading: "inherit"`).
    pub fn fonts(mut self, body: &'static FontFiles, heading: Option<&'static FontFiles>) -> Self {
        self.fonts = ThemeFonts { body, heading: heading.unwrap_or(body) };
        self
    }
```

- [ ] **Step 4: Implement — replace the whole `// ---- Font + apply + accessor ----` font part**

Replace everything from `/// Named font families for shadcn-style weight emphasis.` down to the end of
`fn install_fonts` with:

```rust
use egui::epaint::text::VariationCoords;
use shadcn_egui_core::{FontFiles, FontKind, WEIGHTS};

/// A theme's fonts. `heading == body` is upstream `fontHeading: "inherit"`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeFonts {
    pub body: &'static FontFiles,
    pub heading: &'static FontFiles,
}

impl ThemeFonts {
    /// Headings in the body font.
    pub fn inherit(body: &'static FontFiles) -> Self {
        Self { body, heading: body }
    }
}

/// Named font families. Body regular is `FontFamily::Proportional`.
pub const FAMILY_MEDIUM: &str = "shadcn-body-medium";
pub const FAMILY_SEMIBOLD: &str = "shadcn-body-semibold";
pub const FAMILY_BOLD: &str = "shadcn-body-bold";
pub const FAMILY_HEADING: &str = "shadcn-heading-regular";
pub const FAMILY_HEADING_MEDIUM: &str = "shadcn-heading-medium";
pub const FAMILY_HEADING_SEMIBOLD: &str = "shadcn-heading-semibold";
pub const FAMILY_HEADING_BOLD: &str = "shadcn-heading-bold";

/// Families per [`WEIGHTS`] entry; `None` is `Proportional`.
const BODY_FAMILIES: [Option<&str>; 4] = [None, Some(FAMILY_MEDIUM), Some(FAMILY_SEMIBOLD), Some(FAMILY_BOLD)];
const HEADING_FAMILIES: [&str; 4] = [FAMILY_HEADING, FAMILY_HEADING_MEDIUM, FAMILY_HEADING_SEMIBOLD, FAMILY_HEADING_BOLD];

fn weight_index(w: Weight) -> usize {
    match w {
        Weight::Normal => 0,
        Weight::Medium => 1,
        Weight::Semibold => 2,
        Weight::Bold => 3,
    }
}

/// Return the named font family if it's registered (via `Theme::apply`), else
/// fall back to Proportional. egui defers `set_fonts` to the next frame, so on
/// the very first frame after `apply` a named family may not be in the atlas yet.
pub fn family(ctx: &egui::Context, name: &str) -> egui::FontFamily {
    let fam = egui::FontFamily::Name(name.into());
    if ctx.fonts(|f| f.definitions().families.contains_key(&fam)) {
        fam
    } else {
        egui::FontFamily::Proportional
    }
}

/// The body font family for a weight.
pub fn family_for(ctx: &egui::Context, w: Weight) -> egui::FontFamily {
    match BODY_FAMILIES[weight_index(w)] {
        None => egui::FontFamily::Proportional,
        Some(name) => family(ctx, name),
    }
}

/// The heading font family for a weight (upstream `cn-font-heading`).
pub fn heading_family_for(ctx: &egui::Context, w: Weight) -> egui::FontFamily {
    family(ctx, HEADING_FAMILIES[weight_index(w)])
}

/// `base` plus the theme's fonts: one registration per weight with the
/// `wght` coordinate, each family followed by `base`'s Proportional list so
/// glyphs the font lacks (emoji, CJK, symbols) still render.
fn font_definitions(fonts: ThemeFonts, mut base: FontDefinitions) -> FontDefinitions {
    let fallback = base.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let inherit = fonts.heading == fonts.body;
    for (role, files) in [("body", fonts.body), ("heading", fonts.heading)] {
        let data_role = if role == "heading" && inherit { "body" } else { role };
        for (i, w) in WEIGHTS.into_iter().enumerate() {
            let key = format!("shadcn-{data_role}-{w}");
            if data_role == role {
                let face = files.face(w);
                let mut tweak = egui::FontTweak { y_offset_factor: files.y_offset_factor, ..Default::default() };
                if let Some(v) = face.wght {
                    tweak.coords = VariationCoords::new([(b"wght", v)]);
                }
                base.font_data.insert(key.clone(), Arc::new(FontData::from_static(face.data).tweak(tweak)));
            }
            let family = match (role, i) {
                ("body", 0) => FontFamily::Proportional,
                ("body", _) => FontFamily::Name(BODY_FAMILIES[i].expect("named body family").into()),
                _ => FontFamily::Name(HEADING_FAMILIES[i].into()),
            };
            let mut list = vec![key];
            list.extend(fallback.iter().cloned());
            base.families.insert(family, list);
        }
    }
    if fonts.body.kind == FontKind::Mono {
        base.families.entry(FontFamily::Monospace).or_default().insert(0, "shadcn-body-400".into());
    }
    base
}

/// The fonts last installed on a context by [`Theme::apply`].
#[derive(Clone, PartialEq)]
struct InstalledFonts {
    body: &'static str,
    heading: &'static str,
}

/// Install the theme's fonts. With `base: None` (plain `apply`) nothing
/// happens when the same fonts are installed already, so a light/dark toggle
/// neither rebuilds the atlas nor drops fonts an app added through
/// `apply_with_fonts`.
fn install_fonts(ctx: &egui::Context, fonts: ThemeFonts, base: Option<FontDefinitions>) {
    let id = egui::Id::new("shadcn_egui::installed_fonts");
    let key = InstalledFonts { body: fonts.body.name, heading: fonts.heading.name };
    if base.is_none() && ctx.data(|d| d.get_temp::<InstalledFonts>(id)).as_ref() == Some(&key) {
        return;
    }
    ctx.set_fonts(font_definitions(fonts, base.unwrap_or_default()));
    ctx.data_mut(|d| d.insert_temp(id, key));
}
```

- [ ] **Step 5: Implement — `apply` and `apply_with_fonts`**

Replace the doc comment and signature of `apply` and its first line:

```rust
    /// Push this theme into the egui context: fonts, type scale, spacing, colors.
    ///
    /// Call once, right after the context is created, and again only when the
    /// theme changes. Not every frame: that would silently undo any other
    /// code's style writes. Fonts are rebuilt from egui's defaults plus the
    /// theme's fonts, and only when they differ from the last install on
    /// this context. **An app that installs fonts of its own must use
    /// [`Theme::apply_with_fonts`]** and must not call `ctx.set_fonts` itself
    /// afterwards: that would drop the theme's families, and egui panics when
    /// `TextStyle::Heading` names a family that is gone.
    pub fn apply(&self, ctx: &egui::Context) {
        self.apply_inner(ctx, None);
    }

    /// Like [`Theme::apply`], starting from the app's own font definitions
    /// instead of egui's defaults. Always reinstalls the fonts.
    pub fn apply_with_fonts(&self, ctx: &egui::Context, base: FontDefinitions) {
        self.apply_inner(ctx, Some(base));
    }

    fn apply_inner(&self, ctx: &egui::Context, base: Option<FontDefinitions>) {
        install_fonts(ctx, self.fonts, base);
```

(the rest of the old `apply` body follows unchanged), and in the text-style
table:

```rust
                (TextStyle::Heading, FontId::new(m.text_heading, FontFamily::Name(FAMILY_HEADING_SEMIBOLD.into()))),
```

Remove the provisional Inter lines from Task 2.2 (they were inside the old
`install_fonts`). `crates/shadcn-egui/src/lib.rs`:

```rust
pub use theme::{theme, Metrics, Palette, Theme, ThemeFonts};
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p shadcn-egui --jobs 4 --test fonts --test theme --test theme_switch -- --test-threads 4`
Expected: all pass. Then run the whole suite and confirm that the only
failures are snapshot comparisons (`… snapshot …` tests reporting pixel
differences); list them in the commit message body.

- [ ] **Step 7: Clippy and commit**

```bash
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
git add -A
git commit -m "feat: install fonts per weight with variable coordinates

Theme gains body and heading fonts. Theme::apply registers each font
once per weight (400/500/600/700) with FontTweak::coords, so medium and
semibold render at their real weight; every family keeps egui's fonts as
glyph fallback; fonts are only reinstalled when they change; apps that
manage fonts use apply_with_fonts. Snapshot baselines are regenerated in
the last task of this phase.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2.4: Card and Dialog titles use the heading font

**Files:**
- Modify: `crates/shadcn-egui/src/components/card.rs:21,247`
- Modify: `crates/shadcn-egui/src/components/dialog.rs:17,143`
- Modify: `crates/shadcn-egui/tests/common/mod.rs` (add `snapshot_with_themes`)
- Modify: `crates/shadcn-egui/tests/fonts.rs`

**Interfaces:**
- Consumes: `heading_family_for` (Task 2.3).
- Produces: `common::snapshot_with_themes(name: &str, size: egui::Vec2, themes: Vec<(&'static str, Theme)>, ui_fn: impl FnMut(&mut egui::Ui), prepare: impl FnMut(&mut Harness<'_>))`; `snapshot_all_with` calls it with `themes()`. Phase 5 uses it.

- [ ] **Step 1: Generalise the snapshot helper**

In `crates/shadcn-egui/tests/common/mod.rs`, rename the body of
`snapshot_all_with` into a new function and make the old one delegate:

```rust
/// Like [`snapshot_all`], with `prepare` run on each harness after the first
/// frame (hover a trigger, press a key) and before the snapshot.
pub fn snapshot_all_with(
    name: &str,
    size: egui::Vec2,
    ui_fn: impl FnMut(&mut egui::Ui),
    prepare: impl FnMut(&mut Harness<'_>),
) {
    snapshot_with_themes(name, size, themes(), ui_fn, prepare);
}

/// One snapshot per `(suffix, theme)`: `{name}_{suffix}.png`.
pub fn snapshot_with_themes(
    name: &str,
    size: egui::Vec2,
    themes: Vec<(&'static str, Theme)>,
    mut ui_fn: impl FnMut(&mut egui::Ui),
    mut prepare: impl FnMut(&mut Harness<'_>),
) {
    // One result set for all harnesses, so a failing style does not abort the rest.
    let mut results = SnapshotResults::new();
    for (suffix, theme) in themes {
        let bg = theme.palette.background;
        let mut h = Harness::builder().with_size(size).build_ui(|ui| {
            theme.apply(ui.ctx());
            // kittest wraps the ui in an 8px outer margin; paint the page
            // colour under all of it, so no band of another colour shows.
            let page = ui.ctx().viewport_rect();
            ui.ctx().layer_painter(egui::LayerId::background()).rect_filled(page, 0.0, bg);
            egui::Frame::new().fill(bg).inner_margin(16).show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                ui_fn(ui);
            });
        });
        h.run();
        prepare(&mut h);
        h.snapshot(format!("{name}_{suffix}"));
        results.extend(h.take_snapshot_results());
    }
}
```

- [ ] **Step 2: Write the failing heading snapshot tests**

Append to `crates/shadcn-egui/tests/fonts.rs`:

```rust
fn heading_themes() -> Vec<(&'static str, Theme)> {
    common::themes().into_iter().map(|(n, t)| (n, t.fonts(&shadcn_egui_font_inter::FONT, Some(OXANIUM)))).collect()
}

#[test]
fn card_title_uses_the_heading_font_snapshot() {
    use shadcn_egui::components::card::{card_content, card_header, Card};
    common::snapshot_with_themes("card_heading_font", vec2(360.0, 180.0), heading_themes(), |ui| {
        Card::new().show(ui, |ui| {
            card_header(ui, "Oxanium heading", "Body text stays Inter");
            card_content(ui, |ui| {
                ui.label("Card content");
            });
        });
    }, |_| {});
}

#[test]
fn dialog_title_uses_the_heading_font_snapshot() {
    use shadcn_egui::components::dialog::Dialog;
    common::snapshot_with_themes("dialog_heading_font", vec2(480.0, 260.0), heading_themes(), |ui| {
        let mut open = true;
        Dialog::new("heading_font", "Oxanium heading").description("Body text stays Inter").show(ui.ctx(), &mut open, |ui| {
            ui.label("Dialog content");
        });
    }, |_| {});
}
```

Run: `UPDATE_SNAPSHOTS=1 cargo test -p shadcn-egui --jobs 4 --test fonts heading_font -- --test-threads 4`,
then open `crates/shadcn-egui/tests/snapshots/card_heading_font_vega_light.png`.
Expected: the title is drawn in Inter (the bug: `family_for`), visibly not
Oxanium. This is the failing state; do not commit these PNGs.

- [ ] **Step 3: Use the heading family**

`crates/shadcn-egui/src/components/card.rs`: import line 21 becomes
`use crate::theme::heading_family_for;` and line 247:

```rust
    // upstream: card-title carries `cn-font-heading`
    ui.label(RichText::new(text).family(heading_family_for(ui.ctx(), weight)).size(px).line_height(Some(line)))
```

`crates/shadcn-egui/src/components/dialog.rs`: import line 17 becomes
`use crate::theme::heading_family_for;` and line 143:

```rust
        // upstream: dialog-title carries `cn-font-heading`
        ui.label(RichText::new(title).family(heading_family_for(ui.ctx(), weight)).size(px).line_height(Some(line)));
```

(If `family_for` is still used elsewhere in either file, import both.)

- [ ] **Step 4: Regenerate and look**

Run: `UPDATE_SNAPSHOTS=1 cargo test -p shadcn-egui --jobs 4 --test fonts heading_font -- --test-threads 4`
Open the 8 new PNGs (`card_heading_font_*`, `dialog_heading_font_*`).
Expected: titles in Oxanium (square, technical shapes), descriptions and
content in Inter. They are reviewed by the user together with all other
baselines in Task 2.6.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: card and dialog titles use the heading font (D17)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2.5: Measure `y_offset_factor` for Inter

**Files:**
- Create: `crates/shadcn-egui/tests/font_offset.rs`
- Modify: `crates/shadcn-egui-font-inter/src/lib.rs` (`y_offset_factor`)
- Modify: `tools/fontgen/FONTS.tsv` (inter row, last column)

**Interfaces:**
- Produces: an `#[ignore]`d measurement test, run on demand with
  `FONT=<id>`; Phase 7 extends `files()` to every font crate.

- [ ] **Step 1: Write the measurement test**

`crates/shadcn-egui/tests/font_offset.rs`:

```rust
//! Measures `FontFiles::y_offset_factor`: the factor that puts the ink
//! centre of capital letters on the vertical centre of a default button,
//! the way Oxanium's 0.18 was found by eye. Run on demand:
//! `FONT=inter cargo test -p shadcn-egui --test font_offset -- --ignored --nocapture`

use egui::vec2;
use egui_kittest::Harness;
use shadcn_egui::components::button::Button;
use shadcn_egui::{FontFiles, Theme};

fn files(name: &str) -> &'static FontFiles {
    match name {
        "inter" => &shadcn_egui_font_inter::FONT,
        "oxanium" => &shadcn_egui_font_oxanium::FONT,
        other => panic!("unknown font {other}"),
    }
}

/// Ink centre of "HHHH" minus the button centre, in px, at `factor`.
fn offset(font: &FontFiles, factor: f32) -> f32 {
    let probe: &'static FontFiles = Box::leak(Box::new(FontFiles { y_offset_factor: factor, ..font.clone() }));
    let theme = Theme::vega_light().fonts(probe, None);
    let rect = std::cell::Cell::new(egui::Rect::NOTHING);
    let mut applied = false;
    let mut h = Harness::builder().with_size(vec2(200.0, 80.0)).build_ui(|ui| {
        if !applied {
            theme.apply(ui.ctx());
            applied = true;
        }
        rect.set(ui.add(Button::new("HHHH")).rect);
    });
    for _ in 0..3 {
        h.step();
    }
    let img = h.render().expect("wgpu render");
    let r = rect.get();
    // Light text on the dark primary fill: ink is brightness above the fill.
    let fill = f64::from(img.get_pixel(r.left() as u32 + 2, r.center().y as u32).0[1]);
    let (mut sum, mut weight) = (0.0f64, 0.0f64);
    for y in r.top().ceil() as u32..r.bottom().floor() as u32 {
        for x in r.left() as u32 + 4..r.right() as u32 - 4 {
            let w = (f64::from(img.get_pixel(x, y).0[1]) - fill).max(0.0);
            sum += w * (f64::from(y) + 0.5);
            weight += w;
        }
    }
    assert!(weight > 0.0, "no text ink found");
    (sum / weight) as f32 - r.center().y
}

#[test]
#[ignore = "measurement, run on demand with FONT=<id>"]
fn measure_y_offset_factor() {
    let name = std::env::var("FONT").expect("set FONT=<font id>");
    let font = files(&name);
    let (mut lo, mut hi) = (-0.5f32, 0.5f32);
    for _ in 0..12 {
        let mid = (lo + hi) / 2.0;
        if offset(font, mid) > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let factor = ((lo + hi) / 2.0 * 100.0).round() / 100.0;
    println!("{name}\t{factor}\t(residual {:.2} px)", offset(font, factor));
}
```

- [ ] **Step 2: Calibrate against Oxanium**

Run: `FONT=oxanium cargo test -p shadcn-egui --jobs 4 --test font_offset -- --ignored --nocapture --test-threads 1`
Expected: `oxanium	0.18	…` or within ±0.03 of it. **If the result is
further than 0.03 from 0.18, STOP** and ask the user which to trust (the
by-eye value or this method); record both numbers.

- [ ] **Step 3: Measure Inter and store it**

Run: `FONT=inter cargo test -p shadcn-egui --jobs 4 --test font_offset -- --ignored --nocapture --test-threads 1`
Put the printed factor into `crates/shadcn-egui-font-inter/src/lib.rs`
(`y_offset_factor: <value>,`) and into the `inter` row of
`tools/fontgen/FONTS.tsv` (last column, replacing `-`).

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "test: measure y_offset_factor per font; set Inter's

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2.6: Regenerate baselines, docs, changelog — user review

**Files:**
- Modify: every PNG under `crates/shadcn-egui/tests/snapshots/`
- Modify: `skills/egui-shadcn/references/gotchas.md` (the `cn-font-heading` bullet), `skills/egui-shadcn/references/token-map.md` (the "14px / medium" row)
- Modify: `CHANGES.md`

- [ ] **Step 1: Regenerate**

```bash
UPDATE_SNAPSHOTS=1 cargo test --workspace --jobs 4 -- --test-threads 4
cargo test --workspace --jobs 4 -- --test-threads 4
cargo test --workspace --all-features --jobs 4 -- --test-threads 4
git status --short crates/shadcn-egui/tests/snapshots | wc -l
```

Expected: the second and third runs pass; every existing PNG changed (Inter
replaces Oxanium; weights differ) plus 8 new heading PNGs.

- [ ] **Step 2: Look at every PNG yourself**

Open each changed PNG. Check: medium/semibold labels (buttons, badges, tabs,
card titles) are visibly heavier than body text; capitals sit centred in
buttons, badges and tabs; no text clipped at the top or bottom of inputs and
selects; emoji/symbol glyphs (if any) still render.

- [ ] **Step 3: STOP — user review**

Show the user the list of changed PNGs (`git status --short crates/shadcn-egui/tests/snapshots`)
and ask them to review and approve them. Do not continue until they approve;
apply their requested changes first.

- [ ] **Step 4: Docs**

`skills/egui-shadcn/references/gotchas.md` — replace the bullet that starts
`- **\`cn-font-heading\` is ignored.**` with:

```markdown
- **Heading font.** Upstream marks card and dialog titles with
  `cn-font-heading`; the crate draws them with `heading_family_for(ctx, weight)`,
  which is the theme's heading font (`Theme::fonts(body, Some(heading))`) or
  the body font when none is set.
```

`skills/egui-shadcn/references/token-map.md` — the row becomes:

```markdown
| 14px / medium | `text_styles[Body]` + `theme::family_for(ctx, Weight::Medium)` |
```

- [ ] **Step 5: Changelog**

At the top of the `## [Unreleased]` → `### Changed` list in `CHANGES.md`:

```markdown
- Medium and semibold text (button labels, badges, tabs, card and dialog
  titles) now renders at its real weight; before, every weight looked like
  regular text.
- The default font is Inter, as on the shadcn site, so all text looks
  different from 0.3. Oxanium remains available through the `font-oxanium`
  feature.
- Card and dialog titles use the theme's heading font, which is the body
  font unless `Theme::fonts(body, Some(heading))` sets another one.
- An app that installs fonts of its own calls
  `Theme::apply_with_fonts(ctx, its_font_definitions)` instead of
  `Theme::apply`; this keeps its fonts and avoids the crash that followed
  replacing the fonts after `apply`.
```

- [ ] **Step 6: Phase gates and commit**

Run every gate from the index (both feature sets, clippy, tools, drift).

```bash
git add -A
git commit -m "test: regenerate baselines for Inter and real font weights

Every component snapshot changes: Inter replaces Oxanium and medium and
semibold now render at their weight. Reviewed and approved by the user.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
