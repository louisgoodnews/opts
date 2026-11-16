//! Vehicles without a script engine animate from a plain variable table (AI traffic).

use bevy::prelude::*;
use opts_bevy::{
    vehicle_alpha_scale_system, vehicle_animation_system, vehicle_light_map_system, AiVariables,
    AlphaScaleBinding, AnimStep, LightMapBinding, MeshAnimation,
};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(
        Update,
        (
            vehicle_animation_system,
            vehicle_light_map_system,
            vehicle_alpha_scale_system,
        ),
    );
    app
}

mod daylight;
mod glass;
mod light_map;
mod wheel_and_passenger;
