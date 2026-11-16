//! What the sky does to the light: its one input ([`WeatherSky`]) and the attenuation of each light.

use super::*;

/// The sky as far as lighting is concerned: the one input of [`light_attenuation`].
///
/// The weather side fills it ([`WeatherSky::from_conditions`]); the lighting code only reads
/// [`LightAttenuation`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherSky {
    /// Share of the sky that blocks the direct light, `0..=1` (clouds, precipitation, fog).
    pub cloud_cover: f32,
    /// Meteorological visibility in metres.
    pub visibility_m: f32,
    /// What falls.
    pub precip: PrecipKind,
    /// Ground wetness `0..=1` (reserved: no rule reads it yet).
    pub wet: f32,
}

impl Default for WeatherSky {
    fn default() -> Self {
        Self {
            cloud_cover: 0.0,
            visibility_m: crate::weather::FOG_MAX_VISIBILITY_M,
            precip: PrecipKind::None,
            wet: 0.0,
        }
    }
}

impl WeatherSky {
    /// The sky of resolved weather conditions.
    #[must_use]
    pub fn from_conditions(c: &WeatherConditions) -> Self {
        Self {
            cloud_cover: effective_cover(c),
            visibility_m: c.fog_visibility_m,
            precip: c.precip,
            wet: c.ground_wet.into_iter().fold(0.0, f32::max).clamp(0.0, 1.0),
        }
    }
}

/// What the sky does to each light, `0..=1` (1 = clear).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightAttenuation {
    /// The direct sunlight.
    pub direct: f32,
    /// The ambient light.
    pub ambient: f32,
    /// The moonlight (cloud blocks the weaker light more).
    pub moon: f32,
    /// Light shafts: they need gaps in the cloud, so a solid overcast has none.
    pub shafts: f32,
}

/// **The one attenuation function** (design `lighting-system.md` 5.6). `direct` and `ambient`
/// come from [`light_scales_for_cover`] (the rule of [`crate::weather::light_scales`]: 0.55 and 0.8 at full cover,
/// the values that were in use before); `moon = 1 − 0.65·cover`; `shafts = direct² × (1 −
/// cover)`, which is 0 under a solid cover.
#[must_use]
pub fn light_attenuation(w: &WeatherSky) -> LightAttenuation {
    let cover = w.cloud_cover.clamp(0.0, 1.0);
    let scales = light_scales_for_cover(cover);
    LightAttenuation {
        direct: scales.sun,
        ambient: scales.ambient,
        moon: 1.0 - 0.65 * cover,
        shafts: scales.sun * scales.sun * (1.0 - cover),
    }
}
