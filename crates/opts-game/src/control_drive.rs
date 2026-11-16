//! Driving and camera commands of the control protocol: `drive.*`, `vehicle.teleport`,
//! `vehicle.reset`, `camera.look`, `camera.move` and `snapshot`.
//!
//! The controls (`drive.set`, `drive.trigger`, `drive.release`) feed
//! [`opts_bevy::vehicle_remote::RemoteControls`], which the vehicle input system merges under the
//! same run conditions as the keyboard. The handlers also test those conditions when the command
//! arrives and answer `1009` (gated) instead of queueing something that would never run.

use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy::input::mouse::{MouseButton, MouseButtonInput, MouseMotion};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::time::{Real, Time, Virtual};
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::handlers::input::primary_window_or_placeholder;
use opts_bevy::control::keys::parse_key;
use opts_bevy::control::params::{opt_f64, opt_u64, str_param};
use opts_bevy::control::state::{ControlRegistry, ControlState, Outcome, Release};
use opts_bevy::vehicle_remote::{raise_trigger, Hold, ParkingRequest, RemoteControls};
use opts_bevy::{
    CameraMode, CameraState, KeyChord, PlayerVehicle, ScriptKeyBindings, VehicleController,
};

use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;
use opts_cli::protocol::{code, RpcError};
use opts_sim::vehicle::inputs::GearPosition;
use serde_json::{json, Value};

use crate::in_game::omsi_world::WorldStats;

/// Registers the methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("drive.controls", controls);
    r.register("drive.set", set);
    r.register("drive.release", release);
    r.register("drive.trigger", trigger);
    r.register("drive.state", drive_state);
    r.register("vehicle.key", vehicle_key);
    r.register("vehicle.keys", vehicle_keys);
    r.register("vehicle.teleport", teleport);
    r.register("vehicle.reset", reset);
    r.register("camera.look", look);
    r.register("camera.reset", reset_camera);
    r.register("camera.move", move_camera);
    r.register("snapshot", snapshot);
}

/// Short names for the standard OMSI trigger names, used when the vehicle binds the standard one.
pub const TRIGGER_ALIASES: &[(&str, &str)] = &[
    ("door_front", "bus_doorfront0"),
    ("door_middle", "bus_doormiddle0"),
    ("door_rear", "bus_doorrear0"),
    ("indicator_left", "blinker_left_set"),
    ("indicator_right", "blinker_right_set"),
    ("indicator_off", "blinker_off"),
    ("lights", "kw_scheinwerfer_toggle"),
];

fn resolve_trigger(bindings: &ScriptKeyBindings, name: &str) -> String {
    if bindings.0.iter().any(|b| b.trigger == name) {
        return name.to_string();
    }
    TRIGGER_ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map_or_else(|| name.to_string(), |(_, real)| (*real).to_string())
}

mod controls;
mod drive_set;
mod gate;
use controls::controls;
use drive_set::{raise, set, trigger};
use gate::{gear_name, modal_open, trigger_gate, vehicle_gate};

// ---- vehicle.key / vehicle.keys -------------------------------------------------------------

mod keys;
use keys::{release, vehicle_key, vehicle_keys};

// ---- drive.state ------------------------------------------------------------------------------

mod state;
pub use state::drive_state_json;
use state::{drive_state, snapshot};

// ---- vehicle.teleport / reset -------------------------------------------------------------------

mod camera;
use camera::{look, move_camera, reset, reset_camera, teleport};

// ---- camera.reset ---------------------------------------------------------------------------

// ---- camera.look / camera.move -----------------------------------------------------------------------

// ---- snapshot --------------------------------------------------------------------------------------

#[cfg(test)]
mod tests;
