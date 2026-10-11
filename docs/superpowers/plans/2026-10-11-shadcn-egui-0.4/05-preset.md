# Phase 4 — `Preset`: enums, codes, resolution to a `Theme`, fallbacks

> Part of [00-index.md](00-index.md). Read its Global Constraints first.

**Spec:** D5, D13, D20, §4, §4.1, §6.2 (types), §7 (preset codes,
fallbacks, Lyra radius).
**Cites:** `docs/superpowers/notes/0.4/p0a-preset-codes.md` (decision for
`a` codes), `tools/stylegen/upstream/preset-codes.txt` (codes from both sites).
**Index decisions applied:** G1 (icon library order from `libraries.ts`),
G2 (Rhea + Large is **not** special-cased), index C (all names, the
`Preset::theme` steps and the warning lines).

**Outcome:** stylegen generates the preset enums from `preset.ts` and
`libraries.ts`; `Preset::from_code` / `to_code` port upstream
`decodePreset` / `encodePreset` and are checked against fixtures from
upstream's own code; `Theme` gains `icons: IconSlots` and `menu: MenuColors`;
the three components read their icons from the theme; `Preset::theme(mode)`
resolves every field with the §4.1 fallbacks and logs one warning per
fallback.

Rendering does not change in this phase: every snapshot must stay
byte-identical (checked in Task 4.4).

**Deviations from the index, and why**

- `Preset`, `FontHeading` and `PresetCodeError` are created in Task 4.2,
  not 4.3: `from_code` needs a struct to decode into. Task 4.3 adds
  `Default`, the setters and `Mode`.
- `palette::base_radius_px` is added in Task 4.5 (its first caller), not
  Phase 3: an uncalled `pub(crate)` fn fails `clippy -D warnings`.
- The fallback tests guard with a runtime check
  (`fonts::files(FontName::Geist).is_none()`, `icons::slots(IconLibrary::Tabler).is_none()`)
  instead of `#[cfg(not(feature = "font-geist"))]` / `"icons-tabler"`: those
  features do not exist before Phases 6/7, and naming an undeclared feature
  in `cfg` raises `unexpected_cfgs`, which `-D warnings` turns into an error.
  Under `--all-features` (once the features exist) the tests print
  `skipped: …` and return, which is what the `cfg` would have done.

---

### Task 4.1: Preset enums generated from `preset.ts` and `libraries.ts`

**Files:**
- Create: `tools/stylegen/src/preset.rs`, `tools/stylegen/tests/preset.rs`
- Modify: `tools/stylegen/src/lib.rs` (`pub mod preset;`), `tools/stylegen/src/main.rs` (`generate`), `tools/stylegen/tests/generated.rs`
- Create: `crates/shadcn-egui/src/preset/mod.rs`
- Create (generated): `crates/shadcn-egui/src/preset/generated.rs`
- Modify: `crates/shadcn-egui/src/lib.rs`
- Create: `crates/shadcn-egui/tests/preset.rs`

**Interfaces:**
- Consumes: fixture copies `tools/stylegen/tests/fixtures/{preset.ts,libraries.ts}` (Phase 3 Task 2); `stylegen::emit::header` (`pub(crate)` since Phase 3).
- Produces:
  - NEW INTERFACE: `stylegen::preset::PresetLists { styles, base_colors, themes, fonts: Vec<String>, radii: Vec<(String, f32)>, menu_accents, menu_colors, icon_libraries: Vec<String>, fields_v1, fields_v2: Vec<(String, u32)> }`, `parse_preset(preset_ts: &str, libraries_ts: &str) -> Result<PresetLists, String>`, `emit_preset(pin: &str, lists: &PresetLists) -> String`.
  - In the crate, exactly as index C: `StyleName`, `BaseColor`, `ThemeColor`, `FontName`, `IconLibrary`, `MenuColor`, `MenuAccent` (each `Clone, Copy, Debug, PartialEq, Eq, Hash`; `pub const ALL: [Self; N]`; `pub const fn id(self) -> &'static str`; `pub fn from_id(&str) -> Option<Self>`), `RadiusChoice` (`Clone, Copy, Debug, PartialEq`; `Default, None, Small, Medium, Large, Px(f32)`; `ALL: [Self; 5]`; `id(self) -> Option<&'static str>`; `rem(self) -> Option<f32>`; `from_id`), `pub(crate) enum Field`, `pub(crate) const FIELDS_V1: [(Field, u32); 8]`, `FIELDS_V2: [(Field, u32); 10]`. All public enums re-exported at the crate root.

- [ ] **Step 1: Write the failing stylegen tests**

`tools/stylegen/tests/preset.rs`:

```rust
use stylegen::preset::{emit_preset, parse_preset};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn read(p: &str) -> String {
    std::fs::read_to_string(format!("{ROOT}/tests/fixtures/{p}")).expect(p)
}

#[test]
fn lists_follow_upstream_order() {
    let l = parse_preset(&read("preset.ts"), &read("libraries.ts")).expect("parse");
    assert_eq!(l.styles, ["nova", "vega", "maia", "lyra", "mira", "luma", "sera", "rhea"]);
    assert_eq!(l.base_colors, ["neutral", "stone", "zinc", "mauve", "olive", "mist", "taupe"]);
    assert_eq!(l.themes.len(), 24);
    assert_eq!((l.themes[0].as_str(), l.themes[3].as_str(), l.themes[23].as_str()), ("neutral", "amber", "taupe"));
    assert_eq!(l.fonts.len(), 26);
    assert_eq!((l.fonts[0].as_str(), l.fonts[17].as_str(), l.fonts[25].as_str()), ("inter", "oxanium", "instrument-serif"));
    let radii: Vec<(&str, f32)> = l.radii.iter().map(|(n, r)| (n.as_str(), *r)).collect();
    assert_eq!(radii, [("default", 0.5), ("none", 0.0), ("small", 0.45), ("medium", 0.625), ("large", 0.875)]);
    assert_eq!(l.menu_accents, ["subtle", "bold"]);
    assert_eq!(l.menu_colors, ["default", "inverted", "default-translucent", "inverted-translucent"]);
    assert_eq!(l.icon_libraries, ["lucide", "tabler", "hugeicons", "phosphor", "remixicon"]);
    assert_eq!(l.fields_v1.iter().map(|f| f.1).sum::<u32>(), 40);
    assert_eq!(l.fields_v2.iter().map(|f| f.1).sum::<u32>(), 51);
    assert_eq!(l.fields_v2[0], ("menuColor".to_string(), 3));
    assert_eq!(l.fields_v2[8], ("chartColor".to_string(), 6));
    assert_eq!(l.fields_v2[9], ("fontHeading".to_string(), 5));
}

#[test]
fn a_list_too_long_for_its_bits_is_an_error() {
    let src = read("preset.ts").replace("values: PRESET_MENU_COLORS, bits: 3 }", "values: PRESET_MENU_COLORS, bits: 1 }");
    let err = parse_preset(&src, &read("libraries.ts")).unwrap_err();
    assert!(err.contains("menuColor"), "{err}");
}

#[test]
fn an_unknown_values_list_is_an_error() {
    let src = read("preset.ts").replace("values: PRESET_STYLES,", "values: PRESET_SHAPES,");
    assert!(parse_preset(&src, &read("libraries.ts")).unwrap_err().contains("PRESET_SHAPES"));
}

#[test]
fn emit_is_deterministic() {
    let l = parse_preset(&read("preset.ts"), &read("libraries.ts")).expect("parse");
    let a = emit_preset("493481fa", &l);
    assert_eq!(a, emit_preset("493481fa", &l));
    assert!(a.contains("pub enum FontName {"));
    assert!(a.contains("Self::SourceSans3 => \"source-sans-3\","));
    assert!(a.contains("pub const ALL: [Self; 26] = ["));
    assert!(a.contains("Px(f32),"));
    assert!(a.contains("pub(crate) const FIELDS_V2: [(Field, u32); 10] = ["));
}
```

Append to `tools/stylegen/tests/generated.rs`:

```rust
#[test]
fn committed_preset_enums_are_current() {
    let pin = read("PIN").trim().to_owned();
    let lists = stylegen::preset::parse_preset(&read("tests/fixtures/preset.ts"), &read("tests/fixtures/libraries.ts")).expect("parse");
    assert_eq!(stylegen::preset::emit_preset(&pin, &lists), read("../../crates/shadcn-egui/src/preset/generated.rs"), "re-run stylegen generate");
}
```

Run: `cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4`
Expected: FAIL to compile (`stylegen::preset` not found).

- [ ] **Step 2: Implement the parser and emitter**

`tools/stylegen/src/preset.rs`:

