//! Resolving a preset file into the [`WeatherConditions`] of the session, and the values derived
//! from them.

use bevy::prelude::Vec2;
use opts_core::formats::weather::{CloudCover, CloudTypes, PrecipKind, WeatherFile};

use super::conditions::{absolute_humidity, CloudSpec, WeatherConditions, WeatherVars};
use super::kind::WeatherKind;

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
