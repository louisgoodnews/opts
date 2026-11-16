//! Remote vehicle controls: what the control protocol's `drive.*` commands feed into the vehicle.
//!
//! The keyboard path ([`crate::vehicle_sim::vehicle_input_system`]) is the only place that writes
//! the driver inputs. Remote controls are merged **inside that system**, so they run under its
//! gates (the vehicle must take input, no modal may be open) exactly like a key press; the
//! protocol handler also checks the gates when the command arrives and answers `1009` instead of
//! queueing a command that would never run.
//!
//! Analog values (pedals, steering) are *held*: they override the keyboard for that axis until
//! released or until their duration of simulation time has run out. Gear and parking brake are
//! one-shot requests. Script triggers (doors, lights, horn, ...) are raised by the handler
//! through [`raise_trigger`], with the matching `_off` scheduled here.

use bevy::prelude::*;
use opts_sim::vehicle::inputs::{GearPosition, VehicleInputs};

use crate::vehicle_script::VehicleScripts;
use crate::vehicle_sim::{PlayerVehicle, VehicleController};
mod gates;
#[cfg(test)]
mod tests;
mod triggers;

pub use gates::{flush_remote_when_gated, odometer_system, vehicle_input_closed};
pub use triggers::{raise_trigger, release_trigger_system};

/// An analog value held on one axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hold {
    /// The value to apply each frame.
    pub value: f32,
    /// Seconds of simulation time left; `None` holds until released.
    pub remaining: Option<f32>,
}

/// A parking-brake request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParkingRequest {
    /// Engage or release.
    Set(bool),
    /// Flip.
    Toggle,
}

/// Remote driver inputs and the odometer.
#[derive(Resource, Debug, Default)]
pub struct RemoteControls {
    /// Held throttle pedal.
    pub throttle: Option<Hold>,
    /// Held service brake pedal.
    pub brake: Option<Hold>,
    /// Held steering wheel position.
    pub steer: Option<Hold>,
    /// Requested gear selector position.
    pub gear: Option<GearPosition>,
    /// Requested parking brake change.
    pub parking: Option<ParkingRequest>,
    /// `_off` triggers waiting for their time: `(name, seconds left)`.
    pub release_triggers: Vec<(String, f32)>,
    /// Metres driven since the session (or the last reset) started.
    pub odometer_m: f64,
    /// Last chassis position seen by the odometer.
    last_position: Option<Vec2>,
    /// Pose the session placed the vehicle at: `(position, heading_rad, elevation)`.
    pub start_pose: Option<(Vec2, f32, f32)>,
    /// A distance (m) a restored save wants the odometer to continue from, and the frames it may
    /// still wait for the vehicle to be activated (which restarts the odometer).
    pub pending_odometer: Option<(f64, u32)>,
}

impl RemoteControls {
    /// Whether anything is held or waiting.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.throttle.is_some()
            || self.brake.is_some()
            || self.steer.is_some()
            || self.gear.is_some()
            || self.parking.is_some()
    }

    /// Drops every held control and request (keys take over again).
    pub fn release_all(&mut self) {
        self.throttle = None;
        self.brake = None;
        self.steer = None;
        self.gear = None;
        self.parking = None;
    }

    /// Writes the remote values into `inputs`; `dt` is the simulation time of this frame.
    pub fn apply(&mut self, inputs: &mut VehicleInputs, dt: f32) {
        fn step(hold: &mut Option<Hold>, dt: f32, set: impl FnOnce(f32)) {
            if let Some(h) = hold {
                set(h.value);
                if let Some(left) = &mut h.remaining {
                    *left -= dt;
                    if *left <= 0.0 {
                        *hold = None;
                    }
                }
            }
        }
        step(&mut self.throttle, dt, |v| inputs.set_throttle(v));
        step(&mut self.brake, dt, |v| inputs.set_service_brake(v));
        step(&mut self.steer, dt, |v| inputs.set_steering(v));
        if let Some(g) = self.gear.take() {
            inputs.set_gear(g);
        }
        match self.parking.take() {
            Some(ParkingRequest::Set(on)) => inputs.set_parking_brake(on),
            Some(ParkingRequest::Toggle) => {
                inputs.set_parking_brake(!inputs.is_parking_brake_engaged());
            }
            None => {}
        }
    }

    /// Continues the odometer from `metres` (a restored save): the next frame only records the
    /// vehicle's position, so the move to the restored place is not counted as driving.
    pub fn set_odometer(&mut self, metres: f64) {
        self.odometer_m = metres;
        self.last_position = None;
    }

    /// Resets the odometer to the current pose of the vehicle.
    pub fn restart_odometer(&mut self) {
        self.odometer_m = 0.0;
        self.last_position = None;
    }
}