```rust
//! `packages/cli/src/preset/preset.ts` + `packages/cli/src/icons/libraries.ts`
//! → `crates/shadcn-egui/src/preset/generated.rs`. Enum order is the
//! upstream list order: it is the index inside a preset code.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PresetLists {
    pub styles: Vec<String>,
    pub base_colors: Vec<String>,
    pub themes: Vec<String>,
    pub fonts: Vec<String>,
    /// `PRESET_RADII`: (key, value in rem).
    pub radii: Vec<(String, f32)>,
    pub menu_accents: Vec<String>,
    pub menu_colors: Vec<String>,
    /// `keys(iconLibraries)` in `libraries.ts` (index G1).
    pub icon_libraries: Vec<String>,
    /// `(key, bits)`, low bits first.
    pub fields_v1: Vec<(String, u32)>,
    pub fields_v2: Vec<(String, u32)>,
}

/// From `marker` to the bracket closing the first `[` or `{` after it.
fn block<'a>(src: &'a str, marker: &str) -> Result<&'a str, String> {
    let at = src.find(marker).ok_or_else(|| format!("`{marker}` not found"))?;
    let rest = &src[at..];
    let open = marker.len() + rest[marker.len()..].find(['[', '{']).ok_or_else(|| format!("`{marker}`: no bracket"))?;
    let mut depth = 0usize;
    for (i, ch) in rest[open..].char_indices() {
        match ch {
            '[' | '{' => depth += 1,
            ']' | '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(&rest[..open + i + 1]);
                }
            }
            _ => {}
        }
    }
    Err(format!("`{marker}`: unbalanced brackets"))
}

/// Every `"…"` literal in the block after `marker`.
fn strings(src: &str, marker: &str) -> Result<Vec<String>, String> {
    let b = block(src, marker)?;
    let body = &b[marker.len()..];
    Ok(body.split('"').skip(1).step_by(2).map(str::to_owned).collect())
}

/// Keys of an object literal: lines indented by exactly one tab.
fn keys(src: &str, marker: &str) -> Result<Vec<String>, String> {
    Ok(block(src, marker)?
        .lines()
        .filter_map(|l| l.strip_prefix('\t'))
        .filter(|l| !l.starts_with(char::is_whitespace))
        .filter_map(|l| l.split_once(':').map(|(k, _)| k.trim_matches('"').to_owned()))
        .collect())
}

fn between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = s.find(start)? + start.len();
    Some(&s[i..i + s[i..].find(end)?])
}

fn fields(src: &str, marker: &str) -> Result<Vec<(String, String, u32)>, String> {
    let mut out = Vec::new();
    for line in block(src, marker)?.lines().filter(|l| l.contains("key: \"")) {
        let bad = || format!("{marker}: cannot read `{}`", line.trim());
        let key = between(line, "key: \"", "\"").ok_or_else(bad)?;
        let values = between(line, "values: ", ",").ok_or_else(bad)?;
        let bits = between(line, "bits: ", " ").ok_or_else(bad)?.parse::<u32>().map_err(|_| bad())?;
        out.push((key.to_owned(), values.to_owned(), bits));
    }
    Ok(out)
}

/// Each field key and the list upstream packs for it.
const FIELD_VALUES: [(&str, &str); 10] = [
    ("menuColor", "PRESET_MENU_COLORS"),
    ("menuAccent", "PRESET_MENU_ACCENTS"),
    ("radius", "PRESET_RADII_KEYS"),
    ("font", "PRESET_FONTS"),
    ("iconLibrary", "PRESET_ICON_LIBRARIES"),
    ("theme", "PRESET_THEME_KEYS"),
    ("baseColor", "PRESET_BASE_COLOR_KEYS"),
    ("style", "PRESET_STYLES"),
    ("chartColor", "PRESET_CHART_COLORS"),
    ("fontHeading", "PRESET_FONT_HEADINGS"),
];

fn require(src: &str, text: &str) -> Result<(), String> {
    src.contains(text).then_some(()).ok_or_else(|| format!("preset.ts no longer contains `{text}`"))
}

pub fn parse_preset(preset_ts: &str, libraries_ts: &str) -> Result<PresetLists, String> {
    // The derived lists the generated code relies on.
    require(preset_ts, "export const PRESET_FONT_HEADINGS = [\"inherit\", ...PRESET_FONTS] as const;")?;
    require(preset_ts, "export const PRESET_CHART_COLORS = PRESET_THEME_KEYS;")?;
    require(preset_ts, "export const PRESET_ICON_LIBRARIES = keys(iconLibraries);")?;
    let radii_block = block(preset_ts, "export const PRESET_RADII =")?;
    let mut radii = Vec::new();
    for line in radii_block.lines().filter(|l| l.contains("value: \"")) {
        let key = line.trim().split_once(':').map(|(k, _)| k.to_owned()).ok_or("PRESET_RADII: no key")?;
        let rem = between(line, "value: \"", "rem\"").ok_or_else(|| format!("PRESET_RADII: no rem value in `{}`", line.trim()))?;
        radii.push((key, rem.parse::<f32>().map_err(|e| format!("PRESET_RADII {rem}: {e}"))?));
    }
    let mut l = PresetLists {
        styles: strings(preset_ts, "export const PRESET_STYLES =")?,
        base_colors: keys(preset_ts, "export const PRESET_BASE_COLORS =")?,
        themes: keys(preset_ts, "export const PRESET_THEMES =")?,
        fonts: strings(preset_ts, "export const PRESET_FONTS =")?,
        radii,
        menu_accents: strings(preset_ts, "export const PRESET_MENU_ACCENTS =")?,
        menu_colors: strings(preset_ts, "export const PRESET_MENU_COLORS =")?,
        icon_libraries: keys(libraries_ts, "export const iconLibraries =")?,
        ..PresetLists::default()
    };
    let v1 = fields(preset_ts, "const PRESET_FIELDS_V1 =")?;
    let mut v2 = v1.clone();
    if !block(preset_ts, "const PRESET_FIELDS_V2 =")?.contains("...PRESET_FIELDS_V1,") {
        return Err("PRESET_FIELDS_V2 no longer starts with ...PRESET_FIELDS_V1".into());
    }
    v2.extend(fields(preset_ts, "const PRESET_FIELDS_V2 =")?);
    let expected: Vec<&str> = FIELD_VALUES.iter().map(|f| f.0).collect();
    let got: Vec<&str> = v2.iter().map(|f| f.0.as_str()).collect();
    if got != expected || v1.len() != 8 {
        return Err(format!("PRESET_FIELDS changed: {got:?}"));
    }
    for (key, values, bits) in &v2 {
        let want = FIELD_VALUES.iter().find(|f| f.0 == key).map(|f| f.1).unwrap_or_default();
        if values != want {
            return Err(format!("field {key} packs {values}, expected {want}"));
        }
        let len = match key.as_str() {
            "menuColor" => l.menu_colors.len(),
            "menuAccent" => l.menu_accents.len(),
            "radius" => l.radii.len(),
            "font" => l.fonts.len(),
            "iconLibrary" => l.icon_libraries.len(),
            "theme" | "chartColor" => l.themes.len(),
            "baseColor" => l.base_colors.len(),
            "style" => l.styles.len(),
            _ => l.fonts.len() + 1, // fontHeading: "inherit" + fonts
        };
        if len > 1 << bits {
            return Err(format!("field {key}: {len} values do not fit in {bits} bits"));
        }
    }
    l.fields_v1 = v1.into_iter().map(|(k, _, b)| (k, b)).collect();
    l.fields_v2 = v2.into_iter().map(|(k, _, b)| (k, b)).collect();
    Ok(l)
}

/// `jetbrains-mono` → `JetbrainsMono`, `menuColor` → `MenuColor`.
fn pascal(id: &str) -> String {
    id.split('-')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
        })
        .collect()
}

fn emit_enum(s: &mut String, name: &str, doc: &str, ids: &[String]) {
    *s += &format!("/// {doc}\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum {name} {{\n");
    for id in ids {
        *s += &format!("    /// `\"{id}\"`\n    {},\n", pascal(id));
    }
    *s += &format!("}}\n\nimpl {name} {{\n    /// Every value, in upstream list order (the index in a preset code).\n");
    *s += &format!("    pub const ALL: [Self; {}] = [{}];\n\n", ids.len(), ids.iter().map(|i| format!("Self::{}", pascal(i))).collect::<Vec<_>>().join(", "));
    *s += "    /// The upstream id.\n    pub const fn id(self) -> &'static str {\n        match self {\n";
    for id in ids {
        *s += &format!("            Self::{} => \"{id}\",\n", pascal(id));
    }
    *s += "        }\n    }\n\n    /// The value with this upstream id.\n    pub fn from_id(id: &str) -> Option<Self> {\n";
    *s += "        Self::ALL.into_iter().find(|v| v.id() == id)\n    }\n}\n\n";
}

pub fn emit_preset(pin: &str, l: &PresetLists) -> String {
    let mut s = crate::emit::header(pin, "`packages/cli/src/preset/preset.ts` and `packages/cli/src/icons/libraries.ts`");
    emit_enum(&mut s, "StyleName", "A shadcn style (upstream `PRESET_STYLES`).", &l.styles);
    emit_enum(&mut s, "BaseColor", "A base colour (upstream `PRESET_BASE_COLORS`).", &l.base_colors);
    emit_enum(&mut s, "ThemeColor", "A colour theme, also used for chart colours (upstream `PRESET_THEMES`).", &l.themes);
    emit_enum(&mut s, "FontName", "A font (upstream `PRESET_FONTS`).", &l.fonts);
    emit_enum(&mut s, "IconLibrary", "An icon library (upstream `iconLibraries` keys).", &l.icon_libraries);
    emit_enum(&mut s, "MenuColor", "Menu content colours (upstream `PRESET_MENU_COLORS`).", &l.menu_colors);
    emit_enum(&mut s, "MenuAccent", "Menu highlight (upstream `PRESET_MENU_ACCENTS`).", &l.menu_accents);
    // RadiusChoice: the coded radii plus `Px`, which has no code.
    s += "/// A radius (upstream `PRESET_RADII`), or `Px` for any other radius.\n";
    s += "#[derive(Clone, Copy, Debug, PartialEq)]\npub enum RadiusChoice {\n";
    for (id, rem) in &l.radii {
        s += &format!("    /// `\"{id}\"` ({rem:?}rem)\n    {},\n", pascal(id));
    }
    s += "    /// Any radius in logical px; such a preset has no code.\n    Px(f32),\n}\n\nimpl RadiusChoice {\n";
    s += "    /// The coded values, in upstream list order.\n";
    s += &format!("    pub const ALL: [Self; {}] = [{}];\n\n", l.radii.len(), l.radii.iter().map(|(i, _)| format!("Self::{}", pascal(i))).collect::<Vec<_>>().join(", "));
    s += "    /// The upstream id; `None` for `Px`.\n    pub const fn id(self) -> Option<&'static str> {\n        match self {\n";
    for (id, _) in &l.radii {
        s += &format!("            Self::{} => Some(\"{id}\"),\n", pascal(id));
    }
    s += "            Self::Px(_) => None,\n        }\n    }\n\n";
    s += "    /// The upstream value in rem; `None` for `Px`.\n    pub const fn rem(self) -> Option<f32> {\n        match self {\n";
    for (id, rem) in &l.radii {
        s += &format!("            Self::{} => Some({rem:?}),\n", pascal(id));
    }
    s += "            Self::Px(_) => None,\n        }\n    }\n\n";
    s += "    /// The coded value with this upstream id.\n    pub fn from_id(id: &str) -> Option<Self> {\n";
    s += "        Self::ALL.into_iter().find(|v| v.id() == Some(id))\n    }\n}\n\n";
    s += "/// The fields of a preset code (upstream `PRESET_FIELDS_V2` keys).\n";
    s += "#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub(crate) enum Field {\n";
    for (k, _) in &l.fields_v2 {
        s += &format!("    {},\n", pascal(k));
    }
    s += "}\n\n";
    for (name, version, fields) in [("FIELDS_V1", "a", &l.fields_v1), ("FIELDS_V2", "b", &l.fields_v2)] {
        s += &format!("/// Version `{version}` codes: (field, bits), low bits first.\n");
        s += &format!("pub(crate) const {name}: [(Field, u32); {}] = [\n", fields.len());
        for (k, b) in fields {
            s += &format!("    (Field::{}, {b}),\n", pascal(k));
        }
        s += "];\n\n";
    }
    s.truncate(s.trim_end().len());
    s.push('\n');
    s
}
```

`tools/stylegen/src/lib.rs`: add `pub mod preset;`.

`tools/stylegen/src/main.rs`: add `preset` to the `use stylegen::{…}` list,
and in `generate`, after the palette block:

```rust
    let enums = preset::parse_preset(&read(&tool.join("tests/fixtures/preset.ts")), &read(&tool.join("tests/fixtures/libraries.ts")))
        .map(|l| preset::emit_preset(pin, &l));
    match enums {
        Ok(src) => {
            let dir = root.join(CRATE).join("src/preset");
            std::fs::create_dir_all(&dir).expect("preset dir");
            write(&dir.join("generated.rs"), &src);
        }
        Err(e) => {
            eprintln!("preset: {e}");
            failed = true;
        }
    }
```

- [ ] **Step 3: Generate and wire the crate module**

```bash
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- generate "$UP" .
git status --short   # new: crates/shadcn-egui/src/preset/generated.rs; nothing else changed
```

`crates/shadcn-egui/src/preset/mod.rs`:

```rust
//! shadcn presets: every choice of the shadcn "create" page (style, colours,
//! fonts, icons, radius, menu) and the preset codes the site shows.
//! The enums are generated from upstream `preset.ts` (`generated.rs`, by
//! `tools/stylegen`); their order is the index inside a preset code.

// `Field` and `FIELDS_*` are read by `code.rs` from the next task on, which
// removes this allow.
#[allow(dead_code)]
mod generated;

