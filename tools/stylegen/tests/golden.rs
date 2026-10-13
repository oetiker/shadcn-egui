use stylegen::css::parse_style_css;
use stylegen::extract::part_strings;
use stylegen::markers::parse_markers;
use stylegen::model::flag;
use stylegen::model::{Field, Value};
use stylegen::resolve::{collect_vars, parts, translate, Marker, Part, Vars};

fn style(name: &str) -> Vec<(String, Vec<String>)> {
    let p = format!(
        "{}/tests/fixtures/style-{name}.css",
        env!("CARGO_MANIFEST_DIR")
    );
    parse_style_css(&std::fs::read_to_string(p).expect("fixture"))
}

fn marker(css: &[(String, Vec<String>)], m: &str) -> Marker {
    let vars = collect_vars(css);
    let classes = &css.iter().find(|(n, _)| n == m).expect(m).1;
    translate(classes, &vars)
}

/// Value of `field` with no condition.
fn base(m: &Marker, field: Field) -> Option<Value> {
    m.decls
        .iter()
        .rfind(|d| d.field == field && d.all == 0 && d.none == 0 && d.rank == 0)
        .map(|d| d.value)
}
fn when(m: &Marker, field: Field, all: u32) -> Option<Value> {
    m.decls
        .iter()
        .rfind(|d| d.field == field && d.all == all)
        .map(|d| d.value)
}

#[test]
fn nova_button_default_size() {
    let css = style("nova");
    let m = marker(&css, "cn-button-size-default");
    assert_eq!(base(&m, Field::Height), Some(Value::Px(32.0)));
    assert_eq!(base(&m, Field::PadLeft), Some(Value::Px(10.0)));
    assert_eq!(when(&m, Field::PadLeft, 1 << 16), Some(Value::Px(8.0)));
    assert!(m.unsupported.is_empty(), "{:?}", m.unsupported);
}

#[test]
fn vega_button_default_is_36() {
    let m = marker(&style("vega"), "cn-button-size-default");
    assert_eq!(base(&m, Field::Height), Some(Value::Px(36.0)));
}

/// `bg-clip-padding` on `cn-button`: the fill stops inside the transparent
/// border, in both styles.
#[test]
fn button_fill_clips_to_padding_box() {
    for name in ["vega", "nova"] {
        let m = marker(&style(name), "cn-button");
        assert_eq!(
            base(&m, Field::BgClipPadding),
            Some(Value::Bool(true)),
            "{name}"
        );
    }
}

#[test]
fn nova_destructive_is_tinted() {
    let m = marker(&style("nova"), "cn-button-variant-destructive");
    assert_eq!(
        base(&m, Field::Bg),
        Some(Value::Paint {
            role: "Destructive",
            alpha: 0.1
        })
    );
    assert_eq!(
        when(&m, Field::Bg, 1),
        Some(Value::Paint {
            role: "Destructive",
            alpha: 0.2
        })
    );
}

#[test]
fn nova_slider_thumb() {
    let m = marker(&style("nova"), "cn-slider-thumb");
    assert_eq!(base(&m, Field::Width), Some(Value::Px(12.0)));
    assert_eq!(base(&m, Field::HitInsetX), Some(Value::Px(8.0)));
    assert_eq!(
        base(&m, Field::Bg),
        Some(Value::Paint {
            role: "White",
            alpha: 1.0
        })
    );
}

#[test]
fn card_spacing_var_resolves_with_size_condition() {
    let m = marker(&style("nova"), "cn-card-content");
    assert_eq!(base(&m, Field::PadLeft), Some(Value::Px(16.0)));
    assert_eq!(when(&m, Field::PadLeft, 1 << 19), Some(Value::Px(12.0)));
}

#[test]
fn later_class_wins_under_same_condition() {
    let css = vec![(
        "cn-x".to_string(),
        vec![
            "h-8".to_string(),
            "h-9".to_string(),
            "hover:h-1".to_string(),
            "hover:h-2".to_string(),
        ],
    )];
    let m = translate(&css[0].1, &collect_vars(&css));
    assert_eq!(
        m.decls.iter().filter(|d| d.field == Field::Height).count(),
        2
    );
    assert_eq!(base(&m, Field::Height), Some(Value::Px(36.0)));
    assert_eq!(when(&m, Field::Height, 1 << 1), Some(Value::Px(8.0)));
}

/// A rule with a condition must outrank the base, or `pick` could let a
/// later base beat it.
#[test]
fn every_conditional_decl_has_a_rank() {
    for s in ["vega", "nova"] {
        let css = style(s);
        let vars = collect_vars(&css);
        for (name, classes) in &css {
            for d in translate(classes, &vars).decls {
                assert!(d.all == 0 && d.none == 0 || d.rank > 0, "{s} {name}: {d:?}");
            }
        }
    }
}

/// Footer present and small size: the card drops its bottom padding.
#[test]
fn nova_card_small_with_footer_has_no_bottom_padding() {
    let m = marker(&style("nova"), "cn-card");
    assert_eq!(
        when(&m, Field::PadBottom, (1 << 19) | (1 << 25)),
        Some(Value::Px(0.0))
    );
}

