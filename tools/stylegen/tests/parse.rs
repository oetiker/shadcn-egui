use stylegen::class::split;
use stylegen::css::parse_style_css;
use stylegen::markers::parse_markers;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture")
}

fn v(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

#[test]
fn parses_nova_markers() {
    let m = parse_style_css(&fixture("style-nova.css"));
    let get = |k: &str| {
        m.iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.clone())
            .expect(k)
    };
    assert_eq!(
        get("cn-button-size-default"),
        [
            "h-8",
            "gap-1.5",
            "px-2.5",
            "has-data-[icon=inline-end]:pr-2",
            "has-data-[icon=inline-start]:pl-2"
        ]
    );
    assert_eq!(get("cn-slider-range"), ["bg-primary"]);
    // ~420 @apply rules; most markers appear once.
    assert!(m.len() > 300, "{}", m.len());
}

#[test]
fn comments_are_not_markers() {
    let css = ".style-x {\n  /* MARK: cn-fake */\n  .cn-b .cn-a-part { @apply w-2; }\n}";
    assert_eq!(
        parse_style_css(css),
        vec![("cn-a-part".to_string(), v(&["w-2"]))]
    );
}

#[test]
fn later_rule_is_prepended() {
    // Upstream: styles[m] = `${apply} ${styles[m]}`.
    let css = ".style-x { .cn-a { @apply h-1; } .cn-b { @apply w-1; } .cn-a { @apply p-1 p-2; } }";
    assert_eq!(
        parse_style_css(css),
        vec![
            ("cn-a".to_string(), v(&["p-1", "p-2", "h-1"])),
            ("cn-b".to_string(), v(&["w-1"]))
        ]
    );
}

#[test]
fn apply_without_semicolon_does_not_swallow_next_rule() {
    let css = ".style-x {\n  .cn-a {\n    @apply h-1 w-2\n  }\n  .cn-b {\n    @apply p-1;\n  }\n}";
    assert_eq!(
        parse_style_css(css),
        vec![
            ("cn-a".to_string(), v(&["h-1", "w-2"])),
            ("cn-b".to_string(), v(&["p-1"]))
        ]
    );
}

#[test]
fn selector_lists_and_nesting() {
    let css = ".style-x { .cn-a, .cn-b > .cn-c { @apply h-1; &:hover { @apply w-1; } } }";
    let m = parse_style_css(css);
    assert_eq!(m[0], ("cn-a".to_string(), v(&["h-1"])));
    assert_eq!(m[1], ("cn-c".to_string(), v(&["h-1"])));
    // `&:hover` has no cn-* of its own, so its @apply is dropped.
    assert_eq!(m.len(), 2);
}

#[test]
fn split_respects_brackets() {
    let c = split("[&_svg:not([class*='size-'])]:size-4");
    assert_eq!(c.variants, ["[&_svg:not([class*='size-'])]"]);
    assert_eq!(c.utility, "size-4");
    let c = split("dark:hover:bg-input/50");
    assert_eq!(c.variants, ["dark", "hover"]);
    assert_eq!(c.utility, "bg-input/50");
    let c = split("[&>svg]:size-3!");
    assert!(c.important);
    assert_eq!(c.utility, "size-3");
    let c = split("hover:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]");
    assert_eq!(c.variants, ["hover"]);
}

#[test]
fn markers_file_parses() {
    let lines =
        parse_markers("# c\nbutton.rs button: cn-button cn-button-size-sm\nlayout.rs card:\n");
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].file, "button.rs");
    assert_eq!(lines[0].dir, "button");
    assert_eq!(lines[0].markers, ["cn-button", "cn-button-size-sm"]);
    assert!(lines[1].markers.is_empty());
}

#[test]
fn shipped_markers_file_parses() {
    let lines = parse_markers(
        &std::fs::read_to_string(format!("{}/markers.txt", env!("CARGO_MANIFEST_DIR"))).unwrap(),
    );
    assert_eq!(lines.len(), 15);
}

#[test]
fn lyra_unterminated_apply_is_accepted() {
    // style-lyra.css:1426 ends an @apply without `;`.
    let css = ".style-x { .cn-menu-translucent { @apply relative bg-popover/70 **:data-[variant=destructive]:**:text-accent-foreground! }\n .cn-after { @apply p-1; } }";
    let m = parse_style_css(css);
    assert_eq!(m[1], ("cn-after".to_string(), v(&["p-1"])));
}
