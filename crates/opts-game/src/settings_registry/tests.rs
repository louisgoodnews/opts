//! Tests of the registry core: defaults, validation, legacy bridge, migration, persistence of a
//! fixture setting declared with one descriptor, messages and the generated reference.

use super::*;
use rusqlite::Connection;
use std::path::PathBuf;

/// A setting declared with exactly one descriptor (the acceptance criterion of M-0063).
const FIXTURE: SettingDescriptor = SettingDescriptor::new(
    "fixture.wheel_sensitivity",
    SettingKind::Int {
        min: 1,
        max: 10,
        step: 1,
    },
    "5",
    Category::Controls,
    5,
    "set-fixture",
    "set-fixture-desc",
)
.keywords(&["steering"]);

fn caps() -> SettingsCapabilities {
    SettingsCapabilities::default()
}

fn registry() -> SettingsRegistry {
    SettingsRegistry::with_builtin()
}

#[test]
fn defaults_of_legacy_keys_equal_the_typed_defaults() {
    let r = registry();
    let s = GameSettings::default();
    for key in GameSettings::FIELD_KEYS {
        let d = r
            .get(key)
            .unwrap_or_else(|| panic!("{key} is not registered"));
        assert_eq!(
            s.raw(key).unwrap_or_default(),
            d.default,
            "default of {key} differs from the field"
        );
    }
}

#[test]
fn every_enum_option_round_trips_through_the_loader_for_legacy_keys() {
    let r = registry();
    for key in GameSettings::FIELD_KEYS {
        let d = r.get(key).expect("registered");
        if let SettingKind::Enum(options) = d.kind {
            for o in options {
                let mut s = GameSettings::default();
                s.set_raw(key, o.value);
                assert_eq!(s.raw(key).as_deref(), Some(o.value), "{key}={}", o.value);
            }
        }
    }
}

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

#[test]
fn every_descriptor_is_well_formed_and_has_both_languages() {
    let r = registry();
    let loc = localization("en");
    for d in r.all() {
        // the default is valid for its own kind
        if d.is_stored() {
            assert!(
                normalize(d, d.default).is_ok(),
                "default of {} invalid",
                d.key
            );
        }
        let mut keys = vec![d.label_key, d.description_key];
        if let SettingKind::Enum(o) = d.kind {
            keys.extend(o.iter().map(|o| o.label_key));
        }
        for k in keys {
            assert!(loc.has("en", k), "{k} missing in en");
            assert!(loc.has("de", k), "{k} missing in de");
        }
    }
    for c in Category::ALL {
        assert!(loc.has("en", c.label_key()) && loc.has("de", c.label_key()));
    }
}

#[test]
fn keys_are_unique_and_never_another_keys_old_name() {
    let r = registry();
    let all = r.all();
    for d in &all {
        for old in d.migrate_from {
            assert!(r.get(old).is_none(), "{old} is both a key and an old name");
        }
    }
}

#[test]
fn search_finds_by_label_keyword_and_description_in_both_languages() {
    let mut r = registry();
    r.register(FIXTURE);
    let mut loc = localization("en");
    let keys = |q: &str, loc: &Localization| -> Vec<&'static str> {
        search::search(&r, loc, q).iter().map(|d| d.key).collect()
    };
    assert!(keys("vsync", &loc).contains(&"graphics.vsync"));
    assert!(keys("tearing", &loc).contains(&"graphics.vsync"), "keyword");
    assert!(
        keys("kilometres", &loc).contains(&"interface.units"),
        "description"
    );
    assert!(keys("steering", &loc).contains(&"fixture.wheel_sensitivity"));
    assert!(
        keys("window size", &loc).contains(&"graphics.resolution"),
        "all words match"
    );
    assert!(keys("zzzz", &loc).is_empty());
    assert!(keys("  ", &loc).is_empty());
    loc.set_locale("de");
    assert!(keys("fenstergröße", &loc).contains(&"graphics.resolution"));
}

