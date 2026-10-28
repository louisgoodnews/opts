//! Pure weather effect derivations: settings, look, cloud drift, precipitation plan, and surface wetness/snow.

use super::*;

/// How many rain/snow quads the precipitation may use at most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrecipitationQuality {
    /// No precipitation is drawn.
    Off,
    /// 1500 quads.
    Low,
    /// 3000 quads.
    #[default]
    Medium,
    /// 6000 quads.
    High,
}

impl PrecipitationQuality {
    /// The stored value.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// The quality of a stored value.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "off" => Some(Self::Off),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }

    /// The quad budget.
    #[must_use]
    pub fn budget(self) -> u32 {
        match self {
            Self::Off => 0,
            Self::Low => 1500,
            Self::Medium => 3000,
            Self::High => 6000,
        }
    }
}

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
    /// How strong rain is drawn, in percent of what the weather says (100: as the preset; the
    /// settings `gameplay.rain_intensity`): the rate of the drops, the wetness, the puddles.
    pub rain_percent: u16,
    /// The same for snow (`gameplay.snow_intensity`): the rate of the flakes and the cover.
    pub snow_percent: u16,
}

impl Default for WeatherEffects {
    fn default() -> Self {
        Self {
            fog: true,
            clouds: true,
            precipitation: PrecipitationQuality::default(),
            wet_ground: true,
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

/// Rain or snow around the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrecipPlan {
    /// Rain or snow.
    pub kind: PrecipKind,
    /// Quads in use, at most the quality's budget.
    pub quads: u32,
    /// Fall speed in m/s.
    pub fall_speed: f32,
    /// Horizontal velocity in m/s (the wind, damped).
    pub drift: Vec2,
    /// Width and length of a quad in metres.
    pub size: Vec2,
}

/// The precipitation to draw: none for dry weather, a budget of `0` or a tiny rate.
#[must_use]
pub fn precip_plan(c: &WeatherConditions, budget: u32) -> Option<PrecipPlan> {
    if c.precip == PrecipKind::None || c.precip_rate <= 0.0 || budget == 0 {
        return None;
    }
    let share = 0.15 + 0.85 * c.precip_rate;
    let quads = ((budget as f32 * share).round() as u32).clamp(1, budget);
    let (fall, damp, size) = match c.precip {
        PrecipKind::Rain => (9.0, 0.6, Vec2::new(0.025, 0.8)),
        _ => (1.4, 0.9, Vec2::new(0.08, 0.08)),
    };
    let mut drift = c.wind_to() * c.wind_speed * damp;
    let cap = fall * 1.5;
    if drift.length() > cap {
        drift = drift.normalize() * cap;
    }
    Some(PrecipPlan {
        kind: c.precip,
        quads,
        fall_speed: fall,
        drift,
        size,
    })
}

/// Wet and snowy surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SurfaceState {
    /// Wetness `0..=1` (darker, smoother).
    pub wet: f32,
    /// Puddle coverage `0..=1` on the ground.
    pub puddles: f32,
    /// Snow cover of the ground `0..=1`.
    pub snow_ground: f32,
    /// Snow cover of the roads `0..=1`.
    pub snow_road: f32,
}

/// Ground wetness and snow from `[groundwet]` and the snow flags. The meaning of the three values
/// is a guess (second / 255 is the wetness, third / 120 the puddles ❓); snow covers part of the
/// wetness.
#[must_use]
pub fn surface_state(c: &WeatherConditions) -> SurfaceState {
    surface_state_for(c, 1.0, 1.0)
}

/// [`surface_state`] with the intensity settings: `rain` scales the wetness and the puddles,
/// `snow` the cover of the ground and the roads (each at most full).
#[must_use]
pub fn surface_state_for(c: &WeatherConditions, rain: f32, snow: f32) -> SurfaceState {
    let snow_ground = if c.snow {
        (0.85 * snow).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let snow_road = if c.snow_on_road {
        (0.7 * snow).clamp(0.0, 1.0)
    } else {
        0.0
    };
    SurfaceState {
        wet: (c.ground_wet[1] / 255.0 * rain).clamp(0.0, 1.0) * (1.0 - 0.6 * snow_ground),
        puddles: 0.45 * (c.ground_wet[2] / 120.0 * rain).clamp(0.0, 1.0) * (1.0 - snow_ground),
        snow_ground,
        snow_road,
    }
}

/// Colour and roughness of a road material under the surface state: darker and smoother when
/// wet, tinted towards snow with snow on the road. `base` is linear RGB.
#[must_use]
pub fn road_surface(base: [f32; 3], roughness: f32, s: &SurfaceState) -> ([f32; 3], f32) {
    let dark = 1.0 - 0.3 * s.wet;
    let snow = [0.92, 0.94, 0.97];
    let c = std::array::from_fn(|i| {
        let wet = base[i] * dark;
        wet + (snow[i] - wet) * s.snow_road
    });
    (c, (roughness * (1.0 - 0.55 * s.wet)).clamp(0.04, 1.0))
}
