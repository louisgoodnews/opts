//! Fixed-step physics integration, wheel stepping, and body suspension helpers for
//! [`VehicleSimulation`].

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

        // The wheel loads the tyres need exist from the first step on.
        if self.body.is_none() {
            self.rebuild_body();
        }

        // 2. Drivetrain & braking forces
        let moving_mass_kg = self.consist_mass_kg();
        let mut forces =
            self.drivetrain
                .step(&self.inputs, speed, first_wheel_radius, moving_mass_kg);
        let mut wheel_forces = false;
        let was_spinning = std::mem::replace(&mut self.wheels_spinning, false);
        if let Some(d) = self.script_drive {
            if speed.abs() > STANDSTILL_M_S {
                // Moving: the scripts' torque and brakes act on the spinning wheels; the tyres
                // turn them into the longitudinal force (never above the friction limit).
                if !was_spinning {
                    for axle in &mut self.axles {
                        for w in [&mut axle.left_wheel, &mut axle.right_wheel] {
                            w.angular_velocity_rad_s = speed / w.rolling_radius_m.max(0.1);
                        }
                    }
                }
                forces.tractive_force_n = self.step_wheels(d, speed, dt);
                forces.braking_force_n = 0.0;
                wheel_forces = true;
                self.wheels_spinning = true;
            } else {
                // Standing: static hold against the scripts' torque, limited by the tyres.
                let (driven_n, braked_n) = self.friction_loads_n();
                let (drive_limit, brake_limit) =
                    (ROAD_FRICTION * driven_n, ROAD_FRICTION * braked_n);
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
            }
        }

        // The service brake: built-in drivetrain only (a script drive brings its own force).
        if self.script_drive.is_none() {
            let (caps, limits) = self.brake_axles();
            self.brakes
                .size_for(caps.iter().sum::<f32>() / BRAKE_FACTOR);
            forces.braking_force_n +=
                self.brakes
                    .step(self.inputs.service_brake(), &caps, &limits, dt);
        } else {
            self.brakes.pressure = 0.0;
        }

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
            // Under script drive the scripts' brakes count, not the pedal alone.
            self.inputs.is_braking() || self.script_drive.is_some_and(|d| d.brake_force_n > 0.0),
            dt,
            moving_mass_kg,
        );

        // 5. Axle rotation and the body on its wheels: heave, pitch, roll and the load on every
        // wheel from the accelerations and the ground under the wheels.
        if self.body.is_none() {
            self.rebuild_body();
        }
        let a_lat = speed * self.chassis.yaw_rate_rad_s;
        let a_long = self.chassis.local_acceleration.y;
        self.sync_payload();
        if let Some(body) = self.body.as_mut() {
            body.step(dt, a_long, a_lat, &self.wheel_ground_m);
            for (i, axle) in self.axles.iter_mut().enumerate() {
                if let Some(w) = body.wheels.get(i * 2..i * 2 + 2) {
                    axle.left_wheel.vertical_load_n = w[0].load_n;
                    axle.right_wheel.vertical_load_n = w[1].load_n;
                    axle.suspension_deflection_m = 0.5 * (w[0].compression_m + w[1].compression_m);
                    axle.suspension_velocity_m_s =
                        0.5 * (w[0].compression_rate_m_s + w[1].compression_rate_m_s);
                }
            }
        }
        if !wheel_forces {
            for axle in &mut self.axles {
                axle.update_rotation(self.chassis.forward_speed_m_s(), dt);
            }
        }

        // 6. Consist kinematics: every unit follows the one ahead, starting with the chassis.
        let mut leader = self.chassis_pose();
        for unit in &mut self.consist {
            let st = unit.step(leader.0, leader.1, f64::from(dt));
            leader = (st.origin, st.heading);
        }
    }

    /// Advances the wheels of the head's axles under the scripts' torque and brake force and
    /// returns the sum of their longitudinal tyre forces (N). The torque goes to the driven
    /// wheels (all of them when no axle is marked driven) in equal parts, the brake force to the
    /// head's axles in proportion to their rated force.
    fn step_wheels(&mut self, d: ScriptDrive, speed: f32, dt: f32) -> f32 {
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
                let st = wheels::step_wheel(w, drive, brake_nm, load, speed, ROAD_FRICTION, dt);
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
                    let limit = ROAD_FRICTION * load;
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
            total -= speed.signum() * trailer_brake_n.min(ROAD_FRICTION * loads);
        }
        total
    }

    /// Sets the factor on the spring of wheel `wheel` (axle `n`: left `2n`, right `2n + 1`) from
    /// the scripts' `Axle_Springfactor`; non-finite values and wheels that do not exist are
    /// ignored, the factor is held to [`suspension::SPRING_FACTOR_RANGE`].
    pub fn set_spring_factor(&mut self, wheel: usize, factor: f32) {
        if !factor.is_finite() {
            return;
        }
        let (lo, hi) = suspension::SPRING_FACTOR_RANGE;
        if let Some(w) = self.body.as_mut().and_then(|b| b.wheels.get_mut(wheel)) {
            w.spring_factor = factor.clamp(lo, hi);
        }
    }

    /// Builds the body-on-wheels model from the axles, the mass and the spec (at rest, on the
    /// static axle loads).
    pub fn rebuild_body(&mut self) {
        let positions: Vec<f32> = self.axles.iter().map(|a| a.config.longitudinal_m).collect();
        let loads = self
            .suspension
            .compute_static_loads(&positions, self.chassis.total_mass_kg);
        let configs: Vec<_> = self.axles.iter().map(|a| a.config.clone()).collect();
        self.body = Some(BodySuspension::new(
            &configs,
            &loads,
            self.chassis.total_mass_kg,
            self.spec.pitch_inertia_kg_m2.value,
            self.spec.roll_inertia_kg_m2.value,
            self.suspension.cg_height_m,
        ));
        for (axle, load) in self.axles.iter_mut().zip(&loads) {
            axle.left_wheel.vertical_load_n = load * 0.5;
            axle.right_wheel.vertical_load_n = load * 0.5;
        }
    }

    /// Brake force at full pressure and what the tyres carry now, per axle over all units (the
    /// head's axles first, then each following unit's): the inputs of the [`BrakeSystem`].
    fn brake_axles(&self) -> (Vec<f32>, Vec<f32>) {
        let (mut caps, mut limits) = (Vec::new(), Vec::new());
        for (i, a) in self.axles.iter().enumerate() {
            caps.push(BRAKE_FACTOR * a.config.max_force_n);
            let load = self.body.as_ref().map_or(
                a.left_wheel.vertical_load_n + a.right_wheel.vertical_load_n,
                |b| b.axle_load_n(i),
            );
            limits.push(ROAD_FRICTION * load);
        }
        for unit in &self.trailer_loads {
            for (axle, load) in unit.axles.iter().zip(unit.axle_loads_n()) {
                caps.push(BRAKE_FACTOR * axle.rated_n);
                limits.push(ROAD_FRICTION * load);
            }
        }
        (caps, limits)
    }
}
