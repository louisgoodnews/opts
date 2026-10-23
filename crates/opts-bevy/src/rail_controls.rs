//! The controls of a rail vehicle: a power lever and a direction selector instead of pedals and
//! a gear selector (the `rail_*` actions of [`crate::action_keys`]).
//!
//! * **Lever** (`rail_lever_up` / `rail_lever_down`, Q and Y): moves between -1 (full braking)
//!   and +1 (full traction) while held and stays where it is left, like the master controller of
//!   a tram. Above 0 it is the throttle, below 0 the service brake; the pedals of a road vehicle
//!   are not used.
//! * **Zero** (`rail_lever_zero`, X): puts the lever back to 0 at once.
//! * **Direction** (`rail_forward` / `rail_reverse`, W and S): the direction of travel. A tram's
//!   script has its own switch for backing up; [`apply_direction`] presses it.
//! * No parking brake: the `parking_brake` action does nothing on a rail vehicle (a tram's rail
//!   brake is a hot-spot of the cab).

use bevy::prelude::*;
use opts_sim::script::ScriptEngine;
use opts_sim::{GearPosition, VehicleInputs};

use crate::action_keys::ActionKeys;
use crate::vehicle_script::VehicleScripts;
use crate::vehicle_sim::PlayerVehicle;

/// Seconds the lever takes from rest to full travel when its key is held.
pub const LEVER_FULL_S: f32 = 2.0;

/// Marks a rail vehicle's controls and keeps their state.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct RailControls {
    /// The power lever, `-1` (braking) to `1` (traction).
    pub lever: f32,
    /// The selected direction of travel: `1` forward, `-1` backwards.
    pub direction: i8,
    /// The direction was changed and the scripts have not been told yet.
    pub direction_dirty: bool,
    /// The scripts' direction switches have been put in place at least once: the first move of
    /// the lever does it when no direction key was pressed before (the vehicle starts in
    /// forward).
    pub direction_set: bool,
}

impl Default for RailControls {
    fn default() -> Self {
        Self {
            lever: 0.0,
            direction: 1,
            direction_dirty: false,
            direction_set: false,
        }
    }
}

impl RailControls {
    /// Moves the lever `up` or `down` for `dt` seconds (both keys cancel each other).
    pub fn move_lever(&mut self, up: bool, down: bool, dt: f32) {
        let step = dt / LEVER_FULL_S;
        if up && !down {
            self.lever = (self.lever + step).min(1.0);
        } else if down && !up {
            self.lever = (self.lever - step).max(-1.0);
        }
    }

    /// Selects a direction and has the scripts' switches put there (also when it is the
    /// direction already selected: the key presses the switches again, which are idempotent).
    pub fn select(&mut self, direction: i8) {
        self.direction = direction;
        self.direction_dirty = true;
        self.direction_set = true;
    }

    /// The pedals the lever stands for: `(throttle, service brake)`.
    #[must_use]
    pub fn pedals(&self) -> (f32, f32) {
        (self.lever.max(0.0), (-self.lever).max(0.0))
    }
}

/// One frame of the rail keys: lever, direction and the pedals they give.
pub fn apply_keys(
    controls: &mut RailControls,
    inputs: &mut VehicleInputs,
    keys: &ButtonInput<KeyCode>,
    action_keys: &ActionKeys,
    dt: f32,
) {
    controls.move_lever(
        action_keys.pressed(keys, "rail_lever_up"),
        action_keys.pressed(keys, "rail_lever_down"),
        dt,
    );
    if action_keys.just_pressed(keys, "rail_lever_zero") {
        controls.lever = 0.0;
    }
    if action_keys.just_pressed(keys, "rail_forward") {
        controls.select(1);
    }
    if action_keys.just_pressed(keys, "rail_reverse") {
        controls.select(-1);
    }
    let (throttle, brake) = controls.pedals();
    inputs.set_throttle(throttle);
    inputs.set_service_brake(brake);
    // The built-in rail dynamics pull only in drive; the direction is the controls' own.
    inputs.set_gear(GearPosition::Drive);
}

/// The scripts' own direction switches, by the variable that shows them: a tram's cab has a
/// reverse-drive switch (`RFahrt_A`, a toggle) next to its drive position (`automatic_D`).
const DIRECTION_SWITCHES: &[(&str, &str)] =
    &[("rfahrt_a", "a_kopf_drive"), ("rfahrt_b", "b_kopf_drive")];

/// Puts the scripts' direction switches where `reverse` says; returns whether any was pressed.
pub fn apply_direction(engine: &mut ScriptEngine, reverse: bool) -> bool {
    let mut pressed = false;
    for (toggle, drive) in DIRECTION_SWITCHES {
        if !engine.has_trigger(toggle) {
            continue;
        }
        if engine.has_trigger("automatic_d") && engine.get(drive) < 0.5 {
            pressed |= engine.trigger("automatic_d");
        }
        if (engine.get(toggle) > 0.5) != reverse {
            pressed |= engine.trigger(toggle);
        }
    }
    pressed
}

/// Tells the scripts of the player's rail vehicle about a direction change.
pub fn direction_system(
    mut vehicles: Query<
        (
            &mut RailControls,
            &VehicleScripts,
            Option<&crate::cab_switch::ActiveCab>,
        ),
        With<PlayerVehicle>,
    >,
    parts: Query<&VehicleScripts, With<crate::consist_scripts::ScriptPart>>,
) {
    for (mut controls, head, active) in &mut vehicles {
        // The switches are the active cab's.
        let scripts = crate::cab_switch::cab_scripts(head, active.copied(), &parts);
        if !controls.direction_set && controls.lever > 0.02 {
            controls.direction_dirty = true;
            controls.direction_set = true;
        }
        if !controls.direction_dirty {
            continue;
        }
        controls.direction_dirty = false;
        if let Ok(mut engine) = scripts.engine.lock() {
            let handled = apply_direction(&mut engine, controls.direction < 0);
            tracing::debug!(reverse = controls.direction < 0, handled, "rail direction");
        }
    }
}

#[cfg(test)]
mod tests;
