//! The per-wheel and per-door input families, the axle and wheel inputs and the lifespan constant.

/// Per-wheel input families: `<family>_<axle>_<l|r>` for four axles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WheelFamily {
    /// `Wheel_RotationSpeed_n_s` (rpm).
    RotationSpeed,
    /// `Wheel_Rotation_n_s` (rad, growing while driving forward); trailer scripts write it.
    Rotation,
    /// `Axle_Steering_n_s` (rad).
    Steering,
    /// `Axle_Suspension_n_s` (m).
    Suspension,
}

/// The wheel families with their name stem.
pub const WHEEL_FAMILIES: &[(WheelFamily, &str, &str)] = &[
    (WheelFamily::RotationSpeed, "wheel_rotationspeed", "rpm"),
    (WheelFamily::Rotation, "wheel_rotation", "rad"),
    (WheelFamily::Steering, "axle_steering", "rad"),
    (WheelFamily::Suspension, "axle_suspension", "m"),
];

/// Axles the predefined names cover.
pub const AXLES: usize = 4;

/// Door request families: `PAX_Entry<n>_Req`, `PAX_Exit<n>_Req` for eight doors; scripts
/// clear them, so they are shared.
pub const DOOR_FAMILIES: &[&str] = &["pax_entry", "pax_exit"];

/// Doors the predefined names cover.
pub const DOORS: usize = 8;

/// One wheel of the host's vehicle.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WheelInput {
    /// Rotational speed (rpm).
    pub speed_rpm: f32,
    /// Accumulated rotation (rad).
    pub rotation_rad: f32,
    /// Steering angle (rad, positive turning right).
    pub steering_rad: f32,
    /// Suspension travel (m): the extension from the unloaded spring, negative at rest.
    pub suspension_m: f32,
}

/// One axle: left and right wheel.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AxleInput {
    /// Left wheel.
    pub left: WheelInput,
    /// Right wheel.
    pub right: WheelInput,
}

/// The lifespan factor of a vehicle that never breaks down (OPTS has no breakdown option yet).
pub const WEARLIFESPAN_NO_BREAKDOWNS: f32 = 1000.0;
