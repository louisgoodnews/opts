//! Distance fog from `[fog]`: `DistanceFog` on the drive camera, exponential-squared with the
//! density that makes objects vanish at the visibility (`opts_bevy::weather::fog_density`), the
//! colour of the sky's horizon (`SkyHorizon`), greyed in thick fog, dark at night.
//!
//! Ground fog (visibility ≤ `FOG_LAYER_MAX_VISIBILITY_M`) is a low layer: the camera's height
//! above the terrain and its pitch go through `opts_bevy::weather::ground_fog`, which raises the
//! visibility the `DistanceFog` gets and clears the sky's veil from above the layer.

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use bevy::render::view::Tonemapping;
use opts_bevy::weather::{fog_density_for, ground_fog_view, weather_look, GroundFogView};
use opts_bevy::{SessionEnvironment, WeatherConditions, WeatherEffects};

use crate::in_game::sky::SkyHorizon;
use crate::in_game::types::InGameCamera;

/// The fog to draw: `None` when the effect is off or the air is clear.
///
/// `tonemapping` is the camera's: the fog colour is blended into the scene before the tonemapper
/// (in the shader or in the HDR pipeline), whereas the clear colour is written already
/// compensated, so the fog is written pre-compensated the same way
/// ([`crate::look::hdr_clear_color`]) and the far geometry fades into the sky that is shown, not
/// into a lighter or darker one.
#[must_use]
pub fn desired_fog(
    conditions: &WeatherConditions,
    effects: &WeatherEffects,
    horizon: [f32; 3],
    sun_alt_deg: f32,
    visibility_m: f32,
    tonemapping: Option<Tonemapping>,
) -> Option<DistanceFog> {
    if !effects.fog {
        return None;
    }
    let density = fog_density_for(conditions, visibility_m)?;
    let mut c = weather_look(conditions, horizon, sun_alt_deg).fog;
    if let Some(tm) = tonemapping {
        c = crate::look::hdr_clear_color(tm, c);
    }
    Some(DistanceFog {
        color: Color::linear_rgb(c[0], c[1], c[2]),
        directional_light_color: Color::NONE,
        directional_light_exponent: 8.0,
        falloff: FogFalloff::ExponentialSquared { density },
    })
}

/// A neutral horizon colour until the sky has computed one (a drive without a sky).
const DEFAULT_HORIZON: [f32; 3] = [0.6, 0.65, 0.7];

/// Measures where the drive camera is relative to the fog layer and publishes it as
/// [`GroundFogView`]: the height above the terrain under it (the ground index; the last known
/// ground, else height 0 when no terrain is loaded) and its pitch.
pub fn measure_fog_view(
    env: Res<SessionEnvironment>,
    effects: Res<WeatherEffects>,
    ground: Option<Res<crate::in_game::omsi_world::GroundIndex>>,
    cameras: Query<&GlobalTransform, With<InGameCamera>>,
    mut last_ground: Local<f32>,
    mut view: ResMut<GroundFogView>,
) {
    let Some(cam) = cameras.iter().next() else {
        return;
    };
    let pos = cam.translation();
    // Session frame: x east, y north; Bevy z is minus north.
    if let Some(g) = ground.and_then(|g| g.terrain_at(pos.x, -pos.z)) {
        *last_ground = g;
    }
    let pitch = cam.forward().y.clamp(-1.0, 1.0).asin().to_degrees();
    let next = ground_fog_view(env.conditions(), effects.fog, pos.y - *last_ground, pitch);
    view.set_if_neq(next);
}

