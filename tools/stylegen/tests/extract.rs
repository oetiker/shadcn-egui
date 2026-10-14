use stylegen::extract::{class_strings, part_strings, BRANCHES};
use stylegen::markers::parse_markers;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn ui() -> std::path::PathBuf {
    std::path::Path::new(ROOT).join("tests/fixtures/ui")
}

fn part(name: &str) -> String {
    let lines = parse_markers(&std::fs::read_to_string(format!("{ROOT}/markers.txt")).unwrap());
    let parts = part_strings(&ui(), &lines).expect("parts");
    parts
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("{name}"))
        .1
        .join(" ")
}

#[test]
fn reads_every_string_form() {
    let src = r#"
<script lang="ts" module>
	export const v = tv({
		base: "cn-x-base a b",
		variants: { size: { sm: "cn-x-sm c", "icon-xs": "cn-x-icon d" } },
		defaultVariants: { size: "sm" },
	});
</script>
<!-- parity-ignore: data-[x]:hidden | not a class string -->
<div class={cn(
	"cn-x e [&_svg:not([class*='size-'])]:size-4",
	// parity-ignore: a comment with "quotes"
	"f h-[calc(100%-1px)]",
	arrowClasses,
	className
)}></div>
<Button variant="ghost" class="cn-x-close g" />
"#;
    let s = class_strings(src).unwrap();
    assert!(s.contains(&"cn-x-base a b".to_string()), "{s:?}");
    assert!(s.contains(&"cn-x-sm c".to_string()), "{s:?}");
    assert!(s.contains(&"cn-x-icon d".to_string()), "{s:?}");
    assert!(
        s.contains(&"cn-x e [&_svg:not([class*='size-'])]:size-4 f h-[calc(100%-1px)]".to_string()),
        "{s:?}"
    );
    assert!(s.contains(&"cn-x-close g".to_string()), "{s:?}");
    assert!(!s.iter().any(|x| x.contains("data-[x]:hidden")), "{s:?}");
    assert!(!s.iter().any(|x| x.contains("quotes")), "{s:?}");
}

#[test]
fn tabs_trigger_joins_its_four_cn_arguments() {
    let t = part("cn-tabs-trigger");
    assert!(
        t.starts_with("cn-tabs-trigger relative inline-flex h-[calc(100%-1px)]"),
        "{t}"
    );
    assert!(
        t.ends_with("group-data-[variant=line]/tabs-list:data-active:after:opacity-100"),
        "{t}"
    );
}

#[test]
fn tv_variant_strings_are_their_own_parts() {
    assert_eq!(
        part("cn-tabs-list-variant-line"),
        "cn-tabs-list-variant-line gap-1 bg-transparent"
    );
    assert!(part("cn-button").starts_with("cn-button group/button inline-flex"));
    assert_eq!(part("cn-button-variant-ghost"), "cn-button-variant-ghost");
}

#[test]
fn cli_markers_are_removed() {
    assert_eq!(part("cn-card-title"), "cn-card-title");
    assert!(!part("cn-select-content").contains("cn-menu-"));
    assert!(!part("cn-popover-content").contains("-logical"));
}

#[test]
fn nested_component_class_is_the_part() {
    assert_eq!(part("cn-dialog-close"), "cn-dialog-close");
    assert_eq!(
        part("cn-select-item-indicator-icon"),
        "cn-select-item-indicator-icon pointer-events-none"
    );
}

#[test]
fn slot_part_is_the_marker_less_string_of_its_file() {
    let lines = parse_markers("separator.rs separator: slot:separator\n");
    let parts = part_strings(&ui(), &lines).expect("slot part");
    assert_eq!(
        parts[0].1.join(" "),
        "shrink-0 bg-border data-horizontal:h-px data-horizontal:w-full data-vertical:w-px data-vertical:self-stretch"
    );
}

/// The decision in the plan: input.rs draws a text field, so the `{:else}`
/// branch of input.svelte (occurrence 1 of cn-input) is the part.
#[test]
fn input_takes_the_else_branch() {
    assert!(BRANCHES
        .iter()
        .any(|(f, m, k, _)| *f == "input/input.svelte" && *m == "cn-input" && *k == 1));
    let src = std::fs::read_to_string(ui().join("input/input.svelte")).unwrap();
    let hits: Vec<String> = class_strings(&src)
        .unwrap()
        .into_iter()
        .filter(|s| s.split_whitespace().any(|t| t == "cn-input"))
        .collect();
    assert_eq!(hits.len(), 2, "{hits:?}");
    let else_at = src.find("{:else}").unwrap();
    let second = src.rfind("\"cn-input ").unwrap();
    assert!(
        second > else_at,
        "occurrence 1 lies in the {{:else}} branch"
    );
    assert_eq!(part("cn-input"), hits[1]);
}

