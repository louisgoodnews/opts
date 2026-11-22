//! Pure weather effect derivations: settings, look, cloud drift, precipitation plan, and surface wetness/snow.

mod look;
mod precip;
mod quality;
mod surface;

pub use look::{cloud_drift, weather_look, WeatherLook};
pub use precip::{precip_plan, PrecipPlan};
pub use quality::PrecipitationQuality;
pub use surface::{road_surface, surface_state, surface_state_for, SurfaceState};

use super::*;

/// Which weather effects are drawn (the settings `graphics.weather_*`; the game writes it).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeatherEffects {
    /// Distance fog.
    pub fog: bool,
    /// Cloud layer.
    pub clouds: bool,
    /// Rain and snow.
    pub precipitation: PrecipitationQuality,
    /// Wet and snowy ground and roads.
    pub wet_ground: bool,
    /// Wet objects and vehicles: materials whose texture has `[moisture]` take the wetness (the
    /// setting *Wet and snowy surfaces*, All; `docs/src/roadmap/phase-12/M-0121.md`).
    pub wet_objects: bool,
    /// How strong rain is drawn, in percent of what the weather says (100: as the preset; the
    /// settings `graphics.rain_intensity`): the rate of the drops, the wetness, the puddles.
    pub rain_percent: u16,
    /// The same for snow (`graphics.snow_intensity`): the rate of the flakes, and the cover of
    /// the ground and roads while the wet and snowy ground setting is on (`wanted_state`).
    pub snow_percent: u16,
}

impl Default for WeatherEffects {
    fn default() -> Self {
        Self {
            fog: true,
            clouds: true,
            precipitation: PrecipitationQuality::default(),
            wet_ground: true,
            wet_objects: false,
            rain_percent: 100,
            snow_percent: 100,
        }
    }
}

impl WeatherEffects {
    /// Factor of the rain intensity (1 at 100 %).
    #[must_use]
    pub fn rain_factor(&self) -> f32 {
        f32::from(self.rain_percent) / 100.0
    }

    /// Factor of the snow intensity (1 at 100 %).
    #[must_use]
    pub fn snow_factor(&self) -> f32 {
        f32::from(self.snow_percent) / 100.0
    }

    /// `c` with the rate of what falls scaled by its intensity setting (the drops and flakes that
    /// are drawn; the surfaces use [`surface_state_for`]).
    #[must_use]
    pub fn scaled(&self, c: &WeatherConditions) -> WeatherConditions {
        let mut c = c.clone();
        let k = match c.precip {
            PrecipKind::Rain => self.rain_factor(),
            PrecipKind::Snow => self.snow_factor(),
            _ => 1.0,
        };
        c.precip_rate = (c.precip_rate * k).clamp(0.0, 1.5);
        c
    }
}
