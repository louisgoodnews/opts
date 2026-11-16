//! Scales the emission of light-map materials by the owners' script variables.

use super::*;

/// Scales light-map emission by the owners' script variables.
pub fn vehicle_light_map_system(
    mut parts: Query<(&mut LightMapBinding, &mut MeshMaterial3d<StandardMaterial>)>,
    vehicles: Query<(Option<&VehicleScripts>, Option<&AiVariables>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    clock: Option<Res<crate::scenery_runtime::SceneryClock>>,
) {
    for (mut binding, mut material) in &mut parts {
        let Ok((scripts, table)) = vehicles.get(binding.owner) else {
            continue;
        };
        // Night maps are switched by the night (`NightlightA`), which vehicle scripts do not
        // own: the environment clock answers it, as for scenery.
        let night = binding
            .variable
            .eq_ignore_ascii_case(crate::scenery_runtime::NIGHT_VARIABLE)
            .then(|| clock.as_ref().map_or(0.0, |c| f32::from(u8::from(c.night))));
        let f = if let Some(n) = night {
            n
        } else {
            let Some(engine) = VariableSource::of(scripts, table) else {
                continue;
            };
            engine.get(&binding.variable).clamp(0.0, MAX_BRIGHTNESS)
        };
        if f == binding.applied {
            continue;
        }
        if !binding.private {
            let Some(copy) = materials.get(&material.0).cloned() else {
                continue;
            };
            material.0 = materials.add(copy);
            binding.private = true;
        }
        if let Some(mut m) = materials.get_mut(&material.0) {
            let gain = f * LIGHT_MAP_GAIN;
            m.emissive = LinearRgba::rgb(gain, gain, gain);
        }
        binding.applied = f;
    }
}
