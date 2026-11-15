//! Conflicts between vehicle keys and the keys OPTS uses itself.


use super::*;


/// Why `chord` cannot be the key of `setting`: a key OPTS uses itself, or another action's key.
#[must_use]
pub fn conflict(
    registry: &SettingsRegistry,
    settings: &GameSettings,
    setting: &str,
    chord: KeyChord,
) -> Option<String> {
    if chord.is_reserved() {
        return Some(format!(
            "{} is used by driving, the cameras or the overlays and cannot be bound (Ctrl+ combinations can)",
            chord.to_text().unwrap_or_default()
        ));
    }
    // Only the keys of the same kind of vehicle can clash: a bus's and a tram's layouts are apart.
    let rail = ACTIONS.iter().any(|a| a.rail_setting == Some(setting))
        || action(setting).is_some_and(|a| a.scope == Scope::Tram);
    for a in ACTIONS
        .iter()
        .filter(|a| a.scope.applies(rail) && a.setting_for(rail) != setting)
    {
        let other = registry
            .text(settings, a.setting_for(rail))
            .unwrap_or_default();
        if KeyChord::parse(&other).ok().flatten() == Some(chord) {
            return Some(format!(
                "{} is already the key of {} ({})",
                chord.to_text().unwrap_or_default(),
                a.id,
                a.setting_for(rail)
            ));
        }
    }
    None
}

/// The chord that a setting holds for `a` now on a bus (`rail`: false) or a rail vehicle.
#[must_use]
pub fn chord_of(
    registry: &SettingsRegistry,
    settings: &GameSettings,
    a: &VehicleKeyAction,
    rail: bool,
) -> Option<KeyChord> {
    let text = registry.text(settings, a.setting_for(rail))?;
    KeyChord::parse(&text).ok().flatten()
}
