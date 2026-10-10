use stylegen::css::parse_style_css;
use stylegen::model::{Field, Value};
use stylegen::resolve::{collect_vars, translate, Marker};

fn style(name: &str) -> Vec<(String, Vec<String>)> {
    let p = format!("{}/tests/fixtures/style-{name}.css", env!("CARGO_MANIFEST_DIR"));
    parse_style_css(&std::fs::read_to_string(p).expect("fixture"))
}

fn marker(css: &[(String, Vec<String>)], m: &str) -> Marker {
    let vars = collect_vars(css);
    let classes = &css.iter().find(|(n, _)| n == m).expect(m).1;
    translate(classes, &vars)
}

/// Value of `field` with no condition.
fn base(m: &Marker, field: Field) -> Option<Value> {
    m.decls.iter().rfind(|d| d.field == field && d.all == 0 && d.none == 0 && d.rank == 0).map(|d| d.value)
}
fn when(m: &Marker, field: Field, all: u32) -> Option<Value> {
    m.decls.iter().rfind(|d| d.field == field && d.all == all).map(|d| d.value)
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
        assert_eq!(base(&m, Field::BgClipPadding), Some(Value::Bool(true)), "{name}");
    }
}

#[test]
fn nova_destructive_is_tinted() {
    let m = marker(&style("nova"), "cn-button-variant-destructive");
    assert_eq!(base(&m, Field::Bg), Some(Value::Paint { role: "Destructive", alpha: 0.1 }));
    assert_eq!(when(&m, Field::Bg, 1), Some(Value::Paint { role: "Destructive", alpha: 0.2 }));
}

#[test]
fn nova_slider_thumb() {
    let m = marker(&style("nova"), "cn-slider-thumb");
    assert_eq!(base(&m, Field::Width), Some(Value::Px(12.0)));
    assert_eq!(base(&m, Field::HitInsetX), Some(Value::Px(8.0)));
    assert_eq!(base(&m, Field::Bg), Some(Value::Paint { role: "White", alpha: 1.0 }));
}

#[test]
fn card_spacing_var_resolves_with_size_condition() {
    let m = marker(&style("nova"), "cn-card-content");
    assert_eq!(base(&m, Field::PadLeft), Some(Value::Px(16.0)));
    assert_eq!(when(&m, Field::PadLeft, 1 << 19), Some(Value::Px(12.0)));
}

#[test]
fn later_class_wins_under_same_condition() {
    let css = vec![("cn-x".to_string(), vec!["h-8".to_string(), "h-9".to_string(), "hover:h-1".to_string(), "hover:h-2".to_string()])];
    let m = translate(&css[0].1, &collect_vars(&css));
    assert_eq!(m.decls.iter().filter(|d| d.field == Field::Height).count(), 2);
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
    assert_eq!(when(&m, Field::PadBottom, (1 << 19) | (1 << 25)), Some(Value::Px(0.0)));
}
