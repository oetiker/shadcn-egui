//! shadcn/ui `packages/registry/src/preset/preset.ts` (lists, fields,
//! `V1_CHART_COLOR_MAP`) + `apps/v4/registry/config.ts` (`RADII` values)
//! → `crates/shadcn-egui/src/preset/generated.rs`. Enum order is the
//! upstream list order: it is the index inside a preset code.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PresetLists {
    pub styles: Vec<String>,
    pub base_colors: Vec<String>,
    pub themes: Vec<String>,
    pub fonts: Vec<String>,
    /// `PRESET_RADII` keys with the `config.ts` `RADII` value in rem
    /// (`None` for `default`, whose value is `""` upstream).
    pub radii: Vec<(String, Option<f32>)>,
    pub menu_accents: Vec<String>,
    pub menu_colors: Vec<String>,
    /// `PRESET_ICON_LIBRARIES` (index §J; not the `libraries.ts` key order).
    pub icon_libraries: Vec<String>,
    /// `V1_CHART_COLOR_MAP`: (theme, chart colour) for `a` codes.
    pub v1_chart_colors: Vec<(String, String)>,
    /// `(key, bits)`, low bits first.
    pub fields_v1: Vec<(String, u32)>,
    pub fields_v2: Vec<(String, u32)>,
}

/// From `marker` to the bracket closing the first `[` or `{` after it.
fn block<'a>(src: &'a str, marker: &str) -> Result<&'a str, String> {
    let at = src
        .find(marker)
        .ok_or_else(|| format!("`{marker}` not found"))?;
    let rest = &src[at..];
    let open = marker.len()
        + rest[marker.len()..]
            .find(['[', '{'])
            .ok_or_else(|| format!("`{marker}`: no bracket"))?;
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
    Ok(body
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect())
}

/// `key: "value",` lines of an object literal, as (key, value).
fn pairs(src: &str, marker: &str) -> Result<Vec<(String, String)>, String> {
    let b = block(src, marker)?;
    Ok(b[marker.len()..]
        .lines()
        .filter_map(|l| l.trim().strip_suffix(','))
        .filter_map(|l| l.split_once(": \""))
        .map(|(k, v)| {
            (
                k.trim_matches('"').to_owned(),
                v.trim_end_matches('"').to_owned(),
            )
        })
        .collect())
}

fn between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = s.find(start)? + start.len();
    Some(&s[i..i + s[i..].find(end)?])
}

fn fields(src: &str, marker: &str) -> Result<Vec<(String, String, u32)>, String> {
    let mut out = Vec::new();
    for line in block(src, marker)?
        .lines()
        .filter(|l| l.contains("key: \""))
    {
        let bad = || format!("{marker}: cannot read `{}`", line.trim());
        let key = between(line, "key: \"", "\"").ok_or_else(bad)?;
        let values = between(line, "values: ", ",").ok_or_else(bad)?;
        let bits = between(line, "bits: ", " ")
            .ok_or_else(bad)?
            .parse::<u32>()
            .map_err(|_| bad())?;
        out.push((key.to_owned(), values.to_owned(), bits));
    }
    Ok(out)
}

/// Each field key and the list upstream packs for it.
const FIELD_VALUES: [(&str, &str); 10] = [
    ("menuColor", "PRESET_MENU_COLORS"),
    ("menuAccent", "PRESET_MENU_ACCENTS"),
    ("radius", "PRESET_RADII"),
    ("font", "PRESET_FONTS"),
    ("iconLibrary", "PRESET_ICON_LIBRARIES"),
    ("theme", "PRESET_THEMES"),
    ("baseColor", "PRESET_BASE_COLORS"),
    ("style", "PRESET_STYLES"),
    ("chartColor", "PRESET_CHART_COLORS"),
    ("fontHeading", "PRESET_FONT_HEADINGS"),
];

fn require(src: &str, text: &str) -> Result<(), String> {
    src.contains(text)
        .then_some(())
        .ok_or_else(|| format!("preset.ts no longer contains `{text}`"))
}

