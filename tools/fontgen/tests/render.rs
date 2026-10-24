//! Unit tests of parsing and rendering on small hand-written inputs.

use fontgen::*;

const HEADER: &str = "name\ttitle\tkind\tlicense\tfiles\tlicense_file\ty_offset_factor\n";

fn tsv(rows: &str) -> Vec<Row> {
    parse_fonts_tsv(&format!("{HEADER}{rows}")).expect("valid FONTS.tsv")
}

fn sample() -> Vec<Row> {
    tsv("inter\tInter\tsans\tOFL-1.1\tofl/inter/Inter[opsz,wght].ttf\tofl/inter/OFL.txt\t0.1\n\
         roboto\tRoboto\tsans\tApache-2.0\tapache/roboto/Roboto[wdth,wght].ttf\tapache/roboto/LICENSE.txt\t-\n\
         ubu\tUbu\tsans\tUFL-1.0\tufl/ubu/Ubu.ttf\tufl/ubu/UFL.txt\t-\n\
         instrument-serif\tInstrument Serif\tserif\tOFL-1.1\t400=ofl/instrumentserif/InstrumentSerif-Regular.ttf\tofl/instrumentserif/OFL.txt\t0.05\n")
}

#[test]
fn variants_follow_the_font_name_enum() {
    assert_eq!(variant("inter"), "Inter");
    assert_eq!(variant("source-sans-3"), "SourceSans3");
    assert_eq!(variant("eb-garamond"), "EbGaramond");
    assert_eq!(variant("ibm-plex-sans"), "IbmPlexSans");
    assert_eq!(variant("jetbrains-mono"), "JetbrainsMono");
}

#[test]
fn rows_parse_variable_static_and_unset_correction() {
    let rows = sample();
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows[0].files,
        Files::Variable("ofl/inter/Inter[opsz,wght].ttf".into())
    );
    assert_eq!(rows[0].y_offset_factor, Some(0.1));
    assert_eq!(
        rows[1].y_offset_factor, None,
        "`-` means no baseline correction set yet"
    );
    assert_eq!(
        rows[3].files,
        Files::Static(vec![(
            400,
            "ofl/instrumentserif/InstrumentSerif-Regular.ttf".into()
        )])
    );
    assert_eq!(rows[3].kind, Kind::Serif);
    assert_eq!(
        rows[3].targets(),
        vec![(
            "fonts/instrument-serif-400.ttf".to_string(),
            "ofl/instrumentserif/InstrumentSerif-Regular.ttf".to_string()
        )]
    );
    assert_eq!(rows[0].targets()[0].0, "fonts/inter.ttf");
}

#[test]
fn malformed_rows_are_errors() {
    assert!(parse_fonts_tsv("name\ttitle\n")
        .unwrap_err()
        .contains("header"));
    let e = parse_fonts_tsv(&format!("{HEADER}inter\tInter\tsans\n")).unwrap_err();
    assert!(e.contains("line 2") && e.contains("7 columns"), "{e}");
    let e =
        parse_fonts_tsv(&format!("{HEADER}x\tX\tsans\tOFL-1.1\t700=a.ttf\tl\t-\n")).unwrap_err();
    assert!(e.contains("400"), "a static font needs a 400 file: {e}");
    let e =
        parse_fonts_tsv(&format!("{HEADER}x\tX\tsans\tOFL-1.1\t450=a.ttf\tl\t-\n")).unwrap_err();
    assert!(e.contains("450"), "{e}");
    let e = parse_fonts_tsv(&format!("{HEADER}x\tX\tcursive\tOFL-1.1\ta.ttf\tl\t-\n")).unwrap_err();
    assert!(e.contains("cursive"), "{e}");
}

#[test]
fn wght_range_rejects_non_truetype() {
    let e = wght_range(b"<!DOCTYPE html><html>404</html>").unwrap_err();
    assert!(e.contains("TrueType"), "{e}");
    assert!(check_truetype(b"").is_err());
    assert!(
        check_truetype(b"OTTO\0\0\0\0").is_err(),
        "CFF fonts are not expected from google/fonts TTF paths"
    );
}

#[test]
fn wght_axis_rejects_min_above_max() {
    assert_eq!(wght_axis(100.0, 900.0), Ok((100.0, 900.0)));
    assert_eq!(wght_axis(400.0, 400.0), Ok((400.0, 400.0)));
    let e = wght_axis(900.0, 100.0).unwrap_err();
    assert!(e.contains("900") && e.contains("100"), "{e}");
    assert!(wght_axis(f32::NAN, 900.0).is_err());
}