#[test]
fn reference_page_is_up_to_date() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/src/reference/settings.md");
    let want = reference::markdown(&registry());
    if std::env::var_os("UPDATE_SETTINGS_DOC").is_some() {
        std::fs::write(&path, &want).expect("write reference page");
    }
    let have = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        have, want,
        "docs/src/reference/settings.md is stale: run `UPDATE_SETTINGS_DOC=1 cargo test -p opts-game settings_registry`"
    );
}

#[test]
fn gameplay_defaults_equal_the_constants_of_the_session_systems() {
    use crate::in_game::omsi_world::{DEFAULT_PEDESTRIAN_BASELINE, DEFAULT_TRAFFIC_BASELINE};
    let r = registry();
    assert_eq!(
        r.get("gameplay.traffic").map(|d| d.default),
        Some(DEFAULT_TRAFFIC_BASELINE.to_string().as_str())
    );
    assert_eq!(
        r.get("gameplay.pedestrians").map(|d| d.default),
        Some(DEFAULT_PEDESTRIAN_BASELINE.to_string().as_str())
    );
}

#[test]
fn the_weather_settings_default_on_and_switch_each_effect() {
    use opts_bevy::weather::PrecipitationQuality;
    let r = SettingsRegistry::with_builtin();
    for key in [
        "graphics.weather_fog",
        "graphics.weather_clouds",
        "graphics.weather_wet_ground",
    ] {
        assert_eq!(r.get(key).map(|d| d.default), Some("true"), "{key}");
    }
    assert_eq!(
        r.get("graphics.weather_precipitation").map(|d| d.default),
        Some("medium")
    );
    let mut settings = GameSettings::default();
    assert_eq!(
        live::weather_effects_of(&settings),
        opts_bevy::WeatherEffects::default()
    );
    for (key, value) in [
        ("graphics.weather_fog", "false"),
        ("graphics.weather_clouds", "false"),
        ("graphics.weather_wet_ground", "false"),
        ("graphics.weather_precipitation", "high"),
    ] {
        settings.set_raw(key, value);
    }
    let fx = live::weather_effects_of(&settings);
    assert!(!fx.fog && !fx.clouds && !fx.wet_ground);
    assert_eq!(fx.precipitation, PrecipitationQuality::High);
    // An unknown stored quality falls back to the default instead of switching rain off.
    settings.set_raw("graphics.weather_precipitation", "ultra");
    assert_eq!(
        live::weather_effects_of(&settings).precipitation,
        PrecipitationQuality::Medium
    );

    // The system keeps the resource equal to the settings.
    let mut app = App::new();
    app.insert_resource(settings)
        .init_resource::<opts_bevy::WeatherEffects>()
        .add_systems(Update, live::sync_weather_effects);
    app.update();
    assert!(!app.world().resource::<opts_bevy::WeatherEffects>().fog);
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("graphics.weather_fog", "true");
    app.update();
    assert!(app.world().resource::<opts_bevy::WeatherEffects>().fog);
}

mod lighting_quality;

#[test]
fn hdr_is_on_by_default_and_follows_the_setting_on_the_main_camera_only() {
    use bevy::camera::Hdr;
    let r = SettingsRegistry::with_builtin();
    assert_eq!(r.get("graphics.hdr").map(|d| d.default), Some("true"));
    let mut app = App::new();
    app.insert_resource(GameSettings::default())
        .add_systems(Update, live::sync_hdr);
    let main = app
        .world_mut()
        .spawn((Camera3d::default(), opts_bevy::MainCamera))
        .id();
    let mirror = app.world_mut().spawn(Camera3d::default()).id();
    app.update();
    assert!(app.world().get::<Hdr>(main).is_some(), "on by default");
    assert!(
        app.world().get::<Hdr>(mirror).is_none(),
        "not on other cameras"
    );
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("graphics.hdr", "false");
    app.update();
    assert!(app.world().get::<Hdr>(main).is_none(), "off, live");
    app.world_mut()
        .resource_mut::<GameSettings>()
        .set_raw("graphics.hdr", "true");
    app.update();
    assert!(app.world().get::<Hdr>(main).is_some(), "on again, live");
}
