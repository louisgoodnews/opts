//! Weather of the session: the preset catalogue, the resolved [`WeatherConditions`] and the pure
//! functions that turn conditions into effect parameters (light scales, fog, sky veil, clouds,
//! precipitation, wet and snowy surfaces).
//!
//! The presets come from the install's `Weather` folder ([`opts_core::formats::weather`]); five
//! built-in presets (the ids of earlier versions) cover a missing install and old saves. The
//! rendering systems live in the game (`opts_game::in_game::weather`); everything here is
//! deterministic and tested without a window. Design: `docs/src/architecture/weather.md`.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::OnceLock;

use bevy::prelude::*;
use opts_core::formats::weather::{
    omsi_language, CloudCover, CloudLayer, CloudTypes, Fog, PrecipKind, PresetFile, WeatherFile,
    WeatherFolder,
};

mod catalogue;
mod effects;
mod fog;
pub use catalogue::{alias_of, builtin_preset, legacy_id, WeatherCatalogue, BUILTIN_IDS};
pub use effects::*;
pub use fog::{
    cloud_tint, effective_cover, fog_color, fog_density, fog_density_for, fog_layer_thickness,
    fogginess, ground_fog, ground_fog_view, light_scales, light_scales_for_cover, sky_veil,
    sky_veil_clear, FogParams, GroundFogView, LightScales, FOG_LAYER_ABOVE, FOG_LAYER_INSIDE,
    FOG_LAYER_LOOK_UP_DEG, FOG_LAYER_LOOK_UP_GAIN, FOG_LAYER_MAX_VISIBILITY_M,
    FOG_LAYER_THICKNESS_PER_VISIBILITY, FOG_LAYER_THICKNESS_RANGE_M, FOG_MAX_SATURATION,
    FOG_MAX_VISIBILITY_M,
};

/// What a preset does to the sky, for classification (fallback light, icons).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeatherKind {
    /// Clear sky.
    Clear,
    /// Overcast.
    Overcast,
    /// Rain.
    Rain,
    /// Snow.
    Snow,
    /// Fog.
    Fog,
}

impl WeatherKind {
    /// The kind of a preset id, by the words it contains (`rain`, `snow`, `fog`, `overcast`);
    /// anything else is clear.
    #[must_use]
    pub fn of_preset(id: &str) -> Self {
        let lower = id.to_lowercase();
        if lower.contains("snow") {
            Self::Snow
        } else if lower.contains("rain") {
            Self::Rain
        } else if lower.contains("fog") {
            Self::Fog
        } else if lower.contains("overcast") {
            Self::Overcast
        } else {
            Self::Clear
        }
    }

    /// Whether the sky is covered (the fallback light turns diffuse).
    #[must_use]
    pub fn overcast(self) -> bool {
        self != Self::Clear
    }
}

/// A cloud layer of the resolved conditions.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudSpec {
    /// Type name (`Cumulus 1`).
    pub name: String,
    /// Texture file below `Texture/`; `None` when `clouds.cfg` does not know the type.
    pub texture: Option<String>,
    /// Edge length of one texture repeat in metres.
    pub size_m: f32,
    /// Base height in metres above the camera's ground reference.
    pub base_m: f32,
    /// How the layer is drawn.
    pub cover: CloudCover,
    /// Sky coverage `0..=1` (overcast 1; scattered 0.25 / 0.4 / 0.55 for `Cumulus 1/2/3` ❓).
    pub coverage: f32,
}

/// The values scripts can read, besides the temperature (`Weather_Temperature`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WeatherVars {
    /// `Weather_AbsHum` in g/m³.
    pub abs_hum: f32,
    /// `PrecipType`: 0 none, 1 rain, 2 snow.
    pub precip_type: f32,
    /// `PrecipRate`: 0 … 1.
    pub precip_rate: f32,
    /// Wind direction (from), degrees; no script variable is documented for it.
    pub wind_from_deg: f32,
    /// Wind speed in m/s; no script variable is documented for it.
    pub wind_speed: f32,
    /// Pressure in hPa; no script variable is documented for it.
    pub pressure_hpa: f32,
    /// Fog visibility in metres; no script variable is documented for it.
    pub fog_visibility_m: f32,
}

/// The weather of the session, resolved from a preset.
#[derive(Debug, Clone, PartialEq)]
pub struct WeatherConditions {
    /// Fog visibility in metres.
    pub fog_visibility_m: f32,
    /// Fog thickness factor `0..=1`.
    pub fog_density: f32,
    /// Direction the wind comes from, degrees clockwise from north.
    pub wind_from_deg: f32,
    /// Wind speed in m/s.
    pub wind_speed: f32,
    /// Air temperature in °C (`Weather_Temperature`).
    pub temperature_c: f32,
    /// Dew point in °C.
    pub dew_point_c: f32,
    /// Pressure in hPa.
    pub pressure_hpa: f32,
    /// The cloud layer, if any.
    pub cloud: Option<CloudSpec>,
    /// What falls.
    pub precip: PrecipKind,
    /// Precipitation rate `0..=1` (the intensity of the file over 255); 0 when dry.
    pub precip_rate: f32,
    /// `[groundwet]` as in the file.
    pub ground_wet: [f32; 3],
    /// Snow lies on the ground.
    pub snow: bool,
    /// Snow lies on the roads.
    pub snow_on_road: bool,
}

