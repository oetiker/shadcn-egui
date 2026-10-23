//! The blob fixture under crates/shadcn-egui/tests/icons/fixture is generated
//! by the command in plugin/skills/port-component/references/upstream.md; this test fails
//! when icongen's output for it changes and the fixture was not regenerated.

use icongen::{convert, parse_slots, render_crate};

const NAMES: [&str; 7] = [
    "big-box",
    "faded",
    "mixed",
    "ring",
    "square-hole",
    "star-evenodd",
    "x",
];
const SLOTS: &str = "check=mixed,select_trigger=ring,chevron_down_plain=ring,chevron_up=ring,\
chevron_left=ring,chevron_right=ring,more_horizontal=ring,\
close=x";
const PROVENANCE: &str =
    "icongen test fixtures (tools/icongen/tests/fixtures); x.svg is Lucide (ISC)";

#[test]
fn committed_fixture_crate_is_current() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let icons: Vec<_> = NAMES
        .iter()
        .map(|n| {
            let svg = std::fs::read_to_string(format!("{manifest}/tests/fixtures/{n}.svg"))
                .expect("fixture");
            convert(n, &svg).expect("converts")
        })
        .collect();
    let (src, blob) =
        render_crate(&icons, &parse_slots(SLOTS).expect("slots"), PROVENANCE).expect("renders");
    let dir = format!("{manifest}/../../crates/shadcn-egui/tests/icons/fixture/src");
    let committed_src = std::fs::read_to_string(format!("{dir}/lib.rs")).expect("committed lib.rs");
    let committed_blob = std::fs::read(format!("{dir}/icons.bin")).expect("committed icons.bin");
    assert!(
        committed_src == src,
        "fixture lib.rs is stale: regenerate it (plugin/skills/port-component/references/upstream.md, After changing icongen)"
    );
    assert!(
        committed_blob == blob,
        "fixture icons.bin is stale: regenerate it (plugin/skills/port-component/references/upstream.md, After changing icongen)"
    );
}
