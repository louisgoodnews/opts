//! The ABS and ASR driver aids (ISS-043, M-0165): the settings `gameplay.abs` and `gameplay.asr`
//! switch them, and the player's vehicle simulation takes both switches every frame. A missing or
//! unreadable setting is off.

use bevy::prelude::*;
use opts_bevy::{PlayerVehicle, VehicleController};
use tracing::debug;

use crate::settings::GameSettings;

/// Key of the ABS switch.
pub const ABS_SETTING: &str = "gameplay.abs";
/// Key of the ASR switch.
pub const ASR_SETTING: &str = "gameplay.asr";

/// Whether a switch reads `true`.
pub(crate) fn switch_on(settings: Option<&GameSettings>, key: &str) -> bool {
    settings
        .and_then(|s| s.raw(key))
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
}

/// Copies the ABS and ASR switches into the player's vehicle simulation.
pub fn apply_driver_aids(
    settings: Option<Res<GameSettings>>,
    mut vehicles: Query<&mut VehicleController, With<PlayerVehicle>>,
) {
    let abs = switch_on(settings.as_deref(), ABS_SETTING);
    let asr = switch_on(settings.as_deref(), ASR_SETTING);
    for mut controller in &mut vehicles {
        let aids = &mut controller.sim.driver_aids;
        if aids.abs != abs || aids.asr != asr {
            debug!(abs, asr, "driver aid switches changed");
        }
        aids.abs = abs;
        aids.asr = asr;
    }
}

/// Registers [`apply_driver_aids`] for the game session.
pub struct DriverAidsPlugin;

impl Plugin for DriverAidsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            apply_driver_aids.run_if(in_state(crate::app_state::AppState::InGame)),
        );
    }
}
