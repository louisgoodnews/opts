//! The session clock as variables of the `[volcurve]`s of scenery sounds.

use opts_bevy::scenery_runtime::HostValues;
use opts_core::formats::sound::SoundDef;

use crate::sound_logic::level;

/// The value of a variable a `[volcurve]` reads: the session clock's, by name. Unknown names read
/// 0 (the format page: "treat input as 0").
#[must_use]
pub fn host_variable(name: &str, host: &HostValues) -> Option<f32> {
    Some(match name.trim().to_lowercase().as_str() {
        "time" => host.time_s,
        "day" => host.day,
        "month" => host.month,
        "year" => host.year,
        "dayofyear" => host.day_of_year,
        "sunalt" => host.sun_alt_deg,
        "temperature" | "weather_temperature" => host.temperature_c,
        "preciprate" => host.precip_rate,
        "preciptype" => host.precip_type,
        _ => return None,
    })
}

/// The level of a definition: the product of its volume curves over the clock.
#[must_use]
pub fn level_of(def: &SoundDef, host: &HostValues) -> f32 {
    level(def, |n| host_variable(n, host).unwrap_or(0.0))
}
