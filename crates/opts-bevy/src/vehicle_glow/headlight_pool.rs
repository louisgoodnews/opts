//! The pool of real headlights: the nearest vehicles whose headlights shine get one spot light.

use bevy::prelude::*;

use super::{VehicleGlow, HEADLIGHT_ON, HEADLIGHT_POOL};
use crate::scenery_runtime::SceneryLight;

mod assign;

use assign::{assign_pool, grow_pool, shining_headlights};

/// A vehicle keeps its spot light until it is this much farther than the nearest one without.
pub const HEADLIGHT_HYSTERESIS: f32 = 1.25;
/// Range (m) and intensity (lumens) of a pooled headlight.
const HEADLIGHT_RANGE_M: f32 = 60.0;
const HEADLIGHT_LUMENS: f32 = 60_000.0;

/// The vehicles that get a real headlight: the `n` nearest of `candidates` (vehicle, distance),
/// except that a vehicle in `current` keeps its light while it is no farther than
/// `hysteresis` × the distance of the nearest candidate left out.
#[must_use]
pub fn choose_headlights(
    candidates: &[(Entity, f32)],
    current: &[Entity],
    n: usize,
    hysteresis: f32,
) -> Vec<Entity> {
    let mut sorted = candidates.to_vec();
    sorted.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut chosen: Vec<(Entity, f32)> = sorted.iter().take(n).copied().collect();
    // An incumbent left out displaces the farthest newcomer when it is nearly as near.
    for inc in sorted.iter().filter(|c| current.contains(&c.0)) {
        if chosen.iter().any(|c| c.0 == inc.0) {
            continue;
        }
        let newcomer = chosen
            .iter()
            .enumerate()
            .filter(|(_, c)| !current.contains(&c.0))
            .max_by(|a, b| a.1 .1.total_cmp(&b.1 .1))
            .map(|(k, c)| (k, c.1));
        match newcomer {
            Some((k, d)) if inc.1 <= d * hysteresis => chosen[k] = *inc,
            _ => {}
        }
    }
    chosen.into_iter().map(|c| c.0).collect()
}

/// A pooled real headlight and the vehicle it belongs to.
#[derive(Component, Debug, Default)]
pub struct PooledHeadlight {
    /// The vehicle root it lights for, if any.
    pub owner: Option<Entity>,
}

/// Shining headlights of one vehicle: owner, summed position, summed direction, count, colour.
type OwnerLights = (Entity, Vec3, Vec3, usize, [f32; 3]);

/// The pooled spot lights with what the pool system changes on them.
type PoolQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut PooledHeadlight,
        &'static mut Transform,
        &'static mut Visibility,
        &'static mut SpotLight,
    ),
>;

/// Assigns the pooled spot lights to the nearest vehicles whose headlights shine and keeps them
/// in front of those vehicles.
pub fn headlight_pool_system(
    mut commands: Commands,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::MainCamera>>,
    glows: Query<(&VehicleGlow, &SceneryLight, &GlobalTransform)>,
    owners: Query<&GlobalTransform, Without<PooledHeadlight>>,
    mut pool: PoolQuery,
) {
    let Some(cam) = cameras
        .iter()
        .find(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation())
    else {
        return;
    };
    let per_owner = shining_headlights(&glows);
    let candidates: Vec<(Entity, f32)> = per_owner
        .iter()
        .map(|p| (p.0, (p.1 / p.3 as f32).distance(cam)))
        .collect();
    let current: Vec<Entity> = pool.iter().filter_map(|p| p.1.owner).collect();
    let chosen = choose_headlights(&candidates, &current, HEADLIGHT_POOL, HEADLIGHT_HYSTERESIS);
    grow_pool(&mut commands, pool.iter().count(), chosen.len());
    assign_pool(&mut pool, &owners, &per_owner, &chosen, &current);
}
