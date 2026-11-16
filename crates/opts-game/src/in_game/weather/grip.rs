//! The road grip of the player's vehicle follows the weather (M-0117): the wetness of the ground
//! and the snow lying on the road set the tyre friction the simulation uses
//! (`VehicleSimulation::road_friction`). The coefficients and the rate limit are in
//! [`opts_sim::surface_grip`]; this module only maps the weather state onto them.

use bevy::prelude::*;
use opts_bevy::reflections::Wetness;
use opts_bevy::{PlayerVehicle, SessionEnvironment, VehicleController, WeatherEffects};
use opts_sim::surface_grip;
use opts_sim::vehicle::ROAD_FRICTION;

use super::surfaces::wanted_state;

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

/// Writes the weather's road friction into the player's vehicle, rate limited.
pub fn drive_road_grip(
    time: Res<Time>,
    env: Res<SessionEnvironment>,
    effects: Res<WeatherEffects>,
    wetness: Res<Wetness>,
    mut vehicles: Query<&mut VehicleController, With<PlayerVehicle>>,
) {
    let snow_road = wanted_state(&env, &effects).snow_road;
    let dt = time.delta_secs();
    for mut controller in &mut vehicles {
        let mu = controller.sim.road_friction;
        controller.sim.road_friction = next_road_friction(mu, wetness.0, snow_road, dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_sim::surface_grip::{MU_SNOW, MU_WET};

    #[test]
    fn the_target_follows_wetness_and_snow_on_the_road() {
        assert_eq!(grip_target(0.0, 0.0), ROAD_FRICTION);
        assert!((grip_target(1.0, 0.0) - MU_WET).abs() < 1e-6);
        assert!((grip_target(1.0, 1.0) - MU_SNOW).abs() < 1e-6);
    }

    #[test]
    fn the_friction_follows_a_change_without_a_step() {
        let dt = 1.0 / 60.0;
        let mut mu = ROAD_FRICTION;
        let mut largest = 0.0_f32;
        for _ in 0..(2.0 / dt) as usize {
            let next = next_road_friction(mu, 1.0, 1.0, dt);
            largest = largest.max((next - mu).abs());
            mu = next;
        }
        assert!((mu - MU_SNOW).abs() < 1e-3, "{mu} after 2 s");
        assert!(largest <= surface_grip::MAX_RATE_PER_S * dt + 1e-6);
    }
}
