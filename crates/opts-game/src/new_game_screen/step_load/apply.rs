//! The handlers of the row's switch and its *Use the tile radius* button.

use super::*;

/// The switch writes the global setting.
pub fn apply_load_all_switch(
    mut toggled: MessageReader<SwitchToggled>,
    switches: Query<(), With<LoadAllSwitch>>,
    mut api: SettingsApi,
) {
    for t in toggled.read() {
        if switches.get(t.entity).is_err() {
            continue;
        }
        let text = if t.is_on { "true" } else { "false" };
        if api.text(KEY) != text {
            let _ = api.change(KEY, Some(text), ChangeSource::Page, true);
        }
    }
}

/// *Use the tile radius* turns the whole-map loading off.
pub fn apply_use_radius(
    mut activated: MessageReader<Activated>,
    buttons: Query<(), With<UseRadiusButton>>,
    mut api: SettingsApi,
) {
    for Activated(e) in activated.read() {
        if buttons.get(*e).is_ok() {
            let _ = api.change(KEY, Some("false"), ChangeSource::Page, true);
        }
    }
}
