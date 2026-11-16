//! Road grip in weather (M-0117): the friction coefficient of the tyres on the road surface from
//! its wetness, the snow lying on it and ice. Pure numbers with no engine dependency: the host
//! maps the weather state (the `Wetness` resource, snow cover, ice) into the inputs and writes the
//! result into [`VehicleSimulation::road_friction`](crate::vehicle::VehicleSimulation).
//!
//! The coefficients are handbook values for tyres on asphalt, wet asphalt, packed snow and ice,
//! each with an uncertainty of about ±0.1 (❓ in `docs/src/architecture/weather.md`, section 10).

use crate::vehicle::ROAD_FRICTION;

/// Dry asphalt: the vehicle's own [`ROAD_FRICTION`].
pub const MU_DRY: f32 = ROAD_FRICTION;
/// Wet asphalt ❓.
pub const MU_WET: f32 = 0.55;
/// Packed snow ❓.
pub const MU_SNOW: f32 = 0.25;
/// Ice ❓.
pub const MU_ICE: f32 = 0.10;
/// Largest change of the friction coefficient per second (1/s). A step from dry to packed snow
/// (0.6) takes 1.2 s, so the grip follows a weather change within the 2 s of M-0117.
pub const MAX_RATE_PER_S: f32 = 0.5;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Friction coefficient of a road surface: `wetness` (0..=1, the `Wetness` resource), `snow`
/// (cover 0..=1) and `ice` (0..=1). Each input blends the coefficient towards its own value, the
/// snow over the wet surface and the ice over the snow; out-of-range inputs are clamped.
pub fn coefficient(wetness: f32, snow: f32, ice: f32) -> f32 {
    let wet = lerp(MU_DRY, MU_WET, wetness.clamp(0.0, 1.0));
    let snowy = lerp(wet, MU_SNOW, snow.clamp(0.0, 1.0));
    lerp(snowy, MU_ICE, ice.clamp(0.0, 1.0))
}

/// Moves `current` towards `target` by at most [`MAX_RATE_PER_S`] times `dt` (s), so that the
/// grip never jumps within a step.
pub fn approach(current: f32, target: f32, dt: f32) -> f32 {
    let max_step = MAX_RATE_PER_S * dt.max(0.0);
    current + (target - current).clamp(-max_step, max_step)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_surface_keeps_the_vehicle_friction() {
        assert_eq!(coefficient(0.0, 0.0, 0.0), ROAD_FRICTION);
    }

    #[test]
    fn full_wetness_snow_and_ice_give_their_coefficients() {
        assert!((coefficient(1.0, 0.0, 0.0) - MU_WET).abs() < 1e-6);
        assert!((coefficient(1.0, 1.0, 0.0) - MU_SNOW).abs() < 1e-6);
        assert!((coefficient(1.0, 1.0, 1.0) - MU_ICE).abs() < 1e-6);
    }

    #[test]
    fn inputs_are_clamped_and_blend_monotonically() {
        assert_eq!(coefficient(5.0, 0.0, 0.0), coefficient(1.0, 0.0, 0.0));
        assert_eq!(coefficient(-1.0, 0.0, 0.0), MU_DRY);
        let half = coefficient(0.5, 0.0, 0.0);
        assert!(half < MU_DRY && half > MU_WET);
        assert!(coefficient(1.0, 0.5, 0.0) < MU_WET);
    }

    #[test]
    fn approach_is_rate_limited_and_reaches_the_target() {
        let dt = 1.0 / 64.0;
        let next = approach(MU_DRY, MU_SNOW, dt);
        assert!((MU_DRY - next) <= MAX_RATE_PER_S * dt + 1e-6);
        let mut mu = MU_DRY;
        for _ in 0..(2.0 / dt) as usize {
            mu = approach(mu, MU_SNOW, dt);
        }
        assert!((mu - MU_SNOW).abs() < 1e-6, "{mu} after 2 s");
        assert_eq!(approach(MU_SNOW, MU_SNOW, dt), MU_SNOW);
    }
}
