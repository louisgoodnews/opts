//! Fog, sky veil and cloud tint derived from the weather.

mod ground;
mod sky;

pub use ground::{
    fog_density, fog_density_for, fog_layer_thickness, ground_fog, ground_fog_view, FogParams,
    GroundFogView, FOG_LAYER_ABOVE, FOG_LAYER_INSIDE, FOG_LAYER_LOOK_UP_DEG, FOG_LAYER_LOOK_UP_GAIN,
    FOG_LAYER_MAX_VISIBILITY_M, FOG_LAYER_THICKNESS_PER_VISIBILITY, FOG_LAYER_THICKNESS_RANGE_M,
};
pub use sky::{cloud_tint, fog_color, sky_veil, sky_veil_clear, FOG_MAX_SATURATION};

use super::*;

/// Visibility above which no distance fog is drawn (metres).
pub const FOG_MAX_VISIBILITY_M: f32 = 15_000.0;

/// How the weather scales the sun's illuminance and the ambient brightness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightScales {
    /// Multiplier of the direct sunlight.
    pub sun: f32,
    /// Multiplier of the ambient light.
    pub ambient: f32,
}

/// How foggy the air is, `0` (10 km visibility or more) to `1` (a few metres).
#[must_use]
pub fn fogginess(c: &WeatherConditions) -> f32 {
    (1.0 - c.fog_visibility_m / 10_000.0).clamp(0.0, 1.0)
}

/// The share of the sky that is covered for lighting: the larger of the clouds, the
/// precipitation (rain and snow come out of a covered sky) and the thick fog.
#[must_use]
pub fn effective_cover(c: &WeatherConditions) -> f32 {
    let clouds = c.cloud.as_ref().map_or(0.0, |cl| cl.coverage);
    let precip = if c.precip == PrecipKind::None || c.precip_rate <= 0.0 {
        0.0
    } else {
        0.5 + 0.5 * c.precip_rate
    };
    let fog = fogginess(c) * (0.5 + 0.5 * c.fog_density);
    clouds.max(precip).max(fog).clamp(0.0, 1.0)
}

/// **The one function through which weather scales the sun and the ambient light.** A covered sky
/// (cover 1) gives the sun 0.55 and the ambient light 0.8, a clear one 1 and 1.
#[must_use]
pub fn light_scales(c: &WeatherConditions) -> LightScales {
    light_scales_for_cover(effective_cover(c))
}

/// [`light_scales`] for a given share of covered sky `0..=1` (the rule itself, so the sky
/// description [`crate::environment::WeatherSky`] shares it).
#[must_use]
pub fn light_scales_for_cover(cover: f32) -> LightScales {
    let cover = cover.clamp(0.0, 1.0);
    LightScales {
        sun: 1.0 - 0.45 * cover,
        ambient: 1.0 - 0.2 * cover,
    }
}
