//! Unit tests for `super` (`settings_theme.rs`).

use super::*;
use rusqlite::Connection;

fn load(rows: &[(&str, &str)]) -> GameSettings {
    let conn = Connection::open_in_memory().expect("db");
    GameSettings::init_database(&conn).expect("schema");
    for (k, v) in rows {
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)",
            [k, v],
        )
        .expect("insert");
    }
    let mut s = GameSettings::default();
    s.apply_from_connection(&conn).expect("load");
    s
}

#[test]
fn gsettings_answers_are_understood() {
    assert_eq!(
        parse_gsettings_scheme("'prefer-dark'\n"),
        Some(ThemeKind::Dark)
    );
    assert_eq!(
        parse_gsettings_scheme("'prefer-light'"),
        Some(ThemeKind::Light)
    );
    assert_eq!(
        parse_gsettings_scheme("'default'"),
        None,
        "no preference falls back"
    );
    assert_eq!(parse_gsettings_scheme(""), None);
}

#[test]
fn neither_key_means_dark() {
    assert_eq!(load(&[]).theme_mode, ThemeMode::Dark);
}

#[test]
fn the_legacy_flag_migrates_when_theme_mode_is_absent() {
    assert_eq!(
        load(&[("is_dark_theme", "true")]).theme_mode,
        ThemeMode::Dark
    );
    assert_eq!(
        load(&[("is_dark_theme", "false")]).theme_mode,
        ThemeMode::Light
    );
}

#[test]
fn theme_mode_wins_over_the_legacy_flag_in_any_row_order() {
    for rows in [
        [("theme_mode", "auto"), ("is_dark_theme", "false")],
        [("is_dark_theme", "false"), ("theme_mode", "auto")],
    ] {
        assert_eq!(load(&rows).theme_mode, ThemeMode::AutoClock);
    }
}

#[test]
fn garbage_values_fall_back_to_defaults() {
    let s = load(&[
        ("theme_mode", "sepia"),
        ("auto_light_from", "25:99"),
        ("ui_scale", "9000"),
        ("density", "tiny"),
    ]);
    assert_eq!(s.theme_mode, ThemeMode::Dark);
    assert_eq!(s.auto_light_from, "07:00");
    assert_eq!(s.ui_scale, 200, "out-of-range scale is clamped");
    assert_eq!(s.density, Density::Comfortable);
}

#[test]
fn old_accent_names_keep_working_and_unknown_ones_fall_back() {
    for (name, preset) in [
        ("Default", AccentPreset::TransitBlue),
        ("Amber", AccentPreset::Amber),
        ("Teal", AccentPreset::Teal),
        ("Unknown", AccentPreset::TransitBlue),
    ] {
        assert_eq!(load(&[("active_theme", name)]).accent(), preset);
    }
}

#[test]
fn preferences_carry_the_times() {
    let s = load(&[("auto_light_from", "6:30"), ("auto_dark_from", "21:00")]);
    let p = s.theme_preferences();
    assert_eq!(format_time(p.light_from), "06:30");
    assert_eq!(format_time(p.dark_from), "21:00");
}
