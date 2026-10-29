//! Powertrain, transmission gearing, braking dynamics, and aerodynamic resistances.
//!
//! Provides a standalone, deterministic vehicle powertrain simulation supporting
//! turbodiesel engine torque curves, automatic gear ratio selection, retarder braking,
//! pneumatic service friction brakes, and aerodynamic drag.

use crate::vehicle::inputs::{GearPosition, VehicleInputs};
use serde::{Deserialize, Serialize};

/// Internal combustion turbodiesel engine specification and torque parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineSpec {
    /// Engine idle rotational speed in revolutions per minute.
    pub idle_rpm: f32,
    /// Maximum governed engine redline speed in revolutions per minute.
    pub max_rpm: f32,
    /// Maximum engine output torque in Newton-meters.
    pub peak_torque_nm: f32,
    /// Engine speed at which peak torque is delivered in RPM.
    pub peak_torque_rpm: f32,
    /// Rated peak power in kilowatts.
    pub rated_power_kw: f32,
}

impl Default for EngineSpec {
    fn default() -> Self {
        Self {
            idle_rpm: 600.0,
            max_rpm: 2200.0,
            peak_torque_nm: 1100.0, // Standard 12m transit diesel (e.g. MAN D08/D20 series)
            peak_torque_rpm: 1200.0,
            rated_power_kw: 210.0, // ~285 hp
        }
    }
}

impl EngineSpec {
    /// Computes engine crankshaft torque output based on current RPM and throttle pedal depression.
    #[must_use]
    pub fn compute_torque(&self, current_rpm: f32, throttle: f32) -> f32 {
        if throttle <= 0.001 || current_rpm > self.max_rpm {
            return 0.0;
        }

        let clamped_rpm = current_rpm.clamp(self.idle_rpm, self.max_rpm);
        let rpm_span = (self.max_rpm - self.idle_rpm).max(100.0);
        let norm_diff = (clamped_rpm - self.peak_torque_rpm) / rpm_span;

        // Quadratic curve delivering broad flat torque plateau around peak_torque_rpm
        let curve_factor = (1.0 - 1.2 * norm_diff * norm_diff).clamp(0.2, 1.0);
        self.peak_torque_nm * curve_factor * throttle
    }
}

/// Transmission gear ratios and differential final drive specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransmissionSpec {
    /// Forward gear drive ratios (1st to Nth).
    pub forward_ratios: Vec<f32>,
    /// Reverse gear drive ratio.
    pub reverse_ratio: f32,
    /// Axle differential final drive reduction ratio.
    pub final_drive_ratio: f32,
    /// Mechanical drivetrain transmission efficiency factor (0.0 to 1.0).
    pub efficiency: f32,
    /// Automatic upshift engine speed threshold in RPM.
    pub upshift_rpm: f32,
    /// Automatic downshift engine speed threshold in RPM.
    pub downshift_rpm: f32,
}

impl Default for TransmissionSpec {
    fn default() -> Self {
        Self {
            // Standard ZF 4 HP 500 / Voith DIWA 4-speed automatic transit ratios
            forward_ratios: vec![3.43, 2.01, 1.42, 1.00],
            reverse_ratio: 3.20,
            final_drive_ratio: 4.76,
            efficiency: 0.88,
            upshift_rpm: 1850.0,
            downshift_rpm: 1050.0,
        }
    }
}

/// Aerodynamic resistance properties of the vehicle body.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AerodynamicSpec {
    /// Air drag coefficient (typically ~0.60 to 0.70 for transit buses).
    pub drag_coefficient: f32,
    /// Frontal projected cross-sectional area in square meters.
    pub frontal_area_m2: f32,
    /// Ambient air density in kilograms per cubic meter.
    pub air_density_kg_m3: f32,
}

impl Default for AerodynamicSpec {
    fn default() -> Self {
        Self {
            drag_coefficient: 0.65,
            frontal_area_m2: 7.5, // 2.5m width * 3.0m height
            air_density_kg_m3: 1.225,
        }
    }
}

impl AerodynamicSpec {
    /// Computes aerodynamic retarding force in Newtons ($F = \frac{1}{2} \rho C_d A v^2$).
    #[must_use]
    pub fn compute_drag_force(&self, speed_m_s: f32) -> f32 {
        0.5 * self.air_density_kg_m3
            * self.drag_coefficient
            * self.frontal_area_m2
            * speed_m_s
            * speed_m_s
    }
}

/// Dynamic output forces delivered by the powertrain and brake systems per tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrivetrainForces {
    /// Forward tractive propulsion force in Newtons.
    pub tractive_force_n: f32,
    /// Total retarding brake force (service + retarder + parking) in Newtons.
    pub braking_force_n: f32,
    /// Aerodynamic retarding drag force in Newtons.
    pub drag_force_n: f32,
    /// Rolling resistance retarding force in Newtons.
    pub rolling_resistance_n: f32,
    /// Engine rotational speed in RPM.
    pub engine_rpm: f32,
    /// Active gear index (0 = Neutral, 1..=N = Forward, 255 = Reverse).
    pub active_gear: u8,
}

/// Rolling resistance coefficient of the road vehicle (force = coefficient · weight).
pub const ROLLING_RESISTANCE_COEFF: f32 = 0.012;

/// High-level powertrain and brake simulator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrivetrainModel {
    /// Engine specification.
    pub engine: EngineSpec,
    /// Transmission specification.
    pub transmission: TransmissionSpec,
    /// Aerodynamic specification.
    pub aero: AerodynamicSpec,
    /// Current engine speed in RPM.
    pub current_rpm: f32,
    /// Current engaged forward gear index (1-indexed).
    pub current_gear: usize,
    /// Rolling resistance as force over weight: the vehicle file's `[rollwiderstand]` over its
    /// weight when it states one (`VehicleDynamicsSpec`), else [`ROLLING_RESISTANCE_COEFF`].
    #[serde(default = "default_rolling_coefficient")]
    pub rolling_coefficient: f32,
}

fn default_rolling_coefficient() -> f32 {
    ROLLING_RESISTANCE_COEFF
}

impl Default for DrivetrainModel {
    fn default() -> Self {
        Self {
            engine: EngineSpec::default(),
            transmission: TransmissionSpec::default(),
            aero: AerodynamicSpec::default(),
            current_rpm: 600.0,
            current_gear: 1,
            rolling_coefficient: ROLLING_RESISTANCE_COEFF,
        }
    }
}

impl DrivetrainModel {
    /// Creates a default commercial transit bus drivetrain model.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

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

#[cfg(test)]
mod tests;
