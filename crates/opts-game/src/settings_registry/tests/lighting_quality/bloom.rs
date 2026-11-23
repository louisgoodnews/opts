//! Lighting quality: the bloom setting texts reach the camera's level in custom mode.
//!
//! Decision (M-0122, 2026-11-23): the registry stays strict. `graphics.bloom` accepts only the
//! enum texts `off`, `low`, `natural`, `high` (the protocol, the settings reference and the
//! `valid` list of a rejected `settings.set` all name these). The old boolean text `true` is
//! not a protocol alias. Stored settings and map profiles written before the enum still hold
//! `true`/`false`, and [`BloomLevel::parse`] reads them (`true` is Natural, `false` is Off); those
//! texts never pass through the registry, so this test covers only the protocol path.

use super::*;

fn custom_with_bloom(text: &str) -> App {
    let mut app = app();
    set(&mut app, "graphics.light_quality_mode", "custom");
    app.update();
    set(&mut app, "graphics.bloom", text);
    app.update();
    app.update();
    app
}

#[test]
fn every_bloom_text_reaches_its_level_in_custom_mode() {
    let cases = [
        ("off", BloomLevel::Off),
        ("low", BloomLevel::Low),
        ("natural", BloomLevel::Natural),
        ("high", BloomLevel::High),
    ];
    for (text, level) in cases {
        let app = custom_with_bloom(text);
        assert_eq!(quality(&app).bloom, level, "graphics.bloom = {text}");
    }
}

#[test]
fn the_old_true_text_is_rejected_by_the_protocol_and_leaves_bloom_unchanged() {
    let mut app = app();
    set(&mut app, "graphics.light_quality_mode", "custom");
    app.update();
    let rejected = apply_change(
        app.world_mut(),
        "graphics.bloom",
        Some("true"),
        ChangeSource::Protocol,
    );
    assert!(rejected.is_err(), "graphics.bloom = true must be rejected");
    app.update();
    app.update();
    assert_eq!(quality(&app).bloom, BloomLevel::Off);
}
