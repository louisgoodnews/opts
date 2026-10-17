//! Wet and snowy surfaces from `[groundwet]`, `[snow]` and `[snowOnRoad]`: the ground materials
//! get the wet parameters of the ground shader (darker, smoother, puddles, snow cover), the
//! road materials of the splines get the same factor on their colour and roughness. One global
//! factor, no per-map code.

use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::reflections::Wetness;
use opts_bevy::weather::{road_surface, surface_state, SurfaceState};
use opts_bevy::{SessionEnvironment, WeatherEffects};

use crate::in_game::omsi_world::WorldLoadJob;

/// Seconds between checks for materials that are new (tiles streamed in).
const SCAN_S: f32 = 0.5;

/// The ground shader's `wet` uniform for a surface state: wetness, puddles, snow cover.
#[must_use]
pub fn ground_wet(s: &SurfaceState) -> Vec4 {
    Vec4::new(s.wet, s.puddles, s.snow_ground, 0.0)
}

/// The surface state to show: dry when the effect is off.
#[must_use]
pub fn wanted_state(env: &SessionEnvironment, effects: &WeatherEffects) -> SurfaceState {
    if effects.wet_ground {
        surface_state(env.conditions())
    } else {
        SurfaceState::default()
    }
}

/// Writes the weather's wetness into the [`Wetness`] resource, the single driver of wet surfaces
/// (the reflections prototype's parameter, `docs/src/architecture/reflections-research.md`).
pub fn drive_wetness(
    env: Res<SessionEnvironment>,
    effects: Res<WeatherEffects>,
    mut wetness: ResMut<Wetness>,
) {
    let wet = wanted_state(&env, &effects).wet;
    if (wetness.0 - wet).abs() > 1e-4 {
        wetness.0 = wet;
    }
}

/// Applies the surface state to the ground and road materials: at once when it changes, and
/// every half second for materials that are new.
#[allow(clippy::too_many_arguments)]
pub fn update_surfaces(
    time: Res<Time<Real>>,
    env: Res<SessionEnvironment>,
    effects: Res<WeatherEffects>,
    wetness: Res<Wetness>,
    job: Option<Res<WorldLoadJob>>,
    mut ground: ResMut<Assets<GroundMaterial>>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    mut originals: Local<HashMap<AssetId<StandardMaterial>, (Color, f32)>>,
    mut last: Local<Option<SurfaceState>>,
    mut next_scan: Local<f32>,
) {
    // Wetness comes from the `Wetness` resource; puddles and snow from the weather.
    let mut state = wanted_state(&env, &effects);
    state.wet = wetness.0.clamp(0.0, 1.0);
    let changed = *last != Some(state);
    if !changed && time.elapsed_secs() < *next_scan {
        return;
    }
    *next_scan = time.elapsed_secs() + SCAN_S;
    *last = Some(state);

    let wet = ground_wet(&state);
    let stale: Vec<_> = ground
        .iter()
        .filter(|(_, m)| m.extension.wet != wet)
        .map(|(id, _)| id)
        .collect();
    for id in stale {
        if let Some(mut m) = ground.get_mut(id) {
            m.extension.wet = wet;
        }
    }

    let Some(job) = job else {
        originals.clear();
        return;
    };
    for handle in job.spline_material_handles() {
        let id = handle.id();
        let Some(m) = standard.get(id) else { continue };
        let (colour, roughness) = *originals
            .entry(id)
            .or_insert((m.base_color, m.perceptual_roughness));
        let base = colour.to_linear();
        let (c, r) = road_surface([base.red, base.green, base.blue], roughness, &state);
        let want = Color::linear_rgba(c[0], c[1], c[2], base.alpha);
        if m.base_color != want || (m.perceptual_roughness - r).abs() > 1e-4 {
            if let Some(mut m) = standard.get_mut(id) {
                m.base_color = want;
                m.perceptual_roughness = r;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_bevy::WeatherCatalogue;

    fn env_of(id: &str) -> SessionEnvironment {
        let mut env = SessionEnvironment::default();
        env.apply(&opts_bevy::EnvironmentChange {
            weather: Some(id.into()),
            ..default()
        });
        env
    }

    #[test]
    fn rain_snow_and_the_switch_set_the_shader_uniform() {
        let on = WeatherEffects::default();
        let dry = wanted_state(&env_of("Clear Summer Day"), &on);
        assert_eq!(ground_wet(&dry), Vec4::ZERO);
        let rain = ground_wet(&wanted_state(&env_of("Rainy Afternoon"), &on));
        assert!(rain.x > 0.9 && rain.y > 0.0 && rain.z == 0.0, "{rain}");
        let snow = ground_wet(&wanted_state(&env_of("Winter Snowfall"), &on));
        assert!(snow.z > 0.8, "{snow}");
        let off = WeatherEffects {
            wet_ground: false,
            ..on
        };
        assert_eq!(
            ground_wet(&wanted_state(&env_of("Rainy Afternoon"), &off)),
            Vec4::ZERO
        );
        // Built-in presets only use the catalogue's built-ins.
        assert!(!WeatherCatalogue::builtin().is_installed());
    }
}