pub use generated::{BaseColor, FontName, IconLibrary, MenuAccent, MenuColor, RadiusChoice, StyleName, ThemeColor};
```

`crates/shadcn-egui/src/lib.rs`: add `pub mod preset;` (alphabetical, after
`pub mod layout;`) and
`pub use preset::{BaseColor, FontName, IconLibrary, MenuAccent, MenuColor, RadiusChoice, StyleName, ThemeColor};`.

- [ ] **Step 4: Write the crate tests**

`crates/shadcn-egui/tests/preset.rs`:

```rust
use shadcn_egui::{BaseColor, FontName, IconLibrary, MenuAccent, MenuColor, RadiusChoice, StyleName, ThemeColor};

#[test]
fn enums_follow_upstream_list_order() {
    assert_eq!(StyleName::ALL.map(|s| s.id()), ["nova", "vega", "maia", "lyra", "mira", "luma", "sera", "rhea"]);
    assert_eq!(BaseColor::ALL.map(|b| b.id()), ["neutral", "stone", "zinc", "mauve", "olive", "mist", "taupe"]);
    assert_eq!(ThemeColor::ALL.len(), 24);
    assert_eq!((ThemeColor::ALL[3], ThemeColor::ALL[23]), (ThemeColor::Amber, ThemeColor::Taupe));
    assert_eq!(FontName::ALL.len(), 26);
    assert_eq!((FontName::ALL[0], FontName::ALL[17], FontName::ALL[25]), (FontName::Inter, FontName::Oxanium, FontName::InstrumentSerif));
    assert_eq!(FontName::SourceSans3.id(), "source-sans-3");
    assert_eq!(IconLibrary::ALL.map(|l| l.id()), ["lucide", "tabler", "hugeicons", "phosphor", "remixicon"]);
    assert_eq!(MenuColor::ALL.map(|m| m.id()), ["default", "inverted", "default-translucent", "inverted-translucent"]);
    assert_eq!(MenuAccent::ALL.map(|m| m.id()), ["subtle", "bold"]);
    assert_eq!(RadiusChoice::ALL.map(|r| r.rem()), [Some(0.5), Some(0.0), Some(0.45), Some(0.625), Some(0.875)]);
}

#[test]
fn ids_round_trip_and_unknown_ids_are_none() {
    for v in StyleName::ALL {
        assert_eq!(StyleName::from_id(v.id()), Some(v));
    }
    for v in BaseColor::ALL {
        assert_eq!(BaseColor::from_id(v.id()), Some(v));
    }
    for v in ThemeColor::ALL {
        assert_eq!(ThemeColor::from_id(v.id()), Some(v));
    }
    for v in FontName::ALL {
        assert_eq!(FontName::from_id(v.id()), Some(v));
    }
    for v in IconLibrary::ALL {
        assert_eq!(IconLibrary::from_id(v.id()), Some(v));
    }
    for v in MenuColor::ALL {
        assert_eq!(MenuColor::from_id(v.id()), Some(v));
    }
    for v in MenuAccent::ALL {
        assert_eq!(MenuAccent::from_id(v.id()), Some(v));
    }
    for v in RadiusChoice::ALL {
        assert_eq!(RadiusChoice::from_id(v.id().expect("coded")), Some(v));
    }
    assert_eq!((RadiusChoice::Px(3.0).id(), RadiusChoice::Px(3.0).rem()), (None, None));
    assert_eq!(StyleName::from_id("Vega"), None, "ids are case sensitive, as upstream");
    assert_eq!(FontName::from_id(""), None);
    assert_eq!(RadiusChoice::from_id("px"), None);
}
```

- [ ] **Step 5: Run everything**

```bash
cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4
cargo test -p shadcn-egui --jobs 4 --test preset -- --test-threads 4
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
cargo clippy --manifest-path tools/stylegen/Cargo.toml --all-targets --jobs 4 -- -D warnings
```

Expected: all pass, warning-clean.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: preset enums generated from upstream preset.ts and libraries.ts

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4.2: `Preset::from_code` / `to_code`

**Files:**
- Modify: `crates/shadcn-egui/src/preset/mod.rs` (`Preset`, `FontHeading`, `mod code`)
- Create: `crates/shadcn-egui/src/preset/code.rs`
- Create: `tools/stylegen/upstream/preset-codes.mjs`
- Create (generated by the script): `crates/shadcn-egui/tests/fixtures/preset_codes.txt`
- Create: `crates/shadcn-egui/tests/preset_codes.rs`
- Modify: `crates/shadcn-egui/src/lib.rs`

**Interfaces:**
- Consumes: Task 4.1 enums, `Field`, `FIELDS_V1`, `FIELDS_V2`; `tools/stylegen/upstream/preset-codes.txt` (P0a).
- Produces (index C): `pub enum FontHeading { Inherit, Font(FontName) }` (`Clone, Copy, Debug, PartialEq, Eq, Hash`); `pub struct Preset { style, base_color, theme, chart_color, font, font_heading, icons, radius, menu_color, menu_accent }` (`Clone, Copy, Debug, PartialEq`); `Preset::from_code(&str) -> Result<Preset, PresetCodeError>`; `Preset::to_code(&self) -> Option<String>`; `PresetCodeError { TooShort, UnknownVersion(char), InvalidChar(char), TooLong }` (`Clone, Copy, Debug, PartialEq, Eq`, `Display`, `Error`). All at the crate root.
- NEW INTERFACE: fixture line `<code> toolong` (upstream would decode a payload above 2^53 imprecisely; we return `TooLong`), besides `<code> null` and the 12-column line of the index.

- [ ] **Step 1: The fixture script**

`tools/stylegen/upstream/preset-codes.mjs`:

```js
// Decodes preset codes with upstream decodePreset/encodePreset
// (packages/cli/src/preset/preset.ts) for crates/shadcn-egui/tests/preset_codes.rs.
// Codes: tools/stylegen/upstream/preset-codes.txt (both sites, P0a), the
// encodes of the svelte CLI DEFAULT_PRESETS and docs PRESETS, and 64 configs
// from a seeded generator.
//
// Usage: node tools/stylegen/upstream/preset-codes.mjs <shadcn-svelte checkout> <scratch dir> \
//          > crates/shadcn-egui/tests/fixtures/preset_codes.txt
// Needs node >= 22.18 (TypeScript type stripping on by default).
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const [up, scratch] = process.argv.slice(2);
if (!up || !scratch) {
  console.error("usage: preset-codes.mjs <shadcn-svelte checkout> <scratch dir>");
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
      if (depth === 0) return text.slice(open, i + 1);
    }
  }
  throw new Error(`unbalanced: ${marker}`);
}
/** An object or array literal from upstream source, evaluated. */
const literal = (text, marker) => new Function(`return ${slice(text, marker).replace(/ as const/g, "")};`)();

const pin = execFileSync("git", ["-C", up, "rev-parse", "HEAD"]).toString().trim().slice(0, 8);
// Only the keys of iconLibraries matter; its helper import is stubbed.
const libs = readFileSync(join(up, "packages/cli/src/icons/libraries.ts"), "utf8")
  .replace(/^import \{ pascalToKebab \} from .*$/m, "const pascalToKebab = (s: string) => s;");
const presetTs = readFileSync(join(up, "packages/cli/src/preset/preset.ts"), "utf8")
  .replace(/^import \{ iconLibraries \} from .*$/m, 'import { iconLibraries } from "./libraries.ts";')
  .replace(/^import \{ keys \} from .*$/m, "const keys = (o: object) => Object.keys(o);");
if (/from "\.\.\//.test(libs) || /from "\.\.\//.test(presetTs)) throw new Error("an upstream import was not rewritten");
writeFileSync(join(scratch, "libraries.ts"), libs);
writeFileSync(join(scratch, "preset.ts"), presetTs);
const P = await import(pathToFileURL(join(scratch, "preset.ts")).href);

const codes = [];
for (const line of readFileSync(new URL("./preset-codes.txt", import.meta.url), "utf8").split("\n")) {
  const code = line.replace(/#.*/, "").trim();
  if (code) codes.push(code);
}
const presets = readFileSync(join(up, "packages/cli/src/preset/presets.ts"), "utf8");
const config = readFileSync(join(up, "docs/src/lib/registry/config.ts"), "utf8");
for (const p of Object.values(literal(presets, "export const DEFAULT_PRESETS ="))) codes.push(P.encodePreset(p));
for (const p of literal(config, "export const PRESETS: Preset[] =")) codes.push(P.encodePreset(p));

const lists = {
  style: P.PRESET_STYLES,
  baseColor: P.PRESET_BASE_COLOR_KEYS,
  theme: P.PRESET_THEME_KEYS,
  chartColor: P.PRESET_CHART_COLORS,
  iconLibrary: P.PRESET_ICON_LIBRARIES,
  font: P.PRESET_FONTS,
  fontHeading: P.PRESET_FONT_HEADINGS,
  radius: P.PRESET_RADII_KEYS,
  menuAccent: P.PRESET_MENU_ACCENTS,
  menuColor: P.PRESET_MENU_COLORS,
};
let seed = 0x5eed2026n;
const next = (n) => {
  seed = (seed * 6364136223846793005n + 1442695040888963407n) & 0xffffffffffffffffn;
  return Number((seed >> 33n) % BigInt(n));
};
for (let i = 0; i < 64; i++) {
  codes.push(P.encodePreset(Object.fromEntries(Object.entries(lists).map(([k, v]) => [k, v[next(v.length)]]))));
}

const out = [
  `# @generated by tools/stylegen/upstream/preset-codes.mjs from shadcn-svelte ${pin}; do not edit.`,
  "# code style base theme chart|- icons font heading radius accent menu canonical(0|1)  |  code null  |  code toolong",
];
const seen = new Set();
for (const code of codes) {
  if (seen.has(code)) continue;
  seen.add(code);
  const payload = code.slice(1);
  if (/^[0-9A-Za-z]+$/.test(payload) && P.fromBase62(payload) > 2 ** 53) {
    out.push(`${code} toolong`);
    continue;
  }
  const d = P.decodePreset(code);
  if (!d) {
    out.push(`${code} null`);
    continue;
  }
  const canonical = code[0] === "b" && P.encodePreset(d) === code ? 1 : 0;
  out.push(
    [code, d.style, d.baseColor, d.theme, d.chartColor ?? "-", d.iconLibrary, d.font, d.fontHeading, d.radius, d.menuAccent, d.menuColor, canonical].join(" "),
  );
}
process.stdout.write(out.join("\n") + "\n");
```

```bash
mkdir -p crates/shadcn-egui/tests/fixtures
node tools/stylegen/upstream/preset-codes.mjs "$UP" "$SCRATCH/preset-codes" > crates/shadcn-egui/tests/fixtures/preset_codes.txt
grep -vc '^#' crates/shadcn-egui/tests/fixtures/preset_codes.txt     # >= 70
grep -c ' 1$' crates/shadcn-egui/tests/fixtures/preset_codes.txt      # canonical b codes, >= 64
```

Run it a second time into a scratch file and `cmp`: identical.

- [ ] **Step 2: Write the failing tests**

`crates/shadcn-egui/tests/preset_codes.rs`:

```rust
use shadcn_egui::{BaseColor, FontHeading, FontName, IconLibrary, MenuAccent, MenuColor, Preset, PresetCodeError, RadiusChoice, StyleName, ThemeColor};

const FIXTURE: &str = include_str!("fixtures/preset_codes.txt");

fn lines() -> impl Iterator<Item = Vec<&'static str>> {
    FIXTURE.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).map(|l| l.split(' ').collect())
}

