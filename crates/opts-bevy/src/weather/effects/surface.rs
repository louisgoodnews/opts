//! Wet and snowy surfaces: ground wetness, puddles, snow cover, and the road material under them.

use super::*;

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
