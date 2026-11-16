//! Storage tests: a fixture setting's storage, validation and reset, unreadable values, migration
//! of renamed keys and an old database.

use super::*;

#[test]
fn a_fixture_setting_needs_one_descriptor_for_storage_validation_and_reset() {
    let mut r = registry();
    r.register(FIXTURE);
    let mut s = GameSettings::default();
    assert_eq!(
        r.value(&s, "fixture.wheel_sensitivity"),
        Some(SettingValue::Int(5))
    );
    r.set(&mut s, &caps(), "fixture.wheel_sensitivity", "7")
        .expect("valid");
    assert_eq!(
        r.value(&s, "fixture.wheel_sensitivity"),
        Some(SettingValue::Int(7))
    );
    assert!(r
        .set(&mut s, &caps(), "fixture.wheel_sensitivity", "11")
        .is_err());
    assert!(r
        .set(&mut s, &caps(), "fixture.wheel_sensitivity", "fast")
        .is_err());
    // persisted through the unchanged table
    let mut conn = Connection::open_in_memory().expect("db");
    s.save_to_connection(&mut conn).expect("save");
    let mut loaded = GameSettings::default();
    loaded.apply_from_connection(&conn).expect("load");
    assert_eq!(
        r.text(&loaded, "fixture.wheel_sensitivity").as_deref(),
        Some("7")
    );
    r.reset(&mut loaded, &caps(), "fixture.wheel_sensitivity")
        .expect("reset");
    assert_eq!(
        r.text(&loaded, "fixture.wheel_sensitivity").as_deref(),
        Some("5")
    );
}

#[test]
fn unreadable_stored_values_fall_back_to_the_default() {
    let mut r = registry();
    r.register(FIXTURE);
    let mut s = GameSettings::default();
    s.set_raw("fixture.wheel_sensitivity", "banana");
    assert_eq!(
        r.text(&s, "fixture.wheel_sensitivity").as_deref(),
        Some("5")
    );
}

#[test]
fn renamed_keys_migrate_when_the_new_key_is_absent() {
    const RENAMED: SettingDescriptor = SettingDescriptor::new(
        "fixture.new_name",
        SettingKind::Bool,
        "false",
        Category::Advanced,
        99,
        "set-fixture",
        "set-fixture-desc",
    )
    .migrate_from(&["fixture.old_name"]);
    let mut r = registry();
    r.register(RENAMED);
    let conn = Connection::open_in_memory().expect("db");
    GameSettings::init_database(&conn).expect("schema");
    conn.execute(
        "INSERT INTO app_settings VALUES ('fixture.old_name', 'yes')",
        [],
    )
    .expect("row");
    let mut s = GameSettings::default();
    s.apply_from_connection(&conn).expect("load");
    assert_eq!(r.migrate(&mut s), vec!["fixture.new_name"]);
    assert_eq!(r.text(&s, "fixture.new_name").as_deref(), Some("true"));
    // the new key wins when both exist
    conn.execute(
        "INSERT INTO app_settings VALUES ('fixture.new_name', 'false')",
        [],
    )
    .expect("row");
    let mut s = GameSettings::default();
    s.apply_from_connection(&conn).expect("load");
    assert!(r.migrate(&mut s).is_empty());
    assert_eq!(r.text(&s, "fixture.new_name").as_deref(), Some("false"));
}

#[test]
fn an_old_database_loads_without_loss_including_the_theme_keys_of_m0061() {
    let conn = Connection::open_in_memory().expect("db");
    GameSettings::init_database(&conn).expect("schema");
    for (k, v) in [
        ("is_dark_theme", "false"),
        ("active_theme", "Amber"),
        ("omsi_path", "/games/OMSI 2"),
        ("recent_map_path", "/maps/Berlin"),
        ("discovery_interval_days", "7"),
        ("texture_filtering", "8"),
        ("look", "natural"),
        ("some_future_key", "kept"),
    ] {
        conn.execute("INSERT INTO app_settings VALUES (?1, ?2)", [k, v])
            .expect("row");
    }
    let mut s = GameSettings::default();
    s.apply_from_connection(&conn).expect("load");
    let r = registry();
    assert_eq!(
        r.text(&s, "theme_mode").as_deref(),
        Some("light"),
        "is_dark_theme migrated"
    );
    assert_eq!(r.text(&s, "active_theme").as_deref(), Some("Amber"));
    assert_eq!(s.omsi_path, Some(PathBuf::from("/games/OMSI 2")));
    assert_eq!(r.text(&s, "discovery_interval_days").as_deref(), Some("7"));
    assert_eq!(r.text(&s, "texture_filtering").as_deref(), Some("8"));
    assert_eq!(r.text(&s, "look").as_deref(), Some("natural"));
    assert_eq!(s.raw("some_future_key").as_deref(), Some("kept"));
    let mut out = Connection::open_in_memory().expect("db");
    s.save_to_connection(&mut out).expect("save");
    let n: String = out
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'some_future_key'",
            [],
            |r| r.get(0),
        )
        .expect("kept on save");
    assert_eq!(n, "kept");
}
