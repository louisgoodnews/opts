//! Weather for AI drivers (M-0117): the road grip and the visibility change the driver's IDM
//! parameters. Both are plain numbers: the host turns the grip of `opts-sim`'s `surface_grip` (a
//! friction coefficient, relative to dry asphalt) and the fog visibility into the inputs here.
//!
//! The rules are design choices, not calibrated values (❓ `docs/src/architecture/weather.md`,
//! section 10): the grip scales every acceleration and speed limit of the driver, and the
//! headway grows with `1 / √grip` and with the fog distance beyond [`VISIBILITY_REF_M`].

use crate::idm::IdmParams;

/// Lowest grip factor used in the rules; below it the road counts as this grip (no division by
/// zero, and a stopped car on ice still gets a finite law).
pub const MIN_GRIP: f64 = 0.05;
/// Visibility (m) at and above which fog does not change the headway.
pub const VISIBILITY_REF_M: f64 = 300.0;
/// Largest headway factor the fog may cause.
pub const MAX_FOG_HEADWAY_FACTOR: f64 = 3.0;
/// Visibility (m) below which the player's HUD shows the low-visibility hint.
pub const LOW_VISIBILITY_M: f64 = 100.0;

/// The driver's parameters on a road with grip factor `grip` (`1.0`: dry asphalt). Speed,
/// accelerations, decelerations and the curve limit scale with the grip (the top speed and the
/// lane-limit factor with its square root, since the speed the driver holds falls less than the
/// braking), and the time headway grows with `1 / √grip`.
#[must_use]
pub fn grip_adjusted(params: &IdmParams, grip: f64) -> IdmParams {
    let g = grip.clamp(MIN_GRIP, 1.0);
    IdmParams {
        desired_speed: params.desired_speed * g.sqrt(),
        speed_factor: params.speed_factor * g.sqrt(),
        max_accel: params.max_accel * g,
        comfort_decel: params.comfort_decel * g,
        max_decel: params.max_decel * g,
        lat_accel: params.lat_accel * g,
        time_headway: params.time_headway / g.sqrt(),
        ..*params
    }
}

/// Headway factor (≥ 1) for a fog visibility `visibility_m`: 1 at and above
/// [`VISIBILITY_REF_M`], growing as the visibility falls, capped at [`MAX_FOG_HEADWAY_FACTOR`].
#[must_use]
pub fn fog_headway_factor(visibility_m: f64) -> f64 {
    if visibility_m.is_nan() || visibility_m <= 0.0 {
        return MAX_FOG_HEADWAY_FACTOR;
    }
    (VISIBILITY_REF_M / visibility_m).clamp(1.0, MAX_FOG_HEADWAY_FACTOR)
}

/// The driver's parameters with the weather applied: the grip of the road and the fog's
/// visibility (m, `f64::INFINITY` in clear weather).
#[must_use]
pub fn weather_adjusted(params: &IdmParams, grip: f64, visibility_m: f64) -> IdmParams {
    let mut p = grip_adjusted(params, grip);
    p.time_headway *= fog_headway_factor(visibility_m);
    p
}

/// Whether the visibility is low enough for the player's HUD hint.
#[must_use]
pub fn is_low_visibility(visibility_m: f64) -> bool {
    visibility_m < LOW_VISIBILITY_M
}

/// The weather every driver of a world sees: the grip factor of the road (`1.0`: dry asphalt)
/// and the fog visibility (m, `f64::INFINITY` in clear air).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherState {
    /// Grip factor of the road (`1.0`: dry asphalt).
    pub grip: f64,
    /// Visibility in metres (`f64::INFINITY` in clear air).
    pub visibility_m: f64,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self {
            grip: 1.0,
            visibility_m: f64::INFINITY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_road_and_clear_air_leave_the_parameters_alone() {
        let base = IdmParams::default();
        assert_eq!(weather_adjusted(&base, 1.0, f64::INFINITY), base);
    }

    #[test]
    fn snow_lowers_the_desired_speed_by_more_than_ten_percent() {
        let base = IdmParams::default();
        let snow = grip_adjusted(&base, 0.25 / 0.85);
        assert!(
            snow.desired_speed <= 0.9 * base.desired_speed,
            "{}",
            snow.desired_speed
        );
        assert!(snow.max_decel < base.max_decel);
    }

    #[test]
    fn low_grip_lengthens_the_headway() {
        let base = IdmParams::default();
        let snow = grip_adjusted(&base, 0.3);
        assert!(snow.time_headway > base.time_headway);
        assert!((snow.time_headway - base.time_headway / 0.3_f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn grip_is_clamped_to_the_minimum_and_the_dry_road() {
        let base = IdmParams::default();
        assert_eq!(grip_adjusted(&base, 2.0), grip_adjusted(&base, 1.0));
        let floor = grip_adjusted(&base, 0.0);
        assert!(floor.max_decel > 0.0 && floor.desired_speed > 0.0);
        assert_eq!(floor, grip_adjusted(&base, MIN_GRIP));
    }

    #[test]
    fn fog_headway_rises_with_falling_visibility_and_is_capped() {
        assert_eq!(fog_headway_factor(1000.0), 1.0);
        assert_eq!(fog_headway_factor(VISIBILITY_REF_M), 1.0);
        let at_150 = fog_headway_factor(150.0);
        let at_100 = fog_headway_factor(100.0);
        assert!(at_150 > 1.0 && at_100 > at_150);
        assert_eq!(fog_headway_factor(1.0), MAX_FOG_HEADWAY_FACTOR);
        assert_eq!(fog_headway_factor(0.0), MAX_FOG_HEADWAY_FACTOR);
        assert_eq!(fog_headway_factor(f64::INFINITY), 1.0);
    }

    #[test]
    fn fog_scales_the_following_distance_of_the_driver() {
        let base = IdmParams::default();
        let fog = weather_adjusted(&base, 1.0, 100.0);
        assert!(fog.desired_gap(10.0, 0.0) > base.desired_gap(10.0, 0.0));
    }

    #[test]
    fn the_hint_threshold_is_one_hundred_metres() {
        assert!(is_low_visibility(99.0));
        assert!(!is_low_visibility(100.0));
    }
}
