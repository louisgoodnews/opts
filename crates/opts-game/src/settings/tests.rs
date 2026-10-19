//! Unit tests of `settings.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;
use opts_gui::theme::{Density, ThemeMode};

#[test]
fn baseline_tracks_what_changed_since_the_load() {
    let mut s = GameSettings::default();
    s.remember_baseline();
    let before = s.baseline.0.lock().unwrap().clone().unwrap();
    s.theme_mode = ThemeMode::Light;
    let now = s.entries();
    let changed: Vec<_> = now
        .iter()
        .filter(|(k, v)| before.get(*k) != Some(*v))
        .collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].0, "theme_mode");
}

#[test]
fn test_sqlite_settings_roundtrip() {
    let mut conn = Connection::open_in_memory().expect("In-memory SQLite connection");
    let settings = GameSettings {
        recent_map_path: Some(PathBuf::from("/maps/Berlin")),
        omsi_path: Some(PathBuf::from("/games/OMSI 2")),
        theme_mode: ThemeMode::AutoClock,
        auto_light_from: "06:30".into(),
        auto_dark_from: "20:15".into(),
        high_contrast: true,
        active_theme: "Amber".to_string(),
        ui_scale: 150,
        density: Density::Compact,
        reduced_motion: true,
        discovery_interval_days: 1,
        texture_filtering: opts_bevy::TextureFiltering::X4,
        look: crate::look::Look::Natural,
        ..GameSettings::default()
    };

    settings
        .save_to_connection(&mut conn)
        .expect("Saving to SQLite must succeed");

    let mut loaded = GameSettings::default();
    loaded
        .apply_from_connection(&conn)
        .expect("Loading from SQLite must succeed");

    assert_eq!(loaded.recent_map_path, Some(PathBuf::from("/maps/Berlin")));
    assert_eq!(loaded.omsi_path, Some(PathBuf::from("/games/OMSI 2")));
    assert_eq!(loaded.theme_mode, ThemeMode::AutoClock);
    assert_eq!(loaded.auto_light_from, "06:30");
    assert_eq!(loaded.auto_dark_from, "20:15");
    assert!(loaded.high_contrast);
    assert_eq!(loaded.active_theme, "Amber");
    assert_eq!(loaded.ui_scale, 150);
    assert_eq!(loaded.density, Density::Compact);
    assert!(loaded.reduced_motion);
    assert_eq!(loaded.texture_filtering, opts_bevy::TextureFiltering::X4);
    assert_eq!(loaded.look, crate::look::Look::Natural);
}

#[test]
fn new_keys_default_when_absent_from_an_old_database() {
    // A database written before the keys existed: only the old ones are present.
    let conn = Connection::open_in_memory().expect("In-memory SQLite connection");
    GameSettings::init_database(&conn).expect("schema");
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES ('active_theme', 'Amber')",
        [],
    )
    .expect("insert");
    let mut s = GameSettings::default();
    s.apply_from_connection(&conn).expect("load");
    assert_eq!(s.active_theme, "Amber");
    assert_eq!(s.texture_filtering, opts_bevy::TextureFiltering::X16);
    assert_eq!(s.look, crate::look::Look::Vivid);
    // Neither theme_mode nor is_dark_theme: the default theme, dark.
    assert_eq!(s.theme_mode, ThemeMode::Dark);
    assert_eq!(s.ui_scale, 100);
    assert_eq!(s.density, Density::Comfortable);
    assert_eq!(s.auto_light_from, "07:00");
    assert_eq!(s.auto_dark_from, "19:00");
    // Unreadable values fall back to the defaults too.
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES ('look', 'garbage'), ('texture_filtering', '3')",
        [],
    )
    .expect("insert");
    s.apply_from_connection(&conn).expect("load");
    assert_eq!(s.look, crate::look::Look::Vivid);
    assert_eq!(s.texture_filtering, opts_bevy::TextureFiltering::X16);
}

#[test]
fn changing_the_new_settings_marks_only_them_as_changed() {
    let mut s = GameSettings::default();
    s.remember_baseline();
    s.look = crate::look::Look::BevyDefault;
    s.texture_filtering = opts_bevy::TextureFiltering::Off;
    let before = s.baseline.0.lock().unwrap().clone().unwrap();
    let changed: Vec<_> = s
        .entries()
        .into_iter()
        .filter(|(k, v)| before.get(k) != Some(v))
        .map(|(k, _)| k)
        .collect();
    assert_eq!(changed, ["look", "texture_filtering"]);
}
