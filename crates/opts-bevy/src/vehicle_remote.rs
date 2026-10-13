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

    /// Resets the odometer to the current pose of the vehicle.
    pub fn restart_odometer(&mut self) {
        self.odometer_m = 0.0;
        self.last_position = None;
    }
}

/// Raises script trigger `name` on the player's vehicle; returns whether a script handled it, or
/// `None` when there is no vehicle with scripts.
pub fn raise_trigger(world: &mut World, name: &str) -> Option<bool> {
    let mut q = world.query_filtered::<&VehicleScripts, With<PlayerVehicle>>();
    let scripts = q.iter(world).next()?;
    let mut engine = scripts.engine.lock().ok()?;
    let handled = engine.trigger(name);
    tracing::debug!(trigger = name, handled, "vehicle script trigger (remote)");
    Some(handled)
}

/// Raises the scheduled `_off` triggers whose time has come.
pub fn release_trigger_system(
    time: Res<Time>,
    mut remote: ResMut<RemoteControls>,
    vehicles: Query<&VehicleScripts, With<PlayerVehicle>>,
) {
    if remote.release_triggers.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    let mut due = Vec::new();
    remote.release_triggers.retain_mut(|(name, left)| {
        *left -= dt;
        if *left <= 0.0 {
            due.push(std::mem::take(name));
            false
        } else {
            true
        }
    });
    for name in due {
        for scripts in &vehicles {
            if let Ok(mut engine) = scripts.engine.lock() {
                let handled = engine.trigger(&name);
                tracing::debug!(trigger = %name, handled, "vehicle script trigger (remote release)");
            }
        }
    }
}

/// The negation of the run conditions of the vehicle input system (no vehicle input now).
#[must_use]
pub fn vehicle_input_closed(
    camera: Option<Res<crate::camera::CameraState>>,
    modal: Option<Res<crate::reporting::state::ModalActive>>,
    menu: Option<Res<crate::reporting::state::MenuModalActive>>,
    viewpoint: Option<Res<crate::camera::ViewpointState>>,
) -> bool {
    let accepts = camera.is_none_or(|s| s.mode != crate::camera::CameraMode::FreeRoam)
        && !viewpoint.is_some_and(|v| v.editing);
    let open = modal.is_some_and(|m| m.0) || menu.is_some_and(|m| m.0);
    !accepts || open
}

/// Drops held controls while the input gates are closed, so nothing fires when they reopen.
pub fn flush_remote_when_gated(mut remote: ResMut<RemoteControls>) {
    if remote.is_active() {
        remote.release_all();
    }
}

/// Counts the distance driven; a jump of more than 50 m in one frame is a placement, not driving.
pub fn odometer_system(
    mut remote: ResMut<RemoteControls>,
    added: Query<(), Added<PlayerVehicle>>,
    vehicles: Query<&VehicleController, With<PlayerVehicle>>,
) {
    let Ok(c) = vehicles.single() else {
        return;
    };
    let ch = &c.sim.chassis;
    if !added.is_empty() || remote.start_pose.is_none() {
        remote.start_pose = Some((ch.position, ch.heading_rad, ch.elevation_z));
        remote.restart_odometer();
    }
    if let Some(last) = remote.last_position {
        let d = ch.position.distance(last);
        if d < 50.0 {
            remote.odometer_m += f64::from(d);
        }
    }
    remote.last_position = Some(ch.position);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_values_override_and_expire_in_simulation_time() {
        let mut remote = RemoteControls {
            throttle: Some(Hold {
                value: 0.6,
                remaining: Some(0.25),
            }),
            steer: Some(Hold {
                value: -1.0,
                remaining: None,
            }),
            ..Default::default()
        };
        let mut inputs = VehicleInputs::new();
        remote.apply(&mut inputs, 0.1);
        assert!((inputs.throttle() - 0.6).abs() < 1e-6);
        assert!((inputs.steering() + 1.0).abs() < 1e-6);
        remote.apply(&mut inputs, 0.1);
        assert!(remote.throttle.is_some());
        remote.apply(&mut inputs, 0.1);
        assert!(
            remote.throttle.is_none(),
            "0.3 s have run out the 0.25 s hold"
        );
        assert!(remote.steer.is_some(), "an open-ended hold stays");
    }

    #[test]
    fn gear_and_parking_brake_are_one_shot() {
        let mut remote = RemoteControls {
            gear: Some(GearPosition::Drive),
            parking: Some(ParkingRequest::Set(false)),
            ..Default::default()
        };
        let mut inputs = VehicleInputs::new();
        assert!(inputs.is_parking_brake_engaged());
        remote.apply(&mut inputs, 0.016);
        assert_eq!(inputs.gear(), GearPosition::Drive);
        assert!(!inputs.is_parking_brake_engaged());
        assert!(!remote.is_active());
        remote.parking = Some(ParkingRequest::Toggle);
        remote.apply(&mut inputs, 0.016);
        assert!(inputs.is_parking_brake_engaged());
    }

    #[test]
    fn release_all_clears_everything() {
        let mut remote = RemoteControls {
            brake: Some(Hold {
                value: 1.0,
                remaining: None,
            }),
            gear: Some(GearPosition::Reverse),
            ..Default::default()
        };
        assert!(remote.is_active());
        remote.release_all();
        assert!(!remote.is_active());
    }
}
