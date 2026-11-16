//! Lighting quality: the defaults, the custom keys and their texts.

use super::*;

#[test]
fn defaults_are_level_zero_in_automatic_mode() {
    let r = SettingsRegistry::with_builtin();
    let settings = GameSettings::default();
    let want = [
        ("graphics.light_quality_mode", "automatic"),
        ("graphics.light_shafts", "medium"),
        ("graphics.light_night_shafts", "true"),
        ("graphics.light_lantern_cones", "true"),
        ("graphics.light_headlight_cones", "true"),
        ("graphics.light_cone_cap", "64"),
        ("graphics.light_cone_depth_fade", "true"),
        ("graphics.mirror_count", "2"),
        ("graphics.mirror_rate", "2"),
        ("graphics.glass_reflections", "true"),
        ("graphics.bloom", "off"),
        ("graphics.overlays", "full"),
        ("graphics.light_governor_target_fps", "60"),
    ];
    for (key, default) in want {
        let d = r.get(key).unwrap_or_else(|| panic!("{key} missing"));
        assert_eq!(d.default, default, "{key}");
        assert_eq!(d.apply, Apply::Live, "{key} applies live");
        assert!(!d.hidden && d.category == Category::Graphics);
    }
    let custom = lighting::custom_of(&settings);
    assert!(custom.same_values(&ladder_preset(0)), "{custom:?}");
    assert_eq!(lighting::mode_of(&settings), LightMode::Automatic);
}

#[test]
fn every_custom_key_is_registered_and_the_texts_round_trip() {
    let r = SettingsRegistry::with_builtin();
    let mut settings = GameSettings::default();
    for level in 0..=5 {
        let q = ladder_preset(level);
        for (key, text) in lighting::CUSTOM_KEYS.iter().zip(lighting::texts_of(&q)) {
            assert!(r.get(key).is_some(), "{key}");
            r.set(&mut settings, &SettingsCapabilities::default(), key, &text)
                .unwrap_or_else(|e| panic!("{key}={text}: {e:?}"));
        }
        assert!(
            lighting::custom_of(&settings).same_values(&q),
            "level {level}"
        );
    }
    let caps = SettingsCapabilities::default();
    assert!(r
        .set(&mut settings, &caps, "graphics.light_cone_cap", "97")
        .is_err());
    assert!(r
        .set(&mut settings, &caps, "graphics.mirror_count", "3")
        .is_err());
    assert!(r
        .set(&mut settings, &caps, "graphics.mirror_rate", "3")
        .is_err());
    assert!(r
        .set(&mut settings, &caps, "graphics.light_quality_mode", "x")
        .is_err());
    assert!(r
        .set(
            &mut settings,
            &caps,
            "graphics.light_governor_target_fps",
            "29"
        )
        .is_err());
    assert!(r
        .set(
            &mut settings,
            &caps,
            "graphics.light_governor_target_fps",
            "241"
        )
        .is_err());
    r.set(
        &mut settings,
        &caps,
        "graphics.light_governor_target_fps",
        "144",
    )
    .expect("144 is in range");
    assert_eq!(lighting::target_fps_of(&settings), 144);
    r.set(&mut settings, &caps, "graphics.light_cone_cap", "96")
        .expect("96 is the maximum");
    assert_eq!(lighting::custom_of(&settings).cone_cap, 96);
}
