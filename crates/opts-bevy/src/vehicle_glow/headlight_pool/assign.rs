//! Pool bookkeeping: the shining headlights per vehicle, growing the pool and assigning lights.

use super::*;

/// Shining headlights per vehicle: the middle of the pair and its forward direction.
pub(super) fn shining_headlights(
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
pub(super) fn grow_pool(commands: &mut Commands, size: usize, chosen: usize) {
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
pub(super) fn assign_pool(
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
