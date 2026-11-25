//! Vehicle physics, kinematics, and powertrain simulation.
//!
//! Provides the primary [`VehicleSimulation`] controller orchestrating multi-axle
//! Ackermann steering, longitudinal propulsion, braking systems, dynamic suspension
//! compliance, and optional multi-unit trailer articulation.

pub mod axle;
pub mod aids;
pub mod brakes;
pub mod chassis;
pub mod consist;
pub mod drivetrain;
pub mod fuel;
pub mod ground;
pub mod inputs;
pub mod spec;
pub mod surface;
pub mod suspension;
pub mod trailer;
pub mod trailer_front;
pub mod wheels;

use self::brakes::BRAKE_FACTOR;
use self::consist::{driven_and_braked_load_n, UnitAxle, GRAVITY};
pub use self::consist::{dynamic_mass_kg, UnitLoad};
use self::ground::{joint_beta_deg, GroundProbe, GroundSample, UnitBody};
use self::suspension::BodySuspension;
use self::trailer::TrailerKinematics;

use glam::{DVec2, Vec2};
use serde::{Deserialize, Serialize};

/// Drive torque and brake force the vehicle's own scripts hand to the physics (`M_Wheel`,
/// `Brakeforce`/`Axle_Brakeforce_*`); when set, they replace the built-in drivetrain's traction
/// and braking for the step (resistance, adhesion and the chassis stay with the physics).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ScriptDrive {
    /// Torque at the driven wheels, summed over the axles (N·m; the unit of `M_Wheel`, see
    /// `docs/src/architecture/vehicle-scripts-route.md`, Q-VS-2). Negative drives backwards.
    pub wheel_torque_nm: f32,
    /// Total brake force at the wheels (N, never negative).
    pub brake_force_n: f32,
}

/// Newton metres at the wheels per unit of the scripts' `M_Wheel`: 1.0, the drive script forms it
/// as engine torque times the total ratio and checks its power with `M_Wheel · n_Wheel · π / 30000`
/// kW (`docs/src/architecture/script-integration-report.md`, the units read from the scripts).
pub const M_WHEEL_UNIT: f32 = 1.0;
/// Newtons at the wheels per unit of the scripts' brake forces (`Brakeforce`,
/// `Axle_Brakeforce_*`): 1.0, the brake script's maximum (`max_bremskraft`, 90,000) is about the
/// friction limit of the bus in newtons.
pub const BRAKE_FORCE_UNIT: f32 = 1.0;

/// Friction coefficient between tyres and road that limits traction and braking of a road
/// vehicle (dry asphalt).
pub const ROAD_FRICTION: f32 = 0.85;

/// Speed below which a vehicle counts as standing (m/s).
const STANDSTILL_M_S: f32 = 0.01;

mod consist_ops;
mod construct;
mod ground_ops;
mod hitch_pose;
mod simulation;
mod step;

pub use simulation::VehicleSimulation;

impl VehicleSimulation {
    /// Position and heading (compass radians) of the chassis in the precision of the consist.
    pub(super) fn chassis_pose(&self) -> (DVec2, f64) {
        let p = self.chassis.position;
        (
            DVec2::new(f64::from(p.x), f64::from(p.y)),
            f64::from(self.chassis.heading_rad),
        )
    }

    /// Places the chassis at `position` heading `heading_rad` (compass radians) and re-seeds the
    /// consist behind it: the explicit response to a spawn, respawn or reset. Velocities of the
    /// chassis are left to the caller.
    pub fn teleport(&mut self, position: Vec2, heading_rad: f32) {
        self.chassis.position = position;
        self.chassis.heading_rad = heading_rad;
        self.reseed_consist();
        self.teleports = self.teleports.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod wheel_model_tests;