#[test]
fn unlicensed_fonts_get_no_crate_and_no_arm() {
    let rows = sample();
    assert!(!rows[2].shipped());
    assert!(crate_lib_rs(&rows[2], None)
        .unwrap_err()
        .contains("UFL-1.0"));
    let reg = registry_rs(&rows, "abc123").unwrap();
    assert!(!reg.contains("FontName::Ubu =>"), "no registry arm: {reg}");
    assert!(
        reg.contains("pub(crate) const UNLICENSED: &[FontName] = &[FontName::Ubu];"),
        "listed for the licence warning: {reg}"
    );
    let block = features_block(&rows);
    assert!(!block.contains("ubu"), "{block}");
    assert!(!workspace_deps_block(&rows, "0.4.0").contains("ubu"));
}

#[test]
fn apache_and_static_crates_render() {
    let rows = sample();
    let roboto = crate_lib_rs(&rows[1], Some((100.0, 900.0))).unwrap();
    assert!(roboto.starts_with(
        "//! Roboto for shadcn-egui. The font is licensed under the Apache License\n//! 2.0 (`LICENSE-FONT.txt`); this crate's code is MIT.\n"
    ), "{roboto}");
    assert!(
        roboto.contains("y_offset_factor: 0.0,"),
        "an unset correction renders as 0.0: {roboto}"
    );
    assert!(crate_cargo_toml(&rows[1]).contains("license = \"MIT AND Apache-2.0\"\n"));
    assert!(
        crate_cargo_toml(&rows[1]).contains(
            "repository.workspace = true\ndocumentation = \"https://docs.rs/shadcn-egui-font-roboto\"\nreadme = \"README.md\"\nkeywords = [\"egui\", \"shadcn\", \"font\", \"roboto\"]\ncategories = [\"gui\"]\ninclude = [\"src/**\", \"fonts/**\", \"LICENSE*\", \"README*\"]\n"
        ),
        "{}",
        crate_cargo_toml(&rows[1])
    );
    let serif = crate_lib_rs(&rows[3], None).unwrap();
    assert!(
        serif.contains("/// Instrument Serif, static weights 400.\n"),
        "{serif}"
    );
    assert!(
        serif.contains("regular: include_bytes!(\"../fonts/instrument-serif-400.ttf\")"),
        "{serif}"
    );
    assert!(
        serif.contains("medium: None") && serif.contains("bold: None"),
        "{serif}"
    );
    assert!(serif.contains("kind: FontKind::Serif"), "{serif}");
    assert!(
        crate_lib_rs(&rows[3], Some((100.0, 900.0))).is_err(),
        "static font with a wght range"
    );
    assert!(
        crate_lib_rs(&rows[1], None).is_err(),
        "variable font without a wght range"
    );
}

#[test]
fn registry_gates_every_font_but_inter_behind_its_feature() {
    let reg = registry_rs(&sample(), "abc123").unwrap();
    assert!(reg.starts_with("// @generated by tools/fontgen"), "{reg}");
    assert!(reg.contains("google/fonts abc123"), "{reg}");
    assert!(
        reg.contains("FontName::Inter => Some(&shadcn_egui_font_inter::FONT),"),
        "{reg}"
    );
    assert!(
        !reg.contains("feature = \"font-inter\""),
        "Inter is always there: {reg}"
    );
    assert!(reg.contains("#[cfg(feature = \"font-roboto\")]\n        FontName::Roboto => Some(&shadcn_egui_font_roboto::FONT),"), "{reg}");
    assert!(
        reg.contains(
            "FontName::InstrumentSerif => Some(&shadcn_egui_font_instrument_serif::FONT),"
        ),
        "{reg}"
    );
    assert!(reg.contains("_ => None,"), "{reg}");
}

#[test]
fn feature_block_opens_dependency_subtables_then_features() {
    let block = features_block(&sample());
    assert!(
        block.starts_with("# BEGIN fontgen\n") && block.ends_with("# END fontgen\n"),
        "{block}"
    );
    assert!(
        !block.contains("shadcn-egui-font-inter"),
        "Inter is a normal dependency: {block}"
    );
    assert!(
        block.contains(
            "[dependencies.shadcn-egui-font-roboto]\nworkspace = true\noptional = true\n"
        ),
        "{block}"
    );
    let deps_end = block.rfind("[dependencies.").unwrap();
    let features = block.find("\n[features]\n").expect("features header");
    assert!(
        deps_end < features,
        "dependency subtables come first: {block}"
    );
    assert!(
        block.contains("font-roboto = [\"dep:shadcn-egui-font-roboto\"]\n"),
        "{block}"
    );
    assert!(
        block.contains("font-instrument-serif = [\"dep:shadcn-egui-font-instrument-serif\"]\n"),
        "{block}"
    );
}