/// `config.ts` `RADII` (`{ name: "small", label: "Small", value: "0.45rem" },`)
/// as (name, rem): `""` is `None` (`default`), `"0"` is 0.
fn radii(config_ts: &str) -> Result<Vec<(String, Option<f32>)>, String> {
    let mut out = Vec::new();
    for line in block(config_ts, "export const RADII =")?
        .lines()
        .filter(|l| l.contains("name: \""))
    {
        let bad = || format!("RADII: cannot read `{}`", line.trim());
        let name = between(line, "name: \"", "\"").ok_or_else(bad)?;
        let value = between(line, "value: \"", "\"").ok_or_else(bad)?;
        let rem = match value {
            "" => None,
            v => Some(
                v.strip_suffix("rem")
                    .unwrap_or(v)
                    .parse::<f32>()
                    .map_err(|_| bad())?,
            ),
        };
        out.push((name.to_owned(), rem));
    }
    Ok(out)
}

pub fn parse_preset(preset_ts: &str, config_ts: &str) -> Result<PresetLists, String> {
    // The derived lists the generated code relies on.
    require(
        preset_ts,
        "export const PRESET_FONT_HEADINGS = [\"inherit\", ...PRESET_FONTS] as const",
    )?;
    require(
        preset_ts,
        "export const PRESET_CHART_COLORS = PRESET_THEMES\n",
    )?;
    let radius_keys = strings(preset_ts, "export const PRESET_RADII =")?;
    let radii = radii(config_ts)?;
    if radii
        .iter()
        .map(|r| r.0.as_str())
        .ne(radius_keys.iter().map(String::as_str))
    {
        return Err(format!(
            "config.ts RADII {:?} differ from PRESET_RADII {radius_keys:?}",
            radii.iter().map(|r| &r.0).collect::<Vec<_>>()
        ));
    }
    let mut l = PresetLists {
        styles: strings(preset_ts, "export const PRESET_STYLES =")?,
        base_colors: strings(preset_ts, "export const PRESET_BASE_COLORS =")?,
        themes: strings(preset_ts, "export const PRESET_THEMES =")?,
        fonts: strings(preset_ts, "export const PRESET_FONTS =")?,
        radii,
        menu_accents: strings(preset_ts, "export const PRESET_MENU_ACCENTS =")?,
        menu_colors: strings(preset_ts, "export const PRESET_MENU_COLORS =")?,
        icon_libraries: strings(preset_ts, "export const PRESET_ICON_LIBRARIES =")?,
        v1_chart_colors: pairs(preset_ts, "export const V1_CHART_COLOR_MAP")?,
        ..PresetLists::default()
    };
    for (theme, chart) in &l.v1_chart_colors {
        if !l.themes.contains(theme) || !l.themes.contains(chart) {
            return Err(format!(
                "V1_CHART_COLOR_MAP: {theme} -> {chart} is not a pair of themes"
            ));
        }
    }
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
        let want = FIELD_VALUES
            .iter()
            .find(|f| f.0 == key)
            .map(|f| f.1)
            .unwrap_or_default();
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
            return Err(format!(
                "field {key}: {len} values do not fit in {bits} bits"
            ));
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
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                .unwrap_or_default()
        })
        .collect()
}

