//! The per-frame system that draws, updates and hides the precipitation.

use bevy::prelude::*;
use opts_bevy::weather::{precip_plan, weather_look};
use opts_core::formats::weather::PrecipKind;

use super::assets::{base_colour, ensure_assets};
use super::layer::{sync_layer, write_geometry};
use super::state::{PrecipLayer, PrecipState};
use crate::in_game::types::InGameCamera;

/// Draws, updates and hides the precipitation.
pub fn update_precipitation(
    scene: super::super::WeatherScene,
    assets: super::super::WeatherAssets,
    mut state: ResMut<PrecipState>,
    cameras: Query<&GlobalTransform, With<InGameCamera>>,
    mut layer: Query<(&mut Visibility, &mut MeshMaterial3d<StandardMaterial>), With<PrecipLayer>>,
    mut commands: Commands,
) {
    let super::super::WeatherScene { time, env, effects } = scene;
    let super::super::WeatherAssets {
        mut images,
        mut meshes,
        mut materials,
    } = assets;
    let plan = precip_plan(
        &effects.scaled(env.conditions()),
        effects.precipitation.budget(),
    );
    let Some(plan) = plan else {
        state.kind = None;
        for (mut v, _) in &mut layer {
            if *v != Visibility::Hidden {
                *v = Visibility::Hidden;
            }
        }
        return;
    };
    let Some(cam) = cameras.iter().next() else {
        return;
    };
    let budget = effects.precipitation.budget() as usize;

    ensure_assets(
        &mut state,
        (&mut images, &mut meshes, &mut materials),
        budget,
    );
    let mesh_handle = state.mesh.clone().unwrap_or_default();
    let want_material = if plan.kind == PrecipKind::Snow {
        state.snow.clone()
    } else {
        state.rain.clone()
    }
    .unwrap_or_default();

    sync_layer(&mut commands, &mut layer, &mesh_handle, &want_material);
    // Colour follows the daylight.
    let tint = weather_look(env.conditions(), [0.0; 3], env.sun_alt_deg()).precip;
    if state.kind != Some(plan.kind) || (state.tint - tint).abs() > 0.01 {
        state.tint = tint;
        for (handle, kind) in [
            (state.rain.clone(), PrecipKind::Rain),
            (state.snow.clone(), PrecipKind::Snow),
        ] {
            if let Some(mut m) = handle.and_then(|h| materials.get_mut(&h)) {
                m.base_color = base_colour(kind, tint);
            }
        }
    }
    state.kind = Some(plan.kind);

    // Geometry.
    write_geometry(
        &mut state,
        &mut meshes,
        &mesh_handle,
        &plan,
        (time.elapsed_secs_f64(), cam),
    );
}
