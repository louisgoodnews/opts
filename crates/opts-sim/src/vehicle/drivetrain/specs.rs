//! The specifications of the drivetrain: engine torque curve, gearbox and aerodynamic drag.

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