fn emit_enum(s: &mut String, name: &str, doc: &str, ids: &[String]) {
    *s += &format!(
        "/// {doc}\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n#[non_exhaustive]\npub enum {name} {{\n"
    );
    for id in ids {
        *s += &format!("    /// `\"{id}\"`\n    {},\n", pascal(id));
    }
    *s += &format!("}}\n\nimpl {name} {{\n    /// Every value, in upstream list order (the index in a preset code).\n");
    *s += &format!(
        "    pub const ALL: [Self; {}] = [{}];\n\n",
        ids.len(),
        ids.iter()
            .map(|i| format!("Self::{}", pascal(i)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    *s += "    /// The upstream id.\n    pub const fn id(self) -> &'static str {\n        match self {\n";
    for id in ids {
        *s += &format!("            Self::{} => \"{id}\",\n", pascal(id));
    }
    *s += "        }\n    }\n\n    /// The value with this upstream id.\n    pub fn from_id(id: &str) -> Option<Self> {\n";
    *s += "        Self::ALL.into_iter().find(|v| v.id() == id)\n    }\n}\n\n";
}

pub fn emit_preset(pin: &str, l: &PresetLists) -> String {
    let mut s = crate::emit::header_react(
        pin,
        "`packages/registry/src/preset/preset.ts` and `apps/v4/registry/config.ts`",
    );
    emit_enum(
        &mut s,
        "StyleName",
        "A shadcn style (upstream `PRESET_STYLES`).",
        &l.styles,
    );
    emit_enum(
        &mut s,
        "BaseColor",
        "A base colour (upstream `PRESET_BASE_COLORS`).",
        &l.base_colors,
    );
    emit_enum(
        &mut s,
        "ThemeColor",
        "A colour theme, also used for chart colours (upstream `PRESET_THEMES`).",
        &l.themes,
    );
    emit_enum(
        &mut s,
        "FontName",
        "A font (upstream `PRESET_FONTS`).",
        &l.fonts,
    );
    emit_enum(
        &mut s,
        "IconLibrary",
        "An icon library (upstream `PRESET_ICON_LIBRARIES`).",
        &l.icon_libraries,
    );
    emit_enum(
        &mut s,
        "MenuColor",
        "Menu content colours (upstream `PRESET_MENU_COLORS`).",
        &l.menu_colors,
    );
    emit_enum(
        &mut s,
        "MenuAccent",
        "Menu highlight (upstream `PRESET_MENU_ACCENTS`).",
        &l.menu_accents,
    );
    // RadiusChoice: the coded radii plus `Px`, which has no code.
    s += "/// A radius (upstream `PRESET_RADII`), or `Px` for any other radius.\n";
    s += "#[derive(Clone, Copy, Debug, PartialEq)]\n#[non_exhaustive]\npub enum RadiusChoice {\n";
    for (id, rem) in &l.radii {
        match rem {
            Some(r) => s += &format!("    /// `\"{id}\"` ({r:?}rem)\n    {},\n", pascal(id)),
            None => {
                s += &format!(
                    "    /// `\"{id}\"` (the base colour's own radius)\n    {},\n",
                    pascal(id)
                )
            }
        }
    }
    s += "    /// Any radius in logical px; such a preset has no code.\n    Px(f32),\n}\n\nimpl RadiusChoice {\n";
    s += "    /// The coded values, in upstream list order.\n";
    s += &format!(
        "    pub const ALL: [Self; {}] = [{}];\n\n",
        l.radii.len(),
        l.radii
            .iter()
            .map(|(i, _)| format!("Self::{}", pascal(i)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    s += "    /// The upstream id; `None` for `Px`.\n    pub const fn id(self) -> Option<&'static str> {\n        match self {\n";
    for (id, _) in &l.radii {
        s += &format!("            Self::{} => Some(\"{id}\"),\n", pascal(id));
    }
    s += "            Self::Px(_) => None,\n        }\n    }\n\n";
    s += "    /// The upstream value in rem; `None` for `Default` (the base colour's\n    /// radius) and for `Px`.\n    pub const fn rem(self) -> Option<f32> {\n        match self {\n";
    for (id, rem) in &l.radii {
        s += &format!("            Self::{} => {rem:?},\n", pascal(id));
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
    for (name, version, fields) in [
        ("FIELDS_V1", "a", &l.fields_v1),
        ("FIELDS_V2", "b", &l.fields_v2),
    ] {
        s += &format!("/// Version `{version}` codes: (field, bits), low bits first.\n");
        s += &format!(
            "pub(crate) const {name}: [(Field, u32); {}] = [\n",
            fields.len()
        );
        for (k, b) in fields {
            s += &format!("    (Field::{}, {b}),\n", pascal(k));
        }
        s += "];\n\n";
    }
    s += "/// Upstream `V1_CHART_COLOR_MAP`: the chart colour of an `a` code, by theme.\n";
    s += &format!(
        "pub(crate) const V1_CHART_COLORS: [(ThemeColor, ThemeColor); {}] = [\n",
        l.v1_chart_colors.len()
    );
    for (theme, chart) in &l.v1_chart_colors {
        s += &format!(
            "    (ThemeColor::{}, ThemeColor::{}),\n",
            pascal(theme),
            pascal(chart)
        );
    }
    s += "];\n\n";
    s.truncate(s.trim_end().len());
    s.push('\n');
    s
}