#[test]
fn branch_choice_picks_the_named_occurrence_and_unnamed_duplicates_fail() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("stylegen-extract-{}", std::process::id()));
    let input = dir.join("input");
    std::fs::create_dir_all(&input).unwrap();
    std::fs::write(
        input.join("input.svelte"),
        "{#if file}<input class=\"cn-input only-file\" />{:else}<input class=\"cn-input only-text\" />{/if}",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("label")).unwrap();
    std::fs::write(
        dir.join("label/label.svelte"),
        "<a class=\"cn-label one\" /><a class=\"cn-label two\" />",
    )
    .unwrap();
    let ok = part_strings(&dir, &parse_markers("input.rs input: cn-input\n")).unwrap();
    assert_eq!(ok[0].1.join(" "), "cn-input only-text");
    let err = part_strings(&dir, &parse_markers("label.rs label: cn-label\n")).unwrap_err();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(err.contains("BRANCHES"), "{err}");
}

#[test]
fn every_shipped_part_has_a_string() {
    let lines = parse_markers(&std::fs::read_to_string(format!("{ROOT}/markers.txt")).unwrap());
    let parts = part_strings(&ui(), &lines).expect("every part found");
    let shipped: usize = lines.iter().map(|l| l.markers.len()).sum();
    assert_eq!(parts.len(), shipped);
}

#[test]
fn unevaluable_class_syntax_is_an_error() {
    let cases = [
        r#"<div class={cn("cn-a x", open && "y")}></div>"#,
        r#"<div class={cn("cn-a x", {"z": on})}></div>"#,
        r#"<div class={cn("cn-a x", big ? "p" : "q")}></div>"#,
        "<div class={cn(`cn-a w-${n}`)}></div>",
        r#"<div class="cn-a {extra} x"></div>"#,
    ];
    for c in cases {
        assert!(class_strings(c).is_err(), "{c}");
    }
}

#[test]
fn comments_hide_keys_and_calls() {
    let tv = "const v = tv({\n// base: \"wrong\"\nbase: \"cn-r right\",\n});";
    assert_eq!(class_strings(tv).unwrap(), vec!["cn-r right".to_string()]);
    let call = "// see cn(\"cn-f ghost\")\n<div class={cn(\"cn-f real\", className)}></div>";
    assert_eq!(class_strings(call).unwrap(), vec!["cn-f real".to_string()]);
}

/// Kept as an error rather than stripped: no upstream `cn(` holds a block
/// comment, and the message names what the extractor stopped at.
#[test]
fn block_comment_in_cn_is_an_error() {
    let err =
        class_strings("<div class={cn(\"cn-a x\", /* note */ className)}></div>").unwrap_err();
    assert_eq!(
        err,
        "cn() argument the extractor cannot evaluate: /* note */ className"
    );
}

#[test]
fn unread_tv_and_class_forms_are_errors() {
    let slots = "const v = tv({ slots: { base: \"cn-s a\", icon: \"cn-s-icon b\" } });";
    let err = class_strings(slots).unwrap_err();
    assert!(err.contains("slots"), "{err}");
    for c in [
        r#"<div class={"cn-a x"}></div>"#,
        "<div class={ 'cn-a x' }></div>",
        "<div class='cn-a x'></div>",
    ] {
        let err = class_strings(c).unwrap_err();
        assert!(err.contains("class"), "{c}: {err}");
    }
}

#[test]
fn quoted_variant_keys_are_keys() {
    let src = "const v = tv({ base: \"cn-x a\", variants: { size: { sm: \"cn-x-sm c\", \"icon-xs\": \"cn-x-icon d\" } } });";
    assert_eq!(
        class_strings(src).unwrap(),
        ["cn-x a", "cn-x-sm c", "cn-x-icon d"]
    );
}

#[test]
fn apostrophe_in_markup_text_opens_no_string() {
    let src = "<p>it's</p> // cn(\"cn-z ghost\")\n<div class={cn(\"cn-z real\")}></div>";
    assert_eq!(class_strings(src).unwrap(), ["cn-z real"]);
}

#[test]
fn tv_keys_match_as_whole_keys() {
    // `database` is not `base`: an unread key, named whole.
    let longer = "const v = tv({ database: \"cn-q wrong\", base: \"cn-q right\" });";
    let err = class_strings(longer).unwrap_err();
    assert!(
        err.contains("key the extractor does not read: database "),
        "{err}"
    );
    let quoted =
        "const v = tv({ variants: { size: { sm: \"cn-q-sm base:x\" } }, base: \"cn-q right\" });";
    // One tv keeps base first, then the variants.
    assert_eq!(
        class_strings(quoted).unwrap(),
        ["cn-q right", "cn-q-sm base:x"]
    );
}

#[test]
fn member_call_in_cn_is_an_error() {
    let err = class_strings("<div class={cn(\"cn-a x\", state.classes(open))}></div>").unwrap_err();
    assert!(err.contains("state.classes"), "{err}");
}

