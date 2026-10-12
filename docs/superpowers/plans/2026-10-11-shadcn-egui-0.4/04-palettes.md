# Phase 3: Palettes (generated base colours and themes, chart colours)

> Part of [00-index.md](00-index.md). Read its Global Constraints first.

**Spec:** D6, D19, §5.1, §7 (palettes: merged palettes equal upstream
`buildRegistryTheme`).
**Index decisions applied:** G3 (theme ids are looked up in the bases first,
then in the overlays), G4 (`destructive-foreground` = `oklch(0.985 0 0)`),
§J (amendment 2026-10-11: palettes, preset lists and fonts come from the
shadcn/ui (React) checkout `$UP_REACT`, pinned in `tools/stylegen/PIN_REACT`;
style CSS stays on `$UP`; `libraries.ts` is not read).

**Outcome:** stylegen reads upstream (React) `themes.ts` and `base-colors.ts` and writes
`crates/shadcn-egui/src/palette/generated.rs` (7 base palettes, 17 colour
overlays, light and dark, as OKLCH constants). `Palette` gains
`chart: [Color32; 5]` and `Palette::from_upstream(base, theme, chart, dark)`,
which merges like upstream `buildRegistryTheme` and is checked against
fixtures produced by running upstream's own code. `stylegen generate` keeps
byte copies of every upstream source it reads in `tools/stylegen/tests/fixtures/`,
and `stylegen drift` reports when one of them no longer matches `$UP` or
`$UP_REACT`.

No rendering changes in this phase: no snapshot changes, no PNG review.

---

### Task 3.1: stylegen parses `themes.ts`

**Files:**
- Create: `tools/stylegen/src/themes.rs`
- Modify: `tools/stylegen/src/lib.rs` (add `pub mod themes;`)
- Modify: `tools/stylegen/src/emit.rs` (`header` becomes `pub(crate)`; new `pub(crate) fn header_react`)
- Create: `tools/stylegen/tests/themes.rs`
- Create: `tools/stylegen/PIN_REACT` (`c2a67849`)
- Create: `tools/stylegen/tests/fixtures/{themes.ts,base-colors.ts}` (byte copies of `$UP_REACT/apps/v4/registry/{themes.ts,base-colors.ts}`), `tools/stylegen/tests/fixtures/SHADCN-UI-LICENSE` (copy of `$UP_REACT/LICENSE.md`)

**Interfaces:**
- Consumes: `stylegen::emit::header(pin: &str, source: &str) -> String` (exists, made `pub(crate)`); new `stylegen::emit::header_react(pin, source)` (same text, `shadcn/ui` in place of `shadcn-svelte`).
- Produces (all `pub` in `stylegen::themes`):
  - `struct Oklch { pub l: f32, pub c: f32, pub h: f32, pub alpha: f32 }` (`Clone, Copy, Debug, PartialEq`)
  - `struct ThemeEntry { pub name: String, pub radius_rem: Option<f32>, pub light: Vec<(String, Oklch)>, pub dark: Vec<(String, Oklch)> }` (`Clone, Debug, Default, PartialEq`)
  - `struct Themes { pub bases: Vec<ThemeEntry>, pub overlays: Vec<ThemeEntry> }` (`Clone, Debug, Default, PartialEq`)
  - `const VARS: [(&str, &str); 23]`: upstream var name → crate `Var` variant, in `Var` order.
  - `fn parse_oklch(s: &str) -> Result<Oklch, String>`
  - `fn parse_themes(themes_ts: &str, base_colors_ts: &str) -> Result<Themes, String>`: React `THEMES` entries; the names `base-colors.ts` filters are the bases (in `THEMES` order), all others overlays
  - `fn emit_palettes(pin: &str, themes: &Themes) -> Result<String, String>`: the text of `palette/generated.rs` (index C).

- [ ] **Step 1: Pin the React checkout and copy the fixtures**

```bash
test "$(git -C "$UP_REACT" rev-parse HEAD)" = c2a67849be260701852b0977ba15eb5f3a0ce2a4
echo c2a67849 > tools/stylegen/PIN_REACT
cp "$UP_REACT/apps/v4/registry/themes.ts" tools/stylegen/tests/fixtures/themes.ts
cp "$UP_REACT/apps/v4/registry/base-colors.ts" tools/stylegen/tests/fixtures/base-colors.ts
cp "$UP_REACT/LICENSE.md" tools/stylegen/tests/fixtures/SHADCN-UI-LICENSE
grep -c 'name: "' tools/stylegen/tests/fixtures/themes.ts   # 24 (no "gray")
grep -n 'includes(' tools/stylegen/tests/fixtures/base-colors.ts   # the 7 base names on one line
```

- [ ] **Step 2: Write the failing tests**

`tools/stylegen/tests/themes.rs`:

