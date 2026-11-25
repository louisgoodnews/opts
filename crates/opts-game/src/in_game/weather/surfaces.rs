//! Wet and snowy surfaces from `[groundwet]`, `[snow]` and `[snowOnRoad]`: the ground materials
//! get the wet parameters of the ground shader (darker, smoother, puddles, snow cover), the
//! road materials of the splines get the same factor on their colour and roughness. One global
//! factor, no per-map code.

use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::ground_material::GroundMaterial;
use opts_bevy::reflections::{Wetness, WetnessHold};
use opts_bevy::weather::{road_surface, SurfaceState};
use opts_bevy::{SessionEnvironment, WeatherEffects};
use opts_sim::surface_grip::approach;

use crate::in_game::omsi_world::WorldLoadJob;

/// Seconds between checks for materials that are new (tiles streamed in).
const SCAN_S: f32 = 0.5;

/// The ground shader's `wet` uniform for a surface state: wetness, puddles, snow cover.
#[must_use]
pub fn ground_wet(s: &SurfaceState) -> Vec4 {
    Vec4::new(s.wet, s.puddles, s.snow_ground, 0.0)
}

/// The surface state to show: dry when the effect is off (the weather mapping is in
/// `opts-world-load`, re-exported here).
pub use opts_world_load::ai_weather::wanted_state;

/// The wetness that follows `target` from `current` over `dt` seconds. It uses the grip's rate
/// limit ([`opts_sim::surface_grip::approach`], at most 0.5 per second), so a shower starts and
/// stops without a one-frame pop; both ends are clamped to 0..=1.
#[must_use]
pub fn next_wetness(current: f32, target: f32, dt: f32) -> f32 {
    approach(current, target.clamp(0.0, 1.0), dt).clamp(0.0, 1.0)
}

/// The wetness after one frame: the held value while a hold is set (`sim.set_time hold=true`,
/// not ramped), else the ramp of [`next_wetness`] toward the weather's target.
#[must_use]
pub fn drive_step(current: f32, target: f32, dt: f32, hold: Option<f32>) -> f32 {
    match hold {
        Some(held) => held.clamp(0.0, 1.0),
        None => next_wetness(current, target, dt),
    }
}

/// Writes the weather's wetness, ramped by [`next_wetness`] unless a hold is set
/// ([`drive_step`]), into the [`Wetness`] resource, the single driver of wet surfaces (the
/// reflections prototype's parameter, `docs/src/architecture/reflections-research.md`).
pub fn drive_wetness(
    time: Res<Time>,
    env: Res<SessionEnvironment>,
    effects: Res<WeatherEffects>,
    hold: Option<Res<WetnessHold>>,
    mut wetness: ResMut<Wetness>,
) {
    let target = wanted_state(&env, &effects).wet;
    let held = hold.and_then(|h| h.0);
    let next = drive_step(wetness.0, target, time.delta_secs(), held);
    if (wetness.0 - next).abs() > 1e-4 {
        wetness.0 = next;
    }
}

/// The ground and standard material stores that [`update_surfaces`] rewrites.
#[derive(bevy::ecs::system::SystemParam)]
pub struct SurfaceMaterials<'w> {
    ground: ResMut<'w, Assets<GroundMaterial>>,
    standard: ResMut<'w, Assets<StandardMaterial>>,
}

/// Applies the surface state to the ground and road materials: at once when it changes, and
/// every half second for materials that are new.
pub fn update_surfaces(
    scene: super::WeatherScene,
    wetness: Res<Wetness>,
    job: Option<Res<WorldLoadJob>>,
    materials: SurfaceMaterials,
    mut originals: Local<HashMap<AssetId<StandardMaterial>, (Color, f32)>>,
    mut last: Local<Option<SurfaceState>>,
    mut next_scan: Local<f32>,
) {
    let super::WeatherScene { time, env, effects } = scene;
    let SurfaceMaterials {
        mut ground,
        mut standard,
    } = materials;
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
mod tests;
