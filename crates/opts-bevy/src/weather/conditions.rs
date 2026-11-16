//! The resolved conditions of the session: cloud layer, wind, temperature, precipitation and the
//! humidity they imply.

use super::{CloudCover, CloudTypes, PrecipKind, WeatherFile};

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

/// Absolute humidity (g/m³) of air at `temp_c` whose dew point is `dew_c`: the vapour pressure at
/// the dew point (Magnus) over the gas law at the air temperature.
#[must_use]
pub fn absolute_humidity(temp_c: f32, dew_c: f32) -> f32 {
    let dew = dew_c.min(temp_c);
    let e_hpa = 6.112 * (17.62 * dew / (243.12 + dew)).exp();
    216.7 * e_hpa / (273.15 + temp_c)
}
