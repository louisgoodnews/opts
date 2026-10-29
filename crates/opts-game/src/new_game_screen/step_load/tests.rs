use super::*;

#[test]
fn memory_is_read_in_the_unit_that_fits() {
    assert_eq!(memory_text(512), "512 MB");
    assert_eq!(memory_text(1024), "1.0 GB");
    assert_eq!(memory_text(5632), "5.5 GB");
    assert_eq!(memory_text(10 * 1024), "10 GB");
    assert_eq!(memory_text(47 * 120), "5.5 GB");
}

#[test]
fn time_switches_to_minutes_after_a_minute_and_a_half() {
    assert_eq!(time_text((4, 15)), "4–15 s");
    assert_eq!(time_text((40, 90)), "40–90 s");
    assert_eq!(time_text((60, 120)), "1–2 min");
    assert_eq!(time_text((100, 301)), "2–6 min");
}

#[test]
fn every_message_exists_in_both_languages() {
    let loc = Localization::new("en");
    for key in [
        "wiz-load-all-title",
        "wiz-load-all-note",
        "wiz-load-all-slow-disk",
        "wiz-load-all-unknown-map",
        "wiz-load-all-no-profile",
        "wiz-load-all-too-large",
        "wiz-load-all-use-radius",
    ] {
        assert!(loc.has("en", key), "{key} missing in en");
        assert!(loc.has("de", key), "{key} missing in de");
    }
}
