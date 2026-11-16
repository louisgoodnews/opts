//! the forces of a step: scripted drive and brake, and the wheel forces.

use super::*;

impl VehicleSimulation {
    pub(super) fn compute_script_forces(
        &mut self,
        d: ScriptDrive,
        speed: f32,
        first_wheel_radius: f32,
        dt: f32,
        was_spinning: bool,
        forces: &mut DrivetrainForces,
    ) -> bool {
        let mu = self.road_friction;
        if speed.abs() > STANDSTILL_M_S {
            if !was_spinning {
                for axle in &mut self.axles {
                    for w in [&mut axle.left_wheel, &mut axle.right_wheel] {
                        w.angular_velocity_rad_s = speed / w.rolling_radius_m.max(0.1);
                    }
                }
            }
            forces.tractive_force_n = self.step_wheels(d, speed, dt);
            forces.braking_force_n = 0.0;
            self.wheels_spinning = true;
            true
        } else {
            let (driven_n, braked_n) = self.friction_loads_n();
            let (drive_limit, brake_limit) = (mu * driven_n, mu * braked_n);
            let driven_radius = self
                .axles
                .iter()
                .find(|a| a.config.is_driven)
                .map_or(first_wheel_radius, |a| a.config.wheel_diameter_m * 0.5)
                .max(0.1);
            forces.tractive_force_n =
                (d.wheel_torque_nm / driven_radius).clamp(-drive_limit, drive_limit);
            forces.braking_force_n = d.brake_force_n.clamp(0.0, brake_limit);
            for axle in &mut self.axles {
                for w in [&mut axle.left_wheel, &mut axle.right_wheel] {
                    w.angular_velocity_rad_s = speed / w.rolling_radius_m.max(0.1);
                }
            }
            false
        }
    }

    /// Advances the wheels of the head's axles under the scripts' torque and brake force and
    /// returns the sum of their longitudinal tyre forces (N). The torque goes to the driven
    /// wheels (all of them when no axle is marked driven) in equal parts, the brake force to the
    /// head's axles in proportion to their rated force.
    pub(super) fn step_wheels(&mut self, d: ScriptDrive, speed: f32, dt: f32) -> f32 {
        let mu = self.road_friction;
        let unit_driven = self
            .trailer_loads
            .iter()
            .flat_map(|u| u.axles.iter())
            .filter(|a| a.is_driven)
            .count();
        let any_driven = self.axles.iter().any(|a| a.config.is_driven) || unit_driven > 0;
        let driven_wheels = (self
            .axles
            .iter()
            .filter(|a| a.config.is_driven || !any_driven)
            .count()
            + unit_driven)
            * 2;
        let torque_each = d.wheel_torque_nm / driven_wheels.max(1) as f32;
        let rated_head: f32 = self.axles.iter().map(|a| a.config.max_force_n).sum();
        let rated_all: f32 = rated_head
            + self
                .trailer_loads
                .iter()
                .flat_map(|u| u.axles.iter())
                .map(|a| a.rated_n)
                .sum::<f32>();
        let head_brake_n = if rated_all > 0.0 {
            d.brake_force_n * rated_head / rated_all
        } else {
            0.0
        };
        let mut total = 0.0;
        let axle_count = self.axles.len().max(1) as f32;
        for axle in &mut self.axles {
            let drive = if axle.config.is_driven || !any_driven {
                torque_each
            } else {
                0.0
            };
            let share = if rated_head > 0.0 {
                axle.config.max_force_n / rated_head
            } else {
                1.0 / axle_count
            };
            for w in [&mut axle.left_wheel, &mut axle.right_wheel] {
                let r = w.rolling_radius_m.max(0.1);
                let brake_nm = head_brake_n * share * 0.5 * r;
                let load = w.vertical_load_n;
                let st = wheels::step_wheel(w, drive, brake_nm, load, speed, mu, dt);
                total += st.force_n;
            }
        }
        // The driven axles of the following units (the driven axle of an articulated bus hangs
        // in its second unit): the tyre carries the torque as far as its load allows.
        let r_unit = self
            .axles
            .first()
            .map_or(0.49, |a| a.left_wheel.rolling_radius_m)
            .max(0.1);
        for unit in &self.trailer_loads {
            for (axle, load) in unit.axles.iter().zip(unit.axle_loads_n()) {
                if axle.is_driven {
                    let limit = mu * load;
                    total += (2.0 * torque_each / r_unit).clamp(-limit, limit);
                }
            }
        }
        // The following units: their brake share, as far as their tyres carry it.
        let trailer_brake_n = d.brake_force_n - head_brake_n;
        if trailer_brake_n > 0.0 {
            let loads: f32 = self
                .trailer_loads
                .iter()
                .flat_map(|u| u.axle_loads_n())
                .sum();
            total -= speed.signum() * trailer_brake_n.min(mu * loads);
        }
        total
    }
}
