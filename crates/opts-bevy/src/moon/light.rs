//! Moonlight: the phase function, the moon's height, the night, and the illuminance they give.

use super::*;

/// Allen's lunar phase function: the moon's brightness relative to the full moon at a phase
/// angle (1 at 0°, 0.23 at 60°, 0.09 at 90°, 0.026 at 120°).
#[must_use]
pub fn allen_phase_function(phase_angle_deg: f32) -> f32 {
    let a = phase_angle_deg.clamp(0.0, 180.0);
    10f32.powf(-0.4 * (0.026 * a + 4e-9 * a.powi(4)))
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Strength of the moon from its own height: nothing below the horizon, fading in to
/// [`MOON_FADE_ABOVE_DEG`], dimmer when low (extinction).
#[must_use]
pub fn moon_height_factor(moon_elevation_deg: f32) -> f32 {
    smoothstep(MOON_FADE_BELOW_DEG, MOON_FADE_ABOVE_DEG, moon_elevation_deg)
        * (0.4 + 0.6 * moon_elevation_deg.to_radians().sin()).max(0.0)
}

/// How much of the night is left: 1 with the sun below [`MOON_SUN_FULL_DEG`], 0 above
/// [`MOON_SUN_GONE_DEG`] (the moon adds no light in daylight).
#[must_use]
pub fn moon_night_factor(sun_elevation_deg: f32) -> f32 {
    1.0 - smoothstep(MOON_SUN_FULL_DEG, MOON_SUN_GONE_DEG, sun_elevation_deg)
}

/// Illuminance of the moonlight in lx:
/// `900 × Allen(α)^γ × h(el) × w(sun) × weather`, and exactly 0 at the new moon.
///
/// `weather_moon` is [`crate::environment::LightAttenuation::moon`].
#[must_use]
pub fn moon_illuminance(moon: &MoonState, sun_elevation_deg: f32, weather_moon: f32) -> f32 {
    let phase = allen_phase_function(moon.phase_angle_deg).powf(MOON_PHASE_GAMMA);
    let new_gate = smoothstep(0.0, MOON_NEW_FRACTION, moon.illuminated);
    MOON_PEAK_LX
        * phase
        * new_gate
        * moon_height_factor(moon.elevation_deg)
        * moon_night_factor(sun_elevation_deg)
        * weather_moon.clamp(0.0, 1.0)
}