/// The chart colour of an `a` code (upstream leaves it unset: "-").
fn v1_chart(_base: BaseColor) -> ThemeColor {
    ThemeColor::ALL[0]
}

fn expected(w: &[&str]) -> Preset {
    let base = BaseColor::from_id(w[2]).expect("base");
    Preset {
        style: StyleName::from_id(w[1]).expect("style"),
        base_color: base,
        theme: ThemeColor::from_id(w[3]).expect("theme"),
        chart_color: if w[4] == "-" { v1_chart(base) } else { ThemeColor::from_id(w[4]).expect("chart") },
        icons: IconLibrary::from_id(w[5]).expect("icons"),
        font: FontName::from_id(w[6]).expect("font"),
        font_heading: if w[7] == "inherit" { FontHeading::Inherit } else { FontHeading::Font(FontName::from_id(w[7]).expect("heading")) },
        radius: RadiusChoice::from_id(w[8]).expect("radius"),
        menu_accent: MenuAccent::from_id(w[9]).expect("accent"),
        menu_color: MenuColor::from_id(w[10]).expect("menu"),
    }
}

#[test]
fn every_fixture_code_decodes_like_upstream() {
    let mut n = 0;
    for w in lines() {
        n += 1;
        match w[1] {
            "null" => assert!(Preset::from_code(w[0]).is_err(), "{}: upstream returns null", w[0]),
            "toolong" => assert_eq!(Preset::from_code(w[0]), Err(PresetCodeError::TooLong), "{}", w[0]),
            _ => assert_eq!(Preset::from_code(w[0]), Ok(expected(&w)), "{}", w[0]),
        }
    }
    assert!(n >= 70, "only {n} fixture codes");
}

#[test]
fn canonical_codes_round_trip() {
    let mut n = 0;
    for w in lines().filter(|w| w.len() == 12 && w[11] == "1") {
        let p = Preset::from_code(w[0]).expect("decodes");
        assert_eq!(p.to_code().as_deref(), Some(w[0]));
        n += 1;
    }
    assert!(n >= 64, "only {n} canonical codes");
}

/// Review Focus 1: pasted junk is an error, never a panic or a wrong preset.
#[test]
fn malformed_codes_are_errors_not_panics() {
    for s in ["", " b0", "b0\n", "b0 ", "bé", "é", "😀", "b😀", "b", "a", "B0", "b0.", "b-1"] {
        assert!(Preset::from_code(s).is_err(), "{s:?} must be an error");
    }
    assert_eq!(Preset::from_code(&format!("b{}", "Z".repeat(999))), Err(PresetCodeError::TooLong));
    for a in 0u8..128 {
        assert!(Preset::from_code(&char::from(a).to_string()).is_err());
        for b in 0u8..128 {
            let s: String = [char::from(a), char::from(b)].iter().collect();
            let valid = matches!(a, b'a' | b'b') && b.is_ascii_alphanumeric();
            assert_eq!(Preset::from_code(&s).is_ok(), valid, "{s:?}");
        }
    }
}
```

Unit tests at the bottom of `crates/shadcn-egui/src/preset/code.rs` (write
them with the module in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use PresetCodeError::*;

    #[test]
    fn each_error() {
        assert_eq!(Preset::from_code(""), Err(TooShort));
        assert_eq!(Preset::from_code("b"), Err(TooShort));
        assert_eq!(Preset::from_code("c0"), Err(UnknownVersion('c')));
        assert_eq!(Preset::from_code("b0-"), Err(InvalidChar('-')));
        assert_eq!(TooLong.to_string(), "preset code is too long");
    }

    #[test]
    fn too_long_starts_above_two_to_the_53() {
        assert!(Preset::from_code(&format!("b{}", to_base62(1 << 53))).is_ok());
        assert_eq!(Preset::from_code(&format!("b{}", to_base62((1 << 53) + 1))), Err(TooLong));
    }

    #[test]
    fn base62() {
        assert_eq!((to_base62(0), to_base62(61), to_base62(62)), ("0".into(), "z".into(), "10".into()));
    }

    #[test]
    fn all_zero_is_index_zero_everywhere() {
        let p = Preset::from_code("b0").expect("decodes");
        assert_eq!(p.style, StyleName::Nova);
        assert_eq!((p.base_color, p.theme, p.chart_color), (BaseColor::Neutral, ThemeColor::Neutral, ThemeColor::Neutral));
        assert_eq!((p.font, p.font_heading, p.icons), (FontName::Inter, FontHeading::Inherit, IconLibrary::Lucide));
        assert_eq!((p.radius, p.menu_color, p.menu_accent), (RadiusChoice::Default, MenuColor::Default, MenuAccent::Subtle));
    }

    #[test]
    fn an_index_past_the_list_decodes_to_index_zero() {
        // style: 6 bits at offset 34; 63 is past the 8 styles.
        assert_eq!(Preset::from_code(&format!("b{}", to_base62(63 << 34))).map(|p| p.style), Ok(StyleName::Nova));
        assert_eq!(Preset::from_code(&format!("b{}", to_base62(1 << 34))).map(|p| p.style), Ok(StyleName::Vega));
        // fontHeading: 5 bits at offset 46; 27 and up are past "inherit" + 26 fonts.
        assert_eq!(Preset::from_code(&format!("b{}", to_base62(31 << 46))).map(|p| p.font_heading), Ok(FontHeading::Inherit));
        assert_eq!(Preset::from_code(&format!("b{}", to_base62(26 << 46))).map(|p| p.font_heading), Ok(FontHeading::Font(FontName::InstrumentSerif)));
    }

    #[test]
    fn a_codes_ignore_the_v2_fields() {
        let p = Preset::from_code(&format!("a{}", to_base62((5 << 40) | (3 << 46)))).expect("decodes");
        assert_eq!((p.chart_color, p.font_heading), (ThemeColor::ALL[0], FontHeading::Inherit));
    }

    #[test]
    fn px_radius_has_no_code() {
        let p = Preset::from_code("b0").expect("decodes");
        assert_eq!(Preset { radius: RadiusChoice::Px(6.0), ..p }.to_code(), None);
        assert_eq!(p.to_code().as_deref(), Some("b0"));
    }
}
```

Run: `cargo test -p shadcn-egui --jobs 4 --test preset_codes -- --test-threads 4`
Expected: FAIL to compile (`Preset`, `FontHeading`, `PresetCodeError` missing).

- [ ] **Step 3: Implement**

`crates/shadcn-egui/src/preset/mod.rs` — remove the `#[allow(dead_code)]`
and its comment above `mod generated;`, add `mod code;` and:

```rust
pub use code::PresetCodeError;

/// The heading font: the body font (upstream `"inherit"`) or another font.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontHeading {
    Inherit,
    Font(FontName),
}

/// A shadcn preset: the choices of the shadcn "create" page. Plain data;
/// [`Preset::theme`] turns it into a [`crate::Theme`]. Light or dark is not
/// part of a preset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preset {
    pub style: StyleName,
    pub base_color: BaseColor,
    pub theme: ThemeColor,
    pub chart_color: ThemeColor,
    pub font: FontName,
    pub font_heading: FontHeading,
    pub icons: IconLibrary,
    pub radius: RadiusChoice,
    pub menu_color: MenuColor,
    pub menu_accent: MenuAccent,
}
```

`crates/shadcn-egui/src/preset/code.rs`:

```rust
//! Preset codes: a port of upstream `decodePreset` / `encodePreset`
//! (`packages/cli/src/preset/preset.ts`). A code is a version letter (`a`:
//! 40 bits, `b`: 51 bits) and a base62 number whose bit fields are the list
//! indices, low bits first in `FIELDS_V2` order.

use super::generated::{Field, FIELDS_V1, FIELDS_V2};
use super::{BaseColor, FontHeading, FontName, IconLibrary, MenuAccent, MenuColor, Preset, RadiusChoice, StyleName, ThemeColor};

const BASE62: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Upstream decodes in f64, exact only up to 2^53; above that we refuse.
const MAX_PAYLOAD: u64 = 1 << 53;

/// Why a string is not a preset code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetCodeError {
    /// Empty or a single character.
    TooShort,
    /// The first character is not a known version (`a`, `b`).
    UnknownVersion(char),
    /// A character after the version is not base62 (`0-9A-Za-z`).
    InvalidChar(char),
    /// The number is above 2^53, which the shadcn sites never write.
    TooLong,
}

impl std::fmt::Display for PresetCodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "preset code is too short"),
            Self::UnknownVersion(c) => write!(f, "unknown preset code version {c:?}"),
            Self::InvalidChar(c) => write!(f, "invalid character {c:?} in preset code"),
            Self::TooLong => write!(f, "preset code is too long"),
        }
    }
}

impl std::error::Error for PresetCodeError {}

fn digit(c: char) -> Option<u64> {
    BASE62.iter().position(|&d| char::from(d) == c).map(|i| i as u64)
}

pub(super) fn to_base62(mut n: u64) -> String {
    let mut out = Vec::new();
    loop {
        out.push(BASE62[(n % 62) as usize]);
        n /= 62;
        if n == 0 {
            break;
        }
    }
    out.reverse();
    String::from_utf8(out).expect("base62 is ASCII")
}

/// `all[idx]`, or `all[0]` past the end (upstream behaviour).
fn pick<T: Copy>(all: &[T], idx: usize) -> T {
    all.get(idx).copied().unwrap_or(all[0])
}

impl Preset {
    /// Decode a preset code from ui.shadcn.com or shadcn-svelte.com
    /// (`"b0"`, `"a…"`). An index past the end of a list decodes to the
    /// list's first value, as upstream does.
    pub fn from_code(code: &str) -> Result<Preset, PresetCodeError> {
        let mut chars = code.chars();
        let version = chars.next().ok_or(PresetCodeError::TooShort)?;
        if code.chars().nth(1).is_none() {
            return Err(PresetCodeError::TooShort);
        }
        let fields: &[(Field, u32)] = match version {
            'a' => &FIELDS_V1,
            'b' => &FIELDS_V2,
            c => return Err(PresetCodeError::UnknownVersion(c)),
        };
        let mut bits: u64 = 0;
        for c in chars {
            let d = digit(c).ok_or(PresetCodeError::InvalidChar(c))?;
            bits = bits.checked_mul(62).and_then(|b| b.checked_add(d)).filter(|&b| b <= MAX_PAYLOAD).ok_or(PresetCodeError::TooLong)?;
        }
        // Fields an `a` code lacks keep these: heading inherit, chart index 0.
        let mut p = Preset {
            style: StyleName::ALL[0],
            base_color: BaseColor::ALL[0],
            theme: ThemeColor::ALL[0],
            chart_color: ThemeColor::ALL[0],
            font: FontName::ALL[0],
            font_heading: FontHeading::Inherit,
            icons: IconLibrary::ALL[0],
            radius: RadiusChoice::ALL[0],
            menu_color: MenuColor::ALL[0],
            menu_accent: MenuAccent::ALL[0],
        };
        let mut offset = 0;
        for &(field, width) in fields {
            let idx = ((bits >> offset) & ((1 << width) - 1)) as usize;
            p.set_index(field, idx);
            offset += width;
        }
        Ok(p)
    }

    /// The `b` code the shadcn sites write for this preset; `None` when the
    /// radius is [`RadiusChoice::Px`], which has no code.
    pub fn to_code(&self) -> Option<String> {
        let mut bits: u64 = 0;
        let mut offset = 0;
        for &(field, width) in &FIELDS_V2 {
            bits |= (self.index(field)? as u64) << offset;
            offset += width;
        }
        Some(format!("b{}", to_base62(bits)))
    }

    fn set_index(&mut self, field: Field, idx: usize) {
        match field {
            Field::MenuColor => self.menu_color = pick(&MenuColor::ALL, idx),
            Field::MenuAccent => self.menu_accent = pick(&MenuAccent::ALL, idx),
            Field::Radius => self.radius = pick(&RadiusChoice::ALL, idx),
            Field::Font => self.font = pick(&FontName::ALL, idx),
            Field::IconLibrary => self.icons = pick(&IconLibrary::ALL, idx),
            Field::Theme => self.theme = pick(&ThemeColor::ALL, idx),
            Field::BaseColor => self.base_color = pick(&BaseColor::ALL, idx),
            Field::Style => self.style = pick(&StyleName::ALL, idx),
            Field::ChartColor => self.chart_color = pick(&ThemeColor::ALL, idx),
            Field::FontHeading => {
                self.font_heading = match idx.checked_sub(1).and_then(|i| FontName::ALL.get(i)) {
                    Some(&f) => FontHeading::Font(f),
                    None => FontHeading::Inherit,
                }
            }
        }
    }

    /// The list index of a field; `None` only for a `Px` radius.
    fn index(&self, field: Field) -> Option<usize> {
        fn pos<T: PartialEq>(all: &[T], v: &T) -> Option<usize> {
            all.iter().position(|x| x == v)
        }
        match field {
            Field::MenuColor => pos(&MenuColor::ALL, &self.menu_color),
            Field::MenuAccent => pos(&MenuAccent::ALL, &self.menu_accent),
            Field::Radius => pos(&RadiusChoice::ALL, &self.radius),
            Field::Font => pos(&FontName::ALL, &self.font),
            Field::IconLibrary => pos(&IconLibrary::ALL, &self.icons),
            Field::Theme => pos(&ThemeColor::ALL, &self.theme),
            Field::BaseColor => pos(&BaseColor::ALL, &self.base_color),
            Field::Style => pos(&StyleName::ALL, &self.style),
            Field::ChartColor => pos(&ThemeColor::ALL, &self.chart_color),
            Field::FontHeading => match self.font_heading {
                FontHeading::Inherit => Some(0),
                FontHeading::Font(f) => pos(&FontName::ALL, &f).map(|i| i + 1),
            },
        }
    }
}
```

