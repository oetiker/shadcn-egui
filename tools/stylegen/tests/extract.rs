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
    let s = class_strings(src);
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