fn classes(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn part_of(style_name: &str, part: &str) -> Part {
    let root = env!("CARGO_MANIFEST_DIR");
    let lines = parse_markers(&std::fs::read_to_string(format!("{root}/markers.txt")).unwrap());
    let comps = part_strings(
        &std::path::Path::new(root).join("tests/fixtures/ui"),
        &lines,
    )
    .unwrap();
    parts(&style(style_name), &comps)
        .into_iter()
        .find(|p| p.name == part)
        .unwrap_or_else(|| panic!("{part}"))
}

fn after_when(p: &Part, field: Field, all: u32) -> Option<Value> {
    p.marker
        .after
        .iter()
        .rfind(|d| d.field == field && d.all == all)
        .map(|d| d.value)
}

#[test]
fn outline_classes_set_the_outline_fields() {
    let m = translate(
        &classes(&[
            "outline-none",
            "focus-visible:outline-1",
            "focus-visible:outline-ring",
        ]),
        &Vars::new(),
    );
    assert_eq!(base(&m, Field::OutlineWidth), Some(Value::Px(0.0)));
    let fv = flag("FOCUS_VISIBLE");
    assert_eq!(when(&m, Field::OutlineWidth, fv), Some(Value::Px(1.0)));
    assert_eq!(
        when(&m, Field::OutlineColor, fv),
        Some(Value::Paint {
            role: "Ring",
            alpha: 1.0
        })
    );
    let hidden = translate(&classes(&["outline-hidden"]), &Vars::new());
    assert_eq!(base(&hidden, Field::OutlineWidth), Some(Value::Px(0.0)));
}

#[test]
fn calc_height_is_a_height_inset() {
    let m = translate(&classes(&["h-[calc(100%-1px)]"]), &Vars::new());
    assert_eq!(base(&m, Field::HeightInset), Some(Value::Px(1.0)));
    assert_eq!(base(&m, Field::Height), None);
}

#[test]
fn tab_line_indicator_is_an_after_part() {
    let p = part_of("vega", "cn-tabs-trigger");
    let (h, line, sel) = (flag("HORIZONTAL"), flag("VARIANT_LINE"), flag("SELECTED"));
    assert_eq!(base(&p.marker, Field::HeightInset), Some(Value::Px(1.0)));
    let after_base = |f: Field| {
        p.marker
            .after
            .iter()
            .rfind(|d| d.field == f && d.all == 0 && d.rank == 0)
            .map(|d| d.value)
    };
    assert_eq!(after_base(Field::Opacity), Some(Value::Factor(0.0)));
    assert_eq!(
        after_base(Field::Bg),
        Some(Value::Paint {
            role: "Foreground",
            alpha: 1.0
        })
    );
    assert_eq!(after_when(&p, Field::Height, h), Some(Value::Px(2.0)));
    assert_eq!(after_when(&p, Field::InsetBottom, h), Some(Value::Px(-5.0)));
    assert_eq!(after_when(&p, Field::InsetLeft, h), Some(Value::Px(0.0)));
    assert_eq!(
        after_when(&p, Field::Opacity, line | sel),
        Some(Value::Factor(1.0))
    );
    // The trigger's own decls hold none of these.
    assert_eq!(when(&p.marker, Field::Height, h), None);
}

#[test]
fn separator_is_a_slot_part() {
    let p = part_of("vega", "slot:separator");
    assert_eq!(
        base(&p.marker, Field::Bg),
        Some(Value::Paint {
            role: "Border",
            alpha: 1.0
        })
    );
    assert_eq!(
        when(&p.marker, Field::Height, flag("HORIZONTAL")),
        Some(Value::Px(1.0))
    );
    assert_eq!(
        when(&p.marker, Field::Width, flag("VERTICAL")),
        Some(Value::Px(1.0))
    );
    assert!(
        p.marker.unsupported.is_empty(),
        "{:?}",
        p.marker.unsupported
    );
}

#[test]
fn slider_thumb_drops_disabled_classes_by_name() {
    let p = part_of("vega", "cn-slider-thumb");
    assert!(
        !p.marker.decls.iter().any(|d| d.field == Field::Opacity),
        "{:?}",
        p.marker.decls
    );
    assert!(p
        .marker
        .ignored
        .iter()
        .any(|(c, _)| c == "disabled:opacity-50"));
}

#[test]
fn checkbox_and_switch_hit_area_come_from_the_base_classes() {
    for part in ["cn-checkbox", "cn-switch"] {
        let p = part_of("vega", part);
        assert_eq!(
            base(&p.marker, Field::HitInsetX),
            Some(Value::Px(12.0)),
            "{part}"
        );
        assert_eq!(
            base(&p.marker, Field::HitInsetY),
            Some(Value::Px(8.0)),
            "{part}"
        );
    }
}

#[test]
fn highlighted_item_and_placeholder_text_have_their_states() {
    let item = part_of("vega", "cn-select-item");
    assert_eq!(
        when(&item.marker, Field::Bg, flag("FOCUS")),
        Some(Value::Paint {
            role: "Accent",
            alpha: 1.0
        })
    );
    let input = part_of("vega", "cn-input");
    assert_eq!(
        when(&input.marker, Field::Text, flag("PLACEHOLDER")),
        Some(Value::Paint {
            role: "MutedForeground",
            alpha: 1.0
        })
    );
}

#[test]
fn luma_trigger_border_stays_important() {
    let p = part_of("luma", "cn-tabs-trigger");
    assert!(p
        .marker
        .decls
        .iter()
        .any(|d| d.field == Field::BorderColor && d.all == 0 && d.rank == 1 << 127));
}

#[test]
fn every_exported_style_translates_every_part() {
    for s in ["vega", "nova", "maia", "lyra", "mira", "luma", "rhea"] {
        let root = env!("CARGO_MANIFEST_DIR");
        let lines = parse_markers(&std::fs::read_to_string(format!("{root}/markers.txt")).unwrap());
        let comps = part_strings(
            &std::path::Path::new(root).join("tests/fixtures/ui"),
            &lines,
        )
        .unwrap();
        for p in parts(&style(s), &comps) {
            assert!(
                p.marker.unsupported.is_empty(),
                "{s} {}: {:?}",
                p.name,
                p.marker.unsupported
            );
        }
    }
}
