//! body, axle and consist updates after a step, service brakes and the spring factors.

use super::*;

impl VehicleSimulation {
    pub(super) fn apply_service_brakes(&mut self, dt: f32, forces: &mut DrivetrainForces) {
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
    }

    pub(super) fn update_body_and_axles(&mut self, speed: f32, dt: f32, wheel_forces: bool) {
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
    }

    pub(super) fn update_consist(&mut self, dt: f32) {
        let mut leader = self.chassis_pose();
        for unit in &mut self.consist {
            let st = unit.step(leader.0, leader.1, f64::from(dt));
            leader = (st.origin, st.heading);
        }
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
    pub(super) fn brake_axles(&self) -> (Vec<f32>, Vec<f32>) {
        let (mut caps, mut limits) = (Vec::new(), Vec::new());
        let mu = self.road_friction;
        for (i, a) in self.axles.iter().enumerate() {
            caps.push(BRAKE_FACTOR * a.config.max_force_n);
            let load = self.body.as_ref().map_or(
                a.left_wheel.vertical_load_n + a.right_wheel.vertical_load_n,
                |b| b.axle_load_n(i),
            );
            limits.push(mu * load);
        }
        for unit in &self.trailer_loads {
            for (axle, load) in unit.axles.iter().zip(unit.axle_loads_n()) {
                caps.push(BRAKE_FACTOR * axle.rated_n);
                limits.push(mu * load);
            }
        }
        (caps, limits)
    }
}
