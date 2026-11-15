//! The per-step drivetrain solve: gear selection, tractive and braking forces, and the resistances.

use super::model::{DrivetrainForces, DrivetrainModel};
use crate::vehicle::inputs::{GearPosition, VehicleInputs};

impl DrivetrainModel {
    fn resolve_gear_ratio(&mut self, gear: GearPosition) -> (f32, u8) {
        match gear {
            GearPosition::Neutral => (0.0, 0),
            GearPosition::Reverse => (
                -self.transmission.reverse_ratio * self.transmission.final_drive_ratio,
                255,
            ),
            GearPosition::Forward(gear) => {
                let idx = (gear as usize)
                    .saturating_sub(1)
                    .min(self.transmission.forward_ratios.len().saturating_sub(1));
                self.current_gear = idx + 1;
                (
                    self.transmission.forward_ratios[idx] * self.transmission.final_drive_ratio,
                    self.current_gear as u8,
                )
            }
            GearPosition::Drive => {
                if self.current_rpm > self.transmission.upshift_rpm
                    && self.current_gear < self.transmission.forward_ratios.len()
                {
                    self.current_gear += 1;
                } else if self.current_rpm < self.transmission.downshift_rpm
                    && self.current_gear > 1
                {
                    self.current_gear -= 1;
                }
                let idx = self
                    .current_gear
                    .saturating_sub(1)
                    .min(self.transmission.forward_ratios.len() - 1);
                (
                    self.transmission.forward_ratios[idx] * self.transmission.final_drive_ratio,
                    self.current_gear as u8,
                )
            }
        }
    }

    fn compute_tractive_force(&self, throttle: f32, ratio: f32, wheel_radius: f32) -> f32 {
        let engine_torque = self.engine.compute_torque(self.current_rpm, throttle);
        if ratio != 0.0 {
            let sign = if ratio > 0.0 { 1.0 } else { -1.0 };
            (engine_torque * ratio.abs() * self.transmission.efficiency / wheel_radius) * sign
        } else {
            0.0
        }
    }

    fn compute_braking_force(
        &self,
        inputs: &VehicleInputs,
        abs_speed: f32,
        wheel_radius: f32,
        vehicle_mass_kg: f32,
    ) -> f32 {
        let g = 9.81;
        let retarder_torque_nm = (inputs.retarder_stage() as f32) * 500.0;
        let retarder_brake_n = (retarder_torque_nm * self.transmission.final_drive_ratio
            / wheel_radius)
            * (abs_speed / 2.0).min(1.0);

        let parking_brake_n = if inputs.is_parking_brake_engaged() {
            0.5 * vehicle_mass_kg * g
        } else {
            0.0
        };
        retarder_brake_n + parking_brake_n
    }

    /// Evaluates drivetrain propulsion, braking, and motion resistances for one simulation step.
    ///
    /// # Arguments
    /// * `inputs` - Driver control inputs.
    /// * `speed_m_s` - Current forward longitudinal speed in meters per second.
    /// * `wheel_radius_m` - Dynamic wheel rolling radius in meters.
    /// * `vehicle_mass_kg` - Total vehicle curb weight in kilograms.
    ///
    /// # Returns
    /// A [`DrivetrainForces`] snapshot containing resolved longitudinal forces.
    #[must_use]
    pub fn step(
        &mut self,
        inputs: &VehicleInputs,
        speed_m_s: f32,
        wheel_radius_m: f32,
        vehicle_mass_kg: f32,
    ) -> DrivetrainForces {
        let abs_speed = speed_m_s.abs();
        let wheel_radius = wheel_radius_m.max(0.1);
        let g = 9.81;

        // 1. Determine active gear and overall drive ratio
        let (ratio, active_gear_tag) = self.resolve_gear_ratio(inputs.gear());

        // 2. Compute engine RPM from wheel speed and ratio
        let wheel_rad_s = abs_speed / wheel_radius;
        let driveshaft_rad_s = wheel_rad_s * ratio.abs();
        let mechanical_rpm = driveshaft_rad_s * (60.0 / std::f32::consts::TAU);
        self.current_rpm = mechanical_rpm
            .max(self.engine.idle_rpm)
            .min(self.engine.max_rpm);

        // 3. Engine torque and tractive force
        let tractive_force_n = self.compute_tractive_force(inputs.throttle(), ratio, wheel_radius);

        // 4. Braking forces (Retarder + Parking)
        let braking_force_n =
            self.compute_braking_force(inputs, abs_speed, wheel_radius, vehicle_mass_kg);

        // 5. Resistances
        let drag_force_n = self.aero.compute_drag_force(abs_speed);
        let rolling_resistance_n = self.rolling_coefficient * vehicle_mass_kg * g;

        DrivetrainForces {
            tractive_force_n,
            braking_force_n,
            drag_force_n,
            rolling_resistance_n,
            engine_rpm: self.current_rpm,
            active_gear: active_gear_tag,
        }
    }
}
