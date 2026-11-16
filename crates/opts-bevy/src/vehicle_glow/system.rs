//! The per-frame side of the glows: their mesh and material, and their brightness from the variables.

use bevy::prelude::*;

use crate::ai_variables::{AiVariables, VariableSource};
use crate::scenery_runtime::{SceneryLight, SceneryLightAssets};
use crate::vehicle_script::VehicleScripts;

use super::VehicleGlow;

/// Shared sprite mesh and per-colour materials of vehicle glows.
#[derive(Resource, Default)]
pub struct VehicleGlowAssets(pub SceneryLightAssets);

/// Gives new glow sprites their mesh and material.
pub fn attach_glow_assets(
    mut commands: Commands,
    fresh: Query<(Entity, &VehicleGlow), Without<Mesh3d>>,
    mut assets: ResMut<VehicleGlowAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, glow) in &fresh {
        let mesh = assets.0.glow_mesh(&mut meshes);
        let material = assets.0.material(&mut materials, glow.color);
        commands
            .entity(e)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }
}

/// Sets each glow's brightness from its owner's variable, smoothed by its time constant.
pub fn vehicle_glow_system(
    time: Res<Time<Real>>,
    vehicles: Query<(Option<&VehicleScripts>, Option<&AiVariables>)>,
    mut glows: Query<(&VehicleGlow, &mut SceneryLight)>,
) {
    let dt = time.delta_secs().min(0.5);
    for (glow, mut light) in &mut glows {
        let Ok((scripts, table)) = vehicles.get(glow.owner) else {
            continue;
        };
        let value = if glow.variable.is_empty() {
            1.0
        } else {
            let Some(source) = VariableSource::of(scripts, table) else {
                continue;
            };
            source.get(&glow.variable)
        };
        let target = (value * glow.factor).clamp(0.0, 1.0);
        let level = glow.approach(light.level, target, dt);
        let level = if (level - target).abs() < 1e-3 {
            target
        } else {
            level
        };
        if light.level != level {
            light.level = level;
        }
    }
}