`crates/shadcn-egui/src/lib.rs` — the preset re-export becomes:

```rust
pub use preset::{BaseColor, FontHeading, FontName, IconLibrary, MenuAccent, MenuColor, Preset, PresetCodeError, RadiusChoice, StyleName, ThemeColor};
```

**V1 chart map branch.** Only if `docs/superpowers/notes/0.4/p0a-preset-codes.md`
says a site decoder applies `V1_CHART_COLOR_MAP`: in `from_code`, before
`Ok(p)`, add

```rust
        if version == 'a' {
            // Upstream V1_CHART_COLOR_MAP: `a` codes predate chart colours.
            p.chart_color = match p.base_color {
                BaseColor::Neutral => ThemeColor::Blue,
                BaseColor::Stone => ThemeColor::Lime,
                BaseColor::Zinc => ThemeColor::Amber,
                BaseColor::Mauve => ThemeColor::Emerald,
                BaseColor::Olive => ThemeColor::Violet,
                BaseColor::Mist => ThemeColor::Rose,
                BaseColor::Taupe => ThemeColor::Cyan,
            };
        }
```

make `v1_chart` in `tests/preset_codes.rs` return the same mapping, and
change the unit test `a_codes_ignore_the_v2_fields` to expect
`ThemeColor::Blue` (base neutral). The fixture script prints the upstream
decoder's raw `chartColor` (`-`), so the fixture itself needs no change.
Otherwise leave both as written.

- [ ] **Step 4: Run the tests**

```bash
cargo test -p shadcn-egui --jobs 4 --lib preset -- --test-threads 4
cargo test -p shadcn-egui --jobs 4 --test preset_codes --test preset -- --test-threads 4
```

Expected: all pass. A fixture mismatch prints the code: fix `code.rs`,
never the fixture.

- [ ] **Step 5: Clippy and commit**

```bash
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
git add -A
git commit -m "feat: decode and encode shadcn preset codes

Port of upstream decodePreset/encodePreset, checked against codes from
both shadcn sites decoded by upstream's own code. Payloads above 2^53,
which upstream decodes imprecisely, are refused.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4.3: `Preset::default`, setters, `Mode`

**Files:**
- Modify: `crates/shadcn-egui/src/preset/mod.rs`, `crates/shadcn-egui/src/lib.rs`
- Modify: `crates/shadcn-egui/tests/preset.rs`

**Interfaces:**
- Consumes: `Preset`, `FontHeading` (Task 4.2).
- Produces (index C): `impl Default for Preset` (upstream `DEFAULT_CONFIG` + heading inherit); `pub enum Mode { Light, Dark }` (`Clone, Copy, Debug, PartialEq, Eq`); by-value setters `style`, `base_color`, `theme_color` (for the `theme` field), `chart_color`, `font`, `font_heading`, `icons`, `radius`, `menu_color`, `menu_accent`. `Mode` at the crate root.

- [ ] **Step 1: Write the failing tests**

Append to `crates/shadcn-egui/tests/preset.rs`:

```rust
use shadcn_egui::{FontHeading, Mode, Preset};

#[test]
fn default_is_the_sites_default_config_not_code_b0() {
    let p = Preset::default();
    assert_eq!(
        p,
        Preset {
            style: StyleName::Vega,
            base_color: BaseColor::Neutral,
            theme: ThemeColor::Neutral,
            chart_color: ThemeColor::Neutral,
            font: FontName::Inter,
            font_heading: FontHeading::Inherit,
            icons: IconLibrary::Lucide,
            radius: RadiusChoice::Default,
            menu_color: MenuColor::Default,
            menu_accent: MenuAccent::Subtle,
        }
    );
    let b0 = Preset::from_code("b0").expect("decodes");
    assert_eq!(b0.style, StyleName::Nova, "all indices 0 is Nova, as upstream");
    assert_ne!(p, b0);
    assert_eq!(Preset::from_code(&p.to_code().expect("coded")), Ok(p));
}

#[test]
fn each_setter_changes_its_field() {
    let p = Preset::default()
        .style(StyleName::Nova)
        .base_color(BaseColor::Stone)
        .theme_color(ThemeColor::Blue)
        .chart_color(ThemeColor::Rose)
        .font(FontName::Geist)
        .font_heading(FontHeading::Font(FontName::Lora))
        .icons(IconLibrary::Tabler)
        .radius(RadiusChoice::Large)
        .menu_color(MenuColor::Inverted)
        .menu_accent(MenuAccent::Bold);
    assert_eq!(
        p,
        Preset {
            style: StyleName::Nova,
            base_color: BaseColor::Stone,
            theme: ThemeColor::Blue,
            chart_color: ThemeColor::Rose,
            font: FontName::Geist,
            font_heading: FontHeading::Font(FontName::Lora),
            icons: IconLibrary::Tabler,
            radius: RadiusChoice::Large,
            menu_color: MenuColor::Inverted,
            menu_accent: MenuAccent::Bold,
        }
    );
    assert_eq!(Preset::from_code(&p.to_code().expect("coded")), Ok(p));
    assert_eq!(Preset::default().radius(RadiusChoice::Px(6.0)).to_code(), None);
    assert_ne!(Mode::Light, Mode::Dark);
}
```

Run: `cargo test -p shadcn-egui --jobs 4 --test preset -- --test-threads 4`
Expected: FAIL to compile (`Mode`, `Default`, setters missing).

- [ ] **Step 2: Implement**

Append to `crates/shadcn-egui/src/preset/mod.rs`:

```rust
/// Light or dark: chosen by the app, not part of a preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Light,
    Dark,
}

/// The shadcn site's `DEFAULT_CONFIG`: Vega, neutral, Inter, Lucide. Not the
/// same as code `b0` (all indices 0 = Nova), as upstream.
impl Default for Preset {
    fn default() -> Self {
        Preset {
            style: StyleName::Vega,
            base_color: BaseColor::Neutral,
            theme: ThemeColor::Neutral,
            chart_color: ThemeColor::Neutral,
            font: FontName::Inter,
            font_heading: FontHeading::Inherit,
            icons: IconLibrary::Lucide,
            radius: RadiusChoice::Default,
            menu_color: MenuColor::Default,
            menu_accent: MenuAccent::Subtle,
        }
    }
}

impl Preset {
    pub fn style(mut self, v: StyleName) -> Self {
        self.style = v;
        self
    }
    pub fn base_color(mut self, v: BaseColor) -> Self {
        self.base_color = v;
        self
    }
    /// Sets the `theme` field; `theme(mode)` resolves the preset.
    pub fn theme_color(mut self, v: ThemeColor) -> Self {
        self.theme = v;
        self
    }
    pub fn chart_color(mut self, v: ThemeColor) -> Self {
        self.chart_color = v;
        self
    }
    pub fn font(mut self, v: FontName) -> Self {
        self.font = v;
        self
    }
    pub fn font_heading(mut self, v: FontHeading) -> Self {
        self.font_heading = v;
        self
    }
    pub fn icons(mut self, v: IconLibrary) -> Self {
        self.icons = v;
        self
    }
    pub fn radius(mut self, v: RadiusChoice) -> Self {
        self.radius = v;
        self
    }
    pub fn menu_color(mut self, v: MenuColor) -> Self {
        self.menu_color = v;
        self
    }
    pub fn menu_accent(mut self, v: MenuAccent) -> Self {
        self.menu_accent = v;
        self
    }
}
```

`crates/shadcn-egui/src/lib.rs`: add `Mode` to the `pub use preset::{…}` list.

- [ ] **Step 3: Run, clippy, commit**

```bash
cargo test -p shadcn-egui --jobs 4 --test preset --test preset_codes -- --test-threads 4
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
git add -A
git commit -m "feat: Preset default, setters and Mode

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4.4: `IconSlots`, `MenuColors`, `Theme.icons` / `Theme.menu`, font and icon registries

**Files:**
- Modify: `crates/shadcn-egui-core/src/icon.rs`, `crates/shadcn-egui-core/src/lib.rs` (`IconSlots`)
- Modify: `crates/shadcn-egui/src/components/icon.rs` (`LUCIDE_SLOTS`)
- Modify: `crates/shadcn-egui/src/theme.rs` (`MenuColors`, fields, builders, `menu_theme`)
- Modify: `crates/shadcn-egui/src/components/{checkbox.rs,select.rs,dialog.rs,button.rs}`
- Create: `crates/shadcn-egui/src/fonts.rs`, `crates/shadcn-egui/src/icons.rs`
- Modify: `crates/shadcn-egui/src/lib.rs`
- Modify: `crates/shadcn-egui/tests/theme.rs`

