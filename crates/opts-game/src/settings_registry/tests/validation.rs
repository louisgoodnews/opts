//! Validation tests: the messages name the key, planned settings refuse changes.

use super::*;

#[test]
fn validation_messages_name_the_key_and_the_expectation() {
    let r = registry();
    let mut s = GameSettings::default();
    let e = r
        .set(&mut s, &caps(), "ui_scale", "300")
        .expect_err("range");
    assert!(
        e.to_string().contains("ui_scale") && e.to_string().contains("75"),
        "{e}"
    );
    assert!(matches!(
        r.set(&mut s, &caps(), "theme_mode", "purple"),
        Err(SettingError::Invalid(_))
    ));
    assert!(matches!(
        r.set(&mut s, &caps(), "nope", "1"),
        Err(SettingError::Unknown(_))
    ));
    assert_eq!(
        r.set(&mut s, &caps(), "auto_light_from", "6:5")
            .map(|c| c.1)
            .ok()
            .as_deref(),
        Some("06:05")
    );
    assert_eq!(
        r.set(&mut s, &caps(), "high_contrast", "ON")
            .map(|c| c.1)
            .ok()
            .as_deref(),
        Some("true")
    );
}

#[test]
fn planned_settings_refuse_changes_with_the_standard_wording() {
    let r = registry();
    let mut s = GameSettings::default();
    let e = r
        .set(&mut s, &caps(), "gameplay.time_scale", "5")
        .expect_err("not yet");
    assert!(e.to_string().contains("Feature not available yet!"), "{e}");
}
