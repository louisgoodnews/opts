//! Fixed-step physics integration, wheel stepping, and body suspension helpers for
//! [`VehicleSimulation`].

use super::drivetrain::DrivetrainForces;
use super::*;

/// The net force along the vehicle (N, forward positive). Moving, the brakes and the resistances
/// oppose the motion; standing, they hold: the vehicle stays put while the driving force
/// (`tractive` plus gravity along the road, `grade`) does not exceed what the brakes and the
/// rolling resistance withstand, and otherwise moves under the excess.
pub(super) fn longitudinal_force(
    speed: f32,
    tractive: f32,
    grade: f32,
    brake: f32,
    drag: f32,
    rolling: f32,
) -> f32 {
    if speed.abs() > STANDSTILL_M_S {
        let against = if speed > 0.0 { -1.0 } else { 1.0 };
        return tractive + grade + against * (brake + drag + rolling);
    }
    let driving = tractive + grade;
    let hold = brake + rolling;
    if driving.abs() <= hold {
        0.0
    } else {
        driving - driving.signum() * hold
    }
}

impl VehicleSimulation {

    /// Steps the vehicle simulation by a fixed time delta `dt`.
    pub fn step(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }

        let speed = self.chassis.forward_speed_m_s();
        let first_wheel_radius = self
            .axles
            .first()
            .map_or(0.49, |a| a.config.wheel_diameter_m * 0.5);

        // 1. Steering kinematics: update all steered axles
        let central_steer_rad = self.inputs.steering() * 0.82; // Max ~47 degrees lock
        for axle in &mut self.axles {
            axle.apply_steering(central_steer_rad, self.wheelbase_m);
        }

        if self.body.is_none() {
            self.rebuild_body();
        }

        // 2. Drivetrain & braking forces
        let moving_mass_kg = self.consist_mass_kg();
        let mut forces =
            self.drivetrain
                .step(&self.inputs, speed, first_wheel_radius, moving_mass_kg);
        let torque_cut = self.fuel_step(dt);
        forces.tractive_force_n *= torque_cut;
        let was_spinning = std::mem::replace(&mut self.wheels_spinning, false);
        let wheel_forces = self.script_drive.is_some_and(|d| {
            let d = ScriptDrive {
                wheel_torque_nm: d.wheel_torque_nm * torque_cut,
                ..d
            };
            self.compute_script_forces(d, speed, first_wheel_radius, dt, was_spinning, &mut forces)
        });

        self.apply_service_brakes(dt, &mut forces);

        // Gravity along the road: the chassis pitch of the last ground sample, nose up positive.
        let grade_n = -self.head_body.map_or(0.0, |b| b.pitch.sin()) * moving_mass_kg * GRAVITY;
        let net_longitudinal_force = longitudinal_force(
            speed,
            forces.tractive_force_n,
            grade_n,
            forces.braking_force_n,
            forces.drag_force_n,
            forces.rolling_resistance_n,
        );

        // 3. Kinematic turning yaw rate: omega = (v / L) * tan(delta)
        let yaw_rate = (speed / self.wheelbase_m) * central_steer_rad.tan();
        let yaw_torque =
            (yaw_rate - self.chassis.yaw_rate_rad_s) * self.chassis.yaw_inertia_kg_m2 * 10.0;

        // 4. Integrate chassis dynamics
        self.chassis.integrate_with_mass(
            net_longitudinal_force,
            yaw_torque,
            self.inputs.is_braking() || self.script_drive.is_some_and(|d| d.brake_force_n > 0.0),
            dt,
            moving_mass_kg,
        );

        // 5. Axle rotation and body suspension
        self.update_body_and_axles(speed, dt, wheel_forces);

        // 6. Consist kinematics
        self.update_consist(dt);
    }

}

mod body;
mod forces;
