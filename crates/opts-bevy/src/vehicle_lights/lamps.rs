//! Sets the intensity and visibility of point and spot lights from the owners' script variables.

use super::*;

/// Sets point and spot light intensities from the owners' script variables.
pub fn vehicle_light_system(
    mut lights: Query<(
        &ScriptLight,
        Option<&mut PointLight>,
        Option<&mut SpotLight>,
        &mut Visibility,
    )>,
    vehicles: Query<(Option<&VehicleScripts>, Option<&AiVariables>)>,
) {
    for (light, point, spot, mut visibility) in &mut lights {
        let Ok((scripts, table)) = vehicles.get(light.owner) else {
            continue;
        };
        let Some(engine) = VariableSource::of(scripts, table) else {
            continue;
        };
        let f = light.factor(|n| engine.get(n));
        let intensity = light.base_intensity * f;
        if let Some(mut p) = point {
            if p.intensity != intensity {
                p.intensity = intensity;
            }
        }
        if let Some(mut s) = spot {
            if s.intensity != intensity {
                s.intensity = intensity;
            }
        }
        // A dark light is hidden so it costs nothing (shadow maps, clustering).
        let want = if f > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
}
