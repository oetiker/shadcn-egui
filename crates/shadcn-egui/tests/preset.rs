use shadcn_egui::{
    BaseColor, FontName, IconLibrary, MenuAccent, MenuColor, RadiusChoice, StyleName, ThemeColor,
};

#[test]
fn enums_follow_upstream_list_order() {
    assert_eq!(
        StyleName::ALL.map(|s| s.id()),
        ["nova", "vega", "maia", "lyra", "mira", "luma", "sera", "rhea"]
    );
    assert_eq!(
        BaseColor::ALL.map(|b| b.id()),
        ["neutral", "stone", "zinc", "gray", "mauve", "olive", "mist", "taupe"]
    );
    assert_eq!(ThemeColor::ALL.len(), 25);
    assert_eq!(
        (ThemeColor::ALL[3], ThemeColor::ALL[4], ThemeColor::ALL[24]),
        (ThemeColor::Gray, ThemeColor::Amber, ThemeColor::Taupe)
    );
    assert_eq!(FontName::ALL.len(), 26);
    assert_eq!(
        (FontName::ALL[0], FontName::ALL[17], FontName::ALL[25]),
        (
            FontName::Inter,
            FontName::Oxanium,
            FontName::InstrumentSerif
        )
    );
    assert_eq!(FontName::SourceSans3.id(), "source-sans-3");
    assert_eq!(
        IconLibrary::ALL.map(|l| l.id()),
        ["lucide", "hugeicons", "tabler", "phosphor", "remixicon"]
    );
    assert_eq!(
        MenuColor::ALL.map(|m| m.id()),
        [
            "default",
            "inverted",
            "default-translucent",
            "inverted-translucent"
        ]
    );
    assert_eq!(MenuAccent::ALL.map(|m| m.id()), ["subtle", "bold"]);
    assert_eq!(
        RadiusChoice::ALL.map(|r| r.rem()),
        [None, Some(0.0), Some(0.45), Some(0.625), Some(0.875)]
    );
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
    assert_eq!(
        (RadiusChoice::Px(3.0).id(), RadiusChoice::Px(3.0).rem()),
        (None, None)
    );
    assert_eq!(
        StyleName::from_id("Vega"),
        None,
        "ids are case sensitive, as upstream"
    );
    assert_eq!(FontName::from_id(""), None);
    assert_eq!(RadiusChoice::from_id("px"), None);
}