#[test]
fn workspace_block_lists_every_shipped_crate_with_the_version() {
    let block = workspace_deps_block(&sample(), "0.4.0");
    assert!(block.contains(
        "shadcn-egui-font-inter = { path = \"crates/shadcn-egui-font-inter\", version = \"0.4.0\" }\n"
    ), "{block}");
    assert!(
        block.contains("shadcn-egui-font-roboto = { path"),
        "{block}"
    );
    assert_eq!(
        workspace_version("[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.4.0\"\nedition = \"2021\"\n").unwrap(),
        "0.4.0"
    );
    assert!(workspace_version("[package]\nversion = \"1.0.0\"\n").is_err());
}

#[test]
fn replace_block_needs_exactly_one_marker_pair() {
    let text = "a\n# BEGIN fontgen\nold\n# END fontgen\nb\n";
    assert_eq!(
        replace_block(text, "# BEGIN fontgen\nnew\n# END fontgen\n").unwrap(),
        "a\n# BEGIN fontgen\nnew\n# END fontgen\nb\n"
    );
    assert!(replace_block("a\n", "x").unwrap_err().contains("BEGIN"));
    assert!(replace_block(&format!("{text}{text}"), "x")
        .unwrap_err()
        .contains("more than one"));
}

#[test]
fn definitions_mismatch_names_the_font() {
    let ts = "export const FONT_DEFINITIONS = [\n\t{\n\t\tname: \"inter\",\n\t\ttitle: \"Inter\",\n\t\ttype: \"sans\",\n\t},\n\
              \t{\n\t\tname: \"roboto\",\n\t\ttitle: \"Roboto\",\n\t\ttype: \"sans\",\n\t},\n\
              \t{\n\t\tname: \"ubu\",\n\t\ttitle: \"Ubu\",\n\t\ttype: \"sans\",\n\t},\n\
              \t{\n\t\tname: \"instrument-serif\",\n\t\ttitle: \"Instrument Serif\",\n\t\ttype: \"serif\",\n\t\tweight: [\"400\"],\n\t},\n];\n";
    let rows = sample();
    check_definitions(&rows, ts).expect("consistent");
    let e = check_definitions(
        &rows,
        &ts.replace("title: \"Roboto\"", "title: \"Roboto Flex\""),
    )
    .unwrap_err();
    assert!(e.contains("roboto") && e.contains("Roboto Flex"), "{e}");
    let e = check_definitions(
        &rows,
        &ts.replace("weight: [\"400\"]", "weight: [\"400\", \"700\"]"),
    )
    .unwrap_err();
    assert!(e.contains("instrument-serif"), "{e}");
    let e = check_definitions(&rows[..3], ts).unwrap_err();
    assert!(
        e.contains("instrument-serif"),
        "a definition without a FONTS.tsv row: {e}"
    );
}

#[test]
fn changed_kind_names_the_font() {
    let ts = "export const FONT_DEFINITIONS = [\n\t{\n\t\tname: \"inter\",\n\t\ttitle: \"Inter\",\n\t\ttype: \"sans\",\n\t},\n\
              \t{\n\t\tname: \"roboto\",\n\t\ttitle: \"Roboto\",\n\t\ttype: \"sans\",\n\t},\n\
              \t{\n\t\tname: \"ubu\",\n\t\ttitle: \"Ubu\",\n\t\ttype: \"sans\",\n\t},\n\
              \t{\n\t\tname: \"instrument-serif\",\n\t\ttitle: \"Instrument Serif\",\n\t\ttype: \"serif\",\n\t\tweight: [\"400\"],\n\t},\n];\n";
    let changed = ts.replace("type: \"serif\"", "type: \"sans\"");
    let e = check_definitions(&sample(), &changed).unwrap_err();
    assert!(e.contains("instrument-serif") && e.contains("kind"), "{e}");
}

#[test]
fn truncated_fonts_are_rejected() {
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates");
    for name in ["inter", "oxanium"] {
        let ttf = std::fs::read(crates.join(format!("shadcn-egui-font-{name}/fonts/{name}.ttf")))
            .unwrap();
        check_truetype(&ttf).unwrap_or_else(|e| panic!("{name} whole: {e}"));
        for percent in [99, 90, 50, 10] {
            let cut = &ttf[..ttf.len() * percent / 100];
            assert!(
                check_truetype(cut).is_err(),
                "{name} cut to {percent}% accepted by check_truetype"
            );
            assert!(
                wght_range(cut).is_err(),
                "{name} cut to {percent}% accepted by wght_range"
            );
        }
    }
}

#[test]
fn fonts_all_lists_every_optional_font() {
    let block = features_block(&sample());
    assert!(
        block.contains("fonts-all = [\n    \"font-roboto\",\n    \"font-instrument-serif\",\n]\n"),
        "{block}"
    );
    let fonts_all = block.find("fonts-all").unwrap();
    assert!(
        fonts_all > block.find("\n[features]\n").unwrap(),
        "inside [features]: {block}"
    );
}
