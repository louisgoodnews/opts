//! Unit tests for `super` (`weight.rs`).

use super::*;

#[test]
fn a_drive_starts_without_a_test_count() {
    let mut world = World::new();
    world.insert_resource(PassengerCountOverride(Some(40)));
    world
        .run_system_cached(clear_override)
        .expect("system runs");
    assert_eq!(world.resource::<PassengerCountOverride>().0, None);
}

fn settings_with(pairs: &[(&str, &str)]) -> GameSettings {
    let mut s = GameSettings::default();
    for (key, value) in pairs {
        s.set_raw(key, value);
    }
    s
}

#[test]
fn passengers_weigh_on_by_default_at_75_kg() {
    assert_eq!(mass_settings(None), (true, 75.0));
    assert_eq!(mass_settings(Some(&settings_with(&[]))), (true, 75.0));
}

#[test]
fn the_setting_off_gives_no_mass_whatever_the_count() {
    let off = settings_with(&[(SETTING, "false")]);
    let (on, average) = mass_settings(Some(&off));
    assert!(!on);
    for count in [0, 1, 60, 400] {
        assert_eq!(carried_mass_kg(on, average, count), 0.0, "{count} people");
    }
    // "False" and padding mean the same; anything else keeps it on.
    assert!(!mass_settings(Some(&settings_with(&[(SETTING, " False ")]))).0);
    assert!(mass_settings(Some(&settings_with(&[(SETTING, "true")]))).0);
}

#[test]
fn the_carried_mass_is_the_count_times_the_average() {
    // The milestone's example: 60 people at 75 kg add 4,500 kg (11,000 + 4,500 = 15,500 kg).
    assert_eq!(carried_mass_kg(true, 75.0, 60), 4_500.0);
    assert_eq!(
        carried_mass_kg(true, 75.0, 0),
        0.0,
        "a count of 0 is exactly the curb mass"
    );
}

#[test]
fn the_average_comes_from_its_setting_and_an_unreadable_one_falls_back() {
    let s = settings_with(&[(MASS_SETTING, " 80.5 ")]);
    assert_eq!(mass_settings(Some(&s)).1, 80.5);
    let bad = settings_with(&[(MASS_SETTING, "heavy")]);
    assert_eq!(mass_settings(Some(&bad)).1, DEFAULT_MASS_KG);
}
