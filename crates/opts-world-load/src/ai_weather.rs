//! The weather inputs of the AI drivers and the road grip they follow (M-0117): the surface state
//! of the session's weather, the tyre friction a road gets from it, and the `AiWeather` system
//! parameter that hands both to the traffic. The player's grip system and the surface materials
//! stay in `opts-game` (`in_game::weather`), which re-exports these.

use bevy::prelude::*;
use opts_bevy::reflections::Wetness;
use opts_bevy::weather::{surface_state_for, SurfaceState};
use opts_bevy::{SessionEnvironment, WeatherEffects};
use opts_sim::surface_grip;
use opts_sim::vehicle::ROAD_FRICTION;

/// The surface state to show: dry when the effect is off.
#[must_use]
pub fn wanted_state(env: &SessionEnvironment, effects: &WeatherEffects) -> SurfaceState {
    if effects.wet_ground {
        surface_state_for(
            env.conditions(),
            effects.rain_factor(),
            effects.snow_factor(),
        )
    } else {
        SurfaceState::default()
    }
}

/// Friction coefficient the tyres should have on a road with wetness `wet` (0..=1) and snow cover
/// `snow_road` (0..=1); no ice is reported by the weather state.
#[must_use]
pub fn grip_target(wet: f32, snow_road: f32) -> f32 {
    surface_grip::coefficient(wet, snow_road, 0.0)
}

/// The friction `current` moves to over `dt` seconds towards the target of the weather: at most
/// [`surface_grip::MAX_RATE_PER_S`] per second, so the grip never jumps within a step.
#[must_use]
pub fn next_road_friction(current: f32, wet: f32, snow_road: f32, dt: f32) -> f32 {
    surface_grip::approach(current, grip_target(wet, snow_road), dt)
}

/// The weather inputs of the AI drivers: the session's conditions and the surface state.
#[derive(bevy::ecs::system::SystemParam)]
pub struct AiWeather<'w> {
    pub env: Option<Res<'w, SessionEnvironment>>,
    effects: Option<Res<'w, WeatherEffects>>,
    wetness: Option<Res<'w, Wetness>>,
}

impl AiWeather<'_> {
    /// The session's time of day is given by the environment; this hands the AI drivers the grip
    /// factor of the road (friction over dry asphalt) and the fog visibility, so their desired
    /// speed and headway follow the weather (M-0117).
    pub fn sync_traffic(&self, world: &mut opts_traffic::TrafficWorld) {
        let (Some(env), Some(effects)) = (&self.env, &self.effects) else {
            return;
        };
        let wet = self.wetness.as_ref().map_or(0.0, |w| w.0);
        let snow_road = wanted_state(env, effects).snow_road;
        let grip = grip_target(wet, snow_road) / ROAD_FRICTION;
        world.set_weather(
            f64::from(grip),
            f64::from(env.conditions().fog_visibility_m),
        );
    }
}

