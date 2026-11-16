//! The look of the weather effects (fog, clouds and precipitation brightness) and the cloud drift.

use super::*;

/// The colours and brightnesses of the weather effects, **scene-linear** (what the camera
/// renders before tonemapping). The one place that turns the sky's sRGB horizon colour and the
/// time of day into them, so a switch of the camera to HDR (an exposure factor) changes this
/// function only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherLook {
    /// Fog colour, linear RGB.
    pub fog: [f32; 3],
    /// Brightness of the cloud layer, linear `0..=1`.
    pub cloud: f32,
    /// Brightness of rain and snow, linear `0.25..=1`.
    pub precip: f32,
}

/// The [`WeatherLook`] for the conditions, the sky's horizon colour (sRGB `0..=1`, as the sky
/// texture holds it) and the sun's elevation.
#[must_use]
pub fn weather_look(
    c: &WeatherConditions,
    horizon_srgb: [f32; 3],
    sun_alt_deg: f32,
) -> WeatherLook {
    let fog_srgb = fog_color(horizon_srgb, c);
    let lin = Color::srgb(fog_srgb[0], fog_srgb[1], fog_srgb[2]).to_linear();
    let cloud = cloud_tint(c, sun_alt_deg);
    WeatherLook {
        fog: [lin.red, lin.green, lin.blue],
        cloud,
        precip: 0.25 + 0.75 * cloud,
    }
}

/// Cloud drift in metres per second on the ground plane (Bevy axes): the wind carries the clouds
/// three times faster than it blows at the ground.
#[must_use]
pub fn cloud_drift(c: &WeatherConditions) -> Vec2 {
    c.wind_to() * c.wind_speed * 3.0
}