**Interfaces:**
- Consumes: `FontName`, `IconLibrary` (Task 4.1); `shadcn_egui_font_inter::FONT`, `shadcn_egui_font_oxanium::FONT` (Phase 2).
- Produces (index C): `shadcn_egui_core::IconSlots { check, chevron_down, close: Icon }` (`Clone, Copy, Debug, PartialEq`, no `Default`); `components::icon::LUCIDE_SLOTS`; `theme::MenuColors { palette: Option<Palette>, dark: Option<bool>, translucent: bool }` (`Clone, Copy, Debug, Default, PartialEq`) with `TRANSLUCENT_FILL = 0.7`, `TRANSLUCENT_HIGHLIGHT = 0.1`, `TRANSLUCENT_SEPARATOR = 0.05`; `Theme.icons`, `Theme.menu`; `Theme::icons(self, IconSlots)`, `Theme::menu(self, MenuColors)`, `Theme::menu_theme(&self) -> Theme`; `pub mod fonts` with `files(FontName) -> Option<&'static FontFiles>`; `pub mod icons` with `slots(IconLibrary) -> Option<IconSlots>`. `MenuColors` and `IconSlots` at the crate root.
- NEW INTERFACE: `Button::icon(self, icon: &Icon)` and `Button::icon_end(self, icon: &Icon)` take any `&Icon` (copied; was `&'static Icon`). Callers that pass `&CLOSE` keep compiling; the Dialog passes `&t.icons.close`, which is not `'static`.

**Stack and size.** `Theme::vega_dark()` needs ~200 KB of debug stack and
`every_exported_style_builds_on_a_small_stack` (in `src/style/mod.rs`) builds
it on a 256 KB thread. The new fields are `Copy` data of a few hundred
bytes, built inside `Theme::new` without any extra `Style` value, so the
frame does not grow measurably. This task pins that with a size test
(`size_of::<Theme>() <= 1024`) and runs the stack test explicitly.

- [ ] **Step 1: Write the failing tests**

Append to `crates/shadcn-egui/tests/theme.rs`:

```rust
use shadcn_egui::components::icon::{CHECK, CHEVRON_DOWN, CLOSE, LUCIDE_SLOTS};
use shadcn_egui::{FontName, IconLibrary, IconSlots, MenuColors};

#[test]
fn a_new_theme_draws_lucide_and_its_own_menu_colours() {
    let t = Theme::vega_light();
    assert_eq!(t.icons, IconSlots { check: CHECK, chevron_down: CHEVRON_DOWN, close: CLOSE });
    assert_eq!(t.icons, LUCIDE_SLOTS);
    assert_eq!(t.menu, MenuColors::default());
    assert_eq!(t.menu_theme(), t, "no menu colours: the menu draws with the theme itself");
}

#[test]
fn icon_and_menu_builders() {
    let crosses = IconSlots { check: CLOSE, chevron_down: CLOSE, close: CLOSE };
    assert_eq!(Theme::vega_light().icons(crosses).icons, crosses);
    let menu = MenuColors { palette: Some(Palette::dark()), dark: Some(true), translucent: true };
    let t = Theme::vega_light().menu(menu);
    assert_eq!(t.menu, menu);
    let mt = t.menu_theme();
    assert_eq!((mt.palette, mt.dark), (Palette::dark(), true));
    assert!(std::sync::Arc::ptr_eq(&mt.style, &t.style), "the menu theme shares the style");
    assert_eq!((t.palette, t.dark), (Palette::light(), false), "the theme itself is unchanged");
}

#[test]
fn registries_hold_what_this_build_bundles() {
    assert_eq!(shadcn_egui::fonts::files(FontName::Inter).map(|f| f.name), Some("inter"));
    assert_eq!(shadcn_egui::icons::slots(IconLibrary::Lucide), Some(LUCIDE_SLOTS));
    #[cfg(feature = "font-oxanium")]
    assert_eq!(shadcn_egui::fonts::files(FontName::Oxanium).map(|f| f.name), Some("oxanium"));
    #[cfg(not(feature = "font-oxanium"))]
    assert_eq!(shadcn_egui::fonts::files(FontName::Oxanium), None);
}

/// New `Theme` fields must stay small: `Theme::vega_dark()` already needs
/// ~200 KB of a 256 KB test stack (see `every_exported_style_builds_on_a_small_stack`).
#[test]
fn theme_stays_small() {
    assert!(std::mem::size_of::<Theme>() <= 1024, "Theme is {} bytes", std::mem::size_of::<Theme>());
}
```

Run: `cargo test -p shadcn-egui --jobs 4 --test theme -- --test-threads 4`
Expected: FAIL to compile (`IconSlots`, `MenuColors`, `LUCIDE_SLOTS`, `fonts`, `icons` missing).

- [ ] **Step 2: `IconSlots` in core**

Append to `crates/shadcn-egui-core/src/icon.rs`:

```rust
/// The icons the components draw, one per role. Each icon crate exports its
/// set as `SLOTS`; `shadcn_egui::Theme::icons` holds the active one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconSlots {
    /// Checkbox tick and the selected Select item.
    pub check: Icon,
    /// Select trigger (each library uses its own icon here).
    pub chevron_down: Icon,
    /// Dialog close button.
    pub close: Icon,
}
```

`crates/shadcn-egui-core/src/lib.rs`: `pub use icon::{Icon, IconSlots, Seg};`.

- [ ] **Step 3: `LUCIDE_SLOTS`, `MenuColors`, `Theme` fields and builders**

`crates/shadcn-egui/src/components/icon.rs`, after `CHEVRON_DOWN`:

```rust
/// The built-in Lucide slots every new [`Theme`] starts with.
pub const LUCIDE_SLOTS: IconSlots = IconSlots { check: CHECK, chevron_down: CHEVRON_DOWN, close: CLOSE };
```

and `pub use shadcn_egui_core::{Icon, IconSlots, Seg};` in its imports.

`crates/shadcn-egui/src/theme.rs`, after `impl Palette { … with_bold_menu_accent … }`:

```rust
/// How menu content is coloured (upstream `menuColor`): today the Select
/// list. `None` keeps the theme's own palette or dark flag.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MenuColors {
    /// The menu's palette; an inverted menu passes the dark palette.
    pub palette: Option<Palette>,
    /// The menu's dark flag; `Some(true)` for an inverted menu, so `dark:`
    /// rules resolve as in dark mode.
    pub dark: Option<bool>,
    /// Translucent fill, alpha only (egui has no backdrop blur).
    pub translucent: bool,
}

impl MenuColors {
    /// Menu fill: popover colour at this alpha (upstream `bg-popover/70`).
    pub const TRANSLUCENT_FILL: f32 = 0.7;
    /// Highlighted item: foreground colour at this alpha (`bg-foreground/10`).
    pub const TRANSLUCENT_HIGHLIGHT: f32 = 0.1;
    /// Separator: foreground colour at this alpha (`bg-foreground/5`).
    pub const TRANSLUCENT_SEPARATOR: f32 = 0.05;
}
```

The `Theme` struct gains, after `fonts`:

```rust
    /// The icons components draw (default: built-in Lucide).
    pub icons: IconSlots,
    /// Menu content colours (default: the theme's own).
    pub menu: MenuColors,
```

`Theme::new` ends with:

```rust
        Self {
            style,
            palette,
            radius,
            metrics: Metrics::default(),
            dark,
            fonts: ThemeFonts::inherit(&shadcn_egui_font_inter::FONT),
            icons: crate::components::icon::LUCIDE_SLOTS,
            menu: MenuColors::default(),
        }
```

Builders after `fonts(…)`:

```rust
    /// The icon set components draw from (an icon crate's `SLOTS`).
    pub fn icons(mut self, slots: IconSlots) -> Self {
        self.icons = slots;
        self
    }

    /// Menu content colours (upstream `menuColor`). An inverted menu needs
    /// the dark palette: `MenuColors { palette: Some(dark), dark: Some(true), .. }`.
    pub fn menu(mut self, menu: MenuColors) -> Self {
        self.menu = menu;
        self
    }

    /// The theme menu content draws with: this one, with palette and dark
    /// flag replaced by the menu's where set.
    pub fn menu_theme(&self) -> Theme {
        let mut t = self.clone();
        if let Some(p) = self.menu.palette {
            t.palette = p;
        }
        if let Some(d) = self.menu.dark {
            t.dark = d;
        }
        t
    }
```

Add `use shadcn_egui_core::IconSlots;` to the imports of `theme.rs`.

- [ ] **Step 4: Components read `t.icons`**

- `components/checkbox.rs`: import `use crate::components::icon::paint_icon;`; the tick becomes `paint_icon(&painter, icon_rect, &t.icons.check, color, t.metrics.icon_stroke);`.
- `components/select.rs`: import `use crate::components::icon::paint_icon;`; trigger `paint_icon(&painter, icon_rect, &t.icons.chevron_down, c, t.metrics.icon_stroke);`; item tick `paint_icon(ui.painter(), tick, &t.icons.check, color, t.metrics.icon_stroke);`.
- `components/dialog.rs`: delete `use crate::components::icon::CLOSE;`; the close button adds `Button::new("Close").icon(&t.icons.close)`.
- `components/button.rs`: the fields become `icon: Option<Icon>,` and `icon_end: Option<Icon>,`; the setters:

```rust
    pub fn icon(mut self, icon: &Icon) -> Self {
        self.icon = Some(*icon);
        self
    }
    /// An icon after the text (shadcn `<Icon data-icon="inline-end" />`).
    pub fn icon_end(mut self, icon: &Icon) -> Self {
        self.icon_end = Some(*icon);
        self
    }
```

  Where the body passes the stored icon on (`draw_icon(icon, x)` and the
  like), pass `&icon`: the compiler names each spot (`expected &Icon, found Icon`).

- [ ] **Step 5: Registries**

`crates/shadcn-egui/src/fonts.rs`:

```rust
//! The font crates this build bundles, by upstream font id. Inter is always
//! bundled; every other font needs its `font-<id>` feature.

use crate::preset::FontName;
use shadcn_egui_core::FontFiles;

/// The files of `name`, or `None` when its feature is off.
pub fn files(name: FontName) -> Option<&'static FontFiles> {
    match name {
        FontName::Inter => Some(&shadcn_egui_font_inter::FONT),
        #[cfg(feature = "font-oxanium")]
        FontName::Oxanium => Some(&shadcn_egui_font_oxanium::FONT),
        _ => None,
    }
}
```

`crates/shadcn-egui/src/icons.rs`:

```rust
//! The icon sets this build bundles, by upstream library id. Lucide is
//! always bundled; every other set needs its `icons-<id>` feature.

use crate::preset::IconLibrary;
use shadcn_egui_core::IconSlots;

/// The slot icons of `lib`, or `None` when its feature is off.
pub fn slots(lib: IconLibrary) -> Option<IconSlots> {
    match lib {
        IconLibrary::Lucide => Some(crate::components::icon::LUCIDE_SLOTS),
        _ => None,
    }
}
```

`crates/shadcn-egui/src/lib.rs`: add `pub mod fonts;` and `pub mod icons;`
(alphabetical), and `MenuColors` to `pub use theme::{…}`. `IconSlots`
reaches the root through `pub use shadcn_egui_core::*;`.

- [ ] **Step 6: Run everything; snapshots must not change**

```bash
cargo test -p shadcn-egui --jobs 4 --test theme -- --test-threads 4
cargo test -p shadcn-egui --jobs 4 --lib every_exported_style_builds_on_a_small_stack -- --test-threads 4
cargo test --workspace --jobs 4 -- --test-threads 4
cargo test --workspace --all-features --jobs 4 -- --test-threads 4
git status --short crates/shadcn-egui/tests/snapshots   # must print nothing
```

