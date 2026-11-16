//! The regeneration policy of the sky cube and the exposure that scales its intensity.

use super::input::RegenSnapshot;
use super::tuning::{NIGHT_EXPOSURE, REGEN_MIN_INTERVAL_S, REGEN_SUN_DELTA_DEG, WEATHER_EPS};

/// Whether the cube must be rebuilt: `last` is the snapshot and real time (seconds) of the last
/// build, `None` before the first.
#[must_use]
pub fn should_regenerate(
    last: Option<(&RegenSnapshot, f64)>,
    now: &RegenSnapshot,
    now_s: f64,
) -> bool {
    let Some((prev, at)) = last else { return true };
    if now_s - at < REGEN_MIN_INTERVAL_S {
        return false;
    }
    prev.revision != now.revision
        || (prev.sun_elevation_deg - now.sun_elevation_deg).abs() > REGEN_SUN_DELTA_DEG
        || prev
            .weather
            .iter()
            .zip(now.weather)
            .any(|(a, b)| (a - b).abs() > WEATHER_EPS)
}

/// How much of [`BASE_INTENSITY`](super::tuning::BASE_INTENSITY) is used: zero in the dark (a trace of moonlight), full in the
/// day (smoothstep over sun elevation -4 to 10 degrees), scaled by the weather's ambient
/// attenuation.
#[must_use]
pub fn exposure(sun_elevation_deg: f32, ambient_attenuation: f32) -> f32 {
    let t = ((sun_elevation_deg + 4.0) / 14.0).clamp(0.0, 1.0);
    let day = t * t * (3.0 - 2.0 * t);
    (NIGHT_EXPOSURE + (1.0 - NIGHT_EXPOSURE) * day) * ambient_attenuation.clamp(0.0, 1.0)
}
