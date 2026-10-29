//! The pool of real headlights: the nearest vehicles whose headlights shine get one spot light.

use bevy::prelude::*;

use super::{VehicleGlow, HEADLIGHT_ON, HEADLIGHT_POOL};
use crate::scenery_runtime::SceneryLight;

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

/// Shining headlights per vehicle: the middle of the pair and its forward direction.
fn shining_headlights(
    glows: &Query<(&VehicleGlow, &SceneryLight, &GlobalTransform)>,
) -> Vec<OwnerLights> {
    let mut per_owner: Vec<OwnerLights> = Vec::new();
    for (glow, light, global) in glows {
        if !glow.headlight || light.level < HEADLIGHT_ON {
            continue;
        }
        let pos = global.translation();
        let dir = global.rotation() * light.direction;
        match per_owner.iter_mut().find(|p| p.0 == glow.owner) {
            Some(p) => {
                p.1 += pos;
                p.2 += dir;
                p.3 += 1;
            }
            None => per_owner.push((glow.owner, pos, dir, 1, glow.color)),
        }
    }
    per_owner
}

/// Grows the pool lazily up to the number of chosen vehicles (at most [`HEADLIGHT_POOL`]).
fn grow_pool(commands: &mut Commands, size: usize, chosen: usize) {
    for _ in size..HEADLIGHT_POOL.min(chosen.max(size)) {
        commands.spawn((
            SpotLight {
                range: HEADLIGHT_RANGE_M,
                intensity: HEADLIGHT_LUMENS,
                inner_angle: 12f32.to_radians(),
                outer_angle: 30f32.to_radians(),
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
            Visibility::Hidden,
            PooledHeadlight::default(),
            Name::new("PooledHeadlight"),
        ));
    }
}

/// Frees lights whose vehicle was not chosen, gives the free ones to new vehicles and puts each
/// assigned light in front of its vehicle.
fn assign_pool(
    pool: &mut PoolQuery,
    owners: &Query<&GlobalTransform, Without<PooledHeadlight>>,
    per_owner: &[OwnerLights],
    chosen: &[Entity],
    current: &[Entity],
) {
    let mut unassigned: Vec<Entity> = chosen
        .iter()
        .copied()
        .filter(|e| !current.contains(e))
        .collect();
    for (_, mut slot, _, mut vis, _) in pool.iter_mut() {
        if slot.owner.is_some_and(|o| !chosen.contains(&o)) {
            slot.owner = None;
            *vis = Visibility::Hidden;
        }
    }
    for (_, mut slot, mut transform, mut vis, mut spot) in pool.iter_mut() {
        if slot.owner.is_none() {
            slot.owner = unassigned.pop();
        }
        let Some(owner) = slot.owner else {
            continue;
        };
        let Some(p) = per_owner.iter().find(|p| p.0 == owner) else {
            continue;
        };
        if owners.get(owner).is_err() {
            slot.owner = None;
            *vis = Visibility::Hidden;
            continue;
        }
        let pos = p.1 / p.3 as f32;
        let dir = p.2.normalize_or_zero();
        if dir == Vec3::ZERO {
            continue;
        }
        *transform = Transform::from_translation(pos).looking_to(dir, Vec3::Y);
        let c = p.4.map(|v| (v / 255.0).clamp(0.0, 1.0));
        spot.color = Color::srgb(c[0], c[1], c[2]);
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }
    }
}

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
