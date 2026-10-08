//! Driver inputs and vehicle control state.
//!
//! Provides normalized control inputs for steering, throttle, service brakes,
//! retarder braking stages, parking brake, and gear selector positions.

use serde::{Deserialize, Serialize};

/// Transmission gear selector mode or ratio position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GearPosition {
    /// Disengaged transmission in neutral.
    #[default]
    Neutral,
    /// Forward automatic drive mode (computer selects optimum ratio).
    Drive,
    /// Forward fixed or hold gear (1-indexed gear number).
    Forward(u8),
    /// Reverse gear.
    Reverse,
}

/// Driver control inputs applied to the vehicle per simulation tick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleInputs {
    /// Normalized steering input in the range `[-1.0, 1.0]` (negative = left, positive = right).
    steering: f32,
    /// Normalized accelerator / throttle pedal input in the range `[0.0, 1.0]`.
    throttle: f32,
    /// Normalized service brake pedal input in the range `[0.0, 1.0]`.
    service_brake: f32,
    /// Hydrodynamic retarder braking stage (typically 0 = disengaged, 1..=5 = increasing torque).
    retarder_stage: u8,
    /// Mechanical spring parking brake / handbrake engagement status.
    parking_brake: bool,
    /// Transmission selector position.
    gear: GearPosition,
}

impl Default for VehicleInputs {
    fn default() -> Self {
        Self {
            steering: 0.0,
            throttle: 0.0,
            service_brake: 0.0,
            retarder_stage: 0,
            parking_brake: true, // Vehicles default to parked with handbrake set
            gear: GearPosition::Neutral,
        }
    }
}

impl VehicleInputs {
    /// Creates a new vehicle input set with default stationary/parked state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the steering wheel input, clamping to `[-1.0, 1.0]`.
    pub fn set_steering(&mut self, value: f32) {
        self.steering = value.clamp(-1.0, 1.0);
    }

    /// Sets the throttle pedal input, clamping to `[0.0, 1.0]`.
    pub fn set_throttle(&mut self, value: f32) {
        self.throttle = value.clamp(0.0, 1.0);
    }

    /// Sets the service brake pedal input, clamping to `[0.0, 1.0]`.
    pub fn set_service_brake(&mut self, value: f32) {
        self.service_brake = value.clamp(0.0, 1.0);
    }

    /// Sets the retarder brake stage (typically 0 to 5).
    pub fn set_retarder_stage(&mut self, stage: u8) {
        self.retarder_stage = stage;
    }

    /// Sets the parking brake / handbrake state.
    pub fn set_parking_brake(&mut self, engaged: bool) {
        self.parking_brake = engaged;
    }

    /// Sets the transmission gear selector position.
    pub fn set_gear(&mut self, gear: GearPosition) {
        self.gear = gear;
    }

    /// Returns the normalized steering value `[-1.0, 1.0]`.
    #[must_use]
    pub fn steering(&self) -> f32 {
        self.steering
    }

    /// Returns the normalized throttle pedal value `[0.0, 1.0]`.
    #[must_use]
    pub fn throttle(&self) -> f32 {
        self.throttle
    }

    /// Returns the normalized service brake pedal value `[0.0, 1.0]`.
    #[must_use]
    pub fn service_brake(&self) -> f32 {
        self.service_brake
    }

    /// Returns the active retarder braking stage.
    #[must_use]
    pub fn retarder_stage(&self) -> u8 {
        self.retarder_stage
    }

    /// Returns whether the parking brake is engaged.
    #[must_use]
    pub fn is_parking_brake_engaged(&self) -> bool {
        self.parking_brake
    }

    /// Returns the current transmission gear position.
    #[must_use]
    pub fn gear(&self) -> GearPosition {
        self.gear
    }

    /// Returns `true` if any braking effort (service brake, retarder, or parking brake) is requested.
    #[must_use]
    pub fn is_braking(&self) -> bool {
        self.parking_brake || self.service_brake > 0.001 || self.retarder_stage > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inputs_clamping() {
        let mut inputs = VehicleInputs::new();
        inputs.set_steering(2.5);
        assert!((inputs.steering() - 1.0).abs() < f32::EPSILON);

        inputs.set_steering(-3.0);
        assert!((inputs.steering() - -1.0).abs() < f32::EPSILON);

        inputs.set_throttle(1.5);
        assert!((inputs.throttle() - 1.0).abs() < f32::EPSILON);

        inputs.set_throttle(-0.5);
        assert!((inputs.throttle() - 0.0).abs() < f32::EPSILON);

        inputs.set_service_brake(1.2);
        assert!((inputs.service_brake() - 1.0).abs() < f32::EPSILON);

        inputs.set_service_brake(-0.2);
        assert!((inputs.service_brake() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_braking_detection() {
        let mut inputs = VehicleInputs::new();
        assert!(inputs.is_braking()); // default parking brake engaged

        inputs.set_parking_brake(false);
        assert!(!inputs.is_braking());

        inputs.set_service_brake(0.1);
        assert!(inputs.is_braking());

        inputs.set_service_brake(0.0);
        inputs.set_retarder_stage(2);
        assert!(inputs.is_braking());
    }
}
