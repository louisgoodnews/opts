//! Control-protocol methods `settings.list`, `settings.get`, `settings.set`, `settings.reset`
//! (`docs/src/architecture/control-protocol.md`), and the registration of the confirm and revert
//! ([`super::protocol_confirm`]) and batch ([`super::protocol_batch`]) methods. They use the same
//! validation and the same effects as the settings page.

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use super::{
    apply_change_with, protocol_batch, protocol_confirm, Apply, Available, ChangeSource,
    SettingChanged, SettingDescriptor, SettingError, SettingKind, SettingsRegistry,
};
use crate::settings::GameSettings;

/// Registers the methods when the control server exists.
pub fn register(app: &mut App) {
    let Some(mut registry) = app.world_mut().get_resource_mut::<ControlRegistry>() else {
        return;
    };
    registry.register("settings.list", list);
    registry.register("settings.get", get);
    registry.register("settings.set", set);
    registry.register("settings.reset", reset);
    registry.register("settings.reset_keys", reset_keys);
    registry.register("settings.confirm", protocol_confirm::confirm);
    registry.register("settings.revert", protocol_confirm::revert);
    registry.register("settings.set_many", protocol_batch::set_many);
    registry.register("settings.import", protocol_batch::import);
    registry.register("settings.export", protocol_batch::export);
    registry.register("settings.diff", protocol_batch::diff);
}

mod change;
mod describe;
mod methods;
pub(super) use change::*;
pub(super) use describe::*;
pub(crate) use methods::*;

#[cfg(test)]
#[path = "protocol/rpc/tests.rs"]
mod tests;

// The registry-driven coverage and the confirm, batch and profile tests (M-0072); a file named
// `tests.rs` is what the no-blocking-calls lint of the UI modules treats as test code.
#[cfg(test)]
#[path = "protocol/tests.rs"]
mod coverage;
