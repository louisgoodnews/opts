//! The saved live state of one session vehicle as plain data (pose, velocity, driver inputs, gear
//! and the variables of its scripts). The capture and apply systems stay in `opts-game`.

use opts_sim::vehicle::inputs::GearPosition;
use serde::{Deserialize, Serialize};

/// What a vehicle was doing when it was saved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleState {
    /// Absolute OMSI position, east (m).
    pub x: f64,
    /// Absolute OMSI position, north (m).
    pub y: f64,
    /// Height (m).
    pub z: f64,
    /// Heading, degrees as the physics keeps them.
    pub heading_deg: f64,
    /// Velocity along the vehicle's axes (m/s): lateral, forward.
    pub velocity: [f32; 2],
    /// Yaw rate (rad/s).
    pub yaw_rate: f32,
    /// Steering input, `-1..1`.
    pub steering: f32,
    /// Throttle pedal, `0..1`.
    pub throttle: f32,
    /// Service brake pedal, `0..1`.
    pub service_brake: f32,
    /// Retarder stage.
    pub retarder_stage: u8,
    /// Parking brake engaged.
    pub parking_brake: bool,
    /// Gear selector.
    pub gear: GearPosition,
    /// Engine speed (rpm).
    pub rpm: f32,
    /// Index of the engaged gear in the transmission.
    pub gear_index: usize,
    /// Float variables of the scripts that are not zero (doors, lights, switches, counters).
    #[serde(default)]
    pub script_floats: Vec<(String, f32)>,
    /// String variables of the scripts that are not empty.
    #[serde(default)]
    pub script_strings: Vec<(String, String)>,
}
