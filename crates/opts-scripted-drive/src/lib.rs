//! The near-vehicle drive (M-0103): the planner's desired acceleration as the pedals a vehicle's
//! script reads, and the script's wheel torque and brake force back as an acceleration.
//!
//! Pure arithmetic, no traffic or script types: the game connects these to the traffic world's
//! external acceleration and to the script engine's `Throttle`, `Brake`, `M_Wheel` and
//! `Brakeforce` variables.

/// Throttle and brake (each `0..=1`) for a desired acceleration. Accelerating uses the throttle
/// up to `max_accel`; braking uses the brake up to `max_decel` (both m/s²).
#[must_use]
pub fn pedals(desired_accel: f64, max_accel: f64, max_decel: f64) -> (f64, f64) {
    if desired_accel >= 0.0 {
        ((desired_accel / max_accel.max(1e-6)).clamp(0.0, 1.0), 0.0)
    } else {
        (0.0, (-desired_accel / max_decel.max(1e-6)).clamp(0.0, 1.0))
    }
}

/// Acceleration (m/s²) of a vehicle of `mass_kg` on wheels of `radius_m` from the script's
/// `M_Wheel` (N·m) and `Brakeforce` (N): the drive force opposes the brake force.
#[must_use]
pub fn script_accel(m_wheel: f64, brake_force: f64, mass_kg: f64, radius_m: f64) -> f64 {
    (m_wheel / radius_m.max(0.01) - brake_force) / mass_kg.max(1.0)
}
