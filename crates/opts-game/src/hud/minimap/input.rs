//! The keys that change the mini map's settings (F7 family, the zoom keys).

use super::*;

/// F7 shows or hides the map, Shift+F7 switches north-up and heading-up, Ctrl+F7 switches the
/// speed zoom on and off, `+` and `-` zoom (the actions `minimap_*`, rebindable; the numpad
/// `+` and `-` always zoom). The keys change the settings, so the page and the keys agree and the choice is remembered.
pub fn minimap_keys(
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<opts_bevy::ActionKeys>>,
    mut api: crate::settings_registry::SettingsApi,
) {
    use crate::settings_registry::ChangeSource;
    let keys = opts_bevy::action_keys_of(&action_keys);
    let config = MiniMapConfig::read(&api.settings);
    let change = if keys.just_pressed(&keyboard, "minimap_speed_zoom") {
        Some((
            SETTING_ZOOM_MODE,
            config.zoom_mode.toggled().id().to_owned(),
        ))
    } else if keys.just_pressed(&keyboard, "minimap_rotate") {
        Some((
            SETTING_ORIENTATION,
            config.orientation.toggled().id().to_owned(),
        ))
    } else if keys.just_pressed(&keyboard, "minimap_toggle") {
        Some((SETTING_ENABLED, (!config.enabled).to_string()))
    } else if keys.just_pressed(&keyboard, "minimap_zoom_in") {
        Some((SETTING_ZOOM, format!("{}", step_zoom(config.zoom_m, 1))))
    } else if keys.just_pressed(&keyboard, "minimap_zoom_out") {
        Some((SETTING_ZOOM, format!("{}", step_zoom(config.zoom_m, -1))))
    } else {
        None
    };
    if let Some((key, value)) = change {
        if let Err(e) = api.change(key, Some(&value), ChangeSource::Page, true) {
            tracing::warn!("mini map setting {key}: {e}");
        }
    }
}