/// Puts the fog on the drive camera, takes it off, and keeps it current: the colour and density
/// follow the weather, the horizon and (ground fog) the camera's height.
pub fn update_fog(
    env: Res<SessionEnvironment>,
    effects: Res<WeatherEffects>,
    horizon: Option<Res<SkyHorizon>>,
    view: Res<GroundFogView>,
    mut cameras: Query<
        (Entity, Option<&mut DistanceFog>, Option<&Tonemapping>),
        With<InGameCamera>,
    >,
    mut commands: Commands,
) {
    let colour = horizon.map_or(DEFAULT_HORIZON, |h| h.0);
    for (camera, current, tonemapping) in &mut cameras {
        let want = desired_fog(
            env.conditions(),
            &effects,
            colour,
            env.sun_alt_deg(),
            view.effective_visibility_m,
            tonemapping.copied(),
        );
        match (&want, current) {
            (Some(fog), None) => {
                commands.entity(camera).insert(fog.clone());
            }
            (Some(fog), Some(mut cur)) => {
                let moved = cur.color != fog.color || !same_falloff(&cur.falloff, &fog.falloff);
                if moved {
                    *cur = fog.clone();
                }
            }
            (None, Some(_)) => {
                commands.entity(camera).remove::<DistanceFog>();
            }
            _ => {}
        }
    }
}

/// Whether two falloffs are the same to a thousandth (the density moves with the camera).
fn same_falloff(a: &FogFalloff, b: &FogFalloff) -> bool {
    match (a, b) {
        (
            FogFalloff::ExponentialSquared { density: x },
            FogFalloff::ExponentialSquared { density: y },
        ) => (x - y).abs() <= 1e-3 * x.abs().max(1e-9),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_bevy::WeatherCatalogue;

    fn fog_of(cat: &WeatherCatalogue, id: &str, effects: &WeatherEffects) -> Option<DistanceFog> {
        let c = cat.resolve(id);
        desired_fog(&c, effects, [0.5; 3], 45.0, c.fog_visibility_m, None)
    }

    #[test]
    fn clear_air_and_a_disabled_effect_draw_no_fog() {
        let cat = WeatherCatalogue::default();
        let on = WeatherEffects::default();
        assert!(fog_of(&cat, "Clear Summer Day", &on).is_none());
        assert!(fog_of(&cat, "Autumn Morning Fog", &on).is_some());
        let off = WeatherEffects { fog: false, ..on };
        assert!(fog_of(&cat, "Autumn Morning Fog", &off).is_none());
    }

    #[test]
    fn thicker_fog_has_a_larger_density_and_the_horizon_colour() {
        let cat = WeatherCatalogue::default();
        let on = WeatherEffects::default();
        let density = |id: &str| match fog_of(&cat, id, &on).unwrap().falloff {
            FogFalloff::ExponentialSquared { density } => density,
            other => panic!("{other:?}"),
        };
        // 300 m (morning fog) against 6 km (rain).
        assert!(density("Autumn Morning Fog") > density("Rainy Afternoon") * 10.0);
        let c = cat.resolve("Autumn Morning Fog");
        let fog = desired_fog(&c, &on, [0.2, 0.2, 0.2], 45.0, c.fog_visibility_m, None).unwrap();
        let srgb = fog.color.to_linear();
        assert!((srgb.red - srgb.blue).abs() < 0.05 && srgb.red < 0.3);
    }

    #[test]
    fn the_fog_is_written_so_that_it_shows_the_colour_the_sky_clear_colour_shows() {
        let cat = WeatherCatalogue::default();
        let on = WeatherEffects::default();
        let c = cat.resolve("Autumn Morning Fog");
        let horizon = [0.55, 0.6, 0.7];
        let shown = |tm| {
            let fog = desired_fog(&c, &on, horizon, 45.0, c.fog_visibility_m, tm).unwrap();
            fog.color.to_linear()
        };
        let plain = shown(None);
        let comp = shown(Some(Tonemapping::SomewhatBoringDisplayTransform));
        // The compensated colour goes through the tonemapper and comes out as the plain one.
        let back = crate::look::somewhat_boring([comp.red, comp.green, comp.blue]);
        for (b, p) in back.iter().zip([plain.red, plain.green, plain.blue]) {
            assert!((b - p).abs() < 0.01, "{back:?} vs {plain:?}");
        }
    }
}