#[test]
fn unread_tv_keys_are_errors() {
    let compound = "const v = tv({ base: \"cn-c a\", variants: { size: { sm: \"cn-c-sm b\" } }, compoundVariants: [{ size: \"sm\", class: \"cn-c-x c\" }] });";
    let err = class_strings(compound).unwrap_err();
    assert!(err.contains("compoundVariants"), "{err}");
    // `defaultVariants` names variants and carries no classes: read as nothing.
    let defaults = "const v = tv({ base: \"cn-d a\", variants: { size: { sm: \"cn-d-sm b\" } }, defaultVariants: { size: \"sm\" } });";
    assert_eq!(class_strings(defaults).unwrap(), ["cn-d a", "cn-d-sm b"]);
}

#[test]
fn conditional_tv_values_are_errors() {
    for src in [
        "const v = tv({ base: open ? \"cn-t a\" : \"cn-t b\" });",
        "const v = tv({ base: open ? klass : \"cn-t b\" });",
        "const v = tv({ base: \"cn-t a\", variants: { size: { sm: on ? \"cn-t-sm x\" : \"cn-t-sm y\" } } });",
    ] {
        let err = class_strings(src).unwrap_err();
        assert!(err.contains("conditional"), "{src}: {err}");
    }
}

/// Item 6: Svelte 5 array and object class values and a conditional are not
/// read, so they are errors; `class=` counts only as a whole attribute name.
#[test]
fn unread_dynamic_class_values_are_errors() {
    for c in [
        r#"<div class={["cn-a x", on && "y"]}></div>"#,
        r#"<div class={{ "cn-a x": on }}></div>"#,
        r#"<div class={on ? "cn-a x" : "cn-a y"}></div>"#,
    ] {
        let err = class_strings(c).unwrap_err();
        assert!(err.contains("class"), "{c}: {err}");
    }
    // Neither a longer attribute name nor a plain value is a class attribute.
    assert_eq!(
        class_strings(r#"<div subclass='x' data-class="y" class={cn("cn-a z")}></div>"#).unwrap(),
        ["cn-a z"]
    );
    assert_eq!(
        class_strings(r#"<div class={className}></div>"#).unwrap(),
        Vec::<String>::new()
    );
}

/// Item 7: an unclosed string in a `tv` object hides every key after it,
/// so it is an error, not the end of the object.
#[test]
fn unclosed_string_in_tv_is_an_error() {
    let src = "const v = tv({ base: \"cn-u a, slots: { x: \"cn-u-x b\" } });";
    let err = class_strings(src).unwrap_err();
    assert!(err.contains("does not close"), "{err}");
    let cn = "<div class={cn(\"cn-u a)}></div>";
    let err = class_strings(cn).unwrap_err();
    assert!(err.contains("does not close"), "{err}");
}

/// Item 8: a `//` left unmasked (after `:`, a URL) is no comment inside a
/// call either, so the call still closes.
#[test]
fn unmasked_double_slash_keeps_the_call_closed() {
    let src = "<div class={cn(\"cn-v a\", f(a:// b), \"c\")}></div>";
    assert_eq!(class_strings(src).unwrap(), ["cn-v a c"]);
}

/// Item 10: a template literal spans lines; a `//` in it is text.
#[test]
fn double_slash_in_a_multi_line_template_is_text() {
    let src = "<div class={cn(`cn-w a\n  http://x b // c`, \"d\")}></div>";
    assert_eq!(class_strings(src).unwrap(), ["cn-w a http://x b // c d"]);
}

/// Item 13: shorthand, spread and computed `tv` keys hide what they hold.
#[test]
fn unevaluable_tv_keys_are_errors() {
    for src in [
        "const v = tv({ base, variants: { size: { sm: \"cn-k-sm a\" } } });",
        "const v = tv({ ...shared, base: \"cn-k a\" });",
        "const v = tv({ [key]: \"cn-k b\", base: \"cn-k a\" });",
    ] {
        assert!(
            class_strings(src).is_err(),
            "{src}: {:?}",
            class_strings(src)
        );
    }
}

/// A backtick in markup text opens no template literal, so a `//`
/// comment on a later line is still a comment. In `<script>` and in a
/// `{..}` expression a template literal runs on across lines.
#[test]
fn backtick_in_markup_text_opens_no_string() {
    let src = "<p>a ` b</p>\n// cn(\"cn-z ghost\")\n<div class={cn(\"cn-z real\")}></div>";
    assert_eq!(class_strings(src).unwrap(), ["cn-z real"]);
    let script = "<script>\nconst v = tv({ base: `cn-w a\n// b` });\n</script>";
    assert_eq!(class_strings(script).unwrap(), ["cn-w a // b"]);
    let expr = "<div class={cn(`cn-y a\n// b`)}></div>";
    assert_eq!(class_strings(expr).unwrap(), ["cn-y a // b"]);
}

/// A `?` outside strings in a braced class value is a conditional, also
/// inside a call's arguments: the call's value depends on runtime state.
#[test]
fn question_mark_in_a_class_call_is_an_error() {
    for c in [
        "<div class={f(a?.b)}></div>",
        r#"<div class={f(on ? "cn-a x" : "cn-a y")}></div>"#,
    ] {
        let err = class_strings(c).unwrap_err();
        assert!(err.contains("class"), "{c}: {err}");
    }
}