```rust
use stylegen::themes::{emit_palettes, parse_oklch, parse_themes, Oklch, ThemeEntry, Themes, VARS};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn fixture() -> Themes {
    let src = std::fs::read_to_string(format!("{ROOT}/tests/fixtures/themes.ts")).expect("themes.ts fixture");
    let bases = std::fs::read_to_string(format!("{ROOT}/tests/fixtures/base-colors.ts")).expect("base-colors.ts fixture");
    parse_themes(&src, &bases).expect("fixture parses")
}

/// A `base-colors.ts` whose filter names the one base `x`.
const BASES_X: &str = "export const BASE_COLORS = THEMES.filter((theme) =>\n  [\"x\"].includes(\n    theme.name\n  )\n)\n";

#[test]
fn oklch_values_parse() {
    assert_eq!(parse_oklch("oklch(1 0 0)"), Ok(Oklch { l: 1.0, c: 0.0, h: 0.0, alpha: 1.0 }));
    assert_eq!(parse_oklch("oklch(0.147 0.004 49.25)"), Ok(Oklch { l: 0.147, c: 0.004, h: 49.25, alpha: 1.0 }));
    assert_eq!(parse_oklch("oklch(1 0 0 / 10%)").map(|o| o.alpha), Ok(0.1));
    assert_eq!(parse_oklch("oklch(0.5 0.1 20 / 0.5)").map(|o| o.alpha), Ok(0.5));
    assert!(parse_oklch("rgb(1 2 3)").is_err());
    assert!(parse_oklch("oklch(1 0)").is_err());
    assert!(parse_oklch("oklch(1 x 0)").is_err());
}

#[test]
fn the_fixture_has_seven_bases_and_seventeen_overlays_in_upstream_order() {
    let t = fixture();
    let bases: Vec<&str> = t.bases.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(bases, ["neutral", "stone", "zinc", "mauve", "olive", "mist", "taupe"]);
    assert_eq!(t.overlays.len(), 17);
    assert_eq!(t.overlays.first().map(|o| o.name.as_str()), Some("amber"));
    assert_eq!(t.overlays.last().map(|o| o.name.as_str()), Some("yellow"));
    let neutral = &t.bases[0];
    assert_eq!(neutral.radius_rem, Some(0.625));
    assert_eq!(neutral.light[0], ("background".to_string(), Oklch { l: 1.0, c: 0.0, h: 0.0, alpha: 1.0 }));
    let border = neutral.dark.iter().find(|(n, _)| n == "border").expect("dark border");
    assert_eq!(border.1.alpha, 0.1);
    assert!(t.overlays[0].light.iter().any(|(n, _)| n == "primary"));
    assert!(t.overlays.iter().all(|o| o.radius_rem.is_none()));
}

#[test]
fn every_base_defines_every_var_in_both_modes() {
    for b in fixture().bases {
        for (mode, vars) in [("light", &b.light), ("dark", &b.dark)] {
            for (name, _) in VARS {
                assert!(vars.iter().any(|(n, _)| n == name), "{} {mode} lacks {name}", b.name);
            }
        }
    }
}

#[test]
fn emit_is_deterministic_and_skips_sidebar_and_radius() {
    let t = fixture();
    let a = emit_palettes("c2a67849", &t).expect("emit");
    assert_eq!(a, emit_palettes("c2a67849", &t).expect("emit"), "output differs between runs");
    assert!(a.starts_with("// @generated by tools/stylegen — do not edit."));
    assert!(a.contains("(\"neutral\", 0.625, &["));
    assert!(a.contains("(Background, O(1.0, 0.0, 0.0, 1.0)),"));
    assert!(a.contains("(Border, O(1.0, 0.0, 0.0, 0.1)),"));
    assert!(!a.contains("Sidebar") && !a.contains("sidebar"));
}

fn entry(name: &str, light: &[&str]) -> ThemeEntry {
    let c = Oklch { l: 0.5, c: 0.0, h: 0.0, alpha: 1.0 };
    let vars: Vec<(String, Oklch)> = light.iter().map(|n| (n.to_string(), c)).collect();
    ThemeEntry { name: name.into(), radius_rem: Some(0.625), light: vars.clone(), dark: vars }
}

#[test]
fn unknown_vars_and_incomplete_bases_are_errors() {
    let all: Vec<&str> = VARS.iter().map(|v| v.0).collect();
    let complete = Themes { bases: vec![entry("b", &all)], overlays: vec![] };
    assert!(emit_palettes("pin", &complete).is_ok());

    let mut unknown = all.clone();
    unknown.push("glow");
    let err = emit_palettes("pin", &Themes { bases: vec![entry("b", &unknown)], overlays: vec![] }).unwrap_err();
    assert!(err.contains("unknown var `glow`"), "{err}");

    let err = emit_palettes("pin", &Themes { bases: vec![entry("b", &all[1..])], overlays: vec![] }).unwrap_err();
    assert!(err.contains("lacks background"), "{err}");

    let mut no_radius = entry("b", &all);
    no_radius.radius_rem = None;
    assert!(emit_palettes("pin", &Themes { bases: vec![no_radius], overlays: vec![] }).is_err());

    let overlay = Themes { bases: vec![], overlays: vec![entry("o", &["primary", "sidebar-ring"])] };
    assert!(emit_palettes("pin", &overlay).is_ok(), "overlays may be partial; sidebar vars are skipped");
}

#[test]
fn a_var_line_that_is_not_oklch_or_rem_is_an_error() {
    let src = "export const THEMES: RegistryItem[] = [\n  {\n    name: \"x\",\n    cssVars: {\n      light: {\n        background: \"#fff\",\n      },\n    },\n  },\n] as const satisfies RegistryItem[]\n";
    let err = parse_themes(src, BASES_X).unwrap_err();
    assert!(err.starts_with("themes.ts:6:"), "{err}");
}

#[test]
fn a_base_missing_from_themes_is_an_error() {
    let src = "export const THEMES: RegistryItem[] = [\n  {\n    name: \"y\",\n  },\n] as const satisfies RegistryItem[]\n";
    assert!(parse_themes(src, BASES_X).unwrap_err().contains("`x`"));
    assert!(parse_themes(src, "no list here").is_err());
}
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 --test themes -- --test-threads 4`
Expected: FAIL to compile (`stylegen::themes` not found).

- [ ] **Step 4: Implement**

`tools/stylegen/src/emit.rs`: change `fn header(` to `pub(crate) fn header(`, and add
below it:

```rust
/// [`header`] for outputs generated from the shadcn/ui (React) checkout.
pub(crate) fn header_react(pin: &str, source: &str) -> String {
    header(pin, source).replacen("shadcn-svelte", "shadcn/ui", 1)
}
```

`tools/stylegen/src/lib.rs`: add `pub mod themes;` (keep the list alphabetical).

`tools/stylegen/src/themes.rs`:

