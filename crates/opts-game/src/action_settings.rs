//! The key bindings of the settings (M-0142): the built-in actions of `opts_bevy::action_keys`
//! as `controls.<group>.<id>` rows next to the vehicle actions (`controls.vehicle.<id>`), the
//! conflict check across all of them, the live link to [`opts_bevy::ActionKeys`] and the reset of
//! one group or of every binding.

mod conflicts;
mod descriptors;
mod pad;
mod sync;

pub use conflicts::conflict;
pub use descriptors::descriptors;
pub use sync::{chord_of, reset_bindings, sync_action_keys, sync_mouse_driving, ActionKeysPlugin};

use bevy::prelude::*;
use opts_bevy::action_keys::{self};

use crate::vehicle_keys;

/// The groups of key bindings, in the order of the reset rows.
pub const GROUPS: [&str; 6] = ["vehicle", "rail", "drive", "camera", "map", "ui"];

/// `controls.mouse_driving`: whether the wheel press switches the mouse driving mode.
pub const MOUSE_DRIVING_KEY: &str = "controls.mouse_driving";

/// The prefix of the reset rows (`controls.reset.map`, `controls.reset.all`).
pub const RESET_PREFIX: &str = "controls.reset.";

/// Every binding setting with its default and group: the vehicle actions and the built-in ones.
#[must_use]
pub fn binding_settings() -> Vec<(String, &'static str, &'static str)> {
    let mut v: Vec<(String, &'static str, &'static str)> = vehicle_keys::ACTIONS
        .iter()
        .map(|a| (a.setting.to_owned(), a.default, "vehicle"))
        .collect();
    v.extend(
        vehicle_keys::ACTIONS
            .iter()
            .filter_map(|a| Some((a.rail_setting?.to_owned(), a.default, "rail"))),
    );
    v.extend(
        action_keys::ACTIONS
            .iter()
            .map(|a| (action_keys::setting_of(a), a.default, a.group)),
    );
    v
}

#[cfg(test)]
mod tests;
