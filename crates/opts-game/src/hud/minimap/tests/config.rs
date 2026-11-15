//! Zoom and config tests: the zoom steps and the settings the config reads.

use super::*;

#[test]
fn zoom_steps_cover_100_m_to_1_km_and_stop_at_the_ends() {
    assert_eq!(ZOOM_STEPS_M.first(), Some(&100.0));
    assert_eq!(ZOOM_STEPS_M.last(), Some(&1000.0));
    assert_eq!(step_zoom(200.0, 1), 100.0, "in");
    assert_eq!(step_zoom(100.0, 1), 100.0, "in stops");
    assert_eq!(step_zoom(200.0, -1), 500.0, "out");
    assert_eq!(step_zoom(1000.0, -1), 1000.0, "out stops");
    assert_eq!(
        step_zoom(333.0, -1),
        500.0,
        "an unknown value starts at the default"
    );
}

#[test]
fn the_config_reads_the_settings_and_falls_back_to_defaults() {
    let mut s = GameSettings::default();
    assert_eq!(MiniMapConfig::read(&s), MiniMapConfig::default());
    s.set_raw(SETTING_ENABLED, "false");
    s.set_raw(SETTING_ZOOM, "500");
    s.set_raw(SETTING_ORIENTATION, "north");
    let c = MiniMapConfig::read(&s);
    assert!(!c.enabled);
    assert_eq!(c.zoom_m, 500.0);
    assert_eq!(c.orientation, Orientation::North);
    s.set_raw(SETTING_ZOOM, "37");
    s.set_raw(SETTING_ORIENTATION, "sideways");
    let c = MiniMapConfig::read(&s);
    assert_eq!(c.zoom_m, DEFAULT_ZOOM_M);
    assert_eq!(c.orientation, Orientation::Heading);
}
