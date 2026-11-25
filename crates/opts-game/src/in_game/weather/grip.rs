//! The road grip of the player's vehicle follows the weather (M-0117): the wetness of the ground
//! and the snow lying on the road set the tyre friction the simulation uses
//! (`VehicleSimulation::road_friction`). The coefficients and the rate limit are in
//! [`opts_sim::surface_grip`]; the weather mapping and the AI inputs are in `opts-world-load`
//! (`ai_weather`), re-exported here.

use bevy::prelude::*;
use opts_bevy::reflections::Wetness;
use opts_bevy::{PlayerVehicle, SessionEnvironment, VehicleController, WeatherEffects};

pub use opts_world_load::ai_weather::{grip_target, next_road_friction, AiWeather};

use super::surfaces::wanted_state;

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