```rust
//! shadcn/ui `apps/v4/registry/themes.ts` (+ `base-colors.ts`, which names
//! the bases) → `crates/shadcn-egui/src/palette/generated.rs`.
//!
//! A line-based reader: upstream writes one `key: "value",` per line inside
//! each `light: {` / `dark: {` map. Anything else inside such a map is an
//! error, so a format change upstream fails loudly instead of dropping vars.

/// An `oklch(l c h / alpha)` value; `alpha` is 1.0 when absent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    pub l: f32,
    pub c: f32,
    pub h: f32,
    pub alpha: f32,
}

/// One entry of `THEMES`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThemeEntry {
    pub name: String,
    /// `radius: "0.625rem"` (bases only).
    pub radius_rem: Option<f32>,
    /// Colour vars in source order, sidebar vars included.
    pub light: Vec<(String, Oklch)>,
    pub dark: Vec<(String, Oklch)>,
}

/// The base colours (`BASE_COLORS` of `base-colors.ts`) and the other
/// `THEMES` entries, the colour overlays, each in `THEMES` order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Themes {
    pub bases: Vec<ThemeEntry>,
    pub overlays: Vec<ThemeEntry>,
}

/// Upstream var → crate `palette::Var` variant, in `Var` order. A base
/// palette must define all of them; `sidebar-*` and `radius` are skipped.
pub const VARS: [(&str, &str); 23] = [
    ("background", "Background"),
    ("foreground", "Foreground"),
    ("card", "Card"),
    ("card-foreground", "CardForeground"),
    ("popover", "Popover"),
    ("popover-foreground", "PopoverForeground"),
    ("primary", "Primary"),
    ("primary-foreground", "PrimaryForeground"),
    ("secondary", "Secondary"),
    ("secondary-foreground", "SecondaryForeground"),
    ("muted", "Muted"),
    ("muted-foreground", "MutedForeground"),
    ("accent", "Accent"),
    ("accent-foreground", "AccentForeground"),
    ("destructive", "Destructive"),
    ("border", "Border"),
    ("input", "Input"),
    ("ring", "Ring"),
    ("chart-1", "Chart1"),
    ("chart-2", "Chart2"),
    ("chart-3", "Chart3"),
    ("chart-4", "Chart4"),
    ("chart-5", "Chart5"),
];

fn num(s: &str) -> Result<f32, String> {
    s.trim().parse::<f32>().map_err(|e| format!("bad number `{s}`: {e}"))
}

/// `oklch(L C H)` or `oklch(L C H / A)` with `A` a number or a percentage.
pub fn parse_oklch(s: &str) -> Result<Oklch, String> {
    let inner = s.strip_prefix("oklch(").and_then(|r| r.strip_suffix(')')).ok_or_else(|| format!("not oklch(…): {s}"))?;
    let (lch, alpha) = match inner.split_once('/') {
        Some((lch, a)) => {
            let a = a.trim();
            let alpha = match a.strip_suffix('%') {
                Some(p) => num(p)? / 100.0,
                None => num(a)?,
            };
            (lch, alpha)
        }
        None => (inner, 1.0),
    };
    let v: Vec<f32> = lch.split_whitespace().map(num).collect::<Result<_, _>>()?;
    let &[l, c, h] = v.as_slice() else {
        return Err(format!("oklch needs three numbers: {s}"));
    };
    Ok(Oklch { l, c, h, alpha })
}

/// The names in the `[...].includes(` filter of `base-colors.ts`.
fn base_names(base_colors_ts: &str) -> Result<Vec<String>, String> {
    let line = base_colors_ts
        .lines()
        .find(|l| l.contains("].includes("))
        .ok_or("base-colors.ts: no `[...].includes(` line")?;
    let list = line.split_once('[').and_then(|(_, r)| r.split_once(']')).map(|(l, _)| l).unwrap_or_default();
    let names: Vec<String> = list.split('"').skip(1).step_by(2).map(str::to_owned).collect();
    if names.is_empty() {
        return Err("base-colors.ts: empty base list".into());
    }
    Ok(names)
}

/// Read the `THEMES` array; `base_colors_ts` says which entries are bases.
pub fn parse_themes(src: &str, base_colors_ts: &str) -> Result<Themes, String> {
    let base_ids = base_names(base_colors_ts)?;
    let mut inside = false;
    let mut all: Vec<ThemeEntry> = Vec::new();
    // Some(true) inside a `dark: {` map, Some(false) inside `light: {`.
    let mut mode: Option<bool> = None;
    for (i, line) in src.lines().enumerate() {
        let n = i + 1;
        let t = line.trim();
        if t.starts_with("export const THEMES") && t.ends_with('[') {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if mode.is_none() && (t == "]" || t.starts_with("] as const")) {
            inside = false;
            continue;
        }
        let list = &mut all;
        if mode.is_none() {
            if let Some(name) = t.strip_prefix("name: \"").and_then(|r| r.strip_suffix("\",")) {
                list.push(ThemeEntry { name: name.to_owned(), ..ThemeEntry::default() });
                continue;
            }
        }
        match t {
            "light: {" => {
                mode = Some(false);
                continue;
            }
            "dark: {" => {
                mode = Some(true);
                continue;
            }
            "}," | "}" if mode.is_some() => {
                mode = None;
                continue;
            }
            _ => {}
        }
        // title:, type:, cssVars: and closing braces outside a mode map.
        let Some(dark) = mode else { continue };
        let entry = list.last_mut().ok_or_else(|| format!("themes.ts:{n}: var before any `name:`"))?;
        let (key, value) = t
            .strip_suffix(',')
            .and_then(|kv| kv.split_once(": "))
            .ok_or_else(|| format!("themes.ts:{n}: expected `key: \"value\",`: {t}"))?;
        let key = key.trim_matches('"');
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .ok_or_else(|| format!("themes.ts:{n}: value is not a string: {t}"))?;
        if key == "radius" {
            let rem = value.strip_suffix("rem").ok_or_else(|| format!("themes.ts:{n}: radius is not in rem: {value}"))?;
            entry.radius_rem = Some(num(rem).map_err(|e| format!("themes.ts:{n}: {e}"))?);
            continue;
        }
        let c = parse_oklch(value).map_err(|e| format!("themes.ts:{n}: {e}"))?;
        let vars = if dark { &mut entry.dark } else { &mut entry.light };
        vars.push((key.to_owned(), c));
    }
    let mut out = Themes::default();
    for id in &base_ids {
        if !all.iter().any(|e| &e.name == id) {
            return Err(format!("base-colors.ts names `{id}`, which themes.ts lacks"));
        }
    }
    for e in all {
        if base_ids.contains(&e.name) {
            out.bases.push(e);
        } else {
            out.overlays.push(e);
        }
    }
    Ok(out)
}

/// The vars of one mode as generated tuple lines; sidebar vars are skipped.
fn var_lines(theme: &str, mode: &str, vars: &[(String, Oklch)]) -> Result<String, String> {
    let mut s = String::new();
    for (name, c) in vars {
        if name.starts_with("sidebar") {
            continue;
        }
        let v = VARS.iter().find(|v| v.0 == name).ok_or_else(|| format!("{theme} {mode}: unknown var `{name}`"))?.1;
        s += &format!("        ({v}, O({:?}, {:?}, {:?}, {:?})),\n", c.l, c.c, c.h, c.alpha);
    }
    Ok(s)
}

/// `crates/shadcn-egui/src/palette/generated.rs`.
pub fn emit_palettes(pin: &str, themes: &Themes) -> Result<String, String> {
    let mut s = crate::emit::header_react(pin, "`apps/v4/registry/themes.ts` and `base-colors.ts`");
    s += "use super::{Oklch as O, Var, Var::*};\n\n";
    s += "/// The base colours (`base-colors.ts`): (id, radius in rem, light vars, dark vars). Every var is present.\n";
    s += "pub(crate) const BASES: &[(&str, f32, &[(Var, O)], &[(Var, O)])] = &[\n";
    for b in &themes.bases {
        let radius = b.radius_rem.ok_or_else(|| format!("base `{}` has no radius", b.name))?;
        for (mode, vars) in [("light", &b.light), ("dark", &b.dark)] {
            let missing: Vec<&str> = VARS.iter().map(|v| v.0).filter(|k| !vars.iter().any(|(n, _)| n == k)).collect();
            if !missing.is_empty() {
                return Err(format!("base `{}` {mode} lacks {}", b.name, missing.join(", ")));
            }
        }
        s += &format!(
            "    (\"{}\", {radius:?}, &[\n{}    ], &[\n{}    ]),\n",
            b.name,
            var_lines(&b.name, "light", &b.light)?,
            var_lines(&b.name, "dark", &b.dark)?
        );
    }
    s += "];\n\n";
    s += "/// The other `THEMES` entries, colour overlays: (id, light vars, dark vars).\n";
    s += "pub(crate) const OVERLAYS: &[(&str, &[(Var, O)], &[(Var, O)])] = &[\n";
    for o in &themes.overlays {
        s += &format!(
            "    (\"{}\", &[\n{}    ], &[\n{}    ]),\n",
            o.name,
            var_lines(&o.name, "light", &o.light)?,
            var_lines(&o.name, "dark", &o.dark)?
        );
    }
    s += "];\n";
    Ok(s)
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4`
Expected: all pass (the 7 new ones and every existing stylegen test).

- [ ] **Step 6: Clippy and commit**

```bash
cargo clippy --manifest-path tools/stylegen/Cargo.toml --all-targets --jobs 4 -- -D warnings
git add tools/stylegen
git commit -m "feat(stylegen): parse shadcn/ui themes.ts into palette constants

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3.2: `Palette.chart` and generated palettes in the crate

**Files:**
- Create: `tools/stylegen/src/sources.rs`
- Modify: `tools/stylegen/src/lib.rs` (add `pub mod sources;`), `tools/stylegen/src/main.rs` (`generate`)
- Modify: `tools/stylegen/tests/generated.rs` (palette file is current)
- Create: `tools/stylegen/tests/fixtures/{preset.ts,config.ts,font-definitions.ts}` (written by `generate`, from `$UP_REACT`)
- Create: `crates/shadcn-egui/src/palette/mod.rs`
- Create (generated): `crates/shadcn-egui/src/palette/generated.rs`
- Modify: `crates/shadcn-egui/src/theme.rs` (`Palette`), `crates/shadcn-egui/src/lib.rs` (`mod palette;`)
- Create: `crates/shadcn-egui/tests/palettes.rs`

**Interfaces:**
- Consumes: `stylegen::themes::{parse_themes, emit_palettes}` (Task 3.1).
- Produces:
  - `stylegen::sources::STYLES: &[&str]` (moved from `main.rs`), `pub enum Upstream { Svelte, React }`, `stylegen::sources::sources() -> Vec<(Upstream, String, String)>` (`(checkout, fixture file name, path inside that checkout)`), `stylegen::sources::refresh(up: &Path, up_react: &Path, tool: &Path) -> std::io::Result<()>`. Task 3.4 and Phase 4 rely on them.
  - CLI: `stylegen generate|drift <shadcn-svelte> <shadcn-ui> <repo-root>`; `main` checks `PIN` against the svelte checkout and `PIN_REACT` against the React one.
  - In the crate (index C): `Palette.chart: [Color32; 5]`; `Palette::from_upstream(base: &str, theme: &str, chart: &str, dark: bool) -> Option<Palette>`; `pub(crate) mod palette` with `Oklch(pub f32, pub f32, pub f32, pub f32)`, `enum Var` (23 variants), `pub(crate) fn merge(base, theme, chart, dark) -> Option<Palette>`; `generated::{BASES, OVERLAYS}`.
  - `palette::base_radius_px` is **not** added here: nothing would call it before Phase 4 and the lib build would warn (`dead_code`) under `-D warnings`. Phase 4 Task 5 adds it.

- [ ] **Step 1: Write the failing crate test**

`crates/shadcn-egui/tests/palettes.rs`:

```rust
use shadcn_egui::color::oklch_to_srgb;
use shadcn_egui::Palette;

#[test]
fn light_and_dark_are_the_generated_neutral_palette() {
    assert_eq!(Palette::from_upstream("neutral", "neutral", "neutral", false), Some(Palette::light()));
    assert_eq!(Palette::from_upstream("neutral", "neutral", "neutral", true), Some(Palette::dark()));
}

#[test]
fn neutral_charts_are_the_grey_ramp_in_both_modes() {
    let ramp = [0.87, 0.556, 0.439, 0.371, 0.269].map(|l| oklch_to_srgb(l, 0.0, 0.0));
    assert_eq!(Palette::light().chart, ramp);
    assert_eq!(Palette::dark().chart, ramp);
}

#[test]
fn destructive_foreground_is_the_near_white_upstream_dropped() {
    let p = Palette::from_upstream("stone", "blue", "rose", true).expect("known ids");
    assert_eq!(p.destructive_foreground, oklch_to_srgb(0.985, 0.0, 0.0));
}
```

- [ ] **Step 2: Write the failing stylegen test**

Append to `tools/stylegen/tests/generated.rs`:

```rust
/// `palette/generated.rs` is exactly what the generator gives for the fixture copy.
#[test]
fn committed_palettes_are_current() {
    let pin = read("PIN_REACT").trim().to_owned();
    let themes = stylegen::themes::parse_themes(&read("tests/fixtures/themes.ts"), &read("tests/fixtures/base-colors.ts")).expect("themes.ts");
    let fresh = stylegen::themes::emit_palettes(&pin, &themes).expect("emit");
    assert_eq!(fresh, read("../../crates/shadcn-egui/src/palette/generated.rs"), "re-run stylegen generate");
}
```

Run: `cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 --test generated -- --test-threads 4`
Expected: FAIL (`crates/shadcn-egui/src/palette/generated.rs` not found).

- [ ] **Step 3: `sources.rs` and `generate`**

`tools/stylegen/src/sources.rs`:

```rust
//! The upstream files stylegen reads. `generate` keeps a byte copy of each in
//! `tests/fixtures/`; tests read the copies, `drift` compares them with the
//! checkout.

use std::path::Path;

/// Every upstream style; each gets a report, the ones in `EXPORT` also Rust.
pub const STYLES: &[&str] = &["vega", "nova", "maia", "lyra", "mira", "luma", "sera", "rhea"];

/// The two upstream checkouts: shadcn-svelte (`PIN`, style CSS) and
/// shadcn/ui (`PIN_REACT`, preset lists, palettes, fonts).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Upstream {
    Svelte,
    React,
}

/// `(checkout, fixture file name, path inside that checkout)`.
pub fn sources() -> Vec<(Upstream, String, String)> {
    let mut v: Vec<(Upstream, String, String)> = [
        ("themes.ts", "apps/v4/registry/themes.ts"),
        ("base-colors.ts", "apps/v4/registry/base-colors.ts"),
        ("config.ts", "apps/v4/registry/config.ts"),
        ("preset.ts", "packages/registry/src/preset/preset.ts"),
        ("font-definitions.ts", "apps/v4/lib/font-definitions.ts"),
    ]
    .iter()
    .map(|(f, u)| (Upstream::React, f.to_string(), u.to_string()))
    .collect();
    v.extend(STYLES.iter().map(|s| (Upstream::Svelte, format!("style-{s}.css"), format!("docs/src/lib/registry/styles/style-{s}.css"))));
    v
}

/// Copy every source from its checkout (`up`: svelte, `up_react`: React)
/// into `<tool>/tests/fixtures/`.
pub fn refresh(up: &Path, up_react: &Path, tool: &Path) -> std::io::Result<()> {
    for (from, fixture, upstream) in sources() {
        let root = if from == Upstream::React { up_react } else { up };
        std::fs::copy(root.join(&upstream), tool.join("tests/fixtures").join(&fixture))?;
    }
    Ok(())
}
```

`tools/stylegen/src/lib.rs`: add `pub mod sources;`.

`tools/stylegen/src/main.rs`:
- delete the `STYLES` const and import it: `use stylegen::sources::STYLES;`, and add `themes` to the `use stylegen::{…}` list;
- the CLI takes the React checkout as a new second path. Module docs:
  `//! stylegen generate|drift <shadcn-svelte checkout> <shadcn-ui checkout> <repository root>`.
  In `main`: read `(cmd, up, up_react, root)` from `args.get(1..=4)` (usage
  line `"usage: stylegen generate|drift <shadcn-svelte> <shadcn-ui> <repo-root>"`),
  read `PIN_REACT` like `PIN` into `pin_react` and call
  `check_pin(&up_react, &pin_react)` after `check_pin(&up, &pin)`; pass
  `&up_react` and `&pin_react` to `generate` (new parameters after `up` and
  `pin`) and `&up_react` to `drift` (Task 3.4);
- add next to `write`:

```rust
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}
```

- in `generate`, as the first statement:

```rust
    stylegen::sources::refresh(up, up_react, tool).expect("copy the upstream sources into tests/fixtures");
```

- in the style loop, read the fixture copy instead of the checkout (same bytes):

```rust
        let parsed = css::parse_style_css(&read(&tool.join(format!("tests/fixtures/style-{style}.css"))));
```

  and delete the now unused `let path = up.join(…)` line;
- before `if failed {`:

```rust
    let palettes = themes::parse_themes(&read(&tool.join("tests/fixtures/themes.ts")), &read(&tool.join("tests/fixtures/base-colors.ts")))
        .and_then(|t| themes::emit_palettes(pin_react, &t));
    match palettes {
        Ok(src) => {
            let dir = root.join(CRATE).join("src/palette");
            std::fs::create_dir_all(&dir).expect("palette dir");
            write(&dir.join("generated.rs"), &src);
        }
        Err(e) => {
            eprintln!("palettes: {e}");
            failed = true;
        }
    }
```

(If the compiler reports `up` unused in `generate` after this change, it is
not: `refresh` uses it. `drift` uses `up_react` from Task 3.4 on.)

- [ ] **Step 4: Run `generate`**

```bash
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- generate "$UP" "$UP_REACT" .
git status --short
```

Expected: new `crates/shadcn-egui/src/palette/generated.rs`, new fixtures
`preset.ts`, `config.ts`, `font-definitions.ts`; `themes.ts`, `base-colors.ts` and every
`style-*.css` fixture, every `src/style/*.rs` and every report unchanged (if
any of them changed, the fixture copies were stale: stop and find out why
before going on).

- [ ] **Step 5: `palette/mod.rs`**

`crates/shadcn-egui/src/palette/mod.rs`:

```rust
//! shadcn base colours and colour themes, generated from shadcn/ui
//! `themes.ts` (`generated.rs`, by `tools/stylegen`), and the merge upstream
//! `buildRegistryTheme` applies to them.

mod generated;

use crate::color::oklch_to_srgb_a;
use crate::theme::Palette;
use egui::Color32;

/// An upstream `oklch(l c h / alpha)`; alpha 1.0 is opaque.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Oklch(pub f32, pub f32, pub f32, pub f32);

impl Oklch {
    fn color(self) -> Color32 {
        oklch_to_srgb_a(self.0, self.1, self.2, self.3)
    }
}

/// The upstream CSS vars a [`Palette`] holds. `sidebar-*` and `radius` are
/// not palette colours and are not generated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Var {
    Background,
    Foreground,
    Card,
    CardForeground,
    Popover,
    PopoverForeground,
    Primary,
    PrimaryForeground,
    Secondary,
    SecondaryForeground,
    Muted,
    MutedForeground,
    Accent,
    AccentForeground,
    Destructive,
    Border,
    Input,
    Ring,
    Chart1,
    Chart2,
    Chart3,
    Chart4,
    Chart5,
}

type Vars = &'static [(Var, Oklch)];

/// A base colour: (radius in rem, light vars, dark vars).
fn base(id: &str) -> Option<(f32, Vars, Vars)> {
    generated::BASES.iter().find(|b| b.0 == id).map(|b| (b.1, b.2, b.3))
}

/// A theme's vars: a base name overlays that whole base (upstream `THEMES`
/// holds the bases too), else a colour overlay.
fn theme_vars(id: &str, dark: bool) -> Option<Vars> {
    let pick = |light: Vars, d: Vars| if dark { d } else { light };
    base(id)
        .map(|(_, l, d)| pick(l, d))
        .or_else(|| generated::OVERLAYS.iter().find(|o| o.0 == id).map(|o| pick(o.1, o.2)))
}

/// Every field transparent except `destructive_foreground`, which upstream
/// dropped and the components still read.
fn blank() -> Palette {
    let t = Color32::TRANSPARENT;
    Palette {
        background: t,
        foreground: t,
        card: t,
        card_foreground: t,
        popover: t,
        popover_foreground: t,
        primary: t,
        primary_foreground: t,
        secondary: t,
        secondary_foreground: t,
        muted: t,
        muted_foreground: t,
        accent: t,
        accent_foreground: t,
        destructive: t,
        destructive_foreground: Oklch(0.985, 0.0, 0.0, 1.0).color(),
        border: t,
        input: t,
        ring: t,
        chart: [t; 5],
    }
}

fn set(p: &mut Palette, v: Var, c: Color32) {
    match v {
        Var::Background => p.background = c,
        Var::Foreground => p.foreground = c,
        Var::Card => p.card = c,
        Var::CardForeground => p.card_foreground = c,
        Var::Popover => p.popover = c,
        Var::PopoverForeground => p.popover_foreground = c,
        Var::Primary => p.primary = c,
        Var::PrimaryForeground => p.primary_foreground = c,
        Var::Secondary => p.secondary = c,
        Var::SecondaryForeground => p.secondary_foreground = c,
        Var::Muted => p.muted = c,
        Var::MutedForeground => p.muted_foreground = c,
        Var::Accent => p.accent = c,
        Var::AccentForeground => p.accent_foreground = c,
        Var::Destructive => p.destructive = c,
        Var::Border => p.border = c,
        Var::Input => p.input = c,
        Var::Ring => p.ring = c,
        Var::Chart1 => p.chart[0] = c,
        Var::Chart2 => p.chart[1] = c,
        Var::Chart3 => p.chart[2] = c,
        Var::Chart4 => p.chart[3] = c,
        Var::Chart5 => p.chart[4] = c,
    }
}

/// Upstream `buildRegistryTheme` (config.ts): base vars, then the theme on
/// top, then `chart-1..5` from the chart theme. `None` for an unknown id.
pub(crate) fn merge(base_id: &str, theme: &str, chart: &str, dark: bool) -> Option<Palette> {
    let (_, light_vars, dark_vars) = base(base_id)?;
    let theme = theme_vars(theme, dark)?;
    let chart = theme_vars(chart, dark)?;
    let mut p = blank();
    for &(v, c) in if dark { dark_vars } else { light_vars } {
        set(&mut p, v, c.color());
    }
    for &(v, c) in theme {
        set(&mut p, v, c.color());
    }
    for &(v, c) in chart {
        if matches!(v, Var::Chart1 | Var::Chart2 | Var::Chart3 | Var::Chart4 | Var::Chart5) {
            set(&mut p, v, c.color());
        }
    }
    Some(p)
}
```

`crates/shadcn-egui/src/lib.rs`: add `mod palette;` after `pub mod layout;`.

- [ ] **Step 6: `Palette` gains `chart` and `from_upstream`**

In `crates/shadcn-egui/src/theme.rs`, `Palette` gets a last field:

```rust
    /// `chart-1` … `chart-5`.
    pub chart: [Color32; 5],
```

`Palette::light()` and `Palette::dark()` both end with the neutral grey ramp
(upstream neutral uses the same five values in both modes):

```rust
            chart: [c(0.87, 0.0, 0.0), c(0.556, 0.0, 0.0), c(0.439, 0.0, 0.0), c(0.371, 0.0, 0.0), c(0.269, 0.0, 0.0)],
```

Add to the second `impl Palette` block, before `with_bold_menu_accent`:

```rust
    /// The palette the shadcn site builds for a base colour, a theme and a
    /// chart colour (upstream `buildRegistryTheme`): the `base` colour's
    /// vars, the `theme` on top (a colour theme, or a base name, which
    /// overlays that whole base), then `chart-1..5` from the `chart` theme.
    /// Ids are upstream's (`"neutral"`, `"blue"`); `None` when one is
    /// unknown. Menu accent is separate: [`Palette::with_bold_menu_accent`].
    pub fn from_upstream(base: &str, theme: &str, chart: &str, dark: bool) -> Option<Palette> {
        crate::palette::merge(base, theme, chart, dark)
    }
```

- [ ] **Step 7: Run the tests**

```bash
cargo test -p shadcn-egui --jobs 4 --test palettes -- --test-threads 4
cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4
cargo test --workspace --jobs 4 -- --test-threads 4
git status --short crates/shadcn-egui/tests/snapshots   # empty: no PNG changed
```

Expected: all pass; no snapshot changed (the palette values are the same
literals as before).

- [ ] **Step 8: Clippy and commit**

```bash
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
cargo clippy --manifest-path tools/stylegen/Cargo.toml --all-targets --jobs 4 -- -D warnings
git add -A
git commit -m "feat: generated shadcn palettes and chart colours

stylegen generate now keeps byte copies of every upstream source it reads
in tests/fixtures (palettes, preset lists and fonts from a second pinned
checkout, shadcn/ui; style CSS from shadcn-svelte) and writes
palette/generated.rs from themes.ts. Palette
gains chart colours and from_upstream(base, theme, chart, dark), which
merges like upstream buildRegistryTheme.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3.3: Upstream fixtures for `from_upstream`

**Files:**
- Create: `tools/stylegen/upstream/registry-themes.mjs`
- Create (generated by the script): `crates/shadcn-egui/tests/fixtures/registry_themes.txt`
- Modify: `crates/shadcn-egui/tests/palettes.rs`

**Interfaces:**
- Consumes: `Palette::from_upstream`, `Palette::with_bold_menu_accent` (Task 3.2); `shadcn_egui::color::oklch_to_srgb_a`.
- Produces: fixture `registry_themes.txt`, one line per var:
  `<base> <theme> <chart> <subtle|bold> <light|dark> <var> <value…>` (value
  is the rest of the line, for example `oklch(1 0 0 / 10%)`); `#` lines are comments.
  The script follows the index A convention: arguments
  `<shadcn-ui checkout> <scratch dir>`, output on stdout.

- [ ] **Step 1: Write the script**

`tools/stylegen/upstream/registry-themes.mjs`:

```js
// Runs upstream buildRegistryTheme (shadcn/ui apps/v4/registry/config.ts) on a
// fixed set of base colour / theme / chart colour / menu accent cases and
// prints every resulting CSS var, for crates/shadcn-egui/tests/palettes.rs.
//
// Usage: node tools/stylegen/upstream/registry-themes.mjs <shadcn-ui checkout> <scratch dir> \
//          > crates/shadcn-egui/tests/fixtures/registry_themes.txt
// Needs node >= 22.18 (TypeScript type stripping on by default).
// Only RADII, getTheme, getBaseColor and buildRegistryTheme are sliced out of
// config.ts: the rest of that file imports packages that are not installed.
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const [upReact, scratch] = process.argv.slice(2);
if (!upReact || !scratch) {
  console.error("usage: registry-themes.mjs <shadcn-ui checkout> <scratch dir>");
  process.exit(2);
}
mkdirSync(scratch, { recursive: true });

/** From `marker` to the bracket closing the first `[` or `{` after the marker. */
function slice(text, marker) {
  const at = text.indexOf(marker);
  if (at < 0) throw new Error(`not found upstream: ${marker}`);
  const from = at + marker.length;
  const open = from + text.slice(from).search(/[[{]/);
  let depth = 0;
  for (let i = open; i < text.length; i++) {
    const ch = text[i];
    if (ch === "[" || ch === "{") depth++;
    else if (ch === "]" || ch === "}") {
      depth--;
      if (depth === 0) return text.slice(at, i + 1);
    }
  }
  throw new Error(`unbalanced: ${marker}`);
}

const pin = execFileSync("git", ["-C", upReact, "rev-parse", "HEAD"]).toString().trim().slice(0, 8);
const reg = join(upReact, "apps/v4/registry");
// The only import of themes.ts is a type from an uninstalled package; type
// stripping drops the `: RegistryItem[]` / `satisfies` annotations.
writeFileSync(join(scratch, "themes.ts"), readFileSync(join(reg, "themes.ts"), "utf8").replace(/^import .*$/m, ""));
const baseColors = readFileSync(join(reg, "base-colors.ts"), "utf8")
  .replace(/^import \{ THEMES \} from .*$/m, 'import { THEMES } from "./themes.ts";');
if (/from "@\//.test(baseColors)) throw new Error("an upstream import was not rewritten");
writeFileSync(join(scratch, "base-colors.ts"), baseColors);
const config = readFileSync(join(reg, "config.ts"), "utf8");
writeFileSync(
  join(scratch, "registry.ts"),
  [
    'import { THEMES } from "./themes.ts";',
    'import { BASE_COLORS } from "./base-colors.ts";',
    "type DesignSystemConfig = any;",
    slice(config, "export const RADII ="),
    // The `(` keeps getTheme from matching getThemesForBaseColor.
    slice(config, "export function getTheme("),
    slice(config, "export function getBaseColor("),
    slice(config, "export function buildRegistryTheme("),
    "export { BASE_COLORS, THEMES };",
    "",
  ].join("\n"),
);
const { buildRegistryTheme, BASE_COLORS, THEMES } = await import(pathToFileURL(join(scratch, "registry.ts")).href);

const baseIds = BASE_COLORS.map((t) => t.name);
const themeIds = THEMES.map((t) => t.name);
const cases = [
  // Every theme on neutral, charts from the same theme.
  ...themeIds.map((t) => ["neutral", t, t, "subtle"]),
  // Every base with itself.
  ...baseIds.map((b) => [b, b, b, "subtle"]),
  // Crosses: theme, chart colour and accent from different sources.
  ["stone", "blue", "rose", "bold"],
  ["zinc", "amber", "neutral", "subtle"],
  ["mauve", "violet", "emerald", "bold"],
  ["olive", "green", "lime", "subtle"],
  ["mist", "sky", "teal", "bold"],
  ["taupe", "orange", "red", "subtle"],
  ["neutral", "zinc", "purple", "bold"],
  ["stone", "mist", "fuchsia", "subtle"],
  ["zinc", "indigo", "yellow", "bold"],
  ["taupe", "pink", "cyan", "bold"],
];

const out = [
  `# @generated by tools/stylegen/upstream/registry-themes.mjs from shadcn/ui ${pin}; do not edit.`,
  "# <base> <theme> <chart> <subtle|bold> <light|dark> <var> <value>",
];
for (const [baseColor, theme, chartColor, menuAccent] of cases) {
  const r = buildRegistryTheme({
    style: "vega",
    baseColor,
    theme,
    chartColor,
    iconLibrary: "lucide",
    font: "inter",
    menuAccent,
    menuColor: "default",
    radius: "default",
  });
  for (const mode of ["light", "dark"]) {
    for (const [k, v] of Object.entries(r.cssVars[mode])) {
      out.push(`${baseColor} ${theme} ${chartColor} ${menuAccent} ${mode} ${k} ${v}`);
    }
  }
}
process.stdout.write(out.join("\n") + "\n");
```

- [ ] **Step 2: Produce the fixture**

```bash
mkdir -p crates/shadcn-egui/tests/fixtures
node tools/stylegen/upstream/registry-themes.mjs "$UP_REACT" "$SCRATCH/registry-themes" > crates/shadcn-egui/tests/fixtures/registry_themes.txt
grep -vc '^#' crates/shadcn-egui/tests/fixtures/registry_themes.txt
grep -c ' bold ' crates/shadcn-egui/tests/fixtures/registry_themes.txt
```

Expected: more than 2500 lines (41 cases × 2 modes × 23 colour vars, plus
sidebar vars and the light `radius`); the bold count is non-zero. Run the
script a second time into a scratch file and `cmp` it with the first: it
must be identical.

- [ ] **Step 3: Write the comparison test**

Append to `crates/shadcn-egui/tests/palettes.rs`:

```rust
use egui::Color32;
use shadcn_egui::color::oklch_to_srgb_a;

/// `oklch(L C H)` / `oklch(L C H / A%)`, as upstream writes them.
fn oklch(s: &str) -> Color32 {
    let inner = s.strip_prefix("oklch(").and_then(|r| r.strip_suffix(')')).unwrap_or_else(|| panic!("not oklch: {s}"));
    let (lch, alpha) = match inner.split_once('/') {
        Some((lch, a)) => {
            let a = a.trim();
            (lch, a.strip_suffix('%').map_or_else(|| a.parse::<f32>().unwrap(), |p| p.parse::<f32>().unwrap() / 100.0))
        }
        None => (inner, 1.0),
    };
    let v: Vec<f32> = lch.split_whitespace().map(|n| n.parse().unwrap()).collect();
    oklch_to_srgb_a(v[0], v[1], v[2], alpha)
}

/// Our field for an upstream var; `None` for vars the palette does not hold.
fn field(p: &Palette, var: &str) -> Option<Color32> {
    Some(match var {
        "background" => p.background,
        "foreground" => p.foreground,
        "card" => p.card,
        "card-foreground" => p.card_foreground,
        "popover" => p.popover,
        "popover-foreground" => p.popover_foreground,
        "primary" => p.primary,
        "primary-foreground" => p.primary_foreground,
        "secondary" => p.secondary,
        "secondary-foreground" => p.secondary_foreground,
        "muted" => p.muted,
        "muted-foreground" => p.muted_foreground,
        "accent" => p.accent,
        "accent-foreground" => p.accent_foreground,
        "destructive" => p.destructive,
        "border" => p.border,
        "input" => p.input,
        "ring" => p.ring,
        "chart-1" => p.chart[0],
        "chart-2" => p.chart[1],
        "chart-3" => p.chart[2],
        "chart-4" => p.chart[3],
        "chart-5" => p.chart[4],
        v if v == "radius" || v.starts_with("sidebar") => return None,
        other => panic!("upstream var `{other}` has no Palette field"),
    })
}

#[test]
fn merged_palettes_equal_upstream_build_registry_theme() {
    let text = include_str!("fixtures/registry_themes.txt");
    let mut checked = 0;
    for line in text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let w: Vec<&str> = line.splitn(7, ' ').collect();
        let [base, theme, chart, accent, mode, var, value] = w[..] else { panic!("bad line: {line}") };
        let mut p = Palette::from_upstream(base, theme, chart, mode == "dark").unwrap_or_else(|| panic!("{line}: unknown id"));
        if accent == "bold" {
            p = p.with_bold_menu_accent();
        }
        if let Some(ours) = field(&p, var) {
            assert_eq!(ours, oklch(value), "{line}");
            checked += 1;
        }
    }
    assert_eq!(checked, 41 * 2 * 23, "41 cases x 2 modes x 23 colour vars");
}

#[test]
fn unknown_ids_give_none() {
    assert_eq!(Palette::from_upstream("blue", "neutral", "neutral", false), None, "blue is not a base colour");
    assert_eq!(Palette::from_upstream("neutral", "nope", "neutral", false), None);
    assert_eq!(Palette::from_upstream("neutral", "neutral", "nope", true), None);
    assert_eq!(Palette::from_upstream("", "", "", true), None);
    // `gray` is a preset code value on ui.shadcn.com without a palette (index §J).
    assert_eq!(Palette::from_upstream("gray", "neutral", "neutral", false), None);
    assert_eq!(Palette::from_upstream("neutral", "gray", "neutral", false), None);
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p shadcn-egui --jobs 4 --test palettes -- --test-threads 4`
Expected: all pass. If `merged_palettes_equal_upstream_build_registry_theme`
fails on a value, the line it prints names the case: fix the merge or the
generator, never the fixture.

- [ ] **Step 5: Commit**

```bash
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
git add tools/stylegen/upstream/registry-themes.mjs crates/shadcn-egui/tests/fixtures/registry_themes.txt crates/shadcn-egui/tests/palettes.rs
git commit -m "test: merged palettes equal upstream buildRegistryTheme output

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3.4: `drift` covers the upstream sources; changelog

**Files:**
- Modify: `tools/stylegen/src/drift.rs` (add `check_sources`)
- Modify: `tools/stylegen/src/main.rs` (`drift` reports it)
- Modify: `tools/stylegen/tests/drift.rs`
- Modify: `CHANGES.md`

**Interfaces:**
- Consumes: `stylegen::sources::sources()` (Task 3.2).
- Produces: `stylegen::drift::check_sources(up: &Path, up_react: &Path, tool: &Path) -> Vec<String>`: one line per fixture copy that is missing or differs from its checkout (`up`: svelte, `up_react`: React), each ending in `re-run stylegen generate`; empty means clean.

- [ ] **Step 1: Write the failing test**

Append to `tools/stylegen/tests/drift.rs`:

```rust
#[test]
fn reports_fixture_copies_that_differ_from_upstream() {
    use stylegen::drift::check_sources;
    use stylegen::sources::{sources, Upstream};
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("stylegen-sources-{}", std::process::id()));
    let (up, up_react, tool) = (dir.join("up"), dir.join("up-react"), dir.join("tool"));
    for (from, fixture, upstream) in sources() {
        let src = if from == Upstream::React { up_react.join(&upstream) } else { up.join(&upstream) };
        std::fs::create_dir_all(src.parent().unwrap()).unwrap();
        std::fs::write(&src, format!("upstream {fixture}")).unwrap();
        std::fs::create_dir_all(tool.join("tests/fixtures")).unwrap();
        std::fs::write(tool.join("tests/fixtures").join(&fixture), format!("upstream {fixture}")).unwrap();
    }
    assert!(check_sources(&up, &up_react, &tool).is_empty(), "identical copies are clean");

    std::fs::write(up_react.join("apps/v4/registry/themes.ts"), "changed upstream").unwrap();
    std::fs::remove_file(tool.join("tests/fixtures/preset.ts")).unwrap();
    std::fs::write(up.join("docs/src/lib/registry/styles/style-vega.css"), "changed upstream").unwrap();
    let problems = check_sources(&up, &up_react, &tool);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(problems.len(), 3, "{problems:#?}");
    assert!(problems.iter().any(|p| p.contains("themes.ts") && p.ends_with("re-run stylegen generate")));
    assert!(problems.iter().any(|p| p.contains("preset.ts") && p.ends_with("re-run stylegen generate")));
    assert!(problems.iter().any(|p| p.contains("style-vega.css") && p.ends_with("re-run stylegen generate")));
}
```

Run: `cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 --test drift -- --test-threads 4`
Expected: FAIL to compile (`check_sources` not found).

- [ ] **Step 2: Implement**

Append to `tools/stylegen/src/drift.rs`:

```rust
/// Fixture copies (`<tool>/tests/fixtures/`) that no longer match the
/// upstream checkout: tests and generated code would describe stale sources.
pub fn check_sources(up: &Path, up_react: &Path, tool: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    for (from, fixture, upstream) in crate::sources::sources() {
        let root = if from == crate::sources::Upstream::React { up_react } else { up };
        let ours = std::fs::read(tool.join("tests/fixtures").join(&fixture));
        let theirs = std::fs::read(root.join(&upstream));
        match (ours, theirs) {
            (Ok(a), Ok(b)) if a == b => {}
            (_, Err(e)) => problems.push(format!("{upstream}: cannot read the upstream file ({e}); re-run stylegen generate")),
            (Err(_), _) => problems.push(format!("tests/fixtures/{fixture} is missing; re-run stylegen generate")),
            _ => problems.push(format!("tests/fixtures/{fixture} differs from {upstream}; re-run stylegen generate")),
        }
    }
    problems
}
```

`tools/stylegen/src/main.rs`: the `"drift"` arm becomes
`"drift" => drift(&up, &up_react, &root.join(CRATE), &tool, &lines),` and `drift` reads:

```rust
fn drift(up: &Path, up_react: &Path, crate_root: &Path, tool: &Path, lines: &[markers::MarkerLine]) -> ExitCode {
    let mut problems = stylegen::drift::check(up, crate_root, lines);
    problems.extend(stylegen::drift::check_sources(up, up_react, tool));
    for p in &problems {
        println!("{p}");
    }
    if problems.is_empty() {
        println!("drift: clean");
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
```

- [ ] **Step 3: Run the tests and drift**

```bash
cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift "$UP" "$UP_REACT" .
```

Expected: all pass; drift prints `drift: clean`.

- [ ] **Step 4: Changelog**

`CHANGES.md`, under `## [Unreleased]` → `### Added`:

```markdown
- `Palette::from_upstream(base, theme, chart, dark)` gives the palette the
  shadcn site builds for a base colour, a colour theme and a chart colour,
  for example `Palette::from_upstream("stone", "blue", "rose", true)`.
- `Palette` has the five chart colours (`chart-1` … `chart-5`) as `chart`.
```

and under `### Changed`:

```markdown
- A `Palette { .. }` struct literal written for 0.3 no longer compiles,
  because `Palette` has the new field `chart`; start from `Palette::light()`,
  `Palette::dark()` or `Palette::from_upstream(..)` and change single fields.
```

- [ ] **Step 5: Phase gates and commit**

Run every gate from the index (both feature sets, clippy, tools, drift).

```bash
git add -A
git commit -m "feat(stylegen): drift reports stale fixture copies of upstream sources

drift now also compares the fixture copies of themes.ts, base-colors.ts,
config.ts, preset.ts, font-definitions.ts (shadcn/ui pin) and the style
CSS (shadcn-svelte pin) with the pinned checkouts.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
