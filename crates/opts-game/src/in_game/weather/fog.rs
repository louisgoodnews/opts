//! Distance fog from `[fog]`: `DistanceFog` on the drive camera, exponential-squared with the
//! density that makes objects vanish at the visibility (`opts_bevy::weather::fog_density`), the
//! colour of the sky's horizon (`SkyHorizon`), greyed in thick fog, dark at night.

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use opts_bevy::weather::{fog_color, fog_density};
use opts_bevy::{SessionEnvironment, WeatherConditions, WeatherEffects};

use crate::in_game::sky::SkyHorizon;
use crate::in_game::types::InGameCamera;

/// The fog to draw: `None` when the effect is off or the air is clear.
#[must_use]
pub fn desired_fog(
    conditions: &WeatherConditions,
    effects: &WeatherEffects,
    horizon: [f32; 3],
) -> Option<DistanceFog> {
    if !effects.fog {
        return None;
    }
    let density = fog_density(conditions)?;
    let c = fog_color(horizon, conditions);
    Some(DistanceFog {
        color: Color::srgb(c[0], c[1], c[2]),
        directional_light_color: Color::NONE,
        directional_light_exponent: 8.0,
        falloff: FogFalloff::ExponentialSquared { density },
    })
}

/// A neutral horizon colour until the sky has computed one (a drive without a sky).
const DEFAULT_HORIZON: [f32; 3] = [0.6, 0.65, 0.7];

/// Puts the fog on the drive camera, takes it off, and keeps it current.
pub fn update_fog(
    env: Res<SessionEnvironment>,
    effects: Res<WeatherEffects>,
    horizon: Option<Res<SkyHorizon>>,
    cameras: Query<(Entity, Option<&DistanceFog>), With<InGameCamera>>,
    mut commands: Commands,
) {
    let horizon_changed = horizon.as_ref().is_some_and(|h| h.is_changed());
    let colour = horizon.map_or(DEFAULT_HORIZON, |h| h.0);
    let want = desired_fog(env.conditions(), &effects, colour);
    let dirty = env.is_changed() || effects.is_changed() || horizon_changed;
    for (camera, current) in &cameras {
        match (&want, current) {
            (Some(fog), None) => {
                commands.entity(camera).insert(fog.clone());
            }
            (Some(fog), Some(_)) if dirty => {
                commands.entity(camera).insert(fog.clone());
            }
            (None, Some(_)) => {
                commands.entity(camera).remove::<DistanceFog>();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_bevy::WeatherCatalogue;

    #[test]
    fn clear_air_and_a_disabled_effect_draw_no_fog() {
        let cat = WeatherCatalogue::default();
        let on = WeatherEffects::default();
        assert!(desired_fog(&cat.resolve("Clear Summer Day"), &on, [0.5; 3]).is_none());
        let foggy = cat.resolve("Autumn Morning Fog");
        assert!(desired_fog(&foggy, &on, [0.5; 3]).is_some());
        let off = WeatherEffects { fog: false, ..on };
        assert!(desired_fog(&foggy, &off, [0.5; 3]).is_none());
    }

    #[test]
    fn thicker_fog_has_a_larger_density_and_the_horizon_colour() {
        let cat = WeatherCatalogue::default();
        let on = WeatherEffects::default();
        let density = |id: &str| match desired_fog(&cat.resolve(id), &on, [0.2, 0.2, 0.2])
            .unwrap()
            .falloff
        {
            FogFalloff::ExponentialSquared { density } => density,
            other => panic!("{other:?}"),
        };
        // 300 m (morning fog) against 6 km (rain).
        assert!(density("Autumn Morning Fog") > density("Rainy Afternoon") * 10.0);
        let fog = desired_fog(&cat.resolve("Autumn Morning Fog"), &on, [0.2, 0.2, 0.2]).unwrap();
        let srgb = fog.color.to_srgba();
        assert!((srgb.red - srgb.blue).abs() < 0.05 && srgb.red < 0.3);
    }
}