impl Default for WeatherConditions {
    fn default() -> Self {
        Self::from_file(&WeatherFile::default(), &CloudTypes::default())
    }
}

impl WeatherConditions {
    /// The conditions of a preset file, with the cloud type looked up in `clouds`.
    #[must_use]
    pub fn from_file(w: &WeatherFile, clouds: &CloudTypes) -> Self {
        let cloud = w.clouds.as_ref().map(|layer| {
            let known = clouds.find(&layer.kind);
            let cover = known.map_or_else(
                || {
                    if layer.kind.to_ascii_lowercase().contains("ovc")
                        || layer.kind.to_ascii_lowercase().contains("overcast")
                    {
                        CloudCover::Overcast
                    } else {
                        CloudCover::Scattered
                    }
                },
                |t| t.cover,
            );
            let digit = layer
                .kind
                .rsplit(' ')
                .next()
                .and_then(|d| d.parse::<f32>().ok())
                .unwrap_or(2.0);
            CloudSpec {
                name: layer.kind.clone(),
                texture: known.map(|t| t.texture.clone()),
                size_m: known.map_or(2000.0, |t| t.size_m),
                base_m: layer.base_m.max(0.0),
                cover,
                coverage: match cover {
                    CloudCover::Overcast => 1.0,
                    CloudCover::Scattered => (0.1 + 0.15 * digit).clamp(0.25, 0.6),
                },
            }
        });
        let precip = w.precip.kind;
        Self {
            fog_visibility_m: w.fog.visibility_m.max(1.0),
            fog_density: w.fog.density.clamp(0.0, 1.0),
            wind_from_deg: w.wind.direction_deg.rem_euclid(360.0),
            wind_speed: w.wind.speed.max(0.0),
            temperature_c: w.temp.celsius,
            dew_point_c: w.temp.dew_point_c.min(w.temp.celsius),
            pressure_hpa: w.pressure_hpa,
            cloud,
            precip,
            precip_rate: if precip == PrecipKind::None {
                0.0
            } else {
                (w.precip.intensity / 255.0).clamp(0.0, 1.0)
            },
            ground_wet: w.ground_wet,
            snow: w.snow,
            snow_on_road: w.snow_on_road,
        }
    }

    /// The classification used by the fallback light and the icons.
    #[must_use]
    pub fn kind(&self) -> WeatherKind {
        if self.precip == PrecipKind::Snow {
            WeatherKind::Snow
        } else if self.precip == PrecipKind::Rain {
            WeatherKind::Rain
        } else if self.fog_visibility_m < 1000.0 {
            WeatherKind::Fog
        } else if self.cloud.as_ref().is_some_and(|c| c.coverage >= 0.5) {
            WeatherKind::Overcast
        } else {
            WeatherKind::Clear
        }
    }

    /// Absolute humidity in g/m³ from the temperature and the dew point (Magnus formula).
    #[must_use]
    pub fn abs_humidity(&self) -> f32 {
        absolute_humidity(self.temperature_c, self.dew_point_c)
    }

    /// The direction the wind blows **towards**, as a unit vector on the ground in Bevy axes
    /// (x east, z south; north is −z).
    #[must_use]
    pub fn wind_to(&self) -> Vec2 {
        let b = self.wind_from_deg.to_radians();
        Vec2::new(-b.sin(), b.cos())
    }

    /// The values scripts and the host API read.
    #[must_use]
    pub fn vars(&self) -> WeatherVars {
        WeatherVars {
            abs_hum: self.abs_humidity(),
            precip_type: f32::from(self.precip.code()),
            precip_rate: self.precip_rate,
            wind_from_deg: self.wind_from_deg,
            wind_speed: self.wind_speed,
            pressure_hpa: self.pressure_hpa,
            fog_visibility_m: self.fog_visibility_m,
        }
    }
}

/// Absolute humidity (g/m³) of air at `temp_c` whose dew point is `dew_c`: the vapour pressure at
/// the dew point (Magnus) over the gas law at the air temperature.
#[must_use]
pub fn absolute_humidity(temp_c: f32, dew_c: f32) -> f32 {
    let dew = dew_c.min(temp_c);
    let e_hpa = 6.112 * (17.62 * dew / (243.12 + dew)).exp();
    216.7 * e_hpa / (273.15 + temp_c)
}

#[cfg(test)]
mod tests;
