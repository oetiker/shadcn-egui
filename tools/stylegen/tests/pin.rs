use stylegen::pin::{check_pin, parse_pin};

#[test]
fn bad_pins_are_rejected() {
    for bad in [
        "",
        "  \n",
        "4",
        "xyz1234",
        "ABCDEF1",
        "abcdef",
        &"a".repeat(41),
    ] {
        assert!(parse_pin(bad).is_err(), "{bad:?} must be rejected");
    }
}

#[test]
fn current_pins_are_accepted() {
    let tool = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for file in ["PIN", "PIN_REACT"] {
        let text = std::fs::read_to_string(tool.join(file)).unwrap();
        assert_eq!(parse_pin(&text).unwrap(), text.trim());
    }
}

#[test]
fn a_failing_git_command_is_an_error() {
    let missing = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-such-checkout");
    assert!(check_pin(&missing, "abc1234").is_err());
}
