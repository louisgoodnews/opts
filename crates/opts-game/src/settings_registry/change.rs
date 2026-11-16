//! Changes settings from outside a system (the control protocol, tests): validated, stored, saved
//! and announced.

use bevy::prelude::*;

use super::{
    api, ChangeSource, SettingChanged, SettingError, SettingsCapabilities, SettingsRegistry,
};
use crate::settings::GameSettings;

/// Changes settings from outside a system (the control protocol, tests): validates, stores,
/// saves, announces.
pub fn apply_change(
    world: &mut World,
    key: &str,
    text: Option<&str>,
    source: ChangeSource,
) -> Result<SettingChanged, SettingError> {
    apply_change_with(world, key, text, source, true)
}

/// [`apply_change`] that can leave the settings file alone (`save` false): a risky change that
/// waits for confirmation, or a batch that saves once at its end.
pub fn apply_change_with(
    world: &mut World,
    key: &str,
    text: Option<&str>,
    source: ChangeSource,
    save: bool,
) -> Result<SettingChanged, SettingError> {
    let registry = world.resource::<SettingsRegistry>().clone();
    let caps = world
        .get_resource::<SettingsCapabilities>()
        .cloned()
        .unwrap_or_default();
    let mut settings = world.resource_mut::<GameSettings>();
    let changed = api::change_in(&registry, &caps, &mut settings, key, text, source, save)?;
    if changed.old != changed.new {
        world.write_message(changed.clone());
    }
    Ok(changed)
}