Expected: all pass; no PNG changed (the slots hold the same paths as the
consts the components drew before). A changed PNG means a component now
draws another icon: find out why before going on.

- [ ] **Step 7: Clippy and commit**

```bash
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
cargo clippy --workspace --all-targets --all-features --jobs 4 -- -D warnings
git add -A
git commit -m "feat: Theme carries icon slots and menu colours

Checkbox, Select and Dialog draw the theme's icons instead of the built-in
consts (same Lucide paths, no visual change). MenuColors and menu_theme()
prepare inverted and translucent menus. fonts::files and icons::slots map
preset values to what the build bundles. Button::icon takes any &Icon.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4.5: `Preset::theme` and `Preset::apply`

**Files:**
- Modify: `crates/shadcn-egui/src/preset/mod.rs`, `crates/shadcn-egui/src/palette/mod.rs` (`base_radius_px`), `crates/shadcn-egui/src/icons.rs` and `crates/shadcn-egui/src/fonts.rs` (`fallback_warning`)
- Modify: root `Cargo.toml` (`[workspace.dependencies] log = "0.4"`), `crates/shadcn-egui/Cargo.toml` (`log.workspace = true`)
- Modify: `crates/shadcn-egui/tests/preset.rs`

**Interfaces:**
- Consumes: everything from Tasks 4.1–4.4; `Palette::from_upstream`, `with_bold_menu_accent` (Phase 3); `Theme::new`, `Theme::fonts` (Phase 2).
- Produces (index C): `Preset::theme(&self, mode: Mode) -> Theme`, `Preset::apply(&self, ctx: &egui::Context, mode: Mode)`; `pub(crate) fn palette::base_radius_px(base: &str) -> Option<f32>`.
- NEW INTERFACE: `pub(crate) fn icons::fallback_warning(lib: IconLibrary) -> String` — the exact warning line for an icon library `slots` does not bundle. Phase 6 adds the Hugeicons "not available (licence)" arm here when `SETS.tsv` says `ship=no`.
- NEW INTERFACE: `pub(crate) fn fonts::fallback_warning(name: FontName, heading: bool) -> String` — the "needs the feature" line, or "is not available (licence)" for a font P0c left out — and the private `fonts::UNLICENSED: &[FontName]`. Phase 7 generates `UNLICENSED` into `fonts/registry.rs`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/shadcn-egui/tests/preset.rs`:

```rust
use shadcn_egui::{MenuColors, Palette, Theme};

#[test]
fn lyra_forces_radius_zero() {
    assert_eq!(Preset::default().style(StyleName::Lyra).radius(RadiusChoice::Large).theme(Mode::Light).radius, 0.0);
    assert_eq!(Preset::default().style(StyleName::Lyra).radius(RadiusChoice::Px(7.0)).theme(Mode::Light).radius, 0.0);
}

#[test]
fn radius_choices_resolve_to_px() {
    let r = |c| Preset::default().radius(c).theme(Mode::Dark).radius;
    assert_eq!(r(RadiusChoice::Default), 10.0, "the base colour's 0.625rem");
    assert_eq!(r(RadiusChoice::None), 0.0);
    assert_eq!(r(RadiusChoice::Small), 0.45 * 16.0);
    assert_eq!(r(RadiusChoice::Large), 0.875 * 16.0);
    assert_eq!(r(RadiusChoice::Px(7.5)), 7.5);
}

#[test]
fn default_preset_is_vega_with_the_hand_written_palettes() {
    let dark = Preset::default().theme(Mode::Dark);
    assert_eq!((dark.style.name, dark.palette, dark.dark), ("vega", Palette::dark(), true));
    let light = Preset::default().theme(Mode::Light);
    assert_eq!((light.palette, light.dark), (Palette::light(), false));
    assert_eq!(light.fonts.body.name, "inter");
    assert_eq!(light.menu, MenuColors::default());
}

#[test]
fn colours_merge_like_upstream() {
    let t = Preset::default().base_color(BaseColor::Stone).theme_color(ThemeColor::Blue).chart_color(ThemeColor::Rose).theme(Mode::Light);
    assert_eq!(Some(t.palette), Palette::from_upstream("stone", "blue", "rose", false));
}

#[test]
fn bold_accent_highlights_with_primary() {
    let t = Preset::default().theme_color(ThemeColor::Blue).menu_accent(MenuAccent::Bold).theme(Mode::Light);
    assert_eq!((t.palette.accent, t.palette.accent_foreground), (t.palette.primary, t.palette.primary_foreground));
}

#[test]
fn inverted_menus_use_the_dark_palette_and_flag() {
    let p = Preset::default().theme_color(ThemeColor::Blue).menu_accent(MenuAccent::Bold);
    let dark_palette = p.theme(Mode::Dark).palette;
    let t = p.menu_color(MenuColor::Inverted).theme(Mode::Light);
    assert_eq!(t.menu, MenuColors { palette: Some(dark_palette), dark: Some(true), translucent: false });
    assert!(!t.dark);
    assert!(p.menu_color(MenuColor::InvertedTranslucent).theme(Mode::Light).menu.translucent);
    let translucent = p.menu_color(MenuColor::DefaultTranslucent).theme(Mode::Light).menu;
    assert_eq!(translucent, MenuColors { palette: None, dark: None, translucent: true });
}

#[test]
fn a_heading_font_resolves() {
    let t = Preset::default().font_heading(FontHeading::Font(FontName::Inter)).theme(Mode::Light);
    assert_eq!((t.fonts.body.name, t.fonts.heading.name), ("inter", "inter"));
}

/// Review Focus 3: a value that falls back still encodes as requested.
#[test]
fn to_code_keeps_values_that_fall_back() {
    let p = Preset::default().style(StyleName::Sera).font(FontName::Geist);
    let _ = p.theme(Mode::Dark); // falls back to Nova / Inter, the preset stays as it is
    assert_eq!(Preset::from_code(&p.to_code().expect("coded")), Ok(p));
}

#[test]
fn every_preset_id_has_a_palette() {
    for b in BaseColor::ALL {
        for c in ThemeColor::ALL {
            assert!(Palette::from_upstream(b.id(), c.id(), c.id(), false).is_some(), "{b:?} {c:?}");
        }
    }
}

#[test]
fn apply_installs_the_theme() {
    let ctx = egui::Context::default();
    Preset::default().apply(&ctx, Mode::Light);
    assert_eq!(Theme::current(&ctx).palette, Palette::light());
}

/// A `Style` is ~100 KB in a debug build; resolving a preset must not hold
/// two of them, so every style resolves on a 256 KB thread.
#[test]
fn preset_theme_builds_on_a_small_stack() {
    let built = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(|| {
            for s in StyleName::ALL {
                let _ = Preset::default().style(s).theme(Mode::Dark);
            }
            Preset::default().theme(Mode::Dark).style.name
        })
        .expect("spawn")
        .join();
    assert_eq!(built.ok(), Some("vega"));
}
```

Run: `cargo test -p shadcn-egui --jobs 4 --test preset -- --test-threads 4`
Expected: FAIL to compile (`Preset::theme`, `Preset::apply` missing).

- [ ] **Step 2: Dependency, `base_radius_px`, the two `fallback_warning`s**

Root `Cargo.toml`, `[workspace.dependencies]`: `log = "0.4"`.
`crates/shadcn-egui/Cargo.toml`, `[dependencies]`: `log.workspace = true`.

Append to `crates/shadcn-egui/src/palette/mod.rs`:

```rust
/// A base colour's own radius (`radius: "0.625rem"` upstream) in px.
pub(crate) fn base_radius_px(base_id: &str) -> Option<f32> {
    base(base_id).map(|(rem, _, _)| rem * 16.0)
}
```

Append to `crates/shadcn-egui/src/icons.rs`:

```rust
/// The warning `Preset::theme` logs when [`slots`] has nothing for `lib`.
pub(crate) fn fallback_warning(lib: IconLibrary) -> String {
    let id = lib.id();
    format!("shadcn-egui: icon library \"{id}\" needs the \"icons-{id}\" feature; using \"lucide\"")
}
```

Append to `crates/shadcn-egui/src/fonts.rs`:

```rust
/// Fonts that get no crate because their licence is neither OFL-1.1 nor
/// Apache-2.0 (rows of `tools/fontgen/FONTS.tsv` whose `license` column says
/// so, P0c). No feature can bring them in.
const UNLICENSED: &[FontName] = &[/* {P0C_UNLICENSED} */];

/// The warning `Preset::theme` logs when [`files`] has nothing for `name`
/// (`heading`: the preset's heading font).
pub(crate) fn fallback_warning(name: FontName, heading: bool) -> String {
    let id = name.id();
    let what = if heading { "heading font" } else { "font" };
    if UNLICENSED.contains(&name) {
        format!("shadcn-egui: {what} \"{id}\" is not available (licence); using \"inter\"")
    } else {
        format!("shadcn-egui: {what} \"{id}\" needs the \"font-{id}\" feature; using \"inter\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `UNLICENSED` lists exactly the FONTS.tsv rows fontgen skips.
    #[test]
    fn unlicensed_matches_fonts_tsv() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools/fontgen/FONTS.tsv");
        let tsv = std::fs::read_to_string(path).expect("FONTS.tsv");
        let mut expected: Vec<FontName> = tsv
            .lines()
            .skip(1)
            .filter_map(|l| {
                let cols: Vec<&str> = l.split('\t').collect();
                (!matches!(cols[3], "OFL-1.1" | "Apache-2.0")).then(|| FontName::from_id(cols[0]).expect(cols[0]))
            })
            .collect();
        expected.sort_by_key(|f| f.id());
        let mut listed = UNLICENSED.to_vec();
        listed.sort_by_key(|f| f.id());
        assert_eq!(listed, expected);
    }
}
```

`{P0C_UNLICENSED}` is filled from the P0c notes
(`docs/superpowers/notes/0.4/p0c-licences.md`, "shipped: no" rows) as
`FontName::<Variant>` items; it stays empty when all 26 fonts are OFL-1.1 or
Apache-2.0. `unlicensed_matches_fonts_tsv` checks the list against
FONTS.tsv. Phase 7 replaces this const by the one fontgen generates.

- [ ] **Step 3: Implement `theme` and `apply`**

Append to `crates/shadcn-egui/src/preset/mod.rs`:

