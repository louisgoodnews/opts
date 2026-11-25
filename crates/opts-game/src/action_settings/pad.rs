//! The gamepad and wheel bindings in the settings (ISS-049): the `controls.pad.<id>` rows and the
//! sync that keeps [`opts_bevy::gamepad_input::PadBindings`] equal to them. A row holds an axis or
//! a button name (see `opts_bevy::gamepad_input`), not a key chord, and is typed as free text: a
//! name that is not understood is logged and the default stands.

use bevy::prelude::*;
use opts_bevy::gamepad_input::{PadBindings, PAD_SETTINGS};

use crate::settings::GameSettings;
use crate::settings_registry::descriptor::{
    Category as C, SettingDescriptor as D, SettingKind as K,
};
use crate::settings_registry::SettingsRegistry;

use super::descriptors::leak;

/// The descriptors of the pad rows, after the key rows.
#[must_use]
pub fn descriptors() -> Vec<D> {
    PAD_SETTINGS
        .into_iter()
        .enumerate()
        .map(|(i, (id, setting, default))| {
            D::new(
                setting,
                K::Text,
                default,
                C::Controls,
                220 + i as u16,
                leak(format!("set-pad-{id}")),
                leak(format!("set-pad-{id}-desc")),
            )
            .keywords(&["pad", "gamepad", "wheel", "controller", "steer", "pedal"])
        })
        .collect()
}

/// Keeps [`PadBindings`] equal to the `controls.pad.<id>` settings.
pub fn sync_pad_bindings(
    registry: Res<SettingsRegistry>,
    settings: Res<GameSettings>,
    mut pad: ResMut<PadBindings>,
) {
    if !(settings.is_changed() || pad.is_added()) {
        return;
    }
    let want = PadBindings::from_lookup(|setting| registry.text(&settings, setting));
    if *pad != want {
        *pad = want;
    }
}