```rust
use crate::components::icon::LUCIDE_SLOTS;
use crate::style::Style;
use crate::theme::{MenuColors, Palette, Theme};

impl Preset {
    /// The [`Theme`] this preset describes. Pure apart from one
    /// `log::warn!` line per value this build cannot show (style Sera, a
    /// font or icon library whose feature is off); each falls back to a
    /// usable default, and the lines repeat on every call.
    pub fn theme(&self, mode: Mode) -> Theme {
        let dark = mode == Mode::Dark;
        let style = if self.style == StyleName::Sera {
            log::warn!("shadcn-egui: style \"sera\" is not available in egui; using \"nova\"");
            StyleName::Nova
        } else {
            self.style
        };
        let mut theme = with_style(style_fn(style), self.palette(dark), self.radius_px());
        let inter = crate::fonts::files(FontName::Inter).expect("Inter is always bundled");
        let body = crate::fonts::files(self.font).unwrap_or_else(|| {
            log::warn!("{}", crate::fonts::fallback_warning(self.font, false));
            inter
        });
        let heading = match self.font_heading {
            FontHeading::Inherit => None,
            FontHeading::Font(f) => Some(crate::fonts::files(f).unwrap_or_else(|| {
                log::warn!("{}", crate::fonts::fallback_warning(f, true));
                inter
            })),
        };
        let slots = crate::icons::slots(self.icons).unwrap_or_else(|| {
            log::warn!("{}", crate::icons::fallback_warning(self.icons));
            LUCIDE_SLOTS
        });
        let inverted = matches!(self.menu_color, MenuColor::Inverted | MenuColor::InvertedTranslucent);
        let menu = MenuColors {
            palette: inverted.then(|| self.palette(true)),
            dark: inverted.then_some(true),
            translucent: matches!(self.menu_color, MenuColor::DefaultTranslucent | MenuColor::InvertedTranslucent),
        };
        theme = theme.fonts(body, heading).icons(slots).menu(menu);
        theme.dark = dark;
        theme
    }

    /// `self.theme(mode).apply(ctx)`.
    pub fn apply(&self, ctx: &egui::Context, mode: Mode) {
        self.theme(mode).apply(ctx);
    }

    /// Base colour × theme × chart colour, then the menu accent.
    fn palette(&self, dark: bool) -> Palette {
        let p = Palette::from_upstream(self.base_color.id(), self.theme.id(), self.chart_color.id(), dark)
            .expect("every preset id has a generated palette");
        if self.menu_accent == MenuAccent::Bold {
            p.with_bold_menu_accent()
        } else {
            p
        }
    }

    /// Lyra and Sera (as requested, before Sera falls back) draw square
    /// corners, as the shadcn site does.
    fn radius_px(&self) -> f32 {
        if matches!(self.style, StyleName::Lyra | StyleName::Sera) {
            return 0.0;
        }
        match self.radius {
            RadiusChoice::Default => crate::palette::base_radius_px(self.base_color.id()).expect("every base colour has a radius"),
            RadiusChoice::Px(v) => v,
            r => r.rem().expect("coded radii have a rem value") * 16.0,
        }
    }
}

fn style_fn(s: StyleName) -> fn() -> Style {
    match s {
        StyleName::Nova | StyleName::Sera => Style::nova,
        StyleName::Vega => Style::vega,
        StyleName::Maia => Style::maia,
        StyleName::Lyra => Style::lyra,
        StyleName::Mira => Style::mira,
        StyleName::Luma => Style::luma,
        StyleName::Rhea => Style::rhea,
    }
}

/// Builds the `Style` inside its own stack frame, so `Preset::theme` never
/// holds one (a `Style` is ~100 KB in a debug build; see
/// `preset_theme_builds_on_a_small_stack`).
#[inline(never)]
fn with_style(make: fn() -> Style, palette: Palette, radius: f32) -> Theme {
    Theme::new(make(), palette, radius)
}
```

- [ ] **Step 4: Run the tests**

```bash
cargo test -p shadcn-egui --jobs 4 --test preset -- --test-threads 4
cargo test --workspace --jobs 4 -- --test-threads 4
```

Expected: all pass. If `preset_theme_builds_on_a_small_stack` aborts with
"has overflowed its stack", a `Style` value is held in a caller frame: keep
`with_style` `#[inline(never)]` and do not bind its result's style to a
local.

- [ ] **Step 5: Clippy and commit**

```bash
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
cargo clippy --workspace --all-targets --all-features --jobs 4 -- -D warnings
git add -A
git commit -m "feat: Preset resolves to a Theme

Radius follows the base colour unless chosen (Lyra and Sera force 0),
colours merge like upstream, the menu accent and menu colours apply,
and fonts and icons come from what the build bundles.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4.6: Fallback warnings, capture logger, changelog

**Files:**
- Create: `crates/shadcn-egui/tests/common/logcap.rs`
- Modify: `crates/shadcn-egui/tests/common/mod.rs` (`pub mod logcap;`)
- Create: `crates/shadcn-egui/tests/preset_fallbacks.rs`
- Modify: `CHANGES.md`

**Interfaces:**
- Consumes: `Preset::theme` (Task 4.5), `fonts::files`, `icons::slots`, `LUCIDE_SLOTS`.
- Produces (index C): `common::logcap::warnings<R>(f: impl FnOnce() -> R) -> (R, Vec<String>)`. Phase 6 uses it for the Hugeicons line.

- [ ] **Step 1: The capture logger**

`crates/shadcn-egui/tests/common/logcap.rs`:

```rust
//! Captures `log` records per thread. `log` allows one global logger per
//! process and tests run on parallel threads, so each test reads only the
//! lines its own thread logged.

use std::sync::{Mutex, Once};
use std::thread::ThreadId;

struct Capture(Mutex<Vec<(ThreadId, log::Level, String)>>);

impl log::Log for Capture {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Warn
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            let mut v = self.0.lock().unwrap_or_else(|e| e.into_inner());
            v.push((std::thread::current().id(), r.level(), r.args().to_string()));
        }
    }
    fn flush(&self) {}
}

static CAPTURE: Capture = Capture(Mutex::new(Vec::new()));
static INSTALL: Once = Once::new();

/// Run `f` and return its result with the `Warn` lines this thread logged
/// meanwhile (earlier lines of this thread are dropped first).
pub fn warnings<R>(f: impl FnOnce() -> R) -> (R, Vec<String>) {
    INSTALL.call_once(|| {
        log::set_logger(&CAPTURE).expect("logcap is the only logger in this test binary");
        log::set_max_level(log::LevelFilter::Warn);
    });
    let me = std::thread::current().id();
    CAPTURE.0.lock().unwrap_or_else(|e| e.into_inner()).retain(|(t, _, _)| *t != me);
    let r = f();
    let lines = CAPTURE
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .filter(|(t, l, _)| *t == me && *l == log::Level::Warn)
        .map(|(_, _, s)| s.clone())
        .collect();
    (r, lines)
}
```

`crates/shadcn-egui/tests/common/mod.rs`: add `pub mod logcap;` below the
module docs.

- [ ] **Step 2: Write the fallback tests**

`crates/shadcn-egui/tests/preset_fallbacks.rs`:

```rust
//! One test per row of spec §4.1: the fallback result and its exact warning.
mod common;

use common::logcap::warnings;
use shadcn_egui::components::icon::LUCIDE_SLOTS;
use shadcn_egui::{FontHeading, FontName, IconLibrary, Mode, Preset, StyleName};

/// The font and icon library these tests ask for, unless a feature bundles
/// them (then the fallback cannot happen and the test has nothing to check).
fn geist_missing() -> bool {
    let missing = shadcn_egui::fonts::files(FontName::Geist).is_none();
    if !missing {
        eprintln!("skipped: font-geist is enabled");
    }
    missing
}

fn tabler_missing() -> bool {
    let missing = shadcn_egui::icons::slots(IconLibrary::Tabler).is_none();
    if !missing {
        eprintln!("skipped: icons-tabler is enabled");
    }
    missing
}

const SERA: &str = "shadcn-egui: style \"sera\" is not available in egui; using \"nova\"";
const GEIST: &str = "shadcn-egui: font \"geist\" needs the \"font-geist\" feature; using \"inter\"";

#[test]
fn sera_falls_back_to_nova_with_square_corners() {
    let (t, lines) = warnings(|| Preset::default().style(StyleName::Sera).theme(Mode::Dark));
    assert_eq!((t.style.name, t.radius), ("nova", 0.0));
    assert_eq!(lines, [SERA]);
}

#[test]
fn a_font_without_its_feature_falls_back_to_inter() {
    if !geist_missing() {
        return;
    }
    let (t, lines) = warnings(|| Preset::default().font(FontName::Geist).theme(Mode::Light));
    assert_eq!(t.fonts.body.name, "inter");
    assert_eq!(lines, [GEIST]);
}

#[test]
fn a_heading_font_without_its_feature_falls_back_to_inter() {
    if !geist_missing() {
        return;
    }
    let (t, lines) = warnings(|| Preset::default().font_heading(FontHeading::Font(FontName::Geist)).theme(Mode::Light));
    assert_eq!(t.fonts.heading.name, "inter");
    assert_eq!(lines, ["shadcn-egui: heading font \"geist\" needs the \"font-geist\" feature; using \"inter\""]);
}

#[test]
fn an_icon_library_without_its_feature_falls_back_to_lucide() {
    if !tabler_missing() {
        return;
    }
    let (t, lines) = warnings(|| Preset::default().icons(IconLibrary::Tabler).theme(Mode::Light));
    assert_eq!(t.icons, LUCIDE_SLOTS);
    assert_eq!(lines, ["shadcn-egui: icon library \"tabler\" needs the \"icons-tabler\" feature; using \"lucide\""]);
}

#[test]
fn two_unsupported_values_log_two_lines_and_every_call_repeats_them() {
    if !geist_missing() {
        return;
    }
    let p = Preset::default().style(StyleName::Sera).font(FontName::Geist);
    let (_, first) = warnings(|| p.theme(Mode::Light));
    assert_eq!(first, [SERA, GEIST]);
    let (_, toggled) = warnings(|| {
        let _ = p.theme(Mode::Light);
        p.theme(Mode::Dark)
    });
    assert_eq!(toggled, [SERA, GEIST, SERA, GEIST], "each theme() call warns again");
}

#[test]
fn a_supported_preset_logs_nothing() {
    let (_, lines) = warnings(|| Preset::default().theme(Mode::Dark));
    assert!(lines.is_empty(), "{lines:?}");
}
```

- [ ] **Step 3: Run them**

```bash
cargo test -p shadcn-egui --jobs 4 --test preset_fallbacks -- --test-threads 4
cargo test -p shadcn-egui --all-features --jobs 4 --test preset_fallbacks -- --test-threads 4 --nocapture
```

Expected: all pass in both feature sets; in this phase nothing is skipped
(Geist and Tabler have no crates yet). From Phases 6/7 on the
`--all-features` run prints the `skipped:` lines.

- [ ] **Step 4: Changelog**

`CHANGES.md`, `## [Unreleased]` → `### Added`:

```markdown
- `Preset::from_code("b…")` reads a preset code copied from ui.shadcn.com
  or shadcn-svelte.com, and `preset.apply(ctx, Mode::Dark)` sets up the app
  with its style, base colour, theme, chart colour, fonts, icon library,
  radius and menu colours. `Preset::default()` is the site's default (Vega,
  neutral, Inter, Lucide); `to_code()` gives the code back.
- When a preset asks for something this build cannot show (the Sera style,
  or a font or icon library whose Cargo feature is off), the app uses Nova,
  Inter or Lucide instead and logs a warning such as
  `shadcn-egui: font "geist" needs the "font-geist" feature; using "inter"`.
- With the Lyra or Sera style every corner is square, whatever radius the
  preset names, as on the shadcn site.
```

`### Changed`:

```markdown
- `Theme` has the new public fields `icons` and `menu`; set them with
  `Theme::icons(..)` and `Theme::menu(..)`. Checkbox, Select and Dialog draw
  the icons of the theme.
- `Button::icon` and `Button::icon_end` accept any icon reference, not only
  constants.
```

- [ ] **Step 5: Phase gates and commit**

Run every gate from the index (both feature sets, clippy, tools, drift).

```bash
git add -A
git commit -m "test: preset fallbacks log one warning per unsupported value

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
